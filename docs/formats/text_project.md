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
| `data/<art>/NNNN.json` | eine Entität je Datei, `NNNN` vierstellig ab `0000`, lückenlos; eine gelöschte trägt den Umschlag `"deleted": true` (siehe Arten) |
| `data/<art>/NNNN.<ext>` | Beifiles einer Entität (gleicher Stamm) und die Member der Binärarten |
| `data/ids/{maps,trainers,trainer_classes,places,flags,vars}.json` | Register der getauften IDs und der eigenen Flag- und Variablennamen |
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
  "schemaVersion": 2,
  "game": {
    "code": "CPUE",
    "family": "pt",
    "language": "en_US"
  },
  "studioVersion": "0.1.0",
  "schemas": {
    "flags": 1,
    "vars": 1
  },
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
| `schemas` | die Schema-Version jeder Datenart ohne ROM-Gegenstück (`flags`, `vars`, siehe [flags_vars.md](flags_vars.md) „Art-Beschreibung“), vom Export geschrieben; andere Schlüssel sind reserviert und bleiben unverändert (`textproject.reserved`) |
| `plugins` | reserviert; der Inhalt bleibt unverändert, gelesen wird er nicht (`textproject.reserved`) |

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
| `flags` | eigener Name eines Flags (`NNNN.json`, dezimal) | keiner: Datenart ohne ROM-Gegenstück, `{id, description?}` ([flags_vars.md](flags_vars.md)) |
| `vars` | eigener Name einer Variable (`NNNN.json`, dezimal, `16492.json`) | keiner, wie `flags` |

`flags` und `vars` sind Datenarten (`textproject/kinds.rs`): Export und Import fassen sie nicht an,
nur der Check liest sie, und es gibt eine Datei nur für eine Nummer mit eigenem Namen. Eine Art,
deren Archiv das Spiel nicht hat, fehlt. Items gibt es nur mit geprüfter Item-Tabelle
(`arm9_tables.yaml`); ohne sie laufen die Item-Member wie alles Nicht-Abgebildete (Blob bei
Abweichung). Teilen sich IDs einen Member, trägt die erste die Daten, jede weitere
`{"id": …, "same_member_as": "<ID>"}`; ein Member, auf den keine ID zeigt, ist ein Blob, wo er
abweicht.

**Grabstein.** Jede Entitätsdatei unter `data/` (jede Art mit JSON, auch `headers` und gesperrte
Member `{locked, raw}`) kann den Umschlag `"deleted": true` tragen: ein oberster Schlüssel, der zu
keinem Typ gehört, immer als **letzter** Schlüssel des Objekts und nur bei `true` geschrieben
(`textproject/tombstone.rs`). Inhalt und Member bleiben, wie sie sind: der Import liest die Datei
ohne den Umschlag und schreibt ihre Member wie sonst, der Export übernimmt den Umschlag der
vorhandenen Datei (ein Grabstein überlebt den vollen Export, der zweite Export schreibt nichts).
Listen blenden gelöschte Entitäten aus, Prüfungen werten sie als fehlend. Ein `deleted`, das kein
Wahrheitswert ist, ist `textproject.schema` („`deleted` ist kein Wahrheitswert.“); jeder andere
unbekannte oberste Schlüssel bleibt ein Schemafehler.

```json
{
  "id": "TRAINER_0012",
  "data": { "…": "…" },
  "deleted": true
}
```

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

In einer Ereignisdatei nennt `object_events[].hidden_flag` das Flag und `coord_events[].var` die
Variable mit dem **eigenen** Namen aus dem Register (`data/ids/flags.json`, `vars.json`), sonst als
Zahl; 0 bleibt 0, pret-Namen schreibt der Export nie (ein Projekt ohne eigene Namen behält seine
Zahlen). Der Import nimmt den eigenen Namen, den pret-Namen der Familie (`FLAG_UNUSED_0x0094`) und
die Zahl bis `0xFFFF`. `coord_events[].value` ist immer eine Zahl, ebenso die Werte der Level-Scripts.
`warp_events[].dest_header_id` nennt den Header (`MAP_*`).

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

Welche Nummern der Welt auf welche Entität zeigen, steht deklarativ in
`crates/g4-kernel/metadata/entity_refs.yaml` (Quellart → Feld → Zielart); der Referenzindex
(`data/refs.rs`, Leser in `data/refs/entity.rs`) liest danach, alle Verweise lesend:

- `headers` (Slot = Header-ID): `matrix_id` → Matrix, `event_file_id` → Eventdatei,
  `script_file_id` und `level_script_id` → Scriptdatei, `text_archive_id` → Textarchiv,
  `area_data_id` → Gebietsdaten, `location_name` → Ort, `wild_pokemon` → Begegnungsdatei (nicht
  bei `0xFFFF` bzw. HeartGold/SoulSilver `0xFF`)
- `matrices` (Slot = Zelle): `maps` → Karte (nicht bei `0xFFFF`), `headers` → Header
- `events` (Slot = Warp): `warp_events.dest_header_id` → Header, `warp_events.dest_warp_id` →
  Warp `dest_warp_id` des Headers `dest_header_id` (die Tür als Paar)
- `area_data`: `map_tileset` → Kartentextur, `building_tileset` → Gebäudetextur
- `maps` (Slot = Gebäude): `buildings.model_id` → Gebäudemodell; auf HeartGold/SoulSilver zeigt
  eine Karte, die innen liegt (die Gebietsdaten eines Headers, der sie zeigt, haben `area_type`
  0; der Header ist der der Matrixzelle, ohne Header-Schicht jeder Header der Matrix), mit
  `interior_buildings.model_id` auf das Innenraum-Gebäudemodell

Ein Test prüft die Tabelle gegen die Leser (`entity_refs_name_fields_every_reader_yields`).

## Symboltabellen und Register

Spezies (mit Formen), Attacken, Items, Fähigkeiten und Typen haben feste englische Tabellen in
`crates/g4-kernel/metadata/symbols/` (aus [PokeAPI](https://github.com/PokeAPI/pokeapi), BSD
3-Clause, Lizenz in `LICENSE-PokeAPI.txt`). Eine Nummer ohne Eintrag heißt `PREFIX_NNNN` (vier
Stellen, auch über der Tabelle).

Maps, Trainer, Trainerklassen und Orte haben Register `data/ids/<art>.json`, dazu die eigenen
Namen der Flags und Variablen (`flags.json`, `vars.json`): sechs Register, je ein JSON-Objekt
ID → Nummer, Schlüssel sortiert, ohne eigenes Versionsfeld. Der Export schreibt alle sechs (leer
als `{}`). Flags und Variablen werden nie getauft: ihr Register hält nur Namen, die „Als eigenes
Flag benennen…“ vergeben hat, die pret-Namen stehen nicht darin
([flags_vars.md](flags_vars.md)).

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
verschwindet (`textproject.id-dangling`: Maps und Trainer über ihrer höchsten Datei unter
`data/`, eine Lücke darunter ist kein verschwundener Eintrag; Flags und Variablen nie, ihnen fehlt
höchstens die Entitätsdatei). Ein Register darf keine ID doppelt nennen, keine Nummer
doppelt vergeben und keine fremde Fallback-ID tragen (`textproject.schema` an der Stelle der ID).

**Eine Entität von Hand anhängen.** Eine neue Entität hat eine Nummer, die kein Member und kein
Registereintrag kennt; ihre JSON (`data/<art>/NNNN.json`, die Nummer gleich der Länge der Archive,
lückenlos) trägt deshalb die **Fallback-ID** `PREFIX_NNNN` (`MOVE_0004`, `TRAINER_0054`,
`MAP_0161`; vier Stellen), denn `id` muss die ID der Nummer sein (`textproject.id-mismatch`), und
eine andere gibt es noch nicht. Der Import legt den Member an und trägt die Fallback-ID nicht ins
Register ein (ein Register darf keine fremde Fallback-ID tragen). Getauft wird erst beim nächsten
Export oder beim Speichern im Studio: dann bekommt die Entität ihre ID nach den Regeln oben
(Trainer aus Klasse und Name, Header aus dem internen Namen), das Register wird geschrieben und
die JSON trägt die neue ID. Arten mit fester Tabelle (Spezies, Attacken, Items) behalten die
Fallback-ID. Eine ID von Hand zu wählen geht nur über „ID umbenennen“.

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
von `arm9/arm9.bin` behält dabei die Header-Tabelle der Datei (sie gehört den Header-Dateien), und er
weicht nur ab, wo er außerhalb der Tabelle anders ist (fehlt die Datei, kommt der Blob mit seiner
eigenen Tabelle, die die Header-Einheit im selben Vorgang schreibt). Ein Blob außerhalb dieses
Bereichs, an einem Pfad einer Entität oder als NARC einer Entitätsart ist `textproject.raw-path`.
Ein vanilla DSPRE-Ordner hat keinen Blob.

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
Art ist `textproject.member-without-json`: der Import löscht nur Member am Ende ohne JSON, die seit
der Aufzeichnung unverändert sind (`textproject.member-removed`). Die **Kürz-Regel** (3d.4): Fehlen
alle Member ab der letzten JSON einer Art in `data/` (am Ende, keine Lücke), gleicht der
Fingerabdruck jeder ihrer Dateien dem Abgleich (`sync.files` in `.g4/build/state.json`, seit der
Aufzeichnung unverändert; ein Member ohne Eintrag gilt als geändert) und hat die Basis(schicht)
keines von ihnen, plant der Import sie als Entfernung (`removals`, im Check unter `differs`) und
entfernt sie (Warnung `textproject.member-removed`); so baut ein zweiter Git-Branch, auf dem
„Nummer freigeben“ die JSON löschte, nach `git switch` (auch der Build-Schritt `textproject`
entfernt sie). In der Mitte, seit der Aufzeichnung geändert oder in der Basis bleibt es
`member-without-json`. Dieselbe Regel gilt für Header am Ende (Patch „Dynamische Header“): fehlen
die JSON ab einer Nummer, sind ihre Dateien `unpacked/dynamicHeaders/NNNN` und `mapname.bin` seit
der Aufzeichnung unverändert und hat die Basis die Header nicht, schreibt der Import die Tabelle
kürzer (`mapname.bin` verliert ihre Einträge, ihre Dateien gehen). Texturen und Modelle (Binärarten
ohne JSON) deckt die Regel in 3d.4 nicht ab: ein Member ohne Datei und ohne Basis bleibt
`member-without-json`. Neue Entitäten werden angehängt, nur lückenlos. Bei einer Art aus zwei Archiven (Trainer: Properties und Party) gehört auch ein
Member, das nur eins der beiden hat (DSPRE ließ ein Archiv länger), dazu: Ab der ersten Nummer, die
weder eine JSON deckt noch beide Archive haben, meldet der Import `member-without-json`; der Export
schreibt keine JSON dafür und warnt mit demselben Code. Binärarten ohne Datei kommen aus der Basisschicht; ohne Schicht und ohne Basis-ROM
bleiben sie, wie sie sind (`textproject.base-unchecked`, eine Warnung). Lässt sich die Basisschicht nicht
lesen oder aus der gegebenen Basis-ROM nicht anlegen, ist das `textproject.base-missing` (ein Fehler).

**Wenn die andere Seite neuer ist** (seit 4.3a I1): Export und Import verwerfen sonst Arbeit. Der
Export schreibt die JSON aus den Membern (und löscht Beifiles und Blobs, die er nicht schreibt), der
Import schreibt die Member aus der JSON. Gibt es `.g4/build/textproject.json` (nach dem ersten
Export, Import oder Build) und ein Manifest der aktuellen Version, fragen beide vorher den
Build-Schritt `textproject` ohne zu schreiben (sein Urteil wie beim nächsten Build, siehe unten),
was sich seit seinem letzten Stand auf welcher Seite geändert hat:

- JSON-Seite: Einheiten, die der Schritt in die Member schreiben würde (JSON, Beifile oder Blob
  geändert; ohne Eintrag des Schritts: Member fehlt oder gleicht der Basisschicht), die Dateien der
  Fehler seiner Planung (eine kaputte oder gelöschte JSON, ein Blob am falschen Ort; nicht
  `base-missing`, `base-unchecked`) und `data/rotom-quirks.json`, wenn sie den Compile-Zustand
  ändern würde;
- Member-Seite: die Member, die er als neuer behält (`member-newer`);
- beide Seiten: `conflict`; `unsynced` (voll, Member weicht von der Basis ab) zählt für keine Seite.

Der Abgleich von 4.3b schärft das Urteil, ersetzt es aber nicht. **Was sein Stand garantiert:**
`sync` in `.g4/build/state.json` hält je Datei einen Fingerabdruck mit dem Hash von Bytes, die ein
Schreiber selbst geschrieben oder gelesen hat, und nur, solange die Datei sie noch hält:

- Speichern, Build-Schritt, Übernahme, Wahl, „ID umbenennen“, Export, Import und Einrichten
  zeichnen eine Datei, die sie schreiben, nur mit dem Hash der geschriebenen Bytes auf; hält sie
  danach andere (DSPRE oder ein Editor schrieb dazwischen), verliert sie ihren Eintrag;
- Export, Import und Einrichten zeichnen eine Datei, die sie nur lesen (die Member beim Export,
  die JSON beim Import), nur mit dem Hash der gelesenen Bytes auf, und nur, solange sie die noch
  hält; mit zwei verschiedenen Inhalten gelesen, oder von einem Schreiber darin aufgegeben (der
  Build-Schritt im Import), verliert sie ihren Eintrag; eine Datei, die schon die Bytes hält, die
  sie schreiben würden, gilt als gelesen; Skriptquellen und ihre Member gelten als gelesen, wenn
  rotoms Compile-Zustand (aus der Dekompilierung beim Einrichten, dem Compile beim Import) ihre
  Hashes nennt und nicht `Dirty` ist; eine Datei, die sie weder lesen noch schreiben, behält
  ihren alten Eintrag (sie wird nie mit ihrem jetzigen Inhalt neu aufgezeichnet; ohne alten
  Eintrag, etwa beim Import ohne `sync`, bleibt sie ohne, und der nächste Abgleich prüft sie am
  Inhalt). Ebenso zeichnet der Export den Eintrag des Build-Schritts neu auf: eine Eingabe nur
  mit den gelesenen oder geschriebenen Bytes, sonst fehlt sie (der nächste Lauf findet sie
  geändert);
- Speichern, Build-Schritt und „ID umbenennen“ bürgen nur für ihre Schreibvorgänge: eine Datei,
  die sie nur lesen (etwa das Member einer Einheit, die der Schritt stehen lässt), zeichnen sie
  dadurch nicht auf;
- die Quellen (JSON, Beifiles) einer Einheit, die Speichern oder Build-Schritt nicht selbst
  schreiben, zeichnen sie nur auf, wenn die Einheit danach nachweislich gleich ist (aus der JSON
  erzeugte Bytes gleich den Membern, mit den dafür gelesenen Bytes); sonst verlieren sie ihren
  Eintrag (etwa ein von Hand geänderter, nicht gebauter Teil, den das Speichern eines anderen
  Teils aus der Datei übernahm, oder eine JSON, die sich während des Builds änderte);
- eine Übernahme zeichnet die Seite, von der sie übernimmt, nur so auf, wie der Abgleich sie las;
- kein Eintrag gilt als geändert: der nächste Abgleich vergleicht dann den Inhalt und zeichnet
  eine gleiche Einheit wieder auf.

Halten alle Dateien einer Seite einer abweichenden Einheit noch, was er aufzeichnete, ist dieser
Stand nicht älter als der, den der Schritt zuletzt gelesen hat (`ctime` im Eintrag des Schritts),
und halten Manifest und Register ebenfalls ihren Stand, hat sich nur die andere Seite geändert:

- beide Seiten seit dem letzten Lauf geändert (Speichern oder Übernahme, dann eine Änderung auf
  einer Seite): zählt für die geänderte Seite allein; so urteilt auch der Build selbst (Member
  schreiben bzw. `member-newer`, kein `conflict`), auch kopflos mit `g4k build`;
- vom Schritt zu schreiben, aber nur die Member geändert (etwa von DSPRE auf die Basis
  zurückgesetzt): Member-Seite; ein gelöschter Member zählt für keine Seite (der Import holt ihn
  nur zurück);
- als neuer behalten, aber nur die JSON geändert: JSON-Seite;
- `unsynced` (voll): die geänderte Seite.

Fehlt dieser Nachweis in einem vollen Lauf (kein Stand, ein älterer Stand als der des Schritts,
beide Seiten geändert, Manifest oder Register geändert), zählt eine Einheit, die der Schritt
schreiben würde, für **beide** Seiten, und ein `unsynced` ebenso: der Import lehnt ab, statt einen
Member zu überschreiben, den DSPRE geändert haben kann. Im inkrementellen Lauf urteilt dann der
Schritt allein. Speichern und Übernahmen schreiben JSON und Member gleich, ihre Einheiten weichen
also gar nicht ab.

**Wofür `--force` ist:** wenn die Ablehnung eine Seite schützt, die verworfen werden soll (der
Export soll die Member übernehmen, der Import die JSON durchsetzen, etwa nach `git checkout -- data/`
über geänderte Member), oder wenn kein Stand zeigt, welche Seite neuer ist (ein Projekt ohne
`sync`, nach einem Wechsel des Manifests). `--force` prüft nichts davon und überschreibt die andere
Seite.

Der Export lehnt dann JSON-Seite und Konflikte ab (nach der Basisschicht, vor allem anderen, nichts
geschrieben): `Im Textprojekt sind Änderungen, die noch nicht in den Membern sind (<bis zu zehn,
dann …>); der Export würde sie verwerfen: erst bauen (g4k build) oder g4k text import, dann
exportieren.` (ein Konflikt als `<json> (auch <member> geändert)`). Der Import lehnt Member-Seite und
Konflikte ab, bevor er etwas liest: `Member sind neuer als ihre JSON (<…>); der Import würde sie
überschreiben: erst exportieren (g4k text export), dann importieren.` (ein Konflikt als
`<member> (auch <json> geändert)`). Mit `--force` (`ExportOptions { force }`, `ImportOptions { force }`)
gehen beide darüber hinweg und verwerfen die andere Seite. Ohne `.g4/build/textproject.json` (frischer
Klon) urteilt niemand, und beide laufen wie bisher. Die App fragt dasselbe vor ihrer Frage
„Textprojekt exportieren“ (Code `unsynced`).

Ein erfolgreicher Export ist ein Abgleichspunkt des Schritts: hat der Schritt einen Eintrag mit
seinem Salz, zeichnet der Export ihn aus den Eingaben neu auf, wie sie danach sind; sonst sähe die
neu geschriebene JSON beim nächsten Build (und beim nächsten Export) wie eine JSON-Änderung neben
jeder späteren Member-Änderung aus (ein falscher Konflikt). Ohne Eintrag bleibt der nächste Lauf
voll. Der Import braucht das nicht: sein `prepare` zeichnet den Schritt auf.

Ein Beifile, das keine JSON nennt, ändert keinen Member; der Schritt sieht es nicht, und der Export
löscht es weiter ohne Rückfrage.

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
| inkrementell | nein | ja | Member bleibt; Hinweis `textproject.member-newer: <member> ist neuer als <json>; g4k text export übernimmt ihn, g4k text import verwirft ihn.`; ein Member ganz ohne JSON: `textproject.member-newer: <member> hat noch keine <json>; g4k text export legt sie an, g4k text import lehnt das Member ab (der Import löscht nur Member am Ende, die seit der Aufzeichnung unverändert sind).` |
| inkrementell | ja | ja | Fehler `textproject.conflict: <json> und <member> sind beide geändert; nichts geschrieben.` (zeigt der Stand des Abgleichs eine Seite unverändert seit JSON und Member gleich waren, gilt nur die andere: siehe oben) |
| voll (kein Eintrag, anderes Salz) | – | – | Member fehlt oder gleicht dem Member der Basisschicht (`arm9.bin` für die Header-Tabelle nach der Tabelle, für seinen Blob nach allem außerhalb der Tabelle) → schreiben; sonst Fehler `textproject.unsynced: <member> weicht von <json> und von der Basis ab; g4k text import (JSON gilt) oder g4k text export (Member gilt).`; ein Member ganz ohne JSON: `textproject.unsynced: <member> hat keine <json> und fehlt in der Basis; g4k text export legt sie an (Member gilt), g4k text import lehnt das Member ab (der Import löscht nur Member am Ende, die seit der Aufzeichnung unverändert sind).` |

Voll läuft der Schritt auch, wenn `.g4/build/textproject.json` fehlt, nicht lesbar ist oder eine
andere Version hat (Grund `textproject.sync-lost`): dann ist unbekannt, welche Member er als neuer
behalten hat, also urteilt er über die Basisschicht. Ein Member ohne JSON (von Studio oder DSPRE
angehängt, oder ein Teil, der in der JSON fehlt) ist inkrementell `member-newer`, wenn sich sein
Member geändert hat oder schon behalten war, voll `unsynced`, sonst das
`textproject.member-without-json` des Imports. Die Sätze der Tabelle nennen für ein solches Member den
richtigen Ausweg, denn `g4k text import` lehnt ein angehängtes Member ohne JSON ab (er löscht nur
Member am Ende, die seit der Aufzeichnung unverändert sind; ein Member am Ende, das die Kürz-Regel
entfernt, ist kein solches, siehe „Export, Import, Check“). Das überzählige Member einer Art aus zwei Archiven (Trainer) ist davon ausgenommen: Der
Export legt dafür nichts an, der Schritt (inkrementell und voll) und der Abgleich melden den Satz des
Imports als Fehler, der DSPRE als Ausweg nennt: `die Archive haben verschieden viele Member (…); ab
NNNN hat ein Member kein Gegenstück. Weder Export noch Import legen ein Gegenstück an: das Member in
DSPRE ergänzen oder entfernen (es ist keine Entität, die sich im Studio löschen ließe).` Der Abgleich übernimmt es nicht als
Trainer aus DSPRE, die Einheit ist fehlerhaft.

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

**Grabstein: Zurücksetzen und Diff.** „Auf Original zurücksetzen“ nimmt den Grabstein (siehe
„Arten“) zurück: Der Plan (`original::plan_reset`) nennt bei einer gelöschten Karte, Eventdatei,
Begegnungsdatei, einem Trainer oder Header zuerst die Zeile `Grabstein`: `gelöscht` → `vorhanden`
und ist dann nie „schon wie in der Basis“, auch wenn die Member der Basis gleichen; `reset` setzt
die Member und nimmt den Grabstein in **derselben** Transaktion zurück
(`Auf Original zurückgesetzt: …`, ein Schritt). Eine neue Entität hat kein Original und bleibt
abgelehnt („Kein Original: …“); sie kommt mit ⌘Z oder „Wiederherstellen“ zurück. Die Liste
„geändert gegenüber Original“ (`changed_from_original`) liest die Umschläge der gespeicherten
Dateien: eine gelöschte Entität ist `deleted` („Gelöscht“), ob ihre Member der Basis gleichen
oder nicht, mit Rücksetzziel, wenn sie ein Original ist, ohne, wenn sie neu ist. Der Diff einer
Entität (`diff_entity`) liest den Umschlag je Seite (die Basis-ROM hat keinen, `HEAD` den seiner
Datei, die Platte den ihren, der ungespeicherte Stand das Flag des Stores) und meldet ihn als
`tombstone: {old, new}` (nur bei Arten mit Umschlag), nie als Feld `deleted`; eine Seite, die
nur ihren Umschlag änderte, ist `changed`. Die Liste gegen `HEAD` nennt eine Datei, deren
Umschlag sich änderte, wie jede geänderte Datei als `changed`.

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

`arm9.bin` gehört der Header-Tabelle und als Blob sich selbst (verglichen ohne die Tabelle). Für die
Header-Tabelle ist `arm9.bin` nur geändert, wo sich die Tabelle geändert hat: unverändert ist sie,
wenn der aufgezeichnete Hash der Datei der Hash der Bytes außerhalb der Tabelle laut Blob (ohne ihn:
laut Basisschicht) mit der Tabelle von heute ist; eine Änderung außerhalb der Tabelle allein ist also
keine Header-Änderung (eine Header-Datei und eine DSPRE-Änderung daneben sind aus Datei und aus DSPRE,
kein Konflikt). Lässt sich das nicht zeigen, gilt die Tabelle als geändert. Eine
Datei, die keine Einheit entscheidet (Register, Manifest, ein unbekannter Ordner unter `data/`,
eine ID-Datei für rotom), wird nur aufgezeichnet; eine Änderung am Manifest oder an einem
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
Abgleich löscht keine Entität, Löschen geht im Studio („Löschen…“, mit Referenzsperre). Ausgenommen
sind die Member am Ende, die die Kürz-Regel des Imports entfernt (dieselbe Regel, siehe „Export,
Import, Check“): Ihre Einheit ist aus Datei mit der Warnung `textproject.member-removed`, die
Übernahme entfernt sie wie der Import; ein Header am Ende ebenso (seine Datei unter
`unpacked/dynamicHeaders` und sein Eintrag in `mapname.bin`). Ein gelöschter Blob unter
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
Header-Tabelle und als Blob sich selbst), zeichnet sie nicht auf; übernimmt der Abgleich die teilende
Einheit, verliert der Eintrag der Datei seinen Hash, sodass sie für jede Einheit und jede Prüfung
als geändert gilt, bis sie wieder aufgezeichnet wird (geht die Datei zurück auf den alten Stand,
bleibt das sichtbar, statt dass der Build die Seite der übernommenen Einheit darüberschreibt). Ein
geändertes Manifest oder Register bleibt unaufgezeichnet, solange eine Entität offen ist. Eine
Einheit, deren Dateien seit
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
bevor etwas geschrieben wird, mit dem Satz des Exports. `rotom.toml` bleibt unberührt. Auf einem
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
| `sync` | die ID-Dateien für rotom (siehe „IDs in Scripts“), dann `sync` und `texts` in `.g4/build/state.json` voll aufgezeichnet | die beobachteten Dateien (gemeldet alle 128) |

`AdoptReport.include_written` sagt, ob die ID-Dateien geschrieben wurden (der Name stammt aus dem
Plan, in dem hier eine Include-Datei stand). Ein zweites Einrichten auf einem Textprojekt ist ein
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
   Nutzt eine Script-Quelle eine ID, die nur die ID-Dateien kennen (rotom ohne sie kennt den Namen
   nicht; ein `alias` der Datei zählt nicht), wird ebenfalls abgelehnt, denn ohne die Dateien
   übersetzte die Quelle nicht mehr: „Scripts nutzen IDs, die nur das Textprojekt kennt; ohne es
   ließen sie sich nicht mehr übersetzen. Erst durch Nummern ersetzen:
   expanded/scripts/0000.rotom:3 TRAINER_YOUNGSTER_KAI.“ (höchstens fünf Stellen, dann „und
   n weitere“).
3. `g4-world.json` und `g4-checks.json` werden mit den IDs des Textprojekts gelesen und mit
   Nummern geschrieben (ohne Register gelten dort nur Nummern sicher); eine Datei, die schon nur
   Nummern trägt, bleibt unberührt, `rewritten` nennt die geschriebenen. Ist eine der beiden nicht
   lesbar, wird abgelehnt, bevor etwas geschrieben oder verschoben ist: „g4-world.json ist nicht
   lesbar (<Grund>). Erst reparieren, dann zurücknehmen.“
4. `g4-project.json` (zuerst: ohne Manifest ist der Ordner sofort wieder ein DSPRE-Ordner),
   `data/` und die ID-Dateien `.rotom/command_database/.g4-ids-*.json` werden per `rename` in
   einen neuen Ordner
   `.g4/backup/textproject-<UTC-Zeit>/` verschoben (RFC 3339 ohne Doppelpunkte wie bei der
   Migration, `-2`, `-3` …, wenn es ihn gibt), mit denselben Pfaden darin.
5. `sync` in `.g4/build/state.json` wird geleert.

Im Backup liegen also das Manifest, alle Entitäts-JSON mit Beifiles, die Register `data/ids/`,
die Blobs `data/raw/`, die Quirks und die ID-Dateien. Die Text-JSON unter
`expanded/textArchives/`, die Script-Quellen, `rotom.toml` und die Basisschicht bleiben, wo sie
sind; sie gehören auch zum DSPRE-Ordner. Wer die Rücknahme zurücknehmen will, verschiebt die
Einträge zurück und gleicht ab (die ID-Dateien schreibt auch der nächste Compile neu). Ein Build danach ist der
eines DSPRE-Ordners und liefert unverändert die Basis-ROM.

## Nach einem Klon

Ein Textprojekt in Git versioniert nur seine Quellen (siehe „Quellenliste“) und die beiden
Vorlagen; der DSPRE-Teil (`header.yaml`, `arm7/`, `arm9/`, `arm9_overlays/`, `banner/`, `files/`,
`config.yaml`, `header_logo.png`, `multiboot_signature.yaml`, `path_order.txt`), die Member unter
`unpacked/` und alles unter `.g4/` außer `base.json` und `preview.png` stehen in der `.gitignore`
(Spieldateien aus der ROM gehören nicht ins Repository). Ein frischer Klon hat also
`g4-project.json`, aber kein `header.yaml`: `textproject::restore::needs_restore` ist dann wahr,
der Satz dazu `RESTORE_NEEDED` („Dieser Ordner ist ein Textprojekt ohne Spieldateien, etwa ein
frischer Git-Klon.“). Ohne die Spieldateien öffnet kein Projekt und kein Build.

**Vervollständigen** (`textproject::restore::restore_from_base`, `g4k text restore`, in der App
der Dialog „Textprojekt vervollständigen?“) legt sie aus der eigenen Basis-ROM an, die die
Basismarke `.g4/base.json` (im Repository) nennt. Vier Phasen (`RestoreProgress`, je von `done` 0
bis `done` = `total`):

| Phase | Was geschieht | `total` |
|---|---|---|
| `extract` | die Basisschicht (siehe „Basisschicht und Blobs“) unter `.g4/cache/base-layer/<sha1>/`, nur wenn sie fehlt: ds-rom packt die Basis-ROM aus, jedes Archiv ausgepackt. Der Import liest seine Basisteile daraus, die ROM wird also nur einmal ausgepackt | 1 |
| `merge` | jeder Eintrag der Basisschicht, den die Wurzel nicht hat, wird hineinkopiert (auf APFS ein Klon, kein zusätzlicher Platz); einen Ordner, den beide haben, führt die Regel darin weiter zusammen. Kopiert wird, was `build::extract_rom` und das Auspacken jedes Archivs (wie „Git einrichten“ im DSPRE-Ordner) hinterließen: nicht die Marke `.complete` der Schicht, nicht `unpacked/` der Dynamic Headers (das gehört dem Patch), nicht die Ordner der reinen Pack-Einträge. `header.yaml` kommt zuletzt | die Einträge oben in der Schicht, je Ordner unter `unpacked/` einer |
| `import` | `import` mit der Basis-ROM: die Member, die von ihrer JSON abweichen, die Scripts übersetzt, die Texte kodiert; am Ende zeichnet er auf, was er las und schrieb (`ImportReport.synced`) | 1 |
| `sync` | `sync::record_all`: jede beobachtete Datei als abgeglichen aufgezeichnet, keine bleibt ohne Eintrag | die beobachteten Dateien (gemeldet alle 128) |

Die Regel ist **nie überschreiben**: Eine Datei, die im Ordner liegt, bleibt, wie sie ist, auch
wenn die Schicht eine gleichnamige hat; steht eine Datei gegen einen Ordner (in beide Richtungen),
bleibt die des Projekts. Nur Fehlendes kommt hinzu, ein kopierter Ordner zählt in `moved` einmal.
Jede Datei wird in `.g4/tmp/restore-<pid>/` kopiert und dann an ihren Platz umbenannt, liegt also
ganz oder gar nicht da. Weil `header.yaml` zuletzt kommt, bleibt ein abgebrochener Lauf als
unvollständig erkennbar (`needs_restore` bleibt wahr); ein zweiter Lauf kopiert, was noch fehlt.
Zu Beginn werden Reste abgebrochener Läufe (`.g4/tmp/restore-*`, nur diese) entfernt, am Ende der
eigene Ordner, auch bei einem Fehler. Abgelehnt, bevor etwas geschrieben wird: ein Ordner mit
seinen Spieldateien („Das Projekt hat seine Spieldateien schon.“), ohne Basismarke
(`patch::NO_MARK`; eine unlesbare oder neuere Marke mit ihrem Satz) und eine andere ROM als die
der Marke (der Satz von `base::other_rom`, wie beim Import und bei `g4k patch create`). Scheitert
erst der Import, bleiben die Spieldateien liegen; `g4k text import` vervollständigt dann das
Projekt. Danach baut ein unverändertes Projekt SHA-1-gleich zur Basis-ROM, auch aus einem Klon mit
`core.autocrlf=true` (gated `tests/git.rs`, `text_project_in_git_builds_from_base_byte_equal`),
`git status` ist leer und der Abgleich findet nichts zu übernehmen.

`g4k text restore <Projekt> [--base-rom <nds>] [--json]` findet die Basis-ROM wie `text export`
(`--base-rom`, sonst `G4_BASE_ROM_DIR`; die `.nds` daneben nicht, denn ohne `header.yaml` fehlt der
Spielcode zum Vergleich), druckt je Phase `phase=<phase> ms=N`, dann
`restored moved=N members=N raw=N compiled=N encoded=N sync_files=N ms=N`; Exit 1 bei einer
Ablehnung oder einem Fehler. Die App fragt beim Öffnen eines solchen Ordners „Textprojekt
vervollständigen?“ mit `RESTORE_NEEDED` und „Gen4 Studio legt die Spieldateien aus deiner
Basis-ROM an; deine Dateien bleiben, wie sie sind.“ (Task 8 der Phase 4.4).

## IDs in Scripts

In einem Textprojekt (aktuelle Schema-Version, Scripts übernommen) nennen Script-Quellen
Trainer, Items, Spezies, Attacken, Header, Klassen und Orte auch über ihre ID, ohne
Include-Zeile:

```rotom
script Main #1:
    StartTrainerBattle TRAINER_YOUNGSTER_KAI, 0
    if CheckItem(ITEM_POTION, 1) == 1 then
        GivePokemon SPECIES_PIKACHU, 5, 0, VAR_RESULT
    endif
    End
```

**Woher die Namen kommen.** rotom v0.7.0 wertet `#include` und `#define` in DSPRE-Projekten nicht
aus (nur mit einer Decomp-Wurzel; ein Test merkt, wenn sich das ändert), lädt aber jede
`*.json` direkt in seinem Konstantenordner (`database_dir` aus `rotom.toml`, sonst
`.rotom/command_database/`) als `{"<Nummer>": "<NAME>"}`. Gen4 Studio schreibt dort je Art eine
Datei `.g4-ids-<art>.json` (`species` mit den Formen, `moves`, `items`, `abilities`, `types`,
`maps`, `trainers`, `trainer_classes`, `places`, `flags`, `vars`): die IDs der festen Tabellen und
der Register, aufsteigend nach Nummer, kanonisches JSON. `-flags` und `-vars` gibt es nur mit
Registereinträgen und enthalten nur die eigenen Namen; die pret-Namen kennt rotom aus seiner
Datenbank, ein Script mit dem pret-Namen übersetzt zu denselben Bytes wie eines mit dem eigenen. Ersatz-IDs (`TRAINER_0005`) werden nicht für jede
Nummer erzeugt; ein Registereintrag in der Ersatzform der eigenen Nummer (ein Trainer ohne Namen
wurde so getauft, oder eine ID wurde so umbenannt) steht aber darin wie jede andere ID.

```json
{
  "1": "TRAINER_YOUNGSTER_KAI",
  "2": "TRAINER_POKEMON_TRAINER_MIA"
}
```

Der Compile des Kernels, die `rotom`-CLI und rotoms Sprachserver lesen dieselben Dateien; die
Script-Ansicht der App braucht nach neuen IDs eventuell ein Neuladen des Sprachservers. DSPRE
liest den Ordner nie. Der führende Punkt hält die Dateien aus der Quellenliste: sie sind erzeugt
und lokal, nie ein Blob (für Git gehören sie in die `.gitignore`); nach einem Klonen schreibt sie
das Öffnen in Gen4 Studio oder der nächste Compile.

**Wann geschrieben.** Beim Einrichten, nach einer Taufe (ein neuer Trainer, Header, Eintrag),
beim Öffnen eines Projekts in der App und vor jedem Compile (`textproject::include::
write_include`, im Compile `include::refresh`), jede Datei atomar, geprüft und nur, wenn sie sich
ändert; die Datei einer Art ohne IDs verschwindet. Bekommt ein Name eine andere Nummer oder fällt
er weg, werden die Quellen, die ihn nennen, im Compile-Zustand veraltet gesetzt und neu übersetzt.
Der Build-Schritt `rotom` hat die ID-Dateien und die Register als Eingaben.

**Namen, die rotom schon kennt.** rotom leitet eigene Namen aus den Textarchiven ab
(`TRAINER_<NAME>_<NNN>`, `ITEM_…`, `SPECIES_…`, `MOVE_…`, `TRAINER_CLASS_…`, `LOCATION_…`) und
kennt die Konstanten der Befehlsdatenbank. Eine Konstante aus einer JSON schlägt sie still. Darum
fällt eine ID weg, die rotom ohne die ID-Dateien schon mit **anderer** Nummer auflöst (auf Platin
DE etwa zwölf Trainerklassen mit doppeltem Namen: `TRAINER_CLASS_BIKER` ist bei uns 12, bei rotom
13); in Scripts gilt dann rotoms Nummer, und der Check meldet `textproject.id-shadowed` am
Registereintrag. Gleicher Name mit gleicher Nummer bleibt. Ein Name, den schon eine Art davor hat,
fällt ebenso weg, so dass jeder Name in genau einer Datei steht.

**Was nicht geschieht.** Vorhandene Quellen werden nicht auf IDs umgeschrieben; eine
Dekompilierung (Zurücksetzen, Übernahme aus DSPRE) schreibt Zahlen oder rotoms eigene Namen, nie
die IDs. Beide übersetzen zu denselben Bytes. Die Rücknahme lehnt ab, solange eine Quelle eine ID
nutzt, die nur die ID-Dateien kennen (siehe „Einrichten und Zurücknehmen“).

**Im Referenzindex** (`data/refs.rs`) sind die Script-Quellen eine Quelle: welche Parameter welcher
Befehle einen Trainer, ein Item, eine Spezies oder eine Attacke nennen, steht je Familie in
`crates/g4-kernel/metadata/script_refs.yaml`. Ein Argument zählt, wenn es eine Zahl ist oder ein
Name, den der Build auflöst (`alias` der Datei, die IDs, rotoms eigene Namen); ein Wert ab
`0x4000` ist eine Variable, 0 heißt „keiner“. Feld ist der Befehl (rotoms Name, auch für einen
DSPRE-Namen in der Quelle), `slot` die Zeile.

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

## ID umbenennen

Eine **Register-ID** (`MAP_`, `TRAINER_`, `TRAINER_CLASS_`, `PLACE_`, und die eigenen Namen
`FLAG_`, `VAR_`) lässt sich umbenennen; die IDs der festen Tabellen (Spezies, Formen, Attacken,
Items, Fähigkeiten, Typen) und die pret-Namen der Flags und Variablen (`Unbekannte ID: …`) bleiben
fest. Ein eigener Flag-Name zieht dabei auch die `id` seiner Entitätsdatei `data/flags/NNNN.json`
nach (sie liegt unter `data/`). Die
Art einer ID gibt das längste passende Präfix: `TRAINER_CLASS_X` ist eine Trainerklasse, kein
Trainer. Kernel: `textproject::rename::{plan_rename, rename_id, rename_id_files}`, kopflos
`g4k text rename-id <projekt> <alt> <neu> [--json]` (ohne Undo).

**Regeln der neuen ID.** Präfix der Art und `_`, danach nur `A–Z`, `0–9` und `_`, höchstens 64
Zeichen; nicht vergeben (auch nicht als `MAP_<interner Name>` eines anderen Headers); nicht die
Ersatz-ID einer anderen Nummer (die eigene, `TRAINER_0005` für Trainer 5, ist erlaubt und steht
als Registereintrag in den ID-Dateien, Scripts übersetzen weiter); nicht ein
Name, den rotom schon mit anderer Nummer kennt (er fiele aus den ID-Dateien, siehe „IDs in
Scripts“); bei Flags und Variablen nicht der pret-Name einer anderen Nummer (geprüft vor der Form,
pret-Namen haben ein kleines `0x`). Jede Ablehnung kommt, bevor eine Datei geschrieben wird, mit einem dieser Sätze:

- `Kein Textprojekt: IDs gibt es erst nach „Als Textprojekt einrichten…“.` (eine ältere oder
  neuere Schema-Version: der Satz des Manifests)
- `Unbekannte ID: {alt}.`
- `{alt} steht in einer festen Tabelle; umbenennen lassen sich nur IDs mit Register (MAP_, TRAINER_, TRAINER_CLASS_, PLACE_, FLAG_, VAR_).`
- `{neu} ist der pret-Name der Nummer {n}.` (Flags und Variablen)
- `Die neue ID muss mit {PRÄFIX}_ beginnen und darf danach nur A–Z, 0–9 und _ enthalten (höchstens 64 Zeichen).`
- `{neu} wäre eine ID der Art TRAINER_CLASS_; die neue ID muss mit TRAINER_ beginnen, nicht mit TRAINER_CLASS_.`
- `{neu} gibt es schon (Nummer {n}).`
- `{neu} ist die Ersatz-ID der Nummer {n}.`
- `{neu} kennt rotom schon als Nummer {n}.`
- `Die neue ID ist die alte.`
- `{datei} lässt sich nicht lesen ({grund}); erst die Datei reparieren, dann umbenennen.` (eine
  JSON, die die alte ID enthält, aber kein JSON ist)

**Was nachgezogen wird.**

| Datei | was |
|---|---|
| `data/ids/<art>.json` | der Eintrag unter dem neuen Namen (kanonisch, Schlüssel sortiert) |
| `data/**/*.json` (ohne `data/ids/`, ohne die Blobs unter `data/raw/`) | jeder JSON-String, der die alte ID ist, Schlüssel wie Werte: das `id` der Entität, `class` der Trainer, `location_name` der Header, `dest_header_id` der Warps, … |
| `g4-world.json`, `g4-checks.json` | ebenso (Header-Schlüssel und -Werte, `subject.index`) |
| `expanded/scripts/*.rotom` | jedes Bezeichner-Token gleich der alten ID (rotoms Lexer: Kommentare und Strings bleiben) |

Die JSON-Dateien werden String für String ersetzt; alles andere behält seine Bytes. Eine alte ID,
die rotom ohne die ID-Dateien mit **anderer** Nummer auflöst (`textproject.id-shadowed`), ist in
den Scripts rotoms Name: die Script-Quellen bleiben dann, wie sie sind. Danach werden die
ID-Dateien für rotom neu geschrieben (`include::write_include`); sie sind erzeugt und lokal, also
kein Teil des Schritts und nicht mitgezählt (der Plan sagt mit `id_files`, dass es sie gibt). Die
Quellen, die der Abgleich vorher als abgeglichen kannte, werden neu aufgezeichnet: der nächste
Abgleich hält das Umbenennen nicht für eine Änderung von außen. Das Schreiben (auch bei Undo und
Redo) hält die Sperre des Abgleichs für das Projekt, ein Abgleich sieht nie einen halb
umbenannten Stand. Scheitern das Aufzeichnen oder die ID-Dateien, nachdem die Dateien
geschrieben sind, bleibt der Schritt; `RenameResult.warnings` (bzw. `warning:` auf stderr von
`g4k`) nennt es, der nächste Abgleich bzw. Compile holt es nach.

**Ein Undo-Schritt.** Alle Dateien sind **ein** Dateischritt (`FileStep`, wie das Zurücksetzen in
4.1b) mit der Beschriftung `ID umbenannt: {alt} → {neu}`, Asset Trainer `n` bzw. die
Header-Tabelle. ⌘Z schreibt jede Datei zurück, ⌘⇧Z wieder vor; beide prüfen zuerst jede Datei
gegen ihre erwartete Seite. Hat ein anderes Werkzeug eine davon seither geändert, wird nichts
geschrieben, der Schritt fällt aus dem Verlauf, und ein Satz nennt die erste geänderte Datei:
„data/trainers/0005.json wurde seit dem Umbenennen geändert; ⌘Z schreibt nichts, der Schritt
fällt aus dem Verlauf.“ bzw. „… wurde seit dem Rückgängigmachen des Umbenennens geändert; ⌘⇧Z
schreibt nichts, der Schritt fällt aus dem Verlauf.“ (der Schritt trägt die Art
`FileStepKind::IdRename`; das Zurücksetzen von 4.1b behält seinen Satz). Nach Undo und Redo liest der Store die Register neu (`EntitySource` und
`Resolver` sehen sie über `is_current`), und die ID-Dateien werden neu geschrieben.

`plan_rename` zeigt vorher, was geschähe: je Datei die Zahl der Verweise, die Summe, und bei
einer Ablehnung `refusal`. `g4k text rename-id` druckt `<datei> <anzahl>` je Datei und
`renamed <alt> -> <neu> kind=<art> number=<n> files=<n> refs=<n> id_files=true|false ms=<n>`, eine
Ablehnung auf stderr mit Exit 1.

## Schema-Version und Migration

`schemaVersion` zählt die Dateiformate unter `data/` und das Manifest. Jede Namensänderung eines
Schlüssels oder einer Datei erhöht sie und fügt einen Schritt zu `migrate::MIGRATIONS` hinzu; der Test
`schema_fields_match_the_snapshot` vergleicht alle Schlüsselpfade mit `schema.snap` und schlägt fehl,
wenn sich das Schema ohne neue Version ändert. `g4k text migrate` hebt ein älteres Projekt an: alles
im Speicher, dann ein Schnappschuss der berührten Dateien nach
`.g4/backup/textproject-v<von>-<Zeit>/`, dann schreiben. Export, Import, Check und der Build lehnen
eine ältere Version ab („…: `g4k text migrate` ausführen.“), eine neuere immer. `schema.snap` hält
den Umschlag als erste Zeile nach dem Kopf fest (`*.deleted?`).

| Schritt | Was er ändert |
|---|---|
| 0 → 1 | Manifest-Schlüssel `schema` wird `schemaVersion`; jedes Register `{"ids": {"0003": "MAP_T01"}}` wird `{"MAP_T01": 3}` |
| 1 → 2 | in jeder `data/events/NNNN.json` werden `object_events[].hidden_flag` und `coord_events[].var` von der Zahl zum eigenen Namen aus `data/ids/flags.json` bzw. `vars.json`, nur wo einer eingetragen ist (0 und Nummern ohne eigenen Namen bleiben Zahl); sonst ändert sich keine Datei, ein Projekt ohne eigene Namen bleibt bis auf das Manifest byte-gleich. Der Umschlag `deleted` kommt mit dieser Version dazu und braucht keine Änderung |

## Codes

Jede Meldung ist `<code>: <Satz>`, bei einer Entität `<code>: <JSON-Pfad>: <Satz>`. Ein Fehler hält Import
und Build auf; Warnungen und Infos werden gemeldet, halten aber nichts auf.

| Code | Schwere | Bedeutung |
|---|---|---|
| `textproject.schema` | Fehler | Datei passt nicht zum Schema: kein JSON, unbekannter oder vertippter Schlüssel, falscher Typ, Wert außerhalb des Bereichs, Lücke in der Nummerierung; als Warnung: `schemas` nennt für eine Datenart eine höhere Version, als die Fassung kennt |
| `textproject.unknown-id` | Fehler | eine ID, die keine Tabelle und kein Register kennt |
| `textproject.id-mismatch` | Fehler | `id` ist nicht die ID der Nummer der Datei |
| `textproject.side-file` | Fehler | ein Beifile `{"file"}` fehlt oder ist nicht lesbar |
| `textproject.base-missing` | Fehler | ein `base_sha1`-Teil, den weder der Member noch eine Basisschicht hat, oder eine Basisschicht, die sich nicht lesen oder anlegen lässt |
| `textproject.base-mismatch` | Fehler | die Basisschicht hat für den Teil andere Bytes |
| `textproject.map-source` | Fehler | eine Karte mit anderer `source` als `binary` |
| `textproject.header-count` | Fehler | eine Header-Zahl, die das Projekt nicht aufnehmen kann, oder Nummern doppelt bzw. mit Lücke |
| `textproject.member-without-json` | Fehler | ein Member (oder Header) ohne JSON, auch das überzählige Member einer Art aus zwei Archiven (Trainer); der Import löscht nur Member am Ende ohne JSON, die seit der Aufzeichnung unverändert sind (`textproject.member-removed`). Der Export warnt bei dem überzähligen Member mit demselben Code |
| `textproject.member-removed` | Warnung | Import, Build-Schritt und Abgleich: Member (oder Header) am Ende einer Art ohne JSON, seit der Aufzeichnung des Abgleichs unverändert und nicht in der Basis (die JSON ging mit „Nummer freigeben“, auch auf einem anderen Git-Branch); sie werden entfernt (Kürz-Regel, siehe „Export, Import, Check“) |
| `textproject.header-table` | Fehler | die Header-Tabelle ist nicht lesbar oder nicht zu finden |
| `textproject.quirk-key` | Fehler | ein Schlüssel der Quirks-Datei ist keine vorhandene Quelle unter `expanded/scripts` |
| `textproject.raw-path` | Fehler | ein Blob außerhalb des Bereichs der Blobs oder an einem Pfad einer Entität |
| `textproject.unknown-art` | Warnung | ein Ordner unter `data/`, den keine Art kennt; bleibt unberührt |
| `textproject.reserved` | Info | `schemas` hat einen Schlüssel, der keine Datenart ist, oder `plugins` ist nicht leer |
| `textproject.id-dangling` | Warnung | ein Registereintrag nennt eine Nummer, die es nicht mehr gibt; er bleibt (Flags, Variablen: der eigene Name hat keine Entitätsdatei) |
| `textproject.id-shadowed` | Warnung | Check: eine ID fehlt in den ID-Dateien für rotom, weil rotom den Namen schon mit anderer Nummer kennt (in Scripts gilt rotoms) oder eine Art davor ihn hat |
| `textproject.base-unchecked` | Warnung | Binärmember ohne Datei ließen sich ohne Basisschicht nicht vergleichen |
| `textproject.member-newer` | Warnung | Build: ein Member ist seit dem letzten Build geändert, seine JSON nicht; der Member bleibt (`g4k text export` übernimmt ihn, `g4k text import` verwirft ihn) |
| `textproject.conflict` | Fehler | Build: JSON und Member einer Einheit sind beide seit dem letzten Build geändert; nichts geschrieben |
| `textproject.unsynced` | Fehler | Build ohne Eintrag (erster Build, „ohne Cache“, neues Salz): ein Member weicht von seiner JSON und von der Basisschicht ab; nichts geschrieben |

## Quellenliste

`textproject::sources` (`g4k text sources`) listet die Dateien des Textprojekts, relativ zur Wurzel,
mit `/`, sortiert, je wenn vorhanden: `g4-project.json`, alles unter `data/`,
`expanded/textArchives/*.json`, `expanded/scripts/**`, `rotom.toml`, `.rotom/command_database/**`,
`g4-world.json`, `g4-checks.json`, `.g4/base.json`, `.g4/preview.png`; versteckte Einträge in den
Ordnern nicht (also auch nicht die ID-Dateien `.g4-ids-*.json`). Alles andere entsteht aus diesen
Dateien und der Basis-ROM.
