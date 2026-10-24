# Begegnungen (`encounters/NNNN`)

Quelle: pret `pokeplatinum` (`WildEncounters`), pret `pokeheartgold`
(`EncounterData`), DSPRE `ROMFiles/EncounterFile.cs`; Recherche
`docs/research/2026-10-23-3b3/dspre-report.md` §1.1 (Platin) und §1.2
(HeartGold). Geprüft an den drei Vanilla-Projekten: Platin DE 183 Member zu
424 Bytes, HeartGold DE und US je 142 Member zu 196 Bytes. Dort sind
Padding und der ungenutzte Block überall 0, die Level-Wörter und die
Wasser-Slots haben keine gesetzten Bytes außerhalb der benannten Felder.
Implementierung: `crates/g4-formats/src/data/encounter.rs` (`PtEncounters`,
`HgssEncounters`, `Encounters`, `WaterSlot`, `Slot`, `DualSlot`, `Method`,
`MethodView`, `SlotView`). Alle Werte Little Endian.

Diamant und Perl lesen mit dem Platin-Layout (`Encounters::read(bytes,
Family::Dp)`); DSPRE nutzt dafür dieselbe Klasse. Ein Diamant-Projekt lag
nicht vor, die Gleichheit der Layouts ist deshalb nicht an Daten geprüft.

## Platin (424 Bytes, `PtEncounters`)

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0x000 | u32 | `grass_rate` | 0 = auf diesem Gras nie eine Begegnung |
| 0x004 | 12 × 8 | `grass_NN_level`, `grass_NN_species` | je Slot `{u32 level, u32 species}` |
| 0x064 | 2 × u32 | `swarm_0`, `swarm_1` | ersetzen Gras-Slots 0, 1 |
| 0x06C | 2 × u32 | `day_0`, `day_1` | ersetzen Gras-Slots 2, 3 tagsüber |
| 0x074 | 2 × u32 | `night_0`, `night_1` | ersetzen Gras-Slots 2, 3 nachts |
| 0x07C | 4 × u32 | `radar_0` bis `radar_3` | ersetzen Gras-Slots 4, 5, 10, 11 |
| 0x08C | 5 × u32 | `form_rate_0` bis `form_rate_4` | [0] Shellos, [1] Gastrodon: 0 = West, sonst Ost (Vanilla schreibt 100), [2] bis [4] ungenutzt |
| 0x0A0 | u32 | `unown_table` | 0 bis 8, Auswahl der Icognito-Formen |
| 0x0A4 | 5 × 2 × u32 | `dual_ruby_N`, `dual_sapphire_N`, `dual_emerald_N`, `dual_firered_N`, `dual_leafgreen_N` | je Modul zwei Einträge, ersetzen Gras-Slots 8, 9 (nur mit nationalem Dex) |
| 0x0CC | u32 + 5 × 8 | `surf_rate`, `surf_N_*` | Wasser-Slots, siehe unten |
| 0x0F8 | 44 Bytes | `pad_f8` | ungenutzt (Rock Smash, im Original auskommentiert); in Vanilla 0 |
| 0x124 | u32 + 5 × 8 | `old_rod_rate`, `old_rod_N_*` | Alte Angel |
| 0x150 | u32 + 5 × 8 | `good_rod_rate`, `good_rod_N_*` | Profi-Angel |
| 0x17C | u32 + 5 × 8 | `super_rod_rate`, `super_rod_N_*` | Super-Angel, Ende bei 0x1A8 |

Gras-Slot: das Level steht im niedrigen Byte des ersten Worts, die Art in den
unteren 16 Bits des zweiten (Vanilla: alle anderen Bytes 0). `Slot { level,
species }` und die Ersatzlisten (`swarm`, `day`, `night`, `radar`, `dual`)
liefern die unteren 16 Bits; die Setter schreiben nur diese und lassen die
oberen Bytes stehen. Hacks im Stil von hg-engine legen in das obere Wort die
Form; sie bleibt so erhalten.

Wasser-Slot (8 Bytes): `u8 max`, `u8 min`, `u16 pad`, `u32 species`. **max vor
min** (in Platin DE ist Byte 0 in 1078 Slots größer als Byte 1 und in keinem kleiner).
Das Padding und das obere Art-Wort bleiben beim Setzen erhalten.

Rate je Methode: ein `u32` vor den Slots (DSPRE castet auf `u8` und schreibt
`u32`; der Codec kennt den vollen Wert).

## HeartGold und SoulSilver (196 Bytes, `HgssEncounters`)

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0x00 | 6 × u8 | `grass_rate`, `surf_rate`, `rock_rate`, `old_rod_rate`, `good_rod_rate`, `super_rod_rate` | Raten je Methode |
| 0x06 | 2 Bytes | `pad_06` | Padding, in Vanilla 0 |
| 0x08 | 12 × u8 | `grass_level_NN` | ein Level je Slot, gemeinsam für Morgen, Tag und Nacht |
| 0x14 | 12 × u16 | `grass_morning_NN` | Arten morgens |
| 0x2C | 12 × u16 | `grass_day_NN` | Arten tagsüber |
| 0x44 | 12 × u16 | `grass_night_NN` | Arten nachts |
| 0x5C | 4 × u16 | `radio_0` bis `radio_3` | 0, 1 Hoenn-Sound, 2, 3 Sinnoh-Sound |
| 0x64 | 5 × 4 | `surf_N_*` | Wasser-Slots |
| 0x78 | 2 × 4 | `rock_N_*` | Rock Smash |
| 0x80 | 5 × 4 | `old_rod_N_*` | Alte Angel |
| 0x94 | 5 × 4 | `good_rod_N_*` | Profi-Angel |
| 0xA8 | 5 × 4 | `super_rod_N_*` | Super-Angel |
| 0xBC | u16 | `land_swarm` | Schwarm im Gras |
| 0xBE | u16 | `surf_swarm` | Schwarm beim Surfen |
| 0xC0 | u16 | `night_fish` | kein Schwarm: die Nacht-Art der Angeln |
| 0xC2 | u16 | `fish_swarm` | Schwarm beim Angeln |

Wasser-Slot (4 Bytes): `u8 min`, `u8 max`, `u16 species`. **min vor max**,
also umgekehrt zu Platin (in beiden HeartGold-Projekten ist Byte 0 in 244 Slots kleiner als
Byte 1 und in keinem größer). `WaterSlot::min()` und
`max()` verbergen die Reihenfolge: die Oberfläche sieht nie Bytes.

Die vier Einträge ab 0xBC sind je ein einzelner `u16`; `night_fish` heißt in
pret `nightFish` und ist der Nachtangel-Slot, kein Schwarm.

## Methodenansicht

`Encounters::methods()` liefert für beide Familien dieselbe Form:
`MethodView { method, rate, slots: Vec<SlotView { index, species, min, max }> }`.

| Familie | Methoden und Slot-Zahlen |
|---------|--------------------------|
| Platin (und DP) | Grass 12, Surf 5, OldRod 5, GoodRod 5, SuperRod 5 |
| HeartGold | Grass 12, Surf 5, RockSmash 2, OldRod 5, GoodRod 5, SuperRod 5 |

Gras hat ein Level je Slot, also `min == max`. Die HeartGold-Gras-Ansicht zeigt
die Tages-Liste; morgens und nachts stehen nur am Record. Ersatzlisten (Schwarm,
Radar, Radio, Dual-Slot, Nachtangeln) gehören nicht zur Ansicht. Die Slot-Anteile
(20, 20, 10, ...) stehen im Code des Spiels, nicht in der Datei.

## Grenzen des Codecs

`read` lehnt nur eine andere Länge als 424 (Platin, DP) beziehungsweise 196
(HeartGold) ab (`BadSize`). Leere Slots (Art 0, Level 0) sind gültig. Alle
unbenannten Bytes (Padding, ungenutzter Block, obere Art-Wörter) bleiben
verbatim, es gilt `write(read(bytes)) == bytes`. Ein Slot-Index außerhalb
seiner Liste ist ein Programmierfehler (Panic), kein Fehlerwert: er träfe sonst
still die Nachbarliste.
