#!/bin/sh
# Rename what is selected so the names are all one case. FAR's FileCase.
#
# The plugin does the renaming itself — it is a process, with the panel's directory as its
# working directory — and then asks the panel to read again. The actions are for driving the
# program, not for doing the work.
cat >/dev/null
cd "$FCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

# Nothing selected means the row under the cursor, which is the rule everywhere else.
names="$FCMD_SELECTED"
[ -n "$names" ] || names="$FCMD_CURSOR"
[ -n "$names" ] || { echo '{"status":"nothing to rename"}'; exit 0; }

case "$FCMD_COMMAND" in
  *.lower) how=lower ;;
  *.upper) how=upper ;;
  *.title) how=title ;;
  *) echo '{"error":"this plugin does not know that command"}'; exit 0 ;;
esac

done_count=0
skipped=0
printf '%s\n' "$names" | while IFS= read -r name; do
  [ -n "$name" ] || continue
  [ "$name" != ".." ] || continue
  case "$how" in
    lower) new=$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]') ;;
    upper) new=$(printf '%s' "$name" | tr '[:lower:]' '[:upper:]') ;;
    title)
      # Each word's first letter up, the rest down: `my HOLIDAY photo.JPG` becomes
      # `My Holiday Photo.jpg`. The extension is left lower, which is what one wants.
      new=$(printf '%s' "$name" | awk '{
        n = split($0, parts, /[ _]/)
        out = ""
        for (i = 1; i <= n; i++) {
          w = tolower(parts[i])
          if (length(w) > 0) w = toupper(substr(w, 1, 1)) substr(w, 2)
          out = out (i > 1 ? substr($0, 0, 0) : "") w
          if (i < n) out = out " "
        }
        print out
      }') ;;
  esac
  [ "$new" != "$name" ] || continue
  # A rename that would land on a name already there is skipped rather than overwriting it:
  # on a case-insensitive filesystem `A.txt` and `a.txt` are the same file, and `mv` would
  # happily throw one away.
  if [ -e "$new" ] && [ "$(printf '%s' "$new" | tr '[:upper:]' '[:lower:]')" != "$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]')" ]; then
    continue
  fi
  mv -- "$name" "$new" 2>/dev/null
done

printf '{"run":"panel.reread"}\n'
printf '{"status":"renamed to %s case"}\n' "$how"
