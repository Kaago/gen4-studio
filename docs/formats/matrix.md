# Map-Matrix

**In English:** Documents the map matrix: a grid of map IDs, optionally with header IDs and heights,
that forms an area of the world; each map ID points to a member of `land_data.narc`. A matrix is one
member of `map_matrix.narc` (Diamond/Pearl, Platinum) or of `a/0/4/1` (HeartGold/SoulSilver) and
starts with a 5-byte header, followed by the cell tables. The layout was verified byte for byte on
all three vanilla projects (Platinum DE, HeartGold DE and US). The code is
`crates/g4-formats/src/matrix.rs`; all values are little endian.

Quelle: Das Layout folgt dem Plan bzw. dem Kopfkommentar von DSPRE
`GameMatrix.cs`; die Implementierung wurde gegen alle drei Vanilla-Projekte
(Platin DE, HeartGold DE und US) byte-identisch verifiziert.
Implementierung: `crates/g4-formats/src/matrix.rs`. Jedes Member von `map_matrix.narc` (Pt, DP)
bzw. `a/0/4/1` (HGSS) ist eine solche Datei. Alle Werte Little Endian.

Eine Matrix ist ein Raster aus Karten-IDs (und optional Header-IDs und
Höhen), das einen Bereich der Welt bildet. Die Karten-ID verweist auf ein
Member von `land_data.narc`.

## Byte-Layout

| Abschnitt | Inhalt |
|-----------|--------|
| Kopf (5 Bytes) | `u8 width`, `u8 height`, `u8 has_headers`, `u8 has_altitudes`, `u8 name_len` |
| Name | `name_len` Bytes, UTF-8 |
| Header-IDs | nur wenn `has_headers == 1`: `u16` × width·height |
| Höhen | nur wenn `has_altitudes == 1`: `u8` × width·height |
| Karten | `u16` × width·height |

Alle Raster sind zeilenweise gespeichert, Index `y * width + x`.
`0xFFFF` (`MATRIX_EMPTY`) markiert eine leere Zelle; `Matrix::map_at` liefert
dafür und für Koordinaten außerhalb des Rasters `None`.

## Orientierung

Byte 0 ist die Breite (Länge einer Zeile), Byte 1 die Höhe (Anzahl der
Zeilen). Die frühere Vermutung in der Roadmap (Byte 0 = Höhe, laut
pokeplatinum) trifft nicht zu. Beleg: Das Decomp-JSON
`res/field/matrices/map_matrix_240.json` (Platin) hat Byte 0 = 4 und Byte 1 = 5
und beschreibt 5 Zeilen zu je 4 Karten; unsere Implementierung liest genau so
(`width = 4`, `height = 5`). Festgenagelt wird das durch den gated Test
`vanilla_platinum_matrix_240_is_four_wide_five_high` in
`crates/g4-formats/tests/vanilla.rs` (Matrix 240 in Platin DE).

## Strenge beim Lesen

`Matrix::read` lehnt alles ab, was `write` nicht byte-identisch
reproduzieren würde:

- zu kurze Datei: `Truncated { needed, have }`
- Bytes nach dem Karten-Abschnitt: `BadSize { expected, found }`
- `has_headers` oder `has_altitudes` ungleich 0 oder 1:
  `OutOfRange { max: 1 }`
- Name kein gültiges UTF-8: `InvalidUtf8` (es wird nie verlustbehaftet
  konvertiert)

`write` bleibt infallibel und setzt voraus, dass der Name höchstens 255 Bytes
lang ist und `headers`, `altitudes` und `maps` jeweils `width * height`
Einträge haben. Aufrufer müssen vorher `Matrix::validate` aufrufen:
`OutOfRange` bei zu langem Namen, `BadSize` (in Bytes) bei falscher
Rasterlänge. Ohne `validate` entsteht bei Release-Builds eine Datei, die
`read` ablehnt oder falsch deutet.

## Beobachtungen an den Vanilla-Daten

Geprüft an Platin DE (289 Dateien), HeartGold DE und US (je 288 Dateien):
alle sind byte-identisch round-trippbar, alle Namen sind gültiges UTF-8.
In Platin DE haben 287 Matrizen weder Header-IDs noch Höhen, 2 Matrizen
haben beides (z. B. `0000`, 30 × 30, Name `map`).
