//! Disk usage of a whole tree: one walk, every folder's total known at the end (SPEC 06 §12).
//!
//! `F3` on a folder answers with one figure and the usage list with one level; this is the
//! walk behind the map, which has to know what every folder under the root holds before it
//! can draw the first rectangle. The sizes are added up once, here, on the thread that walked,
//! and the children of every folder are left largest first — so the screen that shows the
//! tree never sorts, never sums and never touches the disk.

use std::cmp::Reverse;

use fcmd_core::{CancelToken, EntryKind, VPath};

use crate::fs::JobFs;

/// The most files one folder keeps by name. A folder of a million thumbnails is a million
/// rectangles nobody could tell apart: the largest are kept and the rest are one row that
/// says how many they were and what they weigh, so nothing is lost from the total.
pub const KEPT: usize = 256;

/// How deep the walk goes before it stops descending, as every other walk here does.
const DEEPEST: u32 = 64;

/// What a row of the tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
    /// The files of a folder that were too many and too small to keep by name.
    Rest,
}

/// One folder or file of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The name in its folder; empty for the root, which is known by its path, and for a
    /// `Rest`, which the screen names in the reader's language.
    pub name: String,
    pub kind: Kind,
    /// What it holds: a file's own size, a folder's everything underneath.
    pub size: u64,
    /// How many files that is; one for a file.
    pub files: u64,
    /// The executable bit, where the filesystem has one: it decides the colour, as it does in
    /// the panel.
    pub exec: bool,
    pub parent: Option<u32>,
    /// A folder's rows, largest first and by name among equals.
    pub children: Vec<u32>,
}

/// The whole walk. Node 0 is the root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tree {
    pub nodes: Vec<Node>,
    /// Folders that could not be read: they count as empty, and the screen says how many.
    pub unreadable: usize,
}

impl Tree {
    pub const ROOT: u32 = 0;

    pub fn node(&self, at: u32) -> &Node {
        &self.nodes[at as usize]
    }

    /// The names from the root down to this row, the root's own left out.
    pub fn names(&self, at: u32) -> Vec<&str> {
        let mut out = Vec::new();
        let mut at = Some(at);
        while let Some(n) = at {
            let node = self.node(n);
            if node.parent.is_some() {
                out.push(node.name.as_str());
            }
            at = node.parent;
        }
        out.reverse();
        out
    }
}

/// Walk everything under `root` and add it up.
///
/// Links are neither followed nor counted, which is what `F3` on a folder does, so the two
/// agree about what a folder weighs. `progress` hears the files and bytes seen so far, once
/// per folder; a cancelled walk answers `None`, because half a tree is a map that lies about
/// every folder it did not reach.
pub fn walk(
    fs: &dyn JobFs,
    root: &VPath,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(u64, u64),
) -> Option<Tree> {
    let mut tree = Tree::default();
    tree.nodes.push(Node {
        name: String::new(),
        kind: Kind::Dir,
        size: 0,
        files: 0,
        exec: false,
        parent: None,
        children: Vec::new(),
    });
    let (mut seen_files, mut seen_bytes) = (0u64, 0u64);
    let mut stack: Vec<(u32, VPath, u32)> = vec![(Tree::ROOT, root.clone(), 0)];
    while let Some((at, dir, depth)) = stack.pop() {
        if cancel.is_cancelled() {
            return None;
        }
        let Ok(entries) = fs.read_dir(&dir) else {
            tree.unreadable += 1;
            continue;
        };
        // The files of this folder, to keep the largest of: (size, name, executable).
        let mut files: Vec<(u64, String, bool)> = Vec::new();
        for (name, facts) in entries {
            match facts.kind {
                EntryKind::Parent | EntryKind::Symlink => {}
                EntryKind::Dir => {
                    if depth >= DEEPEST {
                        continue;
                    }
                    let id = tree.nodes.len() as u32;
                    tree.nodes.push(Node {
                        name: name.to_string_lossy().into_owned(),
                        kind: Kind::Dir,
                        size: 0,
                        files: 0,
                        exec: false,
                        parent: Some(at),
                        children: Vec::new(),
                    });
                    tree.nodes[at as usize].children.push(id);
                    stack.push((id, dir.join(&name), depth + 1));
                }
                _ => {
                    seen_files += 1;
                    seen_bytes += facts.size;
                    let exec = facts.mode.is_some_and(|m| m & 0o111 != 0);
                    files.push((facts.size, name.to_string_lossy().into_owned(), exec));
                }
            }
        }
        files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let rest: Vec<(u64, String, bool)> = if files.len() > KEPT {
            files.split_off(KEPT)
        } else {
            Vec::new()
        };
        for (size, name, exec) in files {
            let id = tree.nodes.len() as u32;
            tree.nodes.push(Node {
                name,
                kind: Kind::File,
                size,
                files: 1,
                exec,
                parent: Some(at),
                children: Vec::new(),
            });
            tree.nodes[at as usize].children.push(id);
        }
        if !rest.is_empty() {
            let id = tree.nodes.len() as u32;
            tree.nodes.push(Node {
                name: String::new(),
                kind: Kind::Rest,
                size: rest.iter().map(|f| f.0).sum(),
                files: rest.len() as u64,
                exec: false,
                parent: Some(at),
                children: Vec::new(),
            });
            tree.nodes[at as usize].children.push(id);
        }
        progress(seen_files, seen_bytes);
    }
    // A row is always made after the folder it is in, so going through them backwards meets
    // every child before its parent: one pass, and every folder has its total.
    for at in (1..tree.nodes.len()).rev() {
        let (size, files) = (tree.nodes[at].size, tree.nodes[at].files);
        if let Some(parent) = tree.nodes[at].parent {
            let parent = &mut tree.nodes[parent as usize];
            parent.size += size;
            parent.files += files;
        }
    }
    // And the order the screen wants, settled here once rather than at every frame.
    for at in 0..tree.nodes.len() {
        let mut children = std::mem::take(&mut tree.nodes[at].children);
        children.sort_by(|a, b| {
            let (a, b) = (&tree.nodes[*a as usize], &tree.nodes[*b as usize]);
            Reverse(a.size)
                .cmp(&Reverse(b.size))
                .then_with(|| a.name.cmp(&b.name))
        });
        tree.nodes[at].children = children;
    }
    Some(tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalFs;

    fn names(tree: &Tree, at: u32) -> Vec<(String, u64)> {
        tree.node(at)
            .children
            .iter()
            .map(|c| (tree.node(*c).name.clone(), tree.node(*c).size))
            .collect()
    }

    #[test]
    fn every_folder_holds_the_sum_of_what_is_under_it_largest_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/deep")).unwrap();
        std::fs::create_dir(dir.path().join("empty")).unwrap();
        std::fs::write(dir.path().join("README"), vec![0u8; 30]).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.path().join("src/deep/x.rs"), vec![0u8; 500]).unwrap();
        let mut heard = (0, 0);
        let tree = walk(
            &LocalFs,
            &VPath::local(dir.path()),
            &CancelToken::new(),
            &mut |files, bytes| heard = (files, bytes),
        )
        .expect("nobody cancelled");
        assert_eq!(heard, (3, 630), "the last progress is the whole of it");
        let root = tree.node(Tree::ROOT);
        assert_eq!((root.size, root.files), (630, 3));
        assert_eq!(
            names(&tree, Tree::ROOT),
            [
                ("src".to_string(), 600),
                ("README".to_string(), 30),
                ("empty".to_string(), 0)
            ]
        );
        let src = root.children[0];
        assert_eq!(
            names(&tree, src),
            [("deep".to_string(), 500), ("main.rs".to_string(), 100)]
        );
        let deep = tree.node(src).children[0];
        let x = tree.node(deep).children[0];
        assert_eq!(tree.names(x), ["src", "deep", "x.rs"]);
        assert_eq!(tree.unreadable, 0);
    }

    #[test]
    fn a_folder_of_too_many_files_keeps_the_largest_and_one_row_for_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..KEPT + 10 {
            std::fs::write(dir.path().join(format!("f{i:04}")), vec![0u8; i + 1]).unwrap();
        }
        let tree = walk(
            &LocalFs,
            &VPath::local(dir.path()),
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .unwrap();
        let root = tree.node(Tree::ROOT);
        assert_eq!(root.children.len(), KEPT + 1);
        assert_eq!(root.files, (KEPT + 10) as u64);
        let total: u64 = (1..=(KEPT + 10) as u64).sum();
        assert_eq!(root.size, total, "nothing is lost from the total");
        let rest = root
            .children
            .iter()
            .map(|c| tree.node(*c))
            .find(|n| n.kind == Kind::Rest)
            .expect("a row for the rest");
        // The ten smallest: 1 + 2 + … + 10 bytes.
        assert_eq!((rest.files, rest.size), (10, 55));
        assert_eq!(
            tree.node(root.children[0]).name,
            format!("f{:04}", KEPT + 9)
        );
    }

    #[test]
    fn a_walk_that_is_cancelled_has_no_tree_and_a_missing_folder_is_counted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), "a").unwrap();
        let cancel = CancelToken::new();
        cancel.cancel();
        assert!(walk(&LocalFs, &VPath::local(dir.path()), &cancel, &mut |_, _| {}).is_none());
        let gone = VPath::local(dir.path().join("not-there"));
        let tree = walk(&LocalFs, &gone, &CancelToken::new(), &mut |_, _| {}).unwrap();
        assert_eq!(tree.unreadable, 1);
        assert_eq!(tree.node(Tree::ROOT).size, 0);
    }
}
