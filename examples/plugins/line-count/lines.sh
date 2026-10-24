#!/bin/sh
# Lines by extension under this directory.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

# What to call a file's kind. A function, and not a `case` written inside the command
# substitution below, because a `case` there does not parse in bash 3.2 — the bash macOS ships.
# See the note in examples/plugins/README.md.
extension_of() {
  one_name=${1##*/}
  case "$one_name" in
    *.*) printf '%s\n' "${one_name##*.}" ;;
    *) printf '(none)\n' ;;
  esac
}

# The directories that hold somebody else's code, and the ones a build wrote. Counting them
# is how a project of four thousand lines reports four hundred thousand.
skip='-name node_modules -o -name vendor -o -name target -o -name .git -o -name dist
      -o -name build -o -name .venv -o -name __pycache__ -o -name .next -o -name .gradle'

# shellcheck disable=SC2086
report=$(find . \( $skip \) -prune -o -type f -print 2>/dev/null |
  while IFS= read -r file; do
    # A file with a zero byte in its first block is not text, whatever it is called.
    head -c 1024 -- "$file" 2>/dev/null | tr -d '\0' | wc -c | {
      read -r readable
      head -c 1024 -- "$file" 2>/dev/null | wc -c | {
        read -r total
        [ "$readable" = "$total" ] || exit 0
        ext=$(extension_of "$file")
        lines=$(wc -l < "$file" 2>/dev/null | tr -d ' ')
        printf '%s\t%s\n' "$ext" "${lines:-0}"
      }
    }
  done |
  awk -F'\t' '{ lines[$1] += $2; files[$1]++ }
              END { for (e in lines) printf "%d\t%d\t%s\n", lines[e], files[e], e }' |
  sort -rn | head -25)

if [ -z "$report" ]; then
  echo '{"status":"no text files under here"}'
  exit 0
fi

total=$(printf '%s\n' "$report" | awk -F'\t' '{ n += $1 } END { print n+0 }')
printf '{"message":{"title":"Lines of code (%s total)","lines":[' "$total"
first=1
printf '%s\n' "$report" | while IFS="$(printf '\t')" read -r lines files ext; do
  [ "$first" = 1 ] || printf ','
  first=0
  printf '"%10s lines  %5s files  .%s"' "$lines" "$files" \
    "$(printf '%s' "$ext" | sed 's/\\/\\\\/g; s/"/\\"/g')"
done
printf ']}}\n'
