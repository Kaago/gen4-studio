# AreaData

**In English:** Documents the AreaData record: one 8-byte entry per area, a member of
`area_data.narc` (Diamond/Pearl, Platinum) or of `a/0/4/2` (HeartGold/SoulSilver). An entry points
to the building texture set and the map texture set (tilesets) that the area uses and holds a light
type. The layout is given per game family with offsets and types, all values little endian. The code
is `crates/g4-formats/src/area_data.rs`, which was verified byte for byte against the three vanilla
test projects (Platinum DE, HeartGold DE and US).

Quelle: Das Layout folgt dem Plan bzw. DSPRE `AreaData.cs` in der dort
dokumentierten Form; die Implementierung wurde gegen alle drei
Vanilla-Projekte (Platin DE, HeartGold DE und US) byte-identisch verifiziert.
Implementierung:
`crates/g4-formats/src/area_data.rs`. Jedes Member von `area_data.narc` (Pt,
DP) bzw. `a/0/4/2` (HGSS) ist ein Eintrag von genau 8 Bytes. Alle Werte
Little Endian. AreaData verweist auf die Tilesets (Gebäude- und
Karten-Texturen), die ein Bereich benutzt.

## Layout

DP und Platin (`AreaData::Dppt`):

| Offset | Feld | Typ |
|--------|------|-----|
| 0x00 | `buildings_tileset` | `u16` |
| 0x02 | `map_tileset` | `u16` |
| 0x04 | `unknown` | `u16`, verbatim |
| 0x06 | `light_type` | `u16` |

HeartGold / SoulSilver (`AreaData::Hgss`):

| Offset | Feld | Typ |
|--------|------|-----|
| 0x00 | `buildings_tileset` | `u16` |
| 0x02 | `map_tileset` | `u16` |
| 0x04 | `dynamic_texture_type` | `u16` |
| 0x06 | `area_type` | `u8` |
| 0x07 | `light_type` | `u8` |

`AreaData::read` wählt das Layout über die `Family` (`Hgss` gegen `Dp` und
`Pt`). Die Eingabe muss genau 8 Bytes lang sein, sonst
`BadSize { expected: 8, found }`. Da jedes Feld gespeichert wird, ist der
Round Trip für jede 8-Byte-Eingabe byte-identisch.

## Beobachtungen an den Vanilla-Daten

Geprüft an Platin DE (75 Einträge), HeartGold DE und US (je 106 Einträge):
alle byte-identisch round-trippbar.
