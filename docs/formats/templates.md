# Weltvorlagen (`templates.yaml`, `metadata/templates/`)

Eine Tatsache über Gen4 Studio selbst, keine Spielformat-Quelle: Weltvorlagen sind Rezepte, die eine
Vanilla-Welt umbauen. Heute gibt es eine, die **Leere Region mit Startstadt** (`empty_region`).
Implementierung: `crates/g4-kernel/src/templates.rs` (`region_recipe`, `plan_empty_region`,
`apply_empty_region`, `start_script`, `wrap_message`); Rezept in
`crates/g4-kernel/metadata/templates.yaml`, Start-Scripts in
`crates/g4-kernel/metadata/templates/start_pt.rotom` und `start_hgss.rotom`. Diamant/Perl lehnt der
Kernel ab („Diamant/Perl: ohne Testprojekt nicht freigegeben“).

## Rezept (`templates.yaml`)

| Feld | Platin | HeartGold/SoulSilver | Inhalt |
|---|---|---|---|
| `start_town` | `T01` | `T20` | interner Name (`mapname.bin`) des Headers der Startstadt; er muss `matrix_id 0` haben |
| `starter.species`, `starter.level` | 387, 5 | 152, 5 | das Pokémon, das das Start-Script gibt (Chelast, Endivie) |
| `done_var` | `VAR_UNUSED_0x406C` | `VAR_UNK_4031` | Variable (Name der Befehlsdatenbank), die das Start-Script auf 1 setzt; kein übernommenes Vanilla-Script benutzt sie |
| `message.de`, `message.en` | | | Text des Start-Scripts: deutsche Spiele `de`, alle anderen `en` |

Orte, Variablen und Flags stehen als Namen im Rezept; die Nummern rechnet der Kernel aus der
mitgelieferten Befehlsdatenbank (`metadata/scrcmd/`).

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

## Anwenden (`apply_empty_region`)

Vorbedingungen, in dieser Reihenfolge und jede vor der ersten Plattenänderung:

1. Familie Platin oder HeartGold/SoulSilver (sonst `Refused`, Diamant/Perl-Satz);
2. der Plan sagt nicht `already_empty` (sonst `Refused("the world is already empty")`);
3. die Scripts sind übernommen, `rotom.toml` existiert (sonst `Refused("scripts are not adopted")`);
4. der Store hat keine ungespeicherten Änderungen (sonst `Refused("save first")`).

Danach liest der Kernel noch die Quelle der Scriptdatei der Stadt (sie muss eine rotom-Quelle sein)
und baut daraus Start-Script und Level-Script, bevor er schreibt.

**Sofort auf der Platte, nicht rückgängig** (nach ⌘Z unverwiesen):

1. Rand-Karte: Kopie der Stadtkarte ohne Gebäude (`Store::duplicate_map_with(town_map, true)`), als
   neues letztes Member des Karten-Archivs.
2. Level-Script: neue Scriptdatei (`new_script_file(LevelScript)`) mit
   `{"header_entries": [{"type": "on_var_condition"}], "var_conditions": [{"var": done_var, "value": 0, "script_id": n}]}`,
   übersetzt. Das alte Level-Script der Stadt (Platin: `on_transition #1`) entfällt.
3. Start-Script: die Vorlage der Familie als Script `#n` (n = größte Scriptnummer der Datei + 1) an
   die Scriptdatei der Stadt angehängt und übersetzt (bei einem Übersetzungsfehler kommt der alte
   Quelltext zurück). Der Text ist ein Literal und landet als neue Nachricht im Textarchiv des ersten
   gespeicherten Headers mit dieser Scriptdatei; der Store lädt die berührten Textarchive neu.

**Ein Store-Schritt** `Store::transaction("Vorlage: Leere Region mit Startstadt")`, jedes Asset
einmal; ⌘Z holt die Welt zurück:

| Asset | Änderung |
|---|---|
| Header-Tabelle (`Tx::edit_headers`) | geleerte Header: `event_file_id 0`, `level_script_id` = leeres Level-Script; die Stadt: das neue Level-Script. Scriptdatei, Text, Begegnungen, Musik und alle übrigen IDs bleiben |
| Matrix 0 | jede Rand-Zelle: Rand-Karte, Header der Stadt, Höhe der ersten Stadtzelle; leere Zellen bleiben leer |
| Rand-Karte (`Tx::edit_map`) | alle 1024 Kollisionszellen `{kind 0, collision 0x80}` |
| Eventdateien der behaltenen Header | die Stadt ohne Figuren und Auslöser (Warps und Schilder bleiben); wegfallende Warps entfernt; Warps in eine Datei, die Warps verlor, auf die neuen Nummern umgezählt |

`RegionReport` nennt Plan, Rand-Karte, Level-Script, Start-Script (`(Datei, Nummer)`), den Schritt und
die geänderten Assets (Header-Tabelle, Matrix 0, Rand-Karte, Eventdateien, die beiden Scriptdateien,
die berührten Textarchive). Gespeichert wird nicht; das machen `g4k template` und `create_project`.

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
