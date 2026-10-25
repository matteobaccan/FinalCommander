#!/bin/sh
# Two questions one asks a process list from a file manager: what is running, and what is
# holding this file open. FAR's ProcList answered the first; the second is why one opens it.
cat >/dev/null

json_lines() {
  # Turn standard input into the lines of a `message` action, escaped.
  title=$1
  printf '{"message":{"title":"%s","lines":[' "$title"
  first=1
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    [ "$first" = 1 ] || printf ','
    first=0
    printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/  /g')"
  done
  printf ']}}\n'
}

case "$FCMD_COMMAND" in
  *.who-has-this)
    [ -n "$FCMD_CURSOR" ] || { echo '{"status":"nothing under the cursor"}'; exit 0; }
    cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }
    if command -v lsof >/dev/null 2>&1; then
      out=$(lsof -- "$FCMD_CURSOR" 2>/dev/null | head -30)
    elif command -v fuser >/dev/null 2>&1; then
      out=$(fuser -v -- "$FCMD_CURSOR" 2>&1 | head -30)
    else
      echo '{"error":"neither lsof nor fuser is installed"}'
      exit 0
    fi
    if [ -z "$out" ]; then
      printf '{"status":"nothing has %s open"}\n' "$FCMD_CURSOR"
      exit 0
    fi
    printf '%s\n' "$out" | json_lines "What has $FCMD_CURSOR open"
    ;;
  *)
    # The twenty heaviest, which is the useful end of a process list.
    out=$(ps -eo pid,pcpu,pmem,comm --sort=-pcpu 2>/dev/null | head -21) ||
      out=$(ps ax 2>/dev/null | head -21)
    [ -n "$out" ] || { echo '{"error":"cannot read the process list"}'; exit 0; }
    printf '%s\n' "$out" | json_lines "What is running (by processor use)"
    ;;
esac
