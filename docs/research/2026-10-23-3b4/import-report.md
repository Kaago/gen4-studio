# Recherche 3b.4: Kartenimport (PDSMS, NSBMD, OBJ, Kollision, BDHC)

Stand: 23.10.2026. Für die Spec von Phase 3b.4 „Import und Prüfungen“, nur der Teil Kartenimport.

## Quellen und Stände

| Quelle | Stand | Ort |
|---|---|---|
| DSPRE `main` | `e751187b` (13.10.2026, Release 2.3.4) | lokaler Klon `/Users/POLTEP/Projekte/lab/editor/DSPRE` |
| DSPRE `feature/avalonia` | `0a550272` (21.10.2026) | derselbe Klon, Remote-Ref `origin/feature/avalonia`. **Nicht neu gefetcht**, kann also ein paar Tage alt sein. |
| PDSMS upstream (Trifindo) | `ac30b653` (13.07.2021) | `git clone --depth 1 https://github.com/Trifindo/Pokemon-DS-Map-Studio` → `scratchpad/research-3b4/src/pdsms` |
| PDSMS-Fork AdAstra-LD (v2.4) | `de409512` (24.09.2026) | `https://github.com/AdAstra-LD/Pokemon-DS-Map-Studio` → `scratchpad/research-3b4/src/pdsms-adastra` |
| apicula | `3d4e91e1` (03.08.2024) | `https://github.com/scurest/apicula` → `scratchpad/research-3b4/src/apicula` |
| pret/pokeplatinum | `main`, abgerufen am 23.10.2026 | `src/overlay005/land_data.c`, `loaded_map_buffers.c`, `bdhc.c`, `include/constants/field/bdhc.h`, `src/easy3d.c` |
| gen4-studio | Worktree `phase-3c1` | `crates/g4-formats/src/map.rs`, `docs/formats/map.md`, `docs/ROADMAP.md` |
| Testprojekte | nur gelesen | `/Users/POLTEP/Projekte/lab/editor/Roms/*_DSPRE_contents` (Statistik-Skript: `scratchpad/research-3b4/scripts/mapstats.py`) |

Pfade mit `avalonia:` beziehen sich auf `git show origin/feature/avalonia:<pfad>`, Pfade mit `main:` auf den Branch `main` von DSPRE.

## Kurzfazit

1. **`.pdsmap` ist ein Textformat, `.pdsts` ein binäres Tag-Format (Big Endian).** Beide sind weder JSON noch ZIP. Eine `.pdsmap` enthält nur Kachel- und Höhenraster je Karte. Kollision, Gebäude, BDHC und BGS liegen in eigenen Dateien daneben (`<stem>_XX_YY.per/.bld/.bdhc/.bdhcam/.bgs`). Die `.pdsts` verweist auf PNG-Texturen im selben Ordner. Eine Spec gibt es nicht, maßgeblich ist der Code (`MapMatrix.java`, `TilesetIO.java`). Der Fork AdAstra-LD hat das Format geändert (9 statt 8 Ebenen, Tags `exportgroup`/`egcenter`, Sidecar `.pdsts.meta`).
2. **PDSMS baut selbst kein NSBMD.** Es schreibt IMD und ruft Nintendos `g3dcvtr.exe` aus dem NITRO-SDK auf, den der Nutzer selbst beschaffen muss. DSPRE `feature/avalonia` backt `.pdsmap`/`.pdsts` mit einem eigenen NSBMD-Writer (`TileBake` → `NsbmdWriter`), Lizenz AGPL-3.0. gen4-studio steht unter MIT und darf daraus keinen Code übernehmen.
3. **Die Roadmap-Zeile „OBJ über apicula-Pfad“ stimmt so nicht.** apicula liest nur NSBMD und schreibt glTF oder COLLADA, nicht einmal OBJ. Einen quelloffenen OBJ→NSBMD-Weg außer DSPRE (AGPL) habe ich nicht gefunden.
4. **Ein Modell zu tauschen ist trivial und byte-sicher.** In `MapFile` wird nur `model` ersetzt, `write` rechnet die Längen im Kopf neu, alle anderen Abschnitte bleiben unverändert. Vorher sind Prüfungen nötig, die über DSPRE hinausgehen: Größe ≤ 0xF000, genau ein Modell im MDL0, TEX0 entfernen und Texturnamen gegen das NSBTX des Gebiets prüfen.
5. **Empfehlung:** 3b.4 sagt Import fertiger Binärteile zu (NSBMD, `.per`, `.bld`, `.bdhc`, `.bgs`, ganze Map-`.bin`, PDSMS-Projekt *ohne Baking*). Baking von `.pdsts`, OBJ→NSBMD, BDHC-Erzeugung und Kollisionsableitung bleiben in Phase 6, wo sie die Roadmap ohnehin schon hat (`docs/ROADMAP.md`, Zeilen 170–178).

---

## 1. PDSMS: `.pdsmap` und `.pdsts`

### 1.1 `.pdsmap` (Text, zeilenbasiert)

Writer upstream: `pdsms/src/main/java/editor/mapmatrix/MapMatrix.java`, `saveGridsToFile` (ab Zeile 105). Raster-Ausgabe: `editor/grid/MapGrid.java`, `printMatrixInFile` (Zeile 288).

Aufbau, jedes Tag auf einer eigenen Zeile, der Wert in der Zeile darunter:

```
gameindex
2                      ← 0 Diamond, 1 Pearl, 2 Platin, 3 HG, 4 SS, 5–8 BW/B2W2 (editor/game/Game.java:13–21)
tileset
<stem>.pdsts           ← immer der Stammname der .pdsmap (MapMatrix.java, saveGridsToFile)
mapstart
<x> <y>                ← Matrixposition relativ zur kleinsten Koordinate
areaindex
<n>
[exportgroup           ← nur im AdAstra-Fork
 <n>
 egcenter]             ← nur AdAstra v2.4, ohne Wertzeile
tilegrid               ← je Ebene einmal: 32 Zeilen mit je 32 ganzen Zahlen, -1 = leer
...
heightgrid             ← je Ebene einmal: 32 × 32 ganze Zahlen
...
mapend
(mapstart … mapend wiederholt sich für jede Karte)
```

- Anzahl der Ebenen: upstream 8 (`MapGrid.java:40 numLayers = 8`), im Fork 9 (`pdsms-adastra/.../MapGrid.java:40 numLayers = 9`). Laut Release-Notes des Forks (`pdsms-adastra/RELEASE_NOTES.md`, Abschnitt „Important thing to remember“) kann das upstream-PDSMS neu gespeicherte Karten nicht mehr öffnen.
- Leere Kachel: `-1` (`MapGrid.java:57`, `:158–164`).
- Ausrichtung: DSPRE liest die Zeilen „south to north“ und verankert mehrfeldrige Kacheln im Südwesten von PDSMS, im eigenen Raster aber im Nordwesten (`avalonia:DSPRE.Core/Models/PdsmapFile.cs`, Kommentare in `Read` und `ToGrid`). Am Code nachvollzogen, im Spiel nicht geprüft.
- **Kollision, Gebäude, BDHC und BGS stehen nicht in der `.pdsmap`.** Sie liegen in eigenen Dateien mit dem Muster `<stem>_XX_YY.<ext>` (`MapMatrix.getMapName`, Zeilen 622–627; Endungen: `Collisions.java:18 "per"`, `BuildFile.java:20 "bld"`, `Bdhc.java:11 "bdhc"`, `Bdhcam.java:9 "bdhcam"`, `Backsound.java:18 "bgs"`). Ältere Projekte mit nur einer Karte nutzen `<stem>.<ext>` ohne Koordinaten (`loadBDHCsFromFile`, Zweig „OLD MAP TYPE“).

DSPRE-Leser: `avalonia:DSPRE.Core/Models/PdsmapFile.cs` (130 Zeilen). Er liest bis zu 9 Ebenen, ignoriert unbekannte Tags und versteht `exportgroup`. `egcenter` fällt als unbekanntes Tag ohne Folgezeile einfach durch. Das ist am Code abgelesen, eine v2.4-Datei habe ich nicht getestet.

### 1.2 `.pdsts` (binär, Tag-Länge-Wert, Big Endian)

Writer: `pdsms/src/main/java/tileset/TilesetIO.java`, `saveTilesetToFile`. Jedes Element besteht aus `u8 tag`, `i32 count` (BE) und den Nutzdaten (`writeIntValue` nutzt `ByteBuffer.putInt`, also Big Endian, Zeilen 643–651). Bestätigt am Hexdump von `LowPolyTileset.pdsts` (`09 00000001 00000000 01 00000009 "Grass.png" …`).

- Materialien (`TAG_MATERIAL_START 9` … `TAG_MATERIAL_END 10`) tragen den PNG-Namen (`TAG_IMG_NAME 1`), die Textur- und Palettennamen für IMD und NSBTX (`2`, `3`) sowie Flags wie Nebel, beidseitig, Alpha, Lichter, Kachelung und Farbformat.
- Kacheln (`TAG_TILE 0` …) tragen Breite und Tiefe in Feldern, OBJ-artige Vertex-, UV-, Normalen- und Farbarrays (Tags 15, 16, 22, 49), Faces (17/18 bzw. erweitert 50/51), die Materialzuordnung (19, 24, 25), den OBJ-Dateinamen (21) und Offsets.
- Smart-Drawing-Raster (`26`).
- **Es sind keine Texturpixel enthalten.** Die PNGs liegen neben der `.pdsts`, wie man im Ordner `src/main/resources/tilesets/...` sieht.
- Der Fork legt zusätzlich `<tileset>.pdsts.meta` an (Textdatei, `# Pokemon DS Map Studio tile metadata v7`, Zeilen `tile|<idx>|…`) mit Kollisions-Vorgaben je Kachel. Der DSPRE-Leser und -Schreiber dafür steht in `avalonia:DSPRE.Core/Models/TileCollisions.cs` (`ReadMeta`, `WriteMeta`).

DSPRE-Leser: `avalonia:DSPRE.Core/Models/PdstsFile.cs` (427 Zeilen, dieselben Tag-Nummern; bei einem unbekannten Tag bricht er ab).

### 1.3 Die Seitendateien sind Rohabschnitte der Map-Datei

| Datei | Inhalt | Beleg | Passt zu gen4-studio |
|---|---|---|---|
| `.per` | 2048 Bytes, je Zelle `type, collision`, zeilenweise | `formats/collisions/Collisions.java` (`toByteArray`; Gen 4 hat 2 Ebenen, `CollisionTypes.numLayersPerGame = {2,2,2,2,2,8,8,8,8}`) | identisch mit `MapFile.permissions`, Index `y*32+x` |
| `.bld` | n × 48 Bytes | `editor/buildingeditor2/buildfile/BuildFile.java` | gleiches Layout. **PDSMS liest und schreibt die Modell-ID nur als u16 und setzt Rotation und Dummy-Bytes auf 0.** Bestätigt durch `pdsms-adastra/docs/V2_4_FORMAT_CAPABILITY_MATRIX.md` („discards X/Y/Z rotations and unknown bytes“). Das Spiel ignoriert die Rotation (`docs/formats/map.md`, `applyRotation = FALSE`), funktional also ohne Folgen. |
| `.bdhc` / `.bdhcam` | BDHC roh; bei Pt und HGSS mit angehängten Kameradaten (`.bdhcam`) | `formats/mapbin/MapBinPt.java` und `MapBinHGSS.java`: `.bdhcam` ersetzt `.bdhc`, wenn er mehr als 10 Bytes größer ist | entspricht `MapFile.bdhc` (opak) |
| `.bgs` | `u16 0x1234`, `u16 len`, Platten | `formats/backsound/Backsound.java`, `writeToFile` (Zeilen 56–62) | entspricht `MapFile.bgs` samt Signatur, `validate()` passt |
| `.nsbmd` | Ausgabe von `g3dcvtr` aus IMD | siehe 3.1 | entspricht `MapFile.model`, nach dem Entfernen von TEX0 |

PDSMS bringt auch einen eigenen Packer für Map-`.bin` mit (`formats/mapbin/MapBinPt.java`, `MapBinHGSS.java`, `MapBinDP.java`). Er setzt Kopf, [BGS], PER, BLD, NSBMD und BDHC zusammen, also genau das Layout von `map.rs`. Der Pfad für mehrere Karten (`saveBinaryMaps`) ist upstream auskommentiert (`MapMatrix.java`, Block `/* public void saveBinaryMaps()`), und der Fork stuft ihn als „unverified“ ein (`V2_4_FORMAT_CAPABILITY_MATRIX.md`, Abschnitt „Important BLD limitation“). Der übliche Weg ist deshalb: PDSMS exportiert NSBMD und die Seitendateien, der Nutzer importiert sie einzeln in DSPRE.

### 1.4 Was DSPRE mit PDSMS-Dateien macht

`main` liest **kein** `.pdsmap` oder `.pdsts`. Treffer gibt es nur in README, OldReadme und einem Patch-Dialog (`git grep -il pdsmap main`). `main` importiert NSBMD, `.per`, `.bld`, `.bdhc`/`.bdhcam`, `.bgs` und `.bin` einzeln (`main:DS_Map/Editors/MapEditor.cs`, Handler `importMapButton_Click`, `importMovButton_Click`, `importBuildingsButton_Click`, `bdhcImportButton_Click`, `soundPlatesImportButton_Click`, `replaceMapBinButton_Click`).

`feature/avalonia` (`avalonia:DSPRE.Avalonia/Avalonia/ViewModels/World/MapTilesViewModel.cs`):
- `BringIn` (ab Zeile 702): `.pdsmap` geht an `BringInProject`, `.pdsts` oder `.obj` werden als Tileset geladen, alles andere wird am Inhalt als NSBMD oder ganze Map erkannt (`ReplaceFrom`).
- `BringInProject` und `PaintProjectMap` (ab Zeile 851): liest `.pdsmap` und `.pdsts` samt `.meta`, zeigt eine Liste der Karten und sucht `<stem>_XX_YY.{per,bld,bgs,bdhc}`; `bgs` nur bei HGSS.
- Stammt das Projekt aus einer anderen Spielfamilie, lässt DSPRE `bld` weg (die Gebäude-IDs unterscheiden sich je Spiel), und `bdhc` nur dann, wenn genau eine der beiden Seiten DP ist (`LeavesOut`, `FamilyOf`).
- `Apply` (ab Zeile 2540) erzeugt Folgendes:
  - **NSBMD** über `TileBake.Of` → `TileBake.ToModel` → `NsbmdWriter.Build`. Ramps und Wände werden ergänzt.
  - **Permissions** aus der `.per`, falls vorhanden, sonst aus den Kachel-Vorgaben (`TileCollisions.Apply`). Felder am Fuß einer Wand werden blockiert.
  - **BDHC** aus der `.bdhc`, falls vorhanden, sonst neu aus dem gebackenen Mesh (`BdhcBuild.ForMap`, dann `BlockUngrounded`).
  - **Gebäude** aus der `.bld`, wenn gewünscht; BGS aus der `.bgs`.
  - Vorab eine Größenprüfung (`MapFile.TooBigForTheGame`).
- Texturen: `MissingPictures` und „Add textures to ROM“ (`NsbtxWriter.Extend`/`Build`) schreiben PNGs in das Kartentextur-Pack des Gebiets. Höchstens `NsbtxWriter.MostPictures`, Namen höchstens 16 Zeichen.

---

## 2. NSBMD-Import in eine bestehende Map

### 2.1 DSPRE

**main** (`main:DS_Map/ROMFiles/MapFile.cs`, `LoadMapModel`, ab etwa Zeile 175):
- prüft nur die Magic `BMD0`;
- wenn `numBlocks` an Offset 0x0E größer als 1 ist, entfernt `NSBUtils.GetModelWithoutTextures` den TEX0-Block (`main:DS_Map/DSUtils/NSBUtils.cs:75–103`). **Dabei nimmt DSPRE MDL0 fest an Offset 0x18 und die MDL0-Größe fest an 0x1C an.** PDSMS sucht MDL0 dagegen über beide Blockoffsets (`pdsms/.../formats/mapbin/NsbmdUtils.java`), das ist robuster;
- parst danach mit `NSBMDLoader` und warnt ab 61 000 Bytes (`MAPMODEL_CRITICALSIZE`, `main:DS_Map/Editors/MapEditor.cs`, Region „3D Model Editor“);
- **lässt Permissions, Gebäude, BDHC und BGS unverändert.** Nur `mapModelData` wird ersetzt, `ToByteArray` rechnet die Längen neu.

**avalonia** (`avalonia:DSPRE.Core/ROMFiles/MapFile.cs`):
- `ReadImport` erkennt am Inhalt, ob es sich um NSBMD (Magic) oder eine ganze Map handelt (mit Rückfall auf das Layout der anderen Familie, `ReadForGame`). TEX0 wird wie in main entfernt, dann folgt `CannotUseModel`;
- `ModelRoom = 0xF000` und `TerrainRoom = 0x9000`, mit Verweis auf pokeplatinum (`TooBigForTheGame`);
- `ReplaceFrom` (`MapTilesViewModel.cs`): „Puts a model file in place of this map's model (permissions, buildings and terrain stay)“, danach `MapImportCheck.Note`, das prüft, welche Materialien das NSBTX des Gebiets nicht abdeckt (`avalonia:DSPRE.Core/ROMFiles/MapImportCheck.cs`, `MapTextures`, `RankPacks`);
- Undo über `Remember()`, das bis zu 32 komplette Map-Byte-Stände hält.

### 2.2 Was das Spiel verlangt (pokeplatinum)

- `MAP_MODEL_FILE_SIZE 0xF000` (`src/overlay005/loaded_map_buffers.c:11`, statischer Puffer `sGlobalMapModelFiles[...][0xF000]`). Für HGSS spricht `.space 4 * 0xF000` in `pret/pokeheartgold:asm/overlay_01_021FAC44.s` dafür (gefunden per `gh search code`; dass es dieselbe Bedeutung hat, ist **nicht verifiziert**).
- `BDHC_BUFFER_SIZE 0x9000` (`include/constants/field/bdhc.h`, Assert in `bdhc.c:282`). **Für HGSS nicht verifiziert**, die Suche in pokeheartgold blieb ohne Treffer.
- `GF_ASSERT(mapResMdlSet->dict.numEntry == 1)`: **das MDL0 muss genau ein Modell enthalten**, das Spiel nimmt Index 0 (`land_data.c`, `LandDataManager_LoadMapModel`, etwa Zeilen 1975–1995). DSPRE prüft das nicht.
- Texturen bindet das Spiel über `Easy3D_BindTextureToResource` → `NNS_G3dBindMdlSet` mit dem NSBTX des Gebiets (`src/easy3d.c:106–117`). Die Funktion gibt immer TRUE zurück. Fehlende Namen führen also nicht zum Absturz, sondern zu ungebundenen Materialien. Das sichtbare Ergebnis („grau/weiß“) nennt DSPRE in `MapImportCheck`; im Spiel habe ich es nicht geprüft.
- Die Abschnitte werden nacheinander mit `NARC_ReadFile` in eigene Puffer gelesen (`land_data.c:457–600`). Eine Ausrichtung des Modells innerhalb der Map-Datei verlangt der Code nicht erkennbar. Vanilla-Modelle sind trotzdem alle 4-Byte-ausgerichtet (siehe 6.2), deshalb empfehle ich eine Warnung.

### 2.3 gen4-studio heute (`crates/g4-formats/src/map.rs`)

- Kopf: 16 Bytes, vier `u32`-Längen (Permissions, Gebäude, Modell, BDHC). Bei HGSS folgt der BGS-Block (`0x1234`, `u16 len`, Daten). Dann Permissions (genau 2048 Bytes), Gebäude (n×48 plus `buildings_tail`), Modell und BDHC, beide als opake `Vec<u8>`.
- `read` lehnt überzählige Bytes ab, `validate` prüft Permissions-Länge, Tail-Länge und BGS, `write` ist infallibel und leitet die Längen aus den Feldern ab.
- Es gibt keinen NSBMD-Parser in `g4-formats` (Module: `area_data, error, game, lib, map, matrix, narc, nsbtx, test_support`). Modelle laufen über apicula als GLB (`crates/g4-kernel/src/export.rs`, `nsbmd_to_glb`). Einen NSBTX-Leser mit Texturnamen gibt es (`crates/g4-formats/src/nsbtx.rs`).

**Wie man das Modell byte-sicher tauscht:** `map.model = neues_nsbmd; map.validate()?; map.write(family)`. Die übrigen Abschnitte werden nicht angefasst, deshalb ist der Tausch per Konstruktion byte-sicher. Das gilt auch für den BGS-Block, Gebäude-Rotation und Dummy-Bytes (anders als im PDSMS-Writer) sowie für `buildings_tail` (Platin DE Map 506). Testvorschlag:
1. Für jede Vanilla-Map: Modell durch sich selbst ersetzen, das Ergebnis muss den Originalbytes gleichen.
2. Modell von Map A in eine Kopie von Map B einsetzen: Kopf und Modellabschnitt ändern sich, alle anderen Abschnitte sind slice-gleich; danach zurücktauschen ergibt das Original.
3. `strip_tex0(embed_tex0(vanilla_mdl, area_nsbtx)) == vanilla_mdl`. Das sollte gelten, weil alle Vanilla-Map-Modelle `numBlocks = 1`, BOM `FFFE` und Version `0x0002` haben, MDL0 also an 0x14 liegt (siehe 6.2).

**Prüfungen, die ich für den Import empfehle** (über DSPRE hinaus):
1. Magic `BMD0`, BOM `FF FE`, Version `0x0002`, Dateigrößen-Feld (0x08) gleich der Länge, Kopfgröße 16, Blockanzahl 1 oder 2 und Blockoffsets im gültigen Bereich.
2. TEX0 entfernen, MDL0 über die Blockoffsets suchen (nicht fest an 0x18), den Kopf neu schreiben wie `NsbmdUtils.nsbmdTexToNsbmdOnly`.
3. MDL0: Signatur prüfen und **genau ein Modell im Dictionary verlangen** (Assert im Spiel).
4. Größe ≤ 0xF000 als Fehler. Nicht DSPREs 61 000: dann würden 6 Vanilla-HGSS-Maps die Warnung auslösen (siehe 6.2).
5. Länge % 4 ≠ 0 nur als Warnung.
6. Textur- und Palettennamen der Materialien gegen das Map-NSBTX des Gebiets (Area Data) prüfen und fehlende melden. Dafür braucht es einen kleinen MDL0-Material-Dictionary-Leser in `g4-formats`, Aufwand gering, aber noch nicht vorhanden. Optional wie DSPRE eine Rangliste der Packs (`RankPacks`).
7. Parsen mit apicula als Plausibilitätstest: die Vorschau tut das ohnehin.

---

## 3. OBJ → NSBMD

### 3.1 Vorhandene Werkzeuge

| Werkzeug | Richtung | Lizenz | Beleg / Status |
|---|---|---|---|
| apicula | NSBMD/NSBTX/BCA/BTP/BTA → **COLLADA, glTF**, PNG; Viewer | 0BSD | `apicula/README.md` („Models can be converted to COLLADA or glTF“), `apicula/LICENSE`; `src/convert/` enthält nur `collada` und `gltf`, keinen Writer. Auch kein OBJ-Export. |
| Nintendo `g3dcvtr.exe` (NITRO-System) | IMD → NSBMD/NSBTX | proprietär, aus einem Leak, nicht weitergebbar | PDSMS ruft ihn auf: `pdsms/.../editor/converter/NsbmdOutputInfoDialog.java:460–471` (`-eboth` / `-emdl`, unter Linux/macOS per `wine`); README: „g3dcvtr.exe … must be placed in the bin/converter folder“. Fork: „g3dcvtr.exe is user-supplied and never redistributed“ (`RELEASE_NOTES.md`). |
| PDSMS (upstream und Fork) | Kacheln → OBJ → IMD (eigener Code, `formats/imd/ImdModel.java`) → NSBMD nur über g3dcvtr | **keine Lizenz** (keine LICENSE-Datei; GitHub-API `license: null` für Trifindo, ebenso für den Fork) | Rechtlich „alle Rechte vorbehalten“: weder Code noch Tilesets dürfen übernommen werden. |
| DSPRE `feature/avalonia` | OBJ (`MapObjImport.cs`, erkennt DSPRE- und PDSMS-Koordinaten), `.pdsts`-Kacheln (`TileBake.cs`, 676 Zeilen) → `NsbmdWriter.cs` (634 Zeilen: eigener MDL0 mit GX-Displaylisten, 4.12-Positionen mit Skalierungsschieber, höchstens 8000 Dreiecke und 60 Materialien) | AGPL-3.0 (`LICENSE` in beiden Branches) | Das ist der einzige quelloffene OBJ→NSBMD-Writer, den ich gefunden habe. gen4-studio ist MIT (`Cargo.toml: license = "MIT"`): **Portieren scheidet aus**, nur eine Clean-Room-Umsetzung nach eigener Doku wäre möglich. |
| MKDS Course Modifier (Gericom) | Mario-Kart-DS-Editor, NSBMD-Unterstützung | Freeware, Windows, Quellcode nicht öffentlich | Dass es OBJ→NSBMD kann, ist **nicht verifiziert** (fileinfo.com nennt nur NSBMD-Unterstützung). Für gen4-studio ohnehin unbrauchbar. |
| „nitro“-Tools allgemein | – | – | Die Websuche nach einer offenen g3dcvtr-Alternative oder einem glTF→NSBMD-Writer brachte nichts außer apicula, das die umgekehrte Richtung macht. **Dass es keinen gibt, ist nicht bewiesen**, nur nicht gefunden. |

### 3.2 Ist ein Rust-Weg in 3b.4 realistisch?

Nein, nicht als Zusage. Gründe:
- Die Roadmap ordnet „NSBMD/NSBTX-Writer in `g4-formats`, Round-Trip gegen apicula-Lesepfad und Vanilla“ und das Baking ausdrücklich Phase 6 zu (`docs/ROADMAP.md:170–178`).
- Allein DSPREs Writer-Stack ist rund 2000 Zeilen: `NsbmdWriter` 634, `TileBake` 676, `BdhcBuild` 628, dazu `GxDisplayList`, `NitroDictionary`, `MaterialLook` und `ObjMesh`. Er ist über Monate gewachsen (Ramps, Wände, Licht, Materialflags). Eine Clean-Room-Fassung in Rust mit Tests gegen das Spiel ist ein eigenes Projekt.
- Spezifikationsquellen für Phase 6: der apicula-Leser (0BSD, als Referenz frei nutzbar) und scurests `nsbmd_docs` (im apicula-README verlinkt; **Lizenz nicht geprüft**).

### 3.3 Was 3b.4 zusagen sollte und was nicht

**Zusagen:**
- NSBMD-Import (Datei → Modellabschnitt) mit den Prüfungen aus 2.3, Vorschau über apicula und Undo.
- Import einer ganzen Map-`.bin` (z. B. aus DSPRE exportiert oder aus einem anderen Projekt), mit Umwandlung zwischen Pt und HGSS (BGS-Block anlegen oder entfernen) wie in DSPREs `ReadForGame`/`TakeContentsOf`.
- Import der Seitendateien `.per`, `.bld`, `.bdhc`/`.bdhcam`, `.bgs` (siehe 4 und 5).
- **PDSMS-Projekt ohne Baking:** `.pdsmap` nur lesen, um Karten, Positionen und `areaindex` zu bekommen. Pro Karte die Seitendateien und das vom Nutzer *mit PDSMS/g3dcvtr exportierte* `<stem>_XX_YY.nsbmd` zuordnen und in Matrixzellen oder bestehende Maps setzen. Gebäude bei fremder Spielfamilie weglassen oder warnen, wie DSPREs `LeavesOut`.

**Nicht zusagen (Phase 6):**
- `.pdsts` lesen oder rendern, Kacheln zu NSBMD backen.
- OBJ/glTF → NSBMD.
- PNG → NSBTX (neue Texturen im Pack).
- BDHC-Erzeugung aus Geometrie oder Höhen.
- Kollisionsableitung aus Kacheln (`.pdsts.meta`).

**Roadmap-Text korrigieren:** „OBJ über apicula-Pfad“ ersetzen, etwa durch „OBJ/glTF erst mit dem NSBMD-Writer in Phase 6; apicula ist nur Lese- und Exportpfad“.

---

## 4. Kollisionsimport

**Quellen und Formate:**
- PDSMS `.per`: 2048 Bytes, je Zelle `type, collision`, Zeilen außen, Spalten innen (`Collisions.toByteArray`). Das ist **byte-identisch mit dem Permissions-Abschnitt** der Map und mit `MapFile.permissions` in gen4-studio (Index `y*32+x`, `Cell { kind, collision }`, `map.rs:26–32`).
- DSPRE main: Export und Import als `*.per` (`MapFile.MovepermsFilter`, `CollisionsToByteArray`, `ImportPermissions`).
- DSPRE avalonia: dieselben Bytes, aber Dateiendung `*.mp` (Filter beim Import `*.mp, *.bin, *.*`; `avalonia:.../MapEditorViewModel.cs`, `ImportPermissionsAsync`/`ExportPermissionsAsync`, etwa Zeile 3025).
- **Keine Prüfung bei DSPRE:** `ImportPermissions` liest ohne Längenprüfung 1024 Bytepaare in die Arrays. Eine zu kurze Datei wirft `EndOfStreamException`, nachdem ein Teil schon überschrieben wurde (das Array wird zeilenweise beschrieben, der Teilzustand bleibt). Eine zu lange Datei wird stillschweigend abgeschnitten.
- **Import aus einem Bild:** gibt es in DSPRE nicht. `git grep` nach png/permission/collision in beiden Branches fand nur eine Screenshot-Funktion. In PDSMS habe ich auch nichts gefunden (nicht erschöpfend geprüft).
- **Aus einer anderen Map übernehmen:** DSPRE avalonia per `TakeContentsOf` (die ganze Map) oder über die Undo-Schritte; ein eigener Befehl „Permissions von Map N kopieren“ ist mir nicht aufgefallen (**nicht vollständig geprüft**).
- PDSMS-Kachel-Vorgaben (`.pdsts.meta`, Spalten Type und Collision je Kachelfeld) gibt es nur im Fork. DSPRE wendet sie in `TileCollisions.Apply` an. Das gehört zu Phase 6.

**Empfehlung für 3b.4:**
- Datei-Import `.per`/`.mp`/`.bin`: verlangt **genau 2048 Bytes**, sonst lehnt der Import ab, ohne etwas zu ändern.
- „Permissions aus Map N übernehmen“ innerhalb des Projekts.
- Beides als ein einziger Undo-Schritt.
- Optional: nur die Collision-Bytes oder nur die Type-Bytes übernehmen.
- Bei HGSS-Typnamen gegen die Namen aus 3b.1 eine Warnung, wenn unbekannte `kind`-Werte auftauchen (Idee, keine Anforderung).

---

## 5. BDHC beim Import

- **PDSMS:** BDHC wird von Hand gebaut, mit einem Platten-Editor (`formats/bdhc/BdhcEditorDialog.java`, `Plate`, `Slope`). Standard sind zwei Platten (`Bdhc()`, `Bdhc.java`). Writer `BdhcWriterDP` und `BdhcWriterHGSS`; Platin läuft über den HGSS-Writer (`MapMatrix.saveBDHCs`), was der Fork „fixture verification required“ nennt (`V2_4_FORMAT_CAPABILITY_MATRIX.md`). Eine automatische Erzeugung aus der Geometrie fand ich nicht (Grep nach generate/auto/fromGrid ohne Treffer, **nicht erschöpfend**). `.bdhcam` hängt Kameraplatten an das BDHC an (Pt, HGSS).
- **DSPRE main:** Import und Export roh (`ImportTerrain`, keine Prüfung), Filter DP `*.bdhc`, sonst `*.bdhc;*.bdhcam`.
- **DSPRE avalonia:** zusätzlich `BdhcBuild` (`avalonia:DSPRE.Core/Models/BdhcBuild.cs`). Platten aus den Mesh-Flächen (`PiecesOf`), Rechtecke je Feld (`Squares`), Ausgabe im Platten-Format für Pt und HGSS (`Plates`) oder im Dreiecks-Format für DP (`Triangles`). Von Hand gesetzte Platten bleiben erhalten (`KeptPlates`). `BlockUngrounded` blockiert begehbare Felder ohne Platte, laut Kommentar „since the game won't let anyone stand there“ (**im Spiel nicht verifiziert**). Beim reinen NSBMD-Import (`ReplaceFrom`) bleibt das BDHC unverändert, neu erzeugt wird es nur in `Apply` nach dem Tile-Baking.
- **Spiel:** BDHC wird über die Zähler im Kopf geladen (Magic, dann `pointsCount`, `normalsCount`, `constantsCount`, `platesCount`, `stripsCount`, `accessListCount`; `bdhc.c:239–251`); Puffer 0x9000.
- **gen4-studio:** BDHC ist opaker Byteblock in `MapFile.bdhc`, ohne Parser. Die Matrix-`altitudes` sind ein eigener Abschnitt der Matrix (`crates/g4-formats/src/matrix.rs:23`), keine BDHC-Daten. Sie heben in der Szene die ganze Karte um eine halbe Kachel je Stufe (`docs/ROADMAP.md`, 3b.1). **Das BDHC zu einem importierten Modell erzeugt gen4-studio nicht neu.** Wer nur das Modell tauscht, behält das alte BDHC. Ob Laufhöhe und neue Geometrie zusammenpassen, liegt beim Nutzer.

**Empfehlung für 3b.4:**
- BDHC nur roh importieren (`.bdhc`, `.bdhcam`, aus einer ganzen `.bin` oder aus einer anderen Map) mit den Prüfungen: Magic `BDHC` (alle Vanilla-Maps haben sie, siehe 6.2), Größe ≤ 0x9000, Kopf-Zähler und Länge plausibel. Ein DP-BDHC (Dreiecks-Format) für Pt/HGSS ablehnen; DPs eigenen Aufbau kennt gen4-studio noch nicht.
- Beim Modell-Import einen Hinweis „BDHC unverändert, Laufhöhen prüfen“ zeigen.
- BDHC-Erzeugung bleibt in Phase 6.

---

## 6. Testdaten

### 6.1 Öffentliche `.pdsmap`-Beispiele

- **Weder PDSMS upstream noch der Fork noch DSPRE enthalten `.pdsmap`, `.per`, `.bld` oder `.bdhc`** (`find` in beiden PDSMS-Klonen; `git ls-tree` von DSPRE avalonia findet nur die Quelldateien `PdsmapFile.cs`/`PdstsFile.cs`; die einzige Fixture in `DSPRE.Tests/Fixtures/` ist ein PNG). Die Tests des Forks erzeugen `.pdsmap` zur Laufzeit (`pdsms-adastra/src/test/java/editor/mapmatrix/MapMatrixAreaSaveTest.java:40`).
- PDSMS liefert nur Tilesets mit (`src/main/resources/tilesets/<Spiel>/...`, `.pdsts` plus PNGs). Die meisten sind aus Spielen gerippte Texturen (Ordnernamen wie `Tileset_13_-_Overworld_(by_Jiboule)`, `Black_-_White/Overworld`). Ohne Lizenz und mit Nintendo-Anteilen kommen sie **nicht** ins Repo. `Various/Low_Poly_Tileset` (Texturen von 109 bis 803 Bytes) wirkt selbst gemacht, ist aber ebenfalls ohne Lizenz.
- Beispielprojekte aus Foren oder Discord habe ich nicht gesucht oder geprüft; ihre Lizenz wäre ohnehin unklar.

**Vorschlag:**
1. **Synthetische Fixtures im Testcode erzeugen.** `.pdsmap` ist Text, ein kleiner Generator für 8 und 9 Ebenen, mit und ohne `exportgroup`/`egcenter` und mehreren Karten genügt. `.per`, `.bld` und `.bgs` sind trivial. Eine minimale `.pdsts` (ein Material, eine Kachel) ist mit ein paar Zeilen Big-Endian-TLV erzeugt, aber nur nötig, wenn 3b.4 `.pdsts` überhaupt anfasst; nach Empfehlung 3.3 tut es das nicht.
2. Ein echtes PDSMS-Projekt des Nutzers, **nicht eingecheckt**, als gated Test über eine Umgebungsvariable wie die übrigen gated Tests. Das empfiehlt auch der Fork: „User-owned, non-committed representative files“ (`V2_4_FORMAT_CAPABILITY_MATRIX.md`, „Fixtures Still Required“).

### 6.2 Die drei Testprojekte (nur gelesen)

Ermittelt mit `scratchpad/research-3b4/scripts/mapstats.py` über `unpacked/maps/*`:

| | Platin DE | HeartGold DE | HeartGold US |
|---|---|---|---|
| Maps | 666 | 676 | 676 |
| größtes Modell | 60 556 B | **61 420 B** | 61 420 B |
| Modelle > 61 000 B (DSPRE-main-Warnung) | 0 | **6** | 6 |
| Modelle > 0xF000 (61 440) | 0 | 0 | 0 |
| NSBMD `numBlocks` | immer 1 | immer 1 | immer 1 |
| BOM/Version | immer `FFFE 0200` | ebenso | ebenso |
| Modelllänge % 4 ≠ 0 | 0 | 0 | 0 |
| größtes BDHC | 21 468 B | 10 794 B | 10 794 B |
| BDHC-Magic | immer `BDHC` | immer `BDHC` | immer `BDHC` |
| meiste Gebäude | 32 | 30 | 30 |
| BGS nicht leer | – | 230 Maps, max. 392 B | ebenso |

Daraus folgt:
- Grenze 0xF000 statt 61 000.
- Jedes Vanilla-Modell taugt als Importfixture, zur Laufzeit aus dem Projekt gelesen und nie eingecheckt; so landen keine Nintendo-Assets im Repo.
- Für TEX0-Entfernung und Pack-Abgleich eignet sich ein zur Laufzeit gebautes „texturiertes“ NSBMD aus Modell-MDL0 und Area-NSBTX-TEX0 (`unpacked/mapTextures/NNNN`).
- Gated Testfälle: (a) Modell-Tausch zwischen zwei Maps desselben Gebiets; (b) Import eines HGSS-Map-`.bin` in eine Platin-Kopie und umgekehrt (BGS-Umwandlung); (c) `.per`-Rundweg (Export, Import, gleiche Bytes); (d) Ablehnung eines Modells > 0xF000 (synthetisch aufgeblasen) und eines MDL0 mit zwei Modellen (synthetisch).
- In keinem Projekt liegen `.pdsmap`, `.per` oder `.bld`. Die gefundenen `.nsbmd` unter `files/data/` (`ug_base_cur.nsbmd`, `crystal.nsbmd`) sind keine Map-Modelle.

---

## 7. Offene Punkte und Unsicherheiten

- BDHC-Puffergröße und Modellpuffer bei HGSS: nur indirekt belegt.
- Ob ungebundene Materialien im Spiel tatsächlich grau oder weiß erscheinen: nur DSPREs Aussage.
- `BlockUngrounded` (ohne Platte nicht begehbar): nur DSPREs Kommentar.
- Wie PDSMS die NSBMD-Ausgabe benennt: sie folgt dem OBJ- bzw. IMD-Namen (`NsbmdOutputInfoDialog.java:439–440`), der Ordner ist frei wählbar. Ein Importer muss das NSBMD also suchen oder den Nutzer wählen lassen.
- Lizenz von scurests `nsbmd_docs` (relevant erst für Phase 6).
- Ob der DSPRE-Avalonia-Branch nach dem 21.10. noch Import-Validierungen ergänzt hat: nicht gefetcht.
