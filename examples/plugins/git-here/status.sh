#!/bin/sh
# The context arrives as JSON on standard input; the same facts are in the environment, which
# is what a shell script actually wants.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }
if ! branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null); then
  echo '{"status":"not a git repository"}'
  exit 0
fi
count=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
printf '{"status":"on %s, %s file(s) changed"}\n' "$branch" "$count"
