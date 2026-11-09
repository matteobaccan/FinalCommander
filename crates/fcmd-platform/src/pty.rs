//! A pseudo-terminal with a program on the far end of it.
//!
//! On Unix the kernel makes the pair (`openpty`) and the program is started with the far end
//! as its standard input, output and error, in a session of its own whose controlling
//! terminal that end is — which is what makes `SIGWINCH` reach it when the size changes and
//! `^C` reach the program in front rather than us. On Windows the pair is ConPTY: two pipes,
//! a pseudo-console between them, and the program started attached to it.
//!
//! What comes back is the near end, as a reader and a writer, and a [`Pty`] that resizes it
//! and waits for or stops the program. Everything else — parsing what the program writes,
//! drawing it — is `fcmd-term`'s, which never sees a file descriptor or a handle.

use std::fs::File;
use std::io;
use std::path::Path;

/// A program running on a pseudo-terminal.
pub struct Pty {
    inner: imp::Pty,
}

impl std::fmt::Debug for Pty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pty").finish_non_exhaustive()
    }
}

/// What [`spawn`] gives back: the terminal, and the near end to read what the program writes
/// and to write what it should read.
pub struct Spawned {
    pub pty: Pty,
    pub reader: File,
    pub writer: File,
}

/// Start `program` with `args` on a new pseudo-terminal of `cols` × `rows`, in `cwd` when
/// one is given, with `env` added to the environment this process has.
///
/// `program` is looked for the way the system looks for a program: on the `PATH`, and on
/// Windows with `.exe` when it has no extension.
pub fn spawn(
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
    env: &[(&str, &str)],
    cols: u16,
    rows: u16,
) -> io::Result<Spawned> {
    let (inner, reader, writer) = imp::spawn(program, args, cwd, env, cols.max(1), rows.max(1))?;
    Ok(Spawned {
        pty: Pty { inner },
        reader,
        writer,
    })
}

impl Pty {
    /// Tell the program the terminal is now `cols` × `rows`.
    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        self.inner.resize(cols.max(1), rows.max(1))
    }

    /// The exit code, if the program has finished. A Unix program ended by a signal counts
    /// as 1: it did not succeed, and the number of the signal is not an exit code.
    pub fn try_wait(&mut self) -> io::Result<Option<u32>> {
        self.inner.try_wait()
    }

    /// Stop the program, and wait for it to be gone.
    pub fn kill(&mut self) {
        self.inner.kill();
    }
}

#[cfg(unix)]
mod imp {
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};

    use nix::libc;

    pub struct Pty {
        master: OwnedFd,
        child: Child,
    }

    fn winsize(cols: u16, rows: u16) -> nix::pty::Winsize {
        nix::pty::Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }

    /// Close-on-exec, so neither end leaks into a program started later — a master end held
    /// open by some other child would keep a read of it from ever seeing end-of-file.
    fn cloexec(fd: &OwnedFd) -> io::Result<()> {
        // SAFETY: `fd` is an open descriptor we own; F_GETFD/F_SETFD touch only its flags.
        unsafe {
            let flags = libc::fcntl(fd.as_raw_fd(), libc::F_GETFD);
            if flags == -1
                || libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, flags | libc::FD_CLOEXEC) == -1
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub fn spawn(
        program: &str,
        args: &[String],
        cwd: Option<&Path>,
        env: &[(&str, &str)],
        cols: u16,
        rows: u16,
    ) -> io::Result<(Pty, File, File)> {
        let pair = nix::pty::openpty(&winsize(cols, rows), None)?;
        cloexec(&pair.master)?;
        cloexec(&pair.slave)?;

        let child = {
            let slave = File::from(pair.slave);
            let mut command = Command::new(program);
            command
                .args(args)
                .stdin(Stdio::from(slave.try_clone()?))
                .stdout(Stdio::from(slave.try_clone()?))
                .stderr(Stdio::from(slave));
            if let Some(cwd) = cwd {
                command.current_dir(cwd);
            }
            for (key, value) in env {
                command.env(key, value);
            }
            // SAFETY: between fork and exec only async-signal-safe calls are made — signal,
            // sigprocmask, setsid and ioctl — and nothing is allocated.
            unsafe {
                command.pre_exec(|| {
                    // What this process ignores or blocks, the program should not inherit:
                    // a shell started with SIGINT ignored cannot be interrupted.
                    for signal in [
                        libc::SIGCHLD,
                        libc::SIGHUP,
                        libc::SIGINT,
                        libc::SIGQUIT,
                        libc::SIGTERM,
                        libc::SIGALRM,
                        libc::SIGPIPE,
                    ] {
                        libc::signal(signal, libc::SIG_DFL);
                    }
                    let empty: libc::sigset_t = std::mem::zeroed();
                    libc::sigprocmask(libc::SIG_SETMASK, &empty, std::ptr::null_mut());
                    // A session of its own, with the terminal as its controlling terminal:
                    // without it a resize sends no SIGWINCH and ^C reaches nobody.
                    if libc::setsid() == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    if libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            command.spawn()?
            // `command` goes here, and with it the last copy of the far end on this side:
            // a read of the near end reports end-of-file once the program has gone.
        };

        let reader = File::from(pair.master.try_clone()?);
        let writer = File::from(pair.master.try_clone()?);
        Ok((
            Pty {
                master: pair.master,
                child,
            },
            reader,
            writer,
        ))
    }

    impl Pty {
        pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
            let size = winsize(cols, rows);
            // SAFETY: TIOCSWINSZ reads one `winsize` from the pointer, which lives across the
            // call; the descriptor is the open master end.
            let done = unsafe {
                libc::ioctl(
                    self.master.as_raw_fd(),
                    libc::TIOCSWINSZ as _,
                    &size as *const nix::pty::Winsize,
                )
            };
            if done == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn try_wait(&mut self) -> io::Result<Option<u32>> {
            Ok(self
                .child
                .try_wait()?
                .map(|status| status.code().map_or(1, |code| code as u32)))
        }

        pub fn kill(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use std::path::Path;
    use std::ptr;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, S_OK, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Console::{COORD, HPCON};
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
        InitializeProcThreadAttributeList, TerminateProcess, UpdateProcThreadAttribute,
        WaitForSingleObject, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT, INFINITE,
        LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
        STARTF_USESTDHANDLES, STARTUPINFOEXW,
    };

    type Create = unsafe extern "system" fn(COORD, HANDLE, HANDLE, u32, *mut HPCON) -> i32;
    type Resize = unsafe extern "system" fn(HPCON, COORD) -> i32;
    type Close = unsafe extern "system" fn(HPCON);
    type Proc = unsafe extern "system" fn() -> isize;

    /// The three ConPTY calls, looked up when first needed rather than imported: a Windows
    /// older than 10 1809 has no ConPTY, and an import it cannot satisfy would stop the whole
    /// program from starting instead of only the terminal.
    struct ConPty {
        create: Create,
        resize: Resize,
        close: Close,
    }

    fn conpty() -> io::Result<&'static ConPty> {
        static CONPTY: std::sync::OnceLock<Option<ConPty>> = std::sync::OnceLock::new();
        CONPTY
            .get_or_init(|| {
                let name = wide(OsStr::new("kernel32.dll"));
                // SAFETY: kernel32 is loaded in every process; the names are NUL-terminated,
                // and each address is turned into the signature the documentation gives.
                unsafe {
                    let module = GetModuleHandleW(name.as_ptr());
                    if module.is_null() {
                        return None;
                    }
                    let create = GetProcAddress(module, c"CreatePseudoConsole".as_ptr().cast())?;
                    let resize = GetProcAddress(module, c"ResizePseudoConsole".as_ptr().cast())?;
                    let close = GetProcAddress(module, c"ClosePseudoConsole".as_ptr().cast())?;
                    Some(ConPty {
                        create: std::mem::transmute::<Proc, Create>(create),
                        resize: std::mem::transmute::<Proc, Resize>(resize),
                        close: std::mem::transmute::<Proc, Close>(close),
                    })
                }
            })
            .as_ref()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::Unsupported,
                    "this Windows has no pseudo-console (ConPTY needs Windows 10 1809 or later)",
                )
            })
    }

    pub struct Pty {
        console: HPCON,
        process: HANDLE,
        exit: Option<u32>,
    }

    // SAFETY: the two handles are kernel objects any thread may use; nothing here is tied to
    // the thread that made it.
    unsafe impl Send for Pty {}
    unsafe impl Sync for Pty {}

    fn wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn size(cols: u16, rows: u16) -> COORD {
        COORD {
            X: cols.min(i16::MAX as u16) as i16,
            Y: rows.min(i16::MAX as u16) as i16,
        }
    }

    /// One argument quoted the way `CommandLineToArgvW` reads it back: whole, with its
    /// quotes and the backslashes before them doubled.
    fn quote(arg: &str, out: &mut String) {
        if !arg.is_empty() && !arg.contains([' ', '\t', '\n', '\u{0b}', '"']) {
            out.push_str(arg);
            return;
        }
        out.push('"');
        let mut backslashes = 0;
        for c in arg.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                    out.push('"');
                    backslashes = 0;
                }
                _ => {
                    out.extend(std::iter::repeat_n('\\', backslashes));
                    out.push(c);
                    backslashes = 0;
                }
            }
        }
        out.extend(std::iter::repeat_n('\\', backslashes * 2));
        out.push('"');
    }

    pub(super) fn command_line(program: &str, args: &[String]) -> String {
        let mut line = String::new();
        quote(program, &mut line);
        for arg in args {
            line.push(' ');
            quote(arg, &mut line);
        }
        line
    }

    /// This process's environment with `env` laid over it, as the block `CreateProcessW`
    /// wants: `KEY=value` NUL-separated, sorted without regard to case, with a NUL at the end.
    fn environment(env: &[(&str, &str)]) -> Vec<u16> {
        let mut vars: Vec<(String, String)> = std::env::vars_os()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.to_string_lossy().into_owned(),
                )
            })
            .filter(|(k, _)| !env.iter().any(|(e, _)| e.eq_ignore_ascii_case(k)))
            .collect();
        vars.extend(env.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        vars.sort_by_key(|(k, _)| k.to_uppercase());
        let mut block = Vec::new();
        for (k, v) in vars {
            block.extend(OsStr::new(&format!("{k}={v}")).encode_wide());
            block.push(0);
        }
        block.push(0);
        block
    }

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: a handle we opened and nobody else closes.
                unsafe { CloseHandle(self.0) };
            }
        }
    }
    impl Handle {
        fn take(mut self) -> HANDLE {
            std::mem::replace(&mut self.0, ptr::null_mut())
        }
    }

    fn pipe() -> io::Result<(Handle, Handle)> {
        let (mut read, mut write): (HANDLE, HANDLE) = (ptr::null_mut(), ptr::null_mut());
        // SAFETY: two out-pointers to locals; no security attributes, the default size.
        if unsafe { CreatePipe(&mut read, &mut write, ptr::null(), 0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((Handle(read), Handle(write)))
    }

    pub fn spawn(
        program: &str,
        args: &[String],
        cwd: Option<&Path>,
        env: &[(&str, &str)],
        cols: u16,
        rows: u16,
    ) -> io::Result<(Pty, File, File)> {
        let api = conpty()?;
        // What we write goes into `input_write` and comes out of `input_read` into the
        // console; what the program prints goes into `output_write` and we read it from
        // `output_read`.
        let (input_read, input_write) = pipe()?;
        let (output_read, output_write) = pipe()?;

        let mut console: HPCON = 0;
        // SAFETY: the two handles are open pipe ends; `console` is an out-pointer to a local.
        let result = unsafe {
            (api.create)(
                size(cols, rows),
                input_read.0,
                output_write.0,
                0,
                &mut console,
            )
        };
        if result != S_OK {
            return Err(io::Error::other(format!(
                "CreatePseudoConsole failed (HRESULT {result:#x})"
            )));
        }
        // The console holds its own copies now.
        drop(input_read);
        drop(output_write);
        let console_guard = ConsoleGuard { api, console };

        // The attribute list that attaches the program to the console.
        let mut list_size = 0usize;
        // SAFETY: the first call only reports the size the list needs.
        unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut list_size) };
        let mut list = vec![0u8; list_size];
        let attributes = list.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
        // SAFETY: `list` is `list_size` bytes, as asked for, and outlives every use below.
        if unsafe { InitializeProcThreadAttributeList(attributes, 1, 0, &mut list_size) } == 0 {
            return Err(io::Error::last_os_error());
        }
        struct ListGuard(LPPROC_THREAD_ATTRIBUTE_LIST);
        impl Drop for ListGuard {
            fn drop(&mut self) {
                // SAFETY: a list that was initialised, deleted once.
                unsafe { DeleteProcThreadAttributeList(self.0) };
            }
        }
        let _list_guard = ListGuard(attributes);
        // SAFETY: the value is the console handle itself, passed by value as the
        // documentation for PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE says.
        if unsafe {
            UpdateProcThreadAttribute(
                attributes,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                console as *const core::ffi::c_void,
                std::mem::size_of::<HPCON>(),
                ptr::null_mut(),
                ptr::null(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }

        // SAFETY: a zeroed STARTUPINFOEXW is the documented starting point.
        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        // No standard handles of ours: the console is the program's only terminal.
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.lpAttributeList = attributes;

        let mut line = wide(OsStr::new(&command_line(program, args)));
        let block = environment(env);
        let dir = cwd.map(|c| wide(c.as_os_str()));
        // SAFETY: zeroed PROCESS_INFORMATION is an out-parameter.
        let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: every pointer is to a NUL-terminated buffer that outlives the call; the
        // command line is mutable, as CreateProcessW requires.
        let made = unsafe {
            CreateProcessW(
                ptr::null(),
                line.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                0,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                block.as_ptr().cast(),
                dir.as_ref().map_or(ptr::null(), |d| d.as_ptr()),
                &startup.StartupInfo,
                &mut info,
            )
        };
        if made == 0 {
            return Err(io::Error::last_os_error());
        }
        drop(Handle(info.hThread));

        // SAFETY: the handles are ours and open; each File takes over one of them.
        let reader = unsafe { File::from_raw_handle(output_read.take() as RawHandle) };
        let writer = unsafe { File::from_raw_handle(input_write.take() as RawHandle) };
        let console = console_guard.keep();
        Ok((
            Pty {
                console,
                process: info.hProcess,
                exit: None,
            },
            reader,
            writer,
        ))
    }

    struct ConsoleGuard {
        api: &'static ConPty,
        console: HPCON,
    }
    impl ConsoleGuard {
        fn keep(mut self) -> HPCON {
            std::mem::replace(&mut self.console, 0)
        }
    }
    impl Drop for ConsoleGuard {
        fn drop(&mut self) {
            if self.console != 0 {
                // SAFETY: a console we made and did not hand on.
                unsafe { (self.api.close)(self.console) };
            }
        }
    }

    impl Pty {
        pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
            let api = conpty()?;
            // SAFETY: the console is open for as long as `self` lives.
            let result = unsafe { (api.resize)(self.console, size(cols, rows)) };
            if result != S_OK {
                return Err(io::Error::other(format!(
                    "ResizePseudoConsole failed (HRESULT {result:#x})"
                )));
            }
            Ok(())
        }

        pub fn try_wait(&mut self) -> io::Result<Option<u32>> {
            if self.exit.is_some() {
                return Ok(self.exit);
            }
            // SAFETY: the process handle is open for as long as `self` lives.
            unsafe {
                if WaitForSingleObject(self.process, 0) != WAIT_OBJECT_0 {
                    return Ok(None);
                }
                let mut code = 0u32;
                if GetExitCodeProcess(self.process, &mut code) == 0 {
                    return Err(io::Error::last_os_error());
                }
                self.exit = Some(code);
            }
            Ok(self.exit)
        }

        pub fn kill(&mut self) {
            if self.exit.is_none() {
                // SAFETY: the process handle is open; terminating a process that has already
                // gone fails harmlessly.
                unsafe {
                    TerminateProcess(self.process, 1);
                    WaitForSingleObject(self.process, INFINITE);
                }
                let _ = self.try_wait();
            }
        }
    }

    impl Drop for Pty {
        fn drop(&mut self) {
            // Closing the console is what ends the reader: until then the output pipe stays
            // open whether the program is running or not.
            if let Ok(api) = conpty() {
                // SAFETY: both were made by `spawn` and are closed only here.
                unsafe { (api.close)(self.console) };
            }
            drop(Handle(self.process));
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    use std::fs::File;
    use std::io;
    use std::path::Path;

    pub struct Pty;

    pub fn spawn(
        _: &str,
        _: &[String],
        _: Option<&Path>,
        _: &[(&str, &str)],
        _: u16,
        _: u16,
    ) -> io::Result<(Pty, File, File)> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no pseudo-terminal on this system",
        ))
    }

    impl Pty {
        pub fn resize(&self, _: u16, _: u16) -> io::Result<()> {
            Ok(())
        }
        pub fn try_wait(&mut self) -> io::Result<Option<u32>> {
            Ok(None)
        }
        pub fn kill(&mut self) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    #[cfg(unix)]
    fn a_program_on_the_terminal_sees_a_terminal_and_its_size() {
        let mut spawned = spawn(
            "sh",
            &[
                "-c".into(),
                "stty size; [ -t 0 ] && echo tty; echo $FCMD_PTY_TEST".into(),
            ],
            None,
            &[("FCMD_PTY_TEST", "here")],
            77,
            13,
        )
        .expect("a terminal opens");
        let mut out = Vec::new();
        let _ = spawned.reader.read_to_end(&mut out);
        let out = String::from_utf8_lossy(&out);
        assert!(out.contains("13 77"), "{out:?}");
        assert!(out.contains("tty"), "{out:?}");
        assert!(out.contains("here"), "{out:?}");
        let mut status = None;
        for _ in 0..200 {
            status = spawned.pty.try_wait().unwrap();
            if status.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(status, Some(0));
    }

    #[test]
    #[cfg(unix)]
    fn what_is_written_is_read_and_a_resize_reaches_the_program() {
        let mut spawned = spawn(
            "sh",
            &["-c".into(), "read line; stty size; echo got-$line".into()],
            None,
            &[],
            80,
            24,
        )
        .expect("a terminal opens");
        spawned.pty.resize(100, 30).expect("it resizes");
        spawned.writer.write_all(b"hello\n").unwrap();
        let mut out = Vec::new();
        let _ = spawned.reader.read_to_end(&mut out);
        let out = String::from_utf8_lossy(&out);
        assert!(out.contains("30 100"), "{out:?}");
        assert!(out.contains("got-hello"), "{out:?}");
    }

    #[test]
    #[cfg(unix)]
    fn kill_stops_a_program_that_would_run_for_ever() {
        let mut spawned = spawn("sleep", &["1000".into()], None, &[], 80, 24).unwrap();
        spawned.pty.kill();
        assert!(spawned.pty.try_wait().unwrap().is_some());
    }

    #[test]
    #[cfg(windows)]
    fn arguments_are_quoted_the_way_windows_reads_them_back() {
        assert_eq!(imp::command_line("cmd.exe", &[]), "cmd.exe");
        assert_eq!(
            imp::command_line(
                "a b",
                &["c".into(), "".into(), r#"x"y"#.into(), r"d\".into()]
            ),
            r#""a b" c "" "x\"y" "d\\""#
        );
    }

    #[test]
    #[cfg(windows)]
    fn a_program_runs_on_the_pseudo_console() {
        let mut spawned = spawn(
            "cmd.exe",
            &["/c".into(), "echo %FCMD_PTY_TEST%".into()],
            None,
            &[("FCMD_PTY_TEST", "here")],
            80,
            24,
        )
        .expect("a console opens");
        let mut status = None;
        for _ in 0..500 {
            status = spawned.pty.try_wait().unwrap();
            if status.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(status, Some(0));
        let reader = std::thread::spawn(move || {
            let mut out = Vec::new();
            let _ = spawned.reader.read_to_end(&mut out);
            out
        });
        drop(spawned.pty);
        let out = reader.join().unwrap();
        assert!(String::from_utf8_lossy(&out).contains("here"));
        let _ = spawned.writer.flush();
    }
}
