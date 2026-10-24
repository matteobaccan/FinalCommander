#!/bin/sh
# `xdg-open` on Linux, `open` on macOS. The one thing a terminal file manager cannot do for
# itself: hand something to the graphical session.
#
# It does not wait for the program it starts. A picture viewer that stays open for ten minutes
# would otherwise hold this plugin's timeout open and then be killed with it.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

if command -v xdg-open >/dev/null 2>&1; then
  opener=xdg-open
elif command -v open >/dev/null 2>&1; then
  opener=open
elif command -v gio >/dev/null 2>&1; then
  opener="gio open"
else
  echo '{"error":"no xdg-open or open here — this needs a desktop session"}'
  exit 0
fi

case "$SCMD_COMMAND" in
  *.folder)
    $opener . >/dev/null 2>&1 &
    echo '{"status":"handed this folder to the desktop"}'
    exit 0 ;;
esac

names="$SCMD_SELECTED"
[ -n "$names" ] || names="$SCMD_CURSOR"
[ -n "$names" ] || { echo '{"status":"nothing under the cursor"}'; exit 0; }

count=0
printf '%s\n' "$names" | while IFS= read -r name; do
  [ -n "$name" ] || continue
  [ "$name" != ".." ] || continue
  [ -e "$name" ] || continue
  $opener -- "$name" >/dev/null 2>&1 &
done
count=$(printf '%s\n' "$names" | grep -c . )
printf '{"status":"handed %s to the desktop with %s"}\n' "$count" "$opener"
