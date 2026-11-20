#!/bin/sh
# FinalCommander -- an orthodox file manager for the terminal.
# Copyright (c) Matteo Baccan -- https://github.com/matteobaccan/FinalCommander
# SPDX-License-Identifier: MIT
#
# packaging/android/build.sh [--abi arm64-v8a|x86_64] [--out DIR]
#
# Builds fcmd for Android and its app (packaging/android/README.md):
# 1. `fcmd` for the ABI's Rust target (aarch64-linux-android, or x86_64-linux-android for
#    the emulator of the tests), linked by the Android NDK's clang against the phone's own
#    libc, which every Android has: no runtime dependency, and the phone's DNS;
# 2. DIR/FinalCommander-android-<arch>-<version>.tar.gz: the executable, for Termux or
#    `adb shell`, with the documents every archive carries;
# 3. DIR/FinalCommander-android-<arch>-<version>.apk: the app (AndroidManifest.xml, java/,
#    res/), the executable inside as lib/<abi>/libfcmd.so, compiled with the SDK's aapt2,
#    javac and d8 (no Gradle), aligned and signed with a debug key made for this build.
#
# Needs: the Rust target installed (rustup target add aarch64-linux-android), the Android
# NDK r27 or later (ANDROID_NDK_LATEST_HOME, ANDROID_NDK_HOME or ANDROID_NDK_ROOT), the
# Android SDK with a platform and build-tools (ANDROID_HOME or ANDROID_SDK_ROOT), a JDK
# (javac, keytool) and zip. GitHub's Ubuntu runners have them all
# (.github/workflows/android.yml); nothing is downloaded here.
set -eu

abi=arm64-v8a
out=dist
while [ $# -gt 0 ]; do
    case "$1" in
        --abi) abi=$2; shift 2 ;;
        --out) out=$2; shift 2 ;;
        *) echo "usage: packaging/android/build.sh [--abi arm64-v8a|x86_64] [--out DIR]" >&2; exit 2 ;;
    esac
done
case "$abi" in
    arm64-v8a) triple=aarch64-linux-android arch=aarch64 ;;
    x86_64) triple=x86_64-linux-android arch=x86_64 ;;
    *) echo "error: unknown ABI $abi (arm64-v8a or x86_64)" >&2; exit 2 ;;
esac
# The oldest Android the app runs on: 8.0 (API 26), the first with Process.isAlive.
API=26
# The Android the app is written for (Play's rule since 2024; installs everywhere).
TARGET_API=34

root=$(cd "$(dirname "$0")/../.." && pwd)
here="$root/packaging/android"
cd "$root"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "error: no version in Cargo.toml" >&2; exit 1; }

# --- The NDK and the SDK -----------------------------------------------------------
ndk=${ANDROID_NDK_LATEST_HOME:-${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}}
if [ -z "$ndk" ] || [ ! -d "$ndk" ]; then
    echo "error: no Android NDK: set ANDROID_NDK_HOME to an NDK r27 or later" >&2
    exit 1
fi
rev=$(sed -n 's/^Pkg.Revision *= *\([0-9]*\).*/\1/p' "$ndk/source.properties" 2>/dev/null || true)
if [ -z "$rev" ] || [ "$rev" -lt 27 ]; then
    echo "error: the NDK in $ndk is r${rev:-?}; fcmd needs r27 or later" >&2
    exit 1
fi
tc="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
[ -d "$tc" ] || { echo "error: no LLVM toolchain in $ndk (a Linux x86-64 host is needed)" >&2; exit 1; }

sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}
if [ -z "$sdk" ] || [ ! -d "$sdk/build-tools" ] || [ ! -d "$sdk/platforms" ]; then
    echo "error: no Android SDK with build-tools and a platform: set ANDROID_HOME" >&2
    exit 1
fi
bt=$(ls -d "$sdk"/build-tools/* | sort -V | tail -n 1)
jar=$(ls -d "$sdk"/platforms/android-* | sort -V | tail -n 1)/android.jar
[ -f "$jar" ] || { echo "error: no android.jar in $sdk/platforms" >&2; exit 1; }
echo "NDK r$rev ($ndk), build-tools $(basename "$bt"), $(basename "$(dirname "$jar")")"

# --- 1. The executable -------------------------------------------------------------
upper=$(echo "$triple" | tr 'a-z-' 'A-Z_')
under=$(echo "$triple" | tr '-' '_')
clang="$tc/$triple$API-clang"
export "CARGO_TARGET_${upper}_LINKER=$clang"
# 16 KiB pages: Android 15 phones may use them, and an executable must be aligned to the
# page size to load.
export "CARGO_TARGET_${upper}_RUSTFLAGS=-C link-arg=-Wl,-z,max-page-size=16384"
export "CC_${under}=$clang"
export "CXX_${under}=$clang++"
export "AR_${under}=$tc/llvm-ar"
cargo build --release --locked --target "$triple" -p fcmd
bin="target/$triple/release/fcmd"

work="target/android-$abi"
rm -rf "$work"
mkdir -p "$work/stage" "$out"
cp "$bin" "$work/stage/fcmd"
"$tc/llvm-strip" "$work/stage/fcmd"

# --- 2. The archive ----------------------------------------------------------------
asset="FinalCommander-android-$arch-$version"
cp README.md LICENSE THIRD-PARTY.md CHANGELOG.md docs/fcmd.1 "$work/stage/"
tar -C "$work/stage" -czf "$out/$asset.tar.gz" fcmd README.md LICENSE THIRD-PARTY.md \
    CHANGELOG.md fcmd.1

# --- 3. The app --------------------------------------------------------------------
# The version code Android compares on an update: 0.46.0 -> 4600, 1.2.3 -> 10203.
code=$(echo "$version" | awk -F'[.-]' '{ print $1 * 10000 + $2 * 100 + $3 }')
apk="$work/apk"
mkdir -p "$apk/gen" "$apk/classes" "$apk/dex" "$apk/assets" "$apk/zip/lib/$abi"
cp LICENSE THIRD-PARTY.md "$apk/assets/"
"$bt/aapt2" compile --dir "$here/res" -o "$apk/res.zip"
"$bt/aapt2" link -o "$apk/base.apk" -I "$jar" \
    --manifest "$here/AndroidManifest.xml" \
    --min-sdk-version "$API" --target-sdk-version "$TARGET_API" \
    --version-code "$code" --version-name "$version" \
    -A "$apk/assets" --java "$apk/gen" "$apk/res.zip"
# shellcheck disable=SC2046 # the source files, none with a space
javac -source 8 -target 8 -bootclasspath "$jar" -Xlint:-options -encoding UTF-8 \
    -d "$apk/classes" \
    $(find "$here/java" "$apk/gen" -name '*.java')
# shellcheck disable=SC2046 # the class files, none with a space
"$bt/d8" --release --min-api "$API" --lib "$jar" --output "$apk/dex" \
    $(find "$apk/classes" -name '*.class')
cp "$apk/dex/classes.dex" "$apk/zip/"
cp "$work/stage/fcmd" "$apk/zip/lib/$abi/libfcmd.so"
cp "$apk/base.apk" "$apk/unaligned.apk"
(cd "$apk/zip" && zip -qr ../unaligned.apk classes.dex lib)
"$bt/zipalign" -f -p 4 "$apk/unaligned.apk" "$apk/aligned.apk"
# A debug key made for this build: no secret, and an APK any phone installs once
# "install unknown apps" is allowed. A later build has another key, so Android installs
# it over an earlier one only after that is uninstalled.
keytool -genkeypair -keystore "$apk/debug.keystore" -storepass android -keypass android \
    -alias fcmd-debug -keyalg RSA -keysize 2048 -validity 10000 \
    -dname "CN=FinalCommander debug build, O=FinalCommander" > /dev/null 2>&1
"$bt/apksigner" sign --ks "$apk/debug.keystore" --ks-pass pass:android \
    --key-pass pass:android --ks-key-alias fcmd-debug \
    --out "$out/$asset.apk" "$apk/aligned.apk"
"$bt/apksigner" verify --print-certs "$out/$asset.apk" | head -n 3
ls -la "$out"
