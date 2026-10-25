#!/bin/sh
# Report, or remove, the UTF-8 byte order mark at the front of the selected files.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

names="$FCMD_SELECTED"
[ -n "$names" ] || names="$FCMD_CURSOR"
[ -n "$names" ] || { echo '{"status":"nothing selected"}'; exit 0; }

# `od` rather than `head -c3 | cmp`, because the three bytes have to be compared as bytes and
# a shell variable cannot hold every byte there is.
has_bom() {
  [ -f "$1" ] || return 1
  first=$(od -An -N3 -t x1 -- "$1" 2>/dev/null | tr -d ' \n')
  [ "$first" = "efbbbf" ]
}

found=$(printf '%s\n' "$names" | while IFS= read -r name; do
  has_bom "$name" && printf '%s\n' "$name"
done)

if [ -z "$found" ]; then
  echo '{"status":"none of those begin with a BOM"}'
  exit 0
fi

count=$(printf '%s\n' "$found" | wc -l | tr -d ' ')

case "$FCMD_COMMAND" in
  *.find)
    printf '{"message":{"title":"Files beginning with a BOM (%s)","lines":[' "$count"
    first=1
    printf '%s\n' "$found" | while IFS= read -r name; do
      [ "$first" = 1 ] || printf ','
      first=0
      printf '"%s"' "$(printf '%s' "$name" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    done
    printf ']}}\n'
    ;;
  *)
    # Written beside the file and moved into place, so a file is never half a file: the old
    # bytes stay until the new ones are all there, and the move keeps the directory entry.
    #
    # The copy comes first and the content second, because `cp -p` is the portable way to
    # carry the mode and the times over — `chmod --reference` is GNU's and macOS has not got
    # it — and writing into a file that already exists does not change its permissions. A
    # script that loses its executable bit has stopped being a script.
    printf '%s\n' "$found" | while IFS= read -r name; do
      cp -p -- "$name" "$name.fcmd-bom" 2>/dev/null || continue
      if tail -c +4 -- "$name" > "$name.fcmd-bom" 2>/dev/null; then
        mv -- "$name.fcmd-bom" "$name" 2>/dev/null || rm -f "$name.fcmd-bom"
      else
        rm -f "$name.fcmd-bom"
      fi
    done

    # Counted again rather than tracked: each file is looked at a second time, so what is
    # reported is what is on the disk and not what the loop believed it did.
    left=$(printf '%s\n' "$found" | while IFS= read -r name; do
      has_bom "$name" && printf 'x\n'
    done | wc -l | tr -d ' ')
    done_count=$((count - left))
    printf '{"run":"panel.reread"}\n'
    printf '{"status":"the BOM is off %s file(s)"}\n' "$done_count"
    [ "$left" = 0 ] || printf '{"error":"%s could not be rewritten"}\n' "$left"
    ;;
esac
