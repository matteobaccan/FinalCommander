#!/bin/sh
# Parse the selected files with whichever interpreter owns them, and report what it says.
cat >/dev/null
cd "$SCMD_DIR" 2>/dev/null || { echo '{"error":"not a directory on this machine"}'; exit 0; }

names="$SCMD_SELECTED"
[ -n "$names" ] || names="$SCMD_CURSOR"
[ -n "$names" ] || { echo '{"status":"nothing selected"}'; exit 0; }

# One function per language, so the list of what is understood is the list of these.
check_one() {
  one=$1
  case "$one" in
    *.php)
      command -v php >/dev/null 2>&1 || { echo "php is not installed"; return 2; }
      php -l -- "$one" 2>&1 | grep -v '^No syntax errors' ;;
    *.py)
      command -v python3 >/dev/null 2>&1 || { echo "python3 is not installed"; return 2; }
      python3 -m py_compile "$one" 2>&1 ;;
    *.js|*.cjs|*.mjs)
      command -v node >/dev/null 2>&1 || { echo "node is not installed"; return 2; }
      node --check "$one" 2>&1 | head -3 ;;
    *.json)
      if command -v python3 >/dev/null 2>&1; then
        python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$one" 2>&1 | tail -1
      else
        echo "python3 is not installed"; return 2
      fi ;;
    *.rb)
      command -v ruby >/dev/null 2>&1 || { echo "ruby is not installed"; return 2; }
      ruby -c -- "$one" 2>&1 | grep -v '^Syntax OK' ;;
    *.sh|*.bash)
      sh -n "$one" 2>&1 ;;
    *.pl|*.pm)
      command -v perl >/dev/null 2>&1 || { echo "perl is not installed"; return 2; }
      perl -c "$one" 2>&1 | grep -v 'syntax OK' ;;
    *)
      echo "not a kind this knows"; return 2 ;;
  esac
}

report=$(printf '%s\n' "$names" | while IFS= read -r name; do
  [ -f "$name" ] || continue
  out=$(check_one "$name" 2>&1)
  if [ -z "$out" ]; then
    printf '%s: ok\n' "$name"
  else
    printf '%s: %s\n' "$name" "$(printf '%s' "$out" | tr '\n' ' ' | cut -c1-120)"
  fi
done)

# `py_compile` leaves its work behind; a check that litters is a check nobody runs twice.
rm -rf __pycache__ 2>/dev/null

[ -n "$report" ] || { echo '{"status":"nothing to check among those"}'; exit 0; }

bad=$(printf '%s\n' "$report" | grep -cv ': ok$')
printf '{"message":{"title":"Syntax check: %s of %s have something to say","lines":[' \
  "$bad" "$(printf '%s\n' "$report" | wc -l | tr -d ' ')"
first=1
printf '%s\n' "$report" | while IFS= read -r line; do
  [ "$first" = 1 ] || printf ','
  first=0
  printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/  /g')"
done
printf ']}}\n'
