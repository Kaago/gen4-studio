# NSBMD (BMD0 / MDL0): Modell-Codec

Quellen: apicula `src/nitro/{model,render_cmds,info_block}.rs` und `src/nds/gpu_cmds.rs`
(0BSD) als Layout-Referenz; GBATEK „DS 3D Geometry Commands“, „DS 3D Polygon Attributes“,
„DS 3D Texture Attributes“; pret/pokeplatinum `src/overlay005/land_data.c:1935`
(`numEntry == 1`); Gutachten A (`docs/research/2026-11-05-phase-6/gutachten-a-writer.md`
§2.2 bis §2.6, Prototyp `gutachten-a-nitro.py`, `write_model`/`write_nsbmd`) und eigene
Messung. Implementierung: `crates/g4-formats/src/nsbmd/{mod,model,gpu,sbc}.rs`,
Wörterbuch `crates/g4-formats/src/nitro_dict.rs`. Alle Werte Little Endian.

Der Codec liest ein Kartenmodell in eine Struktur (`Nsbmd`, `Model`) und schreibt es im
Vanilla-Layout zurück. Der Round-Trip ist eine Prüfung, kein Produktpfad: unveränderte
Modelle bleiben beim Import Bytes (`nsbmd_header.md`). Der Bäcker (Phase 6.1, Task 6) baut
neue Modelle mit diesem Codec.

## Container (BMD0)

| Offset | Inhalt | Leser |
|--------|--------|-------|
| 0x00 | `b"BMD0"` | sonst `BadMagic` |
| 0x04 | `u16 0xFEFF` (Bytes `FF FE`) | sonst `BadSignature` |
| 0x06 | `u16` Version (Vanilla 2) | als `Nsbmd::version` gehalten |
| 0x08 | `u32` Dateigröße | gleich der Länge, sonst `BadSize` |
| 0x0C | `u16 0x0010` Kopfgröße | sonst `BadSize` |
| 0x0E | `u16` Anzahl Blöcke | genau 1; 0 → `BadMagic { "MDL0" }`, mehr → `Unsupported` |
| 0x10 | `u32` Blockoffset | Writer: 0x14 |

Genau ein Block `MDL0`. Ein Block `TEX0` (allein oder neben `MDL0`) oder ein zweiter Block
ist `Unsupported { what: "NSBMD block other than MDL0" }`; der Import entfernt TEX0 vorher
mit `nsbmd_header::strip_tex0`. Alle Vanilla-Kartenmodelle haben nur `MDL0` (A §2.2).

**MDL0-Block** (Offsets ab Blockanfang): `b"MDL0"`, `u32` Blocklänge, an +8 das
Modell-Wörterbuch (Datum `u32` = Offset des Modells ab Blockanfang), danach die Modelle
lückenlos hintereinander. Das Spiel verlangt genau ein Modell und nimmt Modell 0
(`land_data.c:1935`); der Codec liest und schreibt beliebig viele.

## Wörterbuch (Nitro-Dictionary)

Jede Namensliste des Modells (Modelle, Knochen, Materialien, Textur- und
Paletten-Paarungen, Shapes) ist ein Wörterbuch. Layout ab `o`:

| Offset | Inhalt |
|--------|--------|
| o + 0 | `u8 0` Revision |
| o + 1 | `u8 n` Einträge (0 bis 255) |
| o + 2 | `u16` Größe des Wörterbuchs = `8 + 4(n + 1) + 4 + n · unit + 16n` |
| o + 4 | `u16 8` |
| o + 6 | `u16 12 + 4n` Offset des Eintragsblocks `e` |
| o + 8 | `n + 1` Knoten zu 4 Byte `[ref_bit, left, right, entry]`, Knoten 0 ist die Wurzel |
| e + 0 | `u16 unit` Größe eines Datums (im Modell immer 4) |
| e + 2 | `u16 4 + unit · n` Offset der Namen ab `e` |
| e + 4 | `n` Daten zu `unit` Byte |
| danach | `n` Namen zu 16 Byte, mit NUL aufgefüllt |

Der Leser (`nitro_dict::read`) nimmt nur genau dieses Layout an (jede Abweichung `BadSignature`
oder `BadSize`) und überspringt den Baum; der Writer rechnet ihn neu, die Byte-Gleichheit
prüft ihn. Neue Namen (`nitro_dict::name`): 1 bis 16 Byte, jedes `0x21..=0x7E`. Doppelte
Namen sind `Unsupported { "duplicate name in a Nitro dictionary" }`, über 255 Einträge
`OutOfRange`.

**Patricia-Baum** (`nitro_dict::tree`, gemessen gegen alle 13 198 Vanilla-Wörterbücher, A §2.6):
Bit `b` eines Namens ist Bit `b & 31` des `u32` LE an Byte `4 · (b >> 5)`. Start mit der
Wurzel `[127, 0, 0, 0]`. Für jeden Eintrag `i` mit Schlüssel `k` in Eintragsreihenfolge
(Sedgewick):

1. Suche: `p = 0`, `x = nodes[0].left`; solange `nodes[p].ref > nodes[x].ref`: `p = x`,
   `x = bit(k, nodes[x].ref) ? right : left`.
2. `other` = Schlüssel von `x` (Knoten 0: 16 Nullbytes, sonst `names[nodes[x].entry]`);
   `diff` = höchstes Bit, in dem `k` und `other` verschieden sind (keins: doppelter Name).
3. Zweiter Abstieg wie die Suche, aber nur solange zusätzlich `nodes[x].ref > diff`.
4. Neuer Knoten `t`: `bit(k, diff) ? [diff, x, t, i] : [diff, t, x, i]`; Verknüpfen:
   `p == 0` → `nodes[0].left = t`, sonst je nach `bit(k, nodes[p].ref)` rechts oder links.

Danach werden die Knoten in Vorordnung über die Abwärtskanten nummeriert (links vor
rechts; Abwärtskante: Kind `c != 0` mit `nodes[c].ref < nodes[i].ref`). Ohne diese
Umnummerierung stimmte nur ein Wörterbuch mit einem Eintrag. Beispiele (Tests in
`nitro_dict.rs`):

| Namen | Knoten |
|-------|--------|
| keine | `[127, 0, 0, 0]` (16 Byte Wörterbuch) |
| `a` | `[127, 1, 0, 0]`, `[6, 0, 1, 0]` |
| `a`, `b` | `[127, 1, 0, 0]`, `[6, 0, 2, 0]`, `[1, 1, 2, 1]` |
| `a`, `aa` | `[127, 1, 0, 0]`, `[14, 2, 1, 1]`, `[6, 0, 2, 0]` (ohne Umnummerierung stünde der Knoten mit Bit 14 an Index 2) |

Die Suche des Spiels geht von der Wurzel abwärts, bis eine Kante nach oben (oder zur
Wurzel) zeigt, und nimmt den Eintrag dieses Knotens.

## Modell

Alle Offsets ab Modellanfang.

### Modellkopf (0x00 bis 0x3F)

| Offset | Typ | Feld | Writer |
|--------|-----|------|--------|
| 0x00 | `u32` | Größe des Modells | berechnet |
| 0x04 | `u32` | `ofsSbc` | berechnet |
| 0x08 | `u32` | `ofsMat` | berechnet |
| 0x0C | `u32` | `ofsShp` | berechnet |
| 0x10 | `u32` | `ofsEvp` | berechnet |
| 0x14 | `u8` | `sbc_type` | wie gehalten |
| 0x15 | `u8` | `scaling_rule` | wie gehalten |
| 0x16 | `u8` | `texmtx_mode` | wie gehalten |
| 0x17 | `u8` | `num_objects` (Knoten) | wie gehalten |
| 0x18 | `u8` | `num_materials` | wie gehalten |
| 0x19 | `u8` | `num_shapes` | wie gehalten |
| 0x1A | `u8` | `first_unused_mtx` | wie gehalten |
| 0x1B | `u8` | `pad` | wie gehalten |
| 0x1C | `i32` | `up_scale` (`posScale`, fx32) | wie gehalten |
| 0x20 | `i32` | `down_scale` (`1 / posScale`, fx32) | wie gehalten |
| 0x24 | `u16` | `num_verts` | wie gehalten |
| 0x26 | `u16` | `num_polys` | wie gehalten |
| 0x28 | `u16` | `num_tris` | wie gehalten |
| 0x2A | `u16` | `num_quads` | wie gehalten |
| 0x2C | 3 × `i16` | `box_pos` (Rahmen, Minimum) | wie gehalten |
| 0x32 | 3 × `i16` | `box_size` (Rahmen, Ausdehnung) | wie gehalten |
| 0x38 | `i32` | `box_scale` | wie gehalten |
| 0x3C | `i32` | `box_scale_inv` | wie gehalten |

Die Felder 0x14 bis 0x3F stehen einzeln in `ModelHead`; `Nsbmd::write` setzt Zähler und
Rahmen nie selbst. `Model::geometry()` zählt Ecken und Polygone über alle Shapes
(`gpu::count`), `Model::counters_match()` vergleicht sie mit dem Kopf. Das Spiel liest die
Zähler nicht: HeartGold DE hat in den Karten 0468, 0475, 0476 veraltete Zähler, HeartGold US
zusätzlich 0473 (A §2.2); der Round-Trip erhält sie.

### Knochen (Objekte)

Wörterbuch an 0x40, Datum `u32` = Offset des Knochens ab Wörterbuchanfang; die Knochen
folgen lückenlos. Ein Knochen beginnt mit dem `u16`-Flagwort; seine Länge folgt aus den
Flags (`model::bone_len`):

| Bit | gelöscht | gesetzt |
|-----|----------|---------|
| 0 (Verschiebung null) | + 12 Byte Verschiebung (3 × fx32) | – |
| 3 (Pivot-Rotation) | wenn Bit 1 gelöscht: + 16 Byte Rotation (3 × 3, gepackt) | + 4 Byte Pivot |
| 2 (Skalierung eins) | + 24 Byte Skalierung und Kehrwert (6 × fx32) | – |

Grundlänge 4 (Flagwort und ein `u16`). Beispiele: `0xF807` (Identität) 4, `0x0000` 56,
`0x0008` 44, `0x0006` 16, `0x0002` 40. apicula liest die Skalierung mit 12 statt 24 Byte;
damit wären 5 Platin-Modelle 12 bis 120 Byte zu kurz (A §2.6). Der Codec hält das Flagwort
und die Bytes danach (`Bone::rest`, `bone_len(flags) − 2` Byte) roh.

### SBC (Render-Befehle)

An `ofsSbc`, jeder Befehl ein Opcode und feste Parameterbytes, bis einschließlich `0x01`;
danach mit Nullbytes auf 4 aufgefüllt. Unbekannter Opcode → `OutOfRange { "SBC command" }`.

| Opcode | Parameterbytes | Bedeutung (apicula `render_cmds.rs`) |
|--------|----------------|--------------------------------------|
| `0x00` | 0 | nichts |
| `0x01` | 0 | Ende |
| `0x02` | 2 | Sichtbarkeit eines Knotens (Knoten, sichtbar) |
| `0x03` | 1 | Matrix vom Stapel laden |
| `0x04`, `0x24`, `0x44` | 1 | Material binden |
| `0x05` | 1 | Shape zeichnen |
| `0x06` | 3 | Knotenmatrix anwenden (Knoten, Eltern, Wert) |
| `0x26`, `0x46` | 4 | ebenso, mit Ablage auf bzw. Laden vom Stapel |
| `0x66` | 5 | ebenso, mit Laden und Ablage |
| `0x07`, `0x08` | 1 | Billboard, Y-Billboard |
| `0x09` | 2 + 3 · (zweites Byte) | Skinning (Hüllmatrix) |
| `0x0B`, `0x2B` | 0 | `posScale` hoch- bzw. herunterskalieren |
| `0x0C`, `0x0D` | 2 | Umgebungs- bzw. Projektions-Mapping |
| `0x40`, `0x80` | 0 | nichts |
| `0x47` | 2 | Billboard, Variante mit Stapelindex |

Grundmuster der Kartenmodelle: `26 02 0B (04 05)* 2B 01` (628 von 666 Platin-, 667 von 676
HeartGold-Modellen, A §2.2). Der Bäcker schreibt
`26 00 00 00 00 02 00 01 0B (04 ii 05 ii)* 2B 01`.

### Materialabschnitt

An `ofsMat`; alle Offsets ab Abschnittsbeginn:

| Teil | Inhalt |
|------|--------|
| +0 | `u16` Offset des Textur-Paarungs-Wörterbuchs |
| +2 | `u16` Offset des Paletten-Paarungs-Wörterbuchs |
| +4 | Material-Wörterbuch, Datum `u32` = Offset des Materials |
| danach | Textur-Paarungs-Wörterbuch, Paletten-Paarungs-Wörterbuch |
| danach | Material-Indizes (`u8`) erst aller Textur-, dann aller Paletten-Paarungen, auf 4 aufgefüllt |
| danach | die Materialien lückenlos |

**Paarung** (Datum `u16 off, u8 n, u8 bound`): `n` Material-Indizes an `off`. Der Name ist
ein Textur- bzw. Palettenname, den das Spiel beim Binden (`NNS_G3dBindMdlSet`) im TEX0 des
Gebiets sucht; ein Material nennt seine Textur nicht selbst. `Pairing::bound` wird
gehalten wie gelesen.

**Material** (`Material`, 44 Byte, dazu `extra`):

| Offset | Typ | Feld |
|--------|-----|------|
| 0x00 | `u16` | `tag` |
| 0x02 | `u16` | Größe: 44, 48 oder 52 (= 44 + `extra.len()`), sonst `Unsupported { "material size" }` |
| 0x04 | `u32` | `dif_amb` (Diffus und Ambient, BGR555) |
| 0x08 | `u32` | `spe_emi` (Spekular und Emission) |
| 0x0C | `u32` | `poly_attr` (`POLYGON_ATTR`) |
| 0x10 | `u32` | `poly_attr_mask` |
| 0x14 | `u32` | `teximage` (`TEXIMAGE_PARAM`) |
| 0x18 | `u32` | `teximage_mask` |
| 0x1C | `u16` | `pltt_base` |
| 0x1E | `u16` | `flag` |
| 0x20 | `u16` | `width` (Breite der Textur) |
| 0x22 | `u16` | `height` |
| 0x24 | 2 × `i32` | `mag` (fx32) |
| 0x2C | 0, 4 oder 8 Byte | `extra`: Texturmatrix, roh |

`POLYGON_ATTR` nach GBATEK: Bits 0 bis 3 Licht 0 bis 3, 4 bis 5 Polygonmodus (0 Modulation,
1 Decal, 2 Toon/Highlight, 3 Schatten), 6 Rückseite zeichnen, 7 Vorderseite zeichnen,
11 Tiefe für durchscheinende Pixel, 12 Polygone an der fernen Ebene, 13 1-Punkt-Polygone,
14 Tiefentest gleich, 15 Nebel, 16 bis 20 Alpha (31 deckend, 0 Drahtgitter), 24 bis 29
Polygon-ID. `TEXIMAGE_PARAM`: Bits 0 bis 15 VRAM-Offset / 8, 16 und 17 Wiederholen S/T,
18 und 19 Spiegeln S/T, 20 bis 22 und 23 bis 25 Breite/Höhe (`8 << n`), 26 bis 28 Format,
29 Farbe 0 durchsichtig, 30 bis 31 Texturkoordinaten-Transformation. Im Material stehen
nur Wiederholen, Spiegeln und Transformation; Offset, Größe und Format setzt das Binden.

Messwerte (A §2.3, Platin 7 346 / HeartGold 13 247 Materialien): Polygonmodus überall
Modulation; Vorderseite überall; `poly_attr_mask` immer `0x3F1FF8FF`, `teximage_mask` immer
`0xFFFFFFFF`; häufigstes `dif_amb` `0x7FFFE739`; `spe_emi` 0 bei 7 315 bzw. 13 208; Größe 44
bei 7 330 bzw. 13 055, mit Texturmatrix 48 oder 52 bei 16 bzw. 192; Wiederholen S und T bei
6 369 bzw. 12 283; Licht 0 an bei 4 300 bzw. 7 924; Nebel an bei 6 207 bzw. 11 211.

### Shape-Abschnitt

An `ofsShp`: Wörterbuch (Datum `u32` = Offset des Kopfs ab Abschnittsbeginn), dann `n`
Köpfe zu 16 Byte, dann die Display Lists in Reihenfolge.

| Offset | Typ | Feld |
|--------|-----|------|
| 0x00 | `u16` | `tag` |
| 0x02 | `u16` | `size` (Vanilla 16, gehalten wie gelesen) |
| 0x04 | `u32` | `flag` (Bit 0 Normale, 1 Farbe, 2 Texturkoordinate; Vanilla meist `0x5` außen, `0x6` innen) |
| 0x08 | `u32` | Offset der Display List ab Kopf |
| 0x0C | `u32` | Länge der Display List |

### GPU-Befehle (Display List)

Eine Display List ist eine Folge von Gruppen: vier Opcode-Bytes, danach die 32-Bit-Parameter
dieser vier Befehle in Reihenfolge. `NOP` (0x00) hat keine Parameter und füllt eine Gruppe
mit weniger als vier Befehlen. `gpu::decode` behält jedes `NOP`, `gpu::encode` füllt nur die
letzte Gruppe, so ist `encode(decode(b)) == b` für jede angenommene Liste. Unbekannter
Opcode → `OutOfRange { "GPU command" }`.

| Opcode | Name (GBATEK) | Parameter |
|--------|---------------|-----------|
| 0x00 | `NOP` | 0 |
| 0x10 | `MTX_MODE` | 1 |
| 0x11 | `MTX_PUSH` | 0 |
| 0x12 | `MTX_POP` | 1 |
| 0x13 | `MTX_STORE` | 1 |
| 0x14 | `MTX_RESTORE` | 1 |
| 0x15 | `MTX_IDENTITY` | 0 |
| 0x16 | `MTX_LOAD_4x4` | 16 |
| 0x17 | `MTX_LOAD_4x3` | 12 |
| 0x18 | `MTX_MULT_4x4` | 16 |
| 0x19 | `MTX_MULT_4x3` | 12 |
| 0x1A | `MTX_MULT_3x3` | 9 |
| 0x1B | `MTX_SCALE` | 3 |
| 0x1C | `MTX_TRANS` | 3 |
| 0x20 | `COLOR` (BGR555) | 1 |
| 0x21 | `NORMAL` (3 × 1.0.9 in 10 Bit) | 1 |
| 0x22 | `TEXCOORD` (2 × 1.11.4 in 16 Bit) | 1 |
| 0x23 | `VTX_16` (3 × 1.3.12) | 2 |
| 0x24 | `VTX_10` (3 × 1.3.6 in 10 Bit) | 1 |
| 0x25 bis 0x27 | `VTX_XY`, `VTX_XZ`, `VTX_YZ` | 1 |
| 0x28 | `VTX_DIFF` | 1 |
| 0x29 | `POLYGON_ATTR` | 1 |
| 0x2A | `TEXIMAGE_PARAM` | 1 |
| 0x2B | `PLTT_BASE` | 1 |
| 0x30 bis 0x33 | `DIF_AMB`, `SPE_EMI`, `LIGHT_VECTOR`, `LIGHT_COLOR` | 1 |
| 0x34 | `SHININESS` | 32 |
| 0x40 | `BEGIN_VTXS` (0 Dreiecke, 1 Quads, 2 Dreieck-Strip, 3 Quad-Strip) | 1 |
| 0x41 | `END_VTXS` | 0 |
| 0x50 | `SWAP_BUFFERS` | 1 |
| 0x60 | `VIEWPORT` | 1 |
| 0x70 | `BOX_TEST` | 3 |
| 0x71 | `POS_TEST` | 2 |
| 0x72 | `VEC_TEST` | 1 |

Zählung (`gpu::count`): jedes `VTX_*` ist eine Ecke; ein Lauf endet an `END_VTXS`, am
nächsten `BEGIN_VTXS` oder am Ende der Liste; Dreiecke `n / 3`, Quads `n / 4`,
Dreieck-Strip `n − 2`, Quad-Strip `(n − 2) / 2`.

### EVP

Von `ofsEvp` bis zum Modellende, roh (`Model::evp`). In allen Vanilla-Kartenmodellen leer
(EVP-Offset = Modellende, A §2.2).

## Reihenfolge der Abschnitte (Layout des Writers)

Kopf (0x40), Knochen-Wörterbuch an 0x40, Knochen lückenlos, SBC (auf 4 aufgefüllt),
Materialabschnitt, Shape-Abschnitt, EVP. So liegen alle Vanilla-Kartenmodelle
(SBC > Material > Shape > EVP, A §2.2). Der Writer (`Nsbmd::write`) rechnet jeden Offset
neu; Wörterbücher, Paarungen, SBC, Materialfelder, Shape-Köpfe und GPU-Befehle kodiert er
aus der Struktur. Grenzen: ein Wörterbuch über 255 Einträge, ein doppelter Name, eine
Paarung über 255 Materialien, ein Offset des Materialabschnitts über `0xFFFF`, ein Knochen,
dessen `rest` nicht `bone_len(flags) − 2` lang ist (`BadSize`), oder ein GPU-Befehl mit
falscher Parameterzahl sind Fehler, nie eine Panik.

## Was der Codec als Bytes hält

- die Bytes eines Knochens nach dem Flagwort (`Bone::rest`),
- die Bytes eines Materials ab 44 (`Material::extra`, Texturmatrix, 0/4/8 Byte),
- den EVP-Bereich (`Model::evp`),
- die Felder 0x14 bis 0x3F des Modellkopfs einzeln (Zähler und Rahmen nie neu berechnet).

Alles andere wird dekodiert und neu kodiert. Keine Leser-Regel musste für Vanilla zu einem
Rohfeld werden: alle Wörterbücher haben das strenge Layout und Datengröße 4, alle Materialien
44, 48 oder 52 Byte, alle SBC- und GPU-Befehle stehen in den Tabellen.

## Verifikation

- `crates/g4-formats/src/nsbmd/model.rs`, Tests: synthetisches Modell (zwei Knochen
  `0xF807` und `0x0002`, Bäcker-SBC über zwei Materialien zu 44 und 52 Byte, Paarungen
  `t0 → [0]`, `t1 → [1]`, `p0 → [0, 1]`, zwei Shapes mit Quads und Quad-Strip) byte-gleich im
  Round-Trip; Layout in Vanilla-Reihenfolge; `nsbmd_header` liest dessen Namen; Zähler;
  Materialgröße 56 und `TEX0` abgelehnt; jede Kürzung ist ein Fehler, zufällig veränderte
  Bytes führen nie zu einer Panik (proptest); `dict_spans` liefert sechs Wörterbücher.
- Gated `crates/g4-formats/tests/vanilla_nitro.rs` (`G4_TEST_PROJECT`, nur lesend): jedes
  Kartenmodell aus `unpacked/maps/NNNN` byte-gleich neu geschrieben, jedes Wörterbuch aus
  `dict_spans` einzeln byte-gleich, veraltete Zähler gezählt. Ergebnis (Release-Profil):

| Spiel | Modelle | byte-gleich | Wörterbücher | veraltete Zähler |
|-------|---------|-------------|--------------|------------------|
| Platin DE (CPUD) | 666 | 666 | 3 996 | keine |
| HeartGold DE (IPKD) | 676 | 676 | 4 056 | 0468, 0475, 0476 |
| HeartGold US (IPKE) | 676 | 676 | 4 056 | 0468, 0473, 0475, 0476 |

Der Lauf je Spiel dauert im Release-Profil rund 2 bis 3 s.
