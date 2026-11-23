#!/usr/bin/env bash
# The notes of a GitHub Release, from CHANGELOG.md: where to download it, then what is new.
#
#   tools/release-notes.sh 0.44.0 [CHANGELOG.md] > notes.md
#
# The section `## [<version>] — <date>` of the changelog opens with a paragraph that says in
# a few lines what the release is about; then come its `### Added`, `### Changed` and
# `### Fixed`. The notes put the downloads first, a table by system, and the section after
# them under "What's new", so the first thing on the page is what to take and the second is
# why. `GITHUB_REPOSITORY` names the repository the links point to.
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

| System | Download |
|---|---|
| **Windows** x86-64 | [ZIP]($(archive windows-x86_64 zip)) |
| **Linux** x86-64 | [tar.gz]($(archive linux-x86_64 tar.gz)) · [static, for any distribution]($(archive linux-x86_64-static tar.gz)) |
| **Linux** ARM 64-bit | [static, for any distribution]($(archive linux-aarch64-static tar.gz)) |
| **Linux** ARM 32-bit (ARMv7) | [static, for any distribution]($(archive linux-armv7-static tar.gz)) |
| **Android** ARM 64-bit | [APK, the app]($(archive android-aarch64 apk)) · [tar.gz, for Termux]($(archive android-aarch64 tar.gz)) |
| **Android** ARM 32-bit (ARMv7) | [APK, the app]($(archive android-armv7 apk)) · [tar.gz, for Termux]($(archive android-armv7 tar.gz)) |
| **macOS** Intel and Apple Silicon | [tar.gz]($(archive macos-universal tar.gz)) |

Each archive holds the \`fcmd\` executable, \`README.md\`, \`LICENSE\`, \`THIRD-PARTY.md\` and
\`CHANGELOG.md\`, and on Linux and macOS the manual page \`fcmd.1\`.
\`SHA256SUMS\` holds the SHA-256 of every archive (\`sha256sum -c SHA256SUMS --ignore-missing\`).
A Mac archive downloaded with a browser needs \`xattr -d com.apple.quarantine fcmd\` once.
The Android app is signed with a key made for each build: uninstall an earlier version
before installing this one.

## What's new

${section}
EOF
