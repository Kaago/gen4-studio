# Phase 3e: Gate prüfen

Stand: 2026-10-25. Phase 3e „Werkbank“ baut die Welt-Bedienung in vier Teilphasen um: 3e.1 Scharnier oben, Teiler, Werkzeugleiste; 3e.2 Weltbaum und Sortierung; 3e.3 Hierarchie und Inspector; 3e.4 ein Dock je Bereich. Jede Teilphase hat ihren Teil unten, mit eigener Ergebnistabelle; am Ende stehen das Gesamtgate aus [docs/ROADMAP.md](ROADMAP.md) („Phase 3e: Werkbank“) und die Tabelle „vorher → jetzt“ der Formulierungen, mit denen die älteren Abnahmen (3a, 3b.1 bis 3b.4, 3c) jetzt laufen.

Grundlagen: [docs/research/2026-10-25-ausbau/entscheidungen.md](research/2026-10-25-ausbau/entscheidungen.md) (Tabellen „Werkbank“ und „Weltmodell“), [gutachten-b-werkbank.md](research/2026-10-25-ausbau/gutachten-b-werkbank.md) (Variante C), [gutachten-a-weltmodell.md](research/2026-10-25-ausbau/gutachten-a-weltmodell.md) (Baum, Sortierung). Pläne: [3e.1](superpowers/plans/2026-10-25-phase-3e1-scharnier-teiler-werkzeugleiste.md), [3e.2](superpowers/plans/2026-10-25-phase-3e2-weltbaum-sortierung.md), [3e.3](superpowers/plans/2026-10-25-phase-3e3-hierarchie-inspector.md), [3e.4](superpowers/plans/2026-10-25-phase-3e4-ein-dock-je-bereich.md).

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer).

## Vorbereitung

1. Kopien anlegen (nie ein Original öffnen; das Ziel darf nicht existieren), aus dem Ordner `Roms`:
   - `_testkopien/3e-pt` aus `Pokemon - Platin-Edition (Germany)_DSPRE_contents`,
   - `_testkopien/3e-hgd` aus `Pokemon - Goldene Edition HeartGold (Germany)_DSPRE_contents`,
   - `_testkopien/3e-hgu` aus `Pokemon - HeartGold Version (USA)_DSPRE_contents`.
2. Test-Umgebung aus dem Worktree mit eigenen Ports: `G4_ROMS_DIR=<absoluter Pfad zu Roms> G4_VITE_PORT=1440 G4_BRIDGE_PORT=1441 bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 &`, warten auf „Brücke läuft“, Seite `http://127.0.0.1:1440/?project=<Kopie>`.
3. Vor jedem Teil `localStorage.clear()` und neu laden (jede Teilphase hebt den Layout-Schlüssel an: `v9`, `v10`, `v11`).
4. Fenster 1440 × 900 (Referenz) und 1100 × 720 (Minimum), Thema Hell, Dichte Komfort, wenn nicht anders gesagt.

## Teil 3e.1: Scharnier oben, Teiler, Werkzeugleiste

Lieferumfang (Roadmap): das Scharnier als Kopfleiste über der Schau (ohne Griff), ein eigener Teiler zwischen Schau und Hand (`role="separator"`, Pfeiltasten, Doppelklick klappt ein, `dock-welt-hand` bleibt), die Werkzeuge als schwebende Leiste in der 3D-Ansicht (Standard senkrecht links, an drei Anker, unter 460 px Höhe in Komfort bzw. 400 px in Kompakt waagrecht, `role="toolbar"` und `data-tool` bleiben; Buchstaben wirken nur bei sichtbarem 3D); Galerie „Rahmen“; Design-System: Prinzipien, Fensterraster, „Werkzeugleiste schwebend“, „Teiler“, Token `schatten/schwebend`.

| Punkt | Prüfung | Wer |
|---|---|---|
| (1.1) Reihenfolge | Von oben: Scharnier (`[role=toolbar][aria-label=Scharnier]`), Reiter der Schau, Schau, Teiler `[role=separator][data-splitter=hand]`, Hand `#dock-welt-hand` (`getBoundingClientRect` aneinander) | Agent |
| (1.2) Werkzeugleiste | In 3D schwebt `[role=toolbar][aria-label=Werkzeugleiste]`; `[data-tool=paint]` per Klick und „B“, „V“ zurück, Esc auf „Auswählen“; mit „Raster“ vorn ändert „B“ nichts und der Toast „Werkzeuge wirken in der 3D-Ansicht“ erscheint | Agent |
| (1.3) Teiler | Pfeil hoch +16 px, Pfeil runter −16 px, Ende 200 px, Pos1 560 px, Enter klappt ein (`aria-valuetext` „eingeklappt“) und wieder aus | Agent |
| (1.4) Script hebt die Hand | Hand auf 200 px, Scriptdatei öffnen → Hand ≥ 400 px | Agent |
| (1.5) Layout-Schlüssel | nach `localStorage.clear()` nur `g4.layout.v9.*`, keine `v8` | Agent |
| (1.6) Fenster 1100 × 720 | Leiste waagrecht rechts neben der Ebenen-Leiste, keine Überdeckung | Agent |
| (1.7) Anker merken | Menü „Lage der Werkzeugleiste“ › „Unten“: Leiste unten mittig, nach Neuladen weiter unten (`g4.shell.v1` `toolbar`) | Agent |
| (1.8) Galerie „Rahmen“ | `?gallery`, Seite „Rahmen“: Scharnier oben mit Teiler, Werkzeugleiste senkrecht, waagrecht, gesperrt | Agent + Blick |

### Ergebnis 3e.1

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-25 | 3e-pt | (1.1) | bestanden: Scharnier 40–84, Reiter 84–120, Schau bis 578, Teiler 578–586, Hand 586–872 (1440 × 900) | – |
| 2026-10-25 | 3e-pt | (1.2) | bestanden: Klick „Gelände malen“ → `paint`, V → `select`, B → `paint`, Esc → `select`; Raster vorn, B → bleibt `select`, Toast „Werkzeuge wirken in der 3D-Ansicht“ | Bei 1440 × 900 mit Hand 286 ist die Schau 458 px hoch: die Leiste liegt in Komfort waagrecht (Schwelle 460, siehe Befund 3) |
| 2026-10-25 | 3e-pt | (1.3) | bestanden: 286 → 302 → 286 (zwei Tastendrücke im selben Takt zählten einmal, siehe Befund 4), Enter klappt ein und aus, Ende 200 | – |
| 2026-10-25 | 3e-pt | (1.4) | bestanden: Scripts übernommen (`adopt_scripts`, 1124 Dateien), Scriptdatei 1052 geöffnet, Hand 200 → 400 | – |
| 2026-10-25 | 3e-pt | (1.5) | bestanden: `g4.layout.v9.welt.schau`, `g4.layout.v9.welt.hand` | – |
| 2026-10-25 | 3e-pt | (1.6) | bestanden: Leiste waagrecht ab x 454, Ebenen-Leiste endet bei x 446 | Ein Größenwechsel der Emulation von 1440 × 900 auf 1100 × 720 bei offenem Script ließ das Raster-Panel über die Hand ragen; nach Neuladen korrekt. Wird mit dem Dock aus 3e.4 erneut geprüft |
| 2026-10-25 | 3e-pt | (1.7) | bestanden: `data-anchor="bottom"`, `g4.shell.v1.toolbar = "bottom"`, nach Neuladen unten mittig | – |
| 2026-10-25 | – | (1.8) | Agent: drei Leisten und der Teiler da (Test `FramePage.test.tsx`); Blick beim Nutzer offen | – |

Befunde von 3e.1 (in der Teilphase behoben bzw. weitergegeben):

1. Die Leiste stieß bei 400 bis 455 px Schauhöhe in Komfort an die Legende; Schwelle je Dichte: Komfort 460, Kompakt 400 (Commit `1040d4d`).
2. Griff und Menü teilen sich keine Taste (Base UI öffnet ein Menü schon beim Drücken): der Griff `[data-toolbar-grip]` zieht nur (für Hilfstechnik verborgen), die Taste „Lage der Werkzeugleiste“ öffnet das Menü „Links“, „Oben“, „Unten“.
3. Die Referenzhöhe 900 px ergibt mit offener Hand (286) eine Schau von 458 px, also eine waagrechte Leiste. Mit der Anordnung „Werkbank“ aus 3e.4 (Script und Probleme unten 240 px) wird die Schau höher; dort erneut prüfen.
4. Zwei synthetische Tastendrücke am Teiler im selben Takt rechnen mit derselben alten Höhe (echte Tastendrücke sind getrennte Ereignisse und zählen richtig); ein Zittern von 1 px bei Klick oder Doppelklick auf eine eingeklappte Hand öffnet sie mit 200 px. Weitergegeben an 3e.3, Task 1 (Baustein `Splitter`: 4 px Schwelle, Start bei der gemerkten Höhe, funktionale Aktualisierung).

## Teil 3e.2: Weltbaum und Sortierung

Lieferumfang (Roadmap): `worldModel.ts` (Weltkarten, Gebiete, Räume über den Warp-Graphen, „Weitere Gebiete“, „Nicht verbunden“, „Sonstiges“ nur in Tüfteln), der Baum Weltkarte › Gebiet › Räume mit Ortsetikett und Klarnamen (umbenennbar, `g4-world.json`), Sortierung im Baum und in der Liste (`worldSort` in `g4.shell.v1`), Glossar mit `worldMap`, `area`, `landscape`, in Bauen kein sichtbares „Raster“, „Matrix“ oder „Header“ (der Reiter „Raster“ selbst bleibt bis 3f.1). Kernel: `HeaderSummary.location_index` und `kanto`, Befehle `world_meta_get` und `world_meta_set`. Gated Test: `workbench/worldModel.vanilla.test.ts` mit `G4_WORLD_DUMP` (Dumps aus `scripts/agent/world-dump.sh`, siehe `docs/dev-setup.md`).

| Punkt | Prüfung | Wer |
|---|---|---|
| (2.1) Weltkarten | Wurzeln des Baums `[role=tree][aria-label=Orte]`: Platin DE „Sinnoh“; HeartGold DE und US „Johto“ und „Kanto“ | Agent |
| (2.2) Weitere Gebiete | „Weitere Gebiete“ (`[data-world-group=more]`) enthält auf Platin „Großmoor“ und „Kampfzone“, auf HeartGold die Safari-Zone | Agent |
| (2.3) Räume zugeordnet | gated Test: Räume unter einem Gebiet / alle Räume ≥ 0,9, jeder Header genau einmal, `buildWorld` < 200 ms | Agent |
| (2.4) Kraterberg | Suche „Kraterberg“ auf Platin: die Ebenen liegen unter ihren Eingängen (Routen) bzw. unter „Weitere Gebiete“, keine unter „Nicht verbunden“ | Agent |
| (2.5) Sortierung | Menü „Sortierung“ › „Name A–Z“ auf HeartGold: in Kanto steht „Route 2“ vor „Route 10“; `g4.shell.v1` `worldSort.tree = "name"` | Agent |
| (2.6) Umbenennen | Raum wählen, „Weitere Aktionen für …“ › „Umbenennen…“: `g4-world.json` in der Kopie hat den Namen, nach Neuladen steht er im Baum | Agent |
| (2.7) Tüfteln | in Tüfteln erscheint die Wurzel „Sonstiges“ | Agent |
| (2.8) DOM-Scan Bauen | Rezept „DOM-Scan Bauen“ in [agent-gate.md](agent-gate.md): Liste, Scharnier, Schau, Hand, Statusleiste mit gewähltem Gebiet und gewählter Figur; erlaubt ist nur der Reiter „Raster“ | Agent |

### Ergebnis 3e.2

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-25 | 3e-pt | (2.1) | bestanden: eine Weltkarte „Sinnoh“ (61 Gebiete); Kopfzeile „1 Weltkarte, 72 Gebiete, 492 Räume“ | – |
| 2026-10-25 | 3e-hgd, 3e-hgu | (2.1) | bestanden: „Johto“ (34 Gebiete) und „Kanto“ (38 Gebiete); „2 Weltkarten, 83 Gebiete, 448 Räume“ | – |
| 2026-10-25 | 3e-pt | (2.2) | bestanden: „Weitere Gebiete, 11 Gebiete“ mit „Großmoor“ und „Kampfzone“ (13 Räume) | Dazu Teile von Kraterberg und Speersäule, Park der Freunde, Platz der Treue, Duellpark, Halle d. Beginns: eigene Matrizen ohne Header-Abschnitt (Regel aus dem Gutachten, siehe Befund 1) |
| 2026-10-25 | 3e-hgd, 3e-hgu | (2.2) | bestanden: „Safari-Zone“ / „Safari Zone“ (14 Räume) unter „Weitere Gebiete“ (11) | Auch die Häfen „Oliviana City“/„Olivine City“ und „Orania City“/„Vermilion City“ stehen dort (eigene Matrix), Befund 1 |
| 2026-10-25 | alle drei | (2.3) | bestanden (gated, `G4_WORLD_DUMP`, 10 Tests): Platin 482/492 Räume (0,980), `buildWorld` 4,0 ms; HeartGold DE und US je 427/448 (0,953), 1,9 bzw. 1,8 ms; jeder Header genau einmal | „Nicht verbunden“: Platin 10 Räume, HeartGold 21 (Sinnoh-Reste wie Kampfareal, Jubelstadt, Sinjoh-Ruinen) |
| 2026-10-25 | 3e-pt | (2.4) | bestanden: Ebene 1 und 2 unter Route 207, Ebene 12 und 13 unter Route 211, Ebene 11 und 3 unter Route 216, Ebene 9 unter Speersäule, Ebenen 3, 6, 7, 8, 10 unter den Kraterberg-Gebieten in „Weitere Gebiete“; keine unter „Nicht verbunden“ | – |
| 2026-10-25 | 3e-hgd | (2.5) | bestanden: Kanto mit „Route 2“ an Stelle 10, „Route 10“ an Stelle 18; `worldSort` `{"tree":"name","list":"id"}`; nach Neuladen weiter A–Z | – |
| 2026-10-25 | 3e-hgd | (2.6) | bestanden: Raum 0061 („Haus 1“, Neuborkia) umbenannt, `g4-world.json` `roomNames` `{"61": …}`, nach Neuladen findet die Suche den neuen Namen | – |
| 2026-10-25 | 3e-hgu | (2.7) | bestanden: Wurzeln „Johto“, „Kanto“, „Weitere Gebiete“, „Nicht verbunden“, „Sonstiges, 4 Header“ | – |
| 2026-10-25 | 3e-pt, 3e-hgd | (2.8) | bestanden: Gebiet (Zweiblattdorf, Neuborkia), Raum („Haus 1“) und Figur (`object:0`) gewählt: Liste, Scharnier, Hand und Statusleiste ohne Treffer, in der Schau nur der Reiter „Raster“ | Der erste Scan zählte den verborgenen Inhalt des Raster-Panels mit (Dockview verbirgt inaktive Panels mit `visibility: hidden`); `visibleWords` überspringt das jetzt (Befund 2) |

Befunde von 3e.2 (in der Teilphase behoben bzw. weitergegeben):

1. „Weitere Gebiete“ hat je Projekt 11 Einträge, mehr als das Gutachten nennt: Matrizen ohne Header-Abschnitt mit `map_type` 1 oder 2 und mehr als einer Zelle sind eigene Gebiete (Kraterberg- und Speersäulen-Teile, Häfen in HGSS). Bewusst so gelassen; umordnen erlaubt `g4-world.json` erst mit 3f.
2. `visibleWords` (`ui/visibleText.ts`) überspringt seit `82a613a` auch Text unter `visibility: hidden`/`collapse`.
3. In Bauen nennt der Inspector die Matrix eines Raums „Weltkarte 0126“ (auch Räume haben eine eigene Matrix); bis 3f.1 das Feld nach Tüfteln wandert, bleibt das Wort.
4. Brotkrume und Inspector-Kopf nennen einen Raum noch „Raum 0412“ statt „Haus 1“: weitergegeben an 3e.3 (Inspector-Kopf, Wurzel der Szene mit dem Namen aus dem Weltmodell).
5. Technik › Prüfen und Bauen teilt die Tabelle mit „Probleme“ und spricht in Bauen deshalb auch Bauen-Wörter.

## Teil 3e.3: Hierarchie und Inspector

Lieferumfang (Roadmap): die Liste „Welt“ mit zwei Abschnitten und Teiler: oben „Orte“ (der Weltbaum aus 3e.2, einklappbar zu einer Ortszeile mit Picker), unten „In dieser Szene“ (`role="tree"`: Gebiet bzw. Raum, Karten mit Gebäuden, Figuren, Türen mit Ziel, Auslöser, Schilder, Scripts; Suche; Enter auf einer Tür folgt ihr; „Zurück“ und „Vor“); „An diesem Ort“ und die Ausgangsliste des Header-Editors sind dorthin gewandert (im Header-Editor eine Zeile „4 Türen“ als Link); der Inspector zeigt nur das Gewählte mit Kopf (Typ-Wort, Titel, Kennung, Plaketten, Menü „Aktionen“); die Hover-Ansichten stehen in der Statusleiste; Baustein `Splitter` (auch unter dem Teiler der Hand); das Weltmodell liegt in den Werkbank-Daten und benennt Kopf, Brotkrume und Türziele; Layout `v10`; Design-System „Hierarchie“, „Baumzeile“, „Inspector-Kopf“, „Teiler“; Galerie „Rahmen“ › „Inspector-Kopf“ und „Hierarchie“.

| Punkt | Prüfung | Wer |
|---|---|---|
| (3.1) Figur wählen, ändern, ⌘Z, ⌘S | Figur in „In dieser Szene“ (`[data-event="object:N"]`) anklicken: Inspector-Kopf „Figur“, Zeile `aria-selected`; „Blickrichtung“ ändern, ⌘Z, erneut ändern, ⌘S; `call.sh get_events` und `dirty` als Gegenprobe | Agent |
| (3.2) Tür folgen, Zurück | Tür wählen, Enter: die Szene des Ziels; „Zurück“ zeigt die vorige Szene, „Vor“ wird frei | Agent |
| (3.3) Suche | „In dieser Szene suchen“ filtert, Escape leert | Agent |
| (3.4) Orte einklappen | „Orte einklappen“: eine Zeile mit dem gezeigten Ort, der Picker „Ort oder Raum wählen“ wechselt die Szene; `g4.shell.v1` `placesFolded` | Agent |
| (3.5) Türen-Zeile | Gebiet wählen: im Inspector „N Türen“; Klick öffnet die Gruppe „Türen“ und setzt den Fokus darauf | Agent |
| (3.6) Kopfmenü der Karte | Karte in „Karten“ wählen: Menü „Aktionen“ mit „Teile importieren…“, „Teile exportieren…“, „Duplizieren“; der Import-Dialog öffnet | Agent |
| (3.7) Tastatur im Baum | Pfeile, Rechts/Links öffnen und schließen, Enter wählt | Agent |
| (3.8) Hover in der Statusleiste | Maus über einem Marker und einer Kachel: Text in der Mitte der Statusleiste, der Inspector bleibt beim Gewählten | Agent |
| (3.9) DOM-Scan Bauen | Rezept „DOM-Scan Bauen“ mit beiden Abschnitten der Liste, Gebiet, Figur und Karte gewählt | Agent |
| (3.10) Kompakt 1100 × 720 | beide Abschnitte mindestens 96 px, Teiler „Höhe der Orte“ mit der Tastatur | Agent + Blick |
| (3.11) Layout-Schlüssel | nach `localStorage.clear()` nur `g4.layout.v10.*` | Agent |
| (3.12) Galerie | `?gallery`, „Rahmen“: Inspector-Kopf (Figur ungespeichert, Gebiet, Header in Tüfteln, Tür gesperrt, Karte mit Menü), Hierarchie mit Beispielszene, Teiler waagrecht und senkrecht | Agent + Blick |

### Ergebnis 3e.3

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-25 | 3e-pt | (3.1) | bestanden: Zweiblattdorf › „Objekt 3 · ID 3“ in der Hierarchie gewählt, Kopf „Figur / Objekt 3 · ID 3 / Objekt 3 · Eventdatei 0390“; „Blickrichtung“ Links → Rechts: Plakette „Ungespeichert“, „1 Änderung nicht gespeichert“; ⌘Z: „Alles gespeichert“; erneut geändert (Oben), ⌘S: `get_events` 390 `objects[3].dir` 2 → 0, `dirty` leer | Nach ⌘Z springt die Auswahl von der Figur auf das Gebiet zurück: so seit 3b.4b (`pinAfterHistory` lässt ein Event los, weil Undo die Indizes verschieben kann), kein Befund von 3e.3 |
| 2026-10-25 | 3e-pt | (3.2) | bestanden: „Warp 0 → Haus 1“, Enter: Szene „Haus 1“ (Kopf „Raum / Haus 1 / Zweiblattdorf · T01R0101“, Brotkrume „Zweiblattdorf › Haus 1“); „Zurück“: Zweiblattdorf, „Vor“ frei | Nach „Zurück“ fiel der Fokus auf `body` (Befund 1) |
| 2026-10-25 | 3e-pt | (3.3) | bestanden: „Haus 2“ lässt Wurzel, „Türen 1“ und „Warp 1 → Haus 2“ übrig; Escape leert, 25 Zeilen wieder da | – |
| 2026-10-25 | 3e-pt | (3.4) | bestanden: eingeklappt eine Zeile „Zweiblattdorf“, „In dieser Szene“ 727 px hoch, `placesFolded: true`; Picker „Ort oder Raum wählen“, Suche „Jubelstadt“, Wahl „Pokémarkt“: Zeile „Jubelstadt › Pokémarkt“, Szene „Pokémarkt“ (0004), Picker zu | Der Fokus erreicht das Suchfeld des Pickers erst nach der Einblende-Zeit; sehr schnelles Tippen direkt nach dem Klick landet im Suchfeld der Szene (Agent tippt schneller als ein Mensch) |
| 2026-10-25 | 3e-pt | (3.5) | bestanden: Gebiet Zweiblattdorf, Zeile „4 Türen“; „Türen“ vorher zugeklappt, Klick öffnet sie und fokussiert `[data-scene-group=warp]` | – |
| 2026-10-25 | 3e-pt | (3.6) | bestanden: „Karte 0000 · Zelle 3, 27“ mit Gebäude 1 bis 3 darunter; Kopf „Karte“, „Aktionen“: Teile importieren…, Teile exportieren…, Duplizieren; „Teile in Karte 0000 importieren“ öffnet | Kennung in Bauen „Weltkarte 0000 · M0000 H0411“ (Befund 2) |
| 2026-10-25 | 3e-pt | (3.7) | bestanden: Pfeil runter zweimal von der Wurzel auf „Karte 0000“, Rechts öffnet (Gebäude sichtbar), Enter wählt die Karte | – |
| 2026-10-25 | 3e-pt | (3.8) | bestanden: Marker „Objekt 5 · ID 5“ und Kachel „Karte 0190 · Kachel x 9, y 13 · …“ in der Mitte der Statusleiste; Inspector bleibt „Raum / Pokémarkt“ | In Bauen stand „Typ 0x00 NONE“ (Befund 3) |
| 2026-10-25 | 3e-pt | (3.9) | bestanden: Orte, In dieser Szene, Scharnier, Hand, Statusleiste ohne Treffer, Schau nur der Reiter „Raster“ (Gebiet, Figur, Karte gewählt) | – |
| 2026-10-25 | 3e-pt | (3.10) | Agent bestanden: Kompakt 1100 × 720, „Orte“ 314 px, „In dieser Szene“ 306 px, Pfeil runter 314 → 330 (`worldListSplit` 0,525); Blick beim Nutzer offen | – |
| 2026-10-25 | 3e-pt | (3.11) | bestanden: `g4.layout.v10.welt.schau`, `g4.layout.v10.welt.hand`; die Hand hat die Reiter Inspector, Script, Probleme | – |
| 2026-10-25 | – | (3.12) | Agent: Test `shows_the_inspector_head_and_the_scene_hierarchy` (`FramePage.test.tsx`); Blick beim Nutzer offen | – |
| 2026-10-25 | 3e-hgd | (2.6) erneut | bestanden: die Türen von Neuborkia nennen den in `g4-world.json` vergebenen Namen („Warp 0 → Haus von Mama“) | – |

Befunde von 3e.3 (in der Teilphase behoben):

1. Fokus nach „Zurück“/„Vor“: der Sprung lief über den Weg eines Header-Links, der den Fokus auf die Liste legt; seit `d82e0e8` bleibt er auf der Taste (wird sie gesperrt, auf der anderen). Nachgeprüft: „Zurück“ aus „Haus 1“ → Zweiblattdorf, Fokus auf „Vor“.
2. Kopf einer Karte in Bauen: seit `d82e0e8` Titel „Karte 0000“, Kennung „Zelle 3, 27 · Zweiblattdorf“ (nachgeprüft); die Kürzel M/H nur in Tüfteln.
3. Kachel-Hover in Bauen ohne Typ-Nummer und ROM-Namen: „Kachel x 24, y 30 · Karte 0174 · blockiert“ (`d82e0e8`, nachgeprüft).
4. Aus den Reviews: Kopf, Brotkrume und Türziele benennen Orte wie der Weltbaum (das Weltmodell liegt seit `7912e74` in den Werkbank-Daten); die Brotkrume nennt einen Raum mit Klarnamen (Befund 4 aus 3e.2 erledigt).
