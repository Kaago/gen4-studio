# Gen4 Studio: Roadmap

Stand: 2026-10-24. Spezifikation: [docs/superpowers/specs/2026-10-21-gen4-studio-design.md](superpowers/specs/2026-10-21-gen4-studio-design.md). Analyse: [docs/research/2026-10-20-dspre-und-suite-analyse.md](research/2026-10-20-dspre-und-suite-analyse.md).

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
| 3b.3 | Daten-Editoren in zwei Teilen: 3b.3a Codecs, Pokédex, Attacken, Gegenstände; 3b.3b Trainer, Begegnungen (3b.3a: Gate Teil A vom Agenten bestanden, Blick beim Nutzer offen) | Neue Trainer und Encounter komplett im Studio |
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

Status: 3b.3a: Gate Teil A vom Agenten bestanden (2026-10-24, (A1) bis (A7) über die Test-Umgebung, `scripts/gate.sh` grün, drei Befunde im Branch der Abnahme behoben: die Suche zeichnete Schau und Hand neu und die Pokédex-Liste lag über 50 ms, leere Schau ohne Taste, Pokédex-Farbe „Violett“); beim Nutzer offen ist der Blick auf die drei Editoren gegen ihre Screens, auf die Item- und Pokémon-Symbole und auf Dunkel und Kompakt. Ergebnisse in [docs/phase-3b3-gate.md](phase-3b3-gate.md), Teil A. 3b.3b beginnt nach dem Gate von 3b.3a.

Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md](superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md). Plan 3b.3a: [docs/superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md](superpowers/plans/2026-10-23-phase-3b3a-codecs-pokemon-data.md). Recherche: [docs/research/2026-10-23-3b3/](research/2026-10-23-3b3/).

Der Teil ist zu groß für einen Plan und läuft in zwei Plänen mit je eigenem Gate:

**3b.3a „Codecs und Pokémon-Daten“** (Gate Teil A vom Agenten bestanden, Blick beim Nutzer offen):
- Eigene Codecs in `g4-formats/src/data/` für alle Daten-Archive, auch Trainer und Begegnungen, damit der Round-Trip von Anfang an vollständig ist: Begegnungen (Platin und HGSS), Trainer-Eigenschaften und -Party, Pokémon-Basisdaten, Lernsets, Entwicklungen, Attacken, Gegenstände, Trainer-Texttabellen; dazu NCGR und NCLR für Symbole. Unbekannte Bits bleiben Rohwerte, nur eine falsche Länge sperrt ein Member. uxie 0.9.7 ist nur noch Test-Orakel.
- Encounter-Writer: entschieden, ein eigener Codec. Der Writer von uxie 0.9.7 ist unbrauchbar (leere Slots gelten als ungültig, der HGSS-Leser ist um 8 Byte versetzt, Schwärme werden doppelt geschrieben); ebenso sein Attacken-Codec (lehnt 172 von 471 Vanilla-Attacken ab, schreibt 14 statt 16 Byte).
- Store: die neuen Archivarten als Assets, mehrteiliger Changeset (`Store::transaction`, ein Undo-Schritt über mehrere Assets), Namenstabellen aus den Textarchiven, arm9-Tabellen nur lesend (Item-Tabelle, TM/VM-Liste, Paletten der Pokémon-Symbole; mit Plausibilitätsprüfung, sonst „nicht verfügbar“), Referenzindex (`refs`, inkrementell), Item- und Pokémon-Symbole aus dem ROM. Der Build packt die Daten-Ordner mit, der Vanilla-Rebuild bleibt byte-identisch.
- Generische Befehle `data_list`, `data_get`, `data_edit`, `data_raw` und `name_tables`, `item_table`, `machine_table`, `refs`, `get_item_icon`, `get_mon_icon` in Tauri und Brücke.
- Oberfläche: Atlas-Bereiche Pokémon (Unterbereiche Pokédex und Attacken; Trainer und Begegnungen im Umschalter gesperrt bis 3b.3b) und Gegenstände; Editoren Pokédex (Werte, Fähigkeiten und Typen, Lernset, Entwicklung, TM/VM mit „Von anderer Art übernehmen“), Attacken, Gegenstände; Rohdaten als Hexblock in Tüfteln; neue Primitives `TypeBadge`, `StatBar`, `Table`, `FilterChips`, `ToggleGrid`, `HexBlock`, `Sprite`, Unterbereichs-Menü im Listenkopf.
- Prüfschritt `g4k check-data` und Szenario `pt-daten.yaml` mit Abschnitt `check:`.

Gate 3b.3a: `vanilla_data` grün über alle Member der zehn Archivarten in Platin DE, HeartGold DE und HeartGold US, das uxie-Orakel stimmt für Personal, Items und Trainer-Eigenschaften, der Vanilla-Rebuild bleibt byte-identisch; das Szenario `pt-daten.yaml` ändert Staravia, Flügelschlag und Trank, baut und findet die Werte im gebauten ROM; die drei Editoren sind gegen ihre Screens wiedererkennbar, ⌘Z nimmt „Wie Staralili“ über zwei Assets in einem Schritt zurück, Tüfteln zeigt den Hexblock mit markiertem Feld, die Liste „Gegenstände“ zeigt Symbole aus dem ROM; `scripts/gate.sh` grün. Ablauf: [docs/phase-3b3-gate.md](phase-3b3-gate.md), Teil A.

**3b.3b „Trainer, Begegnungen, Gate“** (Plan folgt):
- Trainer als zusammengesetztes Asset mit Name und Kampftexten, „Neuer Trainer“, Event ↔ Trainer; Begegnungen mit neuer Datei, Anbindung am Header und „Abspalten“; Pokémon-Karte; `g4emu`-Kampfschritte (`expect-battle`, `walk-until-battle`, `win-battle`), zwei Szenarien.
- Hand-Höhe je Unterbereich (in 3b.3a merkt die App sie je Bereich).

Gate (Roadmap-Gate 3b.3): Zwei neue Trainer und geänderte Encounter entstehen komplett im Studio und sind im Spiel zu sehen; `g4emu` liest Art und Stufe der Gegner aus dem RAM. Ablauf: [docs/phase-3b3-gate.md](phase-3b3-gate.md), Teil B (kommt mit dem Plan).

Nicht in 3b.3: siehe die Tabelle „Später“ unter „Aus dem Design-System, später“.

### Phase 3b.4: Import und Prüfungen

Lieferumfang:
- Kartenimport: PDSMS `.pdsmap`/`.pdsts`, NSBMD-Dateien, OBJ über apicula-Pfad; Kollisionsimport.
- Project Checks: kaputte Warps, Scripts ohne Ziel, Trainer ohne Party, Textlängen.

Gate (für Phase 3b insgesamt): Ein kleiner Hack (neue Route, zwei neue Trainer, neue Texte, ein Ereignis-Script) wird komplett im Studio gebaut und gespielt.

## Phase 3c: Doppeldeck

Status: Gate vom Agenten bestanden (2026-10-24, (A) bis (G) über die Test-Umgebung, sieben Befunde im Branch der Abnahme behoben); beim Nutzer offen sind der Blick auf die Screens und auf Dunkel/Kompakt, rotom-lsp und die gepackte App (H). 3c.1 und 3c.2 sind gebaut und automatisch getestet (`scripts/gate.sh` mit den Schritten `tokens` und `styles`, die Ausnahmeliste der Stil-Prüfung ist leer); die Ergebnisse stehen in [docs/phase-3c-gate.md](phase-3c-gate.md). Eingeschoben zwischen 3b.2 und 3b.3, damit die Daten-Editoren von 3b.3 gleich auf der neuen Komponentenbibliothek aufsetzen; 3b.3a beginnt nach dem Gate.

Spezifikation: [docs/superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md](superpowers/specs/2026-10-23-phase-3c-doppeldeck-design.md). Pläne: [3c.1](superpowers/plans/2026-10-23-phase-3c1-doppeldeck-fundament.md) und [3c.2](superpowers/plans/2026-10-23-phase-3c2-doppeldeck-umzug.md). Gestaltungsquelle: [design-system/README.md](../design-system/README.md).

Lieferumfang:
- 3c.1 „Fundament und Welt“: das Design-System im Repo (`design-system/`, Tokens als eingecheckte Dateien in `apps/studio/src/ui/tokens/`), Paket `ui/` mit Base-UI-Primitives und Galerie (`?gallery`), der Rahmen (Titelleiste, Atlas, Liste, Deck aus Schau und Hand mit Scharnier, Statusleiste, Toasts), der Bereich Welt im neuen Raster mit Werkzeugen mit Buchstabenkürzeln, Tiefe Bauen und Tüfteln im Inspector, Szenenfarben aus den Tokens, Verlauf im Scharnier (Befehl `history`), die Gate-Schritte „tokens“ und „styles“.
- 3c.2 „Texte, Scripts, Technik, Assistenten“: Bereich Texte mit Archivliste, DS-Bildschirm-Vorschau und Nachrichtentabelle, Script und Probleme im neuen Stil (CodeMirror-Thema aus den Tokens), Assistenten als Schrittleiste im Scharnier, Start mit den zuletzt geöffneten Projekten, Technik (Prüfen und Bauen, Patches, Emulator, Werkzeuge) und Einstellungen (Thema mit „Wie System“, Dichte, Emulator) als Flächen, Befehlspalette ⌘K, Tüfteln auch im Raster, Layout-Schlüssel `v8`, Ausnahmeliste der Stil-Prüfung leer, Gate-Ablauf in [docs/phase-3c-gate.md](phase-3c-gate.md).

Gate: Spec §1 (A) bis (H): der Screen „Welt bauen: Figur bearbeiten“ ist wiedererkennbar, Texte, Scripts und Assistent laufen im neuen Raster, dunkles Thema und Kompakt sind lesbar, Tastaturbedienung, die Abnahmen 3a, 3b.1 und 3b.2 über die Test-Umgebung und `scripts/gate.sh` grün; die gepackte App prüft der Nutzer von Hand. Ablauf: [docs/phase-3c-gate.md](phase-3c-gate.md).

Nicht in 3c: die Bereiche Geschichten, Pokémon und Gegenstände (gesperrt im Atlas), Tiefe Erzählen, Live-Vorschau der Assistenten in der Schau, Story-Graph, Spielmodus mit Live-Flags, Veröffentlichen, Plugins, Gelände malen mit Draufsicht und Muster im Overlay (siehe „Aus dem Design-System, später“).

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

## Aus dem Design-System, später

Das Design-System „Doppeldeck“ ([design-system/README.md](../design-system/README.md)) zeigt mehr, als 3c baut. Was bewusst später kommt (Spec 3c §13):

- Tiefe „Erzählen“ mit Rezepten und Live-Vorschau der Assistenten (nach 3b.3, als 3d).
- Bereiche Pokémon und Gegenstände (seit 3b.3a offen; Trainer und Begegnungen mit 3b.3b, Läden später, siehe unten).
- Flags und Variablen sowie Geschichten und Story-Graph (3b.4).
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
| Löschen von Daten, Projektprüfungen (Trainer ohne Party, leere Ersatz-Slots projektweit), Script → Trainer/Item | 3b.4 | Roadmap; Löschen ohne Script-Referenzen wäre ein leeres Versprechen |
| Pokédex-Größe/-Gewicht, Wachstumstabellen, „Auf Original zurücksetzen“ | Phase 4 (Basis-ROM-Vergleich) | Zukan-Daten, Basis-ROM |
| Diamant/Perl | kein Gate | kein Testprojekt; die Metadaten kennen die Pfade, der Editor sperrt DP mit Grund |

Stand nach 3b.3a: Plan-Task 0 hat die Paletten-Tabelle bestätigt, die Pokémon-Menüsymbole kamen mit 3b.3a (Pokédex-Liste, Auswahl der Arten).

## Querschnitt (laufend ab Phase 1)

- Prüfung: `scripts/gate.sh` fährt lokal `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm test`, die Token-Prüfung (`scripts/check-tokens.sh`), die Stilprüfung (`scripts/check-styles.sh`: keine Hex-Farben, keine Primitive, keine Inline-Styles), `pnpm build` und die gated Tests gegen die drei Testprojekte (siehe `docs/dev-setup.md`). Es gibt kein Remote und noch keine CI; Builds für macOS (arm64) und Windows (x64), Linux als Best-Effort, folgen mit einem Remote.
- Releases: Tauri-Bundles, signiert, mit Auto-Update ab Phase 3b.
- Dokumentation: `docs/formats/` für jedes Format mit Byte-Layout und Quelle der Information.
- Lizenz-Audit bei jeder neuen Abhängigkeit (`cargo deny`).
