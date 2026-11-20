#!/bin/sh
# FinalCommander -- an orthodox file manager for the terminal.
# Copyright (c) Matteo Baccan -- https://github.com/matteobaccan/FinalCommander
# SPDX-License-Identifier: MIT
#
# packaging/android/emulator-test.sh APK FCMD
#
# Run on Android's emulator by .github/workflows/android.yml, with `adb` on the PATH:
# 1. the executable from `adb shell`: `fcmd --version`, and `fcmd --web-terminal` serving
#    its page and refusing it without the token;
# 2. the app: installed, started, its fcmd's page reached through `adb forward`, and the
#    page's WebSocket answering the handshake;
# 3. the app stopped, and its fcmd with it.
set -eu
[ $# -eq 2 ] || { echo "usage: packaging/android/emulator-test.sh APK FCMD" >&2; exit 2; }
apk=$1
exe=$2
pkg=io.github.matteobaccan.finalcommander
local_port=18080

fail() {
    echo "FAIL: $*" >&2
    echo "--- the app's log (adb logcat -s FinalCommander) ---" >&2
    adb logcat -d -s FinalCommander:V >&2 || true
    echo "--- fcmd's processes ---" >&2
    adb shell 'ps -A -o PID,PPID,NAME,ARGS | grep -i fcmd | grep -v grep' >&2 || true
    exit 1
}

# What the page and the WebSocket answer at the forwarded port, with `url`'s token.
check_page() {
    url=$1
    token=${url#*\?t=}
    curl -fsS "http://127.0.0.1:$local_port/?t=$token" > page.html || fail "GET / failed"
    grep -q 'xterm.js' page.html || fail "GET / is not the terminal's page"
    code=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$local_port/?t=wrong")
    [ "$code" = 403 ] || fail "GET / without the token answered $code, not 403"
    head=$(curl -s -i --max-time 3 -H 'Connection: Upgrade' -H 'Upgrade: websocket' \
        -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
        "http://127.0.0.1:$local_port/ws?t=$token&c=80&r=24" || true)
    echo "$head" | head -n 1
    echo "$head" | grep -q '^HTTP/1.1 101' || fail "the WebSocket was not opened"
    echo "$head" | grep -q 's3pPLMBiTxaQ9kYGzzhZRbK+xOo=' || fail "the handshake's answer is wrong"
}

adb wait-for-device
echo "device: $(adb shell getprop ro.product.model) (Android $(adb shell getprop ro.build.version.release), $(adb shell getprop ro.product.cpu.abi))"

echo "== 1. fcmd from adb shell"
adb push "$exe" /data/local/tmp/fcmd > /dev/null
adb shell chmod 755 /data/local/tmp/fcmd
adb shell 'HOME=/data/local/tmp /data/local/tmp/fcmd --version; echo "exit=$?"' > version.txt
cat version.txt
grep -q '^exit=0' version.txt || fail "fcmd --version did not exit 0"
grep -q '^fcmd ' version.txt || fail "fcmd --version said no version"
adb shell 'cd /data/local/tmp && HOME=/data/local/tmp nohup ./fcmd --web-terminal 0 > wt.txt 2>&1 &'
url=
i=0
while [ $i -lt 30 ]; do
    url=$(adb shell cat /data/local/tmp/wt.txt 2> /dev/null | tr -d '\r' | grep '^http://127.0.0.1:' | head -n 1 || true)
    [ -n "$url" ] && break
    sleep 1
    i=$((i + 1))
done
[ -n "$url" ] || fail "fcmd --web-terminal printed no address in 30 s: $(adb shell cat /data/local/tmp/wt.txt)"
port=$(echo "$url" | sed -n 's|http://127.0.0.1:\([0-9]*\)/.*|\1|p')
adb forward "tcp:$local_port" "tcp:$port" > /dev/null
check_page "$url"
adb shell 'pkill -f "fcmd --web-terminal"' || true
adb forward --remove "tcp:$local_port" || true

echo "== 2. the app"
adb install -r "$apk" > /dev/null || fail "adb install refused $apk"
adb logcat -c
# The permission a file manager asks for, given as the user would in the settings, so the
# app starts fcmd at once and on the phone's files.
adb shell appops set "$pkg" MANAGE_EXTERNAL_STORAGE allow || fail "appops refused the files permission"
adb shell am start -W -n "$pkg/.MainActivity" > /dev/null || fail "the activity did not start"
i=0
until adb logcat -d -s FinalCommander:I | grep -q 'http://127.0.0.1:'; do
    i=$((i + 1))
    [ $i -lt 60 ] || break
    sleep 1
done
line=$(adb logcat -d -s FinalCommander:I | grep '^.*http://127.0.0.1:' | tail -n 1 || true)
url=$(echo "$line" | sed -n 's|.*\(http://127.0.0.1:[0-9]*/?t=[0-9a-f]*\).*|\1|p')
[ -n "$url" ] || fail "the app logged no address in 60 s"
port=$(echo "$url" | sed -n 's|http://127.0.0.1:\([0-9]*\)/.*|\1|p')
echo "the app's fcmd serves on port $port"
adb forward "tcp:$local_port" "tcp:$port" > /dev/null
check_page "$url"
adb shell pidof libfcmd.so > /dev/null || fail "no libfcmd.so process while the app runs"

echo "== 3. the app stopped"
adb shell am force-stop "$pkg"
i=0
while adb shell pidof libfcmd.so > /dev/null 2>&1; do
    i=$((i + 1))
    [ $i -lt 15 ] || fail "fcmd still runs 15 s after the app stopped"
    sleep 1
done
adb forward --remove "tcp:$local_port" || true
echo "OK: fcmd runs from adb shell and serves its page; the app starts it, shows it and stops it"
