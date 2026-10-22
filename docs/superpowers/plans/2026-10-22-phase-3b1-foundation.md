# Phase 3b.1 „Fundament“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Neue Header per DSPRE-kompatiblem ARM9-Patch aus den Assistenten heraus, Höhen und Objekt-Sprites in der 3D-Szene, HGSS-Kachelnamen, apicula mit Zeitlimit und versioniertem Cache, ein lokales Gate-Skript.

**Architecture:** Der Kernel bekommt zwei neue Module (`dynamic_headers`, `sprites`) und zwei Metadaten-Tabellen; die Header-Tabelle bleibt ein Asset, nur ihr `Location` wechselt nach dem Patch auf einen Ordner mit einer Datei je Header. `g4-formats` bekommt einen NSBTX-Decoder. Im Frontend werden Zellenhöhen in Layout, Picking und Marker eingezogen und Objekt-Marker zu Billboard-Sprites; der Schritt „Header“ der Assistenten bietet „Neuer Header“ und den Patch an.

**Tech Stack:** Rust (g4-formats, g4-kernel, Tauri 2), uxie 0.9, React 19, three.js 0.186, vitest.

**Spec:** [docs/superpowers/specs/2026-10-22-phase-3b1-foundation-design.md](../specs/2026-10-22-phase-3b1-foundation-design.md)

## Global Constraints

- Die Vanilla-Projekte unter `../Roms/*_DSPRE_contents` werden nie beschrieben; schreibende Tests laufen auf `vanilla_copy()`, das GUI nur auf Kopien unter `../Roms/_testkopien`.
- Testbefehle aus der Repo-Wurzel mit `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`. Vor jedem Commit: `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, in `apps/studio`: `pnpm test`, `pnpm build`.
- Gated Tests: `G4_TEST_PROJECT="$PWD/../Roms/<Projekt>_DSPRE_contents"`, `G4_APICULA="$PWD/tools/apicula/bin/apicula"`, `G4_DSROM_CLI="$PWD/../Roms/_tools/dsrom"`; die drei Projekte sind `Pokemon - Platin-Edition (Germany)` (CPUD), `Pokemon - Goldene Edition HeartGold (Germany)` (IPKD), `Pokemon - HeartGold Version (USA)` (IPKE).
- UI-Texte deutsch, Begriffe aus `apps/studio/src/workbench/terms.ts`; Code, Kommentare und Commit-Messages englisch.
- `CmdError.code` bleibt bei `stale | read_only | tool_missing | partial | error`.
- Frontend-Invarianten aus 3a: keine Re-Renders durch Hover oder Meldungen, Panels memoisiert, `useShallowStable` für `WorkbenchData`; wer das Dockview-Panel-Set ändert, zählt `LAYOUT_KEY` hoch (hier nicht nötig).
- Neue Dateien in `unpacked/<ordner>` nur über `write_atomic` (Temp unter `.g4/tmp`, dann `rename`): DSPRE packt jede Datei im Member-Ordner.
- Alles ist in `docs/formats/` zu dokumentieren, was ein Byte-Layout hat (Quelle der Information nennen).

## Review Focus

1. Ein Projekt, in dem DSPRE den Patch schon gesetzt hat: `header_patch_status` meldet `applied`, Header lesen und speichern laufen über die Dateien, kein Byte in `arm9.bin` wird angefasst (Task 2, Test `patched_project_reads_and_saves_headers_from_files`).
2. Eine Region, deren Referenzbytes nicht passen (etwa IPGS oder ein fremd gepatchtes ROM): `apply` schreibt nichts und nennt den Offset (Task 1, Test `apply_refuses_unexpected_bytes_and_writes_nothing`).
3. `mapname.bin` und Header-Dateien laufen auseinander (eine Datei fehlt): die Tabelle gilt als nicht auf Platte, `load` meldet es im Klartext statt zu paniken (Task 2, Test `missing_header_file_is_reported_not_panicked`).
4. Ein Objekt-Event mit `graphics_id`, das die Tabelle nicht kennt, oder ein Sprite in einem nicht unterstützten Format: Marker statt Absturz, kein wiederholter Backend-Aufruf je Frame (Task 9, Test `unknown_graphics_id_keeps_the_yellow_marker`).
5. apicula hängt: der Aufruf endet nach dem Zeitlimit, kein `.fail`-Marker, die Meldung nennt die Sekunden (Task 11, Test `slow_apicula_times_out_without_fail_marker`).

---

### Task 1: Patch-Daten und Patch-Modul im Kernel

**Files:**
- Create: `crates/g4-kernel/metadata/dynamic_headers.yaml`
- Create: `crates/g4-kernel/src/dynamic_headers.rs`
- Create: `docs/formats/dynamic_headers.md`
- Modify: `crates/g4-kernel/src/lib.rs` (Modul anmelden)
- Modify: `crates/g4-kernel/src/error.rs` (keine neue Variante nötig; Fehler sind `InvalidProject`/`UnknownGame`)
- Modify: `crates/g4-kernel/src/test_support.rs:92-105` (arm9 der Fixture bekommt die Referenzstellen)
- Test: `crates/g4-kernel/src/dynamic_headers.rs` (Unit-Tests), `crates/g4-kernel/tests/dynamic_headers.rs` (gated)

**Interfaces:**
- Produces:
  ```rust
  pub const PATCH_SIGNATURE: u16 = 0xB500;
  pub struct PatchPlan { pub init_offset: u64, pub init_bytes: Vec<u8>, pub references: Vec<(u64, u64)>, pub special_case: Option<[u64; 3]>, pub narc: PathBuf, pub unpacked: &'static str }
  /// `None`: the game has no patch (Diamond/Pearl).
  pub fn plan(project: &Project) -> Result<Option<PatchPlan>, KernelError>;
  pub fn is_applied(project: &Project) -> Result<bool, KernelError>;   // false for DP
  /// Checks every precondition, then writes the folder, then arm9.bin. Nothing is written on a failed check.
  pub fn apply(project: &Project, headers: &[MapHeader]) -> Result<(), KernelError>;
  pub fn unpacked_dir(project: &Project) -> PathBuf;                   // <root>/unpacked/dynamicHeaders
  ```

YAML-Layout (Werte aus DSPRE `ToolboxDB.cs` 272–390, siehe Spec §2.1):

```yaml
# Source: DSPRE ToolboxDB.cs (DynamicHeadersPatchData), verified on CPUD and IPKD. Offsets in decompressed arm9/arm9.bin.
init:
  CPUE: { offset: 0x3A024, bytes: "00 B5 01 1C 94 20 00 22 CC F7 48 FD 03 1C DE F7 C7 F8 00 BD" }
  CPUS: { offset: 0x3A0C8, bytes: "00 B5 01 1C 94 20 00 22 CC F7 00 FD 03 1C CC F7 74 FC 00 BD" }
  CPUI: { offset: 0x3A0C8, bytes: "… wie CPUS" }
  CPUF: { offset: 0x3A0C8, bytes: "… wie CPUS" }
  CPUD: { offset: 0x3A0C8, bytes: "… wie CPUS" }
  CPUJ: { offset: 0x39BE0, bytes: "00 B5 01 1C 94 20 00 22 CC F7 0A FF 03 1C DE F7 3D F9 00 BD" }
  IPKS: { offset: 0x3B260, bytes: "00 B5 01 1C 32 20 00 22 CC F7 5C F9 03 1C DF F7 4D FC 00 BD" }
  IPKJ: { offset: 0x3AE20, bytes: "00 B5 01 1C 32 20 00 22 CC F7 08 FB 03 1C DF F7 C7 FC 00 BD" }
  IPGJ: { offset: 0x3AE20, bytes: "… wie IPKJ" }
  IPKE: { offset: 0x3B268, bytes: "00 B5 01 1C 32 20 00 22 CC F7 58 F9 03 1C DF F7 49 FC 00 BD" }
  IPGE: …, IPGS: …, IPKF: …, IPGF: …, IPKI: …, IPGI: …, IPKD: …, IPGD: … (alle mit Offset und Bytes von IPKE)
# "… wie X" steht hier nur zur Kürze: in der Datei werden Offset und Bytefolge für jeden Spielcode ausgeschrieben.
families:
  pt:
    base: CPUE
    narc: files/debug/cb_edit/d_test.narc
    references: [[0x3A03E, 0x3A048], [0x3A052, 0x3A05C], [0x3A066, 0x3A080], [0x3A08E, 0x3A098], [0x3A0A2, 0x3A0AC], [0x3A0B6, 0x3A0C0], [0x3A0CA, 0x3A0D4], [0x3A0DE, 0x3A0E8], [0x3A0F2, 0x3A108], [0x3A116, 0x3A120], [0x3A12A, 0x3A134], [0x3A13E, 0x3A150], [0x3A15A, 0x3A170], [0x3A17A, 0x3A184], [0x3A18E, 0x3A198], [0x3A1A2, 0x3A1B4], [0x3A1BE, 0x3A1D0], [0x3A1DA, 0x3A1EC], [0x3A1F6, 0x3A208], [0x3A212, 0x3A224]]
  hgss:
    base: IPKE
    narc: files/a/0/5/0
    references: [[0x3B282, 0x3B28C], [0x3B296, 0x3B2A8], [0x3B2B2, 0x3B2BC], [0x3B2C6, 0x3B2D0], [0x3B2DA, 0x3B2E4], [0x3B2EE, 0x3B2F8], [0x3B302, 0x3B30C], [0x3B316, 0x3B320], [0x3B32A, 0x3B340], [0x3B34A, 0x3B354], [0x3B35E, 0x3B368], [0x3B372, 0x3B384], [0x3B38E, 0x3B3A4], [0x3B3AE, 0x3B3C4], [0x3B3CE, 0x3B3E0], [0x3B3EA, 0x3B3FC], [0x3B406, 0x3B418], [0x3B422, 0x3B434], [0x3B43E, 0x3B450], [0x3B45A, 0x3B46C], [0x3B476, 0x3B488], [0x3B492, 0x3B4A4], [0x3B4AE, 0x3B4C0], [0x3B4CA, 0x3B4D8], [0x3B4E2, 0x3B4F4], [0x3B4FE, 0x3B514]]
    # Only when the language is neither Japanese nor Spanish: an instruction sits between `mov r1,#0x18` and `mul r1,r0`.
    special_case: { mov: 0x3B522, mul: 0x3B526, literal: 0x3B53C }
unpacked: dynamicHeaders
```

Die Ersetzung je Referenz: 4 Bytes `19 00 C0 46` bei `item1 + diff`; `u32` LE bei `item2 + diff` wird `wert − header_table_offset − 0x0200_0000`. `diff = init.offset(game) − init.offset(base)`. Sonderfall: `19 00` bei `mov + diff`, `C0 46` bei `mul + diff`, `00 00 00 00` bei `literal + diff`. Vorprüfung nach Spec §2.3.

- [ ] **Step 1: Fixture erweitern.** In `synthetic_project()` die 20 Platin-Referenzpaare (CPUE, `diff = 0`) ins `arm9.bin` schreiben: bei jedem Item1 `18 21 41 43`, bei jedem Item2 `(0x0200_0000 + 0xE601C + k) as u32` LE mit `k` = Index des Paares (0..20). Zusätzlich das Vanilla-NARC `files/debug/cb_edit/d_test.narc` mit zwei Dummy-Membern `[1,2,3,4]` und `[5,6]` anlegen (`write_narc`). Der `arm9`-Puffer ist schon groß genug (Tabelle liegt bei 0xE601C).

- [ ] **Step 2: Failing tests schreiben** (Unit, in `dynamic_headers.rs`, mit `crate::test_support::synthetic_project`):

```rust
#[test] fn plan_is_none_for_diamond_pearl()                 // Project mit gamecode ADAE (Fixture-Variante: header.yaml umschreiben) → Ok(None)
#[test] fn is_applied_is_false_on_vanilla_fixture()
#[test] fn apply_writes_routine_references_and_header_files() {
    // nach apply: u16 LE @0x3A024 == 0xB500; die 20 Bytes == init.bytes(CPUE);
    // @item1: 19 00 C0 46; u32 @item2 == k; unpacked/dynamicHeaders/0000 und 0001 == je 24 Header-Bytes aus arm9 (vor dem Patch gelesen);
    // arm9.bin.backup existiert und ist byteweise das alte arm9.bin; is_applied == true; apply ein zweites Mal → Err (Signatur schon da).
}
#[test] fn apply_refuses_unexpected_bytes_and_writes_nothing() {
    // Fixture: bei Item1 des 7. Paares 00 00 00 00 setzen → apply Err, Meldung enthält "0x3A0CA"; arm9.bin unverändert; kein Ordner, kein Backup.
}
#[test] fn apply_refuses_literal_outside_the_table()         // Item2 des 3. Paares auf 0x02000000 → Err nennt "0x3A080"
#[test] fn backup_is_not_overwritten()                       // vorhandenes arm9.bin.backup mit Inhalt b"old" bleibt b"old"
```

- [ ] **Step 3: Run tests, expect compile failure** (`cargo test -p g4-kernel --all-features dynamic_headers`).

- [ ] **Step 4: Implement** `dynamic_headers.rs`: YAML per `include_str!` und `serde_yaml` (Hex-Strings mit einem kleinen Parser `fn hex(s: &str) -> Result<Vec<u8>, KernelError>`), `plan`, `is_applied`, `apply`. `apply` liest `arm9.bin` einmal in den Speicher, prüft alle Vorbedingungen gegen den Puffer, schreibt dann den Ordner (`<root>/unpacked/.dynamicHeaders.writing.<pid>` füllen, vorhandenen Ordner entfernen, `rename`), danach das Backup (nur wenn fehlend, mit `std::fs::copy`), zuletzt `arm9.bin` über die bestehende `write_atomic`-Funktion (nach `crate::fsutil` oder `pub(crate)` in `store.rs` verschieben, wenn nötig). Headers kommen als Parameter (`&[MapHeader]`, `write_map_header_to_bytes`), damit der Store seinen Stand übergibt.

- [ ] **Step 5: Gated test** `crates/g4-kernel/tests/dynamic_headers.rs` (`#![cfg(feature = "test-support")]`): `vanilla_copy()` → `plan` ist `Some` für Pt/HGSS, `is_applied` false, `apply(project, &project.header_listing()-Header…)`; danach `is_applied` true und Listing-Länge gleich. Für IPKD prüft der Test zusätzlich den Sonderfall: bytes `19 00` bei 0x3B522 und `C0 46` bei 0x3B526.

- [ ] **Step 6: Run** `cargo test -p g4-kernel --all-features dynamic_headers` → PASS; gated mit `G4_TEST_PROJECT` für CPUD und IPKD → PASS.

- [ ] **Step 7: Docs** `docs/formats/dynamic_headers.md`: Routine, Referenzen, Sonderfall, Signatur, NARC je Familie, Dateiformat (24 Bytes), Quelle DSPRE, Verifikation auf CPUD/IPKD.

- [ ] **Step 8: Commit** `feat(kernel): dynamic headers patch data, preconditions and apply`.

---

### Task 2: Header-Tabelle aus Dateien lesen und speichern, Build packt das NARC

**Files:**
- Modify: `crates/g4-kernel/metadata/archives.yaml` (pt, hgss: `dynamic_headers`)
- Modify: `crates/g4-kernel/src/project.rs:25-80` (`AssetKind::DynamicHeaders`, `ALL: [AssetKind; 11]`, `slot`, `label` "dynamic headers"), `:107-120` (`FamilyArchives.dynamic_headers: Option<ArchiveEntry>`), `:127-137` (`Location::HeaderFiles { dir: PathBuf, count: usize }`), `:141-200` (`ArchiveMap.entries: [...; 11]`), `:307-357` (`asset_location`, `header_table_extent`), `:430-447` (`ensure_unpacked`: `DynamicHeaders` wie `HeaderTable` nie entpacken, `member_count` → 0), `:587-600` (`read_headers` liest Dateien, wenn gepatcht)
- Modify: `crates/g4-kernel/src/store.rs:60-70` (`is_editable(DynamicHeaders)` false), `:203-226` (`read_disk`/`read_member` für `HeaderFiles`), `:766-800` (`save`: `HeaderFiles` schreibt geänderte Dateien), `:804-820` (`read_location`)
- Modify: `crates/g4-kernel/src/build.rs` (kein Code: `kinds()` liefert den neuen Kind; Kommentar ergänzen)
- Test: `crates/g4-kernel/tests/store.rs`, `crates/g4-kernel/tests/build.rs`, `crates/g4-kernel/tests/project.rs`

**Interfaces:**
- Consumes: `dynamic_headers::{is_applied, unpacked_dir}` (Task 1).
- Produces: `Location::HeaderFiles { dir, count }`; `Project::dynamic_headers_applied(&self) -> Result<bool, KernelError>` (memoisiert pro Aufruf nicht; ruft `dynamic_headers::is_applied`); `AssetKind::DynamicHeaders`.

- [ ] **Step 1: Failing tests** in `tests/store.rs` (Fixture aus Task 1, Patch per `dynamic_headers::apply` im Test gesetzt):

```rust
#[test] fn patched_project_reads_and_saves_headers_from_files() {
    // apply; Store::new; header_table().len() == 2; edit_header(1, "x", |h| set matrix_id 7); save → written == [HEADER_TABLE];
    // Datei 0001 Byte 2..4 == 7u16 LE, Datei 0000 unverändert (mtime egal, Bytes gleich); arm9.bin byteweise wie nach apply.
}
#[test] fn patched_header_save_detects_a_changed_file() {
    // Store lädt; Datei 0000 extern ändern (Byte 0 ^= 1); edit 1; save → Err(Stale(HEADER_TABLE)).
}
#[test] fn missing_header_file_is_reported_not_panicked() {
    // Datei 0001 löschen; Store::new; header_table() → Err; Meldung enthält "0001" und "dynamicHeaders".
}
```
In `tests/build.rs`: `build_packs_dynamic_headers_after_patch` — apply, build, `d_test.narc` neu gelesen hat 2 Member à 24 Bytes; und `build_leaves_vanilla_dynamic_headers_narc_untouched` — ohne Patch (Ordner fehlt in der Fixture) bleibt die Datei unverändert.
In `tests/project.rs`: `asset_kinds_have_eleven_slots` (`AssetKind::ALL.len() == 11`, `slot` bijektiv) und `pt_and_hgss_have_a_dynamic_headers_archive_dp_has_none`.

- [ ] **Step 2: Run, expect failure.**

- [ ] **Step 3: Implement.** `read_location(HeaderFiles)` liest `dir/0000..count`, konkateniert; fehlt eine Datei → `Ok(None)`; `read_member` meldet für `HeaderFiles` mit `None`: `InvalidProject("{dir}/{NNNN} is missing: header table has {count} names but not that file")`. `save`: neue Funktion `write_header_files(project_root, dir, base: Option<&[u8]>, new: &[u8])` — chunked 24, schreibt via `write_atomic` nur Chunks, die vom `base` abweichen (oder alle, wenn `base` `None`). `header_table_extent` prüft bei gepatchtem Projekt nicht die ARM9-Länge, sondern dass `dir` existiert. `read_headers` (für `header_listing`) liest bei Patch die Dateien.

- [ ] **Step 4: Run** `cargo test -p g4-kernel --all-features` → PASS. Gated `vanilla_rebuild_matches_dsrom_cli` für alle drei Projekte → PASS (das Vanilla-`dynamicHeaders`-Verzeichnis wird jetzt gepackt und muss byte-identisch sein; schlägt das fehl, ist das ein Befund für den NARC-Writer, kein Grund, den Kind wegzulassen).

- [ ] **Step 5: Commit** `feat(kernel): header table lives in one file per header once the patch is applied`.

---

### Task 3: `Store::apply_dynamic_headers` und `Store::create_header`

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (nach `duplicate_event_file`, ~Z. 600)
- Modify: `crates/g4-kernel/src/project.rs` (`pub fn mapname_path(&self) -> PathBuf`, aus den zwei Literalen `files/fielddata/maptable/mapname.bin`)
- Test: `crates/g4-kernel/tests/store.rs`, gated `crates/g4-kernel/tests/dynamic_headers.rs`

**Interfaces:**
- Produces:
  ```rust
  /// Refuses with InvalidProject when the header table is dirty or the game has no patch.
  pub fn apply_dynamic_headers(&mut self) -> Result<(), KernelError>;
  pub fn dynamic_headers_applied(&self) -> Result<bool, KernelError>;
  /// New header = copy of `template` as the editor shows it; `internal_name` ASCII ≤ 16 bytes. Returns the new id. Not undoable.
  pub fn create_header(&mut self, template: u16, internal_name: &str) -> Result<u16, KernelError>;
  pub const MAX_HEADERS: usize = 10_000;
  ```

- [ ] **Step 1: Failing tests** (`tests/store.rs`):

```rust
#[test] fn apply_dynamic_headers_refuses_a_dirty_table()   // edit_header ohne save → Err enthält "save"
#[test] fn apply_dynamic_headers_reloads_the_table()        // danach dynamic_headers_applied() true, header_table() == vorher, dirty() leer, undo() == None
#[test] fn create_header_appends_file_and_name() {
    // apply; create_header(1, "ROUTE_230") == 2; Datei 0002 == Bytes von Header 1; mapname.bin 48 Bytes, letzte 16 == b"ROUTE_230\0\0\0\0\0\0\0";
    // header_table().len() == 3; header_listing_for(...)[2].internal_name == "ROUTE_230"; dirty() leer.
}
#[test] fn create_header_copies_unsaved_edits_of_the_template()   // edit_header(0, matrix 9) ohne save; create_header(0, "X") → Datei 000N hat matrix_id 9; Tabelle bleibt dirty nur für Header 0 (save schreibt 0000)
#[test] fn create_header_without_patch_is_refused()        // Err enthält "patch"
#[test] fn create_header_rejects_non_ascii_or_long_names() // "Straße" → Err; 17 Zeichen → Err
```

- [ ] **Step 2: Run, expect failure.**

- [ ] **Step 3: Implement.** `apply_dynamic_headers`: `dirty()` enthält `HEADER_TABLE` → Err; `header_table()` laden; `dynamic_headers::apply(project, headers)`; Asset `HEADER_TABLE` aus `assets` entfernen und alle Changesets der Tabelle aus `undo`/`redo` streichen (wie `reload_from_disk`); erneutes `load`. `create_header`: Patch prüfen; `count = header_count()`; `count < MAX_HEADERS`; Bytes der Vorlage = das 24-Byte-Chunk aus `member_bytes(HEADER_TABLE)` (Tabelle so, wie der Editor sie zeigt); Datei `dir/NNNN` via `write_atomic`, Fehler bei existierender Datei; dann `mapname.bin` neu schreiben (`write_atomic` der Gesamtdatei plus Record); bei Fehler die Header-Datei wieder löschen. Danach das geladene Asset fortschreiben: `current.push(header)`, `saved_bytes.extend(chunk)`. Hinweis: `Asset.saved_bytes` ist die Verkettung; `apply_edit`-Dirty-Vergleich funktioniert weiter.

- [ ] **Step 4: Gated** in `tests/dynamic_headers.rs`: auf Kopie von CPUD und IPKD: `apply_dynamic_headers`, `create_header(0, "G4TEST")`, dann `build(project, tmp.nds)` → Ok, `header_listing().headers.len()` == alt + 1.

- [ ] **Step 5: Run → PASS. Commit** `feat(kernel): apply the dynamic headers patch and create headers from the store`.

---

### Task 4: Tauri-Befehle für Patch-Status und neue Header in den Assistenten

**Files:**
- Create: `apps/studio/src-tauri/src/commands/patches.rs`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs` (Modul, Re-Exports), `apps/studio/src-tauri/src/lib.rs:5-70` (zwei Befehle registrieren)
- Modify: `apps/studio/src-tauri/src/commands/wizards.rs:39-44` (`AssignHeader.new_header`), `:112-170` (`Created.headers`, `created_text`), `:296-320` (`check_header_fits`), `:356-410` (`create_area_impl`), `:494-560` (`create_room_impl`), Tests ab `:700`
- Test: `apps/studio/src-tauri/src/commands/patches.rs`, `commands/wizards.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Serialize)] pub struct HeaderPatchDto { pub supported: bool, pub applied: bool, pub narc: Option<String> }
  pub fn header_patch_status_impl(state: &AppState) -> Result<HeaderPatchDto, CmdError>;
  pub fn apply_header_patch_impl(state: &AppState) -> Result<HeaderPatchDto, CmdError>;
  #[derive(Deserialize, Clone)] pub struct NewHeader { pub template: u16, pub internal_name: String }
  pub struct AssignHeader { pub id: u16, pub area_data_id: Option<u8>, pub event_file: EventFileChoice, #[serde(default)] pub new_header: Option<NewHeader> }
  ```
  `Created` bekommt `headers: Vec<u16>`; `created_text` nennt sie zuerst („header 0470, …“).

- [ ] **Step 1: Failing tests.** `patches.rs`: `status_reports_unsupported_for_dp`, `status_then_apply_then_status` (Fixture; nach apply `applied == true`, `narc == Some("files/debug/cb_edit/d_test.narc")`). `wizards.rs`: `check_assign_refuses_new_header_with_wrong_id` (count 2, `id: 5`, `new_header: Some` → Err enthält "0002"), `check_assign_refuses_new_header_without_patch` (Err enthält "Patch" nicht nötig, englische Meldung "patch"), `create_area_with_new_header_creates_it_first` (nach apply; Report `header == Some(2)`; Listing 3 Header; Header 2 zeigt auf die neue Matrix; `mapname.bin` endet mit dem Namen), `late_failure_names_the_new_header` (Patch gesetzt, `cells` mit ungültiger `source_map` nach einer gültigen? — einfacher: `event_file: Copy(999)` nach dem Header → Err code `partial`, Meldung beginnt mit "header 0002").

- [ ] **Step 2: Run, expect failure.**

- [ ] **Step 3: Implement.** `check_assign`: bei `new_header` → `store.dynamic_headers_applied()?` sonst Err "new headers need the dynamic headers patch"; `assign.id == store.header_count()` sonst Err "a new header gets id {count}, not {id}"; `template` muss existieren; `check_header_fits` benutzt die Vorlage. In `create_area_impl`/`create_room_impl` als erster Schritt: `store.create_header(template, &name)` → `created.headers.push(id)`; alles weitere unverändert (`assign.id` ist jetzt vorhanden).

- [ ] **Step 4: Run → PASS. Commit** `feat(studio): header patch commands and new headers from the wizards`.

---

### Task 5: Frontend: „Neuer Header“ im Schritt Header

**Files:**
- Modify: `apps/studio/src/api.ts:461-465` (`AssignHeader.new_header`, `NewHeader`), neue Befehle `headerPatchStatus`, `applyHeaderPatch`, Typ `HeaderPatchDto`
- Modify: `apps/studio/src/workbench/wizards/wizardSpecs.ts:6-11` (`HeaderChoiceValue.newHeader: { template: number; internalName: string } | null`), `areaSpec`/`roomSpec` (bei `newHeader`: `id = headers.length`, `new_header` setzen), `assignLine` („legt Header 0470 „ROUTE_230“ neu an (Kopie von 0342) und stellt ihn auf …“), neue Funktion `internalName(name: string): string`
- Modify: `apps/studio/src/workbench/wizards/HeaderChoice.tsx` (Props `patch: HeaderPatchDto | null`, `onApplyPatch: () => Promise<void>`, `template: number`, `defaultName: string`)
- Modify: `apps/studio/src/workbench/wizards/NewAreaWizard.tsx:150-240`, `NewRoomWizard.tsx:150-290` (Status laden, Vorlage, Name, `headers.length`)
- Modify: `apps/studio/src/workbench/terms.ts` (Eintrag `patch`: label "ROM-Patch", short "Eine dauerhafte Änderung am Spielcode. Der Patch „Dynamische Header“ verlegt die Header in eine Datei je Header, damit neue angelegt werden können; DSPRE kennt ihn.")
- Test: `wizardSpecs.test.ts`, `HeaderChoice.test.tsx` (neu), `NewAreaWizard.test.tsx`

**Interfaces:**
- Consumes: Task 4 DTOs. Der Schritt lädt `headerPatchStatus()` beim Öffnen (`useEffect`, Ergebnis im Wizard-State); `applyHeaderPatch()` setzt den Status neu und emittiert `headerTable` über den Bus (die Header-Liste wird neu gelesen).

Verhalten: Oberhalb der Header-Liste ein Radio „Vorhandenen Header umstellen | Neuer Header“. „Neuer Header“ ist bei `patch.supported === false` nicht wählbar mit dem Satz aus Spec §2.2; bei `applied === false` zeigt die Wahl einen Kasten (Erklärung in vier Sätzen: ändert Bytes im Spielcode, Header liegen danach in einer Datei je Header, lässt sich nicht zurücknehmen, `arm9.bin.backup` wird angelegt, DSPRE kennt den Patch) mit Knopf „Patch anwenden“; danach erscheinen Vorlage (Text „Kopie von Header 0342 „R201““) und das Feld „Interner Name“ (vorbelegt `internalName(form.name)`, max 16, `pattern` A–Z, 0–9, `_`). Die Zusammenfassung übernimmt die neue `assignLine`.

- [ ] **Step 1: Failing tests.** `wizardSpecs.test.ts`: `internalName("Route 230") === "ROUTE_230"`, `internalName("Straße") === "STRAE"`, `internalName("") === "NEWMAP"`, `internalName("x".repeat(20)).length === 16`; `areaSpec` mit `newHeader` → `header.id === headers.length` wird vom Aufrufer gesetzt (Test: `areaSpec(form, headerCount: 470)` → `id 470`, `new_header: { template: 342, internal_name: "ROUTE_230" }`); `areaSummary` enthält „legt Header 0470 „ROUTE_230“ neu an“. `HeaderChoice.test.tsx`: unsupported → Radio disabled und Satz sichtbar; not applied → Knopf „Patch anwenden“ ruft `onApplyPatch`; applied → Namensfeld vorbelegt. `NewAreaWizard.test.tsx`: mit gemocktem `headerPatchStatus` (applied) und `createArea` wird `spec.header.new_header` gesendet und `id === headers.length`.

- [ ] **Step 2: Run** `cd apps/studio && pnpm vitest run wizards` → FAIL.

- [ ] **Step 3: Implement.** `areaSpec(form, headerCount: number)` und `roomSpec(form, headerCount)` erhalten die Anzahl als Parameter (Aufrufer: `headers.length`).

- [ ] **Step 4: Run → PASS; `pnpm build`. Commit** `feat(studio): wizards offer a new header and apply the dynamic headers patch on demand`.

---

### Task 6: Höhen in der Szene

**Files:**
- Modify: `apps/studio/src/world/layout.ts` (`ALTITUDE_STEP = 0.5`, `cellY(matrix: Pick<MatrixDto,"width"|"altitudes">, col: number, row: number): number`, `cellYAtTile(matrix, x, z)`)
- Modify: `apps/studio/src/world/scene/content.ts:23-33` (Modell-Y), `:60-80` (Overlay-Y je Zelle)
- Modify: `apps/studio/src/world/buildingLayer.ts:53` (`cell` bekommt `y`), `apps/studio/src/world/buildings.ts:13-18` (`buildingTransform(b, cell: {x; y?; z})` addiert `cell.y ?? 0`)
- Modify: `apps/studio/src/world/markers.ts` (Marker-, Beacon-, Flächen- und Outline-Y plus `cellYAtTile`; `moveMarker` berechnet Y neu)
- Modify: `apps/studio/src/world/scene/picking.ts:33-55` (Höhenstufen durchprobieren), `eventDrag.ts:46`, `buildingDrag.ts:62` (Ebene auf Zellenhöhe)
- Modify: `apps/studio/src/app/useSceneSync.ts:40-42` (`sameLayout` vergleicht `altitudes`)
- Test: `apps/studio/src/world/layout.test.ts` (neu), `markers.test.ts`, `buildings.test.ts`, `scene/picking.test.ts` (neu, mit jsdom und einem `SceneHost`-Stub wie in `eventDrag.test.ts`), `app/useSceneSync.test.ts` (`sameLayout`)

**Interfaces:**
- Produces: `cellY`, `cellYAtTile`, `ALTITUDE_STEP`; `buildingTransform` akzeptiert `cell.y`.

- [ ] **Step 1: Failing tests.** `layout.test.ts`: `cellY` ohne Abschnitt 0; Höhe 2 → 1; `cellYAtTile(m, 40, 3)` nimmt Zelle (1, 0). `markers.test.ts`: Objekt auf Kachel (40,3) einer Matrix mit Höhe 2 in Zelle (1,0) hat Marker-Y `MARKER_Y + 1`; `moveMarker` auf eine Kachel in Zelle mit Höhe 0 setzt Y zurück. `buildings.test.ts`: `buildingTransform({y: 0.5…}, {x: 0, y: 1, z: 0}).position[1] === 1.5`. `picking.test.ts`: zwei Zellen (Höhe 0 und 2), Strahl von oben auf Zelle 1 trifft `row 0, col 1`; ein Strahl, der die Ebene y = 1 außerhalb von Zelle 1 schneidet, fällt auf die Ebene y = 0 zurück. `useSceneSync.test.ts`: `sameLayout` false, wenn nur `altitudes` abweicht.

- [ ] **Step 2: Run → FAIL. Step 3: Implement** (Picking: `levels = [...new Set(altitudes ?? [0])].sort((a,b)=>b-a)`; je Level Ebene `-(OVERLAY_Y + level*0.5)`, Treffer gilt, wenn `altitudes[cell] === level`). **Step 4: Run → PASS; `pnpm build`.**

- [ ] **Step 5: Commit** `feat(studio): matrix altitudes raise cells in the scene (half a tile per step)`.

---

### Task 7: NSBTX-Decoder in g4-formats

**Files:**
- Create: `crates/g4-formats/src/nsbtx.rs`
- Modify: `crates/g4-formats/src/lib.rs` (`pub mod nsbtx;`), `crates/g4-formats/src/error.rs` (`FormatError::Unsupported { what: &'static str }`)
- Create: `docs/formats/nsbtx.md`
- Test: `crates/g4-formats/src/nsbtx.rs` (Unit), `crates/g4-formats/tests/vanilla.rs` (gated)

**Interfaces:**
- Produces:
  ```rust
  pub struct TextureInfo { pub name: String, pub width: u32, pub height: u32, pub format: u8, pub color0_transparent: bool }
  pub struct PaletteInfo { pub name: String, pub colors: Vec<u16> }   // BGR555
  pub struct Rgba { pub width: u32, pub height: u32, pub pixels: Vec<u8> } // RGBA8, row-major
  pub struct Nsbtx { pub textures: Vec<TextureInfo>, pub palettes: Vec<PaletteInfo>, /* private: data slices */ }
  impl Nsbtx {
      pub fn read(bytes: &[u8]) -> Result<Nsbtx, FormatError>;        // "BTX0" + TEX0
      /// `force_index0_transparent`: palette index 0 gets alpha 0 even without the texture's color0 flag (overworld sprites).
      pub fn decode_with(&self, texture: usize, palette: usize, force_index0_transparent: bool) -> Result<Rgba, FormatError>; // Unsupported for format 5
      pub fn decode(&self, texture: usize, palette: usize) -> Result<Rgba, FormatError>;   // decode_with(.., false)
  }
  ```

Layout-Referenz: apicula `src/nitro/tex.rs` und `src/nitro/info_block.rs` (`ls ~/.cargo/git/checkouts/apicula-*/*/src/nitro/`), DSPRE `DS_Map/LibNDSFormats/NSBTX/nsbtx.cs:350-664`. Fakten aus der Spec §4.1 (Parameterbits, bpp, Nibble-Reihenfolge, Paletten BGR555 → `r = (c & 31) << 3 | (c & 31) >> 2` usw.). Alpha: A3I5 `alpha = (v >> 5) * 255 / 7`, A5I3 `alpha = (v >> 3) * 255 / 31`, direkt Bit 15, Paletten-Index 0 → 0 wenn `color0_transparent`.

- [ ] **Step 1: Failing tests.** Im Test eine Hilfsfunktion `fn btx0(textures: &[(w, h, format, &[u8])], palettes: &[&[u16]]) -> Vec<u8>` schreiben, die ein minimales BTX0 nach dem apicula-Layout baut (Namen `tex0`, `pal0`). Tests: `reads_texture_and_palette_lists` (2 Texturen 16×32 Format 3, 1 Palette 16 Farben), `decodes_16_colours_low_nibble_first` (Pixel 0 = Index 1 → Farbe 1, Pixel 1 = Index 2), `index_zero_is_transparent_only_with_color0` (zwei Texturen, Flag an/aus), `decodes_a3i5_alpha`, `decodes_256_colours`, `rejects_bad_magic`, `texel_4x4_is_unsupported`.

- [ ] **Step 2: Run → FAIL. Step 3: Implement. Step 4: Run → PASS.**

- [ ] **Step 5: Gated** in `tests/vanilla.rs`: `vanilla_overworld_sprite_0_decodes` — NARC `files/data/mmodel/mmodel.narc` (Pt) bzw. `files/a/0/8/1` (HGSS) lesen, Member 0 als `Nsbtx`, `textures.len() >= 1`, `decode(0, 0)` → `width*height > 0`, mindestens ein Pixel mit Alpha 255 und eines mit Alpha 0 (Index 0 transparent erzwingen: der Test setzt Alpha von Index-0-Pixeln selbst, wenn das Flag fehlt — also prüft er nur, dass es Pixel mit Index 0 und solche mit Alpha 255 gibt).

- [ ] **Step 6: Docs** `docs/formats/nsbtx.md` (Header, TEX0-Blöcke, Info-Dicts, Parameterbits, Formate, Quelle apicula/DSPRE, Verifikation).

- [ ] **Step 7: Commit** `feat(formats): NSBTX reader and texture decoder`.

---

### Task 8: Overworld-Tabelle und Sprite-Cache im Kernel

**Files:**
- Create: `crates/g4-kernel/metadata/overworld_tables.yaml`
- Create: `crates/g4-kernel/src/sprites.rs`
- Create: `docs/formats/overworld_table.md`
- Modify: `crates/g4-kernel/src/lib.rs`, `crates/g4-kernel/src/project.rs` (`pub fn overlay(&self, id: u16) -> Result<Overlay, KernelError>` liest `arm9_overlays/overlays.yaml` → `{ base_address: u32, code_size: u32, path: PathBuf }`)
- Test: `crates/g4-kernel/src/sprites.rs` (Unit mit synthetischen Overlays), `crates/g4-kernel/tests/sprites.rs` (gated)

YAML:
```yaml
# Source: DSPRE RomInfo.SetOWtable (overlay 5 offsets; HGSS: RAM address of the pointer in overlay 1). Verified: CPUD 440 entries, IPKD 901 entries.
overlay5_offset:
  ADAE: 0x22BCC, APAE: 0x22BCC, ADAJ: 0x23BB8, APAJ: 0x23BB8
  ADAD: 0x22B84, APAD: 0x22B84, ADAS: 0x22B84, APAS: 0x22B84, ADAF: 0x22B84, APAF: 0x22B84, ADAI: 0x22B84, APAI: 0x22B84
  CPUE: 0x2BC34, CPUI: 0x2BC44, CPUF: 0x2BC3C, CPUS: 0x2BC3C, CPUD: 0x2BC50, CPUJ: 0x2BA24
overlay1_pointer:
  IPKE: 0x021F92FC, IPGE: 0x021F92FC, IPKI: 0x021F929C, IPGI: 0x021F929C, IPKF: 0x021F931C, IPGF: 0x021F931C
  IPKS: 0x021F931C, IPGS: 0x021F933C, IPKD: 0x021F92DC, IPGD: 0x021F92DC, IPKJ: 0x021F86C4, IPGJ: 0x021F86C4
sprites:
  dp: files/data/mmodel/mmodel.narc
  pt: files/data/mmodel/mmodel.narc
  hgss: files/a/0/8/1
```

**Interfaces:**
- Produces:
  ```rust
  pub struct OverworldTable { entries: HashMap<u16, u16> }       // graphics_id -> sprite member
  impl OverworldTable { pub fn load(project: &Project) -> Result<OverworldTable, KernelError>; pub fn sprite_for(&self, graphics_id: u16) -> Option<u16>; pub fn len(&self) -> usize; }
  #[derive(Clone, Copy)] pub enum Facing { Up, Down, Left, Right }
  impl Facing { pub fn from_dir(dir: i16) -> Facing }              // 0 Up, 1 Down, 2 Left, 3 Right, else Down
  pub fn frame_index(texture_count: usize, facing: Facing) -> usize;   // Tabelle Spec §4.1
  pub fn is_3d_overworld(family: Family, graphics_id: u16) -> bool;    // DP/Pt: 91..=96, 101..=116
  pub struct SpriteCache { table: Option<Result<OverworldTable, String>>, sprites: HashMap<(u16, u8), Option<Arc<Rgba>>> }
  impl SpriteCache { pub fn new() -> Self; pub fn object_sprite(&mut self, project: &Project, graphics_id: u16, facing: Facing) -> Result<Option<Arc<Rgba>>, KernelError>; }
  ```
  `object_sprite`: Tabelle einmal laden (Fehler merken und jedes Mal zurückgeben); kein Eintrag oder 3D → `Ok(None)`; Member aus dem NARC (NARC-Bytes einmal je Cache lesen), `Nsbtx::read`, `decode_with(frame_index(n, facing), 0, true)` (Index 0 immer transparent); `Unsupported` → `Ok(None)`.

- [ ] **Step 1: Failing tests.** Unit: synthetisches Projekt mit `arm9_overlays/overlays.yaml` (ids 1 und 5) und Overlay-Dateien: Pt-Variante (CPUE) mit 3 Einträgen + Terminator bei 0x2BC34 → `len 3`, `sprite_for(0) == Some(90)`; HGSS-Variante (IPKE, header.yaml umgeschrieben) mit Zeiger bei `0x021F92FC − base` auf 2 Einträge → `len 2`; Zeiger außerhalb (`0x023C8000`) → Err enthält "overlay 1"; `frame_index(16, Down) == 11`, `(4, Right) == 3`, `(1, Left) == 0`, `(20, Down) == 27`; `is_3d_overworld(Pt, 94)`, `!is_3d_overworld(Hgss, 94)`; `SpriteCache` liefert für eine im Test gebaute BTX0 im Sprite-NARC `Some` mit erwarteter Größe und für unbekannte ID `None`, zweiter Aufruf liest die Datei nicht erneut (Datei löschen, Ergebnis gleich).

- [ ] **Step 2: Run → FAIL. Step 3: Implement. Step 4: Run → PASS.**

- [ ] **Step 5: Gated** `tests/sprites.rs`: CPUD `len() == 440`, `sprite_for(0) == Some(90)`; IPKD `len() == 901`, `sprite_for(1) == Some(0)`; IPKE lädt ohne Fehler; `object_sprite(project, 1, Down)` → `Some` auf allen drei.

- [ ] **Step 6: Docs** `docs/formats/overworld_table.md`. **Step 7: Commit** `feat(kernel): overworld sprite table and object sprite cache`.

---

### Task 9: Sprites in Tauri und in der Szene

**Files:**
- Modify: `apps/studio/src-tauri/src/state.rs:8-11` (`Session.sprites: g4_kernel::sprites::SpriteCache`), Konstruktion in `commands/project.rs:27-45`
- Modify: `apps/studio/src-tauri/src/commands/world.rs:113-122,197-208` (`ObjectDto.dir: i16`), `commands/events.rs:50-60,140-160` (`EventPatch.dir: Option<i16>`)
- Create: `apps/studio/src-tauri/src/commands/sprites.rs` (`SpriteDto { width: u32, height: u32, rgba: Vec<u8> }`, `get_object_sprite_impl(state, graphics_id: u16, facing: u8) -> Result<Option<SpriteDto>, CmdError>`, `#[tauri::command] get_object_sprite`), Registrierung in `lib.rs`, `commands/mod.rs`
- Modify: `apps/studio/src/api.ts` (`ObjectDto.dir`, `EventPatch.dir`, `SpriteDto`, `getObjectSprite(graphicsId, facing)` über `shareInflight`)
- Create: `apps/studio/src/world/sprites.ts` (`spriteTexture(graphicsId, dir): Promise<{ texture: THREE.Texture; width; height } | null>`, Cache `Map<string, Promise<…>>`, `resetSprites()`, `facingOf(dir)`)
- Modify: `apps/studio/src/world/markers.ts` (Objekt-Marker als `THREE.Sprite`; `entries: Map<THREE.Object3D, …>`; `pickMarker` nutzt `hit.instanceId ?? 0`; `applySprite(set, index, tex)`; Outline-Höhe = Sprite-Höhe), `apps/studio/src/world/scene/content.ts:97-110` (`setMarkers` startet das Nachladen der Texturen und ruft `h.queueRender()`), `apps/studio/src/workbench/useThumbnailReset.ts` (`resetSprites()` beim Projektwechsel), `apps/studio/src/workbench/inspector/eventFields.ts:52-57` (Feld `dir` „Richtung (0 oben, 1 unten, 2 links, 3 rechts)“, I16)
- Test: `commands/sprites.rs` (Unit mit Fixture aus Task 8), `world/markers.test.ts`, `world/sprites.test.ts` (neu), `world/scene/eventDrag.test.ts` (Sprite wird gezogen)

**Interfaces:**
- Consumes: Task 8 `SpriteCache`, Task 6 `cellYAtTile`.
- Produces: `getObjectSprite`, `ObjectDto.dir`.

Szene: je Objekt ein `THREE.Sprite` mit `SpriteMaterial({ map: placeholder, depthTest: false, transparent: true })`, `center.set(0.5, 0)`, Position `(x+0.5, cellYAtTile, z+0.5)`, Skala `(0.8, 0.4)` für den Platzhalter (gelbes 1×1-`DataTexture`), nach Laden `(w/16, h/16)` und `map = texture` (`NearestFilter`, `colorSpace = SRGBColorSpace`, `flipY` so, dass die Oberkante oben ist). `renderOrder 11`. Texturen werden außerhalb des Marker-Baus geladen; ein Ergebnis für einen inzwischen ersetzten `MarkerSet` wird verworfen (Set-Identität prüfen).

- [ ] **Step 1: Failing tests.** `sprites.rs`: `get_object_sprite_returns_null_for_unknown_id`, `get_object_sprite_decodes_known_id` (Fixture mit BTX0 aus Task 8). `markers.test.ts`: Objekt-Marker ist ein `THREE.Sprite` an `(x+0.5, cellY, z+0.5)`; `pickMarker` findet ihn; `moveMarker` bewegt ihn; `applySprite` setzt Skala `(2, 2)` für 32×32. `sprites.test.ts`: `unknown_graphics_id_keeps_the_yellow_marker` — `getObjectSprite` gemockt mit `null`: `spriteTexture` liefert `null`, ein zweiter Aufruf ruft das Backend nicht erneut (Cache); Fehler → `null` ohne Cache (nächster Aufruf versucht erneut). `eventDrag.test.ts`: bestehender Objekt-Drag-Test läuft mit Sprite-Markern.

- [ ] **Step 2: Run → FAIL. Step 3: Implement. Step 4: Run → PASS; `cargo test -p studio`, `pnpm build`.**

- [ ] **Step 5: Commit** `feat(studio): object events show their overworld sprite as a billboard`.

---

### Task 10: Kachelverhalten-Namen je Familie

**Files:**
- Create: `crates/g4-kernel/metadata/tile_behaviors_hgss.yaml`
- Modify: `crates/g4-kernel/src/tiles.rs` (`tile_behaviors(family: Family)`, `tile_behavior(family, value)`), Aufrufer per `rg "tile_behavior"` (`commands/world.rs:240,297,336`, ggf. `export.rs`/`hover`)
- Modify: `apps/studio/src-tauri/src/commands/world.rs:240,297` (`tile_behaviors(state)` liefert den Satz der Session-Familie)
- Modify: `apps/studio/src/api.ts:280`, `apps/studio/src/world/scene/tileNames.ts` (`resetTileNames()`), `apps/studio/src/workbench/useThumbnailReset.ts` (beim Projektwechsel `resetTileNames()`)
- Test: `tiles.rs`, `commands/world.rs`, `scene/tileNames.test.ts` (neu)

- [ ] **Step 1: Quelle holen** (nur lesen, Scratch): `curl -sSL https://raw.githubusercontent.com/pret/pokeheartgold/6988158691eb6178cff697cda0428045ddcba5fb/include/constants/metatile_behavior.h -o <scratch>/metatile_behavior.h`. Enum `TILE_BEHAVIOR` ab Zeile 8: der Wert ist die Position (0-basiert). Einträge `TILE_BEHAVIOR_<n>` (nur Ziffern) und `*_UNUSED` entfallen. Klassen nach Namensteilen: `GRASS`→grass; `WATER|WHIRLPOOL|WATERFALL|PUDDLE|SEA|POND|SURF|DIVE`→water; `ICE`→ice; `SAND|MUD|SWAMP`→sand; `LEDGE|JUMP`→ledge; `BLOCK|WALL|BARRIER|HOLE|IMPASS`→block; `SLIDE|CURRENT|SPIN|RAPIDS`→slide; `WARP|STAIR|LADDER|ESCALATOR|TELEPORT|ELEVATOR|EXIT`→warp; `DOOR`→door; `BRIDGE`→bridge; `TV|PC|BOOKSHELF|COUNTER|TABLE|DESK|CHAIR|BED|SHELF|MACHINE|SIGN|TOWN_MAP|TRASH|SINK|BOARD|POSTER|FRIDGE|OVEN|STOVE|CABINET|VENDING|SLOT`→furniture; sonst other. Kopfkommentar wie in `tile_behaviors.yaml` mit Quelle und Commit.

- [ ] **Step 2: Failing tests.** `tiles.rs`: `hgss_table_names_headbutt` (`tile_behavior(Family::Hgss, 0x06) == Some(HEADBUTT, other)`), `platinum_has_no_headbutt` (`tile_behavior(Family::Pt, 0x06)` ist nicht `HEADBUTT`), `dp_uses_the_platinum_set`, beide Tabellen sortiert, eindeutig, ohne `UNUSED`/rein numerische Namen, Klassen aus der erlaubten Menge. `world.rs`: `tile_behaviors_command_follows_the_session_family` (Fixture CPUE → Platin-Satz). `tileNames.test.ts`: `loadNames` cached; `resetTileNames` erzwingt erneuten Fetch.

- [ ] **Step 3: Run → FAIL. Step 4: Implement. Step 5: Run → PASS. Commit** `feat: tile behaviour names per game family (HGSS set from pokeheartgold)`.

---

### Task 11: apicula mit Zeitlimit, außerhalb des Async-Pools, Cache-Schlüssel mit Werkzeug-Hash

**Files:**
- Modify: `crates/g4-kernel/src/export.rs:14` (`CACHE_SALT = b"g4glb-v3\0"`), `:63-112` (`nsbmd_to_glb` mit Timeout), `:117-125` (`glb_cache_key(tool: u64, nsbmd, nsbtx)`), alle Aufrufer (`glb_cache_get`, `glb_cached`, `glb_fail_*` bekommen `tool: u64`), neu `pub fn tool_fingerprint(apicula: &Path) -> Result<u64, KernelError>` (xxh3 der Datei, memoisiert in `OnceLock<Mutex<HashMap<PathBuf, u64>>>`), `pub fn last_tool(cache_dir) -> Option<u64>` / `remember_tool(cache_dir, tool)` (Datei `last-tool`, 16 Hex)
- Modify: `crates/g4-kernel/src/error.rs` (`ToolTimeout { name: &'static str, secs: u64 }`, Display "apicula did not finish within 60 s")
- Modify: `apps/studio/src-tauri/src/commands/models.rs:119-150` (`glb_with_fallback`: Tool-Hash bestimmen — mit apicula `tool_fingerprint`, ohne `last_tool`; kein `.fail` bei `ToolTimeout`), `:200-210` (`get_map_model_glb` als `pub async fn` mit `spawn_blocking`), `commands/buildings.rs:~200` (`get_building_glb` ebenso)
- Modify: `docs/dev-setup.md:31-45` (Modell-Cache: Salt v3, Werkzeug-Hash, `last-tool`, Timeout-Variable)
- Test: `crates/g4-kernel/tests/export.rs`, `apps/studio/src-tauri/src/commands/models.rs`

- [ ] **Step 1: Failing tests.** `export.rs`: `slow_apicula_times_out_without_fail_marker` — ein Skript `#!/bin/sh\nsleep 5` als apicula, `G4_APICULA_TIMEOUT_SECS=1` (im Test über eine `pub fn nsbmd_to_glb_with_timeout(..., Duration)` statt der Umgebungsvariable, die Variable liest nur der Default-Wrapper) → `Err(ToolTimeout{secs:1})` in < 3 s, Kindprozess beendet (`pgrep -f` nicht nötig: `Child::try_wait` nach `kill` ist `Some`); `cache_key_changes_with_the_tool` — zwei Dateien mit verschiedenem Inhalt → verschiedene Schlüssel, gleicher Inhalt → gleich; `last_tool_round_trips`. `models.rs`: `timeout_records_no_fail_marker` (mit gefälschtem apicula, Cache-Dir leer nach dem Fehler außer Arbeitsordner).

- [ ] **Step 2: Run → FAIL. Step 3: Implement** (`Command::spawn`, Schleife mit `try_wait` alle 50 ms bis Deadline, dann `kill` + `wait`; stdout/stderr über Pipes, nach Ende mit `wait_with_output`). **Step 4: Run → PASS; gated `export`-Tests mit `G4_APICULA`.**

- [ ] **Step 5: Commit** `fix(kernel,studio): apicula runs on a blocking worker with a timeout; cache keys carry the tool hash`.

---

### Task 12: Gate-Skript, Dokumentation, Roadmap

**Files:**
- Create: `scripts/gate.sh`
- Create: `docs/phase-3b1-gate.md`
- Modify: `docs/dev-setup.md:53-100` (Abschnitt „Gate-Skript“), `docs/ROADMAP.md` (Phase 3b in 3b.1–3b.4 aufteilen, 3b.1-Lieferumfang und Gate eintragen, Status „implementiert, Gate offen“, CI-Posten auf das Skript umschreiben; die falsche Behauptung „apicula in `spawn_blocking`“ unter Phase 2 korrigieren), `apps/studio/README.md` (ein Satz zu Sprites und Höhen)

`scripts/gate.sh` (bash, `set -euo pipefail`): Variablen `G4_ROMS_DIR` (Default `$(cd "$(dirname "$0")/.." && pwd)/../Roms`), `G4_APICULA` (Default `tools/apicula/bin/apicula`), `G4_DSROM_CLI` (Default `$G4_ROMS_DIR/_tools/dsrom`); Schritte und Projekt-Schleife wie Spec §7; Ergebnistabelle über `printf`; `trap` druckt die Tabelle auch bei Abbruch.

- [ ] **Step 1: Skript schreiben und einmal komplett laufen lassen** (dauert; im Hintergrund starten). Erwartung: alle Zeilen `ok`.
- [ ] **Step 2: Gate-Dokument** nach dem Muster von `docs/phase-3a-gate.md`: Vorbereitung (Kopien `pt-3b1`, `hg-3b1`), Teile (A) neuer Header per Assistent inkl. DSPRE-Gegenprobe, (B) Höhen, (C) Sprites, (D) Kachelnamen HGSS, (E) Automatik (`scripts/gate.sh`), leere Ergebnistabellen.
- [ ] **Step 3: Docs und Roadmap anpassen. Step 4: Commit** `docs: phase 3b.1 gate, gate script, roadmap split of phase 3b`.

---

## Self-Review

- Spec §2 → Tasks 1–5; §3 → Task 6; §4 → Tasks 7–9; §5 → Task 10; §6 → Task 11; §7 → Task 12; §8 Tests verteilt auf die Tasks.
- Namen über Tasks hinweg: `dynamic_headers::{plan, is_applied, apply, unpacked_dir}` (1) ← 2, 3; `Store::{apply_dynamic_headers, dynamic_headers_applied, create_header}` (3) ← 4; `AssignHeader.new_header`/`NewHeader`/`HeaderPatchDto` (4) ← 5; `cellY`/`cellYAtTile` (6) ← 9; `Nsbtx::{read, decode_with}` (7) ← 8; `SpriteCache::object_sprite`, `Facing`, `frame_index` (8) ← 9; `tile_behaviors(family)` (10); `tool_fingerprint`, `glb_cache_key(tool, …)` (11).
- Review Focus 1–5 haben je einen benannten Test in Task 2, 1, 2, 9, 11.
