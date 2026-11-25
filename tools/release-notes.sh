#!/usr/bin/env bash
# The notes of a GitHub Release, from CHANGELOG.md: where to download it, then what is new.
#
#   tools/release-notes.sh 0.44.0 [CHANGELOG.md] > notes.md
#
# The section `## [<version>] — <date>` of the changelog opens with a paragraph that says in
# a few lines what the release is about; then come its `### Added`, `### Changed` and
# `### Fixed`. The notes put the downloads first — a table of every build, what it runs on,
# when to take it and why it exists — and the section after them under "What's new", so the
# first thing on the page is what to take and the second is what changed. `GITHUB_REPOSITORY` names the repository the links point to.
set -euo pipefail

version="${1:?usage: release-notes.sh <version> [changelog]}"
version="${version#v}"
changelog="${2:-CHANGELOG.md}"
repo="${GITHUB_REPOSITORY:-matteobaccan/FinalCommander}"

# The section for this version, without its heading and without the blank lines around it.
section="$(awk -v v="$version" '
  index($0, "## [" v "]") == 1 { found = 1; next }
  found && /^## / { exit }
  found { print }
' "$changelog" | sed -e '/./,$!d' | sed -e ':a' -e '/^\n*$/{$d;N;ba' -e '}')"

if [ -z "$section" ]; then
  echo "release-notes.sh: CHANGELOG has no section for $version" >&2
  exit 1
fi

base="https://github.com/${repo}/releases/download/v${version}"
archive() { echo "${base}/FinalCommander-$1-${version}.$2"; }

cat <<EOF
## Downloads

On Linux, macOS, Termux or iSH, \`install.sh\` picks the archive for the machine, checks it
against \`SHA256SUMS\` and installs \`fcmd\`:

\`\`\`sh
curl -fsSL https://github.com/${repo}/releases/latest/download/install.sh | sh
# or, where there is no curl:
wget -qO- https://github.com/${repo}/releases/latest/download/install.sh | sh
\`\`\`

Or take the one for your machine. Every one is the same program, with the same features
and the same keys; they differ in what they run on.

| Download | Runs on | Take it when | Why it exists |
|---|---|---|---|
| [**Windows** x86-64]($(archive windows-x86_64 zip)) (ZIP) | Windows 10 and 11, 64-bit | a Windows PC | The C runtime is inside, so it starts on a freshly installed Windows with nothing else to install |
| [**Linux** x86-64]($(archive linux-x86_64 tar.gz)) | a 64-bit PC or server with a current distribution | the usual Linux machine | Built against the system's own C library (glibc): the smallest Linux build |
| [**Linux** x86-64, static]($(archive linux-x86_64-static tar.gz)) | any 64-bit Linux, old or new; Alpine; containers | the one above says \`GLIBC_… not found\`, or you want one file to copy anywhere | Linked with musl and asks the system for no library at all |
| [**Linux** ARM 64-bit, static]($(archive linux-aarch64-static tar.gz)) | Raspberry Pi 3, 4 and 5 with a 64-bit system; ARM servers (Graviton, Ampere); Linux in a browser tab | \`uname -m\` says \`aarch64\` | ARM boards and servers are everywhere now, and it is also the build [the browser demo](https://matteobaccan.github.io/FinalCommander/try/) runs |
| [**Linux** ARM 32-bit, static]($(archive linux-armv7-static tar.gz)) | Raspberry Pi with a 32-bit system, Pi 2, older boards and NAS | \`uname -m\` says \`armv7l\` | Many Raspberry Pi still run 32-bit Raspberry Pi OS, which cannot start the 64-bit build |
| [**Linux** MIPS little-endian]($(archive linux-mipsel-static tar.gz)) · [compact]($(archive linux-mipsel-compact-static tar.gz)) | routers and boards with a little-endian MIPS (MediaTek MT7621 and kin), OpenWrt | \`uname -m\` says \`mips\` and \`install.sh\` finds it little-endian | To manage a router's files from its own shell. Built for size (about 19 MB): keep it on a USB stick or in \`/tmp\`. The compact one, about 13 MB (5.5 MB compressed), is for a router with little flash: English alone, and no WebAssembly, pictures, x86 disassembler, SQLite or web terminal (\`FCMD_COMPACT=1\` makes \`install.sh\` take it) |
| [**Linux** MIPS big-endian]($(archive linux-mips-static tar.gz)) · [compact]($(archive linux-mips-compact-static tar.gz)) | older routers on Atheros and Qualcomm chips, OpenWrt | \`uname -m\` says \`mips\` and \`install.sh\` finds it big-endian | The same two, for the routers whose MIPS stores numbers the other way round |
| [**Linux** x86 32-bit, static]($(archive linux-i686-static tar.gz)) | old 32-bit PCs; iSH on iPad and iPhone | \`uname -m\` says \`i686\` | iSH runs Alpine Linux on iOS by emulating a 32-bit Intel processor. On iSH it does not start yet: its emulator lacks SSE instructions every Rust program uses, and a change to iSH adding them is proposed |
| [**Android** ARM 64-bit]($(archive android-aarch64 apk)) (APK) · [for Termux]($(archive android-aarch64 tar.gz)) | today's Android phones and tablets, Android 8 or later | a phone: the APK; Termux: the tar.gz | The app brings a terminal, with a row of the keys a phone keyboard lacks, the phone's files, its clipboard and its apps to open a file with |
| [**Android** ARM 32-bit]($(archive android-armv7 apk)) (APK) · [for Termux]($(archive android-armv7 tar.gz)) | older or cheaper phones with a 32-bit Android | the 64-bit APK does not install (*Settings → About phone* shows the processor) | The same, for the phones a 64-bit app does not reach |
| [**macOS**]($(archive macos-universal tar.gz)) | any Mac, Intel or Apple Silicon | a Mac | One file with both processors inside, so there is no wrong one to pick |

Each archive holds the \`fcmd\` executable, \`README.md\`, \`LICENSE\`, \`THIRD-PARTY.md\` and
\`CHANGELOG.md\`, and on Linux and macOS the manual page \`fcmd.1\`.
\`SHA256SUMS\` holds the SHA-256 of every file (\`sha256sum -c SHA256SUMS --ignore-missing\`).
A Mac archive downloaded with a browser needs \`xattr -d com.apple.quarantine fcmd\` once.
The Android app is signed with a key made for each build: uninstall an earlier version
before installing this one.

## What's new

${section}
EOF
