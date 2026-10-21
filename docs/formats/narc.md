# NARC (Nitro Archive)

Quelle: gbatek (NDS File Formats, "NARC") und DSPRE `Narc.cs` (Zeilen 87 bis 129).
Implementierung: `crates/g4-formats/src/narc.rs`. Alle Werte Little Endian.

## Byte-Layout

| Abschnitt | Offset | Inhalt |
|-----------|--------|--------|
| Header (16 Bytes) | 0x00 | `b"NARC"` |
| | 0x04 | `u16 0xFFFE` (Byte Order Mark) |
| | 0x06 | `u16 0x0100` (Version) |
| | 0x08 | `u32 total_size` (Gesamtgröße der Datei) |
| | 0x0C | `u16 0x0010` (Header-Größe) |
| | 0x0E | `u16 3` (Anzahl Blöcke) |
| BTAF | 0x10 | `b"BTAF"` |
| | 0x14 | `u32 size = 12 + 8*n` |
| | 0x18 | `u16 n` (Anzahl Member), `u16 0` |
| | 0x1C | n × (`u32 start`, `u32 end`) |
| BTNF | | `b"BTNF"`, `u32 0x10`, `u32 0x4`, `u32 0x00010000` (namenlos) |
| GMIF | | `b"GMIF"`, `u32 size = 8 + payload_len`, danach die Nutzdaten |

`start` und `end` sind relativ zum Beginn der GMIF-Nutzdaten (also nach den
8 Bytes Blockkopf). `start` ist auf 4 Bytes ausgerichtet, `end = start + len`.
Das Member ist also genau `payload[start..end]`; die Länge ohne Padding steht
in der Tabelle.

## Padding

Jedes Member wird in den GMIF-Nutzdaten mit `0xFF` auf ein Vielfaches von 4
Bytes aufgefüllt, auch das letzte. `payload_len` ist deshalb immer durch 4
teilbar, und der nächste `start` ist `end` aufgerundet auf 4.

Die Annahme `0xFF` ist durch die gated Vanilla-Tests bestätigt: Die Archive
für Maps und Matrizen enthalten sehr viele nicht ausgerichtete Member (zum
Beispiel 431 von 666 in `land_data.narc` von Platin) und lassen sich mit
`0xFF`-Padding byte-identisch zurückschreiben (Platin DE, HeartGold DE und
HeartGold US). Eine Abweichung von der Annahme (`0x00`) gab es nicht.

## Grenzen des Codecs

- Es wird nur die namenlose BTNF-Variante geschrieben. `read` überspringt den
  BTNF-Inhalt, ein Archiv mit echten Dateinamen würde beim Schreiben seine
  Namen verlieren. Die Vanilla-Archive von Pt und HGSS sind namenlos.
- `read` ignoriert das Padding; nur `start..end` zählt. Ein `end` jenseits der
  GMIF-Nutzdaten ergibt `FormatError::Truncated`, ein `start > end`
  `FormatError::OutOfRange`.
- Archive, die nicht aus der kanonischen Reihenfolge (BTAF, BTNF, GMIF) oder
  mit überlappenden Membern aufgebaut sind, werden gelesen, aber nicht
  byte-identisch geschrieben.
