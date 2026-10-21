# Phase 2b: Welt-Editoren. Implementierungsplan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Header, Matrizen, Area-Data, Gebäude und Events sind im Studio editierbar, mit Undo über alle Arten, sodass eine neue Route aus bestehenden Karten in einer neuen Matrix mit Warps zu einer bestehenden Stadt gebaut und im Emulator begangen werden kann.

**Architecture:** Der Store bekommt eine `Location`-Abstraktion, damit die Header-Tabelle in `arm9.bin` wie ein Asset behandelt wird (laden, ändern, Stale-Prüfung, atomar schreiben). Neue Member (Matrizen, Kartenkopien) werden sofort angelegt und danach wie alle Assets editiert. Die App bekommt pro Asset-Art einen Inspector-Editor, ein Matrix-Panel, texturierte Gebäudemodelle mit Auswahl und Verschieben, Event-Bearbeitung mit Warp-Zielen und eine Undo-Rückmeldung je Asset, mit der die Oberfläche gezielt nachlädt.

**Tech Stack:** wie Phase 2a. Keine neuen Abhängigkeiten.

**Spec:** [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](../specs/2026-10-21-gen4-studio-design.md). Roadmap: [docs/ROADMAP.md](../../ROADMAP.md). Voraussetzung: [Phase-2a-Plan](2026-10-21-phase-2a-foundation-and-orientation.md) vollständig gemergt.

## Global Constraints

- Wie Phase 2a (Repository, Branch `phase-2b` von `main`, Lizenzen, DSPRE-Layout, Codec-Regeln, `*_impl`-Muster, keine Arbeit unter dem Mutex, Vanilla-Originale nie beschreiben).
- Neue Header lassen sich in Vanilla-ROMs nicht anlegen (die Tabelle liegt fest im ARM9); Phase 2b ändert bestehende Header. Das steht im Header-Editor als Hinweis.
- Gebäude-Rotation wird vom Spiel bei Karten-Gebäuden ignoriert (`applyRotation = FALSE` in pret `map_prop.c`); sie wird verbatim erhalten und im Inspector nur als Hexwert angezeigt, nicht editiert.
- Matrix-Höhenabschnitt (`altitudes`) wird editierbar, aber von der 3D-Ansicht in dieser Phase nicht ausgewertet (Modelle liegen auf y = 0); Hinweis im Matrix-Panel.
- Neue Member entstehen nur am Ende eines Archivs (`index = member_count`), damit `pack_dir` keine Lücke sieht; Anlegen schreibt sofort auf die Platte und ist nicht per Undo rücknehmbar.

## Review Focus

1. Header-Edit auf einem Dump, dessen Header-Tabelle sich mit einer anderen Tabelle überlappt oder dessen `mapname.bin` mehr Einträge hat als die Tabelle: `save` schreibt genau `count × 24` Bytes an den Offset und nichts daneben. Test `header_table_save_touches_only_its_slice` in Task 1.
2. Matrix verkleinern, während Zellen Karten enthalten: die abgeschnittenen Zuordnungen gehen verloren, und der Nutzer wird vorher gefragt. Test `resize_matrix_keeps_overlap_and_fills_empty` in Task 4 (Kernel) plus Rückfrage in der UI.
3. Gebäude mit Modell-ID außerhalb des Archivs (Hack-ROMs): die Szene zeigt eine Box und der Inspector den Fehler, die Karte bleibt editierbar. Test `building_glb_for_unknown_model_is_tool_missing_not_panic` in Task 6.
4. Warp mit `dest_header_id` außerhalb der Header-Liste: Inspector zeigt „unbekannt (1234)“, „Zum Ziel“ ist inaktiv. Test `warp destination label for unknown header` in Task 7.
5. Undo nach einem Header-Edit, dessen Matrix-Wechsel die Szene neu geladen hat: die Szene springt zur alten Matrix zurück. Test `undo_reports_the_asset_it_restored` in Task 8 (Kernel) plus manuelle Prüfung.

---

## Dateistruktur

```
crates/g4-kernel/src/project.rs        Task 1 (Location, HeaderTable), Task 4 (create_matrix, duplicate_map Pfade)
crates/g4-kernel/src/store.rs          Tasks 1, 4, 8
crates/g4-kernel/tests/{store,headers}.rs
apps/studio/src-tauri/src/commands.rs  Tasks 2, 4, 5, 6, 7, 8 (ggf. in commands/{headers,matrix,area,buildings,events}.rs aufteilen, sobald commands.rs 600 Zeilen überschreitet)
apps/studio/src/api.ts
apps/studio/src/workbench/Inspector.tsx + inspector/{HeaderEditor,AreaEditor,BuildingEditor,EventEditor}.tsx
apps/studio/src/workbench/MatrixPanel.tsx, matrix.ts (+ test)
apps/studio/src/world/{MatrixScene.tsx,buildings.ts,drag.ts} (+ tests)
docs/phase-2-gate.md, docs/ROADMAP.md
```

---

### Task 1: Header-Tabelle als Asset

**Files:**
- Modify: `crates/g4-kernel/src/project.rs`, `crates/g4-kernel/src/store.rs`, `crates/g4-kernel/src/error.rs`, `crates/g4-kernel/tests/store.rs`
- Create: `crates/g4-kernel/tests/headers.rs`

**Interfaces:**
- Produces:

```rust
pub enum AssetKind { …, HeaderTable }          // label "header table"; nicht in ArchiveMap; member_count == 1; ensure_unpacked == Ok(1)
pub enum Location { Member(PathBuf), Slice { path: PathBuf, offset: u64, len: usize } }
impl Project {
    pub fn asset_location(&self, kind: AssetKind, index: u16) -> Result<Location, KernelError>;   // HeaderTable: Slice { arm9/arm9.bin, offset aus header_tables.yaml, len = header_count * 24 }; index != 0 → InvalidProject
    pub fn header_count(&self) -> Result<usize, KernelError>;                                   // mapname.bin / 16 (wie bisher in headers())
}
pub use uxie::map_header::{MapHeader, MapHeaderDP, MapHeaderPt, MapHeaderHGSS};
impl Store {
    pub fn header_table(&mut self) -> Result<&[MapHeader], KernelError>;
    pub fn edit_header(&mut self, id: u16, label: &str, edit: impl FnOnce(&mut MapHeader) -> Result<(), FormatError>) -> Result<ChangeId, KernelError>;   // id >= len → Format(OutOfRange { what: "header", .. })
}
```

Store liest und schreibt über `Location`: `Member` wie bisher; `Slice` liest `len` Bytes ab `offset`, prüft Stale nur auf dem Slice, schreibt die ganze Datei atomar (Temp unter `.g4/tmp`, Rename) mit ersetztem Slice. `Loaded::HeaderTable(Vec<MapHeader>)` serialisiert mit `write_map_header_to_bytes` je Eintrag.

- [ ] **Step 1: Failing Tests schreiben** (`tests/headers.rs`, synthetisches Projekt hat 2 Platin-Header an 0xE601C)

```rust
#[test]
fn header_table_save_touches_only_its_slice() {
    let s = common::synthetic_project();
    let p = Arc::new(Project::open(s.dir.path()).unwrap());
    let arm9_path = s.dir.path().join("arm9/arm9.bin");
    let before = std::fs::read(&arm9_path).unwrap();
    let mut st = Store::new(p.clone());
    assert_eq!(st.header_table().unwrap().len(), 2);
    st.edit_header(1, "matrix", |h| { if let MapHeader::Pt(h) = h { h.matrix_id = 7 } Ok(()) }).unwrap();
    assert!(st.is_dirty(AssetId { kind: AssetKind::HeaderTable, index: 0 }));
    st.save().unwrap();
    let after = std::fs::read(&arm9_path).unwrap();
    assert_eq!(before.len(), after.len());
    let off = 0xE601C;
    assert_eq!(before[..off], after[..off]);
    assert_eq!(before[off + 48..], after[off + 48..]);
    assert_eq!(&after[off + 24 + 2..off + 24 + 4], &7u16.to_le_bytes());
    assert_eq!(Project::open(s.dir.path()).unwrap().headers().unwrap()[1].matrix_id, 7);
}

#[test]
fn header_edit_out_of_range_is_an_error() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert!(matches!(st.edit_header(2, "x", |_| Ok(())).unwrap_err(), KernelError::Format(FormatError::OutOfRange { .. })));
}

#[test]
fn vanilla_header_table_round_trips_byte_identical() {   // gated, Kopie: laden, ohne Änderung ist nichts dirty; mit Null-Edit (Wert gleich) bleibt arm9 identisch
    let Some((_tmp, root)) = common::vanilla_copy() else { return };
    let mut st = Store::new(Arc::new(Project::open(&root).unwrap()));
    let n = st.header_table().unwrap().len();
    assert!(n > 500);
    assert!(st.dirty().is_empty());
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen** — Compile-Fehler.
- [ ] **Step 3: Implementieren.** `Project::headers()` bleibt als Leseweg für Listen (Namen); `location_names()` unverändert.
- [ ] **Step 4: Tests laufen lassen** — Kernel, Workspace, gated auf drei Projekten (Kopien), clippy.
- [ ] **Step 5: Commit** — `feat(kernel): header table as a store asset backed by an arm9 slice`

---

### Task 2: Header-Commands

**Files:**
- Modify: `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`

**Interfaces:**
- Commands: `get_header(id: u16) -> MapHeader` (uxie-Serde-Form, extern getaggt: `{"Pt": {...}}`), `set_header(id: u16, header: MapHeader) -> Result<(), CmdError>` (ersetzt den Eintrag über `edit_header`), `list_headers` liefert `matrix_id`, `area_data_id`, `event_file_id` ab jetzt aus `store.header_table()` (Namen weiter aus `Project::headers()`/`location_names()`), damit Änderungen vor dem Speichern sichtbar sind.
- `api.ts`: `type MapHeader = { DP: MapHeaderDP } | { Pt: MapHeaderPt } | { HGSS: MapHeaderHGSS }` mit den Feldern aus uxie `map_header/types.rs` (snake_case, exakt übernehmen), `getHeader(id)`, `setHeader(id, header)`, `headerFamily(h): "DP" | "Pt" | "HGSS"`, `headerFields(h): MapHeaderPt | …`.

- [ ] **Step 1: Failing Test schreiben**

```rust
#[test]
fn set_header_changes_the_list_before_saving() {
    let (_s, state) = open_synthetic();
    let mut h = get_header_impl(&state, 0).unwrap();
    if let MapHeader::Pt(p) = &mut h { p.matrix_id = 1; p.weather_id = 3; }
    set_header_impl(&state, 0, h.clone()).unwrap();
    assert_eq!(list_headers_impl(&state).unwrap()[0].matrix_id, 1);
    assert_eq!(get_header_impl(&state, 0).unwrap(), h);
    assert_eq!(dirty_impl(&state).unwrap(), vec![AssetId { kind: AssetKind::HeaderTable, index: 0 }]);
}
```

- [ ] **Step 2–5:** RED, implementieren, `cargo test -p studio`, `pnpm build`, Commit `feat(studio): header read and write commands`.

---

### Task 3: Header-Editor im Inspector

**Files:**
- Create: `apps/studio/src/workbench/inspector/HeaderEditor.tsx`, `apps/studio/src/workbench/inspector/fields.ts`, `fields.test.ts`
- Modify: `apps/studio/src/workbench/Inspector.tsx`, `apps/studio/src/App.tsx`, `apps/studio/src/workbench/HeaderList.tsx`

**Interfaces:**
- `InspectorModel` Variante `{ kind: "header"; id: number }`. Auswahl eines Headers in der Liste setzt sie (die Welt-Ansicht lädt weiterhin die Matrix); Hover auf Kacheln überschreibt sie nur, solange die Maus über der Szene ist, danach kehrt der Inspector zum Header zurück (Zustand `pinned: InspectorModel`, `hover: InspectorModel | null`).
- `fields.ts`: `headerFieldSpecs(family): FieldSpec[]` mit `FieldSpec = { key: string; label: string; kind: "u8" | "u16" | "bool" | "pick"; pick?: "matrix" | "areaData" | "eventFile" | "textArchive" | "script" | "header" }`; Labels Deutsch („Matrix“, „Area-Data“, „Skriptdatei“, „Level-Skript“, „Textarchiv“, „Musik Tag“, „Musik Nacht“, „Wilde Pokémon“, „Eventdatei“, „Ortsname“, „Wetter“, „Kamera“, „Kampfhintergrund“, „Flags“ …), Reihenfolge wie in der uxie-Struktur. `clampField(spec, value): number` begrenzt auf den Typbereich. Picker erhalten `max` aus `member_count` der jeweiligen Art (neuer Command `member_counts() -> Record<AssetKind, number>`).
- Jede Änderung ruft `setHeader` sofort (ein Changeset je Feld, Label = Feldname); ändert sich `matrix_id`, lädt App.tsx die Matrix neu.

- [ ] **Step 1: Failing Test** (`fields.test.ts`)

```ts
it("lists family-specific header fields in struct order and clamps values", () => {
  const pt = headerFieldSpecs("Pt").map(f => f.key);
  expect(pt.slice(0, 3)).toEqual(["area_data_id", "matrix_id", "script_file_id"]);
  expect(pt).toContain("area_icon"); expect(pt).not.toContain("worldmap_x");
  expect(headerFieldSpecs("HGSS").map(f => f.key)).toContain("kanto_flag");
  expect(clampField({ key: "weather_id", label: "", kind: "u8" }, 300)).toBe(255);
  expect(clampField({ key: "matrix_id", label: "", kind: "pick", pick: "matrix" }, -1)).toBe(0);
});
```

- [ ] **Step 2–5:** RED, implementieren, `pnpm test && pnpm build`; manuell: Header 0415 Matrix auf 126 setzen → Szene zeigt das Rivalenzimmer; Cmd+Z → zurück. Commit `feat(studio): header editor in the inspector`.

---

### Task 4: Matrix-Panel mit Bearbeitung, neue Matrix, Kartenkopie

**Files:**
- Modify: `crates/g4-kernel/src/store.rs`, `crates/g4-kernel/tests/store.rs`, `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/workbench/layout.ts` (Panel `matrix` als Tab neben `world`)
- Create: `apps/studio/src/workbench/MatrixPanel.tsx`, `apps/studio/src/workbench/matrix.ts`, `matrix.test.ts`

**Interfaces:**
- Kernel:

```rust
impl Store {
    pub fn create_matrix(&mut self, name: &str) -> Result<u16, KernelError>;     // 1 × 1, maps [MATRIX_EMPTY], keine Header-/Höhenabschnitte; schreibt unpacked/matrices/{count:04} sofort; lädt es
    pub fn duplicate_map(&mut self, source: u16) -> Result<u16, KernelError>;    // Kopie der Bytes als neues Member am Ende; sofort geschrieben
    pub fn member_count(&self, kind: AssetKind) -> Result<usize, KernelError>;   // Projekt-Zählung plus hier erzeugte Member
}
pub fn resize_matrix(m: &Matrix, width: u8, height: u8) -> Matrix;  // in g4_formats::matrix: Überlappung bleibt, neue Zellen MATRIX_EMPTY (maps) / 0 (headers, altitudes)
```

- Commands: `set_matrix_cell(index, x, y, map: number | null, header: number | null, altitude: number | null)` (nur gesetzte Felder ändern; `header`/`altitude` nur, wenn der Abschnitt existiert), `set_matrix_sections(index, headers: bool, altitudes: bool)`, `resize_matrix(index, width, height)`, `rename_matrix(index, name)`, `create_matrix(name) -> u16`, `duplicate_map(source) -> u16`.
- `matrix.ts`: `cellLabel(map: number | null, header: number | null): string` („—“ für leer, sonst `M0570` plus ` H0415`), `resizePreview(width, height, old: MatrixDto): { lostCells: number }` für die Rückfrage.
- Panel: Tabelle `height × width`, Zelle zeigt `cellLabel`, Klick wählt Zelle (Inspector-Variante `{ kind: "matrixCell"; matrix; x; y }` mit Eingaben Karte, Header, Höhe und Knopf „Karte duplizieren und hier eintragen“); Kopfzeile: Name, Breite, Höhe, Umschalter Header/Höhen, „Neue Matrix“. Verkleinern mit `lostCells > 0` fragt per `ask`.

- [ ] **Step 1: Failing Tests**

```rust
#[test]
fn resize_matrix_keeps_overlap_and_fills_empty() {
    let m = Matrix { name: "m".into(), width: 2, height: 2, headers: Some(vec![1, 2, 3, 4]), altitudes: None, maps: vec![10, 11, 12, 13] };
    let r = resize_matrix(&m, 3, 1);
    assert_eq!(r.maps, vec![10, 11, MATRIX_EMPTY]);
    assert_eq!(r.headers, Some(vec![1, 2, 0]));
    r.validate().unwrap();
}

#[test]
fn create_matrix_and_duplicate_map_append_members() {
    let (s, mut st) = store();
    let n = st.member_count(AssetKind::Matrix).unwrap();
    let idx = st.create_matrix("neu").unwrap();
    assert_eq!(idx as usize, n);
    assert!(s.dir.path().join(format!("unpacked/matrices/{idx:04}")).exists());
    assert_eq!(st.matrix(idx).unwrap().maps, vec![MATRIX_EMPTY]);
    let m2 = st.duplicate_map(1).unwrap();
    assert_eq!(std::fs::read(s.dir.path().join(format!("unpacked/maps/{m2:04}"))).unwrap(), std::fs::read(s.dir.path().join("unpacked/maps/0001")).unwrap());
    st.edit_matrix(idx, "assign", |m| { m.maps[0] = m2; Ok(()) }).unwrap();
    st.save().unwrap();
    assert_eq!(g4_kernel::build::pack_dir(&s.dir.path().join("unpacked/matrices")).unwrap().members.len(), n + 1);
}
```

```ts
it("labels cells and counts lost cells on shrink", () => {
  expect(cellLabel(null, null)).toBe("—");
  expect(cellLabel(570, 415)).toBe("M0570 H0415");
  expect(resizePreview(1, 1, { index: 0, name: "", width: 2, height: 2, maps: [1, 2, null, 4] })).toEqual({ lostCells: 2 });
});
```

- [ ] **Step 2–5:** RED, implementieren, Tests, `pnpm build`; manuell: neue Matrix 2 × 1 aus zwei duplizierten Karten, Header 0001 („NOTHING“) darauf umstellen, Szene zeigt beide Karten nebeneinander. Commit `feat: matrix panel with cell editing, new matrices and map duplication`.

---

### Task 5: Area-Data-Editor

**Files:**
- Create: `apps/studio/src/workbench/inspector/AreaEditor.tsx`
- Modify: `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/workbench/Inspector.tsx`

**Interfaces:**
- Commands: `get_area_data(id: u16) -> AreaDataDto`, `set_area_data(id, dto)` mit

```rust
#[derive(serde::Serialize, serde::Deserialize)] #[serde(tag = "family")]
pub enum AreaDataDto { Dppt { buildings_tileset: u16, map_tileset: u16, unknown: u16, light_type: u16 }, Hgss { buildings_tileset: u16, map_tileset: u16, dynamic_texture_type: u16, area_type: u8, light_type: u8 } }
```

- Inspector-Variante `{ kind: "area"; id: number }`, erreichbar über den Header-Editor („Area-Data 0020 öffnen“). Picker für `map_tileset` (max `member_count(MapTexture)`), `buildings_tileset` (`BuildingTexture`), HGSS `area_type` als Auswahl „Innen (0) / Außen (1)“. Nach `map_tileset`-Änderung lädt die Szene die Modelle neu (Cache-Schlüssel ändert sich automatisch).

- [ ] **Step 1: Failing Test**

```rust
#[test]
fn area_data_commands_round_trip() {
    let (_s, state) = open_synthetic();
    let mut a = get_area_data_impl(&state, 1).unwrap();
    if let AreaDataDto::Dppt { map_tileset, .. } = &mut a { *map_tileset = 3 }
    set_area_data_impl(&state, 1, a.clone()).unwrap();
    assert_eq!(get_area_data_impl(&state, 1).unwrap(), a);
    assert!(dirty_impl(&state).unwrap().contains(&AssetId { kind: AssetKind::AreaData, index: 1 }));
}
```

- [ ] **Step 2–5:** RED, implementieren, Tests, Commit `feat(studio): area data editor`.

---

### Task 6: Gebäude: texturierte Modelle, Auswahl, Bearbeitung

**Files:**
- Create: `apps/studio/src/world/buildings.ts`, `buildings.test.ts`, `apps/studio/src/world/drag.ts`, `drag.test.ts`, `apps/studio/src/workbench/inspector/BuildingEditor.tsx`
- Modify: `crates/g4-kernel/src/project.rs`, `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/world/MatrixScene.tsx`, `apps/studio/src/workbench/Inspector.tsx`

**Interfaces:**
- Kernel: `Project::building_model_bytes(&self, model_id: u32, interior: bool) -> Result<Option<Vec<u8>>, KernelError>` (interior nur HGSS und nur, wenn `has_archive(InteriorBuildingModel)`; sonst exterior; `None` bei Index außerhalb), `Project::building_tileset_bytes(&self, area_data_id: u8) -> Result<Option<Vec<u8>>, KernelError>` (`buildings_tileset` → `BuildingTexture`).
- Commands: `get_building_glb(model_id: u32, area_data_id: u8) -> Response` (interior = HGSS und `area_type == 0`; Cache über `glb_cached`; Modell außerhalb → `CmdError { code: "tool_missing" | "error", message: "building model 1234 does not exist" }`), `update_building(map: u16, index: u16, patch: BuildingPatch)`, `add_building(map: u16) -> u16` (Modell 0, Position 0, Skalierung 1.0, Rotation 0, Dummy 0), `delete_building(map: u16, index: u16)`, `member_counts` liefert auch `BuildingModel`/`InteriorBuildingModel`.

```rust
#[derive(serde::Deserialize)] pub struct BuildingPatch { pub model_id: Option<u32>, pub x: Option<f32>, pub y: Option<f32>, pub z: Option<f32>, pub sx: Option<f32>, pub sy: Option<f32>, pub sz: Option<f32> }
// Umrechnung: position[i] = (tiles * 65536.0).round() as i32; scale[i] = (factor * 4096.0).round() as i32; Rotation unverändert.
```

- `buildings.ts`: `buildingTransform(b: BuildingDto, cell: {x; z}): { position: [number, number, number]; scale: [number, number, number] }` (Position wie `buildingBox`-Mitte in Welt-Einheiten; Modell-GLB in 1/16-Skala wie Karten, multipliziert mit `sx, sy, sz`); `snapHalfTile(v: number): number`.
- `drag.ts`: `planeDrag(start: {x; z}, current: {x; z}, origin: {x; z}): {x; z}` mit Snap auf 0,5 Kacheln; wird für Gebäude benutzt (Task 7 für Events mit Snap 1).
- Szene: Gebäude als GLB-Instanzen (Fallback Box bei Fehler), Klick wählt (Raycast gegen Gebäude vor Kacheln), Auswahl mit Umriss; Drag bei gewähltem Gebäude verschiebt in x/z (Snap 0,5) und ruft am Pointer-Up einmal `update_building`; Entf löscht nach Rückfrage. Inspector-Variante `{ kind: "building"; map: number; index: number }` mit Modell-Picker (Nummer + „Vorschau laden“), x/y/z (Schritt 0,5), Skalierung, Rotation als Hex nur lesbar, „Löschen“, „Neues Gebäude“ (auf der zuletzt gehoverten Kachel).

- [ ] **Step 1: Failing Tests**

```rust
#[test]
fn update_building_converts_tiles_and_scale() {
    let (_s, state) = open_synthetic();
    update_building_impl(&state, 0, 0, BuildingPatch { x: Some(-13.5), sy: Some(2.0), ..Default::default() }).unwrap();
    let m = get_map_impl(&state, 0).unwrap();
    assert_eq!((m.buildings[0].x, m.buildings[0].sy), (-13.5, 2.0));
    let idx = add_building_impl(&state, 0).unwrap();
    assert_eq!(idx, 1);
    delete_building_impl(&state, 0, 1).unwrap();
    assert_eq!(get_map_impl(&state, 0).unwrap().buildings.len(), 1);
}

#[test]
fn building_glb_for_unknown_model_is_tool_missing_not_panic() {
    let (_s, state) = open_synthetic();
    let err = get_building_glb_impl(&state, 99_999, 0).unwrap_err();
    assert!(err.message.contains("99999"));
}
```

```ts
it("snaps drags to half tiles", () => {
  expect(planeDrag({ x: 0, z: 0 }, { x: 1.3, z: -0.7 }, { x: 10, z: 10 })).toEqual({ x: 11.5, z: 9.5 });
});
it("places a building model with the map scale times its own scale", () => {
  const t = buildingTransform({ model_id: 1, x: -4, y: 0, z: 6, sx: 2, sy: 1, sz: 3 }, { x: 64, z: 32 });
  expect(t.position).toEqual([76, 0, 54]);
  expect(t.scale).toEqual([2 / 16, 1 / 16, 3 / 16]);
});
```

- [ ] **Step 2–5:** RED, implementieren, Tests (gated: `vanilla_building_glb_has_images` für PT_DE Header 0415, Gebäude 0, `images > 0`), `pnpm build`; manuell: Haus in Zweiblattdorf (Header 0411) anklicken, verschieben, Undo. Commit `feat(studio): textured building models with selection, drag and inspector editing`.

---

### Task 7: Events bearbeiten

**Files:**
- Create: `apps/studio/src/workbench/inspector/EventEditor.tsx`, `apps/studio/src/world/events.test.ts` (erweitern)
- Modify: `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/world/MatrixScene.tsx`, `apps/studio/src/world/events.ts`, `apps/studio/src/workbench/Inspector.tsx`

**Interfaces:**
- Commands (alle über `Store::edit_event`): `update_event(event_file: u16, kind: EventKind, index: u16, patch: EventPatch)`, `add_event(event_file, kind, x: i32, z: i32) -> u16`, `delete_event(event_file, kind, index)`.

```rust
#[derive(serde::Deserialize, Clone, Copy)] #[serde(rename_all = "lowercase")] pub enum EventKind { Bg, Object, Warp, Coord }
#[derive(serde::Deserialize, Default)] pub struct EventPatch { pub x: Option<i32>, pub z: Option<i32>, pub script: Option<u16>, pub event_type: Option<u16>, pub graphics_id: Option<u16>, pub movement_type: Option<u16>, pub local_id: Option<u16>, pub dest_header_id: Option<u16>, pub dest_warp_id: Option<u16>, pub height: Option<u32>, pub width: Option<u16>, pub length: Option<u16> }
// Felder, die die Art nicht hat, werden ignoriert; x/z werden in den Feldtyp der Art geschnitten (u16 bzw. i16), außerhalb → Format(OutOfRange).
```

Neue Events: Warp `{ x, z, dest_header_id: 0, dest_warp_id: 0, height: 0 }`, Objekt `{ local_id: nächste freie, graphics_id: 0, movement_type: 0, trainer_type: 0, hidden_flag: 0, script: 0, dir: 0, data: [0;3], movement_range_x: 0, movement_range_z: 0, x, z, y: 0 }`, Coord `{ script: 0, x, z, width: 1, length: 1, y: 0, value: 0, var: 0 }`, Bg `{ script: 0, event_type: 0, x, z, y: 0, player_facing_dir: 0 }`.

- `events.ts`: `warpDestinationLabel(dest: number, headers: HeaderSummary[]): { text: string; known: boolean }` → bekannt: `"0414 Zweiblattdorf (T01R0201)"`, unbekannt: `"unbekannt (1234)"`.
- Szene: Marker anklickbar (Inspector-Variante `event` aus Phase 2a wird „gepinnt“), Drag verschiebt mit Snap 1 Kachel und ruft `update_event` am Pointer-Up; Coord-Events zeigen ihren Bereich `width × length` als flache Fläche. Inspector: Felder je Art, Warp-Ziel als Header-Picker mit Suche plus „Zum Ziel“ (wählt den Zielheader in der Liste), „Löschen“, „Hinzufügen“ (Art wählen, auf der zuletzt gehoverten Kachel).

- [ ] **Step 1: Failing Tests**

```rust
#[test]
fn event_commands_add_update_delete() {
    let (_s, state) = open_synthetic();
    let i = add_event_impl(&state, 0, EventKind::Warp, 8, 4).unwrap();
    update_event_impl(&state, 0, EventKind::Warp, i, EventPatch { dest_header_id: Some(1), ..Default::default() }).unwrap();
    let ev = get_events_impl(&state, 0).unwrap();
    assert_eq!((ev.warps[0].x, ev.warps[0].z, ev.warps[0].dest_header_id), (8, 4, 1));
    assert!(update_event_impl(&state, 0, EventKind::Warp, i, EventPatch { x: Some(-1), ..Default::default() }).is_err());
    delete_event_impl(&state, 0, EventKind::Warp, i).unwrap();
    assert!(get_events_impl(&state, 0).unwrap().warps.is_empty());
}
```

```ts
it("labels warp destinations and flags unknown headers", () => {
  const hs = [{ id: 414, internal_name: "T01R0201", location_name: "Zweiblattdorf", matrix_id: 126, area_data_id: 20, event_file_id: 1 }];
  expect(warpDestinationLabel(414, hs)).toEqual({ text: "0414 Zweiblattdorf (T01R0201)", known: true });
  expect(warpDestinationLabel(1234, hs)).toEqual({ text: "unbekannt (1234)", known: false });
});
```

- [ ] **Step 2–5:** RED, implementieren, Tests, `pnpm build`; manuell: Warp in Zweiblattdorf verschieben, Ziel ändern, „Zum Ziel“. Commit `feat(studio): event editing with warp destinations`.

---

### Task 8: Undo meldet das betroffene Asset

**Files:**
- Modify: `crates/g4-kernel/src/store.rs`, `crates/g4-kernel/tests/store.rs`, `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/App.tsx`

**Interfaces:**
- `Store::undo(&mut self) -> Option<AssetId>`, `Store::redo(&mut self) -> Option<AssetId>` (statt `ChangeId`; `ChangeId` bleibt Rückgabe von `edit_*`).
- Commands `undo`/`redo` → `AssetId | null`. App.tsx lädt je `kind` nach: `Map` → Karte neu holen, Overlay und Gebäude der Karte ersetzen; `Matrix` → Matrix und Szene neu; `Event` → Events neu; `AreaData` → Inspector neu, Modelle neu; `HeaderTable` → Header-Liste neu, bei geänderter Matrix des gewählten Headers Szene neu.

- [ ] **Step 1: Failing Test**

```rust
#[test]
fn undo_reports_the_asset_it_restored() {
    let (_s, mut st) = store();
    st.edit_map(1, "a", |m| m.set_cell(0, 0, Cell { kind: 1, collision: 0 })).unwrap();
    st.edit_matrix(0, "b", |m| { m.maps[0] = 2; Ok(()) }).unwrap();
    assert_eq!(st.undo(), Some(AssetId { kind: AssetKind::Matrix, index: 0 }));
    assert_eq!(st.undo(), Some(AssetId { kind: AssetKind::Map, index: 1 }));
    assert_eq!(st.undo(), None);
    assert_eq!(st.redo(), Some(AssetId { kind: AssetKind::Map, index: 1 }));
}
```

- [ ] **Step 2–5:** RED, implementieren (CLI und bestehende Tests anpassen), Tests, Commit `feat: undo and redo report the restored asset`.

---

### Task 9: Querverlinkung und Listen-Synchronisation

**Files:**
- Modify: `apps/studio/src/App.tsx`, `apps/studio/src/workbench/{HeaderList,MatrixPanel}.tsx`, `apps/studio/src/workbench/inspector/*.tsx`

**Interfaces:**
- Eine zentrale Navigationsfunktion `goTo(target: { kind: "header"; id } | { kind: "matrixCell"; matrix; x; y } | { kind: "area"; id } | { kind: "building"; map; index } | { kind: "event"; eventFile; eventKind; index })` in App.tsx, die Auswahl, Panel-Fokus (Dockview `api.getPanel(id)?.focus()`) und Inspector setzt.
- Verlinkungen: Matrix-Zelle mit Header-ID → Header; Header-Editor → Area-Data, Matrix-Panel; Warp → Zielheader; Header-Liste markiert Header, deren Matrix gerade angezeigt wird; `matrixCell`-Inspector zeigt „Header 0415 öffnen“, wenn die Zelle einen Header trägt.

- [ ] **Step 1: Failing Test** (`navigation.test.ts`, reine Reducer-Funktion `nextSelection(state, target)` ohne React)

```ts
it("matrix cell with a header navigates to that header", () => {
  const next = nextSelection({ selectedHeader: 3, matrix: 0 }, { kind: "matrixCell", matrix: 0, x: 1, y: 0, header: 415 });
  expect(next.selectedHeader).toBe(415);
});
```

- [ ] **Step 2–5:** RED, implementieren, `pnpm test && pnpm build`, Commit `feat(studio): cross-links between headers, matrices, areas, buildings and events`.

---

### Task 10: Gate-Prüfung und Dokumentation

**Files:**
- Create: `docs/phase-2-gate.md`
- Modify: `docs/ROADMAP.md`, `docs/dev-setup.md`

Inhalt von `docs/phase-2-gate.md` (Deutsch): Auf einer Kopie von Platin DE:
1. Zwei Karten duplizieren (z. B. die beiden Karten von Route 201, Header 0008 „Route 201“ zeigt die Matrix-Zellen), neue Matrix 2 × 1 mit den Kopien anlegen.
2. Header 0001 („NOTHING“) auf die neue Matrix, Area-Data von Route 201 und ein freies Event-File umstellen; Namen bleiben (keine neuen Header).
3. In Zweiblattdorf (Header 0411) einen Warp hinzufügen, Ziel Header 0001, Warp 0; in der neuen Route einen Warp zurück nach 0411.
4. Speichern, bauen, in melonDS: durch den Warp in die neue Route laufen, beide Karten begehbar, zurückwarpen.
5. Undo-Kette: fünf Schritte zurück und wieder vor, Szene und Inspector folgen.
6. Automatik: `cargo test --workspace --all-features`, `pnpm test`, clippy, fmt, gated Tests auf drei Projekten.

ROADMAP: Phase 2 als abgeschlossen markieren, offene Reste (Altitudes in der Szene, Objekt-Sprites, neue Header via Patch) nach Phase 3 schieben.

- [ ] **Step 1–3:** Dokumente schreiben, Gate durchführen (Ergebnis mit Datum in `phase-2-gate.md` eintragen), Commit `docs: phase 2 gate and roadmap update`.

---

## Self-Review

**Spec-Abdeckung (Roadmap Phase 2):** Header-Editor (1–3), Matrix-Editor inkl. neue Matrix (4), Gebäude mit Modellwahl/Position/Skalierung und Vorschau (6; Rotation bewusst nur lesbar), Events platzieren/verschieben/Warps verbinden (7), Area-Data (5), Workbench (2a), Undo über alle Arten (8), „Open in …“ (9), Gate (10). Nicht enthalten: Objekt-Sprites (Phase 3), Höhen in der Szene, neue Header.

**Typkonsistenz:** `Location`/`HeaderTable` (1) → `get_header`/`set_header` (2) → Header-Editor (3); `member_counts` (3) auch in 5 und 6; `BuildingPatch` (6), `EventPatch`/`EventKind` (7); `undo -> Option<AssetId>` (8) ersetzt die Phase-1-Signatur, Aufrufer in CLI und Commands angepasst; `InspectorModel`-Varianten `header` (3), `matrixCell` (4), `area` (5), `building` (6), `event` (2a/7).

**Review Focus:** 1 → Task 1, 2 → Task 4, 3 → Task 6, 4 → Task 7, 5 → Task 8.
