# Phase 6.1 „Writer“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Das Studio schreibt NSBMD und NSBTX selbst: ein Nitro-Wörterbuch mit Patricia-Baum (`nitro_dict`), ein MDL0-Codec (`nsbmd`: Modellkopf, Knochen, SBC, Materialien mit Paarungen, Shapes mit dekodierter GPU-Display-List; Leser und Writer, der alle Offsets neu berechnet), `nsbtx::write`/`append`/`build` im Vanilla-Layout, ein Texturkonverter RGBA → 16/4 Farben (`texconv`, eigener Median-Cut in BGR555), im Kernel der Bäcker `bake` aus einer neutralen Flächenliste (Quad-Strips, `VTX_10`/`VTX_16`, Stil je Gebiet, Budget-Bericht mit Prüfcodes), „Textur an Paket anhängen“ als Undo-Schritt und das Gate-Werkzeug `bake_test`; byte-gleicher Round-Trip aller Vanilla-Kartenmodelle, Texturpakete und Wörterbücher der drei Spiele als gated Test, Orakel apicula und g4emu.

**Architecture:** `g4-formats` bekommt `nitro_dict.rs`, `nsbmd/{mod,gpu,sbc,model}.rs`, `nsbtx/write.rs` (Untermodul von `nsbtx.rs`) und `texconv.rs`; `nsbmd_header` bleibt die dünne Hülle für `map_import.rs` und liest seine Wörterbücher über `nitro_dict`. Der Round-Trip ist Prüfung, nicht Produktpfad: unveränderte Modelle und Pakete bleiben Bytes. Der Kernel bekommt `bake/{mod,strips,encode,textures,test_surface}.rs`: `bake()` ist rein (Flächen + Pakettexturen + Stil → `Model` → Bytes + Bericht), `textures.rs` hängt Texturen als Dateischritt an das Kartentextur-Paket (`unpacked/mapTextures/NNNN`) und urteilt über VRAM je Gebiet, `test_surface.rs` baut die Testfläche aus Gutachten A §2.7 und `bake_test`, das sie über `map_import` in eine Karte schreibt. Das Studio reicht `bake_test` als Befehl (MCP nur über `g4_call`, `--expert`) durch; `g4k check-data` lernt `map_texture:N:sha1`.

**Tech Stack:** Rust (`g4-formats` ohne neue Abhängigkeit, nur `thiserror`; Tests mit `proptest`), `g4-kernel` (Store, Dateischritte, `map_import`, `export::nsbmd_to_glb`, `png` für den Orakel-Test), Tauri-Befehle und MCP (`dispatch.rs`, `access.rs`), apicula `3d4e91e` als Orakel, Python 3 (`scripts/g4emu`, Szenarien über `studio mcp`).

**Spec:** [2026-11-05-phase-6-kartenerstellung-design.md](../specs/2026-11-05-phase-6-kartenerstellung-design.md) (§1, §2, §3.3, §3.7, §4 Zeile 6.1, §7, §8); [entscheidungen.md](../../research/2026-11-05-phase-6/entscheidungen.md) (Tabellen „Phasenschnitt und Reihenfolge“ Zeile 6.1, „Writer (Gutachten A)“ wörtlich, „Nicht in Phase 6“, „Was bewusst nicht entschieden ist“); [Gutachten A](../../research/2026-11-05-phase-6/gutachten-a-writer.md) (§2.2 bis §2.4 Inventar, §2.5 Teilmenge, §2.6 Round-Trip und Patricia-Baum, §2.7 Prototyp im Spiel, §2.8 NSBTX, §2.9 Lizenzen, §4 Empfehlung, §5 Schnitt und Gate, §6 Risiken, §7 offene Punkte, §8 Nebenbefunde) mit den eigenen Prototypen `gutachten-a-nitro.py` (Layout-Writer `write_model`/`write_nsbmd`, `write_nsbtx_canonical`, `build_tree`), `gutachten-a-minimal.py` (`build_nsbmd`, `STYLE`, `cmd_size`, `merge_btx`) und `gutachten-a-proto/src/main.rs` (`median_cut`, `quantize`, `write_btx0`, `synth_images`); [ROADMAP.md](../../ROADMAP.md) „Phase 6“, Zeile 6.1.

Stand 05.11.2026: `main` bei `d05fad84` (Spezifikation, Gutachten und Entscheidungen zu Phase 6). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-1`, Branch `phase-6-1-writer` (legt der Umsetzer an: `git worktree add .claude/worktrees/phase-6-1 -b phase-6-1-writer d05fad84`). Parallel laufen 6.2 und 6.3a in eigenen Worktrees (Abschnitt „Parallele Teilphasen“). Formate `crates/g4-formats/src`, Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Emulator `scripts/g4emu`.

## Voraussetzungen (auf main `d05fad84`, vor Task 1 mit `grep` prüfen)

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `g4_formats::error::FormatError::{Truncated, BadMagic, BadSignature, BadSize, OutOfRange, InvalidUtf8, Unsupported}` | wie in `error.rs` heute (kein neuer Fall nötig) | alle |
| `g4_formats::nsbmd_header::{NsbmdHeader, strip_tex0, model_count, material_texture_names, material_palette_names, HEADER_LEN}`, privat `dict_at` | Container-Prüfung, Namen über das private Wörterbuch | Task 3 |
| `g4_formats::nsbtx::{Nsbtx::read, Nsbtx::decode, TextureInfo, PaletteInfo, Rgba}`, `pub(crate) fn rgb` | Leser und Dekoder, Format 5 abgelehnt | Task 4, 5, 8 |
| `g4_formats::map::MapFile { model, bdhc, .. }`, `MapFile::read(bytes, family)` | Modell als undurchsichtige Bytes | Task 3, 7 |
| `g4_formats::area_data::AreaData::{Dppt { light_type, .. }, Hgss { light_type, .. }}`, `buildings_tileset()`, `map_tileset()` | Lichttyp je Familie | Task 6, 7 |
| `g4_formats::test_support::{bmd0, mdl0_block, btx0}` (Feature `test-support`) | synthetische Container | Task 3, 4 |
| `g4_kernel::map_import::{import, inspect, Part, PartMessage, PartState}`, privat `map_packs`, `pub(crate) check_model_bytes` | Modellprüfung vor dem Import, Pakete je Karte über Header → Matrix → Area-Data | Task 7 |
| `g4_kernel::store::{Store::{edit_map, record_file_step, squash_since, undo_depth, undo, redo, history, member_count, area_data, header_table, matrix, map, current_bytes, project}, FileStep, FileChange, FileStepKind::Reset, write_file_step (pub(crate))}` | Dateischritt wie `original::reset_files` | Task 7, 8 |
| `g4_kernel::project::{Project::{member_path, ensure_unpacked, map_tileset_member}, AssetKind::{MapTexture, BuildingTexture, AreaData, Map}, AssetId}`, `checks::limits::limits().model.max_bytes.value` (0xF000) | | Task 6, 7 |
| `g4_kernel::export::{nsbmd_to_glb, nsbmd_to_glb_with_timeout}` | `apicula convert -f=glb …`, genau eine GLB | Task 8 |
| `g4_kernel::test_support::{synthetic_project, vanilla_copy}`, `tests/common/mod.rs` | Kartentextur-NARC mit einem Platzhalter-Member, Area-Data 0 = Paket 0 | Task 7, 8 |
| `g4k check-data` (`DataKind`, `MAP_FIELDS`, `parse_expectation`, `map_field`) in `crates/g4-kernel/src/bin/g4k.rs` | Arten `species … map, egg_moves` | Task 9 |
| Studio: `commands/dispatch.rs` (`COMMANDS`, `run!`, Test `every_command_accepts_the_keys_api_ts_sends`, `commands_match_the_tauri_handler`), `commands/mod.rs` (`mod …; pub use …::*`), `lib.rs` (`generate_handler!`), `mcp/access.rs` (`COMMAND_KINDS` in Reihenfolge von `COMMANDS`), `commands/policy.rs` (`Policy::write_path`: Schreiben nach `<projekt>/build`) | | Task 9 |
| `scripts/g4emu/g4emu/scenario.py`: `mcp: [--allow-irreversible, --expert]`, `[g4_call, {command, args}, name]`, `check:` mit `${sha1-of:PFAD}`, `build: build/out.nds`; `games.yaml` CPUD/IPKD/IPKE mit `*-zimmer.sav` | wie in `pt-flag.yaml` | Task 10 |
| apicula `3d4e91e`: Positionen der GLB in Welteinheiten (SBC `0x0B` multipliziert mit `up_scale`, `src/primitives/mod.rs:244`, `:411`), `TEXCOORD_0` = (s / Breite, t / Höhe) (zweimal gespiegelt, `primitives/mod.rs:388`, `convert/gltf/mod.rs:125`), `--more-textures` rät Paletten `<name>` und `<name>_pl` | 0BSD, nur gelesen | Task 8 |

## Parallele Teilphasen

**6.2 BDHC** (Worktree `phase-6-2`, Branch `phase-6-2-bdhc`) legt `crates/g4-formats/src/bdhc.rs`, `g4-kernel/src/terrain/*`, `metadata/terrain_rules.yaml`, Prüfungen in `checks/`, Befehle `get_map_heights` und „Höhenraster setzen“, `apps/studio/src/world/scene/*` und in `scripts/g4emu` `expect-height` und den Farbfix an. **6.3a Weltformat** (Worktree `phase-6-3a`, Branch `phase-6-3a-weltformat`) legt `g4-kernel/src/world/*` (Vernetzer mit `MeshFace`, später per `From` auf `bake::Face`), `textproject/*`, `store/*` und `worldMesh.ts` an. **6.1 baut keine dieser Dateien um**; in `scripts/g4emu` kommen nur die neuen Szenario-Dateien dazu, `textproject` und `store.rs` bleiben unberührt. Die Schnittstelle `bake::Face` (Task 6) ist mit 6.3a abgestimmt und gilt wörtlich. Anhängepunkte (nur anhängen): `crates/g4-formats/src/lib.rs`, `crates/g4-kernel/src/lib.rs`, `apps/studio/src-tauri/src/{lib.rs, commands/mod.rs, commands/dispatch.rs}`, `mcp/access.rs`, `crates/g4-kernel/src/bin/g4k.rs` (Art `map_texture` neben einem möglichen `bdhc_sha1` von 6.2), `docs/mcp.md`, `docs/ROADMAP.md`, `docs/agent-gate.md`. `scripts/gate.sh` und `design-system/README.md` bleiben unverändert (die neuen Tests laufen in den vorhandenen Schritten mit, 6.1 hat keine Oberfläche). In `map_import.rs` kommt nur hinzu, was `bake_test` braucht (Task 7).

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Dateien | `nitro_dict.rs`; `nsbmd/mod.rs` (Container, `Nsbmd`, `dict_spans`), `nsbmd/gpu.rs`, `nsbmd/sbc.rs`, `nsbmd/model.rs`; `nsbtx/write.rs` als Untermodul von `nsbtx.rs` (keine Umbenennung von `nsbtx.rs`), Re-Export `nsbtx::{write, append, build, Btx0, NewTexture, …}`; `texconv.rs`. In `lib.rs` je eine eigene Gruppe am Ende (Leerzeile davor, wie in `g4-kernel/src/lib.rs`), damit rustfmt nichts nach oben sortiert | Entscheidung „Modul `nsbmd` und `nitro_dict`“; kleine Dateien je Verantwortung; Anhängen ohne Konflikt mit 6.2 (`bdhc`) |
| Namen | `nitro_dict::Name = [u8; 16]`; gelesene Namen bleiben Bytes; neue Namen über `nitro_dict::name(text)`: 1 bis 16 Byte, jedes Byte `0x21..=0x7E` (sonst `OutOfRange { what: "Nitro name", .. }`) | Vanilla-Namen sind ASCII ohne Leerzeichen; ein leerer Name wäre der Schlüssel der Wurzel |
| Wörterbuch | Layout `u8 0`, `u8 n`, `u16 Länge`, `u16 8`, `u16 12 + 4n` (Eintragsblock), `n + 1` Knoten zu 4 Byte, `u16 unit`, `u16 4 + unit·n`, Daten, `n` Namen. Der Leser lehnt jede Abweichung davon ab (`BadSignature`/`BadSize`), den Baum liest er nicht (der Writer rechnet ihn neu, Byte-Gleichheit prüft ihn); 0 Einträge sind erlaubt (nur Wurzel), 1 bis 255 getestet, ab 256 `OutOfRange`, ein doppelter Name `Unsupported { what: "duplicate name in a Nitro dictionary" }` | Gutachten A §2.6: mit dieser Regel sind alle 13 198 Vanilla-Wörterbücher byte-gleich; ein Leser, der nur reproduzierbare Layouts annimmt, macht jeden Writer-Fehler sichtbar |
| Patricia-Baum | Algorithmus unter Task 1 wörtlich (Sedgewick, danach Vorordnung) | Gutachten A §2.6; der Baum fällt in apicula nicht auf |
| Was der Codec als Bytes hält | Knochen nach dem Flag-Wort (`rest`, Länge `bone_len(flags) − 2`), Material ab Byte 44 (`extra`, Texturmatrix, 0/4/8 Byte), EVP-Bereich (`evp`), die Felder 0x14 bis 0x3F des Modellkopfs einzeln (Zähler und Rahmen als Felder, nie neu berechnet beim Round-Trip); alles andere (Wörterbücher, Paarungen, SBC, Material-Felder, Shape-Köpfe, GPU-Befehle) wird dekodiert und neu kodiert | genau die Grenze des Layout-Writers aus Gutachten A §2.6 (`write_model`); veraltete Zähler in HG 0468, 0475, 0476 (US 0473) bleiben erhalten |
| Layout des Writers | Container `BMD0` (16 + 4), ein Block `MDL0` an 0x14 (`MDL0`, `u32` Länge, Modell-Wörterbuch, Modelle); je Modell: Kopf 0x40, Knochen-Wörterbuch an 0x40 (Daten = Offset ab Wörterbuch), Knochen lückenlos, SBC, auf 4 aufgefüllt, Materialabschnitt (`u16` Offset Textur-Paarung, `u16` Offset Paletten-Paarung, Material-Wörterbuch, Textur-Paarungs-Wörterbuch, Paletten-Paarungs-Wörterbuch, Material-Indizes erst aller Textur-, dann aller Paletten-Paarungen, auf 4 aufgefüllt, Materialien lückenlos; alle Offsets ab Abschnittsbeginn; Paarungsdatum `u16 off, u8 n, u8 bound`), Shape-Abschnitt (Wörterbuch, `n` Köpfe zu 16 Byte `u16 tag, u16 size, u32 flag, u32 Offset ab Kopf, u32 Länge`, Display Lists in Reihenfolge), EVP; Kopf-Wörter 0x00 bis 0x13 = Größe, `ofsSbc`, `ofsMat`, `ofsShp`, `ofsEvp` | `gutachten-a-nitro.py` `write_model`, `write_nsbmd`: byte-gleich auf 2 018 Modellen |
| Container-Grenzen des Codecs | genau ein Block `MDL0`; `TEX0` im Modell oder ein zweiter Block → `Unsupported { what: "NSBMD block other than MDL0" }` (der Import entfernt TEX0 vorher mit `strip_tex0`); Materialgröße nicht 44/48/52 → `Unsupported { what: "material size" }`; unbekannter GPU- oder SBC-Befehl → `OutOfRange { what: "GPU command" / "SBC command", value: op, .. }` | alle Vanilla-Kartenmodelle haben nur `MDL0` (A §2.2) |
| GPU-Befehle | allgemein `GpuCmd { op, params: Vec<u32> }` mit der Parametertabelle aus GBATEK (Task 2); `decode` liest Vierergruppen von Opcodes samt `NOP`, `encode` füllt die letzte Gruppe mit `NOP` ohne Parameter | Round-Trip ohne Sonderfälle; die Konstruktoren dienen dem Bäcker |
| NSBTX als Struktur | `Btx0` hält Texel-, 4×4- und Palettenblock als Ganzes, die Einträge mit ihren Rohwörtern (`params`, `extra`), die Felder `0x08`, `0x18`, `0x2C` (VRAM-Schlüssel), `0x10`, `0x20` roh, `0x32` (Palettenflag) und ob `0x1E` auf die Texturen zeigt; `write` legt Kopf 0x3C, Textur-Wörterbuch (unit 8), Paletten-Wörterbuch (unit 4), Texel, 4×4-Texel, 4×4-Index, Paletten hintereinander | `write_nsbtx_canonical`: 545 Pakete byte-gleich (A §2.6); byte-gleich aus Bildern geht nicht (geteilte Texel), ist auch nicht nötig |
| Neue Texturen | Format 3 (16 Farben) und 2 (4 Farben); Texel und Paletten je auf 8 Byte ausgerichtet; `params = off8 \| log2(B/8) << 20 \| log2(H/8) << 23 \| format << 26 \| color0 << 29`; `extra = B \| H << 11 \| 1 << 31`; Palettendatum `extra` = 1 genau für 4-Farben-Paletten; Palettenflag `0x8000`, sobald eine 4-Farben-Textur im Paket ist (beim Anhängen ODER mit dem alten Wert); neues Paket (`build`) Version 1, `append` behält die Version | feste Felder aus A §2.4, `write_btx0` und `merge_btx` der Prototypen |
| Palettenname | `texconv::palette_name(t)`: `t` plus so viel von `_pl`, wie in 16 Byte passt (`gras` → `gras_pl`, 15 Byte → `…_`, 16 Byte → unverändert); in einem Paket müssen Textur- und Palettennamen je eindeutig sein | so rät apicula (`image_namer.rs` `append_pl`), das Orakel findet die Palette ohne Modell |
| Quantisierung | Prototyp `median_cut`/`quantize` mit festen Gleichstandsregeln: deckend heißt Alpha ≥ 128; ein durchsichtiges Pixel → Index 0 und `color0`, dann `k = Farben − 1`; sind es nach Rundung auf BGR555 höchstens `k` verschiedene Farben, sind das die Palette (aufsteigend als `u16`); sonst teilt der Median-Cut die Schachtel mit der größten Spannweite eines Kanals (Gleichstand: kleinster Schachtel-Index, dann R vor G vor B), sortiert stabil nach dem Kanal und teilt bei `len / 2`, bis `k` Schachteln da sind oder keine mehr als ein Pixel hat; Farbe = Mittel in RGB8 (ganzzahlig abgerundet) → BGR555; Palette aufsteigend, doppelte entfernt, mit 0 auf 4/16 aufgefüllt; Zuordnung: kleinster quadratischer Abstand in RGB8 zu `rgb(p)` (Gleichstand: kleinerer Index); kein Dithering | Entscheidung „eigener Median-Cut in BGR555 mit `color0`, ohne Dithering, eine Palette je Textur“; Prototyp gemessen besser als NeuQuant (A §2.8) |
| Bäcker: Koordinaten | `posScale` 64 fest (`up_scale` 262 144, `down_scale` 64); eine Ecke in Welteinheiten ist `fx = 64 · w` in 1/4096 Modelleinheiten; `VTX_10`, wenn alle drei `fx % 64 == 0` und `fx / 64` in −512..=511, sonst `VTX_16` (`fx` in −32 768..=32 767); eine Ecke außerhalb −512..=511 Welteinheiten → Fehler `bake.face-out-of-range` | Entscheidung „`VTX_10` auf dem 1/64-Raster bei `posScale` 64, sonst `VTX_16`“; mit ganzzahligen Ecken ist jede Ecke im Bereich auf dem Raster, `VTX_16` bleibt für die Regel im Encoder (Test mit Bruchteilen) |
| Bäcker: Strips | Läufe gleicher Textur, Ebene und Normale: eine Fläche `g` setzt Fläche `f` fort, wenn `g.c0 == f.c3` und `g.c1 == f.c2` (Position und UV). Kettenanfang ist die erste unbenutzte Fläche in Eingabereihenfolge ohne unbenutzten Vorgänger (sonst die erste unbenutzte); Fortsetzung ist die erste unbenutzte passende Fläche in Eingabereihenfolge. Ketten ab zwei Flächen werden Quad-Strips (`BEGIN_VTXS 3`, Ecken `f0.c0, f0.c1, f0.c3, f0.c2, f1.c3, f1.c2, …`), einzelne Flächen Quads (ein `BEGIN_VTXS 1` je Shape am Ende, Ecken `c0..c3`); `END_VTXS` nach jeder Gruppe | Vorderseite `(c1 − c0) × (c2 − c0)` wie alle waagerechten Vanilla-Quads (A §2.7); Quad-Strip zeichnet `v0, v1, v3, v2` |
| Bäcker: `TEXCOORD` | `TEXCOORD` nur, wenn sich (s, t) gegenüber dem zuletzt in diesem Shape gesendeten ändert; Wert `(uv − Verschiebung) · 16` (1.11.4). Ein Strip wird vor der Fläche geteilt, deren neue Ecken nicht mehr in −2 048..=2 047 Texel passen; je Strip und je Einzel-Quad ist die Verschiebung 0, solange die erste Fläche passt, sonst das größte Vielfache der Texturbreite/-höhe unter dem Minimum; passt eine Fläche auch so nicht (Spanne > 2 047), Fehler `bake.face-uv-range` | Entscheidung „`TEXCOORD` nur bei Wechsel, Strips vor ±2 048 Texeln geteilt“; mit Wiederholen S/T ändert eine Verschiebung um ganze Texturen das Bild nicht, und ohne Not bleibt UV gleich der Eingabe (Orakel) |
| Bäcker: Normale und Farbe | Normale aus `(c1 − c0) × (c2 − c0)` (bei Länge 0 aus `(c2 − c0) × (c3 − c0)`, sonst `(0, 1, 0)`), normiert, `round(v · 511)` auf −512..=511 (1.0.9). Außen: `NORMAL` vor dem ersten Eck jedes Strips bzw. Einzel-Quads, wenn sie sich ändert; innen: `COLOR 0x7FFF` einmal am Anfang jedes Shapes | Stil-Entscheidung „außen eine Normale je Ebene, innen Farbe weiß“; außen ersetzt das Spiel Materialfarben durch die Tageszeit |
| Bäcker: Materialien und Shapes | je Textur ein Material und ein Shape in Reihenfolge des ersten Vorkommens; Materialname = Texturname, Shape `polygon<i>`, Modell `g4_bake`, Knochen `g4_root` (`0xF807`, `rest` = `00 00`); Material `tag` 0, Größe 44, `spe_emi` 0, `poly_attr_mask` `0x3F1FF8FF`, `teximage` `0x00030000` (Wiederholen S und T), `teximage_mask` `0xFFFFFFFF`, `pltt_base` 0, `flag` `0x1FCE`, `mag` (4096, 4096), Breite/Höhe der Textur; außen `dif_amb` `0x7FFFE739`, `poly_attr` `0x001F8081` (Licht 0, Vorderseite, Nebel, Alpha 31), Shape-Flag `0x5`; innen `dif_amb` `0x7FFF8000`, `poly_attr` `0x001F0080`, Shape-Flag `0x6`; Textur-Paarung je Material (`bound` 0), Paletten-Paarung je **Palettenname** mit allen Materialien dieser Palette; SBC `26 00 00 00 00 02 00 01 0B (04 ii 05 ii)* 2B 01`; Kopf `num_objects` 1, `num_materials` = `num_shapes` = Zahl der Texturen, `first_unused_mtx` 1, sonst 0 | Werte aus A §4 und `gutachten-a-minimal.py` `STYLE`/`build_nsbmd`; ein Wörterbuchname darf nur einmal vorkommen, Vanilla teilt Paletten zwischen Materialien |
| Bäcker: Rahmen und Zähler | Zähler aus der Display List (`Geometry`); Rahmen aus allen `fx`: `ext` = größter Betrag von Minimum, Maximum und Ausdehnung über alle Achsen; `box_scale` = kleinste Zweierpotenz ≥ 1 mit `ext / box_scale < 7,99 · 4096`; `box_pos = round(min / box_scale)`, `box_size = round((max − min) / box_scale)`, Felder `box_scale · 4096` und `4096 / box_scale` | Prototyp `build_nsbmd`; das Spiel liest die Zähler nicht (A §2.5), der Test verlangt sie trotzdem gleich |
| Stil je Gebiet | `AreaStyle::of(&AreaData)`: Platin/DP Lichttyp 0 oder 3 → `Outdoor`, HGSS Lichttyp 1 → `Outdoor`, sonst `Indoor` | Entscheidung, HG-Regel gemessen (geprüft erst in 6.3b) |
| Prüfcodes | `bake::BakeMessage { code: BakeCode, level: Level, args, message }` statt `PartMessage`: Fehler `bake.empty` (keine Fläche), `bake.model-too-big` (über `model.max_bytes` 0xF000), `bake.texture-missing` (Name nicht im Paket oder ohne erkennbare Palette), `bake.face-out-of-range`, `bake.face-uv-range`, `texture.side` (keine Zweierpotenz 8..=1024), `texture.name` (leer, über 16 Byte, Zeichen außerhalb `0x21..=0x7E`), `texture.taken` (Textur- oder Palettenname im Paket oder doppelt in der Anfrage), `texture.pack-full` (über 255 Einträge, Texel über 262 144 Byte, Paletten über 65 536 Byte oder ein Offset über `0xFFFF · 8`); Warnung `bake.vram-high` | `PartCode::ALL` ist an die Wörter des Import-Dialogs gebunden (Studio-Test gegen `importParts.ts`), 6.1 hat keine Oberfläche; „oder gleichwertig“ der Vorgabe |
| VRAM | VRAM eines Pakets = Texel + 4×4-Texel + 4×4-Index + Paletten; je Gebiet Kartenpaket + Gebäudepaket; Warnung `bake.vram-high` über `VANILLA_VRAM_MAX_PT` 110 640 (Platin, DP) bzw. `VANILLA_VRAM_MAX_HGSS` 87 424 Byte; beim Anhängen für **jedes** Gebiet, dessen `map_tileset` das Paket ist; hart nur die Bänke (256 KiB Texel, 64 KiB Paletten, `texture.pack-full`); Konstanten im Kernel, nicht in `check_limits.yaml` (gemessen, keine Engine-Grenze) | Entscheidung „Warnung über dem Vanilla-Maximum“; Pakete sind geteilt |
| Anhängen im Store | `append_textures` schreibt `unpacked/mapTextures/NNNN` als `FileStep` (`FileStepKind::Reset`, Asset `MapTexture NNNN`) wie `original::reset_files`, Label „Kartentexturen 0020: 2 Texturen angehängt“; im Textprojekt gilt der vorhandene Abgleich (Member weicht von der Basis ab → `data/map_textures/NNNN.nsbtx`), keine eigene Sync-Logik | die Art ist kein Store-Wert (`NotEditable`); derselbe Weg wie 4.1b |
| `bake_test` | Kernel `bake::bake_test(store, &BakeTestOptions)`: Karte `map`, Paket und Gebiet aus dem ersten Header, der die Karte zeigt (`map_import::map_areas`); hängt `g4_bake_a` (16 × 16, 16 Farben) und `g4_bake_b` (16 × 16, 4 Farben) an, soweit das Paket sie noch nicht hat (vorhandene werden wiederverwendet); Boden `[g4_bake_a, V0, g4_bake_b, V1]` mit `V0`, `V1` = die ersten zwei Texturen des Pakets mit erkennbarer Palette; Block aus `g4_bake_a` an `block` oder an der Mitte des Rahmens des alten Modells (`cx = (tx0 + tx1) / 2`, `cz = (tz0 + tz1) / 2 + 1`, auf 0..=30 begrenzt; nicht lesbar: (15, 15)); schreibt das Modell über `map_import::import_model_bytes`; Anhängen und Import werden mit `squash_since` ein Schritt „Testfläche gebacken: Karte 0187“; mit `out_dir` zusätzlich `bake-NNNN.nsbmd` und `maptex-NNNN.nsbtx` dorthin (für `${sha1-of:…}`). Studio-Befehl `bake_test { map, block?: [x, z], outDir? }` (Write), kein kuratiertes Werkzeug (MCP über `g4_call` mit `--expert`) | Gutachten A §5 „Werkzeug“; ersetzt durch das Malen (6.4a), darum nicht im Katalog (Zählgrenze `the_curated_tool_count…`) |
| `g4k check-data` | neue Art `map_texture`, Feld `sha1` (SHA-1 des ganzen Members, 40 Hex) | „`check:` mit `${sha1-of:…}` belegt, dass `out.nds` Modell und Paket enthält“ (A §5 Punkt 5) |
| Szenarien | `pt-bake.yaml` (CPUD, Karte 187, Header 415 → 414), `hg-bake.yaml` (IPKD, Karte 217, Header 64 → 63), dazu `hgus-bake.yaml` (IPKE, gleiche Schritte; `game:` steht fest in der Datei) | Gate „g4emu auf den drei Spielen“; Zuordnung aus A §5 Punkt 5 |
| Doku | `docs/formats/nsbmd.md` neu (Container, Wörterbuch mit Baum, Modellkopf, Knochen, SBC, Material, Paarungen, Shapes, GPU-Befehle, Teilmenge des Bäckers, Quellen pret/apicula/GBATEK/eigene Messung); `nsbmd_header.md`: Quelle PDSMS durch apicula, GBATEK, eigene Messung ersetzt, Verweis auf `nsbmd.md`; `nsbtx.md`: Abschnitt „Schreiben“, im Kopf „Nur Lesen“ gestrichen und die DSPRE-Nennung durch apicula `decode_texture.rs` und GBATEK ersetzt; `docs/dev-setup.md` bleibt unverändert (Spezifikation §8), die neuen gated Tests stehen in der Gate-Datei und in `agent-gate.md` | Entscheidung „Quelle in `nsbmd_header.md`“; dieselbe Lizenzlinie für `nsbtx.md` |
| Nicht in 6.1 | A3I5, A5I3, 256 Farben, 4 × 4 und direkte Texturen schreiben; gemeinsame Paletten; Dithering; Texturen ersetzen oder löschen; eigene Pakete; Bearbeiten von Vanilla-Modellen über den Codec; Vernetzer (6.3a); Oberfläche (6.5a); `quantette` | „Nicht in Phase 6“ und Schnitt |

## Global Constraints

- Arbeitsort: Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-1`, Branch `phase-6-1-writer` (Basis main `d05fad84`). Nie `cd` in die Hauptkopie, nie Branch wechseln, nie nach main mergen, kein `git stash`. `tools/` im Worktree als Symlink auf `$HOME/Projekte/lab/editor/gen4-studio/tools` anlegen (nie einchecken) oder `G4_APICULA=$HOME/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula` setzen (`docs/agent-gate.md`, „Einmalig einrichten“, Punkt 3).
- cargo braucht `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`. Löst `-p g4-kernel` im Worktree den Download in rotoms Build-Skript aus, über den Workspace mit Namensfilter testen (`cargo test --workspace --all-features -- <filter>`).
- Jeder Commit endet mit der Zeile `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`).
- `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut. Originale unter `Roms/` nie öffnen oder ändern; Testkopien nur `Roms/_testkopien/6-1-<zweck>_DSPRE_contents` (`cp -cR`) mit `6-1-<zweck>.nds` daneben (`cp -c`). Gated Tests lesen `G4_TEST_PROJECT` (absolut) und schreiben nur auf `vanilla_copy()`.
- **Keine ROM-Daten, keine ROM-Ableitungen im Repo**: Fixtures (Modelle, Pakete, Bilder) entstehen synthetisch aus Code, nie als kopierte Bytes oder Hexdump; Ausgaben der gated Tests nur Zahlen und Namen; Bilder der Szenarien bleiben unter `target/agent/`. `bash scripts/check-no-rom-data.sh` grün.
- **Lizenz**: nichts aus DSPRE (`$HOME/Projekte/lab/editor/DSPRE`, AGPL) oder PDSMS (keine Lizenz) lesen oder übernehmen; erlaubt sind pret/pokeplatinum, apicula (0BSD), GBATEK, die eigenen Prototypen unter `docs/research/2026-11-05-phase-6/`. Keine neue Abhängigkeit; nur falls der eigene Median-Cut das Gate nicht trägt, `quantette` (MIT/Apache-2.0) mit eigenem Commit und `cargo deny --locked check licenses advisories` grün.
- Lang laufende Befehle (gated Tests, `cargo test --workspace`, Szenarien) als Hintergrund-Befehl mit Log: `mkdir -p target/agent/6-1 && <befehl> > target/agent/6-1/<task>-<name>.log 2>&1; echo "exit $?" >> target/agent/6-1/<task>-<name>.log`; das Ergebnis aus dem Log lesen.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei `scripts/g4emu` `cd scripts/g4emu && uv run pytest -m "not integration" -q`.
- Kein `unwrap`/`expect`/Panik in Produktcode der Codecs: jeder Lesezugriff längengeprüft, Fehler als `FormatError` (Vorbild `nsbtx.rs`, `slice_at`).
- Anhängepunkte nur anhängen; die Dateien von 6.2 und 6.3a (Abschnitt „Parallele Teilphasen“) nicht anfassen.

## Review Focus

- Namen am Rand: ein leerer Name, 16 Byte ohne NUL, lange gemeinsame Präfixe, Namen, die sich nur im letzten Byte unterscheiden, und Doppelte: der Baum findet jeden Eintrag mit der Suche des Spiels, Doppelte und leere Namen werden abgelehnt (Task 1 `every_name_is_found_by_the_games_search`, `names_that_differ_only_in_the_last_byte_are_found`, `duplicate_and_empty_names_are_refused`).
- Kaputte oder fremde Eingaben an die Leser (gekürzt, einzelne Bytes verfälscht, unbekannter GPU- oder SBC-Befehl, Materialgröße 56, TEX0 im Modell, gekürztes Paket): immer `FormatError`, nie Panik (Task 2 `random_bytes_never_panic_the_decoders`, Task 3 `every_truncation_is_an_error_not_a_panic`, `random_byte_flips_never_panic`, Task 4 `truncated_and_flipped_packs_are_errors`).
- UV jenseits ±2 048 Texel (64-Texel-Textur mit 64 Texeln je Kachel über 32 Kacheln, weit verschobene UV, eine einzelne zu breite Fläche): Strips werden geteilt, verschoben um ganze Texturen, zu breite Flächen abgelehnt (Task 6 `long_strips_split_before_the_texcoord_range`, `a_face_far_out_in_uv_is_shifted_by_whole_textures`, `a_face_wider_than_the_texcoord_range_is_refused`).
- Ein geteiltes Paket: Anhängen in einem Gebiet ändert den VRAM jedes Gebiets mit diesem Paket; Namen schon im Paket, doppelt in einer Anfrage oder als Palettenname nach dem Kürzen gleich (Task 7 `vram_is_judged_for_every_area_of_the_pack`, `append_refuses_side_name_taken_and_full`, `palette_names_that_collide_after_cutting_are_taken`).
- Bilder am Rand und Wiederholung: ganz durchsichtig, eine Farbe, höchstens `k` Farben, keine Zweierpotenz; `bake_test` zweimal auf derselben Kopie hängt nicht doppelt an (Task 5 `all_transparent_is_one_colour0_texture`, `few_colours_are_kept_exactly`, `sides_must_be_powers_of_two`; Task 8 `bake_test_twice_reuses_the_own_textures`).

---

### Task 1: `nitro_dict`: Wörterbuch mit Patricia-Baum

**Files:**
- Create: `crates/g4-formats/src/nitro_dict.rs` (+ Tests im Modul)
- Modify: `crates/g4-formats/src/lib.rs` (`pub mod nitro_dict;` in eigener Gruppe am Ende)

**Interfaces:**
- Consumes: `error::FormatError`.
- Produces:

```rust
pub const NAME_LEN: usize = 16;
pub const MAX_ENTRIES: usize = 255;
pub type Name = [u8; NAME_LEN];
/// A new name: 1 to 16 bytes, each 0x21..=0x7E, NUL-padded.
pub fn name(text: &str) -> Result<Name, FormatError>;
/// The name as the readers show it: trailing NULs cut, bytes below 0x20 as '.'.
pub fn name_text(name: &Name) -> String;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dict { pub names: Vec<Name>, pub unit: u16, pub data: Vec<u8> }   // data: n · unit bytes
impl Dict {
    pub fn entry(&self, i: usize) -> &[u8];
    pub fn byte_len(&self) -> usize;                        // 8 + 4(n + 1) + 4 + n·unit + 16n
    pub fn write(&self) -> Result<Vec<u8>, FormatError>;    // tree computed here
}
/// The dictionary at `off` and the offset of its end; refuses any layout `write` would not produce.
pub fn read(bytes: &[u8], off: usize) -> Result<(Dict, usize), FormatError>;
/// The nodes [ref_bit, left, right, entry] of the vanilla tree; node 0 is the root.
pub fn tree(names: &[Name]) -> Result<Vec<[u8; 4]>, FormatError>;
```

Algorithmus `tree` (aus `build_tree`, gemessen gegen alle Vanilla-Wörterbücher): Bit `b` eines Namens = Bit `b & 31` des `u32` LE an Byte `4 · (b >> 5)`. Start `nodes = [[127, 0, 0, 0]]`. Für jeden Eintrag `i` mit Schlüssel `k`: Suche `p = 0, x = nodes[0].left`, solange `nodes[p].ref > nodes[x].ref`: `p = x`, `x = bit(k, nodes[x].ref) ? right : left`; `other` = Schlüssel von `x` (Knoten 0: 16 Nullbytes, sonst `names[nodes[x].entry]`); `diff` = höchstes Bit, in dem `k` und `other` verschieden sind (keins → doppelt); zweiter Abstieg wie die Suche, aber nur solange zusätzlich `nodes[x].ref > diff`; neuer Knoten `t = nodes.len()`: `bit(k, diff) ? [diff, x, t, i] : [diff, t, x, i]`; Verknüpfen: `p == 0` → `nodes[0].left = t`, sonst `bit(k, nodes[p].ref) ? nodes[p].right = t : nodes[p].left = t`. Danach Nummerierung in Vorordnung: `order = [0]`, ab `nodes[0].left` (wenn `n > 0`) Knoten aufnehmen, dann seine Kinder links vor rechts, soweit `c != 0` und `nodes[c].ref < nodes[i].ref`; Knoten in dieser Reihenfolge ausgeben, `left`/`right` umnummeriert.

- [ ] **Step 1: Scheiternde Tests** in `nitro_dict.rs`:
  - `zero_entries_is_the_root_alone`: `tree(&[]) == [[127, 0, 0, 0]]`; `Dict { names: vec![], unit: 4, data: vec![] }.write()` hat 16 Byte, `read` liest es zurück mit Ende 16.
  - `one_entry_is_root_and_one_node`: `tree(&[name("a")])` = `[[127, 1, 0, 0], [6, 0, 1, 0]]`; `two_entries_hand_computed`: `["a", "b"]` = `[[127, 1, 0, 0], [6, 0, 2, 0], [1, 1, 2, 1]]`.
  - `preorder_renumbering_moves_a_later_parent_up`: `["a", "aa"]` = `[[127, 1, 0, 0], [14, 2, 1, 1], [6, 0, 2, 0]]` (ohne Umnummerierung stünde der Knoten mit Bit 14 an Index 2).
  - `every_name_is_found_by_the_games_search` (proptest, 1 bis 255 verschiedene Namen aus `[a-z0-9_]{1,16}`): eine Suche im Test (Wurzel, dann Bits bis zur Aufwärtskante) endet für Name `i` bei Eintrag `i`; jede Abwärtskante führt zu einem höheren Index.
  - `names_that_differ_only_in_the_last_byte_are_found`: 16-Byte-Namen `aaaaaaaaaaaaaaa1` … `…9` ohne NUL.
  - `write_then_read_round_trips_1_to_255_entries`: `n` in `[1, 2, 3, 16, 255]`, Namen `t{i}`, `unit` 4 und 8 → `read(write) == (dict, byte_len)`.
  - `duplicate_and_empty_names_are_refused`: `tree(&[a, a])` Fehler; `name("")`, `name("abcdefghijklmnopq")` (17), `name("a b")`, `name("ä")` Fehler.
  - `more_than_255_entries_is_out_of_range`.
  - `read_refuses_layouts_the_writer_cannot_reproduce`: Revision 1, Wort `+4` ≠ 8, Eintragsoffset ± 4, Namensoffset ± 4, Längenfeld ± 4 → Fehler; jede Kürzung `0..len` → Fehler, keine Panik.
- [ ] **Step 2: Lauf** `cargo test -p g4-formats nitro_dict::` → FAIL (Modul fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces.
- [ ] **Step 4: Lauf** → PASS; `cargo clippy -p g4-formats --all-targets --all-features -- -D warnings`, `cargo fmt --check`.
- [ ] **Step 5: Commit** `feat(formats): nitro dictionary with the vanilla patricia tree` (mit Trailer).

### Task 2: GPU-Display-List und SBC

**Files:**
- Create: `crates/g4-formats/src/nsbmd/mod.rs` (erst nur `pub mod gpu; pub mod sbc;`), `crates/g4-formats/src/nsbmd/gpu.rs`, `crates/g4-formats/src/nsbmd/sbc.rs` (+ Tests je Datei)
- Modify: `crates/g4-formats/src/lib.rs` (`pub mod nsbmd;` in der Gruppe von Task 1)

**Interfaces:**
- Produces:

```rust
// nsbmd/gpu.rs
pub mod op { pub const NOP: u8 = 0x00; pub const COLOR: u8 = 0x20; pub const NORMAL: u8 = 0x21;
    pub const TEXCOORD: u8 = 0x22; pub const VTX_16: u8 = 0x23; pub const VTX_10: u8 = 0x24;
    pub const VTX_XY: u8 = 0x25; pub const VTX_XZ: u8 = 0x26; pub const VTX_YZ: u8 = 0x27;
    pub const VTX_DIFF: u8 = 0x28; pub const BEGIN_VTXS: u8 = 0x40; pub const END_VTXS: u8 = 0x41; }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Prim { Triangles = 0, Quads = 1, TriangleStrip = 2, QuadStrip = 3 }
#[derive(Debug, Clone, PartialEq, Eq)] pub struct GpuCmd { pub op: u8, pub params: Vec<u32> }
pub fn param_count(op: u8) -> Option<usize>;
pub fn decode(bytes: &[u8]) -> Result<Vec<GpuCmd>, FormatError>;
pub fn encode(cmds: &[GpuCmd]) -> Result<Vec<u8>, FormatError>;   // wrong param count → BadSize
pub fn is_vertex(op: u8) -> bool;                                   // 0x23..=0x28
impl GpuCmd {
    pub fn begin(prim: Prim) -> GpuCmd; pub fn end() -> GpuCmd; pub fn color(bgr555: u16) -> GpuCmd;
    pub fn normal(n: [i32; 3]) -> GpuCmd;     // 1/512, each −512..=511, x | y << 10 | z << 20 (10 bits each)
    pub fn texcoord(st: [i32; 2]) -> GpuCmd;  // 1/16 texel, each −32768..=32767, s | t << 16
    pub fn vtx_10(v: [i32; 3]) -> GpuCmd;     // 1/64 model unit, each −512..=511
    pub fn vtx_16(v: [i32; 3]) -> GpuCmd;     // 1/4096 model unit: [x | y << 16, z] (16 bits each)
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Geometry { pub verts: u32, pub polys: u32, pub tris: u32, pub quads: u32 }
/// Adds the vertices and polygons of one display list (Triangles n/3, Quads n/4, TriangleStrip n−2, QuadStrip (n−2)/2).
pub fn count(cmds: &[GpuCmd], into: &mut Geometry);
// nsbmd/sbc.rs
#[derive(Debug, Clone, PartialEq, Eq)] pub struct SbcOp { pub op: u8, pub params: Vec<u8> }
/// The render commands from `start` up to and including the first 0x01, and the offset after it; past `end` → Truncated.
pub fn decode(bytes: &[u8], start: usize, end: usize) -> Result<(Vec<SbcOp>, usize), FormatError>;
pub fn encode(ops: &[SbcOp]) -> Vec<u8>;
```

Parameterzahlen (GBATEK, wie `GPU_PARAMS` in `gutachten-a-nitro.py`): `0x00` 0, `0x10` 1, `0x11` 0, `0x12` 1, `0x13` 1, `0x14` 1, `0x15` 0, `0x16` 16, `0x17` 12, `0x18` 16, `0x19` 12, `0x1A` 9, `0x1B` 3, `0x1C` 3, `0x20` bis `0x2B` je 1 außer `0x23` 2, `0x30` bis `0x33` je 1, `0x34` 32, `0x40` 1, `0x41` 0, `0x50` 1, `0x60` 1, `0x70` 3, `0x71` 2, `0x72` 1. SBC (`SBC_PARAMS`): `0x00` 0, `0x01` 0, `0x02` 2, `0x03` 1, `0x04` 1, `0x05` 1, `0x06` 3, `0x07` 1, `0x08` 1, `0x09` 2 + 3 · (zweites Parameterbyte), `0x0B` 0, `0x0C` 2, `0x0D` 2, `0x24` 1, `0x26` 4, `0x2B` 0, `0x40` 0, `0x44` 1, `0x46` 4, `0x47` 2, `0x66` 5, `0x80` 0.

- [ ] **Step 1: Scheiternde Tests:**
  - `gpu.rs`: `every_opcode_of_the_table_round_trips` (je Opcode ein Befehl mit `param_count` Wörtern, `decode(encode(x)) == x` nach Auffüllen auf Vierergruppen mit `NOP`), `groups_pad_with_nop` (drei Befehle → Opcode-Wort mit viertem Byte 0, keine Parameter dafür), `unknown_opcode_is_out_of_range` (`0x42`), `constructors_pack_like_gbatek` (`vtx_10([-1, 2, 511]).params == [0x3FF | 2 << 10 | 511 << 20]`; `vtx_16([4096, -4096, 1]).params == [0xF000_1000, 1]`; `normal([0, 511, 0]).params == [511 << 10]`; `texcoord([16, -16]).params == [0xFFF0_0010]`; `color(0x7FFF).params == [0x7FFF]`), `counts_follow_the_primitive` (Quads 8 Ecken → 2 Quads; QuadStrip 6 → 2; Triangles 6 → 2; TriangleStrip 5 → 3 Dreiecke; `verts` zählt `VTX_*`), `random_bytes_never_panic_the_decoders` (proptest, 0 bis 512 Byte, `decode` gibt `Ok` oder `Err`).
  - `sbc.rs`: `the_bakers_sbc_round_trips` (Bytes `26 00 00 00 00 02 00 01 0B 04 00 05 00 2B 01`), `variable_length_0x09` (`09 00 02 a b c d e f` liest 8 Parameterbytes), `sbc_without_0x01_is_truncated`, `unknown_sbc_opcode_is_out_of_range`, proptest wie oben.
- [ ] **Step 2: Lauf** `cargo test -p g4-formats nsbmd::` → FAIL.
- [ ] **Step 3: Umsetzen.**
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(formats): GPU display list and SBC codecs for MDL0` (mit Trailer).

### Task 3: MDL0-Codec, `nsbmd_header` als Hülle, gated Round-Trip der Modelle, `nsbmd.md`

**Files:**
- Create: `crates/g4-formats/src/nsbmd/model.rs` (+ Tests), `crates/g4-formats/tests/vanilla_nitro.rs` (gated), `docs/formats/nsbmd.md`
- Modify: `crates/g4-formats/src/nsbmd/mod.rs` (`Nsbmd`, `dict_spans`), `crates/g4-formats/src/nsbmd_header.rs` (privates `dict_at` → `nitro_dict::read` + `name_text`, Modul-Doku „Hülle, Struktur in `nsbmd`“), `docs/formats/nsbmd_header.md` (Quellenzeile: apicula `src/nitro/{container,info_block}.rs` (0BSD), GBATEK, pret `land_data.c`, eigene Messung; PDSMS gestrichen; Verweis auf `nsbmd.md`)

**Interfaces:**
- Consumes: Task 1 (`Name`, `Dict`, `read`, `name_text`), Task 2 (`GpuCmd`, `SbcOp`, `decode`, `encode`, `count`, `Geometry`).
- Produces:

```rust
// nsbmd/mod.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nsbmd { pub version: u16, pub models: Vec<(Name, Model)> }
impl Nsbmd {
    pub fn read(bytes: &[u8]) -> Result<Nsbmd, FormatError>;
    pub fn write(&self) -> Result<Vec<u8>, FormatError>;
}
/// Every dictionary of the file (model dictionary, then per model bones, materials, texture
/// pairings, palette pairings, shapes) as a byte range, for the gated dictionary check.
pub fn dict_spans(bytes: &[u8]) -> Result<Vec<std::ops::Range<usize>>, FormatError>;
// nsbmd/model.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model { pub head: ModelHead, pub bones: Vec<Bone>, pub sbc: Vec<SbcOp>, pub materials: Vec<Material>,
                   pub textures: Vec<Pairing>, pub palettes: Vec<Pairing>, pub shapes: Vec<Shape>, pub evp: Vec<u8> }
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModelHead { pub sbc_type: u8, pub scaling_rule: u8, pub texmtx_mode: u8, pub num_objects: u8,
    pub num_materials: u8, pub num_shapes: u8, pub first_unused_mtx: u8, pub pad: u8, pub up_scale: i32,
    pub down_scale: i32, pub num_verts: u16, pub num_polys: u16, pub num_tris: u16, pub num_quads: u16,
    pub box_pos: [i16; 3], pub box_size: [i16; 3], pub box_scale: i32, pub box_scale_inv: i32 }   // 0x14..0x40
#[derive(Debug, Clone, PartialEq, Eq)] pub struct Bone { pub name: Name, pub flags: u16, pub rest: Vec<u8> }
/// 4 + 12 (bit 0 clear) + 4 (bit 3 set) or 16 (bit 3 and bit 1 clear) + 24 (bit 2 clear: scale and inverse).
pub fn bone_len(flags: u16) -> usize;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material { pub name: Name, pub tag: u16, pub dif_amb: u32, pub spe_emi: u32, pub poly_attr: u32,
    pub poly_attr_mask: u32, pub teximage: u32, pub teximage_mask: u32, pub pltt_base: u16, pub flag: u16,
    pub width: u16, pub height: u16, pub mag: [i32; 2], pub extra: Vec<u8> }   // size field = 44 + extra.len()
#[derive(Debug, Clone, PartialEq, Eq)] pub struct Pairing { pub name: Name, pub materials: Vec<u8>, pub bound: u8 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape { pub name: Name, pub tag: u16, pub size: u16, pub flag: u32, pub commands: Vec<GpuCmd> }
impl Model {
    pub fn geometry(&self) -> Geometry;           // over every shape
    pub fn counters_match(&self) -> bool;         // head counts == geometry
}
```

- [ ] **Step 1: Scheiternde Tests** in `model.rs` mit `fn fixture() -> Nsbmd` aus Code (zwei Knochen: `0xF807` und `0x0002` mit 38 Byte `rest`; SBC wie Task 2 mit zwei Materialien; Material 0 mit 44, Material 1 mit 52 Byte; Textur-Paarungen `t0 → [0]`, `t1 → [1]`, Paletten-Paarung `p0 → [0, 1]`, `bound` 1; zwei Shapes mit `Quads`, `QuadStrip`, `TEXCOORD`, `NORMAL`, `VTX_10`, `VTX_16`, `VTX_DIFF`, Zähler passend):
  - `bone_lengths_follow_the_flags`: `bone_len(0xF807) == 4`, `(0x0000) == 56`, `(0x0008) == 44`, `(0x0006) == 16`, `(0x0002) == 40`.
  - `a_synthetic_model_round_trips_byte_for_byte`: `Nsbmd::read(&x.write()?)? == x` und zweites `write` gleich.
  - `the_layout_is_the_vanilla_order`: `MDL0` an 0x14, Größenfelder gleich den Längen, `ofsSbc` = 0x40 + Knochen-Wörterbuch + Knochen, `ofsMat` auf 4 ausgerichtet direkt nach der SBC, `ofsShp` direkt nach dem letzten Material, `ofsEvp` direkt nach der letzten Display List.
  - `nsbmd_header_reads_the_names_of_a_written_model`: `material_texture_names == ["t0", "t1"]`, `material_palette_names == ["p0"]`, `model_count == 1`; die Tests von `nsbmd_header.rs` bleiben grün.
  - `geometry_counts_every_shape` und `counters_match`.
  - `material_sizes_other_than_44_48_52_are_refused` (56), `a_tex0_block_is_unsupported` (`bmd0` mit `MDL0` und `TEX0`), `every_truncation_is_an_error_not_a_panic` (jede Länge `0..len`), `random_byte_flips_never_panic` (proptest: Byte an beliebiger Stelle verändert).
  - `dict_spans_cover_six_dictionaries` (ein Modell → 6 Bereiche, jeder `nitro_dict::read`-bar, `write` gleich dem Ausschnitt).
  - gated `tests/vanilla_nitro.rs` (`G4_TEST_PROJECT`, nur lesend; `family_of` und `vanilla()` wie `tests/vanilla.rs`; Spielcode aus `header.yaml`): `vanilla_map_models_round_trip_byte_identical` — je `unpacked/maps/NNNN` `MapFile::read` → `Nsbmd::read(&model)` → `write() == model`; jede Spanne aus `dict_spans` neu geschrieben gleich; Erwartung je Spielcode: CPUD 666 Modelle, IPKD und IPKE 676, alle gleich, Wörterbücher `6 × Modelle` gleich, `!counters_match` bei CPUD 0, IPKD 3 (Karten 0468, 0475, 0476), IPKE 4 (dazu 0473); druckt `nitro-models <code> models=<n> equal=<n> dicts=<n> stale=<liste> ms=<n>`; bei Abweichung Karte, Länge und erstes verschiedenes Byte (Offset, nicht den Wert).
- [ ] **Step 2: Lauf** `cargo test -p g4-formats --all-features nsbmd::` → FAIL.
- [ ] **Step 3: Umsetzen** nach dem Layout unter „Entscheidungen“; `Nsbmd::write` setzt `num_*` und Rahmen nie selbst.
- [ ] **Step 4: Lauf** → PASS. Gated im Hintergrund auf frischen Kopien (`6-1-pt`, `6-1-hgde`, `6-1-hgus`), je Spiel: `G4_TEST_PROJECT="$G4_ROMS_DIR/_testkopien/6-1-pt_DSPRE_contents" cargo test --release -p g4-formats --all-features --test vanilla_nitro -- vanilla_map_models --nocapture --test-threads 1 > target/agent/6-1/t3-pt.log 2>&1`; erwartet `models=666 equal=666 dicts=3996` (HG 676/676/4056). Weicht Vanilla von einer Leser-Regel ab, wird die Regel ein Rohfeld, nie ein Überspringen.
- [ ] **Step 5: Doku** `nsbmd.md`: Container, Wörterbuch (Layout, Baum mit Algorithmus und den Beispielen aus Step 1), MDL0, Modellkopf (Tabelle 0x00 bis 0x3F), Knochen (`bone_len`, apicula liest Skalierung mit 12 statt 24 Byte), SBC (Tabelle), Material (Felder, Bitfelder `POLYGON_ATTR`/`TEXIMAGE_PARAM` nach GBATEK, Messwerte A §2.3), Paarungen, Shapes, GPU-Befehle (Tabelle), Reihenfolge der Abschnitte, „Was der Codec als Bytes hält“, Verifikation (die Tests und Zahlen); Quellen apicula `src/nitro/{model,render_cmds,info_block}.rs`, `src/nds/gpu_cmds.rs` (0BSD), GBATEK, pret `land_data.c:1935`, Gutachten A; der Abschnitt „Teilmenge des Bäckers“ folgt mit Task 6. `nsbmd_header.md` wie unter Files.
- [ ] **Step 6: Commit** `feat(formats): MDL0 codec with byte-identical round trip of every vanilla map model` (mit Trailer).

### Task 4: NSBTX schreiben, anhängen, gated Round-Trip der Pakete, „Schreiben“ in `nsbtx.md`

**Files:**
- Create: `crates/g4-formats/src/nsbtx/write.rs` (+ Tests)
- Modify: `crates/g4-formats/src/nsbtx.rs` (`mod write; pub use write::{…};`, Modul-Doku), `crates/g4-formats/tests/vanilla_nitro.rs` (zwei Tests), `docs/formats/nsbtx.md`

**Interfaces:**
- Consumes: Task 1; `Nsbtx` (Rücklesen in Tests).
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Btx0 { pub version: u16, pub textures: Vec<TextureEntry>, pub palettes: Vec<PaletteEntry>,
    pub texels: Vec<u8>, pub compressed: Vec<u8>, pub compressed_index: Vec<u8>, pub palette_data: Vec<u8>,
    pub vram_keys: [u32; 3], pub tex_unknown: u32, pub compressed_unknown: u32,
    pub compressed_info_at_textures: bool, pub palette_flag: u16 }
#[derive(Debug, Clone, PartialEq, Eq)] pub struct TextureEntry { pub name: Name, pub params: u32, pub extra: u32 }
#[derive(Debug, Clone, PartialEq, Eq)] pub struct PaletteEntry { pub name: Name, pub offset8: u16, pub extra: u16 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTexture { pub name: String, pub palette: String, pub width: u32, pub height: u32,
    pub format: u8, pub color0: bool, pub texels: Vec<u8>, pub colors: Vec<u16> }   // format 2 or 3
pub const TEXEL_BANK: usize = 262_144;
pub const PALETTE_BANK: usize = 65_536;
impl Btx0 {
    pub fn read(bytes: &[u8]) -> Result<Btx0, FormatError>;   // one block TEX0 at 0x14, else Unsupported
    pub fn vram_bytes(&self) -> usize;                         // texels + compressed + compressed_index + palette_data
    /// Appends one texture and its palette (aligned to 8); refuses a side that is no power of two
    /// 8..=1024, another format, wrong texel or colour count, a taken name, entry 256, an offset past 0xFFFF·8.
    pub fn push(&mut self, t: &NewTexture) -> Result<(), FormatError>;
}
pub fn write(pack: &Btx0) -> Result<Vec<u8>, FormatError>;
pub fn append(pack: &[u8], new: &[NewTexture]) -> Result<Vec<u8>, FormatError>;
pub fn build(new: &[NewTexture]) -> Result<Vec<u8>, FormatError>;   // version 1
pub fn dict_spans(bytes: &[u8]) -> Result<Vec<std::ops::Range<usize>>, FormatError>;   // textures, palettes
```

- [ ] **Step 1: Scheiternde Tests** in `write.rs` (Texturen aus Code: 8 × 8 Format 3 mit Indizes `i % 16`, 16 × 8 Format 2, 32 × 32 Format 3 mit `color0`):
  - `a_built_pack_reads_back_with_the_reader`: `Nsbtx::read(&build(&ts)?)` → Namen, Größen, Formate, `color0`; `decode(i, palette i)` gleich der Farbe `rgb(colors[idx])` bzw. Alpha 0 für Index 0 mit `color0`.
  - `fixed_fields_are_the_vanilla_ones`: `extra == w | h << 11 | 1 << 31`; Palettendatum `extra` 1 genau bei Format 2; `palette_flag == 0x8000` mit 4-Farben-Textur, sonst 0; VRAM-Schlüssel 0; Wort `0x1E` = Texturen-Info-Offset; `0x24 == 0x28 ==` Palettendaten-Offset; Version 1; Texel- und Palettenoffsets durch 8 teilbar.
  - `write_of_read_is_byte_identical_for_a_built_pack`.
  - `append_keeps_old_entries_byte_identical`: Paket aus zwei Texturen, eine dritte angehängt → alte `TextureEntry`/`PaletteEntry` gleich, `texels`/`palette_data` beginnen mit den alten Bytes, neue `offset8` = alte Blocklänge / 8, `palette_flag` alt ODER neu.
  - `push_refuses_what_the_hardware_cannot_hold`: Seite 12 und 2048, Format 5, 17 Texel-Bytes zu wenig, doppelter Textur- und doppelter Palettenname, 256. Eintrag, Name mit 17 Byte → je Fehler, Paket unverändert.
  - `truncated_and_flipped_packs_are_errors` (jede Kürzung, proptest Bytefehler: keine Panik).
  - gated in `tests/vanilla_nitro.rs`: `vanilla_texture_packs_round_trip_byte_identical` — alle Member von `unpacked/mapTextures` und `unpacked/buildingTextures`, die mit `BTX0` beginnen: `write(&Btx0::read(b)?)? == b`, beide Wörterbücher aus `dict_spans` gleich; Erwartung CPUD 74 + 71, IPKD/IPKE 106 + 94 (die 10 übrigen HG-Member zählen nicht); danach die Summe der Wörterbücher mit Task 3: CPUD 4 286, IPKD und IPKE je 4 456 (zusammen 13 198); druckt `nitro-packs <code> map=<n> building=<n> equal=<n> dicts_total=<n>`. `vanilla_append_keeps_every_old_entry` — je Kartenpaket zwei synthetische `NewTexture` (8 × 8 Format 3, 8 × 8 Format 2, Namen `g4_t_a`, `g4_t_b`) anhängen: alte Einträge, alte Texel- und Palettenblöcke byte-gleich, `Nsbtx::decode` der ersten alten Textur mit ihrer gleichnamigen bzw. ersten Palette vorher gleich nachher, die neuen dekodieren wie gebaut.
- [ ] **Step 2: Lauf** `cargo test -p g4-formats --all-features nsbtx::` → FAIL.
- [ ] **Step 3: Umsetzen** (Kopf-Felder wie unter „Entscheidungen“ und `write_nsbtx_canonical`).
- [ ] **Step 4: Lauf** → PASS; gated im Hintergrund je Spiel wie Task 3 Step 4 mit `-- vanilla_texture_packs vanilla_append`.
- [ ] **Step 5: Doku** `nsbtx.md`: Kopf ohne „Nur Lesen“, Quellen apicula `src/nitro/{container,tex,info_block,name}.rs`, `src/nds/decode_texture.rs` (0BSD), GBATEK, eigene Messung (DSPRE gestrichen); neuer Abschnitt „Schreiben“ (Layout des Writers, Tabelle der festen Felder mit Messwerten A §2.4, neue Texturen, Anhängen, Palettenname, Bänke 256 KiB / 64 KiB, Verifikation mit den Zahlen).
- [ ] **Step 6: Commit** `feat(formats): NSBTX writer, append and build in the vanilla layout` (mit Trailer).

### Task 5: `texconv`: RGBA → 16 oder 4 Farben

**Files:**
- Create: `crates/g4-formats/src/texconv.rs` (+ Tests)
- Modify: `crates/g4-formats/src/lib.rs` (`pub mod texconv;` in der Gruppe)

**Interfaces:**
- Consumes: `nsbtx::{Rgba, NewTexture, rgb}`, `nitro_dict::name`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Colors { Four, Sixteen }
#[derive(Debug, Clone, PartialEq)] pub struct Quantized { pub texture: NewTexture, pub indices: Vec<u8>, pub mean_error: f32 }
pub const ALPHA_OPAQUE: u8 = 128;
/// Palette texture from an image: side a power of two 8..=1024, palette name `palette_name(name)`.
pub fn convert(name: &str, image: &Rgba, colors: Colors) -> Result<Quantized, FormatError>;
pub fn palette_name(texture: &str) -> String;
pub fn to_bgr555(c: [u8; 3]) -> u16;                     // r >> 3 | g >> 3 << 5 | b >> 3 << 10
pub fn median_cut(pixels: &[[u8; 3]], k: usize) -> Vec<u16>;
pub fn nearest(palette: &[u16], c: [u8; 3]) -> usize;
```

- [ ] **Step 1: Scheiternde Tests:**
  - `few_colours_are_kept_exactly`: 16 × 16 mit drei Farben, `Sixteen` → die drei `to_bgr555` in der Palette, jedes Pixel trifft seine Farbe, `color0 == false`, `format == 3`.
  - `transparent_pixels_get_index_0_and_color0`: Alpha 127 → Index 0, Alpha 128 deckend; `color0`; kein deckendes Pixel auf Index 0; `palette[0] == 0`.
  - `all_transparent_is_one_colour0_texture`: alle Indizes 0, Palette 16 × 0.
  - `sixteen_colours_pack_low_nibble_first` (`texels[0] == i0 | i1 << 4`), `four_colours_pack_lowest_bits_first` (`texels[0] == i0 | i1 << 2 | i2 << 4 | i3 << 6`, `format == 2`, Palette 4 Farben).
  - `median_cut_never_exceeds_k_and_is_deterministic` (proptest: `1 ≤ len ≤ k` für nicht leere Eingabe, zweiter Lauf gleich).
  - `sides_must_be_powers_of_two` (12 × 16, 4 × 4, 2048 × 8 → Fehler; 8 × 1024 gut).
  - `palette_name_appends_as_much_of_pl_as_fits` (`gras` → `gras_pl`; `abcdefghijklmn` → `abcdefghijklmn_p`; 16 Byte → unverändert).
  - `readback_through_nsbtx_is_exact`: Verlauf 64 × 64 → `build(&[q.texture])` → `Nsbtx::decode` gleich `rgb(palette[indices[i]])` an jedem Pixel (0 Abweichung).
- [ ] **Step 2: Lauf** `cargo test -p g4-formats texconv::` → FAIL.
- [ ] **Step 3: Umsetzen** nach „Quantisierung“ unter „Entscheidungen“ (Prototyp `gutachten-a-proto/src/main.rs` `median_cut`, `quantize`, `pack_texels`).
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(formats): texture converter with own median cut in BGR555` (mit Trailer).

### Task 6: Kernel `bake`: Flächenliste → Modell und Bericht

**Files:**
- Create: `crates/g4-kernel/src/bake/mod.rs` (Typen, `bake`, Tests), `crates/g4-kernel/src/bake/strips.rs`, `crates/g4-kernel/src/bake/encode.rs`, `crates/g4-kernel/src/bake/test_surface.rs` (erst `test_surface`)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod bake;` am Ende, eigene Gruppe), `docs/formats/nsbmd.md` (Abschnitt „Teilmenge des Bäckers“)

**Interfaces:**
- Consumes: Task 2, 3 (`Nsbmd`, `Model`, `GpuCmd`, `Geometry`), `checks::limits::limits()`, `g4_formats::{area_data::AreaData, game::Family}`.
- Produces (die Struktur `Face` gilt wörtlich, abgestimmt mit 6.3a):

```rust
// bake/mod.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Face { pub corners: [[i32; 3]; 4], pub uv: [[i32; 2]; 4], pub texture: String, pub layer: u8 }
// corners in world units (16 per tile, y up), counter-clockwise seen from above; uv in texels, world-bound;
// texture: name in the map texture pack (≤ 16 bytes); layer: the layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)] #[serde(rename_all = "lowercase")]
pub enum AreaStyle { Outdoor, Indoor }
impl AreaStyle { pub fn of(area: &AreaData) -> AreaStyle; }
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PackTexture { pub name: String, pub palette: String, pub width: u16, pub height: u16 }
pub struct BakeInput<'a> { pub style: AreaStyle, pub textures: &'a [PackTexture], pub vram_bytes: usize, pub vram_limit: usize }
pub const POS_SCALE: i32 = 64;
pub const VANILLA_VRAM_MAX_PT: usize = 110_640;
pub const VANILLA_VRAM_MAX_HGSS: usize = 87_424;
pub fn vanilla_vram_max(family: Family) -> usize;   // Dp, Pt → PT; Hgss → HGSS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BakeCode { Empty, ModelTooBig, TextureMissing, FaceOutOfRange, FaceUvRange, VramHigh,
                    TextureSide, TextureName, TextureTaken, PackFull }
impl BakeCode { pub fn as_str(self) -> &'static str; }   // "bake.empty" … "texture.pack-full"; serde as that string
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)] #[serde(rename_all = "lowercase")]
pub enum Level { Error, Warning }
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BakeMessage { pub code: BakeCode, pub level: Level, pub args: Vec<String>, pub message: String }
#[derive(Debug, Clone, serde::Serialize)]
pub struct BakeReport { #[serde(skip)] pub nsbmd: Vec<u8>, pub size: usize, pub share_of_limit: f32,
    pub textures: Vec<String>, pub vram_bytes: usize, pub verts: u32, pub polys: u32, pub strips: usize,
    pub single_quads: usize, pub messages: Vec<BakeMessage> }
impl BakeReport { pub fn refusal(&self) -> Option<&BakeMessage>; }   // first Error
/// Bakes `faces` (empty nsbmd when a face or texture error refuses before encoding; filled but
/// refused with model-too-big over 0xF000).
pub fn bake(faces: &[Face], input: &BakeInput) -> BakeReport;
// bake/strips.rs
pub(crate) struct Run { pub faces: Vec<usize>, pub strip: bool }
pub(crate) fn normal(face: &Face) -> [i32; 3];                     // 1/512
pub(crate) fn runs(faces: &[Face], of_texture: &[usize]) -> Vec<Run>;   // strips first, then singles
// bake/encode.rs
pub(crate) fn vertex(fx: [i32; 3]) -> GpuCmd;                       // VTX_10 on the raster, else VTX_16
pub(crate) fn model(faces: &[Face], input: &BakeInput) -> Result<(Model, usize, usize), BakeMessage>;
// bake/test_surface.rs
/// The test surface of Gutachten A §2.7: 32 × 32 floor at y 0, texture floor[((tx / 2) + (tz / 2)) % 4],
/// corners x0 = (tx − 16)·16, z0 = (tz − 16)·16, [(x0,0,z0), (x0,0,z0+16), (x0+16,0,z0+16), (x0+16,0,z0)],
/// uv = (x + 256, z + 256); a 2 × 2 × 1 block at `block` (top at y 16, uv like the floor; four sides as
/// in `wall_quads`, s = x + 256 or z + 256 along the wall, t = 16 − y), texture `block_texture`.
pub fn test_surface(floor: [&str; 4], block_texture: &str, block: (u8, u8)) -> Vec<Face>;
```

- [ ] **Step 1: Scheiternde Tests** in `bake/mod.rs` (`PackTexture` aus Code: `t0`..`t3`, `blk`, 16 × 16, Paletten `<name>_pl`):
  - `the_test_surface_bakes_under_40000_bytes`: `test_surface(["t0","t1","t2","t3"], "blk", (15, 15))`, beide Stile → `size ≤ 40_000`, `share_of_limit == size / 61440`, keine Meldung.
  - `the_own_reader_reads_the_baked_model`: `Nsbmd::read` gut, `write` gleich den Bytes, `counters_match`, `num_verts 3100`, `num_quads 1032`, `num_polys 1032`, `num_tris 0`, 5 Materialien, 5 Shapes, ein Knochen `0xF807`, SBC wie unter „Entscheidungen“, `up_scale 262_144`, `down_scale 64`, `report.strips == 514`, `single_quads == 4`.
  - `decoded_corners_equal_the_input`: Display Lists im Test zurückgelesen (`VTX_10` · 1 = Welt, `TEXCOORD` / 16 + Verschiebung) → Menge (Ecke, UV) gleich der Menge aller Flächenecken.
  - `strips_follow_runs_of_texture_and_layer`: Reihe von vier Kacheln `a` → ein Strip mit 10 Ecken; Ebenen `0,0,1,1` → zwei Strips; Texturen `a,a,b,b` → zwei Shapes; UV je Kachel neu bei 0 → vier Einzel-Quads; Wand mit anderer Normale an der Kante → kein gemeinsamer Strip.
  - `vtx_10_on_the_raster_and_vtx_16_off_it`: `vertex([64, 128, -64]).op == VTX_10`, `vertex([65, 0, 0]).op == VTX_16`, `vertex([-32_768, 0, 32_704]).op == VTX_10` (−512 und 511, die Ränder), `vertex([32_767, 0, 0]).op == VTX_16`.
  - `texcoord_only_on_change`: Zahl der `TEXCOORD` im Strip einer Reihe = Zahl der Ecken mit neuem (s, t) seit dem letzten gesendeten.
  - `long_strips_split_before_the_texcoord_range` (Review Focus): Reihe aus 32 Kacheln, Textur 64 × 64, `uv = (tx · 64, …)` → mindestens zwei Strips, jedes `TEXCOORD`-Feld in −32 768..=32 767, Ecken- und UV-Menge modulo 64 Texel gleich der Eingabe.
  - `a_face_far_out_in_uv_is_shifted_by_whole_textures`: eine Fläche mit s 5 000..5 016, Breite 64 → gesendete s 8..24 (Verschiebung 4 992).
  - `a_face_wider_than_the_texcoord_range_is_refused`: s 0..4 096 → `bake.face-uv-range`, `nsbmd` leer.
  - `outdoor_has_light_fog_and_normals_indoor_white_colour`: außen `poly_attr 0x001F8081`, `dif_amb 0x7FFFE739`, Shape-Flag `0x5`, Boden `NORMAL [0, 511, 0]`, Südwand des Blocks `[0, 0, 511]`; innen `0x001F0080`, `0x7FFF8000`, `0x6`, erster Befehl jedes Shapes `COLOR 0x7FFF`, kein `NORMAL`.
  - `a_shared_palette_is_one_pairing_with_both_materials`: `t0`, `t1` mit Palette `pal` → eine Paletten-Paarung `pal → [0, 1]`.
  - `area_style_follows_the_light_rule`: `Dppt` Lichttyp 0, 3 → `Outdoor`, 1, 2 → `Indoor`; `Hgss` 1 → `Outdoor`, 0, 2 → `Indoor`.
  - `an_oversize_model_is_refused_with_model_too_big`: zwei volle Böden (y 0 und 32), UV je Kachel neu bei 0 → `refusal().code == ModelTooBig`, `size > 61_440`, `args[0]` die Größe.
  - `missing_texture_empty_list_and_out_of_range_corner_are_refused`: `bake.texture-missing` (Name nicht in `textures`), `bake.empty`, `bake.face-out-of-range` (Ecke x 512).
  - `vram_over_the_vanilla_maximum_warns`: `vram_bytes 110_641`, `vram_limit 110_640` → Warnung `bake.vram-high`, `refusal() == None`.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- bake::` → FAIL.
- [ ] **Step 3: Umsetzen** nach „Bäcker: …“ unter „Entscheidungen“.
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Doku** `nsbmd.md` „Teilmenge des Bäckers“ (Werte, Strips, `VTX_10`-Regel, `TEXCOORD`, Stil, Rahmen, Prüfcodes, Größen aus A §2.5 und die gemessene Größe der Testfläche).
- [ ] **Step 6: Commit** `feat(bake): bake a neutral face list into a map model with strips, VTX_10 and area style` (mit Trailer).

### Task 7: Pakettexturen, „Textur an Paket anhängen“, VRAM, Modell-Import aus Bytes

**Files:**
- Create: `crates/g4-kernel/src/bake/textures.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/bake/mod.rs` (Re-Exporte), `crates/g4-kernel/src/map_import.rs` (die Schleife über Header → Matrix → Area-Data aus `map_packs` wird `pub fn map_areas`, `map_packs` ruft sie; angehängt `pub fn import_model_bytes`)

**Interfaces:**
- Consumes: Task 4 (`Btx0`, `append`, `NewTexture`, Bänke), Task 6 (Typen, `bake`); `Store`, `FileStep`, `FileChange`, `FileStepKind::Reset`, `write_file_step`, `Project::{member_path, ensure_unpacked}`.
- Produces:

```rust
// bake/textures.rs
/// Textures of a pack with their palette: the palette named like the texture, else `palette_name(name)`,
/// else the only palette of the pack; a texture without one is left out (bake reports it missing).
pub fn pack_textures(pack: &[u8]) -> Result<Vec<PackTexture>, FormatError>;
pub fn pack_vram(pack: &[u8]) -> Result<usize, FormatError>;
/// Map pack (`map_pack` when given, else the member on disk) plus building pack of area `area`.
pub fn area_vram(store: &mut Store, area: u8, map_pack: Option<&[u8]>) -> Result<usize, KernelError>;
pub fn areas_of_pack(store: &mut Store, pack: u16) -> Result<Vec<u8>, KernelError>;   // map_tileset == pack
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppendReport { pub pack: u16, pub appended: Vec<String>, pub size_before: usize, pub size_after: usize,
                          pub vram_bytes: usize, pub messages: Vec<BakeMessage> }
/// Appends `textures` to map texture pack `pack` as one file step labelled `label`; every check before
/// anything is written (errors: KernelError::Refused("<code>: <message>")); Diamond/Pearl refused.
pub fn append_textures(store: &mut Store, pack: u16, textures: &[NewTexture], label: &str) -> Result<AppendReport, KernelError>;
/// `bake` for map `map`: style, pack textures and VRAM of the first area that shows the map.
pub fn bake_for_map(store: &mut Store, map: u16, faces: &[Face]) -> Result<BakeReport, KernelError>;
// map_import.rs
pub fn map_areas(store: &mut Store, map: u16) -> Result<Vec<u8>, KernelError>;
/// Writes `model` as the model of map `map` (the checks of a model import first) as one map step `label`.
pub fn import_model_bytes(store: &mut Store, map: u16, model: Vec<u8>, label: &str) -> Result<Vec<AssetId>, KernelError>;
```

- [ ] **Step 1: Scheiternde Tests** in `textures.rs` (`synthetic_project`; nach `ensure_unpacked(MapTexture)` und `ensure_unpacked(BuildingTexture)` schreibt der Test `nsbtx::build` aus synthetischen `NewTexture` in `member_path(MapTexture, 0)` und `member_path(BuildingTexture, 0)`; Area-Data 0 zeigt Paket 0):
  - `append_writes_the_member_and_undo_restores_it`: zwei Texturen → Datei auf der Platte neu, `history().0.last() == "Kartentexturen 0000: 2 Texturen angehängt"`, `undo` → alte Bytes, `redo` → neue.
  - `append_refuses_side_name_taken_and_full`: Seite 24 → `texture.side`; Name 17 Byte → `texture.name`; Name im Paket und doppelt in der Anfrage → `texture.taken`; 256. Eintrag → `texture.pack-full`; jeweils Datei unverändert, kein Verlaufseintrag.
  - `palette_names_that_collide_after_cutting_are_taken` (Review Focus): `abcdefghijklmnX` und `abcdefghijklmnY` (15 Byte) → beide Palettennamen `…n_` → `texture.taken`.
  - `vram_is_judged_for_every_area_of_the_pack` (Review Focus): Area-Data 1 so umgeschrieben, dass auch sie Paket 0 nutzt, mit einem großen Gebäudepaket → Warnung `bake.vram-high` mit `args[0] == "1"` (das Gebiet), Gebiet 0 bleibt unter der Grenze; nichts abgelehnt.
  - `pack_textures_resolve_palettes_by_name_then_pl_then_single`.
  - `bake_for_map_takes_style_and_textures_of_the_area`: Karte 0 → `Outdoor` (Lichttyp 0), Texturen aus Paket 0, `vram_bytes == pack_vram + Gebäudepaket`.
  - in `map_import.rs` (Tests-Modul): `import_model_bytes_is_one_map_step_and_refuses_oversize` (gebackenes Modell → ein Schritt mit `label`, `undo` stellt das alte Modell her; 0xF004 Byte → Fehler mit `61440`, Karte unverändert); die vorhandenen Tests von `map_packs` bleiben grün.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- bake::textures map_import::` → FAIL.
- [ ] **Step 3: Umsetzen**; der Dateischritt nach `original::file_step`/`reset_files` (`ensure_unpacked`, `write_file_step`, `record_file_step`).
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(bake): append textures to a map texture pack as one undo step, VRAM per area` (mit Trailer).

### Task 8: Testtexturen, `bake_test` im Kernel, Orakel apicula, gated Tests

**Files:**
- Modify: `crates/g4-kernel/src/bake/test_surface.rs` (Bilder, `bake_test`), `crates/g4-kernel/src/bake/mod.rs` (Re-Exporte)
- Create: `crates/g4-kernel/tests/bake.rs`

**Interfaces:**
- Consumes: Task 5 (`convert`, `Colors`), Task 6 (`test_surface`, `bake`), Task 7 (`append_textures`, `pack_textures`, `bake_for_map`, `map_areas`, `import_model_bytes`); `export::nsbmd_to_glb`.
- Produces:

```rust
pub struct SampleImage { pub name: &'static str, pub image: Rgba, pub colors: Colors }
/// The four synthetic images of the prototype (`synth_images`): syn_gras 32 × 32, syn_zaun 16 × 16 with
/// transparent rows 0–3 (color0), syn_weg 8 × 8 four colours, syn_himmel 64 × 64 gradient.
pub fn sample_images() -> [SampleImage; 4];
/// g4_bake_a: 16 × 16, 16 colours, 4-pixel checker of (230, 120, 40) and (40, 90, 200) with a 1-pixel
/// border (20, 20, 20); g4_bake_b: 16 × 16, 4 colours, diagonal stripes ((x + y) / 4) % 4 of four greens.
pub fn bake_textures() -> [SampleImage; 2];
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct BakeTestOptions { pub map: u16, pub block: Option<(u8, u8)>, pub out_dir: Option<PathBuf> }
#[derive(Debug, Clone, serde::Serialize)]
pub struct BakeTestReport { pub map: u16, pub pack: u16, pub area: u8, pub style: AreaStyle,
    pub appended: Vec<String>, pub floor: [String; 4], pub block_texture: String, pub block: (u8, u8),
    pub report: BakeReport, pub model_file: Option<PathBuf>, pub pack_file: Option<PathBuf>, pub changed: Vec<AssetId> }
pub fn bake_test(store: &mut Store, opts: &BakeTestOptions) -> Result<BakeTestReport, KernelError>;
```

- [ ] **Step 1: Scheiternde Tests** in `tests/bake.rs`:
  - `sample_images_read_back_without_a_pixel_off` (hermetisch): `nsbtx::build` aus `sample_images()` über `convert` → `Nsbtx::read`, `decode(i, palette <name>_pl)` gleich der Quantisierung an jedem Pixel (0 falsch); druckt die `mean_error` je Bild (Prototyp: 7,6 / 3,2 / 1,9 / 12,7).
  - `bake_test_on_the_synthetic_project_is_one_undo_step` (hermetisch; Paket 0 wie in Task 7 vorbereitet, zwei 16 × 16-Texturen darin): `appended == ["g4_bake_a", "g4_bake_b"]`, `report.size ≤ 40_000`, `history().0.last() == "Testfläche gebacken: Karte 0000"`, ein `undo` stellt Kartenbytes und Paketdatei her; mit `out_dir` liegen `bake-0000.nsbmd` (gleich `report.nsbmd`) und `maptex-0000.nsbtx` (gleich dem Member) dort.
  - `bake_test_twice_reuses_the_own_textures` (Review Focus): zweiter Lauf → `appended == []`, Paketgröße unverändert, Modell neu geschrieben.
  - `apicula_reads_the_test_surface_like_the_input` (`G4_APICULA`, sonst `skipped`): Testfläche mit 16 × 16-Texturen → `nsbmd_to_glb(apicula, &nsbmd, None, dir)`; GLB im Test zerlegt (JSON-Chunk, BIN-Chunk, Accessor `POSITION` und `TEXCOORD_0`, `componentType` 5126): Menge (gerundete Position, (u · 16, v · 16) in 1/16 Texel) gleich der Eingabemenge; Exit 0 und genau eine GLB prüft `nsbmd_to_glb` selbst.
  - `apicula_decodes_the_sample_pack_like_g4_formats` (`G4_APICULA`): Paket aus `sample_images()` in eine Datei, `apicula convert --more-textures <pack> -o <dir>`; je Textur `<dir>/**/<name>.png` (mit `png` gelesen) gegen `Nsbtx::decode`: 0 Pixel verschieden (bei Alpha 0 nur Alpha verglichen).
  - `vanilla_bake_test_in_the_room` (gated, `vanilla_copy()`): Karte nach Spielcode (CPUD 187, IPKD/IPKE 217) → `pack` 20 bzw. 25, `size ≤ 40_000`, `appended` zwei Namen, die alten Einträge des Pakets byte-gleich (`Btx0::read` vorher/nachher), der Nachbarraum (186 / 216) hat dasselbe Paket (`map_areas` → `map_tileset`), keine Fehlermeldung; `undo` → Karte und Paket wie vorher; druckt `bake-room <code> map=<n> pack=<n> style=<s> size=<n> share=<p> vram=<n> warn=<codes> ms=<n>`.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features --test bake` → FAIL.
- [ ] **Step 3: Umsetzen** nach „`bake_test`“ unter „Entscheidungen“; die Bilder nach `synth_images` und den Formeln oben.
- [ ] **Step 4: Lauf** → PASS hermetisch; mit `G4_APICULA` (absolut) die zwei Orakel-Tests; gated im Hintergrund je Spiel: `G4_TEST_PROJECT=… G4_APICULA=… cargo test --release --workspace --all-features --test bake -- --nocapture --test-threads 1 > target/agent/6-1/t8-<spiel>.log 2>&1`.
- [ ] **Step 5: Commit** `feat(bake): test surface and bake_test with apicula oracle and gated room bake` (mit Trailer).

### Task 9: Befehl `bake_test`, `g4k check-data map_texture`, `docs/mcp.md`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/bake.rs` (+ Tests)
- Modify: `apps/studio/src-tauri/src/commands/mod.rs` (`mod bake; pub use bake::*;` am Ende), `commands/dispatch.rs` (`"bake_test"` am Ende von `COMMANDS`, Zweig mit `policy.write_path` für `outDir`, Fall in `every_command_accepts_the_keys_api_ts_sends` vor `close_project`: `("bake_test", json!({ "map": 0 }))`), `apps/studio/src-tauri/src/lib.rs` (Import und `generate_handler!` am Ende), `mcp/access.rs` (`("bake_test", Kind::Write)` am Ende von `COMMAND_KINDS`, Test), `crates/g4-kernel/src/bin/g4k.rs` (`DataKind::MapTexture`, Name `map_texture`, Feld `sha1`, Doku von `CheckData`), `crates/g4-kernel/tests/cli.rs` (Test), `docs/mcp.md` („Expertenmodus“: ein Absatz zu `bake_test`)

**Interfaces:**
- Consumes: Task 8 (`bake_test`, `BakeTestOptions`, `BakeTestReport`).
- Produces:

```rust
// commands/bake.rs
pub fn bake_test_impl(state: &AppState, map: u16, block: Option<[u8; 2]>, out_dir: Option<PathBuf>) -> Result<BakeTestReport, CmdError>;
#[tauri::command]
pub fn bake_test(state: State<AppState>, map: u16, block: Option<[u8; 2]>, out_dir: Option<String>) -> Result<BakeTestReport, CmdError>;
// dispatch.rs: "bake_test" => run!(json, "bake_test", args, { map: u16, block: Option<[u8; 2]>, out_dir: Option<String> }, …)
// g4k: --expect map_texture:<member>:sha1=<40 hex>  (SHA-1 of the whole member of the map texture archive)
```

- [ ] **Step 1: Scheiternde Tests:**
  - `commands/bake.rs`: `bake_test_over_the_command_path` (`open_synthetic`, Paket 0 vorbereitet wie Task 7, `bake_test_impl(&state, 0, None, Some(<fixture>/build/bake))` → `appended` zwei, Dateien da, `history` mit „Testfläche gebacken: Karte 0000“), `bake_test_refuses_an_unreadable_pack` (Platzhalter-Member → Fehler, nichts geändert).
  - `dispatch.rs`: der neue Fall in `every_command_accepts_the_keys_api_ts_sends`; `commands_match_the_tauri_handler` grün; `bake_test_out_dir_outside_the_roots_is_refused` (`outDir` außerhalb → „Pfad liegt außerhalb“).
  - `mcp/access.rs`: `bake_test_is_a_write` (`kind_of("bake_test") == Some(Kind::Write)`).
  - `tests/cli.rs`: `check_data_reads_the_sha1_of_a_map_texture` (`synthetic_rom()`, Member 0 des Kartentextur-Archivs, `--expect map_texture:0:sha1=<sha1 der Member-Bytes>` → Exit 0, falscher Wert → Exit 1 mit „map_texture 0 sha1“); `parse_expectation("map_texture:0:size=1")` nennt die Felder.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- bake_test check_data_reads_the_sha1 commands_match every_command_accepts` → FAIL.
- [ ] **Step 3: Umsetzen**; `docs/mcp.md`: „`bake_test {map, block?, outDir?}` (schreibend, ein Undo-Schritt, nur über `g4_call`): backt die Testfläche aus Gutachten A §2.7 in Karte `map` und hängt `g4_bake_a`/`g4_bake_b` an ihr Kartentextur-Paket an; Antwort mit `pack`, `appended`, `report` (Größe, Anteil an 0xF000, Texturen, VRAM, Meldungen) und den Dateien unter `outDir` (Standard keine; nur unter `build/`). Bis 6.4a das Werkzeug des Gates 6.1.“
- [ ] **Step 4: Lauf** → PASS; `cargo test -p studio --test mcp_stdio` grün; clippy, fmt.
- [ ] **Step 5: Commit** `feat(studio): bake_test command, check-data map_texture sha1` (mit Trailer).

### Task 10: g4emu-Szenarien `pt-bake`, `hg-bake`, `hgus-bake`

**Files:**
- Create: `scripts/agent/scenarios/pt-bake.yaml`, `scripts/agent/scenarios/hg-bake.yaml`, `scripts/agent/scenarios/hgus-bake.yaml`

**Interfaces:**
- Consumes: Task 9 (`bake_test` über `g4_call`, `check-data map_texture`).

Szenario `pt-bake.yaml` (Kopfkommentar nach dem Muster von `pt-flag.yaml`: Zweck Gate 6.1 Punkt 5, Zimmer Karte 187 (Header 415), Nachbarraum Karte 186 (Header 414), beide Area-Data und Paket 20; Befehl mit absoluten Pfaden aus einem Worktree; Bilder `zimmer.png`, `unten.png` nicht ins Repo):

```yaml
game: CPUD
mcp: [--allow-irreversible, --expert]
edits:
  - [g4_call, {command: bake_test, args: {map: 187, outDir: build/bake}}, bake]
  - [save, {}]
build: build/out.nds
check:
  - [g4k, check-data, build/out.nds,
     --expect, "map:187:model_sha1=${sha1-of:${bake.model_file}}",
     --expect, "map_texture:${bake.pack}:sha1=${sha1-of:${bake.pack_file}}"]
play:
  - continue
  - expect-map 415
  - wait 60
  - shot zimmer
  - walk-to 8 4
  - walk right 1
  - expect-map 414
  - wait 150
  - shot unten
```

`hg-bake.yaml`: `game: IPKD`, Karte 217, `expect-map 64`, `shot zimmer`, `walk left 1`, `walk down 1`, `expect-map 63`, `wait 150`, `shot unten`; `hgus-bake.yaml` gleich mit `game: IPKE` (Kommentar: Zuordnung wie DE angenommen, in A nicht im Spiel geprüft).

- [ ] **Step 1: Kopien** `6-1-szen-pt`, `6-1-szen-hg`, `6-1-szen-hgus` (je Ordner und `.nds` daneben); `cargo build --workspace --all-features`; `G4_STUDIO_BIN`, `G4K_BIN` auf `target/debug/studio` und `target/debug/g4k` des Worktrees.
- [ ] **Step 2: Lauf** im Hintergrund je Spiel aus `scripts/g4emu`: `uv run g4emu scenario ../agent/scenarios/pt-bake.yaml --project "$G4_ROMS_DIR/_testkopien/6-1-szen-pt_DSPRE_contents" --out "$W/target/agent/6-1/pt-bake" > "$W/target/agent/6-1/t10-pt.log" 2>&1`; erwartet Rückgabe 0, `"ok": true`, beide `check-data`-Erwartungen erfüllt, `expect-map` 415 → 414 (HG 64 → 63).
- [ ] **Step 3: Blick des Agenten** auf `zimmer.png` (eigene Texturen: orange-blaues Schachbrett und grüne Streifen, Block mit Oberseite und zwei sichtbaren Seiten) und `unten.png` (Vanilla-Texturen des Erdgeschosses unverändert); liegt der Block außer Sicht, `block` explizit setzen und den Wert im Kopfkommentar festhalten. Farben werden nicht verglichen (g4emu tauscht Rot und Blau bis zum Fix in 6.2, A §8).
- [ ] **Step 4: Commit** `test(g4emu): scenarios pt-bake, hg-bake and hgus-bake for the 6.1 gate` (mit Trailer).

### Task 11: Gate 6.1, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-6-1-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 6.1“), `docs/ROADMAP.md` (Abschnitt „Phase 6“: Stand der Zeile 6.1 unter der Tabelle; Übersicht Zeile 6)

**Interfaces:**
- Consumes: alles Vorige; die Druckzeilen `nitro-models`, `nitro-packs`, `bake-room` der gated Tests und die Szenarien.

- [ ] **Step 1: Gate-Dokument** nach dem Muster von `docs/phase-3d-1-gate.md`: Kopf mit Stand und Branch; der Gate-Text der Spezifikation §4 Zeile 6.1 als Zitat; Legende „Wer“ (**Agent**, **Agent + Blick**, **Hand**) mit Verweis auf `agent-gate.md`; Vorbereitung (Kopien `6-1-{pt,hgde,hgus}` für (1), `6-1-szen-{pt,hg,hgus}` für (3), Werkzeuge, `tools`-Symlink, `G4_ROMS_DIR` und `G4_APICULA` absolut, Ausgaben unter `target/agent/6-1/`, Last mit `uptime`).
  - **(1) Automatik (Agent):** `cargo deny --locked check licenses advisories` grün ohne neue Abhängigkeit; je Spiel im Release-Profil `--test vanilla_nitro` (Modelle 666 / 676 / 676 byte-gleich, Pakete 145 / 200 / 200, Wörterbücher 4 286 / 4 456 / 4 456, zusammen 13 198, Anhängen lässt alte Einträge byte-gleich) und `--test bake` (Testfläche ≤ 40 000 Byte, eigener Leser, Zähler gleich, `bake.model-too-big`, apicula Exit 0 und eine GLB mit gleicher Ecken- und UV-Menge, NSBTX vier Bilder 0 Pixel Abweichung zu `g4-formats` und zu apicula, Zimmer gebacken); `scripts/gate.sh` grün (die neuen Tests laufen in „cargo test (no ROM data)“ und „gated tests (cargo test)“ mit).
  - **(2) Unit-Punkte des Gates** mit Testnamen: `the_test_surface_bakes_under_40000_bytes`, `the_own_reader_reads_the_baked_model`, `an_oversize_model_is_refused_with_model_too_big`, `apicula_reads_the_test_surface_like_the_input`, `sample_images_read_back_without_a_pixel_off`, `apicula_decodes_the_sample_pack_like_g4_formats`, `vanilla_append_keeps_every_old_entry`.
  - **(3) Spiel (Agent + Blick):** `pt-bake`, `hg-bake`, `hgus-bake` (Befehle aus Task 10), Bilder `zimmer.png`, `unten.png` je Spiel, Rückgabe 0, `check-data` erfüllt.
  - **(4) Beim Nutzer:** Blick auf die sechs Bilder (eigene Texturen sichtbar mit Block, Nachbarraum mit Vanilla-Texturen); `docs/formats/nsbmd.md` gelesen.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Modelle gleich | Pakete gleich | Wörterbücher | veraltete Zähler | Testfläche (Byte, Anteil) | Zimmer (Byte, VRAM, Meldungen) | Szenario | Last |
    |---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | | |
    | HeartGold DE | | | | | | | | |
    | HeartGold US | | | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(6.1): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: `agent-gate.md`** Abschnitt „Phase 6.1“ nach dem Muster „Phase 4.4“: Kopien, die Tabelle Punkt / Wer / Wie, die Befehle für (1) und (3) mit absoluten Pfaden aus dem Worktree. **Roadmap**: unter der Tabelle von „Phase 6“ „**Stand 6.1 <Datum>:** Gate vom Agenten bestanden: Round-Trip 666 / 676 / 676 Modelle, 145 / 200 / 200 Pakete, 13 198 Wörterbücher byte-gleich, Testfläche <…> Byte (<…> %), `pt-bake`, `hg-bake`, `hgus-bake` mit eigenen Texturen; Branch `phase-6-1-writer`; [docs/phase-6-1-gate.md](phase-6-1-gate.md); beim Nutzer offen: Blick auf die Bilder“; Übersicht Zeile 6 um „6.1 Gate vom Agenten bestanden <Datum>“.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`.
- [ ] **Step 5: Commit** `docs(6.1): gate of the writer, agent-gate section, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **Leser-Regeln gegen Vanilla:** Lehnt der strenge Wörterbuch- oder Modell-Leser ein Vanilla-Modell ab (etwa ein Wörterbuch mit 0 Einträgen, ein Shape-Kopf mit `size ≠ 16`, ein unbekannter SBC-Befehl in einem der 33 Platin-Modelle mit mehreren Knochen), wird das Feld ein Rohwert der Struktur und der Leser nimmt es an; der gated Test bleibt die Prüfung. Byte-Gleichheit hat Vorrang vor einer schöneren Struktur.
- **apicula und ein Paket ohne Modell:** schreibt `apicula convert --more-textures <pack>` ohne NSBMD keine PNG, bekommt der Aufruf die gebackene Testfläche mit den vier Texturnamen als erstes Argument; die Vergleichsregel bleibt.
- **Textprojekt:** dass der Abgleich nach `append_textures` `data/map_textures/NNNN.nsbtx` schreibt, prüft ein Test mit `TextFixture::exported()` nach dem Muster der Reset-Tests von 4.1b; schreibt er sie nicht, ist das ein Befund für 6.3b (dort braucht der Build-Schritt `world` die Pakete), nicht ein Umbau in 6.1.
- **Block im Bild:** die Rahmenmitte des alten Zimmermodells kann außerhalb der Kamera liegen; dann `block` im Szenario setzen (Wert im Kopfkommentar). Der Spieler läuft durch den Block (Kollision und BDHC bleiben unverändert): das ist gewollt.
- **HeartGold US im Spiel:** in Gutachten A nicht geprüft; weicht Karte oder Paket ab (`bake.pack` ≠ 25), die Werte aus der Kopie lesen und im Kopf von `hgus-bake.yaml` festhalten.
- **Debug-Dauer in `scripts/gate.sh`:** die gated Round-Trips laufen dort im Debug-Profil; dauern sie über 60 s je Projekt, bleibt es so (kein eigener Schritt), die Zeit steht in der Gate-Datei.
- **`quantette`:** nur, wenn `sample_images_read_back_without_a_pixel_off` oder ein Blick zeigt, dass der Median-Cut nicht trägt; dann eigener Commit mit `cargo deny`-Schritt (Global Constraints).
- **Parallele Merges:** `crates/g4-formats/src/lib.rs` und `crates/g4-kernel/src/lib.rs` (6.2 `bdhc`, `terrain`; 6.3a `world`), `dispatch.rs`, `access.rs`, `src-tauri/src/lib.rs`, `commands/mod.rs` (6.2 `get_map_heights`), `g4k.rs` (`MAP_FIELDS`/`DataKind`), `docs/mcp.md`, `ROADMAP.md`, `agent-gate.md` werden Konflikte an Anhängepunkten; die Erfahrung aus Phase 4 und 3d gilt (gesammelt lösen, beide Seiten behalten).
