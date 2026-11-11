//! Where the space of a whole tree goes, as a map and as a list (SPEC 06 §12).
//!
//! The walk (`fcmd_jobs::usage`) comes back with every folder's total and its rows already
//! largest first. This is the screen's half: which folder is being looked at, which row of it
//! the cursor is on, and — for the map — the rectangles of that one folder, laid out when the
//! folder or the room changes and kept until it changes again. A tree of a million files and a
//! tree of ten cost the same to draw: one folder's rows.

use fcmd_core::{CancelToken, VPath};
use fcmd_jobs::usage::{Kind, Tree};

/// The most rows of a folder that get a rectangle of their own. The rows are largest first,
/// so what is past this is small, and it is drawn as one rectangle that says how much it is.
pub const MOST_TILES: usize = 160;

/// One rectangle of the map, in cells from the map's own corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    /// Which row of the folder it is, or [`Tile::OTHERS`] for the one that stands for every
    /// row too small to draw.
    pub row: usize,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Tile {
    pub const OTHERS: usize = usize::MAX;

    fn holds(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// A rectangle inside a folder's rectangle: one of the folder's own rows, drawn so the map
/// shows what a folder is made of before one goes into it. The cursor does not stop on these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inner {
    pub node: u32,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

/// Lay `sizes` out as a squarified treemap in `w` by `h` cells (Bruls, Huizing and van Wijk).
///
/// The sizes are taken largest first, as the tree keeps them. Each rectangle's area is its
/// share of the whole, and the strips are chosen so the rectangles come out as near to square
/// as they can — on the screen, where a cell is about twice as tall as it is wide, which is
/// why the arithmetic is done on a surface twice as high as the cell count and halved at the
/// end. Edges are rounded to cells only then, each from its own exact position, so two
/// neighbours always round their shared edge to the same cell: the result tiles the area with
/// no gap and no overlap. A rectangle that rounds to nothing is left out.
pub fn squarify(sizes: &[u64], w: u16, h: u16) -> Vec<(usize, u16, u16, u16, u16)> {
    let total: u64 = sizes.iter().sum();
    if total == 0 || w == 0 || h == 0 {
        return Vec::new();
    }
    let (full_w, full_h) = (w as f64, h as f64 * 2.0);
    let scale = full_w * full_h / total as f64;
    let areas: Vec<(usize, f64)> = sizes
        .iter()
        .enumerate()
        .filter(|(_, s)| **s > 0)
        .map(|(i, s)| (i, *s as f64 * scale))
        .collect();

    // How far from square the worst rectangle of a strip would be, laid along `side`.
    let worst = |sum: f64, big: f64, small: f64, side: f64| -> f64 {
        let (side2, sum2) = (side * side, sum * sum);
        (side2 * big / sum2).max(sum2 / (side2 * small))
    };
    let mut out = Vec::with_capacity(areas.len());
    let (mut x, mut y, mut rw, mut rh) = (0.0f64, 0.0f64, full_w, full_h);
    let mut place =
        |strip: &[(usize, f64)], x: &mut f64, y: &mut f64, rw: &mut f64, rh: &mut f64| {
            let sum: f64 = strip.iter().map(|s| s.1).sum();
            let cell = |v: f64| v.round().max(0.0) as u16;
            if *rw >= *rh {
                // A column down the left edge of what is left.
                let thick = if *rh > 0.0 { sum / *rh } else { 0.0 };
                let (x0, x1) = (cell(*x), cell(*x + thick).min(w));
                let mut at = *y;
                for (i, area) in strip {
                    let long = if thick > 0.0 { area / thick } else { 0.0 };
                    let (y0, y1) = (cell(at / 2.0), cell((at + long) / 2.0).min(h));
                    at += long;
                    if x1 > x0 && y1 > y0 {
                        out.push((*i, x0, y0, x1 - x0, y1 - y0));
                    }
                }
                *x += thick;
                *rw -= thick;
            } else {
                // A row along the top edge of what is left.
                let thick = if *rw > 0.0 { sum / *rw } else { 0.0 };
                let (y0, y1) = (cell(*y / 2.0), cell((*y + thick) / 2.0).min(h));
                let mut at = *x;
                for (i, area) in strip {
                    let long = if thick > 0.0 { area / thick } else { 0.0 };
                    let (x0, x1) = (cell(at), cell(at + long).min(w));
                    at += long;
                    if x1 > x0 && y1 > y0 {
                        out.push((*i, x0, y0, x1 - x0, y1 - y0));
                    }
                }
                *y += thick;
                *rh -= thick;
            }
        };

    let mut strip: Vec<(usize, f64)> = Vec::new();
    let (mut sum, mut big, mut small) = (0.0f64, 0.0f64, f64::MAX);
    for item in areas {
        let side = rw.min(rh).max(f64::MIN_POSITIVE);
        let (sum2, big2, small2) = (sum + item.1, big.max(item.1), small.min(item.1));
        if strip.is_empty() || worst(sum2, big2, small2, side) <= worst(sum, big, small, side) {
            strip.push(item);
            (sum, big, small) = (sum2, big2, small2);
        } else {
            place(&strip, &mut x, &mut y, &mut rw, &mut rh);
            strip.clear();
            strip.push(item);
            (sum, big, small) = (item.1, item.1, item.1);
        }
    }
    if !strip.is_empty() {
        place(&strip, &mut x, &mut y, &mut rw, &mut rh);
    }
    out
}

/// Which way an arrow goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    Left,
    Right,
    Up,
    Down,
}

/// The screen: a walk still running, or the tree it brought back.
#[derive(Debug)]
pub struct UsageView {
    /// Which answer is this screen's: a walk that was abandoned may still report.
    pub id: u64,
    /// The panel it was opened from, which is the one `Enter` on a file moves.
    pub side: usize,
    pub root: VPath,
    pub cancel: CancelToken,
    /// Files and bytes seen so far, while the walk runs.
    pub progress: (u64, u64),
    pub tree: Option<Tree>,
    /// The folder being looked at.
    pub at: u32,
    /// The row of it the cursor is on, in the tree's order: largest first.
    pub cursor: usize,
    /// The list rather than the map.
    pub list: bool,
    /// The first row the list shows.
    pub top: usize,
    /// The map's room in cells, from the last frame, and the rectangles laid out for it.
    area: (u16, u16),
    laid_for: Option<(u32, u16, u16)>,
    tiles: Vec<Tile>,
    /// How many rows, from the top, have a rectangle of their own.
    kept: usize,
    /// What the folders on the map are made of, one level down.
    inner: Vec<Inner>,
}

impl UsageView {
    pub fn new(id: u64, side: usize, root: VPath, cancel: CancelToken) -> UsageView {
        UsageView {
            id,
            side,
            root,
            cancel,
            progress: (0, 0),
            tree: None,
            at: Tree::ROOT,
            cursor: 0,
            list: false,
            top: 0,
            area: (0, 0),
            laid_for: None,
            tiles: Vec::new(),
            kept: 0,
            inner: Vec::new(),
        }
    }

    /// The walk is back.
    pub fn arrived(&mut self, tree: Tree) {
        self.tree = Some(tree);
        self.at = Tree::ROOT;
        self.cursor = 0;
        self.top = 0;
        self.laid_for = None;
    }

    /// The room the map and the list have, told by the renderer before it draws. The
    /// rectangles are laid out again only when the room or the folder is not the one they
    /// were laid out for — which is what keeps a frame as cheap as painting them.
    pub fn fit(&mut self, w: u16, h: u16) {
        self.area = (w, h);
        if self.laid_for == Some((self.at, w, h)) {
            return;
        }
        self.tiles.clear();
        self.inner.clear();
        self.kept = 0;
        self.laid_for = Some((self.at, w, h));
        let Some(tree) = &self.tree else {
            return;
        };
        let rows = &tree.node(self.at).children;
        let total = tree.node(self.at).size;
        // A row gets a rectangle when its share of the map is at least one cell. The rows are
        // largest first, so the ones that do are a run from the top; everything after it is
        // one rectangle more, at the end, which stands for the others.
        let cells = w as f64 * h as f64;
        let kept = rows
            .iter()
            .take(MOST_TILES)
            .take_while(|c| total > 0 && tree.node(**c).size as f64 * cells >= total as f64)
            .count();
        let mut sizes: Vec<u64> = rows[..kept].iter().map(|c| tree.node(*c).size).collect();
        let drawn: u64 = sizes.iter().sum();
        if kept < rows.len() {
            sizes.push(total.saturating_sub(drawn));
        }
        self.kept = kept;
        self.tiles = squarify(&sizes, w, h)
            .into_iter()
            .map(|(i, x, y, w, h)| Tile {
                row: if i < kept { i } else { Tile::OTHERS },
                x,
                y,
                w,
                h,
            })
            .collect();
        // Inside a folder's rectangle, under the line with its name and above its bottom
        // edge, the folder's own rows by the same rule — where there is room for that to
        // say anything.
        for tile in &self.tiles {
            if tile.row == Tile::OTHERS || tile.w < 6 || tile.h < 4 {
                continue;
            }
            let folder = tree.node(rows[tile.row]);
            if folder.kind != Kind::Dir || folder.size == 0 {
                continue;
            }
            let (x, y, w, h) = (tile.x + 1, tile.y + 1, tile.w - 1, tile.h - 2);
            let cells = w as f64 * h as f64;
            let kept = folder
                .children
                .iter()
                .take(MOST_TILES)
                .take_while(|c| tree.node(**c).size as f64 * cells >= folder.size as f64)
                .count();
            let mut sizes: Vec<u64> = folder.children[..kept]
                .iter()
                .map(|c| tree.node(*c).size)
                .collect();
            // What is too small to draw still takes its share, as the folder's own colour.
            let drawn: u64 = sizes.iter().sum();
            sizes.push(folder.size.saturating_sub(drawn));
            self.inner.extend(
                squarify(&sizes, w, h)
                    .into_iter()
                    .filter(|t| t.0 < kept)
                    .map(|(i, ix, iy, iw, ih)| Inner {
                        node: folder.children[i],
                        x: x + ix,
                        y: y + iy,
                        w: iw,
                        h: ih,
                    }),
            );
        }
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    /// The rectangles inside the folders' rectangles.
    pub fn inner(&self) -> &[Inner] {
        &self.inner
    }

    /// Does this rectangle show what its folder is made of?
    pub fn nests(&self, tile: &Tile) -> bool {
        self.inner.iter().any(|i| tile.holds(i.x, i.y))
    }

    pub fn list_height(&self) -> usize {
        self.area.1 as usize
    }

    /// The rows of the folder being looked at.
    pub fn rows(&self) -> &[u32] {
        match &self.tree {
            Some(tree) => &tree.node(self.at).children,
            None => &[],
        }
    }

    /// The row under the cursor, if the folder has any.
    pub fn chosen(&self) -> Option<u32> {
        self.rows().get(self.cursor).copied()
    }

    /// The rectangle the cursor is in: the row's own, or the one for the others when the row
    /// is too small to have one.
    pub fn chosen_tile(&self) -> Option<&Tile> {
        let want = if self.cursor < self.kept {
            self.cursor
        } else {
            Tile::OTHERS
        };
        self.tiles.iter().find(|t| t.row == want)
    }

    /// How many rows the rectangle for the others stands for, and what they weigh.
    pub fn others(&self) -> (usize, u64) {
        let Some(tree) = &self.tree else {
            return (0, 0);
        };
        let rows = self.rows();
        let drawn: u64 = rows[..self.kept.min(rows.len())]
            .iter()
            .map(|c| tree.node(*c).size)
            .sum();
        (
            rows.len().saturating_sub(self.kept),
            tree.node(self.at).size.saturating_sub(drawn),
        )
    }

    /// An arrow on the map: the rectangle on the other side of that edge, opposite the middle
    /// of this one. `false` at the edge of the map.
    pub fn step(&mut self, way: Way) -> bool {
        let Some(from) = self.chosen_tile().copied() else {
            // The cursor is on a row the map has no room for at all: the first arrow puts it
            // back on the map rather than doing nothing.
            return match self.tiles.first() {
                Some(first) => {
                    self.cursor = if first.row == Tile::OTHERS {
                        self.kept
                    } else {
                        first.row
                    };
                    true
                }
                None => false,
            };
        };
        let (mid_x, mid_y) = (from.x + from.w / 2, from.y + from.h / 2);
        let (x, y) = match way {
            Way::Left if from.x > 0 => (from.x - 1, mid_y),
            Way::Right => (from.x + from.w, mid_y),
            Way::Up if from.y > 0 => (mid_x, from.y - 1),
            Way::Down => (mid_x, from.y + from.h),
            _ => return false,
        };
        let Some(to) = self.tiles.iter().find(|t| t.holds(x, y)) else {
            return false;
        };
        self.cursor = if to.row == Tile::OTHERS {
            self.kept
        } else {
            to.row
        };
        true
    }

    /// The cursor moved in the list by so many rows.
    pub fn move_by(&mut self, delta: i64) {
        let last = self.rows().len().saturating_sub(1) as i64;
        self.cursor = (self.cursor as i64 + delta).clamp(0, last) as usize;
        self.keep_in_view();
    }

    pub fn move_to(&mut self, row: usize) {
        self.cursor = row.min(self.rows().len().saturating_sub(1));
        self.keep_in_view();
    }

    fn keep_in_view(&mut self) {
        let height = self.list_height().max(1);
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + height {
            self.top = self.cursor + 1 - height;
        }
    }

    /// `Enter` on a folder: look inside it. `false` when the row is not a folder.
    pub fn descend(&mut self) -> bool {
        let Some(tree) = &self.tree else {
            return false;
        };
        match self.chosen() {
            Some(row) if tree.node(row).kind == Kind::Dir => {
                self.at = row;
                self.cursor = 0;
                self.top = 0;
                true
            }
            _ => false,
        }
    }

    /// `Backspace`: the folder this one is in, with the cursor on the one just left. `false`
    /// at the folder the walk started from.
    pub fn ascend(&mut self) -> bool {
        let Some(tree) = &self.tree else {
            return false;
        };
        let Some(parent) = tree.node(self.at).parent else {
            return false;
        };
        let left = self.at;
        self.at = parent;
        self.cursor = tree
            .node(parent)
            .children
            .iter()
            .position(|c| *c == left)
            .unwrap_or(0);
        self.top = 0;
        self.keep_in_view();
        true
    }

    /// Where a row is on the disk.
    pub fn path_of(&self, row: u32) -> VPath {
        let mut path = self.root.clone();
        if let Some(tree) = &self.tree {
            for name in tree.names(row) {
                path = path.join(name);
            }
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every cell of the area belongs to exactly one rectangle, whatever the sizes.
    fn tiles_cover(sizes: &[u64], w: u16, h: u16) -> Vec<(usize, u16, u16, u16, u16)> {
        let tiles = squarify(sizes, w, h);
        let mut seen = vec![0u8; w as usize * h as usize];
        for (_, x, y, tw, th) in &tiles {
            for yy in *y..*y + *th {
                for xx in *x..*x + *tw {
                    seen[yy as usize * w as usize + xx as usize] += 1;
                }
            }
        }
        assert!(
            seen.iter().all(|n| *n == 1),
            "{sizes:?} in {w}x{h}: a cell is in {:?} rectangles",
            seen.iter().find(|n| **n != 1)
        );
        tiles
    }

    #[test]
    fn the_rectangles_tile_the_area_with_no_gap_and_no_overlap() {
        tiles_cover(&[6, 6, 4, 3, 2, 2, 1], 60, 20);
        tiles_cover(&[1000, 1, 1, 1], 80, 24);
        tiles_cover(&[5, 5, 5, 5, 5, 5, 5, 5, 5], 37, 11);
        tiles_cover(&[7], 10, 3);
        let many: Vec<u64> = (1..=150).rev().collect();
        tiles_cover(&many, 100, 30);
        assert!(squarify(&[0, 0], 10, 10).is_empty(), "nothing to draw");
        assert!(squarify(&[5], 0, 10).is_empty(), "no room");
    }

    #[test]
    fn a_rectangle_is_as_large_as_its_share() {
        // Half, a quarter and two eighths of 40 by 10: 400 cells.
        let tiles = tiles_cover(&[4, 2, 1, 1], 40, 10);
        let area = |i: usize| -> u32 {
            tiles
                .iter()
                .filter(|t| t.0 == i)
                .map(|t| t.3 as u32 * t.4 as u32)
                .sum()
        };
        assert_eq!(area(0), 200);
        assert_eq!(area(1), 100);
        assert_eq!(area(2) + area(3), 100);
        // And near enough square on the screen, where a cell is twice as tall as wide: the
        // half is 20 cells by 10, which is 20 by 20 to the eye.
        assert_eq!((tiles[0].3, tiles[0].4), (20, 10));
    }

    fn tree() -> Tree {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("big/inner")).unwrap();
        std::fs::write(dir.path().join("big/inner/film.mkv"), vec![0u8; 4000]).unwrap();
        std::fs::write(dir.path().join("big/notes.txt"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.path().join("pack.zip"), vec![0u8; 3000]).unwrap();
        std::fs::write(dir.path().join("small.rs"), vec![0u8; 2000]).unwrap();
        std::fs::write(dir.path().join("tiny"), vec![0u8; 1]).unwrap();
        for i in 0..60 {
            std::fs::write(dir.path().join(format!("t{i}")), vec![0u8; 20]).unwrap();
        }
        fcmd_jobs::usage::walk(
            &fcmd_jobs::LocalFs,
            &VPath::local(dir.path()),
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .unwrap()
    }

    fn view() -> UsageView {
        let mut view = UsageView::new(1, 0, VPath::local("root"), CancelToken::new());
        view.arrived(tree());
        view.fit(40, 10);
        view
    }

    fn name(view: &UsageView) -> String {
        let tree = view.tree.as_ref().unwrap();
        tree.node(view.chosen().unwrap()).name.clone()
    }

    #[test]
    fn the_arrows_walk_the_map_and_enter_and_backspace_the_tree() {
        let mut view = view();
        assert_eq!(name(&view), "big", "the largest is where it starts");
        // `big` is half of it and takes the left of the map; what is right of it is the next.
        assert!(!view.step(Way::Left), "the edge of the map");
        assert!(view.step(Way::Right));
        assert_ne!(name(&view), "big");
        assert!(view.step(Way::Left));
        assert_eq!(name(&view), "big");

        assert!(view.descend());
        view.fit(40, 10);
        assert_eq!(name(&view), "inner");
        assert!(view.step(Way::Right) || view.step(Way::Down));
        assert_eq!(name(&view), "notes.txt");
        assert!(!view.descend(), "a file is not a folder to look into");
        let path = view.path_of(view.chosen().unwrap());
        assert_eq!(path, VPath::local("root").join("big").join("notes.txt"));

        assert!(view.ascend());
        assert_eq!(name(&view), "big", "back on the folder that was left");
        assert!(!view.ascend(), "the walk started here");
    }

    #[test]
    fn a_row_too_small_to_draw_is_in_the_rectangle_for_the_others() {
        let mut view = view();
        // Twenty bytes in eleven thousand is less than one cell of 400, and one byte far
        // less: the sixty-one of them are one rectangle between them.
        assert!(view.tiles().iter().any(|t| t.row == Tile::OTHERS));
        assert_eq!(view.others(), (61, 1201));
        view.move_to(usize::MAX);
        assert_eq!(name(&view), "tiny");
        let tile = *view.chosen_tile().expect("the others stand for it");
        assert_eq!(tile.row, Tile::OTHERS);
        // The folder's rectangle shows what the folder is made of: its two rows, inside it.
        let big = view.tiles()[0];
        let inside: Vec<&Inner> = view
            .inner()
            .iter()
            .filter(|i| big.holds(i.x, i.y) && big.holds(i.x + i.w - 1, i.y + i.h - 1))
            .collect();
        assert_eq!(inside.len(), 2, "{:?}", view.inner());
        assert!(
            inside.iter().all(|i| i.y > big.y),
            "under the folder's name"
        );
        // Nothing is laid out again for a frame that changes nothing.
        let before = view.tiles().to_vec();
        view.fit(40, 10);
        assert_eq!(view.tiles(), before);
    }

    #[test]
    fn the_list_keeps_the_cursor_on_the_screen() {
        let mut view = view();
        view.fit(40, 2);
        view.list = true;
        view.move_by(3);
        assert_eq!((view.cursor, view.top), (3, 2));
        view.move_to(usize::MAX);
        assert_eq!((view.cursor, view.top), (63, 62));
        view.move_by(-100);
        assert_eq!((view.cursor, view.top), (0, 0));
    }
}
