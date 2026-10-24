#!/bin/sh
# Every subdirectory of this one, measured and sorted by size, biggest first.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

# `du -s` on each entry rather than `du -d 1`, because the second prints the parent too and
# the first is the same number in the same number of passes.
report=$(for name in */ .*/; do
  [ -d "$name" ] || continue
  # `.` and `..` match the second glob and are not folders in this one. Written as two tests
  # rather than as a `case`, which must not appear inside a command substitution: see the note
  # in examples/plugins/README.md.
  [ "$name" = "./" ] && continue
  [ "$name" = "../" ] && continue
  size=$(du -sk -- "$name" 2>/dev/null | cut -f1)
  [ -n "$size" ] || continue
  files=$(find "$name" -type f 2>/dev/null | wc -l | tr -d ' ')
  printf '%s\t%s\t%s\n' "$size" "${name%/}" "$files"
done | sort -rn | head -40)

if [ -z "$report" ]; then
  echo '{"status":"no folders here to measure"}'
  exit 0
fi

# Kilobytes into something readable, done here so the plugin needs no `numfmt`, which macOS
# does not have.
human() {
  awk -v kb="$1" 'BEGIN {
    split("K M G T", unit, " ")
    i = 1
    while (kb >= 1024 && i < 4) { kb /= 1024; i++ }
    printf (kb < 10 && i > 1) ? "%.1f %s\n" : "%.0f %s\n", kb, unit[i]
  }'
}

printf '{"message":{"title":"Folder sizes","lines":['
first=1
total=0
printf '%s\n' "$report" | while IFS="$(printf '\t')" read -r size name files; do
  [ "$first" = 1 ] || printf ','
  first=0
  printf '"%8s  %s"' "$(human "$size")" \
    "$(printf '%s (%s files)' "$name" "$files" | sed 's/\\/\\\\/g; s/"/\\"/g')"
done
printf ']}}\n'
