#!/usr/bin/env bash
# Tests for the rotom-lsp build in apps/studio/scripts/prepare-bundle.sh. The script itself builds
# the whole app, so the test runs only the block between "BEGIN rotom-lsp install" and "END
# rotom-lsp install" with a fake `cargo` (records its arguments and the variables that matter) and a
# fake `uname`: on macOS the linker gets -headerpad_max_install_names (without it a new linker
# leaves no room for the second `install_name_tool -add_rpath`), elsewhere nothing changes.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/apps/studio/scripts/prepare-bundle.sh"
HEADERPAD="-C link-arg=-Wl,-headerpad_max_install_names"

# setup <uname>: a folder with block.sh (the block of the script), bin/uname (prints <uname>) and
# bin/cargo (writes cargo.log: its arguments, RUSTFLAGS and CFLAGS, "unset" when not in the
# environment; FAKE_CARGO_STATUS is its exit status, default 0).
setup() {
  local d
  d="$(tmpdir)"
  mkdir -p "$d/bin" "$d/root/tools/rotom/bin"
  sed -n '/^# BEGIN rotom-lsp install$/,/^# END rotom-lsp install$/p' "$SCRIPT" >"$d/block.sh"
  [ -s "$d/block.sh" ] || fail "the markers of the rotom-lsp install block are gone from prepare-bundle.sh"
  printf '#!/bin/sh\necho %s\n' "$1" >"$d/bin/uname"
  cat >"$d/bin/cargo" <<'SH'
#!/usr/bin/env bash
{
  echo "args: $*"
  echo "RUSTFLAGS: ${RUSTFLAGS-unset}"
  echo "CFLAGS: ${CFLAGS-unset}"
} >>"$(dirname "$0")/../cargo.log"
exit "${FAKE_CARGO_STATUS:-0}"
SH
  chmod +x "$d/bin/uname" "$d/bin/cargo"
  printf '%s' "$d"
}

# run_block <dir> <exe> [NAME=value...]: the block with ROOT=<dir>/root and EXE=<exe> ("" or ".exe").
run_block() {
  local d="$1" exe="$2"
  shift 2
  env -u RUSTFLAGS -u CFLAGS PATH="$d/bin:$PATH" ROOT="$d/root" EXE="$exe" ROTOM_TAG=v9.9.9 "$@" \
    bash -c 'set -euo pipefail; . "$1"' _ "$d/block.sh"
}

test_macos_builds_rotom_lsp_with_headerpad() {
  local d
  d="$(setup Darwin)"
  run_block "$d" ""
  grep -q "^RUSTFLAGS: $HEADERPAD\$" "$d/cargo.log" || fail "no headerpad flag: $(cat "$d/cargo.log")"
  grep -q '^CFLAGS: -D_POSIX_C_SOURCE=200809L' "$d/cargo.log" || fail "CFLAGS lost: $(cat "$d/cargo.log")"
  grep -q "^args: install --git https://github.com/KalaayPT/rotom --tag v9.9.9 --locked rotom-lsp --root $d/root/tools/rotom\$" "$d/cargo.log" ||
    fail "unexpected cargo call: $(cat "$d/cargo.log")"
}

# The tag ships a Cargo.lock (checked for v0.7.0 and v0.8.1); `cargo install` ignores it without --locked and
# would take the newest dependencies at build time, on a machine that also holds the signing key.
test_windows_installs_rotom_lsp_locked_too() {
  local d
  d="$(setup MINGW64_NT-10.0)"
  run_block "$d" ".exe"
  grep -q "^args: install --git https://github.com/KalaayPT/rotom --tag v9.9.9 --locked rotom-lsp --root $d/root/tools/rotom\$" "$d/cargo.log" ||
    fail "unexpected cargo call: $(cat "$d/cargo.log")"
}

test_macos_keeps_the_rustflags_of_the_caller() {
  local d
  d="$(setup Darwin)"
  run_block "$d" "" "RUSTFLAGS=-C target-cpu=native"
  grep -q "^RUSTFLAGS: -C target-cpu=native $HEADERPAD\$" "$d/cargo.log" || fail "RUSTFLAGS: $(cat "$d/cargo.log")"
}

test_other_unix_leaves_the_environment_alone() {
  local d
  d="$(setup Linux)"
  run_block "$d" ""
  grep -q '^RUSTFLAGS: unset$' "$d/cargo.log" || fail "RUSTFLAGS was set: $(cat "$d/cargo.log")"
}

test_windows_leaves_the_environment_alone() {
  local d
  d="$(setup MINGW64_NT-10.0)"
  run_block "$d" ".exe"
  grep -q '^RUSTFLAGS: unset$' "$d/cargo.log" || fail "RUSTFLAGS was set: $(cat "$d/cargo.log")"
  grep -q '^CFLAGS: unset$' "$d/cargo.log" || fail "CFLAGS was set: $(cat "$d/cargo.log")"
}

test_an_existing_rotom_lsp_is_not_rebuilt() {
  local d
  d="$(setup Darwin)"
  : >"$d/root/tools/rotom/bin/rotom-lsp"
  run_block "$d" ""
  [ ! -e "$d/cargo.log" ] || fail "cargo ran: $(cat "$d/cargo.log")"
}

test_a_failing_cargo_install_stops_the_script() {
  local d
  d="$(setup Darwin)"
  assert_status 1 run_block "$d" "" FAKE_CARGO_STATUS=1
}

run_tests
