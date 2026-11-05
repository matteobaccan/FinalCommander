//! `.editorconfig` (SPEC 05 §2.9): the indentation and line endings a project asks for.
//!
//! The files are read from the file's folder upwards, until one says `root = true` or the
//! top of the disk; a nearer file wins over a farther one, and within a file a later section
//! wins over an earlier one — the order the format defines. The section names are the
//! format's globs: `*`, `**`, `?`, `[abc]`, `{a,b}` and `{1..3}`; a name without a `/` matches
//! the file's name in any folder below, one with a `/` matches the path from the file's own
//! `.editorconfig`.

use std::path::{Path, PathBuf};

/// What the files said, for one file. `None` is "not said".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Props {
    /// `indent_style = space` is `Some(true)`, `tab` is `Some(false)`.
    pub spaces: Option<bool>,
    /// `indent_size`, or `tab_width` when the size is `tab`.
    pub indent_size: Option<usize>,
    pub tab_width: Option<usize>,
    /// `lf`, `crlf` or `cr`.
    pub end_of_line: Option<String>,
    pub trim_trailing_whitespace: Option<bool>,
    pub insert_final_newline: Option<bool>,
}

impl Props {
    pub fn is_empty(&self) -> bool {
        *self == Props::default()
    }

    /// The width a tab is shown at: `tab_width`, else `indent_size`.
    pub fn width(&self) -> Option<usize> {
        self.tab_width.or(self.indent_size)
    }
}

/// What applies to `file`, from every `.editorconfig` above it.
pub fn resolve(file: &Path) -> Props {
    let mut configs: Vec<(PathBuf, String)> = Vec::new();
    let mut dir = file.parent();
    while let Some(d) = dir {
        let candidate = d.join(".editorconfig");
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            let root = is_root(&text);
            configs.push((d.to_path_buf(), text));
            if root {
                break;
            }
        }
        dir = d.parent();
    }
    let mut props = Props::default();
    // Farthest first, so the nearer files overwrite.
    for (dir, text) in configs.iter().rev() {
        let Ok(rel) = file.strip_prefix(dir) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        apply(&mut props, text, &rel);
    }
    props
}

fn is_root(text: &str) -> bool {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            return false;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim().eq_ignore_ascii_case("root") && v.trim().eq_ignore_ascii_case("true") {
                return true;
            }
        }
    }
    false
}

/// Apply one file's sections to `props`, for the file at `rel` below that file's folder.
pub fn apply(props: &mut Props, text: &str, rel: &str) {
    let mut active = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            active = section_matches(section, rel);
            continue;
        }
        if !active {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().to_ascii_lowercase();
        let unset = value == "unset";
        let flag = match value.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        match key.as_str() {
            "indent_style" => {
                props.spaces = match value.as_str() {
                    "space" => Some(true),
                    "tab" => Some(false),
                    _ => None,
                }
            }
            "indent_size" => {
                props.indent_size = if value == "tab" || unset {
                    None
                } else {
                    value.parse().ok().filter(|n| (1..=16).contains(n))
                }
            }
            "tab_width" => props.tab_width = value.parse().ok().filter(|n| (1..=16).contains(n)),
            "end_of_line" => {
                props.end_of_line =
                    matches!(value.as_str(), "lf" | "crlf" | "cr").then(|| value.clone())
            }
            "trim_trailing_whitespace" => props.trim_trailing_whitespace = flag,
            "insert_final_newline" => props.insert_final_newline = flag,
            _ => {}
        }
    }
}

fn section_matches(section: &str, rel: &str) -> bool {
    let pattern = section.trim();
    if pattern.contains('/') {
        let pattern = pattern.trim_start_matches('/');
        glob(pattern, rel)
    } else {
        // No slash: the name in any folder.
        let name = rel.rsplit('/').next().unwrap_or(rel);
        glob(pattern, name) || glob(&format!("**/{pattern}"), rel)
    }
}

/// The format's glob, by expanding braces into alternatives and matching each.
pub fn glob(pattern: &str, text: &str) -> bool {
    expand(pattern)
        .iter()
        .any(|p| matches(p.as_bytes(), text.as_bytes()))
}

fn expand(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('{') else {
        return vec![pattern.to_string()];
    };
    let mut depth = 0;
    let mut close = None;
    for (i, c) in pattern[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return vec![pattern.to_string()];
    };
    let (head, body, tail) = (
        &pattern[..open],
        &pattern[open + 1..close],
        &pattern[close + 1..],
    );
    let mut options: Vec<String> = Vec::new();
    if let Some((a, b)) = body.split_once("..") {
        if let (Ok(a), Ok(b)) = (a.parse::<i64>(), b.parse::<i64>()) {
            for n in a.min(b)..=a.max(b) {
                options.push(n.to_string());
            }
        }
    }
    if options.is_empty() {
        let mut depth = 0;
        let mut start = 0;
        for (i, c) in body.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    options.push(body[start..i].to_string());
                    start = i + 1;
                }
                _ => {}
            }
        }
        options.push(body[start..].to_string());
    }
    options
        .into_iter()
        .flat_map(|o| expand(&format!("{head}{o}{tail}")))
        .collect()
}

fn matches(p: &[u8], t: &[u8]) -> bool {
    match p.first() {
        None => t.is_empty(),
        Some(b'*') if p.get(1) == Some(&b'*') => {
            let rest = &p[2..];
            let rest = rest.strip_prefix(b"/").map_or(rest, |r| r);
            (0..=t.len()).any(|i| matches(rest, &t[i..]))
        }
        Some(b'*') => (0..=t.len())
            .take_while(|&i| i == 0 || t[i - 1] != b'/')
            .any(|i| matches(&p[1..], &t[i..])),
        Some(b'?') => !t.is_empty() && t[0] != b'/' && matches(&p[1..], &t[1..]),
        Some(b'[') => {
            let Some(end) = p.iter().position(|&c| c == b']') else {
                return !t.is_empty() && t[0] == b'[' && matches(&p[1..], &t[1..]);
            };
            let Some(&c) = t.first() else { return false };
            let set = &p[1..end];
            let (negate, set) = match set.first() {
                Some(b'!') => (true, &set[1..]),
                _ => (false, set),
            };
            let mut hit = false;
            let mut i = 0;
            while i < set.len() {
                if i + 2 < set.len() && set[i + 1] == b'-' {
                    hit |= (set[i]..=set[i + 2]).contains(&c);
                    i += 3;
                } else {
                    hit |= set[i] == c;
                    i += 1;
                }
            }
            hit != negate && matches(&p[end + 1..], &t[1..])
        }
        Some(&c) => t.first() == Some(&c) && matches(&p[1..], &t[1..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_globs() {
        assert!(glob("*.rs", "main.rs"));
        assert!(!glob("*.rs", "src/main.rs"));
        assert!(glob("**.rs", "src/main.rs"));
        assert!(glob("*.{js,py}", "a.py"));
        assert!(!glob("*.{js,py}", "a.rs"));
        assert!(glob("file{1..3}.txt", "file2.txt"));
        assert!(!glob("file{1..3}.txt", "file4.txt"));
        assert!(glob("[Mm]akefile", "makefile"));
        assert!(glob("lib/**/*.c", "lib/a/b/x.c"));
        assert!(glob("?.md", "a.md"));
    }

    #[test]
    fn nearer_files_and_later_sections_win_and_root_stops_the_climb() {
        let dir = tempfile::tempdir().unwrap();
        let top = dir.path();
        std::fs::create_dir_all(top.join("proj/src")).unwrap();
        // Above the root: never read.
        std::fs::write(top.join(".editorconfig"), "[*]\nindent_size = 8\n").unwrap();
        std::fs::write(
            top.join("proj/.editorconfig"),
            "root = true\n\n[*]\nindent_style = space\nindent_size = 4\nend_of_line = lf\n\
             insert_final_newline = true\n\n[Makefile]\nindent_style = tab\n\n\
             [*.{md,txt}]\ntrim_trailing_whitespace = false\n",
        )
        .unwrap();
        std::fs::write(
            top.join("proj/src/.editorconfig"),
            "[*.rs]\nindent_size = 2\n",
        )
        .unwrap();
        let rs = resolve(&top.join("proj/src/main.rs"));
        assert_eq!(rs.spaces, Some(true));
        assert_eq!(rs.indent_size, Some(2), "the nearer file wins");
        assert_eq!(rs.end_of_line.as_deref(), Some("lf"));
        assert_eq!(rs.insert_final_newline, Some(true));
        let mk = resolve(&top.join("proj/Makefile"));
        assert_eq!(mk.spaces, Some(false), "the later section wins");
        assert_eq!(mk.indent_size, Some(4), "and leaves what it does not say");
        let md = resolve(&top.join("proj/src/README.md"));
        assert_eq!(md.trim_trailing_whitespace, Some(false));
        assert!(resolve(&top.join("elsewhere.rs")).indent_size == Some(8));
    }
}
