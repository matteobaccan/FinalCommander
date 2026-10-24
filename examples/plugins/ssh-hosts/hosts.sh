#!/bin/sh
# Read ~/.ssh/config and list what each named host resolves to. Nothing is connected and
# nothing is written: this answers "what was that server called again".
cat >/dev/null
config=${HOME:-~}/.ssh/config
[ -f "$config" ] || { echo '{"status":"no ~/.ssh/config here"}'; exit 0; }

# Include files are followed one level, which is how most people split work from personal.
read_config() {
  awk -v dir="${HOME:-~}/.ssh" '
    tolower($1) == "include" {
      for (i = 2; i <= NF; i++) {
        path = $i
        if (path !~ /^[\/~]/) path = dir "/" path
        while ((getline line < path) > 0) print line
        close(path)
      }
      next
    }
    { print }
  ' "$1"
}

report=$(read_config "$config" |
  awk '
    function flush() {
      if (host != "" && host !~ /[*?]/) {
        printf "%s\t%s\t%s\t%s\n", host,
               (name == "" ? host : name),
               (user == "" ? "-" : user),
               (port == "" ? "22" : port)
      }
      host = ""; name = ""; user = ""; port = ""
    }
    tolower($1) == "host" { flush(); host = $2; next }
    tolower($1) == "hostname" { name = $2; next }
    tolower($1) == "user" { user = $2; next }
    tolower($1) == "port" { port = $2; next }
    END { flush() }
  ')

[ -n "$report" ] || { echo '{"status":"no named hosts in ~/.ssh/config"}'; exit 0; }

count=$(printf '%s\n' "$report" | wc -l | tr -d ' ')
printf '{"message":{"title":"SSH hosts (%s)","lines":[' "$count"
first=1
printf '%s\n' "$report" | while IFS="$(printf '\t')" read -r host name user port; do
  [ "$first" = 1 ] || printf ','
  first=0
  line=$(printf '%-20s %s@%s:%s' "$host" "$user" "$name" "$port")
  printf '"%s"' "$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g')"
done
printf ']}}\n'
