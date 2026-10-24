# Trainer: Eigenschaften und Party (`trainerProperties/NNNN`, `trainerParty/NNNN`)

Quelle: DSPRE `ROMFiles/TrainerFile.cs`, pret `pokeplatinum`
`include/struct_defs/trainer_data.h`, pret `pokeheartgold`
`include/trainer_data.h`; Recherche
`docs/research/2026-10-23-3b3/dspre-report.md` §1.3 und §4.3. Die Werte und die
Reihenfolge sind an Platin DE und beiden HeartGold-Projekten geprüft (alle
928 beziehungsweise 738 Trainer lesen und schreiben byte-identisch zurück).
Implementierung: `crates/g4-formats/src/data/trainer.rs` (`TrainerProps`,
`PartyFormat`, `PartyMon`, `Party`). Alle Werte Little Endian.

## Eigenschaften (`trainerProperties`, 20 Bytes)

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0x00 | u8 | `flags` | Bit 0 `has_moves` (eigene Attacken), Bit 1 `has_items` (Items je Mon) |
| 0x01 | u8 | `class` | Trainerklasse |
| 0x02 | u8 | `unknown` | in Vanilla immer 0; bleibt erhalten |
| 0x03 | u8 | `party_count` | Größe der Party, 0 bis 6 |
| 0x04 | u16 × 4 | `items` | Items des Trainers (`items()`, `set_items`) |
| 0x0C | u32 | `ai` | AI-Flags, alle 32 Bit bleiben erhalten (DSPRE kappt auf 11) |
| 0x10 | u32 | `battle_type` | 0 einfach, 2 doppelt; andere Werte bleiben erhalten |

Die Reihenfolge ist an den Vanilla-Daten geprüft: Byte 3 ist die Party-Größe,
nicht Byte 2. Die Flags in Vanilla (Platin): 0 in 672, 1 in 198, 3 in 54 und 2
in 4 Membern. `party_format()` macht aus Bit 0 und 1 ein `PartyFormat`; das
Flag-Byte ist die eine Quelle der Wahrheit für das Party-Format.

## Party (`trainerParty`, variabel)

Ein Member enthält `party_count` Einträge, danach Füllbytes. Die Länge ist ein
Vielfaches von 4, mindestens 8 (Trainer 0 hat keine Mons und eine 8-Byte-Datei
aus Nullen).

Länge eines Eintrags (`PartyFormat::entry_len(family)`):

| Familie | einfach | + Item | + Attacken | + beides |
|---------|---------|--------|------------|----------|
| Diamant/Perl | 6 | 8 | 14 | 16 |
| Platin, HeartGold/SoulSilver | 8 | 10 | 16 | 18 |

Aufbau eines Eintrags in dieser Reihenfolge:

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0 | u8 | `iv_or_difficulty` | Schwierigkeit; IV = Wert × 31 / 255 |
| 1 | u8 | `byte1` | HGSS: Geschlecht/Fähigkeit (Bits 0 bis 3 Geschlecht, 4 bis 7 Fähigkeit); Platin: oberes Byte der IV-Skala (in Vanilla fast immer 0) |
| 2 | u16 | `level` | Stufe |
| 4 | u16 | `species_word` | `species()` Bits 0 bis 9, `form()` Bits 10 bis 15 |
| 6 | u16 | `item` | nur wenn `items`; sonst `None` |
| | u16 × 4 | `moves` | nur wenn `moves`; sonst `None` |
| | u16 | `ball_capsule` | nur Platin und HGSS; Diamant/Perl hat keine Kapsel |

`byte1` wird in allen Familien als Rohwert geführt; ob er als Geschlecht und
Fähigkeit gedeutet wird, entscheidet die Oberfläche (nur HGSS).

## Lesen und Schreiben

- `Party::read(bytes, format, count, family)` liest `count` Einträge; alles
  danach steht roh in `tail`. Eine Datei, die kürzer ist als
  `count × entry_len`, wird mit `Truncated` abgelehnt, sonst nichts.
- Vanilla: in Platin tragen 42 von 928 Dateien zwei Bytes mehr als
  `count × entry_len`, in beiden HeartGold-Projekten 34 von 738 (diese Bytes
  stehen in `tail`). Alle anderen haben genau `count × entry_len` Bytes, nur
  Trainer 0 (keine Mons) hat acht Nullbytes. Alle Dateien bleiben
  byte-identisch.
- `Party::write(format, family)` schreibt die Einträge im Format. `Party`
  merkt sich in `read_len`, wie viele Bytes die Einträge beim Lesen belegt
  haben (`count × entry_len`). Belegen die geschriebenen Einträge genauso viele
  Bytes (bei gleicher Zahl und gleichem Format), folgt `tail` unverändert; sonst entfällt
  `tail`. Danach kommen Nullen bis zu einem Vielfachen von 4 Bytes (mindestens
  8). Eine bearbeitete Party ist damit genau `n × entry_len`, aufgerundet auf
  ein Vielfaches von 4 und mindestens 8 Bytes lang und wächst bei wiederholtem
  Bearbeiten nie. Eine nicht gelesene Party hat `read_len` 0.
- Das Format, nicht die Optionen am Eintrag, bestimmt die Länge: bei `items`
  ohne `item` steht eine 0, bei Diamant/Perl entfällt die Kapsel.
- Für `read(write(d)) == d` muss `tail` kanonisch sein: Einträge und `tail`
  füllen schon ein Vielfaches von 4 Bytes und `read_len` passt. Bei einer leeren
  Party sind das acht Nullen (`empty_party_is_eight_zero_bytes`).
- Wer das Format ändert („eigene Attacken“ oder „Items“ umschalten), schreibt
  mit dem neuen Format; die Optionen am Eintrag (`item`, `moves`) bleiben dabei
  im Speicher erhalten, bis sie bewusst gelöscht werden.

## Grenzen des Codecs

`TrainerProps::read` lehnt nur eine Länge ungleich 20 ab (`BadSize`).
Unbekannte Bits und Werte (AI, Kampfart, Form-Bits, `unknown`) werden nie
abgelehnt und bleiben unverändert. Gegenüber DSPRE bleiben Padding, hohe
AI-Bits und der Kampfart-Wert erhalten.
