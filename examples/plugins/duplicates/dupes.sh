#!/bin/sh
# Same size first, then the same hash: comparing every pair by content would read the whole
# directory, and two files of different lengths are never the same file.
#
# It reports rather than deletes. Deciding which copy to keep is not a decision to hand to a
# script, and the panel is already good at deleting what one has selected.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

if command -v sha256sum >/dev/null 2>&1; then
  hash() { sha256sum -- "$1" 2>/dev/null | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
  hash() { shasum -a 256 -- "$1" 2>/dev/null | cut -d' ' -f1; }
else
  echo '{"error":"no sha256sum or shasum here to compare with"}'
  exit 0
fi

# What to look at: the selection, or everything in this directory.
if [ -n "$SCMD_SELECTED" ]; then
  list=$(printf '%s\n' "$SCMD_SELECTED")
else
  list=$(ls -A 2>/dev/null)
fi

work=$(mktemp) || { echo '{"error":"cannot make a scratch file"}'; exit 0; }
trap 'rm -f "$work"' EXIT

# size<tab>name, files only.
printf '%s\n' "$list" | while IFS= read -r name; do
  [ -n "$name" ] || continue
  [ -f "$name" ] || continue
  size=$(wc -c < "$name" 2>/dev/null | tr -d ' ')
  [ -n "$size" ] || continue
  printf '%s\t%s\n' "$size" "$name"
done | sort -n > "$work"

# Hash only within a group of equal sizes, and only when the group has more than one member.
report=$(awk -F'\t' '{ count[$1]++; names[$1] = names[$1] "\t" $2 } END {
           for (s in count) if (count[s] > 1) print s names[s] }' "$work" |
  while IFS= read -r line; do
    size=$(printf '%s' "$line" | cut -f1)
    printf '%s' "$line" | cut -f2- | tr '\t' '\n' | while IFS= read -r name; do
      [ -n "$name" ] || continue
      printf '%s\t%s\t%s\n' "$(hash "$name")" "$size" "$name"
    done
  done | sort)

out=$(printf '%s\n' "$report" | awk -F'\t' '
  { if ($1 == "") next; count[$1]++; names[$1] = names[$1] (count[$1] > 1 ? ", " : "") $3; bytes[$1] = $2 }
  END { for (h in count) if (count[h] > 1) printf "%s copies, %s bytes each: %s\n", count[h], bytes[h], names[h] }' | sort -t' ' -k1,1nr)

if [ -z "$out" ]; then
  echo '{"status":"no two of these are the same file"}'
  exit 0
fi

printf '{"message":{"title":"The same file more than once","lines":['
first=1
printf '%s\n' "$out" | while IFS= read -r line; do
  [ -n "$line" ] || continue
  [ "$first" = 1 ] || printf ','
  first=0
  printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g')"
done
printf ']}}\n'
