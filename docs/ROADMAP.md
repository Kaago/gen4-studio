# Gen4 Studio: Roadmap

Stand: 2026-10-23. Spezifikation: [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](superpowers/specs/2026-10-21-gen4-studio-design.md). Analyse: [docs/research/2026-10-20-dspre-und-suite-analyse.md](research/2026-10-20-dspre-und-suite-analyse.md).

Jede Phase endet mit lauffähiger, testbarer Software und einem Gate. Keine Phase beginnt, bevor das Gate der vorherigen bestanden ist. Zeitangaben sind bewusst weggelassen; die Reihenfolge und die Gates sind die Zusage.

## Übersicht

| Phase | Ergebnis | Gate |
|---|---|---|
| 0 | Entscheidungsgrundlage (erledigt) | Analyse, Spec, Roadmap liegen vor |
| 1 | Kernel-Fundament und Shell-Spike: Projekt öffnen, Permissions malen, speichern, ROM bauen, im Emulator laufen | Edit-Build-Play-Schleife funktioniert auf einem HGSS- und einem Platin-Projekt |
| 2 | Welt-Editor v1: Header, Matrix, Gebäude, Events, Warps, 3D-Modelle (implementiert, Gate offen) | Eine neue Route aus bestehenden Karten zusammensetzen und spielen |
| 3a | Welt-Explorer, Assistenten, Weltkarte: Orte statt Tabellen, neue Gebiete und Räume per Assistent, Kartenbilder (in Arbeit, Gate offen) | Neue Route und neuer Raum mit Eingang per Assistent in unter fünf Minuten, in melonDS begehbar |
| 3b.1 | Fundament: neue Header per Patch, Höhen, Objekt-Sprites, HGSS-Kachelnamen, apicula-Zeitlimit, Gate-Skript (implementiert, Gate offen) | Neues Gebiet mit neuem Header per Assistent, in melonDS begehbar und in DSPRE offen; `scripts/gate.sh` grün |
| 3b.2 | Skripte und Text: rotom, CodeMirror 6, chatot, Text-Editor (implementiert, Gate offen) | Ein Ereignis-Script und neue Texte komplett im Studio; `scripts/gate.sh` grün |
| 3b.3 | Daten-Editoren: Encounter, Trainer, Pokémon, Moves, Items | Neue Trainer und Encounter komplett im Studio |
| 3b.4 | Import und Prüfungen: Kartenimport, Projektprüfungen | Ein kleiner Hack (neue Route, zwei neue Trainer, neue Texte, ein Ereignis-Script) komplett im Studio gebaut und gespielt |
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

Status: implementiert, Gate offen. 2a und 2b sind gebaut und automatisch getestet; die Prüfung von Hand (GUI, melonDS) steht in [docs/phase-2-gate.md](phase-2-gate.md) und ist noch nicht durchgeführt. Phase 2 gilt erst als abgeschlossen, wenn dort das Ergebnis eingetragen ist.

Detailpläne: [Phase 2a: Fundament, Bundle und Orientierung](superpowers/plans/2026-10-21-phase-2a-foundation-and-orientation.md) (Eingangskriterien, Workbench, Hover, Event-Marker, Texturen, Ortsnamen, große Matrizen, App-Bundle) und [Phase 2b: Welt-Editoren](superpowers/plans/2026-10-21-phase-2b-editors.md) (Header, Matrix, Area-Data, Gebäude, Events, Undo je Asset, Querverlinkung).

Eingangskriterien aus dem Abschluss-Review von Phase 1:
- Stale-Fall auflösbar machen: einzelne Assets verwerfen oder neu laden, Meldung in Klartext statt `AssetId { … }`.
- App-Bundle lauffähig machen: `libnitroarc_ffi.dylib` und apicula mit ausliefern; `pnpm tauri build` auf einem sauberen Rechner prüfen.
- Cmd+Q-Rückfrage prüfen; falls sie fehlt, Exit-Guard auf Rust-Seite (`RunEvent::ExitRequested`).
- Kernel aufräumen: `Store` entpackt bei Bedarf selbst, Edit-Closures dürfen fehlschlagen, Fehler tragen Pfad und Asset, eigener Fehlertyp für Projekt-Metadaten statt `Build`.
- `MapFile::validate` lehnt `buildings_tail` ab 48 Bytes ab, bevor Gebäude editierbar werden.
- Model-Cache über einen Hash der NSBMD-Bytes statt über mtimes. (Die frühere Angabe, apicula laufe in `spawn_blocking`, stimmte nicht: der Aufruf lief auf dem Async-Pool. Das behebt erst Phase 3b.1, zusammen mit dem Zeitlimit.)
- Gated Vanilla-Tests gegen mindestens ein Platin- und ein HGSS-Projekt. Ohne CI: seit Phase 3b.1 über `scripts/gate.sh` (erledigt).
- Matrix-Orientierung bestätigt (Byte 0 = Breite), Test in g4-formats.
- Orientierung im Welt-Editor: Kachelkoordinaten und Kacheltyp beim Überfahren anzeigen, Warps und Events markieren, Map-Texturen laden. Ohne diese Hilfen ließ sich im GUI-Test die Treppe nicht finden.
- Header-Liste lesbar machen: Anzeigenamen statt interner Kürzel (T01R0102 ist das Haus des Rivalen, T01R0202 das des Spielers), keine Zeilenumbrüche.
- Große Matrizen (Matrix 0, 30 × 30) laden: Modelle nur im sichtbaren Bereich konvertieren, Ladezeit messen.


Lieferumfang:
- Header-Editor (uxie `MapHeader` lesen und schreiben, Header-Tabelle in `arm9.bin` über Offset-Metadaten je Dump). Er ändert bestehende Header; neue Header gibt es seit Phase 3b.1 per Patch (siehe dort).
- Matrix-Editor: Karten zuweisen, Header-Zellen, Höhen; neue Matrix anlegen. Der Höhenabschnitt (`altitudes`) ist editierbar; die Szene wertet ihn seit Phase 3b.1 aus.
- Gebäude: Position, Skalierung, Modellwahl aus dem Gebäude-NARC, Vorschau als GLB. Die Rotation (13 Bytes ab 0x10) wird verbatim erhalten und nur als Hexwert angezeigt, weil das Spiel sie bei Karten-Gebäuden ignoriert.
- Events: uxie `EventFile` (Objekte, Warps, Koordinaten-Trigger, Hintergrund-Events) platzieren und verschieben in der 3D-Ansicht; Warps per Drag verbinden.
- Area-Data-Editor.
- Workbench mit Dockview: Projektbaum, Tabs, Inspector, Problems.
- Undo/Redo über alle Asset-Arten; "Open in ..."-Querverlinkung.

Gate: Eine neue Route aus bestehenden Kartendateien in einer neuen Matrix, mit Warps zu einer bestehenden Stadt, ist im Emulator begehbar. Ablauf und Einzelprüfungen: [docs/phase-2-gate.md](phase-2-gate.md). Teil (B) dieses Gates läuft künftig über die Assistenten von Phase 3a.

Offene Reste von Phase 2, erledigt in Phase 3b.1 (siehe dort):
- Höhen (`altitudes`) in der Szene auswerten (erledigt: eine Stufe ist eine halbe Kachel).
- Objekt-Sprites für Events (erledigt: Billboards, gelber Marker als Rückfall).
- Neue Header per Patch (erledigt: DSPRE-kompatibel über „Dynamic Headers“, angeboten im Assistenten).
- Kachelverhalten-Namen je Spielfamilie (erledigt: HGSS-Satz aus pokeheartgold).
- apicula mit Timeout aufrufen und von `spawn_blocking` ausführen (erledigt).
- Gated Vanilla-Tests statt CI (erledigt: `scripts/gate.sh`; es gibt kein Remote und keine CI).
- Modell-Cache-Schlüssel um die apicula-Version erweitern, damit auch `.fail`-Marker bei einem apicula-Update ungültig werden (erledigt).

## Phase 3a: Welt-Explorer, Assistenten, Weltkarte

Status: in Arbeit. Alle Tasks sind implementiert und automatisch getestet; die Prüfung von Hand (GUI, melonDS) steht in [docs/phase-3a-gate.md](phase-3a-gate.md) und ist noch nicht durchgeführt. Anlass war der Gate-Durchlauf von Phase 2: das Raster-Panel war nicht verständlich, die Reihenfolge der Schritte unklar und das Ergebnis nicht sichtbar.

Spezifikation: [docs/superpowers/specs/2026-10-22-world-explorer-design.md](superpowers/specs/2026-10-22-world-explorer-design.md).

Lieferumfang:
- Welt-Explorer: Baum nach Orten (Außen, Räume) mit Suche, Umschalter auf die Liste, Glossar "Begriffe"; Panel "Raster" mit Kartenbildern.
- Assistenten "Gebiet anlegen", "Raum anlegen" und "Gebiet erweitern" mit Zusammenfassung vor dem Anlegen und Ergebnis-Schritt; Abschnitt "Eingänge / Ausgänge" im Header-Editor (Raum anbinden, Zum Ziel).
- Weltkarte: Region mit Ortsfarben, Räume je Ort, Linien von Räumen zu ihren Eingängen.
- Backend: Übersichten (`list_matrices`, `list_warps`), Assistenten-Befehle (`create_area`, `extend_area`, `create_room`), Store-Erweiterungen für Eventdateien und Karten.

Gate: Die neue Route aus Phase 2 (B) und ein neuer Raum mit Eingang entstehen per Assistent in unter fünf Minuten, ohne eine Zahl außer der Header-Wahl zu tippen, und sind in melonDS begehbar. Ablauf und Einzelprüfungen: [docs/phase-3a-gate.md](phase-3a-gate.md).

Nicht in 3a: neue Header (Patch, jetzt 3b.1), Gelände malen (Phase 6), Höhen und Objekt-Sprites (jetzt 3b.1), Verschieben von Karten in der Weltkarte.

## Phase 3b: Daten-Editoren, Skripte, Text, Import

Phase 3b ist zu groß für eine Spec und läuft in vier Teilprojekten mit je eigener Spec, eigenem Plan und eigenem Gate. 3b.1 wurde auf Wunsch implementiert, während das Gate von Phase 3a noch offen ist; beide Gates sind offen. 3b.2 bis 3b.4 beginnen, wenn beide Gates bestanden sind; die Teile bauen aufeinander auf.

### Phase 3b.1: Fundament

Status: implementiert, Gate offen. Alle Tasks sind gebaut und automatisch getestet (`scripts/gate.sh`); die Prüfung von Hand (GUI, melonDS, DSPRE) steht in [docs/phase-3b1-gate.md](phase-3b1-gate.md) und ist noch nicht durchgeführt.

Spezifikation: [docs/superpowers/specs/2026-10-22-phase-3b1-foundation-design.md](superpowers/specs/2026-10-22-phase-3b1-foundation-design.md). Plan: [docs/superpowers/plans/2026-10-22-phase-3b1-foundation.md](superpowers/plans/2026-10-22-phase-3b1-foundation.md).

Lieferumfang:
- Neue Header, DSPRE-kompatibel über den Patch „Dynamic Headers“ (ein Header je Datei in einem NARC); im Schritt „Header“ der Assistenten „Gebiet anlegen“ und „Raum anlegen“ als „Neuer Header“, der Patch wird nach Nachfrage angewendet. Nur Platin und HeartGold/SoulSilver.
- Höhen (`altitudes`) der Matrix in der 3D-Szene (eine halbe Kachel je Stufe; Modell, Gebäude, Overlay, Marker, Picking und Event-Drag folgen der Oberfläche).
- Objekt-Sprites als Billboards (Richtung aus dem Event, gelber Marker als Rückfall); NSBTX-Leser und Textur-Dekoder in `g4-formats`.
- Kachelverhalten-Namen je Spielfamilie (HGSS aus pokeheartgold).
- apicula mit Zeitlimit (`G4_APICULA_TIMEOUT_SECS`, Standard 60 s) auf einem Arbeitsthread; Modell-Cache-Schlüssel mit apicula-Werkzeug-Hash.
- Gate-Skript `scripts/gate.sh` statt CI.

Gate: Auf Kopien von Platin DE und HeartGold DE entsteht ein neues Gebiet per Assistent mit einem neuen Header (Patch nach Nachfrage), das ROM baut, das Gebiet ist in melonDS begehbar, und DSPRE öffnet das Projekt, zeigt den Patch als angewendet und den neuen Header in seiner Liste. Eine Matrixzelle mit Höhe 2 liegt in der Szene eine Kachel höher als ihre Nachbarn, samt Gebäude, Overlay und Marker. Personen und Trainer zeigen ihr Sprite in Blickrichtung, ein Objekt ohne Sprite den Marker. Auf HeartGold zeigt der Hover HGSS-Kachelnamen (zum Beispiel `HEADBUTT`). `scripts/gate.sh` läuft grün gegen alle drei Projekte. Ablauf: [docs/phase-3b1-gate.md](phase-3b1-gate.md).

Nicht in 3b.1: Header löschen, DSPREs ARM9-Expansion und andere Patches, die OW-Tabellen-Erweiterung (`OWTBLXPANDV1`), Animation der Sprites, CI auf fremden Rechnern.

### Phase 3b.2: Skripte und Text

Status: implementiert, Gate offen. Alle Tasks sind gebaut und automatisch getestet (`scripts/gate.sh`, auch die gated Läufe gegen Platin DE, HeartGold DE und HeartGold US); die Prüfung von Hand (GUI, melonDS, DSPRE) und der Rauchtest der gepackten App stehen in [docs/phase-3b2-gate.md](phase-3b2-gate.md) und sind noch nicht durchgeführt.

Spezifikation: [docs/superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md](superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md). Plan: [docs/superpowers/plans/2026-10-22-phase-3b2-scripts-text.md](superpowers/plans/2026-10-22-phase-3b2-scripts-text.md).

Lieferumfang:
- Textarchive editierbar: Quelle ist `expanded/textArchives/NNNN.json` (chatot-JSON); Speichern schreibt JSON und Member. Panel „Text“ mit Archivliste, Nachrichten, Anfügen, Suche über alle Archive, Überlängen-Warnung und neuem Archiv.
- Scripts übernehmen: expliziter Schritt, der `rotom.toml` und `.rotom/command_database/` anlegt und alle Member dekompiliert; ein von DSPRE oder rotom schon übernommenes Projekt wird ohne Nachfrage benutzt. Die Befehlsdatenbank kommt aus `crates/g4-kernel/metadata/scrcmd/` (Revision 33), nie aus dem Netz.
- Script-Editor: CodeMirror 6 mit rotom-Syntax und `rotom-lsp` als Sidecar (Diagnosen, Completion, Hover, Signaturhilfe, Gehe-zu, Outline). Speichern schreibt Quelle, kompiliert und schreibt den Member; Compile-Fehler stehen im Panel „Probleme“ (unter der 3D-Gruppe) mit Sprung zur Zeile. Ohne `rotom-lsp` laufen Öffnen, Bearbeiten und Speichern weiter.
- Textliterale (`Message "…"`); das betroffene Textarchiv wird nach dem Compile neu geladen.
- Level-Scripts: Formular im Header-Editor plus „Als JSON öffnen“; das Formular speichert nie ein Dokument, das rotom ablehnt.
- Navigation: Scriptdatei und Level-Script im Header-Editor (Öffnen, Neu), Event-Feld „Script“ (Öffnen, Neues Script), Knoten „Scripts“ im Welt-Explorer mit Volltextsuche.
- Build: vor dem Packen werden alle veralteten Quellen kompiliert und alle veralteten Text-JSON kodiert; ein Fehler bricht den Build mit Problemliste ab. Ein unverändertes, übernommenes Projekt baut byte-identisch (gated Test).

Gate: Auf Kopien von Platin DE und HeartGold DE läuft „Scripts übernehmen“ ohne Fehler, ein neuer NPC auf der neuen Route spricht einen Text aus einem Textliteral, gibt einmalig ein Item (Flag per Completion) und sagt danach etwas anderes, ein Vanilla-Dialog ist geändert, ein Level-Script (`on_load`) läuft beim Betreten, ein Text ist geändert und einer angefügt (die Suche findet ihn), DSPRE öffnet das Projekt und zeigt die Textänderung, und `scripts/gate.sh` läuft grün gegen alle drei Projekte (mit `G4_ROTOM_LSP`). Ablauf: [docs/phase-3b2-gate.md](phase-3b2-gate.md).

Nicht in 3b.2: visueller Event- oder Node-Editor; Flag- und Variablen-Picker-Dialoge; DSPREs `.script`-Textformat als Editorformat; Rückwärtsreferenzen „welche Scripts nutzen Nachricht N“; mehrere Sprachen in einem Archiv; ein Diamant/Perl-Gate; Umbenennen, Referenzen und Formatieren im Editor.

### Phase 3b.3: Daten-Editoren

Lieferumfang:
- Encounter (DPPt und HGSS), Trainer (uxie), Pokémon-Basisdaten, Moves, Items (uxie). Für Encounter ist der geplante eigene Writer in `g4-formats` neu zu prüfen: uxie 0.9.7 hat bereits einen Encounter-Writer (`to_binary`).
- Datenbank-UI nach Pokémon-Studio-Muster; Referenzzähler.

Gate: Zwei neue Trainer und geänderte Encounter entstehen komplett im Studio und sind im Spiel zu sehen.

### Phase 3b.4: Import und Prüfungen

Lieferumfang:
- Kartenimport: PDSMS `.pdsmap`/`.pdsts`, NSBMD-Dateien, OBJ über apicula-Pfad; Kollisionsimport.
- Project Checks: kaputte Warps, Scripts ohne Ziel, Trainer ohne Party, Textlängen.

Gate (für Phase 3b insgesamt): Ein kleiner Hack (neue Route, zwei neue Trainer, neue Texte, ein Ereignis-Script) wird komplett im Studio gebaut und gespielt.

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

- Prüfung: `scripts/gate.sh` fährt lokal `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm test`, `pnpm build` und die gated Tests gegen die drei Testprojekte (siehe `docs/dev-setup.md`). Es gibt kein Remote und noch keine CI; Builds für macOS (arm64) und Windows (x64), Linux als Best-Effort, folgen mit einem Remote.
- Releases: Tauri-Bundles, signiert, mit Auto-Update ab Phase 3b.
- Dokumentation: `docs/formats/` für jedes Format mit Byte-Layout und Quelle der Information.
- Lizenz-Audit bei jeder neuen Abhängigkeit (`cargo deny`).
