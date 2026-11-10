# Tiled-Karte (TMJ, Austausch einer Welt-Karte, Phase 6.5b)

**In English:** Documents the exchange of a world-format map with Tiled through TMJ, the JSON form
of a Tiled map; this is about Gen4 Studio itself. A map can be written as TMJ together with its tile
images, painted in Tiled and read back in. TMJ is an exchange format and not the world format: the
source stays the world document in `data/maps/NNNN.json`, and the TMJ carries heights, ramps,
partial models and permissions only as layers and properties. The code is
`crates/g4-kernel/src/world/tmj.rs` and `world/tmj/types.rs`, with the commands in
`apps/studio/src-tauri/src/commands/world_map.rs` and the interface in the import and export dialogs
of the Inspector (phase 6.5b).

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Eine [Welt-Karte](world_map.md) lässt
sich als Tiled-Karte (TMJ, die JSON-Form von Tiled) mit ihren Kachelbildern schreiben, in Tiled
malen und wieder einlesen. **TMJ ist ein Austausch mit Tiled, kein Weltformat:** die Quelle bleibt
das Weltdokument in `data/maps/NNNN.json`; Höhe, Rampen, Teilmodelle und Permissions stehen im TMJ
nur als eigene Ebenen und Eigenschaften, die Bilder sind nur fürs Auge, die Daten sind die
Kachelindizes ([Gutachten C](../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §5.4:
„eigenes JSON als Weltformat; TMJ nur Export/Import“). Quelle: [Spezifikation Phase 6](../superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md)
§3.5, Plan [2026-11-07-phase-6-5b-freiform-austausch.md](../superpowers/plans/2026-11-07-phase-6-5b-freiform-austausch.md)
(„Entscheidungen“, Zeilen „TMJ-…“, „Ebenen im TMJ“, „Drehen und Spiegeln“, „Tileset im TMJ“,
„Byte-Gleichheit“) und das dokumentierte TMJ-Format (doc.mapeditor.org). Implementierung:
`crates/g4-kernel/src/world/tmj.rs` (Abbildung, Export, Leser, Import), die serde-Typen in
`world/tmj/types.rs`; Befehle in `apps/studio/src-tauri/src/commands/world_map.rs`, Oberfläche
in `workbench/inspector/{ImportPartsDialog,ExportPartDialog}.tsx`.

**Lizenz.** Tiled und seine Plugins stehen unter GPL: Gen4 Studio nutzt nur das dokumentierte
Format, **kein Code aus Tiled**; der Leser ist eigener Code über serde, ohne Crate (`libtiled`
wird nicht gebraucht). Gelesen und geschrieben wird eine feste Teilmenge; unbekannte Felder, die
Tiled schreibt (`class`, `parallaxx`, `tintcolor` …), ignoriert der Leser, fehlende füllt er mit
Vorgaben.

## Export (`world_export_tmj`)

`world_export_tmj {map, path}` schreibt die Welt als TMJ nach `path` und die Kachelbilder als PNG in
den Ordner `<stem>.tiles/` daneben (`<stem>` der Dateiname ohne Endung); im Projekt ändert sich
nichts. Erst die Bilder, dann die TMJ; der Ordner `<stem>.tiles` entsteht mit allen fehlenden Ordnern
darüber (`create_dir_all`), also auch der Ordner der TMJ. Bilder, die schon in `<stem>.tiles/` liegen, bleiben liegen, auch wenn die neue
Karte sie nicht mehr nennt. Antwort `{map, path, layers, images, pieces, overrides}` (die Zahl der
Ebenen der TMJ, der geschriebenen Bilder, der Teilmodelle und der Überschreibungen). Abgelehnt:
eine Karte ohne Welt („Karte 0123 ist keine Welt-Karte; erst „Welt anlegen“ (world_create).“),
ein Pfad ohne Dateinamen („… hat keinen Dateinamen für TMJ.“), eine Welt, die sich nicht in Zellen
liest („Karte 0123: <Pfad>: <Satz>“ mit dem ersten Fehler); außerhalb eines Textprojekts lehnt
die App vorher ab („Boden malen gibt es im Textprojekt (Technik › Textprojekt).“). In der App ist
das das Segment „Tiled (TMJ)“ im Dialog „Teil exportieren“ einer Welt-Karte (Vorgabe `0123.tmj`,
Satz „Kachelbilder kommen in den Ordner 0123.tiles daneben.“).

Geschrieben wird mit `serde_json::to_vec_pretty` und einem Zeilenvorschub am Ende, die Schlüssel in
Tileds alphabetischer Reihenfolge (die Reihenfolge der Felder der Typen); jede GID steht auf einer
eigenen Zeile.

## Karte

| Schlüssel | Wert |
|---|---|
| `compressionlevel` | −1 |
| `height`, `width` | 32 (Zellen) |
| `infinite` | `false` |
| `nextlayerid`, `nextobjectid` | die nächste freie ID (Ebenen und Objekte zählen je ab 1 in Dateireihenfolge) |
| `orientation` | `orthogonal` |
| `renderorder` | `right-down` |
| `tiledversion` | `1.10.2` (`TILED_VERSION`) |
| `tileheight`, `tilewidth` | 16 (`TMJ_TILE`: ein Pixel je Welteinheit, 16 je Kachel) |
| `type` | `map` |
| `version` | `1.10` (`TMJ_FILE_VERSION`) |
| `properties` | `g4.format` (Text `gen4-studio-world`, `TMJ_FORMAT`), `g4.version` (Zahl 1, `TMJ_VERSION`), `g4.map` (die Karte), `g4.tileset` (das Paket, `world.tileset`), `g4.derive` (Wahrheitswert, `world.permissions.derive`) |

## Ebenen

In dieser Reihenfolge; jede Ebene trägt Eigenschaften `g4.*`, an denen der Leser sie erkennt.

| Ebene | Art | Eigenschaft | Inhalt |
|---|---|---|---|
| je Welt-Ebene, ihr Name | `tilelayer` | `g4.role` `ground` oder `overlay` | je Zelle die GID der Kachel (unten), 0 für `.` |
| „Höhe“ | `tilelayer` | `g4.kind` `heights` | je Zelle die Kachel der Höhenstufe im Tileset „g4 Höhen“: GID = erste GID + (h + 16) |
| „Rampen“, nur wenn `layers[0].ramps` da ist | `tilelayer` | `g4.kind` `ramps` | je Zelle die Kachel der Rampe im Tileset „g4 Rampen“, 0 für keine |
| „Teilmodelle“ | `objectgroup` (`draworder` `topdown`) | `g4.kind` `pieces` | je Teilmodell ein Kachel-Objekt: `gid` = Kachel + 1 (ohne Drehbits), `x` = `at[0]` · 16, `y` = (`at[1]` + 1) · 16 (Tiled setzt Kachel-Objekte an ihre linke untere Ecke), 16 × 16, Eigenschaft `g4.rotate` (Zahl 0, 90, 180, 270) |
| „Permissions“, nur wenn `permissions.override` da ist | `objectgroup` | `g4.kind` `overrides` | je überschriebene Zelle ein Rechteck 16 × 16 auf der Zelle (`x` = Spalte · 16, `y` = Zeile · 16) mit den Zahlen `g4.kind` und `g4.collision` |
| „Gebäude (nur Ansicht)“ | `objectgroup` | `g4.kind` `buildings` | je Gebäude der Karte ein Punkt bei `position / 4096 + 256` (x und z, in Pixeln von der Kartenecke) mit `g4.model_id` |

Ob „Rampen“ und „Permissions“ fehlen oder leer da sind, folgt dem `Option` im Weltdokument; so
bleibt der Rückweg byte-gleich. Jede Kachelebene hat `opacity` 1, `visible` `true`, `x` und `y` 0,
die Daten als Liste von GIDs (keine Kodierung, keine Kompression).

## GIDs und Tilesets

Drei eingebettete Bildsammlungen (`columns` 0, `grid` 16 × 16 orthogonal, `margin` und `spacing`
0) mit **festen** ersten GIDs; jede trägt die Eigenschaft `g4.kind`, jede Kachel
`{id, image, imagewidth, imageheight, properties: [g4.texture, g4.role]}`:

| Tileset | `firstgid` | `g4.kind` | Kacheln | `g4.texture` / `g4.role` | Bild |
|---|---|---|---|---|---|
| „g4 Paket 0006“ | 1 | `tiles` | N = die Kacheln des Tilesets, Kachel `i` = GID `i + 1` | Texturname / Rolle der Kachel | `tile-XXXX.png` (`XXXX` der Index mit vier kleinen Hex-Ziffern, wie das Token): die Textur aus `tileset_texture` in ihrer dekodierten Größe (etwa 32 × 32), sonst ein Platzhalter 16 × 16 in der Farbe der Rolle |
| „g4 Höhen“ | 1 + N | `heights` | 79 (`HEIGHT_TILES`, Stufen −16 bis 62) | Bildname / `height` | `height-m016.png` bis `height-p062.png`: die sieben cividis-Stufen von `szene/hoehe`, reihum wie die Höhenschicht der Szene |
| „g4 Rampen“ | 1 + N + 79 | `ramps` | 8 (`RAMP_TILES`: `n1 n2 e1 e2 s1 s2 w1 w2`) | Bildname / `ramp` | `ramp-n1.png` bis `ramp-w2.png`: eine Farbe je Richtung, Stufe 2 dunkler (3/5 der Helligkeit) |

Beide Hilfs-Tilesets stehen **immer** in der Datei, auch ohne Rampen; nur die Ebenen „Rampen“ und
„Permissions“ hängen an der Welt. `tilewidth` und `tileheight` eines Tilesets sind die des
größten Bilds. Die Platzhalterfarben der Rollen sind die Szenenfarben `szene/gelaende`
(`ground` `unbekannt`, `wall` `blockiert`, `ramp` `vorsprung`, `water` `wasser`, `grass`
`hohes-gras`, `piece` `spezial`). Kacheln mit derselben Textur bekommen dasselbe Bild (die Palette
der ersten Kachel mit diesem Namen, wie `tileset_texture` sie wählt). Der Import liest die Bilder
nie.

**Drehen und Spiegeln** (Tileds Bits der GID `H` = `0x8000_0000`, `V` = `0x4000_0000`,
`D` = `0x2000_0000`; Tiled wendet D, dann H, dann V an). Abgeleitet aus `mesh::uv_of`: die Zelle
zeigt Texel `uv_of(x, z)`, also kommt das Bild mit der Umkehrung dieser Abbildung in die Zelle
(`gid_flags`, `flags_turn`, eine Tabelle; der Test hält jede Zeile pixelweise gegen `uv_of`):

| Token | Bits |
|---|---|
| ohne | – |
| `r1` | D · H |
| `r2` | H · V |
| `r3` | D · V |
| `f` | H |
| `r1f` | D · H · V |
| `r2f` | V |
| `r3f` | D |

Das Sechseck-Bit `0x1000_0000` lehnt der Import ab („Sechseck-Drehung gibt es nicht.“). Auf
„Höhe“ und „Rampen“ bedeuten die Bits nichts und werden übergangen.

## Import (`world_inspect_tmj`, `world_import_tmj`)

`world_inspect_tmj {map, path}` liest die Datei gegen die Welt der Karte und ändert nichts: Antwort
`{map, layers, cells, notes, errors, change: null, bake}` mit den Ebenen der Welt nach dem Import,
den Zellen, in denen sich eine Ebene, die Höhe, die Rampe oder eine Überschreibung ändert, den
Hinweisen (Codes), den Fehlern `[{path, message}]` und dem Backen der Welt, die sich ergäbe (im
Speicher; `null`, wenn sich nichts ändert oder die Datei Fehler hat). `world_import_tmj` prüft
dasselbe und macht daraus **einen** Undo-Schritt „Tiled importiert: Karte 0123“ (`edit_world`,
backt im selben Schritt; ein abgelehntes Backen steht in `bake.messages` und ist kein Fehler des
Aufrufs, wie bei einem Strich). Eine Datei, die die Welt nicht ändert, ist **kein** Schritt
(`change: null`). Eine Datei mit Fehlern lehnt der Import ab, mit dem ersten Fehler und der Zahl:
„layers[0].data[0]: GID 500 gehört nicht in diese Ebene. (2 Fehler)“. Abgelehnt außerdem: eine
Karte ohne Welt, eine gesperrte Karte; außerhalb eines Textprojekts lehnt die App vorher ab. In
der App ist das die Zeile „Tiled (TMJ)“ im Dialog „Teile importieren…“ einer Welt-Karte
(„2 Ebenen, 17 Zellen geändert“, „Keine Änderung.“ oder die ersten drei Fehler).

**Was woher kommt.** Aus der Datei: die Ebenen, Höhen und Rampen, die Teilmodelle, die
Überschreibungen und `g4.derive` (fehlt die Eigenschaft: der Wert der Welt). Aus der Ziel-Welt:
`version`, `tileset`, `freeform` und `terrain`. **Ausnahme Terrain:** weichen die Höhen oder
Rampen der Datei von denen der Welt ab, fällt das Verbatim-Terrain weg (`terrain` wird `null`),
dieselbe Regel wie eine im Studio gemalte Höhe: die BDHC kommt dann aus dem Raster. Die Token
schreibt der Leser kanonisch (`tile_token`, `ramp_token`, `override_token`, die Höhe als Zahl).
Danach prüfen `WorldGrid::parse` und die Kachelprüfung des Tilesets; ihre Fehler kommen mit dem
Pfad der Welt (`world.layers[0].tiles[3]: …`).

**Welche Ebene was ist.**

- Jede `tilelayer`, deren `g4.kind` nicht `heights` oder `ramps` ist (auch ohne `g4.kind`), ist eine Welt-Ebene, in Dateireihenfolge: die erste ist der
  Boden, jede weitere eine Überlagerung, was `g4.role` auch sagt. Eine in Tiled neu angelegte Ebene
  ohne Eigenschaften wird so eine Überlagerung.
- `g4.kind` `heights` und `ramps` sind die Hilfsebenen; ihre erste GID kommt aus dem Tileset mit
  demselben `g4.kind` (nicht aus den festen Zahlen). Eine zweite „Höhe“ oder „Rampen“ wird auf
  Fehler gelesen, es gilt die erste. Ohne „Rampen“ hat die Welt keine Rampen.
- `objectgroup` mit `g4.kind` `pieces` und `overrides` werden gelesen; ohne Gruppe
  `overrides` hat die Welt keine Überschreibungen. Überschreibungen sind Rechtecke aus ganzen
  Zellen; ein größeres Rechteck überschreibt mehrere Zellen.
- Bildebenen, Tiled-Gruppen und andere Objektgruppen (auch „Gebäude (nur Ansicht)“) liest der
  Import nicht.

**Gebäude** bearbeitet das Studio selbst: der Import übernimmt sie nie. Weichen die Punkte der
Ebene `g4.kind` `buildings` (Lage und `g4.model_id`) von den Gebäuden der Karte ab, auch weil die
Ebene gelöscht ist, steht in `notes` der Code `tmj.buildings-ignored`; die App zeigt dazu „Die
Gebäude der Tiled-Datei weichen von denen der Karte ab und werden nicht übernommen; Gebäude
bearbeitest du im Studio.“

**Daten einer Kachelebene:** eine Liste von GIDs oder Base64 (je GID 4 Byte little-endian). Eine
leere `compression` gilt als keine; jede andere lehnt der Import ab.

**Nicht unterstützt** (still übergangen, kein Fehler): die Drehung `rotation` eines Objekts und
die Dreh- und Spiegelbits in der `gid` eines Teilmodell-Objekts (die Drehung eines Teilmodells
steht nur in `g4.rotate`), Breite und Höhe eines Teilmodell-Objekts, `g4.map` (wird nicht mit der
Karte verglichen), `g4.role` einer Welt-Ebene (die Reihenfolge entscheidet), die Eigenschaften der
Kacheln eines Tilesets und die Bilder.

### Fehler

Alle Fehler werden gesammelt, jeder mit seinem Pfad in der Datei (`TmjError {path, message}`).
Ein Fehler im Kopf (Größe, Kachelgröße, `infinite`, Tilesets, Paket) hält das Lesen der Ebenen an;
ist `type` nicht `map`, bleibt es bei diesem einen Fehler. Eine Datei, die kein JSON in TMJ-Form
ist, ist ein Fehler mit dem Satz des Parsers und leerem Pfad.

| Pfad | Satz |
|---|---|
| `type` | „Keine Tiled-Karte (`type` ist nicht „map“).“ |
| `width` / `height` | „Die Karte ist 40 × 32 Kacheln groß; eine Karte hat 32 × 32.“ |
| `tilewidth` / `tileheight` | „Kacheln sind 32 × 32 Pixel; das Studio schreibt 16 × 16.“ |
| `infinite` | „Unendliche Karten liest das Studio nicht.“ |
| `properties` | „Die Datei nutzt Paket 0006, die Welt Paket 0007.“ (nur wenn `g4.tileset` da ist) |
| `tilesets[i].source` | „Ausgelagerte Tilesets (`source`) liest das Studio nicht; in Tiled das Tileset einbetten.“ |
| `tilesets` / `tilesets[0].tilecount` | „Das Tileset hat 5 Kacheln, Paket 0006 hat 6.“ |
| `tilesets[0].firstgid` | „Das erste Tileset beginnt bei GID 2; das Studio schreibt 1.“ |
| `layers` | „Die Ebene „Höhe“ fehlt.“ |
| `layers[i].compression` | „Komprimierte Ebenen liest das Studio nicht; in Tiled unter Karte › Eigenschaften „Kachelebenenformat“ CSV wählen.“ |
| `layers[i].data` | „Die Ebene hat 1000 Zellen; eine Karte hat 32 × 32.“, „Die Ebenendaten sind kein Base64 (…).“, „Die Ebenendaten sind 1023 Bytes lang, keine Folge von GIDs zu je 4 Bytes.“ |
| `layers[i].data[c]` | „GID 500 gehört nicht in diese Ebene.“, „Sechseck-Drehung gibt es nicht.“, auf „Höhe“ „Zelle (3, 4) hat keine Höhe.“ |
| `layers[i].objects[j]` | „Objekt 7 liegt nicht auf dem Raster.“, „Objekt 7: Kachel 0x0001 ist kein Teilmodell.“ (ohne GID „Kachel –“), „Objekt 7: `g4.rotate` ist keine Drehung (0, 90, 180 oder 270).“, „Objekt 7: `g4.kind` und `g4.collision` brauchen Zahlen von 0 bis 255.“, „Zelle (3, 4) ist zweimal überschrieben.“ |
| `world.…` | was `WorldGrid::parse` und die Kachelprüfung nach der Abbildung ablehnen ([world_map.md](world_map.md) „Grammatik der Zellen“, „Prüfungen“) |

Die Zahlen der Tabelle sind Muster. Ein Objekt liegt auf dem Raster, wenn seine Lage (und bei
einer Überschreibung Breite und Höhe) ganze Vielfache von 16 Pixeln zwischen 0 und 4 096 sind und
es in der Karte bleibt; ein Teilmodell braucht eine Kachel der Rolle `piece`.

Die Kachelzahl folgt dem Paket: bringt eine Freiform eine neue Textur ins Paket, hat das Tileset
danach eine Kachel mehr (gesehen am abgeleiteten Tileset von Paket 0006 ohne Tileset-Datei:
78 → 79, die Textur als Kachel der Rolle `ground`). Eine TMJ, die vorher exportiert wurde, lehnt der
Import dann mit „tilesets[0].tilecount: Das Tileset hat 78 Kacheln, Paket 0006 hat 79.“ ab; nach
dem Hinzufügen von Texturen die TMJ neu exportieren (Befund N3 in
[phase-6-5b-gate.md](../phase-6-5b-gate.md)).

## Rückweg ohne Änderung (Round-Trip)

Für eine **kanonische** Welt (Token, wie die Writer sie schreiben: jede Welt, die ein Store nach
einer Änderung speicherte, jede Welt des Eigenschaftstests) gilt
`from_tmj(to_json(to_tmj(w)), w) == w`; ein Import ohne Änderung ist kein Schritt, die Karten-JSON
bleibt **byte-gleich** (Gate 6.5b). Ein von Hand groß geschriebenes Token kommt klein zurück.
Tests: `round_trip_is_identity`, `prop_round_trip_is_identity` (128 Fälle) und
`import_of_an_unchanged_export_is_byte_equal_on_disk` in `world/tmj/import_tests.rs`; der gated
Test `tmj_round_trip_of_the_painted_map_and_every_adopted_map` (`crates/g4-kernel/tests/world_freeform.rs`)
exportiert und importiert die gemalte Karte (Datei byte-gleich nach dem Speichern) und jede andere
Karte nach Form 1: Platin DE `painted=equal maps=665 equal=665`, HeartGold DE und US je
`painted=equal maps=675 equal=675`.

## Beispiel

Karte 0 als Welt aus `world::doc::fixtures::one_tile_world()` (32 × 32 Zellen `0000` auf Höhe 0,
keine Rampe, keine Überschreibung, `derive: true`) auf dem synthetischen Tileset
(`Tileset::synthetic()`, sechs Kacheln, Paket 0), ohne Gebäude, `to_tmj` mit Stamm `0000` und
ohne Bilder (Platzhalter). **Gekürzt:** von den 1 024 GIDs einer Ebene steht je eine da (`…` für
die übrigen, alle gleich), von den Kacheln der Tilesets nur die erste; die Datei schreibt jede GID
auf eine eigene Zeile.

```json
{
  "compressionlevel": -1,
  "height": 32,
  "infinite": false,
  "layers": [
    {
      "data": [1, "…"],
      "height": 32,
      "id": 1,
      "name": "Boden",
      "opacity": 1.0,
      "properties": [{ "name": "g4.role", "type": "string", "value": "ground" }],
      "type": "tilelayer",
      "visible": true,
      "width": 32,
      "x": 0,
      "y": 0
    },
    {
      "data": [23, "…"],
      "height": 32,
      "id": 2,
      "name": "Höhe",
      "opacity": 1.0,
      "properties": [{ "name": "g4.kind", "type": "string", "value": "heights" }],
      "type": "tilelayer",
      "visible": true,
      "width": 32,
      "x": 0,
      "y": 0
    },
    {
      "draworder": "topdown",
      "id": 3,
      "name": "Teilmodelle",
      "objects": [],
      "opacity": 1.0,
      "properties": [{ "name": "g4.kind", "type": "string", "value": "pieces" }],
      "type": "objectgroup",
      "visible": true,
      "x": 0,
      "y": 0
    },
    {
      "draworder": "topdown",
      "id": 4,
      "name": "Gebäude (nur Ansicht)",
      "objects": [],
      "opacity": 1.0,
      "properties": [{ "name": "g4.kind", "type": "string", "value": "buildings" }],
      "type": "objectgroup",
      "visible": true,
      "x": 0,
      "y": 0
    }
  ],
  "nextlayerid": 5,
  "nextobjectid": 1,
  "orientation": "orthogonal",
  "properties": [
    { "name": "g4.format", "type": "string", "value": "gen4-studio-world" },
    { "name": "g4.version", "type": "int", "value": 1 },
    { "name": "g4.map", "type": "int", "value": 0 },
    { "name": "g4.tileset", "type": "int", "value": 0 },
    { "name": "g4.derive", "type": "bool", "value": true }
  ],
  "renderorder": "right-down",
  "tiledversion": "1.10.2",
  "tileheight": 16,
  "tilesets": [
    {
      "columns": 0,
      "firstgid": 1,
      "grid": { "height": 16, "orientation": "orthogonal", "width": 16 },
      "margin": 0,
      "name": "g4 Paket 0000",
      "properties": [{ "name": "g4.kind", "type": "string", "value": "tiles" }],
      "spacing": 0,
      "tilecount": 6,
      "tileheight": 16,
      "tiles": [
        {
          "id": 0,
          "image": "0000.tiles/tile-0000.png",
          "imagewidth": 16,
          "imageheight": 16,
          "properties": [
            { "name": "g4.texture", "type": "string", "value": "syn_boden" },
            { "name": "g4.role", "type": "string", "value": "ground" }
          ]
        },
        "…"
      ],
      "tilewidth": 16
    },
    { "firstgid": 7, "name": "g4 Höhen", "tilecount": 79, "…": "…" },
    { "firstgid": 86, "name": "g4 Rampen", "tilecount": 8, "…": "…" }
  ],
  "tilewidth": 16,
  "type": "map",
  "version": "1.10",
  "width": 32
}
```

Höhe 0 ist GID 7 + 16 = 23 („g4 Höhen“ beginnt nach den sechs Kacheln bei 7), die Kachel `0000`
GID 1. Geschrieben werden 93 Bilder (6 + 79 + 8) nach `0000.tiles/`; der Bericht nennt 4 Ebenen,
0 Teilmodelle und 0 Überschreibungen. Ein Import dieser Datei auf dieselbe Welt ändert nichts
(`change: null`).

## Was nicht dazugehört

TMJ als Weltformat, TMX (XML), Tiled-„Worlds“ über mehrere Karten, Wang-Sätze im TMJ, Events im
TMJ, Bilder beim Import lesen, Freiform-Modelle und Verbatim-Teile im TMJ (sie bleiben aus der
Ziel-Welt), eine Tiled-Erweiterung und Automapping (Plan, „Nicht in 6.5b“).
