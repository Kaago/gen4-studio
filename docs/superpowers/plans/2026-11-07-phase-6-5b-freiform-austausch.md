# Phase 6.5b „Freiform und Austausch“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eine Welt-Karte trägt glTF-Freiform (`world.freeform[] = { glb, at }`) und Teilmodelle mit echter Geometrie (Tileset-Kachel `model` → Beifile), beides backt zusammen mit dem gemalten Raster in ein Modell und ist in der Szene und in g4emu zu sehen; „Karte als Quelle übernehmen“ bekommt Form 2 (Vanilla-Modell über apicula als Freiform-GLB); eine Welt-Karte geht als Tiled-Karte (TMJ) hinaus und herein, und ein Export, unverändert wieder importiert, ergibt dieselbe Karten-JSON byte-gleich.

**Architecture:** Neues Kernel-Modul `world/freeform.rs`: das **kanonische Beifile** (ein GLB aus Bäcker-Flächen, Materialname = Texturname, keine Bilder), Laden der Beifiles einer Welt, `world_freeform_inspect`/`_add`/`_remove` (Paket, Beifile und Welt als ein Undo-Schritt) und `world_faces` (Netz für die Szene). `world/bake.rs` nimmt die geladenen Beifile-Flächen über `BakeEnv` (bleibt rein), `world/mesh.rs` setzt Teilmodell-Geometrie statt der Fußabdruck-Fläche ein, `world/adopt.rs` bekommt Form 2 über `world_adopt_with` (apicula → `model_import::read_model` → kanonisches Beifile), `world/tileset.rs` `tileset_piece_set`. Neues Modul `world/tmj.rs`: Abbildung Weltdokument ↔ Tiled JSON mit eigenem serde-Leser und -Schreiber, Import als ein Undo-Schritt über `Tx::edit_world`. Das Textprojekt behält referenzierte Beifiles beim Schreiben und Exportieren; der Eingabe-Hash des Schritts `world` nimmt sie auf. Studio: sieben Befehle über `g4_call`, minimale Oberfläche in den vorhandenen Dialogen.

**Tech Stack:** Rust (`g4-kernel`: `model_import` aus 6.5a, `bake`, `export::nsbmd_to_glb`, `serde`/`serde_json`, `image` 0.25 (PNG schreiben, vorhanden), `base64` 0.22 (vorhanden seit 6.5a), `proptest`), `g4-formats` (`nsbmd::Nsbmd`, `nsbtx`), Tauri-Befehle und Brücke (MCP über `g4_call`), React 19 + Vitest, Python/uv für `scripts/g4emu`, apicula als Orakel und Exportweg. **Keine neue Abhängigkeit.**

**Spec:** [2026-11-05-phase-6-kartenerstellung-design.md](../specs/2026-11-05-phase-6-kartenerstellung-design.md) §1 (harte Grenzen, „Nicht in Phase 6“), §3.1 (`freeform`), §3.2 (Teilmodelle als Beifiles), §3.3, §3.5 („Form 2 als glTF über apicula“), §3.6 („TMJ nur als Austausch (6.5b)“), §3.7, §4 Zeile 6.5b, §5, §7, §8; [entscheidungen.md](../../research/2026-11-05-phase-6/entscheidungen.md) Block „Malen, Weltformat, Import, Tiled“ (Tiled T2 mit TMJ-Austausch, Weltformat, „Karte als Quelle übernehmen“, Git LFS → `.gitattributes`, Import, „Vanilla-Teilmodelle herauslösen“ nicht), „Nicht in Phase 6“; [Gutachten C](../../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §4.2 (Abbildung Tiled ↔ Gen 4), §5.4 (TMJ nicht als Weltformat), §5.5 (`.gitattributes`), §6.1 (glTF, Achsen, Einheit, `tiled` 0.16), §12 (Lizenzen); Vorgänger-Pläne [6.3a](2026-11-05-phase-6-3a-weltformat.md), [6.3b](2026-11-06-phase-6-3b-backen-build.md), [6.5a](2026-11-06-phase-6-5a-import.md) (Form und Schnittstellen).

Stand 07.11.2026: geschrieben, als 6.3b und 6.5a noch in Worktrees lagen; beide sind inzwischen in `main` gemergt, `main` bei `53210d31` enthält 6.1, 6.2, 6.3a, 6.3b und 6.5a. **Basis ist `main` nach dem Merge von 6.3b und 6.5a**: `main` bei `53210d31`. Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-5b`, Branch `phase-6-5b-freiform-austausch` (`git worktree add .claude/worktrees/phase-6-5b -b phase-6-5b-freiform-austausch <commit>`). Kernel `crates/g4-kernel/src`, Formate `crates/g4-formats/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`, Emulator `scripts/g4emu`.

## Voraussetzungen (auf main nach 6.3b und 6.5a, vor Task 1 mit grep prüfen)

„bestätigt 53210d31“ = auf `main` bei `53210d31` im Code gelesen (Prüfung vor Task 1); sonst steht die Abweichung mit echtem Namen, echter Signatur und Pfad da. Die Anpassung je Task steht in `.superpowers/sdd/2026-11-07-phase-6-5b-freiform-austausch/prereq.md`.

| Schnittstelle | erwartet | Stand | gebraucht in |
|---|---|---|---|
| `world::bake::{Verbatim::of, BakeEnv {map, tileset, input, family}, GridBake {faces, model, bdhc, permissions, messages, verbatim_model, verbatim_terrain}, WorldBakeReport, WorldBakeMessage {code, level, args, message}, bake_grid, assemble, report, bake_world, baker_text, world_permissions, WORLD_BAKE_REV = 1, FREEFORM_LATER_CODE, VERBATIM_MIXED_CODE, NO_AREA_CODE}` | `bake_grid` prüft `glb` zuerst (`world.freeform-later`), mischt Verbatim und Flächen nicht | bestätigt 53210d31, mit: `bake_grid(grid, world, verbatim, env)` (4 Argumente); `GridBake.faces: Vec<MeshFace>`; `bake_world` bindet `world_pack_textures(pack, &tileset)` (Palette aus dem Tileset), nicht `pack_textures`; die Codes sind `pub const` in `world/bake.rs`, in `world.rs` nicht re-exportiert; `WorldBakeMessage::error` privat (Felder `pub`); `synthetic_pack_textures()` liegt in `world::bake` (test-support); `verbatim-mixed` heute nur bei Verbatim-Modell **und** Rasterflächen | 2 |
| `world::doc::{WorldJson {version, tileset, layers, pieces, freeform, terrain, permissions}, FreeformJson {glb: Option<String>, nsbmd: Option<Part>, at: [i32; 3]}, TerrainJson, WORLD_VERSION = 2, TILE_UNITS, STEP_UNITS, fixtures::{gate_world, one_tile_world, checkerboard_world}}`; `world::grid::WorldGrid::parse` mit den `freeform`-Regeln (genau eines von `glb`/`nsbmd`) | | bestätigt 53210d31, mit: `fixtures::one_tile_world()` ohne Argument (Paket 0), `gate_world(tileset, ground, grass, water)`, dazu `busy_world`, `oversize_world`; `PermissionsJson.overrides` (JSON-Schlüssel `override`); `PieceJson.rotate: u16` in Grad; `WorldGrid` hat kein `freeform`-Feld; `at` eines `glb`-Eintrags ist heute unbegrenzt; vorhandene Tests nutzen `glb` = `a.glb`/`b.glb`/`haus.glb` (grid.rs 769/779/897, keep.rs 496, doc.rs 352, bake/tests.rs 607) | 1, 6, 7 |
| `world::mesh::{MeshFace {corners, uv, texture, layer}, mesh, mesh_textures, PIECE_LAYER = 255, MODEL_BUDGET, footprint_of (pub(crate))}`; `From<MeshFace> for bake::Face` | | bestätigt 53210d31, mit: `uv_of` ist privat (`fn uv_of(x, z, rotate: u8, flip)`), Task 6 braucht `pub(crate)`; `footprints` privat; Kachel 5 des synthetischen Tilesets (`syn_stueck`) hat Fußabdruck `[2, 2]`, nicht `[3, 1]` | 2 |
| `bake::{Face {corners, uv, texture, layer}, Face::is_triangle, bake, BakeInput, BakeReport {triangles, …}, BakeCode::{TextureMissing, FaceOutOfRange, …}, PackTexture {name, palette, width, height}, pack_textures, append_textures (store, pack, textures, label) -> AppendReport, decode_quads, BAKE_REV, WORLD_MIN, WORLD_MAX}` | `decode_quads` liest nach dem Merge auch Dreiecke (6.3b-Plan „Parallele Merges“) | bestätigt 53210d31 (beide gemergt): `decode_quads` liest BEGIN-Dreiecke als `[a, b, c, c]` (decode.rs 193); `WORLD_MIN`/`WORLD_MAX` sind `pub(crate)`; `append_textures(store, pack, &[NewTexture], label) -> Result<AppendReport, KernelError>`; `appended_pack_or_refusal(before, &[NewTexture]) -> Result<Result<Vec<u8>, BakeMessage>, KernelError>` (`pub(crate)`, re-exportiert aus `bake`); `BakeReport {size, polys, verts, strips, single_quads, triangles, …}` | 1, 2, 4, 10 |
| `model_import::{read_model, ImportOptions {unit, colors}, ImportOptions::check, SourceFormat, plan_textures, TexturePlan {bindings, new, previews, notes}, TexturePreview, ModelRefusal {reason, notes}, mesh::{Mesh, MeshMaterial, Binding, MeshStats, Converted, to_faces}, samples::{write_samples, CUBE_OBJ, PLANE_GLB, PLANE_GLTF}}`; `read_gltf`: Material ohne `baseColorTexture` → `MeshMaterial { name, image: None }`; `plan_textures` bindet ein Material ohne Bild an die Paket-Textur seines Namens | | bestätigt 53210d31, mit: `plan_textures(materials, pack, colors: Colors)` (aus `ImportOptions::texconv_colors()`, nicht `u8`); `read_model(path) -> Result<(Mesh, Vec<PartMessage>), PartMessage>`; `to_faces(mesh, unit: f32, bindings) -> Result<Converted, PartMessage>`; `MeshStats {polygons, quads, triangles, dropped}`; `MeshMaterial.image: Option<MeshImage {stem, file, rgba}>`; `read_gltf` ist privates Modul; `pair_triangles` `pub(crate)`; Samples dazu `BIG_OBJ`, `PDSMS_OBJ`, `PNG_40`; Würfel: Material `g4_imp_cube_mat`, Textur `g4_imp_cube` 16 × 16; Ebene: Material `g4_imp_plane_mat` | 1, 4, 5 |
| `map_import::{PartMessage, PartCode}`; `export::{nsbmd_to_glb, embed_images}`, privat `export::write_glb(json, bin)` | `nsbmd_to_glb(apicula, nsbmd, nsbtx, out_dir) -> PathBuf`; apicula legt die Texturen als `<textur>.png` neben das GLB | bestätigt 53210d31, mit: `nsbmd_to_glb(apicula, nsbmd, nsbtx: Option<&[u8]>, out_dir) -> Result<PathBuf, KernelError>`, das GLB liegt in einem frischen Unterordner `<out_dir>/…/converted/`; `write_glb(json: &Value, bin: &[u8]) -> Vec<u8>` privat (mit `expect`); ob apicula `<textur>.png` neben das GLB legt, steht nicht im Code (nur `embed_images` für `uri`-Bilder): Probe Task 5 Step 1 | 1, 5 |
| `g4_formats::nsbmd::{Nsbmd::read, Model {materials, textures: Vec<Pairing {name, materials}>, geometry()}, Geometry {verts, polys, tris, quads}}` | | bestätigt 53210d31, mit: `Nsbmd.models: Vec<(Name, Model)>`, `Pairing {name: Name, materials: Vec<u8>, bound}` (`Name`, kein `String`), `Model.materials: Vec<Material {name: Name, …}>`; `Geometry` hat keine Bytes (die nimmt `nsbmd.len()`) | 5 |
| `world::tileset::{Tileset, TileJson {…, footprint, model: Option<String>}, tileset_problems` („Teilmodell-Dateien kommen mit 6.5b.“), `tileset_texture, tile_outside, tileset_rel, TILESETS_DIR}` | `model` ist reserviert | bestätigt 53210d31, mit: `tileset_texture(project, tileset, texture) -> Result<Option<Rgba>, KernelError>`; der Satz steht in `tileset_problems` und im Test `tileset_problems_name_each_rule` (tileset.rs 1187, 1245); **`check_folder` meldet jede Datei in `data/tilesets/`, die nicht `NNNN.json` heißt, als `textproject.schema`** (trifft `PPPP.<name>.glb`); `TileChange.wall` nutzt `deserialize_with = "present"` | 3, 4 |
| Store: `Store::{transaction, world, tileset, undo_depth, squash_since, rollback_to, record_file_step, busy}`, `Tx::edit_world(map, FnOnce(&mut Option<WorldJson>))`, `FileStep {asset, files: Vec<FileChange {path, before, after}>, rotom_entry, kind: FileStepKind::Reset}`, `write_file_step` | | bestätigt 53210d31, mit: `Tx::edit_world(map, impl FnOnce(&mut Option<WorldJson>) -> Result<(), KernelError>)`; `Store::world(map) -> Result<Option<&WorldJson>, _>`; `Store::tileset(p) -> Result<Arc<Tileset>, _>`; `squash_since(mark, label)` ohne Rückgabe; `rollback_to(mark) -> Result<usize, _>`; `record_file_step(label, step) -> ChangeId`; `write_file_step(project, step)` ist `pub(crate)` in `store.rs`; `FileStep.rotom_entry: Option<RotomEntryChange>`; `Tx::bake_report(map)` | 4, 5, 7 |
| Store backt: `Tx::edit_world` backt im selben Schritt, `Store::world_bake_report`, `Store::forget_world_bakes(package)`, `WorldReport.bake`, `world_derived_refusal` | | bestätigt 53210d31: `Tx::edit_world` backt im selben Schritt (`bake_world` über die Map des Schritts, Bericht `Tx::bake_report`); `Store::world_bake_report(map) -> Result<Option<WorldBakeReport>, _>`; `Store::forget_world_bakes(package)` (läuft auch aus `record_file_step` für Asset `MapTexture` und Pfade `data/tilesets/NNNN.json`, **nicht** für `PPPP.<name>.glb`); `WorldReport.bake: Option<WorldBakeReport>`; `world_derived_refusal(map)` | 2, 4 |
| `world::record::{input_hash, world_salt}`, `world::build::{bake_world_maps, WorldStep}` | Salz enthält `WORLD_BAKE_REV` | bestätigt 53210d31, mit: `input_hash(store, map) -> Result<Option<String>, _>` über `feed(h, label, Option<&[u8]>)` (`None` = fehlt); `world_salt()` = `world#1+bake#1+bdhc#1`, der Test `record/tests.rs:32` hält den Wortlaut; `world::build` exportiert `bake_world_maps, WorldBuild, BakeMode, BakeScope, WORLD_STEP_ID`, `WorldStep` ist `pub(crate)` | 2 |
| `world::adopt::{world_adopt, WorldAdoptRequest {map, tileset?, form}, AdoptForm::{Verbatim, Gltf}, WorldAdoptReport {map, tileset, tileset_guessed, deviations, change, bake}, ADOPT_TOLERANCE, adopt_label}`; `Gltf` lehnt mit „Form 2 (glTF) kommt mit 6.5b.“ ab | | bestätigt 53210d31, mit: `WorldAdoptReport.bake: WorldBakeReport` (kein `Option`); `AdoptForm` nur `Deserialize`; Satz als `ADOPT_GLTF_LATER`, gehalten von `adopt/tests.rs::adopt_refuses_dspre_world_locked_and_gltf` und `src-tauri/…/world_map.rs:803`; der Form-1-Byte-Test heißt `adopt_bakes_byte_equal` | 5 |
| `world::tileset::{tileset_edit, TileChange, TilesetEditRequest}` | | bestätigt 53210d31, mit: `tileset_edit(store, &TilesetEditRequest {package, changes}) -> Result<TilesetEditReport {package, tiles, change}, _>`; Schritt ist ein Dateischritt mit Asset `HEADER_TABLE`; `tileset_file_missing(package)` | 3, 4 |
| Textprojekt: `textproject/source/writers.rs` löscht beim Schreiben einer Entität jede Datei `NNNN.*` im Ordner, die nicht in `files` steht; `textproject/export.rs` löscht unbekannte Dateien im Ordner | Ursache von Task 3 | bestätigt 53210d31: `put_files` (writers.rs 76–104) und `write_folder` (export.rs 677–718, mit `world_map_files`/`carry_world_parts`); `data/tilesets/` schreibt und löscht keiner der beiden (Dateiart `FILE_KINDS`) | 3 |
| `world::keep::world_parts`, `EntitySource::read_world_map`, Plan `world_maps` | | bestätigt 53210d31: `world::keep::world_parts` (`pub fn`), `EntitySource::read_world_map` (`pub(crate)`, `textproject/source/mod.rs:418`) | 3 |
| Tauri: `commands/world_map.rs` (`get_world_mesh_impl`, `get_tileset_texture_impl`, `world_adopt_impl`, `WorldGetDto.bake`), `commands/models.rs::find_apicula`, `dispatch.rs` (`COMMANDS`, `confine_source`), `mcp/access.rs` (`COMMAND_KINDS`, `Kind::Depends` für `export_map_part` mit Schlüssel `path`), `mcp/docs.rs::FORMATS` (Test `table_covers_every_format_file`) | `world_adopt_impl` aus Plan 6.3b Task 8 | teils 53210d31: `get_world_mesh_impl` ruft `mesh` nicht selbst, sondern über den Helfer `faces_of` (auch von `world_get_impl` genutzt); `MeshTextureDto` nimmt Größe und Rolle nur aus dem Tileset; `world_adopt_impl` vorhanden (ruft `world_adopt`); `find_apicula()` ist `pub(super)` in `commands/models.rs` (`Option<PathBuf>`); `confine_source(policy, Source)` gilt nur für `map_import::Source`, ein einzelner Pfad geht über `policy.read_path`/`write_path`; `Depends` hat keine Pfadliste, sondern den Arm `"build_rom" ∣ "export_map_part"` in `access.rs::classify` | 8 |
| Frontend: `workbench/inspector/{ImportPartsDialog.tsx, ExportPartDialog.tsx, importParts.ts, WorldSourceSection.tsx}`, `api.world.ts`, `api.import.ts` | `WorldSourceSection` aus Plan 6.3b Task 10 | bestätigt 53210d31, mit: `worldAdopt(map)` sendet `{req: {map}}`; `ImportPartsDialog` kennt keine Welt-Karten (`data-import-dialog={map}`); `ExportPartDialog` hat kein `data-export-part`; `WorldSourceSection` hat `data-world-adopt=""`; `PART_TEXTS` in `importParts.ts` muss genau `PartCode::ALL` sein (Rust-Test `every_part_code_has_its_german_sentence`) | 9 |
| `scripts/agent/scenarios/{pt-gemalt,hg-gemalt}.yaml`, g4emu-Schritte `make-import-samples`, `shot`, `expect-at` | `pt-gemalt` aus Plan 6.3b Task 11 | bestätigt 53210d31: `pt-gemalt.yaml`/`hg-gemalt.yaml`; `shot`, `expect-at`, `clock` in `scripts/g4emu/g4emu/steps.py`; `make-import-samples` ist ein `prepare`-Schritt (`scenario.py`, Argument `path`) | 11 |
| `crates/g4-kernel/tests/common/glb.rs` (`glb_chunks`, `accessor`, `input_set`, `glb_set`), `tests/bake.rs::apicula()` | | bestätigt 53210d31, mit: `apicula()` ist eine private Funktion in `tests/bake.rs` (für `tests/world_freeform.rs` kopieren oder nach `tests/common/` ziehen) | 10 |
| `crates/g4-kernel/metadata/git/gitattributes` (Vorlage der Projekte, `* -text`, LFS-Kommentar), Repo-`.gitattributes` (`* text=auto eol=lf`) | weder `*.png binary` noch `*.glb binary` | bestätigt 53210d31, mit: der Test `git/templates.rs::attributes_mark_everything_binary_and_text_sources_lf` hält die Regelliste und den Kommentar „# Platz für große Binärquellen (Phase 6)“ wörtlich | 1 |
| `test_support::{world_ready, TextFixture::exported, vanilla_copy}`, `world::doc::fixtures::oversize_world`, `model_import::samples::BIG_OBJ`, `g4_formats::texconv::palette_name` | | bestätigt 53210d31 | 1–5, 10 |
| Gebäude für den TMJ über `Store::map(map) -> &MapFile` | Plan nennt `MapJson.buildings`/`BuildingJson` | Abweichung: `MapFile.buildings: Vec<g4_formats::map::Building {model_id, position: [i32; 3] (fx32), …}>`; `BuildingJson` gibt es nur im Textprojekt-Schema | 6, 7 |
| Abgleich `textproject::sync`: die Einheit einer Welt-Karte folgt `world_side` (`input_hash`) | | bestätigt 53210d31; ein Beifile aus einem Dateischritt wird erst mit Task 2 Teil des Hashes: Test „Hinzufügen, Speichern, ⌘Z → Abgleich ohne Einheit“ in Task 4 aufnehmen | 2, 4 |
| Alte Sätze und Codes außerhalb der Dateilisten des Plans | | `world.freeform-later` in `docs/mcp.md:483` und `docs/formats/text_project.md:1304`; „Form 2 (glTF) kommt mit 6.5b.“ in `docs/mcp.md:490`; Salz-Wortlaut in `docs/formats/build_state.md:135` und `world_map.md:233` | 2, 5, 12 |

Fehlt eine Zeile oder weicht sie ab, zuerst die Abweichung in diese Tabelle schreiben und den betroffenen Task anpassen, dann beginnen.

## Parallele Teilphase 6.4a und Anhängepunkte

6.4a „Malen: Werkzeuge und Rezept“ läuft gleichzeitig in einem eigenen Worktree (Malwerkzeuge in der Szene, Palette, Ebenen, Rezept `new_painted_map`, kuratierte MCP-Werkzeuge `world_get`/`world_edit`/`tileset_get`, Szenarien `pt-gemalt`/`hg-gemalt` über die Oberfläche).

| Bereich | gehört | Regel |
|---|---|---|
| `world/bake.rs`, `world/adopt.rs`, `world/tileset.rs`, `world/mesh.rs`, `world/record.rs`, neu `world/freeform.rs`, `world/tmj.rs`, `model_import/`, `textproject/source/writers.rs`, `textproject/export.rs`, `export.rs` (`write_glb` → `pub(crate)`) | 6.5b | 6.4a ändert dort nichts; braucht 6.4a etwas, hängt es an |
| `recipes/`, `world/edit.rs`, Szene und Werkzeuge im Frontend (`apps/studio/src/world/**`), `mcp/tools.rs` | 6.4a | 6.5b ändert dort nichts; die TMJ-Import- und Freiform-Schritte gehen über `Tx::edit_world` direkt, nicht über `world_edit` |
| `commands/dispatch.rs` (`COMMANDS`, Arme), `mcp/access.rs`, `src-tauri/src/lib.rs` (`generate_handler!`), `commands/mod.rs`, `world.rs` (`pub mod`, `pub use`), `api.world.ts`, `docs/mcp.md`, `docs/agent-gate.md`, `docs/ROADMAP.md`, `design-system/README.md`, `mcp/docs.rs` (`FORMATS`, alphabetisch) | beide | nur anhängen, eigene Blöcke am Ende |
| `commands/world_map.rs` | beide | 6.5b ändert genau eine Zeile in `get_world_mesh_impl` (`mesh` → `world_faces`, Task 8) und hängt eigene `*_impl` am Ende an; beim Merge die Zeile von 6.5b nehmen |
| `world/mesh.rs` `mesh(grid, tileset)` | 6.5b | Signatur bleibt (6.4a ruft sie für die Live-Vorschau); neu `mesh_with` daneben |
| `docs/formats/world_map.md`, `docs/formats/tileset.md` | beide | 6.5b schreibt eigene Abschnitte („Freiform und Beifiles“, „Teilmodelle mit Geometrie“, „Form 2“), 6.4a ändert sie nicht um |
| `.fig` | – | keine Seite fasst sie an (Nachtrag als Anleitung) |

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Form des Beifiles | **Kanonisches GLB aus Bäcker-Flächen**: je `bake::Face` ein Dreieck (`is_triangle`) oder zwei (Ecken 0 1 2 und 0 2 3); Positionen in Kacheln (Welteinheit / `FREEFORM_UNIT` 16), UV als Bruchteil der gebundenen Paket-Textur (t nach unten wie `model_import`), eine Primitive je Textur, ein Material je Textur **ohne Bild**, Materialname = Texturname des Pakets; `asset.generator` „gen4-studio“; ohne Knoten-Transformation. Lesen über `read_model` + `to_faces(mesh, 16, bindings)` mit Bindungen nur aus dem Paket. Eigenschaft: `side_faces(side_glb(f)) ≅ f` (bis auf zyklische Drehung der Ecken) und `side_glb` ist ein Fixpunkt nach einem Durchlauf | Backen zeigt genau, was beim Hinzufügen geprüft wurde; deterministisch und klein; die Texturen stehen einmal im Paket statt doppelt im Beifile; Blender-, OBJ- und apicula-Eingaben enden in derselben Form; `pair_triangles` stellt die Vierecke wieder her |
| Einheit und Achsen | Eingabe: `options: ImportOptions` (Vorgabe `unit` 16, 1 Modelleinheit = eine Kachel; `colors` 16) **nur beim Hinzufügen**; im Beifile fest `FREEFORM_UNIT = 16.0` (1 glTF-Einheit = 1 m = eine Kachel). Achsen wie `model_import`: Y oben, X Osten, Z Süden, Ursprung Kartenmitte. `at` in Welteinheiten, je Achse −512 bis 511 (`world.schema`: „`at` liegt nicht in −512 bis 511.“), wird auf jede Ecke addiert. **Kein `unit`-Schlüssel** in der Welt-JSON | `WorldJson` bleibt `Eq` und Schema `maps.world` 2; Maßstab ist eine Frage der Eingabedatei, nicht der Karte |
| `at` außerhalb | Ecken nach dem Verschieben außerhalb `WORLD_MIN..=WORLD_MAX` lehnt der Bäcker mit `bake.face-out-of-range` ab (vorhandener Code); Ecken außerhalb der Karte (±256), aber im Bereich, sind erlaubt (ragt in die Nachbarkarte wie Vanilla-Kulisse) | eine Regel, die der Bäcker schon hat |
| OBJ als Freiform | **als Eingabe ja** (`world_freeform_add` liest OBJ und glTF über `read_model`), **im Weltformat nur `.glb`**: `glb` nennt immer ein kanonisches Beifile | die Freiform-Zeile im Weltformat bleibt eindeutig; PDSMS-OBJ bleibt nutzbar |
| Beifile-Namen | Freiform `data/maps/NNNN.<name>.glb`, `glb` ist der Dateiname (`"0123.haus.glb"`), `NNNN` = Karte, `<name>` aus `[a-z0-9_-]{1,32}` (`world.schema`: „`glb` nennt ein Beifile dieser Karte: `0123.<name>.glb` mit a–z, 0–9, `_`, `-`.“). Vorgabe aus dem Dateistamm (`side_name`: klein, jedes andere Zeichen `_`, gekürzt, leer → `freiform`, vergeben → `_2`, `_3` …). Form 2: `NNNN.model.glb`. Teilmodell: `data/tilesets/PPPP.<name>.glb`, `TileJson.model = "<name>"` | Spezifikation §3.2 „Beifiles `NNNN.<name>.glb`“; eindeutig je Karte bzw. Paket |
| Ebenen der Flächen | Freiform `FREEFORM_LAYER = 254`, Teilmodell-Geometrie `PIECE_LAYER = 255` (wie die Fußabdrücke) | eigene Läufe im Bäcker (keine Strips über Raster und Freiform), über allen gemalten Ebenen; 6.3a reserviert 255 |
| Wände und Kollision | Freiform und Teilmodelle sind für **BDHC und Permissions Kulisse**: BDHC aus dem Raster (oder verbatim), Permissions aus Raster, Fußabdruck und Überschreibung; keine Wände, keine Höhe aus Polygonen | Höhen aus Polygonen wären „Vanilla-NSBMD zurück ins Raster“ durch die Hintertür; der Nutzer malt Höhe und Kollision darunter |
| Texturen | **Backen ändert kein Paket.** Neue Texturen hängt `world_freeform_add` bzw. `tileset_piece_set` an (`append_textures`, Paket = `world.tileset` bzw. `package`) im selben Undo-Schritt wie Beifile und Welt; fehlt beim Backen eine Textur eines Beifiles im Paket: `bake.texture-missing` (Text von `baker_text`) | Spezifikation §7 „Geteilte Pakete: Anhängen ist unschädlich“; ⌘Z nimmt alles zusammen |
| Beifile fehlt oder liest nicht | `world.side-file` (Fehler, Argumente `[pfad, grund]`): „Das Beifile data/maps/0123.haus.glb fehlt.“ bzw. „Das Beifile data/maps/0123.haus.glb lässt sich nicht lesen ({grund}).“; das Modell wird abgelehnt (letztes Modell bleibt, wie jede Ablehnung in 6.3b); die Szene zeigt das Raster ohne diese Freiform, ein Teilmodell ohne lesbares Beifile als Fußabdruck | §7 „Ablehnen beim Backen, nie kürzen“ |
| Teilmodell-Lage | Geometrie des Beifiles im Modellraum, Ursprung = Mitte des **gedrehten** Fußabdrucks auf der Höhe der Fußabdruck-Fläche (höchste Bodenecke darunter); `rotate` im Uhrzeigersinn von oben, je 90° `(x, z) → (−z, x)`; Ebene 255. Mit Modell ersetzt die Geometrie die Fußabdruck-Fläche; ohne `model` bleibt die Fläche. Permissions bleiben über den Fußabdruck | Spezifikation §3.3 (Vernetzer kennt Teilmodelle), 6.3a „bis 6.5b die Geometrie liefert“; Drehsinn wie die Fußabdruck-Vertauschung |
| Verbatim und Freiform | Ein Verbatim-Modell (Form 1) mit Kacheln **oder** Freiform bleibt `world.verbatim-mixed`, neuer Text: „Ein Verbatim-Modell (Form 1) backt nur allein; für Kacheln oder Freiform daneben die Karte als binäre Karte übernehmen und mit Form 2 (glTF) neu übernehmen.“ `world.freeform-later` entfällt | „Bearbeiten von Vanilla-Modellen über den Codec“ ist nicht in Phase 6; Form 2 ist der Weg |
| Form 2 | `world_adopt {req: {map, form: "gltf"}}` wie Form 1 (Höhen abgetastet, `terrain` verbatim, Permissions als Überschreibung aller Zellen, Gebäude und `bgs` 1:1, Raster leer), nur das Modell ist `freeform = [{glb: "NNNN.model.glb", at: [0, 0, 0]}]`. Weg: `nsbmd_to_glb(apicula, model, Some(paket))` in `.g4/tmp/adopt-NNNN` (scheitert apicula mit Paket: ohne Paket), `read_model`, Bindung je Material (Regel unten), `to_faces(mesh, APICULA_UNIT = 1.0, …)`, kanonisches Beifile. Scheitert das Backen (z. B. `bake.model-too-big`): **kein Schritt**, Ablehnung mit dem Code. Kernel `world_adopt_with(store, req, apicula: Option<&Path>)`; ohne apicula „Form 2 braucht apicula (`G4_APICULA` oder tools/apicula).“; `world_adopt` = `world_adopt_with(…, None)` | Spezifikation §3.5; terrain verbatim hält die Karte begehbar wie vorher; apicula schreibt Welteinheiten (Orakel `glb_set` von 6.1 vergleicht GLB-Positionen direkt mit den Ecken) |
| Texturnamen Form 2 | Je glTF-Material: (1) MDL0-Material gleichen Namens im Vanilla-Modell → Name der Textur, die es bindet (`Model.textures[*].materials`); (2) sonst Bildstamm des Materials (apicula schreibt `<textur>.png`); der Name muss in `pack_textures` des Pakets stehen, sonst Ablehnung `world.adopt-texture` („Material „{m}“ der Karte 0123 bindet keine Textur des Pakets 0006; Form 2 geht für diese Karte nicht, Form 1 schon.“). Im Beifile heißt das Material dann wie die Textur | Regel „Materialname = Texturname“ des Beifiles; (1) hängt nicht davon ab, wie apicula Bilder benennt |
| Polygonzahl Form 2 | `WorldAdoptReport.form2: Option<Form2Report>` mit Vanilla (`Geometry`: Polygone, Vierecke, Dreiecke, Bytes) und gebacken (Flächen, Vierecke, Dreiecke, Streifen, Bytes, weggefallene Flächen, `model.parts-ignored`); Abweichung steht im Bericht, in der Gate-Datei und in `world_map.md` | Gate „Abweichung in der Polygonzahl dokumentiert“ |
| TMJ-Leser | **eigener serde-Leser** (`world/tmj.rs`), kein Crate. Gelesen werden nur Felder der eigenen Abbildung; unbekannte Felder (Tiled schreibt `class`, `parallaxx`, `tintcolor` …) werden ignoriert | `tiled` 0.16.0 (MIT) brächte XML-Leser, Dateilader und eigene Typen für ein Format, von dem wir eine feste Teilmenge schreiben; keine Abhängigkeit, kein Lizenz-Audit nötig |
| TMJ-Schreiben | `serde_json::to_vec_pretty` + `\n`, Schlüsselreihenfolge fest (Structs), `version` „1.10“, `tiledversion` „1.10.2“, `type` „map“, `orientation` „orthogonal“, `renderorder` „right-down“, 32 × 32, Kachel 16 × 16, `infinite: false`, `compressionlevel` −1; Karten-Eigenschaften `g4.format` = „gen4-studio-world“, `g4.version` = 1 (`TMJ_VERSION`), `g4.map`, `g4.tileset`, `g4.derive` | dokumentiertes TMJ-Format (doc.mapeditor.org), Tiled öffnet es |
| Ebenen im TMJ | je Welt-Ebene eine `tilelayer` (Name = Ebenenname, Eigenschaft `g4.role` `ground`/`overlay`); `tilelayer` „Höhe“ (`g4.kind` `heights`, Tileset „g4 Höhen“, 79 Kacheln für −16 bis 62, GID = erster + (h + 16)); `tilelayer` „Rampen“ (`g4.kind` `ramps`) **nur, wenn `ramps` da ist** (Tileset „g4 Rampen“, 8 Kacheln `n1 n2 e1 e2 s1 s2 w1 w2`); `objectgroup` „Teilmodelle“ (`g4.kind` `pieces`, Kachel-Objekte `gid` = Kachel + 1, `x = at.x · 16`, `y = (at.z + 1) · 16`, 16 × 16, Eigenschaft `g4.rotate`); `objectgroup` „Permissions“ (`g4.kind` `overrides`) **nur, wenn `override` da ist**, je Überschreibung ein Rechteck 16 × 16 auf der Zelle mit `g4.kind`, `g4.collision` (Zahlen); `objectgroup` „Gebäude (nur Ansicht)“ (`g4.kind` `buildings`, Punkte bei `position / 4096 + 256` mit `g4.model_id`) | Gutachten C §4.2 (Höhe als eigene Ebene, Gebäude als Punkt-Objekt); „da“/„nicht da“ spiegelt `Option` und hält den Round-Trip byte-gleich |
| Drehen und Spiegeln | Tiled-Bits `H = 0x8000_0000`, `V = 0x4000_0000`, `D = 0x2000_0000`: `r0` –, `r1` D·H, `r2` H·V, `r3` D·V, `f` H, `r1f` D·H·V, `r2f` V, `r3f` D (abgeleitet aus `mesh::uv_of`: Bild → Zelle = Umkehrung der UV-Abbildung, Tiled wendet D, dann H, dann V an); Bit `0x1000_0000` (Sechseck) → Ablehnung | alle acht Fälle verschieden; ein Test hält jede Zeile gegen `uv_of` |
| Tileset im TMJ | eingebettete Bildsammlung (`columns` 0, `grid` 16 × 16) je Tileset, `tiles[i] = {id, image, imagewidth, imageheight, properties: [g4.texture, g4.role]}`; die Bilder schreibt der Export als PNG in den Ordner `<stamm>.tiles/` neben der Datei (`tile-0000.png` …, `height-p012.png`/`height-m016.png`, `ramp-n1.png` …): Kartenkacheln aus `tileset_texture`, ohne Dekodierung ein 16 × 16-Platzhalter in der Rollenfarbe, Höhen als cividis-Stufen, Rampen als Farbe je Richtung. Der Import **liest die Bilder nie** | Tiled zeigt die Kacheln; die Daten bleiben die Indizes |
| TMJ-Import | nur auf eine Welt-Karte (sonst `not_a_world_map`); Paket des TMJ (`g4.tileset`) = `world.tileset` („Die Datei nutzt Paket 0006, die Welt Paket 0007.“); erstes Tileset `firstgid` 1 mit `tilecount` = Kacheln des Tilesets; Karte 32 × 32, Kachel 16, nicht `infinite`; Daten als Liste oder Base64 **ohne** Kompression (`compression` gesetzt → „Komprimierte Ebenen liest das Studio nicht; in Tiled unter Karte › Eigenschaften „Kachelebenenformat“ CSV wählen.“); externes Tileset (`source`) → Ablehnung; eine neue `tilelayer` ohne `g4.*` wird Überlagerung (die erste Ebene ist der Boden); Objekte nicht auf dem Raster → Ablehnung mit Objekt-ID. Ergebnis: `layers`, `pieces`, `permissions` aus der Datei, `version`, `tileset`, `freeform`, `terrain` aus der Ziel-Welt; dann `WorldGrid::parse` (Fehler als `world.schema` mit Pfad). **Gebäude ignoriert** (Hinweis `tmj.buildings-ignored`, wenn Punkte oder Modelle abweichen). Ein Undo-Schritt „Tiled importiert: Karte 0123“; gleich der Welt → kein Schritt (`change: null`). Alle Fehler gesammelt (`TmjError {path, message}`) | „lehnt fremde Kacheln/Größen ab“; Gebäude bearbeitet das Studio (Drag), Events gehören nicht zur Karte (C §4.2); Freiform und Verbatim-Teile kennt Tiled nicht |
| Byte-Gleichheit | Round-Trip über `WorldJson`-Gleichheit (`import(export(w)) == w` für jedes gültige `w`), die Datei schreibt der Store-Writer kanonisch; der Test vergleicht die Datei-Bytes nach `save` | Gate „dieselbe JSON byte-gleich“ |
| `.gitattributes` | Projektvorlage `metadata/git/gitattributes`: die LFS-Kommentarzeile wird zu „Beifiles der Phase 6 (Modelle, Bilder): ohne Diff und Merge; LFS erst ab 1 MB je Datei, etwa: *.glb filter=lfs …“ und darunter `*.png binary`, `*.glb binary`; Repo-`.gitattributes` bekommt dieselben zwei Zeilen | Entscheidungen „Git LFS: nein; `.gitattributes` mit `*.png binary` und `*.glb binary`“ |
| Beifiles im Textprojekt | Schreiben einer Karte und Export behalten jedes `NNNN.<name>.glb`, das `world.freeform` der Karte nennt, und löschen nicht genannte wie jedes andere Beifile; Tileset-Ordner: `PPPP.<name>.glb`, das eine Kachel von `PPPP.json` nennt, bleibt; der Textprojekt-Import macht aus Beifiles keine Member | Writer löscht heute jede fremde Datei `NNNN.*` (Voraussetzungen) |
| Schemata | `WORLD_VERSION` bleibt 2, `maps.world` bleibt 2 (`glb` stand schon im Schema); `TILESET_VERSION` bleibt 1, `model` ist ab jetzt erlaubt: nur bei Rolle `piece`, Name nach der Regel oben, Datei muss nicht existieren (Prüfung `world.side-file` beim Backen) | keine Migration für ein Feld, das bisher abgelehnt wurde |
| Eingabe-Hash | `input_hash` nimmt die Bytes jedes genannten Freiform-Beifiles und jedes Modell-Beifiles der Teilmodelle, die die Karte setzt (fehlend: Marke `missing`); `WORLD_BAKE_REV = 2` | Spezifikation §3.5 „Eingaben … Beifiles“; inkrementell bleibt je Karte |
| Szene | `get_world_mesh` liefert `world_faces` (Raster, Teilmodell-Geometrie, Freiform); eine Textur, die keine Kachel des Tilesets ist (neu angehängt), findet `tileset_texture` im Paket (Palette `palette_name(name)`, sonst 0); im DTO trägt sie die Größe aus `pack_textures` und die Rolle `piece` (Platzhalterfarbe `spezial`) | „Vorschau in der Szene über `get_world_mesh` zeigt sie“ |
| MCP | kein kuratiertes Werkzeug; alle neuen Befehle über `g4_call` (`--expert`), Abschnitt in `docs/mcp.md` | Spezifikation nennt für 6.5b keins; 6.4a besitzt die kuratierten |
| Oberfläche | klein: Import-Dialog erkennt bei Welt-Karten `.glb`/`.gltf`/`.obj` als Teil „Freiform (glTF)“ (Feld „Lage (x, y, z)“ in Welteinheiten, Vorgabe 0, 0, 0) und `.tmj` als „Tiled (TMJ)“; Export-Dialog bietet bei Welt-Karten „Tiled (TMJ)“; der Abschnitt „Quelle“ (6.3b) bekommt „Als glTF übernehmen (Form 2)“ neben „Karte als Quelle übernehmen“; Entfernen einer Freiform nur über `g4_call` | Malwerkzeuge sind 6.4a; der Dialog „Teile importieren…“ ist der vorhandene Ort |

### Nicht in 6.5b

- Vanilla-Teilmodelle herauslösen, Vanilla-NSBMD zurück ins Raster, Bearbeiten von Vanilla-Modellen über den Codec, Tiled-Erweiterung, Automapping (Spezifikation „Nicht in Phase 6“, wörtlich).
- TMJ als Weltformat, TMX (XML), Tiled-„Worlds“ über mehrere Karten, Wang-Sätze im TMJ (6.4b hat die Kantensätze), Events im TMJ, Bilder beim Import lesen.
- Freiform verschieben oder skalieren über einen eigenen Befehl (Entfernen + Hinzufügen; `at` steht in der JSON), Animationen, Kamera, Eckenfarben aus glTF, Kollision oder BDHC aus Freiform-Polygonen, eine Teilmodell-Bibliothek jenseits der Beifiles, Prüfung `git.large-binary` (Gutachten C §5.5 nannte sie, die Entscheidungen nicht).

## Global Constraints

- **Lizenz:** „Lizenz: gen4-studio ist MIT. Die Writer von DSPRE (NsbmdWriter, NsbtxWriter, TileBake, BdhcBuild) sind AGPL-3.0: kein Code übernehmen, auch nicht abschreiben.“ PDSMS hat keine Lizenz: kein Code, kein `.pdsmap`/`.pdsts`. Tiled und seine Plugins sind GPL: nur das dokumentierte Format (TMJ) nutzen, keinen Code; `libtiled` (BSD-2) wird nicht gebraucht. Erlaubt: Formatwissen aus pret/pokeplatinum, apicula (0BSD), die TMJ-Dokumentation, eigene Messungen.
- „Keine Abhängigkeit vom NITRO-SDK (g3dcvtr.exe).“
- „Byte-Treue: Unveränderte Karten bauen weiterhin byte-gleich. Die Original-Assets bleiben unberührt, solange der Nutzer nicht malt.“ Form 1 bleibt verbatim; Form 2 ändert nur die Karte, die der Nutzer übernimmt.
- „Keine ROM-Daten im Repo“ (`scripts/check-no-rom-data.sh` vor jedem Commit): keine GLB aus Vanilla-Modellen als Fixture, keine Kachelbilder; Freiform-GLBs in Tests entstehen synthetisch (`model_import::samples`, `side_glb` aus Testflächen); Szenarien nennen Kacheln nur als Index.
- **Abhängigkeiten:** keine neue (eigener TMJ-Leser; `image`, `base64`, `serde_json` vorhanden). Jede doch nötige Abhängigkeit mit Lizenz-Audit (`cargo deny --locked check licenses advisories`), Version und Lizenz im Commit-Text und in dieser Zeile.
- **„Nicht in Phase 6“:** Vanilla-Teilmodelle herauslösen, Vanilla-NSBMD zurück ins Raster, Bearbeiten von Vanilla-Modellen über den Codec, Tiled-Erweiterung, Automapping.
- **Worktree:** `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut; `tools` als Symlink auf den Haupt-Checkout (`ln -s $HOME/Projekte/lab/editor/gen4-studio/tools tools`, nie einchecken); `G4_APICULA=$HOME/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula`; Test-Umgebung nur auf `G4_VITE_PORT=1580 G4_BRIDGE_PORT=1581`, andere Ports nie anfassen; Testkopien nur `Roms/_testkopien/6-5b-<zweck>` (`cp -cR` Ordner, `cp -c` `.nds`, Ziel existiert noch nicht), Originale nie öffnen; gated Tests schreiben nur auf `vanilla_copy()`.
- cargo-Tests immer mit `--all-features`; löst `-p g4-kernel` den Download in rotoms Build-Skript aus, über `--workspace` mit Namensfilter.
- Jeder Task endet mit einem Commit; jede Nachricht endet mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`). **Kein `scripts/gate.sh` im Worktree** (läuft nach dem Merge auf main), kein Merge, kein Push, kein `git stash`; nie `cd` in den Haupt-Checkout.
- Lang laufende Befehle (gated Tests, Workspace-Tests, Brücke, Szenarien) im Hintergrund mit Log unter `target/agent/6-5b/<zweck>.log`, `uptime` davor; ein Zeittest, der unter Last reißt, wird allein wiederholt, die Grenze nie gelockert.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh`; bei `scripts/g4emu` `uv run pytest -m "not integration" -q`; immer `bash scripts/check-no-rom-data.sh`.
- Kein `unwrap`/`expect`/Panik im Produktcode der Leser (Beifile, TMJ): jede Datei kann kaputt oder von Hand geändert sein.
- UI nur aus Bausteinen und Tokens; deutsche Texte, englische Bezeichner und Commit-Messages.
- **Gate 6.5b** (Spezifikation §4, wörtlich): „glTF-Freiform in einer Welt-Karte backt und ist in g4emu zu sehen; TMJ-Export einer gemalten Karte, Import ohne Änderung ergibt dieselbe JSON byte-gleich; Form 2 von „übernehmen“ für eine Vanilla-Karte backt apicula-lesbar (Abweichung in der Polygonzahl dokumentiert)“, dazu (Kopf der Gate-Spalte) gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US, Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer.

## Review Focus

- **Speichern oder Export löscht ein Beifile:** eine Welt-Karte mit `freeform[0].glb = "0003.haus.glb"` wird gespeichert, exportiert oder nach einem Strich neu geschrieben; ein Tileset mit `model` wird gespeichert: das Beifile bleibt byte-gleich, ein nicht mehr genanntes verschwindet (Task 3 `saving_a_world_map_keeps_its_side_files`, `export_keeps_named_side_files_and_drops_orphans`, `the_tileset_folder_keeps_piece_models`).
- **Beifile fehlt, ist kaputt oder von außen ersetzt** (Git-Checkout ohne die Datei, Blender schreibt ein GLB mit Bildern und eigenen Materialnamen an seine Stelle): `world.side-file` bzw. `bake.texture-missing` mit Pfad und Namen, letztes Modell bleibt, kein Panik, die Szene zeigt das Raster (Task 2 `a_missing_side_file_refuses_the_model_and_keeps_the_last`, `a_foreign_glb_with_unknown_materials_is_texture_missing`).
- **Von Tiled gespeichertes TMJ:** Tiled fügt Felder hinzu (`class`, `parallaxx`, `tintcolor`, `id`), schreibt `encoding: base64`, setzt `compression`, lagert das Tileset aus, der Nutzer legt eine neue Ebene ohne Eigenschaften an oder schiebt ein Objekt neben das Raster: Unbekanntes wird ignoriert, Base64 gelesen, Kompression und externes Tileset mit Text abgelehnt, die neue Ebene wird Überlagerung, das Objekt nennt seine ID (Task 7 `a_tmj_with_tiled_extras_reads`, `base64_reads_and_compression_is_refused`, `a_new_layer_without_properties_is_an_overlay`, `an_object_off_the_grid_is_named`).
- **Atomarität beim Hinzufügen:** Freiform mit neuer Textur: Paket, Beifile und Welt sind **ein** Schritt; ⌘Z entfernt Beifile und Textur, ⇧⌘Z stellt beides her; scheitert der Weltschritt, ist nichts geschrieben (Task 4 `freeform_add_is_one_undo_step_with_pack_and_side_file`, `a_failed_world_step_rolls_back_pack_and_side_file`).
- **Form 2 an Grenzen:** apicula fehlt, scheitert mit dem Paket, ein Material ohne Paket-Textur, Modell über 0xF000 gebacken: Ablehnung mit Text und Code, kein Undo-Schritt, kein Beifile, die Karte bleibt binär (Task 5 `form2_without_apicula_is_refused`, `form2_with_an_unbound_material_is_refused`, `form2_over_the_budget_leaves_no_step`).

---

### Task 1: Kanonisches Beifile, Namensregeln, Schema, `.gitattributes`

**Files:**
- Create: `crates/g4-kernel/src/world/freeform.rs` (reiner Teil, + Tests)
- Modify: `crates/g4-kernel/src/world.rs` (`pub mod freeform;`, Re-Exporte, angehängt), `crates/g4-kernel/src/world/grid.rs` (Regeln für `glb`-Namen und `at`), `crates/g4-kernel/src/export.rs` (`write_glb` → `pub(crate)`), `crates/g4-kernel/metadata/git/gitattributes`, `crates/g4-kernel/src/git/templates.rs` (Erwartung der Vorlage im Test), `.gitattributes`

**Interfaces:**
- Consumes: `model_import::{read_model, mesh::{to_faces, Binding}}`, `bake::{Face, PackTexture}`, `export::write_glb`.
- Produces:

```rust
// world/freeform.rs
pub const FREEFORM_UNIT: f32 = 16.0;        // world units per glTF unit in a side file
pub const APICULA_UNIT: f32 = 1.0;          // world units per unit of an apicula GLB
pub const FREEFORM_LAYER: u8 = 254;
pub const SIDE_FILE_CODE: &str = "world.side-file";
pub const SIDE_NAME_MAX: usize = 32;
/// `stem` cleaned for a side file name (lower case, a–z 0–9 `_` `-` kept, every other char `_`, cut to
/// SIDE_NAME_MAX, empty → "freiform"), `_2`, `_3` … while it is in `taken`.
pub fn side_name(stem: &str, taken: &[String]) -> String;
pub fn freeform_file(map: u16, name: &str) -> String;           // "0123.haus.glb"
pub fn freeform_rel(file: &str) -> PathBuf;                     // data/maps/<file>
pub fn piece_rel(package: u16, name: &str) -> PathBuf;          // data/tilesets/0006.<name>.glb
/// The `<name>` of `file` when it is a side file of `stem` (`NNNN.<name>.glb` with the rule), else None.
pub fn side_name_of(file: &str, stem: u16) -> Option<&str>;
/// The side file of `faces` (rules „Form des Beifiles“), positions divided by `unit`; a face whose texture
/// is not in `textures` is KernelError::Refused (the caller binds first).
pub fn side_glb(faces: &[Face], textures: &[PackTexture], unit: f32) -> Result<Vec<u8>, KernelError>;
/// The faces of a side file read from `path`, bound only to `textures` by material name, scaled by
/// FREEFORM_UNIT, layer `layer`; `world.side-file` (missing, unreadable, no geometry) or
/// `bake.texture-missing` (first material without a pack texture) as the message.
pub fn side_faces(path: &Path, textures: &[PackTexture], layer: u8) -> Result<Vec<Face>, WorldBakeMessage>;
```

`grid.rs`: `glb` muss `side_name_of(glb, map)` erfüllen (der Parser kennt die Karte nicht: `WorldGrid::parse` prüft die Form `NNNN.<name>.glb`, `world_problems` die Nummer gegen die Karte); `at` je Achse −512..=511; Texte wörtlich aus „Entscheidungen“.

- [ ] **Step 1: Scheiternde Tests** in `freeform.rs` (Flächen aus `fixtures::one_tile_world`/`checkerboard_world` über `mesh` + `Face::from`, Paket `synthetic_pack_textures()`, Ordner aus `tempfile`): `side_glb_round_trips_the_faces` (Multimenge der Flächen bis auf zyklische Drehung der Ecken gleich, Ebene `FREEFORM_LAYER`), `a_triangle_stays_one_triangle`, `the_side_file_is_a_fixed_point` (`side_glb(side_faces(side_glb(f)))` byte-gleich `side_glb(f)`), `side_glb_has_no_images_and_names_materials_after_textures` (JSON: keine `images`, Materialnamen = Texturnamen, `asset.generator` „gen4-studio“), `a_side_file_with_an_unknown_material_is_texture_missing`, `a_missing_side_file_is_side_file` (Text „Das Beifile data/maps/0003.haus.glb fehlt.“), `garbage_is_side_file_unreadable`, `side_name_cleans_and_numbers` (`"Haus Groß"` → `haus_gro_`, leer → `freiform`, vergeben → `haus_2`, 40 Zeichen → 32), `side_name_of_accepts_only_the_own_stem` (`0003.haus.glb` für 3 ja, für 4 nein, `0003.Haus.glb`, `0003..glb`, `0003.haus.gltf` nein); `prop_side_glb_round_trips` (proptest, 64 Fälle: Zufallsraster wie `prop_model_floor_meets_the_bdhc` von 6.3b, ohne Teilmodelle). `grid.rs`: `a_glb_name_must_be_a_side_file`, `at_is_bounded`. `templates.rs`: `the_project_attributes_mark_png_and_glb_binary`.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- world::freeform world::grid git::templates` → FAIL.
- [ ] **Step 3: Umsetzen**; `side_glb` baut das JSON mit `serde_json::json!` (Puffer 0, je Textur eine Primitive mit `POSITION`, `TEXCOORD_0`, Indizes `u32`, `min`/`max` an `POSITION`) und schreibt mit `export::write_glb`; `side_faces` = `read_model` → Bindungen `Binding { texture, width, height }` je Material aus `textures` → `to_faces(&mesh, FREEFORM_UNIT, &bindings)` → Ebene setzen; `model.material-untextured` wird `bake.texture-missing` mit dem Materialnamen, jede andere `PartMessage` `world.side-file` mit ihrem Satz als Grund.
- [ ] **Step 4: Lauf** → PASS; fmt, clippy.
- [ ] **Step 5: Commit** `feat(world): canonical side file for free-form models, side file names, binary attributes for png and glb` (mit Trailer).

### Task 2: Bäcker und Vernetzer: Freiform und Teilmodell-Geometrie

**Files:**
- Modify: `crates/g4-kernel/src/world/mesh.rs` (`PieceModels`, `mesh_with`), `crates/g4-kernel/src/world/bake.rs` (`BakeEnv.side`, `bake_grid`, `bake_world`, `WorldBakeReport.freeform_faces`, `WORLD_BAKE_REV = 2`, `world.freeform-later` entfernt, Text von `world.verbatim-mixed`), `crates/g4-kernel/src/world/freeform.rs` (Store-Teil: `load_side_models`, `world_faces`), `crates/g4-kernel/src/world/record.rs` (`input_hash` mit Beifiles), `crates/g4-kernel/src/world/doc.rs` (Fixture `freeform_world`)

**Interfaces:**
- Consumes: Task 1; 6.3b `bake_grid`, `bake_world`, `input_hash`.
- Produces:

```rust
// world/mesh.rs
/// Model-space faces of piece tiles that have a readable side file, by tile index.
#[derive(Debug, Clone, Default)]
pub struct PieceModels { pub by_tile: BTreeMap<u16, Vec<Face>> }
/// `mesh` with the geometry of `pieces` in place of the footprint face (rules „Teilmodell-Lage“); a tile
/// without an entry keeps its footprint face. `mesh(g, t) == mesh_with(g, t, &PieceModels::default())`.
pub fn mesh_with(grid: &WorldGrid, tileset: &Tileset, pieces: &PieceModels) -> Vec<MeshFace>;
// world/freeform.rs (store side)
/// The side files a world reads: per `freeform` entry with `glb` its faces shifted by `at` (layer
/// FREEFORM_LAYER) or the message; per piece tile with `model` that the grid places its faces or the message.
#[derive(Debug, Clone, Default)]
pub struct SideModels { pub freeform: Vec<Result<Vec<Face>, WorldBakeMessage>>, pub pieces: PieceModels,
    pub messages: Vec<WorldBakeMessage> }
pub fn load_side_models(project: &Project, map: u16, world: &WorldJson, tileset: &Tileset,
    textures: &[PackTexture]) -> SideModels;
/// What the scene shows of world map `map`: grid, piece geometry and free-form faces, the pack textures
/// they bind, the messages of the side files. `None` without world.
pub struct WorldFaces { pub faces: Vec<MeshFace>, pub textures: Vec<PackTexture>, pub messages: Vec<WorldBakeMessage> }
pub fn world_faces(store: &mut Store, map: u16) -> Result<Option<WorldFaces>, KernelError>;
// world/bake.rs
pub struct BakeEnv<'a> { pub map: u16, pub tileset: &'a Tileset, pub input: Option<BakeInput<'a>>,
    pub family: Family, pub side: &'a SideModels }
// WorldBakeReport: … , pub freeform_faces: usize   (appended)
pub const WORLD_BAKE_REV: u32 = 2;
// world/doc.rs fixtures (test-support)
/// `one_tile_world(tileset)` with `freeform = [{glb: "0000.wuerfel.glb", at: [0, 0, 0]}]`.
pub fn freeform_world(tileset: u16) -> WorldJson;
```

`bake_grid` (Reihenfolge der Meldungen fest): Meldungen aus `env.side.messages` (je Beifile, als Fehler); Verbatim-Modell mit Raster- **oder** Freiform-Flächen → `world.verbatim-mixed` (neuer Text); sonst Flächen = `mesh_with(grid, tileset, &side.pieces)` + alle `Ok`-Freiform-Flächen; leer → `bake.empty`; ohne Gebiet `world.no-area`; sonst `bake`. Ein Beifile-Fehler lehnt das Modell ab (Modell `None`), BDHC und Permissions entstehen weiter. `bake_world` lädt `load_side_models` mit den `pack_textures` des Pakets `world.tileset`. `input_hash` hängt je Freiform-Beifile und je Modell-Beifile der gesetzten Teilmodell-Kacheln Pfad und Bytes (oder `missing`) an, in Dokumentreihenfolge.

- [ ] **Step 1: Scheiternde Tests:** `mesh.rs` `mesh_with_nothing_is_mesh` (alle 6.3a-Fixtures), `a_piece_model_replaces_its_footprint` (Kachel 5 des synthetischen Tilesets mit einem Würfel 16 × 16 × 16 um den Ursprung, Stück bei (4, 4): Ecken um die Fußabdruck-Mitte, Boden auf der Höhe der Fußabdruck-Fläche, Ebene 255, keine Fußabdruck-Fläche), `a_turned_piece_model_turns_clockwise` (`rotate 90`: eine Ecke bei Modell (8, 0, −8) landet bei (8, 0, 8) relativ zur Mitte; Fußabdruck [3, 1] gedreht), `permissions_still_follow_the_footprint`; `bake.rs` `freeform_faces_join_the_bake` (`freeform_world`: Modell gebacken, `freeform_faces == 6`, `decode_quads` enthält die Würfelflächen verschoben um `at`), `at_moves_the_freeform` (`at [16, 8, 0]`), `freeform_alone_bakes` (Raster ganz `.`, nur Freiform: `ok`), `a_missing_side_file_refuses_the_model_and_keeps_the_last` (Review Focus: `world.side-file`, `model_kept`, BDHC und Permissions neu), `a_foreign_glb_with_unknown_materials_is_texture_missing` (Review Focus: GLB aus `samples::PLANE_GLB` mit eingebettetem Bild und Material `g4_imp_plane_mat` als Beifile → `bake.texture-missing`), `freeform_out_of_range_is_face_out_of_range` (`at [511, 0, 0]`), `verbatim_with_freeform_is_mixed` (Text wörtlich), `freeform_later_is_gone`, `world_faces_show_grid_pieces_and_freeform`; `record.rs` `input_hash_moves_with_a_side_file` (Beifile-Bytes ändern → anderer Hash; Beifile löschen → anderer Hash; ein nicht genanntes `0000.x.glb` ändert nichts).
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- world::mesh world::bake world::freeform world::record` → FAIL.
- [ ] **Step 3: Umsetzen**; die 6.3b-Tests `glb_freeform_is_refused_for_now` und `verbatim_with_tiles_is_refused_and_keeps_the_model` auf die neuen Regeln umstellen und im Commit nennen; die Drehung je Viertel `(x, z) → (−z, x)` auf Ecken (ganzzahlig, keine Rundung).
- [ ] **Step 4: Lauf** → PASS; Workspace `cargo test --workspace --all-features > target/agent/6-5b/t2.log 2>&1` (Hintergrund) grün.
- [ ] **Step 5: Commit** `feat(world): bake free-form side files and piece geometry with the grid` (mit Trailer).

### Task 3: Beifiles im Textprojekt und im Tileset

**Files:**
- Modify: `crates/g4-kernel/src/textproject/source/writers.rs` (Karten- und Tileset-Ordner behalten genannte `.glb`), `crates/g4-kernel/src/textproject/export.rs` (ebenso), `crates/g4-kernel/src/textproject/import.rs` (Beifiles sind keine Member), `crates/g4-kernel/src/world/tileset.rs` (`model` erlaubt, Regel, `tileset_texture` mit Paket-Rückfall, `TileChange.model`), `crates/g4-kernel/src/world/check.rs` (`glb`-Nummer gegen die Karte)

**Interfaces:**
- Consumes: Task 1 (`side_name_of`, `piece_rel`).
- Produces:

```rust
// world/freeform.rs
/// The side files `world` names (file names in data/maps/).
pub fn named_side_files(world: &WorldJson) -> Vec<String>;
/// The piece model side files `tileset` names (file names in data/tilesets/).
pub fn named_piece_files(tileset: &Tileset) -> Vec<String>;
// world/tileset.rs
// TileChange: … , #[serde(default)] pub model: Option<Option<String>>   (appended; Some(None) clears)
```

Regeln: `tileset_problems` lehnt `model` ab, wenn die Rolle nicht `piece` ist („`model` haben nur Teilmodelle (`piece`).“) oder der Name die Regel verletzt („`model` ist ein Name aus a–z, 0–9, `_`, `-`, höchstens 32 Zeichen.“); der alte Satz „Teilmodell-Dateien kommen mit 6.5b.“ entfällt. `tileset_texture`: findet keine Kachel die Textur, gilt die Paket-Textur des Namens mit Palette `palette_name(name)`, sonst 0. Writer und Export: beim Löschen von `NNNN.*` bleiben die Dateien aus `named_side_files` (Karte) bzw. `named_piece_files` (Tileset `PPPP.json`).

- [ ] **Step 1: Scheiternde Tests:** `source/tests.rs` `saving_a_world_map_keeps_its_side_files` (Review Focus: `world_ready()`, `freeform_world(0)` mit Beifile `0000.wuerfel.glb`, Strich, `save`: Beifile byte-gleich; `freeform` geleert, `save`: Beifile weg), `export.rs` `export_keeps_named_side_files_and_drops_orphans`, `the_tileset_folder_keeps_piece_models` (Review Focus), `import.rs` `side_files_are_no_members` (Plan des Imports nennt sie nicht); `tileset.rs` `model_is_allowed_on_pieces_only` (beide Texte), `tileset_edit_sets_and_clears_model`, `tileset_texture_falls_back_to_the_pack` (Paket mit einer angehängten Textur `neu`, Tileset ohne Kachel `neu` → RGBA); `check.rs` `a_glb_of_another_map_is_a_schema_problem` (`0004.haus.glb` an Karte 3: „`glb` nennt ein Beifile dieser Karte: `0003.<name>.glb` …“).
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- textproject:: world::tileset world::check` → FAIL.
- [ ] **Step 3: Umsetzen**; den 6.3a-Test `tiles[2].model: Teilmodell-Dateien kommen mit 6.5b.` auf die neue Regel umstellen und im Commit nennen. `--test textproject` (ohne ROM) bleibt grün.
- [ ] **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(textproject): keep named side files of world maps and tilesets, piece models in the tileset` (mit Trailer).

### Task 4: Freiform hinzufügen und entfernen, Teilmodell setzen (ein Schritt mit Paket und Beifile)

**Files:**
- Modify: `crates/g4-kernel/src/world/freeform.rs` (Befehle, + Tests), `crates/g4-kernel/src/world/tileset.rs` (`tileset_piece_set`), `crates/g4-kernel/src/world.rs`

**Interfaces:**
- Consumes: Tasks 1–3; 6.5a `read_model`, `plan_textures`, `to_faces`, `append_textures`, `appended_pack_or_refusal` (pub(crate)), `ModelRefusal`, `TexturePreview`; Store `undo_depth`, `squash_since`, `rollback_to`, `record_file_step`, `Tx::edit_world`, `forget_world_bakes`.
- Produces:

```rust
// world/freeform.rs
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FreeformAddRequest { pub map: u16, pub path: PathBuf, #[serde(default)] pub at: [i32; 3],
    #[serde(default)] pub options: ImportOptions, #[serde(default)] pub name: Option<String> }
#[derive(Debug, Clone, Serialize)]
pub struct FreeformReport { pub map: u16, pub glb: String, pub stats: MeshStats, pub textures: Vec<TexturePreview>,
    pub notes: Vec<PartMessage>, pub change: Option<ChangeId>, pub bake: Option<WorldBakeReport> }
pub fn freeform_label(map: u16, glb: &str) -> String;          // „Freiform hinzugefügt: Karte 0123 (0123.haus.glb)“
/// Reads and binds the model against the world's pack (new textures planned and appended in memory with
/// `appended_pack_or_refusal`, nothing written) and bakes the world with it in memory (`report.bake`, also
/// when it is refused). Inner Err: the model's refusal (ModelRefusal).
pub fn world_freeform_inspect(store: &mut Store, req: &FreeformAddRequest) -> Result<Result<FreeformReport, ModelRefusal>, KernelError>;
/// One undo step (squash_since, label freeform_label): new textures appended to `world.tileset`, the side
/// file written as a file step, the entry appended to `world.freeform`; refused (Refused with the German
/// sentence) when inspect refuses or its bake is not `ok`; any failure rolls back to the mark.
pub fn world_freeform_add(store: &mut Store, req: &FreeformAddRequest) -> Result<FreeformReport, KernelError>;
/// One undo step „Freiform entfernt: Karte 0123 (0123.haus.glb)“: entry `index` removed, its side file removed
/// as a file step when no other entry names it. Refused for a `nsbmd` entry and a past index.
pub fn world_freeform_remove(store: &mut Store, map: u16, index: usize) -> Result<WorldReport, KernelError>;
// world/tileset.rs
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PieceSetRequest { pub package: u16, pub tile: u16, pub path: PathBuf, pub footprint: [u8; 2],
    #[serde(default)] pub options: ImportOptions, #[serde(default)] pub name: Option<String> }
#[derive(Debug, Clone, Serialize)]
pub struct PieceSetReport { pub package: u16, pub tile: u16, pub model: String, pub stats: MeshStats,
    pub textures: Vec<TexturePreview>, pub notes: Vec<PartMessage>, pub change: ChangeId }
/// One undo step „Teilmodell gesetzt: Kachel 5 in Tileset 0006 (baum)“: textures appended to `package`,
/// side file `PPPP.<name>.glb`, tile set to role `piece`, `footprint`, `model`; needs the tileset file;
/// then forget_world_bakes(package).
pub fn tileset_piece_set(store: &mut Store, req: &PieceSetRequest) -> Result<PieceSetReport, KernelError>;
```

Ablauf `world_freeform_add`: Welt da (sonst `not_a_world_map`), Textprojekt, nicht beschäftigt; `options.check()`; `read_model(path)` (6.5a-Regeln: Dateien nur im Ordner des Modells); `plan_textures(materials, pack_textures(world.tileset), colors)`; `to_faces(mesh, options.unit, bindings)`; Name `side_name(req.name ∪ Dateistamm, vorhandene)`; Marke `undo_depth()`; `append_textures` (nur bei neuen); `side_glb(faces, Paket nach dem Anhängen, FREEFORM_UNIT)` → `FileStep` (`asset` = WorldMap der Karte, `FileStepKind::Reset`) über `write_file_step` + `record_file_step`; `transaction` mit `Tx::edit_world` (Eintrag `{glb, at}` angehängt; backt nach 6.3b); `squash_since(mark, label)`; jeder Fehler → `rollback_to(mark)`.

- [ ] **Step 1: Scheiternde Tests** (`world_ready()`, Beispiele aus `write_samples` im Projektordner `import/`): `inspect_reads_the_cube_without_writing` (Bericht: 6 Vierecke, neue Textur `g4_imp_cube`, Bake `ok`; Paket, Ordner `data/maps/` und Welt unverändert), `freeform_add_is_one_undo_step_with_pack_and_side_file` (Review Focus: Beifile `0000.g4_imp_cube.glb`, Paket mit `g4_imp_cube`, Welt mit Eintrag, `history()` ein Eintrag mit dem Label; `undo` → Beifile weg, Paket und Map-Asset byte-gleich vorher; `redo` → wieder da), `a_failed_world_step_rolls_back_pack_and_side_file` (Review Focus: Welt-Karte gesperrt nach dem Anhängen per Testhaken oder `edit_world`, das ablehnt: nichts bleibt), `the_plane_glb_and_the_cube_obj_both_become_glb_side_files`, `a_second_add_of_the_same_file_numbers_the_name_and_reuses_the_texture`, `a_model_over_the_budget_is_refused_by_inspect_and_add` (`BIG_OBJ`: Inspect gibt `bake.ok == false` mit `bake.model-too-big`, Add lehnt ab, kein Schritt, kein Beifile, Paket unverändert), `add_refuses_without_world_and_outside_the_text_project`, `remove_drops_the_entry_and_the_unshared_side_file` (⌘Z stellt beides her), `remove_refuses_a_verbatim_entry`; `tileset.rs` `piece_set_is_one_step_and_rebakes_the_maps_of_the_package` (Würfel als Kachel 5, Stück auf einer Welt-Karte des Pakets: nach dem Schritt backt das Stück mit Geometrie; ⌘Z → Fußabdruck-Fläche), `piece_set_refuses_without_tileset_file`.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- world::freeform world::tileset` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces.
- [ ] **Step 4: Lauf** → PASS; Workspace im Hintergrund grün.
- [ ] **Step 5: Commit** `feat(world): add and remove free-form models and set piece models as one step with pack and side file` (mit Trailer).

### Task 5: „Karte als Quelle übernehmen“, Form 2 (glTF über apicula)

**Files:**
- Modify: `crates/g4-kernel/src/world/adopt.rs` (`world_adopt_with`, Form 2, `Form2Report`, + Tests), `crates/g4-kernel/src/world/freeform.rs` (`bind_vanilla_materials`)

**Interfaces:**
- Consumes: 6.3b `world_adopt` (Form 1: Höhen, `terrain`, Override), Task 1 (`side_glb`, `APICULA_UNIT`, `freeform_file`), `export::nsbmd_to_glb`, `Nsbmd::read`.
- Produces:

```rust
// world/adopt.rs
pub const ADOPT_TEXTURE_CODE: &str = "world.adopt-texture";
pub const FORM2_SIDE_NAME: &str = "model";
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct VanillaGeometry { pub polygons: u32, pub quads: u32, pub triangles: u32, pub bytes: usize }
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BakedGeometry { pub faces: usize, pub quads: usize, pub triangles: usize, pub strips: usize,
    pub single_quads: usize, pub dropped: usize, pub bytes: usize }
#[derive(Debug, Clone, Serialize)]
pub struct Form2Report { pub vanilla: VanillaGeometry, pub baked: BakedGeometry, pub ignored: Vec<String>, pub glb: String }
// WorldAdoptReport: … , pub form2: Option<Form2Report>   (appended)
/// Form 1 as before; Form 2 (rules „Form 2“) with `apicula` (None → Refused „Form 2 braucht apicula …“).
pub fn world_adopt_with(store: &mut Store, req: &WorldAdoptRequest, apicula: Option<&Path>) -> Result<WorldAdoptReport, KernelError>;
/// Form 2 from a GLB already converted (`glb_path`, units APICULA_UNIT): the step without apicula (tests).
pub fn world_adopt_glb(store: &mut Store, req: &WorldAdoptRequest, glb_path: &Path) -> Result<WorldAdoptReport, KernelError>;
// world/freeform.rs
/// One binding per material of `mesh` by the texture rule of Form 2 against the vanilla `model` and `pack`;
/// Err(material name) for the first that binds nothing in the pack.
pub fn bind_vanilla_materials(mesh: &Mesh, model: &[u8], pack: &[PackTexture]) -> Result<Vec<Option<Binding>>, String>;
```

`world_adopt_glb`: Vorprüfungen wie Form 1 (DSPRE-Layout, schon Welt, gesperrt); Welt wie Form 1 gebaut, aber `freeform = [{glb: freeform_file(map, "model"), at: [0, 0, 0]}]`; Flächen `to_faces(mesh, APICULA_UNIT, bind_vanilla_materials(…))`; Beifile als Dateischritt, Welt über `transaction`, `squash_since(mark, adopt_label(map))`; Bericht `bake.ok == false` → `rollback_to(mark)` und `KernelError::Refused` mit Code und Satz der ersten Fehlermeldung. `world_adopt_with` für `Gltf`: Modell und Paket aus dem Store, `nsbmd_to_glb` in `<root>/.g4/tmp/adopt-NNNN` (zuerst mit Paket; Fehler → ohne), `world_adopt_glb`, Ordner danach löschen. Der 6.3b-Satz „Form 2 (glTF) kommt mit 6.5b.“ entfällt.

- [ ] **Step 1: Probe** (15 Minuten Zeitkasten, kein Commit): auf `_testkopien/6-5b-probe` (Platin DE) für die Karte des ersten Headers von Zweiblattdorf `nsbmd_to_glb` mit Paket; aufschreiben (als Kommentar an `APICULA_UNIT` und in den Commit-Text): Materialnamen gleich den MDL0-Namen?, Bilder als `<textur>.png`?, Knoten, Skins, Eckenfarben (`model.parts-ignored`), Positionen in Welteinheiten (Ecken des Vanilla-Modells über `decode`/apicula verglichen)? Weicht etwas ab, zuerst „Entscheidungen“ und diesen Task anpassen.
- [ ] **Step 2: Scheiternde Tests** (`TextFixture::exported()` mit `world_ready()`-Paket; das „Vanilla-Modell“ ist ein mit `bake` gebackenes synthetisches Modell, das „apicula-GLB“ schreibt der Test mit `side_glb(faces, textures, APICULA_UNIT)` und benennt die Materialien nach den MDL0-Materialien des gebackenen Modells): `form2_adopts_with_a_glb_side_file` (JSON: `freeform[0].glb == "0000.model.glb"`, `terrain.bdhc` verbatim, Override 32 Zeilen; Beifile da; Map-Asset: Modell neu gebacken, BDHC und Permissions byte-gleich dem Original), `form2_reports_both_polygon_counts` (`vanilla` aus `Geometry`, `baked` aus dem Bericht), `form2_binds_by_mdl0_material_then_by_image_stem`, `form2_with_an_unbound_material_is_refused` (Review Focus: Text wörtlich, kein Schritt, kein Beifile), `form2_over_the_budget_leaves_no_step` (Review Focus: `oversize_world`-Flächen als GLB → `bake.model-too-big`, `history()` leer, Karte binär), `form2_without_apicula_is_refused` (Review Focus), `form2_is_one_undo_step` (⌘Z → binär, Beifile weg), `painting_on_a_form2_map_bakes_both` (`world_edit` setzt Kacheln: `ok`, Flächen = Raster + Freiform), `form1_still_bakes_byte_equal` (6.3b-Test bleibt grün).
- [ ] **Step 3: Lauf** `cargo test --workspace --all-features -- world::adopt` → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** → PASS.
- [ ] **Step 6: Commit** `feat(world): adopt a map as source in form 2, the vanilla model as a glTF side file through apicula` (mit Trailer).

### Task 6: TMJ-Export

**Files:**
- Create: `crates/g4-kernel/src/world/tmj.rs` (Typen, Abbildung Welt → TMJ, Bilder; + Tests)
- Modify: `crates/g4-kernel/src/world.rs`

**Interfaces:**
- Consumes: `WorldJson`, `WorldGrid::parse`, `tile_token`/`parse_tile`, `ramp_token`, `override_token`, `tileset_texture`, `MapJson.buildings` (über `Store::map`), `image` (PNG).
- Produces:

```rust
// world/tmj.rs
pub const TMJ_VERSION: i64 = 1;
pub const TMJ_FORMAT: &str = "gen4-studio-world";
pub const HEIGHT_TILES: u32 = 79;          // −16..=62
pub const RAMP_TILES: [&str; 8] = ["n1", "n2", "e1", "e2", "s1", "s2", "w1", "w2"];
pub const FLIP_H: u32 = 0x8000_0000; pub const FLIP_V: u32 = 0x4000_0000; pub const FLIP_D: u32 = 0x2000_0000;
pub const FLIP_HEX: u32 = 0x1000_0000;
/// The TMJ subset this module writes and reads (serde; unknown fields ignored on read).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct TmjMap { /* Felder wie „TMJ-Schreiben“ */ }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct TmjLayer { /* tilelayer | objectgroup */ }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct TmjTileset { /* eingebettete Bildsammlung */ }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct TmjObject { /* id, gid?, x, y, width, height, properties */ }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct TmjProperty { pub name: String, #[serde(rename = "type")] pub kind: String, pub value: serde_json::Value }
pub fn gid_flags(rotate: u8, flip: bool) -> u32;                   // table „Drehen und Spiegeln“
pub fn flags_turn(flags: u32) -> Option<(u8, bool)>;               // inverse; None with FLIP_HEX
/// The TMJ of `world` (map `map`, buildings for the view layer, `tiles` of its tileset) and the images to write
/// next to it (relative path under `<stem>.tiles/`, PNG bytes).
pub fn to_tmj(map: u16, world: &WorldJson, buildings: &[BuildingJson], tileset: &Tileset,
    image_of: &dyn Fn(&TileJson) -> Option<Rgba>, stem: &str) -> (TmjMap, Vec<(String, Vec<u8>)>);
#[derive(Debug, Clone, Serialize)]
pub struct TmjExportReport { pub map: u16, #[serde(serialize_with = "crate::relpath::ser")] pub path: PathBuf,
    pub layers: usize, pub images: usize, pub pieces: usize, pub overrides: usize }
/// Writes `path` (and `<stem>.tiles/`) for world map `map`; refused without world.
pub fn world_export_tmj(store: &mut Store, map: u16, path: &Path) -> Result<TmjExportReport, KernelError>;
```

- [ ] **Step 1: Scheiternde Tests:** `flags_cover_the_eight_turns` (je Zeile der Tabelle; und gegen `mesh::uv_of`: für jede Drehung `(r, f)` und jede Zelle bildet die Tiled-Transformation des Bildes (D, dann H, dann V auf Pixel `(px, py)`) dasselbe Texel auf dieselbe Weltstelle ab), `hex_flag_is_refused`, `the_gate_world_exports_its_layers` (`gate_world`: Ebenen „Boden“, „Höhe“, „Rampen“, „Teilmodelle“, „Permissions“, „Gebäude (nur Ansicht)“ in dieser Reihenfolge; GIDs der Rampe `e1`; Höhe 2 → GID `erster + 18`; Override `6980` an (12, 21) als Rechteck x 192, y 336 mit `g4.kind` 0x69, `g4.collision` 0x80), `no_ramps_no_ramp_layer` (`one_tile_world`), `no_override_no_permission_layer`, `map_properties_name_format_map_tileset_derive`, `images_are_png_with_placeholders` (synthetisches Tileset ohne Dekodierung: 6 + 79 + 8 PNG zu 16 × 16, Platzhalterfarbe der Rolle), `export_writes_file_and_tiles_folder` (`world_ready()`), `buildings_are_points_at_their_position` (Gebäude bei Kachel (3, 5) → Punkt x 56, y 88), `export_refuses_without_world`.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- world::tmj` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(world): export a world map as a Tiled map (TMJ) with tile images` (mit Trailer).

### Task 7: TMJ-Import und Round-Trip

**Files:**
- Modify: `crates/g4-kernel/src/world/tmj.rs` (Leser, Abbildung TMJ → Welt, Import; + Tests), `crates/g4-kernel/src/world/doc.rs` (Fixture `tmj_world`: `gate_world` plus Überlagerung, zwei Teilmodelle, `derive: false`-Variante)

**Interfaces:**
- Consumes: Task 6; `Tx::edit_world`, `WorldGrid::parse`, `not_a_world_map`.
- Produces:

```rust
// world/tmj.rs
pub const TMJ_CODE: &str = "world.tmj";
pub const BUILDINGS_IGNORED_CODE: &str = "tmj.buildings-ignored";
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TmjError { pub path: String, pub message: String }     // path like "layers[2].data[517]"
/// The world `bytes` (a TMJ) describes on top of `target` (version, tileset, freeform, terrain kept); every
/// error collected.
pub fn from_tmj(bytes: &[u8], target: &WorldJson, tileset: &Tileset) -> Result<WorldJson, Vec<TmjError>>;
pub fn import_label(map: u16) -> String;      // „Tiled importiert: Karte 0123“
#[derive(Debug, Clone, Serialize)]
pub struct TmjImportReport { pub map: u16, pub layers: usize, pub cells: usize, pub notes: Vec<String>,
    pub errors: Vec<TmjError>, pub change: Option<ChangeId>, pub bake: Option<WorldBakeReport> }
/// Reads `path` against world map `map` without changing anything.
pub fn world_inspect_tmj(store: &mut Store, map: u16, path: &Path) -> Result<TmjImportReport, KernelError>;
/// One undo step import_label(map); errors → KernelError::Refused with the first error and the count;
/// equal to the world → no step (`change: None`).
pub fn world_import_tmj(store: &mut Store, map: u16, path: &Path) -> Result<TmjImportReport, KernelError>;
```

Texte (je `TmjError.message`, wörtlich): „Keine Tiled-Karte (`type` ist nicht „map“).“, „Die Karte ist {w} × {h} Kacheln groß; eine Karte hat 32 × 32.“, „Kacheln sind {w} × {h} Pixel; das Studio schreibt 16 × 16.“, „Unendliche Karten liest das Studio nicht.“, „Komprimierte Ebenen liest das Studio nicht; in Tiled unter Karte › Eigenschaften „Kachelebenenformat“ CSV wählen.“, „Ausgelagerte Tilesets (`source`) liest das Studio nicht; in Tiled das Tileset einbetten.“, „Die Datei nutzt Paket {a}, die Welt Paket {b}.“, „Das Tileset hat {n} Kacheln, Paket {p} hat {m}.“, „GID {g} gehört nicht in diese Ebene.“, „Sechseck-Drehung gibt es nicht.“, „Objekt {id} liegt nicht auf dem Raster.“, „Objekt {id}: Kachel {t} ist kein Teilmodell.“, „Zelle ({x}, {z}) ist zweimal überschrieben.“, „Die Ebene „Höhe“ fehlt.“, „Zelle ({x}, {z}) hat keine Höhe.“.

- [ ] **Step 1: Scheiternde Tests:** `round_trip_is_identity` (`tmj_world`, beide Varianten, `one_tile_world`, `checkerboard_world`: `from_tmj(to_json(to_tmj(w)), w) == w`), `prop_round_trip_is_identity` (proptest, 128 Fälle: Zufallswelt mit 1–3 Ebenen, Drehungen und Spiegeln, Höhen, Rampen, Teilmodellen, Überschreibungen, `derive` beides), `import_of_an_unchanged_export_is_byte_equal_on_disk` (**Gate**: `world_ready()`, gemalte Welt aus `gate_world`, `save`, Datei-Bytes A; Export, Import, `save`, Datei-Bytes B; `A == B`; `change == None`), `an_edited_cell_imports_as_one_step` (eine GID in der Datei geändert → eine Zelle anders, ein Verlaufseintrag, ⌘Z → Bytes A), `a_tmj_with_tiled_extras_reads` (Review Focus: Datei mit `class`, `parallaxx`, `tintcolor`, `locked`, Objekt-`template`-freien Zusatzfeldern → gleich), `base64_reads_and_compression_is_refused` (Review Focus), `an_external_tileset_is_refused`, `a_new_layer_without_properties_is_an_overlay` (Review Focus), `an_object_off_the_grid_is_named` (Review Focus), `foreign_sizes_and_packages_are_refused` (je Text oben), `gids_past_the_tileset_are_refused`, `buildings_are_ignored_with_a_note` (verschobener Punkt → Hinweis, Gebäude unverändert), `freeform_and_terrain_stay_from_the_target` (Form-1-Welt: Import behält `freeform[0].nsbmd` und `terrain`), `import_refuses_a_binary_map`, `inspect_changes_nothing`.
- [ ] **Step 2: Lauf** `cargo test --workspace --all-features -- world::tmj` → FAIL; **Step 3: Umsetzen** (eigener Leser über serde, `#[serde(default)]` für fehlende Tiled-Felder, keine `deny_unknown_fields`); **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(world): import a Tiled map (TMJ) into a world map as one step; export and import round-trip byte-equal` (mit Trailer).

### Task 8: Befehle: Tauri, Brücke, Zugriff, Szene, `docs/mcp.md`

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/world_map.rs` (`*_impl` + `#[tauri::command]` angehängt; `get_world_mesh_impl` über `world_faces`; `world_adopt_impl` mit `find_apicula()`), `commands/dispatch.rs` (Namen in `COMMANDS`, Arme, `confine_source` für `path` der Lesebefehle; angehängt), `src-tauri/src/lib.rs` (`generate_handler!`), `mcp/access.rs` (Einträge angehängt; Liste der `Depends`-Pfade um `("world_export_tmj", "path")`), `docs/mcp.md` (Abschnitt „Freiform, Form 2, Tiled (6.5b, über `g4_call`)“ am Ende)

**Interfaces:**
- Consumes: Tasks 2, 4, 5, 6, 7.
- Produces (Argumente wie Tauri, camelCase; Art in Klammern):

```rust
pub fn world_freeform_inspect_impl(state: &AppState, req: FreeformAddRequest) -> Result<FreeformInspectDto, CmdError>;   // Read
pub fn world_freeform_add_impl(state: &AppState, req: FreeformAddRequest) -> Result<FreeformReport, CmdError>;           // Write
pub fn world_freeform_remove_impl(state: &AppState, map: u16, index: usize) -> Result<WorldReport, CmdError>;            // Write
pub fn tileset_piece_set_impl(state: &AppState, req: PieceSetRequest) -> Result<PieceSetReport, CmdError>;               // Write
pub fn world_export_tmj_impl(state: &AppState, map: u16, path: String) -> Result<TmjExportReport, CmdError>;             // Depends (path)
pub fn world_inspect_tmj_impl(state: &AppState, map: u16, path: String) -> Result<TmjImportReport, CmdError>;            // Read
pub fn world_import_tmj_impl(state: &AppState, map: u16, path: String) -> Result<TmjImportReport, CmdError>;             // Write
// FreeformInspectDto { ok: bool, report: Option<FreeformReport>, refusal: Option<PartMessage>, notes: Vec<PartMessage> }
// MeshTextureDto for a texture that is no tile: size from pack_textures, role "piece"
```

- [ ] **Step 1: Scheiternde Tests** (unten in `world_map.rs`, Fixture `world_ready()` mit `write_samples`): `freeform_commands_over_dispatch` (`world_freeform_inspect` → `ok`; `world_freeform_add` → `get_world_mesh` hat Flächen der Ebene 254 und eine Textur `g4_imp_cube` mit Größe 16 × 16; `world_bake` `ok`; `undo`), `form2_over_dispatch_with_apicula` (nur mit `G4_APICULA`, sonst `skipped`: synthetisches gebackenes Modell als Karte 0, `world_adopt {form: "gltf"}` → `form2` gesetzt), `tmj_commands_over_dispatch` (Export in `build/`, Inspect ohne Fehler, Import → `change: null`), `world_export_tmj_outside_build_is_irreversible` (Klasse wie `export_map_part`), `piece_set_over_dispatch`, `new_commands_refuse_in_the_dspre_layout` (`WORLD_NEEDS_TEXT_PROJECT`); bestehend `every_command_has_a_kind`, Gleichheit `COMMANDS`/`generate_handler!`, `every_command_accepts_the_keys_api_ts_sends`, `the_curated_set_is_the_decided_one` unverändert.
- [ ] **Step 2: Lauf** `cargo test -p studio -- world_ freeform tmj piece` → FAIL; **Step 3: Umsetzen**; `docs/mcp.md`: die sieben Befehle mit Art, Argumenten, Antwort, Ablehnungen, `world_adopt` mit `form: "gltf"` und `form2`, die Regel „Beifile = kanonisches GLB“ in einem Satz mit Verweis auf `world_map.md`.
- [ ] **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(studio): commands for free-form models, piece models, form 2 and Tiled exchange` (mit Trailer).

### Task 9: Oberfläche: Import-Dialog, Export-Dialog, Form 2 in „Quelle“

**Files:**
- Modify: `apps/studio/src/api.world.ts` (Typen und Aufrufe, angehängt), `apps/studio/src/workbench/inspector/{ImportPartsDialog.tsx, importParts.ts, ExportPartDialog.tsx, WorldSourceSection.tsx}` und ihre Tests

**Interfaces:**
- Consumes: Task 8.
- Produces:

```ts
// api.world.ts
export interface FreeformReport { map: number; glb: string; stats: { polygons: number; quads: number; triangles: number; dropped: number };
  textures: TexturePreview[]; notes: PartMessage[]; change: number | null; bake: WorldBakeReport | null }
export interface FreeformInspect { ok: boolean; report: FreeformReport | null; refusal: PartMessage | null; notes: PartMessage[] }
export interface TmjError { path: string; message: string }
export interface TmjExportReport { map: number; path: string; layers: number; images: number; pieces: number; overrides: number }
export interface TmjImportReport { map: number; layers: number; cells: number; notes: string[]; errors: TmjError[]; change: number | null; bake: WorldBakeReport | null }
export const worldFreeformInspect = (req: { map: number; path: string; at?: [number, number, number]; options?: ImportOptions }) => invoke<FreeformInspect>("world_freeform_inspect", { req });
export const worldFreeformAdd = (req: { map: number; path: string; at?: [number, number, number]; options?: ImportOptions }) => invoke<FreeformReport>("world_freeform_add", { req });
export const worldExportTmj = (map: number, path: string) => invoke<TmjExportReport>("world_export_tmj", { map, path });
export const worldInspectTmj = (map: number, path: string) => invoke<TmjImportReport>("world_inspect_tmj", { map, path });
export const worldImportTmj = (map: number, path: string) => invoke<TmjImportReport>("world_import_tmj", { map, path });
// worldAdopt(map, form?: "verbatim" | "gltf")  (Parameter angehängt, Vorgabe "verbatim")
```

Texte und Attribute: Import-Dialog einer Welt-Karte (`[data-import-dialog]` mit `data-world="true"`): Dateien `.glb`/`.gltf`/`.obj` ergeben die Zeile „Freiform (glTF)“ (`[data-import-part="freeform"]`) mit den Vorschauzeilen von 6.5a („glTF-Modell umgewandelt: …“, Texturen, Budget aus `bake.model_bytes`) und darüber die Zeile „Lage“ mit drei Zahlenfeldern „x“, „y“, „z“ (Welteinheiten, Hilfe „16 = eine Kachel; 0, 0, 0 = Kartenmitte“); `.tmj` ergibt „Tiled (TMJ)“ (`[data-import-part="tmj"]`) mit „{n} Ebenen, {m} Zellen geändert“ bzw. „Keine Änderung.“ oder der Fehlerliste (erste drei Fehler, „… und {k} weitere“); andere Teile sind bei Welt-Karten nicht wählbar mit dem Satz „Bei Welt-Karten entstehen Modell, Terrain und Permissions beim Backen.“. Export-Dialog einer Welt-Karte: Segment „Tiled (TMJ)“ (`[data-export-part="tmj"]`), Dateiname `NNNN.tmj`, Satz „Kachelbilder kommen in den Ordner NNNN.tiles daneben.“. „Quelle“ (binär, Textprojekt): zweite Taste „Als glTF übernehmen (Form 2)“ (`[data-world-adopt="gltf"]`), danach Notice „Übernommen als glTF: {v} Vanilla-Polygone, {b} gebackene Flächen.“; Ablehnung als Notice (Fehler) mit dem Satz.

- [ ] **Step 1: Scheiternde Tests:** `ImportPartsDialog.test.tsx` (`a_world_map_offers_freeform_for_glb_obj_and_gltf`, `freeform_sends_at_and_options`, `tmj_shows_changes_or_errors`, `other_parts_are_disabled_for_world_maps`), `ExportPartDialog.test.tsx` (`a_world_map_exports_tmj`), `WorldSourceSection.test.tsx` (`adopt_as_gltf_shows_the_polygon_counts`, `adopt_as_gltf_refusal_is_a_notice`), `importParts.test.ts` (`world_codes_have_sentences`: `world.side-file`, `world.adopt-texture`, `tmj.buildings-ignored`).
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Check-Skripte grün.
- [ ] **Step 5: Browser-Teil** (Test-Umgebung `G4_VITE_PORT=1580 G4_BRIDGE_PORT=1581`, Kopie `6-5b-web` von Platin DE mit `.nds`, `target/debug/g4k import-samples <Kopie>/import`, Textprojekt über `call.sh open_project`, `assign_base`, `adopt_text_project`; eine Welt-Karte über `world_create` auf der Karte des ersten Headers von Zweiblattdorf): Inspector › „Aktionen“ › „Teile importieren…“ → Pfad `<Kopie>/import/g4_imp_cube.obj`, Lage 0, 0, 0 → Zeile „Freiform (glTF)“, übernehmen → Szene zeigt den Würfel (`[data-world-mesh]`), Verlauf „Freiform hinzugefügt: Karte NNNN (NNNN.g4_imp_cube.glb)“, ⌘Z → weg, ⇧⌘Z; Export „Tiled (TMJ)“ nach `build/NNNN.tmj`, dann Import derselben Datei → „Keine Änderung.“; eine zweite Karte (binär) → „Als glTF übernehmen (Form 2)“ → Notice mit Polygonzahlen, Szene zeigt das gebackene Modell. Hell, Dunkel, Kompakt; Bilder unter `target/agent/gate-<datum>/6-5b/blick-*.png`, Notiz `browser.md`.
- [ ] **Step 6: Commit** `feat(studio): free-form and Tiled in the import and export dialogs, adopt as glTF` (mit Trailer).

### Task 10: Gated Tests auf Platin DE, HeartGold DE, HeartGold US

**Files:**
- Create: `crates/g4-kernel/tests/world_freeform.rs`
- Modify: `crates/g4-kernel/tests/world_bake.rs` (nur ein Aufruf: TMJ-Round-Trip aller Form-1-Karten, siehe unten), `docs/dev-setup.md` (Tabelle `G4_TEST_PROJECT`: `tests/world_freeform.rs`)

**Interfaces:**
- Consumes: Tasks 2–7; `vanilla_copy()`, `G4_TEST_PROJECT`, `G4_APICULA`, `tests/common/glb.rs`.
- Produces: die Belegzeilen `world-freeform …`, `world-piece …`, `world-form2 …`, `world-form2-sweep …`, `world-tmj …` für die Gate-Datei.

- [ ] **Step 1: Tests:**
  - `freeform_on_a_painted_map_bakes_and_apicula_reads_it` (Karte des ersten Headers von Zweiblattdorf bzw. Neuborkia; `tileset_save`, `world_create`, `world_edit` mit den Zellen von `gate_world(…, "0000", "0001", "0002")`, `world_freeform_add` mit `g4_imp_cube.obj` (`unit` 32) bei `at [0, 0, 0]`; verlangt `bake.ok`, Modell ≤ 0xF000, Flächen der Ebene 254 = 6; mit `G4_APICULA`: Exit 0, genau eine GLB, `glb_set` ⊇ `input_set` der Freiform-Flächen; druckt `world-freeform <code> map=<n> faces=<n> freeform_faces=<n> nsbmd=<n> share=<f> apicula=<ok|skipped>`).
  - `piece_model_on_the_copy` (`tileset_piece_set` mit dem Würfel auf einer freien Kachel des Pakets, ein Stück; `ok`; druckt `world-piece <code> pack=<n> tile=<n> faces=<n> nsbmd=<n>`).
  - `form2_of_a_vanilla_map_bakes_apicula_readable` (**Gate**, braucht `G4_APICULA`, sonst `skipped` und der Test zählt im Gate nicht: dieselbe Karte, `world_adopt_with {form: gltf}`; verlangt `bake.ok`, BDHC und Permissions byte-gleich dem Original, apicula liest das gebackene Modell (Exit 0, eine GLB), Ecken- und UV-Menge der GLB gleich `input_set` der gebackenen Flächen; druckt `world-form2 <code> map=<n> vanilla_polys=<n> vanilla_quads=<n> vanilla_tris=<n> baked_faces=<n> baked_quads=<n> baked_tris=<n> strips=<n> dropped=<n> vanilla_bytes=<n> baked_bytes=<n> share=<f> ignored=<liste>`).
  - `form2_sweep_over_every_map` (nur mit `G4_FORM2_SWEEP=1` und `G4_APICULA`, sonst `skipped`; je Karte Form 2 auf einer frischen Store-Instanz; zählt `ok`, `bake.model-too-big`, `world.adopt-texture`, andere; druckt `world-form2-sweep <code> maps=<n> ok=<n> too_big=<n> texture=<n> other=<n> polys_ratio_median=<f> ms=<n>`; keine Schwelle, Beleg für die Gate-Datei).
  - `tmj_round_trip_of_the_painted_map_and_every_adopted_map` (gemalte Karte von oben: Export, Import, `save` → Datei byte-gleich; dann je Karte Form 1 übernehmen (6.3b-Weg), Export, Import → `WorldJson` gleich; druckt `world-tmj <code> painted=equal maps=<n> equal=<n> ms=<n>`).
- [ ] **Step 2: Kopien** `6-5b-pt`, `6-5b-hgde`, `6-5b-hgus` (je `_DSPRE_contents` mit `.nds` daneben).
- [ ] **Step 3: Lauf** je Kopie nacheinander im Hintergrund: `G4_ROMS_DIR=$R G4_TEST_PROJECT=$K/6-5b-<k>_DSPRE_contents G4_APICULA=… cargo test --release --workspace --all-features --test world_freeform -- --nocapture --test-threads 1 > target/agent/6-5b/gated-<k>.log 2>&1` → PASS; einmal je Spiel mit `G4_FORM2_SWEEP=1` (Log `sweep-<k>.log`); dann `--test world_bake`, `--test textproject` und `--test vanilla_checks` je Kopie (Fehler 0, Byte-Gleichheit von Form 1 unverändert).
- [ ] **Step 4: Commit** `test(world): free-form, piece model, form 2 and TMJ round-trip on the three games` (mit Trailer).

### Task 11: Szenarien `pt-freiform` und `hg-freiform`

**Files:**
- Create: `scripts/agent/scenarios/pt-freiform.yaml`, `scripts/agent/scenarios/hg-freiform.yaml`

**Interfaces:**
- Consumes: `pt-gemalt.yaml`/`hg-gemalt.yaml` (6.3b), `make-import-samples` (6.5a), Brückenbefehl `world_freeform_add`, `world_bake`, `get_world_mesh`.

- [ ] **Step 1: Szenarien** als Kopie von `pt-gemalt.yaml`/`hg-gemalt.yaml` (gleicher Patch, `adopt_text_project`, `create_area` neben Zweiblattdorf bzw. Neuborkia, Tileset, gemalte Zellen, Tür) mit drei Zusätzen: `prepare: [[make-import-samples, {path: import}]]`; nach dem letzten `world_edit` `g4_call world_freeform_add {req: {map: …, path: import/g4_imp_cube.obj, at: <A>, options: {unit: 32}}}` (`as: cube`) und `world_bake` mit `expect` `ok: true`, `freeform_faces: 6`; im Spiel nach dem Eintritt `expect-at` am Startpunkt, `shot freiform`. `<A>` so, dass der Würfel (zwei Kacheln Kante) zwei Kacheln nördlich des Startpunkts der Karte steht (Welteinheiten ab Kartenmitte: Zelle `(x, z)` → `(16x + 8 − 256, 16z + 8 − 256)`); Rechnung und Herkunft als Kopfkommentar.
- [ ] **Step 2: Lauf** je Spiel auf frischen Kopien `6-5b-szen-pt`, `6-5b-szen-hg` (mit `.nds`), Debug-Binaries aus `cargo build --workspace --all-features`: `uv run g4emu scenario ../agent/scenarios/pt-freiform.yaml --project $K/6-5b-szen-pt_DSPRE_contents --out $W/target/agent/gate-$(date +%F)/6-5b/pt-freiform <--set wie pt-gemalt> > target/agent/6-5b/pt-freiform.log 2>&1`, ebenso `hg-freiform` → Rückgabe 0, `"ok": true`. Blick des Agenten auf `freiform`: Würfel mit Schachbrett-Textur über dem gemalten Boden, Boden und Gras wie in `pt-gemalt` (Rot/Blau-Tausch beachten, falls noch nicht behoben).
- [ ] **Step 3: Commit** `test(g4emu): scenarios pt-freiform and hg-freiform show a free-form cube on a painted map` (mit Trailer).

### Task 12: Doku der Formate und Design-System-Nachtrag

**Files:**
- Create: `docs/formats/tmj.md`, `design-system/openpencil/NACHTRAG-6-5b.md`, `design-system/openpencil/beispiele/welt-austausch.body.js`
- Modify: `docs/formats/world_map.md` (Abschnitte „Freiform und Beifiles“, „Teilmodelle mit Geometrie“, „Form 2“, Prüfungen `world.side-file`, `world.adopt-texture`, `world.verbatim-mixed` neu, „Was später kommt“), `docs/formats/tileset.md` (`model` erlaubt, `tileset_piece_set`, Beifile-Ordner), `docs/formats/model_import.md` (Satz „Freiform in Welt-Karten: world_map.md“), `docs/formats/project_git.md` (`*.png binary`, `*.glb binary`), `docs/formats/build_state.md` (Eingabe-Hash mit Beifiles, `WORLD_BAKE_REV` 2), `apps/studio/src-tauri/src/mcp/docs.rs` (`FORMATS`: `tmj` an der alphabetischen Stelle), `design-system/README.md` (Abschnitt „Nachtrag 6.5b“ am Ende)

- [ ] **Step 1: Formate:** `tmj.md` mit Zweck („Austausch mit Tiled, kein Weltformat“, Gutachten C §5.4), Abbildungstabelle Welt ↔ TMJ (Ebenen, GIDs, Flip-Tabelle, Eigenschaften `g4.*`, Objekt-Ebenen, Tilesets, Bilder), Import-Regeln und Texte, Round-Trip-Zusage, Lizenzsatz („nur das dokumentierte Format; kein Code aus Tiled“), synthetisches Beispiel aus `one_tile_world` (gekürzt und gesagt). `world_map.md`: Beifile-Form (kanonisches GLB, Einheit 16, Achsen, `at`), Ebenen 254/255, Kulissen-Regel, Texturregel, Form 2 mit Polygonzahl-Abweichung (Zahlen aus Task 10 eingesetzt). Jede Zahl und jeder Wortlaut aus dem Code.
- [ ] **Step 2: README** „Nachtrag 6.5b“ nach dem Muster „Nachtrag 6.5a“: Zeilen „Freiform (glTF)“ und „Tiled (TMJ)“ im Import-Dialog mit „Lage“, Segment „Tiled (TMJ)“ im Export-Dialog, Taste „Als glTF übernehmen (Form 2)“ in „Quelle“, Texte und `data-*`, Bilder unter `target/agent/gate-<datum>/6-5b/`; keine neuen Tokens, keine neue Komponente.
- [ ] **Step 3: Anleitung** `NACHTRAG-6-5b.md` nach `NACHTRAG-6-5a.md` (Frames „Teile importieren: Freiform und Tiled“ und „Quelle: Als glTF übernehmen“, Hell und Dunkel, Vorlage `welt-austausch.body.js` gegen `jsx-lib-app.js`, mit einer Attrappe der Bausteine fehlerfrei durchgelaufen); die `.fig` wird nicht angefasst (Satz „Der Frame wird auf `main` nach dem Merge gerendert, sobald OpenPencil antwortet“).
- [ ] **Step 4:** `bash scripts/check-tokens.sh` grün; `cargo test -p studio table_covers_every_format_file` grün.
- [ ] **Step 5: Commit** `docs(formats): free-form side files, piece models, form 2, Tiled exchange; design-system addendum 6.5b` (mit Trailer).

### Task 13: Gate 6.5b, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-6-5b-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 6.5b“ am Ende), `docs/ROADMAP.md` (Zeile 6 der Übersicht, „Stand 6.5b“ unter der Tabelle)

- [ ] **Step 1: Gate-Datei** nach `docs/phase-6-3a-gate.md`: Kopf mit Stand und Commit; Gate-Text (wörtlich wie in Global Constraints); „So gelesen“ (Beifile kanonisch statt der Nutzerdatei, Freiform ist Kulisse für BDHC und Permissions, Form 2 hält das Terrain verbatim, TMJ ohne Freiform und Gebäude-Import); Legende „Wer“; Namen in der Oberfläche; Vorbereitung (Kopien `6-5b-{pt,hgde,hgus}`, `6-5b-szen-{pt,hg}`, `6-5b-web`; Worktree, Ports 1580/1581, Last).
  - **(1) Automatik (Agent):** `cargo deny --locked check licenses advisories` (keine neue Abhängigkeit), fmt, clippy, `env -u G4_TEST_PROJECT cargo test --workspace --all-features`, `pnpm test`/`pnpm build`, Stil, Tokens, Tiefe, ROM-Daten, `uv run pytest -m "not integration" -q` in `scripts/g4emu`; gated Task 10 je Kopie, Sweep je Spiel; `vanilla_checks` je Kopie. `scripts/gate.sh`: **offen, Lauf in der Hauptsitzung auf main** (die neuen Tests laufen in den bestehenden Schritten mit, `gate.sh` bleibt unverändert; der Sweep läuft dort nicht).
  - **(2) Szenarien (Agent + Blick):** Task 11 mit Rückgabe, Erwartungen und Bild `freiform`.
  - **(3) Browser (Agent):** Task 9 Step 5.
  - **(4) Hand und Blick:** Bilder; `out.nds` von `pt-freiform` in melonDS ansehen; eine eigene Blender-Datei als Freiform und als Teilmodell; die exportierte TMJ in Tiled 1.10 öffnen, eine Zelle malen, speichern, importieren (ein Verlaufseintrag, Szene folgt); DSPRE öffnet eine Form-2-Karte.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Freiform backt (B, Anteil) | apicula | Teilmodell | Form 2 Karte | Vanilla-Polygone (V/D) | gebackene Flächen (V/D, Streifen) | Bytes Vanilla → gebacken | Sweep ok / too-big / Textur / andere | TMJ gemalt byte-gleich | TMJ Form-1-Karten gleich | `vanilla_checks` Fehler | Szenario | Last |
    |---|---|---|---|---|---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | | | | / 666 | 0 | | |
    | HeartGold DE | | | | | | | | | | / 676 | 0 | | |
    | HeartGold US | | | | | | | | | | / 676 | 0 | – | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte eintragen; Befunde beheben (eigene Commits `fix(6.5b): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: `agent-gate.md`** Abschnitt „Phase 6.5b“ nach „Phase 6.5a“: Kopien, Worktree, „Wo was ist“ (`[data-import-part="freeform"|"tmj"]`, `[data-export-part="tmj"]`, `[data-world-adopt="gltf"]`, Befehle über `g4_call --expert`), Tabelle Punkt / Wer / Wie, die Befehle für (1), (2), (3) mit erwarteten Werten. **Roadmap:** „**Stand 6.5b <Datum>:** Gate vom Agenten bestanden: Freiform-Würfel backt auf einer gemalten Karte (<n> B, <Anteil>), apicula liest, in g4emu sichtbar (`pt-freiform`/`hg-freiform`); TMJ-Round-Trip byte-gleich (gemalte Karte, alle 666 / 676 / 676 übernommenen Karten); Form 2 für Zweiblattdorf bzw. Neuborkia backt apicula-lesbar (Polygone Vanilla <v> → gebacken <b>, Sweep <ok>/<n>); Branch `phase-6-5b-freiform-austausch`; [docs/phase-6-5b-gate.md](phase-6-5b-gate.md); beim Nutzer offen: Blick, Tiled, Blender, melonDS, DSPRE, `scripts/gate.sh` auf main“.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`; `git log main..HEAD | grep -c '＠'` gleich 0.
- [ ] **Step 5: Commit** `docs(6.5b): gate of free-form and exchange, agent-gate section, roadmap` (mit Trailer).

---

## Schnittstellen für die Folgephasen

- **6.4a (Malen, parallel):** `world_faces` (Szene mit Freiform und Teilmodell-Geometrie; die Live-Vorschau kann `mesh_with` mit `PieceModels` aus `load_side_models` nutzen), `FREEFORM_LAYER`, `tileset_piece_set`/`TileChange.model` (Tileset-Editor), `world_export_tmj`/`world_import_tmj` (Menüpunkte in Bauen, wenn 6.4a sie will).
- **6.4b (Smart Drawing):** Wang-Sätze im TMJ sind bewusst nicht abgebildet; käme ein Austausch der Kantensätze, gehört er als `wangsets` in das erste Tileset von `tmj.rs`.

## Offen für die Umsetzung

- **apicula-Ausgabe für Vanilla-Karten** (Task 5 Step 1): Materialnamen, Skins, Eckenfarben und Einheit sind am Vanilla-Modell noch nicht gemessen; die Regel „MDL0-Material zuerst“ fängt abweichende Bildnamen ab, ein Skin mit verschobenen Ecken nicht (dann zeigt `form2_of_a_vanilla_map_bakes_apicula_readable` die Abweichung an der Eckenmenge; Abhilfe in `read_gltf` ist ein Befund mit eigenem Commit).
- **Form 2 über 0xF000:** dichte Vanilla-Modelle mit `VTX_DIFF` und Dreiecks-Strips können neu gebacken größer werden; der Sweep zählt sie, Form 1 bleibt für diese Karten der Weg.
- **Tiled-Fassung:** geschrieben wird für 1.10; ob ältere Tiled-Fassungen die Bildsammlung ohne `grid` lesen, prüft der Hand-Punkt.
- **Kachelbilder mit Format 5** (4 × 4) dekodiert `tileset_texture` nicht: Platzhalter im TMJ und in der Szene, bis 6.4a den Dekoder bringt.
- **Parallele Merges:** Konflikte an den Anhängepunkten gesammelt lösen (Opus-Untersitzung bei vielen, Erfahrung „Parallele Merges“); `.fig` von main behalten; die Zeile in `get_world_mesh_impl` von 6.5b nehmen.
