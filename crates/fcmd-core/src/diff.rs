//! Two texts, lined up: which lines are the same, which are not, and where inside a line.
//!
//! The algorithm is Myers' — the one `diff(1)` and git use — in its linear-space form, which
//! is what makes it usable on files of a size somebody would actually compare. It works on
//! *keys* rather than on the lines themselves, so "ignore whitespace" and "ignore case" cost
//! nothing at the comparison: they change what a line hashes to and the algorithm never
//! knows (SPEC 05 §3).
//!
//! Nothing here draws anything. It answers two questions — how do these two lists line up,
//! and which words of these two lines differ — and the screen is somebody else's business.

use std::collections::HashMap;

/// What to pass over when deciding whether two lines are the same.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ignore {
    /// Treat every run of blanks as one, and drop the blanks at both ends.
    pub whitespace: bool,
    /// Compare without case, as `diff -i` does.
    pub case: bool,
    /// Drop a trailing `\r`, so a file written on Windows and one written here can be the
    /// same file. On by default in the dialog, because it is almost never what one means to
    /// be told about.
    pub line_endings: bool,
}

impl Ignore {
    /// The key a line compares by.
    ///
    /// A borrowed line that needs no changing is returned as it is, so the common case costs
    /// no allocation.
    pub fn key<'a>(&self, line: &'a str) -> std::borrow::Cow<'a, str> {
        use std::borrow::Cow;

        let mut out = Cow::Borrowed(line);
        if self.line_endings {
            if let Some(cut) = out.strip_suffix('\r') {
                out = Cow::Owned(cut.to_string());
            }
        }
        if self.whitespace {
            let folded: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
            out = Cow::Owned(folded);
        }
        if self.case {
            out = Cow::Owned(out.to_lowercase());
        }
        out
    }
}

/// One step of the alignment: a run of lines that are the same, or one side's run on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `count` lines are the same, starting at `left` and `right`.
    Same {
        left: usize,
        right: usize,
        count: usize,
    },
    /// `count` lines only the left side has.
    Removed { left: usize, count: usize },
    /// `count` lines only the right side has.
    Added { right: usize, count: usize },
}

/// How the two sides line up, oldest line first.
///
/// Adjacent runs of the same kind are joined, so a hunk of twenty changed lines is three ops
/// and not forty.
pub fn lines(left: &[&str], right: &[&str], ignore: &Ignore) -> Vec<Op> {
    // The comparison is on numbers, not on strings: the lines are hashed once and every loop
    // below compares two `u32`s. A line that appears in only one file gets a number of its
    // own and so is never equal to anything.
    let mut ids: HashMap<String, u32> = HashMap::new();
    let mut number = |line: &str| -> u32 {
        let key = ignore.key(line).into_owned();
        let next = ids.len() as u32;
        *ids.entry(key).or_insert(next)
    };
    let a: Vec<u32> = left.iter().map(|l| number(l)).collect();
    let b: Vec<u32> = right.iter().map(|l| number(l)).collect();

    let mut ops = Vec::new();
    walk(&a, &b, 0, 0, &mut ops);
    join(ops)
}

/// Two sides small enough to line up exactly, by the quadratic table.
///
/// Four hundred by four hundred is a hundred and sixty thousand cells, which is nothing; the
/// point of the limit is that the table is quadratic and two large files with no line in
/// common should not be allowed to ask for a gigabyte.
const EXACT: usize = 400;

/// Line up two regions, patience-style: the lines that appear exactly once on both sides are
/// the anchors, the longest increasing run of them is the alignment, and what lies between
/// two anchors is lined up the same way again.
///
/// This is git's `--patience`, and it is here rather than Myers' because it gives the answer
/// a reader of source code wants — a moved block reads as a moved block — and because it is
/// short enough to be read and believed. A region with no anchor at all falls back to the
/// exact table when it is small, and otherwise to one side then the other, which is the
/// honest answer when two texts have nothing to line up.
fn walk(a: &[u32], b: &[u32], at_a: usize, at_b: usize, ops: &mut Vec<Op>) {
    // The cheap ends first: a common prefix and a common suffix are the bulk of two versions
    // of the same file.
    let prefix = a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count();
    if prefix > 0 {
        ops.push(Op::Same {
            left: at_a,
            right: at_b,
            count: prefix,
        });
        walk(
            &a[prefix..],
            &b[prefix..],
            at_a + prefix,
            at_b + prefix,
            ops,
        );
        return;
    }
    let suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    if suffix > 0 {
        let (a_end, b_end) = (a.len() - suffix, b.len() - suffix);
        walk(&a[..a_end], &b[..b_end], at_a, at_b, ops);
        ops.push(Op::Same {
            left: at_a + a_end,
            right: at_b + b_end,
            count: suffix,
        });
        return;
    }
    if a.is_empty() && b.is_empty() {
        return;
    }
    if b.is_empty() {
        ops.push(Op::Removed {
            left: at_a,
            count: a.len(),
        });
        return;
    }
    if a.is_empty() {
        ops.push(Op::Added {
            right: at_b,
            count: b.len(),
        });
        return;
    }

    let anchors = anchors(a, b);
    if anchors.is_empty() {
        if a.len() <= EXACT && b.len() <= EXACT {
            exact(a, b, at_a, at_b, ops);
        } else {
            ops.push(Op::Removed {
                left: at_a,
                count: a.len(),
            });
            ops.push(Op::Added {
                right: at_b,
                count: b.len(),
            });
        }
        return;
    }
    // Between the anchors, the same question again; the anchors themselves are the same
    // line by construction.
    let mut last = (0usize, 0usize);
    for (x, y) in anchors {
        walk(
            &a[last.0..x],
            &b[last.1..y],
            at_a + last.0,
            at_b + last.1,
            ops,
        );
        ops.push(Op::Same {
            left: at_a + x,
            right: at_b + y,
            count: 1,
        });
        last = (x + 1, y + 1);
    }
    walk(
        &a[last.0..],
        &b[last.1..],
        at_a + last.0,
        at_b + last.1,
        ops,
    );
}

/// The lines that appear exactly once on each side, paired, as the longest run whose
/// positions rise on both sides at once.
///
/// A line that appears twice is no anchor: it cannot say *which* of the two it is. The
/// longest increasing run is what keeps the alignment from crossing itself.
fn anchors(a: &[u32], b: &[u32]) -> Vec<(usize, usize)> {
    let mut count_a: HashMap<u32, (usize, usize)> = HashMap::new();
    for (at, line) in a.iter().enumerate() {
        let slot = count_a.entry(*line).or_insert((0, at));
        slot.0 += 1;
        slot.1 = at;
    }
    let mut count_b: HashMap<u32, (usize, usize)> = HashMap::new();
    for (at, line) in b.iter().enumerate() {
        let slot = count_b.entry(*line).or_insert((0, at));
        slot.0 += 1;
        slot.1 = at;
    }
    // In the order of the left side, so the increasing run only has to rise on the right.
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (at, line) in a.iter().enumerate() {
        let Some((1, _)) = count_a.get(line).map(|(n, p)| (*n, *p)) else {
            continue;
        };
        if let Some((1, there)) = count_b.get(line).map(|(n, p)| (*n, *p)) {
            pairs.push((at, there));
        }
    }
    longest_rising(&pairs)
}

/// The longest subsequence of `pairs` whose second value rises. Patience's own step: the
/// card game it is named after is this.
fn longest_rising(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if pairs.is_empty() {
        return Vec::new();
    }
    // `piles[k]` is the index of the smallest pair that can end a rising run of length k + 1.
    let mut piles: Vec<usize> = Vec::new();
    let mut came_from: Vec<Option<usize>> = vec![None; pairs.len()];
    for (at, pair) in pairs.iter().enumerate() {
        let put = piles.partition_point(|&p| pairs[p].1 < pair.1);
        came_from[at] = put.checked_sub(1).map(|prev| piles[prev]);
        if put == piles.len() {
            piles.push(at);
        } else {
            piles[put] = at;
        }
    }
    let mut out = Vec::with_capacity(piles.len());
    let mut at = piles.last().copied();
    while let Some(here) = at {
        out.push(pairs[here]);
        at = came_from[here];
    }
    out.reverse();
    out
}

/// Line up two small regions exactly, by the longest common subsequence.
///
/// The quadratic table, which is the right tool once the region is known to be small: it
/// gives the shortest edit script there is, and a region with no unique line is exactly where
/// patience has nothing to say.
fn exact(a: &[u32], b: &[u32], at_a: usize, at_b: usize, ops: &mut Vec<Op>) {
    let (n, m) = (a.len(), b.len());
    // `table[i][j]`: the length of the longest common subsequence of `a[i..]` and `b[j..]`.
    let mut table = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if a[i] == b[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            ops.push(Op::Same {
                left: at_a + i,
                right: at_b + j,
                count: 1,
            });
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            ops.push(Op::Removed {
                left: at_a + i,
                count: 1,
            });
            i += 1;
        } else {
            ops.push(Op::Added {
                right: at_b + j,
                count: 1,
            });
            j += 1;
        }
    }
    if i < n {
        ops.push(Op::Removed {
            left: at_a + i,
            count: n - i,
        });
    }
    if j < m {
        ops.push(Op::Added {
            right: at_b + j,
            count: m - j,
        });
    }
}

/// Join neighbouring ops of the same kind, so a hunk reads as a hunk.
fn join(ops: Vec<Op>) -> Vec<Op> {
    let mut out: Vec<Op> = Vec::with_capacity(ops.len());
    for op in ops {
        match (out.last_mut(), op) {
            (
                Some(Op::Same { left, right, count }),
                Op::Same {
                    left: l2,
                    right: r2,
                    count: c2,
                },
            ) if *left + *count == l2 && *right + *count == r2 => *count += c2,
            (
                Some(Op::Removed { left, count }),
                Op::Removed {
                    left: l2,
                    count: c2,
                },
            ) if *left + *count == l2 => *count += c2,
            (
                Some(Op::Added { right, count }),
                Op::Added {
                    right: r2,
                    count: c2,
                },
            ) if *right + *count == r2 => *count += c2,
            _ => out.push(op),
        }
    }
    out
}

/// Which part of two lines differs, as byte ranges into each.
///
/// One range per side: everything between the common prefix and the common suffix. That is
/// not the finest answer a word diff can give, but it is the one a reader wants — the eye
/// goes to the part that changed, and a line broken into six highlighted fragments is harder
/// to read than one. Both ends land on a character boundary, so a line of Cyrillic or an
/// emoji is highlighted and not cut in half.
pub fn inside(left: &str, right: &str) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
    let mut start = 0;
    let (la, lb) = (left.len(), right.len());
    while start < la && start < lb && left.as_bytes()[start] == right.as_bytes()[start] {
        start += 1;
    }
    while start > 0 && !(left.is_char_boundary(start) && right.is_char_boundary(start)) {
        start -= 1;
    }
    let mut back = 0;
    while back < la - start
        && back < lb - start
        && left.as_bytes()[la - back - 1] == right.as_bytes()[lb - back - 1]
    {
        back += 1;
    }
    while back > 0 && !(left.is_char_boundary(la - back) && right.is_char_boundary(lb - back)) {
        back -= 1;
    }
    (start..la - back, start..lb - back)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(left: &[&str], right: &[&str]) -> bool {
        lines(left, right, &Ignore::default())
            .iter()
            .all(|op| matches!(op, Op::Same { .. }))
    }

    /// The alignment has to *be* an alignment: every line of each side accounted for exactly
    /// once, in order. A diff that loses a line is worse than no diff.
    fn check(left: &[&str], right: &[&str], ops: &[Op]) {
        let mut a = 0;
        let mut b = 0;
        for op in ops {
            match *op {
                Op::Same {
                    left: l,
                    right: r,
                    count,
                } => {
                    assert_eq!((l, r), (a, b), "a gap in the alignment: {ops:?}");
                    for n in 0..count {
                        assert_eq!(left[a + n], right[b + n], "not the same line: {ops:?}");
                    }
                    a += count;
                    b += count;
                }
                Op::Removed { left: l, count } => {
                    assert_eq!(l, a, "a gap on the left: {ops:?}");
                    a += count;
                }
                Op::Added { right: r, count } => {
                    assert_eq!(r, b, "a gap on the right: {ops:?}");
                    b += count;
                }
            }
        }
        assert_eq!((a, b), (left.len(), right.len()), "lines lost: {ops:?}");
    }

    #[test]
    fn two_files_that_are_the_same_are_one_run() {
        let a = ["one", "two", "three"];
        assert_eq!(
            lines(&a, &a, &Ignore::default()),
            vec![Op::Same {
                left: 0,
                right: 0,
                count: 3
            }]
        );
        assert!(same(&a, &a));
    }

    #[test]
    fn a_line_added_in_the_middle_is_one_op() {
        let a = ["one", "two", "three"];
        let b = ["one", "and a half", "two", "three"];
        let ops = lines(&a, &b, &Ignore::default());
        check(&a, &b, &ops);
        assert_eq!(
            ops,
            vec![
                Op::Same {
                    left: 0,
                    right: 0,
                    count: 1
                },
                Op::Added { right: 1, count: 1 },
                Op::Same {
                    left: 1,
                    right: 2,
                    count: 2
                },
            ]
        );
    }

    #[test]
    fn a_line_changed_is_one_removed_and_one_added() {
        let a = ["one", "two", "three"];
        let b = ["one", "TWO", "three"];
        let ops = lines(&a, &b, &Ignore::default());
        check(&a, &b, &ops);
        assert_eq!(ops.len(), 4, "{ops:?}");
        assert!(matches!(ops[1], Op::Removed { left: 1, count: 1 }));
        assert!(matches!(ops[2], Op::Added { right: 1, count: 1 }));
    }

    #[test]
    fn nothing_in_common_is_one_side_then_the_other() {
        let a = ["one", "two"];
        let b = ["three", "four", "five"];
        let ops = lines(&a, &b, &Ignore::default());
        check(&a, &b, &ops);
        assert_eq!(
            ops,
            vec![
                Op::Removed { left: 0, count: 2 },
                Op::Added { right: 0, count: 3 },
            ]
        );
    }

    #[test]
    fn an_empty_side_is_all_of_the_other() {
        let a: [&str; 0] = [];
        let b = ["one", "two"];
        check(&a, &b, &lines(&a, &b, &Ignore::default()));
        check(&b, &a, &lines(&b, &a, &Ignore::default()));
        assert_eq!(
            lines(&a, &b, &Ignore::default()),
            vec![Op::Added { right: 0, count: 2 }]
        );
        assert!(lines(&a, &a, &Ignore::default()).is_empty());
    }

    #[test]
    fn what_is_ignored_changes_what_counts_as_the_same() {
        let a = ["  one  two", "Three", "four\r"];
        let b = ["one two", "three", "four"];
        // Nothing ignored: all three lines differ.
        assert!(!same(&a, &b));
        // Everything ignored: the same file.
        let all = Ignore {
            whitespace: true,
            case: true,
            line_endings: true,
        };
        assert!(
            lines(&a, &b, &all)
                .iter()
                .all(|op| matches!(op, Op::Same { .. })),
            "{:?}",
            lines(&a, &b, &all)
        );
        // One at a time, each one answers for its own line.
        let ws = Ignore {
            whitespace: true,
            ..Ignore::default()
        };
        assert_eq!(ws.key("  one  two").as_ref(), "one two");
        let case = Ignore {
            case: true,
            ..Ignore::default()
        };
        assert_eq!(case.key("Three").as_ref(), "three");
        let eol = Ignore {
            line_endings: true,
            ..Ignore::default()
        };
        assert_eq!(eol.key("four\r").as_ref(), "four");
    }

    #[test]
    fn a_long_file_with_one_change_is_still_two_runs_and_a_change() {
        let left: Vec<String> = (0..2000).map(|n| format!("line {n}")).collect();
        let mut right = left.clone();
        right[1000] = "changed".to_string();
        let a: Vec<&str> = left.iter().map(String::as_str).collect();
        let b: Vec<&str> = right.iter().map(String::as_str).collect();
        let ops = lines(&a, &b, &Ignore::default());
        check(&a, &b, &ops);
        assert_eq!(ops.len(), 4, "{ops:?}");
    }

    /// What patience is for: a block that moved reads as a block that moved, rather than as
    /// every line of the file having changed.
    #[test]
    fn a_block_that_moved_reads_as_a_move() {
        let a = ["a", "b", "fn one", "body one", "end", "c", "d"];
        let b = ["a", "b", "c", "d", "fn one", "body one", "end"];
        let ops = lines(&a, &b, &Ignore::default());
        check(&a, &b, &ops);
        // Two runs are the same and the block is moved, not rewritten: no op touches more
        // lines than the block itself.
        let moved: usize = ops
            .iter()
            .map(|op| match *op {
                Op::Removed { count, .. } | Op::Added { count, .. } => count,
                Op::Same { .. } => 0,
            })
            .sum();
        assert!(moved <= 6, "{ops:?}");
    }

    /// A region with no line that appears exactly once has no anchor, and the exact table is
    /// what answers it. The alignment must still be an alignment.
    #[test]
    fn a_region_of_repeated_lines_is_lined_up_exactly() {
        let a = ["x", "x", "x", "y", "x"];
        let b = ["x", "y", "x", "x"];
        let ops = lines(&a, &b, &Ignore::default());
        check(&a, &b, &ops);
        let same: usize = ops
            .iter()
            .map(|op| match *op {
                Op::Same { count, .. } => count,
                _ => 0,
            })
            .sum();
        assert!(same >= 3, "the common lines were found: {ops:?}");
    }

    /// The invariant on a few thousand pairs of small random files: every line accounted for
    /// exactly once, in order, and every `Same` really is the same line. A diff that loses a
    /// line is the one bug that matters here, and it hides in the corner cases.
    #[test]
    fn the_alignment_holds_on_random_files() {
        // A tiny deterministic generator: a test that fails only on Tuesdays is no test.
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let words = ["alpha", "beta", "gamma", "delta", "x", "y", ""];
        for _ in 0..2000 {
            let make = |n: usize, next: &mut dyn FnMut() -> u64| -> Vec<&'static str> {
                (0..n).map(|_| words[(next() % 7) as usize]).collect()
            };
            let left = make((next() % 12) as usize, &mut next);
            let right = make((next() % 12) as usize, &mut next);
            let ignore = Ignore {
                whitespace: next() % 2 == 0,
                case: next() % 2 == 0,
                line_endings: next() % 2 == 0,
            };
            let ops = lines(&left, &right, &ignore);
            // `check` compares the lines themselves, so it is run with nothing ignored; with
            // something ignored the keys are what must match, which is what this checks.
            let mut a = 0;
            let mut b = 0;
            for op in &ops {
                match *op {
                    Op::Same {
                        left: l,
                        right: r,
                        count,
                    } => {
                        assert_eq!((l, r), (a, b), "{ops:?}");
                        for n in 0..count {
                            assert_eq!(
                                ignore.key(left[a + n]),
                                ignore.key(right[b + n]),
                                "{ops:?}"
                            );
                        }
                        a += count;
                        b += count;
                    }
                    Op::Removed { left: l, count } => {
                        assert_eq!(l, a, "{ops:?}");
                        a += count;
                    }
                    Op::Added { right: r, count } => {
                        assert_eq!(r, b, "{ops:?}");
                        b += count;
                    }
                }
            }
            assert_eq!((a, b), (left.len(), right.len()), "lines lost: {ops:?}");
        }
    }

    #[test]
    fn the_part_of_a_line_that_differs_is_what_is_marked() {
        let (l, r) = inside("the quick brown fox", "the quick red fox");
        assert_eq!(&"the quick brown fox"[l], "brown");
        assert_eq!(&"the quick red fox"[r], "red");

        // Nothing in common: the whole of both.
        let (l, r) = inside("abc", "xyz");
        assert_eq!(l, 0..3);
        assert_eq!(r, 0..3);

        // The same line: nothing marked.
        let (l, r) = inside("same", "same");
        assert!(l.is_empty() && r.is_empty());

        // One side longer: only the tail.
        let (l, r) = inside("hello", "hello there");
        assert!(l.is_empty());
        assert_eq!(&"hello there"[r], " there");

        // A character boundary is never cut: `à` is two bytes.
        let (l, r) = inside("città", "citta");
        assert_eq!(&"città"[l], "à");
        assert_eq!(&"citta"[r], "a");
    }
}
