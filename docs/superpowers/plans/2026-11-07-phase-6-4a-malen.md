# Phase 6.4a „Malen: Werkzeuge und Rezept“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Im Welt-Explorer (Tiefe Bauen) malt man eine Welt-Karte des Textprojekts mit dem Werkzeug „Gelände malen“: Unterwerkzeuge Pinsel, Rechteck, Füllen, Radierer, Pipette, Höhe (+/−) und Rampe mit Tiled-Kürzeln, Palette der Kacheln des Tilesets nach Rolle (Bilder aus dem Paket), Ebenen-Liste, Tileset-Editor (Rolle, `kind`, Kollision, Schrittgeräusch, Wand je Kachel), Budget live im Inspector; Ziehen malt, ein Strich (Rechteck, Füllen) ist **ein** `world_edit` und **ein** Verlaufseintrag; das Rezept „Neue Karte malen“ (`new_painted_map`) legt eine gemalte Karte in eine Matrixzelle (Entwurf gestrichelt, Plakette „Nicht rückgängig“ für den Member, der Rest ein Schritt), schaltet Draufsicht und Werkzeug ein und gibt den gesperrten Schritt „Gelände anpassen“ frei; über MCP werden `world_get`, `world_edit`, `tileset_get` kuratierte Werkzeuge und `recipe_*` kennt `new_painted_map`; die Szenarien `pt-gemalt`/`hg-gemalt` laufen über `recipe_apply` + `world_edit`.

**Architecture:** Der Kernel bekommt nur das Rezept (`recipes/new_painted_map.rs`, eine Transaktion aus Matrix-Änderung und `Tx::edit_world`, die nach 6.3b im selben Schritt backt) und einen Ebenen-Befehl `world_layers` (an `world/edit.rs` angehängt). Alles Werkzeughafte rechnet der Client: `world/paint/ops.ts` macht aus Werkzeug, Zellen und dem zuletzt gelesenen Weltdokument (`world_get`) die `WorldChange`s (Bresenham-Linie, Rechteck, Füllen in der 4er-Nachbarschaft gleicher Kachel, Höhe je Zelle ±1, Rampe, Radierer, Pipette), `world/scene/strokes.ts` sammelt Zellen aus Pointer-Ereignissen der Leinwand und zeichnet optimistisch (`strokePreview.ts`), `world/paint/session.ts` schickt beim Loslassen genau einen `world_edit` (seriell in einer Warteschlange) und lädt danach Netz, Höhen und Weltdokument neu. Palette, Ebenen, Tileset-Editor und Budget stehen im Inspector der gewählten Zelle (kein neues Dock-Panel), die Unterwerkzeuge als zweite Gruppe der schwebenden Werkzeugleiste.

**Tech Stack:** Rust (g4-kernel: `recipes`, `world::{edit, doc, grid, tileset, places}`; nach 6.3b `world::bake`), serde, Tauri-Befehle und Brücke, MCP-Katalog (`apps/studio/src-tauri/src/mcp/catalog.rs`), React 19, three.js (vorhandene Orthokamera, `OrbitControls`, `InstancedMesh`), vitest, Python/uv für `scripts/g4emu`; keine neue Abhängigkeit.

**Spec:** [2026-11-05-phase-6-kartenerstellung-design.md](../specs/2026-11-05-phase-6-kartenerstellung-design.md) §1 (harte Grenzen), §3.1, §3.2, §3.6, §4 Zeile 6.4a und „Phasen-Gate 6“ (wörtlich unten), §5, §6, §7, §8; [entscheidungen.md](../../research/2026-11-05-phase-6/entscheidungen.md) (Block „Malen, Weltformat, Import, Tiled“: Malmodell, Draufsicht und Vorschau, Tiled, Store, Nur im Textprojekt, Oberfläche, Malen über Kartengrenzen; „Phasenschnitt“ Zeile 6.4a; „Nicht in Phase 6“ wörtlich); [Gutachten C](../../research/2026-11-05-phase-6/gutachten-c-malen-import.md) §3.5 (Draufsicht, optimistisches Zeichnen), §4.2–4.3 (Tiled als Vorbild, Kürzel), §7 (Oberfläche, MCP, Design-System, Wiederverwendung); Vorgänger-Pläne [6.3a](2026-11-05-phase-6-3a-weltformat.md) (Weltformat, `world_edit`, Szene) und [6.3b](2026-11-06-phase-6-3b-backen-build.md) (Form, Backen, Interfaces).

Stand 07.11.2026: Basis ist `main` **nach** dem Merge von 6.3b und 6.5a (Commit trägt der Umsetzer hier ein: `7f01683b`; Teil 1 auf main vor 6.3b, siehe Voraussetzungen). Arbeit im Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-6-4a`, Branch `phase-6-4a-malen` (`git worktree add .claude/worktrees/phase-6-4a -b phase-6-4a-malen <BASIS>`). Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`, Emulator `scripts/g4emu`.

## Voraussetzungen (auf `7f01683b`, vor Task 1 mit `grep` prüfen)

Spalte „Stand“: **main** = beim Schreiben auf main `e4f248b9` gelesen; **6.3b** = aus den Interfaces-Blöcken des 6.3b-Plans (beim Schreiben im Worktree `phase-6-3b`, Tasks 1–3 von 13 fertig), auf `<BASIS>` zuerst bestätigen.

| Schnittstelle | erwartet | Stand | gebraucht in |
|---|---|---|---|
| `world::edit::{world_create, world_edit, WorldCreateRequest, WorldEditRequest {map, label?, changes}, WorldChange {layer?, cells, tile?, height?, ramp?, permission?}, WorldReport, WORLD_NEEDS_TEXT_PROJECT, NOTHING_TO_CHANGE, not_a_world_map}`; Label vorgegeben „Welt geändert: Karte 0123 (37 Zellen)“, „(1 Zelle)“ | `world/edit.rs` | main | 1, 2, 4 |
| `WorldReport.bake: Option<WorldBakeReport>`, `Store::world_bake_report`, `world_get` → `WorldGetDto.bake`, `WorldBakeReport {ok, model_bytes, model_share, model_kept, textures, vram_bytes, bdhc_bytes, bdhc_payload, messages, …}` | 6.3b Task 2, 4, 8 | 6.3b | 1, 3, 7 |
| `Tx::edit_world` backt im selben Changeset; das Map-Asset einer Welt-Karte ist ihr Bake (`get_map_heights`, `get_map_budget` folgen der Welt) | 6.3b Task 4 | 6.3b | 1, 6, 7 |
| `world_adopt {req: {map}}`, `world_to_binary {map}`, `tileset_edit {req: {package, changes: [TileChange]}}` (ein Dateischritt „Tileset 0006 geändert (2 Kacheln)“, verlangt die Datei: „Erst `tileset_save` für Paket 0006.“), `TilesetEditReport` | 6.3b Task 7, 8 | 6.3b | 8 |
| `WORLD_VERSION = 2`, `maps.world` 2, Textprojekt-Schema 4 | 6.3b Task 3 | 6.3b | 1, 10 |
| Frontend `api.world.ts`: `WorldBakeReport`, `worldAdopt`, `worldToBinary`, `worldBake`; `WorldSourceSection {map, world, textProject}` mit `[data-world-source]`, `[data-world-adopt]`, `[data-world-bake]`; `mapModel` fällt ohne Flächen auf das GLB zurück | 6.3b Task 10 | 6.3b | 7, 8 |
| `scripts/agent/scenarios/{pt,hg}-gemalt.yaml` (`adopt_text_project`, `create_area` 1 × 1 mit neuem Header, Begegnungsdatei Rate 100, Geschenk, Tür, `world_create`, `tileset_save`, `tileset_edit`, `world_edit` mit `gate_world`, `world_bake`, Spielschritte) und Kachel-Indizes im YAML; `--set NEUER_HEADER=593`/`540` | 6.3b Task 11 | 6.3b | 11 |
| `world::doc::{WorldJson {version, tileset, layers, pieces, freeform, permissions}, LayerJson {name, role, tiles, heights?, ramps?}, LayerRole {Ground, Overlay}, WorldJson::blank(tileset, fill, height), fixtures::{one_tile_world, busy_world, gate_world}}` | `world/doc.rs` (`gate_world` ab 6.3b) | main / 6.3b | 1, 2, 3 |
| `world::grid::{parse_tile, tile_token, parse_height, parse_ramp, ramp_token, TileRef {tile, rotate, flip}}`, `world::tileset::{tile_outside, Tileset, TileJson, TileRole}`, `Store::{tileset, world}` | | main | 1, 2 |
| `world::places::{packs_of_map, headers_of_map}` (Header einer Karte mit Paket) | `world/places.rs` | main | 1 |
| `recipes::{list, is_irreversible, plan, apply}`, `types::{RecipeId, Draft {cells, tiles, events, messages, notes, irreversible}, DraftCell {matrix, x, y, source_map}, Outcome, Created, check_name, check_writable, cell_index, late, refused, next_steps}`; `next_steps` sperrt `adjust_terrain` mit „Kommt mit Phase 6“; Test `next_steps_lock_what_is_not_there_yet` | `recipes.rs`, `recipes/types.rs` | main | 1 |
| `recipes/extend.rs` (Muster: Zelle leer oder außerhalb, Matrix wächst über `resize_matrix`, `duplicate_map_with`, eine Transaktion, `next_steps(&[("adjust_terrain", …)])`) | | main | 1 |
| `Store::{duplicate_map_with, transaction, change_generation, changes_since, matrix, header_table}`, `Tx::{edit_matrix, edit_world}` | | main | 1 |
| MCP `catalog.rs`: 87 Werkzeuge (`the_curated_tool_count_stays_within_what_an_agent_reads_at_once`: `30..=90` und `== 87`), `the_curated_set_is_the_decided_one` (Liste der lesenden, Menge der schreibenden), `recipe_args` (Beschreibung nennt jede Eingabe), Test „jedes immer unumkehrbare Rezept steht in `recipe_apply`“; `access.rs` `COMMAND_KINDS` mit `world_*`, `tileset_*`; `dispatch.rs` `COMMANDS` | | main | 3 |
| Szene: `SceneHost.{setTopDown, controls, canvas, loaded, ground}`, `camera.ts::topDownFrame`, `picking.ts::Picker.pickTile`, `canvasClicks.ts::attachCanvasClicks` (nur Klick), `draft.ts::setDraft`, `heightLayer.ts::{showHeightLayer, setHeightsShown}` (Schicht „Höhen“ von 6.2: sieben cividis-Stufen `szene/hoehe/0..6`, Isolinien), `sceneProbe.ts::tileToClient`, `useRefreshQueues.ts::{useMapRefresh, useMapModelRefresh}`, `worldMesh.ts::{meshGeometries (je Ebene), placeholderColor, mapModel}` | | main | 5, 6 |
| Werkbank: `tools.ts::{TOOLS, toolForKey}` (V B F T A S G), `app/useShortcuts.ts` (Buchstaben ohne Modifikator), `ui/frame/FloatingToolbar.tsx`, `SceneOverlays.tsx`, `app/ToolHints.tsx`, `inspector/{MatrixCellEditor.tsx, MapBudgetSection.tsx}`, `ExtendAreaDialog.tsx` + `extend.ts::extendDefaults`, `WorldMapPanel.tsx` (das „+“), `wizards/useDraftPlan.ts`, `wizards/WizardResult.tsx` (`adjust_terrain: {icon: "paint"}`), `recipes.ts` (`NEXT`, `KNOWN`, `lockReason`) | | main | 5, 7, 9 |
| `preview::standard_map` (Startstadt: Header `T01`/`T20` in Matrix 0, erste Zelle) | `preview.rs:327` | main | 10 |

Fehlt eine Zeile oder weicht sie ab (vor allem die mit Stand 6.3b), zuerst die Abweichung in diese Tabelle schreiben und den betroffenen Task anpassen, dann beginnen.

Teil 1 (Tasks 2–6) auf 7f01683b; Teil 2 nach dem Merge von main mit 6.3b.

**6.3b-Zeilen bestätigt nach dem Merge von main `53210d31` (Merge-Commit `f22ea326`):** `WorldReport.bake: Option<WorldBakeReport>`, `Store::world_bake_report`, `WorldBakeReport` (`world/bake.rs`), `test_support::world_ready()`, `fixtures::gate_world(tileset, ground, grass, water)`, `WORLD_VERSION = 2`, `world_adopt` (`world/adopt.rs`), `world_to_binary`, `tileset_edit` (`world/tileset.rs`), Frontend `WorldBakeReport`, `worldAdopt`, `worldToBinary`, `worldBake`, `WorldSourceSection`, Szenarien `{pt,hg}-gemalt.yaml` mit `NEUER_HEADER` sind da. Beim Merge angepasst: `world_layers` übergibt den neuen Ablehnungstext an `refuse_locked_map` und meldet den Bake des Schritts wie `world_edit`; TS `tilesetEdit` mit `TilesetEditReport`.

**Geprüft auf `7f01683b` (07.11.2026, grep), Abweichungen der main-Zeilen:**

- `world::doc::fixtures::gate_world` fehlt auf main (kommt mit 6.3b, wie in der Zeile vermerkt); `one_tile_world`, `checkerboard_world`, `busy_world` sind da. `WorldJson::blank(tileset, fill: TileRef, height: i8)` nimmt eine `TileRef`, keinen Token (Task 1 parst `fill_tile` vorher).
- `world.rs` re-exportiert aus `edit` nur `WORLD_NEEDS_TEXT_PROJECT`; Befehle greifen über `world::edit::…` zu. `headers_of_map` steht in `world/places.rs`, ist aber nicht re-exportiert (`world::places::headers_of_map`); re-exportiert sind `maps_of_packages`, `packs_of_map`.
- `test_support::world_ready()` gibt es auf main nicht (6.3b). Teil 1 nutzt die vorhandenen Fixtures: Kernel `world/edit.rs` `fixture()` auf `TextFixture::exported()`, Befehle `commands/world_map.rs` mit `TextFixture::exported()` + `world_create` (wie `world_commands_over_dispatch`).
- `SceneHost.setTopDown(on, focus: CellRect, matrix: MatrixDto)` hat Fokus und Matrix als Parameter (nicht nur `on`); `SceneHost.topDown` ist ein Getter.
- `WorldReport` auf main: `{map, tileset, cells, change}` ohne `bake` (kommt mit 6.3b); Task 4 führt `bake` im TS-Typ als optional (`bake?: WorldBakeReport | null`) erst in Teil 2, in Teil 1 ohne das Feld.
- Werkzeug-Kürzel auf main: V Auswählen, B Gelände malen, F Figur, T Tür, A Auslöser, S Schild, G Gebäude (wie erwartet; B und G sind im Malmodus die Unterwerkzeuge Pinsel und Füllen).
- Alles Übrige der main-Zeilen (Edit-Typen und Konstanten, `grid`, `tileset`, `Store`/`Tx`-Methoden, `recipes`, `next_steps` mit „Kommt mit Phase 6“, Katalog 87 mit `30..=90`, `access.rs`/`dispatch.rs` mit `world_*`/`tileset_*`, Szene, Werkbank, `preview::standard_map`) wie erwartet.

## Parallele Teilphase 6.5b und Anhängepunkte

6.5b (Freiform glTF backen, Teilmodelle als Beifiles, Form 2 von `world_adopt`, TMJ-Export/-Import) läuft gleichzeitig in einem eigenen Worktree mit eigenen Ports. Ihr gehören `world/bake.rs`, `world/adopt.rs`, `world/tileset.rs` (6.4a liest dort nur), `model_import/` und die TMJ-Dateien. Gemeinsame Dateien:

| Datei | 6.4a tut | Regel |
|---|---|---|
| `crates/g4-kernel/src/recipes/new_painted_map.rs` | neu | 6.5b fasst `recipes/` nicht an |
| `crates/g4-kernel/src/recipes.rs`, `recipes/types.rs` | Variante `NewPaintedMap`, Arme, `Draft.painted`, `adjust_terrain` frei | nur 6.4a |
| `crates/g4-kernel/src/world/edit.rs` | `world_layers` mit Typen und Tests **am Ende** angehängt; nichts Bestehendes geändert | nur anhängen |
| `crates/g4-kernel/src/world.rs` | `pub use edit::{world_layers, …}` angehängt | nur anhängen |
| `commands/world_map.rs`, `commands/dispatch.rs`, `mcp/access.rs`, `mcp/catalog.rs`, `src-tauri/src/lib.rs` (`generate_handler!`) | `world_layers` und drei kuratierte Werkzeuge, jeweils am Ende ihrer Liste | nur anhängen; kommt 6.5b mit einem eigenen kuratierten Werkzeug, zählt der zweite Merge den Katalog neu (90 + n) und hebt die Obergrenze des Zähltests mit Begründung |
| `docs/mcp.md`, `docs/agent-gate.md`, `docs/ROADMAP.md`, `design-system/README.md` | Abschnitte am Ende | nur anhängen |
| `scripts/agent/scenarios/{pt,hg}-gemalt.yaml` | geändert (Task 11) | 6.5b fasst sie nicht an |
| `apps/studio/src/api.world.ts`, `apps/studio/src/api.ts` | Typen und Aufrufe am Ende | nur anhängen |

`scripts/gate.sh` bleibt unverändert (die neuen Tests laufen in den vorhandenen Schritten mit).

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Wie der Strich an die Brücke geht | `world/scene/strokes.ts` hört auf `pointerdown`/`pointermove`/`pointerup`/`pointercancel` der Leinwand (Pointer-Capture), solange das Werkzeug „Gelände malen“ auf der **gewählten** Welt-Karte steht; Zellen kommen aus `pickTile` (kartenlokal), zwischen zwei Ereignissen per Bresenham ergänzt, jede Zelle einmal je Strich; nur Zellen der Karte, auf der der Strich begann. Beim Loslassen macht `ops.ts::strokeRequest` daraus **einen** `WorldEditRequest` mit eigenem `label`, `session.ts` schickt ihn über eine serielle Warteschlange (ein Strich wartet auf den vorigen) | Spezifikation §3.6 „ein Strich = ein Undo-Schritt“; 6.3a-Entscheidung „die Werkzeug-Ops rechnet 6.4a auf Zellen herunter“; die Warteschlange hält die Reihenfolge der Verlaufseinträge |
| Ziel des Strichs | Die gewählte Zelle (Zellrahmen) bestimmt die Karte; ein `pointerdown` auf einer anderen Karte wählt deren Zelle und malt nicht | Palette, Ebenen und Tileset hängen an einer Karte; „Malen über Kartengrenzen: je Karte“ (Entscheidungen) |
| Verlaufstexte | `label` je Werkzeug: „Gemalt: Karte 0123 (37 Zellen)“, „Rechteck gemalt: …“, „Gefüllt: …“, „Radiert: …“, „Höhe geändert: …“, „Rampe gesetzt: …“ („(1 Zelle)“); MCP ohne `label` behält „Welt geändert: Karte 0123 (N Zellen)“ | Gutachten C §7.1 „Boden gemalt (37 Kacheln)“; der Verlauf sagt, welches Werkzeug es war |
| Unveränderte Zellen | `strokeRequest` lässt Zellen weg, deren Token schon das Ziel ist; bleibt keine, geht **kein** Aufruf (kein `NOTHING_TO_CHANGE`, kein Fehler) | ein Klick auf gleiche Kachel ist kein Verlaufseintrag |
| Optimistisches Zeichnen | Während des Strichs zeigt `strokePreview.ts` je berührter Zelle eine Platte knapp über dem Boden (`HEIGHT_LAYER_Y` + 0,01): Pinsel und Rechteck mit der Textur der Kachel (aus dem Palettenspeicher), Radierer schraffiert in `szene/gelaende/unbekannt`, Höhe und Rampe in der Farbe der Zielstufe (`heightColor`); Rechteck zeigt dazu den gestrichelten Rahmen von `draft.ts`; nach der Antwort Netz (`mapModel`), Höhen (`get_map` über die vorhandene Karten-Refresh-Schlange) und Weltdokument (`world_get`) neu, dann Vorschau weg; bei Ablehnung Vorschau weg und Fehlertext | Gutachten C §3.5 („Ein Pinselstrich zeichnet sofort optimistisch, das Netz kommt nach dem Loslassen“) |
| Pinselgröße, Stempel, Zufall | Pinselgröße 1; keine Auswahl als Stempel, kein Zufall, keine Formen außer Rechteck | YAGNI (Auftrag); Zufall und Terrain-Pinsel 6.4b |
| Füllen | Client: 4er-Nachbarschaft gleicher Kachel (Token mit Drehung und Spiegelung) **in der aktiven Ebene** der Karte, ab der geklickten Zelle, aus dem zuletzt gelesenen Weltdokument | Auftrag; die Karte hat 1 024 Zellen, die Rechnung ist billig |
| Pipette | Klick: Kachel der aktiven Ebene, ist sie leer (`.`), die der obersten Ebene mit Kachel; wird die aktive Kachel (mit Drehung), die Ebene bleibt; danach zurück zum Pinsel (wie Tiled); Rechtsklick ist in jedem Mal-Unterwerkzeug die Pipette | Tiled: Rechtsklick nimmt die Kachel auf |
| Höhe (+/−) | Klick oder Ziehen: jede berührte Zelle einmal je Strich auf Höhe-zu-Beginn + 1, mit ⇧ − 1; eine Zelle an −16 bzw. 62 fällt weg, die Statusleiste sagt „N Zellen schon an der Grenze (−16 bis 62).“; Änderungen gleicher Zielhöhe sind ein `WorldChange`, alle zusammen ein Aufruf; immer Ebene 0 | Höhen gibt es nur auf der Bodenebene (6.3a `GROUND_ONLY`); ±1 Stufe ist die halbe Kachel, die Vanilla-Treppen brauchen |
| Rampe | Werkzeugoption Richtung N/O/S/W (Bergauf-Richtung wie `world::grid::Ramp`) und Stufen 1 oder 2; Taste Z dreht die Richtung im Uhrzeigersinn, 1/2 wählt die Stufen; Klick oder Ziehen setzt die Rampe auf jede berührte Zelle, ⇧ entfernt sie (`.`); Höhen setzt das Werkzeug nicht | eine Rampe ist ein Zellwert wie in der Datei; die Treppe aus `gate_world` (e1 auf h 0 und h 1) entsteht mit Höhe + Rampe in vier Strichen |
| Kürzel (Tiled) | Solange „Gelände malen“ auf einer Welt-Karte steht, gehen diese Tasten an die Unterwerkzeuge, vor `toolForKey`: **B** Pinsel (Tiled Stempel), **P** Rechteck (Tiled Formfüllung), **G** Füllen (Tiled Füllen; „Gebäude“ dann nur über die Taste der Leiste), **E** Radierer, **I** Pipette, **H** Höhe, **R** Rampe, **Z** dreht (Kachel `r1..r3` bzw. Rampenrichtung), **X** spiegelt (Kachel `f`), **1**/**2** Stufen der Rampe; Esc führt zum Pinsel, ein zweites Esc zu „Auswählen“ | Gutachten C §4.3 („Stempel B, Füllen G, Radierer E“); R (Tiled: Auswahl) ist frei, weil 6.4a keine Auswahl hat |
| Maus | Während eines Mal-Unterwerkzeugs malt die linke Taste; `SceneHost.setPaintPointer(true)` legt in `OrbitControls` LEFT auf keine Aktion, MIDDLE auf Verschieben, RIGHT bleibt Pipette (kein Kontextmenü); Drehen der Schrägsicht mit ⌥ + Ziehen | sonst dreht oder verschiebt jeder Strich die Kamera |
| Draufsicht | **Nicht** automatisch beim Wählen des Werkzeugs; das Rezept „Neue Karte malen“ und der Schritt „Gelände anpassen“ schalten sie ein (`setTopDown(true)`), danach ist sie die gemerkte Wahl des Nutzers. Malen geht in beiden Kameras | Spezifikation §3.6 / C §7.1 („schaltet Draufsicht und Werkzeug ein“ beim Rezept); ein Werkzeugwechsel soll die Kamera nicht umstellen |
| Höhen-Overlay | 6.2 hat die Schicht „Höhen“ (cividis `szene/hoehe/0..6`, Isolinien, `setHeightsShown`) gebaut; 6.4a ergänzt nur: (a) die Unterwerkzeuge Höhe und Rampe schalten „Höhen“ ein (die vorhandene Ausschlussregel schaltet „Gelände“ aus), (b) nach jedem Strich wird die Schicht mit den neuen Höhen der Karte neu gebaut (`showHeightLayer` nach dem Karten-Refresh) | was da ist, bleibt; Muster-Shader 6.4b |
| Palette | Kacheln von `tileset_get` (Datei oder Ableitung), gruppiert nach Rolle in der Reihenfolge Boden, Gras, Wasser, Wand, Rampe, Teilmodell (`[data-palette-role]`), je Kachel eine Taste 32 × 32 (`[data-palette-tile="0041"]`, `aria-pressed`), Bild = die **ganze** Textur aus `get_tileset_texture` (RGBA → Canvas → `ImageBitmap`, nächster Nachbar, je Paket und Textur zwischengespeichert, derselbe Speicher füttert die Strichvorschau); ohne Bild (`null`: synthetisches Tileset, Format ohne Dekoder) die Platzhalterfarbe der Rolle (`placeholderColor`) mit dem Texturnamen im Tooltip; Tooltip „Kachel 0041 · gras_a · 32 × 32 Texel · Gras“ (Tüfteln zusätzlich „kind 0x02, Kollision 0x00“) | eine Kachel ist ein wiederholbares Material mit 16 Texeln je Zelle (C §2.1); die ganze Textur erkennt man besser als ein 16-Texel-Ausschnitt |
| Format-5-Texturen (Altlast 6.3b) | **nicht in 6.4a**: der Platzhalter reicht; der gated Test zählt je Spiel Kacheln mit und ohne Bild (`palette … placeholder=<n>`); ist `placeholder` auf einem Vanilla-Kartenpaket > 0, wird ein Dekoder eigener Posten | Vanilla-Kartenpakete nutzen „nie 4 × 4“ (Entscheidungen, Messwerte); der Dekoder gehörte in `world/tileset.rs` (6.5b-Datei) |
| Ebenen | Liste im Inspector (`[data-layer-row="<i>"]`): Name, Rolle („Boden“/„Überlagerung“), aktive Ebene (Radio), Auge (Sichtbarkeit nur in der Szene, über die Gruppen von `meshGeometries` je Ebene, nicht gespeichert); „Ebene hinzufügen“ (Name, Vorgabe „Ebene <n>“) und „Entfernen“ (nur Überlagerungen) über den neuen Kernel-Befehl `world_layers {req: {map, op: {add: {name}} \| {remove: {layer}}}}`, je ein Undo-Schritt „Ebene „Büsche“ angelegt: Karte 0123“ / „Ebene 2 „Büsche“ entfernt: Karte 0123“; ablehnend: „Die Bodenebene (0) bleibt.“, „Eine Ebene braucht einen Namen.“, „Ebene „Büsche“ gibt es schon.“ | `world_edit` kann keine Ebene anlegen (6.3a: Änderungen nur auf vorhandenen Ebenen); Umbenennen und Umordnen sind YAGNI |
| Wo Palette und Ebenen stehen | Im **Inspector der gewählten Zelle** (`MatrixCellEditor`), solange das Werkzeug „Gelände malen“ aktiv und die Karte eine Welt-Karte ist: Abschnitte „Palette“, „Ebenen“, „Kachel“ (Tileset-Editor) vor „Quelle“ (6.3b) und „Budget“; kein neues Dock-Panel | Inspector ist die Hand; ein Panel änderte das Layout `g4.layout.v11` mit Migration |
| Budget live | Keine neue Zahlenquelle: das Map-Asset einer Welt-Karte ist ihr Bake (6.3b), `MapBudgetSection` liest `get_map_budget` nach jeder Revision (jeder Strich ist eine); neu ist der Satz unter „Modell“, wenn `world_get.bake.ok` falsch ist: „Nicht gebacken: <message der ersten Fehlermeldung> Das Spiel bekommt das letzte Modell.“ (`[data-budget-refused]`) | eine Rechnung, die der Kern schon macht; Gate „Budget live im Inspector“ prüft, dass die Zahl nach jedem Strich folgt |
| Binäre Karte | „Gelände malen“ bleibt Kollisionsmalen per Klick wie heute; die Werkzeugleiste zeigt keine Unterwerkzeuge, der Inspector oben die Notiz „Boden malen: Karte als Quelle übernehmen“ (`[data-paint-binary]`) mit dem Knopf „Karte als Quelle übernehmen“ (→ `world_adopt`, 6.3b; im DSPRE-Layout deaktiviert mit `WORLD_NEEDS_TEXT_PROJECT`) | Gutachten C §7.1 wörtlich |
| Tileset-Editor | Abschnitt „Kachel“ für die gewählte Palettenkachel: „Rolle“ (`Select` der sechs Rollen), „Gelände“ (`kind`, Bauen: Name aus `tile_behaviors*.yaml` wie das Gelände-Overlay, Tüfteln hex), „Blockiert“ (`Checkbox`, Bit 7 von `collision`), „Schrittgeräusch“ (nur HGSS, 0 bis 127), „Wand“ (Kachelnummer oder leer); jede bestätigte Änderung eines Felds ist **ein** `tileset_edit` (ein Dateischritt). Ohne Datei (`stored: false`) sind die Felder gesperrt und darüber steht „Tileset im Projekt anlegen“ (→ `tileset_save`, ein Dateischritt) | 6.3b: `tileset_edit` verlangt die Datei; eine Änderung, ein Verlaufseintrag |
| Dateischritt an `HEADER_TABLE`, VRAM-Doppelzählung (Altlasten 6.3b) | **nicht in 6.4a**: `tileset_save`/`tileset_edit` bleiben am bisherigen Asset verankert (Undo funktioniert, 6.3a/6.3b getestet); das Budget einer Welt-Karte kommt aus dem Bake (`area_vram`), `terrain::vram_bytes` bleibt für binäre Karten | beide Änderungen lägen in `world/tileset.rs` bzw. `terrain/`, außerhalb der Kernel-Grenze von 6.4a neben 6.5b; nach 6.4b neu bewerten |
| Rezept: Zelle | `new_painted_map {matrix?, x, y, name, tileset?, fill_tile?, height?, header?, source_map?}`; `matrix` fehlt = 0 (die Weltkarte); die Zelle muss leer sein (`MATRIX_EMPTY`) oder außerhalb liegen, dann wächst die Matrix wie bei „Gebiet erweitern“ (höchstens 255 × 255) | Gutachten C §7.1 („Zelle der Matrix (frei oder neben der Matrix wie „Gebiet erweitern“)“) |
| Rezept: Gebietsdaten | Nachbar = erste Zelle mit Karte in der Reihenfolge N, W, O, S. Matrix mit Header-Spalte: Header der Zelle ist `header` oder der des Nachbarn; Matrix ohne Header-Spalte: `header` lehnt ab („Weltkarte 0003 hat keine Header-Spalte; die Karte gehört zum Header der Weltkarte.“). Paket = `tileset`, sonst das Paket des gewählten Headers aus `headers_of_map(Nachbarkarte)`, sonst „Kein Header zeigt eine Nachbarkarte; Tileset angeben (`tileset`).“ | „Gebietsdaten des Nachbarn oder gewählt“ (Spezifikation §3.6); eigene Gebietsdaten anlegen steht in „Nicht in Phase 6“ |
| Rezept: Member | Der neue Member ist eine Kopie der Nachbarkarte **ohne Gebäude** (`duplicate_map_with(base, true)`, unumkehrbar, Plakette), dann **ein** Schritt: Matrix (Wachsen, Zelle, Header) und `Tx::edit_world` mit `WorldJson::blank(Paket, fill_tile oder „0000“, height oder 0)`; 6.3b backt im selben Schritt Modell, BDHC und Permissions über die Kopie | Muster `extend_area`; die Kopie trägt den HGSS-BGS-Block und `buildings_tail`, die ein leerer Member erst erfinden müsste |
| Rezept: Kopie einer Welt-Karte | `source_map` (Welt-Karte) nimmt deren Welt statt der leeren und deren Member als Basis; mit `tileset`, `fill_tile` oder `height` zusammen lehnt es ab („Mit `source_map` kommt die Welt aus der Karte; `tileset`, `fill_tile` und `height` weglassen.“), eine binäre `source_map` ebenso („Karte 0123 ist keine Welt-Karte; eine binäre Karte kopiert „Gebiet erweitern“.“); ohne Nachbar und ohne `source_map`: „Neben Zelle (4, 27) liegt keine Karte; `source_map` nennen.“ | „leere oder kopierte Karte“ (Spezifikation §3.6); 6.3b „Nicht in 6.3b“: „das Rezept von 6.4a übernimmt die Welt“ |
| Rezept: `name` | Pflicht (`check_name`), benennt den Verlaufseintrag „Neue Karte gemalt: „Waldweg“ (Karte 0667)“, die Notiz des Entwurfs und `report.name`; gespeichert wird er nicht | das Projekt kennt keinen Kartennamen (Namen tragen Header und Orte, die „Gebiet anlegen“ vergibt) |
| Rezept: Entwurf | `Draft.cells = [{matrix, x, y, source_map: base}]` (gestrichelt über `draft.ts`, `[data-draft]`), Notizen („Neue Karte „Waldweg“ in Zelle x 4, y 27 der Weltkarte 0000.“, „Gebietsdaten von Header 411.“, „Boden aus Tileset 0006, Kachel 0000, Höhe 0.“, „Die Weltkarte wächst auf 31 × 30 Zellen.“), `irreversible = ["Karte 0667 wird angelegt"]`, neu `Draft.painted = {tileset, fill_tile, height, header}` (aufgelöste Werte für den Dialog) | der Dialog braucht das Paket für die Bodenkachel-Wahl |
| Rezept: unumkehrbar | `list()` führt `new_painted_map` als immer unumkehrbar, `is_irreversible` → „Karte anlegen“; kopflos `--allow-irreversible`, sonst `needs_confirmation` | wie `empty_region`, `new_trainer_here` (C §7.2: „unumkehrbar wie `create_area`“) |
| Rezept: nach dem Anwenden | `next = [("adjust_terrain", "Gelände anpassen")]`, **verfügbar** (`next_steps` kennt `adjust_terrain` als Schritt der App); die App wählt die neue Zelle, schaltet Draufsicht, Werkzeug „Gelände malen“ und Pinsel ein; `extend_area` behält seinen Schritt, der jetzt ebenfalls frei ist (binäre Kopie: Kollisionsmalen mit dem Hinweis) | Spezifikation §3.6 „der gesperrte Schritt `adjust_terrain` wird frei“ |
| Rezept in der Oberfläche | Neuer Dialog `NewPaintedMapDialog` (Felder „Name“, „Boden“ als kleine Palette des Pakets aus `Draft.painted.tileset`, „Höhe“; Tüfteln dazu „Tileset“ und „Header“), erreicht über das „+“ der Weltkarte: `ExtendAreaDialog` bekommt im Textprojekt die Taste „Stattdessen neu malen“ (`[data-paint-instead]`), die ihn mit derselben Zelle ersetzt; Entwurf über `useDraftPlan("new_painted_map", …)` | ein „+“, zwei Wege; kein neues Menü |
| MCP | Kuratiert: `world_get {map}` (lesen), `world_edit {req}` (schreiben), `tileset_get {package}` (lesen); Katalog 87 → 90 (Obergrenze 90 bleibt); `world_layers`, `world_create`, `tileset_save`, `tileset_edit`, `world_adopt` weiter über `g4_call` | Spezifikation §3.6 nennt genau die drei |
| Szenarien | `pt-gemalt`/`hg-gemalt` (6.3b Task 11) werden umgestellt: statt `g4_call world_create` und Zelllisten jetzt `recipe_apply new_painted_map {matrix: ${area.matrix}, x: 1, y: 0, …}` (die 1 × 1-Matrix von `create_area` wächst nach Osten), `world_edit` (kuratiert) mit Strichen wie die Oberfläche sie schickt (Pinsel-, Rechteck-, Füll-, Höhen-, Rampen-Aufrufe je einer), die Tür und das Geschenk per `update_event` um 32 Kacheln nach Osten in die gemalte Karte; Spielschritte von 6.3b mit `x + 32`. Für das Phasen-Gate über die Oberfläche: `pt-gemalt-buehne`/`hg-gemalt-buehne` (alles bis vor das Rezept, ohne Build) und `pt-gemalt-ui`/`hg-gemalt-ui` (`edits: []`, nur `save`, `build_rom`, `play`) | Gate „dasselbe über MCP (`recipe_apply` + `world_edit`)“; Muster „Bühne“ aus dem Phasen-Gate 3d (`pt-geschichte-gate-buehne`) |
| Ausgangsstand für `git status` | Der Stand **nach** dem Anwenden des Rezepts, gespeichert und committet („Karte angelegt“); ⌘Z über alle Striche und Speichern lassen `git status` leer. Das Rezept selbst steht vor der Marke, weil sein Member hinter der Undo-Grenze liegt | wie R7 im Gate 3d.7 (`new_trainer_here`): mit einem unumkehrbaren Schritt in der Kette wird `git status` nie leer |
| Zeitgrenze | Strich über die Befehlsschicht: `world_edit` (37 Zellen auf `busy_world`) + `world_get` + `get_world_mesh` über `dispatch`, Median aus 10 nach einem Aufwärmlauf, mit `G4_TIMING_STRICT=1` ≤ 300 ms im Debug-Build; gedruckt `world-stroke-bridge median_ms=<…>` | 6.3b druckte die Backzeit ohne Grenze („6.4a legt sie fest“); die Vorschau überbrückt die Zeit, 300 ms hält das Malen flüssig |
| Tüfteln | Dieselben Werkzeuge; Palette und Tileset-Editor zeigen Nummern und hex, die Statusleiste nennt Token („Kachel 0041r1 · Ebene 0 · Höhe 2 (16 E)“); kein JSON-Editor für das Weltdokument | YAGNI; der Editor aus C §7.1 bleibt offen |

### Nicht in 6.4a

- Kantensätze, Terrain- und Klippenpinsel, Zufall, Gelände-Muster-Shader (6.4b); glTF-Freiform, Teilmodell-Geometrie, Form 2, TMJ (6.5b); `world_preview` (Bild der Draufsicht über MCP) und `tileset_update` aus C §7.2 (`tileset_edit` von 6.3b deckt es über `g4_call`).
- Auswahl, Stempel aus der Auswahl, Pinselgrößen > 1, Formen außer Rechteck, Ebenen umbenennen oder umordnen, JSON-Editor des Weltdokuments, Kartennamen.
- Dekoder für 4 × 4-Texturen, Dateischritt von `tileset_save`/`tileset_edit` an `HEADER_TABLE`, VRAM-Schätzung `terrain::vram_bytes` (siehe „Entscheidungen“).
- „Nicht in Phase 6“ der Entscheidungen gilt wörtlich.

## Global Constraints

- **Lizenz:** gen4-studio ist MIT. Die Writer von DSPRE (NsbmdWriter, NsbtxWriter, TileBake, BdhcBuild) sind AGPL-3.0: kein Code übernehmen, auch nicht abschreiben. PDSMS hat keine Lizenz: kein Code. Tiled ist GPL: Bedienung und Modell nachbauen, keinen Code aus `src/tiled` (Füllen, Bresenham und Werkzeuglogik entstehen eigenständig).
- Keine Abhängigkeit vom NITRO-SDK (g3dcvtr.exe).
- **Byte-Treue:** Unveränderte Karten bauen weiterhin byte-gleich. Die Original-Assets bleiben unberührt, solange der Nutzer nicht malt.
- Keine ROM-Daten im Repo (`bash scripts/check-no-rom-data.sh` vor jedem Commit); Tilesets und Kachelbilder entstehen zur Laufzeit, Szenarien nennen Kacheln nur als Index, Galerie und `.fig`-Vorlagen zeigen Platzhalter.
- Keine neue npm- oder Cargo-Abhängigkeit (`cargo deny --locked check licenses advisories` grün).
- **Worktree:** `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut; `tools` als Symlink auf den Haupt-Checkout (`ln -s /Users/POLTEP/Projekte/lab/editor/gen4-studio/tools tools`, nie einchecken), `G4_APICULA=/Users/POLTEP/Projekte/lab/editor/gen4-studio/tools/apicula/bin/apicula`; Test-Umgebung nur auf `G4_VITE_PORT=1560 G4_BRIDGE_PORT=1561`, andere Ports nie anfassen (6.5b läuft daneben); Testkopien nur `Roms/_testkopien/6-4a-<zweck>` (`cp -cR` Ordner, `cp -c` `.nds`, Ziel existiert noch nicht), Originale nie öffnen; gated Tests schreiben nur auf `vanilla_copy()`.
- cargo-Tests immer mit `--all-features`. Jeder Task endet mit einem Commit; jede Nachricht endet mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`). **Kein `scripts/gate.sh` im Worktree** (läuft auf main), kein Merge, kein Push; nie `cd` in den Haupt-Checkout.
- Lang laufende Befehle (gated Tests, Brücke, Szenarien) im Hintergrund mit Log unter `target/agent/6-4a/<zweck>.log`, `uptime` davor; ein Zeittest, der unter Last reißt, wird allein wiederholt, die Grenze nie gelockert.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh`; immer `bash scripts/check-no-rom-data.sh`.
- UI nur aus Bausteinen und Tokens (Werkzeug-Taste 40 × 40 mit Kürzel, Statusleiste mit Kürzeln, `Budget`, Plakette „Nicht rückgängig“, `szene/hoehe/0..6`, `szene/gelaende/*`); keine neuen Tokens; deutsche Texte, englische Bezeichner und Commit-Messages.
- **Gate 6.4a** (Spezifikation §4, wörtlich): „Über die Test-Umgebung entsteht mit „Neue Karte malen“ eine Karte neben Zweiblattdorf bzw. Neuborkia, wird mit Pinsel, Füllen, Höhe und Rampe gestaltet und gebaut, in unter fünf Minuten; jeder Strich ein Verlaufseintrag, ⌘Z bis zum Ausgangsstand lässt `git status` leer; dasselbe über MCP (`recipe_apply` + `world_edit`), Szenarien `pt-gemalt`/`hg-gemalt`; Budget live im Inspector; Blick Hell, Dunkel, Kompakt“, dazu gated Tests auf frischen Kopien `_testkopien/6-4a-*` von Platin DE, HeartGold DE und HeartGold US, `scripts/gate.sh` grün, `cargo deny` grün, Hand und Blick beim Nutzer.
- **Phasen-Gate 6** (§4, wörtlich; mit 6.4a über die Oberfläche, Szenarien `pt-gemalt` und `hg-gemalt`): „Eine im Studio gemalte Karte mit eigenem Gelände, Höhenstufe mit Treppe, Wasser und Gras-Kacheln und abgeleiteter Kollision wird gebaut und in eine Matrix gesetzt. Sie ist in g4emu und melonDS begehbar, auf Platin DE und HeartGold DE: die Treppe funktioniert (`expect-height`), die Wand blockiert (`walk-until-blocked`), Gras löst eine Begegnung aus (`expect-battle`), die Außenkarte sieht bei Tag und Nacht richtig beleuchtet aus (Blick). Unveränderte Projekte bauen byte-gleich, `vanilla_checks` hat 0 Fehler, `scripts/gate.sh` ist grün, DSPRE öffnet die Karte (Hand).“

## Review Focus

- **Ein zweiter Strich, bevor der erste beantwortet ist** (schnelles Malen, langsamer Bake): beide kommen in Reihenfolge an, je ein Verlaufseintrag, die Vorschau des zweiten bleibt bis zu seiner Antwort, ⌘Z nimmt den zweiten zuerst (Task 4 `the_queue_sends_strokes_one_after_another`, Task 6 `a_second_stroke_waits_for_the_first`).
- **Ein Strich über den Rand der Karte, in die Nachbarkarte oder ins Leere der Matrix:** nur die Zellen der Startkarte gehen in den Aufruf, keine Zelle außerhalb 0 bis 31, kein Fehler (Task 6 `a_stroke_stays_on_its_map`).
- **Der Kern lehnt ab** (Kachel jenseits des Tilesets nach einem Paketwechsel per Git, eine kaputte Welt-JSON, DSPRE-Layout): Vorschau weg, Fehlertext über den Fehlerweg, kein Verlaufseintrag, das Weltdokument wird neu gelesen (Task 6 `a_refused_stroke_clears_its_preview`).
- **Die Welt ändert sich außerhalb der Malsitzung** (⌘Z, `world_edit` über MCP auf derselben Sitzung, Speichern mit Abgleich): Füllen und Höhe rechnen nie mit einem veralteten Dokument, weil die Sitzung `world_get` nach jeder Revision neu liest (Task 6 `the_world_is_read_again_on_every_revision`).
- **Höhe an der Grenze, Rampe auf einer Überlagerung:** + an 62 und − an −16 lassen die Zelle weg und sagen es; Höhe und Rampe schreiben immer Ebene 0, gleich welche Ebene aktiv ist (Task 4 `height_skips_cells_at_the_range`, `height_and_ramp_write_the_ground`).

---

### Task 1: Rezept `new_painted_map` (Kernel)

**Files:**
- Create: `crates/g4-kernel/src/recipes/new_painted_map.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/recipes.rs` (`pub mod new_painted_map;`, Arme in `is_irreversible`, `plan`, `apply`, `list()` führt es als immer unumkehrbar, Modul-Doku), `crates/g4-kernel/src/recipes/types.rs` (`RecipeId::NewPaintedMap` am Ende von Enum und `all()`, Titel „Neue Karte malen“, `Draft.painted`, `next_steps`: `adjust_terrain` verfügbar, Test `next_steps_lock_what_is_not_there_yet` umgestellt), `crates/g4-kernel/src/bin/g4k.rs` (falls `g4k recipe` die Ids aufzählt)

**Interfaces:**
- Consumes: `world::edit::WORLD_NEEDS_TEXT_PROJECT`, `world::doc::WorldJson::blank`, `world::grid::{parse_tile, parse_height}`, `world::tileset::tile_outside`, `world::places::headers_of_map`, `Store::{tileset, world, duplicate_map_with, transaction, world_bake_report}`, `Tx::{edit_matrix, edit_world}`, `types::{Created, check_name, check_writable, cell_index, late, refused, next_steps}`.
- Produces:

```rust
// recipes/new_painted_map.rs
/// The matrix a painted map goes into when the input names none: the world map.
pub const DEFAULT_MATRIX: u16 = 0;
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct NewPaintedMapInput {
    #[serde(default)] pub matrix: Option<u16>, pub x: u8, pub y: u8, pub name: String,
    #[serde(default)] pub tileset: Option<u16>, #[serde(default)] pub fill_tile: Option<String>,
    #[serde(default)] pub height: Option<i8>, #[serde(default)] pub header: Option<u16>,
    #[serde(default)] pub source_map: Option<u16>,
}
#[derive(Debug, Serialize)]
pub struct NewPaintedMapReport { pub map: u16, pub name: String, pub matrix: u16, pub x: u8, pub y: u8,
    pub width: u8, pub height: u8, pub header: Option<u16>, pub tileset: u16, pub base: u16,
    pub bake: Option<WorldBakeReport> }
/// „Neue Karte gemalt: „Waldweg“ (Karte 0667)“
pub fn label(name: &str, map: u16) -> String;
pub fn plan(store: &mut Store, input: &NewPaintedMapInput) -> Result<Draft, KernelError>;
/// The base map copied without buildings (irreversible, `created.maps`), then one undo step `label`: the matrix
/// grown and its cell (and header cell) set, the new map's world (blank or the source's) set; `next` adjust_terrain.
pub fn apply(store: &mut Store, input: &NewPaintedMapInput) -> Result<Outcome<NewPaintedMapReport>, KernelError>;
// recipes/types.rs
#[derive(Debug, Clone, Default, Serialize)]
pub struct DraftPainted { pub tileset: u16, pub fill_tile: String, pub height: i8, pub header: Option<u16> }
// Draft: … #[serde(default, skip_serializing_if = "Option::is_none")] pub painted: Option<DraftPainted>
```

Eine private `fn resolve(store, input) -> Result<Resolved, KernelError>` macht alle Prüfungen vor dem ersten Schreiben, gemeinsam für `plan` und `apply`, in dieser Reihenfolge: Textprojekt (`WORLD_NEEDS_TEXT_PROJECT`), `check_name`, Zelle leer oder außerhalb und Größe ≤ 255 („Zelle (4, 27) der Weltkarte 0000 ist nicht leer.“, „Die Weltkarte würde breiter als 255 Zellen.“), `check_writable(Matrix)`, `header` gegen die Header-Spalte, Nachbar bzw. `source_map` mit den Sätzen aus „Entscheidungen“, Paket, `fill_tile` (`parse_tile`, eine leere Kachel: „`.` füllt keinen Boden; eine Kachel angeben.“, `tile_outside("fill_tile", …)`), `height` (`parse_height`). Die Weltdaten der Kopie: `store.world(source_map)?.cloned()`.

- [x] **Step 1: Scheiternde Tests** (Fixture: `test_support::world_ready()` von 6.3b, Matrix 0 mit Header-Spalte, eine leere Zelle neben einer Karte mit Header; eine zweite Matrix ohne Header-Spalte): `plan_shows_the_cell_dashed_and_writes_nothing` (Draft-Zelle, `painted == {tileset: 0, fill_tile: "0000", height: 0, header: Some(h)}`, Notizen wörtlich, `irreversible == ["Karte <n> wird angelegt"]`, `member_count(Map)` und `history()` unverändert), `apply_creates_the_member_then_one_step` (`created` eine Karte, `history()` genau ein neuer Eintrag `label("Waldweg", n)`, `world(n)` = `blank(0, 0000, 0)`, Matrixzelle = n, Header der Zelle = Nachbar; `report.bake.ok`), `undo_takes_back_matrix_and_world_but_not_the_member` (nach `undo`: Zelle leer, `world(n) == None`, Member n bleibt), `a_cell_outside_grows_the_matrix`, `a_taken_cell_is_refused`, `header_on_a_matrix_without_header_column_is_refused`, `the_package_follows_the_neighbour_s_header`, `an_explicit_tileset_and_fill_win`, `a_tile_past_the_tileset_is_refused`, `no_neighbour_without_source_map_is_refused`, `source_map_copies_the_world` (Welt der Quelle mit `world_edit` geändert, Kopie trägt dieselbe Welt), `source_map_must_be_a_world_map`, `source_map_with_fill_is_refused`, `the_dspre_layout_refuses_with_the_way` (`TextFixture::dspre()`), `the_base_copy_has_no_buildings`, `next_step_adjust_terrain_is_available`; in `types.rs` `next_steps_lock_what_is_not_there_yet` mit `("adjust_terrain", true, None)`; in `recipes.rs` `new_painted_map_is_always_irreversible` (`list()` und `is_irreversible` → `Some("Karte anlegen")`).
- [x] **Step 2: Lauf** `cargo test -p g4-kernel --all-features recipes::` → FAIL (`new_painted_map` fehlt).
- [x] **Step 3: Umsetzen** wie unter Interfaces und „Entscheidungen“; `apply` nach dem Muster von `extend::apply` (`change_generation`, `Created`, `late` für Fehler nach dem ersten Schreiben). Die Modul-Doku von `recipes.rs` nennt das neue Rezept.
- [x] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` (Hintergrund, `target/agent/6-4a/t1.log`) grün.
- [x] **Step 5: Commit** `feat(recipes): new_painted_map puts a painted map into a matrix cell, one step after the member` (mit Trailer).

### Task 2: Ebenen anlegen und entfernen (`world_layers`, Kernel)

**Files:**
- Modify: `crates/g4-kernel/src/world/edit.rs` (am Ende angehängt: Typen, Funktion, Tests), `crates/g4-kernel/src/world.rs` (Re-Export angehängt)

**Interfaces:**
- Consumes: `Store::{world, transaction}`, `Tx::edit_world`, die privaten Helfer von `edit.rs` (`require_text_project`, `refuse_locked_map`, `first_error`).
- Produces:

```rust
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum LayerOp { Add { name: String }, Remove { layer: u8 } }
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct WorldLayersRequest { pub map: u16, pub op: LayerOp }
/// One undo step: `Add` appends an overlay (role `overlay`, 32 rows of `.`) named `name` (trimmed),
/// label „Ebene „Büsche“ angelegt: Karte 0123“; `Remove` drops overlay `layer`, label „Ebene 2 „Büsche“ entfernt:
/// Karte 0123“. Refused as `world_edit` (text project, world, locked map) and with „Die Bodenebene (0) bleibt.“,
/// „Eine Ebene braucht einen Namen.“, „Ebene „Büsche“ gibt es schon.“, `no_layer`. `WorldReport.cells` is 0.
pub fn world_layers(store: &mut Store, req: &WorldLayersRequest) -> Result<WorldReport, KernelError>;
```

- [x] **Step 1: Scheiternde Tests:** `adding_a_layer_is_one_step` (Ebene 1 „Büsche“, Rolle `overlay`, 32 Zeilen `.`, Verlaufstext wörtlich, `undo` → eine Ebene), `removing_an_overlay_is_one_step`, `the_ground_stays`, `a_blank_or_taken_name_is_refused`, `a_missing_layer_is_refused`, `layers_need_the_text_project`; danach `world_edit` mit `layer: 1` malt auf der neuen Ebene (`painting_on_an_added_layer`).
- [x] **Step 2: Lauf** `cargo test -p g4-kernel --all-features world::edit` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS.
- [x] **Step 5: Commit** `feat(world): add and remove overlay layers as one undo step` (mit Trailer).

### Task 3: Befehle und MCP: `world_layers`, kuratierte `world_get`/`world_edit`/`tileset_get`, `docs/mcp.md`

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/world_map.rs` (`world_layers_impl` + `#[tauri::command]`, Tests am Ende), `commands/dispatch.rs` (`world_layers` in `COMMANDS` und Arm `{ req }`, angehängt), `src-tauri/src/lib.rs` (`generate_handler!`), `mcp/access.rs` (`("world_layers", Kind::Write)` angehängt, Test `world_map_commands_have_their_access` erweitert), `mcp/catalog.rs` (drei Werkzeuge am Ende der Liste mit eigenem Kommentar `// Painting (6.4a).`, `recipe_args`-Beschreibung um `new_painted_map {matrix?, x, y, name, tileset?, fill_tile?, height?, header?, source_map?}`, `recipe_apply`-Beschreibung nennt `new_painted_map` bei den unumkehrbaren, Zähltest 87 → 90 mit Satz im Kommentar, `the_curated_set_is_the_decided_one`: `world_get`, `tileset_get` in die Liste der lesenden, `world_edit` in die Menge der schreibenden), `docs/mcp.md` (Abschnitt „Malen (6.4a)“ am Ende; die Tabelle der Werkzeuge um die drei)

**Interfaces:**
- Consumes: Task 1 (über `recipe_plan`/`recipe_apply`, nichts Neues), Task 2.
- Produces:

```rust
pub fn world_layers_impl(state: &AppState, req: WorldLayersRequest) -> Result<WorldReport, CmdError>;   // Write
// catalog.rs, input schemas
// world_get    { map: uint }            -> command("world_get")
// world_edit   { req: {map, label?, changes: [{layer?, cells: [[x, z]], tile? | height? | ramp? | permission?}]} } -> command("world_edit")
// tileset_get  { package: uint }        -> command("tileset_get")
```

Beschreibungen (Englisch, wie der Katalog): `world_get` „The world of a map of a text project (world null for a binary map or outside a text project): version, tileset (the map texture package), layers (tiles, heights, ramps as 32 rows of 32 tokens), pieces and permissions; faces and estimated model bytes of its mesh; bake (model and terrain bytes, share of 0xF000, textures, VRAM, ok and messages).“ — `world_edit` „Changes cells of a world map as one undo step and bakes it in the same step: changes [{layer?, cells [[x, z]], exactly one of tile (four hex digits, r1..r3 turn, f flip, . empty), height (steps of 8 units, −16 to 62, ground only), ramp (. or n/e/s/w with 1 or 2 steps, uphill, ground only), permission (kkcc hex or . for derived, no layer)}], label? (the history text; default „Welt geändert: Karte NNNN (N Zellen)“). Tools such as fill or rectangle are computed by the caller and sent as cells. Answers cells, change and bake. Refused outside a text project and for a map without world (recipe_apply new_painted_map makes one).“ — `tileset_get` „The tileset of a map texture package: per tile its texture, size in texels, role (ground, wall, ramp, water, grass, piece), kind, collision, height behaviour, step sound (HGSS) and wall tile; stored false while it is derived and not yet in data/tilesets.“

- [x] **Step 1: Scheiternde Tests** (Teil 1; `painted_map_recipe_over_dispatch` in Teil 2, Task 3b: erledigt)**:** `world_layers_over_dispatch` (Fixture `world_ready()`: `world_create`, `world_layers add`, `world_edit` auf Ebene 1, `world_get` → zwei Ebenen, `undo` zweimal → eine Ebene), `painted_map_recipe_over_dispatch` (`recipe_plan`/`recipe_apply` `new_painted_map` → `report.map`, `world_get(map).world` gesetzt, `get_map(map).world == true`), `world_stroke_over_the_command_path_is_fast` (`busy_world` über `world_edit` gesetzt, Aufwärmen, 10 × [`world_edit` 37 Zellen abwechselnd zwei Kacheln, `world_get`, `get_world_mesh`] über `dispatch` mit JSON, Median; druckt `world-stroke-bridge median_ms=<…> faces=<…>`; mit `G4_TIMING_STRICT=1` ≤ 300 ms); `catalog.rs`: Zähltest `== 90`, `the_curated_set_is_the_decided_one` mit den drei Namen, `curated_world_tools_route_to_their_commands` (`world_edit` mit `{req: …}` über `tools/call` → Verlaufseintrag), bestehend `every_command_has_a_kind`, Gleichheit `COMMANDS`/`generate_handler!`, der Test „unumkehrbare Rezepte stehen in `recipe_apply`“ (deckt `new_painted_map` ab).
- [x] **Step 2: Lauf** `cargo test -p studio --all-features world_ catalog` → FAIL; **Step 3: Umsetzen** (Teil 1; `recipe_args`/`recipe_apply` mit `new_painted_map` und dessen Absatz in `docs/mcp.md` in Teil 2, Task 3b: erledigt); `docs/mcp.md` „Malen (6.4a)“: die drei Werkzeuge mit Argumenten, Antwort und Ablehnungen, `new_painted_map` mit Eingabe, Entwurf, Unumkehrbarkeit (`--allow-irreversible`) und Ergebnis, `world_layers` über `g4_call`, ein Beispiel „Strich wie die App“ (Füllen als Zellen, Höhe als Gruppen gleicher Zielhöhe).
- [x] **Step 4: Lauf** → PASS (Zeittest einmal mit `G4_TIMING_STRICT=1`, Wert ins Log; Teil 2, Task 3b: mit Bake wiederholt, siehe „Offen für die Umsetzung“).
- [x] **Step 5: Commit** (Teil 1 ohne „new_painted_map in the recipe tools“; Teil 2 in Task 3b) `feat(mcp): curated world_get, world_edit, tileset_get; world_layers command; new_painted_map in the recipe tools` (mit Trailer).

### Task 4: Malmodell im Client (rein): API, Werkzeug-Ops, Warteschlange

**Files:**
- Create: `apps/studio/src/world/paint/ops.ts`, `ops.test.ts`, `apps/studio/src/world/paint/session.ts`, `session.test.ts`
- Modify: `apps/studio/src/api.world.ts` (am Ende: Typen und Aufrufe), `apps/studio/src/api.ts` (`RecipeId` um `"new_painted_map"`, `DraftDto.painted?`, Re-Exporte), `api.world.test.ts` falls vorhanden

**Interfaces:**
- Consumes: Task 2, 3 (Befehle), 6.3b (`WorldBakeReport`, `WorldReport.bake`, `WorldGetDto.bake`).
- Produces:

```ts
// api.world.ts
export interface WorldChange { layer?: number; cells: [number, number][]; tile?: string; height?: number; ramp?: string; permission?: string }
export interface WorldEditRequest { map: number; label?: string; changes: WorldChange[] }
export interface WorldReport { map: number; tileset: number; cells: number; change: number | null; bake: WorldBakeReport | null }
export const worldEdit = (req: WorldEditRequest) => invoke<WorldReport>("world_edit", { req });
export type LayerOp = { add: { name: string } } | { remove: { layer: number } };
export const worldLayers = (map: number, op: LayerOp) => invoke<WorldReport>("world_layers", { req: { map, op } });
export interface TileJson { texture: string; texture_index: number; palette?: string; size: [number, number]; role: TileRole; kind: number;
  collision: number; height: "flat" | "ramp" | "wall"; sound?: number; wall?: number; footprint?: [number, number]; origin: "vanilla" | "unclear" | "synthetic" | "user" }
export interface TilesetDto { package: number; stored: boolean; tileset: { version: number; package: number; tiles: TileJson[] } }
export const tilesetGet = (pkg: number) => invoke<TilesetDto>("tileset_get", { package: pkg });
export const tilesetSave = (pkg: number) => invoke<{ package: number }>("tileset_save", { package: pkg });
export interface TileChange { tile: number; role?: TileRole; kind?: number; collision?: number; sound?: number; wall?: number | null }
export const tilesetEdit = (pkg: number, changes: TileChange[]) => invoke<{ package: number; tiles: number; change: number }>("tileset_edit", { req: { package: pkg, changes } });
// world/paint/ops.ts
export type PaintSub = "brush" | "rect" | "fill" | "erase" | "pick" | "height" | "ramp";
export type Cell = [x: number, z: number];
export type RampDir = "n" | "e" | "s" | "w";
export interface PaintOptions { layer: number; tile: string; ramp: { dir: RampDir; steps: 1 | 2 }; lower: boolean }
export interface WorldCells { tiles: string[][][]; heights: number[][]; ramps: string[][] }   // [layer][z][x], [z][x]
export const HEIGHT_MIN = -16; export const HEIGHT_MAX = 62;
export function cellsOf(world: WorldJson): WorldCells;
export function lineCells(a: Cell, b: Cell): Cell[];                         // Bresenham, both ends, no repeats
export function rectCells(a: Cell, b: Cell): Cell[];                         // row-major, either corner order
export function fillCells(cells: WorldCells, layer: number, at: Cell): Cell[];   // 4-neighbourhood, same token
export function pickedTile(cells: WorldCells, layer: number, at: Cell): string | null;   // active layer, else topmost non-empty
export function turnTile(token: string): string;                            // 0041 → 0041r1 → r2 → r3 → 0041, f kept
export function flipTile(token: string): string;
export function turnRamp(dir: RampDir): RampDir;                             // n → e → s → w → n
export function strokeLabel(sub: PaintSub, map: number, cells: number): string;
/** The one request of a stroke (`cells` in stroke order, map-local), unchanged cells dropped; null when nothing changes. */
export function strokeRequest(map: number, sub: Exclude<PaintSub, "pick">, cells: Cell[], world: WorldCells, o: PaintOptions):
  { req: WorldEditRequest; skipped: number } | null;
// world/paint/session.ts
export interface StrokeQueue { push(req: WorldEditRequest): Promise<WorldReport>; pending(): number }
export function createStrokeQueue(send: (req: WorldEditRequest) => Promise<WorldReport>, onPending: (n: number) => void): StrokeQueue;
```

`strokeRequest`: `brush`/`rect` setzen `o.tile` auf `o.layer`; `fill` erwartet die Zellen von `fillCells`; `erase` setzt `.` auf `o.layer`; `height` gruppiert je Zielhöhe (`heights[z][x] ± 1`, Zellen an der Grenze zählen in `skipped`), Ebene 0, ohne `layer`; `ramp` setzt `${dir}${steps}` bzw. mit `lower` `.`, Ebene 0. `label` immer `strokeLabel(...)` mit der Zahl der gesendeten Zellen.

- [x] **Step 1: Scheiternde Tests:** `ops.test.ts`: `line_cells_cover_a_diagonal_without_gaps` ((0,0)→(5,2) sechs Zellen, jede Nachbarzelle 8-zusammenhängend), `rect_cells_in_either_corner_order`, `fill_stops_at_another_tile_and_at_the_edge` (Fixture: Insel aus `0001` in `0000`), `fill_counts_turned_tiles_as_other_tiles`, `picked_tile_falls_back_to_the_top_layer`, `turn_and_flip_tokens`, `brush_request_is_one_change_with_its_label` (`"Gemalt: Karte 0123 (3 Zellen)"`), `unchanged_cells_send_nothing` (Pinsel auf gleiche Kachel → `null`), `height_groups_cells_by_target` (Zellen auf 0 und 2 → zwei Änderungen 1 und 3), `height_skips_cells_at_the_range` (62 mit + → `skipped 1`), `height_and_ramp_write_the_ground` (aktive Ebene 1 → kein `layer`), `ramp_token_and_removal` (`e1`, mit `lower` `.`), `erase_sets_empty_on_the_active_layer`; `session.test.ts`: `the_queue_sends_strokes_one_after_another` (zweiter `send` erst nach Auflösung des ersten, `pending` 2 → 1 → 0), `a_refused_stroke_does_not_block_the_next`.
- [x] **Step 2: Lauf** `cd apps/studio && pnpm vitest run src/world/paint` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm test`, `pnpm build`.
- [x] **Step 5: Commit** `feat(studio): paint operations and a serial stroke queue for world maps` (mit Trailer).

### Task 5: Unterwerkzeuge, Kürzel, Werkzeugleiste und Statusleiste

**Files:**
- Create: `apps/studio/src/workbench/paintTools.ts`, `paintTools.test.ts`, `apps/studio/src/workbench/paint/PaintContext.tsx`, `PaintContext.test.tsx`
- Modify: `apps/studio/src/ui/frame/FloatingToolbar.tsx` (+ `.module.css`, Test: zweite Gruppe `[data-paint-tools]`), `apps/studio/src/workbench/SceneOverlays.tsx`, `apps/studio/src/app/useShortcuts.ts` (+ Test), `apps/studio/src/app/ToolHints.tsx` (+ Test), `apps/studio/src/workbench/tools.ts` (Hinweistext von `paint` je Kartenart), `apps/studio/src/workbench/Workbench.tsx` oder `WorkbenchContext.tsx` (Provider)

**Interfaces:**
- Consumes: Task 4 (`PaintSub`, `PaintOptions`, `turnTile`, `flipTile`, `turnRamp`).
- Produces:

```ts
// paintTools.ts
export interface PaintToolSpec { id: PaintSub; label: string; key: string; icon: IconName; hint: string; statusHints: { keys: string; does: string }[] }
export const PAINT_TOOLS: PaintToolSpec[];   // brush B, rect P, fill G, erase E, pick I, height H, ramp R
/** What a plain key does while painting a world map: a sub-tool, turn (Z), flip (X), ramp steps (1, 2); null: not ours. */
export function paintKey(key: string): { sub: PaintSub } | { turn: true } | { flip: true } | { steps: 1 | 2 } | null;
// PaintContext.tsx
export interface PaintState extends PaintOptions { sub: PaintSub }
export interface PaintModel { state: PaintState; set(patch: Partial<PaintState>): void; /** the selected cell's map when it is a world map of the text project */ target: number | null }
export function PaintProvider(props: { children: ReactNode }): JSX.Element;
export function usePaint(): PaintModel;
```

Texte: Werkzeug-Tasten „Pinsel“ (B, `paint`), „Rechteck“ (P, `rectangle`), „Füllen“ (G, `bucket`), „Radierer“ (E, `eraser`), „Pipette“ (I, `eyedropper`), „Höhe“ (H, `heights`), „Rampe“ (R, `ramp`); fehlt ein Symbol in `icons.ts`, kommt es aus Phosphor (`Rectangle`, `PaintBucket`, `Eraser`, `Eyedropper`, `Stairs`) und steht im README-Nachtrag. Statusleiste je Unterwerkzeug, z. B. Pinsel „Ziehen malt · Rechtsklick nimmt auf · Z dreht · X spiegelt“, Höhe „Klick hebt · ⇧Klick senkt · Ziehen über mehrere“, Rampe „Klick setzt Rampe nach Osten, 1 Stufe · Z dreht · 1/2 Stufen · ⇧Klick entfernt“ (Richtung und Stufen aus dem Zustand). Hinweis von „Gelände malen“: Welt-Karte „Malt Kacheln, Höhen und Rampen auf die gewählte Welt-Karte.“, binäre Karte wie heute. `useShortcuts` ruft `paintKey` vor `toolForKey`, nur wenn `tool === "paint"` und `usePaint().target !== null`; ⇧ ist für diese Tasten kein Hindernis (Z mit ⇧ dreht nicht zurück: YAGNI); Esc: Unterwerkzeug ≠ Pinsel → Pinsel, sonst wie bisher. Die Leinwand trägt `data-paint-sub` (Task 6 setzt es).

- [x] **Step 1: Scheiternde Tests:** `paintTools.test.ts` (`keys_are_the_tiled_ones`, `paint_key_maps_turn_flip_and_steps`, `no_paint_key_clashes_with_select_figure_door_trigger_sign`: B, P, G, E, I, H, R, Z, X ∩ {V, F, T, A, S} leer), `useShortcuts.test.ts` (`g_fills_while_painting_a_world_map_and_builds_otherwise`, `escape_returns_to_the_brush_first`), `FloatingToolbar.test.tsx` (`paint_tools_show_only_for_a_world_map`, `aria-pressed` des aktiven Unterwerkzeugs), `ToolHints.test.tsx` (`ramp_hint_names_direction_and_steps`), `PaintContext.test.tsx` (`target_is_null_for_a_binary_map_and_in_the_dspre_layout`).
- [x] **Step 2: Lauf** `pnpm vitest run src/workbench src/app src/ui/frame` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Stil-, Token- und Tiefenprüfung.
- [x] **Step 5: Commit** `feat(studio): paint sub-tools with Tiled keys in the floating toolbar and the status line` (mit Trailer).

### Task 6: Szene: Striche, optimistische Vorschau, ein `world_edit` je Strich

**Files:**
- Create: `apps/studio/src/world/scene/strokes.ts`, `strokes.test.ts`, `apps/studio/src/world/scene/strokePreview.ts`, `strokePreview.test.ts`, `apps/studio/src/world/paint/usePaintSession.ts`, `usePaintSession.test.tsx`
- Modify: `apps/studio/src/world/scene/canvasClicks.ts` (Klicks im Malmodus einer Welt-Karte gehen an `strokes.ts`; Kollisionsmalen binärer Karten bleibt), `world/scene/SceneHost.ts` (`setPaintPointer`), `world/scene/types.ts` (`SceneCallbacks.paint`), `world/scene/useSceneSetup.ts`, `world/MatrixScene.tsx`, `world/scene/heightLayer.ts` (nur Aufruf nach dem Refresh), `world/scene/sceneProbe.ts` (`strokePending()`), `world/scene/canvasClicks.test.ts`

**Interfaces:**
- Consumes: Task 4 (`lineCells`, `rectCells`, `fillCells`, `pickedTile`, `strokeRequest`, `createStrokeQueue`, `worldEdit`, `worldGet`), Task 5 (`usePaint`), `useMapRefresh`/`useMapModelRefresh`, `setDraft`-Rahmen, `heightColor`.
- Produces:

```ts
// strokes.ts
export interface Stroke { map: number; sub: PaintSub; cells: Cell[]; lower: boolean }
export interface PaintCallbacks { sub: PaintSub | null; target: number | null; onStroke(stroke: Stroke): void; onPick(map: number, at: Cell): void; onSelectCell(cell: { col: number; row: number }): void }
/** Pointer strokes on the canvas while `refs.callbacks.current.paint.sub` is set: cells of the target map only, each once,
 * gaps filled by `lineCells`; `rect` reports its two corners' cells (`rectCells`), `fill` and `pick` the clicked cell;
 * right button picks; pointercancel drops the stroke. Draws the preview while the pointer moves. */
export function attachStrokes(h: SceneHost, picker: Picker, refs: SceneRefs): () => void;
// strokePreview.ts
export type PreviewLook = { texture: CanvasImageSource } | { color: number } | { erase: true };
export function setStrokePreview(h: SceneHost, map: number, cells: Cell[], look: PreviewLook): void;
export function setStrokeRect(h: SceneHost, map: number, a: Cell, b: Cell | null): void;
export function clearStrokePreview(h: SceneHost): void;
// SceneHost.ts
setPaintPointer(on: boolean): void;   // LEFT none, MIDDLE pan; keeps the top-down / slanted mapping otherwise
// types.ts
// SceneCallbacks: … paint: PaintCallbacks   (sub null: no paint sub-tool, clicks as before)
// usePaintSession.ts
/** The open paint session of `target`: its world (world_get, read again on every revision), tileset (tileset_get), queue;
 * `onStroke` builds the request, pushes it, then refreshes mesh, heights and world and clears the preview. */
export function usePaintSession(target: number | null, revision: number, refresh: (map: number) => void, onError: (e: unknown) => void):
  { world: WorldGetDto | null; tileset: TilesetDto | null; onStroke(stroke: Stroke): void; onPick(map: number, at: Cell): void; pending: number };
```

Die Leinwand trägt `data-paint-sub="<sub>"` und `data-stroke-pending="<n>"` (für den Browser-Teil: warten auf `"0"`); `window.__g4SceneProbe.strokePending()` gibt dieselbe Zahl. Höhe und Rampe schalten beim Wählen „Höhen“ ein (`useSceneLayers`), nach jedem Refresh baut `showHeightLayer` die Schicht neu, wenn sie sichtbar ist.

- [x] **Step 1: Scheiternde Tests** (mit `dragTestKit.testkit.ts`): `strokes.test.ts`: `a_drag_is_one_stroke_of_distinct_cells`, `fast_moves_fill_the_gaps`, `a_stroke_stays_on_its_map` (Review Focus: Zug über den Rand in die Nachbarkarte und ins Leere → nur Zellen der Startkarte, 0 bis 31), `pointerdown_on_another_map_selects_it_and_paints_nothing`, `right_click_picks`, `rect_reports_its_corners_and_draws_the_dashed_frame`, `cancel_drops_the_stroke_and_the_preview`, `left_button_does_not_orbit_while_painting` (`controls.mouseButtons.LEFT` leer, nach dem Verlassen wiederhergestellt); `strokePreview.test.ts`: `preview_plates_sit_above_the_height_layer`, `clear_frees_the_geometry`; `usePaintSession.test.tsx`: `one_stroke_is_one_world_edit_with_its_label`, `a_second_stroke_waits_for_the_first` (Review Focus), `a_refused_stroke_clears_its_preview` (Review Focus: `worldEdit` verwirft → `clearStrokePreview`, `onError`, `world_get` erneut), `fill_uses_the_world_read_last`, `after_the_answer_mesh_heights_and_world_are_read_again`, `the_world_is_read_again_on_every_revision` (Review Focus: eine neue `revision` ohne Strich, etwa ⌘Z oder `world_edit` über MCP, liest `world_get` neu, das nächste Füllen rechnet mit dem neuen Dokument); `canvasClicks.test.ts`: `binary_maps_keep_collision_clicks`.
- [x] **Step 2: Lauf** `pnpm vitest run src/world` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm test`, `pnpm build`.
- [x] **Step 5: Commit** `feat(studio): paint strokes on world maps, drawn at once, one world_edit on release` (mit Trailer).

### Task 7: Hand: Palette, Ebenen, Budget live

**Files:**
- Create: `apps/studio/src/workbench/inspector/TilePalette.tsx`, `.module.css`, `.test.tsx`; `LayerList.tsx`, `.module.css`, `.test.tsx`; `apps/studio/src/world/paint/paletteImages.ts`, `.test.ts`
- Modify: `apps/studio/src/workbench/inspector/MatrixCellEditor.tsx` (+ Test; Abschnitte im Malmodus), `inspector/MapBudgetSection.tsx` (+ Test; Satz bei abgelehntem Bake), `apps/studio/src/world/scene/worldMesh.ts` (Sichtbarkeit je Ebene: `setLayerVisible(h, map, layer, on)`), `apps/studio/src/ui/gallery/pages/WorldPage.tsx` (+ Test; Blöcke „Malwerkzeuge“, „Palette“, „Ebenen“ mit Platzhaltern)

**Interfaces:**
- Consumes: Task 4 (`tilesetGet`, `getTilesetTexture`, `worldLayers`), Task 5 (`usePaint`), Task 6 (`usePaintSession`: `world`, `tileset`), `placeholderColor`.
- Produces:

```ts
// paletteImages.ts
/** The texture of a tile as a bitmap (cached per package and texture; null without picture). */
export function tileImage(pkg: number, texture: string): Promise<ImageBitmap | null>;
export function resetTileImages(): void;   // on project switch, like resetWorldTextures
// TilePalette.tsx
export function TilePalette(props: { tileset: TilesetDto; tile: string; onTile: (token: string) => void; depth: Depth }): JSX.Element;
// LayerList.tsx
export function LayerList(props: { map: number; world: WorldJson; active: number; hidden: ReadonlySet<number>; onActive: (layer: number) => void;
  onToggle: (layer: number) => void; onError: (e: unknown) => void }): JSX.Element;
// MapBudgetSection: new prop  bake?: WorldBakeReport | null
```

Texte und Attribute: Abschnitt „Palette“ (`[data-paint-palette]`), Gruppenüberschriften „Boden“, „Gras“, „Wasser“, „Wand“, „Rampe“, „Teilmodell“ (leere Gruppen fehlen), Taste `[data-palette-tile="0041"]` mit `aria-label` „Kachel 0041, gras_a“ und `aria-pressed`; die gewählte Kachel zeigt Drehung und Spiegelung als kleine Marke. Abschnitt „Ebenen“ (`[data-paint-layers]`): Zeilen `[data-layer-row="0"]` „Boden · Boden“, `[data-layer-row="1"]` „Büsche · Überlagerung“ mit Radio „Aktiv“, Symboltaste „Zeigen“/„Verbergen“ (`aria-pressed`), bei Überlagerungen „Entfernen“; darunter „Ebene hinzufügen“ mit Namensfeld (Vorgabe „Ebene <n>“). Budget: `[data-budget-refused]` mit dem Satz aus „Entscheidungen“.

- [x] **Step 1: Scheiternde Tests:** `TilePalette.test.tsx` (`tiles_are_grouped_by_role_in_order`, `a_tile_without_picture_shows_its_role_colour`, `clicking_chooses_the_tile`, `tinkering_shows_kind_and_collision_in_the_tooltip`), `LayerList.test.tsx` (`adding_calls_world_layers_with_the_name`, `the_ground_has_no_remove`, `hiding_does_not_call_the_kernel`), `paletteImages.test.ts` (`images_are_cached_per_package_and_texture`, `null_stays_null`), `MatrixCellEditor.test.tsx` (`paint_sections_show_for_a_world_map_while_painting`, `paint_sections_hide_for_another_tool`), `MapBudgetSection.test.tsx` (`budget_follows_each_stroke` (zwei Revisionen, zwei Lesungen, zweite Zahl gezeigt), `a_refused_bake_says_the_game_keeps_the_last_model`), `WorldPage.test.tsx` (drei neue Blöcke).
- [x] **Step 2: Lauf** `pnpm vitest run src/workbench/inspector src/world/paint src/ui/gallery` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Prüfskripte.
- [x] **Step 5: Commit** `feat(studio): tile palette, layer list and live budget in the inspector while painting` (mit Trailer).

### Task 8: Tileset-Editor und binäre Karten

**Files:**
- Create: `apps/studio/src/workbench/inspector/TileEditor.tsx`, `.module.css`, `.test.tsx`
- Modify: `inspector/MatrixCellEditor.tsx` (+ Test: Abschnitt „Kachel“; Notiz `[data-paint-binary]` mit Knopf für binäre Karten im Malmodus)

**Interfaces:**
- Consumes: Task 4 (`tilesetEdit`, `tilesetSave`, `TileJson`), 6.3b (`worldAdopt`), Task 7 (gewählte Palettenkachel), `tile_behaviors`-Namen wie das Gelände-Overlay (`content.ts::overlayColor`-Quelle `names: Map<number, TileBehavior>`).
- Produces:

```ts
export function TileEditor(props: { tileset: TilesetDto; tile: number; family: "pt" | "hgss"; depth: Depth; onChanged: () => void; onError: (e: unknown) => void }): JSX.Element;
```

Texte: Abschnitt „Kachel 0041“ (`[data-tile-editor]`), Felder „Rolle“ (Boden, Wand, Rampe, Wasser, Gras, Teilmodell), „Gelände“ (Bauen: Name, Tüfteln: `0x02`), „Blockiert“, „Schrittgeräusch“ (nur HeartGold/SoulSilver, 0 bis 127), „Wand“ (Kachelnummer, leer = keine); Herkunft als Satz „Aus den Vanilla-Karten abgeleitet“ / „Nicht eindeutig, Vorgabe Boden“ / „Von dir gesetzt“. Ohne Datei: Felder gesperrt, Knopf „Tileset im Projekt anlegen“ (`[data-tileset-save]`) mit dem Satz „Die Vorgaben stehen dann in data/tilesets/0006.json und gelten für alle Welt-Karten dieses Pakets.“. Binäre Karte im Malmodus: Notiz „Boden malen: Karte als Quelle übernehmen“ und Knopf „Karte als Quelle übernehmen“ (`[data-paint-adopt]`, → `worldAdopt(map)`; im DSPRE-Layout deaktiviert mit „Boden malen gibt es im Textprojekt (Technik › Textprojekt).“).

- [ ] **Step 1: Scheiternde Tests:** `TileEditor.test.tsx` (`each_confirmed_field_is_one_tileset_edit` (Rolle → `[{tile: 41, role: "grass"}]`, Blockiert → `collision` mit Bit 7, die übrigen Bits bleiben), `sound_shows_only_for_hgss`, `without_file_the_fields_wait_for_tileset_save`, `tinkering_shows_kind_in_hex`), `MatrixCellEditor.test.tsx` (`a_binary_map_offers_adopt_while_painting`, `adopt_is_disabled_in_the_dspre_layout`).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`.
- [ ] **Step 5: Commit** `feat(studio): tileset editor per tile and the way from a binary map to painting` (mit Trailer).

### Task 9: Rezept in der Oberfläche, „Gelände anpassen“, Browser-Teil

**Files:**
- Create: `apps/studio/src/workbench/NewPaintedMapDialog.tsx`, `.module.css`, `.test.tsx`
- Modify: `apps/studio/src/workbench/ExtendAreaDialog.tsx` (+ Test: „Stattdessen neu malen“), `WorldMapPanel.tsx` (Dialog wechseln, nach dem Anwenden Zelle wählen), `apps/studio/src/workbench/recipes.ts` (+ Test: `KNOWN` mit `new_painted_map`, `NEXT.new_painted_map = [["adjust_terrain", "Gelände anpassen"]]`, `lockReason` ohne `adjust_terrain`), `wizards/WizardResult.tsx` (+ Test: `adjust_terrain` wählt die Karte, schaltet Draufsicht und „Gelände malen“ mit Pinsel ein), `apps/studio/src/recipes/api.ts` (+ Test)

**Interfaces:**
- Consumes: Task 1 (über `recipePlan`/`recipeApply`), Task 5 (`usePaint().set`), `useShell().setTopDown`, `useDraftPlan`, Task 7 (`TilePalette` für „Boden“).
- Produces:

```ts
export function NewPaintedMapDialog(props: { matrix: MatrixDto; x: number; y: number; depth: Depth; onDone: (report: { map: number } | null) => void;
  onError: (e: unknown) => void; onDraft: (draft: DraftDto | null) => void }): JSX.Element;
/** After new_painted_map or the step adjust_terrain: the cell selected, top-down on, tool paint with the brush. */
export function startPainting(cell: { matrix: number; x: number; y: number }): void;   // in WorkbenchContext
```

Texte: Titel „Neue Karte malen“, Felder „Name“ (Pflicht), „Boden“ (Palette des Pakets aus `draft.painted.tileset`, Vorgabe `draft.painted.fill_tile`), „Höhe“ (Stufen, −16 bis 62, Vorgabe 0); Tüfteln zusätzlich „Tileset“ und „Header“; darunter die Notizen des Entwurfs (`DraftNotes`) und die Plakette „Nicht rückgängig“ mit „Karte 0667 wird angelegt“ (`DraftIrreversible`); Knopf „Karte malen“. Ein Fehler des Plans steht wie bei „Gebiet erweitern“ (`DraftFailed`). Im DSPRE-Layout fehlt „Stattdessen neu malen“.

- [ ] **Step 1: Scheiternde Tests:** `NewPaintedMapDialog.test.tsx` (`plans_with_the_cell_and_shows_the_draft`, `apply_reports_the_map_and_starts_painting`, `name_is_required`, `tinkering_offers_tileset_and_header`), `ExtendAreaDialog.test.tsx` (`paint_instead_only_in_the_text_project`), `recipes.test.ts` (`adjust_terrain_is_no_longer_locked`, `new_painted_map_leads_to_adjust_terrain`), `WizardResult.test.tsx` (`adjust_terrain_starts_painting`).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm test`, `pnpm build`, Prüfskripte.
- [ ] **Step 5: Browser-Teil** (Test-Umgebung `G4_VITE_PORT=1560 G4_BRIDGE_PORT=1561`, Debug-Binaries aus `cargo build --workspace --all-features`; Kopie `6-4a-web` von Platin DE und `6-4a-webhg` von HeartGold DE, je mit `.nds`; vorbereitet mit `g4k git init`, `call.sh assign_base`, `call.sh adopt_text_project`, außen `git add -A && git commit -m Textprojekt`). Je Spiel, Zeit ab dem Klick auf „+“ bis `build_rom` fertig (Ziel < 5 min, Zeitstempel ins Protokoll):
  1. Bauen, Weltkarte, Zweiblattdorf bzw. Neuborkia gewählt; „+“ an einer Zelle daneben, „Stattdessen neu malen“, Name „Malweg“, Boden aus der Palette; die Szene zeigt die Zelle gestrichelt (`[data-draft]`), „Karte malen“.
  2. Danach: Draufsicht gedrückt, „Gelände malen“ und Pinsel aktiv, Verlauf „Neue Karte gemalt: „Malweg“ (Karte NNNN)“; ⌘S, außen `git add -A && git commit -m "Karte angelegt"` (Ausgangsstand).
  3. Striche über `__g4SceneProbe.tileToClient` und `left_click_drag` des Browser-Werkzeugs, je Strich warten auf `canvas[data-stroke-pending="0"]`: Füllen (G) Gras-Kachel auf einer Ecke, Pinsel (B) Weg aus Boden quer, Rechteck (P) Wasser, Höhe (H) zweimal über ein 5 × 3-Feld (Stufe 2), Rampe (R, Z bis Osten, 1) auf zwei Zellen davor, Höhe einmal auf die obere Rampenzelle; nach jedem Strich ein neuer Eintrag in `call.sh history` mit dem Werkzeug-Label, `[data-world-mesh-faces]` ändert sich, „Budget“ zeigt neue Modell-Bytes (Budget live).
  4. „Höhen“ zeigt Stufe 2 in `szene/hoehe/2`, Isolinien am Plateau; ⌘S; Technik › Veröffentlichen „Bauen“ → ROM gebaut, Zeile „Karten backen“.
  5. ⌘Z so oft, bis der Verlauf beim Ausgangsstand steht (Zahl der Striche), ⌘S → `git status --porcelain` der Kopie **leer**, `call.sh dirty` `[]`; ⇧⌘Z einmal → wieder ein Strich.
  6. Bilder Hell, Dunkel, Kompakt der Malansicht (Palette, Ebenen, Werkzeugleiste, Budget) und des Dialogs unter `target/agent/gate-<datum>/6-4a/blick-*.png`, Notiz `browser.md` (Zellen, Karte, Zeiten, Zahlen).
- [ ] **Step 6: Commit** `feat(studio): new painted map from the world map's plus, adjust terrain starts painting` (mit Trailer).

### Task 10: Gated Tests auf Platin DE, HeartGold DE, HeartGold US

**Files:**
- Create: `crates/g4-kernel/tests/world_paint.rs`
- Modify: `docs/dev-setup.md` (`tests/world_paint.rs` in der Tabelle `G4_TEST_PROJECT`)

**Interfaces:**
- Consumes: Tasks 1, 2; 6.3b (`world_edit` backt, `Store::world_bake_report`, `build::build`); `preview::standard_map`; `vanilla_copy()`, `G4_TEST_PROJECT`.
- Produces: die Belegzeilen `paint-gate …`, `paint-undo …`, `palette …` für die Gate-Datei.

- [ ] **Step 1: Tests** (Store auf `vanilla_copy()`, als Textprojekt eingerichtet wie `tests/world_bake.rs` von 6.3b):
  - `painted_map_beside_the_start_town_bakes_and_builds`: Zelle = erste Zelle neben einer Zelle der Startstadt (`standard_map`: Header und Zellen in Matrix 0), die leer ist oder außerhalb liegt, Reihenfolge N, W, O, S je Zelle zeilenweise; keine solche: Fehlschlag mit Satz (dann Abweichung in die Gate-Datei); `recipes::apply` `new_painted_map` (Name „Malweg“); dann die Striche der Oberfläche als je ein `world_edit` mit den Labels aus `strokeLabel` (Füllen Gras über die Zellen einer 4er-Füllung, Pinsel, Rechteck Wasser, Höhe zweimal, Rampe, Höhe); `history()` Tiefe = 1 + Striche; `bake.ok`, Modell ≤ 0xF000, BDHC ≤ 0x9000 + 16; `save`, `build::build` → ROM; `vanilla_checks`-Gruppe der Karte 0 Fehler; druckt `paint-gate <code> map=<n> cell=(<x>,<y>) strokes=<n> nsbmd=<n> share=<f> bdhc=<n> ms=<n>`.
  - `undo_to_the_start_leaves_the_files_as_they_were`: nach Rezept und `save` SHA-1 jeder Datei unter `data/` und `project.json`; Striche; `undo` × Striche; `save`; alle SHA-1 gleich, keine Datei dazu (der Beleg für `git status` leer ohne Git im Test); druckt `paint-undo <code> strokes=<n> files=<n> equal=<n>`.
  - `palette_textures_of_every_map_package`: je Kartentextur-Paket `tileset_get`-Ableitung, je Kachel `tileset_texture`; druckt `palette <code> packages=<n> tiles=<n> decoded=<n> placeholder=<n>`; verlangt nur `tiles > 0`.
- [ ] **Step 2: Kopien** `6-4a-pt`, `6-4a-hgde`, `6-4a-hgus` (je `_DSPRE_contents` mit `.nds` daneben).
- [ ] **Step 3: Lauf** je Kopie nacheinander im Hintergrund: `G4_ROMS_DIR=$R G4_TEST_PROJECT=$K/6-4a-<k>_DSPRE_contents cargo test --release -p g4-kernel --all-features --test world_paint -- --nocapture --test-threads 1 > target/agent/6-4a/gated-<k>.log 2>&1` → PASS; dann `--test vanilla_checks` je Kopie (0 Fehler).
- [ ] **Step 4: Commit** `test(world): painted map beside the start town bakes and builds, undo restores every file, palette pictures counted` (mit Trailer).

### Task 11: Szenarien `pt-gemalt`/`hg-gemalt` über MCP, Phasen-Gate über die Oberfläche

**Files:**
- Modify: `scripts/agent/scenarios/pt-gemalt.yaml`, `scripts/agent/scenarios/hg-gemalt.yaml`
- Create: `scripts/agent/scenarios/{pt,hg}-gemalt-buehne.yaml`, `scripts/agent/scenarios/{pt,hg}-gemalt-ui.yaml`

**Interfaces:**
- Consumes: kuratiert `recipe_apply`, `world_edit`, `world_get`, `tileset_get`, `update_event`, `get_map`; über `g4_call` `tileset_save`, `tileset_edit`, `world_bake`; g4emu-Schritte von 6.2/6.3b.

- [ ] **Step 1: `pt-gemalt`/`hg-gemalt` umstellen** (Kopfkommentar: was 6.4a ändert): `mcp: [--allow-irreversible, --expert]`; bis `create_area` wie 6.3b; dann `recipe_apply {recipe: new_painted_map, input: {matrix: "${area.report.matrix}", x: 1, y: 0, name: Malweg, fill_tile: <Boden>}}` (`as: p`, Erwartung `report.width: 2`), `tileset_save`/`tileset_edit` wie 6.3b, dann die Striche als **kuratierte** `world_edit`-Aufrufe mit Labels (Füllen Gras x 16–28, z 3–10; Rechteck Wasser x 2–8, z 24–29; Höhe +1 und +1 auf x 18–22, z 20–22 sowie +1 auf x 17; Rampe `e1` auf x 16–17, z 20–22; Tür-Override `6980` auf (12, 21)), `world_get` mit `expect` `bake.ok: true`; `update_event` verschiebt Tür-Warp und Geschenkfigur um 32 nach Osten; `get_map ${p.report.map}` mit den Permissions von 6.3b; `build: build/out.nds`. Spielschritte wie 6.3b mit `x + 32` (`expect-at 50 21` usw.).
- [ ] **Step 2: Bühne und UI-Spiel:** `*-gemalt-buehne.yaml` = Schritt 1 bis einschließlich `tileset_edit`, ohne Rezept und Build; `*-gemalt-ui.yaml` = `mcp: []`, `edits: []`, `build: build/out.nds`, die Spielschritte von Schritt 1 (Tür und Geschenk verschiebt der Agent nach dem Malen mit `call.sh update_event`).
- [ ] **Step 3: Lauf MCP** je Spiel auf frischen Kopien `6-4a-szen-pt`/`6-4a-szen-hg`: `uv run g4emu scenario ../agent/scenarios/pt-gemalt.yaml --project $K/6-4a-szen-pt_DSPRE_contents --out $W/target/agent/gate-$(date +%F)/6-4a/pt-gemalt --set NEUER_HEADER=593 --set S_WILD=150 --set L_WILD=5 --set S_GIFT=445 > target/agent/6-4a/pt-gemalt.log 2>&1` (HeartGold `NEUER_HEADER=540`) → Rückgabe 0, `"ok": true`, `mcp.jsonl` mit einem Verlaufseintrag je `world_edit`.
- [ ] **Step 4: Phasen-Gate über die Oberfläche** je Spiel auf `6-4a-buehne-pt`/`-hg`: Bühne laufen lassen; Test-Umgebung (1560/1561) auf der Kopie; „+“ östlich der 1 × 1-Zelle, „Neue Karte malen“, die Gestaltung von Schritt 1 mit den Werkzeugen der Oberfläche (je Strich ein Verlaufseintrag), ⌘S, `call.sh update_event` für Tür und Geschenk; dann `*-gemalt-ui.yaml` → Rückgabe 0. Blick des Agenten auf `tag` (und `nacht`, wenn 6.3b die Uhr hat): Boden texturiert, Wand, Treppe, Wasser, Gras, Außenlicht.
- [ ] **Step 5: Commit** `test(g4emu): pt-gemalt and hg-gemalt paint through recipe_apply and world_edit; stage and play for the painted UI map` (mit Trailer).

### Task 12: Doku der Formate und Design-System-Nachtrag

**Files:**
- Modify: `docs/formats/recipes.md` (`new_painted_map`: Eingabe, Regeln, Sätze, Entwurf, Unumkehrbarkeit, Ergebnis, `adjust_terrain`), `docs/formats/world_map.md` (Abschnitt „Malen“: Werkzeuge → Änderungen, Labels, `world_layers`; „Was später kommt“ nachgezogen), `docs/formats/tileset.md` (Tileset-Editor: welches Feld welche Änderung), `design-system/README.md` (Abschnitt „Nachtrag 6.4a“ am Ende)
- Create: `design-system/openpencil/NACHTRAG-6-4a.md`, `design-system/openpencil/beispiele/gelaende-malen.body.js`

- [ ] **Step 1: Formate** wie unter Files; Wortlaut und Zahlen aus dem Code der Tasks 1 bis 9; Beispiele synthetisch (synthetisches Tileset, `fixtures::gate_world`).
- [ ] **Step 2: README** „Nachtrag 6.4a“ nach dem Muster „Nachtrag 6.2“: Quelle (Dateien), Werkzeug „Gelände malen“ mit den sieben Unterwerkzeugen (Tasten, Kürzel, Statusleiste), Maus, Strichvorschau, Palette, Ebenen, Kachel (Tileset-Editor), Budget-Satz, binäre Karte, Dialog „Neue Karte malen“ mit Entwurf und Plakette, „Gelände anpassen“, Galerie-Blöcke, Vergleichsquelle `target/agent/gate-<datum>/6-4a/`; keine neuen Tokens; neue Symbole (falls Task 5 welche brauchte) mit Phosphor-Namen; Abweichungen: Palette und Ebenen im Inspector statt eigener Hand-Gruppe, G ist im Malmodus Füllen, Muster-Shader 6.4b; die Roadmap-Zeile „Gelände malen, Höhen-Overlay, Draufsicht-Kamera und Overlay-Muster (Phase 6)“ in „Aus dem Design-System, später“ zieht für Malen und Kamera nach.
- [ ] **Step 3: Anleitung** `NACHTRAG-6-4a.md` nach `NACHTRAG-6-2.md`: Frames „Gelände malen: Route 201“ (Draufsicht, Werkzeugleiste mit Unterwerkzeugen, Inspector mit Palette, Ebenen, Kachel, Budget-Leiste, Statusleiste mit Kürzeln; `MODE = 'malen'`), „Gelände malen: Route 201 (Dunkel)“, „Neue Karte malen: Entwurf“ (gestrichelte Zelle auf der Weltkarte, Dialog, Plakette; `MODE = 'entwurf'`), „Tileset-Editor“ (`MODE = 'tileset'`), Brett „Komponenten (App): Malen“ (Unterwerkzeuge in Ruhe/Gedrückt, Palettenkachel mit und ohne Bild, Ebenenzeile, Budget mit abgelehntem Bake; `MODE = 'brett'`); `beispiele/gelaende-malen.body.js` gegen `jsx-lib-app.js`, gegen eine Attrappe der Bausteine in allen Modi durchlaufen lassen; die `.fig` nur, wenn OpenPencil auf 127.0.0.1:7600 antwortet, sonst der Satz „Die Frames von 6.4a folgen, sobald OpenPencil antwortet“.
- [ ] **Step 4:** `bash scripts/check-tokens.sh` grün; `cargo test -p studio --all-features table_covers_every_format_file` grün; `bash scripts/check-no-rom-data.sh`.
- [ ] **Step 5: Commit** `docs(6.4a): recipes, world map painting, tileset editor; design-system addendum 6.4a` (mit Trailer).

### Task 13: Gate 6.4a, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-6-4a-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 6.4a“ am Ende), `docs/ROADMAP.md` (Zeile 6 der Übersicht, „Stand 6.4a“ unter der Tabelle)

- [ ] **Step 1: Gate-Datei** nach `docs/phase-6-3a-gate.md`: Kopf mit Stand und Commit; Gate-Text (Gate 6.4a und Phasen-Gate wörtlich wie in Global Constraints); „So gelesen“ (Ausgangsstand nach dem Rezept, „neben Zweiblattdorf“ = die Zelle aus Task 10 und das „+“ daneben, Phasen-Gate über die Oberfläche mit Bühne und UI-Spiel, Kürzel G, Palette im Inspector, Altlasten nicht in 6.4a); Legende „Wer“; Namen in der Oberfläche (`[data-paint-tools]`, `data-paint-sub`, `data-stroke-pending`, `[data-paint-palette]`, `[data-palette-tile]`, `[data-paint-layers]`, `[data-layer-row]`, `[data-tile-editor]`, `[data-tileset-save]`, `[data-paint-binary]`, `[data-paint-adopt]`, `[data-paint-instead]`, `[data-budget-refused]`, Verlaufstexte); Vorbereitung (Kopien `6-4a-{pt,hgde,hgus}`, `6-4a-web`, `6-4a-webhg`, `6-4a-szen-{pt,hg}`, `6-4a-buehne-{pt,hg}`; Worktree, Ports 1560/1561, Last).
  - **(1) Automatik (Agent):** `cargo deny --locked check licenses advisories`, fmt, clippy, `env -u G4_TEST_PROJECT cargo test --workspace --all-features`, Zeittest mit `G4_TIMING_STRICT=1`, `pnpm test`/`pnpm build`, Stil, Tokens, Tiefe, `uv run pytest -q` in `scripts/g4emu`; gated Task 10 je Kopie; `vanilla_checks` je Kopie. `scripts/gate.sh`: **offen, Lauf in der Hauptsitzung auf main**.
  - **(2) Szenarien (Agent + Blick):** Task 11 Step 3 und 4 mit Rückgabe, Erwartungen und Bildern.
  - **(3) Browser (Agent):** Task 9 Step 5 mit Zeiten.
  - **(4) Hand und Blick:** Bilder Hell, Dunkel, Kompakt; `out.nds` von `*-gemalt-ui` in melonDS bei Tag und Nacht (Systemzeit des Emulators); DSPRE öffnet die gebaute Karte.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Zelle neben der Startstadt | Striche = Verlaufseinträge | Zeit bis gebaut (Oberfläche) | NSBMD (Anteil) | BDHC | `git status` nach ⌘Z | Dateien gleich (gated) | Paletten-Bilder / Platzhalter | Szenario MCP | Szenario Oberfläche | `vanilla_checks` Fehler | Last |
    |---|---|---|---|---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | leer | | | | | 0 | |
    | HeartGold DE | | | | | | leer | | | | | 0 | |
    | HeartGold US | | – | – | | | – | | | – | – | 0 | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte eintragen; Befunde beheben (eigene Commits `fix(6.4a): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: `agent-gate.md`** Abschnitt „Phase 6.4a“ nach „Phase 6.3b“: Kopien, Worktree, „Wo was ist“ (die Attribute aus Step 1, `__g4SceneProbe.tileToClient`/`strokePending`, Striche per Ziehen), Tabelle Punkt / Wer / Wie, die Befehle für (1), (2), (3) mit erwarteten Werten. **Roadmap:** „**Stand 6.4a <Datum>:** Gate vom Agenten bestanden: „Neue Karte malen“ neben Zweiblattdorf/Neuborkia, gestaltet mit Pinsel, Füllen, Höhe und Rampe und gebaut in <m:ss> / <m:ss>, je Strich ein Verlaufseintrag, ⌘Z bis zum Ausgangsstand `git status` leer, `pt-gemalt`/`hg-gemalt` über `recipe_apply` + `world_edit` begehbar, Phasen-Gate über die Oberfläche in g4emu; MCP 90 Werkzeuge; Branch `phase-6-4a-malen`; [docs/phase-6-4a-gate.md](phase-6-4a-gate.md); beim Nutzer offen: Blick, melonDS bei Tag und Nacht, DSPRE, `scripts/gate.sh` auf main“.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' <BASIS>..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count <BASIS>..HEAD`; `git log <BASIS>..HEAD | grep -c '＠'` gleich 0.
- [ ] **Step 5: Commit** `docs(6.4a): gate of painting tools and recipe, agent-gate section, roadmap` (mit Trailer).

---

## Schnittstellen für die Folgephasen

- **6.4b (Smart Drawing und Muster):** `PAINT_TOOLS`/`paintKey` (die Unterwerkzeuge „Kantensatz“ und „Klippe“ kommen dazu; Tileds Taste T ist heute „Tür“, 6.4b wählt die Tasten), `strokeRequest` (Terrain-Pinsel rechnet ebenfalls auf Zellen herunter), `strokePreview.ts`, `TilePalette` (Kantensätze als eigene Gruppe), `TileEditor` (Eckfarben), `Tileset.edge_sets`, die Schicht „Gelände“ für die Muster-Shader.
- **6.5b (Freiform und Austausch):** `NewPaintedMapInput.source_map` (Kopie einer Welt mit Freiform), `world_layers` (TMJ-Ebenen bilden auf Überlagerungen ab), `world_get` als kuratierte Leseschnittstelle für den TMJ-Export.

## Offen für die Umsetzung

- **Zelle neben der Startstadt:** ob Matrix 0 neben Zweiblattdorf und Neuborkia eine leere Zelle hat, zeigt Task 10; sonst wächst die Matrix (Zelle außerhalb) oder die nächste leere Zelle gilt, die Gate-Datei nennt die Abweichung.
- **Ereignisse in der gewachsenen Matrix:** die Szenarien verschieben Tür und Geschenk um 32 Kacheln; lädt das Spiel sie in Zelle (1, 0) nicht wie erwartet, ist der Rückweg ein Warp aus der Startkarte über ihre Ostkante (Befund mit Bild).
- **Bake-Zeit unter Last:** reißt der Zeittest nur unter Last, allein wiederholen; reißt er allein, bekommt `world_edit` keinen eigenen Weg, sondern die Vorschau bleibt bis zur Antwort und der Befund geht an 6.3b („Laden backt“, Zwischenspeicher je `input_hash`).
- **Format-5-Texturen:** wenn `palette … placeholder > 0` auf einem Vanilla-Paket, eigener Posten nach 6.4b.
- **Parallele Merges:** Konflikte an den Anhängepunkten gesammelt lösen; `.fig` von main behalten; nach dem Merge von 6.5b den Katalog-Zähltest neu zählen.
- **Teil 1 vor 6.3b (Entscheidung der Umsetzung):** Tasks 2–6 laufen auf `7f01683b` ohne 6.3b. In Task 3 bleiben der Test `painted_map_recipe_over_dispatch` und die Katalogtexte zu `new_painted_map` (`recipe_args`, `recipe_apply`) für Teil 2 offen, weil sie Task 1 brauchen; der Zeittest `world-stroke-bridge` misst in Teil 1 `world_edit` ohne Bake und wird nach dem Merge mit Bake wiederholt. In Task 4 kommt `WorldReport.bake` (TS) erst in Teil 2. Wo Task 6 Bake oder Budget berührt, folgt das in Task 7. **Erledigt in Teil 2 (Task 3b):** `painted_map_recipe_over_dispatch`, die Katalogtexte und der Absatz zu `new_painted_map` in `docs/mcp.md` sind da; der Zeittest backt jetzt auf `world_ready()` wirklich (das Modell wird ganz gebaut und erst dann für seine Größe abgelehnt, die BDHC gebacken) und misst allein im Debug-Build `world-stroke-bridge median_ms=49.74 faces=1774` bei Last etwa 20 (Grenze 300 ms).
- **`world_layers` Namensregel (Task 2):** „gibt es schon“ vergleicht die getrimmten Namen exakt (Groß-/Kleinschreibung zählt, „büsche“ ≠ „Büsche“), auch gegen die Bodenebene; eine fehlende Ebene lehnt mit dem Satz von `world_edit` ab („Ebene 2 gibt es nicht (Karte 0123 hat 1).“). Ein Deckel für die Zahl der Ebenen (Index `u8`) fehlt bewusst.
- **Beschreibung von `world_edit` (Task 3):** der Text des Plans verletzt zwei bestehende Katalogregeln (keine Umlaute in Beschreibungen, höchstens drei Sätze, wobei „(. “ als Satzende zählt). Umgesetzt mit drei kleinen Abweichungen: „ramp (level . or …)“, „label? (the history text; default names the map and the number of cells)“ (der deutsche Standardtext steht im Schema von `label`) und „sent as cells; answers cells, change and bake.“ als ein Satz. Die Regeln bleiben unverändert.
- **Malmodell (Task 4):** `strokeRequest` gibt `null`, wenn keine Zelle gesendet wird, auch wenn alle an der Höhengrenze lagen; dafür zählt `heightSkipped(cells, world, lower)` die Zellen an −16 bzw. 62, damit Task 6 „N Zellen schon an der Grenze (−16 bis 62).“ auch ohne Anfrage sagen kann. `cellsOf` schreibt Rampen kanonisch (`e01` → `e1`). `TilesetSaveReport` (TS) trägt alle Felder des Kernels statt nur `package`; `TileJson.model?` ergänzt (6.5b-Feld, vom Kernel geschrieben, wenn gesetzt). `RECIPE_TITLES` führt „Neue Karte malen“ schon in Teil 1 am Ende (wie `RecipeId::all()` in Task 1).
- **Unterwerkzeuge (Task 5):** `PaintModel.target` kommt aus der gewählten Matrixzelle (`useMatrixSelection`) auf der gezeigten Matrix (`useWorkbench().matrix`), `get_map(map).world` und `useTextProject().active`; gelesen neu bei `mapModelReload`/`mapRefetch` dieser Karte und `matrixData`. Der `PaintProvider` sitzt in `Workbench.tsx`; die Kürzel in `App` lesen ihn über eine Ref (`Workbench paint={ref}`), weil `useShortcuts` über der Werkbank läuft. Die Statusleiste zeigt „Gelände malen · <Unterwerkzeug> · …“. Neue Symbole aus Phosphor: `rectangle` (Rectangle), `bucket` (PaintBucket), `eraser` (Eraser), `eyedropper` (Eyedropper), `ramp` (Stairs); `heights` (Mountains) und `paint` (PaintBrush) gab es. Starttile des Zustands `0000` bis die Palette (Task 7) wählt.
- **Striche und Sitzung (Task 6):** Die Vorschau eines Strichs bleibt nach dem Loslassen unter einer Kennung stehen (`holdStrokePreview`), bis seine eigene Antwort kommt (`clearStrokePreview(h, id)`); so bleibt bei zwei Strichen in der Warteschlange die Vorschau des zweiten. `PreviewLook` nimmt für Höhe und Rampe eine Farbe je Zelle (`{color: (cell) => …}`, Zielstufe aus dem Weltdokument); Pinsel, Rechteck und Füllen zeigen bis zum Palettenspeicher (Task 7) die Platzhalterfarbe der Rolle der Kachel. Die Anfrage eines Strichs entsteht beim Loslassen aus dem zuletzt gelesenen Dokument mit den noch unbeantworteten Strichen darüber (zwei schnelle „+“ ergeben +2). `usePaintSession` hat einen fünften, optionalen Parameter `{scene, onNotice}` (Host der Vorschau; „N Zellen schon an der Grenze …“ als Hinweis über `onStatus`). Rechtsklick nimmt in jedem Unterwerkzeug die Kachel auf, nur die Pipette wechselt danach zum Pinsel; ein Druck mit ⌥ dreht/verschiebt die Kamera statt zu malen. Die Schicht „Höhen“ baut `moveMapGround` nach dem Karten-Refresh schon neu, `heightLayer.ts` blieb unverändert. `PaintModel.set` nimmt auch eine Funktion (`set((s) => …)`), die Kürzel nutzen sie; `PaintModel.world` trägt das Weltdokument der Sitzung für die Tüfteln-Statusleiste („Kachel 0041r1 · Ebene 0 · Höhe 2 (16 E)“).
- **Rezept `new_painted_map` (Task 1):** Das Backen im selben Schritt fand die neue Zelle nicht (`world.no-area`), weil `bake_world` die Matrizen des Stores liest; `Tx::edit_world` backt jetzt mit den im Schritt gestageten Matrizen und der Header-Tabelle (`Tx::with_staged_layout`, nur für den Bake getauscht und zurückgetauscht). Das Paket eines gewählten oder geerbten Headers kommt direkt aus dessen Gebietsdaten (`map_tileset_index`, gleich dem Eintrag in `headers_of_map(Nachbar)`, aber auch für einen `header`, der den Nachbarn nicht zeigt); ohne Header-Spalte gilt der erste Header aus `headers_of_map(Nachbar)`. Mit `source_map` ist `Draft.painted` leer (die Notiz sagt „Die Welt kommt aus Karte 0000.“), `report.tileset` ist das Paket der Quellwelt, der Header der Zelle der des Nachbarn oder `header` (ohne Nachbar `None`). Höher als 255: „Die Weltkarte würde höher als 255 Zellen.“ Die Katalogtexte (`recipe_args`, `recipe_apply`) und die Zähltests in Kernel und Studio sind hier nachgezogen, weil die Tests sonst rot wären; `docs/mcp.md` und die Frontend-Sperre „Kommt mit Phase 6“ (`workbench/recipes.ts`) bleiben für die Folgetasks.
- **Palette, Ebenen, Budget (Task 7):** `PaintModel` trägt zusätzlich das Tileset der Sitzung (`tileset`/`publishTileset`, nur zum Paket der gezeigten Welt) und die in der Szene verborgenen Ebenen der Zielkarte (`hidden`/`toggleHidden`, mit einer anderen Zielkarte vergessen); `MatrixScene` blendet sie über `setLayerVisible` aus und zeigt sie beim Verlassen von Karte oder Werkzeug wieder, ein später geladenes Netz (Strich, ⌘Z) behält sie verborgen (Merkliste je `SceneHost` in `worldMesh.ts`). Ob „Gelände malen“ aktiv ist, sagt `MatrixCellContext.painting` (`panels.tsx`: `d.tool === "paint"`); die Abschnitte stehen ab Bauen. `TilePalette` und `LayerList` haben je einen optionalen Zusatz (`image`, `edit`), damit die Galerie ohne Projekt nichts liest und nichts schreibt; `LayerList.onChanged` (im Inspector `onMapImported`) lässt nach „Ebene hinzufügen“/„Entfernen“ Szene, Welt und Verlauf nachziehen. Eine Kachel aus der Palette wird ohne Drehung und Spiegelung gewählt; aus Radierer, Pipette, Höhe und Rampe geht es dabei zum Pinsel (Tiled: Klick ins Tileset nimmt den Stempel), Rechteck und Füllen bleiben. Wird die aktive Ebene entfernt, malt der Pinsel wieder auf Ebene 0. Bilder: RGBA → `ImageData` → `createImageBitmap` (ohne 2D-Canvas), die Taste zeichnet sie in ein Canvas ohne Glättung; die Strichvorschau nimmt das Bild aus demselben Speicher (`peekTileImage`), solange es fehlt die Platzhalterfarbe; das Bild der gewählten Kachel wird vorab geholt. Neue Symbole `eye` (Eye) und `eyeSlash` (EyeSlash); „Zeigen“ ist gedrückt (`aria-pressed`), solange die Ebene verborgen ist. Der Satz „Nicht gebacken: …“ nimmt die erste Meldung der Stufe `error` (sonst die erste) und ergänzt einen fehlenden Schlusspunkt.
- **Nachbesserung Task 7:** Der Palettenspeicher schließt beim Zurücksetzen keine Bitmaps mehr (eine Palette oder Vorschau kann sie noch zeichnen) und meldet den Reset (`onTileImagesReset`, `tileImagesGeneration`); die Palette fragt dann neu. Nach „Entfernen“ von Ebene i (`LayerList.onRemoved` → `PaintModel.layerRemoved`) rücken die Verbergen-Marken und die aktive Ebene über i um eins nach unten; war i aktiv, wird die Ebene darunter aktiv (mindestens 0). Eine andere Zielkarte vergisst die verborgenen Ebenen. Die Strichvorschau spiegelt v der Bitmap (WebGL ignoriert `flipY` bei `ImageBitmap`), damit die erste Zeile wie bei `toTexture` bei v = 1 liegt, und filtert ohne Glättung (min und mag); ob die Ausrichtung zum Netz passt, prüft der Browser-Teil (Task 9). „Aktiv“ ist ein natives `<input type="radio">`: kein Baustein bietet ein einzelnes Radio je Zeile (`Segmented` ist eine Toggle-Gruppe für eine Zeile), Farbe über `accent-color` und Token.
