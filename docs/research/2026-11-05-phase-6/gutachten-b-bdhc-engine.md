# Gutachten B: BDHC (Laufhöhen), Kollisionsableitung und Engine-Grenzen (Phase 6)

Stand: 05.11.2026, gelesen auf `main` (`91ba0615`) im Arbeitsbaum `/Users/POLTEP/Projekte/lab/editor/gen4-studio`. Am Repo-Code ist nichts geändert; neu sind nur dieses Gutachten und die drei Werkzeuge `gutachten-b-*.py` daneben. Grundlage sind:

- im Repo `crates/g4-formats/src/map.rs`, `docs/formats/{map,matrix,nsbtx}.md`, `crates/g4-kernel/src/map_import.rs` (`check_bdhc`), `crates/g4-kernel/metadata/check_limits.yaml`, `crates/g4-kernel/src/checks/world.rs`, `docs/research/2026-10-23-3b4/task0-3b4.md` §3, `docs/agent-gate.md` (Emulator, Szenario, Teile importieren);
- in der App `apps/studio/src/world/{layout,markers}.ts`, `world/scene/{picking,eventDrag,content}.ts`;
- `design-system/README.md` (Szenenfarben „Gelände“ und „Höhen“, Baustein „Budget“);
- pret/pokeplatinum `80c83a6f` (04.11.2026) und pret/pokeheartgold `e1d5a460` (01.11.2026), flach geklont in den Scratchpad: `src/overlay005/bdhc.c`, `include/overlay005/bdhc.h`, `include/constants/field/bdhc.h`, `src/terrain_collision_manager.c`, `src/map_object_move.c`, `src/overlay005/{land_data,loaded_map_buffers,map_prop,area_data,fieldmap}.c`, `src/map_matrix.c`, `src/map_tile_behavior.c`, `include/constants/field/{map,map_tile_behaviors}.h`, `src/easy3d.c`, `src/g3d_pipeline.c`, `include/global/assert.h`, `src/error_handling.c`; in pokeheartgold `include/constants/metatile_behavior.h`, `asm/unk_02054648.s`, `asm/unk_0205CB48.s`, `src/error_handling.c`;
- DSPRE (lokaler Checkout `e751187b`, AGPL-3.0), **nur Leser**: `DS_Map/ROMFiles/MapFile.cs` (Konstruktor und `Import…`), die Import-/Export-Knöpfe der BDHC in `DS_Map/Editors/MapEditor.cs` (Zeilen 2138 bis 2171) und die Patch-Tabelle `BDHCAMPatchData` in `DS_Map/Resources/ROMToolboxDB/ToolboxDB.cs`. Die Writer- und Baking-Dateien (`BdhcBuild`, `TileBake`, `NsbmdWriter`, `NsbtxWriter`, `TileGrid`, `TilePainter`) habe ich nicht geöffnet.

Gutachten C (`gutachten-c-malen-import.md`) habe ich nach meinen Messungen gelesen, soweit es Höhen und BDHC betrifft; der Abgleich steht in §2.10. Gutachten A habe ich nicht gelesen.

**Datenbasis.** Frische Kopien der drei DSPRE-Ordner unter `Roms/_testkopien/6-gutachten-b-{pt,hg,hgus}` (Platin DE 666 Karten, HeartGold DE und US je 676). Gemessen mit `gutachten-b-messung.py` (alle Karten, Matrizen, Gebiete, Kartentexturen), dazu kleine Abfragen im Scratchpad auf denselben Funktionen. Die BDHC-Daten von HeartGold DE und US sind in allen Zahlen gleich; wo nur „HG“ steht, gilt es für beide.

**Gemessen vs. geschätzt.** „Gemessen“ heißt: an den Vanilla-Dateien gezählt oder im Emulator beobachtet. „Belegt (pret)“ heißt: im Decomp-Code gelesen, nicht im Spiel beobachtet. „Geschätzt“ ist markiert.

**Last.** Zwei andere Gutachter liefen parallel (Load 3 bis 8). Zeiten spielen hier keine Rolle.

**Kurzurteil.**

- **Die BDHC ist einfach, und die Engine-Regeln sind hart.** Eine Platte ist ein achsparalleles Rechteck (zwei Punkte) mit einer Ebene (Normale, Konstante). Das Spiel sucht den Streifen per z, prüft die Rechtecke des Streifens und nimmt von höchstens 10 Treffern die Höhe, die der aktuellen am nächsten liegt. **Ein Schritt mit 20 Einheiten Höhenunterschied oder mehr wird verweigert**, unabhängig von der Permission (pret, im Emulator bestätigt). 16 Einheiten (eine Kachel) gehen ohne Treppe.
- **Die Emulator-Probe ist gelungen**, auf Platin DE, HeartGold DE und HeartGold US gleich: eine selbst erzeugte BDHC (340 B) und Permissions, über `import_map_parts` der Brücke in die Zimmerkarte gesetzt, gebaut und begangen. Die Figur steigt die Rampe (RAM-Höhe 3,989 in der Rampenmitte, genau der vorausberechnete Wert, 16 auf dem Plateau), die Stufe 16 geht, die Stufen 20 und 24 blockieren, `0x80` blockiert.
- **Ein BDHC aus einem Kachelraster passt immer in 0x9000.** Mit Rechtecken auf Kachelgrenzen und halboffenen Streifen ist die Nutzlast höchstens `23 304 + 12 × Normalen` Byte, also selbst mit 1 024 verschiedenen Neigungen 35 592 B < 36 864 B. Typische gemalte Karten liegen bei 0,1 % bis 2 % des Puffers, ein Schachbrett bei 52 %.
- **Die Matrix-Höhe ist nur Kulisse.** Das Spiel zeichnet das Modell um Höhe × 8 Einheiten versetzt, die BDHC aber nicht. In allen drei ROMs hat **keine** Zelle mit Höhe > 0 einen Header. Begehbare Karten tragen ihre Höhe selbst, im Modell und in der BDHC (57 % bzw. 61 % der begehbaren Kacheln liegen nicht auf 0).
- **Das Studio zeigt Höhen heute falsch.** Picking, Event-Drag, Marker und Gelände-Overlay liegen auf der Ebene der Matrix-Höhe (`layout.ts` `cellY`), nicht auf der GLB-Oberfläche und nicht auf der BDHC. Für begehbare Zellen ist das immer die Ebene 0.

**Empfehlung (recommend):** Teilphase 6.2 erzeugt die BDHC aus dem gemalten Höhenraster (eine Ebene je Kachel: flach oder Rampe 8/16 je Kachel entlang einer Achse, Höhe in Stufen zu 8 Einheiten), mit Rechteck-Zerlegung und halboffenen Streifen. Das Raster ist die eine Quelle für BDHC, Kollision und (bei Gutachter A) das Modell. Vanilla-BDHC bleiben unverändert erhalten; das Studio liest sie aber und benutzt die Höhensuche des Spiels für Overlay, Marker und Picking. Die Kollision leitet ein kleiner Regelsatz aus Gelände-Muster und Höhenkanten ab (§2.7). Gate in g4emu auf Platin DE und HeartGold DE (§5).

---

## 1. Anlass

Phase 6 soll Karten malen: Kacheln mit Höhen → gebackenes NSBMD, BDHC und Kollision. Heute behandelt das Studio die BDHC als undurchsichtige Bytes (`map.rs`, `docs/formats/map.md` „NSBMD und BDHC“). Beim Import prüft `check_bdhc` nur die Magic, den 16-Byte-Kopf, Länge = 16 + Nutzlast und Nutzlast ≤ `bdhc.max_bytes` 0x9000. Das Gutachten klärt, was eine selbst erzeugte BDHC erfüllen muss, wie das Spiel sie mit den Permissions zusammen auswertet, welche Grenzen je Karte gelten und was 6.2 liefern soll.

## 2. Befund

### 2.1 BDHC-Format (belegt (pret), bestätigt durch Messung und Probe)

Alle Werte Little Endian, fx32 = 20.12-Festkomma (4096 = 1,0). Eine Kachel ist 16 Welteinheiten; die Koordinaten der BDHC zählen ab der **Kartenmitte** (x, z von −256 bis +256), y ist die Höhe.

| Abschnitt | Eintrag | Größe | Inhalt |
|---|---|---|---|
| Kopf | | 16 B | `"BDHC"`, dann sechs `u16`: Punkte, Normalen, Konstanten, Platten, Streifen, Zugriffsliste |
| Punkte | `BDHCPoint` | 8 B | `fx32 x, fx32 z` |
| Normalen | `VecFx32` | 12 B | `fx32 x, y, z` (Vanilla normiert, z. B. (0, 1, 0), (0,707, 0,707, 0), (0,447, 0,894, 0)) |
| Konstanten | `fx32` | 4 B | d der Ebene n·p + d = 0 |
| Platten | `BDHCPlate` | 8 B | `u16` erster Punkt, zweiter Punkt, Normale, Konstante; die zwei Punkte sind gegenüberliegende Ecken eines achsparallelen Rechtecks |
| Streifen | `BDHCStrip` | 8 B | `fx32 scanline` (z), `u16` Anzahl, `u16` Start in der Zugriffsliste; aufsteigend nach `scanline` |
| Zugriffsliste | `u16` | 2 B | Plattennummern je Streifen, hintereinander |

Nutzlast = 8 P + 12 N + 4 C + 8 Pl + 8 S + 2 A (wie `check_bdhc` und task0-3b4 §3).

**Höhensuche** (`CalculateObjectHeight`, `bdhc.c:152`): Der Streifen für z ist der erste, dessen `scanline` größer als z ist, sonst der letzte. (pret sucht binär; auf sortierten Streifen ist das Ergebnis für 1 bis 33 Streifen identisch mit dieser Regel, gemessen.) Für jede Platte des Streifens prüft das Spiel, ob (x, z) im Rechteck liegt, **Rand eingeschlossen**. Für jeden Treffer rechnet es y = −(nₓ·x + n_z·z + d) / n_y, höchstens 10 Treffer. Bei mehreren gewinnt der, der der aktuellen Höhe der Figur am nächsten liegt. So gehen Brücken: zwei Platten übereinander, die Figur bleibt auf ihrer Ebene. Findet das Spiel keine Platte, meldet es „keine Höhe“ (`CALCULATED_HEIGHT_SOURCE_NONE`).

**Prüfung durch das Spiel** (belegt (pret)):

| Abweichung | Was passiert |
|---|---|
| Magic falsch | nichts: `BDHC_LoadHeader` liest die 4 Bytes, prüft sie aber nicht |
| Nutzlast > 0x9000 | `GF_ASSERT(offset <= BDHC_BUFFER_SIZE)` (`bdhc.c:282`). Im Spiel ist der Assert aktiv (`meson.build` `-DPM_KEEP_ASSERTS`), aber `ErrorHandling_AssertFail` setzt nur zurück, wenn Drahtlos-Kommunikation läuft. Allein im Spiel läuft es weiter, und die Tabellen werden über den Heap-Puffer hinaus gelesen. **Speicher wird überschrieben, ohne Fehlermeldung.** HGSS: `GF_AssertFail` ebenso (`src/error_handling.c:8`). |
| Länge ≠ 16 + Nutzlast | nichts: das Spiel liest nach den Zählern; die Länge aus dem Kartenkopf wird nicht ausgewertet (`dummyE4` in `BDHC_LazyLoad`). Zu wenig Bytes heißt, es liest in die nächsten Daten der NARC. |
| Indizes außerhalb | ungeprüft, liest daneben (Vanilla: 0 Treffer, gemessen) |
| `n_y` = 0 | Division durch 0 im Teiler der Hardware (Vanilla: 1 Platte in HG, gemessen) |
| Streifen unsortiert | die Binärsuche findet falsche Streifen, Höhen fehlen (Vanilla: 0) |
| keine Platte unter der Figur | die Figur behält ihre Höhe (`MapObject_RecalculatePositionHeight` gibt FALSE). Für die Schrittprüfung gilt die Zielhöhe 0 (+ Versatz): steht die Figur 20 oder höher, ist der Schritt blockiert |

Folge: Die Prüfung des Kernels ist strenger als das Spiel (gut so). Es fehlen **Indizes in Grenzen, Streifen aufsteigend und Streifenbereich in der Zugriffsliste**. Diese drei haben in allen 2 018 Vanilla-Karten 0 Treffer (gemessen) und können Fehler werden. `n_y` = 0 nur als Warnung (1 Vanilla-Treffer).

### 2.2 Vanilla-Verteilungen (gemessen, `gutachten-b-messung.py`)

Alle 666 bzw. 676 Karten haben eine BDHC mit Magic, mindestens einer Platte und Länge = 16 + Nutzlast; kein Streifen ist unsortiert.

| je Karte: min / Median / p90 / max | Platin DE | HeartGold DE = US |
|---|---|---|
| Punkte | 2 / 16 / 51 / 1 090 | 2 / 15 / 86 / 544 |
| Normalen | 1 / 1 / 4 / 9 | 1 / 1 / 3 / 29 |
| Konstanten | 1 / 2 / 8 / 22 | 1 / 2 / 7 / 40 |
| Platten | 1 / 9 / 27 / 1 033 | 1 / 8 / 56 / 336 |
| Streifen | 1 / 4 / 13 / 36 | 1 / 4 / 16 / 83 |
| Zugriffsliste | 1 / 18 / 91 / 2 084 | 1 / 18,5 / 153 / 2 073 |
| **Nutzlast (B)** | **50 / 300 / 978 / 21 452** (Karte 188) | **50 / 311 / 1 672 / 10 778** (Karte 658) |
| Anteil an 0x9000: p90 / max | 2,7 % / 58,2 % | 4,5 % / 29,2 % |
| Nutzlast < 1 KiB / < 4 KiB / < 8 KiB / < 16 KiB / ≥ 16 KiB | 603 / 61 / 1 / 0 / 1 | 564 / 99 / 9 / 4 / 0 |
| Karten mit genau einer Platte (50 B) | 173 | 214 |
| verschiedene ebene Höhen je Karte (Median / p90 / max) | 1 / 4 / 12 | 2 / 4 / 9 |
| verschiedene ebene Höhen gesamt, Spanne | 49, −96 bis 480 E | 57, −128 bis 496 E |
| ebene Höhen auf Vielfachen von 8 E | 98,6 % | 96,9 % |
| Karten mit geneigten Platten | 311 | 291 |
| geneigte Platten je Karte (max) | 21 | 162 |
| Steigung je Kachel (häufigste) | 16 E: 791, 8 E: 136 | 16 E: 1 475, 19,4 E: 114 |
| Normalen gesamt verschieden | 37 | 108 |
| Plattenecken auf Kachel / halber Kachel / sonst | 90,6 % / 6,2 % / 3,1 % | 74,2 % / 7,5 % / 18,3 % |
| Zugriffe je Platte (Median) | 1,77 | 1,87 |
| Kachelmitten mit mehreren Kandidaten (Brücken, Überlappung) | 3 287 | 4 738 |
| Kachelmitten, an denen die Streifensuche andere Platten liefert als eine Suche über alle | 2 530 (83 Karten) | 1 766 |

„Karten ohne Terrain“ gibt es nicht. Die kleinste BDHC ist eine Platte über die ganze Karte (Nutzlast 50 B); sie ist auch die richtige Vorgabe für eine leere gemalte Karte. Die letzte Zeile heißt: Vanilla-BDHC sind nicht „sauber“. Maßgeblich ist, was die Streifensuche liefert; ein Leser im Studio muss deshalb genau sie nachbauen und nicht geometrisch über alle Platten suchen.

### 2.3 `.bdhcam`

`.bdhcam` ist **kein Vanilla-Format**. Es gehört zu DSPRE's ROM-Toolbox-Patch „BDHCAM“ (`ToolboxDB.cs` `BDHCAMPatchData`): ein Sprung in ARM9 und Overlay (Platin Overlay 5, HGSS Overlay 1) zu einem nachgeladenen Unterprogramm. Die Tabelle kennt nur Platin Englisch und Spanisch sowie HGSS (laut Kommentar auch SoulSilver, Englisch und Spanisch). **Deutsche ROMs werden nicht unterstützt** (kein Eintrag, `NotSupportedException`). DSPRE bietet `.bdhcam` im Dateifilter für Platin und HGSS an und schreibt die Bytes verbatim; es liest und prüft sie nicht.

In den Vanilla-Daten gibt es keinen Zusatzblock: Bei allen 2 018 Karten gilt Länge = 16 + Nutzlast (gemessen). Den Aufbau einer `.bdhcam` kann ich aus erlaubten Quellen nicht belegen. Der Kernel nimmt die Endung heute als `Part::Bdhc` an; eine Datei mit Zusatzdaten scheitert an `bdhc.counters`. Das ist richtig so: ohne den Patch liest das Spiel nur die BDHC.

### 2.4 Wie das Spiel BDHC und Permissions nutzt

**Ablauf je Schritt** (belegt (pret) `terrain_collision_manager.c`, für HGSS die Schwelle in `asm/unk_02054648.s` `sub_02054954`: `mov r0, #5; lsl r0, #0xe` = 0x14000; im Emulator für alle drei Spiele bestätigt):

1. Zielhöhe = BDHC-Höhe in der Mitte der Zielkachel, mit der aktuellen Höhe als Hinweis für die Kandidatenwahl.
2. Unterschied zur aktuellen Höhe **≥ 20 Einheiten** → Schritt verweigert, egal was die Permission sagt (`TerrainCollisionManager_WillPlayerCollide`). Das gilt auch für Figuren (`WillMapObjectCollide`).
3. Sonst entscheidet die Permission: Bit 15 des 16-Bit-Werts (`collision` & 0x80) blockiert. Davor prüft das Spiel dynamische Hindernisse (Gymmicks).
4. Während des Gehens rechnet das Spiel die Höhe laufend aus der BDHC neu. Auf einer Rampe gleitet die Figur, an einer Stufe springt sie.
5. Das Verhalten (`kind`, Bits 0 bis 7) steuert alles andere: Begegnung, Surfen, Sprung, Eis, Tür, Warp. Die Richtungssperren 0x30 bis 0x37 (Platin `BLOCK_EASTWARD` …) sperren das Verlassen der eigenen Kachel bzw. das Betreten der Zielkachel in einer Richtung (`map_object_move.c:717`).

**Wo entscheidet was:**

| Frage | entscheidet |
|---|---|
| Wie hoch steht die Figur? | BDHC (bei Gymmick-Höhen: der dynamische Wert, Platin Pastoria `0x56`–`0x59`) |
| Kann sie über eine Höhenkante? | BDHC: Unterschied ≥ 20 blockiert immer, < 20 nie |
| Ist die Kachel begehbar? | Permission Bit 7 des zweiten Bytes |
| Was passiert auf der Kachel? | Permission `kind` |
| Wo steht das Modell? | Kartenmodell lokal + Matrix-Höhe × 8 (nur Zeichnen) |

**Permissions je Familie** (Werte aus pret, Anteil blockiert gemessen; Pt = Platin, HG = HeartGold):

| `kind` | pret-Name | Fahne | blockiert in Vanilla (Pt / HG) |
|---|---|---|---|
| 0x00 | NONE | | 56 % / 37 % (Boden, Wände, Dächer) |
| 0x02, 0x03 | TALL_GRASS, VERY_TALL_GRASS | Begegnung | 0 % / 0 % |
| 0x06 | HG: HEADBUTT (Kopfnuss-Baum) | | – / 100 % |
| 0x08 | CAVE_FLOOR | Begegnung | 0 % / 0 % |
| 0x10, 0x15 | WATER_RIVER, WATER_SEA | Surfen + Begegnung | 0 % / 0 % bzw. 8 % |
| 0x13 | WATERFALL | Surfen | 60 % / 40 % |
| 0x16, 0x17 | PUDDLE, SHALLOW_WATER | | 0 % / 0 % |
| 0x20 | ICE | Rutschen | 1 % / 2 % |
| 0x21 | SAND | | 0 % / 0 % |
| 0x30–0x37 | Pt: BLOCK_* (Richtungssperren); HG ohne Namen | | |
| 0x38–0x3B | JUMP_EAST/WEST/NORTH/SOUTH (Vorsprung) | | 100 % / 100 % |
| 0x3C–0x3E | HG: LADDER_* | | – / 0–13 % |
| 0x4B, 0x4C | ROCK_CLIMB (Kraxler) | | 100 % / 100 % |
| 0x5E, 0x5F | WARP_STAIRS_EAST/WEST (Treppen-Warp) | | 0 % / 0 % |
| 0x62–0x65 | WARP_ENTRANCE_* | | 22–73 % / 28–41 % |
| 0x69 | DOOR | | 100 % / 97 % |
| 0x6C–0x6F | WARP_EAST/WEST/NORTH/SOUTH | | 0 % / 0 % |
| 0x70–0x7D | BRIDGE_* (Pt), Brücken | 0x72 und „OVER_ENCS“: Begegnung | 0x71: 0 % / 0 % |
| 0xA1–0xA9 | Pt: SNOW_*, MUD_* (0xA6/0xA7 Begegnung); HG: 0xA4 MUD, 0xA8 SNOW | | 0 % / 3 % |

Quellen: `include/constants/field/map_tile_behaviors.h` und `src/map_tile_behavior.c` (Fahnen) in pokeplatinum, `include/constants/metatile_behavior.h` in pokeheartgold. **Eine Treppe hat kein eigenes `kind`.** Geneigte Kacheln tragen meist 0x00 (Pt 1 646, HG 432), sonst das Bodenverhalten (Höhle 0x08, Wasser 0x15, Brücke 0x71). Die Treppe ist allein die Neigung der BDHC; `WARP_STAIRS` sind die Treppen, die die Karte wechseln.

**Das zweite Byte in HGSS** (gemessen und belegt (pret)). Platin kennt dort nur 0x00 und 0x80. HeartGold nutzt in 565 von 676 Karten **die Bits 0 bis 6 des zweiten Bytes** (Werte 0x01 bis 0x0D). Das Spiel liest sie als 7-Bit-Wert (`sub_020548EC`) und benutzt ihn als Index in eine Tabelle von Schrittgeräuschen (`asm/unk_0205CB48.s`, Tabelle `_020FCB98`, `SEQ_SE_GS_…`). Häufig: Sand 0x05, Höhle 0x0A, harter Boden 0x04/0x06, 0 bei 81 % des freien Bodens. Die Kollision ist auch in HGSS nur Bit 7 (`sub_020548C0`: `asr #0xf`). Das Studio muss die Bits erhalten und für gemalte Kacheln einen Wert vorgeben (Vorgabe 0, sonst je Gelände-Muster).

**Matrix-Höhe** (belegt (pret), gemessen). `LandDataManager_CalculateRenderingPosition` setzt das Modell auf y = Höhe × 8 Einheiten. Die Höhensuche (`GetHeight`) zieht nur den Versatz der Zerrwelt ab, nicht die Matrix-Höhe. Eine Karte auf Höhe 2 zeichnet ihr Modell also 16 Einheiten höher, die Figur liefe aber auf der BDHC-Höhe, also im Modell. Die Daten bestätigen das:

| | Platin DE | HeartGold DE = US |
|---|---|---|
| Matrizen mit Höhen | 0000 (30 × 30), 0240 (4 × 5) | 0000 (47 × 17), 0212 (5 × 4) |
| Zellen mit Höhe > 0 | 313 | 286 |
| davon mit Header (begehbar) | **0** | **0** |
| BDHC der Zellen mit Höhe 0: Median (über die Zellen) der mittleren ebenen Plattenhöhe | 16 E | 32 E |
| dasselbe für Zellen mit Höhe > 0 | 0 E, Modell-Hüllquader beginnt bei 0 | ebenso |

Die Matrix-Höhe hebt nur Kulissen-Karten (Meer, Berge am Rand) an. Begehbare Karten haben in allen drei ROMs Höhe 0 und tragen ihre Höhe selbst: Modell und BDHC im selben lokalen Rahmen (z. B. Karte 100: Modell-Hüllquader ab y = 32, BDHC 32 und 96). **Folge für 6.2:** Eine gemalte Karte mit Header muss in einer Zelle mit Höhe 0 liegen. Sonst sitzt die Figur unter dem Modell. Das ist ein neuer Befund für die Prüfungen (§5).

**Gebäude** (belegt (pret) `map_prop.c`): Gebäude sind nur Grafik; Höhe und Kollision kommen nicht aus ihnen. Ihre y-Position muss zur BDHC an ihrer Stelle passen (sonst schweben oder versinken sie). Je Karte lädt Platin **höchstens 32** Gebäude (`MAX_LOADED_MAP_PROPS`), weitere werden still übergangen. Vanilla-Maximum: Pt 32, HG 30 (gemessen). Für HGSS habe ich die Grenze nicht im asm belegt.

**Kamera:** steht im Header (`cameraType`, Pt `u8`, HGSS 6 Bit), nicht in der Kartendatei. Die BDHC trägt keine Kamera, außer mit dem DSPRE-Patch (§2.3).

### 2.5 Emulator-Probe (gemessen, Platin DE, HeartGold DE, HeartGold US)

**Aufbau.** `gutachten-b-bdhc-bau.py probe` erzeugt aus einem synthetischen Raster eine BDHC (340 B: 18 Punkte, 2 Normalen, 5 Konstanten, 9 Platten, 5 Streifen, 12 Zugriffe) und Permissions (alles frei, Verhalten 0, nur (5, 10) mit 0x80). Keine ROM-Ableitung. Das Raster in Kachelkoordinaten der Zimmerkarte:

- Zeile 6: x 0–3 Höhe 0, x 4–5 Rampe 0 → 16 nach Osten (8 je Kachel), x 6–7 Plateau 16, ab x 8 Höhe 40 (Stufe 24);
- Zeile 8: x 0–3 Höhe 0, x 4–5 Höhe 16 ohne Rampe (Stufe 16), ab x 6 Höhe 36 (Stufe genau 20);
- Zeile 10: Höhe 0, (5, 10) blockiert; sonst überall Höhe 0.

Beide Dateien liegen in `<Kopie>/export/` und gehen per Szenario über die Brücke: erst `inspect_map_import` (beide Teile `ok`), dann `import_map_parts` mit `parts: [bdhc, permissions]` in die Zimmerkarte (Platin Karte 187 = Header 415, HeartGold Karte 217 = Header 64), dann `save` und `build_rom`. Danach geht g4emu mit dem Spielstand `…-zimmer.sav` (als Kopie) die Wege ab. Nach jedem Halt liest `dump-ram`, und `gutachten-b-ramhoehe.py` sucht im RAM den Vektor (x, y, z) der Figur zur Kachelmitte; das y ist die Höhe.

**Ergebnis** (alle drei Spiele gleich, Rückgabe 0, alle `expect-at` erfüllt):

| Schritt | erwartet | gemessen (RAM-Höhe, 6 Treffer je Abzug) |
|---|---|---|
| Start (Pt 7, 4; HG 3, 4) | 0 | 0 |
| (2, 6) flach | 0 | 0 |
| (4, 6) Rampenmitte | 3,989 (Rundung der Normalen 0,894) | **3,989** (4 ×), dazu 2 × 0,989 (andere Struktur, vermutlich Schatten oder Kamera) |
| `walk-until-blocked right 8` ab (4, 6) | 3 Schritte bis (7, 6), dann Klippe 24 | **3 Schritte, steht bei (7, 6)**, RAM-Höhe 16 |
| (2, 8) → (4, 8), Stufe 16 ohne Rampe | erlaubt | **erreicht**, RAM-Höhe 16 |
| `walk-until-blocked right 4` ab (4, 8), Stufe 20 | 1 Schritt bis (5, 8) | **1 Schritt, steht bei (5, 8)** |
| `walk-until-blocked right 5` ab (2, 10), `0x80` auf (5, 10) | 2 Schritte bis (4, 10) | **2 Schritte, steht bei (4, 10)** |

Dass die Blockaden aus der BDHC kommen, ist durch die Permissions gesichert: außer (5, 10) ist jede Kachel frei. Die Rampenhöhe 3,989 statt 4 ist genau der Wert, den der Prototyp mit derselben fx32-Rechnung vorausgesagt hat. Das Spiel liest also die erzeugte Datei so, wie das Werkzeug sie versteht. Bilder: `target/agent/gutachten-6b/{pt,hg,hgus}/s0_start.png` bis `s6_vor_kollision.png`. Auf `s3_plateau_vor_klippe.png` steht die Figur sichtbar eine Kachel höher als auf `s1_z6_x2.png`. Dazu die Schrittprotokolle `play.jsonl` und die Szenarien `*-bdhc-probe.yaml`.

Nicht geprüft habe ich in der Probe: Gras mit Begegnung (das Zimmer hat keine Begegnungsdatei; dass importiertes `kind` 0x02 Begegnungen auslöst, belegt schon das 3b-Gesamtgate `pt-kleiner-hack.yaml`), eine BDHC auf einer Außenkarte mit Nachbarn und Brücken.

### 2.6 Erzeugung aus einem Höhenraster (Prototyp gemessen, `gutachten-b-bdhc-bau.py`)

**Eingabe.** 32 × 32 Zellen, je Zelle eine Ebene: flach mit Höhe h oder Rampe (Höhe an einer Kante, Steigung je Kachel, Achse N/O/S/W). Höhe in Stufen zu 8 Einheiten. Das passt zu den Vanilla-Daten (98,6 % / 96,9 % der Höhen auf Vielfachen von 8), zur Matrix-Stufe (`ALTITUDE_STEP` halbe Kachel) und zum Design-System („eine Stufe ist eine halbe Kachel“). Die Steigung ist 8 oder 16 Einheiten je Kachel, also eine oder zwei Stufen; Vanilla nutzt fast nur 16 und 8.

**Algorithmus.**

1. **Rechteck-Zerlegung** je gleicher Ebene: Zeilenweise die erste freie Zelle nehmen, nach rechts wachsen, solange die Ebene gleich ist, dann nach unten, solange die ganze Breite gleich ist. Greedy, nicht minimal, aber schnell und stabil (gleiche Eingabe → gleiche Bytes).
2. **Ebene → fx32:** Aus y = a + bₓx + b_z z wird n = (−bₓ, 1, −b_z)/|…|, d = −a/|…|, gerundet auf 1/4096. Normalen und Konstanten werden entdoppelt.
3. **Punkte:** zwei Ecken je Rechteck, entdoppelt.
4. **Streifen:** die sortierten z-Kanten aller Rechtecke; Streifen i hat `scanline` = Kante i+1. Seine Zugriffsliste enthält die Rechtecke, die das Band **echt überlappen** (z_min < oben und z_max > unten). Das reicht, weil ein Punkt genau auf einer Kante dem Streifen darunter zufällt, und dort liegt das Rechteck, das an der Kante beginnt. Jede Platte steht in so vielen Streifen, wie sie Bänder überdeckt.
5. **Prüfen:** Das Werkzeug liest die erzeugte Datei mit derselben Streifensuche und Kandidatenwahl wie das Spiel. Es vergleicht an jeder Kachelmitte und an vier Punkten eine Viertelkachel daneben mit der Sollhöhe. Größte Abweichung: 0,06 Einheiten (Rundung bei Rampen), keine Lücke.

**Größen** (gemessen am Prototyp, Nutzlast ohne Kopf):

| Raster | Platten | Streifen | Zugriffe | Nutzlast | Anteil an 0x9000 |
|---|---|---|---|---|---|
| flach | 1 | 1 | 1 | 50 B | 0,1 % |
| Route: 3 Ebenen, 4 Treppen | 13 | 5 | 13 | 422 B | 1,1 % |
| Stadt: 6 Dächer, Teich | 20 | 7 | 43 | 646 B | 1,8 % |
| Probe (Emulator) | 9 | 5 | 12 | 324 B | 0,9 % |
| Flecken: 4 Stufen, zufällig, geglättet | 412 | 32 | 958 | 11 232 B | 30,5 % |
| Schachbrett 0/16 | 1 024 | 32 | 1 024 | 19 212 B | 52,1 % |
| jede Kachel eigene Höhe | 1 024 | 32 | 1 024 | 23 300 B | 63,2 % |
| jede Kachel Rampe, 8 Neigungen | 1 024 | 32 | 1 024 | 21 956 B | 59,6 % |

**Obere Schranke (gerechnet):** Auf Kachelgrenzen gibt es höchstens 33 × 33 = 1 089 Punkte, 1 024 Platten, 1 024 Konstanten, 32 Streifen und (wegen der echten Überlappung) 1 024 Zugriffe. Nutzlast ≤ 8 712 + 12 N + 4 096 + 8 192 + 256 + 2 048 = 23 304 + 12 N Byte. Mit N ≤ 1 024 Normalen sind das höchstens 35 592 B < 36 864 B. **Eine so erzeugte BDHC kann 0x9000 nicht überschreiten.** Das Budget „BDHC“ ist für gemalte Karten deshalb nur Anzeige, nie Sperre. (Mit geschlossenen Bändern, wie Vanilla sie oft hat, wären es bis zu 3 072 Zugriffe; dann wäre die Schranke mit vielen Neigungen knapp über 0x9000. Halboffen ist also nicht nur kleiner, sondern auch beweisbar sicher.)

**Grenzen des Ansatzes:** Vanilla-BDHC haben feinere Geometrie: Plattenecken auf halben Kacheln oder schief, in HG 26 % der Ecken. Eine Vanilla-Karte lässt sich deshalb nur verlustbehaftet in ein Kachelraster umwandeln (Abtasten der BDHC in der Kachelmitte, Rampen aus dem Unterschied der Kanten). Brücken (zwei Ebenen an einer Kachel) gehen im Format und in der Engine, aber nicht im einfachen Raster; sie brauchen eine zweite Ebene (§7).

### 2.7 Kollisionsableitung aus Kacheln (Regeln aus Engine und Vanilla-Daten)

Die Engine-Regel ist hart. Wie Vanilla an Höhenkanten blockiert, zeigt die Zählung benachbarter **ebener** Kachelpaare mit verschiedener Höhe (gemessen, Pt / HG):

| Stufe | beide frei | obere blockiert | untere blockiert | beide blockiert |
|---|---|---|---|---|
| 8 E (1 Stufe) | 1 669 / 3 333 | 1 684 / 1 495 | 165 / 158 | 486 / 478 |
| 16 E (2 Stufen) | 147 / 33 | 1 837 / 249 | 1 172 / 196 | 1 627 / 223 |
| 24 E | 15 / 69 | 739 / 431 | 291 / 68 | 86 / 96 |
| 32 E | 167 / 7 | 895 / 44 | 832 / 159 | 1 000 / 42 |

Und benachbarte freie Paare insgesamt: gleich hoch 464 492 / 359 438, unter 8 E 2 403 / 606, 8 bis unter 20 E 4 367 / 1 453 (Rampen und Stufen), ab 20 E 441 / 173 (dort blockiert nur die Höhe).

**Regelsatz (Vorschlag für 6.2, je Zelle aus Gelände-Muster und Nachbarhöhen):**

| Regel | `kind` | `collision` | Begründung |
|---|---|---|---|
| Muster „blockiert“ (Wand, Baum, Haus, Fels) | 0x00 | 0x80 | Vanilla-Boden mit Kollision |
| Boden, Weg | 0x00 | 0x00 | |
| hohes Gras | 0x02 (sehr hoch 0x03) | 0x00 | Begegnungsfahne; braucht Begegnungsdatei mit Gras-Rate > 0 am Header |
| Wasser | 0x15 (Meer) / 0x10 (Fluss) | 0x00 | Surfen kommt aus dem Verhalten, nicht aus der Kollision (0 % blockiert) |
| Eis | 0x20 | 0x00 | |
| Vorsprung | 0x38–0x3B je Sprungrichtung | 0x80 | 100 % in Vanilla: der Sprung ignoriert die Kollision |
| Tür | 0x69 | 0x80 | wie das Tür-Werkzeug (3b.4b) und Vanilla (100 % / 97 %) |
| Treppe (Rampe) | Bodenverhalten (meist 0x00) | 0x00 | BDHC-Neigung 8 oder 16 je Kachel; kein eigenes `kind` |
| Höhenkante 1 Stufe (8 E) ohne Treppe | – | frei | in Vanilla zu 42 % / 61 % frei: Bordstein, Schwelle |
| Höhenkante 2 Stufen (16 E) ohne Treppe | – | **obere Zelle 0x80** | die Engine ließe den Schritt zu; Vanilla blockiert zu 97 % / 95 %, öfter oben als unten |
| Höhenkante ≥ 3 Stufen (≥ 24 E) | – | obere Zelle 0x80 | die Engine blockiert ohnehin; Vanilla setzt trotzdem fast immer 0x80 (klar im Overlay, sicher für Figuren) |
| Steigung je Kachel ≥ 20 E | – | – | **nicht erlaubt** (Mitte zu Mitte ≥ 20 blockiert die eigene Rampe) |
| HGSS Schrittgeräusch (Bits 0–6) | – | Vorgabe 0, je Muster einstellbar | |

Ob die obere oder die untere Zelle blockiert, ist eine Frage des Aussehens. Beides ist vanilla-treu; die obere kommt bei 16 E häufiger vor (1 837 : 1 172 bzw. 249 : 196). Gutachten C schlägt die untere vor (Wandfläche vor der unteren Zelle). Das soll die Spezifikation einmal festlegen (§7); für BDHC und Engine ist es gleichgültig. Die Richtungssperren 0x30–0x37 würden keine Zelle verlieren, belegen aber das `kind` (kein Gras auf der Kante) und sind für HGSS nicht benannt. Deshalb nicht in 6.2.

### 2.8 Engine-Grenzen je Karte

| Grenze | Wert | Quelle | Vanilla-Maximum (Pt / HG) | Art |
|---|---|---|---|---|
| Modell (NSBMD ohne Texturen) | 0xF000 = 61 440 B | pret `loaded_map_buffers.c:11`, HG asm (task0-3b4 §3) | 60 556 B (98,6 %) / 61 420 B (99,97 %) | hart, Fehler (besteht) |
| BDHC-Nutzlast | 0x9000 = 36 864 B | pret `bdhc.h:4`, HG asm | 21 452 B / 10 778 B | hart, Fehler (besteht); gemalt nie erreichbar (§2.6) |
| Permissions | 2 048 B | Format | – | hart (besteht) |
| Gebäude je Karte | 32 (Pt; HG nicht belegt) | pret `map_prop.h:13` | 32 / 30 | still abgeschnitten → **neue Prüfung**, Warnung |
| geladene Karten | **4** (2 × 2 um die Figur), nicht 3 × 3 | pret `QUADRANT_COUNT 4`, vier Modell- und BDHC-Puffer; HG `4 * 0xF000` | – | Grundlage fürs Polygon-Budget |
| Polygone / Vertices je Bild | 2 048 / 6 144 (DS-Hardware, nach Clipping) | GBATEK „DS 3D“ | je Karte laut Modell-Info: Polygone 1 646 / 1 706, Vertices 4 757 / 4 854 | weich, Hinweis |
| Textur-VRAM im Feld | 256 KiB Texturen (Bänke A+B), 64 KiB Paletten (E), geteilt mit Figuren | pret `fieldmap.c:527`, `easy3d.c:83` | je Gebiet (Karten- + Gebäudetexturen, 4×4 mit Index): 107 808 B / 84 944 B; Paletten 4 704 / 4 192 B | weich; Anteil der Figuren nicht gemessen (geschätzt: Schwelle „knapp“ beim Vanilla-Maximum) |
| Höhe der Figur | fx32, Vanilla −128 bis 496 E | – | – | keine |
| Stufe ohne Treppe | < 20 E begehbar, ≥ 20 E blockiert | §2.4 | – | Regel, keine Grenze |

**Polygon-Budget, gemessen:** Summen über 2 × 2-Fenster der Matrizen (so viele Karten sind geladen). Platin: Median 911 Polygone, p90 2 311, max 4 573; 149 von 1 071 Fenstern über 2 048. HeartGold: Median 814, p90 2 509, max 5 939; 134 von 881 über 2 048. Bei den Vertices liegen 234 bzw. 139 Fenster über 6 144. Vanilla überschreitet die Hardware-Grenze also rechnerisch oft. Was zählt, ist der sichtbare Ausschnitt nach Clipping. **Eine harte Polygon-Grenze je Karte lässt sich daraus nicht ableiten.** Das Studio soll die Modell-Zähler zeigen und ab dem Vanilla-Maximum (rund 1 700 Polygone, 4 850 Vertices) „knapp“ melden.

**Platin vs. HGSS:**

| | Platin | HGSS |
|---|---|---|
| BGS-Block in der Kartendatei | nein | ja (`0x1234`, leer `34 12 00 00`) |
| zweites Permission-Byte | nur 0x00/0x80 | Bit 7 Kollision, Bits 0–6 Schrittgeräusch |
| BDHC-Format, Höhensuche, 20-E-Regel | gleich | gleich (Probe identisch) |
| Puffer Modell/BDHC | 0xF000 / 0x9000 | gleich |
| Matrix-Höhen | nur Kulisse | nur Kulisse |
| `.bdhcam` | DSPRE-Patch (nicht DE) | DSPRE-Patch (nicht DE) |
| Kamera | Header `cameraType` | Header `cameraType` (6 Bit) |
| `kind`-Werte | 0x30–0x37 Richtungssperren benannt | 0x06 Kopfnuss, 0x3C–0x3E Leitern, 0x2C Magma |

**Budget je Karte im Studio** (Baustein „Budget“ des Design-Systems: Ruhig, Knapp, Voll; Speichern blockiert nie):

1. **Modell** „n von 61 440 B“: Voll = Fehler (besteht), Knapp ab 90 %.
2. **BDHC** „n von 36 864 B“: Voll = Fehler (besteht); bei gemalten Karten immer ruhig.
3. **Gebäude** „n von 32“: Voll = Warnung „wird im Spiel nicht geladen“.
4. **Polygone / Vertices** des Modells: nur Hinweis ab Vanilla-Maximum.
5. **Texturen des Gebiets** (VRAM) „n von 262 144 B“: Knapp ab dem Vanilla-Maximum (rund 108 KB), Voll geschätzt ab 200 KB. Gehört zum Gebiet, nicht zur Karte; an der Karte nur verlinkt.

### 2.9 DSPRE-Kompatibilität (nur Leser, nur Fakten)

`MapFile(Stream, gameFamily)` liest der Reihe nach:

1. vier `Int32`-Längen (Permissions, Gebäude, Modell, BDHC);
2. **nur HGSS:** `u16` Signatur; ist sie nicht 0x1234, meldet DSPRE „BackGround Sound data is corrupted“ und markiert die Karte als fehlerhaft, sonst `u16` Länge und den Block;
3. Permissions: so viele Bytes wie die Länge, daraus fest 32 × 32 Paare. Weniger als 2 048 Byte führen zu einem Lesefehler;
4. Gebäude: Länge / 48 Einträge zu 48 Byte, ein Rest wird nicht gelesen;
5. Modell: erstes `u32` muss `BMD0` sein (sonst Meldung „Please select an NSBMD file“, Modell leer). Hat die Datei mehr als einen Block (`u16` an 0x0E > 1), entfernt DSPRE die eingebetteten Texturen; dann parst es das Modell mit seinem NSBMD-Leser;
6. BDHC: die Bytes verbatim. **Keine Prüfung**, keine Zähler, die Oberfläche zeigt nur die Größe; Import und Export nehmen jede Datei (Filter `.bdhc`, bei Pt/HGSS auch `.bdhcam`).

Eine gemalte Karte öffnet DSPRE also, wenn die Längen stimmen, die Permissions 2 048 B haben, HGSS den BGS-Block trägt und DSPRE das Modell lesen kann. Die BDHC ist für DSPRE ohne Belang; für das Spiel gelten §2.1 und §2.6. Ob DSPRE's NSBMD-Leser ein Studio-Modell annimmt, gehört zu Gutachter A.

### 2.10 Studio heute und Abgleich mit Gutachten C

**Frontend (gelesen).** Die Ausgangslage sagt, Höhen, Picking und Event-Drag folgten der GLB-Oberfläche. Das stimmt nicht ganz: **Sie folgen Ebenen auf der Matrix-Höhe.** `picking.ts` `surfaceHit` schneidet den Sichtstrahl mit einer Ebene je Höhenstufe (`levelPlanes(m.altitudes)`). `eventDrag.ts` nimmt `picker.surfacePoint`. `markers.ts` setzt Marker auf `cellYAtTile`, `content.ts` das Gelände-Overlay auf `OVERLAY_Y + cellY`. Die GLB von apicula wird nur gezeichnet. Weil begehbare Zellen immer Höhe 0 haben (§2.4), liegt all das auf y = 0. Der Boden liegt aber auf 57 % (Pt) bzw. 61 % (HG) der begehbaren Kacheln höher, auf 46 % bzw. 48 % mindestens eine Kachel (446 bzw. 524 Karten, gemessen). In der Schrägsicht verschiebt das Marker und Overlay (Parallaxe), und `pickTile` trifft am Hang die falsche Kachel. In der Draufsicht (orthografisch) fällt es nicht auf. Eine BDHC liest die App heute nirgends (`bdhc` kommt nur in Import, Export, Verlauf und Diff vor).

**Abgleich mit Gutachten C.** C empfiehlt M1 (Zellraster, Höhe je Zelle in halben Kacheln, Rampen eine oder zwei halbe Kacheln je Zelle). Das deckt sich mit §2.6: 8 E je Stufe, Steigung 8 oder 16. C misst „verschiedene Höhen waagerechter Flächen je Karte, Median 5 / 8“ am **Modell**; meine 1 / 2 messen die **BDHC**. Beides stimmt: Das Modell hat Dächer, Tische und Kanten, auf denen niemand läuft; die BDHC nur begehbare Flächen. Für die Malhöhe zählt die BDHC. Zwei Punkte weichen ab:

- C nennt für die Treppe ein `kind` „Treppe“. Ein solches gibt es nicht (§2.4); die Treppe ist die Rampe mit dem Bodenverhalten.
- C blockiert an Klippen die untere Zelle, ich schlage die obere vor (§2.7). Beides ist vanilla-treu.

C's offene Frage 4 (Brücken): Format und Engine tragen zwei Platten an einer Stelle (bis 10 Kandidaten, die nächste Höhe gewinnt). Vanilla nutzt das an 3 287 / 4 738 Kachelmitten. Die Permission ist aber nur eine Schicht: Das Brücken-`kind` (0x70–0x7D) regelt, was oben und unten geht (`MapObject_IsStatusOnElevatedBridge`). Machbar, aber nicht in 6.2.

## 3. Optionen

| Option | Inhalt | für | gegen |
|---|---|---|---|
| **A** BDHC bleibt verbatim | gemalte Karten bekommen die Ein-Platten-BDHC (flach 0) oder eine importierte | null Aufwand | keine Höhen beim Malen; Gate „Treppe steigen“ unmöglich |
| **B** BDHC aus dem Höhenraster | §2.6: Rechtecke je Ebene, Rampen 8/16, halboffene Streifen; Vanilla bleibt verbatim, wird aber gelesen (Höhensuche wie im Spiel) | klein (ein Codec, ein Generator), beweisbar ≤ 0x9000, eine Quelle für Modell, BDHC und Kollision, im Emulator erprobt | Vanilla nur verlustbehaftet umwandelbar; keine Brücken |
| **C** BDHC aus dem gebackenen Modell | waagerechte und geneigte Dreiecke des NSBMD je Kachel abtasten | folgt jedem Modell, auch importiertem OBJ | Dächer, Zäune und Deko werden begehbar; Mehrdeutigkeit je Kachel; hängt am Writer von A; schwer zu testen |
| **D** voller BDHC-Editor | Platten frei ziehen, Teilkacheln, Brücken | volle Vanilla-Treue | groß; die Bedienung passt nicht zum Malen; 6.2 bräuchte Wochen |

Bei C ist die Quelle der Höhe das Ergebnis eines anderen Schritts. Ein Fehler im Writer wird dann zur Laufhöhe; der Nutzer sieht ihn erst im Spiel. Bei B sind Modell und BDHC zwei Ableitungen derselben Zellen. Sie passen zusammen, wenn beide dieselbe Höhe je Zelle nehmen. Das ist mit einem einzigen Test prüfbar: Höhe der BDHC in der Zellmitte = Höhe der Bodenfläche des Modells dort.

## 4. Empfehlung (recommend)

**Option B.** Im Einzelnen:

1. **Datenmodell:** je Zelle `h` (ganzzahlig, Stufen zu 8 E; Bereich −16 bis 62 deckt Vanilla −128 bis 496) und `ramp` (keine oder N/O/S/W mit 1 oder 2 Stufen je Zelle). Das ist dasselbe wie M1 in Gutachten C.
2. **Codec im Kernel (`g4-formats/src/bdhc.rs`):** lesen, prüfen (zusätzlich Indizes, Sortierung, Streifenbereiche als Fehler; `n_y` = 0 als Warnung), schreiben, und die Höhensuche des Spiels (§2.1) als Funktion `height_at(x, z, current)`.
3. **Generator:** Raster → BDHC nach §2.6, deterministisch. Rundtrip-Test: jede Zelle an fünf Punkten gegen die Sollhöhe (≤ 0,1 E), Größe immer ≤ 0x9000 (Eigenschaftstest über Zufallsraster).
4. **Kollisionsableitung** nach §2.7 aus Muster und Nachbarhöhen, mit Vorschau im Gelände-Overlay. Was der Nutzer von Hand setzt, überschreibt die Ableitung je Zelle und bleibt erhalten.
5. **Höhen im Studio aus der BDHC** (alle Karten, auch Vanilla): Overlay, Marker, Picking und Event-Drag nehmen `height_at` in der Kachelmitte statt der Matrix-Ebene. Dazu die Schicht „Höhen“ (Design-System: sieben Stufen cividis, Isolinien bei Stufenwechsel).
6. **Neue Prüfungen:**
   - `map.altitude-walkable` (Fehler): Zelle mit Header und Matrix-Höhe ≠ 0; Vanilla 0 Treffer.
   - `map.step-unblocked` (Hinweis, mit Fix „obere Zelle blockieren“ oder „Treppe setzen“): Kante 2 Stufen ohne Treppe und ohne Kollision.
   - `map.buildings-over-limit` (Warnung): mehr als 32 Gebäude.
   - `map.bdhc-size` um die Strukturregeln von §2.1 erweitern.
7. **Vanilla-Karten** bleiben verbatim, bis der Nutzer ausdrücklich „in Malkarte umwandeln“ wählt: BDHC an den Kachelmitten abtasten, Abweichungen als Liste zeigen. Das gehört in 6.3, nicht in 6.2.

## 5. Schnitt 6.2 „BDHC“ und Aufwand

**Inhalt:** Punkte 2 bis 6 der Empfehlung. Nicht in 6.2: Modell-Baking (Gutachter A, 6.1), Malwerkzeuge und Weltformat (Gutachter C), Umwandeln von Vanilla, Brücken, `.bdhcam`, Gymmick-Höhen.

**Arbeitspakete** (geschätzt, Agententage):

| AP | Inhalt | Aufwand |
|---|---|---|
| 6.2.1 | `bdhc.rs`: Leser, Prüfung, Writer, `height_at`; `docs/formats/bdhc.md`; gated Test: alle 2 018 Vanilla-BDHC lesen und byte-gleich schreiben, Strukturregeln 0 Treffer | 1 |
| 6.2.2 | Generator Raster → BDHC, Rundtrip- und Eigenschaftstests (Größe, Höhe, Lücken) | 1 |
| 6.2.3 | Kollisionsableitung (Regeltabelle je Familie in `metadata/`, HGSS-Schrittgeräusch-Vorgabe) und die drei neuen Prüfungen mit deutschen Texten und Fix | 1 |
| 6.2.4 | Kernel-Befehl `get_map_heights` (32 × 32 Höhen der Kachelmitten je Karte) und Frontend: Overlay, Marker, Picking, Drag auf BDHC-Höhe; Schicht „Höhen“ | 1,5 |
| 6.2.5 | Budget-Anzeige je Karte (§2.8) im Inspector | 0,5 |
| 6.2.6 | g4emu: Schritt `expect-height Y [±T]` (RAM-Suche wie `gutachten-b-ramhoehe.py`), Szenarien `pt-gelaende.yaml` und `hg-gelaende.yaml`, Gate-Datei | 1 |
| | **Summe** | **6** |

**Gate 6.2** (messbar, Agent, je auf frischer Kopie von Platin DE und HeartGold DE; HeartGold US optional mit `game: IPKE`):

1. Kernel: gated Vanilla-Rundtrip aller BDHC byte-gleich; Strukturregeln 0 Treffer; Generator-Eigenschaftstest (1 000 Zufallsraster, alle ≤ 0x9000, Abweichung ≤ 0,1 E).
2. Szenario: ein 1 × 1-Gebiet (`create_area`), dessen Karte per Malbefehl (oder bis 6.3 per Kernel-Befehl „Höhenraster setzen“) drei Ebenen bekommt: 0, 16, 40. Dazu eine Treppe 0 → 16, eine Kante 16 → 40 ohne Treppe, eine Wand („blockiert“) und ein Grasfeld (Begegnungsdatei mit Rate 100). Bauen, spielen:
   - **Treppe:** `walk` über die Rampe, `expect-height 16` auf dem Plateau;
   - **Klippe:** `walk-until-blocked` + `expect-at` vor der Kante 24;
   - **Stufe 2 ohne Treppe:** die Prüfung `map.step-unblocked` meldet sie, ihr Fix blockiert die obere Zelle, danach `walk-until-blocked`;
   - **Wand:** `walk-until-blocked`;
   - **Gras:** `walk-until-battle` + `expect-battle`;
   - Bilder an jeder Station.
3. Gegenprobe: dasselbe Szenario mit der flachen Ein-Platten-BDHC scheitert am Schritt `expect-height` (Rückgabe 2).
4. Studio (Test-API im Browser): auf Vanilla Platin Karte 100 (Boden 32 E) stehen Marker und Overlay auf 2 Kacheln Höhe; `pickTile` am Hang trifft die Kachel unter dem Zeiger (Statusleiste „Kachel x …“).
5. Hand: DSPRE (Windows) öffnet die gebaute Karte (Längen, BGS) und zeigt die BDHC-Größe.

## 6. Risiken

| Risiko | Folge | Gegenmittel |
|---|---|---|
| Stufen von 16 E sind begehbar | gemalte Terrassen lassen die Figur hochsteigen, obwohl es wie eine Wand aussieht | Ableitung blockiert ab 2 Stufen; Prüfung `map.step-unblocked` |
| Matrix-Höhe ≠ 0 an begehbarer Zelle | Figur läuft im Modell | `map.altitude-walkable` als Fehler (0 Vanilla-Treffer) |
| Modell und BDHC weichen ab (Writer von A nimmt andere Höhen) | Figur schwebt oder versinkt | eine Quelle (Zellraster) für beide; Test Modellboden = BDHC in Zellmitte; Bild im Gate |
| Überlauf über 0x9000 | stilles Überschreiben des Heaps, kein Absturz | Fehler bleibt hart; gemalte BDHC nach Konstruktion ≤ 35 592 B |
| Vanilla-Feinheiten (halbe Kacheln, schiefe Platten, HG 26 %) | Umwandeln ins Raster verliert sie | Vanilla verbatim; Umwandeln nur ausdrücklich, mit Liste |
| fx32-Rundung an Rampen | bis 0,06 E Abweichung | ohne Folge (Schwelle 20 E); in Tests Toleranz 0,1 E |
| HGSS-Schrittgeräusch falsch | falscher Klang, sonst nichts | Vorgabe 0 (81 % des Vanilla-Bodens), je Muster einstellbar |
| Polygon- und VRAM-Budget nicht hart bestimmbar | Ruckeln oder fehlende Texturen erst im Spiel | nur Hinweise ab Vanilla-Maximum; Anteil der Figuren am VRAM offen |
| `expect-height` liest RAM per Suche | Treffer könnten mehrdeutig sein | Mehrheit der Treffer (Probe: 6 gleiche), Toleranz |

## 7. Offene Punkte für die Spezifikation

1. Welche Zelle blockiert an Höhenkanten, die obere oder die untere (Abgleich mit C)? Vorschlag: obere, einstellbar je Kantensatz.
2. Wertebereich und Anzeige der Höhe: Stufen −16 bis 62 (8 E) und sieben Farben cividis im Zyklus?
3. Brücken (zwei Ebenen) und ihre Permissions: in welcher Teilphase?
4. Rampen nur entlang einer Achse? Vanilla hat fast nur achsparallele Neigungen; Ecken und Diagonalen fehlen im Vorschlag.
5. Soll `kind` 0x30–0x37 (Richtungssperren) für Kanten ohne Zellverlust angeboten werden? Für HGSS fehlt der Beleg.
6. Grenze der Gebäude in HGSS im asm belegen (Vanilla max 30).
7. Textur-VRAM: wie viel brauchen Figuren und Effekte im Feld? Messen per RAM/VRAM-Abzug in g4emu (nicht gemacht).
8. `.bdhcam`: nicht unterstützen. Der Import sollte bei dieser Endung sagen, dass sie einen DSPRE-Patch braucht, der für deutsche ROMs nicht existiert.

## Anhang A: Werkzeuge, Bilder, Kopien

| Datei | Zweck |
|---|---|
| `docs/research/2026-11-05-phase-6/gutachten-b-messung.py` | alle Zählungen von §2.2, §2.4, §2.7 (Paare), §2.8: `python3 -I -B gutachten-b-messung.py <Kopie> pt\|hg [--json <Datei>]`; gibt nur Zahlen aus |
| `docs/research/2026-11-05-phase-6/gutachten-b-bdhc-bau.py` | Generator und Prüfer (§2.6): `groessen` (Tabelle), `probe <Ordner>` (Probe-BDHC und -Permissions, synthetisch) |
| `docs/research/2026-11-05-phase-6/gutachten-b-ramhoehe.py` | Figurenhöhe aus `dump-ram`-Abzügen (§2.5) |
| `target/agent/gutachten-6b/{pt,hg,hgus}/s*.png`, `play.jsonl` | Bilder und Schrittprotokolle der Probe |
| `target/agent/gutachten-6b/{pt,hg,hgus}-bdhc-probe.yaml` | die Szenarien (Aufruf: `uv run g4emu scenario <yaml> --project <Kopie> --out <Ordner> --set KOPIE=<Kopie>` mit `G4_ROMS_DIR`, `G4_BRIDGE_BIN`, `G4K_BIN`) |

Die Zahlen von §2.2 und §2.7 (Stufen, Seiten, Anteile je `kind`, Indexprüfung, Höhe ≠ 0) kamen teils aus kurzen Abfragen im Scratchpad, die die Funktionen von `gutachten-b-messung.py` aufrufen; die Rechenwege stehen im Text.

**Testkopien:** `Roms/_testkopien/6-gutachten-b-pt`, `6-gutachten-b-hg`, `6-gutachten-b-hgus`. Jede ist durch die Probe verändert (Zimmerkarte 187 bzw. 217 mit Probe-BDHC und -Permissions, `export/probe.{bdhc,per}`, gebautes `<Kopie>/out.nds`). Für weitere Messungen an Vanilla eine neue Kopie nehmen. Die Spielstände unter `_saves` wurden nicht verändert (g4emu kopiert sie). Brücke und g4k liefen als Kopien der vorhandenen Debug-Binaries im Scratchpad (`studio-bridge batch`, keine Ports).

## Anhang B: Quellen und Lizenzen

| Quelle | Lizenz | genutzt als |
|---|---|---|
| pret/pokeplatinum `80c83a6f` | keine Lizenzdatei im Repo | Formatwissen und Verhalten (gelesen); kein Code übernommen. Die Werkzeuge setzen die Höhensuche nach der Beschreibung in §2.1 um, die Streifensuche als gemessene Regel |
| pret/pokeheartgold `e1d5a460` | keine Lizenzdatei im Repo | asm-Stellen für HGSS (Schwelle, Kollisionsbit, Schrittgeräusch, Assert) |
| DSPRE `e751187b` | AGPL-3.0 | nur Fakten aus dem Leser `MapFile.cs`, den BDHC-Knöpfen in `MapEditor.cs` und der Patch-Tabelle; Writer und Baking nicht geöffnet, nichts übernommen |
| GBATEK (Martin Korth) | frei zugängliche Dokumentation | DS-Grenzen 2 048 Polygone und 6 144 Vertices je Bild |
| gen4-studio (Repo) | MIT | Dokumentation, Kernel, g4emu; die drei Werkzeuge stehen unter MIT |
| Spiel-ROMs (Kopien) | – | nur gemessen; nichts davon im Repo, die Werkzeuge geben nur Zahlen aus |
