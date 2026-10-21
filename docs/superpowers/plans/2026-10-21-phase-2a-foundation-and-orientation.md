# Phase 2a: Fundament, Bundle und Orientierung. Implementierungsplan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die Eingangskriterien aus dem Phase-1-Review erledigen (Kernel-Verträge, Stale-Auflösung, Cmd+Q, Bundle) und die Welt-Ansicht so weit orientierbar machen, dass ein Nutzer ohne Raten Treppen, Warps, Events und Kachelkoordinaten erkennt und texturierte Karten sieht.

**Architecture:** Der Kernel bekommt alle Archivarten des Welt-Editors (Events, Texturen, Gebäudemodelle, Textarchive) in `ArchiveMap`, typisierte Fehler mit Kontext, einen Store, der selbst entpackt, Edit-Closures mit `Result`, und einen Modell-Cache nach Inhalts-Hash mit optionalem Tileset. Die Tauri-App bekommt strukturierte Fehler für den Stale-Fall, einen Exit-Guard auf Rust-Seite, eine Dockview-Workbench mit Inspector, Kachel-Hover, Event-Marker, lesbare Header-Namen und ein lauffähiges App-Bundle.

**Tech Stack:** wie Phase 1 (Rust stable Edition 2024, `uxie 0.9`, `ds-rom` Git-Pin, Tauri 2.12, React 19, three 0.186, vitest) plus `xxhash-rust 0.8` (Feature `xxh3`), `dockview 8.6`, `serde_yaml 0.9`.

**Spec:** [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](../specs/2026-10-21-gen4-studio-design.md). Roadmap: [docs/ROADMAP.md](../../ROADMAP.md). Vorgänger: [Phase-1-Plan](2026-10-21-phase-1-kernel-and-shell.md). Nachfolger: [Phase-2b-Plan](2026-10-21-phase-2b-editors.md).

## Global Constraints

- Repository `/Users/POLTEP/Projekte/lab/editor/gen4-studio`, Branch `phase-2a` von `main`. Cargo-Workspace `crates/g4-formats`, `crates/g4-kernel`, `apps/studio/src-tauri`. Cargo via `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.
- Lizenz MIT. Keine GPL- oder AGPL-Crates. `nitroarc` (LGPL) bleibt eine separate Shared Library.
- Keine ROM-Daten im Repository. Gated Tests lesen `G4_TEST_PROJECT`; schreibende Tests arbeiten nur auf `g4_kernel::test_support::vanilla_copy()`. Testprojekte: PT_DE `/Users/POLTEP/Projekte/lab/editor/Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents` (CPUD), HG_DE `…/Pokemon - Goldene Edition HeartGold (Germany)_DSPRE_contents` (IPKD), HG_US `…/Pokemon - HeartGold Version (USA)_DSPRE_contents` (IPKE). Gated Tests laufen einmal pro Projekt.
- DSPRE-Layout bleibt verbindlich: entpackte Member `{:04}` unter `unpacked/<Name>`; Namen `maps`, `matrices`, `areaData`, `eventFiles`, `textArchives`, `mapTextures`, `buildingTextures`, `exteriorBuildingModels`, `interiorBuildingModels`.
- Codecs erhalten unbekannte Bytes verbatim; `read` lehnt ab, was `write` nicht reproduziert; `validate()` vor jedem Schreiben.
- Kein `unwrap()` außerhalb von Tests und `test_support`. Kernel-Fehlertexte Englisch, UI-Texte Deutsch, Doku Deutsch, Commits Conventional Commits auf Englisch.
- Tauri-Commands: dünner `#[tauri::command]`-Wrapper über einer getesteten `*_impl(&AppState, ..)`. Lange Arbeit (apicula, ds-rom) nie unter dem Mutex.
- Vanilla-Originale werden nie beschrieben. Die App schreibt in ein geöffnetes Projekt nur `unpacked/<Kind>` beim Entpacken, `.g4/` und beim Speichern die Member.

## Review Focus

1. Ein Projekt, in dem DSPRE parallel eine Karte geändert hat: Speichern meldet den Stale-Fall je Asset mit Klartext und bietet „Neu laden“ und „Meine Version behalten“, ohne fremde Daten still zu überschreiben. Test `stale_recovery_reload_and_accept` in Task 4 und `save_stale_returns_structured_error` in Task 5.
2. Cmd+Q bei ungespeicherten Änderungen: Rückfrage statt Datenverlust. Test `exit_is_blocked_only_with_dirty_assets_and_without_force` in Task 6; der Rest ist manuell (Gate).
3. Karten ohne Texturen im Tileset oder mit fehlendem Area-Data-Eintrag: die Karte erscheint untexturiert, die Szene bricht nicht ab. Test `glb_cached_without_tileset_still_converts` in Task 7.
4. Ein Event mit Koordinaten außerhalb der Matrix (z. B. Warp auf x = 200 bei 1 × 1): der Marker wird nicht gezeichnet, die restlichen Events schon. Test `markerPosition returns null outside the matrix` in Task 10.
5. Eine Matrix mit mehr als 400 Zellen (Platin-Matrix 0 hat 900): das Laden blockiert die Oberfläche nicht, höchstens vier Konvertierungen laufen gleichzeitig, Fortschritt ist sichtbar. Test `limitConcurrency never exceeds the cap` in Task 12.

---

## Dateistruktur

```
crates/g4-formats/src/{matrix,map}.rs           Task 1 (Tests, validate)
crates/g4-kernel/src/error.rs                    Task 2 (IoAt, InvalidProject, NotEditable, Display)
crates/g4-kernel/src/project.rs                  Task 3 (ArchiveMap für alle Kinds, Display für AssetId), Task 11 (location_names, HeaderSummary)
crates/g4-kernel/metadata/archives.yaml          Task 3
crates/g4-kernel/metadata/tile_behaviors.yaml    Task 9
crates/g4-kernel/src/store.rs                    Task 4
crates/g4-kernel/src/build.rs                    Task 3 (alle vorhandenen Kinds packen)
crates/g4-kernel/src/export.rs                   Task 7 (glb_cached)
crates/g4-kernel/src/tiles.rs                    Task 9 (TileBehavior-Tabelle)
crates/g4-kernel/src/bin/g4k.rs                  Task 4 (Result-Closure), Task 11 (info zeigt Ortsnamen)
apps/studio/src-tauri/src/{commands,state,lib}.rs   Tasks 5–11
apps/studio/src-tauri/src/errors.rs              Task 5 (CmdError)
apps/studio/src-tauri/build.rs, tauri.conf.json  Task 13
apps/studio/scripts/prepare-bundle.sh            Task 13
apps/studio/src/api.ts                           Tasks 5, 7, 9, 10, 11
apps/studio/src/App.tsx, Toolbar.tsx             Tasks 5, 6, 8, 11, 12
apps/studio/src/workbench/{Workbench.tsx,layout.ts,Inspector.tsx}   Task 8
apps/studio/src/world/{MatrixScene.tsx,hover.ts,events.ts,load.ts}  Tasks 7, 9, 10, 12
docs/formats/matrix.md, docs/dev-setup.md, docs/release.md, docs/ROADMAP.md
```

---

### Task 1: Codec-Korrekturen aus dem Phase-1-Review

**Files:**
- Modify: `crates/g4-formats/src/map.rs` (`validate`), `crates/g4-formats/tests/vanilla.rs`, `docs/formats/matrix.md`, `docs/ROADMAP.md`

**Interfaces:**
- Consumes: `Matrix { width, height, maps }`, `Matrix::map_at`, `MapFile::validate`.
- Produces: unveränderte Signaturen; `MapFile::validate` lehnt `buildings_tail.len() >= 48` mit `FormatError::BadSize { expected: 47, found }` ab.

Faktenlage: Die Roadmap vermutete, Byte 0 einer Matrix sei die Höhe. Das Decomp-JSON `res/field/matrices/map_matrix_240.json` (Platin, Byte 0 = 4, Byte 1 = 5) zeigt 5 Zeilen zu je 4 Karten, also ist Byte 0 die Zeilenlänge (Breite). Unsere Implementierung ist richtig; das wird mit einem Test gegen Vanilla-Daten festgenagelt.

- [ ] **Step 1: Failing Tests schreiben**

In `tests/vanilla.rs` (gated, nur PT_DE hat Matrix 240 in dieser Form; auf HGSS früh mit `eprintln!("skipped: not Platinum")` zurückkehren, wenn `family_of(&root) != Family::Pt`):

```rust
#[test]
fn vanilla_platinum_matrix_240_is_four_wide_five_high() {
    let Some(root) = vanilla() else { return };
    if family_of(&root) != Family::Pt { eprintln!("skipped: not Platinum"); return }
    let m = Matrix::read(&std::fs::read(root.join("unpacked/matrices/0240")).unwrap()).unwrap();
    assert_eq!((m.width, m.height), (4, 5));
    assert_eq!(m.map_at(0, 0), Some(176));
    assert_eq!(m.map_at(1, 1), Some(570));
    assert_eq!(m.map_at(2, 1), Some(571));
    assert_eq!(m.map_at(1, 3), Some(574));
    assert_eq!(m.map_at(4, 0), None);
}
```

In `map.rs` Modultests:

```rust
#[test]
fn validate_rejects_buildings_tail_of_a_whole_building() {
    let mut map = MapFile::read(&synthetic(Family::Pt, None), Family::Pt).unwrap();
    map.buildings_tail = vec![0; 48];
    assert!(matches!(map.validate().unwrap_err(), FormatError::BadSize { expected: 47, found: 48 }));
    map.buildings_tail = vec![0x0D, 0x0A];
    map.validate().unwrap();
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-formats validate_rejects_buildings_tail` und `G4_TEST_PROJECT=<PT_DE> cargo test -p g4-formats --test vanilla matrix_240`
Expected: der erste schlägt fehl (`validate` akzeptiert 48 Bytes); der zweite besteht bereits oder schlägt fehl, falls die Orientierung doch falsch wäre.

- [ ] **Step 3: `validate` ergänzen; `docs/formats/matrix.md` um den Absatz „Orientierung“ (Byte 0 = Breite, Beleg Matrix 240) erweitern; in `docs/ROADMAP.md` die Zeile „Matrix-Dimensionen prüfen …“ durch „Matrix-Orientierung bestätigt (Byte 0 = Breite), Test in g4-formats“ ersetzen**

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-formats` und die gated Vanilla-Tests auf PT_DE, HG_DE, HG_US
Expected: alle `ok`.

- [ ] **Step 5: Commit**

```bash
git add crates/g4-formats docs/formats/matrix.md docs/ROADMAP.md
git commit -m "fix(formats): reject building tails of a whole building, pin matrix orientation"
```

---

### Task 2: Kernel-Fehler mit Kontext

**Files:**
- Modify: `crates/g4-kernel/src/error.rs`, `crates/g4-kernel/src/project.rs`, `crates/g4-kernel/src/store.rs`, `crates/g4-kernel/src/build.rs`, `crates/g4-kernel/src/export.rs`
- Test: `crates/g4-kernel/src/error.rs` (Modultests)

**Interfaces:**
- Produces:

```rust
impl std::fmt::Display for AssetId   // "map 0005", "matrix 0001", "area data 0002", "event file 0003", ...
impl AssetKind { pub fn label(self) -> &'static str }   // "map" | "matrix" | "area data" | "event file" | "text archive" | "map texture" | "building texture" | "building model" | "interior building model"

pub enum KernelError {
    NotAProject(PathBuf),
    UnknownGame(String),
    #[error("{path}: {source}")] IoAt { path: PathBuf, #[source] source: std::io::Error },
    #[error(transparent)] Io(#[from] std::io::Error),            // bleibt für `?` ohne Pfad; neue Aufrufe nutzen IoAt
    #[error(transparent)] Format(#[from] FormatError),
    #[error("uxie: {0}")] Uxie(String),
    #[error("tool {name} unavailable: {reason}")] ToolMissing { name: &'static str, reason: String },
    #[error("invalid project metadata: {0}")] InvalidProject(String),   // header.yaml, archives.yaml, header_tables.yaml
    #[error("{0} cannot be edited")] NotEditable(AssetKind),
    #[error("build: {0}")] Build(String),
    #[error("{0} changed on disk since it was loaded")] Stale(AssetId),
}

pub fn io_at(path: &Path) -> impl FnOnce(std::io::Error) -> KernelError + '_   // für `.map_err(io_at(&p))?`
```

(`AssetKind` erhält die neuen Varianten erst in Task 3; `label()` wird dort erweitert. In diesem Task gibt es `Map`, `Matrix`, `AreaData`.)

- [ ] **Step 1: Failing Tests schreiben**

```rust
#[test]
fn error_messages_name_asset_and_path() {
    let id = AssetId { kind: AssetKind::Map, index: 5 };
    assert_eq!(id.to_string(), "map 0005");
    assert_eq!(KernelError::Stale(id).to_string(), "map 0005 changed on disk since it was loaded");
    let e = io_at(Path::new("/p/unpacked/maps/0005"))(std::io::Error::other("boom"));
    assert_eq!(e.to_string(), "/p/unpacked/maps/0005: boom");
    assert_eq!(KernelError::NotEditable(AssetKind::AreaData).to_string(), "area data cannot be edited");
    assert_eq!(KernelError::InvalidProject("header.yaml: missing gamecode".into()).to_string(),
               "invalid project metadata: header.yaml: missing gamecode");
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel error_messages`
Expected: Compile-Fehler (`IoAt`, `io_at`, `Display`).

- [ ] **Step 3: Implementieren und bestehende Fehlstellen umstellen**

`header.yaml`-, `archives.yaml`- und `header_tables.yaml`-Fehler in `project.rs` → `InvalidProject`; `Loaded::from_bytes` für AreaData → `NotEditable` (bis Task 4); `mismatch` in store.rs → `InvalidProject`; jede `std::fs::read/write/rename/create_dir_all` in project.rs, store.rs, build.rs, export.rs → `.map_err(io_at(&path))`.

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-kernel --features test-support` und `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: alle `ok`; bestehende Tests, die auf `KernelError::Build` für Metadaten prüften, sind auf `InvalidProject` angepasst.

- [ ] **Step 5: Commit**

```bash
git add crates/g4-kernel
git commit -m "refactor(kernel): typed errors with asset and path context"
```

---

### Task 3: Alle Archivarten des Welt-Editors im Projekt

**Files:**
- Modify: `crates/g4-kernel/metadata/archives.yaml`, `crates/g4-kernel/src/project.rs`, `crates/g4-kernel/src/build.rs`, `crates/g4-kernel/src/test_support.rs`, `crates/g4-kernel/tests/{project,build}.rs`

**Interfaces:**
- Produces:

```rust
pub enum AssetKind { Map, Matrix, AreaData, Event, TextArchive, MapTexture, BuildingTexture, BuildingModel, InteriorBuildingModel }
impl AssetKind { pub const ALL: [AssetKind; 9]; pub fn label(self) -> &'static str; }

impl ArchiveMap {
    pub fn packed(&self, kind: AssetKind) -> Option<&Path>;   // None: Archiv existiert in dieser Familie nicht (InteriorBuildingModel außerhalb HGSS)
    pub fn unpacked_name(&self, kind: AssetKind) -> &str;
    pub fn kinds(&self) -> impl Iterator<Item = AssetKind> + '_;   // alle mit packed()
}
impl Project {
    pub fn has_archive(&self, kind: AssetKind) -> bool;
    // unverändert: unpacked_dir, member_path, ensure_unpacked (Err(InvalidProject) für has_archive == false), member_count
}
```

`archives.yaml` (Quelle DSPRE `RomInfo.SetNarcDirs`; DP-Suffix `_release` für nicht-japanische Dumps gilt für `maps` und `events`):

```yaml
dp:
  maps:                    { packed: "files/fielddata/land_data/land_data_release.narc", packed_jp: "files/fielddata/land_data/land_data.narc", unpacked: "maps" }
  matrices:                { packed: "files/fielddata/mapmatrix/map_matrix.narc", unpacked: "matrices" }
  area_data:               { packed: "files/fielddata/areadata/area_data.narc", unpacked: "areaData" }
  events:                  { packed: "files/fielddata/eventdata/zone_event_release.narc", packed_jp: "files/fielddata/eventdata/zone_event.narc", unpacked: "eventFiles" }
  text_archives:           { packed: "files/msgdata/msg.narc", unpacked: "textArchives" }
  map_textures:            { packed: "files/fielddata/areadata/area_map_tex/map_tex_set.narc", unpacked: "mapTextures" }
  building_textures:       { packed: "files/fielddata/areadata/area_build_model/areabm_texset.narc", unpacked: "buildingTextures" }
  building_models:         { packed: "files/fielddata/build_model/build_model.narc", unpacked: "exteriorBuildingModels" }
pt:
  maps:                    { packed: "files/fielddata/land_data/land_data.narc", unpacked: "maps" }
  matrices:                { packed: "files/fielddata/mapmatrix/map_matrix.narc", unpacked: "matrices" }
  area_data:               { packed: "files/fielddata/areadata/area_data.narc", unpacked: "areaData" }
  events:                  { packed: "files/fielddata/eventdata/zone_event.narc", unpacked: "eventFiles" }
  text_archives:           { packed: "files/msgdata/pl_msg.narc", unpacked: "textArchives" }
  map_textures:            { packed: "files/fielddata/areadata/area_map_tex/map_tex_set.narc", unpacked: "mapTextures" }
  building_textures:       { packed: "files/fielddata/areadata/area_build_model/areabm_texset.narc", unpacked: "buildingTextures" }
  building_models:         { packed: "files/fielddata/build_model/build_model.narc", unpacked: "exteriorBuildingModels" }
hgss:
  maps:                    { packed: "files/a/0/6/5", unpacked: "maps" }
  matrices:                { packed: "files/a/0/4/1", unpacked: "matrices" }
  area_data:               { packed: "files/a/0/4/2", unpacked: "areaData" }
  events:                  { packed: "files/a/0/3/2", unpacked: "eventFiles" }
  text_archives:           { packed: "files/a/0/2/7", unpacked: "textArchives" }
  map_textures:            { packed: "files/a/0/4/4", unpacked: "mapTextures" }
  building_textures:       { packed: "files/a/0/7/0", unpacked: "buildingTextures" }
  building_models:         { packed: "files/a/0/4/0", unpacked: "exteriorBuildingModels" }
  interior_building_models:{ packed: "files/a/1/4/8", unpacked: "interiorBuildingModels" }
```

`build()` packt jede Art aus `archives.kinds()`, deren `unpacked_dir` existiert (Reihenfolge `AssetKind::ALL`), weiterhin erst alles packen, dann nur geänderte schreiben. `BuildReport.packed` listet alle gepackten Arten mit `unpacked_name`.

`synthetic_project()` legt zusätzlich leere, gültige NARCs für `events` (zwei leere Event-Dateien: je 16 Nullbytes = vier Zähler 0), `text_archives` (ein Member mit beliebigen 8 Bytes), `map_textures`, `building_textures`, `building_models` (je ein Member `b"BTX0"`/`b"BMD0"` plus 4 Bytes) an.

- [ ] **Step 1: Failing Tests schreiben**

`tests/project.rs`:

```rust
#[test]
fn archive_map_knows_every_world_kind() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();   // CPUE
    assert_eq!(p.archives.packed(AssetKind::Event), Some(Path::new("files/fielddata/eventdata/zone_event.narc")));
    assert_eq!(p.archives.unpacked_name(AssetKind::Event), "eventFiles");
    assert_eq!(p.archives.packed(AssetKind::TextArchive), Some(Path::new("files/msgdata/pl_msg.narc")));
    assert!(!p.has_archive(AssetKind::InteriorBuildingModel));
    assert!(matches!(p.ensure_unpacked(AssetKind::InteriorBuildingModel).unwrap_err(), KernelError::InvalidProject(_)));
    assert_eq!(p.ensure_unpacked(AssetKind::Event).unwrap(), 2);
    assert_eq!(p.archives.kinds().count(), 8);
}

#[test]
fn vanilla_every_archive_path_exists() {
    let Some(root) = common::vanilla_project() else { return };
    let p = Project::open(&root).unwrap();
    for kind in p.archives.kinds() {
        let packed = root.join(p.archives.packed(kind).unwrap());
        assert!(packed.exists(), "{kind:?}: {}", packed.display());
        assert!(p.unpacked_dir(kind).is_dir(), "{kind:?} not unpacked by DSPRE layout");
    }
    assert_eq!(p.has_archive(AssetKind::InteriorBuildingModel), p.family == Family::Hgss);
}
```

`tests/build.rs`:

```rust
#[test]
fn build_packs_every_unpacked_kind() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    for k in [AssetKind::Map, AssetKind::Event, AssetKind::MapTexture] { p.ensure_unpacked(k).unwrap(); }
    let err = build(&p, &s.dir.path().join("o.nds")).unwrap_err();   // ds-rom fails on the synthetic project
    assert!(matches!(err, KernelError::Build(_)));
    // packed only what was unpacked; everything byte-identical to the original NARCs
    for k in [AssetKind::Map, AssetKind::Event, AssetKind::MapTexture] {
        let packed = s.dir.path().join(p.archives.packed(k).unwrap());
        assert_eq!(g4_formats::narc::Narc::read(&std::fs::read(packed).unwrap()).unwrap().members.len(), p.member_count(k).unwrap());
    }
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel --features test-support --test project --test build`
Expected: Compile-Fehler (`AssetKind::Event`).

- [ ] **Step 3: Implementieren** (`ArchiveMap` intern als `HashMap<AssetKind, Entry>`; YAML-Schlüssel wie oben, `interior_building_models` optional)

- [ ] **Step 4: Tests laufen lassen**

Run: Kernel-Tests, `cargo test --workspace --all-features`, gated auf PT_DE, HG_DE, HG_US; clippy.
Expected: alle `ok`. Der Vanilla-Build-Test (`vanilla_build_produces_rom`) bleibt byte-identisch zur dsrom-CLI (unveränderte Archive werden nicht geschrieben).

- [ ] **Step 5: Commit**

```bash
git add crates/g4-kernel
git commit -m "feat(kernel): archive map for events, text, textures and building models"
```

---

### Task 4: Store, der selbst entpackt, mit Result-Closures, Events, Area-Data und Stale-Auflösung

**Files:**
- Modify: `crates/g4-kernel/src/store.rs`, `crates/g4-kernel/src/bin/g4k.rs`, `apps/studio/src-tauri/src/commands.rs` (nur Aufrufer von `edit_map`), `crates/g4-kernel/tests/store.rs`

**Interfaces:**
- Consumes: `uxie::event_file::EventFile` (`from_binary(&mut Cursor)`, `to_binary_for(&mut Cursor, uxie::GameFamily)`), `g4_formats::area_data::AreaData`.
- Produces:

```rust
pub use uxie::event_file::{EventFile, BgEvent, ObjectEvent, WarpEvent, CoordEvent};

impl Store {
    pub fn map(&mut self, index: u16) -> Result<&MapFile, KernelError>;            // ruft project.ensure_unpacked(kind) beim ersten Zugriff je Kind
    pub fn matrix(&mut self, index: u16) -> Result<&Matrix, KernelError>;
    pub fn area_data(&mut self, index: u16) -> Result<&AreaData, KernelError>;
    pub fn event(&mut self, index: u16) -> Result<&EventFile, KernelError>;
    pub fn edit_map(&mut self, index: u16, label: &str, edit: impl FnOnce(&mut MapFile) -> Result<(), FormatError>) -> Result<ChangeId, KernelError>;
    pub fn edit_matrix(.., edit: impl FnOnce(&mut Matrix) -> Result<(), FormatError>) -> Result<ChangeId, KernelError>;
    pub fn edit_area_data(.., edit: impl FnOnce(&mut AreaData) -> Result<(), FormatError>) -> Result<ChangeId, KernelError>;
    pub fn edit_event(.., edit: impl FnOnce(&mut EventFile) -> Result<(), FormatError>) -> Result<ChangeId, KernelError>;
    pub fn reload_from_disk(&mut self, id: AssetId) -> Result<(), KernelError>;   // verwirft eigene Änderungen am Asset, lädt Platte, entfernt dessen Changesets aus Undo und Redo
    pub fn accept_disk_as_base(&mut self, id: AssetId) -> Result<(), KernelError>;  // saved_bytes := Platte; eigene Änderungen bleiben; nächstes save überschreibt
    // unverändert: undo, redo, is_dirty, dirty, save
}
```

Regeln: Ein Fehler aus der Closure oder aus `validate()` (Map, Matrix) bzw. aus `to_binary_for` (Event: Warp-Höhe ≠ 0 außerhalb HGSS) rollt zurück und wird als `KernelError::Format` bzw. `KernelError::Uxie` zurückgegeben. `Loaded::to_bytes` wird `Result<Vec<u8>, KernelError>`; `is_dirty` meldet bei einem Serialisierungsfehler `true`, `save` gibt ihn zurück. Lesende Arten (`TextArchive`, Texturen, Modelle) sind nicht im Store; `Loaded::from_bytes` liefert dafür `NotEditable(kind)`.

- [ ] **Step 1: Failing Tests schreiben**

```rust
#[test]
fn store_unpacks_on_first_access() {
    let s = common::synthetic_project();
    let mut st = Store::new(Arc::new(Project::open(s.dir.path()).unwrap()));
    assert!(!s.dir.path().join("unpacked/eventFiles").exists());
    assert_eq!(st.event(0).unwrap().warp_events.len(), 0);
    assert!(s.dir.path().join("unpacked/eventFiles/0000").exists());
}

#[test]
fn failing_edit_closure_is_rolled_back() {
    let (_s, mut st) = store();
    let err = st.edit_map(1, "x", |m| m.set_cell(99, 0, Cell::default())).unwrap_err();
    assert!(matches!(err, KernelError::Format(FormatError::OutOfRange { .. })));
    assert!(!st.is_dirty(AssetId { kind: AssetKind::Map, index: 1 }));
    assert!(st.undo().is_none());
}

#[test]
fn event_edit_round_trips_and_validates_warp_height() {
    let (s, mut st) = store();                      // synthetic project is Platinum
    st.edit_event(0, "warp", |e| { e.warp_events.push(WarpEvent { x: 8, z: 4, dest_header_id: 414, dest_warp_id: 0, height: 0 }); Ok(()) }).unwrap();
    st.save().unwrap();
    let bytes = std::fs::read(s.dir.path().join("unpacked/eventFiles/0000")).unwrap();
    assert_eq!(&bytes[8..12], &1u32.to_le_bytes());           // warp count after bg and object counts
    let err = st.edit_event(0, "bad", |e| { e.warp_events[0].height = 1; Ok(()) }).unwrap_err();
    assert!(matches!(err, KernelError::Uxie(_)));
    assert_eq!(st.event(0).unwrap().warp_events[0].height, 0);
}

#[test]
fn area_data_edit_round_trips() {
    let (s, mut st) = store();
    st.edit_area_data(1, "tileset", |a| { if let AreaData::Dppt { map_tileset, .. } = a { *map_tileset = 7 } Ok(()) }).unwrap();
    st.save().unwrap();
    assert_eq!(std::fs::read(s.dir.path().join("unpacked/areaData/0001")).unwrap()[2..4], 7u16.to_le_bytes());
}

#[test]
fn stale_recovery_reload_and_accept() {
    let (s, mut st) = store();
    let id = AssetId { kind: AssetKind::Map, index: 0 };
    st.edit_map(0, "mine", |m| m.set_cell(1, 1, Cell { kind: 1, collision: 0 })).unwrap();
    let path = s.dir.path().join("unpacked/maps/0000");
    let mut theirs = std::fs::read(&path).unwrap(); theirs[16 + 2 * (5 * 32 + 5) + 1] = 0x80; std::fs::write(&path, &theirs).unwrap();
    assert!(matches!(st.save().unwrap_err(), KernelError::Stale(i) if i == id));
    st.reload_from_disk(id).unwrap();
    assert!(!st.is_dirty(id));
    assert_eq!(st.map(0).unwrap().cell(5, 5).unwrap().collision, 0x80);   // theirs
    assert_eq!(st.map(0).unwrap().cell(1, 1).unwrap().kind, 0);           // mine gone
    assert!(st.undo().is_none());
    // second round: keep mine
    st.edit_map(0, "mine2", |m| m.set_cell(2, 2, Cell { kind: 2, collision: 0 })).unwrap();
    std::fs::write(&path, &theirs[..]).unwrap(); // unchanged content, but touch a byte to make it stale again
    let mut t2 = theirs.clone(); t2[16 + 2 * (6 * 32 + 6) + 1] = 0x80; std::fs::write(&path, &t2).unwrap();
    assert!(st.save().is_err());
    st.accept_disk_as_base(id).unwrap();
    assert!(st.is_dirty(id));
    st.save().unwrap();
    let on_disk = MapFile::read(&std::fs::read(&path).unwrap(), Family::Pt).unwrap();
    assert_eq!(on_disk.cell(2, 2).unwrap().kind, 2);
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel --features test-support --test store`
Expected: Compile-Fehler.

- [ ] **Step 3: Implementieren; Aufrufer anpassen** (`g4k map set-perm` und `set_permission_impl` geben `m.set_cell(..)` direkt aus der Closure zurück; die bisherigen „failure capture“-Konstrukte entfallen)

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test --workspace --all-features`, gated Store-Tests auf PT_DE, HG_DE, HG_US (Kopien), clippy
Expected: alle `ok`.

- [ ] **Step 5: Commit**

```bash
git add crates/g4-kernel apps/studio/src-tauri
git commit -m "feat(kernel): store unpacks on demand, fallible edits, events and area data, stale recovery"
```

---

### Task 5: Strukturierte Command-Fehler und Stale-Auflösung in der App

**Files:**
- Create: `apps/studio/src-tauri/src/errors.rs`
- Modify: `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src-tauri/src/lib.rs`, `apps/studio/src/api.ts`, `apps/studio/src/Toolbar.tsx` (StatusBar), `apps/studio/src/App.tsx`

**Interfaces:**
- Produces (Rust):

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct CmdError { pub code: &'static str, pub message: String, pub asset: Option<AssetId> }   // code: "stale" | "tool_missing" | "error"
impl From<KernelError> for CmdError;    // Stale → code "stale" + asset; ToolMissing → "tool_missing"; sonst "error"; message = to_string()
impl From<String> for CmdError;         // code "error"
```

Alle `*_impl` geben ab jetzt `Result<T, CmdError>` zurück (die Wrapper unverändert dünn). Neue Commands: `reload_asset(kind: AssetKind, index: u16) -> Result<(), CmdError>` und `accept_disk(kind, index) -> Result<(), CmdError>`.

- Produces (TS, `api.ts`): `interface CmdError { code: "stale" | "tool_missing" | "error"; message: string; asset: AssetId | null }`, `isCmdError(e: unknown): e is CmdError`, `reloadAsset(asset)`, `acceptDisk(asset)`.
- StatusBar: `Message` bekommt optional `actions: { label: string; run: () => void }[]`. Bei `code === "stale"` zeigt App.tsx: „{asset} wurde außerhalb geändert.“ mit den Aktionen „Neu laden (eigene Änderung verwerfen)“ → `reloadAsset` und „Meine Version behalten“ → `acceptDisk`, danach jeweils Matrix neu laden und `dirty` aktualisieren.

- [ ] **Step 1: Failing Tests schreiben**

Rust (`commands.rs` Modultests, synthetisches Projekt über `test_support`):

```rust
#[test]
fn save_stale_returns_structured_error() {
    let (s, state) = open_synthetic();      // Helfer: open_project_impl auf synthetic_project()
    set_permission_impl(&state, 0, 1, 1, 0, 0x80).unwrap();
    let path = s.dir.path().join("unpacked/maps/0000");
    let mut b = std::fs::read(&path).unwrap(); b[20] ^= 0xFF; std::fs::write(&path, &b).unwrap();
    let err = save_impl(&state).unwrap_err();
    assert_eq!(err.code, "stale");
    assert_eq!(err.asset, Some(AssetId { kind: AssetKind::Map, index: 0 }));
    assert!(err.message.starts_with("map 0000 changed on disk"));
    reload_asset_impl(&state, AssetKind::Map, 0).unwrap();
    assert!(dirty_impl(&state).unwrap().is_empty());
}
```

vitest (`src/api.test.ts`):

```ts
it("recognises structured command errors", () => {
  expect(isCmdError({ code: "stale", message: "m", asset: { kind: "Map", index: 0 } })).toBe(true);
  expect(isCmdError("plain string")).toBe(false);
});
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen** — Compile-Fehler bzw. `isCmdError` fehlt.

- [ ] **Step 3: Implementieren** (StatusBar-Aktionen als Buttons rechts neben dem Text; keine modalen Dialoge)

- [ ] **Step 4: Tests laufen lassen** — `cargo test -p studio`, `pnpm test`, `pnpm build`, clippy.

- [ ] **Step 5: Commit**

```bash
git add apps/studio
git commit -m "feat(studio): structured command errors with stale recovery actions"
```

---

### Task 6: Exit-Guard für Cmd+Q

**Files:**
- Modify: `apps/studio/src-tauri/src/lib.rs`, `apps/studio/src-tauri/src/state.rs`, `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src-tauri/capabilities/default.json`, `apps/studio/src/App.tsx`

**Interfaces:**
- Produces (Rust): `AppState` erhält `pub force_quit: AtomicBool`. `pub fn exit_is_blocked(dirty: usize, force: bool) -> bool` in `state.rs` (true genau dann, wenn `dirty > 0 && !force`). In `lib.rs`: `.build(..)?.run(|app, event| if let RunEvent::ExitRequested { api, .. } = event { … })`: liest `dirty().len()` aus dem Store (Mutex kurz), ruft `api.prevent_exit()` wenn blockiert und sendet `app.emit("confirm-quit", ())`. Command `force_quit(app: AppHandle)`: setzt `force_quit = true`, ruft `app.exit(0)`.
- Frontend: `listen("confirm-quit")` in App.tsx fragt per `ask("Ungespeicherte Änderungen verwerfen und beenden?")`; bei Ja `invoke("force_quit")`. Der bestehende `onCloseRequested`-Guard bleibt.
- Capability: zusätzlich `core:event:default` (für `listen`), falls `core:default` es nicht schon enthält (prüfen; nichts Überflüssiges hinzufügen).

- [ ] **Step 1: Failing Test schreiben**

```rust
#[test]
fn exit_is_blocked_only_with_dirty_assets_and_without_force() {
    assert!(exit_is_blocked(1, false));
    assert!(!exit_is_blocked(0, false));
    assert!(!exit_is_blocked(3, true));
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag prüfen** — Compile-Fehler.
- [ ] **Step 3: Implementieren.**
- [ ] **Step 4: Verifizieren** — `cargo test -p studio`, `pnpm build`, `pnpm tauri build --debug --no-bundle`. Manuell (Gate): Kachel ändern, Cmd+Q → Rückfrage; „Abbrechen“ lässt die App offen; ohne Änderungen beendet Cmd+Q sofort.
- [ ] **Step 5: Commit** — `fix(studio): ask before quitting with unsaved edits (Cmd+Q)`

---

### Task 7: Modell-Cache nach Inhalts-Hash mit Texturen

**Files:**
- Modify: `crates/g4-kernel/src/export.rs`, `crates/g4-kernel/Cargo.toml` (`xxhash-rust = { version = "0.8", features = ["xxh3"] }`), `crates/g4-kernel/tests/export.rs`, `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/world/MatrixScene.tsx`, `docs/dev-setup.md`

**Interfaces:**
- Produces (Kernel):

```rust
/// Cache-Schlüssel: xxh3_64 über nsbmd ++ nsbtx (leer, wenn None) als 16 Hex-Zeichen; Datei `<cache_dir>/<hash>.glb`.
pub fn glb_cache_key(nsbmd: &[u8], nsbtx: Option<&[u8]>) -> String;
/// Liefert GLB-Bytes aus dem Cache oder konvertiert mit apicula (NSBMD plus optionales NSBTX im selben Aufruf) und legt sie ab (temp + rename).
pub fn glb_cached(cache_dir: &Path, apicula: &Path, nsbmd: &[u8], nsbtx: Option<&[u8]>) -> Result<Vec<u8>, KernelError>;
/// Tileset einer Karte: AreaData[area_data_id].map_tileset → Member von MapTexture; Ok(None), wenn Index außerhalb des Archivs.
impl Project { pub fn map_tileset_bytes(&self, area_data_id: u8) -> Result<Option<Vec<u8>>, KernelError>; }
```

Der Cache liegt weiter unter `<project>/.g4/models/`. Das mtime-basierte `is_fresh` entfällt. `nsbmd_to_glb` bekommt einen Parameter `nsbtx: Option<&[u8]>` (zweite Temp-Datei `.nsbtx`, als zweites Argument an apicula).

- Produces (Commands): `get_map_model_glb(index: u16, area_data_id: u8) -> Response` (Bytes); DTO-freie Änderung in `api.ts`: `getMapModelGlb(index, areaDataId)`. `MatrixScene` erhält die Prop `areaDataId: number` (aus dem gewählten Header) und reicht sie an `loadMap` durch.

- [ ] **Step 1: Failing Tests schreiben**

`tests/export.rs`:

```rust
#[test]
fn cache_key_depends_on_model_and_tileset() {
    assert_eq!(glb_cache_key(b"BMD0", None).len(), 16);
    assert_ne!(glb_cache_key(b"BMD0", None), glb_cache_key(b"BMD0", Some(b"BTX0")));
}

#[test]
fn glb_cached_hits_the_cache_second_time() {   // mit dem Fake-apicula-Skript aus Phase 1, das eine Zähldatei hochzählt
    let (apicula, counter) = fake_apicula_counting();
    let dir = tempfile::tempdir().unwrap();
    let a = glb_cached(dir.path(), &apicula, b"BMD0one", None).unwrap();
    let b = glb_cached(dir.path(), &apicula, b"BMD0one", None).unwrap();
    assert_eq!(a, b);
    assert_eq!(std::fs::read_to_string(counter).unwrap().trim(), "1");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);   // keine Temp-Reste
}

#[test]
fn vanilla_textured_map_glb_has_images() {       // gated: G4_TEST_PROJECT + G4_APICULA; nur lesen, Cache im tempdir
    …; let p = Project::open(&root).unwrap();
    let headers = p.headers().unwrap(); let h = &headers[415.min(headers.len() - 1)];
    let map_index = Matrix::read(&std::fs::read(p.member_path(AssetKind::Matrix, h.matrix_id)).unwrap()).unwrap().maps[0];
    let map = MapFile::read(&std::fs::read(p.member_path(AssetKind::Map, map_index)).unwrap(), p.family).unwrap();
    let tex = p.map_tileset_bytes(h.area_data_id).unwrap();
    assert!(tex.is_some());
    let glb = glb_cached(tmp.path(), &apicula, &map.model, tex.as_deref()).unwrap();
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    assert!(json["images"].as_array().map_or(0, |a| a.len()) > 0);
}

#[test]
fn glb_cached_without_tileset_still_converts() { … glb_cached(.., None) auf demselben Vanilla-Modell → Ok, images == 0 }
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen** — Compile-Fehler.
- [ ] **Step 3: Implementieren; `get_map_model_glb_with/_impl` auf `glb_cached` umstellen (Projekt-Arc, NSBMD und Tileset unter einem Lock holen, Konvertierung außerhalb); `docs/dev-setup.md` Abschnitt „Cache“ auf Hash-Schlüssel ändern.**
- [ ] **Step 4: Tests laufen lassen** — Kernel-Tests, Studio-Tests, gated auf PT_DE, HG_DE, HG_US mit `G4_APICULA`; `pnpm build`.
- [ ] **Step 5: Commit** — `feat(kernel): content-hashed GLB cache with map tilesets`

---

### Task 8: Dockview-Workbench mit Inspector

**Files:**
- Create: `apps/studio/src/workbench/Workbench.tsx`, `apps/studio/src/workbench/layout.ts`, `apps/studio/src/workbench/layout.test.ts`, `apps/studio/src/workbench/Inspector.tsx`, `apps/studio/src/workbench/HeaderList.tsx`
- Modify: `apps/studio/package.json` (`dockview@8.6.1`), `apps/studio/src/App.tsx`, `apps/studio/src/main.tsx` (Dockview-CSS-Import)

**Interfaces:**
- `Workbench` rendert `DockviewReact` mit den Panels `headers` (HeaderList), `world` (MatrixScene-Host, mittig), `inspector` (rechts), `status` (unten, StatusBar). Standardlayout: `headers` links 280 px, `inspector` rechts 320 px, `status` unten 40 px.
- `layout.ts`: `export const LAYOUT_KEY = "g4.layout.v1"`, `saveLayout(json: object): void`, `loadLayout(): object | null` (localStorage, try/catch, ungültiges JSON → null), `DEFAULT_PANELS: readonly PanelSpec[]` mit `PanelSpec = { id: "headers" | "world" | "inspector" | "status"; title: string; position: { direction: "left" | "right" | "below" | "within"; referencePanel?: string }; size?: number }`.
- `Inspector` zeigt einen `InspectorModel` an: `{ kind: "none" } | { kind: "tile"; … } | { kind: "event"; … }` (Varianten werden in Task 9 und 10 gefüllt; hier nur `none` mit Hinweistext). App.tsx hält `const [inspector, setInspector] = useState<InspectorModel>({ kind: "none" })` und reicht `setInspector` an MatrixScene.
- CSP bleibt; Dockview-CSS kommt aus dem Paket (`dockview/dist/styles/dockview.css`).

- [ ] **Step 1: Failing Test schreiben** (`layout.test.ts`)

```ts
it("round-trips a layout through localStorage and tolerates garbage", () => {
  saveLayout({ a: 1 });
  expect(loadLayout()).toEqual({ a: 1 });
  localStorage.setItem(LAYOUT_KEY, "{not json");
  expect(loadLayout()).toBeNull();
});
it("default layout has the four panels once", () => {
  expect(DEFAULT_PANELS.map(p => p.id).sort()).toEqual(["headers", "inspector", "status", "world"]);
});
```

(vitest mit `environment: "jsdom"` für diesen Test; `jsdom` als Dev-Dependency.)

- [ ] **Step 2: Test laufen lassen, Fehlschlag prüfen.**
- [ ] **Step 3: Implementieren** (Toolbar bleibt oberhalb der Workbench; `onDidLayoutChange` → `saveLayout(api.toJSON())`, beim Start `api.fromJSON(loadLayout())` mit Fallback auf `DEFAULT_PANELS`, wenn das gespeicherte Layout nicht alle vier Panels enthält).
- [ ] **Step 4: Verifizieren** — `pnpm test`, `pnpm build`, `pnpm tauri build --debug --no-bundle`; manuell: Panels verschieben, App neu starten, Layout bleibt.
- [ ] **Step 5: Commit** — `feat(studio): dockview workbench with header list, world, inspector and status panels`

---

### Task 9: Kachel-Hover mit Koordinate und Typ

**Files:**
- Create: `crates/g4-kernel/metadata/tile_behaviors.yaml`, `crates/g4-kernel/src/tiles.rs`, `apps/studio/src/world/hover.ts`, `apps/studio/src/world/hover.test.ts`
- Modify: `crates/g4-kernel/src/lib.rs`, `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/world/MatrixScene.tsx`, `apps/studio/src/workbench/Inspector.tsx`

**Interfaces:**
- `tile_behaviors.yaml`: Liste `- { value: 0x5E, name: WARP_STAIRS_EAST, class: warp }` für alle benannten Werte des Decomp-Enums `TileBehavior` (`include/constants/field/map_tile_behaviors.h` in pret/pokeplatinum; Wert = Position im Enum, `UNUSED_*`/`UNKNOWN_*` weglassen). `class` ∈ `grass | water | ice | sand | ledge | block | slide | warp | door | bridge | furniture | other`: `TALL_GRASS`, `VERY_TALL_GRASS` → grass; `WATER_*`, `WATERFALL`, `PUDDLE*`, `SHALLOW_WATER` → water; `JUMP_*` → ledge; `BLOCK_*` → block; `SLIDE_*` → slide; `WARP_*`, `ESCALATOR*` → warp; `DOOR` → door; `*BRIDGE*` → bridge; `TABLE`, `PC`, `TOWN_MAP`, `TV` → furniture; sonst other.
- Kernel: `pub struct TileBehavior { pub value: u8, pub name: String, pub class: String }`, `pub fn tile_behaviors() -> Result<Vec<TileBehavior>, KernelError>` (eingebettet, `InvalidProject` bei YAML-Fehler), `pub fn tile_behavior(value: u8) -> Option<TileBehavior>`.
- Command `tile_behaviors() -> Vec<TileBehavior>`; `api.ts`: `TileBehavior`, `tileBehaviors()`.
- `hover.ts`: `describeTile(cell: [number, number], names: Map<number, TileBehavior>): TileInfo` mit `TileInfo = { kind: number; collision: number; blocked: boolean; name: string | null; class: string | null }`; `InspectorModel` Variante `{ kind: "tile"; map: number; x: number; y: number; absX: number; absZ: number; info: TileInfo }`.
- MatrixScene: `pointermove` (gedrosselt auf einen Animation-Frame) → `pickCell` → `setInspector({ kind: "tile", … })`; außerhalb jeder Zelle → `{ kind: "none" }`. Overlay-Farben nutzen jetzt `class` statt der festen Bereiche aus `colors.ts` (`permissionColor(kind, collision, class)`; Signatur erweitert, Tests angepasst: 0x02 grass → `#4caf50`, 0x10 water → `#3b82f6`, warp → `#f59e0b`, Kollision weiter `#d64545`).

- [ ] **Step 1: Failing Tests schreiben**

Rust:

```rust
#[test]
fn tile_behaviors_cover_the_known_values() {
    let t = tile_behaviors().unwrap();
    let by = |v: u8| t.iter().find(|b| b.value == v).map(|b| (b.name.as_str(), b.class.as_str()));
    assert_eq!(by(0x02), Some(("TALL_GRASS", "grass")));
    assert_eq!(by(0x5E), Some(("WARP_STAIRS_EAST", "warp")));
    assert_eq!(by(0x69), Some(("DOOR", "door")));
    assert_eq!(by(0x86), Some(("TV", "furniture")));
    assert_eq!(by(0x01), None);
    assert!(t.iter().all(|b| !b.name.starts_with("UNUSED")));
}
```

vitest (`hover.test.ts`):

```ts
it("describes a stairs tile and an unknown tile", () => {
  const names = new Map([[0x5e, { value: 0x5e, name: "WARP_STAIRS_EAST", class: "warp" }]]);
  expect(describeTile([0x5e, 0x80], names)).toEqual({ kind: 0x5e, collision: 0x80, blocked: true, name: "WARP_STAIRS_EAST", class: "warp" });
  expect(describeTile([0x33, 0], names)).toEqual({ kind: 0x33, collision: 0, blocked: false, name: null, class: null });
});
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen.**
- [ ] **Step 3: Implementieren.** Inspector-Darstellung für `tile`: „Karte 0187 · Kachel x 8, y 4 · Typ 0x5E WARP_STAIRS_EAST · blockiert“.
- [ ] **Step 4: Verifizieren** — Rust- und vitest-Tests, `pnpm build`; manuell: über die Treppe im Spielerzimmer (Header 0415 in Platin) fahren → „0x5E WARP_STAIRS_EAST“.
- [ ] **Step 5: Commit** — `feat(studio): tile hover with coordinates and behaviour names`

---

### Task 10: Events lesen und als Marker zeigen

**Files:**
- Create: `apps/studio/src/world/events.ts`, `apps/studio/src/world/events.test.ts`
- Modify: `apps/studio/src-tauri/src/commands.rs`, `apps/studio/src/api.ts`, `apps/studio/src/world/MatrixScene.tsx`, `apps/studio/src/workbench/Inspector.tsx`, `crates/g4-kernel/src/project.rs` (`HeaderSummary.event_file_id`)

**Interfaces:**
- `HeaderSummary` erhält `pub event_file_id: u16` (aus `MapHeader::event_file_id()`).
- Commands: `get_events(event_file_id: u16) -> EventsDto` mit

```rust
#[derive(serde::Serialize)] pub struct EventsDto { pub event_file_id: u16, pub bg: Vec<BgDto>, pub objects: Vec<ObjectDto>, pub warps: Vec<WarpDto>, pub coords: Vec<CoordDto> }
pub struct BgDto { pub index: u16, pub script: u16, pub event_type: u16, pub x: i32, pub z: i32, pub y: i32 }
pub struct ObjectDto { pub index: u16, pub local_id: u16, pub graphics_id: u16, pub movement_type: u16, pub script: u16, pub x: u16, pub z: u16, pub y: i32 }
pub struct WarpDto { pub index: u16, pub x: u16, pub z: u16, pub dest_header_id: u16, pub dest_warp_id: u16, pub height: u32 }
pub struct CoordDto { pub index: u16, pub script: u16, pub x: i16, pub z: i16, pub width: u16, pub length: u16 }
```

Koordinaten sind absolute Kacheln über die ganze Matrix (Zelle = `x / 32`, lokal = `x % 32`); das ist die Konvention der Spieldaten (Decomp `events_*.json`, DSPRE „matrix position“).

- `events.ts`: `markerPosition(x: number, z: number, matrix: MatrixDto): { wx: number; wz: number } | null` (null, wenn `x < 0 || z < 0 || x >= matrix.width * 32 || z >= matrix.height * 32`), `MARKER_COLORS = { warp: "#2563eb", object: "#eab308", coord: "#22c55e", bg: "#a855f7" }`.
- MatrixScene: Marker als `InstancedMesh` von `BoxGeometry(0.8, 0.4, 0.8)` auf Höhe 0.3 je Eventklasse (vier Meshes), Warps zusätzlich mit `BoxGeometry(0.8, 1.2, 0.8)`; Umschalter „Events anzeigen“ neben „Permissions anzeigen“. Hover über einen Marker (Raycast gegen die Marker-Meshes hat Vorrang vor der Kachel) → `InspectorModel` Variante `{ kind: "event"; eventKind: "warp" | "object" | "coord" | "bg"; index: number; data: WarpDto | ObjectDto | CoordDto | BgDto; destName?: string }` (für Warps `destName` = `location_name` des Zielheaders aus der Header-Liste, sobald Task 11 sie liefert; vorher `internal_name`).

- [ ] **Step 1: Failing Tests schreiben**

Rust (Studio-Modultests, gated Vanilla-Test auf PT_DE):

```rust
#[test]
fn vanilla_platinum_player_bedroom_has_the_stairs_warp() {
    let Some(root) = g4_kernel::test_support::vanilla_project() else { return };
    let state = AppState::default();
    let info = open_project_impl(&state, root.to_str().unwrap()).unwrap();
    if info.game_code != "CPUD" { eprintln!("skipped: not Platinum DE"); return }
    let headers = list_headers_impl(&state).unwrap();
    let ev = get_events_impl(&state, headers[415].event_file_id).unwrap();
    assert_eq!(ev.warps.len(), 1);
    assert_eq!((ev.warps[0].x, ev.warps[0].z, ev.warps[0].dest_header_id), (8, 4, 414));
    assert_eq!(ev.objects.len(), 1);          // Barry
    assert_eq!(ev.coords.len(), 4);
}
```

vitest (`events.test.ts`):

```ts
const m = { index: 0, name: "m", width: 2, height: 1, maps: [1, 2] };
it("places markers in world space and rejects coordinates outside the matrix", () => {
  expect(markerPosition(8, 4, m)).toEqual({ wx: 8.5, wz: 4.5 });
  expect(markerPosition(40, 4, m)).toEqual({ wx: 40.5, wz: 4.5 });
  expect(markerPosition(64, 4, m)).toBeNull();
  expect(markerPosition(8, 32, m)).toBeNull();
  expect(markerPosition(-1, 0, m)).toBeNull();
});
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen.**
- [ ] **Step 3: Implementieren.**
- [ ] **Step 4: Verifizieren** — Tests, `pnpm build`; manuell im Spielerzimmer: blauer Marker auf der Treppe, Hover zeigt „Warp 0 → Header 0414 (T01R0201)“.
- [ ] **Step 5: Commit** — `feat(studio): event markers with warp, object, trigger and background events`

---

### Task 11: Lesbare Header-Namen und Listenlayout

**Files:**
- Modify: `crates/g4-kernel/src/project.rs`, `crates/g4-kernel/tests/project.rs`, `crates/g4-kernel/src/bin/g4k.rs`, `apps/studio/src/workbench/HeaderList.tsx`, `apps/studio/src/api.ts`

**Interfaces:**
- Kernel:

```rust
impl Project {
    /// Ortsnamen aus dem Textarchiv `TextBankIds::for_game(family, language).locations` (uxie), dekodiert mit uxie::text_bank::read_text_archive_bin.
    pub fn location_names(&self) -> Result<Vec<String>, KernelError>;
}
pub struct HeaderSummary { pub id: u16, pub internal_name: String, pub location_name: String, pub matrix_id: u16, pub area_data_id: u8, pub event_file_id: u16 }
```

`location_name` = `names[header.location_name]` (DP: `u16`, Pt/HGSS: `u8`; außerhalb der Liste → leer). `uxie::GameLanguage` wird aus unserer `Language` gemappt. `headers()` ruft `location_names()` einmal und entpackt dafür `TextArchive` bei Bedarf.

- `g4k info` druckt zusätzlich pro Projekt nichts Neues; `g4k headers <project>` (neu) druckt `id  location_name  internal_name  matrix` je Zeile.
- HeaderList: CSS-Grid mit vier Spalten (`id` monospace 48 px, Ortsname, Kürzel grau, `M<matrix>` rechtsbündig), `white-space: nowrap; overflow: hidden; text-overflow: ellipsis`; Suchfeld oben filtert nach Ortsname und Kürzel (case-insensitive).

- [ ] **Step 1: Failing Tests schreiben**

```rust
#[test]
fn vanilla_headers_carry_location_names() {
    let Some(root) = common::vanilla_project() else { return };
    let p = Project::open(&root).unwrap();
    let names = p.location_names().unwrap();
    assert!(names.len() > 100);
    let headers = p.headers().unwrap();
    let t01 = headers.iter().find(|h| h.internal_name == "T01").unwrap();
    let expected = match p.game_code.as_str() { "IPKE" => "new bark town", "IPKD" => "neuborkia", "CPUD" => "zweiblattdorf", _ => return };
    assert_eq!(t01.location_name.to_lowercase(), expected);
    assert!(headers.iter().all(|h| h.event_file_id < p.member_count(AssetKind::Event).unwrap() as u16 || h.event_file_id == u16::MAX));
}
```

(Erwartete Namen: HG US „New Bark Town“, HG DE „Neuborkia“, Platin DE „Zweiblattdorf“. Weicht die Dekodierung bei Umlauten ab, ist das ein Befund für chatots Charmap und wird im Report festgehalten; der Test prüft dann nur Nicht-Leere und die US-Variante.)

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen.**
- [ ] **Step 3: Implementieren.**
- [ ] **Step 4: Verifizieren** — gated auf PT_DE, HG_DE, HG_US; `pnpm build`; manuell: Liste ohne Zeilenumbrüche, Suche nach „Zweiblatt“ findet 0411–0417.
- [ ] **Step 5: Commit** — `feat: location names for map headers and a searchable header list`

---

### Task 12: Große Matrizen laden

**Files:**
- Create: `apps/studio/src/world/load.ts`, `apps/studio/src/world/load.test.ts`
- Modify: `apps/studio/src/world/MatrixScene.tsx`, `apps/studio/src/workbench/Inspector.tsx` oder StatusBar (Fortschritt)

**Interfaces:**
- `load.ts`: `limitConcurrency<T>(tasks: (() => Promise<T>)[], cap: number, onProgress?: (done: number, total: number) => void): Promise<PromiseSettledResult<T>[]>`; `MODEL_CONCURRENCY = 4`.
- MatrixScene: Permissions-Overlay und Marker erscheinen sofort nach `getMap` aller Zellen (ohne Modelle); Modelle werden danach über `limitConcurrency` nachgeladen und einzeln in die Szene gehängt; StatusBar zeigt „Modelle 37/900“ bis alle da sind; ein Wechsel des Headers bricht ausstehende Ladevorgänge ab (bestehender `cancelled`-Mechanismus).
- Abnahme: Platin-Header 0003 (C01, Matrix 0, 900 Zellen) mit leerem `.g4/models`: Overlay in unter 5 s, Modelle vollständig in unter 3 min auf diesem Mac, zweites Öffnen (Cache) unter 15 s. Die gemessenen Zeiten kommen in den Report.

- [ ] **Step 1: Failing Test schreiben**

```ts
it("limitConcurrency never exceeds the cap and reports progress", async () => {
  let running = 0, peak = 0; const seen: number[] = [];
  const tasks = Array.from({ length: 10 }, (_, i) => async () => { running++; peak = Math.max(peak, running); await new Promise(r => setTimeout(r, 5)); running--; return i; });
  const results = await limitConcurrency(tasks, 3, done => seen.push(done));
  expect(peak).toBeLessThanOrEqual(3);
  expect(results.map(r => r.status === "fulfilled" ? r.value : -1).sort((a, b) => a - b)).toEqual([0,1,2,3,4,5,6,7,8,9]);
  expect(seen.at(-1)).toBe(10);
});
it("a rejected task does not stop the others", async () => {
  const results = await limitConcurrency([async () => { throw new Error("x"); }, async () => 1], 2);
  expect(results[0].status).toBe("rejected"); expect(results[1].status).toBe("fulfilled");
});
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag prüfen.**
- [ ] **Step 3: Implementieren.**
- [ ] **Step 4: Verifizieren** — `pnpm test`, `pnpm build`; die drei Messungen durchführen und notieren.
- [ ] **Step 5: Commit** — `perf(studio): stream map models with bounded concurrency and progress`

---

### Task 13: App-Bundle mit dylib und apicula

**Files:**
- Create: `apps/studio/scripts/prepare-bundle.sh`, `docs/release.md`
- Modify: `apps/studio/src-tauri/tauri.conf.json`, `apps/studio/src-tauri/build.rs`, `apps/studio/package.json` (Script `bundle`), `.gitignore` (`apps/studio/src-tauri/binaries/`, `apps/studio/src-tauri/frameworks/`), `docs/dev-setup.md`

**Interfaces:**
- `prepare-bundle.sh`: kopiert `tools/apicula/bin/apicula` nach `src-tauri/binaries/apicula-$(rustc -vV | sed -n 's/host: //p')` und `target/release/libnitroarc_ffi.dylib` (Pfad über `cargo build --release -p studio` erzeugt; falls die Datei woanders liegt, per `find target/release -name 'libnitroarc_ffi.dylib' -maxdepth 3` suchen) nach `src-tauri/frameworks/libnitroarc_ffi.dylib`; bricht mit Fehlertext ab, wenn eine Quelle fehlt.
- `tauri.conf.json`: `bundle.externalBin: ["binaries/apicula"]`, `bundle.macOS.frameworks: ["frameworks/libnitroarc_ffi.dylib"]`, `bundle.macOS.minimumSystemVersion: "13.0"`.
- `build.rs`: zusätzlich rpath `@executable_path/../Frameworks` (macOS), damit das Bundle die dylib findet; `@executable_path` bleibt für `target/`.
- apicula-Suche (Task 10 in Phase 1): Eintrag 2 „neben `current_exe()`“ findet den Sidecar, weil Tauri `externalBin` nach `Contents/MacOS/` legt und den Triple-Suffix entfernt.
- `package.json`: `"bundle": "bash scripts/prepare-bundle.sh && tauri build"`.
- `docs/release.md` (Deutsch): Schritte, Prüfbefehle, bekannte Grenzen (nicht signiert, nicht notarisiert; Gatekeeper-Freigabe nötig).

- [ ] **Step 1: Prüfskript als Test schreiben** (`apps/studio/scripts/check-bundle.sh`, wird in Step 4 ausgeführt; schlägt fehl, solange das Bundle nicht existiert)

```bash
#!/usr/bin/env bash
set -euo pipefail
APP=$(ls -d target/release/bundle/macos/*.app | head -1)
test -x "$APP/Contents/MacOS/apicula"
test -f "$APP/Contents/Frameworks/libnitroarc_ffi.dylib"
otool -l "$APP/Contents/MacOS/studio" | grep -A2 LC_RPATH | grep -q "@executable_path/../Frameworks"
"$APP/Contents/MacOS/apicula" --help >/dev/null
echo "bundle ok: $APP"
```

- [ ] **Step 2: Skript laufen lassen, Fehlschlag prüfen** — `bash apps/studio/scripts/check-bundle.sh` → `No such file`.
- [ ] **Step 3: Implementieren** (Skripte, Konfiguration, build.rs, Docs).
- [ ] **Step 4: Verifizieren** — `cd apps/studio && pnpm bundle && bash scripts/check-bundle.sh` → `bundle ok`; manuell: die `.app` aus `target/release/bundle/macos` starten (Gatekeeper-Freigabe), Projektkopie öffnen, einen Header mit Modellen laden (apicula wird aus dem Bundle genutzt, `G4_APICULA` darf nicht gesetzt sein).
- [ ] **Step 5: Commit** — `build(studio): bundle nitroarc dylib and apicula sidecar`

---

## Gate für Phase 2a

Auf einer Kopie von Platin DE und HeartGold US, mit der gebündelten App (nicht `tauri dev`):

1. Header „Zweiblattdorf“ (0415, Spielerzimmer) wählen: texturierte Karte, blauer Warp-Marker auf der Treppe, Hover zeigt „0x5E WARP_STAIRS_EAST“ und „Warp → 0414“.
2. Kachel vor der Treppe blockieren, speichern, bauen, in melonDS prüfen (Figur bleibt stehen).
3. In DSPRE (oder per Hex-Editor) dieselbe Karte in der Kopie ändern, im Studio erneut speichern: Stale-Leiste mit „Neu laden“ und „Meine Version behalten“, beide Wege funktionieren.
4. Kachel ändern, Cmd+Q: Rückfrage.
5. Header 0003 (Jubelstadt, Matrix 0) öffnen: Overlay sofort, Modelle strömen nach, Fortschritt sichtbar; zweites Öffnen aus dem Cache deutlich schneller.
6. `cargo test --workspace --all-features`, `pnpm test`, clippy, fmt grün; gated Tests auf allen drei Projekten grün.

## Self-Review

**Spec-Abdeckung:** Eingangskriterien aus der Roadmap: Stale (Task 4, 5), Bundle (13), Cmd+Q (6), Kernel-Aufräumen (2, 3, 4), buildings_tail (1), Cache per Hash (7), Matrix-Orientierung (1), Hover (9), Marker (10), Texturen (7), lesbare Namen (11), große Matrizen (12), CI für gated Tests: nicht in diesem Plan (braucht einen Runner mit ROM-Daten; bleibt Querschnittsthema der Roadmap). Workbench (Spec 4.3) in Task 8.

**Typkonsistenz:** `AssetKind` mit neun Varianten ab Task 3 wird in 4, 5, 7, 10, 11 verwendet; `CmdError` aus Task 5 in allen späteren Commands; `InspectorModel`-Varianten `none` (8), `tile` (9), `event` (10); `HeaderSummary.event_file_id` (10) und `location_name` (11) werden beide in `project.rs` ergänzt, Task 10 zuerst.

**Review Focus:** 1 → Task 4/5, 2 → Task 6, 3 → Task 7, 4 → Task 10, 5 → Task 12.

**Offene Annahmen, die ein Test oder die Abnahme entscheidet:** Tauri legt `externalBin` und `frameworks` wie beschrieben ab (Task 13 prüft mit `check-bundle.sh`); chatots Standard-Charmap dekodiert deutsche Ortsnamen (Task 11); apicula findet Texturen bei allen Tilesets (Task 7, Vanilla-Test).
