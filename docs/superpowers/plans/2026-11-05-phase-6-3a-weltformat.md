# Phase 6.3a „Weltformat und Vorschau“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eine Karte des Textprojekts kann ein Weltdokument tragen: `data/maps/NNNN.json` mit `source: "world"` und dem Objekt `world` (Version, Kartentextur-Paket, Ebenen mit Kacheln, Höhen und Rampen als 32 Zeilen, Teilmodelle, Freiform als Schema, Permissions mit Ableitung und Überschreibung); dazu `data/tilesets/NNNN.json` je Kartentextur-Paket, abgeleitet aus dem Paket und den Vanilla-Permissions; die Store-Art `WorldMap` mit „Welt anlegen“ und `world_edit` als je einem Undo-Schritt; ein Vernetzer, der aus dem Raster eine Flächenliste macht (zusammengefasste Böden, Wände, Rampen, Fußabdrücke), die über die Brücke in die Szene geht und dort statt des GLB gezeigt wird (`[data-world-mesh]`); Prüfungen `world.schema`, `world.tileset-mismatch`, `world.not-baked`, `world.budget`. Der Member der Karte bleibt bis 6.3b, wie er ist.

**Architecture:** Neues Kernel-Modul `crates/g4-kernel/src/world/` mit `doc.rs` (serde-Form der Datei), `grid.rs` (Zeilen-Grammatik, Prüfung, Raster), `tileset.rs` (Format, synthetisches Tileset, Ableitung, Texturen), `places.rs` (Paket ↔ Karten), `mesh.rs` (Vernetzer, Budget-Schätzung), `edit.rs` (Anlegen, Ändern), `keep.rs` (das Weltobjekt beim Schreiben des Members mitnehmen) und `check.rs` (Prüfungen im Textprojekt-Check). Das Textprojekt kennt `world` als optionales Feld von `MapJson`, die Ordnerart `tilesets` und den Schemaschlüssel `maps.world` über die Art-Beschreibung (`kinds.rs`) und hebt `schemaVersion` auf 3 (Migration ändert nur das Manifest). Der Store bekommt `AssetKind::WorldMap` als Store-Art ohne Archiv (wie `Trainer`), deren Bytes das kanonische JSON von `Option<WorldJson>` sind und die als zweites Asset in der Einheit `data/maps/NNNN.json` der Karte lebt (`Location::Derived`, kein Member). Tauri-Befehle in `commands/world_map.rs`, Frontend `world/scene/worldMesh.ts` plus ein Haken in `SceneHost.ts` und je eine Zeile in den Ladern.

**Tech Stack:** Rust (g4-kernel, g4-formats nur lesend: `nsbtx`, `nsbmd_header::material_texture_names`, `map`), serde/serde_json mit `preserve_order`, proptest, Tauri-Befehle und Brücke, MCP nur über `g4_call` (keine kuratierten Werkzeuge vor 6.4a), React 19 und three.js (`BufferGeometry`, `DataTexture`), vitest, Python 3 `-I` für die Messung in Task 0.

**Spec:** [2026-11-05-phase-6-kartenerstellung-design.md](../specs/2026-11-05-phase-6-kartenerstellung-design.md) §1, §2, §3.1, §3.2, §3.3, §3.5, §4 Zeile 6.3a (Gate wörtlich), §6, §7, §8; [entscheidungen.md](../../research/2026-11-05-phase-6/entscheidungen.md) (Tabellen „Phasenschnitt“ Zeile 6.3a, „Malen, Weltformat, Import, Tiled (Gutachten C)“ wörtlich, „Writer“ Zeilen „Eingabe des Bäckers“ und „Vorschau gebackener Modelle“, „BDHC“ Zeile „Höheneinheit“, „Nicht in Phase 6“, „Was bewusst nicht entschieden ist“); [Gutachten C](../../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §2.1, §2.4, §3.1, §3.2 (M1), §3.3, §3.5, §5.2, §5.3, §7.4, §8.2, §8.3, §9, §11 (Messwerkzeug `gutachten-c-kartentexturen.py`); [Gutachten A](../../research/2026-11-05-phase-6/gutachten-a-writer.md) §2.4, §2.5; [Gutachten B](../../research/2026-11-05-phase-6/gutachten-b-bdhc-engine.md) §2.6, §2.7; Muster: [Plan 3d.1](2026-10-30-phase-3d-1-flags-variablen.md) (Aufbau), [Plan 4.3b](2026-10-28-phase-4-3b-textprojekt-quelle.md) (Einheiten, `EntitySource`), [Plan 3d.6](2026-10-31-phase-3d-6-geschichten-lesen.md) (Art-Beschreibung, Dateischritt).

Stand 05.11.2026: `main` bei `d05fad84` (Phase-6-Spezifikation, Gutachten, Entscheidungen). Arbeit im Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-3a`, Branch `phase-6-3a-weltformat` (Basis `d05fad84`). Parallel laufen 6.1 (Worktree `phase-6-1`, Branch `phase-6-1-writer`) und 6.2 (Worktree `phase-6-2`, Branch `phase-6-2-bdhc`); 6.2 mergt vor 6.3a, Task 10 holt es herein. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Voraussetzungen (auf main `d05fad84`, vor Task 1 mit grep prüfen)

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `textproject::SCHEMA_VERSION`, `migrate::{MIGRATIONS, Tree, migrate}` | `= 2` (`mod.rs:78`); zwei Schritte `from: 0`, `from: 1` (`migrate.rs:34-43`), die Kette schreibt das Manifest zuletzt | Task 4 |
| `textproject::kinds::{DataKind, DATA_KINDS: [DataKind; 3], schemas, check_files}` | `flags`, `vars`, `stories`; `schemas` hält fremde Schlüssel (`kinds.rs:54, 89, 102`); `arts::is_known_folder` fragt `kinds::data_kind` (`arts.rs:142`) | Task 2, 4 |
| `schema::map::{MapJson, MapEntity, map_bytes, sample_map, MAP_SOURCE_BINARY}`, `codes::MAP_SOURCE_CODE` | `MapJson` mit `deny_unknown_fields`: `source`, `permissions`, `buildings`, `buildings_tail?`, `model`, `bdhc`, `bgs?` (`map.rs:34-46`); `map_bytes` lehnt `source ≠ "binary"` mit `textproject.map-source` ab (`map.rs:206`) | Task 4 |
| `export::{write_folder, entity_files}` mit der Hülle `file_deleted` (`export.rs:620-690`); `source/writers.rs` mit dem Muster `keep_eggs` (`writers.rs:211-235`) | eine JSON auf der Platte behält, was der Member nicht kennt | Task 4 |
| `source::units::{Place, place_of, key_of, json_rel}`, `source::{unit_of, EntitySource::{read, write}}` | `Place::Entity { art, index }`; `unit_of` setzt `assets` aus `art.kinds()` (`mod.rs:169-225`) | Task 6 |
| `project::{AssetKind::ALL: [AssetKind; 29], slot, label, Location (6 Varianten, `project.rs:283`), ArchiveMap::for_game, unpacked_name, member_location, ensure_unpacked, member_count}` | `Trainer` als Store-Art ohne Archiv (`project.rs:126, 455-458, 735, 928, 1009, 1328`) | Task 6 |
| `store::{Loaded, is_store_kind, Tx::edit, Store::{transaction, record_file_step, save, undo, redo, history}, FileStep, FileStepKind::Reset, HEADER_TABLE}` | `store.rs:91, 622, 1546, 2977, 3096, 3400`; `Loaded::is_editable` listet die Store-Arten (`store.rs:123`) | Task 5, 6 |
| `Project::map_tileset_member(tileset) -> Result<Option<Vec<u8>>>` (`project.rs:1046`), `Store::{header_table, matrix, map, map_tileset_index, member_count}`, `map_import::map_packs` (privat, `map_import.rs:992`, nur als Muster) | Paket eines Gebiets über `area_data.map_tileset()` | Task 5, 7 |
| `g4_formats::{nsbtx::{Nsbtx::read, decode, TextureInfo, PaletteInfo, Rgba}, nsbmd_header::material_texture_names, map::{MapFile, Cell, MAP_SIZE}}` | unverändert (6.1 baut `nsbmd.rs`, `nitro_dict.rs`, `texconv.rs`, nicht diese Leser um) | Task 5 |
| `tiles::tile_behavior(family, value) -> Option<TileBehavior { value, name, class }>` | Klassen `grass`, `water`, … (`tiles.rs:36`) | Task 5 |
| `textproject::check::hints` (`check.rs:108`), `text::{Problem, Severity}`, `locate::key_position`, `canon::{to_canonical, parse_json}` | Hinweise des Textprojekt-Checks | Task 4, 7 |
| `test_support::{TextFixture::{exported, dspre}, vanilla_copy}` (`test_support.rs:730, 869-884`); `tests/common::vanilla_project`; `tests/textproject.rs::{round_trip, export_idempotent, exported_copy}` (`:2498, :2619`) | Fixture des Textprojekts, gated Kopien | Task 3 bis 9 |
| Tauri `commands::{dispatch::COMMANDS, world::{MapDto, get_map_impl}, sprites::SpriteDto, open_synthetic, mod.rs (mod + pub use)}`, `lib.rs` `generate_handler!`, `mcp/access.rs::COMMAND_KINDS` mit `every_command_has_a_kind` | Anhängepunkte; `commands/world.rs` existiert (darum heißt die neue Datei `world_map.rs`) | Task 9 |
| Frontend `world/scene/{SceneHost.ts, useSceneLoader.ts:113, useRefreshQueues.ts:83, content.ts addModel}`, `world/sprites.ts` (`toTexture` nicht exportiert, `:33`), `reload.ts::reloadPlan` (Switch über `AssetKind`), `api.ts` (`AssetKind` `:415`, `MapDto` `:167`, `export * from` ab `:1087`), `ui/tokens/tokens.ts` (`szene.gelaende`), `workbench/useThumbnailReset.ts:21` (`resetSprites()`) | GLB je Karte über `getMapModelGlb(id, area).then(parseGlb)`, Modell um die Kartenmitte, 16 Einheiten je Kachel (`layout.ts:7`) | Task 11 |
| `scripts/gate.sh` (`run_step`, `run_timings`), `scripts/agent/{studio-web.sh, call.sh, mcp.sh}`, `docs/agent-gate.md` „Je Lauf“ | Test-Umgebung mit eigenen Ports, `g4_call` mit `agent_set_expert` | Task 11, 14 |

Fehlt eine Zeile oder weicht sie ab, zuerst die Abweichung in diese Tabelle schreiben und den betroffenen Task anpassen, dann beginnen.

## Parallele Teilphasen

- **6.1 Writer** (Worktree `phase-6-1`, Branch `phase-6-1-writer`) baut `g4-formats/src/{nitro_dict.rs, nsbmd.rs, nsbtx.rs, texconv.rs}`, `g4-kernel/src/bake/*` und `map_import.rs`. **6.3a berührt diese Dateien nicht**; Texturen liest es über die heutigen Leser (`Nsbtx::read`, `decode`, `nsbmd_header::material_texture_names`), Karten eines Pakets über eine eigene Funktion in `world/places.rs` (Muster `map_import::map_packs`, das privat bleibt). 6.1 definiert `bake::Face` in derselben Form wie `world::mesh::MeshFace`; 6.3b verbindet beide per `From`.
- **6.2 BDHC** (Worktree `phase-6-2`, Branch `phase-6-2-bdhc`) baut `g4-formats/src/bdhc.rs`, `g4-kernel/src/terrain/*`, neue Prüfungen in `checks/*`, `commands/` (`get_map_heights`, Höhenraster setzen), **`apps/studio/src/world/scene/*`** (Höhenquelle in `SceneHost`, Picking, Marker, Drag, Overlay, Schicht „Höhen“) und `scripts/g4emu`. **6.2 mergt vor 6.3a.** 6.3a nimmt Höhen und Rampen aus dem Weltdokument, nicht aus `bdhc.rs`; seine Prüfungen laufen im Textprojekt-Check (`textproject/check.rs`), nicht in `checks/*`. Die Szenen-Änderung bleibt auf die neue Datei `worldMesh.ts`, einen Haken in `SceneHost.ts` und je eine Zeile in `useSceneLoader.ts` und `useRefreshQueues.ts`; Task 10 mergt main nach dem Merge von 6.2 und vor den Frontend-Tasks herein.
- Anhängepunkte (nur anhängen): `crates/g4-kernel/src/lib.rs`, `commands/dispatch.rs`, `commands/mod.rs`, `src-tauri/src/lib.rs`, `mcp/{access,catalog,tools}.rs` (6.3a hängt nur an `access.rs` an), `apps/studio/src/api.ts`, `docs/mcp.md`, `docs/ROADMAP.md`, `docs/agent-gate.md`, `design-system/README.md`, `scripts/gate.sh`, `textproject/kinds.rs`.

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Lage von `world` in der Datei | `MapJson` bekommt `world: Option<WorldJson>` direkt nach `source`; `source: "world"` genau dann, wenn `world` da ist (sonst `textproject.map-source`: „source: „world“ braucht das Objekt `world`.“ bzw. „world: nur mit source „world“.“). `permissions`, `buildings`, `buildings_tail`, `model`, `bdhc`, `bgs` bleiben Pflicht wie heute: **sie sind bis 6.3b der bisherige Member**, aus dem der Import die Karte baut; `world` fließt in 6.3a in keinen Member | Spezifikation §3.1 („`buildings` und `bgs` wie heute“), Auftrag „der Member entsteht in 6.3b“; so bleibt jeder Build byte-gleich |
| Form des Weltobjekts | Schlüssel in dieser Reihenfolge: `version` (1), `tileset` (Paketnummer), `layers`, `pieces` (leer weggelassen), `freeform` (leer weggelassen), `permissions` `{ "derive": true, "override"?: [32 Zeilen] }`. Je Ebene `name`, `role` (`ground` \| `overlay`), `tiles` (32 Zeilen zu 32 Zellen, ein Leerzeichen), nur die Bodenebene `heights` (Pflicht) und `ramps` (weggelassen ohne Rampe). Alle Typen `deny_unknown_fields` | C §5.2; „Brücken und zwei Ebenen an einer Stelle: nicht in Phase 6“, darum tragen nur die Bodenebene Höhen, Überlagerungen folgen ihr |
| Zell-Grammatik | Kachel: `.` oder vier Hex-Ziffern (`0012`), optional `r1`/`r2`/`r3` (90°-Schritte gegen den Uhrzeigersinn von oben), dann optional `f` (Spiegeln an der z-Achse nach der Drehung); groß geschrieben angenommen, geschrieben klein. Höhe: ganze Zahl −16 bis 62 (Stufe zu 8 Einheiten). Rampe: **eigene Zeilen `ramps`**, Zelle `.` oder `n1 n2 e1 e2 s1 s2 w1 w2`. Override: `.` oder `kkcc` wie die Permissions | Suffix an der Höhe hätte zwei Bedeutungen in einem Token; eigene Zeilen zeigen eine gesetzte Rampe als eigene geänderte Zeile im Git-Diff |
| Rampe | `world::grid::Ramp { None, North(u8), East(u8), South(u8), West(u8) }` mit 1 oder 2 Stufen; **die Richtung ist bergauf**: die Zelle liegt an der gegenüberliegenden Kante auf `h`, an der genannten Kante auf `h + Stufen` (`East(1)` bei `h = 0`: Westkante 0, Ostkante 8 Einheiten) | B §2.5/§2.6 („Rampe 0 → 16 nach Osten (8 je Kachel)“); dieselbe Form wie 6.2s `terrain::Ramp`, Task 10 legt `From` in beide Richtungen an |
| Koordinaten des Netzes | Welteinheiten um die Kartenmitte (x, z von −256 bis 256, 16 je Kachel, z wächst nach Süden), Höhe `y = Stufe × 8`; Zelle (cx, cz) überdeckt x `16·cx − 256` bis `16·cx − 240` | so liegen die Vanilla-Modelle und das GLB (`layout.ts:6`, `content.ts:37` „centred on the map centre“); der Bäcker in 6.3b übernimmt die Ecken ohne Umrechnung |
| UV und Drehung | welt-gebunden, 1 Texel je Einheit (16 je Kachel): ohne Drehung `(u, v) = (x, z)`; `r1` `(z, −x)`, `r2` `(−x, −z)`, `r3` `(−z, x)`; `f` danach `u = −u`. Wände: `u` die Koordinate entlang der Kante (x bei Nord/Süd-Kanten, z bei West/Ost), `v = −y` | linear in x und z, darum ändert Zusammenfassen nichts; ±2 048 Texel (`TEXCOORD`) werden nie erreicht (A §2.5) |
| Zusammenfassen | gierig je Ebene, Zeile für Zeile: Schlüssel = (Kachel-Token, Höhe, Rampe); erst in x verlängern, dann ganze Zeilen in z. **Rampenzellen** fassen nur quer zum Hang zusammen (`North`/`South` nur in x, `East`/`West` nur in z) | ein Rechteck gleicher Rampe entlang des Hangs wäre eine falsche Ebene; Gate „eine Kachel ≤ 4, Schachbrett ≤ 1 024“ |
| Wände | zwischen zwei Bodenzellen derselben Karte, deren Oberkanten an der gemeinsamen Kante verschieden hoch sind: eine senkrechte Fläche vom unteren zum oberen Profil, Vorderseite zur niedrigeren Zelle; Textur = `wall` der Kachel der oberen Zelle im Tileset, ohne `wall` deren eigene Kachel; benachbarte Wandstücke gleicher Textur, gleicher Unter- und Oberkante auf einer Linie werden eine Fläche; **keine Wand** an der Kartengrenze, zu einer leeren Zelle (`.`) und wo sich die Profile kreuzen (gegenläufige Rampen nebeneinander) | C §3.3 („Wandtextur des Kantensatzes, Metadatum `wall` der oberen Kachel“); Nachbarkarten sieht erst 6.4b; der Kreuzungsfall ist selten und ein Dreieck wäre eine Sonderform |
| Teilmodelle in 6.3a | `pieces[i] = { tile, at: [x, z], rotate: 0 \| 90 \| 180 \| 270 }`; die Kachel muss Rolle `piece` mit `footprint: [b, t]` haben; der Vernetzer legt je Stück **eine** waagerechte Fußabdruck-Fläche in Höhe der höchsten Bodenzelle darunter mit der Textur der Kachel und `layer = PIECE_LAYER = 255`; die Geometrie kommt mit den Beifiles in 6.5b | Auftrag „Teilmodell-Fußabdrücke“; die Vorschau zeigt, wo ein Stück steht, ohne Format vorwegzunehmen |
| Überlagerungen | Ebenen ab 1 (`overlay`) folgen Höhe und Rampe der Bodenebene, werfen keine Wände, `layer` = Index; die Szene setzt `polygonOffset` je Ebene | „eine Ebene je Zelle“ (Nicht in Phase 6: Brücken) |
| Reihenfolge der Flächen | Ebene aufsteigend; je Ebene erst Böden in Reihenfolge ihrer Startzelle (Zeile, dann Spalte), dann Wände (Nordkanten zeilenweise, dann Westkanten spaltenweise, je Kante die Richtung zur niedrigeren Zelle), zuletzt Fußabdrücke in Listenreihenfolge | deterministisch ohne Hash-Reihenfolge; der Bäcker bildet Streifen über Läufe gleicher Textur |
| Budget-Schätzung | `estimate_model_bytes(faces, textures) = 1 024 + 600 · textures + 56 · faces` (Vierecke mit `TEXCOORD` und `VTX_16`, also eine obere Schranke; A §2.5: 1 024 Flächen, eine Textur 57 776 Byte gemessen, Schätzung 58 968); Prüfung `world.budget` (Info) | Spezifikation „Budget-Schätzung aus der Flächenzahl (Hinweis)“; die echte Zahl liefert der Bäcker in 6.3b |
| Tileset-Datei | `data/tilesets/NNNN.json` (Stamm = Paket): `{ version: 1, package, tiles: [...], edge_sets?: [] }`, je Kachel `texture`, `texture_index`, `palette?`, `size: [b, h]` (Texel), `role` (`ground`, `wall`, `ramp`, `water`, `grass`, `piece`), `kind`, `collision`, `height` (`flat`, `ramp`, `wall`), `sound?` (nur HGSS, Bits 0–6), `wall?` (Kachelindex), `footprint?`, `model?`, `origin` (`vanilla`, `unclear`, `synthetic`, `user`). Kachel `i` der Ableitung = Textur `i` des Pakets. **Reserviert:** `edge_sets` muss leer sein (6.4b), `model` muss fehlen (6.5b, Beifile `NNNN.<name>.glb`) | Spezifikation §3.2; Zahlen als JSON-Zahlen, die Oberfläche zeigt sie hex |
| Wann die Tileset-Datei entsteht | **nicht** beim Anlegen einer Welt: `Store::tileset(p)` liefert die Datei, wenn es sie gibt (neu gelesen, wenn sich Größe oder Zeit ändern), sonst `textures_only` (Kacheln aus den Texturen ohne Vorgaben, je Sitzung zwischengespeichert); die Ableitung mit Vorgaben (`derive_tileset`) läuft nur in `tileset_get` ohne Datei und in `tileset_save`. Geschrieben wird die Datei nur mit `tileset_save` (ein Dateischritt, Undo entfernt sie wieder) | ein Weltschritt und ein Dateischritt wären zwei Verlaufseinträge; Vernetzen und Prüfen brauchen nur Namen und Größen, das Lesen aller Karten eines Pakets bremste die Brücke; die Datei braucht erst der Tileset-Editor (6.4a) |
| Vorgaben je Textur | Regel und Schwelle legt **Task 0** aus zwei Kandidaten fest, beide aus den heutigen Lesern berechenbar: **K1 Mehrheit:** für Textur T des Pakets P die Karten M_T (Karten von P, deren Modell T über `material_texture_names` bindet); `votes(k) = Σ_{m∈M_T} Zellen_m(k) / |Namen_m|`; `k*` = größtes `votes` (Gleichstand: kleineres `k`); entschieden, wenn `votes(k*) / Σ votes ≥ UNIQUE_SHARE` (Vorgabe 0,6). **K2 Abhebung:** `p_T(k) = votes(k) / Σ votes`, `base(k)` = Anteil von `k` an allen Zellen der Karten von P; `k*` = größtes `p_T(k) / base(k)` über `k ≠ 0` mit `p_T(k) ≥ 0,05`; entschieden, wenn der Quotient ≥ 2; sonst Kind 0, wenn `p_T(0) ≥ UNIQUE_SHARE`, sonst unklar. Unklar oder ohne Karte: `kind 0`, `collision 0`, `sound 0`, `origin: unclear`. Kollision: `0x80`, wenn mindestens die Hälfte der gewichteten Zellen mit `k*` Bit 7 trägt; HGSS-Geräusch: häufigstes `collision & 0x7F` dieser Zellen | Entscheidungen „Ableitung aus den Vanilla-Permissions (Textur → häufigstes `kind`), Eindeutigkeit misst Task 0“; nur Karten-weite Zählung ist ohne den Display-List-Codec von 6.1 möglich |
| Rolle aus `kind` | `tile_behavior(family, kind).class`: `grass` → `grass`, `water` → `water`, sonst `ground`; `height: flat`; `wall`, `ramp`, `piece` leitet nichts ab | Spezifikation §3.2 („unklare Texturen sind Boden und im Tileset-Editor umstellbar“) |
| Palette einer Kachel | die Palette `<Textur>_pl`, sonst die gleichnamige, sonst keine (`palette` fehlt; dekodiert mit Palette 0); Format 7 (direkt) ohne Palette | Task 0 zählt, wie oft welcher Fall eintritt; ab 5 % „sonst keine“ ergänzt Task 5 die Regel „Palette mit dem Index der Textur“ |
| Synthetisches Tileset | `Tileset::synthetic()` (nur `test-support`), Paket `SYNTHETIC_PACKAGE = 0`, sechs Kacheln: 0 `syn_boden` 16×16 `ground` (`wall: 3`), 1 `syn_gras` 32×32 `grass` `kind 0x02`, 2 `syn_wasser` 16×16 `water` `kind 0x15`, 3 `syn_wand` 16×16 `wall` `height wall` `collision 0x80`, 4 `syn_rampe` 16×16 `ramp` `height ramp`, 5 `syn_stueck` 16×16 `piece` `footprint [2, 2]`; alle `origin: synthetic` | Entscheidung „eigene Tilesets nur als synthetisches Test-Tileset aus Code“; Namen ≤ 16 Byte |
| Store-Art | `AssetKind::WorldMap` (ALL[29], `label` „world map“), Store-Art ohne Archiv wie `Trainer`; Index = Kartennummer, `member_count(WorldMap) = member_count(Map)`; Bytes = `to_canonical(&Option<WorldJson>)` (eine Karte ohne Welt: `null`); `Loaded::World(Option<WorldJson>)`; Ort `Location::Derived { json }` (kein Member: Lesen außerhalb des Textprojekts lehnt ab, Member schreiben schreibt nichts); im Textprojekt ist `place_of(WorldMap i) = Place::Entity { art: Maps, index: i }` und `unit_of` der Karte nennt `assets = [Map i, WorldMap i]` | eine Einheit, ein Schreiben der JSON (Muster: die Art aus drei Assets); `null` statt eines fehlenden Assets erspart dem Store das Anlegen und Löschen von Assets, Undo von „Welt anlegen“ setzt einfach `null` |
| Schreiben der Karten-JSON | der Writer der Karte nimmt `source` und `world` aus dem WorldMap-Asset (gegeben, sonst wie der Store es hält, sonst von der Platte, `world::keep::carry_world`), den Rest aus den Map-Bytes; der volle Export (`write_folder` für `maps`) nimmt `source`/`world` von der Platte wie die Grabstein-Hülle | sonst löschte jedes Speichern einer Permission und jeder Export das Weltobjekt |
| `world.not-baked` | Info je Welt-Karte im Textprojekt-Check: „Karte 0123 ist eine Welt-Karte; bis zum Backen (6.3b) baut der Build ihren bisherigen Member.“ | Auftrag; der Konflikt `world.member-changed` kommt mit 6.3b |
| Prüfungen | im Textprojekt-Check (`hints`), nicht im Prüf-Register (`checks/*` gehört 6.2): `world.schema` (Fehler: Zeilen, Token, Bereiche, Kachel außerhalb des Tilesets, Paket außerhalb, Teilmodell, Override; serde-Formfehler bleiben `textproject.schema` mit Pfad `world.…`), `world.tileset-mismatch` (Warnung: ein Header, dessen Matrix die Karte zeigt, nennt Gebietsdaten mit anderem `map_tileset`), `world.not-baked` (Info), `world.budget` (Info, „Geschätzt höchstens 1 680 Byte (3 % von 0xF000) aus 1 Fläche und 1 Textur.“; über 0xF000 dazu „; mehr als das Modell fassen darf, das Backen (6.3b) lehnt es ab, wenn es nicht kleiner wird“) | Spezifikation „Prüfungen“, Auftrag „`world.schema` über `textproject.schema`“ |
| Schemaversion | `SCHEMA_VERSION = 3`; Migration `v2_to_v3` setzt nur `schemas["maps.world"] = 1` und `schemas["tilesets"] = 1` im Manifest; `kinds.rs` hängt `FILE_KINDS: [FileKind; 1]` (`tilesets`, ohne Register) und `PART_SCHEMAS: [(&str, u32); 1] = [("maps.world", 1)]` an, `schemas()` schreibt beide | C §5.2 („Migration der Projektversion einmal“); ein älteres Studio sieht `Newer` und liest nur; der Snapshot `schema.snap` verlangt die Version |
| Nur im Textprojekt | `world_create`, `world_edit`, `tileset_save` lehnen im DSPRE-Layout mit `WORLD_NEEDS_TEXT_PROJECT` = „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“ ab; `world_get` antwortet dort `world: null`, `tileset_get` und `get_world_mesh` gehen (Ableitung, leeres Netz) | Entscheidungen „Nur im Textprojekt“ wörtlich |
| `world_edit` | `{ map, label?, changes: [ { layer?, cells: [[x, z], …], tile? \| height? \| ramp? \| permission? } ] }`, je Änderung genau ein Feld, Token wie in der Datei; Höhe und Rampe nur Ebene 0, `permission` ohne `layer`; alle Änderungen eines Aufrufs sind **ein** Undo-Schritt, Label vorgegeben „Welt geändert: Karte 0123 (37 Zellen)“; geschrieben werden nur die berührten Zeilen (normalisiert), die anderen bleiben Byte für Byte | Spezifikation §3.6 „ein Strich = ein Undo-Schritt“; C §7.2 nennt Werkzeug-Ops (`paint`, `rect`, `fill`): die rechnet 6.4a auf Zellen herunter |
| `world_create` | `{ map, tileset?, fill?, height? }`: Bodenebene „Boden“ ganz mit `fill` (Vorgabe `0000`), Höhe `height` (Vorgabe 0), keine Rampe, `permissions { derive: true }`; `tileset` fehlt → das erste Paket aus `places::packs_of_map` (keins: „Karte 0123 liegt in keiner Matrixzelle mit Header; Tileset angeben (`tileset`).“); Label „Welt angelegt: Karte 0123“ | Rezept `new_painted_map` (6.4a) baut darauf |
| MCP | keine kuratierten Werkzeuge in 6.3a (`world_get`, `world_edit`, `tileset_get` kommen mit 6.4a); die Befehle gehen über `g4_call` (`--expert`), `access.rs` klassifiziert: `world_get`, `tileset_get`, `get_world_mesh`, `get_tileset_texture` `Read`; `world_create`, `world_edit`, `tileset_save` `Write` | Spezifikation §4 Zeile 6.4a; der Katalog-Test `the_curated_set_is_the_decided_one` bleibt |
| Netz-DTO | `WorldMeshDto { map, tileset, faces, textures: [{ name, width, height, role, palette? }], corners: Vec<i32> (12 je Fläche), uv: Vec<i32> (8 je Fläche), texture: Vec<u16> (Index in `textures`), layer: Vec<u8> }` | flache Felder: ein voller Plan unter 50 ms im Debug-Build |
| Texturen in der Szene | `get_tileset_texture(package, texture) -> Option<SpriteDto>` (RGBA mit der Palette der Kachel), im Frontend wie die Sprites über `toTexture` (exportiert), `RepeatWrapping`, `NearestFilter`, `alphaTest 0.5`, `uv = (u / Breite, −v / Höhe)`; ohne Textur (`null`, synthetisches Tileset) ein Platzhalter `MeshLambertMaterial` mit `szene.gelaende` nach Rolle: `grass` `hohes-gras`, `water` `wasser`, `wall` `blockiert`, `piece` `spezial`, sonst `unbekannt` | „Texturen aus dem Paket über den bestehenden Weg für Texturen“; keine neuen Tokens |
| `[data-world-mesh]` | das Canvas des 3D-Views trägt `data-world-mesh="<Karten aufsteigend, Komma>"` und `data-world-mesh-faces="<Summe>"`, solange mindestens eine Welt-Karte gezeigt wird; ohne Welt-Karte fehlen beide | Test-API liest Attribute, nicht WebGL |
| Zeitgrenzen | Vernetzen einer vollen Karte (`fixtures::busy_world`) ≤ 20 ms im Release-Build (Median aus 20 Läufen, `G4_TIMING_STRICT=1`, Schritt in `gate.sh`); `get_world_mesh` über `dispatch` warm ≤ 50 ms im Debug-Build (Studio-Test, strikt nur mit `G4_TIMING_STRICT=1`) | Gate §4; ohne die Variable drucken die Tests nur (Muster `tests/flags.rs:32`) |
| Nicht in 6.3a | Malwerkzeuge, Palette, Draufsicht-Änderung, Höhen-Overlay (6.4a), Kantensätze (6.4b), Backen, Build-Schritt `world`, `world.member-changed`, „Karte als Quelle übernehmen“ (6.3b), Beifiles und Freiform-Geometrie (6.5b), Vorschaubilder gemalter Karten aus dem Netz (`thumbnailStore` bleibt beim GLB), kuratierte MCP-Werkzeuge | Spezifikation §4, Entscheidungen „Nicht in Phase 6“ |

## Global Constraints

- Arbeitsort: Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-3a`, Branch `phase-6-3a-weltformat`; nie `cd` in die Hauptkopie, nie Branch wechseln, nie nach main mergen (Task 10 holt main **in** den Worktree), kein `git stash`. `tools/apicula` per Symlink auf den Haupt-Checkout (`ln -s /Users/POLTEP/Projekte/lab/editor/gen4-studio/tools/apicula tools/apicula`, nie einchecken) oder `export G4_APICULA=/Users/POLTEP/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula`.
- cargo braucht `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`; `pnpm install` einmal in `apps/studio`.
- Jeder Task endet mit einem Commit; jede Nachricht endet mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`).
- `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` immer absolut. Originale nie öffnen oder ändern; Testkopien nur `Roms/_testkopien/6-3a-<zweck>` (`cp -cR` für den Ordner `…_DSPRE_contents`, `cp -c` für die `.nds` daneben, Ziel existiert noch nicht). Gated Tests lesen `G4_TEST_PROJECT` und schreiben nur auf `vanilla_copy()`. **Keine ROM-Daten und keine Vanilla-Tileset-Metadaten im Repo** (keine Texturnamen, keine abgeleiteten Tilesets, keine Member); Fixtures synthetisch; Task 0 schreibt nur Zahlen.
- Test-Umgebung nur auf `G4_VITE_PORT=1460 G4_BRIDGE_PORT=1461` (`bash scripts/agent/studio-web.sh`); andere Ports nie anfassen.
- Lang laufende Befehle (gated Tests, `gate.sh`, Release-Builds, die Brücke) als Hintergrund-Befehle mit Log unter `target/agent/` (`… > target/agent/6-3a-<zweck>.log 2>&1`), dann das Log lesen.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh`; immer `bash scripts/check-no-rom-data.sh`.
- UI nur aus Bausteinen und Tokens; deutsche Texte, englische Bezeichner; three.js-Farben nur aus `ui/tokens/tokens.ts`.
- Anhängepunkte nur anhängen; die Dateien von 6.1 nicht anfassen; in `world/scene/*` nur `worldMesh.ts` (neu), der Haken in `SceneHost.ts` und je eine Zeile in `useSceneLoader.ts`/`useRefreshQueues.ts`.
- Gate 6.3a (Spezifikation §4, wörtlich): „Für jedes Kartentextur-Paket von Platin DE (74) und HeartGold DE (106) leitet der Kernel ein Tileset ab, dessen Bodenkacheln die Zahl der Texturen treffen (3 130 / 3 659); Task 0 misst die Eindeutigkeit der Vorgaben; eine 32 × 32-Karte aus einer Kachel vernetzt zu ≤ 4 Flächen, ein Schachbrett aus zwei Kacheln zu ≤ 1 024; Vernetzen einer vollen Karte ≤ 20 ms im Release-Build, Netz über die Brücke ≤ 50 ms im Debug-Build; Export idempotent, Round-Trip der JSON byte-gleich, Migration `maps.world` ändert keine andere Datei; ⌘Z nimmt einen Strich (über einen Kernel-Befehl) als Ganzes zurück; die Szene zeigt das Netz einer Welt-Karte mit `[data-world-mesh]`“, dazu gated Tests auf frischen Kopien von Platin DE, HeartGold DE und HeartGold US, Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer.

## Review Focus

- Ein Speichern oder Export, der die Karte aus ihren Member-Bytes neu schreibt (Permission gemalt, Gebäude verschoben, „Auf Original zurücksetzen“ der Karte, zweiter Export): das Weltobjekt bleibt Byte für Byte (Task 4 `writing_a_map_keeps_its_world`, `export_keeps_a_world_map`; Task 6 `a_map_edit_keeps_the_world`).
- Hand-editierte Welt-JSON mit Fehlern (31 Zeilen, `00g2`, Höhe 70, `n3`, Kachel 0x0120 bei 94 Kacheln, Überlagerung mit `heights`, unbekannter Schlüssel): der Check nennt Datei, Pfad und Zeile, der Import lehnt nur diese Karte ab, `world_edit` lehnt mit dem Text ab und ändert nichts (Task 1 `parse_reports_every_error_with_its_path`, Task 7 `schema_problems_name_path_and_line`, Task 6 `world_edit_refuses_a_broken_world_unchanged`).
- DSPRE-Layout und ältere/neuere Schemaversion: `world_create`/`world_edit`/`tileset_save` lehnen mit `WORLD_NEEDS_TEXT_PROJECT` ab, `get_map` sagt `world: false`, die Szene zeigt das GLB (Task 6 `dspre_refuses_with_the_way`, Task 9 `world_commands_in_the_dspre_layout`).
- Ränder des Rasters: Rampe an der Kartengrenze, leere Zellen neben hohen, Höhe −16 und 62, gegenläufige Rampen nebeneinander, Teilmodell am Rand (`at` + `footprint` > 32): kein Panik, keine Wand ins Leere, Fußabdruck abgelehnt mit `world.schema` (Task 3 `edges_of_the_grid_mesh_without_panic`, Task 1 `a_piece_must_fit_on_the_map`).
- Undo über die Grenze von Welt und Karte: `world_create`, `set_permission`, `world_edit`, dann dreimal ⌘Z und dreimal ⇧⌘Z: jeder Schritt einzeln, die Datei nach dem Speichern jeweils gleich dem Stand des Schritts, der Member nie verändert (Task 6 `world_and_map_steps_undo_one_by_one`).

---

### Task 0: Messung: wie eindeutig nennen die Vanilla-Permissions das Gelände einer Textur

**Files:**
- Create: `docs/research/2026-11-05-phase-6/task0-6-3a-vorgaben.py`, `docs/research/2026-11-05-phase-6/task0-6-3a-vorgaben.md`

**Interfaces:**
- Produces: die Wahl `VoteRule::K1 | K2 | None` und `UNIQUE_SHARE` für Task 5, die Palettenregel, die Vergleichszahlen für den gated Test von Task 5.

- [ ] **Step 1: Kopien** `cp -cR "/Users/POLTEP/Projekte/lab/editor/Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" /Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/6-3a-task0-pt`, ebenso `6-3a-task0-hgde` (HeartGold DE) und `6-3a-task0-hgus` (HeartGold US), Namen wie in `docs/agent-gate.md` „Je Lauf“.
- [ ] **Step 2: Skript** `task0-6-3a-vorgaben.py` (Kopf wie `gutachten-c-kartentexturen.py`: liest nur, druckt nur Zahlen, keine Namen, keine Pixel; `read_btx`, `run_dl`, `split`, `sbc_shape_materials`, `read_mdl` wörtlich übernommen, weil `python3 -I` keine Nachbardatei lädt). Je Spiel: (a) Karten je Paket wie Gutachten C (bester Treffer der Texturnamen); (b) **Wahrheit**: je waagerechtem Polygon mit allen Ecken auf dem 16er-Raster die überdeckten Zellen und ihr `kind`, je Textur die Mehrheit (Anteil ≥ 0,6, sonst „keine“); (c) **K1** und **K2** genau wie in „Entscheidungen“, K1 für `UNIQUE_SHARE` ∈ {0,6, 0,7, 0,8, 0,9}; (d) Übereinstimmung mit der Wahrheit über die Texturen mit Wahrheit; Verteilung der Anteile (min/Median/p90); Zahl entschiedener Texturen und mit `k* ≠ 0`; Kollision und HGSS-Geräusch wie entschieden; Paletten: `_pl` / gleichnamig / keine; Texturen gesamt und Pakete (Kontrolle 3 130 / 74, 3 659 / 106, US gleich DE).
- [ ] **Step 3: Lauf** `python3 -I docs/research/2026-11-05-phase-6/task0-6-3a-vorgaben.py /Users/POLTEP/Projekte/lab/editor/Roms/_testkopien/6-3a-task0-pt --json > target/agent/6-3a-task0-pt.json 2> target/agent/6-3a-task0-pt.log` (Hintergrund), ebenso hgde, hgus.
- [ ] **Step 4: Festlegen** nach dieser Regel: gewählt wird der Kandidat mit der höheren Übereinstimmung (Gleichstand: K1); bei K1 der kleinste `UNIQUE_SHARE`, der ≥ 70 % Übereinstimmung erreicht, sonst 0,9; erreicht der gewählte Kandidat unter 50 %, ist die Wahl `None` (jede Textur `unclear`, Rolle `ground`). Palettenregel: Anteil „keine“ ≥ 5 % → Task 5 ergänzt „Palette mit dem Index der Textur“.
- [ ] **Step 5: Protokoll** `task0-6-3a-vorgaben.md` nach dem Muster `docs/research/2026-10-30-phase-3d/task3-3d3-formen.md`: Gegenstand, Vorgehen, Tabellen je Spiel (Pakete, Texturen, Karten je Paket, Wahrheit, K1 je Schwelle, K2, Übereinstimmung, Kollision, Paletten), Entscheidung mit Zahlen, Grenzen (Karten-weite Zählung, Polygone außerhalb des Rasters). Nur Zahlen.
- [ ] **Step 6: Commit** `docs(6.3a): task 0, how clearly vanilla permissions name the terrain of a texture` (mit Trailer).

### Task 1: Weltdokument: Datei-Form, Zeilen-Grammatik, Raster und Prüfung

**Files:**
- Create: `crates/g4-kernel/src/world.rs` (Modul-Doku, `pub mod doc; pub mod grid;` und die Module der Folgetasks, Re-Exporte), `crates/g4-kernel/src/world/doc.rs`, `crates/g4-kernel/src/world/grid.rs` (+ Tests je Datei)
- Modify: `crates/g4-kernel/src/lib.rs` (am Ende `pub mod world;` mit Leerzeile davor wie `pub mod delete;`)

**Interfaces:**
- Produces:

```rust
// world/doc.rs
pub const MAP_SOURCE_WORLD: &str = "world";
pub const WORLD_VERSION: u32 = 1;
pub const TILE_UNITS: i32 = 16;          // Welteinheiten und Texel je Kachel
pub const STEP_UNITS: i32 = 8;           // Einheiten je Höhenstufe
pub const HEIGHT_MIN: i8 = -16;
pub const HEIGHT_MAX: i8 = 62;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldJson { pub version: u32, pub tileset: u16, pub layers: Vec<LayerJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub pieces: Vec<PieceJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub freeform: Vec<FreeformJson>,
    pub permissions: PermissionsJson }
#[derive(…)] #[serde(deny_unknown_fields)]
pub struct LayerJson { pub name: String, pub role: LayerRole, pub tiles: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub heights: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub ramps: Option<Vec<String>> }
#[derive(…, Copy)] #[serde(rename_all = "snake_case")] pub enum LayerRole { Ground, Overlay }
#[derive(…)] #[serde(deny_unknown_fields)] pub struct PieceJson { pub tile: u16, pub at: [u8; 2], #[serde(default)] pub rotate: u16 }
#[derive(…)] #[serde(deny_unknown_fields)] pub struct FreeformJson {   // nur Schema bis 6.3b/6.5b
    #[serde(default, skip_serializing_if = "Option::is_none")] pub glb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub nsbmd: Option<String>,
    #[serde(default)] pub at: [i32; 3] }
#[derive(…)] #[serde(deny_unknown_fields)] pub struct PermissionsJson { pub derive: bool,
    #[serde(default, rename = "override", skip_serializing_if = "Option::is_none")] pub overrides: Option<Vec<String>> }
impl WorldJson { pub fn blank(tileset: u16, fill: TileRef, height: i8) -> WorldJson; }  // Bodenebene „Boden“
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {   // synthetisch, Tileset::synthetic()
    pub fn one_tile_world() -> WorldJson;      // 32×32 `0000`, Höhe 0
    pub fn checkerboard_world() -> WorldJson;  // `0000`/`0001` im Schachbrett ((x + z) % 2)
    pub fn busy_world() -> WorldJson;          // Schachbrett, Höhe (x / 4 + z / 4) % 4, jede 7. Zelle (x + 32 z) `n1`, Überlagerung „Deko“ mit 64 Zellen `0002`
}
// world/grid.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct TileRef { pub tile: u16, pub rotate: u8, pub flip: bool }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum Ramp { #[default] None, North(u8), East(u8), South(u8), West(u8) }
#[derive(Debug, Clone, PartialEq, Eq)] pub struct WorldError { pub path: String, pub message: String }  // path z. B. "world.layers[0].tiles[4]"
pub struct GridLayer { pub name: String, pub role: LayerRole, pub tiles: Vec<Option<TileRef>> /* 1024, y·32+x */ }
pub struct WorldGrid { pub tileset: u16, pub layers: Vec<GridLayer>, pub heights: Vec<i8>, pub ramps: Vec<Ramp>,
    pub pieces: Vec<PieceJson>, pub derive: bool, pub overrides: Vec<Option<g4_formats::map::Cell>> }
impl WorldGrid { pub fn parse(world: &WorldJson) -> Result<WorldGrid, Vec<WorldError>>; }
pub fn parse_tile(token: &str) -> Result<Option<TileRef>, String>;   pub fn tile_token(t: Option<TileRef>) -> String;
pub fn parse_ramp(token: &str) -> Result<Ramp, String>;              pub fn ramp_token(r: Ramp) -> String;
pub fn parse_height(token: &str) -> Result<i8, String>;
pub fn parse_override(token: &str) -> Result<Option<Cell>, String>;  pub fn override_token(c: Option<Cell>) -> String;
/// Replaces cell `x` of row `row` (32 tokens, one space), keeping the other tokens as they are.
pub fn set_token(row: &str, x: usize, token: &str) -> Result<String, String>;
```

`WorldGrid::parse` sammelt **alle** Fehler (Pfade und Texte): Version ≠ 1 („Version 2 kennt diese Fassung nicht (1).“), keine Ebene oder Ebene 0 nicht `ground` oder eine zweite `ground` („die erste Ebene ist die einzige Bodenebene (`ground`).“), Zeilenzahl ≠ 32 und Zellen ≠ 32 (Wortlaut wie `read_permissions`, `map.rs:157-190`), Token-Fehler mit dem Token in Backticks, Höhe außerhalb („`70` liegt nicht in −16 bis 62.“), Rampe mit 0 oder 3 Stufen, `heights`/`ramps` an einer Überlagerung („nur die Bodenebene trägt Höhen und Rampen.“), Bodenebene ohne `heights`, `derive: false` ohne vollständiges `override`, Teilmodell mit `rotate` ∉ {0, 90, 180, 270}, `freeform` mit beiden oder keinem von `glb`/`nsbmd`. Kachelindex gegen das Tileset prüft erst Task 7 (Grid kennt kein Tileset); Fußabdruck gegen den Rand prüft Task 3 (`mesh` braucht die Größe).

- [ ] **Step 1: Scheiternde Tests:** `grid.rs`: `tile_tokens_parse_and_write_back` (`0012` → `{0x12, 0, false}`, `0012r1`, `0012r3f`, `00ABf` → geschrieben `00abf`, `.` → `None`; abgelehnt `12`, `0012r4`, `0012fr1`, `g012`), `ramp_tokens_are_direction_and_steps` (`n1` → `North(1)`, `w2` → `West(2)`, `.`; abgelehnt `n3`, `x1`, `n0`), `height_range_is_minus_16_to_62`, `set_token_keeps_the_other_cells`, `parse_reports_every_error_with_its_path` (alle Fälle oben in einem Dokument, `errors.len()` gleich ihrer Zahl, je Pfad), `a_piece_must_fit_on_the_map` (nur `rotate` hier; `at` [31, 31] mit Fußabdruck in Task 3); `doc.rs`: `a_blank_world_has_one_ground_layer` (Schlüsselreihenfolge `version, tileset, layers, permissions`, `override` fehlt, 32 Zeilen `0000 0000 …`), `unknown_keys_are_refused` (je Ebene ein unbekannter Schlüssel → serde `unknown field`), `prop_world_json_round_trips_byte_equal` (proptest: zufällige Token je Zelle, Höhe, Rampe → `WorldJson` → `to_canonical` → `serde_json::from_slice` → `to_canonical` gleich; `WorldGrid::parse` ohne Fehler).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::` → FAIL (Modul fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `to_canonical` aus `textproject::canon` (für `world` `pub(crate)` genügt).
- [ ] **Step 4: Lauf** → PASS; clippy, fmt.
- [ ] **Step 5: Commit** `feat(world): the world document, its row grammar and its schema check` (mit Trailer).

### Task 2: Tileset-Format und synthetisches Tileset

**Files:**
- Create: `crates/g4-kernel/src/world/tileset.rs` (Typen, `synthetic`, Datei-Prüfung; die Ableitung folgt in Task 5)
- Modify: `crates/g4-kernel/src/world.rs` (`pub mod tileset;`)

**Interfaces:**
- Consumes: Task 1 (`TileRef`).
- Produces:

```rust
pub const TILESET_VERSION: u32 = 1;
pub const TILESETS_DIR: &str = "data/tilesets";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct Tileset { pub version: u32, pub package: u16, pub tiles: Vec<TileJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub edge_sets: Vec<serde_json::Value> }
#[derive(…)] #[serde(deny_unknown_fields)]
pub struct TileJson { pub texture: String, pub texture_index: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub palette: Option<String>,
    pub size: [u16; 2], pub role: TileRole, pub kind: u8, pub collision: u8, pub height: HeightBehavior,
    #[serde(default, skip_serializing_if = "is_zero")] pub sound: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub wall: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub footprint: Option<[u8; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub model: Option<String>,
    pub origin: TileOrigin }
#[serde(rename_all = "snake_case")] pub enum TileRole { Ground, Wall, Ramp, Water, Grass, Piece }
#[serde(rename_all = "snake_case")] pub enum HeightBehavior { Flat, Ramp, Wall }
#[serde(rename_all = "snake_case")] pub enum TileOrigin { Vanilla, Unclear, Synthetic, User }
impl Tileset {
    #[cfg(any(test, feature = "test-support"))] pub fn synthetic() -> Tileset;  // „Entscheidungen“, Paket SYNTHETIC_PACKAGE
    pub fn tile(&self, index: u16) -> Option<&TileJson>;
}
#[cfg(any(test, feature = "test-support"))] pub const SYNTHETIC_PACKAGE: u16 = 0;
pub fn tileset_rel(package: u16) -> PathBuf;                                   // data/tilesets/0006.json
pub fn read_tileset_file(project: &Project, package: u16) -> Result<Option<Tileset>, KernelError>;
/// Problems of one tileset file beyond serde: `package` ≠ stem, `edge_sets` not empty („Kantensätze kommen mit 6.4b.“),
/// `model` set („Teilmodell-Dateien kommen mit 6.5b.“), `wall` past the tiles or not role `wall`, `footprint` on a tile
/// that is not `piece` or a `piece` without it, a texture name longer than 16 bytes, `sound` ≠ 0 outside HGSS.
pub fn tileset_problems(tileset: &Tileset, stem: u16, family: Family) -> Vec<String>;
/// „<path>: Kachel 0x0120 gibt es im Tileset 0006 nicht (94 Kacheln).“ (check and world_edit share it).
pub fn tile_outside(path: &str, tile: u16, package: u16, tiles: usize) -> String;
```

- [ ] **Step 1: Scheiternde Tests:** `synthetic_tileset_has_six_roles` (Namen, Größen, Rollen, `wall: Some(3)` an Kachel 0, `footprint [2, 2]` an 5), `tileset_json_round_trips_and_refuses_unknown_keys`, `tileset_problems_name_each_rule` (je Regel ein Fall, Text wörtlich), `tileset_rel_is_four_digits`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::tileset` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(world): the tileset format and the synthetic test tileset` (mit Trailer).

### Task 3: Vernetzer und Budget-Schätzung

**Files:**
- Create: `crates/g4-kernel/src/world/mesh.rs` (+ Tests), `crates/g4-kernel/tests/world.rs` (Zeittest, ohne ROM)
- Modify: `crates/g4-kernel/src/world.rs` (`pub mod mesh;`)

**Interfaces:**
- Consumes: Task 1 (`WorldGrid`, `Ramp`, `TileRef`, Konstanten), Task 2 (`Tileset`, `TileJson.wall`, `footprint`).
- Produces (die Struktur gilt wörtlich, abgestimmt mit 6.1 `bake::Face`):

```rust
/// One quad: corners in world units (16 per tile, height = step × 8, y up), counter-clockwise seen from its front
/// (floors from above); UV in texels, world-bound, 16 per tile; `texture` a name of the package (≤ 16 bytes); `layer` the layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshFace { pub corners: [[i32; 3]; 4], pub uv: [[i32; 2]; 4], pub texture: String, pub layer: u8 }
pub const PIECE_LAYER: u8 = 255;
pub const MODEL_BUDGET: usize = 0xF000;
/// The faces of `grid` with the textures of `tileset`; tiles past the tileset are skipped (the check reports them).
pub fn mesh(grid: &WorldGrid, tileset: &Tileset) -> Vec<MeshFace>;
/// The distinct texture names of `faces`, in order of first use.
pub fn mesh_textures(faces: &[MeshFace]) -> Vec<String>;
pub fn estimate_model_bytes(faces: usize, textures: usize) -> usize;  // 1024 + 600·textures + 56·faces
/// Pieces whose footprint leaves the map: `world.pieces[i]` with „Das Teilmodell ragt über den Kartenrand (x 31, Breite 2).“
pub fn piece_problems(grid: &WorldGrid, tileset: &Tileset) -> Vec<WorldError>;
```

Ein Boden einer Zelle (cx, cz) auf Höhe `h` ohne Rampe hat die Ecken `[x0, y, z0], [x0, y, z1], [x1, y, z1], [x1, y, z0]` (`x0 = 16·cx − 256`, `x1 = x0 + 16·Breite`, `z0`, `z1` ebenso, `y = 8·h`); eine Rampe hebt die genannte Kante um `8·Stufen`. Ein Fußabdruck mit `rotate` 90/270 tauscht Breite und Tiefe.

- [ ] **Step 1: Scheiternde Tests** (alle mit `Tileset::synthetic()` und `fixtures`): `one_tile_map_meshes_to_one_face` (Gate ≤ 4; genau 1: Ecken `[[-256,0,-256],[-256,0,256],[256,0,256],[256,0,-256]]`, UV `[[-256,-256],[-256,256],[256,256],[256,-256]]`, `syn_boden`, Ebene 0), `checkerboard_meshes_to_1024_faces` (Gate ≤ 1 024; genau 1 024), `floor_corners_are_counter_clockwise_from_above` (Kreuzprodukt y > 0 für jede Bodenfläche), `a_raised_block_gets_four_merged_walls` (2×2 Block Höhe 2 auf 0 aus Kachel 0: genau 4 Wandflächen, je 32 Einheiten lang und 16 hoch, Textur `syn_wand`, Normale zur niedrigeren Seite), `walls_use_the_upper_tile_without_a_wall_tile` (Block aus Kachel 1 → `syn_gras`), `walls_face_the_lower_cell`, `ramp_north_one_step_rises_to_the_north` (Zelle (5, 5) `h = 1` `n1`: Südkante y 8, Nordkante y 16), `ramps_merge_only_across_the_slope` (vier Zellen x 4–7, z 5, `n1`, h 0 → 1 Fläche; zwei Zellen z 4–5, x 4, `n1` mit h 1 und 0 → 2 Flächen), `rotation_and_flip_turn_the_uv` (`0000r1` an (0, 0): je Ecke `(u, v) = (z, −x)`; `0000r2f`: `(x, −z)`), `overlay_layers_follow_the_ground_height` (Überlagerung auf Höhe 2 → y 16, Ebene 1, keine Wand), `pieces_leave_a_footprint_face` (Stück Kachel 5 bei [3, 4] → eine Fläche 32×32 Einheiten, Ebene 255, `syn_stueck`), `empty_cells_have_no_face_and_no_wall`, `edges_of_the_grid_mesh_without_panic` (Rampe an jeder Kartengrenze, Höhe −16 neben 62, gegenläufige Rampen: keine Wand, keine Panik; `piece_problems` für ein Stück bei [31, 31] mit Fußabdruck 2×2), `mesh_is_deterministic` (zweimal gleich, `busy_world`), `mesh_textures_in_first_use_order`, `estimate_matches_the_flat_field_of_gutachten_a` (1 024 Flächen, 1 Textur = 58 968; ≥ 57 776 und ≤ 57 776 × 1,05).
- [ ] **Step 2: Zeittest** `tests/world.rs` (Kopf `#![cfg(feature = "test-support")]` und `mod common;` wie `tests/flags.rs`) `mesh_of_a_full_map_is_fast`: `busy_world` parsen, 20 Läufe `mesh`, Median; druckt `world-mesh faces=<n> median_us=<…>`; mit `G4_TIMING_STRICT=1` `assert!(median ≤ 20 ms)`.
- [ ] **Step 3: Lauf** `cargo test -p g4-kernel --all-features world::mesh` und `--test world` → FAIL; **Step 4: Umsetzen** (Zusammenfassen und Wände wie in „Entscheidungen“; ein `Vec<bool>` „besucht“ je Ebene, keine Hash-Maps).
- [ ] **Step 5: Lauf** → PASS; dann im Hintergrund `G4_TIMING_STRICT=1 cargo test --release -p g4-kernel --all-features --test world -- mesh_of_a_full_map_is_fast --nocapture > target/agent/6-3a-mesh-timing.log 2>&1` → PASS, Median notieren.
- [ ] **Step 6: Commit** `feat(world): mesher from layers, heights, ramps and footprints to a face list` (mit Trailer).

### Task 4: Textprojekt: `source: "world"`, Weltobjekt mitnehmen, Art-Beschreibung, Migration auf Version 3

**Files:**
- Create: `crates/g4-kernel/src/world/keep.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/textproject/schema/map.rs` (`MapJson.world`, `map_bytes` nimmt `"world"` an, `sample_map(true)` mit `WorldJson::blank`, Tests), `textproject/export.rs` (`write_folder`: Karten-JSON über `carry_world`), `textproject/source/writers.rs` (Karten-Writer nimmt `source`/`world` nach „Entscheidungen“), `textproject/kinds.rs` (anhängen: `FileKind`, `FILE_KINDS`, `PART_SCHEMAS`; `schemas()` und `check_files` lesen sie), `textproject/arts.rs` (`is_known_folder` kennt `FILE_KINDS`), `textproject/migrate.rs` (`v2_to_v3`, dritter Eintrag in `MIGRATIONS`), `textproject/mod.rs` (`SCHEMA_VERSION = 3`), `textproject/schema.snap` (neu mit `G4_UPDATE_SNAPSHOT=1`), `textproject/manifest.rs` (Tests), `crates/g4-kernel/src/world/tileset.rs` (`check_folder`), `crates/g4-kernel/src/world.rs` (`pub mod keep;`)

**Interfaces:**
- Consumes: Task 1 (`WorldJson`, `MAP_SOURCE_WORLD`, `WorldJson::blank`), Task 2 (`tileset_problems`, `TILESETS_DIR`).
- Produces:

```rust
// world/keep.rs
/// `source` and `world` of the map JSON `bytes` (`None` for a binary map); a file that does not parse is `Err`.
pub fn world_field(bytes: &[u8]) -> Result<Option<WorldJson>, KernelError>;
/// `fresh` (a map JSON written from the member) with `source`/`world` set to `world` (`None`: left as `binary`).
pub fn with_world(fresh: Vec<u8>, world: Option<&WorldJson>) -> Result<Vec<u8>, KernelError>;
/// [`with_world`] with the world of the file on disk (`on_disk`, `None` without file).
pub fn carry_world(on_disk: Option<&[u8]>, fresh: Vec<u8>) -> Result<Vec<u8>, KernelError>;
// textproject/kinds.rs (angehängt)
pub struct FileKind { pub folder: &'static str, pub schema_version: u32,
    pub check: fn(&Project, &mut Vec<Problem>) -> Result<(), KernelError> }
pub const FILE_KINDS: [FileKind; 1];                        // tilesets, 1, world::tileset::check_folder
pub const PART_SCHEMAS: [(&str, u32); 1] = [("maps.world", 1)];
// world/tileset.rs (angehängt)
pub fn check_folder(project: &Project, problems: &mut Vec<Problem>) -> Result<(), KernelError>;  // textproject.schema mit Datei und Zeile
```

- [ ] **Step 1: Scheiternde Tests:** `map.rs`: `a_world_map_imports_its_binary_parts` (Karte aus `map()` mit `source: "world"` und `WorldJson::blank(0, …)` → `import` gleich den Member-Bytes ohne Welt), `source_and_world_go_together` (zwei Texte aus „Entscheidungen“, Code `textproject.map-source`), `a_typo_in_world_is_serdes` (`world.layers[0].tile` → `unknown field`); `keep.rs`: `carry_world_takes_source_and_world_from_disk`, `carry_world_without_world_on_disk_is_the_fresh_file`, `with_world_none_writes_binary`; `export.rs` (Tests-Modul, `TextFixture::exported()`): `export_keeps_a_world_map` (Welt in `data/maps/0000.json` über `with_world`, zweiter Export: `written 0`, Datei byte-gleich); `source/tests.rs`: `writing_a_map_keeps_its_world` (`EntitySource::write` der Karte 0 mit geänderter Permission: `world` unverändert); `kinds.rs`: `schemas_name_every_kind_and_keep_foreign_keys` erwartet `flags`, `maps.world`, `stories`, `tilesets`, `vars` je 1, `tilesets_is_a_known_folder`, `check_files_reads_tilesets` (eine Datei mit `package` 7 unter `0006.json` → `textproject.schema`); `migrate.rs`: `v2_to_v3_changes_only_the_manifest` (Baum mit Manifest Version 2 und Dateien unter `data/` → nach der Kette nur `g4-project.json` geändert, `schemas` mit `"maps.world": 1`, `"tilesets": 1`), `the_chain_reaches_version_three`; bestehend grün nach `G4_UPDATE_SNAPSHOT=1 cargo test -p g4-kernel --all-features schema_fields_match_the_snapshot`: der Snapshot nennt `world.*` als optional.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features textproject:: world::keep` → FAIL; **Step 3: Umsetzen** wie unter Interfaces; `write_folder` und der Writer rufen `carry_world` nur für `Art::Maps`.
- [ ] **Step 4: Lauf** → PASS; der ganze Kernel: `cargo test -p g4-kernel --all-features > target/agent/6-3a-t4.log 2>&1` (Hintergrund) grün.
- [ ] **Step 5: Commit** `feat(textproject): maps with source world keep their world object, tilesets kind, schema version 3` (mit Trailer).

### Task 5: Tileset aus dem Paket ableiten, Texturen dekodieren, Tileset speichern

**Files:**
- Create: `crates/g4-kernel/src/world/places.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/world/tileset.rs` (`textures_only`, `VoteRule`, `UNIQUE_SHARE`, `Deriver`, `derive_tileset`, `tileset_texture`, `save_tileset`), `crates/g4-kernel/src/store.rs` (`Store::tileset` mit Zwischenspeicher), `crates/g4-kernel/src/world.rs` (`pub mod places;`), `crates/g4-kernel/tests/world.rs` (gated)

**Interfaces:**
- Consumes: Task 0 (Regel, Schwelle, Palettenregel), Task 2 (`Tileset`, `tileset_rel`, `read_tileset_file`).
- Produces:

```rust
// world/places.rs
/// Map texture packages of the area data of every header whose matrix shows `map`, ascending (rule of map_import::map_packs).
pub fn packs_of_map(store: &mut Store, map: u16) -> Result<Vec<u16>, KernelError>;
/// Every package with the maps that show it (one pass over headers and matrices).
pub fn maps_of_packages(store: &mut Store) -> Result<BTreeMap<u16, Vec<u16>>, KernelError>;
// world/tileset.rs
/// One tile per texture of `btx` (names, indices, sizes, palettes), every tile `ground`, `kind 0`, `origin unclear`: what
/// meshing and range checks need, without reading a map.
pub fn textures_only(package: u16, btx: &Nsbtx) -> Tileset;
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum VoteRule { K1, K2, None }
pub const DEFAULT_RULE: VoteRule;      // aus Task 0
pub const UNIQUE_SHARE: f64;           // aus Task 0 (0.6, 0.7, 0.8 oder 0.9)
pub struct Deriver { maps: BTreeMap<u16, Vec<u16>> }
impl Deriver {
    pub fn new(store: &mut Store) -> Result<Deriver, KernelError>;
    pub fn maps_of(&self, package: u16) -> &[u16];
    pub fn derive(&self, store: &mut Store, package: u16) -> Result<Tileset, KernelError>;
}
pub fn derive_tileset(store: &mut Store, package: u16) -> Result<Tileset, KernelError>;   // Deriver::new(store)?.derive(…)
/// RGBA of `texture` with its tile's palette; `None` when the package or the texture is missing (synthetic tileset).
pub fn tileset_texture(project: &Project, tileset: &Tileset, texture: &str) -> Result<Option<Rgba>, KernelError>;
#[derive(Debug, Clone, Serialize)]
pub struct TilesetSaveReport { pub package: u16, #[serde(serialize_with = "crate::relpath::ser")] pub file: PathBuf,
    pub tiles: usize, pub vanilla: usize, pub unclear: usize, pub change: ChangeId }
/// Writes the derived tileset of `package` as one file step (FileStepKind::Reset, asset HEADER_TABLE, label
/// „Tileset 0006 abgeleitet“); refused in the DSPRE layout and when the file exists („data/tilesets/0006.json gibt es schon.“).
pub fn save_tileset(store: &mut Store, package: u16) -> Result<TilesetSaveReport, KernelError>;
// store.rs
/// The tileset file of `package` (read again when its size or time changes), else `textures_only` of the package (cached);
/// a missing package is `Refused` („Kartentextur-Paket 0200 gibt es nicht (74 Pakete).“).
impl Store { pub fn tileset(&mut self, package: u16) -> Result<Arc<Tileset>, KernelError>; }
```

`Store::tileset` braucht keine Karte (Vernetzen, Bereichsprüfung, Brücke bleiben schnell); die Ableitung mit Vorgaben (`derive_tileset`) läuft nur in `tileset_get` ohne Datei und in `tileset_save`. `derive` liest das Paket (`map_tileset_member`, `Nsbtx::read`; ein Paket, das nicht liest, ist `Refused` mit dem Grund), je Textur `i` eine Kachel `texture_index = i`, `size = [width, height]`, Palette nach Task 0, Vorgaben nach `DEFAULT_RULE` aus den Karten `maps_of(package)` (Modell über `material_texture_names`, ein Modell, das nicht liest, zählt nicht; Permissions aus `store.map`).

- [ ] **Step 1: Scheiternde Tests:** Unit mit einer synthetischen Fixture (`TextFixture::exported()`, Paket 0 durch `g4_formats::test_support::btx0` mit drei Texturen 16×16, 32×32, 16×16 und Paletten `t0_pl`, `t1`, keine ersetzt, Karte 0 mit Modell aus `g4_formats::test_support::{bmd0, mdl0_block}`, das `t0` bindet, Permissions 1 000 Zellen `kind 0x02` und 24 Zellen 0): `derive_gives_one_ground_tile_per_texture` (3 Kacheln, Indizes, Größen, Paletten `t0_pl`, `t1`, keine), `derive_takes_the_vote_of_the_maps` (`t0` → `grass` `kind 0x02` `origin vanilla`; `t1`, `t2` ohne Karte → `unclear`), `textures_only_matches_derive_in_names_and_count`, `store_tileset_prefers_the_file_and_rereads_it` (Datei schreiben → Datei; Datei ändern → neu gelesen; Datei löschen → `textures_only`), `packs_of_map_follows_headers_and_matrices`, `maps_of_packages_inverts_it`, `tileset_texture_decodes_with_the_tile_palette`, `save_tileset_is_one_undo_step` (Datei da, `undo` → weg) und `save_tileset_refuses_existing_and_dspre`.
- [ ] **Step 2: Gated** `tests/world.rs` `vanilla_tilesets_have_one_ground_tile_per_texture` (`vanilla_copy()`; `Deriver::new` einmal; je Paket `0..member_count(MapTexture)`: `tiles.len() == textures.len()`, Namen und Indizes gleich, Rollen nur `ground`/`grass`/`water`; druckt `tilesets <code> packages=<n> tiles=<n> vanilla=<n> unclear=<n> ms=<n>`; CPUD verlangt 74 / 3 130, IPKD 106 / 3 659, IPKE gleich seiner Texturzahl), `vanilla_vote_shares_match_task0` (druckt je Spiel die Verteilung der Anteile wie Task 0, keine Grenze).
- [ ] **Step 3: Lauf** Unit → FAIL; **Step 4: Umsetzen**; **Step 5: Lauf** Unit → PASS; gated im Hintergrund je Projekt: `G4_TEST_PROJECT="/Users/POLTEP/Projekte/lab/editor/Roms/Pokemon - Platin-Edition (Germany)_DSPRE_contents" cargo test --release -p g4-kernel --all-features --test world -- vanilla_ --nocapture --test-threads 1 > target/agent/6-3a-tilesets-pt.log 2>&1`, ebenso HeartGold DE und US → PASS, Zahlen ins Protokoll von Task 0 nachtragen (Abschnitt „Gegenprobe im Kernel“).
- [ ] **Step 6: Commit** `feat(world): derive the tileset of a map texture package from its textures and the vanilla permissions` (mit Trailer).

### Task 6: Store-Art `WorldMap`: lesen, anlegen, ändern als ein Undo-Schritt

**Files:**
- Create: `crates/g4-kernel/src/world/edit.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/project.rs` (`AssetKind::WorldMap`, `ALL: [AssetKind; 30]`, `slot` 29, `label` „world map“, `Location::Derived { json: PathBuf }`, `ArchiveMap` `None` am Ende, `unpacked_name` `"maps+world"`, `member_location`, `ensure_unpacked`/`member_count` wie Karten, Test `:1328`), `crates/g4-kernel/src/store.rs` (`Loaded::World`, `is_editable`, `from_bytes`/`to_bytes`, jedes `match` auf `Location` um `Derived`, `Store::world`, `Tx::edit_world`), `textproject/source/{units.rs, mod.rs, writers.rs}` (`place_of(WorldMap)`, `unit_of` mit zwei Assets, `read` des Weltfelds, Writer mit WorldMap-Bytes), jede Stelle, die `AssetKind::Trainer` als Store-Art ohne Archiv einordnet (`git grep -n "AssetKind::Trainer =>" crates`: `base_layer.rs`, `original.rs`, `bin/g4k.rs`, `textproject/step.rs` prüfen), `crates/g4-kernel/src/world.rs` (`pub mod edit;`)

**Interfaces:**
- Consumes: Task 1 (`WorldJson`, `set_token`, Parser), Task 2 (`Tileset`, `tile_outside`), Task 4 (`with_world`), Task 5 (`Store::tileset`, `packs_of_map`).
- Produces:

```rust
// store.rs
impl Store { pub fn world(&mut self, map: u16) -> Result<Option<&WorldJson>, KernelError>; }
impl Tx<'_> { pub fn edit_world(&mut self, map: u16,
    edit: impl FnOnce(&mut Option<WorldJson>) -> Result<(), KernelError>) -> Result<(), KernelError>; }
// world/edit.rs
pub const WORLD_NEEDS_TEXT_PROJECT: &str = "Boden malen gibt es im Textprojekt (Technik › Textprojekt).";
pub const ONE_FIELD: &str = "Eine Änderung setzt genau eines von tile, height, ramp, permission.";
pub const NOTHING_TO_CHANGE: &str = "Nichts zu ändern.";
pub fn not_a_world_map(map: u16) -> String;   // „Karte 0123 ist keine Welt-Karte; erst „Welt anlegen“ (world_create).“
pub fn already_world(map: u16) -> String;     // „Karte 0123 ist schon eine Welt-Karte.“
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldCreateRequest { pub map: u16, #[serde(default)] pub tileset: Option<u16>,
    #[serde(default)] pub fill: Option<String>, #[serde(default)] pub height: Option<i8> }
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldEditRequest { pub map: u16, #[serde(default)] pub label: Option<String>, pub changes: Vec<WorldChange> }
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldChange { #[serde(default)] pub layer: Option<u8>, pub cells: Vec<[u8; 2]>,
    #[serde(default)] pub tile: Option<String>, #[serde(default)] pub height: Option<i8>,
    #[serde(default)] pub ramp: Option<String>, #[serde(default)] pub permission: Option<String> }
#[derive(Debug, Clone, Serialize)] pub struct WorldReport { pub map: u16, pub tileset: u16, pub cells: usize, pub change: Option<ChangeId> }
pub fn world_create(store: &mut Store, req: &WorldCreateRequest) -> Result<WorldReport, KernelError>;
pub fn world_edit(store: &mut Store, req: &WorldEditRequest) -> Result<WorldReport, KernelError>;
```

Ablehnungen (`KernelError::Refused`, nichts geändert): DSPRE-Layout oder ältere/neuere Version (`WORLD_NEEDS_TEXT_PROJECT`), keine Welt (`not_a_world_map`), schon Welt (`already_world`), mehr oder weniger als ein Feld (`ONE_FIELD`), Zelle außerhalb („Zelle (32, 4) liegt außerhalb der Karte (0 bis 31).“), Ebene fehlt („Ebene 2 gibt es nicht (Karte 0123 hat 1).“), Höhe/Rampe auf einer Überlagerung („Höhen und Rampen gibt es nur auf der Bodenebene (0).“), `permission` mit `layer` („permission gilt für die Karte, nicht für eine Ebene; layer weglassen.“), ein Token-Fehler (Text des Parsers), eine Kachel außerhalb des Tilesets (`tileset::tile_outside`), Paket außerhalb (Ablehnung von `Store::tileset`), keine Änderung (`NOTHING_TO_CHANGE`), ein Weltdokument, das schon vor der Änderung `WorldGrid::parse` nicht besteht (erster Fehler mit Pfad). Nach dem Anwenden muss `WorldGrid::parse` bestehen, sonst Ablehnung.

- [ ] **Step 1: Scheiternde Tests** (`TextFixture::exported()`, Tileset-Datei `data/tilesets/0000.json` aus `Tileset::synthetic()` geschrieben): `project.rs` `world_map_is_a_store_kind_without_archive`; `edit.rs`: `world_create_and_edit_are_one_undo_step_each` (anlegen, ein `world_edit` mit 37 Zellen `0001` und 4 Höhen → `history()` zwei Einträge „Welt angelegt: Karte 0000“, „Welt geändert: Karte 0000 (41 Zellen)“; `undo` → Welt wie nach dem Anlegen; `undo` → `world(0) == None`; `redo` zweimal), `undo_of_create_returns_to_binary` (gespeichert: `source: "binary"`, kein `world`), `save_writes_world_and_keeps_member` (SHA-1 des Members `unpacked/…/0000` vor und nach gleich, JSON mit `world`), `a_map_edit_keeps_the_world` (`edit_map` Permission nach `world_create`, speichern: beides in der Datei), `world_and_map_steps_undo_one_by_one` (Review Focus), `world_edit_writes_only_touched_rows` (eine Zelle in Zeile 7 → nur `tiles[7]` anders, Byte für Byte), `world_edit_refuses_each_rule` (je Ablehnung ein Fall, Text wörtlich, Welt unverändert), `world_edit_refuses_a_broken_world_unchanged`, `dspre_refuses_with_the_way` (`TextFixture::dspre()`), `create_takes_the_package_of_the_header` (`packs_of_map` der Fixture); `source/tests.rs`: `the_map_unit_holds_map_and_world` (`unit_of(Map 0).assets == [Map 0, WorldMap 0]`, gleicher `key`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world:: store:: textproject::source` → FAIL; **Step 3: Umsetzen**; `Location::Derived` beim Lesen außerhalb einer Einheit → `Refused(WORLD_NEEDS_TEXT_PROJECT)`, beim Member-Schreiben nichts, `member_targets` leer.
- [ ] **Step 4: Lauf** → PASS; ganzer Workspace `cargo test --workspace --all-features > target/agent/6-3a-t6.log 2>&1` (Hintergrund; `AssetKind::ALL` zählen Tests in Studio und Kernel, etwa `tests/project.rs`) grün.
- [ ] **Step 5: Commit** `feat(store): the world map as a store kind, created and edited in one undo step` (mit Trailer).

### Task 7: Prüfungen im Textprojekt-Check

**Files:**
- Create: `crates/g4-kernel/src/world/check.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/textproject/check.rs` (`hints` ruft `world::check::world_problems`), `crates/g4-kernel/src/textproject/import.rs` (eine Karte mit `world.schema`-Fehler wird nicht importiert, ihr Problem steht im Plan), `crates/g4-kernel/src/world.rs` (`pub mod check;`)

**Interfaces:**
- Consumes: Task 1 (`WorldGrid::parse`, `WorldError`), Task 2 (`tile_outside`, `read_tileset_file`), Task 3 (`mesh`, `mesh_textures`, `estimate_model_bytes`, `piece_problems`), Task 4 (`world_field`), Task 5 (`packs_of_map`, `textures_only`).
- Produces:

```rust
pub const WORLD_SCHEMA_CODE: &str = "world.schema";
pub const WORLD_TILESET_MISMATCH_CODE: &str = "world.tileset-mismatch";
pub const WORLD_NOT_BAKED_CODE: &str = "world.not-baked";
pub const WORLD_BUDGET_CODE: &str = "world.budget";
pub fn budget_text(faces: usize, textures: usize) -> String;   // Wortlaut aus „Entscheidungen“
/// Problems of every world map of the project (the `data/maps/*.json` with `source: "world"`, read with `world_field`):
/// schema (grid, tiles against the tileset file or `textures_only` of the package, package range, pieces), tileset
/// mismatch, not baked, budget. The headers for the mismatch come from a Store on `Project::open(&project.root)`, read only.
pub fn world_problems(project: &Project) -> Result<Vec<Problem>, KernelError>;
```

`world.tileset-mismatch` (Warnung): „Karte 0123 nimmt Tileset 0006, ihr Header 0411 nennt Gebietsdaten mit Paket 0007.“ (je abweichendem Header einmal). Zeile und Spalte über `locate::position` mit dem Pfad `world.…`.

- [ ] **Step 1: Scheiternde Tests** (Fixture wie Task 6, Tileset-Datei aus `Tileset::synthetic()`): `schema_problems_name_path_and_line` (Datei mit `00g2` in Zeile 12 der Datei: `world.schema`, Zeile und Spalte des Tokens), `tiles_past_the_tileset_are_schema_errors`, `a_header_with_another_package_warns` (Fixture: Gebietsdaten 1 auf Paket 1 umgestellt), `every_world_map_is_not_baked`, `budget_is_an_info_with_faces_and_textures` (`one_tile_world` → „Geschätzt höchstens 1 680 Byte (3 % von 0xF000) aus 1 Fläche und 1 Textur.“), `the_import_skips_a_broken_world_map_with_its_problem`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(world): checks world.schema, tileset-mismatch, not-baked and budget in the text project check` (mit Trailer).

### Task 8: Gated: Round-Trip, Export idempotent und Migration mit einer Welt-Karte

**Files:**
- Modify: `crates/g4-kernel/tests/textproject.rs` (zwei Tests, ein Helfer), `crates/g4-kernel/tests/textproject_migrate.rs` (ein Test)

**Interfaces:**
- Consumes: Task 4, 5 (`packs_of_map`), 6 (`world_create`, `world_edit`); `exported_copy`, `round_trip`, `export_idempotent` (`tests/textproject.rs:2498, 2619`).
- Produces: `fn world_on_first_map(copy: &Path) -> (u16, Vec<u8>)` (Store auf der Kopie, erste Karte, für die `packs_of_map` genau ein Paket liefert, `world_create`, `world_edit` mit einem Schachbrett aus Kachel 0 und 1 über die Zeilen 0–3 (ein Paket mit nur einer Textur: nur Kachel 0) und Höhe 2 im Block x 8–11, z 8–11, `save`; gibt Karte und Bytes der JSON zurück).

- [ ] **Step 1: Tests:** `textproject_round_trip_with_world` (wie `round_trip`, nach dem Export `world_on_first_map`: Member gelöscht, Import, ROM byte-gleich zur Basis, die Karten-JSON danach byte-gleich zu den gemerkten Bytes), `textproject_export_idempotent_with_world` (zweiter Export `written 0, removed 0`, Datei byte-gleich), `vanilla_migration_v2_to_v3_changes_only_the_manifest` (exportierte Kopie, Manifest auf Version 2 ohne `maps.world`/`tilesets` gesetzt, `migrate` → `changed == ["g4-project.json"]`, SHA-1 aller anderen Quellen gleich).
- [ ] **Step 2: Lauf** im Hintergrund je Projekt (Platin DE, HeartGold DE, HeartGold US): `G4_TEST_PROJECT=<Original> G4_APICULA=… cargo test --release -p g4-kernel --all-features --test textproject --test textproject_migrate -- with_world vanilla_migration --nocapture --test-threads 1 > target/agent/6-3a-t8-<spiel>.log 2>&1` → PASS; Karte und Zeiten notieren.
- [ ] **Step 3: Commit** `test(world): round trip, idempotent export and migration with a world map on the vanilla copies` (mit Trailer).

### Task 9: Befehle: Tauri, Brücke, Zugriff

**Files:**
- Create: `apps/studio/src-tauri/src/commands/world_map.rs` (+ Tests unten in der Datei)
- Modify: `commands/mod.rs` (`mod world_map; pub use world_map::*;` angehängt), `commands/dispatch.rs` (sieben Namen in `COMMANDS` und sieben Arme angehängt), `src-tauri/src/lib.rs` (`generate_handler!` angehängt), `mcp/access.rs` (sieben Einträge nach „Entscheidungen“), `commands/world.rs` (`MapDto.world: bool`, `get_map_impl` setzt es aus `store.world`), `docs/mcp.md` (Abschnitt „Welt-Karten (6.3a, über `g4_call`)“ mit den sieben Befehlen, ihrer Art und den Ablehnungen)

**Interfaces:**
- Consumes: Task 3 (`mesh`, `mesh_textures`, `estimate_model_bytes`), Task 5 (`Store::tileset`, `derive_tileset`, `save_tileset`, `tileset_texture`), Task 6 (`world_create`, `world_edit`, `Store::world`), Task 7.
- Produces (Tauri, `*_impl(state, …)` plus `#[tauri::command]`):

```rust
pub fn world_create_impl(state: &AppState, req: WorldCreateRequest) -> Result<WorldReport, CmdError>;
pub fn world_edit_impl(state: &AppState, req: WorldEditRequest) -> Result<WorldReport, CmdError>;
#[derive(Serialize)] pub struct WorldGetDto { pub map: u16, pub world: Option<WorldJson>, pub faces: usize,
    pub textures: usize, pub estimate: usize, pub share: f64 }          // share = estimate / 0xF000
pub fn world_get_impl(state: &AppState, map: u16) -> Result<WorldGetDto, CmdError>;
#[derive(Serialize)] pub struct TilesetDto { pub package: u16, pub stored: bool, pub tileset: Tileset }
pub fn tileset_get_impl(state: &AppState, package: u16) -> Result<TilesetDto, CmdError>;
pub fn tileset_save_impl(state: &AppState, package: u16) -> Result<TilesetSaveReport, CmdError>;
#[derive(Serialize)] pub struct MeshTextureDto { pub name: String, pub width: u16, pub height: u16, pub role: TileRole,
    #[serde(skip_serializing_if = "Option::is_none")] pub palette: Option<String> }
#[derive(Serialize)] pub struct WorldMeshDto { pub map: u16, pub tileset: u16, pub faces: usize,
    pub textures: Vec<MeshTextureDto>, pub corners: Vec<i32>, pub uv: Vec<i32>, pub texture: Vec<u16>, pub layer: Vec<u8> }
pub fn get_world_mesh_impl(state: &AppState, map: u16) -> Result<WorldMeshDto, CmdError>;   // ohne Welt: faces 0, leere Felder
pub fn get_tileset_texture_impl(state: &AppState, package: u16, texture: String) -> Result<Option<SpriteDto>, CmdError>;
```

`world_get` und `get_world_mesh` antworten außerhalb des Textprojekts `world: None` bzw. `faces: 0` statt der Ablehnung; `tileset_get` ohne Datei antwortet `derive_tileset` mit `stored: false`; `get_world_mesh` nimmt `Store::tileset`. Argumente in `dispatch` wie Tauri (camelCase): `world_create { req }`, `world_edit { req }`, `world_get { map }`, `tileset_get { package }`, `tileset_save { package }`, `get_world_mesh { map }`, `get_tileset_texture { package, texture }`.

- [ ] **Step 1: Scheiternde Tests:** `world_commands_over_dispatch` (Fixture `TextFixture::exported()` mit `Tileset::synthetic()` als Datei: `world_create`, `world_edit` mit 37 Zellen, `world_get` → `faces`, `get_world_mesh` → `corners.len() == 12 · faces`, `uv.len() == 8 · faces`, `texture`/`layer` je Fläche, Texturnamen `syn_*`; `undo` → `world_get` wie nach dem Anlegen; `get_map` → `world: true`), `get_tileset_texture_is_null_for_the_synthetic_tileset`, `world_commands_in_the_dspre_layout` (Review Focus: `world_create`/`world_edit`/`tileset_save` → Fehler mit `WORLD_NEEDS_TEXT_PROJECT`, `get_map` `world: false`, `get_world_mesh` `faces 0`), `world_mesh_over_the_command_path_is_fast` (`busy_world` über `world_edit` gesetzt, ein Aufruf zum Aufwärmen, dann 10 Aufrufe `dispatch("get_world_mesh")` mit JSON-Serialisierung, Median; druckt `world-mesh-bridge median_ms=<…> faces=<…>`; mit `G4_TIMING_STRICT=1` ≤ 50 ms im Debug-Build); bestehend: `every_command_has_a_kind`, die Gleichheit von `COMMANDS` und `generate_handler!`, `the_curated_set_is_the_decided_one` unverändert.
- [ ] **Step 2: Lauf** `cargo test -p studio world_` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS; `G4_TIMING_STRICT=1 cargo test -p studio world_mesh_over_the_command_path_is_fast -- --nocapture > target/agent/6-3a-bridge-timing.log 2>&1` → PASS, Median notieren.
- [ ] **Step 5: Commit** `feat(studio): commands for world maps, tilesets and the mesh DTO` (mit Trailer).

### Task 10: main einmergen (nach dem Merge von 6.2), `Ramp` verbinden

**Files:**
- Modify: Konfliktstellen des Merges; `crates/g4-kernel/src/world/grid.rs` (`From<Ramp> for terrain::Ramp` und zurück, wenn `terrain::Ramp` auf main existiert)

**Interfaces:**
- Consumes: main mit 6.2 (`git log main --oneline | grep -i "6.2"`; `git grep -n "pub enum Ramp" main -- crates/g4-kernel/src/terrain`).
- Produces: der Worktree auf main mit 6.2; dieselbe Rampe in beiden Modulen.

- [ ] **Step 1: Prüfen**, dass 6.2 auf main ist (Merge-Commit „6.2“ in `git log main`); sonst wartet dieser Task, Tasks 11 bis 14 hängen daran.
- [ ] **Step 2: Mergen** `git merge main` im Worktree; Konflikte an Anhängepunkten gesammelt lösen (beide Seiten anhängen: `checks/mod.rs` gehört 6.2, `dispatch.rs`, `lib.rs`, `access.rs`, `api.ts`, `docs/mcp.md`, `ROADMAP.md`, `design-system/README.md`, `.fig` von main behalten); nach dem Merge zählt `git log --format='%(trailers:only,unfold)' main..HEAD` die Trailer weiter richtig (Merge-Commit mit Trailer).
- [ ] **Step 3: Test** `ramp_direction_matches_terrain` in `grid.rs` (`East(1)` bei `h = 0` hat an der Westkante 0 und an der Ostkante 8 Einheiten in beiden Modulen; `From` in beide Richtungen verlustfrei für 1 und 2 Stufen). Weicht 6.2s Richtungssinn ab, folgt `world::grid::Ramp` 6.2 (Änderung der Doku `world_map.md`, der Tests in Task 3 und des Token-Sinns), weil die BDHC die Wahrheit des Spiels ist.
- [ ] **Step 4: Lauf** `cargo test --workspace --all-features > target/agent/6-3a-t10.log 2>&1` und `cd apps/studio && pnpm test` (Hintergrund) grün.
- [ ] **Step 5: Commit** (Merge-Commit plus `feat(world): ramps convert to and from the terrain ramp of 6.2`, beide mit Trailer).

### Task 11: Frontend: Netz in der Szene, `[data-world-mesh]`, Undo-Nachladen

**Files:**
- Create: `apps/studio/src/api.world.ts`, `apps/studio/src/world/scene/worldMesh.ts`, `apps/studio/src/world/scene/worldMesh.test.ts`
- Modify: `apps/studio/src/api.ts` (`export * from "./api.world";` angehängt; `AssetKind` um `"WorldMap"`; `MapDto.world: boolean`), `world/scene/SceneHost.ts` (Haken), `world/scene/useSceneLoader.ts` und `world/scene/useRefreshQueues.ts` (je die Zeile `getMapModelGlb(id, area).then(parseGlb)` → `mapModel(h, id, area)`), `world/sprites.ts` (`export function toTexture`), `reload.ts` (`case "WorldMap"`), `workbench/useThumbnailReset.ts` (`resetWorldTextures()` neben `resetSprites()`), Tests `SceneHost.test.ts`, `reload.test.ts`

**Interfaces:**
- Consumes: Task 9 (DTOs, Befehle), Task 10 (Szene von 6.2).
- Produces:

```ts
// api.world.ts
export interface MeshTextureDto { name: string; width: number; height: number; role: "ground" | "wall" | "ramp" | "water" | "grass" | "piece"; palette?: string }
export interface WorldMeshDto { map: number; tileset: number; faces: number; textures: MeshTextureDto[]; corners: number[]; uv: number[]; texture: number[]; layer: number[] }
export const getWorldMesh = (map: number) => invoke<WorldMeshDto>("get_world_mesh", { map });
export const getTilesetTexture = (pkg: number, texture: string) => invoke<SpriteDto | null>("get_tileset_texture", { package: pkg, texture });
export const worldGet = (map: number) => invoke<WorldGetDto>("world_get", { map });   // WorldGetDto wie Task 9
// worldMesh.ts
/** One geometry per texture index: positions from `corners`, uv = (u / width, −v / height), indices 0 1 2, 0 2 3. */
export function meshGeometries(dto: WorldMeshDto): Map<number, THREE.BufferGeometry>;
/** The mesh group of map `map` (name `world-mesh:<map>`), textured where the package has the texture, else the placeholder of the role. */
export async function worldMeshModel(map: number): Promise<{ model: THREE.Group; faces: number }>;
/** The terrain model of `map` for the scene: the world mesh when `h.loaded.get(map)?.map.world`, else the GLB as before. */
export async function mapModel(h: SceneHost, map: number, area: number): Promise<THREE.Object3D>;
export function placeholderColor(role: MeshTextureDto["role"]): number;   // szene.gelaende nach „Entscheidungen“
export function resetWorldTextures(): void;
// SceneHost.ts
worldMeshes: Map<number, number>;                       // Karte → Flächen
noteWorldMesh(map: number, faces: number | null): void; // setzt/entfernt data-world-mesh und data-world-mesh-faces am Canvas
```

Material je Ebene: `MeshLambertMaterial`, `side: FrontSide`, `alphaTest: 0.5`, Überlagerungen `polygonOffset: true`, `polygonOffsetFactor = polygonOffsetUnits = −layer`; Fußabdrücke (255) halbdurchsichtig `opacity 0.6`. `removeMapModel` ruft `noteWorldMesh(map, null)`; `mapModel` ruft `noteWorldMesh(map, faces)`.

- [ ] **Step 1: Scheiternde Tests:** `worldMesh.test.ts`: `one_geometry_per_texture_with_normalized_uv` (DTO mit zwei Flächen, zwei Texturen 16×16 und 32×32: Positionen gleich den Ecken, UV geteilt und v negiert), `indices_make_two_counter_clockwise_triangles`, `placeholder_by_role_when_the_texture_is_missing` (`getTilesetTexture` → `null` → Farbe `szene.gelaende['hohes-gras']` für `grass`), `overlays_get_a_polygon_offset`, `map_model_takes_the_glb_for_binary_maps` (`world: false` → `getMapModelGlb` gerufen, `getWorldMesh` nicht); `SceneHost.test.ts`: `marks_the_canvas_with_world_meshes` (zwei Karten → `data-world-mesh="12,40"`, Flächen summiert; `null` für beide → Attribute weg); `reload.test.ts`: `world_map_reloads_map_and_model` (`WorldMap` 12 in der Szene → `map`, `mapModel`, `thumbnail`).
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, die Check-Skripte grün.
- [ ] **Step 5: Browser-Teil** (Test-Umgebung `G4_VITE_PORT=1460 G4_BRIDGE_PORT=1461`, Kopie `_testkopien/6-3a-web` von Platin DE mit `.nds` daneben, als Textprojekt eingerichtet über `call.sh adopt_text_project`): Projekt öffnen (`http://127.0.0.1:1460/?project=…`), Matrix 0 mit Zweiblattdorf; `call.sh agent_set_expert '{"on":true}'`; `mcp.sh call g4_call '{"command":"world_create","args":{"req":{"map":<Karte von Zweiblattdorf>}}}'`; Szene: `document.querySelector('canvas[data-world-mesh]').dataset` zeigt die Karte und `worldMeshFaces` = 1; `mcp.sh call g4_call '{"command":"world_edit","args":{"req":{"map":<…>,"changes":[{"cells":[[4,4],[5,4],[4,5],[5,5]],"height":2},{"cells":[[0,0],[1,0],[2,0]],"tile":"0001"}]}}}'` → Flächen > 1, Verlauf „Welt geändert: Karte … (7 Zellen)“; ⌘Z in der App → `call.sh world_get` gleich dem Stand nach dem Anlegen, Flächen 1; ⇧⌘Z → wieder > 1. Hell, Dunkel; Bilder über das Browser-Werkzeug beschreiben (Ort, Zahlen), Notiz nach `target/agent/gate-<datum>/6-3a/browser.md`.
- [ ] **Step 6: Commit** `feat(scene): world maps show their mesh instead of the GLB, data-world-mesh on the canvas` (mit Trailer).

### Task 12: Doku der Formate

**Files:**
- Create: `docs/formats/world_map.md`, `docs/formats/tileset.md`
- Modify: `docs/formats/text_project.md` (Tabelle „Arten“: `maps` mit `source: "world"`, Zeile `tilesets`; „Manifest“: `schemaVersion` 3, `schemas` mit `maps.world` und `tilesets`; „Migration“: Schritt 2 → 3; „Prüfungen“: die vier `world.*`-Codes), `docs/formats/map.md` (Satz „Welt-Karten: siehe `world_map.md`; bis 6.3b ist der Member der bisherige“)

- [ ] **Step 1: `world_map.md`:** Zweck und Grenze von 6.3a (Member bis 6.3b), Schlüssel mit Typen und Reihenfolge, Grammatik der Zellen (Tabelle Token → Bedeutung), Höhen und Rampen mit Richtungssinn und Zahlen (8 Einheiten je Stufe, −16 bis 62), Koordinaten und UV mit den Formeln aus „Entscheidungen“, Teilmodelle, Freiform (Schema), Permissions, ein synthetisches Beispiel (aus `fixtures::one_tile_world`, Zeilen mit „…“ gekürzt und gesagt), Store-Art und Undo, Prüfungen mit Wortlaut, Vernetzer-Regeln (Zusammenfassen, Wände, Reihenfolge, Budget-Formel), was 6.3b und 6.4a ergänzen.
- [ ] **Step 2: `tileset.md`:** Datei und Felder, Ableitung (Regel und Schwelle aus Task 0 mit Verweis auf `task0-6-3a-vorgaben.md`, Palettenregel, Rolle aus `kind`), Zwischenspeicher und `tileset_save`, reservierte Felder, synthetisches Tileset, „nie im Repo“.
- [ ] **Step 3: Commit** `docs(formats): world map and tileset formats, text project schema 3` (mit Trailer).

### Task 13: Design-System-Nachtrag

**Files:**
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 6.3a“ am Ende der Nachträge; unter „Was die App umsetzt“ die Zeile „Welt-Karten zeigen ihr Netz (6.3a)“; unter „Szene (three.js)“ Platzhalter-Material nach Rolle und `polygonOffset` je Ebene)

- [ ] **Step 1: Nachtrag** nach dem Muster „Nachtrag 3d.6“: Quelle (`world/scene/worldMesh.ts`, `SceneHost.ts`), Plan und Gate (`docs/phase-6-3a-gate.md`), keine neuen Tokens (Platzhalter aus `szene/gelaende/*`: `hohes-gras`, `wasser`, `blockiert`, `spezial`, `unbekannt`), keine neue Komponente, kein neuer Frame: „6.3a ändert keine Oberfläche außer dem Gelände in der Schau; Frames für Palette, Ebenen und Werkzeug kommen mit 6.4a (Screen „Gelände malen: Route 201“)“; Abweichung: Fußabdrücke halbdurchsichtig, bis 6.5b ohne Geometrie.
- [ ] **Step 2:** `bash scripts/check-tokens.sh` grün; **Step 3: Commit** `docs(design-system): addendum 6.3a, world mesh in the scene` (mit Trailer).

### Task 14: Gate 6.3a, `agent-gate.md`, `gate.sh`, Roadmap

**Files:**
- Create: `docs/phase-6-3a-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 6.3a“), `scripts/gate.sh` (Schritt angehängt), `docs/ROADMAP.md` (Abschnitt „Phase 6“: Stand unter der Tabelle, Zeile 6.3a), `docs/dev-setup.md` (gated `tests/world.rs` in der Tabelle `G4_TEST_PROJECT`)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben `tilesets …`, `world-mesh …`, `world-mesh-bridge …`, `textproject-stats …`.

- [ ] **Step 1: `gate.sh`:** nach der Schleife `run_timings` anhängen: `run_step "world mesh (timing)" "" env G4_TIMING_STRICT=1 cargo test --release -p g4-kernel --all-features --test world -- mesh_of_a_full_map_is_fast --nocapture --test-threads 1` und `run_step "world mesh bridge (timing)" "" env G4_TIMING_STRICT=1 cargo test -p studio world_mesh_over_the_command_path_is_fast -- --nocapture --test-threads 1` (Debug); die gated Tests von `tests/world.rs` laufen im bestehenden Schritt je Projekt mit.
- [ ] **Step 2: Gate-Datei** nach dem Muster `docs/phase-3d-6-gate.md`: Kopf mit Stand, das Gate aus Spezifikation §4 Zeile 6.3a als Zitat (wörtlich wie in Global Constraints); Legende „Wer“; Namen in der Oberfläche (`canvas[data-world-mesh]`, `data-world-mesh-faces`, Verlaufstexte „Welt angelegt: Karte NNNN“, „Welt geändert: Karte NNNN (N Zellen)“); Vorbereitung (Kopien `_testkopien/6-3a-{pt,hgde,hgus}` für (1), `6-3a-web` für (3), je mit `.nds`; Worktree-Hinweise: `G4_ROMS_DIR` absolut, `tools/apicula`, Ports 1460/1461; Last mit `uptime`). **(1) Automatik:** `scripts/gate.sh` grün (Hintergrund, `target/agent/6-3a-gate.log`); einzeln je Projekt im Release-Profil `vanilla_tilesets_have_one_ground_tile_per_texture`, `textproject_round_trip_with_world`, `textproject_export_idempotent_with_world`, `vanilla_migration_v2_to_v3_changes_only_the_manifest`; ohne ROM `one_tile_map_meshes_to_one_face`, `checkerboard_meshes_to_1024_faces`, `prop_world_json_round_trips_byte_equal`, `v2_to_v3_changes_only_the_manifest`, `world_create_and_edit_are_one_undo_step_each`, die zwei Zeittests; `cargo deny` grün. **(2) Messung:** Task 0 (Protokoll, Wahl). **(3) Browser-Teil** wie Task 11 Step 5. **(4) Hand und Blick:** Blick auf die Szene einer Welt-Karte (Textur, Wand, Rampe) in Hell und Dunkel; Hand: das Textprojekt mit Welt-Karte in einem älteren Studio (`schemaVersion` 3 → nur lesen). **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Pakete / Kacheln | vanilla / unklar | Regel (Task 0) | Übereinstimmung | Netz Release (ms) | Brücke Debug (ms) | Round-Trip | Export 2. Lauf | Migration | Last |
    |---|---|---|---|---|---|---|---|---|---|---|
    | Platin DE | 74 / 3 130 | | | | | | | | | |
    | HeartGold DE | 106 / 3 659 | | | | | | | | | |
    | HeartGold US | | | | | | | | | | |

- [ ] **Step 3: Gate abarbeiten** (Agent: (1) bis (3)); Werte eintragen; Befunde beheben (eigene Commits `fix(6.3a): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 4: `agent-gate.md`** Abschnitt „Phase 6.3a“ nach dem Muster „Phase 3d.6“: Kopien, was wo ist (`data-*`, Befehle über `g4_call` mit `--expert`), Tabelle Punkt / Wer / Wie, die Befehle für (1) und (3). **Roadmap:** unter der Tabelle „Phase 6“ „**Stand 6.3a <Datum>:** Gate vom Agenten bestanden: Tilesets 74 / 3 130 und 106 / 3 659, Vorgaben <Regel> mit <Übereinstimmung>, Netz <…> ms (Release), Brücke <…> ms (Debug), Round-Trip und Export idempotent mit einer Welt-Karte, Migration nur das Manifest; Branch `phase-6-3a-weltformat`; [docs/phase-6-3a-gate.md](phase-6-3a-gate.md); beim Nutzer offen: Blick, älteres Studio“.
- [ ] **Step 5: Trailer prüfen** `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`.
- [ ] **Step 6: Commit** `docs(6.3a): gate of world format and preview, agent-gate section, gate step, roadmap` (mit Trailer).

---

## Offen für die Umsetzung

- **Vorgaben aus Karten-weiter Zählung:** K1 und K2 kennen nur, welche Texturen eine Karte bindet, nicht, wo sie liegen; zeigt Task 0 unter 50 % Übereinstimmung, ist jede Textur „unklar“ und das Gate bleibt erfüllt (es zählt Kacheln). Die polygon-genaue Zuordnung kann 6.3b über den Codec von 6.1 nachreichen (`Deriver` ist der eine Ort dafür).
- **Karten je Paket:** Task 0 ordnet Karten wie Gutachten C über Texturnamen zu, der Kernel über Header und Matrizen; der gated Test druckt `maps per package`, weichen die Zahlen stark ab, steht der Unterschied im Protokoll (Abschnitt „Gegenprobe im Kernel“).
- **Zwei Assets in einer Einheit:** Der Abgleich (`textproject/sync`) und `check_units` kennen mehrteilige Einheiten von den Arten (Personal, Learnset, Evolution); prüft ein Test mit äußerer Änderung der Karten-JSON (Git-Checkout) etwas anderes als erwartet, folgt der Abgleich dem Muster der Arten, nicht einem Sonderweg für Welten.
- **`Location::Derived`:** jede `match`-Stelle auf `Location` (`store.rs` etwa acht, `project.rs`, Tests) braucht einen Arm; `rustc` findet sie. Ein `reload_from_disk` oder „Auf Original zurücksetzen“ einer Welt lehnt ab („Eine Welt hat kein Original; „Karte als Quelle übernehmen“ und zurück kommen mit 6.3b.“).
- **Gelöschte Karte mit Welt** (Grabstein aus 3d.4): die Hülle und `world` stehen nebeneinander in der Datei; `carry_world` und die Hülle dürfen sich nicht überschreiben (ein Test in Task 4, wenn `TOMBSTONE_ARTS` Karten enthält).
- **Name `worldMaps`:** `g4-world.json` (`world_meta`) nennt Außenkarten `worldMaps`; die Store-Art heißt im TypeScript `"WorldMap"`. In Doku und Oberfläche „Welt-Karte“ nur für Karten mit `source: "world"` verwenden.
- **Texturen der Szene:** Paletten mit falscher Zuordnung zeigen falsche Farben, nicht falsche Formen; fällt das im Blick auf, ergänzt Task 5 die Palettenregel aus den Material-Paaren der Vanilla-Modelle (6.1-Codec).
- **Wand an der Kartengrenze:** keine Wand in 6.3a; eine Karte, deren Rand höher liegt als die Nachbarkarte, zeigt ein Loch bis 6.4b (Nachbarn lesend).
- **Zeitgrenze Brücke:** gemessen warm; der erste Aufruf liest das Paket für `textures_only` und wird nur gedruckt. `tileset_get` ohne Datei leitet mit Vorgaben ab und liest dafür die Karten des Pakets; dauert das im Browser-Teil spürbar (über eine Sekunde), bekommt `Store` einen zweiten Zwischenspeicher für die Ableitung.
- **Parallele Merges:** `checks/mod.rs` (6.2), `world/scene/*` (6.2), `commands/dispatch.rs`, `lib.rs`, `access.rs`, `api.ts`, `ROADMAP.md`, `design-system/README.md` werden Konflikte an Anhängepunkten; die Erfahrung aus Phase 4 und 3d gilt (gesammelt lösen, `.fig` von main behalten).
