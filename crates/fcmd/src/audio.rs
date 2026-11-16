//! Sound, through a player on the machine (SPEC 05 §1.15).
//!
//! fcmd decodes no sound, for the reason it decodes no film: the codecs and the libraries
//! that play them are not ours to link. `p` in the viewer hands the file to `ffplay` — part
//! of the same `ffmpeg` that draws a film's frames — or, where that is missing, to `mpv`,
//! each run as a program with no window and no terminal of its own, and never linked.
//!
//! Neither can be told to pause without a window or a terminal to send the key to, so a
//! pause is the player stopped and the moment remembered, and playing on is the player
//! started again from that moment. What the status line says is worked out from the clock
//! here, not read back from the player: a second either way is all it can be off by.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

/// The players, in the order they are looked for on the `PATH`.
pub const PLAYERS: [&str; 2] = ["ffplay", "mpv"];

/// The first player this machine has, by name and where it is.
pub fn find() -> Option<(&'static str, PathBuf)> {
    PLAYERS
        .iter()
        .find_map(|name| fcmd_arc::which(name).map(|path| (*name, path)))
}

/// What a player is run with: no window, no terminal, nothing printed, and the file from
/// `from` seconds in. The path is the last argument, after `--` where the player takes it,
/// so a file whose name starts with a dash is still a file.
pub fn args(player: &str, file: &Path, from: f64) -> Vec<OsString> {
    let from = if from.is_finite() && from > 0.0 {
        from
    } else {
        0.0
    };
    let mut out: Vec<OsString> = Vec::new();
    if player == "mpv" {
        for a in ["--no-video", "--no-terminal", "--really-quiet"] {
            out.push(a.into());
        }
        if from > 0.0 {
            out.push(format!("--start={from:.3}").into());
        }
        out.push("--".into());
    } else {
        for a in ["-nodisp", "-autoexit", "-nostats", "-loglevel", "quiet"] {
            out.push(a.into());
        }
        if from > 0.0 {
            out.push("-ss".into());
            out.push(format!("{from:.3}").into());
        }
    }
    out.push(file.as_os_str().to_os_string());
    out
}

/// One sound being played, or paused.
#[derive(Debug)]
pub struct Playing {
    /// The player's name, and where it is.
    pub player: &'static str,
    program: PathBuf,
    /// The file on this disk.
    pub file: PathBuf,
    /// How long it runs, when the file says.
    pub length: Option<f64>,
    /// Where in the sound the player was started from.
    from: f64,
    /// When it was started; `None` while paused.
    since: Option<Instant>,
    child: Option<Child>,
}

impl Playing {
    /// Start `file` from its beginning with the player at `program`.
    pub fn start(
        player: &'static str,
        program: PathBuf,
        file: PathBuf,
        length: Option<f64>,
    ) -> std::io::Result<Playing> {
        let mut playing = Playing {
            player,
            program,
            file,
            length,
            from: 0.0,
            since: None,
            child: None,
        };
        playing.resume()?;
        Ok(playing)
    }

    /// Where the sound is now, in seconds: where it was started from and how long ago,
    /// never past its end.
    pub fn at(&self) -> f64 {
        let run = self.since.map_or(0.0, |t| t.elapsed().as_secs_f64());
        let at = self.from + run;
        match self.length {
            Some(length) => at.min(length),
            None => at,
        }
    }

    pub fn paused(&self) -> bool {
        self.since.is_none()
    }

    /// Stop the player and remember where it was.
    pub fn pause(&mut self) {
        if self.paused() {
            return;
        }
        self.from = self.at();
        self.since = None;
        self.kill();
    }

    /// Start the player again from where it was paused.
    pub fn resume(&mut self) -> std::io::Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        // A sound paused within a second of its end starts again from the beginning: there
        // is nothing left of it to play on.
        if self.length.is_some_and(|l| self.from >= l - 1.0) {
            self.from = 0.0;
        }
        let child = Command::new(&self.program)
            .args(args(self.player, &self.file, self.from))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        self.child = Some(child);
        self.since = Some(Instant::now());
        Ok(())
    }

    /// Has the player finished by itself? `Some(true)` when it got to the end of the sound,
    /// `Some(false)` when it stopped with an error — a file it could not play — and `None`
    /// while it is still playing, or paused.
    pub fn ended(&mut self) -> Option<bool> {
        let status = self.child.as_mut()?.try_wait().ok()??;
        self.child = None;
        self.from = 0.0;
        self.since = None;
        Some(status.success())
    }

    /// What the status line says: playing or paused, where, and of how long.
    pub fn note(&self) -> String {
        let mark = if self.paused() { "⏸" } else { "▶" };
        let at = fcmd_view::media::Film::clock(self.at());
        match self.length {
            Some(length) => format!("{mark} {at} / {}", fcmd_view::media::Film::clock(length)),
            None => format!("{mark} {at}"),
        }
    }

    fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// A sound nobody is listening to any more is not left playing: the viewer closed, the
/// program quit, the reader went on to another file.
impl Drop for Playing {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn each_player_is_asked_for_sound_alone_from_the_moment_it_was_left() {
        let file = Path::new("-song.mp3");
        assert_eq!(
            text(&args("ffplay", file, 0.0)),
            [
                "-nodisp",
                "-autoexit",
                "-nostats",
                "-loglevel",
                "quiet",
                "-song.mp3"
            ]
        );
        assert_eq!(
            text(&args("ffplay", file, 42.5)),
            [
                "-nodisp",
                "-autoexit",
                "-nostats",
                "-loglevel",
                "quiet",
                "-ss",
                "42.500",
                "-song.mp3"
            ]
        );
        assert_eq!(
            text(&args("mpv", file, 3.0)),
            [
                "--no-video",
                "--no-terminal",
                "--really-quiet",
                "--start=3.000",
                "--",
                "-song.mp3"
            ]
        );
        assert_eq!(text(&args("mpv", file, f64::NAN)).len(), 5, "no start");
    }

    /// A stand-in for the player: a program that runs for a while and then exits as asked.
    /// `sh` on Unix; the arguments it is given are the player's, which it ignores.
    #[cfg(unix)]
    fn stand_in(dir: &Path, name: &str, seconds: &str, code: i32) -> PathBuf {
        testing::stand_in(dir, name, &format!("sleep {seconds}\nexit {code}"))
    }

    #[cfg(unix)]
    #[test]
    fn a_pause_stops_the_player_and_playing_on_starts_it_from_where_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let program = stand_in(dir.path(), "player", "30", 0);
        let mut p =
            Playing::start("ffplay", program, dir.path().join("a.mp3"), Some(200.0)).unwrap();
        assert!(!p.paused());
        assert!(p.note().starts_with("▶ 00:00 / 03:20"), "{}", p.note());
        assert_eq!(p.ended(), None, "still playing");
        std::thread::sleep(std::time::Duration::from_millis(50));
        p.pause();
        assert!(p.paused());
        assert!(p.child.is_none(), "the player was stopped");
        let left = p.at();
        assert!(left > 0.0 && left < 5.0, "{left}");
        assert_eq!(p.at(), left, "a paused sound does not move");
        assert!(p.note().starts_with("⏸ "), "{}", p.note());
        p.resume().unwrap();
        assert!(!p.paused() && p.at() >= left);
    }

    #[cfg(unix)]
    #[test]
    fn a_player_that_finishes_says_whether_it_got_to_the_end() {
        let dir = tempfile::tempdir().unwrap();
        for (code, fine) in [(0, true), (1, false)] {
            let program = stand_in(dir.path(), &format!("player{code}"), "0", code);
            let mut p = Playing::start("mpv", program, dir.path().join("a.flac"), None).unwrap();
            let mut said = None;
            for _ in 0..200 {
                said = p.ended();
                if said.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert_eq!(said, Some(fine));
            assert!(p.paused(), "back at the start, ready to play again");
            assert_eq!(p.at(), 0.0);
        }
    }

    #[test]
    fn a_player_that_is_not_there_is_an_error_not_a_hang() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-player");
        assert!(Playing::start("ffplay", missing, dir.path().join("a.mp3"), None).is_err());
    }
}

/// What the tests here and the viewer's share: a player that is a shell script.
#[cfg(all(test, unix))]
pub(crate) mod testing {
    use std::path::{Path, PathBuf};

    /// Write `body` as an executable script called `name` in `dir`, and run it once with
    /// `probe` as its argument, which it answers by exiting. The test binary runs tests on
    /// many threads, and a file just written can still be held open for writing by a
    /// child another thread is forking — `exec` then fails with "text file busy" — so the
    /// probe is tried until that has passed, and the script is ready when this returns.
    pub(crate) fn stand_in(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join(name);
        std::fs::write(
            &script,
            format!("#!/bin/sh\n[ \"$1\" = probe ] && exit 0\n{body}\n"),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        for _ in 0..200 {
            match std::process::Command::new(&script).arg("probe").status() {
                Ok(_) => return script,
                Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("{}: {e}", script.display()),
            }
        }
        panic!("{} stayed busy", script.display());
    }
}
