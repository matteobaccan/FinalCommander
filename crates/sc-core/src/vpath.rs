use std::ffi::OsStr;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// Which provider owns a path. M1 only implements [`Scheme::Local`]; the enum exists so that
/// nothing in the UI assumes a local path (see `docs/spec/04-vfs.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scheme {
    Local,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Scheme::Local => "file",
        }
    }
}

/// A VFS-aware path.
///
/// Names are kept as [`OsString`] so that non-UTF-8 names on Unix and the odd UTF-16 name on
/// Windows survive a round trip; display is lossy and only used for rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VPath {
    scheme: Scheme,
    path: PathBuf,
}

impl VPath {
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self {
            scheme: Scheme::Local,
            path: path.into(),
        }
    }

    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }

    pub fn file_name(&self) -> Option<&OsStr> {
        self.path.file_name()
    }

    /// The name to show in a panel header: the last component, or the whole path for roots.
    pub fn display_name(&self) -> String {
        match self.path.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => self.to_string(),
        }
    }

    pub fn parent(&self) -> Option<VPath> {
        self.path.parent().map(|p| VPath {
            scheme: self.scheme,
            path: p.to_path_buf(),
        })
    }

    pub fn join(&self, name: impl AsRef<OsStr>) -> VPath {
        VPath {
            scheme: self.scheme,
            path: self.path.join(name.as_ref()),
        }
    }

    /// The filesystem root of this path (`/`, `C:\`, …).
    pub fn root(&self) -> VPath {
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
        VPath {
            scheme: self.scheme,
            path: root,
        }
    }

    pub fn is_root(&self) -> bool {
        self.path.parent().is_none()
    }

    /// Normalise `.`/`..` lexically, without touching the filesystem.
    pub fn normalised(&self) -> VPath {
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
        VPath {
            scheme: self.scheme,
            path: out,
        }
    }
}

impl fmt::Display for VPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_and_root() {
        let p = VPath::local("/usr/local/bin");
        assert_eq!(p.parent().unwrap().to_string(), "/usr/local");
        assert_eq!(p.root().to_string(), "/");
        assert!(VPath::local("/").is_root());
        assert!(!p.is_root());
    }

    #[test]
    fn normalisation_is_lexical() {
        assert_eq!(
            VPath::local("/a/b/../c/./d").normalised().to_string(),
            "/a/c/d"
        );
    }

    #[test]
    fn display_name_falls_back_to_the_whole_path() {
        assert_eq!(VPath::local("/etc/hosts").display_name(), "hosts");
        assert_eq!(VPath::local("/").display_name(), "/");
    }
}
