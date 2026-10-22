# Phase 3b.2 „Skripte und Text“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Textarchive werden editierbar (chatot-JSON als Quelle), Scripts werden als rotom-Projekt übernommen und in einem CodeMirror-Editor mit `rotom-lsp` bearbeitet; Speichern kompiliert bzw. kodiert in die NARC-Member, der Build zieht Veraltetes nach und bricht mit einer Problemliste ab.

**Architecture:** Der Kernel bekommt zwei Module: `text` (chatot-JSON ↔ Member, `AssetKind::TextArchive` wird ein editierbares Store-Asset mit zwei Dateien) und `scripts` (rotom als Lib: Übernehmen, Compile über rotoms Projekt-API, Level-Script-JSON, Script-Auflösung); `build()` bekommt zwei Vorschritte und einen strukturierten Fehler `Compile(Vec<Problem>)`. Die Tauri-Schicht bekommt einen langlebigen stdio-Sidecar `rotom-lsp` mit `Channel`-Brücke und den Fehlercode `compile`. Das Frontend bekommt drei feste Panels („Text“, „Script“, „Probleme“), einen CodeMirror-Editor mit `@codemirror/lsp-client`, Navigation vom Header und vom Event zum Script und ein Level-Script-Formular.

**Tech Stack:** Rust (g4-kernel, Tauri 2), rotom v0.7.0 (git), chatot 0.8, uxie 0.9, toml 0.8; React 19, CodeMirror 6 (`codemirror`, `@codemirror/lsp-client`, `@codemirror/lang-json`), Dockview 8.6, vitest 5.

**Spec:** [docs/superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md](../specs/2026-10-22-phase-3b2-scripts-text-design.md)

## Global Constraints

- Die Vanilla-Projekte unter `../Roms/*_DSPRE_contents` werden nie beschrieben; schreibende Tests laufen auf `vanilla_copy()`, das GUI nur auf Kopien unter `../Roms/_testkopien`.
- Testbefehle aus der Repo-Wurzel mit `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`. Vor jedem Commit: `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, in `apps/studio`: `pnpm test`, `pnpm build`.
- Gated Tests: `G4_TEST_PROJECT="$PWD/../Roms/<Projekt>_DSPRE_contents"`, `G4_APICULA="$PWD/tools/apicula/bin/apicula"`, `G4_DSROM_CLI="$PWD/../Roms/_tools/dsrom"`, neu `G4_ROTOM_LSP="$PWD/tools/rotom/bin/rotom-lsp"`; die drei Projekte sind `Pokemon - Platin-Edition (Germany)` (CPUD), `Pokemon - Goldene Edition HeartGold (Germany)` (IPKD), `Pokemon - HeartGold Version (USA)` (IPKE).
- UI-Texte deutsch, Begriffe aus `apps/studio/src/workbench/terms.ts`; Schreibweise **„Script“** (Scriptdatei, Level-Script), nicht „Skript“. Code, Kommentare und Commit-Messages englisch.
- DSPRE-Layout 1:1 (Spec §3): `rotom.toml`, `.rotom/`, `expanded/scripts/NNNN.rotom|.json`, `expanded/textArchives/NNNN.json`, Member in `unpacked/scripts/NNNN` und `unpacked/textArchives/NNNN`. Nichts Eigenes daneben; in Member-Ordnern liegen nur `NNNN`-Dateien (`pack_dir` bricht sonst ab).
- Schreibreihenfolge Text: erst JSON (`write_atomic`), dann Member; die JSON wird danach nicht mehr angefasst (mtime ≥ Member, DSPREs `--newer`-Regel). Zeitstempel werden nie zurückgesetzt.
- Kein Netz zur Laufzeit: die Befehlsdatenbank kommt aus `crates/g4-kernel/metadata/scrcmd/`, nie aus `rotom init`.
- `rotom` (Lib) und `rotom-lsp` (Sidecar) stehen auf demselben Git-Tag `v0.7.0`.
- `CmdError.code` ∈ `stale | read_only | tool_missing | partial | error | compile` (neu: `compile`, mit `problems`).
- Neue Dateien in `unpacked/<ordner>` und `expanded/<ordner>` nur über `fsutil::write_atomic` (Temp unter `.g4/tmp`, dann `rename`). Ausnahme: rotoms eigener Schreibpfad für `unpacked/scripts/NNNN` und `.rotom/status/compile-state.json` (das ist der DSPRE-Vertrag, Spec §5.1).
- Frontend-Invarianten aus 3a: keine Re-Renders durch Hover oder Meldungen, Panels memoisiert, `useShallowStable` für `WorkbenchData`; wer das Dockview-Panel-Set ändert, zählt `LAYOUT_KEY` hoch (hier: v4 in Task 4, v5 in Task 11).
- Alles mit Byte-Layout wird in `docs/formats/` dokumentiert (Quelle nennen).
- rotom ist 0.x: jede benutzte API-Funktion wird im Code mit ihrem Pfad in rotom genannt (`// rotom v0.7.0: src/project/compile.rs compile_project`), damit ein Update die Stellen findet.

## Review Focus

1. Ein Projekt, das DSPRE oder die rotom-CLI schon übernommen hat (`rotom.toml`, Quellen, `compile-state.json` vorhanden, eine Quelle vom Nutzer geändert): `scripts_status` meldet `adopted`, nichts wird neu dekompiliert, die geänderte Quelle bleibt, Compile läuft (Task 6, Test `adopted_project_is_used_without_touching_sources`).
2. Ein Textarchiv, dessen Member neuer ist als seine JSON (in DSPRE `main` geändert): der Member gewinnt, die JSON wird beim ersten Speichern neu geschrieben, der Editor zeigt den Member-Stand (Task 2, Test `member_newer_than_json_wins`).
3. Ein Compile-Fehler, auch in einer Zeile mit Umlauten vor der Fehlstelle: die Quelle ist gespeichert, der Member unverändert, Zeile und Spalte stimmen (UTF-16-Spalte wie der LSP, 1-basiert) (Task 6, Tests `compile_error_reports_line_and_column` und `compile_error_reports_line_and_column_after_umlauts`).
4. `rotom-lsp` fehlt oder stirbt: Öffnen, Bearbeiten und Speichern eines Scripts funktionieren weiter (Compile ist in-process), die Kopfzeile sagt „rotom-lsp fehlt“ (Task 9, Test `missing_lsp_is_tool_missing`; Task 11, Test `editor_saves_without_lsp`).
5. „Bauen“ mit einem offenen, ungespeicherten Script und einer veralteten Text-JSON: das Script wird zuerst gespeichert, die JSON kodiert; schlägt das Speichern fehl, bleibt der Build aus (Task 7, Test `build_encodes_stale_text_json`; Task 11, Test `build_saves_open_scripts_first_and_stops_on_failure`).

---

## Teil A: Text

### Task 1: Kernel-Modul `text` (chatot-JSON ↔ Member)

**Files:**
- Create: `crates/g4-kernel/src/text.rs`
- Create: `docs/formats/text_archive.md`
- Modify: `crates/g4-kernel/Cargo.toml` (`chatot = "0.8"`)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod text;`)
- Modify: `crates/g4-formats/src/game.rs` (`Language::locale_key`)
- Test: `crates/g4-kernel/src/text.rs` (Unit)

**Interfaces:**
- Consumes: `chatot::{get_default_charmap, decode::decode_archive, encode::{encode_texts, validate_message}, TextSource, BinarySource, Settings, format::line_is_too_long}` (chatot 0.8.1, Spec §2.4); `fsutil::write_atomic(root, path, bytes)`.
- Produces:
  ```rust
  // crates/g4-formats/src/game.rs
  impl Language { pub fn locale_key(self) -> &'static str }   // "en_US" | "fr_FR" | "de_DE" | "it_IT" | "es_ES" | "ja_JP" | "ko_KR"

  // crates/g4-kernel/src/text.rs
  pub struct TextMessage { pub id: String, pub text: String, pub other: serde_json::Map<String, serde_json::Value> }
  pub struct TextArchiveDoc { pub key: u16, pub lang: String, pub messages: Vec<TextMessage> }
  pub enum Severity { Error, Warning }
  pub struct Problem { pub severity: Severity, pub file: PathBuf, pub line: u32, pub col: u32, pub message: String }
  impl TextArchiveDoc {
      /// chatot JSON (`{"key", "messages":[{"id", "<lang>": string | [segments]}]}`); segments are joined.
      pub fn from_json(json: &[u8], lang: &str) -> Result<TextArchiveDoc, KernelError>;
      /// Canonical JSON: pretty, `id` first, then the languages; a text with `\n`, `\r` or `\f` is split into segments like chatot does.
      pub fn to_json(&self) -> Vec<u8>;
      /// Decodes a member with chatot; `id` is `msg_<index:05>` like uxie writes it; `other` is empty.
      pub fn from_member(bytes: &[u8], lang: &str) -> Result<TextArchiveDoc, KernelError>;
  }
  /// Writes `json` (already canonical) to `json_path` atomically, then encodes it with chatot into `member_path` (temp under `<root>/.g4/tmp`, then rename). The JSON is not touched after the member.
  pub fn write_archive(root: &Path, json_path: &Path, member_path: &Path, doc: &TextArchiveDoc) -> Result<Vec<Problem>, KernelError>;
  /// Dry run of the encoder plus the 200 px line check; warnings only.
  pub fn validate(text: &str, file: &Path) -> Vec<Problem>;
  /// `msg_<index:05>`
  pub fn default_message_id(index: usize) -> String;
  ```

- [ ] **Step 1: Failing tests schreiben** (`text.rs`, `#[cfg(test)]`, Fixture-Helfer `crate::test_support::text_archive_bytes`):

```rust
#[test] fn member_round_trips_through_json()            // text_archive_bytes(&["Hi", "Zwei\\nZeilen"]) → from_member → to_json → from_json → gleiche messages; to_json enthält "Zwei\\n" als Segment-Array
#[test] fn json_keeps_foreign_languages()                // JSON mit "de_DE" und "en_US" → from_json(lang "de_DE") → text aus de_DE, other enthält en_US; to_json schreibt beide
#[test] fn write_archive_produces_chatot_bytes()         // write_archive in Tempdir → Member == text_archive_bytes der gleichen Texte (key 0); JSON-mtime >= Member-mtime
#[test] fn write_archive_keeps_key()                     // doc.key = 0x782C → from_member(member).key == 0x782C
#[test] fn validate_warns_on_long_line()                 // 60 × "W" → ein Problem mit Severity::Warning; "Hallo" → leer
#[test] fn locale_key_follows_language()                 // Language::German.locale_key() == "de_DE"
```

- [ ] **Step 2: Run tests, expect compile failure**: `cargo test -p g4-kernel --all-features text::`

- [ ] **Step 3: Implement** `Language::locale_key`, dann `text.rs`: `from_json` mit `serde_json::Value` (Fehler → `KernelError::InvalidProject("textArchives/NNNN.json: …")`), `to_json` mit `serde_json::to_vec_pretty` über eine `Map` in fester Reihenfolge, `from_member` über `decode_archive(get_default_charmap(), &mut Cursor::new(bytes), false)`, `write_archive`: `write_atomic` der JSON; `encode_texts(charmap, &TextSource { txt: Some(vec![json_path]), text_dir: None }, &BinarySource { archive: Some(vec![tmp]), archive_dir: None }, &Settings { json: true, lang, newer_only: false, msgenc_format: false })`, dann `rename(tmp, member_path)`; Warnungen aus `validate` je Nachricht sammeln (chatot druckt seine auf stderr, das ignorieren wir). Die genauen Feldnamen von `TextSource`/`BinarySource`/`Settings` stehen in `chatot-0.8.1/src/lib.rs:18-36`.

- [ ] **Step 4: Run** `cargo test -p g4-kernel --all-features text::` → PASS.

- [ ] **Step 5: Docs** `docs/formats/text_archive.md`: Header (`u16 count`, `u16 key`), Eintragstabelle, Verschlüsselung (`key*0x2FD*(i+1)`, `0x91BD3*(i+1)`, `+0x493D`), Steuercodes, JSON-Form, Quelle chatot 0.8.1 und DSPRE-Report, verifiziert am Platin-Archiv.

- [ ] **Step 6: Commit** `feat(kernel): text module, chatot JSON and member round trip`.

### Task 2: Textarchiv als editierbares Store-Asset

**Files:**
- Modify: `crates/g4-kernel/src/project.rs` (`Location::TextSource`, `asset_location` für `TextArchive`, `expanded_dir(kind)`, `location_names` auf den neuen Lader)
- Modify: `crates/g4-kernel/src/store.rs` (`Loaded::TextArchive`, `is_editable`, `from_bytes`/`to_bytes`, `read_location`, `save`, `text_archive()`, `edit_text_archive`, `create_text_archive`, `text_index`, `search_text`, `text_archive_users`)
- Test: `crates/g4-kernel/src/store.rs` (Unit, `read_only_kinds_are_not_editable` anpassen), `crates/g4-kernel/tests/store.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces:
  ```rust
  // project.rs
  pub enum Location { Member(PathBuf), Slice{..}, HeaderFiles{..},
      /// A text archive: its chatot JSON source and its member. The asset's bytes are the canonical JSON.
      TextSource { json: PathBuf, member: PathBuf } }
  impl Project { pub fn expanded_dir(&self, kind: AssetKind) -> PathBuf }   // <root>/expanded/<unpacked_name>, only TextArchive and Script use it

  // store.rs
  pub struct TextHit { pub archive: u16, pub index: u16, pub excerpt: String }
  impl Store {
      pub fn text_archive(&mut self, index: u16) -> Result<&TextArchiveDoc, KernelError>;
      pub fn edit_text_archive(&mut self, index: u16, label: &str, f: impl FnOnce(&mut TextArchiveDoc)) -> Result<Option<ChangeId>, KernelError>;
      /// Appends a member (key 0, one message "Neuer Text") and its JSON; on disk at once, not undoable.
      pub fn create_text_archive(&mut self) -> Result<u16, KernelError>;
      /// Case-insensitive substring search over every archive; the index is built on first use and dropped per archive on save/reload.
      pub fn search_text(&mut self, query: &str, limit: usize) -> Result<Vec<TextHit>, KernelError>;
      /// Headers whose `text_archive_id` is `index`.
      pub fn text_archive_users(&mut self, index: u16) -> Result<Vec<u16>, KernelError>;
  }
  ```
- Laderegel (Spec §4.1): `read_location(TextSource)` liefert die kanonische JSON: wenn `json` existiert und `mtime(json) >= mtime(member)` → `from_json` → `to_json`; sonst wenn `member` existiert → `from_member` → `to_json`; sonst `None`. `save` schreibt über `text::write_archive`. Die Stale-Prüfung vergleicht wie bisher `read_location` mit `saved_bytes`.

- [ ] **Step 1: Failing tests** (Unit in `store.rs` auf `synthetic_project()`; das Fixture hat 434 Textarchive, Member 433 mit drei Nachrichten):

```rust
#[test] fn text_archive_loads_from_member_without_json()   // text_archive(433).messages[1].text == "Twinleaf Town"; lang == "en_US" (CPUE)
#[test] fn text_archive_edit_save_writes_json_and_member()  // edit: messages[1].text = "Zweiblattdorf"; save → expanded/textArchives/0433.json existiert und enthält "Zweiblattdorf"; from_member(unpacked/textArchives/0433) liest es; is_dirty false
#[test] fn text_archive_undo_restores_text()                 // edit, undo → text wieder "Twinleaf Town", dirty false
#[test] fn member_newer_than_json_wins()                      // JSON mit "Alt" schreiben, dann Member mit "Neu" (text_archive_bytes) mit späterer mtime (filetime oder sleep 1.1 s + touch) → text_archive(433) zeigt "Neu"
#[test] fn json_newer_than_member_wins()                      // umgekehrt → "Alt"
#[test] fn create_text_archive_appends_member_and_json()      // → 434; member_count 435; Datei 0434 und 0434.json existieren; text_archive(434).messages == ["Neuer Text"]
#[test] fn search_text_finds_across_archives()                // search_text("twinleaf", 10) → [TextHit{archive 433, index 1, ..}]
#[test] fn text_archive_users_lists_headers()                 // edit_header(0): text_archive_id = 433 → text_archive_users(433) == [0]; text_archive_users(5) == []
```
Den Test `read_only_kinds_are_not_editable` von `TextArchive` befreien.

- [ ] **Step 2: Run, expect failure**: `cargo test -p g4-kernel --all-features store::text`

- [ ] **Step 3: Implement** nach den Interfaces. `asset_location(TextArchive, i)` → `TextSource { json: expanded_dir/NNNN.json, member: member_path }`. `Loaded::TextArchive(TextArchiveDoc)`; `from_bytes` = `TextArchiveDoc::from_json(bytes, lang)`, `to_bytes` = `to_json`; `lang` = `project.language.locale_key()`. `save`-Zweig `TextSource` ruft `text::write_archive` und hängt die Warnungen an `SaveReport.problems: Vec<Problem>` (neues Feld, Default leer). `location_names` liest über `TextArchiveDoc::from_member` (Member bleibt die Quelle für Ortsnamen).

- [ ] **Step 4: Run** `cargo test -p g4-kernel --all-features` → PASS (auch `tests/store.rs`).

- [ ] **Step 5: Commit** `feat(kernel): text archives are editable store assets with a JSON source`.

### Task 3: Tauri-Befehle für Text

**Files:**
- Create: `apps/studio/src-tauri/src/commands/text.rs`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `apps/studio/src-tauri/src/lib.rs` (`generate_handler!`)
- Modify: `apps/studio/src-tauri/src/commands/project.rs` (`SaveDto.problems`)
- Test: `apps/studio/src-tauri/src/commands/text.rs` (auf `open_synthetic()`)

**Interfaces:**
- Consumes: Task 2.
- Produces:
  ```rust
  #[derive(Serialize)] pub struct ProblemDto { pub severity: &'static str /* "error" | "warning" */, pub file: String, pub line: u32, pub col: u32, pub message: String, pub source: &'static str /* "compile" | "text" */ }
  #[derive(Serialize)] pub struct TextMessageDto { pub id: String, pub text: String }
  #[derive(Serialize)] pub struct TextArchiveDto { pub index: u16, pub key: u16, pub lang: String, pub messages: Vec<TextMessageDto>, pub users: Vec<u16> }
  #[derive(Deserialize)] pub enum TextEdit { Set { index: u16, text: String }, Append, RemoveLast }
  pub fn get_text_archive_impl(state, index: u16) -> Result<TextArchiveDto, CmdError>;
  pub fn edit_text_archive_impl(state, index: u16, edit: TextEdit) -> Result<Option<ChangeId>, CmdError>;
  pub fn create_text_archive_impl(state) -> Result<u16, CmdError>;
  pub fn search_text_impl(state, query: String, limit: usize) -> Result<Vec<TextHit>, CmdError>;
  pub fn validate_text_impl(text: String) -> Vec<ProblemDto>;
  ```
  Tauri-Namen: `get_text_archive`, `edit_text_archive`, `create_text_archive`, `search_text`, `validate_text`. `save` liefert `SaveDto { written, problems }`.

- [ ] **Step 1: Failing tests**: `get_text_archive_returns_messages_and_users`, `edit_set_then_save_writes_json`, `append_and_remove_last_change_count`, `search_text_hits`, `validate_text_warns_on_overflow`.
- [ ] **Step 2: Run, expect failure**: `cargo test -p studio text::`
- [ ] **Step 3: Implement** (dünne Wrapper, Muster `patches.rs`).
- [ ] **Step 4: Run** → PASS; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] **Step 5: Commit** `feat(studio): text archive commands`.

### Task 4: Panel „Text“

**Files:**
- Create: `apps/studio/src/workbench/text/TextPanel.tsx`, `apps/studio/src/workbench/text/textModel.ts` (reine Funktionen: Filter, Segmentanzeige, Dirty-Hinweis), `apps/studio/src/workbench/text/useTextArchive.ts`
- Modify: `apps/studio/src/api.ts` (DTOs, `getTextArchive`, `editTextArchive`, `createTextArchive`, `searchText`, `validateText`; `SaveReport.problems`)
- Modify: `apps/studio/src/assets.ts` (unverändert „Textarchiv“), `apps/studio/src/reload.ts` (`case "TextArchive": [{ kind: "textPanel", archive }]`), `apps/studio/src/bus/editBus.ts` (`{ topic: "textData"; archive: number }`), `apps/studio/src/app/useHistoryReload.ts`
- Modify: `apps/studio/src/workbench/layout.ts` (`PanelId` + `"text"`, `DEFAULT_PANELS` Eintrag `{ id: "text", title: "Text", position: { direction: "within", referencePanel: "world" } }`, `LAYOUT_KEY = "g4.layout.v4"`), `apps/studio/src/workbench/Workbench.tsx` (`COMPONENTS.text`), `apps/studio/src/workbench/panels.tsx` (`TextTab`)
- Modify: `apps/studio/src/workbench/terms.ts` (`textArchive`, `message`), `apps/studio/src/workbench/navigation.ts` (`{ kind: "textArchive"; index: number; message?: number }` → `panel: "text"`, pinned bleibt)
- Test: `apps/studio/src/workbench/text/TextPanel.test.tsx`, `textModel.test.ts`, `layout.test.ts`, `reload.test.ts`, `navigation.test.ts`, `terms.test.ts`

**Interfaces:**
- Consumes: Task 3.
- Produces: `WorkbenchData.textTarget: { index: number; message: number | null } | null` und `onOpenText(index, message?)` (gesetzt von `goTo` für `textArchive`); Bus-Topic `textData`; `TERMS.textArchive.label === "Textarchiv"`, `TERMS.message.label === "Nachricht"`.
- Panel-Aufbau (Spec §8.2): links Archivliste (`Textarchiv 0199 · 12 Nachrichten · Header 0415`), Suchfeld oben (ruft `searchText` mit 300 ms Debounce, Treffer „0199 · 12 · Ausschnitt“, Klick wählt), rechts Nachrichtenliste (Index, erste Zeile, `\n` `\r` `\f` als `⏎`/`▼`/`▽` sichtbar), darunter `<textarea>` für die Auswahl mit `validateText`-Warnung (Debounce 300 ms); Knöpfe „Nachricht anfügen“, „Letzte entfernen“, „Neues Archiv“. Jede Textänderung wird beim Blur oder nach 500 ms Ruhe als `editTextArchive(Set)` committet (eine Undo-Stufe je Commit); `useBusRevision(bus, "textData")` und `headerTable` laden neu.

- [ ] **Step 1: Failing tests**: `textModel.test.ts` (`visibleLine("a\\nb") === "a⏎b"`, `filterArchives`); `TextPanel.test.tsx` (jsdom; `api` gemockt: zeigt Archive, Klick lädt Nachrichten, Tippen + Blur ruft `editTextArchive(433, {Set…})`, Suche ruft `searchText` und Klick auf Treffer wählt Archiv 433/Nachricht 1, „Neues Archiv“ ruft `createTextArchive` und wählt den neuen); `layout.test.ts` (v3-Layout wird verworfen, `text` ist Default-Panel); `reload.test.ts` (`TextArchive` → `textPanel`); `navigation.test.ts` (`textArchive` → `panel: "text"`).
- [ ] **Step 2: Run, expect failure**: `pnpm test -- text layout reload navigation`
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat(studio): text panel with search, append and new archive`.

## Teil B: Scripts

### Task 5: rotom-Abhängigkeit, Befehlsdatenbank, Script-Archiv, Fixture

**Files:**
- Modify: `crates/g4-kernel/Cargo.toml` (`rotom = { git = "https://github.com/KalaayPT/rotom", tag = "v0.7.0" }`, `toml = "0.8"`)
- Create: `crates/g4-kernel/metadata/scrcmd/diamond_pearl_v2.json`, `platinum_v2.json`, `hgss_v2.json`, `SOURCE.md`
- Modify: `crates/g4-kernel/metadata/archives.yaml` (je Familie `scripts:`), `crates/g4-kernel/src/project.rs` (`AssetKind::Script`, `ALL` 12, `label` "script file", `slot`, `ArchiveMap.entries` 12, `FamilyArchives.scripts`, `for_game`)
- Modify: `crates/g4-kernel/src/store.rs` (`from_bytes`: `Script` ist `NotEditable`), `crates/g4-kernel/src/test_support.rs` (Script-NARC im Fixture)
- Modify: `apps/studio/src/api.ts` (`AssetKind` + `"Script"`), `apps/studio/src/assets.ts` (`Script: "Scriptdatei"`), `apps/studio/src/reload.ts` (`Script` → `[]`), `apps/studio/src/assets.test.ts`, `apps/studio/src-tauri/src/commands/project.rs` (Test `member_counts_cover_every_archive…` erwartet `Script == 3`)
- Test: `crates/g4-kernel/src/project.rs` (Unit), `crates/g4-kernel/tests/build.rs`

**Interfaces:**
- Produces: `AssetKind::Script` (store-only wie `DynamicHeaders`: Build packt `unpacked/scripts`, Store editiert nicht). `archives.yaml`: dp `files/fielddata/script/scr_seq_release.narc` (Kommentar `# unverified: no DP project at hand; DSPRE RomInfo`), pt `files/fielddata/script/scr_seq.narc`, hgss `files/a/0/1/2`, unpacked `scripts`. `pub fn scrcmd_database(family: Family) -> &'static str` in `scripts.rs` (Task 6) liest die eingebetteten JSONs; hier nur die Dateien. Fixture: `files/fielddata/script/scr_seq.narc` mit drei Membern — Member 0 kompiliert aus
  ```
  script Main #1:
      Message 1
      End
  script Second #2:
      End
  ```
  (per `rotom::compile(src, &db, &constants, BinaryQuirk::default())` im Fixture-Code mit der leeren `ConstantDb`; wie sie entsteht — `Default` oder `new()` — steht in rotom `src/database.rs`/`src/compiler/constants.rs`), Member 1 = Level-Script `[0,0,0,0]`, Member 2 = leer. Header 0: `script_file_id 0`, `level_script_id 1`, `text_archive_id 433` (arm9-Bytes bei Offset 4, 6, 8 des Headers setzen).
- `SOURCE.md`: Repo `DS-Pokemon-Rom-Editor/scrcmd-database`, Asset `db-latest.zip`, `revision.txt` = 33, Download-Datum; „Dieselbe Revision bettet rotom v0.7.0 beim Bauen ein.“ Download: `curl -L -o /tmp/db-latest.zip https://github.com/DS-Pokemon-Rom-Editor/scrcmd-database/releases/latest/download/db-latest.zip` in ein leeres Scratch-Verzeichnis, `unzip`, `revision.txt` prüfen; ist sie nicht 33, die vorliegende Revision eintragen (die Spec nennt 33 als Stand vom 2026-10-22).

- [ ] **Step 1: Failing tests**:
```rust
// project.rs
#[test] fn script_archive_is_known_per_family()      // ArchiveMap::for_game(Pt, English).packed(Script) == "files/fielddata/script/scr_seq.narc"; Hgss → "files/a/0/1/2"
#[test] fn scrcmd_databases_load()                   // rotom::DatabaseV2::load für die drei eingebetteten Dateien (über include_str! in ein Tempfile) → Ok, game_family passt
// tests/build.rs
#[test] fn build_packs_scripts_folder()              // synthetic: ensure_unpacked(Script) → unpacked/scripts/0000..0002; build → scr_seq.narc byte-identisch zur Fixture-Datei
```
- [ ] **Step 2: Run, expect failure**: `cargo test -p g4-kernel --all-features script`
- [ ] **Step 3: Implement**; `cargo build` zieht rotom (erster Build braucht Netz: rotoms `build.rs`). Prüfen, dass `cargo tree -p g4-kernel -i uxie` genau eine uxie-Version (0.9.7) zeigt; sonst `uxie = "0.9.7"` pinnen und melden.
- [ ] **Step 4: Run** `cargo test --workspace --all-features`, `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat(kernel): rotom dependency, scrcmd database, script archive kind`.

### Task 6: Kernel-Modul `scripts` (Übernehmen, Compile, Quellen, Auflösung)

**Files:**
- Create: `crates/g4-kernel/src/scripts.rs`
- Create: `docs/formats/script_file.md`, `docs/formats/rotom_project.md`
- Modify: `crates/g4-kernel/src/lib.rs`, `crates/g4-kernel/src/error.rs` (`Compile(Vec<Problem>)`: „{n} problem(s) in scripts or texts“)
- Test: `crates/g4-kernel/src/scripts.rs` (Unit), `crates/g4-kernel/tests/scripts.rs` (gated)

**Interfaces:**
- Consumes: Task 1 (`Problem`, `Severity`), Task 5. rotom v0.7.0: `project::config::{RotomConfig, ProjectMetadata, WorkspaceConfig, PathsConfig, DatabaseConfig, ProjectTypeConfig::Dspre, load_config}`, `project::compile::{compile_project, decompile_project}`, `ProjectContext`, `LevelScript`, `SourceMap`, `compiler::{Lexer, Parser}`; uxie `script_file::resolve_script_id`, `GlobalScriptTable`.
- Produces:
  ```rust
  pub const ROTOM_TOML: &str = "rotom.toml";
  pub fn is_adopted(project: &Project) -> bool;
  pub struct AdoptProgress { pub done: u32, pub total: u32 }
  pub struct AdoptReport { pub decompiled: u32, pub skipped_empty: u32, pub failed: Vec<(u16, String)> }
  /// Writes rotom.toml and the command database, then decompiles every member. Refuses when rotom.toml exists or `expanded/scripts/*.script` exist.
  pub fn adopt(project: &Project, progress: &mut dyn FnMut(AdoptProgress)) -> Result<AdoptReport, KernelError>;
  pub struct CompileReport { pub compiled: Vec<u16>, pub problems: Vec<Problem>, pub touched_archives: Vec<u16> }
  /// `rotom::project::compile::compile_project(root, &config, false)`: everything stale per compile-state. Spans → 1-based line, UTF-16 column.
  pub fn compile(project: &Project) -> Result<CompileReport, KernelError>;
  pub enum SourceKind { Script, LevelScript, Binary }
  pub enum ScriptSource { Rotom(String), LevelScript(LevelScriptDoc), Binary { size: u64 } }
  pub type LevelScriptDoc = rotom::LevelScript;   // serde: header_entries (type on_load|on_resume|on_transition|on_var_condition, script_id), var_conditions (var, value, script_id)
  pub struct ScriptFileInfo { pub file: u16, pub kind: SourceKind, pub size: u64 }
  pub fn list_files(project: &Project) -> Result<Vec<ScriptFileInfo>, KernelError>;
  pub fn source_path(project: &Project, file: u16, kind: SourceKind) -> PathBuf;   // expanded/scripts/NNNN.rotom | NNNN.json
  pub fn read_source(project: &Project, file: u16) -> Result<ScriptSource, KernelError>;
  /// Writes the source atomically; does not compile.
  pub fn write_source(project: &Project, file: u16, source: &ScriptSource) -> Result<(), KernelError>;
  /// Next member number; source "script Main #1:\n\tEnd" (Script) or an empty LevelScript JSON; writes source and compiles it so the member exists.
  pub fn new_script_file(project: &Project, kind: SourceKind) -> Result<u16, KernelError>;
  /// Appends "\nscript Main #N:\n\tEnd\n" with N = highest `#n` + 1, writes and compiles; returns N.
  pub fn add_script(project: &Project, file: u16) -> Result<u16, KernelError>;
  pub enum ScriptTarget { File { file: u16, line: u32 }, Ambiguous(Vec<u16>), Unknown }
  pub fn resolve_script(project: &Project, headers: &[MapHeader], header: u16, n: u16) -> Result<ScriptTarget, KernelError>;
  pub struct SearchHit { pub file: u16, pub line: u32, pub excerpt: String }
  pub fn search_sources(project: &Project, query: &str, limit: usize) -> Result<Vec<SearchHit>, KernelError>;
  pub fn scrcmd_database(family: Family) -> (&'static str /* file name */, &'static str /* json */);
  /// 1-based (line, column) of a byte offset; the column counts UTF-16 units like the LSP (`rotom::SourceMap::byte_to_position` + 1).
  pub fn position_of(source: &str, byte: usize) -> (u32, u32);
  ```
- `adopt`: `RotomConfig { format_version: 1, project: ProjectMetadata { name: <Ordnername> }, workspace: WorkspaceConfig { project_type: Dspre, game_family: Some(DP|Platinum|HGSS) }, paths: PathsConfig { database_dir: ".rotom/command_database", cache_dir: ".rotom/cache", status_dir: ".rotom/status", source_roots: ["expanded/scripts"], include_roots: [], binary_roots: ["unpacked/scripts"] }, database: Some(DatabaseConfig { default_file: "<family>_v2.json" }) }` per `toml::to_string_pretty` → `write_atomic`. Vorher `project.ensure_unpacked(Script)`. Existiert `unpacked/Scripts` und nicht `unpacked/scripts`, wird `binary_roots = ["unpacked/Scripts"]`. Dann `decompile_project`; Fortschritt: rotom bietet keinen Callback für `decompile_project`, also `progress` einmal mit `done = 0` vor und einmal mit `done = total` nach dem Aufruf (die Fortschrittsleiste zeigt „läuft …“). `failed` aus `BatchDecompileResult.failures` (Pfad → Member-Nummer), `skipped_empty` = Member mit 0 Byte.
- `compile`: Hash aller `expanded/textArchives/*.json` vorher/nachher → `touched_archives`. `Problem.file` = Pfad relativ zur Projektwurzel. `CompileError`-Varianten ohne Span → Zeile 1, Spalte 1. `BatchCompileResult.failures` → `Severity::Error`, `successes[].warnings` → `Severity::Warning` (Warnungstext aus `Display`; Span, wenn die Variante einen hat, sonst 1:1).
- `resolve_script`: `n < 2000` → Datei `headers[header].script_file_id`, Zeile aus `Parser::new_fallible(Lexer::new(src))`-AST: erstes `script`-Statement mit Slot `#n` (fehlt es: Zeile 1); `n >= 2000` → uxie `resolve_script_id` mit `GlobalScriptTable` der Familie (HGSS aus `arm9.bin` über uxie), Mehrdeutigkeit → `Ambiguous`, DP → `Unknown`.

- [ ] **Step 1: Failing tests** (Unit auf `synthetic_project()`; `Project::open`):
```rust
#[test] fn adopt_writes_config_database_sources_and_state()   // rotom.toml parst mit load_config; .rotom/command_database/platinum_v2.json == eingebettete Datei; expanded/scripts/0000.rotom enthält "Main"; 0001.json ist ein LevelScript; kein 0002.*; .rotom/status/compile-state.json existiert; report.decompiled == 2, skipped_empty == 1
#[test] fn adopt_refuses_when_already_adopted()                 // zweiter Aufruf → Err(InvalidProject), nichts verändert
#[test] fn adopt_refuses_dspre_script_exports()                 // expanded/scripts/0000.script anlegen → Err nennt ".script"
#[test] fn adopted_project_is_used_without_touching_sources()   // adopt; 0000.rotom ändern (Kommentar anhängen); is_adopted; compile → compiled == [0]; Quelle unverändert; Member neu
#[test] fn compile_after_adopt_reproduces_members()             // adopt; Member-Bytes vorher merken; Quelle 0000 touch (mtime) → compile → Bytes identisch
#[test] fn compile_error_reports_line_and_column()             // Quelle "script Main #1:\n\t// Kommentar\n\tNoSuchCommand 3\n\tEnd\n" → compile: problems[0].line == 3, col == 2 (Tab = 1 Zeichen), message enthält "NoSuchCommand"; Member unverändert
#[test] fn compile_error_reports_line_and_column_after_umlauts() // position_of("\tMessage \"Äöü\" x", byte_offset_of('x')) == (1, 15): UTF-16-Spalte, 1-basiert; die Umlaute sind je 2 UTF-8-Bytes, aber 1 UTF-16-Einheit
#[test] fn text_literal_appends_to_the_header_archive()         // Quelle mit `Message "Hallo Welt"`; compile → touched_archives == [433]; 0433.json enthält "Hallo Welt"; Member 0433 kodiert
#[test] fn level_script_round_trips()                           // read_source(1) → LevelScript leer; write_source mit on_load → script_id 1; compile → Member != [0,0,0,0]; read_source zeigt den Eintrag
#[test] fn new_script_file_and_add_script()                     // new_script_file(Script) == 3; Member 0003 existiert; add_script(3) == 2; Quelle enthält "#2"
#[test] fn resolve_local_common_and_unknown()                   // resolve(header 0, n 2) → File{0, line 4}; n 3000 (Pt-Trainer) → File{file aus uxie-Tabelle, ..}; n 1 → File{0, 1}
#[test] fn search_sources_finds_text()                          // search_sources("second", 10) → [SearchHit{file 0, line 4, ..}]
```
- [ ] **Step 2: Run, expect failure**: `cargo test -p g4-kernel --all-features scripts::`
- [ ] **Step 3: Implement** `scripts.rs` nach den Interfaces (jede rotom-Funktion mit Pfadkommentar, siehe Global Constraints).
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Gated test** `crates/g4-kernel/tests/scripts.rs` (`#![cfg(feature = "test-support")]`, `vanilla_copy()`): `adopt` → `failed.is_empty()` (schlägt es für ein Projekt fehl, die Nummern als `assert_eq!` mit Projektkommentar festschreiben, nicht abschwächen); danach `compile` → `compiled.is_empty()` (nichts veraltet) und `problems.is_empty()`; `resolve_script(.., 3000)` → `File`.
- [ ] **Step 6: Run gated** für CPUD und IPKD → PASS.
- [ ] **Step 7: Docs** `script_file.md` (Sprungtabelle, `0xFD13`, Level-Script-Layout aus rotom `levelscript.rs`, Common-Script-Bereiche), `rotom_project.md` (Layout, `rotom.toml`, compile-state, DB-Herkunft).
- [ ] **Step 8: Commit** `feat(kernel): scripts module, rotom project adoption, compile and resolution`.

### Task 7: Build-Vorschritte

**Files:**
- Modify: `crates/g4-kernel/src/build.rs` (`build`: Vorschritte; `BuildReport.problems`), `crates/g4-kernel/src/text.rs` (`encode_stale`)
- Test: `crates/g4-kernel/tests/build.rs` (Unit auf Fixture, gated nach `adopt`)

**Interfaces:**
- Consumes: Task 2, Task 6.
- Produces:
  ```rust
  // text.rs
  /// Every expanded/textArchives/NNNN.json whose member is missing or older (mtime) is encoded like on save.
  pub fn encode_stale(project: &Project) -> Result<(Vec<u16>, Vec<Problem>), KernelError>;
  // build.rs
  pub struct BuildReport { pub packed: Vec<(AssetKind, usize)>, pub rom: PathBuf, pub compiled: Vec<u16>, pub encoded: Vec<u16>, pub problems: Vec<Problem> }
  ```
  Ablauf in `build`: nach `reject_hidden_files`: `if scripts::is_adopted(project) { scripts::compile(project)? }`, dann `text::encode_stale(project)?`; enthält eine der Listen ein `Severity::Error` → `Err(KernelError::Compile(problems))` **vor** der Pack-Schleife. Warnungen landen in `BuildReport.problems`.

- [ ] **Step 1: Failing tests**:
```rust
#[test] fn build_compiles_stale_scripts_before_packing()   // adopt; Quelle 0000 ändern (Message 2); build → compiled == [0]; NARC enthält neuen Member
#[test] fn build_encodes_stale_text_json()                  // JSON 0433 mit "Neu" schreiben (mtime neuer als Member) → build → encoded == [433]; Member kodiert "Neu"; NARC enthält ihn
#[test] fn build_with_compile_error_writes_nothing()        // Quelle mit Fehler → build Err(Compile(problems)) mit problems[0].line; kein ROM, scr_seq.narc unverändert
#[test] fn vanilla_rebuild_matches_dsrom_cli_after_adopt()  // gated: wie vanilla_rebuild_matches_dsrom_cli, aber adopt(vanilla_copy) vor build; ROM byte-identisch
```
- [ ] **Step 2: Run, expect failure**: `cargo test -p g4-kernel --all-features --test build`
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** Unit → PASS; gated (CPUD, IPKD, IPKE) → PASS; `g4k build` unverändert benutzbar.
- [ ] **Step 5: Commit** `feat(kernel): build compiles stale scripts and encodes stale text before packing`.

### Task 8: Tauri-Befehle für Scripts und Fehlercode `compile`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/scripts.rs`
- Modify: `apps/studio/src-tauri/src/errors.rs` (`code: "compile"`, `problems: Vec<ProblemDto>`), `apps/studio/src-tauri/src/commands/project.rs` (`BuildDto.problems`, `BuildDto.compiled`), `commands/mod.rs`, `lib.rs`
- Modify: `apps/studio/src/api.ts` (`CmdError.code` um `"compile"`, `problems`, `isCmdError`), `apps/studio/src/api.test.ts`
- Test: `apps/studio/src-tauri/src/errors.rs`, `apps/studio/src-tauri/src/commands/scripts.rs`

**Interfaces:**
- Consumes: Task 6, Task 7, `ProblemDto` (Task 3).
- Produces (Tauri-Namen = Funktionsnamen ohne `_impl`):
  ```rust
  pub struct ScriptsStatusDto { pub adopted: bool, pub file_count: u16 }
  pub fn scripts_status_impl(state) -> Result<ScriptsStatusDto, CmdError>;
  /// spawn_blocking; progress over the channel; the state lock is held while rotom writes.
  pub async fn adopt_scripts(state, progress: Channel<AdoptProgressDto>) -> Result<AdoptReportDto, CmdError>;
  pub struct ScriptFileDto { pub file: u16, pub kind: &'static str /* "script" | "levelScript" | "binary" */, pub size: u64, pub headers: Vec<u16> }
  pub fn list_script_files_impl(state) -> Result<Vec<ScriptFileDto>, CmdError>;
  pub enum ScriptSourceDto { Rotom { text: String }, LevelScript { doc: LevelScriptDoc }, Binary { size: u64 } }  // serde tag "kind"
  pub fn read_script_impl(state, file: u16) -> Result<ScriptSourceDto, CmdError>;
  pub struct SaveScriptDto { pub problems: Vec<ProblemDto>, pub touched_archives: Vec<u16> }
  /// Refuses with code "error" and message "Textarchiv NNNN erst speichern" when any text archive is dirty; writes the source, compiles, reloads touched archives in the store.
  pub fn save_script_impl(state, file: u16, text: String) -> Result<SaveScriptDto, CmdError>;
  pub fn save_level_script_impl(state, file: u16, doc: LevelScriptDoc) -> Result<SaveScriptDto, CmdError>;
  pub fn new_script_file_impl(state, kind: String /* "script" | "levelScript" */) -> Result<u16, CmdError>;
  pub fn add_script_impl(state, file: u16) -> Result<u16, CmdError>;
  pub enum ScriptTargetDto { File { file: u16, line: u32 }, Ambiguous { candidates: Vec<u16> }, Unknown }
  pub fn resolve_script_impl(state, header: u16, n: u16) -> Result<ScriptTargetDto, CmdError>;
  pub fn search_scripts_impl(state, query: String, limit: usize) -> Result<Vec<SearchHit>, CmdError>;
  ```
  `CmdError::from(KernelError::Compile(p))` → `code: "compile"`, `problems` gefüllt, `message` „3 Fehler in Scripts oder Texten“. `BuildDto { saved, packed, rom, compiled: Vec<u16>, encoded: Vec<u16>, problems }`.

- [ ] **Step 1: Failing tests**: `errors.rs`: `compile_error_maps_to_code_compile_with_problems`; `scripts.rs`: `status_before_and_after_adopt`, `save_script_compiles_and_reports_problems`, `save_script_refuses_with_dirty_text_archive` (Text 433 editieren, nicht speichern → Err message enthält "0433"), `save_script_with_literal_reloads_archive` (Text 433 nicht dirty; nach Save enthält `get_text_archive(433)` den Literaltext), `save_level_script_round_trip`, `resolve_script_dto`; `api.test.ts`: `isCmdError` akzeptiert `compile` mit `problems`.
- [ ] **Step 2: Run, expect failure**: `cargo test -p studio scripts:: errors::`
- [ ] **Step 3: Implement.** `touched_archives` → `store.reload_from_disk(AssetId { kind: TextArchive, index })` je Archiv.
- [ ] **Step 4: Run** → PASS; `pnpm test`.
- [ ] **Step 5: Commit** `feat(studio): script commands and the compile error code`.

### Task 9: Sidecar `rotom-lsp`

**Files:**
- Create: `apps/studio/src-tauri/src/lsp.rs`
- Modify: `apps/studio/src-tauri/src/state.rs` (`Session.lsp: Option<lsp::LspProcess>`), `apps/studio/src-tauri/src/lib.rs` (`lsp_start`, `lsp_send`, `lsp_stop`; Stop bei `ExitRequested`), `apps/studio/src-tauri/src/commands/project.rs` (`open_project_impl` stoppt den alten LSP), `crates/g4-kernel/src/export.rs` (`kill_tree` → `pub fn`)
- Modify: `apps/studio/src-tauri/tauri.conf.json` (`externalBin` + `"binaries/rotom-lsp"`), `apps/studio/src-tauri/build.rs` (Platzhalter `binaries/rotom-lsp-{triple}`), `apps/studio/scripts/prepare-bundle.sh` (baut `rotom-lsp`, kopiert es, dylib rpath), `apps/studio/scripts/check-bundle.sh` (`test -x …/rotom-lsp`), `scripts/gate.sh` (`G4_ROTOM_LSP` Default `tools/rotom/bin/rotom-lsp`, `env -u`, Preflight, Projektschleife), `docs/dev-setup.md` (Abschnitt „rotom und rotom-lsp“)
- Test: `apps/studio/src-tauri/src/lsp.rs` (Unit mit Fake-LSP-Skript)

**Interfaces:**
- Produces:
  ```rust
  // lsp.rs
  pub fn find_rotom_lsp() -> Option<PathBuf>;   // G4_ROTOM_LSP, next to current_exe, debug: <manifest>/../../../tools/rotom/bin/rotom-lsp; empty files do not count
  pub struct LspProcess { /* child, stdin: Mutex<ChildStdin>, reader thread, restarts: u8, last_start: Instant */ }
  impl LspProcess {
      /// Spawns `rotom-lsp` with cwd = root, stdio piped; the reader thread parses Content-Length frames and sends each body to `sink`.
      pub fn start(binary: &Path, root: &Path, sink: tauri::ipc::Channel<String>) -> Result<LspProcess, KernelError>;
      pub fn send(&self, body: &str) -> Result<(), KernelError>;   // writes "Content-Length: N\r\n\r\n" + body
      pub fn stop(&mut self);                                       // kill_tree, join
      pub fn alive(&mut self) -> bool;
  }
  pub fn frame(body: &str) -> Vec<u8>;
  pub fn parse_frames(buf: &mut Vec<u8>) -> Vec<String>;            // pure: drains complete frames
  // commands
  pub async fn lsp_start(state, channel: Channel<String>) -> Result<(), CmdError>;   // tool_missing when find_rotom_lsp is None
  pub fn lsp_send(state, body: String) -> Result<(), CmdError>;
  pub fn lsp_stop(state) -> Result<(), CmdError>;
  ```
  Neustart: `lsp_send` auf einen toten Prozess startet ihn neu, wenn seit dem letzten Start ≥ 60 s vergangen sind, sonst `Err("rotom-lsp beendet sich wiederholt")`.
- `prepare-bundle.sh`: `cargo install --git https://github.com/KalaayPT/rotom --tag v0.7.0 rotom-lsp --root "$ROOT/tools/rotom"` (nur wenn `tools/rotom/bin/rotom-lsp` fehlt), `install -m 755` nach `binaries/rotom-lsp-$TRIPLE`, `install_name_tool -add_rpath @executable_path/../Frameworks` auf das Binary (falls noch nicht vorhanden), `codesign --force --sign -`. `docs/dev-setup.md`: derselbe Befehl von Hand, `libnitroarc_ffi.dylib` neben das Binary kopieren (`target/release/` von `cargo build --release -p studio`), `G4_ROTOM_LSP`, Netz beim ersten Build (rotoms `build.rs`), LGPL-Hinweis.

- [ ] **Step 1: Failing tests** (`lsp.rs`): `frames_round_trip` (`parse_frames(frame("{}"))`), `partial_frame_waits`, `fake_lsp_answers_initialize` (Shell-Skript liest einen Frame, schreibt `Content-Length`-Antwort `{"id":1,"result":{}}`; `start` mit Fake-`Channel` — Tauri `Channel::new(|msg| …)` — empfängt sie), `missing_lsp_is_tool_missing` (`G4_ROTOM_LSP` auf Pfad ohne Datei → `lsp_start` Err code `tool_missing`), `stop_kills_the_process_group`.
- [ ] **Step 2: Run, expect failure**: `cargo test -p studio lsp::`
- [ ] **Step 3: Implement** inkl. Bundle-Skripte, `gate.sh`, Doku.
- [ ] **Step 4: Run** → PASS; `bash apps/studio/scripts/prepare-bundle.sh` lokal einmal ausführen (baut `rotom-lsp`), `tools/rotom/bin/rotom-lsp` startet (`echo | tools/rotom/bin/rotom-lsp` endet ohne dyld-Fehler; sonst dylib daneben legen und rpath setzen, wie dokumentiert).
- [ ] **Step 5: Commit** `feat(studio): rotom-lsp sidecar with stdio bridge over a Tauri channel`.

### Task 10: Editor-Kern (CodeMirror, rotom-Sprache, LSP-Transport)

**Files:**
- Create: `apps/studio/src/workbench/script/rotomLanguage.ts`, `apps/studio/src/workbench/script/tauriTransport.ts`, `apps/studio/src/workbench/script/lspExtras.ts`, `apps/studio/src/workbench/script/ScriptEditor.tsx`, `apps/studio/src/workbench/script/documents.ts`
- Modify: `apps/studio/package.json` (`codemirror`, `@codemirror/state`, `@codemirror/view`, `@codemirror/language`, `@codemirror/lang-json`, `@codemirror/autocomplete`, `@codemirror/lint`, `@codemirror/lsp-client`, `@codemirror/theme-one-dark`, Versionen aus Spec §2.6 oder neuer), `apps/studio/src/api.ts` (`lspStart(onMessage)`, `lspSend`, `lspStop`)
- Test: `rotomLanguage.test.ts`, `tauriTransport.test.ts`, `lspExtras.test.ts`, `ScriptEditor.test.tsx`, `documents.test.ts`

**Interfaces:**
- Produces:
  ```ts
  // rotomLanguage.ts
  export const rotomLanguage: StreamLanguage<unknown>;   // keywords script|function|action|if|else|while|match, commands (Capitalised identifiers), `#N`, numbers, strings, // comments, #include/#define
  export function tokenize(line: string): { text: string; style: string }[];   // for tests
  // tauriTransport.ts
  export class TauriTransport implements Transport { constructor(send: (s: string) => Promise<void>, subscribe: (cb: (s: string) => void) => () => void) }
  export function createTauriTransport(): Promise<TauriTransport>;   // lspStart + lspSend from api.ts
  // lspExtras.ts
  export function documentSymbols(client: LSPClient, uri: string): Promise<{ name: string; detail?: string; line: number }[]>;
  export const inlayHints: Extension;   // decorations from textDocument/inlayHint, debounced 300 ms
  export const codeLenses: Extension;   // line widgets above scripts, display only
  // documents.ts
  export interface ScriptDocument { file: number; kind: "script" | "levelScript"; text: string; savedText: string; state: EditorState | null; cursor: number }
  export class ScriptDocuments { open(file, kind, text): ScriptDocument; get(file): ScriptDocument | undefined; setText(file, text): void; markSaved(file): void; close(file): void; dirty(): number[]; list(): ScriptDocument[] }
  // ScriptEditor.tsx
  export function ScriptEditor(props: { doc: ScriptDocument; language: "rotom" | "json"; lsp: LSPClient | null; uri: string; onChange: (text: string) => void; onSave: () => void; onDiagnostics: (d: Diagnostic[]) => void; theme: "light" | "dark" }): JSX.Element;
  ```
  Cmd+S/Ctrl+S im Editor ruft `onSave`. `uri` = `file://<root>/expanded/scripts/NNNN.rotom`. Der Editor hält die `EditorView` im `ScriptDocument.state`, so dass Undo je Dokument beim Wechsel bleibt.

- [ ] **Step 1: Failing tests**: `rotomLanguage.test.ts` (`tokenize("script Main #1:")` → keyword, variableName, number-ish für `#1`; `Message "Hi" // c` → command, string, comment); `tauriTransport.test.ts` (send ruft `lspSend`, subscribe liefert Channel-Nachrichten); `lspExtras.test.ts` (Fake-Client liefert zwei Symbole → `documentSymbols` mit Zeilen); `documents.test.ts` (open/setText/dirty/markSaved/close); `ScriptEditor.test.tsx` (jsdom: mountet, Tippen ruft `onChange`, Mod-S ruft `onSave`, LSP-Round-Trip gegen Fake-Transport setzt eine Diagnose → `onDiagnostics`).
- [ ] **Step 2: Run, expect failure**: `pnpm test -- script`
- [ ] **Step 3: Implement**; `pnpm add` der Pakete; `csp.test.ts` bleibt unverändert grün.
- [ ] **Step 4: Run** `pnpm test`, `pnpm build` → PASS; Bundle-Größe des Editor-Chunks in der Build-Ausgabe notieren (Erwartung ≈ 160–200 KB gzip).
- [ ] **Step 5: Commit** `feat(studio): CodeMirror script editor with rotom language and LSP transport`.

### Task 11: Panels „Script“ und „Probleme“, Übernehmen, Build-Fluss

**Files:**
- Create: `apps/studio/src/workbench/script/ScriptPanel.tsx`, `apps/studio/src/workbench/script/AdoptBox.tsx`, `apps/studio/src/workbench/script/useScripts.ts` (Status, Dokumente, LSP-Client, Save), `apps/studio/src/workbench/problems/ProblemsPanel.tsx`, `apps/studio/src/workbench/problems/problems.ts` (Store: `replaceForFile`, `replaceLive`, `replaceText`, `counts`)
- Modify: `apps/studio/src/workbench/layout.ts` (`PanelId` + `"script" | "problems"`; `script` `within world`, `problems` `within status`; `LAYOUT_KEY = "g4.layout.v5"`), `Workbench.tsx`, `panels.tsx`, `WorkbenchContext.tsx` (`WorkbenchData.scripts`, `problems`), `apps/studio/src/app/useWorkbenchModel.ts`, `apps/studio/src/app/useProjectSession.ts` (`onBuild`: erst `saveScript` je dirty Dokument, bei Fehler abbrechen; `compile`-Fehler → Probleme-Store, Meldung „Build abgebrochen: N Fehler“), `apps/studio/src/Toolbar.tsx` (Dirty-Zähler zählt offene Scripts mit; Statuszeile „2 Fehler, 1 Warnung“), `apps/studio/src/workbench/navigation.ts` (`{ kind: "script"; file: number; line?: number }` → `panel: "script"`), `apps/studio/src/app/useSceneSync.ts` (`goTo` öffnet das Dokument), `apps/studio/src/api.ts` (Script-Befehle, `BuildReport.problems`), `apps/studio/src/workbench/terms.ts` (`scriptFile`, `script`, `levelScript`, `problem`), `apps/studio/src/workbench/inspector/fields.ts`, `eventFields.ts`, `apps/studio/src/world/events.ts` (Literale → Terme, „Script“)
- Test: `ScriptPanel.test.tsx`, `AdoptBox.test.tsx`, `problems.test.ts`, `ProblemsPanel.test.tsx`, `layout.test.ts`, `navigation.test.ts`, `useProjectSession`-Test für den Build-Fluss (`apps/studio/src/app/useProjectSession.test.tsx`, neu), `terms.test.ts`

**Interfaces:**
- Consumes: Task 8, 9, 10.
- Produces: `WorkbenchData.scripts: { status: ScriptsStatusDto | null; documents: ScriptDocuments; active: number | null; open(file, line?): Promise<void>; save(file): Promise<void>; close(file): void; adopt(): Promise<void>; lsp: "running" | "stopped" | "missing" }`, `WorkbenchData.problems: ProblemsStore`, `WorkbenchData.onOpenScript(file, line?)`; `TERMS.scriptFile.label === "Scriptdatei"`, `TERMS.script.label === "Script"`, `TERMS.levelScript.label === "Level-Script"`, `TERMS.problem.label === "Problem"`.
- Panel „Script“ (Spec §8.2): ohne `adopted` → `AdoptBox` (Text aus Spec §8.2, Knopf „Übernehmen“, Fortschritt aus dem Channel, Bericht); sonst Kopfzeile (Dokument-Auswahl mit „●“ für dirty, Titel `Scriptdatei 0042` / `Level-Script 0042 (JSON)`, Outline-Menü aus `documentSymbols`, „Speichern“, „Script anfügen“, LSP-Status „rotom-lsp fehlt — Completion und Diagnosen aus“) und `ScriptEditor`. Schließen eines dirty Dokuments: `ask("Änderungen an Scriptdatei 0042 verwerfen?", { kind: "warning" })`. `save` → `saveScript` → `problems.replaceForFile(file, …)`, `touched_archives` → `bus.emit({ topic: "textData", archive })` je Archiv. Panel „Probleme“: Tabelle Schwere · Datei · Zeile · Meldung · Quelle; Klick → `goTo({ kind: "script", file, line })` bzw. `textArchive`.

- [ ] **Step 1: Failing tests**: `problems.test.ts` (replace je Datei ersetzt nur diese Datei; counts); `AdoptBox.test.tsx` (ohne adopted sichtbar, Klick ruft `adoptScripts`, Fortschritt rendert, Bericht); `ScriptPanel.test.tsx` (öffnet Datei → Editor mit Text; Tippen setzt „●“; Speichern ruft `saveScript` und ersetzt Probleme; `editor_saves_without_lsp`: `lspStart` wirft `tool_missing` → Kopfzeile zeigt den Hinweis, Speichern funktioniert; Wechsel zwischen zwei Dokumenten behält Text); `ProblemsPanel.test.tsx` (Klick ruft `goTo` mit `{kind:"script", file, line}`); `useProjectSession.test.tsx` (`build_saves_open_scripts_first_and_stops_on_failure`: zwei dirty Dokumente → `saveScript` zweimal vor `buildRom`; wirft das erste, wird `buildRom` nicht gerufen; `buildRom` mit `code: "compile"` füllt den Probleme-Store); `layout.test.ts` (v5, v4-Layout verworfen); `navigation.test.ts`; `terms.test.ts`.
- [ ] **Step 2: Run, expect failure**: `pnpm test`
- [ ] **Step 3: Implement**; Literale „Skript“/„Skriptdatei“/„Level-Skript“ in `fields.ts`, `eventFields.ts`, `world/events.ts` durch `TERMS.*.label` ersetzen (Tests, die die alten Wörter prüfen, mitziehen).
- [ ] **Step 4: Run** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat(studio): script and problems panels, adoption box, build saves open scripts`.

### Task 12: Navigation vom Header, vom Event und im Welt-Explorer; Level-Script-Formular

**Files:**
- Create: `apps/studio/src/workbench/inspector/LevelScriptSection.tsx`, `apps/studio/src/workbench/inspector/levelScript.ts` (reine Funktionen: Zeilen ↔ `LevelScriptDoc`, Auslöser-Labels)
- Modify: `apps/studio/src/workbench/inspector/fields.ts` (`PICK_ASSET.script = "Script"`, `level_script_id` wird `pick("level_script_id", TERMS.levelScript.label, "script")`), `apps/studio/src/workbench/inspector/HeaderEditor.tsx` (Knöpfe „Öffnen“/„Neu“ an beiden Feldern, `LevelScriptSection`), `apps/studio/src/workbench/inspector/EventEditor.tsx` („Öffnen“, „Neues Script“ am Feld `script`), `apps/studio/src/workbench/WorldExplorer.tsx` + `places.ts` (Kinder „Scriptdatei NNNN“, „Level-Script NNNN“ unter dem Header; Wurzelknoten „Scripts“ mit Liste aus `listScriptFiles` und Suche über `searchScripts`), `apps/studio/src/api.ts`
- Test: `levelScript.test.ts`, `LevelScriptSection.test.tsx`, `HeaderEditor`-Test (bestehender oder neu `HeaderEditor.test.tsx`), `EventEditor.test.tsx`, `WorldExplorer.test.tsx`, `fields`-Test (`pickMax("script", {Script: 3}) === 2`)

**Interfaces:**
- Consumes: Task 8, 11 (`onOpenScript`, `goTo`).
- Produces:
  ```ts
  // levelScript.ts
  export const TRIGGER_LABELS: Record<"on_load" | "on_resume" | "on_transition" | "on_var_condition", string>;  // "Beim Laden (on_load)", "Beim Fortsetzen (on_resume)", "Beim Betreten (on_transition)", "Variablen-Bedingungen (on_var_condition)"
  export function addHeaderEntry(doc: LevelScriptDoc, type: HeaderEntryType, scriptId: number): LevelScriptDoc;
  export function addVarCondition(doc: LevelScriptDoc, v: { var: number; value: number; script_id: number }): LevelScriptDoc;
  export function removeEntry(doc: LevelScriptDoc, which: { list: "header_entries" | "var_conditions"; index: number }): LevelScriptDoc;
  // LevelScriptSection.tsx props
  { header: HeaderSummary; levelScriptId: number; onOpenScript: (file: number, line?: number) => void; onCreate: () => Promise<void>; onError: (e: unknown) => void }
  ```
  Verhalten: `levelScriptId === 0` → „Kein Level-Script — Anlegen“ (ruft `newScriptFile("levelScript")`, `setHeader(level_script_id)`); sonst lädt `readScript(id)`; ist `kind !== "levelScript"` → Hinweis „Datei NNNN ist kein Level-Script“ und „Als JSON öffnen“ aus. Jede Formularänderung ruft `saveLevelScript(file, doc)` sofort (Compile); Fehler in `onError`. „Als JSON öffnen“ → `onOpenScript(file)`. Event: „Öffnen“ → `resolveScript(header, n)` → `File` → `onOpenScript(file, line)`; `Ambiguous` → `onError("Script N liegt in mehreren Dateien: …")`; `Unknown` → `onError("Script N ist nicht auffindbar")`. „Neues Script“ → `addScript(scriptFileId des Headers)` → `updateEvent(.., { script: n })`.

- [ ] **Step 1: Failing tests**: `levelScript.test.ts` (add/remove erzeugen genau rotoms JSON-Form, `type: "on_load"`, `script_id`); `LevelScriptSection.test.tsx` (ID 0 zeigt „Anlegen“; mit Doc rendert Tabelle; Hinzufügen ruft `saveLevelScript` mit erwartetem Doc; „Öffnen“ ruft `onOpenScript(file, line)`); `EventEditor.test.tsx` („Öffnen“ löst auf und ruft `onOpenScript(0, 4)`; „Neues Script“ ruft `addScript` und `updateEvent` mit der Nummer); `WorldExplorer.test.tsx` (Header-Knoten hat Kinder „Scriptdatei 0000“, „Level-Script 0001“; Wurzel „Scripts“ listet Dateien, Suche zeigt Treffer mit Zeile und ruft `onOpenScript(file, line)`); `fields.test.ts` (`pickMax("script", …)`).
- [ ] **Step 2: Run, expect failure**: `pnpm test -- inspector WorldExplorer`
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat(studio): open scripts from headers, events and the explorer; level-script form`.

### Task 13: Gate-Dokument, Roadmap, Rauchtest

**Files:**
- Create: `docs/phase-3b2-gate.md` (Muster `docs/phase-3b1-gate.md`: Kopf, Teile (A) Übernehmen und Rebuild, (B) NPC mit Textliteral, (C) Flag und Item, (D) Vanilla-Dialog, (E) Level-Script, (F) Text-Panel, (G) DSPRE, (H) `tauri build` Rauchtest, (I) `scripts/gate.sh`; „Namen in der Oberfläche“; Vorbereitung; Checklisten; Ergebnis-Tabelle)
- Modify: `docs/ROADMAP.md` (Block 3b.2: Status „implementiert, Gate offen“, Spec- und Plan-Links, „Monaco“ → „CodeMirror 6“, Lieferumfang nach Spec §1), `docs/dev-setup.md` (Querverweise prüfen), `docs/superpowers/specs/2026-10-21-gen4-studio-design.md` (Zeile 23/35: „Monaco“ → „CodeMirror 6“ mit Datum)

- [ ] **Step 1: Docs schreiben.**
- [ ] **Step 2: Rauchtest**: `pnpm bundle` (ruft `prepare-bundle.sh`), `bash apps/studio/scripts/check-bundle.sh`, gepackte App auf einer Testkopie starten: Script öffnen, Completion erscheint, Speichern mit Fehler zeigt das Problem. Ergebnis im Gate-Dokument unter (H) als „vorab geprüft am <Datum>“ notieren, wenn bestanden; sonst Befund notieren und melden.
- [ ] **Step 3: `scripts/gate.sh`** komplett laufen lassen (im Hintergrund, ~25 min) → grün; Ausgabe-Tabelle ins Gate-Dokument.
- [ ] **Step 4: Commit** `docs: phase 3b.2 gate, roadmap and dev setup`.

---

## Self-Review (durchgeführt)

- **Spec-Abdeckung:** §4 Text → Tasks 1–4; §5 Scripts → 5, 6, 8; §6 Build → 7; §7 Sidecar → 9; §8 Frontend → 10–12; §9 Befehle → 3, 8, 9; §10 Fehler → 6, 8; §11 Tests → in jedem Task, gated in 6 und 7, Rauchtest in 13; §12 Risiken → uxie-Vereinheitlichung geprüft in Task 5 Step 3, Netz dokumentiert in Task 9; §13 Doku → 1, 6, 9, 13. Nicht im Plan, weil die Spec es ausschließt: Rückwärtsreferenzen, Mehrsprachigkeit, DP-Gate.
- **Typen:** `Problem`/`Severity` (Task 1) werden in 6, 7, 8 benutzt; `ProblemDto` (Task 3) in 8, 11; `LevelScriptDoc = rotom::LevelScript` (Task 6) in 8, 12; `ScriptDocuments` (Task 10) in 11; `NavTarget` `script`/`textArchive` (Tasks 4, 11) in 12; `LAYOUT_KEY` v4 (Task 4) → v5 (Task 11).
- **Review Focus:** 1 → Task 6 `adopted_project_is_used_without_touching_sources`; 2 → Task 2 `member_newer_than_json_wins`; 3 → Task 6 `compile_error_reports_line_and_column_after_umlauts`; 4 → Task 9 `missing_lsp_is_tool_missing`, Task 11 `editor_saves_without_lsp`; 5 → Task 7 `build_encodes_stale_text_json`, Task 11 `build_saves_open_scripts_first_and_stops_on_failure`.
- **Offene Entscheidungen, die der Implementierer treffen darf:** exakte Feldnamen in rotom-Typen (im Crate nachlesen, Pfade stehen in Spec §2.3), Debounce-Werte außer den genannten, CSS.
