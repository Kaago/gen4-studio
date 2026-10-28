# Textprojekt

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Das Textprojekt ist ein
DSPRE-Ordner, dessen Spieldaten zusätzlich als JSON unter `data/` liegen, damit das Projekt in Git
leben und von Hand bearbeitet werden kann. Die Wurzel des Textprojekts ist der DSPRE-Ordner selbst;
Texte und Scripts bleiben, wo sie sind (`expanded/`). Implementierung:
`crates/g4-kernel/src/textproject/` (`export.rs`, `import.rs`, `check.rs`, `arts.rs`, die
Schemata unter `schema/`, `part.rs`, `record.rs`, `registry.rs`, `symbols.rs`, `base_layer.rs`,
`quirks.rs`, `manifest.rs`, `migrate.rs`, `canon.rs`, `locate.rs`, `codes.rs`). Alle Beispiele
hier stammen aus dem synthetischen Testprojekt, nicht aus einer ROM.

Nur Platin und HeartGold/SoulSilver; Diamant/Perl lehnt der Export ab („Kein Textprojekt für
Diamant/Perl in Phase 4.“).

## Ablage

| Pfad | Inhalt |
|---|---|
| `g4-project.json` | Manifest; seine Anwesenheit macht den Ordner zum Textprojekt |
| `data/<art>/NNNN.json` | eine Entität je Datei, `NNNN` vierstellig ab `0000`, lückenlos |
| `data/<art>/NNNN.<ext>` | Beifiles einer Entität (gleicher Stamm) und die Member der Binärarten |
| `data/ids/{maps,trainers,trainer_classes,places}.json` | Register der getauften IDs |
| `data/raw/<pfad>` | Blobs: geänderte Dateien, die keine Entität und keine Quelle besitzt |
| `data/rotom-quirks.json` | rotoms Binär-Eigenheiten je Script-Quelle |
| `expanded/textArchives/NNNN.json` | Texte (wie in 4.2, unverändert) |
| `expanded/scripts/NNNN.rotom`, `NNNN.json` | Scripts (rotom-Quellen, Level-Scripts) |

Jede Datei ist kanonisches JSON: `serde_json::to_vec_pretty` (zwei Leerzeichen), Schlüssel in der
Reihenfolge des Typs, Zeilenende `\n`, eine Zeilenschaltung am Ende. Geschrieben wird nur, wenn sich
der Inhalt ändert (atomar über `.g4/tmp/`); eine gleiche Datei behält ihre mtime.

## Manifest

```json
{
  "format": "g4-project",
  "schemaVersion": 1,
  "game": {
    "code": "CPUE",
    "family": "pt",
    "language": "en_US"
  },
  "studioVersion": "0.1.0",
  "schemas": {},
  "plugins": []
}
```

| Feld | Inhalt |
|---|---|
| `format` | immer `g4-project` |
| `schemaVersion` | Version der Dateiformate unter `data/` (siehe Schema-Version und Migration) |
| `game.code` | Spielcode aus `header.yaml`; der Import prüft ihn gegen das Projekt |
| `game.family` | `pt` oder `hgss` |
| `game.language` | Sprachschlüssel (`de_DE`, `en_US`, …) |
| `studioVersion` | Version des Kernels, der zuletzt exportierte |
| `schemas`, `plugins` | reserviert; Phase 4 hält den Inhalt unverändert, liest ihn aber nicht (`textproject.reserved`) |

Keine Basis-SHA-1: die Basis-ROM nennt `.g4/base.json` ([project_base.md](project_base.md)).

## Arten

| Art (`data/<art>/`) | Datei je | Member |
|---|---|---|
| `headers` | Header | Ausschnitt der Header-Tabelle in `arm9.bin` (oder Datei von `unpacked/dynamicHeaders`) und Eintrag von `mapname.bin` |
| `matrices` | Matrix | `matrices` |
| `maps` | Karte | `maps` |
| `area_data` | Bereichsdatei | `areaData` |
| `events` | Ereignisdatei | `eventFiles` |
| `encounters` | Begegnungsdatei | `encounters` |
| `species` | Spezies (mit Formen) | `personalPokeData`, `learnsets`, `evolutions` derselben Nummer |
| `moves` | Attacke | `moveData` |
| `items` | Item-ID | `itemData` über die ARM9-Item-Tabelle |
| `trainers` | Trainer | `trainerProperties` und `trainerParty` derselben Nummer |
| `trainer_text_table` | Archiv-Member | `trainerTextTable` |
| `trainer_text_offsets` | Archiv-Member | `trainerTextOffset` |
| `map_textures`, `building_textures` | – | `mapTextures`, `buildingTextures`: nur `NNNN.nsbtx`, wo der Member von der Basisschicht abweicht |
| `building_models`, `interior_building_models` | – | `exteriorBuildingModels`, `interiorBuildingModels` (nur HeartGold/SoulSilver): nur `NNNN.nsbmd` bei Abweichung |

Eine Art, deren Archiv das Spiel nicht hat, fehlt. Items gibt es nur mit geprüfter Item-Tabelle
(`arm9_tables.yaml`); ohne sie laufen die Item-Member wie alles Nicht-Abgebildete (Blob bei
Abweichung). Teilen sich IDs einen Member, trägt die erste die Daten, jede weitere
`{"id": …, "same_member_as": "<ID>"}`; ein Member, auf den keine ID zeigt, ist ein Blob, wo er
abweicht.

Beispiele aus dem synthetischen Testprojekt (gekürzt, `…` steht für weggelassene Felder):

`headers` (`data/headers/0000.json`):

```json
{
  "id": "MAP_T01",
  "internal_name": "T01",
  "area_data_id": 0,
  "matrix_id": 0,
  "script_file_id": 0,
  "level_script_id": 1,
  "text_archive_id": 433,
  "location_name": "PLACE_TWINLEAF_TOWN",
  "…": "…"
}
```

`matrices` (`data/matrices/0000.json`):

```json
{ "name": "m", "width": 2, "height": 1, "maps": ["0000 0001"] }
```

`maps` (`data/maps/0000.json`):

```json
{
  "source": "binary",
  "permissions": ["0080 0100 0200 …", "…"],
  "buildings": [{ "model_id": 7, "position": [196608, 0, 0], "rotation": [171, 0, 0], "scale": [4096, 0, 0] }],
  "model": { "hex": "000000000000" },
  "bdhc": { "hex": "cafe01" }
}
```

`area_data` (`data/area_data/0001.json`, Platin):

```json
{ "buildings_tileset": 1, "map_tileset": 2, "unknown": 0, "light_type": 0 }
```

`events` (`data/events/0000.json`):

```json
{ "bg_events": [], "object_events": [], "warp_events": [], "coord_events": [] }
```

`encounters` (`data/encounters/0000.json`, Platin):

```json
{
  "data": {
    "grass_rate": 10,
    "grass_00_level": 2,
    "grass_00_species": "SPECIES_BULBASAUR",
    "grass_01_level": 0,
    "grass_01_species": "SPECIES_NONE",
    "…": "…",
    "super_rod_4_species": "SPECIES_NONE"
  }
}
```

`species` (`data/species/0001.json`):

```json
{
  "id": "SPECIES_BULBASAUR",
  "personal": { "hp": 2, "type1": "TYPE_FIGHTING", "ability1": "ABILITY_STENCH", "…": "…", "tm_bits": "01000000000000000000000000000000" },
  "learnset": { "moves": [{ "level": 1, "move": "MOVE_POUND" }, { "level": 5, "move": "MOVE_KARATE_CHOP" }], "terminated": true },
  "evolution": { "method0": 4, "param0": 16, "target0": "SPECIES_IVYSAUR", "…": "…" }
}
```

`moves` (`data/moves/0001.json`):

```json
{
  "id": "MOVE_POUND",
  "data": {
    "effect": 0,
    "category": 0,
    "power": 10,
    "move_type": "TYPE_NORMAL",
    "accuracy": 100,
    "pp": 35,
    "…": "…",
    "pad_0e": "0000"
  }
}
```

`items` (von Hand, das synthetische Projekt hat keine geprüfte Item-Tabelle): die erste ID eines
Members trägt die Daten (`data/items/0001.json`), eine weitere ID desselben Members verweist auf sie
(`data/items/0002.json`):

```json
{
  "id": "ITEM_MASTER_BALL",
  "data": { "price": 100, "hold_effect": 0, "…": "…", "natural_gift_type": "TYPE_NORMAL", "…": "…" }
}
```

```json
{ "id": "ITEM_ULTRA_BALL", "same_member_as": "ITEM_MASTER_BALL" }
```

`trainers` (`data/trainers/0001.json`):

```json
{
  "id": "TRAINER_YOUNGSTER_KAI",
  "data": {
    "class": "TRAINER_CLASS_YOUNGSTER",
    "has_moves": false,
    "has_items": false,
    "unknown": 0,
    "items": ["ITEM_NONE", "ITEM_NONE", "ITEM_NONE", "ITEM_NONE"],
    "ai": 0,
    "battle_type": 0,
    "party": [{ "species": "SPECIES_BULBASAUR", "form": 0, "level": 5, "iv_or_difficulty": 0, "byte1": 0, "ball_capsule": 0 }]
  }
}
```

`trainer_text_table` (`data/trainer_text_table/0000.json`):

```json
{ "entries": [[1, 0], [1, 2]] }
```

`trainer_text_offsets` (`data/trainer_text_offsets/0000.json`):

```json
{ "offsets": [0, 0] }
```

Karten: `source` ist in Phase 4 immer `binary` (`textproject.map-source` sonst), `permissions` sind
32 Zeilen zu 32 Zellen `kkcc` (Art, Kollision in Hex), `buildings_tail` (Hex) nur, wenn da, `bgs`
nur auf HeartGold/SoulSilver. Header: `internal_name_bytes` (Hex) nur, wenn die 16 Byte von
`mapname.bin` nicht „Name + NUL-Füllung“ sind; ohne den Patch „Dynamische Header“ ändert sich die
Zahl der Header nicht (`textproject.header-count`).

## Teile und Gesperrtes

Ein binärer Teil (`Part`) ist ein Objekt mit genau einem Schlüssel:

| Form | Wann |
|---|---|
| `{"hex": "cafe01"}` | höchstens 64 Byte |
| `{"base_sha1": "<40 Hex>"}` | die Bytes gleichen demselben Teil der Basisschicht |
| `{"file": "0003.nsbmd"}` | sonst: Beifile mit dem Stamm der Entität daneben |

Ein Beifile gewinnt: seine SHA-1 steht nicht in der JSON, es kann von Hand ersetzt werden. Beim
Import kommt ein `base_sha1`-Teil vom aktuellen Member, wenn dessen Teil dieselbe SHA-1 hat, sonst
aus der Basisschicht (`textproject.base-missing`, `textproject.base-mismatch`). Die Basis wird nur
für geänderte `base_sha1`-Teile gebraucht.

Ein Member, das nicht decodiert oder sich nicht Byte für Byte zurückschreibt, steht als
`{"locked": "<Grund>", "raw": <Part>}` an der Stelle seines Wertes (Trainer:
`{"locked", "props", "party"}`); der Import schreibt `raw` unverändert. Header haben keine gesperrte
Form: jedes ihrer 24 Bytes liegt in genau einem Feld.

## Feldnamen

Die Namen sind die des Codecs: `RawRecord::FIELDS` für die festen Datensätze (Spezies, Attacken,
Items, Begegnungen), die Rust-Feldnamen von uxie und `g4-formats` für die übrigen (`dest_header_id`,
`model_id`, …). Ein Container mit Bit-Feldern erscheint nur als seine Bit-Felder; übrige Bits stehen
als `<container>_rest` nur, wenn sie ≠ 0 sind. `Bytes` (Füllbytes, Bitfelder ohne Namen) sind
Kleinbuchstaben-Hex. Kein Feld kommt doppelt vor. Die vollständige Liste aller Schlüsselpfade steht in
`crates/g4-kernel/src/textproject/schema.snap` (`?` = darf fehlen, `<member>` = kann gesperrt sein,
`→PREFIX` = Verweis).

## Verweise

Felder, die eine Spezies, Attacke, ein Item, eine Fähigkeit, einen Typ, Header (`MAP_*`), Ort
(`PLACE_*`) oder eine Trainerklasse nennen, schreibt der Export als ID, wenn der ganze Feldwert eine
Nummer der Tabelle bzw. des Registers ist, sonst als Zahl. Der Import liest ID **oder** Zahl; eine
unbekannte ID ist `textproject.unknown-id`, eine Zahl außerhalb des Feldes `textproject.schema`. Das
Feld `id` einer Spezies, Attacke, eines Items, Trainers oder Headers wird gegen Tabelle bzw.
Register geprüft (`textproject.id-mismatch`) und nie umbenannt. Die Gitter der Matrizen bleiben
Zahlen.

## Symboltabellen und Register

Spezies (mit Formen), Attacken, Items, Fähigkeiten und Typen haben feste englische Tabellen in
`crates/g4-kernel/metadata/symbols/` (aus [PokeAPI](https://github.com/PokeAPI/pokeapi), BSD
3-Clause, Lizenz in `LICENSE-PokeAPI.txt`). Eine Nummer ohne Eintrag heißt `PREFIX_NNNN` (vier
Stellen, auch über der Tabelle).

Maps, Trainer, Trainerklassen und Orte haben Register `data/ids/<art>.json`: ein JSON-Objekt
ID → Nummer, Schlüssel sortiert, ohne eigenes Versionsfeld.

```json
{
  "TRAINER_0000": 0,
  "TRAINER_POKEMON_TRAINER_MIA": 2,
  "TRAINER_YOUNGSTER_KAI": 1
}
```

Getauft wird beim Export in der Projektsprache: Header `MAP_` + interner Name aus `mapname.bin`,
Trainer `TRAINER_` + Klasse und Name (ohne Namen `TRAINER_NNNN`, auch wenn der Name allein
normalisiert nichts ergibt, etwa `-`), Klassen `TRAINER_CLASS_`, Orte `PLACE_` aus der
Ortsnamen-Bank. Normalisierung: Markup weg, `♂/♀` → `_M/_F`, deutsche Umlaute nur
bei deutscher Projektsprache, Diakritika weg, Nicht-Alphanumerisches → `_`, groß; leer oder nur
Ziffern → `PREFIX_NNNN`. Gleichstand in Nummernreihenfolge `_2`, `_3`, … (nächstes freies Suffix).
Eingefroren: ein vorhandener Eintrag bleibt, auch wenn sich der Name ändert oder die Nummer
verschwindet (`textproject.id-dangling`). Ein Register darf keine ID doppelt nennen, keine Nummer
doppelt vergeben und keine fremde Fallback-ID tragen (`textproject.schema` an der Stelle der ID).

## Basisschicht und Blobs

Die Basisschicht ist die Basis-ROM (aus `.g4/base.json`) mit `extract_rom` entpackt, jedes Archiv in
`unpacked/`, einmal je Basis unter `.g4/cache/base-layer/<sha1>/` (fertig mit der Marke
`.complete`, jederzeit löschbar). Sie ist der Maßstab für „weicht ab“.

`data/raw/<pfad>` hält jede Datei der Wurzel, die von der Basisschicht abweicht oder ihr fehlt,
außer: versteckten Einträgen, `data/`, `expanded/`, `build/`, `plugins/`, den Projektdateien
(`g4-*.json`, `rotom.toml`), `*.nds`, `*.srl`, `*.xdelta`, Membern mit Entität, Text- und
Script-Membern mit Quelle, `mapname.bin`, `unpacked/dynamicHeaders/` bei angewandtem Patch, den
gepackten NARCs aller Archive mit `unpacked/`-Ordner (Build-Ausgabe) und Ordnern unter `unpacked/`,
die der Build nicht packt (nur genannt, `not_built`). `arm9/arm9.bin` wird mit ausgeblendeter
Header-Tabelle verglichen, YAML nach ihrem Wert. Der Import legt jeden Blob an seinen Pfad; ein Blob
außerhalb dieses Bereichs, an einem Pfad einer Entität oder als NARC einer Entitätsart ist
`textproject.raw-path`. Ein vanilla DSPRE-Ordner hat keinen Blob.

## rotom-Quirks

`data/rotom-quirks.json` hält rotoms Binär-Eigenheiten (Zahl der Endmarken einer Sprungtabelle,
Füllbytes eines Level-Scripts) je Quelle, Schlüssel wie in `.rotom/status/compile-state.json`
(relativ zur Wurzel, `/`), nur Einträge, die vom Standard abweichen; immer geschrieben, auch leer:

```json
{
  "quirks": {
    "expanded/scripts/0101.rotom": {
      "jump_table_end_marker_count": 0
    }
  }
}
```

Der Export nimmt den Compile-Zustand (dazu Einträge der Datei, deren Quelle der Zustand nicht kennt,
aber noch existiert); Import und Build legen die Datei vor jedem `compile` in den Compile-Zustand.
Ein Schlüssel, der kein Pfad einer vorhandenen Quelle unter `expanded/scripts` ist, ist
`textproject.quirk-key`.

## Export, Import, Check

**Export** (`textproject::export`, `g4k text export`): lehnt ab bei Diamant/Perl, ohne Basismarke,
bei anderer Basis-ROM, älterer oder neuerer Schema-Version, Texten im Konflikt
(`TextConflict`), fehlenden Item-Membern (von der Item-Tabelle genannt oder eine Lücke; erst
`g4k text import`) und nicht übernommenen Scripts ohne `adopt_scripts`. Dann: Basisschicht (nur wenn
sie fehlt), alle abgebildeten Archive entpackt, Text-JSON nachziehen (fehlende aus dem Member, die
nach 4.2 der Member gilt), Taufe, je Art die Dateien, Register, Quirks, Blobs, Manifest. In seinen
eigenen Ordnern löscht der Export Dateien, die er nicht mehr schreibt (alte Beifiles, alte Blobs);
unbekannte Ordner unter `data/` bleiben (`textproject.unknown-art`). Ein zweiter Export schreibt
nichts.

**Import** (`textproject::import`, `g4k text import`): liest und prüft **alles**, bevor er die
erste Datei schreibt; ein Fehler und nichts wird geschrieben, alle Probleme kommen mit Datei, Zeile,
Spalte (UTF-16) und dem JSON-Pfad des Feldes (`textproject.schema: data.powr: …`). Dann: Blobs,
Member je Art (Archiv vorher entpackt), Header zuletzt, Quirks in den Compile-Zustand, danach
`build::prepare` (Scripts kompilieren, Texte kodieren). Ein Member jenseits der letzten JSON seiner
Art ist `textproject.member-without-json` (Löschen kommt mit 4.3b); neue Entitäten werden angehängt,
nur lückenlos. Binärarten ohne Datei kommen aus der Basisschicht; ohne Schicht und ohne Basis-ROM
bleiben sie, wie sie sind (`textproject.base-unchecked`, eine Warnung). Lässt sich die Basisschicht nicht
lesen oder aus der gegebenen Basis-ROM nicht anlegen, ist das `textproject.base-missing` (ein Fehler).

**Check** (`textproject::check`, `g4k text check`): wie der Import, schreibt nie (auch keine
Basisschicht); nennt die Probleme, die Hinweise und die Dateien, die ein Import schreiben würde.

**Build** (Schritt `textproject`, `textproject::step`, nur mit `g4-project.json`, vor `rotom` und
`chatot`): plant wie der Import, schreibt aber nur die Einheiten, deren JSON-Seite gilt. Eine
Einheit ist eine Entität (JSON mit Beifiles ↔ ihre Member), die Item-Dateien eines Item-Members, ein
Binärmember, ein Blob (`data/raw/X` ↔ `X`), die Header-Tabelle (alle `data/headers/*` ↔ Ausschnitt in
`arm9.bin` bzw. Dynamic-Header-Dateien und `mapname.bin`) oder die Quirks-Datei (↔ Compile-Zustand).
Je Einheit, deren erzeugte Bytes vom Member abweichen:

| Lauf | JSON-Seite geändert | Member-Seite geändert | Wirkung |
|---|---|---|---|
| inkrementell | ja | nein | Member schreiben |
| inkrementell | nein | ja | Member bleibt; Hinweis `textproject.member-newer: <member> ist neuer als <json>; g4k text export übernimmt ihn, g4k text import verwirft ihn.` |
| inkrementell | ja | ja | Fehler `textproject.conflict: <json> und <member> sind beide geändert; nichts geschrieben.` |
| voll (kein Eintrag, anderes Salz) | – | – | Member fehlt oder gleicht dem Member der Basisschicht → schreiben; sonst Fehler `textproject.unsynced: <member> weicht von <json> und von der Basis ab; g4k text import (JSON gilt) oder g4k text export (Member gilt).` |

Voll läuft der Schritt auch, wenn `.g4/build/textproject.json` fehlt, nicht lesbar ist oder eine
andere Version hat (Grund `textproject.sync-lost`): dann ist unbekannt, welche Member er als neuer
behalten hat, also urteilt er über die Basisschicht. Ein Member ohne JSON (von Studio oder DSPRE
angehängt, oder ein Teil, der in der JSON fehlt) ist inkrementell `member-newer`, wenn sich sein
Member geändert hat oder schon behalten war, voll `unsynced`, sonst das
`textproject.member-without-json` des Imports.

Gleichen die erzeugten Bytes dem Member, geschieht nichts. Eine Änderung am Manifest oder an einem
Register plant alle Einheiten und gilt für jede abweichende als JSON-Änderung; eine abweichende
Einheit ohne Änderung auf einer der beiden Seiten (die Items werden als Ganzes geplant) gilt als
`member-newer`. Die als neuer behaltenen Member merkt sich der Schritt in
`.g4/build/textproject.json` (nach jedem erfolgreichen Lauf geschrieben, auch leer), damit eine
spätere Änderung ihrer JSON ein Konflikt ist und kein stilles Überschreiben; ein voller Lauf, ein
erfolgreicher Export und ein erfolgreicher Import vergessen sie (danach gleicht jede JSON ihrem
Member). Die Quirks gehen immer in den Compile-Zustand (rotoms Cache); ist der Schritt
übersprungen und hat der Compile-Zustand sie verloren, legt `prepare` sie vor `rotom` wieder hinein. Erst werden alle Einheiten entschieden, dann geschrieben: ein Fehler und nichts
wird geschrieben. Ohne Basismarke oder Basisschicht darf ein voller Lauf nur fehlende Member
schreiben.

## Bearbeiten im Studio

Im Textprojekt (Manifest mit der aktuellen Schema-Version, sonst lehnt `EntitySource::open` ab) ist
die JSON die Quelle des Stores (`textproject::source::EntitySource`). Der
Store behält seine Byte-Form, die der Member (ein Trainer als Eigenschaften und Party in einem,
die Header-Tabelle als ihre 24-Byte-Datensätze); Laden liest die JSON und importiert sie gegen
die Member auf der Platte. Eine JSON, die sich nicht lesen lässt, lehnt das Laden ab, mit Datei,
Zeile, Spalte und JSON-Pfad wie der Import.

**Einheit und Schlüssel.** Gespeichert wird je Einheit, wie im Import: eine Entität aus ganzen
Membern (eine Spezies sind Personal, Lernset und Entwicklung), die Item-Dateien aller IDs, deren
Eintrag in der Item-Tabelle auf denselben Member zeigt, die Header-Tabelle (alle
`data/headers/*.json`) oder ein Binärmember. Der Schlüssel einer Einheit (im Abgleich und in den
Antworten der App) ist der Pfad ihrer JSON (`data/species/0025.json`), `data/items/@<Member>`,
`data/headers/` oder `data/<art>/NNNN.<ext>`; außerhalb der Store-Assets `data/raw/<pfad>`,
`data/rotom-quirks.json`, `expanded/textArchives/NNNN.json` und `expanded/scripts/NNNN` (beide
Quellformen). Texte und Scripts behalten ihre eigene Quelle.

**Reihenfolge beim Speichern.** Erst die JSON der Einheit (mit Beifiles, jede Datei nur, wenn sie
sich ändert, alte Beifiles der Entität fort), dann der Member wie in einem DSPRE-Ordner (er ist
Build-Zwischenstand, den Szene, Prüfungen und DSPRE lesen), dann die Aufzeichnung beider für den
Abgleich. Die Einheit fällt aus der Liste der als neuer behaltenen Member
(`.g4/build/textproject.json`). Die JSON entsteht aus den Bytes des gespeicherten Teils und den
übrigen Teilen der Einheit (ungespeicherte Werte des Stores, sonst gespeicherte, sonst aus der JSON
gelesen, sonst der Member) genau so, wie sie ein voller Export schriebe: Speichern von Personal 25
ändert nur `data/species/0025.json`, Speichern der Header-Tabelle nur die Header, deren JSON sich
ändert. Die internen Namen der Header kommen dabei aus den Header-Dateien, nur für eben angelegte
Header aus `mapname.bin`. Binärmember, die am Store vorbei geschrieben werden (Zurücksetzen von
Texturen und Modellen, Kartenimport), exportiert `after_member_write` ebenso: die Datei unter
`data/<art>/` entsteht, wo der Member von der Basisschicht abweicht, und geht, wo er ihr wieder
gleicht.

**Neue Entitäten.** Ein angehängter Member (neuer Trainer, neue Map, neuer Header) bekommt seine
JSON sofort, nicht erst beim nächsten Export: fehlt der Entität ihre ID im Register, tauft das
Speichern wie der Export (Trainer aus Klasse und Name, Maps aus dem internen Namen) und schreibt
das geänderte Register gleich mit. Diese Schritte sind wie das Anhängen selbst nicht rückgängig zu
machen.

**Teile ohne Basisschicht.** Speichern legt die Basisschicht nie an und braucht weder sie noch die
Basis-ROM. Fehlt sie, bleibt ein Teil `{"base_sha1"}`, solange seine Bytes die SHA-1 behalten, die
die JSON nannte; ein geänderter Teil wird Beifile oder `{"hex"}`. Ein Binärmember ohne Datei unter
`data/` bleibt ohne Datei, solange seine Bytes die SHA-1 behalten, mit der er gelesen wurde; sonst
wird er als Datei geschrieben. Der nächste volle Export mit Basisschicht normalisiert beides
(ein auf die Basis zurückgesetzter Teil wird wieder `base_sha1`, eine der Basis gleiche Datei
geht).

## DSPRE parallel

DSPRE bleibt neben dem Studio nutzbar: Es liest und schreibt die Member, das Studio die JSON.
Der Abgleich (`textproject::sync`) findet, was sich seit der letzten Aufzeichnung (`sync` in
`.g4/build/state.json`, siehe [build_state.md](build_state.md)) auf welcher Seite geändert hat,
übernimmt Einseitiges und meldet Beidseitiges.

Kopflos gleicht `g4k text sync` ab: Einseitiges wird übernommen (Zeilen `dspre <schlüssel> <label>`,
`files …`), Konflikte und fehlerhafte Dateien werden gelistet (`conflict …`, `broken …`, die
Probleme auf stderr); solange eines davon bleibt, endet der Befehl mit Exit 1.
`g4k text sync --keep studio|dspre <schlüssel>` löst einen Konflikt.

**Beobachtete Dateien.** Quellseite: `g4-project.json`, alles unter `data/`,
`expanded/textArchives/*.json`, alles unter `expanded/scripts/`. DSPRE-Seite: jede Datei im
Bereich der Blobs (siehe „Basisschicht und Blobs“: Member, `arm9/`, Overlays, `files/`,
`unpacked/**`), ohne die gepackten NARCs, deren `unpacked/`-Ordner es gibt (der Build schreibt sie),
und ohne die Ordner unter `unpacked/`, die der Build nicht packt. Versteckte Einträge (`.g4/`,
`.rotom/`) nie. Ein Scan nimmt von jeder Datei den `stat`; gehasht wird nur, wo der `stat` vom
aufgezeichneten abweicht (bei gleicher Größe) oder der Eintrag racy ist. So kostet ein Scan ohne
Änderung nur die `stat`-Aufrufe; eine DSPRE-Änderung in einem nicht abgebildeten Ordner wird ein
Blob statt still verloren.

**Einheiten** (Schlüssel und Label, deutsch):

| Einheit | Schlüssel | Quellen ↔ Member | Label |
|---|---|---|---|
| Entität | `data/<art>/NNNN.json` | JSON und Beifiles ↔ ihre Member | `Pokémon 0025 · SPECIES_PIKACHU`, `Attacke 0001 · MOVE_POUND`, `Trainer 0001 · TRAINER_…`, `Karte 0003`, `Matrix 0000`, `Gebietsdaten 0001`, `Eventdatei 0000`, `Begegnungsdatei 0000`, `Trainertexte 0000`, `Trainertext-Offsets 0000` |
| Item-Member | `data/items/@MMMM` | Dateien aller Item-IDs mit diesem Member ↔ der Member | `Item 0004 · ITEM_POKE_BALL` (die erste ID) |
| Header-Tabelle | `data/headers/` | alle Header-Dateien ↔ Tabelle in `arm9.bin` bzw. Dynamic-Header-Dateien und `mapname.bin` | `Header-Tabelle` |
| Binärmember | `data/<art>/NNNN.<ext>` | die Datei (ohne sie: der Member der Basis) ↔ der Member | `Kartentextur 0003`, `Gebäudetextur …`, `Gebäudemodell …`, `Innenraum-Gebäudemodell …` |
| Blob | `data/raw/<pfad>` | `data/raw/<pfad>` (ohne sie: die Datei der Basisschicht) ↔ `<pfad>` | `Datei <pfad>` |
| Quirks | `data/rotom-quirks.json` | die Datei ↔ rotoms Compile-Zustand | `rotom-Eigenheiten` |
| Textarchiv | `expanded/textArchives/NNNN.json` | JSON ↔ Member | `Textarchiv 0012` |
| Scriptdatei | `expanded/scripts/NNNN` | `.rotom` bzw. `.json` ↔ Member (nur mit übernommenen Scripts) | `Scriptdatei 0101` |

`arm9.bin` gehört der Header-Tabelle und als Blob sich selbst (verglichen ohne die Tabelle). Eine
Datei, die keine Einheit entscheidet (Register, Manifest, ein unbekannter Ordner unter `data/`,
eine Include-Datei der Scripts), wird nur aufgezeichnet; eine Änderung am Manifest oder an einem
Register beurteilt alle Entitäten neu und zählt für jede abweichende als Änderung der Quellseite.

**Regeltabelle.** Für jede Einheit mit einer geänderten Datei:

| Quellseite | Member-Seite | Bytes aus der Quelle gleich den Membern | Seite | Übernahme |
|---|---|---|---|---|
| Inhalt gleich | Inhalt gleich | – (nur der `stat` wanderte) | abgeglichen | neu aufzeichnen |
| – | – | ja | abgeglichen | neu aufzeichnen |
| unverändert | geändert | nein | **aus DSPRE** | Quelle aus dem Member schreiben |
| geändert | unverändert | nein | **aus Datei** | Member aus der Quelle schreiben |
| geändert | geändert | nein | **Konflikt** | nichts; Wahl in Technik › Textprojekt |
| nicht lesbar | – | – | **fehlerhaft** | nichts; Datei, Zeile, Spalte |

Ohne `sync` (erster Abgleich) gelten beide Seiten als geändert: Gleiche Einheiten werden
aufgezeichnet, abweichende sind Konflikte. Eine Einheit mit Membern und ganz ohne JSON (ein von
DSPRE angehängter Member) ist aus DSPRE, eine mit JSON und ohne Member aus Datei. Ein seit der
Aufzeichnung gelöschter Member oder eine gelöschte JSON, deren Member bleibt, ist fehlerhaft: Der
Abgleich löscht keine Entität, Löschen kommt mit der Referenzsperre. Ein gelöschter Blob unter
`data/raw/` setzt die Datei auf die Fassung der Basisschicht zurück (ohne Schicht fehlerhaft).

**Übernahme.** Aus DSPRE: Entitäten, Items, Header (mit den internen Namen aus `mapname.bin`) und
Binärmember über `EntitySource` wie ein Speichern (neue Entitäten getauft), ein Blob als Kopie nach
`data/raw/` (gleicht die Datei der Basisschicht, geht der Blob), ein Text über
`resolve_conflict(…, Member)`, ein Script über `decompile_member` und `reset_script` (Quelle und
Compile-Eintrag; dekompiliert rotom den Member nicht, ist die Einheit fehlerhaft mit diesem Grund).
Aus Datei: der Import-Plan nur dieser Einheiten (`plan_import` mit ihren Quellen) und
`apply_import`; ein Blob über `place_raw` (`arm9.bin` behält seine Header-Tabelle); ein Text wird
kodiert; ein Script bleibt (rotom kompiliert es beim Build). Danach zeichnet der Abgleich die
übernommenen und alle abgeglichenen Einheiten auf und nimmt die Member aus der Liste der als neuer
behaltenen (`.g4/build/textproject.json`). Texte folgen den Fingerabdrücken unter `texts` (4.2):
`MemberNewer` ist aus DSPRE, `JsonNewer` aus Datei, `Conflict` ein Konflikt; Texte mit gleichem
Inhalt auf beiden Seiten sind abgeglichen, nicht übernommen. Scripts sind abgeglichen, wo rotoms
Compile-Zustand die Hashes von Quelle und Member nennt; ein Member, den rotom selbst schrieb, ist
keine Änderung von außen.

Was unaufgezeichnet bleibt (Konflikt, fehlerhaft, zurückgehalten, nicht übernommen), bleibt es
mit **allen** seinen Dateien: Eine abgeglichene Einheit, die eine davon teilt (`arm9.bin` gehört der
Header-Tabelle und als Blob sich selbst), zeichnet sie nicht auf, und ein geändertes Manifest oder
Register bleibt unaufgezeichnet, solange eine Entität offen ist. Eine Einheit, deren Dateien seit
dem Scan einen anderen `stat` haben, übernimmt der Abgleich nicht und zeichnet sie nicht auf; der
nächste Scan urteilt neu; ebenso eine Datei, deren `stat` im racy Fenster des Scans liegt und deren
Inhalt nicht mehr der Hash ist, den der Scan nahm (eine neue oder in der Größe geänderte Datei mit
racy `stat` hasht der Scan dafür gleich). Eine abgeglichene Datei, die seit dem Scan geschrieben
wurde, verliert ihren Eintrag, sodass der nächste Scan sie als neu beurteilt. Eine Übernahme, die
nichts aufzeichnet, legt keine leere Aufzeichnung an. Eine Script-Quelle gilt als geändert, solange ihr Hash nicht der ist, den
rotoms Compile-Zustand nennt (noch nicht übersetzt): Ändert DSPRE inzwischen den Member, ist das ein
Konflikt, kein Dekompilieren über die Bearbeitung; eine geänderte Quelle allein wird aufgezeichnet,
aber nicht als übernommen gemeldet. Ein Register oder Manifest, das sich nicht lesen lässt, ist
eine fehlerhafte Einheit mit Datei, Zeile und Spalte; die Entitäten warten, bis es lesbar ist.

Nach jedem Scan, auch einem leeren, ruft die App `apply_one_sided`: Das rückt die Scan-Zeit der
Aufzeichnung vor, sodass kurz vorher aufgezeichnete (racy) Einträge danach allein über den `stat`
verglichen werden. Der Fortschritt zählt die Dateien und danach die Einheiten.

Scan und Übernahme eines Projekts laufen nacheinander (eine Sperre je Projektwurzel im Prozess).

**Ungespeicherte Werte.** Mit dem Store (`sync_store`) wird eine Einheit mit einem ungespeicherten
Asset nie übernommen und das Asset nie neu geladen: Jede Änderung von außen daran ist ein Konflikt
mit `unsaved`. Übernommene Einheiten laden ihre geladenen, sauberen Assets neu (`reloaded`).

**Konflikte und Wahl.** Konflikt ist eine Einheit, die auf beiden Seiten geändert ist, oder eine
einseitig geänderte mit ungespeichertem Asset. `other` ist `files`, wenn nur die Quelle von außen
geändert ist (mit ungespeichertem Asset), sonst `dspre`. `conflict_diff` zeigt beide Seiten: die
Quelle auf der Platte gegen die Member, in den Speicher exportiert (nie geschrieben); JSON nach
Feldpfad (`data.power`, `messages[2].text`, bei Einheiten mit mehreren Dateien mit dem Dateinamen
davor: `0003.json · data.music_day_id`) mit beiden Werten als kompaktes JSON (`fehlt` für eine
fehlende Seite), Scripts zeilenweise (`Zeile 12`, drei Zeilen Umgebung), Binärmember, Beifiles und
Blobs nach `Größe` und `xxh3`; höchstens 40 Zeilen, der Rest als Zahl (`more`). Die Wahl
(`resolve`, ohne Store `resolve_files`) prüft mit einem Scan, dass die Einheit noch Konflikt ist
(sonst „Für diese Einheit gibt es keinen Konflikt mehr.“), und zeichnet die Einheit danach auf:

| Konflikt | Studio-Fassung behalten | andere Fassung übernehmen |
|---|---|---|
| beide Seiten, oder DSPRE mit ungespeichertem Asset (`other = dspre`) | Member aus der Quelle; Ungespeichertes bleibt ungespeichert und unberührt (samt Undo; änderte sich die Quelle auch, wird sein Speichern mit `Stale` abgelehnt); saubere geladene Assets, deren Quelle sich änderte, werden neu geladen | Quelle aus den Membern; alle geladenen Assets werden neu geladen, Ungespeichertes fällt weg |
| Quelle mit ungespeichertem Asset (`other = files`) | Member aus der geänderten Quelle, die der neue Stand des Stores wird (`accept_disk_as_base`): Das nächste Speichern überschreibt sie ohne `Stale` | Member aus der geänderten Quelle; die Assets werden neu geladen, Ungespeichertes fällt weg |

Ein Text folgt `resolve_conflict` (`Json` bzw. `Member`), ein Script behält seine Quelle (sie wird
in rotoms Compile-Zustand veraltet gesetzt, der nächste Build übersetzt sie über DSPRE's Member)
bzw. wird aus dem Member dekompiliert, ein Blob wird kopiert. Lässt sich die Einheit nicht
übernehmen, nennt die Ablehnung den Grund (rotom dekompiliert den Member nicht, die Quelle lässt
sich nicht importieren, ihre Dateien änderten sich seit dem Scan). Bei Scripts zeigt der Diff
`fehlt` für eine Zeile, die nur eine Seite hat, und Größe und xxh3, wenn nur Zeilenenden abweichen.

## Einrichten und Zurücknehmen

**Einrichten** (`textproject::adopt::adopt`, `g4k text adopt`, in der App „Als Textprojekt
einrichten…“ und das Häkchen im Assistenten) macht aus einem DSPRE-Ordner ein Textprojekt.
Voraussetzungen wie beim Export: Platin oder HGSS (nicht Diamant/Perl), eine Basismarke, die
gefundene Basis-ROM dieser Marke (`--base-rom`, sonst die `.nds` daneben, sonst
`G4_BASE_ROM_DIR`; fehlt sie, sagt es `base::missing_text`), kein Manifest einer älteren oder
neueren Schema-Version, keine Texte im Konflikt, keine fehlenden Item-Member. Eine Ablehnung kommt,
bevor etwas geschrieben wird, mit dem Satz des Exports. Ebenso vorher abgelehnt: ein `rotom.toml`,
das sich nicht lesen lässt oder dessen `include_roots` weder Liste noch Pfad ist. Auf einem
Textprojekt mit Aufzeichnung prüft ein Scan zuerst, dass alles abgeglichen ist; eine Änderung von
außen (etwa eine von Hand bearbeitete JSON), ein Konflikt oder eine fehlerhafte Datei lehnt mit
dem Satz der Rücknahme ab, denn der Export schriebe die JSON aus den Membern neu. Die App verlangt
dazu gespeicherten Stand.
Der Fortschritt kommt in vier Phasen (`AdoptProgress`, je von `done` 0 bis `done` = `total`):

| Phase | Was geschieht | `total` |
|---|---|---|
| `scripts` | Scripts übernehmen (`scripts::adopt`), wenn sie es nicht sind | Script-Member; 0, wenn schon übernommen |
| `base_layer` | Basisschicht unter `.g4/cache/base-layer/<sha1>/`, nur wenn sie fehlt | 1 |
| `entities` | der Export: Text-JSON, Taufe, je Art ihr Ordner unter `data/`, Register, Quirks, dann die Blobs | 17 (16 Arten, eine auch, wenn das Spiel sie nicht hat, und die Blobs) |
| `sync` | `rotom.toml` bekommt `.g4/include` in `include_roots`, die Include-Datei, dann `sync` und `texts` in `.g4/build/state.json` voll aufgezeichnet | die beobachteten Dateien (gemeldet alle 128) |

`rotom.toml` wird nur ergänzt: fehlt `include_roots`, kommt der Schlüssel dazu; steht dort eine
andere Liste (oder ein einzelner Pfad), wird `.g4/include` angehängt; Kommentare und Reihenfolge
bleiben (`toml_edit`). Für DSPRE-Projekte wertet rotom die Include-Wurzeln beim Übersetzen nicht
aus; ohne Datei ist der Eintrag harmlos. Ein zweites Einrichten auf einem Textprojekt ist ein
Export, der nichts schreibt, und eine neue Aufzeichnung. Danach baut ein unverändertes Projekt
SHA-1-gleich zur Basis-ROM. Der Assistent richtet mit dem Häkchen nach der Phase `check` (die
Basismarke steht) und vor dem Umbenennen des Staging-Ordners ein, mit der gewählten ROM als Basis
(`start::create_project_with_options`, Phase `text_project`; ihr Fortschritt ist ein Zähler bis
1000 über die vier Phasen, gewichtet 5 / 5 / 50 / 40 %, der nie zurückgeht).

**Zurücknehmen** (`textproject::adopt::revert`, `g4k text revert`, in der App „Zurücknehmen…“)
verschiebt das Manifest, statt einen Schnappschuss zurückzuspielen. Das ist sofort und verlustfrei
möglich, weil der DSPRE-Teil jederzeit vollständig ist: Der Export ändert keinen Member und
überschreibt nur Text-JSON, die nach 4.2 ohnehin dem geltenden Member gleichen; danach halten
Speichern und Abgleich die Member stets aktuell. Ein Schnappschuss vor dem Einrichten kostete rund
150 MB und wäre nach der ersten Bearbeitung veraltet. Ablauf:

1. Nur ein Textprojekt der aktuellen Schema-Version (sonst der Satz von `manifest::refusal`).
2. Ein Scan des Abgleichs; Abgeglichenes wird aufgezeichnet, nichts übernommen. Findet er eine
   Änderung von außen (`dspre`, `files`), einen Konflikt oder eine fehlerhafte Datei, wird
   abgelehnt: „Erst abgleichen: es gibt Änderungen von außen, Konflikte oder fehlerhafte Dateien
   (Technik › Textprojekt).“ Ungespeicherte Werte prüft die App vorher.
3. `g4-world.json` und `g4-checks.json` werden mit den IDs des Textprojekts gelesen und mit
   Nummern geschrieben (ohne Register gelten dort nur Nummern sicher); eine Datei, die schon nur
   Nummern trägt, bleibt unberührt, `rewritten` nennt die geschriebenen. Ist eine der beiden nicht
   lesbar, wird abgelehnt, bevor etwas geschrieben oder verschoben ist: „g4-world.json ist nicht
   lesbar (<Grund>). Erst reparieren, dann zurücknehmen.“
4. `g4-project.json` (zuerst: ohne Manifest ist der Ordner sofort wieder ein DSPRE-Ordner),
   `data/` und `.g4/include/` werden per `rename` in einen neuen Ordner
   `.g4/backup/textproject-<UTC-Zeit>/` verschoben (RFC 3339 ohne Doppelpunkte wie bei der
   Migration, `-2`, `-3` …, wenn es ihn gibt), mit denselben Pfaden darin.
5. `sync` in `.g4/build/state.json` wird geleert.

Im Backup liegen also das Manifest, alle Entitäts-JSON mit Beifiles, die Register `data/ids/`,
die Blobs `data/raw/`, die Quirks und die Include-Datei. Die Text-JSON unter
`expanded/textArchives/`, die Script-Quellen, `rotom.toml` (mit der Include-Wurzel) und die
Basisschicht bleiben, wo sie sind; sie gehören auch zum DSPRE-Ordner. Wer die Rücknahme
zurücknehmen will, verschiebt die drei Einträge zurück und gleicht ab. Ein Build danach ist der
eines DSPRE-Ordners und liefert unverändert die Basis-ROM.

## Projektdateien neben dem Textprojekt

`g4-world.json` (Weltbaum: Namen der Weltkarten und Räume, Räume von Hand einem Gebiet zugeordnet)
und `g4-checks.json` (ausgeblendete Befunde, abgeschaltete Prüfungen) liegen in der Projektwurzel,
in DSPRE-Ordnern wie in Textprojekten. Seit 4.3b nennen sie Header und Trainer durch **ID oder
Nummer**; gelesen wird über den `Resolver` des Projekts (`textproject::ids`), geschrieben im
Textprojekt der aktuellen Schema-Version die ID (wo die Nummer eine hat), sonst die Nummer. Im
Speicher, in Tauri und MCP bleiben es Nummern. Eine Map-ID gilt auch ohne Textprojekt als
`MAP_<interner Name>` aus `mapname.bin`; eine Trainer-ID braucht das Register.

**`g4-world.json`** (`world_meta::{load, save}`, `version` bleibt 1): die Schlüssel von `roomNames`
und `roomOwner` sind Header, ebenso ein Wert von `roomOwner`, der eine Zahl ist oder mit `MAP_`
beginnt; andere Werte (Gebietsschlüssel wie `w0:3`, `m240`) bleiben, wie sie sind. Geschrieben wird
in Header-Reihenfolge. Die Matrizen (`worldMaps`, `notWorldMaps`) bleiben Zahlen.

```json
{
  "version": 1,
  "worldMaps": [],
  "notWorldMaps": [],
  "roomNames": { "MAP_T01R0101": "Haus von Lucas", "600": "Keller" },
  "roomOwner": { "MAP_T01R0101": "MAP_T01", "600": "w0:3" }
}
```

Eine unbekannte ID, eine Zahl über 65535 oder ein Header, der zweimal steht, macht die Datei
„nicht lesbar“ mit Grund (`unbekannte ID MAP_X in roomNames`, `70000 in roomNames ist keine
Header-Nummer`, `Header 0 steht zweimal in roomNames`): der Weltbaum öffnet leer, und die Datei
wird nicht überschrieben, bis sie wieder lesbar ist (Regel R15).

**`g4-checks.json`** (`checks::IgnoreSet::{load, save}`) hat `version: 1` (`IGNORE_VERSION`, immer
geschrieben; fehlt das Feld, gilt 1; eine höhere Version wird abgelehnt: „g4-checks.json: Version 2
stammt aus einer neueren Fassung von Gen4 Studio.“). `subject.index` ist bei `header` und `trainer`
eine ID oder Nummer, bei allen übrigen Arten eine Nummer (sonst „eventFile nimmt nur eine Nummer,
nicht …“). Die Listen sind nach Code, Subjekt (Nummer), Schlüssel und Fingerabdruck sortiert.

```json
{
  "version": 1,
  "ignored": [
    { "code": "header.text-split", "subject": { "kind": "header", "index": "MAP_T01" },
      "key": "text_archive", "fingerprint": "0c79e202f9ccd0cf" },
    { "code": "trainer.no-party", "subject": { "kind": "trainer", "index": "TRAINER_YOUNGSTER_KAI" },
      "key": "party", "fingerprint": "00000000000000ff" }
  ],
  "disabled": ["encounter.empty-slot"]
}
```

Eine Datei, die nicht lesbar ist (auch eine unbekannte ID), blendet nichts aus, und das Studio
schreibt sie nicht, bis sie wieder lesbar ist. „Zurücknehmen“ schreibt beide mit Nummern (siehe
oben).

## Schema-Version und Migration

`schemaVersion` zählt die Dateiformate unter `data/` und das Manifest. Jede Namensänderung eines
Schlüssels oder einer Datei erhöht sie und fügt einen Schritt zu `migrate::MIGRATIONS` hinzu; der Test
`schema_fields_match_the_snapshot` vergleicht alle Schlüsselpfade mit `schema.snap` und schlägt fehl,
wenn sich das Schema ohne neue Version ändert. `g4k text migrate` hebt ein älteres Projekt an: alles
im Speicher, dann ein Schnappschuss der berührten Dateien nach
`.g4/backup/textproject-v<von>-<Zeit>/`, dann schreiben. Export, Import, Check und der Build lehnen
eine ältere Version ab („…: `g4k text migrate` ausführen.“), eine neuere immer.

## Codes

Jede Meldung ist `<code>: <Satz>`, bei einer Entität `<code>: <JSON-Pfad>: <Satz>`. Ein Fehler hält Import
und Build auf; Warnungen und Infos werden gemeldet, halten aber nichts auf.

| Code | Schwere | Bedeutung |
|---|---|---|
| `textproject.schema` | Fehler | Datei passt nicht zum Schema: kein JSON, unbekannter oder vertippter Schlüssel, falscher Typ, Wert außerhalb des Bereichs, Lücke in der Nummerierung |
| `textproject.unknown-id` | Fehler | eine ID, die keine Tabelle und kein Register kennt |
| `textproject.id-mismatch` | Fehler | `id` ist nicht die ID der Nummer der Datei |
| `textproject.side-file` | Fehler | ein Beifile `{"file"}` fehlt oder ist nicht lesbar |
| `textproject.base-missing` | Fehler | ein `base_sha1`-Teil, den weder der Member noch eine Basisschicht hat, oder eine Basisschicht, die sich nicht lesen oder anlegen lässt |
| `textproject.base-mismatch` | Fehler | die Basisschicht hat für den Teil andere Bytes |
| `textproject.map-source` | Fehler | eine Karte mit anderer `source` als `binary` |
| `textproject.header-count` | Fehler | eine Header-Zahl, die das Projekt nicht aufnehmen kann, oder Nummern doppelt bzw. mit Lücke |
| `textproject.member-without-json` | Fehler | ein Member (oder Header) ohne JSON; der Import löscht nicht |
| `textproject.header-table` | Fehler | die Header-Tabelle ist nicht lesbar oder nicht zu finden |
| `textproject.quirk-key` | Fehler | ein Schlüssel der Quirks-Datei ist keine vorhandene Quelle unter `expanded/scripts` |
| `textproject.raw-path` | Fehler | ein Blob außerhalb des Bereichs der Blobs oder an einem Pfad einer Entität |
| `textproject.unknown-art` | Warnung | ein Ordner unter `data/`, den keine Art kennt; bleibt unberührt |
| `textproject.reserved` | Info | `schemas` oder `plugins` im Manifest sind nicht leer |
| `textproject.id-dangling` | Warnung | ein Registereintrag nennt eine Nummer, die es nicht mehr gibt; er bleibt |
| `textproject.base-unchecked` | Warnung | Binärmember ohne Datei ließen sich ohne Basisschicht nicht vergleichen |
| `textproject.member-newer` | Warnung | Build: ein Member ist seit dem letzten Build geändert, seine JSON nicht; der Member bleibt (`g4k text export` übernimmt ihn, `g4k text import` verwirft ihn) |
| `textproject.conflict` | Fehler | Build: JSON und Member einer Einheit sind beide seit dem letzten Build geändert; nichts geschrieben |
| `textproject.unsynced` | Fehler | Build ohne Eintrag (erster Build, „ohne Cache“, neues Salz): ein Member weicht von seiner JSON und von der Basisschicht ab; nichts geschrieben |

## Quellenliste

`textproject::sources` (`g4k text sources`) listet die Dateien des Textprojekts, relativ zur Wurzel,
mit `/`, sortiert, je wenn vorhanden: `g4-project.json`, alles unter `data/`,
`expanded/textArchives/*.json`, `expanded/scripts/**`, `rotom.toml`, `.rotom/command_database/**`,
`g4-world.json`, `g4-checks.json`, `.g4/base.json`, `.g4/preview.png`; versteckte Einträge in den
Ordnern nicht. Alles andere entsteht aus diesen Dateien und der Basis-ROM.
