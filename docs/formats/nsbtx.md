# NSBTX (BTX0 / TEX0)

Quellen: apicula `src/nitro/{container,tex,info_block,name}.rs` und
`src/nds/decode_texture.rs` (0BSD) für Layout und Texturformate; GBATEK „DS 3D Texture
Formats“ und „DS 3D Texture Attributes“; Gutachten A
(`docs/research/2026-11-05-phase-6/gutachten-a-writer.md` §2.4, §2.6, §2.8, Prototypen
`gutachten-a-nitro.py` `write_nsbtx_canonical`, `gutachten-a-minimal.py` `merge_btx`,
`gutachten-a-proto` `write_btx0`) und eigene Messung. Implementierung:
`crates/g4-formats/src/nsbtx.rs` (Leser, Dekodierer) und `crates/g4-formats/src/nsbtx/write.rs`
(`Btx0`, `write`, `append`, `build`, `dict_spans`), Wörterbuch
`crates/g4-formats/src/nitro_dict.rs` (Layout in `nsbmd.md`, Abschnitt „Wörterbuch“).
Alle Werte Little Endian.

## Container (BTX0)

| Offset | Inhalt |
|--------|--------|
| 0x00 | `b"BTX0"` |
| 0x04 | `u16 0xFEFF` (Byte Order Mark, Bytes `FF FE`) |
| 0x06 | `u16` Version (wird nicht geprüft) |
| 0x08 | `u32 file_size` (kleiner als die Eingabe ist ein Fehler `Truncated`) |
| 0x0C | `u16 0x0010` (Header-Größe) |
| 0x0E | `u16 n` Anzahl Sektionen |
| 0x10 | n × `u32` Offset der Sektion (ab Dateianfang) |

Der Leser nimmt die erste Sektion mit `b"TEX0"`; fehlt sie, ist das `BadMagic { "TEX0" }`.

## TEX0-Sektion

Alle Offsets in der Sektion sind relativ zum Beginn der Sektion (`b"TEX0"`).

| Offset | Größe | Inhalt |
|--------|-------|--------|
| 0x00 | 4 | `b"TEX0"` |
| 0x04 | 4 | `u32 section_size` (nicht genutzt) |
| 0x0C | 2 | `u16` Länge des Texturblocks / 8 (nicht genutzt) |
| 0x0E | 2 | `u16 texture_off` Offset des Texturen-Info-Blocks |
| 0x14 | 4 | `u32 tex_block_off` Offset der Texeldaten |
| 0x1C | 2 | `u16` Länge des komprimierten Blocks / 8 (Format 5, nicht genutzt) |
| 0x1E | 2 | `u16` Offset des komprimierten Info-Blocks (nicht genutzt) |
| 0x24, 0x28 | 4 + 4 | Offsets der Texeldaten für Format 5 (nicht genutzt) |
| 0x30 | 2 | `u16 pal_block_len / 8` |
| 0x34 | 4 | `u32 palette_off` Offset des Paletten-Info-Blocks |
| 0x38 | 4 | `u32 pal_block_off` Offset der Palettendaten |

Der feste Kopf ist 0x3C Bytes lang. Nicht aufgeführte Felder sind unbekannt und werden
übersprungen.

## Info-Block

Gemeinsame Struktur für Texturen und Paletten (Offset `o` ab Sektionsbeginn, n = Anzahl):

| Offset | Größe | Inhalt |
|--------|-------|--------|
| o + 0 | 1 | `u8 0` (Dummy, sonst Fehler `BadSignature`) |
| o + 1 | 1 | `u8 n` |
| o + 2 | 2 | `u16` Größe des Kopfs (wird nicht genutzt) |
| o + 4 | 8 | Unterkopf (`u16 8`, `u16` Größe, `u32 0x017F`), nicht genutzt |
| o + 12 | 4n | n × `u32` unbekannt (nicht genutzt) |
| o + 12 + 4n | 2 | `u16` Größe eines Datums (Textur 8, Palette 4, sonst `BadSize`) |
| o + 14 + 4n | 2 | `u16` Größe des Datenabschnitts (nicht genutzt) |
| o + 16 + 4n | n × Datum | Daten |
| danach | 16n | n Namen, je 16 Bytes, mit NUL aufgefüllt |

Namen werden Byte für Byte als Zeichen gelesen; Bytes unter 0x20 werden zu `.`.

### Texturdatum (8 Bytes)

`u32 params`, `u32` unbekannt. `params`:

| Bits | Bedeutung |
|------|-----------|
| 0 bis 15 | Offset der Texeldaten / 8, relativ zu `tex_block_off` |
| 16 bis 19 | Wiederholen und Spiegeln S/T (nicht genutzt) |
| 20 bis 22 | Breite = `8 << v` |
| 23 bis 25 | Höhe = `8 << v` |
| 26 bis 28 | Format |
| 29 | `color0`: Palettenindex 0 ist transparent |
| 30 bis 31 | Texturkoordinaten-Transformation (nicht genutzt) |

Die Spec (§4.1) nennt dieselben Felder in der oberen Hälfte des Worts (`p = params >> 16`):
Format Bits 10 bis 12, Breite `8 << ((p >> 4) & 7)`, Höhe `8 << ((p >> 7) & 7)`, `color0` Bit 13.

### Paletten-Datum (4 Bytes)

`u16 offset / 8` (relativ zu `pal_block_off`), `u16` unbekannt. Die Länge einer Palette
steht nicht in der Datei: sie reicht bis zum nächsthöheren Palettenoffset, die letzte bis
zum Ende des Palettenblocks. Farben sind BGR555 (`u16`: Bits 0 bis 4 Rot, 5 bis 9 Grün,
10 bis 14 Blau).

## Texturformate

| Format | Name | Bits je Texel | Bytes (B × H) | Dekodierung |
|--------|------|---------------|---------------|-------------|
| 1 | A3I5 | 8 | B·H | Index = Bits 0 bis 4, Alpha = `(v >> 5) * 255 / 7` |
| 2 | 4 Farben | 2 | B·H / 4 | niedrigste Bits zuerst, Alpha 255 |
| 3 | 16 Farben | 4 | B·H / 2 | niedriges Nibble zuerst, Alpha 255 |
| 4 | 256 Farben | 8 | B·H | Alpha 255 |
| 5 | 4×4-Texel | 2 + 1 | | nicht unterstützt (`FormatError::Unsupported`), Texeldaten werden nicht gelesen |
| 6 | A5I3 | 8 | B·H | Index = Bits 0 bis 2, Alpha = `(v >> 3) * 255 / 31` |
| 7 | direkt | 16 | 2·B·H | BGR555 mit Bit 15 = Alpha (0 oder 255), keine Palette |

Zeilen laufen von oben nach unten, Pixel von links nach rechts (Zeile für Zeile).
5-Bit-Komponenten werden mit `v << 3 | v >> 2` auf 8 Bit erweitert.

Paletten-Index 0 ist transparent (Alpha 0), wenn das `color0`-Flag der Textur gesetzt ist
(Formate 2, 3, 4; die Formate 1 und 6 ignorieren das Flag wie die Hardware) oder wenn
`decode_with(.., force_index0_transparent = true)` das erzwingt (dann für jedes
Palettenformat; die Sprite-Schicht nutzt das für Overworld-Sprites). Ein Farbindex hinter
dem Palettenende liest als Farbe 0 (schwarz).

## Schreiben

`Btx0` hält ein ganzes Paket: beide Wörterbücher mit ihren Rohwörtern (`TextureEntry
{ name, params, extra }`, `PaletteEntry { name, offset8, extra }`), Texel-, 4×4-Texel-,
4×4-Index- und Palettenblock als Ganzes, die Version und die Kopffelder, die der Writer
nicht berechnet. `write(&Btx0::read(b)?)? == b` gilt für jedes Vanilla-Karten- und
Gebäudetexturpaket. Byte-gleich aus dekodierten Bildern ginge nicht (geteilte Texel,
Texelblock nicht in Wörterbuchreihenfolge, A §2.4); deshalb bleiben die Blöcke Bytes.

### Layout des Writers

`Btx0::read` nimmt nur dieses Layout an; jede andere Lage ist `Unsupported` (kaputte Dateien
`BadMagic`, `BadSignature`, `BadSize`, `Truncated`):

| Lage | Inhalt |
|------|--------|
| 0x00 | BTX0-Kopf: `b"BTX0"`, `0xFEFF`, Version (gehalten), Dateigröße (gleich der Länge), `0x0010`, ein Block |
| 0x10 | `u32 0x14`, Offset des einzigen Blocks |
| 0x14 | TEX0-Kopf, 0x3C Byte (Tabelle unten) |
| 0x14 + 0x3C | Texturen-Wörterbuch (`unit` 8: `u32 params`, `u32 extra`) |
| danach | Paletten-Wörterbuch (`unit` 4: `u16 offset8`, `u16 extra`) |
| danach | Texelblock (`0x0C` · 8 Byte) |
| danach | 4×4-Texelblock (`0x1C` · 8 Byte), dann 4×4-Index (`0x1C` · 4 Byte) |
| danach | Palettenblock (`0x30` · 8 Byte) bis zum Dateiende |

Felder des TEX0-Kopfs (relativ zu `b"TEX0"`):

| Offset | Feld | Writer | Vanilla (A §2.4, eigene Messung) |
|--------|------|--------|----------------------------------|
| 0x04 | Blockgröße | berechnet | Dateigröße − 0x14 |
| 0x08, 0x18, 0x2C | VRAM-Schlüssel (`vram_keys`) | roh | 0 |
| 0x0C | Texelblock / 8 | berechnet | |
| 0x0E | Texturen-Info-Offset | 0x3C | 0x3C |
| 0x10 | `tex_unknown` | roh | |
| 0x14 | Texelblock-Offset | berechnet | |
| 0x1C | 4×4-Block / 8 | berechnet | 0 (kein Format 5 in Karten- und Gebäudepaketen) |
| 0x1E | 4×4-Info-Offset | 0x3C, wenn `compressed_info_at_textures`, sonst 0; anderes ist `Unsupported` | gleich dem Texturen-Info-Offset |
| 0x20 | `compressed_unknown` | roh | |
| 0x24, 0x28 | 4×4-Texel- und 4×4-Index-Offset | berechnet | beide gleich dem Paletten-Offset (Blöcke leer) |
| 0x30 | Palettenblock / 8 | berechnet | |
| 0x32 | Palettenflag (`palette_flag`) | roh, `push` setzt Bit 15 | `0x8000` genau dann, wenn eine 4-Farben-Textur im Paket ist (Platin 66 von 74, HeartGold 99 von 106 Kartenpaketen) |
| 0x34 | Paletten-Info-Offset | berechnet | |
| 0x38 | Palettenblock-Offset | berechnet | |

Ein Block muss durch 8 teilbar sein und seine Länge / 8 in ein `u16` passen (bis
`0xFFFF · 8` = 524 280 Byte), der 4×4-Index ist halb so lang wie der 4×4-Block, sonst ist
`write` ein Fehler (`BadSize`, `OutOfRange`). Die Wörterbücher schreibt `nitro_dict` mit dem
Vanilla-Baum (höchstens 255 Einträge, Namen je Wörterbuch eindeutig).

### Neue Texturen

`NewTexture { name, palette, width, height, format, color0, texels, colors }`; `Btx0::push`
hängt Textur und Palette hinten an, `append(pack, new)` liest, hängt an und schreibt (die
Version bleibt), `build(new)` macht ein neues Paket (Version 1, Kopffelder wie Vanilla:
VRAM-Schlüssel 0, 0x1E = 0x3C, 0x10 und 0x20 = 0).

| Feld | Wert |
|------|------|
| Format | 3 (16 Farben, 4 Bit, niedriges Nibble zuerst) oder 2 (4 Farben, 2 Bit, niedrigste Bits zuerst) |
| Texel | hinter dem alten Texelblock, auf 8 Byte ausgerichtet |
| `params` | `off8 \| log2(B/8) << 20 \| log2(H/8) << 23 \| format << 26 \| color0 << 29` (Wiederholen und texgen 0, sie stehen im Material) |
| `extra` der Textur | `B \| H << 11 \| 1 << 31` (alle Vanilla-Texturen) |
| Palette | hinter dem alten Palettenblock, genau 4 bzw. 16 Farben BGR555; eine 16-Farben-Palette beginnt auf einer 16-Byte-Grenze (`offset8` gerade), eine 4-Farben-Palette auf einer 8-Byte-Grenze; die Lücke ist 0 |
| `extra` der Palette | 1 genau für die Palette einer 4-Farben-Textur, sonst 0 (Vanilla: Platin 598, HeartGold 871 solche Paletten) |
| Palettenflag | alter Wert ODER `0x8000` bei einer 4-Farben-Textur |

`push` lehnt ab und lässt das Paket unverändert:

| Fall | Fehler |
|------|--------|
| Seite keine Zweierpotenz 8 bis 1024 | `OutOfRange { what: "texture side" }` |
| Format außer 2 und 3 | `Unsupported { what: "new texture format other than 2 and 3" }` |
| Texelzahl ≠ `B·H/4` (Format 2) bzw. `B·H/2` (Format 3) | `BadSize` |
| Farbzahl ≠ 4 (Format 2) bzw. ≠ 16 (Format 3) | `BadSize { expected: 4 \| 16 }` |
| Name leer, über 16 Byte, Zeichen außerhalb `0x21..=0x7E` | `OutOfRange { what: "Nitro name" }` (`nitro_dict::name`) |
| Texturname schon im Paket | `Unsupported { what: "texture name already in the pack" }` |
| Palettenname schon im Paket | `Unsupported { what: "palette name already in the pack" }` |
| 256. Eintrag | `OutOfRange { what: "Nitro dictionary entries" }` |
| Texel- oder Palettenblock danach über `0xFFFF · 8` Byte | `OutOfRange { what: "texel offset" }` bzw. `"palette offset"` |

Warum 16 Byte: die Palettenbasis des Polygons (`PLTT_BASE`, GBATEK „DS 3D Texture
Attributes“) zählt in 16-Byte-Schritten, nur für 4-Farben-Texturen in 8-Byte-Schritten. Eine
16-Farben-Palette auf einer ungeraden 8-Byte-Adresse wäre von der Hardware nicht erreichbar.
In Vanilla liegt keine Palette ohne 4-Farben-Markierung auf einer ungeraden `offset8`
(gated gezählt, `odd16` unten). Der Leser (`Nsbtx`) bemisst eine Palette bis zur nächsten:
die Lücke hinter einer 4-Farben-Palette liest er als Farben 0 mit.

### Anhängen

Die alten Einträge und Blöcke bleiben byte-gleich: die neuen Einträge kommen ans Ende beider
Wörterbücher, ihre Texel und Paletten hinter die alten Blöcke, die alten `offset8` ändern
sich nicht. Nur die Wörterbücher (Baum, Größe) und die Offsets des TEX0-Kopfs werden neu
berechnet. Die letzte alte Palette reichte bis zum Blockende und reicht danach bis zur
ersten neuen; ihre Farben bleiben dieselben (dahinter höchstens die Ausrichtungslücke aus
Nullen).

### Palettenname

Jede neue Textur bringt eine eigene Palette. In einem Paket müssen Texturnamen und
Palettennamen je eindeutig sein (die Wörterbücher sind getrennt: eine Palette darf wie eine
Textur heißen, wie in Vanilla bei 1 837 von 3 000 bzw. 546 von 3 628 Paletten). Den Namen
rät `texconv::palette_name` wie apicula (`image_namer.rs` `append_pl`): der Texturname plus
so viel von `_pl`, wie in 16 Byte passt.

### Bänke

`TEXEL_BANK` = 262 144 Byte (256 KiB, `GX_VRAM_TEX_01_AB`) und `PALETTE_BANK` = 65 536 Byte
(64 KiB, `GX_VRAM_TEXPLTT_0123_E`) teilen sich Karten- und Gebäudepaket eines Gebiets (A
§2.4: Vanilla höchstens 110 640 Byte für beide). `Btx0::vram_bytes` = Texel + 4×4-Texel +
4×4-Index + Paletten. `push` prüft nur die Grenzen des Formats; die Bänke prüft der Aufrufer
über alle Pakete eines Gebiets.

## Import aus PNG

Phase 6.5a (`g4_kernel::model_import::textures`): die Bilder eines Modells (OBJ `map_Kd`,
glTF `baseColorTexture`) und PNG-Dateien als Teil „Texturen“ werden neue Texturen des
Kartenpakets ihres Gebiets.

- **Weg:** PNG/JPEG → `load_image` (RGBA8) → `texconv::convert` (Median-Cut in BGR555, 16
  oder 4 Farben nach `options.colors`, `color0` bei Alpha < 128, ohne Dithering, eine Palette
  je Textur, Palettenname `palette_name`) → `NewTexture` → Anhängen wie oben
  (`appended_pack_or_refusal` im Speicher für die Vorschau, `append_textures` als
  Dateischritt beim Import).
- **Keine Skalierung:** jede Seite muss eine Zweierpotenz von 8 bis 1 024 sein, sonst
  `texture.side` `[name, breite, höhe]`. Eine Seite über 1 024 wird schon am Bildkopf
  abgelehnt, bevor dekodiert wird (ein Kopf kann jede Größe behaupten); der Dekoder läuft
  zusätzlich mit 1 024 × 1 024 und 64 MiB als Grenze.
- **Name:** `texture_name(stamm)`: druckbares ASCII `0x21..=0x7E` bleibt, jedes andere
  Zeichen wird `_` (eins je Zeichen, auch für ein Mehrbyte-UTF-8-Zeichen: `größe` →
  `gr__e`), auf 16 Byte gekürzt, leer → `g4_tex`. Zwei verschiedene Bilder mit gleichem
  Namen in einem Aufruf: `_2`, `_3` … (der Stamm gekürzt, damit 16 Byte reichen; ein Name,
  den das Paket hat, wird übersprungen); dasselbe Bild zweimal ist eine Textur.
- **Name schon im Paket:** beim Modell wird die Paket-Textur benutzt (Hinweis
  `texture.reused`, nichts angehängt, Größe aus dem Paket); bei einer PNG-Datei
  `texture.taken` `["texture", name]` (Ersetzen gibt es nicht). Ein Material ohne Bild
  bindet die Paket-Textur seines Namens, sonst bleibt es ohne Textur
  (`model.material-untextured`, sobald eine Fläche es nutzt).
- **Fehlercodes:** `texture.side`, `texture.name`, `texture.taken` (`texture` oder
  `palette`), `texture.pack-full` (`[was, wert, höchstens]`), `texture.unreadable`
  (`[datei, grund]`); Hinweis `texture.new` `[name, breite, höhe, farben, mittlerer fehler
  mit einer Nachkommastelle]`; Warnung `texture.vram-high` `[gebiet, vram, höchstens]`. Die
  Ablehnungen des Anhängens bildet `model_import::from_bake` mit denselben Argumenten ab.
- **Messung „40 Farben“** (`a_64x64_png_with_40_colours_becomes_a_16_colour_texture_within_one_step`,
  `g4_imp_40.png` aus `samples::write_samples`, 64 × 64, 40 Farben in einem Raster aus 8 × 5
  Feldern): eine Textur Format 3 mit 16 Farben, gebaut, gelesen und dekodiert. Keine der 40
  Farben steht genau in der Palette (0/40: der Median-Cut nimmt Mittelwerte der Boxen); 8 der
  40 Farben haben eine Palettenfarbe innerhalb einer 5-Bit-Stufe je Kanal („Treffer“, 816 von
  4 096 Pixeln), und jedes dieser Pixel kommt innerhalb einer Stufe zurück. Über alle Pixel
  höchstens 3 Stufen je Kanal, mittlerer Farbfehler 12,8 (RGB8 je Kanal). Log:
  `png40 hits=8/40 pixels=816/4096 max_step=3 mean=12.8203125 exact=0/40`.

## Fehlerverhalten

Jeder Zugriff ist längengeprüft (`Truncated`, `BadMagic`, `BadSignature`, `BadSize`,
`OutOfRange` für Textur- und Palettenindex bei `decode`), ohne Panic. Die Texeldaten und
Paletten werden schon in `read` bereichsgeprüft und kopiert.

## Verifikation

- Unit-Tests in `nsbtx.rs` bauen ein minimales BTX0 nach diesem Layout und prüfen die
  Listen, die Formate 1, 2, 3, 4, 6, 7, `color0`, das Erzwingen von Index 0, Format 5,
  fehlerhafte Magic, ungültige Indizes, jede Kürzung der Eingabe und Bytefehler an jeder
  Position (kein Panic).
- Gated (`tests/vanilla.rs`, `G4_TEST_PROJECT`): Member 0 des Sprite-NARC
  (`files/data/mmodel/mmodel.narc` bzw. `files/a/0/8/1`) ist ein BTX0 mit 16 Texturen
  (32×32, Format 3, `color0` gesetzt) und einer Palette; Pixel mit Alpha 255 und 0 sind
  vorhanden. Alle Members, die mit `BTX0` beginnen (Platin DE 421 von 470, HeartGold DE und
  US je 832 von 863; der Rest sind BMD0 und andere Daten), lassen sich lesen und mit jeder
  Palette dekodieren (kein Format 5 darunter).
- Unit-Tests in `nsbtx/write.rs` (synthetische Texturen 8 × 8 Format 3, 16 × 8 Format 2,
  32 × 32 Format 3 mit `color0`): ein gebautes Paket liest der Leser mit Namen, Größen,
  Formaten und Pixeln zurück; die festen Felder sind die aus der Tabelle; `write` von
  `read` ist byte-gleich; Anhängen lässt alte Einträge und Blöcke byte-gleich; jede Ablehnung
  von `push` mit ihrer Fehlervariante und unverändertem Paket; 16-Farben-Paletten auf
  gerader `offset8`, auch beim Anhängen hinter einen Block mit Länge 8 mod 16; jede andere
  Lage mit ihrer Variante; jede Kürzung ist ein Fehler, Bytefehler (proptest) ohne Panic.
- Gated (`tests/vanilla_nitro.rs`): `vanilla_texture_packs_round_trip_byte_identical` schreibt
  jedes Member von `unpacked/mapTextures` und `unpacked/buildingTextures`, das mit `BTX0`
  beginnt, byte-gleich zurück und beide Wörterbücher aus `dict_spans` byte-gleich neu:
  Platin 74 + 71 Pakete, HeartGold DE und US je 106 + 94 (die 10 übrigen HeartGold-Gebäude-
  Member sind keine BTX0); mit den Wörterbüchern der Kartenmodelle (`nsbmd.md`) zusammen
  Platin 4 286, HeartGold DE und US je 4 456 Wörterbücher (13 198 von 13 198). Paletten
  ohne 4-Farben-Markierung auf ungerader `offset8` (`odd16`): 0 in allen drei Spielen.
  `vanilla_append_keeps_every_old_entry` hängt an jedes Kartenpaket zwei synthetische
  Texturen an (`g4_t_a` 8 × 8 Format 3, `g4_t_b` 8 × 8 Format 2): alte Einträge, Texel- und
  Palettenblöcke byte-gleich, die erste alte Textur dekodiert vorher wie nachher (ein Fehler
  beim Dekodieren zählt als Abweichung), die neuen
  wie gebaut (Platin 74 von 74, HeartGold DE und US je 106 von 106).
