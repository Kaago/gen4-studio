# Phase 3g.1 „Neues Spiel aus ROM“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aus einer eigenen `.nds` ein neues Projekt anlegen: `inspect_rom` erkennt Spiel, Sprache und Freigabe, `create_project` entpackt mit Fortschritt und schreibt die Marke `.g4/base.json`; der Start-Screen hat den Hauptknopf „Neues Spiel aus ROM…“ und nimmt eine `.nds` per Drag-and-drop; der Assistent im Scharnier führt durch ROM, Vorlage, Name und Ziel, Zusammenfassung, Anlegen und „Öffnen“. Ein frisch angelegtes Original-Projekt baut byte-gleich zur Eingabe-ROM.

**Architecture:** Kernel-Modul `crates/g4-kernel/src/start.rs` (Kopf lesen, SHA-1, Freigabe, Ziel prüfen, anlegen über das vorhandene `extract_rom` in einen Temp-Ordner, dann umbenennen) plus Metadatei `metadata/base_roms.yaml`; `build()` setzt nach dem Bauen Secure-Area-CRC und Header-CRC aus `.g4/base.json` wieder ein, wenn die Secure Area unverändert ist. App-Befehle in `apps/studio/src-tauri/src/commands/start.rs`, Brücke mit `Roots::check_new_project`. Frontend in `apps/studio/src/project/` (eigener Ordner): API, Assistent, Start-Bereich mit eigenem kleinen Deck (Schau, Scharnier im Assistentenmodus, Hand) und Drop-Zone; `App.tsx` tauscht nur `StartPane` gegen `ProjectStart` und reicht „Öffnen“ durch.

**Tech Stack:** Rust (g4-kernel, ds-rom über `extract_rom`, `sha1`, `libc::statvfs`), Tauri 2 (`Channel` für den Fortschritt, `app.path().document_dir()`, `getCurrentWebview().onDragDropEvent`), React 19 + `ui/primitives`, vitest (jsdom, `createRoot` + `act`, kein testing-library).

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) Abschnitt „Phase 3g: Projektstart“ (3g.1, Gate, „Nicht in 3g“); [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) Tabelle „Projektstart“; [gutachten-c-projektstart-mcp.md](../../research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md) Teil A (A.2, A.5, A.6, A.7, A.8).

Stand 25.10.2026, Branch `phase-3g-projektstart` (Worktree `.claude/worktrees/phase-3g`). Pfade relativ zur Repo-Wurzel; Frontend relativ zu `apps/studio/src`.

## Befunde vor dem Plan (gemessen 25.10.2026 an Kopien unter `_testkopien/3g-probe`)

- `g4k extract` + `g4k build` auf Platin DE und HeartGold DE: die ROM ist **nicht** byte-gleich, es weichen genau zwei Stellen ab: `0x6C` (Secure-Area-CRC16, ds-rom schreibt ohne Blowfish-Schlüssel `0000`) und `0x15E` (Header-CRC16, hängt davon ab). Die Bytes `0x4000..0x8000` (Secure Area) sind gleich. Die Header-CRC ist CRC-16/MODBUS (Init `0xFFFF`, Polynom `0xA001` reflektiert) über `0x000..0x15E`; nachgerechnet für beide Originale.
- SHA-1 der drei Testkopien: Platin DE (CPUD) `d40f19ac24d67c721a4ab25e1c013154443d75b4`, HeartGold DE (IPKD) `13b6853aa93121069df1d17446d3ecb36872aecf`, HeartGold US (IPKE) `4fcded0e2713dc03929845de631d0932ea2b5a37`. Ein Abgleich mit No-Intro ist nicht gemacht; die Tabelle nennt sie deshalb „bekannte Basis (Testprojekt)“, nicht „sauberer Dump“.
- Entpacken dauert im Debug-Build rund 1 s (Platin), Bauen 0,2 s (Platin) bzw. 3,7 s (HeartGold).
- Die Brücke behandelt jede Pfadkomponente mit `_DSPRE_contents` als Original und sperrt sie (`bridge/roots.rs`).

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Byte-gleicher Neubau | `create_project` speichert `secure_area_crc` (u16 aus `0x6C`) und `secure_area_sha1` (SHA-1 von `0x4000..0x8000`) in `.g4/base.json`; `build()` schreibt nach `raw_rom.save` die CRC zurück und rechnet die Header-CRC neu, **nur** wenn die gebaute Secure Area denselben SHA-1 hat. Ohne Marke (DSPRE-Ordner) ändert sich nichts. | Gate verlangt byte-gleich; ds-rom kann die CRC ohne ARM7-BIOS nicht rechnen; bei veränderter Secure Area bleibt `0000` wie heute |
| Dump-Status | `metadata/base_roms.yaml` mit den drei SHA-1 oben (`label`, `game_code`, `source: "Testprojekt"`); `RomInfo.known_base: Option<String>` (Label) | Gutachten A.2; keine ungeprüften No-Intro-Behauptungen |
| Verschlüsselte ROM | `inspect_rom` erkennt sie nicht eigens; `create_project` meldet den Fehler von `extract_rom` („encrypted ROMs are not supported“) | YAGNI, Dumps gängiger Werkzeuge sind entschlüsselt |
| Atomar | `create_project` entpackt nach `<parent>/.<name>.g4-new`, schreibt die Marke, prüft und benennt erst dann nach `dest` um; bei Fehler wird nur dieser Temp-Ordner gelöscht. Ein vorhandenes leeres `dest` wird vorher mit `remove_dir` (nur leer) entfernt. Fehlende Eltern legt der Kernel an (`create_dir_all`). | kein halbes Projekt; nie fremde Dateien löschen |
| Vorlagen in 3g.1 | `Template::{Original, DataOnly}`; `EmptyRegion` kommt in 3g.2 (die Karte steht im Assistenten schon da, gesperrt mit „Kommt mit 3g.2“, wird in 3g.2 freigeschaltet). `DataOnly` ändert nichts am Projekt, nur `template: "data_only"` in der Marke; nach „Öffnen“ startet die App im Bereich Pokémon. | Entscheidung „Nur Daten (eine Voreinstellung der Oberfläche)“ |
| Fortschritt | Tauri: `Channel<CreateProgressDto>` wie `adopt_scripts`; Brücke: ohne Live-Fortschritt, die Antwort trägt `phases: [{phase, ms}]`, die Oberfläche hakt danach alle Phasen ab | ein Muster, das es schon gibt; 1 s Dauer |
| Weg ohne Suffix-Sperre (Brücke) | Der Zielname ist frei; der Assistent hat den Schalter „Endung _DSPRE_contents (für DSPRE)“. `default_project_parent` liefert in Tauri `~/Documents/Gen4 Studio` mit Schalter an, in der Brücke die erste erlaubte Wurzel (`_testkopien`) mit Schalter aus. Die Brücke lehnt ein Ziel mit der Endung ab („Originale (_DSPRE_contents) sind gesperrt: … Im Browser-Modus ohne die Endung anlegen.“). CLI: `g4k create` nimmt `dest` wörtlich. | Gutachten A.5 („Agenten-Tests legen in `_testkopien` ohne diese Endung an … ein freier Name“) |
| Ziel in der Brücke | ROM über `Roots::read_path` (Wurzeln oder `G4_IMPORT_ROOT`), Ziel über neues `Roots::check_new_project(dest)`: kanonischer Elternordner existiert und liegt unter einer Wurzel, letzter Name ohne `_DSPRE_contents`, nicht `_saves`, Ziel fehlt oder ist ein leerer Ordner | wie `build_rom` (Gutachten A.6) |
| Freier Platz | `needed = rom_size * 5 / 2`; `free` per `libc::statvfs` am nächsten existierenden Vorfahren (nur unix, sonst `None`); zu wenig Platz sperrt „Weiter“ | Gutachten A.5 |
| Start ohne Projekt | Der Assistent läuft im Bereich Start in einem eigenen kleinen Deck (`ProjectStart`): Schau (Vorschau der Wahl), `Hinge` im Assistentenmodus mit `[data-wizard-bar]`, Hand mit `[data-wizard-hand]`; `WizardFrame` portiert hinein wie bei den Welt-Assistenten. Keine Änderung an `useWizards`/`WizardHost`. | Welt-Deck ohne Projekt wäre leer; `findSlots` nimmt die ersten Slots im Dokument |
| Hauptknopf | „Neues Spiel aus ROM…“ wird die primäre Taste auf Start, „Projekt öffnen“ (⌘O) sekundär; darunter „Du brauchst deine eigene ROM-Datei (.nds).“ und „Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM.“; dazu Palette-Befehl „Neues Spiel aus ROM…“ | Roadmap; höchstens eine primäre Taste je Ansicht |
| Ergebnis-Taste | `WizardFrame` bekommt `showLabel?: string` (Standard „Zeigen“); der Projekt-Assistent setzt „Öffnen“ und kein `onPlay` | kleinste Änderung am Rahmen |
| Drag-and-drop | Tauri: `getCurrentWebview().onDragDropEvent` (Pfade), nur solange Start sichtbar ist; Browser: HTML5 `drop` auf der Start-Fläche, Pfad aus `text/uri-list` (`file://`) oder `text/plain` (absoluter Pfad auf `.nds`); eine Datei ohne Pfad öffnet den Assistenten im Schritt ROM mit dem Hinweis „Im Browser-Modus kennt die Seite den Pfad der Datei nicht. Gib ihn ein.“ Ein gültiger Pfad springt nach erfolgreichem `inspect_rom` zu Schritt 2. | Gutachten S3; im Browser liefert ein Datei-Drop keinen Pfad |

## Global Constraints

- Nie ROM-Daten, ROM-Dateien oder feste ROM-Pfade ins Repo; Tests mit ROM nur gated über `G4_TEST_ROM` (wie `extract_rom_creates_openable_project`). Nicht-gated Tests bauen sich eine synthetische ROM aus `test_support::synthetic_project()` mit `build()`.
- Originale unter `Roms/` werden nie geschrieben und nie als Projekt geöffnet; Test-Eingaben sind Kopien unter `Roms/_testkopien/3g-*`, neue Projekte nur dort.
- Diamant/Perl (ADA*, APA*): gesperrt mit genau `"Diamant/Perl: ohne Testprojekt nicht freigegeben"` (wie `DP_LOCKED`); unbekannt: `"Kein Gen-4-Spiel erkannt"`. Freigegeben: CPUD, IPKD, IPKE; ohne Testprojekt: alle anderen CPU*, IPK*, IPG*.
- Neue Brückenbefehle `inspect_rom`, `default_project_parent`, `check_project_dest`, `create_project`: je ein Eintrag **am Ende** von `COMMANDS` und am Ende der `match`-Arme in `apps/studio/src-tauri/src/bridge/dispatch.rs` und am Ende von `generate_handler!` in `lib.rs`; nichts umsortieren (die 3h-Session verschiebt die Datei).
- Design-System: nur Bausteine aus `ui/primitives` und Tokens `--g4-*`, keine Hex-Farben, keine Inline-Styles außer `/* dyn */`; `design-system/README.md` nur am Ende ein Abschnitt „Nachtrag 3g“; die `.fig` bleibt unberührt.
- Vor jedem Commit grün: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (berührte Teile, mindestens die neuen Tests), `pnpm -C apps/studio vitest run`, `pnpm -C apps/studio build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`. `PATH="/opt/homebrew/opt/rustup/bin:$PATH"` setzen. In diesem Worktree nur auf Workspace-Ebene bauen und testen.
- Jeder Commit endet mit der Zeile `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (das Modell, das diese Phase umsetzt; normales `@` wie in der Git-Historie).

## Review Focus

- Ziel ist ein vorhandener, nicht leerer Ordner (oder eine Datei): `create_project` lehnt ab, bevor es etwas schreibt, und der Ordner ist danach unverändert (Task 2).
- Entpacken scheitert mittendrin (kaputte ROM, zu kleine Datei): kein halbes Projekt unter `dest`, kein Temp-Ordner bleibt liegen (Task 2).
- Ein frisch angelegtes Projekt wird bearbeitet, so dass sich die Secure Area ändert (oder `.g4/base.json` ist kaputt): `build()` lässt `0x6C` auf `0000` und scheitert nicht (Task 2).
- Brücke: Ziel mit `_DSPRE_contents`, Ziel außerhalb `_testkopien`, ROM aus `_saves` oder über einen Symlink nach außen: abgelehnt mit dem Satz der Sperre (Task 3).
- Der Nutzer zieht eine Nicht-`.nds`-Datei oder zwei Dateien auf Start, oder zieht während eines laufenden Anlegens: nichts passiert außer einem Hinweis-Toast „Nur eine .nds-Datei ablegen.“; ein laufender Assistent wird nicht ersetzt (Task 5).

---

### Task 1: `inspect_rom` im Kernel

**Files:**
- Create: `crates/g4-kernel/src/start.rs` (Modul `start`, in `lib.rs` als `pub mod start;` eintragen), `crates/g4-kernel/metadata/base_roms.yaml`, `crates/g4-kernel/tests/project_start.rs`
- Modify: `crates/g4-kernel/src/lib.rs`, `crates/g4-kernel/src/test_support.rs` (neu: `pub fn synthetic_rom() -> (tempfile::TempDir, PathBuf)`: baut `synthetic_project()` mit `build()` zu `<tmp>/synthetic.nds`; `pub fn with_game_code(rom: &Path, code: &str)`: schreibt `code` nach `0x0C` und rechnet die Header-CRC bei `0x15E` neu)

**Interfaces:**
- Produces:
  - `pub struct RomInfo { pub title: String, pub game_code: String, pub family: Option<Family>, pub language: Option<Language>, pub revision: u8, pub size: u64, pub sha1: String, pub known_base: Option<String>, pub support: Support }` (`Serialize`, snake_case)
  - `pub enum Support { Verified, Untested, Locked { reason: String }, Unknown { reason: String } }` (`#[serde(tag = "kind", rename_all = "snake_case")]`)
  - `pub fn inspect_rom(path: &Path) -> Result<RomInfo, KernelError>` (Titel `0x00..0x0C` ohne Nullen, Code `0x0C..0x10`, Revision `0x1E`; SHA-1 der ganzen Datei gestreamt; Datei unter `0x200` Byte → `KernelError::Refused("too small for a DS ROM: N bytes")`)
  - `pub fn header_crc16(bytes: &[u8]) -> u16` (CRC-16/MODBUS), `pub const DP_LOCKED: &str`, `pub const NOT_GEN4: &str = "Kein Gen-4-Spiel erkannt"`
- Consumes: `g4_formats::game::{Family, Language}`

- [ ] **Step 1: Failing tests** in `start.rs` (`#[cfg(test)]`, synthetische Köpfe aus Bytes) und `tests/project_start.rs`:
  - `header_crc16_matches_the_modbus_check_value`: `header_crc16(b"123456789") == 0x4B37`.
  - `support_follows_the_game_code`: CPUD/IPKD/IPKE → `Verified`; CPUE, IPGD, IPKF → `Untested`; ADAD, APAE → `Locked { reason: DP_LOCKED }`; `ABCD`, `\0\0\0\0` → `Unknown { reason: NOT_GEN4 }`.
  - `inspect_reads_title_code_revision_and_sha1` (synthetische ROM aus `test_support::synthetic_rom()`): `game_code == "CPUE"`, `title == "POKEMON PL"`, `sha1` = SHA-1 der Datei (40 Hex klein), `known_base == None`, `support == Untested`.
  - `inspect_refuses_a_tiny_file`: 100 Byte → `Err(Refused)` mit „too small“.
  - `inspect_locks_diamond`: `with_game_code(rom, "ADAD")` → `Locked`.
  - gated `project_start_inspects_the_test_rom` (`G4_TEST_ROM`, sonst `eprintln!("skipped: G4_TEST_ROM not set")`): Code = Bytes `0x0C..0x10`, `support == Verified`, `known_base.is_some()`.
- [ ] **Step 2: Run** `cargo test --workspace --all-features project_start_ start::` → FAIL (Modul fehlt).
- [ ] **Step 3: Implement** `start.rs` und `base_roms.yaml` (Einträge `{sha1, game_code, label: "Platin DE (Testprojekt)" | "HeartGold DE (Testprojekt)" | "HeartGold US (Testprojekt)", source: "Testprojekt"}`, per `include_str!` geladen wie die übrigen Metadateien) sowie die zwei `test_support`-Helfer.
- [ ] **Step 4: Run** dieselben Tests → PASS; gated einmal mit `G4_TEST_ROM=/Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/3g-probe/pt.nds`.
- [ ] **Step 5: Commit** `feat(kernel): inspect_rom reads game code, language, release level and SHA-1 of a .nds`

---

### Task 2: `create_project`, Marke `.g4/base.json`, byte-gleicher Neubau, `g4k`

**Files:**
- Modify: `crates/g4-kernel/src/start.rs`, `crates/g4-kernel/src/build.rs` (nach `raw_rom.save(out_rom)` ein Aufruf `crate::start::restore_header_crcs(&project.root, out_rom)?`), `crates/g4-kernel/src/bin/g4k.rs` (Unterbefehle `inspect-rom <rom>` → JSON von `RomInfo`; `create <rom> <dest> [--template original|data-only]` → druckt `root` und die Phasen), `crates/g4-kernel/tests/project_start.rs`, `crates/g4-kernel/tests/cli.rs`, `scripts/gate.sh` (nach der Schleife über die Projekte eine Schleife über `"$G4_ROMS_DIR"/*.nds`: `run_step "gated tests (project start)" "<Name>" env G4_TEST_ROM="$rom" G4_DSROM_CLI=… cargo test --workspace --all-features project_start_`; ohne `.nds` eine Zeile „skipped (keine ROM)“)
- Create: `docs/formats/project_base.md` (Marke `.g4/base.json`, `base_roms.yaml`, CRC-Wiederherstellung), Abschnitt „Projekt aus ROM anlegen (3g.1)“ in `docs/dev-setup.md` (g4k `inspect-rom`/`create`, gated Test, Gate-Schritt)

**Interfaces:**
- Consumes: Task 1 (`inspect_rom`, `Support`, `header_crc16`); `build::extract_rom(nds, dest) -> Result<Project, KernelError>`
- Produces:
  - `pub enum Template { Original, DataOnly }` (`serde` snake_case: `"original"`, `"data_only"`; 3g.2 ergänzt `EmptyRegion`)
  - `pub enum Phase { Read, Extract, Template, Check }` (`Serialize` snake_case) und `pub struct Progress { pub phase: Phase, pub done: u32, pub total: u32 }`
  - `pub struct BaseMark { pub version: u32 /* 1 */, pub origin: String /* "extracted" */, pub sha1: String, pub game_code: String, pub title: String, pub revision: u8, pub size: u64, pub template: Template, pub created: String /* RFC 3339 UTC, ohne Zusatz-Crate */, pub studio_version: String /* env!("CARGO_PKG_VERSION") */, pub secure_area_crc: u16, pub secure_area_sha1: String }` (`Serialize + Deserialize`); `pub fn read_base_mark(root: &Path) -> Option<BaseMark>` (fehlend oder kaputt → `None`)
  - `pub enum DestState { Missing, Empty, NotEmpty, NotADirectory }` und `pub struct DestCheck { pub state: DestState, pub free_bytes: Option<u64>, pub needed_bytes: u64, pub ok: bool }`; `pub fn check_destination(dest: &Path, rom_size: u64) -> DestCheck` (`ok` = Missing/Empty und `free_bytes.map_or(true, |f| f >= needed)`)
  - `pub struct Created { pub root: PathBuf, pub info: RomInfo, pub template: Template, pub phases: Vec<(Phase, u128 /* ms */)> }`
  - `pub fn create_project(rom: &Path, dest: &Path, template: Template, progress: &mut dyn FnMut(Progress)) -> Result<Created, KernelError>`: Read (`inspect_rom`; `Locked`/`Unknown` → `Refused(reason)`), Extract (`extract_rom` in den Temp-Ordner), Template (für `Original`/`DataOnly` nichts, `done = total = 0`), Check (`Project::open`, `header_count() > 0`, Marke schreiben, umbenennen). Jede Phase meldet zu Beginn `Progress{phase, done: 0, total}`.
  - `pub fn restore_header_crcs(root: &Path, rom: &Path) -> Result<bool, KernelError>` (true = zurückgeschrieben)

- [ ] **Step 1: Failing tests** (nicht gated, mit `synthetic_rom()`):
  - `create_project_writes_the_base_mark`: Marke mit `origin == "extracted"`, `sha1 == inspect.sha1`, `template == Original`, `version == 1`, `created` endet auf `Z`.
  - `create_project_reports_the_four_phases_in_order`: Callback sieht `Read, Extract, Template, Check`; `Created.phases.len() == 4`.
  - `create_then_build_is_byte_identical_to_the_input_rom`: `build(&Project::open(root), out)` → SHA-1 gleich der Eingabe (die synthetische ROM hat eine echte Secure Area? Falls `0x6C == 0` in der Eingabe, prüft der Test zusätzlich die Wiederherstellung mit `with_game_code` nicht; siehe nächster Test).
  - `restore_header_crcs_writes_the_crc_only_for_an_unchanged_secure_area`: Marke mit `secure_area_crc = 0x1234` und passendem SHA-1 → `0x6C..0x6E == 34 12`, `0x15E` = `header_crc16(rom[..0x15E])`; mit falschem `secure_area_sha1` → Datei unverändert, `Ok(false)`; ohne Marke → `Ok(false)`.
  - `create_project_refuses_a_non_empty_destination` (Datei `keep.txt` bleibt, kein `.<name>.g4-new`), `create_project_refuses_diamond` (Grund `DP_LOCKED`, `dest` fehlt danach), `create_project_cleans_up_after_a_broken_rom` (gültiger Kopf, abgeschnitten auf 0x4000 Byte → `Err`, weder `dest` noch Temp-Ordner existieren), `create_project_creates_missing_parents`, `create_project_reuses_an_empty_destination`.
  - `check_destination_states`: fehlt → `Missing, ok`; leer → `Empty, ok`; Datei → `NotADirectory, !ok`; nicht leer → `NotEmpty, !ok`; `needed_bytes == size * 5 / 2`.
  - `tests/cli.rs`: `g4k inspect-rom <synthetic>` druckt JSON mit `"game_code": "CPUE"`; `g4k create <synthetic> <tmp>/neu` legt `neu/.g4/base.json` an.
  - gated `project_start_create_then_build_matches_the_rom` (`G4_TEST_ROM`): anlegen in Tempdir, bauen, SHA-1 gleich `RomInfo.sha1`; dazu `Project::open(root).headers()` nicht leer.
- [ ] **Step 2: Run** `cargo test --workspace --all-features project_start start:: cli` → FAIL.
- [ ] **Step 3: Implement** in `start.rs`, `build.rs`, `g4k.rs`; `gate.sh`-Schleife; Doku.
- [ ] **Step 4: Run** dieselben Tests → PASS; gated für `_testkopien/3g-probe/pt.nds` und `hg.nds`; `bash -n scripts/gate.sh`.
- [ ] **Step 5: Commit** `feat(kernel): create_project extracts a ROM with progress and a base mark; rebuilds restore the secure area CRC`

---

### Task 3: App-Befehle und Brücke

**Files:**
- Create: `apps/studio/src-tauri/src/commands/start.rs` (in `commands/mod.rs` als `mod start; pub use start::*;`)
- Modify: `apps/studio/src-tauri/src/lib.rs` (vier Namen am Ende von `generate_handler!` und im `use commands::{…}`), `apps/studio/src-tauri/src/bridge/dispatch.rs` (vier Namen am Ende von `COMMANDS`, vier Arme am Ende des `match`, Testfälle in `every_command_accepts_the_keys_api_ts_sends` und eigene Tests), `apps/studio/src-tauri/src/bridge/roots.rs` (`check_new_project` + Tests)
- Create: `apps/studio/src/project/api.start.ts` (+ `api.start.test.ts`); in `api.ts` am Ende `export * from "./project/api.start";` wie bei `api.checks`

**Interfaces:**
- Consumes: Task 1/2 (`inspect_rom`, `check_destination`, `create_project`, `Template`, `Progress`)
- Produces (Rust, `*_impl` testbar ohne Tauri):
  - `inspect_rom_impl(path: &str) -> Result<RomInfo, CmdError>`
  - `default_project_parent_impl(documents: Option<PathBuf>) -> ProjectParentDto { parent: String, dspre_suffix: bool }` (Tauri: `<documents>/Gen4 Studio`, `true`; Brücke ruft es nicht, sondern antwortet mit der ersten Wurzel und `false`; `Roots` bekommt dafür `pub fn first(&self) -> &Path`)
  - `check_project_dest_impl(dest: &str, rom_size: u64) -> DestCheck`
  - `create_project_impl(rom: &str, dest: &str, template: Template, progress: &mut dyn FnMut(Progress)) -> Result<CreatedDto, CmdError>` mit `CreatedDto { root: String, game_code: String, template: Template, sha1: String, phases: Vec<PhaseTimeDto { phase: Phase, ms: u64 }> }`; öffnet das Projekt **nicht**
  - Tauri: `#[tauri::command] async fn create_project(rom: String, dest: String, template: Template, progress: Channel<Progress>)` über `spawn_blocking`; `#[tauri::command] fn default_project_parent(app: AppHandle)`; `inspect_rom`, `check_project_dest` synchron über `off_main`, falls vorhanden, sonst wie `run_checks`
  - Brücke: `"inspect_rom" { path }` → `roots.read_path`; `"default_project_parent" {}` → erste Wurzel, `dspreSuffix: false`; `"check_project_dest" { dest, romSize }` → zuerst `roots.check_new_project(dest)` (Fehler als `CmdError`), dann `check_project_dest_impl`; `"create_project" { rom, dest, template }` → `read_path(rom)`, `check_new_project(dest)`, Fortschritt verworfen
  - `Roots::check_new_project(&self, dest: &str) -> Result<PathBuf, CmdError>`: Elternordner muss existieren (kanonisch unter einer Wurzel, `check_canonical`), letzter Name endet nicht auf `_DSPRE_contents` (Meldung `"Originale (_DSPRE_contents) sind gesperrt: <dest>. Im Browser-Modus ohne die Endung anlegen."`), ist nicht `_saves`; existiert `dest`, muss es kanonisch unter der Wurzel bleiben (kein Symlink nach außen)
- Produces (TS, `project/api.start.ts`): Typen `RomInfo`, `Support` (`{kind: "verified"} | {kind: "untested"} | {kind: "locked"; reason} | {kind: "unknown"; reason}`), `Template = "original" | "data_only"`, `CreateProgress = {phase: "read"|"extract"|"template"|"check"; done: number; total: number}`, `DestCheck {state: "missing"|"empty"|"not_empty"|"not_a_directory"; free_bytes: number|null; needed_bytes: number; ok: boolean}`, `Created`; Funktionen `inspectRom(path)`, `defaultProjectParent()`, `checkProjectDest(dest, romSize)`, `createProject(rom, dest, template, onProgress)` (Tauri: `Channel`, Browser: ohne Fortschritt, danach `onProgress` je Phase mit `done = total` wie `adoptScripts`).

- [ ] **Step 1: Failing tests**:
  - `roots.rs`: `new_project_under_the_root_is_accepted`, `new_project_with_the_dspre_suffix_is_refused` (Meldung beginnt mit „Originale (_DSPRE_contents) sind gesperrt“ und enthält „ohne die Endung“), `new_project_outside_or_in_saves_is_refused`, `new_project_needs_an_existing_parent`.
  - `dispatch.rs`: `create_project_from_a_synthetic_rom_then_open_it` (ROM aus `synthetic_rom()` unter der Wurzel, `create_project` → `root`, dann `open_project` mit diesem Pfad → `game_code == "CPUE"`); `create_project_refuses_the_dspre_suffix`; `default_project_parent_is_the_first_root_without_suffix`; die vier Befehle in `every_command_accepts_the_keys_api_ts_sends` mit den Schlüsseln aus `api.start.ts` (`path`, `dest`, `romSize`, `rom`, `template`); `commands_match_the_tauri_handler` bleibt grün.
  - `api.start.test.ts`: `createProject` ruft in Tauri `invoke("create_project", {rom, dest, template, progress})` mit einem `Channel`; im Browser ohne `progress` und meldet danach vier Phasen mit `done === total`.
- [ ] **Step 2: Run** `cargo test --workspace --all-features -p studio roots:: dispatch::` (bzw. Workspace-Filter) und `pnpm -C apps/studio vitest run src/project` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS; `cargo clippy …`, `pnpm -C apps/studio build`.
- [ ] **Step 5: Commit** `feat(studio): inspect_rom, default_project_parent, check_project_dest and create_project in the app and the bridge`

---

### Task 4: Assistent „Neues Spiel aus ROM“

**Files:**
- Create: `apps/studio/src/project/NewProjectWizard.tsx`, `NewProjectWizard.module.css`, `NewProjectWizard.test.tsx`, `apps/studio/src/project/projectName.ts` (+ Test)
- Modify: `apps/studio/src/workbench/wizards/WizardFrame.tsx` (Prop `showLabel?: string`, Standard „Zeigen“; Test in `WizardFrame.test.tsx`)

**Interfaces:**
- Consumes: Task 3 (`api.start.ts`); `WizardFrame`, `WizardStep`, `DoneResult` aus `workbench/wizards`; `dialogs.openPath` (`platform/dialogs.ts`; für die ROM `openPath({directory: false, title: "ROM-Datei wählen"})`, in Tauri mit Filter `.nds`, dazu ggf. eine kleine Erweiterung `filters?` in `dialogs.ts`)
- Produces:
  - `export function NewProjectWizard(props: { initialRom?: string; initialNotice?: string; onCancel: () => void; onOpen: (created: Created) => void; onChoice?: (c: WizardChoice) => void }): JSX.Element`
  - `export type WizardChoice = { rom: RomInfo | null; template: Template; name: string; dest: string | null }` (für die Schau in Task 5)
  - `projectName.ts`: `suggestName(info: RomInfo): string` („Neues Platin“, „Neues HeartGold“, „Neues SoulSilver“, sonst „Neues Spiel“), `nameProblem(name: string): string | null` (leer → „Gib dem Projekt einen Namen.“; enthält `/`, `\`, `:` oder beginnt mit `.` → „Der Name darf kein /, \ oder : enthalten und nicht mit einem Punkt beginnen.“; über 64 Zeichen → „Höchstens 64 Zeichen.“), `destOf(parent: string, name: string, dspreSuffix: boolean): string`

Schritte (Titel wörtlich, `title="Neues Spiel aus ROM"`, Symbol des Bereichs Start/„open“):
1. **ROM** – Feld „ROM-Datei“ (Pfad) mit „Datei wählen…“; nach einer Wahl `inspectRom`: Karte mit `DSScreen` (Titel aus dem Kopf), Zeilen Spiel („Platin“, „HeartGold“, „SoulSilver“ aus Code IPK/IPG), Sprache („Deutsch“ …), Spielcode in Mono, Plakette Freigabe (`success` „Freigegeben“, `warning` „Nicht geprüft“ mit Satz „Für diese Sprache gibt es kein Testprojekt. Entpacken geht, Editoren und Vorlagen sind nicht geprüft.“, `locked` „Gesperrt“ mit dem Grund, `error` „Kein Gen-4-Spiel erkannt“), Basis („Bekannte Basis-ROM: Platin DE (Testprojekt)“ bzw. „Unbekannte Fassung: vielleicht schon verändert.“); Satz „Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM. Die ROM bleibt unverändert.“ `canNext` nur bei `verified`/`untested`.
2. **Vorlage** – drei Auswahlkarten (Radiogruppe, `data-template`): „Original“ („Das Spiel, wie es ist.“), „Leere Region mit Startstadt“ (gesperrt, „Kommt mit 3g.2“), „Nur Daten“ („Für Balancing: Werte, Lernsets, Trainer. Startet im Bereich Pokémon.“). Bei `untested` an jeder Karte die Plakette „Nicht geprüft“.
3. **Name und Ziel** – „Name“ (Vorschlag `suggestName`), „Ordner“ (aus `defaultProjectParent`, mit „Ordner wählen…“), Schalter „Endung _DSPRE_contents (für DSPRE)“ (Start aus `dspreSuffix`), darunter der volle Pfad in Mono; `checkProjectDest` nach jeder Änderung (entprellt 200 ms): `not_empty` → Fehler „Den Ordner gibt es schon und er ist nicht leer. Wähle einen anderen Namen.“, `not_a_directory` → „Dort liegt schon eine Datei.“, zu wenig Platz → „Zu wenig Platz: <frei> frei, gebraucht etwa <nötig>.“ (MB, gerundet), Befehlsfehler (Brücke) → sein Text. `canNext` = kein Namensproblem und `ok`.
4. **Zusammenfassung** – `summary`: „Legt „<Name>“ aus <Spiel> (<Sprache>) mit der Vorlage <Vorlage> an, in <Pfad>. Die ROM bleibt unverändert.“; `submitLabel` „Anlegen“.
- Anlegen: `busy`, der Schritt zeigt `CheckList` mit „Lesen“, „Entpacken“, „Vorlage anwenden“, „Prüfen“ (Zustände offen, läuft mit Fortschritt `n/m`, fertig); Fehler → `error` mit `errorText(e)`, zurück zur Zusammenfassung möglich.
- Ergebnis: `DoneResult` Titel „Projekt angelegt“, Zeilen Pfad und „Basis-ROM SHA-1 <erste 8 Zeichen>…“, Als Nächstes „Öffnen lädt das Projekt.“ (bei „Nur Daten“: „Öffnen startet im Bereich Pokémon.“); `onShow` = `onOpen(created)`, `showLabel="Öffnen"`, kein `onPlay`.

- [ ] **Step 1: Failing tests** (`NewProjectWizard.test.tsx`, API gemockt wie in `NewAreaWizard.test.tsx`):
  - `walks_rom_template_name_and_summary_then_creates` (Klicks/Enter durch alle Schritte; `createProject` mit `rom`, `dest = "<parent>/Neues Platin"` im Browser-Fall ohne Endung bzw. `…/Neues Platin_DSPRE_contents` bei Schalter an, `template: "original"`; Ergebnis zeigt „Projekt angelegt“ und die Taste „Öffnen“, Klick ruft `onOpen`).
  - `locks_diamond_with_its_reason` (Weiter gesperrt, Text „Diamant/Perl: ohne Testprojekt nicht freigegeben“).
  - `untested_language_shows_the_plaque` („Nicht geprüft“).
  - `refuses_a_non_empty_destination` (Weiter gesperrt, Satz wie oben).
  - `initial_rom_jumps_to_the_template_step` (mit `initialRom` und `verified` steht `aria-current="step"` auf „2. Vorlage“).
  - `shows_progress_and_an_error` (`createProject` verwirft mit `{code:"error", message:"build: …"}` → Meldung sichtbar, Schritt Zusammenfassung erreichbar).
  - `projectName.test.ts`: `suggestName`, `nameProblem`, `destOf` (beide Schalterstellungen, Elternpfad mit und ohne `/` am Ende).
  - `WizardFrame.test.tsx`: `result_uses_the_show_label`.
- [ ] **Step 2: Run** `pnpm -C apps/studio vitest run src/project src/workbench/wizards/WizardFrame.test.tsx` → FAIL.
- [ ] **Step 3: Implement** mit dem Skill `frontend-design:frontend-design` (Schritt-Layout aus `WizardStep.module.css`, Auswahlkarten nach dem Muster „große Auswahlkarten“, nur Tokens).
- [ ] **Step 4: Run** → PASS; `bash scripts/check-styles.sh`.
- [ ] **Step 5: Commit** `feat(studio): wizard "Neues Spiel aus ROM" with ROM, template, name and destination, summary, progress and result`

---

### Task 5: Start-Bereich, Drag-and-drop, App-Anbindung, Galerie

**Files:**
- Create: `apps/studio/src/project/ProjectStart.tsx`, `ProjectStart.module.css`, `ProjectStart.test.tsx`, `apps/studio/src/project/romDrop.ts` (+ Test), `apps/studio/src/ui/gallery/pages/ProjectPage.tsx` (Galerie-Seite „Projektstart“: Start mit Hauptknopf und Drop-Hinweis, die vier Schritte mit Beispieldaten, Fortschritt, Ergebnis; Eintrag in `pages/index.ts`)
- Modify: `apps/studio/src/ui/frame/StartPane.tsx` (+ Test): optionale Props `onNewGame?: () => void`; mit ihr ist „Neues Spiel aus ROM…“ die primäre Taste und „Projekt öffnen“ sekundär, darunter die zwei Sätze aus den Entscheidungen; ohne sie bleibt alles wie heute. `apps/studio/src/App.tsx` (Start rendert `ProjectStart` statt `StartPane`; `onOpenCreated(created)` → `session.onOpen(resetProject, created.root)` und bei `template === "data_only"` `setArea("pokemon")` statt Welt), `apps/studio/src/app/paletteEntries.ts` (+ Test; Befehl „Neues Spiel aus ROM…“ wechselt auf Start und öffnet den Assistenten; gesperrt, solange ein Welt-Assistent offen ist)

**Interfaces:**
- Consumes: Task 4 (`NewProjectWizard`, `WizardChoice`), `Hinge` (`ui/frame/Hinge.tsx`, Prop `wizard`; Vorbild `WizardHinge` in `ui/gallery/pages/FramePage.tsx`), `StartPane`
- Produces:
  - `export function ProjectStart(props: StartPaneProps & { wizard: StartWizardRequest | null; onWizard: (r: StartWizardRequest | null) => void; onOpenCreated: (c: Created) => void }): JSX.Element` mit `type StartWizardRequest = { rom?: string; notice?: string }` (State in `App.tsx`, damit die Palette öffnen kann)
  - Ohne Assistent: `StartPane` in einer Drop-Fläche (`data-drop="idle|over"`; bei `over` ein Rahmen mit „ROM hier ablegen“). Mit Assistent: Spalte aus Schau (Vorschau: `DSScreen` mit dem ROM-Titel oder „Noch keine ROM“, darunter Spiel, Vorlage, Zielpfad), `Hinge wizard` (enthält `[data-wizard-bar]`) und Hand-Fläche `[data-wizard-hand]`, Höhe wie `HAND_DEFAULT` des Decks.
  - `romDrop.ts`: `pathFromDataTransfer(dt: DataTransfer): {path: string} | {file: string} | {none: true}` (`text/uri-list` mit `file://` → dekodierter Pfad; `text/plain` absoluter Pfad; sonst `files[0].name`), `isRomPath(p: string): boolean` (Endung `.nds`, Groß/klein egal), `useRomDrop(enabled: boolean, onRom: (r: StartWizardRequest) => void, onRefuse: (msg: string) => void)` (Tauri: `getCurrentWebview().onDragDropEvent`, `type === "drop"`; mehrere Pfade oder keine `.nds` → `onRefuse("Nur eine .nds-Datei ablegen.")`).

- [ ] **Step 1: Failing tests**:
  - `StartPane.test.tsx`: `new_game_is_the_primary_action` (erste Taste „Neues Spiel aus ROM…“ mit `data-kind="primary"` o. ä. wie im Button-Test, „Projekt öffnen“ sekundär, Satz „Du brauchst deine eigene ROM-Datei (.nds).“); ohne `onNewGame` unverändert.
  - `romDrop.test.ts`: URI-Liste mit Leerzeichen (`file:///a%20b/x.nds` → `/a b/x.nds`), `text/plain`, nur Datei, `isRomPath("X.NDS")`.
  - `ProjectStart.test.tsx`: `drop_of_a_rom_path_opens_the_wizard_with_it` (synthetisches `drop`-Event mit `text/plain`), `drop_of_a_file_without_path_opens_the_wizard_with_the_notice`, `drop_of_two_files_or_a_txt_is_refused`, `drop_while_the_wizard_is_open_does_nothing`, `wizard_renders_into_its_own_hinge_and_hand` (`[data-wizard-bar]` und `[data-wizard-hand]` im Start, `role=dialog` mit `data-wizard`).
  - `App.test.tsx` (vorhandenes Muster): `data_only_opens_in_pokemon` (gemocktes `createProject` und `openProject`), `palette_new_game_opens_the_start_wizard`.
- [ ] **Step 2: Run** `pnpm -C apps/studio vitest run` → FAIL.
- [ ] **Step 3: Implement** mit `frontend-design:frontend-design`; `App.tsx` nur: State `startWizard`, Rendern von `ProjectStart`, `onOpenCreated`, Palette-Eintrag.
- [ ] **Step 4: Run** `pnpm -C apps/studio vitest run`, `pnpm -C apps/studio build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` → PASS.
- [ ] **Step 5: Commit** `feat(studio): start screen with "Neues Spiel aus ROM…", drag-and-drop of a .nds and the wizard in its own hinge`

---

### Task 6: Browser-Gate 3g.1, Doku, Nachtrag Design-System

**Files:**
- Create: `docs/phase-3g-gate.md` (Muster `docs/phase-3b4-gate.md`: Kopf, Tabelle der Punkte mit „Wer“, Abschnitt „Ergebnis“; Teil A = 3g.1), `design-system/openpencil/beispiele/projektstart.body.js` (Screen „App: Start, Neues Spiel aus ROM“ mit `jsx-lib-app.js`, Schritt 1 mit erkannter Platin-ROM), `design-system/openpencil/NACHTRAG-3g.md` (welche Seite, welche Frames, wie einspielen: `build-screen.sh`, `op.mjs render`, danach `fix_fig_text.mjs --rgb555`; nicht von dieser Session ausgeführt)
- Modify: `design-system/README.md` (nur neuer Abschnitt „Nachtrag 3g“ am Ende: Start mit Hauptknopf und Drop-Fläche, Assistent ohne Projekt mit eigenem Scharnier, Auswahlkarte „Vorlage“, Fortschrittsliste beim Anlegen), `docs/agent-gate.md` (Abschnitt „Phase 3g im Browser-Modus“ am Ende: Befehle `inspect_rom`, `default_project_parent`, `check_project_dest`, `create_project` mit `call.sh`-Beispielen; Drop im Browser per `text/plain`), `docs/ROADMAP.md` (nur „Status:“ in Abschnitt 3g: „3g.1 umgesetzt, Abnahme offen; 3g.2 offen“)

Gate-Punkte Teil A (Spalte „Wer“):
| (A1) gated `project_start_*` auf Platin DE, HeartGold DE, HeartGold US: Anlegen, Bauen, SHA-1 gleich der Eingabe | Agent |
| (A2) `inspect_rom`: Kopie mit Code ADAD (synthetisch) abgelehnt mit Grund; nicht leeres Ziel abgelehnt | Agent |
| (A3) Browser: Start zeigt „Neues Spiel aus ROM…“ primär; Drop eines Pfads; Schritte ROM, Vorlage, Name und Ziel (ohne Endung, unter `_testkopien/3g-gate-a/`), Zusammenfassung, Anlegen, „Öffnen“ landet in der Welt mit dem neuen Projekt; „Nur Daten“ landet in Pokémon | Agent + Blick |
| (A4) Brücke lehnt Ziel mit `_DSPRE_contents` ab | Agent |
| (A5) DSPRE öffnet den angelegten Ordner (mit Endung, aus der App) | Hand |
| (A6) Tauri-App: nativer Dateidialog mit `.nds`-Filter, Drop aus dem Finder, Vorschlag `~/Documents/Gen4 Studio` | Hand |

- [ ] **Step 1:** Kopien anlegen: `mkdir -p …/_testkopien/3g-gate-a && cp "<Platin DE>.nds" …/_testkopien/3g-gate-a/pt.nds` (ebenso `hg.nds`).
- [ ] **Step 2:** Test-Umgebung starten (Ports 1450/1451, siehe Auftrag), A3 und A4 im Browser bzw. mit `call.sh` durchspielen, Zahlen und Orte notieren.
- [ ] **Step 3:** A1 gated mit `G4_TEST_ROM` je Kopie (`cargo test --workspace --all-features project_start_ -- --nocapture`).
- [ ] **Step 4:** Doku, Nachtrag, Gate-Datei Teil A mit Ergebnis, Roadmap-Status.
- [ ] **Step 5: Commit** `docs(3g.1): gate part A, design-system addendum 3g, agent gate section; roadmap status`
