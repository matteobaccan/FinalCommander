#!/bin/sh
# Create a sums file for what is selected, or check the files against the sums file under the
# cursor. The format is the one `sha256sum` and `shasum` both read, which is the one every
# project publishes beside its downloads — including this one.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

if command -v sha256sum >/dev/null 2>&1; then
  make_sums() { sha256sum -- "$@"; }
  check_sums() { sha256sum -c --quiet -- "$1" 2>&1; }
elif command -v shasum >/dev/null 2>&1; then
  make_sums() { shasum -a 256 -- "$@"; }
  check_sums() { shasum -a 256 -c --quiet -- "$1" 2>&1; }
else
  echo '{"error":"no sha256sum or shasum here"}'
  exit 0
fi

case "$SCMD_COMMAND" in
  *.verify)
    file="$SCMD_CURSOR"
    case "$file" in
      *SHA256SUMS*|*.sha256|*.sha256sum|*.sum) ;;
      *)
        printf '{"error":"%s does not look like a sums file — put the cursor on one"}\n' "$file"
        exit 0 ;;
    esac
    out=$(check_sums "$file")
    if [ -z "$out" ]; then
      printf '{"status":"every file in %s is as it was"}\n' "$file"
      exit 0
    fi
    # Something failed: the names are what matters, so they go in a message and the files
    # that are wrong are selected in the panel for whatever comes next.
    printf '{"message":{"title":"These do not match","lines":['
    first=1
    printf '%s\n' "$out" | head -40 | while IFS= read -r line; do
      [ -n "$line" ] || continue
      [ "$first" = 1 ] || printf ','
      first=0
      printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    done
    printf ']}}\n'
    ;;
  *)
    names="$SCMD_SELECTED"
    [ -n "$names" ] || names="$SCMD_CURSOR"
    [ -n "$names" ] || { echo '{"status":"nothing selected to sum"}'; exit 0; }
    # One `sha256sum` call with every name, so the file comes out in one piece.
    count=0
    : > SHA256SUMS.part || { echo '{"error":"cannot write here"}'; exit 0; }
    printf '%s\n' "$names" | while IFS= read -r name; do
      [ -n "$name" ] || continue
      [ -f "$name" ] || continue
      make_sums "$name" >> SHA256SUMS.part 2>/dev/null
    done
    count=$(wc -l < SHA256SUMS.part | tr -d ' ')
    if [ "$count" = "0" ]; then
      rm -f SHA256SUMS.part
      echo '{"status":"no files to sum (directories are skipped)"}'
      exit 0
    fi
    # Written beside the originals and renamed over the top, the way the program saves a file:
    # an interrupted run leaves the old sums file alone.
    mv SHA256SUMS.part SHA256SUMS
    printf '{"run":"panel.reread"}\n'
    printf '{"status":"SHA256SUMS written for %s file(s)"}\n' "$count"
    ;;
esac
