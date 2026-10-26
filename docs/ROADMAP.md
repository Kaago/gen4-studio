# Gen4 Studio: Roadmap

Stand: 2026-10-25. Spezifikation: [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](superpowers/specs/2026-10-21-gen4-studio-design.md). Analyse: [docs/research/2026-10-20-dspre-und-suite-analyse.md](research/2026-10-20-dspre-und-suite-analyse.md).

Jede Phase endet mit lauffähiger, testbarer Software und einem Gate. Keine Phase beginnt, bevor das Gate der vorherigen bestanden ist. Zeitangaben sind bewusst weggelassen; die Reihenfolge und die Gates sind die Zusage.

## Übersicht

| Phase | Ergebnis | Gate |
|---|---|---|
| 0 | Entscheidungsgrundlage (erledigt) | Analyse, Spec, Roadmap liegen vor |
| 1 | Kernel-Fundament und Shell-Spike: Projekt öffnen, Permissions malen, speichern, ROM bauen, im Emulator laufen | Edit-Build-Play-Schleife funktioniert auf einem HGSS- und einem Platin-Projekt |
| 2 | Welt-Editor v1: Header, Matrix, Gebäude, Events, Warps, 3D-Modelle (durch Gate 3a abgelöst) | Eine neue Route aus bestehenden Karten zusammensetzen und spielen |
| 3a | Welt-Explorer, Assistenten, Weltkarte: Orte statt Tabellen, neue Gebiete und Räume per Assistent, Kartenbilder (Gate bestanden, DSPRE und gepackte App auf Wunsch ausgelassen) | Neue Route und neuer Raum mit Eingang per Assistent in unter fünf Minuten, in melonDS begehbar |
| 3b.1 | Fundament: neue Header per Patch, Höhen, Objekt-Sprites, HGSS-Kachelnamen, apicula-Zeitlimit, Gate-Skript (Gate bestanden, DSPRE und gepackte App auf Wunsch ausgelassen) | Neues Gebiet mit neuem Header per Assistent, in melonDS begehbar und in DSPRE offen; `scripts/gate.sh` grün |
| 3b.2 | Scripts und Text: rotom, CodeMirror 6, chatot, Text-Editor (Gate bestanden, DSPRE und gepackte App auf Wunsch ausgelassen) | Ein Ereignis-Script und neue Texte komplett im Studio; `scripts/gate.sh` grün |
| 3c | Doppeldeck: Design-System in der App (Tokens, Komponentenbibliothek, Rahmen aus Titelleiste, Atlas, Liste, Schau, Scharnier, Hand), Bereich Welt im neuen Raster; 3c.2 bringt Texte, Scripts, Technik, Assistenten (Gate vom Agenten bestanden; Blick, rotom-lsp und gepackte App beim Nutzer offen) | Der Screen „Welt bauen“ ist in der App wiedererkennbar, die Abnahmen 3a, 3b.1 und 3b.2 laufen über die Test-Umgebung erneut grün; `scripts/gate.sh` grün |
| 3b.3 | Daten-Editoren in zwei Teilen: 3b.3a Codecs, Pokédex, Attacken, Gegenstände; 3b.3b Trainer, Begegnungen (3b.3a: Gate Teil A vom Agenten bestanden, Blick beim Nutzer offen; 3b.3b: Gate Teil B vom Agenten bestanden; Blick und DSPRE beim Nutzer offen) | Neue Trainer und Encounter komplett im Studio |
| 3b.4 | Prüfungen und Import in zwei Teilen: 3b.4a Prüfen und Beheben; 3b.4b Import fertiger Kartenteile und Gesamtgate (3b.4a: Gate A vom Agenten bestanden, Blick beim Nutzer offen; 3b.4b: Gate B (3b-Gesamtgate) vom Agenten bestanden (2026-10-25: Szenarien auf Platin DE, HeartGold DE und HeartGold US, Browser-Zeilen (B5) bis (B7), `scripts/gate.sh` grün); Blick (B4: Route texturiert, Nachbar, Geber) und Hand (B8 DSPRE, B9 Dateidialog) beim Nutzer offen) | 3b-Gesamtgate: Ein kleiner Hack (neue Route mit importierter Karte, zwei neue Trainer, neue Texte, ein Ereignis-Script) komplett im Studio gebaut und gespielt; die Prüfungen sind vorher rot und nachher grün |
| 3e | Werkbank: Scharnier über der Schau, schwebende Werkzeugleiste, Weltbaum Weltkarte › Gebiet › Räume mit Ortsetikett und Sortierung, Hierarchie „In dieser Szene“ statt „An diesem Ort“, Inspector nur fürs Gewählte, ein Dock je Bereich mit freier Anordnung (geplant) | Screen „Welt bauen“ in der Anordnung „Werkbank“ wiedererkennbar, in Bauen kein „Raster“, „Matrix“ oder „Header“ sichtbar, die Abnahmen 3a bis 3c laufen erneut grün; `scripts/gate.sh` grün |
| 3f | Weltmodell: Reiter „Raster“ nur in Tüfteln, Weltkarte und 3D übernehmen Zellen und Auswahl, eigene Orte als Ortsnamen im ROM, Patch mit einer Zustimmung im Assistenten, optional nahtlose Gebiete und Abspalten (geplant) | Gebiet mit neuem Ort „Neuheim“ in einem Durchgang in Bauen, Einblendung in melonDS, `vanilla_checks` 0 Fehler |
| 3g | Projektstart: „Neues Spiel aus ROM…“ mit Vorlagen Original, Leere Region mit Startstadt, Nur Daten; „Welt leeren…“ (geplant) | Frisches Projekt baut byte-gleich zur Eingabe-ROM; die leere Region startet in `g4emu` mit Starter, `run_checks` 0 Fehler |
| 3h | Agenten-Schnittstelle (MCP): kuratierte Tools auf der Befehlsschicht der Brücke, `studio mcp` kopflos und in der laufenden App, Freigabe je Projekt, Agenten-Schritte im Verlauf (geplant) | Agent legt über MCP ein Gebiet an, die Szene zeigt es ohne Neuladen, ⌘Z nimmt es zurück; `scripts/gate.sh` mit Schritt `mcp` grün |
| 4 | Textprojekt und Build-Graph | Projekt in Git, inkrementeller Build, xdelta-Export, byte-identischer Rebuild eines unveränderten Projekts |
| 5 | Plugin-Host | Ein First-Party-Editor und ein Beispiel-Plugin laufen über dieselbe API |
| 6 | Kartenerstellung und Weltformat | Karte im Studio malen, zu NSBMD/BDHC bauen, spielen |

Phase 3 (3a, 3b.1 bis 3b.4, 3c) ist abgeschlossen (2026-10-25): alle Teile implementiert, die Gates von 3a, 3b.1, 3b.2, 3c, 3b.3, 3b.4a und 3b.4b (das 3b-Gesamtgate) sind vom Agenten bestanden, zuletzt Gate B am 2026-10-25 (Szenario-Teil auf Platin DE, HeartGold DE und HeartGold US, Browser-Zeilen (B5) bis (B7), `scripts/gate.sh` grün in 2734 s). Vor Phase 4 laufen die Ausbau-Phasen 3e bis 3h aus der Wunschliste des Nutzers (Abschnitt „Ausbau nach Phase 3“ unten). Beim Nutzer offen bleiben die Blicke und Hand-Punkte der Teil-Gates (in den Abschnitten unten je Teil genannt), für 3b.4b der Blick (B4: Route texturiert, Nachbar, Geber) und die Hand-Punkte (B8) DSPRE und (B9) Dateidialog.

## Phase 1: Kernel-Fundament und Shell-Spike

Detailplan: [docs/superpowers/plans/2026-10-21-phase-1-kernel-and-shell.md](superpowers/plans/2026-10-21-phase-1-kernel-and-shell.md).

Lieferumfang:
- Cargo-Workspace mit `g4-formats` (NARC, Map, Matrix, AreaData) und `g4-kernel` (Project, Store, Build, CLI `g4k`).
- Tauri-App `apps/studio`: Projekt öffnen, Header-Liste, Matrix-Ansicht in three.js mit Permissions-Overlay und Gebäude-Boxen, NSBMD-Modelle über apicula als GLB.
- Permissions malen, speichern, ROM bauen, Emulator starten.

Gate: Auf einem HGSS- und einem Platin-Projekt wird eine Kollision geändert, das ROM gebaut und der Effekt im Emulator gezeigt. Alle Vanilla-Round-Trip-Tests laufen byte-identisch.

Risiken: uxie-Header-Offsets für HGSS; NARC-Padding; Gebäudekoordinaten-Konvention.

## Phase 2: Welt-Editor v1

Status: durch Gate 3a abgelöst. 2a und 2b sind gebaut und automatisch getestet; die Prüfung von Hand in [docs/phase-2-gate.md](phase-2-gate.md) wurde nicht mehr einzeln durchgeführt, ihre Punkte laufen im Gate von Phase 3a ([docs/phase-3a-gate.md](phase-3a-gate.md)).

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

Status: Gate bestanden (DSPRE und gepackte App auf Wunsch ausgelassen); die Ergebnisse stehen in [docs/phase-3a-gate.md](phase-3a-gate.md). Anlass war der Gate-Durchlauf von Phase 2: das Raster-Panel war nicht verständlich, die Reihenfolge der Schritte unklar und das Ergebnis nicht sichtbar.

Spezifikation: [docs/superpowers/specs/2026-10-22-world-explorer-design.md](superpowers/specs/2026-10-22-world-explorer-design.md).

Lieferumfang:
- Welt-Explorer: Baum nach Orten (Außen, Räume) mit Suche, Umschalter auf die Liste, Glossar "Begriffe"; Panel "Raster" mit Kartenbildern.
- Assistenten "Gebiet anlegen", "Raum anlegen" und "Gebiet erweitern" mit Zusammenfassung vor dem Anlegen und Ergebnis-Schritt; Abschnitt "Eingänge / Ausgänge" im Header-Editor (Raum anbinden, Zum Ziel).
- Weltkarte: Region mit Ortsfarben, Räume je Ort, Linien von Räumen zu ihren Eingängen.
- Backend: Übersichten (`list_matrices`, `list_warps`), Assistenten-Befehle (`create_area`, `extend_area`, `create_room`), Store-Erweiterungen für Eventdateien und Karten.

Gate: Die neue Route aus Phase 2 (B) und ein neuer Raum mit Eingang entstehen per Assistent in unter fünf Minuten, ohne eine Zahl außer der Header-Wahl zu tippen, und sind in melonDS begehbar. Ablauf und Einzelprüfungen: [docs/phase-3a-gate.md](phase-3a-gate.md).

Nicht in 3a: neue Header (Patch, jetzt 3b.1), Gelände malen (Phase 6), Höhen und Objekt-Sprites (jetzt 3b.1), Verschieben von Karten in der Weltkarte.

## Phase 3b: Daten-Editoren, Scripts, Text, Import

Phase 3b ist zu groß für eine Spec und läuft in vier Teilprojekten mit je eigener Spec, eigenem Plan und eigenem Gate. 3b.1 wurde auf Wunsch implementiert, während das Gate von Phase 3a noch offen war; beide Gates sind inzwischen bestanden. Die Teile bauen aufeinander auf.

### Phase 3b.1: Fundament

Status: Gate bestanden (DSPRE und gepackte App auf Wunsch ausgelassen). Alle Tasks sind gebaut und automatisch getestet (`scripts/gate.sh`); die Ergebnisse der Prüfung stehen in [docs/phase-3b1-gate.md](phase-3b1-gate.md).

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

### Phase 3b.2: Scripts und Text

Status: Gate bestanden (DSPRE und gepackte App auf Wunsch ausgelassen). Alle Tasks sind gebaut und automatisch getestet (`scripts/gate.sh`, auch die gated Läufe gegen Platin DE, HeartGold DE und HeartGold US); die Ergebnisse der Prüfung stehen in [docs/phase-3b2-gate.md](phase-3b2-gate.md).

Spezifikation: [docs/superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md](superpowers/specs/2026-10-22-phase-3b2-scripts-text-design.md). Plan: [docs/superpowers/plans/2026-10-22-phase-3b2-scripts-text.md](superpowers/plans/2026-10-22-phase-3b2-scripts-text.md).

Lieferumfang:
- Textarchive editierbar: Quelle ist `expanded/textArchives/NNNN.json` (chatot-JSON); Speichern schreibt JSON und Member. Panel „Text“ mit Archivliste, Nachrichten, Anfügen, Suche über alle Archive, Überlängen-Warnung und neuem Archiv.
- Scripts übernehmen: expliziter Schritt, der `rotom.toml` und `.rotom/command_database/` anlegt und alle Member dekompiliert; ein von DSPRE oder rotom schon übernommenes Projekt wird ohne Nachfrage benutzt. Die Befehlsdatenbank kommt aus `crates/g4-kernel/metadata/scrcmd/` (Revision 33), nie aus dem Netz.
- Script-Editor: CodeMirror 6 mit rotom-Syntax und `rotom-lsp` als Sidecar (Diagnosen, Completion, Hover, Signaturhilfe, Gehe-zu, Outline). Speichern schreibt Quelle, kompiliert und schreibt den Member; Compile-Fehler stehen im Panel „Probleme“ (unter der 3D-Gruppe) mit Sprung zur Zeile. Ohne `rotom-lsp` laufen Öffnen, Bearbeiten und Speichern weiter.
- Textliterale (`Message "…"`); das betroffene Textarchiv wird nach dem Compile neu geladen.
- Level-Scripts: Formular im Header-Editor plus „Als JSON öffnen“; das Formular speichert nie ein Dokument, das rotom ablehnt.
- Navigation: Scriptdatei und Level-Script im Header-Editor (Öffnen, Neu), Event-Feld „Script“ (Öffnen, Neues Script), Knoten „Scripts“ im Welt-Explorer mit Volltextsuche.
- Build: vor dem Packen werden alle veralteten Quellen kompiliert und alle veralteten Text-JSON kodiert; ein Fehler bricht den Build mit Problemliste ab. Ein unverändertes, übernommenes Projekt baut byte-identisch (gated Test).

Gate: Auf Kopien von Platin DE und HeartGold DE läuft „Scripts übernehmen“ ohne Fehler, ein neuer NPC auf der neuen Route spricht einen Text aus einem Textliteral, gibt einmalig ein Item (Flag per Completion) und sagt danach etwas anderes, ein Vanilla-Dialog ist geändert, ein Level-Script (Variablen-Bedingung, `on_var_condition`) zeigt beim Betreten eine Nachricht, ein Text ist geändert und einer angefügt (die Suche findet ihn), DSPRE öffnet das Projekt und zeigt die Textänderung, und `scripts/gate.sh` läuft grün gegen alle drei Projekte (mit `G4_ROTOM_LSP`). Ablauf: [docs/phase-3b2-gate.md](phase-3b2-gate.md).

Nicht in 3b.2: visueller Event- oder Node-Editor; Flag- und Variablen-Picker-Dialoge; DSPREs `.script`-Textformat als Editorformat; Rückwärtsreferenzen „welche Scripts nutzen Nachricht N“; mehrere Sprachen in einem Archiv; ein Diamant/Perl-Gate; Umbenennen, Referenzen und Formatieren im Editor.

### Phase 3b.3: Daten-Editoren

Status: 3b.3a: Gate Teil A vom Agenten bestanden (2026-10-24, (A1) bis (A7) über die Test-Umgebung, `scripts/gate.sh` grün, drei Befunde im Branch der Abnahme behoben: die Suche zeichnete Schau und Hand neu und die Pokédex-Liste lag über 50 ms, leere Schau ohne Taste, Pokédex-Farbe „Violett“); beim Nutzer offen ist der Blick auf die drei Editoren gegen ihre Screens, auf die Item- und Pokémon-Symbole und auf Dunkel und Kompakt. Ergebnisse in [docs/phase-3b3-gate.md](phase-3b3-gate.md), Teil A. 3b.3b: Gate Teil B vom Agenten bestanden (2026-10-24, (B1) bis (B10) über die Test-Umgebung, die Szenarien `pt-trainer-begegnung.yaml` und `hg-trainer-begegnung.yaml` auf frischen Kopien grün, optional auch HeartGold US (B13), `scripts/gate.sh` grün; vier Befunde im Branch der Abnahme behoben: englische Ablehnungen beim Anlegen, die Schau des Trainers folgte dem Platzieren nicht, die Felder min und max der Wasser-Begegnungen liefen über, die Meldung nannte die Kampf-Scriptdatei nicht); beim Nutzer offen sind der Blick auf die Screens und auf Dunkel/Kompakt (B10, B11) und die DSPRE-Gegenprobe (B12), siehe [docs/phase-3b3-gate.md](phase-3b3-gate.md), Teil B.

Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md](superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md). Plan 3b.3a: [docs/superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md](superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md). Plan 3b.3b: [docs/superpowers/plans/2026-10-24-phase-3b3b-trainer-encounters.md](superpowers/plans/2026-10-24-phase-3b3b-trainer-encounters.md). Recherche: [docs/research/2026-10-23-3b3/](research/2026-10-23-3b3/).

Der Teil ist zu groß für einen Plan und läuft in zwei Plänen mit je eigenem Gate:

**3b.3a „Codecs und Pokémon-Daten“** (Gate Teil A vom Agenten bestanden, Blick beim Nutzer offen):
- Eigene Codecs in `g4-formats/src/data/` für alle Daten-Archive, auch Trainer und Begegnungen, damit der Round-Trip von Anfang an vollständig ist: Begegnungen (Platin und HGSS), Trainer-Eigenschaften und -Party, Pokémon-Basisdaten, Lernsets, Entwicklungen, Attacken, Gegenstände, Trainer-Texttabellen; dazu NCGR und NCLR für Symbole. Unbekannte Bits bleiben Rohwerte, nur eine falsche Länge sperrt ein Member. uxie 0.9.7 ist nur noch Test-Orakel.
- Encounter-Writer: entschieden, ein eigener Codec. Der Writer von uxie 0.9.7 ist unbrauchbar (leere Slots gelten als ungültig, der HGSS-Leser ist um 8 Byte versetzt, Schwärme werden doppelt geschrieben); ebenso sein Attacken-Codec (lehnt 172 von 471 Vanilla-Attacken ab, schreibt 14 statt 16 Byte).
- Store: die neuen Archivarten als Assets, mehrteiliger Changeset (`Store::transaction`, ein Undo-Schritt über mehrere Assets), Namenstabellen aus den Textarchiven, arm9-Tabellen nur lesend (Item-Tabelle, TM/VM-Liste, Paletten der Pokémon-Symbole; mit Plausibilitätsprüfung, sonst „nicht verfügbar“), Referenzindex (`refs`, inkrementell), Item- und Pokémon-Symbole aus dem ROM. Der Build packt die Daten-Ordner mit, der Vanilla-Rebuild bleibt byte-identisch.
- Generische Befehle `data_list`, `data_get`, `data_edit`, `data_raw` und `name_tables`, `item_table`, `machine_table`, `refs`, `get_item_icon`, `get_mon_icon` in Tauri und Brücke.
- Oberfläche: Atlas-Bereiche Pokémon (Unterbereiche Pokédex und Attacken; Trainer und Begegnungen im Umschalter gesperrt bis 3b.3b) und Gegenstände; Editoren Pokédex (Werte, Fähigkeiten und Typen, Lernset, Entwicklung, TM/VM mit „Von anderer Art übernehmen“), Attacken, Gegenstände; Rohdaten als Hexblock in Tüfteln; neue Primitives `TypeBadge`, `StatBar`, `Table`, `FilterChips`, `ToggleGrid`, `HexBlock`, `Sprite`, Unterbereichs-Menü im Listenkopf.
- Prüfschritt `g4k check-data` und Szenario `pt-daten.yaml` mit Abschnitt `check:`.

Gate 3b.3a: `vanilla_data` grün über alle Member der zehn Archivarten in Platin DE, HeartGold DE und HeartGold US, das uxie-Orakel stimmt für Personal, Items und Trainer-Eigenschaften, der Vanilla-Rebuild bleibt byte-identisch; das Szenario `pt-daten.yaml` ändert Staravia, Flügelschlag und Trank, baut und findet die Werte im gebauten ROM; die drei Editoren sind gegen ihre Screens wiedererkennbar, ⌘Z nimmt „Wie Staralili“ über zwei Assets in einem Schritt zurück, Tüfteln zeigt den Hexblock mit markiertem Feld, die Liste „Gegenstände“ zeigt Symbole aus dem ROM; `scripts/gate.sh` grün. Ablauf: [docs/phase-3b3-gate.md](phase-3b3-gate.md), Teil A.

**3b.3b „Trainer, Begegnungen, Gate“** (Gate Teil B vom Agenten bestanden; Blick und DSPRE beim Nutzer offen):
- Trainer als zusammengesetztes Asset (Eigenschaften, Team, Name, Kampftexte) im Unterbereich Trainer: Baum nach Ort oder Klasse, Team als Pokémon-Karten (`PokemonCard`), Hand mit Team, Trainer und Rohdaten, Kampftexte mit Warnung ohne ⏎. „Trainer anlegen…“ (`create_trainer`, nicht rückgängig, eine Undo-Grenze im Verlauf) erweitert die Kampf-Scriptdatei, wenn die neue ID hinter ihren Plätzen liegt, und gibt der ID des „gesehen“-Slots (Platin 929, HGSS 740) einen gesperrten Platzhalter ([docs/formats/trainer_scripts.md](formats/trainer_scripts.md)); `set_trainer_texts`.
- Event ↔ Trainer: „In der Welt platzieren“ (die nächste Figur bekommt Script 3000 + ID − 1, Sichtlinie und Sichtweite 3 in einem Schritt), Feld „Trainer“ im Inspector, Trainer-Figuren in „An diesem Ort“, Partner-Scripts 5000–6999.
- Begegnungen im Unterbereich Begegnungen: Anteile, Ersatzlisten und Warnungen aus dem Kernel (`encounter_rules.rs`), Methodenreiter, HGSS-Gras mit drei Tageszeiten; „Begegnungsdatei anlegen…“ (`create_encounter_file`), Feld „Begegnungen“ im Header-Editor, „Für diesen Ort abspalten“ (`split_encounter_file`).
- `g4emu`-Kampfschritte (`expect-battle`, `walk-until-battle`, `end-battle`, `win-battle`, `mash`; Platin und HeartGold DE mit Kampfzeiger), Antworten als Platzhalter in Szenarien, `g4k check-data` für Begegnungen, zwei Szenarien `pt-trainer-begegnung.yaml` und `hg-trainer-begegnung.yaml`.
- Nicht gekommen: die Hand-Höhe je Unterbereich (die Hand merkt sie weiter je Bereich; wartet auf den Umbau des Decks).

Gate (Roadmap-Gate 3b.3): Zwei neue Trainer und geänderte Encounter entstehen komplett im Studio und sind im Spiel zu sehen; `g4emu` liest Art und Stufe der Gegner aus dem RAM. Stand 2026-10-24: beide Szenarien grün (Platin DE: Trainer 928 per Sichtlinie, 930 angesprochen, Mewtu Lv 5 im Gras; HeartGold DE: 738 per Sichtlinie, 741 angesprochen, Mewtu Lv 5), `scripts/gate.sh` grün; die Abnahme der Editoren über die Test-Umgebung ist bestanden (Befunde behoben), HeartGold US läuft das Szenario ebenfalls grün; offen sind beim Nutzer der Blick auf die Screens und die DSPRE-Gegenprobe. Ablauf: [docs/phase-3b3-gate.md](phase-3b3-gate.md), Teil B.

Nicht in 3b.3: siehe die Tabelle „Später“ unter „Aus dem Design-System, später“.

### Phase 3b.4: Prüfungen und Import

Status: 3b.4a: Gate A vom Agenten bestanden (2026-10-25, (A1) bis (A6) über die Test-Umgebung: gated `vanilla_checks` auf allen drei Projekten, Zeitmessungen, `scripts/gate.sh`, das Szenario `pt-pruefungen.yaml` und der Browser-Teil auf frischen Kopien; sieben Befunde der Abnahme und die der Schlussprüfung in der Fix-Welle behoben, darunter der gesperrte Platzhalter-Trainer des „gesehen“-Slots als falscher Fehler, Script-Speichern ohne neue Prüfung, eine kaputte `g4-checks.json`, die das Projekt zuhielt, „Alles stimmig“ vor dem ersten Bericht und `matrix.header-elsewhere` als Fehler); beim Nutzer offen ist der Blick auf die rote Treppe (A6) 3. Ergebnisse in [docs/phase-3b4-gate.md](phase-3b4-gate.md), Teil A. 3b.4b: implementiert (NSBMD-Kopfleser, `map_import`, Befehle, Dialoge „Teile importieren…“ und „Teile exportieren…“ mit deutschen Sätzen je Teil und den Befunden der Karte nach dem Import, die Szenarien „kleiner Hack“). Szenario-Teil von Gate B (3b-Gesamtgate) vom Agenten bestanden (2026-10-25): `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml` auf frischen Kopien grün, optional auch HeartGold US, die Gegenproben ohne Import scheitern wie verlangt am Weg. Die Browser-Zeilen (B5) bis (B7) hat der Agent am selben Tag über die Test-Umgebung bestanden (Prüfliste, deutsche Fehlertexte, Befunde nach dem Import, Export, Szene, Vorschaubild, ⌘Z); ihre Befunde GB1 bis GB4 sind danach behoben (Export in einen fehlenden Ordner, die Meldung nach einem Wechsel des Teils, überlappende Segmente „Teil“, das Rezept für die Szene), GB6 und GB7 sind behoben (2026-10-25). `scripts/gate.sh` ist auf dem Stand der Abnahme grün (2026-10-25, 2734 s, alle 13 Schritte, (B1)); damit ist Gate B vom Agenten bestanden. Beim Nutzer offen sind der Blick (B4) auf die Bilder der Szenarien (Route texturiert, Nachbar, Geber) und die Hand-Punkte (B8) DSPRE und (B9) nativer Dateidialog der gebündelten App. Ergebnisse in [docs/phase-3b4-gate.md](phase-3b4-gate.md), Teil B.

Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md](superpowers/specs/2026-10-23-phase-3b4-import-checks-design.md). Plan 3b.4a: [docs/superpowers/plans/2026-10-24-phase-3b4a-checks.md](superpowers/plans/2026-10-24-phase-3b4a-checks.md). Plan 3b.4b: [docs/superpowers/plans/2026-10-24-phase-3b4b-import.md](superpowers/plans/2026-10-24-phase-3b4b-import.md). Recherche: [docs/research/2026-10-23-3b4/](research/2026-10-23-3b4/).

Der Teil läuft in zwei Plänen mit je eigenem Gate. Prüfungen brauchen keinen Import, das Gesamtgate braucht beides.

**3b.4a „Prüfen und Beheben“** (Gate A vom Agenten bestanden, Blick beim Nutzer offen):
- Kernel-Modul `checks` mit 39 Prüfungen (Welt, Scripts, Texte, Daten), jede mit stabilem Code, deutscher Meldung mit Ausweg und Navigationsziel. Harte Regel: „Fehler“ nur bei 0 Treffern auf Platin DE, HeartGold DE und HeartGold US, gated bewiesen; Grenzen und Ausnahmen mit Quelle in `check_limits.yaml` (Dialogzeile 216 px aus pret statt rotoms 200 px, nur für Dialogarchive).
- Snapshot unter dem Store-Lock (gemessen unter 5 ms), voller Lauf ohne Lock mit Fortschritt, Text- und Script-Prüfungen je Datei gecacht; der volle Lauf kommt beim Öffnen, auf „Erneut prüfen“ und 300 ms nach jeder Änderung (warm rund 250 bis 290 ms).
- Vier Fixes mit Vorschau, einem Schritt im Verlauf und Nachprüfung: Umbrechen (`text.too-wide`), Nachricht anlegen (`script.message-missing`), Tauschen (`encounter.min-max`), Script anlegen (`event.script-missing`, nicht rückgängig); ein veralteter Befund wird abgelehnt („Befund ist nicht mehr aktuell“). Warp-Ziele und fehlende Trainer behebt man im Inspector.
- Ausblenden von Warnungen und Hinweisen, „Ausgangslage übernehmen“ und Schalter je Prüfung in `g4-checks.json` im Projekt (Fingerprint je Stelle: ändert sie sich, meldet sich der Befund wieder); Fehler nie.
- Oberfläche: Prüflicht mit drei Zählern und Fortschritt, Panel „Probleme“ mit Gruppen nach Bereich und Code und den Tasten „Öffnen“, „Beheben…“ (Fix-Dialog mit Vorher/Nachher), „Ausblenden“, Fuß „N ausgeblendet · Wieder melden…“ und „Erneut prüfen“; dieselbe Tabelle in Technik › Prüfen und Bauen; Inspector-Abschnitt „Prüfungen“; Fehlerfarbe an Warp-Markern; Markierungen in Explorer-, Trainer- und Begegnungslisten. Die Begegnungswarnungen aus 3b.3b sind jetzt Prüfungen (`encounter.*`).
- Befehle `run_checks`, `checks_status`, `plan_fix`, `apply_fix`, `ignore_finding`, `unignore_finding`, `accept_baseline`, `set_check_enabled` in Tauri und Brücke; `g4k check --expect`; Erwartungen `{expect: {path, equals}}` an den Änderungen eines Szenarios; Szenario `pt-pruefungen.yaml`.

Gate A: `vanilla_checks` gated grün (Soll je Code und Projekt, Fehler 0 überall); auf einer Kopie zählt `run_checks` vier absichtliche Fehler (Warp ohne Ziel-Warp, Figur mit fehlendem Script, Trainer-Event ohne Trainer, zu lange Zeile) je Code 1 und nach Fixes und Edits 0; Prüflicht, Panel, Fix-Dialog und „N ausgeblendet · Wieder melden“ im Browser-Modus über Rollen; `scripts/gate.sh` grün. Ablauf: [docs/phase-3b4-gate.md](phase-3b4-gate.md), Teil A.

**3b.4b „Import und Gesamtgate“** (Gate B vom Agenten bestanden: Szenarien, Browser-Zeilen (B5) bis (B7) und `scripts/gate.sh`; Blick und Hand beim Nutzer offen):
- Import fertiger Binärteile in bestehende Karten über `edit_map` (ein Schritt im Verlauf): NSBMD (TEX0 wird entfernt, genau ein Modell, höchstens 0xF000 Byte), `.per`, `.bld`, `.bdhc`, `.bgs` (nur HGSS), ganze Map-`.bin` auch zwischen Platin und HGSS, „Teile aus Karte N übernehmen“; Export der Gegenstücke. PDSMS-Exporte gehen Karte für Karte über die Mehrfachauswahl desselben Stamms; `.pdsmap` wird nicht gelesen.
- Dialog „Teile importieren…“ mit Prüfliste im Inspector der Karte und in der Palette: je Teil ein deutscher Satz mit Ausweg aus dem Code der Teilprüfung (`PartCode`), nach dem Import die Befunde der Karte mit „Zeigen“; „Teile exportieren…“; `g4k check-data` prüft Modell und Kollision einer Karte im gebauten ROM (`map:N:model_sha1`, `map:N:per_sha1`).
- Szenarien `pt-kleiner-hack.yaml` und `hg-kleiner-hack.yaml` als Fortsetzung der Szenarien von 3b.3b, mit den Gegenproben `*-kleiner-hack-ohne-import.yaml`; `g4emu scenario` kann dafür Dateihashes `${sha1-of:PFAD}`, erwartete Ablehnungen `{expect: {error: …}}` und `prepare:` (synthetisches Modell über 0xF000).
- Die Restpunkte von 3b.4a (Ruling R17): kein Script nach Abbruch von „Speichern und anlegen“, ein Prüflauf danach, Vorzeichen der Vorschau bleiben beim seitlichen Rollen sichtbar, ein einzelnes „Ausblenden“ klappt die Liste „Ausgeblendet“ nicht zu, deutsche Sätze statt englischer Kernel-Texte, die Kommentare zum „billigen“ Lauf.

Gate B ist das Gate für Phase 3b insgesamt: Ein kleiner Hack (neue Route mit **importierter Karte**, zwei neue Trainer, neue Texte, ein Ereignis-Script) wird komplett im Studio gebaut und gespielt. Die Prüfungen sind dabei **vorher rot und nachher grün** (Befundzahlen je Code im Brücken-Batch), das Modell im ROM gleicht der exportierten Datei, und `g4emu` läuft den Weg, den nur die importierte Kollision freigibt. Stand 2026-10-25: beide Szenarien grün (Platin DE: neuer Header 593 mit Karte 666, darin Modell und Kollision von Karte 6, der Weg nach (5, 17) nur mit der importierten Kollision, zwei Trainer gewonnen, Mewtu Lv 5 im Gras; HeartGold DE: Header 540 mit Karte 676, darin Modell und Kollision von Karte 2, Weg nach (20, 24); HeartGold US ebenso), vier Prüfungen je Lauf rot und danach grün, `g4k check-data` findet Modell und Kollision der neuen Karte im ROM; die Browser-Zeilen (B5) bis (B7) und `scripts/gate.sh` (2734 s) sind am selben Tag bestanden. Ablauf: [docs/phase-3b4-gate.md](phase-3b4-gate.md), Teil B.

Nicht in 3b.4 (Spezifikation §1, die Liste gilt wörtlich, damit der Umfang nicht wandert): kein OBJ- oder glTF-Import über apicula (apicula liest nur NSBMD, einen quelloffenen Weg von OBJ zu NSBMD gibt es außer DSPREs AGPL-Code nicht), kein Baking aus PDSMS-`.pdsts` und kein PDSMS-Ordnerimport über `.pdsmap`, kein NSBMD-Writer, kein PNG → NSBTX, keine BDHC-Erzeugung und Kollisionsableitung: alles Phase 6. Flags und Variablen, Geschichten und Story-Graph sowie das Löschen von Daten mit Referenzsperre: 3d (die Spezifikation 3b.3 hatte das Löschen 3b.4 zugeordnet). Prüfungen des Build-Graphs: Phase 4. Später: `script.rotom-all`, Flag- und Variablenbereiche, Typen der Script-Parameter und Script → Trainer/Item (`script.trainer-target`).

## Phase 3c: Doppeldeck

Status: Gate vom Agenten bestanden (2026-10-24, (A) bis (G) über die Test-Umgebung, sieben Befunde im Branch der Abnahme behoben); beim Nutzer offen sind der Blick auf die Screens und auf Dunkel/Kompakt, rotom-lsp und die gepackte App (H). 3c.1 und 3c.2 sind gebaut und automatisch getestet (`scripts/gate.sh` mit den Schritten `tokens` und `styles`, die Ausnahmeliste der Stil-Prüfung ist leer); die Ergebnisse stehen in [docs/phase-3c-gate.md](phase-3c-gate.md). Eingeschoben zwischen 3b.2 und 3b.3, damit die Daten-Editoren von 3b.3 gleich auf der neuen Komponentenbibliothek aufsetzen; 3b.3a beginnt nach dem Gate.

Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md](superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md). Pläne: [3c.1](superpowers/plans/2026-10-23-phase-3c1-doppeldeck-fundament.md) und [3c.2](superpowers/plans/2026-10-23-phase-3c2-doppeldeck-umzug.md). Gestaltungsquelle: [design-system/README.md](../design-system/README.md).

Lieferumfang:
- 3c.1 „Fundament und Welt“: das Design-System im Repo (`design-system/`, Tokens als eingecheckte Dateien in `apps/studio/src/ui/tokens/`), Paket `ui/` mit Base-UI-Primitives und Galerie (`?gallery`), der Rahmen (Titelleiste, Atlas, Liste, Deck aus Schau und Hand mit Scharnier, Statusleiste, Toasts), der Bereich Welt im neuen Raster mit Werkzeugen mit Buchstabenkürzeln, Tiefe Bauen und Tüfteln im Inspector, Szenenfarben aus den Tokens, Verlauf im Scharnier (Befehl `history`), die Gate-Schritte „tokens“ und „styles“.
- 3c.2 „Texte, Scripts, Technik, Assistenten“: Bereich Texte mit Archivliste, DS-Bildschirm-Vorschau und Nachrichtentabelle, Script und Probleme im neuen Stil (CodeMirror-Thema aus den Tokens), Assistenten als Schrittleiste im Scharnier, Start mit den zuletzt geöffneten Projekten, Technik (Prüfen und Bauen, Patches, Emulator, Werkzeuge) und Einstellungen (Thema mit „Wie System“, Dichte, Emulator) als Flächen, Befehlspalette ⌘K, Tüfteln auch im Raster, Layout-Schlüssel `v8`, Ausnahmeliste der Stil-Prüfung leer, Gate-Ablauf in [docs/phase-3c-gate.md](phase-3c-gate.md).

Gate: Spec §1 (A) bis (H): der Screen „Welt bauen: Figur bearbeiten“ ist wiedererkennbar, Texte, Scripts und Assistent laufen im neuen Raster, dunkles Thema und Kompakt sind lesbar, Tastaturbedienung, die Abnahmen 3a, 3b.1 und 3b.2 über die Test-Umgebung und `scripts/gate.sh` grün; die gepackte App prüft der Nutzer von Hand. Ablauf: [docs/phase-3c-gate.md](phase-3c-gate.md).

Nicht in 3c: die Bereiche Geschichten, Pokémon und Gegenstände (gesperrt im Atlas), Tiefe Erzählen, Live-Vorschau der Assistenten in der Schau, Story-Graph, Spielmodus mit Live-Flags, Veröffentlichen, Plugins, Gelände malen mit Draufsicht und Muster im Overlay (siehe „Aus dem Design-System, später“).

## Ausbau nach Phase 3: Phasen 3e bis 3h

Nach dem Abschluss von Phase 3 kam eine Wunschliste des Nutzers: sortierbare Orte und eigene Orte, leichtere Dynamische Header und Patches, ein Projektstart aus der `.nds` mit Vorlagen je Edition, die lokale Test-API als MCP-Server für alle Nutzer, und eine Unity-artige Werkbank (schwebende Werkzeugleiste, Hierarchie getrennt vom Inspector, Scharnier über den Reitern, frei anordenbare Panels, Raster und Header hinter Weltkarte › Gebiete › Räume verborgen). Drei Gutachten haben die schwierigen Fragen an den drei Vanilla-Projekten geprüft; die Entscheidungen stehen in [docs/research/2026-10-25-ausbau/entscheidungen.md](research/2026-10-25-ausbau/entscheidungen.md), die Gutachten daneben. Die vier Phasen laufen vor Phase 4 in der Reihenfolge 3e → 3f → 3g → 3h (3g.1 kann parallel zu 3f laufen). Jede Teilphase bekommt vor der Umsetzung eine eigene Spezifikation (Brainstorming) und einen Plan; jede UI-Teilphase erweitert das Design-System (`design-system/README.md`, Seite „App: Werkbank“ in `Editor_DesignSystem.fig` per OpenPencil, `fix_fig_text.mjs --rgb555`) mit dem Frontend-Design-Skill. Phase 3d (Erzählen, Flags und Variablen, Geschichten, Löschen mit Referenzsperre) bleibt wie bisher „später“; ob sie nach 3h oder nach Phase 4 kommt, ist offen.

## Phase 3e: Werkbank

Status: umgesetzt und abgenommen (2026-10-25, Branch `phase-3e-werkbank`). 3e.1 bis 3e.4 umgesetzt, Agent-Teil der Gates bestanden ([docs/phase-3e-gate.md](phase-3e-gate.md), Teile 3e.1 bis 3e.4), Abnahme 3e bestanden: die Agent-Punkte von 3a, 3b.1, 3b.2, 3c, 3b.3 und 3b.4 liefen über die Test-Umgebung und die Szenarien mit den Formulierungen der Tabelle „vorher → jetzt“ erneut grün, sieben Befunde behoben; `scripts/gate.sh` grün im Worktree (3168 s auf `572410b`). Beim Nutzer offen: die Blicke (Galerie „Rahmen“, Screen „Werkbank“ 1440 × 900, Maximieren und schwebender Inspector, 1100 × 720 Kompakt in Hell und Dunkel, dazu die Blicke der älteren Gates) und die Hand-Punkte (Reiter mit der Maus ziehen, DSPRE, rotom-lsp, gepackte App), alle im Abschnitt „Abnahme 3e“ des Gate-Dokuments aufgelistet. Gutachten: [docs/research/2026-10-25-ausbau/gutachten-b-werkbank.md](research/2026-10-25-ausbau/gutachten-b-werkbank.md) (Variante C, Hybrid) und [gutachten-a-weltmodell.md](research/2026-10-25-ausbau/gutachten-a-weltmodell.md) (Baum und Sortierung). Das Doppeldeck entwickelt sich weiter: Prinzip 1 wird „Zeigen und anfassen“ (in der Welt steht die Hand an den Rändern, in Texten und Daten unter der Schau), Prinzip 2 bleibt mit dem Scharnier als Kopfleiste über der Schau; der Name „Scharnier“ und die Tokens `scharnier/*` bleiben.

Lieferumfang in vier Teilphasen, jede für sich lauffähig mit eigenem Layout-Schlüssel (`v9`, `v10`, `v11`) und Gate über die Test-Umgebung:
- **3e.1 Scharnier oben, Teiler, Werkzeugleiste** (S): das Scharnier als Kopfleiste über dem Dock (ohne Griff, mit „Schau maximieren“ ab 3e.4), ein eigener Teiler zwischen Schau und Hand (`role="separator"`, Pfeiltasten, Doppelklick klappt ein, `dock-welt-hand` bleibt), die Werkzeuge als schwebende Leiste in der 3D-Ansicht (Standard senkrecht links, ziehbar an drei Anker, unter 400 px Höhe waagrecht (umgesetzt: unter 460 px in Komfort, 400 px in Kompakt, damit die Leiste die Legende nicht berührt), `role="toolbar"` und `data-tool` bleiben; Buchstaben wirken nur bei sichtbarem 3D); Galerie „Rahmen“ nachgezogen; Design-System: Prinzipien, Fensterraster, Komponenten „Werkzeugleiste schwebend“, „Teiler“, Token `schatten/schwebend`.
- **3e.2 Weltbaum und Sortierung** (M, nur Frontend): `worldModel.ts` (Weltkarten = Matrizen mit Header-Abschnitt und mindestens zwei Ortsnamen, in HGSS Johto und Kanto als zwei Weltkarten über `kanto_flag`; Gebiete = Zellen eines Ortsnamens plus eigenständige Außen-Matrizen unter „Weitere Gebiete“; Räume über den Warp-Graphen dem Gebiet ihres Eingangs zugeordnet, sonst „Nicht verbunden“; Landschaft = Zellen mit Header 0; Verwaistes nur in Tüfteln); der Baum Weltkarte › Gebiet › Räume mit Ortsetikett und Ortsfarbe, Klarnamen für Räume aus einer Heuristik (umbenennbar), die Projektdatei `g4-world.json` nur für Namen der Weltkarten, Klarnamen und manuelle Zuordnungen; Sortierung im Baum (Spielreihenfolge, Name A–Z, Zuletzt gewählt) und in der Liste (Nummer, Name, Kürzel) als `worldSort` in `g4.shell.v1`; Glossar mit `worldMap`, `area`, `place`, `room`, `map`, `landscape`; in Bauen heißt nichts mehr „Raster“, „Matrix“ oder „Header“ (rund 37 Zeichenketten und 34 `TERMS.matrix`-Stellen), der Reiter „Raster“ selbst bleibt bis 3f.1.
- **3e.3 Hierarchie und Inspector** (M): die Liste „Welt“ mit zwei Abschnitten und Teiler: oben „Orte“ (einklappbar zu einer Ortszeile mit Picker), unten „In dieser Szene“ (`role="tree"`: Header, Karten mit Gebäuden, Figuren, Türen mit Ziel, Auslöser, Schilder, Scripts; Suche; Enter auf einer Tür folgt ihr; „Zurück“ zur vorigen Szene); „An diesem Ort“ und die Ausgangsliste des Header-Editors wandern dorthin; der Inspector zeigt nur das Gewählte mit Kopf (Typ-Wort, Titel, Kennung, Plaketten, Aktionsmenü mit „Teile importieren…“); die Hover-Ansichten gehen in die Statusleiste; Selektoren `data-event`, `data-building`, `data-header-id`, `data-place-key`, `data-script-file` bleiben, neu `[data-scene-group]`; Design-System: „Hierarchie“, „Baumzeile“ erweitert, „Inspector-Kopf“.
- **3e.4 Ein Dock je Bereich** (M): eine Dockview-Instanz je Bereich (`data-dock`, `data-dock-group`, `data-dock-panel`), Reiter immer sichtbar, Panels ziehbar, schwebend (`floatingGroupBounds`, Titelleiste als Griff) und maximierbar (Doppelklick auf den Reiter, ⤢ im Scharnier); die Schau-Panels 3D, Raster, Weltkarte bleiben in der Mitte und schweben nicht, kein Ausklappen in Fenster; Vorlagen „Werkbank“ (Welt: Schau Mitte, Inspector rechts 340 px in voller Höhe, Script und Probleme unten 240 px) und „Doppeldeck“ (Texte, Pokémon, Gegenstände: Schau oben, Bearbeiten unten) über die Palette, dazu „Layout zurücksetzen“, „Inspector schweben lassen“, „andocken“, „Schau maximieren“; deutsche Dockview-Ansagen (`messages`, Kontextmenü); eigene F6-Runde durch Atlas, Liste, Scharnier, Dock-Gruppen, Statusleiste; der Assistenten-Schritt liegt als Streifen unter dem Dock; `handHeight` und „Script hebt die Hand auf 400 px“ werden `setSize` an der Gruppe; Design-System: „Dock-Reiter“, „Schwebende Gruppe“, Muster „Anordnung“, Seite „App: Werkbank“ in der `.fig`.

Gate: Der Screen „Welt bauen: Figur bearbeiten“ ist in der Anordnung „Werkbank“ wiedererkennbar (Scharnier oben, Reiter darunter, Werkzeugleiste in 3D, Hierarchie links mit „Orte“ und „In dieser Szene“, Inspector rechts mindestens 600 px hoch bei 900 px Fensterhöhe); ein DOM-Scan in Bauen findet in Liste, Scharnier, Schau und Hand kein sichtbares „Raster“, „Matrix“ oder „Header“; der Baum zeigt auf Platin DE die Weltkarte Sinnoh, auf HeartGold DE und US Johto und Kanto, Großmoor und Safari unter „Weitere Gebiete“, mindestens 90 % der Räume unter einem Gebiet, den Kraterberg unter seinem Eingang (gated Test); Sortierung Name A–Z stellt „Route 2“ vor „Route 10“; eine Figur wird in „In dieser Szene“ gewählt, geändert, ⌘Z, ⌘S; der Inspector lässt sich per Ziehen und per Palette schweben und andocken, das Layout überlebt das Neuladen; three.js zeichnet nach Maximieren und Schweben ohne Verzerrung (Bild); Screenreader-Ansagen deutsch; F6-Runde; die Abnahmen 3a, 3b.1 bis 3b.4 und 3c laufen über die Test-Umgebung mit den neuen Formulierungen erneut grün (Tabelle „vorher → jetzt“ im Gate-Dokument); `scripts/gate.sh` grün. Bilder in 1440 × 900 und 1100 × 720, Dunkel und Kompakt als Blick des Nutzers.

Nicht in 3e: der Reiter „Raster“ (3f.1), eigene Orte (3f.2), eine frei andockbare Hierarchie und eigene Werkzeugleisten für Raster und Weltkarte (optional 3e.5 „Feinschliff“: Anker merken, Kantengruppe als Spike), Ausklappen in eigene Fenster.

## Phase 3f: Weltmodell

Status: geplant, setzt 3e.2 voraus. Gutachten: [docs/research/2026-10-25-ausbau/gutachten-a-weltmodell.md](research/2026-10-25-ausbau/gutachten-a-weltmodell.md) (Variante B).

Lieferumfang:
- **3f.1 Raster nach Tüfteln** (L): die Weltkarte übernimmt Zellen-Auswahl, „+“ und Kartenbilder mit einem Umschalter der Weltkarten, mehrzellige Räume bekommen einen Raumplan (eigene Matrix, 64 px); in 3D wählt ein Klick auf den Boden die Karte, die Kamera rahmt das Gebiet, nur Gebiet plus Nachbarn werden geladen (heute alle 468 bzw. 492 Karten der Matrix 0), Draufsicht-Kamera; der Reiter „Raster“ (`MatrixPanel`: Name, Größe, Abschnitte, „Neue Matrix“) erscheint nur in Tüfteln; Brotkrume, Statusleiste und Palette in Nutzerworten.
- **3f.2 Eigene Orte** (M): Kernel `create_place`, `rename_place`, `assign_place` (Nachricht an die Ortsnamen-Bank anhängen, Platin Archiv 433, HGSS 279; `location_name` mehrerer Header in einem Undo-Schritt; Ortsnamen aus dem Store statt von der Platte), Prüfungen (Index ≤ 255, Warnung ab 17 Zeichen, HGSS mit 21 freien Plätzen und Wiederverwendung ungenutzter Sinnoh-Namen mit Warnsatz); Inspector „Ort“ als Picker, „Neuer Ort…“ im +-Menü und im Assistenten; `docs/formats/map_header.md`.
- **3f.3 Leichter Patch** (S bis M): im Assistenten der Schritt „Ort“ statt „Header“, der Header wird immer neu angelegt (interner Name automatisch), der Patch „Dynamic Headers“ wird einmal in der Zusammenfassung bestätigt (Plakette „Nicht rückgängig“); der Kernel wendet ihn auch bei ungespeicherter Header-Tabelle an und legt vorher einen Schnappschuss von `unpacked/dynamicHeaders` an (`allow_patch`); Technik › Patches bleibt für Reparatur und Status.
- **3f.4 Nahtlos und Abspalten** (optional, M): `create_area` mit Ziel „bestehende Weltkarte“ (der Assistent wählt freie Zellen, der Ortsname wechselt ohne Warp), `split_room` für geteilte Räume (bis zu 33 Räume auf einer Karte; bis dahin Plakette „Geteilt“).

Gate: Auf Kopien von Platin DE und HeartGold DE entsteht in Bauen, ohne den Reiter „Raster“ zu öffnen und ohne Technik zu besuchen, ein Gebiet mit dem neuen Ort „Neuheim“ in einem Durchgang (Patch mit einer Zustimmung, auch bei einer ungespeicherten Header-Änderung, deren Undo danach noch geht), das Gebiet wird um eine Karte erweitert, in die neue Karte werden Teile importiert und eine Höhe gesetzt; in melonDS erscheint beim Betreten die Einblendung „Neuheim“, ein dort gefangenes Pokémon zeigt den Fundort; DSPRE zeigt Name und Patch (Hand); `vanilla_checks` behält 0 Fehler, die Ladezeit von Zweiblattdorf ist kleiner als vor 3f.1 (gemessen); `scripts/gate.sh` grün. Mit 3f.4: eine neue Route grenzt an Route 201 und ist ohne Warp begehbar (`g4emu` läuft hinüber, der Ortsname wechselt), ein abgespaltenes Pokémon-Center ändert nur einen Ort.

Nicht in 3f: Stadtkarte und Flugziele neuer Orte (eigener Recherche-Posten: Overlay-Tabellen, Patch), Editor-Ordner oder Tags (bei Bedarf später als Feld in `g4-world.json`), Gelände malen (Phase 6).

## Phase 3g: Projektstart

Status: geplant, unabhängig von 3e und 3f (3g.1 kann parallel laufen). Gutachten: [docs/research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md](research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md), Teil A. Der Kernel entpackt eine ROM schon (`extract_rom` über ds-rom, `g4k extract`; Platin DE in 0,49 s, HeartGold DE in 0,37 s; arm9, Overlays, `files` und `header.yaml` byte-gleich zu DSPRE-Ordnern, nur `padding:` in `config.yaml` fehlt).

Lieferumfang:
- **3g.1 Neues Spiel aus ROM** (M): Start-Screen mit dem Hauptknopf „Neues Spiel aus ROM…“ („Du brauchst deine eigene ROM-Datei“) und Drag-and-drop einer `.nds`; Assistent im Scharnier: ROM wählen (`inspect_rom`: Spielcode, Sprache, Freigabe CPUD, IPKD, IPKE, Diamant/Perl gesperrt mit Grund), Vorlage (Original, Leere Region mit Startstadt ab 3g.2, Nur Daten), Name und Zielordner (Vorschlag `~/Documents/Gen4 Studio/<Name>_DSPRE_contents`, änderbar; Prüfung leer oder fehlt, freier Platz), Zusammenfassung, Anlegen mit Fortschritt (`create_project`: Lesen, Entpacken, Vorlage, Prüfen), Ergebnis „Öffnen“; Marke `.g4/base.json` mit SHA-1 der Basis-ROM und `origin` (Phase 4 braucht sie für xdelta); Brückenbefehle und ein Weg ohne die Suffix-Sperre für Agenten-Tests.
- **3g.2 Leere Region mit Startstadt** (M): das Rezept `templates::apply` als ein Store-Schritt: alle Header-IDs bleiben, Matrix 0 behält die Startstadt plus eine gesperrte Rand-Karte, Außen-Header bekommen leere Eventdateien und keine Level-Scripts, Scripts und Texte werden nur nicht mehr verwiesen, Start-Header und Spielerhaus bleiben, ein Start-Script als rotom-Vorlage gibt den Starter und setzt Pokédex- und Laufschuh-Flags; derselbe Schritt als Assistent „Welt leeren…“ im offenen Projekt (Technik › Werkzeuge); nach dem Rezept meldet `run_checks` 0 Fehler.

Gate: Ein frisch aus der ROM angelegtes Projekt (Original) baut byte-gleich zur Eingabe-ROM (gated), die Browser-Zeilen des Assistenten laufen über die Test-Umgebung, DSPRE öffnet den Ordner (Hand); die leere Region startet in `g4emu` auf Platin DE und HeartGold DE: Intro, Aufwachen zu Hause, Starter aus dem Start-Script, kein Weg aus der Startstadt außer den Rand-Karten, `run_checks` 0 Fehler; `scripts/gate.sh` grün.

Nicht in 3g: eigener Startort und übersprungenes Intro (Spike zu ARM9-Patches), ROM-Dateien im Repo oder in Tests außerhalb der gated Läufe, Diamant/Perl.

## Phase 3h: Agenten-Schnittstelle (MCP)

Status: geplant. Gutachten: [docs/research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md](research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md), Teil B. Die Entwickler-Brücke (`studio-bridge`, 91 Befehle, Feature `dev-bridge`) wird zur Produkt-Schnittstelle nach dem Model Context Protocol, für Coding-Agenten wie Claude Code; sie braucht den Plugin-Host (Phase 5) nicht, Phase 5 baut später auf derselben Befehlsschicht auf.

Lieferumfang:
- **3h.1 Werkzeugschicht und kopflos** (M): `dispatch` aus dem Feature nach `commands/dispatch.rs`, `Roots` wird `Policy` (`DevPolicy`: `_testkopien` wie heute; `UserPolicy`: nur das offene Projekt, Build-Ziele unter `<Projekt>/build/` oder freigegebene Ordner); `mcp::tools` mit rund 30 kuratierten englischen Tools (Schemas, Ressourcen), `g4_batch` (ein Undo-Schritt über `Store::transaction`) und `g4_call` (alle Befehle, Expertenmodus, Standard aus); `studio mcp` (stdio, `rmcp`) kopflos mit `--project`, nur lesend; Einrichtung in einer Zeile (`claude mcp add gen4-studio -- <studio> mcp`), die die App mit Kopierknopf zeigt.
- **3h.2 Schreiben und Batch** (M): schreibende Tools kopflos mit `UserPolicy`, Bestätigungspflicht für Unumkehrbares (Patch, Trainer anlegen, Script anlegen, Bauen außerhalb `build/`, Emulator, Ausgangslage; kopflos `--allow-irreversible`); Gate-Schritt `mcp`.
- **3h.3 In der laufenden App** (M): Einstellungen › Agenten („Lokale Agenten zulassen“, Standard aus) startet Streamable HTTP auf `127.0.0.1` (Token je Start, Entdeckungsdatei im App-Datenordner mit Rechten 0600, Host- und Origin-Prüfung); `studio mcp` reicht an die laufende App durch; Freigabe je Projekt in den App-Einstellungen („nicht / lesen / lesen und bearbeiten“, nach kanonischem Pfad und `.g4/base.json`-ID, nie im Projektordner), Nachfrage beim ersten Verbinden; Statusleiste „Agent verbunden: <Client>“ mit „Trennen“ und „Nur lesen“; `Store::Change` mit `origin: User | Agent { client }`, der Verlauf zeigt es, ⌘Z nimmt Agenten-Schritte zurück; nach jedem schreibenden Aufruf `agent-changed` mit den Assets und der `reloadPlan` wie nach Undo; die Brücke bekommt `/mcp` auf derselben Sitzung, die Agenten-Testumgebung wird Kunde des MCP-Servers.
- **3h.4 Politur** (optional): Ressourcen für Doku und Glossar, Tool-Beschreibungen aus den Nutzerworten (Gebiet, Raum, Ort), Beispiele.

Gate: Claude Code verbindet sich per `claude mcp add` kopflos mit einer Testkopie und liest Header, Prüfungen und Texte (3h.1); ein `g4emu`-Szenario legt kopflos über MCP eine Route mit Trainer und Text an, baut und begeht sie, ein Batch ist ein Undo-Schritt (3h.2); über `/mcp` der Brücke legt der Agent im Browser-Modus ein Gebiet an, die Szene zeigt es ohne Neuladen, der Verlauf markiert den Schritt mit dem Client-Namen, ⌘Z nimmt ihn zurück, „Nur lesen“ sperrt, `g4_call` ist im Release standardmäßig aus (3h.3); in der gepackten App arbeiten Claude Code und die App im selben Projekt (Hand); `scripts/gate.sh` mit dem Schritt `mcp` grün.

Nicht in 3h: Plugins (Phase 5), Zugriff von anderen Rechnern (nur Loopback), `open_project` und Schließen für Agenten in der App, Agenten-Zugriff auf Originale.

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
- Aus 3b.4 hierher verschoben (Spezifikation 3b.4 §1): OBJ- und glTF-Import, Baking aus PDSMS-`.pdsts` und PDSMS-Ordnerimport über `.pdsmap`, PNG → NSBTX. 3b.4b importiert nur fertige Binärteile (NSBMD, `.per`, `.bld`, `.bdhc`, `.bgs`, Map-`.bin`).
- NSBMD/NSBTX-Writer in `g4-formats`, Round-Trip gegen apicula-Lesepfad und Vanilla.
- Textuelles Weltformat: Karte als JSON (Tiles, Höhen, Gebäude, Kollision) plus glTF für Freiform-Geometrie; Compiler zu Kartendatei.
- 2D-Top-Down-Modus für schnelles Arbeiten.

Gate: Eine im Studio gemalte Karte wird gebaut, in eine Matrix gesetzt und im Emulator begangen.

## Aus dem Design-System, später

Das Design-System „Doppeldeck“ ([design-system/README.md](../design-system/README.md)) zeigt mehr, als 3c baut. Was bewusst später kommt (Spec 3c §13):

- Tiefe „Erzählen“ mit Rezepten und Live-Vorschau der Assistenten (nach 3b.3, als 3d).
- Bereiche Pokémon und Gegenstände (seit 3b.3a offen, Trainer und Begegnungen seit 3b.3b; Läden später, siehe unten).
- Hand-Höhe je Unterbereich (Spec 3b.3 §9; die Hand merkt ihre Höhe je Bereich, das wartet auf den Umbau des Decks).
- Flags und Variablen sowie Geschichten und Story-Graph (3d; bis zur Spezifikation 3b.4 standen sie bei 3b.4, der Atlas-Eintrag „Geschichten“ sagt seit 3b.4a „Kommt mit Phase 3d“).
- Prüfen und Bauen mit Build-Schritten, Spielmodus live (Live-Flags, bernsteinfarbenes Scharnier) und Veröffentlichen (Phase 4).
- Plugins im Bereich Technik (Phase 5).
- Gelände malen, Höhen-Overlay, Draufsicht-Kamera und Overlay-Muster (Phase 6).

Aus Phase 3b.3 (Spec 3b.3 §1, „Später“; die Tabelle gilt wörtlich, damit der Umfang nicht wandert):

| Thema | Wohin | Grund |
|---|---|---|
| Ei-Attacken | 3d oder später | Pt im Overlay 5, HGSS als NARC: zwei Mechaniken |
| Attacken-Lehrer, Läden (Sortiment, Orden-Staffel), Honigbäume, Großmoor, Kopfnuss, Safari, Käferturnier | später (Overlay-Tabellen, Patch-Layer) | je Spiel verschieden, braucht Patch-Schutz |
| Assistent „Neuer Trainer“ mit Live-Vorschau; Erzählen-Layouts für Begegnungen und Läden | 3d (Erzählen) | Erzählen und Live-Vorschau sind in 3c ausgeschlossen |
| DS-Bildschirm-Varianten (Pokédex-Profil, Kampfvorschau, Laden) | 3d | Schau-Kosmetik ohne Beweiswert |
| Kampfsprites von Pokémon und Trainern | 3d | verschlüsselte NCGR, Pt und HGSS verschieden (offen) |
| Pokémon-Menüsymbole | 3b.3a, wenn Plan-Task 0 die Paletten-Tabelle bestätigt, sonst 3d | Kandidat: arm9 Pt DE 0xF07D8, HG DE 0xFFBC4, HG US 0xFFC10 (Muster gefunden, Bedeutung ungeprüft) |
| arm9-Tabellen schreiben (Item-Tabelle, TM-Liste), Klassen-Geschlecht, Preisfaktor, Eye-Music, Trainername > 7 Zeichen | später | Schreiben in arm9 braucht Patch-Schutz; Offsets teils nur aus DSPRE |
| Projektprüfungen (Trainer ohne Party, leere Ersatz-Slots projektweit) | 3b.4a (gekommen: `trainer.no-party`, `encounter.empty-slot` und die übrigen `encounter.*`) | Roadmap |
| Löschen von Daten mit Referenzsperre | 3d | Spezifikation 3b.4 §1; Löschen ohne Script-Referenzen wäre ein leeres Versprechen |
| Script → Trainer/Item (Referenzen aus Scripts prüfen) | später | Spezifikation 3b.4 §4 („später“: `script.trainer-target`); rotom prüft keine IDs |
| Pokédex-Größe/-Gewicht, Wachstumstabellen, „Auf Original zurücksetzen“ | Phase 4 (Basis-ROM-Vergleich) | Zukan-Daten, Basis-ROM |
| Diamant/Perl | kein Gate | kein Testprojekt; die Metadaten kennen die Pfade, der Editor sperrt DP mit Grund |

Stand nach 3b.3a: Plan-Task 0 hat die Paletten-Tabelle bestätigt, die Pokémon-Menüsymbole kamen mit 3b.3a (Pokédex-Liste, Auswahl der Arten).

## Querschnitt (laufend ab Phase 1)

- Prüfung: `scripts/gate.sh` fährt lokal `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm test`, die Token-Prüfung (`scripts/check-tokens.sh`), die Stilprüfung (`scripts/check-styles.sh`: keine Hex-Farben, keine Primitive, keine Inline-Styles), `pnpm build` und die gated Tests gegen die drei Testprojekte (siehe `docs/dev-setup.md`). Es gibt kein Remote und noch keine CI; Builds für macOS (arm64) und Windows (x64), Linux als Best-Effort, folgen mit einem Remote.
- Releases: Tauri-Bundles, signiert, mit Auto-Update ab Phase 3b.
- Dokumentation: `docs/formats/` für jedes Format mit Byte-Layout und Quelle der Information.
- Lizenz-Audit bei jeder neuen Abhängigkeit (`cargo deny`).
