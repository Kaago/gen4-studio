# NCGR und NCLR (Kachelgrafik und Palette)

Quelle: öffentliche Formatbeschreibungen (gbatek, NDS File Formats) und der Augenschein an den Archiven von
Platin DE, HeartGold DE und HeartGold US (Dateikopf und Blöcke wie unten, 1:1 geprüft).
Implementierung: `crates/g4-formats/src/ncgr.rs` und `nclr.rs` (nur Lesen, nichts schreibt sie zurück),
Symbole als PNG in `crates/g4-kernel/src/data/icons.rs`. Alle Werte Little Endian.

## NCLR (`RLCN`)

| Offset | Inhalt |
|--------|--------|
| 0x00 | `b"RLCN"`, `u16 0xFEFF` (BOM), `u16 0x0100` (Version), `u32` Dateigröße, `u16 0x10` (Kopfgröße), `u16` Blockzahl |
| 0x10 | Block `b"TTLP"`, `u32` Blockgröße |
| 0x18 | `u32` Tiefe (3: 4 bpp, 4: 8 bpp) |
| 0x1C | `u32` Füllwert |
| 0x20 | `u32` Datengröße in Bytes |
| 0x24 | `u32` Offset der Farben, gezählt ab 0x18 (hinter Magic und Größe des Blocks): immer `0x10` |
| 0x28 | Farben: `u16` je Farbe, 15 Bit BGR (Rot in den unteren fünf Bits) |

`Nclr::read` liefert `palettes: Vec<[u16; 16]>`, 16 Farben je Palette. Eine Datei mit 8 bpp wird als 16 Paletten
zu 16 gelesen. Der Dekoder prüft nur Magics und Längen: abgeschnittene Daten sind `Truncated`, eine Datengröße,
die kein Vielfaches von 32 Bytes ist, `BadSize`. Tiefe, Füllwert und weitere Blöcke bleiben unbeachtet.

Die Umrechnung von RGB555 nach RGB8 ist die des NSBTX-Dekoders aus 3b.1 (`nsbtx::rgb`): die fünf Bits werden
nach oben gespiegelt (`(v << 3) | (v >> 2)`), `0x7FFF` ergibt (255, 255, 255). Das weicht um eins von einer
Umrechnung `v * 255 / 31` ab (ein Grünwert 25 wird 206 statt 205).

## NCGR (`RGCN`)

| Offset | Inhalt |
|--------|--------|
| 0x00 | `b"RGCN"`, `u16 0xFEFF`, `u16 0x0101`, `u32` Dateigröße, `u16 0x10`, `u16` Blockzahl |
| 0x10 | Block `b"RAHC"`, `u32` Blockgröße |
| 0x18 | `u16` Kacheln in der Höhe (`0xFFFF`: offen) |
| 0x1A | `u16` Kacheln in der Breite (`0xFFFF`: offen) |
| 0x1C | `u32` Tiefe (3: 4 bpp, 4: 8 bpp) |
| 0x20 | `u32` Mapping (`0x10`) |
| 0x24 | `u32` Kachelmodus (0: gekachelt) |
| 0x28 | `u32` Datengröße in Bytes |
| 0x2C | `u32` Offset der Daten, gezählt ab 0x18: immer `0x18` |
| 0x30 | Kacheln, je 8 × 8 Pixel, Zeile für Zeile |

Eine 4-bpp-Kachel hat 32 Bytes, das niedrige Halbbyte ist der linke Pixel; eine 8-bpp-Kachel hat 64 Bytes.
Die Kacheln liegen in 1D-Reihenfolge: Kachel `n` sitzt in Spalte `n % Breite` und Zeile `n / Breite` des Bildes.

`Ncgr::read` liefert `width` und `height` in Pixeln, `bpp` (4 oder 8) und die rohen Kachelbytes. Alle
Symbole im ROM tragen `0xFFFF` als Kachelzahl, denn die Größe steht erst in den NCER-Zellen. Der Dekoder nimmt dann
eine Breite von vier Kacheln (32 Pixel) an, so groß sind die Symbole beider Archive, und leitet die Höhe aus den
Daten ab (bei weniger als vier Kacheln eine Zeile). Kachelmodus und weitere Blöcke bleiben unbeachtet.
Fehler: falsche Magics, abgeschnittene Daten (`Truncated`, auch wenn der Kopf mehr Kacheln nennt, als die Daten
enthalten), eine Datengröße, die kein Vielfaches einer Kachel ist (`BadSize`), und eine Tiefe außer 3 und 4
(`Unsupported`).

`render_rgba(ncgr, palette)` liefert `width * height * 4` Bytes RGBA. Palettenindex 0 ist durchsichtig (alle
Bytes null, auch wenn die Palette dort eine Farbe hat). Bei 8 bpp sind Indizes ab 16 ebenfalls durchsichtig,
weil die Palette 16 Farben hat.

## Item-Symbole (`itemIcons`)

Pt `files/itemtool/itemdata/item_icon.narc`, HGSS `files/a/0/1/8`, DSPRE-Ordner `itemIcons`. Member 0 und 1 sind
NANR und NCER, danach folgen NCGR (560 Bytes: 32 × 32, 4 bpp) und NCLR (552 Bytes: 16 Paletten) im Wechsel.
Die arm9-Item-Tabelle (`Project::item_table`) nennt je Gegenstand den NCGR-Member (`icon`) und den NCLR-Member
(`palette`); es gilt die erste Palette des NCLR. In allen drei Projekten nutzt Gegenstand 17 (Trank) die Member
24 und 25. `icons::item_icon(project, id)` rendert das Symbol als 32 × 32 PNG und liefert `None`, wenn die Item-Tabelle
fehlt, die Nummer außerhalb liegt oder der Member fehlt.

## Pokémon-Symbole (`monIcons`)

Pt `files/poketool/icongra/pl_poke_icon.narc` (547 Member), HGSS `files/a/0/2/0` (551 Member), DSPRE-Ordner
`monIcons`. Member 0 ist ein NCLR mit 16 Paletten (genutzt werden 0 bis 2), Member 1 bis 6 sind NANR und NCER, ab
Member 7 folgt je Symbol ein NCGR zu 1072 Bytes: 32 × 64 Pixel, 4 bpp, zwei Bilder von 32 × 32 untereinander
(das zweite ist das Wippen). `icons::mon_icon` liefert das erste Bild als 32 × 32 PNG.

Die Palette steht in der arm9-Tabelle `Project::mon_icon_palettes` (Werte 0 bis 2). Sie ist nach dem Symbol-Index
geordnet (NCGR-Member − 7), nicht nach der Personal-Nummer. Für die Arten 0 bis 493 sind beide gleich; für Ei und
Formen gilt `FORM_ICON_MEMBERS` (`icons::mon_icon_source`):

| Personal-Member | NCGR-Member | Palettentabelle |
|---|---|---|
| 0 bis 493 | Member + 7 | `table[Member]` |
| 494 (Ei) | 501 | `table[494]` |
| 495 (Bad Egg) | 7 (das Symbol „?“) | `table[0]` |
| 496 bis 498 (Deoxys) | 503 bis 505 | `table[496..=498]` |
| 499, 500 (Burmadame) | 536, 537 | `table[529]`, `table[530]` |
| 501 (Giratina, Urform) | 540 | `table[533]` |
| 502 (Shaymin, Zenitform) | 541 | `table[534]` |
| 503 bis 507 (Rotom) | 542 bis 546 | `table[535..=539]` |

Darüber (ab 508) gibt es kein Symbol. HGSS hat vier weitere Symbole (Formeo, Kinoso), die keine Personal-Nummer
haben und hier nicht erreichbar sind. Die Zuordnung ist belegt durch Task 0 (`docs/research/2026-10-23-3b3/task0-3b3a.md`
§1.2) und durch den Augenschein an Platin DE: Ei, „?“, die drei Deoxys-Formen, die Umhänge, Giratina (Urform), Shaymin
(Zenitform) und die fünf Rotom-Geräte erscheinen an der erwarteten Stelle. In allen drei Projekten nutzt Bisasam
Palette 1 (häufigste Farbe (123, 156, 74)), Glumanda und Schiggy nutzen Palette 0, und jede Art und Form von 0 bis 507
liefert ein nicht leeres Symbol.
