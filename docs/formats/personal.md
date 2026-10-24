# Personal (`personalPokeData/NNNN`)

Quelle: DSPRE `ROMFiles/PokemonPersonalData.cs`, pret `pokeplatinum`
`include/struct_defs/species.h` (`SpeciesData`); Recherche
`docs/research/2026-10-23-3b3/dspre-report.md` §1.4. Die Werte sind an Platin DE
Member 0397 (Staravia) geprüft.
Implementierung: `crates/g4-formats/src/data/personal.rs` (`Personal`).
Alle Werte Little Endian. Ein Member ist genau 44 Bytes.

## Byte-Layout

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0x00 | u8 | `hp` | Basiswert KP |
| 0x01 | u8 | `attack` | Angriff |
| 0x02 | u8 | `defense` | Verteidigung |
| 0x03 | u8 | `speed` | Initiative |
| 0x04 | u8 | `sp_attack` | Spezial-Angriff |
| 0x05 | u8 | `sp_defense` | Spezial-Verteidigung |
| 0x06 | u8 | `type1` | Typ-Index, 0 bis 17 (9 ist der Typ „???“) |
| 0x07 | u8 | `type2` | wie `type1` |
| 0x08 | u8 | `catch_rate` | Fangrate |
| 0x09 | u8 | `base_exp` | Basis-Erfahrung |
| 0x0A | u16 | `ev_yield` | je 2 Bit pro Wert, siehe unten |
| 0x0C | u16 | `item1` | Wildgegenstand 1 (Gegenstands-ID) |
| 0x0E | u16 | `item2` | Wildgegenstand 2 |
| 0x10 | u8 | `gender_ratio` | 0 nur männlich, 254 nur weiblich, 255 geschlechtslos |
| 0x11 | u8 | `egg_cycles` | Brutzyklen |
| 0x12 | u8 | `base_friendship` | Basis-Freundschaft |
| 0x13 | u8 | `growth_rate` | Wachstumskurve (0 bis 5) |
| 0x14 | u8 | `egg_group1` | Ei-Gruppe 1 |
| 0x15 | u8 | `egg_group2` | Ei-Gruppe 2 |
| 0x16 | u8 | `ability1` | Fähigkeit 1 |
| 0x17 | u8 | `ability2` | Fähigkeit 2 |
| 0x18 | u8 | `flee_rate` | Safari-Fluchtrate |
| 0x19 | u8 | `color_flip` | Bits 0 bis 6 `color`, Bit 7 `flip` |
| 0x1A | 2 Bytes | `pad_1a` | Padding, in Vanilla immer 0 |
| 0x1C | 16 Bytes | `tm_bits` | TM/HM-Bits, siehe unten |

## EP-Ausbeute (`ev_yield`)

Das u16 an 0x0A trägt sechs Felder zu je zwei Bits, Wert 0 bis 3, in der
Reihenfolge KP (Bits 0 bis 1), Angriff (2 bis 3), Verteidigung (4 bis 5),
Initiative (6 bis 7), Spezial-Angriff (8 bis 9), Spezial-Verteidigung (10 bis
11). `ev_yield(i)` und `set_ev_yield(i, v)` nehmen `i` in dieser Reihenfolge.
Die Bits 12 bis 15 sind in Vanilla 0 und bleiben beim Setzen unberührt.
Beispiel Staravia: `0x0080`, also Initiative 2.

## TM/HM-Bits (`tm_bits`)

Vier u32 an 0x1C, als 16 Bytes gelesen. Bit `n` liegt in Byte `n / 8`, dort an
Bitstelle `n % 8` (niedrigstes Bit zuerst). `n = 0` ist TM01, `n = 91` TM92,
`n = 92` HM01, `n = 99` HM08; höher gesetzte Bits gibt es in Vanilla nicht,
`tm(n)` und `set_tm(n, ..)` akzeptieren nur `n` unter 100, die Bytes darüber
bleiben unverändert. Die Reihenfolge ist an Staravia belegt: Bit 16 (TM17
Protect), Bit 39 (TM40 Aerial Ace), Bit 93 (HM02 Fly) und Bit 96 (HM05 Defog,
deutsch Auflockern) sind gesetzt, Bit 0 (TM01 Focus Punch) nicht.

## Grenzen des Codecs

`read` lehnt nur eine Länge ungleich 44 ab (`BadSize`); jedes Byte bleibt
verbatim im Datensatz, daher gilt `write(read(bytes)) == bytes`. Die Methoden
`ev_yield`, `tm` und ihre Setter brechen bei einem Index außerhalb des
Bereichs ab (Programmfehler, keine Eingabe aus der Datei).
Formen (Member 496 bis 507) haben dasselbe Layout.
