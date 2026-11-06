# Welt-Karte (Weltdokument einer Karte, Phase 6.3a)

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Eine Karte des Textprojekts kann
statt nur ihrer Binärteile ein **Weltdokument** tragen, das Objekt `world` in
`data/maps/NNNN.json` mit `source: "world"`. Es beschreibt den Boden als Raster aus Kacheln eines
Kartentextur-Pakets, mit Höhen, Rampen, Überlagerungen, Teilmodellen und Permissions, als Text,
den Git zeilenweise vergleicht. Quelle: [Spezifikation Phase 6](../superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md)
§3.1 und §3.5, [Gutachten C](../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §5.2,
[entscheidungen.md](../research/2026-11-05-phase-6/entscheidungen.md) („Malen, Weltformat,
Import, Tiled“), Plan [2026-11-05-phase-6-3a-weltformat.md](../superpowers/plans/2026-11-05-phase-6-3a-weltformat.md)
(„Entscheidungen“). Implementierung: `crates/g4-kernel/src/world/` (`doc.rs` serde-Form, `grid.rs`
Grammatik und Raster, `mesh.rs` Vernetzer, `edit.rs` Anlegen und Ändern, `keep.rs` Mitnehmen beim
Schreiben, `check.rs` Prüfungen, `places.rs` Paket und Karten); das Tileset, auf das die Kacheln
zeigen, steht in [tileset.md](tileset.md). Alle Beispiele stammen aus synthetischen Welten
(`world::doc::fixtures`, Tileset `Tileset::synthetic()`), nicht aus einer ROM.

**Grenze von 6.3a.** Das Weltdokument fließt noch in **keinen Member**. Die Pflichtfelder der Karte
(`permissions`, `buildings`, `buildings_tail?`, `model`, `bdhc`, `bgs?`) bleiben, was sie in
[text_project.md](text_project.md) sind: der bisherige Member, aus dem Import und Build die Karte
bauen, Byte für Byte wie ohne Welt. Die Szene zeigt das Netz der Welt statt des Kartenmodells
(„Vorschau“ unten), der Check meldet `world.not-baked`. Den Member aus der Welt backt 6.3b.

## Lage in der Karten-Datei

`world` steht direkt nach `source`; `source` ist genau dann `"world"`, wenn `world` da ist. Sonst
lehnen Import und Check mit `textproject.map-source` ab: „source: „world“ braucht das Objekt
`world`.“ bzw. „world: nur mit source „world“.“ Alle Typen des Weltdokuments lehnen unbekannte
Schlüssel ab (`deny_unknown_fields`); ein Formfehler ist `textproject.schema` mit dem Pfad
`world.…`, Grammatik und Bereiche prüft `world.schema` (siehe „Prüfungen“).

## Schlüssel

Reihenfolge wie in der Tabelle (die des Typs; geschrieben als kanonisches JSON wie jede Datei des
Textprojekts).

| Schlüssel | Typ | Inhalt |
|---|---|---|
| `version` | Zahl | immer `1` (`WORLD_VERSION`); eine andere: „Version 2 kennt diese Fassung nicht (1).“ |
| `tileset` | Zahl `u16` | das Kartentextur-Paket (`map_tileset` der Gebietsdaten), auf dessen Kacheln die Indizes zeigen |
| `layers` | Liste | Ebene 0 ist die einzige Bodenebene (`role: "ground"`), jede weitere eine Überlagerung (`"overlay"`) |
| `layers[].name` | Text | frei („Boden“ bei „Welt anlegen“) |
| `layers[].role` | `ground` \| `overlay` | die erste Ebene `ground`, alle anderen `overlay` („die erste Ebene ist die einzige Bodenebene (`ground`).“) |
| `layers[].tiles` | 32 Texte | 32 Zeilen (Nord nach Süd, `z` 0 bis 31) zu 32 Kachel-Token (West nach Ost, `x` 0 bis 31), getrennt durch ein Leerzeichen |
| `layers[0].heights` | 32 Texte | Pflicht auf der Bodenebene: die Höhenstufe jeder Zelle |
| `layers[0].ramps` | 32 Texte | nur Bodenebene, weggelassen ohne Rampe |
| `pieces` | Liste | Teilmodelle `{tile, at: [x, z], rotate}`; weggelassen, wenn leer |
| `freeform` | Liste | Freiform-Modelle `{glb? \| nsbmd?, at: [x, y, z]}`; weggelassen, wenn leer, nur Schema |
| `permissions` | Objekt | `{derive, override?}` |

Eine Überlagerung mit `heights` oder `ramps` ist ein Fehler („nur die Bodenebene trägt Höhen und
Rampen.“): Überlagerungen folgen Höhe und Rampe der Bodenebene. Brücken und zwei Ebenen an einer
Stelle gibt es in Phase 6 nicht.

## Grammatik der Zellen

Jede Zeile hat genau 32 Token, jede Liste genau 32 Zeilen; sonst „31 Zeilen statt 32.“ bzw.
„31 Zellen statt 32.“ am Pfad der Liste oder Zeile. Gelesen wird an beliebigem Leerraum, geschrieben
mit einem Leerzeichen.

| Liste | Token | Bedeutung |
|---|---|---|
| `tiles` | `.` | leere Zelle: keine Fläche, keine Wand |
| | `0012` | Kachel 0x0012 des Tilesets (genau vier Hex-Ziffern) |
| | `0012r1`, `r2`, `r3` | dazu 1 bis 3 Vierteldrehungen gegen den Uhrzeigersinn, von oben gesehen |
| | `0012f`, `0012r2f` | dazu gespiegelt an der z-Achse, **nach** der Drehung |
| `heights` | `0`, `-3`, `62` | Höhenstufe, ganze Zahl von −16 bis 62 (eine Stufe = 8 Welteinheiten) |
| `ramps` | `.` | keine Rampe |
| | `n1`, `e2`, `s1`, `w2` | Rampe bergauf nach Nord, Ost, Süd oder West mit 1 oder 2 Stufen |
| `permissions.override` | `.` | keine Überschreibung |
| | `0280` | `kkcc`: `kind` 0x02 und `collision` 0x80 wie die Zeilen von `permissions` |

**Lesen und Schreiben.** Kachel und Override werden in jeder Schreibweise gelesen (`00AB`,
`0012R1F`) und klein geschrieben. Höhe und Rampe teilen sich mit den Zeilen von 6.2s
`set_map_terrain` ([terrain.md](terrain.md), `terrain::parse_grid`) **einen** Lexer
(`world::grid::lex_height`, `lex_ramp`): gelesen werden auch die Schreibweisen von 6.2, ein
Vorzeichen und führende Nullen (`+5`, `007`, `e01`); geschrieben wird die kanonische Form (`5`,
`7`, `e1`). Die Rampe liest nur Kleinbuchstaben. `world_edit` schreibt das gesetzte Token
kanonisch; die übrigen Token einer berührten Zeile behalten ihre Schreibweise, die Zeile wird mit
einem Leerzeichen neu gefügt, jede andere Zeile bleibt Byte für Byte.

Wortlaut der Token-Fehler (je Zeile der erste, „Zelle x: …“ am Pfad der Zeile):

- „`00g2` ist keine Kachel (`.` oder vier Hex-Ziffern, dahinter `r1` bis `r3` und `f`).“
- „`x` ist keine Höhe (ganze Zahl von −16 bis 62).“, „`70` liegt nicht in −16 bis 62.“
- „`n3` ist keine Rampe (`.` oder `n`, `e`, `s`, `w` mit 1 oder 2 Stufen).“
- „`zz00` ist keine Permission (`.` oder `kkcc` aus vier Hex-Ziffern, Art und Kollision).“

## Höhen und Rampen

Eine Zelle auf Stufe `h` liegt eben auf `y = 8 h` Welteinheiten (−128 bis 496). Eine Rampe nennt die
Richtung **bergauf**: die Zelle liegt an der gegenüberliegenden Kante auf `8 h`, an der genannten
Kante auf `8 (h + s)`. `East(1)` bei `h = 0`: Westkante 0, Ostkante 8 Einheiten. Die Ecken
`[nw, ne, sw, se]`:

| Rampe | nw | ne | sw | se |
|---|---|---|---|---|
| keine | `8h` | `8h` | `8h` | `8h` |
| `n s` | `8(h+s)` | `8(h+s)` | `8h` | `8h` |
| `s s` | `8h` | `8h` | `8(h+s)` | `8(h+s)` |
| `e s` | `8h` | `8(h+s)` | `8h` | `8(h+s)` |
| `w s` | `8(h+s)` | `8h` | `8(h+s)` | `8h` |

Die Oberkante einer Rampe liegt höchstens auf 62: `h + s ≤ 62`, sonst `world.schema` am Token der
Rampe: „Zelle 5: Die Rampe endet über 62 (Höhe 61, 2 Stufen).“ Das ist derselbe Sinn wie 6.2s
`bdhc::Ramp` (Höhe an der unteren Kante); `world::grid::Ramp` wandelt verlustfrei in beide
Richtungen (`From`), `WorldGrid::height_grid` gibt das Höhenraster von 6.2.

**Laufhöhe in der App.** `get_map` (`MapDto.heights`) gibt für eine Welt-Karte die Höhe der
Kachelmitte aus dem Weltdokument (`HeightGrid::center_units`: `8 h`, auf einer Rampe `8 h + 4 s`),
nicht aus der BDHC des Members; Overlay, Marker und Picking stehen damit auf dem gemalten Boden.
Eine Welt, die sich nicht lesen lässt, behält die Höhen des Members. `get_map_heights` (6.2) liest
bis 6.3b weiter die BDHC des Members.

## Koordinaten und UV

Das Netz liegt in Welteinheiten um die Kartenmitte, wie die Vanilla-Modelle und das GLB der Szene:
x und z von −256 bis 256, 16 Einheiten je Kachel, z wächst nach Süden, y nach oben, `y = Stufe × 8`.
Zelle `(cx, cz)` überdeckt x von `16 cx − 256` bis `16 cx − 240`, z ebenso. Der Bäcker von 6.3b
übernimmt die Ecken ohne Umrechnung.

UV in Texeln, **welt-gebunden**, 1 Texel je Einheit (16 je Kachel); so ändert das Zusammenfassen
von Zellen nichts an der Textur, und ±2 048 Texel (`TEXCOORD`) werden nie erreicht:

| Kachel | `(u, v)` |
|---|---|
| ohne Drehung | `(x, z)` |
| `r1` | `(z, −x)` |
| `r2` | `(−x, −z)` |
| `r3` | `(−z, x)` |
| `f` (nach der Drehung) | `u = −u` |
| Wand | `u` die Koordinate entlang der Kante (x an Nord- und Südkanten, z an West- und Ostkanten), `v = −y` |

Die Szene teilt durch die Größe der Textur: `uv = (u / Breite, −v / Höhe)`, `RepeatWrapping`.

## Teilmodelle und Freiform

**Teilmodell** `{ "tile": 5, "at": [x, z], "rotate": 0 }`: die Kachel muss im Tileset die Rolle
`piece` mit `footprint: [Breite, Tiefe]` haben; `rotate` ist 0, 90, 180 oder 270 (Vorgabe 0, bei 90
und 270 tauschen Breite und Tiefe). `at` liegt auf der Karte (0 bis 31), der Fußabdruck ragt nicht
über den Rand. In 6.3a legt der Vernetzer je Stück eine waagerechte **Fußabdruck-Fläche** über die
Zellen des Abdrucks, auf der höchsten Ecke der Bodenzellen darunter, mit der Textur der Kachel
(gedreht wie das Stück, nicht gespiegelt) und Ebene `PIECE_LAYER = 255`. Die Geometrie des Stücks
(Beifile `NNNN.<name>.glb`) kommt mit 6.5b.

**Freiform** `{ "glb": "…" }` oder `{ "nsbmd": "…" }`, dazu `at: [x, y, z]` in Welteinheiten
(Vorgabe `[0, 0, 0]`): nur das Schema; genau eines von `glb` und `nsbmd` („braucht genau eines von
`glb` und `nsbmd`.“). Gelesen wird es bis 6.5b von nichts.

## Permissions

`{ "derive": true }`: die Permissions kommen aus den Vorgaben der Kacheln im Tileset (`kind`,
`collision`, in HeartGold/SoulSilver `sound`); `override` (32 Zeilen `.` oder `kkcc`) setzt
einzelne Zellen. Mit `derive: false` braucht jede Zelle eine Überschreibung: ohne `override`
„ohne Ableitung (`derive: false`) braucht es `override` mit 32 Zeilen.“, mit offenen Zellen „ohne
Ableitung (`derive: false`) braucht jede Zelle eine Permission; 3 Zellen sind `.`.“

In 6.3a werden diese Permissions gelesen und geprüft, aber nicht angewandt: der Member und das
Studio arbeiten mit den `permissions` der Karten-Datei wie bisher. Ableiten und Anwenden beim
Backen kommt mit 6.3b.

## Beispiel

Karte 0 als Welt aus `world::doc::fixtures::one_tile_world()` (32 × 32 Zellen `0000` auf Höhe 0,
synthetisches Tileset auf Paket 0). Gekürzt: jede Zeile hier zeigt nur ihren Anfang und ihr Ende
(`…`), von den 32 Zeilen einer Liste nur die erste; die Pflichtfelder der Karte stehen wie im
synthetischen Beispiel von [text_project.md](text_project.md).

```json
{
  "source": "world",
  "world": {
    "version": 1,
    "tileset": 0,
    "layers": [
      {
        "name": "Boden",
        "role": "ground",
        "tiles": [
          "0000 0000 0000 … 0000",
          "…"
        ],
        "heights": [
          "0 0 0 … 0",
          "…"
        ]
      }
    ],
    "permissions": {
      "derive": true
    }
  },
  "permissions": ["0080 0100 0200 …", "…"],
  "buildings": [],
  "model": { "hex": "000000000000" },
  "bdhc": { "hex": "cafe01" }
}
```

Diese Welt vernetzt zu **einer** Fläche (512 × 512 Einheiten auf y = 0); der Check meldet
`world.not-baked` und „Geschätzt höchstens 1 680 Byte (3 % von 0xF000) aus 1 Fläche und 1 Textur.“
Das Schachbrett aus `0000` und `0001` (`fixtures::checkerboard_world`) vernetzt zu 1 024 Flächen.
Eine Rampe, eine Überlagerung, ein Teilmodell und eine Überschreibung sähen so aus (Ausschnitte
von Hand, kein ganzes JSON):

```text
"ramps": [". . e1 e1 . … .", "…"],
"…": "…",
{ "name": "Deko", "role": "overlay", "tiles": [". 0002 . … .", "…"] },
"pieces": [{ "tile": 5, "at": [10, 4], "rotate": 90 }],
"permissions": { "derive": true, "override": [". . 0280 … .", "…"] }
```

## Store-Art und Undo

Die Welt einer Karte ist im Store das Asset `AssetKind::WorldMap` (`ALL[29]`, `label` „world map“,
Name des Arbeitsordners `maps+world`), eine Store-Art ohne Archiv wie `Trainer`: Index =
Kartennummer, `member_count(WorldMap) = member_count(Map)`. Seine Bytes sind das kanonische JSON von
`Option<WorldJson>` (eine Karte ohne Welt: `null`), sein Ort `Location::Derived { json }` (kein
Member). Im Textprojekt gehört es mit der Karte zu **einer** Einheit: `place_of(WorldMap i)` ist
`Place::Entity { art: Maps, index: i }`, `unit_of` der Karte nennt `assets = [Map i, WorldMap i]`;
gespeichert wird die eine JSON. Außerhalb eines Textprojekts lehnt das Lesen ab, und Member
schreiben schreibt nichts.

- **„Welt anlegen“** (`world_create`, `{map, tileset?, fill?, height?}`): Bodenebene „Boden“ ganz
  mit `fill` (Vorgabe `0000`) auf `height` (Vorgabe 0), keine Rampe, `permissions { derive: true }`;
  ohne `tileset` das erste Paket der Header, deren Matrix die Karte zeigt (`places::packs_of_map`;
  keins: „Karte 0123 liegt in keiner Matrixzelle mit Header; Tileset angeben (`tileset`).“). Ein
  Undo-Schritt „Welt angelegt: Karte 0123“; ⌘Z setzt die Welt wieder auf `null`.
- **`world_edit`** (`{map, label?, changes: [{layer?, cells: [[x, z], …], tile? | height? | ramp? |
  permission?}]}`): je Änderung genau ein Feld („Eine Änderung setzt genau eines von tile, height,
  ramp, permission.“), Token wie in der Datei; Höhe und Rampe nur auf Ebene 0, `permission` ohne
  `layer`. Alle Änderungen eines Aufrufs sind **ein** Undo-Schritt, Label vorgegeben „Welt
  geändert: Karte 0123 (37 Zellen)“ („(1 Zelle)“). Geschrieben werden nur die berührten Zeilen;
  `ramps` und `override` entstehen bei Bedarf als 32 Zeilen `.` und gehen wieder, wenn die
  Änderung sie ganz leer lässt. Die Welt muss vorher und nachher fehlerfrei lesen; ein Aufruf, der
  nichts ändert: „Nichts zu ändern.“
- **Nur im Textprojekt:** im DSPRE-Layout und in einem Textprojekt anderer Schemaversion lehnen
  `world_create`, `world_edit` und `tileset_save` mit „Boden malen gibt es im Textprojekt (Technik
  › Textprojekt).“ ab; `world_get` antwortet `world: null`, `get_world_mesh` ein leeres Netz,
  `get_map` `world: false`.
- **Gesperrte Karte:** lässt sich der Member einer Karte nicht lesen (sie steht als `{locked, raw}`
  in der Datei), hat sie keinen Platz für eine Welt. `world_create` und `world_edit` lehnen ab, und
  ein Speichern oder Export, das die Karte gesperrt neu schriebe, lehnt ab, statt die Welt zu
  verlieren: „textproject.map-source: Karte 0123 ist eine Welt-Karte, ihr Member lässt sich nicht
  lesen; Schreiben verlöre das Weltobjekt. Erst den Member reparieren (oder die Welt aus der Datei
  sichern).“
- **Mitnehmen beim Schreiben:** der Writer der Karte nimmt `source` und `world` aus dem
  WorldMap-Asset (sonst wie der Store es hält, sonst von der Platte, `world::keep::carry_world`),
  den Rest aus den Map-Bytes; der volle Export nimmt `source`/`world` von der Platte wie den
  Grabstein-Umschlag. Ein gespeicherter Permission-Strich, ein verschobenes Gebäude und jeder
  Export lassen das Weltobjekt Byte für Byte stehen, auch eines, das der Check ablehnt.
- **Neu lesen und Zurücksetzen:** `reload_from_disk` eines WorldMap-Assets liest die JSON neu.
  „Auf Original zurücksetzen“ lehnt ab: „Eine Welt hat kein Original; „Karte als Quelle übernehmen“
  und zurück kommen mit 6.3b.“

Die Befehle für Agenten (über `g4_call`, kuratierte Werkzeuge erst mit 6.4a) stehen in
[mcp.md](../mcp.md), Abschnitt „Welt-Karten (6.3a, über `g4_call`)“.

## Prüfungen

Im Check des Textprojekts (`textproject::check::hints`, `world/check.rs`), nicht im Prüf-Register
(`checks/*`). Jede Meldung `<code>: <JSON-Pfad>: <Satz>` mit Datei, Zeile und Spalte; ein
Zeilenfehler zeigt auf sein Token in der Zeile.

| Code | Schwere | Wortlaut |
|---|---|---|
| `world.schema` | Fehler | Zeilen- und Token-Fehler (siehe „Grammatik“), Bereiche, Rolle der Ebenen, Rampe über 62, `derive: false` ohne volle Überschreibung, Teilmodell (`rotate`, `at`), Freiform; dazu gegen das Tileset: „Zelle 2: Kachel 0x0006 gibt es im Tileset 0000 nicht (6 Kacheln).“ (je Zeile die erste), ein Paket, das es nicht gibt („world.tileset: Kartentextur-Paket 0007 gibt es nicht (1 Paket).“), eine Teilmodell-Kachel, die keine ist („Kachel 0x0001 ist kein Teilmodell (Rolle `grass`).“) oder deren Fußabdruck keine Fläche hat („… (0 × 2).“), ein Stück über dem Rand („Das Teilmodell ragt über den Kartenrand (x 31, Breite 2).“) |
| `world.tileset-mismatch` | Warnung | je Header, dessen Matrix die Karte zeigt und dessen Gebietsdaten ein anderes Paket nennen, an `world.tileset`: „Karte 0000 nimmt Tileset 0000, ihr Header 0001 nennt Gebietsdaten mit Paket 0001.“ |
| `world.not-baked` | Info | je Welt-Karte an `world`: „Karte 0123 ist eine Welt-Karte; bis zum Backen (6.3b) baut der Build ihren bisherigen Member.“ |
| `world.budget` | Info | je fehlerfreier Welt an `world`: „Geschätzt höchstens 1 680 Byte (3 % von 0xF000) aus 1 Fläche und 1 Textur.“; über 0xF000 vor dem Punkt dazu „; mehr als das Modell fassen darf, das Backen (6.3b) lehnt es ab, wenn es nicht kleiner wird“ |

Geprüft werden die Kacheln gegen die Tileset-Datei des Pakets, sonst gegen die Texturen des Pakets
(`textures_only`, [tileset.md](tileset.md)). Der Import lässt eine Karte mit `world.schema`-Fehler
aus (ihr Problem im Plan, wie eine Entität, die sich nicht importieren lässt) und importiert die
übrigen; der Check zeigt jedes Problem einmal. Ein Paket, das nur in der Basis liegt, ist kein
Fehler: der Import stellt seinen Member wieder her.

## Vernetzer

`world::mesh::mesh(grid, tileset)` macht aus dem Raster eine Liste von Vierecken (`MeshFace`:
`corners` 4 × `[x, y, z]`, `uv` 4 × `[u, v]`, `texture` Name im Paket, `layer`), Ecken gegen den
Uhrzeigersinn von ihrer Vorderseite gesehen (Böden von oben: nw, sw, se, ne). Kacheln jenseits des
Tilesets und leere Zellen ergeben keine Fläche.

- **Zusammenfassen:** gierig je Ebene, Zeile für Zeile; Schlüssel ist (Kachel-Token, Höhe, Rampe).
  Erst in x verlängern, dann ganze Zeilen gleicher Breite in z. Rampenzellen fassen nur quer zum
  Hang zusammen (`n`/`s` nur in x, `e`/`w` nur in z). Eine Karte aus einer Kachel wird eine Fläche,
  ein Schachbrett aus zwei Kacheln 1 024.
- **Überlagerungen:** Ebenen ab 1 folgen Höhe und Rampe der Bodenebene, `layer` = Index, keine
  Wände.
- **Wände:** zwischen zwei Bodenzellen, deren Oberkanten an der gemeinsamen Kante verschieden hoch
  liegen, eine senkrechte Fläche vom unteren zum oberen Profil, Vorderseite zur niedrigeren Zelle,
  `layer` 0. Textur: `wall` der Kachel der oberen Zelle im Tileset, ohne `wall` deren eigene
  Kachel. Wandstücke einer Linie mit gleicher Textur, gleicher Richtung und geradem Unter- und
  Oberprofil werden eine Fläche. Keine Wand an der Kartengrenze (Nachbarkarten sieht erst 6.4b), zu
  einer leeren Zelle und wo sich die Profile **kreuzen** (gegenläufige Rampen nebeneinander). Liegt
  eine Rampe neben einer ebenen Zelle, die an einem Ende gleich hoch ist, wird die Wand ein
  **entartetes Viereck**: zwei Ecken fallen zusammen, es ist ein Dreieck.
- **Fußabdrücke:** je Teilmodell eine Fläche auf `PIECE_LAYER` (siehe „Teilmodelle“).
- **Reihenfolge:** Ebene aufsteigend; je Ebene erst die Böden in Reihenfolge ihrer Startzelle
  (Zeile, dann Spalte), auf der Bodenebene danach die Wände (Nordkanten zeilenweise, dann
  Westkanten spaltenweise), zuletzt die Fußabdrücke in Listenreihenfolge. Deterministisch, ohne
  Hash-Reihenfolge.
- **Budget-Schätzung:** `estimate_model_bytes(Flächen, Texturen) = 1 024 + 600 · Texturen +
  56 · Flächen`, eine obere Schranke (Vierecke mit `TEXCOORD` und `VTX_16`; gemessen in
  [Gutachten A](../research/2026-11-05-phase-6/gutachten-a-writer.md) §2.5: 1 024 Flächen mit
  einer Textur 57 776 Byte, geschätzt 58 968). Grenze `MODEL_BUDGET = 0xF000`. Die echte Zahl
  liefert der Bäcker von 6.3b.
- **Zeit:** eine volle Karte (`fixtures::busy_world`) vernetzt im Release-Build in höchstens 20 ms,
  das Netz über die Brücke (`get_world_mesh`, warm) im Debug-Build in höchstens 50 ms (Tests
  `mesh_of_a_full_map_is_fast` in `crates/g4-kernel/tests/world.rs` und
  `world_mesh_over_the_command_path_is_fast` in `commands/world_map.rs`, strikt mit
  `G4_TIMING_STRICT=1`).

## Vorschau in der Szene

`get_world_mesh(map)` gibt das Netz als flache Felder (`WorldMeshDto`: `faces`, `textures` je
`{name, width, height, role, palette?}`, `corners` 12 Werte je Fläche, `uv` 8, `texture` Index in
`textures`, `layer`); `get_tileset_texture(package, texture)` das RGBA einer Textur mit der Palette
ihrer Kachel oder `null`. Die Szene (`apps/studio/src/world/scene/worldMesh.ts`) zeigt für eine
Welt-Karte das Netz **statt des GLB** des Members: eine Geometrie je Textur, je Ebene eine Gruppe,
`MeshLambertMaterial` mit `alphaTest 0.5`. Ohne Textur (synthetisches Tileset, eine Textur im
Format 5, die noch keinen Decoder hat) steht ein Platzhalter in der Farbe der Rolle aus
`szene/gelaende` (`grass` `hohes-gras`, `water` `wasser`, `wall` `blockiert`, `piece` `spezial`,
sonst `unbekannt`). Überlagerungen bekommen `polygonOffset −Ebene`, Fußabdrücke `−(höchste
Überlagerung + 1)` statt −255 und sind halbdurchsichtig (Deckkraft 0,6). Das Canvas trägt
`data-world-mesh="<Karten aufsteigend, Komma>"` und `data-world-mesh-faces="<Summe>"`, solange es
mindestens eine Welt-Karte zeigt.

Der erste Aufbau traut `MapDto.world`; ein Auffrischen (⌘Z, ⇧⌘Z, Änderungen von Agenten) fragt
zuerst `get_world_mesh` und nimmt das Netz, wenn es Flächen hat, sonst das GLB. Vorschaubilder
der Karten kommen bis auf Weiteres aus dem GLB. Gestaltung: [design-system/README.md](../../design-system/README.md),
„Nachtrag 6.3a“.

## Was später kommt

- **6.3b:** der Bäcker macht aus Netz, Tileset und Permissions den Member (Modell, BDHC,
  Permissions), Build-Schritt `world`, Prüfung `world.member-changed`, „Karte als Quelle
  übernehmen“, die echte Modellgröße statt der Schätzung, `get_map_heights` aus der Welt.
- **6.4a:** Malwerkzeuge, Palette, Ebenen und Höhen-Overlay in der App, die kuratierten
  MCP-Werkzeuge (`world_get`, `world_edit`, `tileset_get`), der Tileset-Editor; Werkzeug-Ops
  (`paint`, `rect`, `fill`) rechnet 6.4a auf die Zellen von `world_edit` herunter.
- **6.4b:** Kantensätze (`edge_sets` im Tileset) und Wände zu Nachbarkarten.
- **6.5b:** Beifiles der Teilmodelle (`model`, `NNNN.<name>.glb`) und Freiform-Geometrie.
