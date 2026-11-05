# BDHC (Laufhöhen)

Quelle: pret/pokeplatinum `include/overlay005/bdhc.h` und `src/overlay005/bdhc.c`
(Leser und Höhensuche, nur gelesen, Formatwissen), Messungen und Emulator-Probe aus
[Gutachten B](../research/2026-11-05-phase-6/gutachten-b-bdhc-engine.md) §2.1, §2.2, §2.5 mit
dem eigenen Prototyp `gutachten-b-bdhc-bau.py`.
Implementierung: `crates/g4-formats/src/bdhc.rs`. Alle Werte Little Endian.

Die BDHC ist der vierte Abschnitt einer Kartendatei (`MapFile::bdhc`, siehe [map.md](map.md)).
Sie sagt dem Spiel, auf welcher Höhe eine Figur an einem Punkt der Karte steht. Die Kollision
(Permissions) ist davon getrennt.

## Byte-Layout

| Abschnitt | Eintrag | Größe | Inhalt |
|---|---|---|---|
| Kopf | | 16 B | `"BDHC"`, dann sechs `u16`: Punkte, Normalen, Konstanten, Platten, Streifen, Zugriffsliste |
| Punkte | `Point` | 8 B | `fx32 x, fx32 z` |
| Normalen | `Normal` | 12 B | `fx32 x, y, z` (Vanilla normiert, z. B. (0, 1, 0), (0,707, 0,707, 0), (0,447, 0,894, 0)) |
| Konstanten | `i32` | 4 B | `d` der Ebene `n · p + d = 0` |
| Platten | `Plate` | 8 B | `u16` erster Punkt, zweiter Punkt, Normale, Konstante; die zwei Punkte sind gegenüberliegende Ecken eines achsparallelen Rechtecks |
| Streifen | `Strip` | 8 B | `fx32 scanline` (z), `u16` Anzahl, `u16` Start in der Zugriffsliste; aufsteigend nach `scanline` |
| Zugriffsliste | `u16` | 2 B | Plattennummern je Streifen, hintereinander |

Nutzlast (`Bdhc::payload_len`) = 8 P + 12 N + 4 C + 8 Pl + 8 S + 2 A. Das Spiel liest sie in
einen Puffer von 0x9000 Bytes.

`Bdhc::read` verlangt `"BDHC"` (sonst `BadMagic`), mindestens den 16-Byte-Kopf (sonst
`Truncated`) und eine Länge von genau 16 + Nutzlast (sonst `BadSize { expected, found }`, auch
bei zu wenig Bytes). Indizes prüft `read` nicht, damit jede Vanilla-Datei gelesen und
zurückgeschrieben werden kann; das macht `validate`. `write` ist infallibel und schreibt die
Tabellenlängen als Zähler. Eine Tabelle über `u16::MAX` bekäme einen übergelaufenen Zähler,
darum vorher `validate`.

## Koordinaten und Einheiten

- Alle Koordinaten und Höhen sind **fx32** (20.12-Festkomma, `FX_ONE` = 4096 = 1 E).
- `x` und `z` zählen ab der **Kartenmitte**, von −256 bis +256 E; `y` ist die Höhe.
- Eine Kachel ist `TILE_UNITS` = 16 E, eine Höhenstufe `STEP_UNITS` = 8 E.
- `tile_center(x, y)` gibt die Mitte der Kachel (Spalte `x`, Zeile `y` der Permissions):
  `((16 x + 8 − 256) · 4096, (16 y + 8 − 256) · 4096)`. Zeile `y` wächst nach Süden (z).
- `height_at` nimmt und gibt fx32; in Einheiten (`f32`) rechnet erst die DTO-Grenze um.

## Höhensuche (`Bdhc::height_at`)

`height_at(x, z, current)` rechnet wie `CalculateObjectHeight` des Spiels (`bdhc.c`):

1. **Streifen**: der erste mit `scanline > z`, sonst der letzte. Ein `z` genau auf einer
   Streifengrenze nimmt also den südlichen Streifen. (pret sucht binär; auf sortierten Streifen
   ist das Ergebnis dasselbe, gemessen für 1 bis 33 Streifen.) Ohne Streifen: `None`.
2. **Platten** des Streifens in Zugriffsreihenfolge. Eine Platte trifft, wenn der Punkt im
   Rechteck liegt, **Rand eingeschlossen** (`min..=max` beider Punkte, die Reihenfolge der Ecken
   ist gleich).
3. Je Treffer `y = fx_div(−(fx_mul(nx, x) + fx_mul(nz, z) + d), ny)`, höchstens
   `MAX_CANDIDATES` = 10 Treffer; weitere Platten zählen nicht. Eine Platte mit `ny == 0`
   liefert keinen Kandidaten (das Spiel teilte durch 0) und zählt nicht zu den zehn.
4. **Gewinner** ist der Kandidat mit dem kleinsten `|y − current|` (in `i64`), bei Gleichstand
   der erste. So gehen Brücken: zwei Platten übereinander, die Figur bleibt auf ihrer Ebene.
   Ohne Kandidaten: `None` (im Spiel behält die Figur ihre Höhe).

Festkomma wie im Spiel und im Prototyp:

- `fx_mul(a, b) = ((a as i64 · b as i64) >> 12) as i32`, also abgerundet (`fx_mul(−1, 1) = −1`);
- `fx_div(a, b) = ±(|a| << 12) / |b|`, Betrag abgerundet, Vorzeichen aus beiden
  (`fx_div(−1, 3) = −1365`), `None` bei `b == 0`.

Die Summe rechnet mit Überlauf wie das Spiel (`wrapping_add`). Einträge außerhalb der Tabellen
(die `validate` meldet) überspringt die Suche, sie panickt also auf keiner gelesenen Datei.

**Beispiel** (Emulator-Probe, B §2.5): Eine Rampe steigt nach Osten 8 E je Kachel, Westkante
der Kachel 4 auf Höhe 0. Die Normale `(−1832, 3664, 0)` ist `(−0,5; 1; 0)` normiert und
gerundet, die Konstante −351 703. An der Mitte der Kachel (4, 6), `x = −184 · 4096`, ist
`fx_mul(−1832, x) = 337 088`, die Summe mit `d` −14 615, also `y = fx_div(14 615, 3664) =
16 338` fx = **3,989 E** statt 4. Genau diesen Wert hat das Spiel im RAM (alle drei Spiele).

## Prüfung (`Bdhc::validate`)

`validate` gibt eine Liste von `BdhcProblem` in Tabellenreihenfolge (erst die Zähler, dann die
Platten, die Streifen, die Zugriffsliste), Fehler und Warnung gemischt; `is_error()`
unterscheidet sie, `message()` gibt den englischen Satz mit Zahlen.

| Problem | Art | Satz (`message`) |
|---|---|---|
| `TooMany { table, len }` | Fehler | `points: 70000 entries, more than a u16 counter holds` |
| `PlateIndex { plate, table, index, len }` (Punkt, Normale oder Konstante) | Fehler | `plate 3 names point 40, past the 18 points` |
| `VerticalPlate { plate, normal }` (Normale mit `y == 0`) | **Warnung** | `plate 5 is vertical (normal 3 has y = 0)` |
| `StripRange { strip, start, count, len }` (`start + count` über der Zugriffsliste) | Fehler | `strip 2 covers access 10..14, past the 12 entries` |
| `StripOrder { strip, scanline, previous }` (`scanline` kleiner als beim Vorgänger; gleich ist erlaubt) | Fehler | `strip 4 starts at z -32.000, below strip 3 at z 16.000` |
| `AccessIndex { entry, plate, plates }` | Fehler | `access entry 7 names plate 12, past the 9 plates` |

Das Spiel selbst prüft fast nichts (belegt (pret), B §2.1):

| Abweichung | Was im Spiel passiert |
|---|---|
| Magic falsch | nichts: die 4 Bytes werden gelesen, aber nicht geprüft |
| Nutzlast > 0x9000 | `GF_ASSERT` setzt nur bei laufender Drahtlos-Kommunikation zurück; sonst liest das Spiel über den Heap-Puffer hinaus und überschreibt Speicher ohne Fehlermeldung |
| Länge ≠ 16 + Nutzlast | nichts: das Spiel liest nach den Zählern, zu wenig Bytes heißt, es liest in die nächsten Daten der NARC |
| Indizes außerhalb | ungeprüft, liest daneben (Vanilla: 0 Treffer) |
| `n_y` = 0 | Division durch 0 im Teiler der Hardware (Vanilla: 1 Platte in HeartGold) |
| Streifen unsortiert | die Binärsuche findet falsche Streifen, Höhen fehlen (Vanilla: 0) |
| keine Platte unter der Figur | die Figur behält ihre Höhe; für die Schrittprüfung gilt die Zielhöhe 0 |

Die Fehler haben auf allen Vanilla-Karten 0 Treffer; `n_y` = 0 ist darum nur eine Warnung (eine
Vanilla-Platte, HeartGold DE und US Karte 464).

## Round-Trip-Garantie

Für jede Datei, die `read` annimmt, gilt `write(read(bytes)) == bytes`. Der gated Test
`crates/g4-formats/tests/vanilla_bdhc.rs` bestätigt das für alle 2 018 Vanilla-BDHC (Platin DE
666, HeartGold DE 676, HeartGold US 676), dazu: kein Fehler aus `validate`, Warnungen Platin 0,
HeartGold je 1, größte Nutzlast Platin 21 452 B (Karte 188), HeartGold 10 778 B (Karte 658).

## Vanilla-Verteilung (B §2.2)

| je Karte: min / Median / p90 / max | Platin DE | HeartGold DE = US |
|---|---|---|
| Platten | 1 / 9 / 27 / 1 033 | 1 / 8 / 56 / 336 |
| **Nutzlast (B)** | **50 / 300 / 978 / 21 452** (Karte 188) | **50 / 311 / 1 672 / 10 778** (Karte 658) |

Die kleinste BDHC ist eine Platte über die ganze Karte (Nutzlast 50 B, `Bdhc::flat(h)` mit der
Höhe `h` in fx32); 173 Karten in Platin und 214 in HeartGold haben genau sie. Sie ist die
Vorgabe für eine leere gemalte Karte.
