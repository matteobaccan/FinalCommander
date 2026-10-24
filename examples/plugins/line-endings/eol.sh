#!/bin/sh
# Convert, or just look. The looking is the command one reaches for first: "why does this
# script not run" is answered by it.
#
# A binary file is left alone. Stripping carriage returns out of a PNG is the kind of help
# nobody wants, so each file is checked for a zero byte first.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

names="$SCMD_SELECTED"
[ -n "$names" ] || names="$SCMD_CURSOR"
[ -n "$names" ] || { echo '{"status":"nothing selected"}'; exit 0; }

looks_binary() {
  # A zero byte in the first few kilobytes: the same test the viewer uses to decide that text
  # would be a lie.
  head -c 4096 -- "$1" 2>/dev/null | tr -d '\0' | wc -c | {
    read -r kept
    size=$(head -c 4096 -- "$1" 2>/dev/null | wc -c)
    [ "$kept" != "$size" ]
  }
}

case "$SCMD_COMMAND" in
  *.what)
    report=$(printf '%s\n' "$names" | while IFS= read -r name; do
      [ -n "$name" ] || continue
      [ -f "$name" ] || continue
      if looks_binary "$name"; then
        printf '%s: binary, left alone\n' "$name"
        continue
      fi
      crlf=$(tr -dc '\r' < "$name" 2>/dev/null | wc -c | tr -d ' ')
      lf=$(tr -dc '\n' < "$name" 2>/dev/null | wc -c | tr -d ' ')
      case "$crlf:$lf" in
        0:0) printf '%s: no line endings at all\n' "$name" ;;
        0:*) printf '%s: LF, %s lines\n' "$name" "$lf" ;;
        *)   if [ "$crlf" = "$lf" ]; then
               printf '%s: CRLF, %s lines\n' "$name" "$lf"
             else
               printf '%s: mixed — %s CR and %s LF\n' "$name" "$crlf" "$lf"
             fi ;;
      esac
    done)
    [ -n "$report" ] || { echo '{"status":"no text files among those"}'; exit 0; }
    printf '{"message":{"title":"Line endings","lines":['
    first=1
    printf '%s\n' "$report" | while IFS= read -r line; do
      [ "$first" = 1 ] || printf ','
      first=0
      printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    done
    printf ']}}\n'
    exit 0 ;;
  *.lf)   to=lf ;;
  *.crlf) to=crlf ;;
  *) echo '{"error":"this plugin does not know that command"}'; exit 0 ;;
esac

changed=0
printf '%s\n' "$names" | while IFS= read -r name; do
  [ -n "$name" ] || continue
  [ -f "$name" ] || continue
  looks_binary "$name" && continue
  # Written beside and renamed over, like every save in this program: an interrupted
  # conversion must not leave half a file.
  part="$name.eol-part"
  if [ "$to" = lf ]; then
    tr -d '\r' < "$name" > "$part" 2>/dev/null
  else
    # Strip first, then add: a file that is already CRLF must not become CRCRLF.
    tr -d '\r' < "$name" 2>/dev/null | sed 's/$/\r/' > "$part"
  fi
  if [ -s "$part" ] || [ ! -s "$name" ]; then
    mv -- "$part" "$name"
  else
    rm -f -- "$part"
  fi
done

printf '{"run":"panel.reread"}\n'
printf '{"status":"line endings are now %s"}\n' "$(printf '%s' "$to" | tr '[:lower:]' '[:upper:]')"
