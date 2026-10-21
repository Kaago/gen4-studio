# Gen4 Studio: Roadmap

Stand: 2026-10-21. Spezifikation: [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](superpowers/specs/2026-10-21-gen4-studio-design.md). Analyse: [docs/research/2026-10-20-dspre-und-suite-analyse.md](research/2026-10-20-dspre-und-suite-analyse.md).

Jede Phase endet mit lauffähiger, testbarer Software und einem Gate. Keine Phase beginnt, bevor das Gate der vorherigen bestanden ist. Zeitangaben sind bewusst weggelassen; die Reihenfolge und die Gates sind die Zusage.

## Übersicht

| Phase | Ergebnis | Gate |
|---|---|---|
| 0 | Entscheidungsgrundlage (erledigt) | Analyse, Spec, Roadmap liegen vor |
| 1 | Kernel-Fundament und Shell-Spike: Projekt öffnen, Permissions malen, speichern, ROM bauen, im Emulator laufen | Edit-Build-Play-Schleife funktioniert auf einem HGSS- und einem Platin-Projekt |
| 2 | Welt-Editor v1: Header, Matrix, Gebäude, Events, Warps, 3D-Modelle | Eine neue Route aus bestehenden Karten zusammensetzen und spielen |
| 3 | Daten-Editoren v1: Scripts (rotom), Encounter, Trainer, Text, Pokémon-Basisdaten; Kartenimport | Ein kleiner Hack (neue Route, neue Trainer, neue Texte, ein Script) komplett im Studio |
| 4 | Textprojekt und Build-Graph | Projekt in Git, inkrementeller Build, xdelta-Export, byte-identischer Rebuild eines unveränderten Projekts |
| 5 | Plugin-Host | Ein First-Party-Editor und ein Beispiel-Plugin laufen über dieselbe API |
| 6 | Kartenerstellung und Weltformat | Karte im Studio malen, zu NSBMD/BDHC bauen, spielen |

## Phase 1: Kernel-Fundament und Shell-Spike

Detailplan: [docs/superpowers/plans/2026-10-21-phase-1-kernel-and-shell.md](superpowers/plans/2026-10-21-phase-1-kernel-and-shell.md).

Lieferumfang:
- Cargo-Workspace mit `g4-formats` (NARC, Map, Matrix, AreaData) und `g4-kernel` (Project, Store, Build, CLI `g4k`).
- Tauri-App `apps/studio`: Projekt öffnen, Header-Liste, Matrix-Ansicht in three.js mit Permissions-Overlay und Gebäude-Boxen, NSBMD-Modelle über apicula als GLB.
- Permissions malen, speichern, ROM bauen, Emulator starten.

Gate: Auf einem HGSS- und einem Platin-Projekt wird eine Kollision geändert, das ROM gebaut und der Effekt im Emulator gezeigt. Alle Vanilla-Round-Trip-Tests laufen byte-identisch.

Risiken: uxie-Header-Offsets für HGSS; NARC-Padding; Gebäudekoordinaten-Konvention.

## Phase 2: Welt-Editor v1

Eingangskriterien aus dem Abschluss-Review von Phase 1:
- Stale-Fall auflösbar machen: einzelne Assets verwerfen oder neu laden, Meldung in Klartext statt `AssetId { … }`.
- App-Bundle lauffähig machen: `libnitroarc_ffi.dylib` und apicula mit ausliefern; `pnpm tauri build` auf einem sauberen Rechner prüfen.
- Cmd+Q-Rückfrage prüfen; falls sie fehlt, Exit-Guard auf Rust-Seite (`RunEvent::ExitRequested`).
- Kernel aufräumen: `Store` entpackt bei Bedarf selbst, Edit-Closures dürfen fehlschlagen, Fehler tragen Pfad und Asset, eigener Fehlertyp für Projekt-Metadaten statt `Build`.
- `MapFile::validate` lehnt `buildings_tail` ab 48 Bytes ab, bevor Gebäude editierbar werden.
- Model-Cache über einen Hash der NSBMD-Bytes statt über mtimes; apicula mit Timeout und in `spawn_blocking`.
- Gated Vanilla-Tests in CI gegen mindestens ein Platin- und ein HGSS-Projekt.
- Matrix-Dimensionen prüfen: Laut pokeplatinum ist Byte 0 die Höhe und Byte 1 die Breite (DSPRE benennt es umgekehrt). Mit einer nicht quadratischen Matrix testen, bevor die 3D-Ansicht Karten anordnet.
- Orientierung im Welt-Editor: Kachelkoordinaten und Kacheltyp beim Überfahren anzeigen, Warps und Events markieren, Map-Texturen laden. Ohne diese Hilfen ließ sich im GUI-Test die Treppe nicht finden.
- Header-Liste lesbar machen: Anzeigenamen statt interner Kürzel (T01R0102 ist das Haus des Rivalen, T01R0202 das des Spielers), keine Zeilenumbrüche.
- Große Matrizen (Matrix 0, 30 × 30) laden: Modelle nur im sichtbaren Bereich konvertieren, Ladezeit messen.


Lieferumfang:
- Header-Editor (uxie `MapHeader` lesen und schreiben, Header-Tabelle in `arm9.bin` über Offset-Metadaten je Dump).
- Matrix-Editor: Karten zuweisen, Header-Zellen, Höhen; neue Matrix anlegen.
- Gebäude: Position, Skalierung, Rotation (Dekodierung der 13 Bytes ab 0x10), Modellwahl aus dem Gebäude-NARC, Vorschau als GLB.
- Events: uxie `EventFile` (Objekte, Warps, Koordinaten-Trigger, Hintergrund-Events) platzieren und verschieben in der 3D-Ansicht; Warps per Drag verbinden.
- Area-Data-Editor.
- Workbench mit Dockview: Projektbaum, Tabs, Inspector, Problems.
- Undo/Redo über alle Asset-Arten; "Open in ..."-Querverlinkung.

Gate: Eine neue Route aus bestehenden Kartendateien in einer neuen Matrix, mit Warps zu einer bestehenden Stadt, ist im Emulator begehbar.

## Phase 3: Daten-Editoren v1 und Kartenimport

Lieferumfang:
- Scripts: rotom-Projekt (`rotom.toml`, `.rotom/command_database/`) anlegen oder übernehmen; Monaco mit rotom-lsp; Compile beim Build; Decompile beim ersten Öffnen.
- Text: chatot-Pipeline (JSON im Projekt), Text-Editor mit Suche, Referenzen aus Scripts.
- Encounter (DPPt und HGSS, eigener Writer in `g4-formats`), Trainer (uxie), Pokémon-Basisdaten, Moves, Items (uxie).
- Datenbank-UI nach Pokémon-Studio-Muster; Referenzzähler.
- Kartenimport: PDSMS `.pdsmap`/`.pdsts`, NSBMD-Dateien, OBJ über apicula-Pfad; Kollisionsimport.
- Project Checks: kaputte Warps, Scripts ohne Ziel, Trainer ohne Party, Textlängen.

Gate: Ein kleiner Hack (neue Route, zwei neue Trainer, neue Texte, ein Ereignis-Script) wird komplett im Studio gebaut und gespielt.

## Phase 4: Textprojekt und Build-Graph

Lieferumfang:
- Projektformat: eine JSON-Datei pro Entität, symbolische IDs (`MAP_NEW_BARK_TOWN`, `SPECIES_PIKACHU`), Schema-Version und Migrationen.
- Export aus dem DSPRE-Layout in das Textprojekt und zurück; byte-identischer Rebuild eines unveränderten Projekts als Test.
- Build-Graph mit Fingerprints: nur geänderte Schritte laufen (rotom, chatot, NARC-Packen, ds-rom).
- xdelta-Export; Basis-ROM-Prüfung per SHA-1.
- Git-Integration minimal: `.gitignore`-Vorlage, Diff-Ansicht pro Entität.

Gate: Projekt liegt in Git, `g4k build` erzeugt aus dem Textprojekt ein ROM, ein unverändertes Projekt rebuildet byte-identisch.

## Phase 5: Plugin-Host

Lieferumfang:
- `g4-plugin-api` (WIT-Definitionen) und Wasmtime-Host im Kernel: Codecs, Build-Steps, Checks als WASM-Komponenten.
- Frontend-Contribution-Registry: commands, menus, docks, inspectors, editors als JS-Module mit Manifest (`plugin.json`: id, semver, engine-Bereich, activationEvents, contributes).
- Settings-Namespaces, Activation Events, Entladen ohne Neustart.
- Erster First-Party-Editor (Area-Data) ausschließlich über die Plugin-API; Beispiel-Plugin "Encounter-Tabelle als CSV exportieren".

Gate: Beispiel-Plugin wird aus einem Ordner geladen, trägt Command, Dock und Build-Step bei, und ein Absturz im Plugin lässt den Host laufen.

## Phase 6: Kartenerstellung und Weltformat

Lieferumfang:
- Tile-Malen (Tileset aus NSBTX, Smart-Drawing) und Baking zu NSBMD; BDHC-Erzeugung aus Höhenkarte; Kollisionsableitung.
- NSBMD/NSBTX-Writer in `g4-formats`, Round-Trip gegen apicula-Lesepfad und Vanilla.
- Textuelles Weltformat: Karte als JSON (Tiles, Höhen, Gebäude, Kollision) plus glTF für Freiform-Geometrie; Compiler zu Kartendatei.
- 2D-Top-Down-Modus für schnelles Arbeiten.

Gate: Eine im Studio gemalte Karte wird gebaut, in eine Matrix gesetzt und im Emulator begangen.

## Querschnitt (laufend ab Phase 1)

- CI: `cargo test`, `cargo clippy -D warnings`, `pnpm test`, Builds für macOS (arm64) und Windows (x64); Linux als Best-Effort.
- Releases: Tauri-Bundles, signiert, mit Auto-Update ab Phase 3.
- Dokumentation: `docs/formats/` für jedes Format mit Byte-Layout und Quelle der Information.
- Lizenz-Audit bei jeder neuen Abhängigkeit (`cargo deny`).
