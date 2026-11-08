# Phase 6.4b „Smart Drawing und Muster“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein Tileset trägt **Kantensätze** (Wang-Ecken-Sätze mit den 13 PDSMS-Plätzen plus zwei Diagonalen, je Platz eine oder mehrere Kacheln mit Drehung, Außenkachel, Regel „obere/untere Zelle blockiert“), die man im **Kantensatz-Editor** im Inspector aus der Palette füllt; der **Terrain-Pinsel** (T) malt eine Fläche, und der Kernel setzt um sie herum die passenden Kanten- und Eckkacheln ohne falsche Ecke, der **Klippenpinsel** (K) hebt oder senkt die gemalten Zellen um eine Stufenzahl, setzt den Rand des Satzes oben auf das Plateau, Wände entstehen an der Höhenkante und die Kollision folgt der Kantenregel des Satzes; mehrere Kacheln je Platz wählt der Pinsel **zufällig, aber reproduzierbar** (Samen je Strich); an der Kartengrenze liest der Pinsel die Nachbarkarten der Matrix nur; die Schicht „Gelände“ zeigt **Muster** je Gelände (blockiert schraffiert, Wasser Wellen, hohes Gras Punkte, Vorsprung Pfeil, Eis Kreuzschraffur); über MCP malt `world_edit` (kuratiert) mit `terrain`/`cliff`, `tileset_edit` (über `g4_call`) ändert Kantensätze; `.fig` zeigt Palette, Ebenen und Kantensatz-Editor.

**Architecture:** Die Logik liegt im **Kernel**, nicht im Client: `world/edge.rs` (Datenmodell Kantensatz, Plätze als Eckmasken, Prüfungen, Zufallswahl), `world/smart.rs` (rein: Eckfarben aus vorhandenen Kacheln lesen, Strich anwenden, Masken zu Kacheln auflösen, Klippe mit Höhen und Überschreibungen, `wrong_corners`; dazu der Store-Teil `seams` für die Naht). `world_edit` bekommt zwei neue Änderungsarten `terrain` und `cliff`, die **vor** der bestehenden Prüfung zu gewöhnlichen `tile`/`height`/`ramp`/`permission`-Änderungen aufgelöst werden: ein Strich bleibt ein Undo-Schritt, das Weltformat, der Vernetzer und der Bäcker bleiben unverändert (Wände und Kantenregel gibt es seit 6.3a/6.3b). `tileset_edit` bekommt `sets` (Kantensatz anlegen, ersetzen, entfernen) als einen Dateischritt. Der Client (6.4a-Malmodell) schickt für T und K nur Zellen, Satz und Samen; der Kantensatz-Editor ist ein Abschnitt im Inspector neben Palette und Ebenen; die Muster sind ein Instanz-Attribut plus `onBeforeCompile` am Material der vorhandenen Gelände-Schicht (`content.ts`).

**Tech Stack:** Rust (g4-kernel: `world::{edge, smart, edit, tileset, places, bake}`, `terrain::derive`), serde, `proptest` (vorhanden), Tauri-Befehle und Brücke, MCP-Katalog (`apps/studio/src-tauri/src/mcp/catalog.rs`), React 19, three.js (`InstancedMesh`, `InstancedBufferAttribute`, `onBeforeCompile`), vitest, Python/uv für `scripts/g4emu`; **keine neue Abhängigkeit**.

**Spec:** [2026-11-05-phase-6-kartenerstellung-design.md](../specs/2026-11-05-phase-6-kartenerstellung-design.md) §1 (harte Grenzen, „Nicht in Phase 6“), §3.2 („Kantensätze (Wang-Ecken, 13 + 2 Plätze) ab 6.4b“), §3.4 (Kantenregel „obere Zelle“), §3.6 („Smart Drawing (6.4b): Kantensatz-Editor, Terrain-Pinsel, Klippenpinsel, Zufall“), §4 Zeile 6.4b (wörtlich unten) und „Phasen-Gate 6“, §6 (Gelände-Muster, Kantensatz-Editor), §7 („Umfangsdruck“, „Lizenz“), §8; [entscheidungen.md](../../research/2026-11-05-phase-6/entscheidungen.md) (Zeilen „Smart Drawing“, „Obere oder untere Zelle an der Kante“, „Malen über Kartengrenzen“, „Tileset-Quelle“, „Phasenschnitt“ Zeile 6.4b, „Nicht in Phase 6“ wörtlich, „Was bewusst nicht entschieden ist“ letzter Punkt); [Gutachten C](../../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §3.3 (Klippe), §3.4 (S2 als Datenmodell, S1 als Bedienung), §8.3 (Gate 6.4b); Formate [world_map.md](../../formats/world_map.md), [tileset.md](../../formats/tileset.md), [terrain.md](../../formats/terrain.md); Vorgänger-Pläne [6.4a](2026-11-07-phase-6-4a-malen.md) (Malmodell, Interfaces, Gate-Form) und [6.5b](2026-11-07-phase-6-5b-freiform-austausch.md) (parallel, Anhängepunkte).

Stand 08.11.2026: Basis ist `main` nach dem Merge von 6.4a, `<BASIS>` = `7a8e3915` (enthält 6.1, 6.2, 6.3a, 6.3b, 6.4a, 6.5a); **6.5b noch nicht gemergt** (Branch `phase-6-5b-freiform-austausch` in Arbeit): alle Zeilen mit Stand 6.5b gelten als fehlend, angehängt wird am jeweils aktuellen Dateiende, `TileChange.model` wird nicht vorausgesetzt; landet 6.5b auf main, wird main vor dem Abschluss-Review in den Branch gemergt. Arbeit im Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-4b`, Branch `phase-6-4b-smart-drawing` (`git worktree add .claude/worktrees/phase-6-4b -b phase-6-4b-smart-drawing <BASIS>`). Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`, Emulator `scripts/g4emu`.

## Voraussetzungen (auf `<BASIS>`, vor Task 1 mit `grep` prüfen)

Spalte „Stand“: **main** = beim Schreiben auf main `53210d31` gelesen; **6.4a** = aus dem Branch `phase-6-4a-malen` (beim Schreiben `c8339765`, Tasks 2–6 von 13 fertig) bzw. aus den Interfaces-Blöcken des 6.4a-Plans; **6.5b** = aus dem 6.5b-Plan (Branch `phase-6-5b-freiform-austausch`, in Arbeit). Zeilen mit 6.4a oder 6.5b zuerst auf `<BASIS>` bestätigen.

| Schnittstelle | erwartet | Stand | gebraucht in |
|---|---|---|---|
| `world::tileset::{Tileset {version, package, tiles, edge_sets: Vec<serde_json::Value>}, TileJson {…, wall, …}, TILESET_VERSION = 1, tileset_problems` („edge_sets: Kantensätze kommen mit 6.4b.“), `tile_outside, tileset_rel, read_tileset_file, Tileset::synthetic()` (6 Kacheln)`}` | `edge_sets` reserviert, muss leer sein | main | 1 |
| `world::tileset::{tileset_edit, TilesetEditRequest {package, changes}, TileChange {tile, role?, kind?, collision?, sound?, wall?}, TilesetEditReport {package, tiles, change}, tileset_edit_label, tileset_file_missing}`; ein Dateischritt, danach `forget_world_bakes` | | main | 1, 5 |
| `TileChange.model` angehängt, `model`-Regel in `tileset_problems` („`model` haben nur Teilmodelle (`piece`).“), Satz „Teilmodell-Dateien kommen mit 6.5b.“ entfällt | 6.5b Task 3 | **fehlt auf 7a8e3915** (6.5b nicht gemergt): `sets` wird am Ende von `TilesetEditRequest` angehängt | 1 (nur: wo angehängt wird) |
| `world::edit::{world_edit, WorldEditRequest {map, label?, changes}, WorldChange {layer?, cells, tile?, height?, ramp?, permission?}, WorldReport {map, tileset, cells, change, bake}, ONE_FIELD, NOTHING_TO_CHANGE, not_a_world_map}`; Änderungen werden gegen `before` geprüft (`check_change`), dann der Reihe nach angewandt; `height`/`ramp` entfernen `world.terrain` | `world/edit.rs:95–392` | 7a8e3915 | 4 |
| `world_layers`, `LayerOp`, `WorldLayersRequest` am Ende von `edit.rs` (baut `WorldReport`) | 6.4a Task 2 | 7a8e3915: `edit.rs:1445`; Konstruktoren von `WorldReport` in `edit.rs` 285, 380, 409, 1496 (keiner in `recipes/`) | 4 |
| `recipes/new_painted_map.rs` baut `WorldReport`/`NewPaintedMapReport`; `world_create` baut `WorldReport` | 6.4a Task 1 | 6.4a / main | 4 |
| `world::grid::{WorldGrid {tileset, layers: Vec<GridLayer {name, role, tiles: Vec<Option<TileRef>>}>, heights, ramps, pieces, derive, overrides}, WorldGrid::parse, height_grid, TileRef {tile, rotate, flip}, Ramp, parse_tile, tile_token, parse_height, override_token, set_token}` | | main | 2, 3 |
| `world::bake::{world_permissions, EMPTY_CELL, bake_grid, synthetic_pack_textures}`; Reihenfolge Kachel → Fußabdruck → `block_edges` → Überschreibung | `bake.rs:208–252` | 7a8e3915 | 3 |
| `terrain::block_edges(grid: &HeightGrid, cells: &mut [Cell])`, `edge_rule().free_below` (16 E), `HeightGrid::edge_units` | `terrain/derive.rs:38` | 7a8e3915: `derive.rs:38` | 3 |
| `world::mesh::mesh`: Wand zwischen Bodenzellen mit verschiedener Kantenhöhe, Textur `wall` der Kachel der **oberen** Zelle, sonst deren Kachel; keine Wand am Kartenrand | `mesh.rs` | main | 3, 9 |
| `world::doc::fixtures::{one_tile_world, gate_world}`, `WorldJson::blank` | | main | 2, 3, 9 |
| `world::places::{packs_of_map, headers_of_map}`, `Store::{matrix(index), member_count(AssetKind::Matrix), world, tileset}`, `Matrix {width, maps, map_at(x, y)}` | | main | 4 |
| MCP `catalog.rs`: 90 kuratierte Werkzeuge, Zähltest `30..=90` und `== 90`, `world_edit {req}` kuratiert mit Beschreibung („… exactly one of tile …, height …, ramp …, permission …“), `world_get`, `tileset_get` kuratiert; `tileset_edit`, `world_layers` über `g4_call` | 6.4a Task 3; 6.5b fügt keins hinzu | 7a8e3915: `catalog.rs:2477–2478` (`(30..=90).contains`, `assert_eq!(catalog().len(), 90)`; der Prüf-grep `== 90` trifft deshalb nicht) | 5 |
| Frontend `api.world.ts`: `WorldChange`, `WorldEditRequest`, `WorldReport`, `worldEdit`, `TileJson`, `TilesetDto`, `tilesetGet`, `tilesetSave`, `TileChange`, `tilesetEdit(pkg, changes)` | 6.4a Task 4 | 6.4a | 6, 7 |
| `world/paint/ops.ts::{PaintSub` (`brush rect fill erase pick height ramp`)`, Cell, PaintOptions {layer, tile, ramp, lower}, WorldCells, cellsOf, strokeLabel, strokeRequest(map, sub, cells, world, o), HEIGHT_MIN, HEIGHT_MAX}`; `VERBS` je Unterwerkzeug | `apps/studio/src/world/paint/ops.ts` | 7a8e3915: `ops.ts:11` | 6 |
| `workbench/paintTools.ts::{PaintToolSpec, PAINT_TOOLS` (B P G E I H R)`, PaintKeyAction, paintKey, paintPatch, paintStatusHints}`; `workbench/paint/PaintContext.tsx::{PaintState, INITIAL_PAINT, usePaint, PaintProvider}` | | 7a8e3915: `paintTools.ts:21` | 6 |
| `world/scene/strokes.ts::{Stroke {map, sub, cells, lower, preview?}, PaintCallbacks {…, look?(sub, lower)}, attachStrokes}`, `strokePreview.ts::{PreviewLook, setStrokePreview, holdStrokePreview, clearStrokePreview}`, `world/paint/usePaintSession.ts` (`strokeLook`, `applyRequest`, Warteschlange, `onNotice`), Leinwand `data-paint-sub`, `data-stroke-pending`, `__g4SceneProbe.{tileToClient, strokePending}` | | 6.4a | 6, 8 |
| `workbench/inspector/{TilePalette.tsx, LayerList.tsx, TileEditor.tsx, MatrixCellEditor.tsx}` (Abschnitte „Palette“, „Ebenen“, „Kachel“ im Malmodus; `[data-paint-palette]`, `[data-palette-tile]`, `[data-paint-layers]`, `[data-tile-editor]`, `[data-tileset-save]`), `world/paint/paletteImages.ts::tileImage`, Galerie `ui/gallery/pages/WorldPage.tsx` mit „Malwerkzeuge“, „Palette“, „Ebenen“ | 6.4a Tasks 7–8 | 7a8e3915: `TilePalette.tsx:76` | 7 |
| Szene: `content.ts::{buildContent, overlayColor, recolourMap, OVERLAY_OPACITY}` (Gelände-Schicht als `InstancedMesh` mit `MeshBasicMaterial`), `colors.ts::{permissionColor, CLASS_KEYS}` (`block water grass ledge ice …`), `legend.ts::{legendEntries, TerrainEntry}`, `TileBehavior {name, class}` | | main | 8 |
| `scripts/agent/scenarios/{pt,hg}-gemalt.yaml` über `recipe_apply new_painted_map` + kuratiertes `world_edit` (Matrix wächst nach Osten, Spielschritte `x + 32`) | 6.4a Task 11 | 7a8e3915 | 10 |
| `crates/g4-kernel/tests/world_paint.rs` (Einrichtung des Textprojekts auf `vanilla_copy()`, Zelle neben der Startstadt) | 6.4a Task 10 | 7a8e3915 | 9 |
| `docs/formats/world_map.md` „Was später kommt“ (Zeile „6.4b: Kantensätze … und Wände zu Nachbarkarten“), Abschnitte „Freiform und Beifiles“ (6.5b) und „Malen“ (6.4a); `docs/formats/tileset.md` mit `model` (6.5b) und Tileset-Editor (6.4a) | | 7a8e3915 ohne die 6.5b-Teile (Abschnitt „Freiform und Beifiles“, `model` fehlen) | 11 |
| `design-system/openpencil/NACHTRAG-6-4a.md`, `beispiele/gelaende-malen.body.js`, `jsx-lib-app.js`; README „Nachtrag 6.4a“, „Nachtrag 6.5b“ | 6.4a Task 12, 6.5b Task 12 | 7a8e3915 ohne „Nachtrag 6.5b“ (6.5b nicht gemergt) | 11 |

Prüfbefehle (auf `<BASIS>` im Worktree, jede Zeile muss treffen):

```bash
grep -n "edge_sets: Vec<serde_json::Value>" crates/g4-kernel/src/world/tileset.rs
grep -n "pub model: Option<Option<String>>" crates/g4-kernel/src/world/tileset.rs        # 6.5b
grep -n "pub fn world_layers" crates/g4-kernel/src/world/edit.rs                           # 6.4a
grep -n "pub fn block_edges" crates/g4-kernel/src/terrain/derive.rs
grep -n "== 90" apps/studio/src-tauri/src/mcp/catalog.rs                                    # 6.4a
grep -n "export type PaintSub" apps/studio/src/world/paint/ops.ts                          # 6.4a
grep -n "export const PAINT_TOOLS" apps/studio/src/workbench/paintTools.ts                 # 6.4a
grep -n "export function TilePalette" apps/studio/src/workbench/inspector/TilePalette.tsx  # 6.4a
grep -n "recipe_apply" scripts/agent/scenarios/pt-gemalt.yaml                              # 6.4a
```

Fehlt eine Zeile oder weicht sie ab (vor allem die mit Stand 6.4a/6.5b), zuerst die Abweichung in diese Tabelle schreiben und den betroffenen Task anpassen, dann beginnen.

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Wo die Logik liegt | Auflösung von Terrain- und Klippenpinsel im **Kernel** (`world/smart.rs`), aufgerufen aus `world_edit`; der Client schickt Zellen, Satz, Samen | eine Implementierung für App, MCP und gated Test; die Naht braucht Nachbarkarten, die nur der Kernel liest; 6.4a rechnet Werkzeuge im Client nur, wo sie ohne Tileset-Wissen gehen |
| Datenmodell Kantensatz | `Tileset.edge_sets: Vec<EdgeSetJson>`, je Satz `{name, outside, block, places}`; zwei Farben je Satz: **innen** (die gemalte Fläche) und **außen**; `outside` ist die Kachel der reinen Außenfläche (Pflicht), `block` `upper` (Vorgabe, weggelassen) oder `lower`, `places` ein Objekt mit den 15 Plätzen als Listen von Kachel-Token | Wang-Ecken wie Tiled (S2), Bedienung PDSMS (S1, C §3.4); zwei Farben genügen den 13 + 2 Plätzen; mehrere Gelände je Satz sind YAGNI |
| Plätze und Masken | Eckbits `NW = 1`, `NE = 2`, `SW = 4`, `SE = 8` (Bit gesetzt = innen). `center` 15; Kanten nach der Seite, die **außen** liegt: `n` 12, `s` 3, `w` 10, `e` 5; Außenecken nach ihrer Lage an der Fläche: `outer_nw` 8, `outer_ne` 4, `outer_sw` 2, `outer_se` 1; Innenecken nach der Ecke, die außen liegt: `inner_nw` 14, `inner_ne` 13, `inner_sw` 11, `inner_se` 7; Diagonalen `diag_nwse` 9, `diag_nesw` 6; Maske 0 = `outside`. Schlüsselreihenfolge in der Datei wie diese Aufzählung | 13 PDSMS-Plätze plus zwei Diagonalen = die 15 Masken ≠ 0 (C §3.4: „16 Kombinationen, S1 sind 13 davon, dazu die zwei Diagonalen und „leer““) |
| Drehung je Platz | Jeder Eintrag eines Platzes ist ein vollständiges Kachel-Token mit `r1..r3`/`f` (`"0007r1"`); eine Kachel darf gedreht mehrere Plätze füllen; **dasselbe Token** (nach `parse_tile` verglichen) darf im Satz nur einmal stehen (auch nicht als `outside`) | die Rückwärtssuche Token → Platz muss eindeutig sein; Vanilla-Pakete haben wenige Texturen (Median 30–38), Drehen spart Kacheln |
| Unvollständige Sätze | Speichern erlaubt (leere Plätze weggelassen); der **Strich** lehnt ab, sobald er einen leeren Platz bräuchte: „Kantensatz „Weg“: der Platz „Ecke innen NW“ hat keine Kachel (Zelle (6, 7)).“; nichts wird geschrieben | „ohne falsche Ecke“ heißt: lieber ablehnen als eine falsche Kachel setzen; der Editor füllt Plätze nach und nach |
| Eckfarben lesen | Eine Zelle ist **Mitglied** des Satzes, wenn ihr Token in einem Platz steht (Maske des Platzes), **außen**, wenn es `outside` ist (Maske 0), sonst **fremd** (liest nichts). Eine Ecke ist innen, wenn mindestens eine der bis zu vier angrenzenden lesbaren Zellen sie innen meldet | Gen-4-Karten speichern keine Eckfarben; die Kacheln tragen sie; „innen gewinnt“ hält eine bestehende Fläche zusammen |
| Terrain-Pinsel | Eine gemalte Zelle setzt ihre **vier Ecken** auf innen (⇧: außen); danach bekommt jede Zelle der Karte, die eine geänderte Ecke berührt, die Kachel ihrer neuen Maske. Unverändert bleiben: Zellen, deren Maske gleich bleibt (keine neue Zufallswahl), fremde Zellen mit neuer Maske 0. Mitglieder mit neuer Maske 0 bekommen `outside`. Ebene: `layer` des Strichs (Vorgabe 0) | PDSMS-Bedienung: die Fläche wächst um einen Ring mit Kanten und Ecken; stabile Zellen ändern sich nicht |
| Klippenpinsel | Die gemalten Zellen P bekommen die absolute Höhe `height` (Rampen dort entfernt). Eine Ecke, die P berührt, ist innen, wenn **alle** angrenzenden Zellen (in der Karte und lesbare Nahtzellen) mindestens `height` hoch liegen; neu aufgelöst werden P und jede Zelle mit Höhe ≥ `height`, die eine geänderte Ecke berührt; tiefere Zellen behalten ihre Kachel. Nur auf der Bodenebene: „Klippen gibt es nur auf der Bodenebene.“ | der Rand des Satzes liegt **oben** auf dem Plateau (Klippenkante wie in PDSMS), der Ring darunter bleibt Boden; zwei Striche mit demselben Satz nebeneinander verschmelzen |
| Wände der Klippe | Keine neue Regel: der Vernetzer zieht die Wand an der Höhenkante mit der `wall` der oberen Kachel (6.3a); der Editor setzt mit „Wand“ (`[data-edge-wall]`) die `wall` **aller** Kacheln des Satzes in einem `tileset_edit` | ein Bake ohne neue Regel bleibt byte-gleich für alles Bestehende; Wände ohne `wall` nehmen die Kachel selbst (wie heute) |
| Kollision der Klippe, Regel je Satz | `block: upper`: nichts zusätzlich, `terrain::block_edges` blockiert beim Backen die obere Zelle (Vorgabe, Entscheidung „obere Zelle“). `block: lower`: der Strich schreibt für jede Kante mit ≥ `edge_rule().free_below` (16 E) Unterschied, deren obere oder untere Zelle in P oder im Ring um P liegt, Überschreibungen: obere Zelle = ihr Kachelwert ohne Kanten-Bit (`tile_cells`), untere Zelle = Kachelwert `| 0x80`; eine Zelle mit schon vorhandener Überschreibung bleibt (Notiz „Zelle (12, 21) hat eine eigene Permission; die Klippe lässt sie stehen.“) | Entscheidungen: „im Kantensatz-Editor (6.4b) je Satz umstellbar“; Überschreibungen gibt es im Weltformat schon (`permissions.override`), der Bäcker bleibt unverändert; eine Tür-Überschreibung geht nie verloren |
| Zufall | Hat ein Platz mehrere Einträge, wählt der Kernel `entries[variant(seed, z · 32 + x, n)]` mit `variant(seed, cell, n) = splitmix64((u64::from(seed) << 32) \| cell as u64) % n` (Konstanten `0x9E3779B97F4A7C15`, `0xBF58476D1CE4E5B9`, `0x94D049BB133111EB`, Verschiebungen 30, 27, 31); `seed` steht in der Änderung (`terrain.seed`, `cliff.seed`, Vorgabe 0); die App zieht je Strich einen Samen (`crypto.getRandomValues`) | derselbe Strich mit demselben Samen gibt dieselben Kacheln (Test, MCP); Undo/Redo stellen die Kacheln aus dem Dokument her; nur neu aufgelöste Zellen würfeln |
| Gewichte | keine (gleich wahrscheinlich) | YAGNI; eine Kachel zweimal eintragen geht nicht (Eindeutigkeit), Gewichte später |
| Naht zu Nachbarkarten | Der Kernel sucht die **erste** Matrixzelle, die die Karte zeigt (Matrizen aufsteigend, Zellen zeilenweise), und liest aus den 8 Nachbarzellen dieser Matrix den Ring der Zellen direkt hinter dem Kartenrand (Koordinaten −1 und 32): für den Terrain-Pinsel die Kacheln der gleichnamigen Ebene (Index), für die Klippe die Höhen der Bodenebene; nur Nachbarn, die **Welt-Karten mit demselben Tileset** sind; alle anderen und leere Zellen lesen nichts. Nachbarkarten werden **nie** geschrieben; passt eine Nachbarzelle danach nicht mehr, steht eine Notiz im Bericht („Naht zu Karte 0124: 3 Zellen dort passen nicht mehr zum Satz „Weg“; die Nachbarkarte bleibt, wie sie ist.“) | Entscheidungen „Malen über Kartengrenzen: je Karte, Nachbarn nur lesend (Kantensätze an der Naht)“; das Spiel lädt 2 × 2 Karten, die Matrix-Lage ist die eine Nachbarschaft |
| Wände zu Nachbarkarten | **nicht in 6.4b**; `world_map.md` „Was später kommt“ wird von „6.4b: … Wände zu Nachbarkarten“ auf „später“ umgestellt | eine Wand an der Naht machte das Backen einer Karte von der Nachbarkarte abhängig (Eingabe-Hash, inkrementeller Build, Byte-Treue); weder Spezifikation noch Gate verlangen sie |
| Weltformat, Bäcker, Schemata | unverändert: `WORLD_VERSION` 2, `WORLD_BAKE_REV` wie nach 6.5b, `TILESET_VERSION` 1 (das Feld `edge_sets` war schon reserviert; Dateien ohne es lesen weiter), keine Migration | Smart Drawing erzeugt nur Kacheln, Höhen und Überschreibungen, die das Format schon hat |
| `world_edit`-Änderungen | `WorldChange` bekommt `terrain: {set, inside = true, seed = 0}` und `cliff: {set, height, seed = 0}`; `ONE_FIELD` wird „Eine Änderung setzt genau eines von tile, height, ramp, permission, terrain, cliff.“; die Auflösung läuft der Reihe nach auf einer Arbeitskopie (eine spätere Änderung sieht die frühere) und ersetzt jede Smart-Änderung durch ihre Zell-Änderungen, dann gilt der bestehende Weg (Prüfung, ein Schritt, Backen). Das Label zählt die **gemalten** Zellen | ein Strich = ein Undo-Schritt (Spezifikation §3.6); das Label sagt, was der Nutzer berührt hat |
| Nichts geändert | Ein Smart-Strich, der keine Zelle ändert, bleibt `NOTHING_TO_CHANGE` im Kernel; die App zeigt dafür **keinen** Fehler (still wie 6.4a „Unveränderte Zellen“) | der Client kann vorher nicht wissen, ob sich etwas ändert |
| Bericht | `WorldReport.notes: Vec<String>` angehängt (`skip_serializing_if = "Vec::is_empty"`), Naht- und Überschreibungs-Notizen; jeder Konstruktor von `WorldReport` setzt `notes: Vec::new()` | die App zeigt Notizen über `onNotice` (6.4a), MCP liest sie in der Antwort |
| Kantensätze ändern | `TilesetEditRequest.sets: Vec<EdgeSetOp>` angehängt (Vorgabe leer; `changes` darf leer sein, wenn `sets` nicht leer ist), `EdgeSetOp = {put: EdgeSetJson} \| {remove: {name}}`; `put` ersetzt den Satz gleichen Namens an seiner Stelle oder hängt an; **ein** Dateischritt mit Label „Kantensatz „Weg“ angelegt: Tileset 0006“ / „… geändert: …“ / „… entfernt: …“ (nach der ersten Operation; mit `sets` gilt immer dieses Label); `remove` eines fehlenden Satzes: „Kantensatz „Weg“ gibt es im Tileset 0006 nicht.“ | 6.3b-Befehl erweitern statt eines neuen; eine Änderung im Editor = ein Verlaufseintrag |
| MCP | `world_edit` (kuratiert) beschreibt `terrain` und `cliff`; `tileset_get` gibt `edge_sets` typisiert mit aus; `tileset_edit` mit `sets` bleibt über `g4_call`. **Kuratierte Zahl bleibt 90** (6.4a 90, 6.5b +0, 6.4b +0): Zähltest und `the_curated_set_is_the_decided_one` unverändert, ein Test hält die neuen Felder in der Beschreibung | Spezifikation §3.6 nennt nur `world_get`/`world_edit`/`tileset_get`; kein neues Werkzeug nötig |
| Tasten | **T** Terrain-Pinsel (Tileds Terrain-Pinsel; im Malmodus vor „Tür“, wie G vor „Gebäude“ in 6.4a), **K** Klippenpinsel (frei); ⇧ beim Strich: Terrain nimmt die Fläche weg (außen), Klippe senkt statt hebt; Esc wie 6.4a | Gutachten C §4.3 (Tiled-Kürzel); K ist in keiner Leiste belegt |
| Klippenhöhe in der App | Werkzeugoption „Stufen“ (1 bis 8, Vorgabe 2) im Abschnitt „Kantensätze“; Ziel = Höhe der ersten Zelle des Strichs ± Stufen, geklemmt auf −16 bis 62 (sonst Notiz wie 6.4a „N Zellen schon an der Grenze …“, kein Aufruf) | 2 Stufen = 16 E ist die kleinste Kante, die die Kantenregel blockiert (B §2.7) |
| Aktiver Satz | `PaintState.edgeSet: string \| null` (Name), gewählt im Abschnitt „Kantensätze“; ohne Satz malen T/K nicht, Notiz „Erst einen Kantensatz wählen (Inspector › Kantensätze).“ | Palette und Satz hängen am Tileset der gewählten Karte |
| Strichvorschau | T: Platten in der Platzhalter-/Texturfarbe der ersten `center`-Kachel (⇧: der `outside`-Kachel); K: Farbe der Zielstufe (`stepColor`) wie die Höhe in 6.4a; nach der Antwort Netz und Welt neu (6.4a) | der Ring ist erst nach der Kernel-Antwort bekannt; Gutachten C §3.5 (optimistisch, Netz danach) |
| Kantensatz-Editor: Ort und Bedienung | Abschnitt „Kantensätze“ im Inspector (`MatrixCellEditor`, Malmodus einer Welt-Karte) unter „Palette“/„Ebenen“: Liste der Sätze mit Radio „Aktiv“, „Neuer Kantensatz“ (Name, `outside` = gewählte Palettenkachel), darunter der Editor des aktiven Satzes: Raster der Plätze (Block 3 × 3 `outer_nw n outer_ne / w center e / outer_sw s outer_se`, Block 2 × 2 `inner_nw inner_ne / inner_sw inner_se`, Zeile `diag_nwse diag_nesw`), ein Platz wird per Klick gewählt (`aria-pressed`), ein Klick auf eine Palettenkachel hängt das aktive Token (mit Z/X-Drehung) an den gewählten Platz; Varianten als Chips mit „Entfernen“; Felder „Außen“, „Blockiert an der Klippe“ (Segment „Obere Zelle“/„Untere Zelle“), „Wand“, „Satz entfernen“; Zähler „13 von 13 Plätzen · Diagonalen 0 von 2“. Jede bestätigte Änderung = ein `tileset_edit` | Inspector ist die Hand (6.4a); Klick statt Ziehen (das Gutachten nennt Hineinziehen; Klick ist dieselbe Handlung ohne Drag-Code und tastaturbedienbar) |
| Gelände-Muster | Je Zelle der Gelände-Schicht ein Instanz-Attribut `aPattern = (code, dir)`: Vorsprung (Klasse `ledge`) **Pfeil** in Sprungrichtung aus dem Verhaltensnamen (`…NORTH/EAST/SOUTH/WEST`, sonst Nord), sonst Bit 7 der Kollision **Schraffur**, sonst Klasse `water` **Wellen**, `grass` **Punkte**, `ice` **Kreuzschraffur**, sonst keins; der Fragment-Shader (über `onBeforeCompile` am vorhandenen `MeshBasicMaterial`) zeichnet die Musterlinien in der Zellfarbe ×0,55 mit doppelter Deckkraft; `recolourMap` setzt das Muster mit; die Legende „Gelände“ zeigt je Eintrag das Muster | Spezifikation §6 (Liste wörtlich); Vorsprünge tragen 0x80, springen aber: der Pfeil sagt mehr als die Schraffur; die Farbe bleibt `permissionColor` (keine neuen Tokens) |
| Muster auf welchen Karten | allen (binär und Welt), immer wenn „Gelände“ sichtbar ist | die Schicht ist dieselbe |
| Fester gated Musterfall | Das **Plus** aus Querbalken x 4–12, z 8–10 und Längsbalken x 7–9, z 5–13 auf `one_tile_world` (Außenkachel), Terrain-Pinsel mit dem 13er-Satz; Erwartung als Platz-Raster (Task 2) in synthetischen Tests und als Kachel-Raster auf den drei ROM-Kopien (Task 9) | das Plus braucht alle 13 Plätze und keine Diagonale; ein Raster ist die wörtliche Gate-Form |
| Synthetische Testdaten | `Tileset::synthetic_edges()` = `synthetic()` plus 16 Bodenkacheln 6–21 (`syn_kante_00` … `syn_kante_15`, 16 × 16) und die Sätze `fixtures::weg13()` (13 Plätze `0006` (Mitte) bis `0012`, außen `0000`) und `fixtures::weg15()` (dazu `diag_nwse` `0013`, `diag_nesw` `0014`, Mitte mit Variante `["0006", "0015"]`; Belegung in Task 1); `synthetic_edge_pack_textures()` in `bake.rs`; `synthetic()` bleibt unverändert | 6.3a/6.3b-Tests zählen 6 Kacheln; keine ROM-Daten |
| `.fig`-Frames | „Gelände malen: Kantensatz-Editor“ (Draufsicht mit Plus und Klippe, Werkzeugleiste mit T und K, Inspector mit Palette, Ebenen, Kantensätzen und offenem Editor; `MODE = 'kantensatz'`), dasselbe Dunkel, „Gelände-Muster“ (Schicht „Gelände“ mit allen fünf Mustern und Legende; `MODE = 'muster'`), Brett „Komponenten (App): Kantensatz“ (Platz leer/gefüllt/gewählt/mit Varianten-Zähler, Varianten-Chip, Segment Obere/Untere Zelle, Musterproben; `MODE = 'brett'`) | Gate „`.fig` zeigt Palette, Ebenen, Kantensatz-Editor“: der erste Frame zeigt alle drei, auch falls die Frames von 6.4a noch nicht gerendert sind |

### Nicht in 6.4b

- Kanten-Sätze (Tiled „edge“, Wege und Zäune als Linien), mehr als zwei Gelände je Satz, Gewichte, Blob/47-Sätze, Automapping; Kantensätze im TMJ (6.5b „Nicht in 6.5b“).
- Wände zu Nachbarkarten, Schreiben in Nachbarkarten, Malen über mehrere Karten in einem Strich.
- Klippen auf Überlagerungen, Rampen aus dem Klippenpinsel, Klippen mit Treppe; Aufräumen alter Überschreibungen, wenn eine Klippe später eingeebnet wird (bleibt Sache des Nutzers über `permission: "."`).
- Ziehen aus der Palette in den Editor (Klick reicht), Kantensätze zwischen Tilesets kopieren.
- „Nicht in Phase 6“ der Entscheidungen gilt wörtlich („Automapping und Kantensätze über die 13 + 2 Plätze hinaus“).

## Parallele Arbeit und Anhängepunkte

6.4b beginnt nach den Merges von 6.4a und 6.5b; es läuft keine Schwester-Teilphase daneben. Trotzdem nur anhängen an: `commands/dispatch.rs`, `mcp/access.rs`, `src-tauri/src/lib.rs`, `world.rs` (`pub mod edge; pub mod smart;`, `pub use`), `api.world.ts`, `docs/mcp.md`, `docs/agent-gate.md`, `docs/ROADMAP.md`, `design-system/README.md`, `mcp/docs.rs`. `scripts/gate.sh` bleibt unverändert (die neuen Tests laufen in den vorhandenen Schritten mit).

## Global Constraints

- **Lizenz:** „Lizenz: gen4-studio ist MIT. Die Writer von DSPRE (NsbmdWriter, NsbtxWriter, TileBake, BdhcBuild) sind AGPL-3.0: kein Code übernehmen, auch nicht abschreiben. Erlaubt sind Formatwissen aus pret/pokeplatinum (bdhc.c, land_data.c, easy3d.c), eigene Messungen und Dokumentation.“ Tiled ist GPL und PDSMS hat keine Lizenz: das Datenmodell der Wang-Ecken folgt Tileds Idee, die Füll- und Eckenlogik entsteht eigenständig, kein Code aus beiden (kein Blick in `src/tiled`, kein PDSMS-Quelltext).
- „Keine Abhängigkeit vom NITRO-SDK (g3dcvtr.exe).“
- „Byte-Treue: Unveränderte Karten bauen weiterhin byte-gleich. Die Original-Assets bleiben unberührt, solange der Nutzer nicht malt.“ Der Bäcker ändert sich nicht; der gated Test `world_bake` (alle Karten mit Form 1 byte-gleich) läuft im Gate mit.
- „Keine ROM-Daten im Repo.“ (`bash scripts/check-no-rom-data.sh` vor jedem Commit); Kantensätze von Vanilla-Paketen entstehen nur im Projekt des Nutzers und in gated Tests auf Kopien; Szenarien und Tests nennen Kacheln nur als Index; Galerie und `.fig` zeigen Platzhalter.
- **Keine neue Abhängigkeit** (npm oder Cargo); `cargo deny --locked check licenses advisories` grün.
- **Worktree:** `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; `tools` als Symlink auf den Haupt-Checkout (`ln -s /Users/POLTEP/Projekte/lab/editor/gen4-studio/tools tools`, nie einchecken), `G4_APICULA=/Users/POLTEP/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula`; Test-Umgebung nur über `scripts/agent/studio-web.sh` mit `G4_VITE_PORT=1600 G4_BRIDGE_PORT=1601`, andere Ports nie anfassen; Testkopien nur `Roms/_testkopien/6-4b-<zweck>` (`cp -cR` Ordner, `cp -c` `.nds`, Ziel existiert noch nicht), Originale nie öffnen; gated Tests schreiben nur auf `vanilla_copy()`.
- cargo-Tests immer mit `--all-features`. Jeder Task endet mit einem Commit; jede Nachricht endet mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`). **Kein `scripts/gate.sh` im Worktree** (läuft auf main), kein Merge, kein Push, kein `git stash`; nie `cd` in den Haupt-Checkout.
- Lang laufende Befehle (gated Tests, Workspace-Tests, Brücke, Szenarien) im Hintergrund mit Log unter `target/agent/6-4b/<zweck>.log`, `uptime` davor; ein Test, der unter Last reißt, wird allein wiederholt, nie gelockert.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh`; bei `scripts/g4emu` `uv run pytest -m "not integration" -q`; immer `bash scripts/check-no-rom-data.sh`.
- Kein `unwrap`/`expect`/Panik im Produktcode von `edge.rs` und `smart.rs`: eine Tileset-Datei kann von Hand geändert sein (Satz mit kaputtem Token, Satz verschwunden).
- UI nur aus Bausteinen und Tokens (Werkzeug-Taste 40 × 40 mit Kürzel, Segment, Chip, Statusleiste, `szene/gelaende/*`, `szene/hoehe/0..6`); **keine neuen Tokens**; deutsche Texte, englische Bezeichner und Commit-Messages.
- **Gate 6.4b** (Spezifikation §4, wörtlich): „Ein Kantensatz aus 13 Kacheln umrandet eine gemalte Fläche ohne falsche Ecke (gated über ein festes Muster, Erwartung als Kachel-Raster); Klippenpinsel setzt Wände und Kollision nach der Kantenregel; Muster im Overlay sichtbar (Blick); `.fig` zeigt Palette, Ebenen, Kantensatz-Editor“, dazu (Kopf der Gate-Spalte) gated Tests auf frischen Kopien `_testkopien/6-4b-*` von Platin DE, HeartGold DE und HeartGold US, Browser-Teil über die Test-Umgebung, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer.
- **Phasen-Gate 6** (§4, wörtlich; mit 6.3b und 6.4a erreicht, 6.4b darf es nicht brechen: `pt-gemalt`/`hg-gemalt` laufen im Gate unverändert mit): „Eine im Studio gemalte Karte mit eigenem Gelände, Höhenstufe mit Treppe, Wasser und Gras-Kacheln und abgeleiteter Kollision wird gebaut und in eine Matrix gesetzt. Sie ist in g4emu und melonDS begehbar, auf Platin DE und HeartGold DE: die Treppe funktioniert (`expect-height`), die Wand blockiert (`walk-until-blocked`), Gras löst eine Begegnung aus (`expect-battle`), die Außenkarte sieht bei Tag und Nacht richtig beleuchtet aus (Blick). Unveränderte Projekte bauen byte-gleich, `vanilla_checks` hat 0 Fehler, `scripts/gate.sh` ist grün, DSPRE öffnet die Karte (Hand).“

## Review Focus

- **Ein Strich bräuchte einen leeren Platz** (Satz mit 13 Plätzen, zwei Flächen berühren sich nur an einer Ecke → Diagonale; ein halb gefüllter Satz aus dem Editor): Ablehnung mit Platzname und Zelle, nichts geschrieben, kein Verlaufseintrag (Task 2 `a_missing_place_refuses_with_its_name`, Task 4 `a_refused_smart_stroke_writes_nothing`).
- **Fremde Kacheln und ein zweiter Satz in der Nähe:** fremde Zellen außerhalb des Rings bleiben Byte für Byte, fremde Zellen im Ring werden überschrieben, Zellen eines anderen Satzes lesen als fremd (Task 2 `foreign_tiles_outside_the_ring_stay`, `a_second_set_reads_as_foreign`).
- **Die Naht:** Nachbarkarte ist binär, hat ein anderes Tileset, fehlt, die Karte liegt am Matrixrand oder in zwei Matrizen: Nachbarn werden nie geschrieben, nur lesbare Nachbarn zählen, eine Notiz nennt kaputte Nahtzellen (Task 4 `neighbours_are_never_written`, `a_binary_or_foreign_neighbour_reads_nothing`, `the_seam_continues_a_neighbour_s_area`).
- **Kantensatz geändert, nachdem gemalt wurde** (Kachel aus einem Platz genommen, Satz umbenannt oder entfernt, Datei von Hand kaputt): bestehende Zellen lesen als fremd, kein Panik; ein Strich mit fehlendem Satz: „Kantensatz „Weg“ gibt es im Tileset 0006 nicht.“; eine kaputte Datei lehnt mit ihrem Pfad ab (Task 2 `cells_of_a_changed_set_read_as_foreign`, Task 4 `a_stroke_with_a_missing_set_is_refused`, Task 1 `a_broken_set_is_a_schema_problem`).
- **Klippe mit Regel „untere Zelle“ neben einer Tür-Überschreibung, dann Regel zurück auf „obere“:** die Tür bleibt (Notiz), nur Kantenzellen bekommen Überschreibungen, nach dem Umstellen setzt ein neuer Klippenstrich keine Überschreibungen mehr (Task 3 `lower_rule_keeps_existing_overrides`, `upper_rule_writes_no_overrides`).

---

### Task 1: Kantensatz im Tileset (Datenmodell, Prüfungen, `tileset_edit` mit `sets`)

**Files:**
- Create: `crates/g4-kernel/src/world/edge.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/world/tileset.rs` (`edge_sets: Vec<EdgeSetJson>`, Prüfungen, `TilesetEditRequest.sets`, Label, `Tileset::synthetic_edges()`, Modul-Doku), `crates/g4-kernel/src/world/bake.rs` (`synthetic_edge_pack_textures()` neben `synthetic_pack_textures`), `crates/g4-kernel/src/world.rs` (`pub mod edge;`, Re-Exporte angehängt)

**Interfaces:**
- Consumes: `world::grid::{parse_tile, tile_token, TileRef}`, `tileset::{tile_outside, tileset_problems, tileset_edit}`.
- Produces:

```rust
// world/edge.rs
pub const NW: u8 = 1; pub const NE: u8 = 2; pub const SW: u8 = 4; pub const SE: u8 = 8;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place { Center, N, E, S, W, OuterNw, OuterNe, OuterSw, OuterSe, InnerNw, InnerNe, InnerSw, InnerSe, DiagNwse, DiagNesw }
impl Place {
    pub const ALL: [Place; 15];                    // file order
    pub const GATE13: [Place; 13];                 // ALL without the diagonals
    pub fn mask(self) -> u8;                       // table „Plätze und Masken“
    pub fn of_mask(mask: u8) -> Option<Place>;     // None for 0 (outside) and > 15
    pub fn key(self) -> &'static str;              // "outer_nw"
    pub fn label(self) -> &'static str;            // „Ecke außen NW“, „Kante N“, „Mitte“, „Ecke innen NW“, „Diagonale NW–SO“, „Diagonale NO–SW“
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum EdgeBlock { #[default] Upper, Lower }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct EdgePlaces { /* 15 fields named Place::key, each #[serde(default, skip_serializing_if = "Vec::is_empty")] Vec<String> */ }
impl EdgePlaces { pub fn get(&self, p: Place) -> &[String]; pub fn get_mut(&mut self, p: Place) -> &mut Vec<String>; }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct EdgeSetJson { pub name: String, pub outside: String,
    #[serde(default, skip_serializing_if = "is_upper")] pub block: EdgeBlock, pub places: EdgePlaces }
/// Problems of set `index` (paths relative to `edge_sets[index]`) against a tileset of `tiles` tiles.
pub fn edge_set_problems(set: &EdgeSetJson, index: usize, package: u16, tiles: usize) -> Vec<String>;
/// The variant of a place with `n` entries for cell `cell` (z · 32 + x) and stroke seed `seed` (rule „Zufall“).
pub fn variant(seed: u32, cell: usize, n: usize) -> usize;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum EdgeSetOp { Put(EdgeSetJson), Remove { name: String } }    // JSON {"put": {…}} | {"remove": {"name": …}}
pub fn edge_set_label(op: &EdgeSetOp, existed: bool, package: u16) -> String;  // „Kantensatz „Weg“ angelegt|geändert|entfernt: Tileset 0006“
pub fn missing_set(name: &str, package: u16) -> String;                       // „Kantensatz „Weg“ gibt es im Tileset 0006 nicht.“
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures { pub fn weg13() -> EdgeSetJson; pub fn weg15() -> EdgeSetJson; }
// world/tileset.rs
// Tileset: #[serde(default, skip_serializing_if = "Vec::is_empty")] pub edge_sets: Vec<EdgeSetJson>
// TilesetEditRequest: … , #[serde(default)] pub sets: Vec<EdgeSetOp>   (appended)
#[cfg(any(test, feature = "test-support"))] impl Tileset { pub fn synthetic_edges() -> Tileset; }
// world/bake.rs
#[cfg(any(test, feature = "test-support"))] pub fn synthetic_edge_pack_textures() -> Vec<PackTexture>;
```

Fixtures: `weg13` = `{name: "Weg", outside: "0000", places: center ["0006"], n ["0007"], e ["0008"], s ["0009"], w ["000a"], outer_nw ["000b"], outer_ne ["000c"], outer_sw ["000d"], outer_se ["000e"], inner_nw ["000f"], inner_ne ["0010"], inner_sw ["0011"], inner_se ["0012"]}`; `weg15` = `weg13` mit Name „Weg15“, `diag_nwse ["0013"]`, `diag_nesw ["0014"]`, `center ["0006", "0015"]`. `synthetic_edges()` hat 22 Kacheln (`synthetic()` + `syn_kante_00..15` an 6–21, Rolle `ground`, `origin synthetic`) und `edge_sets = [weg13()]`.

Prüfsätze in `edge_set_problems` (Pfad vorn, wie die übrigen `tileset_problems`): leerer Name „edge_sets[0].name: Ein Kantensatz braucht einen Namen.“; Token, das `parse_tile` ablehnt (sein Satz); `.` „edge_sets[0].places.n[1]: `.` ist keine Kachel eines Platzes.“ (für `outside` ebenso); Kachel jenseits des Tilesets (`tile_outside`); doppeltes Token „edge_sets[0].places.e[0]: Kachel 0007r1 steht zweimal im Satz („n“ und „e“).“ (`outside` heißt dort „outside“). `tileset_problems` ruft sie je Satz auf und meldet doppelte Namen „edge_sets[1].name: Kantensatz „Weg“ gibt es zweimal.“; der Satz „Kantensätze kommen mit 6.4b.“ entfällt.

`tileset_edit` mit `sets`: Operationen der Reihe nach auf die Arbeitskopie, dann die vorhandenen `changes`; „Nichts zu ändern.“ wenn die Datei gleich bleibt; `changes` leer und `sets` leer bleibt „Nichts zu ändern.“; Label = `edge_set_label` der ersten Operation, wenn `sets` nicht leer ist, sonst wie bisher; `origin: user` nur für Kacheln, die `changes` ändert.

- [ ] **Step 1: Scheiternde Tests** in `edge.rs`: `masks_are_the_fifteen_non_zero_ones` (`ALL` deckt 1–15 je einmal, `of_mask(m).mask() == m`, `of_mask(0) == None`), `gate13_has_no_diagonal`, `keys_and_labels_in_file_order`, `a_set_round_trips_in_key_order` (JSON von `weg13()`: Schlüssel `name, outside, places`, ohne `block`; `places` in der Reihenfolge `center, n, e, s, w, outer_nw, …`; leere Plätze fehlen; mit `block: lower` steht `block` nach `outside`), `variant_is_stable_and_spread` (gleiche Eingabe gleiche Ausgabe; für `n = 2` über die 1 024 Zellen bei Samen 0 und 1 je Wert zwischen 400 und 624 Treffer; `n = 1` immer 0), `edge_set_problems_name_each_case` (alle Sätze oben wörtlich), `labels_say_what_happened`. In `tileset.rs`: `edge_sets_read_and_check` (Datei mit `weg13()` liest und hat keine Probleme; der alte Test `edge_sets: Kantensätze kommen mit 6.4b.` auf `a_broken_set_is_a_schema_problem` umgestellt: Satz mit `"0g07"` → Pfad und Satz des Parsers, doppelte Namen), `tileset_edit_puts_and_removes_a_set_as_one_step` (Datei aus `synthetic_edges()` ohne Sätze: `put weg13` → Verlauf ein Eintrag „Kantensatz „Weg“ angelegt: Tileset 0000“, Datei mit Satz; `put` geänderter Satz → „… geändert …“ an derselben Stelle; `remove` → „… entfernt …“; `undo` dreimal → Datei wie am Anfang), `removing_a_missing_set_is_refused`, `sets_and_wall_changes_are_one_step` (`put` plus `changes` mit `wall` für alle 13 Kacheln: ein Eintrag, Label des Satzes), `files_without_edge_sets_read_unchanged` (Datei einer 6.3b-Fixture byte-gleich nach Lesen und Schreiben).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::edge world::tileset` → FAIL (`edge` fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; die Modul-Doku von `tileset.rs` nennt `edge_sets` nicht mehr „reserviert“.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` (Hintergrund, `target/agent/6-4b/t1.log`) grün.
- [ ] **Step 5: Commit** `feat(world): edge sets in the tileset (wang corners, 13 + 2 places), edited as one file step` (mit Trailer).

### Task 2: Terrain-Pinsel auflösen (rein)

**Files:**
- Create: `crates/g4-kernel/src/world/smart.rs` (reiner Teil, + Tests in `crates/g4-kernel/src/world/smart/tests.rs`)
- Modify: `crates/g4-kernel/src/world.rs` (`pub mod smart;`)

**Interfaces:**
- Consumes: Task 1 (`Place`, `EdgeSetJson`, `variant`), `WorldGrid`, `TileRef`, `parse_tile`, `tile_token`.
- Produces:

```rust
// world/smart.rs
/// Cells just past the map's edge (x or z −1 or 32) read from neighbour world maps: tiles per layer index and ground
/// heights; a cell that is not there reads nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seams { pub tiles: BTreeMap<(usize, i8, i8), TileRef>, pub heights: BTreeMap<(i8, i8), i8>,
    /// The neighbour map of each seam cell, for the notes.
    pub owner: BTreeMap<(i8, i8), u16> }
/// What a smart change resolves to (cells map-local, each once, row-major).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolved { pub tiles: Vec<([u8; 2], String)>, pub heights: Vec<([u8; 2], i8)>, pub ramps_cleared: Vec<[u8; 2]>,
    pub overrides: Vec<([u8; 2], String)>, pub notes: Vec<String> }
/// The terrain brush on `layer`: the corners of `cells` set inside (`inside`) or outside, the touched cells resolved
/// (rules „Eckfarben lesen“, „Terrain-Pinsel“). Err: the sentence of the first place without tile.
pub fn paint_terrain(grid: &WorldGrid, layer: usize, set: &EdgeSetJson, cells: &[[u8; 2]], inside: bool, seed: u32,
    seams: &Seams) -> Result<Resolved, String>;
/// Cells of `layer` whose corners disagree with a neighbour's (both members of `set` or `outside`), by the top-left
/// cell of each pair, sorted, without repeats.
pub fn wrong_corners(grid: &WorldGrid, layer: usize, set: &EdgeSetJson) -> Vec<[u8; 2]>;
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    /// The plus: x 4–12 z 8–10 and x 7–9 z 5–13.
    pub fn plus_cells() -> Vec<[u8; 2]>;
    /// The expected places of the plus on `one_tile_world`, x 2–14, z 3–15 (legend in the test).
    pub const PLUS_EXPECTED: [&str; 13];
}
```

Eckfarbe als Raster 33 × 33 plus Nahtecken; `Resolved.tiles` enthält nur Zellen, deren Token sich ändert. Notiz an der Naht (Satz unter „Entscheidungen“), wenn eine Nahtzelle, die vorher passte, nach dem Strich eine andere Maske hätte. Tokens werden kanonisch (`tile_token(parse_tile(..))`) zurückgegeben.

`PLUS_EXPECTED` (Spalten x 2–14, Zeilen z 3–15; `.` unverändert `0000`, `C` center, `N E S W` Kanten, `1 2 3 4` outer_nw/ne/sw/se, `5 6 7 8` inner_nw/ne/sw/se):

```text
z 3   .............
z 4   ....1NNN2....
z 5   ....WCCCE....
z 6   ....WCCCE....
z 7   .1NN5CCC6NN2.
z 8   .WCCCCCCCCCE.
z 9   .WCCCCCCCCCE.
z 10  .WCCCCCCCCCE.
z 11  .3SS7CCC8SS4.
z 12  ....WCCCE....
z 13  ....WCCCE....
z 14  ....3SSS4....
z 15  .............
```

- [ ] **Step 1: Scheiternde Tests** (`one_tile_world`, `Tileset::synthetic_edges()`, `Seams::default()`): `the_plus_is_ringed_without_a_wrong_corner` (**Gate-Muster**: `paint_terrain(…, weg13(), plus_cells(), true, 0, …)` angewandt ergibt Zelle für Zelle `PLUS_EXPECTED` mit den Tokens von `weg13`, alle übrigen Zellen `0000`; `wrong_corners == []`; jeder der 13 Plätze kommt vor), `painting_the_same_again_changes_nothing` (`Resolved.tiles` leer), `a_single_cell_is_ringed_by_outer_corners_and_edges` (Zelle (10, 10): 3 × 3 aus `1 N 2 / W C E / 3 S 4`), `erasing_the_middle_of_the_plus` (⇧ auf x 8, z 9: die Zelle wird `0000`, um sie Kanten und Innenecken, `wrong_corners == []`), `erasing_everything_returns_to_outside` (alle Plus-Zellen außen → Karte ganz `0000`), `a_missing_place_refuses_with_its_name` (Review Focus: `weg13`, zwei Zellen (4, 4) und (5, 5) → Diagonale nötig → „Kantensatz „Weg“: der Platz „Diagonale NW–SO“ hat keine Kachel (Zelle (5, 5)).“ – Zelle und Platz aus der tatsächlichen Maske), `diagonals_resolve_with_weg15` (dieselben Zellen mit `weg15`: eine Zelle `diag_*`, `wrong_corners == []`), `variants_follow_the_seed` (`weg15`, Plus mit Samen 1 und 2: Mitte aus `0006`/`0015` je nach `variant`; gleicher Samen → gleiches Ergebnis; ein zweiter Strich daneben würfelt die alten Mittelzellen nicht neu), `foreign_tiles_outside_the_ring_stay` (Review Focus: Zellen `0001` neben dem Ring bleiben, `0001` im Ring wird Kante), `a_second_set_reads_as_foreign` (Review Focus: zweiter Satz mit eigenen Tokens), `cells_of_a_changed_set_read_as_foreign` (Review Focus: nach dem Plus `n` aus dem Satz genommen → die `0007`-Zellen lesen fremd, kein Panik, ein Strich ändert sie nur im Ring), `overlay_layer_is_painted_alone` (`layer 1` einer Welt mit Überlagerung: Ebene 0 unverändert), `prop_random_strokes_leave_no_wrong_corner` (proptest, 128 Fälle: 1–6 Striche aus Zufallszellen, innen oder außen, mit `weg15` → kein `Err`, `wrong_corners == []`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::smart` → FAIL.
- [ ] **Step 3: Umsetzen** `paint_terrain` und `wrong_corners`; Ablauf: Rückwärtstabelle Token → Platz (geparste `TileRef`), Eckraster aus den lesbaren Zellen (und `Seams.tiles` am Rand), Strich setzt Ecken, Zellen um geänderte Ecken neu bestimmen, fehlender Platz → `Err` mit der ersten Zelle in Zeilenreihenfolge.
- [ ] **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(world): terrain brush resolves wang corners into the places of an edge set` (mit Trailer).

### Task 3: Klippenpinsel (rein): Höhe, Rand oben, Kantenregel je Satz

**Files:**
- Modify: `crates/g4-kernel/src/world/smart.rs` (`paint_cliff`, Tests in `smart/tests.rs`), `crates/g4-kernel/src/world/bake.rs` (`tile_cells` aus `world_permissions` herausgezogen, `pub(crate)`), `crates/g4-kernel/src/terrain/derive.rs` (`high_edges` aus `block_edges` herausgezogen, `pub`)

**Interfaces:**
- Consumes: Task 2 (`Seams`, `Resolved`, Eckraster, Auflösung), `EdgeBlock`, `HeightGrid::edge_units`, `edge_rule()`.
- Produces:

```rust
// terrain/derive.rs
/// Every pair of neighbours inside the map (each once, row-major) whose edge heights differ by at least
/// `edge_rule().free_below`: (upper cell, lower cell). `block_edges` blocks the first of each.
pub fn high_edges(grid: &HeightGrid) -> Vec<(usize, usize)>;
// world/bake.rs
/// The cells of `grid` from tiles and footprints, before the edge rule and the hand values (first half of
/// `world_permissions`, which is now `tile_cells` + `block_edges`).
pub(crate) fn tile_cells(grid: &WorldGrid, tileset: &Tileset, family: Family) -> Vec<Cell>;
// world/smart.rs
/// The cliff brush: `cells` to `height` (ramps there cleared), the rim of `set` on top (rule „Klippenpinsel“), and for
/// `set.block == Lower` the overrides of rule „Kollision der Klippe“. Err: a place without tile, or
/// „Klippen gibt es nur auf der Bodenebene.“ is the caller's (layer) check.
pub fn paint_cliff(grid: &WorldGrid, tileset: &Tileset, family: Family, set: &EdgeSetJson, cells: &[[u8; 2]], height: i8,
    seed: u32, seams: &Seams) -> Result<Resolved, String>;
```

`block_edges` und `world_permissions` bleiben im Verhalten gleich (die 6.2/6.3b-Tests sind der Beleg).

- [ ] **Step 1: Scheiternde Tests:** `derive.rs` `high_edges_are_what_block_edges_blocks` (für `scenario_input()` und `gate_world`: die ersten Elemente = die Zellen, die `block_edges` sperrt); `bake.rs` `world_permissions_is_tile_cells_plus_block_edges` (alle Fixtures). `smart/tests.rs` (`synthetic_edges`, `weg13`, `Family::Pt`): `a_cliff_raises_the_cells_with_the_rim_on_top` (Rechteck x 20–25, z 20–24 auf Höhe 2: Höhen dort 2, sonst 0; Rand `1 N N N N 2 / W C C C C E / … / 3 S S S S 4` auf den gemalten Zellen, Ring darunter unverändert `0000`; `wrong_corners == []`), `a_cliff_bakes_walls_and_blocks_the_upper_cell` (Welt mit `Resolved` angewandt, `mesh`: die 22 Randkanten (2 · 6 + 2 · 5) sind von senkrechten Flächen der Ebene 0 überdeckt; `world_permissions`: die 18 Randzellen blockiert, die 26 Ringzellen frei; keine Überschreibung in `Resolved`), `upper_rule_writes_no_overrides` (Review Focus), `lower_rule_blocks_the_ring_by_overrides` (`block: lower`: `Resolved.overrides` = 18 Randzellen mit Kachelwert ohne 0x80, 22 Ringzellen an Kanten (ohne die vier Eckzellen des Rings, die keine Kante teilen) mit `| 0x80`; nach dem Backen ist genau der Ring blockiert), `lower_rule_keeps_existing_overrides` (Review Focus: Tür `6980` auf einer Ringzelle bleibt, Notiz wörtlich), `lowering_digs_a_pit` (Höhe −2: der gemalte Bereich ist unten, der Rand liegt auf den gemalten Zellen nur, wo alle Nachbarn ≥ −2 sind, also im Inneren; Ring oben blockiert nach `block_edges`), `a_second_cliff_next_to_the_first_merges` (zweites Rechteck direkt östlich, gleiche Höhe: die gemeinsame Randspalte wird `C`, `wrong_corners == []`), `one_step_cliffs_stay_walkable` (Höhe 1: Wände ja, `world_permissions` blockiert nichts), `a_thin_cliff_is_outside_on_top` (eine Zeile breit: Maske 0 → `outside`, kein Fehler), `cliff_ramps_are_cleared`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::smart world::bake terrain::derive` → FAIL.
- [ ] **Step 3: Umsetzen**; die Überschreibungen kommen aus `high_edges(new_grid.height_grid())` gefiltert auf Paare mit einer Zelle in P oder ihrem 8er-Ring, Werte aus `tile_cells(new_grid, …)` als `override_token`.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` (Hintergrund) grün.
- [ ] **Step 5: Commit** `feat(world): cliff brush raises cells with the rim on top and blocks by the edge set's rule` (mit Trailer).

### Task 4: Naht und `world_edit` mit `terrain` und `cliff`

**Files:**
- Modify: `crates/g4-kernel/src/world/smart.rs` (Store-Teil `seams`, `expand`), `crates/g4-kernel/src/world/places.rs` (`neighbour_maps`), `crates/g4-kernel/src/world/edit.rs` (`WorldChange.terrain`, `.cliff`, `ONE_FIELD`, Aufruf von `expand` vor `check_change`, `WorldReport.notes`; die bestehenden Tests mit dem alten `ONE_FIELD`-Text umgestellt), jeder Konstruktor von `WorldReport` (`edit.rs`, `recipes/new_painted_map.rs`, ggf. `world/freeform.rs` von 6.5b)

**Interfaces:**
- Consumes: Tasks 1–3; `Store::{world, tileset, matrix, member_count}`.
- Produces:

```rust
// world/places.rs
/// The maps in the 8 matrix cells around the first cell that shows `map` (matrices ascending, cells row-major),
/// order NW, N, NE, W, E, SW, S, SE; None for an empty cell, a cell off the matrix, or no cell at all.
pub fn neighbour_maps(store: &mut Store, map: u16) -> Result<[Option<u16>; 8], KernelError>;
// world/edit.rs
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct TerrainPaint { pub set: String, #[serde(default = "yes")] pub inside: bool, #[serde(default)] pub seed: u32 }
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct CliffPaint { pub set: String, pub height: i8, #[serde(default)] pub seed: u32 }
// WorldChange: … , #[serde(default)] pub terrain: Option<TerrainPaint>, #[serde(default)] pub cliff: Option<CliffPaint>
// WorldReport: … , #[serde(default, skip_serializing_if = "Vec::is_empty")] pub notes: Vec<String>
pub const ONE_FIELD: &str = "Eine Änderung setzt genau eines von tile, height, ramp, permission, terrain, cliff.";
// world/smart.rs (store side)
/// The seam cells of `map` on `layer` (rule „Naht zu Nachbarkarten“).
pub fn seams(store: &mut Store, map: u16, world: &WorldJson, layer: usize) -> Result<Seams, KernelError>;
/// `changes` with every terrain and cliff change replaced by its cell changes (tile on its layer, height, ramp ".",
/// permission), resolved in order on a copy of `world`; the notes of all of them. Refused with the set's or the
/// brush's sentence (missing set, place without tile, cliff off the ground, height outside −16 to 62).
pub fn expand(store: &mut Store, map: u16, world: &WorldJson, tileset: &Tileset, changes: &[WorldChange])
    -> Result<(Vec<WorldChange>, Vec<String>), KernelError>;
```

`world_edit`: nach `first_error_with(&before, …)` `expand`, dann wie bisher mit den aufgelösten Änderungen; `cells` (Label) zählt `req.changes` (die gemalten Zellen); eine Smart-Änderung mit `layer` jenseits der Ebenen: der vorhandene Satz für fehlende Ebenen; `cliff` mit `layer` ≠ 0: „Klippen gibt es nur auf der Bodenebene.“; die Notizen gehen nach `WorldReport.notes`.

- [ ] **Step 1: Scheiternde Tests** in `edit.rs` (Fixture `test_support::world_ready()` mit Tileset-Datei aus `synthetic_edges()` und Welt `one_tile_world` auf Karte 0; eine zweite Welt-Karte östlich in der Matrix für die Naht): `a_terrain_stroke_is_one_step` (`world_edit {changes: [{cells: plus_cells(), terrain: {set: "Weg"}}], label: "Gelände gemalt: Karte 0000 (45 Zellen)"}` (`plus_cells()` hat 45 Zellen ohne Wiederholung) → ein Verlaufseintrag mit diesem Label, Welt = `PLUS_EXPECTED`, `bake.ok`; `undo` → `one_tile_world`), `a_cliff_stroke_is_one_step_and_drops_the_verbatim_terrain` (Klippe auf einer Welt mit `terrain` → `terrain` weg, Höhen gesetzt, ein Eintrag), `a_refused_smart_stroke_writes_nothing` (Review Focus: Diagonale nötig → Ablehnung mit Platzsatz, `history()` unverändert), `a_stroke_with_a_missing_set_is_refused` (Review Focus: Satz wörtlich), `cliff_off_the_ground_is_refused`, `one_field_names_terrain_and_cliff`, `smart_and_plain_changes_apply_in_order` (erst `tile` 0001 auf x 0–31 z 0, dann `terrain` auf (5, 1): der Ring sieht die 0001-Zeile als fremd), `nothing_changed_is_nothing_to_change`, `the_seam_continues_a_neighbour_s_area` (Review Focus: Nachbarkarte östlich mit Fläche bis x 31 → Strich bis x 31 auf Karte 0 ergibt an x 31 Mitte statt Kante), `neighbours_are_never_written` (Review Focus: Welt und Datei der Nachbarkarte byte-gleich, eine Notiz „Naht zu Karte 0001: …“, wenn die Nachbarzellen nicht mehr passen), `a_binary_or_foreign_neighbour_reads_nothing` (Review Focus: binäre Nachbarkarte, Nachbar mit anderem Paket, Karte am Matrixrand: Ergebnis wie ohne Nachbar), `notes_are_in_the_report`; in `places.rs` `neighbour_maps_in_the_first_matrix` (Reihenfolge, Rand → `None`, Karte in zwei Matrizen → die erste).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::edit world::places world::smart` → FAIL.
- [ ] **Step 3: Umsetzen**; `recipes`- und `world_layers`-Tests bleiben grün (nur `notes: Vec::new()` ergänzt).
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` (Hintergrund, `target/agent/6-4b/t4.log`) grün.
- [ ] **Step 5: Commit** `feat(world): world_edit paints terrain and cliffs with edge sets, neighbours read at the seam` (mit Trailer).

### Task 5: Befehle und MCP: `world_edit`-Beschreibung, `tileset_edit` mit `sets`, `docs/mcp.md`

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/world_map.rs` (Tests am Ende), `apps/studio/src-tauri/src/mcp/catalog.rs` (Beschreibung und Eingabeschema von `world_edit`, Test), `docs/mcp.md` (Abschnitt „Smart Drawing (6.4b)“ am Ende; Tabelle der Werkzeuge: Zeile `world_edit` ergänzt)

**Interfaces:**
- Consumes: Task 1 (`TilesetEditRequest.sets`), Task 4 (`WorldChange.terrain/.cliff`, `WorldReport.notes`).
- Produces: keine neuen Befehle; Schema `world_edit.changes[]` um `terrain {set, inside?, seed?}` und `cliff {set, height, seed?}`; Beschreibung von `world_edit` um den Satz: „terrain {set, inside? (default true), seed?} paints the inside (or with inside false the outside) of an edge set of the tileset on the cells and sets the matching edge and corner tiles around them; cliff {set, height, seed?} sets the cells to height (ground only) with the set's rim on top, walls follow the height edge and collision the set's rule (block upper or lower); entries with several tiles are chosen by seed. Neighbour maps are read at the seam, never written; notes say where they no longer fit.“

- [ ] **Step 1: Scheiternde Tests:** `world_map.rs` `edge_sets_over_dispatch` (Fixture `world_ready()` mit `tileset_save` für Paket 0 aus `synthetic_edges` ohne Sätze: `tileset_edit {req: {package: 0, changes: [], sets: [{put: weg13}]}}` → `tileset_get` zeigt `edge_sets[0].name == "Weg"`; `world_edit` mit `terrain` → Verlauf zwei Einträge; `undo` zweimal), `smart_stroke_notes_reach_the_answer`; `catalog.rs`: `world_edit_describes_terrain_and_cliff` (Beschreibung enthält `terrain {set`, `cliff {set, height`; Schema hat beide Felder), Zähltest `== 90` und `the_curated_set_is_the_decided_one` **unverändert** grün, `curated_world_tools_route_to_their_commands` mit einer `terrain`-Änderung.
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features world_ catalog` → FAIL.
- [ ] **Step 3: Umsetzen**; `docs/mcp.md` „Smart Drawing (6.4b)“: Kantensatz-Form (Plätze und Masken als Tabelle, `outside`, `block`), `tileset_edit` mit `sets` über `g4_call` (Beispiel `put`/`remove`, Labels, Ablehnungen), `world_edit` mit `terrain`/`cliff` (Beispiel „das Plus“, Ablehnungen, Notizen), Satz „Die Zahl der kuratierten Werkzeuge bleibt 90.“.
- [ ] **Step 4: Lauf** → PASS.
- [ ] **Step 5: Commit** `feat(mcp): world_edit paints terrain and cliffs, tileset_edit changes edge sets; curated count stays 90` (mit Trailer).

### Task 6: Malmodell im Client: Terrain- und Klippenpinsel

**Files:**
- Modify: `apps/studio/src/api.world.ts` (Typen am Ende ergänzt), `apps/studio/src/world/paint/ops.ts` (+ Test), `apps/studio/src/workbench/paintTools.ts` (+ Test), `apps/studio/src/workbench/paint/PaintContext.tsx` (+ Test), `apps/studio/src/world/paint/usePaintSession.ts` (+ Test), `apps/studio/src/app/useShortcuts.ts` (+ Test, falls die Tasten dort gelistet sind), `apps/studio/src/ui/primitives/icons.ts` (zwei Symbole, falls nicht da)

**Interfaces:**
- Consumes: Task 5 (Befehlsform), 6.4a (`strokeRequest`, `PAINT_TOOLS`, `paintKey`, `paintStatusHints`, `usePaintSession`, `strokeLook`).
- Produces:

```ts
// api.world.ts
export type EdgePlace = "center" | "n" | "e" | "s" | "w" | "outer_nw" | "outer_ne" | "outer_sw" | "outer_se"
  | "inner_nw" | "inner_ne" | "inner_sw" | "inner_se" | "diag_nwse" | "diag_nesw";
export const EDGE_PLACES: EdgePlace[];   // file order
export interface EdgeSetJson { name: string; outside: string; block?: "upper" | "lower"; places: Partial<Record<EdgePlace, string[]>> }
export type EdgeSetOp = { put: EdgeSetJson } | { remove: { name: string } };
// WorldChange: … ; terrain?: { set: string; inside?: boolean; seed?: number }; cliff?: { set: string; height: number; seed?: number }
// WorldReport: … ; notes?: string[]
// TilesetDto.tileset: … ; edge_sets?: EdgeSetJson[]
export const tilesetEdit = (pkg: number, changes: TileChange[], sets: EdgeSetOp[] = []) => invoke<…>("tileset_edit", { req: { package: pkg, changes, sets } });
// ops.ts
export type PaintSub = "brush" | "rect" | "fill" | "erase" | "pick" | "height" | "ramp" | "terrain" | "cliff";
// PaintOptions: … ; edgeSet: string | null; cliffRise: number   (1..8, default 2)
export function strokeRequest(map: number, sub: Exclude<PaintSub, "pick">, cells: Cell[], world: WorldCells, o: PaintOptions, seed?: number):
  { req: WorldEditRequest; skipped: number } | null;
// paintTools.ts: PAINT_TOOLS + terrain (T, „Terrain-Pinsel“), cliff (K, „Klippenpinsel“)
```

`strokeRequest`: `terrain` → `{layer: o.layer, cells: own, terrain: {set: o.edgeSet, inside: !o.lower, seed}}`, `cliff` → Ziel = `heights[z0][x0] ± o.cliffRise` der ersten eigenen Zelle; außerhalb −16 bis 62 → `null` mit `skipped = own.length`; sonst `{cells: own, cliff: {set, height, seed}}`; ohne `o.edgeSet` → `null`. Labels: `VERBS.terrain` „Gelände gemalt“, `VERBS.cliff` „Klippe gesetzt“, mit ⇧ „Gelände entfernt“ / „Klippe gesenkt“. `usePaintSession`: Samen je Strich aus `crypto.getRandomValues(new Uint32Array(1))[0]`; `NOTHING_TO_CHANGE` („Nichts zu ändern.“) aus einem T/K-Strich ist kein Fehler; `report.notes` gehen an `onNotice`; ohne Satz Notiz „Erst einen Kantensatz wählen (Inspector › Kantensätze).“; `strokeLook`: `terrain` Platzhalterfarbe der Rolle der ersten `center`-Kachel (⇧: `outside`), `cliff` `stepColor(start ± rise)`; `applyRequest` lässt `terrain`/`cliff` aus (die nächste Füllung rechnet nach der Antwort mit dem neu gelesenen Dokument). Statusleiste: T „Ziehen malt „Weg“ · ⇧Ziehen nimmt es weg“, K „Ziehen hebt um 2 Stufen mit dem Rand „Weg“ · ⇧Ziehen senkt“ (Name und Stufen aus dem Zustand). Symbole: `terrain`/`cliff` aus `icons.ts`, sonst Phosphor `Mountains`/`Stack`, im README-Nachtrag genannt.

- [ ] **Step 1: Scheiternde Tests:** `ops.test.ts`: `terrain_request_sends_cells_set_and_seed`, `shift_terrain_paints_outside`, `cliff_targets_the_first_cell_plus_rise` (erste Zelle auf 1, Stufen 2 → `height 3`; ⇧ → `-1`), `cliff_past_the_range_sends_nothing`, `no_edge_set_sends_nothing`, `smart_labels`; `paintTools.test.ts`: `t_and_k_are_the_smart_tools`, `no_paint_key_clashes` (B P G E I H R T K Z X ∩ {V, F, A, S} leer; T schlägt „Tür“ nur im Malmodus); `PaintContext.test.tsx`: `edge_set_and_rise_default_to_none_and_two`; `usePaintSession.test.tsx`: `a_terrain_stroke_is_one_world_edit_with_seed`, `nothing_to_change_from_a_smart_stroke_is_silent`, `notes_become_notices`, `without_a_set_the_stroke_asks_for_one`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm vitest run src/world/paint src/workbench src/app` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm test`, `pnpm build`, Prüfskripte.
- [ ] **Step 5: Commit** `feat(studio): terrain and cliff brushes (T, K) send an edge set, its seed and the cells` (mit Trailer).

### Task 7: Kantensatz-Editor im Inspector

**Files:**
- Create: `apps/studio/src/workbench/inspector/EdgeSetEditor.tsx`, `.module.css`, `.test.tsx`; `apps/studio/src/workbench/inspector/edgeSets.ts`, `edgeSets.test.ts` (reine Helfer)
- Modify: `apps/studio/src/workbench/inspector/MatrixCellEditor.tsx` (+ Test: Abschnitt „Kantensätze“ im Malmodus), `apps/studio/src/workbench/inspector/TilePalette.tsx` (+ Test: Klick hängt an den gewählten Platz, solange ein Platz gewählt ist), `apps/studio/src/ui/gallery/pages/WorldPage.tsx` (+ Test: Block „Kantensatz-Editor“ mit Platzhaltern)

**Interfaces:**
- Consumes: Task 6 (`EdgeSetJson`, `EdgeSetOp`, `EDGE_PLACES`, `tilesetEdit`, `usePaint().set({edgeSet, cliffRise})`), 6.4a (`TilePalette`, `tileImage`, `placeholderColor`, `usePaintSession().tileset`).
- Produces:

```ts
// edgeSets.ts
export const PLACE_LABEL: Record<EdgePlace, string>;      // „Mitte“, „Kante N“, „Ecke außen NW“, „Ecke innen NW“, „Diagonale NW–SO“ …
export const PLACE_GRID: (EdgePlace | null)[][];          // 3 × 3, 2 × 2, 1 × 2 blocks of rule „Kantensatz-Editor“
export function withVariant(set: EdgeSetJson, place: EdgePlace, token: string): EdgeSetJson;   // appended, not twice
export function withoutVariant(set: EdgeSetJson, place: EdgePlace, token: string): EdgeSetJson;
export function completeness(set: EdgeSetJson): { places: number; diagonals: number };          // of 13 and of 2
export function newSet(name: string, outside: string): EdgeSetJson;
// EdgeSetEditor.tsx
export function EdgeSetEditor(props: { tileset: TilesetDto; active: string | null; place: EdgePlace | null; tile: string;
  onActive(name: string | null): void; onPlace(place: EdgePlace | null): void; onChanged(): void; onError(e: unknown): void; depth: Depth }): JSX.Element;
```

Texte und Attribute: Abschnitt „Kantensätze“ (`[data-edge-sets]`), Zeilen `[data-edge-set="Weg"]` mit Radio „Aktiv“ und dem Zähler „13 von 13 Plätzen · Diagonalen 0 von 2“; „Neuer Kantensatz“ (Namensfeld, Vorgabe „Satz <n>“; `outside` = aktive Palettenkachel); Editor `[data-edge-editor]`: Plätze `[data-edge-place="outer_nw"]` (40 × 40, Bild der ersten Variante oder leere Fläche mit gestricheltem Rand, kleiner Zähler bei mehr als einer Variante, `aria-pressed`, `aria-label` „Ecke außen NW, 2 Kacheln“), Varianten des gewählten Platzes als Chips `[data-edge-variant="0007r1"]` mit „Entfernen“, „Außen“ `[data-edge-outside]` (übernimmt die aktive Kachel), Segment „Blockiert an der Klippe“ `[data-edge-block]` „Obere Zelle“/„Untere Zelle“, „Wand“ `[data-edge-wall]` (Kachelnummer, leer = keine; setzt `wall` aller Kacheln des Satzes über `changes` im selben `tileset_edit`), „Stufen der Klippe“ `[data-cliff-rise]` (1 bis 8, nur Zustand), „Satz entfernen“. Ohne Datei: gesperrt mit dem Knopf „Tileset im Projekt anlegen“ wie im Abschnitt „Kachel“ (6.4a). Jede bestätigte Änderung = **ein** `tileset_edit` mit `sets: [{put}]` bzw. `[{remove}]`. Tüfteln: Plätze zeigen zusätzlich die Token.

- [ ] **Step 1: Scheiternde Tests:** `edgeSets.test.ts` (`grid_holds_all_fifteen_places_once`, `variants_append_once_and_remove`, `completeness_counts_thirteen_and_two`); `EdgeSetEditor.test.tsx` (`clicking_a_place_selects_it`, `a_palette_click_adds_the_turned_token_to_the_place` (über `MatrixCellEditor`: Platz `n`, Kachel `0007`, Z → `tilesetEdit(0, [], [{put: …n: ["0007r1"]}])`), `removing_a_variant_is_one_edit`, `block_lower_is_one_edit`, `wall_sets_every_tile_of_the_set_in_the_same_edit`, `new_set_takes_the_active_tile_as_outside`, `without_file_the_editor_waits_for_tileset_save`, `the_active_set_goes_to_the_paint_state`); `MatrixCellEditor.test.tsx` (`edge_sets_show_while_painting_a_world_map`); `WorldPage.test.tsx` (neuer Block).
- [ ] **Step 2: Lauf** `pnpm vitest run src/workbench/inspector src/ui/gallery` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Prüfskripte.
- [ ] **Step 5: Commit** `feat(studio): edge set editor with 13 + 2 places in the inspector while painting` (mit Trailer).

### Task 8: Gelände-Muster in der Szene, Legende, Browser-Teil

**Files:**
- Create: `apps/studio/src/world/scene/patterns.ts`, `patterns.test.ts`
- Modify: `apps/studio/src/world/scene/content.ts` (+ Test: Attribut je Instanz, Material mit Muster), `apps/studio/src/world/legend.ts` (+ Test: `TerrainEntry.pattern`), die Legenden-Komponente der Szene (+ Test: Musterprobe je Eintrag), `apps/studio/src/ui/gallery/pages/WorldPage.tsx` (+ Test: Block „Gelände-Muster“)

**Interfaces:**
- Consumes: `overlayColor`, `recolourMap`, `TileBehavior {name, class}`, `permissionColor`.
- Produces:

```ts
// patterns.ts
export type Pattern = "none" | "hatch" | "waves" | "dots" | "arrow" | "crosshatch";
export const PATTERN_CODE: Record<Pattern, number>;   // none 0, hatch 1, waves 2, dots 3, arrow 4, crosshatch 5
/** The pattern of a permission cell (rule „Gelände-Muster“); dir 0 N, 1 E, 2 S, 3 W (arrows only). */
export function patternOf(kind: number, collision: number, behavior: TileBehavior | undefined): { pattern: Pattern; dir: number };
/** Adds the instanced attribute `aPattern` (2 floats per instance) to `mesh` and the pattern code to its material. */
export function withPatterns(mesh: THREE.InstancedMesh): void;
export function setPatternAt(mesh: THREE.InstancedMesh, i: number, p: { pattern: Pattern; dir: number }): void;
```

Shader (eigenständig geschrieben): Plane-UV `vUv` je Zelle; Schraffur = Diagonalstreifen (`fract((u + v) * 4.0) < 0.22`), Kreuzschraffur = beide Diagonalen, Wellen = `abs(fract(v * 3.0 + 0.15 * sin(u * 6.2832 * 2.0)) - 0.5) < 0.09`, Punkte = Kreise Radius 0,12 auf einem 3 × 3-Raster, Pfeil = Schaft und Spitze in Richtung `dir` (UV vorher um `dir · 90°` gedreht); Musterpixel in Zellfarbe × 0,55 mit Deckkraft `min(1, 2 · OVERLAY_OPACITY)`, übrige Pixel wie bisher. Die Leinwand trägt `data-terrain-patterns="<Zahl der Zellen mit Muster>"`, solange „Gelände“ sichtbar ist.

- [ ] **Step 1: Scheiternde Tests:** `patterns.test.ts` (`blocked_cells_are_hatched`, `water_waves_grass_dots_ice_crosshatch`, `ledges_point_where_they_jump` (`JUMP_EAST` → `arrow`, dir 1; Vorsprung mit 0x80 bleibt Pfeil), `unknown_classes_have_none`, `with_patterns_adds_the_attribute_and_the_shader_hook` (Attribut `aPattern` mit `itemSize 2`, `onBeforeCompile` ersetzt `#include <color_fragment>` und deklariert `aPattern`)); `content.test.ts` (`build_content_sets_a_pattern_per_cell`, `recolour_updates_the_pattern`); `legend.test.ts` (`terrain_entries_name_their_pattern`); Legenden-Komponente (`legend_shows_a_pattern_swatch`); `WorldPage.test.tsx`.
- [ ] **Step 2: Lauf** `pnpm vitest run src/world src/ui/gallery` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm test`, `pnpm build`, Prüfskripte.
- [ ] **Step 5: Browser-Teil** (Test-Umgebung `G4_VITE_PORT=1600 G4_BRIDGE_PORT=1601 bash scripts/agent/studio-web.sh`, Debug-Binaries aus `cargo build --workspace --all-features`; Kopie `6-4b-web` von Platin DE mit `.nds`; vorbereitet wie 6.4a Task 9 Step 5 (`g4k git init`, `call.sh assign_base`, `call.sh adopt_text_project`, außen `git add -A && git commit -m Textprojekt`), dann „Neue Karte malen“ neben Zweiblattdorf über das „+“ (6.4a) und „Tileset im Projekt anlegen“):
  1. Abschnitt „Kantensätze“: „Neuer Kantensatz“ „Weg“ mit der Außenkachel 0000; die 13 Plätze aus den Palettenkacheln 0001 bis 000d (Platz wählen, Kachel klicken; je Klick ein Verlaufseintrag „Kantensatz „Weg“ geändert: Tileset NNNN“); Zähler „13 von 13 Plätzen“.
  2. T, das Plus (Task 2) über `__g4SceneProbe.tileToClient` als zwei Striche (Querbalken, Längsbalken) per `left_click_drag`, je Strich warten auf `canvas[data-stroke-pending="0"]`; `call.sh world_get <karte>` → Ebene 0 gleich `PLUS_EXPECTED` mit den Tokens (Vergleich mit einem Python-Einzeiler gegen das Raster, Ergebnis ins Protokoll).
  3. K mit Stufen 2 auf x 20–25, z 20–24: Szene zeigt Plateau und Wände, „Gelände“ zeigt die 18 Randzellen schraffiert; Segment auf „Untere Zelle“, zweiter Klippenstrich x 4–8, z 20–23: der Ring schraffiert, der Rand frei.
  4. „Gelände“ auf einer Vanilla-Karte mit Wasser, Gras und Vorsprung (Route 201 bzw. die nächste Karte mit allen dreien) und auf der gemalten Karte: Wellen, Punkte, Pfeil, Schraffur sichtbar (`[data-terrain-patterns]` > 0); Legende mit Mustern.
  5. Ausgangsstand: direkt nach „Tileset im Projekt anlegen“ ⌘S und außen `git add -A && git commit -m "Tileset angelegt"`; am Ende ⌘Z über alle Editor-Änderungen und Striche bis zu diesem Stand, ⌘S → `git status --porcelain` der Kopie **leer**, `call.sh dirty` `[]`.
  6. Bilder Hell, Dunkel, Kompakt (Kantensatz-Editor mit Palette und Ebenen; Plus und Klippe in der Draufsicht; Gelände-Muster) unter `target/agent/gate-2026-11-07/6-4b/blick-*.png`, Notiz `target/agent/gate-2026-11-07/6-4b/browser.md` (Karte, Satz, Zahlen, Zeiten).
- [ ] **Step 6: Commit** `feat(studio): terrain patterns in the scene overlay and the legend` (mit Trailer).

### Task 9: Gated Tests auf Platin DE, HeartGold DE, HeartGold US

**Files:**
- Create: `crates/g4-kernel/tests/world_smart.rs`
- Modify: `docs/dev-setup.md` (`tests/world_smart.rs` in der Tabelle `G4_TEST_PROJECT`)

**Interfaces:**
- Consumes: Tasks 1–4; 6.4a (`tests/world_paint.rs`-Einrichtung, `recipes::apply new_painted_map`); `vanilla_copy()`, `G4_TEST_PROJECT`, `build::build`.
- Produces: die Belegzeilen `smart-ring …`, `smart-cliff …` für die Gate-Datei.

- [ ] **Step 1: Tests** (Store auf `vanilla_copy()`, Textprojekt wie `tests/world_paint.rs`; Karte = `new_painted_map` neben der Startstadt wie dort, Name „Kantenweg“, Boden Kachel `0000`; `tileset_save` des Pakets):
  - `edge_set_rings_the_plus_without_a_wrong_corner` (**Gate**): das Paket braucht ≥ 14 Kacheln, sonst Fehlschlag „Paket NNNN hat nur n Kacheln; der Satz braucht 14.“; `tileset_edit` mit `changes` (Kacheln 0000 bis 000d: `role: ground`, `kind: 0`, `collision: 0`, damit keine abgeleitete Wasser- oder Graskachel die Zählung stört) und `sets: [{put: {name: "Gate", outside: "0000", places: Place::GATE13 der Reihe nach mit 0001 … 000d}}]` in einem Aufruf; `world_edit` mit `terrain` über `plus_cells()`; Ebene 0 der Welt **gleich dem Kachel-Raster** aus `PLUS_EXPECTED` (Buchstabe → Token des Platzes, `.` → `0000`) über die ganze Karte; `wrong_corners == []`; `bake.ok`; druckt `smart-ring <code> map=<n> package=<n> cells=<n> places=13 wrong=0 nsbmd=<n>`.
  - `cliff_sets_walls_and_collision_by_the_edge_rule` (**Gate**): derselbe Satz, `cliff {height: 2}` auf x 20–25, z 20–24: Höhen, Rand wie Task 3; gebackene Permissions des Map-Assets: 18 Randzellen mit Bit 7, 26 Ringzellen ohne; `mesh` der Welt: 22 Randkanten senkrecht überdeckt; Satz auf `block: lower` umgestellt, `cliff {height: 2}` auf x 4–8, z 20–23: Ring blockiert, Rand frei (Permissions des Map-Assets); BDHC ≤ 0x9000 + 16, Modell ≤ 0xF000; `save`, `build::build` → ROM; druckt `smart-cliff <code> map=<n> walls=<n> upper_blocked=<n> lower_blocked=<n> nsbmd=<n> share=<f> bdhc=<n>`.
- [ ] **Step 2: Kopien** `6-4b-pt`, `6-4b-hgde`, `6-4b-hgus` (je `_DSPRE_contents` mit `.nds` daneben).
- [ ] **Step 3: Lauf** je Kopie nacheinander im Hintergrund: `G4_ROMS_DIR=$R G4_TEST_PROJECT=$K/6-4b-<k>_DSPRE_contents cargo test --release -p g4-kernel --all-features --test world_smart -- --nocapture --test-threads 1 > target/agent/6-4b/gated-<k>.log 2>&1` → PASS; danach je Kopie `--test world_bake` (Form 1 byte-gleich) und `--test vanilla_checks` (0 Fehler).
- [ ] **Step 4: Commit** `test(world): an edge set rings the plus without a wrong corner and a cliff blocks by the edge rule on the three games` (mit Trailer).

### Task 10: Szenarien `pt-klippe`/`hg-klippe`

**Files:**
- Create: `scripts/agent/scenarios/pt-klippe.yaml`, `scripts/agent/scenarios/hg-klippe.yaml`

**Interfaces:**
- Consumes: `pt-gemalt.yaml`/`hg-gemalt.yaml` (6.4a-Fassung), kuratiert `recipe_apply`, `world_edit`, `world_get`, `tileset_get`; über `g4_call` `tileset_save`, `tileset_edit`; g4emu-Schritte `walk`, `walk-until-blocked`, `expect-at`, `shot`.

- [ ] **Step 1: Szenarien** als Kopie von `pt-gemalt.yaml`/`hg-gemalt.yaml` (gleiche Vorbereitung, Rezept, Tileset, Tür, Geschenk; Kopfkommentar: was 6.4b ergänzt) mit drei Zusätzen nach den gemalten Strichen: `g4_call tileset_edit {req: {package: …, changes: [die 13 Kacheln mit role ground, kind 0, collision 0], sets: [{put: {name: Gate, outside: <Boden>, places: {…13 Indizes…}}}]}}` (Indizes als Variablen `--set GATE_TILES=…`, Platz-Reihenfolge `Place::GATE13`); kuratiertes `world_edit` mit `terrain` über ein Rechteck x 2–8, z 12–16 und mit `cliff {height: 2}` auf x 2–8, z 4–8 (westlich von Treppe und Plateau aus `gate_world`, nicht über Wasser oder Gras); `world_get` mit `expect` `bake.ok: true`. Spielschritte nach dem Eintritt (Koordinaten in der gemalten Karte `x + 32`): von der Tür nach Westen bis x 37, dann nach Norden `walk-until-blocked` + `expect-at 37 9` (die Zelle vor dem blockierten Rand z 8), `shot klippe` mit Blick nach Norden auf Wand und Rand. Rechnung der Koordinaten als Kopfkommentar.
- [ ] **Step 2: Lauf** je Spiel auf frischen Kopien `6-4b-szen-pt`/`6-4b-szen-hg`: `uv run g4emu scenario ../agent/scenarios/pt-klippe.yaml --project $K/6-4b-szen-pt_DSPRE_contents --out $W/target/agent/gate-2026-11-07/6-4b/pt-klippe <--set wie pt-gemalt> --set GATE_TILES=<…> > target/agent/6-4b/pt-klippe.log 2>&1` (HeartGold entsprechend) → Rückgabe 0, `"ok": true`, `mcp.jsonl` mit einem Verlaufseintrag je `world_edit`; Blick des Agenten auf `klippe` (Wand an der Kante, Rand oben, Figur unten davor).
- [ ] **Step 3: Regression** `pt-gemalt`/`hg-gemalt` auf frischen Kopien `6-4b-gemalt-pt`/`-hg` unverändert → Rückgabe 0 (Phasen-Gate 6 bleibt).
- [ ] **Step 4: Commit** `test(g4emu): pt-klippe and hg-klippe paint terrain and a cliff through world_edit and walk against the rim` (mit Trailer).

### Task 11: Doku der Formate und Design-System-Nachtrag

**Files:**
- Modify: `docs/formats/tileset.md` (Abschnitt „Kantensätze (6.4b)“, Zeile `edge_sets` der Tabelle, Prüfungen, `tileset_edit` mit `sets`; „reserviert“ entfällt), `docs/formats/world_map.md` (Abschnitt „Smart Drawing (6.4b)“: `terrain`/`cliff` in `world_edit`, Eckfarben, Naht, Klippe mit Kantenregel; „Was später kommt“: 6.4b-Zeile ersetzt durch „später: Wände zu Nachbarkarten, Kanten-Sätze, Automapping“), `docs/formats/terrain.md` (ein Absatz unter „Kollisionsableitung“: `high_edges`, die Regel „untere Zelle“ eines Kantensatzes als Überschreibungen), `design-system/README.md` (Abschnitt „Nachtrag 6.4b“ am Ende), `docs/ROADMAP.md` Zeile 408 („… die Muster mit 6.4b umgesetzt (Branch `phase-6-4b-smart-drawing`, [phase-6-4b-gate.md](phase-6-4b-gate.md))“)
- Create: `design-system/openpencil/NACHTRAG-6-4b.md`, `design-system/openpencil/beispiele/kantensatz.body.js`

- [ ] **Step 1: Formate** wie unter Files; Wortlaut und Zahlen aus dem Code der Tasks 1 bis 8; Beispiele synthetisch (`synthetic_edges`, `weg13`, das Plus als Platz-Raster); Lizenzsatz „Datenmodell nach Tileds Wang-Ecken, Logik eigenständig; kein Code aus Tiled oder PDSMS“.
- [ ] **Step 2: README** „Nachtrag 6.4b“ nach dem Muster „Nachtrag 6.4a“: Quelle (Dateien), Unterwerkzeuge T und K (Tasten, Statusleiste, Vorschau), Abschnitt „Kantensätze“ mit Editor (Plätze-Raster, Varianten-Chips, Außen, Segment, Wand, Stufen, Zähler, gesperrt ohne Datei), Gelände-Muster (fünf Muster, Regel, Legende), Galerie-Blöcke, Vergleichsquelle `target/agent/gate-2026-11-07/6-4b/`; keine neuen Tokens; neue Symbole mit Phosphor-Namen; Abweichungen: Klick statt Ziehen in den Editor, Vorsprung zeigt Pfeil statt Schraffur; die Roadmap-Zeile „Overlay-Muster“ zieht nach.
- [ ] **Step 3: Anleitung** `NACHTRAG-6-4b.md` nach `NACHTRAG-6-4a.md`: Frames aus „Entscheidungen“ (`.fig`-Frames) mit `MODE = 'kantensatz' | 'muster' | 'brett'` und Dunkel-Variante; `beispiele/kantensatz.body.js` gegen `jsx-lib-app.js`, gegen eine Attrappe der Bausteine in allen Modi fehlerfrei durchlaufen lassen; die `.fig` rendern, wenn OpenPencil auf 127.0.0.1:7600 antwortet (Erfahrung „OpenPencil .fig limits“: bei Knotengrenze oder Zeitüberschreitung den Frame neu rendern statt Einzelknoten zu ändern), sonst der Satz „Die Frames von 6.4b werden auf `main` nach dem Merge gerendert, sobald OpenPencil antwortet“ und der Gate-Punkt bleibt offen.
- [ ] **Step 4:** `bash scripts/check-tokens.sh` grün; `cargo test -p studio --all-features table_covers_every_format_file` grün; `bash scripts/check-no-rom-data.sh`.
- [ ] **Step 5: Commit** `docs(6.4b): edge sets, smart drawing in world_edit, cliff rule; design-system addendum 6.4b` (mit Trailer).

### Task 12: Gate 6.4b, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-6-4b-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 6.4b“ am Ende), `docs/ROADMAP.md` (Zeile 6 der Übersicht, „Stand 6.4b“ unter der Tabelle)

- [ ] **Step 1: Gate-Datei** nach `docs/phase-6-4a-gate.md`: Kopf mit Stand und Commit; Gate-Text (Gate 6.4b und Phasen-Gate wörtlich wie in Global Constraints); „So gelesen“ („13 Kacheln“ = der Satz `Place::GATE13`, festes Muster = das Plus, Erwartung als Kachel-Raster über die ganze Karte; „ohne falsche Ecke“ = Raster gleich und `wrong_corners` leer; „Kantenregel“ = obere Zelle als Vorgabe, untere je Satz über Überschreibungen; Muster für alle Karten; Wände zu Nachbarkarten nicht in 6.4b); Legende „Wer“; Namen in der Oberfläche (`[data-edge-sets]`, `[data-edge-set]`, `[data-edge-editor]`, `[data-edge-place]`, `[data-edge-variant]`, `[data-edge-outside]`, `[data-edge-block]`, `[data-edge-wall]`, `[data-cliff-rise]`, `data-terrain-patterns`, Verlaufstexte der Tasks 1 und 6); Vorbereitung (Kopien `6-4b-{pt,hgde,hgus}`, `6-4b-web`, `6-4b-szen-{pt,hg}`, `6-4b-gemalt-{pt,hg}`; Worktree, Ports 1600/1601, Last).
  - **(1) Automatik (Agent):** `cargo deny --locked check licenses advisories`, fmt, clippy, `env -u G4_TEST_PROJECT cargo test --workspace --all-features`, `pnpm test`/`pnpm build`, Stil, Tokens, Tiefe, ROM-Daten, `uv run pytest -m "not integration" -q` in `scripts/g4emu`; gated Task 9 je Kopie, `world_bake` und `vanilla_checks` je Kopie. `scripts/gate.sh`: **offen, Lauf in der Hauptsitzung auf main**.
  - **(2) Szenarien (Agent + Blick):** Task 10 mit Rückgabe, Erwartungen, Bild `klippe`; Regression `pt-gemalt`/`hg-gemalt`.
  - **(3) Browser (Agent):** Task 8 Step 5 mit Raster-Vergleich und Bildern.
  - **(4) Hand und Blick:** Bilder Hell, Dunkel, Kompakt (Muster im Overlay, Kantensatz-Editor); `.fig`-Frames „Gelände malen: Kantensatz-Editor“ (Palette, Ebenen, Kantensatz-Editor), „Gelände-Muster“, Brett; `out.nds` von `pt-klippe` in melonDS (Klippe von oben und unten).
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Karte / Paket | Ring gleich Raster | falsche Ecken | Randkanten mit Wand | blockiert (obere Regel: Rand) | blockiert (untere Regel: Ring) | NSBMD (Anteil) | BDHC | Szenario `*-klippe` | `*-gemalt` weiter | `world_bake` byte-gleich | `vanilla_checks` Fehler | Last |
    |---|---|---|---|---|---|---|---|---|---|---|---|---|---|
    | Platin DE | | ja | 0 | 22 | 18 | 22 | | | | | / 666 | 0 | |
    | HeartGold DE | | ja | 0 | 22 | 18 | 22 | | | | | / 676 | 0 | |
    | HeartGold US | | ja | 0 | 22 | 18 | 22 | | | – | – | / 676 | 0 | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte eintragen; Befunde beheben (eigene Commits `fix(6.4b): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: `agent-gate.md`** Abschnitt „Phase 6.4b“ nach „Phase 6.4a“: Kopien, Worktree, „Wo was ist“ (Attribute aus Step 1, T/K, `__g4SceneProbe.tileToClient`/`strokePending`), Tabelle Punkt / Wer / Wie, die Befehle für (1), (2), (3) mit erwarteten Werten. **Roadmap:** „**Stand 6.4b <Datum>:** Gate vom Agenten bestanden: ein Kantensatz aus 13 Kacheln umrandet das Plus ohne falsche Ecke (Raster gleich auf Platin DE, HeartGold DE und US), der Klippenpinsel setzt Wände an 22 Randkanten und blockiert nach der Kantenregel (obere Zelle; untere je Satz), `pt-klippe`/`hg-klippe` in g4emu, Gelände-Muster im Overlay; MCP weiter 90 kuratierte Werkzeuge; Branch `phase-6-4b-smart-drawing`; [docs/phase-6-4b-gate.md](phase-6-4b-gate.md); beim Nutzer offen: Blick, `.fig`, melonDS, `scripts/gate.sh` auf main“; Zeile 6 der Übersicht um „6.4b Gate vom Agenten bestanden <Datum>“.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' <BASIS>..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count <BASIS>..HEAD`; `git log <BASIS>..HEAD | grep -c '＠'` gleich 0.
- [ ] **Step 5: Commit** `docs(6.4b): gate of smart drawing and patterns, agent-gate section, roadmap` (mit Trailer).

---

## Schnittstellen für die Folgephasen

- **Phase 5 und später:** `world::edge::{EdgeSetJson, Place}` (ein Kanten-Satz-Typ „edge“ käme als eigenes `kind` neben den Ecken-Sätzen), `smart::Seams` (eine Wand an der Naht nutzte dieselbe Nachbarschaft, braucht aber den Eingabe-Hash der Nachbarkarte), `patterns.ts` (weitere Muster je Klasse).
- **TMJ (6.5b):** Kantensätze sind im TMJ bewusst nicht abgebildet; ein Austausch gehörte als `wangsets` (Typ `corner`, Farben innen/außen, `wangid` aus `Place::mask`) in das erste Tileset von `world/tmj.rs`.

## Offen für die Umsetzung

- **Paketgröße:** ob jedes Paket der Startstadt-Karten ≥ 14 Kacheln hat, zeigt Task 9; sonst Satz mit gedrehten Kacheln (vier Kanten aus einer Kachel `r0..r3`) und die Abweichung in die Gate-Datei.
- **Muster-Lesbarkeit im Dunkel-Thema:** der Faktor 0,55 ist eine Annahme; reicht der Kontrast im Blick nicht, wird er hell und dunkel getrennt festgelegt (keine neuen Tokens).
- **Klippe neben Rampe:** eine Rampe am Fuß einer Klippe ergibt Wände mit entarteten Vierecken (6.3a-Regel); ob das im Spiel gut aussieht, zeigt der Blick auf `klippe`.
- **Zwei Matrizen mit derselben Karte:** die Naht folgt der ersten Matrix; ob Nutzer das bei Innenraum-Matrizen anders erwarten, zeigt das Malen.
- **`.fig`:** rendert OpenPencil im Worktree nicht, rendert die Hauptsitzung die Frames nach dem Merge (Erfahrung „Parallele Merges“: `.fig` von main behalten und neu rendern).
