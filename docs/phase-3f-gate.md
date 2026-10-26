# Phase 3f: Gate prüfen

Stand: 2026-10-26. Phase 3f „Weltmodell“ baut das Weltmodell in Teilphasen aus: 3f.1 Raster nach Tüfteln, 3f.2 Eigene Orte, 3f.3 Leichter Patch, optional 3f.4 Nahtlos und Abspalten. Jede Teilphase hat unten ihren Teil mit eigener Ergebnistabelle; am Ende steht das Gesamtgate aus [docs/ROADMAP.md](ROADMAP.md) („Phase 3f: Weltmodell“, Absatz „Gate:“).

Grundlagen: [docs/research/2026-10-25-ausbau/entscheidungen.md](research/2026-10-25-ausbau/entscheidungen.md) (Tabelle „Weltmodell“), [gutachten-a-weltmodell.md](research/2026-10-25-ausbau/gutachten-a-weltmodell.md) (Variante B). Pläne: [3f.1](superpowers/plans/2026-10-26-phase-3f1-raster-nach-tuefteln.md), [3f.2](superpowers/plans/2026-10-26-phase-3f2-eigene-orte.md), [3f.3](superpowers/plans/2026-10-26-phase-3f3-leichter-patch.md), [3f.4](superpowers/plans/2026-10-26-phase-3f4-nahtlos-abspalten.md).

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

## Teil 3f.3: Leichter Patch

Lieferumfang (Roadmap): im Assistenten der Schritt „Ort“ statt „Header“, der Header wird immer neu angelegt (interner Name automatisch), der Patch „Dynamic Headers“ wird einmal in der Zusammenfassung bestätigt (Plakette „Nicht rückgängig“); der Kernel wendet ihn auch bei ungespeicherter Header-Tabelle an und legt vorher einen Schnappschuss von `unpacked/dynamicHeaders` an (`allow_patch`); Technik › Patches bleibt für Reparatur und Status. Dazu: `AssignHeader.place` (vorhandener oder neuer Ort im selben Schritt), die Texte der Assistenten in Bauen ohne „Raster“, „Matrix“, „Header“; Tüfteln behält den Schritt „Header“.

| Punkt | Prüfung | Wer |
|---|---|---|
| (3.1) Ungespeicherte Änderung | Inspector von Zweiblattdorf (Platin) bzw. Neuborkia (HeartGold): „Musik Tag“ +1, ungespeichert (`dirty` HeaderTable, Verlauf „Musik Tag“) | Agent |
| (3.2) Schritte | „Neu anlegen“ › „Gebiet anlegen…“: Schrittleiste „Quelle, Anordnung, Ort, Zusammenfassung“, Quelle nach Weltkarten benannt, kein „Raster/Matrix/Header“ in allen Schritten (`visibleWords`) | Agent |
| (3.3) Schritt „Ort“ | „Vorhandener Ort | Neuer Ort“, Vorgabe „Neuer Ort“ mit dem Gebietsnamen, „Aussehen wie <Ort der Vorlage>“ | Agent |
| (3.4) Zustimmung | Zusammenfassung mit Plakette „Nicht rückgängig“ und „Einmalig für dieses Projekt: Spielcode für neue Orte anpassen (DSPRE-kompatibel, Sicherung arm9.bin.backup)“; „Anlegen“ erst mit Haken | Agent |
| (3.5) Ein Aufruf | „Anlegen“: Patch angewendet, neuer Header mit automatischem Namen, neuer Ort, Schnappschuss unter `.g4/backup/`, die ungespeicherte Änderung bleibt | Agent |
| (3.6) Undo über den Patch | ⌘Z bis vor die Änderung von (3.1): „Musik Tag“ wieder alt; ⌘⇧Z bis zum Ende: alles wieder da | Agent |
| (3.7) Speichern | ⌘S: keine ungespeicherten Assets, `unpacked/dynamicHeaders/<id>` geschrieben | Agent |
| (3.8) Tüfteln | Schritt „Header“ mit „Vorhandenen Header umstellen | Neuer Header“ unverändert (vitest `shows_the_step_place_in_bauen_and_header_in_tuefteln`) | Agent |

### Ergebnis 3f.3

Stand: bis `f6f958e`, Test-Umgebung 1470/1471 (nach den Rust-Änderungen neu gestartet), Kopien `3f3-pt` (Platin DE) und `3f3-hgd` (HeartGold DE). Der Reiter „Raster“ wurde nicht geöffnet (es gibt ihn in Bauen nicht), Technik nicht besucht.

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-26 | 3f3-pt, 3f3-hgd | (3.1) | bestanden: Platin „Musik Tag“ 1004 → 1005, HeartGold 1018 → 1019; Verlauf `["Musik Tag"]`, `dirty` HeaderTable; Patch fehlte (`applied: false`) | – |
| 2026-10-26 | 3f3-pt, 3f3-hgd | (3.2) | bestanden: Schrittleiste „Quelle, Anordnung, Ort, Zusammenfassung“; Quelle „Sinnoh“ bzw. „Johto / Kanto“, „Karten aus einer Weltkarte oder einem Gebiet“; Platin Zellen 3,26 und 4,26 (Karten 0005, 0006, Route 201), HeartGold 18,12 und 19,12 (Karten 0001, 0002, Route 29); Name „Neuheim“; `visibleWords` ohne Treffer in jedem Schritt | Vorbelegt war die Zelle des gewählten Gebiets (Zweiblattdorf), abgewählt |
| 2026-10-26 | 3f3-pt, 3f3-hgd | (3.3) | bestanden: „Neuer Ort“ gedrückt, „Name des neuen Orts“ = „Neuheim“ | „Aussehen wie Jubelstadt“ bzw. „Route 26“ (erster Ort mit diesem Aussehen), seit `68cc29e` nennt es den Ort der Vorlage („Route 201“) |
| 2026-10-26 | 3f3-pt, 3f3-hgd | (3.4) | bestanden: Zusammenfassung „Legt das Gebiet „Neuheim“ mit 2 × 1 Karten an. Kopiert Karten 0005, 0006, mit Gebäuden. Neuer Ort „Neuheim“. … Eine neue, leere Liste der Figuren und Türen. Anlegen lässt sich nicht rückgängig machen.“, Plakette, Kästchen; „Anlegen“ gesperrt, nach dem Haken frei | – |
| 2026-10-26 | 3f3-pt | (3.5) | bestanden: Ergebnis „Gebiet „Neuheim“ angelegt: 2 Karten.“; `applied: true`; Header 0593 `NEUHEIM`, Ort 126 „Neuheim“, Matrix 0289, Eventdatei 0534, Aussehen 6; `.g4/backup/dynamicHeaders-20261026-115543/` mit den 12 Vanilla-Dateien; Header 0411 weiter mit „Musik Tag“ 1005 im Speicher; Verlauf `["Musik Tag", "build area", "Gebiet „Neuheim“ angelegt"]`; „Zeigen“: Brotkrume „Neuheim › Gebiet 0593“, unter „Weitere Gebiete“ | – |
| 2026-10-26 | 3f3-hgd | (3.5) | bestanden: Header 0540 `NEUHEIM`, Ort 235 „Neuheim“ (erster Platz hinter den Vanilla-Namen), Schnappschuss `dynamicHeaders-20261026-120320` | – |
| 2026-10-26 | 3f3-pt, 3f3-hgd | (3.6) | bestanden: Platin ⌘Z ×3 → Verlauf leer, „Musik Tag“ 1004; ⌘⇧Z ×3 → 1005, Gebiet zurück; HeartGold ⌘Z ×6 (nach Erweitern, Import und Höhe) → Verlauf leer, Header 0540 wieder „Route 29“; ⌘⇧Z ×6 → „Neuheim“ | – |
| 2026-10-26 | 3f3-pt, 3f3-hgd | (3.7) | bestanden: `dirty` leer, `unpacked/dynamicHeaders/0411` und `0593` je 24 Byte | – |
| 2026-10-26 | – | (3.8) | bestanden (vitest) | – |

Befunde der Abschlussprüfung von 3f.3 (behoben in `68cc29e`, `f6f958e`): der deutsche Fehlertext nach einem späten Fehler verschwieg, dass der Patch bleibt (oder zeigte den englischen Text); das Aussehen nannte den ersten Ort statt der Vorlage; der Pfad der Sicherung erschien nirgends (jetzt im Ergebnis und in Technik › Patches, in Bauen ohne Pfad: „Die alten Dateien sind unter .g4/backup gesichert.“); ein Fehler beim Nachladen nach dem Patch konnte als Ablehnung neben dem Ergebnis stehen; das Gitter im Assistenten hieß für Screenreader „Raster“.

## Gesamtgate 3f

Roadmap, Absatz „Gate:“: Auf Kopien von Platin DE und HeartGold DE entsteht in Bauen, ohne den Reiter „Raster“ zu öffnen und ohne Technik, ein Gebiet mit dem neuen Ort „Neuheim“ in einem Durchgang (Patch mit einer Zustimmung, auch bei ungespeicherter Header-Änderung, deren Undo danach noch geht), das Gebiet wird um eine Karte erweitert, in die neue Karte werden Teile importiert und eine Höhe gesetzt; im Emulator erscheint beim Betreten die Einblendung „Neuheim“, ein dort gefangenes Pokémon zeigt den Fundort; DSPRE zeigt Name und Patch (Hand); `vanilla_checks` behält 0 Fehler, die Ladezeit von Zweiblattdorf ist kleiner als vor 3f.1; `scripts/gate.sh` grün (läuft später auf main).

| Punkt | Prüfung | Wer |
|---|---|---|
| (G.1) Gebiet in Bauen | Teil 3f.3 (3.1)–(3.7) auf Platin DE und HeartGold DE, ohne Reiter „Raster“, ohne Technik | Agent |
| (G.2) Erweitern, Teile, Höhe | Raumplan des neuen Gebiets: „+“ bei 2,0, „Karte anlegen“ (neue Karte im Gebiet, 3 × 1); Inspector „Karte“ › „Aktionen“ › „Teile importieren…“ aus einer Karte im Projekt; „Höhe“ 1; `run_checks` 0 Fehler | Agent |
| (G.3) Einblendung | Szenarien `pt-neuheim.yaml` und `hg-neuheim.yaml` (dieselben Schritte über die Brücke, die Treppe des Zimmers führt ins Gebiet): Bild `einblendung` | Agent + Blick |
| (G.4) Fundort | ein im neuen Gebiet gefangenes Pokémon zeigt „Neuheim“ als Fundort | Hand |
| (G.5) DSPRE | DSPRE öffnet eine Kopie nach dem Gate, zeigt den Patch als angewendet und „Neuheim“ als Ortsnamen | Hand |
| (G.6) Prüfungen | gated `vanilla_checks` (41 Prüfungen) auf frischen Kopien Platin DE, HeartGold DE und US: 0 Fehler; dazu die gated Workspace-Tests | Agent |
| (G.7) Ladezeit | Zweiblattdorf (Platin) und Neuborkia (HeartGold) vorher (`974a545`) und nachher, `measureSceneLoad` | Agent |

### Ergebnis Gesamtgate

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-26 | 3f3-pt, 3f3-hgd | (G.1) | bestanden (Teil 3f.3) | – |
| 2026-10-26 | 3f3-pt | (G.2) | bestanden: „+“ bei 2,0, Dialog „Kopie von Karte 0667 (Nachbarzelle)“, „Die neue Karte gehört zu „Neuheim“.“, „Die Karte wird größer: 3 × 1.“; Karte 0668 im Gebiet 0593; „Teile importieren…“ aus Karte 0005: „3 Teile übernommen“ (Modell, Kollision, BDHC); „Höhe“ 1; Matrix 0289 `maps [666, 667, 668]`, `headers [593, 593, 593]`, `altitudes [0, 0, 1]`; Verlauf dazu „extend area“, „map 0668: model and permissions and terrain imported“, „edit matrix cell“; 3D zeigt drei Karten, die dritte mit dem Modell von Route 201 Karte 5, eine halbe Kachel höher; `run_checks` 0 Fehler | Erster Import aus Karte 6: „Nichts geändert: die Teile waren schon so.“ (die neue Karte ist selbst eine Kopie von 6) |
| 2026-10-26 | 3f3-hgd | (G.2) | bestanden: Karte 0678 bei 2,0, „Teile importieren…“ aus Karte 0003: „5 Teile übernommen“ (mit Gebäuden und Hintergrundklang), „Höhe“ 1, `run_checks` 0 Fehler | – |
| 2026-10-26 | 3f-gate-pt | (G.3) | bestanden: `pt-neuheim` exit 0 (Erwartungen: Patch vorher aus, nachher an, `location_name` 126, Breite 3, Höhe 1, 0 Fehler), Header 415 → 593; Bilder `target/agent/gate-2026-10-26/pt-neuheim/einblendung.png` (Einblendung „Neuheim“ oben links über Route-201-Gelände) und `osten.png` (x 43 auf der zweiten Karte) | Ein Lauf weiter nach Osten blieb bei x 29 stehen (vermutlich eine Begegnung im Gras, nicht verfolgt) |
| 2026-10-26 | 3f-gate-hgd | (G.3) | bestanden: `hg-neuheim` exit 0 (Ort 235 „Neuheim“), Header 64 → 540; Bild `target/agent/gate-2026-10-26/hg-neuheim/einblendung.png`: Einblendung „Neuheim“ oben links über Route-29-Gelände, unten das Menü des Touchscreens. HeartGold zeigt also einen Ortsnamen hinter den Vanilla-Plätzen | – |
| – | – | (G.4) | beim Nutzer offen: der Spielstand liegt vor dem ersten Pokémon und ohne Bälle; Fangen über `g4emu` ist nicht vorgesehen | – |
| – | – | (G.5) | beim Nutzer offen (DSPRE, Kopien `3f3-pt`, `3f3-hgd` gespeichert) | – |
| 2026-10-26 | 3f-gt-pt, 3f-gt-hgd, 3f-gt-hgu | (G.6) | bestanden: `G4_TEST_PROJECT=<Kopie> cargo test --workspace --all-features -- --test-threads 1` auf `ab1eb37` je Kopie alle 34 Testprogramme ok (102 „test result: ok“, 0 Fehlschläge), darin `vanilla_checks` (3 Tests, SOLL 0 für die neuen Codes, alle Fehler-Codes 0) und `dynamic_headers` (`vanilla_project_takes_the_patch`); Log `target/agent/gated-3f.log` | Vorher (nach 3f.2) `vanilla_checks` auf `3f-pt`, `3f-hgd`, `3f-hgu` einzeln: je 3 ok |
| 2026-10-26 | 3f-pt, 3f-hgd | (G.7) | bestanden: Zweiblattdorf vorher kalt 7523 ms, warm 2851/2481 ms (852 Anfragen, 176 Modelle) → nachher (`f6f958e`) kalt 471 ms, warm 570 ms (45–55 Anfragen, 6 Modelle; nach 3f.1 warm 347/510 ms); Neuborkia vorher kalt 9856, warm 3854 ms (1058 Anfragen, 204 Modelle) → nachher warm 596 ms (55 Anfragen, 6 Modelle) | Die Brücke läuft als Debug-Build; Zeiten im Browser-Fenster, das verborgen lief |

Was beim Nutzer bleibt (Gesamtgate): (G.3) die Bilder der Einblendung (`target/agent/gate-2026-10-26/pt-neuheim/einblendung.png`, `…/hg-neuheim/einblendung.png`, dazu `…/pt-neuer-ort/einblendung.png` aus 3f.2); (G.4) ein im neuen Gebiet gefangenes Pokémon zeigt den Fundort „Neuheim“; (G.5) DSPRE öffnet `3f3-pt` bzw. `3f3-hgd` und zeigt den Patch und „Neuheim“; `scripts/gate.sh` auf main.

## Teil 3f.4: Nahtlos und Abspalten

Lieferumfang (Roadmap, optional): `create_area` mit Ziel „bestehende Weltkarte“ (der Assistent wählt freie Zellen, der Ortsname wechselt ohne Warp), `split_room` für geteilte Räume (bis zu 33 Räume auf einer Karte; bis dahin Plakette „Geteilt“). Begonnen, weil der Agent-Teil des Pflicht-Gates 3f.1 bis 3f.3 grün ist (offen beim Nutzer sind nur Hand- und Blick-Punkte). Zielzellen dürfen leer oder Landschaft (Header 0) sein; die Plakette „Geteilt mit N Räumen“ steht im Inspector-Kopf und im Baum; „Für diesen Raum abspalten…“ nur für Räume auf einer Karte ohne Gebietsabschnitt.

| Punkt | Prüfung | Wer |
|---|---|---|
| (4.1) Lage im Assistenten | Schritt „Anordnung“: „Wo liegt das Gebiet?“ mit „Eigene Karte (über Türen verbunden)“ und „Auf der Weltkarte Sinnoh (nahtlos begehbar)“, Gitter „Weltkarte, 30 Zeilen, 30 Spalten“; Klick auf eine Gebietszelle tut nichts, Klick auf Landschaft wählt; Zusammenfassung nennt Lage und ersetzte Landschaft | Agent |
| (4.2) Nahtlos im Spiel | Szenario `pt-nahtlos.yaml`: „Neuweg“ in der Landschaftszelle (4, 27) östlich von Zweiblattdorf, `g4emu` läuft ohne Tür hinüber, die Einblendung wechselt zu „Neuweg“ | Agent + Blick |
| (4.3) Plakette „Geteilt“ | Pokémon-Center Jubelstadt (0006, Matrix 116, 17 Header): Baumzeile mit Symbol „geteilt“, Inspector-Kopf „Geteilt mit 16 Räumen“ | Agent |
| (4.4) Abspalten | „Aktionen“ › „Für diesen Raum abspalten…“, Dialog „Raum abspalten?“ mit Plakette „Nicht rückgängig“, „Abspalten“: Header 0006 auf neuer Matrix mit eigener Kartenkopie, die anderen 16 bleiben auf Matrix 116 | Agent |
| (4.5) Nur ein Ort | eine Kollisionsänderung an der Kopie lässt die Karte der anderen Pokémon-Center unverändert | Agent |

### Ergebnis 3f.4

Stand: bis `9494656`, Test-Umgebung 1470/1471 (neu gestartet), Kopien `3f4-pt` (Browser), `3f4-pt3` (Szenario; `3f4-pt` und `3f4-pt2` aus früheren Läufen des Szenarios mit anderem Weg).

| Datum | Projekt | Punkt | Ergebnis | Auffälligkeiten |
|---|---|---|---|---|
| 2026-10-26 | 3f4-pt | (4.1) | bestanden: Quelle Sinnoh Zelle 3,26 (Karte 0005); „Wo liegt das Gebiet?“, Gitter „Weltkarte, 30 Zeilen, 30 Spalten“; Klick auf 4,24 (Gebiet) ohne Wirkung, 6,24 (Landschaft) gewählt, „Weiter“ frei; Zusammenfassung „Legt das Gebiet „Neues Gebiet“ mit 1 × 1 Karten an. Liegt auf der Weltkarte Sinnoh bei x 6, y 24 (nahtlos begehbar); ersetzt 1 Landschaftskarte. …“, kein „Raster/Matrix/Header“ (abgebrochen) | Vorgewählte Quelle war die Matrix des gewählten Raums; die Quelle muss man dann auf „Sinnoh“ stellen |
| 2026-10-26 | 3f4-pt3 | (4.2) | bestanden: `pt-nahtlos` exit 0 (`headers.814` = 593, 0 Fehler), Header 415 → 414 → 411 → 593 beim Laufen über x 128 (kein Warp); Bild `target/agent/gate-2026-10-26/pt-nahtlos/einblendung.png`: Einblendung „Neuweg“ oben links, rechts das Gelände der neuen Karte | Weil Zweiblattdorf nach Osten von Bäumen umgeben ist, öffnet das Szenario die Zeile 15 auf beiden Karten (je 6 Kacheln); neben Route 201 gibt es keine freie oder Landschaftszelle mit begehbarem Rand (Begründung im Szenario). Die Roadmap nennt Route 201; geprüft ist dasselbe Verhalten neben Zweiblattdorf |
| 2026-10-26 | 3f4-pt | (4.3) | bestanden: Baumzeile „Pokémon-Center“ mit Symbol „geteilt“ (Titel „Geteilt mit 16 Räumen“), Kopf „Raum · Pokémon-Center · Jubelstadt · C01PC0101 · Geteilt mit 16 Räumen“ | – |
| 2026-10-26 | 3f4-pt | (4.4) | bestanden: Dialog „Raum abspalten? Nicht rückgängig. Der Raum „Pokémon-Center“ bekommt eine eigene Kopie seiner Karte. Danach ändern Änderungen an ihm keinen anderen Raum mehr. Die Kopie lässt sich nicht zurücknehmen.“; „Abspalten“: Toast „Raum „Pokémon-Center“ abgespalten“, Verlauf `["Raum „Pokémon-Center“ abgespalten"]`, Header 0006 auf Matrix 0289 „m_pc01_ (6)“ mit Karte 0667, Header 0036 weiter auf 0116; die Plakette ist danach weg | – |
| 2026-10-26 | 3f4-pt | (4.5) | bestanden: `set_permission` auf Karte 0667 Kachel (5, 5) → `[0, 128]`; Karte 0191 (alle anderen Pokémon-Center) Kachel (5, 5) weiter `[128, 128]`, Prüfsumme der Kollision unverändert | – |

Befunde der Abschlussprüfung von 3f.4 (behoben in `9494656`): der Dialog konnte „0 Karten“ sagen, solange die Übersicht fehlte (jetzt ohne Zahl, wenn sie nicht sicher ist); „Abspalten“ erschien auch für Räume auf einer Weltkarte, die der Kernel ablehnt; ein Fehler beim Nachladen nach dem Abspalten ging verloren. `split_room` ist kein kuratiertes MCP-Werkzeug (über `g4_call`, `docs/mcp.md`).
