# Map-Matrix

Quelle: DSPRE `GameMatrix.cs` (Kopfkommentar). Implementierung:
`crates/g4-formats/src/matrix.rs`. Jedes Member von `map_matrix.narc` (Pt, DP)
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

## Strenge beim Lesen

`Matrix::read` lehnt alles ab, was `write` nicht byte-identisch
reproduzieren würde:

- zu kurze Datei: `Truncated { needed, have }`
- Bytes nach dem Karten-Abschnitt: `BadSize { expected, found }`
- `has_headers` oder `has_altitudes` ungleich 0 oder 1:
  `OutOfRange { max: 1 }`
- Name kein gültiges UTF-8: `InvalidUtf8` (es wird nie verlustbehaftet
  konvertiert)

`write` setzt voraus, dass der Name höchstens 255 Bytes lang ist und
`headers`, `altitudes` und `maps` jeweils `width * height` Einträge haben.

## Beobachtungen an den Vanilla-Daten

Geprüft an Platin DE (289 Dateien), HeartGold DE und US (je 288 Dateien):
alle sind byte-identisch round-trippbar, alle Namen sind gültiges UTF-8.
In Platin DE haben 287 Matrizen weder Header-IDs noch Höhen, 2 Matrizen
haben beides (z. B. `0000`, 30 × 30, Name `map`).
