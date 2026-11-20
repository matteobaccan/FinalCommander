# FinalCommander on Android

Two things come out of `packaging/android/build.sh`, and both are on every release page:

- **`FinalCommander-android-aarch64-<version>.apk`** — the app. Install it (Android asks to
  allow installs from the browser or the file manager the first time), open it, and say
  whether it may see all the phone's files: with that permission the panels open on the
  phone's storage, without it in a folder of the app's own.
- **`FinalCommander-android-aarch64-<version>.tar.gz`** — the executable alone, for Termux
  or `adb shell`, where it runs in the terminal that is already there.

## How the app is made

fcmd is a program for a terminal, and a phone has none, so the app brings one: it starts
`fcmd --web-terminal 0`, which listens on `127.0.0.1` with a token chosen for the run and
serves a page — a terminal written in JavaScript (xterm.js, MIT, compiled into `fcmd`) —
whose WebSocket is joined to a pseudo-terminal on which fcmd runs (SPEC 11 §2a). The app is
a web view on that page, and nothing else:

- `java/…/MainActivity.java` starts the executable, reads the address from its first line
  of output, shows the page, sends the Back button as `Esc`, and stops fcmd when it is
  closed;
- the page puts a row of keys above the phone's keyboard: `Esc`, `Tab`, `F1`–`F12`, the
  arrows, `PgUp`/`PgDn`, `Home`/`End`, `Ins`/`Del`, and `Ctrl`, `Alt` and `Shift`, which
  hold until the next key — so `Alt` then `F7` is FAR's search;
- the executable is `lib/<abi>/libfcmd.so` in the APK: the one place Android lets an app
  run a file it ships. It is `fcmd` built with the NDK against Android's own C library, so
  names resolve with the phone's DNS (the static musl build of the Linux archives would not
  find `/etc/resolv.conf` there).

No Gradle: the SDK's `aapt2`, `javac`, `d8`, `zipalign` and `apksigner`, the way inga's
app is built. The signature is a debug key made for each build, which needs no secret and
means one thing to know: **Android installs a new version over an old one only after the
old one is uninstalled**, because the two keys differ.

## Building it

```sh
rustup target add aarch64-linux-android
export ANDROID_HOME=…            # an SDK with build-tools and a platform
export ANDROID_NDK_HOME=…        # NDK r27 or later
sh packaging/android/build.sh --abi arm64-v8a --out dist
```

`.github/workflows/android.yml` does the same on every change to the app and to the web
terminal, and builds the x86_64 one for Android's emulator, where
`packaging/android/emulator-test.sh` runs the executable from `adb shell`, installs the
app, reaches its page and its WebSocket through `adb forward`, and stops it.
