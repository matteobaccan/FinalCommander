#!/bin/sh
# Unpack or test the archive under the cursor with a program this machine happens to have.
# Shift+F2 already handles zip and the tar family; this is for the rest.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }
file="$SCMD_CURSOR"
[ -n "$file" ] || { echo '{"status":"nothing under the cursor"}'; exit 0; }
[ -f "$file" ] || { echo '{"status":"that is not a file"}'; exit 0; }

# The first tool that is here and knows the format. 7z knows nearly everything; unar is the
# macOS answer; the per-format tools are the fallback.
# `tar` is last on purpose: the program reads the tar family itself, so this only reaches it
# on a machine with nothing else — and even then it is better than refusing.
tool=""
for candidate in 7zz 7z 7za unar bsdtar tar; do
  if command -v "$candidate" >/dev/null 2>&1; then
    tool="$candidate"
    break
  fi
done

case "$file" in
  *.rar|*.RAR) command -v unrar >/dev/null 2>&1 && tool=${tool:-unrar} ;;
esac

if [ -z "$tool" ]; then
  echo '{"error":"none of 7z, unar, bsdtar or tar is installed — one of them is what this needs"}'
  exit 0
fi

testing=false
case "$SCMD_COMMAND" in *.test-any) testing=true ;; esac

if [ "$testing" = true ]; then
  case "$tool" in
    7zz|7z|7za) out=$("$tool" t -- "$file" 2>&1 | tail -5) ;;
    unar)       out=$(lsar -t -- "$file" 2>&1 | tail -5) ;;
    bsdtar|tar) out=$("$tool" -tf "$file" >/dev/null 2>&1 && echo "OK" || echo "$tool could not read it") ;;
    unrar)      out=$(unrar t -- "$file" 2>&1 | tail -5) ;;
  esac
  case "$out" in
    *Everything\ is\ Ok*|*OK*|*No\ errors*) printf '{"status":"%s is sound (%s)"}\n' "$file" "$tool" ;;
    *) printf '{"message":{"title":"What %s said about %s","lines":["%s"]}}\n' "$tool" "$file" \
         "$(printf '%s' "$out" | tr '\n' ' ' | sed 's/\\/\\\\/g; s/"/\\"/g')" ;;
  esac
  exit 0
fi

# Into a folder named after the archive, which is what one means and never leaves a hundred
# files loose in the current directory.
into=$(printf '%s' "$file" | sed 's/\.[A-Za-z0-9]*$//')
[ -n "$into" ] && [ "$into" != "$file" ] || into="$file.unpacked"
mkdir -p -- "$into" 2>/dev/null || { printf '{"error":"cannot make %s"}\n' "$into"; exit 0; }

case "$tool" in
  7zz|7z|7za) "$tool" x -y -o"$into" -- "$file" >/dev/null 2>&1 ;;
  unar)       unar -quiet -output-directory "$into" -- "$file" >/dev/null 2>&1 ;;
  bsdtar|tar) "$tool" -xf "$file" -C "$into" >/dev/null 2>&1 ;;
  unrar)      unrar x -y -- "$file" "$into/" >/dev/null 2>&1 ;;
esac
status=$?

if [ $status -ne 0 ]; then
  # Leave the folder: what did come out is often what one wanted, and deleting it behind the
  # user's back would be the worse mistake.
  printf '{"run":"panel.reread"}\n'
  printf '{"error":"%s could not finish %s — what came out is in %s"}\n' "$tool" "$file" "$into"
  exit 0
fi

count=$(find "$into" -type f 2>/dev/null | wc -l | tr -d ' ')
printf '{"run":"panel.reread"}\n'
printf '{"chdir":{"path":"%s"}}\n' "$into"
printf '{"status":"%s file(s) out of %s, with %s"}\n' "$count" "$file" "$tool"
