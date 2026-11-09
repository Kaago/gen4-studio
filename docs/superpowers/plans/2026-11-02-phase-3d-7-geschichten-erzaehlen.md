# Phase 3d.7 „Geschichten erzählen“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eine Geschichte entsteht in der Tiefe Erzählen aus Rezepten: „Neue Geschichte“ nimmt eine Variable aus dem Pool, benennt sie `VAR_STORY_…` und legt die Geschichte mit dem Schritt „Anfang“ (Wert 0) an; „Gespräch schaltet weiter“ gibt einer Figur einen Script-Zweig `if VAR == k then … SetVar k+1`, „Figur erscheint/verschwindet ab Schritt“ hängt ein Sichtbarkeits-Flag aus dem Pool an eine Figur und schaltet es beim Betreten des Orts nach dem Stand der Variable, „Szene beim Betreten“ trägt eine Level-Script-Bedingung `var == k` mit einem Script ein, das einen Text zeigt und auf k+1 schaltet, „Belohnung einmalig“ gibt beim Ansprechen ab Schritt k einmal einen Gegenstand (Flag aus dem Pool). Jedes Rezept ist ein Undo-Schritt, schreibt den Schritt mit `made` in die Geschichte (Arten `dialog`, `appearance`, `reward`, dazu `start`), zeigt den Pool („noch 20 Variablen frei“) und läuft kopflos über MCP und `g4k` in den Szenarien `pt-geschichte`/`hg-geschichte`.

**Architecture:** Fünf neue Kernel-Rezepte im Rahmen von 3d.2/3d.5 (`RecipeId` + `plan`/`apply` je Modul `recipes/{new_story,talk_advances,figure_appears,scene_on_enter,reward_once}.rs`, gemeinsame Helfer in `recipes/story_common.rs`, Script-Texte aus Vorlagen `metadata/recipes/story_*.rotom` mit Platzhaltern wie `figure_pt.rotom`). Scripts werden **nie umgeschrieben**, sondern als neue Slots angehängt (`scripts::append_source`): ein Gesprächs- oder Belohnungszweig ist ein Hüll-Script, auf das das Objekt zeigt und das am Ende per `Jump` zum bisherigen Script der Figur springt; die Sichtbarkeit je Ort und Geschichte ist ein Block `sichtbarkeit_<S>_<H>` im Übergangs-Script (`on_transition`) des Orts, der beim nächsten Rezept zwischen zwei Markerzeilen erweitert wird (kein Parsen); die Szene ist ein `var_conditions`-Eintrag im Level-Script des Orts. Nachrichten sind **nummeriert** (Archiv des Headers, `Message n`/`NPCMsg n`) wie in 3d.5. Flags und Variablen kommen aus dem Pool über den Aufrufer (`free_flag`, wie 3d.5), das Rezept benennt sie (`name_flag`) und trägt den Schritt in die Geschichte (`update_story`); `squash_since` macht aus Transaktion und Dateischritten einen Undo-Schritt. App: die vier Schritt-Rezepte sind Modal-Dialoge (`stories/recipes/`), erreichbar aus dem Detail der Geschichte, aus „Zu einer Geschichte“ nach „Neue Figur“ und aus „Dialog anlegen…“ einer Figur ohne Script; „Neue Geschichte“ läuft über das Rezept; `story_list` nennt den Pool; der Graph speichert gezogene Knoten ins Beifile.

**Tech Stack:** Rust (g4-kernel `Store`/`Tx`, `scripts`, `stories`, `flags`, `recipes`, rotom 0.7.0 über uxie 0.9.5, serde), Tauri 2 (`commands/recipes.rs`, `dispatch.rs`, MCP `catalog.rs`/`access.rs`), React 19 mit `ui/primitives` (`Dialog`, `Picker`, `PickField` `choice="item"`, `FlagField`, `DSScreen`, `Segmented`, `Select`, `Badge`, `Tooltip`), `@xyflow/react` (Drag-Ende), vitest, `g4emu`-Szenarien über `studio mcp`.

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) (§3.1 Geschichte, §3.3 Tiefe Erzählen und Rezepte wörtlich, §4 Zeile 3d.7 und „Phasen-Gate 3d“ wörtlich, §7 „Pool“ und „Zwei Befehlsdialekte“, §8); [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Tabelle „Phasenschnitt“ Zeile 3d.7; „Geschichten und Erzählen“ Zeilen „Was ist eine Geschichte?“, „Modell“, „Knotenarten“, „Rezepte“; „Flags und Variablen“ Zeile „Frei“); [Gutachten A](../../research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md) §2.1 (Mechaniken: `hidden_flag`, `on_var_condition`, `on_transition`), §3.2 (c), §3.3 Rezepttabelle (Zeilen Neue Geschichte bis Belohnung einmalig), §5 Zeile G2; [Plan 3d.5](2026-10-31-phase-3d-5-figur-dialog-tuer.md) (Rezept-Muster `new_figure`, nummerierte Nachrichten, `NextStep` `story_join`, Szenario-Vorspann `pt-erzaehlen.yaml`); [Plan 3d.6](2026-10-31-phase-3d-6-geschichten-lesen.md) (Entität, `update_story`, `StoryDetail`, „Nicht in 3d.6“: Rezepte, Schritte in der Oberfläche, Pool-Anzeige, Knoten ziehen und speichern); [docs/formats/stories.md](../../formats/stories.md) (Branch 3d.6: Datei, Schritte und Arten, Verweise); [phase-3d-3-gate.md](../../phase-3d-3-gate.md) als Muster der Gate-Datei; [design-system/README.md](../../../design-system/README.md) (Nachtrag 3d.6 als Muster).

**Stand** 02.11.2026: `main` bei `96a278aa` (3d.1, 3d.2, 3d.3 Teil A und B, 4.5 gemergt). **Basis dieses Plans: main nach dem Merge von 3d.5 (Branch `phase-3d-5-figur-dialog-tuer`, Tasks 1–8 committet bis `707e727e`, Tasks 9–11 laufen) und 3d.6 Teil A (Branch `phase-3d-6-geschichten-lesen`, `600ff479`).** Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-7`, Branch `phase-3d-7-geschichten-erzaehlen` (legt der Hauptorchestrator nach beiden Merges an). Parallel laufen 3d.4 (Löschen), 3d.6 Teil B (nach 3d.4: `story_delete`, `mentions`, Menü „Aktionen“ im `StoryDetail`), 3d.8 (Erzählen-Layouts: Trainer-Assistent mit Vorschau, Begegnungen, Texte) und 3d.9 (Spielmodus live: `Hinge`, `FlagList`, `EventEditor` Zeile „Haltepunkt“). Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`, Emulator `scripts/g4emu`.

## Befunde vor dem Plan (02.11.2026, Branches 3d.5 `707e727e` und 3d.6 `600ff479`)

- **Rezept-Rahmen (3d.5):** kein Trait, ein geschlossenes `RecipeId` (`recipes/types.rs:17`, serde snake_case, sieben Werte, `title()`, `all()`) mit vier `match`-Armen in `recipes.rs` (`list()` mit `irreversible: matches!(…)`, `is_irreversible`, `plan`, `apply`); Modul-Konvention `XInput` (Deserialize), `XReport` (Serialize), `fn check(store, input) -> Checked` (alle Verweigerungen vor dem ersten Schreiben), `plan`, `apply`. `Draft { cells, tiles, events, messages, notes, irreversible }`, `Outcome<R> { report, created, change, changed, next }`, `next_steps(&[(id, label)])`: verfügbar, wenn `edit_dialog` oder eine `RecipeId` aus `list()`; sonst gesperrt mit `play_here` „Kommt mit 3d.9“, `adjust_terrain` „Kommt mit Phase 6“, alles andere (auch `story_join`) „Kommt mit 3d.7“ (`types.rs:200`). Feste Erwartungen auf „sieben Rezepte“: `types.rs` Unit-Test `next_steps_lock_what_is_not_there_yet`, `tests/cli.rs:2002` `cli_recipe_list`, `tests/recipes_erzaehlen.rs:159`, `catalog.rs:2154-2216`, `apps/studio/src/recipes/api.test.ts`, `workbench/recipes.test.ts`, `docs/mcp.md:249`.
- **Ein Undo-Schritt (Muster `new_figure::apply`, 3d.5):** `generation = store.change_generation(); mark = store.undo_depth()`; Schnappschüsse `scripts::snapshot_script`, `snapshot_compile_state`, `snapshot_archive`; `store.transaction(label, |tx| { tx.edit_text_archive(..)?; tx.edit_event(file, |f| …) })`; `scripts::append_source(&project, file, source_before, &addition) -> Vec<u16>` (schreibt, kompiliert, stellt bei Fehler den alten Text her; dann `scripts::restore_compile_state` und `store.rollback_to(mark)`); Dateischritte `script_file_step`, `archive_file_step`, `compile_state_step` je `store.record_file_step(label, step)`; `store.squash_since(mark, label)`; `remove_materialized_json` entfernt die Archiv-JSON, die der Compile im DSPRE-Layout schreibt; `changed: store.changes_since(generation).1`. `pub(crate)` nutzbar aus `recipes/`: `scripts::{rotom_source, declared_slots, append_source, write_and_compile, restore_compile_state, names_identifier}`, `FIRST_COMMON_SCRIPT` (2000). Der einzige Weg, ein **vorhandenes** Script zu ändern, ist `write_and_compile` (Muster `dialog::split_message`, `dialog.rs:168`).
- **Nachrichten:** nummeriert über `tx.edit_text_archive(archive, |doc| doc.messages.push(TextMessage { id: default_message_id(len), text, other }))`, Nummer = bisherige Zahl, Grenze `dialog::highest_message(family, cmd)` (255). Befehle je Familie (`metadata/recipes/figure_{pt,hgss}.rotom`): Pt `Message n` / `WaitButton` / `CloseMessage`, HGSS `NPCMsg n` / `WaitButton` / `CloseMsg`; `LockAll`, `FacePlayer`, `ReleaseAll`, `End` gleich.
- **Variablen und Flags in rotom:** Pt setzt mit `SetVarFromValue VAR, n` (so `start_pt.rotom`), HGSS mit `SetVar VAR, n`; `if VAR_X == 3 then … else … endif` und `if FLAG_X == 1 then` kompilieren (`rotoscript_spec.md` §4.1, Zeile 66; `tests/flags.rs:376`), Operatoren `==, !=, >, <, >=, <=`; `Jump Name` springt zu einem Label **oder Script** derselben Datei (§4.5, `GoTo` gleichbedeutend); `SetFlag`/`ClearFlag` gleich in beiden Familien; Gegenstand Pt `GiveItem item, amount, VAR_RESULT` (scrcmd 123; `AddItem` ist der DSPRE-Name) und HGSS `GiveItem item, amount, VAR_SPECIAL_RESULT` (scrcmd 125); Kommentare `// …`. Eigene Namen aus `data/ids/{flags,vars}.json` gelten im Script (3d.1, `pt-flag.yaml` `SetFlag FLAG_SZENARIO_FIGUR`).
- **Events:** `ObjectEvent { local_id, graphics_id, movement_type, trainer_type, hidden_flag (0 = immer da; die Figur fehlt, solange das Flag gesetzt ist), script, dir, data, movement_range_x/z, x, z, y }`; `CoordEvent { script, x, z, width, length, y, value, var }` (feuert, solange `var == value`); Script-Nummer: 0 keins, ≥ 3000 Trainer (`3000 + ID − 1`, Sicht-Script der Engine), ≥ 2000 gemeinsam, sonst Slot `#n` der Scriptdatei des Headers. `DialogSource::{Local { file, slot, line }, Common { script }, Trainer { id }, None, Unreadable }` aus `dialog::dialog_of(store, header, file, DialogKind, index)`.
- **Level-Scripts:** `LevelScriptDoc = rotom::LevelScript { header_entries: Vec<{ type, script_id? }>, var_conditions: Vec<{ var, value, script_id }> }`, Typen `on_load`, `on_resume`, `on_transition`, `on_var_condition` (Vanilla Platin: 46 / 36 / 244 / 105 Einträge); `scripts::read_source(project, file) -> ScriptSource::LevelScript(doc)`, `write_source`, `source_kind`; `empty_region.rs:86-90, 259-276` legt eine neue Level-Script-Datei an (`scripts::new_script_file_reported(project, SourceKind::LevelScript)`, bleibt nach Undo, `Created`) und setzt `level_script_id` des Headers in der Transaktion (`tx.edit_header`, Helfer `set_events`); `script_id` eines Eintrags ist ein Slot der Scriptdatei des Headers. Welche Header eine Level-Script-Datei nutzen: `store.header_table()` durchgehen (`phase-3d-1: commands/flags.rs:189-199` `Users::of`). Eine neue Fläche aus `create_area` mit `new_header: { template: 342 }` (Pt) bzw. `33` (HGSS) übernimmt das Level-Script der Vorlage, das beim Betreten Script 1 der Scriptdatei startet (`pt-flag.yaml`, `hg-flag.yaml`); die neue Scriptdatei der Szenarien hat `script Main #1: End`.
- **Pool und Namen (3d.1):** `flags::free_var(project, &usage) -> Option<u16>`, `flags::pool(project, &usage, kind) -> Pool { range, free, total, reserved }`, `in_save(family, kind, n)`, `area_of(family, kind, n)`, `name_flag(store, &NameRequest { kind, number, id: Option<String>, description }) -> NameReport { id, created, files, change, … }` (ein Dateischritt, nur im Textprojekt; eine vergebene ID → `taken_text`); der `RefIndex` lebt in der Tauri-Sitzung (`session.refs`), **ein Rezept hat nur `&mut Store`**: Nummern kommen über die Eingabe (3d.5 `hidden_flag`; App und MCP rufen vorher `free_flag { kind }` → `FreeFlagDto { number, name, pool_free, scripts_indexed }`).
- **Geschichten (3d.6):** `stories::{Story, Step { key, value, kind: StepKind, name, notes, made: Option<Made { recipe, studio }>, uses: Vec<Use> }, Use::{Trainer, Map, Flag, Var, Object { file, local_id }, Script { file, slot }, Message { archive, index }, LevelScript { file, condition }}, Progress::{Var(id), Flag(id)}}`; `StepKind::{Start, Dialog, Condition, Battle, Reward, Progress, Appearance}` (`word()` deutsch); `create_story(store, &StoryRequest { name, progress, notes, steps, flags })`, `update_story(store, id, &StoryPatch { name, notes, progress, steps, flags, layout })` ersetzt jedes gegebene Feld ganz (**Schritte lesen, ergänzen, ganz zurückschreiben**), je ein `FileStep` (`HEADER_TABLE`, `Reset`), Label `Geschichte „X“ angelegt` / `… geändert`; `read_story(project, n)`, `number_of(project, id)`, `list_stories`; Schlüssel eindeutig, doppelter → `duplicate_key_text`; Verweigerungen `NOT_A_TEXT_PROJECT`, `name_empty_text`, `unknown_id_text`. Der Graph legt den **ersten** Schritt je Wert auf den Wert-Knoten (`GraphNode::Value.step`); das Detail zeigt alle Schritte in der Tabelle, `made` und `notes` nicht. `story_list` → `StoryListDto { stories: Vec<StoryRowDto { number, id, name, progress, steps, problem }>, text_project, ms }`; `story_get` → `StoryDto { number, story, layout, progress_number, ms }`. Frontend: `GeschichtenArea` (Liste „Fäden“, „Geschichten“, „Flags und Variablen“ mit Untertitel), `StoriesSurface` (Tabelle, `[data-new-story]`, `NewStoryDialog` mit `Segmented` Variable | Flag, `FlagField`, ruft `storyCreate`), `StoryDetail` (Kopf, Notizen, `Section` „Schritte“ mit `NO_STEPS = "Noch keine Schritte. Rezepte legen sie an (kommt mit 3d.7)."`, `Section` „Graph“ mit `ThreadGraph`, `NO_VALUES`), `StoryNode` (neun Arten), `useStory`/`useStories` mit `{ project, revision }`; `api.stories.ts` (`storyCreate` ohne `steps`/`flags`, `storyUpdate(id, patch & { layout? })`). Anlegen und Ändern dauern unter Last 2,6 bis 3,6 s (`apply_file_step` schreibt die ID-Dateien neu): die Oberfläche braucht einen Busy-Zustand.
- **Oberfläche der Rezepte (3d.5):** `workbench/recipes.ts` (`RECIPE_BAR` fünf Einträge, `NEXT`, `KNOWN`, `ALWAYS = ["edit_dialog"]`, `lockReason`, `recipeRequest`), `WizardRequest` (`workbench/WorkbenchContext.tsx:29`, Arten `area`, `room`, `clearWorld`, `publish`, `new_figure`, `connect_doors`, `new_trainer_here`), `app/WizardHost.tsx` (ein `case` je Art; `step(id, made)` kettet den nächsten Assistenten, `edit_dialog` öffnet den Dialog-Editor über `onOpenDialog`), Assistenten in `workbench/wizards/` mit `WizardFrame`, `WizardResult` (`DoneResult { next, onStep }`), `useDraftPlan(recipe, input, onDraft)` (300 ms), `wizardSpecs.ts`; sie rendern in `[data-wizard-bar]` des Scharniers und `[data-wizard-hand]` des Decks — **der Bereich Geschichten hat kein Deck** (3d.6). `DialogSection.tsx`: bei `source.kind === "none"` die gesperrte Taste „Dialog anlegen…“ mit `ANLEGEN_LATER = "Kommt mit 3d.7"` (der Haken für dieses Rezept); `openWizard` im Kontext (`panels.tsx:506`). `EventEditor.tsx` rendert `FlagField` für `hidden_flag` (Objekt) und `var` (Auslöser). Gegenstände wählt `PickField choice="item"` (`data/trainer/TrainerHand.tsx:221`), Orte `HeaderPicker` (`workbench/inspector/HeaderPicker.tsx:40`), Figuren-Aussehen `FigureLookField`.
- **Szenarien:** Schritte `continue`, `expect-map`, `walk-to`, `walk`, `walk-until-blocked R N`, `expect-at X Y`, `press a`, `mash a N`, `shot`, `expect-battle ART LEVEL`, `win-battle`, `expect-flag N 0|1`, `expect-var N W`, `set-var`; kein `talk` (das ist `press a` + `mash a 240`). `mcp: [--allow-irreversible, --expert]`, `edits` als Werkzeugaufrufe mit Bindung `[tool, {args}, name]` und `${name.feld}`, `{expect: [{path, equals}]}`; `pt-flag.yaml`/`hg-flag.yaml` (Vorspann: `assign_base`, `apply_header_patch`, `adopt_text_project`, `create_area` aus Vorlage 342 bzw. 33, neue Scriptdatei per `g4_call new_script_file`, `free_flag`, `name_flag`, `add_event`, `save`, `save_script`; Figur bei (54, 17) Pt bzw. (15, 13) HG, Ankunft (44, 21) bzw. (24, 16), Treppe Eventdatei 394 Warp 0 bzw. 61 Warp 0, `press left:30` bzw. `right:30` in die Tür, nach `expect-map` `wait 150`), `pt-trainer-begegnung.yaml` (Spieler ohne Pokémon: ein Geschenk-Objekt bei (45, 20) mit `GivePokemon` Level 100 einmal per Flag; Trainer-Sichtkampf `walk right 1`, `mash a 240`, `expect-battle`), `pt-raum-assistent.yaml` (`create_room` mit `entrance`, innen (8, 4), `walk right 1` hinaus). 3d.5 Task 9 schreibt `pt-erzaehlen.yaml`/`hg-erzaehlen.yaml` (Vorspann mit neuer Scriptdatei **und neuem Textarchiv** am Header, `recipe_apply new_figure`/`connect_doors`/`new_trainer_here`); dieser Plan übernimmt ihren Vorspann.
- **Spielstände:** `$G4_ROMS_DIR/_testkopien/_saves/{CPUD,IPKD,IPKE}-zimmer.sav` (Zimmer, Header 415 bzw. 64, ohne Pokémon).

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Rezept-IDs | `RecipeId::{NewStory, TalkAdvances, FigureAppears, SceneOnEnter, RewardOnce}` (serde `new_story`, `talk_advances`, `figure_appears`, `scene_on_enter`, `reward_once`), Titel „Neue Geschichte“, „Gespräch schaltet weiter“, „Figur erscheint/verschwindet ab Schritt“, „Szene beim Betreten“, „Belohnung einmalig“; `all()` hängt sie in dieser Reihenfolge an (zwölf Rezepte); keines ist `irreversible`, außer `figure_appears`/`scene_on_enter`, wenn sie eine Level-Script-Datei anlegen müssen (`is_irreversible` prüft wie `plan`: „Level-Script-Datei anlegen“) | Rahmen 3d.2/3d.5; eine neue Scriptdatei ist ein Member und bleibt nach Undo (`empty_region`) |
| Woher Nummern kommen | wie 3d.5: `NewStoryInput.var`, `FigureAppearsInput.flag`, `RewardOnceInput.flag` sind **Pflicht**; die App holt sie mit `freeFlag("var"|"flag")` beim Öffnen des Dialogs (Feld bleibt änderbar, Fuß „Freie Variable nehmen“), MCP-Szenarien mit `free_flag`; `g4k recipe` nimmt sie aus `--input`. Das Rezept prüft `in_save`, `area_of == General`, keinen eigenen Namen (sonst `taken_text`), nicht die Reservierung `done_var`; ob die Nummer unbenutzt ist, prüft der Aufrufer (Index) | ein Rezept hat keinen `RefIndex` (Befund); 3d.5 „Sichtbarkeits-Flag“ |
| Namen der Variablen und Flags | `VAR_STORY_<NAME>` (`NAME` = `symbols::normalize(name, true)`, gekürzt auf 64 mit Präfix, Kollision `_2`, `_3` … wie `stories::entities::new_id`), Flags `FLAG_STORY_<NAME>_SICHTBAR_<local_id>` und `FLAG_STORY_<NAME>_BELOHNUNG_<local_id>`; Beschreibung „Fortschritt der Geschichte „X““ bzw. „Sichtbarkeit von Figur k in „X““ / „Belohnung von Figur k in „X““ über `name_flag` (ein Dateischritt, im selben `squash_since`); Flags landen in `story.flags` | Spezifikation „eigener Name“; die Fläche „Flags und Variablen“ zeigt den Zweck |
| Schritte | `new_story` legt `Step { key: "s0", value: 0, kind: Start, name: "Anfang" }` an; `talk_advances` `key "s{k+1}"`, `value k+1`, `kind Dialog`, Name „Gespräch mit Figur {local_id}“; `scene_on_enter` `key "s{k+1}"`, `value k+1`, `kind Dialog`, Name „Szene beim Betreten“; `figure_appears` `key "a{flag}"`, `value k`, `kind Appearance`, Name „Figur {local_id} erscheint ab Schritt {k}“ / „… verschwindet ab Schritt {k}“; `reward_once` `key "r{flag}"`, `value k`, `kind Reward`, Name „Belohnung von Figur {local_id}“; jede Eingabe hat `step_name: Option<String>` (die App setzt Ortsnamen und Gegenstand ein: „Szene beim Betreten von Zweiblattdorf“, „Belohnung: Trank“); `made = Made { recipe: <serde-Name>, studio: env!("CARGO_PKG_VERSION") }`; `uses` je Rezept in den Tasks; ein `value`, das schon ein Schritt mit Art `Dialog` trägt, wird verweigert („Schritt {k+1} gibt es schon: „{name}“.“), `Appearance`/`Reward` dürfen neben andere Schritte mit demselben Wert | Modell (c): Schritte sind Werte, Erscheinen und Belohnung sind Inhalte eines Werts; der Graph zeigt den ersten Schritt je Wert (3d.6), die Tabelle alle; `start` und `progress`: „Anfang“ ist `start`, Weiterschalten steckt in `dialog` (Beispiel in `stories.md`) |
| Scripts anhängen, nie umschreiben | Gespräch und Belohnung sind **Hüll-Scripts** (neuer Slot `#n` = höchster deklarierter + 1, Verweigerung ab 2000): `LockAll`, `FacePlayer`, der Zweig, `ReleaseAll`, dann `Jump <bisheriges Script>` (sein Name aus `declared_slots`) oder `End` (Script 0); das Objekt zeigt danach auf `#n` (`tx.edit_event`); der zuletzt angelegte Zweig prüft zuerst. Voraussetzung: `dialog_of` liefert `Local` oder `None`; `Common` (≥ 2000) → „Figur {k} spricht über das gemeinsame Script {s}; das Rezept braucht ein eigenes Script.“, `Trainer` → „Trainer sprechen über ihre Kampftexte (Bereich Pokémon).“; ein Script, das andere Figuren teilen, bleibt für sie unverändert (nur **dieses** Objekt zeigt auf die Hülle) | Umschreiben bräuchte einen Parser; `dialog_of` folgt `Jump` (3d.5), der Dialog-Abschnitt zeigt alle Nachrichten der Kette |
| Sichtbarkeit | `figure_appears` setzt `hidden_flag = flag` am Objekt (Verweigerung, wenn schon ≠ 0: „Figur {k} hat schon das Sichtbarkeits-Flag {FLAG}.“) und hält je Ort und Geschichte **einen** Block im Übergangs-Script: `script sichtbarkeit_{S}_{H} #{n}:` (S = Nummer der Geschichte, H = Header), je Regel vier Zeilen `if VAR >= k then` / `ClearFlag F` / `else` / `SetFlag F` / `endif` (erscheint) bzw. getauscht (verschwindet), zum Schluss `Jump <bisheriges on_transition-Script>` oder `End` und die Markerzeile `// ende sichtbarkeit_{S}_{H}`; gibt es den Block schon, fügt das Rezept die neue Regel vor der `Jump`/`End`-Zeile ein (`write_and_compile`, Muster `split_message`), sonst hängt es ihn an (`append_source`) und setzt den `on_transition`-Eintrag des Level-Scripts auf `#n` (bisheriges Ziel wird das `Jump`-Ziel; ein bisheriges Ziel ≥ 2000 oder ohne Slot → „Das Übergangs-Script des Orts ist kein eigener Slot.“); kein Eintrag → neuer Eintrag `{ type: on_transition, script_id: n }`. Ob `on_transition` **vor** dem Aufstellen der Objekte läuft, misst Task 5 gated; läuft es danach, nimmt der Block `on_load` (eine Konstante `VISIBILITY_ENTRY`) | Gutachten A §3.3 „`SetFlag`/`ClearFlag` im Script des Schritts, `on_transition`-Eintrag“; ein Block je Ort braucht kein Parsen (Marker) |
| Level-Script-Datei des Orts | nutzt sie ein zweiter Header (`header_table()`), legt das Rezept eine Kopie als neue Datei an (`new_script_file_reported(LevelScript)`, `write_source`) und setzt `level_script_id` des Headers in der Transaktion; `level_script_id == 0` oder `SourceKind::Binary` ebenso neu (leeres Dokument); `plan` nennt das in `irreversible` („Level-Script-Datei {n} wird angelegt und bleibt“), `Outcome.created` nennt sie | Muster `empty_region`; Entscheidung „Grenze von Undo“ aus 3d.2 |
| Szene | `scene_on_enter`: Slot `script szene_{n} #{n}:` mit `LockAll`, `Message m` (`NPCMsg`), `WaitButton`, `CloseMessage` (`CloseMsg`), `SetVarFromValue VAR, k+1` (`SetVar`), `ReleaseAll`, `End` in der Scriptdatei des Headers; `var_conditions.push({ var, value: k, script_id: n })` und, falls keiner da, `header_entries.push({ type: on_var_condition })`; die Szene läuft genau einmal, weil sie weiterschaltet; `uses`: `levelScript { file, condition }`, `script`, `message`, `map` (ID des Headers, wenn das Register einen Namen hat, sonst keine) | Gutachten A §2.1 `on_var_condition`; ohne Weiterschalten liefe sie bei jedem Betreten |
| Belohnung | `reward_once`: `if VAR >= k then` / `if FLAG == 0 then` / Nachricht / `GiveItem item, 1, VAR_RESULT` (HGSS `VAR_SPECIAL_RESULT`) / `SetFlag FLAG` / `ReleaseAll` / `End` / `endif` / `endif`; ein voller Beutel wird nicht geprüft (Ruling: `GiveItem` scheitert still, das Flag ist dann gesetzt; Hinweis in `stories.md`); nur Gegenstände, kein Pokémon | Spezifikation 3d.7 „Belohnung einmalig“ mit `expect-flag`; „Pokémon als Belohnung“ unter „Nicht in 3d.7“ |
| `story_join` | `next_steps` macht `story_join` verfügbar (wie `edit_dialog`, kein Rezept); die App öffnet damit „Gespräch schaltet weiter“ mit der Figur vorbelegt; der Standard-Grund für Unbekanntes wird „Kommt später“ (Kernel und `lockReason`) | 3d.5 Entscheidung `NextStep` |
| Oberfläche der Schritt-Rezepte | **Modal-Dialoge** (`Dialog`-Primitive, wie `DialogEditor`), nicht Scharnier-Assistenten: `stories/recipes/StoryRecipeDialog.tsx` mit `[data-story-recipe-dialog=<id>]`, Formulare `TalkAdvancesForm`, `FigureAppearsForm`, `SceneOnEnterForm`, `RewardOnceForm`, gemeinsam `FigureField` (Ort über `HeaderPicker`, Figur aus den Objekt-Events der Eventdatei des Headers als `Picker` „Figur {local_id} · Aussehen {graphics_id}“), `StepField` (`Select` der Schritte der Geschichte, Text „{value}: {name}“), Text mit `DSScreen`-Vorschau, `FlagField` (Variable/Flag) mit Vorbelegung aus `freeFlag`; rechts die Sätze aus `recipe_plan` (`Draft.notes`, `irreversible`) über `useDraftPlan`; primäre Taste „Anlegen“, Busy „Wird angelegt …“, `Notice` bei Verweigerung; Ergebnis: Toast `Schritt „{name}“ angelegt` und `onRecipeApplied(changed)`. Einstiege: Leiste „Schritt anlegen“ im `StoryDetail` (`stories/recipes/StepRecipes.tsx`, vier `Button`s `[data-story-recipe=<id>]`, gesperrt mit Tooltip „Rezepte brauchen eine Variable als Fortschritt“ bei Flag-Fortschritt), `story_join` aus dem Ergebnis von „Neue Figur“ (`WizardHost.step`), „Dialog anlegen…“ im `DialogSection` (Figur ohne Script; der Dialog hat dann ein Feld „Geschichte“ mit `Select` und dem Link „Neue Geschichte…“) | der Bereich Geschichten hat kein Deck und keinen Entwurf in der Schau (3d.6); ein Modal geht aus jedem Bereich |
| „Neue Geschichte“ | `NewStoryDialog`: Variante Variable ruft `recipeApply("new_story", { name, var, notes })` (Feld vorbelegt mit `freeFlag("var").number`, Zeile „Noch {pool_free} Variablen frei“, leer: „Keine freie Variable im allgemeinen Bereich“ und Taste gesperrt), Variante Flag bleibt `storyCreate` (3d.6) | Spezifikation „eine Variable aus dem Pool, eigener Name“; Flag-Geschichten bleiben möglich |
| Pool-Anzeige | `StoryListDto.free_vars: Option<u32>` (aus `flags::pool(project, &index.usage(Var), Var).free.len()` der Sitzung; `None` ohne Index oder außerhalb des Textprojekts); Untertitel der Zeile „Geschichten“ in `GeschichtenArea`: „{n} Geschichten · {free} Variablen frei“ | Entscheidungen 3d.7 „Pool-Anzeige („noch 20 Variablen frei“)“ |
| Graph speichern | `ThreadGraph` bekommt `onLayout?: (layout: Layout) => void` (bei `onNodeDragStop`, nur mit Geschichte); `StoryDetail` schreibt es verzögert (500 ms) über `onUpdate({ layout })` (ein Undo-Schritt je Ziehen, Label „Geschichte „X“ geändert“); `StoryDetail` zeigt je Schritt `made.recipe` als Tooltip der Art-Plakette und die Schritt-Notizen | 3d.6 „erst 3d.7 schreibt es aus der Oberfläche“ |
| Inspector in Erzählen | unter dem `FlagField` von `hidden_flag` (Objekt) und `var` (Auslöser) eine Zeile `text/xs` „Geschichte „X“: {Schrittname}“, wenn eine Geschichte das Flag in `flags` bzw. die Variable als `progress` führt (`StoryRowDto.flags: Vec<String>` neu; Auflösung über die ID des Flag-Eintrags) | Spezifikation §3.3 „Geschichte („Sichtbar ab Schritt …“)“ |
| Szenarien | `pt-geschichte.yaml` (CPUD), `hg-geschichte.yaml` (IPKD), `mcp: [--allow-irreversible, --expert]`; Vorspann aus `pt-erzaehlen.yaml` (Gebiet aus der Vorlage mit neuer Eventdatei, Scriptdatei und Textarchiv), Geschenk-Figur wie `pt-trainer-begegnung.yaml` per `save_script` (Script 2, `GivePokemon`), dann: `free_flag var` → `recipe_apply new_story`; `recipe_apply new_figure` (Figur A) → `recipe_apply talk_advances` (ab 0); `recipe_apply new_trainer_here` (Trainer B) → `free_flag flag` → `recipe_apply figure_appears` (ab 1, erscheint); `recipe_apply create_room` (Vorlage 415 bzw. HG-Zimmer, Tür im Gebiet) → `recipe_apply scene_on_enter` (ab 1); `free_flag flag` → `recipe_apply reward_once` (ab 2, Trank 17); `expect` auf `pool_free` vor und nach; `save`, Build; Spiel: Treppe → Gebiet, `expect-var V 0`, Weg durch Bs Kachel frei (`walk-until-blocked`, `expect-at` oben), A ansprechen → `shot`, `expect-var V 1`, Tür des Raums → Szene (`shot szene`, `mash a 120`), `expect-var V 2`, hinaus, Bs Kachel jetzt blockiert bzw. Sichtkampf → `expect-battle`, `win-battle`, A ansprechen → Belohnung `shot`, `expect-flag R 1`, noch einmal → `expect-flag R 1` und kein zweiter Text (`shot`) | Spezifikation §4 Zeile 3d.7 wörtlich; das Phasen-Gate 3d nutzt dieselben Szenarien |
| Gated Kernel-Tests | `crates/g4-kernel/tests/recipes_story.rs` (neu): synthetisch über `TextFixture::exported()` (Muster `recipes_erzaehlen.rs` `fixture_with(source)`) und gated auf `vanilla_copy()` je Spiel mit `eprintln!`-Zahlen; Kopien `3d-7-{pt,hgde,hgus}` | eigene Datei: keine Konflikte mit 3d.5 an `recipes_erzaehlen.rs` |
| Nicht in 3d.7 | ein Kampf-Rezept (`StartTrainerBattle`/`TrainerBattle`; der Kampf kommt vom Trainer aus „Neuer Trainer hier“), Pokémon als Belohnung, Auslöser-Fläche statt Level-Script, Bewegungen und Zwischensequenzen, Graph mit Code erzeugenden Knoten, Rezepte auf Flag-Geschichten, Erzählen-Layouts (3d.8), `story_delete` und `mentions` (3d.6 Teil B), Live-Werte (3d.9) | Entscheidungen „Nicht in 3d“ und Schnitt |

## Global Constraints

- Arbeitsort ist der Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-7`, Branch `phase-3d-7-geschichten-erzaehlen` (Basis main nach dem Merge von 3d.5 und 3d.6 Teil A); nie `cd` in die Hauptkopie, nie Branch wechseln, nie nach main mergen; kein `git stash`. `tools/` ist ein Symlink (nie einchecken).
- cargo braucht `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"`; `pnpm install` einmal in `apps/studio`.
- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, genau einmal).
- Originale unter `$HOME/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur `Roms/_testkopien/3d-7-<zweck>` (`cp -cR` für `3d-7-<zweck>_DSPRE_contents`, `cp -c` für `3d-7-<zweck>.nds`, nebeneinander); `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut; gated Tests lesen `G4_TEST_PROJECT` und schreiben nur auf `vanilla_copy()`; keine ROM-Daten ins Repo; Fixtures synthetisch (`TextFixture::exported()`).
- Test-Umgebung nur auf `G4_VITE_PORT=1770 G4_BRIDGE_PORT=1771` (`bash scripts/agent/studio-web.sh`); andere Ports nie anfassen; eigenes Chrome-Profil unter `target/agent/`.
- Deutsche Oberfläche, englische Bezeichner; UI nur aus `apps/studio/src/ui/primitives` und Tokens `--g4-*` (`bash scripts/check-styles.sh`, `check-tokens.sh`, `check-depth.sh`); höchstens eine primäre Taste je Dialog.
- Scripts entstehen aus Vorlagen `crates/g4-kernel/metadata/recipes/story_*.rotom` mit Platzhaltern in geschweiften Klammern (`{n}`, `{var}` …), je Familie dort, wo sich die Befehle unterscheiden (`_pt`, `_hgss`); Dp nimmt Pt (wie `figure_script`).
- Anhängepunkte nur anhängen: `recipes/types.rs` (`RecipeId`, `title`, `all`, `next_steps`), `recipes.rs` (vier `match`-Arme, `pub mod`), `apps/studio/src-tauri/src/commands/dispatch.rs`, `mcp/catalog.rs` (Beschreibung von `recipe_args`), `docs/mcp.md`, `apps/studio/src/recipes/api.ts` (`RecipeId`, Typen), `workbench/recipes.ts` (`KNOWN`, `NEXT`, `ALWAYS`, `lockReason`), `workbench/WorkbenchContext.tsx` (`openStoryRecipe`), `app/WizardHost.tsx` (`story_join`), `App.tsx` (Zustand `storyRecipe` und ein Mount), `stories/StoryDetail.tsx` (ein Mount `StepRecipes` und die Props `onLayout`, `onRecipe`), `stories/GeschichtenArea.tsx` (Untertitel), `workbench/inspector/DialogSection.tsx` (Taste „Dialog anlegen…“), `EventEditor.tsx` (die Hinweiszeile unter dem `FlagField`), `docs/formats/{stories,recipes}.md`, `docs/agent-gate.md`, `docs/ROADMAP.md`, `design-system/README.md`.
- **Konfliktstellen mit 3d.4, 3d.6 Teil B, 3d.8, 3d.9 (parallel):** `recipes/types.rs`/`recipes.rs` (nur anhängen), `dispatch.rs`, `mcp/*.rs`, `api.ts`, `App.tsx` (3d.8 Layouts, 3d.9 `livePlay`; hier nur Zustand und Mount), `WizardHost.tsx` (3d.8 Trainer-Assistent; hier nur der `story_join`-Zweig), `EventEditor.tsx` (3d.9 Zeile „Haltepunkt“; hier nur die Hinweiszeile), `StoryDetail.tsx` (3d.6 Teil B Menü „Aktionen“; hier nur `StepRecipes` und `onLayout`), `commands/stories.rs` (3d.6 Teil B `story_delete`; hier `free_vars` und `flags` in den DTOs), `docs/mcp.md`, `ROADMAP.md`, `design-system/README.md`, `.fig`. **Nicht anfassen:** `store.rs`, `project.rs`, `data/refs.rs`, `flags/*.rs` (nur aufrufen), `stories/{schema,threads}.rs` (nur aufrufen; `entities.rs` nur, wenn Task 1 `update_story` um einen Schritt-Anhang erweitert, siehe dort), `delete*`, `textproject/{export,import,schema/*}.rs`, `recipes/{new_figure,connect_doors,new_trainer_here,room,area,extend,empty_region}.rs`, `workbench/wizards/*` außer `wizardSpecs.ts` (nur `story_join`), `ui/frame/Hinge*`, `flags/FlagList*`.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build` und die drei Check-Skripte.
- Gate-Zahlen (Spezifikation §4, Zeile 3d.7, wörtlich): Szenarien `pt-geschichte`/`hg-geschichte` über MCP: Geschichte mit drei Schritten anlegen (eine Variable aus dem Pool, eigener Name); Figur A ansprechen → `expect-var` Schritt 0 → 1, `shot`; Figur B erscheint ab Schritt 1 (vorher `walk-until-blocked` durch ihre Kachel frei, danach blockiert, `expect-at`); Raum betreten startet die Szene (`shot`); Kampf `expect-battle`; Belohnung beim zweiten Ansprechen nicht noch einmal (`expect-flag`); `vanilla_checks` 0 Fehler; Pool um eine Variable und die Zahl der Flags kleiner; jedes Rezept ein Undo-Schritt, ⌘Z bis zum Ausgangsstand lässt `git status` leer.

## Review Focus

- Eine Figur, deren Script eine zweite Figur teilt (`shared_with > 0`): „Gespräch schaltet weiter“ lässt den alten Slot für die zweite Figur unverändert und zeigt nur dieses Objekt auf die Hülle; der Dialog der zweiten Figur ist danach gleich (Task 3 `a_shared_script_stays_for_the_other_figure`).
- Ein Wert, den die Geschichte schon als Dialog-Schritt hat (`talk_advances` ab Schritt k, wenn `s{k+1}` existiert): verweigert mit dem Satz, nichts geschrieben, `undo_depth` unverändert (Task 3 `a_taken_value_is_refused_before_writing`).
- Zwei Sichtbarkeits-Regeln am selben Ort derselben Geschichte: der Block wächst um vier Zeilen vor dem `Jump`, der `on_transition`-Eintrag bleibt, der Compile läuft, Undo entfernt nur die zweite Regel (Task 5 `a_second_rule_grows_the_block_and_undo_removes_it`).
- Eine Level-Script-Datei, die zwei Header teilen: das Rezept kopiert sie, der zweite Header behält die alte, `Outcome.created` nennt die neue, `plan.irreversible` kündigt es an (Task 6 `a_shared_level_script_is_copied_for_this_header`).
- Speichern zwischen Anwenden und Undo (DSPRE-Layout, Archiv-JSON des Compiles): Undo stellt Script, Archiv, Eventdatei und Geschichte her, `git status`/Dateivergleich wie vor dem Rezept (Task 4 `a_saved_reward_is_undone_byte_equal`, Muster `a_saved_figure_in_a_dspre_project_is_undone_and_redone`).

---

## Teil A: Kernel

### Task 1: Rezept-IDs, Helfer, „Neue Geschichte“

**Files:**
- Modify: `crates/g4-kernel/src/recipes/types.rs` (`RecipeId` + fünf Werte, `title`, `all`; `next_steps`: `story_join` verfügbar, Standard-Grund „Kommt später“), `crates/g4-kernel/src/recipes.rs` (`pub mod story_common; pub mod new_story; …` und die vier `match`-Arme für alle fünf; die vier Schritt-Rezepte erst als `todo!()`-freie Stubs, die `Refused("Kommt mit Task 3")` liefern, bis ihr Task sie füllt), `crates/g4-kernel/src/stories/entities.rs` (nur: `pub fn append_step(store, id, step: Step, flags: &[String]) -> Result<StoryReport, KernelError>`: liest die Geschichte, verweigert doppelten Schlüssel, hängt den Schritt und neue Flags an, ruft `update_story`)
- Create: `crates/g4-kernel/src/recipes/story_common.rs`, `crates/g4-kernel/src/recipes/new_story.rs`, `crates/g4-kernel/src/recipes/new_story/tests.rs`
- Test: `crates/g4-kernel/tests/recipes_story.rs` (neu, Fixture-Helfer wie `recipes_erzaehlen.rs`: `fixture_with(source)`, `state(&store)`, `refused(result)`)

**Interfaces:**
- Consumes: `stories::{create_story, read_story, number_of, Step, StepKind, Made, Progress, Use}`, `flags::{name_flag, NameRequest, in_save, area_of, Area::General, entities::own_names}`, `textproject::symbols::normalize`, `recipes::types::*`.
- Produces (`story_common.rs`, `pub(crate)`):
  - `pub(crate) fn studio_version() -> &'static str` (`env!("CARGO_PKG_VERSION")`), `pub(crate) fn made(recipe: RecipeId) -> Made`.
  - `pub(crate) struct LoadedStory { pub number: u16, pub story: Story, pub var: u16, pub var_id: String }`; `pub(crate) fn load_story(store: &mut Store, id: &str) -> Result<LoadedStory, KernelError>`: verweigert unbekannte ID (`unknown_id_text`), Flag-Fortschritt („Geschichte „X“ hält ihren Fortschritt in einem Flag; die Rezepte brauchen eine Variable.“), eine `progress.var`, die nichts nennt.
  - `pub(crate) fn step_of(story: &Story, value: u16) -> Option<&Step>`, `pub(crate) fn dialog_step_taken(story: &Story, value: u16) -> Result<(), KernelError>` (Satz „Schritt {value} gibt es schon: „{name}“.“).
  - `pub struct FigureRef { pub header: u16, pub local_id: u16 }` (Deserialize, Serialize; `pub`, weil die Eingaben der Rezepte es tragen), `pub(crate) struct Figure { pub event_file: u16, pub index: u16, pub local_id: u16, pub script: u16, pub trainer: bool, pub hidden_flag: u16 }`; `pub(crate) fn figure(store, &FigureRef) -> Result<Figure, KernelError>` („Auf Header {h} gibt es keine Figur {local_id}.“).
  - `pub(crate) struct HeaderFiles { pub script_file: u16, pub text_archive: u16, pub level_script: u16, pub event_file: u16 }`; `pub(crate) fn header_files(store, header: u16) -> Result<HeaderFiles, KernelError>` über `header_table()`.
  - `new_figure::remove_materialized_json` wird `pub(crate)` (die einzige Änderung an `new_figure.rs`), damit die Hüllen-Rezepte es nach dem Compile rufen.
  - `pub(crate) fn story_flag_id(story_name: &str, kind: &str, local_id: u16, taken: &dyn Fn(&str) -> bool) -> String` (`FLAG_STORY_<NAME>_<KIND>_<local_id>`, `_2` …), `pub(crate) fn story_var_id(name: &str, taken) -> String`.
  - `pub(crate) fn check_pool_number(project, kind: FlagKind, n: u16) -> Result<(), KernelError>` (`in_save`, `area_of == General`, kein eigener Name; Sätze „{KIND} 0x… liegt nicht im Spielstand.“ / „… nicht im allgemeinen Bereich.“ / `taken_text`).
  - `pub(crate) fn next_slot(project, file) -> Result<(u16, String /*Quelle*/), KernelError>` (`declared_slots`, Verweigerung ab `FIRST_COMMON_SCRIPT`), `pub(crate) fn slot_name(source: &str, slot: u16) -> Option<String>`.
  - `pub(crate) fn push_message(tx, archive, text) -> Result<u16, KernelError>` (Index = bisherige Zahl; Grenze `dialog::highest_message`).
  - `pub(crate) fn template(name: &str, family: Family) -> &'static str` (`include_str!` der `metadata/recipes/story_*.rotom`), `pub(crate) fn fill(template: &str, pairs: &[(&str, String)]) -> String`.
- `new_story.rs`: `pub struct NewStoryInput { pub name: String, pub var: u16, #[serde(default)] pub notes: String }`, `pub struct NewStoryReport { pub story: String, pub number: u16, pub var: u16, pub var_id: String, pub files: Vec<PathBuf> }`, `pub fn plan(store, &NewStoryInput) -> Result<Draft, KernelError>` (nur `notes`: „Legt Geschichte „X“ mit dem Schritt „Anfang“ an.“, „Benennt Variable 0x40A5 als VAR_STORY_X.“), `pub fn apply(store, &NewStoryInput) -> Result<Outcome<NewStoryReport>, KernelError>`: `mark`; `name_flag` (Var, `story_var_id`, Beschreibung „Fortschritt der Geschichte „X““); `create_story` mit `Step { key: "s0", value: 0, kind: Start, name: "Anfang", made, uses: [] }`; `squash_since(mark, "Geschichte „X“ angelegt")`; `next: next_steps(&[("talk_advances", "Gespräch schaltet weiter"), ("figure_appears", "Figur erscheint ab Schritt")])`.

- [ ] **Step 1: Failing tests** in `tests/recipes_story.rs`:

```rust
#[test] fn list_names_twelve_recipes_with_the_story_recipes_last() { /* list().len() == 12, letzte fünf ids und titles wie in der Entscheidung, keines irreversible */ }
#[test] fn next_steps_after_3d7_free_story_join_and_say_later() { /* story_join available, play_here "Kommt mit 3d.9", "nonsense" "Kommt später" */ }
#[test] fn plan_of_a_new_story_writes_nothing_and_names_var_and_step() { /* Draft.notes 2 Sätze, state unverändert */ }
#[test] fn apply_of_a_new_story_names_the_var_and_is_one_undo_step() { /* data/ids/vars.json hat VAR_STORY_TESTFADEN → 0x4030, data/vars/NNNN.json, data/stories/0000.json mit steps[0] = {s0, 0, start, Anfang, made}; undo_depth == mark + 1; undo stellt state her, redo wieder */ }
#[test] fn a_var_outside_the_pool_or_with_a_name_is_refused() { /* 0x8001, 0x4000 (nicht General), eine schon benannte: refused, nichts geschrieben */ }
#[test] fn a_second_story_with_the_same_name_gets_var_story_x_2() { }
#[test] fn append_step_refuses_a_duplicate_key_and_adds_new_flags_once() { }
```

- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features --test recipes_story` → FAIL (Varianten fehlen)
- [ ] **Step 3: Umsetzen** nach Interfaces; die Stubs der vier anderen Rezepte in `recipes.rs` verweigern mit „Kommt mit Task N“; `is_irreversible` liefert für die fünf `None` (Task 5/6 ändern das für Level-Script-Kopien). Feste Erwartungen anpassen: `types.rs` Unit-Test, `tests/cli.rs` `cli_recipe_list` (zwölf Zeilen), `tests/recipes_erzaehlen.rs:159` und `next_steps_after_3d5_lock_with_3d7_and_3d9` (jetzt: `story_join` frei), `catalog.rs`-Tests (Task 8 zieht die Beschreibung nach; hier nur, was ohne Katalogtext bricht).
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` grün.
- [ ] **Step 5: Commit** `feat(recipes): story recipe ids, story_common helpers, new_story names the var and writes the start step`

### Task 2: Script-Vorlagen und Textbausteine

**Files:**
- Create: `crates/g4-kernel/metadata/recipes/story_talk_pt.rotom`, `story_talk_hgss.rotom`, `story_reward_pt.rotom`, `story_reward_hgss.rotom`, `story_scene_pt.rotom`, `story_scene_hgss.rotom`, `story_visibility_head.rotom`, `story_visibility_rule_appear.rotom`, `story_visibility_rule_vanish.rotom`; `crates/g4-kernel/src/recipes/story_scripts.rs` (+ `story_scripts/tests.rs`)
- Modify: `crates/g4-kernel/src/recipes.rs` (`pub mod story_scripts;`)

**Interfaces:**
- Produces (`story_scripts.rs`, `pub(crate)`):
  - `pub(crate) fn talk_script(family, n: u16, local_id: u16, var_id: &str, from: u16, message: u16, tail: &Tail) -> String`
  - `pub(crate) fn reward_script(family, n, local_id, var_id, from, flag_id: &str, item: u16, message, tail) -> String`
  - `pub(crate) fn scene_script(family, n, var_id, next: u16, message) -> String`
  - `pub(crate) enum Tail { Jump(String), End }` (Zeile `    Jump {name}` oder `    End`)
  - `pub(crate) fn visibility_block(story: u16, header: u16, n: u16, rules: &[VisibilityRule], tail: &Tail) -> String`, `pub(crate) struct VisibilityRule { pub var_id: String, pub step: u16, pub flag_id: String, pub appears: bool }`, `pub(crate) fn visibility_rule(rule: &VisibilityRule) -> String` (vier Zeilen plus `endif`, eingerückt wie der Block), `pub(crate) fn visibility_name(story: u16, header: u16) -> String` (`sichtbarkeit_{story}_{header}`), `pub(crate) fn visibility_end_marker(story, header) -> String` (`// ende sichtbarkeit_{story}_{header}`), `pub(crate) fn insert_rule(source: &str, story, header, rule: &str) -> Result<String, KernelError>` (findet die Markerzeile, fügt die Regel vor der `Jump`/`End`-Zeile davor ein; ohne Marker `Refused("Der Sichtbarkeits-Block von Geschichte {S} fehlt in Scriptdatei {f}.")`).
- Vorlagen (Pt; HGSS mit `NPCMsg`, `CloseMsg`, `SetVar`, `VAR_SPECIAL_RESULT`):

```
script gespraech_{local_id}_{value} #{n}:
    LockAll
    FacePlayer
    if {var} == {from} then
        Message {message}
        WaitButton
        CloseMessage
        SetVarFromValue {var}, {value}
        ReleaseAll
        End
    endif
    ReleaseAll
{tail}
```

```
script belohnung_{local_id}_{n} #{n}:
    LockAll
    FacePlayer
    if {var} >= {from} then
        if {flag} == 0 then
            Message {message}
            WaitButton
            CloseMessage
            GiveItem {item}, 1, VAR_RESULT
            SetFlag {flag}
            ReleaseAll
            End
        endif
    endif
    ReleaseAll
{tail}
```

```
script szene_{n} #{n}:
    LockAll
    Message {message}
    WaitButton
    CloseMessage
    SetVarFromValue {var}, {value}
    ReleaseAll
    End
```

Block (`story_visibility_head.rotom` ist die erste Zeile, Regeln dazwischen, dann `{tail}`, dann der Marker):

```
script sichtbarkeit_{story}_{header} #{n}:
    if {var} >= {step} then
        ClearFlag {flag}
    else
        SetFlag {flag}
    endif
{tail}
// ende sichtbarkeit_{story}_{header}
```

- [ ] **Step 1: Failing tests** (`story_scripts/tests.rs`): `talk_pt_matches_the_template`, `talk_hgss_uses_npcmsg_closemsg_setvar`, `reward_script_checks_var_then_flag`, `scene_script_switches_to_next`, `visibility_block_with_two_rules_and_a_jump`, `insert_rule_adds_before_the_tail_and_keeps_the_marker`, `insert_rule_without_marker_is_refused`, und ein Compile-Test je Familie über `TextFixture` (`scripts::write_source` + Compile ohne Fehler: `talk_and_reward_compile_in_both_families`, Flag-Vergleich `== 0` eingeschlossen; scheitert `if FLAG == 0`, nimmt die Vorlage `if {flag} == 1 then` / `else`-Zweig, Ruling notieren).
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS
- [ ] **Step 5: Commit** `feat(recipes): story script templates per family and the visibility block with markers`

### Task 3: „Gespräch schaltet weiter“

**Files:**
- Create: `crates/g4-kernel/src/recipes/talk_advances.rs` (+ `talk_advances/tests.rs` für Unit-Fälle ohne Fixture)
- Modify: `crates/g4-kernel/src/recipes.rs` (Stub ersetzen)
- Test: `crates/g4-kernel/tests/recipes_story.rs`

**Interfaces:**
- Consumes: Task 1 (`story_common`), Task 2 (`talk_script`, `Tail`), `dialog::dialog_of` (nur zur Einordnung `Local`/`None`/`Common`/`Trainer`), `scripts::{append_source, snapshot_script, snapshot_compile_state, snapshot_archive, script_file_step, archive_file_step, compile_state_step, restore_compile_state}`, `stories::append_step`.
- Produces: `pub struct TalkAdvancesInput { pub story: String, pub figure: FigureRef, pub from_step: u16, pub text: String, pub step_name: Option<String> }`, `pub struct TalkAdvancesReport { pub step: String, pub value: u16, pub event_file: u16, pub object: u16, pub script_file: u16, pub slot: u16, pub previous_slot: u16, pub archive: u16, pub message: u16 }`, `pub fn plan`, `pub fn apply`.
- `check`: Textprojekt und Scripts übernommen (`ADOPT_FIRST`), `load_story`, `step_of(from_step)` muss es geben („Schritt {k} gibt es in „X“ nicht.“), `dialog_step_taken(k+1)`, Figur (kein Trainer, Script `Local` oder `None`, sonst die Sätze der Entscheidung), Scriptdatei des Headers rotom, Archiv und Eventdatei schreibbar, Text nicht leer, Slot, Nachricht.
- `plan`: `Draft { messages: [DraftMessage { archive, text }], notes: ["Figur {k} bekommt den Zweig „ab Schritt {k}“ und schaltet auf {k+1}.", "Script #{n} in Scriptdatei {f}, Nachricht {m} in Archiv {a}."] }`.
- `apply`: Transaktion `Gespräch von Figur {k} schaltet „X“ auf {k+1}` (Nachricht, `object.script = n`), `append_source` mit `talk_script(…, Tail::Jump(alter Name) | Tail::End)`, Dateischritte, `append_step` (`uses: [Object { file, local_id }, Script { file, slot: n }, Message { archive, index }, Var(var_id)]`), `squash_since`; `next: next_steps(&[("figure_appears", "Figur erscheint ab Schritt"), ("scene_on_enter", "Szene beim Betreten")])`.

- [ ] **Step 1: Failing tests**:

```rust
#[test] fn plan_of_a_talk_writes_nothing_and_names_slot_message_and_value() {}
#[test] fn apply_of_a_talk_wraps_the_old_script_and_is_one_undo_step() { /* Objekt zeigt auf #n, Quelle enthält "Jump zwei" (alter Slot-Name), Archiv +1, story steps: s0, s1 (dialog, made.recipe == "talk_advances", uses 4), undo stellt alles her */ }
#[test] fn a_figure_without_a_script_gets_a_tail_end() {}
#[test] fn a_shared_script_stays_for_the_other_figure() {}
#[test] fn a_taken_value_is_refused_before_writing() {}
#[test] fn a_trainer_a_common_script_and_a_missing_step_are_refused() {}
#[test] fn hgss_talk_uses_npcmsg_and_setvar() {}
```

- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS (auch `recipes_erzaehlen`, `dialog` grün)
- [ ] **Step 5: Commit** `feat(recipes): talk_advances wraps the figure's script with a branch that advances the story`

### Task 4: „Belohnung einmalig“

**Files:**
- Create: `crates/g4-kernel/src/recipes/reward_once.rs`
- Modify: `crates/g4-kernel/src/recipes.rs`
- Test: `crates/g4-kernel/tests/recipes_story.rs`

**Interfaces:**
- Consumes: Tasks 1–3 (dasselbe Hüll-Muster), `flags::name_flag`.
- Produces: `pub struct RewardOnceInput { pub story: String, pub figure: FigureRef, pub from_step: u16, pub item: u16, pub text: String, pub flag: u16, pub step_name: Option<String> }`, `pub struct RewardOnceReport { pub step: String, pub flag: u16, pub flag_id: String, pub event_file: u16, pub object: u16, pub script_file: u16, pub slot: u16, pub previous_slot: u16, pub archive: u16, pub message: u16 }`.
- `check` wie Task 3 ohne `dialog_step_taken`, dazu `check_pool_number(Flag)`, `item` 1 ..= Zahl der Gegenstände des Spiels (`store`-Zählung des Item-Archivs; sonst „Gegenstand {i} gibt es nicht.“).
- `apply`: `name_flag` (`FLAG_STORY_<NAME>_BELOHNUNG_<local_id>`), Transaktion `Belohnung von Figur {k} in „X“`, Hülle `reward_script`, Dateischritte, `append_step` (`key "r{flag}"`, `kind Reward`, `uses: [Object, Script, Message, Flag(flag_id)]`, Flag in `story.flags`), `squash_since`; `next: play_here`.

- [ ] **Step 1: Failing tests**: `apply_of_a_reward_names_the_flag_wraps_the_script_and_lists_the_flag_in_the_story`, `a_saved_reward_is_undone_byte_equal` (DSPRE-Fixture mit `store.save()` dazwischen, Muster `a_saved_figure_in_a_dspre_project_is_undone_and_redone`), `a_used_flag_number_a_bad_item_and_a_trainer_are_refused`, `reward_after_talk_chains_both_wrappers` (Figur mit Gespräch ab 0, dann Belohnung ab 1: Objekt zeigt auf die Belohnungs-Hülle, deren `Jump` auf `gespraech_…`, `dialog_of` liefert beide Nachrichten).
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS
- [ ] **Step 5: Commit** `feat(recipes): reward_once gives an item once behind a named flag`

### Task 5: „Figur erscheint/verschwindet ab Schritt“

**Files:**
- Create: `crates/g4-kernel/src/recipes/figure_appears.rs`, `crates/g4-kernel/src/recipes/level_script.rs` (gemeinsam mit Task 6)
- Modify: `crates/g4-kernel/src/recipes.rs` (`is_irreversible` für `figure_appears`/`scene_on_enter`: `plan`-Prüfung „Level-Script-Datei anlegen“)
- Test: `crates/g4-kernel/tests/recipes_story.rs`

**Interfaces:**
- Consumes: Tasks 1, 2 (`visibility_*`, `insert_rule`), `scripts::{read_source, write_source, write_and_compile, new_script_file_reported, next_script_file, source_kind}`, `store.header_table()`, `tx.edit_header` (Setzen von `level_script_id`, Muster `empty_region::set_events`).
- Produces (`level_script.rs`, `pub(crate)`): `pub(crate) struct LevelScriptPlan { pub file: u16, pub create: bool, pub doc: LevelScriptDoc, pub transition: Option<u16> /* bisheriges Ziel */ }`; `pub(crate) fn plan_level_script(store, header) -> Result<LevelScriptPlan, KernelError>` (eigene Datei, geteilt → `create: true` mit Kopie, 0 oder Binary → `create: true` mit leerem Dokument); `pub(crate) fn ensure_level_script(store, plan: &LevelScriptPlan, created: &mut Created) -> Result<u16, KernelError>` (legt die Datei an, wenn `create`); `pub(crate) fn transition_entry(doc: &LevelScriptDoc) -> Option<u16>`, `pub(crate) fn set_transition(doc: &mut LevelScriptDoc, script_id: u16)`, `pub(crate) fn push_condition(doc: &mut LevelScriptDoc, var: u16, value: u16, script_id: u16) -> u16 /* Index */`; `pub(crate) const VISIBILITY_ENTRY: &str = "on_transition"` (Ruling nach der Messung, siehe Step 1).
- `figure_appears.rs`: `pub struct FigureAppearsInput { pub story: String, pub figure: FigureRef, pub step: u16, pub appears: bool, pub flag: u16, pub step_name: Option<String> }`, `pub struct FigureAppearsReport { pub step: String, pub flag: u16, pub flag_id: String, pub event_file: u16, pub object: u16, pub level_script: u16, pub level_script_created: bool, pub script_file: u16, pub slot: u16, pub rules: u32 }`.
- `check`: Textprojekt, Scripts übernommen, `load_story`, `step_of(step)` muss es geben, Figur mit `hidden_flag == 0` (sonst der Satz der Entscheidung), `check_pool_number(Flag)`, Scriptdatei rotom, Level-Script-Plan, bisheriges `on_transition`-Ziel ein Slot der Scriptdatei (`slot_name`) oder keins.
- `plan`: `notes` („Figur {k} bekommt das Sichtbarkeits-Flag FLAG_… und erscheint ab Schritt {s}.“, „Block sichtbarkeit_{S}_{H} in Scriptdatei {f}“), `irreversible: ["Level-Script-Datei {n} wird angelegt und bleibt"]` bei `create`.
- `apply`: `ensure_level_script`; `name_flag`; Transaktion `Figur {k} erscheint ab Schritt {s} in „X“` (`object.hidden_flag = flag`; `level_script_id` bei `create`); Script: Block vorhanden → `insert_rule` + `write_and_compile`, sonst `append_source(visibility_block(…, Tail::Jump(alt) | End))` und `set_transition(doc, n)` + `write_source(LevelScript)`; Dateischritte für Scriptdatei (und Level-Script-Datei: `script_file_step` gilt für jede Scriptdatei, auch `.json`; prüfen, sonst `snapshot_script` auf die Level-Script-Datei anwenden), `append_step` (`key "a{flag}"`, `kind Appearance`, `uses: [Object { file, local_id }, Flag(flag_id), Script { file, slot: block }]`; kein `LevelScript`-Verweis, weil `condition` eine `var_conditions`-Nummer ist und der Block über `on_transition` läuft), Flag in `story.flags`; `squash_since`; `next: play_here`.

- [ ] **Step 1: Messung (gated, vor dem Umsetzen):** auf `3d-7-pt` eine Figur mit `hidden_flag = F` (frei), Level-Script des Orts mit `on_transition` → Script, das `SetFlag F` setzt, bauen, `g4emu`: Ort betreten, `walk-until-blocked` durch die Kachel. Ist die Figur beim **ersten** Betreten schon weg, bleibt `VISIBILITY_ENTRY = "on_transition"`; sonst dasselbe mit `on_load`; das Ergebnis als Ruling in die Gate-Datei (Task 12) und als Kommentar an die Konstante.
- [ ] **Step 2: Failing tests**:

```rust
#[test] fn apply_of_an_appearance_sets_hidden_flag_names_the_flag_and_adds_the_block() { /* Quelle enthält "script sichtbarkeit_0_0 #", "ClearFlag FLAG_STORY_TESTFADEN_SICHTBAR_4", Marker; level script: on_transition → n; Fixture ohne vorheriges on_transition → Tail End */ }
#[test] fn a_previous_transition_script_becomes_the_jump_target() {}
#[test] fn a_second_rule_grows_the_block_and_undo_removes_it() {}
#[test] fn vanish_swaps_set_and_clear() {}
#[test] fn a_figure_with_a_flag_and_a_common_transition_script_are_refused() {}
#[test] fn a_shared_level_script_is_copied_for_this_header() { /* zwei Header auf dieselbe Level-Script-Datei: created nennt die neue, Header 1 behält die alte, is_irreversible == Some("Level-Script-Datei anlegen") */ }
```

- [ ] **Step 3: Lauf** → FAIL → **Step 4: Umsetzen** → **Step 5: Lauf** → PASS
- [ ] **Step 6: Commit** `feat(recipes): figure_appears with a visibility block in the transition script and a level-script entry`

### Task 6: „Szene beim Betreten“

**Files:**
- Create: `crates/g4-kernel/src/recipes/scene_on_enter.rs`
- Modify: `crates/g4-kernel/src/recipes.rs`, `crates/g4-kernel/src/recipes/level_script.rs` (falls Task 5 `push_condition` nicht brauchte)
- Test: `crates/g4-kernel/tests/recipes_story.rs`

**Interfaces:**
- Produces: `pub struct SceneOnEnterInput { pub story: String, pub header: u16, pub step: u16, pub text: String, pub step_name: Option<String> }`, `pub struct SceneOnEnterReport { pub step: String, pub value: u16, pub level_script: u16, pub level_script_created: bool, pub condition: u16, pub script_file: u16, pub slot: u16, pub archive: u16, pub message: u16 }`.
- `check`: Textprojekt, Scripts übernommen, `load_story`, `step_of(step)`, `dialog_step_taken(step + 1)`, Scriptdatei rotom, Archiv schreibbar, Level-Script-Plan, Text, Slot, Nachricht; dieselbe Bedingung (`var`, `value`) schon in `var_conditions` → „Der Ort hat schon eine Szene für Schritt {k}.“.
- `plan`: `messages`, `notes` („Beim Betreten von Header {h} mit Schritt {k} läuft die Szene und schaltet auf {k+1}.“), `irreversible` bei `create`.
- `apply`: `ensure_level_script`, Transaktion `Szene beim Betreten von Header {h} in „X“` (Nachricht, `level_script_id` bei `create`), `append_source(scene_script)`, `push_condition` + `write_source(LevelScript)`, Dateischritte, `append_step` (`key "s{k+1}"`, `kind Dialog`, `uses: [LevelScript { file, condition }, Script, Message, Var]`), `squash_since`; `next: play_here`.

- [ ] **Step 1: Failing tests**: `apply_of_a_scene_adds_a_condition_and_a_script_and_a_step`, `a_scene_on_a_shared_level_script_copies_it`, `a_second_scene_for_the_same_step_and_a_taken_value_are_refused`, `undo_of_a_scene_restores_level_script_source_and_story`.
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS
- [ ] **Step 5: Commit** `feat(recipes): scene_on_enter with a level-script condition that plays once and advances`

### Task 7: Gated Kernel-Tests und `g4k`

**Files:**
- Modify: `crates/g4-kernel/tests/recipes_story.rs` (gated Teil), `crates/g4-kernel/tests/cli.rs` (`cli_recipe_list` zwölf Zeilen; `cli_recipe_apply_new_story_prints_the_outcome_and_saves` auf `TextFixture`), `crates/g4-kernel/src/bin/g4k.rs` (nur, wenn `recipe apply` ein `--input @datei.json` braucht: Rulings)
- Test: dieselben

**Interfaces:**
- Consumes: Tasks 1–6; `vanilla_copy()`, `scripts::adopt`, `textproject` Einrichtung wie `tests/stories.rs` (`text_project_with_a_story_builds_from_base_byte_equal`).

- [ ] **Step 1: Failing tests** (gated, `G4_TEST_PROJECT`, Pt und HGSS; Dp übersprungen):

```rust
#[test] fn vanilla_story_chain_is_five_undo_steps_and_git_status_is_empty_after_undo() { /* Kopie als Textprojekt in Git (Muster stories.rs); new_story → new_figure (3d.5) → talk_advances → new_trainer_here → figure_appears → scene_on_enter (Ort = derselbe Header) → reward_once; undo_depth wächst je Rezept um 1 (new_trainer_here: Sperre + 1); Datei-Diff zu Git nennt nur die erwarteten Pfade; danach undo bis mark: `git status --porcelain` leer (Trainer-Member bleibt: vorher anlegen und committen); eprintln! je Rezept ms */ }
#[test] fn vanilla_pool_shrinks_by_one_var_and_two_flags() { /* pool(Var).free vorher − 1, pool(Flag).free − 2 nach der Kette (Index neu gebaut) */ }
#[test] fn vanilla_story_chain_builds_and_checks_clean() { /* build; `checks::run` (vanilla_checks-Äquivalent, wie tests/checks_runner.rs) 0 Fehler; die Scripts kompilieren */ }
```

- [ ] **Step 2: Lauf** auf `3d-7-pt`, `3d-7-hgde`, `3d-7-hgus` (`cargo test --release -p g4-kernel --all-features --test recipes_story -- vanilla --nocapture --test-threads 1`) → FAIL → **Step 3: Umsetzen** (Fehler in den Rezepten beheben; Zahlen notieren) → **Step 4: Lauf** → PASS auf allen drei; `cli.rs` grün.
- [ ] **Step 5: Commit** `test(recipes): gated story chain on the three games, pool counts, cli list with twelve recipes`

## Teil B: App

### Task 8: Tauri, MCP, Doku der Befehle

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/stories.rs` (`StoryListDto.free_vars: Option<u32>`, `StoryRowDto.flags: Vec<String>`; `story_list_impl` liest `session.refs` wie `free_flag_impl` und `flags::pool`), `apps/studio/src-tauri/src/mcp/catalog.rs` (`recipe_args`-Beschreibung: die fünf Rezepte mit ihren Eingabeschlüsseln; Test `recipe_apply description names every irreversible` bleibt grün), `apps/studio/src-tauri/src/mcp/access.rs` (nichts, `Depends` über `is_irreversible`; Test ergänzen `story_recipes_are_depends`), `docs/mcp.md` (Zeilen `recipe_*`: die Eingaben; `story_list`: `free_vars`, `flags`), `docs/formats/recipes.md` (fünf Zeilen), `docs/formats/stories.md` (Abschnitt „Rezepte“: Schlüssel, `made`, Hülle, Block, Szene, Belohnung, Beispiel `made.recipe` auf `talk_advances` korrigiert)
- Test: `apps/studio/src-tauri/src/commands/stories/tests.rs` (`story_list_names_free_vars_and_flags`), `apps/studio/src-tauri/src/commands/dispatch.rs` (Beispielargumente unverändert: `recipe_apply` bleibt generisch; Test, dass `recipe_apply` mit `recipe: new_story` über die Brücke den Kernel erreicht), `mcp/catalog.rs` Tests

- [ ] **Step 1: Failing tests** → **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** `cargo test -p studio --lib` → PASS
- [ ] **Step 5: Commit** `feat(app): story_list with the free-variable pool and the story flags, recipe docs for the five story recipes`

### Task 9: Frontend-API, „Neue Geschichte“ über das Rezept, Pool, Graph speichern

**Files:**
- Modify: `apps/studio/src/recipes/api.ts` (`RecipeId` + fünf; `NewStoryInput/Report`, `FigureRef`, `TalkAdvancesInput/Report`, `FigureAppearsInput/Report`, `SceneOnEnterInput/Report`, `RewardOnceInput/Report` mit den Rust-Feldnamen), `apps/studio/src/api.stories.ts` (`StoryListDto.free_vars`, `StoryRowDto.flags`), `workbench/recipes.ts` (`KNOWN` + fünf, `NEXT` wie die Kernel-`next`, `ALWAYS = ["edit_dialog", "story_join"]`, `lockReason` Standard „Kommt später“), `stories/NewStoryDialog.tsx` (+ test; Variante Variable: `freeFlag("var")` beim Öffnen, Zeile `[data-pool-free]` „Noch {n} Variablen frei“ / „Keine freie Variable im allgemeinen Bereich“, `onCreate` ruft `recipeApply("new_story", …)`; Variante Flag unverändert), `stories/StoriesSurface.tsx` (Ergebnis des Rezepts → `onCreated`, Toast-Text bleibt), `stories/GeschichtenArea.tsx` (Untertitel „{n} Geschichten · {free} Variablen frei“, ohne Index nur „{n} Geschichten“), `stories/graph/ThreadGraph.tsx` (+ test; `onLayout?`), `stories/StoryDetail.tsx` (+ test; `onLayout` verzögert 500 ms → `onUpdate({ layout })`; Tooltip der Art-Plakette „Angelegt von „{Titel des Rezepts}“ (Studio {studio})“ bei `made`, Schritt-Notizen unter dem Namen), `stories/storyWords.ts` (`recipeTitle(id)`)
- Test: `apps/studio/src/recipes/api.test.ts`, `workbench/recipes.test.ts`, `stories/*.test.tsx`

- [ ] **Step 1: Failing tests** (vitest): `new story dialog takes the free variable and calls the recipe`, `without a free variable the dialog says so and locks the button`, `the stories row counts the pool`, `dragging a node writes the layout once after 500 ms`, `a step made by a recipe shows the recipe in the badge tooltip`, `next steps after 3d7 free story_join`
- [ ] **Step 2: Lauf** `pnpm test` → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS, `pnpm build`, Check-Skripte
- [ ] **Step 5: Commit** `feat(stories): new story over the recipe with the pool line, layout saved on drag, recipe badge tooltip`

### Task 10: Schritt-Rezepte als Dialoge, Einstiege, Inspector-Hinweis

**Files:**
- Create: `apps/studio/src/stories/recipes/StoryRecipeDialog.tsx` (+ `.module.css`, `.test.tsx`), `TalkAdvancesForm.tsx`, `FigureAppearsForm.tsx`, `SceneOnEnterForm.tsx`, `RewardOnceForm.tsx`, `FigureField.tsx` (+ test), `StepField.tsx`, `StepRecipes.tsx` (+ test), `storyRecipeModel.ts` (+ test: `StoryRecipeRequest = { recipe: "talk_advances" | "figure_appears" | "scene_on_enter" | "reward_once"; prefill?: { story?: string; figure?: FigureRef; header?: number } }`, `inputOf(form)`, `defaultStepName(form, names)`), `app/useStoryRecipe.ts`, `app/StoryRecipeHost.tsx` (+ test)
- Modify: `workbench/WorkbenchContext.tsx` (`openStoryRecipe(request)`), `workbench/panels.tsx` (Durchreichen), `App.tsx` (+ test; Zustand, `<StoryRecipeHost>` neben `WizardHost`, `onRecipeApplied` nach Erfolg, Toast `Schritt „{name}“ angelegt`), `app/WizardHost.tsx` (`step("story_join", outcome, input)` → `openStoryRecipe({ recipe: "talk_advances", prefill: { figure: { header: input.header, local_id: report.local_id } } })`), `workbench/wizards/wizardSpecs.ts` (`figureResult`: `story_join` verfügbar), `workbench/inspector/DialogSection.tsx` (+ test; „Dialog anlegen…“ aktiv → `openStoryRecipe` mit der Figur; `ANLEGEN_LATER` entfällt), `stories/StoryDetail.tsx` (Mount `<StepRecipes story onRecipe>`), `workbench/inspector/EventEditor.tsx` (+ test; Hinweiszeile `[data-story-hint]` unter dem `FlagField` in Erzählen: „Geschichte „X“: {Schrittname}“ über `useStoryHint(kind, number)` aus `storyList` (Flags: `flags` enthält die ID des Eintrags; Variable: `progress.var`), Hook in `stories/useStoryHint.ts`)
- Test: die genannten `.test.tsx`

**Interfaces:**
- `StoryRecipeDialog` props: `{ request: StoryRecipeRequest; stories: StoryRowDto[]; project; revision; headers; onApplied(outcome, request); onClose; onError }`; Marker `[data-story-recipe-dialog=<recipe>]`, Titel = Rezept-Titel, Beschreibung je Rezept („Die Figur bekommt einen Gesprächszweig: ab dem gewählten Schritt sagt sie den Text und schaltet die Geschichte weiter.“ usw.), Felder je Formular, rechts `[data-story-recipe-notes]` mit `Draft.notes` und `Draft.irreversible` als `Notice kind="warning"`, Fuß „Abbrechen“ und primär „Anlegen“ (`data-story-recipe-apply`).
- `FigureField`: `{ header: number | null; localId: number | null; onChange; headers; project; revision }`: Ort über `HeaderPicker`, Figur über `Picker` der Objekt-Events (`useEvents(eventFile)` des Headers; Zeile „Figur {local_id} · Aussehen {graphics_id}“, Trainer mit Plakette „Trainer“ und gesperrt für Gespräch und Belohnung mit Tooltip „Trainer sprechen über ihre Kampftexte“).
- `StepField`: `Select` über `story.steps` nach `value` („{value}: {name}“), Standard der höchste Wert.
- Formulare: Gespräch (Geschichte, Figur, ab Schritt, Text mit `DSScreen`); Erscheinen (Geschichte, Figur, Schritt, `Segmented` erscheint | verschwindet, `FlagField kind="flag"` vorbelegt aus `freeFlag`); Szene (Geschichte, Ort `HeaderPicker`, Schritt, Text); Belohnung (Geschichte, Figur, ab Schritt, Gegenstand `PickField choice="item"`, Text vorbelegt „Du erhältst {Gegenstand}!“, `FlagField`); `step_name` aus `defaultStepName` (Ortsname aus `headers`, Gegenstandsname aus den Namen).
- `StepRecipes`: `{ story: StoryDto; textProject: boolean; onRecipe(request) }`, vier `Button`s `[data-story-recipe=<id>]`, gesperrt mit Tooltip bei Flag-Fortschritt oder ohne Textprojekt.

- [ ] **Step 1: Failing tests**: `the step bar opens the talk dialog with the story prefilled`, `a figure without a script opens the talk recipe from the dialog section`, `story_join after a new figure opens the talk recipe with the figure`, `the figure field lists the objects of the header and locks trainers for talk`, `applying shows the plan notes and then the toast`, `the inspector names the story of a visibility flag in Erzählen`
- [ ] **Step 2: Lauf** → FAIL → **Step 3: Umsetzen** → **Step 4: Lauf** → PASS, `pnpm build`, Check-Skripte; **UI-Test** über die Test-Umgebung (Ports 1770/1771) auf einer Kopie `3d-7-web` als Textprojekt: Geschichte anlegen, Figur per Rezept, Gespräch, Erscheinen, Szene, Belohnung über die Dialoge; `call.sh history` nennt je einen Schritt; ⌘Z bis zum Anfang; Screenshots `blick-schritt-rezept-{gespraech,erscheinen,szene,belohnung}-hell.png`, `blick-pool-hell.png`, `blick-geschichte-schritte-dunkel.png` nach `target/agent/gate-<datum>/3d-7/`
- [ ] **Step 5: Commit** `feat(stories): step recipes as dialogs from the story, the figure and the dialog section; story hint in the inspector`

## Teil C: Szenarien, Gate, Doku

### Task 11: Szenarien `pt-geschichte` und `hg-geschichte`

**Files:**
- Create: `scripts/agent/scenarios/pt-geschichte.yaml`, `scripts/agent/scenarios/hg-geschichte.yaml`
- Modify: `docs/agent-gate.md` (Abschnitt „Phase 3d.7“: Befehle, Platzhalter, Kacheln), `scripts/g4emu/README.md` (falls die Liste der Szenarien dort steht)

**Interfaces:**
- Consumes: Tasks 1–8; `pt-erzaehlen.yaml`/`hg-erzaehlen.yaml` (3d.5) als Vorspann, `pt-trainer-begegnung.yaml` (Geschenk), `pt-raum-assistent.yaml` (Raum).
- Produces: zwei Szenarien nach der Entscheidung „Szenarien“; Platzhalter `BASIS`, `NEUER_HEADER`, `RAUM_HEADER`, `X_A Z_A`, `X_B Z_B`, `TUER_X TUER_Z`, `S_GIFT`, `S_T L_T`; Kacheln aus den Karten der Kopie messen (Pt: Figur A bei (48, 19) oder nächstes freies Feld mit Zugang von Süden, Trainer B bei (54, 17) mit Blick nach Süden und Sicht 3, Tür des Raums bei (46, 18) mit Türkacheln; HG entsprechend auf Route-29-Karte 1).

- [ ] **Step 1: Schreiben** beider Szenarien mit `expect` auf `pool_free` (`free_flag` vor und nach der Kette: Variablen −1, Flags −2), `check: [g4k, text, check, …]` 0 Fehler (oder das `checks`-Äquivalent, das `pt-pruefungen.yaml` nutzt)
- [ ] **Step 2: Lauf** auf `3d-7-szen-pt` und `3d-7-szen-hg` (`uv run g4emu scenario … --out ../../target/agent/gate-$(date +%F)/3d-7/<name>`) → grün; Bilder `gespraech`, `szene`, `belohnung`, `belohnung-2` im Out-Ordner; bei Rot: Kernel oder Szenario beheben, Befund notieren
- [ ] **Step 3: Commit** `test(g4emu): pt-geschichte and hg-geschichte play a recipe-made story end to end`

### Task 12: Design-System-Nachtrag, Gate-Datei, Doku, Roadmap

**Files:**
- Create: `docs/phase-3d-7-gate.md` (Muster `docs/phase-3d-3-gate.md`: Stand, Gate-Text aus Spezifikation §4 Zeile 3d.7 wörtlich, Legende „Wer“, Namen in der Oberfläche: `[data-story-recipe]`, `[data-story-recipe-dialog]`, `[data-story-recipe-notes]`, `[data-story-recipe-apply]`, `[data-pool-free]`, `[data-story-hint]`; Sätze „Noch {n} Variablen frei“, „Keine freie Variable im allgemeinen Bereich“, „Rezepte brauchen eine Variable als Fortschritt“, „Trainer sprechen über ihre Kampftexte“; Vorbereitung: Kopien `3d-7-{pt,hgde,hgus}`, `3d-7-szen-{pt,hg}`, `3d-7-web`; Punkte (1) gated Kernel (Zahlen aus Task 7), (2) Szenarien (Task 11, Bilder), (3) Browser-Teil (Task 10, Bilder), (4) Rulings: `VISIBILITY_ENTRY`, voller Beutel, Flag-Vergleich; „Beim Nutzer offen“: Hand mit echter Spielfigur, Blick, `.fig`)
- Modify: `design-system/README.md` (Nachtrag 3d.7: Schritt-Rezepte als Dialoge mit Plan-Sätzen, Pool-Zeile, Graph ziehen, Hinweiszeile im Inspector; Komponenten `FigureField`, `StepField`, `StepRecipes`), `docs/ROADMAP.md` (Zeile 3d.7 „Stand“, Zeile 3d der Übersicht), `docs/formats/stories.md` (Abschnitt „Rezepte“ aus Task 8 prüfen und ergänzen), `docs/agent-gate.md` (Lauf der Gate-Datei)
- Danach: Testkopien `3d-7-*` löschen (`_saves`, `Manual` bleiben), Rückmeldung nach dem gemeinsamen Auftrag; **kein** `scripts/gate.sh` im Worktree (läuft auf main).

- [ ] **Step 1: Gate-Dokument und Nachträge schreiben**, Bilder benennen
- [ ] **Step 2: Prüfen** `pnpm test`, `cargo test -p g4-kernel --all-features`, `cargo test -p studio --lib`, Check-Skripte grün auf dem letzten Commit
- [ ] **Step 3: Commit** `docs(3d.7): gate of the story recipes, design-system addendum, roadmap status`

## Reihenfolge und Parallelität

- **1 → 2 → 3 → 4 → 5 → 6 → 7** (Kernel, seriell: alle hängen an `story_common` und den `match`-Armen; 4 und 5 könnten parallel, sie berühren `recipes.rs` nur in je einem Arm — der Orchestrator entscheidet nach Last).
- **8 ∥ 9** nach 7; **10** nach 9; **11** nach 8 (braucht nur Kernel und MCP, kann neben 10 laufen); **12** zuletzt.
- Gated Tests laufen je Task auf frischen Kopien `3d-7-{pt,hgde,hgus}` (`G4_TEST_PROJECT`), die Kopien bleiben bis Task 12.

## Offen für die Umsetzung (Rulings des Orchestrators)

- **`VISIBILITY_ENTRY`** nach der Messung in Task 5 (`on_transition` oder `on_load`).
- **Flag-Vergleich** `if FLAG == 0 then` (Task 2): kompiliert es nicht, `if FLAG == 1 then` mit leerem Then-Zweig und `else`.
- **`script_file_step` für Level-Script-Dateien** (`.json`): deckt `snapshot_script` sie ab (sie sind Scriptdateien)? Sonst eigener `FileStep` über `FileChange` wie `stories::entities`.
- **`GiveItem` bei vollem Beutel** bleibt ungeprüft; `item` nur gegen die Zahl der Gegenstände geprüft.
- **Kacheln der Szenarien** aus den Karten der Kopien messen (Permissions wie `pt-flag.yaml`).
- **`append_step`** in `stories/entities.rs` (Task 1) ist die einzige Änderung an 3d.6-Dateien; sie hängt nur an.
- **MCP-Katalog-Grenze** (3d.6 hob sie auf 80, 75 Werkzeuge): 3d.7 fügt kein Werkzeug hinzu (alles über `recipe_*`).
