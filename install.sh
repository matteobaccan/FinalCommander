#!/bin/sh
# FinalCommander -- an orthodox file manager for the terminal.
# Copyright (c) Matteo Baccan -- https://github.com/matteobaccan/FinalCommander
# SPDX-License-Identifier: MIT
#
# Installs `fcmd` from the latest release (or FCMD_VERSION), for the machine it runs on:
#
#   curl -fsSL https://github.com/matteobaccan/FinalCommander/releases/latest/download/install.sh | sh
#   wget -qO- https://github.com/matteobaccan/FinalCommander/releases/latest/download/install.sh | sh
#
# Linux (x86-64, x86 32-bit — iSH on an iPad or iPhone —, ARM 64-bit, ARM 32-bit), macOS,
# and Termux on Android. It picks the release's archive for the system, checks it against
# the release's SHA256SUMS, and puts `fcmd` (and its manual page, where there is a place for
# one) in:
#   FCMD_INSTALL_DIR, if set;
#   $PREFIX/bin in Termux;
#   /usr/local/bin when run as root, or when it is writable;
#   ~/.local/bin otherwise (said so if it is not on PATH).
# FCMD_VERSION=0.46.1 installs that version rather than the latest. Nothing else is touched,
# and nothing is asked for: no sudo, no configuration, no service.
set -eu

repo=matteobaccan/FinalCommander
say() { printf 'fcmd install: %s\n' "$*" >&2; }
die() { say "$*"; exit 1; }

# --- How to download -----------------------------------------------------------------
if command -v curl > /dev/null 2>&1; then
    fetch() { curl -fsSL "$1"; }
    fetch_to() { curl -fsSL -o "$2" "$1"; }
elif command -v wget > /dev/null 2>&1; then
    fetch() { wget -qO- "$1"; }
    fetch_to() { wget -qO "$2" "$1"; }
else
    die "needs curl or wget"
fi

# --- Which archive -------------------------------------------------------------------
os=$(uname -s)
machine=$(uname -m)
system=$(uname -o 2> /dev/null || true)
case "$machine" in
    x86_64 | amd64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    armv7* | armv8l) arch=armv7 ;;
    i386 | i486 | i586 | i686 | x86) arch=i686 ;;
    *) die "no build for this processor ($machine)" ;;
esac
case "$os" in
    Darwin) platform=macos-universal ;;
    Linux)
        if [ "$system" = Android ]; then
            # Termux: the Android build, linked against the phone's own C library.
            case "$arch" in
                aarch64 | armv7) platform=android-$arch ;;
                *) die "no Android build for $machine" ;;
            esac
        else
            platform=linux-$arch-static
        fi
        ;;
    *) die "no build for $os here; Windows has a ZIP on the release page" ;;
esac
platform=${FCMD_PLATFORM:-$platform}

# --- Which version -------------------------------------------------------------------
version=${FCMD_VERSION:-}
if [ -z "$version" ]; then
    version=$(fetch "https://api.github.com/repos/$repo/releases/latest" \
        | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' | head -n 1)
    [ -n "$version" ] || die "could not learn the latest version (set FCMD_VERSION)"
fi
version=${version#v}

# --- Where ---------------------------------------------------------------------------
if [ -n "${FCMD_INSTALL_DIR:-}" ]; then
    dir=$FCMD_INSTALL_DIR
elif [ "$system" = Android ] && [ -n "${PREFIX:-}" ]; then
    dir=$PREFIX/bin
elif [ "$(id -u)" = 0 ] || [ -w /usr/local/bin ]; then
    dir=/usr/local/bin
else
    dir=$HOME/.local/bin
fi

# --- Download, check, install --------------------------------------------------------
name=FinalCommander-$platform-$version.tar.gz
base=https://github.com/$repo/releases/download/v$version
tmp=$(mktemp -d 2> /dev/null || mktemp -d -t fcmd)
trap 'rm -rf "$tmp" || :' EXIT INT TERM

say "FinalCommander $version for $platform"
fetch_to "$base/$name" "$tmp/$name" || die "could not download $base/$name"

if fetch_to "$base/SHA256SUMS" "$tmp/SHA256SUMS" 2> /dev/null; then
    want=$(grep " $name\$" "$tmp/SHA256SUMS" | cut -d ' ' -f 1)
    if command -v sha256sum > /dev/null 2>&1; then
        got=$(sha256sum "$tmp/$name" | cut -d ' ' -f 1)
    elif command -v shasum > /dev/null 2>&1; then
        got=$(shasum -a 256 "$tmp/$name" | cut -d ' ' -f 1)
    else
        got=
        say "no sha256sum or shasum here: the archive is not checked"
    fi
    if [ -n "$got" ]; then
        [ -n "$want" ] || die "$name is not in the release's SHA256SUMS"
        [ "$got" = "$want" ] || die "$name does not match the release's SHA256SUMS"
        say "checksum ok"
    fi
else
    say "the release has no SHA256SUMS: the archive is not checked"
fi

mkdir -p "$tmp/x" "$dir"
tar xzf "$tmp/$name" -C "$tmp/x"
[ -f "$tmp/x/fcmd" ] || die "no fcmd in $name"
chmod 755 "$tmp/x/fcmd"
# Replaced by a rename on the same file system, so a running fcmd keeps its own file.
cp "$tmp/x/fcmd" "$dir/.fcmd.new"
mv -f "$dir/.fcmd.new" "$dir/fcmd"
say "installed $dir/fcmd"

if [ -f "$tmp/x/fcmd.1" ]; then
    man=$(dirname "$dir")/share/man/man1
    if mkdir -p "$man" 2> /dev/null && cp "$tmp/x/fcmd.1" "$man/" 2> /dev/null; then
        say "manual page in $man"
    fi
fi

"$dir/fcmd" --version || die "$dir/fcmd was installed but does not run on this machine"
case ":$PATH:" in
    *":$dir:"*) ;;
    *) say "$dir is not on PATH: add it, or run $dir/fcmd" ;;
esac
