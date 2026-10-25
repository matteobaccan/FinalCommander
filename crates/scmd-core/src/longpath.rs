//! Windows' `MAX_PATH`, and the one way round it.
//!
//! A Windows API call given a path of 260 characters or more fails — `ERROR_FILENAME_EXCED_RANGE`
//! or `ERROR_PATH_NOT_FOUND` — however much room the filesystem itself has. NTFS has allowed
//! 32,767 characters since 1993; it is the Win32 layer in front of it that does not, and a
//! file manager meets the difference every day: `node_modules` nested four deep, a build tree,
//! a path from a server that was fine where it came from.
//!
//! There is one way round it that works on every Windows this program runs on: the **verbatim**
//! prefix `\\?\`, which tells Win32 to hand the path to the filesystem as it is — no `MAX_PATH`,
//! no `.` and `..` resolution, no turning `/` into `\`. The other way, a manifest declaring the
//! process long-path aware, also needs a registry setting on the machine, so it is not
//! something a program can rely on by itself.
//!
//! So: every path this program hands to the operating system goes through [`os_path`] first. On
//! Windows it adds the prefix to an absolute path that needs it; on every other platform it is
//! the identity and compiles to nothing.
//!
//! What the prefix costs, and why it is only added when it is needed: a verbatim path is not
//! normalised by Windows, so a `.` or a `..` left in it would reach the filesystem as a
//! *name* rather than as a step. [`os_path`] therefore refuses to prefix a path with either in
//! it, and such a path keeps whatever `MAX_PATH` allows it — which is the behaviour it had
//! before, rather than a new way to fail.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

/// Where Windows stops. A directory is allowed 12 characters less than a file (`MAX_PATH - 12`),
/// because anything in it has to fit too, so the prefix goes on well before 260.
const LIMIT: usize = 240;

/// The path as it should be given to the operating system.
///
/// On Windows, an absolute path at or over [`LIMIT`] characters comes back with the verbatim
/// prefix; everything else, and every path on every other platform, is borrowed unchanged.
#[cfg(windows)]
pub fn os_path(path: &Path) -> Cow<'_, Path> {
    let text = path.as_os_str();
    if text.len() < LIMIT {
        return Cow::Borrowed(path);
    }
    // Already verbatim: `\\?\C:\x` or `\\?\UNC\server\share`. Prefixing it again would make a
    // path with the prefix in it as a name.
    if matches!(
        path.components().next(),
        Some(Component::Prefix(prefix)) if prefix.kind().is_verbatim()
    ) {
        return Cow::Borrowed(path);
    }
    // A relative path cannot be made verbatim — the prefix means "exactly this, from the root"
    // — and neither can one holding `.` or `..`, which Windows would stop resolving.
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Cow::Borrowed(path);
    }
    let text = path.to_string_lossy();
    // `\\server\share` becomes `\\?\UNC\server\share`: the two leading slashes are the share's
    // own, and the prefix replaces them.
    let prefixed = match text.strip_prefix(r"\\") {
        Some(rest) => format!(r"\\?\UNC\{rest}"),
        None => format!(r"\\?\{text}"),
    };
    Cow::Owned(PathBuf::from(prefixed))
}

/// On everything but Windows there is no `MAX_PATH` and nothing to do.
#[cfg(not(windows))]
pub fn os_path(path: &Path) -> Cow<'_, Path> {
    // Named so the Windows version's imports are not dead code here.
    let _ = (LIMIT, Component::CurDir, PathBuf::new());
    Cow::Borrowed(path)
}

/// Whether a path is long enough to need the prefix on Windows. For a message that wants to
/// say why something was refused, and for the tests.
pub fn is_long(path: &Path) -> bool {
    path.as_os_str().len() >= LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path with `deep` components of `part`, under `root`.
    fn deep(root: &str, part: &str, times: usize) -> PathBuf {
        let mut path = PathBuf::from(root);
        for _ in 0..times {
            path.push(part);
        }
        path
    }

    #[test]
    fn a_short_path_is_handed_over_untouched_everywhere() {
        for text in ["/tmp/x", r"C:\Users\mb\notes.txt", "relative/x"] {
            let path = Path::new(text);
            assert!(matches!(os_path(path), Cow::Borrowed(_)), "{text}");
            assert_eq!(os_path(path).as_ref(), path, "{text}");
        }
        assert!(!is_long(Path::new("/tmp/x")));
    }

    #[test]
    fn a_long_path_is_recognised_as_long_on_every_platform() {
        // The *recognition* is platform-independent, which is what lets the rest of the
        // program say "this is a long path" in a message on any machine.
        let long = deep("/tmp", "a-directory-with-a-long-name", 12);
        assert!(long.as_os_str().len() >= 240, "{}", long.display());
        assert!(is_long(&long));
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_a_long_absolute_path_gets_the_verbatim_prefix() {
        let long = deep(r"C:\work", "a-directory-with-a-long-name", 12);
        let os = os_path(&long);
        let text = os.to_string_lossy();
        assert!(text.starts_with(r"\\?\C:\work"), "{text}");
        // And prefixing it again does nothing: the second call sees it is already verbatim.
        let again = os_path(os.as_ref());
        assert_eq!(again.as_ref(), os.as_ref());

        // A share becomes `\\?\UNC\…`, with the two leading slashes replaced.
        let share = deep(r"\\server\share", "a-directory-with-a-long-name", 12);
        let text = os_path(&share).to_string_lossy().into_owned();
        assert!(text.starts_with(r"\\?\UNC\server\share"), "{text}");
        assert!(!text.starts_with(r"\\?\\\"), "{text}");
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_a_path_that_cannot_be_verbatim_is_left_alone() {
        // Relative: the prefix means "from the root", so there is nothing to prefix.
        let relative = deep("work", "a-directory-with-a-long-name", 12);
        assert!(matches!(os_path(&relative), Cow::Borrowed(_)));

        // With `..` in it: a verbatim path is not normalised, so the prefix would turn the
        // `..` into a directory called `..` and the path would stop working rather than start.
        let mut dots = deep(r"C:\work", "a-directory-with-a-long-name", 12);
        dots.push("..");
        dots.push("x");
        assert!(matches!(os_path(&dots), Cow::Borrowed(_)));
    }
}
