# Release und CI: Implementierungsplan

> **Für Agenten:** Pflicht-Skill ist superpowers:subagent-driven-development (empfohlen) oder superpowers:executing-plans. Die Schritte nutzen Checkboxen (`- [ ]`).

**Ziel:** Gen4 Studio wird auf GitHub Actions hermetisch geprüft und als DMG (macOS arm64) und NSIS-Installer (Windows x64) als Release-Entwurf gebaut. Beides ist geprüft und enthält keine ROM-Daten.

**Architektur:**
- `scripts/gate.sh` bekommt den Schalter `G4_GATE_HERMETIC=1`. CI und lokal fahren damit dieselbe Liste.
- Neue Prüfskripte sind `check-no-rom-data`, `check-actions` und `version`, mit Shell-Tests unter `scripts/tests/`.
- Zwei Workflows: `ci.yml` und `release.yml`. Probeläufe vom Branch laufen über PR-Labels.
- Unter Windows kommt ein Zweig in `prepare-bundle.sh` dazu, außerdem eine Tauri-Konfiguration je Plattform und die Sidecar-Suche mit `EXE_SUFFIX`.
- `studio doctor` und `smoke-packaged.sh` prüfen die gepackte App ohne ROM.

**Technik:** Bash, GitHub Actions, Rust/Tauri 2, pnpm 12.16.1, Node 26, uv, cargo-deny 0.20.2, cargo-about, NSIS, `hdiutil`, `gh`.

**Spezifikation:** [docs/superpowers/specs/2026-11-04-release-ci-design.md](../specs/2026-11-04-release-ci-design.md). Wer umsetzt, liest sie ganz. Der Plan nennt nur, was dort nicht steht oder wo die Entscheidung fällt.

## Globale Vorgaben

- Keine ROMs, Spielstände oder ROM-Ableitungen nach GitHub, weder als Datei, Cache, Artefakt, Log, Secret noch Fixture. In CI laufen nur hermetische Schritte.
- Das Repo bleibt privat, und Releases bleiben Entwürfe (`--draft`). Gepusht wird nur `release-ci`, später `ci-rot`, `main` und der Tag `v0.1.0-rc.1`. Kein Force-Push auf `main`.
- Jede `uses:`-Zeile ist auf einen 40-stelligen SHA festgelegt und steht in `.github/actions-allowlist.txt`. Lizenzen sind nur aus der Erlaubnisliste von `deny.toml` zulässig, also kein `Swatinem/rust-cache`.
- nitroarc bleibt eine separate Datei: `libnitroarc_ffi.dylib` oder `nitroarc_ffi.dll`.
- Fassungen der Werkzeuge: apicula `3d4e91e14045392a49c89e86dab8cb936225588c`, rotom `v0.7.0`, cargo-deny `0.20.2`, pnpm `12.16.1`, Node `26`. Alle stehen in `scripts/tool-versions.env`.
- **Kontingent:** macOS zählt zehnfach, Windows doppelt. Erst lokal grün, dann gebündelt pushen. Höchstens ein Push auf den PR, solange ein Lauf noch läuft (concurrency bricht ab).
- **Commits:** Jeder Commit endet mit `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`, mit ASCII-`@`.
- **Sprache:** Kommentare in Skripten und Rust auf Englisch, Dokumente und Ausgaben für Nutzer auf Deutsch, wie im Repo.
- **Umgebung im Worktree:**
  - `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"` vor `cargo`;
  - Symlink `tools -> $HOME/Projekte/lab/editor/gen4-studio/tools`;
  - `pnpm install` in `apps/studio`;
  - `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` für das volle Gate.

## Review-Fokus

1. **Ein Fehltreffer oder ein übersehener Treffer von `check-no-rom-data.sh`:** Eine Datei mit `.nds`-Header unter fremdem Namen, etwa `foo.bin`, muss auffallen. Ein PNG oder GLB des Repos darf nicht anschlagen. Abgedeckt in Task 1 mit den Fällen `renamed_rom_detected` und `repo_files_clean`.
2. **`version.sh check` bei `v0.1.0-rc.1`:** Der Tag muss passen, obwohl die Version `0.1.0-rc.1` lautet und der CHANGELOG nur `## [0.1.0]` hat. Abgedeckt in Task 2 mit `rc_tag_uses_base_section`.
3. **Ein Pfad mit Leerzeichen:** `Gen4 Studio.app` und `RUNNER_TEMP` unter Windows dürfen kein Skript brechen. Abgedeckt in Task 1 (`path_with_space`) und Task 6 (Rauchtest mit `Gen4 Studio.app`).
4. **Eine leere Platzhalter-Datei als Sidecar in einem Release-Build:** Weder `studio doctor` noch `check-bundle*.sh` dürfen sie als Fund melden. Abgedeckt in Task 5 (`placeholder_is_not_found`) und Task 6 (Status 1 bei Platzhaltern).
5. **Ein gesetztes `G4_*` in der Shell beim Rauchtest:** Es darf die Prüfung „ohne Entwicklungsbaum“ nicht verfälschen. Abgedeckt in Task 6, `smoke-packaged.sh` läuft mit `env -i`, Test `env_is_clean`.

---

### Task 0: Worktree

- [ ] Im Hauptcheckout `git worktree add .claude/worktrees/release-ci -b release-ci main` ausführen, dann den Symlink `tools` anlegen und `pnpm install` in `apps/studio` ausführen.
- [ ] Prüfen: `G4_GATE_HERMETIC` gibt es noch nicht. `bash scripts/gate.sh` bis `g4emu unit` ist lokal grün. Das ist die Ausgangsmessung, die Sekunden je Schritt kommen ins Gate-Protokoll.

### Task 1: ROM-Schutz und Shell-Tests

**Dateien:**
- Neu: `scripts/check-no-rom-data.sh`, `scripts/tests/run.sh`, `scripts/tests/check-no-rom-data.test.sh`
- Ändern: `scripts/gate.sh`, mit den Schritten `script tests` und `no rom data` nach `npm licenses`

**Schnittstellen:**
- Erzeugt: `check-no-rom-data.sh [--history <rev-range>] [path …]`. Status 0 mit letzter Zeile `no rom data: <n> files ok`, sonst Status 1 und eine Zeile `ROM? <pfad>: <grund>` je Treffer. Gründe sind `extension .nds`, `folder _DSPRE_contents` und `nds header`.
- Erzeugt: `scripts/tests/run.sh` führt jedes `scripts/tests/*.test.sh` aus und endet mit `script tests: <n> ok`.
- Jede `*.test.sh` definiert Funktionen `test_<name>` und ruft am Ende `run_tests` aus `scripts/tests/lib.sh` auf. `lib.sh` bietet `assert_status <erwartet> <befehl…>`, `assert_contains <text> <befehl…>` und `tmpdir`.

- [ ] **Tests schreiben** in `check-no-rom-data.test.sh`. Jeder Test arbeitet in einem eigenen `tmpdir`:
  - `test_clean_dir_ok`: nur `a.txt` und `b.png` (das PNG aus `design-system/marke/`). Erwartet Status 0 und `no rom data: 2 files ok`.
  - `test_extension_nds`: `x.nds` (leer). Erwartet Status 1 und `extension .nds`.
  - `test_extensions_all`: je eine Datei mit `.sav`, `.dsv`, `.srl`, `.xdelta` und `.narc`. Erwartet Status 1 mit fünf Treffern.
  - `test_dspre_folder`: `P_DSPRE_contents/x.txt`. Erwartet Status 1 und `folder _DSPRE_contents`.
  - `test_renamed_rom_detected`: `foo.bin` mit 0x200 Nullbytes, an 0x15C die Bytes `56 CF`. Erwartet Status 1 und `nds header`.
  - `test_small_file_no_header_check`: eine Datei von 0x100 Byte. Erwartet Status 0.
  - `test_path_with_space`: Ordner `Gen4 Studio.app/` mit `x.nds`. Erwartet Status 1, und der Pfad wird ganz genannt.
  - `test_repo_files_clean`: ohne Argument im Repo. Erwartet Status 0.
  - `test_history_range`: in einem temporären Repo ein Commit mit `r.nds`, der danach gelöscht wird. `--history HEAD~2..HEAD` muss Status 1 ergeben.
- [ ] **Rot sehen:** `bash scripts/tests/run.sh`. Erwartet FAIL, weil das Skript fehlt.
- [ ] **Umsetzen:**
  - Die Header-Prüfung nimmt `dd`/`od` oder `python3 -I`, nur Standardbibliothek. Unter Git Bash unter Windows ist `python3` vorhanden.
  - Für `--history` werden `git rev-list --objects <range>` und `git cat-file --batch` mit Pfadnamen zur Endungsprüfung kombiniert. Die Blobs ab 0x160 Byte kommen in die Header-Prüfung.
  - DMG-Pfade (`*.dmg`): `hdiutil attach -readonly -nobrowse -mountpoint <tmp>`, prüfen, wieder aushängen. Unter Windows gibt es keine DMG.
- [ ] **Grün sehen:** `bash scripts/tests/run.sh`. Erwartet `script tests: 1 ok`.
- [ ] **Historie prüfen:** `bash scripts/check-no-rom-data.sh --history origin/main..main` im Hauptcheckout ausführen. Die Ausgabe kommt nach `target/agent/release-ci/history-check.txt`. Bei einem Treffer: **Stopp**, nichts pushen, den Befund an den Orchestrator melden.
- [ ] **Commit:** `feat(gate): check-no-rom-data and shell tests`

### Task 2: Versionierung

**Dateien:**
- Neu: `scripts/version.sh`, `scripts/tests/version.test.sh`, `CHANGELOG.md`
- Ändern:
  - `Cargo.toml` mit `[workspace.package] version = "0.1.0"`;
  - `crates/*/Cargo.toml` und `apps/studio/src-tauri/Cargo.toml` mit `version.workspace = true`;
  - `apps/studio/src-tauri/tauri.conf.json` mit `"version": "../package.json"`;
  - `scripts/gate.sh` mit dem Schritt `version` nach `no rom data`.

**Schnittstellen:**
- Erzeugt: `version.sh`, `version.sh set <semver>` und `version.sh check [<tag>]`, wie in Spezifikation §4. Ein Fehler gibt `version: <datei>: <wert>` aus und endet mit Status 1.
- Mit `G4_VERSION_ROOT` als Umgebungsvariable lässt sich das Skript für Tests auf eine Kopie richten.

- [ ] **Tests schreiben** in `version.test.sh`, auf einer Kopie von `Cargo.toml`, `package.json` und `CHANGELOG.md` in einem `tmpdir`:
  - `test_prints_version`: Ausgabe `0.1.0`.
  - `test_check_ok`.
  - `test_mismatch_fails`: `package.json` hat `0.2.0`. Erwartet Status 1 mit `package.json`.
  - `test_rc_tag_uses_base_section`: zuerst `set 0.1.0-rc.1`, dann `check v0.1.0-rc.1`. Erwartet Status 0.
  - `test_wrong_tag_fails`: `check v0.1.1`. Erwartet Status 1.
  - `test_missing_changelog_section`: `set 0.2.0` ohne Abschnitt. Ergibt Status 1 mit `CHANGELOG.md`.
  - `test_invalid_semver`: `set 1.0`. Erwartet Status 1, und nichts wird geschrieben.
- [ ] **Rot sehen, umsetzen, grün sehen.** `set` arbeitet mit `sed` auf genau der Zeile `version = …` in `[workspace.package]` und auf dem Feld `"version"` in `package.json`. Danach folgt `cargo update -w --offline`, das in Tests mit `G4_VERSION_SKIP_LOCK=1` übersprungen wird.
- [ ] **CHANGELOG schreiben:** Deutsch, „Keep a Changelog“, Abschnitt `## [0.1.0] - unveröffentlicht` mit 6 bis 10 Zeilen über die Phasen. Quelle sind die Überschriften der Phasen in `docs/ROADMAP.md`.
- [ ] **Prüfen:**
  - `cargo build -p studio` geht.
  - `bash scripts/version.sh check` ist grün.
  - `pnpm tauri info` oder `cargo tauri` liest `0.1.0`.
- [ ] **Commit:** `feat(release): one version for cargo, npm and tauri; changelog`

### Task 3: Hermetischer Schalter, Actions-Audit, `ci.yml`

**Dateien:**
- Neu: `scripts/check-actions.sh`, `scripts/tests/check-actions.test.sh`, `.github/actions-allowlist.txt`, `.github/workflows/ci.yml`, `scripts/tool-versions.env`
- Ändern: `scripts/gate.sh` mit dem Schritt `actions` nach `npm licenses` und mit `G4_GATE_HERMETIC`

**Schnittstellen:**
- Erzeugt: `check-actions.sh [workflow-dir]`, Standard `.github/workflows`. Es endet mit `actions: <n> uses ok`. Ein Fehler gibt `actions: <datei>:<zeile>: <grund>` aus, mit einem dieser Gründe: `not pinned to a sha`, `not in allowlist` oder `license <x> not allowed`.
- `tool-versions.env` ist eine Datei zum Einlesen mit `source`. Sie enthält:
  - `APICULA_REV=3d4e91e14045392a49c89e86dab8cb936225588c`
  - `ROTOM_TAG=v0.7.0`
  - `CARGO_DENY_VERSION=0.20.2`
  - `PNPM_VERSION=12.16.1`
  - `NODE_VERSION=26`

- [ ] **Tests schreiben:**
  - `test_pinned_allowed_ok`
  - `test_tag_not_pinned`, mit `uses: actions/checkout@v4`
  - `test_unknown_action`
  - `test_lgpl_rejected`: Allowlist-Zeile `Swatinem/rust-cache LGPL-3.0`. Erwartet `license LGPL-3.0 not allowed`.
  - `test_local_action_ok`, mit `uses: ./.github/x`
  - Die erlaubten Lizenzen liest das Skript aus `deny.toml` (`allow = [...]`). Keine zweite Liste.
- [ ] **Rot, umsetzen, grün.**
- [ ] **`gate.sh`:** Mit `G4_GATE_HERMETIC=1` nach `g4emu unit` `record`en und `exit 0`. `print_table` schreibt dann `Gate (hermetisch): ok (<s> s)`. Ohne den Schalter bleibt alles gleich.
- [ ] **`ci.yml`** nach Spezifikation §3.2:
  - Für `gate-macos` ist `rustup show` der erste Rust-Schritt (er installiert die Toolchain aus `rust-toolchain.toml`), danach `rustup component add rustfmt clippy`.
  - `gate-windows` hat die Bedingung `github.event_name != 'pull_request' || contains(github.event.pull_request.labels.*.name, 'windows')`. Den Inhalt liefert Task 4.
  - Die SHAs der Actions holt der Agent mit `gh api repos/<o>/<r>/git/ref/tags/<tag>`. Bei einem annotierten Tag zeigt die Ref auf ein Tag-Objekt; dann `gh api repos/<o>/<r>/git/tags/<sha>`, um den Commit-SHA zu bekommen.
- [ ] **Lokal grün:** `G4_GATE_HERMETIC=1 bash scripts/gate.sh`. Die Tabelle kommt nach `target/agent/release-ci/hermetic-local.txt`.
- [ ] **Commit:** `ci: hermetic gate switch, actions audit and ci workflow`
- [ ] **Erster Push** (nach Task 1, Historie sauber):
  - `git push -u origin release-ci`.
  - `gh pr create --draft --base main --title "Release und CI" --body …`. Der Body endet mit der Zeile 🤖 Generated with [Claude Code](https://claude.com/claude-code).
  - Erwartet wird ein grüner `gate-macos`, mit gemessenen Minuten kalt.

### Task 4: Windows-Machbarkeit (Spezifikation §6.1)

**Dateien:** `.github/workflows/ci.yml` (Job `gate-windows`), eventuell `.cargo/config.toml`, eventuell Kernel-Code mit Windows-Fehlern

- [ ] Das Label `windows` auf den PR setzen und `gate-windows` mit `cargo build -p studio` als erstem Schritt fahren. Dabei MSVC versuchen: MinGW-`gcc` und `make` liegen in `C:\mingw64\bin` oder kommen aus `msys2/setup-msys2` mit `mingw-w64-x86_64-gcc make`, und der Pfad steht vor `PATH`.
- [ ] Scheitert der Link mit MSVC, `x86_64-pc-windows-gnu` versuchen: `rustup target add`, dann `--target` und `CARGO_BUILD_TARGET` im Job.
- [ ] Wenn `CFLAGS` unter Windows stört, gilt es nur noch für macOS: `[env]` entfällt aus `.cargo/config.toml`, und der Wert wird nur unter macOS gesetzt, in `with-rust.sh`, `gate.sh`, `ci.yml` und `release.yml`. Unter macOS muss es danach weiter grün sein.
- [ ] Danach `cargo clippy` und `cargo test` wie in Spezifikation §3.2. Fehler nur unter Windows beheben, jeweils mit einem Test, der unter macOS dieselbe Logik prüft, wo das geht.
- [ ] **Entscheidungsregel:** Bauen weder MSVC noch GNU, gilt: **Stopp**. Der Befund kommt nach `target/agent/release-ci/windows-feasibility.md` und geht an den Orchestrator. Die Tasks 5 bis 8 laufen dann nur für macOS.
- [ ] Das gewählte Ziel und den Grund in die Notiz schreiben. Sie fließt später in `docs/release.md` ein.
- [ ] **Commit:** `ci(windows): build and test on windows-latest (<ziel>)`

### Task 5: Sidecar-Suche, Platzhalter, Tauri-Konfiguration je Plattform

**Dateien:**
- Ändern:
  - `apps/studio/src-tauri/src/commands/models.rs` (`find_apicula`);
  - `apps/studio/src-tauri/src/lsp.rs` (`find_rotom_lsp`);
  - `apps/studio/src-tauri/build.rs`, `tauri.conf.json`, `.gitignore`.
- Neu: `apps/studio/src-tauri/tauri.macos.conf.json`, `apps/studio/src-tauri/tauri.windows.conf.json`
- Neu: `apps/studio/src-tauri/src/sidecar.rs`, ein gemeinsamer Helfer, weil beide Suchen dieselbe Logik haben.

**Schnittstellen:**
- Erzeugt: `pub(crate) fn sidecar_name(stem: &str) -> String`, also `format!("{stem}{}", std::env::consts::EXE_SUFFIX)`.
- Erzeugt: `pub(crate) fn find_sidecar(env_var: &str, stem: &str, debug_rel: &str) -> Option<PathBuf>`. Die Reihenfolge ist `env_var`, dann neben dem Binary, dann nur im Debug-Build `CARGO_MANIFEST_DIR/../../../<debug_rel><EXE_SUFFIX>`. Eine leere Datei zählt nicht.
- `find_apicula` und `find_rotom_lsp` rufen `find_sidecar` auf. Ihre Signaturen bleiben.

- [ ] **Tests** in `sidecar.rs`:
  - `sidecar_name_has_exe_suffix_on_windows`, mit `assert_eq!(sidecar_name("apicula"), format!("apicula{}", EXE_SUFFIX))`.
  - `placeholder_is_not_found`: In einem `tempdir` liegt eine leere `apicula<EXE_SUFFIX>`. Erwartet `None`, über eine interne Variante `find_in(&[dir])`.
  - `env_wins_even_if_missing`.
- [ ] **Rot, umsetzen, grün:** `cargo test -p studio sidecar`.
- [ ] **`build.rs`:** Platzhalter `binaries/<stem>-<TARGET><.exe wenn target_os windows>`, dazu je nach `CARGO_CFG_TARGET_OS` entweder `frameworks/libnitroarc_ffi.dylib` oder `windows/nitroarc_ffi.dll`.
- [ ] **Konfiguration aufteilen:**
  - `bundle.macOS` und `targets: ["app"]` kommen nach `tauri.macos.conf.json`.
  - `targets: ["nsis"]`, `resources: {"windows/nitroarc_ffi.dll": "nitroarc_ffi.dll"}` und `windows.nsis.installMode: "currentUser"` kommen nach `tauri.windows.conf.json`.
  - Die Lizenz-Ressourcen aus Task 7 stehen in `tauri.conf.json`.
  - `.gitignore` bekommt `apps/studio/src-tauri/windows/`.
- [ ] **Pfade unter Windows durchsuchen:** feste `"/"`-Joins, `cfg!(target_os = "macos")` ohne anderen Zweig und den Start von melonDS (`crates/g4-kernel/src/run.rs`). Die Funde werden behoben oder in der Windows-Notiz als „bekannt, Hand“ geführt.
- [ ] **Prüfen:**
  - `pnpm bundle` lokal ergibt weiter `bundle ok` (`check-bundle.sh`).
  - `cargo test --workspace` ist grün.
  - Unter Windows im nächsten PR-Lauf ist es ebenfalls grün.
- [ ] **Commit:** `feat(studio): sidecar lookup with exe suffix, tauri config per platform`

### Task 6: `studio doctor`, synthetisches Projekt, Rauchtest

**Dateien:**
- Neu: `apps/studio/src-tauri/src/doctor.rs`, `crates/g4-kernel/examples/make_synthetic.rs` (mit `required-features = ["test-support"]` in `[[example]]`), `scripts/smoke-packaged.sh`, `scripts/tests/smoke-packaged.test.sh`
- Ändern: `apps/studio/src-tauri/src/main.rs`, `apps/studio/src-tauri/src/lib.rs`

**Schnittstellen:**
- Erzeugt: `pub fn doctor::main(args: Vec<OsString>) -> ExitCode`. `main.rs` verzweigt bei `doctor` wie bei `mcp`, ohne Tauri.
- Erzeugt: `pub struct DoctorReport { version: String, binary: PathBuf, apicula: Probe, rotom_lsp: Probe, git: Option<PathBuf>, melonds: Option<PathBuf>, nitroarc: bool }`, `#[derive(Serialize)]`, und `enum Probe { Missing, Found { path: PathBuf, starts: bool, detail: String } }`.
- Ausgabe ohne `--json`:
  - eine Zeile je Werkzeug, etwa `apicula: <pfad> (startet)` oder `rotom-lsp: fehlt`;
  - die Schlusszeile `doctor: ok` beziehungsweise `doctor: Sidecar fehlt oder startet nicht`.
- Zum Status: Er ist 0, wenn `apicula` und `rotom_lsp` den Zustand `Found { starts: true }` haben. Sonst ist er 1.
- Erzeugt: `make_synthetic <ordner>` schreibt `test_support::synthetic_project()` in den Ordner und gibt den Pfad aus.
- Erzeugt: `smoke-packaged.sh <studio-binary> <projekt-ordner>`. Es endet mit `smoke ok` und Status 0.

- [ ] **Tests** (`doctor.rs`):
  - `missing_sidecars_exit_1`: Ein Binary-Ordner ohne Sidecars ergibt `Probe::Missing` und Status 1. Der Ordner wird als Parameter an die interne Funktion `probe_all(exe_dir: &Path) -> DoctorReport` gegeben.
  - `placeholder_counts_as_missing`.
  - `lsp_handshake_detects_dead_server`: Ein Skript, das sofort endet, ergibt `starts: false`. Unter Windows wird dieser Test mit `#[cfg(unix)]` ausgelassen.
  - `json_shape`: Die Schlüssel `version`, `apicula`, `rotom_lsp`, `git`, `melonds` und `nitroarc` sind vorhanden.
- [ ] **Umsetzen:**
  - Der LSP-Handshake nutzt `lsp::frame`: `initialize` mit `rootUri: null`, `shutdown`, `exit` und ein Timeout von 10 s.
  - apicula wird mit `--help` und Timeout 10 s gestartet.
  - nitroarc: ein Aufruf in uxie, der ohne Datei geht, etwa eine Versions- oder Init-Funktion. Gibt es keine, prüft er einen leeren NARC-Puffer und erwartet den Fehler „kein NARC“ statt eines Ladefehlers.
  - Die Suche nach melonDS kommt aus dem vorhandenen Kernel-Code; nicht neu erfinden.
- [ ] **`smoke-packaged.sh`:**
  - Es startet sich selbst neu unter `env -i HOME="$tmp/home" TMPDIR="$tmp/tmp" PATH=/usr/bin:/bin`, unter Windows `PATH=/c/Windows/System32:/usr/bin`, und zwar im Ordner `$tmp`.
  - Dann `doctor`. Danach `studio mcp --project <ordner>` mit einem Python-Client (`python3 -I`, nur Standardbibliothek, JSON-RPC über stdio): `initialize`, `tools/list`, eine Kollision ändern, `save` und, wenn das synthetische Projekt baubar ist, `build_rom`.
  - Die genauen Namen der Werkzeuge stehen im MCP-Katalog (`apps/studio/src-tauri/src/mcp/catalog.rs`) und in `tests/mcp_stdio.rs`. Ob das synthetische Projekt baubar ist, zeigt der vorhandene `mcp_stdio`-Test.
- [ ] **Test** `smoke-packaged.test.sh`:
  - `test_env_is_clean`: Mit `G4_APICULA=/nope` exportiert und einem Fake-Binary, das `env` ausgibt, kommt kein `G4_` in der Ausgabe vor.
  - `test_path_with_space`: Das Binary liegt unter `Gen4 Studio.app/`.
- [ ] **Lokal:** `pnpm bundle`, dann `cargo run -p g4-kernel --example make_synthetic --features test-support -- "$tmp/syn"` und `bash scripts/smoke-packaged.sh "target/release/bundle/macos/Gen4 Studio.app/Contents/MacOS/studio" "$tmp/syn"`. Erwartet `smoke ok`.
- [ ] **Commit:** `feat(studio): studio doctor and packaged smoke test`

### Task 7: Bundle-Vorbereitung für beide Plattformen, Lizenzhinweise, Windows-Prüfung

**Dateien:**
- Ändern: `apps/studio/scripts/prepare-bundle.sh`, `apps/studio/scripts/with-rust.sh`, `apps/studio/scripts/check-bundle.sh`
- Neu: `apps/studio/scripts/check-bundle-windows.sh`, `apps/studio/scripts/third-party-notices.sh`, `about.toml`, `about.hbs`, `apps/studio/src-tauri/licenses/`
- Ändern: `docs/dev-setup.md`, nur der apicula-Befehl liest jetzt `tool-versions.env`.

**Schnittstellen:**
- Verwendet: `scripts/tool-versions.env` aus Task 3 und das Windows-Ziel aus Task 4.
- Erzeugt: `third-party-notices.sh <out>` schreibt `THIRD-PARTY-NOTICES.txt` aus `cargo about generate about.hbs`, angehängt kommt `pnpm licenses list --prod`, beides Text. `prepare-bundle.sh` ruft es auf, Ziel ist `apps/studio/src-tauri/licenses/THIRD-PARTY-NOTICES.txt` (gitignored, erzeugt). Die übrigen Dateien unter `licenses/` (`nitroarc-LGPL-3.0.txt`, `nitroarc-GPL-3.0.txt`, `nitroarc-AUSTAUSCH.txt`, `apicula-LICENSE.txt`, `rotom-LICENSE.txt`) sind eingecheckt. Ihre Texte kommen aus den Quellen in `~/.cargo/registry/src/*/uxie-0.9.7/nitroarc/COPYING*` und den Repos von apicula und rotom.
- `tauri.conf.json` erhält `bundle.resources: {"licenses/": "licenses/"}`.

- [ ] **`prepare-bundle.sh`:**
  - Am Anfang wird `tool-versions.env` eingelesen. apicula wird mit `--rev $APICULA_REV` gebaut, wenn `tools/apicula/bin/apicula<exe>` fehlt; rotom bleibt wie bisher.
  - Dazu der Zweig `case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*)`, mit den Zielen aus Spezifikation §6.2 ohne `otool`, `install_name_tool` und `codesign`. Die DLL kommt aus `target/release/nitroarc_ffi.dll` oder, unter dem GNU-Ziel, aus `target/<triple>/release/`.
  - Der macOS-Zweig bleibt unverändert.
- [ ] **`with-rust.sh`:** unter Windows die MinGW- und Cargo-Pfade aus Task 4. Den macOS-Zweig nicht verändern.
- [ ] **`check-bundle.sh`:** zusätzlich `Contents/Resources/licenses/THIRD-PARTY-NOTICES.txt`, nicht leer und mit `nitroarc` oder `LGPL`.
- [ ] **`check-bundle-windows.sh <setup.exe>`** nach Spezifikation §6.3. Er endet mit `bundle ok (windows)` und Status 0.
- [ ] **Lizenzen prüfen:** `cargo deny check licenses` ist unverändert grün. Die Lizenz von `cargo-about` steht in `.github/actions-allowlist.txt` nicht, weil es keine Action ist, sondern in `docs/release.md` (Task 9).
- [ ] **Lokal:** `pnpm bundle` und `check-bundle.sh`, Erwartung `bundle ok`. Windows folgt in Task 8.
- [ ] **Commit:** `feat(bundle): windows branch, tool versions, third-party notices`

### Task 8: `release.yml` und Probelauf

**Dateien:** Neu `.github/workflows/release.yml`

**Schnittstellen:**
- Verwendet: `version.sh check` (Task 2), `prepare-bundle.sh` und `check-bundle*.sh` (Task 7), `smoke-packaged.sh` und `make_synthetic` (Task 6), `check-no-rom-data.sh` (Task 1).
- Erzeugt die Artefakte `macos-dmg` und `windows-nsis` mit den Dateinamen aus Spezifikation §5.

- [ ] **`release.yml`** nach Spezifikation §5:
  - **Auslöser:** `push: tags: ['v*']`, `workflow_dispatch: inputs: dry_run (boolean, default true)` und `pull_request: types: [labeled, synchronize]`.
  - **Bedingung auf jedem Job:** `github.event_name != 'pull_request' || contains(github.event.pull_request.labels.*.name, 'release-dry-run')`. `DRY_RUN` ist `true` bei einem PR und bei `inputs.dry_run`.
  - **Cache der Werkzeuge:** Pfad `tools/`, Schlüssel `tools-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('scripts/tool-versions.env') }}`.
  - **DMG:** `CI=true pnpm bundle -- --bundles app,dmg`. Scheitert Tauris DMG-Skript, gilt der Rückfall `hdiutil create -volname "Gen4 Studio" -srcfolder "<app>" -ov -format UDZO <dmg>`. Die Wahl kommt in die Windows- und Release-Notiz.
  - **Umbenennen** auf `Gen4-Studio_<v>_aarch64.dmg` und `Gen4-Studio_<v>_x64-setup.exe`.
  - **`publish`:** `sha256sum` beziehungsweise `shasum -a 256` erzeugt `SHA256SUMS.txt`. Die Notizen schneidet `awk` aus `CHANGELOG.md`, und zwar den Abschnitt der Basisversion.
  - **Signierschritt:** `sign-notarize` mit `if: false`, mit den Kommentaren aus Spezifikation §5.
- [ ] **`check-actions.sh`** ist grün.
- [ ] **Probelauf:** das Label `release-dry-run` auf den PR setzen. Beide Bundle-Jobs grün: `check-bundle`, `check-bundle-windows`, `no rom data` und `smoke ok`. Die Minuten kommen ins Protokoll. Für den NSIS-Fall der Version (Spezifikation §6.4) wird `v0.1.0-rc.1` vorweggenommen: `version.sh set 0.1.0-rc.1` nur lokal und nicht committet, dann `pnpm bundle` unter Windows im selben Lauf. Dafür hat der Workflow beim PR den Eingang `G4_PROBE_VERSION`. Das Ergebnis kommt in die Notiz.
- [ ] **Artefakte prüfen:** `gh run download` der Artefakte nach `target/agent/release-ci/dry-run/`, dann `check-no-rom-data.sh` darüber. Erwartet `ok`.
- [ ] **Commit:** `ci: release workflow with draft release, dry run via label`

### Task 9: Dokumentation

**Dateien:**
- Ändern: `docs/release.md`, `docs/dev-setup.md` (rund um Zeile 718), `docs/ROADMAP.md` (Querschnitt, Zeile 400 f., und eine neue Zeile „Release und CI“)
- Neu: `docs/release-ci-gate.md` als Gerüst mit G1 bis G8, Zeiten und Hand. Die Ergebnisse trägt der Orchestrator ein.

- [ ] Die Inhalte stehen in Spezifikation §11. In `docs/release.md` kommen dazu:
  - das gewählte Windows-Ziel samt Grund;
  - die Entscheidungen zu DMG und NSIS-Version aus den Notizen;
  - die Lizenz von cargo-about;
  - wie man nitroarc austauscht: die Datei in `Contents/Frameworks/` beziehungsweise neben `studio.exe` ersetzen, unter macOS danach `codesign --force --sign -`.
- [ ] Kein Satz behauptet mehr „kein Remote“ oder „keine CI“. Prüfen mit `grep -rn "kein Remote\|keine CI" docs/`. Erlaubt sind die historischen Zeilen in Phase 2 und 3b.1 der Roadmap.
- [ ] **Commit:** `docs(release-ci): release, dev setup, roadmap, gate skeleton`

### Task 10: Übergabe an den Orchestrator (nicht mergen)

- [ ] `G4_GATE_HERMETIC=1 bash scripts/gate.sh` ist lokal grün. Der letzte PR-Lauf (`ci` und `release-dry-run`) ist grün.
- [ ] Trailer zählen: `git log --format='%(trailers:only,unfold)' main..release-ci | grep -c 'Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>'` muss gleich `git rev-list --count main..release-ci` sein.
- [ ] **Bericht** nach `target/agent/release-ci/report.md`:
  - Commits, gemessene Minuten, Windows-Ziel und Entscheidungen;
  - offene Punkte;
  - PR-URL und die Links der Läufe.

Danach übernimmt der Orchestrator. Er hat diese Aufgaben:

- Merge nach `main` und Push (G1), `ci-rot` (G2), dann der Tag `v0.1.0-rc.1` (G3);
- G5 lokal mit dem heruntergeladenen DMG und einer Kopie von Platin DE;
- G6 und G7 prüfen;
- das volle `scripts/gate.sh` (G8);
- `docs/release-ci-gate.md` und die Roadmap-Zeile mit Stand.
