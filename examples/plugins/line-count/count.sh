#!/bin/sh
# The `lines` column: how many lines each file has, for a whole directory in one run.
#
# A batch (`batch = true` in plugin.toml): the paths arrive on standard input, one per line,
# and each answer is the path as it came, a tab, and the value. A file this says nothing
# about — a binary, one that has gone — simply has an empty cell.
tab=$(printf '\t')
while IFS= read -r file; do
  [ -f "$file" ] || continue
  # A zero byte in the first block: not text, whatever it is called.
  readable=$(head -c 1024 -- "$file" 2>/dev/null | tr -d '\0' | wc -c | tr -d ' ')
  total=$(head -c 1024 -- "$file" 2>/dev/null | wc -c | tr -d ' ')
  [ "$readable" = "$total" ] || continue
  lines=$(wc -l < "$file" 2>/dev/null | tr -d ' ')
  printf '%s%s%s\n' "$file" "$tab" "${lines:-0}"
done
