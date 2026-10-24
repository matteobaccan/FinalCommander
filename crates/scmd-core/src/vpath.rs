use std::ffi::OsStr;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Which provider owns a path. `file` is the core's; every other scheme is a provider's
/// (see `docs/spec/04-vfs.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scheme {
    Local,
    Sftp,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Scheme::Local => "file",
            Scheme::Sftp => "sftp",
        }
    }

    pub fn parse(text: &str) -> Option<Scheme> {
        match text.to_ascii_lowercase().as_str() {
            "file" | "local" => Some(Scheme::Local),
            "sftp" | "ssh" | "scp" => Some(Scheme::Sftp),
            _ => None,
        }
    }

    /// Does this scheme need an authority (`user@host:port`) and a connection?
    pub fn is_remote(self) -> bool {
        !matches!(self, Scheme::Local)
    }
}

/// A VFS-aware path.
///
/// Local names are kept as [`std::ffi::OsString`] so that non-UTF-8 names on Unix and the odd
/// UTF-16 name on Windows survive a round trip; display is lossy and only used for rendering.
/// Remote paths are always `/`-separated UTF-8, whatever the local platform does, because that
/// is what the wire carries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VPath {
    scheme: Scheme,
    /// `user@host:port` for a remote path; `None` for a local one.
    authority: Option<Arc<str>>,
    path: PathBuf,
}

impl VPath {
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self {
            scheme: Scheme::Local,
            authority: None,
            path: path.into(),
        }
    }

    /// A path on a remote provider: `VPath::remote(Scheme::Sftp, "mb@host:22", "/srv/www")`.
    pub fn remote(scheme: Scheme, authority: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        Self {
            scheme,
            authority: Some(Arc::from(authority.as_ref())),
            path: PathBuf::from(normalise_remote(path.as_ref())),
        }
    }

    /// Parse `sftp://mb@host:22/srv/www`. Anything that is not a known remote URL is a local
    /// path, so a Windows path (`C:\x`) and a file with a colon in its name still work.
    pub fn parse(text: &str) -> VPath {
        if let Some((head, rest)) = text.split_once("://") {
            if let Some(scheme) = Scheme::parse(head) {
                if scheme.is_remote() {
                    let (authority, path) = match rest.find('/') {
                        Some(i) => (&rest[..i], &rest[i..]),
                        None => (rest, "/"),
                    };
                    if !authority.is_empty() {
                        return VPath::remote(scheme, authority, path);
                    }
                }
            }
        }
        VPath::local(text)
    }

    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// `user@host:port`, for a remote path.
    pub fn authority(&self) -> Option<&str> {
        self.authority.as_deref()
    }

    pub fn is_local(&self) -> bool {
        self.scheme == Scheme::Local
    }

    pub fn is_remote(&self) -> bool {
        self.scheme.is_remote()
    }

    /// The path without the scheme or authority. Only meaningful to the provider that owns it:
    /// a caller that reaches for `std::fs` must check [`VPath::is_local`] first.
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// The `/`-separated path a remote provider puts on the wire.
    pub fn remote_path(&self) -> String {
        self.path.to_string_lossy().replace('\\', "/")
    }

    /// Same provider and same connection?
    pub fn same_provider(&self, other: &VPath) -> bool {
        self.scheme == other.scheme && self.authority == other.authority
    }

    pub fn file_name(&self) -> Option<&OsStr> {
        if self.is_remote() {
            let path = self.path.to_str()?;
            let name = path.rsplit('/').next()?;
            return if name.is_empty() {
                None
            } else {
                Some(OsStr::new(name))
            };
        }
        self.path.file_name()
    }

    /// The name to show in a panel header: the last component, or the whole path for roots.
    pub fn display_name(&self) -> String {
        match self.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => self.to_string(),
        }
    }

    pub fn parent(&self) -> Option<VPath> {
        if self.is_remote() {
            let path = self.remote_path();
            if path == "/" {
                return None;
            }
            let cut = path.rfind('/')?;
            let parent = if cut == 0 { "/" } else { &path[..cut] };
            return Some(self.with_remote(parent));
        }
        self.path.parent().map(|p| self.with_path(p.to_path_buf()))
    }

    pub fn join(&self, name: impl AsRef<OsStr>) -> VPath {
        if self.is_remote() {
            let name = name.as_ref().to_string_lossy();
            let base = self.remote_path();
            let joined = if base.ends_with('/') {
                format!("{base}{name}")
            } else {
                format!("{base}/{name}")
            };
            return self.with_remote(&joined);
        }
        self.with_path(self.path.join(name.as_ref()))
    }

    /// The filesystem root of this path (`/`, `C:\`, …). A remote path's root is always `/`.
    pub fn root(&self) -> VPath {
        if self.is_remote() {
            return self.with_remote("/");
        }
        let mut root = PathBuf::new();
        for c in self.path.components() {
            match c {
                Component::Prefix(p) => root.push(p.as_os_str()),
                Component::RootDir => root.push(Component::RootDir.as_os_str()),
                _ => break,
            }
        }
        if root.as_os_str().is_empty() {
            root.push(Component::RootDir.as_os_str());
        }
        self.with_path(root)
    }

    pub fn is_root(&self) -> bool {
        if self.is_remote() {
            return self.remote_path() == "/";
        }
        self.path.parent().is_none()
    }

    /// Is `self` inside `other` (or the same path)? Never true across providers.
    pub fn starts_with(&self, other: &VPath) -> bool {
        if !self.same_provider(other) {
            return false;
        }
        if self.is_remote() {
            let (a, b) = (self.remote_path(), other.remote_path());
            return a == b
                || a.starts_with(&b)
                    && (b.ends_with('/') || a.as_bytes().get(b.len()) == Some(&b'/'));
        }
        self.path.starts_with(&other.path)
    }

    /// Normalise `.`/`..` lexically, without touching the filesystem.
    pub fn normalised(&self) -> VPath {
        if self.is_remote() {
            let path = self.remote_path();
            let mut out: Vec<&str> = Vec::new();
            for part in path.split('/') {
                match part {
                    "" | "." => {}
                    ".." => {
                        out.pop();
                    }
                    other => out.push(other),
                }
            }
            return self.with_remote(&format!("/{}", out.join("/")));
        }
        let mut out = PathBuf::new();
        for c in self.path.components() {
            match c {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !out.pop() {
                        out.push(Component::ParentDir.as_os_str());
                    }
                }
                other => out.push(other.as_os_str()),
            }
        }
        if out.as_os_str().is_empty() {
            out.push(Component::CurDir.as_os_str());
        }
        self.with_path(out)
    }

    fn with_path(&self, path: PathBuf) -> VPath {
        VPath {
            scheme: self.scheme,
            authority: self.authority.clone(),
            path,
        }
    }

    fn with_remote(&self, path: &str) -> VPath {
        VPath {
            scheme: self.scheme,
            authority: self.authority.clone(),
            path: PathBuf::from(normalise_remote(path)),
        }
    }
}

/// A remote path always starts with `/` and never ends with one (except the root itself).
fn normalise_remote(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

impl fmt::Display for VPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.authority {
            Some(authority) => write!(
                f,
                "{}://{}{}",
                self.scheme.as_str(),
                authority,
                self.remote_path()
            ),
            None => write!(f, "{}", self.path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_and_root() {
        let p = VPath::local("/usr/local/bin");
        assert_eq!(p.parent().unwrap(), VPath::local("/usr/local"));
        // `root` rebuilds the path, so it comes out with this platform's separator.
        assert_eq!(p.root(), VPath::local(std::path::MAIN_SEPARATOR_STR));
        assert!(VPath::local("/").is_root());
        assert!(!p.is_root());
    }

    #[test]
    fn normalisation_is_lexical() {
        // Built the same way the result is: from components, with the platform's separator.
        let expected: PathBuf = [std::path::MAIN_SEPARATOR_STR, "a", "c", "d"]
            .iter()
            .collect();
        assert_eq!(
            VPath::local("/a/b/../c/./d").normalised(),
            VPath::local(expected)
        );
    }

    #[test]
    fn display_name_falls_back_to_the_whole_path() {
        assert_eq!(VPath::local("/etc/hosts").display_name(), "hosts");
        assert_eq!(VPath::local("/").display_name(), "/");
    }

    #[test]
    fn a_remote_url_round_trips() {
        let p = VPath::parse("sftp://mb@host.example:2222/srv/www");
        assert_eq!(p.scheme(), Scheme::Sftp);
        assert_eq!(p.authority(), Some("mb@host.example:2222"));
        assert_eq!(p.remote_path(), "/srv/www");
        assert_eq!(p.to_string(), "sftp://mb@host.example:2222/srv/www");
        assert!(p.is_remote() && !p.is_local());
        // No path at all means the remote root.
        let root = VPath::parse("sftp://host");
        assert_eq!(root.remote_path(), "/");
        assert!(root.is_root());
        assert_eq!(root.parent(), None);
    }

    #[test]
    fn a_local_path_is_never_mistaken_for_a_url() {
        for text in ["/etc/hosts", "C:\\Users\\mb", "weird://name", "a:b"] {
            assert!(VPath::parse(text).is_local(), "{text} parsed as remote");
        }
    }

    #[test]
    fn remote_paths_always_use_forward_slashes() {
        let p = VPath::remote(Scheme::Sftp, "host", "/srv/");
        let child = p.join("www").join("index.html");
        assert_eq!(child.remote_path(), "/srv/www/index.html");
        assert_eq!(child.display_name(), "index.html");
        assert_eq!(child.parent().unwrap().remote_path(), "/srv/www");
        assert_eq!(child.root().remote_path(), "/");
        assert_eq!(
            child.parent().unwrap().parent().unwrap().remote_path(),
            "/srv"
        );
        assert_eq!(
            child.parent().unwrap().parent().unwrap().parent(),
            Some(VPath::remote(Scheme::Sftp, "host", "/"))
        );
        assert_eq!(
            VPath::remote(Scheme::Sftp, "host", "/a/b/../c/./d")
                .normalised()
                .remote_path(),
            "/a/c/d"
        );
    }

    #[test]
    fn starts_with_never_crosses_providers() {
        let local = VPath::local("/srv/www");
        let remote = VPath::remote(Scheme::Sftp, "host", "/srv/www");
        let other_host = VPath::remote(Scheme::Sftp, "host2", "/srv");
        assert!(remote.starts_with(&VPath::remote(Scheme::Sftp, "host", "/srv")));
        assert!(remote.starts_with(&remote));
        assert!(!remote.starts_with(&local));
        assert!(!remote.starts_with(&other_host));
        // `/srv/www2` is not inside `/srv/www`.
        assert!(
            !VPath::remote(Scheme::Sftp, "host", "/srv/www2").starts_with(&remote),
            "a name prefix is not a path prefix"
        );
        assert!(local.starts_with(&VPath::local("/srv")));
    }
}
