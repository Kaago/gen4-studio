# Phase 6.3b „Backen und Build“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eine Welt-Karte des Textprojekts wird gebacken: aus dem Weltdokument entstehen Modell (Flächenliste des Vernetzers → NSBMD über den Bäcker von 6.1), Terrain (Höhenraster → BDHC über den Generator von 6.2) und Permissions (Vorgaben der Kacheln, Kantenregel, Handwerte) im Member `unpacked/<land_data>/NNNN`; der Store hält den gebackenen Member als Map-Asset (jeder Strich backt im selben Undo-Schritt), der neue Build-Schritt `world` backt inkrementell je Karte, ein außen geänderter Member ist der Konflikt `world.member-changed` mit den Wegen „Neu backen“ und „Als binäre Karte übernehmen“, „Karte als Quelle übernehmen“ (Form 1, verbatim) macht jede binäre Karte zur Welt-Karte, die byte-gleich backt; eine per Befehl gemalte Karte ist in g4emu begehbar (`pt-gemalt`, `hg-gemalt`).

**Architecture:** Neues Kernel-Modul `world/bake.rs` (rein: Permissions, `bake_grid`, `assemble`; mit Store: `bake_world`), `world/record.rs` (Datensatz je Karte im Abschnitt `world` von `.g4/build/state.json`), `world/build.rs` (`bake_world_maps`, der Schritt `world`), `world/adopt.rs` (Form 1), `bake/decode.rs` (Rückleser der Bäcker-Teilmenge für Tests). Das Map-Asset einer Welt-Karte ist im Textprojekt die Ausgabe des Backens: der Store liest es, indem er die Karten-JSON liest und backt; `Tx::edit_world` backt im selben Schritt neu; Speichern backt jede schmutzige Welt-Karte, schreibt die JSON ohne abgeleitete Teile und den Member und zeichnet beide auf. `textproject` lässt Welt-Karten aus, `world` läuft direkt danach (vor `rotom`). Die Karten-Datei bekommt Schema `maps.world` 2 (Welt-Karten ohne `permissions`/`model`/`bdhc`, `freeform[].nsbmd` als Teil, neues `world.terrain`), Textprojekt-Schema 4 mit Migration.

**Tech Stack:** Rust (g4-kernel, g4-formats: `bake`, `nsbmd`, `bdhc::from_grid`, `terrain`), serde/serde_json, proptest, xxhash (vorhanden), Tauri-Befehle und Brücke (MCP über `g4_call`), React 19, vitest, Python/uv für `scripts/g4emu`, apicula als Orakel.

**Spec:** [2026-11-05-phase-6-kartenerstellung-design.md](../specs/2026-11-05-phase-6-kartenerstellung-design.md) §1 (harte Grenzen), §3.1, §3.3, §3.4, §3.5, §4 Zeile 6.3b und „Phasen-Gate 6“ (wörtlich unten), §5 („Modell und BDHC weichen ab“, „Store-Modell“), §6, §7, §8; [entscheidungen.md](../../research/2026-11-05-phase-6/entscheidungen.md) (Block „Writer“: Stil je Gebiet, Eingabe des Bäckers; Block „BDHC“: Terrain gemalter Karten, Kollisionsableitung, obere Zelle; Block „Malen, Weltformat“: Build, Store, „Karte als Quelle übernehmen“; „Phasenschnitt“ Zeile 6.3b; „Nicht in Phase 6“); [Gutachten C](../../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §5.3 (Schritt, Form 1, Prüfstein); Vorgänger-Pläne [6.1](2026-11-05-phase-6-1-writer.md), [6.2](2026-11-05-phase-6-2-bdhc.md), [6.3a](2026-11-05-phase-6-3a-weltformat.md) (Form und Schnittstellen).

Stand 06.11.2026: `main` bei `1af7783c` (6.1 Writer, 6.2 BDHC, 6.3a Weltformat gemergt). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-3b`, Branch `phase-6-3b-backen-build` (Basis `1af7783c`). Kernel `crates/g4-kernel/src`, Formate `crates/g4-formats/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Voraussetzungen (auf main `1af7783c`, vor Task 1 mit grep prüfen)

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `bake::{bake, Face, BakeInput {style, textures, vram_bytes, vram_limit}, BakeReport, BakeMessage, BakeCode, Level, AreaStyle::of, PackTexture, pack_textures, area_vram, vanilla_vram_max, bake_context}` | `bake/mod.rs`, `bake/textures.rs`; `bake_context` nimmt `map_import::map_areas(store, map)?.first()` und lehnt ohne Header ab | 2, 4 |
| `bake::strips::normal` (`pub(crate)`, (c1−c0)×(c2−c0), Rückfall (c2−c0)×(c3−c0)) | Normalen der Strips | 2 (nur lesen) |
| `world::mesh::{MeshFace, mesh, mesh_textures, estimate_model_bytes, PIECE_LAYER, MODEL_BUDGET}`, `world::grid::{WorldGrid::{parse, height_grid}, Ramp, lex_height, lex_ramp}`, `world::tileset::{Tileset, TileJson {role, kind, collision, sound, wall, footprint}, TileRole}` | `MeshFace` hat dieselben Felder wie `bake::Face`; `height_grid()` gibt `bdhc::HeightGrid` | 1, 2 |
| `world::doc::{WorldJson, FreeformJson {glb: Option<String>, nsbmd: Option<String>, at}, WORLD_VERSION = 1, fixtures::*}`, `world::edit::{world_create, world_edit, WorldReport {map, tileset, cells, change}, WORLD_NEEDS_TEXT_PROJECT, WORLD_HAS_NO_ORIGINAL}`, `world::keep::{world_field, with_world, carry_world}`, `world::check::{world_problems, WORLD_NOT_BAKED_CODE, WORLD_BUDGET_CODE}`, `world::places::packs_of_map` | 6.3a | 3, 4, 7 |
| `g4_formats::bdhc::{Bdhc::{read, write, height_at, payload_len}, from_grid, HeightGrid::edge_units, tile_center, Direction}` | 6.2 | 1, 2, 7 |
| `terrain::{derive_permissions, apply_overrides, map_heights, edge_rule, kind_rule}`, `terrain/budget.rs::area_of_map` (privat, Regel „erster Header in Tabellenordnung“), `terrain/rows.rs` importiert `world::grid::{lex_height, lex_ramp}` | 6.2 / 6.3a | 1 |
| `textproject::schema::map::{MapJson {source, world, permissions, buildings, buildings_tail?, model, bdhc, bgs?}, map_bytes, map_json}`, `textproject::part::{Part, part_out, part_in_at, sha1_hex}`, `entity_part` | `permissions`, `model`, `bdhc` Pflicht | 3 |
| `textproject::{SCHEMA_VERSION = 3, kinds::PART_SCHEMAS = [("maps.world", 1)], migrate::{MIGRATIONS, Tree, v2_to_v3}}`, `textproject/schema.snap` | 6.3a | 3 |
| `store::{Store::{map, world, tileset, transaction, edit_map, save, entity_bytes, read_disk_as, source}, Tx::{edit_map, edit_world}, Loaded::{Map, World}}`, `Location::Derived` | `Tx::edit_world` ändert nur das WorldMap-Asset | 4 |
| `build::{prepare, prepare_steps, clear_cache}`, `build::step::{Step, Dirty, StepRun}`, `build::state::{BuildState {steps, texts, rom, sync}, BuildState::{update, clear_keeping_texts}}` | `clear_cache` behält `texts` und `sync` | 5 |
| `textproject::step::TextProjectStep`, `textproject::import::{plan_import, apply_import}` | Einheiten je Entität | 5 |
| `textproject::sync::{scan, apply_one_sided, resolve, Keep {Studio, Dspre}, ConflictEntry {key, label, assets, unsaved, other}}`, `sync/watch.rs::Of` | Abgleich 4.3b | 6 |
| Tauri `commands::{dispatch::COMMANDS, world_map.rs (world_get_impl, get_world_mesh_impl, WorldGetDto), terrain.rs (get_map_heights_impl), textproject (resolve_external_conflict_impl)}`, `mcp/access.rs::COMMAND_KINDS` mit `every_command_has_a_kind`, `mcp/docs.rs::FORMATS` (Test `table_covers_every_format_file`) | Anhängepunkte | 8 |
| Frontend `api.world.ts`, `world/scene/worldMesh.ts::mapModel`, `technik/{stepWords.ts, TextProjectSurface.tsx (ConflictBody)}`, `workbench/inspector/{MatrixCellEditor.tsx, MapBudgetSection.tsx}`, `api.textproject.ts::ConflictEntry` | 6.2 / 6.3a / 4.3b | 10 |
| `scripts/agent/scenarios/{pt-gelaende,hg-gelaende}.yaml` (Vorlage: `create_area`, Begegnung, Geschenk, `build: out.nds`), `scripts/g4emu` Schritte `expect-height`, `walk-until-blocked`, `walk-until-battle`, `expect-battle`, `shot`; Brückenbefehl `adopt_text_project` | 6.2 | 11 |
| `crates/g4-kernel/tests/bake.rs` Helfer `glb_chunks`, `glb_set`, `input_set`, `apicula()` | 6.1 | 9 |

Fehlt eine Zeile oder weicht sie ab, zuerst die Abweichung in diese Tabelle schreiben und den betroffenen Task anpassen, dann beginnen.

## Parallele Teilphase 6.5a und Anhängepunkte

6.5a läuft parallel im Worktree `phase-6-5a` (Ports 1500/1501). Es führt `bake::Face::is_triangle` (c3 == c2) und `BakeReport.triangles` ein und erweitert `map_import` um `Part::Textures`, `inspect_with`, `import_with`. Beide Teilphasen berühren:

| Datei | 6.3b tut | Regel |
|---|---|---|
| `crates/g4-kernel/src/bake/mod.rs` | hängt `mod decode;`, `pub use decode::{BakedQuad, decode_quads};` und `pub const BAKE_REV: u32 = 1;` ans Ende der Deklarationen an | nur anhängen; ändert 6.5a die Bytes eines Vierecks, hebt der zweite Merge `BAKE_REV` |
| `crates/g4-kernel/src/bake/decode.rs` | neu | nach dem Merge von 6.5a liest `decode_quads` auch `BEGIN` Dreiecke (ein Test in Task 2 deckt es dann ab) |
| `crates/g4-kernel/src/map_import.rs` | **nicht** berührt: ein Import abgeleiteter Teile in eine Welt-Karte scheitert an der Ablehnung in `Tx::edit_map` (Task 4) | keine Änderung |
| `commands/dispatch.rs`, `mcp/access.rs`, `docs/mcp.md`, `docs/agent-gate.md`, `docs/ROADMAP.md` | Einträge und Abschnitte am Ende | nur anhängen |
| `mcp/docs.rs` | nicht berührt (6.3b legt keine neue Datei unter `docs/formats/` an) | – |

Weitere Anhängepunkte: `crates/g4-kernel/src/lib.rs` (nichts Neues), `crates/g4-kernel/src/world.rs`, `commands/mod.rs`, `src-tauri/src/lib.rs` (`generate_handler!`), `apps/studio/src/api.ts`, `design-system/README.md`.

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Wo der Member entsteht | Das **Map-Asset einer Welt-Karte ist der gebackene Member.** Der Store liest es im Textprojekt, indem er die Karten-JSON liest (`EntitySource::read_world_map`) und backt (`bake_world`); `Tx::edit_world` backt im selben Undo-Schritt neu (beide Assets sind Teile des einen Changesets); `save` backt jede schmutzige Welt-Karte unmittelbar vor dem Schreiben neu; der Build-Schritt `world` backt nach Datensatz; der Textprojekt-Import backt nach seinen Membern | Spezifikation §3.5 („der Member entsteht beim Speichern oder Bauen“); weil das Map-Asset die Ausgabe ist, stimmen Höhen (`get_map_heights`), Budget, Permissions-Overlay und Vorschaubild (GLB über apicula) ohne Sonderweg |
| Karten-Datei einer Welt-Karte | `permissions`, `model` und `bdhc` **fehlen** (`textproject.map-source`: „Eine Welt-Karte trägt `permissions`, `model` und `bdhc` nicht; sie entstehen beim Backen.“); `buildings`, `buildings_tail?`, `bgs?` bleiben Quelle wie heute. Eine binäre Karte braucht alle drei (`textproject.schema`: „Schlüssel `model` fehlt.“). In `MapJson` werden die drei `Option` | §3.5 „Der Member ist Ausgabe, nicht Quelle“; §3.1 „Git zeigt eine gemalte Zelle als geänderte Zeile“ (kein Beifile ändert sich mit jedem Strich) |
| Verbatim-Teile (Form 1) | **Modell:** `world.freeform = [{ "nsbmd": <Teil> }]` (genau ein Eintrag, `at` `[0, 0, 0]`); `FreeformJson.nsbmd` wird `Option<Part>` (`{"base_sha1"}`, `{"file": "0123.nsbmd"}`, `{"hex"}`, Basisschlüssel `maps NNNN model`). **Terrain:** neues `world.terrain = { "bdhc": <Teil> }` (Basisschlüssel `maps NNNN bdhc`), nach `freeform`, vor `permissions` | §3.1 („`freeform` (glTF oder verbatim-NSBMD, ab 6.3b …)“); als Teil statt Dateiname, damit eine übernommene Vanilla-Karte `base_sha1` trägt und keine Vanilla-Bytes als Beifile ins Projekt des Nutzers schreibt; Terrain verbatim ist für den Prüfstein „byte-gleich“ nötig (C §5.3: „Modell, BDHC verbatim“), §3.1 nennt dafür keinen Ort |
| Verbatim im Store | Solange die Welt ein Verbatim-Modell bzw. `terrain` trägt, **sind** Modell bzw. BDHC des Map-Assets die Verbatim-Bytes (`Verbatim::of(world, &map)`); der Writer bildet die Teile mit `part_out` aus den Map-Bytes (dieselbe Regel wie `map_json`), `world_adopt` legt sie mit derselben Regel an | ein Teil wird nie im Store aufgelöst; Store und Datei bleiben byte-gleich |
| Terrain verbatim endet | `world_edit` mit `height` oder `ramp` entfernt `world.terrain` im selben Schritt (Label unverändert); ab dann kommt die BDHC aus dem Raster | eine Quelle für Modell und BDHC (§7 „Modell und BDHC weichen ab“) |
| Verbatim-Modell und Kacheln | Backen lehnt das Modell ab (`world.verbatim-mixed`: „Ein Verbatim-Modell (Form 1) und gemalte Kacheln zusammen backt erst 6.5b; Kacheln leeren oder „Als binäre Karte übernehmen“.“); das Map-Asset behält das Verbatim-Modell | Gutachten C sah das Raster „obendrauf“; das hieße Shapes in ein Vanilla-MDL0 einfügen, „Bearbeiten von Vanilla-Modellen über den Codec“ steht in „Nicht in Phase 6“, die Freiform-Zusammenführung kommt mit 6.5b |
| Freiform sonst | `glb` gilt im Schema, Backen lehnt ab (`world.freeform-later`: „glTF-Freiform backt 6.5b.“); mehr als ein `nsbmd` oder `at ≠ [0, 0, 0]` ist `world.schema` („Höchstens ein Verbatim-Modell (`nsbmd`) in `freeform`.“, „Ein Verbatim-Modell liegt an [0, 0, 0].“) | 6.5b |
| Schemaversionen | `WORLD_VERSION = 2`, `PART_SCHEMAS = [("maps.world", 2)]`, `SCHEMA_VERSION = 4`; Migration `v3_to_v4`: Manifest; jede `data/maps/*.json` mit `source: "world"`: `world.version` 1 → 2, `permissions`, `model`, `bdhc` entfernt, ein Beifile, auf das nur ein entfernter Teil zeigte, gelöscht. Ein Manifest mit `maps.world` > 2 gilt wie eine neuere `schemaVersion` (nur lesen, Text von `manifest::refusal`) | 6.3a-Welten waren Vorschau („bis 6.3b baut der Build ihren bisherigen Member“, nur über `g4_call --expert` erreichbar); ab 6.3b ist die Welt die Quelle. Schließt die 6.3a-Altlast „keine Warnung bei neuerer `maps.world`“ |
| Permissions einer Welt | `derive: true`: je Zelle die **oberste** Ebene mit Kachel; deren Tileset-Kachel gibt `Cell { kind, collision \| sound }` (`sound` nur HGSS); eine Zelle ohne Kachel in allen Ebenen `EMPTY_CELL = {kind 0, collision 0x80}`; der Fußabdruck eines Teilmodells setzt die Kachel des Stücks; dann `terrain::block_edges` (Kante ≥ `edge_rule().free_below` blockiert die **obere** Zelle, Kartenrand ist keine Kante); dann die Überschreibungen. `derive: false`: nur die Überschreibungen (das Schema verlangt alle 1 024) | Entscheidungen B („Handwerte überschreiben“, „obere Zelle“); dieselbe Kantenregel wie `derive_permissions` (6.2), aus ihr herausgelöst |
| Gebiet und Textur | Texturen aus dem Paket `world.tileset` (`pack_textures` des Members), Tileset `Store::tileset(world.tileset)`; Stil (`AreaStyle::of`) und VRAM (`area_vram` mit diesem Paket) vom ersten Gebiet aus `map_areas(store, map)`; ohne Gebiet und mit Flächen: `world.no-area` („Karte 0123 liegt in keiner Matrixzelle mit Header; Backen braucht Licht und VRAM ihres Gebiets.“); ohne Flächen (Form 1) braucht das Backen kein Gebiet | §3.3 Stil je Gebiet; Form 1 muss auch für Karten ohne Header byte-gleich backen |
| Teilmodelle | Fußabdruck-Flächen (Ebene 255) werden mitgebacken wie in der Vorschau, bis 6.5b die Geometrie liefert | was die Szene zeigt, zeigt das Spiel |
| Abgelehnter Bäcker beim Strich | Der Strich bleibt (ein Undo-Schritt); das Map-Asset behält sein **letztes Modell**, BDHC und Permissions folgen der Welt; `WorldReport.bake` nennt die Ablehnung; Check `world.bake` (Fehler); der Build lehnt die Karte ab | §7 „Ablehnen beim Backen, nie kürzen“; Malen über das Budget und wieder zurück bleibt möglich |
| Abgeleitete Teile direkt ändern | `Tx::edit_map`/`Store::edit_map` auf einer Welt-Karte lehnen ab, wenn `permissions`, `model` oder `bdhc` sich ändern (`world_derived_refusal`: „Karte 0123 ist eine Welt-Karte: Modell, Terrain und Permissions entstehen beim Backen; Permissions über `world_edit` (`permission`), zurück zur binären Karte über „Als binäre Karte übernehmen“.“); Gebäude und BGS gehen. Damit lehnen `set_permission`, `set_map_terrain`, `bake_test` und `map_import` (Modell, BDHC, Permissions, ganze Karte) auf Welt-Karten ab, ohne eigenen Code | eine Quelle; kein Eingriff in `map_import.rs` (6.5a) |
| Schritt `world` | Läuft **nach `textproject`, vor `rotom`**, nur im Textprojekt. Eingaben: `data/maps/`, `data/tilesets/`, die Member-Ordner der Kartentexturen, Gebietsdaten und Matrizen, `arm9/arm9.bin`, `unpacked/dynamicHeaders/`. Ausgaben: die Member der Welt-Karten. Salz `world#<WORLD_BAKE_REV>+bake#<BAKE_REV>+bdhc#<GENERATOR_REV>`. Je Karte entscheidet ihr **Eingabe-Hash** (xxh3 über Salz, Familie, Karten-JSON, aufgelöste Verbatim-Bytes, Tileset-Datei oder `textures_only`-Quelle, Paket-Bytes, Gebiet und Stil) gegen den Datensatz | Abweichung von §3.5 („vor `textproject`“): `world` liest Paket-, Gebiets-, Header- und Matrix-Member, die `textproject` aus der JSON schreibt; Welt-Karten sind aus den Einheiten von `textproject` ausgenommen, die Reihenfolge erzeugt also keinen Konflikt. Inkrementell je Karte wie gefordert |
| Datensatz | Abschnitt `world` in `.g4/build/state.json`: je Karte `{ "input": "<xxh3>", "member": "<xxh3>" }`; geschrieben von Schritt, Speichern, Import, Konfliktlösung; **„Neu bauen (ohne Cache)“ behält ihn** wie `texts` und `sync`. Ohne Datensatz gilt die Welt (der Member wird gebacken und überschrieben) | wie `texts` (build_state.md); ohne Datensatz lässt sich „Welt geändert“ nicht von „Member geändert“ trennen |
| `world.member-changed` | Member-Hash ≠ Datensatz: im Schritt `world` **Fehler** je Karte, nichts geschrieben („Karte 0123: Der Member wurde außerhalb des Studios geändert (DSPRE?). In Technik › Textprojekt „Neu backen“ oder „Als binäre Karte übernehmen“.“); im Abgleich mit DSPRE **immer Konflikt** (`other: dspre`, `world: true`), nie eine einseitige Übernahme; im Textprojekt-Check Warnung. Wege über `resolve_external_conflict`: `Keep::Studio` = „Neu backen“ (Member aus der Welt, Datensatz neu), `Keep::Dspre` = „Als binäre Karte übernehmen“ (JSON aus dem Member mit `source: "binary"`, Welt weg) | §3.5, §7 „Synchronisierung mit DSPRE wie bei Scripts“: wie bei rotom ist der Member Ausgabe und der Abgleich urteilt über den Datensatz |
| „Als binäre Karte übernehmen“ ohne Konflikt | `world_to_binary {map}`: ein Undo-Schritt „Welt aufgegeben: Karte 0123“, WorldMap → `null`, das Map-Asset (gebacken) bleibt; Speichern schreibt die Karte binär. „Auf Original zurücksetzen“ einer Welt lehnt weiter ab, Text neu: „Eine Welt hat kein Original; erst „Als binäre Karte übernehmen“.“ | der Rückweg aus §3.5 ohne DSPRE |
| „Karte als Quelle übernehmen“ | `world_adopt {map, tileset?, form?}`; `form` nur `"verbatim"` (Vorgabe; `"gltf"`: „Form 2 (glTF) kommt mit 6.5b.“). Ein Undo-Schritt „Karte als Quelle übernommen: Karte 0123“: Welt v2 mit `tileset` (Anfrage, sonst erstes von `packs_of_map`, sonst 0 mit `tileset_guessed: true`), Bodenebene „Boden“ ganz `.`, `heights` aus `terrain::map_heights` der BDHC (Stufe = `round(y / 8)`, auf −16 bis 62 begrenzt; ohne Platte 0), keine Rampe, `freeform = [{nsbmd: part_out(model)}]`, `terrain = {bdhc: part_out(bdhc)}`, `permissions = {derive: true, override: alle 1 024 Zellen}`; Gebäude und `bgs` bleiben im Map-Asset. Abweichungen: Zellen ohne Platte oder mit `|y − 8·Stufe| > ADOPT_TOLERANCE = 0,1` E, je `{x, z, height: f64 \| null, step}`. Lehnt ab: DSPRE-Layout, schon Welt, gesperrte Karte (`locked_map_refusal`) | §3.5; Prüfstein „alle Vanilla-Karten mit Form 1 übernommen backen byte-gleich“; Toleranz wie das Gate von 6.2 (≤ 0,1 E) |
| Modellboden = BDHC | Test, nicht Laufzeit: je Zelle mit Bodenkachel liest `decode_quads` das gebackene Modell zurück, die höchste nach oben zeigende Fläche über der Zellmitte gibt die Bodenhöhe; Abstand zu `Bdhc::height_at(Mitte, i32::MAX)` ≤ `FLOOR_TOLERANCE = 0,1` E | §4 Zeile 6.3b, §7; 0,1 E ≥ 0,08 E (6.2 maß 0,0786 E) |
| Prüfungen | `world.not-baked` entfällt. Neu im Textprojekt-Check: `world.bake` (Fehler, je Ablehnung des Bäckers mit dessen Code: `bake.model-too-big`, `bake.texture-missing`, `bake.face-uv-range`, `world.no-area`, `world.verbatim-mixed`, `world.freeform-later`, `bake.empty`), `world.member-changed` (Warnung). `world.budget` wird echt: Info „Gebacken: Modell 21 740 Byte (35 % von 0xF000), Terrain 334 Byte (1 % von 0x9000), 3 Texturen, VRAM 21 656 Byte.“, Warnung mit „; VRAM über dem Vanilla-Höchstwert 110 640 Byte“ bei `bake.vram-high` | Gutachten C §8 „Budgets als Prüfungen“; die echte Zahl ersetzt die Schätzung |
| Tileset ändern | `tileset_edit {req: {package, changes: [{tile, role?, kind?, collision?, sound?, wall?}]}}`: ein Dateischritt wie `tileset_save` (Label „Tileset 0006 geändert (2 Kacheln)“), verlangt die Datei („Erst `tileset_save` für Paket 0006.“), prüft das Ergebnis mit `tileset_problems`; danach vergisst der Store die sauberen Welt-Karten dieses Pakets (sie werden beim nächsten Zugriff neu gebacken), schmutzige backt das nächste Speichern | Gras- und Wasser-Kacheln mit abgeleiteter Kollision brauchen Rollen, die die Vanilla-Ableitung kaum setzt (Task 0 von 6.3a: K1 nennt fast nur `kind 0`); der Tileset-Editor (6.4a) baut darauf |
| Szene | `mapModel` fragt für Welt-Karten `get_world_mesh`; **0 Flächen oder Fehler → GLB** (das gebackene Modell über apicula) | eine Form-1-Karte hat kein Raster; schließt die 6.3a-Altlast „kein GLB-Rückfall“; das Vorschaubild aus dem GLB ist jetzt das gebackene Modell |
| Lichtregel | `AreaStyle::of` bleibt (HGSS Lichttyp 1 = außen); `WorldBakeReport.style` nennt den Stil; `hg-gemalt` erwartet `outdoor` und macht ein Tagbild; Nacht: Task 11 prüft, ob melonDSDS die Uhr über eine Kernoption setzen lässt (dann Schritt `clock HH:MM` in g4emu und ein Nachtbild), sonst ist Nacht ein Hand-Punkt in melonDS | §7 „Licht außen: Außenkarte bei Tag und Nacht im Phasen-Gate“ |
| Altlasten | **in 6.3b:** `From<MeshFace> for bake::Face` (Task 2); `get_map_heights` für Welt-Karten (durch das gebackene Map-Asset, Test in Task 4); Vorschaubild aus dem GLB (gebacken, Task 10); GLB-Rückfall (Task 10); Warnung bei neuerer `maps.world` (Task 3); Lexer von `world::grid` nach `terrain::rows` (Task 1); `area_of_map` → `map_import::map_areas` (Task 1). **Nicht in 6.3b** (siehe unten): Format-5-Texturen in der Szene, Dateischritt von `tileset_save` an `HEADER_TABLE`, doppelt gezählte Texel der VRAM-Schätzung | was das Backen braucht oder berührt, kommt hier; Darstellung und Tileset-Editor gehören zu 6.4a |

### Nicht in 6.3b

- Malwerkzeuge, Palette, Draufsicht, Höhen-Overlay, kuratierte MCP-Werkzeuge, Rezept `new_painted_map`, Tileset-Editor-Oberfläche (6.4a); Kantensätze, Wände zu Nachbarkarten (6.4b); glTF-Freiform, Teilmodell-Geometrie, Form 2, Verbatim-Modell plus Raster (6.5b).
- Format-5-Texturen in der Szene dekodieren (6.4a, Palette braucht die Bilder); Dateischritt von `tileset_save`/`tileset_edit` an `HEADER_TABLE` (6.4a, mit dem Tileset-Editor); VRAM-Schätzung `terrain::vram_bytes` zählt geteilte Texel doppelt (6.4a stellt `map_budget` auf `bake::pack_vram` um, das die Größen aus dem BTX0-Kopf nimmt); eine Welt-Karte kopieren (`create_area` mit einer Welt-Karte als Quelle kopiert die gebackene Binärkarte; das Rezept von 6.4a übernimmt die Welt).
- „Nicht in Phase 6“ der Entscheidungen gilt wörtlich.

## Global Constraints

- **Lizenz:** gen4-studio ist MIT. Die Writer von DSPRE (NsbmdWriter, NsbtxWriter, TileBake, BdhcBuild) sind AGPL-3.0: kein Code übernehmen, auch nicht abschreiben. PDSMS hat keine Lizenz: ebenfalls kein Code. Erlaubt sind Formatwissen aus pret/pokeplatinum, eigene Messungen und Dokumentation.
- Keine Abhängigkeit vom NITRO-SDK (g3dcvtr.exe).
- **Byte-Treue:** Unveränderte Karten bauen weiterhin byte-gleich. Die Original-Assets bleiben unberührt, solange der Nutzer nicht malt. Form 1 von „übernehmen“ hält den Member verbatim.
- Keine ROM-Daten im Repo (`scripts/check-no-rom-data.sh` vor jedem Commit); Tilesets werden zur Laufzeit abgeleitet, nie eingecheckt. Szenarien nennen Kacheln nur als Index.
- Keine neue Abhängigkeit ohne Lizenz-Audit (`cargo deny --locked`); voraussichtlich keine nötig (xxhash ist da).
- **Worktree:** `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut; `tools` als Symlink auf den Haupt-Checkout (`ln -s $HOME/Projekte/lab/editor/gen4-studio/tools tools`, nie einchecken), `G4_APICULA=$HOME/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula`; Test-Umgebung nur auf `G4_VITE_PORT=1520 G4_BRIDGE_PORT=1521`, andere Ports nie anfassen (6.5a: 1500/1501); Testkopien nur `Roms/_testkopien/6-3b-<zweck>` (`cp -cR` Ordner, `cp -c` `.nds`, Ziel existiert noch nicht), Originale nie öffnen; gated Tests schreiben nur auf `vanilla_copy()`.
- Jeder Task endet mit einem Commit; jede Nachricht endet mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`). **Kein `scripts/gate.sh` im Worktree** (läuft auf main), kein Merge, kein Push; nie `cd` in den Haupt-Checkout.
- Lang laufende Befehle (gated Tests, Release-Builds, Brücke, Szenarien) im Hintergrund mit Log unter `target/agent/6-3b/<zweck>.log`, `uptime` davor; ein Zeittest, der unter Last reißt, wird allein wiederholt, die Grenze nie gelockert.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh`; immer `bash scripts/check-no-rom-data.sh`.
- UI nur aus Bausteinen und Tokens; deutsche Texte, englische Bezeichner und Commit-Messages.
- **Gate 6.3b** (Spezifikation §4, wörtlich): „Alle 666 / 676 Vanilla-Karten mit Form 1 übernommen backen **byte-gleich**; eine gemalte Karte (Boden, zwei Höhen mit Wand, Rampe, Wasser, Gras) backt mit NSBMD ≤ 0xF000 und BDHC ≤ 0x9000 + 16, apicula liest sie, Netz gleich der apicula-Lesung (Ecken, UV, Materialien), Modellboden = BDHC-Höhe in jeder Zellmitte; `vanilla_checks` 0 Fehler; Build-Schritt `world` inkrementell (eine geänderte JSON → eine Karte), `world.member-changed` mit beiden Wegen; **Phasen-Gate**, siehe unten“, dazu (Kopf der Gate-Spalte) gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US, Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer.
- **Phasen-Gate 6** (§4, wörtlich; in 6.3b der Emulator-Teil, die Karte per Befehl gemalt): „Eine im Studio gemalte Karte mit eigenem Gelände, Höhenstufe mit Treppe, Wasser und Gras-Kacheln und abgeleiteter Kollision wird gebaut und in eine Matrix gesetzt. Sie ist in g4emu und melonDS begehbar, auf Platin DE und HeartGold DE: die Treppe funktioniert (`expect-height`), die Wand blockiert (`walk-until-blocked`), Gras löst eine Begegnung aus (`expect-battle`), die Außenkarte sieht bei Tag und Nacht richtig beleuchtet aus (Blick). Unveränderte Projekte bauen byte-gleich, `vanilla_checks` hat 0 Fehler, `scripts/gate.sh` ist grün, DSPRE öffnet die Karte (Hand).“

## Review Focus

- **DSPRE ändert den Member einer Welt-Karte, während der Store einen ungespeicherten Strich hält:** Konflikt mit `unsaved`, keine Seite geht still verloren, Speichern ist `Stale`, der Build lehnt mit `world.member-changed` ab (Task 6 `dspre_change_with_an_unsaved_stroke_is_a_conflict`, Task 5 `a_changed_member_fails_the_step_and_writes_nothing`).
- **Strich über das Budget oder mit fehlender Textur:** Strich bleibt ein Undo-Schritt, das Modell bleibt das letzte, BDHC und Permissions folgen, `world.bake` nennt den Code, ⌘Z stellt alles zurück, nichts wird gekürzt (Task 4 `an_oversize_stroke_keeps_the_last_model_and_reports`).
- **Hand-editierte oder alte Karten-Datei:** eine Welt-Karte mit `model`/`permissions` (Git-Merge, Datei aus 6.3a ohne Migration), zwei `nsbmd`, `at ≠ 0`, `glb`: Check mit Pfad, Import lässt nur diese Karte aus, `world_adopt`/`world_edit` lehnen mit Text ab (Task 3 `a_world_map_with_derived_keys_is_refused_with_its_path`).
- **Zustand verloren:** „Neu bauen (ohne Cache)“ behält den Abschnitt `world`; eine gelöschte `state.json` lässt die Welt gelten, ohne Konflikt-Flut und ohne Absturz (Task 5 `clear_cache_keeps_world_records`, `without_a_record_the_world_wins`).
- **Karten ohne Header oder Paket:** Form 1 übernimmt und backt byte-gleich ohne Gebiet; eine gemalte Karte ohne Gebiet lehnt mit `world.no-area` ab statt zu paniken; ein Paket, das nicht liest, gibt `bake.texture-missing` (Task 2 `form1_needs_no_area`, Task 4 `a_painted_map_without_area_reports_no_area`).

---

### Task 1: Gelände: Kantenregel als Baustein, Lexer nach `terrain`, ein Gebiet je Karte

**Files:**
- Modify: `crates/g4-kernel/src/terrain/derive.rs` (`block_edges`, `derive_permissions` nutzt es), `crates/g4-kernel/src/terrain/rows.rs` (`lex_height`, `lex_ramp` hierher, ohne Import aus `world`), `crates/g4-kernel/src/terrain.rs` (`pub use derive::block_edges; pub use rows::{lex_height, lex_ramp};`), `crates/g4-kernel/src/world/grid.rs` (die beiden Funktionen entfernt, `pub use crate::terrain::{lex_height, lex_ramp};` damit `world::grid::lex_*` bleibt), `crates/g4-kernel/src/terrain/budget.rs` (`area_of_map` gelöscht, Aufrufer nehmen `crate::map_import::map_areas(store, map)?.first().copied()`)

**Interfaces:**
- Produces:

```rust
// terrain/derive.rs
/// Blocks (`| 0x80`, sound bits kept) the cell with the higher edge of every pair of neighbours inside the
/// map whose edge heights (`HeightGrid::edge_units`) differ by at least `edge_rule().free_below` units.
pub fn block_edges(grid: &HeightGrid, cells: &mut [Cell]);
// terrain/rows.rs (moved, bodies unchanged)
pub fn lex_ramp(token: &str) -> Option<g4_formats::bdhc::Ramp>;
pub fn lex_height(token: &str) -> Option<i64>;
```

- [ ] **Step 1: Scheiternde Tests:** `derive.rs` `block_edges_matches_derive_permissions` (für `scenario_input()`: `kind_rule`-Zellen plus `block_edges` == `derive_permissions`), `block_edges_keeps_sound_bits` (HGSS-Zelle `collision 0x05` an einer Kante → `0x85`); `rows.rs` `lexers_are_shared_with_the_world_grid` (`terrain::lex_ramp("e01") == world::grid::lex_ramp("e01") == Some(East(1))`); `budget.rs` `budget_area_is_the_first_of_map_areas` (Fixture mit zwei Headern auf dieselbe Karte: Gebiet des ersten in Tabellenordnung).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features terrain:: world::grid` → FAIL (`block_edges` fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `git grep -n "world::grid::lex_" crates` danach leer außer dem Re-Export.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features --test terrain` (ohne ROM) grün.
- [ ] **Step 5: Commit** `refactor(terrain): edge rule as block_edges, row lexers in terrain, one area rule per map` (mit Trailer).

### Task 2: Bäcker der Welt (rein) und Rückleser

**Files:**
- Create: `crates/g4-kernel/src/world/bake.rs` (+ Tests), `crates/g4-kernel/src/bake/decode.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/world.rs` (`pub mod bake;` und Re-Exporte), `crates/g4-kernel/src/world/mesh.rs` (`impl From<MeshFace> for crate::bake::Face`), `crates/g4-kernel/src/bake/mod.rs` (nur anhängen, siehe „Parallele Teilphase“), `crates/g4-formats/src/bdhc/grid.rs` (`pub const GENERATOR_REV: u32 = 1;`), `crates/g4-kernel/src/world/doc.rs` (Fixtures `gate_world`, `oversize_world`)

**Interfaces:**
- Consumes: Task 1 (`block_edges`); 6.1 `bake`, 6.2 `from_grid`, 6.3a `mesh`.
- Produces:

```rust
// bake/decode.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakedQuad { pub corners: [[i32; 3]; 4], pub uv: [[i32; 2]; 4], pub material: String }
/// The quads of a model `bake` wrote, in world units and texels (strips expanded, display-list order): the
/// inverse of `encode` for its subset (BEGIN quads / quad strip, TEXCOORD, NORMAL, COLOR, VTX_10, VTX_16, END);
/// any other command is `FormatError::Unsupported`.
pub fn decode_quads(nsbmd: &[u8]) -> Result<Vec<BakedQuad>, FormatError>;
// bake/mod.rs (appended)
pub const BAKE_REV: u32 = 1;
// world/bake.rs
pub const WORLD_BAKE_REV: u32 = 1;
pub const FLOOR_TOLERANCE: f64 = 0.1;   // world units
pub const EMPTY_CELL: Cell = Cell { kind: 0, collision: 0x80 };
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Verbatim { pub model: Option<Vec<u8>>, pub bdhc: Option<Vec<u8>> }
impl Verbatim { /// The verbatim parts `world` names, taken from `map` (store side: the Map asset holds them).
    pub fn of(world: &WorldJson, map: &MapFile) -> Verbatim; }
/// What the pure bake needs: the tileset, the bake input of the map's area (`None` without area) and the family.
pub struct BakeEnv<'a> { pub tileset: &'a Tileset, pub input: Option<BakeInput<'a>>, pub family: Family }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorldBakeMessage { pub code: String, pub level: Level, pub args: Vec<String>, pub message: String }
/// The parts of a bake: `model` is `None` when it is refused (a message says why) or verbatim.
pub struct GridBake { pub faces: Vec<MeshFace>, pub model: Option<BakeReport>, pub bdhc: Vec<u8>,
    pub permissions: Vec<Cell>, pub messages: Vec<WorldBakeMessage> }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldBakeReport { pub map: u16, pub ok: bool, pub style: Option<AreaStyle>, pub faces: usize,
    pub model_bytes: usize, pub model_share: f32, pub model_kept: bool, pub textures: Vec<String>,
    pub vram_bytes: usize, pub bdhc_bytes: usize, pub bdhc_payload: usize, pub plates: usize,
    pub blocked: usize, pub verbatim_model: bool, pub verbatim_terrain: bool, pub messages: Vec<WorldBakeMessage> }
pub fn world_permissions(grid: &WorldGrid, tileset: &Tileset, family: Family) -> Vec<Cell>;
pub fn bake_grid(grid: &WorldGrid, world: &WorldJson, verbatim: &Verbatim, env: &BakeEnv) -> GridBake;
/// `base` (buildings, tail, BGS kept) with the bake's permissions, BDHC and model; the model stays `base.model`
/// when the bake has none (refused): the bool says so.
pub fn assemble(base: &MapFile, bake: &GridBake, verbatim: &Verbatim) -> (MapFile, bool);
pub fn report(map: u16, bake: &GridBake, file: &MapFile, model_kept: bool, style: Option<AreaStyle>) -> WorldBakeReport;
#[cfg(any(test, feature = "test-support"))]
/// Pack textures for `Tileset::synthetic()`: each name with palette `<name>_pl`, sizes of the tiles.
pub fn synthetic_pack_textures() -> Vec<PackTexture>;
// world/doc.rs fixtures (test-support)
/// The gate map (tokens given): ground `ground` at 0; grass `grass` x 16–28, z 3–10; water `water` x 2–8,
/// z 24–29; stair `e1` at x 16 (h 0) and 17 (h 1), z 20–22; plateau h 2 at x 18–22, z 20–22; door override
/// `6980` at (12, 21).
pub fn gate_world(tileset: u16, ground: &str, grass: &str, water: &str) -> WorldJson;
/// A world over 0xF000 baked: checkerboard of three tiles, height (7x + 13z) mod 40.
pub fn oversize_world() -> WorldJson;
```

`bake_grid` (Reihenfolge der Meldungen fest): Freiform prüfen (`glb` → `world.freeform-later`); Verbatim-Modell mit Flächen im Raster → `world.verbatim-mixed`, Modell `None`; sonst ohne Verbatim-Modell: `mesh` → keine Fläche: `bake.empty`; `env.input == None`: `world.no-area`; sonst `bake(&faces.into_iter().map(Face::from).collect::<Vec<_>>(), input)`, eine Ablehnung (`report.refusal()`) wird `WorldBakeMessage` mit dem Code des Bäckers und `model: None`, die Warnung `bake.vram-high` wird übernommen. BDHC: `verbatim.bdhc` oder `from_grid(&grid.height_grid()).write()`. Permissions: `derive` → `world_permissions` dann `grid.overrides` darüber; sonst nur die Überschreibungen. `ok` = keine Meldung mit `Level::Error`.

- [ ] **Step 1: Scheiternde Tests** (synthetisches Tileset, `synthetic_pack_textures()`, `BakeInput { style: Outdoor, vram_bytes: 0, vram_limit: VANILLA_VRAM_MAX_PT }`): `decode.rs`: `decode_quads_returns_the_input_faces` (Testfläche von 6.1 und `checkerboard_world`: Multimenge `(corners, uv, material)` gleich der Eingabe), `decode_refuses_another_command`; `bake.rs`: `permissions_take_the_top_tile_then_edges_then_overrides` (Überlagerung Gras über Boden → `[0x02, 0]`; Plateau-Rand blockiert die obere Zelle; Override `6980` an (12, 21)), `empty_cells_are_blocked` (`.` in allen Ebenen → `EMPTY_CELL`), `pieces_set_their_tile_on_the_footprint`, `hgss_adds_the_sound_bits`, `derive_false_takes_only_the_overrides`, `one_tile_world_bakes_small` (Modell ≤ 4 096 Byte, eine Textur, 1 Fläche), `checkerboard_bakes_under_the_limit` (≤ 0xF000, `ok`), `gate_world_bakes_with_walls_ramp_grass_and_water` (`ok`, Wände vorhanden: Flächen mit waagerechter Normale; Rampe: Flächen mit geneigter Normale; Permissions Gras `[0x02, 0]`, Wasser `[0x15, 0]`; BDHC ≤ 0x9000 + 16), `bdhc_is_the_generator_s` (gleich `from_grid(height_grid).write()`), `model_floor_meets_the_bdhc_at_every_cell_centre` (`gate_world`, je Zelle mit Bodenkachel: höchste Fläche aus `decode_quads` mit Normale y > 0, deren x-z-Projektion die Mitte `tile_center` enthält, baryzentrisch interpoliert, gegen `Bdhc::read(bdhc).height_at(mx, mz, i32::MAX) / 4096`; druckt die größte Abweichung; ≤ `FLOOR_TOLERANCE`), `prop_model_floor_meets_the_bdhc` (proptest, 64 Fälle: Zufallsraster ohne Teilmodelle, Höhen −16 bis 60, gültige Rampen, Kacheln 0 bis 2), `verbatim_parts_pass_through_byte_equal` (`Verbatim { model: Some(m), bdhc: Some(b) }` mit leerem Raster: `assemble` gibt `m` und `b` unverändert), `form1_needs_no_area` (`input: None`, leeres Raster, Verbatim-Modell → `ok`), `verbatim_with_tiles_is_refused_and_keeps_the_model`, `glb_freeform_is_refused_for_now`, `an_oversize_world_is_refused_with_model_too_big` (`oversize_world`: erst `bake` direkt > 0xF000 prüfen, dann `bake_grid` → `bake.model-too-big`, `model: None`; `assemble` → altes Modell, `true`), `a_painted_world_without_area_reports_no_area`, `bake_of_a_busy_world_prints_its_time` (druckt `world-bake faces=… median_us=…` über 10 Läufe, keine Grenze; 6.4a legt sie fest).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::bake bake::decode` → FAIL.
- [ ] **Step 3: Umsetzen**; `decode_quads` spiegelt `bake/encode.rs` (Positionen `VTX_10`/`VTX_16` mit `POS_SCALE` zurück in Welteinheiten, die Eckenreihenfolge eines Strip-Vierecks wie `encode` sie schreibt), kein eigenes Format-Wissen außerhalb der Teilmenge.
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(world): bake a world grid into model, terrain and permissions; decode baked quads` (mit Trailer).

### Task 3: Karten-Datei Schema 2: Welt-Karten ohne abgeleitete Teile, Verbatim-Teile, Migration 3 → 4

**Files:**
- Modify: `crates/g4-kernel/src/world/doc.rs` (`WORLD_VERSION = 2`, `TerrainJson`, `WorldJson.terrain`, `FreeformJson.nsbmd: Option<Part>`, `blank` mit Version 2, Tests), `world/grid.rs` (Version 2, Freiform-Regeln), `world/keep.rs` (`world_parts`), `textproject/schema/map.rs` (`MapJson` mit drei `Option`, Regeln, `world_source`, `sample_map`), `textproject/source/{mod.rs, writers.rs}` (`read_world_map`; der Karten-Writer ruft für Welt-Karten `world_parts`), `textproject/export.rs` (`write_folder`: Welt-Karten über `carry_world` + `world_parts`), `textproject/import.rs` (Welt-Karten: JSON prüfen, Teile auflösen, **kein Member**; `Plan::world_maps: Vec<u16>`), `textproject/kinds.rs` (`PART_SCHEMAS = [("maps.world", 2)]`), `textproject/mod.rs` (`SCHEMA_VERSION = 4`), `textproject/migrate.rs` (`v3_to_v4`, vierter Eintrag), `textproject/manifest.rs` (Teilschema neuer als bekannt → `SchemaState::Newer`), `textproject/schema.snap` (`G4_UPDATE_SNAPSHOT=1`)

**Interfaces:**
- Consumes: Task 2 (`Verbatim`).
- Produces:

```rust
// world/doc.rs
pub const WORLD_VERSION: u32 = 2;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct TerrainJson { pub bdhc: Part }
// WorldJson: … freeform, #[serde(default, skip_serializing_if = "Option::is_none")] pub terrain: Option<TerrainJson>, permissions
// FreeformJson { glb: Option<String>, nsbmd: Option<Part>, at: [i32; 3] }
// textproject/schema/map.rs
pub struct MapJson { pub source: String, pub world: Option<WorldJson>, pub permissions: Option<Vec<String>>,
    pub buildings: Vec<BuildingJson>, pub buildings_tail: Option<String>, pub model: Option<Part>,
    pub bdhc: Option<Part>, pub bgs: Option<Part> }          // skip_serializing_if = "Option::is_none" on the three
pub(crate) struct WorldMapSource { pub world: WorldJson, pub base: MapFile /* permissions, model, bdhc empty */, pub verbatim: Verbatim }
/// A world map's source: buildings, tail, BGS read like a binary map; the verbatim parts resolved with
/// `part_in_at` (keys `maps NNNN model` / `bdhc`, `current` answering first).
pub(crate) fn world_source(j: &MapJson, index: u32, current: Option<&[u8]>, ctx: &ImportCtx<'_>) -> Result<WorldMapSource, String>;
// world/keep.rs
/// `fresh` (written from the baked member) as a world map's file: `permissions`, `model`, `bdhc` removed; with a
/// verbatim model its `model` part becomes `world.freeform[0].nsbmd`, with verbatim terrain its `bdhc` part
/// `world.terrain.bdhc`; a side file of a removed part leaves `side`.
pub fn world_parts(fresh: Vec<u8>, world: &WorldJson, side: &mut Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, KernelError>;
// textproject/source/mod.rs
impl EntitySource { pub fn read_world_map(&mut self, project: &Project, map: u16) -> Result<Option<WorldMapSource>, KernelError>; }
// textproject/migrate.rs
fn v3_to_v4(tree: &mut Tree) -> Result<(), String>;
```

- [ ] **Step 1: Scheiternde Tests:** `doc.rs` `a_v2_world_round_trips_with_terrain_and_verbatim_model` (Schlüsselreihenfolge `version, tileset, layers, freeform, terrain, permissions`; `nsbmd: {"base_sha1": …}`), `freeform_nsbmd_is_a_part`; `grid.rs` `version_one_is_refused` („Version 1 kennt diese Fassung nicht (2).“), `at_most_one_verbatim_model_at_the_origin` (beide Texte); `map.rs` `a_world_map_with_derived_keys_is_refused_with_its_path` (Review Focus; `textproject.map-source`, je Schlüssel), `a_binary_map_needs_its_three_parts` („Schlüssel `model` fehlt.“ usw.), `world_source_resolves_the_verbatim_parts` (`base_sha1` gegen `MemBase`, `file` gegen ein Beifile), `the_import_leaves_world_maps_to_the_world_step` (Plan: `world_maps == [0]`, Member von Karte 0 unverändert, alle anderen geschrieben); `keep.rs` `world_parts_drop_derived_keys_and_move_verbatim_parts` (ohne Verbatim: drei Schlüssel weg, Beifiles `0000.nsbmd`/`0000.bdhc` aus `side` entfernt; mit: Teile an ihren Platz); `source/tests.rs` `writing_a_world_map_writes_no_derived_keys`; `export.rs` `export_of_a_world_map_is_idempotent`; `migrate.rs` `v3_to_v4_drops_derived_parts_of_world_maps` (Baum mit einer v1-Welt-Karte samt Beifile `0003.nsbmd` und einer binären: die Welt-Karte ohne die drei Schlüssel mit `version` 2, Beifile weg, binäre Karte und alles andere byte-gleich, Manifest `schemaVersion` 4, `maps.world` 2), `the_chain_reaches_version_four`; `manifest.rs` `a_newer_part_schema_reads_only` (`maps.world` 3 → `Newer`); bestehend nach `G4_UPDATE_SNAPSHOT=1 cargo test -p g4-kernel --all-features schema_fields_match_the_snapshot` grün.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features textproject:: world::doc world::grid world::keep` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces und „Entscheidungen“. `textproject::part` ist `pub(crate)`: weil `WorldJson` öffentlich ist, wird `Part` als `pub use part::Part;` in `textproject/mod.rs` öffentlich (sonst meldet `private_interfaces` unter `-D warnings`), das Modul bleibt `pub(crate)`. Die 6.3a-Tests, die `model`/`permissions` an Welt-Karten setzen (`sample_map(true)`, `a_world_map_imports_its_binary_parts`), auf die neue Form umstellen und im Commit nennen.
- [ ] **Step 4: Lauf** → PASS; ganzer Kernel `cargo test -p g4-kernel --all-features > target/agent/6-3b/t3.log 2>&1` (Hintergrund) grün.
- [ ] **Step 5: Commit** `feat(textproject): world maps without derived parts, verbatim parts, schema 4 with migration` (mit Trailer).

### Task 4: Store: das Map-Asset einer Welt-Karte ist gebacken

**Files:**
- Modify: `crates/g4-kernel/src/world/bake.rs` (Store-Teil), `crates/g4-kernel/src/store.rs` (`entity_bytes` für `Map` einer Welt-Karte, `Tx::edit_world` backt, `Tx::edit_map`/`Store::edit_map` mit Ablehnung, `save` backt schmutzige Welt-Karten vor dem Schreiben, `Store::world_bake_report`, `Store::forget_world_bakes`), `crates/g4-kernel/src/world/edit.rs` (`WorldReport.bake`, `world_edit` entfernt `terrain` bei `height`/`ramp`, `world_to_binary`, Text von `WORLD_HAS_NO_ORIGINAL`), `crates/g4-kernel/src/world/check.rs` (`world.not-baked` weg; `world.bake`, `world.budget` echt; Store über `Project::open` wie bisher), `crates/g4-kernel/src/test_support.rs` (`world_ready`)

**Interfaces:**
- Consumes: Task 2 (`bake_grid`, `assemble`, `report`, `Verbatim`), Task 3 (`read_world_map`, `WorldMapSource`).
- Produces:

```rust
// world/bake.rs
pub struct WorldBake { pub file: MapFile, pub report: WorldBakeReport }
/// Bakes `world` for map `map` with the store: `Store::tileset(world.tileset)`, `pack_textures` of that package's
/// member, style and VRAM of the first of `map_areas(store, map)`. `Err` only for a world that does not parse
/// (`world.schema`) or I/O; refusals are in the report.
pub fn bake_world(store: &mut Store, map: u16, world: &WorldJson, base: &MapFile, verbatim: &Verbatim) -> Result<WorldBake, KernelError>;
pub fn world_derived_refusal(map: u16) -> String;   // Wortlaut in „Entscheidungen“
// store.rs
impl Store {
    /// The bake report of world map `map` as the store holds it (baked anew, nothing changed); `None` without world.
    pub fn world_bake_report(&mut self, map: u16) -> Result<Option<WorldBakeReport>, KernelError>;
    /// Unloads the clean Map and WorldMap assets of every loaded world map on `package` (read and baked anew on use).
    pub fn forget_world_bakes(&mut self, package: u16);
}
// world/edit.rs
pub struct WorldReport { pub map: u16, pub tileset: u16, pub cells: usize, pub change: Option<ChangeId>, pub bake: Option<WorldBakeReport> }
pub fn world_to_binary_label(map: u16) -> String;   // „Welt aufgegeben: Karte 0123“
pub fn world_to_binary(store: &mut Store, map: u16) -> Result<WorldReport, KernelError>;
// world/check.rs
pub const WORLD_BAKE_CODE: &str = "world.bake";
pub const WORLD_MEMBER_CHANGED_CODE: &str = "world.member-changed";   // Check-Teil folgt mit Task 5
pub fn baked_budget_text(report: &WorldBakeReport, vram_max: usize) -> String;
// test_support.rs
/// `TextFixture::exported()` with map texture package 0 as a real NSBTX of the six `syn_*` textures
/// (16 colours, palettes `<name>_pl`) and `data/tilesets/0000.json` from `Tileset::synthetic()`.
pub fn world_ready() -> TextFixture;
```

Laden: `entity_bytes(Map i)` liest bei `source: "world"` über `read_world_map` und gibt `assemble(...)` aus `bake_world` mit `base` = gelesene Gebäude/BGS (Modell beim Laden ohne Ablehnung: das gebackene; bei abgelehntem Modell der Member auf der Platte als `base.model`). `Tx::edit_world`: nach dem Schließen der Änderung `bake_world` mit `base` = aktuelles Map-Asset, `Verbatim::of(new_world, &map)`, das Ergebnis als zweiter Teil desselben Changesets über einen internen Weg ohne die Ablehnung abgeleiteter Teile.

- [ ] **Step 1: Scheiternde Tests** (`world_ready()`): `world_create_bakes_the_map_in_the_same_step` (nach `world_create`: Map-Modell = `bake` einer Fläche, BDHC = Generator, Permissions abgeleitet; `history()` ein Eintrag; `undo` → Map-Asset byte-gleich zum Member vorher), `world_edit_rebakes_and_undo_restores_both`, `save_writes_member_and_json_without_derived_keys` (Member = Map-Asset; JSON ohne `permissions`/`model`/`bdhc`), `loading_a_world_map_bakes_it` (Store neu öffnen: Map-Asset = gespeicherter Member), `get_map_heights_of_a_world_map_follow_the_world` (6.3a-Altlast: `terrain::map_heights` des Map-Assets an der Plateau-Mitte 16 E nach einem `world_edit` mit Höhe 2), `an_oversize_stroke_keeps_the_last_model_and_reports` (Review Focus: `world_edit` auf `oversize_world`-Zellen → Report `ok: false`, `model_kept: true`, Code `bake.model-too-big`; BDHC und Permissions neu; ⌘Z stellt alles zurück), `editing_derived_parts_of_a_world_map_is_refused` (`edit_map` Permissions → `world_derived_refusal`; Gebäude verschieben geht und bleibt nach einem weiteren Strich), `height_edit_ends_verbatim_terrain` (Welt mit `terrain` → `world_edit height` → `terrain: None`, BDHC aus dem Raster), `world_to_binary_keeps_the_baked_map` (Undo-Schritt, WorldMap `null`, Map-Asset unverändert, Speichern schreibt `source: "binary"` mit den drei Teilen), `reset_of_a_world_names_world_to_binary`, `forget_world_bakes_rebakes_on_use` (Tileset-Datei ändern, `forget_world_bakes(0)`, Map-Asset neu gebacken), `a_painted_map_without_area_reports_no_area` (Matrix ohne die Karte: `world.no-area`, kein Panik), `a_package_that_does_not_read_reports_texture_missing` (Review Focus: `TextFixture::exported()` mit seinem Stub-Paket und der Tileset-Datei aus `Tileset::synthetic()`, `world_create` → Report `bake.texture-missing`, kein Fehler des Befehls); `check.rs`: `bake_problems_name_the_code` (`world.bake` mit `bake.model-too-big`), `budget_is_the_baked_size` (Wortlaut), `not_baked_is_gone`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world:: store::` → FAIL.
- [ ] **Step 3: Umsetzen**; die 6.3a-Tests, deren Sinn „Member bleibt“ war (`save_writes_world_and_keeps_member`), auf „Member = Bake“ umstellen und im Commit nennen.
- [ ] **Step 4: Lauf** → PASS; Workspace `cargo test --workspace --all-features > target/agent/6-3b/t4.log 2>&1` (Hintergrund) grün.
- [ ] **Step 5: Commit** `feat(store): a world map's map asset is its bake, one undo step per stroke, derived parts guarded` (mit Trailer).

### Task 5: Datensatz und Build-Schritt `world`, Import backt Welt-Karten

**Files:**
- Create: `crates/g4-kernel/src/world/record.rs`, `crates/g4-kernel/src/world/build.rs` (+ Tests je Datei)
- Modify: `crates/g4-kernel/src/build/state.rs` (`WorldRecord`, `BuildState.world`, `clear_keeping_texts` behält `world`), `crates/g4-kernel/src/build.rs` (`prepare`: `WorldStep` nach `textproject`, nur mit Manifest; Probleme in `Prepared.problems`), `crates/g4-kernel/src/textproject/step.rs` (Welt-Karten sind keine Einheit des Schritts; `TEXTPROJECT_REV` 3), `crates/g4-kernel/src/textproject/import.rs` (`apply_import` ruft nach den Membern `bake_world_maps(project, &BakeScope::Maps(plan.world_maps), BakeMode::Import)`), `crates/g4-kernel/src/store.rs` (`save` zeichnet `WorldRecord` für gespeicherte Welt-Karten mit `ok`-Bake auf), `crates/g4-kernel/src/world/check.rs` (`world.member-changed` Warnung), `crates/g4-kernel/src/world.rs`

**Interfaces:**
- Consumes: Task 4 (`bake_world`, `Store`), Task 3 (`Plan::world_maps`, `read_world_map`).
- Produces:

```rust
// build/state.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRecord { pub input: String, pub member: String }      // xxh3, 16 hex digits each
// BuildState: #[serde(default, with = "archive_keys")] pub world: BTreeMap<u16, WorldRecord>
// world/record.rs
pub fn world_salt() -> String;   // "world#<WORLD_BAKE_REV>+bake#<BAKE_REV>+bdhc#<GENERATOR_REV>"
/// xxh3 of what baking `map` reads (salt, family, map JSON, resolved verbatim bytes, tileset file or package
/// bytes, package bytes, area id and style); `None` when the map is no world map.
pub fn input_hash(store: &mut Store, map: u16) -> Result<Option<String>, KernelError>;
pub fn member_hash(bytes: &[u8]) -> String;
/// Against the record of `map`: input hash and member hash equal → InStep; input differs → SourceChanged;
/// member differs → MemberChanged; both → Both; no record → Unrecorded. A **missing member** counts as
/// SourceChanged (an output that is gone is baked again, like the build's `removed`), never as MemberChanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSide { InStep, SourceChanged, MemberChanged, Both, Unrecorded }
pub fn world_side(store: &mut Store, map: u16) -> Result<Option<WorldSide>, KernelError>;
pub fn record_world(project: &Project, entries: &[(u16, WorldRecord)]) -> Result<(), KernelError>;
// world/build.rs
pub const WORLD_STEP_ID: &str = "world";
#[derive(Debug, Clone, PartialEq, Eq)] pub enum BakeScope { All, Maps(Vec<u16>) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum BakeMode { Step, Import, Rebake }   // Rebake: over a changed member
#[derive(Debug, Clone, Default, Serialize)]
pub struct WorldBuild { pub baked: Vec<u16>, pub skipped: usize, pub problems: Vec<Problem> }
/// Opens a store on `project`, decides every world map of `scope` by [`world_side`] (Step/Import: MemberChanged
/// or Both → `world.member-changed` error for that map, nothing written for any map; Rebake: bakes over),
/// bakes the others whose side is not InStep, writes their members and records them. A refused bake is a
/// `world.bake` error. Errors come back in `problems`; nothing is written when one is an error.
pub fn bake_world_maps(project: &Project, scope: &BakeScope, mode: BakeMode) -> Result<WorldBuild, KernelError>;
#[derive(Default)] pub(crate) struct WorldStep { pub(crate) problems: RefCell<Vec<Problem>> }
impl Step for WorldStep { /* id WORLD_STEP_ID, salt world_salt(), inputs/outputs per „Entscheidungen“,
    run: bake_world_maps(All, Step); StepRun { done: baked, total: world maps } */ }
```

- [ ] **Step 1: Scheiternde Tests** (`world_ready()` mit zwei Welt-Karten 0 und 1, gespeichert, Build einmal): `state.rs` `clear_cache_keeps_world_records`; `record.rs` `input_hash_moves_with_json_tileset_and_package`, `side_is_member_changed_after_an_outside_write`; `build.rs` `world_step_bakes_one_map_for_one_changed_json` (JSON von Karte 1 per `world_edit` + `save` außerhalb des Builds ändern → nächster `prepare`: Zeile `world` inkrementell, `baked == [1]`), `a_changed_tileset_rebakes_its_maps_only`, `a_changed_member_fails_the_step_and_writes_nothing` (Member von Karte 0 per Dateischreiben ändern → `KernelError::Compile` mit `world.member-changed` an Karte 0, Member von Karte 1 unverändert), `without_a_record_the_world_wins` (`state.json` gelöscht → Schritt backt, kein Konflikt), `textproject_step_leaves_world_maps_alone` (Welt-Karte: kein `member-newer`, kein `unsynced`, nach dem Schritt Member = Bake), `the_import_bakes_world_maps` (Member löschen, `g4k text import` → Member = Bake), `a_refused_bake_fails_the_step_with_world_bake`, `save_records_the_world`; `check.rs` `member_changed_is_a_warning_in_the_check`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::record world::build build:: textproject::step` → FAIL.
- [ ] **Step 3: Umsetzen**; `--test build_graph` und `--test textproject` (ohne ROM) bleiben grün.
- [ ] **Step 4: Lauf** → PASS; Workspace im Hintergrund grün.
- [ ] **Step 5: Commit** `feat(build): step world bakes world maps per map after textproject, records input and member` (mit Trailer).

### Task 6: Abgleich mit DSPRE: `world.member-changed` mit beiden Wegen

**Files:**
- Modify: `crates/g4-kernel/src/textproject/sync/{scan.rs, apply.rs, conflict.rs, diff.rs, watch.rs}` (Welt-Karten nach `world_side`), `crates/g4-kernel/src/textproject/sync/conflict_tests.rs`, `crates/g4-kernel/src/world/build.rs` (`take_member_as_binary`)

**Interfaces:**
- Consumes: Task 5 (`world_side`, `bake_world_maps(…, Rebake)`, `record_world`).
- Produces:

```rust
// sync/conflict.rs
pub struct ConflictEntry { /* … */ pub world: bool }   // true: a world map's member changed outside
// world/build.rs
/// „Als binäre Karte übernehmen“: the map JSON written from the member as `source: "binary"` (world dropped,
/// `EntitySource::write` with WorldMap `None`), the world record of `map` removed, the unit recorded for the sync.
pub fn take_member_as_binary(project: &Project, map: u16) -> Result<(), KernelError>;
```

Regeln im Scan: eine Karten-Einheit mit `source: "world"` urteilt über `world_side`: `InStep` → in Schritt; `SourceChanged` → `Files` (Übernahme = `bake_world_maps(Maps([i]), Import)`); `MemberChanged`, `Both` → `Conflict` mit `other: dspre`, `world: true`; `Unrecorded` → `Files`. `resolve(key, Keep::Studio)` bei `world` → `bake_world_maps(Maps([i]), Rebake)`; `Keep::Dspre` → `take_member_as_binary`. `conflict_diff` einer Welt-Karte zeigt die Felder des Members gegen den Bake (`permissions`, Größen von `model`/`bdhc`, Gebäude).

- [ ] **Step 1: Scheiternde Tests** (`conflict_tests.rs`, `world_ready()`, gespeichert und gebaut): `a_member_changed_by_dspre_is_a_world_conflict` (nie einseitig übernommen, `world: true`), `rebake_overwrites_the_member_and_records_it` (danach `scan` leer, Member = Bake), `take_as_binary_drops_the_world_and_keeps_dspre_bytes` (JSON `source: "binary"` mit den Teilen des DSPRE-Members, Member byte-gleich, Datensatz weg), `a_git_change_of_the_world_rebakes_one_sided`, `dspre_change_with_an_unsaved_stroke_is_a_conflict` (Review Focus: Store mit ungespeichertem `world_edit`, Member außen geändert → `unsaved: true`; `save` → `Stale`), `the_build_refuses_open_world_conflicts`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features textproject::sync` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(sync): a world map's member changed outside is a conflict, rebake or take it as binary` (mit Trailer).

### Task 7: „Karte als Quelle übernehmen“ (Form 1) und `tileset_edit`

**Files:**
- Create: `crates/g4-kernel/src/world/adopt.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/world/tileset.rs` (`tileset_edit`), `crates/g4-kernel/src/world.rs`

**Interfaces:**
- Consumes: Task 4 (`Tx::edit_world` mit Bake), 6.2 `terrain::map_heights`, `textproject::part::part_out`, `Store::source` (Basis).
- Produces:

```rust
// world/adopt.rs
pub const ADOPT_TOLERANCE: f64 = 0.1;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)] #[serde(rename_all = "lowercase")]
pub enum AdoptForm { #[default] Verbatim, Gltf }
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldAdoptRequest { pub map: u16, #[serde(default)] pub tileset: Option<u16>, #[serde(default)] pub form: AdoptForm }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HeightDeviation { pub x: u8, pub z: u8, pub height: Option<f64>, pub step: i8 }
#[derive(Debug, Clone, Serialize)]
pub struct WorldAdoptReport { pub map: u16, pub tileset: u16, pub tileset_guessed: bool, pub deviations: Vec<HeightDeviation>,
    pub change: Option<ChangeId>, pub bake: WorldBakeReport }
pub fn adopt_label(map: u16) -> String;   // „Karte als Quelle übernommen: Karte 0123“
pub fn world_adopt(store: &mut Store, req: &WorldAdoptRequest) -> Result<WorldAdoptReport, KernelError>;
// world/tileset.rs
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct TileChange { pub tile: u16, #[serde(default)] pub role: Option<TileRole>, #[serde(default)] pub kind: Option<u8>,
    #[serde(default)] pub collision: Option<u8>, #[serde(default)] pub sound: Option<u8>, #[serde(default)] pub wall: Option<Option<u16>> }
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct TilesetEditRequest { pub package: u16, pub changes: Vec<TileChange> }
#[derive(Debug, Clone, Serialize)]
pub struct TilesetEditReport { pub package: u16, pub tiles: usize, pub change: ChangeId }
/// One file step „Tileset 0006 geändert (2 Kacheln)“; a changed tile gets `origin: user`; refused without the file
/// („Erst `tileset_save` für Paket 0006.“), in the DSPRE layout, for a tile past the tileset and when the result has
/// `tileset_problems`; then `Store::forget_world_bakes(package)`.
pub fn tileset_edit(store: &mut Store, req: &TilesetEditRequest) -> Result<TilesetEditReport, KernelError>;
```

- [ ] **Step 1: Scheiternde Tests:** `adopt.rs` (`TextFixture::exported()`, Karte 0 mit eigener BDHC aus `scenario_input()`): `adopt_bakes_byte_equal` (Map-Asset vor und nach dem Übernehmen byte-gleich; Speichern; Store neu öffnen; Map-Asset = Original-Member; JSON: `source: "world"`, `freeform[0].nsbmd`, `terrain.bdhc`, `override` 32 Zeilen ohne `.`), `adopt_samples_heights_and_lists_deviations` (Rampenmitte 4 E → Stufe 0 mit Abweichung; Plateau 16 E → Stufe 2 ohne), `adopt_uses_base_sha1_when_the_part_is_vanilla` (Fixture mit Basisschicht), `adopt_without_header_guesses_package_zero`, `adopt_is_one_undo_step`, `adopt_refuses_dspre_world_locked_and_gltf` (je Text), `painting_on_an_adopted_map_reports_verbatim_mixed`; `tileset.rs` `tileset_edit_sets_roles_as_one_file_step` (Undo stellt die Datei her), `tileset_edit_refuses_without_file_and_past_the_tiles`, `tileset_edit_rebakes_world_maps_on_its_package` (Gras-Rolle → Permissions der Welt-Karte `[0x02, 0]` nach dem nächsten Zugriff).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(world): adopt a map as source verbatim (form 1), edit tileset tiles as one step` (mit Trailer).

### Task 8: Befehle: Tauri, Brücke, Zugriff, `docs/mcp.md`

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/world_map.rs` (vier `*_impl` + `#[tauri::command]`, `WorldGetDto.bake`), `commands/dispatch.rs` (vier Namen in `COMMANDS` und Arme, angehängt), `src-tauri/src/lib.rs` (`generate_handler!`, angehängt), `mcp/access.rs` (vier Einträge angehängt), `docs/mcp.md` (Abschnitt „Welt-Karten backen (6.3b, über `g4_call`)“ am Ende), Studio-DTO der Konflikte (`world`), Tests unten in `world_map.rs`

**Interfaces:**
- Consumes: Tasks 4, 6, 7.
- Produces (Argumente wie Tauri, camelCase):

```rust
pub fn world_adopt_impl(state: &AppState, req: WorldAdoptRequest) -> Result<WorldAdoptReport, CmdError>;        // Write
pub fn world_to_binary_impl(state: &AppState, map: u16) -> Result<WorldReport, CmdError>;                      // Write
pub fn world_bake_impl(state: &AppState, map: u16) -> Result<Option<WorldBakeReport>, CmdError>;               // Read
pub fn tileset_edit_impl(state: &AppState, req: TilesetEditRequest) -> Result<TilesetEditReport, CmdError>;    // Write
// WorldGetDto { map, world, faces, textures, estimate, share, bake: Option<WorldBakeReport> }
```

Dispatch: `world_adopt { req }`, `world_to_binary { map }`, `world_bake { map }`, `tileset_edit { req }`. Konflikte gehen weiter über `resolve_external_conflict { key, keep }`.

- [ ] **Step 1: Scheiternde Tests:** `world_bake_commands_over_dispatch` (Fixture `world_ready()`: `world_create`, `world_edit`, `world_get` → `bake.ok`, `world_bake` gleich, `get_map_heights` folgt der Welt, `world_to_binary` → `world_get.world == null`, `undo` → wieder Welt), `world_adopt_over_dispatch` (Form 1 → `get_map_model_glb` gleich vorher), `tileset_edit_over_dispatch`, `world_commands_refuse_in_the_dspre_layout` (`world_adopt`, `world_to_binary`, `tileset_edit` mit `WORLD_NEEDS_TEXT_PROJECT`); bestehend `every_command_has_a_kind`, Gleichheit `COMMANDS`/`generate_handler!`, `the_curated_set_is_the_decided_one` unverändert.
- [ ] **Step 2: Lauf** `cargo test -p studio world_` → FAIL; **Step 3: Umsetzen**; `docs/mcp.md`: die vier Befehle mit Art, Argumenten, Antwort, Ablehnungen, dazu `world_get.bake` und der Satz „Konflikte `world.member-changed` über `resolve_external_conflict` (`studio` = Neu backen, `dspre` = Als binäre Karte übernehmen)“.
- [ ] **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(studio): commands to adopt, bake, release world maps and edit tileset tiles` (mit Trailer).

### Task 9: Gated Tests auf Platin DE, HeartGold DE, HeartGold US

**Files:**
- Create: `crates/g4-kernel/tests/world_bake.rs`, `crates/g4-kernel/tests/common/glb.rs` (die GLB-Helfer aus `tests/bake.rs` hierher, `tests/bake.rs` nutzt sie von dort)
- Modify: `crates/g4-kernel/tests/bake.rs` (Helfer importiert), `crates/g4-kernel/tests/textproject.rs` (`textproject_round_trip_with_world`: nach dem Import ist der Member der Welt-Karte gleich `bake_world` der gespeicherten Welt, alle übrigen Member byte-gleich zur Basis)

**Interfaces:**
- Consumes: Tasks 2 bis 7; `vanilla_copy()`, `G4_TEST_PROJECT`, `G4_APICULA`.
- Produces: die Belegzeilen `world-adopt …`, `world-adopt-build …`, `world-gate …`, `world-step …`, `world-conflict …` für die Gate-Datei.

- [ ] **Step 1: Tests:**
  - `vanilla_maps_adopted_form1_bake_byte_equal` (Store auf `vanilla_copy()`, als Textprojekt eingerichtet; je Karte `world_adopt` und `Store::map` → byte-gleich zum Original-Member; druckt `world-adopt <code> maps=<n> adopted=<n> equal=<n> deviating_maps=<n> deviating_cells=<n> ms=<n>`; verlangt `adopted == equal == member_count(Map)`, CPUD 666, IPKD/IPKE 676).
  - `vanilla_adopted_project_builds_byte_equal` (alle Karten übernommen, `save`, `g4k text import` nach gelöschten Map-Membern, `build` → ROM byte-gleich zur Basis-ROM; `check` des Textprojekts ohne Fehler; druckt `world-adopt-build <code> ms=<n>`).
  - `painted_map_on_a_vanilla_pack_bakes_within_limits` (die Karte des ersten Headers von Zweiblattdorf bzw. Neuborkia; `tileset_save`, `tileset_edit` setzt Kachel 1 `grass` `kind 0x02`, Kachel 2 `water` `kind 0x15`; `world_create` + `world_edit` mit den Zellen von `gate_world(…, "0000", "0001", "0002")`; verlangt `bake.ok`, Modell ≤ 0xF000, BDHC ≤ 0x9000 + 16, Modellboden = BDHC je Zelle ≤ `FLOOR_TOLERANCE`; mit `G4_APICULA`: apicula Exit 0, genau eine GLB, Menge `(Ecke, UV)` der GLB gleich der von `mesh`, Materialnamen gleich `mesh_textures`; druckt `world-gate <code> map=<n> pack=<n> nsbmd=<n> share=<f> bdhc=<n> faces=<n> floor_max_dev=<f> apicula=<ok|skipped>`).
  - `world_step_is_incremental_per_map_on_the_copy` (zwei Welt-Karten, gebaut; eine JSON ändern → Zeile `world` baked 1; druckt `world-step <code> baked=1 of=2 ms=<n>`).
  - `member_changed_both_ways_on_the_copy` (Member außen ändern → Build scheitert mit `world.member-changed`; `Keep::Studio` → Member = Bake; erneut ändern, `Keep::Dspre` → JSON binär, Member byte-gleich den geänderten Bytes; druckt `world-conflict <code> rebake=ok binary=ok`).
- [ ] **Step 2: Kopien** `6-3b-pt`, `6-3b-hgde`, `6-3b-hgus` (je `_DSPRE_contents` mit `.nds` daneben).
- [ ] **Step 3: Lauf** je Kopie nacheinander im Hintergrund: `G4_ROMS_DIR=$R G4_TEST_PROJECT=$K/6-3b-<k>_DSPRE_contents G4_APICULA=… cargo test --release -p g4-kernel --all-features --test world_bake --test textproject -- --nocapture --test-threads 1 > target/agent/6-3b/gated-<k>.log 2>&1` → PASS; dann `--test vanilla_checks` je Kopie (Zahlen gleich 6.2: Fehler 0).
- [ ] **Step 4: Commit** `test(world): form 1 byte-equal on every vanilla map, painted map within limits, step and conflict on the copies` (mit Trailer).

### Task 10: Oberfläche: Quelle der Karte, Konflikt mit zwei Wegen, Build-Zeile, GLB-Rückfall

**Files:**
- Create: `apps/studio/src/workbench/inspector/WorldSourceSection.tsx`, `WorldSourceSection.module.css`, `WorldSourceSection.test.tsx`
- Modify: `apps/studio/src/api.world.ts` (Typen und Aufrufe), `apps/studio/src/api.textproject.ts` (`ConflictEntry.world: boolean`), `workbench/inspector/MatrixCellEditor.tsx` (Abschnitt über „Budget“), `technik/stepWords.ts` (`world`), `technik/TextProjectSurface.tsx` (`ConflictBody`), `world/scene/worldMesh.ts` (`mapModel`), Tests `stepWords.test.ts`, `TextProjectSurface.test.tsx`, `worldMesh.test.ts`

**Interfaces:**
- Consumes: Task 8.
- Produces:

```ts
// api.world.ts
export interface WorldBakeMessage { code: string; level: "error" | "warning"; args: string[]; message: string }
export interface WorldBakeReport { map: number; ok: boolean; style: "outdoor" | "indoor" | null; faces: number; model_bytes: number;
  model_share: number; model_kept: boolean; textures: string[]; vram_bytes: number; bdhc_bytes: number; bdhc_payload: number;
  plates: number; blocked: number; verbatim_model: boolean; verbatim_terrain: boolean; messages: WorldBakeMessage[] }
export interface HeightDeviation { x: number; z: number; height: number | null; step: number }
export interface WorldAdoptReport { map: number; tileset: number; tileset_guessed: boolean; deviations: HeightDeviation[]; change: number | null; bake: WorldBakeReport }
export const worldAdopt = (map: number) => invoke<WorldAdoptReport>("world_adopt", { req: { map } });
export const worldToBinary = (map: number) => invoke<WorldReport>("world_to_binary", { map });
export const worldBake = (map: number) => invoke<WorldBakeReport | null>("world_bake", { map });
// WorldSourceSection.tsx
export function WorldSourceSection(props: { map: number; world: boolean; textProject: boolean }): JSX.Element;
```

Texte und Attribute: Abschnitt „Quelle“ `[data-world-source="binary|world"]`. Binär im Textprojekt: Satz „Binäre Karte aus dem ROM oder aus DSPRE.“, Knopf „Karte als Quelle übernehmen“ `[data-world-adopt]`; danach Notice „Übernommen. 12 Kacheln weichen von der Höhe ab.“ (0: „Übernommen.“). Im DSPRE-Layout der Knopf deaktiviert mit „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“. Welt: „Welt-Karte“, Zeile `[data-world-bake="ok|refused"]` „Gebacken · Modell 21.740 B · Terrain 334 B“ bzw. Notice (Fehler) mit dem Satz der ersten Fehlermeldung, Knopf „Als binäre Karte übernehmen“. Konflikt mit `world`: Knöpfe „Neu backen“ (`studio`) und „Als binäre Karte übernehmen“ (`dspre`), darüber „Diese Welt-Karte wurde außerhalb des Studios geändert.“. Build-Zeile `world`: Name „Karten backen“. `mapModel`: Welt-Karte → `getWorldMesh`; `faces === 0` oder Fehler → `getMapModelGlb`.

- [ ] **Step 1: Scheiternde Tests:** `WorldSourceSection.test.tsx` (`binary_offers_adopt_in_the_text_project`, `adopt_shows_the_deviations`, `dspre_layout_disables_adopt`, `world_shows_the_bake_and_its_refusal`, `to_binary_calls_the_command`), `TextProjectSurface.test.tsx` (`world_conflict_offers_rebake_and_binary`), `stepWords.test.ts` (`world_step_is_named`), `worldMesh.test.ts` (`map_model_falls_back_to_the_glb_without_faces`, `…_on_error`).
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, die Check-Skripte grün.
- [ ] **Step 5: Browser-Teil** (Test-Umgebung `G4_VITE_PORT=1520 G4_BRIDGE_PORT=1521`, Kopie `6-3b-web` von Platin DE mit `.nds`, Textprojekt über `call.sh open_project`, `assign_base`, `adopt_text_project`): Zweiblattdorf, Inspector der Zelle von Karte 0000: `[data-world-source="binary"]`; Klick „Karte als Quelle übernehmen“ → `[data-world-source="world"]`, `[data-world-bake="ok"]`, Szene zeigt das GLB (kein `data-world-mesh`), Verlauf „Karte als Quelle übernommen: Karte 0000“; ⌘Z → binär; ⇧⌘Z; „Speichern“, dann Datei `data/maps/0000.json` ohne `model`; Konflikt: Member `unpacked/…/0000` per `cp` einer anderen Karte ersetzen, Technik › Textprojekt „Abgleichen“ → Konflikt mit „Neu backen“/„Als binäre Karte übernehmen“, „Neu backen“ → Member wieder gleich dem Bake (`shasum`); Technik › Veröffentlichen „Bauen“ → Zeile „Karten backen“. Hell, Dunkel, Kompakt; Bilder unter `target/agent/gate-<datum>/6-3b/blick-*.png`, Notiz `browser.md`.
- [ ] **Step 6: Commit** `feat(studio): source section with adopt and release, world conflicts, bake step row, GLB fallback` (mit Trailer).

### Task 11: Szenarien `pt-gemalt` und `hg-gemalt`, Lichtregel im Spiel

**Files:**
- Create: `scripts/agent/scenarios/pt-gemalt.yaml`, `scripts/agent/scenarios/hg-gemalt.yaml`
- Modify (nur bei positiver Probe, Step 1): `scripts/g4emu/g4emu/{emu.py, steps.py}` (Schritt `clock HH:MM`), `scripts/g4emu/tests/test_steps.py`

**Interfaces:**
- Consumes: Brückenbefehle `adopt_text_project`, `create_area`, `world_create`, `tileset_save`, `tileset_edit`, `world_edit`, `world_bake`, `get_map`, `save`; g4emu-Schritte von 6.2.

- [ ] **Step 1: Probe Uhr** (10 Minuten Zeitkasten): liest melonDSDS eine Kernoption oder Umgebung für Datum und Uhrzeit (`tools/g4emu/core/`, Optionsliste über `uv run g4emu` REPL)? **Ja:** Schritt `clock HH:MM` (setzt die Option vor `continue`), Test `clock_sets_the_core_option`; Nacht wird Agent + Blick. **Nein:** kein Code; Nacht ist Hand in melonDS (Gate-Datei). Ergebnis als Kopfkommentar in beide Szenarien.
- [ ] **Step 2: Kacheln wählen** auf `6-3b-szen-pt` bzw. `6-3b-szen-hg`: `tileset_get` des Pakets der Vorlage (Platin Route 201, Vorlage 342; HeartGold Vorlage 33), Bilder über `get_tileset_texture`; Indizes für Boden, Gras, Wasser und Wand wählen, die so aussehen; nur die Indizes kommen ins YAML.
- [ ] **Step 3: Szenarien** nach `pt-gelaende.yaml`/`hg-gelaende.yaml` (Patch, Scripts, **`adopt_text_project`**, `create_area` 1 × 1 mit neuem Header, Begegnungsdatei Rate 100, Geschenk, Warp über die Tür): dann `world_create {req: {map, fill: <Boden>}}` (`as: w`), `tileset_save {package: "${w.tileset}"}`, `tileset_edit` (Gras `grass` `kind 2`, Wasser `water` `kind 21`, Boden `wall: <Wand>`), `world_edit` mit den Zellen von `gate_world` (Rechtecke als Zelllisten, YAML-Anker), `world_bake` mit `expect` `ok: true`, `style: outdoor`; `get_map` mit `permissions.<z·32+x>`: Gras (16, 3) `[2, 0]`, Wasser (2, 24) `[21, 0]`, Plateau-Rand (22, 21) `[0, 128]`, Tür `[105, 128]`; `save`; `build: out.nds`. Spiel: `expect-height 0` auf (14, 21), `walk right 4`, `expect-at 18 21`, `expect-height 16`, `walk left 1`, `expect-height 12 0.1`, `walk right 1`, `walk-until-blocked right 8` + `expect-at 21 21`, zur Südseite des Plateaus und `walk-until-blocked up 4` + `expect-at 20 23` (die Wand), `shot tag`, ins Gras `walk-until-battle` + `expect-battle`; mit Uhr zusätzlich `clock 23:00`, `shot nacht`.
- [ ] **Step 4: Lauf** je Spiel auf frischen Kopien, Debug-Binaries aus `cargo build --workspace --all-features`: `uv run g4emu scenario ../agent/scenarios/pt-gemalt.yaml --project $K/6-3b-szen-pt_DSPRE_contents --out $W/target/agent/gate-$(date +%F)/6-3b/pt-gemalt --set NEUER_HEADER=593 --set S_WILD=150 --set L_WILD=5 --set S_GIFT=445 > target/agent/6-3b/pt-gemalt.log 2>&1`, ebenso `hg-gemalt` (`NEUER_HEADER=540`) → Rückgabe 0, `"ok": true`. Blick des Agenten auf `tag` (und `nacht`): Boden texturiert, Wand sichtbar, Rampe, Wasser und Gras erkennbar, Außenlicht (keine weiße Innenraumfarbe).
- [ ] **Step 5: Commit** `test(g4emu): scenarios pt-gemalt and hg-gemalt walk a map painted by commands` (mit Trailer).

### Task 12: Doku der Formate und Design-System-Nachtrag

**Files:**
- Modify: `docs/formats/world_map.md` (Grenze von 6.3a ersetzt: Backen, Schema 2, `terrain`, Verbatim, Permissions-Regel, Prüfungen, Store, Übernehmen, Rückweg; „Was später kommt“), `docs/formats/text_project.md` („Arten“ `maps`: Welt-Karten ohne abgeleitete Teile; „Manifest“ Schema 4, `maps.world` 2; „Schema-Version und Migration“ Schritt 3 → 4; „DSPRE parallel“ Welt-Karten; „Codes“), `docs/formats/build_state.md` (Abschnitt `world`, Schritt `world` in „Schritte“, „ohne Cache“ behält `world`), `docs/formats/tileset.md` (`tileset_edit`), `docs/formats/terrain.md` (`block_edges`, Lexer-Ort), `docs/formats/map.md` (Satz zu Welt-Karten), `design-system/README.md` (Abschnitt „Nachtrag 6.3b“)
- Create: `design-system/openpencil/NACHTRAG-6-3b.md`, `design-system/openpencil/beispiele/welt-quelle.body.js`

- [ ] **Step 1: Formate** wie unter Files; jede Zahl und jeder Wortlaut aus dem Code der Tasks 2 bis 7, Beispiele synthetisch (`fixtures::gate_world` mit dem synthetischen Tileset).
- [ ] **Step 2: README** „Nachtrag 6.3b“ nach dem Muster „Nachtrag 6.2“: Abschnitt „Quelle“ im Inspector (Texte, `data-*`), Konflikt mit zwei Wegen, Build-Zeile „Karten backen“, Bilder unter `target/agent/gate-<datum>/6-3b/`; keine neuen Tokens, keine neue Komponente; Abweichung: der Konflikt ist eine Zeile in Technik › Textprojekt wie jeder Konflikt des Abgleichs, kein eigener Dialog (Spezifikation §6 nennt „Dialog mit zwei Wegen“).
- [ ] **Step 3: Anleitung** `NACHTRAG-6-3b.md` nach `NACHTRAG-6-2.md` (Frames „App: Inspector, Karte als Quelle“ und „App: Textprojekt, Welt-Konflikt“, Vorlage `welt-quelle.body.js` gegen `jsx-lib-app.js`); die `.fig` nur, wenn OpenPencil antwortet, sonst der Satz „Die Frames von 6.3b folgen, sobald OpenPencil antwortet“.
- [ ] **Step 4:** `bash scripts/check-tokens.sh` grün; `cargo test -p studio table_covers_every_format_file` grün.
- [ ] **Step 5: Commit** `docs(formats): bake, schema 2, world step, adopt; design-system addendum 6.3b` (mit Trailer).

### Task 13: Gate 6.3b, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-6-3b-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 6.3b“ am Ende), `docs/ROADMAP.md` (Zeile 6 der Übersicht, „Stand 6.3b“ unter der Tabelle), `docs/dev-setup.md` (`tests/world_bake.rs` in der Tabelle `G4_TEST_PROJECT`)

- [ ] **Step 1: Gate-Datei** nach `docs/phase-6-3a-gate.md`: Kopf mit Stand und Commit; Gate-Text (§4 Zeile 6.3b und Phasen-Gate wörtlich wie in Global Constraints); „So gelesen“ (Abweichungen aus „Entscheidungen“: Schritt nach `textproject`, Schema 2, Verbatim-Modell plus Kacheln später, Nacht); Legende „Wer“; Namen in der Oberfläche; Vorbereitung (Kopien `6-3b-{pt,hgde,hgus}`, `6-3b-szen-{pt,hg}`, `6-3b-web`; Worktree, Ports 1520/1521, Last).
  - **(1) Automatik (Agent):** `cargo deny --locked check licenses advisories`, fmt, clippy, `env -u G4_TEST_PROJECT cargo test --workspace --all-features`, `pnpm test`/`pnpm build`, Stil, Tokens, Tiefe, `uv run pytest -q` in `scripts/g4emu`; gated Task 9 je Kopie; `vanilla_checks` je Kopie. `scripts/gate.sh`: **offen, Lauf in der Hauptsitzung auf main** (die neuen Tests laufen in den bestehenden Schritten mit, `gate.sh` bleibt unverändert).
  - **(2) Szenarien (Agent + Blick):** Task 11 mit Rückgabe, Erwartungen und Bildern.
  - **(3) Browser (Agent):** Task 10 Step 5.
  - **(4) Hand und Blick:** Bilder (Tag, Nacht falls vorhanden, Inspector Hell/Dunkel/Kompakt); `out.nds` von `pt-gemalt` und `hg-gemalt` in melonDS begehen, bei Tag und Nacht (Systemzeit des Emulators); DSPRE öffnet die gebaute Karte.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Form 1 byte-gleich (Karten) | abweichende Karten / Zellen | Projekt gebaut byte-gleich | gemalte Karte NSBMD (Anteil) | BDHC | Boden max. Abw. (E) | apicula | Schritt inkrementell | Konflikt beide Wege | `vanilla_checks` Fehler | Szenario | Last |
    |---|---|---|---|---|---|---|---|---|---|---|---|---|
    | Platin DE | 666 / 666 | | | | | | | | | 0 | | |
    | HeartGold DE | 676 / 676 | | | | | | | | | 0 | | |
    | HeartGold US | 676 / 676 | | | | | | | | | 0 | – | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte eintragen; Befunde beheben (eigene Commits `fix(6.3b): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: `agent-gate.md`** Abschnitt „Phase 6.3b“ nach „Phase 6.3a“: Kopien, Worktree, „Wo was ist“ (`[data-world-source]`, `[data-world-adopt]`, `[data-world-bake]`, Konfliktknöpfe, Build-Zeile, Befehle über `g4_call --expert`), Tabelle Punkt / Wer / Wie, die Befehle für (1), (2), (3) mit erwarteten Werten. **Roadmap:** „**Stand 6.3b <Datum>:** Gate vom Agenten bestanden: Form 1 byte-gleich auf 666 / 676 / 676 Karten, Projekt mit allen Karten als Welt baut byte-gleich, gemalte Karte <n> B (<Anteil>) mit BDHC <n> B, Boden ≤ <f> E, apicula gleich, Schritt `world` inkrementell, `world.member-changed` mit beiden Wegen, `pt-gemalt`/`hg-gemalt` begehbar; Branch `phase-6-3b-backen-build`; [docs/phase-6-3b-gate.md](phase-6-3b-gate.md); beim Nutzer offen: Blick, melonDS bei Tag und Nacht, DSPRE, `scripts/gate.sh` auf main“.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`; `git log main..HEAD | grep -c '＠'` gleich 0.
- [ ] **Step 5: Commit** `docs(6.3b): gate of bake and build, agent-gate section, roadmap` (mit Trailer).

---

## Schnittstellen für die Folgephasen

- **6.4a (Malen):** `world_edit`/`world_create` mit `WorldReport.bake` (Budget live aus `WorldBakeReport`: `model_bytes`, `model_share`, `bdhc_bytes`, `vram_bytes`, `messages`), `Store::world_bake_report`, `world_get.bake`, `world::bake::world_permissions` (Gelände-Muster und Kollision im Overlay), `tileset_edit`/`TileChange` (Tileset-Editor), `world_adopt` und `world_to_binary` (Knöpfe in Bauen), `ConflictEntry.world`, die Szenarien `pt-gemalt`/`hg-gemalt` (6.4a ersetzt die Befehlsfolge durch die Oberfläche), `FLOOR_TOLERANCE`, `fixtures::gate_world`.
- **6.5b (Freiform und Austausch):** `FreeformJson { glb, nsbmd: Option<Part>, at }`, die Stelle `bake_grid` mit `world.freeform-later` und `world.verbatim-mixed` (dort kommen glTF und das Raster obendrauf hinein), `Verbatim`, `WorldAdoptRequest.form` (`AdoptForm::Gltf` heute abgelehnt), `decode_quads` (Vergleich gebackener Modelle), `input_hash` (Beifiles `NNNN.<name>.glb` als weitere Eingabe), `world_salt`/`WORLD_BAKE_REV`.

## Offen für die Umsetzung

- **Laden backt:** jede geladene Welt-Karte backt einmal (ms je Karte); zeigt der Browser-Teil spürbare Ladezeit, bekommt `Store` einen Zwischenspeicher je `input_hash`.
- **`base` beim Laden mit abgelehntem Modell:** das Modell kommt dann aus dem Member auf der Platte; fehlt der Member, ist das Modell leer und `world.bake` erklärt es.
- **Gelöschte Karte mit Welt** (Grabstein aus 3d.4): der Schritt `world` backt keine Karte mit Grabstein; ein Test in Task 5, wenn `TOMBSTONE_ARTS` Karten enthält.
- **`create_area` aus einer Welt-Karte** kopiert die gebackene Binärkarte (siehe „Nicht in 6.3b“); der Satz steht in `world_map.md`.
- **Wasser ohne Animation:** ob es im Spiel stört, zeigt das Bild von Task 11 (Entscheidungen, C offene Frage 8); kein Code in 6.3b.
- **Parallele Merges:** Konflikte an den Anhängepunkten gesammelt lösen; `.fig` von main behalten; nach dem Merge von 6.5a `decode_quads` um Dreiecke erweitern und `BAKE_REV` prüfen.
