# Gegenstände (`itemData/NNNN`)

**In English:** Documents the item data file (`itemData/NNNN`): one member of exactly 34 bytes per
item with its price, hold effect, party-use parameters and further fields. The byte layout,
including the bit layout of the party parameters, was checked on Platinum DE (member 17, Potion, and
several party-effect items). Format knowledge comes from the pret decompilation. The codec is
`crates/g4-formats/src/data/item.rs`; all values are little endian.

Quelle: DSPRE `ROMFiles/ItemData.cs`, pret `pokeplatinum` `include/item.h`
(`ItemData`, `ItemPartyParam`); Recherche
`docs/research/2026-10-23-3b3/dspre-report.md` §1.6. Die Werte und die Bit-Lage
der Party-Parameter sind an Platin DE geprüft (Member 0017 Trank; für die
Party-Bits 0021, 0022, 0028, 0044, 0045 bis 0053 und 0055 bis 0062).
Implementierung: `crates/g4-formats/src/data/item.rs` (`Item`).
Alle Werte Little Endian. Ein Member ist genau 34 Bytes.

## Byte-Layout

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0x00 | u16 | `price` | Preis (Wert, nicht zwingend der Kaufpreis) |
| 0x02 | u8 | `hold_effect` | Trage-Effekt |
| 0x03 | u8 | `hold_param` | Trage-Parameter |
| 0x04 | u8 | `pluck_effect` | Pluck-Effekt |
| 0x05 | u8 | `fling_effect` | Schleuder-Effekt |
| 0x06 | u8 | `fling_power` | Schleuder-Stärke |
| 0x07 | u8 | `natural_gift_power` | Natural-Gift-Stärke |
| 0x08 | u16 | `pocket_word` | Bitfeld, siehe unten |
| 0x0A | u8 | `field_use_func` | Feld-Funktion |
| 0x0B | u8 | `battle_use_func` | Kampf-Funktion (Kategorie) |
| 0x0C | u8 | `party_use` | 1: Party-Parameter gültig |
| 0x0D | u8 | `pad_0d` | Padding |
| 0x0E | 4 Bytes | `party_flags_0e` | Party-Bits 0 bis 31, siehe unten |
| 0x12 | u8 | `party_flags_12` | Party-Bits 32 bis 39 |
| 0x13 | u8 | `party_flags_13` | Party-Bits 40 bis 47 |
| 0x14 | u8 | `party_flags_14` | Party-Bits 48 bis 55 |
| 0x15 | i8 | `hp_ev` | EP-Änderung KP |
| 0x16 | i8 | `attack_ev` | EP-Änderung Angriff |
| 0x17 | i8 | `defense_ev` | EP-Änderung Verteidigung |
| 0x18 | i8 | `speed_ev` | EP-Änderung Initiative |
| 0x19 | i8 | `sp_attack_ev` | EP-Änderung Spezial-Angriff |
| 0x1A | i8 | `sp_defense_ev` | EP-Änderung Spezial-Verteidigung |
| 0x1B | u8 | `hp_amount` | KP-Wert (Trank 20; Member 0028 und 0029 tragen 0xFE und 0xFF) |
| 0x1C | u8 | `pp_amount` | AP-Wert |
| 0x1D | i8 | `friendship_low` | Freundschaft, Stufe niedrig |
| 0x1E | i8 | `friendship_mid` | Freundschaft, Stufe mittel |
| 0x1F | i8 | `friendship_high` | Freundschaft, Stufe hoch |
| 0x20 | 2 Bytes | `pad_20` | Padding |

Ein Feld `pocket` gibt es nicht: das Wort an 0x08 trägt beide Taschen.

## Das Wort an 0x08 (`pocket_word`)

| Bits | Feld |
|------|------|
| 0 bis 4 | `natural_gift_type` |
| 5 | `prevent_toss` (nicht wegwerfbar) |
| 6 | `can_register` (registrierbar) |
| 7 bis 10 | `field_pocket` (Feldtasche) |
| 11 bis 15 | `battle_pocket` (Kampftasche) |

Alle 16 Bits sind benannt. Bits setzen ändert nur die eigene Maske.

## Party-Parameter (`ItemPartyParam`, 0x0E bis 0x20)

Die sieben Bytes ab 0x0E bilden einen 56-Bit-Wert (Bit 0 ist das niedrigste Bit
von Byte 0x0E). 52 Bits sind benannt, die vier höchsten Bits (52 bis 55)
bleiben roh.

| Bits | Feld |
|------|------|
| 0 bis 6 | `slp_heal`, `psn_heal`, `brn_heal`, `frz_heal`, `prlz_heal`, `cfs_heal`, `inf_heal` (Schlaf, Gift, Verbrennung, Eis, Paralyse, Verwirrung, Anziehung) |
| 7 | `guard_spec` |
| 8, 9 | `revive`, `revive_all` |
| 10, 11 | `level_up`, `evolve` |
| 12 bis 15 | `attack_stages` |
| 16 bis 19 | `defense_stages` |
| 20 bis 23 | `sp_attack_stages` |
| 24 bis 27 | `sp_defense_stages` |
| 28 bis 31 | `speed_stages` |
| 32 bis 35 | `accuracy_stages` |
| 36 bis 37 | `crit_stages` |
| 38, 39 | `pp_up`, `pp_max` |
| 40, 41, 42 | `pp_restore`, `pp_restore_all`, `hp_restore` |
| 43 bis 48 | `hp_ev_flag`, `attack_ev_flag`, `defense_ev_flag`, `speed_ev_flag`, `sp_attack_ev_flag`, `sp_defense_ev_flag` |
| 49, 50, 51 | `friendship_low_flag`, `friendship_mid_flag`, `friendship_high_flag` |

Die Reihenfolge der Stufen-Felder (Angriff, Verteidigung, Spezial-Angriff,
Spezial-Verteidigung, Initiative, Genauigkeit) und der EP-Felder (KP, Angriff,
Verteidigung, Initiative, Spezial-Angriff, Spezial-Verteidigung) ist an den
Mustern in Vanilla belegt: Member 0057 bis 0062 setzen je genau ein Stufen-Feld,
0045 bis 0049 und 0052 je genau ein EP-Flag samt Wert im passenden Byte.
Beobachtung: Member 0050 (Bonbon mit `level_up`) hat zusätzlich Bit 8
(`revive`) gesetzt; der Codec rührt das nicht an.

## Grenzen des Codecs

`read` lehnt nur eine Länge ungleich 34 ab (`BadSize`). Insbesondere wird ein
36-Byte-Member (so schreibt DSPRE, siehe Recherche §1.6) nicht gekürzt, sondern
abgelehnt; die Länge eines Members entscheidet das Kernel-Asset. Unbenannte
Bits bleiben verbatim, es gilt `write(read(bytes)) == bytes`.

Item-ID und Member-Index sind nicht dasselbe: die Zuordnung steht in der
ARM9-Tabelle und gehört nicht zu diesem Codec.
