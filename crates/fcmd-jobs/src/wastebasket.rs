//! The system's wastebasket as a place: what is in it, putting something back, deleting for
//! good (SPEC 03 §4a).
//!
//! Sending a file there is the engine's business ([`crate::JobFs::trash`]); this is the other
//! direction, and it is kept behind a trait for one reason: the real wastebasket is the
//! user's, and no test may list it, restore from it or empty it. [`restore`] — the part with
//! decisions in it, what to do when something already stands where an item came from — works
//! on any [`Wastebasket`], and the tests hand it one made of two temporary folders.
//!
//! The freedesktop trash and the Windows recycle bin can be listed and restored from. The
//! macOS one cannot from outside the Finder, so [`SUPPORTED`] is false there and every call
//! says so rather than answering with an empty list.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// One thing in the wastebasket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trashed {
    /// What the system calls it: the `.trashinfo` file on Linux, the shell's parsing name on
    /// Windows. Only the wastebasket that listed it knows what to do with one.
    pub id: OsString,
    /// The name it had.
    pub name: OsString,
    /// The folder it was in.
    pub parent: PathBuf,
    /// When it was thrown away, in seconds since 1970.
    pub deleted: i64,
    /// Its size in bytes; a folder's is not known without walking it, and is 0.
    pub size: u64,
    pub folder: bool,
}

impl Trashed {
    /// Where it was, and where putting it back puts it.
    pub fn original(&self) -> PathBuf {
        self.parent.join(&self.name)
    }
}

/// A wastebasket: the system's, or a test's.
pub trait Wastebasket {
    /// Everything in it, in no order.
    fn list(&self) -> Result<Vec<Trashed>, String>;
    /// Put one item back where it was. Only called when nothing stands there.
    fn put_back(&self, item: &Trashed) -> Result<(), String>;
    /// Delete these for good.
    fn purge(&self, items: &[Trashed]) -> Result<(), String>;
    /// Throw away what stands at `path`: how *overwrite* makes room without losing anything.
    fn throw_away(&self, path: &Path) -> Result<(), String>;
}

/// What to do when something already stands where an item came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnClash {
    /// Leave the item where it is and report it, for the caller to ask about.
    Ask,
    /// The one standing there goes to the wastebasket and the item takes its place: the two
    /// change places, and nothing is lost.
    Overwrite,
    /// The item comes back beside it, as `name (2).ext`.
    Rename,
    /// The item stays in the wastebasket.
    Skip,
}

/// What a [`restore`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Restored {
    /// Where each item that came back now is.
    pub back: Vec<PathBuf>,
    /// The items left alone because something stands in their place ([`OnClash::Ask`]).
    pub clashes: Vec<Trashed>,
    /// How many were passed over ([`OnClash::Skip`]).
    pub skipped: usize,
    /// What went wrong, one line per item, each naming its path.
    pub errors: Vec<String>,
}

/// Put `items` back where they came from.
///
/// Each is looked at when its turn comes, not all at once before: two items thrown away from
/// one path are a clash between themselves, which only shows once the first is back.
pub fn restore(basket: &dyn Wastebasket, items: &[Trashed], clash: OnClash) -> Restored {
    let mut out = Restored::default();
    for item in items {
        let target = item.original();
        // `symlink_metadata`, because a dangling link standing there is still in the way.
        if target.symlink_metadata().is_err() {
            match basket.put_back(item) {
                Ok(()) => out.back.push(target),
                Err(e) => out.errors.push(format!("{}: {e}", target.display())),
            }
            continue;
        }
        let done = match clash {
            OnClash::Ask => {
                out.clashes.push(item.clone());
                continue;
            }
            OnClash::Skip => {
                out.skipped += 1;
                continue;
            }
            OnClash::Overwrite => basket
                .throw_away(&target)
                .and_then(|()| basket.put_back(item))
                .map(|()| target.clone()),
            OnClash::Rename => beside(basket, item, &target),
        };
        match done {
            Ok(path) => out.back.push(path),
            Err(e) => out.errors.push(format!("{}: {e}", target.display())),
        }
    }
    out
}

/// Bring `item` back beside what stands at `target`, under the first free `name (n).ext`.
///
/// A wastebasket only knows how to put a thing back where it was, so the one standing there
/// steps aside for a moment under a name nothing else has, the item comes back and takes its
/// new name, and the first returns to its own. Every step is a rename inside one folder; if
/// one fails, what stepped aside is put back before the error is reported.
fn beside(basket: &dyn Wastebasket, item: &Trashed, target: &Path) -> Result<PathBuf, String> {
    let parent = target.parent().ok_or("no folder to put it back into")?;
    let name = item.name.to_string_lossy();
    let free = free_beside(parent, &name).ok_or("no free name beside it")?;
    let aside = (0..10_000u32)
        .map(|n| parent.join(format!(".{name}.fcmd-aside-{n}")))
        .find(|p| p.symlink_metadata().is_err())
        .ok_or("no free name beside it")?;
    std::fs::rename(target, &aside).map_err(|e| e.to_string())?;
    let came = basket
        .put_back(item)
        .and_then(|()| std::fs::rename(target, &free).map_err(|e| e.to_string()));
    // Whatever happened, the one that was there goes back — unless the item came back and
    // could not be moved on, in which case its place is taken and the name stays aside,
    // which the error says.
    let returned = match target.symlink_metadata() {
        Err(_) => std::fs::rename(&aside, target).map_err(|e| e.to_string()),
        Ok(_) => Err(format!("what was there is now {}", aside.display())),
    };
    came.and(returned).map(|()| free)
}

/// `name (2).ext`, `name (3).ext`… — the first that nothing in `parent` has.
fn free_beside(parent: &Path, name: &str) -> Option<PathBuf> {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..10_000u32)
        .map(|n| parent.join(format!("{stem} ({n}){ext}")))
        .find(|p| p.symlink_metadata().is_err())
}

/// The wastebasket of the machine.
#[derive(Debug, Clone, Copy, Default)]
pub struct System;

/// Can this system's wastebasket be listed and restored from by a program?
pub const SUPPORTED: bool = sys::SUPPORTED;

impl Wastebasket for System {
    fn list(&self) -> Result<Vec<Trashed>, String> {
        sys::list()
    }

    fn put_back(&self, item: &Trashed) -> Result<(), String> {
        sys::put_back(item)
    }

    fn purge(&self, items: &[Trashed]) -> Result<(), String> {
        sys::purge(items)
    }

    fn throw_away(&self, path: &Path) -> Result<(), String> {
        ::trash::delete(path).map_err(|e| e.to_string())
    }
}

#[cfg(any(
    target_os = "windows",
    all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android")
    )
))]
mod sys {
    use super::Trashed;
    use ::trash::{os_limited, TrashItem, TrashItemSize};

    pub const SUPPORTED: bool = true;

    /// The crate's own item again: it compares and restores by `id`, and the rest is what
    /// the listing said.
    fn item(t: &Trashed) -> TrashItem {
        TrashItem {
            id: t.id.clone(),
            name: t.name.clone(),
            original_parent: t.parent.clone(),
            time_deleted: t.deleted,
        }
    }

    pub fn list() -> Result<Vec<Trashed>, String> {
        let items = os_limited::list().map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(|i| {
                // An item whose file has gone from under its record still has a row: it is
                // in the wastebasket as far as the system is concerned, and can be purged.
                let size = os_limited::metadata(&i).ok().map(|m| m.size);
                Trashed {
                    folder: matches!(size, Some(TrashItemSize::Entries(_))),
                    size: size.and_then(|s| s.size()).unwrap_or(0),
                    id: i.id,
                    name: i.name,
                    parent: i.original_parent,
                    deleted: i.time_deleted,
                }
            })
            .collect())
    }

    pub fn put_back(t: &Trashed) -> Result<(), String> {
        os_limited::restore_all([item(t)]).map_err(|e| e.to_string())
    }

    pub fn purge(items: &[Trashed]) -> Result<(), String> {
        os_limited::purge_all(items.iter().map(item)).map_err(|e| e.to_string())
    }
}

#[cfg(not(any(
    target_os = "windows",
    all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android")
    )
)))]
mod sys {
    use super::Trashed;

    pub const SUPPORTED: bool = false;

    const WHY: &str = "this system's wastebasket can only be opened from its own file manager";

    pub fn list() -> Result<Vec<Trashed>, String> {
        Err(WHY.into())
    }

    pub fn put_back(_item: &Trashed) -> Result<(), String> {
        Err(WHY.into())
    }

    pub fn purge(_items: &[Trashed]) -> Result<(), String> {
        Err(WHY.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;

    /// A wastebasket made of a folder: an item is a file in it, and its id is that file's
    /// path. Nothing here goes near the system's.
    struct Folder {
        dir: PathBuf,
        thrown: RefCell<Vec<PathBuf>>,
    }

    impl Folder {
        fn new(dir: &Path) -> Folder {
            fs::create_dir_all(dir).unwrap();
            Folder {
                dir: dir.to_path_buf(),
                thrown: RefCell::new(Vec::new()),
            }
        }

        /// Throw `path` away, as the system would have, and say what it became.
        fn take(&self, path: &Path) -> Trashed {
            let n = fs::read_dir(&self.dir).unwrap().count();
            let held = self.dir.join(format!("item-{n}"));
            let size = fs::metadata(path).unwrap().len();
            fs::rename(path, &held).unwrap();
            Trashed {
                id: held.into_os_string(),
                name: path.file_name().unwrap().to_os_string(),
                parent: path.parent().unwrap().to_path_buf(),
                deleted: 1_700_000_000 + n as i64,
                size,
                folder: false,
            }
        }
    }

    impl Wastebasket for Folder {
        fn list(&self) -> Result<Vec<Trashed>, String> {
            Ok(Vec::new())
        }

        fn put_back(&self, item: &Trashed) -> Result<(), String> {
            fs::rename(Path::new(&item.id), item.original()).map_err(|e| e.to_string())
        }

        fn purge(&self, items: &[Trashed]) -> Result<(), String> {
            for item in items {
                fs::remove_file(Path::new(&item.id)).map_err(|e| e.to_string())?;
            }
            Ok(())
        }

        fn throw_away(&self, path: &Path) -> Result<(), String> {
            self.thrown.borrow_mut().push(path.to_path_buf());
            self.take(path);
            Ok(())
        }
    }

    /// A folder of files, a wastebasket beside it, and `notes.txt` thrown away.
    fn thrown_away() -> (tempfile::TempDir, Folder, PathBuf, Trashed) {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let file = home.join("notes.txt");
        fs::write(&file, "the old one").unwrap();
        let basket = Folder::new(&tmp.path().join("basket"));
        let item = basket.take(&file);
        (tmp, basket, file, item)
    }

    #[test]
    fn an_item_goes_back_where_it_was() {
        let (_tmp, basket, file, item) = thrown_away();
        assert!(!file.exists());
        let out = restore(&basket, &[item], OnClash::Ask);
        assert_eq!(out.back, vec![file.clone()]);
        assert!(out.clashes.is_empty() && out.errors.is_empty());
        assert_eq!(fs::read_to_string(&file).unwrap(), "the old one");
    }

    #[test]
    fn something_in_its_place_is_reported_and_nothing_is_touched() {
        let (_tmp, basket, file, item) = thrown_away();
        fs::write(&file, "the new one").unwrap();
        let out = restore(&basket, std::slice::from_ref(&item), OnClash::Ask);
        assert!(out.back.is_empty());
        assert_eq!(out.clashes, vec![item.clone()]);
        assert_eq!(fs::read_to_string(&file).unwrap(), "the new one");
        assert!(Path::new(&item.id).exists(), "still in the wastebasket");
    }

    #[test]
    fn overwrite_changes_places_and_loses_nothing() {
        let (_tmp, basket, file, item) = thrown_away();
        fs::write(&file, "the new one").unwrap();
        let out = restore(&basket, &[item], OnClash::Overwrite);
        assert_eq!(out.back, vec![file.clone()]);
        assert_eq!(fs::read_to_string(&file).unwrap(), "the old one");
        // The one that stood there was thrown away, not deleted.
        assert_eq!(*basket.thrown.borrow(), vec![file.clone()]);
        let kept: Vec<String> = fs::read_dir(&basket.dir)
            .unwrap()
            .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
            .collect();
        assert_eq!(kept, vec!["the new one".to_string()]);
    }

    #[test]
    fn rename_brings_it_back_beside_the_one_that_is_there() {
        let (_tmp, basket, file, item) = thrown_away();
        fs::write(&file, "the new one").unwrap();
        let out = restore(&basket, &[item], OnClash::Rename);
        let beside = file.with_file_name("notes (2).txt");
        assert_eq!(out.back, vec![beside.clone()]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert_eq!(fs::read_to_string(&file).unwrap(), "the new one");
        assert_eq!(fs::read_to_string(&beside).unwrap(), "the old one");
        // Nothing left under the name used while the two passed each other.
        let names: Vec<String> = fs::read_dir(file.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
    }

    #[test]
    fn skip_leaves_it_in_the_wastebasket() {
        let (_tmp, basket, file, item) = thrown_away();
        fs::write(&file, "the new one").unwrap();
        let out = restore(&basket, std::slice::from_ref(&item), OnClash::Skip);
        assert_eq!(out.skipped, 1);
        assert!(out.back.is_empty() && out.clashes.is_empty());
        assert!(Path::new(&item.id).exists());
    }

    #[test]
    fn two_items_from_one_path_clash_with_each_other() {
        let (_tmp, basket, file, first) = thrown_away();
        fs::write(&file, "the second").unwrap();
        let second = basket.take(&file);
        let out = restore(&basket, &[first, second.clone()], OnClash::Ask);
        assert_eq!(out.back, vec![file.clone()]);
        assert_eq!(out.clashes, vec![second]);
        assert_eq!(fs::read_to_string(&file).unwrap(), "the old one");
    }

    #[test]
    fn a_folder_that_has_gone_is_an_error_that_names_the_path() {
        let (_tmp, basket, file, item) = thrown_away();
        fs::remove_dir_all(file.parent().unwrap()).unwrap();
        let out = restore(&basket, &[item], OnClash::Ask);
        assert!(out.back.is_empty());
        assert_eq!(out.errors.len(), 1);
        assert!(
            out.errors[0].starts_with(&file.display().to_string()),
            "{:?}",
            out.errors
        );
    }

    /// The real one, for a person to run on a machine whose wastebasket they do not mind:
    /// it throws one file of its own away, finds it, and puts it back. Never part of the
    /// suite — the list it reads is the user's.
    #[test]
    #[ignore = "reads the wastebasket of whoever runs it"]
    fn the_system_wastebasket_lists_and_restores_what_was_thrown_away() {
        if !SUPPORTED {
            return;
        }
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap();
        let tmp = tempfile::tempdir_in(home).unwrap();
        let file = tmp.path().join("fcmd-wastebasket-test.txt");
        fs::write(&file, "back again").unwrap();
        System.throw_away(&file).unwrap();
        assert!(!file.exists());
        let item = System
            .list()
            .unwrap()
            .into_iter()
            .find(|i| i.original() == file)
            .expect("listed");
        assert_eq!(item.size, 10);
        let out = restore(&System, &[item], OnClash::Ask);
        assert_eq!(out.back, vec![file.clone()]);
        assert_eq!(fs::read_to_string(&file).unwrap(), "back again");
    }
}
