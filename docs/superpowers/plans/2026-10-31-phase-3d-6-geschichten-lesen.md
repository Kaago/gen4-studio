# Phase 3d.6 „Geschichten lesen“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Der Atlas-Bereich „Geschichten“ wird frei: Er zeigt die Fäden des Spiels (Variablen mit mindestens drei Werten an mindestens zwei Orten, aus dem Referenzindex von 3d.1, ohne zweiten Parser) als Übersicht mit Werten und Orten und je Faden einen Graph mit einem Knoten je Wert und Kanten „gesetzt in → geprüft in“; eine Geschichte ist eine Projekt-Entität des Textprojekts (`data/stories/NNNN.json` mit Name, Notizen, `progress.var` oder `progress.flag`, Schritten mit `value`, `kind`, `name`, `made.recipe`, `uses`; Anordnung im Beifile `NNNN.layout.json`; Register `data/ids/stories.json` mit `STORY_…`), angelegt und geändert über Oberfläche, MCP (`story_list`, `story_get`, `story_create`, `story_update`, `list_threads`, `thread_graph`) und `g4k`, als ein Undo-Schritt, mit Feld-Diff (`DiffSubject::Story`), mitgezogen von „ID umbenennen“, ohne Bytes im Build (`text_project_in_git_builds_from_base_byte_equal` mit einer Geschichte grün). Teil B (nach dem Merge von 3d.4): Geschichten als **schwache** Verweise beim Löschen (warnen mit Liste, nie sperren), Löschen einer Geschichte als „Datei entfernen“ mit Undo, Prüfung `story.dangling-ref` auch für Grabsteine, Gate und Doku.

**Architecture:** Kernel: der Referenzindex (`data/refs.rs`, `data/refs/flagvars.rs` aus 3d.1) bekommt je Verweis den Wert, den ein Befehl setzt oder vergleicht (`Ref.value: Option<u16>`; Regeln deklarativ in `metadata/script_flagvars.yaml`, Abschnitt `values`; Auslöser `value`, Level-Script `var_conditions[i].value`). Ein neues Modul `crates/g4-kernel/src/stories/` trägt die Entität (`schema.rs`: `Story`, `Step`, `Use`, `Layout`, Kanon, Prüfung), die Art-Beschreibung als dritter `DataKind` (`textproject/kinds.rs`, Register `STORY_`, `SymbolKind::Story`), Anlegen und Ändern als `FileStep` (`entities.rs`, Muster `flags/entities.rs`), die Fäden (`threads.rs`: Fäden-Regel und Graph aus dem Index) und in Teil B die schwachen Verweise (`uses.rs`) und das Entfernen. Diff: `DiffSubject::Story { index }` liest beide Seiten als JSON (`EntityDiff::Fields`). App: Befehle in `commands/stories.rs` (Tauri, Brücke, `dispatch.rs`), MCP-Werkzeuge im Katalog, `g4k story …`; Frontend neuer Ordner `apps/studio/src/stories/` mit dem Bereich `GeschichtenArea` (eine Fläche wie Technik, kein Deck: Liste „Fäden“, „Geschichten“, „Flags und Variablen“ (zieht aus Technik um), rechts die Fläche), dem Graph über `@xyflow/react` mit `@dagrejs/dagre` als nachgeladener Chunk (`React.lazy`), den Knoten als Design-System-Baustein `StoryNode` (sieben Arten), dem Dialog „Neue Geschichte“ und dem Detail einer Geschichte (Notizen, Schritte, Graph). Lizenzen: `cargo deny` bleibt, dazu ein npm-Lizenzschritt `scripts/check-licenses-npm.sh` in `scripts/gate.sh`.

**Tech Stack:** Rust (g4-kernel, serde_json mit `preserve_order`, serde_yaml für `script_flagvars.yaml`, rotom 0.7.0 über `scripts::visit`, uxie für Events), Tauri-Befehle, MCP (Befehlstabelle, Katalog, Zugriff, symbolische Argumente), React 19 mit den Bausteinen aus `apps/studio/src/ui/`, `@xyflow/react` 12 (MIT) und `@dagrejs/dagre` 1 (MIT), vitest (jsdom mit `ResizeObserver`-Attrappe aus `vitest.setup.ts:9`), Bash (`pnpm licenses list --prod --json`, python3 `-I` zum Prüfen der Liste).

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) §1, §2 (Zeile „Handlung im Spiel ist Zustand“: Fäden 59 / 62), §3.1 (Geschichte, Modell (c), Knotenarten, Fortschritt, Erscheinen/Verschwinden, schwache Verweise, DSPRE-Ordner), §3.2 (nur zum Verständnis), §4 Zeile 3d.6 (das Gate, gilt wörtlich) und Zeile 3d.7 (damit 3d.6 die Grundlage legt), §6, §7, §8; [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Tabellen „Phasenschnitt und Reihenfolge“ Zeilen 3d.6 und 3d.7, „Geschichten und Erzählen (Gutachten A)“ wörtlich, „Nicht in 3d“, „Was bewusst nicht entschieden ist“ zu den Knotenarten); [gutachten-a-geschichten-erzaehlen.md](../../research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md) §2.2 (Messung der Fäden, `gutachten-a-story.py` `var_threads`), §2.3 („Was der Code heute trägt“), §3.1, §3.2 (Modelle a/b/c, die Beispiel-JSON von (c) ist die Vorlage der Entität), §5 (G1 mit Gate), §6 (Fragen 2, 4, 5); [Plan 3d.1](2026-10-30-phase-3d-1-flags-variablen.md) (Tasks 2 bis 5: `DataKind`, `FileStep` mit `HEADER_TABLE`, `Ref.access`, `usage`, `flag_usage` mit Header je Verweis; der Stand im Worktree `phase-3d-1` ist die Quelle der Schnittstellen); [Plan 3d.3](2026-10-30-phase-3d-3-sprites-ds-eier.md) als Muster für Aufbau und Gate.

Stand 31.10.2026: `main` bei `fe6d591` (3d.2 gemergt: Tiefe Erzählen, `depthAtLeast`/`viewDepth`, `scripts/check-depth.sh`, Rezept-Rahmen `crates/g4-kernel/src/recipes/`, `FieldSpec.from`). **Basis: main nach dem Merge von 3d.1** (Worktree `.claude/worktrees/phase-3d-1`, Kopf `9903a38` plus 24 noch nicht committete Dateien, siehe Befund B13). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-6`, Branch `phase-3d-6-geschichten-lesen` (legt der Orchestrator nach dem 3d.1-Merge an). Parallel laufen in eigenen Worktrees 3d.4 (Löschen) und 3d.5 (Figur, Dialog, Tür); 3d.3 Teil B (Ei-Attacken) wartet ebenfalls auf 3d.1. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Befunde vor dem Plan (am Code geprüft, Zeilen von `main fe6d591` bzw. dem Worktree `phase-3d-1`)

- **B1 Der Index kennt keine Werte.** `Ref { source, field, slot, target, access }` (`phase-3d-1: crates/g4-kernel/src/data/refs.rs:117-123`); `script_flagvar_refs` (`refs/flagvars.rs:450`) meldet über `note()` (`:357`) nur das Ziel; `command()` (`:387`) liest nur den Parameter des Flags oder der Variable, `condition()` (`:407`) nur die Operanden. Auslöser (`refs.rs:794-796`) und Level-Scripts (`:1248-1253`) stecken ihren `value` nicht in den `Ref`. „Ein Knoten je Wert“ braucht den Wert aus dem Index: Task 1 hängt `Ref.value: Option<u16>` an.
- **B2 Die Fäden-Messung** (`gutachten-a-story.py:471-535`, `var_threads`): Werte aus `SetVarFromValue`, `CompareVarToValue`, `SetVar` (zweites Argument als Zahl), `var_conditions[i].value` und Auslöser `value`; Orte = Header über `script_file_id`, `level_script_id`, `event_file_id`, sonst die Datei selbst; Faden = Variable `0x4030 ≤ n < 0x8000`, deren pret-Kategorie nicht temporär, Objektgrafik, Konstante oder `UNUSED`/`DUMMY` ist, mit ≥ 3 Werten und ≥ 2 Orten: 59 (Platin DE), 62 (HeartGold DE). Task 3 baut dieselbe Regel auf dem Index.
- **B3 Es gibt keinen npm-Lizenzschritt.** `scripts/gate.sh` prüft nur `cargo deny` (`:105`); `pnpm licenses list --prod --json` liefert heute `MIT: 40, OFL-1.1: 4, "Apache-2.0 OR MIT": 1, "MIT OR Apache-2.0": 1`. `deny.toml` erlaubt MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, CC0, 0BSD, MIT-0 u. a.; EPL-2.0 (elkjs) fehlt. Task 6 legt den Schritt an.
- **B4 Arten mit Member sind ein `pub(crate) enum Art`** (`textproject/arts.rs:31-46`, 16 Einträge mit `kinds()`); Arten ohne Member meldet 3d.1 über `textproject/kinds.rs` an (`DATA_KINDS: [DataKind; 2]`, `:53`; `check_files` `:102` ruft `check_kind`, das jede Datei als `FlagEntity` liest, `:150-200`). Die Geschichte wird der dritte `DataKind`; `check_kind` muss je `symbol` verzweigen (Task 2).
- **B5 `FileStep` braucht ein `AssetId`** (`store.rs:505-515`); 3d.1 nimmt für `name_flag` `HEADER_TABLE` (`store.rs:940`) mit `FileStepKind::IdRename` (`phase-3d-1: flags/entities.rs:332-336`). Die Geschichte macht es ebenso, mit `FileStepKind::Reset` (keine Folgen für ID-Dateien; `store.rs:517-532`), damit `store.rs` unberührt bleibt (3d.4 baut dort den Grabstein).
- **B6 `sources()` und „ID umbenennen“ laufen über ganz `data/`:** `check.rs:67` listet alles unter `data/`, `rename.rs:424` ersetzt jede JSON-Zeichenkette unter `data/` (ohne `ids/` und `raw/`). Eine Geschichte mit `TRAINER_0042` zieht beim Umbenennen des Trainers ohne neuen Code mit; die eigene ID `STORY_…` braucht `kind_of` (`rename.rs:261`), `REGISTRY_KINDS` (`phase-3d-1: registry.rs:24-31`, sechs Einträge) und `registry_file`.
- **B7 Der Diff kennt Dateien nur als Zeilen:** `DiffSubject::File` (`diff/subject.rs:44`) wird `Kind::File` (`diff/side.rs:413`) und `EntityDiff::Lines` (`diff.rs:364`); JSON-Seiten werden `EntityDiff::Fields` über `json_changes` (`diff.rs:326-334`, `diff/json.rs:96`). `of_path` (`subject.rs:263`) ordnet `data/…` über `entity_of_data_file` zu. Der Frontend-Typ `DiffSubject` (`api.git.ts:23-45`) und `EntityDiff.tsx:138` (Feldname = Pfad) kommen ohne Umbau aus.
- **B8 Der Bereich ist gesperrt und kein Erzählen-Bereich:** `App.tsx:125-127` (`LOCKED_AREAS = ["geschichten"]`, „Kommt mit Phase 3d“), `shellState.ts:27` (`NARRATING_AREAS = ["welt"]`), `layout.ts:20` (`DeckArea` ohne `geschichten`); Flächen ohne Deck rendert `App.tsx:1275-1310` (`start`, `technik`). `Atlas.test.tsx:14` nennt noch „Kommt mit Phase 3b.3“ (eigene Attrappe, bleibt).
- **B9 „Flags und Variablen“ liegt in Technik:** `phase-3d-1: technik/TechnikArea.tsx:120-130` (`ENTRIES` mit `flags`), Mount `:254`, Komponenten unter `apps/studio/src/flags/` (24 Dateien); die Entscheidung von 3d.1 sagt „mit 3d.6 zieht der Eintrag in den Bereich Geschichten“.
- **B10 Befehle und Katalog:** `dispatch.rs:48` `COMMANDS` (anhängen), `access.rs:218-221` (`recipe_*` als Muster), `catalog.rs` (`name: "…"`, Test `the_curated_set_is_the_decided_one` `:1708`), `symbolic.rs:58-65` (`FromKindField`), `api.ts:1048-1060` (`export * from`), `g4k.rs:120` (`Command::Text(TextCommand)`, `:1373`, `:1493`). Namen im Katalog sind verb-zuerst (`list_flags`, `create_trainer`) und objekt-zuerst (`data_get`, `recipe_plan`); 3d.1 bringt `list_flags`, `flag_usage`, `free_flag`, `name_flag`.
- **B11 Der `vitest.setup.ts:9-14` legt eine `ResizeObserver`-Attrappe** (React Flow braucht sie), `check-styles.sh:13,54` prüft nur `apps/studio/src` (eine CSS-Datei aus `node_modules` ist frei), `check-depth.sh` verbietet Vergleiche der Tiefe.
- **B12 `text_project_in_git_builds_from_base_byte_equal`** (`tests/git.rs`) verlangt, dass jeder Index-Pfad in `sources()` liegt und der Build byte-gleich bleibt; `data/stories/*` kommt über B6 in beide Listen, der Build fasst `data/stories` nie an (kein `Art`, kein Member).
- **B13 Der 3d.1-Worktree hat 24 nicht committete Dateien** (`git diff --stat`: `kinds.rs` „schemas never lower a newer version“, `include.rs`, `flagvars/tests.rs`, `flags.rs` …); sein Test `schemas_name_every_kind_and_keep_foreign_keys` (`kinds.rs:304-307`) nimmt **`stories`** als Beispiel eines fremden Schlüssels. Nach dem Merge gilt der gemergte Stand; Task 2 ändert dieses Beispiel (`"plugins"` statt `"stories"`).
- **B14 Auf main gibt es noch kein `plan_delete`** (`git grep -l plan_delete main -- …dispatch.rs` leer); das ist das Kennzeichen von 3d.4 für Task 9.

## Entscheidungen (wo Spezifikation und Gutachten offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Entität | `data/stories/NNNN.json` nach Gutachten A §3.2 (c): `{ id, name, notes?, progress: { var } \| { flag }, steps: [ { key, value, kind, name, notes?, made?: { recipe, studio }, uses: [ … ] } ], flags: [ … ] }`; `kind` englisch `start`, `dialog`, `condition`, `battle`, `reward`, `progress`, `appearance` (die sieben Arten; `appearance` deckt Erscheinen und Verschwinden, die Richtung steht im Namen des Schritts); `uses` je Eintrag genau ein Schlüssel: `trainer` (`TRAINER_…`), `map` (`MAP_…`), `flag` (`FLAG_…`), `var` (`VAR_…`), `object { file, local_id }`, `script { file, slot }`, `message { archive, index }`, `levelScript { file, condition }`; alle Typen `deny_unknown_fields`, kanonisch über `canon::to_canonical`; `value` ist `u16`, Schlüssel `key` eindeutig je Geschichte | Spezifikation §3.1 wörtlich; englische Bezeichner wie alle JSON-Schlüssel des Textprojekts (`hidden_flag`, `coord_events`), die deutschen Wörter trägt die Oberfläche (`storyWords.ts`) |
| Beifile | `data/stories/NNNN.layout.json` = `{ "nodes": { "<key>": { "x": 0, "y": 0 } } }`; beim Anlegen leer (`{ "nodes": {} }`), ein fehlendes Beifile ist kein Fehler (automatisches Layout); nie Teil des Diffs der Bedeutung (ein eigener `DiffSubject::File`) | Gutachten A §3.2 „damit das Ziehen von Knoten den Diff der Bedeutung nicht verrauscht“; das Gate verlangt, dass `git status` JSON **und** Layout zeigt |
| Art-Beschreibung | dritter Eintrag in `DATA_KINDS` (`[DataKind; 3]`): `{ folder: "stories", symbol: SymbolKind::Story, schema_version: 1, ref_fields: &["progress.var", "progress.flag", "steps[].uses[]", "flags[]"], delete: DeleteRule::RemoveFile }`; `check_kind` verzweigt auf `kind.symbol`: `Flag`/`Var` wie bisher, `Story` → `stories::schema::check_file`; `manifest_for` schreibt `schemas.stories: 1` | Spezifikation §3.2 „die neuen Projekt-Arten (Flag, Variable, Geschichte) melden sich über eine Art-Beschreibung als Daten an“; keine Store-`AssetKind` (siehe nächste Zeile) |
| Keine `AssetKind::Story` | die Geschichte ist **kein** Store-Asset: `AssetKind::ALL` (`project.rs:96`, 28 Slots) zählt Archive, Build und Pack; ein Asset ohne Member bräuchte eine neue `Location`-Art im Store, den 3d.4 zeitgleich umbaut. Anlegen, Ändern und (Teil B) Entfernen sind je ein `FileStep` (`asset: HEADER_TABLE`, `kind: FileStepKind::Reset`, Dateien Register, JSON, Layout) über `Store::record_file_step`; „ungespeichert“ gibt es für Geschichten nicht (geschrieben beim Schritt, wie `name_flag` und `rename_id`) | die Spezifikation sagt „Store-Art `Story` (kein Member, kein Build-Schritt)“; das Ziel (⌘Z, Verlauf, Label) erreicht der Dateischritt ohne Umbau von `store.rs` und `project.rs` (B5); die Abweichung steht in `docs/formats/stories.md` |
| ID und Register | `SymbolKind::Story` (Präfix `STORY`, keine feste Tabelle, `fallback_id` `STORY_0000`), Register `data/ids/stories.json` (`REGISTRY_KINDS` 7, `registry_file` `stories.json`, `include::KINDS` **nicht**: rotom kennt keine Geschichten, keine ID-Datei), Nummer = kleinste freie ab 0, vierstellig; ID aus dem Namen über `symbols::normalize(name, german)` mit Präfix, Regeln von `rename::format_text` (A–Z, 0–9, `_`, ≤ 64), bei Kollision `_2`, `_3` …; `rename_id STORY_X` ändert Register und `id` der Datei; `kind_of("STORY_…") == Some(Story)` | Spezifikation §3.1 „Register `data/ids/stories.json` (`STORY_…`)“; Taufe wie Trainer (`baptize` nie, weil es keine Member gibt) |
| Werte im Index | `Ref.value: Option<u16>` (angehängt, `Copy` bleibt): bei einem `Var`-Verweis der konstante Wert, den der Befehl setzt oder vergleicht (`script_flagvars.yaml` je Familie neuer Abschnitt `values: { <Befehl>: <Parametername> }`, mindestens `SetVar`, `SetVarFromValue`, `CompareVarToValue` (pt) und die HGSS-Gegenstücke, die `script_flagvars_name_existing_commands_and_params` gegen die Datenbank prüft), bei einer Bedingung `VAR_X == 3` (jeder Vergleichsoperator) der Literal-Operand, am Auslöser `value`, im Level-Script `var_conditions[i].value`; ein Operand, der selbst eine Variable ist (`≥ 0x4000`) oder nicht auflöst, gibt `None`; Flags immer `None` | B1; die Messung (B2) nimmt genau diese Stellen; ein Wert ist 16 Bit |
| Fäden-Regel | `threads()` = jede Variable mit `area_of(family, Var, n) == General` (ab `0x4030`), deren pret-Hauptname fehlt oder weder `UNUSED` noch `DUMMY` enthält (`UNK` zählt), mit ≥ 3 verschiedenen Werten über alle Verweise mit `value` (gesetzt oder geprüft) und ≥ 2 verschiedenen Orten; Ort = der erste Header, dessen `script_file_id`, `level_script_id` oder `event_file_id` die Quelle nennt (Muster `Users::of`, `phase-3d-1: commands/flags.rs:189-199`), sonst die Quelle selbst (`AssetId`); sortiert nach Nummer; eine Geschichte, deren `progress.var` die Nummer ist, wird am Faden genannt (`story: Option<String>`) | B2 wörtlich; Platin ≥ 59, HeartGold ≥ 62 als Untergrenze (der Index löst Aliasse und Konstanten mehr auf als `int(args[1], 0)`) |
| Graph eines Fadens | Knoten: ein `Value`-Knoten je Wert (`id "v:<wert>"`) und ein `Place`-Knoten je Quelle (`id "p:<kind>:<index>"`, Label der Ortsname oder „Scriptdatei 0101“); Kanten: je Verweis mit `value` und `Write`/`ReadWrite` eine Kante Ort → Wert („gesetzt in“), je `Read`/`ReadWrite` eine Kante Wert → Ort („geprüft in“), beschriftet mit `field` und `slot` (Zeile, Objekt, Bedingung); Verweise ohne Wert (`SetVar VAR_X, VAR_Y`) erscheinen als Ort-Knoten ohne Kante zu einem Wert (`loose: true`), nie als eigener Wert; Lage aus `.layout.json`, wenn die Geschichte eine hat, sonst dagre (`rankdir LR`, Ränge: Orte links, Werte Mitte, Orte rechts) | Gate „ein Knoten je Wert und Kanten gesetzt in → geprüft in“; Gutachten A §3.2 „Kanten aus dem Index“ |
| Graph einer Geschichte | derselbe Graph ihrer `progress`-Nummer (Variable; ein Flag hat die Werte 0 und 1); die Schritte legen sich auf die Wert-Knoten (`step: Some(key)`: Name und Art des Schritts im Knoten, `StoryNode` in der Art des Schritts statt `value`); ein Schritt, dessen Wert der Index nicht kennt, bekommt einen Wert-Knoten ohne Kanten mit der Plakette „nur notiert“ (gestrichelt) | Gutachten A §3.2 (c) „nur notierte Kanten erscheinen gestrichelt“ |
| Graph-Bibliothek | **`@xyflow/react` 12 (MIT) mit `@dagrejs/dagre` 1 (MIT)**, beide als genaue Version in `package.json` (die neueste 12.x bzw. 1.x beim Umsetzen, `pnpm view <paket> version`; `@xyflow/react` ≥ 12.3 wegen React 19). Lizenz: MIT wie 40 der 46 heutigen Pakete, keine neue Lizenz in der Liste; `dagre` 0.8.5 (MIT, unverändert seit 2019) wird **nicht** genommen, `@dagrejs/dagre` ist der gepflegte Fork (1.x seit 2023, ohne lodash). Größe: nur im Graph-Chunk (`React.lazy(() => import("./ThreadGraph"))`), das Hauptbündel wächst um 0; die Chunk-Größe druckt `pnpm build` (Erwartung ≈ 50–70 kB gzip für beide zusammen, gemessen in Task 6, in die Gate-Tabelle). Rendering: DOM/SVG, kein Canvas, eigene Knoten als React-Komponenten (`StoryNode`), Kanten als SVG-Pfade; nur `@xyflow/react/dist/base.css` (Maße, keine Farben), alle Farben aus Tokens. Layout: dagre (Sugiyama für gerichtete Graphen, Ränge und Kreuzungsminimierung). **Alternative:** ein eigenes SVG-Layout in drei Spalten (Orte, Werte, Orte; Fäden haben höchstens 16 Werte und 29 Orte): ohne Abhängigkeit, aber ohne Zoom, Ziehen und Minimap, die 3d.7 für Geschichten mit vielen Schritten braucht; `elkjs` fällt weg (EPL-2.0 nicht in der Allow-Liste, 1,4 MB) | Entscheidungen „Graph-Bibliothek“ (React Flow + dagre nach Lizenzprüfung, Rückfall eigenes SVG); das Design-System nennt React Flow (`README.md:167`) |
| npm-Lizenzen | neues `scripts/check-licenses-npm.sh`: `cd apps/studio && pnpm licenses list --prod --json` durch `python3 -I` gegen die Allow-Liste `MIT`, `Apache-2.0`, `ISC`, `BSD-2-Clause`, `BSD-3-Clause`, `0BSD`, `CC0-1.0`, `Unlicense`, `OFL-1.1`, `MPL-2.0`, `Zlib` (ein `A OR B` gilt, wenn eine Seite erlaubt ist; `AND` nur, wenn beide erlaubt sind); jede andere Lizenz nennt Paket und Lizenz und endet mit 1; `scripts/gate.sh` ruft es als Schritt `npm licenses` direkt nach `cargo deny` | Gate „`cargo deny` und die npm-Lizenzen grün mit der Graph-Bibliothek“; B3 |
| MCP-Namen | `story_list` (Read), `story_get` (Read), `story_create` (Write), `story_update` (Write), `list_threads` (Read), `thread_graph` (Read); in Teil B `story_delete` (Write, ein Undo-Schritt) | objekt-zuerst wie `recipe_*`, `data_*` für die Entität; verb-zuerst wie `list_flags`/`flag_usage` für die Abfragen (B10); `story_set` aus den Entscheidungen wird zu `story_create` und `story_update`, weil der Katalog Anlegen (`create_trainer`) und Ändern (`data_edit`) trennt und das Label des Verlaufs sagen muss, was geschah |
| Oberfläche des Bereichs | `GeschichtenArea` als Fläche ohne Deck (wie Technik, `App.tsx:1290` als Muster): `ListPane` „Geschichten“ mit den Einträgen „Fäden“ (`threads`), „Geschichten“ (`stories`), „Flags und Variablen“ (`flags`, der Eintrag und der Mount ziehen aus Technik hierher, `FlagsSurface` bleibt unter `apps/studio/src/flags/`); rechts die Fläche; `NARRATING_AREAS` wird `["welt", "geschichten"]`; in Erzählen zeigt die Fäden-Übersicht nur Namen (Variable als `displayName`), ab Bauen dazu die Nummer (`tech`); Tüfteln ändert nichts weiter | ein Deck (Dock, `AREA_PANELS`, Schau-Panels) ist der Umfang von 3d.7/3d.8; Entscheidung 3d.1 „mit 3d.6 zieht der Eintrag in den Bereich Geschichten“ |
| Geschichte in der Oberfläche | Liste der Geschichten (Name, Fortschritt als `Reference`-Chip, Zahl der Schritte) mit „Neue Geschichte…“ (Dialog: Name, „Fortschritt“ als `FlagField` der Art Variable mit Umschalter „Flag“, Notizen; gesperrt im DSPRE-Layout mit „Als Textprojekt einrichten…“); Detail: Kopf mit Name und Chip, Notizen (`TextField` mehrzeilig, `story_update` beim Verlassen), Tabelle der Schritte (Wert, Art als Plakette, Name, Verweise als Chips; **nur lesen**), darunter der Graph. Schritte anlegen und ändern geht in 3d.6 nur über MCP/`g4k` (`story_update` mit `steps`) | Spezifikation §4 Zeile 3d.7: die Rezepte legen Schritte an; 3d.6 ist „Geschichten lesen“ |
| Schwache Verweise (Teil B) | `stories::uses::uses_of(project, target: &UseTarget) -> Vec<StoryUse { story_id, story_name, step_key, step_name, file }>` liest die Geschichten-Dateien (keine Quelle im `RefIndex`: dessen Quellen sind `AssetId`s, eine Geschichte hat keine); 3d.4s `plan_delete` bekommt im Report das Feld `mentions: Vec<StoryUse>` (warnt, sperrt nicht), `delete_entity` schreibt dieselbe Liste als `warnings` in die Antwort; die Prüfung `story.dangling-ref` (Warnung) meldet jeden `uses`-Eintrag, dessen Ziel fehlt oder (ab 3d.4) einen Grabstein trägt | Entscheidung „schwach: warnen mit Liste, nicht sperren“; ≤ Dutzende Dateien, ein Scan kostet nichts |
| Geschichte löschen (Teil B) | `story_delete { id }`: ein `FileStep`, der JSON, Layout und den Registereintrag entfernt (`FileChange { after: None }`), Label „Geschichte „X“ gelöscht“; ⌘Z stellt alles her; kein Grabstein, keine Nummernfreigabe (die Nummer bleibt frei und wird wiederverwendet, weil nichts darauf verweist) | Spezifikation §3.4 „eigene Orte, Geschichten, eigene Flag-Namen: Datei entfernen“ |
| Diff | `DiffSubject::Story { index }` (Gruppe „Geschichten“, Label „Geschichte 0000“ bzw. der Name), beide Seiten JSON der Datei (Platte, `HEAD`, Commit; gegen die Basis abgelehnt mit `NO_BASE_FOR_FILES` wie `File`), Form `Fields`; `of_path("data/stories/0000.json")` → `Story`, das Layout bleibt `File`; TS `DiffSubject` + `{ kind: "story"; index }` | Gate „Diff zeigt Feldnamen“ (B7) |
| `g4k` | `g4k story list <project>`, `g4k story get <project> <id>`, `g4k story create <project> --name … (--var VAR_… \| --flag FLAG_…) [--notes …]`, `g4k story threads <project> [--json]`, `g4k story graph <project> <VAR_…>` (JSON); ohne Store (`stories::entities::create_files`), wie `g4k text rename-id` | das Gate läuft kopflos; Muster `TextCommand` |
| Schema-Version | `schemaVersion` des Manifests bleibt 1 (3d.4 hebt auf 2); `schemas.stories: 1` kommt dazu; der Snapshot `schema.snap` bleibt (Geschichten sind keine `Art`) | 3d.1-Muster „Art-Beschreibung“ |
| Nicht in 3d.6 | Rezepte (Neue Geschichte usw., 3d.7), Schritte in der Oberfläche anlegen oder ändern, Pool-Anzeige, Knoten im Graph ziehen und speichern (das Layout wird gelesen, aber erst 3d.7 schreibt es aus der Oberfläche), Bewegungen als Knoten, Geschichten als Quelle im `RefIndex` oder in „Verwendet in“ eines Trainers, ein Deck für den Bereich, Erzählen-Layouts | Entscheidungen „Nicht in 3d“ und Schnitt |

## Global Constraints

- Arbeitsort ist der Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-6`, Branch `phase-3d-6-geschichten-lesen` (Basis main nach dem Merge von 3d.1); nie `cd` in die Hauptkopie, nie Branch wechseln, nie nach main mergen (Ausnahme: Task 9 „Main mergen“ holt main **in den Worktree**); kein `git stash`. `tools/` ist dort ein Symlink (nie einchecken).
- cargo braucht `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`; `pnpm install` einmal in `apps/studio` (Task 6 ändert `package.json` und `pnpm-lock.yaml`).
- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`).
- Originale unter `$HOME/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur `Roms/_testkopien/3d-6-<zweck>` (`cp -cR` für `3d-6-<zweck>_DSPRE_contents`, `cp -c` für `3d-6-<zweck>.nds`, nebeneinander). `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut. Gated Tests lesen `G4_TEST_PROJECT` und schreiben nur auf `vanilla_copy()` (`test_support.rs:649`), die Fäden-Tests übernehmen die Scripts einmal je Lauf (`OnceLock`, Muster `tests/flags.rs:39`). Keine ROM-Daten ins Repo; Fixtures synthetisch (`TextFixture::exported()`, `test_support.rs:803`).
- Test-Umgebung nur auf `G4_VITE_PORT=1760 G4_BRIDGE_PORT=1761` (`bash scripts/agent/studio-web.sh`); andere Ports nie anfassen.
- UI nur aus `apps/studio/src/ui/primitives` und Tokens `--g4-*`, keine Hex-Farben, keine Inline-Styles (`bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh`); deutsche Oberfläche, englische Bezeichner; höchstens eine primäre Taste je Dialog. React Flow nur über `apps/studio/src/stories/graph/` (eigene Knoten, `base.css`, Farben aus Tokens).
- Anhängepunkte nur anhängen: `apps/studio/src-tauri/src/commands/dispatch.rs`, `lib.rs`, `mcp/{access,catalog,tools,symbolic}.rs`, `crates/g4-kernel/src/lib.rs`, `apps/studio/src/api.ts`, `workbench/navigation.ts` (`NavTarget` + `story`, `thread`), `ui/primitives/icons.ts` (`story`, `thread`), `textproject/kinds.rs` (`DATA_KINDS`), `textproject/symbols.rs` (`SymbolKind::Story`), `registry.rs` (`REGISTRY_KINDS`), `diff/subject.rs`, `docs/mcp.md`, `docs/ROADMAP.md`, `design-system/README.md`, `scripts/gate.sh`.
- **Konfliktstellen mit 3d.4 und 3d.5 (parallel):** `data/refs.rs` und `refs/flagvars.rs` (3d.4 erweitert die Ziele deklarativ; 3d.6 hängt nur `Ref.value` an und nutzt `push`/`note`, Task 1), `tests/refs.rs` (Helfer `r`), `textproject/kinds.rs` und `check.rs` (3d.4 führt „Datei entfernen“ aus), `diff/subject.rs` (3d.4 „Gelöscht“), `commands/dispatch.rs`, `mcp/*.rs`, `api.ts`, `navigation.ts`, `icons.ts`, `App.tsx` (3d.5 der Dialog-Editor; 3d.6 nur `LOCKED_AREAS`, der Bereich und das Umziehen der Flag-Fläche), `shellState.ts:27` (`NARRATING_AREAS`; 3d.5 könnte „texte“ hinzufügen), `TechnikArea.tsx` (Eintrag `flags` weg), `docs/mcp.md`, `ROADMAP.md`, `design-system/README.md`, `.fig`. Teil B berührt zusätzlich 3d.4s Löschweg (`plan_delete`, `delete_entity`) erst nach Task 9. **Nicht anfassen:** `store.rs`, `project.rs`, `textproject/{export,import,arts,schema/*}.rs`, `scripts.rs`, `checks/*`, `recipes/*`, `commands/wizards.rs`, `workbench/inspector/*`, `world/scene/*`.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, die drei Check-Skripte; nach Task 6 auch `bash scripts/check-licenses-npm.sh`.
- Zeitgrenzen nur unter `G4_TIMING_STRICT=1` (`threads()` warm ≤ 100 ms, `thread_graph` ≤ 20 ms; sonst nur gedruckt), Muster `tests/flags.rs:32` (`strict()`).
- Gate-Zahlen (Spezifikation §4, Zeile 3d.6): Übersicht ≥ 59 (Platin DE) bzw. ≥ 62 (HeartGold DE, HeartGold US) Fäden mit Werten und Orten; Graph ein Knoten je Wert, Kanten „gesetzt in → geprüft in“; `git status` nach dem Anlegen nur `data/stories/0000.json` und `data/stories/0000.layout.json`; ⌘Z entfernt beide; `text_project_in_git_builds_from_base_byte_equal` mit einer Geschichte grün; `rename_id` eines Trainers zieht die Geschichte mit; Löschen des Trainers warnt (Teil B); Diff zeigt Feldnamen; `cargo deny` und npm-Lizenzen grün.
- Nicht in 3d.6 (wörtlich aus „Entscheidungen“): Rezepte, Schritte in der Oberfläche, Pool-Anzeige, Knoten ziehen und speichern, Bewegungen, Geschichten im `RefIndex`, Deck, Erzählen-Layouts; aus „Nicht in 3d“: Scripts aus Geschichten beim Build, Graph mit Code erzeugenden Knoten.

## Review Focus

- Eine Geschichte, deren `progress.var` der Index nicht kennt (keine Verweise, etwa eine freie Variable nach „Neue Geschichte“): der Graph hat null Wert-Knoten und keinen Fehler, die Übersicht nennt keinen Faden, das Detail sagt „Noch keine Werte im Spiel“ (Task 3 `a_variable_without_references_has_an_empty_graph`, Task 8 `a_story_without_values_says_so`).
- Ein Variablen-Argument, das keine Konstante ist (`SetVar VAR_X, VAR_Y`, `CompareVarToValue VAR_X, VAR_RESULT`, ein Alias ohne Wert): kein `value`, kein falscher Wert-Knoten, der Ort erscheint lose (Task 1 `a_variable_operand_gives_no_value`, Task 3 `places_without_a_value_are_loose_nodes`).
- Eine kaputte Geschichten-Datei (unbekanntes Feld, doppelter `key`, `id` ≠ Register, Nummer ohne Registereintrag): `textproject.schema`/`id-mismatch`/`id-dangling` mit Datei und Zeile, `story_list` lässt sie mit `problem` aus statt zu scheitern, Export und Import bleiben unberührt (Task 2 `check_file_reports_schema_and_id_problems`, Task 5 `story_list_skips_an_unreadable_file_with_its_problem`).
- DSPRE-Ordner ohne Manifest: Fäden und Graph lesbar, `story_create` antwortet `not_text_project` mit `NOT_A_TEXT_PROJECT`, die Fläche zeigt „Als Textprojekt einrichten…“ (Task 5 `story_create_refuses_the_dspre_layout`, Task 8 `the_dspre_layout_locks_creating_with_the_way_out`).
- Umbenennen: `rename_id VAR_MEIN → VAR_DEIN` (eigener Name aus 3d.1) zieht `progress.var` mit und der Faden behält seine Nummer; `rename_id STORY_A → STORY_B` ändert Register und `id` (Task 2 `renaming_a_variable_follows_the_story_progress`, `renaming_a_story_id_changes_register_and_file`).

---

## Teil A: Entität, Fäden, Graph, Bereich (nach dem Merge von 3d.1)

### Task 1: Index: der Wert eines Variablen-Verweises

**Files:**
- Modify: `crates/g4-kernel/src/data/refs.rs` (`Ref.value: Option<u16>` angehängt; `push` (`:634`) setzt `None`, ein neues `push_value(field, slot, target, value)`; Auslöser `:794-796` und Level-Scripts `:1248-1253` geben ihren `value`), `crates/g4-kernel/src/data/refs/flagvars.rs` (`FlagVarParam.value_arg: Option<usize>`; `command()` liest den Wert des `values`-Parameters, `condition()` den Literal-Operanden eines Vergleichs; `note()` nimmt `value`), `crates/g4-kernel/src/data/refs/flagvars/tests.rs`, `crates/g4-kernel/metadata/script_flagvars.yaml` (je Familie `values:`), `crates/g4-kernel/tests/refs.rs` (Helfer `r` mit `value: None`), `apps/studio/src-tauri/src/commands/flags.rs` (`FlagRefDto.value: Option<u16>`, damit die Fläche von 3d.1 den Wert zeigen kann: „= 3“ hinter dem Label), `apps/studio/src/api.flags.ts` (`FlagRefDto.value`), `docs/formats/flags_vars.md` (Abschnitt „Nutzungsindex › Scripts“: der Wert)

**Interfaces:**
- Consumes: `Ref`, `Access`, `FlagVarTable::params`, `ScriptNames::value`, `scripts::visit` mit `AstItem::Condition` (3d.1).
- Produces:

```rust
// data/refs.rs
pub struct Ref { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub target: Target, pub access: Access,
    /// For a variable: the constant the reference sets it to or compares it with (a command's value parameter,
    /// a comparison's literal operand, a trigger's `value`, a level script condition's `value`); None for flags
    /// and where the other operand is a variable or does not resolve.
    pub value: Option<u16> }
// data/refs/flagvars.rs
pub(crate) struct FlagVarParam { pub name: String, pub arg: usize, pub role: ParamRole, pub access: Access, pub direct: bool,
    /// The argument that carries the value of this variable parameter (`values` of the family), when there is one.
    pub value_arg: Option<usize> }
```

- `script_flagvars.yaml`: je Familie ein Abschnitt `values: { SetVar: valueOrVarID, SetVarFromValue: <Parameter>, CompareVarToValue: <Parameter>, CompareVar: valueOrVarID, … }` (pt) und die HGSS-Gegenstücke; der Kopfkommentar nennt die Messung (B2) als Quelle; der Test `script_flagvars_name_existing_commands_and_params` prüft jeden `values`-Eintrag gegen die Datenbank (Befehl bekannt, Parameter an einer Stelle, Befehl hat genau einen `Var`-Parameter). Ein Wert zählt nur `< 0x4000` (`FIRST_VARIABLE`); ein Argument `≥ 0x4000` ist eine Variable, dann `None`.

- [ ] **Step 1: Scheiternde Tests** (`flagvars/tests.rs`, synthetisch wie dort): `set_and_compare_carry_their_value` (`SetVar VAR_UNUSED_0x406C, 3` → `Var(0x406C)` Write `value Some(3)`; `CompareVarToValue 0x406C, 7` → Read `Some(7)`), `a_condition_literal_is_the_value` (`if VAR_UNUSED_0x406C == 2 then` → `Some(2)`; `while VAR_… < 4 do` → `Some(4)`; `match VAR_…` → `None`), `a_variable_operand_gives_no_value` (Review Focus: `SetVar VAR_X, VAR_Y` → beide Verweise `None`; `CompareVarToValue VAR_X, VAR_RESULT` → `None`, `script_local + 1`), `flags_never_carry_a_value`, `values_name_existing_commands` (Erweiterung des Datenbank-Tests); `tests/refs.rs`: `a_trigger_carries_its_value` (`coord_events[0]` mit `var 0x40A5, value 2` → `Some(2)`), `a_level_script_condition_carries_its_value`; alle bestehenden Tests mit `value: None` grün; `incremental_equals_full_rebuild` unverändert.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features refs` → FAIL (Feld fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `command()` liest `value_arg` über `self.value(arg)` und `u16::try_from`, nur wenn `< FIRST_VARIABLE`; `condition()` gibt bei `Infix { left, operator ≠ And|Or, right }` dem Variablen-Operanden den Wert des anderen Operanden, wenn der ein Literal `< FIRST_VARIABLE` ist.
- [ ] **Step 4: Lauf** → PASS; `cargo test --workspace --all-features` (die Studio-Tests bauen mit `FlagRefDto.value`); `cd apps/studio && pnpm test`.
- [ ] **Step 5: Doku** `flags_vars.md` wie unter Files (der Wert, der Abschnitt `values`, was `None` bleibt).
- [ ] **Step 6: Commit** `feat(refs): the value a variable reference sets or compares, from commands, conditions, triggers and level scripts` (mit Trailer).

### Task 2: Kernel `stories`: Entität, Art-Beschreibung, Register, Anlegen und Ändern als Dateischritt

**Files:**
- Create: `crates/g4-kernel/src/stories.rs` (Modul-Doku, Re-Exporte), `crates/g4-kernel/src/stories/schema.rs` (+ Tests), `crates/g4-kernel/src/stories/entities.rs` (+ Tests), `crates/g4-kernel/tests/stories.rs` (Textprojekt-Fixtures, gated Git-Test), `docs/formats/stories.md`
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod stories;`), `crates/g4-kernel/src/textproject/symbols.rs` (`SymbolKind::Story`, Präfix `STORY`, `table` → `None`, `files` leer; die Tests `tables_cover_the_gen4_ranges` und `fallback_ids_round_trip` nehmen `Story` in die Liste ohne feste Tabelle), `crates/g4-kernel/src/textproject/registry.rs` (`REGISTRY_KINDS` 7, `registry_file` `stories.json`), `crates/g4-kernel/src/textproject/kinds.rs` (`DATA_KINDS: [DataKind; 3]`; `check_kind` verzweigt auf `kind.symbol`; der Test `schemas_name_every_kind_and_keep_foreign_keys` nimmt `"plugins"` als fremden Schlüssel, B13), `crates/g4-kernel/src/textproject/rename.rs` (`KINDS` + `Story`; `kind_of("STORY_…")`; `judge` wie für `Place`: kein pret-Name, nur Register), `crates/g4-kernel/src/textproject/include.rs` (**nichts**: ein Test `stories_get_no_id_file` sichert, dass `tables` die Art `Story` auslässt), `docs/formats/text_project.md` (Tabelle „Arten“ Zeile `stories` „Geschichte (Projekt-Entität ohne Member, Beifile `.layout.json`)“; „Symboltabellen und Register“: sieben Register; „Manifest“: `schemas.stories`; „ID umbenennen“: `STORY_`)

**Interfaces:**
- Consumes: `textproject::{canon::to_canonical, registry::{Registry, Symbols}, ids::Resolver, rename::format_text, codes::{SCHEMA_CODE, ID_MISMATCH_CODE, ID_DANGLING_CODE}}`, `store::{FileStep, FileChange, FileStepKind::Reset, HEADER_TABLE, Store::record_file_step, write_file_step}`, `flags::{FlagKind, names}` (Prüfung von `progress`).
- Produces:

```rust
// stories/schema.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum StepKind { Start, Dialog, Condition, Battle, Reward, Progress, Appearance }
impl StepKind { pub fn all() -> &'static [StepKind]; pub fn word(self) -> &'static str; } // Start, Dialog, Bedingung, Kampf, Belohnung, Fortschritt, Erscheinen/Verschwinden
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum Progress { Var(String), Flag(String) }           // { "var": "VAR_…" } | { "flag": "FLAG_…" }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum Use { Trainer(String), Map(String), Flag(String), Var(String), Object { file: u16, local_id: u16 },
               Script { file: u16, slot: u16 }, Message { archive: u16, index: u16 }, LevelScript { file: u16, condition: u16 } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct Made { pub recipe: String, pub studio: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct Step { pub key: String, pub value: u16, pub kind: StepKind, pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")] pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub made: Option<Made>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub uses: Vec<Use> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct Story { pub id: String, pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")] pub notes: String,
    pub progress: Progress,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub flags: Vec<String> }
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct Layout { pub nodes: BTreeMap<String, Position> }   // Position { x: f64, y: f64 }
pub const STORIES_DIR: &str = "data/stories";
pub fn story_rel(n: u16) -> PathBuf;            // data/stories/0000.json
pub fn layout_rel(n: u16) -> PathBuf;           // data/stories/0000.layout.json
/// Reads and judges one story file: not JSON or not the schema → `textproject.schema` (file, line, col via `parse_json`);
/// `id` without the STORY_ prefix or ≠ the register's id of `n` → `textproject.id-mismatch`; a duplicate step `key` →
/// `textproject.schema`; `progress`/`uses`/`flags` that name no known id or number (resolver) → warning `story.dangling-ref`.
pub fn check_file(project: &Project, registry: &Registry, resolver: &Resolver, n: u16, bytes: &[u8]) -> Vec<Problem>;
pub fn read_story(project: &Project, n: u16) -> Result<Option<Story>, KernelError>;     // None: no file
pub fn read_layout(project: &Project, n: u16) -> Result<Option<Layout>, KernelError>;
/// The number of `id` from the register; None for an unknown id.
pub fn number_of(project: &Project, id: &str) -> Result<Option<u16>, KernelError>;
pub const DANGLING_REF_CODE: &str = "story.dangling-ref";
// stories/entities.rs
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)] #[serde(deny_unknown_fields)]
pub struct StoryRequest { pub name: String, pub progress: Progress, #[serde(default)] pub notes: String,
                          #[serde(default)] pub steps: Vec<Step>, #[serde(default)] pub flags: Vec<String> }
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)] #[serde(deny_unknown_fields)]
pub struct StoryPatch { pub name: Option<String>, pub notes: Option<String>, pub progress: Option<Progress>,
                        pub steps: Option<Vec<Step>>, pub flags: Option<Vec<String>>, pub layout: Option<Layout> }
#[derive(Debug, Clone, Serialize)]
pub struct StoryReport { pub id: String, pub number: u16, pub files: Vec<PathBuf>, pub change: Option<ChangeId>, pub ms: u64 }
/// Creates the story: id from the name (STORY_ + normalize, `_2`… on a clash), the smallest free number, register entry,
/// JSON (canonical) and an empty layout, as one file step (label „Geschichte „<name>“ angelegt“). Refused before anything
/// is written: not a current text project (NOT_A_TEXT_PROJECT), an empty name, a progress id that resolves nowhere
/// (`unknown_id_text`), a duplicate step key, a value above u16.
pub fn create_story(store: &mut Store, req: &StoryRequest) -> Result<StoryReport, KernelError>;
pub fn create_story_files(project: &Project, req: &StoryRequest) -> Result<StoryReport, KernelError>;   // ohne Store (g4k, Tests)
/// Applies `patch` (fields given replace, `layout` writes the side file) as one file step (label „Geschichte „<name>“ geändert“);
/// NOTHING_TO_CHANGE when nothing differs; the same refusals as `create_story`; `id` is never patched (rename_id does that).
pub fn update_story(store: &mut Store, id: &str, patch: &StoryPatch) -> Result<StoryReport, KernelError>;
pub fn update_story_files(project: &Project, id: &str, patch: &StoryPatch) -> Result<StoryReport, KernelError>;
pub fn list_stories(project: &Project) -> Result<Vec<(u16, Result<Story, Problem>)>, KernelError>;  // register order; a bad file as its problem
pub const NOTHING_TO_CHANGE: &str = "Nichts zu ändern.";
pub fn name_empty_text() -> &'static str;      // „Die Geschichte braucht einen Namen.“
```

- `check_kind` in `kinds.rs`: `match kind.symbol { Flag | Var => bisher, Story => stories::schema::check_file(…) je Datei, dazu die Nummern-Prüfung des Dateinamens und `id-dangling` für Registereinträge ohne Datei wie bisher }`. `rename::judge` für `Story`: Präfix `STORY_`, `format_text`, nicht vergeben; die Nummer bleibt.

- [ ] **Step 1: Scheiternde Tests:**
  - `schema.rs`: `a_story_round_trips_canonically` (das Beispiel aus Gutachten A §3.2 (c) mit sieben Schritten, je Art einer, alle `Use`-Formen: `to_canonical(parse(x)) == x`), `unknown_fields_and_duplicate_keys_are_schema_problems`, `check_file_reports_schema_and_id_problems` (Review Focus: `{"id":"STORY_B"}` unter `0000.json` mit Register `STORY_A: 0` → `id-mismatch` mit Zeile; `progress.var: "VAR_NOPE"` → `story.dangling-ref` Warnung), `kinds_have_seven_words`.
  - `entities.rs` (`TextFixture::exported()`, 3d.1-Register vorhanden): `create_writes_register_json_and_empty_layout` (`name "Testfaden"`, `Progress::Var("VAR_UNUSED_0x406C")` → `data/ids/stories.json` `{"STORY_TESTFADEN": 0}`, `data/stories/0000.json` kanonisch, `0000.layout.json` `{"nodes":{}}`), `a_second_story_takes_the_next_number_and_a_clashing_name_gets_a_suffix` (`STORY_TESTFADEN_2`, `0001`), `create_refuses_an_unknown_progress_id_and_an_empty_name`, `create_refuses_the_dspre_layout` (`TextFixture::dspre()` → `NOT_A_TEXT_PROJECT`), `create_over_the_store_is_one_undo_step` (nach `undo` fehlen beide Dateien und der Registereintrag; `redo` stellt sie her; Label „Geschichte „Testfaden“ angelegt“), `update_replaces_given_fields_and_records_one_step` (`notes`, `steps` mit zwei Schritten; `undo` stellt die alten Bytes her), `update_with_layout_writes_the_side_file_only`, `update_without_change_is_refused`, `list_stories_keeps_a_bad_file_as_its_problem` (Review Focus).
  - `kinds.rs`: `three_data_kinds_are_known_folders` (`is_known_folder("stories")`), `schemas_name_every_kind_and_keep_foreign_keys` mit `"plugins"`, `check_files_judges_stories_by_their_schema`.
  - `symbols.rs`/`registry.rs`/`rename.rs`: `story_has_a_prefix_and_no_fixed_table`, `registry_file_per_kind` + `stories.json`, `renaming_a_story_id_changes_register_and_file` (Review Focus), `renaming_a_variable_follows_the_story_progress` (Review Focus: `name_flag` `VAR_MEIN` für `0x406C`, Geschichte darauf, `rename_id VAR_MEIN → VAR_DEIN` → `progress.var == "VAR_DEIN"`), `renaming_a_trainer_follows_the_story_uses` (`Use::Trainer("TRAINER_X")` → neuer Name; ein Undo-Schritt), `include.rs::stories_get_no_id_file`.
  - `tests/stories.rs` (Fixtures): `export_and_import_leave_stories_alone` (Export nach dem Anlegen schreibt `stories/` nicht neu, Import liest es nicht; `second_export_writes_nothing` bleibt grün), `check_lists_the_story_files_as_sources` (`textproject::sources` enthält beide Dateien); gated: `text_project_with_a_story_builds_from_base_byte_equal` (Kopie wie `tests/git.rs`: `adopt`, `git_init`, `create_story_files`, `commit_all`, `build` → SHA-1 gleich der Basis, `git status` leer, der Index enthält beide Dateien; druckt `story-git <game_code> build_ms=<n>`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features stories:: textproject::` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces; die ID aus dem Namen über `symbols::normalize(name, true)` (Umlaute wie die Trainer-Taufe).
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` ganz (Textprojekt-Tests der Phase 4 und 3d.1 grün); gated einmal auf Platin DE und HeartGold DE.
- [ ] **Step 5: Doku** `docs/formats/stories.md` (Kopf wie `flags_vars.md:1-16`: eine Tatsache über Gen4 Studio; Abschnitte „Datei“ mit dem Beispiel, „Schritte und Arten“, „Verweise“ (`uses`, schwach), „Beifile“, „Register und ID“, „Dateischritt statt Store-Asset“ (die Abweichung von der Spezifikation mit Grund), Platzhalter „Fäden“ (Task 3), „Befehle“ (Task 5), „Löschen“ (Teil B)); `text_project.md` wie unter Files.
- [ ] **Step 6: Commit** `feat(stories): the story entity with layout side file, data kind, STORY_ register, create and update as one file step` (mit Trailer).

### Task 3: Fäden und Graph aus dem Index, gated Zählung

**Files:**
- Create: `crates/g4-kernel/src/stories/threads.rs` (+ Tests), `crates/g4-kernel/tests/threads.rs` (gated)
- Modify: `crates/g4-kernel/src/stories.rs` (Re-Exporte), `docs/formats/stories.md` („Fäden“: Regel, Orte, Graph), `docs/dev-setup.md` (Tabelle `G4_TEST_PROJECT`: `tests/threads.rs`, `tests/stories.rs`)

**Interfaces:**
- Consumes: Task 1 (`Ref.value`), `RefIndex::{refs, usage, refresh}` (`refs.rs:290, 343`), `Target::Var`, `flags::{area_of, names, name_is_unused, FlagKind, Area}`, `Store::header_table()`, `uxie::MapHeader::{script_file_id, level_script_id, event_file_id}` (Muster `Users::of`), Task 2 (`list_stories`).
- Produces:

```rust
// stories/threads.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub access: Access, pub value: Option<u16>,
                   /// The first header whose script, level script or event file the source is; None for a source no header names.
                   pub header: Option<u16> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Thread { pub var: u16, pub name: Option<String>, pub own_id: Option<String>, pub values: Vec<u16>,
                    /// Distinct places: headers, and sources without a header, in index order.
                    pub places: u32, pub headers: Vec<u16>, pub refs: Vec<Place>, pub story: Option<String> }
/// Every thread (decisions „Fäden-Regel“), ascending by number; builds or refreshes the index like `RefIndex::refs`.
pub fn threads(store: &mut Store, index: &mut RefIndex) -> Result<Vec<Thread>, KernelError>;
#[derive(Debug, Clone, PartialEq, Serialize)] #[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphNode { Value { id: String, value: u16, step: Option<String>, noted_only: bool },
                     Place { id: String, source: AssetId, header: Option<u16>, label: String, loose: bool } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphEdge { pub from: String, pub to: String, pub value: u16, pub access: Access, pub field: &'static str, pub slot: Option<u16> }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThreadGraph { pub var: u16, pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge>, pub layout: Option<Layout> }
/// The graph of variable `var` (decisions „Graph eines Fadens“); with `story` the steps lie on the value nodes and the
/// layout is the story's. Empty nodes and edges for a variable the index does not know.
pub fn thread_graph(store: &mut Store, index: &mut RefIndex, var: u16, story: Option<&Story>) -> Result<ThreadGraph, KernelError>;
pub fn place_label(source: AssetId) -> String;   // „Scriptdatei 0101“, „Eventdatei 0394“, „Level-Script 0005“
```

- [ ] **Step 1: Scheiternde Tests** (`threads.rs`, Fixture `TextFixture` mit übernommenen Scripts wie `tests/refs.rs::script_store`: Variable `0x4030` in zwei Scriptdateien (Header 1 und 2) mit `SetVar … 1`, `SetVar … 2`, `CompareVarToValue … 2`, ein Level-Script `var_conditions {0x4030, 3}` an Header 3, ein Auslöser `{0x4030, 1}`; Variable `0x4031` nur mit zwei Werten an einem Ort; `0x4020` (Objektgrafik) mit vielen Werten):
  - `a_thread_needs_three_values_at_two_places` (`0x4030` ist Faden mit `values [1, 2, 3]`, `headers [1, 2, 3]`; `0x4031` nicht; `0x4020` nicht).
  - `an_unused_named_variable_is_no_thread` (pt `VAR_UNUSED_0x406C` mit drei Werten an zwei Orten → kein Faden; `VAR_UNK_…` (hgss) → Faden).
  - `a_story_claims_its_thread` (nach `create_story_files` mit `progress.var` `0x4030` → `story == Some("STORY_…")`).
  - `the_graph_has_one_node_per_value_and_edges_set_to_checked` (drei `Value`-Knoten; Kante `p:script:0101 → v:1` Write, `v:2 → p:script:0102` Read, `v:3 → p:script:0005` (`var_conditions`), `v:1 → p:event:0394` (`var`)).
  - `places_without_a_value_are_loose_nodes` (Review Focus: `SetVar VAR_X, VAR_Y` → Ort-Knoten `loose: true`, keine Kante).
  - `a_variable_without_references_has_an_empty_graph` (Review Focus).
  - `steps_lie_on_their_value_nodes_and_unknown_values_are_noted_only` (Geschichte mit Schritten 1 und 9 → `v:1.step == Some("s1")`, `v:9` mit `noted_only: true`).
  - gated `tests/threads.rs` (`vanilla_copy()`, einmal `adopt` je Lauf, Release): `vanilla_threads_count_at_least_the_measured` (Platin ≥ 59, HGSS ≥ 62; jeder Faden hat ≥ 3 Werte und ≥ 2 Orte; druckt `threads <game_code> count=<n> values=<summe> max_values=<n> max_places=<n> ms=<n>`; Erwartung aus Gutachten A §2.2: 272 / 311 Werte, höchstens 16 Werte und 22 / 29 Orte, als Druck), `vanilla_thread_graph_of_the_first_thread_has_a_node_per_value` (`Value`-Knoten == `values.len()`, jede Kante nennt einen vorhandenen Knoten; unter `G4_TIMING_STRICT=1` `threads()` warm ≤ 100 ms, `thread_graph` ≤ 20 ms; druckt `graph <game_code> var=<hex> nodes=<n> edges=<n> threads_ms=<n> graph_ms=<n>`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features stories::threads` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `threads()` läuft einmal über `usage(Var)` (Kandidaten) und dann je Kandidat `refs(Target::Var(n))`; die Header-Zuordnung einmal je Aufruf aus `store.header_table()`.
- [ ] **Step 4: Lauf** → PASS; gated auf `3d-6-{pt,hgde,hgus}` (`G4_TEST_PROJECT=<Kopie> cargo test --release -p g4-kernel --all-features --test threads -- --nocapture`), die Zeilen in den Bericht.
- [ ] **Step 5: Doku** `stories.md` „Fäden“ und `dev-setup.md`.
- [ ] **Step 6: Commit** `feat(stories): threads (variables with three values at two places) and the thread graph from the reference index` (mit Trailer).

### Task 4: Diff `DiffSubject::Story`, Prüfung `story.dangling-ref` im Textprojekt-Check

**Files:**
- Modify: `crates/g4-kernel/src/diff/subject.rs` (`Story { index }` angehängt: `label` „Geschichte 0000“, `group` „Geschichten“ (neue Konstante `STORIES_GROUP`), `check` ok, `of_path` für `data/stories/NNNN.json` (das `.layout.json` bleibt `File`), `reset_target` `None`, `from_asset` unberührt), `crates/g4-kernel/src/diff/side.rs` (`Kind::JsonFile(PathBuf)`: Platte, Commit und `HEAD` lesen die Datei als `Side::Json`; Basis → `NO_BASE_FOR_FILES`; „Gespeichert“ = die Datei (Geschichten haben keinen ungespeicherten Zustand)), `crates/g4-kernel/src/diff.rs` (`empty()` → `Fields`; der Basis-Zweig lehnt `Story` wie `File` ab), `crates/g4-kernel/src/diff/tests.rs`, `crates/g4-kernel/src/textproject/check.rs` (`hints`: `story.dangling-ref` aus `check_files` zählt als Warnung; `codes.rs` + `DANGLING_REF_CODE` in der Codeliste), `crates/g4-kernel/src/textproject/codes.rs`, `apps/studio/src/api.git.ts` (`DiffSubject` + `{ kind: "story"; index: number }`), `apps/studio/src/technik/diffWords.ts` (+ test: `navTargetOf` für `story` → `{ kind: "story", index }` aus Task 7; bis dahin `null`), `docs/formats/text_project.md` („Codes“: `story.dangling-ref`), `docs/formats/stories.md` („Diff“, „Prüfung“)

**Interfaces:**
- Consumes: Task 2 (`story_rel`, `check_file`), `json_changes`, `GitRepo` (Lesen einer Datei aus `HEAD` wie für `File`).
- Produces: `DiffSubject::Story { index: u16 }` (serde `story`), `diff_entity(…, &DiffSubject::Story { index: 0 }, Against::Head)` → `EntityDiff::Fields` mit Pfaden wie `name`, `notes`, `steps[1].name`, `steps[1].uses[0].trainer`; `textproject::check` meldet `story.dangling-ref` als Warnung mit Datei und Zeile des Eintrags.

- [ ] **Step 1: Scheiternde Tests** (`diff/tests.rs` mit `GitFixture`): `a_story_diffs_as_fields_against_head` (Geschichte committen, `notes` und einen Schrittnamen ändern → `Changed`, zwei `FieldChange`s mit den Pfaden), `a_new_story_is_new_against_head_and_refused_against_base`, `of_path_maps_the_story_json_and_leaves_the_layout_a_file`, `story_label_and_group`; `check.rs`: `a_dangling_story_use_is_a_warning_with_its_line` (`uses: [{ "trainer": "TRAINER_NOPE" }]` → Warnung `story.dangling-ref`, Zeile der Zeichenkette; `check` lehnt nichts ab).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features diff:: textproject::check` → FAIL.
- [ ] **Step 3: Umsetzen** → **Step 4: Lauf** → PASS; `cd apps/studio && pnpm test` (`api.git` Typ).
- [ ] **Step 5: Commit** `feat(diff,textproject): DiffSubject::Story as a field diff, story.dangling-ref as a warning of the check` (mit Trailer).

### Task 5: Befehle: Tauri, Brücke, MCP, `g4k story`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/stories.rs` (+ Tests, gated Test)
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `src/lib.rs` (Handler anhängen), `src/commands/dispatch.rs` (`COMMANDS` + sechs, Beispielargumente), `src/mcp/access.rs` (`("story_list", Read)`, `("story_get", Read)`, `("list_threads", Read)`, `("thread_graph", Read)`, `("story_create", Write)`, `("story_update", Write)`), `src/mcp/catalog.rs` (sechs kuratierte Werkzeuge; `the_curated_set_is_the_decided_one`), `src/mcp/tools.rs` (nichts in `NOT_IN_BATCH`), `src/mcp/symbolic.rs` (`arg("thread_graph", "/var", Fixed(SymbolKind::Var))`, `arg("story_create", "/progress/var", Fixed(Var))`, `arg("story_create", "/progress/flag", Fixed(Flag))`, dieselben zwei für `story_update` unter `/patch/progress/…`; `story_get`/`story_update` `/id` ist die `STORY_`-ID als Text, keine Auflösung), `crates/g4-kernel/src/bin/g4k.rs` (`Command::Story(StoryCommand)`), `crates/g4-kernel/tests/cli.rs`, `docs/mcp.md` (Tabelle „Werkzeuge“: sechs Zeilen; „ID oder Nummer“: `thread_graph.var`, `story_create.progress.var/flag`; „Textprojekt als Quelle“: `story_create`/`story_update` nur im Textprojekt), `docs/formats/stories.md` („Befehle“)

**Interfaces:**
- Consumes: Task 2, 3; `commands::{lock, session, open_project_arc}`; `Resolver` (symbolische Argumente); `header_names` (Muster `phase-3d-1: commands/flags.rs:275`, wird `pub(crate)` in einem gemeinsamen Helfer `commands/places_names.rs` oder bleibt in `flags.rs` und wird `pub(crate)`: Entscheidung beim Umsetzen, im Commit nennen).
- Produces:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ThreadDto { pub var: u16, pub name: Option<String>, pub own_id: Option<String>, pub values: Vec<u16>, pub places: u32,
                       pub headers: Vec<u16>, pub header_names: Vec<String>, pub story: Option<String> }
#[derive(Debug, Clone, Serialize)]
pub struct ThreadListDto { pub threads: Vec<ThreadDto>, pub text_project: bool,
                           /// Whether the scripts are adopted (without them the index sees no script and the list is empty).
                           pub scripts_adopted: bool, pub ms: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct PlaceNodeDto { pub id: String, pub source: AssetId, pub header: Option<u16>, pub header_name: Option<String>,
                          pub label: String, pub loose: bool, pub nav: NavTargetDto }
#[derive(Debug, Clone, Serialize)] #[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphNodeDto { Value { id: String, value: u16, step: Option<StepDto>, noted_only: bool }, Place(PlaceNodeDto) }
#[derive(Debug, Clone, Serialize)]
pub struct ThreadGraphDto { pub var: u16, pub name: Option<String>, pub own_id: Option<String>, pub nodes: Vec<GraphNodeDto>,
                            pub edges: Vec<GraphEdge>, pub layout: Option<Layout>, pub ms: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct StoryRowDto { pub number: u16, pub id: String, pub name: String, pub progress: Progress, pub steps: u32, pub problem: Option<String> }
#[derive(Debug, Clone, Serialize)]
pub struct StoryListDto { pub stories: Vec<StoryRowDto>, pub text_project: bool, pub ms: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct StoryDto { pub number: u16, pub story: Story, pub layout: Layout, pub progress_number: Option<u16>, pub ms: u64 }
pub fn list_threads_impl(state: &AppState) -> Result<ThreadListDto, CmdError>;
pub fn thread_graph_impl(state: &AppState, var: u16, story: Option<String>) -> Result<ThreadGraphDto, CmdError>;
pub fn story_list_impl(state: &AppState) -> Result<StoryListDto, CmdError>;
pub fn story_get_impl(state: &AppState, id: String) -> Result<StoryDto, CmdError>;          // unknown id → CmdError code unknown_id
pub fn story_create_impl(state: &AppState, req: StoryRequest) -> Result<StoryReport, CmdError>;  // Refused → not_text_project / invalid_id / unknown_id / refused
pub fn story_update_impl(state: &AppState, id: String, patch: StoryPatch) -> Result<StoryReport, CmdError>;
// NavTargetDto / NavTarget (angehängt, Task 7 im Frontend): Story { number: u16 }, Thread { var: u16 }   (TS: { kind: "story"; number }, { kind: "thread"; var })
```

- Alle unter dem Sitzungsschloss; `story_create_impl`/`story_update_impl` rufen nach dem Schritt `session.after_write`/`files_written()` wie `name_flag`. Beispielargumente in `dispatch.rs`: `("list_threads", json!({}))`, `("thread_graph", json!({"var": 16432, "story": null}))`, `("story_list", json!({}))`, `("story_get", json!({"id": "STORY_TEST"}))`, `("story_create", json!({"req": {"name": "Test", "progress": {"var": "VAR_UNUSED_0x406C"}}}))`, `("story_update", json!({"id": "STORY_TEST", "patch": {"notes": "x"}}))`. Katalog (englisch): `list_threads` „List the story threads of the game“: „Every saved variable of the general area that the scripts, level scripts and triggers set to at least three different values at at least two places (headers): number, its pret name and own id, the values, the places and the story that claims it. Threads come from the reference index; no story file is needed.“; `thread_graph` „Graph of one thread“: „One node per value of the variable and one per place; an edge from a place to a value where a script sets it, from a value to a place where a script, level script or trigger checks it. With story the steps lie on the value nodes. var takes a number or VAR_… (also an own id).“ + `data_note!()`; `story_list`, `story_get` („the story file with its layout“), `story_create` („Creates data/stories/NNNN.json and its .layout.json in a text project as one undo step; progress names a variable or a flag (VAR_…, FLAG_…, own ids or numbers); the id STORY_… comes from the name“), `story_update` („Replaces the given fields (name, notes, progress, steps, flags, layout) as one undo step; the id changes only with rename_id“).
- `g4k story list|get|create|threads|graph` (siehe Entscheidungen) drucken JSON mit `--json` bzw. eine Zeile je Faden („0x4030 VAR_… 3 Werte · 3 Orte“).

- [ ] **Step 1: Scheiternde Tests** (`commands/stories.rs`, Fixture mit übernommenen Scripts und `TextFixture`): `list_threads_names_values_places_and_header_names`, `thread_graph_carries_nav_targets_per_place` (`nav.kind == script` mit Zeile, `event`, `header` für ein Level-Script), `story_create_and_update_over_the_command_path_are_one_undo_step_each`, `story_create_refuses_the_dspre_layout` (Review Focus: Code `not_text_project`), `story_list_skips_an_unreadable_file_with_its_problem` (Review Focus), `story_get_of_an_unknown_id_is_unknown_id`, `thread_graph_takes_a_symbolic_var` (`{"var": "VAR_UNUSED_0x406C"}` über `dispatch`); `access.rs`, `catalog.rs`, `symbolic.rs`, `dispatch.rs` (Fixtures aller sechs; `commands_match_the_tauri_handler`); `cli.rs`: `story_create_and_list_on_a_text_fixture` (synthetisch; `g4k story create … --name Test --var VAR_UNUSED_0x406C` → Datei, `g4k story list` nennt sie, `g4k story threads --json` ist gültiges JSON). Gated: `stories_over_the_command_path_on_the_vanilla_copy` (Kopie als Textprojekt: `list_threads_impl().threads.len() ≥ 59` (pt) bzw. `≥ 62`; `story_create_impl` → `git status`-Äquivalent über `textproject::sources` und die Dateien; `undo` entfernt sie; druckt `stories <game_code> threads_ms=<n> graph_ms=<n> create_ms=<n>`).
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`, `cargo test -p g4-kernel --test cli`.
- [ ] **Step 5: Doku** `docs/mcp.md`, `stories.md` wie unter Files.
- [ ] **Step 6: Commit** `feat(studio,g4k): list_threads, thread_graph, story_list, story_get, story_create and story_update over the bridge, MCP and g4k` (mit Trailer).

### Task 6: Graph-Bibliothek, npm-Lizenzschritt, Baustein `StoryNode`

**Files:**
- Create: `scripts/check-licenses-npm.sh`, `apps/studio/src/ui/primitives/StoryNode.tsx`, `StoryNode.module.css`, `StoryNode.test.tsx`, `apps/studio/src/stories/graph/layout.ts` (+ test: dagre-Aufruf hinter einer Funktion), `apps/studio/src/stories/graph/ThreadGraph.tsx` (+ `.module.css`, test), `apps/studio/src/stories/graph/graphModel.ts` (+ test: DTO → React-Flow-Knoten und -Kanten)
- Modify: `apps/studio/package.json` und `pnpm-lock.yaml` (`@xyflow/react`, `@dagrejs/dagre` mit genauer Version), `scripts/gate.sh` (Schritt `npm licenses` nach `cargo deny`, `:105`), `apps/studio/src/ui/primitives/index.ts` (`StoryNode`), `apps/studio/src/ui/primitives/icons.ts` (`story: BookOpenText` ist belegt durch `narrate`; neu `thread: GitBranch`, `story: Scroll` aus Phosphor; Namen beim Umsetzen gegen `@phosphor-icons/react` prüfen), `apps/studio/src/ui/gallery/pages/ContentPage.tsx` (Block „Story-Knoten“: die sieben Arten in Ruhe und Gewählt, dazu „nur notiert“), `docs/dev-setup.md` (der Lizenzschritt)

**Interfaces:**
- Consumes: Task 5 (`ThreadGraphDto`, `GraphNodeDto`, `GraphEdge`), Tokens `--g4-*` (Bereichsfarbe `geschichten` für Wert-Knoten, `flaeche/blatt`, `linie/standard`, `auswahl/kante`).
- Produces:

```ts
// ui/primitives/StoryNode.tsx
export type StoryNodeKind = "start" | "dialog" | "condition" | "battle" | "reward" | "progress" | "appearance" | "value" | "place";
export type StoryNodeProps = { kind: StoryNodeKind; title: string; subtitle?: string; selected?: boolean; notedOnly?: boolean; loose?: boolean;
                               onOpen?: () => void; dataAttrs?: RowDataAttrs };
export function StoryNode(props: StoryNodeProps): JSX.Element;   // role="button" wenn onOpen; data-story-node, data-kind, data-noted-only, data-loose
export function storyKindWord(kind: StoryNodeKind): string;      // Start, Dialog, Bedingung, Kampf, Belohnung, Fortschritt, Erscheinen/Verschwinden, Wert, Ort
// stories/graph/graphModel.ts
export function toFlow(graph: ThreadGraphDto): { nodes: Node<StoryNodeData>[]; edges: Edge[] };  // Knoten-Typ "story", Kanten mit label "Zeile 12 · SetVar" bzw. "Objekt 3", gestrichelt bei noted_only
// stories/graph/layout.ts
export function layoutGraph(nodes: Node[], edges: Edge[], fixed: Layout | null): Node[];  // dagre rankdir LR, nodesep 24, ranksep 64; fixed positions win
// stories/graph/ThreadGraph.tsx (default export für React.lazy)
export default function ThreadGraph(props: { graph: ThreadGraphDto; selected: string | null; onSelect: (id: string | null) => void; onOpenPlace: (nav: NavTargetDto) => void }): JSX.Element;
// data-thread-graph, data-node-count, data-edge-count; Knoten über StoryNode; nur base.css; fitView, Minimap aus, Ziehen aus (3d.6 liest das Layout nur)
```

- `check-licenses-npm.sh`: wie unter Entscheidungen; druckt am Ende `npm licenses: <n> packages ok`; Test im Skript selbst nicht nötig, der Gate-Lauf ist der Test.

- [ ] **Step 1: Abhängigkeiten** `cd apps/studio && pnpm add @xyflow/react@<12.x> @dagrejs/dagre@<1.x>` (genaue Versionen, keine `^`), `pnpm licenses list --prod --json` ansehen (neue Lizenzen beider Pakete und ihrer Abhängigkeiten: erwartet MIT, dazu `d3-*` ISC/BSD-3-Clause), das Skript schreiben, `bash scripts/check-licenses-npm.sh` grün, `cargo deny --locked check licenses advisories` unverändert grün.
- [ ] **Step 2: Scheiternde Tests**: `StoryNode.test.tsx` (`seven_kinds_render_their_word`, `selected_and_noted_only_set_data_attributes`, `a_node_with_onOpen_is_a_button`), `graphModel.test.ts` (`value_nodes_become_story_nodes_with_their_step`, `edges_carry_field_and_slot_labels`, `a_loose_place_has_no_edges`), `layout.test.ts` (`fixed_positions_win_over_dagre`, `dagre_puts_setters_left_of_values_and_readers_right`), `ThreadGraph.test.tsx` (`renders_one_story_node_per_graph_node` mit `data-node-count`; `selecting_a_place_calls_onOpen` nicht, nur `onSelect`; das Öffnen eines Orts über die Taste „Öffnen“ im gewählten Knoten).
- [ ] **Step 3: Lauf** `pnpm test -- StoryNode graph` → FAIL → **Step 4: Umsetzen** → **Step 5: Lauf** → PASS; `pnpm build` (die Chunk-Größe des Graphs aus der Vite-Ausgabe in den Commit-Text und die Gate-Tabelle); `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`; Galerie-Block; Screenshots der Galerie-Seite „Inhalt“ (Block „Story-Knoten“) in Hell, Dunkel, Kompakt nach `target/agent/gate-<datum>/3d-6/galerie-*.png` (Test-Umgebung 1760/1761, `?gallery`).
- [ ] **Step 6: Commit** `feat(ui): StoryNode with seven kinds, thread graph over @xyflow/react and @dagrejs/dagre as a lazy chunk, npm license check in the gate` (mit Trailer).

### Task 7: Frontend: API, Bereich „Geschichten“ frei, Fäden-Übersicht mit Graph, Flag-Fläche zieht um

**Files:**
- Create: `apps/studio/src/api.stories.ts` (+ test), `apps/studio/src/stories/storyWords.ts` (+ test), `apps/studio/src/stories/useThreads.ts` (+ test), `apps/studio/src/stories/useThreadGraph.ts` (+ test), `apps/studio/src/stories/GeschichtenArea.tsx` (+ `.module.css`, test), `apps/studio/src/stories/ThreadsSurface.tsx` (+ test), `apps/studio/src/stories/storiesTesting.ts`
- Modify: `apps/studio/src/api.ts` (`export * from "./api.stories";`), `apps/studio/src/api.data.ts` (`NavTargetDto` + `story`, `thread`), `apps/studio/src/workbench/navigation.ts` (`NavTarget` + `{ kind: "story"; number: number }`, `{ kind: "thread"; var: number }`), `apps/studio/src/ui/frame/shellState.ts` (`NARRATING_AREAS = ["welt", "geschichten"]`, + test), `apps/studio/src/App.tsx` (`LOCKED_AREAS = []`, `LOCKED_HINTS` ohne `geschichten`; Mount des Bereichs neben `technik` in `:1290`; `goTo` für `story`/`thread`/`flag` → Bereich Geschichten; `onShowInDirectory` der Picker → Geschichten › Flags), `apps/studio/src/App.test.tsx` (der Bereich öffnet), `apps/studio/src/technik/TechnikArea.tsx` (+ test: Eintrag `flags`, `subtitles.flags` und der Mount weg; `TechnikEntry` ohne `flags`), `apps/studio/src/app/paletteEntries.ts` (+ test: „Geschichten“, „Fäden“, „Flags und Variablen“ zeigt auf Geschichten), `apps/studio/src/technik/diffWords.ts` (+ test: `navTargetOf` für `{ kind: "story", index }` → `{ kind: "story", number: index }`; das Ziel wählt in Geschichten › Geschichten die Zeile mit dieser Nummer), `apps/studio/src/ui/gallery/pages/ContentPage.tsx` (Zeile der Fäden-Übersicht)

**Interfaces:**
- Consumes: Task 5 (DTOs, Befehle), Task 6 (`ThreadGraph`, `StoryNode`), 3d.1 (`FlagsSurface`, `FlagsSurfaceProps`, `displayName`, `hex4`), `ListPane`, `ListRow`, `Table`, `Reference`, `EmptyState`, `Notice`, `useShell()` (`viewDepth`).
- Produces (TS):

```ts
// api.stories.ts
export type StepKind = "start" | "dialog" | "condition" | "battle" | "reward" | "progress" | "appearance";
export type Progress = { var: string } | { flag: string };
export type StoryUse = { trainer: string } | { map: string } | { flag: string } | { var: string } | { object: { file: number; local_id: number } }
  | { script: { file: number; slot: number } } | { message: { archive: number; index: number } } | { levelScript: { file: number; condition: number } };
export interface Step { key: string; value: number; kind: StepKind; name: string; notes?: string; made?: { recipe: string; studio: string }; uses?: StoryUse[] }
export interface Story { id: string; name: string; notes?: string; progress: Progress; steps?: Step[]; flags?: string[] }
export interface Layout { nodes: Record<string, { x: number; y: number }> }
export interface ThreadDto { var: number; name: string | null; own_id: string | null; values: number[]; places: number; headers: number[]; header_names: string[]; story: string | null }
export interface ThreadListDto { threads: ThreadDto[]; text_project: boolean; scripts_adopted: boolean; ms: number }
export type GraphNodeDto = { kind: "value"; id: string; value: number; step: Step | null; noted_only: boolean }
  | { kind: "place"; id: string; source: AssetId; header: number | null; header_name: string | null; label: string; loose: boolean; nav: NavTargetDto };
export interface GraphEdgeDto { from: string; to: string; value: number; access: FlagAccess; field: string; slot: number | null }
export interface ThreadGraphDto { var: number; name: string | null; own_id: string | null; nodes: GraphNodeDto[]; edges: GraphEdgeDto[]; layout: Layout | null; ms: number }
export interface StoryRowDto { number: number; id: string; name: string; progress: Progress; steps: number; problem: string | null }
export interface StoryListDto { stories: StoryRowDto[]; text_project: boolean; ms: number }
export interface StoryDto { number: number; story: Story; layout: Layout; progress_number: number | null; ms: number }
export interface StoryReport { id: string; number: number; files: string[]; change: number | null; ms: number }
export const listThreads: () => Promise<ThreadListDto>;
export const threadGraph: (variable: number, story?: string) => Promise<ThreadGraphDto>;   // invoke("thread_graph", { var: variable, story: story ?? null })
export const storyList: () => Promise<StoryListDto>;
export const storyGet: (id: string) => Promise<StoryDto>;
export const storyCreate: (req: { name: string; progress: Progress; notes?: string }) => Promise<StoryReport>;   // invoke("story_create", { req })
export const storyUpdate: (id: string, patch: Partial<Pick<Story, "name" | "notes" | "progress" | "steps" | "flags">> & { layout?: Layout }) => Promise<StoryReport>;
// stories/storyWords.ts
export function threadName(t: ThreadDto): string;               // own_id, sonst name, sonst "Variable 0x4030" (displayName aus flagWords)
export function valuesText(values: readonly number[]): string;  // "3 Werte: 0, 1, 2" (ab 9 Werten "16 Werte: 0 … 15")
export function placesText(t: ThreadDto): string;               // "3 Orte: Zweiblattdorf, Route 201, …" (höchstens drei Namen, dann "+ 4")
export function stepKindWord(kind: StepKind): string;
export function progressLabel(p: Progress): { kind: FlagKind; id: string };
// stories/useThreads.ts, useThreadGraph.ts
export function useThreads(deps: { project: ProjectInfo | null; revision: number }): { list: ThreadListDto | null; error: string | null; reload: () => void };
export function useThreadGraph(variable: number | null, story: string | null, deps: { project: ProjectInfo | null; revision: number }): { graph: ThreadGraphDto | null; error: string | null; busy: boolean };
// stories/GeschichtenArea.tsx
export type GeschichtenEntry = "threads" | "stories" | "flags";
export interface GeschichtenAreaProps { project: string | null; entry: GeschichtenEntry; onEntry: (e: GeschichtenEntry) => void; listOpen: boolean; onToggleList: () => void;
  depth: Depth; revision: number; goTo: (t: NavTarget) => void; selectedThread: number | null; onSelectThread: (v: number | null) => void;
  selectedStory: number | null; onSelectStory: (n: number | null) => void; flags: GeschichtenFlagsProps; textProject: boolean; onOpenTextProject: () => void; onError: (text: string) => void }
// selectedStory ist die Nummer (wie NavTarget.story); die Fläche sucht die ID in der Liste (`StoryRowDto.number` → `id`) für `storyGet`/`storyUpdate`
export function GeschichtenArea(props: GeschichtenAreaProps): JSX.Element;   // data-geschichten=<entry>; ListPane „Geschichten“ mit drei ListRows (Untertitel: „59 Fäden“, „2 Geschichten“, „gesetzt/geprüft“-Zähler aus 3d.1)
// stories/ThreadsSurface.tsx
export function ThreadsSurface(props: { threads: ThreadListDto | null; error: string | null; depth: Depth; selected: number | null; onSelect: (v: number | null) => void; goTo: (t: NavTarget) => void; project: ProjectInfo | null; revision: number }): JSX.Element;
// Table (aria „Fäden“) mit den Spalten „Faden“ (Name; ab Bauen `tech` hex4), „Werte“, „Orte“, „Geschichte“ (Chip oder „–“); Zeile `[data-thread-row=<var>]`; unter der Tabelle der Graph der gewählten Zeile (`React.lazy(ThreadGraph)` mit `Suspense`-Platzhalter „Graph wird geladen…“); leer: EmptyState „Keine Fäden: die Scripts sind nicht übernommen.“ mit Taste „Scripts übernehmen…“ (→ Technik › Textprojekt)
```

- [ ] **Step 1: Scheiternde Tests:** `api.stories.test.ts` (`story_commands_send_the_documented_arguments`), `storyWords.test.ts` (`values_and_places_texts`, `thread_name_prefers_the_own_id`), `useThreads.test.tsx`, `useThreadGraph.test.tsx` (`drops_an_older_answer`), `ThreadsSurface.test.tsx` (`rows_show_name_values_places_and_story`, `in_erzaehlen_the_number_is_hidden_and_from_bauen_shown`, `selecting_a_row_loads_its_graph`, `without_adopted_scripts_the_empty_state_offers_the_way`), `GeschichtenArea.test.tsx` (`three_entries_and_their_surfaces`, `the_flags_entry_mounts_the_flags_surface`), `shellState.test.ts` (`geschichten_is_a_narrating_area`), `TechnikArea.test.tsx` (kein Eintrag `flags`), `App.test.tsx` (`the_geschichten_area_is_open`, `goTo_flag_opens_geschichten_flags`), `paletteEntries.test.ts`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** (CSS-Module nur mit Tokens; der Bereich übernimmt die Props der Flag-Fläche aus `App.tsx` unverändert); **Step 4: Lauf** → PASS, `pnpm build`, die drei Check-Skripte; Browser auf `3d-6-web` (Platin DE, Textprojekt): Geschichten › Fäden, ≥ 59 Zeilen, Klick auf `VAR_…` → Graph mit Wert-Knoten; Screenshots `blick-faeden-{liste,graph}-{hell,dunkel,kompakt}.png`, dazu `blick-faeden-liste-erzaehlen-hell.png` (ohne Nummern).
- [ ] **Step 5: Commit** `feat(studio): the area Geschichten with the thread overview and graph, flags surface moved from Technik` (mit Trailer).

### Task 8: Frontend: Geschichten anlegen und lesen

**Files:**
- Create: `apps/studio/src/stories/StoriesSurface.tsx` (+ `.module.css`, test), `apps/studio/src/stories/StoryDetail.tsx` (+ test), `apps/studio/src/stories/NewStoryDialog.tsx` (+ test), `apps/studio/src/stories/useStories.ts` (+ test), `apps/studio/src/stories/useStory.ts` (+ test)
- Modify: `apps/studio/src/stories/GeschichtenArea.tsx` (+ test: Mount von `StoriesSurface`), `apps/studio/src/App.tsx` (Toast nach dem Anlegen „Geschichte „X“ angelegt“, `revision` nach `story_create`/`story_update` wie nach `name_flag`), `apps/studio/src/ui/gallery/pages/ContentPage.tsx` (Zeile einer Geschichte und der Dialog)

**Interfaces:**
- Consumes: Task 7 (API, Hooks, `ThreadGraph`), 3d.1 (`FlagField` der Art Variable/Flag, `FlagPicker`), `Dialog`, `TextField`, `Table`, `Badge`, `Reference`, `Section`, `Notice`, `EmptyState`, `Button`.
- Produces:

```ts
export function StoriesSurface(props: { stories: StoryListDto | null; error: string | null; selected: number | null; onSelect: (n: number | null) => void;
  textProject: boolean; onOpenTextProject: () => void; depth: Depth; goTo: (t: NavTarget) => void; project: ProjectInfo | null; revision: number; onCreated: (r: StoryReport) => void; onError: (t: string) => void }): JSX.Element;
// Kopf: PanelHead „Geschichten“ mit Taste „Neue Geschichte…“ (`[data-new-story]`; im DSPRE-Layout gesperrt, Tooltip NOT_A_TEXT_PROJECT, darunter EmptyState mit Taste „Als Textprojekt einrichten…“); Liste als Table (Name, Fortschritt als Reference-Chip der Fläche geschichten mit tech hex4, Schritte); Zeile `[data-story-row=<id>]`; eine Zeile mit `problem` zeigt Notice warning; rechts/unten StoryDetail der gewählten
export function StoryDetail(props: { story: StoryDto; depth: Depth; goTo: (t: NavTarget) => void; onUpdate: (patch: Parameters<typeof storyUpdate>[1]) => Promise<void>; project: ProjectInfo | null; revision: number }): JSX.Element;
// data-story-detail; Kopf Name (TextField, commit beim Verlassen → onUpdate({ name })), Chip Fortschritt (öffnet Geschichten › Flags auf der Nummer), Notizen (TextField mehrzeilig, data-story-notes, commit beim Verlassen → onUpdate({ notes })), Section „Schritte“ mit Table (Wert, Art als Badge mit stepKindWord, Name, Verweise als Reference-Chips: trainer → Pokémon-Trainer, map → Welt, flag/var → Flags; object/script/message/levelScript als Chips mit nav aus den DTOs, soweit navTargetOf sie kennt, sonst Text); ohne Schritte „Noch keine Schritte. Rezepte legen sie an (kommt mit 3d.7).“; Section „Graph“ mit ThreadGraph der progress-Nummer und der Geschichte; progress_number null oder Graph ohne Knoten → „Noch keine Werte im Spiel“ (data-story-empty-graph)
export function NewStoryDialog(props: { open: boolean; onClose: () => void; onCreate: (req: { name: string; progress: Progress; notes?: string }) => Promise<void>; project: ProjectInfo | null; revision: number }): JSX.Element;
// Dialog „Neue Geschichte“ (data-new-story-dialog): „Name“ (TextField, Pflicht), „Fortschritt“ als Segmented „Variable | Flag“ (data-progress-kind) mit FlagField der gewählten Art (Wert 0 = nichts gewählt → Taste gesperrt), „Notizen“; Aktionen „Abbrechen“, „Geschichte anlegen“ (primär; gesperrt ohne Name oder ohne Fortschritt); ein Fehler des Backends als Notice error im Dialog
```

- [ ] **Step 1: Scheiternde Tests:** `StoriesSurface.test.tsx` (`rows_show_name_progress_and_step_count`, `the_dspre_layout_locks_creating_with_the_way_out` (Review Focus), `a_row_with_a_problem_shows_the_notice`), `NewStoryDialog.test.tsx` (`create_is_locked_without_name_or_progress`, `creating_sends_name_progress_and_notes`, `a_backend_error_stays_in_the_dialog`), `StoryDetail.test.tsx` (`notes_commit_on_blur`, `steps_show_value_kind_name_and_uses`, `a_story_without_values_says_so` (Review Focus), `the_graph_carries_the_steps_on_value_nodes`), `useStories.test.tsx`, `useStory.test.tsx`, `GeschichtenArea.test.tsx` (`the_stories_entry_mounts_the_surface`).
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS, `pnpm build`, Check-Skripte; Browser auf `3d-6-web`: „Neue Geschichte…“ mit `VAR_UNUSED_0x406C` (Picker „Freie Variable nehmen“) → Toast, `git status` in der Kopie zeigt genau `data/stories/0000.json` und `data/stories/0000.layout.json` (als `??`), ⌘Z → beide weg und das Register ohne Eintrag; Notizen ändern → `git status` zeigt die JSON geändert; Screenshots `blick-geschichten-{liste,dialog,detail}-{hell,dunkel,kompakt}.png`.
- [ ] **Step 5: Commit** `feat(studio): stories list, the dialog Neue Geschichte and the story detail with notes, steps and graph` (mit Trailer).

---

## Übergang

### Task 9: Main mergen (3d.4 muss drin sein)

**Files:**
- Modify (nur Konfliktlösung): `data/refs.rs`, `refs/flagvars.rs`, `tests/refs.rs`, `textproject/{kinds,check,codes,rename,symbols,registry}.rs`, `diff/subject.rs`, `dispatch.rs`, `lib.rs`, `mcp/*.rs`, `api.ts`, `api.data.ts`, `navigation.ts`, `icons.ts`, `App.tsx`, `shellState.ts`, `TechnikArea.tsx`, `docs/mcp.md`, `ROADMAP.md`, `design-system/README.md`, `.fig`

**Interfaces:**
- Consumes: main mit gemergtem 3d.4 (und 3d.5, wenn es vorher drin ist).
- Produces: der Worktree auf dem Stand von main plus Teil A; 3d.4s `plan_delete`, `delete_entity`, `restore_entity`, `DeleteRule`-Ausführung und `schemaVersion` 2 sind im Worktree.

- [ ] **Step 1: Prüfen** im Worktree: `git log --oneline main -5` und `git grep -n '"plan_delete"' main -- apps/studio/src-tauri/src/commands/dispatch.rs` → mindestens ein Treffer (der MCP-Befehl von 3d.4, Spezifikation §3.4). **Kein Treffer: hier stoppen, Teil A ist fertig und committet, Bericht an den Nutzer „3d.4 ist nicht auf main, Teil B offen“; nichts weiter tun.**
- [ ] **Step 2: Mergen** `git merge main` (im Worktree, nie umgekehrt); Konflikte nur an Anhängepunkten erwartet: beide Seiten behalten, Reihenfolge main zuerst, dann 3d.6; `.fig` von main behalten; `schema.snap` und `schemaVersion` von main (3d.4) behalten; hat 3d.4 `Ref` um Felder erweitert, bleibt `value` am Ende; hat 3d.4 `DATA_KINDS` die Ausführung von `RemoveFile` gegeben, nimmt der Geschichten-Eintrag sie mit.
- [ ] **Step 3: Volle Prüfung** `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cd apps/studio && pnpm test && pnpm build`, die vier Check-Skripte (inklusive `check-licenses-npm.sh`); die Fixtures in `dispatch.rs` zählen alle Befehle beider Seiten.
- [ ] **Step 4: Commit** der Merge (`Merge branch 'main' into phase-3d-6-geschichten-lesen`; der Trailer wird angehängt).

---

## Teil B: Schwache Verweise, Löschen, Gate

### Task 10: Geschichten als schwache Verweise beim Löschen, `story.dangling-ref` mit Grabsteinen

**Files:**
- Create: `crates/g4-kernel/src/stories/uses.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/stories.rs`, 3d.4s Löschplan im Kernel (die Datei, die `plan_delete` rechnet; Name nach dem Merge feststellen, erwartet `crates/g4-kernel/src/textproject/delete.rs` oder `data/delete.rs`: Feld `mentions: Vec<StoryUse>` im Plan-Report, gefüllt aus `uses_of`; **sperrt nicht**), `apps/studio/src-tauri/src/commands/<delete>.rs` (`plan_delete`-DTO + `mentions`, `delete_entity`-Antwort + `warnings`), der Lösch-Dialog im Frontend aus 3d.4 (Abschnitt „Erwähnt in Geschichten“ mit Chips, `[data-delete-mentions]`, Hinweis „Geschichten sperren nicht; der Verweis wird als offen gemeldet.“), `crates/g4-kernel/src/stories/schema.rs` (`check_file`: ein `uses`-Ziel mit Grabstein → `story.dangling-ref` „… ist gelöscht“), `docs/mcp.md` (`plan_delete`/`delete_entity`: `mentions`/`warnings`), `docs/formats/stories.md` („Verweise“: schwach, der Löschweg)

**Interfaces:**
- Consumes: Task 2, 3d.4 (`plan_delete`, `delete_entity`, Grabstein-Abfrage `is_deleted(kind, index)` o. ä.).
- Produces:

```rust
// stories/uses.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] #[serde(tag = "kind", rename_all = "snake_case")]
pub enum UseTarget { Trainer(u16), Map(u16), Flag(u16), Var(u16), Object { file: u16, local_id: u16 }, Script { file: u16, slot: u16 },
                     Message { archive: u16, index: u16 }, LevelScript { file: u16, condition: u16 } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoryUse { pub story_id: String, pub story_name: String, pub number: u16, pub step_key: Option<String>, pub step_name: Option<String>, pub file: PathBuf }
/// Every story (and step) that names `target` in progress, uses or flags; ids resolved through the resolver; a story file that does not read is skipped.
pub fn uses_of(project: &Project, resolver: &Resolver, target: &UseTarget) -> Result<Vec<StoryUse>, KernelError>;
```

- [ ] **Step 1: Scheiternde Tests:** `uses.rs` `a_trainer_named_in_a_step_is_found_with_story_and_step`, `progress_var_counts_as_a_use_without_a_step`, `an_unreadable_story_is_skipped`; Löschplan: `deleting_a_trainer_a_story_mentions_warns_and_does_not_lock` (Fixture: Trainer ohne harte Verweise, Geschichte mit `uses: [{ trainer }]` → Plan `allowed`, `mentions` eine Zeile; `delete_entity` legt den Grabstein und antwortet `warnings` mit der Zeile), `a_hard_reference_still_locks_with_the_story_beside_it`; `schema.rs` `a_use_of_a_deleted_entity_is_dangling`; Frontend-Test des Dialogs `mentions_are_listed_without_locking`.
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS; gated `vanilla_delete_over_the_command_path` (3d.4) bleibt grün; Browser auf `3d-6-web`: Trainer aus der Geschichte löschen → Dialog mit „Erwähnt in Geschichten“, Löschen geht, ⌘Z; Screenshot `blick-loeschen-erwaehnt-hell.png`.
- [ ] **Step 5: Commit** `feat(stories): stories as weak references in the delete plan, dangling-ref for tombstones` (mit Trailer).

### Task 11: Geschichte löschen: Datei entfernen als Undo-Schritt, `story_delete`

**Files:**
- Modify: `crates/g4-kernel/src/stories/entities.rs` (`delete_story`, `delete_story_files`), `crates/g4-kernel/tests/stories.rs`, `apps/studio/src-tauri/src/commands/stories.rs` (`story_delete_impl`), `dispatch.rs`, `lib.rs`, `mcp/{access,catalog}.rs` (`story_delete` Write), 3d.4s `delete_entity` (wenn es `kind` als Text nimmt: `"story"` leitet an `delete_story` weiter; sonst bleibt `story_delete` der einzige Weg, im Commit nennen), `crates/g4-kernel/src/bin/g4k.rs` (`g4k story delete`), `apps/studio/src/api.stories.ts` (`storyDelete`), `apps/studio/src/stories/StoryDetail.tsx` (+ test: Menü „Aktionen“ › „Geschichte löschen…“, Dialog „Geschichte „X“ löschen?“ mit „Rückgängig mit ⌘Z“, Aktionen „Abbrechen“, „Löschen“), `docs/mcp.md`, `docs/formats/stories.md` („Löschen“)

**Interfaces:**
- Produces:

```rust
/// Removes JSON, layout and the register entry as one file step (label „Geschichte „<name>“ gelöscht“); undo brings all three back.
pub fn delete_story(store: &mut Store, id: &str) -> Result<StoryReport, KernelError>;
pub fn delete_story_files(project: &Project, id: &str) -> Result<StoryReport, KernelError>;
pub fn story_delete_impl(state: &AppState, id: String) -> Result<StoryReport, CmdError>;
```

- [ ] **Step 1: Scheiternde Tests:** `delete_removes_both_files_and_the_register_entry_as_one_undo_step` (nach `undo` byte-gleich zurück, auch das Layout mit Positionen), `delete_of_an_unknown_id_is_unknown_id`, `a_deleted_number_is_free_again` (nächstes `create` nimmt `0000`), `story_delete_over_the_command_path`, Dispatch/Katalog/Zugriff, `StoryDetail.test.tsx` `deleting_asks_and_calls_story_delete`.
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS; Browser: Geschichte löschen, ⌘Z; `git status` nach ⌘Z wie vor dem Löschen.
- [ ] **Step 5: Commit** `feat(stories): delete a story as one undoable file step over app, MCP and g4k` (mit Trailer).

---

## Abschluss

### Task 12: Design-System-Nachtrag, Gate-Datei, Doku, Roadmap

**Files:**
- Create: `docs/phase-3d-6-gate.md`
- Modify: `design-system/README.md` (Komponenten-Tabelle: Zeile „Story-Knoten“ wird „Start, Dialog, Bedingung, Kampf, Belohnung, Fortschritt, Erscheinen/Verschwinden; Wert, Ort; Ruhe, Gewählt, nur notiert, lose“ mit Quelle `ui/primitives/StoryNode.tsx` und „Graph-Ansicht über `@xyflow/react` mit `@dagrejs/dagre`, nur gelesen in 3d.6“; neue Zeile „Fäden-Übersicht“; Screens-Tabelle „Geschichten: Übersicht (Fäden)“ und „Geschichte: Graph“ → umgesetzt; „Was die App umsetzt“ + 3d.6 (Bereich Geschichten frei); Abschnitt „Nachtrag 3d.6“ nach dem Muster „Nachtrag 3d.3“: Quelle (`stories/*`, `ui/primitives/StoryNode*`, `stories/graph/*`), Plan- und Gate-Verweis, Texte und `data-*` aus Task 6–8 und 11, Vergleichsbilder unter `target/agent/gate-<datum>/3d-6/`, Abweichungen: kein Deck, Schritte nur lesen, Knoten nicht ziehbar, Layout nur gelesen, „die `.fig` ist zurzeit nicht erreichbar, die Frames folgen“), `design-system/Audit-Design-System.md` (Zeilen Story-Knoten, Fäden-Übersicht, Geschichte: Graph), `docs/agent-gate.md` (Abschnitt „Phase 3d.6“ nach dem Muster „Phase 3d.3“ `:1152-1198`: Kopien, Ports 1760/1761, Wo was ist, Tabelle Punkt/Wer/Wie, die Befehle; die Gate-Datei in der Liste oben), `docs/mcp.md` („Bekannte Grenzen“: Schritte nur über `story_update`; „Noch nicht umgesetzt“: Rezepte der Geschichten 3d.7), `docs/dev-setup.md` (Tabelle der gated Tests: `threads.rs`, `stories.rs`, der Studio-Test; der Lizenzschritt), `docs/ROADMAP.md` (Zeile 3d.6 mit Stand; „Aus dem Design-System, später“: Geschichten und Story-Graph → „3d.6 umgesetzt (lesen), 3d.7 erzählen“; Zeile „Flags und Variablen“ ergänzt: Fläche seit 3d.6 im Bereich Geschichten), `docs/formats/stories.md`, `docs/formats/text_project.md`, `docs/formats/flags_vars.md` (letzter Abgleich mit dem Code)

**Interfaces:**
- Consumes: alles Vorige; die Zeilen `threads …`, `graph …`, `stories …`, `story-git …` der gated Tests; die Chunk-Größe aus Task 6; die Screenshots aus Task 6–8, 10, 11.

- [ ] **Step 1: Gate-Dokument** `docs/phase-3d-6-gate.md` nach dem Muster von `docs/phase-3d-3-gate.md`: Kopf mit Stand (Branch, Basis, Kopf, Teil A/Teil B), Gate-Text aus Spezifikation §4 (Zeile 3d.6) als Zitat; Legende „Wer“ mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (`[data-geschichten=threads|stories|flags]`, `[data-thread-row=<var>]`, `[data-thread-graph]`, `[data-node-count]`, `[data-story-node][data-kind]`, `[data-noted-only]`, `[data-loose]`, `[data-new-story]`, `[data-new-story-dialog]`, `[data-progress-kind]`, `[data-story-row=<id>]`, `[data-story-detail]`, `[data-story-notes]`, `[data-story-empty-graph]`, `[data-delete-mentions]`; Sätze „Noch keine Werte im Spiel“, „Noch keine Schritte. Rezepte legen sie an (kommt mit 3d.7).“, „Keine Fäden: die Scripts sind nicht übernommen.“, `NOT_A_TEXT_PROJECT`, „Geschichten sperren nicht; der Verweis wird als offen gemeldet.“); Vorbereitung (Kopien `_testkopien/3d-6-{pt,hgde,hgus}` für (1), `3d-6-kpt`, `3d-6-khgde` für (2), `3d-6-web` (Platin DE), `3d-6-hgweb` (HeartGold DE) für (3), je `.nds` daneben; alle als Textprojekt über `adopt_text_project`; Test-Umgebung 1760/1761; Last mit `uptime`).
  - **(1) Automatik:** `scripts/gate.sh` grün (mit dem neuen Schritt `npm licenses`); einzeln je Projekt im Release-Profil: `vanilla_threads_count_at_least_the_measured` (≥ 59 / ≥ 62, die Zeilen `threads …`), `vanilla_thread_graph_of_the_first_thread_has_a_node_per_value` mit `G4_TIMING_STRICT=1`, `text_project_with_a_story_builds_from_base_byte_equal`, der Studio-Test `stories_over_the_command_path_on_the_vanilla_copy`; `cargo deny --locked check licenses advisories` und `bash scripts/check-licenses-npm.sh` grün, die Chunk-Größe des Graphs aus `pnpm build`.
  - **(2) Szenario kopflos:** auf `3d-6-kpt`: `studio mcp --project` → `list_threads` (Zahl ≥ 59), `thread_graph {"var":"VAR_…"}` (Wert-Knoten == Werte des Fadens), `story_create` → `git status` zeigt nur die zwei Dateien, `undo` → leer; `story_create` erneut, `rename_id` eines Trainers aus einem Schritt (`story_update` mit `uses`) → die JSON trägt den neuen Namen; `diff_entity {"kind":"story","index":0}` gegen `HEAD` nach einem Commit → `Fields` mit Pfaden; `plan_delete` des Trainers → `mentions` mit der Geschichte, `delete_entity` → Grabstein, `warnings`; `story_delete` → `undo`; dasselbe auf `3d-6-khgde` (≥ 62).
  - **(3) Browser:** Geschichten › Fäden (Liste, Graph eines Fadens, Erzählen ohne Nummern, Bauen mit Nummern), „Neue Geschichte…“ → Toast, `git status` in der Kopie, ⌘Z; Detail mit Notizen, Schritten (über MCP angelegt), Graph mit Schritten auf den Wert-Knoten; Löschen eines Trainers mit „Erwähnt in Geschichten“; Geschichte löschen und ⌘Z; Galerie-Block „Story-Knoten“; Blick Hell, Dunkel, Kompakt (Bilder aus Task 6–8, 10, 11).
  - **(4) Hand und Blick beim Nutzer:** die Fäden-Namen gegen die Spielkenntnis (sind die 59 Fäden die Handlung?), der Graph eines bekannten Fadens (etwa die Galaktik-Handlung) am Bild, die Knotenarten Fortschritt und Erscheinen/Verschwinden als eigene Knoten (Entscheidungen „bewusst nicht entschieden“), `.fig`-Frames folgen.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle: Spiel | Fäden | Werte (Summe) | größter Faden (Werte / Orte) | `threads()` warm (ms) | `thread_graph` (ms) | Geschichte anlegen (ms) | Graph-Chunk (kB gzip) | Last.
- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(3d.6): …` mit Regressionstest), bis alles grün ist; `scripts/gate.sh` im Hintergrund, allein, aus dem Worktree mit absolutem `G4_ROMS_DIR` und `tools`-Symlink; `target/agent/gate-*` vor `git worktree remove` sichern.
- [ ] **Step 3: Doku und Roadmap** wie unter Files; Roadmap-Zeile: „**Stand <Datum>:** Gate vom Agenten bestanden: Fäden-Übersicht mit <n> (Platin DE) / <n> (HeartGold DE) / <n> (HeartGold US) Fäden aus dem Index, Graph mit einem Knoten je Wert, Geschichte über App, MCP und `g4k` als ein Undo-Schritt (`git status` nur JSON und Layout), `text_project_in_git_builds_from_base_byte_equal` mit Geschichte grün, `rename_id` zieht mit, Löschen warnt mit der Geschichte, Diff als Felder, `cargo deny` und npm-Lizenzen grün mit `@xyflow/react`; Branch `phase-3d-6-geschichten-lesen`; [docs/phase-3d-6-gate.md](phase-3d-6-gate.md); beim Nutzer offen: Fäden gegen die Spielkenntnis, Knotenarten am Bild, `.fig`“.
- [ ] **Step 4: Commit** `docs(3d.6): design-system addendum, gate of threads, stories and the graph, roadmap status` (mit Trailer).

---

## Reihenfolge und Parallelität

- Seriell: **1 → 2 → 3 → 4 → 5 → 7 → 8**; **6** kann parallel zu 2–5 laufen (nur Frontend, `package.json`, `gate.sh`); 7 braucht 5 und 6.
- **9** erst, wenn 3d.4 auf main ist; davor sind Teil A und ein Zwischenbericht fertig (die Gate-Datei entsteht in Task 12; der Zwischenbericht nennt die Zahlen der gated Tests aus Task 3 und 5).
- **10 → 11 → 12**; 10 und 11 berühren 3d.4s Dateien erst nach dem Merge.
- Die gated Tests laufen je Task auf frischen Kopien `3d-6-{pt,hgde,hgus}`; die Kopien bleiben bis Task 12 und werden danach gelöscht (`Manual`, `_saves` bleiben).

## Abschluss

Nach Task 12: `superpowers:finishing-a-development-branch` (Merge nach `main` erst nach grünem `scripts/gate.sh` und unabhängiger Abnahme über die Test-Umgebung, Muster Phase 4); die Erfahrung aus Phase 4 gilt: Konflikte an Anhängepunkten gesammelt lösen, `.fig` von main behalten und neu rendern.

## Offen für die Umsetzung

- **Werte je Familie:** welche HGSS-Befehle Werte setzen und vergleichen, sagt die Datenbank (`script_flagvars.yaml` `values`); fehlt einem der drei gemessenen Befehle das Gegenstück, bleibt die Untergrenze 62 dennoch das Ziel, und der Commit von Task 1 nennt die Abweichung.
- **`FlagVarParam` und `Ref` nach 3d.4:** erweitert 3d.4 die Verweise deklarativ (neue Ziele, neue Felder), bleibt `value` am Ende des Structs; die Konfliktlösung in Task 9 nimmt beide Seiten.
- **Header-Namen:** `header_names` aus `commands/flags.rs` (3d.1) wird für `list_threads` gebraucht; ob es `pub(crate)` bleibt oder in einen Helfer zieht, entscheidet Task 5 (im Commit nennen).
- **Lösch-API von 3d.4:** die Namen der Kernel-Datei und der Report-Felder von `plan_delete`/`delete_entity` stehen erst nach dem Merge fest; Task 10 und 11 passen `mentions`/`warnings` und `delete_entity {kind: "story"}` daran an, die Regeln dieses Plans bleiben.
- **Graph-Chunk:** liegt die gemessene Größe beider Pakete über 100 kB gzip, bleibt die Entscheidung (eigener Chunk), die Zahl kommt in die Gate-Tabelle; die Alternative (eigenes SVG) wird nur gezogen, wenn React Flow in jsdom nicht testbar ist (dann im Commit begründen).
- **Phosphor-Symbole:** `thread` und `story` brauchen Namen, die `@phosphor-icons/react` 2.1.10 kennt (`GitBranch`, `Scroll` oder `BookBookmark`); Task 6 prüft es.
- **Erzählen-Ansicht:** mit `geschichten` in `NARRATING_AREAS` zeigen die Daten-Hände dieses Bereichs die Tiefe „Erzählen“; 3d.6 nutzt sie nur in der Fäden-Übersicht (keine Nummern); weitere Erzählen-Ansichten (Layouts) sind 3d.8.
- **Knotenarten Fortschritt und Erscheinen/Verschwinden:** als eigene Knoten umgesetzt (Empfehlung des Gutachtens); bleibt der Nutzer beim Blick bei Fortschritt als Kantenbeschriftung, ist das ein Posten für 3d.7 (die Art bleibt im Schema).
- **`.fig`:** OpenPencil war nicht erreichbar; die Frames „Story-Knoten“, „Fäden-Übersicht“, „Geschichte: Graph“ folgen, die README trägt die Beschreibung sofort.
