#!/bin/sh
# Two ways one wants it: now, or the same as that one. The second is the useful one — making a
# directory of files share a timestamp is otherwise a shell line nobody remembers.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

names="$SCMD_SELECTED"
[ -n "$names" ] || names="$SCMD_CURSOR"
[ -n "$names" ] || { echo '{"status":"nothing selected"}'; exit 0; }

case "$SCMD_COMMAND" in
  *.match)
    [ -n "$SCMD_CURSOR" ] || { echo '{"status":"nothing under the cursor to match"}'; exit 0; }
    [ -e "$SCMD_CURSOR" ] || { echo '{"error":"the file under the cursor is not here"}'; exit 0; }
    printf '%s\n' "$names" | while IFS= read -r name; do
      [ -n "$name" ] || continue
      [ "$name" != ".." ] || continue
      [ "$name" != "$SCMD_CURSOR" ] || continue
      touch -r "$SCMD_CURSOR" -- "$name" 2>/dev/null
    done
    printf '{"run":"panel.reread"}\n'
    printf '{"status":"timed to match %s"}\n' "$SCMD_CURSOR"
    ;;
  *)
    printf '%s\n' "$names" | while IFS= read -r name; do
      [ -n "$name" ] || continue
      [ "$name" != ".." ] || continue
      touch -- "$name" 2>/dev/null
    done
    count=$(printf '%s\n' "$names" | grep -c .)
    printf '{"run":"panel.reread"}\n'
    printf '{"status":"%s file(s) timed now"}\n' "$count"
    ;;
esac
