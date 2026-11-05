# Phase 6 „Kartenerstellung und Weltformat“: Entscheidungen (2026-11-05)

Anlass: Phase 3d, die Zwischenphase 4.5 und „Release und CI“ sind abgeschlossen (2026-11-05). Die Roadmap nennt Phase 6 „Kartenerstellung und Weltformat“ mit dem Gate „Eine im Studio gemalte Karte wird gebaut, in eine Matrix gesetzt und im Emulator begangen“. Der Nutzer hat Phase 6 mit einem Briefing beauftragt (Ziel, Ausgangslage, harte Grenzen, Lieferumfang, Schnittvorschlag 6.1 bis 6.5, offene Fragen) und zusätzlich gefragt, ob das Malen wie bei [Tiled](https://www.mapeditor.org) gehen kann. Drei Gutachten (nur lesend am Code, mit Messungen an frischen Kopien von Platin DE, HeartGold DE und HeartGold US und mit Proben im Emulator) haben die Ausgangslage und den Vorschlag geprüft:

- [gutachten-a-writer.md](gutachten-a-writer.md): Inventar aller 2 018 Vanilla-Kartenmodelle und 545 Texturpakete, zwingende MDL0-Teile, drei Strategien für den Writer, NSBTX-Writer mit Quantisierung, Lizenz-Audit, Schnitt 6.1; Werkzeuge `gutachten-a-nitro.py` (Inventar, byte-gleicher Round-Trip), `gutachten-a-minimal.py` (Bäcker-Prototyp), `gutachten-a-proto/` (Rust: NSBTX-Writer).
- [gutachten-b-bdhc-engine.md](gutachten-b-bdhc-engine.md): BDHC-Format aus pret und Messung, Höhensuche des Spiels, 20-Einheiten-Regel, Kollisionsregeln aus Vanilla, Erzeugung aus dem Höhenraster, Engine-Grenzen, DSPRE-Leser, Schnitt 6.2; Emulator-Probe auf allen drei Spielen; Werkzeuge `gutachten-b-messung.py`, `gutachten-b-bdhc-bau.py`, `gutachten-b-ramhoehe.py`.
- [gutachten-c-malen-import.md](gutachten-c-malen-import.md): Kartentexturen und Vanilla-Geometrie gemessen, Tileset-Quelle, Malmodell, Smart Drawing, Tiled, Weltformat, Import mit Lizenzen, Oberfläche, Schnitt 6.3 bis 6.5; Werkzeuge `gutachten-c-kartentexturen.py`, `gutachten-c-pdsts.py`.

Die Entscheidungen folgen der Empfehlung („recommend“) der Gutachten. Wo sie sich widersprechen oder etwas offen lassen, steht hier die gewählte Vorgabe mit Grund. Der Nutzer kann jede einzeln umstoßen; die Spezifikation [2026-11-05-phase-6-kartenerstellung-design.md](../../superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md) und die Roadmap setzen sie voraus. Die Spezifikation jeder Teilphase folgt mit deren Brainstorming und Plan.

## Wichtigste Messwerte

| Messung | Platin DE | HeartGold DE | Quelle |
|---|---|---|---|
| Kartenmodelle / Texturpakete (Karten + Gebäude) | 666 / 145 | 676 / 200 (US 676 / 200) | A §2.2 |
| Round-Trip Leser → Writer byte-gleich (Modelle, Pakete, Wörterbücher) | alle | alle (US alle); 13 198 Wörterbücher | A §2.6 |
| Modellgröße Median / Maximum gegen 0xF000 (61 440 B) | 20 KB / 884 B darunter | 23 KB / **20 B darunter** | C §3.1 |
| Flaches 32 × 32-Feld, Quads `VTX_16` / Quad-Strips `VTX_10` / mit vier Texturen | 57 776 B (94 %) / 21 740 B (35 %) / 34 728 B (57 %) | gleich | A §2.5 |
| Polygone: Vierecke / schräg / waagerecht auf dem Kachelraster | 77–81 % / 53–62 % / 66–68 % | | C §3.1 |
| Texturen je Paket (Median), 16 × 16 / 32 × 32 / 64 × 64 | 38; 41 / 32 / 11,5 % | 30; 36 / 47 / 6,4 % | C §2.1 |
| Texturformate der Kartenpakete | 16 Farben 73 %, 4 Farben, A3I5, A5I3, 256 Farben; nie 4 × 4, nie direkt | gleich | A §2.4, C §2.1 |
| Wiederholen S/T steht im Material, nicht in der NSBTX | 87 % | 93 % | C §2.1 |
| VRAM Karten- plus Gebäudepaket je Gebiet, Maximum (Bank 256 KiB) | 110 640 B | 87 424 B | A §2.4 |
| BDHC Nutzlast Median / Maximum gegen 0x9000 | 300 B / 21 452 B | 311 B / 10 778 B | B §2.2 |
| Gemalte BDHC nach Konstruktion höchstens | 35 592 B (< 36 864) | gleich | B §2.6 |
| Schritt verweigert ab Höhenunterschied (Mitte zu Mitte) | 20 E, unabhängig von der Permission; 16 E ohne Treppe erlaubt | gleich (asm) | B §2.4, §2.5 |
| Begehbare Kacheln nicht auf Höhe 0 / Matrix-Zellen mit Höhe > 0 und Header | 57 % / 0 | 61 % / 0 | B §2.2 |
| Höhenkante 16 E, obere / untere Zelle blockiert | 1 837 / 1 172 | 249 / 196 | B §2.7 |
| Gleichzeitig geladene Karten | 2 × 2 | 2 × 2 | B §2.8 |
| Emulator-Probe: Höhe in der Rampenmitte (RAM) / Plateau | 3,989 (Soll 3,989) / 16 | gleich, auch US | B §2.5 |
| Minimal-Modell (ein Knochen, ein Material je Textur, Quads/Strips) im Spiel | Zimmer, Karte 187, eigene Texturen an Paket 20 | Zimmer, Karte 217 | A §2.7 |
| HGSS-Kollisionsbyte: Bits 0–6 als Schrittgeräusch | – | 565 von 676 Karten | B §2.4 |

## Phasenschnitt und Reihenfolge

Der Vorschlag der Hauptsitzung (6.1 Writer ∥ 6.2 BDHC → 6.3 Weltformat → 6.4 Malen ∥ 6.5 Import) bleibt in den Abhängigkeiten richtig, wird aber nach Gutachten C §8 geteilt: Schema, Tileset-Ableitung, Vernetzer und Vorschau (6.3a) brauchen keinen Writer, PNG → NSBTX und OBJ/glTF in binäre Karten (6.5a) kein Weltformat. Ohne die Teilung wäre 6.3 mit zehn Arbeitspaketen die größte und am spätesten beginnende Teilphase.

| Teilphase | Inhalt | Größe (AP) | braucht | Gutachten |
|---|---|---|---|---|
| **6.1 Writer** | `g4-formats`: Wörterbuch mit Patricia-Baum, MDL0-Codec (Leser, Writer, Display-List- und SBC-Codec), NSBTX-Writer und „Textur an Paket anhängen“, Texturkonverter RGBA → 16/4 Farben; `g4-kernel`: `bake` aus einer neutralen Flächenliste (Strips, `VTX_10`/`VTX_16`, Stil je Gebiet) mit Budget-Bericht; `docs/formats/nsbmd.md`; Orakel-Tests apicula und g4emu | M (6) | – | A §4, §5 |
| **6.2 BDHC** | `g4-formats/src/bdhc.rs` (lesen, prüfen, schreiben, `height_at` wie im Spiel), Generator Höhenraster → BDHC, Kollisionsableitung mit Regeltabelle je Familie, Prüfungen `map.altitude-walkable`, `map.step-unblocked`, `map.buildings-over-limit`, Strukturregeln in `map.bdhc-size`; Höhen im Studio aus der BDHC (Overlay, Marker, Picking, Event-Drag, Schicht „Höhen“); Budget je Karte im Inspector; g4emu `expect-height`, Szenarien `pt-gelaende`/`hg-gelaende`; Fix des Rot-Blau-Tauschs in g4emu-Bildern | M (6) | – | B §4, §5 |
| **6.3a Weltformat und Vorschau** | `source: "world"` in `data/maps/NNNN.json` mit `world.version` und Migration, `data/tilesets/NNNN.json` mit Ableitung aus dem Kartentextur-Paket (Bodenkacheln, Vorgaben aus Vanilla-Permissions), synthetisches Test-Tileset aus Code, Store-Art `WorldMap` mit Undo, Vernetzer (Zusammenfassen, Wände, Rampen, Teilmodelle) → Flächenliste, Netz-DTO und Anzeige in der Szene, Budget-Schätzung, Prüfungen | L (6) | Phase 4 (Textprojekt); parallel zu 6.1 und 6.2 | C §5, §8 |
| **6.3b Backen und Build** | Compiler Welt → Map-`.bin` (Flächen → NSBMD über 6.1, Höhen → BDHC über 6.2, Permissions über 6.2), Build-Schritt `world`, Konflikt `world.member-changed`, „Karte als Quelle übernehmen“ (Form 1 verbatim), Budgets als Prüfungen, gated Tests, Emulator-Gate | M–L (4) | 6.1, 6.2, 6.3a | C §5.3, §8 |
| **6.4a Malen: Werkzeuge und Rezept** | Pinsel, Rechteck, Füllen, Radierer, Pipette, Höhe, Rampe, Ebenen, Palette, ein Strich = ein Undo-Schritt, Höhen-Overlay, Rezept `new_painted_map`, MCP `world_get`/`world_edit`/`tileset_get`, Szenarien `pt-gemalt`/`hg-gemalt` | L (5) | 6.3a (Emulator-Teil des Gates: 6.3b) | C §7, §8 |
| **6.4b Smart Drawing und Muster** | Wang-Ecken-Sätze im Tileset, Kantensatz-Editor (13 + 2 Plätze), Terrain-Pinsel, Klippenpinsel, Zufall, Gelände-Muster-Shader, `.fig`-Frames | M (3) | 6.4a | C §3.4, §8 |
| **6.5a PNG und Modelle** | PNG → NSBTX (Quantisierung, Anhängen ans Paket) in der Oberfläche, OBJ/glTF → Modell-Teil einer binären Karte über `map_import` (Vorschau, Budgets, Ablehnen mit Grund), Doku „aus PDSMS über OBJ“ | M (3) | 6.1 | C §6, §8 |
| **6.5b Freiform und Austausch** | glTF als `freeform` und als Teilmodell im Tileset, Form 2 von „übernehmen“ (Geometrie als glTF über apicula), TMJ-Export und -Import | M (3) | 6.3b, 6.5a | C §4, §5, §8 |

Reihenfolge: (6.1 ∥ 6.2 ∥ 6.3a) → (6.3b ∥ 6.5a) → (6.4a ∥ 6.5b) → 6.4b. Zusammen 36 Arbeitspakete. „∥“ heißt parallel in eigenen Worktrees. Berührungen: 6.2 (Höhen in der Szene) und 6.3a (Netz in der Szene) teilen `apps/studio/src/world/scene/*`; 6.2 mergt zuerst, 6.3a zieht nach. 6.3a und 6.4a teilen Store und Szene; 6.5a berührt `map_import.rs`, den Import-Dialog und die Writer-Seite von `g4-formats` (nach dem Merge von 6.1). Anhängepunkte wie immer: `commands/dispatch.rs`, `mcp/tools.rs`, `docs/mcp.md`, `api.ts`, `ROADMAP.md`, `design-system/README.md`, `.fig`; für die Merges gilt die Erfahrung aus Phase 4 und 3d.

## Writer (Gutachten A)

| Frage | Entscheidung | Grund |
|---|---|---|
| Strategie | **(c)**: MDL0-Codec (Leser und Writer) in `g4-formats`, byte-gleicher Round-Trip auf allen Vanilla-Kartenmodellen und Texturpaketen der drei Spiele als **gated Test**, nicht als Produktpfad; der Bäcker erzeugt die minimale Teilmenge und schreibt über denselben Writer; unveränderte Modelle und Pakete bleiben verbatim | im Prototyp gezeigt (2 018 Modelle, 545 Pakete, 13 198 Wörterbücher byte-gleich); der Patricia-Baum fällt in apicula nicht auf, im Spiel aber als untexturierte Fläche, nur Byte-Gleichheit prüft ihn; Malen auf Vanilla-Karten bleibt später möglich |
| Teilmenge des Bäckers | ein Modell, ein Knochen `0xF807`, SBC `26 02 0b (04 05)* 2b 01`, ein Material und ein Shape je Textur mit Vanilla-Werten (`poly_attr_mask` `0x3F1FF8FF`, Wiederholen S/T, `flag` `0x1FCE`, `mag` 1,0), Quad-Strips über Läufe gleicher Textur und Ebene, Quads für den Rest, `VTX_10` auf dem Raster, sonst `VTX_16`, `posScale` 64, `TEXCOORD` nur bei Wechsel, Strips vor ±2 048 Texeln geteilt | ohne Strips braucht ein flaches Feld 94 % von 0xF000; Dreiecke, mehrere Knochen, Texturmatrix und Alpha < 31 liest und schreibt der Codec, der Bäcker braucht sie nicht |
| Stil je Gebiet | außen: Licht 0, Nebel, `dif_amb` `0x7FFFE739`, eine Normale je Ebene; innen: Licht aus, `dif_amb` `0x7FFF8000`, Farbe weiß; Regel Platin Lichtarchiv 0 oder 3 = außen (pret), HeartGold Lichttyp 1 = außen (gemessen, A offener Punkt 2) | außen ersetzt das Spiel Materialfarben durch die Tageszeit; eine beleuchtete Fläche ohne Normale wird falsch hell; die HeartGold-Regel prüft das Gate von 6.3b an einer Außenkarte bei Tag und Nacht |
| Eingabe des Bäckers | **neutrale Flächenliste** (vier Ecken, UV in Texeln, Texturschlüssel, Ebene, Normale oder Farbe); der Vernetzer aus 6.3a erzeugt sie aus dem Kachelraster und liefert dieselbe Liste als Netz-DTO an die Szene | A offener Punkt 1 mit C §3.5: ein Vernetzer für Vorschau und Backen; 6.1 prüft den Bäcker ohne Malen mit einer Testfläche (Schachbrett plus Block) |
| NSBTX | Writer im Vanilla-Layout mit Patricia-Baum; Grundoperation **„Textur an Paket anhängen“**; Vanilla-Pakete bleiben sonst verbatim; Prüfungen Zweierpotenz, Name ≤ 16 Byte und eindeutig, Paketgröße und VRAM | mehrere Gebiete teilen ein Paket; Anhängen lässt alte Einträge byte-gleich, Ersetzen oder Löschen träfe alle |
| Texturformate | 16 und 4 Farben schreiben; A3I5, A5I3 und 256 Farben später; 4 × 4-komprimiert und direkt nie | Vanilla-Karten nutzen 16 Farben zu 73 %, nie 4 × 4 oder direkt |
| Quantisierung | **eigener Median-Cut** in BGR555 mit `color0` als Transparenz, **ohne Dithering**, eine Palette je Textur; keine neue Abhängigkeit | A §2.9: kein lizenzverträgliches Crate schreibt NSBMD/NSBTX (`romkit` liest nur); `imagequant` ist GPL und ausgeschlossen; C nannte `quantette` (MIT/Apache) als Rückfall, falls die eigene Quantisierung nicht reicht; Pixel-Art-Kacheln vertragen Dithering meist nicht; gemeinsame Paletten später |
| Textur-Pakete neu anlegen | **nicht in Phase 6**: eine gemalte Karte gehört zum Kartentextur-Paket ihres Gebiets (anhängen); eigene Pakete und eigene Gebietsdaten mit Licht und Gebäudepaket später | A offener Punkt 3; hält 6.1 und 6.4 klein, „Neue Karte malen“ nimmt die Gebietsdaten des Nachbarn oder ein gewähltes Gebiet |
| VRAM-Grenze | Warnung über dem Vanilla-Maximum (110 640 B Platin, 87 424 B HeartGold für Karte plus Gebäude); keine harte Grenze, bis der Anteil der Figuren gemessen ist | A offener Punkt 4, B §2.8: harte Zahl nicht ermittelt; `Easy3D_UploadTextureToVRAM` endet sonst in `GF_ASSERT` |
| Modul im Code | neues Modul `nsbmd` (Struktur, Leser, Writer) und `nitro_dict`; `nsbmd_header` bleibt als dünne Hülle für die heutigen Aufrufer (`map_import.rs`: Kopf, Zahl der Modelle, Namen) | A offener Punkt 7; keine Umstellung der Aufrufer in 6.1 |
| Quelle in `docs/formats/nsbmd_header.md` | die Nennung von PDSMS als Layout-Quelle wird mit `docs/formats/nsbmd.md` durch pret, apicula (0BSD), GBATEK und eigene Messung ersetzt | A §8: das PDSMS-Repo hat keine Lizenz |
| Vorschau gebackener Modelle | gebackene Modelle gehen wie heute über apicula → GLB in die Szene; die **Mal-Vorschau** zeichnet die Flächenliste direkt in three.js (6.3a) | A offener Punkt 8 mit C §3.5: kein apicula-Lauf je Pinselstrich |

## BDHC, Kollision, Engine-Grenzen (Gutachten B)

| Frage | Entscheidung | Grund |
|---|---|---|
| Terrain gemalter Karten | **Option B**: BDHC aus dem Höhenraster; je Zelle `h` (ganzzahlig, Stufen zu 8 Einheiten, Bereich −16 bis 62) und `ramp` (keine oder N/O/S/W mit 1 oder 2 Stufen je Kachel, nur achsparallel); Platten per Rechteck-Zerlegung je Ebene, Streifen listen nur überlappende Platten; Vanilla-BDHC bleiben verbatim | im Emulator auf drei Spielen bestätigt (Rampenhöhe 3,989 gleich Soll); nach Konstruktion ≤ 35 592 B < 0x9000; dasselbe Raster ist Quelle für Modell, BDHC und Kollision; Option C (aus dem Modell abtasten) machte Dächer begehbar und hinge am Writer |
| Höheneinheit | 1 Stufe = 8 Einheiten = eine halbe Kachel (16 E je Kachel); Anzeige als Stufen, sieben Farben cividis im Zyklus, Isolinien bei Stufenwechsel | B und C meinen dasselbe Raster (B offener Punkt 2) |
| Codec | `g4-formats/src/bdhc.rs`: lesen, prüfen (Indizes, Sortierung, Streifenbereiche als Fehler, `n_y` = 0 als Warnung), schreiben, `height_at(x, z, current)` wie die Höhensuche des Spiels; `docs/formats/bdhc.md`; gated Test: alle 2 018 Vanilla-BDHC byte-gleich, Strukturregeln 0 Treffer | B §4 |
| Kollisionsableitung | Regeltabelle aus B §2.7 je Familie als Metadaten (`metadata/terrain_rules.yaml` oder in `check_limits.yaml`): blockiert 0x00/0x80, Boden 0x00/0x00, hohes Gras 0x02 (sehr hoch 0x03), Wasser 0x15 Meer / 0x10 Fluss, Eis 0x20, Vorsprung 0x38–0x3B mit 0x80, Tür 0x69 mit 0x80, Treppe = Neigung ohne eigenes `kind`; Höhenkante 1 Stufe frei, **ab 2 Stufen ohne Treppe blockiert die obere Zelle**; Steigung ≥ 20 E je Kachel nicht erlaubt; HGSS-Schrittgeräusch Vorgabe 0, je Kachel im Tileset einstellbar; Handwerte je Zelle überschreiben die Ableitung und bleiben | B §2.7, §4 |
| Obere oder untere Zelle an der Kante | **obere** (B), nicht untere (C) | in Vanilla häufiger (1 837 : 1 172 bzw. 249 : 196); der Fix von `map.step-unblocked` blockiert ebenfalls die obere; für Engine und BDHC gleichgültig; im Kantensatz-Editor (6.4b) je Satz umstellbar |
| Richtungssperren `kind` 0x30–0x37 | nicht in Phase 6 | belegen das `kind` (kein Gras auf der Kante), für HGSS nicht belegt |
| Neue Prüfungen | `map.altitude-walkable` (Fehler: Zelle mit Header und Matrix-Höhe ≠ 0; Vanilla 0 Treffer), `map.step-unblocked` (Hinweis mit Fix „obere Zelle blockieren“ oder „Treppe setzen“), `map.buildings-over-limit` (Warnung ab 32), `map.bdhc-size` um Strukturregeln erweitert; BDHC über 0x9000 bleibt **harter Fehler** | über 0x9000 überschreibt das Spiel den Heap ohne Meldung (B §2.4) |
| Höhen im Studio | Overlay, Marker, Picking und Event-Drag nehmen für **alle** Karten (auch Vanilla) `height_at` in der Kachelmitte statt der Matrix-Ebene; Kernel-Befehl `get_map_heights` (32 × 32); Schicht „Höhen“ | heute liegt alles auf der Matrix-Ebene (Höhe 0), obwohl 57 % / 61 % der begehbaren Kacheln nicht auf 0 liegen; die Ausgangslage im Briefing („folgen der Oberfläche“) stimmte nicht |
| Matrix-Höhe | bleibt, was sie ist (Kulisse); keine Ableitung aus ihr | 0 Zellen mit Höhe > 0 tragen einen Header |
| `.bdhcam` | nicht unterstützt; der Import sagt bei dieser Endung, dass sie einen DSPRE-Patch braucht, den es für deutsche ROMs nicht gibt | B §2.3 |
| Brücken (zwei Ebenen) | nicht in Phase 6 | B offener Punkt 3, C offene Frage 4; eine Ebene je Zelle hält Raster, BDHC und Kollision einfach |
| Budget je Karte | im Inspector: Modell (Bytes und Anteil an 0xF000), BDHC (Anteil an 0x9000), Texturen und VRAM des Pakets gegen das Vanilla-Maximum, Gebäude (gegen 32), Polygone als Hinweis ab Vanilla-Maximum | B §2.8; Polygon- und VRAM-Budget sind nicht hart bestimmbar |
| g4emu | Schritt `expect-height Y [±T]` (RAM-Suche wie `gutachten-b-ramhoehe.py`, Mehrheit der Treffer); Fix des Rot-Blau-Tauschs in `scripts/g4emu/g4emu/emu.py` (A §8) in 6.2 | Gates von 6.2 und 6.3b messen Höhe statt Bild; Farbvergleiche in späteren Gates brauchen richtige Farben |
| DSPRE | prüft die BDHC beim Lesen nicht; eine gemalte Karte braucht nur gültige Längen und (HGSS) den BGS-Block | B §2.9; Hand-Punkt im Gate von 6.3b |

## Malen, Weltformat, Import, Tiled (Gutachten C)

| Frage | Entscheidung | Grund |
|---|---|---|
| Tileset-Quelle | **Kacheln zur Laufzeit aus dem Kartentextur-Paket des Gebiets** (`map_textures` über `area_data.map_tileset`); Metadaten im Textprojekt unter `data/tilesets/NNNN.json` (Kacheln mit Rolle, Gelände-Vorgabe, Kollision, Höhenverhalten, HGSS-Schrittgeräusch; Kantensätze ab 6.4b; Teilmodelle als Beifiles); eine Bodenkachel ist ein wiederholbares Material mit 16 Texeln je Kachel und welt-gebundener UV; eigene Tilesets nur als synthetisches Test-Tileset aus Code; PDSMS-Tilesets nicht | so sind die Vanilla-Karten gemessen gebaut (kein Atlas); PDSMS hat keine Lizenz und seine Tilesets sind Spielgrafik |
| Vanilla-Tileset-Metadaten im Repo | **nein**, auch nicht als Indizes und SHA-1; die Ableitung läuft im Projekt des Nutzers und in den gated Tests auf Testkopien; im Repo liegt nur das synthetische Test-Tileset | harte Grenze „keine ROM-Ableitungen nach GitHub“ (C offene Frage 1) |
| Vorgaben je Textur | Ableitung aus den Vanilla-Permissions (Textur → häufigstes `kind`), Eindeutigkeit misst Task 0 des Plans von 6.3a; unklare Texturen bekommen „Boden“ und sind im Tileset-Editor umstellbar | C §2.4 |
| Malmodell | **M1**: Zellraster je Ebene mit Höhe je Zelle (Stufen, siehe B), automatische Wände an Höhenkanten, Rampen und Treppen als Kacheln mit Höhenverhalten, Bäume, Zäune und Sockel als Teilmodelle mit Fußabdruck; Höhe je Ecke (M2) nach 6.4 neu bewerten | C §3.2; deckt sich mit B |
| Smart Drawing | Datenmodell **Wang-Ecken-Sätze** wie Tiled, Bedienung ein Kantensatz-Editor mit den 13 PDSMS-Plätzen plus zwei Diagonalen; Terrain-Pinsel, Klippenpinsel; in 6.4b | C §3.4; die Füll-Logik entsteht eigenständig, nicht aus Tileds GPL-Teil |
| Draufsicht und Vorschau | Draufsicht über die vorhandene three.js-Orthokamera (`SceneHost.setTopDown`), kein eigenes Canvas; Live-Vorschau zeichnet das Netz aus dem Rust-Vernetzer (Netz-DTO über die Brücke), derselbe Vernetzer liefert die Flächenliste an den Bäcker | C §3.5, §7.4 |
| Tiled | **T2: Modell und Werkzeuge im Studio nachbauen** (Ebenen, Tileset mit Eigenschaften, Stempel, Rechteck, Füllen, Radierer, Pipette, Terrain-Pinsel); **TMJ-Export und -Import nur als Austausch in 6.5b**; Tiled allein als externer Editor nicht; keine Tiled-Erweiterung in Phase 6 | Tiled kennt keine Höhe, kein 3D, kein Undo im Store und kein MCP; es gibt keine JS-Bibliothek mit Editor-Logik, nur Lader und Renderer; Tiled und seine Plugins sind GPL, `libtiled` BSD-2 (nur das Format wird genutzt) |
| Weltformat | eigenes JSON in `data/maps/NNNN.json` mit `source: "world"` (reserviertes Feld) und `world.version`; Ebenen mit `tiles` und `heights` als 32 Zeilen wie die Permissions (Kachel vierstellig hex, `r1..r3` Drehung, `f` Spiegeln, `.` leer; Höhe als Stufe), `pieces` (Teilmodelle), `freeform` (glTF, ab 6.5b), `permissions` mit `derive` und `override` je Zelle; Gebäude und `bgs` wie heute; Schema-Version der Art in `manifest.schemas` als `"maps.world"`; eine Migration der Projektversion wie bei 3d.4 | C §5.2; TMJ als Weltformat abgewogen und verworfen (keine Höhe, keine Ableitung, fremdes Schema) |
| Build | neuer Schritt `world` vor `textproject`: Eingaben Welt-JSON, Tilesets, Beifiles, Texturpakete; Ausgaben die Member `unpacked/maps/NNNN`; Salz aus Compiler-, Writer- und BDHC-Revision; inkrementell je Karte; der Member ist Ausgabe, nicht Quelle; ein außen geänderter Member einer Welt-Karte ist der Konflikt `world.member-changed` mit „als binäre Karte übernehmen“ oder „neu backen“ | C §5.3 |
| Store | neue Asset-Art `WorldMap` (hält das Weltdokument, nicht die Member-Bytes); ein Pinselstrich ist ein Undo-Schritt; der Member entsteht beim Speichern oder Bauen; die Szene holt bei Welt-Karten das Netz statt des GLB | C §5.3 |
| Nur im Textprojekt | gemalte Karten nur im Textprojekt; im DSPRE-Layout sagt das Werkzeug „Boden malen gibt es im Textprojekt (Technik › Textprojekt)“ | Entscheidung aus 3d: neue Projekt-Daten nur im Textprojekt |
| „Karte als Quelle übernehmen“ | Permissions (als Überschreibung), Gebäude, `bgs` 1:1; Höhen durch Abtasten der BDHC an den Kachelmitten mit Liste der Abweichungen; Geometrie **Form 1 verbatim** (`freeform: [{ "nsbmd": … }]`, Raster leer) in 6.3b, **Form 2 als glTF** über apicula in 6.5b; Vanilla-NSBMD zurück in ein Kachelraster **nie** | 53 bis 62 % schräge Polygone und 34 % freie Bodenfläche machen die Rückwandlung unzuverlässig; Prüfstein: alle 666 / 676 Vanilla-Karten übernommen mit Form 1 backen byte-gleich |
| Git LFS | nein; `.gitattributes` mit `*.png binary` und `*.glb binary` | Beifiles sind klein |
| Import | OBJ mit `tobj` (MIT), glTF mit `gltf` (MIT/Apache-2.0), Triangulation, Streifen, `posScale`-Wahl, Budget 0xF000 mit Ablehnung; PNG → NSBTX über den Konverter aus 6.1; **PDSMS nur über PDSMS' eigenen OBJ-Export**, kein `.pdsmap`/`.pdsts`; Lizenz-Audit (`cargo deny`) je Crate im Plan von 6.5a | C §6; PDSMS hat keine Lizenz, `.pdsts` ist ein undokumentiertes Binärformat |
| Oberfläche | Werkzeug „Gelände malen“ in der Tiefe **Bauen** des Welt-Explorers (Draufsicht und Picking vorhanden), Palette, Ebenen, Höhen-Overlay und Muster-Shader nach Doppeldeck; Rezept `new_painted_map` (leere oder kopierte Karte in eine Matrixzelle, ein Undo-Schritt, auch über MCP `recipe_*`); MCP `world_get`, `world_edit`, `tileset_get` | C §7; Roadmap-Lieferumfang „Rezept Neue Karte malen“ |
| Malen über Kartengrenzen | je Karte; der Pinsel sieht Nachbarkarten der Matrix nur lesend (Kantensätze an der Naht, 6.4b) | C offene Frage 7 |
| Wasser-Animation | außerhalb des Malens; ob eine gemalte Wasserkachel ohne Animation stört, zeigt das Gate von 6.3b | C offene Frage 8 |
| Vanilla-Teilmodelle herauslösen | nicht in Phase 6; in 6.5b nur importierte Stücke | C offene Frage 3 |

## Nicht in Phase 6

- Brücken und mehrere Ebenen an einer Stelle; Höhe je Ecke (M2); Rampen diagonal oder an Ecken; Richtungssperren 0x30–0x37.
- Eigene Texturpakete und eigene Gebietsdaten anlegen; Texturen ersetzen oder löschen; A3I5, A5I3, 256 Farben, 4 × 4 und direkte Texturen schreiben; gemeinsame Paletten; Dithering.
- Vanilla-NSBMD in ein Kachelraster zurückwandeln; Vanilla-Teilmodelle herauslösen; Bearbeiten von Vanilla-Modellen über den Codec (lesen, Shapes ergänzen) ohne Malen.
- `.bdhcam`; `.pdsmap` und `.pdsts`; Tiled-Erweiterung; Automapping und Kantensätze über die 13 + 2 Plätze hinaus; mehrere Karten gleichzeitig malen; Matrix-weites Weltdokument.
- Wasser- und Texturanimation, `dynamic_texture_type`; Kamera und Licht je Karte; Gebäude-Modelle (`.bld`-NSBMD) schreiben.
- Kampfsprites schreiben (aus 3d verschoben, „Quelle PNG → Member“): bleibt offen, bis die PNG → NSBTX-Strecke (6.5a) steht; dann eigener Posten.
- Stadtkarte und Flugziele neuer Orte; Git LFS.

## Was bewusst nicht entschieden ist

- Die harte VRAM-Grenze je Gebiet (Anteil der Figuren und Effekte am Textur-VRAM im Feld; Messung per VRAM-Abzug in g4emu, B offener Punkt 7). Bis dahin nur Warnung über dem Vanilla-Maximum.
- Die Gebäudegrenze in HGSS im Assembler (Vanilla-Maximum 30, Platin-Grenze 32 aus pret). Bis dahin Warnung ab 32 für beide.
- Die Stilregel HeartGold „Lichttyp 1 = außen“ (gemessen, nicht in pokeheartgold nachgelesen). Das Gate von 6.3b prüft eine gemalte Außenkarte bei Tag und Nacht.
- Wie eindeutig die Gelände-Vorgabe je Textur aus den Vanilla-Permissions folgt (Task 0 des Plans von 6.3a).
- Ob der eigene Median-Cut für Fotos oder weiche Verläufe reicht; `quantette` (MIT/Apache-2.0) bleibt der Rückfall nach Lizenz-Audit.
- Was das Verkaufs-ROM tut, wenn ein Paket nicht in den VRAM passt (`GF_ASSERT` in `area_data.c`): Halt oder weiter ohne Texturen.
- Die Kantensatz-Regel „obere Zelle blockiert“ ist je Satz umstellbar (6.4b); ob die Vorgabe für Innenräume anders sein sollte, zeigt das Malen.
