#!/bin/sh
# The system clipboard, through whichever of five programs is installed. Wayland, X11, macOS
# and WSL each have their own, which is exactly the sort of thing a plugin is for: the program
# itself will grow a clipboard with a platform crate behind it, and until then this works.
cat >/dev/null

if command -v wl-copy >/dev/null 2>&1; then
  clip() { wl-copy; }
  with=wl-copy
elif command -v pbcopy >/dev/null 2>&1; then
  clip() { pbcopy; }
  with=pbcopy
elif command -v xclip >/dev/null 2>&1; then
  clip() { xclip -selection clipboard; }
  with=xclip
elif command -v xsel >/dev/null 2>&1; then
  clip() { xsel --clipboard --input; }
  with=xsel
elif command -v clip.exe >/dev/null 2>&1; then
  clip() { clip.exe; }
  with=clip.exe
else
  echo '{"error":"no clipboard program here — one of wl-copy, pbcopy, xclip, xsel"}'
  exit 0
fi

names="$SCMD_SELECTED"
[ -n "$names" ] || names="$SCMD_CURSOR"
[ -n "$names" ] || { echo '{"status":"nothing under the cursor"}'; exit 0; }

full=false
case "$SCMD_COMMAND" in *.path) full=true ;; esac

text=$(printf '%s\n' "$names" | while IFS= read -r name; do
  [ -n "$name" ] || continue
  if [ "$full" = true ] && [ -n "$SCMD_DIR" ]; then
    printf '%s/%s\n' "${SCMD_DIR%/}" "$name"
  else
    printf '%s\n' "$name"
  fi
done)

printf '%s\n' "$text" | clip
count=$(printf '%s\n' "$text" | grep -c .)
what=names
[ "$full" = false ] || what=paths
printf '{"status":"%s %s copied with %s"}\n' "$count" "$what" "$with"
