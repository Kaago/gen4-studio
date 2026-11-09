# Weltvorlagen (`templates.yaml`, `metadata/templates/`)

**In English:** Documents world templates, recipes that rebuild a vanilla world; this is about Gen4
Studio itself. There is one today, "Empty region with a start town" (`empty_region`), which turns
the vanilla region into an empty one around a start town and adds a start script. The document
describes the recipe file, its fields per game family, the preconditions and the start script;
Diamond/Pearl is refused. The code is `crates/g4-kernel/src/templates.rs` and
`crates/g4-kernel/src/recipes/empty_region.rs`, with the recipe in
`crates/g4-kernel/metadata/templates.yaml` and the start scripts in
`crates/g4-kernel/metadata/templates/`.

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Weltvorlagen sind Rezepte, die eine
Vanilla-Welt umbauen. Heute gibt es eine, die **Leere Region mit Startstadt** (`empty_region`).
Implementierung: `crates/g4-kernel/src/templates.rs` (`region_recipe`, `plan_empty_region`,
`start_script`, `wrap_message`) und das Rezept „Welt leeren“ `crates/g4-kernel/src/recipes/empty_region.rs`
(`plan`, `apply`, `apply_empty_region`, das `templates::apply_empty_region` wieder ausführt); Rezept in
`crates/g4-kernel/metadata/templates.yaml`, Start-Scripts in
`crates/g4-kernel/metadata/templates/start_pt.rotom` und `start_hgss.rotom`. Diamant/Perl lehnt der
Kernel ab („Diamant/Perl: ohne Testprojekt nicht freigegeben“).

## Rezept (`templates.yaml`)

| Feld | Platin | HeartGold/SoulSilver | Inhalt |
|---|---|---|---|
| `start_town` | `MAP_T01` | `MAP_T20` | `MAP_` und der interne Name (`mapname.bin`) des Headers der Startstadt; er muss `matrix_id 0` haben. Der bloße interne Name (`T01`, die Form vor 4.3b) gilt auch |
| `starter.species`, `starter.level` | `SPECIES_TURTWIG`, 5 | `SPECIES_CHIKORITA`, 5 | das Pokémon, das das Start-Script gibt (Chelast 387, Endivie 152): ID der festen Tabelle oder Nummer (`387`, die Form vor 4.3b) |
| `done_var` | `VAR_UNUSED_0x406C` | `VAR_UNK_4031` | Variable (Name der Befehlsdatenbank), die das Start-Script auf 1 setzt; kein übernommenes Vanilla-Script benutzt sie |
| `message.de`, `message.en` | | | Text des Start-Scripts: deutsche Spiele `de`, alle anderen `en` |

Orte, Variablen und Flags stehen als Namen im Rezept; die Nummern rechnet der Kernel aus der
mitgelieferten Befehlsdatenbank (`metadata/scrcmd/`). Die Spezies löst `Resolver::fixed()` auf (feste
Tabelle, Fallback `SPECIES_NNNN`, Nummer), so dass das Rezept auch in einem DSPRE-Ordner gilt; eine
unbekannte ID ist ein Fehler der Metadaten (`templates.yaml: Unbekannte ID: …`). Die Startstadt bleibt
im Kernel der interne Name (`RegionRecipe::start_town`, `templates::start_town` geben `T01` bzw. `T20`):
`MAP_<interner Name>` heißt der Header auch ohne Textprojekt und nach einem Umbenennen im Register.

## Plan (`plan_empty_region`)

Liest nur (auch ungespeicherte Änderungen des Stores) und liefert `RegionPlan`:

- **Stadt**: der Header mit dem internen Namen aus dem Rezept und `matrix_id 0`; ihre Zellen sind die
  Zellen von Matrix 0 mit ihrem Header und einer Karte, außer Zellen mit einer Rand-Karte (alle 1024
  Kollisionszellen `{kind 0, collision 0x80}`; eine leere oder kürzere Liste ist keine Rand-Karte).
  So findet ein zweiter Plan nach dem Rezept die echte Stadt wieder.
- **Behalten** (`kept`): die Stadt und jeder Header, den man von ihr über Warps erreicht, ohne einen
  anderen Header mit `matrix_id 0` zu betreten (Innenräume, Spielerhaus).
- **Geleert** (`emptied`): jeder andere Header, der nicht schon Eventdatei 0 und das leere
  Level-Script hat. Das leere Level-Script ist das von Header 0, wenn es genau vier Nullbytes hat,
  sonst das erste solche Member; ohne eines lehnt der Plan ab.
- **Rand-Zellen** (`border_cells`): jede Zelle von Matrix 0 mit Karte (≠ `0xFFFF`) außerhalb der Stadt.
- **Wegfallende Warps** (`dropped_warps`): Warps behaltener Eventdateien in einen nicht behaltenen
  Header.
- **Starter** (`starter_species`, `starter_level`): das Pokémon, das das Start-Script gibt (aus dem
  Rezept), damit der Assistent es nennen kann.
- `already_empty`: jede Zelle mit Karte trägt den Header der Stadt, die Zellen außerhalb teilen eine
  Karte, die Stadt hat keine Figuren. Das sieht nur Matrix 0 und die Figuren der Stadt an, nicht die
  Header.

Gemessen an den Vanilla-Projekten: Platin DE Zweiblattdorf = Header 411, Zelle (3, 27), behalten
411 bis 417, 536 Header geleert, 467 Rand-Zellen; HeartGold DE Neuborkia = Header 60, Zelle (21, 12),
behalten 60 bis 66 und 384, 482 Header geleert, 491 Rand-Zellen. Keine wegfallenden Warps.

## Anwenden (`apply_empty_region`, Rezept `empty_region`)

`recipes::empty_region::apply` (über `recipes::apply` mit `{"recipe": "empty_region"}`, den Weg
von `recipe_apply` und `g4k recipe apply`) und `templates::apply_empty_region` (= `apply(…).report`, für `apply_empty_region`, `g4k template` und
`create_project`) tun dasselbe. `recipes::empty_region::plan` macht dieselben Prüfungen und schreibt
nichts; sein Entwurf nennt die Rand-Zellen von Matrix 0 (`cells`, je `source_map` = Karte der Stadt,
`count(0)` = `border_cells`), die Nachricht des Start-Scripts in der Sprache des Spiels mit ihrem
Textarchiv (`messages`), Sätze aus dem Plan (`notes`) und die Rand-Karte (`irreversible`).

Vorbedingungen, in dieser Reihenfolge und jede vor der ersten Plattenänderung:

1. Familie Platin oder HeartGold/SoulSilver (sonst `Refused`, Diamant/Perl-Satz);
2. der Plan sagt nicht `already_empty` (sonst `Refused("the world is already empty")`);
3. die Scripts sind übernommen, `rotom.toml` existiert (sonst `Refused("scripts are not adopted")`);
4. der Store hat keine ungespeicherten Änderungen (sonst `Refused("save first")`).

Danach liest der Kernel noch die Quelle der Scriptdatei der Stadt (sie muss eine rotom-Quelle sein)
und baut daraus Start-Script und Level-Script, bevor er schreibt. Vor dem ersten Schreiben sichert er
die Dateien, die der Schritt zurücknehmen kann: Quelle (`.rotom`/`.json`) und Member der Scriptdatei
der Stadt und der künftigen Level-Script-Datei (nichts da), JSON und Member der Textarchive, in die die
Übersetzung schreiben kann (`town_archives`: das des Start-Headers und das jedes Headers mit der
Scriptdatei der Stadt), und rotoms `.rotom/status/compile-state.json` als ganze Datei.

**Sofort auf der Platte** (die Rand-Karte bleibt nach ⌘Z als unverwiesenes Member, alles Übrige nimmt
der Schritt zurück):

1. Rand-Karte: Kopie der Stadtkarte ohne Gebäude (`Store::duplicate_map_with(town_map, true)`), als
   neues letztes Member des Karten-Archivs. Nicht rückgängig (`Outcome::created`).
2. Level-Script: neue Scriptdatei (`new_script_file(LevelScript)`) mit
   `{"header_entries": [{"type": "on_var_condition"}], "var_conditions": [{"var": done_var, "value": 0, "script_id": n}]}`,
   übersetzt. Das alte Level-Script der Stadt (Platin: `on_transition #1`) entfällt.
3. Start-Script: die Vorlage der Familie als Script `#n` (n = größte Scriptnummer der Datei + 1) an
   die Scriptdatei der Stadt angehängt und übersetzt (bei einem Übersetzungsfehler kommt der alte
   Quelltext zurück). Der Text ist ein Literal und landet als neue Nachricht im Textarchiv des ersten
   gespeicherten Headers mit dieser Scriptdatei; der Store lädt die berührten Textarchive neu.

**Ein Undo-Schritt** „Vorlage: Leere Region mit Startstadt“: die Store-Transaktion (jedes Asset
einmal) und danach die Dateischritte (`FileStep`, Art `Recipe`), mit `Store::squash_since` zu einem
Schritt verschmolzen; ⌘Z holt die Welt und die Dateien zurück, ⌘⇧Z bringt beides wieder:

| Asset | Änderung |
|---|---|
| Header-Tabelle (`Tx::edit_headers`) | geleerte Header: `event_file_id 0`, `level_script_id` = leeres Level-Script; die Stadt: das neue Level-Script. Scriptdatei, Text, Begegnungen, Musik und alle übrigen IDs bleiben |
| Matrix 0 | jede Rand-Zelle: Rand-Karte, Header der Stadt, Höhe der ersten Stadtzelle; leere Zellen bleiben leer |
| Rand-Karte (`Tx::edit_map`) | alle 1024 Kollisionszellen `{kind 0, collision 0x80}` |
| Eventdateien der behaltenen Header | die Stadt ohne Figuren und Auslöser (Warps und Schilder bleiben); wegfallende Warps entfernt; Warps in eine Datei, die Warps verlor, auf die neuen Nummern umgezählt |
| Level-Script-Datei (Dateischritt `script_file_step`) | Quelle `NNNN.json` und Member entstehen; ⌘Z entfernt sie |
| Scriptdatei der Stadt (Dateischritt) | Quelle `NNNN.rotom` und Member wie vor dem Anhängen |
| berührte Textarchive (Dateischritt `archive_file_step`) | `expanded/textArchives/NNNN.json` und Member wie vorher; ⌘Z und ⌘⇧Z laden ein geladenes Archiv danach neu (`reload_from_disk`) |

Ein Textarchiv, das die Übersetzung berührt, ohne in `town_archives` zu stehen, bekommt als `before`
die geladene Kopie des Stores (der Stand vor der Übersetzung; sein Member behält, was die Übersetzung
schrieb, das zurückgelegte JSON gilt und der nächste Build kodiert es); ist es nicht geladen, nimmt ⌘Z
es nicht zurück. Auf Platin DE und HeartGold DE kommt das nicht vor (Test
`vanilla_empty_region_undo_restores_the_world`).

`compile-state.json` geht als ganze Datei zurück (Dateischritt `compile_state_step`, Asset die
Scriptdatei der Stadt): die Übersetzung schreibt die Datei ganz neu, die Einträge in der Reihenfolge
von rotoms `HashMap`, und markiert jede Quelle, die sie neu übersetzt, als „Compiled“ (auch eine
übernommene „Decompiled“ mit gleichem Member). Einträge einzeln zurückgesetzt (`set_state_entry`)
gäben die Datei in anderer Reihenfolge wieder; im DSPRE-Layout ist sie versioniert, so dass `git
status` nach ⌘Z nur die Rand-Karte nennt. Darum lehnt ⌘Z auch ab, wenn seit dem Schritt irgendeine
Übersetzung lief (etwa im Script-Editor), und nennt dann die Scriptdatei der Stadt.

Hat jemand eine der Dateien seit dem Schritt geändert (von Hand, im Script-Editor), lehnt ⌘Z mit
`Refused` („Scriptdatei NNNN wurde nach dem Rezept geändert; ⌘Z nimmt den Schritt nicht zurück, er
fällt aus dem Verlauf.“) ab, schreibt nichts, und der Schritt fällt aus dem Verlauf.

`RegionReport` nennt Plan, Rand-Karte, Level-Script, Start-Script (`(Datei, Nummer)`), den Schritt und
die geänderten Assets (Header-Tabelle, Matrix 0, Rand-Karte, Eventdateien, die beiden Scriptdateien,
die berührten Textarchive). Das Rezept antwortet `Outcome` mit diesem Bericht, `created` = die
Rand-Karte und `next` = „Gebiet anlegen“ (`create_area`). Gespeichert wird nicht; das machen
`g4k template` und `create_project`.

## Start-Script

Platzhalter: `{n}`, `{species}`, `{level}`, `{done_var}`, `{message}` (`start_script`; `{message}`
zuletzt, ein `"` darin wird zu `\"`). Jeder Befehl ist gegen die mitgelieferte Befehlsdatenbank der
Familie geprüft (Name und Parameterzahl):

| Platin (`start_pt.rotom`) | HeartGold/SoulSilver (`start_hgss.rotom`) |
|---|---|
| `LockAll` | `LockAll` |
| `Message "{message}"` | `NonNPCMsg "{message}"` (Befehl 44 wie Platins `Message`) |
| `WaitButton` | `WaitButton` |
| `GivePokemon {species}, {level}, 0, VAR_RESULT` | `GiveMon {species}, {level}, 0, 0, 0, VAR_SPECIAL_RESULT` |
| `GivePokedex` | `GivePokedex` |
| `SetFlag FLAG_HAS_POKEDEX` | `SetFlag FLAG_GOT_POKEDEX`, `SetFlag FLAG_GOT_STARTER` |
| `GiveRunningShoes` | `GiveRunningShoes` |
| `SetVarFromValue VAR_PLAYER_STARTER, {species}` | `SetVar VAR_PLAYER_STARTER, {species}` |
| `SetVarFromValue {done_var}, 1` | `SetVar {done_var}, 1` |
| `CloseMessage` | `CloseMsg` |
| `ReleaseAll`, `End` | `ReleaseAll`, `End` |

Die HeartGold-Datenbank kennt `Message`, `SetVarFromValue` und `CloseMessage` nicht (`Message` ist
dort der Altname von `NPCMsg`, Befehl 45); die Spalte rechts benutzt ihre Namen. Platin zeigt im
Startmenü „POKéMON“ nur bei gesetztem `VAR_PLAYER_STARTER`; HeartGold prüft `FLAG_GOT_STARTER` und
`FLAG_GOT_POKEDEX` (pret `pokeplatinum`, `pokeheartgold`).

Der Text wird vorher mit chatots `word_wrap` auf die Breite des Textfensters umbrochen
(`dialog_line_px`, 216 px; `wrap_message`), wie die Behebung von `text.too-wide`; das Spiel bricht
keine Zeile selbst um. Der deutsche Text wird zu
`Willkommen in deiner leeren Region! Hier\nist dein erstes Pokémon.`

## Wege

- `g4k template <projekt> empty-region`: übernimmt die Scripts, wenn `rotom.toml` fehlt, wendet an,
  speichert und druckt den Bericht.
- `create_project(.., Template::EmptyRegion, ..)` und `g4k create … --template empty-region`: Phase
  `template` = Scripts übernehmen (Fortschritt `done/total` über die Member, `total` = Member + 1),
  dann ein Schritt für das Rezept und `Store::save`; die Marke trägt `template: "empty_region"`
  ([project_base.md](project_base.md)). Scheitert das Rezept, löscht der Kernel nur den Staging-Ordner.
