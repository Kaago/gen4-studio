# Phase 6.5a „PNG und Modelle“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Der Import-Dialog „Teile importieren…“ nimmt neben fertigen Binärteilen auch PNG (→ Texturen, angehängt an das Kartentextur-Paket des Gebiets über den Konverter aus 6.1) und OBJ/glTF (→ Modell-Teil der Karte über den Bäcker aus 6.1), prüft vorher mit Vorschau (Kennzahlen, Budget gegen 0xF000, neue und vorhandene Texturen, VRAM) und lehnt mit Grund ab; Import und Anhängen sind **ein** Undo-Schritt.

**Architecture:** Neues Kernel-Modul `model_import/` liest OBJ (`tobj`) und glTF (`gltf`, ohne Standard-Features) in ein neutrales Netz `Mesh`, macht daraus die Flächenliste `bake::Face` (Skalieren, Runden auf Welteinheiten, Ohrenschneiden von n-Ecken, Dreieckspaare zu Vierecken) und plant die Texturen (PNG/JPEG → `texconv::convert`, Name aus dem Dateistamm, vorhandene Namen im Paket wiederverwendet). `map_import` bekommt den Teil `Textures`, erkennt `.obj`/`.gltf`/`.glb` als Modell und `.png` als Texturen, backt beim Prüfen ohne zu schreiben und schreibt beim Import Paket und Karte als einen Schritt (wie `bake_test`). Der Bäcker lernt Dreiecke (eine Fläche mit `c3 == c2`), sonst bleibt 6.1 unverändert. Das Studio reicht `options {unit, colors}` durch und zeigt die Vorschau im vorhandenen Dialog; MCP nur über `g4_call` (die Spezifikation nennt für 6.5a kein kuratiertes Werkzeug).

**Tech Stack:** Rust (`g4-kernel`: neu `tobj` 4.0.5, `gltf` 1.4.1 ohne Standard-Features mit `utils` und `names`, `base64` 0.22 (schon im Lockfile); vorhanden `image` 0.25 mit `png`/`jpeg`, `png` 0.18, `serde_json`, `proptest`), `g4-formats` (`texconv`, `nsbtx`, `nsbmd`) unverändert, Tauri/React (`ImportPartsDialog.tsx`, `importParts.ts`, `api.import.ts`, Vitest), apicula `3d4e91e` als Orakel, Python 3 (`scripts/g4emu`).

**Spec:** [2026-11-05-phase-6-kartenerstellung-design.md](../specs/2026-11-05-phase-6-kartenerstellung-design.md) (§1 harte Grenzen, §2 „Werkzeuge und Lizenzen“, §3.3, §3.7, §4 Zeile 6.5a, §5, §6 „Import-Dialog mit Vorschau und Ablehnungsgrund“, §7, §8); [entscheidungen.md](../../research/2026-11-05-phase-6/entscheidungen.md) (Zeile 6.5a, Tabelle „Malen, Weltformat, Import, Tiled“ Zeile „Import“, „Nicht in Phase 6“); [Gutachten C](../../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §6 (Import, Lizenzen, PDSMS-Weg), §8.3; [Gutachten A](../../research/2026-11-05-phase-6/gutachten-a-writer.md) §2.4, §2.5, §2.8, §2.9; Vorlage für Form und Gate: [Plan 6.1](2026-11-05-phase-6-1-writer.md).

Stand 06.11.2026: `main` bei `81411881` (6.1 und 6.2 gemergt). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-5a`, Branch `phase-6-5a-import` (legt der Umsetzer an: `git worktree add .claude/worktrees/phase-6-5a -b phase-6-5a-import 81411881`). Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, App `apps/studio/src`, Emulator `scripts/g4emu`.

## Voraussetzungen (auf main `81411881`, vor Task 1 mit `grep` prüfen)

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `g4_kernel::bake::{Face, bake, BakeInput, BakeReport, BakeCode, BakeMessage, Level, PackTexture, AreaStyle, bake_context, BakeContext, pack_textures, area_vram, areas_of_pack, append_textures, appended_pack (pub(crate))}` | wie in `bake/mod.rs`, `bake/textures.rs` | Task 1, 6, 7 |
| `bake/strips.rs` `Run { faces, strip: bool }`, `runs`, `normal`; `bake/encode.rs` `List::{strip, quads}`, `MAX_TEXTURES` 255 | | Task 1 |
| `g4_formats::texconv::{convert, Colors, Quantized, palette_name, to_bgr555}`, `nsbtx::{Nsbtx, NewTexture, Rgba, Btx0}`, `gpu::Prim::Triangles` | | Task 1, 6, 8 |
| `g4_kernel::map_import::{Part, Source, PartCode (ALL: 30), PartMessage, PartState, PartReport, inspect, import, import_model_bytes, map_areas}`, privat `prepare`, `check`, `mark_duplicates`, `texture_note`, `Payload`, `PartMessage::new` | | Task 7 |
| `g4_kernel::terrain::budget::{BudgetItem, BudgetState, budget_state}` | `budget_state(used, max)` | Task 7 |
| `Store::{undo_depth, squash_since, rollback_to, history, edit_map, busy}` | wie `bake_test` sie nutzt | Task 7 |
| `crate::relpath::ser_opt` | Pfade relativ zum Projekt | Task 6, 7 |
| `crates/g4-kernel/tests/bake.rs` `glb_chunks`, `accessor`, `input_set`, `glb_set`, `prepare`; `test_support::{synthetic_project, vanilla_copy}` | | Task 7, 8 |
| `crates/g4-kernel/src/bin/g4k.rs` (Unterbefehle mit `clap`) | | Task 3 |
| Studio: `commands/map_import.rs` (`inspect_map_import_impl`, `import_map_parts_impl`, Test `every_part_code_has_its_german_sentence`), `dispatch.rs` (`confine_source`, Fälle `inspect_map_import`/`import_map_parts`, `every_command_accepts_the_keys_api_ts_sends`), `mcp/access.rs` (beide Befehle schon `Read`/`Write`) | | Task 9 |
| App: `api.import.ts` (`MapPart`, `PartReport`, `inspectMapImport`, `importMapParts`), `importParts.ts` (`PART_NAME`, `PART_TEXTS`, `partText` mit `num`/`family`), `ImportPartsDialog.tsx` (`openFiles({ title: "Teile wählen" })`), `ExportPartDialog.tsx`, `ui/primitives/Budget.tsx` | | Task 9 |
| `scripts/g4emu/g4emu/scenario.py` `_PREPARE_STEPS`, `_prepare` (nur `make-oversize-nsbmd`), `mcp:`-Szenarien mit `g4_call`; `scripts/agent/scenarios/pt-bake.yaml` (Header 415 → 414, `walk-to 8 4`, `walk right 1`), `hg-bake.yaml` (64 → 63) | | Task 10 |
| Policy: relative Pfade in `read_path`/`write_path` gelten relativ zum Projektordner (`commands/policy.rs`) | | Task 10, 11 |

## Parallele Teilphasen

**6.3a Weltformat** (Worktree `phase-6-3a`, läuft) legt `g4-kernel/src/world/*`, `textproject/*`, `store/*`, `apps/studio/src/world/scene/*` an und bildet `MeshFace` per `From` auf `bake::Face` ab. **6.3b Backen und Build** startet nach 6.3a parallel zu 6.5a und ruft `bake` aus dem Build-Schritt `world`. **6.5a baut keine dieser Dateien um.** `bake::Face` bleibt Feld für Feld gleich (Task 1 fügt nur `is_triangle` hinzu und legt eine Konvention fest, die jede Vierecks-Fläche unberührt lässt); `BakeReport` bekommt ein Feld `triangles` (nur angehängt). Anhängepunkte (nur anhängen): `crates/g4-kernel/src/lib.rs`, `crates/g4-kernel/Cargo.toml`, `Cargo.lock`, `bin/g4k.rs`, `map_import.rs` (`PartCode::ALL` hinten), `dispatch.rs`, `api.import.ts`, `importParts.ts`, `docs/mcp.md`, `docs/ROADMAP.md`, `docs/agent-gate.md`, `design-system/README.md`. `scripts/gate.sh` bleibt unverändert (die neuen Tests laufen in den vorhandenen Schritten mit).

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Abhängigkeiten | `tobj = { version = "4.0.5", default-features = false }` (ohne `ahash`); `gltf = { version = "1.4.1", default-features = false, features = ["utils", "names"] }` (ohne `import`, also ohne `image`- und `base64`-Kopie und ohne Dateizugriff des Crates); `base64 = "0.22"` direkt im Kernel (0.22.1 steht schon im Lockfile über `studio`). Neu im Lockfile damit genau: `tobj` 4.0.5 (MIT), `gltf` 1.4.1, `gltf-json` 1.4.1, `gltf-derive` 1.4.1 (je MIT OR Apache-2.0), `inflections` 1.1.1 (MIT), `lazy_static` 1.5.1 (MIT OR Apache-2.0); `byteorder`, `serde`, `serde_json` sind schon da. Geprüft am 06.11.2026 mit `cargo info` und `cargo deny check licenses advisories` an einer Probe mit genau diesen Zeilen: Lizenzen ok, keine Meldung | Spezifikation §3.7 nennt `tobj` und `gltf`; alles in der Positivliste von `deny.toml`; der eigene Leser als Rückfall ist nicht nötig |
| Wo der Import lebt | Kernel `model_import/{mod,mesh,read_obj,read_gltf,textures,samples}.rs`; `map_import` ruft es aus `prepare`; kein eigener Befehl, sondern die vorhandenen `inspect_map_import`/`import_map_parts` mit `options` | Spezifikation §3.7 „über `map_import` mit Vorschau“; der Dialog „Teile importieren…“ ist schon da (3b.4b) |
| Dateiarten | Endung entscheidet (wie heute): `.obj` → Modell (OBJ), `.gltf`/`.glb` → Modell (glTF), `.png` → Teil **„Texturen“** (`Part::Textures`); alle PNG eines Aufrufs bilden **einen** Teil (eine Zeile, `preview.files`), zwei Modell-Dateien sind wie heute `part.duplicate` | eine Zeile je Teil im Dialog; mehrere Kacheln in einem Schritt |
| Einheit und Achsen | `options.unit`: Welteinheiten je Modelleinheit, Vorgabe **16** (1 Einheit = eine Kachel, wie PDSMS exportiert; glTF: 1 m = eine Kachel), erlaubt 0,0625 bis 256; Y oben, X Osten, Z Süden, Ursprung = Kartenmitte (wie `test_surface`); keine Achsen-Umrechnung | Gutachten C §6.1 „Einheit wählbar (Vorgabe: 1 = eine Kachel)“; OBJ und glTF sind rechtshändig mit Y oben wie die Welt des Bäckers |
| Genauigkeit | Ecken auf **ganze Welteinheiten** gerundet (halbe weg von 0), UV auf **ganze Texel**; eine Fläche ohne Fläche nach dem Runden fällt weg (Hinweis `model.faces-dropped`); Ecken außerhalb −512..=511 → `model.out-of-range` mit Koordinate | `bake::Face` (mit 6.3a abgestimmt) trägt `i32`-Welteinheiten und Texel; 1 Welteinheit = 1/16 Kachel = ein Texel einer Bodenkachel; `posScale` bleibt 64 (`POS_SCALE`) |
| Polygone | Dreieck bleibt Dreieck; Viereck bleibt Viereck, wenn es eben (Abstand von `c3` zur Ebene `c0 c1 c2` ≤ 0,5 E) und konvex ist, sonst Ohrenschneiden; n-Ecke ab 5 per Ohrenschneiden in der Ebene der Newell-Normale (eigener Code, kein `earcutr`), Rückfall Fächer bei einem Selbstschnitt; danach **Dreieckspaare zu Vierecken**: gleiches Material, gemeinsame Kante mit gleicher UV an beiden Ecken, eben (wie oben), konvex; gierig in Eingabereihenfolge; Viereck aus A (gedreht, sodass die Kante `A1 A2` ist) und B = `[A0, A1, B_gegenüber, A2]` | DS kennt Drei- und Vierecke, Vanilla ist zu 77–81 % Viereck; glTF hat nur Dreiecke (eine Ebene = zwei Dreiecke = ein Viereck) |
| Dreiecke im Bäcker | eine `Face` mit `corners[3] == corners[2]` **und** `uv[3] == uv[2]` ist ein Dreieck (`Face::is_triangle`); `strips::runs` nimmt Dreiecke in keinen Strip, `encode` schickt sie in einer Gruppe `BEGIN_VTXS Triangles` je Shape nach Strips und Quads; `BakeReport.triangles` zählt sie | spart gegenüber einem entarteten Viereck eine Ecke und ist sauber; Vierecks-Eingaben (6.1, 6.3a, 6.3b) ändern sich nicht |
| Wickelung | Reihenfolge der Ecken bleibt (OBJ und glTF: gegen den Uhrzeigersinn = Vorderseite, Normale `(v1 − v0) × (v2 − v0)` nach außen, gleich der Regel des Bäckers); bei einer Knoten-Matrix mit negativer Determinante wird die Reihenfolge umgedreht | Gutachten A §2.7; glTF-Spezifikation |
| UV | im `Mesh` als Bruchteil der Textur mit t nach unten: OBJ `t = 1 − v`, glTF `t = v`; in Texeln `s · Breite`, `t · Höhe` der gebundenen Textur; Wiederholen S/T steht im Material (Bäcker), UV außerhalb 0..1 ist erlaubt | DS-Textur beginnt oben links |
| Materialien und Texturen | je Material: hat es ein Bild (OBJ `map_Kd`, glTF `baseColorTexture`) → Textur aus dem Bild, Name `texture_name(Stamm)`; steht der Name schon im Paket → die Paket-Textur wird benutzt (Hinweis `texture.reused`, nichts angehängt); zwei verschiedene Bilder mit gleichem Namen in einem Aufruf → `_2`, `_3` … (auf 16 Byte gekürzt); ohne Bild → die Paket-Textur mit dem Namen des Materials, sonst `model.material-untextured`; ein Polygon ohne Material ebenso (Name leer) | Wiederholter Import hängt nicht doppelt an; OBJ aus PDSMS oder Blender kann Vanilla-Texturen beim Namen nennen; Ersetzen gibt es nicht (Phase 6) |
| PNG als Teil „Texturen“ | Name = `texture_name(Dateistamm)`; steht er im Paket → `texture.taken` (kein stilles Wiederverwenden, der Nutzer will hier eine neue Textur) | „Texturen ersetzen oder löschen“ ist nicht in Phase 6 |
| Bilder | gelesen mit `image::load_from_memory` (PNG, JPEG) → RGBA8; Seiten müssen Zweierpotenzen 8 bis 1 024 sein (`texture.side`), **kein Skalieren**; Farben `options.colors` 16 (Vorgabe) oder 4 für alle Texturen des Aufrufs; Konverter `texconv::convert` unverändert (Median-Cut BGR555, `color0` bei Alpha < 128, ohne Dithering, eine Palette je Textur) | Spezifikation §3.7; Skalieren wäre eine stille Änderung des Bildes |
| Name | `texture_name(stem)`: jedes Byte außerhalb `0x21..=0x7E` wird `_`, auf 16 Byte gekürzt, leer → `g4_tex`; Palettenname `palette_name(name)` wie 6.1 | `nitro_dict::name`-Regel; apicula rät `<name>_pl` |
| Dateien neben dem Modell | MTL, Bilder und glTF-Puffer nur relativ und **innerhalb des Ordners des Modells** (auch Unterordner); absolute Pfade, `..` heraus und URIs mit Schema außer `data:` → `model.file-outside`; fehlend → `model.file-missing`; glTF-`data:`-URIs (base64) sind erlaubt | die Policy des Studios prüft nur den gewählten Pfad; der Kernel darf nicht über ein Modell an fremde Dateien |
| glTF | `.glb` (BIN-Chunk) und `.gltf` mit externen Puffern/Bildern oder `data:`-URIs; Szene: `default_scene`, sonst Szene 0, ohne Szene alle Wurzelknoten; Knoten-Matrizen werden angewandt, ein Mesh an zwei Knoten kommt zweimal; Primitive `Triangles`, `TriangleStrip`, `TriangleFan` (zu Dreiecken entfaltet), `Points`/`Lines*` → Hinweis `model.parts-ignored` (`lines`, `points`); Animationen, Skins, Kameras, Lichter, Eckenfarben → derselbe Hinweis; `extensionsRequired` nicht leer → `model.source-unreadable` | Blender exportiert alle drei glTF-Formen; Draco u. a. kann der Leser nicht |
| OBJ | `tobj::load_obj_buf` mit `single_index: true`, `triangulate: false`, `ignore_points`, `ignore_lines`; MTL über die Rückruffunktion (relativ zur OBJ, eingegrenzt); `map_Kd` relativ zur MTL; negative Indizes übernimmt `tobj` | Vierecke müssen als Vierecke ankommen (`face_arities`) |
| Prüfen ohne Schreiben | `inspect` rechnet alles bis zum Bäcker: Kontext der Karte (`bake_context`), Texturplan, Paket nach dem Anhängen mit `appended_pack` (nur im Speicher), `pack_textures`/`area_vram` des neuen Pakets, `to_faces`, `bake`; geschrieben wird nichts | „Vorschau vor dem Import wie heute bei `map_import::inspect`“ (Gutachten C §6.1) |
| Import als ein Schritt | `import` sammelt die neuen Texturen aller gewählten Teile (Modell und Texturen, ein Paket), hängt sie mit `append_textures` an, schreibt dann die Karte (`edit_map`) und fasst mit `squash_since` zu **einem** Schritt mit dem vorhandenen Label `map NNNN: <teile> imported` zusammen (`model`, `textures`); scheitert der Kartenschritt, `rollback_to(mark)` (Muster `bake_test`); zurück kommen `Map NNNN` und `MapTexture <pack>` | ⌘Z nimmt Modell und Paket zusammen zurück |
| Vorschau | `PartReport.preview` (nur bei umgewandelten Teilen): Paket, Gebiet, Stil, VRAM des Gebiets und Vanilla-Maximum, Texturen (Name, Datei, Größe, Farben, neu oder vorhanden, mittlerer Fehler), beim Modell Format, Polygone, Vierecke, Dreiecke, Strips, Einzel-Vierecke und `budget: BudgetItem` (Größe gegen 0xF000, Zustand nach `budget_state`); im Dialog als Zeilentext und Baustein `Budget`; **keine 3D-Vorschau im Dialog** (nach dem Import zeigt die Szene das Modell über apicula wie bei jedem Modell, ⌘Z ist der Rückweg) | README 3b.4b: „Dialog ohne eigene Vorschau (die Szene ist die Vorschau, ⌘Z der Rückweg)“; Spezifikation §6 „Vorschau und Ablehnungsgrund“ als Kennzahlen und Budget |
| Prüfcodes | 21 neue `PartCode` hinten an `ALL` mit deutschem Satz (Abschnitt „Prüfcodes und Sätze“); `bake`-Meldungen werden mit `from_bake` abgebildet; `texture.*` gelten für den Modell- und den Texturen-Teil | der Dialog wortet Codes, ein Rust-Test hält `PART_TEXTS` gleich `ALL` |
| Hinweis „Texturnamen fehlen“ | `texture_note` läuft nicht für umgewandelte Modelle (ihre Namen stehen im Paket nach dem Anhängen) | sonst meldete die Prüfung die eigenen neuen Namen als fehlend |
| Beispiele | `model_import::samples::write_samples(dir)` erzeugt alle Testdateien aus Code (Würfel-OBJ, Ebene als `.glb` und `.gltf`, OBJ im PDSMS-Stil, zu großes OBJ, PNG mit 40 Farben); `g4k import-samples <dir>` schreibt sie für Szenarien und Browser; nichts davon im Repo | „im Test erzeugt, keine fremden Assets“ (Gutachten C §8.3); keine ROM-Daten |
| PDSMS | kein `.pdsmap`/`.pdsts`; „PDSMS: Export as OBJ with Textures → Teile importieren“ als Abschnitt in `docs/formats/model_import.md`; das Beispiel im PDSMS-Stil folgt der öffentlichen Beschreibung (Vierecke, `mtllib`/`usemtl`, `map_Kd` mit PNG, `vn`, `s off`, eine Kachel = eine Einheit), nicht PDSMS-Code | Entscheidungen „PDSMS nur über PDSMS' eigenen OBJ-Export“; PDSMS hat keine Lizenz |
| MCP | kein neues Werkzeug; `inspect_map_import`/`import_map_parts` mit `options` über `g4_call` (`--expert`); Absatz in `docs/mcp.md` | Spezifikation nennt für 6.5a kein kuratiertes Werkzeug |
| Nicht in 6.5a | Freiform in Weltkarten, Teilmodelle im Tileset, TMJ, „übernehmen“ als glTF (6.5b); Bilder skalieren; A3I5/A5I3/256 Farben; gemeinsame Paletten; Eckenfarben und Licht aus dem Modell; `VTX_XZ`/`VTX_DIFF`; BDHC aus dem Modell (die BDHC der Karte bleibt, Hinweis `model.bdhc-unchanged` wie heute); Kampfsprites | Schnitt der Spezifikation §4, „Nicht in Phase 6“ |

## Global Constraints

- Lizenz: gen4-studio ist MIT. Die Writer von DSPRE (NsbmdWriter, NsbtxWriter, TileBake, BdhcBuild) sind AGPL-3.0: kein Code übernehmen, auch nicht abschreiben. PDSMS hat keine Lizenz: ebenfalls kein Code. Erlaubt sind Formatwissen aus pret/pokeplatinum, eigene Messungen und Dokumentation.
- Keine Abhängigkeit vom NITRO-SDK (g3dcvtr.exe).
- Byte-Treue: Unveränderte Karten bauen weiterhin byte-gleich. Die Original-Assets bleiben unberührt, solange der Nutzer nicht importiert.
- Keine ROM-Daten im Repo (scripts/check-no-rom-data.sh vor jedem Commit; Testbilder und Testmodelle im Repo müssen selbst erzeugt sein, keine Spielgrafik).
- Jede neue Abhängigkeit (tobj, gltf und ihre transitiven Crates) mit Lizenz-Audit über cargo deny --locked; nur MIT/Apache/BSD/Zlib-artig. Geprüft: `tobj` 4.0.5 MIT; `gltf`, `gltf-json`, `gltf-derive` 1.4.1 MIT OR Apache-2.0; `inflections` 1.1.1 MIT; `lazy_static` 1.5.1 MIT OR Apache-2.0; `base64` 0.22.1 schon im Lockfile. Taucht nach `cargo update -p` ein anderes neues Crate auf, wird es vor dem Commit geprüft und im Commit-Text genannt.
- Arbeitsort: Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-5a`, Branch `phase-6-5a-import` (Basis main `81411881`). Nie `cd` in die Hauptkopie, nie Branch wechseln, **kein Merge, kein Push**, kein `git stash`.
- `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`. Löst `-p g4-kernel` den Download in rotoms Build-Skript aus, über den Workspace mit Namensfilter testen (`cargo test --workspace --all-features -- <filter>`).
- `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut. `tools/` im Worktree als Symlink auf `$HOME/Projekte/lab/editor/gen4-studio/tools` (nie einchecken); `G4_APICULA=$HOME/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula`. Originale unter `Roms/` nie öffnen oder ändern; Testkopien nur `Roms/_testkopien/6-5a-<zweck>_DSPRE_contents` (`cp -cR`) mit `6-5a-<zweck>.nds` daneben (`cp -c`); gated Tests schreiben nur auf `vanilla_copy()`.
- Test-Umgebung auf eigenen Ports: `G4_VITE_PORT=1500 G4_BRIDGE_PORT=1501 bash scripts/agent/studio-web.sh`.
- Kein `scripts/gate.sh` im Worktree (es läuft nach dem Merge auf main in der Hauptsitzung).
- Jeder Commit endet mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`).
- Lange Läufe (gated Tests, `cargo test --workspace`, Szenarien) im Hintergrund mit Log: `mkdir -p target/agent/6-5a && <befehl> > target/agent/6-5a/<task>-<name>.log 2>&1; echo "exit $?" >> target/agent/6-5a/<task>-<name>.log`; `uptime` davor. Ein Zeittest, der unter Last reißt, wird allein wiederholt, die Grenze nie gelockert.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates, `bash scripts/check-no-rom-data.sh`; bei App-Dateien `(cd apps/studio && pnpm test && pnpm build)`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`; bei `scripts/g4emu` `cd scripts/g4emu && uv run pytest -m "not integration" -q`.
- Kein `unwrap`/`expect`/Panik in Produktcode der Leser: jede Fremddatei kann kaputt sein, Fehler als `PartMessage` mit Code.

## Review Focus

- Dateien, die aus dem Ordner des Modells zeigen (`mtllib ../x.mtl`, `map_Kd /etc/x.png`, glTF-URI `../a.bin`, `https://…`, ein Symlink nach außen): abgelehnt mit `model.file-outside`, nichts außerhalb gelesen (Task 4 `references_outside_the_folder_are_refused`, Task 5 `gltf_uris_outside_the_folder_or_with_a_scheme_are_refused`).
- Maßstab am Rand: `unit` so groß, dass Ecken über ±512 E gehen, oder so klein, dass Flächen beim Runden verschwinden: Ablehnung mit Koordinate bzw. Hinweis mit Zahl, ganz leer → `model.no-geometry` (Task 2 `corners_outside_the_map_range_are_refused`, `faces_that_collapse_when_rounded_are_dropped_with_a_note`, `a_model_that_collapses_entirely_has_no_geometry`).
- Texturnamen: zwei Bilder mit gleichem Stamm, ein Stamm mit Leer- oder Umlautzeichen oder über 16 Byte, ein Stamm gleich einer Vanilla-Textur, ein PNG-Name schon im Paket, eine Seite keine Zweierpotenz: nummeriert, bereinigt, wiederverwendet mit Hinweis bzw. abgelehnt mit Code (Task 6 `two_images_with_one_stem_get_numbered_names`, `texture_name_keeps_printable_ascii_and_cuts_to_16`, `a_pack_texture_of_the_same_name_is_reused_for_a_model_but_taken_for_png`, `sides_not_power_of_two_are_texture_side`).
- glTF-Szenengraph: Knoten mit Verschiebung und negativer Skalierung, ein Mesh an zwei Knoten, Strip- und Fächer-Primitive, Linien: richtig platziert, Vorderseite bleibt außen, zweimal importiert, entfaltet bzw. mit Hinweis weggelassen (Task 5 `node_transforms_are_applied_and_a_mirror_flips_the_winding`, `a_mesh_at_two_nodes_is_imported_twice`, `strips_and_fans_are_unfolded`, `lines_and_points_are_ignored_with_a_note`).
- Geteiltes Paket und Atomarität: Prüfen schreibt nichts; Import von Modell und Texturen ist ein Schritt, ⌘Z stellt Karte und Paket her, andere Karten bleiben byte-gleich, ein zweiter Import hängt nicht doppelt an (Task 7 `inspect_converts_a_cube_obj_without_writing`, `import_writes_model_and_pack_as_one_undo_step`, `unchanged_maps_stay_byte_identical`, `reimporting_the_cube_reuses_its_texture`).

## Prüfcodes und Sätze (gilt für Task 2 bis 9)

21 neue `map_import::PartCode`, hinten an `ALL` (dann `[PartCode; 51]`) in der Reihenfolge der Tabelle; `model.too-big` (vorhanden) kommt beim umgewandelten Modell aus dem Bäcker mit `[größe, 61440]`. **Wer einen Code anlegt, trägt im selben Commit seinen Satz in `PART_TEXTS` (`apps/studio/src/workbench/inspector/importParts.ts`) ein**, sonst wird der Studio-Test `every_part_code_has_its_german_sentence` (läuft in `cargo test --workspace`) rot. Formate in `partText`: vorhanden `{n}`, `{n:num}`, `{n:family}`; neu `{n:dec}` (Punkt → Komma, mit Task 6) und `{n:ignored}` (Wörter `lines` Linien, `points` Punkte, `animations` Animationen, `skins` Skelette, `cameras` Kameras, `lights` Lichter, `vertex-colours` Eckenfarben, `texture-options` Texturoptionen, mit „, “ verbunden; mit Task 5), je mit einem Vitest-Fall in `importParts.test.ts`. Hinweise ändern den Zustand eines Teils nicht.

| Code (`PartCode`) | Art, Argumente | Task | Satz |
|---|---|---|---|
| `model.source-unreadable` (`ModelSourceUnreadable`) | Fehler `[„OBJ“ \| „glTF“, grund]` | 4 | Die Datei ist kein lesbares {0}-Modell ({1}). Exportiere das Modell neu als OBJ oder glTF. |
| `model.file-missing` (`ModelFileMissing`) | Fehler `[pfad wie geschrieben]` | 4 | Die Datei {0}, auf die das Modell verweist, fehlt. Lege sie neben das Modell oder exportiere es neu. |
| `model.file-outside` (`ModelFileOutside`) | Fehler `[pfad wie geschrieben]` | 4 | Das Modell verweist auf {0} außerhalb seines Ordners. Lege Material und Texturen in den Ordner des Modells. |
| `model.no-geometry` (`ModelNoGeometry`) | Fehler `[]` | 2 | Das Modell enthält keine Flächen. Wähle ein Modell mit Flächen oder eine größere Einheit. |
| `model.material-untextured` (`ModelMaterialUntextured`) | Fehler `[material]` (leer ohne Material) | 2 | Das Material „{0}“ hat keine Textur, und das Gebiets-Paket hat keine Textur dieses Namens. Gib dem Material eine PNG-Textur. |
| `model.uv-missing` (`ModelUvMissing`) | Fehler `[material]` | 2 | Flächen mit dem Material „{0}“ haben keine Texturkoordinaten. Exportiere das Modell mit UV. |
| `model.out-of-range` (`ModelOutOfRange`) | Fehler `[x, y, z, -512, 511]` (gerundete Welteinheiten der ersten Ecke außerhalb) | 2 | Eine Ecke liegt bei ({0}, {1}, {2}), außerhalb von {3} bis {4} Einheiten. Verkleinere das Modell oder wähle eine kleinere Einheit. |
| `model.uv-range` (`ModelUvRange`) | Fehler `[textur, achse, spanne]` | 7 | Die Textur {0} läuft auf einer Fläche über {2:num} Texel in {1}; das Spiel trägt höchstens 2.047. Teile die Fläche oder wiederhole die Textur seltener. |
| `model.texture-count` (`ModelTextureCount`) | Fehler `[n, 255]` | 7 | Das Modell nutzt {0:num} Texturen, ein Kartenmodell trägt höchstens {1:num}. Fasse Materialien zusammen. |
| `model.converted` (`ModelConverted`) | Hinweis `[format, polygone, vierecke, dreiecke, strips, einzel-vierecke]` | 7 | {0}-Modell umgewandelt: {1:num} Flächen, {2:num} Vierecke, {3:num} Dreiecke, {4:num} Streifen. |
| `model.faces-dropped` (`ModelFacesDropped`) | Hinweis `[anzahl]` | 2 | {0:num} Flächen fallen weg: nach dem Runden auf ganze Einheiten haben sie keine Fläche mehr. |
| `model.parts-ignored` (`ModelPartsIgnored`) | Hinweis `[liste]` (Tokens mit Komma, Reihenfolge wie oben, je einmal) | 5 | Nicht übernommen: {0:ignored}. |
| `part.no-area` (`PartNoArea`) | Fehler `[karte]` | 7 | Kein Header zeigt die Karte {0}; ohne Gebiet gibt es kein Texturpaket und kein Licht. Setze die Karte zuerst in eine Matrix mit Header. |
| `texture.side` (`TextureSide`) | Fehler `[name, breite, höhe]` | 6 | Die Textur {0} ist {1} × {2} Pixel groß; jede Seite muss eine Zweierpotenz von 8 bis 1.024 sein. Skaliere das Bild. |
| `texture.name` (`TextureName`) | Fehler `[name]` | 6 | „{0}“ taugt nicht als Texturname (höchstens 16 Zeichen, ASCII ohne Leerzeichen). Benenne die Datei um. |
| `texture.taken` (`TextureTaken`) | Fehler `[„texture“ \| „palette“, name]` | 6 | Der Name {1} ist im Gebiets-Paket schon vergeben. Benenne die Datei um; Ersetzen gibt es nicht. |
| `texture.pack-full` (`TexturePackFull`) | Fehler `[was, wert, höchstens]` | 6 | Das Gebiets-Paket ist voll ({0}: {1:num}, höchstens {2:num}). Nimm weniger oder kleinere Texturen. |
| `texture.unreadable` (`TextureUnreadable`) | Fehler `[datei, grund]` | 4 | Das Bild {0} lässt sich nicht lesen ({1}). Wähle ein PNG. |
| `texture.new` (`TextureNew`) | Hinweis `[name, breite, höhe, farben, mittlerer fehler mit einer Nachkommastelle („7.6“)]` | 6 | Neue Textur {0}: {1} × {2}, {3} Farben, mittlerer Farbfehler {4:dec}. |
| `texture.reused` (`TextureReused`) | Hinweis `[name]` | 6 | Die Textur {0} steht schon im Gebiets-Paket und wird benutzt, nicht neu angehängt. |
| `texture.vram-high` (`TextureVramHigh`) | Warnung `[gebiet, vram, höchstens]` | 6 | Gebiet {0}: die Texturen belegen {1:num} B VRAM, mehr als das größte Vanilla-Gebiet ({2:num} B). Prüfe die Karte im Spiel. |

Die Reihenfolge in `ALL` ist die der Tabelle, auch wenn die Codes in verschiedenen Tasks entstehen (ein später Code wird an seiner Tabellenstelle eingefügt, nicht ans Ende); `PART_TEXTS` folgt derselben Reihenfolge.

---

### Task 1: Bäcker: Dreiecke

**Files:**
- Modify: `crates/g4-kernel/src/bake/mod.rs` (`Face::is_triangle`, Doku von `Face`, `BakeReport.triangles`, Tests), `crates/g4-kernel/src/bake/strips.rs` (`RunKind`), `crates/g4-kernel/src/bake/encode.rs` (`List::triangles`), `docs/formats/nsbmd.md` (Abschnitt „Teilmenge des Bäckers“: Dreiecke)

**Interfaces:**
- Consumes: `gpu::{GpuCmd, Prim::Triangles}`, `Geometry`.
- Produces:

```rust
// bake/mod.rs
impl Face {
    /// A triangle: corner 3 repeats corner 2 (position and uv); sent as BEGIN_VTXS Triangles.
    pub fn is_triangle(&self) -> bool;
}
pub struct BakeReport { /* … wie 6.1 …, */ pub triangles: usize }   // faces sent as triangles
// bake/strips.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunKind { Strip, Quads, Triangles }
pub(crate) struct Run { pub faces: Vec<usize>, pub kind: RunKind }
pub(crate) fn runs(faces: &[Face], of_texture: &[usize]) -> Vec<Run>;   // strips, then quads, then triangles
// bake/encode.rs
impl List<'_> { fn triangles(&mut self, run: &[usize]) -> Result<usize, BakeMessage>; }  // corners 0, 1, 2
```

- [ ] **Step 1: Scheiternde Tests** in `bake/mod.rs`:
  - `a_triangle_face_is_sent_as_a_triangle`: eine Fläche `[(0,0,0), (0,0,16), (16,0,16), (16,0,16)]`, UV entsprechend mit `uv[3] == uv[2]` → `Nsbmd::read` gut, `counters_match`, `num_tris 1`, `num_quads 0`, `num_verts 3`, `report.triangles == 1`, `report.single_quads == 0`.
  - `triangles_never_join_a_strip`: zwei Dreiecke, deren Kanten `c3 c2`/`c0 c1` übereinstimmen, und eine Reihe aus drei Vierecken → ein Strip (3 Flächen), keine Dreiecke darin, eine `Triangles`-Gruppe mit 6 Ecken.
  - `a_quad_with_equal_corner_positions_but_other_uv_stays_a_quad`: `c3 == c2` in der Position, nicht in der UV → `is_triangle() == false`, `num_quads 1`.
  - `triangle_normal_points_out`: Dreieck wie oben außen → `NORMAL [0, 511, 0]`.
  - Die Tests von 6.1 (`the_own_reader_reads_the_baked_model` mit `num_tris 0` usw.) bleiben grün.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- bake::` → FAIL (`is_triangle` fehlt).
- [ ] **Step 3: Umsetzen**: `runs` sortiert Dreiecke vor der Kettenbildung aus (in Eingabereihenfolge), `encode::model` ruft je `RunKind` `strip`, `quads` oder `triangles`; `triangles` sendet `NORMAL` (außen) und `TEXCOORD` nach denselben Regeln wie `quads` (`shift` über die drei Ecken).
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Doku** `nsbmd.md` „Teilmenge des Bäckers“: ein Satz zu Dreiecken (`c3 == c2`, eigene Gruppe am Ende des Shapes, nie im Strip); die Entscheidung der Phase „der Bäcker braucht keine Dreiecke“ ist damit für Importe aufgehoben (Verweis auf diesen Plan).
- [ ] **Step 6: Commit** `feat(bake): triangles as faces with a repeated last corner` (mit Trailer).

### Task 2: Neutrales Netz und Flächen (`model_import::mesh`)

**Files:**
- Create: `crates/g4-kernel/src/model_import/mod.rs` (erst `pub mod mesh;`, `ImportOptions`, `SourceFormat`, `from_bake`), `crates/g4-kernel/src/model_import/mesh.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod model_import;` in eigener Gruppe am Ende), `crates/g4-kernel/src/map_import.rs` (`PartMessage::new` → `pub(crate)`; die Codes dieses Tasks aus „Prüfcodes und Sätze“: `ModelNoGeometry`, `ModelMaterialUntextured`, `ModelUvMissing`, `ModelOutOfRange`, `ModelFacesDropped`), `apps/studio/src/workbench/inspector/importParts.ts` (ihre Sätze)

**Interfaces:**
- Consumes: Task 1 (`Face`, `is_triangle`), `bake::strips::normal` (pub(crate)), `map_import::{PartCode, PartMessage}`, `bake::{BakeMessage, BakeCode}`, `texconv::Colors`.
- Produces:

```rust
// model_import/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ImportOptions { pub unit: f32, pub colors: u8 }   // Default: unit 16.0, colors 16
pub const UNIT_MIN: f32 = 0.0625;
pub const UNIT_MAX: f32 = 256.0;
impl ImportOptions {
    /// unit finite in UNIT_MIN..=UNIT_MAX, colors 4 or 16; else KernelError::Refused (English reason).
    pub fn check(&self) -> Result<(), KernelError>;
    pub fn texconv_colors(&self) -> Colors;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)] #[serde(rename_all = "lowercase")]
pub enum SourceFormat { Obj, Gltf }
impl SourceFormat {
    pub fn of(path: &Path) -> Option<SourceFormat>;   // obj → Obj; gltf, glb → Gltf (case-insensitive)
    pub fn label(self) -> &'static str;              // "OBJ", "glTF" (argument of model.* messages)
}
/// The finding of `bake` as a part message; `area` is the first argument of texture.vram-high.
pub(crate) fn from_bake(m: &BakeMessage, area: u8) -> PartMessage;
// model_import/mesh.rs
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh { pub positions: Vec<[f32; 3]>, pub uvs: Vec<[f32; 2]>, pub polygons: Vec<Polygon>, pub materials: Vec<MeshMaterial> }
// positions in model units (y up, node transforms applied); uvs as fractions of the texture, t downwards
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon { pub material: Option<usize>, pub corners: Vec<(u32, Option<u32>)> }   // (position, uv), ≥ 3
#[derive(Debug, Clone, PartialEq)]
pub struct MeshMaterial { pub name: String, pub image: Option<MeshImage> }
#[derive(Debug, Clone, PartialEq)]
pub struct MeshImage { pub stem: String, pub file: Option<PathBuf>, pub rgba: Rgba }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding { pub texture: String, pub width: u16, pub height: u16 }
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct MeshStats { pub polygons: usize, pub quads: usize, pub triangles: usize, pub dropped: usize }
#[derive(Debug, Clone, PartialEq)]
pub struct Converted { pub faces: Vec<Face>, pub stats: MeshStats, pub notes: Vec<PartMessage> }
/// Faces of `mesh` (layer 0), material m bound to `bindings[m]`; refuses with the first error.
pub fn to_faces(mesh: &Mesh, unit: f32, bindings: &[Option<Binding>]) -> Result<Converted, PartMessage>;
pub(crate) fn ear_clip(points: &[[f64; 3]]) -> Vec<[usize; 3]>;   // n − 2 triangles, fan on failure
pub(crate) fn pair_triangles(faces: Vec<Face>) -> Vec<Face>;      // triangles merged into quads where allowed
```

- [ ] **Step 1: Scheiternde Tests** in `mesh.rs` (Netze aus Code; `bind(name, w, h)` als Hilfe):
  - `a_unit_quad_becomes_one_face_scaled_by_unit`: Viereck `(0,0,0) (0,0,1) (1,0,1) (1,0,0)`, `unit 16`, Bindung 16 × 16, UV `(0,0) (0,1) (1,1) (1,0)` → eine `Face` mit Ecken `(0,0,0) (0,0,16) (16,0,16) (16,0,0)`, UV `(0,0) (0,16) (16,16) (16,0)`, Textur `t`, Ebene 0, `stats.quads 1`.
  - `uv_is_scaled_by_the_bound_texture`: UV `(0.5, 0.25)` bei 32 × 16 → `(16, 4)`.
  - `rounding_goes_to_the_nearest_world_unit`: `unit 16`, x `0.03` → 0, `0.04` → 1, `-0.04` → −1.
  - `two_coplanar_triangles_sharing_an_edge_become_one_quad`: Ebene aus `[0,1,2]`, `[0,2,3]` → eine Fläche, ihre Ecken eine zyklische Drehung von `v0 v1 v2 v3`, `strips::normal == [0, 511, 0]`, `stats.quads 1`, `triangles 0`.
  - `triangles_are_not_paired_across_a_crease_a_uv_seam_a_material_or_into_a_concave_quad`: je zwei Dreiecke → zwei Dreiecks-Flächen (`is_triangle`).
  - `a_non_planar_or_concave_quad_is_split`: Viereck mit einer Ecke 4 E über der Ebene → zwei Dreiecke; ein Pfeil-Viereck → zwei Dreiecke, Flächensumme gleich.
  - `an_ngon_is_ear_clipped`: L-förmiges Sechseck → vor dem Paaren 4 Dreiecke, Flächensumme gleich der des Sechsecks; nach `to_faces` Fläche gleich.
  - `ear_clip_covers_every_convex_polygon` (proptest: regelmäßiges n-Eck, 3 ≤ n ≤ 12, Radius 1 bis 100, gedreht): `n − 2` Dreiecke, Flächensumme gleich (± 1e-6 relativ).
  - `faces_that_collapse_when_rounded_are_dropped_with_a_note`: `unit 1`, ein Dreieck mit Kanten 0,1 → wegfallen, Hinweis `model.faces-dropped` `["1"]`, `stats.dropped 1`; `a_model_that_collapses_entirely_has_no_geometry` → `Err(model.no-geometry)`.
  - `a_polygon_without_binding_is_refused` (`model.material-untextured`, Name des Materials bzw. leer), `a_textured_polygon_without_uv_is_refused` (`model.uv-missing`).
  - `corners_outside_the_map_range_are_refused`: `unit 16`, x 40 → `model.out-of-range` mit `["640", "0", "0", "-512", "511"]`.
  - `winding_is_kept`: das Viereck aus dem ersten Test zeigt nach oben (`normal [0, 511, 0]`); dasselbe mit umgekehrter Eckenfolge nach unten.
  - in `mod.rs`: `options_default_and_check` (Vorgabe 16/16; `unit 0`, `unit f32::NAN`, `unit 300`, `colors 8` → Fehler), `source_format_follows_the_extension` (`a.OBJ`, `b.glb`, `c.gltf`, `d.png` → `None`).
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- model_import::` → FAIL.
- [ ] **Step 3: Umsetzen** nach „Polygone“, „Genauigkeit“, „UV“ unter „Entscheidungen“; Ebenen-Toleranz 0,5 Welteinheiten nach dem Skalieren, vor dem Runden gerechnet in `f64`.
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(model-import): neutral mesh to baker faces with ear clipping and triangle pairing` (mit Trailer).

### Task 3: Beispieldateien und `g4k import-samples`

**Files:**
- Create: `crates/g4-kernel/src/model_import/samples.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/model_import/mod.rs` (`pub mod samples;`), `crates/g4-kernel/src/bin/g4k.rs` (Unterbefehl `import-samples <dir>`), `crates/g4-kernel/tests/cli.rs` (Test)

**Interfaces:**
- Consumes: `png`, `serde_json`.
- Produces:

```rust
pub const CUBE_OBJ: &str = "g4_imp_cube.obj";        // + g4_imp_cube.mtl, g4_imp_cube.png
pub const PLANE_GLB: &str = "g4_imp_plane.glb";      // PNG im BIN-Chunk
pub const PLANE_GLTF: &str = "g4_imp_plane.gltf";    // + g4_imp_plane.bin, g4_imp_plane.png
pub const PDSMS_OBJ: &str = "g4_pds_feld.obj";       // + g4_pds_feld.mtl, g4_pds_gras.png, g4_pds_weg.png
pub const BIG_OBJ: &str = "g4_imp_big.obj";          // + g4_imp_big.mtl (nutzt g4_imp_cube.png)
pub const PNG_40: &str = "g4_imp_40.png";
/// Writes every sample (14 files) into `dir` (created); returns the paths in a fixed order.
pub fn write_samples(dir: &Path) -> Result<Vec<PathBuf>, KernelError>;
/// The 40 colours of g4_imp_40.png: i in 0..40 → [40 + 28·(i % 8), 40 + 45·(i / 8), 200 − 20·(i % 8)].
pub fn forty_colours() -> [[u8; 3]; 40];
// g4k import-samples <dir>: prints one path per line, exit 0
```

Inhalt (alles deterministisch):
- **Würfel**: Einheitswürfel `x, z` in −0,5..0,5, `y` 0..1, 8 `v`, 4 `vt`, 6 `vn`, sechs Vierecke `f v/vt/vn` gegen den Uhrzeigersinn von außen, UV je Seite 0..1, `mtllib g4_imp_cube.mtl`, `usemtl g4_imp_cube_mat`, MTL `newmtl g4_imp_cube_mat`, `Kd 1 1 1`, `map_Kd g4_imp_cube.png`; PNG 16 × 16, Schachbrett aus 4-Pixel-Feldern (230, 120, 40) und (40, 90, 200) mit Rand (20, 20, 20).
- **Ebene**: 4 × 4 m bei y 0 um den Ursprung, Ecken `(-2,0,-2) (-2,0,2) (2,0,2) (2,0,-2)`, Indizes `[0, 1, 2, 0, 2, 3]` (Normale nach oben), `TEXCOORD_0` 0..4 (vier Wiederholungen), ein Knoten ohne Transformation, Material mit `baseColorTexture` → Bild `g4_imp_plane` (PNG 16 × 16, diagonale Streifen `((x + y) / 4) % 4` aus vier Grüntönen); `.glb` mit dem PNG als `bufferView` (`mimeType image/png`), `.gltf` mit `uri` auf `g4_imp_plane.bin` und `g4_imp_plane.png`; Chunks auf 4 Byte aufgefüllt (JSON mit Leerzeichen, BIN mit 0).
- **PDSMS-Stil**: Feld 8 × 8 Kacheln (eine Kachel = eine Einheit) um den Ursprung bei y 0, je Kachel ein Viereck mit eigenen `vt` 0..1, Schachbrett aus `usemtl gras` und `usemtl weg`, mit `# Pokemon DS Map Studio style export (synthetic)`, `o feld`, `g feld`, `s off`, `vn 0 1 0`, `f v/vt/vn`; MTL mit `Ka`, `Kd`, `Ks`, `d 1`, `illum 1`, `map_Kd g4_pds_gras.png` bzw. `g4_pds_weg.png` (je 16 × 16, zwei Muster aus Code).
- **Zu groß**: 64 × 64 Vierecke zu 0,5 Einheiten um den Ursprung bei y 0, jedes mit UV 0..1 (kein Strip möglich), Material mit `map_Kd g4_imp_cube.png`.
- **40 Farben**: 64 × 64, Pixel `(x, y)` hat Farbe `forty_colours()[(x / 8) + 8 · min(4, y / 13)]`, deckend.

- [ ] **Step 1: Scheiternde Tests** in `samples.rs`: `samples_are_written_and_deterministic` (zweimal in zwei Ordner → 14 Dateien, gleiche Bytes), `the_40_colour_png_has_40_colours` (mit `png` gelesen: 64 × 64, RGBA oder RGB, genau 40 verschiedene Farben, nach `to_bgr555` weiter 40 verschiedene), `the_glb_is_a_valid_container` (Magie `glTF`, Version 2, Längenfeld = Dateilänge, JSON-Chunk mit `serde_json` lesbar, BIN-Länge durch 4 teilbar, `bufferViews` im BIN), `the_cube_obj_names_its_mtl_and_png` (Textzeilen `mtllib`, `usemtl`, `map_Kd`). In `tests/cli.rs`: `import_samples_writes_the_files` (`g4k import-samples <tmp>` → Exit 0, 14 Zeilen, Dateien da).
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- model_import::samples import_samples_writes` → FAIL.
- [ ] **Step 3: Umsetzen** (GLB-Writer klein und lokal in `samples.rs`: JSON mit `serde_json::to_vec`, auf 4 Byte mit `0x20` aufgefüllt, BIN mit 0).
- [ ] **Step 4: Lauf** → PASS; clippy, fmt; `bash scripts/check-no-rom-data.sh` (es entstehen keine Dateien im Repo).
- [ ] **Step 5: Commit** `feat(model-import): synthetic samples and g4k import-samples` (mit Trailer).

### Task 4: OBJ lesen (`tobj`), Lizenz-Audit

**Files:**
- Create: `crates/g4-kernel/src/model_import/read_obj.rs` (+ Tests)
- Modify: `crates/g4-kernel/Cargo.toml` (`tobj = { version = "4.0.5", default-features = false }`), `Cargo.lock`, `crates/g4-kernel/src/model_import/mod.rs` (`read_model`, `resolve_inside`, `decode_image`; Task 6 macht es als `load_image` öffentlich), `map_import.rs` (Codes `ModelSourceUnreadable`, `ModelFileMissing`, `ModelFileOutside`, `TextureUnreadable`), `importParts.ts` (ihre Sätze)

**Interfaces:**
- Consumes: Task 2 (`Mesh`, `Polygon`, `MeshMaterial`, `MeshImage`), Task 3 (Beispiele in den Tests), `image::load_from_memory`.
- Produces:

```rust
// model_import/mod.rs
/// The mesh of an OBJ or glTF file and its notes (model.parts-ignored); the refusal with code.
pub fn read_model(path: &Path) -> Result<(Mesh, Vec<PartMessage>), PartMessage>;
/// `reference` (as written in a file inside `root`, relative to `from`) as a path inside the
/// canonical folder `root`: model.file-outside for absolute paths, schemes and anything
/// leaving `root` after canonicalising; model.file-missing when it does not exist.
pub(crate) fn resolve_inside(root: &Path, from: &Path, reference: &str) -> Result<PathBuf, PartMessage>;
pub(crate) fn decode_image(bytes: &[u8], file: &str) -> Result<Rgba, PartMessage>;   // texture.unreadable [file, reason]
// model_import/read_obj.rs
pub(crate) fn read_obj(path: &Path) -> Result<(Mesh, Vec<PartMessage>), PartMessage>;
```

Ein OBJ ohne Flächen gibt ein `Mesh` ohne Polygone (die Ablehnung `model.no-geometry` kommt aus `to_faces`).

- [ ] **Step 1: Abhängigkeit**: `tobj` eintragen, `cargo build --workspace`, `git diff Cargo.lock` zeigt genau `tobj` 4.0.5 neu; `cargo deny --locked check licenses advisories` → „advisories ok, licenses ok“.
- [ ] **Step 2: Scheiternde Tests** in `read_obj.rs` (Ordner aus `write_samples`):
  - `the_cube_reads_as_six_quads_with_one_textured_material`: 6 Polygone zu 4 Ecken, alle mit UV, ein Material `g4_imp_cube_mat` mit Bild `stem == "g4_imp_cube"`, 16 × 16, Datei `…/g4_imp_cube.png`; die UV der Ecke mit `vt 0 0` ist `(0, 1)` (t nach unten).
  - `the_pdsms_style_obj_reads_with_two_materials`: 64 Polygone zu 4 Ecken, Materialien `gras`, `weg` mit Bild, je 32 Polygone.
  - `negative_indices_and_missing_uv_are_read`: Text `v …×4`, `f -4 -3 -2 -1` ohne `vt` → ein Polygon, UV `None`.
  - `references_outside_the_folder_are_refused` (Review Focus): `mtllib ../x.mtl` → `model.file-outside` `["../x.mtl"]`; `map_Kd /tmp/x.png` → `model.file-outside`; ein Symlink im Ordner auf eine Datei außerhalb → `model.file-outside`; `mtllib fehlt.mtl` → `model.file-missing`.
  - `a_broken_obj_is_source_unreadable` (`f 1 2 x`) und `a_broken_png_is_texture_unreadable` (8 Byte Müll als `g4_imp_cube.png`).
- [ ] **Step 3: Lauf** `cargo test --workspace --all-features -- model_import::read_obj` → FAIL.
- [ ] **Step 4: Umsetzen** nach „OBJ“ und „Dateien neben dem Modell“; `face_arities` leer heißt „nur Dreiecke“; Positionen und UV aus `single_index`-Netzen je Modell von `tobj` zusammengeführt (Indizes verschoben).
- [ ] **Step 5: Lauf** → PASS; clippy, fmt.
- [ ] **Step 6: Commit** `feat(model-import): read OBJ and MTL with tobj, confined to the model folder` (mit Trailer; im Text die Lizenz von `tobj`).

### Task 5: glTF lesen (`gltf`), Lizenz-Audit

**Files:**
- Create: `crates/g4-kernel/src/model_import/read_gltf.rs` (+ Tests)
- Modify: `crates/g4-kernel/Cargo.toml` (`gltf = { version = "1.4.1", default-features = false, features = ["utils", "names"] }`, `base64 = "0.22"`), `Cargo.lock`, `model_import/mod.rs` (`read_model` verzweigt), `map_import.rs` (Code `ModelPartsIgnored`), `importParts.ts` (Satz und Format `{n:ignored}`), `importParts.test.ts` (`partText` für `["lines,animations"]` → „Nicht übernommen: Linien, Animationen.“)

**Interfaces:**
- Consumes: Task 2, Task 3, Task 4 (`resolve_inside`, `decode_image`).
- Produces:

```rust
pub(crate) fn read_gltf(path: &Path) -> Result<(Mesh, Vec<PartMessage>), PartMessage>;
// model.parts-ignored (hint): tokens as in „Prüfcodes und Sätze“
```

- [ ] **Step 1: Abhängigkeit**: eintragen, `cargo build --workspace`; `git diff Cargo.lock` zeigt genau `gltf`, `gltf-json`, `gltf-derive` 1.4.1, `inflections` 1.1.1, `lazy_static` 1.5.1 neu (`base64` 0.22.1 nur als neue Kante); `cargo deny --locked check licenses advisories` grün.
- [ ] **Step 2: Scheiternde Tests** in `read_gltf.rs`:
  - `the_plane_glb_reads_as_two_triangles_with_its_image`: 4 Positionen, 2 Polygone zu 3 Ecken, UV `(0,0)..(4,4)`, ein Material mit Bild 16 × 16 aus dem BIN-Chunk, `stem == "g4_imp_plane"`, Datei `None`.
  - `the_plane_gltf_with_files_equals_the_glb`: `Mesh` aus `.gltf` (externe `.bin`/`.png`) gleich dem aus `.glb` bis auf `image.file`.
  - `data_uris_are_read`: im Test aus der `.gltf` ein JSON mit `data:application/octet-stream;base64,…` und `data:image/png;base64,…` gebaut → gleiches `Mesh`.
  - `node_transforms_are_applied_and_a_mirror_flips_the_winding` (Review Focus): Knoten mit Verschiebung (1, 0, 2) und Skalierung 2 → Positionen verschoben und skaliert; Skalierung (−1, 1, 1) → Eckenfolge jedes Polygons umgedreht (die Normale nach `to_faces` zeigt weiter nach oben).
  - `a_mesh_at_two_nodes_is_imported_twice`: 4 Polygone.
  - `strips_and_fans_are_unfolded`: `TriangleStrip` mit 4 Ecken → 2 Dreiecke mit gleicher Wickelung; `TriangleFan` mit 5 → 3.
  - `lines_and_points_are_ignored_with_a_note`: ein `Lines`-Primitiv neben der Ebene → Hinweis `model.parts-ignored` `["lines"]`, die Ebene bleibt; eine Animation dazu → `["lines,animations"]`.
  - `gltf_uris_outside_the_folder_or_with_a_scheme_are_refused` (Review Focus): `uri "../a.bin"`, `"https://example.com/a.bin"`, `"/abs/a.png"` → `model.file-outside`.
  - `required_extensions_and_missing_positions_are_source_unreadable`: `extensionsRequired: ["KHR_draco_mesh_compression"]` → `model.source-unreadable` mit dem Namen; Primitiv ohne `POSITION` → ebenso.
- [ ] **Step 3: Lauf** `cargo test --workspace --all-features -- model_import::read_gltf` → FAIL.
- [ ] **Step 4: Umsetzen** nach „glTF“ unter „Entscheidungen“ (`gltf::Gltf::from_slice`, Puffer selbst auflösen: `Source::Bin` → `blob`, `Source::Uri` → `data:` mit `base64::engine::general_purpose::STANDARD` oder Datei über `resolve_inside`; `primitive.reader(|b| …)`, `read_positions`, `read_tex_coords(0).into_f32()`, `read_indices().into_u32()`, ohne Indizes 0..n; Knoten-Matrix aus `transform().matrix()` (Spalten zuerst) rekursiv multipliziert).
- [ ] **Step 5: Lauf** → PASS; clippy, fmt.
- [ ] **Step 6: Commit** `feat(model-import): read glTF and GLB with gltf, node transforms, confined resources` (mit Trailer; im Text die neuen Crates mit Lizenz).

### Task 6: Texturen planen, PNG → NSBTX, Gate „40 Farben“

**Files:**
- Create: `crates/g4-kernel/src/model_import/textures.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/model_import/mod.rs` (Re-Exporte), `crates/g4-kernel/src/bake/textures.rs` (`appended_pack_or_refusal`; `appended_pack` und `append_textures` darüber, Verhalten gleich), `map_import.rs` (Codes `TextureSide`, `TextureName`, `TextureTaken`, `TexturePackFull`, `TextureNew`, `TextureReused`, `TextureVramHigh`), `importParts.ts` (ihre Sätze und Format `{n:dec}`), `importParts.test.ts` (`texture.new` mit `["g4_imp_cube","16","16","16","7.6"]` → „… mittlerer Farbfehler 7,6.“), `docs/formats/nsbtx.md` (Abschnitt „Import aus PNG“)

**Interfaces:**
- Consumes: Task 2 (`MeshMaterial`, `Binding`), Task 4 (`decode_image`), `texconv::{convert, Colors, palette_name}`, `bake::PackTexture`.
- Produces:

```rust
// model_import/textures.rs
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct TexturePreview { pub name: String,
    #[serde(serialize_with = "crate::relpath::ser_opt")] pub file: Option<PathBuf>,
    pub width: u32, pub height: u32, pub colors: u8, pub new: bool, pub mean_error: Option<f32> }
#[derive(Debug, Clone, PartialEq)]
pub struct TexturePlan { pub bindings: Vec<Option<Binding>>, pub new: Vec<NewTexture>,
                         pub previews: Vec<TexturePreview>, pub notes: Vec<PartMessage> }
/// Printable ASCII 0x21..=0x7E kept, every other byte '_', cut to 16 bytes, empty → "g4_tex".
pub fn texture_name(stem: &str) -> String;
pub fn load_image(bytes: &[u8], file: &str) -> Result<Rgba, PartMessage>;   // = decode_image, public
/// One binding per material (rules „Materialien und Texturen“); new textures converted with `colors`.
pub fn plan_textures(materials: &[MeshMaterial], pack: &[PackTexture], colors: Colors) -> Result<TexturePlan, PartMessage>;
/// The PNG files as new textures (no bindings); a name already in the pack is texture.taken.
pub fn png_textures(files: &[PathBuf], pack: &[PackTexture], colors: Colors) -> Result<TexturePlan, PartMessage>;
// bake/textures.rs
/// The pack `before` with `textures` appended, or the refusal as a message (texture.side, .name,
/// .taken, .pack-full); the outer error is a pack that cannot be read.
pub(crate) fn appended_pack_or_refusal(before: &[u8], textures: &[NewTexture]) -> Result<Result<Vec<u8>, BakeMessage>, KernelError>;
```

- [ ] **Step 1: Scheiternde Tests** in `textures.rs`:
  - `a_64x64_png_with_40_colours_becomes_a_16_colour_texture_within_one_step` (**Gate**): `write_samples` → `png_textures(&[g4_imp_40.png], &[], Sixteen)` → eine `NewTexture` Format 3, 16 Farben, 64 × 64; `nsbtx::build(&plan.new)` → `Nsbtx::read` → `decode`; je Pixel: `q = to_bgr555(quelle)`, ist `q` in der Palette („Palettentreffer“), dann ist je Kanal `|kanal5(to_bgr555(dekodiert)) − kanal5(q)| ≤ 1`; mindestens ein Treffer; druckt `png40 hits=<farben getroffen>/40 pixels=<n>/4096 max_step=<n> mean=<f>`.
  - `texture_name_keeps_printable_ascii_and_cuts_to_16`: `"Gras Weg"` → `"Gras_Weg"`, `"größe"` → `"gr__e"` (Bytes), 20 Zeichen → 16, `""` → `"g4_tex"`.
  - `two_images_with_one_stem_get_numbered_names`: zwei Materialien mit Stamm `kachel`, verschiedene Bilder → `kachel`, `kachel_2`; gleiches Bild zweimal → eine neue Textur, zwei Bindungen auf denselben Namen.
  - `a_pack_texture_of_the_same_name_is_reused_for_a_model_but_taken_for_png` (Review Focus): Paket mit `g4_imp_cube` → `plan_textures`: `new` leer, Hinweis `texture.reused`, Bindung mit Größe aus dem Paket; `png_textures` → `Err(texture.taken)` `["texture", "g4_imp_cube"]`.
  - `a_material_without_image_binds_the_pack_texture_of_its_name` und `an_unknown_material_without_image_is_untextured`.
  - `sides_not_power_of_two_are_texture_side` (24 × 16 → `["kachel", "24", "16"]`), `an_unreadable_image_is_texture_unreadable`, `four_colours_when_asked` (Format 2, 4 Farben).
  - in `bake/textures.rs`: `appended_pack_or_refusal_gives_the_message` (Name doppelt → `Ok(Err(m))` mit `m.code == TextureTaken`); die Tests von 6.1 bleiben grün.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- model_import::textures bake::textures` → FAIL.
- [ ] **Step 3: Umsetzen**; `TexturePreview.mean_error` aus `Quantized::mean_error`, bei wiederverwendeten `None`.
- [ ] **Step 4: Lauf** → PASS (mit `--nocapture` die Zeile `png40 …` ins Log `target/agent/6-5a/t6-png40.log`); clippy, fmt.
- [ ] **Step 5: Doku** `nsbtx.md` „Import aus PNG“: Weg PNG → `texconv` → Anhängen, Namensregel, Fehlercodes, keine Skalierung, Messung des 40-Farben-Bildes (Zahlen aus dem Log).
- [ ] **Step 6: Commit** `feat(model-import): plan textures from images and PNG files for the map texture pack` (mit Trailer).

### Task 7: `map_import`: Modell aus OBJ/glTF, Teil „Texturen“, Vorschau, ein Undo-Schritt

**Files:**
- Modify: `apps/studio/src/workbench/inspector/importParts.ts` (Sätze der vier Codes, `PART_NAME.textures = "Texturen"`), `apps/studio/src/api.import.ts` (`MapPart` um `"textures"`, damit `Record<MapPart, …>` übersetzt), `apps/studio/src/workbench/inspector/ExportPartDialog.tsx` (bietet weiter genau die sechs Binärteile an, nicht „Texturen“; Vitest-Fall in `ExportPartDialog.test.tsx`), `crates/g4-kernel/src/map_import.rs` (`Part::Textures`, `from_path`, `label`, die Codes `ModelUvRange`, `ModelTextureCount`, `ModelConverted`, `PartNoArea`, `PartReport.preview`, `ImportPreview`, `ModelPreview`, `inspect_with`, `import_with`, `Payload::{Converted, Textures}`, `prepare` mit `prepare_converted`/`prepare_textures`, `import` mit Anhängen + `squash_since`, `part_bytes`/`export` lehnen `Textures` ab; Tests), `crates/g4-kernel/src/model_import/mod.rs` (`convert_for_map`), `crates/g4-kernel/src/test_support.rs` (`write_texture_packs`, aus `tests/bake.rs` `prepare` verschoben; `tests/bake.rs` nutzt es)
- Create: `docs/formats/model_import.md`

**Interfaces:**
- Consumes: Task 1–6; `bake::{bake_context, bake, BakeInput, pack_textures, area_vram, areas_of_pack, append_textures, vanilla_vram_max}`, `terrain::budget::{BudgetItem, budget_state}`.
- Produces:

```rust
// map_import.rs
pub enum Part { Model, Permissions, Buildings, Bdhc, Bgs, Whole, Textures }   // "textures", label "textures"
pub struct PartReport { /* … */ #[serde(skip_serializing_if = "Option::is_none")] pub preview: Option<ImportPreview> }
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ImportPreview { pub pack: u16, pub area: u8, pub style: AreaStyle, pub vram_bytes: usize,
    pub vram_limit: usize, pub textures: Vec<TexturePreview>, pub model: Option<ModelPreview>,
    #[serde(serialize_with = "crate::relpath::ser_vec")] pub files: Vec<PathBuf> }
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ModelPreview { pub format: SourceFormat, pub stats: MeshStats, pub strips: usize,
    pub single_quads: usize, pub budget: BudgetItem }   // budget: size of 0xF000; stats.triangles = faces sent as triangles
pub fn inspect_with(store: &mut Store, map: u16, source: &Source, opts: &ImportOptions) -> Result<Vec<PartReport>, KernelError>;
pub fn import_with(store: &mut Store, map: u16, source: &Source, parts: &[Part], keep_buildings: bool,
                   opts: &ImportOptions) -> Result<Vec<AssetId>, KernelError>;
// inspect / import: unchanged signatures, call the *_with versions with ImportOptions::default()
// model_import/mod.rs
/// Everything `inspect` needs for a model file of map `map`: the bake (nothing written).
pub struct ConvertedModel { pub faces: Vec<Face>, pub nsbmd: Vec<u8>, pub pack: u16, pub new: Vec<NewTexture>,
    pub preview: ImportPreview, pub state: PartState, pub notes: Vec<PartMessage> }
/// Outer error: the project cannot be read; inner: the refusal with its code.
pub fn convert_for_map(store: &mut Store, map: u16, path: &Path, opts: &ImportOptions) -> Result<Result<ConvertedModel, PartMessage>, KernelError>;
// test_support.rs
/// Map and building texture pack 0 as real BTX0 (two 16 × 16 textures `t0`, `t1`), as 6.1's tests prepare them.
pub fn write_texture_packs(project: &Project);
```

Codes dieses Tasks: `ModelUvRange`, `ModelTextureCount`, `ModelConverted`, `PartNoArea` (an ihrer Stelle in „Prüfcodes und Sätze“, mit Satz); danach hat `ALL` alle 51. `model.too-big` (vorhanden) kommt aus dem Bäcker mit `[größe, 61440]`.

`convert_for_map` (Reihenfolge, jede Ablehnung als `Ok(Err(…))` mit Code): `bake_context` (kein Header → `part.no-area`) → `read_model` → `plan_textures` → Texturzahl ≤ 255 → `appended_pack_or_refusal` (nur wenn `new` nicht leer) → `pack_textures`/`area_vram` des neuen Pakets → `to_faces` → `bake` → Ablehnung des Bäckers über `from_bake`; Zustand `Warning(texture.vram-high)` für das eigene Gebiet, weitere Gebiete des Pakets (`areas_of_pack`) als Hinweise mit demselben Code; Hinweise in der Reihenfolge: `model.converted`, Leser-Hinweise, `model.faces-dropped`, `texture.new`/`texture.reused` je Textur.

- [ ] **Step 1: Scheiternde Tests** im Testmodul von `map_import.rs` (`synthetic_project` + `write_texture_packs`, Beispiele aus `write_samples` im Projektordner):
  - `inspect_converts_a_cube_obj_without_writing` (Review Focus): `inspect` mit `g4_imp_cube.obj` → ein Bericht `part == Model`, `state Ok`, `preview.model.stats.quads == 6`, `preview.textures == [g4_imp_cube, new: true, 16 Farben]`, Hinweise `model.converted`, `texture.new`, `model.bdhc-unchanged`, kein `model.textures-missing`; danach `undo_depth` gleich, Kartenmember und Paketdatei byte-gleich.
  - `import_writes_model_and_pack_as_one_undo_step` (Review Focus): `import(…, [Model])` → `history().0.last() == "map 0000: model imported"`, Rückgabe `[Map 0, MapTexture 0]`, Modell der Karte = `nsbmd` des Berichts, Paket enthält `g4_imp_cube` und die alten Einträge byte-gleich (`Btx0::read`); ein `undo` → Karte und Paket wie vorher; `redo` → wieder neu.
  - `the_plane_glb_and_gltf_give_the_same_model`: beide importieren (auf zwei Kopien) → gleiche Modellbytes; `preview.model.stats.quads == 1`, `triangles == 0`.
  - `the_oversize_obj_is_refused_with_the_number`: `inspect` → `state Error`, Code `model.too-big`, `args[1] == "61440"`, `args[0]` als Zahl > 61 440, Nachricht enthält beide Zahlen; `import` → `Err` mit „61440“; nichts geändert.
  - `the_pdsms_style_obj_imports`: Zustand ok, zwei neue Texturen, `stats.quads == 64`, Import ein Schritt.
  - `pngs_are_one_textures_part_appended_in_one_step`: `g4_imp_40.png` und `g4_imp_cube.png` → **ein** Bericht `part == Textures`, `preview.files` beide, `preview.model == None`, `size` = Zuwachs des Pakets; Import → beide im Paket, Label `map 0000: textures imported`, Rückgabe `[MapTexture 0]`, Karte unverändert.
  - `model_and_textures_together_are_one_step`: Würfel + PNG gewählt → Label `map 0000: model and textures imported`, ein `undo` nimmt beides.
  - `a_png_name_already_in_the_pack_is_taken`: `t0.png` → `Error(texture.taken)`.
  - `reimporting_the_cube_reuses_its_texture` (Review Focus): zweiter Import → `preview.textures[0].new == false`, Hinweis `texture.reused`, Paketgröße unverändert, Rückgabe `[Map 0]`.
  - `unchanged_maps_stay_byte_identical` (Review Focus): nach dem Import in Karte 0 sind alle anderen Kartenmember und alle anderen Pakete byte-gleich.
  - `a_map_without_header_is_part_no_area`, `options_out_of_range_are_refused` (`unit 0`, `colors 8` → `KernelError::Refused`, nichts geprüft), `two_model_files_are_part_duplicate` (`.obj` + `.nsbmd`), `textures_cannot_be_exported` (`export(…, Part::Textures, …)` → Fehler).
  - Die vorhandenen Tests von `map_import` bleiben grün (`inspect`/`import` ohne Optionen).
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- map_import:: model_import::` → FAIL.
- [ ] **Step 3: Umsetzen** nach „Prüfen ohne Schreiben“, „Import als ein Schritt“, „Vorschau“, „Hinweis Texturnamen fehlen“; PNG-Pfade sammelt `prepare` aus `Source::Files` und baut am Ende einen `Prepared` für `Textures`; `import` mit mehreren umgewandelten Teilen hängt alle neuen Texturen in **einem** `append_textures` an (Namen über beide Teile eindeutig, sonst `texture.taken` vor dem Schreiben).
- [ ] **Step 4: Lauf** → PASS; die Tests von `bake` und `tests/bake.rs` (mit `write_texture_packs`) grün; clippy, fmt.
- [ ] **Step 5: Doku** `docs/formats/model_import.md` (neu): Zweck, Dateiarten, Einheit und Achsen, Genauigkeit, Polygone und Paare, Materialien und Texturnamen, Dateien neben dem Modell, glTF-Teilmenge, Vorschau und Codes (Tabelle Code / Argumente / Bedeutung), ein Undo-Schritt, **„Karten aus Pokémon DS Map Studio“** (PDSMS: „Export as OBJ with Textures“ → „Teile importieren…“ mit der OBJ; Einheit 16; BDHC und Kollision bleiben die der Karte; kein `.pdsmap`/`.pdsts`, PDSMS hat keine Lizenz), Quellen (OBJ/MTL-Beschreibung von Wavefront, glTF-2.0-Spezifikation von Khronos, `tobj` MIT, `gltf` MIT/Apache-2.0, eigene Messung), Verifikation (Tests und Gate).
- [ ] **Step 6: Commit** `feat(map-import): import OBJ and glTF models and PNG textures with preview as one undo step` (mit Trailer).

### Task 8: Orakel apicula und gated Tests auf den drei Spielen

**Files:**
- Create: `crates/g4-kernel/tests/model_import.rs`, `crates/g4-kernel/tests/common/glb.rs`
- Modify: `crates/g4-kernel/tests/common/mod.rs` (`pub mod glb;`), `crates/g4-kernel/tests/bake.rs` (`glb_chunks`, `accessor`, `input_set`, `glb_set`, `CornerSet` aus `common::glb`)

**Interfaces:**
- Consumes: Task 7 (`inspect_with`, `import_with`, `ImportPreview`), Task 3 (`write_samples`), `export::nsbmd_to_glb`, `bake::Face`.
- Produces (`tests/common/glb.rs`, für 6.5b wiederverwendbar):

```rust
pub type CornerSet = std::collections::BTreeSet<([i32; 3], [i32; 2])>;   // world units, 1/16 texel
pub fn glb_chunks(glb: &[u8]) -> (serde_json::Value, Vec<u8>);
pub fn accessor(json: &serde_json::Value, bin: &[u8], index: usize, n: usize) -> Vec<Vec<f32>>;
pub fn input_set(faces: &[Face]) -> CornerSet;   // corners and uv · 16
pub fn glb_set(json: &serde_json::Value, bin: &[u8]) -> CornerSet;
```

- [ ] **Step 1: Scheiternde Tests** in `tests/model_import.rs`:
  - `apicula_reads_the_imported_cube_and_plane` (`G4_APICULA`, sonst „skipped“; hermetisch auf `synthetic_project` + `write_texture_packs`): Würfel und Ebene je importieren, Modell der Karte → `nsbmd_to_glb` (Exit 0, genau eine GLB prüft die Funktion) → `glb_set` gleich `input_set` der Flächen (die Flächen über `convert_for_map` ein zweites Mal berechnet; dieselben Eingaben geben dieselben Flächen).
  - `vanilla_cube_and_plane_in_the_rooms` (gated, `vanilla_copy()`): Karten nach Spielcode (CPUD 187 und 186, IPKD/IPKE 217 und 216); Würfel → 187/217, Ebene (`.glb`) → 186/216; je `inspect` ohne Fehler, Paket 20 bzw. 25, Stil `indoor`; nach dem Import alte Einträge des Pakets byte-gleich, `Nsbtx::decode` der neuen Textur gleich der Quantisierung; mit `G4_APICULA` die Mengenprobe wie oben; alle anderen Kartenmember byte-gleich; zwei `undo` → Karten und Paket wie vorher; druckt `import-room <code> cube map=<n> size=<n> share=<p> quads=<n> plane map=<n> size=<n> quads=<n> tris=<n> pack=<n> appended=<namen> vram=<n> warn=<codes> ms=<n>`.
  - `vanilla_png_40_colours_into_the_room_pack` (gated): `g4_imp_40.png` als Teil „Texturen“ in Karte 187/217 → Paket 20/25 um eine Textur länger, alte Einträge byte-gleich, Treffer-Probe wie in Task 6 auf dem Member nach dem Import; druckt `png40-room <code> pack=<n> hits=<n>/40 max_step=<n> mean=<f>`.
  - `vanilla_oversize_is_refused_with_the_number` (gated): `g4_imp_big.obj` → `model.too-big`, `args[1] == "61440"`; `import` → Fehler mit „61440“; Kopie unverändert (Karte und Paket byte-gleich).
  - `vanilla_pdsms_style_obj_imports` (gated): in 187/217, Zustand ok, Import ein Schritt.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features --test model_import --test bake` → FAIL (Datei fehlt), danach `bake` grün.
- [ ] **Step 3: Umsetzen** (Helfer verschieben, Tests schreiben; kein Produktcode, außer ein Befund verlangt ihn: dann eigener Commit `fix(model-import): …` mit Regressionstest).
- [ ] **Step 4: Lauf** hermetisch und mit `G4_APICULA`; gated im Hintergrund auf frischen Kopien `6-5a-pt`, `6-5a-hgde`, `6-5a-hgus`, je Spiel: `G4_TEST_PROJECT="$G4_ROMS_DIR/_testkopien/6-5a-pt_DSPRE_contents" G4_APICULA=… cargo test --release --workspace --all-features --test model_import -- --nocapture --test-threads 1 > target/agent/6-5a/t8-pt.log 2>&1`; erwartet 5/5, Zeilen `import-room`, `png40-room`.
- [ ] **Step 5: Commit** `test(model-import): apicula oracle and gated imports on the three games` (mit Trailer).

### Task 9: Studio: Befehle mit Optionen, Import-Dialog mit Vorschau, Design-System-Nachtrag

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/map_import.rs` (`options: Option<ImportOptions>` an beiden Befehlen, Tests), `commands/dispatch.rs` (beide Fälle mit `options: Option<ImportOptions>`, die zwei Fälle in `every_command_accepts_the_keys_api_ts_sends` mit `"options": { "unit": 16, "colors": 16 }`, Test `inspect_over_the_bridge_converts_an_obj`), `apps/studio/src/api.import.ts` (`ImportOptions`, `ImportPreview`, `ModelPreview`, `TexturePreview`, `PartReport.preview?`, `inspectMapImport(map, source, options?)`, `importMapParts(map, source, parts, keepBuildings, options?)`), `apps/studio/src/api.import.test.ts`, `apps/studio/src/workbench/inspector/importParts.ts` (`previewLines(r)`), `importParts.test.ts`, `ImportPartsDialog.tsx` (Dateifilter, Optionen, Vorschau), `ImportPartsDialog.test.tsx`, `apps/studio/src/platform/dialogs.ts` (`openFiles` mit `filters`), `docs/mcp.md` (Absatz im „Expertenmodus“), `design-system/README.md` (Abschnitt „Nachtrag 6.5a“, Zeile „Kartenteile importieren“ ergänzt)
- Create: `design-system/openpencil/NACHTRAG-6-5a.md` (Anleitung für die Frames, Muster `NACHTRAG-6-2.md`), `design-system/openpencil/beispiele/import-modell.body.js` (Vorlage des Frames)

**Interfaces:**
- Consumes: Task 7.
- Produces:

```rust
pub fn inspect_map_import_impl(state: &AppState, map: u16, source: &Source, options: &ImportOptions) -> Result<Vec<PartReport>, CmdError>;
pub fn import_map_parts_impl(state: &AppState, map: u16, source: &Source, parts: &[Part], keep_buildings: bool, options: &ImportOptions) -> Result<Vec<AssetId>, CmdError>;
// dispatch: { map: u16, source: Source, options: Option<ImportOptions> } and { …, keep_buildings: Option<bool>, options: Option<ImportOptions> }
```

```ts
// MapPart has "textures" since Task 7
export interface ImportOptions { unit?: number; colors?: 4 | 16 }
export interface TexturePreview { name: string; file: string | null; width: number; height: number; colors: number; new: boolean; mean_error: number | null }
export interface ModelPreview { format: "obj" | "gltf"; stats: { polygons: number; quads: number; triangles: number; dropped: number }; strips: number; single_quads: number; budget: { used: number; max: number; state: "calm" | "tight" | "full" } }
export interface ImportPreview { pack: number; area: number; style: "outdoor" | "indoor"; vram_bytes: number; vram_limit: number; textures: TexturePreview[]; model: ModelPreview | null; files: string[] }
/** The lines under a converted part: „OBJ · 6 Vierecke · 0 Dreiecke · 2 Streifen“, one per texture „g4_imp_cube · 16 × 16 · 16 Farben · neu“ (or „im Paket“), „VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B“. */
export function previewLines(r: PartReport): string[];
```

Die Sätze aller Codes stehen seit Task 2 bis 7 in `PART_TEXTS` (Tabelle „Prüfcodes und Sätze“); Task 9 prüft nur, dass die Liste vollständig ist.

Dialog (Muster und Wörter des Design-Systems, Abschnitt 3b.4b): `openFiles` (`platform/dialogs.ts`) bekommt ein optionales `filters: { name: string; extensions: string[] }[]` (an `tauriOpen` durchgereicht); der Dialog gibt einen Filter „Kartenteile und Modelle“ mit `nsbmd, bmd, per, mp, bld, bdhc, bgs, bin, obj, gltf, glb, png` und den Titel „Teile oder Modelle wählen“; enthält die Pfadliste eine `.obj`/`.gltf`/`.glb` oder `.png`, erscheint unter der Quelle eine Zeile „Umwandeln“ mit `Segmented` „Farben je Textur“ (`16`, `4`, Vorgabe 16) und, nur bei einem 3D-Modell, `TextField` „Einheit“ (Zahl, Vorgabe 16, Hilfe „Welteinheiten je Modelleinheit; 16 = eine Kachel“, ungültig: „Zwischen 0,0625 und 256.“); jede Änderung prüft neu (`inspectMapImport` mit `options`). Unter der Zeile eines umgewandelten Teils `previewLines` als kleine Zeilen und beim Modell `Budget` „Modell“ mit `used`/`max`/`state` aus `preview.model.budget`, `unit="B"`, Tüfteln `tech="0xF000"`. Teile mit Fehler bleiben nicht ankreuzbar; die Taste bleibt „N Teile übernehmen“.

- [ ] **Step 1: Scheiternde Tests:**
  - Rust `commands/map_import.rs`: `every_part_code_has_its_german_sentence` (vorhanden, jetzt mit 51 Codes), `inspect_converts_an_obj_over_the_command_path` (`open_synthetic` + `write_texture_packs` + `write_samples` im Fixture → Bericht `model`, `preview.model.stats.quads == 6`), `import_with_options_four_colours` (`colors 4` → neue Textur Format 2).
  - Rust `dispatch.rs`: die zwei Fälle mit `options` in `every_command_accepts_the_keys_api_ts_sends`; `options_out_of_range_over_the_bridge` (`unit 0` → Fehler); die Pfadregeln (`confine_source`) gelten für `.obj`/`.png` wie für `.per` (vorhandener Test um einen Fall `a.obj` außerhalb erweitert).
  - Vitest `api.import.test.ts`: `inspectMapImport(187, { files: ["/p/a.obj"] }, { unit: 16, colors: 4 })` ruft `invoke("inspect_map_import", { map: 187, source: { files: ["/p/a.obj"] }, options: { unit: 16, colors: 4 } })`; ohne Optionen ohne Schlüssel `options`.
  - Vitest `importParts.test.ts`: `previewLines` für einen Würfel-Bericht (drei Zeilen wie im Kommentar), für „Texturen“ ohne Modellzeile.
  - Vitest `ImportPartsDialog.test.tsx`: `a_model_file_shows_the_options_and_the_preview` (Pfad `a.obj` → Zeile „Umwandeln“ mit „Farben je Textur“ und „Einheit“, Budget „Modell 21.740 von 61.440 B“ aus einem gestellten Bericht, Taste „1 Teil übernehmen“), `changing_colours_inspects_again_with_options` (Klick auf „4“ → zweiter Aufruf mit `colors: 4`), `a_png_only_shows_colours_not_unit`, `an_oversize_model_is_not_selectable_and_says_why` („Das Modell ist 139.264 Byte groß, erlaubt sind 61.440. …“, Kästchen gesperrt), `import_passes_the_options`.
- [ ] **Step 2: Lauf** `cargo test -p studio -- map_import dispatch::tests::every_command` und `(cd apps/studio && pnpm test -- importParts ImportPartsDialog api.import)` → FAIL.
- [ ] **Step 3: Umsetzen**; `docs/mcp.md` im Abschnitt „Expertenmodus“: „Kartenteile importieren (seit 3b.4b, mit 3D-Modellen und PNG seit 6.5a): `inspect_map_import {map, source, options?}` (lesend) und `import_map_parts {map, source, parts, keepBuildings?, options?}` (schreibend, ein Undo-Schritt), nur über `g4_call`. `.obj`, `.gltf`, `.glb` sind der Teil `model`, alle `.png` zusammen der Teil `textures`; `options` `{unit (Welteinheiten je Modelleinheit, Vorgabe 16 = eine Kachel), colors (16 oder 4)}`. Der Bericht trägt bei umgewandelten Teilen `preview` (Paket, VRAM, Texturen neu oder vorhanden, beim Modell Flächen, Strips und `budget` gegen 61 440 B); ein zu großes Modell ist `model.too-big` mit der Größe. Paket und Karte ändern sich zusammen; `undo` nimmt beides. Formate: [model_import.md](formats/model_import.md).“
- [ ] **Step 4: Lauf** → PASS; `cargo test -p studio --test mcp_stdio`; `pnpm build`; `check-styles.sh`, `check-tokens.sh`; clippy, fmt.
- [ ] **Step 5: Design-System:** README „Nachtrag 6.5a“ (Stand, was neu ist: Zeile „Umwandeln“ mit `Segmented` und `TextField`, Vorschauzeilen unter dem Teil, `Budget` im Dialog, Teil „Texturen“; keine neuen Tokens, keine neuen Symbole; Zeile „Kartenteile importieren“ in der Liste der Funktionen um „OBJ, glTF und PNG (6.5a)“ ergänzt; „Nicht in 6.5a“: 3D-Vorschau im Dialog, Skalieren von Bildern). `openpencil/NACHTRAG-6-5a.md`: Frame „Teile importieren: Modell aus OBJ“ (Hell und Dunkel, 480 px Dialog) mit Würfel-Zeile, Vorschau, Budget und einer Fehlerzeile `model.too-big`; Vorlage `beispiele/import-modell.body.js` nach dem Muster der 6.2-Vorlage. Antwortet OpenPencil nicht oder greift die 10 000-Knoten-Grenze, bleibt die `.fig` unangetastet und der Nachtrag sagt das (wie 6.2).
- [ ] **Step 6: Commit** `feat(studio): import dialog converts OBJ, glTF and PNG with options and preview` (mit Trailer); Design-System als eigener Commit `docs(design-system): addendum 6.5a import dialog with preview` (mit Trailer).

### Task 10: g4emu: Vorbereitung `make-import-samples`, Szenarien `pt-import` und `hg-import`

**Files:**
- Modify: `scripts/g4emu/g4emu/scenario.py` (`_PREPARE_STEPS["make-import-samples"] = ("path",)`; `_prepare(project, step, g4k)` bekommt die Befehlszeile aus `g4k_command(repo)` und verzweigt nach Art: `make-import-samples` legt den Ordner im Projekt an und ruft `import_samples_command(g4k, ziel)`; der Aufrufer in `run_scenario` reicht `g4k_command(repo)` durch), `scripts/g4emu/tests/test_scenario.py` (Test)
- Create: `scripts/agent/scenarios/pt-import.yaml`, `scripts/agent/scenarios/hg-import.yaml`

**Interfaces:**
- Consumes: Task 3 (`g4k import-samples`), Task 9 (`options` über `g4_call`).
- Produces: `def import_samples_command(g4k: list[str], target: Path) -> list[str]` (`[*g4k, "import-samples", str(target)]`).

`pt-import.yaml` (Kopfkommentar nach `pt-bake.yaml`: Zweck Gate 6.5a Zusatzpunkt „in g4emu sichtbar“, Zimmer Karte 187 (Header 415) mit dem Würfel, Erdgeschoss Karte 186 (Header 414) mit der Ebene, Paket 20; Bilder nicht ins Repo):

```yaml
game: CPUD
mcp: [--expert]
prepare:
  - [make-import-samples, {path: import}]
edits:
  - - g4_call
    - {command: inspect_map_import, args: {map: 187, source: {files: [import/g4_imp_big.obj]}}}
    - expect:
        - {path: "0.state.code", equals: model.too-big}
        - {path: "0.state.args.1", equals: "61440"}
  - - g4_call
    - {command: import_map_parts, args: {map: 187, source: {files: [import/g4_imp_big.obj]}, parts: [model]}}
    - expect: [{error: "61440"}]
  - [g4_call, {command: import_map_parts, args: {map: 187, source: {files: [import/g4_imp_cube.obj]}, parts: [model], options: {unit: 16, colors: 16}}}, cube]
  - [g4_call, {command: import_map_parts, args: {map: 186, source: {files: [import/g4_imp_plane.glb]}, parts: [model]}}, plane]
  - [g4_call, {command: export_map_part, args: {map: 187, part: model, path: build/import/cube-0187.nsbmd}}]
  - [g4_call, {command: export_map_part, args: {map: 186, part: model, path: build/import/plane-0186.nsbmd}}]
  - [save, {}]
build: build/out.nds
check:
  - [g4k, check-data, build/out.nds,
     --expect, "map:187:model_sha1=${sha1-of:build/import/cube-0187.nsbmd}",
     --expect, "map:186:model_sha1=${sha1-of:build/import/plane-0186.nsbmd}"]
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

`hg-import.yaml`: `game: IPKD`, Karten 217 (Würfel) und 216 (Ebene), `expect-map 64`, `shot zimmer`, `walk left 1`, `walk down 1`, `expect-map 63`, `wait 150`, `shot unten` (Schritte wie `hg-bake.yaml`).

- [ ] **Step 1: Scheiternder Test** `test_make_import_samples_is_a_prepare_step` (Szenario mit dem Schritt lädt; ohne `path` Fehler „path fehlt“; `import_samples_command` gibt die Liste oben; ein Ziel außerhalb des Projekts wird wie bei `make-oversize-nsbmd` abgelehnt).
- [ ] **Step 2: Lauf** `cd scripts/g4emu && uv run pytest -m "not integration" -q -k import_samples` → FAIL.
- [ ] **Step 3: Umsetzen**; dann Kopien `6-5a-szen-pt`, `6-5a-szen-hg` (je Ordner und `.nds`), `cargo build --workspace --all-features`, `G4_STUDIO_BIN` und `G4K_BIN` auf `target/debug/studio` und `target/debug/g4k` des Worktrees.
- [ ] **Step 4: Lauf** im Hintergrund je Spiel aus `scripts/g4emu`: `uv run g4emu scenario ../agent/scenarios/pt-import.yaml --project "$G4_ROMS_DIR/_testkopien/6-5a-szen-pt_DSPRE_contents" --out "$W/target/agent/gate-$(date +%F)/6-5a/pt-import" > "$W/target/agent/6-5a/t10-pt.log" 2>&1`; erwartet Rückgabe 0, `"ok": true`, zwei erfüllte Erwartungen und eine erwartete Ablehnung, „2 expectation(s) met“ im `check`, Header 415 → 414 (HG 64 → 63).
- [ ] **Step 5: Blick des Agenten** auf `zimmer.png` (Würfel mit orange-blauem Schachbrett, sonst schwarz: der Raum hat nur noch den Würfel) und `unten.png` (grüne Streifen-Ebene); liegt der Würfel außer Sicht, im Kopfkommentar festhalten und `unit` nicht ändern (die Lage ist die Kartenmitte).
- [ ] **Step 6: Commit** `test(g4emu): make-import-samples and scenarios pt-import, hg-import` (mit Trailer).

### Task 11: Gate 6.5a, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-6-5a-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 6.5a“), `docs/ROADMAP.md` (Stand unter der Tabelle „Phase 6“, Übersicht Zeile 6, Zeile 196 „Nicht in 3b.4“: OBJ/glTF-Import und PNG → NSBTX mit Verweis „seit 6.5a“, PDSMS-Zeile nach „später, nur wenn ein Nutzer es braucht“)

**Interfaces:**
- Consumes: alles Vorige; die Druckzeilen `png40`, `import-room`, `png40-room` und die Szenarien.

- [ ] **Step 1: Gate-Dokument** nach dem Muster von `docs/phase-6-1-gate.md`: Kopf mit Stand und Branch; das Gate der Spezifikation §4 Zeile 6.5a als Zitat: „Ein 64 × 64-PNG mit 40 Farben wird 16-Farben-NSBTX, zurückdekodiert mit Farbfehler ≤ 1 BGR555-Stufe je Kanal für Palettentreffer; eine Würfel-OBJ und eine Ebene-glTF (im Test erzeugt) werden Modell-Teil einer Karte, apicula liest sie, Ablehnung über 0xF000 mit Zahl; eine OBJ im PDSMS-Stil (Vierecke + MTL) wird importiert; `cargo deny` grün mit `tobj` und `gltf`“, dazu die Kopfzeile der Tabelle (gated Tests auf frischen Kopien der drei Spiele, Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer) und der **Zusatzpunkt der Hauptsitzung** „Würfel und Ebene in g4emu sichtbar“ (nicht in der Spezifikation, Abschnitt „Abweichungen“); Legende „Wer“; Vorbereitung (Kopien `6-5a-{pt,hgde,hgus}` für (1), `6-5a-szen-{pt,hg}` für (3), `6-5a-web` für (4), Werkzeuge, `tools`-Symlink, `G4_ROMS_DIR` und `G4_APICULA` absolut, Ausgaben unter `target/agent/6-5a/`, `uptime`).
  - **(1) Automatik (Agent):** `cargo deny --locked check licenses advisories` „advisories ok, licenses ok“ mit den neuen Crates (Liste mit Version und Lizenz); fmt, clippy, `env -u G4_TEST_PROJECT cargo test -p g4-formats -p g4-kernel --all-features`, `cargo test -p studio`, `(cd apps/studio && pnpm test && pnpm build)`, `check-styles.sh`, `check-tokens.sh`, `check-no-rom-data.sh`, `uv run pytest -m "not integration" -q`; je Spiel im Release-Profil `--test model_import` (5/5) und `--test bake` (unverändert grün); `scripts/gate.sh` läuft nach dem Merge auf main.
  - **(2) Unit-Punkte des Gates** mit Testnamen: `a_64x64_png_with_40_colours_becomes_a_16_colour_texture_within_one_step`, `import_writes_model_and_pack_as_one_undo_step`, `the_plane_glb_and_gltf_give_the_same_model`, `apicula_reads_the_imported_cube_and_plane`, `the_oversize_obj_is_refused_with_the_number`, `the_pdsms_style_obj_imports`.
  - **(3) Spiel (Agent + Blick):** `pt-import`, `hg-import` (Befehle aus Task 10), Bilder `zimmer.png`, `unten.png` je Spiel.
  - **(4) Browser (Agent):** Test-Umgebung `G4_ROMS_DIR=$R G4_VITE_PORT=1500 G4_BRIDGE_PORT=1501 bash scripts/agent/studio-web.sh > target/agent/6-5a/studio-web.log 2>&1 &`; vorher `target/debug/g4k import-samples $R/_testkopien/6-5a-web_DSPRE_contents/import`; Seite `http://127.0.0.1:1500/?project=$R/_testkopien/6-5a-web_DSPRE_contents`, 1440 × 900, Bauen. Ablauf: (a) Zelle der Karte 187 wählen (Ort „Zuhause, 1. Stock“ über `call.sh list_headers`, Header 415), Inspector-Kopf › Aktionen › „Teile importieren…“, Quelle „Datei“, Pfad `import/g4_imp_big.obj` → Zeile „Modell“ mit Fehler „Das Modell ist … Byte groß, erlaubt sind 61.440. …“, Kästchen gesperrt; (b) Pfad `import/g4_imp_cube.obj` → Zeile „Modell“ ok mit „OBJ · 6 Vierecke …“, „g4_imp_cube · 16 × 16 · 16 Farben · neu“, Budget „Modell“ Ruhig, Zeile „Umwandeln“ mit „Farben je Textur“ und „Einheit“; „4“ wählen → Zeile „4 Farben“; zurück auf „16“; (c) „1 Teil übernehmen“ → Erfolgsmeldung, Szene zeigt den Würfel, `call.sh history '{}'` letzter Eintrag `map 0187: model imported`; (d) ⌘Z (echte Taste) → Szene zeigt das Vanilla-Zimmer, `call.sh` zeigt das Paket 20 in alter Größe; (e) Pfad `import/g4_imp_40.png` → Zeile „Texturen“ mit „Neue Textur g4_imp_40: 64 × 64, 16 Farben …“, ohne Feld „Einheit“; (f) Bilder `blick-import-{fehler,wuerfel,png}-{hell,dunkel}.png` unter `target/agent/gate-<datum>/6-5a/`.
  - **(5) Beim Nutzer (Hand und Blick):** Blick auf die vier g4emu-Bilder und die sechs Browser-Bilder; in der gebündelten App „Dateien wählen…“ mit einer eigenen Blender-Datei (`.glb` und `.gltf`) und, wer PDSMS hat, mit einem echten „Export as OBJ with Textures“: Import, Szene, Spiel; `docs/formats/model_import.md` gelesen.
  - **Abweichungen** (Abschnitt): die Liste aus „Abweichungen von Spezifikation und Gutachten“ unten.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und Zahlentabelle:

    | Spiel | Würfel (Byte, Anteil, Vierecke) | Ebene (Byte, Vierecke, Dreiecke) | Paket (angehängt, VRAM, Meldungen) | PNG 40 (Treffer, max. Stufe, Mittel) | zu groß (Byte) | Szenario | Last |
    |---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | |
    | HeartGold DE | | | | | | – | |
    | HeartGold US | | | | | | – | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (4)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(6.5a): …` mit Regressionstest), bis alles grün ist; die Test-Umgebung danach beenden.
- [ ] **Step 3: `agent-gate.md`** Abschnitt „Phase 6.5a“ nach dem Muster „Phase 6.1“/„Phase 6.2“ (Kopien, Worktree-Hinweise mit Ports 1500/1501, „Wo was ist“: `[data-part="model"]` der Prüfliste, Zeile „Umwandeln“, Budget `[data-budget-state]`; Tabelle Punkt / Wer / Wie; die Befehle für (1), (3), (4) mit absoluten Pfaden; erwartete Zahlen). **Roadmap:** „**Stand 6.5a <Datum>:** Gate vom Agenten bestanden: PNG mit 40 Farben → 16-Farben-NSBTX (<Treffer>/40 Palettentreffer ≤ 1 Stufe), Würfel-OBJ <…> B und Ebene-glTF <…> B als Modell der Zimmer, apicula liest sie, zu großes OBJ mit <…> B abgelehnt, OBJ im PDSMS-Stil importiert, `cargo deny` grün mit `tobj` 4.0.5 und `gltf` 1.4.1; `pt-import`, `hg-import`; Branch `phase-6-5a-import`; [docs/phase-6-5a-gate.md](phase-6-5a-gate.md); beim Nutzer offen: Blick, eigene Blender- und PDSMS-Dateien“; Übersicht Zeile 6 um „6.5a Gate vom Agenten bestanden <Datum>“.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`; `git log main..HEAD | grep -c '＠'` gleich 0.
- [ ] **Step 5: Commit** `docs(6.5a): gate of PNG and model import, agent-gate section, roadmap status` (mit Trailer). Kein Merge, kein Push: die Hauptsitzung mergt nach ihrer Abnahme und lässt `scripts/gate.sh` auf main laufen.

---

## Abweichungen von Spezifikation und Gutachten

- **g4emu-Punkt zusätzlich:** Die Gate-Zeile der Spezifikation verlangt „apicula liest sie“; der Auftrag der Hauptsitzung nennt „in g4emu sichtbar“. Der Plan erfüllt beides: apicula als Gate-Punkt (2), g4emu als Zusatzpunkt (3) mit `pt-import`/`hg-import`.
- **Bäcker mit Dreiecken:** Die Entscheidungen (Writer, „Teilmenge des Bäckers“) sagen, der Bäcker brauche keine Dreiecke. Importierte Modelle haben welche (glTF nur Dreiecke, nicht paarbare Reste); Task 1 erweitert den Bäcker um Dreiecke, ohne `Face` zu ändern.
- **Genauigkeit 1 Welteinheit statt `pos_scale`-Wahl:** Gutachten C §6.1 schlägt vor, `pos_scale` je Modell zu wählen (1/64 Einheit Genauigkeit). Der Bäcker hat `POS_SCALE` 64 fest und `Face` (mit 6.3a abgestimmt) ganze Welteinheiten und Texel; Importe werden darum auf 1/16 Kachel und ganze Texel gerundet. Feinere Modelle brauchen später eine `Face` mit Bruchteilen (eigener Posten, nicht 6.5a).
- **Vorschau als Kennzahlen:** „Import-Dialog mit Vorschau“ (Spezifikation §6) ist hier Prüfbericht mit Budget und Texturliste, keine 3D-Ansicht im Dialog; die Szene zeigt das Modell nach dem Import, ⌘Z ist der Rückweg (Regel des Dialogs seit 3b.4b, Gutachten C §6.1 „wie heute bei `map_import::inspect`“).
- **PNG im selben Dialog:** Die Entscheidungen sagen „PNG → NSBTX … in der Oberfläche“ ohne Ort; der Plan nimmt den vorhandenen Dialog „Teile importieren…“ (Teil „Texturen“, Paket des Gebiets der Karte) statt eines eigenen Dialogs am Gebiet.
- **Eigenes Ohrenschneiden statt `earcutr`, kein `meshopt`:** Gutachten C nennt beide als Möglichkeit; wenige Zeilen eigener Code, keine weitere Abhängigkeit.

## Offen für die Umsetzung

- **`map_Kd` mit Optionen** (`map_Kd -s 1 1 1 bild.png`): reicht `tobj` die Optionen im Pfad durch, nimmt der Leser das letzte Wort als Datei und den Rest als Hinweis `model.parts-ignored` `texture-options`; ein Test mit dieser Zeile entscheidet.
- **Reale PDSMS-Exporte:** das Beispiel folgt der öffentlichen Beschreibung; weicht ein echter Export ab (Achsen, Ursprung an einer Ecke statt in der Mitte, Unterordner für Texturen), wird das im Hand-Punkt festgestellt und in `model_import.md` nachgetragen; ein Versatz-Feld in `ImportOptions` kommt erst, wenn ein echter Export es braucht.
- **Namensgleichheit zwischen Modell- und PNG-Teil in einem Aufruf:** fällt erst beim gemeinsamen Anhängen auf und lehnt den Import mit `texture.taken` ab (nichts geschrieben); die Prüfung je Teil bleibt unabhängig.
- **Vanilla-Paket fast voll:** hängt der Würfel an ein Paket mit 255 Einträgen, ist das `texture.pack-full`; die gated Tests drucken die Eintragszahl, falls es auf einer Kopie zutrifft.
- **Würfel außer Sicht im Zimmer:** die Kartenmitte kann hinter dem Spieler liegen; dann bleibt das Bild, wie es ist, der Kopfkommentar sagt es, und der Blick des Nutzers entscheidet. `unit` und Lage werden nicht für das Bild verändert.
- **Parallele Merges:** `PartCode::ALL` (Länge im Typ), `map_import.rs`, `dispatch.rs`, `api.import.ts`, `importParts.ts`, `Cargo.toml`/`Cargo.lock`, `ROADMAP.md`, `agent-gate.md`, `design-system/README.md` werden Konflikte an Anhängepunkten mit 6.3a/6.3b; beide Seiten behalten, `Cargo.lock` nach dem Merge mit `cargo build` neu erzeugen und `cargo deny` erneut laufen lassen.
