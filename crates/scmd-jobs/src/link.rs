//! Creating a link to a file that is already there: FAR's `Alt+F6` (`docs/BACKLOG.md`, the
//! "Files" section).
//!
//! This is not a job. There is nothing to plan, nothing to count and nothing to cancel, so it
//! is one call that either works or says exactly which path was wrong and why.

use std::io;
use std::path::{Component, Path, PathBuf};

use scmd_core::VPath;

use crate::fs::JobFs;

/// Which kind of link to make, and for a symbolic link what it should say.
///
/// Relative or absolute is a real choice rather than a detail, which is why the dialog offers
/// it: a relative link survives the whole tree being moved, copied or mounted somewhere else,
/// an absolute one survives the link alone being moved to another directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// A second name for the same file. Local filesystem only, and never for a directory.
    Hard,
    SymlinkAbsolute,
    SymlinkRelative,
}

impl LinkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkKind::Hard => "hard",
            LinkKind::SymlinkAbsolute => "symlink-absolute",
            LinkKind::SymlinkRelative => "symlink-relative",
        }
    }

    pub fn is_symlink(self) -> bool {
        !matches!(self, LinkKind::Hard)
    }
}

/// Every way a link can be refused. Each one names the path it is about, because these go
/// straight into the dialog the user is looking at.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("{path}: a link can only be created on a local filesystem")]
    NotLocal { path: VPath },

    #[error("{path}: already exists")]
    Exists { path: VPath },

    #[error("{path}: there is nothing here to link to")]
    NoTarget { path: VPath },

    #[error("{path}: a hard link to a directory is not allowed")]
    HardLinkToDirectory { path: VPath },

    #[error("{path}: a symbolic link needs an absolute path to work from")]
    NotAbsolute { path: VPath },

    #[error("{target}: no relative spelling of this path from {from}")]
    NotRelatable { target: VPath, from: VPath },

    #[error("{path}: {source}")]
    Io {
        path: VPath,
        #[source]
        source: io::Error,
    },
}

/// Create `link`, pointing at `target`.
///
/// Both paths must be local: a hard link is a second directory entry for one inode, and a
/// symbolic link means whatever the filesystem holding it decides it means, so neither
/// crosses a provider. A path on a remote or archive filesystem is refused by name rather
/// than attempted.
///
/// An existing `link` is never replaced, which is how the copy and move operations behave on
/// a conflict: the caller asks the user and then calls again with another name.
///
/// A symbolic link is allowed to point at something that is not there — occasionally that is
/// the point, and the panel marks the result as broken — so only the hard link insists on an
/// existing target.
pub fn create_link(
    fs: &dyn JobFs,
    target: &VPath,
    link: &VPath,
    kind: LinkKind,
) -> Result<(), LinkError> {
    for path in [target, link] {
        if !path.is_local() {
            return Err(LinkError::NotLocal { path: path.clone() });
        }
    }
    if fs.exists(link) {
        return Err(LinkError::Exists { path: link.clone() });
    }

    match kind {
        LinkKind::Hard => {
            // The OS refuses both of these anyway; asking first is what lets the message name
            // the target instead of reporting `EPERM` on the link.
            let facts = match fs.facts(target) {
                Ok(facts) => facts,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    return Err(LinkError::NoTarget {
                        path: target.clone(),
                    })
                }
                Err(source) => {
                    return Err(LinkError::Io {
                        path: target.clone(),
                        source,
                    })
                }
            };
            if facts.is_dir() {
                return Err(LinkError::HardLinkToDirectory {
                    path: target.clone(),
                });
            }
            fs.hard_link(target, link).map_err(|source| LinkError::Io {
                path: link.clone(),
                source,
            })
        }

        LinkKind::SymlinkAbsolute | LinkKind::SymlinkRelative => {
            // Neither spelling can be worked out from a relative path: one would be a lie and
            // the other has no directory to count from.
            for path in [target, link] {
                if !path.as_path().is_absolute() {
                    return Err(LinkError::NotAbsolute { path: path.clone() });
                }
            }
            let text = if kind == LinkKind::SymlinkRelative {
                let from = link.parent().ok_or_else(|| LinkError::NotRelatable {
                    target: target.clone(),
                    from: link.clone(),
                })?;
                relative_path(from.normalised().as_path(), target.normalised().as_path())
                    .ok_or_else(|| LinkError::NotRelatable {
                        target: target.clone(),
                        from,
                    })?
            } else {
                target.normalised().as_path().to_path_buf()
            };
            fs.symlink(&text, link).map_err(|source| LinkError::Io {
                path: link.clone(),
                source,
            })
        }
    }
}

/// How a symbolic link kept in `from_dir` has to spell `to`.
///
/// Lexical, and deliberately so: a link may point at something that is not there yet, which
/// leaves nothing to canonicalise. Both paths must already be absolute and normalised.
/// `None` means there is no relative spelling at all — two Windows drives share no root to
/// count from.
fn relative_path(from_dir: &Path, to: &Path) -> Option<PathBuf> {
    let from: Vec<Component> = from_dir.components().collect();
    let to: Vec<Component> = to.components().collect();
    if from.first() != to.first() {
        return None;
    }
    let common = from
        .iter()
        .zip(to.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut out = PathBuf::new();
    // One `..` for every directory between the link and the point the two paths part.
    for _ in common..from.len() {
        out.push(Component::ParentDir.as_os_str());
    }
    for part in &to[common..] {
        out.push(part.as_os_str());
    }
    // The target is the link's own directory, which `ln -s . x` is a legal way to ask for.
    if out.as_os_str().is_empty() {
        out.push(Component::CurDir.as_os_str());
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LocalFs;
    use scmd_core::Scheme;
    use std::fs as stdfs;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        stdfs::create_dir_all(dir.path().join("a/b")).unwrap();
        stdfs::create_dir_all(dir.path().join("a/c")).unwrap();
        stdfs::write(dir.path().join("a/b/target.txt"), b"shared").unwrap();
        dir
    }

    #[test]
    fn a_hard_link_is_a_second_name_for_the_same_bytes() {
        let dir = tree();
        let target = VPath::local(dir.path().join("a/b/target.txt"));
        let link = VPath::local(dir.path().join("a/c/same.txt"));
        create_link(&LocalFs, &target, &link, LinkKind::Hard).unwrap();

        stdfs::write(target.as_path(), b"rewritten").unwrap();
        assert_eq!(stdfs::read(link.as_path()).unwrap(), b"rewritten");
        assert!(
            !stdfs::symlink_metadata(link.as_path())
                .unwrap()
                .is_symlink(),
            "a hard link is not a symlink"
        );
    }

    #[test]
    fn a_relative_symlink_counts_from_the_directory_it_lives_in() {
        let dir = tree();
        let target = VPath::local(dir.path().join("a/b/target.txt"));
        let link = VPath::local(dir.path().join("a/c/rel.txt"));
        create_link(&LocalFs, &target, &link, LinkKind::SymlinkRelative).unwrap();

        let expected: PathBuf = ["..", "b", "target.txt"].iter().collect();
        assert_eq!(LocalFs.read_link(&link).unwrap(), expected);
        assert_eq!(stdfs::read(link.as_path()).unwrap(), b"shared");

        // The whole tree moving is what a relative link is for: it still reads.
        stdfs::rename(dir.path().join("a"), dir.path().join("moved")).unwrap();
        assert_eq!(
            stdfs::read(dir.path().join("moved/c/rel.txt")).unwrap(),
            b"shared"
        );
    }

    #[test]
    fn an_absolute_symlink_spells_the_whole_path() {
        let dir = tree();
        let target = VPath::local(dir.path().join("a/b/target.txt"));
        let link = VPath::local(dir.path().join("a/c/abs.txt"));
        create_link(&LocalFs, &target, &link, LinkKind::SymlinkAbsolute).unwrap();
        assert_eq!(LocalFs.read_link(&link).unwrap(), target.as_path());
        assert_eq!(stdfs::read(link.as_path()).unwrap(), b"shared");
    }

    #[test]
    fn a_symlink_may_point_at_what_is_not_there_yet() {
        let dir = tree();
        let target = VPath::local(dir.path().join("a/b/later.txt"));
        let link = VPath::local(dir.path().join("a/c/early.txt"));
        create_link(&LocalFs, &target, &link, LinkKind::SymlinkRelative).unwrap();
        assert!(stdfs::read(link.as_path()).is_err(), "it points at nothing");
        stdfs::write(target.as_path(), b"here now").unwrap();
        assert_eq!(stdfs::read(link.as_path()).unwrap(), b"here now");
    }

    #[test]
    fn an_existing_name_is_never_quietly_replaced() {
        let dir = tree();
        let target = VPath::local(dir.path().join("a/b/target.txt"));
        let link = VPath::local(dir.path().join("a/c/taken.txt"));
        stdfs::write(link.as_path(), b"mine").unwrap();
        for kind in [
            LinkKind::Hard,
            LinkKind::SymlinkAbsolute,
            LinkKind::SymlinkRelative,
        ] {
            let err = create_link(&LocalFs, &target, &link, kind).unwrap_err();
            assert!(matches!(err, LinkError::Exists { .. }), "{kind:?}: {err}");
        }
        assert_eq!(stdfs::read(link.as_path()).unwrap(), b"mine");
    }

    #[test]
    fn a_remote_path_is_refused_by_name() {
        let dir = tree();
        let local = VPath::local(dir.path().join("a/b/target.txt"));
        let remote = VPath::remote(Scheme::Sftp, "host", "/srv/www");
        let err = create_link(&LocalFs, &remote, &local, LinkKind::Hard).unwrap_err();
        assert!(matches!(err, LinkError::NotLocal { .. }), "{err}");
        let err = create_link(&LocalFs, &local, &remote, LinkKind::SymlinkAbsolute).unwrap_err();
        assert!(matches!(err, LinkError::NotLocal { .. }), "{err}");
    }

    #[test]
    fn a_hard_link_needs_a_file_that_is_there_and_is_not_a_directory() {
        let dir = tree();
        let err = create_link(
            &LocalFs,
            &VPath::local(dir.path().join("a/b/ghost.txt")),
            &VPath::local(dir.path().join("a/c/x.txt")),
            LinkKind::Hard,
        )
        .unwrap_err();
        assert!(matches!(err, LinkError::NoTarget { .. }), "{err}");

        let err = create_link(
            &LocalFs,
            &VPath::local(dir.path().join("a/b")),
            &VPath::local(dir.path().join("a/c/dirlink")),
            LinkKind::Hard,
        )
        .unwrap_err();
        assert!(
            matches!(err, LinkError::HardLinkToDirectory { .. }),
            "{err}"
        );
    }

    #[test]
    fn a_symlink_from_a_relative_path_is_refused_rather_than_guessed() {
        let dir = tree();
        let err = create_link(
            &LocalFs,
            &VPath::local("b/target.txt"),
            &VPath::local(dir.path().join("a/c/rel.txt")),
            LinkKind::SymlinkRelative,
        )
        .unwrap_err();
        assert!(matches!(err, LinkError::NotAbsolute { .. }), "{err}");
    }

    #[test]
    fn the_relative_walk_is_lexical_and_handles_its_own_directory() {
        let rel = |from: &str, to: &str| {
            relative_path(
                VPath::local(from).normalised().as_path(),
                VPath::local(to).normalised().as_path(),
            )
            .map(|p| p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"))
        };
        assert_eq!(rel("/a/c", "/a/b/t.txt").as_deref(), Some("../b/t.txt"));
        assert_eq!(rel("/a", "/a/b/t.txt").as_deref(), Some("b/t.txt"));
        assert_eq!(rel("/a/b/c", "/t.txt").as_deref(), Some("../../../t.txt"));
        assert_eq!(rel("/a/b", "/a/b").as_deref(), Some("."));
        // A name prefix is not a path prefix: `/ab` is not inside `/a`.
        assert_eq!(rel("/a", "/ab/t.txt").as_deref(), Some("../ab/t.txt"));
    }
}
