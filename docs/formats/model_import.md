# Modelle und Texturen importieren (OBJ, glTF, PNG)

Quellen: die OBJ- und MTL-Beschreibung von Wavefront (Anhang B1 und B2 des „Advanced
Visualizer“-Handbuchs), die glTF-2.0-Spezifikation von Khronos, die Crates `tobj` 4.0.5 (MIT)
und `gltf` 1.4.1 (MIT OR Apache-2.0), Gutachten C
(`docs/research/2026-11-05-phase-6/gutachten-c-malen-import.md` §6, §8.3) und eigene Messung.
Implementierung: `crates/g4-kernel/src/model_import/` (`mod.rs` mit `convert_for_map`,
`read_obj.rs`, `read_gltf.rs`, `mesh.rs`, `textures.rs`, `samples.rs`) und
`crates/g4-kernel/src/map_import.rs` (Teile, Prüfung, Import). Der Bäcker, der aus den Flächen
das Kartenmodell schreibt, steht in `nsbmd.md`, das Anhängen an das Texturpaket in `nsbtx.md`.

## Zweck

Der Dialog „Teile importieren…“ einer Karte nimmt neben den fertigen Binärteilen (`.nsbmd`,
`.per`, `.bld`, `.bdhc`, `.bgs`, `.bin`) auch Modelle aus OBJ und glTF und Bilder aus PNG:

- ein Modell wird zum **Modell-Teil** der Karte: gelesen, in Flächen des Bäckers umgewandelt,
  gebacken und auf das Budget des Spiels (0xF000 Byte) geprüft; seine Bilder werden Texturen
  im Kartentextur-Paket des Gebiets,
- die PNG-Dateien eines Aufrufs werden **ein Teil „Texturen“**, angehängt an dasselbe Paket.

Geprüft wird vorher ohne Schreiben (Vorschau mit Kennzahlen, Budget und Ablehnungsgrund),
importiert wird als **ein** Undo-Schritt.

Freiform in Welt-Karten: [world_map.md](world_map.md) „Freiform und Beifiles (6.5b)“. Bei einer
Welt-Karte wird ein Modell aus OBJ oder glTF nicht der Modell-Teil, sondern die Zeile „Freiform
(glTF)“: dieselben Leser und `options`, das Ergebnis ist ein kanonisches Beifile
`data/maps/NNNN.<name>.glb`, das mit dem Raster backt; Teilmodelle bekommen ihr Modell über
`tileset_piece_set` ([tileset.md](tileset.md)), Form 2 liest das GLB von apicula mit denselben
Lesern.

## Dateiarten

Die Endung entscheidet (Groß- und Kleinschreibung egal):

| Endung | Teil | Weg |
|---|---|---|
| `.obj` | Modell | OBJ mit MTL (`tobj`) |
| `.gltf`, `.glb` | Modell | glTF 2.0 als JSON mit Puffern oder als Binärdatei (`gltf`) |
| `.png` | Texturen | alle PNG eines Aufrufs zusammen, eine Zeile im Dialog (`preview.files`) |

Zwei Modell-Dateien in einem Aufruf (auch `.obj` neben `.nsbmd`) sind wie zwei Dateien eines
anderen Teils `part.duplicate`. Der Teil „Texturen“ gehört nicht zur Kartendatei: er wird weder
exportiert („Teile exportieren…“ bietet ihn nicht an, `export` lehnt ihn ab) noch aus einer
anderen Karte übernommen.

## Einheit und Achsen

`options.unit` gibt die Welteinheiten je Modelleinheit an, Vorgabe **16**: eine Modelleinheit
ist eine Kachel (so exportiert Pokémon DS Map Studio; bei glTF ist 1 m eine Kachel). Erlaubt
sind 0,0625 bis 256, sonst lehnt die Prüfung den Aufruf ab, bevor sie eine Datei liest.
`options.colors` ist 16 (Vorgabe) oder 4 Farben je Textur.

Die Achsen bleiben, wie sie sind: Y oben, X nach Osten, Z nach Süden, der Ursprung ist die
Kartenmitte. OBJ und glTF sind rechtshändig mit Y oben wie die Welt des Bäckers.

## Genauigkeit

Jede Ecke wird auf **ganze Welteinheiten** gerundet (halbe weg von 0; eine Welteinheit ist 1/16
Kachel), jede Texturkoordinate auf **ganze Texel** der gebundenen Textur. Eine Fläche, die nach
dem Runden keine Fläche mehr hat, fällt weg (Hinweis `model.faces-dropped` mit der Anzahl);
bleibt keine, ist das Modell `model.no-geometry`. Eine Ecke außerhalb von −512 bis 511
Welteinheiten ist `model.out-of-range` mit ihrer Koordinate.

Die Texturkoordinaten einer Fläche müssen in den Bereich des Spiels passen (höchstens 2 047
Texel Spanne je Achse, auch nach Verschieben um ganze Texturen), sonst `model.uv-range` mit
Textur, Achse und Spanne.

## Polygone und Paare

- Ein Dreieck bleibt ein Dreieck (im Bäcker eine Fläche, deren letzte Ecke die dritte
  wiederholt).
- Ein Viereck bleibt ein Viereck, wenn es eben ist (die vierte Ecke höchstens 0,5 Einheiten von
  der Ebene der ersten drei) und konvex; sonst wird es wie ein n-Eck zerlegt.
- n-Ecke ab fünf Ecken werden per Ohrenschneiden in der Ebene ihrer Newell-Normale in Dreiecke
  zerlegt (eigener Code); bei einem Selbstschnitt als Fächer.
- Danach werden **Dreieckspaare zu Vierecken** zusammengelegt: gleiches Material, eine
  gemeinsame Kante mit gleicher UV an beiden Ecken, das Viereck eben und konvex; gierig in der
  Reihenfolge der Eingabe. Eine Ebene aus glTF (zwei Dreiecke) wird so ein Viereck.

Die Reihenfolge der Ecken bleibt (gegen den Uhrzeigersinn ist die Vorderseite); spiegelt eine
glTF-Knotenmatrix (negative Determinante), wird sie umgedreht.

## Materialien und Texturnamen

Je Material:

- hat es ein Bild (OBJ `map_Kd`, glTF `baseColorTexture`), wird daraus eine Textur mit dem
  Namen des Bildstamms: jedes Zeichen außerhalb von `0x21..=0x7E` wird `_`, auf 16 Byte
  gekürzt, leer wird `g4_tex`; die Palette heißt wie in 6.1 `<name>_pl`;
- steht der Name schon im Paket, wird die Textur des Pakets benutzt (Hinweis
  `texture.reused`, nichts angehängt), so hängt ein zweiter Import nichts doppelt an;
- zwei verschiedene Bilder mit gleichem Namen in einem Aufruf werden `_2`, `_3` … (auf 16
  Byte gekürzt);
- ein Material ohne Bild benutzt die Textur des Pakets mit seinem Namen, sonst ist es
  `model.material-untextured` (ebenso ein Polygon ohne Material).

Für PNG-Dateien als Teil „Texturen“ gilt derselbe Name; steht er im Paket, ist das
`texture.taken`: wer PNG wählt, will eine neue Textur, Ersetzen gibt es nicht. Ein Modell
trägt höchstens 255 Texturen (`model.texture-count`).

Bilder werden mit `image` (PNG, JPEG) gelesen, **nicht skaliert**: jede Seite muss eine
Zweierpotenz von 8 bis 1 024 sein (`texture.side`). Umgewandelt wird mit `texconv::convert`
aus 6.1 (Median-Cut in BGR555, Farbe 0 für Alpha unter 128, kein Dithering, eine Palette je
Textur); der mittlere Farbfehler steht im Hinweis `texture.new`.

Der Hinweis „Texturnamen fehlen“ (`model.textures-missing`) läuft für umgewandelte Modelle
nicht: ihre Namen stehen nach dem Anhängen im Paket.

## Dateien neben dem Modell

MTL-Dateien, Bilder und glTF-Puffer werden nur relativ und **innerhalb des Ordners des
Modells** gelesen (Unterordner erlaubt, Rückwärtsstriche gelten als Trenner). Ein absoluter
Pfad, ein `..` aus dem Ordner, eine URI mit Schema (außer `data:` in glTF) und ein Symlink nach
außen sind `model.file-outside`, bevor eine Datei gelesen wird; eine fehlende Datei ist
`model.file-missing`. glTF-`data:`-URIs (base64) sind erlaubt.

## glTF-Teilmenge

- `.glb` (Puffer im BIN-Chunk) und `.gltf` mit externen Puffern und Bildern oder
  `data:`-URIs.
- Szene: `scene` des Dokuments, sonst Szene 0, ohne Szene alle Wurzelknoten. Knotenmatrizen
  werden angewandt; ein Mesh an zwei Knoten kommt zweimal.
- Primitive `TRIANGLES`, `TRIANGLE_STRIP`, `TRIANGLE_FAN` (zu Dreiecken entfaltet).
- Nicht übernommen, mit dem Hinweis `model.parts-ignored`: Linien, Punkte, Animationen,
  Skelette, Kameras, Lichter, Eckenfarben, Texturoptionen. Ein glTF aus nur Linien und Punkten
  ist `model.no-geometry`, der Hinweis bleibt daneben stehen.
- `extensionsRequired` nicht leer (etwa Draco) ist `model.source-unreadable`.

## Prüfen und Vorschau

Die Prüfung (`inspect` / `inspect_with`) rechnet alles bis zum Bäcker und schreibt nichts:
Gebiet der Karte (erster Header, der sie zeigt; keiner → `part.no-area`), Modell lesen,
Texturplan gegen das Paket, Paket mit den neuen Texturen im Speicher, VRAM des Gebiets mit dem
neuen Paket, Flächen, Backen. Jeder Teil kommt als Zeile (`PartReport`) mit Größe (beim Modell
die Bytes des gebackenen Modells, bei Texturen der Zuwachs des Pakets), Zustand, Hinweisen und
`preview`:

| Feld | Inhalt |
|---|---|
| `pack`, `area`, `style` | Kartentextur-Paket, Gebiet und Lichtstil des Gebiets |
| `vram_bytes`, `vram_limit` | VRAM des Gebiets (Kartenpaket und Gebäudepaket) mit den neuen Texturen; das größte Vanilla-Gebiet |
| `textures` | jede benutzte Textur: `name`, `file`, `width`, `height`, `colors`, `new`, `mean_error` |
| `model` | nur beim Modell: `format` (`obj`, `gltf`), `stats` (`polygons`, `quads`, `triangles`, `dropped`), `strips`, `single_quads`, `budget` (Größe gegen 0xF000 mit Zustand) |
| `files` | die Dateien des Teils (beim Modell die Modelldatei, bei Texturen alle PNG) |

Eine 3D-Vorschau gibt es im Dialog nicht: nach dem Import zeigt die Szene das Modell, ⌘Z ist
der Rückweg. Liegt das VRAM des Gebiets über dem größten Vanilla-Gebiet, ist der Teil eine
Warnung `texture.vram-high`; weitere Gebiete desselben Pakets stehen als Hinweise mit demselben
Code daneben.

### Codes

Die Sätze stehen in `apps/studio/src/workbench/inspector/importParts.ts`; Hinweise ändern den
Zustand eines Teils nicht.

| Code | Argumente | Bedeutung |
|---|---|---|
| `model.source-unreadable` | Format, Grund | die Datei (oder ihre MTL) ist kein lesbares OBJ/glTF |
| `model.file-missing` | Pfad wie geschrieben | eine Datei, auf die das Modell verweist, fehlt |
| `model.file-outside` | Pfad wie geschrieben | eine Datei liegt außerhalb des Modellordners |
| `model.no-geometry` | – | keine Fläche (keine gelesen oder alle beim Runden weg) |
| `model.material-untextured` | Material | kein Bild und keine Paket-Textur dieses Namens |
| `model.uv-missing` | Material | texturierte Flächen ohne Texturkoordinaten |
| `model.out-of-range` | x, y, z, −512, 511 | eine gerundete Ecke außerhalb der Karte |
| `model.uv-range` | Textur, Achse, Spanne | eine Fläche spannt mehr Texel, als das Spiel trägt |
| `model.texture-count` | Anzahl, 255 | mehr Texturen, als ein Kartenmodell trägt |
| `model.converted` | Format, Polygone, Vierecke, Dreiecke, Streifen, Einzel-Vierecke | Hinweis: umgewandelt |
| `model.faces-dropped` | Anzahl | Hinweis: Flächen ohne Fläche nach dem Runden |
| `model.parts-ignored` | Liste (`lines`, `points`, …) | Hinweis: nicht übernommene Teile des glTF |
| `model.too-big` | Größe, 61440 | das gebackene Modell ist größer als der Puffer des Spiels |
| `model.bdhc-unchanged` | – | Hinweis: die BDHC der Karte bleibt |
| `part.no-area` | Karte | kein Header zeigt die Karte, also kein Paket und kein Licht |
| `texture.side` | Name, Breite, Höhe | eine Seite ist keine Zweierpotenz von 8 bis 1 024 |
| `texture.name` | Name | kein gültiger Texturname |
| `texture.taken` | `texture`/`palette`, Name | der Name steht schon im Paket (PNG) oder kommt zweimal vor |
| `texture.pack-full` | was, Wert, höchstens | das Paket ist voll (Einträge, Texel, Paletten, Offset) |
| `texture.unreadable` | Datei, Grund | das Bild lässt sich nicht lesen |
| `texture.new` | Name, Breite, Höhe, Farben, Fehler | Hinweis: neue Textur |
| `texture.reused` | Name | Hinweis: die Textur des Pakets wird benutzt |
| `texture.vram-high` | Gebiet, VRAM, höchstens | Warnung: mehr VRAM als das größte Vanilla-Gebiet |

## Ein Undo-Schritt

`import` / `import_with` prüft wie oben, sammelt die neuen Texturen aller gewählten Teile
(Modell und Texturen, ein Paket), hängt sie in **einem** Schritt an das Paket an
(`append_textures`; ein Name, den Modell und PNG beide wählen, ist vorher `texture.taken`),
schreibt dann die Karte (`edit_map`) und fasst beides mit `squash_since` zu **einem** Schritt
zusammen, Label `map NNNN: model imported`, `map NNNN: textures imported` oder
`map NNNN: model and textures imported`. Scheitert der Kartenschritt, wird das Anhängen
zurückgenommen (`rollback_to`, kein Redo-Eintrag). Zurück kommen die geänderten Assets: die
Karte, dann das Paket. ⌘Z nimmt Karte und Paket zusammen zurück; andere Karten und Pakete
bleiben byte-gleich.

## Karten aus Pokémon DS Map Studio

Pokémon DS Map Studio (PDSMS) hat keine Lizenz; gen4-studio liest daher kein `.pdsmap` und
kein `.pdsts` und enthält keinen Code aus PDSMS. Der Weg geht über den eigenen OBJ-Export von
PDSMS:

1. In PDSMS die Karte mit „Export as OBJ with Textures“ in einen eigenen Ordner exportieren
   (OBJ, MTL und die PNG der Kacheln liegen nebeneinander).
2. In gen4-studio die Zielkarte wählen, „Teile importieren…“ öffnen und die OBJ wählen.
3. Einheit 16 lassen (PDSMS exportiert eine Kachel als eine Einheit) und prüfen: die Vorschau
   nennt die neuen und die wiederverwendeten Texturen und das Budget.
4. Importieren. BDHC und Kollision bleiben die der Karte (Hinweis `model.bdhc-unchanged`);
   Laufhöhen und Kollision werden in gen4-studio gesetzt.

Das Beispiel `g4_pds_feld.obj` aus `samples.rs` (8 × 8 Kacheln, Vierecke, `mtllib`/`usemtl`,
`map_Kd` mit PNG, `vn`, `s off`) folgt der öffentlichen Beschreibung dieses Exports, nicht
PDSMS-Code.

## Beispiele

`model_import::samples::write_samples(dir)` erzeugt alle Testdateien aus Code: Würfel-OBJ,
Ebene als `.glb` und `.gltf`, OBJ im PDSMS-Stil, ein zu großes OBJ (64 × 64 Vierecke), ein PNG
mit 40 Farben. `g4k import-samples <dir>` schreibt sie für Szenarien und den Browser. Keine
dieser Dateien liegt im Repo, keine enthält Spielgrafik.

## Verifikation

- Kernel-Tests in `model_import` (Leser, Netz, Texturplan, Abbildung der Bäcker-Meldungen) und
  in `map_import` (`inspect_converts_a_cube_obj_without_writing`,
  `import_writes_model_and_pack_as_one_undo_step`, `the_plane_glb_and_gltf_give_the_same_model`,
  `the_oversize_obj_is_refused_with_the_number`, `the_pdsms_style_obj_imports`,
  `pngs_are_one_textures_part_appended_in_one_step`, `model_and_textures_together_are_one_step`,
  `reimporting_the_cube_reuses_its_texture`, `unchanged_maps_stay_byte_identical` u. a.) auf
  dem synthetischen Projekt mit echten Texturpaketen (`test_support::write_texture_packs`).
- Der Studio-Test `every_part_code_has_its_german_sentence` hält `PART_TEXTS` gleich
  `PartCode::ALL`.
- Das Gate (`scripts/gate.sh`) führt diese Tests in seinen vorhandenen Schritten mit.
