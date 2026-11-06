# NSBMD-Kopf (BMD0) und MDL0-Wörterbücher

Quelle: apicula `src/nitro/{container,info_block}.rs` (0BSD) als Layout-Referenz, GBATEK,
pret/pokeplatinum `land_data.c` (`numEntry == 1`), eigene Messung
(`docs/research/2026-10-23-3b4/task0-3b4.md` §6).
Implementierung: `crates/g4-formats/src/nsbmd_header.rs`. Alle Werte Little Endian.
Eine dünne Hülle für den Import: kein Geometrie-Leser, der MDL0-Block wird byteweise
übernommen. Die Wörterbücher liest `nitro_dict`; die ganze Struktur des Modells (Wörterbuch
mit Baum, Knochen, SBC, Materialien, Shapes, GPU-Befehle) und der Writer stehen in
[`nsbmd.md`](nsbmd.md).

## Container

| Offset | Inhalt | Prüfung |
|--------|--------|---------|
| 0x00 | `b"BMD0"` | sonst `BadMagic` |
| 0x04 | `u16 0xFEFF` (Bytes `FF FE`) | sonst `BadSignature` |
| 0x06 | `u16 0x0002` Version | sonst `BadSignature` |
| 0x08 | `u32` Dateigröße | muss der Länge entsprechen (`BadSize`) |
| 0x0C | `u16 0x0010` Kopfgröße | sonst `BadSize` |
| 0x0E | `u16 n` Anzahl Blöcke | 1 oder 2, sonst `OutOfRange` |
| 0x10 | n × `u32` Blockoffset ab Dateianfang | hinter der Offsettabelle, Block (Tag + `u32` Größe) ganz in der Datei |

Alle Vanilla-Kartenmodelle (Pt DE 666, HG DE/US je 676) haben einen Block `MDL0` an 0x14.

## TEX0 entfernen (`strip_tex0`)

- MDL0 wird über die Blockoffsets gesucht, nicht fest an 0x18 angenommen; TEX0 darf vor oder hinter MDL0 liegen.
- Ein Modell mit nur `MDL0` bleibt unverändert (Vanilla byte-gleich).
- Sonst: neuer Kopf mit einem Block, Offset 0x14, Größenfeld = neue Länge, danach der MDL0-Block (Größe aus seinem Feld). Offsets im MDL0 sind blockrelativ und bleiben gültig.

## Wörterbuch (Nitro-Dictionary)

| Offset `o` | Inhalt |
|------------|--------|
| o + 0 | `u8 0` Revision |
| o + 1 | `u8 n` Einträge |
| o + 6 | `u16` Offset des Eintragsblocks `e` (relativ zu `o`) |
| e + 0 | `u16` Größe eines Datums |
| e + 2 | `u16` Offset der Namen (relativ zu `e`) |
| e + 4 | n Daten |
| Namen | n × 16 Byte, mit NUL aufgefüllt |

Gelesen mit `nitro_dict::read`: nur das Vanilla-Layout wird angenommen (Größenfeld,
Eintragsoffset `12 + 4n` und Namensoffset `4 + unit · n` müssen passen, sonst `BadSize`;
Wort an o + 4 gleich 8). Einzelheiten und der Baum in [`nsbmd.md`](nsbmd.md).

## MDL0

- Modell-Wörterbuch an MDL0 + 8; `model_count` = n. Das Spiel verlangt genau ein Modell.
- Modell 0 liegt bei MDL0 + `u32` (erstes Datum). Modellkopf: `u32` Größe, `ofsSbc`, `ofsMat` (+8), `ofsShp`, `ofsEvp`.
- Materialabschnitt `m` = Modell + `ofsMat`: `u16 ofsDictTexToMatList`, `u16 ofsDictPlttToMatList` (relativ zu `m`), Material-Wörterbuch an m + 4.
- `material_texture_names` / `material_palette_names` liefern die Namen der beiden Wörterbücher.
- Fehlende Namen gegenüber dem Kartentextur-Paket des Gebiets gibt es schon in Vanilla (Pt 14, HG 7 Karten): beim Import nur ein Hinweis.
