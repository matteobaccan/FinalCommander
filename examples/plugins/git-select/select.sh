#!/bin/sh
# Select, in the panel, the files git says are modified — then F5 copies exactly those, or
# Shift+F1 packs them. The point of the plugin tier: the selection is the program's, the
# knowledge of what changed is git's.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }
if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo '{"status":"not a git repository"}'
  exit 0
fi

# Only the names in this directory: a path with a slash in it is in a subdirectory, and the
# panel has no row for it. `git status --porcelain` prints the path from the repository root,
# so the listing is taken relative to here.
names=$(git status --porcelain --untracked-files=all . 2>/dev/null |
        cut -c4- |
        sed 's|^"\(.*\)"$|\1|' |
        while IFS= read -r path; do
          # A name with a slash in it is in a subdirectory and the panel has no row for it.
          # Written as one test rather than as a `case`, which must not appear inside a
          # command substitution: see the note in examples/plugins/README.md.
          [ -n "$path" ] || continue
          [ "${path#*/}" = "$path" ] || continue
          printf '%s\n' "$path"
        done)

if [ -z "$names" ]; then
  echo '{"status":"nothing changed in this directory"}'
  exit 0
fi

# One JSON string per name, joined: a file name may contain anything but a slash, so the
# quoting matters more than the brevity.
printf '{"select":['
first=1
printf '%s\n' "$names" | while IFS= read -r name; do
  [ -n "$name" ] || continue
  [ "$first" = 1 ] || printf ','
  first=0
  printf '"%s"' "$(printf '%s' "$name" | sed 's/\\/\\\\/g; s/"/\\"/g')"
done
printf ']}\n'
printf '{"status":"selected what git reports as changed"}\n'
