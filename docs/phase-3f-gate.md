# Phase 3f: Gate prüfen

Stand: 2026-10-26. Phase 3f „Weltmodell“ baut das Weltmodell in Teilphasen aus: 3f.1 Raster nach Tüfteln, 3f.2 Eigene Orte, 3f.3 Leichter Patch, optional 3f.4 Nahtlos und Abspalten. Jede Teilphase hat unten ihren Teil mit eigener Ergebnistabelle; am Ende steht das Gesamtgate aus [docs/ROADMAP.md](ROADMAP.md) („Phase 3f: Weltmodell“, Absatz „Gate:“).

Grundlagen: [docs/research/2026-10-25-ausbau/entscheidungen.md](research/2026-10-25-ausbau/entscheidungen.md) (Tabelle „Weltmodell“), [gutachten-a-weltmodell.md](research/2026-10-25-ausbau/gutachten-a-weltmodell.md) (Variante B). Pläne: [3f.1](superpowers/plans/2026-10-26-phase-3f1-raster-nach-tuefteln.md), [3f.2](superpowers/plans/2026-10-26-phase-3f2-eigene-orte.md).

Wer was prüft: Die Spalte „Wer“ sagt **Agent** (Test-Umgebung aus [docs/agent-gate.md](agent-gate.md): Browser-Fenster mit `studio-bridge`, `call.sh`), **Agent + Blick** (der Agent prüft und beschreibt Ort und Zahlen, der Nutzer bestätigt kurz am Bild) oder **Hand** (der Nutzer).

## Vorbereitung

1. Kopien anlegen (nie ein Original öffnen; das Ziel darf nicht existieren), aus dem Ordner `Roms`:
   - `_testkopien/3f-pt` aus `Pokemon - Platin-Edition (Germany)_DSPRE_contents`,
   - `_testkopien/3f-hgd` aus `Pokemon - Goldene Edition HeartGold (Germany)_DSPRE_contents`,
   - für die späteren Teile je eine frische Kopie mit eigenem Namen (Anlegen, Patch und Bauen lassen sich nicht zurücknehmen).
2. Test-Umgebung aus dem Worktree mit eigenen Ports: `G4_ROMS_DIR=<absoluter Pfad zu Roms> G4_VITE_PORT=1470 G4_BRIDGE_PORT=1471 bash scripts/agent/studio-web.sh > target/agent/studio-web.log 2>&1 &`, warten auf „Brücke läuft“, Seite `http://127.0.0.1:1470/?project=<Kopie>`. Nach jeder Änderung am Rust-Code die Umgebung neu starten (die Brücke läuft sonst mit dem alten Stand).
3. Vor jedem Teil `localStorage.clear()` und neu laden.
4. Fenster 1440 × 900, Thema Hell, Dichte Komfort, Tiefe Bauen, wenn nicht anders gesagt.
5. Ladezeit: `scripts/agent/measure-scene-load.js` in das Browser-Fenster einfügen, dann `await measureSceneLoad('[data-area-key="w0:1"]')` (Platin, Zweiblattdorf) bzw. `'[data-area-key="w0:126"]'` (HeartGold, Neuborkia). Gemessen wird vom Klick auf die Baumzeile bis zur letzten Antwort der Brücke, wenn 4 s lang keine Anfrage mehr kam; „kalt“ heißt erste Messung nach dem Start der Brücke, „warm“ jede weitere nach Neuladen der Seite.

## Teil 3f.1: Raster nach Tüfteln

Lieferumfang (Roadmap): die Weltkarte übernimmt Zellen-Auswahl, „+“ und Kartenbilder mit einem Umschalter der Weltkarten, mehrzellige Räume bekommen einen Raumplan (eigene Matrix, 64 px); in 3D wählt ein Klick auf den Boden die Karte, die Kamera rahmt das Gebiet, nur Gebiet plus Nachbarn werden geladen, Draufsicht-Kamera; der Reiter „Raster“ (`MatrixPanel`) erscheint nur in Tüfteln; Brotkrume, Statusleiste und Palette in Nutzerworten. Dazu: eine Höhe in Bauen legt den Höhenabschnitt selbst an (ein Undo-Schritt).

| Punkt | Prüfung | Wer |
|---|---|---|
| (1.1) Reiter je Tiefe | Bauen: Reiter der Schau `[data-dock-tab]` nur „3D“ und „Weltkarte“; Tüfteln: „3D“, „Raster“, „Weltkarte“ (Raster nicht vorn); zurück in Bauen wieder ohne „Raster“ | Agent |
| (1.2) DOM-Scan Bauen | Rezept „DOM-Scan Bauen“ ([agent-gate.md](agent-gate.md)) mit Gebiet, Figur und Weltkarte vorn: kein Treffer, auch nicht im Dock | Agent |
| (1.3) Umschalter | Kopf des Reiters „Weltkarte“: Platin „Sinnoh“, „Weitere Gebiete“ (mit gewähltem Raum dazu „Raumplan“); HeartGold „Johto“, „Kanto“, „Johto und Kanto“, „Weitere Gebiete“; `data-world-view` | Agent |
| (1.4) Gebiet und Karte auf der Weltkarte | Klick auf eine Zelle eines Gebiets wählt das Gebiet (Baumzeile `aria-selected`), Klick auf Landschaft und ⌥Klick wählen die Karte (Inspector „Karte“), die Karte springt zum gewählten Gebiet | Agent |
| (1.5) „+“ an einem Gebiet | ein Gebiet mit freier Nachbarzelle: „+“ (`data-extend`, „… (nicht rückgängig)“), Dialog mit Plakette „Nicht rückgängig“ und „Die neue Karte gehört zu „…““, ohne „Raster/Header“; „Karte anlegen“ füllt die Zelle mit einer neuen Karte im Gebiet | Agent |
| (1.6) Raumplan | ein mehrzelliger Raum (Ewigwald Ebene 1, 3 × 3): Ansicht `plan`, Zellen 64 px, „+“ rechts und unten, Gitter „Raumplan, 3 Zeilen, 3 Spalten“ (Tüfteln „Raster 7, …“) | Agent |
| (1.7) Karte in 3D wählen | „Auswählen“, Klick auf den Boden ohne Marker: Inspector „Karte NNNN“, Brotkrume „… › Karte NNNN Zelle x, y“, Statusleiste „<Ort>, Karte NNNN (x, y)“, Rahmen um die Zelle | Agent + Blick |
| (1.8) Draufsicht | Taste „Draufsicht“ (`aria-pressed`), Szene senkrecht von oben, `g4.shell.v1.topDown`; zurück zur Schrägsicht | Agent + Blick |
| (1.9) Höhe in Bauen | Raum ohne Höhenabschnitt (Ewigwald, Matrix 7): Zelle im Raumplan wählen, „Höhe“ 3: `get_matrix` hat den Abschnitt, ⌘Z bzw. `undo` nimmt Abschnitt und Wert zurück | Agent |
| (1.10) Ladezeit | Zweiblattdorf (Platin) und Neuborkia (HeartGold) vor 3f.1 (`974a545`) und danach mit `measureSceneLoad` | Agent |

### Ergebnis 3f.1

Stand: Branch `phase-3f-weltmodell` bis `16d19c5`, Test-Umgebung 1470/1471, Kopien `3f-pt` und `3f-hgd`.

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-26 | 3f-pt | (1.1) | bestanden: Bauen `world`, `worldmap` (dazu `script`, `problems`, `inspector`); Tüfteln `world`, `matrix`, `worldmap`, vorn blieb „Weltkarte“; zurück in Bauen ohne `matrix` | – |
| 2026-10-26 | 3f-pt | (1.2) | bestanden: Orte, In dieser Szene, Scharnier, Dock, Statusleiste ohne Treffer mit Gebiet Zweiblattdorf, Figur `object:0` und Weltkarte vorn; im Dock gibt es den Reiter „Raster“ nicht mehr | – |
| 2026-10-26 | 3f-pt, 3f-hgd | (1.3) | bestanden: Platin „Sinnoh“, „Weitere Gebiete“, mit Ewigwald Ebene 1 dazu „Raumplan“; HeartGold „Johto“, „Kanto“, „Johto und Kanto“, „Weitere Gebiete“; Kanto `w0k` zeigt den Ausschnitt ab Zelle 21,0 (425 Zellen, 4 gedämpft) | „Weitere Gebiete“ (Platin): 11 Kacheln, u. a. Kraterberg, Speersäule, Großmoor (20 Karten), Kampfzone (9 Karten) |
| 2026-10-26 | 3f-pt | (1.4) | bestanden: Klick auf 3,27 wählt Zweiblattdorf (Gebiet 0411), Klick und ⌥Klick auf Landschaft 4,27 heften Karte 0173 an; nach dem Fix `16d19c5` liegt Zweiblattdorf (Zelle 564–596 × 373–405) mitten im Ausschnitt 360–800 × 145–632 | vor dem Fix stand die Karte am Ursprung oben links, das gewählte Gebiet außerhalb (Befund 2) |
| 2026-10-26 | 3f-pt | (1.5) | bestanden: Vollmond-Insel (1,8) bietet „+“ bei 1,9 („Karte bei x 1, y 9 anlegen (nicht rückgängig)“); Dialog „Kopie von Karte 0052 (Nachbarzelle)“, „Die neue Karte gehört zu „Vollmond-Insel“.“, „Die Größe bleibt 30 × 30.“, Plakette „Nicht rückgängig“, kein „Raster/Matrix/Header“; danach Zelle 1,9 = Karte 0666 im Gebiet 0260, „+“ wandert nach 1,10; 3D zeigt beide Karten | – |
| 2026-10-26 | 3f-pt | (1.6) | bestanden: Ewigwald Ebene 1 (0203): `data-world-view="plan"`, Zellen 64 px, „+“ bei 3,0–3,2 und 0,3–2,3, Gitter „Raumplan, 3 Zeilen, 3 Spalten“; in Tüfteln „Raster 7, 3 Zeilen, 3 Spalten“ | – |
| 2026-10-26 | 3f-pt | (1.7) | bestanden: Klick auf den Boden neben der Vollmond-Insel: Inspector „Karte 0174, Zelle 2, 9“, Brotkrume „Vollmond-Insel › Gebiet 0260 › Karte 0174 Zelle 2, 9“, Statusleiste „Vollmond-Insel, Karte 0174 (2, 9)“; Zweiblattdorf: „Karte 0000 · Zelle 3, 27 · Zweiblattdorf“. Bild: Rahmen als orangefarbener Ring um die Zelle, zwischen den Bäumen teils verdeckt (Blick beim Nutzer) | vor dem Fix ein 1-px-Strich, kaum zu sehen (Befund 3) |
| 2026-10-26 | 3f-pt | (1.8) | bestanden: `aria-pressed="true"`, `topDown: true`; Bild: Vollmond-Insel mit der neuen Karte darunter senkrecht von oben, Norden oben; Schrägsicht kehrt zurück. Seit `16d19c5` verschiebt die linke Maustaste die Draufsicht (Blick beim Nutzer) | vor dem Fix verschob nur die rechte Maustaste (Befund 1) |
| 2026-10-26 | 3f-pt | (1.9) | bestanden: Inspector „Karte 0342“, „Höhe“ 3 mit Enter: `get_matrix` 7 `altitudes` `[0,0,0,0,3,0,0,0,0]`, Kachel „Höhe 3“; über `call.sh`: `set_matrix_cell` mit Höhe 2 legt den Abschnitt an, ein `undo` → `altitudes` `null` | Erst nach Neustart der Brücke (sie lief noch mit dem Stand vor Task 1: „the matrix has no altitude section“) |
| 2026-10-26 | 3f-pt | (1.10) | bestanden: Zweiblattdorf vorher (`974a545`) kalt 7523 ms, warm 2851 und 2481 ms, 852 Anfragen, `get_map` 352, `get_map_model_glb` 176; nachher warm 347 und 510 ms, 45 Anfragen, `get_map` 12, Modelle 6 | Die Szene lädt Zweiblattdorf plus den Ring der 8 Nachbarn |
| 2026-10-26 | 3f-hgd | (1.10) | bestanden: Neuborkia vorher kalt 9856 ms, warm 3854 ms, 1058 Anfragen, `get_map` 408, Modelle 204; nachher warm 596 ms, 55 Anfragen, `get_map` 12, Modelle 6 | – |

Befunde von 3f.1 (behoben in `16d19c5`, nachgeprüft):

1. Draufsicht: die linke Maustaste tat nichts (OrbitControls behält „drehen“ auf links, das Drehen war nur abgeschaltet); jetzt schiebt links in der Draufsicht, die Schrägsicht behält die Belegung.
2. Die Weltkarte stand nach dem Wechsel der Ansicht oder der Auswahl am Ursprung, das gewählte Gebiet lag außerhalb; jetzt rückt sie das Gebiet bzw. die angeheftete Karte in die Mitte (nicht, wenn es schon ganz zu sehen ist, damit ein Doppelklick nicht verrutscht).
3. Der Rahmen um die gewählte Zelle war ein 1-px-Strich; jetzt ein flacher Ring (eine halbe Kachel breit) in der Auswahlfarbe, über einer nicht geladenen Zelle gibt es keinen Rahmen.
4. „+“ im Raumplan neben Landschaftszellen nannte den Raum, schickte aber Header 0; jetzt gehört die neue Karte dem gezeigten Raum.
5. Inspector „Karte“ eines Raums sprach von der Weltkarte („Die Weltkarte hat keinen Abschnitt für Gebiete.“); in Bauen entfällt die Notiz, außerhalb heißt es „Die Zelle liegt außerhalb der Karten (W × H).“

Gesehen, nicht behoben (klein): im Inspector „Karte“ steht in Bauen das Gebiet als Nummer mit Verweis („Gebiet 411“, „Gebiet 0411“), der Picker „Gehört zu“ aus dem Gutachten fehlt; der Kopf eines außerhalb liegenden oder noch ladenden Felds sagt „Weltkarte NNNN“.

## Teil 3f.2: Eigene Orte

Lieferumfang (Roadmap): Kernel `create_place`, `rename_place`, `assign_place` (Nachricht an die Ortsnamen-Bank anhängen, Platin Archiv 433, HGSS 279; `location_name` mehrerer Header in einem Undo-Schritt; Ortsnamen aus dem Store statt von der Platte), Prüfungen (Index ≤ 255, Warnung ab 17 Zeichen, HGSS mit 21 freien Plätzen und Wiederverwendung ungenutzter Sinnoh-Namen mit Warnsatz); Inspector „Ort“ als Picker, „Neuer Ort…“ im +-Menü (im Assistenten mit dem Schritt „Ort“ in 3f.3); `docs/formats/map_header.md`. Dazu `place_slots` und die Prüfungen `place.name-too-long`, `place.index-out-of-bank` (41 Prüfungen).

| Punkt | Prüfung | Wer |
|---|---|---|
| (2.1) Picker „Ort“ | Inspector eines Gebiets: Feld „Ort“ mit Ortsfarbe und Name (`button[data-place-picker]`), „Ort wählen“ listet alle Namen der Bank mit Zahl der Gebiete und Räume, Fußzeile „Neuer Ort…“ | Agent |
| (2.2) Neuer Ort | „+“ in „In dieser Szene“ › „Neuer Ort…“: Dialog „Neuer Ort“ mit „Name des Orts“ und „Mitnehmen“ (Gebiet und Räume vorgewählt), „Neuheim“ anlegen: Toast „Ort „Neuheim“ angelegt“, Baum zeigt das Gebiet als „Neuheim“, Verlauf ein Schritt, ungespeichert `TextArchive 433` und `HeaderTable` | Agent |
| (2.3) Undo und Redo | ⌘Z: Baum wieder „Zweiblattdorf“; ⌘⇧Z: wieder „Neuheim“ | Agent |
| (2.4) Lange Namen | ab 17 Zeichen die Notiz „Länger als 16 Zeichen: im Spiel kann der Name abgeschnitten werden.“ | Agent |
| (2.5) Umbenennen | Kopfmenü „Aktionen“ › „Ort umbenennen…“: Dialog mit dem Namen und dem Satz „Trifft alle N Gebiete und Räume mit diesem Ort und den Fundort gefangener Pokémon.“ | Agent |
| (2.6) Plätze | `place_slots`: Platin 126 Namen, 130 frei; HeartGold DE Bank 279, 235 Namen, 21 frei, unbenutzte Sinnoh-Namen wiederverwendbar | Agent |
| (2.7) Prüfungen | `run_checks` 0 Fehler mit dem neuen Ort; gated `vanilla_checks` auf Kopien von Platin DE, HeartGold DE und US mit 0 für beide neuen Codes | Agent |
| (2.8) Im Spiel | Szenario `pt-neuer-ort.yaml`: `create_place` „Neuheim“ nur für den Außen-Header 0411, bauen, in `g4emu` aus dem Haus treten: Einblendung „Neuheim“ | Agent + Blick |

### Ergebnis 3f.2

Stand: bis `c344ea1`, Test-Umgebung 1470/1471 (nach den Rust-Änderungen neu gestartet), Kopien `3f2-pt`, `3f2-hgd`, für das Szenario `3f2e-pt` (erster Lauf, Weg angepasst) und `3f2f-pt`; gated Läufe auf `3f-pt`, `3f-hgd`, `3f-hgu`.

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-26 | 3f2-pt | (2.1) | bestanden: Feld „Ort“ mit Punkt und „Zweiblattdorf“; „Ort wählen“ mit 126 Einträgen, zuerst „Kein Ort (Mysteriöser Ort)“, „Zweiblattdorf“, „Sandgemme“, „Flori“; Fußzeile „Neuer Ort…“ | Zählung damals je Header („7 Gebiete und Räume“), seit `c344ea1` je Gebiet |
| 2026-10-26 | 3f2-pt | (2.2) | bestanden: Kästchen Zweiblattdorf, Haus 1, Haus 1 1. Stock, Haus 2, Haus 2 1. Stock, Haus 3, Haus 4 vorgewählt; „Neuheim“: Toast, Baumzeile `w0:126` „Neuheim, 6 Räume“, Brotkrume „Neuheim › Gebiet 0411“, `history` `["Ort „Neuheim“ angelegt"]`, `dirty` TextArchive 433 und HeaderTable | – |
| 2026-10-26 | 3f2-pt | (2.3) | bestanden: ⌘Z → `w0:1` „Zweiblattdorf, 6 Räume“, ⌘⇧Z → „Neuheim“ | – |
| 2026-10-26 | 3f2-pt | (2.4) | bestanden: „Neuheim am langen Fluss“ zeigt die Notiz | – |
| 2026-10-26 | 3f2-pt | (2.5) | bestanden: „Ort umbenennen“, Feld mit „Neuheim“, „Trifft alle 7 Gebiete und Räume mit diesem Ort und den Fundort gefangener Pokémon.“ (abgebrochen) | seit `c344ea1` gezählt je Gebiet |
| 2026-10-26 | 3f2-pt, 3f2-hgd | (2.6) | bestanden: Platin nach dem neuen Ort `{bank 433, count 127, free 129, reusable []}`; HeartGold DE `bank 279, count 235, free 21`, 112 wiederverwendbare Sinnoh-Namen (1, 2, 3, 4, 5, 7, …) | – |
| 2026-10-26 | 3f2-pt, Kopien | (2.7) | bestanden: `run_checks` 0 Fehler, kein `place.*`; gated `vanilla_checks` auf `3f-pt`, `3f-hgd`, `3f-hgu` je 3 Tests ok (SOLL 0 für beide Codes) | – |
| 2026-10-26 | 3f2f-pt | (2.8) | bestanden: Szenario `pt-neuer-ort` exit 0, Header 415 → 414 → 411; Bild `target/agent/gate-2026-10-26/pt-neuer-ort/einblendung.png`: oben links die Einblendung „Neuheim“ vor dem Spielerhaus (Blick beim Nutzer) | erster Lauf auf `3f2e-pt` blieb im Zimmer stehen (Startpunkt des Spielstands x 7, y 4); Weg angepasst |

Befunde der Abschlussprüfung von 3f.2 (behoben in `c344ea1`): die Bank-Nummer konnte beim schnellen Bestätigen noch fehlen (die offene Nachrichtentabelle blieb dann alt); die Zahl der Nutzer zählte Abschnitte eines Gebiets einzeln; „Ort anlegen“ und „Umbenennen“ gingen mit leerem Namen (der Kernel lehnte erst danach ab); Namen behielten Leerzeichen am Rand; `plan_empty_region` las Ortsnamen von der Platte; der Prüftext setzte den Namen ohne Anführungszeichen.

Offen (Blick beim Nutzer, Recherche): ob HeartGold Namen ab Index 235 im Spiel richtig zeigt, prüft das Gesamtgate mit `hg-neuheim.yaml`; Stadtkarte und Flugziele neuer Orte sind nicht Teil von 3f.
