# Karten-Header und Ortsnamen

Quelle: Das Layout folgt uxie 0.9.7 (`map_header/binary.rs`, `map_header/types.rs`:
`MapHeaderDP`, `MapHeaderPt`, `MapHeaderHGSS`); Zahlen und Werte zu den Vanilla-Daten stammen
aus dem Gutachten A (`docs/research/2026-10-25-ausbau/gutachten-a-weltmodell.md`, §1 und §3
„Zu Frage 2“). Studio-seitig: `crates/g4-kernel/src/places.rs` (Orte) und
`crates/g4-kernel/src/checks/world.rs` (Prüfungen). Alle Werte Little Endian. Wo die Daten
des Spiels selbst nicht nachgeprüft sind, steht es dabei.

Ein Header ist ein 24-Byte-Eintrag der Ortstabelle: er verbindet Matrix, Skriptdatei,
Textarchiv, Musik, Begegnungen, Eventdatei und Ortsname eines Gebiets oder Raums. Wo die
Tabelle liegt (fest in `arm9.bin` oder, mit dem Patch, als Dateien), steht in
[`dynamic_headers.md`](dynamic_headers.md). Die Zahl der Header ist die Zahl der 16-Byte-Namen
in `files/fielddata/maptable/mapname.bin` (Platin 593, HeartGold/SoulSilver 540).

## Platin (24 Byte)

| Offset | Typ | Feld | Bedeutung |
|---|---|---|---|
| 0 | u8 | `area_data_id` | Satz der Texturen (siehe [`area_data.md`](area_data.md)) |
| 1 | u8 | `unknown1` | unbekannt |
| 2 | u16 | `matrix_id` | Member von `map_matrix.narc` (siehe [`matrix.md`](matrix.md)) |
| 4 | u16 | `script_file_id` | Skriptdatei |
| 6 | u16 | `level_script_id` | Level-Skript |
| 8 | u16 | `text_archive_id` | Textarchiv der Dialoge dieses Headers |
| 10 | u16 | `music_day_id` | Musik am Tag |
| 12 | u16 | `music_night_id` | Musik in der Nacht |
| 14 | u16 | `wild_pokemon` | Begegnungsdatei; `0xFFFF` = keine (`ENCOUNTERS_NONE_DPPT`) |
| 16 | u16 | `event_file_id` | Eventdatei (Warps, Figuren, Trigger, Schilder) |
| 18 | u8 | `location_name` | Index in die Ortsnamen-Bank (siehe unten) |
| 19 | u8 | `area_icon` | Stil der Gebietseinblendung |
| 20 | u8 | `weather_id` | Wetter |
| 21 | u8 | `camera_angle_id` | Kamerawinkel |
| 22 | u16 | `map_settings` | Bits 0–6 `location_specifier`, Bits 7–11 `battle_background`, Bits 12–15 `flags` |

## HeartGold / SoulSilver (24 Byte)

| Offset | Typ | Feld | Bedeutung |
|---|---|---|---|
| 0 | u8 | `wild_pokemon` | Begegnungsdatei; `0xFF` = keine (`ENCOUNTERS_NONE_HGSS`) |
| 1 | u8 | `area_data_id` | Satz der Texturen |
| 2 | u16 | Koordinaten | Bits 0–3 `unknown0`, Bits 4–9 `worldmap_x`, Bits 10–15 `worldmap_y` |
| 4 | u16 | `matrix_id` | Member von `a/0/4/1` |
| 6 | u16 | `script_file_id` | Skriptdatei |
| 8 | u16 | `level_script_id` | Level-Skript |
| 10 | u16 | `text_archive_id` | Textarchiv der Dialoge |
| 12 | u16 | `music_day_id` | Musik am Tag |
| 14 | u16 | `music_night_id` | Musik in der Nacht |
| 16 | u16 | `event_file_id` | Eventdatei |
| 18 | u8 | `location_name` | Index in die Ortsnamen-Bank |
| 19 | u8 | Gebietsbyte | Bits 0–3 `area_icon`, Bits 4–7 `unknown1` |
| 20 | u32 | Schlusswort | Bit 0 `kanto_flag`, Bits 1–7 `weather_id`, Bits 8–11 `location_type`, Bits 12–17 `camera_angle_id`, Bits 18–19 `follow_mode`, Bits 20–24 `battle_background`, Bits 25–31 `flags` |

`worldmap_x/y` ist die Lage des Headers auf der Pokégear-Karte; 132 der 540 HGSS-Header haben
(0, 0). `kanto_flag` haben 199 Header (Zahlen aus dem Gutachten).

Zum Vergleich Diamant/Perl: gleiche Reihenfolge bis Offset 16 wie Platin, aber
`location_name` ist ein **u16** bei 18, danach `weather_id` (20), `camera_angle_id` (21),
`location_specifier` (22) und ein Byte `map_settings` (23: Bits 0–3 `battle_background`,
Bits 4–7 `flags`). Das Studio legt in DP keine eigenen Orte an.

## Ortsname

`location_name` ist in Platin und HGSS ein **u8** und zugleich der Index einer Nachricht in
der Ortsnamen-Bank des Textarchivs (Format: [`text_archive.md`](text_archive.md)). Die Bank
hängt vom Spiel und der Sprache ab (uxie `TextBankIds::locations`):

| Spiel | Bank | Namen in Vanilla | freie Plätze (Index) |
|---|---|---|---|
| Platin | 433 (JP 427, KR 428) | 126 | 130 (126–255) |
| HeartGold/SoulSilver | 279 (JP 272, KR 274) | 235 | 21 (235–255) |

Index 0 ist der „Mysteriöse Ort“ und gilt im Studio als „kein Ort“. Ein u8 adressiert höchstens
256 Namen. Der Ortsname ist nicht der interne Name des Headers (`mapname.bin`, etwa
`C01PC0102`) und nicht das Textarchiv der Dialoge (`text_archive_id`).

Pt benutzt 125 der 126 Namen (nur 92 „GTS“ nicht), HGSS nur 121 der 235. Der längste Name
beider Spiele hat 16 Zeichen („Wettbewerbshalle“; Kürzungen wie „Schleiede-Kaufh.“ deuten auf
eine feste Grenze). Das ist ein Befund aus den Namen, kein belegter Wert aus dem Spielcode.

### HGSS: Index 1–125 sind Sinnoh-Namen

In HGSS stehen auf den Indizes 1–125 die Sinnoh-Namen (von Zweiblattdorf bis Gipfelruinen);
Johto und Kanto beginnen bei 126 (Neuborkia). Der Grund: derselbe Index steckt als „Fundort“
im Pokémon, und Pokémon aus Sinnoh kommen per Transfer oder Pal Park herein. Ein neuer Name
auf einem solchen Index ändert also den angezeigten Fundort dieser Pokémon. Ein Umbenennen
trifft immer alle Header mit diesem Index und alle Pokémon mit diesem Fundort.

## Ortstyp, Gebietssymbol, Wetter

Das Studio kennt den Ortstyp als `HeaderSummary.map_type`: in Platin `location_specifier`
(Bits 0–6 von `map_settings`), in HGSS `location_type` (Bits 8–11 des Schlusworts). Die Werte
nach den Vanilla-Daten (Zahl der Header Pt / HGSS in Klammern):

| Wert | Bedeutung |
|---|---|
| 0 | nichts (`NOTHING`) |
| 1 | Stadt (20 / 32) |
| 2 | Route, Außenbereich (71 / 64) |
| 3 | Höhle, Dungeon (166 / 116) |
| 4 | Gebäude (334 / 326) |
| 6 | Untergrund |

Die Bedeutungen sind aus den Daten abgeleitet, nicht aus dem Spielcode belegt; 5 und höher
(außer 6) kommen in den untersuchten Daten nicht vor oder sind nicht ausgewertet.
`area_icon` (Pt Byte 19, HGSS Bits 0–3 von Byte 19) wählt den Stil der Einblendung beim Betreten
des Gebiets; das Studio setzt ihn nicht selbst, ein neuer Ort übernimmt ihn vom Header, der
ihn bekommt. Welche Zahl welchen Stil ergibt, ist hier nicht belegt.

## Was das Studio anlegt

Alles liegt in `places.rs` und läuft über `Store::transaction`: ein Befehl ist ein
Undo-Schritt, ungespeicherte Namen sind sofort in den Listen (die Namen kommen aus dem Store,
nicht vom Datenträger). Gespeichert wird die Bank wie jedes Textarchiv, die Header wie in
`dynamic_headers.md` beschrieben.

- `create_place(name, headers)` hängt den Namen als neue Nachricht an die Bank (Index = Zahl
  der bisherigen Namen) und setzt `location_name` aller `headers` auf diesen Index, in einem
  Undo-Schritt „Ort „…“ angelegt“. Die Nachricht bekommt die Kennung
  `default_message_id(index)`. Ist die Bank mit 256 Namen voll und ist es HGSS, ersetzt der Name
  stattdessen den kleinsten ungenutzten Sinnoh-Namen (1–125, siehe oben); das Ergebnis meldet
  den Index und den ersetzten Namen (`PlaceCreated.reused`).
- `rename_place(index, name)` ersetzt den Namen für alle Header mit diesem Index, ein
  Undo-Schritt „Ort „alt“ in „neu“ umbenannt“. Index 0 ist gesperrt.
- `assign_place(headers, index)` setzt `location_name` auf einen vorhandenen Ort, ein Undo-Schritt
  „Ort „…“ zugewiesen“.
- `place_slots` liefert `{ bank, count, free, reusable }`: `free = 256 - count` (nie
  negativ), `reusable` die Sinnoh-Indizes 1–125, die kein Header benutzt (nur HGSS).

Abgelehnt wird (nichts ändert sich): DP („Eigene Orte gibt es nur für Platin und
HeartGold/SoulSilver“), ein leerer Name, ein Name mit mehr als einer Zeile (Zeilenumbruch oder
`\n`, `\r`, `\f` im Text), ein Zeichen, das chatot nicht kodiert, ein Header, den die Tabelle nicht
hat, ein Index ohne Namen in der Bank und eine volle Bank ohne wiederverwendbaren Platz. Der
Name wird nie gekürzt.

### Prüfungen

- `place.name-too-long` (Warnung): ein Name der Bank hat mehr als 16 Zeichen; Argumente
  `[Index, Name, Länge]`. Länger ist erlaubt, das Spiel kann den Namen aber abschneiden.
- `place.index-out-of-bank` (Fehler): ein Header hat `location_name` ungleich 0 und nicht
  kleiner als die Zahl der Namen der Bank; Argumente `[Header, Index, Zahl der Namen]`.

Auf den drei Vanilla-Projekten findet keine der beiden Prüfungen etwas (Pt größter Index 125
bei 126 Namen, HGSS 234 bei 235).

## Grenzen

- Die Stadtkarte bzw. Pokégear-Karte und die Flugziele stehen nach unserem Wissen in
  Code-Tabellen der Overlays, nicht im Header. Im Repo ist das **nicht belegt**; die Recherche
  dazu ist offen. Ein neuer Ort erscheint dort deshalb nicht. HGSS hat mit `worldmap_x/y` immerhin
  die Lage auf der Pokégear-Karte im Header.
- Das Studio setzt `worldmap_x/y`, `kanto_flag`, `area_icon` und den Ortstyp beim Anlegen nicht
  selbst; das Anlegen ändert nur die Bank und `location_name`.
- Mehr als 256 Orte sind nicht möglich (u8). In Platin bleiben 130 Plätze, in HGSS 21 plus die
  ungenutzten Sinnoh-Namen.
- Ohne den Patch „Dynamische Header“ ist die Zahl der Header fest; ein Ort braucht aber keinen
  neuen Header, nur einen neuen Namen. Neue Header (und damit neue Gebiete) gibt es nur mit
  dem Patch, siehe [`dynamic_headers.md`](dynamic_headers.md).
