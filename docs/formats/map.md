# Map-Datei ("Land Data")

**In English:** Documents the map file ("land data"): one member of `land_data.narc` (Diamond/Pearl,
Platinum) or of `a/0/6/5` (HeartGold/SoulSilver). A file consists of a header, a background sound
block (HeartGold/SoulSilver only), the permissions (1,024 byte pairs of kind and collision), the
buildings (48 bytes each), the 3D model (NSBMD) and the BDHC walking heights, and the document gives
offsets and checks for each. For maps painted in the world format the member is the output of baking
and keeps this layout. The code is `crates/g4-formats/src/map.rs`; all values are little endian.

Quelle: DSPRE `MapFile.cs` (Kopfkommentar und Konstruktor, Zeilen 81 bis 128),
für das Gebäude-Layout pret/pokeplatinum `src/overlay005/map_prop.c`
(`MapPropFile`). Implementierung: `crates/g4-formats/src/map.rs`.
Alle Werte Little Endian. Jedes Member von `land_data.narc` (Pt, DP) bzw.
`a/0/6/5` (HGSS) ist eine solche Datei.

Welt-Karten: siehe [world_map.md](world_map.md). Seit 6.3b ist der Member einer
Welt-Karte die Ausgabe des Backens: Permissions, NSBMD und BDHC entstehen aus dem
Weltdokument (Gebäude und BGS kommen aus der Karten-Datei), das Layout bleibt das
hier beschriebene.

## Byte-Layout

| Abschnitt | Offset | Inhalt |
|-----------|--------|--------|
| Kopf (16 Bytes) | 0x00 | `u32` Länge Permissions (immer 2048) |
| | 0x04 | `u32` Länge Gebäude-Abschnitt |
| | 0x08 | `u32` Länge NSBMD (3D-Modell) |
| | 0x0C | `u32` Länge BDHC (Terrain) |
| BGS (nur HGSS) | 0x10 | `u16 0x1234` (Signatur), `u16 len`, danach `len` Bytes |
| Permissions | | 1024 Bytepaare `kind, collision`, Index `y * 32 + x` |
| Gebäude | | n × 48 Bytes, dann optional `Länge % 48` Restbytes |
| NSBMD | | Modell verbatim (`BMD0 ...`) |
| BDHC | | Terrain verbatim |

Der BGS-Block (Hintergrundsound) existiert nur bei HGSS. Ein leerer Block ist
`34 12 00 00` (`BLANK_BGS`). Fehlt die Signatur `0x1234`, liefert `read`
`BadSignature { expected: 0x1234, found }`. Die Datei ist genau so lang wie
Kopf + BGS + die vier Abschnittslängen; zu kurze Dateien ergeben `Truncated`,
überzählige Bytes `BadSize`.

## Permissions

Pro Zelle zwei Bytes: `kind` (Verhaltens-/Geländetyp) und `collision`
(`0x80` = blockiert). Das Raster ist 32 × 32 Zellen. `MapFile::cell` und
`set_cell` akzeptieren Koordinaten `0..=31`; sonst `OutOfRange` mit
`max: 31` (größter gültiger Index, nicht die Rastergröße 32), z. B.
`cell x out of range: 32 (max 31)`. Ein Permissions-Abschnitt mit anderer
Länge als 2048 ergibt `BadSize { expected: 2048, found }`.

## Gebäude (48 Bytes)

| Offset | Feld | Typ |
|--------|------|-----|
| 0x00 | `model_id` | `u32` |
| 0x04 | `position` x, y, z | 3 × `i32`, fx32 (20.12 Festkomma), `y` ist vertikal |
| 0x10 | `rotation` | 3 × `i32` |
| 0x1C | `scale` | 3 × `i32`, fx32, `0x1000` = 1.0 |
| 0x28 | `dummy` | 8 Bytes, verbatim |

Der Kopfkommentar in DSPRE ist an dieser Stelle falsch: Er nennt die
Reihenfolge X, Z, Y und führt "width/height/length" ab 0x1D. Tatsächlich ist
die Reihenfolge x, y, z, und das, was DSPRE als "width = 16" liest, ist der
Skalierungswert `0x1000` (fx32 = 1.0) um ein Byte versetzt gelesen (das
Byte `0x10` an 0x1D ist das zweite Byte von `scale[0]`). Die Felder
`rotation` und `scale` werden in Phase 1 nur als rohe `i32` gehalten.

Einheiten: Eine Kachel sind 16 Welteinheiten, `position` ist fx32 in
Welteinheiten, also entspricht eine Kachel dem Rohwert `0x10000` (65536). Der
Studio rechnet Positionen in Kacheln ab Kartenmitte (`position = Kacheln ×
65536`, gerundet) und Skalierungen als Faktor (`scale = Faktor × 4096`,
gerundet). Das Spiel ignoriert `rotation` bei Karten-Gebäuden
(`applyRotation = FALSE` in `map_prop.c`); der Studio erhält sie verbatim und
zeigt sie nur als Hexwert.

### `buildings_tail`

Der Gebäude-Abschnitt ist nicht immer ein Vielfaches von 48 Bytes. In
Platin DE hat Map 506 einen Abschnitt von 146 Bytes: 3 Gebäude (144 Bytes)
und danach die zwei Bytes `0D 0A` (CRLF), unmittelbar vor `BMD0`. Damit der
Round Trip byte-identisch bleibt, legt `read` die `Länge % 48` Restbytes in
`MapFile::buildings_tail` ab; `write` hängt sie hinter die Gebäude und
rechnet sie in die Abschnittslänge ein. In allen anderen Vanilla-Maps ist das
Feld leer. Die Bytes werden nicht interpretiert und nie verworfen.

## NSBMD und BDHC

Beide Abschnitte werden als undurchsichtige Bytes (`model`, `bdhc`) erhalten
und beim Schreiben unverändert ausgegeben. Die Abschnittslängen im Kopf
berechnet `write` aus den Feldern (`permissions.len() * 2`,
`buildings.len() * 48 + buildings_tail.len()`, `model.len()`, `bdhc.len()`).

Den Inhalt der BDHC (Tabellen, Strukturprüfung, Höhensuche) beschreibt
[bdhc.md](bdhc.md); `g4_formats::bdhc::Bdhc::read(&map.bdhc)` liest ihn. `MapFile`
behält die Bytes trotzdem verbatim.

## Round-Trip-Garantie

Für jede erfolgreich gelesene Datei gilt `write(read(bytes)) == bytes`. Die
gated Vanilla-Tests bestätigen das für alle Maps von Platin DE (666),
HeartGold DE (676) und HeartGold US (676), sowohl für die entpackten Dateien
als auch für die Members des Land-Data-NARC. Bei HGSS wird ein fehlender BGS-Block
(`bgs: None`) als `BLANK_BGS` geschrieben; bei DP/Pt wird `bgs` ignoriert.

## Validierung vor dem Schreiben

`MapFile::write` bleibt infallibel. Aufrufer müssen vorher
`MapFile::validate` aufrufen: `permissions` muss 1024 Zellen haben
(`BadSize { expected: 2048, found }`, in Bytes), und ein vorhandener `bgs`
muss ein vollständiger Block sein: mindestens 4 Bytes (`Truncated`),
Signatur `0x1234` (`BadSignature`) und Längenfeld gleich `bgs.len() - 4`
(`BadSize`). `bgs` wird unabhängig von der Familie geprüft.
`buildings_tail` muss kürzer als ein Gebäude sein (höchstens 47 Bytes,
`BadSize { expected: 47, found }`), sonst würde `read` die Bytes als Gebäude
lesen.
