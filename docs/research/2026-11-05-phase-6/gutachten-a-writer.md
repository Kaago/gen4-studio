# Gutachten A: NSBMD- und NSBTX-Writer (Phase 6)

Stand: 05.11.2026. Gutachter: Architektur-Review (nur lesend am Code, mit eigenen Messungen und Prototypen). Repo: `$HOME/Projekte/lab/editor/gen4-studio`, Branch `main` (`91ba0615`).
Datenbasis: frische Kopien der DSPRE-Ordner von Platin DE (`CPUD`), HeartGold DE (`IPKD`) und HeartGold US (`IPKE`) unter `Roms/_testkopien/6-gutachten-a-{pt,hg,hgus}`. Die Originale unter `Roms/` blieben unberührt. Gemessen wurde an unveränderten Kopien; erst danach wurden in den Kopien von Platin und HeartGold DE einzelne Karten für die Spielprobe ersetzt (§2.7, Liste in §9).

Werkzeuge in diesem Ordner (geben nur Zahlen, Namen, Offsets und Verteilungen aus, keine Bytes der ROM-Dateien):
- `gutachten-a-nitro.py` (Python 3, nur Standardbibliothek, `python3 -I gutachten-a-nitro.py inventar|roundtrip <projekt> <pt|hg>`): Leser für BMD0/MDL0 (Modellkopf, Knochen, SBC, Materialien mit Textur- und Palettenpaarung, Shapes mit GPU-Display-Lists) und BTX0/TEX0, Zuordnung Karte → Header → Matrix → Area-Data → Texturpaket, Patricia-Baum der Wörterbücher, ein Layout-Writer (Struktur → Bytes mit neu berechneten Offsets) und ein NSBTX-Writer im Vanilla-Layout.
- `gutachten-a-minimal.py` (Python 3, lädt die Bausteine aus `gutachten-a-nitro.py` über `runpy`): minimaler Writer für gebackene Kacheln (`quad`, `room`, `size`, `merge`).
- `gutachten-a-proto/` (Rust, eigener Workspace; `CARGO_TARGET_DIR=<scratchpad> cargo run --release -- synth|png|cmp|dicts`): NSBTX-Writer aus RGBA mit Quantisierung (eigener Median-Cut und `color_quant`), Patricia-Baum, Rückprüfung über den Leser von `g4-formats` (Pfad-Abhängigkeit, unverändert) und gegen die PNGs von apicula.

„Gemessen“ heißt: an den Testkopien mit diesen Werkzeugen, mit `g4k build`, apicula (`tools/apicula/bin/apicula`, Fassung `3d4e91e` aus `scripts/tool-versions.env`) oder `g4emu` ermittelt. „Geschätzt“ ist ausdrücklich gekennzeichnet. Zeiten (Last um 2,7, zwei weitere Gutachter parallel): `inventar` 8,1 s, `roundtrip` 5,6 s je Spiel; `g4k build` 13,8 s; ein `g4emu run` mit `continue … shot` 3,4 s.

---

## 1. Anlass

Phase 6 soll Karten im Studio malen: Kacheln → gebackenes NSBMD (Geometrie, Material, Textur) plus BDHC und Kollision. Heute kann das Studio Kartenmodelle nur als fertige Teile importieren und exportieren; es schreibt kein einziges NSBMD- oder NSBTX-Byte selbst. Dieses Gutachten beantwortet:
1. was die Vanilla-Kartenmodelle und -Texturpakete tatsächlich nutzen (§2.2 bis §2.4),
2. welche Teilmenge das Spiel zwingend braucht und welche Vereinfachungen es trägt (§2.5),
3. ob ein voller Writer mit byte-gleichem Round-Trip oder ein minimaler Writer der richtige Weg ist, mit Prototyp im Spiel (§2.6, §2.7, §3),
4. was ein NSBTX-Writer braucht (§2.4, §2.8),
5. welche Quellen und Crates lizenzrechtlich gehen (§2.9),
6. wie die Teilphase 6.1 „Writer“ geschnitten wird (§5).

BDHC und Engine-Grenzen (Gutachten B) sowie Malen, Import und Tiled (Gutachten C) werden nur berührt, wo der Writer etwas von ihnen braucht.

---

## 2. Befund

### 2.1 Was das Repo heute trägt (nachgeprüft)

- **`crates/g4-formats/src/map.rs`, `docs/formats/map.md`:** Kartendatei mit Kopf (vier Längen), BGS (nur HGSS), 1024 Permission-Paaren, Gebäuden zu 48 Byte und `buildings_tail`; `model` und `bdhc` sind undurchsichtige Bytes, `write(read(x)) == x` für alle 2.018 Vanilla-Karten. Der Writer muss also nur die Bytes des Modells liefern; die Kartendatei rechnet die Längen selbst.
- **`crates/g4-formats/src/nsbmd_header.rs`, `docs/formats/nsbmd_header.md`:** Container `BMD0`, Blockoffsets, `model_count`, Namen der Textur- und Palettenpaarung, `strip_tex0`. Kein Leser für Knochen, SBC, Materialfelder, Shapes oder Display Lists, kein Writer. Der Patricia-Baum der Wörterbücher ist nicht beschrieben (die Doku nennt nur Kopf, Eintragsblock und Namen).
- **`crates/g4-formats/src/nsbtx.rs`, `docs/formats/nsbtx.md`:** Leser und Dekoder (Formate 1 bis 4, 6, 7; Format 5 abgelehnt), kein Writer.
- **`crates/g4-kernel/src/map_import.rs`:** prüft vor dem Import Blockzahl, `model_count == 1`, Größe gegen `model.max_bytes` = 0xF000 (`check_limits.yaml`, `verified: true`, Quelle `MAP_MODEL_FILE_SIZE`), Ausrichtung auf 4, entfernt TEX0, meldet fehlende Texturnamen gegen das Paket des Gebiets als Hinweis. Ein Import ist ein `edit_map`-Schritt (Undo).
- **`crates/g4-kernel/src/export.rs`:** `nsbmd_to_glb` ruft `apicula convert -f=glb <nsbmd> [<nsbtx>] -o <dir>` mit Zeitlimit, verlangt genau eine GLB (apicula endet auch bei Lesefehlern mit 0), Cache-Schlüssel über Werkzeug-Fingerabdruck, NSBMD und NSBTX. Die Szene im Frontend ist three.js auf dieser GLB.
- **Abhängigkeiten:** `image` 0.25 (nur `png`, `jpeg`) und `png` 0.18 sind schon Abhängigkeiten von `g4-kernel`; `deny.toml` erlaubt nur MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, 0BSD u. a., keine GPL.

### 2.2 Inventar der Vanilla-Kartenmodelle (gemessen)

| | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| Kartenmodelle (Lesefehler) | 666 (0) | 676 (0) | 676 (0) |
| Größe min / Median / max (Byte) | 476 / 20.272 / 60.556 | 416 / 23.196 / 61.420 | 416 / 23.196 / 61.420 |
| Modelle ≥ 75 % bzw. ≥ 90 % von 0xF000 | 46 / 12 | 84 / 38 | wie DE |
| Blöcke | nur `MDL0` (666) | nur `MDL0` (676) | wie DE |
| Modelle je Datei | 1 (alle) | 1 (alle) | 1 |
| Knochen je Modell: Median / max; Dateien mit > 1 | 1 / 25; 33 | 1 / 1; 0 | wie DE |
| Materialien je Modell: Median / max | 10 / 39 | 20,5 / 44 | wie DE |
| Shapes je Modell: Median / max | 10 / 42 | 17 / 38 | wie DE |
| Texturnamen / Palettennamen je Modell (Median / max) | 9 / 39, 9 / 39 | 16 / 33, 16 / 32 | wie DE |
| Ecken je Modell (Median / max) | 1.534 / 4.757 | 1.668 / 4.854 | wie DE |
| Polygone je Modell (Median / max) | 447 / 1.646 | 553,5 / 1.706 | wie DE |
| Bytes der Display Lists je Polygon (Median) | 39,0 | 36,6 | wie DE |
| Reihenfolge der Abschnitte im Modell | SBC > Material > Shape > EVP (alle) | ebenso | ebenso |
| Hüllmatrizen (EVP, Skinning, SBC `0x09`) | keine (EVP-Offset = Modellende, alle) | keine | keine |
| `posScale` (`up_scale`) | 64 (458), 32 (198), 16, 128, 8 | 64 (611), 32 (46), 128 (17), 1, 4 | wie DE |

HeartGold US weicht in der Summe um 660 Byte ab (vier Karten, wie schon aus Phase 4 bekannt); alle Verteilungen sonst gleich.

**SBC (Render-Befehle).** Platin hat 30 verschiedene Muster, HeartGold 5. Das Grundmuster `26 02 0b (04 05)* 2b 01` (Knoten 0 mit Ablage auf Stapel 0, Sichtbarkeit, Hochskalieren, je Material Binden + Zeichnen, Herunterskalieren, Ende) haben 628 von 666 bzw. 667 von 676 Modellen; `06 02 0b … 2b 01` weitere 5 bzw. 6. Die 33 Platin-Modelle mit mehreren Knochen nutzen `0x26`, `0x46`, `0x66` (Matrix laden/ablegen) und `0x24`/`0x44` (Material-Varianten). Knochen-Flags: `0xF807` (Identität) 710 von 917 bzw. 671 von 676, sonst mit Verschiebung, Rotation (Pivot) oder Skalierung. Kein `MTX_RESTORE` in irgendeiner Display List.

**Display Lists (GPU-Befehle).**

| | Platin | HeartGold |
|---|---|---|
| `BEGIN_VTXS` gesamt | 69.498 | 97.579 |
| davon Quad-Strips / Tri-Strips / Quads / Dreiecke | 50.196 / 11.036 / 6.414 / 1.852 | 60.885 / 24.913 / 8.628 / 3.153 |
| Dateien nur mit Quads und Quad-Strips | 88 | 91 |
| `VTX_XY` / `VTX_XZ` / `VTX_YZ` | 378.068 / 335.081 / 100.177 | 449.154 / 444.361 / 82.118 |
| `VTX_10` / `VTX_16` / `VTX_DIFF` | 196.032 / 52.332 / 5.297 | 91.358 / 164.516 / 48.341 |
| `TEXCOORD` | 1.030.833 | 1.235.085 |
| `NORMAL` / `COLOR` | 249.757 / 88.911 | 165.576 / 134.127 |
| Shapes mit Normale / mit Farbe / mit Texturkoordinate (von) | 4.406 / 3.135 / 7.360 (7.547) | 5.495 / 5.491 / 10.693 (10.818) |
| Shape-Flag (Bit 0 Normale, 1 Farbe, 2 Texcoord) | 0x5: 4.340, 0x6: 2.934, Rest 273 | 0x5: 5.263, 0x6: 5.173, Rest 382 |

Das Shape-Flag stimmt mit dem Inhalt der Display List überein (Normale ↔ Bit 0, Farbe ↔ Bit 1, Texturkoordinate ↔ Bit 2). Die Zähler im Modellkopf (Ecken, Polygone, Dreiecke, Quads) lassen sich aus den Display Lists exakt nachrechnen: Platin 666 von 666; HeartGold DE 673 von 676, US 672 von 676 (Karten 0468, 0475, 0476, US zusätzlich 0473 haben veraltete Zähler, das Spiel verlässt sich also nicht darauf).

### 2.3 Materialien (gemessen)

| Feld (GBATEK `POLYGON_ATTR`, `TEXIMAGE_PARAM`) | Platin (7.346 Materialien) | HeartGold (13.247) |
|---|---|---|
| Polygonmodus | Modulation 100 % | Modulation 100 % |
| Licht 0 an / alle aus / andere | 4.300 / 2.955 / 91 | 7.924 / 5.319 / 4 |
| Rückseite zeichnen (Dateien) | 54 (51) | 21 (5) |
| Vorderseite zeichnen | 100 % | 100 % |
| Nebel an | 6.207 | 11.211 |
| Alpha < 31 (Dateien) | 155 (107) | 972 (523) |
| Polygon-ID ≠ 0 | 470 | 930 |
| `poly_attr_mask` | `0x3F1FF8FF` (alle) | ebenso |
| Textur wiederholen S und T / nur S / nur T / keins | 6.369 / 346 / 96 / 535 | 12.283 / 307 / 154 / 501 (dazu 2 mit Spiegeln) |
| Texturkoordinaten-Transformation (texgen) | 16 Materialien (8 Dateien) | 2.623 (420 Dateien) |
| Größe 44 Byte / mit Texturmatrix 48 oder 52 | 7.330 / 16 | 13.055 / 192 |
| `dif_amb` häufigster Wert | `0x7FFFE739` (5.480) | `0x7FFFE739` (8.952) |
| `spe_emi` = 0 | 7.315 | 13.208 |
| `texImageParamMask` | `0xFFFFFFFF` (alle) | ebenso |
| Material ohne Textur (Breite 0) | 170 | 2.556 |

**Zusammenhang mit dem Licht des Gebiets** (Area-Data, Karten mit eindeutigem Gebiet):
- Platin, Lichttyp 0 (außen): 3.669 Materialien, alle mit Licht 0 und Normale. Lichttyp 1 und 2 (innen): 2.847 ohne Licht mit Eckenfarbe, 499 mit Licht.
- HeartGold, Lichttyp 1: 6.834 mit Licht, 26 ohne. Lichttyp 0: 5.292 ohne Licht, 743 mit Licht.
- pret (`area_data.c:205`) nennt für Platin: Lichtarchiv 0 oder 3 = außen; dann ruft der Lader `AreaLight_UseGlobalModelAttributes` (Diffus, Ambient, Spekular, Emission des Modells werden durch die globalen Werte der Tageszeit ersetzt). Für HeartGold ist die Regel nur gemessen, nicht in pret nachgelesen (der Lader liegt dort in Assembler).

### 2.4 Texturen: wo sie liegen und wie das Material sie findet (gemessen)

- **Kein Kartenmodell trägt Texturen**: alle 2.018 Modelle haben nur `MDL0`. Die Texturen liegen je Gebiet in einem NSBTX: Header → `area_data_id` → Area-Data → `map_tileset` → `unpacked/mapTextures/NNNN` (Platin 74, HeartGold 106 Pakete). Gebäude haben ein eigenes Paket (`buildings_tileset` → `buildingTextures`).
- **Bindung per Name**: Das Material nennt seine Textur nicht selbst. Der Materialabschnitt hat zwei Wörterbücher „Textur → Materialliste“ und „Palette → Materialliste“; ihre Namen (16 Byte) werden beim Laden in den Wörterbüchern des TEX0 gesucht (`Easy3D_BindTextureToResource` → `NNS_G3dBindMdlSet`, `src/easy3d.c:106`). Ein Material hat seine Breite und Höhe selbst (`origWidth/Height`); bei 32 Platin-Materialien weicht sie von der Textur im Paket ab, bei HeartGold bei keinem.
- **Zuordnung geprüft** (Ergebnis wie `task0-3b4.md` §6): Platin 632 Karten mit Header, davon 618 mit allen Namen im Paket, 14 mit fehlenden, 8 mit mehreren Paketen, 34 ohne Header; HeartGold 646, 639, 7, 2, 30.
- **Eine Textur je Material**: Ein Texturname gehört in 98,6 % der Fälle genau einem Material (Platin 6.961 von 7.060, HeartGold 10.377 von 10.529). Ein gebundenes Material wird in 98,6 % bzw. 99,98 % genau einmal gezeichnet (Platin 7.242 von 7.346, HeartGold 10.814 von 10.816). HeartGold hat zusätzlich 2.431 Materialien, die kein SBC-Befehl bindet (Platin 0); der Codec muss sie für den Round-Trip halten. Eine Karte nutzt im Median 22 % (Platin) bzw. 36 % (HeartGold) der Texturen ihres Pakets.

**Kartentexturpakete:**

| | Platin (74 Pakete) | HeartGold (106) |
|---|---|---|
| Dateigröße min / Median / max | 652 / 17.500 / 79.964 | 708 / 14.336 / 58.704 |
| Texturen je Paket Median / max (gesamt) | 38 / 94 (3.130) | 30 / 126 (3.659) |
| Paletten je Paket Median / max (gesamt) | 38,5 / 90 (3.000) | 28,5 / 126 (3.628) |
| Format 16 Farben / 4 Farben / A3I5 / A5I3 / 256 Farben | 2.269 / 599 / 198 / 40 / 24 | 2.662 / 871 / 27 / 96 / 3 |
| 4×4-komprimiert / direkt (16 Bit) | 0 / 0 | 0 / 0 |
| von Kartenmodellen genutzt: 16 / 4 Farben / A3I5 / A5I3 / 256 | 5.455 / 865 / 279 / 51 / 56 | 7.988 / 1.988 / 77 / 183 / 3 |
| Index 0 durchsichtig (`color0`) | 970 | 1.183 |
| Größen (häufigste) | 16×16 1.286, 32×32 1.002, 64×64 359, 8×8 180; bis 128×128 | 32×32 1.711, 16×16 1.307, 64×64 234, 8×8 205; bis 128×128 |
| Wiederholen/Spiegeln, texgen im Paket | immer 0 (steht im Material) | immer 0 |
| Texturen mit geteilten Texeldaten | 216 | 175 |
| Paletten mit geteiltem Offset | 299 | 216 |
| Texelblock in Wörterbuchreihenfolge | 37 von 74 | 65 von 106 |
| Lücken im Texelblock | 0 | 0 |
| Texel + Paletten je Paket (VRAM) Median / max | 16.120 / 76.416 | 12.520 / 57.136 |
| Paletten je Paket max | 1.936 | 2.784 |
| Karten- + Gebäudepaket je Gebiet Median / max | 37.744 / **110.640** | 27.144 / **87.424** |

Feste Felder, die ein Writer setzen muss (alle Vanilla-Pakete): zweites Wort des Texturdatums = `Breite | Höhe << 11 | 1 << 31`; zweites Halbwort des Palettendatums = 1 genau für Paletten von 4-Farben-Texturen (Platin 598, HeartGold 871), sonst 0; Palettenflag im TEX0-Kopf (+0x32) = `0x8000` genau dann, wenn eine 4-Farben-Textur im Paket ist (66 von 74 bzw. 99 von 106); VRAM-Schlüssel 0; 4×4-Felder leer, ihr Info-Offset gleich dem Texturen-Info-Offset.

**VRAM.** Das Feld schaltet `GX_VRAM_TEX_01_AB` (256 KiB Texturen) und `GX_VRAM_TEXPLTT_0123_E` (64 KiB Paletten) (Platin `fieldmap.c:527`, HeartGold `field/fieldmap.c:450`); der Verwalter von `Easy3D_Init` hat 256 KiB / 64 KiB. Karten- und Gebäudepaket werden beim Betreten des Gebiets hochgeladen, die Texel danach im RAM freigegeben (`AreaData_StripTextureData`). Wie viel die übrigen Nutzer (Figuren, Effekte) belegen, ist nicht gemessen; Vanilla bleibt bei höchstens 110.640 Byte für beide Pakete.

### 2.5 Was das Spiel zwingend braucht und was es trägt

Aus pret/pokeplatinum (`src/overlay005/land_data.c`, `easy3d.c`, `area_data.c`, `area_light.c`), bestätigt durch die Spielprobe (§2.7):

| Teil | zwingend | Beleg |
|---|---|---|
| Container `BMD0`, ein Block `MDL0` | ja | `NNS_G3dGetMdlSet` auf der Datei |
| genau ein Modell im Wörterbuch | ja | `GF_ASSERT(mapModelSet->dict.numEntry == 1)` (`land_data.c:1935`) |
| Modell 0 über Index | ja | `NNS_G3dGetMdlByIdx(…, 0)` |
| Datei ≤ 0xF000 | ja | `MAP_MODEL_FILE_SIZE` |
| Knochen, SBC | ja (Zeichnen ist `NNS_G3dDraw` auf dem RenderObj) | `Easy3D_DrawRenderObj` |
| Material mit Textur- und Palettenpaarung | ja, sonst untexturiert | `NNS_G3dBindMdlSet` per Name, Rückgabe wird ignoriert |
| richtige Patricia-Bäume in den Wörterbüchern | ja für Namenssuche | apicula liest Namen linear und prüft den Baum nicht; das Spiel fand die Texturen unserer Dateien (§2.7) |
| Zähler, Rahmen (Box) im Modellkopf | nein (HG hat Vanilla-Modelle mit veralteten Zählern) | §2.2 |
| Normale | nur bei Licht an | Außen-Gebiete ersetzen Materialfarben durch das Tageslicht |

**Vereinfachungen, die das Spiel trägt** (im Spiel gezeigt, §2.7): ein Modell mit einem Knochen ohne Transformation, SBC im Grundmuster, ein Material und ein Shape je Textur, nur `BEGIN_VTXS` Quads oder Quad-Strips, `VTX_16` oder `VTX_10`, eine Farbe (`COLOR`, innen) bzw. eine Normale (außen) je Shape, `TEXCOORD` je Ecke. Licht aus mit Eckenfarbe ist die Innen-Variante von Vanilla; außen nutzt Vanilla zu 100 % Licht 0 mit Normalen, damit die Tageszeit wirkt (eine unbeleuchtete Außenkarte würde nach dieser Logik nicht mit der Tageszeit dunkler, geschätzt, nicht im Spiel geprüft).

**Was die Vereinfachung kostet** (gemessen mit `gutachten-a-minimal.py size`, flaches 32×32-Feld, `posScale` 64):

| Kodierung | eine Textur | vier Texturen (2×2-Schachbrett) |
|---|---|---|
| Quads, `VTX_16` | 57.776 B (94 % von 0xF000) | 58.296 B (95 %) |
| Quads, `VTX_10` | 41.392 B (67 %) | 41.912 B (68 %) |
| Quad-Strips je Lauf, `VTX_10` | 21.740 B (35 %) | 34.728 B (57 %) |

„Nur Quads mit `VTX_16`“ reicht also schon für ein flaches Feld kaum; mit Wänden und Höhen läuft es über die Grenze. Der Bäcker braucht von Anfang an Quad-Strips über Läufe gleicher Textur und Höhe und `VTX_10`, wo die Ecken auf dem 1/64-Raster liegen (bei `posScale` 64 sind das ganze Welteinheiten, eine Kachel sind 16). `posScale` 32 reicht nicht: die Kartenkante +256 liegt dann bei 8,0 und passt nicht in `fx16` (gemessen, Abbruch beim Kodieren); darum nutzen volle Außenkarten in Vanilla 64.

### 2.6 Round-Trip: Leser → Struktur → Writer (gemessen)

| Prüfung | Platin | HeartGold DE | HeartGold US |
|---|---|---|---|
| Wörterbücher der Modelle aus (Daten, Namen) neu geschrieben, byte-gleich | 3.996 / 3.996 | 4.056 / 4.056 | 4.056 / 4.056 |
| Wörterbücher der Texturpakete (Karten + Gebäude, Rust-Writer) | 290 / 290 | 400 / 400 | 400 / 400 |
| **Layout-Writer**: Modell aus der Struktur, alle Offsets neu berechnet, byte-gleich | **666 / 666** | **676 / 676** | **676 / 676** |
| NSBTX im Vanilla-Layout neu geschrieben (Texel- und Palettenblock als Ganzes), byte-gleich | 74 + 71 / 145 | 106 + 94 / 200 | 106 + 94 / 200 |

(10 Members des HeartGold-Gebäudetexturordners sind keine BTX0 und zählen nicht.)

Der Layout-Writer kodiert Wörterbücher (mit Patricia-Baum), SBC, Materialfelder, Paarungen, Shape-Köpfe und die GPU-Befehle aus der gelesenen Struktur neu und berechnet alle Offsets; als Bytes übernimmt er nur die Knochendaten (je Knochen, Länge aus den Flags), die Texturmatrix hinter Byte 44 eines Materials, die festen Felder des Modellkopfs und den (überall leeren) EVP-Rest. Dabei fielen zwei Lücken im Wissen auf, beide geschlossen:
- **Patricia-Baum**: Knoten 0 ist die Wurzel (refBit 127). Einfügen in Eintragsreihenfolge nach Sedgewick (höchstes unterscheidendes Bit des 16-Byte-Namens als Folge von vier `u32` LE), danach werden die Knoten in Vorordnung über die Abwärtskanten nummeriert (links vor rechts). Mit dieser Regel sind alle 13.198 Vanilla-Wörterbücher byte-gleich. Ohne die Umnummerierung stimmte nur ein Wörterbuch mit einem Eintrag.
- **Knochen mit Skalierung** tragen 24 Byte (Skalierung und Kehrwert), nicht 12, wie apicula liest. Mit 12 Byte waren 5 Platin-Modelle 12 bis 120 Byte zu kurz.

Ein voller Leser/Writer für alle Vanilla-Kartenmodelle ist damit kein großes Risiko mehr, sondern gemessen: Leser und Writer sind in `gutachten-a-nitro.py` rund 700 Zeilen Python (die Datei hat mit Statistik und Zuordnung 1.330).

### 2.7 Prototyp im Spiel (gemessen, mit Bild)

Bilder unter `target/agent/gutachten-6a/` (nicht im Repo; sie zeigen Spielgrafik):

| Probe | Datei | Ergebnis |
|---|---|---|
| Ein Quad, eine Textur | `quad.nsbmd` 488 B (Scratchpad) | apicula liest und wandelt (1 GLB) |
| Platin, Spielerzimmer (Header 415, Karte 187): Modell ersetzt durch Schachbrett aus zwei Texturen des Gebietspakets (80 Quads) und einen Block 2×2×1 mit Wandtextur (8 Quads), innen (Licht aus, Eckenfarbe) | `pt-baked/baked.png`, Vergleich `pt-vanilla/vanilla.png` | Modell 2.572 → 5.708 B; geladen, texturiert, Block mit allen Seiten sichtbar, Figur läuft (BDHC und Kollision unverändert) |
| dasselbe mit Licht 0 und Normale (Außen-Variante) | `pt-baked-licht/baked-licht.png` | geladen, gleich hell (das Zimmer ist kein Außen-Gebiet) |
| HeartGold DE, Zimmer (Header 64, Karte 217), BGS-Block erhalten | `hg-baked/baked.png`, Vergleich `hg-vanilla/vanilla.png` | Modell 6.576 → 7.948 B; geladen, texturiert |
| Platin: eigene Texturen (synthetisch, vier PNG-artige Bilder aus dem Rust-Prototyp) an das Gebietspaket 0020 angehängt (45 + 4 Texturen, 8.108 → 11.124 B), Zimmer mit diesen Texturen, Block mit `color0` | `pt-eigene-texturen/eigene-texturen.png`, Ausschnitt `block-zoom.png` | eigene Texturen sichtbar; Index 0 des Zauns durchsichtig (Boden scheint durch) |
| Erdgeschoss (Header 414, Karte 186, gleiches Paket 20, Vanilla-Modell) | `pt-eigene-texturen/unten.png` | alle Vanilla-Texturen weiter gebunden: die Namenssuche im neu geschriebenen 49er-Wörterbuch funktioniert |
| Quad-Strips mit `VTX_10` (Läufe von 2 Kacheln, zwei Texturen) | `pt-streifen/streifen.png` | 3.236 B; geladen, richtig orientiert |

Gegenprobe mit apicula für jedes Modell: Beenden mit 0, eine GLB, die Eckenmenge der GLB ist gleich der Eingabe des Generators (108 von 108 Ecken beim Zimmer mit Block). Die Drehrichtung (Vorderseite = `(v1−v0)×(v2−v0)` zeigt nach außen) stimmt mit allen waagerechten Vanilla-Quads überein. HeartGold US ist im Spiel nicht geprüft (Modelle bis auf vier Karten gleich DE).

### 2.8 NSBTX-Writer (Prototyp, gemessen)

`gutachten-a-proto synth`: vier synthetische Bilder (32×32 viele Farben, 16×16 mit durchsichtigem Rand, 8×8 vier Farben, 64×64 Verlauf), Quantisierung auf 16 bzw. 4 Farben in BGR555, Index 0 durchsichtig bei Alpha < 128 (`color0`), Paletten auf 8 Byte ausgerichtet, Layout wie Vanilla.

| | eigener Median-Cut | `color_quant` (NeuQuant) |
|---|---|---|
| mittlerer Fehler je Kanal (0–255) der vier Bilder | 7,6 / 3,2 / 1,9 / 12,7 | 13,7 / 5,4 / 12,8 / 17,0 |
| Rücklesen mit `g4_formats::nsbtx`, Pixel falsch | 0 | 0 |
| Dekodierung durch apicula gegen `g4-formats`, Pixel falsch (je Bild) | 0 / 0 / 0 / 0 | – |
| Datei | 3.128 B | 3.128 B |

Was PNG → NSBTX braucht:
- Seiten als Zweierpotenz 8 bis 1024 (Karten nutzen bis 128);
- Format 3 (16 Farben) als Standard, Format 2 (4 Farben) für einfache Kacheln, später A3I5/A5I3 für Wasser und Glas (Vanilla 238 bzw. 123 Texturen);
- Quantisierung in BGR555;
- Index 0 als Transparenz mit `color0`;
- Palettenname je Textur (Vanilla: Palettenname gleich einem Texturnamen bei 1.837 von 3.000 bzw. 546 von 3.628 Paletten; die übrigen haben eigene Namen, und in den Modellen gehört ein Palettenname oft mehreren Materialien: Platin 351, HeartGold 240 Paarungen mit mehr als einem Material);
- das Palettenflag 0x8000 und die 4-Farben-Markierung (§2.4);
- Anhängen an ein bestehendes Paket: Texel- und Palettenblock hinten anfügen, Offsets der neuen Einträge verschieben, Wörterbücher neu (gezeigt mit `gutachten-a-minimal.py merge`).

Byte-gleich aus dekodierten Bildern wäre Vanilla nicht zu erzeugen (geteilte Texel, Reihenfolge des Texelblocks nicht nach Wörterbuch, §2.4); das ist auch nicht nötig, weil unveränderte Pakete verbatim bleiben und Anhängen die alten Blöcke nicht anfasst.

### 2.9 Lizenz-Audit

| Quelle | Lizenz | Nutzung hier |
|---|---|---|
| apicula (`scurest/apicula`, `3d4e91e`) | 0BSD | Layout-Referenz (`nitro/model.rs`, `render_cmds.rs`, `tex.rs`, `info_block.rs`, `nds/gpu_cmds.rs`) und Orakel (GLB, PNG); schon Werkzeug des Projekts |
| pret/pokeplatinum (`80c83a6`), pret/pokeheartgold (`e1d5a46`) | keine Lizenzdatei im Repo | nur Formatwissen (Asserts, Bankbelegung, Aufrufe), kein Code; die NitroSystem-Bibliothek liegt dort nur als Subprojekt-Verweis und wurde nicht geöffnet |
| GBATEK (Martin Korth) | Dokumentation | Bitfelder `POLYGON_ATTR`, `TEXIMAGE_PARAM`, GPU-Befehle und Parameterzahlen |
| eigene Messungen und Prototypen | MIT (dieses Repo) | Patricia-Baum, Layout, feste Felder |
| `romkit` 0.1.1 (crates.io, `platinvm/romkit`) | MIT OR Apache-2.0 | geprüft: liest NSBMD/NSBTX und rendert glTF; schreibt nicht (nur Test-Bauer mit leerem Baum `0x17F`, für das Spiel ungültig); 31 Downloads. Kein Kandidat |
| andere Crates zu `nsbmd`, `nsbtx`, `nitro`, `btx0`, `bmd0`, `nds texture`, `nintendo ds`, `g3d` | – | Suche auf crates.io am 05.11.2026: kein Crate schreibt NSBMD oder NSBTX (`ds-rom` MIT und `nds` MIT sind ROM-Container) |
| NitroPaint (`Garhoogin/NitroPaint`, C) | BSD-2-Clause | laut Projektbeschreibung Textur-Editor für DS; nicht geöffnet. Lizenz verträglich, als Gegenprobe von Hand denkbar, nicht als Abhängigkeit |
| Tinke (`pleonex/tinke`) | GPL-3.0 | nicht nutzbar |
| Pokémon DS Map Studio (`Trifindo/Pokemon-DS-Map-Studio`) | keine Lizenz angegeben (GitHub-API) | nicht genutzt, siehe Nebenbefund §8 |
| DSPRE | AGPL-3.0 | nicht geöffnet |
| NITRO-SDK (`g3dcvtr` u. a.) | proprietär | nicht genutzt |
| `image` 0.25, `png` 0.18 | MIT OR Apache-2.0 | schon im Projekt; PNG lesen |
| `color_quant` 1.1 / 2.0 | MIT | NeuQuant; gemessen schlechter als Median-Cut bei 16 Farben |
| `quantette` 0.6 | MIT OR Apache-2.0 | Wu und k-Means; Kandidat, nicht gemessen |
| `kmeans_colors` 0.7 | MIT OR Apache-2.0 | Kandidat, nicht gemessen |
| `exoquant` 0.2 | MIT | seit 2016 unverändert |
| `imagequant` 4.4 | GPL-3.0-or-later | **nicht erlaubt** (`deny.toml`) |

Ergebnis: Es gibt keinen lizenzverträglichen fertigen Writer. Eigenbau ist nötig. Die Quantisierung braucht keine fremde Bibliothek: ein Median-Cut in BGR555 sind rund 80 Zeilen und war in der Messung besser als NeuQuant.

---

## 3. Optionen

### (a) Voller Writer mit byte-gleichem Round-Trip als Produktpfad

Leser → eigene Struktur → Writer für jedes Modell. Jede Karte, die das Studio speichert, geht durch die Struktur, auch unveränderte.
- **Nutzen:** Vanilla-Modelle lassen sich gezielt ändern (Textur eines Materials tauschen, ein Shape löschen, gemalte Kacheln zu Vanilla-Geometrie hinzufügen); eine Struktur für Vorschau, Prüfungen und Export.
- **Aufwand:** Leser und Writer für alle genutzten Teile (mehrere Knochen, `0x46`/`0x66`, Texturmatrix, alle Vertex-Befehle). Gemessen: in Python rund 700 Zeilen; in Rust mit Tests geschätzt 1.500 bis 2.000 Zeilen.
- **Risiko:** gering bis mittel. Byte-Gleichheit ist für alle 2.018 Modelle gezeigt; der Produktpfad schreibt aber auch dort neu, wo nichts zu ändern war, und jeder Kodierfehler träfe alle Karten.
- **Test-Orakel:** Byte-Gleichheit auf Vanilla (stark für den Codec), dazu apicula und das Spiel für Änderungen.

### (b) Minimaler Writer nur für gebackene Karten

Ein Writer, der nur die Teilmenge aus §2.5 erzeugt. Unveränderte Modelle bleiben verbatim (so wie heute).
- **Nutzen:** kleinster Code, schnell im Spiel.
- **Aufwand:** klein (Prototyp: rund 450 Zeilen Python für Modell, Strips, `VTX_10`, Paket anhängen).
- **Risiko:** mittel. Ohne Leser gibt es kein Orakel für den Codec außer apicula und dem Spiel; apicula prüft weder den Patricia-Baum noch Bitfelder des Materials. Malen auf einer Vanilla-Karte (Geometrie behalten, Kacheln ergänzen) geht nicht.
- **Test-Orakel:** apicula (liest, GLB-Geometrie gleich der Eingabe), Spiel (g4emu-Bild), eigene Plausibilität.

### (c) Ein Codec mit Round-Trip als Prüfung, Bäcker auf der Teilmenge, Unverändertes bleibt verbatim (recommend)

Eine strukturierte Darstellung von MDL0 (Modellkopf, Knochen, SBC, Materialien mit Paarung, Shapes mit dekodierter Display List) mit Leser und Writer im Formats-Crate. Der Round-Trip ist **Prüfung** (gated Test auf allen Vanilla-Modellen), nicht Produktpfad: Karten, die niemand bemalt, bleiben Bytes wie heute. Der Bäcker erzeugt dieselbe Struktur mit der Teilmenge aus §2.5 und gibt sie demselben Writer.
- **Nutzen:** der Writer, der gebackene Karten schreibt, ist auf 2.018 Vanilla-Modellen byte-genau geprüft; Wörterbücher und Baum ebenso (13.198). Später kann Malen auf Vanilla-Karten dieselbe Struktur nutzen (lesen, Shapes ergänzen, schreiben), ohne neuen Codec.
- **Aufwand:** (b) plus Leser, rund 40 % mehr Code als (b) (geschätzt), aber im Prototyp schon gezeigt.
- **Risiko:** gering. Byte-Gleichheit schützt den Codec, apicula und das Spiel prüfen den Bäcker, unveränderte Karten berührt nichts.
- **Test-Orakel je Ebene:**
  1. Codec: Byte-Gleichheit Leser → Writer auf allen Vanilla-Modellen und Texturpaketen der drei Spiele.
  2. Wörterbuch: alle Vanilla-Wörterbücher aus (Daten, Namen) byte-gleich.
  3. Bäcker: eigener Leser liest; Zähler im Modellkopf gleich der Display List; apicula beendet mit 0 und schreibt genau eine GLB, deren Ecken- und UV-Menge der Eingabe gleicht; Größe ≤ 0xF000.
  4. Spiel: g4emu-Bild eines gebackenen Raums je Spiel, dazu ein Nachbarraum mit demselben Paket (Namenssuche).
  5. NSBTX: Rücklesen mit `g4-formats` gleich der Quantisierung (0 Pixel), apicula gleich `g4-formats` (0 Pixel).

| | (a) voller Writer als Pfad | (b) minimal | (c) Codec + Bäcker |
|---|---|---|---|
| unveränderte Karten | neu geschrieben | verbatim | verbatim |
| Orakel für den Codec | Vanilla byte-gleich | nur apicula und Spiel | Vanilla byte-gleich |
| Patricia-Baum geprüft | ja | nur im Spiel | ja |
| Malen auf Vanilla-Karten später | ja | nein | ja |
| Aufwand (geschätzt) | M | S | S bis M |
| Risiko | gering bis mittel | mittel | gering |

---

## 4. Empfehlung

1. **Strategie (Frage 3): recommend (c).** Ein MDL0-Codec (Leser und Writer) in `g4-formats` mit byte-gleichem Round-Trip auf allen Vanilla-Kartenmodellen als gated Test; der Bäcker erzeugt die minimale Teilmenge und schreibt über denselben Writer; unveränderte Modelle bleiben verbatim. Kein Umweg über ein Zwischenformat wie IMD und kein NITRO-SDK.
2. **Teilmenge für gebackene Kacheln (Frage 2):**
   - ein Modell, ein Knochen `0xF807`, SBC `26 02 0b (04 05)* 2b 01`;
   - ein Material und ein Shape je Textur, Material-Werte aus Vanilla (`poly_attr_mask` `0x3F1FF8FF`, Wiederholen S/T, `flag` `0x1FCE`, `mag` 1,0);
   - Stil je Gebiet: **außen** Licht 0, Nebel, `dif_amb` `0x7FFFE739`, eine Normale je Ebene; **innen** Licht aus, `dif_amb` `0x7FFF8000`, `COLOR` weiß. Die Regel ist Platin Lichtarchiv 0 oder 3 = außen (pret), HeartGold Lichttyp 1 = außen (gemessen);
   - Quad-Strips über Läufe gleicher Textur und Ebene, Quads für den Rest, `VTX_10` auf dem Raster, sonst `VTX_16`; `posScale` 64 für volle Karten;
   - `TEXCOORD` nur bei Wechsel, Zähler und Rahmen berechnet.
   Dreiecke, mehrere Knochen, Texturmatrix und Alpha < 31 braucht der Bäcker nicht; der Codec liest und schreibt sie trotzdem (Round-Trip).
3. **NSBTX (Frage 4):**
   - Writer im Vanilla-Layout mit Patricia-Baum;
   - „Textur an Paket anhängen“ als Grundoperation (Gebietspakete bleiben sonst verbatim);
   - Konverter RGBA → Format 3/2 mit eigenem Median-Cut in BGR555 und `color0`;
   - Prüfungen: Zweierpotenz, Name ≤ 16 Byte und eindeutig, Größe des Pakets und VRAM gegen das Vanilla-Maximum (Warnung über 110.640 bzw. 87.424 Byte für Karte + Gebäude, harte Grenze offen).
   A3I5/A5I3 und 256 Farben später; 4×4-komprimiert und direkt nicht (Vanilla-Karten nutzen sie nicht).
4. **Crates (Frage 5):** keine neue Abhängigkeit nötig. `image`/`png` sind da; Quantisierung selbst (MIT). Falls doch eine Bibliothek: `quantette` (MIT OR Apache-2.0) vor `color_quant`; `imagequant` (GPL) nie.
5. **Schnitt (Frage 6):** Teilphase 6.1 „Writer“ wie in §5, vor dem Malen (Gutachten C) und parallel zum BDHC-Bau (Gutachten B).

---

## 5. Schnitt der Teilphase 6.1 „Writer“

**Inhalt.**
- **`g4-formats`:**
  - `nitro_dict` (Wörterbuch lesen und schreiben, Patricia-Baum);
  - `nsbmd` (MDL0-Struktur, Leser, Writer, GPU-Display-List-Codec, SBC-Codec; `nsbmd_header` geht darin auf oder bleibt als dünne Hülle);
  - `nsbtx::write` und `nsbtx::append`;
  - `texconv` (RGBA → Palettentextur);
  - `docs/formats/nsbmd.md` (vollständig, mit Patricia-Baum und Knochen-Layout) und Abschnitt „Schreiben“ in `nsbtx.md`.
- **`g4-kernel`:**
  - `bake` mit neutraler Eingabe: eine Liste von Flächen (vier Ecken, UV in Texeln, Texturschlüssel, Ebene) und dem Stil des Gebiets. Daraus Materialien, Strips, Vertex-Kodierung, Normale oder Farbe, `posScale`, Rahmen, Zähler;
  - Bericht mit Größe und Anteil an 0xF000, Texturen und VRAM;
  - Prüfcodes `bake.model-too-big`, `bake.texture-missing`, `bake.vram-high` (Namen vorläufig).
  Wie Kacheln zu Flächen werden (Höhen, Wände, Rampen), gehört zu 6.2 (Gutachten C); 6.1 liefert eine Testfläche (Schachbrett plus Block wie §2.7).
- **Werkzeug:** `g4k bake-test <projekt> <karte> [--texture png…]` oder ein MCP-Befehl, der die Testfläche in eine Karte schreibt und Texturen anhängt (für das Gate, später ersetzt durch Malen).

**Arbeitspakete (Größen geschätzt).**

| AP | Inhalt | Größe |
|---|---|---|
| 1 | Wörterbuch mit Patricia-Baum, Unit-Tests (Einfügen, Nummerierung, 1 bis 255 Einträge, doppelte Namen abgelehnt) | S |
| 2 | MDL0-Codec: Modellkopf, Knochen (Längen aus Flags, Skalierung 24 Byte), SBC, Materialien (44/48/52 Byte), Paarungen, Shapes, GPU-Befehle; gated Round-Trip Pt/HG/US | M |
| 3 | NSBTX-Writer und Anhängen, gated Round-Trip der Karten- und Gebäudepakete, feste Felder aus §2.4 | S |
| 4 | Texturkonverter (Median-Cut BGR555, 16 und 4 Farben, `color0`), Prüfungen | S |
| 5 | Bäcker: Flächen → Struktur (Strips über Läufe, `VTX_10`/`VTX_16`, Stil je Gebiet und Familie, Rahmen, Zähler), Budget-Bericht | M |
| 6 | Orakel-Tests (apicula-GLB gegen Eingabe, g4emu-Szenario je Spiel), Doku | S bis M |

Zusammen geschätzt 2.500 bis 3.500 Zeilen Rust mit Tests, Größe M.

**Gate 6.1 (messbar).**
1. Gated Round-Trip auf frischen Kopien der drei Spiele: Leser → Writer byte-gleich für alle Kartenmodelle (666, 676, 676) und alle BTX0 der Karten- und Gebäudetexturen (145, 200, 200); alle Wörterbücher dabei byte-gleich.
2. Unit: der Bäcker schreibt die Testfläche (32×32, vier Texturen 2×2, Block); Größe ≤ 40.000 Byte (gemessen 34.728 ohne Block); der eigene Leser liest sie; Zähler im Kopf gleich der Display List; ein Modell über 0xF000 wird mit `bake.model-too-big` abgelehnt.
3. apicula: für die Testfläche Exit 0, genau eine GLB, Ecken- und UV-Menge der GLB gleich der Eingabe (Test mit Werkzeug, wie die Export-Tests).
4. NSBTX: vier synthetische Bilder → Paket; Rücklesen mit `g4-formats` 0 Pixel Abweichung gegen die Quantisierung; apicula-PNG gleich `g4-formats` (0 Pixel); Anhängen an ein Vanilla-Paket lässt die alten Einträge byte-gleich (Texel, Paletten, Wörterbuchdaten).
5. Spiel (g4emu, Agent + Blick, auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US):
   - Testfläche mit zwei eigenen Texturen in das Zimmer schreiben (Platin Karte 187, HeartGold Karte 217), bauen, `continue`, `expect-map 415` bzw. `64`, `shot`: eigene Texturen sichtbar, der Block mit allen Seiten;
   - danach in den Nachbarraum mit demselben Paket (Platin `walk-to 8 4`, `walk right 1`, `expect-map 414`, Karte 186, Area-Data und Paket 20; HeartGold `walk left 1`, `walk down 1`, `expect-map 63`, Karte 216, Area-Data und Paket 25; Zuordnung gemessen), `shot`: Vanilla-Texturen unverändert;
   - `check:` mit `${sha1-of:…}` belegt, dass `out.nds` Modell und Paket enthält.
6. `cargo deny` grün, keine neue GPL-Abhängigkeit; `scripts/gate.sh` grün.

**Voraussetzungen und Nachbarn.** 6.1 braucht nichts aus B und C. Gutachten B liefert den BDHC-Bau und die Engine-Grenzen (VRAM-Rest, Kamera); Gutachten C das Malen, die Kachel-Bibliothek und den Tiled-Import, die die Flächen für `bake` erzeugen.

---

## 6. Risiken

- **Patricia-Baum:** Ein falscher Baum fällt in apicula und three.js nicht auf (apicula sucht linear), im Spiel aber als untexturierte Fläche. Gegenmittel: gated Test aller 13.198 Vanilla-Wörterbücher und das Gate mit Nachbarraum (§5 Punkt 5).
- **Größe 0xF000:** Schon ein flaches Feld braucht Strips und `VTX_10` (§2.5). Mit Höhen und Wänden wird es eng. Vanilla hat 12 bzw. 38 Modelle über 90 %. Der Bericht zeigt den Anteil schon beim Malen; ein Bäcker ohne Strips wäre nicht brauchbar.
- **`TEXCOORD`-Bereich:** 1.11.4 mit Vorzeichen, also ±2.048 Texel. Ein Strip über 32 Kacheln mit einer 64-Texel-Textur erreicht 2.048 und läuft über. Der Bäcker muss Strips teilen oder die Koordinaten zurücksetzen (bei 16-Texel-Texturen bleibt es bei 512).
- **`VTX_10`-Raster:** 1/64 Modelleinheit, bei `posScale` 64 eine Welteinheit. Höhen aus dem Malen, die nicht darauf liegen, brauchen `VTX_16` (größer).
- **Licht:** außen ersetzt das Spiel die Materialfarben durch die Tageszeit; ohne Normale wird eine beleuchtete Fläche falsch hell. Die Außen-Variante ist nur in einem Innenraum geprüft, nicht in einem echten Außen-Gebiet und nicht über Tag und Nacht. Für HeartGold ist die Regel „Lichttyp 1 = außen“ nur gemessen.
- **VRAM:** Angehängte Texturen vergrößern das Gebietspaket. Eine harte Grenze ist nicht ermittelt; Vanilla kommt auf höchstens 110.640 Byte für Karte und Gebäude bei 256 KiB Bank. Passt ein Paket nicht, gibt `Easy3D_UploadTextureToVRAM` FALSE und der Lader endet in `GF_ASSERT(textureUploaded)` (`area_data.c:81` und `:87`); was das Verkaufs-ROM dann tut (Halt oder weiter ohne Texturen), ist nicht geprüft.
- **Geteilte Pakete:** Mehrere Gebiete nutzen dasselbe Paket. Anhängen ist für sie unschädlich (alte Einträge unverändert); Löschen oder Ersetzen einer Textur trifft alle. 6.1 bietet darum nur Anhängen.
- **HGSS-Besonderheiten:** Texturmatrix-Materialien (HeartGold 420 Dateien) und dynamische Texturen (`dynamic_texture_type` der Area-Data) sind im Codec nur als Bytes erhalten. Für den Bäcker spielen sie keine Rolle, für späteres Bearbeiten von Vanilla-Modellen schon.
- **Orakel apicula:** prüft Geometrie, UV und Texturdekodierung, nicht Material-Bitfelder, Licht oder Baum. Das Spiel bleibt das letzte Orakel; die Bilder prüft der Agent mit Blick, nicht ein Pixelvergleich (siehe Nebenbefund g4emu).

---

## 7. Offene Punkte für die Spezifikation

1. **Eingabe des Bäckers:** Flächenliste (Empfehlung, neutral) oder direkt das Kachelraster aus Gutachten C? Davon hängt ab, wo Wände, Rampen und Übergänge entstehen.
2. **Stilregel HeartGold:** „Lichttyp 1 = außen“ in pokeheartgold nachlesen (Assembler des Laders) oder aus Area-Data übernehmen wie gemessen?
3. **Texturen neu gegen anhängen:** Gehört eine gemalte Karte immer zum Paket ihres Gebiets (anhängen), oder darf der Assistent ein eigenes Paket und eine eigene Area-Data anlegen (dann auch Gebäudepaket und Licht wählen)?
4. **VRAM-Grenze:** harte Zahl aus Gutachten B übernehmen oder bis dahin nur warnen über dem Vanilla-Maximum?
5. **Quantisierung:** eigener Median-Cut (Empfehlung) oder `quantette`; Dithering ja oder nein (Pixel-Art-Kacheln vertragen es meist nicht)?
6. **Palette teilen:** eine Palette je Textur (einfach) oder gemeinsame Paletten für Kachelfamilien (spart Palettenspeicher, Vanilla teilt bei 1.163 bzw. 3.082 Paletten den Namen nicht mit der Textur)?
7. **Name der Struktur im Code:** `nsbmd_header` erweitern oder durch `nsbmd` ersetzen; die heutigen Aufrufer (`map_import.rs`) brauchen nur Kopf, Zahl der Modelle und Namen.
8. **Vorschau:** Das Frontend zeigt Karten über apicula → GLB; gebackene Modelle gehen denselben Weg (gemessen: apicula liest sie). Ob die Mal-Vorschau ohne apicula direkt aus der Flächenliste in three.js zeichnet, entscheidet Gutachten C.

---

## 8. Nebenbefunde

- **g4emu vertauscht Rot und Blau in den Bildern.** Eine Textur mit der Farbe (148, 90, 41), von apicula und `g4-formats` gleich dekodiert, erscheint in `shot` als (44, 93, 150); der Vanilla-Holzboden des Spielerzimmers erscheint bläulich (gemessen an `pt-eigene-texturen/eigene-texturen.png` und `pt-vanilla/vanilla.png`). `scripts/g4emu/g4emu/emu.py:65` wandelt mit Rohmodus `BGRX`; offenbar liefert der Kern die Bytes schon in RGB-Reihenfolge. Bisherige Gates prüfen Bilder nur mit Blick und Formen, nicht mit Farben; ein Farbvergleich in Phase 6 bräuchte zuerst die Korrektur.
- **apicula liest Knochen mit Skalierung zu kurz** (12 statt 24 Byte). Für die Darstellung folgenlos (die Knochen stehen hintereinander im Wörterbuch, apicula springt über Offsets), für einen Writer nach apicula-Layout aber ein Fehler (§2.6).
- **Lizenzlage PDSMS:** `docs/formats/nsbmd_header.md` nennt `NsbmdUtils.java` von Pokémon DS Map Studio als Quelle; das Repo hat laut GitHub keine Lizenz. Es geht dort nur um eine Tatsache (MDL0 über die Blockoffsets suchen), die auch apicula belegt. Vorschlag: die Quelle in der Doku auf apicula umstellen.
- **Veraltete Zähler** in den HeartGold-Karten 0468, 0475, 0476 (US auch 0473): Ecken- und Polygonzahl im Modellkopf größer als in den Display Lists. Ein Codec muss die Zähler als Felder halten (für den Round-Trip) und darf sie beim Backen neu berechnen.

---

## 9. Testkopien, Werkzeuge, Bilder

**Testkopien** (unter `Roms/_testkopien/`, angelegt mit `cp -R` aus den DSPRE-Ordnern, nichts gelöscht, `_saves` nur gelesen):
- `6-gutachten-a-pt`: gemessen unverändert; danach ersetzt `unpacked/maps/0187` (zuletzt die Strip-Probe) und `unpacked/mapTextures/0020` (Paket mit vier angehängten synthetischen Texturen); gebaute ROMs `out.nds`, `out-outdoor.nds`, `out-syn.nds`, `out-strip.nds`.
- `6-gutachten-a-hg`: gemessen unverändert; danach ersetzt `unpacked/maps/0217`; `out.nds`.
- `6-gutachten-a-hgus`: nur gemessen, unverändert.

**Werkzeuge:**
- `docs/research/2026-11-05-phase-6/gutachten-a-nitro.py`
- `docs/research/2026-11-05-phase-6/gutachten-a-minimal.py`
- `docs/research/2026-11-05-phase-6/gutachten-a-proto/` (`Cargo.toml`, `Cargo.lock`, `src/main.rs`)
- Klone von apicula, pokeplatinum, pokeheartgold und `romkit` im Scratchpad der Sitzung, nicht im Repo.

**Bilder** (`target/agent/gutachten-6a/`, nicht im Repo):
- `pt-vanilla/vanilla.png`, `pt-baked/baked.png`, `pt-baked-licht/baked-licht.png`
- `pt-eigene-texturen/eigene-texturen.png`, `block-zoom.png`, `unten.png`
- `pt-streifen/streifen.png`
- `hg-vanilla/vanilla.png`, `hg-baked/baked.png`
