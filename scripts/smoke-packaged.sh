#!/usr/bin/env bash
# Smoke test of the packaged app, without a ROM (spec release-ci 8.1).
#
#   bash scripts/smoke-packaged.sh [--doctor-only] <studio-binary> <project-dir>
#
# <studio-binary> is the packaged one (macOS: Gen4 Studio.app/Contents/MacOS/studio, Windows: the
# installed studio.exe); <project-dir> a synthetic project (see crates/g4-kernel/examples/
# make_synthetic.rs). The script runs in a clean environment (`env -i`: HOME and TMPDIR in a fresh
# folder outside the repo, a minimal PATH, no G4_* variable), so nothing of a developer shell can
# make a broken package pass. Steps:
#   1. `studio doctor` must exit 0 (sidecars start, nitroarc loads);
#   2. `studio mcp --project <copy of project-dir> --expert` over stdio, spoken by python3
#      (standard library only): initialize, tools/list, one collision edit (g4_call
#      set_permission), a check that get_map shows it, and save. No build_rom: the synthetic
#      project is not buildable (its ds-rom config is too small, see
#      apps/studio/src-tauri/tests/mcp_stdio.rs), so a build would only prove an error.
# The smoke also requires "nitroarc": true in `studio doctor --json`.
# --doctor-only stops after step 1 (for scripts/tests/smoke-packaged.test.sh, which has no real
# studio). Ends with `smoke ok` and status 0. Works with bash 3.2 (macOS) and Git Bash on Windows.
set -euo pipefail

die() {
  echo "smoke: $*" >&2
  exit 1
}

# resolve_python: prints the absolute path of a working python (python3, python, "py -3"), or
# nothing. `sys.executable` resolves the Windows launcher `py` and rejects the Store stub. A
# native Windows python prints CRLF, so the \r goes before the path is used.
resolve_python() {
  local candidate found
  for candidate in "python3" "python" "py -3"; do
    # shellcheck disable=SC2086 # "py -3" is a command with an argument
    if found="$($candidate -I -c 'import sys; print(sys.executable)' 2>/dev/null)"; then
      found="${found%$'\r'}"
      if [ -n "$found" ]; then
        printf '%s\n' "$found"
        return 0
      fi
    fi
  done
  return 0
}

# Sourced (by the script tests): only the functions are wanted.
if [ "${BASH_SOURCE[0]}" != "$0" ]; then
  return 0
fi

# The packaged tools run from this script's environment, not from the caller's: the outer part
# prepares paths and re-runs the script as `--inner` under `env -i`.
inner=0
doctor_only=0
while [ $# -gt 0 ]; do
  case "$1" in
    --inner) inner=1 ;;
    --doctor-only) doctor_only=1 ;;
    *) break ;;
  esac
  shift
done
[ $# -ge 2 ] || die "usage: smoke-packaged.sh [--doctor-only] <studio-binary> <project-dir>"
STUDIO="$1"
PROJECT="$2"

if [ "$inner" -eq 0 ]; then
  [ -f "$STUDIO" ] || die "$STUDIO: no such file"
  [ -d "$PROJECT" ] || die "$PROJECT: no such folder"
  SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
  STUDIO="$(cd "$(dirname "$STUDIO")" && pwd)/$(basename "$STUDIO")"
  PROJECT="$(cd "$PROJECT" && pwd)"

  # python3 is looked up here, before the minimal PATH takes over, and handed on as an absolute
  # path.
  PYTHON=""
  if [ "$doctor_only" -eq 0 ]; then
    PYTHON="$(resolve_python)"
    [ -n "$PYTHON" ] || die "python3 fehlt"
  fi

  tmp="$(mktemp -d "${TMPDIR:-/tmp}/g4-smoke.XXXXXX")"
  trap 'rm -rf "$tmp"' EXIT
  mkdir "$tmp/home" "$tmp/tmp"

  case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*)
      # Git Bash needs /usr/bin for its own tools. Windows programs need SYSTEMROOT and WINDIR
      # (without them many fail to start) and look for their folders in TEMP/TMP/USERPROFILE.
      clean_env=(
        "HOME=$tmp/home" "TMPDIR=$tmp/tmp" "PATH=/c/Windows/System32:/usr/bin"
        "SYSTEMROOT=${SYSTEMROOT:-C:\\Windows}" "WINDIR=${WINDIR:-C:\\Windows}"
        "TEMP=$(cygpath -w "$tmp/tmp")" "TMP=$(cygpath -w "$tmp/tmp")"
        "USERPROFILE=$(cygpath -w "$tmp/home")"
      )
      ;;
    *)
      clean_env=("HOME=$tmp/home" "TMPDIR=$tmp/tmp" "PATH=/usr/bin:/bin")
      ;;
  esac

  inner_args=(--inner)
  [ "$doctor_only" -eq 1 ] && inner_args+=(--doctor-only)
  status=0
  (
    cd "$tmp"
    env -i "${clean_env[@]}" "$BASH" "$SELF" "${inner_args[@]}" "$STUDIO" "$PROJECT" "$PYTHON"
  ) || status=$?
  exit "$status"
fi

# --- inner part: runs under env -i, in the fresh folder ---
PYTHON="${3:-}"
WORK="$(pwd)"

echo "== doctor"
"$STUDIO" doctor
echo "smoke: doctor ok"
# doctor's exit status does not depend on nitroarc (spec); the smoke test does.
"$STUDIO" doctor --json | grep -q '"nitroarc": true' || die "doctor: nitroarc geladen? nein"
echo "smoke: nitroarc ok"
if [ "$doctor_only" -eq 1 ]; then
  echo "smoke: doctor only"
  exit 0
fi

# The edit goes to a copy, so the folder that was handed in stays as it was.
cp -R "$PROJECT" "$WORK/project"

echo "== mcp"
"$PYTHON" -I - "$STUDIO" "$WORK/project" <<'PY'
import hashlib
import json
import os
import queue
import subprocess
import sys
import threading

studio, project = sys.argv[1], sys.argv[2]
LIMIT = 120  # seconds to wait for one answer


def snapshot():
    """Path -> SHA-256 of every file of the project, except the .g4 bookkeeping folder."""
    files = {}
    for folder, dirs, names in os.walk(project):
        dirs[:] = [d for d in dirs if d != ".g4"]
        for name in names:
            path = os.path.join(folder, name)
            with open(path, "rb") as f:
                files[os.path.relpath(path, project)] = hashlib.sha256(f.read()).hexdigest()
    return files


def fail(message):
    print("smoke: " + message, file=sys.stderr)
    sys.exit(1)


server = subprocess.Popen(
    [studio, "mcp", "--project", project, "--expert"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
)
lines = queue.Queue()


def read_lines():
    for line in server.stdout:
        lines.put(line)
    lines.put(None)  # EOF: the server has ended


threading.Thread(target=read_lines, daemon=True).start()
next_id = [0]


def send(message):
    # Binary pipes: Windows must not turn the line feed into CRLF.
    server.stdin.write((json.dumps(message) + "\n").encode("utf-8"))
    server.stdin.flush()


def request(method, params):
    next_id[0] += 1
    wanted = next_id[0]
    send({"jsonrpc": "2.0", "id": wanted, "method": method, "params": params})
    while True:
        try:
            line = lines.get(timeout=LIMIT)
        except queue.Empty:
            server.kill()
            fail(method + ": no answer in %d s" % LIMIT)
        if line is None:
            fail(method + ": the server ended without an answer")
        answer = json.loads(line.decode("utf-8"))
        if answer.get("id") == wanted:
            if "error" in answer:
                fail(method + ": " + json.dumps(answer["error"]))
            return answer["result"]


def call(tool, arguments):
    result = request("tools/call", {"name": tool, "arguments": arguments})
    if result.get("isError"):
        fail(tool + ": " + json.dumps(result))
    return result


init = request(
    "initialize",
    {
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "clientInfo": {"name": "smoke", "version": "0"},
    },
)
if init["serverInfo"]["name"] != "gen4-studio":
    fail("unexpected server: " + json.dumps(init["serverInfo"]))
send({"jsonrpc": "2.0", "method": "notifications/initialized"})
print("smoke: initialize ok")

names = {t["name"] for t in request("tools/list", {})["tools"]}
for need in ("project_info", "g4_call", "save", "undo"):
    if need not in names:
        fail("tools/list has no " + need)
print("smoke: tools/list ok (%d tools)" % len(names))

# Cell (0, 0) of map 0 gets kind 0 and collision 64, a value the synthetic map has nowhere
# else (it uses 0 and 128). The edit must show in get_map, and save must write the map.
def structured(result):
    return result["structuredContent"]["result"]


before = snapshot()
call("g4_call", {"command": "set_permission", "args": {"index": 0, "x": 0, "y": 0, "kind": 0, "collision": 64}})
print("smoke: collision edit ok")
cell = structured(call("g4_call", {"command": "get_map", "args": {"index": 0}}))["permissions"][0]
if cell != [0, 64]:
    fail("get_map does not show the collision edit: " + json.dumps(cell))
saved = structured(call("save", {}))
if {"kind": "Map", "index": 0} not in saved["written"] or saved["problems"]:
    fail("save did not write the map: " + json.dumps(saved))
changed = sorted(p for p, h in snapshot().items() if before.get(p) != h)
if not changed:
    fail("save left every file of the project as it was")
print("smoke: save ok (on disk: %s)" % ", ".join(changed))

server.stdin.close()
try:
    status = server.wait(timeout=60)
except subprocess.TimeoutExpired:
    server.kill()
    fail("the server did not end after its stdin closed")
if status != 0:
    fail("the server ended with status %d" % status)
PY

echo "smoke ok"
