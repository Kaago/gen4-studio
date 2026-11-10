# Trainer-Texttabellen (`trainerTextTable/0000`, `trainerTextOffset/0000`)

**In English:** Documents the two trainer text tables, `trainerTextTable/0000` and
`trainerTextOffset/0000`, which are archives with exactly one member each. The text table is a
sequence of 4-byte entries (trainer ID and trigger) in which entry `i` belongs to message `i` of the
battle text archive; the offset table holds byte offsets into the text table, one per trainer. The
sizes were checked on all three vanilla projects, and the document notes what is not known, such as
how the game behaves for a missing offset. The codec is
`crates/g4-formats/src/data/trainer_text.rs`; all values are little endian.

Quelle: DSPRE `RomInfo.cs` (`trainerTextTable`, `trainerTextOffset`); Recherche
`docs/research/2026-10-23-3b3/dspre-report.md` §0.1 und
`docs/research/2026-10-23-3b3/task0-3b3a.md`. Die Größen sind an allen drei
Vanilla-Projekten geprüft. Implementierung:
`crates/g4-formats/src/data/trainer_text.rs` (`TrainerTextTable`,
`TrainerTextOffsets`). Alle Werte Little Endian. Beide Archive haben genau ein
Member (`0000`).

## Texttabelle (`trtbl`)

Folge von Einträgen zu je 4 Bytes.

| Offset | Länge | Feld | Bemerkung |
|--------|-------|------|-----------|
| 0 | u16 | Trainer | Trainer-ID |
| 2 | u16 | Auslöser | Art des Kampftextes |

`TrainerTextTable { entries: Vec<(u16, u16)> }`: Eintrag `i` gehört zur
Nachricht `i` des Kampftext-Textarchivs (Platin 617 mit 2497 Nachrichten und
9988 Bytes, HGSS 728 mit 1717 Nachrichten und 6868 Bytes). Die Tabelle ist nicht
nach Trainer sortiert.

## Offset-Tabelle (`trtblofs`)

Folge von `u16`, Byte-Offsets in die Texttabelle. `TrainerTextOffsets { offsets:
Vec<u16> }`. Platin hat 928 Einträge (einer je Trainer, 1856 Bytes), HGSS nur
735 (1470 Bytes) bei 738 Trainern. Wie das Spiel bei einem fehlenden Eintrag
sucht, ist nicht untersucht.

## Grenzen des Codecs

`TrainerTextTable::read` lehnt nur eine Länge ab, die kein Vielfaches von 4 ist,
`TrainerTextOffsets::read` nur eine ungerade Länge (jeweils `BadSize`). Werte
werden nie geprüft; es gilt `write(read(bytes)) == bytes`. Der Codec deutet die
Offsets nicht und hält Tabelle und Offsets nicht konsistent.
