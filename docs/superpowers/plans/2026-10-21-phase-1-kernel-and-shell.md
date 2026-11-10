# Phase 1: Kernel-Fundament und Shell-Spike. Implementierungsplan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein Rust-Kernel (`g4-formats`, `g4-kernel`, CLI `g4k`) und eine Tauri-2-Shell, mit denen ein DSPRE-kompatibles Gen-4-Projekt geöffnet, die Matrix eines Headers in 3D angezeigt, Permissions gemalt, gespeichert, das ROM gebaut und im Emulator gestartet wird.

**Architecture:** Zwei Rust-Crates ohne globalen Zustand: `g4-formats` hält Codecs (NARC, Map, Matrix, AreaData) mit byte-identischem Round-Trip; `g4-kernel` hält `Project` (Layout, Spielkennung, Header über uxie), `Store` (In-Memory-Assets, Changesets, Undo, Dirty) und `build` (NARC packen, ROM über die ds-rom-Bibliothek). Die Tauri-App ruft den Kernel über Commands; three.js rendert Permissions, Gebäude-Boxen und per apicula zu GLB konvertierte Modelle.

**Tech Stack:** Rust stable (Edition 2024), `uxie 0.9`, `ds-rom 0.8`, `binrw 0.15`, `thiserror 2`, `serde 1`, `serde_json 1`, `serde_yaml 0.9`, `clap 4`, `tempfile 3`, `assert_cmd 2`; Tauri 2.12, React 19, TypeScript, three 0.186, vitest; apicula (Sidecar-CLI).

**Spec:** [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](../specs/2026-10-21-gen4-studio-design.md). Roadmap: [docs/ROADMAP.md](../../ROADMAP.md).

## Global Constraints

- Repository-Wurzel: `$HOME/Projekte/lab/editor/gen4-studio`. Cargo-Workspace mit Membern `crates/g4-formats`, `crates/g4-kernel`, `apps/studio/src-tauri`.
- Rust stable, Edition 2024, `rust-toolchain.toml` mit `channel = "stable"`. Node 24, pnpm 11.
- Crate-Versionen: `uxie = "0.9"`, `ds-rom = "0.8"`, `binrw = "0.15"`, `thiserror = "2"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `serde_yaml = "0.9"`, `clap = { version = "4", features = ["derive"] }`. Dev: `tempfile = "3"`, `assert_cmd = "2"`, `predicates = "3"`.
- npm: `create-tauri-app@4.7.5`, `@tauri-apps/api@2.12`, `@tauri-apps/cli@2.12`, `@tauri-apps/plugin-dialog@2`, `three@0.186`, `vitest@3`.
- Lizenz MIT in jedem `Cargo.toml` und `package.json`. Keine GPL- oder AGPL-Crates.
- Keine ROM-Daten im Repository. Tests gegen Vanilla-Daten lesen `G4_TEST_PROJECT` (Pfad zu einem extrahierten ds-rom-Projekt mit `unpacked/maps`, `unpacked/matrices`, `unpacked/areaData`) und geben ohne die Variable früh zurück mit `eprintln!("skipped: G4_TEST_PROJECT not set")`.
- Entpackte Member heißen `{:04}` (vierstellig), Ordner `unpacked/maps`, `unpacked/matrices`, `unpacked/areaData`, `unpacked/eventFiles`. ROM-Dateisystem-Wurzel ist `files/`.
- Codecs erhalten unbekannte Bytes verbatim. Jeder Codec hat einen synthetischen und einen gegateten Vanilla-Round-Trip-Test.
- Bezeichner Englisch, Doku Deutsch, Commits Conventional Commits (`feat:`, `test:`, `chore:`), Englisch.
- Kein `unwrap()` außerhalb von Tests. Fehler sind typisiert (`FormatError`, `KernelError`).

## Review Focus

1. HGSS-Karte mit leerem BGS-Block (`34 12 00 00`): `write` muss genau diese vier Bytes ausgeben, nicht null Bytes. Test `map_hgss_blank_bgs_writes_four_bytes` in Task 3.
2. Matrixname mit Nicht-ASCII-Zeichen (z. B. "Höhle"): das Längenbyte zählt UTF-8-Bytes, nicht Zeichen. Test `matrix_name_length_counts_utf8_bytes` in Task 4.
3. Lücke in `unpacked/maps` (z. B. `0041`, `0043` ohne `0042`): Packen darf die Member nicht stillschweigend verschieben. Test `pack_dir_with_gap_is_error` in Task 7.
4. Speichern, während DSPRE dieselbe Datei verändert hat: `save` darf fremde Änderungen nicht überschreiben. Test `save_refuses_when_disk_changed_since_load` in Task 6.
5. Permission-Koordinate außerhalb 0..32: darf nicht panicken, sondern `FormatError::OutOfRange` liefern. Test `set_cell_out_of_range_is_error` in Task 3.

---

## Dateistruktur

```
gen4-studio/
  Cargo.toml                         Workspace
  rust-toolchain.toml
  .gitignore
  LICENSE                            MIT
  crates/g4-formats/
    Cargo.toml
    src/lib.rs                       pub mod game, error, narc, map, matrix, area_data
    src/game.rs                      Family, Language
    src/error.rs                     FormatError
    src/narc.rs                      Narc
    src/map.rs                       MapFile, Building, Cell
    src/matrix.rs                    Matrix
    src/area_data.rs                 AreaData
    tests/vanilla.rs                 gegatete Round-Trips
  crates/g4-kernel/
    Cargo.toml
    metadata/archives.yaml           NARC-Pfade je Familie
    src/lib.rs
    src/error.rs                     KernelError
    src/project.rs                   Project, AssetKind, ArchiveMap, HeaderSummary
    src/store.rs                     Store, AssetId, ChangeId, SaveReport
    src/build.rs                     pack_dir, build
    src/export.rs                    nsbmd_to_glb
    src/run.rs                       launch_emulator
    src/bin/g4k.rs                   CLI
    tests/common/mod.rs              synthetic_project(), vanilla_project()
    tests/project.rs, store.rs, build.rs, cli.rs
  apps/studio/
    package.json, vite.config.ts, vitest.config.ts
    src-tauri/Cargo.toml, src-tauri/src/main.rs, lib.rs, commands.rs, state.rs
    src/api.ts                       typisierte invoke-Wrapper
    src/App.tsx
    src/world/MatrixScene.tsx        three.js
    src/world/layout.ts              cellOrigin, buildingBox
    src/world/colors.ts              permissionColor
    src/world/*.test.ts
  docs/formats/narc.md, map.md, matrix.md, area_data.md
```

---

### Task 1: Repository und `g4-formats` mit Spielkennung

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `LICENSE`, `crates/g4-formats/Cargo.toml`, `crates/g4-formats/src/lib.rs`, `crates/g4-formats/src/game.rs`, `crates/g4-formats/src/error.rs`
- Test: `crates/g4-formats/src/game.rs` (Modultests)

**Interfaces:**
- Produces: `g4_formats::game::{Family, Language}` mit `Family::from_game_code(code: &str) -> Option<Family>` (Varianten `Dp`, `Pt`, `Hgss`; Präfixe `ADA`/`APA` → `Dp`, `CPU` → `Pt`, `IPK`/`IPG` → `Hgss`), `Language::from_game_code(code: &str) -> Option<Language>` (4. Zeichen: `E`→`English`, `F`→`French`, `D`→`German`, `I`→`Italian`, `S`→`Spanish`, `J`→`Japanese`, `K`→`Korean`). `g4_formats::error::FormatError` mit Varianten `Truncated { needed: usize, have: usize }`, `BadMagic { expected: &'static str }`, `BadSignature { expected: u16, found: u16 }`, `BadSize { expected: usize, found: usize }`, `OutOfRange { what: &'static str, value: usize, max: usize }`.

- [ ] **Step 1: Rust installieren und prüfen**

Run: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source "$HOME/.cargo/env" && cargo --version`
Expected: `cargo 1.9x.x (...)`

- [ ] **Step 2: Repository anlegen**

Run: `cd $HOME/Projekte/lab/editor/gen4-studio && git init -b main && git add docs && git commit -m "docs: design spec, roadmap, phase 1 plan"`
Expected: `1 file changed` je Dokument, kein Fehler.

- [ ] **Step 3: Workspace-Dateien schreiben**

`Cargo.toml`: `[workspace] resolver = "3"`, `members = ["crates/g4-formats", "crates/g4-kernel", "apps/studio/src-tauri"]`, `[workspace.package] edition = "2024"`, `license = "MIT"`. Die beiden noch nicht existierenden Member werden in Task 5 und Task 9 angelegt; bis dahin stehen nur `crates/g4-formats` im Array.
`rust-toolchain.toml`: `[toolchain] channel = "stable"`.
`.gitignore`: `target/`, `node_modules/`, `dist/`, `*.nds`, `tools/`, `.DS_Store`.
`crates/g4-formats/Cargo.toml`: `[package] name = "g4-formats"`, `version = "0.1.0"`, `edition.workspace = true`, `license.workspace = true`; `[dependencies] thiserror = "2"`.

- [ ] **Step 4: Failing Tests für `Family` und `Language` schreiben**

In `crates/g4-formats/src/game.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_from_game_code() {
        assert_eq!(Family::from_game_code("ADAE"), Some(Family::Dp));
        assert_eq!(Family::from_game_code("APAJ"), Some(Family::Dp));
        assert_eq!(Family::from_game_code("CPUE"), Some(Family::Pt));
        assert_eq!(Family::from_game_code("IPKE"), Some(Family::Hgss));
        assert_eq!(Family::from_game_code("IPGD"), Some(Family::Hgss));
        assert_eq!(Family::from_game_code("XXXX"), None);
        assert_eq!(Family::from_game_code("CP"), None);
    }

    #[test]
    fn language_from_game_code() {
        assert_eq!(Language::from_game_code("CPUE"), Some(Language::English));
        assert_eq!(Language::from_game_code("IPGD"), Some(Language::German));
        assert_eq!(Language::from_game_code("ADAJ"), Some(Language::Japanese));
        assert_eq!(Language::from_game_code("IPKK"), Some(Language::Korean));
        assert_eq!(Language::from_game_code("CPUX"), None);
    }
}
```

- [ ] **Step 5: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-formats`
Expected: Compile-Fehler `cannot find type Family`.

- [ ] **Step 6: `Family`, `Language` (beide `Copy, Clone, Debug, PartialEq, Eq`) und `FormatError` (`#[derive(Debug, thiserror::Error)]`) implementieren; `lib.rs` mit `pub mod game; pub mod error;`**

- [ ] **Step 7: Tests laufen lassen**

Run: `cargo test -p g4-formats`
Expected: `test result: ok. 2 passed`

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore LICENSE crates/g4-formats
git commit -m "feat(formats): workspace scaffold with game identity and FormatError"
```

---

### Task 2: NARC-Codec

**Files:**
- Create: `crates/g4-formats/src/narc.rs`, `crates/g4-formats/tests/vanilla.rs`, `docs/formats/narc.md`
- Modify: `crates/g4-formats/src/lib.rs`

**Interfaces:**
- Produces: `g4_formats::narc::Narc { pub members: Vec<Vec<u8>> }`, `Narc::read(bytes: &[u8]) -> Result<Narc, FormatError>`, `Narc::write(&self) -> Vec<u8>`.

Layout (Little Endian), aus gbatek und DSPREs `Narc.cs` bestätigt:
- Header 16 Bytes: `b"NARC"`, `u16 0xFFFE`, `u16 0x0100`, `u32 total_size`, `u16 0x0010`, `u16 3`.
- BTAF: `b"BTAF"`, `u32 size = 12 + 8*n`, `u16 n`, `u16 0`, dann n × (`u32 start`, `u32 end`), relativ zum Beginn der GMIF-Nutzdaten. `start` ist auf 4 ausgerichtet, `end = start + len`.
- BTNF: `b"BTNF"`, `u32 0x10`, `u32 0x4`, `u32 0x00010000` (namenlos).
- GMIF: `b"GMIF"`, `u32 size = 8 + payload_len`, Nutzdaten; jedes Member wird mit `0xFF` auf ein Vielfaches von 4 aufgefüllt, auch das letzte.

- [ ] **Step 1: Failing Tests schreiben**

In `narc.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narc_round_trips_members_with_padding() {
        let narc = Narc { members: vec![vec![1, 2, 3], vec![], vec![9; 8]] };
        let bytes = narc.write();
        assert_eq!(&bytes[0..4], b"NARC");
        assert_eq!(&bytes[4..8], &[0xFE, 0xFF, 0x00, 0x01]);
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize, bytes.len());
        assert_eq!(&bytes[12..16], &[0x10, 0x00, 0x03, 0x00]);
        assert_eq!(&bytes[16..20], b"BTAF");
        assert_eq!(u16::from_le_bytes(bytes[24..26].try_into().unwrap()), 3);
        let back = Narc::read(&bytes).unwrap();
        assert_eq!(back.members, narc.members);
    }

    #[test]
    fn narc_pads_each_member_to_four_bytes_with_ff() {
        let bytes = Narc { members: vec![vec![1, 2, 3], vec![4]] }.write();
        let gmif = bytes.windows(4).position(|w| w == b"GMIF").unwrap();
        let payload = &bytes[gmif + 8..];
        assert_eq!(payload, &[1, 2, 3, 0xFF, 4, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn narc_rejects_bad_magic() {
        let err = Narc::read(b"NOPE").unwrap_err();
        assert!(matches!(err, FormatError::BadMagic { expected: "NARC" }));
    }
}
```

In `tests/vanilla.rs`:

```rust
use std::path::PathBuf;

pub fn vanilla() -> Option<PathBuf> {
    match std::env::var_os("G4_TEST_PROJECT") {
        Some(p) => Some(PathBuf::from(p)),
        None => { eprintln!("skipped: G4_TEST_PROJECT not set"); None }
    }
}

#[test]
fn vanilla_area_data_narc_round_trips_byte_identical() {
    let Some(root) = vanilla() else { return };
    let candidates = ["files/fielddata/areadata/area_data.narc", "files/a/0/4/2"];
    let path = candidates.iter().map(|c| root.join(c)).find(|p| p.exists()).expect("area data narc");
    let bytes = std::fs::read(&path).unwrap();
    let narc = g4_formats::narc::Narc::read(&bytes).unwrap();
    assert_eq!(narc.write(), bytes);
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-formats narc`
Expected: Compile-Fehler `unresolved import g4_formats::narc`.

- [ ] **Step 3: `Narc::read` und `Narc::write` in `narc.rs` implementieren; `pub mod narc;` in `lib.rs`**

`read` prüft Magic, liest n aus BTAF, schneidet jedes Member aus der GMIF-Nutzlast (`start..end`), ignoriert Padding. Ein `end > payload_len` ergibt `FormatError::Truncated`.

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-formats narc` und `G4_TEST_PROJECT=<pfad> cargo test -p g4-formats --test vanilla`
Expected: alle `ok`. Schlägt der Vanilla-Test an Padding-Bytes fehl, ist die Annahme `0xFF` falsch; dann `0x00` verwenden und die Abweichung in `docs/formats/narc.md` festhalten.

- [ ] **Step 5: `docs/formats/narc.md` mit dem Byte-Layout und der Quelle (gbatek, DSPRE `Narc.cs` Zeilen 87 bis 129) schreiben**

- [ ] **Step 6: Commit**

```bash
git add crates/g4-formats docs/formats/narc.md
git commit -m "feat(formats): NARC reader and writer with byte-identical round trip"
```

---

### Task 3: Map-Codec

**Files:**
- Create: `crates/g4-formats/src/map.rs`, `docs/formats/map.md`
- Modify: `crates/g4-formats/src/lib.rs`, `crates/g4-formats/tests/vanilla.rs`

**Interfaces:**
- Consumes: `Family`, `FormatError` (Task 1).
- Produces:

```rust
pub const MAP_SIZE: usize = 32;
pub const PERMISSIONS_LEN: usize = 2048;
pub const BUILDING_LEN: usize = 48;
pub const BGS_SIGNATURE: u16 = 0x1234;
pub const BLANK_BGS: [u8; 4] = [0x34, 0x12, 0x00, 0x00];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Cell { pub kind: u8, pub collision: u8 }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Building {
    pub model_id: u32,
    pub x_frac: u16, pub x: u16,
    pub z_frac: u16, pub z: u16,
    pub y_frac: u16, pub y: u16,
    pub unknown_10: [u8; 13],   // Offset 0x10..0x1D, enthält laut DSPRE die Rotation; Phase 2 dekodiert
    pub width: u32, pub height: u32, pub length: u32,   // 0x1D, 0x21, 0x25
    pub tail: [u8; 7],          // 0x29..0x30
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapFile {
    pub bgs: Option<Vec<u8>>,       // HGSS: kompletter Block inkl. Signatur und Länge; DPPt: None
    pub permissions: Vec<Cell>,     // 1024 Einträge, Index y * 32 + x
    pub buildings: Vec<Building>,
    pub model: Vec<u8>,             // NSBMD-Bytes verbatim
    pub bdhc: Vec<u8>,
}

impl MapFile {
    pub fn read(bytes: &[u8], family: Family) -> Result<MapFile, FormatError>;
    pub fn write(&self, family: Family) -> Vec<u8>;
    pub fn cell(&self, x: usize, y: usize) -> Result<Cell, FormatError>;
    pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) -> Result<(), FormatError>;
}
```

Layout: vier `u32` Sektionslängen (Permissions, Buildings, NSBMD, BDHC); bei HGSS folgt der BGS-Block (`u16 0x1234`, `u16 len`, `len` Bytes); dann Permissions (1024 Bytepaare `kind, collision`), Gebäude (je 48 Bytes), NSBMD, BDHC. Bei HGSS ohne Signatur `0x1234` ist das Ergebnis `FormatError::BadSignature { expected: 0x1234, found }`.

- [ ] **Step 1: Failing Tests schreiben**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Family;

    fn synthetic(family: Family, bgs: Option<&[u8]>) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&2048u32.to_le_bytes());
        v.extend_from_slice(&48u32.to_le_bytes());
        v.extend_from_slice(&6u32.to_le_bytes());
        v.extend_from_slice(&3u32.to_le_bytes());
        if family == Family::Hgss { v.extend_from_slice(bgs.unwrap()); }
        for i in 0..1024u16 { v.push((i % 7) as u8); v.push(if i % 5 == 0 { 0x80 } else { 0 }); }
        let mut b = vec![0u8; 48];
        b[0..4].copy_from_slice(&7u32.to_le_bytes());
        b[6..8].copy_from_slice(&3u16.to_le_bytes());
        b[0x1D..0x21].copy_from_slice(&16u32.to_le_bytes());
        b[0x10] = 0xAB;
        v.extend_from_slice(&b);
        v.extend_from_slice(b"BMD0\x01\x02");
        v.extend_from_slice(&[0xCA, 0xFE, 0x01]);
        v
    }

    #[test]
    fn map_dppt_round_trips() {
        let bytes = synthetic(Family::Pt, None);
        let map = MapFile::read(&bytes, Family::Pt).unwrap();
        assert_eq!(map.bgs, None);
        assert_eq!(map.buildings.len(), 1);
        assert_eq!(map.buildings[0].model_id, 7);
        assert_eq!(map.buildings[0].x, 3);
        assert_eq!(map.buildings[0].width, 16);
        assert_eq!(map.buildings[0].unknown_10[0], 0xAB);
        assert_eq!(map.cell(0, 0).unwrap(), Cell { kind: 0, collision: 0x80 });
        assert_eq!(map.cell(1, 0).unwrap(), Cell { kind: 1, collision: 0 });
        assert_eq!(map.write(Family::Pt), bytes);
    }

    #[test]
    fn map_hgss_round_trips_with_bgs() {
        let bgs = [0x34, 0x12, 0x02, 0x00, 0xAA, 0xBB];
        let bytes = synthetic(Family::Hgss, Some(&bgs));
        let map = MapFile::read(&bytes, Family::Hgss).unwrap();
        assert_eq!(map.bgs.as_deref(), Some(&bgs[..]));
        assert_eq!(map.write(Family::Hgss), bytes);
    }

    #[test]
    fn map_hgss_blank_bgs_writes_four_bytes() {
        let bytes = synthetic(Family::Hgss, Some(&BLANK_BGS));
        let map = MapFile::read(&bytes, Family::Hgss).unwrap();
        assert_eq!(map.bgs.as_deref(), Some(&BLANK_BGS[..]));
        assert_eq!(map.write(Family::Hgss), bytes);
    }

    #[test]
    fn map_hgss_without_signature_is_error() {
        let bytes = synthetic(Family::Hgss, Some(&[0x00, 0x00, 0x00, 0x00]));
        let err = MapFile::read(&bytes, Family::Hgss).unwrap_err();
        assert!(matches!(err, FormatError::BadSignature { expected: 0x1234, found: 0 }));
    }

    #[test]
    fn set_cell_out_of_range_is_error() {
        let mut map = MapFile::read(&synthetic(Family::Pt, None), Family::Pt).unwrap();
        assert!(map.set_cell(32, 0, Cell::default()).is_err());
        assert!(map.set_cell(0, 32, Cell::default()).is_err());
        map.set_cell(31, 31, Cell { kind: 2, collision: 0x80 }).unwrap();
        assert_eq!(map.cell(31, 31).unwrap().collision, 0x80);
    }
}
```

In `tests/vanilla.rs` ergänzen:

```rust
#[test]
fn vanilla_maps_round_trip_byte_identical() {
    let Some(root) = vanilla() else { return };
    let family = family_of(&root);   // liest files/../header.yaml game_code; Helfer in vanilla.rs
    let dir = root.join("unpacked/maps");
    let mut n = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        let map = g4_formats::map::MapFile::read(&bytes, family).unwrap();
        assert_eq!(map.write(family), bytes);
        n += 1;
    }
    assert!(n > 0);
}
```

`family_of(root)` liest `root/header.yaml`, extrahiert die Zeile `game_code: "XXXX"` per einfachem String-Parsing und ruft `Family::from_game_code`.

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-formats map`
Expected: Compile-Fehler `unresolved import`.

- [ ] **Step 3: `map.rs` implementieren; `pub mod map;` in `lib.rs`**

Sektionslängen beim Schreiben aus den Feldern berechnen (`permissions.len() * 2`, `buildings.len() * 48`, `model.len()`, `bdhc.len()`). Ein Permissions-Abschnitt mit anderer Länge als 2048 ergibt `FormatError::BadSize { expected: 2048, found }`.

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-formats map` und `G4_TEST_PROJECT=<pfad> cargo test -p g4-formats --test vanilla`
Expected: alle `ok`.

- [ ] **Step 5: `docs/formats/map.md` schreiben (Layout, Quelle: DSPRE `MapFile.cs` Kopfkommentar und Konstruktor Zeilen 81 bis 128)**

- [ ] **Step 6: Commit**

```bash
git add crates/g4-formats docs/formats/map.md
git commit -m "feat(formats): map file codec with HGSS BGS block and cell access"
```

---

### Task 4: Matrix- und AreaData-Codecs

**Files:**
- Create: `crates/g4-formats/src/matrix.rs`, `crates/g4-formats/src/area_data.rs`, `docs/formats/matrix.md`, `docs/formats/area_data.md`
- Modify: `crates/g4-formats/src/lib.rs`, `crates/g4-formats/tests/vanilla.rs`

**Interfaces:**
- Produces:

```rust
pub const MATRIX_EMPTY: u16 = 0xFFFF;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matrix {
    pub name: String,
    pub width: u8, pub height: u8,
    pub headers: Option<Vec<u16>>,   // Länge width*height, Index y*width+x
    pub altitudes: Option<Vec<u8>>,
    pub maps: Vec<u16>,
}
impl Matrix {
    pub fn read(bytes: &[u8]) -> Result<Matrix, FormatError>;
    pub fn write(&self) -> Vec<u8>;
    pub fn map_at(&self, x: usize, y: usize) -> Option<u16>;   // None bei MATRIX_EMPTY oder außerhalb
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaData {
    Dppt { buildings_tileset: u16, map_tileset: u16, unknown: u16, light_type: u16 },
    Hgss { buildings_tileset: u16, map_tileset: u16, dynamic_texture_type: u16, area_type: u8, light_type: u8 },
}
impl AreaData {
    pub fn read(bytes: &[u8], family: Family) -> Result<AreaData, FormatError>;   // genau 8 Bytes
    pub fn write(&self) -> Vec<u8>;
    pub fn buildings_tileset(&self) -> u16;
    pub fn map_tileset(&self) -> u16;
}
```

Matrix-Layout: `u8 width`, `u8 height`, `u8 has_headers`, `u8 has_altitudes`, `u8 name_len`, Name (UTF-8), dann optional Header-Sektion (`u16` × w×h), optional Höhen (`u8` × w×h), dann Karten (`u16` × w×h). AreaData: 8 Bytes, Reihenfolge wie in den Varianten.

- [ ] **Step 1: Failing Tests schreiben**

```rust
// matrix.rs
#[test]
fn matrix_round_trips_all_sections() {
    let m = Matrix { name: "Route".into(), width: 2, height: 1,
        headers: Some(vec![10, 11]), altitudes: Some(vec![0, 3]), maps: vec![5, MATRIX_EMPTY] };
    let bytes = m.write();
    assert_eq!(&bytes[0..5], &[2, 1, 1, 1, 5]);
    assert_eq!(&bytes[5..10], b"Route");
    assert_eq!(Matrix::read(&bytes).unwrap(), m);
    assert_eq!(m.map_at(0, 0), Some(5));
    assert_eq!(m.map_at(1, 0), None);
    assert_eq!(m.map_at(0, 1), None);
}

#[test]
fn matrix_name_length_counts_utf8_bytes() {
    let m = Matrix { name: "Höhle".into(), width: 1, height: 1, headers: None, altitudes: None, maps: vec![0] };
    let bytes = m.write();
    assert_eq!(bytes[4], 6);
    assert_eq!(Matrix::read(&bytes).unwrap().name, "Höhle");
}

#[test]
fn matrix_truncated_is_error() {
    assert!(matches!(Matrix::read(&[2, 2, 0, 0, 0, 1]).unwrap_err(), FormatError::Truncated { .. }));
}

// area_data.rs
#[test]
fn area_data_dppt_round_trips() {
    let a = AreaData::Dppt { buildings_tileset: 1, map_tileset: 2, unknown: 3, light_type: 4 };
    let bytes = a.write();
    assert_eq!(bytes, [1, 0, 2, 0, 3, 0, 4, 0]);
    assert_eq!(AreaData::read(&bytes, Family::Pt).unwrap(), a);
}

#[test]
fn area_data_hgss_round_trips() {
    let a = AreaData::Hgss { buildings_tileset: 1, map_tileset: 2, dynamic_texture_type: 3, area_type: 1, light_type: 9 };
    let bytes = a.write();
    assert_eq!(bytes, [1, 0, 2, 0, 3, 0, 1, 9]);
    assert_eq!(AreaData::read(&bytes, Family::Hgss).unwrap(), a);
}

#[test]
fn area_data_wrong_size_is_error() {
    assert!(matches!(AreaData::read(&[0; 7], Family::Pt).unwrap_err(), FormatError::BadSize { expected: 8, found: 7 }));
}
```

In `tests/vanilla.rs`: `vanilla_matrices_round_trip_byte_identical` über `unpacked/matrices` und `vanilla_area_data_round_trip_byte_identical` über `unpacked/areaData`, analog zum Map-Test.

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-formats matrix area_data`
Expected: Compile-Fehler.

- [ ] **Step 3: `matrix.rs` und `area_data.rs` implementieren; Module in `lib.rs` eintragen**

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-formats` und `G4_TEST_PROJECT=<pfad> cargo test -p g4-formats --test vanilla`
Expected: alle `ok`.

- [ ] **Step 5: `docs/formats/matrix.md` und `docs/formats/area_data.md` schreiben (Quelle: DSPRE `GameMatrix.cs` Kopfkommentar, `AreaData.cs`)**

- [ ] **Step 6: Commit**

```bash
git add crates/g4-formats docs/formats
git commit -m "feat(formats): matrix and area data codecs"
```

---

### Task 5: `g4-kernel` mit `Project`

**Files:**
- Create: `crates/g4-kernel/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/project.rs`, `metadata/archives.yaml`, `tests/common/mod.rs`, `tests/project.rs`
- Modify: `Cargo.toml` (Member ergänzen)

**Interfaces:**
- Consumes: `g4_formats::{game::{Family, Language}, narc::Narc}`.
- Produces:

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("not a ds-rom project: {0} (header.yaml missing)")] NotAProject(PathBuf),
    #[error("unknown game code {0}")] UnknownGame(String),
    #[error(transparent)] Io(#[from] std::io::Error),
    #[error(transparent)] Format(#[from] g4_formats::error::FormatError),
    #[error("uxie: {0}")] Uxie(String),
    #[error("tool {name} unavailable: {reason}")] ToolMissing { name: &'static str, reason: String },
    #[error("build: {0}")] Build(String),
    #[error("stale: {0:?} changed on disk since it was loaded")] Stale(AssetId),
}

// project.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetKind { Map, Matrix, AreaData }

pub struct ArchiveMap { /* aus archives.yaml */ }
impl ArchiveMap {
    pub fn packed(&self, kind: AssetKind) -> &Path;       // relativ zur Projektwurzel, z. B. files/a/0/6/5
    pub fn unpacked_name(&self, kind: AssetKind) -> &str;  // "maps" | "matrices" | "areaData"
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HeaderSummary { pub id: u16, pub internal_name: String, pub matrix_id: u16, pub area_data_id: u8 }

pub struct Project { pub root: PathBuf, pub game_code: String, pub family: Family, pub language: Language, pub archives: ArchiveMap }
impl Project {
    pub fn open(root: impl AsRef<Path>) -> Result<Project, KernelError>;
    pub fn unpacked_dir(&self, kind: AssetKind) -> PathBuf;           // root/unpacked/<name>
    pub fn member_path(&self, kind: AssetKind, index: u16) -> PathBuf; // root/unpacked/<name>/NNNN
    pub fn ensure_unpacked(&self, kind: AssetKind) -> Result<usize, KernelError>; // entpackt bei Bedarf, liefert Member-Anzahl
    pub fn member_count(&self, kind: AssetKind) -> Result<usize, KernelError>;
    pub fn headers(&self) -> Result<Vec<HeaderSummary>, KernelError>;
}
```

`metadata/archives.yaml` (per `include_str!` eingebettet):

```yaml
dp:
  maps:      { packed: "files/fielddata/land_data/land_data_release.narc", packed_jp: "files/fielddata/land_data/land_data.narc", unpacked: "maps" }
  matrices:  { packed: "files/fielddata/mapmatrix/map_matrix.narc", unpacked: "matrices" }
  area_data: { packed: "files/fielddata/areadata/area_data.narc", unpacked: "areaData" }
pt:
  maps:      { packed: "files/fielddata/land_data/land_data.narc", unpacked: "maps" }
  matrices:  { packed: "files/fielddata/mapmatrix/map_matrix.narc", unpacked: "matrices" }
  area_data: { packed: "files/fielddata/areadata/area_data.narc", unpacked: "areaData" }
hgss:
  maps:      { packed: "files/a/0/6/5", unpacked: "maps" }
  matrices:  { packed: "files/a/0/4/1", unpacked: "matrices" }
  area_data: { packed: "files/a/0/4/2", unpacked: "areaData" }
```

`packed_jp` gilt für `Language::Japanese` (Quelle: DSPRE `RomInfo.SetNarcDirs`, Suffix `_release` für alle nicht-japanischen DP-Dumps).

`headers()`: `uxie::workspace::Workspace::open(&self.root)` (uxie 0.9.7, `src/workspace.rs` Zeile 66), Anzahl aus `files/fielddata/maptable/mapname.bin` (Dateilänge / 16), Name über `Workspace::get_map_internal_name(id)`, `matrix_id` und `area_data_id` aus dem `MapHeader` des Providers (Methodenname in `src/provider.rs` ab Zeile 21, Trait `DataProvider`). uxie-Fehler werden als `KernelError::Uxie(format!("{e}"))` gemeldet.

- [ ] **Step 1: Crate anlegen**

`crates/g4-kernel/Cargo.toml`: `[package] name = "g4-kernel"`, Workspace-Felder, `[dependencies] g4-formats = { path = "../g4-formats" }`, `uxie = "0.9"`, `thiserror = "2"`, `serde`, `serde_json`, `serde_yaml`; `[dev-dependencies] tempfile = "3"`. Member in Workspace-`Cargo.toml` ergänzen.

Run: `cargo build -p g4-kernel`
Expected: baut (uxie kompiliert nitroarc über `build.rs`; ein C-Compiler muss vorhanden sein: `xcode-select --install` auf macOS).

- [ ] **Step 2: Synthetischen Projekt-Helfer und failing Tests schreiben**

`tests/common/mod.rs`:

```rust
use std::path::{Path, PathBuf};
use g4_formats::narc::Narc;

pub struct Synthetic { pub dir: tempfile::TempDir }

/// ds-rom-Layout für Platinum US mit 3 Karten, 1 Matrix (2x1, Karten 0 und 1), 2 AreaData, gepackt.
pub fn synthetic_project() -> Synthetic {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("header.yaml"), "game_title: \"POKEMON PL\"\ngame_code: \"CPUE\"\nmaker_code: \"01\"\n").unwrap();
    std::fs::write(dir.path().join("config.yaml"), "").unwrap();
    let maps: Vec<Vec<u8>> = (0..3u8).map(|i| synthetic_map_bytes(i)).collect();
    write_narc(dir.path(), "files/fielddata/land_data/land_data.narc", &maps);
    let matrix = g4_formats::matrix::Matrix { name: "m".into(), width: 2, height: 1, headers: None, altitudes: None, maps: vec![0, 1] };
    write_narc(dir.path(), "files/fielddata/mapmatrix/map_matrix.narc", &[matrix.write()]);
    write_narc(dir.path(), "files/fielddata/areadata/area_data.narc", &[vec![0;8], vec![1,0,2,0,0,0,0,0]]);
    Synthetic { dir }
}

pub fn synthetic_map_bytes(seed: u8) -> Vec<u8> { /* wie Task 3 `synthetic(Family::Pt, None)`, model = [seed; 6] */ }

fn write_narc(root: &Path, rel: &str, members: &[Vec<u8>]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, Narc { members: members.to_vec() }.write()).unwrap();
}

pub fn vanilla_project() -> Option<PathBuf> { /* wie in g4-formats tests/vanilla.rs */ }
```

`tests/project.rs`:

```rust
mod common;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::error::KernelError;
use g4_formats::game::{Family, Language};

#[test]
fn open_reads_identity_from_header_yaml() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert_eq!(p.game_code, "CPUE");
    assert_eq!(p.family, Family::Pt);
    assert_eq!(p.language, Language::English);
    assert_eq!(p.archives.packed(AssetKind::Map), std::path::Path::new("files/fielddata/land_data/land_data.narc"));
}

#[test]
fn open_without_header_yaml_is_not_a_project() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(Project::open(dir.path()).unwrap_err(), KernelError::NotAProject(_)));
}

#[test]
fn ensure_unpacked_writes_four_digit_members() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    assert_eq!(p.ensure_unpacked(AssetKind::Map).unwrap(), 3);
    assert!(p.member_path(AssetKind::Map, 2).ends_with("unpacked/maps/0002"));
    assert!(p.member_path(AssetKind::Map, 2).exists());
    assert_eq!(p.member_count(AssetKind::Map).unwrap(), 3);
    assert_eq!(p.ensure_unpacked(AssetKind::Map).unwrap(), 3); // zweiter Aufruf entpackt nicht erneut
}

#[test]
fn vanilla_headers_count_matches_mapname_bin() {
    let Some(root) = common::vanilla_project() else { return };
    let p = Project::open(&root).unwrap();
    let expected = std::fs::metadata(root.join("files/fielddata/maptable/mapname.bin")).unwrap().len() / 16;
    let headers = p.headers().unwrap();
    assert_eq!(headers.len() as u64, expected);
    assert!(headers.iter().all(|h| !h.internal_name.is_empty()));
}
```

- [ ] **Step 3: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel --test project`
Expected: Compile-Fehler.

- [ ] **Step 4: `error.rs`, `project.rs`, `archives.yaml`, `lib.rs` (`pub mod error; pub mod project;`) implementieren**

`open` liest `header.yaml` mit `serde_yaml` in eine Struktur mit nur `game_code: String`. `ensure_unpacked` liest den gepackten NARC mit `g4_formats::narc::Narc::read` und schreibt die Member, wenn der Ordner fehlt.

- [ ] **Step 5: Tests laufen lassen**

Run: `cargo test -p g4-kernel --test project` und mit `G4_TEST_PROJECT`
Expected: alle `ok`. Schlägt `vanilla_headers_count_matches_mapname_bin` für HGSS fehl, weil uxie die Header-Tabelle für den Dump nicht kennt, wird der Dump in `docs/formats/headers.md` als "uxie-Lücke" notiert und das Projekt mit Platin fortgesetzt; die Lücke geht als Issue an uxie.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/g4-kernel
git commit -m "feat(kernel): project model with archive map, unpacking and header listing"
```

---

### Task 6: `Store` mit Changesets, Undo und Save

**Files:**
- Create: `crates/g4-kernel/src/store.rs`, `tests/store.rs`
- Modify: `crates/g4-kernel/src/lib.rs`, `src/error.rs` (Variante `Stale` ist schon definiert)

**Interfaces:**
- Consumes: `Project`, `AssetKind` (Task 5), `MapFile`, `Matrix` (Tasks 3, 4).
- Produces:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetId { pub kind: AssetKind, pub index: u16 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangeId(pub u64);

#[derive(Debug, Default, serde::Serialize)]
pub struct SaveReport { pub written: Vec<AssetId> }

pub struct Store { /* project: Arc<Project>, geladene Assets mit saved_bytes/current, undo/redo Stacks */ }
impl Store {
    pub fn new(project: Arc<Project>) -> Store;
    pub fn project(&self) -> &Project;
    pub fn map(&mut self, index: u16) -> Result<&MapFile, KernelError>;       // lädt bei Bedarf von member_path
    pub fn matrix(&mut self, index: u16) -> Result<&Matrix, KernelError>;
    pub fn edit_map(&mut self, index: u16, label: &str, edit: impl FnOnce(&mut MapFile)) -> Result<ChangeId, KernelError>;
    pub fn edit_matrix(&mut self, index: u16, label: &str, edit: impl FnOnce(&mut Matrix)) -> Result<ChangeId, KernelError>;
    pub fn undo(&mut self) -> Option<ChangeId>;
    pub fn redo(&mut self) -> Option<ChangeId>;
    pub fn is_dirty(&self, id: AssetId) -> bool;        // write(current) != saved_bytes
    pub fn dirty(&self) -> Vec<AssetId>;
    pub fn save(&mut self) -> Result<SaveReport, KernelError>;
}
```

Ein Changeset speichert `(AssetId, before: Vec<u8>, after: Vec<u8>, label)`. `save` prüft vor dem Schreiben jedes dirty Assets, ob die Datei auf der Platte noch `saved_bytes` entspricht; sonst `KernelError::Stale(id)` und es wird nichts geschrieben. Nach erfolgreichem Schreiben wird `saved_bytes = after`.

- [ ] **Step 1: Failing Tests schreiben**

`tests/store.rs`:

```rust
mod common;
use std::sync::Arc;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::store::{AssetId, Store};
use g4_kernel::error::KernelError;
use g4_formats::map::Cell;

fn store() -> (common::Synthetic, Store) {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    p.ensure_unpacked(AssetKind::Matrix).unwrap();
    (s, Store::new(Arc::new(p)))
}

#[test]
fn edit_marks_dirty_and_undo_clears_it() {
    let (_s, mut st) = store();
    let id = AssetId { kind: AssetKind::Map, index: 1 };
    assert!(!st.is_dirty(id));
    st.edit_map(1, "block tile", |m| m.set_cell(4, 4, Cell { kind: 0, collision: 0x80 }).unwrap()).unwrap();
    assert!(st.is_dirty(id));
    assert_eq!(st.dirty(), vec![id]);
    assert!(st.undo().is_some());
    assert!(!st.is_dirty(id));
    assert!(st.redo().is_some());
    assert_eq!(st.map(1).unwrap().cell(4, 4).unwrap().collision, 0x80);
}

#[test]
fn save_writes_only_dirty_members() {
    let (s, mut st) = store();
    st.edit_map(2, "x", |m| m.set_cell(0, 0, Cell { kind: 9, collision: 0 }).unwrap()).unwrap();
    let before0 = std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap();
    let report = st.save().unwrap();
    assert_eq!(report.written, vec![AssetId { kind: AssetKind::Map, index: 2 }]);
    assert_eq!(std::fs::read(s.dir.path().join("unpacked/maps/0000")).unwrap(), before0);
    let saved = std::fs::read(s.dir.path().join("unpacked/maps/0002")).unwrap();
    assert_eq!(g4_formats::map::MapFile::read(&saved, g4_formats::game::Family::Pt).unwrap().cell(0, 0).unwrap().kind, 9);
    assert!(st.dirty().is_empty());
}

#[test]
fn save_refuses_when_disk_changed_since_load() {
    let (s, mut st) = store();
    st.edit_map(0, "x", |m| m.set_cell(1, 1, Cell { kind: 1, collision: 0 }).unwrap()).unwrap();
    let path = s.dir.path().join("unpacked/maps/0000");
    let mut foreign = std::fs::read(&path).unwrap();
    foreign[16] ^= 0xFF;
    std::fs::write(&path, &foreign).unwrap();
    let err = st.save().unwrap_err();
    assert!(matches!(err, KernelError::Stale(AssetId { kind: AssetKind::Map, index: 0 })));
    assert_eq!(std::fs::read(&path).unwrap(), foreign);
}

#[test]
fn matrix_edit_round_trips_through_save() {
    let (s, mut st) = store();
    st.edit_matrix(0, "assign", |m| m.maps[1] = 2).unwrap();
    st.save().unwrap();
    let bytes = std::fs::read(s.dir.path().join("unpacked/matrices/0000")).unwrap();
    assert_eq!(g4_formats::matrix::Matrix::read(&bytes).unwrap().maps, vec![0, 2]);
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel --test store`
Expected: Compile-Fehler.

- [ ] **Step 3: `store.rs` implementieren; `pub mod store;` in `lib.rs`**

Assets intern als `enum Loaded { Map(MapFile), Matrix(Matrix) }` mit `fn to_bytes(&self, family) -> Vec<u8>` und `fn from_bytes(kind, bytes, family)`. Undo wendet `before` per `from_bytes` an, Redo `after`.

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-kernel --test store`
Expected: `4 passed`.

- [ ] **Step 5: Commit**

```bash
git add crates/g4-kernel
git commit -m "feat(kernel): asset store with changesets, undo/redo, dirty tracking and stale-safe save"
```

---

### Task 7: Build-Pipeline (NARC packen, ROM bauen) und ROM-Extraktion

**Files:**
- Create: `crates/g4-kernel/src/build.rs`, `tests/build.rs`
- Modify: `crates/g4-kernel/Cargo.toml` (`ds-rom = "0.8"`), `src/lib.rs`

**Interfaces:**
- Consumes: `Project`, `AssetKind`, `Narc`.
- Produces:

```rust
pub struct BuildReport { pub packed: Vec<(AssetKind, usize)>, pub rom: PathBuf }

/// Liest alle Dateien `NNNN` in `dir`, numerisch sortiert; eine Lücke oder ein Name, der nicht vierstellig numerisch ist, ist ein Fehler.
pub fn pack_dir(dir: &Path) -> Result<Narc, KernelError>;

/// Packt jede Asset-Art, deren unpacked-Ordner existiert, an ihren gepackten Pfad und baut dann das ROM mit ds-rom.
pub fn build(project: &Project, out_rom: &Path) -> Result<BuildReport, KernelError>;

/// Extrahiert eine .nds in ein neues ds-rom-Projektverzeichnis (ds-rom `Rom::extract` + `Rom::save`).
pub fn extract_rom(nds: &Path, dest: &Path) -> Result<Project, KernelError>;
```

ds-rom-Bibliothek (0.8.0, `lib/src/rom/rom.rs`): `Rom::load(config_path, RomLoadOptions)` → `rom.build(None)` → `raw::Rom` → in Datei schreiben; `raw::Rom` aus Datei lesen (Konstruktor in `lib/src/rom/raw/`), `Rom::extract(&raw)` → `rom.save(dest, None)`. Fehler werden als `KernelError::Build(format!("{e}"))` gemeldet. Der `config_path` ist `root/config.yaml`.

- [ ] **Step 1: Failing Tests schreiben**

`tests/build.rs`:

```rust
mod common;
use g4_kernel::build::{build, pack_dir};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::error::KernelError;

#[test]
fn pack_dir_orders_members_numerically() {
    let d = tempfile::tempdir().unwrap();
    for (name, body) in [("0002", b"c"), ("0000", b"a"), ("0001", b"b")] {
        std::fs::write(d.path().join(name), body).unwrap();
    }
    let narc = pack_dir(d.path()).unwrap();
    assert_eq!(narc.members, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
}

#[test]
fn pack_dir_with_gap_is_error() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("0000"), b"a").unwrap();
    std::fs::write(d.path().join("0002"), b"c").unwrap();
    let err = pack_dir(d.path()).unwrap_err();
    assert!(matches!(err, KernelError::Build(ref m) if m.contains("0001")));
}

#[test]
fn build_repacks_unpacked_dirs_into_packed_paths() {
    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    p.ensure_unpacked(AssetKind::Map).unwrap();
    let edited = common::synthetic_map_bytes(0xEE);
    std::fs::write(p.member_path(AssetKind::Map, 1), &edited).unwrap();
    // ds-rom kann das synthetische Projekt nicht bauen (keine arm9); nur das Packen wird hier geprüft:
    let err = build(&p, &s.dir.path().join("out.nds")).unwrap_err();
    assert!(matches!(err, KernelError::Build(_)));
    let packed = std::fs::read(s.dir.path().join("files/fielddata/land_data/land_data.narc")).unwrap();
    let narc = g4_formats::narc::Narc::read(&packed).unwrap();
    assert_eq!(narc.members[1], edited);
}

#[test]
fn vanilla_build_produces_rom() {
    let Some(root) = common::vanilla_project() else { return };
    let p = Project::open(&root).unwrap();
    let out = tempfile::tempdir().unwrap().path().join("out.nds");
    let report = build(&p, &out).unwrap();
    assert!(report.rom.exists());
    assert!(std::fs::metadata(&report.rom).unwrap().len() > 16 * 1024 * 1024);
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel --test build`
Expected: Compile-Fehler.

- [ ] **Step 3: `build.rs` implementieren; `pub mod build;` in `lib.rs`**

Reihenfolge in `build`: erst alle vorhandenen unpacked-Ordner packen und schreiben (Map, Matrix, AreaData), dann ds-rom. Schlägt das Packen fehl, wird kein NARC geschrieben (erst alle packen, dann alle schreiben).

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-kernel --test build` und mit `G4_TEST_PROJECT`
Expected: alle `ok`; das Vanilla-ROM startet im Emulator (manuell prüfen).

- [ ] **Step 5: Commit**

```bash
git add crates/g4-kernel
git commit -m "feat(kernel): build pipeline packing NARCs and building the ROM with ds-rom"
```

---

### Task 8: CLI `g4k`

**Files:**
- Create: `crates/g4-kernel/src/bin/g4k.rs`, `tests/cli.rs`
- Modify: `crates/g4-kernel/Cargo.toml` (`clap`, dev `assert_cmd`, `predicates`)

**Interfaces:**
- Consumes: `Project`, `Store`, `build`, `extract_rom`.
- Produces Kommandos:
  - `g4k info <project>` druckt `game_code=<code> family=<Dp|Pt|Hgss> language=<..> headers=<n>`.
  - `g4k extract <rom.nds> <dest>` ruft `extract_rom`.
  - `g4k map dump <project> <index> --json` druckt `{"index":n,"buildings":[...],"permissions":[[kind,collision],...]}` (serde_json, 1024 Paare).
  - `g4k map set-perm <project> <index> <x> <y> --kind <u8> --collision <u8>` editiert über `Store` und speichert.
  - `g4k build <project> --out <rom.nds>` druckt `packed maps=<n> matrices=<n> areaData=<n>` und `rom=<pfad>`.
  Exit-Code 1 bei Fehler, Fehlertext auf stderr.

- [ ] **Step 1: Failing Tests schreiben**

`tests/cli.rs`:

```rust
mod common;
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn info_prints_identity() {
    let s = common::synthetic_project();
    Command::cargo_bin("g4k").unwrap().args(["info"]).arg(s.dir.path())
        .assert().success().stdout(contains("game_code=CPUE")).stdout(contains("family=Pt"));
}

#[test]
fn set_perm_then_dump_shows_change() {
    let s = common::synthetic_project();
    let root = s.dir.path();
    Command::cargo_bin("g4k").unwrap().args(["map", "set-perm"]).arg(root).args(["1", "3", "4", "--kind", "2", "--collision", "128"])
        .assert().success();
    let out = Command::cargo_bin("g4k").unwrap().args(["map", "dump"]).arg(root).args(["1", "--json"])
        .assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["permissions"][4 * 32 + 3], serde_json::json!([2, 128]));
}

#[test]
fn info_on_non_project_fails_with_message() {
    let d = tempfile::tempdir().unwrap();
    Command::cargo_bin("g4k").unwrap().args(["info"]).arg(d.path())
        .assert().failure().stderr(contains("not a ds-rom project"));
}
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel --test cli`
Expected: Fehler `no bin target named g4k`.

- [ ] **Step 3: `g4k.rs` mit `clap::Parser` implementieren (Subcommands `Info`, `Extract`, `Map { Dump, SetPerm }`, `Build`)**

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-kernel`
Expected: alle `ok`.

- [ ] **Step 5: Commit**

```bash
git add crates/g4-kernel
git commit -m "feat(kernel): g4k CLI for info, extract, map dump/set-perm and build"
```

---

### Task 9: Tauri-App mit Projekt-Öffnen und Header-Liste

**Files:**
- Create: `apps/studio/` (Scaffold), `apps/studio/src-tauri/src/state.rs`, `commands.rs`, `apps/studio/src/api.ts`, `src/App.tsx`, `vitest.config.ts`
- Modify: `Cargo.toml` (Member `apps/studio/src-tauri`), `apps/studio/src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: `Project`, `Store`, `HeaderSummary`, `Matrix`, `MapFile`.
- Produces Tauri-Commands (Rust, in `commands.rs`, alle `#[tauri::command]`, Fehler als `String`):

```rust
#[derive(serde::Serialize)] pub struct ProjectInfo { pub root: String, pub game_code: String, pub family: String, pub language: String, pub header_count: usize }
#[derive(serde::Serialize)] pub struct MatrixDto { pub index: u16, pub name: String, pub width: u8, pub height: u8, pub maps: Vec<Option<u16>> }
#[derive(serde::Serialize)] pub struct BuildingDto { pub model_id: u32, pub x: i16, pub y: i16, pub z: i16, pub width: u32, pub height: u32, pub length: u32 }
#[derive(serde::Serialize)] pub struct MapDto { pub index: u16, pub permissions: Vec<[u8; 2]>, pub buildings: Vec<BuildingDto> }

pub fn open_project(state, path: String) -> Result<ProjectInfo, String>;
pub fn list_headers(state) -> Result<Vec<HeaderSummary>, String>;
pub fn get_matrix(state, index: u16) -> Result<MatrixDto, String>;
pub fn get_map(state, index: u16) -> Result<MapDto, String>;
```

`state.rs`: `pub struct AppState(pub Mutex<Option<Session>>)`, `pub struct Session { pub project: Arc<Project>, pub store: Store }`. `BuildingDto.x` ist `building.x as i16` (vorzeichenbehaftete Kachel-Einheit, Ursprung in der Kartenmitte; gegen DSPREs `MapEditor` prüfen, siehe Review).

`src/api.ts` exportiert `openProject(path): Promise<ProjectInfo>`, `listHeaders(): Promise<HeaderSummary[]>`, `getMatrix(index): Promise<MatrixDto>`, `getMap(index): Promise<MapDto>` mit gleichnamigen TypeScript-Typen.

- [ ] **Step 1: Scaffold erzeugen**

Run: `cd $HOME/Projekte/lab/editor/gen4-studio/apps && pnpm create tauri-app@4.7.5 studio --template react-ts --manager pnpm --yes && cd studio && pnpm install && pnpm add @tauri-apps/plugin-dialog three && pnpm add -D vitest @types/three`
Expected: `apps/studio/src-tauri` existiert, `pnpm tauri --version` druckt `2.12.x`.

- [ ] **Step 2: Workspace verbinden**

`apps/studio/src-tauri/Cargo.toml`: `g4-kernel = { path = "../../../crates/g4-kernel" }`, `tauri-plugin-dialog = "2"`, `serde`, `serde_json`; `edition.workspace = true`. Die vom Scaffold erzeugte `[workspace]`-Sektion in `src-tauri/Cargo.toml` entfernen (das Root ist der Workspace). `capabilities/default.json`: `dialog:default` ergänzen.

Run: `cargo build -p studio` (Paketname wie vom Scaffold vergeben, in `src-tauri/Cargo.toml` nachsehen)
Expected: baut.

- [ ] **Step 3: Failing Rust-Test für DTO-Mapping schreiben**

In `commands.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn map_dto_has_1024_permission_pairs_and_signed_building_coords() {
        let mut map = g4_formats::map::MapFile { bgs: None, permissions: vec![Default::default(); 1024], buildings: vec![], model: vec![], bdhc: vec![] };
        map.buildings.push(g4_formats::map::Building { model_id: 1, x_frac: 0, x: 0xFFF0, z_frac: 0, z: 2, y_frac: 0, y: 0, unknown_10: [0; 13], width: 16, height: 16, length: 16, tail: [0; 7] });
        let dto = MapDto::from_map(3, &map);
        assert_eq!(dto.permissions.len(), 1024);
        assert_eq!(dto.buildings[0].x, -16);
        assert_eq!(dto.buildings[0].z, 2);
    }
}
```

- [ ] **Step 4: Test laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p studio`
Expected: Compile-Fehler `MapDto`.

- [ ] **Step 5: `state.rs`, `commands.rs` (mit `MapDto::from_map(index: u16, map: &MapFile) -> MapDto`, `MatrixDto::from_matrix`), Registrierung in `lib.rs` (`.manage(AppState::default())`, `.invoke_handler(tauri::generate_handler![open_project, list_headers, get_matrix, get_map])`, `.plugin(tauri_plugin_dialog::init())`) implementieren**

- [ ] **Step 6: Test laufen lassen**

Run: `cargo test -p studio`
Expected: `1 passed`.

- [ ] **Step 7: Failing vitest-Test für `api.ts`-Typen und einen reinen Helfer schreiben**

`src/world/colors.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { permissionColor } from "./colors";

describe("permissionColor", () => {
  it("marks blocked cells red regardless of kind", () => {
    expect(permissionColor(0, 0x80)).toBe("#d64545");
    expect(permissionColor(5, 0x80)).toBe("#d64545");
  });
  it("colors walkable by kind family", () => {
    expect(permissionColor(0, 0)).toBe("#6b7280");
    expect(permissionColor(0x02, 0)).toBe("#4caf50");   // Gras
    expect(permissionColor(0x10, 0)).toBe("#3b82f6");   // Wasser
  });
});
```

- [ ] **Step 8: Test laufen lassen, Fehlschlag prüfen**

Run: `cd apps/studio && pnpm vitest run`
Expected: FAIL `Cannot find module ./colors`.

- [ ] **Step 9: `src/world/colors.ts` (`export function permissionColor(kind: number, collision: number): string`; Zuordnung: Kollision 0x80 → `#d64545`; kind 0x02..0x07 → `#4caf50`; kind 0x10..0x1F → `#3b82f6`; sonst `#6b7280`), `src/api.ts`, `src/App.tsx` (Button "Projekt öffnen" über `open({ directory: true })` aus `@tauri-apps/plugin-dialog`, danach Liste `id  internal_name  matrix` aus `listHeaders()`) implementieren**

- [ ] **Step 10: Tests und App prüfen**

Run: `pnpm vitest run` (Expected: `2 passed`), dann `pnpm tauri dev`.
Expected: Fenster öffnet; nach Auswahl eines ds-rom-Projekts erscheint die Header-Liste mit Namen wie `T01` oder `NEW BARK TOWN`-artigen internen Namen; Konsole ohne Fehler.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml apps/studio
git commit -m "feat(studio): tauri shell with project open, header list and DTO commands"
```

---

### Task 10: Matrix-Ansicht in three.js mit Modellen über apicula

**Files:**
- Create: `crates/g4-kernel/src/export.rs`, `crates/g4-kernel/tests/export.rs`, `apps/studio/src/world/layout.ts`, `layout.test.ts`, `MatrixScene.tsx`
- Modify: `apps/studio/src-tauri/src/commands.rs` (`get_map_model_glb`), `src/api.ts`, `src/App.tsx`

**Interfaces:**
- Produces Kernel:

```rust
/// Schreibt `nsbmd` als Temp-Datei und ruft `apicula convert -f=glb <file> -o <out_dir>`; liefert den Pfad der erzeugten .glb.
pub fn nsbmd_to_glb(apicula: &Path, nsbmd: &[u8], out_dir: &Path) -> Result<PathBuf, KernelError>;
```

Command `get_map_model_glb(state, index: u16) -> Result<String, String>`: Cache unter `<project>/.g4/models/<index>.glb`; liefert den Pfad; apicula-Pfad aus `G4_APICULA` oder `tools/apicula` neben dem Studio-Binary. Fehlt apicula, Fehler `tool apicula unavailable`.

- Produces Frontend (`layout.ts`):

```ts
export const MAP_SIZE = 32;
export function cellOrigin(col: number, row: number): { x: number; z: number };     // { x: col*32, z: row*32 }
export function buildingBox(b: BuildingDto, cell: { x: number; z: number }): { cx: number; cy: number; cz: number; sx: number; sy: number; sz: number };
```

`buildingBox`: Mitte `cx = cell.x + 16 + b.x`, `cz = cell.z + 16 + b.z`, `cy = b.y + sy/2`, Größe `sx = b.width/16`, `sy = b.height/16`, `sz = b.length/16`.

- [ ] **Step 1: Failing Tests schreiben**

`crates/g4-kernel/tests/export.rs`:

```rust
use g4_kernel::export::nsbmd_to_glb;
use g4_kernel::error::KernelError;

#[test]
fn missing_apicula_is_tool_missing() {
    let out = tempfile::tempdir().unwrap();
    let err = nsbmd_to_glb(std::path::Path::new("/nonexistent/apicula"), b"BMD0", out.path()).unwrap_err();
    assert!(matches!(err, KernelError::ToolMissing { name: "apicula", .. }));
}

#[test]
fn vanilla_map_model_converts_to_glb() {
    let (Some(root), Some(apicula)) = (std::env::var_os("G4_TEST_PROJECT"), std::env::var_os("G4_APICULA")) else { eprintln!("skipped"); return };
    let p = g4_kernel::project::Project::open(root).unwrap();
    p.ensure_unpacked(g4_kernel::project::AssetKind::Map).unwrap();
    let bytes = std::fs::read(p.member_path(g4_kernel::project::AssetKind::Map, 0)).unwrap();
    let map = g4_formats::map::MapFile::read(&bytes, p.family).unwrap();
    let out = tempfile::tempdir().unwrap();
    let glb = nsbmd_to_glb(std::path::Path::new(&apicula), &map.model, out.path()).unwrap();
    assert_eq!(&std::fs::read(&glb).unwrap()[0..4], b"glTF");
}
```

`apps/studio/src/world/layout.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { cellOrigin, buildingBox } from "./layout";

describe("layout", () => {
  it("places matrix cells 32 units apart", () => {
    expect(cellOrigin(0, 0)).toEqual({ x: 0, z: 0 });
    expect(cellOrigin(2, 1)).toEqual({ x: 64, z: 32 });
  });
  it("centers a building relative to the cell centre and scales by 1/16", () => {
    const box = buildingBox({ model_id: 1, x: -4, y: 0, z: 6, width: 32, height: 16, length: 48 }, { x: 64, z: 32 });
    expect(box).toEqual({ cx: 76, cy: 0.5, cz: 54, sx: 2, sy: 1, sz: 3 });
  });
});
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p g4-kernel --test export` und `pnpm vitest run`
Expected: Compile-Fehler bzw. `Cannot find module ./layout`.

- [ ] **Step 3: `export.rs` (`pub mod export;`), Command `get_map_model_glb`, `layout.ts` implementieren**

`nsbmd_to_glb` prüft `apicula.exists()` vor dem Aufruf; nicht-null Exit-Code wird `KernelError::Tool`-artig als `ToolMissing { name: "apicula", reason: stderr }` gemeldet.

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p g4-kernel --test export` (mit `G4_TEST_PROJECT` und `G4_APICULA`) und `pnpm vitest run`
Expected: alle `ok`.

- [ ] **Step 5: `MatrixScene.tsx` implementieren**

three.js-Szene (`WebGLRenderer`, `PerspectiveCamera`, `OrbitControls` aus `three/examples/jsm/controls/OrbitControls.js`): pro belegter Matrixzelle ein 32×32-Gitter aus `PlaneGeometry`-Instanzen (`InstancedMesh`, Farbe aus `permissionColor`), halbtransparent über dem GLB-Modell (`GLTFLoader`, Pfad über `convertFileSrc` aus `@tauri-apps/api/core`), Gebäude als `BoxGeometry` aus `buildingBox`. `App.tsx`: Klick auf einen Header lädt `getMatrix(header.matrix_id)` und rendert die Szene; Umschalter "Permissions anzeigen".

- [ ] **Step 6: App prüfen**

Run: `G4_APICULA=<pfad> pnpm tauri dev`
Expected: Nach Auswahl eines Headers erscheint die Matrix mit Modellen, Gebäude-Boxen sitzen auf den Häusern (sonst Vorzeichen oder Ursprung in `BuildingDto`/`buildingBox` korrigieren und den Test anpassen), Permissions-Overlay zeigt rote Kollisionen an Hauswänden.

- [ ] **Step 7: Commit**

```bash
git add crates/g4-kernel apps/studio
git commit -m "feat(studio): three.js matrix view with permissions overlay, buildings and apicula GLB models"
```

---

### Task 11: Permissions malen, speichern, bauen, starten

**Files:**
- Create: `crates/g4-kernel/src/run.rs`, `apps/studio/src/world/paint.ts`, `paint.test.ts`
- Modify: `crates/g4-kernel/src/lib.rs`, `apps/studio/src-tauri/src/commands.rs`, `src/api.ts`, `src/App.tsx`, `src/world/MatrixScene.tsx`, `src-tauri/capabilities/default.json`

**Interfaces:**
- Produces Kernel:

```rust
/// Startet den Emulator mit dem ROM als Argument, ohne zu warten; liefert die PID.
pub fn launch_emulator(emulator: &Path, rom: &Path) -> Result<u32, KernelError>;
```

- Produces Commands:
  - `set_permission(state, index: u16, x: u8, y: u8, kind: u8, collision: u8) -> Result<(), String>` (über `Store::edit_map`, Label `"paint"`).
  - `undo(state) -> Result<bool, String>`, `redo(state) -> Result<bool, String>`.
  - `dirty(state) -> Result<Vec<AssetId>, String>`.
  - `save(state) -> Result<SaveReport, String>`.
  - `build_rom(state, out: String) -> Result<BuildReport, String>`.
  - `run_rom(emulator: String, rom: String) -> Result<u32, String>`.

- Produces Frontend (`paint.ts`): `export function pickCell(hit: { x: number; z: number }, cell: { x: number; z: number }): { x: number; y: number } | null` (Weltkoordinate auf Zellenkoordinate 0..31 abbilden; außerhalb `null`).

- [ ] **Step 1: Failing Tests schreiben**

In `commands.rs` (Rust):

```rust
#[test]
fn set_permission_rejects_out_of_range_without_panicking() {
    let s = /* synthetisches Projekt wie in g4-kernel tests/common; Helfer als pub fn in state.rs unter #[cfg(test)] */;
    let state = AppState::from_session(s);
    assert!(set_permission_impl(&state, 0, 32, 0, 0, 0x80).is_err());
    assert!(set_permission_impl(&state, 0, 3, 3, 0, 0x80).is_ok());
    assert_eq!(dirty_impl(&state).unwrap().len(), 1);
}
```

(`set_permission_impl` und `dirty_impl` sind die testbaren Funktionen hinter den `#[tauri::command]`-Wrappern, die nur `State<AppState>` entpacken.)

`paint.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { pickCell } from "./paint";

describe("pickCell", () => {
  it("maps a world hit to the cell's tile coordinates", () => {
    expect(pickCell({ x: 64.5, z: 33.2 }, { x: 64, z: 32 })).toEqual({ x: 0, y: 1 });
    expect(pickCell({ x: 95.9, z: 63.9 }, { x: 64, z: 32 })).toEqual({ x: 31, y: 31 });
  });
  it("returns null outside the cell", () => {
    expect(pickCell({ x: 96.1, z: 40 }, { x: 64, z: 32 })).toBeNull();
    expect(pickCell({ x: 63.9, z: 40 }, { x: 64, z: 32 })).toBeNull();
  });
});
```

- [ ] **Step 2: Tests laufen lassen, Fehlschlag prüfen**

Run: `cargo test -p studio` und `pnpm vitest run`
Expected: Compile-Fehler bzw. `Cannot find module ./paint`.

- [ ] **Step 3: `run.rs` (`pub mod run;`), Commands, `paint.ts` implementieren; `shell:allow-spawn` ist nicht nötig, da der Kernel selbst `std::process::Command` nutzt**

- [ ] **Step 4: Tests laufen lassen**

Run: `cargo test -p studio && cd apps/studio && pnpm vitest run`
Expected: alle `ok`.

- [ ] **Step 5: UI verdrahten**

`MatrixScene.tsx`: Raycast auf Klick, `pickCell`, Linksklick setzt `collision: 0x80`, Shift-Klick setzt `0x00`, danach `set_permission` und lokale Farbe aktualisieren. `App.tsx`: Toolbar mit Undo (`Cmd/Ctrl+Z`), Redo, Save (`Cmd/Ctrl+S`, zeigt `SaveReport.written.length`), Build (Zielpfad über `save({ filters: [{ name: "ROM", extensions: ["nds"] }] })` aus dem Dialog-Plugin), Run (Emulator-Pfad in `localStorage` unter `g4.emulatorPath`, beim ersten Mal über `open()` wählen). Fehler aus Commands erscheinen in einer nicht-modalen Leiste unten, nicht als `alert`.

- [ ] **Step 6: End-to-End prüfen (Gate der Phase 1)**

Run: `pnpm tauri dev`
Ablauf: HGSS-Projekt öffnen, Header von New Bark Town wählen, vor der Haustür eine Kachel auf Kollision setzen, speichern (Leiste zeigt `1 Datei gespeichert`), Build nach `out.nds`, Run mit melonDS.
Expected: Im Emulator ist die Kachel blockiert. Dasselbe mit einem Platin-Projekt. `cargo test --workspace` und `pnpm vitest run` grün; mit `G4_TEST_PROJECT` grün.

- [ ] **Step 7: Commit und Tag**

```bash
git add crates/g4-kernel apps/studio
git commit -m "feat(studio): permission painting with undo, save, build and emulator launch"
git tag phase-1
```

---

## Self-Review

**Spec-Abdeckung Phase 1:** Projekt öffnen (Task 5, 9), DSPRE-kompatibles Layout (Task 5), Codecs mit Round-Trip (Tasks 2 bis 4), Store ohne globalen Zustand (Task 6), Build über ds-rom (Task 7), headless CLI (Task 8), three.js-Matrix mit apicula (Task 10), Edit-Build-Play (Task 11). Nicht in Phase 1, bewusst: Header schreiben, Events, Scripts, Text (Phasen 2 und 3).

**Typkonsistenz:** `AssetKind` wird in Task 5 definiert und in 6, 7, 8, 9, 11 verwendet; `AssetId` in Task 6 und 11; `HeaderSummary` in 5 und 9; `MapDto`/`BuildingDto` in 9 und 10; `Narc::read/write` in 2, 5, 7; `MapFile::read/write/cell/set_cell` in 3, 6, 9, 11.

**Review Focus:** Punkt 1 (Task 3), 2 (Task 4), 3 (Task 7), 4 (Task 6), 5 (Task 3 und 11) sind mit Tests belegt.

**Offene Annahmen, die ein Test entscheidet:** NARC-Padding-Byte (Task 2), Gebäudekoordinaten-Konvention (Task 9 und 10), uxie-Header-Abdeckung für HGSS (Task 5).
