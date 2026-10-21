# Gen4 Studio: Design-Spezifikation

Stand: 2026-10-21. Arbeitstitel `gen4-studio` (Ordner- und Crate-Präfix `g4`; ein Rename ist ein `sed`). Grundlage: [Analyse von DSPRE und der Suite](../../research/2026-10-20-dspre-und-suite-analyse.md) vom 2026-10-20 und die darauf folgenden Entscheidungen.

## 1. Ziel

Ein eigenständiger, schlanker, moderner Desktop-Editor für Pokémon-Gen-4-Welten (Diamant, Perl, Platin, HeartGold, SoulSilver), der auf der vorhandenen Rust-Basis des Ökosystems aufsetzt (ds-rom, uxie, rotom, chatot, apicula), unabhängig vom DSPRE-Projekt entwickelt wird und langfristig nach dem Prinzip "Everything as a Plugin" erweiterbar ist.

Das Produkt ist eine **Authoring-Engine mit Build-Pipeline**: Quelle ist ein Projektordner, Ziel ist ein ROM (oder ein xdelta-Patch), das der Nutzer aus seinem eigenen Basis-ROM baut.

## 2. Nicht-Ziele

- Keine Laufzeit-Engine, kein PC-Port. Das Spiel läuft auf DS-Hardware oder im Emulator.
- Keine Auslieferung von ROM-abgeleiteten Assets. Der Nutzer liefert das Basis-ROM.
- Keine Abhängigkeit von `DSPRE.Core` (AGPL-3.0, keine stabile API, keine Paketveröffentlichung). DSPRE dient als Referenz für Formatwissen und als Testorakel (byte-identischer Output), nicht als Bibliothek. Code wird nicht portiert, sondern aus Formatbeschreibungen neu geschrieben.
- Kein Nachbau aller rund 100 DSPRE-Editoren. Der Scope ist der Welt-Editor plus die Daten, die eine Welt braucht.

## 3. Entscheidungen

| Frage | Entscheidung | Begründung |
|---|---|---|
| Kernel-Sprache | Rust | MIT-lizenzierte Basis (uxie, rotom, chatot, ds-rom), spätere WASM-Plugins (Zed-Modell), headless als CLI nutzbar |
| UI-Shell | Tauri 2 mit React, TypeScript, three.js | Web-Stack für UX-Geschwindigkeit (Pokémon-Studio-Niveau), macOS und Windows, LSP-Clients für Monaco fertig |
| Lizenz des Editors | MIT | Nur möglich, weil keine AGPL-Abhängigkeit; nitroarc (LGPL) wird als separate Shared Library ausgeliefert |
| DSPRE-Projektformat | Kompatibel lesen und schreiben (ds-rom-Layout plus `unpacked/<Kind>/NNNN`) | Bestehende Projekte öffnen, DSPRE parallel nutzbar, Testorakel |
| Script-Sprache | rotom als kanonische Form | Compiler, Decompiler, LSP vorhanden |
| 3D-Import | apicula als Sidecar-CLI (NSBMD zu glTF/GLB) | apicula ist nur Binary-Crate (glium-Abhängigkeit), 0BSD |
| NARC | eigener Reader/Writer in `g4-formats` | uxie/nitroarc kann NARCs nicht erzeugen; Format ist öffentlich dokumentiert |
| Plugin-Modell | dreistufig: WASM-Komponenten (Kernel), JS-Module (Frontend), Out-of-Process-Tools | Isolation ohne Prozessabsturz, Lizenzgrenze, Rust-Tools unverändert |
| Dateisystem der Daten | Phase 1 bis 3: DSPRE-Layout. Ab Phase 4: Textprojekt (eine Datei pro Entität, symbolische IDs) mit Build zum ROM | Erst Nutzen, dann Format |

## 4. Architektur

```
apps/studio            Tauri 2 (React, TypeScript, three.js, Monaco + rotom-lsp)
   │ Tauri-Commands / Events (JSON)
crates/g4-kernel       Projekt, Store (Changesets, Undo), Build, Export, CLI `g4k`
crates/g4-formats      NARC, Map, Matrix, AreaData, (später BDHC, NSBMD-Writer)
externe Crates         uxie (Header, Events, Trainer, Personal, ...), ds-rom (ROM), chatot (Text), rotom (Scripts)
Sidecars               apicula (3D-Export), rotom-lsp (Sprachserver), Emulator
```

### 4.1 Kernel

- `Project`: Wurzel, Spielkennung (Game-Code, Familie, Sprache), Archivzuordnung (gepackter NARC-Pfad und entpackter Ordner je Asset-Art), Header-Tabelle über uxie.
- `Store`: In-Memory-Assets pro `AssetId { kind, index }`, Änderungen als Changesets mit Vorher/Nachher-Bytes, Undo/Redo, Dirty-Erkennung durch Byte-Vergleich, `save()` schreibt nur geänderte Member in den entpackten Ordner.
- `build`: packt entpackte Ordner zu NARCs, schreibt sie an den gepackten Pfad, baut das ROM über die ds-rom-Bibliothek. Jeder Schritt meldet Diagnostics statt Dialoge.
- Kein globaler Zustand. Ein `Project` ist ein Wert; mehrere Projekte pro Prozess sind erlaubt.
- Keine UI-Begriffe im Kernel. Keine blockierenden Rückfragen; Funktionen geben `Result` mit typisierten Fehlern zurück.

### 4.2 Formate

Ein Codec pro Format mit `read(bytes, family) -> Result<T>` und `write(&T, family) -> Vec<u8>`. Jeder Codec hat einen synthetischen Round-Trip-Test und einen gegen Vanilla-Daten gegateten byte-identischen Round-Trip-Test. Unbekannte Bytes werden verbatim erhalten, nie "normalisiert".

### 4.3 Shell

- Eine Workbench: Projektbaum links, Editor-Tabs mittig, Inspector rechts, Problems/Build unten (Dockview).
- Welt-Ansicht: ganze Matrix eines Headers in three.js, Modi Permissions, Gebäude, Events, Warps.
- Datenbank-Editoren nach Pokémon-Studio-Muster (Liste plus Detail, Referenzzähler, Querverlinkung).
- Events zweigleisig: visuell und rotom-Text, verlustfrei umschaltbar.
- Build und Run: Build-Button, strukturierte Diagnostics, Emulator-Start.
- Designregeln: eine Toolbar, ein ID-Picker, ein Checkbox-Stil, Sentence Case, Light und Dark.

### 4.4 Plugins (ab Phase 5)

Contribution Points: backends, assetKinds/codecs, schemas, metadata (Offsets, Enums), editors, inspectors, docks, commands/menus/keybindings, tools (Welt-Editor), importers/exporters, scriptCommands, patches, buildSteps, languages, settings. First-Party-Editoren nutzen dieselbe API (Dogfooding).

## 5. Scope

**v1 (Phasen 1 bis 3):** DSPRE-kompatibles Projekt öffnen oder ROM extrahieren; Header, Matrizen, Karten-Permissions und Gebäude, Area-Data, Events, Scripts (rotom), Encounter, Trainer, Text, Pokémon-Basisdaten bearbeiten; Build zu ROM; Emulator starten. Kartenerstellung über Import (PDSMS-Dateien, OBJ über apicula-Pfad).

**v2 (Phase 4):** Textprojekt als Quelle, inkrementeller Build, xdelta-Export, Git-freundlich.

**v3 (Phasen 5 und 6):** Plugin-Host, Kartenerstellung im Editor (Tile-Baking zu NSBMD, BDHC), textuelles Weltformat.

## 6. Globale Constraints

- Rust stable, Edition 2024 (uxie und rotom verlangen sie). `rust-toolchain.toml` pinnt den Kanal.
- Node 24, pnpm 11. Tauri 2.12, `@tauri-apps/api` 2.12, create-tauri-app 4.7.
- Crates: `uxie = "0.9"`, `ds-rom = "0.8"`, `binrw = "0.15"`, `thiserror = "2"`, `serde_json = "1"`. rotom als Git-Abhängigkeit (`https://github.com/KalaayPT/rotom`, Tag `v0.7.0`); das Crate `rotom` auf crates.io ist ein anderes Projekt.
- Lizenz MIT für alle eigenen Crates und Apps. Keine GPL- oder AGPL-Abhängigkeit im Prozess. LGPL (nitroarc) nur als separate Shared Library.
- Keine ROM-Daten im Repository. Tests gegen Vanilla-Daten lesen den Pfad aus der Umgebungsvariable `G4_TEST_PROJECT` (ein extrahiertes ds-rom-Projekt mit `unpacked/`) und überspringen sich, wenn sie fehlt.
- DSPRE-Dateinamen beibehalten: entpackte Member heißen `NNNN` (vierstellig, führende Nullen), Ordner `unpacked/maps`, `unpacked/matrices`, `unpacked/areaData`, `unpacked/eventFiles`.
- Bezeichner im Code Englisch, Dokumentation Deutsch, Commit-Messages Englisch (Conventional Commits).
- Jeder Codec: unbekannte Bytes erhalten, byte-identischer Round-Trip gegen Vanilla.

## 7. Risiken und offene Fragen

- uxie-HGSS-Abdeckung ist laut README "partial in some workflows"; Header-Tabellen-Offsets für alle Dumps müssen geprüft werden.
- Rotation der Gebäude liegt in den 13 Bytes ab Offset 0x10 des Gebäudeeintrags; Phase 1 erhält sie verbatim, Phase 3 dekodiert sie.
- NARC-Padding-Byte (0x00 oder 0xFF) wird durch den Vanilla-Round-Trip-Test festgelegt.
- Tile-Baking zu NSBMD ist der längste Posten; Formatfakten aus apicula-Dokumentation und Community-Wikis, nicht aus DSPRE-Code.
- Busfaktor des Ökosystems: uxie und rotom werden von einer Person gepflegt. Forks sind eingeplant (MIT).

## 8. Referenzen

- DSPRE `main`: `DS_Map/ROMFiles/MapFile.cs` (Kartenformat), `GameMatrix.cs` (Matrixformat), `AreaData.cs`, `RomInfo.cs` (NARC-Pfade je Familie, `SetNarcDirs`).
- uxie 0.9.7: `src/workspace.rs` (`Workspace::open`), `src/ds_rom.rs` (`DspreProject`, Ordnernamen), `src/provider.rs` (`DataProvider`, `Arm9Provider`), `src/map_header/binary.rs`, `src/event_file/binary.rs`, `src/narc.rs`.
- ds-rom 0.8.0: `lib/src/rom/rom.rs` (`Rom::extract`, `Rom::save`, `Rom::load`, `Rom::build`).
- apicula: `apicula convert -f=glb <NITRO FILES> -o <OUTPUT DIR>`.
- rotom 0.7.0: `src/lib.rs` (`project_compile`, `compile_batch`, `DatabaseV2`).
