//! Frames of a film, through `ffmpeg` (SPEC 05 §1.15).
//!
//! fcmd decodes no video. The codecs carry patents, the libraries that decode them are LGPL
//! or GPL, and no decoder written in Rust is ready — so a frame is asked of the `ffmpeg`
//! that is on the machine, run as a program and never linked, the way a `.rar` is listed by
//! `7z`. What comes back down its pipe is a PNG, and from there a frame is a picture like
//! any other: half blocks, or real pixels where the terminal draws them.
//!
//! Everything here runs on a worker. A frame near the end of a long film is a seek and a
//! decode that can take a second, a film on a slow disk longer, and a file `ffmpeg` cannot
//! make sense of is given a time limit rather than the benefit of the doubt.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fcmd_view::media::Film;
use fcmd_view::picture::Picture;

/// The program, by the name looked for on the `PATH`.
pub const PROGRAM: &str = "ffmpeg";

/// How long one still frame may take before `ffmpeg` is stopped.
pub const FRAME_WAIT: Duration = Duration::from_secs(15);

/// How many frames a second a film is played at. A terminal is not a screen for films:
/// every frame is megabytes of escape sequences, and eight of them a second is what the
/// slowest of the protocols keeps up with while still reading as movement.
pub const PLAY_FPS: u32 = 8;

/// Where the quick view and `F3` first look: a tenth of the way in, which is past the
/// black frame and the studio's logo that the first second of most films is.
pub const FIRST_LOOK: f64 = 0.1;

/// Where in the film a frame is wanted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum At {
    /// A part of the whole, for the first frame, asked before the film's length is known.
    Fraction(f64),
    Seconds(f64),
}

/// One frame to fetch.
#[derive(Debug, Clone)]
pub struct Ask {
    pub file: PathBuf,
    pub at: At,
    /// What the container said, when it has been read already; the first ask reads it.
    pub film: Option<Film>,
    /// The most pixels worth having, across and down: the window the frame is drawn in.
    /// `ffmpeg` scales to it, so a 4K frame does not cross a pipe to be averaged away.
    pub bounds: (u32, u32),
    /// This ask's number, and the cell that holds the newest one: a frame nobody is waiting
    /// for any more is not fetched, and one being fetched is abandoned.
    pub number: u64,
    pub newest: Arc<AtomicU64>,
}

/// The same frame of the same file under the same number; which cell holds the newest
/// number is not part of what is asked.
impl PartialEq for Ask {
    fn eq(&self, other: &Ask) -> bool {
        (&self.file, self.at, &self.film, self.bounds, self.number)
            == (
                &other.file,
                other.at,
                &other.film,
                other.bounds,
                other.number,
            )
    }
}

impl Ask {
    fn outdated(&self) -> bool {
        self.newest.load(Ordering::Relaxed) != self.number
    }
}

/// A frame, and the film it came out of.
#[derive(Debug, Clone)]
pub struct Frame {
    pub film: Film,
    /// The moment it was asked at, in seconds.
    pub at: f64,
    pub picture: Picture,
}

/// Why there is no frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    /// The file is not a film: a container of sound alone, or not a container after all.
    /// Nothing to say — it is shown as what it is.
    NotAFilm,
    /// `ffmpeg` is not on the `PATH`. The film is real; the viewer says what to install.
    NoProgram(Film),
    /// `ffmpeg` ran and gave no picture: what it said, or that it ran out of time.
    Failed(String),
    /// Somebody asked for another frame meanwhile. Nothing to say.
    Outdated,
}

/// The size a frame is scaled to inside `bounds`, as `ffmpeg`'s `scale` filter is told:
/// never larger than the film's own, never larger than the window, its shape kept.
fn scale_filter(bounds: (u32, u32)) -> String {
    let (w, h) = (bounds.0.max(16), bounds.1.max(16));
    format!("scale='min({w},iw)':'min({h},ih)':force_original_aspect_ratio=decrease")
}

/// The arguments that make `ffmpeg` write one frame at `at` seconds to its standard output
/// as a PNG.
///
/// `-ss` before `-i` seeks in the container rather than decoding up to the moment, which is
/// what makes the ninetieth minute as quick as the first. `-nostdin` because `ffmpeg` reads
/// keys from its terminal when it has one, and ours is the same terminal. Nothing is written
/// but errors, and they go to the other stream.
pub fn frame_args(file: &Path, at: f64, bounds: (u32, u32)) -> Vec<OsString> {
    let mut args: Vec<OsString> = ["-hide_banner", "-loglevel", "error", "-nostdin", "-ss"]
        .into_iter()
        .map(OsString::from)
        .collect();
    args.push(format!("{:.3}", at.max(0.0)).into());
    args.push("-i".into());
    args.push(file.as_os_str().to_os_string());
    for arg in ["-frames:v", "1", "-an", "-sn", "-vf"] {
        args.push(arg.into());
    }
    args.push(scale_filter(bounds).into());
    for arg in ["-f", "image2pipe", "-vcodec", "png", "-"] {
        args.push(arg.into());
    }
    args
}

/// The arguments that make `ffmpeg` write the film from `from` seconds on as a stream of
/// PNGs, [`PLAY_FPS`] a second, at the speed the film runs (`-re`): it is `ffmpeg` that
/// keeps the time, so nothing here needs a clock.
pub fn play_args(file: &Path, from: f64, bounds: (u32, u32)) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-re",
        "-ss",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    args.push(format!("{:.3}", from.max(0.0)).into());
    args.push("-i".into());
    args.push(file.as_os_str().to_os_string());
    for arg in ["-an", "-sn", "-vf"] {
        args.push(arg.into());
    }
    args.push(format!("fps={PLAY_FPS},{}", scale_filter(bounds)).into());
    for arg in ["-f", "image2pipe", "-vcodec", "png", "-"] {
        args.push(arg.into());
    }
    args
}

/// The moment an ask means, once the film's length is known.
pub fn moment(at: At, film: &Film) -> f64 {
    match at {
        At::Seconds(s) => s.max(0.0),
        At::Fraction(part) => (film.seconds * part.clamp(0.0, 1.0)).min(film.last()),
    }
}

/// Fetch one frame: read the container if nobody has, find `ffmpeg`, run it, decode what it
/// wrote.
pub fn frame(ask: &Ask) -> Result<Frame, Failure> {
    let film = match &ask.film {
        Some(film) => film.clone(),
        None => fcmd_view::film_in(&ask.file).ok_or(Failure::NotAFilm)?,
    };
    let Some(program) = fcmd_arc::which(PROGRAM) else {
        return Err(Failure::NoProgram(film));
    };
    frame_with(&program, ask, film)
}

/// The same with the program already found, so a test can name one.
pub fn frame_with(program: &Path, ask: &Ask, film: Film) -> Result<Frame, Failure> {
    if ask.outdated() {
        return Err(Failure::Outdated);
    }
    let at = moment(ask.at, &film);
    let args = frame_args(&ask.file, at, ask.bounds);
    let png = run(program, &args, FRAME_WAIT, || ask.outdated())?;
    let picture = fcmd_view::picture::read(&png).map_err(Failure::Failed)?;
    Ok(Frame { film, at, picture })
}

/// Run `ffmpeg` and take what it writes: its standard output whole, or the first line of
/// what it complained of.
fn run(
    program: &Path,
    args: &[OsString],
    wait: Duration,
    outdated: impl Fn() -> bool,
) -> Result<Vec<u8>, Failure> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Failure::Failed(format!("{PROGRAM}: {e}")))?;
    let (Some(mut out), Some(mut err)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Failure::Failed(format!("{PROGRAM}: no pipe")));
    };
    // Each stream on a thread of its own: a program that fills one while the other is being
    // waited on would otherwise stop for ever, and so would this.
    let read_out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = out.read_to_end(&mut bytes);
        bytes
    });
    let read_err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = err.read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        let late = started.elapsed() >= wait;
        if late || outdated() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = read_out.join();
            let _ = read_err.join();
            return Err(if late {
                Failure::Failed(format!("{PROGRAM}: no frame after {} s", wait.as_secs()))
            } else {
                Failure::Outdated
            });
        }
        std::thread::sleep(Duration::from_millis(15));
    };
    let bytes = read_out.join().unwrap_or_default();
    let said = read_err.join().unwrap_or_default();
    if !bytes.is_empty() {
        return Ok(bytes);
    }
    // Nothing on the pipe: what it said, or — past the last frame, where `ffmpeg` succeeds
    // with nothing to show — that there was no frame there.
    let said = String::from_utf8_lossy(&said);
    let line = said.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    Err(Failure::Failed(if !line.is_empty() {
        line.trim().to_string()
    } else if status.is_some_and(|s| s.success()) {
        format!("{PROGRAM}: no frame at that moment")
    } else {
        format!("{PROGRAM}: failed")
    }))
}

/// Cut one PNG off the front of a stream of them: how many bytes the first whole file takes,
/// or `None` while it has not all arrived.
///
/// A PNG is its eight-byte signature and then chunks — a length, a four-letter type, that
/// many bytes and a checksum — ending with the one called `IEND`. `ffmpeg` writes them one
/// after another with nothing between, so this is the only way to know where a frame stops.
pub fn png_len(stream: &[u8]) -> Option<usize> {
    const SIGNATURE: usize = 8;
    let mut at = SIGNATURE;
    loop {
        let head = stream.get(at..at + 8)?;
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        let end = at.checked_add(12)?.checked_add(len)?;
        if end > stream.len() {
            return None;
        }
        if &head[4..8] == b"IEND" {
            return Some(end);
        }
        at = end;
    }
}

/// Play a film from `ask`'s moment: every frame `ffmpeg` writes is handed to `show` with the
/// moment it belongs to, until the film ends, `ffmpeg` stops, or the ask is outdated — which
/// is how `Space` pauses. Says how it ended.
///
/// The frames arrive at the film's own speed because `ffmpeg` paces them; this thread only
/// reads. That is the whole clock: nothing in the program counts time for it.
pub fn play(ask: &Ask, mut show: impl FnMut(Frame)) -> Result<(), Failure> {
    let film = match &ask.film {
        Some(film) => film.clone(),
        None => fcmd_view::film_in(&ask.file).ok_or(Failure::NotAFilm)?,
    };
    let Some(program) = fcmd_arc::which(PROGRAM) else {
        return Err(Failure::NoProgram(film));
    };
    play_with(&program, ask, film, &mut show)
}

/// The same with the program already found.
pub fn play_with(
    program: &Path,
    ask: &Ask,
    film: Film,
    show: &mut dyn FnMut(Frame),
) -> Result<(), Failure> {
    use std::process::{Command, Stdio};
    let from = moment(ask.at, &film);
    let mut child = Command::new(program)
        .args(play_args(&ask.file, from, ask.bounds))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Failure::Failed(format!("{PROGRAM}: {e}")))?;
    let Some(mut out) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Failure::Failed(format!("{PROGRAM}: no pipe")));
    };
    let mut stream: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    let mut shown = 0u32;
    let ended = 'film: loop {
        if ask.outdated() {
            break Err(Failure::Outdated);
        }
        match out.read(&mut chunk) {
            Ok(0) | Err(_) => break Ok(()),
            Ok(n) => stream.extend_from_slice(&chunk[..n]),
        }
        while let Some(len) = png_len(&stream) {
            // Several frames can arrive in one read; the one after a stop is not shown.
            if ask.outdated() {
                break 'film Err(Failure::Outdated);
            }
            if let Ok(picture) = fcmd_view::picture::read(&stream[..len]) {
                show(Frame {
                    film: film.clone(),
                    at: from + f64::from(shown) / f64::from(PLAY_FPS),
                    picture,
                });
            }
            shown += 1;
            stream.drain(..len);
        }
    };
    // Stopped or finished, the program is not left running behind a film nobody watches.
    let _ = child.kill();
    let _ = child.wait();
    ended
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn text(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    pub fn ask_for(file: &Path, at: At) -> Ask {
        Ask {
            file: file.to_path_buf(),
            at,
            film: None,
            bounds: (640, 384),
            number: 1,
            newest: Arc::new(AtomicU64::new(1)),
        }
    }

    /// A two-second film written by `ffmpeg` itself, or `None` on a machine without it —
    /// where every test that wants one says so and passes.
    pub fn a_film_in(dir: &Path) -> Option<(PathBuf, PathBuf)> {
        let program = fcmd_arc::which(PROGRAM)?;
        let file = dir.join("film.mkv");
        let made = std::process::Command::new(&program)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
            ])
            .arg("testsrc=duration=2:size=64x48:rate=8")
            .args(["-c:v", "mpeg4", "-y"])
            .arg(&file)
            .stdin(std::process::Stdio::null())
            .output()
            .ok()?;
        (made.status.success() && file.is_file()).then_some((program, file))
    }

    #[test]
    fn one_frame_is_asked_for_by_seeking_first_and_writing_a_png_to_the_pipe() {
        let file = std::env::temp_dir().join("a film.mkv");
        let args = text(&frame_args(&file, 83.5, (1280, 720)));
        let name = file.to_string_lossy().into_owned();
        assert_eq!(
            args,
            [
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-ss",
                "83.500",
                "-i",
                name.as_str(),
                "-frames:v",
                "1",
                "-an",
                "-sn",
                "-vf",
                "scale='min(1280,iw)':'min(720,ih)':force_original_aspect_ratio=decrease",
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "-",
            ]
        );
        // The seek comes before the input, which is what makes it a seek and not a decode
        // of everything before the moment.
        let ss = args.iter().position(|a| a == "-ss").unwrap();
        let input = args.iter().position(|a| a == "-i").unwrap();
        assert!(ss < input);
        // A moment before the start is the start, and a window too small to name is not
        // handed to `ffmpeg` as zero.
        let args = text(&frame_args(&file, -3.0, (0, 0)));
        assert!(args.contains(&"0.000".to_string()));
        assert!(args.iter().any(|a| a.starts_with("scale='min(16,iw)'")));
    }

    #[test]
    fn playing_asks_for_the_film_at_its_own_speed_and_eight_frames_a_second() {
        let file = std::env::temp_dir().join("film.mp4");
        let args = text(&play_args(&file, 12.0, (640, 384)));
        assert!(args.contains(&"-re".to_string()), "ffmpeg keeps the time");
        assert!(args.contains(&"12.000".to_string()));
        assert!(args
            .iter()
            .any(|a| a.starts_with("fps=8,scale='min(640,iw)'")));
        assert!(!args.contains(&"-frames:v".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("-"));
    }

    #[test]
    fn a_tenth_of_the_way_in_is_worked_out_once_the_length_is_known() {
        let film = Film {
            seconds: 5400.0,
            codec: "H.264/AVC".into(),
            width: 1920,
            height: 1080,
        };
        assert_eq!(moment(At::Fraction(FIRST_LOOK), &film), 540.0);
        assert_eq!(moment(At::Seconds(60.0), &film), 60.0);
        assert_eq!(moment(At::Seconds(-1.0), &film), 0.0);
        // The whole of it is the last frame worth asking for, not the instant after it.
        assert_eq!(moment(At::Fraction(1.0), &film), 5399.0);
        // A film that does not say how long it is starts at its start.
        let unknown = Film {
            seconds: 0.0,
            ..film
        };
        assert_eq!(moment(At::Fraction(FIRST_LOOK), &unknown), 0.0);
    }

    #[test]
    fn a_stream_of_pngs_is_cut_where_each_one_ends() {
        // The smallest PNG there is: signature, a header chunk, an end chunk.
        let mut one = b"\x89PNG\r\n\x1a\n".to_vec();
        one.extend_from_slice(&13u32.to_be_bytes());
        one.extend_from_slice(b"IHDR");
        one.extend_from_slice(&[0u8; 13 + 4]);
        one.extend_from_slice(&0u32.to_be_bytes());
        one.extend_from_slice(b"IEND");
        one.extend_from_slice(&[0u8; 4]);
        assert_eq!(png_len(&one), Some(one.len()));
        // Two in a row: the first is found, and what is left is the second.
        let mut two = one.clone();
        two.extend_from_slice(&one);
        assert_eq!(png_len(&two), Some(one.len()));
        // One that has not all arrived yet is waited for, wherever it was cut.
        for cut in [0, 4, 8, 12, 20, one.len() - 1] {
            assert_eq!(png_len(&one[..cut]), None, "cut at {cut}");
        }
        // A length that would run off the end of memory is not followed there.
        let mut wild = b"\x89PNG\r\n\x1a\n".to_vec();
        wild.extend_from_slice(&u32::MAX.to_be_bytes());
        wild.extend_from_slice(b"IDAT");
        assert_eq!(png_len(&wild), None);
    }

    #[test]
    fn a_file_that_is_not_a_film_is_not_handed_to_ffmpeg() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, b"plain text").unwrap();
        assert_eq!(
            frame(&ask_for(&file, At::Fraction(FIRST_LOOK))).err(),
            Some(Failure::NotAFilm)
        );
        // Nor is one that is not there.
        let gone = dir.path().join("gone.mkv");
        assert_eq!(
            frame(&ask_for(&gone, At::Seconds(0.0))).err(),
            Some(Failure::NotAFilm)
        );
    }

    #[test]
    fn a_frame_nobody_waits_for_is_not_fetched() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("film.mkv");
        let film = Film {
            seconds: 2.0,
            codec: "MPEG-4".into(),
            width: 64,
            height: 48,
        };
        let mut ask = ask_for(&file, At::Seconds(1.0));
        ask.number = 1;
        ask.newest = Arc::new(AtomicU64::new(2));
        // The program is never run, so it need not exist.
        let nowhere = dir.path().join("no-such-program");
        assert_eq!(
            frame_with(&nowhere, &ask, film).err(),
            Some(Failure::Outdated)
        );
    }

    #[test]
    fn a_program_that_cannot_be_run_says_so_instead_of_hanging() {
        let dir = tempfile::tempdir().unwrap();
        let film = Film {
            seconds: 2.0,
            codec: "MPEG-4".into(),
            width: 64,
            height: 48,
        };
        let ask = ask_for(&dir.path().join("film.mkv"), At::Seconds(1.0));
        let nowhere = dir.path().join("no-such-program");
        match frame_with(&nowhere, &ask, film) {
            Err(Failure::Failed(why)) => assert!(why.starts_with("ffmpeg: "), "{why}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ffmpeg_gives_a_frame_of_a_real_film_scaled_to_the_window() {
        let dir = tempfile::tempdir().unwrap();
        let Some((_, file)) = a_film_in(dir.path()) else {
            eprintln!("skipped: ffmpeg is not installed here");
            return;
        };
        let got = frame(&ask_for(&file, At::Fraction(FIRST_LOOK))).expect("a frame");
        assert_eq!((got.film.width, got.film.height), (64, 48));
        assert!((got.film.seconds - 2.0).abs() < 0.2, "{}", got.film.seconds);
        assert!(
            (got.at - 0.2).abs() < 0.05,
            "a tenth of two seconds: {}",
            got.at
        );
        // Smaller than the window, so it is left at its own size.
        assert_eq!((got.picture.width, got.picture.height), (64, 48));

        // A window smaller than the film scales it down and keeps its shape.
        let mut small = ask_for(&file, At::Seconds(1.0));
        small.bounds = (32, 32);
        let got = frame(&small).expect("a frame");
        assert_eq!((got.picture.width, got.picture.height), (32, 24));
        assert_eq!(got.at, 1.0);

        // Past the end there is no frame, and that is said in words.
        match frame(&ask_for(&file, At::Seconds(60.0))) {
            Err(Failure::Failed(why)) => assert!(!why.is_empty()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ffmpeg_refuses_a_file_that_only_looks_like_a_film_with_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        let Some((program, _)) = a_film_in(dir.path()) else {
            eprintln!("skipped: ffmpeg is not installed here");
            return;
        };
        let file = dir.path().join("broken.mkv");
        std::fs::write(&file, b"\x1a\x45\xdf\xa3 and then nothing a film has").unwrap();
        let film = Film {
            seconds: 2.0,
            codec: "MPEG-4".into(),
            width: 64,
            height: 48,
        };
        match frame_with(&program, &ask_for(&file, At::Seconds(0.0)), film) {
            Err(Failure::Failed(why)) => assert!(!why.is_empty()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_film_plays_frame_by_frame_until_it_ends_or_is_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let Some((_, file)) = a_film_in(dir.path()) else {
            eprintln!("skipped: ffmpeg is not installed here");
            return;
        };
        // From one second in, a two-second film at eight frames a second has about eight
        // frames left, each a little later than the one before.
        let ask = ask_for(&file, At::Seconds(1.0));
        let mut moments = Vec::new();
        let ended = play(&ask, |frame| {
            assert_eq!((frame.picture.width, frame.picture.height), (64, 48));
            moments.push(frame.at);
        });
        assert_eq!(ended, Ok(()));
        assert!((6..=10).contains(&moments.len()), "{moments:?}");
        assert_eq!(moments[0], 1.0);
        assert_eq!(moments[1], 1.125);
        assert!(moments.windows(2).all(|w| w[1] > w[0]));

        // Stopped after the second frame: the ask is outdated, as `Space` does it, and the
        // film does not run on.
        let ask = ask_for(&file, At::Seconds(0.0));
        let mut seen = 0;
        let ended = play(&ask, |_| {
            seen += 1;
            if seen == 2 {
                ask.newest.store(99, Ordering::Relaxed);
            }
        });
        assert_eq!(ended, Err(Failure::Outdated));
        assert!(seen < 8, "{seen} frames after being told to stop");
    }
}
