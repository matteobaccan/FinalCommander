#!/bin/sh
# A scan, a download, a day's notes: the folder one makes twenty times a year and names the
# same way every time. The plugin makes it and takes the panel into it.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }
name=$(date +%Y-%m-%d)
if [ -d "$name" ]; then
  # Already there: go in rather than complain. Twice in one day is the ordinary case.
  printf '{"chdir":{"path":"%s"}}\n' "$name"
  printf '{"status":"%s was already here"}\n' "$name"
  exit 0
fi
if ! mkdir "$name" 2>/dev/null; then
  printf '{"error":"cannot make %s here"}\n' "$name"
  exit 0
fi
# The panel does not know about a directory made behind its back: re-read it, then enter.
printf '{"run":"panel.reread"}\n'
printf '{"chdir":{"path":"%s"}}\n' "$name"
printf '{"status":"made %s"}\n' "$name"
