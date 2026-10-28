//! What the command line says to the left of what one types (SPEC 06 §2b).
//!
//! It was the panel's path and a `>`, which is what `cmd.exe` shows and what FAR shows by
//! default. FAR lets it be written out — `$p$g` is that default, and the other letters put the
//! time, the drive, a bracket or a marker for an elevated session in it — and the plugin that
//! everybody who works in a repository ends up installing adds one more thing: the branch.
//! Both are here, the letters FAR takes and `%GITBRANCH%`, because a prompt that cannot say
//! which branch one is on is a prompt one checks with a command.

use fcmd_core::VPath;

/// The path, the branch where there is one, and a `>`.
///
/// FAR's own default is `$p$g`; ours adds `$r`, which is nothing at all outside a repository
/// and ` (branch)` inside one. A prompt that cannot say which branch one is on is a prompt
/// that gets checked with a command, and the `git` branch is the one thing everybody who uses
/// FAR in a repository installs a plugin for.
pub const DEFAULT: &str = "$p$r$g ";

/// The prompt, with everything in the format expanded.
///
/// `branch` is the repository's current branch where the directory is in one; nothing in the
/// format asks the disk, so this function is pure and the lookup is somebody else's job.
pub fn expand(format: &str, cwd: &VPath, branch: Option<&str>, pushed: usize) -> String {
    /// The plugin's own spelling of the branch, which is what somebody who used it will try
    /// first. Matched without regard to case, like everything else in a format.
    const BRANCH: &str = "%gitbranch%";

    let chars: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut at = 0usize;
    // Byte offsets and character offsets are not the same thing in a format somebody wrote
    // by hand, so the scan is by character and the one word is matched by rebuilding it.
    while at < chars.len() {
        let c = chars[at];
        if c == '%' {
            let word: String = chars[at..]
                .iter()
                .take(BRANCH.chars().count())
                .collect::<String>()
                .to_lowercase();
            if word == BRANCH {
                out.push_str(branch.unwrap_or(""));
                at += BRANCH.chars().count();
                continue;
            }
            out.push('%');
            at += 1;
            continue;
        }
        if c != '$' {
            out.push(c);
            at += 1;
            continue;
        }
        at += 1;
        let Some(&what) = chars.get(at) else {
            // A format ending in a lone `$` means a `$`, rather than nothing.
            out.push('$');
            break;
        };
        at += 1;
        match what.to_ascii_lowercase() {
            'p' => out.push_str(&cwd.to_string()),
            // The last part of the path on its own, which is what one usually wants on a
            // narrow terminal.
            'w' => out.push_str(&last_part(cwd)),
            // The drive or the scheme: `C:` on Windows, `/` on Unix, `sftp://host` on a
            // connection — the part of the path that says *where* rather than *which folder*.
            'n' => out.push_str(&root_of(cwd)),
            'g' => out.push('>'),
            'l' => out.push('<'),
            'a' => out.push('&'),
            'b' => out.push('|'),
            'c' => out.push('('),
            'f' => out.push(')'),
            'q' => out.push('='),
            's' => out.push(' '),
            // The branch, with its own decoration, so one format reads properly in a
            // repository and out of it. `%GITBRANCH%` is the bare name for anybody who
            // wants to bracket it themselves.
            'r' => {
                if let Some(branch) = branch.filter(|b| !b.is_empty()) {
                    out.push_str(&format!(" ({branch})"));
                }
            }
            'd' => out.push_str(&today()),
            't' => out.push_str(&clock()),
            // FAR's own marker for a session with more rights than usual.
            '@' => {
                if elevated() {
                    out.push_str("[admin]")
                }
            }
            // Erase what came before it, which is how a format takes a decoration back out
            // when what it decorated turned out to be empty.
            'h' => {
                out.pop();
            }
            // One plus per directory on `pushd`'s stack, which is what `cmd.exe` writes and
            // what FAR copies: the prompt says how far in one has gone.
            '+' => out.push_str(&"+".repeat(pushed)),
            '$' => out.push('$'),
            // An unknown letter is left as it was written, so a typo in the configuration is
            // visible rather than silently swallowed.
            _ => {
                out.push('$');
                out.push(what);
            }
        }
    }
    out
}

/// The last component of a path, or the whole of it when there is only one.
fn last_part(cwd: &VPath) -> String {
    let text = cwd.to_string();
    let trimmed = text.trim_end_matches(['/', '\\']);
    match trimmed.rsplit_once(['/', '\\']) {
        Some((_, last)) if !last.is_empty() => last.to_string(),
        _ => text,
    }
}

/// Where the path starts: the drive letter on Windows, `/` on Unix, the authority of a remote
/// path.
fn root_of(cwd: &VPath) -> String {
    let text = cwd.to_string();
    if let Some((scheme, rest)) = text.split_once("://") {
        let host = rest.split(['/', '\\']).next().unwrap_or("");
        return format!("{scheme}://{host}");
    }
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic() => format!("{letter}:"),
        (Some('/'), _) => "/".to_string(),
        _ => String::new(),
    }
}

/// Today, as `2026-10-28`, and the time as `17:42`, both in local time.
///
/// Both come out of the one stamp the interface already knows how to write, which is why
/// there is no date library in here: `format_stamp` is `2026-10-28 17:42:05`, and these are
/// the two halves of it that a prompt asks for.
fn today() -> String {
    stamp()[..10].to_string()
}

fn clock() -> String {
    stamp()[11..16].to_string()
}

fn stamp() -> String {
    fcmd_ui::format_stamp(std::time::SystemTime::now())
}

/// Is this session running with more rights than an ordinary one?
fn elevated() -> bool {
    #[cfg(unix)]
    {
        // Nothing unsafe and nothing to ask: the id is in the environment the process already
        // has, and zero is root.
        nix::unistd::Uid::effective().is_root()
    }
    #[cfg(not(unix))]
    {
        // Windows keeps it behind a token lookup, which belongs in `fcmd-platform` with the
        // rest; until it is there the marker says nothing rather than guessing (`BACKLOG`).
        false
    }
}

/// The branch the directory is on, where it is in a git repository.
///
/// Nothing is run: `git` need not be installed, and a prompt that starts a process on every
/// keystroke is a prompt that makes a slow terminal slower. `HEAD` is one small file, and the
/// walk upwards stops at the first `.git` — which may be a file rather than a directory, as it
/// is in a worktree or a submodule, and then it says where the real one is.
pub fn branch_of(dir: &std::path::Path) -> Option<String> {
    let mut at = Some(dir);
    while let Some(here) = at {
        let dot = here.join(".git");
        if dot.is_dir() {
            return head_of(&dot);
        }
        if dot.is_file() {
            // `gitdir: /path/to/.git/worktrees/name`
            let text = std::fs::read_to_string(&dot).ok()?;
            let target = text.trim().strip_prefix("gitdir:")?.trim();
            let target = std::path::Path::new(target);
            let target = if target.is_absolute() {
                target.to_path_buf()
            } else {
                here.join(target)
            };
            return head_of(&target);
        }
        at = here.parent();
    }
    None
}

/// `ref: refs/heads/main` as `main`, and a head that is not on a branch as the short commit.
fn head_of(git_dir: &std::path::Path) -> Option<String> {
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref:") {
        Some(reference) => {
            let reference = reference.trim();
            Some(
                reference
                    .strip_prefix("refs/heads/")
                    .unwrap_or(reference)
                    .to_string(),
            )
        }
        // Detached: the file holds the commit itself, and seven characters of it is what
        // everything else that shows a commit shows.
        None if head.len() >= 7 && head.chars().all(|c| c.is_ascii_hexdigit()) => {
            Some(head[..7].to_string())
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_path_the_branch_and_a_sign() {
        let cwd = VPath::local("/home/mb/project");
        let text = expand(DEFAULT, &cwd, None, 0);
        assert!(text.starts_with(&cwd.to_string()), "{text:?}");
        assert!(text.ends_with("> "), "{text:?}");
        // In a repository the branch is in it, in brackets, and outside one the brackets are
        // not there either — which is the whole point of the letter.
        let text = expand(DEFAULT, &cwd, Some("work"), 0);
        assert!(text.ends_with(" (work)> "), "{text:?}");
        assert_eq!(expand("$r", &cwd, Some(""), 0), "");
    }

    #[test]
    fn the_letters_far_takes_are_the_letters_we_take() {
        let cwd = VPath::local("/home/mb/project");
        assert_eq!(expand("$w$g", &cwd, None, 0), "project>");
        assert_eq!(expand("$n", &cwd, None, 0), "/");
        assert_eq!(expand("$c$w$f", &cwd, None, 0), "(project)");
        assert_eq!(expand("$a$b$q$l", &cwd, None, 0), "&|=<");
        assert_eq!(expand("a$sb", &cwd, None, 0), "a b");
        assert_eq!(expand("$$", &cwd, None, 0), "$");
        // The case of the letter is not the letter.
        assert_eq!(expand("$W$G", &cwd, None, 0), "project>");
        // A letter nobody knows is left as it was typed, so a mistake can be seen.
        assert_eq!(expand("$z", &cwd, None, 0), "$z");
        // One plus per directory `pushd` has put away, as `cmd.exe` writes it.
        assert_eq!(expand("$+$g", &cwd, None, 3), "+++>");
        assert_eq!(expand("$+$g", &cwd, None, 0), ">");
        // The date and the time are there, in the shape they are asked for.
        assert_eq!(expand("$d", &cwd, None, 0).len(), 10);
        assert_eq!(expand("$t", &cwd, None, 0).len(), 5);
    }

    #[test]
    fn the_branch_goes_in_the_prompt_the_way_the_plugin_spells_it() {
        let cwd = VPath::local("/home/mb/project");
        assert_eq!(
            expand("$w $c%GITBRANCH%$f$g", &cwd, Some("main"), 0),
            "project (main)>"
        );
        // Outside a repository it says nothing, and `$h` takes the decoration back out: the
        // bracket, the space, and then the sign is written again.
        assert_eq!(expand("$w$s%GITBRANCH%$g", &cwd, None, 0), "project >");
        assert_eq!(expand("%gitbranch%", &cwd, Some("work"), 0), "work");
        // A lone percent is a percent.
        assert_eq!(expand("100%", &cwd, None, 0), "100%");
    }

    #[test]
    fn a_remote_path_keeps_its_authority_as_the_root() {
        let cwd = VPath::parse("sftp://mb@host:22/srv/www");
        assert_eq!(expand("$n", &cwd, None, 0), "sftp://mb@host:22");
        assert_eq!(expand("$w", &cwd, None, 0), "www");
    }

    /// This repository is one, so the branch is readable from inside the tests.
    #[test]
    fn the_branch_of_this_very_checkout_is_readable() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // A checkout on a branch, or a detached head in CI, is an answer; a source tree
        // unpacked from an archive is not a repository at all, which is also fine.
        if let Some(branch) = branch_of(here) {
            assert!(!branch.is_empty());
        }
    }
}
