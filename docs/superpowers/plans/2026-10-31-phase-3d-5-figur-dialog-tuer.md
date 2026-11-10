# Phase 3d.5 „Figur, Dialog, Tür“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die drei gesperrten Tasten der Rezept-Leiste werden frei: „Neue Figur“ (Objekt-Event, Script-Slot mit `Message n`/`NPCMsg n`, Nachricht im Textarchiv des Headers, optional ein Sichtbarkeits-Flag), „Tür verbinden“ (ein Warp-Paar zwischen zwei Orten über zwei Eventdateien, Türkacheln) und „Neuer Trainer hier“ (Trainer mit Team und Kampftexten, Objekt-Event mit Script `3000 + ID − 1` und Sichtlinie) liegen als Kernel-Module im Rahmen von 3d.2 (`plan` ohne Schreiben mit Entwurf, `apply` als ein Undo-Schritt mit `FileStep` für Scripts), über MCP `recipe_plan`/`recipe_apply`, `g4k recipe` und in Erzählen über die Rezept-Leiste mit Entwurf in der Schau; der Kernel kennt den Dialog einer Figur (`dialog_of` mit Sprungverfolgung und Zähler „geteilt“), der Inspector zeigt ihn in Erzählen als Feld „Dialog“ (Seiten in `DSScreen`) mit dem Dialog-Editor, der nur das eine Textarchiv ändert, eine geteilte Nachricht als Plakette „Geteilt mit N Figuren“ zeigt und sie auf Wunsch für diese Figur abspaltet; `Target::Message` trägt den Rückweg von der Nachricht zur Figur.

**Architecture:** Kernel: neues Modul `crates/g4-kernel/src/dialog.rs` (`dialog_of`, `split_message`; Sprünge über die Sprungbefehle je Familie aus `metadata/check_limits.yaml`, Grundlage `checks::scripts::MessageUse`), `data/refs/messages.rs` (Ziel `Target::Message`, Quelle Scripts), drei Rezept-Module `recipes/{new_figure,connect_doors,new_trainer_here}.rs` mit je `Input`, `Report`, `plan`, `apply`; `recipes/room.rs` gibt seine Tür-Helfer (`warp_at`, `outside_map`, `fires`, `set_door`, `local`) nach `recipes/doors.rs` ab, ohne ihr Verhalten zu ändern. Nachrichten neuer Figuren entstehen **nummeriert** über den Store (eine Transaktion mit Objekt-Event und Archiv), nicht als rotom-Literal (Befund: Literale werden nach Text dedupliziert und bei Änderung neu angehängt). Tauri und MCP bekommen `dialog_of` und `split_message`; die Rezepte laufen über `recipe_list`/`recipe_plan`/`recipe_apply`, deren Schema die Liste selbst liest. Frontend: `workbench/inspector/DialogSection.tsx` (Feld „Dialog“, DS-Seiten, Plakette „Geteilt“), `workbench/dialog/DialogEditor.tsx` (Modal über `editTextArchive`, „Für diese Figur abspalten“), `WarpField` zeigt das Gegenstück und „Tür verbinden…“, drei Assistenten `workbench/wizards/{NewFigureWizard,ConnectDoorsWizard,NewTrainerHereWizard}.tsx` nach dem Muster von `NewRoomWizard` mit `useDraftPlan` und `recipeApply`; `draft.ts` zeichnet den Marker mit dem Sprite des `look`. `g4emu`: ein Szenario je Spiel (`pt-erzaehlen`, `hg-erzaehlen`), das alle drei Rezepte über `studio mcp` anwendet und spielt.

**Tech Stack:** Rust (g4-kernel `Store`/`Tx`, `scripts`, `recipes`, `trainer_create`, rotom 0.7.0 über uxie 0.9.5 für Literale, serde), Tauri 2 (Befehle, `dispatch.rs`, MCP `access.rs`/`catalog.rs`/`tools.rs`), React 19 mit `ui/primitives` (`DSScreen`, `Badge`, `Picker`, `Dialog`, `MessageEditor` aus `workbench/text`), three.js (Sprite im Entwurf), vitest, `g4emu`-Szenarien (YAML über `studio mcp`).

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) (§1, §2 „rotom 0.7.0“, §3.3 Tiefe Erzählen und Rezepte, §4 Zeile 3d.5 wörtlich, §6, §7 „Zwei Befehlsdialekte“, §8); [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Tabelle „Phasenschnitt“ Zeile 3d.5; „Geschichten und Erzählen (Gutachten A)“ Zeilen Rezepte, Dialog bearbeiten, Tür verbinden; „Nicht in 3d“; „Was bewusst nicht entschieden ist“ Punkt rotom-Literale); [Gutachten A](../../research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md) §2.4 (Weg Figur → Text, 738/701 Sprünge, 66/100 geteilte Nachrichten), §3.3 (Rezepttabelle Neue Figur, Tür verbinden, Neuer Trainer hier, Wilde Pokémon hier; Live-Vorschau Punkt 3 und 4), §3.4 (Dialog bearbeiten, Tür verbinden), §5 Zeile E2 mit Gate, §6 Frage 3 und 7; [Plan 3d.2](2026-10-30-phase-3d-2-erzaehlen-rahmen.md) (Rahmen: `RecipeId`, `Draft`, `Outcome`, `NextStep`, `FileStep::Recipe`, `squash_since`, `RecipeBar`, `useDraftPlan`); [Plan 3d.1](2026-10-30-phase-3d-1-flags-variablen.md) (Felder `hidden_flag`, `FlagField`, `free_flag`, `g4emu`-Schritte `expect-flag`); [phase-3d-2-gate.md](../../phase-3d-2-gate.md) als Muster der Gate-Datei; [design-system/README.md](../../../design-system/README.md) (Nachtrag 3d.2 als Muster, Komponente „Plakette“).

**Stand** 31.10.2026: Basis: `main` nach dem Merge von 3d.1 (heute `fe6d591`, 3d.2 ist gemergt; 3d.1 liegt im Worktree `.claude/worktrees/phase-3d-1` auf `9903a38` und wird vor Beginn gemergt). Arbeit im Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-5`, Branch `phase-3d-5-figur-dialog-tuer` (anlegen von main nach dem Merge). Parallel laufen in eigenen Worktrees 3d.4 (`phase-3d-4-loeschen`: Textprojekt-Schema `deleted`, `data/refs.rs` mit Welt-Zielen, Warp-Löschen, Listen- und Inspector-Kopf-Plakette, Diff, MCP `plan_delete`/`delete_entity`/`restore_entity`) und 3d.6 Teil A (`phase-3d-6-geschichten`: Kernel-Modul `stories`, Atlas-Bereich „Geschichten“, Graph). Aus 3d.1 nutzt 3d.5: `crates/g4-kernel/src/flags/pool.rs` (`free_flag`, nur über den Befehl), `crates/g4-kernel/src/data/refs.rs` mit `data/refs/flagvars.rs` (Ordner `data/refs/`, `Target::Flag`), `apps/studio/src-tauri/src/commands/flags.rs` (`free_flag_impl`), `apps/studio/src/flags/{FlagField,FlagPicker}.tsx` (Feld „Sichtbarkeits-Flag“ im Assistenten), `apps/studio/src/workbench/inspector/eventFields.ts` (Feld `hidden_flag` mit `pick: "flag"`, `EventPatch.hidden_flag`), `scripts/g4emu/g4emu/steps.py` (`expect-flag`), `scripts/agent/scenarios/{pt,hg}-flag.yaml` (Muster für `adopt_text_project` und die Figur-Kacheln). Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`, Emulator `scripts/g4emu`.

## Befunde vor dem Plan (31.10.2026, main `fe6d591` und Worktree 3d.1 `9903a38`)

- **rotom-Literale** (die offene Frage der Entscheidungen): `Message "…"` lowert über `find_or_add_message` (rotom `src/compiler/ir/lowerer.rs:1644`) in uxie 0.9.5 `src/workspace.rs:528-556`: ist der Text im Archiv schon da, kommt **dessen** Index zurück; sonst wird der erste Slot, der nur aus Leerraum besteht, überschrieben; sonst angehängt. Folgen: zwei Figuren mit gleichem Text teilen sich eine Nachricht; ein geändertes Literal bekommt beim nächsten Compile einen **neuen** Index, die alte Nachricht bleibt verwaist. Darum schreibt „Neue Figur“ eine nummerierte Nachricht über den Store und `Message n`/`NPCMsg n` ins Script (Vanilla tut dasselbe: 4 999 von 5 043 Nachrichten als Nummer, Gutachten A §2.2).
- **Befehlsdialekte:** Platin `Message` (`check_limits.yaml:82`), HGSS `NPCMsg` mit `legacy_name: "Message"` (`scrcmd/hgss_v2.json:662-666`; `message_params` hgss `NonNPCMsg`, `NPCMsg`, `NonNPCMsgVar`, `NPCMsgVar`, `check_limits.yaml:96-100`). `start_hgss.rotom` und `hg-flag.yaml` (3d.1) nutzen `NonNPCMsg`/`NPCMsg`, `WaitButton`, `CloseMsg`; die Platin-Szenarien `Message`, `WaitButton`, `CloseMessage`. `StartTrainerBattle` (`platinum_v2.json:3458-3474`: `trainer_1`, `trainer_2` flex) und `TrainerBattle` (`hgss_v2.json:2974-2990`: `trainer_1`, `trainer_2`, `result` …) gibt es; ein Objekt mit Sichtlinie startet seinen Kampf aber über das Script `3000 + ID − 1` und das Sicht-Script der Engine (`scripts.rs:2024-2036` `TrainerScriptMeta.seen_script` Pt 3928, HGSS 3739; `workbench/trainerScript.ts:4-10`), nicht über ein eigenes Script.
- **Trainer anlegen** (Frage 7 des Gutachtens): `Store::append_trainer` (`store.rs:1560-1566`) schreibt beide Member und im Textprojekt die JSON sofort, „Not undoable“; `create_trainer_with` (`trainer_create.rs:112-350`) legt danach Name und Textoffset als `barrier_transaction("Trainer {id} angelegt", not_undoable)` an (Z. 320). Es bleibt eine Undo-Sperre, auch im Textprojekt. `set_trainer_texts_labelled` (Z. 363) ist ein eigener Undo-Schritt mit wählbarem Label; `TrainerText { kind, text }` (Z. 58), `TEXT_BEFORE_BATTLE` 0 (Z. 40), `ADOPT_FIRST` (Z. 28).
- **Nachrichten im Script:** `checks::scripts::MessageUse { script, command, message, line, col }` (`checks/scripts.rs:54-64`, `pub(crate)`); `walk` (Z. 241-305) sammelt je Datei alle nummerierten Argumente der `message_params`-Befehle, mit dem ersten Slot des umgebenden Scripts; Sprünge folgt es nicht. Das Archiv einer Scriptdatei ist `text_archive_id` des ersten Headers, der die Datei nennt (`ScriptMessageMissing`, Z. 716-721; `Store::text_archive_users` Z. 2407). Sprungbefehle tragen in den Datenbanken den Typ `u32 relative_jump` (`platinum_v2.json:341-349` `GoTo`), in der Quelle steht ein Label-Bezeichner; eine deklarative Liste je Familie gibt es nicht.
- **Scripts:** `script_stub(n)` (`scripts.rs:1305`), `append_source` (Z. 1323, `pub(crate)`: schreibt, kompiliert, stellt bei Fehler den alten Text her, liefert `touched_archives`), `add_script_reported` (Z. 1261: nächster Slot = höchster `#n` + 1), `declared_slots` (Z. 1245), `snapshot_script`/`script_file_step` (Z. 1783/1802), `snapshot_archive`/`archive_file_step` (Z. 1938/1947), `resolve_script` (Z. 2508, `ScriptTarget::File { file, line }`, lokale Scripts unter `FIRST_COMMON_SCRIPT` 2000), `new_script_file_reported` (Z. 1003).
- **Rezept-Rahmen:** `RecipeId` hat vier Varianten (`recipes/types.rs:16-21`); `next_steps` (Z. 189-209) sperrt jede unbekannte Kennung mit „Kommt mit 3d.5“, `adjust_terrain` mit „Kommt mit Phase 6“; `is_irreversible` (`recipes.rs:61-80`); `room.rs` hält die Tür-Helfer privat (`warp_count` 76, `warp_at` 82, `header_event_file` 97, `outside_map` 108, `fires` 142, `set_door` 149, `local` 163, `plan_entrance` 188, `warp` 309) und `connect` (Z. 321-357) schreibt ein Warp-Paar über ein oder zwei `tx.edit_event`; `apply` (Z. 463-531) endet mit `next_steps(&[("new_figure", "Neue Figur hier")])`. `catalog.rs:1457-1476` baut das Schema von `recipe_plan`/`recipe_apply` aus `recipes::list()`: neue Rezepte erscheinen ohne Änderung am Katalog; `access.rs:316-319` fragt `is_irreversible`; `commands/recipes.rs` `recipe_apply_impl` ruft `after_write` und `files_written(&[])`.
- **Events:** `EventPatch` (`commands/events.rs:49-64`, 15 Felder; 3d.1 ergänzt `hidden_flag`, `var`, `value`), `add_event_impl` (Z. 279-353: Objekt mit `hidden_flag: 0`, Patch in derselben Änderung, Label wählbar); `Tx::edit_event` (`store.rs:635`), `Tx::edit_text_archive` (Z. 726), `Store::edit_text_archive(index, label, edit) -> Option<ChangeId>` (Z. 1912-1920), `Tx::edit_trainer` (Z. 863). `TextArchiveDoc { key, lang, messages }`, `TextMessage { id, text, other }`, `default_message_id` (`text.rs:29-66`). `edit_text_archive_impl` (`commands/text.rs:65-114`) kennt `TextEdit::{Set, Append, RemoveLast}`, Label „edit text“.
- **Referenzindex:** `Target` hat sechs Varianten (`data/refs.rs:64-75`); 3d.1 hängt `Flag(u16)`, `Var(u16)` an (Worktree Z. 97-100) und legt den Ordner `data/refs/` an (`flagvars.rs`, 461 Zeilen); `TargetDto` (`commands/data/dto.rs:556-563`). 3d.4 erweitert `refs.rs` um Welt-Ziele: `refs.rs` ist Konfliktstelle, nur anhängen.
- **Frontend:** `EventEditor.tsx` zeigt in Erzählen für `bg` den Satz „Der Text des Schilds kommt mit 3d.5.“ (`LATER`, Z. 65; `narrating` Z. 147; `Notice` Z. 178), `TrainerField` (Z. 187), `WarpField` (Z. 257). `eventFields.ts`: `from` (Z. 22-23), `NARRATING` (Z. 132-136), `eventFieldsAt` (Z. 139-142); 3d.1 ergänzt `pick`, `hidden_flag` und `TRIGGER_VAR_FIELDS`. `workbench/recipes.ts`: `RECIPE_BAR` mit `reason: COMES_WITH_3D5` an drei Einträgen (Z. 22-26), `NEXT` (Z. 30-38), `KNOWN` (Z. 41), `recipeRequest` (Z. 54-58); `WizardRequest` kennt `area`, `room`, `clearWorld`, `publish` (`WorkbenchContext.tsx:28`); `draft.ts:72` „a `look` gets its sprite later (3d.5)“; `DSScreen` (`DSScreen.tsx:13-30`: `variant` textbox/sign/empty, `pages`, `page`), `textModel.pages(text)` (Z. 86), `MessageEditor` (`onCommit: (text) => Promise<boolean>`); `useEvents(file, onError, revision)` (`useEvents.ts:10`); `trainerScript.ts` `TRAINER_SCRIPT` 3000, `trainerScript(id)`, `DEFAULT_SIGHT` 3; `api.ts` `editTextArchive` (Z. 889), `TextEdit` (Z. 874), `addEvent` (Z. 371).
- **`g4emu`:** Schritte `expect-map (1,2)`, `expect-battle (2,3)`, `walk-until-battle (2,3)`, `walk-to`, `press`, `mash`, `shot`, `walk-until-blocked`, `expect-at` (`steps.py:19-38`); `_walk_until_battle` (Z. 319-) geht Kachel für Kachel, bis ein Gang blockiert und die Kampfsignatur steht (gemacht für Gras); der Sichtkampf eines Trainers braucht nach dem Erkennen Tastendrücke durch seinen Text (`pt-trainer-begegnung.yaml`: `walk right 1`, `mash a 240`, `expect-battle`). `scenario.py:60`: mit `mcp: [flags]` sind die `edits` Werkzeugaufrufe einer `studio mcp`-Sitzung. Die Trainer-Szenarien liefern Vorlage, Kacheln und Treppe je Spiel (Platin Vorlage 342, Karten 5 und 6, Ankunft Tür (44, 21), Spieler tritt nach (45, 21); HeartGold Vorlage 33, Karte 1, Tür (24, 16), Spieler nach (23, 16), Treppe Eventdatei 61 Warp 0).
- **„Wilde Pokémon hier“** ordnet die Entscheidungen-Tabelle nicht 3d.5 zu (Zeile 3d.5: `dialog_of`, Dialog-Editor, `Target::Message`, drei Rezepte); Gutachten A §3.3 nennt es „vorhanden als „Begegnungsdatei anlegen…“, Erzählen-Layout neu“, die Zeile 3d.8 „Begegnungen „Wer hier wartet““. Es gehört zu 3d.8 und kommt hier nicht vor.

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Nachricht einer neuen Figur | nummeriert: `apply` hängt die Nachricht in **einer** `Store::transaction` an das Archiv `text_archive_id` des Headers (`Tx::edit_text_archive`, Index = bisherige Zahl) und legt das Objekt-Event an (`Tx::edit_event`, `local_id` frei wie `add_event_impl`); das Script nennt die Nummer (`Message {n}` / `NPCMsg {n}`); Literale nur in den Vorlagen von 3g.2 | Befund rotom-Literale: Dedup nach Text und neuer Index bei Änderung; der Dialog-Editor bearbeitet so immer die Nachricht im Archiv, ohne die Quelle anzufassen; `plan` kennt den Index vorab |
| Script einer neuen Figur | ein neuer Slot `#n` (n = höchster deklarierter Slot + 1, Refusal ab 2000) in der Scriptdatei `script_file_id` des Headers, Text je Familie aus `metadata/recipes/figure_{pt,hgss}.rotom` mit Platzhaltern `{n}`, `{local_id}`, `{message}`: Pt `script figur_{local_id} #{n}:` / `LockAll` / `FacePlayer` / `Message {message}` / `WaitButton` / `CloseMessage` / `ReleaseAll` / `End`; HGSS dasselbe mit `NPCMsg {message}` und `CloseMsg`; geschrieben über `append_source`, danach `script_file_step(file, &before)` und je berührtem Archiv `archive_file_step`; `squash_since(mark, label)` | Spezifikation §3.3 „je Spielfamilie“; dieselben Bausteine wie `empty_region.rs` (3d.2) |
| Voraussetzungen von „Neue Figur“ | Scripts übernommen (`is_adopted`, sonst `Refused(ADOPT_FIRST)`), die Scriptdatei des Headers ist eine rotom-Quelle (`SourceKind::Script`), Archiv und Eventdatei schreibbar (`check_writable`), Kachel frei von einem Objekt derselben Datei (sonst `Refused("auf x, z steht schon Objekt k")`), `text` nicht leer, Flag (falls gesetzt) `in_save` | `plan` läuft dieselben Prüfungen bis vor das erste Schreiben, wie bei Raum |
| Sichtbarkeits-Flag | `NewFigureInput.hidden_flag: Option<u16>`; „aus dem Pool“ löst der Aufrufer über `free_flag` (Picker-Fuß „Freies Flag nehmen“ aus 3d.1, kopflos das Werkzeug `free_flag`), nicht das Rezept | `pool::free_flag` braucht `RefIndex::usage` (Sitzung, nicht Store); ein Index im Rezept kostete `plan` 0,3 bis 1 s je Tastendruck |
| „Tür verbinden“ | `recipes/connect_doors.rs`: `ConnectDoorsInput { a: DoorEnd, b: DoorEnd, back: bool, door_tiles: bool }`, `DoorEnd { header, warp: Option<u16>, tile: Option<(x, z)> }` (genau eins von beiden); ein vorhandener Warp wird umgezielt, eine Kachel bekommt einen neuen Warp (Index = `warp_count`), `door_tiles` setzt Türkacheln wie `plan_entrance` (nur, wenn die Kachel noch keine Tür ist); `back: false` zielt nur A auf B; eine Transaktion `Tür „{A}“ ↔ „{B}“ verbunden` über ein oder zwei `edit_event` wie `connect` | Entscheidung „`connect_doors` aus dem Code von „Raum anlegen“, Warp-Paar als ein Undo-Schritt über zwei Eventdateien“ |
| Tür-Helfer | `recipes/doors.rs` (`pub(crate)`): `warp_count`, `warp_at`, `header_event_file`, `outside_map`, `fires`, `set_door`, `local`, `warp` ziehen 1 : 1 aus `room.rs` um, `room.rs` importiert sie; Verhalten und Tests unverändert | zwei Rezepte, ein Code; `room.rs` ist Datei von 3d.2 auf main, die Umzugsdiff ist mechanisch |
| Entwurf von „Tür verbinden“ | `events` je Ende (`Warp` mit `target` auf das andere Ende, `matrix` = Matrix des Headers über `outside_map`), `tiles` je neuer Türkachel; die Schau zeigt das Ende der gezeigten Matrix, die Weltkarte des Ziel-Headers das fremde (`WorldMapPanel` zeigt schon Zellen mit `matrix` der gezeigten; Kacheln und Events zeichnet die Schau) | Gutachten A §3.3 Punkt 3 |
| „Neuer Trainer hier“ | `recipes/new_trainer_here.rs`: `NewTrainerHereInput { header, x, z, dir, sight (1–15, Standard 3), class, name, party: Vec<{species, level}> (1–6), texts: { before, lose, after } }`; `apply`: `create_trainer` (Member und Sperre wie heute, `created` nennt den Trainer), dann `mark = undo_depth()`, Team über `Tx::edit_trainer` (`newMon`-Regel des Kernels: `TrainerDoc` Partyformat des Spiels, keine Attacken, keine Items), Kampftexte über `set_trainer_texts_labelled`, Objekt-Event mit `script = 3000 + id − 1`, `trainer_type: 1`, `sight`, `dir`, `graphics_id` (Eingabe, Standard aus `metadata/recipes.yaml` je Familie: das Aussehen der Trainerklasse kennt der Kernel nicht), alles ab `mark` mit `squash_since(mark, "Trainer „{name}“ hier aufgestellt")` zu **einem** Schritt hinter der Sperre; `is_irreversible` → `Some("Trainer anlegen")` | Befund Trainer anlegen: die Sperre bleibt (Entscheidungen „kein Versprechen“); alles Übrige ist ein Schritt |
| Kein Script mit `StartTrainerBattle` | der Trainer kämpft über `3000 + ID − 1` und die Sichtlinie der Engine; `StartTrainerBattle`/`TrainerBattle` kommen erst mit den Story-Rezepten (3d.7, Kampf als Schritt) | Befund Befehlsdialekte: ein eigenes Script bräche den Sichtkampf; der Gate-Text verlangt den Kampf aus dem Gehen |
| `dialog_of` | `dialog::dialog_of(store, header, file, kind, index) -> Dialog { source: DialogSource, messages: Vec<DialogMessage> }`; `DialogSource::{Local { file, slot, line }, Common { script }, Trainer { id }, None, Unreadable(String) }`; `DialogMessage { archive, index, text: Option<String>, command, line, col, branch: Option<String>, shared_with: u32 }`; Reihenfolge wie im Script; Sprünge: ein Befehl aus `script.jump_params` je Familie (neue Tabelle in `check_limits.yaml`: Pt `GoTo`, `Call`, `GoToIf`, `CallIf`, `GoToIfSet`, `GoToIfUnset`; HGSS `Jump`, `Call`, `JumpIf`, `CallIf`, `JumpIfSet`, `JumpIfUnset`; ein Test prüft jeden Namen gegen die Datenbank) mit einem Bezeichner-Argument, das eine Funktion derselben Datei nennt, wird einmal verfolgt (Menge besuchter Labels, Rekursion bricht ab); `branch` ist der Text der Bedingung des `if`/`elseif`, in dem die Nachricht steht (`None` außerhalb); `shared_with` = Zahl der Objekte (alle Eventdateien der Header, die das Archiv nutzen), deren Dialog dieselbe (`archive`, `index`) enthält, **ohne** diese Figur; gemeinsame und Trainer-Scripts nur `source`, keine Nachrichten | Entscheidung „`dialog_of(header, event)` auf Basis von `MessageUse` mit Verfolgung von `GoTo`/`Call` in der Datei“; 738/701 Figuren springen |
| `split_message` | `dialog::split_message(store, header, file, index, archive, message) -> SplitReport { new_index, change }`: eine Transaktion (Kopie der Nachricht ans Archiv) plus Dateischritt (in der Quelle jede Stelle **dieses Dialogs** mit `(archive, index)` an `line:col` auf `new_index` umgeschrieben, `append_source`-gleich geschrieben und kompiliert, `script_file_step`), `squash_since(mark, "Dialog von Figur k abgespalten")`; Refusal, wenn `shared_with == 0` | Entscheidung „Für diese Figur abspalten (neue Nachricht, Script zeigt darauf)“; der Editor bearbeitet danach die neue Nachricht |
| Dialog-Editor | Modal `DialogEditor` (`workbench/dialog/DialogEditor.tsx`, `[data-dialog-editor]`) aus dem Feld „Dialog“: links die Nachrichten als Liste mit `MessageEditor` je Nachricht (Commit über `editTextArchive(archive, { Set })`, Store-Label „Dialog geändert“ über ein neues Argument `label` von `edit_text_archive`), rechts `DSScreen` mit den Seiten der gewählten Nachricht (`pages(text)`, Blättern); eine Nachricht mit `shared_with > 0` trägt `Badge` „Geteilt mit N Figuren“ und die Taste „Für diese Figur abspalten“ (`splitMessage`); kein neues Hand-Panel | ein Modal braucht weder `handPanels` noch `dockRules` (Dateien von 3c/3d.2) noch den Atlas (3d.6); ⌘Z gilt über den Store wie in Texte |
| Feld „Dialog“ | `DialogSection` im `EventEditor` nur in Erzählen, für `object` (Variante `textbox`) und `bg` (Variante `sign`, Titel „Text“): `DSScreen` der ersten Seite der ersten Nachricht, Zeile „N Nachrichten · M Seiten“, bei `Common`/`Trainer` der Satz „Gemeinsames Script 2xxx: in Bauen bearbeiten.“ bzw. „Kampftexte des Trainers: im Bereich Pokémon.“, Taste „Dialog bearbeiten…“ (bei `None` „Dialog anlegen…“: ruft `add_script` + Script-Nummer, kommt mit 3d.7, hier gesperrt mit Tooltip „Kommt mit 3d.7“); der Satz `LATER.bg` fällt weg | Spezifikation §3.3 (Figur: Dialog mit DS-Vorschau; Schild: Text); die Felder von Bauen bleiben unberührt |
| Tür im Inspector | `WarpField` bekommt die Zeile „Gegenstück“: liest die Warps der Ziel-Eventdatei (hat es schon) und sagt „Tür k in {Ort} · führt zurück“ (Ziel-Warp zeigt auf diesen Header und diesen Warp) oder „führt nicht zurück“; darunter die Taste „Tür verbinden…“ (`wizards.open({ kind: "connect_doors", prefill: { a: { header, warp } } })`) | Gutachten A §3.4 |
| `Target::Message` | `Target::Message { archive: u16, index: u16 }` (angehängt), Quelle Scripts in `data/refs/messages.rs` (`script_message_refs(file, facts, archive)`: je `MessageUse` ein `Ref` mit `field` = Befehl, `slot` = Zeile, `access` Read), Archiv der Datei wie `ScriptMessageMissing`; `TargetDto::Message { archive, index }`; in `TextHand` unter der Nachricht „Verwendet in“ mit `Reference`-Chips (Fläche `welt`, Weg `script` mit Zeile) über `useRefs` | Entscheidung „`Target::Message` für den Rückweg“; `shared_with` nutzt den Index **nicht** (es zählt Figuren, der Index Zeilen) |
| Rezepte in der Oberfläche | drei Assistenten im Scharnier wie „Raum anlegen“ (`WizardRequest` `new_figure`, `connect_doors`, `new_trainer_here` mit `prefill`), Schritte: Figur „Wo und wie“ (Header = Auswahl, Kachel x/z als `NumberField`, Aussehen über `FigureLookField`, Bewegung `MOVEMENT_WORDS`, Blickrichtung), „Dialog“ (`TextField` mehrzeilig mit `DSScreen`-Vorschau, Sichtbarkeits-Flag über `FlagField`), „Zusammenfassung“ (`DraftNotes`); Tür „Tür A“ (vorhandene Tür des Orts per `WarpField`-Picker oder Kachel), „Tür B“ (Ort per `HeaderPicker`, Tür oder Kachel, „Rückweg anlegen“, „Türkacheln setzen“), „Zusammenfassung“; Trainer „Trainer“ (Klasse `classOptions`, Name ≤ 8 Zeichen), „Team“ (1 bis 6 Zeilen Art per `Picker` über `name_tables`, Level 1–100), „Texte“ (vor, verloren, nach; vor endet auf `\r`), „Wo“ (Kachel, Blick, Sichtweite), „Zusammenfassung“; jeder über `useDraftPlan` und `recipeApply`; das Ergebnis zeigt `outcome.next` | Muster 3d.2 (`NewRoomWizard`, `useDraftPlan`, `WizardResult`); Screen 9, 6, 20 |
| `NextStep` nach 3d.5 | Figur → `edit_dialog` „Dialog bearbeiten“ (frei; die App öffnet den Dialog-Editor der neuen Figur, kopflos nur ein Name) und `story_join` „Zu einer Geschichte“ („Kommt mit 3d.7“); Tür → `play_here` „Hier spielen“ („Kommt mit 3d.9“); Trainer → `play_here`; `next_steps`: Standard-Grund wird „Kommt mit 3d.7“, `play_here` „Kommt mit 3d.9“, `edit_dialog` immer `available` (kein Rezept); Raum und Gebiet nennen weiter `new_figure`/`connect_doors`, jetzt frei | Gutachten A §3.3 Rezepttabelle |
| Szenarien | ein Szenario je Spiel, `pt-erzaehlen.yaml` (CPUD) und `hg-erzaehlen.yaml` (IPKD), `mcp: [--allow-irreversible, --expert]`: Patch, `adopt_scripts`, neues Gebiet aus der Vorlage mit neuer Eventdatei, neue Scriptdatei und **neues Textarchiv** am Header, dann `recipe_apply new_figure`, `recipe_apply connect_doors` (Treppe des Zimmers ↔ neue Tür mit Türkacheln), `recipe_apply new_trainer_here`, `save`, Build, Spiel: Treppe → `expect-map` neu → Figur `press a` → `shot figur` → Tür zurück `expect-map 415` (HG 64) → wieder hinunter `expect-map` neu → Trainer `walk right 1`, `mash a 240`, `expect-battle S L` | drei Szenarien je Spiel wiederholten Patch, Übernahme, Gebiet und Build (je ≈ 80 s) dreimal; ein Lauf prüft zudem, dass die Rezepte nebeneinander bestehen; die Gate-Datei nennt je Rezept den Schritt |
| Gated Kernel-Tests | `crates/g4-kernel/tests/recipes_erzaehlen.rs` (neu, nicht `tests/recipes.rs`): synthetisch (`TextFixture`, Scripts über `scripts::adopt` wie `tests/templates.rs:466`) und gated auf `vanilla_copy()` je Spiel, mit `eprintln!`-Zahlen | eigene Datei: kein Konflikt mit 3d.4 an `tests/recipes.rs` |
| Nicht in 3d.5 | „Wilde Pokémon hier“ (3d.8), „Dialog anlegen“ für eine Figur ohne Script (3d.7 „Gespräch schaltet weiter“), Geschichten, Löschen einer Figur mit ihrem Slot (3d.4), Erzählen-Layout der Texte nach Figur (3d.8, nutzt `dialog_of` je Ort), `StartTrainerBattle` in Rezepten (3d.7), Live-Vorschau mit Kampfsprite im Trainer-Assistenten (3d.8; hier die `PokemonCard`-Zeile des Teams) | Entscheidungen „Phasenschnitt“ und „Nicht in 3d“ |

## Global Constraints

- Arbeitsort ist der Worktree `$HOME/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-5` auf dem Branch `phase-3d-5-figur-dialog-tuer` (Basis main nach dem Merge von 3d.1): nie `cd` in die Hauptkopie, nie den Branch wechseln, nie nach main mergen, kein `git stash`. `tools/` ist dort ein Symlink (nie einchecken).
- Vor jedem cargo-Aufruf `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`; `pnpm install` einmal in `apps/studio`.
- Commits enden mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`).
- Originale unter `$HOME/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur unter `Roms/_testkopien/3d-5-<zweck>` (`cp -cR` für Ordner, `cp -c` für `.nds`, Ordner `3d-5-<zweck>_DSPRE_contents` und `3d-5-<zweck>.nds` direkt nebeneinander); `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms` absolut. Gated Tests lesen `G4_TEST_PROJECT` und schreiben nur auf `vanilla_copy()`. Keine ROM-Daten ins Repo (keine `.nds`, keine Member, keine exportierte JSON eines Spiels; Fixtures synthetisch).
- Test-Umgebung nur auf `G4_VITE_PORT=1750 G4_BRIDGE_PORT=1751` (`G4_ROMS_DIR=<absolut> G4_VITE_PORT=1750 G4_BRIDGE_PORT=1751 bash scripts/agent/studio-web.sh`); andere Ports nie anfassen.
- UI nur aus `apps/studio/src/ui/primitives` und Tokens `--g4-*` (keine Hex-Farben, keine Inline-Styles außer `/* dyn */`); `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `bash scripts/check-depth.sh` grün; deutsche Oberfläche, Bezeichner englisch; höchstens eine primäre Taste je Dialog.
- Anhängepunkte nur anhängen: `apps/studio/src-tauri/src/commands/dispatch.rs` (`COMMANDS`, `match`, Beispielargumente), `src-tauri/src/lib.rs` (`generate_handler!`), `mcp/access.rs` (`COMMAND_KINDS`), `mcp/catalog.rs` (`ToolDef`s am Ende), `mcp/tools.rs` (`NOT_IN_BATCH`), `crates/g4-kernel/src/lib.rs`, `crates/g4-kernel/src/recipes.rs` (Varianten im Verteiler), `recipes/types.rs` (`RecipeId`-Varianten am Ende, `next_steps`-Gründe), `crates/g4-kernel/src/data/refs.rs` (eine Variante, ein Aufruf), `commands/data/dto.rs` (`TargetDto`), `apps/studio/src/api.ts`, `apps/studio/src/recipes/api.ts` (`RecipeId`-Union), `workbench/recipes.ts`, `WorkbenchContext.tsx` (`WizardRequest`), `app/WizardHost.tsx`, `metadata/check_limits.yaml` (neuer Schlüssel), `docs/mcp.md`, `docs/agent-gate.md`, `docs/ROADMAP.md`, `design-system/README.md`.
- Parallel laufen 3d.4 (`textproject/*` Schema `deleted`, `data/refs.rs` Welt-Ziele, Warp-Löschen in `commands/events.rs`, Listen und `InspectorHead.tsx`, `diff/*`, MCP `plan_delete`/`delete_entity`/`restore_entity`) und 3d.6 Teil A (Kernel `stories`, Atlas und Bereich „Geschichten“, `App.tsx` `LOCKED_AREAS`, Graph): **diese Dateien in 3d.5 nicht umbauen.** Konfliktstellen, an denen 3d.5 nur anhängt: `data/refs.rs` (3d.4 Ziele, 3d.6 schwache Quelle), `commands/data/dto.rs` (`TargetDto`, `NavTargetDto`), `commands/events.rs` (3d.4 Warp-Löschen: 3d.5 ändert dort nichts, Objekte legt das Rezept über `Tx::edit_event` im Kernel an), `InspectorHead.tsx` (3d.4 Plakette: 3d.5 fasst den Kopf nicht an), `EventEditor.tsx` (3d.5 nur der `DialogSection`-Aufruf im Erzählen-Zweig und der Wegfall von `LATER.bg`), `dispatch.rs`, `lib.rs`, `access.rs`, `catalog.rs`, `tools.rs`, `api.ts`, `ROADMAP.md`, `README.md`, `mcp.md`, `agent-gate.md`. `tests/recipes.rs` und `tests/refs.rs` bleiben unverändert (eigene Testdateien).
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen `cd apps/studio && pnpm test && pnpm build` und die drei Prüfskripte; bei `scripts/g4emu` `cd scripts/g4emu && uv run pytest -m "not integration" -q`.
- Nicht in 3d.5 (Entscheidungen, wörtlich): „Wilde Pokémon hier“ und die Erzählen-Layouts (3d.8), Geschichten und die Story-Rezepte (3d.6, 3d.7), Löschen (3d.4), Modell (b) nie.
- Die Szenarien `pt-trainer-begegnung.yaml`, `pt-raum-assistent.yaml`, `pt-flag.yaml`, `hg-flag.yaml` bleiben wörtlich unverändert und grün.

## Review Focus

- Eine Figur, deren Script eine Nachricht aus einem Sprungziel zeigt, das zwei Scripts der Datei teilen (`Call hilfe` aus Figur 3 und Figur 7): beide Dialoge nennen die Nachricht, `shared_with` ist 1 je Figur, Abspalten aus Figur 3 schreibt nur die Nummer in `hilfe` um, wenn `hilfe` **nur** aus Figur 3 erreichbar ist; sonst lehnt es ab mit „Die Nachricht steht in einem gemeinsamen Sprungziel; in Bauen im Script trennen.“ (Task 1 `a_shared_jump_target_counts_both_figures`, Task 2 `split_refuses_a_message_in_a_shared_label`).
- Eine Nachricht mit demselben Text in zwei Nachrichten des Archivs (Vanilla-Dubletten, uxie dedupliziert Literale): der Dialog-Editor zeigt und ändert nur die Nachricht mit **dem** Index, den das Script nennt, nicht die gleichlautende (Task 1 `dialog_names_the_index_not_the_text`).
- „Tür verbinden“ mit Tür A = Tür B oder mit einer Kachel, auf der schon ein Warp liegt: `plan` lehnt ab („Tür A und Tür B sind dieselbe Tür“, „auf x, z liegt schon Tür k; wähle sie als Tür“), nichts wird geschrieben; `back: false` lässt den Warp von B unverändert (Task 4 `the_same_door_twice_and_a_taken_tile_are_refused`, `without_back_only_a_points_to_b`).
- „Neuer Trainer hier“ scheitert nach `create_trainer` (gesperrte Eventdatei): der Trainer liegt als Member vor (`Partial` mit `created_text`), der Verlauf hat die Sperre „Trainer N angelegt“ und **keinen** halben Schritt danach; ein zweiter Aufruf nimmt die nächste ID (Task 5 `a_failed_placement_leaves_the_trainer_and_no_half_step`).
- Nach `recipe_apply new_figure`, ⌘Z, ⌘⇧Z: Quelle, Member, `compile-state.json`, Archiv-JSON und -Member byte-gleich je Stand; eine Quelle, die jemand zwischendurch von Hand änderte, lässt ⌘Z mit `Refused` ab (Task 3 `undo_and_redo_of_a_figure_restore_every_file`, `undo_refuses_a_source_changed_meanwhile`).

---

### Task 1: Kernel `dialog_of` mit Sprungverfolgung und Zähler „geteilt“

**Files:**
- Create: `crates/g4-kernel/src/dialog.rs` (Modul-Doku, Typen, `dialog_of`, Tests im Modul mit `TextFixture`), `crates/g4-kernel/tests/dialog.rs` (gated)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod dialog;` am Ende), `crates/g4-kernel/metadata/check_limits.yaml` (neuer Schlüssel `script.jump_params` je Familie, angehängt), `crates/g4-kernel/src/checks/limits.rs` (Leser des Schlüssels, Muster `message_params`), `crates/g4-kernel/src/checks/scripts.rs` (nur: `MessageUse`, `ScriptSlots`, `facts_of`, `ScriptInput` werden `pub(crate)` bleiben, dazu `pub(crate) fn message_uses_of(statements, …)` als Hülle um `walk`, die den Slot-Rumpf **eines** Scripts liest; `walk` selbst unverändert), `docs/formats/script_file.md` (Abschnitt „Dialog einer Figur“)

**Interfaces:**
- Consumes: `checks::scripts::{MessageUse, database, facts_of, ScriptInput}` (`checks/scripts.rs:54-107`), `checks::limits::Limits::message_params(family)`, `scripts::{resolve_script, ScriptTarget, parse_source, rotom_source, source_kind, SourceKind, FIRST_COMMON_SCRIPT}`, `Store::{text_archive, text_archive_users, header_table, event}`, `MapHeader::{script_file_id, text_archive_id}`, `rotom::compiler::ast::{Statement, StatementKind, ExpressionKind}`, `trainerScript`-Regel aus `scripts::FIRST_TRAINER_SCRIPT` (3000, `trainer_script_meta`).
- Produces:

```rust
// dialog.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case", tag = "kind")]
pub enum DialogSource {
    /// Script `#slot` of the header's script file, at its 1-based line.
    Local { file: u16, slot: u16, line: u32 },
    /// A common script (2000 and up): read in Bauen.
    Common { script: u16 },
    /// A trainer's battle (3000 to 6999): its texts live in the Pokémon area.
    Trainer { id: u16 },
    /// Script 0: the figure says nothing.
    None,
    /// The source cannot be read (not adopted, binary, parse error): the reason.
    Unreadable { reason: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DialogMessage { pub archive: u16, pub index: u16, /// `None` when the archive has no message at `index` (script.message-missing).
                           pub text: Option<String>, pub command: String, pub line: u32, pub col: u32,
                           /// The condition of the `if`/`elseif` the message stands in, as written; `None` outside.
                           pub branch: Option<String>, /// How many other figures (objects of every header using the archive) show the same message.
                           pub shared_with: u32 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dialog { pub source: DialogSource, pub messages: Vec<DialogMessage>, /// Labels the walk followed (`GoTo`, `Call`), in order.
                    pub followed: Vec<String> }
/// The dialog of event `index` of class `kind` (`Object` or `Bg`) in event file `file`, looked up on header `header`
/// (its script file and text archive), see the decisions: jumps inside the file are followed once each.
pub fn dialog_of(store: &mut Store, header: u16, file: u16, kind: DialogKind, index: u16) -> Result<Dialog, KernelError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)] #[serde(rename_all = "snake_case")]
pub enum DialogKind { Object, Bg }
/// The (archive, index) pairs script `#slot` of `file` shows, jumps followed, without the counter: what `shared_with` and the reference index read.
pub(crate) fn messages_of_slot(project: &Project, family: Family, file: u16, slot: u32, archive: u16) -> Result<Vec<DialogMessage>, KernelError>;
// checks/limits.rs
impl Limits { /// The commands whose first identifier argument names a label of the same file (`script.jump_params`). pub fn jump_commands(&self, family: Family) -> &[String]; }
```

- `check_limits.yaml` (angehängt unter `script:`): `jump_params: { pt: [GoTo, Call, GoToIf, CallIf, GoToIfSet, GoToIfUnset], hgss: [Jump, Call, JumpIf, CallIf, JumpIfSet, JumpIfUnset], dp: [GoTo, Call, GoToIf, CallIf] }` mit Kopfkommentar (Quelle: die Datenbanken, `legacy_name`); ein Name, den die Datenbank der Familie nicht kennt (`database(family).get_command`), ist ein Testfehler (`jump_params_name_existing_commands`), die Liste folgt dann der Datenbank.
- `dialog_of`: `script` des Events (`ObjectEvent.script`, `BgEvent.script`) → 0 `None`; 3000–6999 `Trainer { id: script − 3000 + 1 }` (5000er wie `trainerOfScript`); ≥ 2000 `Common`; sonst `resolve_script(project, headers, header, script)` → `Local { file, slot, line }`, `messages_of_slot(...)`, `shared_with` über alle Objekte (nicht BG) der Eventdateien aller `text_archive_users(archive)`-Header (je Datei einmal gelesen, `messages_of_slot` je Script einmal, `BTreeMap<(u16, u32), …>` als Zwischenspeicher), gleiche `(archive, index)` → +1, die eigene Figur abgezogen.

- [ ] **Step 1: Scheiternde Tests** (`dialog.rs` `mod tests`, `TextFixture::exported()` plus `scripts::adopt`, Quelle je Test geschrieben mit `write_source`; Platin, ein HGSS-Fall):
  - `a_figure_with_one_message_names_archive_index_and_line` (`script figur #2: … Message 5 …` → `Local { file, slot: 2, line }`, eine `DialogMessage { archive: text_archive_id, index: 5, text: Some(…), command: "Message", branch: None, shared_with: 0 }`).
  - `jumps_in_the_file_are_followed_once` (`#2` ruft `Call hilfe`, `hilfe:` zeigt `Message 7` und `GoTo hilfe` → Nachrichten 7 genau einmal, `followed == ["hilfe"]`).
  - `a_shared_jump_target_counts_both_figures` (Review Focus: Objekt 3 `#3` und Objekt 7 `#7` rufen `hilfe` → beide `shared_with == 1` für Nachricht 7).
  - `dialog_names_the_index_not_the_text` (Review Focus: Archiv mit `["Hallo", "Hallo"]`, Script zeigt 1 → `index == 1`, `shared_with == 0` trotz gleichem Text an 0).
  - `branches_carry_their_condition` (`if FLAG_UNUSED_0x0094 then Message 1 else Message 2 endif` → `branch` `Some("FLAG_UNUSED_0x0094")` und `Some("else")`).
  - `common_trainer_and_none_scripts_have_no_messages` (Script 2004 → `Common { script: 2004 }`; 3005 → `Trainer { id: 6 }`; 0 → `None`; je `messages.is_empty()`).
  - `a_binary_script_file_is_unreadable_with_a_reason` (ohne `adopt` → `Unreadable`, Text beginnt mit `ADOPT_FIRST`).
  - `hgss_counts_npcmsg_and_jump` (Familie Hgss über `synthetic_hgss_project`: `NPCMsg 2`, `Jump ende`).
  - `limits.rs`: `jump_params_name_existing_commands`.
  - gated `tests/dialog.rs` `vanilla_dialogs_of_a_town_resolve` (`vanilla_copy()`, `scripts::adopt`; für jedes Objekt des Start-Orts (Platin Zweiblattdorf 411, HGSS Neuborkia 60) `dialog_of`: keine `Unreadable`, jede `Local`-Nachricht mit `text.is_some()`, mindestens eine Figur mit `shared_with ≥ 1` auf Platin (Gutachten A: 66 geteilte Nachrichten); druckt `dialog <game_code> objects=<n> local=<n> messages=<n> shared=<n> ms=<n>`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features dialog:: limits::` → FAIL (Modul fehlt).
- [ ] **Step 3: Umsetzen** wie unter Interfaces; der Rumpf eines Scripts ist die `StatementKind::Function` mit dem `#slot` in `headers` (`slots_in`), Labels sind `Function`s ohne `id` mit Namen; Bedingungen werden über den Quelltext des `if`-Ausdrucks (`span` → `text[start..end]`) als `branch` getragen.
- [ ] **Step 4: Lauf** → PASS; gated einmal auf Platin DE (`G4_TEST_PROJECT=<Roms>/_testkopien/3d-5-pt_DSPRE_contents cargo test --release -p g4-kernel --all-features --test dialog -- --nocapture`); `cargo clippy`, `cargo fmt --check`.
- [ ] **Step 5: Doku** `docs/formats/script_file.md` „Dialog einer Figur“: Weg Script → Slot → Nachrichten, Sprungliste, Zähler, was Gemeinsames und Trainer-Scripts bedeuten.
- [ ] **Step 6: Commit** `feat(kernel): dialog_of with jump following, branch and shared-with counter, jump commands per family as metadata` (mit Trailer).

### Task 2: `split_message`, `Target::Message` im Index, `edit_text_archive` mit Label

**Files:**
- Create: `crates/g4-kernel/src/data/refs/messages.rs` (+ Tests im Modul)
- Modify: `crates/g4-kernel/src/dialog.rs` (`split_message`, `SplitReport`), `crates/g4-kernel/src/data/refs.rs` (nur: Variante `Target::Message { archive, index }` am Ende des Enums, `pub mod messages;`, ein Aufruf `messages::script_message_refs` dort, wo `script_refs` die Datei liest), `crates/g4-kernel/src/scripts.rs` (`append_source` → `pub(crate) fn write_and_compile(project, file, text) -> Result<Vec<u16>>` daneben: schreibt `text` ganz, kompiliert, stellt bei Fehler den alten Text her; `append_source` ruft es), `apps/studio/src-tauri/src/commands/data/dto.rs` (`TargetDto::Message { archive, index }` angehängt, `From` in `Target`), `apps/studio/src-tauri/src/commands/data/refs.rs` (Label und Weg: „Zeile 12 · Message“, `source_name` „Scriptdatei 0101“, `nav` `script` mit Zeile), `apps/studio/src-tauri/src/commands/text.rs` (`edit_text_archive_impl` nimmt `label: Option<String>`, Standard wie heute), `apps/studio/src-tauri/src/commands/dispatch.rs` (Argument `label` optional), `crates/g4-kernel/tests/refs_messages.rs` (gated), `docs/formats/text_archive.md` (Abschnitt „Abspalten“)

**Interfaces:**
- Consumes: Task 1 (`messages_of_slot`, `dialog_of`), `scripts::{snapshot_script, script_file_step, rotom_source, position_of}`, `Store::{transaction, record_file_step, squash_since, undo_depth, reload_from_disk}`, `Tx::edit_text_archive`, `RefIndex`-Bausteine aus `refs.rs` (`Ref`, `Access::Read`, `ScriptNames`), `checks::scripts::facts_of`.
- Produces:

```rust
// dialog.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SplitReport { pub archive: u16, pub old_index: u16, pub new_index: u16, pub change: Option<ChangeId>, pub lines: Vec<u32> }
/// Gives figure (`file`, `index`) its own copy of message (`archive`, `message`): the copy appended to the archive and every use of the
/// message in this figure's dialog (its slot and the labels only it reaches) rewritten to the new index, as one undo step
/// „Dialog von Figur {local_id} abgespalten“. Refused when the message is not shared (`NOT_SHARED`), when a use sits in a label another
/// script of the file also reaches (`SHARED_LABEL`), when the archive is full or locked.
pub fn split_message(store: &mut Store, header: u16, file: u16, index: u16, archive: u16, message: u16) -> Result<SplitReport, KernelError>;
pub const NOT_SHARED: &str = "Die Nachricht gehört nur dieser Figur.";
pub const SHARED_LABEL: &str = "Die Nachricht steht in einem gemeinsamen Sprungziel; in Bauen im Script trennen.";
// data/refs.rs
pub enum Target { /* … Flag(u16), Var(u16), */ /// Message `index` of text archive `archive`. Message { archive: u16, index: u16 } }
// data/refs/messages.rs
/// The message references of one parsed source: a `Ref` per `MessageUse` (field the command, slot the line, `Access::Read`),
/// `archive` the file's archive (the first header naming it); `None` archive: nothing.
pub(crate) fn script_message_refs(file: u16, uses: &[MessageUse], archive: Option<u16>) -> Vec<Ref>;
// commands/text.rs
pub fn edit_text_archive_impl(state: &AppState, index: u16, edit: TextEdit, label: Option<String>) -> Result<Option<ChangeId>, CmdError>;
```

- Ablauf `split_message`: `dialog_of` → Treffer mit `(archive, message)` und ihren `line:col`; Rumpf-Labels, die ein anderes Script der Datei erreicht (Menge `followed` je Script), sperren mit `SHARED_LABEL`; `mark = undo_depth()`; `before = snapshot_script(file)`; `transaction("Dialog von Figur {local_id} abgespalten", |tx| tx.edit_text_archive(archive, push Kopie))` → `new_index`; Quelle: an jeder Stelle die Zahl ersetzen (von hinten nach vorn, damit Spalten stimmen), `write_and_compile`, `record_file_step(label, script_file_step(file, &before))`, je `touched_archives` `archive_file_step`; `squash_since(mark, label)`. Bei Compile-Fehler: Transaktion schon im Verlauf → `undo()` einmal, dann der Fehler (kein halber Schritt).
- Index-Quelle: `script_refs` in `refs.rs` kennt die Datei und ihre Facts; `archive` = `text_archive_id` des ersten Headers mit `script_file_id == file` (die Header-Tabelle hat `script_refs` über `ScriptNames`/Store; sonst `None`).

- [ ] **Step 1: Scheiternde Tests:**
  - `dialog.rs`: `split_copies_the_message_and_rewrites_this_figures_uses_as_one_step` (zwei Figuren zeigen 5; nach `split_message` Figur 3: Archiv +1, Figur 3 zeigt `new_index`, Figur 7 zeigt 5, `shared_with` beider 0, `undo_depth` + 1, `undo()` stellt Quelle, Member und Archiv byte-gleich her, `redo()` zurück), `split_refuses_a_message_in_a_shared_label` (Review Focus), `split_refuses_an_unshared_message` (`NOT_SHARED`), `a_compile_error_leaves_no_half_step` (Quelle vorher mit einem absichtlichen Fehler in einem anderen Script → `Compile`, `undo_depth` unverändert, Archiv wie vorher).
  - `refs/messages.rs`: `message_uses_become_read_refs_with_line_and_command`, `a_file_without_header_has_no_message_refs`; `tests/refs_messages.rs` gated `vanilla_message_refs_count_the_script_messages` (`refs(Target::Message { archive: text_archive_id(411), index })` einer bekannten Nachricht ≥ 1; Gesamtzahl der Message-Verweise ≈ 5 000 ± 20 % je Spiel (Gutachten A: 4 999 / 4 407 nummerierte); druckt `refs <game_code> messages=<n> warm_ms=<n>`).
  - `commands/text.rs`: `edit_text_archive_takes_a_label` (Verlauf nennt „Dialog geändert“); `dispatch.rs` Beispielargument um `"label": null`.
  - `data/refs.rs` bestehende Tests grün (`tests/refs.rs` unverändert).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features dialog:: refs` und `cargo test -p studio --all-features --lib text` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces.
- [ ] **Step 4: Lauf** → PASS; gated einmal auf Platin DE; `vanilla_index_builds_under_a_second` (`tests/refs.rs`) bleibt grün (die Message-Verweise kommen in denselben Durchlauf wie die Flags).
- [ ] **Step 5: Doku** `text_archive.md` „Abspalten“; `docs/formats/text_project.md` nur, falls dort der Index beschrieben ist (Zeile zu `Target::Message`).
- [ ] **Step 6: Commit** `feat(kernel): split_message as one undo step, Target::Message from scripts in the reference index, edit_text_archive with a label` (mit Trailer).

### Task 3: Rezept „Neue Figur“ im Kernel

**Files:**
- Create: `crates/g4-kernel/src/recipes/new_figure.rs`, `crates/g4-kernel/metadata/recipes/figure_pt.rotom`, `figure_hgss.rotom`, `crates/g4-kernel/metadata/recipes.yaml` (`default_look` je Familie für Figur und Trainerklasse-Fallback, Kopfkommentar mit Quelle: die Werte aus den Szenarien, Platin Figur 6, Wanderer 20; HGSS aus `hg-flag.yaml`), `crates/g4-kernel/tests/recipes_erzaehlen.rs`
- Modify: `crates/g4-kernel/src/recipes.rs` (`pub mod new_figure; pub mod events;`, Verteiler `plan`/`apply`, `is_irreversible` → `None`), `crates/g4-kernel/src/recipes/types.rs` (`RecipeId::NewFigure` am Ende, `title` „Neue Figur“, `all`; `next_steps`: Standard-Grund „Kommt mit 3d.7“, `play_here` → „Kommt mit 3d.9“, `edit_dialog` → `available: true`; die `next_steps`-Tabellen von `room.rs` und `area.rs` bleiben, ihre Tasten werden über `list()` frei), `apps/studio/src-tauri/src/commands/events.rs` (nur: `next_free_local_id` ruft den Kernel-Helfer, eine Zeile), neu `docs/formats/recipes.md` (Abschnitt „Neue Figur“)

**Interfaces:**
- Consumes: `scripts::{is_adopted, rotom_source, declared_slots, append_source, snapshot_script, script_file_step, snapshot_archive, archive_file_step, source_kind, FIRST_COMMON_SCRIPT}`, `Store::{transaction, record_file_step, squash_since, undo_depth, event, text_archive, header_table, reload_from_disk, change_generation, changes_since}`, `Tx::{edit_event, edit_text_archive}`, `recipes::types::{Draft, DraftEvent, DraftEventKind, DraftMessage, Outcome, next_steps, check_header_id, check_writable, refused}`, `flags::ranges::in_save` (3d.1), `trainer_create::ADOPT_FIRST`; die Regel `next_free_local_id` aus `commands/events.rs` zieht als Kernel-Helfer nach `recipes/events.rs` (neu, Muster `doors.rs`).
- Produces:

```rust
// recipes/new_figure.rs
#[derive(Debug, Clone, Deserialize)]
pub struct NewFigureInput { pub header: u16, pub x: u16, pub z: u16, pub graphics_id: Option<u16>, pub movement_type: u16, pub dir: i16,
                            pub text: String, pub hidden_flag: Option<u16> }
#[derive(Debug, Clone, Serialize)]
pub struct NewFigureReport { pub event_file: u16, pub object: u16, pub local_id: u16, pub script_file: u16, pub slot: u16, pub archive: u16, pub message: u16 }
pub fn plan(store: &mut Store, input: &NewFigureInput) -> Result<Draft, KernelError>;
pub fn apply(store: &mut Store, input: &NewFigureInput) -> Result<Outcome<NewFigureReport>, KernelError>;
/// The script text of a figure for `family` (`metadata/recipes/figure_{pt,hgss}.rotom`, Dp like Pt): `{n}`, `{local_id}`, `{message}` filled.
pub fn figure_script(family: Family, n: u16, local_id: u16, message: u16) -> String;
// recipes/events.rs
pub fn next_free_local_id(objects: &[ObjectEvent]) -> Result<u16, KernelError>;
```

- `plan`: Prüfungen aus „Entscheidungen“ (Voraussetzungen), dann `events = [DraftEvent { kind: Object, matrix: Some(matrix des Headers über doors::outside_map), x, z, look: Some(graphics_id oder default_look), target: None }]`, `messages = [DraftMessage { archive: Some(text_archive_id), text }]`, `notes` („Stellt Figur mit Script #{n} und Nachricht {m} in Archiv {a} auf.“, bei Flag „Sichtbar, solange Flag 0x…. 0 ist.“), `irreversible` leer.
- `apply`: `mark = undo_depth()`, `before_script = snapshot_script(file)`, `before_archive = snapshot_archive(archive)`; `transaction("Figur „{text gekürzt auf 20 Zeichen}“ aufgestellt", |tx| edit_text_archive push + edit_event push ObjectEvent { local_id, graphics_id, movement_type, dir, script: n, hidden_flag, trainer_type 0, data [0;3], x, z, y 0 })`; dann `append_source(project, file, text, &figure_script(...))` (bei Fehler `undo()` der Transaktion, Fehler durch), `record_file_step(label, script_file_step(file, &before_script))`, je `touched` (sollte leer sein, nummeriert) `archive_file_step`, **nicht** das Archiv der Transaktion (es ist Store-Teil); `squash_since(mark, label)`; `next = next_steps(&[("edit_dialog", "Dialog bearbeiten"), ("story_join", "Zu einer Geschichte")])`.

- [ ] **Step 1: Scheiternde Tests** (`tests/recipes_erzaehlen.rs`; synthetisch `TextFixture::exported()` + `scripts::adopt` wie `tests/templates.rs:462-466`, Header mit `script_file_id` auf eine rotom-Quelle und `text_archive_id` auf ein Archiv):
  - `list_names_five_recipes_with_new_figure_last`.
  - `plan_of_a_figure_writes_nothing_and_names_event_message_and_slot` (`events.len() == 1` mit `look`, `messages[0].text == input.text`, `notes` nennt `#n`; danach `undo_depth`, `dirty()`, Dateiliste unter `expanded/` unverändert).
  - `apply_of_a_figure_is_one_undo_step_with_a_numbered_message` (Objekt da mit `script == n`, Archiv +1 mit genau `input.text`, Quelle enthält `Message {m}` (Pt) und keinen Literal, Member neu; `undo_depth` + 1; Label).
  - `undo_and_redo_of_a_figure_restore_every_file` (Review Focus).
  - `undo_refuses_a_source_changed_meanwhile` (Review Focus).
  - `hgss_writes_npcmsg_and_closemsg` (`figure_script(Hgss, …)` enthält `NPCMsg 3` und `CloseMsg`, kein `Message`).
  - `a_figure_on_a_taken_tile_and_an_empty_text_are_refused`, `a_figure_needs_adopted_scripts` (`ADOPT_FIRST`), `a_slot_from_2000_on_is_refused`.
  - `next_steps_after_3d5_lock_with_3d7_and_3d9` (`story_join` „Kommt mit 3d.7“, `play_here` „Kommt mit 3d.9“, `edit_dialog` frei, `new_figure`/`connect_doors`/`new_trainer_here` frei sobald in `list()`; `types.rs`-Test `next_steps_lock_what_is_not_there_yet` anpassen).
  - Gated: `vanilla_new_figure_message_equals_the_input` (je Spiel: Start-Ort (411 / 60), Kachel neben der Tür des Zimmers wie die Szenarien, `text` „Gate 3d.5“, `apply`, dann `store.text_archive(archive).messages[m].text == "Gate 3d.5"`, Quelle enthält den Slot; `undo` → `git`-gleicher Vergleich der Dateien unter `expanded/scripts/`, `expanded/textArchives/`, `unpacked/textArchives/` byte-gleich; druckt `figure <game_code> slot=<n> message=<m> apply_ms=<n> undo_ms=<n>`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features --test recipes_erzaehlen` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features --test recipes --test templates` (unverändert grün); gated einmal auf Platin DE und HeartGold DE; `cargo test -p studio --all-features --lib` (Kataloge und `access` sehen das fünfte Rezept: `recipe_args` liest `list()`; `the_curated_tool_count…` unverändert, keine neuen Werkzeuge).
- [ ] **Step 5: Doku** `docs/formats/recipes.md` (neu, Kopf wie `templates.md`): Rahmen kurz, „Neue Figur“ mit Eingabe, Script je Familie, Nachricht nummeriert (Befund rotom-Literale), Undo-Schritt; Platzhalter-Überschriften „Tür verbinden“, „Neuer Trainer hier“.
- [ ] **Step 6: Commit** `feat(recipes): new_figure with a numbered message in the header's archive and a script slot per family, one undo step` (mit Trailer).

### Task 4: Rezept „Tür verbinden“ im Kernel, Tür-Helfer nach `doors.rs`

**Files:**
- Create: `crates/g4-kernel/src/recipes/doors.rs`, `crates/g4-kernel/src/recipes/connect_doors.rs`
- Modify: `crates/g4-kernel/src/recipes/room.rs` (die acht Helfer raus, `use super::doors::*`; sonst nichts), `crates/g4-kernel/src/recipes.rs` (`pub mod doors; pub mod connect_doors;`, Verteiler), `crates/g4-kernel/src/recipes/types.rs` (`RecipeId::ConnectDoors` „Tür verbinden“), `crates/g4-kernel/tests/recipes_erzaehlen.rs`, `docs/formats/recipes.md` („Tür verbinden“)

**Interfaces:**
- Consumes: `doors::{warp_count, warp_at, header_event_file, outside_map, fires, set_door, local, warp}` (Signaturen wie heute in `room.rs:76-319`), `tiles::{DOOR, DOOR_COLLISION, warp_fires}`, `Tx::{edit_event, edit_map}`, `Store::{transaction, header_table, event, matrix, map}`, `recipes::types::{Draft, DraftEvent, DraftTarget, DraftTile, Outcome, next_steps}`, `places`-Namen für die Labels (`HeaderSummary`-Name über `store.header_table()` und `internal_names`: wie `room.rs` den Namen des Außen-Headers nennt).
- Produces:

```rust
// recipes/connect_doors.rs
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct DoorEnd { pub header: u16, #[serde(default)] pub warp: Option<u16>, #[serde(default)] pub tile: Option<Tile> }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)] pub struct Tile { pub x: u16, pub z: u16 }
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectDoorsInput { pub a: DoorEnd, pub b: DoorEnd, #[serde(default = "yes")] pub back: bool, #[serde(default = "yes")] pub door_tiles: bool }
#[derive(Debug, Clone, Serialize)]
pub struct DoorReport { pub header: u16, pub event_file: u16, pub warp: u16, pub created: bool, pub door_tile: Option<DoorTile> }
#[derive(Debug, Clone, Serialize)] pub struct ConnectDoorsReport { pub a: DoorReport, pub b: DoorReport, pub back: bool }
pub fn plan(store: &mut Store, input: &ConnectDoorsInput) -> Result<Draft, KernelError>;
pub fn apply(store: &mut Store, input: &ConnectDoorsInput) -> Result<Outcome<ConnectDoorsReport>, KernelError>;
```

- Prüfungen (`plan` und `apply`): je Ende genau eins von `warp`/`tile` (sonst `Refused("Tür A braucht eine Tür oder eine Kachel")`), Header vorhanden, `warp < warp_count`, Kachel ohne Warp (`warp_at` → `Refused("auf x, z liegt schon Tür k; wähle sie als Tür")`), beide Enden verschieden („Tür A und Tür B sind dieselbe Tür“), Eventdateien schreibbar; `door_tiles` nur für Kachel-Enden, deren Kachel nicht schon `warp_fires` (`fires`), Karte über `outside_map`.
- `plan`: `events` je Ende (`Warp`, `matrix` der Matrix des Headers, `x`/`z` (bei `warp` die des Warps), `target` auf das andere Ende: `warp` der andere Index oder, bei Kachel, `warp_count` des anderen), `tiles` je neuer Türkachel, `notes` („Tür A: {Ort} Tür k → Tür B: {Ort} Tür j, und zurück.“).
- `apply`: eine `transaction("Tür „{Ort A}“ ↔ „{Ort B}“ verbunden", …)`: wie `connect` je Datei ein `edit_event` (gleiche Datei: ein Aufruf; neue Warps `push`, vorhandene `dest_header_id`/`dest_warp_id` setzen; ohne `back` nur A), Türkacheln über `set_door`; `changed` aus `changes_since`; `next = next_steps(&[("play_here", "Hier spielen")])`.

- [ ] **Step 1: Scheiternde Tests:**
  - `tests/recipes.rs` unverändert grün nach dem Umzug der Helfer (vorab laufen lassen: das ist der Test des Umzugs).
  - `plan_of_two_doors_names_both_ends_and_the_door_tiles` (A vorhandener Warp 0 von Header 1, B Kachel auf Header 0 mit Türkachel → zwei Events mit gegenseitigen `target`, eine Kachel, nichts geschrieben).
  - `apply_connects_two_event_files_in_one_step` (beide Warps zeigen aufeinander, Türkachel gesetzt, `undo_depth` + 1, `undo()` stellt beide Eventdateien und die Karte byte-gleich her).
  - `a_shared_event_file_is_one_edit` (beide Header dieselbe Datei).
  - `the_same_door_twice_and_a_taken_tile_are_refused`, `without_back_only_a_points_to_b` (Review Focus).
  - Gated: `vanilla_doors_round_trip_in_the_town` (je Spiel: A = Treppe im Zimmer (Platin Eventdatei 394 Warp 0, HGSS 61 Warp 0), B = eine freie Kachel vor dem Haus im Start-Ort; `apply`, dann `get`-gleiche Prüfung: Warp A → (Start-Ort, Index B), Warp B → (Zimmer, 0); `undo` byte-gleich; druckt `doors <game_code> a=<file>:<i> b=<file>:<j> ms=<n>`).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS (`--test recipes --test recipes_erzaehlen`, `cargo test -p studio --all-features --lib wizards`), gated einmal je Spiel.
- [ ] **Step 5: Doku** `recipes.md` „Tür verbinden“.
- [ ] **Step 6: Commit** `feat(recipes): connect_doors as one undo step over two event files, door helpers shared with the room recipe` (mit Trailer).

### Task 5: Rezept „Neuer Trainer hier“ im Kernel

**Files:**
- Create: `crates/g4-kernel/src/recipes/new_trainer_here.rs`
- Modify: `crates/g4-kernel/src/recipes.rs` (`pub mod new_trainer_here;`, Verteiler, `is_irreversible` → `Some("Trainer anlegen")`), `crates/g4-kernel/src/recipes/types.rs` (`RecipeId::NewTrainerHere` „Neuer Trainer hier“), `crates/g4-kernel/src/data/trainer_create.rs` (nur: `pub fn create_trainer_report_name`? nein; `create_trainer` bleibt; `TrainerText` bekommt `pub const KIND_LOSE: u16 = 1; KIND_AFTER: u16 = 2;` neben `TEXT_BEFORE_BATTLE`), `crates/g4-kernel/metadata/recipes.yaml` (`default_look.trainer` je Familie), `crates/g4-kernel/tests/recipes_erzaehlen.rs`, `docs/formats/recipes.md`, `docs/mcp.md` (Zeile `recipe_apply`: unumkehrbar auch mit `new_trainer_here`)

**Interfaces:**
- Consumes: `trainer_create::{create_trainer, set_trainer_texts_labelled, TrainerText, TEXT_BEFORE_BATTLE, ADOPT_FIRST, before_battle_problem}`, `data::names::NameTables` (`create_trainer` braucht `&mut NameTables`: der Store hat sie nicht; **Entscheidung:** `recipes::apply` bekommt die Tabellen über `Store::name_tables()`? Prüfen, wie `commands/data/create.rs:27-35` sie hält (`session.names`); im Kernel nimmt das Rezept `NameTables::load(project)` neu, wie `g4k` es tut, und die Tauri-Hülle `recipe_apply_impl` ruft danach `session.names.reload()`-gleich, falls es das gibt, sonst `after_write` genügt, weil die Namenbank aus dem Store gelesen wird: der Implementierer prüft `NameTables::names(store, Bank::BattleTexts)` (`trainer_create.rs:395`) und wählt den Weg, der die 74 Trainer-Tests grün lässt), `Tx::{edit_trainer, edit_event}`, `TrainerDoc`-Partyformat (`newMon`-Regel: `data/trainer_rules.rs`), `doors::outside_map`, `recipes/events.rs::next_free_local_id`, `scripts::FIRST_TRAINER_SCRIPT`.
- Produces:

```rust
// recipes/new_trainer_here.rs
#[derive(Debug, Clone, Deserialize)] pub struct PartyMon { pub species: u16, pub level: u8 }
#[derive(Debug, Clone, Deserialize)] pub struct BattleTexts { pub before: String, pub lose: String, pub after: String }
#[derive(Debug, Clone, Deserialize)]
pub struct NewTrainerHereInput { pub header: u16, pub x: u16, pub z: u16, pub dir: i16, #[serde(default = "three")] pub sight: u16,
                                 pub class: u8, pub name: String, pub party: Vec<PartyMon>, pub texts: BattleTexts, pub graphics_id: Option<u16> }
#[derive(Debug, Clone, Serialize)]
pub struct NewTrainerHereReport { pub trainer: u16, pub placeholder: Option<u16>, pub script_extended: bool, pub event_file: u16, pub object: u16,
                                  pub script: u16, pub trainer_flag: u16, pub text_problems: Vec<Problem> }
pub fn plan(store: &mut Store, input: &NewTrainerHereInput) -> Result<Draft, KernelError>;
pub fn apply(store: &mut Store, input: &NewTrainerHereInput) -> Result<Outcome<NewTrainerHereReport>, KernelError>;
```

- Prüfungen (`plan` und `apply`, vor dem ersten Schreiben): `is_adopted`, Name ≤ 8 Einheiten (Text von `create_trainer_with`), `party` 1–6 mit `species < member_count(Personal)` und `level` 1–100, `sight` 1–15, Kachel frei, Eventdatei schreibbar, `texts.before` endet mit `\r` (sonst Warnung in `notes`, wie `before_battle_problem`). `plan`: `events = [Object mit look]`, `messages` = die drei Texte mit `archive: Some(Bank::BattleTexts)`, `notes` („Legt Trainer „{class} {name}“ als Nummer {id} an (nächste freie), Team {n} Pokémon, Trainer-Flag 0x{550+id:04X}.“), `irreversible = ["Trainer {id} wird angelegt (Undo-Sperre)"]`.
- `apply`: `create_trainer(...)` → `created` nennt `CreatedObject { kind: Trainer, index }` (und den Platzhalter); `mark = undo_depth()`; `transaction("Trainer „{name}“ hier aufgestellt", |tx| tx.edit_trainer(id, Team) ; tx.edit_event(file, push Objekt { script: 3000 + id − 1, trainer_type: 1, data[0] = sight, dir, graphics_id }))`; `set_trainer_texts_labelled(store, names, id, [before 0, lose 1, after 2], Some(label))`; `squash_since(mark, label)`; `next = next_steps(&[("play_here", "Hier spielen")])`.

- [ ] **Step 1: Scheiternde Tests** (synthetisch nach dem Muster `tests/trainer_create.rs` (Kampf-Script, Namenbank, Textbank); gated je Spiel):
  - `plan_of_a_trainer_names_the_next_id_the_flag_and_three_messages`.
  - `apply_creates_the_trainer_then_one_step_for_team_texts_and_object` (`history()` endet mit „Trainer N angelegt“ und dem Rezept-Label; `undo()` nimmt Team, Texte und Objekt zurück und hält an der Sperre; Objekt `script == 3000 + N − 1`, `trainer_type == 1`, `data[0] == sight`).
  - `a_failed_placement_leaves_the_trainer_and_no_half_step` (Review Focus).
  - `is_irreversible_names_trainer_anlegen`.
  - `the_seen_slot_is_skipped_like_create_trainer` (ID vor dem Sicht-Slot → `placeholder`).
  - Gated: `vanilla_trainer_here_in_the_town` (je Spiel: Klasse 9, „Jens“, Team [(25, 12)], Texte wie `pt-trainer-begegnung.yaml`, Kachel wie dort; `report.trainer` = 928 / 738; `g4k check-data`-gleich: `store.trainer(id)` Team stimmt; druckt `trainer <game_code> id=<n> ms=<n>`).
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, `cargo test -p g4-kernel --all-features --test trainer_create` grün, `cargo test -p studio --all-features --lib` (Trainer-Tests, `access.rs` `recipe_apply_is_irreversible_only_with_allow_patch_or_empty_region` um den Fall `new_trainer_here` erweitert), gated einmal je Spiel.
- [ ] **Step 5: Doku** `recipes.md` „Neuer Trainer hier“ (warum kein `StartTrainerBattle`), `mcp.md` Zeile `recipe_apply`.
- [ ] **Step 6: Commit** `feat(recipes): new_trainer_here: trainer with team and battle texts, object with battle script and sight line, one step behind the create barrier` (mit Trailer).

### Task 6: Befehle und MCP: `dialog_of`, `split_message`; `api.ts`; `g4k`

**Files:**
- Create: `apps/studio/src-tauri/src/commands/dialog.rs` (+ Tests), `apps/studio/src/api.dialog.ts` (+ Test)
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `lib.rs` (Handler anhängen), `commands/dispatch.rs` (`COMMANDS`, `match`, Beispielargumente: `("dialog_of", json!({"header": 1, "eventFile": 0, "kind": "object", "index": 0}))`, `("split_message", json!({"header": 1, "eventFile": 0, "index": 0, "archive": 3, "message": 5}))`), `mcp/access.rs` (`("dialog_of", Read)`, `("split_message", Write)`), `mcp/catalog.rs` (zwei `ToolDef` am Ende), `mcp/tools.rs` (`split_message` in `NOT_IN_BATCH`: schreibt Scripts als Dateischritt), `apps/studio/src/api.ts` (`export * from "./api.dialog"`; `editTextArchive(index, edit, label?)`), `apps/studio/src/recipes/api.ts` (`RecipeId` um `new_figure`, `connect_doors`, `new_trainer_here`; Typen `NewFigureInput`, `ConnectDoorsInput`, `NewTrainerHereInput` und die Berichte), `crates/g4-kernel/src/bin/g4k.rs` (nur, falls `g4k recipe plan/apply` die Eingabe nicht schon als JSON durchreicht: nichts; `g4k recipe list` zeigt sieben), `crates/g4-kernel/tests/cli.rs` (`cli_recipe_list` zählt sieben Zeilen), `docs/mcp.md` (Tabelle: `dialog_of`, `split_message`; `recipe_list` nennt sieben Kennungen; Beispiel-Prompt „Stell eine Figur …“)

**Interfaces:**
- Consumes: Task 1, 2 (`dialog::{dialog_of, split_message, Dialog, DialogKind, SplitReport}`), Task 3–5 (Rezepte über `recipes::*`).
- Produces:

```rust
// commands/dialog.rs
pub fn dialog_of_impl(state: &AppState, header: u16, event_file: u16, kind: DialogKind, index: u16) -> Result<Dialog, CmdError>;   // Lock, Session
pub fn split_message_impl(state: &AppState, header: u16, event_file: u16, index: u16, archive: u16, message: u16) -> Result<SplitReport, CmdError>; // writable, after_write, files_written(&[])
```

```ts
// api.dialog.ts
export type DialogSource = { kind: "local"; file: number; slot: number; line: number } | { kind: "common"; script: number } | { kind: "trainer"; id: number } | { kind: "none" } | { kind: "unreadable"; reason: string };
export interface DialogMessage { archive: number; index: number; text: string | null; command: string; line: number; col: number; branch: string | null; shared_with: number }
export interface Dialog { source: DialogSource; messages: DialogMessage[]; followed: string[] }
export const dialogOf: (header: number, eventFile: number, kind: "object" | "bg", index: number) => Promise<Dialog>;
export const splitMessage: (header: number, eventFile: number, index: number, archive: number, message: number) => Promise<{ archive: number; old_index: number; new_index: number; change: number | null; lines: number[] }>;
```

- Katalog (englisch): `dialog_of` „The dialog of a figure or sign“: „The messages the script of object or sign `index` of event file `eventFile` shows on header `header`, in order, jumps inside the script file followed: archive, index, text, command, line, the branch condition, and how many other figures show the same message (shared_with). source says whether it is a local script, a common script (2000 and up, read only here), a trainer's battle (its texts are trainer texts) or nothing. “ + `data_note!()`; `split_message` „Give a figure its own copy of a shared message“: „Appends a copy of message (archive, message) to the archive and rewrites every use in this figure's dialog to the new index, as one undo step (script file written and compiled, a file step). Refused when the message is not shared or sits in a label other scripts of the file reach. Not in g4_batch.“
- `recipes/api.ts`: `RecipeId = … | "new_figure" | "connect_doors" | "new_trainer_here"`, die drei Eingaben als TS-Spiegel der Rust-Structs (snake_case), `NewFigureReport`, `ConnectDoorsReport`, `NewTrainerHereReport`.

- [ ] **Step 1: Scheiternde Tests:** `dispatch.rs` Beispielargumente der zwei Befehle; `commands/dialog.rs` `dialog_of_over_the_command_path` (synthetisch mit übernommenen Scripts), `split_message_is_one_undo_step_over_the_command_path`; `access.rs` die zwei Zeilen; `catalog.rs` `the_curated_set_is_the_decided_one` (lesend `dialog_of`, schreibend `split_message`), `read_tools_on_texts_and_scripts_say_data_is_not_instructions` mit `dialog_of`, Werkzeugzahl (reißt `the_curated_tool_count…`, Grenze mit Begründung heben); `tools.rs` `split_message_is_never_in_a_batch`; `tests/cli.rs` `cli_recipe_list` sieben; `api.dialog.test.ts` `dialog_commands_send_the_documented_arguments`; `recipes/api.test.ts` `recipe_ids_name_seven`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS (`cargo test -p studio --all-features --lib`, `cargo test -p studio --test mcp_stdio`, `cargo test -p g4-kernel --test cli`, `pnpm test`).
- [ ] **Step 5: Doku** `docs/mcp.md` wie unter Files.
- [ ] **Step 6: Commit** `feat(mcp): dialog_of and split_message over the bridge and MCP; the three recipes in recipe_list, api.ts and g4k` (mit Trailer).

### Task 7: Feld „Dialog“, Dialog-Editor, Plakette „Geteilt“, Schild-Text, Rückweg in Texte

**Files:**
- Create: `apps/studio/src/workbench/inspector/DialogSection.tsx` (+ `.module.css`, Test), `apps/studio/src/workbench/dialog/DialogEditor.tsx` (+ `.module.css`, Test), `apps/studio/src/workbench/dialog/useDialog.ts` (+ Test), `apps/studio/src/workbench/dialog/dialogModel.ts` (+ Test: `pageCount(messages)`, `dialogSummary(dialog)`, `sharedText(n)`)
- Modify: `apps/studio/src/workbench/inspector/EventEditor.tsx` (im Erzählen-Zweig für `object` und `bg` `<DialogSection …/>` nach den Feldern; `LATER` verliert `bg`; `EventEditorContext` bekommt `openDialogEditor: (target: DialogTarget) => void`), `EventEditor.test.tsx`, `apps/studio/src/App.tsx` (Zustand `dialogEditor: DialogTarget | null`, rendert `DialogEditor`; `revision` nach Commit), `apps/studio/src/workbench/text/TextHand.tsx` (+ Test: unter der gewählten Nachricht „Verwendet in“ über `useRefs(…, { Message: { archive, index } }, ["script"])` mit `Reference`-Chips, Klick `goTo({ kind: "script", file, line })`), `apps/studio/src/data/useRefs.ts` (nur, falls `TargetDto` dort als Typ gespiegelt ist: `api.data.ts` `TargetDto` um `Message`), `apps/studio/src/workbench/inspector/inspectorFields.test.ts` (Snapshot: die Feldlisten ändern sich nicht; der Abschnitt steht außerhalb der Liste, ein Test `erzaehlen_renders_the_dialog_section_for_object_and_bg` in `EventEditor.test.tsx`)

**Interfaces:**
- Consumes: Task 6 (`dialogOf`, `splitMessage`, `editTextArchive` mit Label), `DSScreen`, `pages`/`toTextForm` (`workbench/text/textModel.ts`), `MessageEditor`, `Badge`, `Button`, `Dialog` (Modal), `Notice`, `Reference`, `Tooltip`, `useRefs`.
- Produces:

```ts
// workbench/dialog/dialogModel.ts
export interface DialogTarget { header: number; eventFile: number; kind: "object" | "bg"; index: number; /** The figure's name in the head: „Objekt 3“ / „Schild 0“. */ title: string }
export const sharedText = (n: number) => `Geteilt mit ${n} ${n === 1 ? "Figur" : "Figuren"}`;
export function dialogSummary(d: Dialog): string;   // „2 Nachrichten · 3 Seiten“, „Sagt nichts“, „Gemeinsames Script 2004: in Bauen bearbeiten.“, „Kampftexte des Trainers: im Bereich Pokémon.“, „Script nicht lesbar: …“
// workbench/dialog/useDialog.ts
export function useDialog(target: DialogTarget | null, revision: number): { dialog: Dialog | null; error: string | null; reload: () => void };
// workbench/inspector/DialogSection.tsx
export function DialogSection(props: { target: DialogTarget; revision: number; onEdit: (t: DialogTarget) => void; onError: (e: unknown) => void }): JSX.Element;
// <section data-dialog-section> Titel „Dialog“ (bg: „Text“), DSScreen variant textbox|sign der ersten Seite, Zeile dialogSummary, Taste „Dialog bearbeiten…“ (quiet S; bei none „Dialog anlegen…“ gesperrt, Tooltip „Kommt mit 3d.7“; bei common/trainer/unreadable keine Taste)
// workbench/dialog/DialogEditor.tsx
export function DialogEditor(props: { target: DialogTarget; revision: number; onClose: () => void; onChanged: () => void; onError: (e: unknown) => void }): JSX.Element;
// Modal [data-dialog-editor] Titel „Dialog von {title}“; links je Nachricht ein Block [data-dialog-message="<archive>:<index>"] mit Zeile „Nachricht {index} · Archiv {archive}“ (bei branch „wenn {branch}“), MessageEditor (onCommit → editTextArchive(archive, { Set: { index, text } }, "Dialog geändert") → true; refused → Notice), bei shared_with > 0 Badge (Warnung) sharedText und Taste „Für diese Figur abspalten“ (splitMessage, danach reload und Meldung „Nachricht {new} ist jetzt nur diese Figur.“); rechts DSScreen der gewählten Nachricht mit Seitenwahl „Seite k von n“ (Tasten ‹ ›); Fuß „Schließen“; Esc schließt
```

- Der Editor merkt sich keinen Text: nach jedem Commit `reload()`; ⌘Z in der App (Store `undo`) erhöht `revision` wie bei Texte, der Editor liest neu.

- [ ] **Step 1: Scheiternde Tests:** `dialogModel.test.ts` `summary_counts_messages_and_pages_and_words_the_other_sources`, `shared_text_singular_and_plural`; `useDialog.test.tsx` `reads_on_target_and_revision_and_drops_an_older_answer`; `DialogSection.test.tsx` `shows_the_first_page_summary_and_the_edit_button`, `a_sign_uses_the_sign_variant_and_the_title_text`, `common_and_trainer_sources_show_the_sentence_without_a_button`, `none_locks_anlegen_with_3d7`; `DialogEditor.test.tsx` `edits_one_message_through_edit_text_archive_with_the_label`, `a_shared_message_shows_the_badge_and_splits` (Fake `splitMessage`, danach Meldung), `a_refused_text_shows_the_notice`, `esc_closes`; `EventEditor.test.tsx` `erzaehlen_renders_the_dialog_section_for_object_and_bg` und der bestehende Test zu „Der Text des Schilds kommt mit 3d.5.“ wird umgedreht (Satz weg); `TextHand.test.tsx` `a_message_lists_where_it_is_used`; `App.test.tsx` `edit_dialog_opens_the_editor_for_the_selected_figure`.
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** mit `frontend-design:frontend-design` für den Editor (zwei Spalten ab 760 px, darunter untereinander; Plakette in `status/warnung`; Tokens); **Step 4: Lauf** → PASS, `pnpm build`, drei Prüfskripte grün.
- [ ] **Step 5: Commit** `feat(inspector): dialog field with DS preview in Erzählen, dialog editor over the text store with the shared badge and splitting, sign text, where a message is used` (mit Trailer).

### Task 8: Die drei Assistenten, freie Rezept-Tasten, Tür-Gegenstück, Sprite im Entwurf

**Files:**
- Create: `apps/studio/src/workbench/wizards/NewFigureWizard.tsx` (+ `.module.css`, Test), `ConnectDoorsWizard.tsx` (+ Test), `NewTrainerHereWizard.tsx` (+ Test), `apps/studio/src/workbench/wizards/wizardSpecs.ts` (Ergänzung, falls vorhanden: `figureResult`, `doorsResult`, `trainerResult` → `DoneResult`-Zeilen)
- Modify: `apps/studio/src/workbench/recipes.ts` (`RECIPE_BAR` ohne `reason` an den drei Einträgen; `NEXT` um die drei Rezepte; `KNOWN` sieben; `recipeRequest` kennt `new_figure` (prefill `header`, Kachel aus `prefill.tile`), `connect_doors` (prefill `a`), `new_trainer_here`; `edit_dialog` → `null` plus Rückgabe `{ action: "edit_dialog" }`? **Entscheidung:** `recipeRequest` bleibt `WizardRequest | null`; `WizardHost.onStep("edit_dialog")` ruft `openDialogEditor` mit dem Ergebnis des Rezepts), `apps/studio/src/workbench/WorkbenchContext.tsx` (`WizardRequest` um `{ kind: "new_figure"; prefill?: { header?: number; tile?: { x: number; z: number } } } | { kind: "connect_doors"; prefill?: { a?: { header: number; warp?: number; tile?: { x: number; z: number } } } } | { kind: "new_trainer_here"; prefill?: { header?: number; tile?: … } }`), `apps/studio/src/app/WizardHost.tsx` (die drei Assistenten, `onStep` `edit_dialog`), `apps/studio/src/App.tsx` (`onRecipe` der Leiste öffnet sie mit der Auswahl: Header = gewählter Ort, Kachel = zuletzt gewählte Kachel der Szene, falls `SceneOverlays`/`WorkbenchLive` sie hält, sonst leer), `apps/studio/src/workbench/inspector/WarpField.tsx` (+ Test: Zeile „Gegenstück“, Taste „Tür verbinden…“ über eine neue Prop `onConnect?: () => void`), `EventEditor.tsx` (reicht `onConnect` aus dem Kontext: `openWizard(request)`), `apps/studio/src/world/scene/draft.ts` (+ Test: `look` → `Sprite` mit `spriteTexture(look)`/`peekSprite` aus `world/sprites.ts` bei 50 %, Platzhalter solange die Textur lädt), `apps/studio/src/ui/frame/RecipeBar.test.tsx` (`lists_five_recipes_all_runnable`), `docs/agent-gate.md` (Zeile „Welt, Reiter „3D““: die drei Assistenten, `data-*`)

**Interfaces:**
- Consumes: Task 6 (`recipePlan`, `recipeApply`, Eingabetypen), 3d.2 (`useDraftPlan`, `DraftNotes`, `WizardBar`, `WizardResult.DoneResult`, `HeaderChoice`, `HeaderPicker`, `FigureLookField`, `MOVEMENT_WORDS`, `FACINGS`, `WarpField`), 3d.1 (`FlagField` mit `kind: "flag"`), `classOptions`, `name_tables` (Arten), `PokemonCard`, `NumberField`, `TextField`, `Checkbox`, `SelectField`, `Picker`, `DSScreen`, `pages`.
- Produces:

```ts
// NewFigureWizard.tsx: Props { headers, selected: number | null, prefill?, depth, onDraft, onDone: (r: RecipeOutcome<NewFigureReport>) => void, onCancel, onError }
//   Schritte „Wo und wie“ → „Dialog“ → „Zusammenfassung“; [data-wizard="new_figure"]; Eingabe = NewFigureInput; useDraftPlan("new_figure", input)
//   Felder: „Ort“ (HeaderChoice, vorbelegt), „Kachel x“/„z“ (NumberField), „Aussehen“ (FigureLookField), „Bewegung“ (SelectField MOVEMENT_WORDS), „Blickrichtung“ (FACINGS),
//           „Was sie sagt“ (TextField mehrzeilig, Shift+Enter neue Seite, rechts DSScreen pages(text)), „Sichtbarkeits-Flag“ (FlagField, 0 = keins)
//   Ergebnis: „Figur aufgestellt“, Zeilen „Objekt k in Eventdatei NNNN“, „Script #n, Nachricht m in Archiv NNNN“, steps = outcome.next (edit_dialog frei, story_join gesperrt)
// ConnectDoorsWizard.tsx: Props wie oben mit prefill.a; Schritte „Tür A“ → „Tür B“ → „Zusammenfassung“; je Ende Segment „Vorhandene Tür | Neue Kachel“,
//   vorhandene über den Picker von WarpField (Einträge „Tür k · x, z“), Kachel x/z; Kästchen „Rückweg anlegen“ (an), „Türkacheln setzen“ (an); Entwurf beider Enden
// NewTrainerHereWizard.tsx: Schritte „Trainer“ (Klasse, Name) → „Team“ (1–6 Zeilen: Art per Picker über die Artnamen, Level NumberField 1–100; PokemonCard je Zeile) → „Texte“ (vor/verloren/nach, Hinweis „Der Text vor dem Kampf endet auf ▼“) → „Wo“ (Ort, Kachel, Blickrichtung, Sichtweite 1–15) → „Zusammenfassung“ (DraftNotes, Plakette „Trainer anlegen lässt sich nicht rückgängig machen“)
// WarpField: Props + { self?: { header: number; warp: number }; onConnect?: () => void }  → Zeile „Gegenstück: Tür k in {Ort} · führt zurück“ | „… führt nicht zurück“ | „Ziel hat keine Tür k“; Taste „Tür verbinden…“ [data-connect-doors]
// draft.ts: ein DraftEvent mit look zeichnet den Sprite der Grafik (spriteTexture), opacity DRAFT_OPACITY
```

- [ ] **Step 1: Scheiternde Tests:** `recipes.test.ts` `all_bar_recipes_run_and_next_knows_the_new_ones`, `recipe_request_opens_the_three_wizards_with_prefill`; `NewFigureWizard.test.tsx` `builds_the_input_shows_the_draft_and_applies` (Fake `recipePlan`/`recipeApply`; `onDraft` mit Entwurf, Esc → `null`; „Aufstellen“ → `onDone` mit `next`), `a_text_with_a_page_break_previews_two_pages`; `ConnectDoorsWizard.test.tsx` `two_ends_door_or_tile_with_back_and_door_tiles`, `prefill_a_from_the_selected_door`; `NewTrainerHereWizard.test.tsx` `team_rows_one_to_six_and_the_before_text_hint`, `summary_names_the_barrier`; `WarpField.test.tsx` `names_the_counterpart_and_offers_connect`; `draft.test.ts` `an_event_with_a_look_draws_its_sprite`; `RecipeBar.test.tsx` fünf frei; `App.test.tsx` `recipe_bar_opens_the_figure_wizard_with_the_selected_header`, `edit_dialog_after_a_figure_opens_the_editor`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen** (Assistenten nach `NewRoomWizard`, CSS nur Tokens); **Step 4: Lauf** → PASS, `pnpm build`, Prüfskripte grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (Kopie `_testkopien/3d-5-web` aus Platin DE mit `g4k git init`, Scripts übernommen; `G4_VITE_PORT=1750 G4_BRIDGE_PORT=1751`, Seite `http://127.0.0.1:1750/?project=<Kopie>`): Erzählen, Zweiblattdorf; „Neue Figur“ bis zur Zusammenfassung → `[data-draft-count]` = `draftCount(call.sh recipe_plan …, 0)` (1); Aufstellen → Ergebnis mit „Dialog bearbeiten“, Klick öffnet `[data-dialog-editor]`; Figur wählen → Feld „Dialog“; eine Vanilla-Figur mit geteilter Nachricht (Platin: aus `dialog <game_code> shared=` des gated Tests die Figur ablesen) → Plakette, Abspalten; `git -C <Kopie> status --porcelain` nach einem Dialog-Commit nennt nur `expanded/textArchives/NNNN.json` und `unpacked/textArchives/NNNN` desselben Archivs; ⌘Z → leer; „Tür verbinden“ und „Neuer Trainer hier“ je bis zum Ergebnis; Bilder `target/agent/3d-5/blick-{figur,dialog,dialog-geteilt,tuer,trainer}-{hell,dunkel,kompakt}.png` (Hell und Dunkel in derselben CDP-Verbindung, Hinweis H6 der 3d.2-Gate-Datei).
- [ ] **Step 6: Commit** `feat(wizards): new figure, connect doors and new trainer here in Erzählen with live draft, free recipe buttons, door counterpart, sprite in the draft` (mit Trailer).

### Task 9: Szenarien `pt-erzaehlen` und `hg-erzaehlen`

**Files:**
- Create: `scripts/agent/scenarios/pt-erzaehlen.yaml`, `scripts/agent/scenarios/hg-erzaehlen.yaml`
- Modify: `scripts/g4emu/tests/test_scenario.py` (nur, falls ein Test die Szenario-Dateien parst: die zwei dazu), `docs/agent-gate.md` („Szenarien über MCP“: die zwei Namen)

**Interfaces:**
- Consumes: Task 3–6 (die Rezepte über `recipe_apply`), die Werkzeuge `apply_header_patch`, `adopt_scripts`, `create_area`, `get_header`, `new_script_file`, `create_text_archive`, `set_header`, `save`, `build_rom`; Schritte `continue`, `expect-map`, `walk-to`, `walk`, `wait`, `press`, `mash`, `shot`, `expect-battle`, `expect-at`.
- Produces (Platin; HeartGold mit den Werten aus `hg-trainer-begegnung.yaml`: `game: IPKD`, Vorlage 33, Karte 1, Zimmer 64, Treppe Eventdatei 61 Warp 0, Tür (24, 16), Spieler (23, 16), Figur (23, 15), Trainer (20, 18) Blick oben, Sicht (20, 16), `NEUER_HEADER` 540):

```yaml
# Kopfkommentar wie pt-trainer-begegnung.yaml: Zweck (Gate 3d.5, drei Rezepte kopflos über MCP), Kacheln, Platzhalter
#   NEUER_HEADER (593), S_T1 L_T1 (25, 12), TEXT ("Hallo, Reisender!")
game: CPUD
mcp: [--allow-irreversible, --expert]
edits:
  - [apply_header_patch, {}]
  - [adopt_scripts, {}]
  - [create_area, {spec: {… wie pt-trainer-begegnung.yaml, Vorlage 342, Karten 5 und 6, header.id NEUER_HEADER, event_file New …}}, area]
  - [get_header, {id: "${area.header}"}, hdr]
  - [new_script_file, {kind: script}, datei]
  - [create_text_archive, {}, archiv]
  - [set_header, {id: "${area.header}", header: {Pt: {$base: "${hdr.Pt}", script_file_id: "${datei}", text_archive_id: "${archiv}"}}}]
  - [save, {}]
  - [recipe_apply, {recipe: new_figure, input: {header: "${area.header}", x: 45, z: 20, graphics_id: 6, movement_type: 15, dir: 1, text: "${TEXT}"}}, figur]
  - [recipe_apply, {recipe: connect_doors, input: {a: {header: 415, warp: 0}, b: {header: "${area.header}", tile: {x: 44, z: 21}}, back: true, door_tiles: true}}, tuer]
  - [recipe_apply, {recipe: new_trainer_here, input: {header: "${area.header}", x: 50, z: 23, dir: 0, sight: 3, class: 9, name: Jens, party: [{species: "${S_T1}", level: "${L_T1}"}], texts: {before: 'Halt! Ich bin Jens!\r', lose: 'Du bist stark!', after: 'Gut gekämpft.'}, graphics_id: 20}}, trainer]
  - [save, {}]
build: out.nds
check:
  - [g4k, check-data, out.nds, --expect, "trainer:${trainer.report.trainer}:party0=${S_T1}"]   # nur, wenn check-data Trainer kennt; sonst entfällt der Block
play:
  - continue
  - expect-map 415
  - walk-to 8 4
  - walk right 1
  - expect-map ${area.header}          # Tür verbinden, hin
  - wait 150
  - shot ankunft
  - press up:8                         # die Figur steht nördlich (45, 20)
  - press a
  - wait 90
  - shot figur                         # der Text „Hallo, Reisender!“ steht in der Box (Blick)
  - mash a 120
  - walk-to 44 21
  - expect-map 415                     # Tür verbinden, zurück
  - walk-to 8 4
  - walk right 1
  - expect-map ${area.header}
  - wait 150
  - walk-to 49 21
  - walk right 1                       # in die Sichtlinie von Jens
  - mash a 240
  - expect-battle ${S_T1} ${L_T1} 900  # Neuer Trainer hier
  - wait 300
  - shot trainer
  - win-battle
  - wait 120
  - press b
  - expect-map ${area.header}
```

- Der Schritt zur Figur: `press up:8` dreht den Spieler nach oben (wie im Trainer-Szenario zum Geschenk); `press a` öffnet die Box, `shot figur` vor dem `mash`. Die Kachel (45, 20) ist im Trainer-Szenario die des Geschenks: frei.

- [ ] **Step 1: Szenarien schreiben** (Kopfkommentar mit dem Beispielaufruf).
- [ ] **Step 2: Lauf** auf frischen Kopien `_testkopien/3d-5-szen-pt` (Platin DE) und `3d-5-szen-hg` (HeartGold DE), aus `scripts/g4emu`: `uv run g4emu scenario ../agent/scenarios/pt-erzaehlen.yaml --project $K/3d-5-szen-pt_DSPRE_contents --out ../../target/agent/3d-5/pt-erzaehlen --set NEUER_HEADER=593 --set S_T1=25 --set L_T1=12 --set TEXT="Hallo, Reisender!"` → `"ok": true`; `figur.png` zeigt den Text; HeartGold mit `NEUER_HEADER=540`. Scheitert ein Schritt an einer Kachel, die Kachel in der Test-Umgebung ablesen und den Kopfkommentar anpassen (Bilder nicht ins Repo).
- [ ] **Step 3: Bestand** `pt-trainer-begegnung.yaml` auf einer frischen Kopie `3d-5-trainer` (Werte aus dem Kopf) → `"ok": true` (Gate-Text), ebenso `pt-flag.yaml` auf `3d-5-flag` (3d.1 ist gemergt; prüft, dass `hidden_flag` und `free_flag` neben den Rezepten bestehen).
- [ ] **Step 4: Doku** `docs/agent-gate.md` „Szenarien über MCP“: `pt-erzaehlen`, `hg-erzaehlen` mit ihren `--set`.
- [ ] **Step 5: Commit** `feat(g4emu): scenarios pt-erzaehlen and hg-erzaehlen: figure, doors and trainer through the recipes, played` (mit Trailer).

### Task 10: Design-System fortschreiben

**Files:**
- Create: `design-system/openpencil/NACHTRAG-3d-5.md`, `design-system/openpencil/beispiele/figur-dialog-tuer.body.js`
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 3d.5“ nach „Nachtrag 4.5“; „Komponenten“: Zeile „Plakette“ um den Zustand „Geteilt“ (Warnung, Text „Geteilt mit N Figuren“) und „Dialog-Editor“; „Was die App umsetzt“: Rezept-Leiste mit fünf freien Tasten, Dialog-Feld, Dialog-Editor; „Abweichungen“: Live-Vorschau mit Kampfsprite im Trainer-Assistenten erst 3d.8, Texte als Dialoge nach Figur 3d.8), `design-system/Audit-Design-System.md` (Screens 6, 9, 14, 20: Stand), bei Antwort von OpenPencil `design-system/Editor_DesignSystem.fig`

**Interfaces:**
- Consumes: Texte und Zustände aus Task 7 und 8, die Bilder aus Task 8 Step 5.

- [ ] **Step 1: README** „Nachtrag 3d.5“ nach dem Muster „Nachtrag 3d.2“: Quelle (`workbench/inspector/DialogSection.tsx`, `workbench/dialog/{DialogEditor,useDialog,dialogModel}`, `workbench/wizards/{NewFigureWizard,ConnectDoorsWizard,NewTrainerHereWizard}.tsx`, `workbench/inspector/WarpField.tsx`, `workbench/recipes.ts`, `world/scene/draft.ts`), Plan- und Gate-Verweis, Bilder unter `target/agent/3d-5/`; keine neuen Tokens (Plakette „Geteilt“ auf `status/warnung`), keine neuen Symbole; Komponenten: „Feld Dialog“ (DS-Vorschau, Zusammenfassung, Taste; Zustände lokal, gemeinsam, Trainer, nichts, nicht lesbar), „Dialog-Editor“ (Liste mit Nachrichten-Feldern, Plakette „Geteilt mit N Figuren“ mit „Für diese Figur abspalten“, DS-Seiten rechts), „Assistent Neue Figur / Tür verbinden / Neuer Trainer hier“ (Schritte, Entwurf mit Sprite, Ergebnis mit „Dialog bearbeiten“ und „Hier spielen“ gesperrt „Kommt mit 3d.9“), „Tür-Gegenstück“ im Inspector; Abweichungen wie unter Files; die `.fig`: Satz „Die Frames von 3d.5 folgen, sobald OpenPencil antwortet (Anleitung `openpencil/NACHTRAG-3d-5.md`)“.
- [ ] **Step 2: Vorlage** `beispiele/figur-dialog-tuer.body.js` nach `erzaehlen-rahmen.body.js`, `MODE`: `'dialog'` (Frame „App: Welt, Erzählen, Dialog-Editor“: Inspector der Figur mit Feld „Dialog“, darüber das Modal mit zwei Nachrichten, eine mit Plakette „Geteilt mit 2 Figuren“), `'figur'` (Frame „App: Welt, Erzählen, Neue Figur“: Schritt „Dialog“ mit DS-Vorschau, Entwurf mit halbtransparentem Sprite in der Schau), `'brett'` (Brett „Komponenten (App): Plakette Geteilt, Feld Dialog, Tür-Gegenstück“).
- [ ] **Step 3: Anleitung** `NACHTRAG-3d-5.md` nach `NACHTRAG-3d-2.md` (Lage der Frames, Bauen, `fix_fig_text.mjs --rgb555`, die Hinweise aus dem Gedächtnis: 10 000-Knoten-Grenze, keine Einzeländerungen, Zeitüberschreitungen bei gesperrtem Bildschirm).
- [ ] **Step 4: `.fig`** nur, wenn OpenPencil antwortet; sonst bleibt der Satz aus Step 1.
- [ ] **Step 5: Audit** wie unter Files; `bash scripts/check-tokens.sh` grün.
- [ ] **Step 6: Commit** `docs(design-system): addendum 3d.5 dialog field and editor with the shared badge, the three recipe wizards` (mit Trailer).

### Task 11: Gate 3d.5, `agent-gate.md`, Roadmap, `mcp.md`

**Files:**
- Create: `docs/phase-3d-5-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben; Abschnitt „Phase 3d.5“ nach „Phase 4.5“ mit Kopien, `data-*`, Tabelle Punkt / Wer / Wie, Befehle für (1) bis (3)), `docs/ROADMAP.md` (Tabelle „Phase 3d“ Zeile 3d.5 mit Stand; Übersicht Zeile 3d; „Aus dem Design-System, später“: „Neuer Trainer hier“ 3d.5 umgesetzt), `docs/mcp.md` (Abschnitt „Beispiele (Prompts)“: Figur mit Text, Tür verbinden; „Unumkehrbares“: `recipe_apply` mit `new_trainer_here`), `docs/dev-setup.md` (gated `tests/dialog.rs`, `tests/refs_messages.rs`, `tests/recipes_erzaehlen.rs` in der Tabelle `G4_TEST_PROJECT`), `docs/formats/recipes.md` (Verweis auf die Gate-Datei)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben `dialog …`, `refs …`, `figure …`, `doors …`, `trainer …` der gated Tests; die Szenarien.

- [ ] **Step 1: Gate-Dokument** `docs/phase-3d-5-gate.md` nach dem Muster `docs/phase-3d-2-gate.md`: Kopf mit Stand, Branch, Worktree; Gate-Text der Spezifikation (§4 Zeile 3d.5) als Zitat; Legende „Wer“; Namen in der Oberfläche (`[data-recipe=new_figure|connect_doors|new_trainer_here]` frei, `[data-wizard=…]`, `[data-dialog-section]`, `[data-dialog-editor]`, `[data-dialog-message="a:i"]`, Plakette „Geteilt mit N Figuren“, „Für diese Figur abspalten“, Zeile „Gegenstück“, `[data-connect-doors]`, Ergebnis-Tasten „Dialog bearbeiten“ frei und „Hier spielen“ gesperrt „Kommt mit 3d.9“, Agentenfrage „Trainer anlegen“); Vorbereitung (Kopien `_testkopien/3d-5-{pt,hgde,hgus}` für (1), `3d-5-k{pt,hgde,hgus}` für (2) mit `g4k git init`, `3d-5-szen-{pt,hg}`, `3d-5-trainer`, `3d-5-flag` für die Szenarien, `3d-5-web` (Platin DE), `3d-5-webhg` (HeartGold DE), `3d-5-mcp` für (3), je mit `.nds` daneben; `cargo build --release` von `g4k`, `studio` und den Release-Testbinaries; Test-Umgebung auf 1750/1751; Last mit `uptime`).
  - **(1) Automatik:** `scripts/gate.sh` grün; einzeln je Projekt im Release-Profil mit `--nocapture --test-threads 1`: `--test dialog`, `--test refs_messages`, `--test recipes_erzaehlen` (gated: Nachricht im Archiv gleich der Eingabe, Türen hin und zurück, Trainer), dazu `--test recipes`, `--test templates`, `--test trainer_create` unverändert grün; `cd scripts/g4emu && uv run pytest -q`.
  - **(2) Szenario kopflos** (Release-`studio mcp --project <Kopie> --allow-irreversible --expert`, je Spiel, auch HeartGold US): `recipe_list` → sieben; `recipe_plan new_figure` → `events` 1 mit `look`, `messages` 1, `git status` leer; `recipe_apply new_figure` → `history` + 1, `report.message`, `get_text_archive` des Header-Archivs zeigt den Text an `report.message` **gleich der Eingabe**; `undo` → `git status` leer (Quelle, Member, `compile-state.json`, Archiv zurück); `redo`; `recipe_apply connect_doors` → ein Schritt, `get_events` beider Dateien zeigen aufeinander; `recipe_apply new_trainer_here` ohne `--allow-irreversible` → `needs_confirmation` „Trainer anlegen“, mit → `created` nennt den Trainer, `history` endet mit „Trainer N angelegt“ und dem Rezept-Label; `dialog_of` der neuen Figur → eine Nachricht, `shared_with` 0; `dialog_of` einer Vanilla-Figur mit `shared_with ≥ 1` (aus `dialog <game_code> shared=`), `split_message` → `new_index`, `git status` nennt die Quelle, den Member, `compile-state.json` und das eine Archiv (JSON und Member), `undo` → leer; die Szenarien `pt-erzaehlen`, `hg-erzaehlen` → `"ok": true`, `figur.png` zeigt den Text (Agent + Blick), `expect-map` zweimal, `expect-battle`; `pt-trainer-begegnung`, `pt-flag` grün.
  - **(3) Browser-Teil** (Test-Umgebung, `3d-5-web`, Chrome headless 1440 × 900): Erzählen, Rezept-Leiste mit fünf freien Tasten; „Neue Figur“ bis zur Zusammenfassung → `[data-draft-count]` gleich `draftCount(recipe_plan)`, Esc → 0, `git status` leer; Aufstellen → Ergebnis, „Dialog bearbeiten“ öffnet den Editor; Vanilla-Figur mit Dialog ändern (`MessageEditor`, Blur) → `git -C <Kopie> status --porcelain` nennt **nur** `expanded/textArchives/NNNN.json` und `unpacked/textArchives/NNNN` dieses Archivs; ⌘Z (Fokus in der Seite) → `call.sh dirty` leer, `git status` leer; Figur mit geteilter Nachricht → Plakette „Geteilt mit N Figuren“, „Für diese Figur abspalten“ → `git status` nennt Quelle, Member, `compile-state.json`, Archiv; ⌘Z → leer; Tür in Erzählen → Zeile „Gegenstück“, „Tür verbinden…“ öffnet den Assistenten mit Tür A vorbelegt; „Neuer Trainer hier“ bis zum Ergebnis (Agentenfrage bei MCP: `mcp.sh call recipe_apply '{"recipe":"new_trainer_here",…}'` → `[data-agent-question]` „Trainer anlegen (lässt sich nicht rückgängig machen)“); Konsole ohne Fehler; HeartGold DE derselbe Ablauf mit Neuborkia.
  - **(4) Hand und Blick beim Nutzer:** Blick auf Feld „Dialog“, Dialog-Editor (Hell, Dunkel, Kompakt), Entwurf mit Sprite, die drei Assistenten (Bilder aus Task 8 Step 5); Hand: `figur.png` der Szenarien lesen (der Text steht lesbar in der Box), die gepackte App mit einem Dialog-Commit.
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | `recipe_apply` Figur (ms) | `undo` Figur (ms) | `recipe_apply` Tür (ms) | `recipe_apply` Trainer (ms) | `dialog_of` Start-Ort (Objekte / geteilt / ms) | `split_message` (ms) | Szenario (s) | Last |
    |---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | | |
    | HeartGold DE | | | | | | | | |
    | HeartGold US | | | | | | | – | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Befunde beheben (eigene Commits `fix(3d.5): …` mit Regressionstest), bis alles grün ist; `G4_ROMS_DIR=$HOME/Projekte/lab/editor/Roms bash scripts/gate.sh > target/agent/gate-sh-3d-5.log 2>&1` im Hintergrund, allein (unter Last scheitern Zeitgrenzen: dann einzeln wie im Gate von 3d.2 und der Lauf des ganzen Skripts auf main nach dem Merge); `target/agent/3d-5/` vor `git worktree remove` sichern.
- [ ] **Step 3: Roadmap und Doku** `docs/ROADMAP.md` Zeile 3d.5: „**Stand <Datum>:** Gate vom Agenten bestanden: `dialog_of` mit Sprüngen und Zähler „geteilt“, Dialog-Editor über das eine Textarchiv mit Abspalten, Rezepte „Neue Figur“ (nummerierte Nachricht), „Tür verbinden“, „Neuer Trainer hier“ (ein Schritt hinter der Sperre) im Kernel, über MCP und in Erzählen; Szenarien `pt-erzaehlen`, `hg-erzaehlen` mit `expect-map` und `expect-battle`; Branch `phase-3d-5-figur-dialog-tuer`; [docs/phase-3d-5-gate.md](phase-3d-5-gate.md); beim Nutzer offen: Hand und Blick“; `docs/agent-gate.md`, `docs/mcp.md`, `docs/dev-setup.md`, `docs/formats/recipes.md` wie unter Files; in den Entscheidungen (`docs/research/2026-10-30-phase-3d/entscheidungen.md`, „Was bewusst nicht entschieden ist“) den Punkt rotom-Literale durchstreichen und mit dem Befund (uxie `find_or_add_message`: Dedup nach Text, neuer Index bei Änderung; Rezepte schreiben darum Nummern) und dem Datum abschließen.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`.
- [ ] **Step 5: Commit** `docs(3d.5): gate of figure, dialog and doors, agent-gate section, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **Sprungbefehle:** die Namen in `script.jump_params` sind aus den Datenbanken geraten (`GoTo` Pt `platinum_v2.json:341`, `GoToIfSet` in `scripts.rs:3202`); der Test `jump_params_name_existing_commands` ist maßgeblich, ein Name, den die Datenbank nicht kennt, wird gestrichen oder durch den `legacy_name` ersetzt. Ob rotom `if … then` in `GoToIf`-Befehle lowert oder `IfStatement`-Knoten im AST hält (letzteres: `walk` in `checks/scripts.rs:246-264` kennt `IfStatement`), entscheidet, ob `branch` aus dem AST (`IfStatement { condition, .. }`) oder aus dem Befehl kommt; der Plan setzt den AST voraus.
- **Labels im AST:** ob ein Label (`hilfe:`) eine `StatementKind::Function` ohne `id` ist, mit `cargo doc -p rotom --open` prüfen; sonst der Knoten, den rotom dafür hat (`slots_in`, `scripts.rs:1226-1243`, zeigt die Form der Funktionen).
- **`NameTables` im Rezept:** `create_trainer` nimmt `&mut NameTables` (Task 5); der Weg (Laden im Rezept oder Reichen durch die Hülle) hängt daran, wie `commands/data/create.rs:27-35` und `g4k` sie halten; die 74 Trainer-Tests und `tests/trainer_create.rs` sind die Spezifikation.
- **`check-data` für Trainer:** kennt `g4k check-data` keinen Trainer-Erwartungswert, entfällt der `check`-Block im Szenario; der gated Test prüft das Team.
- **Kachel der Figur in HeartGold:** (23, 15) ist die Geschenk-Kachel des Trainer-Szenarios; der Spieler steht nach der Tür auf (23, 16) und drückt `up`. Weicht die Karte ab, die Kachel in der Test-Umgebung ablesen und den Kopfkommentar anpassen.
- **`text_archive_id` in `set_header`:** der Schlüssel der Header-JSON (`Pt`/`Hgss`) heißt im DTO `text_archive_id` (`api.ts:31, 51`); heißt er im Kernel-Header anders, den Namen aus `get_header` übernehmen.
- **Werkzeugzahl:** `the_curated_tool_count_stays_within_what_an_agent_reads_at_once` (`catalog.rs:1701`) kann mit zwei Werkzeugen reißen; dann Grenze heben mit Begründung im Commit (Task 6).
- **`WizardHost` und 3d.6:** 3d.6 öffnet den Bereich Geschichten (`App.tsx` `LOCKED_AREAS`); 3d.5 hängt in `App.tsx` nur den Zustand `dialogEditor` und die drei `WizardRequest`-Fälle an; beim Merge liegen die Konflikte an diesen Anhängepunkten und in `WizardHost.tsx` (gesammelt lösen, Erfahrung Phase 4: 13+ Konflikte an einen Opus-Subagenten, `.fig` von main behalten und neu rendern).
- **Merge mit 3d.4:** `data/refs.rs` (3d.4 Welt-Ziele, 3d.5 `Target::Message` und ein Aufruf), `commands/data/dto.rs` (`TargetDto`), `EventEditor.tsx` (3d.4 nichts, 3d.5 `DialogSection`), `InspectorHead.tsx` (3d.4 Plakette, 3d.5 nichts), `mcp.md`, `agent-gate.md`, `ROADMAP.md`, `README.md`; die Testdateien sind getrennt (`tests/dialog.rs`, `tests/refs_messages.rs`, `tests/recipes_erzaehlen.rs`).
- **Sprite im Entwurf:** `spriteTexture`/`peekSprite` (`world/sprites.ts`) laden asynchron; der Marker zeigt bis dahin den Platzhalter und tauscht die Textur beim Laden (`useDraft` rendert erneut); fehlt ein Sprite zur Grafik, bleibt der Platzhalter, kein Fehler.
