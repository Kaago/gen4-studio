#!/usr/bin/env bash
# Tests for scripts/check-gate-vars.sh: a test reads G4_TEST_PROJECT, G4_TEST_ROM, G4_APICULA,
# G4_DSROM_CLI and G4_ROTOM_LSP only through g4_formats::test_support::gate_var, which logs the skip
# for the full run of the gate. The script scans the .rs files of a repository root it is given.
set -euo pipefail

. "$(dirname "$0")/lib.sh"

SCRIPT="$REPO_ROOT/scripts/check-gate-vars.sh"

# write_rs <root> <path below the root> <text>: a source file with the text as its content.
write_rs() {
  mkdir -p "$(dirname "$1/$2")"
  printf '%s\n' "$3" >"$1/$2"
}

test_a_direct_read_outside_test_support_fails() {
  local d var
  d="$(tmpdir)"
  for var in G4_TEST_PROJECT G4_TEST_ROM G4_APICULA G4_DSROM_CLI G4_ROTOM_LSP; do
    write_rs "$d" crates/g4-kernel/tests/read.rs \
      "fn f() { let _ = std::env::var_os(\"$var\"); }"
    assert_status 1 bash "$SCRIPT" "$d"
    assert_contains "crates/g4-kernel/tests/read.rs:1:" bash "$SCRIPT" "$d"
    assert_contains "$var" bash "$SCRIPT" "$d"
    # The text is German and names the helper.
    assert_contains "g4_formats::test_support::gate_var" bash "$SCRIPT" "$d"
  done
}

test_every_spelling_of_the_read_is_found() {
  local d
  d="$(tmpdir)"
  write_rs "$d" apps/studio/src-tauri/src/a.rs 'let a = std::env::var("G4_APICULA").ok();'
  assert_status 1 bash "$SCRIPT" "$d"
  assert_contains "apps/studio/src-tauri/src/a.rs:1:" bash "$SCRIPT" "$d"
  # Imported, with blanks, with a map after it.
  write_rs "$d" apps/studio/src-tauri/src/a.rs 'let a = var_os( "G4_TEST_ROM" ).map(PathBuf::from);'
  assert_status 1 bash "$SCRIPT" "$d"
  write_rs "$d" apps/studio/src-tauri/src/a.rs 'let a = env::var_os ("G4_DSROM_CLI");'
  assert_status 1 bash "$SCRIPT" "$d"
}

test_a_read_split_over_lines_is_found() {
  local d out
  d="$(tmpdir)"
  # As rustfmt breaks a long call.
  write_rs "$d" crates/g4-kernel/src/x.rs 'fn f() {
    let apicula = std::env::var_os(
        "G4_APICULA",
    )
    .map(PathBuf::from);
}'
  assert_status 1 bash "$SCRIPT" "$d"
  out="$(bash "$SCRIPT" "$d" 2>&1 || true)"
  # The line of the call, not the line of the name.
  case "$out" in
    *"crates/g4-kernel/src/x.rs:2:"*) ;;
    *) fail "the hit is not at line 2: $out" ;;
  esac
}

test_both_test_support_files_may_read() {
  local d
  d="$(tmpdir)"
  write_rs "$d" crates/g4-formats/src/test_support.rs 'let v = std::env::var_os("G4_TEST_PROJECT");'
  write_rs "$d" crates/g4-kernel/src/test_support.rs 'let v = std::env::var_os("G4_APICULA");'
  assert_status 0 bash "$SCRIPT" "$d"
  # A file of the same name elsewhere is not one of them.
  write_rs "$d" crates/g4-kernel/tests/test_support.rs 'let v = std::env::var_os("G4_APICULA");'
  assert_status 1 bash "$SCRIPT" "$d"
}

test_other_variables_and_other_uses_pass() {
  local d
  d="$(tmpdir)"
  write_rs "$d" crates/g4-kernel/tests/ok.rs '// reads std::env::var_os("G4_TEST_PROJECT") in a comment
fn f() {
    let _ = std::env::var_os("G4_ROMS_DIR");
    let _ = std::env::var("G4_APICULA_TIMEOUT_SECS");
    let _ = std::env::var_os("G4_TEST_PROJECT_OTHER");
    std::env::set_var("G4_APICULA", "x");
    std::env::remove_var("G4_TEST_ROM");
    let _ = find_sidecar("G4_APICULA", "apicula", "tools/apicula/bin/apicula");
    let _ = g4_formats::test_support::gate_var("G4_TEST_PROJECT");
    eprintln!("skipped: G4_DSROM_CLI not set");
}'
  assert_status 0 bash "$SCRIPT" "$d"
  assert_contains "gate vars: ok (" bash "$SCRIPT" "$d"
}

test_build_output_is_not_scanned() {
  local d
  d="$(tmpdir)"
  write_rs "$d" crates/g4-kernel/target/debug/build/x.rs 'let v = std::env::var_os("G4_APICULA");'
  write_rs "$d" apps/studio/src-tauri/target/y.rs 'let v = std::env::var_os("G4_APICULA");'
  assert_status 0 bash "$SCRIPT" "$d"
}

test_every_hit_is_listed() {
  local d out
  d="$(tmpdir)"
  write_rs "$d" crates/g4-kernel/tests/a.rs 'fn a() { std::env::var_os("G4_APICULA"); }'
  write_rs "$d" crates/g4-formats/tests/b.rs 'fn b() { std::env::var_os("G4_TEST_PROJECT"); }
fn c() { std::env::var_os("G4_TEST_ROM"); }'
  out="$(bash "$SCRIPT" "$d" 2>&1 || true)"
  case "$out" in
    *"crates/g4-kernel/tests/a.rs:1:"*"crates/g4-formats/tests/b.rs:1:"*) ;;
    *"crates/g4-formats/tests/b.rs:1:"*"crates/g4-kernel/tests/a.rs:1:"*) ;;
    *) fail "not both files are listed: $out" ;;
  esac
  case "$out" in
    *"crates/g4-formats/tests/b.rs:2:"*) ;;
    *) fail "the second read of a file is not listed: $out" ;;
  esac
  assert_contains "3 Funde" bash "$SCRIPT" "$d"
}

test_a_gate_call_inside_a_shared_cache_fails() {
  local d out
  d="$(tmpdir)"
  # The way tests/flags.rs had it: the first test that asks is the only one that logs its skip.
  write_rs "$d" crates/g4-kernel/tests/cache.rs 'fn adopted() -> Option<&'"'"'static Path> {
    static ADOPTED: OnceLock<Option<PathBuf>> = OnceLock::new();
    ADOPTED
        .get_or_init(|| {
            let src = common::vanilla_project()?;
            Some(src)
        })
        .as_deref()
}'
  assert_status 1 bash "$SCRIPT" "$d"
  out="$(bash "$SCRIPT" "$d" 2>&1 || true)"
  case "$out" in
    *"crates/g4-kernel/tests/cache.rs:5:"*"[in get_or_init]"*) ;;
    *) fail "the call in the cache is not named: $out" ;;
  esac
  for name in gate_var vanilla_copy vanilla_base_rom; do
    write_rs "$d" crates/g4-kernel/tests/cache.rs "fn f() { CACHE.get_or_init(|| { $name(\"G4_APICULA\") }); }"
    assert_status 1 bash "$SCRIPT" "$d"
  done
  # Asked on every call and only the work cached: fine.
  write_rs "$d" crates/g4-kernel/tests/cache.rs 'fn adopted() -> Option<&'"'"'static Path> {
    static ADOPTED: OnceLock<Option<PathBuf>> = OnceLock::new();
    let src = common::vanilla_project()?;
    ADOPTED
        .get_or_init(|| {
            let nds = base_rom(&src)?;
            Some(nds)
        })
        .as_deref()
}'
  assert_status 0 bash "$SCRIPT" "$d"
  # A get_or_init that asks for something else is no business of this check.
  write_rs "$d" crates/g4-kernel/tests/cache.rs 'fn re() -> &'"'"'static Regex { RE.get_or_init(|| Regex::new("a").unwrap()) }'
  assert_status 0 bash "$SCRIPT" "$d"
}

test_the_repository_passes() {
  assert_status 0 bash "$REPO_ROOT/scripts/check-gate-vars.sh"
  assert_contains "gate vars: ok (" bash "$REPO_ROOT/scripts/check-gate-vars.sh"
}

test_a_missing_root_is_refused() {
  assert_status 2 bash "$SCRIPT" "$(tmpdir)/nope"
}

run_tests
