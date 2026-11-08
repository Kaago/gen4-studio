# Welt-Karte (Weltdokument einer Karte, Phasen 6.3a, 6.3b, 6.4a und 6.5b)

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Eine Karte des Textprojekts kann
statt nur ihrer Binärteile ein **Weltdokument** tragen, das Objekt `world` in
`data/maps/NNNN.json` mit `source: "world"`. Es beschreibt den Boden als Raster aus Kacheln eines
Kartentextur-Pakets, mit Höhen, Rampen, Überlagerungen, Teilmodellen und Permissions, als Text,
den Git zeilenweise vergleicht. Quelle: [Spezifikation Phase 6](../superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md)
§3.1 und §3.5, [Gutachten C](../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §5.2,
[entscheidungen.md](../research/2026-11-05-phase-6/entscheidungen.md) („Malen, Weltformat,
Import, Tiled“, „Writer“, „BDHC“), Pläne [2026-11-05-phase-6-3a-weltformat.md](../superpowers/plans/2026-11-05-phase-6-3a-weltformat.md)
und [2026-11-06-phase-6-3b-backen-build.md](../superpowers/plans/2026-11-06-phase-6-3b-backen-build.md)
(je „Entscheidungen“), das Malen in der App aus [2026-11-07-phase-6-4a-malen.md](../superpowers/plans/2026-11-07-phase-6-4a-malen.md), Freiform, Teilmodell-Geometrie und Form 2 aus [2026-11-07-phase-6-5b-freiform-austausch.md](../superpowers/plans/2026-11-07-phase-6-5b-freiform-austausch.md). Implementierung: `crates/g4-kernel/src/world/` (`doc.rs` serde-Form, `grid.rs`
Grammatik und Raster, `mesh.rs` Vernetzer, `bake.rs` Backen, `record.rs` Datensatz je Karte,
`build.rs` Build-Schritt `world`, `adopt.rs` „Karte als Quelle übernehmen“, `edit.rs` Anlegen,
Ändern und Aufgeben, `keep.rs` Mitnehmen beim Schreiben, `check.rs` Prüfungen, `places.rs` Paket
und Karten, seit 6.5b `freeform.rs` Beifiles und Freiform, `tmj.rs` Austausch mit Tiled, [tmj.md](tmj.md)); das Tileset, auf das die Kacheln zeigen, steht in [tileset.md](tileset.md). Alle
Beispiele stammen aus synthetischen Welten (`world::doc::fixtures`, Tileset `Tileset::synthetic()`),
nicht aus einer ROM.

**Quelle und Ausgabe (seit 6.3b).** Die Welt ist die Quelle, der Member `unpacked/<land_data>/NNNN`
ihre Ausgabe: das **Backen** macht aus dem Weltdokument das Modell (Flächen des Vernetzers → NSBMD
über den Bäcker von 6.1), das Terrain (Höhenraster → BDHC über den Generator von 6.2) und die
Permissions (Vorgaben der Kacheln, Kantenregel, Handwerte); siehe „Backen“. Seit Schema 4 trägt die
Karten-Datei einer Welt-Karte darum `permissions`, `model` und `bdhc` nicht (`buildings`,
`buildings_tail?`, `bgs?` bleiben Quelle, wie in [text_project.md](text_project.md)). Gebacken wird
bei jedem Strich im Store (im selben Undo-Schritt), beim Speichern, im Build-Schritt `world` und im
Import ([build_state.md](build_state.md)).

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
| `version` | Zahl | immer `2` (`WORLD_VERSION`, seit 6.3b; die Migration 3 → 4 hebt 1 an); eine andere: „Version 1 kennt diese Fassung nicht (2).“ |
| `tileset` | Zahl `u16` | das Kartentextur-Paket (`map_tileset` der Gebietsdaten), auf dessen Kacheln die Indizes zeigen |
| `layers` | Liste | Ebene 0 ist die einzige Bodenebene (`role: "ground"`), jede weitere eine Überlagerung (`"overlay"`) |
| `layers[].name` | Text | frei („Boden“ bei „Welt anlegen“) |
| `layers[].role` | `ground` \| `overlay` | die erste Ebene `ground`, alle anderen `overlay` („die erste Ebene ist die einzige Bodenebene (`ground`).“) |
| `layers[].tiles` | 32 Texte | 32 Zeilen (Nord nach Süd, `z` 0 bis 31) zu 32 Kachel-Token (West nach Ost, `x` 0 bis 31), getrennt durch ein Leerzeichen |
| `layers[0].heights` | 32 Texte | Pflicht auf der Bodenebene: die Höhenstufe jeder Zelle |
| `layers[0].ramps` | 32 Texte | nur Bodenebene, weggelassen ohne Rampe |
| `pieces` | Liste | Teilmodelle `{tile, at: [x, z], rotate}`; weggelassen, wenn leer |
| `freeform` | Liste | Freiform-Modelle `{glb? \| nsbmd?, at: [x, y, z]}`; weggelassen, wenn leer |
| `terrain` | Objekt | `{bdhc: <Teil>}`: das Verbatim-Terrain (Form 1); weggelassen ohne |
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
| | `0012r1`, `r2`, `r3` | dazu 1 bis 3 Vierteldrehungen: von oben gesehen (Norden oben) dreht sich das Bild im Uhrzeigersinn, die Texturkoordinaten gegen ihn (Tabelle unter „Koordinaten und UV“) |
| | `0012f`, `0012r2f` | dazu gespiegelt an der z-Achse, **nach** der Drehung |
| `heights` | `0`, `-3`, `62` | Höhenstufe, ganze Zahl von −16 bis 62 (eine Stufe = 8 Welteinheiten) |
| `ramps` | `.` | keine Rampe |
| | `n1`, `e2`, `s1`, `w2` | Rampe bergauf nach Nord, Ost, Süd oder West mit 1 oder 2 Stufen |
| `permissions.override` | `.` | keine Überschreibung |
| | `0280` | `kkcc`: `kind` 0x02 und `collision` 0x80 wie die Zeilen von `permissions` |

**Lesen und Schreiben.** Kachel und Override werden in jeder Schreibweise gelesen (`00AB`,
`0012R1F`) und klein geschrieben. Höhe und Rampe teilen sich mit den Zeilen von 6.2s
`set_map_terrain` ([terrain.md](terrain.md), `terrain::parse_grid`) **einen** Lexer
(`terrain::lex_height`, `lex_ramp` in `terrain/rows.rs`, als `world::grid::lex_*` re-exportiert): gelesen werden auch die Schreibweisen von 6.2, ein
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
`null` wo die Bodenebene leer ist (`.`); Overlay, Marker und Picking stehen damit auf dem gemalten
Boden. Eine Welt, die sich nicht lesen lässt, behält die Höhen des Map-Assets. Eine Welt mit
Verbatim-Terrain (`world.terrain`, eine übernommene Karte, Form 1) läuft auf dieser BDHC: dann gibt
`get_map` die Höhen der gebackenen BDHC, dieselben wie `get_map_heights`, auch nach gemalten Kacheln
(sie lassen `world.terrain` stehen); erst `height` oder `ramp` beendet das Verbatim-Terrain, ab dann
gilt das Weltdokument (`null` auf `.`).
`get_map_heights` (6.2) liest die BDHC des Map-Assets, also seit 6.3b die gebackene: sie folgt der
Welt (bei Verbatim-Terrain die übernommene BDHC).

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
(gedreht wie das Stück, nicht gespiegelt) und Ebene `PIECE_LAYER = 255`. Diese Fläche wird seit
6.3b mitgebacken wie in der Vorschau, und der Abdruck setzt die Permission der Kachel des Stücks
(„Permissions“). Seit 6.5b kann die Kachel ein Modell tragen (`model` im Tileset); dann steht seine
Geometrie statt der Fläche da („Teilmodelle mit Geometrie (6.5b)“).

**Freiform** `{ "glb": "…" }` oder `{ "nsbmd": <Teil> }`, dazu `at: [x, y, z]` in Welteinheiten
(Vorgabe `[0, 0, 0]`); genau eines von `glb` und `nsbmd` („braucht genau eines von `glb` und
`nsbmd`.“). `nsbmd` ist seit 6.3b das Verbatim-Modell (Form 1) als Teil wie in
[text_project.md](text_project.md) „Teile“ (`{"base_sha1"}`, `{"file": "0123.nsbmd"}`, `{"hex"}`,
Basisschlüssel `maps NNNN model`): höchstens eines („Höchstens ein Verbatim-Modell (`nsbmd`) in
`freeform`.“) an `[0, 0, 0]` („Ein Verbatim-Modell liegt an [0, 0, 0].“). Beides ist
`world.schema` am Pfad des Eintrags (`world.freeform[1]`, `world.freeform[0].at`). `glb` nennt seit
6.5b ein Beifile der Karte und backt mit („Freiform und Beifiles (6.5b)“).

**Terrain** `{ "bdhc": <Teil> }` (nach `freeform`, vor `permissions`; Basisschlüssel
`maps NNNN bdhc`): solange es steht, ist die BDHC der Karte dieser Teil, nicht die aus dem Raster.
`world_edit` mit `height` oder `ramp` entfernt `terrain` im selben Schritt (Label unverändert),
auch wenn die Höhe gleich bleibt; ab dann kommt die BDHC aus dem Raster.

**Verbatim im Store.** Solange die Welt ein Verbatim-Modell bzw. `terrain` trägt, sind Modell bzw.
BDHC des Map-Assets genau diese Bytes (`world::bake::Verbatim::of`); ein Teil wird nie im Store
aufgelöst. Der Writer bildet die Teile mit `part_out` aus den Map-Bytes (dieselbe Regel wie für
binäre Karten: `base_sha1` bei Gleichheit mit der Basisschicht, sonst Beifile oder Hex); so trägt
eine übernommene Vanilla-Karte `base_sha1` und schreibt keine Vanilla-Bytes als Beifile ins
Projekt.

## Freiform und Beifiles (6.5b)

Ein Freiform-Modell ist ein Eintrag `{ "glb": "0123.haus.glb", "at": [x, y, z] }` in `freeform`, der
ein **Beifile** der Karte nennt: `data/maps/NNNN.<name>.glb`, `NNNN` die Karte, `<name>` 1 bis 32
Zeichen aus a–z, 0–9, `_`, `-` (`SIDE_NAME_MAX = 32`). Ein Teilmodell nennt sein Beifile über das
Tileset: `data/tilesets/PPPP.<name>.glb` mit `PPPP` dem Paket („Teilmodelle mit Geometrie (6.5b)“).
Quelle: Plan [2026-11-07-phase-6-5b-freiform-austausch.md](../superpowers/plans/2026-11-07-phase-6-5b-freiform-austausch.md)
(„Entscheidungen“). Implementierung: `world/freeform.rs`.

**Schema** (`world.schema` am Pfad des Eintrags):

- `glb` hat die Form eines Beifile-Namens: „`glb` nennt ein Beifile dieser Karte: `NNNN.<name>.glb`
  mit a–z, 0–9, `_`, `-`.“ (der Leser kennt die Karte nicht und schreibt `NNNN`); nennt ein Name
  dieser Form eine andere Karte, sagt der Check (und der Import des Textprojekts) dasselbe mit der
  Nummer der Karte: „`glb` nennt ein Beifile dieser Karte: `0123.<name>.glb` mit a–z, 0–9, `_`, `-`.“
  (`world.freeform[0].glb`). Die Nummer hat genau vier Ziffern, wie `{:04}` sie schreibt.
- `at` liegt je Achse in −512 bis 511 Welteinheiten („`at` liegt nicht in −512 bis 511.“, nur für
  Einträge ohne `nsbmd`).
- Ob das Beifile da ist, prüft erst das Backen (`world.side-file`).

**Form des Beifiles.** Ein Beifile ist nie die Datei des Nutzers, sondern immer das **kanonische
GLB**, das `side_glb` aus den Flächen des Bäckers schreibt; so backt die Karte genau, was beim
Hinzufügen geprüft wurde, und OBJ-, Blender- und apicula-Eingaben enden in derselben Form:

- eine Primitive je Textur, in der Reihenfolge ihres ersten Auftretens in den Flächen, je Primitive
  ein Material **ohne Bild**, benannt nach der Textur des Pakets (keine `textures`, keine `images`);
  ein Knoten `{mesh: 0}` ohne Transformation; `asset.generator` „gen4-studio“;
- Positionen in Kacheln: Welteinheiten / `FREEFORM_UNIT` (16, eine glTF-Einheit ist eine Kachel);
  UV als Bruchteil der gebundenen Paket-Textur (Texel / Breite bzw. Höhe, t nach unten wie
  [model_import.md](model_import.md)); Indizes als `u32`, Modus Dreiecke;
- je Fläche ihre Ecken ohne eine, die an der Stelle der vorigen liegt (zyklisch; eine Wand neben
  einer Rampe hat zwei gleiche Ecken): vier Ecken sind ein Viereck, geschrieben als die Dreiecke
  **`3 0 1` und `3 1 2`** (so setzt `pair_triangles` beim Lesen das Viereck mit seinen Ecken in
  derselben Reihenfolge wieder zusammen), drei Ecken ein Dreieck; Dreiecke ohne Fläche entfallen.
  Je Primitive stehen die Vierecke vor den einzelnen Dreiecken, damit kein einzelnes Dreieck beim
  Lesen die Hälfte eines Vierecks nimmt;
- ohne Flächen ein gültiges GLB ohne Netz (Lesen gibt `world.side-file`, keine Geometrie).

Gelesen wird ein Beifile mit `read_model` und `to_faces` (Einheit 16), gebunden **nur** an die
Texturen des Pakets `world.tileset`, wie das Backen sie bindet (`world_pack_textures`), nach dem
Namen des Materials (`side_faces`). Die Flächen kommen zurück, wie sie geschrieben wurden (bis auf
Ecken an der Stelle der vorigen), und ein aus dem Gelesenen geschriebenes Beifile hat dieselben
Bytes (Fixpunkt; Tests in `world/freeform.rs`, Eigenschaftstest `prop_side_glb_round_trips`).

**Achsen und Lage** wie das Netz der Welt und [model_import.md](model_import.md): y oben, x Osten,
z Süden, Ursprung die Kartenmitte. `at` (Welteinheiten, Vorgabe `[0, 0, 0]`) wird auf jede Ecke
addiert. Ecken außerhalb der Karte (±256), aber im Bereich des Bäckers, sind erlaubt (das Modell
ragt in die Nachbarkarte wie eine Vanilla-Kulisse); außerhalb von −512 bis 511 lehnt der Bäcker
mit `bake.face-out-of-range` ab. Die Freiform-Flächen tragen Ebene `FREEFORM_LAYER = 254`, die
Geometrie der Teilmodelle Ebene `PIECE_LAYER = 255` (eigene Läufe im Bäcker, keine Streifen über
Raster und Freiform hinweg).

**Kulisse für BDHC und Permissions.** Freiform und Teilmodelle ändern weder Terrain noch
Permissions: die BDHC kommt aus dem Raster (oder verbatim aus `terrain`), die Permissions aus
Raster, Fußabdruck und Überschreibung; aus Polygonen entstehen keine Wände und keine Höhen. Höhe
und Kollision unter einem Modell malt man im Raster.

**Texturen.** Das Backen ändert kein Paket. Neue Texturen eines Modells hängt `world_freeform_add`
(bzw. `tileset_piece_set`) an das Paket `world.tileset` an (`append_textures`), im selben
Undo-Schritt wie Beifile und Eintrag. Fehlt beim Backen die Textur eines Materials im Paket:
`bake.texture-missing` mit dem Satz des Bäckers („Die Textur „syn_boden“ fehlt im
Kartentextur-Paket oder hat dort keine Palette.“).

**Beifile fehlt oder liest nicht** (`world.side-file`, Fehler; das Modell wird abgelehnt, die Karte
behält ihr letztes Modell, BDHC und Permissions folgen der Welt): „Das Beifile
data/maps/0123.haus.glb fehlt.“ (Argumente `[Pfad]`) oder „Das Beifile data/maps/0123.haus.glb
lässt sich nicht lesen ({Grund}).“ (`[Pfad, Grund]`). Der Pfad steht ab dem letzten Ordner `data`.
Gründe: der Satz des Lesers (englisch, aus `model_import`, etwa der von `model.no-geometry`) und, für einen
Namen, der kein Beifile-Name der Karte bzw. des Pakets ist, „kein Beifile-Name der Karte 0123“ bzw.
„kein Beifile-Name des Pakets 0006“: ein solcher Name wird gar nicht gelesen (Schutz vor Pfaden
aus dem Projekt hinaus in einer von Hand geänderten Datei). Die Szene zeigt das Raster ohne diese
Freiform; ein Teilmodell ohne lesbares Beifile steht als Fußabdruck. Die App sagt dazu „Das Beifile
data/maps/0123.haus.glb fehlt oder lässt sich nicht lesen. Füge das Modell neu hinzu.“

**Hinzufügen und Entfernen** (Kernel `world_freeform_inspect`, `world_freeform_add`,
`world_freeform_remove`; Befehle in [mcp.md](../mcp.md) „Freiform, Form 2, Tiled (6.5b, über
`g4_call`)“):

- `world_freeform_inspect {req: {map, path, at?, options?, name?}}` liest das Modell (OBJ oder glTF,
  `read_model`; `options` wie beim Import von 6.5a, Vorgabe `unit` 16 und 16 Farben), plant seine
  Texturen gegen das Paket der Welt, hängt neue nur im Speicher an und backt die Welt mit dem Modell
  im Speicher; nichts wird geschrieben. Lehnt das Modell ab, kommt seine Ablehnung mit Code.
- `world_freeform_add` (gleiche Anfrage) ist **ein** Undo-Schritt „Freiform hinzugefügt: Karte 0123
  (0123.haus.glb)“: neue Texturen ans Paket, das Beifile als Dateischritt, der Eintrag
  `{glb, at}` am Ende von `world.freeform`, gebacken. Der Name kommt aus `name`, sonst aus dem
  Dateistamm (`side_name`: ASCII klein, jedes andere Zeichen `_`, höchstens 32 Zeichen, leer
  „freiform“, vergeben `_2`, `_3` …; vergeben ist auch jedes Beifile der Karte, das schon im Ordner
  liegt, so überschreibt ein neues Modell keine verwaiste Datei). Abgelehnt: was inspect ablehnt
  („Freiform nicht hinzugefügt: <Satz> (<Code>)“), ein Backen mit Fehler („Freiform nicht
  hinzugefügt: <Satz des ersten Fehlers>“, etwa `bake.model-too-big` oder `world.verbatim-mixed`
  auf einer Karte nach Form 1), eine Karte ohne Welt, eine gesperrte Karte, außerhalb eines
  Textprojekts. Scheitert ein Teil des Schritts, nimmt er alles zurück.
- `world_freeform_remove {map, index}` ist ein Undo-Schritt „Freiform entfernt: Karte 0123
  (0123.haus.glb)“: der Eintrag geht, sein Beifile im selben Schritt, wenn kein anderer Eintrag es
  nennt. Abgelehnt: ein Index jenseits der Liste („Karte 0123 hat keine Freiform 3 (1 Einträge).“)
  und das Verbatim-Modell („Freiform 0 von Karte 0123 ist das Originalmodell (Form 1); zurück zur
  binären Karte über „Als binäre Karte übernehmen“.“). In der App gibt es das Entfernen nicht (nur
  über `g4_call`, oder ⌘Z).
- In der App ist das Hinzufügen die Zeile „Freiform (glTF)“ im Dialog „Teile importieren…“ einer
  Welt-Karte (`.obj`, `.gltf`, `.glb`), mit dem Feld „Lage“ (x, y, z in Welteinheiten; „16 = eine
  Kachel; 0, 0, 0 = Kartenmitte“), den Texturen und dem Budget des Modells der Karte
  ([design-system/README.md](../../design-system/README.md), „Nachtrag 6.5b“). Ein Modell verschieben
  heißt entfernen und neu hinzufügen; `at` steht in der JSON.

**Im Textprojekt.** Speichern einer Karte und der Export behalten jedes Beifile, das
`world.freeform` der Karte nennt, und löschen ein nicht genanntes `NNNN.*` wie jedes andere
Beifile; der Import des Textprojekts macht aus Beifiles keine Member und keine Einheit. Die
`.gitattributes` der Projekte markiert `*.glb` und `*.png` binär
([project_git.md](project_git.md)). Jedes Beifile, das das Backen einer Karte liest, gehört zu
ihrem Eingabe-Hash ([build_state.md](build_state.md)): ein geändertes Beifile backt die Karte im
nächsten Build neu.

**Gate-Messung** (gated Test `freeform_on_a_painted_map_bakes_and_apicula_reads_it` in
`crates/g4-kernel/tests/world_freeform.rs`): die gemalte Startkarte (19 Flächen) mit dem Würfel
`g4_imp_cube.obj` (`unit` 32, `at [0, 0, 0]`) backt auf Platin DE, HeartGold DE und US gleich zu
25 Flächen, davon 6 Freiform, Modell 1 972 Byte (3,2 % von 0xF000); apicula liest das Modell, die
Freiform-Flächen stehen im GLB. Die Szenarien `pt-freiform.yaml` und `hg-freiform.yaml` zeigen den
Würfel im Spiel.

## Teilmodelle mit Geometrie (6.5b)

Eine Kachel der Rolle `piece` kann im Tileset `model: "<name>"` tragen ([tileset.md](tileset.md));
ihr Beifile ist `data/tilesets/PPPP.<name>.glb` im Paket der Welt, in derselben kanonischen Form wie
ein Freiform-Beifile. Gelesen wird es einmal je Kachel, die die Karte als Teilmodell setzt
(Kacheln, die sie nicht setzt, zählen nicht).

**Lage** (`mesh_with`): die Geometrie steht im Modellraum, ihr Ursprung kommt auf die Mitte des
**gedrehten, nicht abgeschnittenen** Fußabdrucks (x = linke Kante + Breite · 8, z ebenso, ganze
Welteinheiten), auf die Höhe der Fußabdruck-Fläche (die höchste Ecke der Bodenzellen darunter,
innerhalb der Karte); `rotate` dreht sie im Uhrzeigersinn von oben, je Vierteldrehung
`(x, z) → (−z, x)`, auf ganzen Welteinheiten. Ihre Flächen tragen Ebene 255, UV und Texturen
bleiben. Mit Modell ersetzt die Geometrie die Fußabdruck-Fläche; ohne `model` oder ohne lesbares
Beifile bleibt die Fläche. Die Permissions folgen weiter dem Fußabdruck.

Gesetzt wird das Modell mit `tileset_piece_set` (ein Undo-Schritt, [tileset.md](tileset.md)
„Teilmodell setzen (6.5b)“); die sauberen Welt-Karten des Pakets backen danach neu. In der Szene
liegt die Geometrie wie die Fußabdrücke halbdurchsichtig (Deckkraft 0,6).

Gate-Messung (gated Test `piece_model_on_the_copy`): der Würfel als Modell von Kachel 3 mit
Fußabdruck 1 × 1, ein Stück auf der gemalten Startkarte: 25 Flächen, Modell 1 972 Byte, auf allen
drei Spielen.

## Backen

`world::bake::bake_world(store, map, world, base, verbatim)` backt eine Welt mit dem Store,
`bake_grid` und `assemble` sind der reine Teil:

- **Tileset** `Store::tileset(world.tileset)` (die Datei, sonst `textures_only`, [tileset.md](tileset.md)).
- **Texturen** aus dem Member des Pakets `world.tileset` (`world_pack_textures`): eine Textur, die
  eine Kachel des Tilesets mit einer Palette nennt, die das Paket hat, bekommt diese Palette (was
  die Szene zeigt, zeigt das Spiel; Platins `allpeak` bindet in jedem Vanilla-Modell `apeak`, die
  Palette seines Index, die `pack_textures` von 6.1 nicht findet); jede andere Textur bindet
  `pack_textures`. Nennen mehrere Kacheln dieselbe Textur, entscheidet die **erste** Kachel in
  Tileset-Reihenfolge, die überhaupt eine Palette nennt: hat das Paket diese Palette, gilt sie
  (eine andere Palette einer späteren Kachel bleibt ungenutzt), sonst bindet `pack_textures`. Ein
  Paket, das sich nicht lesen lässt, gibt keine Texturen und kein VRAM: der Bäcker lehnt mit
  `bake.texture-missing` ab.
- **Gebiet:** Stil (`AreaStyle::of`, HeartGold/SoulSilver Lichttyp 1 = außen) und VRAM
  (`area_vram` mit diesem Paket, Grenze `vanilla_vram_max` der Familie) vom ersten Gebiet aus
  `map_import::map_areas(store, map)` (erster Header in Tabellenordnung, dessen Matrix die Karte
  zeigt). Ohne Gebiet, oder wenn dessen Gebietsdaten nicht im Archiv liegen, und mit Flächen:
  `world.no-area`. Ohne Flächen (Form 1) braucht das Backen kein Gebiet.
- **Modell:** die Flächen des Vernetzers (`From<MeshFace> for bake::Face`; seit 6.5b mit der
  Geometrie der Teilmodelle, `mesh_with`), dahinter die Flächen jedes Freiform-Beifiles, das sich
  liest, verschoben um sein `at` (`world::freeform::load_side_models`), über `bake::bake`. Mit
  Verbatim-Modell sind es dessen Bytes.
- **Terrain:** `bdhc::from_grid(grid.height_grid()).write()`, mit `terrain` dessen Bytes.
- **Permissions:** siehe „Permissions“. BDHC und Permissions entstehen immer, auch wenn das Modell
  abgelehnt wird.

Meldungen in fester Reihenfolge (`WorldBakeMessage {code, level, args, message}`, Text deutsch):

| Code | Wann | Satz |
|---|---|---|
| `world.side-file` | seit 6.5b: ein Beifile (Freiform oder Teilmodell) fehlt, liest sich nicht oder ist kein Beifile-Name; je Beifile einmal, Freiform vor Teilmodellen | „Das Beifile data/maps/0123.haus.glb fehlt.“ (Argumente `[Pfad]`), „Das Beifile data/maps/0123.haus.glb lässt sich nicht lesen ({Grund}).“ (`[Pfad, Grund]`); Gründe unter „Freiform und Beifiles (6.5b)“ |
| `world.verbatim-mixed` | Verbatim-Modell und gemalte Flächen oder ein `glb` in `freeform` | „Ein Verbatim-Modell (Form 1) backt nur allein; für Kacheln oder Freiform daneben die Karte als binäre Karte übernehmen und mit Form 2 (glTF) neu übernehmen.“ (die Karte behält das Verbatim-Modell) |
| `bake.empty` | ohne Verbatim-Modell keine Fläche | „Die Welt hat keine Fläche zum Backen.“ |
| `world.no-area` | Flächen, aber kein Gebiet | „Karte 0123 liegt in keiner Matrixzelle mit Header; Backen braucht Licht und VRAM ihres Gebiets.“ |
| `bake.model-too-big` | Modell über 0xF000 | „Das gebackene Modell hat 61 952 Byte, das Spiel fasst höchstens 61 440 Byte; die Karte behält ihr letztes Modell.“ |
| `bake.texture-missing` | eine Textur fehlt im Paket oder hat dort keine Palette | „Die Textur „syn_boden“ fehlt im Kartentextur-Paket oder hat dort keine Palette.“ |
| `bake.face-out-of-range` | eine Ecke außerhalb des Bereichs des Bäckers | „Fläche 3 hat die Ecke (0, 600, 0); Ecken liegen zwischen -512 und 511 Welteinheiten.“ |
| `bake.face-uv-range` | Texturkoordinaten zu weit | „Fläche 3 spannt 4 100 Texel in u; ihre Texturkoordinaten passen weder so noch um ganze Texturen verschoben in den Bereich des Modells.“ |
| `bake.vram-high` | Warnung: VRAM des Gebiets über dem Vanilla-Höchstwert | „Die Texturen des Gebiets belegen 120 000 Byte VRAM, das größte Vanilla-Gebiet 110 640 Byte.“ |

Ein `world.side-file` lehnt das Modell ab (das letzte bleibt), auch wenn die übrigen Flächen
backen würden. Vom Bäcker kommt nur sein erster Fehler (`refusal`) und jede `bake.vram-high`-Warnung. Zahlen im
Satz mit Leerzeichen als Tausendertrenner (`baker_text`); die Beispielzahlen der Tabelle sind
Muster, keine Messung.

**Abgelehntes Modell.** Bei jeder Ablehnung hat das Backen kein Modell: das Map-Asset behält sein
**letztes Modell** (`model_kept: true`), BDHC und Permissions folgen der Welt. Nichts wird gekürzt.

**Bericht** `WorldBakeReport` (`world_bake`, `world_get.bake`, `WorldReport.bake`):
`{map, ok, style, faces, model_bytes, model_share, model_kept, textures, vram_bytes, bdhc_bytes,
bdhc_payload, plates, blocked, verbatim_model, verbatim_terrain, messages, freeform_faces, strips,
single_quads}`; `ok` heißt keine Meldung der Stufe Fehler, `faces` zählt seit 6.5b die Flächen des
Vernetzers mit der Geometrie der Teilmodelle und die Freiform-Flächen, `freeform_faces` die
Freiform-Flächen darunter, `strips` und `single_quads` die Viereckstreifen und einzelnen Vierecke,
die das gebackene Modell sendet (0 ohne gebackenes Modell; alle drei seit 6.5b), `textures` die
Texturen der Materialien des Modells der Karte (auch eines behaltenen oder verbatim), `vram_bytes` 0
ohne gebackenes Modell, `blocked` die Zellen mit Bit 0x80.

**Revisionen:** `WORLD_BAKE_REV = 2` (`world/bake.rs`, seit 6.5b: Teilmodell-Geometrie und Freiform
backen mit), `bake::BAKE_REV = 1`, `bdhc::GENERATOR_REV = 1`; jede steigt, wenn dieselbe Welt andere
Bytes ergibt, und salzt den Build-Schritt `world` (`world#2+bake#1+bdhc#1`).

**Modellboden = BDHC** ist ein Test, keine Laufzeitprüfung: je Zelle mit Bodenkachel liest
`bake::decode_quads` das gebackene Modell zurück; die höchste nach oben zeigende Fläche über der
Zellmitte gibt die Bodenhöhe, ihr Abstand zu `Bdhc::height_at(Mitte, i32::MAX)` ist höchstens
`FLOOR_TOLERANCE = 0,1` E (Tests `model_floor_meets_the_bdhc_at_every_cell_centre`,
`prop_model_floor_meets_the_bdhc`).

## Permissions

Seit Schema 4 trägt die Karten-Datei einer Welt-Karte keine `permissions`: die Permissions des
Members entstehen beim Backen aus `world.permissions` (`world::bake::world_permissions`).

`{ "derive": true }`, je Zelle in dieser Reihenfolge:

1. **Oberste Kachel:** die oberste Ebene, die an der Zelle eine Kachel hat (Überlagerungen vor dem
   Boden); deren Tileset-Kachel gibt `Cell { kind, collision }`, in HeartGold/SoulSilver
   `collision | sound`. Eine Zelle ohne Kachel in allen Ebenen ist `EMPTY_CELL` (`kind` 0,
   `collision` 0x80, blockiert); eine Kachel jenseits des Tilesets zählt als keine Kachel dieser
   Ebene (der Check meldet sie ohnehin).
2. **Teilmodelle:** der Fußabdruck eines Stücks setzt auf seinen Zellen die Kachel des Stücks.
3. **Kanten:** `terrain::block_edges`, dieselbe Regel wie `derive_permissions` von 6.2
   ([terrain.md](terrain.md) „Kollisionsableitung“): eine Kante innerhalb der Karte, deren
   Unterschied mindestens `edge.free_below` (16 E, zwei Stufen) ist, blockiert die **obere** Zelle
   (`| 0x80`, Geräusch-Bits bleiben); der Kartenrand ist keine Kante.
4. **Handwerte:** `override` (32 Zeilen `.` oder `kkcc`) ersetzt die ganze Zelle.

Mit `derive: false` gelten nur die Überschreibungen, und das Schema verlangt jede Zelle: ohne
`override` „ohne Ableitung (`derive: false`) braucht es `override` mit 32 Zeilen.“, mit offenen
Zellen „ohne Ableitung (`derive: false`) braucht jede Zelle eine Permission; 3 Zellen sind `.`.“

Permissions einer Welt-Karte ändert `world_edit` (`permission`); `set_permission` und jede andere
direkte Änderung lehnen ab („Store-Art und Undo“).

## Beispiel

Karte 0 als Welt aus `world::doc::fixtures::one_tile_world()` (32 × 32 Zellen `0000` auf Höhe 0,
synthetisches Tileset auf Paket 0). Gekürzt: jede Zeile hier zeigt nur ihren Anfang und ihr Ende
(`…`), von den 32 Zeilen einer Liste nur die erste; `permissions`, `model` und `bdhc` fehlen
(seit Schema 4 entstehen sie beim Backen).

```json
{
  "source": "world",
  "world": {
    "version": 2,
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
  "buildings": []
}
```

Diese Welt vernetzt zu **einer** Fläche (512 × 512 Einheiten auf y = 0) und backt zu einem Modell
mit einer Textur (`syn_boden`) von höchstens 4 096 Byte (Test `one_tile_world_bakes_small`); der
Check meldet `world.budget` als Info („Prüfungen“). Das Schachbrett aus `0000` und `0001`
(`fixtures::checkerboard_world`) vernetzt zu 1 024 Flächen und backt zu 41 564 Byte (unter
0xF000). `fixtures::gate_world` (Boden, Treppe als Rampe `e1`, ein Plateau auf Stufe 2 mit Wänden,
Gras und Wasser, eine Überschreibung) gibt 19 Flächen, ein Modell von 1 704 Byte, eine BDHC von
232 Byte (216 Byte Nutzlast) und 12 blockierte Zellen; der Modellboden trifft die BDHC in allen
1 024 Zellmitten auf 0,0000 E genau (Tests in `world/bake/tests.rs`, synthetisches Tileset und
synthetische Paket-Texturen).
Eine Rampe, eine Überlagerung, ein Teilmodell, ein Freiform-Modell (wie
`fixtures::freeform_world`) und eine Überschreibung sähen so aus (Ausschnitte
von Hand, kein ganzes JSON):

```text
"ramps": [". . e1 e1 . … .", "…"],
"…": "…",
{ "name": "Deko", "role": "overlay", "tiles": [". 0002 . … .", "…"] },
"pieces": [{ "tile": 5, "at": [10, 4], "rotate": 90 }],
"freeform": [{ "glb": "0000.wuerfel.glb", "at": [0, 0, 0] }],
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

**Das Map-Asset einer Welt-Karte ist ihr gebackener Member** (seit 6.3b). Der Store liest es im
Textprojekt, indem er die Karten-JSON liest (`EntitySource::read_world_map`) und backt: Gebäude,
Rest und BGS aus der Datei, das Modell des Members auf der Platte als das „letzte Modell“, das ein
abgelehntes Backen behält. Eine Datei mit `permissions`, `model` oder `bdhc` lehnt das Lesen als
`textproject.map-source` mit Datei, Zeile und Pfad ab. Eine Welt, die nicht backt (Zeilen, die
sich nicht lesen, ein Paket, das es nicht gibt), lässt das Map-Asset der Member auf der Platte sein
wie vor 6.3b; der Check nennt das Problem der Welt. Beim Laden der Karte lädt der Store auch ihr
WorldMap-Asset. Weil das Map-Asset die Ausgabe ist, folgen Höhen (`get_map_heights`), Budget,
Permissions-Overlay und GLB der Szene der Welt ohne Sonderweg.

- **Jeder Strich backt im selben Undo-Schritt:** `Tx::edit_world` backt nach der Änderung über die
  Karte, wie die Transaktion sie hält (`Verbatim::of` der neuen Welt), und legt WorldMap- und
  Map-Asset in **ein** Changeset; ⌘Z stellt beide zurück. Den Bericht gibt `WorldReport.bake`.
  Lehnt der Bäcker ab, bleibt der Strich ein Undo-Schritt, das Modell das letzte, BDHC und
  Permissions folgen der Welt, und `bake` nennt den Code; Malen über das Budget und wieder zurück
  bleibt möglich.
- **Speichern** backt jede Welt-Karte mit ungespeicherten Änderungen unmittelbar vor dem Schreiben
  neu (Tileset, Paket oder Gebiet können sich seit dem Strich geändert haben); ein anderes
  Ergebnis ersetzt den Wert des Map-Assets ohne eigenen Undo-Schritt. Ein Backen, das scheitert
  (Paket weg, Lesefehler im Gebiet), lässt diese Karte, wie der Strich sie backte, und hält das
  Speichern der übrigen nicht auf. Geschrieben werden die JSON ohne abgeleitete Teile und der
  Member; beide zeichnet das Speichern auf (der Datensatz `world` nur bei einem Backen ohne
  Fehler, sonst verliert die Karte ihren Datensatz, [build_state.md](build_state.md)).
  **Lehnt der Bäcker beim Speichern ab** (Budget, fehlende Textur), schreibt das Speichern den
  Member trotzdem, und zwar mit dem **letzten Modell** und der **neuen** BDHC und den neuen
  Permissions der Welt (das Map-Asset, wie es nach „Abgelehntes Modell“ ist). Dieser Member ist in
  sich nicht stimmig: Modell und Terrain gehören zu verschiedenen Ständen der Welt. Der Datensatz
  `world` der Karte entfällt, der Check meldet `world.bake`, und der Build-Schritt `world` lehnt
  die Karte ab, bis die Welt wieder backt; DSPRE dagegen öffnet den Member, wie er liegt, also mit
  altem Modell über neuem Terrain.
- **Stale-Prüfung beim Speichern:** für das Map-Asset einer Welt-Karte vergleicht sie nur Gebäude,
  Rest und BGS mit dem Stand, den der Store zuletzt sah, nicht die Bytes (`same_base`); die Welt
  selbst prüft das WorldMap-Asset. Grund: das Lesen backt, und das Backen folgt auch Tileset,
  Paket und Gebiet, ein Byte-Vergleich meldete nach einer ungespeicherten Änderung dort ein
  falsches `Stale`. Preis: eine Änderung von außen nur an den abgeleiteten Bytes des Members bei
  unveränderter JSON sieht diese Prüfung nicht; sie fängt der Datensatz je Karte
  (`world.member-changed` im Abgleich, im Build-Schritt `world` und im Check).
- **Abgeleitete Teile direkt ändern** lehnt `Tx::edit_map`/`Store::edit_map` (und `Tx::replace`)
  auf einer Welt-Karte ab, sobald sich `permissions`, `model` oder `bdhc` ändern: „Karte 0123 ist
  eine Welt-Karte: Modell, Terrain und Permissions entstehen beim Backen; Permissions über
  `world_edit` (`permission`), zurück zur binären Karte über „Als binäre Karte übernehmen“.“ Damit
  lehnen `set_permission`, `set_map_terrain`, `bake_test` und `map_import` (Modell, BDHC,
  Permissions, ganze Karte) auf Welt-Karten ab; Gebäude verschieben und BGS gehen.
- **Tileset geändert:** nach jedem Dateischritt, der eine Tileset-Datei schrieb (`tileset_save`,
  `tileset_edit`, [tileset.md](tileset.md)) oder die Kartentexturen eines Pakets änderte
  (`append_textures`), und nach Undo oder Redo eines solchen Schritts backt
  `Store::forget_world_bakes(package)` die sauberen geladenen Welt-Karten dieses Pakets **an Ort
  und Stelle** neu (aus der JSON gelesen wie beim ersten Zugriff), statt sie zu entladen: ihr
  Verlauf bleibt, denn Undo braucht geladene Assets. Vorher fällt das zwischengespeicherte Tileset
  des Pakets weg. Schmutzige Welt-Karten backt das nächste Speichern.
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
  nichts ändert: „Nichts zu ändern.“ `world_create` und `world_edit` antworten mit `bake` des
  Schritts; ein abgelehntes Backen ist kein Fehler des Aufrufs.
- **Nur im Textprojekt:** im DSPRE-Layout und in einem Textprojekt anderer Schemaversion lehnen
  `world_create`, `world_edit`, `world_adopt`, `world_to_binary`, `tileset_save` und
  `tileset_edit` mit „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“ ab;
  `world_get` antwortet `world: null`, `world_bake` `null`, `get_world_mesh` ein leeres Netz,
  `get_map` `world: false`.
- **Gesperrte Karte:** lässt sich der Member einer Karte nicht lesen (sie steht als `{locked, raw}`
  in der Datei), hat sie keinen Platz für eine Welt. `world_create` und `world_adopt` lehnen ab
  („Karte 0001 lässt sich nicht als Welt anlegen: ihr Member lässt sich nicht lesen (…).“ bzw.
  „… nicht übernehmen: …“ mit dem Grund), `world_edit` mit dem Satz unten, und ein Speichern oder
  Export, das die Karte gesperrt neu schriebe, lehnt ab, statt die Welt zu verlieren:
  „textproject.map-source: Karte 0123 ist eine Welt-Karte, ihr Member lässt sich nicht
  lesen; Schreiben verlöre das Weltobjekt. Erst den Member reparieren (oder die Welt aus der Datei
  sichern).“
- **Mitnehmen beim Schreiben:** der Writer der Karte nimmt `source` und `world` aus dem
  WorldMap-Asset (sonst wie der Store es hält, sonst von der Platte, `world::keep::carry_world`),
  `buildings`, `buildings_tail` und `bgs` aus den Map-Bytes und lässt `permissions`, `model` und
  `bdhc` weg (`world::keep::world_parts`); Verbatim-Teile gehen an ihren Platz in der Welt
  (`freeform[0].nsbmd`, `terrain.bdhc`), ein Beifile, auf das nur ein weggelassener Teil zeigte,
  wird gelöscht. Ein Freiform-Beifile `NNNN.<name>.glb`, das `world.freeform` der Karte nennt, bleibt
  (seit 6.5b, auch wenn die Welt sich nicht liest: dann zählen die Texte unter
  `world.freeform[].glb` der Datei); ein nicht genanntes geht wie jedes andere Beifile. Der volle Export nimmt `source`/`world` von der Platte wie den
  Grabstein-Umschlag (`carry_world_parts`). Ein verschobenes Gebäude und jeder Export lassen das
  Weltobjekt Byte für Byte stehen, auch eines, das der Check ablehnt.
- **Neu lesen und Zurücksetzen:** `reload_from_disk` eines WorldMap-Assets liest die JSON neu.
  „Auf Original zurücksetzen“ lehnt ab: „Eine Welt hat kein Original; erst „Als binäre Karte
  übernehmen“.“
- **Lesen:** `world_get` antwortet mit `bake` (der Bericht, wie der Store die Karte hält); `bake`
  ist `null` ohne Welt und bei einer Welt, deren Zeilen sich nicht lesen (die Welt bleibt mit
  0 Flächen). `world_bake` lehnt dann mit dem ersten Fehler ab („world.schema:
  world.layers[0].tiles[0]: 31 Zellen statt 32.“).

Die Befehle für Agenten stehen in [mcp.md](../mcp.md): seit 6.4a die kuratierten Werkzeuge
`world_get`, `world_edit` und `tileset_get` („Malen (6.4a)“), die übrigen über `g4_call`
(„Welt-Karten (6.3a, über `g4_call`)“, „Welt-Karten backen (6.3b, über `g4_call`)“).

## Karte als Quelle übernehmen (Form 1)

`world_adopt {map, tileset?, form?}` macht eine binäre Karte zur Welt-Karte, die **byte-gleich**
backt: ein Undo-Schritt „Karte als Quelle übernommen: Karte 0123“ (`world::adopt`). Die Welt:

- `version` 2, `tileset` aus der Anfrage, sonst das erste Paket von `places::packs_of_map`, sonst 0
  mit `tileset_guessed: true` im Bericht;
- Bodenebene „Boden“ ganz `.` (keine gemalte Kachel), keine Rampe;
- `heights` aus der BDHC: je Zellmitte die Höhe aus `terrain::map_heights` (höchster Kandidat),
  Stufe = `round_ties_even(y / 8)` auf −16 bis 62 begrenzt, ohne Platte 0. Gerundet wird zur
  geraden Nachbarstufe, damit die Mitte einer Rampe von einer Stufe (4 E, eine halbe Stufe)
  Stufe 0 wird, nicht 1 (12 E → 2, 15,95 E → 2). Eine BDHC, die sich nicht lesen lässt, zählt als
  „keine Platte“: die Karte wird eben übernommen, jede Zelle ist eine Abweichung (die BDHC selbst
  bleibt verbatim);
- `freeform = [{nsbmd: <Teil>, at: [0, 0, 0]}]` und `terrain = {bdhc: <Teil>}`, beide in der Form,
  die ein Speichern schreibt (`Store::world_as_written`: `base_sha1`, Beifile oder Hex);
- `permissions = {derive: true, override: alle 1 024 Zellen}` aus den Permissions der Karte (`kkcc`,
  in HeartGold/SoulSilver mit den Geräusch-Bits);
- Gebäude und `bgs` bleiben im Map-Asset und in der Datei.

Der Bericht `{map, tileset, tileset_guessed, deviations, change, bake}` nennt in `deviations` jede
Zelle ohne Platte (`height: null`) oder mit `|y − 8 · Stufe| > ADOPT_TOLERANCE = 0,1` E als
`{x, z, height, step}`. Abgelehnt (nichts geändert): außerhalb eines Textprojekts („Boden malen
gibt es im Textprojekt (Technik › Textprojekt).“), eine Welt-Karte („Karte 0123 ist schon eine Welt-Karte.“), eine gesperrte Karte oder eine,
deren Member sich nicht liest (siehe „Gesperrte Karte“), ein Paket, das es nicht gibt.

`form: "gltf"` ist Form 2 (unten). Malt man nach Form 1 Kacheln, lehnt das Backen mit
`world.verbatim-mixed` ab (das Verbatim-Modell bleibt; `world_freeform_add` lehnt eine Freiform
daneben mit demselben Satz ab); eine Änderung von `height` oder `ramp` beendet das Verbatim-Terrain. Prüfstein (gated
Test `vanilla_maps_adopted_form1_bake_byte_equal` in `crates/g4-kernel/tests/world_bake.rs`): alle
Vanilla-Karten von Platin DE (666), HeartGold DE und US (676) backen nach dem Übernehmen
byte-gleich.

## Form 2: Als glTF übernehmen (6.5b)

`world_adopt {map, tileset?, form: "gltf"}` macht eine binäre Karte zur Welt-Karte wie Form 1
(Höhen aus der BDHC abgetastet, `terrain` verbatim, alle Permissions als Überschreibung, Gebäude
und `bgs` im Map-Asset, Bodenebene ganz `.`), nur das Modell ist ein Freiform-Beifile:
`freeform = [{glb: "NNNN.model.glb", at: [0, 0, 0]}]` (`FORM2_SIDE_NAME = "model"`). Das Modell
backt **neu**, also nicht byte-gleich; Terrain und Permissions bleiben, wie sie waren. Danach
lassen sich Kacheln und weitere Freiform-Modelle daneben malen und backen (kein Verbatim-Modell,
also kein `world.verbatim-mixed`). Ein Undo-Schritt mit dem Label von Form 1 („Karte als Quelle
übernommen: Karte 0123“): erst das Beifile als Dateischritt, dann die Welt. Implementierung:
`world/adopt.rs` (`world_adopt_with`, `world_adopt_glb`), die Bindung `world/freeform.rs`
(`bind_vanilla_materials`).

**Weg.** apicula (`G4_APICULA`, sonst neben der App, im Debug-Build `tools/apicula/bin/apicula`)
wandelt das Modell der Karte mit den Texturen des Pakets nach `<Projekt>/.g4/tmp/adopt-NNNN`
(scheitert das, ohne Paket); der Ordner wird danach immer entfernt. Das GLB liest `read_model`,
jedes Material bindet eine Textur des Pakets (Regel unten), `to_faces` wandelt mit
`APICULA_UNIT = 1` (apicula schreibt Welteinheiten), und die Flächen werden das kanonische Beifile
(„Freiform und Beifiles (6.5b)“). Ohne apicula: „Form 2 braucht apicula (`G4_APICULA` oder
tools/apicula).“; scheitert apicula selbst, kommt dessen Fehler. Der Kernel kennt apicula nur als
Argument (`world_adopt_with(store, req, apicula)`); `world_adopt` ohne apicula ist Form 1 oder
diese Ablehnung.

**Probe** (Task 5, apicula `convert -f=glb` mit Paket auf dem Modell der ersten Karte von
Zweiblattdorf, Platin DE, Karte 0, Paket 6; Kommentar an `APICULA_UNIT`): die Positionen sind
Welteinheiten (die Box des GLB ist die Box im Kopf des Modells); jedes glTF-Material heißt wie sein
MDL0-Material, das eine Textur anderen Namens binden kann; die Bilder heißen `<Textur>.png`, ein
`.` im Texturnamen als `_`, ohne Paket schreibt apicula keine; der Leser lässt `skins` und
`vertex-colours` aus (`model.parts-ignored`); das GLB hat 1 628 Dreiecke für die 815 Polygone (813
Vierecke, 2 Dreiecke) des MDL0.

**Paket.** `tileset` aus der Anfrage; sonst sucht Form 2 nach Passung: das erste Paket von
`places::packs_of_map`, das jede Textur hat, die das erste Modell im MDL0 mit einem Material paart
(gemessen an den Texturen, wie das Backen sie bindet, `world_pack_textures`), sonst das erste
solche Paket des Archivs in aufsteigender Reihenfolge (`tileset_guessed: true`), sonst die Wahl von
Form 1 (das erste Paket der Header, sonst 0 mit `tileset_guessed`). Die Wahl von Form 1 ändert sich
dadurch nicht.

**Bindung je glTF-Material** (`bind_vanilla_materials`):

1. Das MDL0-Material gleichen Namens im ersten Modell der Karte gibt die Textur, die es bindet
   (Paarungen `Model.textures[*].materials`), wenn das Paket sie hat;
2. sonst (auch wenn die Textur aus 1 nicht im Paket steht) der Stamm des Bilds des Materials, wenn
   das Paket eine Textur dieses Namens hat. Ein Modell, das sich nicht liest, hat keine
   MDL0-Materialien: nur 2.
3. Ein Material ohne MDL0-Textur **und** ohne Bild ist **ohne Textur** (ein Material nur mit
   Eckenfarben, etwa Mayas `lambert1`): seine Polygone fallen weg, ihre Zahl steht in
   `baked.untextured` und zählt in `baked.dropped` mit, und `ignored` nennt es als
   `untextured:<Material>`. Polygone ganz ohne Material bleiben (`to_faces` lehnt sie ab).
4. Ein Material, das eine Textur nennt (über MDL0 oder sein Bild), die das Paket nicht bindet, lehnt
   ab: `world.adopt-texture`, „Material „lake_lm2“ der Karte 0123 bindet keine Textur des Pakets
   0006; Form 2 geht für diese Karte nicht, Form 1 schon.“

Im Beifile heißt das Material danach wie die Textur des Pakets.

**Abgelehnt** (Text „{Satz} ({Code})“, kein Schritt, kein Beifile, die Karte bleibt binär): ein GLB,
das sich nicht liest oder zu nichts wird (sein `model.*`-Code; auch wenn nur Polygone ohne Textur
da waren, dann `model.no-geometry`), `world.adopt-texture`, ein Backen mit Fehler (der erste, etwa
`bake.model-too-big` oder `world.no-area`; der Schritt wird zurückgenommen); dazu alles, was Form 1
ablehnt. In der App ist das die Taste „Als glTF übernehmen (Form 2)“ im Abschnitt „Quelle“, die
Ablehnung steht dort als Fehlermeldung.

**Bericht.** `WorldAdoptReport.form2` (bei Form 1 `null`): `{vanilla: {polygons, quads, triangles,
bytes}, baked: {faces, quads, triangles, strips, single_quads, dropped, bytes, untextured}, ignored,
glb}`. `vanilla` zählt, was die Shapes des ersten MDL0-Modells zeichnen
(`Model::geometry`), `baked` die Flächen des Backens, die Vierecke und Dreiecke, die die Umwandlung
dem Bäcker gab, die Streifen und einzelnen Vierecke, die das Modell sendet, die weggefallenen
Polygone (nach dem Runden ohne Fläche oder ohne Textur) und die Bytes. Die App meldet „Übernommen
als glTF: 815 Vanilla-Polygone, 816 gebackene Flächen.“

**Abweichung der Polygonzahl.** Die beiden Zahlen weichen ab: apicula zerlegt jedes Polygon in
Dreiecke, `to_faces` setzt Paare wieder zu Vierecken zusammen (`pair_triangles`), aber nicht jedes
Paar so, wie das Vanilla-Modell es hatte, und der Bäcker zeichnet Viereckstreifen, wo das
Vanilla-Modell eigene Listen hat. Gemessen (gated Test `form2_of_a_vanilla_map_bakes_apicula_readable`,
die Startkarte, Task 10; nach Task 10b unverändert):

| Spiel | Karte | Vanilla: Polygone (Vierecke + Dreiecke), Bytes | gebacken: Flächen (Vierecke + Dreiecke), Streifen, Bytes | weggefallen | Anteil an 0xF000 |
|---|---|---|---|---|---|
| Platin DE (CPUD) | 0 | 815 (813 + 2), 38 876 B | 816 (812 + 4), 4 Streifen, 38 224 B | 0 | 0,622 |
| HeartGold DE (IPKD) | 0 | 336 (318 + 18), 16 604 B | 332 (322 + 10), 24 Streifen, 16 248 B | 0 | 0,264 |
| HeartGold US (IPKE) | 0 | 336 (318 + 18), 16 604 B | 332 (322 + 10), 24 Streifen, 16 248 B | 0 | 0,264 |

`ignored` ist auf allen drei `skins,vertex-colours`. Das gebackene Modell liest apicula wieder, die
Ecken und UV des GLB sind die der gebackenen Flächen, BDHC und Permissions sind die des Originals.

**Über alle Karten** (gated Test `form2_sweep_over_every_map`, nur mit `G4_FORM2_SWEEP=1` und
apicula; eine Karte zählt als ok, `bake.model-too-big`, `world.adopt-texture` oder anderes). Vor
und nach Task 10b (Polygone ohne Textur weglassen, Paket nach Passung):

| Spiel | Karten | vorher: ok / zu groß / Textur / anderes | nachher: ok / zu groß / Textur / anderes | Median Flächen : Polygone |
|---|---|---|---|---|
| Platin DE | 666 | 465 / 1 / 187 / 13 | 601 / 1 / 28 / 36 | 1,000 |
| HeartGold DE | 676 | 518 / 14 / 131 / 13 | 607 / 14 / 8 / 47 | 0,997 |
| HeartGold US | 676 | 518 / 14 / 131 / 13 | 607 / 14 / 8 / 47 | 0,997 |

Nachher ließ Form 2 auf Platin DE in 126 Karten zusammen 2 785 Polygone ohne Textur weg, auf
HeartGold DE und US in je 88 Karten 2 709; 8 Karten von Platin gelangen auf einem gesuchten Paket.
„Anderes“ wuchs, weil Karten ohne Header früher schon beim Binden auf dem geratenen Paket 0
scheiterten und jetzt bis zum Backen kommen: Platin 29 × `world.no-area`, 4 × `model.no-geometry`
(nur Polygone ohne Textur), 3 × `model.out-of-range`; HeartGold 30 × `world.no-area`, 15 ×
`model.out-of-range`, 2 × `model.no-geometry`.

**Grenzen.**

- **Ursache C bleibt eine Ablehnung:** das Paket hat die Textur, aber `pack_textures` findet ihre
  Palette nicht (und keine Kachel des Tilesets nennt eine, die das Paket hat); dann fehlt sie unter
  den Texturen, gegen die gebunden wird, und Form 2 lehnt mit `world.adopt-texture` ab (bei der
  Diagnose vor Task 10b Platin DE 13, HeartGold 1 Karte). Ebenso eine Textur, die in keinem Paket
  liegt. Form 1 geht für diese Karten.
- `model.out-of-range`: Vanilla-Modelle, deren Ecken über −512 bis 511 Welteinheiten hinausragen
  (bis ±768), lehnt der Leser von 6.5a ab; Form 1 übernimmt sie byte-gleich.
- `world.no-area`: eine Karte in keiner Matrixzelle mit Header hat kein Gebiet zum Backen.
- Eckenfarben und Skins des Vanilla-Modells gehen verloren (`ignored`).

## Zurück: Als binäre Karte übernehmen

`world_to_binary {map}`: ein Undo-Schritt „Welt aufgegeben: Karte 0123“, das WorldMap-Asset wird
`null`, das zuletzt gebackene Map-Asset bleibt; das nächste Speichern schreibt die Karte mit
`source: "binary"` und allen drei Teilen. Auf einer Karte ohne Welt: „Karte 0123 ist keine
Welt-Karte; erst „Welt anlegen“ (world_create).“ Derselbe Weg mit einem Konflikt ist die Wahl
„Als binäre Karte übernehmen“ (`take_member_as_binary`, unten): dort wird der Member, den DSPRE
schrieb, zur binären Karte.

## Build, Datensatz und DSPRE

- **Build-Schritt `world`** (nur im Textprojekt) läuft direkt nach `textproject`, und nur wenn
  `textproject` gelang, vor `rotom`. Er backt jede Welt-Karte ohne Grabstein, deren **Eingabe-Hash**
  sich gegen ihren Datensatz geändert hat oder deren Member fehlt, und schreibt die Member; eine
  geänderte JSON backt eine Karte. Eingaben, Salz und Datensatz in [build_state.md](build_state.md).
- **Datensatz** je Karte im Abschnitt `world` von `.g4/build/state.json`: `{input, member}` (xxh3),
  geschrieben vom Schritt, vom Speichern, vom Import und von der Wahl in einem Konflikt. Ohne
  Datensatz gilt die Welt: der Member wird gebacken und überschrieben.
- **`world.member-changed`:** weicht der Member vom Datensatz ab (DSPRE schrieb ihn), ist das im
  Schritt `world` ein Fehler je Karte, und nichts wird geschrieben: „Karte 0123: Der Member wurde
  außerhalb des Studios geändert (DSPRE?). In Technik › Textprojekt „Neu backen“ oder „Als binäre
  Karte übernehmen“.“ Im Abgleich mit DSPRE ist es **immer** ein Konflikt (`other: "dspre"`,
  `world: true`), nie eine einseitige Übernahme; Wege über `resolve_external_conflict`:
  `studio` = „Neu backen“ (Member aus der Welt, Datensatz neu), `dspre` = „Als binäre Karte
  übernehmen“ (JSON aus dem Member mit `source: "binary"`, Welt und Datensatz weg). Im Check ist es
  eine Warnung. Eine nur über Git geänderte Welt (JSON neu, Member wie aufgezeichnet) übernimmt
  der Abgleich einseitig: er backt sie.
- **Import** (`g4k text import`): er schreibt die Member der übrigen Entitäten, dann backt er die
  Welt-Karten. Ob ein Member außen geändert ist, entscheidet er **vor** dem ersten Schreiben: dann
  lehnt er mit `world.member-changed` ab, und nichts ist geschrieben; mit `--force` backt er über
  den Member. Ein abgelehntes Backen (`world.bake`) zeigt sich erst nach den anderen Schreibvorgängen
  und kommt als Problem des Berichts zurück; die Karte behält ihren Member. Der Import bleibt
  erfolgreich: das `build::prepare` am Ende des Imports findet im Schritt `world` denselben
  Fehler, lässt ihn aber nicht scheitern (nur `world.bake`, jeder andere Fehler schon), die
  übrigen Schritte laufen, und der Abgleich zeichnet das Geschriebene auf. Erst ein späterer
  Build lehnt die Karte ab, bis die Welt wieder backt.

## Prüfungen

Im Check des Textprojekts (`textproject::check::hints`, `world/check.rs`), nicht im Prüf-Register
(`checks/*`). Jede Meldung `<code>: <JSON-Pfad>: <Satz>` mit Datei, Zeile und Spalte; ein
Zeilenfehler zeigt auf sein Token in der Zeile.

| Code | Schwere | Wortlaut |
|---|---|---|
| `world.schema` | Fehler | Zeilen- und Token-Fehler (siehe „Grammatik“), Bereiche, Rolle der Ebenen, Rampe über 62, `derive: false` ohne volle Überschreibung, Teilmodell (`rotate`, `at`), Freiform (seit 6.5b auch der Name in `glb` und `at`, „Freiform und Beifiles (6.5b)“); dazu gegen das Tileset: „Zelle 2: Kachel 0x0006 gibt es im Tileset 0000 nicht (6 Kacheln).“ (je Zeile die erste), ein Paket, das es nicht gibt („world.tileset: Kartentextur-Paket 0007 gibt es nicht (1 Paket).“), eine Teilmodell-Kachel, die keine ist („Kachel 0x0001 ist kein Teilmodell (Rolle `grass`).“) oder deren Fußabdruck keine Fläche hat („… (0 × 2).“), ein Stück über dem Rand („Das Teilmodell ragt über den Kartenrand (x 31, Breite 2).“) |
| `world.tileset-mismatch` | Warnung | je Header, dessen Matrix die Karte zeigt und dessen Gebietsdaten ein anderes Paket nennen, an `world.tileset`: „Karte 0000 nimmt Tileset 0000, ihr Header 0001 nennt Gebietsdaten mit Paket 0001.“ |
| `world.bake` | Fehler | je Ablehnung des Backens einer Welt ohne `world.schema`-Fehler, an `world`, mit dem Code des Backens vorn: „world.bake: world: bake.model-too-big: Das gebackene Modell hat …“; Codes `bake.model-too-big`, `bake.texture-missing`, `bake.face-out-of-range`, `bake.face-uv-range`, `bake.empty`, `world.no-area`, `world.verbatim-mixed`, seit 6.5b `world.side-file` (Sätze unter „Backen“; `world.freeform-later` gibt es seit 6.5b nicht mehr); ein Backen, das mit einem anderen Fehler als dem der Datei scheitert, ist ebenfalls `world.bake` mit dessen Text |
| `world.budget` | Info | je Welt, die ohne Fehler backt, an `world`: „Gebacken: Modell 21 740 Byte (35 % von 0xF000), Terrain 334 Byte (1 % von 0x9000), 3 Texturen, VRAM 21 656 Byte.“ (Terrain ist die Nutzlast der BDHC, Anteile auf ganze Prozent gerundet; Beispielzahlen des Tests `budget_is_the_baked_size`); **Warnung** mit `bake.vram-high`, vor dem Punkt dazu „; VRAM über dem Vanilla-Höchstwert 110 640 Byte“. Bei einem abgelehnten Backen fehlt die Zeile |
| `world.member-changed` | Warnung | der Member weicht vom Datensatz der Karte ab: „world.member-changed: world: Karte 0123: Der Member wurde außerhalb des Studios geändert (DSPRE?). In Technik › Textprojekt „Neu backen“ oder „Als binäre Karte übernehmen“.“ (im Build-Schritt `world` ein Fehler, „Build, Datensatz und DSPRE“) |

Eine Prüfung „noch nicht gebacken“ gibt es seit 6.3b nicht mehr. Gebacken wird für den Check
mit einem Store auf dem Projekt, wie er die Karte hält; eine Datei mit abgeleiteten Schlüsseln ist
der Fehler des Imports (`textproject.map-source` mit Pfad) und wird nicht gebacken.

Geprüft werden die Kacheln gegen die Tileset-Datei des Pakets, sonst gegen die Texturen des Pakets
(`textures_only`, [tileset.md](tileset.md)). Der Import lässt eine Karte mit `world.schema`-Fehler
oder abgeleiteten Schlüsseln aus (ihr Problem im Plan, wie eine Entität, die sich nicht importieren lässt) und importiert die
übrigen; der Check zeigt jedes Problem einmal. Ein Paket, das nur in der Basis liegt, ist kein
Fehler: der Import stellt seinen Member wieder her.

**Codes von 6.5b.**

| Code | Wo | Schwere | Wortlaut |
|---|---|---|---|
| `world.side-file` | Backen (Strich, Speichern, Build-Schritt `world`, Import), im Check als `world.bake` | Fehler | „world.bake: world: world.side-file: Das Beifile data/maps/0123.haus.glb fehlt.“, „… lässt sich nicht lesen ({Grund}).“ („Freiform und Beifiles (6.5b)“) |
| `world.verbatim-mixed` | Backen, im Check als `world.bake` | Fehler | „Ein Verbatim-Modell (Form 1) backt nur allein; für Kacheln oder Freiform daneben die Karte als binäre Karte übernehmen und mit Form 2 (glTF) neu übernehmen.“ (seit 6.5b auch mit einem `glb`-Eintrag neben dem Verbatim-Modell, selbst wenn dessen Beifile fehlt) |
| `world.adopt-texture` | keine Prüfung: Ablehnung von `world_adopt` mit `form: "gltf"` | – | „Material „lake_lm2“ der Karte 0123 bindet keine Textur des Pakets 0006; Form 2 geht für diese Karte nicht, Form 1 schon. (world.adopt-texture)“ („Form 2“) |

Dazu im Schema die Regeln für `glb` und `at` („Freiform und Beifiles (6.5b)“) und im Tileset die
Regeln für `model` sowie ein Beifile im Tileset-Ordner, das keine Kachel nennt ([tileset.md](tileset.md)
„Prüfungen“). `world.freeform-later` gibt es nicht mehr.

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
  Kachel. Wandstücke einer Linie mit gleicher Wandkachel (Kachelindex, nicht Texturname), gleicher
  Richtung und geradem Unter- und Oberprofil werden eine Fläche. Keine Wand an der Kartengrenze (Nachbarkarten sieht erst 6.4b), zu
  einer leeren Zelle und wo sich die Profile **kreuzen** (gegenläufige Rampen nebeneinander). Liegt
  eine Rampe neben einer ebenen Zelle, die an einem Ende gleich hoch ist, wird die Wand ein
  **entartetes Viereck**: zwei Ecken fallen zusammen, es ist ein Dreieck.
- **Fußabdrücke:** je Teilmodell eine Fläche auf `PIECE_LAYER` (siehe „Teilmodelle“); trägt die
  Kachel ein lesbares Modell, stattdessen dessen Flächen (`mesh_with`, „Teilmodelle mit Geometrie
  (6.5b)“). `mesh(grid, tileset)` ist `mesh_with` ohne Modelle.
- **Reihenfolge:** Ebene aufsteigend; je Ebene die Böden in Reihenfolge ihrer Startzelle
  (Zeile, dann Spalte), auf der Bodenebene danach die Wände (Nordkanten zeilenweise, dann
  Westkanten spaltenweise). Die Fußabdrücke kommen einmal nach allen Ebenen, in
  Listenreihenfolge. Deterministisch, ohne Hash-Reihenfolge.
- **Budget-Schätzung:** `estimate_model_bytes(Flächen, Texturen) = 1 024 + 600 · Texturen +
  56 · Flächen`, eine obere Schranke (Vierecke mit `TEXCOORD` und `VTX_16`; gemessen in
  [Gutachten A](../research/2026-11-05-phase-6/gutachten-a-writer.md) §2.5: 1 024 Flächen mit
  einer Textur 57 776 Byte, geschätzt 58 968). Grenze `MODEL_BUDGET = 0xF000`. Seit 6.3b
  meldet der Check die echte Zahl des Bäckers (`world.budget`); die Schätzung bleibt für Werkzeuge,
  die nicht backen.
- **Zeit:** eine volle Karte (`fixtures::busy_world`) vernetzt im Release-Build in höchstens 20 ms,
  das Netz über die Brücke (`get_world_mesh`, warm) im Debug-Build in höchstens 50 ms (Tests
  `mesh_of_a_full_map_is_fast` in `crates/g4-kernel/tests/world.rs` und
  `world_mesh_over_the_command_path_is_fast` in `commands/world_map.rs`, strikt mit
  `G4_TIMING_STRICT=1`).

## Vorschau in der Szene

`get_world_mesh(map)` gibt das Netz als flache Felder (`WorldMeshDto`: `faces`, `textures` je
`{name, width, height, role, palette?}`, `corners` 12 Werte je Fläche, `uv` 8, `texture` Index in
`textures`, `layer`); `get_tileset_texture(package, texture)` das RGBA einer Textur mit der Palette
ihrer Kachel oder `null`. Seit 6.5b liefert `get_world_mesh` die Flächen von
`world::freeform::world_faces` (Raster, Teilmodell-Geometrie, dann die Freiform-Flächen auf Ebene
254); eine Textur, die keine Kachel des Tilesets ist (von einem Modell angehängt), trägt die Größe
aus dem Paket und die Rolle `piece`, `get_tileset_texture` findet sie im Paket (Palette
`palette_name(name)`, sonst 0). Ein Beifile, das fehlt, zeigt die Szene nicht (eine Freiform fehlt,
ein Teilmodell steht als Fußabdruck). Die Szene (`apps/studio/src/world/scene/worldMesh.ts`, `mapModel`) zeigt
für eine Welt-Karte das Netz **statt des GLB**, wenn es Flächen hat; mit 0 Flächen (eine
übernommene Karte, Form 1) oder wenn `get_world_mesh` scheitert das GLB, und das ist seit 6.3b das
gebackene Modell (über apicula aus dem Map-Asset). Das Netz: eine Geometrie je Textur, je Ebene eine Gruppe,
`MeshLambertMaterial` mit `alphaTest 0.5`. Ohne Textur (synthetisches Tileset, eine Textur im
Format 5, die noch keinen Decoder hat) steht ein Platzhalter in der Farbe der Rolle aus
`szene/gelaende` (`grass` `hohes-gras`, `water` `wasser`, `wall` `blockiert`, `piece` `spezial`,
sonst `unbekannt`). Überlagerungen bekommen `polygonOffset −Ebene`, Fußabdrücke `−(höchste
Überlagerung + 1)` statt −255 und sind halbdurchsichtig (Deckkraft 0,6). Das Canvas trägt
`data-world-mesh="<Karten aufsteigend, Komma>"` und `data-world-mesh-faces="<Summe>"`, solange es
mindestens eine Welt-Karte zeigt.

Der erste Aufbau traut `MapDto.world`; ein Auffrischen (⌘Z, ⇧⌘Z, Änderungen von Agenten) fragt
zuerst `get_world_mesh` und nimmt das Netz, wenn es Flächen hat, sonst das GLB. Vorschaubilder
der Karten kommen aus dem GLB, für eine Welt-Karte also aus dem gebackenen Modell. Im Inspector
einer Kartenzelle zeigt der Abschnitt „Quelle“ binäre Karte oder Welt-Karte mit der Zeile des
Backens und den Wegen „Karte als Quelle übernehmen“ und „Als binäre Karte übernehmen“; ein
`world.member-changed` ist eine Zeile in Technik › Textprojekt mit „Neu backen“ und „Als binäre
Karte übernehmen“. Gestaltung: [design-system/README.md](../../design-system/README.md),
„Nachtrag 6.3a“, „Nachtrag 6.3b“, „Nachtrag 6.4a“ und „Nachtrag 6.5b“.

## Malen (6.4a)

In der App malt das Werkzeug „Gelände malen“ (Taste B) eine Welt-Karte mit sieben Unterwerkzeugen.
Jeder Strich wird zu **einem** `world_edit` mit eigenem `label`, also einem Undo-Schritt, der im
selben Schritt backt. Quelle: Plan
[2026-11-07-phase-6-4a-malen.md](../superpowers/plans/2026-11-07-phase-6-4a-malen.md) („Entscheidungen“).
Implementierung: das reine Modell `apps/studio/src/world/paint/ops.ts` (Zellen, Token, `strokeRequest`,
`strokeLabel`), die Sitzung `world/paint/session.ts` und `usePaintSession.ts` (Warteschlange,
Neulesen), der Zeiger `world/scene/strokes.ts`, die Vorschau `world/scene/strokePreview.ts`, die
Unterwerkzeuge `workbench/paintTools.ts` und der Zustand `workbench/paint/PaintContext.tsx`.

**Ziel:** die gewählte Zelle der gezeigten Matrix, wenn ihre Karte eine Welt-Karte ist (Textprojekt).
Ein Druck auf eine andere Karte wählt deren Zelle und malt nicht. In einen Strich gehen nur Zellen
der Karte, auf der er begann (kartenlokal 0 bis 31); was über den Rand, in die Nachbarkarte oder ins
Leere der Matrix geht, fällt still weg. Auf einer **binären** Karte bleibt „Gelände malen“ das
Kollisionsmalen per Klick wie vor 6.4a; ohne Unterwerkzeuge, der Inspector zeigt oben „Boden malen:
Karte als Quelle übernehmen“ mit dem Weg dorthin (`world_adopt`, „Karte als Quelle übernehmen (Form 1)“).

| Unterwerkzeug | Taste | Geste | Zellen | Änderung (`changes`) | Verlaufstext |
|---|---|---|---|---|---|
| Pinsel | B | Ziehen | unter dem Zeiger, zwischen zwei Zeigerereignissen nach Bresenham ergänzt (8er-Nachbarschaft), jede einmal je Strich | `{layer: <aktiv>, cells, tile: <gewählt>}` | „Gemalt: Karte 0000 (6 Zellen)“ |
| Rechteck | P | Ziehen | das Rechteck zwischen Start- und Endzelle, zeilenweise | wie Pinsel | „Rechteck gemalt: …“ |
| Füllen | G | Klick | die 4er-Nachbarschaft gleichen Tokens (Drehung und Spiegelung zählen mit) in der aktiven Ebene ab der Zelle, aus dem zuletzt gelesenen Weltdokument | wie Pinsel | „Gefüllt: …“ |
| Radierer | E | Ziehen | wie Pinsel | `{layer: <aktiv>, cells, tile: "."}` | „Radiert: …“ |
| Pipette | I | Klick | eine | keine: die Kachel der aktiven Ebene, ist sie leer, die der obersten Ebene mit Kachel, wird die gewählte Kachel (mit Drehung und Spiegelung), danach gilt der Pinsel | – |
| Höhe | H | Klick oder Ziehen | wie Pinsel | je Zielhöhe eine `{cells, height}`: Höhe zu Beginn + 1, mit ⇧ − 1; immer Ebene 0 | „Höhe geändert: …“ |
| Rampe | R | Klick oder Ziehen | wie Pinsel | `{cells, ramp: "e1"}` (Richtung und Stufen der Werkzeugoption), mit ⇧ `"."`; Höhen setzt sie nicht; immer Ebene 0 | „Rampe gesetzt: …“ |

- **Unveränderte Zellen** (Token schon das Ziel) lässt `strokeRequest` weg; bleibt keine, geht **kein**
  Aufruf (kein Verlaufseintrag, kein „Nichts zu ändern.“). Die Zahl im Verlaufstext ist die der
  gesendeten Zellen („(1 Zelle)“); ein Aufruf über MCP ohne `label` heißt weiter „Welt geändert: …“.
- **Höhe an der Grenze:** eine Zelle, die schon auf 62 (bzw. mit ⇧ auf −16) steht, fällt weg; die
  Statusleiste sagt „3 Zellen schon an der Grenze (−16 bis 62).“, auch wenn deshalb kein Aufruf
  geht.
- **Kachel drehen und spiegeln:** Z dreht die gewählte Kachel eine Vierteldrehung weiter (`0041` →
  `0041r1` → … → `0041r3` → `0041`, die Spiegelung bleibt), X spiegelt (`f` an oder aus); im
  Unterwerkzeug Rampe dreht Z die Richtung im Uhrzeigersinn (Norden → Osten → Süden → Westen), 1 und
  2 wählen die Stufen. Die Tasten gelten, solange „Gelände malen“ eine Welt-Karte malt, vor den
  Tasten der Werkzeuge (G ist dann „Füllen“, „Gebäude“ nur über seine Taste in der Leiste). Esc führt
  zum Pinsel, ein zweites Esc zu „Auswählen“.
- **Rechtsklick** nimmt in jedem Unterwerkzeug die Kachel auf wie die Pipette; nur die Pipette selbst
  wechselt danach zum Pinsel. Mit ⌥ dreht bzw. verschiebt ein Druck die Kamera statt zu malen.
- **Reihenfolge:** die Sitzung schickt Striche über eine serielle Warteschlange; ein Strich wartet auf
  die Antwort des vorigen. Seine Anfrage entsteht beim Loslassen aus dem zuletzt gelesenen Dokument
  mit den noch unbeantworteten Strichen darüber (zwei schnelle Höhe-Striche ergeben + 2). Nach jeder
  Revision (Strich, ⌘Z, ein Agent auf derselben Sitzung, Speichern mit Abgleich) liest die Sitzung
  `world_get` und das Tileset neu, so rechnen Füllen und Höhe nie mit einem alten Stand.
- **Vorschau:** bis zur Antwort liegt je berührter Zelle eine Platte knapp über dem Boden
  (`HEIGHT_LAYER_Y` + 0,01, Deckkraft 0,85): Pinsel, Rechteck und Füllen mit dem Bild der Kachel aus
  dem Palettenspeicher, gedreht und gespiegelt wie im Netz (die UV der Platte aus `uv_of`, oben; ohne Bild die Platzhalterfarbe ihrer Rolle), der Radierer und das Entfernen einer
  Rampe schraffiert in `szene/gelaende/unbekannt`, Höhe und Rampe in der Farbe der Zielstufe; das Rechteck zeigt dazu den
  gestrichelten Rahmen des Entwurfs. Nach der Antwort kommen Netz, Höhen und Weltdokument neu, dann
  geht die Vorschau; eine Ablehnung nimmt die Vorschau weg und geht an den Fehlerweg, ohne
  Verlaufseintrag.
- **Zeit:** ein Strich über die Befehlsschicht (`world_edit` mit 37 Zellen auf `busy_world`,
  `world_get`, `get_world_mesh` über `dispatch`, mit Backen) braucht im Debug-Build im Median aus 10
  höchstens 300 ms (Test `world_stroke_over_the_command_path_is_fast` in
  `apps/studio/src-tauri/src/commands/world_map.rs`, strikt mit `G4_TIMING_STRICT=1`, druckt
  `world-stroke-bridge median_ms=<…> faces=<…>`; gemessen 49,74 ms bei Last etwa 20).

**Ebenen** (`world_layers {req: {map, op}}`, `world/edit.rs`): die Liste „Ebenen“ im Inspector legt
mit „Ebene hinzufügen“ eine Überlagerung an (`op: {add: {name}}`, Vorgabe des Namens „Ebene <n>“) und
entfernt eine mit „Entfernen“ (`op: {remove: {layer}}`, nur Überlagerungen), je ein Undo-Schritt, der
backt: „Ebene „Büsche“ angelegt: Karte 0000“ (die neue Ebene: Rolle `overlay`, 32 Zeilen `.`), „Ebene
2 „Büsche“ entfernt: Karte 0000“. Abgelehnt wie `world_edit` (kein Textprojekt, keine Welt,
gesperrte Karte, eine Welt, die sich nicht liest) und mit „Die Bodenebene (0) bleibt.“, „Eine Ebene
braucht einen Namen.“ (leer nach dem Trimmen), „Ebene „Büsche“ gibt es schon.“ (getrimmte Namen,
Groß- und Kleinschreibung zählt, auch gegen die Bodenebene) und „Ebene 3 gibt es nicht (Karte 0000
hat 2).“. Der Bericht ist ein `WorldReport` mit `cells: 0`. Umbenennen und Umordnen gibt es nicht.
Welche Ebene „Aktiv“ ist (der Pinsel malt dorthin) und welche in der Szene verborgen ist, merkt sich
nur die App, nicht die Datei; wird die aktive Ebene entfernt, gilt die darunter.

**Tüfteln:** dieselben Werkzeuge; die Statusleiste nennt die Token der Zelle unter dem Zeiger
(„Kachel 0041r1 · Ebene 0 · Höhe 2 (16 E)“), Palette und Tileset-Editor zeigen Nummern und hex. Einen
Editor für das JSON des Weltdokuments gibt es nicht.

**Beispiel:** `fixtures::gate_world(0, "0000", "0001", "0002")` auf dem synthetischen Tileset (Boden
`0000`, Gras `0001`, Wasser `0002`) entsteht aus einer leeren Welt `0000` auf Höhe 0 in sieben
Strichen; jede Zeile ist **ein** Aufruf (Zellen gekürzt):

| Strich | `changes` | Verlaufstext |
|---|---|---|
| Rechteck (16, 3)–(28, 10), Kachel `0001` | `[{layer: 0, cells: [[16, 3], …, [28, 10]], tile: "0001"}]` | „Rechteck gemalt: Karte 0000 (104 Zellen)“ |
| Rechteck (2, 24)–(8, 29), Kachel `0002` | `[{layer: 0, cells: [[2, 24], …], tile: "0002"}]` | „Rechteck gemalt: Karte 0000 (42 Zellen)“ |
| Höhe, im Zickzack über x 18–22, z 20–22 gezogen | `[{cells: [[18, 20], …], height: 1}]` | „Höhe geändert: Karte 0000 (15 Zellen)“ |
| Höhe, derselbe Zug | `[{cells: […], height: 2}]` | „Höhe geändert: Karte 0000 (15 Zellen)“ |
| Höhe, x 17, z 20–22 | `[{cells: [[17, 20], [17, 21], [17, 22]], height: 1}]` | „Höhe geändert: Karte 0000 (3 Zellen)“ |
| Rampe „nach Osten, 1 Stufe“, über x 16–17, z 20–22 | `[{cells: [[16, 20], [17, 20], …], ramp: "e1"}]` | „Rampe gesetzt: Karte 0000 (6 Zellen)“ |
| Tür-Handwert (kein Werkzeug, über MCP) | `[{cells: [[12, 21]], permission: "6980"}]` | „Welt geändert: Karte 0000 (1 Zelle)“ |

Dieselbe Folge malen die Szenarien `pt-gemalt.yaml` und `hg-gemalt.yaml` über MCP und das Phasen-Gate
über die Oberfläche (dort mit den Kacheln des Spiels und der Rampe in zwei Strichen, weil ein Zug
der Testumgebung, `left_click_drag`, nur von einem Punkt zu einem zweiten führt). Für Agenten: [mcp.md](../mcp.md), „Malen (6.4a)“. Gestaltung:
[design-system/README.md](../../design-system/README.md), „Nachtrag 6.4a“.

## Was später kommt

- **Seit 6.4a da:** Malwerkzeuge, Palette, Ebenen (`world_layers`) und Tileset-Editor in der App
  („Malen (6.4a)“, [tileset.md](tileset.md) „Palette und Tileset-Editor (6.4a)“), die kuratierten MCP-Werkzeuge
  (`world_get`, `world_edit`, `tileset_get`) und das Rezept `new_painted_map`
  ([recipes.md](recipes.md)); die Werkzeug-Ops rechnet die App auf die Zellen von `world_edit`
  herunter. **Eine Welt-Karte kopieren** geht über `new_painted_map` mit `source_map`; `create_area`
  und „Gebiet erweitern“ mit einer Welt-Karte als Quelle kopieren weiter die gebackene **binäre**
  Karte.
- **Nicht in 6.4a, offen:** Texturen im Format 5 (4 × 4 komprimiert) in der Szene und in der Palette
  dekodieren (bis dahin der Platzhalter der Rolle; ein eigener Posten nach 6.4b, wenn ein
  Vanilla-Kartenpaket welche nutzt); der Dateischritt von `tileset_save`/`tileset_edit` hängt noch am
  Asset `HEADER_TABLE`; die VRAM-Schätzung `terrain::vram_bytes` zählt geteilte Texel doppelt (für
  eine Welt-Karte kommt das Budget aus dem Bake). Beides liegt in `world/tileset.rs` bzw. `terrain/`
  und wird nach 6.4b neu bewertet. Pinselgröße über 1, Stempel aus einer Auswahl, Zufall und Formen
  außer dem Rechteck gibt es nicht.
- **6.4b:** Kantensätze (`edge_sets` im Tileset) mit Terrain-Pinsel, Wände zu Nachbarkarten und die
  Muster-Shader der Schicht „Gelände“.
- **Seit 6.5b da:** Freiform-Beifiles und die Geometrie der Teilmodelle im Backen („Freiform und
  Beifiles (6.5b)“, „Teilmodelle mit Geometrie (6.5b)“), Form 2 von „Karte als Quelle übernehmen“
  („Form 2: Als glTF übernehmen (6.5b)“) und der Austausch mit Tiled ([tmj.md](tmj.md)). Ein
  Verbatim-Modell zusammen mit gemalten Kacheln bleibt `world.verbatim-mixed`; der Weg dorthin ist
  Form 2.
- **Nicht in 6.5b, offen:** Ursache C von `world.adopt-texture` (Palette, die `pack_textures` nicht
  findet) und Vanilla-Modelle über ±512 (`model.out-of-range`) in Form 2; eine Freiform verschieben
  oder skalieren (heute entfernen und neu hinzufügen), sie in der App entfernen; Kollision oder BDHC
  aus Polygonen; die Teilmodell-Geometrie zeigt die Szene halbdurchsichtig wie einen Fußabdruck.
  Vanilla-Teilmodelle herauslösen, Vanilla-NSBMD zurück ins Raster, Bearbeiten von
  Vanilla-Modellen über den Codec, eine Tiled-Erweiterung und Automapping sind nicht in Phase 6.
