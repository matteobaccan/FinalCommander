//! A shared folder, and a caller's path resolved under it (SPEC 04 §9.6).
//!
//! The caller's path is always taken as relative to the share, whatever it looks like, and
//! refused — on this side, whatever the caller sends — if it climbs out: by `..`, by a drive
//! or a prefix, or through a symbolic link that leads outside.

use std::io;
use std::path::{Component, Path, PathBuf};

/// One folder offered to callers.
#[derive(Debug, Clone)]
pub struct Share {
    /// What the caller asks for and its panel shows.
    pub name: String,
    /// The folder, canonical.
    pub root: PathBuf,
    /// Whether a caller may be let in to write (step 2; read only for now).
    pub write: bool,
}

impl Share {
    /// A share of `root`, which must exist.
    pub fn new(name: &str, root: &Path, write: bool) -> io::Result<Share> {
        Ok(Share {
            name: name.to_string(),
            root: root.canonicalize()?,
            write,
        })
    }

    /// The real path a caller's `/`-separated path names, or why not.
    pub fn resolve(&self, wire: &str) -> io::Result<PathBuf> {
        let mut out = self.root.clone();
        for part in wire.split('/') {
            match part {
                "" | "." => {}
                ".." => return Err(refused(wire)),
                other => {
                    // A part that is itself a path (`C:`, `\\server`, `a\b`) is refused rather
                    // than joined, since `join` would honour it.
                    let p = Path::new(other);
                    let plain = p.components().count() == 1
                        && matches!(p.components().next(), Some(Component::Normal(_)))
                        && !other.contains('\\');
                    if !plain {
                        return Err(refused(wire));
                    }
                    out.push(other);
                }
            }
        }
        // A symbolic link may still lead out: what exists is checked where it really is, and
        // for what does not exist yet, its folder is.
        let real = match out.canonicalize() {
            Ok(real) => real,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let parent = out.parent().map(Path::to_path_buf).unwrap_or_default();
                let real_parent = parent.canonicalize()?;
                if !real_parent.starts_with(&self.root) {
                    return Err(refused(wire));
                }
                return Ok(out);
            }
            Err(e) => return Err(e),
        };
        if !real.starts_with(&self.root) {
            return Err(refused(wire));
        }
        Ok(out)
    }
}

fn refused(wire: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("{wire}: outside the shared folder"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_resolves_inside_and_nothing_climbs_out() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("shared");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/a.txt"), b"a").unwrap();
        std::fs::write(dir.path().join("secret.txt"), b"s").unwrap();
        let share = Share::new("shared", &root, false).unwrap();

        assert_eq!(
            share.resolve("/sub/a.txt").unwrap(),
            share.root.join("sub/a.txt")
        );
        assert_eq!(share.resolve("/").unwrap(), share.root);
        assert_eq!(
            share.resolve("sub/./a.txt").unwrap(),
            share.root.join("sub/a.txt")
        );
        assert!(
            share.resolve("/sub/new.txt").is_ok(),
            "a file still to be made"
        );

        assert!(share.resolve("/../secret.txt").is_err());
        assert!(share.resolve("/sub/../../secret.txt").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path().join("secret.txt"), root.join("out")).unwrap();
            std::os::unix::fs::symlink(dir.path(), root.join("outdir")).unwrap();
            assert!(share.resolve("/out").is_err(), "a link that leads outside");
            assert!(share.resolve("/outdir/secret.txt").is_err());
            assert!(share.resolve("/outdir/new.txt").is_err());
        }
        #[cfg(windows)]
        {
            assert!(share.resolve("/C:/Windows").is_err());
            assert!(share.resolve("/a\\..\\..\\secret.txt").is_err());
        }
    }
}
