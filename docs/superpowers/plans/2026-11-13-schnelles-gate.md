# Schnelles Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `scripts/gate.sh` ohne Argumente läuft auf dem Entwicklungs-Mac in höchstens 900 s und führt weiter alle Tests ohne ROM aus; die volle ROM-Matrix läuft entkoppelt als `gate.sh --full`.

**Architecture:** Teil A baut jede Variante des Kunstprojekts einmal pro Testprozess als Vorlage und gibt jedem Test einen Klon (`clonefile(2)` auf macOS). Teil B teilt `gate.sh` in ein schnelles Gate und einen Volllauf mit Wiederaufsetzen, Ergebnisdatei und Nachtlauf. Bedingte Tests werden zur Laufzeit erkannt: Jeder Test, der ohne eine `G4_*`-Variable überspringt, schreibt seinen Namen in ein Protokoll, aus dem der Volllauf seine Auswahl bildet.

**Tech Stack:** Rust (stable, `libc`, `tempfile`), bash 3.2, `scripts/tests/lib.sh`, launchd, `jq` nicht voraussetzen (JSON per `printf` schreiben, per `python3 -I` lesen).

**Spec:** `docs/superpowers/specs/2026-11-13-schnelles-gate-design.md`

## Global Constraints

- Das schnelle Gate lässt keinen Test weg, der keine ROM braucht (Ziel 2).
- Jeder schreibende Test arbeitet auf seiner eigenen Kopie; Vorlagen werden nach dem Bau nie verändert.
- Öffentliche Signaturen in `crates/g4-kernel/src/test_support.rs` bleiben; Aufrufstellen werden nicht angefasst.
- Kein Wiederholungsversuch für Tests im Volllauf, außer dem vorhandenen bei `build graph (timings)`.
- Bedingter Test: überspringt ohne eine von `G4_TEST_PROJECT`, `G4_TEST_ROM`, `G4_APICULA`, `G4_DSROM_CLI`, `G4_ROTOM_LSP`.
- Grün ist ein Volllauf nur ohne fehlgeschlagene und ohne „skipped“-Zeile; sonst `rot` oder `unvollstaendig`.
- Release-Regel ist die lockere Fassung: `release-local.sh` warnt, blockiert nie.
- Nicht anfassen: `.github/workflows/*` außer der Kommentar-Anpassung in Task 5, Defender, cargo-nextest.
- Sprache: Code-Kommentare englisch, Ausgaben und Doku deutsch, wie im Bestand.
- Messungen nur bei ruhiger Maschine (`uptime` unter 5 vor dem Lauf); Zahlen ins Protokoll `../gate-artefakte/2026-11-13-schnelles-gate/`.
- Commits nur auf einem eigenen Branch `schnelles-gate`, nie direkt auf `main`.

## Review Focus

1. Ein Test verändert die Vorlage statt seines Klons (etwa über einen zwischengespeicherten Pfad): Der nächste Test muss trotzdem ein unverändertes Projekt sehen. Test in Task 2.
2. Der Klon liegt an einem anderen Pfad als die Vorlage; eine Datei im Projekt, die absolute Pfade enthält, zeigt dann auf die Vorlage. Erwartet: Klon und Neuaufbau sind bis auf den Wurzelpfad gleich. Test in Task 2.
3. `--from` nach einer Code-Änderung oder mit schmutzigem Arbeitsbaum: muss ablehnen, nicht still ein altes grünes Ergebnis übernehmen. Test in Task 7.
4. Ein Volllauf ohne Werkzeug oder ROM endet mit „skipped“-Zeilen: Ergebnis muss `unvollstaendig` sein, nicht `gruen`. Test in Task 7.
5. Ein neuer bedingter Test liest eine `G4_*`-Variable direkt statt über den Helfer und fällt so aus dem Volllauf. Erwartet: eine Prüfung schlägt an. Test in Task 6.

---

### Task 1: `clone_tree`

**Files:**
- Create: `crates/g4-kernel/src/test_support/clone.rs`
- Modify: `crates/g4-kernel/src/test_support.rs` (`mod clone; pub use clone::clone_tree;`, `vanilla_copy` Zeile 762–768)

**Interfaces:**
- Produces: `pub fn clone_tree(src: &Path, dst: &Path)` — `dst` darf nicht existieren, sein Elternordner muss existieren. macOS: `libc::clonefile(src, dst, 0)`; bei Fehler oder anderer Plattform `copy_dir(src, dst)`. Panik nur, wenn auch `copy_dir` scheitert.

- [ ] **Step 1: Tests schreiben** (in `clone.rs`, `#[cfg(test)]`)

```rust
#[test]
fn clone_tree_copies_nested_files_and_leaves_the_source_untouched() {
    let src = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("a/b")).unwrap();
    std::fs::write(src.path().join("a/b/x.bin"), [1, 2, 3]).unwrap();
    let out = tempfile::tempdir().unwrap();
    let dst = out.path().join("copy");
    clone_tree(src.path(), &dst);
    assert_eq!(std::fs::read(dst.join("a/b/x.bin")).unwrap(), [1, 2, 3]);
    std::fs::write(dst.join("a/b/x.bin"), [9]).unwrap();
    assert_eq!(std::fs::read(src.path().join("a/b/x.bin")).unwrap(), [1, 2, 3]);
}

#[test]
fn clone_tree_falls_back_to_a_copy_across_volumes() {
    // dst on another volume makes clonefile fail with EXDEV; only where /Volumes has one.
}
```

Der zweite Test überspringt, wenn kein zweites Volume da ist; sein Kern ist ein direkter Aufruf des Rückfallpfads über eine interne `fn clone_or_copy(src, dst, force_copy: bool)`.

- [ ] **Step 2:** `cargo test -p g4-kernel --all-features --lib test_support::clone` → FAIL (nicht definiert).
- [ ] **Step 3:** `clone_tree` implementieren; `vanilla_copy()` ruft `clone_tree(&src, &dst)` statt `copy_dir`.
- [ ] **Step 4:** Derselbe Befehl → PASS.
- [ ] **Step 5:** Commit `test(kernel): clone_tree, one clonefile per fixture copy`.

### Task 2: Vorlagen und Probeumbau an `TextFixture` (Haltepunkt)

**Files:**
- Create: `crates/g4-kernel/src/test_support/template.rs`
- Modify: `crates/g4-kernel/src/test_support.rs` (`TextFixture::dspre`, `TextFixture::exported`, Zeilen 908–957)

**Interfaces:**
- Consumes: `clone_tree` (Task 1).
- Produces:
  - `pub(crate) fn template(key: &str, build: impl FnOnce(&Path)) -> &'static Path` — baut `build(dir)` höchstens einmal pro Prozess und Schlüssel (Sperre pro Schlüssel, damit parallele Tests warten statt doppelt zu bauen). Ablage unter `std::env::temp_dir()/g4-fixtures-<pid>/<key>`; ein `libc::atexit`-Handler löscht `g4-fixtures-<pid>`.
  - `pub(crate) fn from_template(key: &str, build: impl FnOnce(&Path)) -> tempfile::TempDir` — frisches `TempDir`, darin Klon der Vorlage unter demselben Unterordnernamen, den der Neuaufbau heute benutzt.
  - Schlüssel: `"text-dspre"`, `"text-exported"`.

- [ ] **Step 1: Ausgangsmessung.** Ruhige Maschine, dann

```bash
RUSTC_BOOTSTRAP=1 cargo test -p g4-kernel --all-features --lib -- -Z unstable-options --report-time textproject > ../gate-artefakte/2026-11-13-schnelles-gate/textproject-vorher.log 2>&1
```

Summe der `<…s>`-Werte und „finished in“ notieren.

- [ ] **Step 2: Tests schreiben** (in `template.rs`)

```rust
#[test]
fn a_clone_of_text_exported_equals_a_fresh_build() {
    // tree_digest: sorted (relative path, sha1 of content) of every file, root path replaced by "<root>" in text files
    let cloned = TextFixture::exported();
    let fresh = TextFixture::exported_uncached();
    assert_eq!(tree_digest(cloned.root()), tree_digest(fresh.root()));
}

#[test]
fn writing_into_one_clone_does_not_reach_the_next() {
    let a = TextFixture::dspre();
    a.write("header.yaml", b"changed");
    let b = TextFixture::dspre();
    assert_ne!(b.read("header.yaml"), b"changed");
}

#[test]
fn template_builds_once_under_parallel_callers() {
    // 8 threads call template("probe-once", |d| { COUNTER += 1; write a file }); COUNTER == 1
}
```

`exported_uncached()` ist der heutige Rumpf von `exported()`, `#[cfg(test)]`-sichtbar behalten.

- [ ] **Step 3:** `cargo test -p g4-kernel --all-features --lib test_support::template` → FAIL.
- [ ] **Step 4:** `template`, `from_template` implementieren; `dspre()` und `exported()` darauf umstellen. Die synthetische ROM (`synthetic_rom()`) bleibt pro Test, sie ist eine Datei. Schlägt der Gleichheitstest an einer Datei mit absolutem Pfad fehl: diese Datei nach dem Klon neu schreiben, nicht den Test lockern.
- [ ] **Step 5:** Step-3-Befehl → PASS; dann `cargo test -p g4-kernel --all-features --lib textproject` → PASS.
- [ ] **Step 6: Nachmessung** wie Step 1 nach `textproject-nachher.log`.
- [ ] **Step 7: Haltepunkt.** Ist die Summe der Testzeiten nachher größer als ein Fünftel von vorher: nicht weiter mit Task 3 und 4, Zahlen und Stack-Sample eines langsamen Tests (`sample <binary> 10 -wait`) an den Auftraggeber melden; Tasks 5–10 laufen weiter.
- [ ] **Step 8:** Commit `test(kernel): build the text fixtures once per process, clone per test`.

### Task 3: Vorlagen für die übrigen Varianten

**Files:**
- Modify: `crates/g4-kernel/src/test_support.rs` (`synthetic_project` Z. 63, `synthetic_hgss_project` Z. 389, `synthetic_hgss_egg_moves` Z. 455, `synthetic_platinum_egg_moves` Z. 469, `TextFixture::with`/`unmarked` Z. 932–957)
- Modify: gleichartige Helfer unter `apps/studio/src-tauri/src` (mit `grep -rn "synthetic_project\|make_synthetic" apps/studio/src-tauri/src` finden)

**Interfaces:**
- Consumes: `template`, `from_template` (Task 2).
- Schlüssel: `"pt"`, `format!("hgss-{headers}-{}", location_names.join("\u{1f}"))`, `"hgss-egg"`, `format!("pt-egg-{table_at}")`.
- `with(prepare)` und `unmarked(prepare)`: Klon von `"pt"`, dann `prepare`, dann `unpack_all` und `place_layer` wie heute (die Closure ist nicht ablegbar).

- [ ] **Step 1:** Für jede Variante einen Gleichheitstest nach dem Muster aus Task 2 (`a_clone_of_<key>_equals_a_fresh_build`), je mit `_uncached`-Zwilling.
- [ ] **Step 2:** `cargo test -p g4-kernel --all-features --lib test_support` → FAIL.
- [ ] **Step 3:** Varianten umstellen.
- [ ] **Step 4:** `cargo test --workspace --all-features` (ohne `G4_*`) → alle grün, Dauer notieren.
- [ ] **Step 5:** Commit `test(kernel): build every synthetic project once per process`.

### Task 4: Löschkosten messen und entscheiden

**Files:**
- Modify: `crates/g4-kernel/src/test_support.rs` (nur wenn die Messung es verlangt)

- [ ] **Step 1:** Voller Lauf mit `--report-time` wie in Task 2 Step 1, aber `--workspace`, nach `workspace-nach-task3.log`. Die 30 langsamsten Tests listen.
- [ ] **Step 2:** Einen davon mit `sample` abtasten. Liegt mehr als ein Drittel der Stichproben in `__unlinkat`/`rmdir`: Step 3, sonst Step 4.
- [ ] **Step 3 (nur bei Bedarf):** `pub fn shared_synthetic_project() -> &'static Path` (die Vorlage `"pt"` selbst, per `chmod -R a-w` schreibgeschützt) einführen und nur lesende Tests der drei teuersten Module darauf umstellen. Ein Test, der dabei schreibt, scheitert an `EACCES` und bleibt beim Klon.
- [ ] **Step 4:** `cargo test --workspace --all-features` → grün. Ist die Wanduhrzeit von `cargo test (no ROM data)` über 480 s, die Zahlen im Protokoll festhalten; Task 5 Step 6 entscheidet über Parallelität.
- [ ] **Step 5:** Commit (falls geändert) `test(kernel): read-only tests share the template`.

### Task 5: `gate.sh` in schnelles Gate und Volllauf teilen

**Files:**
- Modify: `scripts/gate.sh`
- Create: `scripts/tests/gate.test.sh`
- Modify: `.github/workflows/ci.yml` (nur Kommentar Z. 2 und 11, falls er die Reihenfolge nennt)

**Interfaces:**
- Produces (Kommandozeile): `gate.sh [--full] [--from <Schritt>]`. `G4_GATE_HERMETIC=1` endet weiter nach dem letzten Schritt ohne ROM.
- Produces (Schrittnamen, Reihenfolge schnelles Gate): `cargo fmt --check`, `cargo clippy (-D warnings)`, `cargo deny (licenses, advisories)`, `npm licenses`, `script tests`, `actions`, `no rom data`, `links`, `version`, `tokens`, `styles`, `depth`, `pnpm test`, `pnpm build`, `dist has no bridge`, `g4emu unit`, `cargo test (no ROM data)`, `mcp`; lokal danach `rom sample (project start)` und `rom sample (timings)`, beide an `Pokemon - Platin-Edition (Germany)`.
- Produces (Test-Naht): `G4_GATE_CMD_PREFIX` — ist sie gesetzt, stellt `run_step` sie jedem Kommando voran. Die Skript-Tests setzen sie auf ein Stub-Skript, das sein Argument protokolliert und nach `G4_GATE_STUB_FAIL` (Schrittname) scheitert.
- `uv`-Schritte laufen mit `UV_NATIVE_TLS=1`.

- [ ] **Step 1: Tests schreiben** (`scripts/tests/gate.test.sh`, mit `lib.sh`)

```bash
test_fast_gate_runs_cheap_steps_before_cargo_test() {
  out="$(run_gate)"                       # stub prefix, G4_GATE_HERMETIC=1
  assert_before "$out" "pnpm test" "cargo test (no ROM data)"
  assert_before "$out" "g4emu unit" "cargo test (no ROM data)"
}
test_fast_gate_has_no_per_project_pass() {
  assert_not_contains "gated tests (cargo test)" run_gate_local
  assert_contains "rom sample (project start)" run_gate_local
}
test_full_gate_keeps_the_per_project_pass() {
  assert_contains "gated tests (cargo test)" run_gate_local --full
}
test_uv_steps_get_native_tls() {
  assert_contains "UV_NATIVE_TLS=1" run_gate
}
test_unknown_argument_is_refused() {
  assert_status 2 bash "$REPO_ROOT/scripts/gate.sh" --voll
}
```

`run_gate_local` legt ein Wegwerf-`G4_ROMS_DIR` mit drei leeren `*_DSPRE_contents/header.yaml` und drei leeren `.nds` an. `assert_before` und `assert_not_contains` kommen nach `lib.sh`.

- [ ] **Step 2:** `bash scripts/tests/gate.test.sh` → FAIL.
- [ ] **Step 3:** `gate.sh` umbauen: Argumente lesen, Schritte umsortieren, den Block ab Zeile 195 hinter `--full` legen, ROM-Stichprobe für das schnelle lokale Gate, Kopfkommentar anpassen.
- [ ] **Step 4:** `bash scripts/tests/run.sh` → `script tests: <n> ok`.
- [ ] **Step 5:** Echtlauf `G4_GATE_HERMETIC=1 bash scripts/gate.sh` auf ruhiger Maschine; Tabelle ins Protokoll.
- [ ] **Step 6 (nur wenn die Summe über 900 s liegt):** `pnpm test` und `pnpm build` als Hintergrundprozess neben `cargo test (no ROM data)` starten, Ergebnis und Sekunden beider nach `wait` in die Tabelle; Test `test_parallel_steps_both_reported` ergänzen.
- [ ] **Step 7:** Commit `feat(gate): fast gate by default, the ROM matrix behind --full`.

### Task 6: Bedingte Tests erkennen und auswählen

**Files:**
- Modify: `crates/g4-formats/src/test_support.rs` (neuer Helfer)
- Modify: `crates/g4-kernel/src/test_support.rs` (`vanilla_project` Z. 751, `vanilla_base_rom` Z. 823 und alle weiteren `G4_*`-Leser dort)
- Modify: jede Testdatei, die eine der fünf Variablen direkt liest (`grep -rnE 'var(_os)?\("G4_(TEST_PROJECT|TEST_ROM|APICULA|DSROM_CLI|ROTOM_LSP)"' crates apps/studio/src-tauri`)
- Create: `scripts/check-gate-vars.sh`, `scripts/tests/check-gate-vars.test.sh`
- Modify: `scripts/gate.sh`

**Interfaces:**
- Produces: `pub fn gate_var(name: &str) -> Option<std::ffi::OsString>` in `g4_formats::test_support`. Fehlt die Variable: `eprintln!("skipped: {name} not set")` und, wenn `G4_SKIP_LOG` gesetzt ist, eine Zeile `<name>\t<Testname>` anhängen (`std::thread::current().name()`; ein Append pro Aufruf mit `O_APPEND`).
- Produces: `scripts/check-gate-vars.sh` — scheitert, wenn außerhalb der beiden `test_support.rs` eine der fünf Variablen per `env::var`/`var_os` gelesen wird.
- Produces in `gate.sh`: `cargo test (no ROM data)` läuft mit `G4_SKIP_LOG=$ROOT/target/gate/skips.tsv` (vorher geleert, nie mit `--test-threads 1`). Der Volllauf bildet daraus drei Listen: Zeilen mit `G4_TEST_PROJECT` → `gated tests (cargo test)` je Projekt; `G4_TEST_ROM` → wie heute `project_start_`; übrige → neuer Schritt `tool tests`, einmal, mit allen Werkzeug-Variablen und dem Platin-Projekt gesetzt. Aufruf je Liste: `cargo test --workspace --all-features --no-fail-fast -- --exact <Namen…>`, ohne Namen mit `build_graph_timings`.
- Produces in `gate.sh`: jeder dieser Schritte setzt ein eigenes `G4_SKIP_LOG`; ist es danach nicht leer, zählt der Schritt als FAILED mit der Meldung `bedingter Test übersprungen: <Zeilen>`.

- [ ] **Step 1: Tests schreiben**

```rust
// crates/g4-formats/src/test_support.rs
#[test]
fn gate_var_logs_the_test_name_when_the_variable_is_missing() {
    // set G4_SKIP_LOG to a temp file, call gate_var("G4_NOT_SET_FOR_THIS_TEST")
    // file content == "G4_NOT_SET_FOR_THIS_TEST\t<this test's path>\n"
}
```

```bash
# scripts/tests/check-gate-vars.test.sh
test_a_direct_read_outside_test_support_fails() { ... assert_status 1 ... }
test_the_repository_passes() { assert_status 0 bash "$REPO_ROOT/scripts/check-gate-vars.sh"; }
# scripts/tests/gate.test.sh
test_full_gate_runs_only_logged_tests_per_project() { ... assert_contains "--exact" ... }
test_a_skip_in_the_full_run_is_a_failure() { ... stub writes a line into G4_SKIP_LOG ... assert_contains "bedingter Test übersprungen" ... }
```

- [ ] **Step 2:** Tests laufen lassen → FAIL.
- [ ] **Step 3:** `gate_var` schreiben, alle direkten Leser darauf umstellen (Verhalten sonst unverändert), `check-gate-vars.sh` als Schritt `gate vars` nach `version` ins Gate, Auswahl im Volllauf.
- [ ] **Step 4:** `bash scripts/tests/run.sh` und `cargo test --workspace --all-features` → grün; `wc -l target/gate/skips.tsv` notieren.
- [ ] **Step 5:** Gegenprobe gegen heute: für ein Projekt die Zahl der im alten Durchlauf nicht übersprungenen bedingten Tests (`grep -c` in `gate-5-fix.log` nicht möglich, daher einmal `--full` nur für Platin laufen lassen: `G4_GATE_ONLY_PROJECT="Pokemon - Platin-Edition (Germany)"`) — der Schritt muss grün sein und sein `G4_SKIP_LOG` leer.
- [ ] **Step 6:** Commit `feat(gate): the full run picks the conditional tests from a skip log`.

### Task 7: Ergebnisdatei, „grün“, Wiederaufsetzen

**Files:**
- Modify: `scripts/gate.sh`, `scripts/tests/gate.test.sh`

**Interfaces:**
- Produces: `G4_GATE_ARTEFAKTE` (Vorgabe: `<Haupt-Checkout>/../gate-artefakte/full`, Haupt-Checkout = Elternordner von `git rev-parse --git-common-dir`); `G4_ROMS_DIR`-Vorgabe ebenso am Haupt-Checkout verankert.
- Produces: nach jedem `--full`-Lauf `<sha>.json` und `<sha>.log` dort. JSON-Felder: `sha`, `ergebnis` (`gruen` | `rot` | `unvollstaendig`), `sekunden`, `wiederaufgesetzt` (bool), `beginn` (ISO 8601), `zeilen` (Liste aus `schritt`, `projekt`, `ergebnis`, `sekunden`).
- `ergebnis`: `rot` bei einer FAILED-Zeile, sonst `unvollstaendig` bei einer Zeile, deren Ergebnis mit `skipped` beginnt, sonst `gruen`. Mit schmutzigem Arbeitsbaum wird keine Datei geschrieben, nur die Tabelle.
- `--full` bricht nach einem Fehlschlag in einem Projekt nicht ab: `run_step` bekommt im Volllauf für die Schritte ab `gated tests` den Modus „weiter“, Exit-Status am Ende 1.
- `--from <Schritt>`: nur mit `--full`; verlangt sauberen Arbeitsbaum und ein vorhandenes `<sha>.json` für `HEAD`; übernimmt dessen `ok`-Zeilen vor `<Schritt>`, führt den Rest aus, schreibt `wiederaufgesetzt: true`. Sonst Exit 2 mit `--from: kein Ergebnis für <sha>` bzw. `--from: Arbeitsbaum nicht sauber`. Bei mehrfach vorkommendem Schrittnamen: `--from "<Schritt>@<Projekt>"`.

- [ ] **Step 1: Tests schreiben** (alle mit Stub-Präfix in einem Wegwerf-Repo)

```bash
test_full_run_writes_a_green_result()            # ergebnis == gruen, sha == HEAD
test_a_skipped_row_makes_the_result_incomplete() # ohne Emulatorkern → unvollstaendig
test_a_failed_project_does_not_stop_the_others() # G4_GATE_STUB_FAIL trifft Projekt 1; Projekt 2 und 3 stehen in zeilen
test_from_resumes_and_marks_the_result()         # wiederaufgesetzt == true, Schritte davor nicht erneut aufgerufen
test_from_refuses_a_dirty_tree()                 # Exit 2, "Arbeitsbaum nicht sauber"
test_from_refuses_another_commit()               # nach neuem Commit: Exit 2, "kein Ergebnis für"
test_the_result_path_is_anchored_at_the_main_checkout() # aus einem git worktree gestartet
```

- [ ] **Step 2:** `bash scripts/tests/gate.test.sh` → FAIL.
- [ ] **Step 3:** Implementieren.
- [ ] **Step 4:** `bash scripts/tests/run.sh` → ok.
- [ ] **Step 5:** Commit `feat(gate): result file, incomplete runs and --from for the full run`.

### Task 8: Nachtlauf

**Files:**
- Create: `scripts/gate-nightly.sh`, `scripts/release/de.gen4studio.gate-nightly.plist`, `scripts/tests/gate-nightly.test.sh`

**Interfaces:**
- Consumes: `gate.sh --full`, `<sha>.json` (Task 7).
- Produces: `scripts/gate-nightly.sh` — `git fetch origin main`; endet mit 0 und `Nachtlauf: <sha> ist schon grün`, wenn `<sha>.json` mit `ergebnis: gruen` existiert; sonst Worktree `../gen4-studio-nightly` auf `origin/main` anlegen oder aktualisieren, darin `tools` auf das `tools/` des Haupt-Checkouts verlinken, `pnpm install --frozen-lockfile` in `apps/studio`, `CARGO_TARGET_DIR=<Haupt-Checkout>/target`, dann `bash scripts/gate.sh --full`.
- Produces: plist mit `StartCalendarInterval` 01:30, `StandardOutPath`/`StandardErrorPath` nach `gate-artefakte/full/nightly.log`, Programm `/bin/bash <Haupt-Checkout>/scripts/gate-nightly.sh` (Pfad als Platzhalter `@REPO@`, den die Anleitung per `sed` ersetzt).

- [ ] **Step 1:** Tests: `test_a_green_result_skips_the_run`, `test_a_red_result_does_not_block_the_next_run`, `test_the_worktree_gets_tools_and_node_modules` (Stubs für `git fetch`, `pnpm`, `gate.sh`).
- [ ] **Step 2:** → FAIL. **Step 3:** Implementieren. **Step 4:** `bash scripts/tests/run.sh` → ok.
- [ ] **Step 5:** `plutil -lint scripts/release/de.gen4studio.gate-nightly.plist` → OK.
- [ ] **Step 6:** Commit `feat(gate): nightly full run on origin/main`.

### Task 9: Hinweis in `release-local.sh`

**Files:**
- Modify: `scripts/release-local.sh` (in Schritt 1, nach der Prüfung HEAD == main == origin/main), `scripts/tests/release-local.test.sh`

**Interfaces:**
- Consumes: `<sha>.json`, `G4_GATE_ARTEFAKTE` (Task 7).
- Produces: genau eine der Ausgaben, das Skript läuft immer weiter:
  - `Volllauf: grün auf diesem Commit (<sha>)`
  - `WARNUNG Volllauf: letzter grüner Stand <sha> (<n> Commits zurück)`, danach `git log --oneline <sha>..HEAD` und `git diff --stat <sha>..HEAD`
  - `WARNUNG Volllauf: kein grünes Ergebnis auf einem Vorfahren`
- Gesucht wird entlang `git rev-list HEAD`, höchstens 200 Commits.

- [ ] **Step 1:** Tests `test_full_run_on_head_is_reported`, `test_full_run_on_an_ancestor_warns_and_lists_the_commits`, `test_no_full_run_warns_and_continues` (jeweils zusätzlich: Exit-Status des Probelaufs bleibt 0).
- [ ] **Step 2:** → FAIL. **Step 3:** Implementieren. **Step 4:** `bash scripts/tests/release-local.test.sh` → ok.
- [ ] **Step 5:** Commit `feat(release): say where the last green full run is`.

### Task 10: Doku und Abnahme

**Files:**
- Modify: `docs/release.md` (Z. 127–140 Gate-Regel, Ablauf Z. 142–196), `docs/dev-setup.md` (Z. 725, Z. 762 Zeitangabe, neuer Abschnitt „Nachtlauf einrichten“), `docs/agent-gate.md` (Aufrufe und Zeiten), `docs/briefing-release-ci.md` Z. 19, `CONTRIBUTING.md` falls es das Gate nennt

- [ ] **Step 1:** Doku anpassen: schnelles Gate vor jedem Merge, Volllauf nachts und auf Abruf, `--from`, Ergebnisdatei, lockere Release-Regel mit ihrem Preis, gemessene Zeiten statt der alten.
- [ ] **Step 2:** `bash scripts/check-links.sh` und `bash scripts/tests/run.sh` → ok.
- [ ] **Step 3: Abnahme schnelles Gate.** Ruhige Maschine, `bash scripts/gate.sh` → `Gate: ok (<n> s)` mit n ≤ 900; Tabelle ins Protokoll. Liegt n darüber: Zahlen an den Auftraggeber, nichts weglassen.
- [ ] **Step 4: Abnahme Volllauf.** `bash scripts/gate.sh --full` → `<sha>.json` mit `ergebnis: gruen`; Dauer ins Protokoll und in die Doku.
- [ ] **Step 5:** Commit `docs: fast gate and nightly full run`.

## Abhängigkeiten

Tasks 1–4 (Teil A) und 5–9 (Teil B) sind voneinander unabhängig; innerhalb gilt die Reihenfolge. Task 6 braucht 5, Task 7 braucht 6, Tasks 8 und 9 brauchen 7. Task 10 kommt zuletzt.
