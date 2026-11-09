# Phase 3d.2 „Erzählen-Rahmen“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die Tiefe bekommt app-weit den dritten Wert `"erzaehlen"` (Segment im Scharnier mit drei Optionen, Helfer `depthAtLeast`/`viewDepth` statt Gleichheit, alle direkten Vergleiche umgestellt, ein Lint im Gate hält es so); in Erzählen zeigt der Inspector je Art nur die Felder mit `from: "erzaehlen"` und Picker statt Zahlen; die Rezepte Gebiet anlegen, Raum anlegen, Gebiet erweitern und Welt leeren liegen als Kernel-Modul `recipes` mit `plan` (ohne Schreiben, liefert einen Entwurf) und `apply` (**ein** Undo-Schritt, Scripts als `FileStep`) vor, erreichbar über MCP `recipe_list`/`recipe_plan`/`recipe_apply` und `g4k recipe`; die Schau zeigt den Entwurf als Schicht mit gestrichelten Zellen und halbtransparenten Markern, die Assistenten zeigen ihn vor dem Anlegen; das Ergebnis nennt den nächsten sinnvollen Schritt als Rezept-Taste; ⌘Z nach „Welt leeren“ stellt auch die Scripts wieder her.

**Architecture:** Frontend: `shellState.ts` trägt `Depth`, `DEPTHS`, `readDepth`, `depthAtLeast`, `viewDepth` und `NARRATING_AREAS`; `App.tsx` rechnet einmal `viewDepth(area, depth)` und reicht das Ergebnis weiter, das Scharnier bekommt den rohen Wert; `scripts/check-depth.sh` verbietet direkte Vergleiche außerhalb von `shellState.ts`. `FieldSpec.from`/`EventFieldSpec.from` mit `shownFields` filtern den Inspector, neue Felder `LookField` (Aussehen über `LookPicker`), `WarpField` (Tür als Ziel-Warp) und die Wörter `MOVEMENT_WORDS`. Kernel: `crates/g4-kernel/src/recipes/` mit `mod.rs` (Typen `RecipeId`, `RecipeCall`, `Draft`, `Outcome`, `NextStep`, Verteiler `plan`/`apply`/`list`), `area.rs`, `room.rs`, `extend.rs` (die Logik aus `src-tauri/src/commands/wizards.rs`, Spezifikationen und Berichte ziehen mit) und `empty_region.rs` (die Anwendung aus `templates.rs`, mit `FileStep`s über `scripts::{snapshot_script, script_file_step}` und `Store::squash_since` zu einem Schritt); die Tauri-Befehle werden dünne Hüllen. MCP: drei Werkzeuge, `recipe_apply` mit `Kind::Depends` (unumkehrbar, wenn das Rezept es sagt). Szene: `world/scene/draft.ts` baut die Schicht „Entwurf“ (eigene `THREE.Group` am `SceneHost`, nie von den Ladern berührt), `MatrixScene` bekommt `draft`, das Panel `[data-draft][data-draft-count]`; `AreaGrid` zeichnet `[data-draft-cell]`. Die Assistenten rufen `recipePlan` mit ihrer gebauten Spezifikation (300 ms entprellt) und reichen den Entwurf über den `WizardHost` an die App; das Ergebnis zeigt `NextStep`s als `RecipeButton`; in Erzählen ersetzt `RecipeBar` die Werkzeugleiste.

**Tech Stack:** Rust (g4-kernel `Store`, `scripts`, `templates`, serde), Tauri 2 (Befehle, Brücke `dispatch.rs`, MCP `tools.rs`/`catalog.rs`/`access.rs`), React 19 mit den Bausteinen aus `apps/studio/src/ui/`, three.js (`LineDashedMaterial`), vitest, Bash (Lint wie `check-styles.sh`), `g4emu`-Szenarien.

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) (§1, §2, §3.3, §4 Zeile 3d.2, §6, §7, §8); [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Tabellen „Phasenschnitt und Reihenfolge“, „Geschichten und Erzählen (Gutachten A)“ Zeilen Tiefe Erzählen, Rezepte, Live-Vorschau; „Nicht in 3d“); [gutachten-a-geschichten-erzaehlen.md](../../research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md) §2.3 „Was der Code heute trägt“, §3.3 „Die Tiefe „Erzählen““ (Helfer, Lint, `FieldSpec.from`, Tabelle je Bereich, Rezepttabelle, Live-Vorschau, Mindestumfang), §5 Zeile E1 mit Gate; [Spezifikation 3c](../specs/2026-10-23-phase-3c-doppeldeck-design.md) §7.7 Tiefen; [design-system/README.md](../../../design-system/README.md) (Prinzip 3 und 7, Komponenten „Segmente“ und „Werkzeug-Taste, Rezept-Taste“, „Was die App umsetzt“, Nachtrag 4.4 als Muster). Muster für Rezepte: [Plan 3g.2](2026-10-25-phase-3g2-leere-region.md) (`plan_empty_region`/`apply_empty_region`); Muster für Aufbau und Gate: [Plan 4.4](2026-10-28-phase-4-4-git-minimal.md), [phase-4-4-gate.md](../../phase-4-4-gate.md).

Stand 31.10.2026: `main` bei `3c194c7` (Phase 4 abgeschlossen, Spezifikation 3d mit Schnitt 3d.1 bis 3d.9). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-2`, Branch `phase-3d-2-erzaehlen-rahmen` (existiert, Basis `3c194c7`). Parallel laufen in eigenen Worktrees 3d.1 (`phase-3d-1-flags-variablen`), 3d.3 (`phase-3d-3-sprites-ds-eier`) und 4.5 (`phase-4-5-veroeffentlichen`). Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`.

## Befunde vor dem Plan (31.10.2026)

- `grep -rnE '(===|!==) "(tuefteln|bauen)"' apps/studio/src --include='*.ts' --include='*.tsx' | grep -v '\.test\.'` findet 88 Zeilen in 45 Dateien; darunter `shellState.ts:149` (der Leser des gespeicherten Werts, bleibt) und `TechnikArea.tsx:214` `entry === "bauen"` (der Technik-Eintrag „Bauen“, keine Tiefe). Jede Zeile mit der Tiefe hat `depth` oder `.depth` auf der linken Seite (`useShell().state.depth === "tuefteln"` in fünf Daten-Händen).
- `Hinge.tsx` `DEPTHS` hat zwei Einträge; `Segmented` nimmt beliebig viele Optionen mit `icon`. `App.tsx` reicht `shell.state.depth` an neun Stellen weiter (Z. 364, 908, 960, 1114–1120, 1186, 1212, 1270, 1281); `WizardHost.tsx` und die fünf Daten-Hände lesen `useShell().state.depth` selbst.
- `FieldSpec` (`fields.ts`) und `EventFieldSpec` (`eventFields.ts`) kennen keine Tiefe; `headerSections` teilt in „Karten“, „Aussehen“, „Texte und Scripts“; `Field.tsx` rendert `pick`-Felder als `Reference` plus `Picker`; `LookPicker` (Assistenten) zeigt Aussehen mit Kartenbild; `HeaderPicker` wählt einen Header; `TrainerField` steht am Script-Feld einer Figur; `FACINGS` benennt die Blickrichtung.
- `wizards.rs` (2 609 Zeilen, 74 Tests): `create_area_impl` (Z. 706), `extend_area_impl` (794), `create_room_impl` (1078) arbeiten auf `Session` nur für `project.family` und `project.header_count()`; alles Übrige geht über `&mut Store` (`duplicate_map_with`, `create_matrix`, `create_header`, `create_event_file`, `duplicate_event_file`, `apply_dynamic_headers`, `edit_matrix`, `edit_event`, `edit_map`, `transaction`). Heute sind Gebiet (Matrix, dann Header mit Ort) zwei und Raum (Matrix, Header, zwei Warps, bis zwei Türkacheln) bis sechs Undo-Schritte; `CmdError::partial` meldet „angelegt, dann gescheitert“ (`KernelError::Partial(String)` gibt es).
- `templates::apply_empty_region` schreibt Rand-Karte, Level-Script-Datei (`new_script_file_reported` + `write_source`), das Start-Script (`append_source`, kompiliert) und lädt die berührten Textarchive mit `reload_from_disk`, dann eine `Store::transaction` `REGION_LABEL`. `Store::record_file_step(label, FileStep)` legt einen eigenen Changeset an; `Store::squash_since(mark, label)` fasst Parts **und** FileSteps der Schritte seit `mark` zu einem zusammen. `FileStepKind` hat `Reset` und `IdRename`; `scripts::script_step` baut einen `FileStep` nur für das Zurücksetzen.
- `SceneHost` hält `content`, `markers`, `cellFrame` (eigene Gruppe, überlebt den Inhalt); `buildMarkers` (`markers.ts`) baut Marker aus `EventsDto`; `AreaGrid` zeichnet `[data-extend]`-Zellen; `WorldMapPanel` zeigt Weltkarte und Raumplan.
- MCP: `COMMANDS` und `match` in `dispatch.rs`, `COMMAND_KINDS` in `access.rs` (`Kind::Read`, `Write`, `Depends`, `Irreversible(&str)`), `ToolDef` in `catalog.rs` (`name`, `title`, `description`, `input_schema`, `output_schema`, `route`), `NOT_IN_BATCH` in `tools.rs`; `apply_empty_region` ist `Irreversible("Welt leeren")`, `create_area`/`create_room` sind `Depends` (unumkehrbar mit `header.allow_patch`).
- Szenarien: `pt-raum-assistent.yaml` ruft `create_room` mit `spec`, `pt-leere-region.yaml`/`hg-leere-region.yaml` rufen `apply_empty_region {}` und prüfen `plan.start_header`; sie bleiben wörtlich unverändert.

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Rang der Tiefen | `DEPTHS = ["erzaehlen", "bauen", "tuefteln"]`; `depthAtLeast(depth, min)` vergleicht den Rang; `=== "tuefteln"` wird `depthAtLeast(depth, "tuefteln")`, `!== "tuefteln"` wird `!depthAtLeast(depth, "tuefteln")`, `=== "bauen"` (die Wörter des Spiels) wird `!depthAtLeast(depth, "tuefteln")`, so spricht Erzählen wie Bauen; „nur in Erzählen“ heißt `!depthAtLeast(depth, "bauen")` | zwei Helfer decken alle 88 Stellen; keine Stelle fällt still in Tüfteln |
| `viewDepth` | `viewDepth(area, depth)` gibt `"erzaehlen"` nur für Bereiche in `NARRATING_AREAS` (in 3d.2 genau `["welt"]`), sonst `"bauen"` für Erzählen; `App.tsx` rechnet es einmal (`const depth = viewDepth(shell.state.area, shell.state.depth)`) und reicht es an Werkbank, Inspector, Assistenten, Statuszeile und Daten-Hände; das Scharnier bekommt `shell.state.depth` (das Segment zeigt „Erzählen“ gedrückt, auch in Technik) | ein Bereich ohne Erzählen-Ansicht gleicht Bauen; die Daten-Hände lesen die Tiefe aus `useShell()`, darum liefert `ShellContext` zusätzlich `viewDepth` für den aktiven Bereich |
| Gespeicherter Wert | `readDepth(v)`: `"erzaehlen"`, `"bauen"`, `"tuefteln"` wörtlich, alles andere `"bauen"`; `g4.shell.v1` bleibt | Entscheidung „unbekannte Werte als Bauen“; kein `v2` |
| Lint | `scripts/check-depth.sh` nach `check-styles.sh`: in `apps/studio/src` (`*.ts`, `*.tsx`, ohne `*.test.*`, ohne `ui/frame/shellState.ts`) scheitert jede Zeile mit `[Dd]epth[A-Za-z0-9_.]*[[:space:]]*(===\|!==)[[:space:]]*"`, mit `(===\|!==)[[:space:]]*"(erzaehlen\|tuefteln)"` oder mit `case[[:space:]]+"(erzaehlen\|tuefteln)"`; Ausgabe `datei:zeile [depth]`, Exit 1; Schritt „depth“ in `scripts/gate.sh` nach „styles“ | die linke Seite jeder Tiefenstelle heißt `depth`; `"bauen"` allein ist auch der Technik-Eintrag, darum nur mit `depth` davor; Tests dürfen Werte setzen |
| Segment | drei Optionen „Erzählen“ (Symbol `narrate`, neu in `icons.ts`: Phosphor `BookOpenText`), „Bauen“ (`build`), „Tüfteln“ (`tinker`), `ariaLabel` „Tiefe“ | Design-System „Segmente: Erzählen, Bauen, Tüfteln“ |
| `from` | `FieldSpec.from?: Depth` und `EventFieldSpec.from?: Depth`, Standard `"bauen"` (`fieldFrom(spec)`); `shownFields(specs, depth)`: in Erzählen nur `from === "erzaehlen"`, in Bauen und Tüfteln alle (Tüfteln ergänzt „Technisch“ wie bisher) | Spezifikation §3.3; ein Feld ab Bauen gibt es in 3d.2 nicht, darum keine Stufe dazwischen |
| Felder in Erzählen | **Figur:** `graphics_id` „Aussehen“ (Picker mit Sprite), `movement_type` „Bewegung“ (Auswahl mit Wörtern `MOVEMENT_WORDS`, ein Wert ohne Wort bleibt „Bewegung N“), `dir` „Blickrichtung“ (`FACINGS`), `script` nur als `TrainerField` „Trainer“ (die Nummer ist verborgen); **Tür:** `dest_header_id` „Ziel“ (Referenz mit `HeaderPicker`, vorhanden) und `dest_warp_id` „Tür“ (`WarpField`: Picker über die Warps der Eventdatei des Ziel-Headers, „Tür 0 · x 6, z 10“); **Schild** und **Auslöser:** keine Felder, eine `Notice` (Info) „Der Text des Schilds kommt mit 3d.5.“ bzw. „Geschichte und Schritt kommen mit 3d.7.“; **Gebiet und Raum:** ein Abschnitt „Ort“ mit `location_name` „Ort“ (`PlacePicker`, vorhanden), `area_data_id` „Aussehen“ (`LookField` über `LookPicker`), `music_day_id` „Musik (Tag)“, `music_night_id` „Musik (Nacht)“ als Zahlfeld ohne Dez-Hex; der Name steht im Kopf, die Zeile „Name“ trägt „Umbenennen…“ über `onRename` des Weltbaums (`RenameTarget`) | Gutachten A §3.3; Dialog und „Sichtbar ab Schritt“ sind 3d.5/3d.7; für Musik gibt es keine Namenstabelle (README-Abweichung, Picker mit 3d.8) |
| `plan` nimmt `&mut Store` | `plan(store: &mut Store, …)`, weil der Store Assets beim Lesen lädt; „ohne Schreiben“ prüft je Rezept ein Test (`undo_depth`, `dirty()` und die Member-Zahlen unverändert, kein neuer Pfad auf der Platte) | `Store::matrix`, `event`, `header_table` sind `&mut self` |
| Form der Rezepte | `enum RecipeId { CreateArea, CreateRoom, ExtendArea, EmptyRegion }` (serde `create_area`, `create_room`, `extend_area`, `empty_region`), `RecipeCall { recipe, input: Value }`; je Rezept ein Modul mit `plan(store, &Input) -> Result<Draft>` und `apply(store, &Input) -> Result<Outcome<Report>>`; `recipes::plan`/`apply`/`list` verteilen; ein Trait gibt es nicht | vier Rezepte mit vier Eingabetypen; ein Enum trägt `recipe_list` und die Schemata ohne Objektsicherheit; 3d.5 und 3d.7 hängen Varianten an |
| Ein Undo-Schritt | Member entstehen vorher (nicht rückgängig, wie heute, `created` im Ergebnis); dann **eine** `Store::transaction` mit dem Label des Rezepts (Gebiet `Gebiet „{name}“ angelegt`, Raum `Raum „{name}“ angelegt`, Erweitern `extend area` wie heute, Welt leeren `REGION_LABEL`); Dateischritte werden mit `record_file_step` angefügt und über `squash_since(mark, label)` mit der Transaktion zu einem Schritt verschmolzen | Entscheidung „jedes Rezept ein Undo-Schritt“; `squash_since` trägt Parts und FileSteps |
| `FileStep` für Scripts | neue Variante `FileStepKind::Recipe`; `scripts::snapshot_script(project, file) -> ScriptSnapshot` (Quelle `.rotom`/`.json`, Member, Eintrag in `compile-state.json`) vor dem Schreiben, `scripts::script_file_step(project, file, &before) -> FileStep` danach (ein Pfad ohne Datei vorher ist `before: None`, undo entfernt ihn); die vom Compile geschriebenen Textarchive bekommen je einen `FileStep` ihres Members (`asset` das Archiv); undo und redo eines `Recipe`-Schritts laden `step.asset` nach dem Schreiben mit `reload_from_disk`, wenn es geladen ist | ⌘Z stellt Quelle, Member, Compile-Eintrag und Archive wieder her; die Rand-Karte (ein Map-Member) bleibt als unverwiesene Datei, wie bisher gesagt |
| `Draft` | `Draft { cells, tiles, events, messages, notes, irreversible }`: Zellen `(matrix: Option<u16>, x, y, source_map)` (`None`: eine Matrix, die erst entsteht), Kacheln `(matrix, x, z)` (Türkacheln), Events `(kind, matrix, x, z, look, target)`, Nachrichten `(archive, text)`; `draft_count(matrix)` = Zellen + Kacheln + Events mit `matrix == Some(matrix)` | Gutachten A §3.3 Live-Vorschau; die Zählung der Test-API muss aus demselben Plan folgen |
| `Outcome` | `Outcome<R> { report: R, created: Vec<CreatedObject>, change: Option<ChangeId>, changed: Vec<AssetId>, next: Vec<NextStep> }`; die Tauri-Befehle antworten weiter mit `AreaReport`, `ExtendReport`, `RoomReport`, `RegionReport` (= `outcome.report`, Form unverändert), MCP und `g4k` mit `OutcomeDto = Outcome<Value>` | die Szenarien und `api.ts` bleiben; die neuen Felder kommen über `recipe_apply` |
| `NextStep` | `NextStep { id, label, available, reason }`; Tabelle: Gebiet → `connect_doors` „Tür verbinden“, `new_figure` „Neue Figur hier“; Raum → `new_figure`; Erweitern → `adjust_terrain` „Gelände anpassen“ (`reason` „Kommt mit Phase 6“); Welt leeren → `create_area` „Gebiet anlegen“; `available` = `recipes::list()` kennt die `id`, sonst `reason` „Kommt mit 3d.5“ | Gutachten A §3.3 Rezepttabelle; die Taste ist gesperrt, bis das Rezept da ist |
| Unumkehrbar über MCP | `recipe_plan` und `recipe_list` `Kind::Read`; `recipe_apply` `Kind::Depends`: `create_area`/`create_room` mit `input.header.allow_patch == true` → `Irreversible("Header-Patch einspielen")`, `empty_region` → `Irreversible("Welt leeren")`, sonst `Write`; `recipe_apply` steht in `NOT_IN_BATCH`; die alten Befehle `create_area`, `create_room`, `extend_area`, `apply_empty_region` behalten ihre Klassen | Muster `--allow-irreversible`; neue Member und die Rand-Karte bleiben nach ⌘Z |
| Kopflos | `g4k recipe list`, `g4k recipe plan <projekt> <rezept> [--input <json>]`, `g4k recipe apply <projekt> <rezept> [--input <json>] [--allow-irreversible]` (ohne Schalter lehnt ein unumkehrbares Rezept mit dem Satz von `needs_confirmation` ab); `apply` speichert den Store danach und druckt `OutcomeDto` als JSON | Gate (2) ohne App; `g4k template` bleibt |
| Entwurf in der Szene | `world/scene/draft.ts`: `setDraft(h, draft, matrix)` baut in `h.draft` (eigene `THREE.Group`, am `scene` wie `cellFrame`) je Zelle einen Rahmen aus `LineSegments` mit `LineDashedMaterial` (`dashSize` 1, `gapSize` 0.5, Farbe `szene.auswahl.halo`, `computeLineDistances`), je Kachel einen kleinen Rahmen, je Event einen Marker wie `buildMarkers` mit `opacity` 0.5 (`transparent`); `MatrixScene` bekommt `draft: DraftDto | null`, der Host trägt `data-draft=""` und `data-draft-count` (Zahl der gezeichneten Zellen, Kacheln und Marker); die Lader fassen `h.draft` nie an | Gutachten A §3.3 Punkt 2; kein neues Token (die Auswahlfarbe gestrichelt heißt „geplant“) |
| Entwurf außerhalb der Szene | `AreaGrid` bekommt `draftCells?: readonly { x: number; y: number }[]`, zeichnet sie gestrichelt mit `data-draft-cell="x,y"`; `WorldMapPanel` reicht die Zellen des Entwurfs weiter, deren `matrix` die gezeigte ist; Zellen mit `matrix: null` (neue Matrix eines Raums) zeigt kein Gitter, der Entwurfssatz im Assistenten nennt sie | Gutachten A §3.3 Punkt 3; die neue Matrix hat noch keinen Raumplan |
| Woher der Entwurf kommt | die Assistenten rufen `recipePlan(recipe, input)` mit der gebauten Spezifikation, sobald sie vollständig ist (300 ms entprellt, eine veraltete Antwort wird verworfen), und reichen `Draft | null` über `onDraft` an den `WizardHost`, der an die App (`draft` im Zustand, an `MatrixScene` und `WorldMapPanel`); Esc, „Abbrechen“, „Schließen“, ein Fehler des Plans und das Anlegen setzen `null`; `ExtendAreaDialog` ebenso; `ClearWorldWizard` zeigt die Randzellen von `plan_empty_region` als Entwurf | Vorschau und Ergebnis kommen aus demselben Plan |
| Werkzeugleiste in Erzählen | `RecipeBar` (`ui/frame/RecipeBar.tsx`) statt `FloatingToolbar`, wenn `viewDepth` Erzählen ist: Rezept-Tasten „Gebiet anlegen“, „Raum anlegen“ (öffnen die Assistenten), „Neue Figur“, „Tür verbinden“, „Neuer Trainer hier“ (gesperrt, Tooltip „Kommt mit 3d.5“); die Tasten tragen `data-recipe="<id>"`; Werkzeugbuchstaben wirken in Erzählen nicht | Gutachten A §3.3 Tabelle Welt; die Rezepte selbst kommen mit 3d.5 |
| `RecipeButton` | `ui/primitives/RecipeButton.tsx`: Symbol, Name, Bereichsfarbe (`Area` wie `Reference`), Zustände Ruhe, Hover, Aktiv (`aria-pressed`), Gesperrt (`disabled` mit Tooltip `reason`), `data-recipe`; in `index.ts` exportiert, in der Galerie unter „Rahmen“ | Design-System „Werkzeug-Taste, Rezept-Taste: Ruhe, Hover, Aktiv, Gesperrt / 4 Bereiche“ |
| Ergebnis mit Taste | `DoneResult` bekommt `steps?: NextStep[]` und `onStep?: (id) => void`: unter „Als Nächstes“ je Schritt eine `RecipeButton` (gesperrt mit `reason`), darunter die bisherigen Textzeilen; `create_area` öffnet den Assistenten „Gebiet anlegen“, `create_room` „Raum anlegen“ mit `outsideHeader` des neuen Headers | Entscheidung „nächster sinnvoller Schritt als Taste“ |
| Was in `wizards.rs` bleibt | die Tauri-Befehle, `AreaSpec`/`RoomSpec`/… werden aus dem Kernel re-exportiert (`pub use g4_kernel::recipes::area::{AreaSpec, …}`), `late`/`Created` werden `KernelError::Partial` (Text wie heute: `created_text` zieht mit in den Kernel); die 74 Tests bleiben in `wizards.rs` und laufen über die Hüllen, nur Tests, die Undo-Schritte zählen, erwarten einen | Verhalten der Befehle unverändert; die Tests sind die Spezifikation |

## Global Constraints

- Arbeitsort ist der Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-2` auf dem Branch `phase-3d-2-erzaehlen-rahmen` (existiert, Basis main `3c194c7`): nie `cd` in die Hauptkopie, nie den Branch wechseln, nie nach main mergen, kein `git stash`. `tools/` ist dort ein Symlink (nie einchecken).
- Vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`; `pnpm install` einmal in `apps/studio`.
- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Originale unter `$HOME/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur unter `Roms/_testkopien/3d-2-<zweck>` (`cp -cR` für Ordner, `cp -c` für `.nds`, Ordner `3d-2-<zweck>_DSPRE_contents` und `.nds` direkt nebeneinander); `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut. Gated Tests lesen `G4_TEST_PROJECT` und schreiben nur auf `vanilla_copy()`.
- Test-Umgebung nur auf `G4_VITE_PORT=1710 G4_BRIDGE_PORT=1711` (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1710 G4_BRIDGE_PORT=1711 bash scripts/agent/studio-web.sh`).
- Keine ROM-Daten ins Repo: keine `.nds`, keine Member, keine exportierte JSON eines Spiels; Fixtures nur synthetisch.
- UI nur aus `apps/studio/src/ui/primitives` und Tokens `--g4-*` (keine Hex-Farben, keine Inline-Styles außer `/* dyn */`); `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` und der neue `bash scripts/check-depth.sh` grün; deutsche Oberfläche, Bezeichner englisch.
- Anhängepunkte nur anhängen: `apps/studio/src-tauri/src/commands/dispatch.rs` (`COMMANDS`, `match`, Beispielargumente), `lib.rs` (`generate_handler!`), `mcp/access.rs` (`COMMAND_KINDS`), `mcp/catalog.rs`, `mcp/tools.rs` (`NOT_IN_BATCH`), `mcp/symbolic.rs`, `crates/g4-kernel/src/lib.rs`, `apps/studio/src/api.ts`.
- Parallel laufen 3d.1 (Flags: `data/refs.rs`, `textproject/*`, `eventFields.ts` bekommt neue Felder, Fläche Flags, `games.yaml`), 3d.3 (`ncgr.rs`, `battle_sprites.rs`, `DSScreen`, Pokédex- und Trainer-Ansichten) und 4.5 (`technik/PublishSurface*`, `patch.rs`, `git.rs`, Titelleiste): diese Dateien in 3d.2 nicht umbauen; in `eventFields.ts` nur `from` an vorhandene Felder und die Helfer ergänzen, keine Felder umsortieren; die Tiefen-Vergleiche in den Dateien dieser Phasen trotzdem umstellen (der Lint gilt überall), das sind kleine Zeilenänderungen, die der Merge trägt.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh`.
- Nicht in 3d.2 (Entscheidungen, wörtlich): die neuen Rezepte selbst (Neue Figur, Tür verbinden, Neuer Trainer hier: 3d.5; Story-Rezepte: 3d.7), der Dialog-Editor, Erzählen-Layouts für Trainer, Begegnungen und Texte (3d.8), Geschichten (3d.6); Modell (b) „Geschichten erzeugen Scripts beim Build“ nie.
- Die Szenarien `pt-raum-assistent.yaml`, `pt-leere-region.yaml`, `hg-leere-region.yaml` bleiben wörtlich unverändert und grün.

## Review Focus

- Ein gespeicherter Rahmen mit `depth: "erzaehlen"` wird von einem älteren Stand der App gelesen, oder ein neuer Stand liest `"experte"` aus einer Hand-Bearbeitung: beides liest sich als Bauen, nichts bricht (Task 1 `reads_an_unknown_depth_as_bauen`).
- Nutzer in Erzählen wechselt nach Technik oder Pokémon: die Bereiche zeigen Bauen, das Segment zeigt weiter „Erzählen“, und zurück in der Welt ist der reduzierte Inspector wieder da (Task 1 `view_depth_falls_back_to_bauen_outside_narrating_areas`, Task 2 `event_fields_per_depth_and_kind` Snapshot).
- „Raum anlegen“ mit Eingang, der Nutzer drückt Esc, während der Plan noch läuft: die späte Antwort darf den Entwurf nicht zurückbringen, `[data-draft-count]` ist 0, `git status` leer (Task 8 `a_late_plan_answer_after_cancel_is_dropped`).
- `recipe_apply` von `empty_region`, danach ⌘Z, dann ⌘⇧Z: Scripts, Level-Script-Datei und Textarchive sind vor und nach jedem Schritt byte-gleich mit dem jeweiligen Stand; ein Script, das jemand zwischendurch von Hand änderte, lässt ⌘Z mit `Refused` ab, ohne eine Datei anzufassen (Task 4 `undo_and_redo_of_the_empty_region_restore_every_file`, `undo_refuses_a_script_changed_meanwhile`).
- `create_room` scheitert nach der Kopie der Karte (etwa gesperrte Eventdatei beim Warp): die Antwort ist `Partial` mit dem Text der angelegten Member, der Verlauf hat **keinen** halben Schritt (die Transaktion wird verworfen), die Member liegen auf der Platte (Task 3 `a_failed_room_leaves_no_half_step_and_names_the_members`).

---

### Task 1: Drei Tiefen, Helfer, Lint, Umstellung der Vergleiche

**Files:**
- Create: `scripts/check-depth.sh`
- Modify: `apps/studio/src/ui/frame/shellState.ts`, `shellState.test.ts`, `apps/studio/src/ui/frame/Hinge.tsx`, `Hinge.test.tsx`, `apps/studio/src/ui/frame/ShellContext.tsx` (`viewDepth` im Kontext), `apps/studio/src/ui/primitives/icons.ts` (`narrate`), `apps/studio/src/App.tsx` (einmal `viewDepth`), `scripts/gate.sh` (Schritt „depth“ nach „styles“), `docs/dev-setup.md` (Prüfskripte), und alle Dateien des grep aus „Befunde“ (ohne `shellState.ts` und ohne `TechnikArea.tsx:214`)

**Interfaces:**
- Produces:

```ts
// ui/frame/shellState.ts
export type Depth = "erzaehlen" | "bauen" | "tuefteln";
/** The depths in their order: Erzählen tells, Bauen builds, Tüfteln shows the ROM. */
export const DEPTHS: readonly Depth[] = ["erzaehlen", "bauen", "tuefteln"];
/** The stored value as a depth; anything unknown reads as Bauen. */
export function readDepth(value: unknown): Depth;
/** Whether `depth` is `min` or deeper (Bauen is at least Erzählen, Tüfteln is at least both). */
export function depthAtLeast(depth: Depth, min: Depth): boolean;
/** The areas that have a view of their own in Erzählen; every other area shows Bauen there. */
export const NARRATING_AREAS: readonly AreaId[] = ["welt"];
/** The depth `area` shows for the chosen `depth`: Erzählen only in a narrating area, else Bauen; Bauen and Tüfteln as chosen. */
export function viewDepth(area: AreaId, depth: Depth): Depth;
// ui/frame/ShellContext.tsx
interface ShellContextValue { /* … */ setDepth: (depth: Depth) => void; /** `viewDepth(state.area, state.depth)` */ viewDepth: Depth; }
// ui/frame/Hinge.tsx
const DEPTHS_OPTIONS = [{ value: "erzaehlen", label: "Erzählen", icon: "narrate" }, { value: "bauen", label: "Bauen", icon: "build" }, { value: "tuefteln", label: "Tüfteln", icon: "tinker" }] as const;
```

- Umstellung nach der Tabelle „Rang der Tiefen“; `readShellState` liest `depth: readDepth(s.depth)`; `App.tsx` ersetzt jedes `shell.state.depth` außer dem für `Hinge` durch `depth` (= `shell.viewDepth`); `WizardHost.tsx` und die fünf Daten-Hände lesen `useShell().viewDepth`; `layout.ts` `panelsFor` und `dockRules.ts` `syncDepthPanels` bekommen die Ansichtstiefe (der Reiter „Raster“ kommt nur mit `depthAtLeast(depth, "tuefteln")`).
- `scripts/check-depth.sh`: Aufbau wie `check-styles.sh` (`set -euo pipefail`, `find … | sort`, `grep -nE`, `report`), Ausnahmen `*.test.*` und `apps/studio/src/ui/frame/shellState.ts`, die drei Muster aus „Entscheidungen“, am Ende `check-depth: ok` oder die Treffer und Exit 1.

- [ ] **Step 1: Scheiternde Tests** (`shellState.test.ts`, `Hinge.test.tsx`):
  - `reads_erzaehlen_and_keeps_bauen_and_tuefteln` (`readDepth` je Wert).
  - `reads_an_unknown_depth_as_bauen` (Review Focus: `"experte"`, `null`, `7` → `"bauen"`; ein gespeicherter Rahmen mit `depth: "erzaehlen"` liest `"erzaehlen"`).
  - `depth_at_least_follows_the_order` (alle neun Paare: `depthAtLeast("erzaehlen","bauen") === false`, `depthAtLeast("tuefteln","erzaehlen") === true`, …).
  - `view_depth_falls_back_to_bauen_outside_narrating_areas` (Review Focus: `("welt","erzaehlen") → "erzaehlen"`, `("technik","erzaehlen") → "bauen"`, `("pokemon","tuefteln") → "tuefteln"`, `("gegenstaende","bauen") → "bauen"`).
  - `Hinge` `has_three_depths_and_switches_to_erzaehlen` (gedrückt „Bauen“; Klick „Erzählen“ ruft `onDepth("erzaehlen")`; `aria-label` „Tiefe“); der Test `switches_the_depth_and_does_nothing_for_the_one_that_is_active` bleibt grün.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm exec vitest run src/ui/frame` → FAIL.
- [ ] **Step 3: Umsetzen** `shellState.ts`, `Hinge.tsx`, `icons.ts`, `ShellContext.tsx`; dann `scripts/check-depth.sh` schreiben und laufen lassen: er nennt die Stellen; jede nach der Tabelle umstellen, bis `check-depth: ok` steht (`App.tsx` zuerst, dann die Werkbank, die Assistenten, die Daten-Hände). Dabei die Tests der berührten Dateien weiter grün (sie rendern mit `depth: "bauen"`/`"tuefteln"`; für `"erzaehlen"` kommen die Tests in Task 2).
- [ ] **Step 4: Lauf** `pnpm test && pnpm build`, `bash scripts/check-depth.sh` → 0 Treffer, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`; der grep aus „Befunde“ findet außerhalb von `shellState.ts` nur noch `TechnikArea.tsx:214` (`entry`, keine Tiefe).
- [ ] **Step 5: Gate und Doku** `scripts/gate.sh`: `run_step "depth" "" bash scripts/check-depth.sh` nach dem Schritt „styles“; `docs/dev-setup.md` nennt das Skript bei den Prüfskripten.
- [ ] **Step 6: Commit** `feat(shell): third depth erzaehlen with depthAtLeast and viewDepth, every direct comparison replaced, lint check-depth.sh in the gate` (mit Trailer).

### Task 2: `from` an den Feldern, reduzierter Inspector

**Files:**
- Create: `apps/studio/src/workbench/inspector/LookField.tsx`, `WarpField.tsx`, `inspectorFields.test.ts` (Snapshot der Feldlisten)
- Modify: `apps/studio/src/workbench/inspector/fields.ts` (`from`, `fieldFrom`, `shownFields`), `eventFields.ts` (nur `from` an vorhandene Felder, `MOVEMENT_WORDS`, nichts umsortieren), `headerSections.ts`, `EventEditor.tsx`, `HeaderEditor.tsx`, `AreaEditor.tsx`, `InspectorHead.tsx` (Wort je Art in Erzählen = Bauen), `EventEditor.test.tsx`, `HeaderEditor.test.tsx`, `headerSections.test.ts`, `eventFields.test.ts`

**Interfaces:**
- Consumes: Task 1 (`Depth`, `depthAtLeast`), `LookPicker` und `lookItems` (`workbench/wizards/`), `HeaderPicker`, `TrainerField`, `useEvents` (`workbench/wizards/NewRoomWizard.tsx`, zieht nach `workbench/useEvents.ts`, falls nicht exportiert), `spriteTexture`/`peekSprite` (`world/sprites.ts`).
- Produces:

```ts
// inspector/fields.ts
export interface FieldSpec { /* … */ /** The lowest depth that shows the field; absent: Bauen. */ from?: Depth; }
export const fieldFrom = (spec: { from?: Depth }): Depth => spec.from ?? "bauen";
/** The fields `depth` shows: in Erzählen those from Erzählen, in Bauen and Tüfteln all of them. */
export function shownFields<T extends { from?: Depth }>(specs: readonly T[], depth: Depth): T[];
// inspector/eventFields.ts
export interface EventFieldSpec { /* … */ from?: Depth; }
/** What a figure does, in words (pret movement types); a stored value without a word shows as „Bewegung N“. */
export const MOVEMENT_WORDS: { value: number; label: string }[];
// inspector/headerSections.ts
export type HeaderSectionId = "maps" | "look" | "texts" | "ort";   // "ort": the one section of Erzählen
// inspector/LookField.tsx
export function LookField(props: { value: number; headers: readonly HeaderSummary[]; template: HeaderSummary | null; disabled?: boolean; onCommit: (areaDataId: number) => void }): JSX.Element;
// inspector/WarpField.tsx
export function WarpField(props: { destHeader: HeaderSummary | undefined; value: number; disabled?: boolean; onCommit: (warp: number) => void; onError: (e: unknown) => void }): JSX.Element;  // Picker „Tür wählen“, Einträge „Tür {index} · x {x}, z {z}“
```

- `from: "erzaehlen"` tragen: Header `location_name`, `area_data_id`, `music_day_id`, `music_night_id`; Objekt `graphics_id`, `movement_type`, `dir`, `script`; Warp `dest_warp_id` (und `destinationSpec`); alle übrigen Felder bleiben ohne `from`. Labels in Erzählen: `area_data_id` „Aussehen“, `graphics_id` „Aussehen“, `music_*` „Musik (Tag)“/„Musik (Nacht)“, `dest_warp_id` „Tür“ (die Umbenennung passiert in `headerSections`/`eventFieldSpecs` nur für `!depthAtLeast(depth, "bauen")`).
- `headerSections(family, depth)` in Erzählen: `[{ id: "ort", title: "Ort", specs: shownFields(…) }]`; `HeaderEditor.fieldRow` rendert `area_data_id` in Erzählen als `LookField` (Vorlage = der Header selbst), `script` einer Figur in Erzählen nur als `TrainerField`, Zahlfelder ohne `showBase`; `EventEditor` zeigt in Erzählen für `coord` die `Notice` „Geschichte und Schritt kommen mit 3d.7.“ und für `bg` „Der Text des Schilds kommt mit 3d.5.“ statt der Felder; die Zeile „y“, `AddEvent` und der Hinweis zum Ziehen bleiben.

- [ ] **Step 1: Scheiternde Tests:**
  - `inspectorFields.test.ts` `event_fields_per_depth_and_kind` (Review Focus): `toMatchInlineSnapshot` der Schlüssel aus `shownFields(eventFieldSpecs(kind, family), depth)` für `kind` ∈ object, warp, coord, bg × `family` ∈ `"Pt"`, `"Hgss"` × `depth` ∈ drei Tiefen; `header_fields_per_depth_and_family` (DP, Pt, HGSS × drei Tiefen aus `headerSections`); `area_fields_per_depth` (Hgss, Dppt). Erzählen enthält genau die Schlüssel aus der Aufzählung oben; Bauen und Tüfteln sind gleich.
  - `eventFields.test.ts` `movement_words_name_the_common_types` (jeder Eintrag hat ein Wort ohne Zahl; die Werte sind eindeutig).
  - `EventEditor.test.tsx` `erzaehlen_shows_look_movement_facing_and_trainer_only` (Figur: Zeilen „Aussehen“, „Bewegung“, „Blickrichtung“, „Trainer“; keine „x (Kachel)“, keine „Lokale ID“, kein Dez-Hex-Schalter), `erzaehlen_shows_a_door_as_target_and_door` („Ziel“ mit Referenz, „Tür“ mit Picker „Tür wählen“, Einträge aus der Eventdatei des Ziels), `erzaehlen_tells_sign_and_trigger_what_comes_later` (die zwei Sätze), `bauen_and_tuefteln_show_every_field_as_before` (bestehende Tests grün).
  - `HeaderEditor.test.tsx` `erzaehlen_shows_one_section_ort_with_place_look_and_music` (Abschnitt „Ort“; `LookField` mit „Aussehen wie …“; Musik als Zahlfeld ohne Schalter; keine „Karten“); `headerSections.test.ts` `erzaehlen_has_the_section_ort_only`.
- [ ] **Step 2: Lauf** `pnpm exec vitest run src/workbench/inspector` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces (CSS nur über die vorhandenen Module).
- [ ] **Step 4: Lauf** → PASS; `pnpm test && pnpm build`, die drei Prüfskripte grün.
- [ ] **Step 5: Commit** `feat(inspector): FieldSpec.from and EventFieldSpec.from, the reduced inspector of Erzählen with pickers for look, movement, door and trainer` (mit Trailer).

### Task 3: Kernel-Modul `recipes`: Typen, Gebiet, Raum, Erweitern aus `wizards.rs`

**Files:**
- Create: `crates/g4-kernel/src/recipes.rs` (Modul-Doku, Verteiler, Re-Exporte) und der Ordner `crates/g4-kernel/src/recipes/{types.rs,area.rs,room.rs,extend.rs}` (Muster `git.rs` + `git/`), `crates/g4-kernel/tests/recipes.rs` (synthetisch und gated)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod recipes;` am Ende), `apps/studio/src-tauri/src/commands/wizards.rs` (Typen und Logik raus, `pub use g4_kernel::recipes::{area::*, room::*, extend::*}`, die drei `*_impl` rufen den Kernel, `late`/`Created` entfallen), `apps/studio/src-tauri/src/commands/dispatch.rs` (nur Importe)

**Interfaces:**
- Consumes: `Store::{duplicate_map_with, create_matrix, create_header, create_event_file, duplicate_event_file, apply_dynamic_headers, dynamic_headers_applied, transaction, undo_depth, member_count, locked, header_table, matrix, event, map, project}`, `Tx::{edit_matrix, edit_header, edit_event, edit_map}`, `places::{check_place, plan_new_place, apply_new_place, set_location_name, location_bank}`, `Project::{family, header_count, internal_names}`, `tiles::{DOOR, DOOR_COLLISION, warp_fires}`.
- Produces:

```rust
// recipes/types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum RecipeId { CreateArea, CreateRoom, ExtendArea, EmptyRegion }
impl RecipeId { pub fn title(self) -> &'static str; /* „Gebiet anlegen“, „Raum anlegen“, „Gebiet erweitern“, „Welt leeren“ */ pub fn all() -> &'static [RecipeId]; }
/// `input` as the recipe's module reads it; absent or `null` reads as `{}` (so `empty_region` needs none).
#[derive(Debug, Clone, Deserialize)] pub struct RecipeCall { pub recipe: RecipeId, #[serde(default = "empty_object", deserialize_with = "null_as_object")] pub input: serde_json::Value }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct DraftCell { pub matrix: Option<u16>, pub x: u8, pub y: u8, pub source_map: u16 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct DraftTile { pub matrix: Option<u16>, pub x: u16, pub z: u16 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")] pub enum DraftEventKind { Object, Warp, Coord, Bg }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct DraftTarget { pub header: u16, pub warp: Option<u16> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftEvent { pub kind: DraftEventKind, pub matrix: Option<u16>, pub x: u16, pub z: u16, pub look: Option<u16>, pub target: Option<DraftTarget> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct DraftMessage { pub archive: Option<u16>, pub text: String }
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Draft { pub cells: Vec<DraftCell>, pub tiles: Vec<DraftTile>, pub events: Vec<DraftEvent>, pub messages: Vec<DraftMessage>,
                   /// German sentences for the wizard („Legt Raum „Keller“ … an“).
                   pub notes: Vec<String>, /// What apply does that undo does not take back („Karte 0187 wird kopiert“).
                   pub irreversible: Vec<String> }
impl Draft { /// Cells, tiles and events of `matrix`: what the scene of that matrix draws. pub fn count(&self, matrix: u16) -> usize; }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct CreatedObject { pub kind: AssetKind, pub index: u16 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextStep { pub id: String, pub label: String, pub available: bool, pub reason: Option<String> }
#[derive(Debug, Clone, Serialize)]
pub struct Outcome<R> { pub report: R, pub created: Vec<CreatedObject>, pub change: Option<ChangeId>, pub changed: Vec<AssetId>, pub next: Vec<NextStep> }
pub type OutcomeDto = Outcome<serde_json::Value>;
impl<R: Serialize> Outcome<R> { pub fn into_dto(self) -> Result<OutcomeDto, KernelError>; }
/// `NextStep`s for `ids` with `available` from `list()`; an unknown id gets `reason` „Kommt mit 3d.5“, `adjust_terrain` „Kommt mit Phase 6“.
pub fn next_steps(ids: &[(&str, &str)]) -> Vec<NextStep>;
// recipes.rs
#[derive(Debug, Clone, Serialize)] pub struct RecipeInfo { pub id: RecipeId, pub title: &'static str, pub irreversible: bool /* without input: empty_region true */ }
pub fn list() -> Vec<RecipeInfo>;
/// Whether applying `call` does something undo does not take back (the access check of `recipe_apply`).
pub fn is_irreversible(call: &RecipeCall) -> Result<Option<&'static str>, KernelError>;  // Some("Header-Patch einspielen"), Some("Welt leeren"), None
pub fn plan(store: &mut Store, call: &RecipeCall) -> Result<Draft, KernelError>;
pub fn apply(store: &mut Store, call: &RecipeCall) -> Result<OutcomeDto, KernelError>;
// recipes/area.rs (die Typen aus wizards.rs: EventFileChoice, AreaCell, NewHeader, PlaceChoice, AssignHeader, AreaSpec, AreaTarget, AreaReport, auto_internal_name; dazu)
pub fn plan(store: &mut Store, spec: &AreaSpec) -> Result<Draft, KernelError>;
pub fn apply(store: &mut Store, spec: &AreaSpec) -> Result<Outcome<AreaReport>, KernelError>;
// recipes/room.rs (Entrance, RoomSpec, DoorTile, RoomReport)
pub fn plan(store: &mut Store, spec: &RoomSpec) -> Result<Draft, KernelError>;
pub fn apply(store: &mut Store, spec: &RoomSpec) -> Result<Outcome<RoomReport>, KernelError>;
// recipes/extend.rs (ExtendSpec, ExtendReport)
pub fn plan(store: &mut Store, spec: &ExtendSpec) -> Result<Draft, KernelError>;
pub fn apply(store: &mut Store, spec: &ExtendSpec) -> Result<Outcome<ExtendReport>, KernelError>;
// recipes/types.rs (shared)
pub(crate) struct Created { pub patch: bool, pub headers: Vec<u16>, pub matrices: Vec<u16>, pub maps: Vec<u16>, pub event_files: Vec<u16> }
pub(crate) fn created_text(created: &Created) -> String;              // wörtlich wie heute
pub(crate) fn late(created: &Created, step: &str, e: impl Display) -> KernelError;  // Partial(created_text; step failed: e) oder Refused(e), wenn nichts angelegt
```

- Fehler: die `CmdError`-Texte von heute werden `KernelError::Refused(<Text>)`; ein Scheitern nach dem ersten Member `KernelError::Partial(<created_text>; <step> failed: <e>)`; die Tauri-Hülle bildet `Partial` auf `CmdError::partial` ab (prüfen, ob `From<KernelError> for CmdError` das schon tut).
- `plan` je Rezept: dieselben Prüfungen wie `apply` bis vor das erste Schreiben (`check_area`/`check_target`, `check_assign`, `plan_entrance`, `plan_place`), dann der Entwurf: Gebiet → `cells` je `AreaCell` (`matrix` = `target.matrix` oder `None`), `notes` die Sätze der Zusammenfassung des Assistenten in Bauen-Worten (eine Zeile je Zelle genügt nicht: „Legt Gebiet „{name}“ mit {n} Karten an.“, „Header {id} zeigt auf die neue Weltkarte.“), `irreversible` „{n} Karten werden kopiert“, „Der Header-Patch wird eingespielt“ (nur mit `allow_patch` und fehlendem Patch); Raum → eine Zelle `(None, 0, 0, source_map)`, Events Warp außen `(Some(matrix des Außen-Headers), x, z, target { header: spec.header.id, warp: room_index })` und Warp innen `(None, inside_x, inside_z, target { outside_header, outside_index })`, `tiles` je Türkachel, die `plan_entrance` setzt; Erweitern → eine Zelle `(Some(matrix), x, y, source_map)`.
- `apply` je Rezept: Prüfungen, Member (Patch, Header, Karten, Matrix, Eventdatei; wie heute, in `created`), dann **eine** `store.transaction(label, |tx| …)` mit allen Edits von heute (`edit_matrix`, Ort über `places::apply_new_place(tx, …)` plus `edit_header`, `edit_event` für die Warps, `edit_map` für die Türkacheln); `changed` = die Assets der Transaktion plus die neuen Member; `next` aus der Tabelle „`NextStep`“.
- `wizards.rs` danach: nur noch die drei `#[tauri::command]` und `*_impl` (Lock, `session.writable()`, `recipes::area::apply(&mut session.store, &spec)?.report`, `session.after_write()`), die Re-Exporte und die 74 Tests.

- [ ] **Step 1: Scheiternde Tests** (`tests/recipes.rs`, `test_support::synthetic_project()`; Muster `tests/templates.rs`):
  - `list_names_the_four_recipes_with_titles` (Reihenfolge `RecipeId::all()`; `empty_region.irreversible`).
  - `plan_of_an_area_writes_nothing` (zwei Zellen, neuer Header mit `allow_patch: false`: `cells.len() == 2`, `matrix == None`; danach `undo_depth`, `dirty()`, `member_count` je Art und die Dateiliste unter `unpacked/` unverändert).
  - `plan_of_a_room_names_both_doors_and_the_door_tiles` (Eingang auf einer Nicht-Tür-Kachel mit `door_tiles: true`: zwei Events `Warp`, `tiles.len() == 2`, `count(outside matrix) == 2` (ein Warp, eine Kachel), `irreversible` nennt die Kartenkopie).
  - `apply_of_an_area_is_one_undo_step` (`undo_depth` + 1; `undo()` stellt Matrix und Header byte-gleich her; die Kartenkopien bleiben Member; Label `Gebiet „…“ angelegt`).
  - `apply_of_a_room_with_an_entrance_is_one_undo_step` (Warps, Header, Matrix, Türkacheln in einem Schritt; `report` wie `create_room_impl` heute: `outside_warp`, `inside_warp`, `door_tiles`; `next == [new_figure gesperrt „Kommt mit 3d.5“]`).
  - `a_failed_room_leaves_no_half_step_and_names_the_members` (Review Focus: Außen-Eventdatei gesperrt über einen Store, dessen Codec sie nicht schreibt — wie der Test in `wizards.rs` für gesperrte Dateien — oder `outside_header == header.id`: `Partial`-Text beginnt mit `created_text`; `undo_depth` unverändert; die Karte liegt als Member vor).
  - `extend_apply_keeps_its_label_and_report` (`extend area`, `ExtendReport { map, width, height }`).
  - `is_irreversible_follows_allow_patch_and_empty_region`.
  - Gated (`G4_TEST_PROJECT`, `vanilla_copy()`): `vanilla_room_recipe_matches_the_wizard_scenario` (die Spezifikation aus `pt-raum-assistent.yaml` mit Header 593, Tür (105, 876): `plan.count(0) == 2`, `apply` → `outside_warp == Some(…)`, `door_tiles.len() == 1`, ein Undo-Schritt; nur Platin DE, andere Spiele überspringen mit `eprintln!`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features --test recipes` → FAIL (Modul fehlt).
- [ ] **Step 3: Umsetzen:** Typen und Logik aus `wizards.rs` nach `recipes/` ziehen (Funktionen 1 : 1, `Session` durch `store.project()` ersetzt), `transaction` um die Edits, die Hüllen in `wizards.rs`.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p studio --all-features --lib wizards` (74 Tests; die, die Undo-Schritte zählen, auf einen Schritt angepasst, sonst unverändert), `cargo test -p studio --all-features --lib dispatch`; gated einmal auf Platin DE; `cargo fmt --check`, `clippy`.
- [ ] **Step 5: Szenario** `pt-raum-assistent.yaml` auf einer frischen Kopie `_testkopien/3d-2-raum` (aus `g4k create <rom> <kopie>`): `cd scripts/g4emu && uv run g4emu scenario ../agent/scenarios/pt-raum-assistent.yaml --project <kopie> --out ../../target/agent/3d-2/raum --set NEUER_HEADER=593 --set TUER_X=105 --set TUER_Y=876` → `"ok": true`.
- [ ] **Step 6: Commit** `feat(kernel): recipes module with plan and apply; area, room and extend move from the app into the kernel as one undo step each` (mit Trailer).

### Task 4: „Welt leeren“ als Rezept mit `FileStep`s, ⌘Z stellt die Scripts her

**Files:**
- Create: `crates/g4-kernel/src/recipes/empty_region.rs`
- Modify: `crates/g4-kernel/src/templates.rs` (`apply_empty_region` wird `pub use recipes::empty_region::apply_empty_region`; `RegionReport` bleibt dort), `crates/g4-kernel/src/scripts.rs` (`snapshot_script`, `script_file_step`), `crates/g4-kernel/src/store.rs` (`FileStepKind::Recipe`, Nachladen in `apply_change`), `crates/g4-kernel/tests/templates.rs`, `crates/g4-kernel/tests/recipes.rs`, `docs/formats/templates.md` (Abschnitt „Anwenden“: ein Schritt, Dateischritte), `docs/formats/script_file.md` (nur falls dort die Dateischritte beschrieben sind)

**Interfaces:**
- Consumes: Task 3; `templates::{plan_empty_region, RegionPlan, RegionReport, region_recipe, start_script, wrap_message, REGION_LABEL}`, `scripts::{new_script_file_reported, write_source, append_source, compile, problems_in, rotom_source, declared_slots}`, `Store::{record_file_step, squash_since, undo_depth, reload_from_disk}`.
- Produces:

```rust
// scripts.rs
/// A script file as it is on disk: its sources, its member and its compile-state entry (each `None` when absent).
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptSnapshot { pub rotom: Option<Vec<u8>>, pub json: Option<Vec<u8>>, pub member: Option<Vec<u8>>, pub entry: Option<serde_json::Value> }
pub fn snapshot_script(project: &Project, file: u16) -> Result<ScriptSnapshot, KernelError>;
/// The step from `before` to what is on disk now, kind `Recipe`; a path absent on one side is `None` there.
pub fn script_file_step(project: &Project, file: u16, before: &ScriptSnapshot) -> Result<FileStep, KernelError>;
/// The step of a text archive's member from `before` to the bytes on disk now (kind `Recipe`, asset the archive).
pub fn archive_file_step(project: &Project, archive: u16, before: Option<Vec<u8>>) -> Result<FileStep, KernelError>;
// store.rs
pub enum FileStepKind { Reset, IdRename, /// A recipe's files (scripts, their members, text archives the compile wrote): undo and redo reload the step's asset when it is loaded.
                        Recipe }
// recipes/empty_region.rs
#[derive(Debug, Clone, Copy, Default, Deserialize)] pub struct EmptyRegionInput {}   // `recipe_plan`/`recipe_apply` nehmen `{}`
pub fn plan(store: &mut Store, _: &EmptyRegionInput) -> Result<Draft, KernelError>;   // cells = Randzellen von Matrix 0 (source_map = town_map), messages = [Startnachricht], notes aus RegionPlan
pub fn apply(store: &mut Store, _: &EmptyRegionInput) -> Result<Outcome<RegionReport>, KernelError>;
/// `apply(store, &EmptyRegionInput::default())?.report` (what `templates::apply_empty_region` was).
pub fn apply_empty_region(store: &mut Store) -> Result<RegionReport, KernelError>;
```

- Ablauf von `apply`: Prüfungen wie heute; `mark = store.undo_depth()`; Schnappschüsse: `snapshot_script(town_file)`, die Member-Bytes jedes Textarchivs, das der Compile berühren kann (das Archiv der Stadt: `text_archive_id` des Start-Headers; nach dem Compile zusätzlich jedes in `touched_archives`, dessen Bytes vorher nicht gesichert waren, mit `before` aus der geladenen Store-Kopie, falls geladen, sonst aus der Datei vor dem Compile — darum vor dem ersten Schreiben die Bytes aller Archive lesen, die `declared` Nachrichten der Stadt-Datei nennen; im Zweifel das Archiv der Stadt genügt, siehe „Offen“); Rand-Karte, Level-Script-Datei (vorher `snapshot_script(level)` mit allem `None`), Start-Script, `reload_from_disk` der Archive wie heute; die Transaktion `REGION_LABEL` wie heute; dann `record_file_step(REGION_LABEL, script_file_step(level, &before_level))`, `record_file_step(REGION_LABEL, script_file_step(town_file, &before_town))`, je Archiv `record_file_step(REGION_LABEL, archive_file_step(…))`; `squash_since(mark, REGION_LABEL)`; `change` = die Id des letzten aufgezeichneten Schritts; `RegionReport.changed` wie heute; `next = [create_area „Gebiet anlegen“, available]`.
- `apply_change` (store.rs): nach dem Schreiben der Dateien eines `Recipe`-Schritts `reload_from_disk(step.asset)` für jedes geladene Asset (Textarchive), Scripts sind kein Store-Wert.

- [ ] **Step 1: Scheiternde Tests** (`tests/recipes.rs`, `tests/templates.rs`; synthetisches Projekt mit übernommenen Scripts wie in `tests/templates.rs`):
  - `empty_region_apply_is_one_step_with_file_steps` (`undo_depth` + 1; `history()` nennt `REGION_LABEL` genau einmal; `report.plan.start_header` wie vorher).
  - `undo_and_redo_of_the_empty_region_restore_every_file` (Review Focus: vor `apply` die Bytes der Stadt-Scriptquelle, ihres Members, des Stadt-Textarchivs und die Dateiliste unter `expanded/scripts/` und dem Member-Ordner sichern; nach `apply` weichen sie ab und die Level-Script-Datei existiert; nach `undo()` alles byte-gleich und die Level-Script-Quelle und ihr Member fehlen; `redo()` bringt den Stand nach `apply` zurück; das Store-Archiv entspricht nach `undo` der Datei (`reload_from_disk` gelaufen); die Rand-Karte bleibt Member).
  - `undo_refuses_a_script_changed_meanwhile` (Review Focus: nach `apply` eine Zeile an die Stadt-Quelle hängen; `undo()` → `Refused`; keine der gesicherten Dateien verändert).
  - `plan_of_the_empty_region_counts_the_border_cells` (`cells.len() == plan.border_cells`, `messages[0].text` = die Nachricht der Sprache, `count(0) == border_cells`).
  - `templates_apply_empty_region_still_answers_the_region_report` (die bestehenden Tests in `tests/templates.rs` grün; `apply_is_one_undo_step` dort zählt weiter einen Schritt).
  - Gated: `vanilla_empty_region_undo_restores_the_world` (bestehend) erweitert um die Scripts: nach `undo` `git status`-gleicher Vergleich der Pfade unter `expanded/scripts/` und `.rotom/status/compile-state.json`-Eintrag der Stadt-Quelle wie vorher.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS; gated einmal auf Platin DE und HeartGold DE (Kopien).
- [ ] **Step 5: Szenarien** `pt-leere-region.yaml` und `hg-leere-region.yaml` auf frischen Kopien `_testkopien/3d-2-leer-{pt,hg}` (aus `g4k create`), Ausgabe `target/agent/3d-2/leer-{pt,hg}` → beide `"ok": true`.
- [ ] **Step 6: Doku** `docs/formats/templates.md` „Anwenden“: ein Schritt, welche Dateien der Schritt trägt, was bleibt (Rand-Karte).
- [ ] **Step 7: Commit** `feat(kernel): the empty region as a recipe with file steps for its scripts, one undo step that restores every file` (mit Trailer).

### Task 5: MCP `recipe_list`, `recipe_plan`, `recipe_apply`; `g4k recipe`; `api.ts`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/recipes.rs`, `apps/studio/src/recipes/api.ts` (+ Test; in `api.ts` am Ende re-exportiert)
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `lib.rs` (`generate_handler!` am Ende), `commands/dispatch.rs` (am Ende von `COMMANDS`, `match`, Beispielargumente des Tests `every_command_accepts_the_keys_api_ts_sends`), `mcp/access.rs` (`COMMAND_KINDS` am Ende: `("recipe_list", Read)`, `("recipe_plan", Read)`, `("recipe_apply", Depends)`; die `Depends`-Auflösung ruft `recipes::is_irreversible`), `mcp/catalog.rs` (drei `ToolDef` am Ende), `mcp/tools.rs` (`NOT_IN_BATCH` + `recipe_apply`; Test der Werkzeugzahl, falls die Grenze reißt, mit Begründung heben), `crates/g4-kernel/src/bin/g4k.rs` (`Recipe(RecipeCommand)`), `crates/g4-kernel/tests/cli.rs`, `docs/mcp.md` (Tabelle „Werkzeuge“, Abschnitt „Schreiben und Bestätigung“, „Unumkehrbares“)

**Interfaces:**
- Consumes: Task 3, 4 (`recipes::{list, plan, apply, is_irreversible, RecipeCall, Draft, OutcomeDto, RecipeInfo}`).
- Produces:

```rust
// commands/recipes.rs
pub fn recipe_list_impl() -> Vec<RecipeInfo>;
pub fn recipe_plan_impl(state: &AppState, call: RecipeCall) -> Result<Draft, CmdError>;      // Lock, Session, kein after_write
pub fn recipe_apply_impl(state: &AppState, call: RecipeCall) -> Result<OutcomeDto, CmdError>; // writable, apply, after_write, files_written(&[]) (Scripts und Archive liegen auf der Platte)
#[tauri::command] pub fn recipe_list() -> Vec<RecipeInfo>;
#[tauri::command] pub async fn recipe_plan(app: AppHandle, call: RecipeCall) -> Result<Draft, CmdError>;      // spawn_blocking
#[tauri::command] pub async fn recipe_apply(app: AppHandle, call: RecipeCall) -> Result<OutcomeDto, CmdError>; // spawn_blocking
// dispatch.rs: "recipe_list" => {}, "recipe_plan" => { recipe: RecipeId, input: Option<Value> }, "recipe_apply" => dito (api.ts sendet { recipe, input })
// g4k
enum RecipeCommand { List, Plan { project: PathBuf, recipe: String, #[arg(long)] input: Option<String> }, Apply { project: PathBuf, recipe: String, #[arg(long)] input: Option<String>, #[arg(long)] allow_irreversible: bool } }
```

```ts
// apps/studio/src/recipes/api.ts
export type RecipeId = "create_area" | "create_room" | "extend_area" | "empty_region";
export interface RecipeInfo { id: RecipeId; title: string; irreversible: boolean }
export interface DraftCell { matrix: number | null; x: number; y: number; source_map: number }
export interface DraftTile { matrix: number | null; x: number; z: number }
export interface DraftEvent { kind: "object" | "warp" | "coord" | "bg"; matrix: number | null; x: number; z: number; look: number | null; target: { header: number; warp: number | null } | null }
export interface DraftDto { cells: DraftCell[]; tiles: DraftTile[]; events: DraftEvent[]; messages: { archive: number | null; text: string }[]; notes: string[]; irreversible: string[] }
export interface NextStep { id: string; label: string; available: boolean; reason: string | null }
export interface RecipeOutcome<R = unknown> { report: R; created: { kind: AssetKind; index: number }[]; change: number | null; changed: AssetId[]; next: NextStep[] }
export const recipeList: () => Promise<RecipeInfo[]>;
export const recipePlan: (recipe: RecipeId, input: unknown) => Promise<DraftDto>;
export const recipeApply: <R = unknown>(recipe: RecipeId, input: unknown) => Promise<RecipeOutcome<R>>;
/** What the scene of `matrix` draws of `draft` (cells, tiles and events of that matrix). */
export function draftCount(draft: DraftDto | null, matrix: number): number;
```

- MCP-Texte (englisch, wie der Katalog): `recipe_list` „Lists the recipes …“; `recipe_plan` „Plans a recipe without writing: the cells, tiles, events and messages it would make (a draft the app shows dashed) and what undo would not take back“; `recipe_apply` „Applies a recipe as one undo step … create_area/create_room with header.allow_patch and empty_region cannot be undone completely (new members stay); headless they need --allow-irreversible“. Eingabeschema `{ recipe: enum, input: object }`.
- `g4k recipe plan` druckt `Draft` als JSON; `apply` ohne `--allow-irreversible` bei `is_irreversible == Some(why)` → `error: <why> lässt sich nicht rückgängig machen; mit --allow-irreversible` (Exit 1); sonst `OutcomeDto` als JSON, danach `store.save()`.

- [ ] **Step 1: Scheiternde Tests:**
  - `dispatch.rs`: die drei Befehle im Schlüssel-Test (`("recipe_plan", json!({ "recipe": "extend_area", "input": { … } }))`), `recipe_plan_writes_nothing_and_recipe_apply_is_one_step` (synthetisch: `plan` → `dirty` leer, `history` unverändert; `apply` von `extend_area` → `history().undo.len()` + 1, `changed` nennt die Matrix).
  - `access.rs`: `recipe_apply_is_irreversible_only_with_allow_patch_or_empty_region` (drei Fälle über die `Depends`-Auflösung).
  - `tools.rs`: `recipe_apply_is_never_in_a_batch`; der Katalog-Test, der jede `ToolDef` gegen `COMMANDS` prüft, grün.
  - `tests/cli.rs`: `cli_recipe_list` (vier Zeilen), `cli_recipe_plan_extend_prints_a_draft` (synthetisch, `--input '{"matrix":0,"x":…}'` → JSON mit `cells`), `cli_recipe_apply_empty_region_needs_the_flag` (ohne Schalter Exit 1 mit „Welt leeren“).
  - `recipes/api.test.ts`: Aufrufnamen und `draftCount` (Zellen, Kacheln, Events der Matrix; `null`-Matrix zählt nicht).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS (`cargo test -p studio --all-features --lib`, `cargo test -p studio --test mcp_stdio`, `cargo test -p g4-kernel --test cli`, `pnpm test`).
- [ ] **Step 5: Doku** `docs/mcp.md`: drei Zeilen in der Tabelle, `recipe_apply` in „Unumkehrbares braucht eine Freigabe“ und „Batch“, ein Beispiel-Prompt („Plane einen Raum …“).
- [ ] **Step 6: Commit** `feat(mcp): recipe_list, recipe_plan and recipe_apply; g4k recipe; recipes api in the app` (mit Trailer).

### Task 6: `RecipeButton`, Rezept-Leiste in Erzählen, „nächster Schritt“ als Taste

**Files:**
- Create: `apps/studio/src/ui/primitives/RecipeButton.tsx` (+ `.module.css`, Test), `apps/studio/src/ui/frame/RecipeBar.tsx` (+ `.module.css`, Test), `apps/studio/src/workbench/recipes.ts` (die Tasten der Leiste)
- Modify: `apps/studio/src/ui/primitives/index.ts` (Export am Ende), `apps/studio/src/workbench/wizards/WizardResult.tsx` (`steps`, `onStep`), `WizardResult.module.css`, `apps/studio/src/workbench/wizards/wizardSpecs.ts` (`areaResult`/`roomResult` geben `steps` aus dem Rezept oder, solange die Assistenten die alten Befehle rufen, aus `nextStepsFor(recipe)`), `NewAreaWizard.tsx`, `NewRoomWizard.tsx`, `apps/studio/src/project/ClearWorldWizard.tsx`, `apps/studio/src/app/WizardHost.tsx` (`onStep` öffnet den nächsten Assistenten), `apps/studio/src/App.tsx` (Leiste je Tiefe), Galerie-Seite „Rahmen“ (`RecipeButton` in vier Zuständen)

**Interfaces:**
- Consumes: Task 1 (`viewDepth`), Task 5 (`NextStep`), `Area` und `areaIcon` (`Reference.tsx`), `Tooltip`, `useRovingList`.
- Produces:

```ts
// ui/primitives/RecipeButton.tsx
export type RecipeButtonProps = { id: string; label: string; icon: IconName; area: Area; /** Second line of the tooltip: what the recipe makes. */ hint?: string;
  pressed?: boolean; /** Why the recipe cannot run now; the button is disabled and the tooltip says it. */ reason?: string | null; onClick: () => void; size?: "m" | "s" };
export function RecipeButton(props: RecipeButtonProps): JSX.Element;   // <button data-recipe={id} aria-pressed={pressed} disabled={reason != null}>
// workbench/recipes.ts
export interface RecipeEntry { id: "create_area" | "create_room" | "new_figure" | "connect_doors" | "new_trainer_here"; label: string; icon: IconName; area: Area; hint: string; /** Absent: the recipe runs; set: the button is locked with it. */ reason?: string }
export const RECIPE_BAR: readonly RecipeEntry[];   // Gebiet anlegen, Raum anlegen; Neue Figur, Tür verbinden, Neuer Trainer hier mit reason „Kommt mit 3d.5“
/** The next steps a wizard shows while it still calls the old command (the kernel's table). */
export function nextStepsFor(recipe: "create_area" | "create_room" | "extend_area" | "empty_region"): NextStep[];
// ui/frame/RecipeBar.tsx
export function RecipeBar(props: { entries: readonly RecipeEntry[]; onRecipe: (id: RecipeEntry["id"]) => void; density: Density; anchor: ToolbarAnchor }): JSX.Element;  // role="toolbar" aria-label="Rezepte", schwebt wie die Werkzeugleiste
// workbench/wizards/WizardResult.tsx
export function DoneResult(props: { title: string; lines: string[]; next: string[]; steps?: NextStep[]; onStep?: (id: string) => void }): JSX.Element;
```

- `App.tsx`: in der 3D-Ansicht `viewDepth === "erzaehlen" ? <RecipeBar …/> : <FloatingToolbar …/>` (dieselbe Lage, derselbe Anker); `onRecipe("create_area")` → `wizards.open({ kind: "area" })`, `"create_room"` → `{ kind: "room", prefill: { outsideHeader: selected } }`; die Buchstabenkürzel der Werkzeuge sind in Erzählen stumm.
- `WizardHost.onStep(id)`: `create_room` → `wizards.open({ kind: "room", prefill: { outsideHeader: <neuer Header> } })` nach `created(…)`; `create_area` → `{ kind: "area" }`; gesperrte Schritte haben keinen Klick.

- [ ] **Step 1: Scheiternde Tests:** `RecipeButton.test.tsx` `has_name_icon_area_and_data_recipe`, `locked_with_a_reason_is_disabled_and_says_why`, `pressed_is_aria_pressed`; `RecipeBar.test.tsx` `lists_five_recipes_two_runnable_three_locked_with_3d5`, `click_runs_the_recipe`; `WizardResult.test.tsx` (neu) `steps_render_as_recipe_buttons_locked_with_reason_and_click_calls_on_step`; `App.test.tsx` `erzaehlen_shows_the_recipe_bar_instead_of_the_tools_and_bauen_the_tools`; `NewRoomWizard.test.tsx` `the_result_offers_neue_figur_hier_locked`; `ClearWorldWizard.test.tsx` `the_result_offers_gebiet_anlegen_and_opens_it`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen** mit `frontend-design:frontend-design` für `RecipeButton` (Bereichsfarbe aus den `Reference`-Klassen, Hover `flaeche/hover`, Aktiv Kobalt-Kante, Gesperrt `text/gedaempft`, nie nur Farbe: Gesperrt zeigt ein Schloss-Symbol); **Step 4: Lauf** → PASS, `pnpm build`, Prüfskripte grün.
- [ ] **Step 5: Commit** `feat(ui): RecipeButton, the recipe bar of Erzählen, and next steps as buttons in the wizard results` (mit Trailer).

### Task 7: Schicht „Entwurf“ in der Szene, gestrichelte Zellen in Weltkarte und Raumplan

**Files:**
- Create: `apps/studio/src/world/scene/draft.ts` (+ `draft.test.ts`), `apps/studio/src/world/scene/useDraft.ts`
- Modify: `apps/studio/src/world/scene/SceneHost.ts` (`draft: THREE.Group`, `draftCount`, `dispose`), `apps/studio/src/world/MatrixScene.tsx` (Prop `draft`, `data-draft`, `data-draft-count`), `apps/studio/src/world/scene/useSceneProps.ts` (oder `useDraft.ts`), `apps/studio/src/workbench/AreaGrid.tsx` (`draftCells`, `data-draft-cell`), `AreaGrid.module.css` (gestrichelter Rahmen über `linie/standard`), `apps/studio/src/workbench/WorldMapPanel.tsx` (Prop `draft`), `apps/studio/src/App.tsx` (Zustand `draft`, an Szene und Weltkarte), `MatrixScene.test.tsx`, `AreaGrid.test.tsx`, `WorldMapPanel.test.tsx`

**Interfaces:**
- Consumes: Task 5 (`DraftDto`, `draftCount`), `cellOrigin`, `cellY`, `MAP_SIZE` (`world/layout.ts`), `MARKER_Y` und der Platzhalter der Marker (`markers.ts`), `tokens.ts` `szene.auswahl.halo`.
- Produces:

```ts
// world/scene/draft.ts
/** Replaces the draft layer of `h` with `draft` for `matrix` (null: nothing); returns how many cells, tiles and markers it drew. Does not render. */
export function setDraft(h: SceneHost, draft: DraftDto | null, matrix: MatrixDto | null): number;
export const DRAFT_OPACITY = 0.5;
export const DRAFT_DASH = { dashSize: 1, gapSize: 0.5 };
// world/scene/SceneHost.ts
readonly draft = new THREE.Group();   // added to `scene` beside `cellFrame`; the loaders never touch it
draftCount = 0;
// world/MatrixScene.tsx
interface Props { /* … */ /** The planned cells, tiles and events of an open wizard (null: none); drawn dashed and half transparent, never replaced by a load. */ draft: DraftDto | null }
// host div: data-draft="" data-draft-count={count}
// world/scene/useDraft.ts
export function useDraft(refs: SceneRefs, draft: DraftDto | null, matrix: MatrixDto): number;   // calls setDraft on change and after a load (sceneBuilt), renders
// workbench/AreaGrid.tsx
interface Props { /* … */ draftCells?: readonly { x: number; y: number }[] }   // <button … data-draft-cell="x,y" aria-label="Geplant: Karte bei x 1, y 9">
// workbench/WorldMapPanel.tsx
interface Props { /* … */ draft?: DraftDto | null }   // cells with matrix === shown grid → draftCells
```

- Zellrahmen: ein Rechteck aus vier Segmenten auf Höhe `OVERLAY_Y + cellY + 0.1` um die Zelle (`cellOrigin`, `MAP_SIZE`), `LineDashedMaterial({ color: szene.auswahl.halo, dashSize, gapSize, transparent: true, opacity: DRAFT_OPACITY, depthTest: false })`, `computeLineDistances()`, `renderOrder` 11; Kachelrahmen 1 × 1 ebenso; Marker: ein `Sprite` mit dem Platzhalter der Markerart (`markers.ts`) und `opacity` 0.5 auf `MARKER_Y`; ein `look` lädt kein Sprite (3d.5 ergänzt es). Beim Dispose: Geometrie und Material jedes Kindes.
- `useDraft` läuft nach jedem Aufbau der Szene erneut (`sceneBuilt`), weil `cellY` von der Matrix abhängt; `h.draft` wird nie von `buildContent`, `setMarkers` oder `removeModels` geleert.

- [ ] **Step 1: Scheiternde Tests:** `draft.test.ts` `draws_cells_tiles_and_events_of_the_shown_matrix_only` (Draft mit zwei Zellen (Matrix 0 und `null`), einer Kachel, zwei Events (Matrix 0 und 3): `setDraft(h, draft, matrix0)` → 3 Kinder, Rückgabe 3; `null` → 0 Kinder, Materialien freigegeben), `a_dashed_material_with_half_opacity` (`LineDashedMaterial`, `opacity 0.5`, `computeLineDistances` gerufen: `geometry.attributes.lineDistance` vorhanden); `MatrixScene.test.tsx` `host_carries_data_draft_and_its_count` (mit dem jsdom-Stub der Szene: `[data-draft]` vorhanden, `data-draft-count` „0“ ohne Draft, „3“ mit); `AreaGrid.test.tsx` `draft_cells_render_dashed_with_data_draft_cell`; `WorldMapPanel.test.tsx` `passes_the_draft_cells_of_the_shown_matrix`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Prüfskripte grün.
- [ ] **Step 5: Commit** `feat(scene): draft layer with dashed cell frames and half-transparent markers, data-draft count; dashed draft cells on the world map and room plan` (mit Trailer).

### Task 8: Die Assistenten zeigen den Entwurf aus `recipe_plan`

**Files:**
- Create: `apps/studio/src/workbench/wizards/useDraftPlan.ts` (+ Test)
- Modify: `apps/studio/src/workbench/wizards/NewRoomWizard.tsx`, `NewAreaWizard.tsx`, `apps/studio/src/workbench/ExtendAreaDialog.tsx`, `apps/studio/src/project/ClearWorldWizard.tsx`, `apps/studio/src/app/WizardHost.tsx` (`onDraft`), `apps/studio/src/App.tsx` (`draft` aus dem Host; `ExtendAreaDialog` bekommt `onDraft`), `apps/studio/src/workbench/wizards/WizardFrame.tsx` (nur falls der Entwurfssatz dort steht: eine Zeile `notes` unter der Zusammenfassung), `NewRoomWizard.test.tsx`, `NewAreaWizard.test.tsx`, `ExtendAreaDialog.test.tsx`, `ClearWorldWizard.test.tsx`, `docs/agent-gate.md` (Zeile „Welt, Reiter „3D““: `[data-draft]`)

**Interfaces:**
- Consumes: Task 5 (`recipePlan`, `DraftDto`), Task 7 (`draft` an Szene und Weltkarte), `useStableCallback`.
- Produces:

```ts
// workbench/wizards/useDraftPlan.ts
/**
 * The draft of `recipe` for `input` (null while the input is incomplete, the plan failed or no answer is in yet). Asks 300 ms after
 * the last change; an answer for an older input is dropped; `null` input clears at once. Reports every change through `onDraft`.
 */
export function useDraftPlan(recipe: RecipeId, input: unknown | null, onDraft: (draft: DraftDto | null) => void): { draft: DraftDto | null; error: string | null };
export const DRAFT_DEBOUNCE_MS = 300;
// WizardHost props: onDraft: (draft: DraftDto | null) => void; die Assistenten und der Dialog bekommen dieselbe Prop
```

- Eingabe je Assistent: `NewRoomWizard` die gebaute `RoomSpec` (`built.spec`), sobald `built` ohne Fehler ist, sonst `null`; `NewAreaWizard` die `AreaSpec`; `ExtendAreaDialog` die `ExtendSpec` (sofort, der Dialog hat alle Werte); `ClearWorldWizard` `{}` für `empty_region`, sobald die Vorbedingungen erfüllt sind. Beim Schließen (Esc, „Abbrechen“, „Schließen“, nach dem Anlegen) `onDraft(null)` im Aufräumeffekt des Assistenten. Der Schritt „Zusammenfassung“ zeigt `draft.notes` als Zeilen und `draft.irreversible` hinter der Plakette; ein Fehler des Plans steht als `Notice` (Warnung) „Vorschau nicht möglich: …“, das Anlegen bleibt möglich.

- [ ] **Step 1: Scheiternde Tests:** `useDraftPlan.test.ts` `asks_once_after_the_debounce_and_reports_the_draft`, `a_late_plan_answer_after_cancel_is_dropped` (Review Focus: Eingabe setzen, vor der Antwort `null` → die Antwort kommt, `onDraft` sah zuletzt `null`), `a_failed_plan_reports_null_and_the_error`; `NewRoomWizard.test.tsx` `shows_the_draft_while_the_entrance_is_complete_and_clears_it_on_cancel` (Fake `recipePlan`; `onDraft` mit Draft, Esc → `null`); `NewAreaWizard.test.tsx` und `ExtendAreaDialog.test.tsx` analog; `ClearWorldWizard.test.tsx` `shows_the_border_cells_as_draft_when_ready`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `pnpm build`, Prüfskripte grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (Kopie `_testkopien/3d-2-web` aus Platin DE, `G4_VITE_PORT=1710 G4_BRIDGE_PORT=1711`, Seite `http://127.0.0.1:1710/?project=<Kopie>`): Erzählen wählen, „Raum anlegen“ über die Rezept-Leiste, Schritte bis zur Zusammenfassung mit Eingang bei (105, 876) → `document.querySelector('[data-draft]').dataset.draftCount` gleich `scripts/agent/call.sh recipe_plan '{"recipe":"create_room","input":<spec>}'` → `draftCount(draft, 0)` (Tür und Türkachel: 2); Esc → `0`, `git -C <Kopie> status --porcelain` leer; Bilder `target/agent/3d-2/blick-entwurf-{hell,dunkel,kompakt}.png`, dazu der Inspector einer Figur, einer Tür, eines Gebiets in Erzählen und die Rezept-Leiste (`blick-erzaehlen-*.png`).
- [ ] **Step 6: Commit** `feat(wizards): live draft from recipe_plan in the area, room, extend and clear-world wizards, cleared on cancel` (mit Trailer).

### Task 9: Design-System fortschreiben

**Files:**
- Create: `design-system/openpencil/NACHTRAG-3d-2.md`, `design-system/openpencil/beispiele/erzaehlen-rahmen.body.js`
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 3d.2“ nach „Nachtrag 4.4“; in „Was die App umsetzt“ die Zeile „Die Tiefe Erzählen fehlt …“ unter „Abweichungen“ ersetzen durch den Stand; „Tiefe Tüfteln“-Zeile um Erzählen ergänzen), `design-system/Audit-Design-System.md` (Zeilen „Segmente“, „Rezept-Taste“, „Entwurf“), bei Antwort von OpenPencil `design-system/Editor_DesignSystem.fig`

**Interfaces:**
- Consumes: Texte und Zustände aus Task 1, 2, 6, 7, 8; die Bilder aus Task 8 Step 5.

- [ ] **Step 1: README** „Nachtrag 3d.2“ nach dem Muster „Nachtrag 4.4“: Quelle (`ui/frame/{shellState,Hinge,RecipeBar}.tsx`, `ui/primitives/RecipeButton.tsx`, `workbench/inspector/{fields,eventFields,headerSections,LookField,WarpField}`, `world/scene/draft.ts`, `workbench/AreaGrid.tsx`, `workbench/wizards/{WizardResult,useDraftPlan}`), Plan- und Gate-Verweis, Bilder unter `target/agent/3d-2/`; neues Symbol `narrate` (Phosphor `BookOpenText`), keine neuen Tokens (der Entwurf nutzt `szene/auswahl/halo` gestrichelt bei 50 %); Komponenten: „Segmente“ mit drei Tiefen (Wort je Tiefe, Symbol), „Rezept-Taste“ (Ruhe, Hover, Aktiv, Gesperrt mit Grund und Schloss, Bereichsfarbe, `data-recipe`), „Rezept-Leiste“ (ersetzt in Erzählen die Werkzeugleiste; fünf Tasten, drei gesperrt „Kommt mit 3d.5“), „Entwurf-Schicht“ (gestrichelte Zellrahmen, Kachelrahmen, halbtransparente Marker; Weltkarte und Raumplan `data-draft-cell`; `[data-draft-count]`), „Erzählen-Inspector“ (die Feldtabelle je Art aus Task 2 mit den Sätzen für Schild und Auslöser), „Ergebnis mit nächstem Schritt“; Abweichungen: Musik ohne Picker (keine Namenstabelle, 3d.8), Erzählen nur in der Welt (Geschichten 3d.6, Pokémon und Texte 3d.8), Sprite im Entwurfsmarker erst mit 3d.5, die `.fig` kennt die Frames noch nicht (OpenPencil nicht erreichbar).
- [ ] **Step 2: Vorlage** `beispiele/erzaehlen-rahmen.body.js` nach dem Muster `aenderungen-git.body.js`, `MODE`: `'inspector'` (Frame „App: Welt, Erzählen, Inspector Figur“: Scharnier mit drei Segmenten, „Erzählen“ gedrückt; Rezept-Leiste; Inspector mit Aussehen, Bewegung, Blickrichtung, Trainer), `'entwurf'` (Frame „App: Welt, Erzählen, Entwurf Raum anlegen“: Schrittleiste im Scharnier, gestrichelte Zelle und halbtransparenter Marker in der Schau, Zusammenfassung mit Sätzen), `'brett'` (Brett „Komponenten (App): Rezept-Taste und Segmente“: Rezept-Taste in vier Zuständen und vier Bereichsfarben, Segmente mit drei Tiefen, Ergebnis mit Tasten „Neue Figur hier“ gesperrt).
- [ ] **Step 3: Anleitung** `NACHTRAG-3d-2.md` nach `NACHTRAG-4-4.md`: „Die Datei wurde nicht angefasst“, Frames auf „App: Werkbank“ (nächste freie Spalte, x = Spalte × 1520, y = Reihe × 1000, vorher `get_page_tree`), Brett auf „Komponenten“, Bauen (`bash ../build-screen.sh erzaehlen-rahmen`, `MODE` je Frame, `op.mjs render`, Vergleich per `export_image`), `fix_fig_text.mjs --rgb555`, README-Satz; die Hinweise aus dem Gedächtnis (10 000-Knoten-Grenze, keine Einzeländerungen, bei gesperrtem Bildschirm Zeitüberschreitungen).
- [ ] **Step 4: `.fig`** nur, wenn OpenPencil antwortet; sonst bleibt „die `.fig` kennt das noch nicht, die Frames folgen“.
- [ ] **Step 5: Audit** wie unter Files; `bash scripts/check-tokens.sh` grün.
- [ ] **Step 6: Commit** `docs(design-system): addendum 3d.2 three depths, recipe button and bar, draft layer, Erzählen inspector` (mit Trailer).

### Task 10: Gate 3d.2, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-3d-2-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben; Abschnitt „Phase 3d.2“ mit Befehlen, `data-*`, der Test-API-Zählung; Zeile „Scharnier“: Umschalter mit drei Tiefen), `docs/ROADMAP.md` (Zeile 3d.2 der Tabelle „Phase 3d“ mit Stand; „Aus dem Design-System, später“: Tiefe Erzählen mit Rezept-Rahmen und Live-Vorschau „3d.2 umgesetzt“, die weiteren Teile bei 3d.5, 3d.7, 3d.8), `docs/dev-setup.md` (gated `tests/recipes.rs` in der Tabelle `G4_TEST_PROJECT`)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben der Szenarien und der gated Tests.

- [ ] **Step 1: Gate-Dokument** `docs/phase-3d-2-gate.md` nach dem Muster `docs/phase-4-4-gate.md`: Kopf mit Stand, Branch, Worktree; Gate-Text der Spezifikation (§4 Zeile 3d.2) als Zitat; Legende „Wer“; Namen in der Oberfläche (Segment „Tiefe“ mit „Erzählen“, `[data-recipe]`, `[data-draft][data-draft-count]`, `[data-draft-cell]`, Inspector-Zeilen „Aussehen“, „Bewegung“, „Blickrichtung“, „Trainer“, „Ziel“, „Tür“, „Ort“, „Musik (Tag)“, die Sätze für Schild und Auslöser, Ergebnis-Tasten, Agentenfrage „Welt leeren“); Vorbereitung (Kopien `_testkopien/3d-2-{pt,hgde,hgus}` für (1), `3d-2-k{pt,hgde,hgus}` für (2), `3d-2-web`, `3d-2-mcp`, `3d-2-raum`, `3d-2-leer-{pt,hg}` für (2) und (3); `G4_ROMS_DIR` absolut; Test-Umgebung auf 1710/1711; Last mit `uptime`).
  - **(1) Automatik:** `scripts/gate.sh` grün (mit dem Schritt „depth“); Snapshot-Test `inspectorFields.test.ts` (Feldlisten je Tiefe und Art); `bash scripts/check-depth.sh` 0 Treffer; gated je Projekt `cargo test --release -p g4-kernel --all-features --test recipes -- --nocapture --test-threads 1` und `--test templates`.
  - **(2) Szenario kopflos** (Release-`g4k`, `studio mcp --project <Kopie> [--allow-irreversible]`): `recipe_list` → vier Einträge; `recipe_plan create_room` mit der Spezifikation aus `pt-raum-assistent.yaml` → `cells` 1, `events` 2, `tiles` 1; `recipe_apply extend_area` → ein Eintrag mehr in `history`, `undo` → Matrix wie vorher; `recipe_apply empty_region` ohne Erlaubnis → `needs_confirmation`, mit → `change`, `history` + 1; `git -C <Kopie> status --porcelain` nennt Scripts und Archive; `undo` → `git status` nennt nur die Rand-Karte (und keinen Pfad unter `expanded/scripts/`); `redo`; `pt-raum-assistent`, `pt-leere-region`, `hg-leere-region` grün (Task 3 Step 5, Task 4 Step 5, hier noch einmal auf frischen Kopien auf dem Endstand).
  - **(3) Browser-Teil** (Test-Umgebung, `3d-2-web`): Segment „Erzählen“; Welt zeigt Rezept-Leiste und reduzierten Inspector (Figur, Tür, Schild, Auslöser, Gebiet, Raum); Technik, Pokémon, Texte zeigen Bauen (`[data-dock-tab]`-Zahl wie in Bauen, Inspector der Pokémon-Hand ohne Hexblock); zurück in Welt wieder Erzählen; „Raum anlegen“ bis zur Zusammenfassung → `[data-draft-count]` gleich `recipe_plan` über `call.sh`, Esc → `0`, `git status` leer; Anlegen → Ergebnis mit „Neue Figur hier“ gesperrt „Kommt mit 3d.5“; „Welt leeren“ in Technik (Bauen) → Ergebnis mit Taste „Gebiet anlegen“, Klick öffnet den Assistenten; ⌘Z → `call.sh dirty` leer und `git status` nur die Rand-Karte; MCP über die Brücke: `mcp.sh call recipe_plan …` ohne Rückfrage, `recipe_apply empty_region` → Agentenfrage „Welt leeren“.
  - **(4) Hand und Blick beim Nutzer:** die drei Tiefen in der gepackten App, der Entwurf in Hell, Dunkel, Kompakt (Bilder aus Task 8 Step 5), die Rezept-Leiste mit Tastatur (Roving, Tooltip der gesperrten Tasten).
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | `recipe_plan` Raum (ms) | `recipe_apply` Raum (ms) | `recipe_apply` Welt leeren (ms) | `undo` danach (ms) | Scripts und Archive nach `undo` byte-gleich | Last |
    |---|---|---|---|---|---|---|
    | Platin DE | | | | | | |
    | HeartGold DE | | | | | | |
    | HeartGold US | | | | | | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Befunde beheben (eigene Commits `fix(3d.2): …` mit Regressionstest), bis alles grün ist; `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms bash scripts/gate.sh > target/agent/gate-sh-3d-2.log 2>&1` im Hintergrund, allein; `target/agent/3d-2/` vor `git worktree remove` sichern.
- [ ] **Step 3: Roadmap und Doku** `docs/ROADMAP.md`: in der Tabelle „Phase 3d“ Zeile 3d.2 „**Stand <Datum>:** Gate vom Agenten bestanden: drei Tiefen mit Lint (0 Treffer), Feldlisten je Tiefe im Snapshot, Rezepte Gebiet, Raum, Erweitern, Welt leeren im Kernel als je ein Undo-Schritt (⌘Z stellt nach „Welt leeren“ Scripts und Archive wieder her), `recipe_list`/`recipe_plan`/`recipe_apply` über MCP und `g4k`, Entwurf in Schau, Weltkarte und Raumplan, Szenarien `pt-raum-assistent`, `pt-leere-region`, `hg-leere-region` grün; Branch `phase-3d-2-erzaehlen-rahmen`; [docs/phase-3d-2-gate.md](phase-3d-2-gate.md); beim Nutzer offen: Hand und Blick“; `docs/agent-gate.md` und `docs/dev-setup.md` wie unter Files.
- [ ] **Step 4: Commit** `docs(3d.2): gate of the Erzählen frame, agent gate, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **`MOVEMENT_WORDS`:** die Zuordnung Wert → Wort kommt aus pret (`pokeplatinum` `include/constants/movement.h`, HGSS gleich nummeriert); ohne Zugriff zählt der Implementierer die häufigsten Werte der Testkopie (`get_events` über alle Eventdateien) und benennt mindestens 0 „Steht still“, die Blick-Typen und „Geht umher“; ein Wert ohne Wort zeigt „Bewegung N“ und ist kein Fehler.
- **Textarchive vor dem Compile:** welche Archive der Compile des Start-Scripts schreibt, weiß erst `CompileReport::touched_archives`; der Plan sichert vorab das Archiv der Stadt (`text_archive_id` des Start-Headers). Berührt der Compile ein weiteres Archiv, dessen Bytes vorher nicht gesichert waren, nimmt `archive_file_step` `before` aus der geladenen Store-Kopie (sie ist der Stand vor dem Compile, `reload_from_disk` läuft erst danach) und meldet es im Testlauf; bleibt ein Archiv unsicherbar, sichert `apply` vor dem Compile alle Archive, die `declared_slots`-Nachrichten der Stadt-Datei nennen.
- **`apply_change` und `Recipe`-Schritte:** ob das Nachladen eines Textarchivs nach dem Schreiben der Datei im Store schon geschieht (`reload_from_disk` steht in `apply_change` für Reset-Schritte?) prüft Task 4 zuerst; fehlt es, kommt es für `FileStepKind::Recipe` dazu, die anderen Arten bleiben.
- **`ScriptSnapshot` und `compile-state.json`:** der Eintrag der Quelle wird über `state_entry`/`set_state_entry` gelesen und geschrieben wie in `script_step`; der Schlüssel ist `state_key(project, &source)`; für eine neue Datei ist `before.entry == None`.
- **Undo-Zähler in den Tests von `wizards.rs`:** Tests, die nach „Raum anlegen“ mehrere `undo()` erwarten, werden auf einen Schritt umgestellt; ihr Verhalten (Matrix, Header, Warps, Türkacheln wie vorher) bleibt die Prüfung.
- **`Depends` in `access.rs`:** die Auflösung für `recipe_apply` liest `args.recipe` und `args.input.header.allow_patch`; der Test `the_depends_commands_are_resolved_like_their_targets` (oder wie er heißt) bekommt die drei Fälle.
- **Werkzeugzahl:** reißt `tool_count_is_about_thirty` (oder der heutige Grenzwert) mit drei Werkzeugen, hebt Task 5 die Grenze mit Begründung im Commit.
- **`useEvents` aus dem Raum-Assistenten:** für `WarpField` muss der Hook aus `NewRoomWizard.tsx` exportiert oder nach `workbench/useEvents.ts` gezogen werden; die Tests des Assistenten bleiben.
- **Rename der Zeile „Name“:** `onRename(target: RenameTarget, name)` des Weltbaums wird über den `HeaderEditorContext` gereicht; fehlt ein Dialog für Gebiete (nur Räume haben „Umbenennen…“), zeigt die Zeile bei Gebieten den Namen ohne Taste.
- **Entwurf für eine neue Matrix:** Zellen mit `matrix: null` zeigt kein Gitter; der Entwurfssatz nennt sie („Der Raum entsteht auf einer neuen Weltkarte.“). 3d.5 („Tür verbinden“) zeigt fremde Enden auf der Weltkarte des Ziel-Headers.
- **Merge mit 3d.1 und 3d.3:** in `eventFields.ts` fügt 3d.1 Felder (`hidden_flag`, `var`, `value`) hinzu, 3d.2 nur `from` und `MOVEMENT_WORDS`; die Konflikte liegen an den Anhängepunkten (`dispatch.rs`, `access.rs`, `catalog.rs`, `tools.rs`, `api.ts`, `lib.rs`, `ROADMAP.md`, `README.md`) und werden beim Merge nach main gesammelt gelöst (Erfahrung Phase 4: 13+ Konflikte an Anhängepunkten an einen Opus-Subagenten, `.fig` von main behalten).
- **Lint-Muster:** findet `check-depth.sh` nach dem Merge eine Stelle aus 3d.1 oder 3d.3 (sie laufen ohne den Lint), wird sie beim Merge umgestellt; die Zielzahl 0 gilt auf main.
