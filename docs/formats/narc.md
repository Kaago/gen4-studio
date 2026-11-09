# NARC (Nitro Archive)

**In English:** Documents the NARC (Nitro Archive), the container the games use for most of their
data: one file holding numbered members. The document gives the byte layout of the 16-byte header
and of the BTAF (allocation table), BTNF (name table) and GMIF (file image) blocks. The sources are
the public GBATEK description of the NDS file formats. The code is `crates/g4-formats/src/narc.rs`;
all values are little endian.

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

Der Codec erhält unbekannte Bytes verbatim oder lehnt die Eingabe ab. `read`
liefert einen Fehler (bestehende `FormatError`-Varianten) für alles, was
`write` nicht byte-identisch reproduzieren würde:

- BTNF ist nicht exakt der kanonische namenlose Block (`BTNF`, `0x10`, `0x4`,
  `0x00010000`): `BadSize` bei abweichender Blockgröße, `BadMagic` bei
  abweichendem Inhalt. Archive mit echten Dateinamen werden also abgelehnt und
  verlieren ihre Namen nicht stillschweigend. Die Vanilla-Archive von Pt und
  HGSS sind namenlos.
- Version ungleich `0x0100` (`BadSignature`), Blockanzahl ungleich 3 oder
  Header-Größe ungleich `0x10` (`BadSize`), BTAF mit falscher Größe oder
  Reserved-Feld ungleich 0.
- Bytes hinter dem GMIF-Block, also `bytes.len() != total_size` oder ein
  GMIF-Ende ungleich `total_size` (`BadSize`).
- `end` jenseits der GMIF-Nutzdaten ergibt `Truncated`, `start > end`
  `OutOfRange`.
- Nicht-kanonisches Layout: Der erste `start` muss 0 sein und jeder weitere
  `start` muss dem auf 4 aufgerundeten `end` des Vorgängers entsprechen
  (`BadSize`, `expected` ist der kanonische Start). Überlappende, lückenhafte
  oder nicht ausgerichtete Member werden also abgelehnt. Ebenso Nutzdaten
  hinter dem aufgerundeten Ende des letzten Members (`BadSize`).
- Padding-Bytes ungleich `0xFF` (`BadSignature { expected: 0xFF, found }`,
  hier tragen die u16-Felder das erwartete und das gefundene Byte).

Damit gilt für jedes erfolgreich gelesene Archiv `write(read(bytes)) == bytes`.
