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
    /// Object storage that speaks the S3 protocol — AWS itself, and MinIO, Ceph, R2, Wasabi
    /// and the rest. The authority is `[profile@]bucket` rather than a host: which host to
    /// talk to is a matter of configuration, while which bucket a path is in is part of the
    /// path. An empty bucket (`s3://work@/`) is the list of buckets the credentials can see.
    S3,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Scheme::Local => "file",
            Scheme::Sftp => "sftp",
            Scheme::S3 => "s3",
        }
    }

    pub fn parse(text: &str) -> Option<Scheme> {
        match text.to_ascii_lowercase().as_str() {
            "file" | "local" => Some(Scheme::Local),
            "sftp" | "ssh" | "scp" => Some(Scheme::Sftp),
            "s3" => Some(Scheme::S3),
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
    /// Inside a container: one entry per nesting level, each a `/`-separated path within the
    /// container the level before it names (SPEC 04 §4). `sftp://srv/x.tar.gz!/etc!/nginx.conf`
    /// is `path = /x.tar.gz` with the levels `["/etc", "/nginx.conf"]`.
    ///
    /// Behind a pointer, and `None` for an ordinary path: a `VPath` is cloned on every row of
    /// every listing, so the common case pays one word and no allocation.
    inner: Option<Arc<Vec<Arc<str>>>>,
}

impl VPath {
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self {
            scheme: Scheme::Local,
            authority: None,
            path: path.into(),
            inner: None,
        }
    }

    /// A path on a remote provider: `VPath::remote(Scheme::Sftp, "mb@host:22", "/srv/www")`.
    pub fn remote(scheme: Scheme, authority: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        Self {
            scheme,
            authority: Some(Arc::from(authority.as_ref())),
            path: PathBuf::from(normalise_remote(path.as_ref())),
            inner: None,
        }
    }

    /// Parse `sftp://mb@host:22/srv/www`. Anything that is not a known remote URL is a local
    /// path, so a Windows path (`C:\x`) and a file with a colon in its name still work.
    pub fn parse(text: &str) -> VPath {
        // `archive!/inside` opens a container (SPEC 04 §4). The separator is `!` followed by a
        // path separator, which no single file name can contain; a directory actually named
        // `x!` is the one ambiguity, and the syntax is the one FAR users already type.
        if let Some(cut) = text.find("!/").or_else(|| text.find("!\\")) {
            let mut base = VPath::parse(&text[..cut]);
            for level in text[cut..].split('!').skip(1) {
                base = base.enter(level);
            }
            return base;
        }
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
        self.scheme == other.scheme
            && self.authority == other.authority
            // Two paths in the same archive share a provider; two archives do not, and neither
            // do an archive and the directory it sits in.
            && self.depth() == other.depth()
            && (!self.in_container()
                || (self.path == other.path
                    && self.inner()[..self.depth() - 1] == other.inner()[..other.depth() - 1]))
    }

    /// The levels of container this path is inside; empty for an ordinary path.
    pub fn inner(&self) -> &[Arc<str>] {
        match &self.inner {
            Some(levels) => levels,
            None => &[],
        }
    }

    /// Is this path inside a container (an archive)?
    pub fn in_container(&self) -> bool {
        self.inner.is_some()
    }

    /// How many containers deep, for the nesting limit of SPEC 04 §4.
    pub fn depth(&self) -> usize {
        self.inner().len()
    }

    /// The path *within* the innermost container, `/`-separated; `/` at its root. Empty for a
    /// path that is not in a container.
    pub fn inner_path(&self) -> &str {
        self.inner().last().map(|p| &**p).unwrap_or("")
    }

    /// Step into the container this path names, landing on `inside` (`/` for its root).
    pub fn enter(&self, inside: impl AsRef<str>) -> VPath {
        let mut levels = self.inner().to_vec();
        levels.push(Arc::from(normalise_remote(inside.as_ref()).as_str()));
        self.with_levels(levels)
    }

    /// Leave the innermost container: the path of the container file itself, so the panel lands
    /// in the directory that holds it with the cursor on it. `None` outside a container.
    pub fn leave(&self) -> Option<VPath> {
        let mut levels = self.inner().to_vec();
        levels.pop()?;
        Some(self.with_levels(levels))
    }

    /// The same place within the innermost container, at `inside`.
    pub fn with_inner(&self, inside: impl AsRef<str>) -> VPath {
        match self.leave() {
            Some(host) => host.enter(inside),
            None => self.clone(),
        }
    }

    pub fn file_name(&self) -> Option<&OsStr> {
        if self.in_container() {
            let name = self.inner_path().rsplit('/').next().unwrap_or("");
            if !name.is_empty() {
                return Some(OsStr::new(name));
            }
            // At the container's root the name is the container's own, so fall through.
        }
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
        if self.in_container() {
            let path = self.inner_path();
            if path == "/" {
                // Up from the root of an archive is out of it, onto the archive itself — which
                // is what `leave` gives, and then its own parent is the directory holding it.
                return self.leave().and_then(|host| host.parent());
            }
            let cut = path.rfind('/')?;
            let parent = if cut == 0 { "/" } else { &path[..cut] };
            return Some(self.with_inner(parent));
        }
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
        if self.in_container() {
            let name = name.as_ref().to_string_lossy();
            let base = self.inner_path();
            let joined = if base.ends_with('/') {
                format!("{base}{name}")
            } else {
                format!("{base}/{name}")
            };
            return self.with_inner(&joined);
        }
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
        if self.in_container() {
            return self.with_inner("/");
        }
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
        // The root of an archive has a parent — the directory the archive lives in — so it is
        // not a root, and `..` stays on the panel to get out.
        if self.in_container() {
            return false;
        }
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
        if self.in_container() {
            let (a, b) = (self.inner_path(), other.inner_path());
            return a == b
                || a.starts_with(b)
                    && (b.ends_with('/') || a.as_bytes().get(b.len()) == Some(&b'/'));
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
        if self.in_container() {
            let mut out: Vec<&str> = Vec::new();
            for part in self.inner_path().split('/') {
                match part {
                    "" | "." => {}
                    ".." => {
                        out.pop();
                    }
                    other => out.push(other),
                }
            }
            return self.with_inner(format!("/{}", out.join("/")));
        }
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

    fn with_levels(&self, levels: Vec<Arc<str>>) -> VPath {
        VPath {
            scheme: self.scheme,
            authority: self.authority.clone(),
            path: self.path.clone(),
            inner: (!levels.is_empty()).then(|| Arc::new(levels)),
        }
    }

    fn with_path(&self, path: PathBuf) -> VPath {
        VPath {
            scheme: self.scheme,
            authority: self.authority.clone(),
            path,
            inner: self.inner.clone(),
        }
    }

    fn with_remote(&self, path: &str) -> VPath {
        VPath {
            scheme: self.scheme,
            authority: self.authority.clone(),
            path: PathBuf::from(normalise_remote(path)),
            inner: self.inner.clone(),
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
            )?,
            None => write!(f, "{}", self.path.display())?,
        }
        for level in self.inner() {
            write!(f, "!{level}")?;
        }
        Ok(())
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

    // ── containers (SPEC 04 §4) ─────────────────────────────────────────────────────────────

    #[test]
    fn a_container_path_round_trips_through_the_bang_syntax() {
        let p = VPath::parse("/home/mb/src.zip!/doc/readme.md");
        assert!(p.in_container() && p.is_local());
        assert_eq!(p.as_path(), std::path::Path::new("/home/mb/src.zip"));
        assert_eq!(p.inner_path(), "/doc/readme.md");
        assert_eq!(p.depth(), 1);
        assert_eq!(p.display_name(), "readme.md");
        assert_eq!(p.to_string(), "/home/mb/src.zip!/doc/readme.md");

        // Nesting, and over a remote path too.
        let n = VPath::parse("sftp://srv/backups/x.tar.gz!/etc!/nginx.conf");
        assert_eq!(n.depth(), 2);
        assert_eq!(n.remote_path(), "/backups/x.tar.gz");
        assert_eq!(n.inner(), ["/etc".into(), "/nginx.conf".into()]);
        assert_eq!(
            n.to_string(),
            "sftp://srv/backups/x.tar.gz!/etc!/nginx.conf"
        );

        // A path with no `!/` in it is untouched, so an ordinary name keeps working.
        assert!(!VPath::parse("/home/mb/hello!.txt").in_container());
    }

    #[test]
    fn walking_inside_a_container_stays_inside_it_until_the_root() {
        let root = VPath::local("/a/x.zip").enter("/");
        assert_eq!(root.inner_path(), "/");
        // The frame of a panel at the root of an archive says the archive's name.
        assert_eq!(root.display_name(), "x.zip");
        assert_eq!(root.to_string(), "/a/x.zip!/");

        let file = root.join("dir").join("f.txt");
        assert_eq!(file.inner_path(), "/dir/f.txt");
        assert_eq!(file.as_path(), std::path::Path::new("/a/x.zip"));
        assert_eq!(file.parent().unwrap().inner_path(), "/dir");
        assert_eq!(file.parent().unwrap().parent().unwrap(), root);
        assert_eq!(file.root(), root);
        // An archive root is not a root: `..` has somewhere to go, which is out.
        assert!(!root.is_root());
        assert_eq!(root.parent(), Some(VPath::local("/a")));
        assert_eq!(root.leave(), Some(VPath::local("/a/x.zip")));
        assert_eq!(VPath::local("/a/x.zip").leave(), None);
        // `..` inside is lexical, like everywhere else.
        assert_eq!(
            root.join("dir").join("..").join("g.txt").normalised(),
            root.join("g.txt")
        );
    }

    #[test]
    fn two_archives_are_two_providers() {
        let one = VPath::local("/a/x.zip").enter("/f.txt");
        let two = VPath::local("/a/y.zip").enter("/f.txt");
        let outside = VPath::local("/a/x.zip");
        assert!(one.same_provider(&one.with_inner("/other.txt")));
        assert!(!one.same_provider(&two));
        // The archive and the directory it sits in are not one provider either: a copy out of
        // it has to go through the job engine, not through a rename.
        assert!(!one.same_provider(&outside));
        assert!(one.starts_with(&one.root()));
        assert!(!one.starts_with(&two.root()));
        assert!(!one.starts_with(&outside));
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
