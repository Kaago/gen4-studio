# NSBTX (BTX0 / TEX0)

Quelle: apicula `src/nitro/{container,tex,info_block,name}.rs` und `src/nds/decode_texture.rs`
(0BSD) als Layout-Referenz; DSPRE `LibNDSFormats/NSBTX/nsbtx.cs` (Zeilen 350 bis 664) für
die Texturformate (nur Fakten). Implementierung: `crates/g4-formats/src/nsbtx.rs`.
Alle Werte Little Endian. Nur Lesen und Dekodieren, kein Schreiben.

## Container (BTX0)

| Offset | Inhalt |
|--------|--------|
| 0x00 | `b"BTX0"` |
| 0x04 | `u16 0xFEFF` (Byte Order Mark, Bytes `FF FE`) |
| 0x06 | `u16` Version (wird nicht geprüft) |
| 0x08 | `u32 file_size` (kleiner als die Eingabe ist ein Fehler `Truncated`) |
| 0x0C | `u16 0x0010` (Header-Größe) |
| 0x0E | `u16 n` Anzahl Sektionen |
| 0x10 | n × `u32` Offset der Sektion (ab Dateianfang) |

Der Leser nimmt die erste Sektion mit `b"TEX0"`; fehlt sie, ist das `BadMagic { "TEX0" }`.

## TEX0-Sektion

Alle Offsets in der Sektion sind relativ zum Beginn der Sektion (`b"TEX0"`).

| Offset | Größe | Inhalt |
|--------|-------|--------|
| 0x00 | 4 | `b"TEX0"` |
| 0x04 | 4 | `u32 section_size` (nicht genutzt) |
| 0x0C | 2 | `u16` Länge des Texturblocks / 8 (nicht genutzt) |
| 0x0E | 2 | `u16 texture_off` Offset des Texturen-Info-Blocks |
| 0x14 | 4 | `u32 tex_block_off` Offset der Texeldaten |
| 0x1C | 2 | `u16` Länge des komprimierten Blocks / 8 (Format 5, nicht genutzt) |
| 0x1E | 2 | `u16` Offset des komprimierten Info-Blocks (nicht genutzt) |
| 0x24, 0x28 | 4 + 4 | Offsets der Texeldaten für Format 5 (nicht genutzt) |
| 0x30 | 2 | `u16 pal_block_len / 8` |
| 0x34 | 4 | `u32 palette_off` Offset des Paletten-Info-Blocks |
| 0x38 | 4 | `u32 pal_block_off` Offset der Palettendaten |

Der feste Kopf ist 0x3C Bytes lang. Nicht aufgeführte Felder sind unbekannt und werden
übersprungen.

## Info-Block

Gemeinsame Struktur für Texturen und Paletten (Offset `o` ab Sektionsbeginn, n = Anzahl):

| Offset | Größe | Inhalt |
|--------|-------|--------|
| o + 0 | 1 | `u8 0` (Dummy, sonst Fehler `BadSignature`) |
| o + 1 | 1 | `u8 n` |
| o + 2 | 2 | `u16` Größe des Kopfs (wird nicht genutzt) |
| o + 4 | 8 | Unterkopf (`u16 8`, `u16` Größe, `u32 0x017F`), nicht genutzt |
| o + 12 | 4n | n × `u32` unbekannt (nicht genutzt) |
| o + 12 + 4n | 2 | `u16` Größe eines Datums (Textur 8, Palette 4, sonst `BadSize`) |
| o + 14 + 4n | 2 | `u16` Größe des Datenabschnitts (nicht genutzt) |
| o + 16 + 4n | n × Datum | Daten |
| danach | 16n | n Namen, je 16 Bytes, mit NUL aufgefüllt |

Namen werden Byte für Byte als Zeichen gelesen; Bytes unter 0x20 werden zu `.`.

### Texturdatum (8 Bytes)

`u32 params`, `u32` unbekannt. `params`:

| Bits | Bedeutung |
|------|-----------|
| 0 bis 15 | Offset der Texeldaten / 8, relativ zu `tex_block_off` |
| 16 bis 19 | Wiederholen und Spiegeln S/T (nicht genutzt) |
| 20 bis 22 | Breite = `8 << v` |
| 23 bis 25 | Höhe = `8 << v` |
| 26 bis 28 | Format |
| 29 | `color0`: Palettenindex 0 ist transparent |
| 30 bis 31 | Texturkoordinaten-Transformation (nicht genutzt) |

Die Spec (§4.1) nennt dieselben Felder in der oberen Hälfte des Worts (`p = params >> 16`):
Format Bits 10 bis 12, Breite `8 << ((p >> 4) & 7)`, Höhe `8 << ((p >> 7) & 7)`, `color0` Bit 13.

### Paletten-Datum (4 Bytes)

`u16 offset / 8` (relativ zu `pal_block_off`), `u16` unbekannt. Die Länge einer Palette
steht nicht in der Datei: sie reicht bis zum nächsthöheren Palettenoffset, die letzte bis
zum Ende des Palettenblocks. Farben sind BGR555 (`u16`: Bits 0 bis 4 Rot, 5 bis 9 Grün,
10 bis 14 Blau).

## Texturformate

| Format | Name | Bits je Texel | Bytes (B × H) | Dekodierung |
|--------|------|---------------|---------------|-------------|
| 1 | A3I5 | 8 | B·H | Index = Bits 0 bis 4, Alpha = `(v >> 5) * 255 / 7` |
| 2 | 4 Farben | 2 | B·H / 4 | niedrigste Bits zuerst, Alpha 255 |
| 3 | 16 Farben | 4 | B·H / 2 | niedriges Nibble zuerst, Alpha 255 |
| 4 | 256 Farben | 8 | B·H | Alpha 255 |
| 5 | 4×4-Texel | 2 + 1 | | nicht unterstützt (`FormatError::Unsupported`), Texeldaten werden nicht gelesen |
| 6 | A5I3 | 8 | B·H | Index = Bits 0 bis 2, Alpha = `(v >> 3) * 255 / 31` |
| 7 | direkt | 16 | 2·B·H | BGR555 mit Bit 15 = Alpha (0 oder 255), keine Palette |

Zeilen laufen von oben nach unten, Pixel von links nach rechts (Zeile für Zeile).
5-Bit-Komponenten werden mit `v << 3 | v >> 2` auf 8 Bit erweitert.

Paletten-Index 0 ist transparent (Alpha 0), wenn das `color0`-Flag der Textur gesetzt ist
(Formate 2, 3, 4; die Formate 1 und 6 ignorieren das Flag wie die Hardware) oder wenn
`decode_with(.., force_index0_transparent = true)` das erzwingt (dann für jedes
Palettenformat; die Sprite-Schicht nutzt das für Overworld-Sprites). Ein Farbindex hinter
dem Palettenende liest als Farbe 0 (schwarz).

## Fehlerverhalten

Jeder Zugriff ist längengeprüft (`Truncated`, `BadMagic`, `BadSignature`, `BadSize`,
`OutOfRange` für Textur- und Palettenindex bei `decode`), ohne Panic. Die Texeldaten und
Paletten werden schon in `read` bereichsgeprüft und kopiert.

## Verifikation

- Unit-Tests in `nsbtx.rs` bauen ein minimales BTX0 nach diesem Layout und prüfen die
  Listen, die Formate 1, 2, 3, 4, 6, 7, `color0`, das Erzwingen von Index 0, Format 5,
  fehlerhafte Magic, ungültige Indizes, jede Kürzung der Eingabe und Bytefehler an jeder
  Position (kein Panic).
- Gated (`tests/vanilla.rs`, `G4_TEST_PROJECT`): Member 0 des Sprite-NARC
  (`files/data/mmodel/mmodel.narc` bzw. `files/a/0/8/1`) ist ein BTX0 mit 16 Texturen
  (32×32, Format 3, `color0` gesetzt) und einer Palette; Pixel mit Alpha 255 und 0 sind
  vorhanden. Alle Members, die mit `BTX0` beginnen (Platin DE 421 von 470, HeartGold DE und
  US je 832 von 863; der Rest sind BMD0 und andere Daten), lassen sich lesen und mit jeder
  Palette dekodieren (kein Format 5 darunter).
