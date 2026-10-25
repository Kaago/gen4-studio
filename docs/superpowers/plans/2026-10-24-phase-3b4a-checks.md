# Phase 3b.4a „Prüfen und Beheben“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein Kernel-Modul `checks` mit dem Prüfkatalog der Spec (harte Regel: Fehler nur bei 0 Vanilla-Treffern, gated bewiesen), Snapshot unter dem Lock und Volllauf ohne Lock, Fixes als Plan → Vorschau → ein Changeset → Nachprüfung, Ausblenden in `g4-checks.json`; Prüflicht mit drei Zählern, Panel Probleme mit Gruppen, Öffnen, Beheben, Ausblenden, dieselbe Tabelle in Technik, Inspector-Abschnitt „Prüfungen“, Fehlerfarbe an Warp-Markern.

**Architecture:** `crates/g4-kernel/src/checks/` (`mod.rs`, `limits.rs`, `snapshot.rs`, `world.rs`, `scripts.rs`, `text.rs`, `data.rs`, `fix.rs`, `ignore.rs`). Der Snapshot kopiert unter dem Lock nur geladene, geänderte Assets (Overlay über `Store::peek`), alles andere liest er wie `RefIndex`/`Disk` von der Platte; geprüft wird auf einem Arbeitsthread. Befunde tragen ein `Subject` plus Feld; die Abbildung auf `NavTargetDto` geschieht im Tauri-Crate. Die Oberfläche bekommt die Befunde als Quelle `check` im vorhandenen `ProblemsStore` (je Lauf als Ganzes ersetzt), die Schwere `info` und `countsText` mit Hinweisen.

**Tech Stack:** Rust (g4-kernel, Tauri 2, serde_yaml für `check_limits.yaml`, rotom-Parser für den AST, chatot `measure_line_width`/`word_wrap`), React 19 + `ui/`, vitest 5, `g4emu`-Szenario mit `expect` an Edit-Einträgen.

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md](../specs/2026-10-23-phase-3b4-import-checks-design.md) §1 (3b.4a), §3, §4, §5, §6, §7, §8, §10 (3b.4a), §11 Gate A, §12. **Geprüfte Werte:** [docs/research/2026-10-23-3b4/task0-3b4.md](../../research/2026-10-23-3b4/task0-3b4.md) (216 px belegt; `0x0FFF` nur als Paar mit Ziel-Warp 256; Fehler-Kandidaten 0/0/0; `map.bdhc-size` wird Fehler, Grenze 0x9000 ab Byte 16; `matrix.header-elsewhere` 0/0/0 mit Ausnahme Header-Zelle 0; Trainer ohne Party stürzt nicht ab; Snapshot unter dem Lock < 0,3 ms; Pt-Scriptdateien 0101/0164/0365 ohne 0xFD13; HGSS-Header-Matrix an Offset 4). Katalog-Zählungen: [checks-report.md](../../research/2026-10-23-3b4/checks-report.md).

Stand 24.10.2026, geschrieben gegen `main` nach dem Merge von 3b.3b (Task 0 unten gleicht die Namen ab). Pfade relativ zur Repo-Wurzel; Frontend relativ zu `apps/studio/src`.

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Lauf | bei Bedarf (`run_checks`) und automatisch nach jeder Store-Änderung mit 300 ms Debounce (nur die Gruppe `Cheap`); Text- und Script-Prüfungen je Datei gecacht (`text_generation`, Quell-Hash) | billige Prüfungen 20–80 ms; Text kalt 2,2 s |
| Snapshot | unter dem Lock nur `peek` geladener Assets und die Header-Tabelle; Platte ohne Lock | Messung < 0,3 ms unter dem Lock, Budget 5 ms |
| Ergebnis-Ort | `ProblemsStore` Quelle `check`, Schwere `info`; die 3b.3b-Quelle „Begegnungen“ geht in `check` auf (`encounter.*`) | ein Store für Prüflicht, Panel, Technik; keine Doppelzählung |
| Fixes im Kernel | nur ohne Auswahl: `wrap`, `append_message`, `swap_min_max`, `create_script`; `warp.dest-*` und `event.trainer-missing` über UI-Picker plus `update_event` | das Gate ruft `apply_fix` über die Brücke |
| Ausblenden | `<root>/g4-checks.json` (nicht `.g4/`, das ist löschbarer Cache) | Ausgangslage wandert mit dem Projekt |
| Doppelte Befunde | 200-px-Warnung aus `text::validate` und `write_archive` raus; rotoms `MessageLineTooLong` an der Umwandlung in `scripts.rs` per Variante verworfen | sonst dieselbe Zeile zweimal |
| `trainer.no-party` | bleibt Fehler (genutzt) bzw. Warnung (ungenutzt), Meldung ohne „stürzt ab“: „Der Kampf läuft gegen ein unsichtbares Pokémon auf Stufe 0.“ | gemessen |
| `map.bdhc-size` | Fehler in beiden Spielen (0x9000 ab Byte 16) | asm-Beleg für HGSS |
| Texturnamen gegen Area-NSBTX | kein Fehler, nur Hinweis beim Import (3b.4b) | Vanilla hat 14/7/7 fehlende Namen |

Geändert in der Umsetzung (Entscheid R12, Task 6): der automatische Lauf nach einer Änderung ist der **volle** Lauf über alle Gruppen (`run_checks { scope: "all" }`, 300 ms nach dem Bus-Thema `edited`, von der Oberfläche angestoßen, in Tauri und Brücke gleich, kein Zeitgeber im Backend). Die Caches je Textarchiv und Scriptdatei machen ihn warm billig (gemessen 261 ms auf HeartGold DE, 250 bis 290 ms auf allen drei Projekten); jeder Bericht ersetzt die Befunde als Ganzes. `scope: "cheap"` bleibt für Brücke und Szenarien.

## Global Constraints

- Wie die Pläne 3b.3a/3b.3b (Vanilla nie beschreiben, `vanilla_copy()`, PATH, `cargo test --workspace --all-features` mit Filter, gated mit `G4_TEST_PROJECT`/`G4_ROMS_DIR`, Styling nur über `ui/`, Base UI nur in `ui/primitives`, Tests über Rollen und `data-*` (`data-check`, `data-finding`), deutsche UI-Texte mit Ausweg, Kernel-Texte englisch mit Zahlen, Commits englisch mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`).
- Harte Regel: jede Prüfung mit Schwere `Error` hat im gated Test `vanilla_checks` das Soll 0 auf Platin DE, HeartGold DE und HeartGold US; Warnungen und Hinweise haben feste Sollzahlen aus Task 0 (Pt / HG DE / HG US): Schild ohne Script 1/0/0, `header.level-target` 1/1/0 (HG US ohne übernommene Scripts: `skipped`), `event.trigger-empty` 0/1/1, `event.same-spot` 2/8/8, `warp.never-fires` 10/12/12, `text.too-wide` 90/114/113; `matrix.header-elsewhere` ist Fehler mit 0/0/0 (Header-Zelle 0 ausgenommen, Ruling R16).
- Ausnahmen mit Quelle in `check_limits.yaml`: Warp-Ziel `(0x0FFF, 256)` = dynamisch; Trainer 0 = Dummy; Eventdatei 0 = geteilt; Script 0 = Dekoration; Begegnungs-Ende Pt 0xFFFF / HGSS 0xFF; Header-Zelle 0 in Matrizen; Scriptdateien ohne 0xFD13-Ende werden über rotoms Parser gelesen, nie per Byte-Scan.
- Neue Befehle nur aus Spec §10 (3b.4a): `run_checks`, `checks_status`, `plan_fix`, `apply_fix`, `ignore_finding`, `unignore_finding`, `accept_baseline`, `set_check_enabled`; dazu `g4k check`. Jeder in `bridge/dispatch.rs`, `generate_handler!`, `api.ts`. Die Brücke ist synchron: `run_checks` dort ohne Debounce.
- Prüfungen blockieren nie das Bauen und nie den Store-Lock länger als 5 ms (Test).

## Review Focus

- Store-Änderung während eines laufenden Volllaufs: das Ergebnis wird verworfen und neu angestoßen, die Oberfläche zeigt nie Befunde einer alten Generation (Task 2, Task 6).
- `apply_fix` auf einem Befund, dessen Subjekt sich seit `plan_fix` geändert hat: „Befund ist nicht mehr aktuell“, nichts wird geschrieben (Task 5).
- Ein Projekt ohne übernommene Scripts: `script.*` und `event.script-missing` melden `skipped` mit Grund, nie 0 Befunde als „ok“ (Task 3).
- Ausgeblendeter Befund, dessen Subjekt sich ändert: der Fingerprint passt nicht mehr, der Befund erscheint wieder; unveränderte bleiben ausgeblendet (Task 4).
- 724 Textarchive kalt: der erste Lauf meldet Fortschritt „Prüft … Texte 312/724“, die Oberfläche bleibt bedienbar, der Lock wird je Archiv nur für `peek` gehalten (Task 2).

---

### Task 0: Abgleich mit `main` nach 3b.3b

- [ ] **Step 1:** Prüfen und im Bericht nennen: `AssetKind::Trainer` und `TrainerDoc` (`crates/g4-formats/src/data/trainer_doc.rs`), `encounter_rules::warnings` (`crates/g4-kernel/src/data/encounter_rules.rs`), `NavTarget`/`NavTargetDto` kennen `trainer` und `encounter` (`workbench/navigation.ts`, `commands/data/dto.rs`), `ProblemsStore`-Quellen (`workbench/problems/problems.ts`), Platzhalter-Syntax in `scripts/g4emu/g4emu/scenario.py`. Fehlt etwas, holt dieser Task es nach (kleinster Eingriff) und nennt es im Bericht.

---

### Task 1: Modul `checks`: Typen, Limits, Registry, `Severity::Info`

**Files:**
- Create: `crates/g4-kernel/src/checks/mod.rs`, `checks/limits.rs`, `crates/g4-kernel/metadata/check_limits.yaml`, `docs/formats/check_limits.md`
- Modify: `crates/g4-kernel/src/lib.rs`, `crates/g4-kernel/src/text.rs` (`Severity` + `Info`, serde `"info"`; die 200-px-Warnung aus `validate` und `write_archive` entfällt; `text::validate` liest die Breite nur noch für `text.encode`-Fehler), `crates/g4-kernel/src/scripts.rs` (Umwandlung `done.warnings` → `Problem`: `CompileWarning::MessageLineTooLong` verwerfen), zugehörige Tests

**Interfaces:**
- `pub enum Severity { Error, Warning, Info }` (in `text.rs`, re-exportiert in `checks`); `pub enum Subject { Header(u16), EventFile(u16), ScriptFile(u16), TextArchive(u16), Trainer(u16), Encounter(u16), Matrix(u16), Map(u16), Project }`; `pub struct Finding { pub code: &'static str, pub severity: Severity, pub subject: Subject, pub key: String, pub message: String, pub field: Option<String>, pub also: Vec<Subject>, pub fixes: Vec<FixId>, pub file: Option<(PathBuf, u32, u32)>, pub fingerprint: u64 }`; `pub enum FixId { Wrap, AppendMessage, SwapMinMax, CreateScript }`; `pub enum Group { Cheap, PerScript, PerArchive }`; `pub trait Check: Sync { fn code(&self) -> &'static str; fn severity(&self) -> Severity; fn group(&self) -> Group; fn run(&self, snap: &Snapshot, limits: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped>; }`; `pub struct Skipped { pub why: String }`; `Registry::all() -> &'static [&'static dyn Check]`; `Limits` aus `check_limits.yaml` mit `dialog_line_px: 216 (verified: true, source: pokeplatinum src/field_message.c:30, pokeheartgold src/dialog_box.c:12)`, `dialog_lines_max: 2`, `warp.dynamic: { header: 0x0FFF, warp: 256 }`, `trainer.dummy: 0`, `event_file.shared: 0`, `script.decoration: 0`, `encounter.none: { pt: 0xFFFF, hgss: 0xFF }`, `matrix.header_none: 0`, `model.max_bytes: 0xF000`, `bdhc.max_bytes: 0x9000` (ab Byte 16), `permissions.bytes: 2048`; `Project::limits() -> &Limits`.
- `fingerprint(subject, key, relevant_bytes) -> u64` (FNV über Subjekt, Schlüssel und die geprüften Felder, nicht über die ganze Datei).

- [ ] **Step 1: Failing tests**: `limits_yaml_parses_with_sources`, `severity_info_serialises_as_info`, `validate_no_longer_warns_at_200_px`, `compile_drops_message_line_too_long`, `fingerprint_changes_only_with_relevant_bytes`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(kernel): checks module skeleton, limits metadata, info severity; drop the 200 px text warning`

---

### Task 2: Snapshot und Runner

**Files:**
- Create: `crates/g4-kernel/src/checks/snapshot.rs`, `checks/runner.rs`, `crates/g4-kernel/tests/checks_runner.rs`
- Modify: `crates/g4-kernel/src/store.rs` (`peek` wird `pub(crate)` nutzbar aus `checks`; `Store::snapshot_overlay() -> Overlay { generation: u64, assets: Vec<(AssetId, Vec<u8>)>, locked: Vec<(AssetId, String)>, text_docs: Vec<(u16, TextArchiveDoc)> }` nur aus geladenen Assets, ohne Platte), `apps/studio/src-tauri/src/state.rs` (Session hält `ChecksState { last: Option<ChecksReport>, running: bool, progress: Option<(String, u32, u32)>, debounce: Instant }`)

**Interfaces:**
- `Snapshot::build(project: &Project, overlay: Overlay) -> Snapshot` (ohne Lock: Header-Tabelle, Matrizen, Eventdateien, Karten-Köpfe (Modellgröße, BDHC-Länge aus `MapFile::read`), Trainer, Begegnungen, Scriptquellen (bei übernommenen Scripts) und Textarchive von der Platte, Overlay drüber; Text- und Script-Teile lazy mit Cache `HashMap<u16, (u64 /* generation|hash */, Decoded)>`, der zwischen Läufen in `ChecksState` lebt); `Runner::run(snap, limits, ignore, groups: &[Group], progress: impl FnMut(&str, u32, u32)) -> ChecksReport { findings, ignored: Vec<Finding>, skipped: Vec<(code, why)>, ran_ms, generation }`; nach dem Lauf vergleicht der Aufrufer `generation` mit `store.change_generation()`; abweichend → verwerfen und neu.
- Tauri: `run_checks { scope?: "all" | "cheap" }` (Tauri `spawn_blocking`, Brücke synchron), `checks_status {}` → `{ running, progress, generation, counts }`; Auto-Lauf: nach jedem schreibenden Befehl setzt die Session `debounce`; ein Timer-Task startet `Cheap` nach 300 ms Ruhe (Tauri) – in der Brücke nur auf Abruf.

- [ ] **Step 1: Failing tests**: `overlay_copies_only_loaded_assets` (Fixture: zwei Eventdateien geladen und geändert, Overlay enthält genau zwei), `snapshot_reads_unloaded_assets_from_disk`, `stale_generation_is_discarded`, `text_cache_hits_until_text_generation_changes`; gated `checks_runner.rs`: `overlay_under_lock_takes_under_5_ms_on_hgss` und `full_run_on_vanilla_under_three_seconds_warm`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (gated gegen HG DE) → **Step 5: Commit** `feat(kernel,studio): checks snapshot from a store overlay, runner with progress and generation guard`

---

### Task 3: Prüfungen Welt, Scripts, Text, Daten

**Files:**
- Create: `crates/g4-kernel/src/checks/world.rs`, `scripts.rs`, `text.rs`, `data.rs`, `crates/g4-kernel/tests/vanilla_checks.rs` (gated)
- Modify: `crates/g4-kernel/src/scripts.rs` (`declared_slots` → `pub(crate)`), `crates/g4-kernel/src/data/encounter_rules.rs` (Warnungen liefern `code` je Regel: `encounter.empty-slot`, `encounter.level-zero`, `encounter.min-max`), `crates/g4-kernel/metadata/check_limits.yaml` (`script.message_params`: je Familie die Befehle und Argumentpositionen mit Nachrichten-ID, Quelle rotom-Datenbank; Pt 7 Befehle mit `msg_id`, HGSS 3 plus die `u16`-typisierten `message`, `option_msg` usw.)

**Interfaces:**
- Codes und Schwere nach Spec §4 mit den Task-0-Korrekturen: Fehler `warp.dest-header`, `warp.dest-warp` (Paar `(0x0FFF, 256)` ausgenommen), `warp.off-map`, `event.script-missing` (Figuren, Trigger), `event.trainer-missing`, `event.off-map`, `header.ref-range`, `header.wild-dangling`, `header.script-kind`, `script.message-missing`, `trainer.no-party` (von einem Event genutzt, RefIndex), `trainer.empty-species`, `trainer.id-range`, `matrix.map-range`, `matrix.header-range`, `map.model-invalid` (Kopf `BMD0`, BOM, Version, Größenfeld, genau ein MDL0-Modell, ≤ 0xF000; **vorhandener** Leser `crates/g4-formats/src/nsbmd_header.rs` aus 3b.4b Tasks 1–2 (`NsbmdHeader::read`, `model_count`), nur wiederverwenden; die Konstanten `MODEL_MAX_BYTES`/`BDHC_MAX_BYTES` in `crates/g4-kernel/src/map_import.rs` werden in Task 1 auf `Project::limits()` umgestellt), `map.bdhc-size` (> 0x9000 ab Byte 16), `text.encode`, `text.json-broken`, `asset.unreadable`, `map.building-range`, `area.tileset-range`, `matrix.header-elsewhere` (Zelle 0 ausgenommen); Warnung `event.script-missing` (Schilder), `header.level-target`, `event.trainer-blind`, `event.trigger-empty`, `event.same-spot`, `header.text-split`, `trainer.no-party` (ungenutzt), `trainer.no-moves`, `trainer.double-one`, `text.too-wide` (216 px, nur Dialogarchive = von einem Header als `text_archive_id` genutzt), `text.three-lines`, `encounter.empty-slot`, `encounter.level-zero`; Hinweis `warp.never-fires` (`tiles::warp_fires`), `encounter.min-max`. Meldungen: der Kernel liefert `code`, `args: Vec<String>` und `message` als englischen Satz mit Zahlen (Fallback für `g4k check`); die deutschen Sätze mit Ausweg stehen in `apps/studio/src/checks/messages.ts` (Schlüssel = Code, Platzhalter `{0}`, `{1}`).
- Scripts: `script.message-missing` über den rotom-Parser (AST wie `project_compile::global_dependencies`), Nachrichtenzahl aus dem Archiv des Headers, der die Scriptdatei nutzt (mehrere Header mit verschiedenen Archiven → kleinste Zahl, `also` nennt die Header); ohne übernommene Scripts `Skipped { why: "scripts not adopted" }`; Common-Scripts ohne Header `skipped`.

- [ ] **Step 1: Failing tests** je Prüfung (synthetische Fixture, Treffer und Nicht-Treffer, Ausnahmen): mindestens `warp_dest_header_ignores_the_dynamic_pair`, `event_script_missing_is_warning_for_signs`, `script_message_missing_uses_the_header_archive_and_skips_without_adoption`, `text_too_wide_only_in_dialog_archives_at_216`, `trainer_no_party_is_error_only_when_used`, `map_model_invalid_rejects_two_models_and_oversize`, `map_bdhc_size_counts_from_byte_16`, `matrix_header_elsewhere_ignores_zero`; gated `vanilla_checks.rs`: Soll je Code und Projekt wie in den Global Constraints, Fehler 0/0/0, Laufzeit gemessen.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (gated gegen alle drei Projekte) → **Step 5: Commit** `feat(kernel): the check catalogue for world, scripts, texts and data with the gated vanilla proof`

---

### Task 4: Ausblenden, Ausgangslage, Befehle, `g4k check`

**Files:**
- Create: `crates/g4-kernel/src/checks/ignore.rs` (+ Tests), `apps/studio/src-tauri/src/commands/checks.rs`, `apps/studio/src/api.checks.ts` (+ test)
- Modify: `crates/g4-kernel/src/bin/g4k.rs` (`g4k check <projekt> [--json] [--expect code=n …]`: Exit 1 bei Abweichung mit Code, Soll, Ist), `crates/g4-kernel/tests/cli.rs`, `bridge/dispatch.rs`, `lib.rs`, `commands/data/dto.rs` (`NavTargetDto` vollständig: `header, event, warp, matrixCell, script, textArchive, species, move, item, trainer, encounter, map`; `subject_to_nav(Subject, field) -> NavTargetDto`), `apps/studio/src/assets.ts`

**Interfaces:**
- `IgnoreFile { ignored: Vec<{ code, subject, key, fingerprint }>, disabled: Vec<String> }` in `<root>/g4-checks.json` (atomar, `write_atomic`); `ignore_finding`, `unignore_finding`, `accept_baseline` (alle aktuellen Warnungen und Hinweise), `set_check_enabled`; Fehler lassen sich nie ausblenden (`KernelError::InvalidProject` mit Text).
- `ChecksReportDto { findings: Vec<ProblemDto>, ignored: Vec<ProblemDto>, skipped: Vec<{ code, why }>, ranMs, generation, running, progress? }`; `ProblemDto` + optionale Felder `code`, `subject: { kind, index }`, `key`, `label` („Eventdatei 0411 · Warp 2“), `target: NavTargetDto`, `also: Vec<NavTargetDto>`, `fixes: Vec<{ id, label, irreversible }>`, `fingerprint`, Quelle `check`, Schwere `info`.

- [ ] **Step 1: Failing tests**: `ignore_round_trips_the_file`, `changed_subject_reappears`, `errors_cannot_be_ignored`, `accept_baseline_ignores_every_warning_and_info`, `g4k_check_expect_fails_with_code_soll_ist` (`cli.rs`), Dispatch-Fixtures je Befehl, `commands_match_the_tauri_handler`, `api.checks.test.ts`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(kernel,studio): ignore file and baseline, checks commands in Tauri and the bridge, g4k check`

**Task 4 ist erledigt** (Branch `phase-3b4a-checks`, Commits 7f1822e und edd000f): `checks/ignore.rs` (`IgnoreFile`, `IgnoreSet::load/save/ignore/unignore/accept_baseline/set_enabled`, `<projekt>/g4-checks.json`, Fingerprints als 16 Hex-Ziffern), `commands/checks.rs` mit den sechs Befehlen und `commands/checks/label.rs` (deutsche Orte „Eventdatei 0394 · Warp 0“), `g4k check [--json] [--expect CODE=N]`, `api.checks.ts`; ein während eines Laufs wieder eingeschalteter Check gilt nicht mehr als abgeschaltet (edd000f). Abweichend vom Text oben: `run_checks` nimmt `"full"` als `"all"` (R1, für die Entwürfe von 3b.4b); `NavTargetDto` spiegelt `navigation.ts` genau (`header`, `matrixCell`, `area`, `building`, `event`, `textArchive`, `script`, `species`, `move`, `item`, `trainer`, `encounter`; ein Warp ist `event` mit `eventKind: "warp"`, es gibt kein `map` und kein `warp`), `target` ist optional (R3); die Fix-Ids sind überall snake_case (R2); die Verbreiterung von `ProblemDto` in TypeScript (`info`, `check`) kam erst mit Task 6 (R8).

---

### Task 5: Fixes: Plan, Vorschau, Changeset, Nachprüfung

**Files:**
- Create: `crates/g4-kernel/src/checks/fix.rs` (+ Tests)
- Modify: `apps/studio/src-tauri/src/commands/checks.rs` (`plan_fix`, `apply_fix`), `bridge/dispatch.rs`, `api.checks.ts`

**Interfaces:**
- `plan_fix(snapshot, finding, fix: FixId) -> Result<FixPlan { changes: Vec<FixChange { asset: AssetId, label: String, before: String, after: String }>, irreversible: bool }, KernelError>`; `apply_fix(store, project, finding, fix) -> Result<FixResult { changed: Vec<AssetId>, remaining: bool }, KernelError>`: prüft den Fingerprint (sonst `KernelError::Stale` „finding is no longer current“), läuft als `Store::transaction` (Label „Behoben: <Code>“), außer `CreateScript` (ruft `add_script_reported`, nicht rückgängig), prüft danach das Subjekt erneut (`remaining`). Fixes: `Wrap` (chatot `word_wrap` mit 216 px auf die betroffene Nachricht, Vorschau als Seiten), `AppendMessage` (leere Nachricht anfügen bis zur fehlenden Nummer), `SwapMinMax`, `CreateScript` (Script #n in der Scriptdatei des Headers anlegen).

- [ ] **Step 1: Failing tests**: je Fix `finding_disappears_after_apply` und `undo_brings_it_back` (außer CreateScript), `stale_fingerprint_is_refused_and_changes_nothing`, `plan_shows_before_and_after_for_wrap`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(kernel,studio): deterministic fixes with preview, one changeset and re-check`

**Task 5 ist erledigt** (Branch `phase-3b4a-checks`, Commit 4f76542): `checks/fix.rs` (`plan_fix`, `apply_fix` mit `IgnoreKey` als Identität des Befunds, Nachprüfung nur des Subjekts), `plan_fix`/`apply_fix` in Tauri, Brücke und `api.checks.ts`. Abweichend vom Text oben: beide Befehle nehmen `{ finding, fix }`, den Befund so, wie `run_checks` ihn liefert (gelesen werden `code`, `subject`, `key`, `fingerprint`), `fix` ist `wrap`, `append_message`, `swap_min_max` oder `create_script` (R2); ein veralteter Befund ist die neue Variante `KernelError::FindingStale` („finding is no longer current“, `CmdError`-Code `finding_stale`, Oberfläche „Befund ist nicht mehr aktuell“), `KernelError::Stale` bleibt „auf der Platte geändert“ (R5); „verändertes Subjekt“ heißt ein anderer Fingerprint des Befunds (R10); `create_script` legt genau Script #n an (neues `scripts::add_script_at_reported`), nicht den nächsten freien Platz wie `add_script_reported` (R9).

---

### Task 6: Oberfläche: Prüflicht, Panel Probleme, Technik, Inspector, Marker

**Files:**
- Create: `ui/primitives/BeforeAfter.tsx` (+ css, test), `ui/primitives/CheckList.tsx` (+ css, test), `apps/studio/src/checks/messages.ts` (+ test: jeder Code hat einen Satz mit Ausweg), `apps/studio/src/checks/useChecks.ts` (+ test: ruft `run_checks`, pollt `checks_status` während `running`, schreibt Quelle `check` in den `ProblemsStore`), `workbench/problems/FixDialog.tsx` (+ test), `workbench/inspector/ChecksSection.tsx` (+ test)
- Modify: `workbench/problems/problems.ts` (+ test: Quelle `check`, Schwere `info`, `ProblemCounts.infos`, `countsText` „2 Fehler, 1 Warnung, 3 Hinweise“, `same()` über `code+subject+key`, `problemTarget` über `target`), `workbench/problems/ProblemsPanel.tsx` und `ProblemsTable` (+ test: Gruppen nach Bereich (Welt, Scripts, Texte, Pokémon) und Code mit Zähler; Zeilen mit Meldung aus `messages.ts` und `label`; Tasten „Öffnen“ (`goTo(target)`), „Beheben…“ (`FixDialog` mit `BeforeAfter`, Plakette „Nicht rückgängig“ bei `irreversible`), „Ausblenden“ (nie bei Fehlern); Fuß „12 ausgeblendet · Wieder melden“, Taste „Erneut prüfen“, Spinner mit Fortschritt; `skipped` als Hinweiszeile), `ui/frame/Hinge.tsx` (+ test: drei Zähler, `lightKind` mit Hinweisen, Spinner während `running`), `technik/` „Prüfen und Bauen“ (+ test: dieselbe Tabelle, „Ausgangslage übernehmen“ mit Frage-Dialog und Zahl, Schalter je Code), `workbench/Inspector.tsx` (Abschnitt „Prüfungen“ mit den Befunden des gewählten Subjekts), `world/scene/markers.ts` (+ test: Warp-Marker in `status/fehler`, wenn ein Fehler-Befund das Event trifft; Farbe aus `tokens.ts` `szene`), `workbench/WorldExplorer.tsx` und `data/DataList.tsx` (Warnsymbol über `ListRow.trailing`/`TreeRow`), `data/encounters/*` (die 3b.3b-Warnungen kommen jetzt aus `check`, nicht mehr aus einer eigenen Quelle)

**Interfaces:**
- `BeforeAfter { before: string; after: string; label?: string }` (zwei Spalten, geänderte Zeilen hervorgehoben in `auswahl/flaeche`); `CheckList { rows: { id, state: "ok" | "warning" | "error" | "info", text: string, trailing?: ReactNode }[] }` (`role="list"`, Symbol plus Text, nie nur Farbe).
- Auto-Lauf im Frontend: `useChecks` stößt nach dem Bus-Thema `edited` mit 300 ms Debounce `run_checks { scope: "cheap" }` an (Tauri und Browser gleich); ein Volllauf über „Erneut prüfen“ oder beim Projektöffnen.

- [ ] **Step 1: Failing tests** (Rollen, `data-check`, `data-finding`): Zähler und `countsText` mit Hinweisen, Gruppen und Tasten im Panel, „Ausblenden“ fehlt bei Fehlern, Fix-Dialog zeigt Vorher/Nachher und ruft `apply_fix`, Inspector-Abschnitt listet Befunde des gewählten Events, Marker-Farbe bei Fehler, Technik-Schalter ruft `set_check_enabled`, `useChecks` pollt während `running`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, `bash scripts/check-styles.sh`; Browser: vier absichtliche Fehler auf einer Kopie (siehe Task 7) im Panel sehen, einen beheben, einen ausblenden.
- [ ] **Step 5: Commit** `feat(studio): check light with three counts, grouped problems with open, fix and ignore, checks in Technik, inspector and markers`

**Task 6 ist erledigt** (Branch `phase-3b4a-checks`, Commits 5f0a585 und 1eac4d9): Prüflicht mit drei Zählern und Fortschritt, Panel „Probleme“ mit Gruppen, `FixDialog`, Technik mit Ausgangslage und Schaltern, `ChecksSection` im Inspector, Fehlerfarbe an Warps, Markierungen in Explorer-, Trainer- und Begegnungsliste, `checks/messages.ts` mit Rust-Wächter, `checks/useChecks.ts`, neues Primitive `BeforeAfter`, neues Szenen-Token `szene/status/fehler` (`#DE3142`). Abweichend vom Text oben: `CheckList` gab es schon (3b.4b Task 3) und wird wiederverwendet; ein TypeScript-Typ `ProblemDto` für Probleme und Befunde (R8); der automatische Lauf ist der volle Lauf, nicht `run_checks { scope: "cheap" }` (R4, ersetzt durch R12, siehe „Entscheidungen“; gemessen warm 261 ms auf HeartGold DE); die Vorschau von „Umbrechen“ zeigt Vorher und Nachher als Text in `BeforeAfter`, nicht als DS-Bildschirm (R11); Markierungen nur in den Listen, die Befunde haben können (`DataList` für Pokédex, Attacken und Gegenstände hat keine Prüfung); dazu ein fünfter Bereich „Projekt“ für `asset.unreadable`.

---

### Task 7: Szenario-Teil Gate A, `expect` in `scenario.py`, Docs, Roadmap, volle Prüfung

**Files:**
- Create: `scripts/agent/scenarios/pt-pruefungen.yaml`, `docs/phase-3b4-gate.md` (Teil A mit Ergebnistabelle, Spalte „Wer“)
- Modify: `scripts/g4emu/g4emu/scenario.py` (+ `test_scenario.py`: Edit-Einträge dürfen ein drittes Element `{expect: {path, equals}}` tragen, geprüft gegen die passende Antwortzeile aus `bridge.jsonl`; Exit 2 mit Pfad, Soll, Ist), `docs/agent-gate.md` (Rezepte Prüfen, Beheben, Ausblenden, Ausgangslage; `call.sh run_checks`, `g4k check --expect`), `docs/dev-setup.md` (`vanilla_checks`, `g4k check`), `docs/ROADMAP.md` (3b.4 mit Schnitt a/b und korrigiertem Lieferumfang nach Spec §14; „3b.4a implementiert, Gate offen“), `design-system/README.md`

**Szenario `pt-pruefungen.yaml`** (`game: CPUD`, kein `play`): `accept_baseline`; `run_checks` mit `expect` Fehler 0, Warnungen 0; vier absichtliche Fehler: `update_event` Warp 0 der Eventdatei 394 auf Ziel-Warp 99 → `run_checks` `expect warp.dest-warp == 1`; `add_event` Figur mit Script 950 in einer neuen Scriptdatei (`new_script_file`, `set_header`) → `event.script-missing == 1`; `update_event` Figur mit `trainer_type = 1` und Script 3999 (kein Trainer) → `event.trainer-missing == 1`; `edit_text_archive` Nachricht mit 60 Zeichen in einem Dialogarchiv → `text.too-wide == 1`; dann `update_event` Warp zurück, `apply_fix create_script`, `update_event` `trainer_type = 0`, `apply_fix wrap` → `run_checks` 0/0.

- [ ] **Step 1: `scenario.py` `expect`** mit Test; Szenario auf einer frischen Kopie `pt-3b4a` bis grün.
- [ ] **Step 2: Docs** nach Dateiliste.
- [ ] **Step 3: Volle Prüfung**: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `cargo test --workspace --all-features`, `uv run pytest -q`, dann `G4_ROMS_DIR=<absolut> scripts/gate.sh` im Hintergrund; Tabelle in den Bericht.
- [ ] **Step 4: Commit** `feat(gate): checks scenario with expectations; gate document part A, recipes and roadmap for 3b.4a`

**Task 7 ist erledigt** (Branch `phase-3b4a-checks`, Commit 2ddd1b0 für `expect` und das Szenario, danach der Commit mit diesem Text, Gate-Dokument, Rezepten und Roadmap): `scenario.py` nimmt als drittes Element einer Änderung einen Namen oder `{expect: {path, equals} | [{path, equals}, …], as?: name}` (R13: Pfade aus Schlüsseln, Listenindizes, `.length` an Listen und Filtern `[schlüssel=wert]` mit Textvergleich, wie die Entwürfe von 3b.4b sie nutzen; verfehlt oder fehlend Rückgabe 2 mit Pfad, Soll, Ist); `pt-pruefungen.yaml` auf der frischen Kopie `pt-3b4a` grün (21 Erwartungen, 19 s); `docs/phase-3b4-gate.md` Teil A, Rezepte und Zuordnung in `docs/agent-gate.md`, `docs/dev-setup.md`, `docs/ROADMAP.md`, `design-system/README.md`; `scripts/gate.sh` grün. Abweichend vom Text oben: das Szenario übernimmt zuerst die Scripts (`create_script` braucht sie) und setzt beim Zurücknehmen von Objekt 6 auch das Script zurück, nicht nur `trainer_type = 0` (`event.trainer-missing` liest das Script); der Atlas-Eintrag „Geschichten“ sagt jetzt „Kommt mit Phase 3d“ (Story-Graph nach 3d, Spezifikation §1). Offen: der Browser-Teil von Gate A in der Abnahme.

---

## Reihenfolge und Parallelität

**0 → 1 → 2 → 3** seriell (jeder baut auf den Typen des vorigen); **4 ∥ 5** nach 3 (getrennte Dateien); **6** nach 4 und 5; **7** zuletzt.

## Abschluss

Nach Task 7: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`), Abnahme Gate A über die Test-Umgebung in einer eigenen Session. Danach 3b.4b ([Plan](2026-10-24-phase-3b4b-import.md)).
