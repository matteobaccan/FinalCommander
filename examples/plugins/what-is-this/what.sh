#!/bin/sh
# `file` is everywhere and answers the first question. The rest join in when they are here:
# a picture's size, a video's length, a document's page count, an MP3's tags.
#
# F11 in the viewer already reads an executable properly — this is for everything else.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }
name="$FCMD_CURSOR"
[ -n "$name" ] || { echo '{"status":"nothing under the cursor"}'; exit 0; }
[ -e "$name" ] || { echo '{"error":"that is not here any more"}'; exit 0; }

# What this kind of file has to say, asked of whichever tool is installed. A function, and
# not a `case` written inside the command substitution below, because a `case` there does
# not parse in bash 3.2 — the bash macOS ships, where the `)` of the first pattern ends the
# substitution. See the note in examples/plugins/README.md.
by_kind() {
  case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    *.jpg|*.jpeg|*.png|*.gif|*.webp|*.tif|*.tiff|*.bmp)
      command -v identify >/dev/null 2>&1 &&
        identify -format '%wx%h, %[colorspace], %[bit-depth] bit\n' -- "$1" 2>/dev/null
      command -v exiftool >/dev/null 2>&1 &&
        exiftool -s3 -Model -DateTimeOriginal -LensModel -- "$1" 2>/dev/null ;;
    *.mp3|*.flac|*.ogg|*.m4a|*.opus|*.wav|*.mp4|*.mkv|*.webm|*.mov|*.avi)
      command -v ffprobe >/dev/null 2>&1 &&
        ffprobe -v error -show_entries format=duration,bit_rate:stream=codec_name,width,height \
                -of default=noprint_wrappers=1 -- "$1" 2>/dev/null | head -8 ;;
    *.pdf)
      command -v pdfinfo >/dev/null 2>&1 && pdfinfo -- "$1" 2>/dev/null | head -8 ;;
    *.json)
      command -v python3 >/dev/null 2>&1 &&
        python3 -c 'import json,sys;d=json.load(open(sys.argv[1]));print("valid JSON,", type(d).__name__, "with", len(d), "entries" if hasattr(d,"__len__") else "")' "$1" 2>/dev/null ||
        echo "not valid JSON" ;;
    *.txt|*.md|*.rs|*.py|*.js|*.php|*.java|*.sh|*.c|*.h|*.toml|*.yaml|*.yml)
      wc -lw < "$1" 2>/dev/null | awk '{printf "%s lines, %s words\n", $1, $2}' ;;
  esac
}

out=$(
  if [ -d "$name" ]; then
    files=$(find "$name" -type f 2>/dev/null | wc -l | tr -d ' ')
    dirs=$(find "$name" -mindepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')
    printf 'a directory: %s file(s) in %s folder(s)\n' "$files" "$dirs"
    du -sh -- "$name" 2>/dev/null | sed 's/\t/  /'
  else
    size=$(wc -c < "$name" 2>/dev/null | tr -d ' ')
    printf '%s bytes\n' "$size"
    command -v file >/dev/null 2>&1 && file -b -- "$name" 2>/dev/null

    # The specific tools, each only if it is installed and only for what it knows.
    by_kind "$name"

    # A checksum, because the question "is this the same file as that one" comes next.
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum -- "$name" 2>/dev/null | cut -c1-24 | sed 's/^/sha256 /;s/$/…/'
    elif command -v shasum >/dev/null 2>&1; then
      shasum -a 256 -- "$name" 2>/dev/null | cut -c1-24 | sed 's/^/sha256 /;s/$/…/'
    fi
  fi
)

[ -n "$out" ] || { echo '{"status":"nothing to say about that"}'; exit 0; }

printf '{"message":{"title":"%s","lines":[' "$(printf '%s' "$name" | sed 's/\\/\\\\/g; s/"/\\"/g')"
first=1
printf '%s\n' "$out" | while IFS= read -r line; do
  [ -n "$line" ] || continue
  [ "$first" = 1 ] || printf ','
  first=0
  printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/  /g')"
done
printf ']}}\n'
