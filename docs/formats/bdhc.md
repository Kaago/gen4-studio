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

## Erzeugen aus dem Höhenraster

Implementierung: `crates/g4-formats/src/bdhc/grid.rs` (`HeightGrid`, `from_grid`,
`from_planes`), Algorithmus und Größen aus dem eigenen Prototyp `gutachten-b-bdhc-bau.py`
(`rectangles`, `plane_fx`, `build`, B §2.6). `from_planes` schreibt dieselben Bytes wie
`build`: gegen den Prototyp verglichen für die Probe, die Beispielraster und 1 004
Zufalls- und Extremraster, alle byte-gleich.

### Raster

`HeightGrid { height: [i8; 1024], ramp: [Ramp; 1024] }`, Index `y * 32 + x` wie die
Permissions (Zeile `y` wächst nach Süden, also nach +z). Höhen in **Stufen** zu `STEP_UNITS` =
8 E (eine Stufe ist eine halbe Kachel). Eine Zelle ist flach (`Ramp::None`) oder eine Rampe
`North(s)`, `East(s)`, `South(s)`, `West(s)`: sie steigt in diese Himmelsrichtung um `s` Stufen
über die Kachel. `height` einer Rampe ist die Höhe ihrer **unteren** Kante (gegenüber der
Richtung). Kantenhöhen (`edge_units`, Mitte der Kante) und Mitte (`center_units`) in E:

```
             Nord: 8h + 4s
           +---------------+
           |               |
 West: 8h  |  East(s) ->   |  Ost: 8h + 8s       Mitte: 8h + 4s
           |               |
           +---------------+
             Süd: 8h + 4s
```

Eine flache Zelle hat überall `8h`. Eine Treppe 0 → 16 E über zwei Zellen ist `East(1)` mit den
Höhen 0 und 1, ein Plateau danach Höhe 2; beide Rampenzellen haben dieselbe Ebene und werden ein
Rechteck.

`HeightGrid::check` nennt die erste falsche Zelle in Indexreihenfolge (`GridProblem`), je Zelle
in dieser Reihenfolge: `height` außerhalb `MIN_STEP..=MAX_STEP` = −16..=62
(`HeightOutOfRange`), Rampenstufen nicht 1 oder 2 (`RampSteps`, drei Stufen wären 24 E je
Kachel, das Spiel verweigert ab 20 E), Oberkante `height + s` über 62 (`RampTop`).
`from_grid` bleibt total und rechnet auch ein falsches Raster; der Kernel lehnt vorher ab.

### Ebene je Zelle

`HeightGrid::plane(i)` gibt den ganzzahligen Schlüssel `CellPlane { c, kx, kz }` mit
`2 · y = c + kx · x + kz · z` (E ab der Kartenmitte, `CellPlane::units_at`). Mit den Kanten der
Zelle `x_w = 16 x − 256`, `x_e = x_w + 16`, `z_n = 16 y − 256`, `z_s = z_n + 16`:

| Zelle | `(c, kx, kz)` |
|---|---|
| flach | `(16h, 0, 0)` |
| `East(s)` | `(16h − s · x_w, s, 0)` |
| `West(s)` | `(16h + s · x_e, −s, 0)` |
| `South(s)` | `(16h − s · z_n, 0, s)` |
| `North(s)` | `(16h + s · z_s, 0, −s)` |

### Algorithmus (B §2.6)

1. **Rechtecke** je gleicher Ebene (`CellPlane`-Gleichheit): zeilenweise die erste freie Zelle
   nehmen, nach rechts wachsen, solange die Ebene gleich ist, dann nach unten, solange die ganze
   Breite gleich ist. Greedy, nicht minimal, aber stabil: gleiches Raster, gleiche Bytes.
2. **Ebene → fx32**: mit `L = √(1 + (kx/2)² + (kz/2)²)` ist
   `n = round((−kx/2, 1, −kz/2) / L · 4096)` und `d = round(−c/2 / L · 4096)` (`f64::round`).
3. **Tabellen**: je Rechteck eine Platte aus der nordwestlichen und der südöstlichen Ecke;
   Punkte, Normalen und Konstanten werden in Plattenreihenfolge entdoppelt.
4. **Streifen** über die sortierten z-Kanten aller Rechtecke: das Band zwischen zwei
   benachbarten Kanten wird ein Streifen mit `scanline` = obere (südliche, größere) Kante. Seine
   Zugriffsliste enthält nur die Rechtecke, die das Band **echt überlappen**
   (`z1 < oben && z2 > unten`). Das reicht, weil ein Punkt genau auf einer Kante dem Streifen
   darunter zufällt (erster mit `scanline > z`), und dort liegt das Rechteck, das an dieser
   Kante beginnt.

### Schranke

Alle Ecken liegen auf Kachelgrenzen, also höchstens 33 × 33 = 1 089 Punkte; höchstens 1 024
Platten (eine je Zelle), so viele Konstanten und Normalen (N ≤ 1 024); höchstens 33 z-Kanten, also
32 Streifen. Wegen der halboffenen Bänder steht ein Rechteck in höchstens so vielen Streifen,
wie es Zeilen hat, und das ist nicht mehr, als es Zellen hat: zusammen höchstens 1 024 Zugriffe.

```
Nutzlast ≤ 8 712 + 12 N + 4 096 + 8 192 + 256 + 2 048 = 23 304 + 12 N ≤ 35 592 < 36 864 (0x9000)
```

`MAX_PAYLOAD_FROM_GRID` = 35 592. Eine so erzeugte BDHC kann den Puffer des Spiels nicht
überlaufen, für jedes Raster und jedes Ebenenfeld von `from_planes`; das Budget „BDHC“ ist für
gemalte Karten nur Anzeige. Mit geschlossenen Bändern (ein Rechteck auch in den Streifen, die
nur seine Kante berühren, wie in Vanilla oft) wären es bis zu 3 072 Zugriffe und mit vielen
Neigungen knapp über 0x9000. Der Eigenschaftstest `generated_bdhc_never_exceeds_the_bound`
rechnet 1 004 Raster; die größte Nutzlast ist 19 744 B.

### Größen (B §2.6, Nutzlast ohne Kopf)

| Raster | Platten | Streifen | Zugriffe | Nutzlast | Anteil an 0x9000 |
|---|---|---|---|---|---|
| flach (`Bdhc::flat(0)`) | 1 | 1 | 1 | 50 B | 0,1 % |
| Route: 3 Ebenen, 4 Treppen | 13 | 5 | 13 | 422 B | 1,1 % |
| Stadt: 6 Dächer, Teich | 20 | 7 | 43 | 646 B | 1,8 % |
| Probe (Emulator) | 9 | 5 | 12 | 324 B | 0,9 % |
| Flecken: 4 Stufen, zufällig, geglättet | 412 | 32 | 958 | 11 232 B | 30,5 % |
| Schachbrett 0/16 | 1 024 | 32 | 1 024 | 19 212 B | 52,1 % |
| jede Kachel eigene Höhe (`from_planes`) | 1 024 | 32 | 1 024 | 23 300 B | 63,2 % |
| jede Kachel Rampe, 8 Neigungen (`from_planes`, bis 32 E je Kachel) | 1 024 | 32 | 1 024 | 21 956 B | 59,6 % |

Flach, Route, Probe, Schachbrett und „jede Kachel eigene Höhe“ prüfen die Tests von `grid.rs`
mit genau diesen Zahlen.

### Rundung

Die Höhensuche des Spiels trifft auf den erzeugten Platten an jeder Kachelmitte und an vier
Punkten 4 E daneben die Sollhöhe bis auf die Rundung von `n` und `d` auf 1/4096. In den
Beispielrastern des Gutachtens sind das höchstens **0,06 E** (die Rampenmitte der Probe:
3,989 E statt 4). Der Fehler wächst mit der Höhe, weil `n_y` gerundet ist: an einer
Zwei-Stufen-Rampe auf 488 E sind es 0,079 E, das Höchste der 1 004 Testraster
(`generated_heights_hit_five_points_per_cell`, Grenze 0,1 E). Flache Platten haben keinen
Fehler.
