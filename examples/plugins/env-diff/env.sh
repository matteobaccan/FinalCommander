#!/bin/sh
# Compare the keys of `.env` with those of `.env.example` — names only, never the values:
# a secret that reaches a dialog is a secret in a screenshot.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

example=""
for candidate in .env.example .env.sample .env.dist env.example; do
  [ -f "$candidate" ] && { example=$candidate; break; }
done
[ -n "$example" ] || { echo '{"status":"no .env.example here"}'; exit 0; }
[ -f .env ] || { echo '{"error":"there is an example but no .env beside it"}'; exit 0; }

keys_of() {
  sed -e 's/[[:space:]]*#.*$//' -e 's/^[[:space:]]*export[[:space:]]*//' "$1" 2>/dev/null |
    grep -E '^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=' |
    sed -e 's/[[:space:]]*=.*$//' -e 's/^[[:space:]]*//' | sort -u
}

# `comm` wants two files, so two there are, in whatever this system calls its temporary
# directory, and both are gone before the first line is printed.
tmp=${TMPDIR:-/tmp}
wanted=$tmp/fcmd-env-wanted.$$
present=$tmp/fcmd-env-present.$$
keys_of "$example" > "$wanted"
keys_of .env > "$present"
missing=$(comm -23 "$wanted" "$present")
extra=$(comm -13 "$wanted" "$present")
rm -f "$wanted" "$present"

say() {
  printf '%s\n' "$1" | while IFS= read -r key; do
    [ -n "$key" ] || continue
    printf ',"  %s"' "$(printf '%s' "$key" | sed 's/\\/\\\\/g; s/"/\\"/g')"
  done
}

if [ -z "$missing" ] && [ -z "$extra" ]; then
  printf '{"status":".env has exactly the keys of %s"}\n' "$example"
  exit 0
fi

printf '{"message":{"title":".env against %s","lines":[' "$example"
if [ -n "$missing" ]; then
  printf '"missing from .env:"'
  say "$missing"
fi
if [ -n "$extra" ]; then
  [ -n "$missing" ] && printf ',""'
  [ -n "$missing" ] || printf '"in .env and not in the example:"'
  [ -n "$missing" ] && printf ',"in .env and not in the example:"'
  say "$extra"
fi
printf ']}}\n'
