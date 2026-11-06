# Gelände (Höhenraster, Gelände-Muster, abgeleitete Kollision)

Quelle: [Gutachten B](../research/2026-11-05-phase-6/gutachten-b-bdhc-engine.md) §2.4 und §2.7
(Messung auf Platin DE, HeartGold DE und US), Verhaltenswerte aus pret/pokeplatinum
`include/constants/field/map_tile_behaviors.h` und pret/pokeheartgold
`include/constants/metatile_behavior.h` (nur gelesen, Formatwissen), Zeilenformat nach
[Gutachten C](../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §5.2.
Implementierung: `crates/g4-kernel/src/terrain.rs` und `terrain/{rules,rows,derive,heights,apply,budget}.rs`,
Regeltabelle `crates/g4-kernel/metadata/terrain_rules.yaml`. Die BDHC selbst und der Generator
`from_grid` stehen in [bdhc.md](bdhc.md), die Permissions in [map.md](map.md).

Eine gemalte Karte besteht aus zwei Quellen: dem **Höhenraster** (wie hoch jede Kachel liegt)
und dem **Gelände-Muster** je Kachel (was die Figur dort tut). Aus dem Raster entsteht die BDHC
(`from_grid`), aus Raster und Muster die Kollision (`derive_permissions`), darüber legen sich die
Handwerte. Einheiten: 1 E = 4096 fx32, eine Kachel 16 E, eine Höhenstufe 8 E.

## Höhenraster

`HeightGrid` (aus `g4_formats::bdhc`, im Kernel re-exportiert): je Zelle `y * 32 + x` eine Höhe
in **Stufen** (`i8`, −16 bis 62, also −128 bis 496 E) und eine Rampe.

- **Ebene Zelle:** überall `8 h` E.
- **Rampe** `North/East/South/West(s)`: steigt in die genannte Richtung um `s` Stufen über die
  Kachel. `h` ist die Höhe der **unteren** Kante (gegenüber der Richtung). Kantenmitten: untere
  `8 h`, obere `8 h + 8 s`, die zwei Seitenkanten `8 h + 4 s`; Mitte `8 h + 4 s`.
- **Erlaubt** sind 1 oder 2 Stufen je Kachel (8 oder 16 E). Ab 20 E zwischen zwei Kachelmitten
  verweigert das Spiel den Schritt; eine Rampe von 3 Stufen (24 E) sperrte sich selbst. Die
  Oberkante `h + s` liegt höchstens bei 62.
- **Treppe:** eine Treppe 0 → 16 E über zwei Kacheln ist `East(1)` mit Höhen 0 und 1, ein
  Plateau dahinter Höhe 2. Eine Treppe hat **kein eigenes `kind`**, sie ist allein die Neigung
  der BDHC (`WARP_STAIRS_*` sind die Treppen, die die Karte wechseln).
- **Kante** zwischen zwei Nachbarn: Unterschied der beiden Kantenmitten (`edge_units`). Eine
  Rampe hat zu ihrer Fortsetzung keine Kante (beide Seiten gleich hoch).

## Gelände-Muster

`TerrainKind` mit Zeichen für die Zeilen und dem Permission-Wert je Familie
(`terrain_rules.yaml`; Diamant/Perl nehmen die Platin-Tabelle, `dp: { like: pt }`):

| Muster | Zeichen | `kind` | pret-Name | `collision` (Pt, HGSS) |
|---|---|---|---|---|
| `Blocked` (Wand, Baum, Haus, Fels) | `#` | 0x00 | NONE | 0x80 |
| `Ground` (Boden, Weg; Vorgabe) | `.` | 0x00 | NONE | 0x00 |
| `TallGrass` | `g` | 0x02 | TALL_GRASS | 0x00 |
| `VeryTallGrass` | `G` | 0x03 | VERY_TALL_GRASS | 0x00 |
| `Sea` | `~` | 0x15 | WATER_SEA | 0x00 |
| `River` | `r` | 0x10 | WATER_RIVER | 0x00 |
| `Ice` | `i` | 0x20 | ICE | 0x00 |
| `Ledge(North)` | `^` | 0x3A | JUMP_NORTH | 0x80 |
| `Ledge(East)` | `>` | 0x38 | JUMP_EAST | 0x80 |
| `Ledge(South)` | `v` | 0x3B | JUMP_SOUTH | 0x80 |
| `Ledge(West)` | `<` | 0x39 | JUMP_WEST | 0x80 |
| `Door` | `D` | 0x69 | DOOR | 0x80 |

`Ledge(d)` springt in Richtung `d`; der Sprung übergeht die Kollision (Vanilla 100 % blockiert).
Surfen und Begegnungen kommen aus dem Verhalten, nicht aus der Kollision. Hohes Gras braucht am
Header eine Begegnungsdatei mit Gras-Rate > 0.

`collision = sound | (blocked ? 0x80 : 0)`. `sound` ist das **Schrittgeräusch von HeartGold /
SoulSilver** (Bits 0–6 des zweiten Bytes, Index in die Geräuschtabelle des Spiels); Vorgabe 0
(81 % des freien Vanilla-Bodens), überall 0 in der Tabelle. Platin kennt dort nur 0x00 und 0x80.
Die Kollision ist in beiden Familien nur Bit 7. Ein Test hält jeden `name` gegen
`tile_behaviors.yaml` / `tile_behaviors_hgss.yaml` (HeartGold nennt 0x00 nicht, dort gilt NONE).

## Kollisionsableitung

`derive_permissions(grid, kinds, family)` setzt die 1 024 Zellen in dieser Reihenfolge:

1. **Muster:** jede Zelle ist die Regel ihres Musters (`kind_rule(family, kind).cell()`).
2. **Kanten:** jede Kante zwischen zwei Nachbarn **innerhalb der Karte**, je Paar einmal: ist der
   Unterschied der Kantenmitten mindestens `edge.free_below` (16 E, zwei Stufen), bekommt die
   Zelle mit der **höheren** Kante `| 0x80` (Schrittgeräusch-Bits bleiben). Eine Stufe (8 E)
   bleibt frei (Bordstein, Schwelle; Vanilla zu 42 % / 61 % frei). Zwei Stufen ließe die Engine
   zu, Vanilla blockiert sie zu 97 % / 95 %, öfter oben als unten (Entscheidung „obere Zelle“).
   Ab 24 E blockiert die Engine ohnehin; die Ableitung setzt trotzdem 0x80 (klar im Overlay,
   sicher für Figuren).
3. **Kartenränder** zählen nicht: die Nachbarkarte kennt die Ableitung nicht. Eine Zelle in
   Spalte 31 auf Höhe 5 neben Spalte 30 auf Höhe 0 wird über die Kante zur 30 blockiert, nicht
   über den Rand.
4. **Handwerte:** `apply_overrides(cells, overrides)` ersetzt je `CellOverride { x, y, kind,
   collision }` die ganze Zelle, spätere über frühere. Ein Wert außerhalb (x oder y ab 32) wird
   abgelehnt, bevor sich eine Zelle ändert.

## Zeilenformat

Wie die Permissions und das Weltformat (Gutachten C §5.2): **32 Zeilen** (Zeile `y`, ab 0) zu je
**32 Zellen** (Spalte `x`, ab 0), getrennt durch ein Leerzeichen (beim Lesen zählt jeder
Leerraum als Trenner; `format_*` schreibt genau eines).

| Feld | Zelle | Beispiel |
|---|---|---|
| `grid.heights` | ganze Zahl (Stufen) | `0 0 2 2 -1 …` |
| `grid.ramps` | `.` oder Richtung bergauf `n e s w` und Stufen je Kachel: `n1 n2 e1 e2 s1 s2 w1 w2` | `. . e1 e1 . …` |
| `kinds` | ein Zeichen aus `# . g G ~ r i ^ > v < D` | `# . . g g …` |

`heights` und `ramps` sind wörtlich die Grammatik des Weltformats (6.3a, `layers[0].heights`,
`ramps`). Eine Rampe mit anderer Stufenzahl (`e3`) ist lesbar und wird von der Rasterprüfung mit
eigenem Satz abgelehnt. Ablehnungen (`KernelError::Refused`, deutsch, mit Ort):

| Fall | Satz |
|---|---|
| Zeilenzahl | `heights: 32 Zeilen erwartet, 30 da.` |
| Zellenzahl | `heights: Zeile 4 hat 31 Zellen statt 32.` |
| keine Zahl | `heights: Zeile 4, Spalte 7: „x“ ist keine ganze Zahl.` |
| keine Rampe | `ramps: Zeile 2, Spalte 5: „q1“ ist keine Rampe (erlaubt: . n1 n2 e1 e2 s1 s2 w1 w2).` |
| kein Gelände | `kinds: Zeile 2, Spalte 5: „Q“ ist kein Gelände (erlaubt: # . g G ~ r i ^ > v < D).` |
| Höhe | `Kachel x 4, y 6: Höhe 63 liegt außerhalb von −16 bis 62 Stufen.` |
| Rampe zu steil | `Kachel x 4, y 6: Rampe mit 3 Stufen je Kachel (24 E); erlaubt sind 1 oder 2 Stufen (8 oder 16 E), ab 20 E verweigert das Spiel den Schritt.` |
| Rampe zu hoch | `Kachel x 4, y 6: die Rampe endet bei 63 Stufen, über 62.` |
| Handwert | `overrides: Kachel x 32, y 0 liegt außerhalb der Karte (0 bis 31).` |

Die Zeilen- und Zellenzahl gilt ebenso für `ramps` und `kinds`. Die Rasterprüfung meldet die
erste Zelle in Indexreihenfolge (Höhe, dann Stufen, dann Oberkante).

## Höhen der Kachelmitten

`map_heights(bdhc)` gibt je Kachel die Laufhöhe (fx32) an der Kachelmitte (`tile_center`), mit
`current = i32::MAX`: es gewinnt der **höchste Kandidat**, das obere Deck einer Brücke, wie die
Draufsicht es sieht (Vanilla hat viele Kachelmitten mit mehreren Kandidaten, B §2.2). `None`, wo
keine Platte unter der Mitte liegt; eine unlesbare BDHC ist ein `FormatError`. Platin-Karte 100
(Vanilla): Boden 32 E, dazu 96 E (B §2.4).

## Freie Stufen

`free_steps(map)` findet Kanten der BDHC, an denen die Figur zwei Stufen hochsteigen kann, obwohl
es wie eine Wand aussieht (Grundlage von `map.step-unblocked`). Je Nachbarpaar innerhalb der
Karte:

1. **Vorfilter:** Mitten `c_a`, `c_b` (wie `map_heights`); nur wenn
   `prefilter − tolerance ≤ |c_a − c_b| < engine_limit` (7,9 bis unter 20 E: das Spiel lässt den
   Schritt zu) und beide Zellen ohne 0x80.
2. **Kantenhöhe je Seite:** eine Probe 1 E innen von der Kantenmitte, mit der eigenen Mitte als
   `current`, und von der Mitte aus linear bis zur Kante verlängert:
   `e = c + (Probe − c) · 8 / 7`. Auf einer ebenen Platte ist das die Probe; auf einer Rampe die
   Höhe ihrer Ebene an der Kante. Ohne die Verlängerung läge eine Stufe von 16 E an der unteren
   Kante einer Rampe mit 2 Stufen (Mitten nur 8 E auseinander) bei 15 E und fiele durch.
3. **Befund:** `|e_a − e_b| ≥ free_below − tolerance` (15,9 E). `upper` ist die höhere Seite,
   `step` der Unterschied an der Kante, `centres` der der Mitten (beide fx32). Sortiert nach
   `(upper, lower)`.

Eine 16-E-Rampe über zwei Zellen hat Mitten 8 E auseinander, aber an der Kante keinen Unterschied
(kein Befund); eine Stufe von 24 E hat Mitten 24 E auseinander, die das Spiel ohnehin sperrt.
Kartenränder zählen nicht.

## Höhenraster setzen

`set_map_terrain(store, map, input)` (Befehl `set_map_terrain`, MCP gleichnamig) liest die Zeilen
(`parse_grid`, `parse_kinds`), baut die BDHC (`from_grid`, mit `bdhc: flat` stattdessen
`Bdhc::flat(0)`), leitet die Permissions ab (`derive_permissions`, mit `permissions: keep` die
der Karte) und legt die Handwerte darüber. Jede Ablehnung (Zeilen, Raster, Handwert) fällt,
bevor etwas geschrieben ist; dann schreibt **ein** Schritt „set terrain“ (`Store::edit_map`)
BDHC und Permissions zusammen, `undo` nimmt beide zurück. Der Bericht `TerrainReport` nennt
Länge, Nutzlast, Platten und Streifen der BDHC, die blockierten Zellen, die Zahl der Handwerte
und `permissions_kept`. Die Höhen der Kachelmitten trägt `get_map` als `heights` (E, `null` ohne
Platte und überall bei unlesbarer BDHC); `get_map_heights` nennt dazu den Grund des Lesers.

## Budget je Karte

`map_budget(store, map)` (Befehl `get_map_budget`) misst die Karte gegen die Grenzen aus
`check_limits.yaml` (B §2.8). Je Posten `{used, max, state}`; der Zustand ist **Voll** über
`max`, **Knapp** ab 90 % (`10 · used ≥ 9 · max`), sonst **Ruhig** (`budget_state`). Das Budget
zeigt nur an; Speichern blockiert es nie.

| Posten | `used` | `max` |
|---|---|---|
| `model` | Länge des Modells | `model.max_bytes` 0xF000 = 61 440 B |
| `bdhc` | Nutzlast aus den Zählern des Kopfs (`8 P + 12 N + 4 C + 8 Pl + 8 S + 2 A`), 0 ohne Kopf | `bdhc.max_bytes` 0x9000 = 36 864 B; ein Raster bleibt unter 35 592 B |
| `buildings` | Zahl der Gebäude | `buildings.max_loaded` 32 (32 sind Knapp, ab 33 Voll) |
| `textures.vram` | Schätzung von Karten- plus Gebäudepaket des Gebiets | `vram.bank` 262 144 B |

**Gebiet:** das der Gebietsdaten des ersten Headers in Tabellenreihenfolge, dessen Matrix die
Karte zeigt (eine Zelle mit der Karte und, wo die Matrix einen Header-Abschnitt hat, mit diesem
Header; dieselbe Regel wie die Texturpakete des Kartenimports). Kartenpaket `map_tileset`,
Gebäudepaket `buildings_tileset` der Gebietsdaten; `textures` ist die Zahl der Texturen im
Kartenpaket. Ohne Gebiet ist `textures` `null` und `textures_missing` sagt warum: „Kein Header
zeigt diese Karte.“, „Gebietsdaten 12 fehlen.“, „Texturpaket 6 nicht lesbar: <Grund>.“ bzw.
„Gebäude-Texturpaket 6 nicht lesbar: <Grund>.“ (der Grund des NSBTX-Lesers oder „liegt außerhalb
des Archivs“).

**VRAM-Schätzung** (`vram_bytes`): je Textur `Breite · Höhe · bpp / 8` mit bpp 8, 2, 4, 8, 3, 8,
16 für Format 1 bis 7 (A3I5, 4 Farben, 16, 256, 4×4 mit Indexdaten, A5I3, direkt), dazu zwei
Byte je Palettenfarbe. Der Zustand folgt nicht der 90-%-Regel: **Knapp** über dem
Vanilla-Maximum der Familie (`vram.vanilla_max`: Platin 110 640 B, HGSS 87 424 B, A §2.4),
**Voll** über `vram.full_estimate` 204 800 B. **Grenzen der Schätzung:** Texel, die zwei
Texturen teilen, zählen doppelt; die Ausrichtung im VRAM und der Anteil der Figuren an der Bank
sind nicht gemessen (die Bank teilen Karte, Gebäude und Figuren); Polygone und Vertices fehlen
(sie folgen mit dem Modell-Codec in 6.3b). Ein Paket, das nicht passt, bricht im Spiel beim
Hochladen ab (A §2.4); die Schätzung ist eine Warnung, keine Prüfung.

## Quellen

- Gutachten B §2.4 (Ablauf je Schritt, Permissions je Familie, Schrittgeräusch in HGSS,
  Karte 100) und §2.7 (Kantenzählung in Vanilla, Regelsatz).
- pret/pokeplatinum `include/constants/field/map_tile_behaviors.h`, pret/pokeheartgold
  `include/constants/metatile_behavior.h` (Verhaltenswerte und Namen).
- Gutachten B §2.8 (Grenzen und Budget je Karte), Gutachten A §2.4 (VRAM der Texturpakete).
- Gutachten C §5.2 (Zeilen wie die Permissions).
- Entscheidungen der Phase 6.2 im Plan `docs/superpowers/plans/2026-11-05-phase-6-2-bdhc.md`
  („Regeltabelle“, „Kollision zusammensetzen“, „Zeilenformat des Befehls“, „current im Studio“,
  „map.step-unblocked auf binären Karten“, „Name und Form „Höhenraster setzen““, „Höhen in der
  Szene“, „Budget“).
