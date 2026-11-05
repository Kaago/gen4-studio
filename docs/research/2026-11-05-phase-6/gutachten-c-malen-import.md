# Gutachten C: Malen, Weltformat, Import und Tiled als Vorbild (Phase 6)

Stand: 2026-11-05. Gutachter: Architektur-Review (nur lesend am Code, mit Messungen an frischen Kopien). Repo: `/Users/POLTEP/Projekte/lab/editor/gen4-studio`, Branch `main` (`91ba0615`).

Gelesen: `docs/ROADMAP.md` (Phase 6, „Aus dem Design-System, später“, 3f), `docs/formats/{map,nsbtx,nsbmd_header,area_data,text_project,project_git,build_state,recipes}.md`, `docs/mcp.md` (Rezepte, Unumkehrbares), `docs/research/2026-10-20-dspre-und-suite-analyse.md` (PDSMS, Tiled, Plugin-Slots), das Vorbild `docs/research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md`; im Code `crates/g4-formats/src/{map,nsbtx,nsbmd_header,area_data}.rs` (Köpfe), `crates/g4-kernel/src/{map_import,export,tiles}.rs`, `recipes/{area,extend,room,types}.rs`, `textproject/schema/map.rs`, `textproject/source/mod.rs`, `build/step.rs`, `metadata/check_limits.yaml`; im Frontend `apps/studio/src/world/{layout,paint}.ts`, `world/scene/{SceneHost,camera,picking,content,draft,canvasClicks}.ts`, `workbench/tools.ts`, `ui/tokens/tokens.ts`, `src-tauri/src/commands/world.rs` (`set_permission_impl`); Design-System `design-system/README.md` (Prinzipien, Muster, Szene), `design-system/tokens/Szene/Wert.tokens.json`, `gen4-studio/design-system/Audit-Design-System.md` (Screen 7). **Nicht geöffnet:** DSPRE (AGPL) und jeder Quelltext von PDSMS (das Repo trägt gar keine Lizenz, Abschnitt 2.3).

Messungen (Skripte neben diesem Gutachten; sie lesen nur und geben nur Zahlen aus, keine Namen, keine Pixel):

| Skript | Was es misst |
|---|---|
| `gutachten-c-kartentexturen.py` | je Kartentextur-Paket (`unpacked/mapTextures`) Zahl, Größe, Format, Wiederholen-Bits der Texturen; je Karte (`unpacked/maps`) Größen von NSBMD und BDHC, MDL0-Kopfzahlen, Display-Listen ausgeführt (Primitivarten, Lage der Ecken zum Kachelraster, Höhen, Ausdehnung und Texturdichte der waagerechten Polygone, UV-Spannen), Wiederholen-Bits der Materialien, Anteil der benutzten Paket-Texturen, Permissions (`kind`, `collision`) |
| `gutachten-c-pdsts.py` | grobe Lage einer PDSMS-Tileset-Datei: Container, druckbarer Anteil, Zahl eingebetteter `.png`- und `.obj`-Namen |

Datenbasis: frische APFS-Klone (`cp -cR`) der DSPRE-Ordner von Platin DE und HeartGold DE unter `Roms/_testkopien/6-gutachten-c-{pt,hg}`; Originale nur kopiert, unter `Roms` nichts gelöscht, die Kopien bleiben stehen. Formatwissen der Display-Listen: öffentliche GX-Befehlsliste des DS (GBATEK) und das MDL0-Layout, wie apicula (0BSD) es liest; `docs/formats/nsbtx.md`, `nsbmd_header.md`. Laufzeit beider Läufe zusammen rund 5 s. Eine PDSMS-Tileset-Datei (13 075 Byte, öffentlich auf GitHub) wurde nur in den Werkzeug-Ergebnisordner der Sitzung geladen und gemessen, nicht ins Repo. Zwei andere Gutachter liefen parallel. **Gemessen** steht bei Zahlen aus den Skripten, **geschätzt** bei Rechnungen ohne Lauf.

---

## 1. Anlass

Phase 6 soll Karten im Studio malen lassen: Tileset aus NSBTX und PNG, Pinsel, Fläche, Höhe, Smart Drawing, 2D-Draufsicht (Pflicht), 3D (optional), Baking zu NSBMD + BDHC + Kollision, ein textuelles Weltformat mit Compiler, Import von OBJ, glTF, PDSMS und PNG, und das Rezept „Neue Karte malen“ in der Tiefe Bauen, auch über MCP. Das Gate der Roadmap: „Eine im Studio gemalte Karte wird gebaut, in eine Matrix gesetzt und im Emulator begangen.“ Der Nutzer fragt zusätzlich, ob das Malen „wie bei Tiled“ gehen kann. Dieses Gutachten beantwortet sieben Fragen: Tileset-Quelle (§2), Malmodell (§3), Tiled (§4), Weltformat (§5), Import (§6), Oberfläche (§7), Schnitt (§8). Die Writer (NSBMD, NSBTX) gehören Gutachter A, BDHC und Engine-Grenzen Gutachter B; hier steht nur, was das Malen von ihnen braucht.

---

## 2. Tileset-Quelle

### 2.1 Befund: wie Vanilla-Kartentexturen beschaffen sind (gemessen)

| Größe | Platin DE | HeartGold DE |
|---|---|---|
| Kartentextur-Pakete (`mapTextures`) | 74 | 106 |
| Texturen gesamt | 3 130 | 3 659 |
| Texturen je Paket min / Median / p90 / max | 3 / 38 / 75 / 94 | 1 / 30 / 63 / 126 |
| Paletten je Paket (Median) | 38 | 28 |
| Texelbytes je Paket Median / max | 14 848 / 76 384 | 11 792 / 56 576 |
| Größen | 16×16 41 %, 32×32 32 %, 64×64 11,5 %, 8×8 5,8 %, 16×32 5,5 %, Rest < 3 % | 32×32 47 %, 16×16 36 %, 64×64 6,4 %, 8×8 5,6 %, Rest < 3 % |
| Formate | 16 Farben 2 269, 4 Farben 599, A3I5 198, A5I3 40, 256 Farben 24; **kein 4×4 (Format 5)** | 16 Farben 2 662, 4 Farben 871, A5I3 96, A3I5 27, 256 Farben 3; kein 4×4 |
| `color0` (Index 0 durchsichtig) | 970 | 1 183 |
| Wiederholen/Spiegeln in den NSBTX-Parametern | bei allen 0 | bei allen 0 |
| Wiederholen in den **Materialien** der Karten-NSBMD (S und T) | 6 369 von 7 346 (87 %); nur S 346, nur T 96, keins 535 | 12 283 von 13 247 (93 %); Spiegeln nur 2 |
| verschiedene Texturnamen / in mehr als einem Paket | 1 443 / 258 | 1 958 / 300 |
| Anteil der Paket-Texturen, die eine Karte benutzt (Median / p90) | 21 % / 45 % | 31 % / 65 % |

Und an den Karten selbst (Display-Listen ausgeführt, Platin 666, HeartGold 676 Karten, alle lesbar, kein unbekannter GX-Befehl):

| Größe | Platin DE | HeartGold DE |
|---|---|---|
| Texturdichte waagerechter Polygone: **16 Texel je Kachel** | 78 196 von 105 895 waagerechten Polygonen (≈ 74 %) | 111 290 von 156 864 (≈ 71 %) |
| 1×1-Polygone auf Texturen > 16 Texel: Spanne 16 Texel | 5 208 von 5 612 | 12 272 von 13 218 |
| davon genau ein 16×16-Ausschnitt im 16er-Raster, nicht am Ursprung der Textur | 1 048 von 1 532 | 1 408 von 1 852 |
| UV-Spanne je Polygon ≤ eine Texturgröße | 67 % (p90: 4, max 32 Texturgrößen) | 79 % (p90: 3, max 32) |

**Folgerung.** Vanilla-Kartentexturen sind **kein Atlas aus Einzelkacheln**, sondern wiederholbare Materialien in einer festen Dichte: eine Kachel sind 16 Texel, ein 32×32-Gras deckt 2 × 2 Kacheln und wiederholt sich (Material-Bit „Wiederholen“ bei fast allen), ein 64×64-Weg 4 × 4. Welcher Ausschnitt einer größeren Textur auf eine Zelle fällt, folgt aus der Lage in der Welt (die UV läuft über die Fläche weiter), nicht aus einer Zellen-Kennung. Das ist genau PDSMS' „globale Texturierung“ und macht das Zusammenfassen gleicher Flächen zu großen Polygonen möglich (Vanilla: 2×2, 4×4, 8×8-Polygone kommen tausendfach vor, §3.1). Wiederholen ist eine Eigenschaft des **Materials im NSBMD**, nicht der NSBTX: das muss der Writer von Gutachter A setzen. Eine Karte nutzt im Mittel nur ein Fünftel bis ein Drittel ihres Pakets, und Pakete teilen Namen: das Paket ist das Tileset eines Gebiets (`area_data.map_tileset`), nicht einer Karte.

### 2.2 Optionen

| Option | Vorteile | Nachteile |
|---|---|---|
| **(a) eigene, mitgelieferte Tilesets** (selbst gemacht oder CC0) | im Repo erlaubt, Tests ohne ROM, Gate ohne Spielgrafik | niemand im Projekt zeichnet DS-Grafik; CC0-Sammlungen (etwa OpenGameArt, Kenney) haben einen anderen Stil und andere Dichte; eine Karte mit fremden Texturen braucht PNG → NSBTX (6.5) und Platz im Gebietspaket, also erst nach dem Import nutzbar |
| **(b) Kacheln zur Laufzeit aus den NSBTX des ROMs** | sofort der Spielstil, kein Asset im Repo, Texturen sind schon im Paket des Gebiets (kein VRAM- und Paket-Zuwachs), Dekoder schon da (`g4-formats::nsbtx`, alle Formate außer 4×4, das Vanilla nicht nutzt) | Vanilla trägt keine Bedeutung je Textur (Gras, Wegkante, Wasser): die Metadaten (Gelände-Art, Kantensatz) muss jemand anlegen; Texturnamen sind kryptisch |
| **(c) PDSMS-Tilesets (`.pdsts`)** | Community-Bestand mit Kacheln samt Geometrie und Smart-Drawing-Sätzen | rechtlich und technisch schlecht, 2.3 |

### 2.3 PDSMS: Lizenz- und Formatlage (geprüft)

- **Lizenz:** `Trifindo/Pokemon-DS-Map-Studio` (276 Sterne, letzter Push 2024-03-26, Release 2.2 von 2021-06-23) hat **keine Lizenzdatei** (GitHub-API: `license: null`, `/license` 404), ebenso der aktive Fork `AdAstra-LD/Pokemon-DS-Map-Studio` (letzter Push 2026-09-24). Ohne Lizenz gilt „alle Rechte vorbehalten“: Code weder lesen-und-nachbauen noch übernehmen; die Vermutung „GPL“ trifft nicht zu, es ist **strenger**.
- **Tilesets:** 30 `.pdsts` liegen im Repo unter `src/main/resources/tilesets/` (13 KB bis 445 KB, mit 1 318 PNG). Die Namen der eingebetteten Texturen der gemessenen Datei sind die internen Texturnamen des Spiels (Muster `la_…`), die Ordner heißen „Tileset 13 – Overworld (by Jiboule)“ usw.: das sind **aus den Spielen gerissene Texturen** mit Nacharbeit einzelner Community-Mitglieder, ohne Lizenz. Für das Repo verboten (ROM-Daten), für Nutzer allenfalls ihre eigene Sache.
- **Format (gemessen, `gutachten-c-pdsts.py`):** eigenes Binärformat (kein ZIP, kein JSON, keine Java-Serialisierung), erstes `u32` = 9, 13 % druckbare Bytes, 7 eingebettete `.png`-Namen (Materialien mit Palettennamen), 28 eingebettete `.obj`-Namen (je Kachel Geometrie). Öffentliche Beschreibung gibt es keine; die Doku von DS Pokémon Hacking beschreibt nur die Bedienung (Kachel = OBJ mit Fläche 1 bis 6 Zellen, Drehung in 90°, Versatz X/Y/Z, „X/Y Tileable“, „U/V Tileable“, globale Texturierung, Smart-Drawing-Satz aus Mitte, vier Kanten, vier Innen- und vier Außenecken, acht Ebenen, Höhe je Zelle und Ebene). AdAstra-Tilesets lassen sich in Trifindos Fassung oft nicht laden.
- **Export von PDSMS:** OBJ mit Texturen → IMD → NSBMD; das letzte Stück braucht laut README Nintendos `g3dcvtr.exe` (nicht weitergebbar). PDSMS selbst schreibt **OBJ**: genau das liest unser OBJ-Import (6.5).

### 2.4 Empfehlung (recommend)

**(b) als Quelle der Kacheln, (a) nur als synthetisches Test-Tileset aus Code, (c) nicht als Tileset-Quelle.**

- **Tileset = ein Kartentextur-Paket (`map_textures` NNNN) plus Projekt-Metadaten.** Beim ersten Öffnen leitet der Kernel ein Start-Tileset ab: je Textur eine „Bodenkachel“ (Material: Textur + Palette, Dichte 16 Texel je Kachel, Wiederholen an, Größe der Kachelgruppe = Texturgröße / 16, also 1×1, 2×2, 4×4), dazu die Gelände-Art, die Vanilla-Karten unter dieser Textur am häufigsten tragen (Abgleich Polygon → Zelle → `kind`/`collision` über die Karten, die das Paket nutzen; zu messen in 6.3a, hier nicht gemessen). Der Nutzer verfeinert: Namen, Kantensätze (§3.3), Kollision, Höhenverhalten.
- **Ablage der Metadaten im Textprojekt** (`data/tilesets/NNNN.json`, Stamm = Paketnummer), nicht im Repo. Wer kuratierte Vanilla-Metadaten im Repo will, darf dort höchstens **Indizes und Hashes** ablegen (Familie, Paket, Texturindex, SHA-1 der Texel, Rolle), keine Namen, keine Pixel: offene Frage 1.
- **(a)** als deterministisch erzeugtes Mini-Tileset (Flächen, Schachbrett, Kanten in 16 × 16, BGR555-Farben) unter `test-support`: Unit-Tests, kopflose Tests und das synthetische Projekt laufen damit ohne ROM. Kein „Inhalt“, keine Künstlerarbeit.
- **(c)** PDSMS-Bestand erreicht das Studio über PDSMS' eigenen OBJ-Export und unseren OBJ-Import (6.5); ein nativer `.pdsts`-Leser bleibt „später, wenn überhaupt“ (§6.3).

---

## 3. Malmodell

### 3.1 Befund: wie Vanilla-Geometrie aussieht (gemessen)

| Größe | Platin DE | HeartGold DE |
|---|---|---|
| NSBMD je Karte Median / p90 / max (Byte) | 20 224 / 42 108 / 60 556 | 23 248 / 48 964 / 61 420 |
| Grenze `MAP_MODEL_FILE_SIZE` 0xF000 = 61 440 | Abstand des größten: 884 Byte | **20 Byte** |
| Polygone je Karte Median / max (MDL0-Kopf) | 447 / 1 646 | 554 / 1 706 |
| Ecken je Karte Median / max | 1 532 / 4 757 | 1 669 / 4 854 |
| Materialien / Shapes je Karte (Median) | 10 / 10 | 21 / 17 |
| Primitive: Viereckstreifen / Dreiecksstreifen / Vierecke / Dreiecke | 50 196 / 11 036 / 6 414 / 1 852 | 60 885 / 24 913 / 8 628 / 3 153 |
| Polygone nach Ausrollen: Vierecke | 81 % | 77 % |
| … waagerecht / senkrecht / schräg | 32 % / 6 % / 62 % | 37 % / 10 % / 53 % |
| waagerechte Fläche mit allen Ecken auf dem Kachelraster | 66 % | 68 % |
| häufigste waagerechte Polygone (ganzzahlig) | 1×1 21 747, 2×2 7 038, 4×4 3 525, 3×3 2 486, 8×8 1 442 | 1×1 43 955, 8×8 3 327, 2×1 2 129, 2×2 2 116 |
| Ecken mit x und z auf ganzen Kacheln | 32 % | 30 % |
| Eckhöhen auf halben Kacheln | 39 % | 35 % |
| verschiedene Höhen waagerechter Flächen je Karte (Median / p90) | 5 / 12 | 8 / 16 |
| `pos_scale` (Median) | 64 | 64 |
| Knoten je Karte | 1, bei wenigen Karten bis 25 | immer 1 |

(Die Höhen- und Rasterzahlen ignorieren Knoten-Matrizen; auf Platin betrifft das wenige Karten mit mehr als einem Knoten.)

Und die Permissions:

| Größe | Platin DE | HeartGold DE |
|---|---|---|
| verschiedene `kind`-Werte gesamt / je Karte (Median, max) | 94 / 4, 12 | 65 / 4, 11 |
| Zellen mit `kind` 0 | 85 % | 75 % |
| blockierte Zellen je Karte (Median) | 52 % | 34 % |
| `collision`-Werte | nur `00` und `80` | `00`, `80`, dazu `06` (44 379), `04` (27 435), `0A` (13 424), `07` (2 538) |

**Folgerung.** Vanilla ist **zur Hälfte Raster, zur Hälfte frei**: der Boden liegt zu zwei Dritteln als achsenparallele Polygone auf dem Kachelraster, oft zu Rechtecken zusammengefasst; Bäume, Dächer, Felsen, Zäune und Böschungen sind freie Geometrie (schräge Polygone 53 bis 62 %). Ein Malmodell, das nur Bodenquadrate kennt, ergibt eine flache „PDSMS-Optik“; eines, das jede Kachel als freies Teilmodell behandelt, wird groß (Budget 0xF000, HeartGold liegt mit dem größten Vanilla-Modell 20 Byte darunter). Das HeartGold-Kollisionsbyte ist mehr als ein Blockiert-Bit (Werte 04, 06, 07, 0A): das Kollisions-Modell muss je Familie die Werte kennen (Gutachter B, offene Frage 5).

### 3.2 Optionen für das Raster

| Option | Vorteile | Nachteile |
|---|---|---|
| **M1 Zellraster, Höhe je Zelle** (wie PDSMS): Zelle = Kachel + Höhe in halben Kacheln, je Ebene | einfach, deckt sich mit Matrix-Höhen (halbe Kachel je Stufe, `ALTITUDE_STEP`) und mit dem Medianbefund (5 bis 8 Höhen je Karte, viele auf halben Kacheln), Kanten zwischen Höhen werden automatisch Wände; BDHC-Platten bleiben eben oder einfach geneigt | Hänge nur über eigene Rampen-Kacheln |
| **M2 Höhe je Ecke** (Höhenfeld 33 × 33) | Hügel ohne Sonderkacheln | Vierecke werden unplan (DS rendert sie, BDHC-Platten sind aber Ebenen: jede unplane Zelle muss in zwei Dreiecke und zwei Platten), Wände an Klippen unklar, Gen-4-Look hat kaum weiche Hügel |
| **M3 frei** (Polygone direkt) | alles möglich | das ist ein 3D-Modellierer; dafür gibt es Blender und den glTF-Import (6.5) |

**Empfehlung (recommend): M1 mit Teilmodellen.** Eine Zelle je Ebene trägt `tile` (Index ins Tileset, dazu Drehung 0/90/180/270 und Spiegeln) und `h` (Höhe in halben Kacheln, ganzzahlig; den Wertebereich legt 6.3a aus den Vanilla-Höhen und den BDHC-Grenzen von Gutachter B fest). Eine Kachel des Tilesets ist eines von zwei Dingen:

1. **Bodenkachel** (Material, welt-gebundene UV mit 16 Texel je Kachel): die Masse des Bodens; gleichartige Nachbarn fasst der Bäcker zu großen Rechtecken zusammen (wie Vanilla).
2. **Teilmodell** (glTF/OBJ-Stück mit Fußabdruck 1×1 bis 6×6 Zellen, Versatz, Drehung, eigenen Materialien aus demselben Paket): Bäume, Zäune, Rampen, Treppen, Felskanten, Sockel. Quelle ist der Import (6.5) oder das Herauslösen eines Vanilla-Stücks im Projekt (Offene Frage 3).

### 3.3 Treppen, Klippen, Wasser, Brücken, Gebäude-Sockel

| Ding | im Modell | gebacken |
|---|---|---|
| **Klippe / Höhenkante** | Nachbarzellen mit verschiedenem `h` | senkrechte Wandflächen mit der Wandtextur des Kantensatzes (Metadatum `wall` der oberen Kachel), Kollision der unteren Zelle an der Kante blockiert; BDHC: zwei ebene Platten |
| **Rampe / Hang** | Teilmodell oder Bodenkachel mit Höhenverhalten `ramp: N/O/S/W`, Steigung eine oder zwei halbe Kacheln je Zelle | geneigtes Viereck, BDHC geneigte Platte (Gutachter B) |
| **Treppe** | Teilmodell mit `ramp` und `kind` Treppe (Familienwert aus `tile_behaviors*.yaml`) | Geometrie des Stücks, BDHC als Rampe, Permission aus dem Metadatum |
| **Wasser** | Bodenkachel mit `terrain: water`, Standard-`kind` Wasser, `h` eine halbe Kachel unter dem Ufer, Ufer per Kantensatz | Wasserfläche + Ufer, Kollision frei, `kind` surfbar; Animation über die Gebietsdaten (HGSS `dynamic_texture_type`), nicht Teil des Malens |
| **Brücke** | zweite Ebene über dem Boden mit eigener Höhe, Kachel mit `kind` Brücke | zwei Flächen übereinander; BDHC mit zwei Platten an einer Stelle und Permission „Brücke“: ob und wie das Spiel das trägt, prüft Gutachter B (Offene Frage 4) |
| **Gebäude-Sockel** | Objekt-Ebene: Gebäude (`model_id`, Lage in Kacheln) wie heute; Fußabdruck aus dem Gebäudemodell (Hülle über apicula) | Boden bleibt Boden; Kollision unter dem Fußabdruck blockiert, Türzelle `kind` 0x69 mit Kollision 0x80 (`tiles::DOOR`, schon im Kernel) |

### 3.4 Smart Drawing

| Option | Vorteile | Nachteile |
|---|---|---|
| **S1 PDSMS-Satz** (Mitte, 4 Kanten, 4 Innen-, 4 Außenecken = 13 Kacheln je Satz) | bekannt aus der Community | nur zwei Gelände je Satz, keine Diagonalen, kein Mehrfarben-Übergang |
| **S2 Wang-Ecken-Sätze** (Tiled „corner“): jede Kachel trägt an ihren vier Ecken eine Geländefarbe; der Pinsel setzt Eckfarben und wählt die Kachel, deren Ecken passen | Standard (Tiled 1.5+), bildet S1 vollständig ab (zwei Farben: 16 Kombinationen, S1 sind 13 davon, dazu die zwei Diagonalen und „leer“), mehrere Gelände und Übergänge, Zufallswahl bei mehreren passenden Kacheln | Metadaten je Kachel nötig (vier Eckfarben) |
| **S3 Kanten-Sätze** (Tiled „edge“) | Wege, Zäune, Flüsse als Linien | eigener Pinsel; seltener |
| **S4 Blob/47-Autotiles** (RPG Maker) | sehr glatte Übergänge | 47 Kacheln je Gelände; Vanilla-Pakete haben solche Sätze nicht (Paket-Median 30 bis 38 Texturen insgesamt) |

**Empfehlung (recommend): S2 als Datenmodell, S1 als Bedienung.** Ein „Kantensatz“ im Studio ist ein Wang-Ecken-Satz; sein Editor zeigt die 13 PDSMS-Plätze plus die zwei Diagonalen als Raster zum Hineinziehen. S3 später. Höhe als eigenes „Gelände“ im Satz (eine Eckfarbe „oben“) macht den PDSMS-Trick „Satz hebt die Mitte um eine Stufe“ und den Klippenpinsel möglich.

**Metadaten je Kachel** (Tileset-JSON): `material` (Textur und Palette des Pakets; Teilmodell: Datei), `size` (Fußabdruck in Zellen), `rotate`/`flip` erlaubt, `terrain` (vier Eckfarben eines Satzes oder keine), `kind` und `collision` als Vorgabe (Zahl der Familie, Name aus `tile_behaviors*.yaml`), `height` (`flat`, `ramp: N|O|S|W` mit Stufen, `wall` = Wandmaterial für Höhenkanten), `layer_hint` (Boden, Überlagerung, Brücke), `alpha` (A3I5/A5I3, `color0`), `weight` (Zufallswahl), `name`.

### 3.5 2D-Draufsicht und 3D-Vorschau

- **Draufsicht: three.js-Orthokamera, kein eigenes Canvas.** Sie existiert schon (`SceneHost.setTopDown`, `camera.ts` `topDownFrame`, Design-System: „Draufsicht schaltet orthografisch zum präzisen Malen“). Picking (`picking.ts` `pickTile`, Ebenen je Höhe), Gelände-Overlay (`content.ts`, InstancedMesh, 42 % Deckkraft), Entwurfsrahmen (`draft.ts`, gestrichelt), Marker und Gebäude laufen unverändert in beiden Kameras. Ein 2D-Canvas wäre eine zweite Szene mit eigenem Picking, eigenen Markern und eigenen Overlays.
- **Live-Vorschau ohne Baking: ja, aus demselben Vernetzer.** Der Kernel erzeugt aus dem Weltformat ein Netz (Positionen, UV, Material je Teilnetz), das auch der Bäcker benutzt; die Szene zeichnet es direkt als `BufferGeometry` mit den dekodierten Texturen des Pakets (Dekoder vorhanden), ohne NSBMD und ohne apicula. Nur der Build schreibt NSBMD. Geschätzt: eine 32×32-Karte vernetzt im Kernel in wenigen Millisekunden; über die Brücke ist das Netz einer Karte 50 bis 200 KB (Float32). Ein Pinselstrich zeichnet sofort optimistisch (Kacheltextur auf der Zelle im Overlay), das Netz kommt nach dem Loslassen. Die Vorschau zeigt, was das Spiel zeigt, weil beide aus einer Quelle kommen; ein gated Test vergleicht Netz und apicula-Lesung des gebackenen NSBMD (Ecken, UV, Materialien).

---

## 4. Tiled als Vorbild

### 4.1 Befund

- **Tiled** (`mapeditor/tiled`, 13 027 Sterne, aktiv, Release v1.12.2 vom 2026-05-27): Programm und Plugins **GPL** (`COPYING`: `src/tiled`, `src/plugins`), `libtiled`, `tmxviewer`, `tmxrasterizer` **BSD-2-Clause**. Formate TMX (XML) und TMJ (JSON) sind öffentlich dokumentiert (`doc.mapeditor.org`, „JSON Map Format“). Modell: Karte (`orientation`, `width`, `height`, `tilewidth`, `tileheight`, `infinite`), Ebenen (`tilelayer` mit GIDs als CSV oder Base64 mit zlib/gzip/zstd, `objectgroup`, `imagelayer`, `group`), Tilesets (`firstgid`, Bild oder Bildsammlung, `tiles` mit `properties`, `wangsets` vom Typ `corner`/`edge`/`mixed` mit `wangid` aus acht Farbindizes), eigene Eigenschaften (`string`, `int`, `float`, `bool`, `color`, `file`, `object`, `class`) und eigene Typen; Drehen und Spiegeln in den oberen GID-Bits. Werkzeuge: Stempel (mit Zufall und Wang-Modus), Terrain-Pinsel (Wang), Füllen, Formfüllung, Radierer, Auswahl (Rechteck, Zauberstab, gleiche Kachel), Objekt-Werkzeuge, Automapping, JavaScript-Erweiterungen (`registerMapFormat`).
- **JS/TS-Bibliotheken mit Tiled-Modell** (npm, geprüft): `pixi-tiledmap` 2.11.0 (MIT, 2026-09), `@excaliburjs/plugin-tiled` 0.32.0 (BSD-2-Clause, 2025-12), Phaser (MIT) und melonJS (MIT) mit TMJ-Laden, `tiled-types` 1.3.0 (MIT, 2021), `@kayahr/tiled` 0.0.1 (MIT, 2024), `tmx-parser` 1.5.0 (MIT, 2017), `tilemap-editor` 0.7.8 (MIT, 2021, eigenständiger Mini-Editor), `gl-tiled` (MIT, 2020). **Alle sind Lader und Renderer für fertige Karten in 2D-Engines**, keine liefert Editor-Logik (Wang-Pinsel, Undo, Auswahl) für eine three.js-Szene. In Rust: `tiled` 0.16.0 (MIT, 2026-07) liest TMX/TMJ.
- **LDtk** (`deepnight/ldtk`, MIT, aktiv) ist die freie Alternative zu Tiled mit Auto-Layer-Regeln; dieselben Grenzen wie Tiled (2D), daher hier nur am Rand.

### 4.2 Abbildung der Begriffe auf Gen 4

| Tiled | Gen 4 / Studio | passt? |
|---|---|---|
| Karte 32 × 32, `tilewidth` 16 | Karte (`MAP_SIZE` 32, 16 Welteinheiten je Kachel) | ja; eine Tiled-Karte je Matrix (n × 32) ginge mit Tiled „Worlds“, die Karte bleibt aber 32 × 32 fest |
| Kachel-Ebene (GIDs) | Boden-, Überlagerungs-, Brücken-Ebene (Kachel-IDs) | ja |
| Höhe | — (Tiled kennt keine) | **nein**: nur als zweite Kachel-Ebene mit Zahlen-Kacheln oder Eigenschaft je Zelle, ohne 3D-Sicht |
| Gelände-/Kollisions-Ebene | Permissions `kind`, `collision` | ja als Kachel-Ebene mit einem „Verhaltens-Tileset“ (eine Kachel je Wert, Name aus `tile_behaviors*.yaml`) |
| Objekt-Ebene | Gebäude (`model_id`, fx32-Lage), Events (Figur, Warp, Auslöser, Schild) | teilweise: Gebäude ja (Punkt-Objekt mit Eigenschaften); Events liegen in Eventdateien je Header, nicht je Karte, und haben Scripts: im Studio besser, nicht in Tiled |
| Tileset (Bild + Kachelraster) | NSBTX-Paket + Metadaten | **schief**: Tiled will ein Bild mit festem Raster oder Einzelbilder; Vanilla-Texturen sind wiederholbare Materialien verschiedener Größe mit welt-gebundener UV (§2.1). Tiled müsste je 32×32-Textur vier Kacheln zeigen, die Lage in der Welt wählt der Nutzer dann von Hand |
| Wang-Sets | Kantensätze (§3.4) | ja, das ist das Vorbild |
| Drehen/Spiegeln (GID-Bits) | Drehung/Spiegeln der Zelle | ja |
| Teilmodelle, Rampen, Treppen, Wände | — | **nein**: 3D-Stücke mit Fußabdruck und Höhenverhalten hat Tiled nicht |

### 4.3 Optionen

| Option | Vorteile | Nachteile |
|---|---|---|
| **T1 Tiled als externer Editor** (TMJ-Import und -Export, eventuell eine Tiled-Erweiterung in JS) | null Malcode, ausgereifte Werkzeuge, GPL des Programms berührt uns nicht (wir lesen nur ein dokumentiertes Format) | keine Höhe in 3D, keine Vorschau im Spielstil (globale UV, Teilmodelle), kein ⌘Z im Store, kein MCP, kein Rezept, Kontextwechsel aus der Tiefe Bauen, Nutzer muss Tiled installieren; die Tileset-Abbildung ist schief (4.2); Rückweg TMJ → Weltformat verliert alles, was Tiled nicht kennt |
| **T2 Tileds Bedienung und Modell im Studio nachbauen** | alles in einer Szene (Höhe, 3D, Vorschau), Store-Undo, MCP, Rezepte, Design-System; Begriffe und Kürzel sind gelernt (Stempel B, Füllen G, Radierer E, Auswahl R, Terrain) | Aufwand L (§8); Wang-Füllen muss selbst geschrieben werden (aus der veröffentlichten Beschreibung, nicht aus Tileds GPL-Code `src/tiled`) |
| **T3 JS-Bibliothek mit Tiled-Modell** | – | es gibt keine passende (4.1); die Lader bringen nur Typen, die wir in einer Datei selbst schreiben |

**Empfehlung (recommend): T2, mit TMJ als Austausch in 6.5.** Das Studio übernimmt Tileds **Modell** (Ebenen, Tileset mit Eigenschaften, Wang-Ecken-Sätze, Objekt-Ebene) und Tileds **Werkzeuge** (Stempel mit Auswahl als Stempel, Rechteck, Füllen, Radierer, Pipette, Zufall, Terrain-Pinsel, Auswahl) und ergänzt, was Gen 4 braucht und Tiled nicht hat (Höhe mit Wänden, Rampen, Teilmodelle, 3D-Vorschau). Dazu kommt ein **TMJ-Export und -Import** als Brücke für Leute, die Tiled gewohnt sind (Boden-, Kollisions-, Objekt-Ebene; Höhe als Eigenschaft je Zelle in einer eigenen Ebene), nicht als Weltformat (§5.4). Antwort an den Nutzer: „wie bei Tiled“ ja, **im Studio**; Tiled selbst bleibt ein optionaler Weg hinein und hinaus.

---

## 5. Weltformat

### 5.1 Befund

Das Textprojekt hält eine Karte heute als `data/maps/NNNN.json` mit `source: "binary"` (einziger Wert; anders ist `textproject.map-source`), `permissions` als 32 Zeilen `kkcc`, `buildings` als Liste, `model`, `bdhc`, `bgs` als Teile (`hex`, `base_sha1` oder Beifile `NNNN.nsbmd`). Das Feld `source` ist genau für Phase 6 reserviert. Die Build-Schritte sind `textproject` (JSON → Member), `rotom` (Quelle → Member, schreibt Ausgaben selbst), `chatot`, `pack:*`, `rom`; ein Schritt nennt Eingaben, Ausgaben, Salz. `.gitattributes` setzt `* -text` und hält eine auskommentierte LFS-Zeile „für große Binärquellen (Phase 6)“ bereit. Die Gebietsdaten verweisen auf das Paket (`map_tileset`), die Matrix auf die Karte, die Karte auf Gebäudemodelle.

### 5.2 Skizze (recommend)

`data/maps/0123.json` mit neuer Quelle `"world"`:

```json
{
  "source": "world",
  "world": {
    "version": 1,
    "tileset": 6,
    "layers": [
      { "name": "Boden", "role": "ground",
        "tiles":   ["0012 0012 0013r1 …", "… 32 Zeilen …"],
        "heights": ["2 2 2 4 4 …", "… 32 Zeilen …"] },
      { "name": "Brücke", "role": "bridge", "tiles": ["…"], "heights": ["…"] }
    ],
    "pieces": [{ "tile": 40, "at": [12, 7], "rotate": 90 }],
    "freeform": [{ "file": "0123.weg.glb", "at": [0, 0, 0] }],
    "permissions": { "derive": true, "override": ["…32 Zeilen kkcc oder leer…"] }
  },
  "buildings": [{ "model_id": 7, "position": [196608, 0, 0], "rotation": [0, 0, 0], "scale": [4096, 4096, 4096] }],
  "bgs": { "hex": "34120000" }
}
```

- **`version`** eigenes Feld der Welt (Schema-Version der Art in `manifest.schemas` als `"maps.world": 1`), damit das Malen sein Format weiterentwickeln kann, ohne `schemaVersion` des ganzen Projekts zu heben; das Feld `world` ist für ältere Studios ein unbekannter Schlüssel (`deny_unknown_fields`): Migration der Projektversion einmal, wie bei 3d.4.
- **Zeilen wie die Permissions** (32 Zeilen, Zellen durch Leerzeichen): Kachel als vierstellige Hex-ID, Suffix `r1..r3` für Drehung, `f` für Spiegeln, `.` für leer; Höhen als Zahl in halben Kacheln. Git zeigt eine gemalte Zelle als geänderte Zeile.
- **`tileset`** ist die Nummer des Kartentextur-Pakets; die Metadaten liegen in `data/tilesets/0006.json` (Kacheln, Kantensätze, Teilmodelle als Beifiles `0006.<name>.glb`). Bei einer Karte, deren Header andere Gebietsdaten nennt, warnt die Prüfung (`world.tileset-mismatch`).
- **`permissions`**: abgeleitet aus den Kachel-Vorgaben, Höhenkanten und Gebäuden, mit Überschreibungen je Zelle (leer = abgeleitet), damit die Kollisions-Ebene von Hand bleibt, wo nötig.
- **Gebäude und `bgs`** wie heute.

### 5.3 Build-Graph und Textprojekt

- **Neuer Schritt `world`** vor `textproject`: Eingaben `data/maps/*.json` mit `source: "world"`, `data/tilesets/`, die Beifiles und die Kartentextur-Pakete (Texturgrößen und -namen); Ausgaben die Member `unpacked/maps/NNNN` dieser Karten. Salz: Compiler-Revision + Writer-Revision (A) + BDHC-Revision (B). Er läuft je Karte inkrementell (eine geänderte JSON → eine Karte). Der Schritt `textproject` behandelt Welt-Karten wie rotom die Scripts: der Member ist **Ausgabe**, nicht Quelle; ein von außen (DSPRE) geänderter Member einer Welt-Karte ist der Konflikt `world.member-changed` mit zwei Wegen: „als binäre Karte übernehmen“ (`source` wird `binary`, die Welt fällt weg) oder „neu backen“.
- **Store und Undo:** Der Store hält für eine Welt-Karte das Weltdokument (die JSON), nicht die Member-Bytes; ein Pinselstrich ist eine Store-Änderung (ein Undo-Schritt), der Member entsteht beim Speichern oder Bauen. Das braucht eine neue Asset-Art im Store (etwa `AssetKind::WorldMap`) neben `Map`; die Szene holt bei Welt-Karten das Netz statt des GLB.
- **Nur im Textprojekt.** Gemalte Karten brauchen eine Quelldatei; im DSPRE-Layout gibt es keine. Dort sagt das Werkzeug: „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“ Das folgt der Entscheidung aus 3d, neue Projekt-Daten nur noch im Textprojekt anzulegen.
- **„Karte als Quelle übernehmen“** (bestehende Karte → `source: "world"`): übernehmbar sind Permissions (als Überschreibung, 1:1), Gebäude (1:1), `bgs` (1:1); **Höhen** nur, soweit Gutachter B BDHC-Platten auf Zellen abbilden kann (Vorschlag: Höhe einer Zelle = Höhe der Platte über ihrer Mitte); **Geometrie** in zwei Formen zur Wahl: (1) **verbatim** als `freeform: [{ "nsbmd": "0123.nsbmd" }]`, das Raster beginnt leer und kommt obendrauf, oder (2) **als glTF** über apicula (verlustbehaftet: Materialien, Vertex-Farben, Streifen), danach Bearbeitung in Blender. Vanilla-NSBMD in ein Kachelraster zurückzuverwandeln ist bei 34 % freier Bodenfläche und 53 bis 62 % schrägen Polygonen nicht verlässlich: **nicht anbieten**.
- **Prüfstein für 6.3:** jede der 666 bzw. 676 Vanilla-Karten übernommen mit Form (1) und ohne Raster backt **byte-gleich** zum Original (Permissions, Gebäude, Modell, BDHC verbatim). Erst mit Raster entstehen neue Bytes.

### 5.4 TMJ als Weltformat?

| | eigenes JSON (5.2) | TMJ |
|---|---|---|
| Höhe, Rampen, Teilmodelle, Wände | eigene Felder | nur als `properties`, umständlich |
| Git-Diff | Zeilen je Kachelreihe | CSV in einer langen Zeile oder Base64 |
| Bezug auf NSBTX-Paket, Gebäude-IDs, fx32 | direkt | Umwege über Eigenschaften und ein Bild-Tileset, das es im Projekt nicht gibt |
| in Tiled öffnen | nein | ja, aber ohne 3D und ohne Spielstil |
| Schema-Pflege | unsere | fremd (Tiled ändert Felder, `class` seit 1.9, `mode` seit 1.12) |

**Empfehlung (recommend): eigenes JSON als Weltformat; TMJ nur Export/Import (6.5).**

### 5.5 Git LFS

Gemessen: Kartentextur-Pakete Median 11 bis 15 KB, größtes 76 KB; NSBMD höchstens 61 KB. Geschätzt: ein PNG-Tileset 10 bis 200 KB, ein Teilmodell als `.glb` 5 bis 100 KB, eine Freiform-Karte bis wenige 100 KB. **Empfehlung (recommend): kein LFS als Standard.** Die Größen liegen weit unter dem, wofür LFS sich lohnt; LFS kostet jeden Nutzer eine Einrichtung, bei GitHub Kontingente, und bricht Klone ohne LFS still (Zeigerdateien statt Bilder). Stattdessen: `.gitattributes` bekommt `*.png binary`, `*.glb binary` (Diff aus, kein Merge-Versuch), `.gltf` mit externen Puffern wird beim Import zu `.glb`. Die vorhandene auskommentierte LFS-Zeile bleibt als dokumentierte Option („über 1 MB je Datei“); eine Prüfung `git.large-binary` warnt ab 1 MB.

---

## 6. Import

### 6.1 OBJ und glTF → NSBMD

Was es braucht (der Writer selbst ist Gutachter A):

| Schritt | Grund und Grenze |
|---|---|
| Triangulation von n-Ecken, Vierecke behalten, wo sie plan und konvex sind | DS kennt nur Drei- und Vierecke; Vanilla ist zu 77 bis 81 % Viereck, Viereckstreifen sind das häufigste Primitiv (gemessen) |
| Streifen bilden (Viereck- und Dreiecksstreifen) | spart Ecken; nötig für das Budget |
| Ecken quantisieren: `pos_scale` wählen, sodass alle Ecken in ±8 (4.12 `s16`) passen; Vanilla nutzt 64 (Median), eine 512-Einheiten-Karte passt mit 64 | Genauigkeit dann 1/64 Einheit; `VTX_10` (gröber) und `VTX_XZ`/`VTX_DIFF` sparen Platz, wo es passt |
| Texturkoordinaten als 1.11.4: ±2 048 Texel; über eine Fläche läuft die UV bis 32 Texturgrößen weit (Vanilla max 32) | eine 32-Kachel-Fläche mit 16 Texel je Kachel sind 512 Texel, passt |
| ein Material je Textur und Palette (Namen müssen im Paket des Gebiets stehen; Vanilla: Pt 14, HG 7 Karten haben Fehlnamen, nur Hinweis) | Wiederholen-Bits im Material setzen (§2.1) |
| Budgets: Modell ≤ 0xF000 (`check_limits.yaml`), BDHC-Nutzlast ≤ 0x9000; Texturen müssen ins Paket passen (VRAM: Gutachter B) | HeartGold-Maximum liegt 20 Byte unter 0xF000: Ablehnen mit Zahl, nie still kürzen |
| Achsen: glTF Y oben, rechtshändig, Meter; OBJ ohne Einheit | Einheit wählbar (Vorgabe: 1 = eine Kachel), Vorschau vor dem Import wie heute bei `map_import::inspect` |

**Geschätzt, warum Zusammenfassen und Streifen Pflicht sind:** eine Ecke mit `TEXCOORD` + `VTX_16` kostet 3 Parameterwörter plus ½ Wort Befehle = 14 Byte. Eine 32×32-Karte mit einem einzelnen Viereck je Zelle sind 4 096 Ecken ≈ 57 KB, knapp unter 0xF000, ohne Wände, Teilmodelle und Gebäudesockel. Als Viereckstreifen je Reihe ≈ 30 KB, mit `VTX_XZ` auf gleicher Höhe ≈ 21 KB, zusammengefasste Rechtecke gleicher Bodenkachel (wie Vanilla 2×2 bis 8×8) deutlich darunter. Der Bäcker muss also Rechtecke zusammenfassen, bevor er Streifen bildet; die Budgetanzeige („Budget: ruhig, knapp, voll“, Design-System) zeigt den Stand beim Malen.

**Abhängigkeiten (Lizenz-Audit, alle ohne GPL):**

| Crate | Version | Lizenz | Zweck |
|---|---|---|---|
| `gltf` (+ `gltf-json`) | 1.4.1 | MIT OR Apache-2.0 | glTF/GLB lesen (und das Vorschau-GLB schreiben, falls gewünscht) |
| `tobj` | 4.0.5 | MIT | OBJ + MTL lesen (aktiv gepflegt, 2026-08); `obj` 0.10.2 (Apache-2.0) ist seit 2020 still |
| `earcutr` / `earcut` | 0.5.0 / 0.4.12 | ISC / MIT OR Apache-2.0 | n-Ecken triangulieren (oder eigene Ohrenschneidung, wenige Zeilen) |
| `meshopt` | 0.7.0 | MIT OR Apache-2.0 | optional: Streifen und Eckenreihenfolge (bindet C++ meshoptimizer, MIT) |
| `image`, `png` | 0.25 / 0.18 (schon im Kernel) | MIT OR Apache-2.0 | Texturen der Modelle |
| `tiled` | 0.16.0 | MIT | TMX/TMJ lesen (6.5); Schreiben über `serde_json` selbst |

### 6.2 PNG → NSBTX

Das Format bearbeitet Gutachter A; hier nur Lizenz und Crates. Gebraucht werden: Bild lesen (`image`, schon da), auf BGR555 runden, auf 4, 16 oder 256 Farben quantisieren (Vanilla zu 73 % 16 Farben, 19 bis 24 % 4 Farben, gemessen), Index 0 durchsichtig (`color0`), Größen 8 bis 1 024 als Zweierpotenz, wahlweise A3I5/A5I3 für Halbtransparenz.

| Crate | Lizenz | Urteil |
|---|---|---|
| `color_quant` 2.0.0 (NeuQuant) | MIT | erlaubt, klein, Standard des `image`-Ökosystems |
| `quantette` 0.6.0 | MIT OR Apache-2.0 | erlaubt, bessere Qualität (k-means, Wu) |
| `exoquant` 0.2.0 | MIT | erlaubt, seit 2016 still |
| **`imagequant` 4.4.1 (libimagequant)** | **GPL-3.0-or-later** | **verboten** |

**Empfehlung (recommend): `quantette`**, mit eigenem Schritt „auf BGR555 einrasten, dann quantisieren“, damit gleiche Farben nach der Rundung nicht doppelt Plätze belegen. Neue Texturen werden an das Paket des Gebiets **angehängt**, nie umsortiert (alle Karten des Gebiets verweisen per Name; Pakete teilen Namen, 258 bzw. 300). Dasselbe Muster „PNG → Member“ braucht später das Schreiben der Kampfsprites (Gutachten C von 3d).

### 6.3 PDSMS `.pdsmap` / `.pdsts`

Beide Formate sind nicht öffentlich beschrieben, das Repo hat keine Lizenz, die Tilesets sind Spielgrafik (§2.3). Messen statt lesen ginge (das Format ist ein eigenes Tag-Format mit eingebetteten OBJ-Stücken), wäre aber Reverse-Engineering eines unbeschriebenen Formats für einen Bestand, den PDSMS selbst als **OBJ mit Texturen** exportiert. **Empfehlung (recommend): kein nativer PDSMS-Import in Phase 6.** Der Weg ist „in PDSMS: Export as OBJ with Textures → im Studio: Modell importieren (OBJ)“; die Doku `docs/formats/` bekommt dafür einen Abschnitt, das Import-Gate prüft eine OBJ, wie PDSMS sie schreibt (Vierecke, MTL mit PNG, eine Kachel = eine Einheit). Die Roadmap-Zeile „Baking aus PDSMS-`.pdsts` und Ordnerimport über `.pdsmap`“ geht nach „später, nur wenn ein Nutzer es braucht“.

---

## 7. Oberfläche

### 7.1 Wo das Malen lebt (recommend)

- **Tiefe Bauen, Welt-Explorer, Werkzeug „Gelände malen“ (B)**, kein eigener Modus. Heute setzt das Werkzeug nur Kollision per Klick (`canvasClicks.ts`, `set_permission_impl`, Screen 7 im Audit „teilweise (Phase 6): nur Kollision an/aus, ohne Draufsicht, Palette, Muster“). In Phase 6 bekommt es: Tileset-Palette in der Hand (Kachelraster, Kantensätze, Suche), Ebenen-Liste, Unterwerkzeuge in der Werkzeugleiste (Pinsel, Rechteck, Füllen, Radierer, Pipette, Höhe +/−, Rampe, Kantensatz-Pinsel), Ziehen malt (Strich statt Klick). Bei einer binären Karte bleibt es beim Kollisionsmalen mit dem Hinweis „Boden malen: Karte als Quelle übernehmen“.
- **Rezept „Neue Karte malen“** (`new_painted_map`): Zelle der Matrix (frei oder neben der Matrix wie „Gebiet erweitern“), Name, Tileset (Vorgabe: das Paket der Gebietsdaten des Headers), Grundkachel, Grundhöhe. Plan zeigt die Zelle gestrichelt (`draft.ts`); Anwenden legt den Member an (**nicht rückgängig**, Plakette, wie „Gebiet anlegen“) und macht den Rest als einen Schritt, schaltet Draufsicht und Werkzeug ein. Der gesperrte nächste Schritt `adjust_terrain` „Gelände anpassen“ („Kommt mit Phase 6“, `recipes/types.rs`) wird frei und öffnet das Werkzeug.
- **Tüfteln:** dieselbe Szene mit Zahlen statt Namen (Kachel-IDs, Höhen in halben Kacheln, `kind` hex), dazu das Weltdokument im JSON-Editor (CodeMirror ist da).
- **Undo:** ein Strich (Pointer runter bis hoch), ein Füllen, ein Rechteck, eine Satz-Anwendung = **ein** Store-Schritt; Verlauf „Boden gemalt (37 Kacheln)“. Das Ableiten der Permissions geschieht beim Backen und ist kein eigener Schritt.

### 7.2 MCP

- `recipe_plan`/`recipe_apply` mit `new_painted_map {matrix?, x, y, name, tileset?, fill_tile?, height?, header?}` (unumkehrbar wie `create_area`: kopflos `--allow-irreversible`, sonst `needs_confirmation`).
- `world_get {map}` (lesend): Ebenen, Höhen, Permissions abgeleitet und überschrieben, Budget (Byte des gebackenen Modells, geschätzt).
- `world_edit {map, ops: [...]}` (schreibend, **ein** Undo-Schritt je Aufruf): `paint {layer, cells, tile}`, `rect {layer, from, to, tile}`, `fill {layer, at, tile}`, `height {cells, h}`, `ramp {cell, dir}`, `terrain {set, corners}`, `piece {tile, at, rotate}`, `permission {cells, kind, collision}`.
- `tileset_get {map_textures}` (Kacheln, Sätze, Vorgaben), `tileset_update {…}` (schreibend), `world_preview {map}` (Bild der Draufsicht als PNG-Inhaltsblock, Muster `project_preview_get`).

### 7.3 Design-System

Vorhanden und zu nutzen: Draufsicht-Kamera, Werkzeug-Taste 40 × 40 mit Kürzel, Statusleiste mit Kürzeln, Rezept-Taste, Budget-Komponente, Plakette „Nicht rückgängig“ (Prinzip 8 „Reversibel sichtbar“), Tokens `szene/gelaende/*`, `szene/hoehe/0..6` (cividis, bisher ungenutzt), `gelaendeMuster` (Index je Muster in `tokens.ts`). Zu bauen: **Muster im Gelände-Overlay** (Shader statt `setColorAt`, wie Spezifikation 3c §8.1 es für Phase 6 vorsieht), **Höhen-Overlay** (sieben Stufen, eine Stufe = halbe Kachel, Isolinien bei Stufenwechsel), **Tileset-Palette**, **Ebenen-Liste**, **Kantensatz-Editor**. Die drei letzten stehen nicht im Doppeldeck; sie brauchen Frames in der `.fig` (Screen „Gelände malen: Route 201“ erweitern) und Nachträge im README.

### 7.4 Was von three.js und vom Picking wiederverwendbar ist

| Teil | Wiederverwendung |
|---|---|
| `SceneHost`, `setTopDown`, `camera.ts` | unverändert |
| `picking.ts` (`pickTile`, Ebenen je Höhe) | für Welt-Karten mit Höhe je Zelle: Strahl gegen das Vorschaunetz oder gegen ein Höhenraster je Zelle statt gegen eine Ebene je Matrix-Höhe; Schnittstelle bleibt |
| `content.ts` Overlay (InstancedMesh je Zelle) | Grundlage für Höhen-Overlay und Pinsel-Hover; Muster per Shader |
| `draft.ts` (gestrichelte Rahmen) | Pinsel-Fußabdruck, Rechteck, Plan des Rezepts |
| `markers.ts`, `buildingLayer.ts`, `buildingDrag.ts`, `eventDrag.ts` | Objekt-Ebene unverändert |
| `canvasClicks.ts` | um Striche (Pointer-Bewegung) erweitern; heute nur Klick |
| `thumbnailRenderer.ts` | Kartenbilder gemalter Karten aus dem Netz statt aus dem GLB |
| `sprites.ts` (NSBTX-Texturen in three.js) | Muster für die Paket-Texturen der Vorschau |

---

## 8. Schnitt 6.3 / 6.4 / 6.5

### 8.1 Bewertung des Vorschlags

Vorschlag: 6.3 Weltformat (braucht 6.1 Writer, 6.2 BDHC) → 6.4 Malen (braucht 6.3) ∥ 6.5 Import (braucht 6.1, 6.3). Zwei Schwächen:

1. **6.3 wartet unnötig auf 6.1 und 6.2.** Schema, Tileset-Ableitung, Vernetzer und Vorschau brauchen keinen Writer; nur das Backen und das Emulator-Gate brauchen ihn. Wer 6.3 erst nach 6.1 und 6.2 beginnt, verschenkt die längste Kette.
2. **6.5 hängt nur zum Teil an 6.3.** PNG → NSBTX braucht nur den NSBTX-Writer, OBJ/glTF → Modell-Teil einer **binären** Karte nur den NSBMD-Writer und `map_import` (das heute schon fertige Teile einsetzt). Nur „Freiform im Weltformat“ und TMJ brauchen 6.3.

### 8.2 Empfohlener Schnitt (recommend)

| Teilphase | Inhalt | Größe (AP) | braucht |
|---|---|---|---|
| **6.3a Weltformat und Vorschau** | `source: "world"` mit Schema und Migration, `data/tilesets/` mit Ableitung aus dem Paket (Bodenkacheln, Vorgaben aus Vanilla-Permissions), synthetisches Test-Tileset, Store-Art `WorldMap` mit Undo, Vernetzer (Zusammenfassen, Wände, Rampen, Teilmodelle), Netz-DTO und Anzeige in der Szene, Budget-Schätzung | L (6 AP: Schema+Migration, Tileset-Ableitung, Store, Vernetzer, DTO+Szene, Prüfungen) | Phase 3d/4 (Textprojekt); **parallel zu 6.1/6.2** |
| **6.3b Backen und Build** | Compiler Welt → Map-`.bin` (Netz → NSBMD über 6.1, Höhen → BDHC über 6.2, Permissions ableiten), Schritt `world` im Build-Graph, Konflikt `world.member-changed`, „Karte als Quelle übernehmen“ (Form 1 verbatim, Form 2 glTF erst mit 6.5b), Budgets als Prüfungen, gated Tests | M–L (4 AP) | 6.1, 6.2, 6.3a |
| **6.4a Malen: Werkzeuge und Rezept** | Pinsel, Rechteck, Füllen, Radierer, Pipette, Höhe, Rampe, Ebenen, Palette, Striche als ein Undo-Schritt, Höhen-Overlay, Rezept `new_painted_map` (+ `adjust_terrain` frei), MCP `world_get`/`world_edit`/`tileset_get` | L (5 AP) | 6.3a (Emulator-Teil des Gates: 6.3b) |
| **6.4b Smart Drawing und Muster** | Wang-Ecken-Sätze im Tileset, Kantensatz-Editor (13 + 2 Plätze), Terrain-Pinsel, Klippenpinsel, Zufall, Gelände-Muster-Shader, `.fig`-Frames | M (3 AP) | 6.4a |
| **6.5a PNG und Modelle** | PNG → NSBTX (Quantisierung, Anhängen ans Paket), OBJ/glTF → Modell-Teil einer Karte über `map_import` (Vorschau, Budgets, Ablehnen mit Grund), Doku „aus PDSMS über OBJ“ | M (3 AP) | 6.1 (nicht 6.3) |
| **6.5b Freiform und Austausch** | glTF als `freeform` und als Teilmodell im Tileset, Form 2 von „übernehmen“, TMJ-Export/-Import | M (3 AP) | 6.3b, 6.5a |

**Reihenfolge:** (6.1 ∥ 6.2 ∥ 6.3a) → (6.3b ∥ 6.5a) → (6.4a ∥ 6.5b) → 6.4b. 6.4a darf schon nach 6.3a beginnen (alles außer dem Emulator-Teil des Gates). Zusammen 24 AP; ohne die Teilung wäre 6.3 mit 10 AP die größte und am spätesten beginnende Teilphase.

**Berührungen:** 6.3a/6.4a teilen `world/scene/*` und Store; 6.5a berührt `map_import.rs`, Import-Dialog und `g4-formats` (Writer-Seite, mit A abzustimmen); Anhängepunkte wie immer (`commands/dispatch.rs`, `mcp/tools.rs`, `docs/mcp.md`, `api.ts`, `ROADMAP.md`, README, `.fig`).

### 8.3 Gates (messbar)

- **6.3a:** Für jedes Kartentextur-Paket von Platin DE (74) und HeartGold DE (106) leitet der Kernel ein Tileset ab, dessen Bodenkacheln die Zahl der Texturen des Pakets treffen (3 130 bzw. 3 659 gesamt); eine 32×32-Karte aus einer Bodenkachel vernetzt zu ≤ 4 Polygonen (zusammengefasst), eine Schachbrett-Karte aus zwei Kacheln zu ≤ 1 024; Vernetzen einer vollen Karte ≤ 20 ms im Release-Build, Netz über die Brücke ≤ 50 ms im Debug-Build; Export idempotent, Round-Trip der JSON byte-gleich; ⌘Z nimmt einen Strich als Ganzes zurück.
- **6.3b:** Alle 666 bzw. 676 Vanilla-Karten übernommen (Form 1) backen **byte-gleich**; eine gemalte 32×32-Karte (Boden, zwei Höhen mit Wand, eine Rampe, Wasser) backt mit NSBMD ≤ 0xF000 und BDHC ≤ 0x9000 + 16, apicula liest das NSBMD, das Netz stimmt mit der apicula-Lesung überein (Ecken, UV, Materialien); `vanilla_checks` 0 Fehler; `g4emu`: die Figur läuft über die Karte, wird an einer abgeleiteten blockierten Zelle gestoppt, steigt über die Rampe (Position und Höhe gelesen) – das ist das Roadmap-Gate.
- **6.4a:** Über die Test-Umgebung entsteht mit „Neue Karte malen“ eine Karte neben Zweiblattdorf bzw. Neuborkia, wird mit Pinsel, Füllen, Höhe und Rampe gestaltet und gebaut, in unter fünf Minuten; jeder Strich ein Verlaufseintrag; dasselbe über MCP (`recipe_apply` + `world_edit`), Szenarien `pt-gemalt.yaml`, `hg-gemalt.yaml`; Blick Hell, Dunkel, Kompakt.
- **6.4b:** Ein Kantensatz aus 13 Kacheln umrandet eine gemalte Fläche ohne falsche Ecke (gated Test über ein festes Muster, Erwartung als Kachel-Raster); Muster im Overlay sichtbar (Blick); `.fig` zeigt Palette, Ebenen, Kantensatz-Editor.
- **6.5a:** Ein 64 × 64-PNG mit 40 Farben wird 16-Farben-NSBTX, dekodiert zurück mit Farbfehler ≤ 1 BGR555-Stufe je Kanal für Farben, die die Palette trifft; eine Würfel-OBJ und eine Ebene-glTF (im Test erzeugt, keine fremden Assets) werden Modell-Teil einer Karte, apicula liest es, Ablehnung über 0xF000 mit Zahl; eine OBJ im PDSMS-Stil (Vierecke + MTL) wird importiert.
- **6.5b:** glTF-Freiform in einer Welt-Karte backt und wird in `g4emu` gesehen (Kamera-Bild ungleich ohne); TMJ-Export einer gemalten Karte, Import ohne Änderung ergibt dieselbe JSON (byte-gleich); Form 2 von „übernehmen“ für eine Vanilla-Karte backt mit apicula lesbar (nicht byte-gleich, Abweichung in Polygonzahl dokumentiert).

---

## 9. Risiken

1. **Budget 0xF000** (HeartGold-Maximum 20 Byte darunter): eine gemalte Karte mit Teilmodellen und Wänden reißt es leicht; ohne Zusammenfassen und Streifen schon die nackte Bodenkarte fast (§6.1, geschätzt). Abhilfe: Budget live beim Malen, Ablehnen beim Backen, nie kürzen.
2. **Writer- und BDHC-Qualität** (A, B): 6.3b und das Emulator-Gate stehen und fallen damit; 6.3a und 6.4a laufen trotzdem.
3. **Gemeinsame Gebietspakete:** neue Texturen ändern das Paket aller Karten des Gebiets (VRAM, Größe; Grenzen bei B). Nur anhängen, nie umordnen; Prüfung, ob das Paket nach dem Anhängen noch passt.
4. **Vanilla-Tilesets ohne Bedeutung:** Wer welche Textur als Wegkante nutzt, steht nirgends; die Ableitung aus den Vanilla-Permissions (§2.4) ist ungemessen. Ohne gute Vorgaben ist das Malen mit Vanilla-Texturen mühsam.
5. **Umfangsdruck „wie Tiled“:** Tiled hat zehn Jahre Werkzeuge; 6.4 muss bei den genannten Werkzeugen bleiben (Automapping, Kanten-Sätze, mehrere Karten gleichzeitig: später).
6. **Lizenz:** Wang-Füllen nicht aus Tileds GPL-Teil (`src/tiled`) ableiten; PDSMS nicht lesen (keine Lizenz); `imagequant` nicht einbinden.
7. **Store-Modell:** eine Asset-Art, deren Member abgeleitet ist, ist neu (bisher nur Scripts über rotom); Synchronisierung mit DSPRE und `member-newer` braucht eine klare Regel (§5.3).

---

## 10. Empfehlungen (Zusammenfassung)

1. **Tileset aus dem Kartentextur-Paket des Gebiets zur Laufzeit** plus Projekt-Metadaten in `data/tilesets/NNNN.json`; Bodenkachel = wiederholbares Material mit 16 Texel je Kachel und welt-gebundener UV (gemessen: so ist Vanilla gebaut); synthetisches Test-Tileset aus Code; PDSMS-Tilesets nicht (keine Lizenz, Spielgrafik).
2. **Malmodell M1:** Zellraster je Ebene mit Höhe je Zelle in halben Kacheln, automatische Wände an Höhenkanten, Rampen und Treppen als Kacheln mit Höhenverhalten, Teilmodelle mit Fußabdruck; Smart Drawing als Wang-Ecken-Sätze mit PDSMS-artigem 13+2-Editor.
3. **Tiled: Modell und Werkzeuge im Studio nachbauen (T2)**, TMJ nur als Austausch in 6.5b; Tiled als externer Editor allein reicht nicht (keine Höhe, kein 3D, kein Undo, kein MCP), eine JS-Bibliothek mit Editor-Logik gibt es nicht.
4. **Weltformat:** eigenes JSON in `data/maps/NNNN.json` mit `source: "world"` (das reservierte Feld), Zeilen wie die Permissions, eigene `version`, neuer Build-Schritt `world` mit dem Member als Ausgabe, nur im Textprojekt; „übernehmen“ mit Geometrie verbatim (byte-gleich als Gate) oder als glTF; kein LFS als Standard.
5. **Import:** OBJ (`tobj`) und glTF (`gltf`) → NSBMD mit Triangulation, Streifen, `pos_scale`-Wahl, Budget; PNG → NSBTX mit `quantette` (nicht `imagequant`, GPL); PDSMS nur über PDSMS' eigenen OBJ-Export.
6. **Oberfläche:** Werkzeug „Gelände malen“ in Bauen erweitern (Draufsicht und Picking vorhanden), Rezept `new_painted_map`, ein Strich = ein Undo-Schritt, MCP `world_get`/`world_edit`/`tileset_get`, Höhen-Overlay und Muster-Shader nach Doppeldeck.
7. **Schnitt:** (6.1 ∥ 6.2 ∥ 6.3a) → (6.3b ∥ 6.5a) → (6.4a ∥ 6.5b) → 6.4b, 24 AP; der Vorschlag der Hauptsitzung bleibt in der Abhängigkeit richtig, teilt aber 6.3 und 6.5, damit nichts auf die Writer wartet, was sie nicht braucht.

---

## 11. Offene Fragen

1. **Kuratierte Vanilla-Tileset-Metadaten im Repo** (nur Familie, Paket, Texturindex, SHA-1 der Texel, Rolle; keine Namen, keine Pixel): erlaubt im Sinne von „Nichts aus den ROMs ins Repo“? Vorschlag: ja für zwei Gate-Pakete (Platin Route, HeartGold Route), sonst nur im Projekt.
2. **Ableitung der Vorgaben je Textur aus Vanilla-Permissions** (Textur → häufigstes `kind`): Plan-Task 0 von 6.3a misst, wie eindeutig das ist.
3. **Vanilla-Teilmodelle herauslösen** (ein Baum aus einer Vanilla-Karte als Teilmodell im Projekt): über apicula-GLB und Auswahl im Studio, oder nur über Blender? Vorschlag: später; in 6.5b nur importierte Stücke.
4. **Brücken und mehrere Platten an einer Stelle** (BDHC, Permission „Brücke“): trägt das Spiel zwei Ebenen in einer gemalten Karte? Gutachter B.
5. **HGSS-Kollisionswerte 04, 06, 07, 0A** (gemessen, 87 776 Zellen): Bedeutung je Wert für die Kollisions-Ebene und die Ableitung; Gutachter B oder pokeheartgold.
6. **Höhe je Ecke** (M2) für Hügel: nach 6.4 neu bewerten, wenn Nutzer es vermissen.
7. **Malen über Kartengrenzen:** der Pinsel sieht die Nachbarkarten der Matrix (Kantensätze an der Naht); die Daten bleiben je Karte. Reicht das, oder braucht es ein Matrix-weites Weltdokument? Vorschlag: je Karte, Nachbarn nur lesend.
8. **Wasser-Animation** (HGSS `dynamic_texture_type`, Platin Animationsdateien des Gebiets): bleibt außerhalb des Malens; ob eine gemalte Wasserkachel ohne Animation im Spiel stört, zeigt das 6.3b-Gate.
9. **Tiled-Erweiterung** (JS, `registerMapFormat`), die direkt unser Weltformat schreibt: lohnt sie neben dem TMJ-Austausch? Vorschlag: nein in Phase 6.

## 12. Quellen und Lizenzen

| Quelle | Lizenz | genutzt für |
|---|---|---|
| eigene Messungen (`gutachten-c-*.py`) an `Roms/_testkopien/6-gutachten-c-{pt,hg}` | – (Skripte MIT wie das Repo) | alle gemessenen Zahlen |
| apicula (`src/nitro/*`) | 0BSD | Layout von TEX0, MDL0, Wörterbüchern (wie schon in `docs/formats/`) |
| GBATEK (GX-Befehle, Festkommaformate) | öffentliche Beschreibung | Display-Listen ausführen |
| pret/pokeplatinum, pokeheartgold (über `check_limits.yaml`) | keine Lizenz im Repo; nur Tatsachen | Budget 0xF000, BDHC 0x9000 |
| Tiled (`github.com/mapeditor/tiled`, `doc.mapeditor.org`) | Programm GPL, libtiled BSD-2-Clause; Doku nur als Tatsachenquelle | Modell, TMJ-Felder, Werkzeuge |
| PDSMS (`github.com/Trifindo/Pokemon-DS-Map-Studio`): README, Dateiliste, Release-Daten, eine `.pdsts` als Bytes gemessen | **keine Lizenz** (alle Rechte vorbehalten); kein Code gelesen | Lizenzlage, Exportweg, Format-Lage |
| DS Pokémon Hacking, Seiten „Tiles and Tilesets“, „Models“ (`ds-pokemon-hacking.github.io`) | keine Lizenzangabe; nur Tatsachen | PDSMS-Bedienung (Kacheln, Smart Drawing, Ebenen, Höhen, Export) |
| crates.io, npm-Registry, GitHub-API | – | Versionen, Lizenzen, Pflegestand der Abhängigkeiten |
| DSPRE | AGPL-3.0 | **nicht geöffnet** |
