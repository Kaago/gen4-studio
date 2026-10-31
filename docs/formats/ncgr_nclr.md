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
| 0x24 | `u32` Kachelmodus (0: gekachelt, 1: linear, Pixel für Pixel zeilenweise) |
| 0x28 | `u32` Datengröße in Bytes |
| 0x2C | `u32` Offset der Daten, gezählt ab 0x18: immer `0x18` |
| 0x30 | Kacheln, je 8 × 8 Pixel, Zeile für Zeile |

Eine 4-bpp-Kachel hat 32 Bytes, das niedrige Halbbyte ist der linke Pixel; eine 8-bpp-Kachel hat 64 Bytes.
Die Kacheln liegen in 1D-Reihenfolge: Kachel `n` sitzt in Spalte `n % Breite` und Zeile `n / Breite` des Bildes.

`Ncgr::read` liefert `width` und `height` in Pixeln, `bpp` (4 oder 8) und die rohen Kachelbytes. Alle
Symbole im ROM tragen `0xFFFF` als Kachelzahl, denn die Größe steht erst in den NCER-Zellen. Der Dekoder nimmt dann
eine Breite von vier Kacheln (32 Pixel) an, so groß sind die Symbole beider Archive, und leitet die Höhe aus den
Daten ab (bei weniger als vier Kacheln eine Zeile). Der Kachelmodus landet in `Ncgr.mode` (`TileMode::Tiled` oder
`Linear`), weitere Blöcke bleiben unbeachtet. Fehler: falsche Magics, abgeschnittene Daten (`Truncated`, auch wenn der Kopf mehr Kacheln nennt, als die Daten
enthalten), eine Datengröße, die kein Vielfaches einer Kachel ist (`BadSize`), und eine Tiefe außer 3 und 4
und ein Kachelmodus außer 0 und 1 (beide `Unsupported`).

`render_rgba(ncgr, palette)` liefert `width * height * 4` Bytes RGBA. Palettenindex 0 ist durchsichtig (alle
Bytes null, auch wenn die Palette dort eine Farbe hat). Bei 8 bpp sind Indizes ab 16 ebenfalls durchsichtig,
weil die Palette 16 Farben hat. Im Modus `Linear` liegt Pixel `(x, y)` im Halbbyte `y * Breite + x` (bei 8 bpp im Byte
`y * Breite + x`, das niedrige Halbbyte ist wie gekachelt der linke Pixel); es gibt keine Kachelrechnung. Daten, die
früher enden, lassen den Rest durchsichtig.

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

## Kampfsprites (`pokemonBattleSprites`, `otherPokemonBattleSprites`, `trainerGraphics`)

Alle Archive sind in Platin DE, HeartGold DE und HeartGold US gleich gebaut (gemessen, Gutachten C §3.1 in
`docs/research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md`).

| Archiv | Platin (`files/poketool/…`) | HeartGold/SoulSilver | DSPRE-Ordner | Inhalt |
|---|---|---|---|---|
| Pokémon | `pokegra/pl_pokegra.narc` | `a/0/0/4` | `pokemonBattleSprites` | 2 964 Member = 494 Arten × 6: vier NCGR (6 448 Byte), zwei NCLR (72 Byte) |
| Formen und Eier | `pokegra/pl_otherpoke.narc` | `a/1/1/4` | `otherPokemonBattleSprites` | Platin 253 Member (157 NCGR, 96 NCLR), HGSS 261 (161, 100), eigene Reihenfolge |
| Lage im Kampf | `poke_edit/pl_poke_data.narc` | `a/1/8/0` | `pokemonSpriteOffsets` | 494 × 89 Byte, ungeprüft und nicht gelesen |
| Trainer | `trgra/trfgra.narc` | `a/0/5/8` | `trainerGraphics` | 5 Member je Klasse (Platin 105, HGSS 129) |

**Plätze je Art.** Member `6 · s + Platz`: Platz 0 Rückseite weiblich, 1 Rückseite männlich oder einzig, 2 Vorderseite
weiblich, 3 Vorderseite männlich oder einzig; Palette `6 · s + 4` normal, `6 · s + 5` schillernd (je ein NCLR mit
16 Farben). Ein leeres Member (0 Byte) heißt, dass es kein eigenes Bild gibt (66 Arten ohne Platz 0 und 2, 16 ohne
Platz 1 und 3). Die Formen im zweiten Archiv folgen einer eigenen Ordnung (Abschnitt „Formen“).

**Bild.** Jedes Bild-NCGR hat 20 × 10 Kacheln, 4 bpp, Kachelmodus 1: 160 × 80 Pixel, zwei Bilder zu 80 × 80
nebeneinander, links das Standbild, rechts das Animationsbild. Alle Symbole dagegen haben Modus 0.

**Formen.** `otherPokemonBattleSprites` hat erst alle Bilder, dann alle Paletten (je ein NCLR mit einer Palette):
Platin Member 0–153 NCGR, 154–247 NCLR, dann 248 NCGR, 249 NCGR, 250 NCLR, 251 NCGR, 252 NCLR; HeartGold/SoulSilver
0–157 NCGR (vier Pichu-Bilder mehr), 158–255 NCLR, dann 256, 257 NCGR, 258 NCLR, 259 NCGR, 260 NCLR. Die Bilder
stimmen in beiden Familien überein (außer der Wandelform von Giratina, 150 und 151, die HGSS neu zeichnet), die
Paletten auch, nur liegt in HGSS jede Palette vier Member höher. Die Zuordnung ist am Augenschein belegt (Platin DE,
HeartGold DE; HeartGold US ist byte-gleich mit HeartGold DE; Protokoll
`docs/research/2026-10-30-phase-3d/task3-3d3-formen.md`) und steht je Familie in `forms` von
`crates/g4-kernel/metadata/battle_sprites.yaml`. `f` ist der Formindex des Spiels; Form 0 ist die Grundform und kommt
aus `pokemonBattleSprites` (ihre Bilder liegen hier auch, das Studio nutzt sie nicht). Keine Form hat eigene
weibliche Bilder. Alle Einträge sind belegt, keiner ist `unverified`.

| Art | Form `f` | Personal | Rückseite / Vorderseite | Palette normal / schillernd, Platin | HGSS | Status |
|---|---|---|---|---|---|---|
| Deoxys (386) | 1 Angriffs-, 2 Verteidigungs-, 3 Initiativeform | 496–498 | `2f` / `2f + 1` | 154 / 155 (alle Formen) | 158 / 159 | belegt |
| Icognito (201) | 1–25 B–Z, 26 „!“, 27 „?“ | – | `8 + 2f` / `9 + 2f` | 156 / 157 (alle Formen) | 160 / 161 | belegt |
| Formeo (351) | 1 Sonnen-, 2 Regen-, 3 Schneeform | – | `64 + f` / `68 + f` | `158 + f` / `162 + f` | + 4 | belegt |
| Burmy (412) | 1 Sand-, 2 Lumpenumhang | – | `72 + 2f` / `73 + 2f` | `166 + 2f` / `167 + 2f` | + 4 | belegt |
| Burmadame (413) | 1 Sand-, 2 Lumpenumhang | 499, 500 | `78 + 2f` / `79 + 2f` | `172 + 2f` / `173 + 2f` | + 4 | belegt |
| Schalellos (422) | 1 Östliches Meer | – | 85 / 87 | 180 / 181 | 184 / 185 | belegt |
| Gastrodon (423) | 1 Östliches Meer | – | 89 / 91 | 184 / 185 | 188 / 189 | belegt |
| Kinoso (421) | 1 Sonnenform | – | 93 / 95 | 187 / 189 | 191 / 193 | belegt |
| Arceus (493) | 1–17 Typ (Typindex, 9 ist „???“) | – | `96 + 2f` / `97 + 2f` | `190 + 2f` / `191 + 2f` | + 4 | belegt |
| Ei | 0 | 494 | – / 132 | 226 / – | 230 / – | belegt |
| Shaymin (492) | 1 Zenitform | 502 | 136 / 137 | 230 / 231 | 234 / 235 | belegt |
| Rotom (479) | 1 Hitze, 2 Wasch, 3 Frost, 4 Wirbel, 5 Schneid | 503–507 | `138 + 2f` / `139 + 2f` | `232 + 2f` / `233 + 2f` | + 4 | belegt |
| Giratina (487) | 1 Urform | 501 | 152 / 153 | 246 / 247 | 250 / 251 | belegt |
| Pichu (172), nur HGSS | 1 Strubbelohr | – | 156 / 157 | – | 254 / 255 | belegt |

Das Ei hat nur eine Vorderseite und eine Palette: seine Rückseite ist kein Bild, schillernd zeigt die normale Palette.
Nicht in der Tabelle: das Manaphy-Ei (Platin 133 mit Palette 227, HGSS 133 mit 231; keine Form, das Spiel wählt es
nach der Art im Ei), das Bildpaar 248/249 mit Palette 250 (HGSS 256/257, 258; Rückseite und Vorderseite eines kleinen
grünen Pokémon, Zweck unbekannt) und 251 mit Palette 252 (HGSS 259, 260: drei flache Ellipsen, wohl Schatten).
Diamant und Perl haben keine Tabelle (Familie ungeprüft, `battle_sprite` liefert dort nichts).

Auflösung (`data::battle_sprites::battle_sprite`): eine Art ab 494 ist ein Personal-Member und wird über die
Spalte „Personal“ zu Art und Form (503 ist Rotom, Form 1; das Ei bleibt 494, Form 0); dann gilt der Eintrag der
Familie für Art und Form, sonst die Grundform aus `pokemonBattleSprites`. Fehlt das Bild eines Eintrags im Archiv,
zeigt das Studio ebenfalls die Grundform; das Ei hat keine.

**Trainer.** Je Klasse `k` fünf Member: Member `5 · k + 4` ist das lineare NCGR (160 × 80, die linke Hälfte ist das
Bild), die Palette steht in Member `5 · k + 1` (NCLR mit 16 Paletten, genutzt wird die erste); die Member `5 · k` (NCGR
für Zellen), `5 · k + 2` (NCER) und `5 · k + 3` (NANR) liest das Studio nicht.

**Verschlüsselung.** Die Pixelwörter (`u16`, Little Endian) sind mit einem linearen Kongruenzgenerator verschlüsselt,
`decrypt` in `ncgr.rs`: der Schlüssel `k` beginnt mit dem ersten Wort der Richtung, jedes Wort wird mit
`k & 0xFFFF` verknüpft (XOR), danach gilt `k = k · 0x41C64E6D + 0x6073` (32 Bit, mit Überlauf). Das Startwort selbst
wird so zu 0 (der Klartext ist eine durchsichtige Ecke), sein verschlüsselter Wert ist der Schlüssel. Die Richtung ist
eine Eigenschaft je Archiv und Spielfamilie:

| Archiv | Platin | HeartGold/SoulSilver |
|---|---|---|
| Pokémon | vorwärts | vorwärts |
| Formen und Eier | vorwärts | vorwärts |
| Trainer | vorwärts | rückwärts (Startwort ist das letzte Wort, die Schleife läuft rückwärts) |

Diamant und Perl sind ungemessen.

**Quelle.** Eigene Messung, keine fremde Beschreibung: die Skripte `gutachten-c-sprites-krypto.py`,
`gutachten-c-sprites-inventar.py` und der Prototyp `gutachten-c-proto.rs` liegen neben dem Gutachten. Über alle 1 812
Pokémon-Bilder (Modus 1) lesen sich in der richtigen Richtung 68,7 % der Pixel als durchsichtig und 1 778 von 1 812
Bildern haben eine durchsichtige Ecke; in der falschen Richtung und ohne Entschlüsselung sind es 7,3 % bzw. 6,3 %, was
dem Zufallswert (1/16) entspricht. Bei den Trainern ist es in der richtigen Richtung 86,2 % (Platin, vorwärts) bzw.
85,7 % (HeartGold, rückwärts). Es ist kein Code aus DSPRE oder pret übernommen.
