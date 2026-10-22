#!/usr/bin/env bash
# Starts the dev bridge (port 1431) and the studio frontend in Vite (port 1430)
# for browser-based agent testing. Ctrl-C / SIGTERM stops both.
#
# Variables (all optional):
#   G4_ROMS_DIR  folder holding the test projects (default: ../Roms); the bridge
#                only accepts paths under $G4_ROMS_DIR/_testkopien
#   G4_APICULA   apicula binary (default: tools/apicula/bin/apicula if present)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

# Use the pinned toolchain when it exists, otherwise whatever the shell has.
TOOLCHAIN_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
if [ -d "$TOOLCHAIN_BIN" ]; then
  export PATH="$TOOLCHAIN_BIN:$PATH"
fi

VITE_PORT=1430
BRIDGE_PORT=1431
G4_ROMS_DIR="${G4_ROMS_DIR:-$ROOT/../Roms}"

for port in "$VITE_PORT" "$BRIDGE_PORT"; do
  if lsof -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "Port $port belegt - bitte den laufenden Prozess beenden oder einen anderen Port wählen." >&2
    exit 1
  fi
done

mkdir -p target/agent
TOKEN="$(openssl rand -hex 16)"
(umask 077; printf '%s' "$TOKEN" > target/agent/bridge.token)
chmod 600 target/agent/bridge.token

if [ -z "${G4_APICULA:-}" ] && [ -x "$ROOT/tools/apicula/bin/apicula" ]; then
  export G4_APICULA="$ROOT/tools/apicula/bin/apicula"
fi

cargo build -p studio --features dev-bridge --bin studio-bridge

BRIDGE_PID=""
VITE_PID=""

# Kills a process and all its descendants (pnpm spawns node as a child).
kill_tree() { # kill_tree <pid>
  local child
  for child in $(pgrep -P "$1" 2>/dev/null || true); do
    kill_tree "$child"
  done
  kill "$1" 2>/dev/null || true
}
cleanup() {
  trap - EXIT INT TERM
  [ -z "$VITE_PID" ] || kill_tree "$VITE_PID"
  [ -z "$BRIDGE_PID" ] || kill_tree "$BRIDGE_PID"
  wait 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

G4_BRIDGE_TOKEN="$TOKEN" "$ROOT/target/debug/studio-bridge" serve \
  --port "$BRIDGE_PORT" \
  --origin "http://127.0.0.1:$VITE_PORT" \
  --allow-root "$G4_ROMS_DIR/_testkopien" &
BRIDGE_PID=$!

# Wait up to 30 s for the bridge to answer /health.
ready=0
for _ in $(seq 1 60); do
  if ! kill -0 "$BRIDGE_PID" 2>/dev/null; then
    echo "Brücke ist beim Start beendet worden." >&2
    exit 1
  fi
  if curl -sf -o /dev/null -H "X-G4-Bridge-Token: $TOKEN" "http://127.0.0.1:$BRIDGE_PORT/health"; then
    ready=1
    break
  fi
  sleep 0.5
done
if [ "$ready" != 1 ]; then
  echo "Brücke antwortet nicht auf /health (30 s)." >&2
  exit 1
fi

echo "Brücke läuft auf 127.0.0.1:$BRIDGE_PORT, Frontend startet auf http://127.0.0.1:$VITE_PORT"
# Vite runs in the background and we wait for it: a plain foreground child would
# delay the traps until it exits, and exec would drop them altogether.
cd apps/studio
G4_BRIDGE_PORT="$BRIDGE_PORT" G4_BRIDGE_TOKEN="$TOKEN" \
  pnpm exec vite --host 127.0.0.1 --port "$VITE_PORT" --strictPort &
VITE_PID=$!
wait "$VITE_PID"
