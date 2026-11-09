# Tileset (Kacheln eines Kartentextur-Pakets, Phasen 6.3a, 6.3b, 6.4a, 6.5b und 6.4b)

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Ein Tileset sagt je Textur eines
Kartentextur-Pakets (`map_tileset` der Gebietsdaten, Member von `mapTextures`), wie sie als Kachel
einer [Welt-Karte](world_map.md) wirkt: Rolle auf dem Boden, Vorgaben der Permissions,
Höhenverhalten, Wandtextur, Fußabdruck eines Teilmodells; dazu die Kantensätze des Terrain- und
Klippenpinsels (6.4b, „Kantensätze (6.4b)“). Quelle: [Spezifikation Phase 6](../superpowers/specs/2026-11-05-phase-6-kartenerstellung-design.md)
§3.2, [Gutachten C](../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §3.3 und §5.2,
die Messung von Task 0 in [task0-6-3a-vorgaben.md](../research/2026-11-05-phase-6/task0-6-3a-vorgaben.md),
Plan [2026-11-05-phase-6-3a-weltformat.md](../superpowers/plans/2026-11-05-phase-6-3a-weltformat.md)
(„Entscheidungen“), für die Kantensätze Plan [2026-11-07-phase-6-4b-smart-drawing.md](../superpowers/plans/2026-11-07-phase-6-4b-smart-drawing.md).
Implementierung: `crates/g4-kernel/src/world/tileset.rs`, die Kantensätze `world/edge.rs`, die Karten eines
Pakets `world/places.rs`, der Zwischenspeicher `Store::tileset`. Texturen liest es über die
bestehenden Leser (`g4_formats::nsbtx`, `nsbmd_header::material_texture_names`).

**Nie im Repo.** Ein Tileset, das aus einer ROM abgeleitet ist, enthält die Texturnamen und
Vorgaben des Spiels: es entsteht nur im Projekt des Nutzers (`data/tilesets/`) oder im Speicher,
nie als Fixture, Beispiel oder Testdatei im Repository (keine Vanilla-Tileset-Metadaten, keine
Texturnamen aus einer ROM; `scripts/check-no-rom-data.sh` fängt die ROM-Dateien selbst ab, diese
Regel gilt darüber hinaus). Das einzige Tileset im Repo ist das synthetische aus Code
(unten); alle Beispiele hier stammen daraus.

## Datei

`data/tilesets/NNNN.json`, der Stamm ist die Nummer des Pakets (vier Dezimalziffern). Kanonisches
JSON wie jede Datei des Textprojekts, Schlüssel in der Reihenfolge der Tabelle, unbekannte
Schlüssel abgelehnt, Zahlen als JSON-Zahlen (die Oberfläche zeigt sie hex). Die Ordnerart
`tilesets` hat keine IDs und kein Register; Export und Import lassen den Ordner liegen, nur der
Check liest ihn (Dateiart `FILE_KINDS` in `textproject/kinds.rs`, Schemaschlüssel `tilesets` im
Manifest, siehe [text_project.md](text_project.md)).

| Schlüssel | Typ | Inhalt |
|---|---|---|
| `version` | Zahl | immer `1` (`TILESET_VERSION`) |
| `package` | Zahl | das Paket; gleich dem Stamm der Datei |
| `tiles` | Liste | eine Kachel je Textur des Pakets: Kachel `i` ist Textur `i` |
| `edge_sets` | Liste | die Kantensätze des Terrain- und Klippenpinsels (6.4b, „Kantensätze (6.4b)“); leer weggelassen |

Je Kachel:

| Schlüssel | Typ | Inhalt |
|---|---|---|
| `texture` | Text | Name der Textur im Paket (höchstens 16 Byte) |
| `texture_index` | Zahl | Index der Textur im Paket |
| `palette` | Text, weggelassen | Name der Palette (fehlt: keine, dekodiert mit Palette 0) |
| `size` | `[b, h]` | Breite und Höhe in Texeln |
| `role` | `ground` \| `wall` \| `ramp` \| `water` \| `grass` \| `piece` | was die Kachel auf der Karte ist |
| `kind` | Zahl | Vorgabe des Verhaltens (`kind`) einer Zelle mit dieser Kachel |
| `collision` | Zahl | Vorgabe des Kollisionsbytes (Bit 7 sperrt) |
| `height` | `flat` \| `ramp` \| `wall` | wie eine Zelle mit der Kachel ihre Höhe nimmt |
| `sound` | Zahl, weggelassen bei 0 | nur HeartGold/SoulSilver: Schrittgeräusch, Bits 0 bis 6 des Kollisionsbytes |
| `wall` | Zahl, weggelassen | die Kachel (Rolle `wall`), deren Textur die Wände unter einer Zelle mit dieser Kachel tragen |
| `footprint` | `[b, t]`, weggelassen | Breite und Tiefe in Zellen; genau die Kacheln der Rolle `piece` haben einen |
| `model` | Text, weggelassen | seit 6.5b: der `<name>` des Modell-Beifiles `data/tilesets/PPPP.<name>.glb` (`PPPP` das Paket) eines Teilmodells; nur bei Rolle `piece`, 1 bis 32 Zeichen aus a–z, 0–9, `_`, `-`; fehlt: das Teilmodell ist seine Fußabdruck-Fläche (unten „Teilmodell setzen (6.5b)“) |
| `origin` | `vanilla` \| `unclear` \| `synthetic` \| `user` | woher die Werte kommen: abgeleitet, nicht entscheidbar, synthetisch, vom Nutzer |

`model` war bis 6.5b reserviert und ist seitdem erlaubt; ob das Beifile da ist, prüft erst das
Backen (`world.side-file`, [world_map.md](world_map.md) „Freiform und Beifiles (6.5b)“).
`edge_sets` war bis 6.4a ebenso reserviert; eine Datei ohne den Schlüssel liest seit 6.4b
unverändert. `TILESET_VERSION` bleibt 1, keine Migration.

## Prüfungen

Der Check des Textprojekts liest jede Datei unter `data/tilesets/` (`tileset::check_folder`), jede
Meldung `textproject.schema` mit Datei, Zeile und dem JSON-Pfad in der Datei:

- ein Dateiname, der keine Paketnummer ist: „Der Dateiname ist keine Paketnummer (`NNNN.json`); die
  Datei gehört zu keinem Kartentextur-Paket.“
- eine Datei, die serde nicht liest (Pfad und Grund von serde);
- `version`: „Version 2 kennt diese Fassung nicht (1).“; `package`: „Paket 0007 passt nicht zum
  Dateinamen 0006.json.“;
- `tiles[i].model` (seit 6.5b) auf einer Kachel, die kein Teilmodell ist („`model` haben nur
  Teilmodelle (`piece`).“), oder ein Name außerhalb der Regel („`model` ist ein Name aus a–z, 0–9,
  `_`, `-`, höchstens 32 Zeichen.“); beides wird getrennt gemeldet;
- ein Modell-Beifile `PPPP.<name>.glb` im Ordner, das keine Kachel von `PPPP.json` als `model`
  nennt (auch ohne `PPPP.json`): „Das Beifile 0006.baum.glb nennt keine Kachel von 0006.json;
  löschen oder `model` setzen.“ Lässt sich `PPPP.json` nicht lesen, bleibt es bei deren Problem.
  Jede andere Datei, die kein `NNNN.json` ist, hat weiter den Satz „Der Dateiname ist keine
  Paketnummer …“;
- je Kantensatz (`edge_set_problems`, Pfad `edge_sets[i]`): ein leerer Name („edge_sets[0].name: Ein
  Kantensatz braucht einen Namen.“), ein Name zweimal („edge_sets[1].name: Kantensatz „Weg“ gibt es
  zweimal.“); je Token in `outside` und den Plätzen ein Token, das der Parser ablehnt (sein Satz:
  „edge_sets[0].places.n[1]: `0g07` ist keine Kachel …“), `.` („`.` ist keine Kachel eines
  Platzes.“), eine Kachel jenseits des Tilesets („edge_sets[0].places.center[0]: Kachel 0x0006 gibt
  es im Tileset 0000 nicht (6 Kacheln).“) und dasselbe Token zweimal im Satz, auch als `outside`
  („edge_sets[0].places.e[0]: Kachel 0007 steht zweimal im Satz („n“ und „e“).“); ein unbekannter
  Platz oder ein `block` außer `upper`/`lower` ist ein Fehler von serde;
- `tiles[i].wall` jenseits der Kacheln („Kachel 0x0120 gibt es im Tileset 0006 nicht (94
  Kacheln).“) oder ohne Rolle `wall` („Kachel 0x0002 ist keine Wand (Rolle `water`).“);
- `tiles[i].footprint` auf einer Kachel, die kein Teilmodell ist („einen Fußabdruck haben nur
  Teilmodelle (`piece`).“), ein Teilmodell ohne („ein Teilmodell (`piece`) braucht einen Fußabdruck
  (`footprint`).“);
- ein Texturname über 16 Byte; `sound` ≠ 0 außerhalb von HeartGold/SoulSilver („Geräusche gibt es
  nur in HeartGold/SoulSilver.“) oder über Bit 6.

Eine höhere Version unter `schemas.tilesets` im Manifest ist eine Warnung, der Ordner bleibt
ungeprüft, wie er ist.

## Welches Tileset gilt

`Store::tileset(package)` gibt die Tileset-Datei, wenn es sie gibt, sonst **`textures_only`** des
Pakets: je Textur eine Kachel mit Name, Index, Größe und Palette, Rolle `ground`, `kind 0`,
`collision 0`, `height flat`, `origin unclear`, ohne eine Karte zu lesen. Das reicht für Vernetzen,
Szene und die Bereichsprüfung der Kacheln (`world.schema`). Der Store hält das Ergebnis je Paket
zwischen und liest neu, wenn sich Größe oder Änderungszeit der Datei bzw. des Paket-Members ändern
oder die Datei kommt oder geht.

Die Ableitung mit Vorgaben (`derive_tileset`) läuft nur in `tileset_get` ohne Datei (Antwort
`stored: false`) und in `tileset_save`. Angelegt wird die Datei **nur** mit `tileset_save`, geändert
mit `tileset_edit` (6.3b, unten); „Welt anlegen“ schreibt keine. `tileset_save(package)` leitet ab und schreibt
`data/tilesets/NNNN.json` als einen Dateischritt („Tileset 0006 abgeleitet“, Undo entfernt die Datei
wieder) und antwortet mit `{package, file, tiles, vanilla, unclear, change}`. Abgelehnt: im
DSPRE-Layout („Boden malen gibt es im Textprojekt (Technik › Textprojekt).“), wenn die Datei schon
da ist („data/tilesets/0006.json gibt es schon.“), ein Paket, das es nicht gibt
(„Kartentextur-Paket 0200 gibt es nicht (74 Pakete).“), eines, das sich nicht lesen lässt
(„Kartentextur-Paket 0006 lässt sich nicht lesen: …“). `tileset_get` leitet in jedem Layout ab.

## Kacheln ändern (`tileset_edit`, 6.3b)

`tileset_edit {req: {package, changes: [{tile, role?, kind?, collision?, sound?, wall?}], sets?}}` ändert
Kacheln der vorhandenen Datei `data/tilesets/NNNN.json` als **einen** Dateischritt wie
`tileset_save` (`FileStepKind::Reset`; Undo schreibt die Datei, wie sie war), Label „Tileset 0006
geändert (2 Kacheln)“ („(1 Kachel)“). Ein fehlender Schlüssel lässt den Wert, `"wall": null`
entfernt die Wandkachel. Gezählt und mit `origin: user` versehen werden nur Kacheln, deren Werte
sich wirklich ändern; eine Rolle setzt kein `kind` mit (Gras mit abgeleiteter Begegnung braucht
`role: "grass"` und `kind: 2` getrennt). Antwort `{package, tiles, change}`. `changes` ist optional
(Vorgabe leer); seit 6.4b ändert dasselbe Kommando mit `sets` die Kantensätze im selben Schritt
(„Kantensätze ändern“ unten).

Abgelehnt (nichts geschrieben, Verlauf unverändert): im DSPRE-Layout („Boden malen gibt es im
Textprojekt (Technik › Textprojekt).“), ohne Datei („Erst `tileset_save` für Paket 0006.“), eine
Kachel jenseits des Tilesets („changes[0].tile: Kachel 0x0120 gibt es im Tileset 0006 nicht (94
Kacheln).“), ein Aufruf, der weder eine Kachel noch einen Kantensatz ändert („Nichts zu ändern.“), ein Ergebnis mit einem Problem der
Prüfungen oben (das erste, mit dem Pfad der Datei: „data/tilesets/0006.json: tiles[0].wall: …“),
eine Datei, die sich nicht lesen lässt.

Danach backt der Store die sauberen geladenen Welt-Karten dieses Pakets an Ort und Stelle neu
(`Store::forget_world_bakes`, ihr Verlauf bleibt; das zwischengespeicherte Tileset des Pakets fällt
vorher weg), schmutzige backt das nächste Speichern; dasselbe nach `tileset_save`, nach dem
Dateischritt der Kartentexturen des Pakets (`append_textures`) und nach Undo und Redo eines dieser
Dateischritte (`Store::record_file_step` erkennt sie an der Tileset-Datei bzw. am Asset `MapTexture`). Der Build-Schritt `world` backt die Karten des Pakets beim
nächsten Build, denn die Tileset-Datei gehört zu ihrem Eingabe-Hash ([build_state.md](build_state.md)).
Der Tileset-Editor von 6.4a baut auf diesem Befehl (unten).

## Teilmodell setzen (6.5b)

Ein Teilmodell (Rolle `piece`) kann statt seiner Fußabdruck-Fläche die Geometrie eines Modells
tragen: `model: "<name>"` an der Kachel nennt das **Modell-Beifile** `data/tilesets/PPPP.<name>.glb`
neben `PPPP.json`, in der kanonischen Form der Beifiles ([world_map.md](world_map.md) „Freiform und
Beifiles (6.5b)“); wo es auf der Karte steht und wie es sich dreht, steht in world_map.md
„Teilmodelle mit Geometrie (6.5b)“. Implementierung: `tileset_piece_set` in `world/tileset.rs`.

`tileset_piece_set {req: {package, tile, path, footprint: [Breite, Tiefe], options?, name?}}` liest
das Modell (OBJ oder glTF, `read_model`, `options` wie beim Import von 6.5a, Vorgabe `unit` 16), plant
seine Texturen gegen das Paket und macht **einen** Undo-Schritt „Teilmodell gesetzt: Kachel 5 in
Tileset 0006 (baum)“: neue Texturen ans Paket (`append_textures`), das Beifile als Dateischritt,
dann die Tileset-Datei als Dateischritt mit Rolle `piece`, `footprint`, `model` und `origin: user`;
das Aufzeichnen dieses Schritts backt die sauberen geladenen Welt-Karten des Pakets neu
(`Store::forget_world_bakes`, schon mit dem Beifile), Undo und Redo ebenso. Der Name kommt aus
`name`, sonst aus dem Dateistamm (`side_name`, nummeriert an den Namen der anderen Kacheln und an
Beifiles des Pakets vorbei, die schon im Ordner liegen; die Kachel darf ihren eigenen behalten).
Das bisherige Modell-Beifile der Kachel geht im selben Schritt, wenn keine andere Kachel es nennt
(sonst meldete der Check eine verwaiste Datei). Antwort `{package, tile, model, stats, textures,
notes, change}`.

Abgelehnt (nichts geschrieben): außerhalb eines Textprojekts („Boden malen gibt es im Textprojekt
(Technik › Textprojekt).“), ohne Tileset-Datei („Erst `tileset_save` für Paket 0006.“), eine Kachel
jenseits des Tilesets („tile: Kachel 0x0120 gibt es im Tileset 0006 nicht (94 Kacheln).“), ein
Fußabdruck mit einer 0 („Ein Fußabdruck ist mindestens 1 × 1 Zelle (0 × 2).“), ein Modell, das der
Leser oder die Umwandlung ablehnt („Teilmodell nicht gesetzt: <Satz> (<Code>)“), ein Ergebnis mit
einem Problem der Prüfungen oben („data/tilesets/0006.json: …“), ein beschäftigter Store.
Scheitert ein Teil des Schritts, nimmt er alles zurück.

**Beifile-Ordner.** Neben `PPPP.json` liegen die Modell-Beifiles `PPPP.<name>.glb`, die eine Kachel
nennt. Ein Beifile, das keine Kachel nennt, meldet der Check als `textproject.schema` („Prüfungen“);
**gelöscht wird im Ordner nichts** von selbst, auch nicht beim Speichern, beim Export oder mit
`tileset_edit` (eine verwaiste Datei bleibt, bis der Nutzer sie löscht oder wieder nennt). Speichern,
Export und `tileset_edit` lassen die genannten Beifiles byte-gleich stehen.

`tileset_edit` nimmt seit 6.5b auch `model` (`{tile, model: "<name>"}`, `"model": null` entfernt
den Namen; das Beifile bleibt liegen). Ein Befehl in der App, der ein Teilmodell setzt, fehlt noch;
der Weg ist `tileset_piece_set` über `g4_call` ([mcp.md](../mcp.md) „Freiform, Form 2, Tiled (6.5b,
über `g4_call`)“).

## Palette und Tileset-Editor (6.4a)

Solange das Werkzeug „Gelände malen“ eine Welt-Karte malt, zeigt der Inspector ihrer Zelle (Bauen und
Tüfteln) vor „Quelle“ und „Budget“ drei Abschnitte: „Palette“, „Ebenen“ ([world_map.md](world_map.md),
„Malen (6.4a)“) und „Kachel 0029“, den Tileset-Editor. Implementierung:
`apps/studio/src/workbench/inspector/TilePalette.tsx`, `TileEditor.tsx` (im `MatrixCellEditor`), die
Bilder `world/paint/paletteImages.ts`. Das Tileset liest die Malsitzung mit `tileset_get` für das
Paket der gezeigten Welt, nach jeder Revision neu (auch nach Undo und Redo eines Tileset-Schritts, der
am Asset `HEADER_TABLE` hängt).

**Palette** (`[data-paint-palette]`): die Kacheln des Tilesets nach Rolle in der Reihenfolge Boden,
Gras, Wasser, Wand, Rampe, Teilmodell (`[data-palette-role]`, leere Gruppen fehlen), je Kachel eine
Taste 32 × 32 (`[data-palette-tile="0029"]`, die vier Hex-Ziffern wie im Weltdokument,
`aria-pressed` für die gewählte). Das Bild ist die **ganze** Textur aus `get_tileset_texture`
(RGBA → `ImageData` → `ImageBitmap`, ohne Glättung gezeichnet, je Paket und Textur
zwischengespeichert; derselbe Speicher füttert die Strichvorschau); ohne Bild (`null`, etwa das
synthetische Tileset oder eine Textur im Format 5) die Platzhalterfarbe der Rolle. Tooltip „Kachel
0001 · syn_gras · 32 × 32 Texel · Gras“, in Tüfteln dazu „kind 0x02, Kollision 0x00“ und die Nummer
unter der Taste. Ein Klick wählt die Kachel ohne Drehung und Spiegelung; aus Radierer, Pipette, Höhe
und Rampe geht es dabei zum Pinsel, Rechteck und Füllen bleiben. Die gewählte Kachel trägt ihre
Drehung und Spiegelung als kleine Marke („90°“, „⇆“).

**Tileset-Editor** (`[data-tile-editor]`, Abschnitt „Kachel 0029“ für die Kachel der Palette, die
Nummer wie dort): jede bestätigte Änderung eines Felds ist **ein** `tileset_edit` mit einer Änderung,
also ein Dateischritt „Tileset 0000 geändert (1 Kachel)“, und gibt der Kachel `origin: user`.

| Feld | Baustein | Änderung |
|---|---|---|
| „Rolle“ | Auswahl der sechs Rollen (Boden, Wand, Rampe, Wasser, Gras, Teilmodell) | `{tile, role}`; `kind` und `height` bleiben |
| „Gelände“ | Bauen: Auswahl der Verhaltensnamen aus `tile_behaviors*.yaml` (dieselbe Quelle wie das Gelände-Overlay); Tüfteln: Zahlfeld hex mit dem Hinweis „0x02 <Name>“ | `{tile, kind}` |
| „Blockiert“ | Kästchen, Bit 7 von `collision` | `{tile, collision}` mit gesetztem bzw. gelöschtem Bit 7, die übrigen Bits bleiben |
| „Schrittgeräusch“ | nur HeartGold/SoulSilver, Zahlfeld, Hinweis „0 bis 127“ | `{tile, sound}` |
| „Wand“ | Textfeld mit Hex-Ziffern wie die Palette, leer „keine“, Hinweis „Kachelnummer, leer = keine“; außerhalb: „Eine Kachel von 0000 bis 0005.“ | `{tile, wall}`, leer `{tile, wall: null}` |

Darunter die Herkunft der Vorgaben: „Aus den Vanilla-Karten abgeleitet“ (`vanilla`), „Nicht eindeutig,
Vorgabe Boden“ (`unclear`), „Von dir gesetzt“ (`user`); die synthetische Herkunft hat keinen Satz.
**Ohne Datei** (`stored: false`, das abgeleitete Tileset) sind die Felder gesperrt, darüber steht die
Taste „Tileset im Projekt anlegen“ (`[data-tileset-save]`, `tileset_save`, ein Dateischritt) mit dem
Satz „Die Vorgaben stehen dann in data/tilesets/0006.json und gelten für alle Welt-Karten dieses
Pakets.“ Eine Kachel, die das Tileset nicht hat: „Das Tileset hat keine Kachel 0029.“ Nach jeder
Änderung frischt die App Netz, Bake und Budget der Karte auf; eine Ablehnung geht an den Fehlerweg des
Inspectors.

Beispiel auf dem synthetischen Tileset: „Blockiert“ an Kachel 1 (`syn_gras`) schickt
`tileset_edit {req: {package: 0, changes: [{tile: 1, collision: 128}]}}`, „Wand“ = `0003` an
Kachel 4 `{tile: 4, wall: 3}`, „Rolle“ Wasser an Kachel 0 `{tile: 0, role: "water"}`; jede ist ein
eigener Verlaufseintrag „Tileset 0000 geändert (1 Kachel)“.

## Palette beim Backen (6.3b)

Das Backen einer Welt-Karte bindet jede Textur mit der Palette, die das Tileset für sie nennt, wenn
das Paket diese Palette hat (`world::bake::world_pack_textures`), sonst wie `bake::pack_textures`
(die gleichnamige Palette, sonst `<Textur>_pl`, sonst die einzige des Pakets). Nennen mehrere Kacheln dieselbe Textur,
entscheidet die erste Kachel in Tileset-Reihenfolge, die eine Palette nennt; eine andere Palette
einer späteren Kachel bleibt ungenutzt. Grund: was die Szene zeigt, zeigt das Spiel; Platins
`allpeak` bindet in jedem Vanilla-Modell die Palette seines Index (`apeak`), die `pack_textures`
nicht findet.

## Ableitung

Kachel `i` der Ableitung ist Textur `i` des Pakets; Name, Index, Größe und Palette wie
`textures_only`, die Vorgaben aus den Vanilla-Permissions der Karten, die das Paket zeigen.

**Karten eines Pakets** (`places::maps_of_packages`, ein Durchgang über Header und Matrizen): ein
Header zeigt die Karten seiner Matrix in den Zellen, die seine sind (jede Zelle einer Matrix ohne
Header-Tabelle); seine Gebietsdaten nennen das Paket. Eine Karte kann so zu mehreren Paketen
gehören. Eine Karte, die nicht lädt, zählt nicht; eine, deren Modell sich nicht lesen lässt, bindet
keine Textur.

**Regel K1 (Mehrheit), `UNIQUE_SHARE = 0,6`** (Task 0, [task0-6-3a-vorgaben.md](../research/2026-11-05-phase-6/task0-6-3a-vorgaben.md)
§8 „Festlegung“, `DEFAULT_RULE = VoteRule::K1`): für Textur T sind M_T die Karten des Pakets, deren
Modell T bindet (`material_texture_names`). Jede Zelle einer solchen Karte stimmt mit dem Gewicht
`1 / |Namen der Karte|` für ihr `kind`: `votes(k) = Σ_{m ∈ M_T} Zellen_m(k) / |Namen_m|`. `k*` ist
das `kind` mit den meisten Stimmen (Gleichstand: das kleinere); entschieden, wenn
`votes(k*) / Σ votes ≥ 0,6`. Die zweite gemessene Regel K2 (Abhebung gegen alle Zellen des Pakets)
liegt im Code, ist aber nicht gewählt.

| Ergebnis | `kind` | `collision` | `sound` (HGSS) | `role` | `origin` |
|---|---|---|---|---|---|
| entschieden | `k*` | 0 | häufigstes `collision & 0x7F` der gewichteten Zellen mit `k*` (Gleichstand: das kleinere) | aus `k*` | `vanilla` |
| unklar oder ohne Karte | 0 | 0 | 0 | `ground` | `unclear` |

**Kollision immer 0**, abweichend vom Plan (dort `0x80`, wenn mindestens die Hälfte der gewichteten
Zellen mit `k*` Bit 7 trägt): die Planregel stimmte in Task 0 seltener als „überall 0“ (44,5 % und
64,0 % gegen 78,7 % und 77,2 % der Texturen mit Wahrheit, Platin DE und HeartGold DE), und eine
blockierte Vorgabe machte gemalten Boden unbegehbar. Das Geräusch von HeartGold/SoulSilver folgt
dem Plan.

**Rolle aus `kind`:** die Klasse des Verhaltens (`tiles::tile_behavior(family, kind).class`):
`grass` → `grass`, `water` → `water`, sonst `ground`; `height` immer `flat`. `wall`, `ramp` und
`piece` leitet nichts ab: unklare und abgeleitete Kacheln sind Boden und werden im Tileset-Editor
(6.4a) umgestellt. Mit K1 sind abgeleitete `grass`- und `water`-Kacheln selten: die Zählung ist
karten-weit (wo eine Textur auf der Karte liegt, sieht erst der Display-List-Codec von 6.1), und auf
den meisten Karten überwiegt `kind 0` (Task 0, §8 und §10).

**Palette** einer Kachel: die Palette `<Textur>_pl` (der Name vor dem Vergleich auf 16 Byte
gekürzt), sonst die gleichnamige, sonst die Palette mit dem Index der Textur, sonst keine
(`palette` fehlt). Eine Textur im Format 7 (direkte Farbe) hat keine. Die dritte Stufe hat Task 0
ergänzt: ohne sie blieben 20,4 % (Platin) und 9,1 % (HeartGold) der Texturen ohne Palette.

**Zahlen** (Gegenprobe im Kernel, Task 0 §12): Platin DE 74 Pakete mit 3 130 Kacheln, davon 1 765
`vanilla`; HeartGold DE und US 106 Pakete mit 3 659 Kacheln, davon 2 612 `vanilla`. Die Zahl der
Kacheln je Paket ist immer die Zahl seiner Texturen.

## Texturen für die Szene

`get_tileset_texture(package, texture)` dekodiert die Textur mit der Palette ihrer Kachel
(`tileset_texture`; ohne `palette` Palette 0) zu RGBA. `null` kommt, wenn das Paket oder die Textur
fehlt, sich das Paket nicht lesen lässt (das synthetische Testprojekt), die Kachel eine Palette
nennt, die das Paket nicht hat, oder das Format keinen Decoder hat: Texturen im **Format 5**
(4 × 4 komprimiert) dekodiert der Kernel noch nicht, die Szene zeigt dort den Platzhalter der Rolle
([world_map.md](world_map.md), „Vorschau in der Szene“).
Seit 6.5b dekodiert es auch eine Textur, die keine Kachel nennt (von einem Modell an das Paket
angehängt, nachdem die Tileset-Datei geschrieben wurde), mit der Palette `palette_name(<Textur>)` des
Pakets, sonst Palette 0.

## Synthetisches Tileset

`Tileset::synthetic()` (nur mit `test-support`), Paket `SYNTHETIC_PACKAGE = 0`, alle Kacheln
`origin: synthetic`, `kind`/`collision` 0, wo nicht anders genannt:

| Kachel | `texture` | `size` | `role` | Besonderes |
|---|---|---|---|---|
| 0 | `syn_boden` | 16 × 16 | `ground` | `wall: 3` |
| 1 | `syn_gras` | 32 × 32 | `grass` | `kind 0x02` |
| 2 | `syn_wasser` | 16 × 16 | `water` | `kind 0x15` |
| 3 | `syn_wand` | 16 × 16 | `wall` | `height wall`, `collision 0x80` |
| 4 | `syn_rampe` | 16 × 16 | `ramp` | `height ramp` |
| 5 | `syn_stueck` | 16 × 16 | `piece` | `footprint [2, 2]` |

Kachel 0 als Datei (Ausschnitt von `data/tilesets/0000.json`, die übrigen fünf Kacheln mit „…“
weggelassen):

```json
{
  "version": 1,
  "package": 0,
  "tiles": [
    {
      "texture": "syn_boden",
      "texture_index": 0,
      "size": [
        16,
        16
      ],
      "role": "ground",
      "kind": 0,
      "collision": 0,
      "height": "flat",
      "wall": 3,
      "origin": "synthetic"
    },
    "…"
  ]
}
```

## Kantensätze (6.4b)

Ein **Kantensatz** sagt, welche Kacheln eine gemalte Fläche samt Rändern und Ecken bilden: Der
Terrain-Pinsel und der Klippenpinsel ([world_map.md](world_map.md), „Smart Drawing (6.4b)“) setzen
mit ihm die Kachel jeder Zelle nach den Farben ihrer vier Ecken. Quelle: Plan
[2026-11-07-phase-6-4b-smart-drawing.md](../superpowers/plans/2026-11-07-phase-6-4b-smart-drawing.md)
(„Entscheidungen“), [Gutachten C](../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §3.4.
Implementierung: `crates/g4-kernel/src/world/edge.rs` (Plätze, Masken, Prüfung, `variant`,
`EdgeSetOp`), gelesen und aufgelöst in `world/smart.rs`. Datenmodell nach Tileds Wang-Ecken, Logik
eigenständig; kein Code aus Tiled oder PDSMS.

**Datei.** Je Satz ein Objekt in `edge_sets`, Schlüssel in dieser Reihenfolge:

| Schlüssel | Typ | Inhalt |
|---|---|---|
| `name` | Text | Name des Satzes, im Tileset eindeutig; die Striche nennen ihn (`terrain.set`, `cliff.set`) |
| `outside` | Token | Pflicht: die Kachel der reinen Außenfläche (Maske 0) |
| `block` | `upper` \| `lower`, weggelassen bei `upper` | welche Zelle einer Klippenkante blockiert ([world_map.md](world_map.md), „Klippe“) |
| `places` | Objekt | die 15 Plätze, je eine Liste von Kachel-Token; leere Plätze weggelassen |

Ein Satz hat zwei Farben: **innen** (die gemalte Fläche) und **außen**. Jede Zelle hat vier Ecken
mit den Bits `NW = 1`, `NE = 2`, `SW = 4`, `SE = 8` (gesetzt = innen); ihre **Maske** ist die Summe.
Jede der 15 Masken ≠ 0 hat genau einen Platz, Maske 0 ist `outside`. Die Plätze in der Reihenfolge der
Datei (`Place::ALL`), mit ihrem Namen in der App und in den Meldungen:

| Platz | Maske | Name | liegt |
|---|---|---|---|
| `center` | 15 | Mitte | ganz innen |
| `n` | 12 | Kante N | Kante, Norden außen |
| `e` | 5 | Kante O | Kante, Osten außen |
| `s` | 3 | Kante S | Kante, Süden außen |
| `w` | 10 | Kante W | Kante, Westen außen |
| `outer_nw` | 8 | Ecke außen NW | Außenecke im Nordwesten der Fläche (nur SE innen) |
| `outer_ne` | 4 | Ecke außen NO | Außenecke im Nordosten (nur SW innen) |
| `outer_sw` | 2 | Ecke außen SW | Außenecke im Südwesten (nur NE innen) |
| `outer_se` | 1 | Ecke außen SO | Außenecke im Südosten (nur NW innen) |
| `inner_nw` | 14 | Ecke innen NW | Innenecke, die Ecke NW liegt außen |
| `inner_ne` | 13 | Ecke innen NO | Innenecke, die Ecke NE liegt außen |
| `inner_sw` | 11 | Ecke innen SW | Innenecke, die Ecke SW liegt außen |
| `inner_se` | 7 | Ecke innen SO | Innenecke, die Ecke SE liegt außen |
| `diag_nwse` | 9 | Diagonale NW–SO | NW und SE innen |
| `diag_nesw` | 6 | Diagonale NO–SW | NE und SW innen |

Die ersten 13 (`Place::GATE13`) sind die Plätze von PDSMS; eine Fläche aus Rechtecken braucht nur
sie, die Diagonalen erst zwei Flächen, die sich an einer Ecke berühren.

- **Token:** jeder Eintrag ist ein vollständiges Kachel-Token wie im Weltdokument, mit Drehung und
  Spiegelung (`"0007"`, `"0007r1"`, `"0007f"`); eine Kachel darf gedreht mehrere Plätze füllen.
  **Dasselbe Token** (nach `parse_tile` verglichen, Groß- und Kleinschreibung zählt nicht) steht im
  Satz nur einmal, auch nicht zugleich als `outside`: die Rückwärtssuche Token → Platz muss eindeutig
  sein.
- **Unvollständige Sätze** lassen sich speichern; erst ein Strich, der einen leeren Platz bräuchte,
  lehnt ab („Kantensatz „Weg“: der Platz „Ecke innen NW“ hat keine Kachel (Zelle (6, 7)).“), und
  nichts wird geschrieben.
- **Eckfarben lesen:** Gen-4-Karten speichern keine Eckfarben, die Kacheln tragen sie. Eine Zelle ist
  **Mitglied** des Satzes, wenn ihr Token in einem Platz steht (Maske des Platzes), **außen**, wenn
  es `outside` ist (Maske 0), sonst **fremd** (sie liest nichts). Eine Ecke ist innen, sobald
  mindestens eine der bis zu vier angrenzenden lesbaren Zellen sie innen meldet.
- **Varianten:** hat ein Platz mehrere Einträge, wählt der Kernel für Zelle (x, z) den Eintrag
  `variant(seed, z · 32 + x, n)`, gleich wahrscheinlich: `splitmix64((seed << 32) | Zelle) % n` mit
  den Konstanten `0x9E3779B97F4A7C15`, `0xBF58476D1CE4E5B9`, `0x94D049BB133111EB` und den
  Verschiebungen 30, 27, 31. `seed` steht im Strich (Vorgabe 0); derselbe Strich mit demselben Samen
  gibt dieselben Kacheln. Gewichte gibt es nicht.
- **Wände der Klippe:** keine eigene Regel. Das Backen zieht die Wand an einer Höhenkante wie seit 6.3a
  mit der `wall` der oberen Kachel ([world_map.md](world_map.md), „Vernetzer“). Oben liegen meist
  Kacheln der Plätze, aber nicht immer: eine Klippe von einer Zelle Breite (ein einzelner Zug mit K)
  hat keinen Rand, ihre Ecken berühren alle eine tiefere Zelle, oben bleibt oder wird `outside`. Der
  Editor setzt mit „Wand“ darum die `wall` **aller Kacheln des Satzes**: der 15 Plätze und von
  `outside` (Hinweis „Für jede Kachel des Satzes, auch Außen“). Gehört die Außenkachel zu mehreren
  Sätzen oder liegt sie oben an einer anderen Höhenkante, bekommt sie dort dieselbe Wand; die
  Änderung ist ein Schritt und lässt sich zurücknehmen.

**Kantensätze ändern** (`tileset_edit` mit `sets`). `sets` ist eine Liste von Operationen
(`EdgeSetOp`, Vorgabe leer), angewandt der Reihe nach vor den Kachel-Änderungen, alles zusammen
**ein** Dateischritt wie oben:

- `{"put": {name, outside, block?, places}}` ersetzt den Satz gleichen Namens an seiner Stelle oder
  hängt ihn an;
- `{"remove": {"name": "Weg"}}` entfernt ihn; einen Satz, den es nicht gibt: „Kantensatz „Weg“ gibt es
  im Tileset 0006 nicht.“

Mit `sets` heißt der Schritt nach der **ersten** Operation „Kantensatz „Weg“ angelegt: Tileset 0006“,
„Kantensatz „Weg“ geändert: Tileset 0006“ oder „Kantensatz „Weg“ entfernt: Tileset 0006“, auch
wenn `changes` zugleich Kacheln ändert; `origin: user` bekommen nur die Kacheln, deren Werte sich
ändern, und `tiles` der Antwort zählt nur sie (bei reinen Satz-Operationen 0). Ein `put`, das den
Satz nicht ändert, ist „Nichts zu ändern.“; ein Satz mit einem Problem der Prüfungen oben wird mit
dem Pfad der Datei abgelehnt („data/tilesets/0000.json: edge_sets[0].places.e[0]: Kachel 0007 steht
zweimal im Satz („n“ und „e“).“). Danach backen die sauberen Welt-Karten des Pakets neu wie nach jedem
`tileset_edit`; ihr Bake ändert sich durch einen Satz allein nicht, erst ein Strich setzt Kacheln.
`tileset_get` gibt die Sätze typisiert unter `tileset.edge_sets` mit aus; über MCP läuft
`tileset_edit` mit `sets` über `g4_call` ([mcp.md](../mcp.md), „Smart Drawing (6.4b)“).

**Editor in der App.** Der Abschnitt „Kantensätze“ (`apps/studio/src/workbench/inspector/EdgeSetEditor.tsx`,
`edgeSets.ts`) steht im Inspector einer Welt-Karte beim Malen nach „Palette“, „Ebenen“ und „Kachel 0029“, vor
„Quelle“: die Sätze mit dem Radio
„Aktiv“ (mit ihm malen T und K) und dem Zähler „13 von 13 Plätzen · Diagonalen 0 von 2“, „Neuer
Kantensatz“ (Name, `outside` = gewählte Palettenkachel), die „Stufen der Klippe“ und der Editor des
aktiven Satzes: das Raster der Plätze (Block 3 × 3 `outer_nw n outer_ne / w center e / outer_sw s
outer_se`, Block 2 × 2 der Innenecken, Zeile der Diagonalen), ein Platz per Klick gewählt, dann hängt
ein Klick auf eine Palettenkachel ihr Token (mit der Drehung von Z und X) an; die Einträge des Platzes
als Chips mit „Entfernen“, „Außen“ („Aktive Kachel übernehmen“), „Blockiert an der Klippe“ (Segment
„Obere Zelle“/„Untere Zelle“ = `block`), „Wand“ und „Satz entfernen“. Jede bestätigte Änderung ist
**ein** `tileset_edit` mit `sets: [{put}]` bzw. `[{remove}]`, „Wand“ dazu mit den `changes` aller
Kacheln des Satzes (Plätze und `outside`). Ohne Datei (`stored: false`) ist alles gesperrt, darüber „Tileset im Projekt anlegen“. Die
Gestaltung steht in [design-system/README.md](../../design-system/README.md), „Nachtrag 6.4b“.

**Synthetische Sätze.** `Tileset::synthetic_edges()` (nur mit `test-support`) ist `synthetic()` plus
16 Bodenkacheln 6 bis 21 (`syn_kante_00` bis `syn_kante_15`, 16 × 16, `origin: synthetic`) und der
Satz `weg13` darauf; `synthetic()` selbst bleibt bei sechs Kacheln. Die Sätze stehen in
`world::edge::fixtures`:

| Platz | `weg13` („Weg“) | `weg15` („Weg15“) |
|---|---|---|
| `outside` | `0000` | `0000` |
| `center` | `0006` | `0006`, `0015` |
| `n`, `e`, `s`, `w` | `0007`, `0008`, `0009`, `000a` | wie `weg13` |
| `outer_nw`, `outer_ne`, `outer_sw`, `outer_se` | `000b`, `000c`, `000d`, `000e` | wie `weg13` |
| `inner_nw`, `inner_ne`, `inner_sw`, `inner_se` | `000f`, `0010`, `0011`, `0012` | wie `weg13` |
| `diag_nwse`, `diag_nesw` | – | `0013`, `0014` |

`weg13` als Datei (Ausschnitt von `edge_sets` in `data/tilesets/0000.json`; `block` fehlt, denn er ist
`upper`, die Plätze nach `n` mit „…“ weggelassen):

```json
"edge_sets": [
  {
    "name": "Weg",
    "outside": "0000",
    "places": {
      "center": [
        "0006"
      ],
      "n": [
        "0007"
      ],
      "…": "…"
    }
  }
]
```
