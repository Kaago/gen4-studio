#!/usr/bin/env bash
# Calls one command on the running dev bridge.
#   scripts/agent/call.sh <command> ['<json args>']
# Prints the JSON response; for application/octet-stream only "<n> bytes".
# Exit 1 on any HTTP status other than 200.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TOKEN_FILE="$ROOT/target/agent/bridge.token"

if [ $# -lt 1 ]; then
  echo "Aufruf: scripts/agent/call.sh <befehl> ['<json>']" >&2
  exit 2
fi
if [ ! -f "$TOKEN_FILE" ]; then
  echo "Token-Datei fehlt ($TOKEN_FILE) - läuft scripts/agent/studio-web.sh?" >&2
  exit 1
fi

cmd="$1"
body="${2:-}"
[ -n "$body" ] || body='{}'

out="$(mktemp)"
trap 'rm -f "$out"' EXIT

meta="$(curl -sS -o "$out" -w '%{http_code} %{content_type}' -X POST \
  -H "Content-Type: application/json" \
  -H "X-G4-Bridge-Token: $(cat "$TOKEN_FILE")" \
  --data "$body" "http://127.0.0.1:1431/invoke/$cmd")" || {
  echo "Brücke nicht erreichbar (127.0.0.1:1431) - läuft scripts/agent/studio-web.sh?" >&2
  exit 1
}
status="${meta%% *}"
ctype="${meta#* }"

if [ "$status" != "200" ]; then
  echo "HTTP $status" >&2
  cat "$out" >&2
  echo >&2
  exit 1
fi
case "$ctype" in
  application/octet-stream*) echo "$(wc -c < "$out" | tr -d ' ') bytes" ;;
  *) cat "$out"; echo ;;
esac
