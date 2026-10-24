#!/bin/sh
# Climb until something says "this is the top". The markers are in the order of how much they
# mean: a repository boundary beats a package file, because a workspace has package files all
# the way down.
cat >/dev/null
dir="$SCMD_DIR"
[ -n "$dir" ] || { echo '{"error":"not a directory on this machine"}'; exit 0; }
cd "$dir" 2>/dev/null || { echo '{"error":"cannot enter that directory"}'; exit 0; }

here=$(pwd)
found=""
what=""
candidate="$here"
while [ -n "$candidate" ] && [ "$candidate" != "/" ]; do
  for marker in .git .hg .svn .jj; do
    if [ -e "$candidate/$marker" ]; then
      found="$candidate"
      what="$marker"
      break
    fi
  done
  [ -z "$found" ] || break
  candidate=$(dirname "$candidate")
done

# No repository: the outermost package file, which is the top of a project that is not under
# version control.
if [ -z "$found" ]; then
  candidate="$here"
  while [ -n "$candidate" ] && [ "$candidate" != "/" ]; do
    for marker in Cargo.toml package.json pyproject.toml go.mod pom.xml composer.json Makefile; do
      if [ -f "$candidate/$marker" ]; then
        found="$candidate"
        what="$marker"
      fi
    done
    candidate=$(dirname "$candidate")
  done
fi

if [ -z "$found" ]; then
  echo '{"status":"this does not look like it is inside a project"}'
  exit 0
fi
if [ "$found" = "$here" ]; then
  printf '{"status":"already at the top (%s)"}\n' "$what"
  exit 0
fi

printf '{"chdir":{"path":"%s"}}\n' "$(printf '%s' "$found" | sed 's/\\/\\\\/g; s/"/\\"/g')"
printf '{"status":"the top of this %s project"}\n' "$what"
