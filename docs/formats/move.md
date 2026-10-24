# Attacken (`moveData/NNNN`)

Quelle: DSPRE `ROMFiles/MoveData.cs`, pret `pokeplatinum` `include/move_table.h`;
Recherche `docs/research/2026-10-23-3b3/dspre-report.md` §1.5. Die Werte sind an
Platin DE Member 0017 (Flügelschlag) geprüft.
Implementierung: `crates/g4-formats/src/data/moves.rs` (`Move`).
Alle Werte Little Endian. Ein Member ist genau 16 Bytes.

## Byte-Layout

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0x00 | u16 | `effect` | Kampfeffekt, Index in die Effekt-Skripte |
| 0x02 | u8 | `category` | 0 physisch, 1 speziell, 2 Status |
| 0x03 | u8 | `power` | Stärke, 0 steht für „keine“ |
| 0x04 | u8 | `move_type` | Typ-Index, 0 bis 17 |
| 0x05 | u8 | `accuracy` | Genauigkeit, 0 trifft immer |
| 0x06 | u8 | `pp` | Attackenpunkte |
| 0x07 | u8 | `effect_chance` | Effektchance in Prozent |
| 0x08 | u16 | `target` | Ziel (Bitfeld, 0 oder `1 << 0` bis `1 << 10`) |
| 0x0A | i8 | `priority` | Priorität, mit Vorzeichen |
| 0x0B | u8 | `flags` | Bits: Kontakt, Schutz, Magiemantel, Brüller, Spiegeltrick, King-Stein, HP-Leiste behalten, Schatten verbergen; Rohbyte |
| 0x0C | u8 | `contest` | Wettbewerbs-Effekt |
| 0x0D | u8 | `contest_type` | 0 bis 4: Coolness, Schönheit, Anmut, Klugheit, Stärke |
| 0x0E | 2 Bytes | `pad_0e` | Padding, in Vanilla immer 0 |

Beispiel Flügelschlag: Stärke 60, Typ 2 (Flug), Genauigkeit 100, AP 35,
Flags `0x73`, Wettbewerbs-Effekt 5.

## Grenzen des Codecs

`read` lehnt nur eine Länge ungleich 16 ab (`BadSize`). Werte außerhalb der
Vanilla-Bereiche werden nicht geprüft; `flags` bleibt ein Rohbyte. Es gilt
`write(read(bytes)) == bytes`.
