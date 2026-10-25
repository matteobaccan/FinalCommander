#!/bin/sh
# The ten biggest things in this directory, subdirectories measured. `F3` on a directory
# measures one; this ranks them, which is the question one actually has.
#
# The timeout in the manifest is a minute because `du` on a deep tree is not instant — and a
# plugin that takes a minute says so there rather than being killed at the default twenty
# seconds.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

lines=$(du -sh -- * .[!.]* 2>/dev/null | sort -rh | head -10 | sed 's/\t/  /')
if [ -z "$lines" ]; then
  echo '{"status":"nothing here to measure"}'
  exit 0
fi

# A message is a title and lines, so the JSON is built one string at a time.
printf '{"message":{"title":"Biggest here","lines":['
first=1
printf '%s\n' "$lines" | while IFS= read -r line; do
  [ "$first" = 1 ] || printf ','
  first=0
  printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g')"
done
printf ']}}\n'
