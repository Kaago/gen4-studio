# Phase 3d.1 „Flags und Variablen“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Flags und Variablen bekommen Namen und einen Nutzungsindex: die pret-Namen aus den scrcmd-Datenbanken je Spielfamilie, eigene Namen im Textprojekt (Register `data/ids/{flags,vars}.json` plus dünne Entitäten `data/{flags,vars}/NNNN.json`, Pflichtpräfix `FLAG_`/`VAR_`, als zweiter Name derselben Nummer, für rotom über `.g4-ids-flags.json`/`-vars.json`), die Ziele `Target::Flag`/`Var` mit Zugriff (`Read`, `Write`, `ReadWrite`, `Unknown`) aus Scripts (Befehle und Bedingungen), Trainer-Flags, Events (Sichtbarkeits-Flag, versteckte Items, Auslöser), Level-Scripts und der Rezept-Variable, dazu `usage(kind)` in einem Durchlauf, ein gebündeltes `refresh` und „frei“ aus dem Index; in der App die Fläche „Flags und Variablen“ (Liste mit Suche und Filtern, leere Spalte „Live“, Detail „Wo gesetzt“/„Wo geprüft“ mit Wegen, „Umbenennen…“, Beschreibung, „Als eigenes Flag benennen…“), die Picker „Flag“/„Variable“ mit den neuen Feldern „Sichtbarkeits-Flag“ am Objekt, „Variable“ und „Wert“ am Auslöser und im Level-Script, der Fuß „Freies Flag nehmen“; MCP `list_flags`, `flag_usage`, `free_flag`, `name_flag`, `rename_id` mit den neuen Präfixen; `g4emu` liest und setzt Flags und Variablen im RAM (`save_block` je Spielcode, Schritte `flag`, `set-flag`, `var`, `set-var`, `expect-flag`, `expect-var`), die Szenarien `pt-flag`/`hg-flag`; der Fix für `textproject.id-dangling`.

**Architecture:** Ein neues Kernel-Modul `flags` (`names.rs` pret-Namen je Familie aus `metadata/scrcmd/*_v2.json`, `ranges.rs` Bereiche aus `metadata/flag_ranges.yaml`, `pool.rs` „frei“, `entities.rs` eigene Namen als Register plus Entität, `save_layout.rs` die RAM-Lage aus `metadata/save_layout.yaml`) und die Erweiterung des Referenzindex (`data/refs.rs` mit `data/refs/flagvars.rs`: deklarative Regeln aus `metadata/script_flagvars.yaml`, neue Quellen, `usage`, Zeitsperre für das Statten der Script-Quellen). Das Textprojekt meldet die neuen Arten über eine Art-Beschreibung als Daten an (`textproject/kinds.rs`); Register, `include.rs`, `rename.rs` und `check.rs` lernen die Arten `Flag`/`Var` von `SymbolKind`. Die App bekommt vier Befehle (`list_flags`, `flag_usage`, `free_flag`, `name_flag`) und drei neue Event-Felder im Patch; die Fläche lebt als Technik-Eintrag „Flags und Variablen“ (Komponenten unter `apps/studio/src/flags/`), bis 3d.6 den Bereich Geschichten öffnet. `g4emu` liest den Spielstand-Block hinter dem Zeiger, den `games.yaml` schon für den Ort nutzt.

**Tech Stack:** Rust (g4-kernel, rotom 0.7.0 als Parser, uxie 0.9.7 für Events, serde_yaml für die Metadaten, serde_json mit `preserve_order`), Tauri-Befehle, MCP (Befehlstabelle, Katalog, Zugriff, symbolische Argumente), React 19 mit den Bausteinen aus `apps/studio/src/ui/`, vitest, Python 3 (`scripts/g4emu`, pytest).

**Spec:** [2026-10-30-phase-3d-erzaehlen-design.md](../specs/2026-10-30-phase-3d-erzaehlen-design.md) (§1, §2 „Pool“, „rotom 0.7.0“, „Referenzindex“, „App“, §3.2, §3.6, §4 Zeile 3d.1, §7, §8); [entscheidungen.md](../../research/2026-10-30-phase-3d/entscheidungen.md) (Tabellen „Phasenschnitt und Reihenfolge“ Zeile 3d.1, „Flags und Variablen (Gutachten B)“ wörtlich, „Nicht in 3d“, „Was bewusst nicht entschieden ist“); [Gutachten B](../../research/2026-10-30-phase-3d/gutachten-b-flags-variablen-live.md) (§1.1 bis §1.5 Inventar und Bereiche, §2.1 und §2.2 Namen und Variante C, §3.1 bis §3.4 Index und Fläche, §4.2 RAM-Lage, §4.4 `g4emu`-Schritte, §6 Schnitt und Gate); [Gutachten A](../../research/2026-10-30-phase-3d/gutachten-a-geschichten-erzaehlen.md) §2.3 („Was der Code heute trägt“) und §5 („Was der Schnitt voraussetzt, von Gutachten B“: Register, Index-Ziele, die Felder `hidden_flag` und `var`/`value` in Inspector und Patch, `allocate_flag`/`allocate_var`, hier `free_flag`/`free_var`); [Gutachten C](../../research/2026-10-30-phase-3d/gutachten-c-loeschen-sprites-schnitt.md) §2.1 (Fix `id-dangling`, `textproject/check.rs:226-233`) und §6.6 (Art-Beschreibung als Daten); [ROADMAP.md](../../ROADMAP.md) „Phase 3d“, Zeile 3d.1.

Stand 30.10.2026: `main` bei `3c194c7` (Phase 4 abgeschlossen, Spezifikation 3d). Arbeit im Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-1`, Branch `phase-3d-1-flags-variablen` (existiert, Basis main `3c194c7`). Parallel laufen in eigenen Worktrees 3d.2 (`phase-3d-2-erzaehlen-rahmen`), 3d.3 (`phase-3d-3-sprites-ds-eier`) und 4.5 (`phase-4-5-veroeffentlichen`); Abschnitt „Parallele Teilphasen“. Kernel `crates/g4-kernel/src`, Tauri `apps/studio/src-tauri/src`, Frontend `apps/studio/src`, Emulator `scripts/g4emu`.

## Voraussetzungen (auf main `3c194c7`, vor Task 1 mit `grep` prüfen)

| Schnittstelle | erwartet | gebraucht in |
|---|---|---|
| `data::refs::{RefIndex, Ref, Target, ScriptRefTable, ScriptSources, ScriptNames, script_id, MAX_READERS, FIRST_VARIABLE}` | wie in `refs.rs` heute (`Ref { source, field, slot, target }`, `Target` mit sechs Arten) | Task 3, 4 |
| `scripts::{visit, AstItem, parse_source, rotom_sources, source_path, SourceKind, LevelScriptDoc, position_of, database_dir, base_constants, is_adopted, resolve_script}` | `AstItem::{Command, Alias, Declared, Expression}`; `LevelScriptDoc = rotom::LevelScript` mit `var_conditions: Vec<LevelScriptVarConditionEntry>` (`var`, `value`, `script_id`) | Task 3, 4, 5 |
| `checks::scripts::database(family) -> Option<&'static rotom::DatabaseV2>` (`commands[name].params[i].{name, type, access}`) | die gebündelten `scrcmd/{platinum,hgss}_v2.json` mit `flags` und `vars` (`{ "<NAME>": { "id": n } }`) | Task 1, 3 |
| `textproject::symbols::{SymbolKind, SymbolTable, table, fallback_id, parse_fallback}`, `registry::{Registry, Symbols, REGISTRY_KINDS, registry_file, IDS_DIR}`, `include::{KINDS, tables, write_include, ScriptLookup}`, `rename::{plan_rename, rename_id, kind_of, well_formed, format_text, judge, prepare, apply, NOT_A_TEXT_PROJECT}`, `ids::{Resolver, RefInput}`, `manifest::{Manifest, manifest_for, schema_state}`, `arts::{is_known_folder, numbered}`, `check::{hints, dangling}`, `codes::*` | wie heute | Task 1, 2 |
| `store::{FileStep, FileStepKind::IdRename, FileChange, write_file_step, HEADER_TABLE, Store::record_file_step}`; `rename::{before_move, after_move}` | der Dateischritt eines ID-Umbenennens mit seinen Folgen (Abgleich, ID-Dateien) | Task 2 |
| `templates::variable_id(family, name) -> Option<u16>`, `metadata/templates.yaml` `done_var` | `VAR_UNUSED_0x406C` (pt), `VAR_UNK_4031` (hgss) | Task 4 |
| `data::arm9::Layout::{trainer_flag_start, trainer_flag_capacity}` (`0x550`, 1040) | `arm9_tables.yaml` CPUD, IPKD, IPKE | Task 1 (die Bereiche stimmen damit überein, Test) |
| `commands::events::EventPatch` (15 Felder), `ObjectDto`, `CoordDto`, `update_event`, `add_event` (`events.rs:310, 341` setzen `hidden_flag: 0`, `value: 0`) | ohne `hidden_flag`, `var`, `value` | Task 5 |
| `commands/data/{dto.rs: TargetDto, RefDto, NavTargetDto; refs.rs: refs(session, target)}`, `commands::refs_impl` | `RefDto { source, field, slot, label, source_name, nav }` | Task 5 |
| `mcp/{access.rs: Kind, APP_ONLY; catalog.rs: ToolDef, object, text, no_args, answers_object, renamed, data_note!; tools.rs: NOT_IN_BATCH; symbolic.rs: SYMBOLIC_ARGS, ArgRef, RefKindOf, kind_named}` | wie heute | Task 5 |
| Frontend `workbench/inspector/{eventFields.ts: EventFieldSpec, eventFieldSpecs, PATCH_KEYS; EventEditor.tsx; LevelScriptSection.tsx; levelScript.ts}`, `ui/primitives/{Picker, Reference, PanelHead, ListRow, FilterChips, Badge, Section, Notice, Table, Tooltip, EmptyState, TextField}`, `technik/TechnikArea.tsx` (`TechnikEntry`, `ENTRIES`, `subtitles`, `Surface`), `textproject/RenameIdDialog.tsx` (`{ id, onClose, onRenamed }`), `workbench/navigation.ts` (`NavTarget`), `app/paletteEntries.ts`, `data/useRefs.ts` | wie heute | Task 6, 7 |
| `scripts/g4emu/g4emu/{games.yaml, games.py: Game, load_games, read_location, RAM_BASE; steps.py: ARITY, HANDLERS, Step, StepFailed, run_step; scenario.py}`, `tests/conftest.py` (`FakeEmu`, `put_location`) | wie heute | Task 8 |
| Szenarien `scripts/agent/scenarios/{pt,hg}-trainer-begegnung.yaml` | der Weg „neues Gebiet aus Vorlage, neue Eventdatei, neue Scriptdatei, Treppe im Zimmer führt dorthin“ | Task 8 |

## Parallele Teilphasen

3d.2 ändert `apps/studio/src/ui/frame/{shellState.ts,Hinge.tsx}`, `workbench/inspector/{fields.ts,eventFields.ts}` (Feld `from`), ein Kernel-Modul `recipes`, `src-tauri/src/commands/wizards.rs` und `world/scene/SceneHost.ts`; 3d.3 ändert `g4-formats` (`ncgr.rs`), `data/battle_sprites.rs`, `ui/primitives/DSScreen.tsx` und die Pokédex- und Trainer-Ansichten; 4.5 ändert `technik/PublishSurface*`, `patch.rs`, `git.rs` und die Titelleiste. **3d.1 baut keine dieser Dateien um.** In `eventFields.ts` hängt 3d.1 nur die neuen Felder und das Feld `pick` an (3d.2 hängt `from` an derselben Schnittstelle an; der Merge löst beides als Anhängen). `TechnikArea.tsx` bekommt nur den neuen Eintrag am Ende von `ENTRIES`, `subtitles` und der Zuordnung zur Fläche. Anhängepunkte (nur anhängen): `commands/dispatch.rs`, `src-tauri/src/lib.rs`, `mcp/{access,catalog,tools,symbolic}.rs`, `crates/g4-kernel/src/lib.rs`, `apps/studio/src/api.ts`, `workbench/navigation.ts`, `api.data.ts` (`NavTargetDto`), `commands/data/dto.rs` (`NavTargetDto`), `ui/primitives/icons.ts`, `docs/mcp.md`, `docs/ROADMAP.md`, `design-system/README.md`.

## Entscheidungen (wo Spezifikation und Entscheidungen offen lassen)

| Frage | Entscheidung | Grund |
|---|---|---|
| Ort der Fläche in 3d.1 | Technik-Eintrag „Flags und Variablen“ (`TechnikEntry` `flags`, angehängt nach „Werkzeuge“), Komponenten unter `apps/studio/src/flags/`; dazu die Palette („Flags und Variablen“), der Weg aus den Pickern („Im Verzeichnis zeigen“) und ein `NavTarget` `flag`. Mit 3d.6 zieht der Eintrag in den Bereich Geschichten (eine Zeile in `ENTRIES`, ein Mount) | der Bereich Geschichten bleibt gesperrt („Kommt mit Phase 3d“); Technik hält schon projektweite Flächen (Änderungen, Patches) und ist nur ein Anhängepunkt (4.5 baut `PublishSurface*`, nicht `TechnikArea.tsx` um); eine eigene Liste im gesperrten Bereich bräuchte ein Dock und `AREA_PANELS`, die 3d.2 berührt |
| `SymbolKind::Flag`/`Var` und die feste Tabelle | `table(kind)` behält seine Signatur und antwortet für `Flag`/`Var` `None`; die pret-Namen liefert `flags::names(family, kind) -> &'static NameTable` (je Familie, mehrere Namen je Nummer); `Symbols` bekommt `with_family(family)`, dann lösen `id`/`number` für `Flag`/`Var` zuerst das Register, dann die pret-Tabelle; `Resolver::for_project` setzt die Familie | 20 Aufrufer von `table()`; die pret-Tabellen sind je Familie, die Register je Projekt: die Mischform „zweiter Name derselben Nummer“ gehört in `Symbols`, nicht in die feste Tabelle |
| Hauptname einer Nummer mit mehreren pret-Namen | der erste Name der Datei mit dem Präfix `FLAG_`/`VAR_`, der nicht auf `_START`, `_END`, `_BASE` endet und nicht die Hex-Form `PREFIX_0x…` hat; sonst der erste mit Präfix; sonst der erste überhaupt (`VARS_START` für `0x4000` heißt so `VAR_MAP_LOCAL_0x00`, `0x800C` `VAR_RESULT`, Platin `0x997` `FLAG_VILLA_FURNITURE_TABLE`). Die übrigen Namen bleiben als Aliasse (Suche, `number()`) | Gutachten B §1.2: `VAR_`-Namen vor Bereichskonstanten |
| Zahl der Namen (Gate) | `names(Pt, Flag).len()` = 4 097, `names(Hgss, Flag).len()` = 2 244 (verschiedene Nummern; die Dateien haben 4 098 und 2 252 Einträge), Variablen 307 / 386 | Spezifikation §4 nennt „Nummern“ |
| Bereiche | neue Datei `crates/g4-kernel/metadata/flag_ranges.yaml` je Familie (`pt`, `hgss`): Flags `map_local` `0x001`–`0x040` (hgss `0x03F`), `general` `0x041` (hgss `0x040`)–`0x54F`, darin `hidden_items` `0x2DA`–`0x3F5` (hgss `0x320`–`0x406`; der engste Bereich gewinnt), `trainer` `0x550`–`0x95F`, `system` `0x960`–`0xA9F`, `daily` `0xAA0`–`0xB5F`, hgss `temp` `0x4000`–`0x403F`; `flags_end: 0xB60` (eins nach dem letzten Flag des Spielstands, 2 912 Flags, 364 Byte); Variablen `map_local` `0x4000`–`0x401F`, `object_gfx` `0x4020`–`0x402F`, `general` `0x4030`–`0x411F` (hgss `0x416F`), `script_local` `0x8000`–`0x8010` (hgss `0x800D`); `vars_end: 0x4120` (hgss `0x4170`). Flag 0 und Nummern außerhalb: `Area::Outside` | Gutachten B §1.5 (Grenzen aus den Namen geprüft: Platin `0x2DA` erstes und `0x3F5` letztes `FLAG_OBTAINED_HIDDEN_…`, hgss `HIDDEN_ITEMS_FLAG_BASE` `0x320` + `NUM_HIDDEN_ITEMS` 231; `VARS_END` ist in Platin exklusiv `0x4120`, in hgss der Name der letzten Variable `0x416F`); eine eigene Datei, weil `arm9_tables.yaml` je Spielcode und ARM9-Offsets führt |
| Basis der versteckten Items | aus `flag_ranges.yaml` (`hidden_items.start`), je Familie, nicht je Spielcode | Platin `0x2DA`, HeartGold `0x320` sind Konstanten der Familie (hgss `HIDDEN_ITEMS_FLAG_BASE`); die Entscheidung sagt „je Spielcode“, gemeint ist je Spiel; die drei Testspiele sind damit abgedeckt |
| „frei“ | `area_of(n) == General` **und** im Index ohne Verweis (jeder Zugriff zählt) **und** ohne eigenen Namen **und** der pret-Name fehlt oder enthält `UNUSED`, `DUMMY` oder `UNK` **und** nicht die `done_var` der Vorlage; `free_flag`/`free_var` geben die kleinste Nummer. Der Namens-Teil engt nur ein, nie aus: `FLAG_UNUSED_0x0096` an einem Objekt ist nicht frei | Entscheidung „nur aus dem Index“ plus die Pool-Zahlen der Spezifikation §2 (135 / 152 Flags, 21 / 83 Variablen, gezählt mit der Namensregel aus Gutachten A); ein vanilla benannter Fortschritts-Flag ohne Script-Verweis wird von der Engine gesetzt und darf nie vorgeschlagen werden |
| `done_var` der Vorlage | keine `Ref` (eine `Ref` braucht eine Asset-Quelle), sondern eine **Reservierung**: `pool` nimmt sie aus „frei“, `UsageEntry.reserved: Option<String>` (`„Vorlage „Leere Region““`) zeigt sie in Liste und Detail | Zweck der Entscheidung („damit frei es nicht vorschlägt“) ohne eine Quelle ohne Ort |
| Eigene Namen: Dateien | `data/ids/flags.json`, `data/ids/vars.json` (Register wie die vier anderen, Taufe nie; Export schreibt sie leer `{}`), `data/flags/NNNN.json` bzw. `data/vars/NNNN.json` mit `{"id": "FLAG_…", "description": "…"}` (`description` nur, wenn nicht leer; Nummer dezimal, mindestens vier Stellen wie `member_number` liest: `0148`, `16492`); jeder eigene Name hat eine Entitätsdatei; ohne eigenen Namen gibt es keine Datei und keine Beschreibung (der Kopf zeigt „Beschreibung…“ erst nach dem Benennen) | Variante C wörtlich („nur für benannte oder beschriebene Nummern“); eine Datei mit dem pret-Namen als `id` bräche, sobald pret den Namen ändert |
| Art-Beschreibung | `textproject/kinds.rs`: `DataKind { folder, symbol: SymbolKind, schema_version: u32, ref_fields: &[&str], delete: DeleteRule::RemoveFile }`, `DATA_KINDS: [DataKind; 2]`; `arts::is_known_folder` kennt sie, `check` prüft ihre Dateien, `manifest_for` schreibt `schemas: {"flags": 1, "vars": 1}` (die Werte von `schema_version`) und der Hinweis `textproject.reserved` gilt nur noch für fremde Schlüssel; Export und Import fassen die Ordner nicht an (keine Member); `DiffSubject::File` zeigt sie als Zeilen-Diff; „Datei entfernen“ ist deklariert, ausgeführt wird es erst in 3d.4 | Gutachten C §6.6 (Prüfstein für Phase 5), Entscheidung „eingebaute Arten bleiben im Enum“ |
| `name_flag` als Dateischritt | ein `FileStep` (Register und Entitätsdatei) mit `FileStepKind::IdRename` und den Folgen von `rename::apply` (Schloss des Abgleichs, Aufzeichnung, ID-Dateien neu); Label „Flag benannt: FLAG_X (0x0094)“, „Variable benannt: …“, „Beschreibung geändert: FLAG_X“; `rename::apply` wird `pub(crate) apply_file_step` | dieselben Folgen wie ein Umbenennen; ein neuer `FileStepKind` wäre ein Umbau von `store.rs` ohne Mehrwert |
| „Umbenennen…“ auf einem pret-Namen | legt keinen eigenen Namen an; der Kopf zeigt für eine Nummer ohne eigenen Namen „Als eigenes Flag benennen…“ (`name_flag`), mit eigenem Namen „Umbenennen…“ (`RenameIdDialog` von 4.3b, `rename_id`) | ein Dialog je Tätigkeit; Gutachten B §2.2 lässt beides zu |
| Regeln der Script-Quellen | `metadata/script_flagvars.yaml`: `name_rule` (rotoms Parameternamen für Flag und Variable, `ir_to_source.rs:28-58`), `access_words` (Datenbank `access` → `read`, `write`, `read_write`, `unknown`), je Familie `flags` (Befehl → Parameter → Zugriff; ein Eintrag erklärt den Parameter zum Flag, gleich welchen Typs), `vars` (ebenso für Variablen; hgss die Ausnahmeliste `SetVar`, `CopyVar`, `SetOrCopyVar`, `AddVar`, `SubVar`), `trainer_flags` (`Set/Clear/CheckTrainerFlag` → `Flag(0x550 + n)`), `flag_default: read`, `var_default: database` (pt) bzw. `borrow` (hgss: der gleichnamige Platin-Befehl, Parameter an derselben Stelle, sonst `unknown`); ein Test prüft jeden Eintrag gegen die Datenbank | Entscheidung „deklarativ, Typ plus Namensregel plus Bedingungen, HGSS borgt mit Ausnahmeliste“ |
| Welcher Wert zählt | Variablen-Parameter (Typ `var`/`flex`, Namensregel oder Eintrag): Wert `0x4000`–`0x7FFF` → `Var`; ab `0x8000` nur gezählt (`ScriptCounts.script_local`); unter `0x4000` kein Verweis. Flag-Parameter: Wert `1`–`0x3FFF` → `Flag`; `0` keiner; ab `0x4000` „Flag aus einer Variable“, nur gezählt (`ScriptCounts.dynamic`). Bedingungen (`if`/`elseif`/`while`/`match`): ein nackter Bezeichner, dessen Name mit `FLAG_` beginnt und auflöst → `Flag` Read; ein Operand eines Vergleichs, der mit `VAR_` beginnt oder als Zahl `≥ 0x4000` auflöst → `Var` Read | Gutachten B §3.2 Punkte 1 und 2; `FIRST_VARIABLE` gilt weiter |
| Bedingungen im AST | `scripts::visit` meldet zusätzlich `AstItem::Condition(&Expression)` für die Bedingung von `if`, `elseif`, `while` und den Kopf von `match` (die `Expression`-Meldungen bleiben), damit der Index Bedingungen von Argumenten unterscheidet | ohne Kontext zählte jeder Bezeichner doppelt |
| Level-Scripts als Quelle | dieselbe Quell-ID wie eine Scriptdatei (`AssetId { Script, file }`): eine Datei ist `.rotom` oder `.json`; `ScriptSources` stattet beide Formen, `read_scripts` liest je `SourceKind`; Feld `var_conditions`, Slot der Index der Bedingung, `Var` Read | eine Nummer, eine Quelle; die neue Quellart ist der `SourceKind`, nicht eine neue `AssetKind` |
| Trainer-Flags aus Objekt-Events | Objekt mit Script in `TRAINER_SCRIPTS`/`PARTNER_SCRIPTS` → zusätzlich `Flag(0x550 + n)` ReadWrite, Feld `script`; der Trainer-Verweis bleibt | Gutachten B §3.2 Punkt 3 (Engine liest und setzt) |
| Versteckte Items | BG-Event mit `event_type == 2` und Script `8000 + n` → `Flag(hidden_items.start + n)` ReadWrite, Feld `hidden_item`; `n` über die Länge des Bereichs hinaus → kein Verweis | Gutachten B §1.3 |
| `Ref.access` | neues Feld `access: Access` (`Read`, `Write`, `ReadWrite`, `Unknown`, serde `snake_case`); alle heutigen Verweise `Read`; `RefDto` trägt es | Gutachten B §3.2 |
| `refresh` gebündelt | Zeitsperre: `RefIndex` merkt sich `scripts_checked: Option<Instant>`; das Statten der Script-Quellen (`ScriptSources::refresh`, das teure Stück) läuft höchstens alle `REFRESH_SCRIPTS_EVERY` = 250 ms, Store-Änderungen (`changes_since`, billig) immer; `RefIndex::refresh_scripts_now(store)` erzwingt es (Tests, nach `save_script`) | Entscheidung „Zeitsperre ~250 ms oder Generationszähler“: die Sperre braucht keinen Umbau der App und hilft der Trainerliste von heute mit |
| `usage(kind)` | `BTreeMap<u16, Usage { read, write, unknown, sources }>` in einem Durchlauf über den Index (`ReadWrite` zählt bei `read` und `write`, `sources` = verschiedene Quellen), dazu `ScriptCounts { script_local, dynamic }` für die Fläche; kein Cache | 27 000 Verweise in unter einer Millisekunde |
| Kartenlokale Flags und Variablen | der Index speichert sie wie jeden Verweis; `flag_usage` liefert je Verweis den Header (Eventdatei → Header über `event_file_id`, Scriptdatei → Header über `script_file_id`, Level-Script → Header über `level_script_id`), die Fläche gruppiert das Detail für `MapLocal` nach Header („Zweiblattdorf (0411)“, `MAP_T01` in Tüfteln) statt nach Quelle; kartenlokale Nummern sind nie „frei“ und die Liste zeigt sie mit dem Zähler „an 54 Orten“ | Gutachten B §3.2 „gruppiert nach Header“ |
| Zahlen in der Liste | Nummer hex vierstellig und dezimal („0x0094 · 148“); eigener Name fett, pret-Name grau in Mono; Zähler „gesetzt N · geprüft M“ (`write`, `read`; `unknown` als „· unklar K“, nur wenn > 0) | Gutachten B §3.4 |
| Filter | `FilterChips` „Flags | Variablen“ (Art), „Alle | allgemein | kartenlokal | Trainer | versteckte Items | System | Tagesflags | Script-lokal“ (Bereich, je Art die passenden), Kästchen „nur unbenutzt“, „nur frei“, „nur mit eigenem Namen“; Suche über eigenen Namen, pret-Namen, Aliasse, Nummer hex und dezimal | Gutachten B §3.4 |
| Spalte „Live“ | Kopf „Live“ in `status/warnung` (Bernstein), jede Zelle „–“ sekundär mit Tooltip „Kommt mit dem Spielmodus (3d.9)“, `data-flag-live`; kein neues Token | Spezifikation §3.6 „leere Spalte Live“ |
| Picker | `FlagPicker` über `Picker`: Treffer „Name“ (eigener fett, sonst pret), Beschreibung der pret-Name (bei eigenem) oder der Bereich, `tech` die Nummer hex, `count` „gesetzt N · geprüft M“; Filter-Plaketten die Bereiche; Fuß „Freies Flag nehmen“ (`free_flag`, gesperrt mit „Kein freies Flag im allgemeinen Bereich“) und „Im Verzeichnis zeigen“ (öffnet die Fläche auf dem gewählten Treffer); „Keins (0)“ als erster Treffer | Gutachten B §3.4; 0 bleibt gültig (kein Flag, keine Variable) |
| Felder im Inspector | `EventFieldSpec` bekommt `pick?: "flag" \| "var"`; Objekt: `{ key: "hidden_flag", label: "Sichtbarkeits-Flag", pick: "flag" }` nach „Sichtweite“; Auslöser: `{ key: "var", label: "Variable", pick: "var" }`, `{ key: "value", label: "Wert" }` nach „Script“; `PATCH_KEYS` um die drei Schlüssel; der Patch (`EventPatch`) um `hidden_flag`, `var`, `value`; `ObjectDto.hidden_flag`, `CoordDto.{var, value}`; die Feldzeile zeigt `FlagField` (Name als `Reference`-Chip der Fläche `geschichten` mit Nummer als `tech`, „Wählen…“ öffnet den Picker, in Tüfteln dazu das Zahlfeld) | Gutachten A §5; Entscheidung „in Bauen und Tüfteln“ |
| Level-Script | die Felder „Variable“ (Bestand und „Neue Variable“) in `LevelScriptSection.tsx` werden `FlagField` der Art Variable; „Wert“ und „Script der Bedingung“ bleiben Zahlfelder | Gutachten B §3.4 „dazu im Level-Script“ |
| MCP | `list_flags` (Read), `flag_usage` (Read, `data_note!()`), `free_flag` (Read), `name_flag` (Write, nicht in `NOT_IN_BATCH`), `rename_id` (Text um `FLAG_`, `VAR_`); symbolisch: `flag_usage.number` und `name_flag.number` nehmen ID oder Nummer (`kind_named`: `flag` → `Flag`, `var` → `Var`) | Entscheidung nennt vier Werkzeuge; „Als eigenes Flag benennen…“ braucht einen schreibenden Befehl, und das Szenario benennt kopflos |
| `save_block` | `games.yaml` je Spielcode `save_block: { pointer, header, vars, vars_count, flags, flags_bytes }` (CPUD `0x02101EE0`, `0x14`, `0xDAC`, 288, `0xFEC`, 364; IPKD `0x02111860`, `0x10`, `0xDE4`, 368, `0x10C4`, 364; IPKE `0x02111880`, sonst wie IPKD); `metadata/save_layout.yaml` im Kernel mit denselben Schlüsseln; ein Kernel-Test liest beide Dateien und verlangt Gleichheit | Gutachten B §4.2; `vars_count` und `flags_bytes` fehlen in der Entscheidung, ohne sie kann kein Schritt einen Bereich prüfen |
| `g4emu`-Schritte | `flag N` → `{flag, value}`; `set-flag N 0\|1`; `var N` → `{var, value}`; `set-var N W` (`W` 0–65535); `expect-flag N W`; `expect-var N W`; `N` dezimal oder `0x…`; ohne gültigen Zeiger `StepFailed("kein gültiger Spielstand im RAM")`; außerhalb des Blocks `ValueError`. Flag `n`: Bit `n % 8` von Byte `n / 8`; Variable `v`: u16 an `2 × (v − 0x4000)` | Gutachten B §4.4 |
| Szenarien | `pt-flag.yaml`, `hg-flag.yaml` laufen über MCP (`mcp: [--allow-irreversible]`) nach dem Muster der Trainer-Szenarien: `adopt_text_project`, neues Gebiet aus der Vorlage mit neuer Eventdatei und Scriptdatei, Treppe dorthin; `free_flag`, `name_flag` → `FLAG_SZENARIO_FIGUR`, Figur mit `hidden_flag` (Nummer), Script `SetFlag FLAG_SZENARIO_FIGUR`; spielen: ansprechen, durch die Tür zurück, wieder hinunter, `expect-flag N 1`, `walk-until-blocked` durch die Kachel der Figur frei, `shot` | Gate §4; mit `adopt_text_project` läuft der ganze Weg eigener Name → ID-Datei → Compile |
| Gated Tests | neu `crates/g4-kernel/tests/flags.rs` (`G4_TEST_PROJECT`, schreibt nur auf `vanilla_copy()`), Studio `flags_over_the_command_path_on_the_vanilla_copy` in `commands/flags.rs`; Zeitgrenzen nur mit `G4_TIMING_STRICT=1` (Index warm ≤ 1 000 ms, 200 Abfragen ≤ 50 ms, `usage` ≤ 50 ms) | Global Constraints |
| Nicht in 3d.1 | Namen statt Zahlen in den Event-JSON (3d.4, `schemaVersion` 2), Spielmodus live in der App (3d.9), Ei-Attacken (3d.3), Geschichten (3d.6), „Datei entfernen“ eines eigenen Namens (3d.4), `g4k`-Befehle für Flags (MCP genügt dem Gate), ein `DiffSubject::Flag` (3d.4) | Entscheidung „Nicht in 3d“ und Schnitt |

## Global Constraints

- Arbeitsort: Worktree `/Users/POLTEP/Projekte/lab/editor/gen4-studio/.claude/worktrees/phase-3d-1`, Branch `phase-3d-1-flags-variablen` (existiert, Basis main `3c194c7`). Nie `cd` in die Hauptkopie, nie Branch wechseln, nie nach main mergen, kein `git stash`. `tools/` ist dort ein Symlink auf den Haupt-Checkout (nie einchecken).
- cargo braucht `export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"`. `pnpm install` einmal in `apps/studio`.
- Jeder Commit endet mit der Zeile `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` (ASCII-`@`, kein `＠`).
- Originale unter `/Users/POLTEP/Projekte/lab/editor/Roms` nie öffnen oder ändern; Testkopien nur `Roms/_testkopien/3d-1-<zweck>` (`cp -cR` für den Ordner `3d-1-<zweck>_DSPRE_contents`, `cp -c` für `3d-1-<zweck>.nds`, beide direkt nebeneinander). `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms` absolut. Gated Tests lesen `G4_TEST_PROJECT` und schreiben nur auf `vanilla_copy()`. Keine ROM-Daten ins Repo (keine `.nds`, keine Member, keine exportierte JSON eines Spiels; Fixtures synthetisch).
- Test-Umgebung nur auf `G4_VITE_PORT=1700 G4_BRIDGE_PORT=1701` (`bash scripts/agent/studio-web.sh`); andere Ports nie anfassen.
- UI nur aus `apps/studio/src/ui/primitives` und Tokens `--g4-*`, keine Hex-Farben, keine Inline-Styles (`bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`); deutsche Oberfläche, Bezeichner englisch; höchstens eine primäre Taste je Dialog.
- Anhängepunkte (`dispatch.rs`, `lib.rs`, `access.rs`, `catalog.rs`, `tools.rs`, `symbolic.rs`, `crates/g4-kernel/src/lib.rs`, `api.ts`) nur anhängen. Die Dateien der parallelen Teilphasen (Abschnitt „Parallele Teilphasen“) nicht umbauen; in `eventFields.ts` nur neue Felder anhängen.
- Vor jedem Commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, die Tests der berührten Crates; bei Frontend-Änderungen zusätzlich `cd apps/studio && pnpm test && pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`; bei `scripts/g4emu` `cd scripts/g4emu && uv run pytest -m "not integration" -q`.
- Zeitgrenzen nur unter `G4_TIMING_STRICT=1` (Index warm ≤ 1 000 ms, 200 Abfragen ≤ 50 ms), nach dem Muster von `vanilla_index_builds_under_a_second` (`tests/refs.rs:999`): ohne die Variable drucken die Tests nur.
- Nicht in 3d.1 (wörtlich): Namen statt Zahlen in den Event-JSON des Textprojekts (kommt mit `schemaVersion` 2 in 3d.4), Spielmodus live in der App, Ei-Attacken, Geschichten.
- Texte der Oberfläche genau wie in den Tabellen der Tasks 6 und 7.

## Review Focus

- Eine Nummer mit pret-Namen **und** eigenem Namen (`FLAG_UNUSED_0x0094` und `FLAG_HAT_MEDAILLE`): die ID-Datei enthält nur den eigenen Namen, ein Script mit dem pret-Namen kompiliert weiter, die Liste zeigt beide, `rename_id` auf den eigenen Namen zieht Script und Entitätsdatei nach (Task 2 `own_name_is_a_second_name_and_both_compile_the_same`, `renaming_an_own_flag_name_follows_script_and_entity`).
- Ein Flag, das `UNUSED` heißt und benutzt wird (Platin `0x96` an einem Objekt): nie „frei“, nie Vorschlag von „Freies Flag nehmen“ (Task 4 `a_used_unused_flag_is_not_free`).
- Script-Argumente, die keine Nummer sind: ein Flag aus einer Variable (`SetFlag VAR_RESULT`, `SetFlagVar`), Variablen ab `0x8000`, ein Alias ohne Wert, ein nicht lesbares Script: keine falschen Verweise, nur Zähler, kein Fehler (Task 3 `dynamic_flags_and_script_locals_are_counted_not_indexed`, `a_source_that_does_not_parse_contributes_nothing_for_flags`).
- DSPRE-Layout ohne Textprojekt: Liste, Detail und Picker mit pret-Namen, „Umbenennen…“ und „Als eigenes Flag benennen…“ gesperrt mit dem Grund `NOT_A_TEXT_PROJECT` und dem Weg; `name_flag` über MCP antwortet `not_text_project` (Task 5 `name_flag_refuses_the_dspre_layout`, Task 7 `the_head_locks_naming_in_the_dspre_layout`).
- Randwerte: Sichtbarkeits-Flag 0 und Auslöser-Variable 0 sind kein Verweis, der Picker zeigt „Keins (0)“ und der Wert bleibt speicherbar; kartenlokale Flags (`0x20` an 54 Objekten in Platin) stehen als eine Zeile „an 54 Orten“, das Detail gruppiert nach Header, „frei“ sind sie nie; eine Nummer außerhalb des Spielstands (`0xFFF`) trägt den Bereich „außerhalb“ und lässt sich nicht benennen (Task 4 `zero_flags_and_variables_are_no_reference`, `map_local_and_trainer_flags_are_never_free`; Task 6 `the_picker_offers_none_and_keeps_zero`; Task 7 `map_local_rows_count_places`, `map_local_groups_by_header`; Task 2 `naming_refuses_a_number_outside_the_save`).

---

### Task 1: pret-Namen, Bereiche, RAM-Lage als Metadaten

**Files:**
- Create: `crates/g4-kernel/metadata/flag_ranges.yaml`, `crates/g4-kernel/metadata/save_layout.yaml`, `crates/g4-kernel/src/flags.rs` (Modul-Doku, `FlagKind`, Re-Exporte), `crates/g4-kernel/src/flags/names.rs`, `crates/g4-kernel/src/flags/ranges.rs`, `crates/g4-kernel/src/flags/save_layout.rs` (+ Tests je Datei)
- Modify: `crates/g4-kernel/src/lib.rs` (`pub mod flags;` am Ende), `crates/g4-kernel/src/textproject/symbols.rs` (`SymbolKind::Flag`, `SymbolKind::Var`, Präfixe `FLAG`, `VAR`; `table()` antwortet `None`, `files()` leer; die Tests `tables_cover_the_gen4_ranges` und `fallback_ids_round_trip` nehmen die zwei Arten in die Liste der Arten ohne feste Tabelle auf)

**Interfaces:**
- Consumes: `checks::scripts::scrcmd_database`/`database(family)`; die JSON `flags`/`vars` der Datenbanken; `g4_formats::game::Family`.
- Produces:

```rust
// flags.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum FlagKind { Flag, Var }
impl FlagKind {
    pub fn symbol(self) -> SymbolKind;            // Flag → SymbolKind::Flag, Var → SymbolKind::Var
    pub fn prefix(self) -> &'static str;          // "FLAG", "VAR"
    pub fn folder(self) -> &'static str;          // "flags", "vars" (data/<folder>/, data/ids/<folder>.json)
    pub fn word(self) -> &'static str;            // "Flag", "Variable"
}
// flags/names.rs
pub struct NameTable { /* number → Hauptname, number → alle Namen (Dateireihenfolge), name → number */ }
impl NameTable {
    pub fn name(&self, n: u16) -> Option<&str>;               // der Hauptname (Entscheidungen)
    pub fn names(&self, n: u16) -> &[String];                 // alle Namen der Nummer
    pub fn number(&self, name: &str) -> Option<u16>;          // jeder Name, auch Alias
    pub fn iter(&self) -> impl Iterator<Item = (u16, &str)>;  // Hauptnamen, aufsteigend
    pub fn len(&self) -> usize;                               // verschiedene Nummern
    pub fn is_empty(&self) -> bool;
}
/// The pret names of `kind` in `family`'s command database (`flags`/`vars`); empty for Dp.
pub fn names(family: Family, kind: FlagKind) -> &'static NameTable;
/// Whether a pret name says the number is unused: it is `None` or contains UNUSED, DUMMY or UNK.
pub fn name_is_unused(name: Option<&str>) -> bool;
// flags/ranges.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum Area { MapLocal, General, HiddenItems, Trainer, System, Daily, Temp, ObjectGfx, ScriptLocal, Outside }
impl Area { pub fn word(self) -> &'static str; }  // kartenlokal, allgemein, versteckte Items, Trainer, System, Tagesflags, temporär, Objektgrafik, Script-lokal, außerhalb
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range { pub area: Area, pub start: u16, pub end: u16 }  // inclusive
pub struct Ranges { pub flags: Vec<Range>, pub flags_end: u16, pub vars: Vec<Range>, pub vars_end: u16 }
pub fn ranges(family: Family) -> &'static Ranges;             // Dp: wie pt (unverified), per Kommentar in der Datei
pub fn area_of(family: Family, kind: FlagKind, n: u16) -> Area; // der engste Bereich, der n enthält; sonst Outside
/// Whether `n` is one the save block holds: flags 1..flags_end, variables 0x4000..vars_end.
pub fn in_save(family: Family, kind: FlagKind, n: u16) -> bool;
pub fn hidden_items_base(family: Family) -> Option<u16>;        // hidden_items.start
pub fn trainer_flag(family: Family, trainer: u16) -> Option<u16>; // trainer.start + trainer, wenn ≤ trainer.end
pub fn pool_range(family: Family, kind: FlagKind) -> Option<Range>; // der Bereich General
// flags/save_layout.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveBlock { pub pointer: u32, pub header: u32, pub vars: u32, pub vars_count: u32, pub flags: u32, pub flags_bytes: u32 }
pub fn save_block(game_code: &str) -> Option<SaveBlock>;      // CPUD, IPKD, IPKE
```

- `flag_ranges.yaml` wörtlich mit den Werten aus „Entscheidungen“ (Kopfkommentar: Quelle Gutachten B §1.5, die Namen an den Grenzen; `hidden_items` liegt in `general`, der engste Bereich gewinnt; Diamant/Perl ungeprüft: `dp: { like: pt }`). `save_layout.yaml`: je Spielcode die sechs Schlüssel, Kommentar mit Messung (Gutachten B §4.2: Zeiger fest je Sitzung, Block wandert je Start bis `0xB4`; der Lesepuffer bei `0x0236D6xx`/`0x023775xx` ist nicht lebendig). Beide per `include_str!`, geparst einmal je Prozess (`OnceLock`), Parse-Fehler sind Test-Fehler (leer zur Laufzeit).

- [ ] **Step 1: Scheiternde Tests:**
  - `names.rs`: `platinum_and_heartgold_name_the_decided_counts` (Flags 4 097 / 2 244 Nummern, Variablen 307 / 386; `names(Dp, Flag).is_empty()`), `the_main_name_prefers_a_prefixed_plain_name` (pt `0x4000` → `VAR_MAP_LOCAL_0x00`, `0x800C` → `VAR_RESULT`, `0x997` → `FLAG_VILLA_FURNITURE_TABLE`; hgss `0x320` → `FLAG_HIDDENITEM_T20_POTION`, `0x40` → `FLAG_UNK_040`, `0x550` → `TRAINER_FLAG_BASE` (einziger Name)), `aliases_resolve_to_their_number` (`number("VARS_START") == Some(0x4000)`, `number("HIDDEN_ITEMS_FLAG_BASE") == Some(0x320)`), `unused_names_are_unused_dummy_unk_or_none`.
  - `ranges.rs`: `ranges_follow_the_decisions` (pt `area_of(Flag, 0x20) == MapLocal`, `0x41 == General`, `0x2DA == HiddenItems`, `0x3F5 == HiddenItems`, `0x3F6 == General`, `0x550 == Trainer`, `0x95F == Trainer`, `0x960 == System`, `0xAA0 == Daily`, `0xB5F == Daily`, `0xB60 == Outside`, `0 == Outside`; `area_of(Var, 0x4030) == General`, `0x411F == General`, `0x4120 == Outside` (pt) und `General` (hgss bis `0x416F`), `0x8000 == ScriptLocal`; hgss `area_of(Flag, 0x4000) == Temp`), `in_save_matches_the_block_sizes` (pt Flags `1..0xB60`, 2 912 = 364 × 8; Variablen pt `0x4000..0x4120` = 288, hgss `0x4000..0x4170` = 368), `trainer_flags_match_the_arm9_tables` (`trainer_flag(Pt, 0) == Some(0x550)`, `trainer_flag(Pt, 1039) == Some(0x95F)`, `trainer_flag(Pt, 1040) == None`; gleich `Layout::trainer_flag_start`/`capacity` von CPUD, IPKD, IPKE aus `data::arm9`), `pool_range_is_general`.
  - `save_layout.rs`: `three_game_codes_with_the_measured_values` (CPUD `0x02101EE0`, `0x14`, `0xDAC`, 288, `0xFEC`, 364; IPKD `0x02111860`, `0x10`, `0xDE4`, 368, `0x10C4`, 364; IPKE `0x02111880` sonst wie IPKD; `save_block("ADAE") == None`), `the_counts_match_the_ranges` (`vars_count` = `vars_end − 0x4000`, `flags_bytes × 8` = `flags_end`), `g4emu_games_yaml_carries_the_same_block` (liest `include_str!("../../../../scripts/g4emu/g4emu/games.yaml")`, Schlüssel `save_block` je Code gleich; **schlägt bis Task 8 fehl: der Test wird in Task 1 mit `#[ignore = "games.yaml bekommt save_block in Task 8"]` angelegt, Task 8 nimmt das `ignore`**).
  - `symbols.rs`: `flag_and_var_have_prefixes_and_no_fixed_table` (`prefix() == "FLAG"`, `"VAR"`; `table(Flag).is_none()`; `fallback_id(Flag, 148) == "FLAG_0148"`, `parse_fallback(Var, "VAR_16492") == Some(16492)`).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features flags:: symbols::` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces; die Namenstabelle liest die JSON der Datenbank über `serde_json::Value` (`preserve_order`, Dateireihenfolge) einmal je Familie und Art.
- [ ] **Step 4: Lauf** → PASS; `cargo clippy`, `cargo fmt --check`.
- [ ] **Step 5: Commit** `feat(flags): pret names per family, ranges and the save block as metadata` (mit Trailer).

### Task 2: Eigene Namen: Register, Entitäten, Art-Beschreibung, ID-Dateien, Umbenennen, Fix `id-dangling`

**Files:**
- Create: `crates/g4-kernel/src/textproject/kinds.rs` (+ Tests), `crates/g4-kernel/src/flags/entities.rs` (+ Tests), `docs/formats/flags_vars.md` (Abschnitte „Namen aus pret“, „Bereiche“, „Eigene Namen“, „Art-Beschreibung“)
- Modify: `crates/g4-kernel/src/textproject/mod.rs` (`pub mod kinds;`), `registry.rs` (`REGISTRY_KINDS` 6, `registry_file` `flags.json`/`vars.json`, `Symbols::{family, with_family}`), `ids.rs` (`Resolver::for_project` setzt die Familie; `Resolver::fixed()` ohne), `include.rs` (`KINDS` 11: `(Flag, "flags")`, `(Var, "vars")`), `rename.rs` (`KINDS` 11; `judge` lehnt einen pret-Namen einer anderen Nummer ab; `apply` → `pub(crate) fn apply_file_step`; `well_formed`, `format_text` `pub(crate)`), `arts.rs` (`is_known_folder` kennt `kinds::data_kind`), `manifest.rs` (`manifest_for` schreibt `schemas` der Art-Beschreibungen), `check.rs` (`dangling` zählt höchste Nummer + 1 und lässt `Flag`/`Var` aus; `hints` ruft `kinds::check_files`; der Hinweis `reserved` nur für fremde Schlüssel), `docs/formats/text_project.md` (Tabelle „Arten“ Zeilen `flags`, `vars`; „Symboltabellen und Register“: sechs Register, Flags und Variablen ohne Taufe; „IDs in Scripts“: die Dateien `-flags`, `-vars` nur mit Registereinträgen; „Manifest“: `schemas` mit den Arten; „ID umbenennen“: `FLAG_`, `VAR_`)

**Interfaces:**
- Consumes: Task 1 (`FlagKind`, `names`, `in_save`, `name_is_unused`); `store::{FileStep, FileStepKind::IdRename, FileChange, HEADER_TABLE}`; `rename::{before_move, after_move, judge-Regeln}`; `canon::to_canonical`; `Resolver`.
- Produces:

```rust
// textproject/kinds.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum DeleteRule { RemoveFile }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataKind { pub folder: &'static str, pub symbol: SymbolKind, pub schema_version: u32,
                      pub ref_fields: &'static [&'static str], pub delete: DeleteRule }
pub const DATA_KINDS: [DataKind; 2];                       // flags (SymbolKind::Flag, 1, &[], RemoveFile), vars
pub fn data_kind(folder: &str) -> Option<&'static DataKind>;
pub fn data_kind_of(symbol: SymbolKind) -> Option<&'static DataKind>;
/// `schemas` of the manifest for this Studio: every data kind with its version, plus `keep`'s foreign keys.
pub fn schemas(keep: &BTreeMap<String, Value>) -> BTreeMap<String, Value>;
/// Problems of the files of every data kind: a file that is no `{id, description?}` (`textproject.schema`),
/// an `id` that is not the register's id of the number (`textproject.id-mismatch`), a number outside the
/// save (`textproject.schema`), a register entry without its file (`textproject.id-dangling`, own sentence),
/// a kind whose `schemas` version is higher than known (warning `textproject.schema`, nothing refused).
pub(crate) fn check_files(project: &Project, manifest: &Manifest) -> Result<Vec<Problem>, KernelError>;
// flags/entities.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct FlagEntity { pub id: String, #[serde(default, skip_serializing_if = "String::is_empty")] pub description: String }
pub fn entity_rel(kind: FlagKind, n: u16) -> PathBuf;        // data/flags/0148.json, data/vars/16492.json
pub fn read_entity(project: &Project, kind: FlagKind, n: u16) -> Result<Option<FlagEntity>, KernelError>;
/// Every own name of `kind`: number → entity (register and file; a register entry without file has an empty description).
pub fn own_names(project: &Project, kind: FlagKind) -> Result<BTreeMap<u16, FlagEntity>, KernelError>;
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NameRequest { pub kind: FlagKind, pub number: u16, pub id: Option<String>, pub description: Option<String> }
#[derive(Debug, Clone, Serialize)]
pub struct NameReport { pub kind: FlagKind, pub number: u16, pub id: String, pub created: bool,
                        pub files: Vec<PathBuf>, pub change: Option<ChangeId>, pub ms: u64, pub warnings: Vec<String> }
/// Names `number` (`id`, required for a number without an own name) and/or sets its description, as one
/// file step (register and entity file). Refusals before anything is written: not a current text project
/// (`NOT_A_TEXT_PROJECT`), the number outside the save, an id without the kind's prefix or with other
/// characters (`rename::format_text`), an id taken (register of any kind, pret name of another number,
/// rotom's constants of another number), a number that already has another own name while `id` differs
/// (`HAS_OWN_NAME`), nothing to do (`NOTHING_TO_CHANGE`).
pub fn name_flag(store: &mut Store, req: &NameRequest) -> Result<NameReport, KernelError>;
pub fn name_flag_files(project: &Project, req: &NameRequest) -> Result<NameReport, KernelError>; // ohne Store (Tests, g4k später)
pub fn not_in_save_text(kind: FlagKind, n: u16, family: Family) -> String; // „Nummer 0x0FFF liegt außerhalb des Spielstands (Flags 0x0001 bis 0x0B5F).“
pub const HAS_OWN_NAME: &str = "Die Nummer hat schon einen eigenen Namen; ändern über „Umbenennen…“.";
pub const NOTHING_TO_CHANGE: &str = "Nichts zu ändern.";
pub fn taken_text(id: &str, by: &str) -> String;             // „FLAG_X gibt es schon (Nummer 148).“ / „FLAG_X ist der pret-Name der Nummer 148.“ / „FLAG_X kennt rotom schon als Nummer 148.“
// registry.rs
impl Symbols { pub fn with_family(self, family: Family) -> Symbols; }  // id()/number() der Arten Flag/Var: Register, dann flags::names
```

- `Symbols::save` schreibt `flags.json`/`vars.json` wie die anderen (leer `{}`); `baptize` wird für `Flag`/`Var` nie gerufen. `include::tables`: Register-Einträge der beiden Arten wie bei `Map` (ohne feste Tabelle), Schatten-Regel unverändert. `rename::judge`: für `Flag`/`Var` zusätzlich `names(family, kind).number(new)` ≠ Nummer → „{new} ist der pret-Name der Nummer {n}.“ `rename_in_json` erfasst die Entitätsdatei (`id`) ohne neuen Code (sie liegt unter `data/`). `check::dangling`: `count = numbered(…).last().map_or(0, |n| n + 1)`.

- [ ] **Step 1: Scheiternde Tests** (`TextFixture::exported()` mit `adopt`, synthetisch):
  - `kinds.rs`: `two_data_kinds_are_known_folders` (`is_known_folder("flags")`, `data_kind("vars").unwrap().symbol == SymbolKind::Var`, `data_kind("stories").is_none()`), `schemas_name_every_kind_and_keep_foreign_keys`, `check_files_reports_schema_mismatch_and_dangling` (eine Datei `{"id": "FLAG_X", "x": 1}` → `textproject.schema`; `{"id": "FLAG_Y"}` unter `0148.json` mit Register `FLAG_X: 148` → `id-mismatch`; Register `FLAG_Z: 300` ohne Datei → `id-dangling`; `schemas.flags: 2` → Warnung).
  - `entities.rs`: `naming_writes_register_entity_and_id_file` (`name_flag_files` `Flag` 148 `FLAG_HAT_MEDAILLE` „Orden erhalten“ → `data/ids/flags.json` `{"FLAG_HAT_MEDAILLE": 148}`, `data/flags/0148.json` kanonisch mit beiden Feldern, `.rotom/command_database/.g4-ids-flags.json` `{"148": "FLAG_HAT_MEDAILLE"}`; `created`), `naming_a_variable_uses_five_digits` (`Var` `0x406C` → `data/vars/16492.json`), `description_only_keeps_the_id` (zweiter Aufruf nur `description` → `!created`, eine Datei), `naming_refuses_a_number_outside_the_save` (Review Focus: `Flag` `0xFFF` → `Refused(not_in_save_text)`; `Var` `0x3FFF` ebenso), `naming_refuses_taken_and_malformed_ids` (`FLAG_UNUSED_0x0095` für 148 → pret-Name der Nummer 149; `flag_x` → `format_text`; `VAR_X` für ein Flag → Präfix; ein zweiter Name für 148 → `HAS_OWN_NAME`), `naming_refuses_the_dspre_layout` (`TextFixture::dspre()` → `NOT_A_TEXT_PROJECT`), `name_flag_over_the_store_is_one_undo_step` (Store: `name_flag`, `undo` → Register und Datei weg, ID-Datei ohne den Namen; `redo` stellt beides her).
  - `include.rs` (Tests-Modul): `own_name_is_a_second_name_and_both_compile_the_same` (Review Focus: Script A `SetFlag FLAG_UNUSED_0x0094` / `if FLAG_UNUSED_0x0094 then ClearFlag FLAG_UNUSED_0x0094 endif` / `SetVar VAR_UNUSED_0x406C, 1`; nach `name_flag` Script B mit `FLAG_HAT_MEDAILLE`/`VAR_MEIN_ZAEHLER`; `scripts::compile` → beide Member byte-gleich; die ID-Datei enthält den eigenen, nicht den pret-Namen).
  - `rename.rs`: `renaming_an_own_flag_name_follows_script_and_entity` (Review Focus: `rename_id` `FLAG_HAT_MEDAILLE` → `FLAG_ORDEN`: Register, `data/flags/0148.json` `id`, Script-Quelle, ID-Datei; ein Undo-Schritt), `renaming_to_a_pret_name_of_another_number_is_refused`, `a_pret_name_cannot_be_renamed` (`FLAG_UNUSED_0x0094` → `unknown_id_text`).
  - `check.rs`: `id_dangling_counts_the_highest_number_not_the_files` (Trainer `0500.json` entfernt: kein `id-dangling` für die letzte ID; `0927.json` entfernt: `id-dangling` für 927), `register_entries_of_flags_are_never_dangling`.
  - `manifest.rs`: `manifest_for_writes_the_data_kind_schemas` (`schemas == {"flags": 1, "vars": 1}` plus fremde Schlüssel aus `keep`); `check.rs` `reserved_hint_only_for_foreign_schema_keys`.
  - bestehende Tests grün: `export_idempotent`, `textproject_round_trip` (die beiden leeren Register kommen mit dem Export), `registry_file_per_kind` ergänzt.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features textproject:: flags::` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces.
- [ ] **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features` ganz (Textprojekt-Tests der Phase 4 grün).
- [ ] **Step 5: Doku** `docs/formats/flags_vars.md` (Kopf wie `project_git.md`: eine Tatsache über Gen4 Studio, keine Spielformat-Quelle; Abschnitte „Namen aus pret“ mit der Hauptnamen-Regel, „Bereiche“ mit der Tabelle aus `flag_ranges.yaml`, „Eigene Namen“ mit Register, Entität, ID-Datei, Regeln von `name_flag`, „Art-Beschreibung“ mit `DATA_KINDS` und `schemas`; Platzhalter-Überschriften „Nutzungsindex“ und „RAM-Lage“ für Task 4 und 8); `text_project.md` wie unter Files.
- [ ] **Step 6: Commit** `feat(textproject): own flag and variable names as register plus entity, data kinds as data, id-dangling fix` (mit Trailer).

### Task 3: Index: Ziele `Flag`/`Var`, Zugriff, deklarative Regeln, Script-Quellen

**Files:**
- Create: `crates/g4-kernel/metadata/script_flagvars.yaml`, `crates/g4-kernel/src/data/refs/flagvars.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/data/refs.rs` (`Target::Flag(u16)`, `Target::Var(u16)`, `Ref.access`, `Access`, `script_refs` ruft `flagvars::script_flagvar_refs`, `ScriptCounts`), `crates/g4-kernel/src/scripts.rs` (`AstItem::Condition`, `visit_statements` meldet Bedingungen), `crates/g4-kernel/tests/refs.rs` (Helfer `r` mit `access: Access::Read`), `apps/studio/src-tauri/src/commands/data/{dto.rs,refs.rs}` (nur `RefDto.access`, damit der Workspace baut), `docs/formats/flags_vars.md` (Abschnitt „Nutzungsindex“, Teil Scripts)

**Interfaces:**
- Consumes: Task 1 (`FlagKind`, `trainer_flag`, `ranges`); `ScriptRefTable`, `ScriptNames`, `ScriptLookup`; `checks::scripts::database`.
- Produces:

```rust
// data/refs.rs
pub enum Target { Species(u16), Move(u16), Item(u16), Trainer(u16), TrainerClass(u8), Encounter(u16), Flag(u16), Var(u16) }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum Access { Read, Write, ReadWrite, Unknown }
pub struct Ref { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub target: Target, pub access: Access }
/// What the scripts use without a target: variables from 0x8000 on and flags named through a variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ScriptCounts { pub script_local: u32, pub dynamic: u32 }
// scripts.rs
pub(crate) enum AstItem<'a> { Command {..}, Alias {..}, Declared(..), Expression(..), /// the condition of if/elseif/while, the head of match
    Condition(&'a rotom::compiler::ast::Expression) }
// data/refs/flagvars.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamRole { Flag, Var, TrainerFlag }
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlagVarParam { pub name: String, pub arg: usize, pub role: ParamRole, pub access: Access }
/// The commands of one family whose parameters name a flag, a variable or a trainer flag, from
/// `metadata/script_flagvars.yaml` and the database (type, rotom's name rule, `access`), positioned by the
/// bundled database; `params(name)` resolves legacy names and aliases like `ScriptRefTable::params`.
pub(crate) struct FlagVarTable { /* family, commands: BTreeMap<String, Vec<FlagVarParam>> */ }
impl FlagVarTable { pub(crate) fn params(&self, name: &str) -> Option<(&str, &[FlagVarParam])>; }
pub(crate) fn flagvar_table(family: Family) -> &'static FlagVarTable;
/// The flag and variable references of one parsed source (commands and conditions) and its counts.
pub(crate) fn script_flagvar_refs(file: u16, text: &str, ast: &ScriptFile, names: &ScriptNames<'_>,
                                  table: &'static FlagVarTable, family: Family) -> (Vec<Ref>, ScriptCounts);
```

- `script_flagvars.yaml` nach „Entscheidungen“ (Regeln der Script-Quellen); Kopfkommentar mit Quelle (Gutachten B §1.1, §3.2; rotom `ir_to_source.rs:28-58`, `lowerer.rs:1300-1356, 1792-1807`). `field` eines Script-Verweises ist rotoms Befehlsname (Bedingungen `if`, `while`, `match`), `slot` die Zeile. Die Zähler je Datei summiert der Index (`Built.counts: ScriptCounts`), `RefIndex::script_counts(store)` liefert sie.

- [ ] **Step 1: Scheiternde Tests** (`flagvars.rs`, synthetisch mit `TextFixture` wie `tests/refs.rs::script_store`):
  - `script_flagvars_name_existing_commands_and_params` (jeder Eintrag der YAML steht in der Datenbank seiner Familie, Parameter an der Stelle; `access_words` deckt jedes `access` der Datenbank; `name_rule.var` hat 22 Namen und `name_rule.flag` 3).
  - `platinum_flag_commands_read_or_write` (`SetFlag 148` → `Flag(148)` Write Zeile; `ClearFlag` Write; `CheckFlag` Read; `GoToIfSet`-ähnliche Befehle mit Flag-Parameter → Read).
  - `platinum_variables_take_the_database_access` (`CompareVarToValue 0x4030, 2` → `Var(0x4030)` Read; `SetVar VAR_UNUSED_0x406C, 1` → Write (Makro, über die Varianten `SetVarFromValue`: der Test nennt das, was die Datenbank für den Makro-Parameter ergibt; fehlt `access`, ist es `Unknown`, dann steht `SetVar` in der YAML mit `destVarID: write`); `AddVar` → ReadWrite).
  - `heartgold_borrows_from_platinum_with_exceptions` (`CompareVarToValue` hgss → Read geborgt; `SetVar variable, val` → Write aus der Ausnahmeliste; `CopyVar` → Write/Read; ein Befehl ohne Gegenstück → Unknown).
  - `trainer_flags_follow_the_trainer_number` (`SetTrainerFlag 3` → `Flag(0x553)` Write **und** `Trainer(3)`; `CheckTrainerFlag` → Read; `SetTrainerFlag VAR_RESULT` → nur `dynamic + 1`).
  - `conditions_count_as_reads` (`if FLAG_UNUSED_0x0094 then` → `Flag(148)` Read Feld `if`; `while VAR_UNUSED_0x406C == 0 do` → `Var` Read Feld `while`; `if CheckItem(ITEM_POTION, 1) == 1 then` → kein Flag, kein Var; `match VAR_UNUSED_0x406C` → Read Feld `match`).
  - `dynamic_flags_and_script_locals_are_counted_not_indexed` (Review Focus: `SetFlag VAR_RESULT`, `SetFlagVar`/`CheckFlagFromVar` → `dynamic`; `SetVar VAR_RESULT, 1` → `script_local`, kein `Var(0x800C)`; `CompareVarToValue 5, 1` → nichts).
  - `an_own_name_counts_like_its_number` (nach `name_flag` `FLAG_HAT_MEDAILLE`: `SetFlag FLAG_HAT_MEDAILLE` → `Flag(148)`), `an_alias_of_the_file_resolves` (`alias FLAG_UNUSED_0x0094 as MEIN_FLAG` → `SetFlag MEIN_FLAG` → `Flag(148)`).
  - `a_source_that_does_not_parse_contributes_nothing_for_flags` (Review Focus), `a_zero_flag_is_no_reference`.
  - `tests/refs.rs`: alle bestehenden Tests mit `access: Access::Read` grün; `incremental_equals_full_rebuild` unverändert (die Script-Quellen sind nicht Teil der Operationen).
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features refs` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces; `script_refs` in `refs.rs` parst einmal und ruft beide Tabellen (`ScriptRefTable`, `FlagVarTable`) über denselben `ScriptNames`.
- [ ] **Step 4: Lauf** → PASS; `cargo test --workspace --all-features` (die Studio-Tests bauen mit `RefDto.access`).
- [ ] **Step 5: Doku** `flags_vars.md` „Nutzungsindex“, Teil Scripts: Regeln, Zugriff, was nicht indexiert ist, die YAML.
- [ ] **Step 6: Commit** `feat(refs): flags and variables as targets with access from scripts, declarative rules per family` (mit Trailer).

### Task 4: Index: Events, Level-Scripts, Trainer-Flags, `usage`, gebündeltes `refresh`, Pool, gated Tests

**Files:**
- Create: `crates/g4-kernel/src/flags/pool.rs` (+ Tests), `crates/g4-kernel/tests/flags.rs` (gated)
- Modify: `crates/g4-kernel/src/data/refs.rs` (`event_refs` mit `hidden_flag`, versteckten Items, Auslösern und Trainer-Flags; `ScriptSources` mit Level-Scripts; `usage`, `script_counts`, `refresh_scripts_now`, Zeitsperre), `crates/g4-kernel/src/flags.rs` (Re-Exporte), `crates/g4-kernel/tests/refs.rs` (neue Tests), `docs/formats/flags_vars.md` („Nutzungsindex“ Teil Events, Level-Scripts, Trainer, Reservierung; „Frei“), `docs/dev-setup.md` (Tabelle `G4_TEST_PROJECT`: `tests/flags.rs`)

**Interfaces:**
- Consumes: Task 1 (`hidden_items_base`, `trainer_flag`, `pool_range`, `area_of`, `name_is_unused`, `names`), Task 2 (`own_names`), Task 3; `templates::variable_id`, `templates.yaml` `done_var`; `scripts::{source_kind, SourceKind::LevelScript, read_source}`.
- Produces:

```rust
// data/refs.rs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Usage { pub read: u32, pub write: u32, pub unknown: u32, pub sources: u32 }
impl RefIndex {
    /// Every flag or variable the index knows with its counts, one pass; ReadWrite counts for read and write.
    pub fn usage(&mut self, store: &Store, kind: FlagKind) -> Result<BTreeMap<u16, Usage>, KernelError>;
    pub fn script_counts(&mut self, store: &Store) -> Result<ScriptCounts, KernelError>;
    /// `refresh` with the script sources statted now, whatever the time gate says.
    pub fn refresh_scripts_now(&mut self, store: &Store) -> Result<Vec<AssetId>, KernelError>;
}
/// Least time between two stats of the script sources inside `refresh`.
pub const REFRESH_SCRIPTS_EVERY: Duration = Duration::from_millis(250);
// Felder der Event-Verweise: "hidden_flag" (Flag Read), "hidden_item" (Flag ReadWrite), "var" (Var Read),
// "script" (Trainer, und Flag(0x550 + n) ReadWrite); Level-Script: "var_conditions" (Var Read, Slot = Index).
// flags/pool.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pool { pub kind: FlagKind, pub range: Range, pub free: Vec<u16>, pub total: u32, pub reserved: Vec<u16> }
/// The pool of `kind`: every number of the general range that is unused in `usage`, has no own name, whose pret
/// name is unused (`name_is_unused`) and is not reserved (the template's `done_var`); ascending.
pub fn pool(project: &Project, usage: &BTreeMap<u16, Usage>, kind: FlagKind) -> Result<Pool, KernelError>;
pub fn free_flag(project: &Project, usage: &BTreeMap<u16, Usage>) -> Result<Option<u16>, KernelError>;
pub fn free_var(project: &Project, usage: &BTreeMap<u16, Usage>) -> Result<Option<u16>, KernelError>;
/// The variables the recipes of `metadata/templates.yaml` reserve for `family` (`done_var`), with the recipe's word.
pub fn reserved(family: Family) -> Vec<(u16, &'static str)>;   // ("Vorlage „Leere Region““)
pub const POOL_EMPTY: &str = "Kein freies Flag im allgemeinen Bereich.";   // Variable: „Keine freie Variable im allgemeinen Bereich.“
pub fn pool_empty_text(kind: FlagKind) -> &'static str;
```

- `ScriptSources::refresh` stattet `.rotom` und `.json` (`scripts::numbered_files(dir, Some("json"))`); `read_scripts` liest je `SourceKind`: ein Level-Script über `serde_json::from_slice::<LevelScriptDoc>` → je `var_conditions[i]` mit `var ≥ 0x4000` ein `Var(var)` Read (`var` 0 beendet die Tabelle: kein Verweis). Zeitsperre: `RefIndex { built, scripts_checked: Option<Instant> }`; `refresh` lässt den Script-Teil aus, wenn `scripts_checked` jünger als `REFRESH_SCRIPTS_EVERY` ist; `rebuild_full` und `refresh_scripts_now` setzen ihn.

- [ ] **Step 1: Scheiternde Tests:**
  - `tests/refs.rs`: `hidden_flag_reads_the_flag` (Objekt mit `hidden_flag` 0x173 → `Flag(0x173)` Read Feld `hidden_flag` Slot Objektindex), `a_hidden_item_reads_and_writes_the_base_plus_n` (BG-Event `event_type` 2, Script 8003 → pt `Flag(0x2DD)` ReadWrite Feld `hidden_item`; hgss `Flag(0x323)`; Script 8300 (über den Bereich) → nichts), `a_trigger_reads_its_variable` (`var` 0x40A5 → `Var` Read Feld `var`), `a_trainer_object_reads_and_writes_its_trainer_flag` (Script 3000 → `Trainer(1)` **und** `Flag(0x551)` ReadWrite), `zero_flags_and_variables_are_no_reference` (Review Focus), `a_level_script_condition_reads_its_variable` (`expanded/scripts/0005.json` mit zwei `var_conditions` → zwei `Var` Read Slots 0, 1 Feld `var_conditions`; Datei geändert → `refresh_scripts_now` liest sie neu), `usage_counts_read_write_and_sources_in_one_pass` (ein Flag: Objekt Read + Script Write + Script Read → `read 2, write 1, sources 2`; Trainer-Flag ReadWrite → `read 1, write 1`), `refresh_stats_scripts_at_most_every_250_ms` (zwei `refs()` direkt nacheinander → `LOOKUPS_MADE` und ein Stat-Zähler (`#[cfg(test)]` `SCRIPT_STATS` in `refs.rs`) steigen nur einmal; nach `refresh_scripts_now` erneut), `a_saved_source_shows_after_the_gate` (Quelle geändert, `refresh_scripts_now` → neuer Verweis).
  - `pool.rs`: `the_pool_is_the_general_range_minus_used_named_and_reserved` (synthetisch pt: `0x41`–`0x54F` ohne `hidden_items`; ein Objekt mit `hidden_flag` `0x96` → `0x96` nicht frei; `FLAG_UNUSED_0x0094` ohne Verweis → frei; ein Flag mit pret-Namen `FLAG_HIDE_…` ohne Verweis → nicht frei; nach `name_flag` 148 → nicht frei), `a_used_unused_flag_is_not_free` (Review Focus), `free_var_skips_the_templates_done_var` (pt `0x406C` reserviert mit „Vorlage „Leere Region““; `free_var` ≠ `0x406C`), `map_local_and_trainer_flags_are_never_free`, `an_empty_pool_is_none` (alle Nummern belegt → `None`).
  - gated `tests/flags.rs` (`vanilla_copy()`, einmal `adopt` je Test-Lauf über eine `OnceLock`-Kopie wie `tests/textproject_source.rs`, Release):
    - `vanilla_flag_names_count_the_metadata`: `names(family, Flag).len()` = 4 097 (pt) bzw. 2 244 (hgss), Variablen 307 / 386; druckt `names <game_code> flags=<n> vars=<n>`.
    - `vanilla_usage_matches_a_recount_of_the_sources`: Vergleichszählung im Test ohne den Index: Objekte mit `hidden_flag ≠ 0` (uxie `EventFile` je Member) = Verweise mit Feld `hidden_flag`; BG-Events Typ 2 mit Script 8000+n im Bereich = Feld `hidden_item`; Auslöser mit `var ≠ 0` = Feld `var`; Objekte mit Trainer-Script = Feld `script` mit Ziel `Flag`; `var_conditions` mit `var ≠ 0` aller `expanded/scripts/*.json` = Feld `var_conditions`; Zeilen der `.rotom`-Quellen, die mit `SetFlag`/`ClearFlag`/`CheckFlag` und einem Argument beginnen, das Zahl oder `FLAG_…`-Name ist = Verweise mit diesen Feldern; druckt `usage <game_code> hidden_flag=<n> hidden_item=<n> trigger=<n> trainer=<n> level=<n> setflag=<n> script_local=<n> dynamic=<n> flags_distinct=<n> vars_distinct=<n>` (Erwartung aus Gutachten B §1.2/§1.3 für DE: Objekte 1 613 / 1 108, versteckte Items 262 / 241, Auslöser 186 / 195, Level-Scripts 207 / 230, Trainer 403 / 390, als Druck, nicht als Assertion: US weicht ab).
    - `vanilla_flag_index_is_fast`: warmer `rebuild_full` ≤ 1 000 ms, 200 × `refs(Target::Flag(0x550 + i))` ≤ 50 ms, `usage(Flag)` ≤ 50 ms (strikt nur mit `G4_TIMING_STRICT=1`); druckt `index <game_code> warm_ms=<n> refs200_ms=<n> usage_ms=<n>`.
    - `vanilla_pool_counts_the_free_numbers`: `pool(Flag).free.len()` zwischen 100 und 170 und `pool(Var).free.len()` zwischen 10 und 100 (Gutachten A: 135 / 152 und 21 / 83); kein freies Flag ist kartenlokal, im Trainer- oder System-Bereich; `0x96` (pt) nicht frei; druckt `pool <game_code> flags=<n> vars=<n> first_flag=<hex> first_var=<hex>`.
    - `vanilla_own_name_compiles_byte_equal`: `free_flag` → n; `name_flag` `FLAG_GATE_3D1`; eine Quelle mit `if FLAG_GATE_3D1 then … endif` und `SetFlag FLAG_GATE_3D1` kompiliert zu denselben Bytes wie mit dem pret-Namen; Register, Entität und ID-Datei da; `rename_id` → `FLAG_GATE_3D1_B` zieht Quelle und Entität nach, `undo` stellt alles her; druckt `own <game_code> flag=<hex> compile_ms=<n> rename_ms=<n>`.
- [ ] **Step 2: Lauf** `cargo test -p g4-kernel --all-features --test refs flags::pool` → FAIL.
- [ ] **Step 3: Umsetzen** wie unter Interfaces.
- [ ] **Step 4: Lauf** → PASS; gated einmal auf Platin DE: `G4_TEST_PROJECT="<Roms>/_testkopien/3d-1-pt_DSPRE_contents" cargo test --release -p g4-kernel --all-features --test flags -- --nocapture --test-threads 1`; `vanilla_index_builds_under_a_second` bleibt ≤ 1 000 ms (die Level-Scripts kommen dazu).
- [ ] **Step 5: Doku** `flags_vars.md` wie unter Files; `dev-setup.md`.
- [ ] **Step 6: Commit** `feat(refs): flags and variables from events, level scripts and trainer flags, usage in one pass, gated refresh, free pool` (mit Trailer).

### Task 5: App-Befehle, Event-Felder im Patch, MCP

**Files:**
- Create: `apps/studio/src-tauri/src/commands/flags.rs` (+ Tests, gated Test)
- Modify: `src/commands/mod.rs`, `src/commands/events.rs` (`EventPatch` + `hidden_flag`, `var`, `value`; `ObjectDto.hidden_flag`, `CoordDto.{var, value}`; `update_event`/`add_event` nehmen sie), `src/commands/data/dto.rs` (`NavTargetDto::Flag`), `src/commands/data/refs.rs` (Quellname und Weg für `AssetKind::Script` als Level-Script, `access` im DTO), `src/lib.rs` (Handler anhängen), `src/commands/dispatch.rs` (anhängen, Beispielargumente), `src/mcp/access.rs`, `src/mcp/catalog.rs`, `src/mcp/tools.rs` (nichts in `NOT_IN_BATCH`), `src/mcp/symbolic.rs` (`kind_named` + `flag`/`var`, zwei `ArgRef`), `docs/mcp.md`

**Interfaces:**
- Consumes: Task 1 bis 4; `commands::open_project_arc`, `lock`/`session`; `Resolver`.
- Produces:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct FlagEntryDto { pub number: u16, pub name: Option<String>, pub aliases: Vec<String>, pub own_id: Option<String>,
                          pub description: Option<String>, pub area: Area, pub usage: Usage, pub free: bool,
                          pub reserved: Option<String>, pub places: u32 }   // places: distinct headers (map-local), else 0
#[derive(Debug, Clone, Serialize)]
pub struct FlagListDto { pub kind: FlagKind, pub entries: Vec<FlagEntryDto>, pub pool_free: u32, pub pool_total: u32,
                         pub counts: ScriptCounts, pub text_project: bool, pub family: String, pub ms: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct FlagRefDto { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub access: Access,
                        pub label: String, pub source_name: String, pub nav: NavTargetDto, pub header: Option<u16>, pub header_name: Option<String> }
#[derive(Debug, Clone, Serialize)]
pub struct FlagUsageDto { pub kind: FlagKind, pub number: u16, pub name: Option<String>, pub own_id: Option<String>,
                          pub description: Option<String>, pub area: Area, pub usage: Usage, pub reserved: Option<String>,
                          pub refs: Vec<FlagRefDto>, pub ms: u64 }
#[derive(Debug, Clone, Serialize)] pub struct FreeFlagDto { pub kind: FlagKind, pub number: u16, pub name: Option<String>, pub pool_free: u32 }
pub fn list_flags_impl(state: &AppState, kind: FlagKind) -> Result<FlagListDto, CmdError>;
pub fn flag_usage_impl(state: &AppState, kind: FlagKind, number: u16) -> Result<FlagUsageDto, CmdError>;
pub fn free_flag_impl(state: &AppState, kind: FlagKind) -> Result<FreeFlagDto, CmdError>;   // leer: Fehlercode pool_empty
pub fn name_flag_impl(state: &AppState, req: NameRequest) -> Result<NameReport, CmdError>;  // Refused → not_text_project / invalid_id / taken / refused
// NavTargetDto / NavTarget (angehängt): Flag { flag_kind: FlagKind, number: u16 }   (TS: { kind: "flag"; flagKind: "flag" | "var"; number: number })
```

- Alle vier unter dem Sitzungsschloss (der Index gehört dem Store); `list_flags` baut die Einträge aus `names` (jede Nummer mit Namen) ∪ `usage` (jede Nummer mit Verweis) ∪ `own_names`; `free` aus `pool`. `FlagRefDto.label` wie `RefDto` („Objekt 3 · Sichtbarkeits-Flag“, „Zeile 12 · SetFlag“, „Bedingung 1 · var_conditions“, „Trainer 0928 · Script“), `source_name` („Eventdatei 0394“, „Scriptdatei 0101“, „Level-Script 0005“), `nav` (`event`, `script` mit Zeile, `header` für ein Level-Script, `trainer`), `header` für Event- und Script-Quellen über die Header-Tabelle (erster Header, der die Datei nutzt). `name_flag_impl` ruft nach dem Schritt `session.after_write`/`files_written()` wie `rename_id`.
- Zugriff (anhängen): `("list_flags", Read)`, `("flag_usage", Read)`, `("free_flag", Read)`, `("name_flag", Write)`. Katalog (englisch):
  - `list_flags` „List flags or variables“: „Lists every flag (kind flag) or variable (kind var) the game names or the project uses: number, the pret name and its aliases, the project's own id and description (text projects), its area (map_local, general, hidden_items, trainer, system, daily, temp, object_gfx, script_local, outside), how often scripts, events, level scripts and trainers read and write it, whether it is free (general area, unused, no own name, pret name unused, not reserved by a recipe) and how many places a map-local number is used at. Also the size of the free pool and the counts of script-local variables and flags named through a variable, which are not indexed.“; Argumente `{ kind: "flag" | "var" }`.
  - `flag_usage` „Where a flag or variable is used“: „Every reference to one flag or variable with its access (read, write, read_write, unknown), the source (event file and event, script file and line, level script and condition, trainer), the header it belongs to and the place in the app it opens. number takes a number or a name (FLAG_…, VAR_…, also an own id). “ + `data_note!()`; Argumente `{ kind, number }`.
  - `free_flag` „A free flag or variable“: „The smallest free number of the general area (unused in the index, no own name, pret name unused, not reserved); pool_empty when there is none. Take it with name_flag or write it into hidden_flag or a trigger.“; Argumente `{ kind }`.
  - `name_flag` „Name a flag or variable“: „Gives a number of a text project its own id (FLAG_… or VAR_…, A-Z, 0-9 and _, at most 64 characters, not taken) and/or a description, written to data/ids/<kind>.json and data/<kind>/NNNN.json and to rotom's id files, as one undo step. The pret name stays valid. A number outside the save, a taken id or the DSPRE layout are refused before anything is written; a number with another own id is renamed with rename_id.“; Argumente `{ kind, number, id?, description? }`.
  - `rename_id`: Beschreibung um „FLAG_, VAR_ (own flag and variable names)“.
- Symbolisch: `arg("flag_usage", "/number", FromKindField("/kind"))`, `arg("name_flag", "/number", FromKindField("/kind"))`; `kind_named` `"flag"` → `SymbolKind::Flag`, `"var"` → `SymbolKind::Var`; der `Resolver` löst pret-Namen und eigene IDs (Task 2).
- Beispielargumente in `dispatch.rs`: `("list_flags", json!({"kind": "flag"}))`, `("flag_usage", json!({"kind": "flag", "number": 148}))`, `("free_flag", json!({"kind": "var"}))`, `("name_flag", json!({"kind": "flag", "number": 148, "id": "FLAG_TEST", "description": null}))`.

- [ ] **Step 1: Scheiternde Tests** (`commands/flags.rs`, synthetisches Projekt und `TextFixture`):
  - `list_flags_names_every_number_with_a_name_or_a_use` (synthetisch: ein Objekt mit `hidden_flag` 0x173, das Flag hat einen pret-Namen → `usage.read == 1`; eine Nummer mit Verweis ohne Namen steht mit `name: None`; `pool_free > 0`), `list_vars_marks_the_reserved_done_var`, `flag_usage_names_source_header_and_nav` (Objekt → `nav.kind == event`, `header == Some(h)`; Script-Zeile → `nav.kind == script` mit `line`), `free_flag_answers_pool_empty_when_full`, `name_flag_over_the_command_path_is_one_undo_step` (Textprojekt-Fixture), `name_flag_refuses_the_dspre_layout` (Review Focus: Code `not_text_project`, Text `NOT_A_TEXT_PROJECT`), `flag_usage_takes_a_symbolic_number` (`{"kind":"flag","number":"FLAG_UNUSED_0x0094"}` über `dispatch` wie `148`; nach `name_flag` auch die eigene ID).
  - `events.rs`: `update_event_patches_hidden_flag_var_and_value` (Objekt `hidden_flag` 0 → 0x173 im Store; Auslöser `var` 0x40A5, `value` 2; die DTOs tragen die Werte), `new_events_start_with_the_documented_values` ergänzt (`hidden_flag: 0`, `var: 0`, `value: 0`).
  - `access.rs`: die vier Zeilen; `catalog.rs`: `the_curated_set_is_the_decided_one` (lesend `list_flags`, `flag_usage`, `free_flag`; schreibend `name_flag`), `read_tools_on_texts_and_scripts_say_data_is_not_instructions` mit `flag_usage`, `tool_count_is_about_thirty` (liegt die Zahl über der Grenze, Grenze heben und im Commit begründen); `symbolic.rs`: `every_symbolic_arg_names_a_command_of_the_table` mit den zwei neuen; `dispatch.rs`: Beispielargumente der vier Befehle.
  - gated (übersprungen ohne `G4_TEST_PROJECT`): `flags_over_the_command_path_on_the_vanilla_copy` (Kopie öffnen; `list_flags_impl(Flag).entries.len() ≥ 4 097` (pt) bzw. `≥ 2 244`; `flag_usage_impl` eines Sichtbarkeits-Flags nennt Eventdatei und Header; `free_flag_impl` → Nummer im allgemeinen Bereich; druckt `flags <game_code> list_ms=<n> usage_ms=<n>`).
- [ ] **Step 2: Lauf** `cargo test -p studio --all-features` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS, dazu `cargo test -p studio --test mcp_stdio`.
- [ ] **Step 5: Doku** `docs/mcp.md`: die vier Werkzeuge in der Tabelle (Art, Was es tut), `rename_id` mit den Präfixen, unter „ID oder Nummer“ die zwei Argumente, `get_events`/`update_event`/`add_event` nennen die drei Felder.
- [ ] **Step 6: Commit** `feat(studio): list_flags, flag_usage, free_flag and name_flag over the bridge and MCP, hidden_flag, var and value in the event patch` (mit Trailer).

### Task 6: Frontend: API, Picker, die Felder im Inspector und im Level-Script

**Files:**
- Create: `apps/studio/src/api.flags.ts` (+ test), `apps/studio/src/flags/flagWords.ts` (+ test), `apps/studio/src/flags/useFlagList.ts` (+ test), `apps/studio/src/flags/FlagPicker.tsx` (+ `.module.css`, test), `apps/studio/src/flags/FlagField.tsx` (+ test)
- Modify: `apps/studio/src/api.ts` (`export * from "./api.flags";` und die Felder in `EventPatch`, `ObjectDto`, `CoordDto`), `apps/studio/src/api.data.ts` (`NavTargetDto` + `flag`), `apps/studio/src/workbench/navigation.ts` (`NavTarget` + `flag`), `apps/studio/src/workbench/inspector/eventFields.ts` (anhängen: `pick`, drei Felder, `PATCH_KEYS`), `apps/studio/src/workbench/inspector/EventEditor.tsx` (+ test: `FlagField` für Felder mit `pick`), `apps/studio/src/workbench/inspector/LevelScriptSection.tsx` (+ test: Variable als `FlagField`), `apps/studio/src/ui/primitives/icons.ts` (`flag: Flag`, Phosphor), `apps/studio/src/workbench/inspector/technical.ts` (die neuen Schlüssel erscheinen unter „Technisch“ wie die anderen; nur falls der Test es verlangt)

**Interfaces:**
- Consumes: Task 5 (Befehle, DTOs); `Picker`, `PickerItem`, `Reference`, `Button`, `NumberField`, `InspectorRow`, `Tooltip`; `useDataArea`-Muster von `useRefs.ts` für das Nachladen.
- Produces (TS):

```ts
// api.flags.ts
export type FlagKind = "flag" | "var";
export type FlagArea = "map_local" | "general" | "hidden_items" | "trainer" | "system" | "daily" | "temp" | "object_gfx" | "script_local" | "outside";
export type FlagAccess = "read" | "write" | "read_write" | "unknown";
export interface FlagUsageCounts { read: number; write: number; unknown: number; sources: number }
export interface FlagEntryDto { number: number; name: string | null; aliases: string[]; own_id: string | null; description: string | null; area: FlagArea; usage: FlagUsageCounts; free: boolean; reserved: string | null; places: number }
export interface FlagListDto { kind: FlagKind; entries: FlagEntryDto[]; pool_free: number; pool_total: number; counts: { script_local: number; dynamic: number }; text_project: boolean; family: string; ms: number }
export interface FlagRefDto { source: AssetId; field: string; slot: number | null; access: FlagAccess; label: string; source_name: string; nav: NavTargetDto; header: number | null; header_name: string | null }
export interface FlagUsageDto { kind: FlagKind; number: number; name: string | null; own_id: string | null; description: string | null; area: FlagArea; usage: FlagUsageCounts; reserved: string | null; refs: FlagRefDto[]; ms: number }
export interface NameReport { kind: FlagKind; number: number; id: string; created: boolean; files: string[]; change: number | null; ms: number; warnings: string[] }
export const listFlags: (kind: FlagKind) => Promise<FlagListDto>;
export const flagUsage: (kind: FlagKind, number: number) => Promise<FlagUsageDto>;
export const freeFlag: (kind: FlagKind) => Promise<{ kind: FlagKind; number: number; name: string | null; pool_free: number }>;
export const nameFlag: (req: { kind: FlagKind; number: number; id?: string; description?: string }) => Promise<NameReport>;
// flags/flagWords.ts
export function hex4(n: number): string;                             // "0x0094"
export function numberText(n: number): string;                       // "0x0094 · 148"
export function displayName(e: { name: string | null; own_id: string | null }, kind: FlagKind, n: number): string; // own_id, sonst name, sonst "Flag 0x0094" / "Variable 0x4030"
export function areaWord(area: FlagArea): string;                    // kartenlokal, allgemein, versteckte Items, Trainer, System, Tagesflags, temporär, Objektgrafik, Script-lokal, außerhalb
export function usageText(u: FlagUsageCounts): string;               // "gesetzt 3 · geprüft 1" (+ " · unklar 2" nur > 0); "unbenutzt" bei 0/0/0
export function accessWord(a: FlagAccess): string;                   // "setzt", "prüft", "setzt und prüft", "unklar"
export function kindWord(kind: FlagKind): string;                    // "Flag", "Variable"
export function pickerItems(list: FlagListDto, query: string): PickerItem[]; // "Keins (0)" zuerst, dann Treffer (Entscheidungen „Picker“)
// flags/useFlagList.ts
export function useFlagList(kind: FlagKind, deps: { project: ProjectInfo | null; revision: number }): { list: FlagListDto | null; error: string | null; reload: () => void };
// flags/FlagPicker.tsx
export function FlagPicker(props: { open: boolean; kind: FlagKind; value: number; onOpenChange: (open: boolean) => void; onPick: (number: number) => void; onShowInDirectory?: (number: number) => void; project: ProjectInfo | null; revision: number }): JSX.Element;
// flags/FlagField.tsx
export function FlagField(props: { kind: FlagKind; label: string; value: number; onCommit: (n: number) => void; disabled?: boolean; depth: Depth; project: ProjectInfo | null; revision: number; onShowInDirectory?: (number: number) => void }): JSX.Element;
```

- `FlagField`: eine `InspectorRow` mit einem `Reference`-Chip der Fläche `geschichten` (Label `displayName`, `tech` `hex4`; Klick öffnet den Picker; `data-flag-field=<key>`), daneben „Wählen…“ (quiet S); in Tüfteln dazu das `NumberField` mit Dez/Hex; `value` 0 zeigt „Keins“. `FlagPicker` Titel „Flag wählen“ / „Variable wählen“, Platzhalter „Name oder Nummer“, Filter die Bereiche, Fuß zwei quiet Tasten: „Freies Flag nehmen“ (`freeFlag`, bei `pool_empty` gesperrt mit dem Satz des Kerns als Tooltip) und „Im Verzeichnis zeigen“ (nur mit `onShowInDirectory`), `data-flag-picker`.
- Texte: Objekt „Sichtbarkeits-Flag“ (Hinweis unter dem Feld in Bauen: „Die Figur ist da, solange das Flag 0 ist.“), Auslöser „Variable“, „Wert“ (Hinweis: „Der Auslöser feuert, wenn die Variable diesen Wert hat.“); Level-Script „Variable“ wie heute.

- [ ] **Step 1: Scheiternde Tests:** `api.flags.test.ts` `flag_commands_send_the_documented_arguments`; `flagWords.test.ts` `numbers_show_hex_and_decimal`, `display_name_prefers_the_own_id`, `usage_text_names_set_checked_and_unclear`, `picker_items_start_with_none_and_match_name_alias_and_number`; `useFlagList.test.tsx` `reads_on_open_and_after_a_revision`, `drops_an_older_answer`; `FlagPicker.test.tsx` `the_picker_offers_none_and_keeps_zero` (Review Focus), `free_flag_takes_the_pool_number`, `empty_pool_locks_the_footer_with_the_reason`, `show_in_directory_calls_back`; `FlagField.test.tsx` `shows_the_name_and_opens_the_picker`, `tinker_adds_the_number_field`; `eventFields.test.ts` `object_has_hidden_flag_and_coord_has_var_and_value` (Reihenfolge, `pick`, `PATCH_KEYS`), `eventPatch_carries_the_new_fields`; `EventEditor.test.tsx` `a_pick_field_renders_the_flag_field_and_commits_the_number`; `LevelScriptSection.test.tsx` `the_variable_is_a_flag_field` (Bestand und „Neue Variable“); `api.checks.test.ts` (der Typ-Test `NavTargetDto` = `NavTarget` ohne `sceneGroup` bleibt grün mit `flag`).
- [ ] **Step 2: Lauf** `cd apps/studio && pnpm test` → FAIL; **Step 3: Umsetzen** (CSS-Module nur mit Tokens); **Step 4: Lauf** → PASS, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh` grün.
- [ ] **Step 5: Commit** `feat(studio): flag and variable pickers, the fields Sichtbarkeits-Flag, Variable and Wert in the inspector and the level script` (mit Trailer).

### Task 7: Frontend: Fläche „Flags und Variablen“ in Technik, Palette, Benennen und Umbenennen

**Files:**
- Create: `apps/studio/src/flags/FlagsSurface.tsx` (+ `.module.css`, test), `apps/studio/src/flags/FlagList.tsx` (+ test), `apps/studio/src/flags/FlagDetail.tsx` (+ test), `apps/studio/src/flags/NameFlagDialog.tsx` (+ test), `apps/studio/src/flags/useFlagUsage.ts` (+ test), `apps/studio/src/flags/flagFilter.ts` (+ test)
- Modify: `apps/studio/src/technik/TechnikArea.tsx` (+ test: Eintrag `flags` am Ende von `ENTRIES`, `subtitles.flags`, Mount der Fläche mit Props), `apps/studio/src/App.tsx` (Props der Fläche: `goTo`, `revision`, Auswahl aus `NavTarget` `flag`; `onShowInDirectory` der Picker → Technik › flags), `apps/studio/src/app/paletteEntries.ts` (+ test: „Flags und Variablen“, Symbol `flag`, nur mit Projekt), `apps/studio/src/ui/gallery/pages/ContentPage.tsx` (Beispiel: eine Zeile der Flag-Liste und ein Detail mit zwei Wegen)

**Interfaces:**
- Consumes: Task 5, 6; `RenameIdDialog` (4.3b), `renameEntry`; `ListRow`, `FilterChips`, `Checkbox`, `TextField`, `Badge`, `Section`, `Notice`, `EmptyState`, `Tooltip`, `Button`, `Dialog`; `NavTarget`, `goTo`.
- Produces (TS):

```ts
// flags/flagFilter.ts
export interface FlagFilter { kind: FlagKind; area: FlagArea | "all"; unusedOnly: boolean; freeOnly: boolean; ownOnly: boolean; query: string }
export const DEFAULT_FILTER: FlagFilter;              // flag, all, false, false, false, ""
export function areasOf(kind: FlagKind): FlagArea[];  // Flag: map_local, general, hidden_items, trainer, system, daily, temp(hgss); Var: map_local, object_gfx, general, script_local
export function filterEntries(entries: readonly FlagEntryDto[], filter: FlagFilter): FlagEntryDto[]; // Suche über own_id, name, aliases, hex4, dezimal
export function groupRefsByHeader(refs: readonly FlagRefDto[]): { header: number | null; name: string; refs: FlagRefDto[] }[];
// flags/useFlagUsage.ts
export function useFlagUsage(kind: FlagKind, number: number | null, deps: { project: ProjectInfo | null; revision: number }): { usage: FlagUsageDto | null; error: string | null; busy: boolean };
// flags/FlagsSurface.tsx
export type FlagsSurfaceProps = { project: ProjectInfo | null; revision: number; selected: { kind: FlagKind; number: number } | null; onSelect: (sel: { kind: FlagKind; number: number } | null) => void; goTo: (t: NavTarget) => void; depth: Depth; readOnly: boolean; onError: (e: unknown) => void };
export function FlagsSurface(props: FlagsSurfaceProps): JSX.Element;
export function flagsSubtitle(list: FlagListDto | null): string;   // „4.097 Flags · 135 frei“, „Wird gelesen …“
// flags/NameFlagDialog.tsx
export function NameFlagDialog(props: { kind: FlagKind; number: number; name: string | null; current: { id: string | null; description: string | null }; onClose: () => void; onDone: (r: NameReport) => void }): JSX.Element;
```

- Texte wörtlich:

| Stelle | Text |
|---|---|
| Technik-Eintrag (`ENTRIES`, Symbol `flag`) | „Flags und Variablen“; Untertitel `flagsSubtitle`: „4.097 Flags · 135 frei“, „Wird gelesen …“, ohne Projekt „Kein Projekt“ |
| Fläche `[data-technik=flags]`, Titel / Satz | „Flags und Variablen“ / „Welche Flags und Variablen das Spiel kennt und wo Scripts, Figuren, Auslöser, Level-Scripts und Trainer sie setzen oder prüfen.“; im DSPRE-Layout dazu `Notice` (Info) „Eigene Namen gibt es im Textprojekt: Technik › Textprojekt › „Als Textprojekt einrichten…“.“ |
| Filterzeile `[data-flag-filter]` | `FilterChips` „Flags \| Variablen“; `FilterChips` „Alle“ + `areaWord` je Bereich; Kästchen „nur unbenutzt“, „nur frei“, „nur mit eigenem Namen“; Suche „Name oder Nummer suchen“ |
| Liste `[data-flag-list]`, Zeile `[data-flag-row=<nummer dezimal>]` | Label `displayName` (eigener Name fett), darunter grau in Mono der pret-Name (wenn ein eigener da ist) und `numberText`; rechts `usageText`, bei kartenlokalen „an 54 Orten“, bei reservierten `Badge` (Info) „reserviert“, bei freien `Badge` (Erfolg) „frei“; Spalte „Live“ `[data-flag-live]` „–“ (Tooltip „Kommt mit dem Spielmodus (3d.9)“); leer „Nichts passt zu Filter und Suche.“; Fuß „{n} von {m} · Script-lokale Variablen {counts.script_local} Vorkommen, Flags aus Variablen {counts.dynamic} (nicht im Index)“ |
| Detail ohne Auswahl `[data-flag-detail]` | „Wähle links ein Flag oder eine Variable.“ |
| Detail, Kopf | Titel `displayName`; Zeile `numberText` · `areaWord` (· „Vorlage „Leere Region““ bei reserviert); Beschreibung als Text oder „Keine Beschreibung“ (sekundär); Tasten: mit eigenem Namen „Umbenennen…“ (öffnet `RenameIdDialog`) und „Beschreibung…“ (öffnet `NameFlagDialog` ohne ID-Feld), ohne eigenen Namen „Als eigenes Flag benennen…“ / „Als eigene Variable benennen…“ (öffnet `NameFlagDialog`); im DSPRE-Layout gesperrt (fokussierbar) mit Tooltip `NOT_A_TEXT_PROJECT` + „ (Technik › Textprojekt)“; außerhalb des Spielstands gesperrt mit „Liegt außerhalb des Spielstands.“ |
| Detail, Abschnitte | „Wo gesetzt“ (`access` write oder read_write), „Wo geprüft“ (read oder read_write), „Unklar“ (unknown, nur wenn da); je Zeile `[data-flag-use]` ein `Reference`-Chip (Fläche `welt` für Events, Scripts und Level-Scripts, `pokemon` für Trainer) mit `label` und `tech` `source_name`, Klick `goTo(nav)`; kartenlokal nach Header gruppiert (Überschrift `header_name` „(0411)“, in Tüfteln `MAP_…`), sonst nach `source_name`; leer „Nirgends gesetzt.“ / „Nirgends geprüft.“ / Hinweis „Gelesen, aber nirgends gesetzt: die Engine oder ein Script über eine Variable setzt es, oder nie.“ bei Read ohne Write |
| `NameFlagDialog` `[data-name-flag-dialog]` | Titel „Als eigenes Flag benennen“ / „Als eigene Variable benennen“ / „Beschreibung“; Felder „Eigener Name“ (vorbelegt `FLAG_` bzw. `VAR_`; Hilfe „A–Z, 0–9 und _; der pret-Name {name} bleibt gültig.“) und „Beschreibung“ (mehrzeilig, optional); Ablehnung des Kerns als `Notice` (Fehler) unter dem Feld; Tasten „Abbrechen“, „Benennen“ (primär) bzw. „Speichern“; danach Meldung „{id} benannt (ein Schritt im Verlauf).“ / „Beschreibung gespeichert.“ |
| Palette | „Flags und Variablen“ (Symbol `flag`, ohne Projekt gesperrt mit `NO_PROJECT`) |
| Weg aus den Pickern | „Im Verzeichnis zeigen“ → Technik › flags mit der Nummer gewählt |

- [ ] **Step 1: Scheiternde Tests:** `flagFilter.test.ts` `filters_by_kind_area_unused_free_own_and_query` (Suche findet Alias `VARS_START`, `0x0094`, `148`), `groups_refs_by_header`; `useFlagUsage.test.tsx` `reads_on_selection_and_after_a_revision`; `FlagList.test.tsx` `rows_show_own_name_pret_name_number_and_usage`, `the_live_column_is_empty_with_its_tooltip`, `map_local_rows_count_places`; `FlagDetail.test.tsx` `set_and_checked_sections_with_ways` (Klick ruft `goTo` mit dem `nav`), `map_local_groups_by_header`, `read_without_write_shows_the_hint`, `the_head_locks_naming_in_the_dspre_layout` (Review Focus), `own_name_offers_rename_and_description`; `NameFlagDialog.test.tsx` `names_with_prefix_and_shows_the_kernel_refusal`, `description_only_saves`; `FlagsSurface.test.tsx` `filter_list_and_detail_work_together`, `selection_follows_the_nav_target`; `TechnikArea.test.tsx` `flags_is_the_ninth_entry_with_its_subtitle` (die bestehenden Tests zählen neun Einträge); `paletteEntries.test.ts` `flags_and_variables_only_with_a_project`; `App.test.tsx` `show_in_directory_opens_technik_flags_on_the_number`.
- [ ] **Step 2: Lauf** → FAIL; **Step 3: Umsetzen** (Liste und Detail nebeneinander ab 760 px Containerbreite wie Technik › Änderungen, sonst untereinander; CSS-Module nur mit Tokens); **Step 4: Lauf** → PASS, `pnpm build`, Stil- und Token-Prüfung grün.
- [ ] **Step 5: Blick in der Test-Umgebung** (`docs/agent-gate.md`, „Je Lauf“; Kopie `_testkopien/3d-1-ui_DSPRE_contents` mit `3d-1-ui.nds`, Basis zugeordnet, Scripts übernommen; dazu `3d-1-ui-text` als Textprojekt): Fläche mit Filtern, Zeile mit eigenem Namen, Detail mit „Wo gesetzt“/„Wo geprüft“ (kartenlokal gruppiert), DSPRE-Sperre, Picker im Inspector einer Figur und am Level-Script, Dialog „Als eigenes Flag benennen“; Hell, Dunkel, Kompakt; Screenshots nach `target/agent/gate-<datum>/3d-1/blick-<fläche>-<zustand>-<thema>.png` (nicht ins Repo).
- [ ] **Step 6: Commit** `feat(technik): the surface Flags und Variablen with list, filters, usage detail and naming` (mit Trailer).

### Task 8: `g4emu`: Spielstand-Block, Flag- und Variablen-Schritte, Szenarien

**Files:**
- Create: `scripts/agent/scenarios/pt-flag.yaml`, `scripts/agent/scenarios/hg-flag.yaml`
- Modify: `scripts/g4emu/g4emu/games.yaml` (`save_block` je Code), `scripts/g4emu/g4emu/games.py` (`SaveBlock`, `Game.save_block`, `read_flag`, `read_var`, `write_flag`, `write_var`), `scripts/g4emu/g4emu/steps.py` (`ARITY`, `HANDLERS`, sechs Handler), `scripts/g4emu/tests/test_games.py`, `scripts/g4emu/tests/test_steps.py`, `scripts/g4emu/tests/conftest.py` (`put_save_block`), `crates/g4-kernel/src/flags/save_layout.rs` (das `#[ignore]` von Task 1 fällt), `docs/agent-gate.md` („Werkzeuge“: die sechs Schritte in der Schrittliste; Abschnitt „Phase 3d.1“ kommt mit Task 10), `docs/formats/flags_vars.md` („RAM-Lage“)

**Interfaces:**
- Consumes: Task 1 (`save_layout.yaml`, dieselben Werte); `read_location`-Muster (`games.py`); `_require_game`, `_int`, `StepFailed`.
- Produces (Python):

```python
@dataclass(frozen=True)
class SaveBlock:
    pointer: int; header: int; vars: int; vars_count: int; flags: int; flags_bytes: int
# Game bekommt `save_block: SaveBlock | None`; load_games liest `save_block` (alle sechs Schlüssel Pflicht, sonst ValueError "games.yaml: <code>: save_block braucht …")
def block_start(ram, game) -> int | None       # (u32 @ pointer) − RAM_BASE + header; None, solange der Zeiger ungültig ist
def read_flag(ram, game, n: int) -> bool       # Bit n % 8 von Byte flags + n // 8; ValueError außerhalb 0 ≤ n < flags_bytes × 8
def write_flag(ram: bytearray | memoryview, game, n: int, value: bool) -> None
def read_var(ram, game, v: int) -> int         # u16 little-endian an vars + 2 × (v − 0x4000); ValueError außerhalb 0x4000 ≤ v < 0x4000 + vars_count
def write_var(ram, game, v: int, value: int) -> None   # 0 ≤ value ≤ 0xFFFF
# steps.py: ARITY "flag": (1, 1), "set-flag": (2, 2), "var": (1, 1), "set-var": (2, 2), "expect-flag": (2, 2), "expect-var": (2, 2)
# Zeilen: flag → {"flag": n, "value": 0|1}; set-flag → {"flag": n, "value": v}; var → {"var": v, "value": w}; set-var → {"var": v, "value": w};
# expect-flag/expect-var: wie flag/var, StepFailed("Flag 0x0173 ist 0 statt 1") / ("Variable 0x40A5 ist 1 statt 0") bei Abweichung;
# ohne Spielstand (block_start None): StepFailed("kein gültiger Spielstand im RAM"); `n`/`v` dezimal oder 0x…; ohne save_block: StepFailed("save_block fehlt in games.yaml")
```

- Szenario `pt-flag.yaml` (Kopfkommentar wie die Trainer-Szenarien; Platzhalter `NEUER_HEADER`, `X_FIGUR`, `Z_FIGUR`): `game: CPUD`, `mcp: [--allow-irreversible]`; `edits`: `apply_header_patch`, `adopt_text_project` (statt `adopt_scripts`: der eigene Name braucht das Textprojekt), `create_area` wie `pt-trainer-begegnung.yaml` (Vorlage 342, zwei Zellen, `with_headers`, neuer Header, Eventdatei `New`) → `area`; `get_header` → `hdr`; `new_script_file` → `datei`; `set_header` mit `script_file_id`; `free_flag {kind: flag}` → `frei`; `name_flag {kind: flag, number: "${frei.number}", id: FLAG_SZENARIO_FIGUR, description: "Figur im Szenario"}`; `add_event` Objekt bei (`X_FIGUR`, `Z_FIGUR`) mit `graphics_id` 6, `movement_type` 15, `dir` 1, `script` 2, `hidden_flag: "${frei.number}"`; `add_event` Warp 0 bei (44, 21) nach 415/0 mit `set_permission` wie im Trainer-Szenario; `update_event` Eventdatei 394 Warp 0 → `${area.header}`; `save`; `save_script` der Datei mit `script Main #1: End` und `script Figur #2: LockAll / FacePlayer / Message "Bis bald!" / WaitButton / CloseMessage / SetFlag FLAG_SZENARIO_FIGUR / ReleaseAll / End`; `build: build/out.nds`; kein `check`-Block (das Textprojekt prüft der gated Kernel-Test); `play`: `continue`, `expect-map 415`, `walk-to 8 4`, `walk right 1`, `expect-map ${area.header}`, `wait 150`, `expect-flag ${frei.number} 0`, `walk-until-blocked up 6` (die Figur steht nördlich: blockiert bei ihr, `expect-at`), `press a`, `mash a 240`, `shot gespraech`, `expect-flag ${frei.number} 1`, zurück zur Tür (`walk-to 44 21`, `expect-map 415`), `walk-to 8 4`, `walk right 1`, `expect-map ${area.header}`, `wait 150`, `expect-flag ${frei.number} 1`, `walk-until-blocked up 6` durch die Kachel hindurch (`expect-at` eine Kachel weiter als vorher), `shot weg`. `hg-flag.yaml` ebenso auf `hg-trainer-begegnung.yaml` (IPKD, Header 64/63, Vorlage und Zellen von dort, `GiveItem`-Dialekt spielt keine Rolle: `SetFlag` und `Message` heißen in beiden Familien gleich).

- [ ] **Step 1: Scheiternde Tests:**
  - `test_games.py`: `test_save_block_is_read_for_the_three_games` (CPUD `0x02101EE0`, `0x14`, `0xDAC`, 288, `0xFEC`, 364; IPKD, IPKE), `test_save_block_needs_every_key` (eine Datei ohne `flags_bytes` → `ValueError`), `test_read_and_write_flag_and_var` (`put_save_block(ram, game, block)`; `write_flag(ram, game, 0x173, True)` → Byte `0x173 // 8` Bit 3; `read_var(ram, game, 0x40A5)` nach `write_var` 2; außerhalb `ValueError`; ungültiger Zeiger → `block_start is None`).
  - `test_steps.py`: `test_parse_flag_steps_take_hex_and_decimal`, `test_flag_and_var_steps_read_the_block`, `test_set_flag_and_set_var_write`, `test_expect_flag_and_var_fail_with_the_values`, `test_flag_steps_without_a_valid_pointer_fail` (`StepFailed` „kein gültiger Spielstand im RAM“).
  - `save_layout.rs`: `g4emu_games_yaml_carries_the_same_block` ohne `ignore`.
- [ ] **Step 2: Lauf** `cd scripts/g4emu && uv run pytest -m "not integration" -q` → FAIL; **Step 3: Umsetzen**; **Step 4: Lauf** → PASS; `cargo test -p g4-kernel --all-features flags::save_layout` → PASS.
- [ ] **Step 5: Szenarien** schreiben und einmal laufen lassen (Platin DE und HeartGold DE, frische Kopien `3d-1-szen-pt`, `3d-1-szen-hg`; `uv run g4emu scenario ../agent/scenarios/pt-flag.yaml --project <Kopie> --out ../../target/agent/gate-<datum>/pt-flag --set NEUER_HEADER=593 --set X_FIGUR=<x> --set Z_FIGUR=<z>`; die Kacheln der Figur in der Test-Umgebung an der Karte ablesen und in den Kopfkommentar schreiben); Bilder `gespraech.png`, `weg.png` nicht ins Repo.
- [ ] **Step 6: Doku** `flags_vars.md` „RAM-Lage“ (Tabelle, Bit- und Wortregel, Lesepuffer, Zeiger je Sitzung); `agent-gate.md` Schrittliste.
- [ ] **Step 7: Commit** `feat(g4emu): read and set flags and variables in the save block, expect-flag and expect-var, scenarios pt-flag and hg-flag` (mit Trailer).

### Task 9: Design-System fortschreiben

**Files:**
- Create: `design-system/openpencil/beispiele/flags-variablen.body.js`, `design-system/openpencil/NACHTRAG-3D-1.md`
- Modify: `design-system/README.md` (Abschnitt „Nachtrag 3d.1“ nach „Nachtrag 4.4“; unter „Was die App umsetzt“ eine Zeile „Flags und Variablen (3d.1)“ und in „Abweichungen“ die Zeile „Die Fläche liegt bis 3d.6 in Technik“; die Tabelle „Komponenten“ um „Flag-Picker“ und „Flag-Feld“), `design-system/Audit-Design-System.md` (Screen 13 „Flags und Variablen“: Stand), bei Antwort von OpenPencil `design-system/Editor_DesignSystem.fig`

**Interfaces:**
- Consumes: Texte und Zustände aus Task 6 und 7, die Screenshots aus Task 7 Step 5.

- [ ] **Step 1: README** „Nachtrag 3d.1“ nach dem Muster von „Nachtrag 4.4“: Quelle (`flags/*`, `workbench/inspector/{eventFields.ts,EventEditor.tsx,LevelScriptSection.tsx}`, `technik/TechnikArea.tsx`, `app/paletteEntries.ts`), Plan- und Gate-Verweis (`docs/phase-3d-1-gate.md`), Vergleichsbilder unter `target/agent/gate-<datum>/3d-1/`, neues Symbol `flag` (Phosphor `Flag`), keine neuen Tokens (Spalte „Live“ auf `status/warnung`); Komponenten „Flag-Picker“ (Treffer mit Name, pret-Name oder Bereich, Nummer, Zähler; Filter; Fuß „Freies Flag nehmen“, „Im Verzeichnis zeigen“; „Keins (0)“), „Flag-Feld“ (Chip, „Wählen…“, Zahlfeld in Tüfteln); die Fläche mit Filterzeile, Liste (Zustände: eigener Name, pret-Name, unbenannt, frei, reserviert, kartenlokal, Spalte „Live“), Detail (ohne Auswahl, gesetzt/geprüft/unklar, Gruppierung nach Header, DSPRE-Sperre, Hinweis „Gelesen, aber nirgends gesetzt“), Dialog „Als eigenes Flag benennen“, die Felder im Inspector; Abweichungen: die Fläche liegt bis 3d.6 in Technik statt im Bereich Geschichten (Screen 13), „Live“ ist leer bis 3d.9, Namen stehen nur in der Oberfläche, die Event-JSON des Textprojekts tragen bis 3d.4 Zahlen; **die `.fig` ist zurzeit nicht erreichbar: der Satz „Die Frames von 3d.1 folgen, sobald OpenPencil antwortet (Anleitung `openpencil/NACHTRAG-3D-1.md`)“**.
- [ ] **Step 2: Vorlage** `beispiele/flags-variablen.body.js` nach dem Muster von `aenderungen-git.body.js` (Bausteine aus `jsx-lib-app.js`, Farben über `v('token')`), `MODE`: `'liste'` (Frame „App: Technik, Flags und Variablen“: neun Technik-Einträge, „Flags und Variablen“ gewählt, Filterzeile, sechs Zeilen (eigener Name `FLAG_HAT_MEDAILLE` fett mit `FLAG_UNUSED_0x0094` grau, ein `FLAG_HIDE_…` mit „gesetzt 1 · geprüft 3“, ein kartenlokales „an 54 Orten“, ein freies, ein reserviertes `VAR_UNUSED_0x406C`), Spalte „Live“ leer; rechts das Detail von `FLAG_HAT_MEDAILLE` mit „Wo gesetzt“ (Scriptzeile) und „Wo geprüft“ (Figur, Level-Script)), `'picker'` (Frame „App: Dialog, Flag wählen“ über dem Inspector einer Figur mit dem Feld „Sichtbarkeits-Flag“), `'brett'` (Brett „Komponenten (App): Flag-Picker und Flag-Feld“: Flag-Feld in Bauen und Tüfteln, Picker-Zeile in vier Zuständen, Zeile der Liste in den Zuständen); Projekt „Sinnoh-Remix“, Platin DE.
- [ ] **Step 3: Anleitung** `NACHTRAG-3D-1.md` nach dem Muster von `NACHTRAG-4-4.md`: „Die Datei wurde nicht angefasst“; zwei Frames auf „App: Texte, Technik, Dialoge“ und das Brett auf „Komponenten“, Lage (nächste freie Spalte, x = Spalte × 1520, y = Reihe × 1000, vorher `get_page_tree`), Bauen (`bash ../build-screen.sh flags-variablen`, `MODE` je Frame, `op.mjs render`, Vergleich mit den Screenshots, erzeugte `.jsx` nicht einchecken), danach `fix_fig_text.mjs --rgb555` und README-Satz; was nicht gezeichnet wird (Dialog „Als eigenes Flag benennen“, Level-Script-Feld, Leerzustände).
- [ ] **Step 4: `.fig`** nur, wenn OpenPencil antwortet (bei gesperrtem Bildschirm laufen die RPCs in Zeitüberschreitungen; bei 10 000 Knoten Einzeländerungen vermeiden und die Frames neu rendern); sonst bleibt der Satz aus Step 1.
- [ ] **Step 5: Audit** wie unter Files; `bash scripts/check-tokens.sh` grün.
- [ ] **Step 6: Commit** `docs(design-system): addendum 3d.1 flags and variables, pickers and fields` (mit Trailer).

### Task 10: Gate 3d.1, `agent-gate.md`, Roadmap

**Files:**
- Create: `docs/phase-3d-1-gate.md`
- Modify: `docs/agent-gate.md` (Gate-Datei in der Liste oben, Abschnitt „Phase 3d.1“ mit Befehlen und `data-*`), `docs/ROADMAP.md` (Übersicht Zeile 3d; Abschnitt „Phase 3d“: Stand der Zeile 3d.1 unter der Tabelle), `docs/dev-setup.md` (gated `tests/flags.rs` und der Studio-Test in der Tabelle `G4_TEST_PROJECT`, falls Task 4 es noch nicht tat)

**Interfaces:**
- Consumes: alles Vorige; die Ausgaben `names …`, `usage …`, `index …`, `pool …`, `own …`, `flags …` der gated Tests; die Szenarien.

- [ ] **Step 1: Gate-Dokument** `docs/phase-3d-1-gate.md` nach dem Muster von `docs/phase-4-4-gate.md`: Kopf mit Stand, der Gate-Text der Spezifikation §4 Zeile 3d.1 als Zitat; Legende „Wer“ (Agent, Agent + Blick, Hand) mit Verweis auf `docs/agent-gate.md`; Namen in der Oberfläche (`[data-technik=flags]`, `[data-flag-filter]`, `[data-flag-list]`, `[data-flag-row=<n>]`, `[data-flag-live]`, `[data-flag-detail]`, `[data-flag-use]`, `[data-flag-picker]`, `[data-flag-field=<key>]`, `[data-name-flag-dialog]`, die Texte aus Task 7); Vorbereitung (Kopien `_testkopien/3d-1-{pt,hgde,hgus}` für (1), `3d-1-szen-{pt,hg}` für (2), `3d-1-web`, `3d-1-mcp`, `3d-1-ui-text` für (3), je mit `.nds` daneben; `cargo build --release -p g4-kernel --bin g4k`, `cargo build --release -p studio --bin studio` und die Release-Testbinaries; Worktree-Hinweise; Last mit `uptime`).
  - **(1) Automatik:** `scripts/gate.sh` grün (die neuen gated Tests laufen im Schritt je Projekt mit; `vanilla_index_builds_under_a_second` bleibt im Zeitschritt); einzeln je Projekt im Release-Profil mit `--nocapture --test-threads 1`: `vanilla_flag_names_count_the_metadata`, `vanilla_usage_matches_a_recount_of_the_sources`, `vanilla_flag_index_is_fast` (mit `G4_TIMING_STRICT=1`), `vanilla_pool_counts_the_free_numbers`, `vanilla_own_name_compiles_byte_equal`, der Studio-Test `flags_over_the_command_path_on_the_vanilla_copy`; `cd scripts/g4emu && uv run pytest -q` (Unit und Integration).
  - **(2) Szenarien:** `pt-flag.yaml` auf Platin DE, `hg-flag.yaml` auf HeartGold DE (Befehle aus Task 8 Step 5), Ausgaben `expect-flag … 1`, die Bilder `gespraech.png` und `weg.png`; dazu kopflos `studio mcp --project <3d-1-szen-pt> --base-rom <nds>`: `list_flags {kind: flag}` → `entries.length ≥ 4097`, `flag_usage` der Figur-Nummer → ein Objekt-Verweis `hidden_flag` und ein Script-Verweis `SetFlag` write.
  - **(3) Browser-Teil** (Test-Umgebung `G4_VITE_PORT=1700 G4_BRIDGE_PORT=1701`, Kopie `3d-1-web` mit übernommenen Scripts, Basis zugeordnet): Technik › Flags und Variablen (Liste zählt im Untertitel 4.097 Flags), Filter „nur frei“, Zeile wählen, Detail mit Wegen: Klick auf einen Script-Weg öffnet den Script-Editor an der Zeile, Klick auf eine Figur wählt sie in der Schau; Picker im Inspector der Figur: „Freies Flag nehmen“, speichern, `get_events` über `call.sh` zeigt `hidden_flag`; Level-Script „Neue Variable“ über den Picker; im DSPRE-Layout „Als eigenes Flag benennen…“ gesperrt mit Grund; `3d-1-ui-text` (Textprojekt): benennen → Register, `data/flags/NNNN.json`, `.g4-ids-flags.json` auf der Platte; „Umbenennen…“ → Script-Quelle und Entität folgen, ⌘Z nimmt es zurück; MCP über die Brücke (`3d-1-mcp`, Freigabe „Lesen und bearbeiten“): `mcp.sh call list_flags '{"kind":"flag"}'`, `mcp.sh call flag_usage '{"kind":"flag","number":"FLAG_UNUSED_0x0094"}'`, `mcp.sh call free_flag '{"kind":"var"}'`, `mcp.sh call name_flag '{…}'` → `not_text_project` im DSPRE-Layout.
  - **(4) Hand und Blick beim Nutzer:** Blick auf Fläche, Picker, Dialog in Hell, Dunkel, Kompakt (Bilder aus Task 7 Step 5); Hand: ein eigener Name in DSPRE geöffnet (DSPRE liest den Konstantenordner nie; die pret-Namen bleiben); Hand: `expect-flag` auf HeartGold US (`IPKE`, Zeiger `0x02111880`, nur in `games.yaml` übernommen, nicht gemessen).
  - **Ergebnis:** Tabelle Punkt / Wer / Ergebnis / Datum und die Zahlentabelle:

    | Spiel | Namen (Flags / Variablen) | Verweise neu (Flags / Variablen) | Index warm (ms) | 200 Abfragen (ms) | `usage` (ms) | Pool frei (Flags / Variablen) | Szenario `expect-flag` | Last |
    |---|---|---|---|---|---|---|---|---|
    | Platin DE | | | | | | | | |
    | HeartGold DE | | | | | | | | |
    | HeartGold US | | | | | | | – | |

- [ ] **Step 2: Gate abarbeiten** (Agent: (1) bis (3)); Werte und Befunde eintragen; Befunde beheben (eigene Commits `fix(3d.1): …` mit Regressionstest), bis alles grün ist.
- [ ] **Step 3: Roadmap** `docs/ROADMAP.md`: Übersicht Zeile 3d („3d.1 Gate vom Agenten bestanden <Datum>, Hand und Blick beim Nutzer offen; 3d.2, 3d.3, 4.5 laufen“) und im Abschnitt „Phase 3d“ unter der Tabelle „**Stand 3d.1 <Datum>:** Gate vom Agenten bestanden: Namen 4 097 / 2 244 Flags und 307 / 386 Variablen, Index warm <…> ms, 200 Abfragen <…> ms, Pool <…> / <…> Flags, eigener Name kompiliert byte-gleich und zieht beim Umbenennen nach, `pt-flag` und `hg-flag` mit `expect-flag`; Branch `phase-3d-1-flags-variablen`; [docs/phase-3d-1-gate.md](phase-3d-1-gate.md); beim Nutzer offen: Blick, DSPRE mit eigenem Namen, HeartGold US im Emulator“. `docs/agent-gate.md` Abschnitt „Phase 3d.1“ nach dem Muster „Phase 4.4“: Kopien, was wo ist (`data-*`), die Tabelle Punkt / Wer / Wie, die Befehle für (1) bis (3) und die Szenarien.
- [ ] **Step 4: Trailer prüfen** `git log --format='%(trailers:only,unfold)' main..HEAD | grep -c 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>'` gleich `git rev-list --count main..HEAD`.
- [ ] **Step 5: Commit** `docs(3d.1): gate of flags and variables, agent-gate section, roadmap status` (mit Trailer).

---

## Offen für die Umsetzung

- **rotoms Namensregel:** die 22 Variablen-Parameternamen und 3 Flag-Namen aus `ir_to_source.rs:28-58` (Checkout `fb11503`) wörtlich in `script_flagvars.yaml` übernehmen; weicht die Zahl ab, folgt der Test der Datei, die Regel bleibt.
- **Makros wie `SetVar` (Platin):** der Zugriff eines Makro-Parameters kommt aus seinen Varianten (`SetVarFromValue`/`SetVarFromVar`: `destVarID` `must_write`); trägt die Datenbank für den Makro-Parameter selbst nichts, steht der Eintrag in der YAML (`SetVar: { destVarID: write }`); der Test `platinum_variables_take_the_database_access` nennt, was gilt.
- **Bedingungen im AST:** wo `if`/`elseif`/`while`/`match` in `rotom::compiler::ast::Statement` ihre Bedingung tragen, mit `cargo doc -p rotom --open` nachsehen; eine Vergleichsform, die rotom als Befehl lowert (`CheckItem(...) == 1`), ist eine `Command`-Meldung und zählt nicht als Variable.
- **Hauptnamen-Regel:** stellt die Vanilla-Liste Nummern heraus, deren Hauptname unglücklich fällt (etwa ein `FLAG_UNK_…` neben einem sprechenden Namen), wird die Regel in `names.rs` an einer Stelle ergänzt; die Aliasse bleiben suchbar.
- **Header einer Quelle:** eine Scriptdatei, die mehrere Header nutzen, bekommt den ersten Header der Tabelle; kartenlokale Flags in gemeinsamen Dateien (Scripts ab 2000) werden so einem Header zugeordnet, der nicht jeder Verwendung entspricht. Reicht das in der Fläche nicht, gruppiert das Detail nach Quelle, wo `header` fehlt.
- **Versteckte Items und `event_type`:** ob jedes BG-Event mit Script `8000 + n` den Typ 2 trägt, prüft der Vergleichstest (`vanilla_usage_matches_a_recount_of_the_sources`); weicht Vanilla ab, zählt das Script allein.
- **Zeitsperre und der Script-Editor:** speichert der Editor eine Quelle und fragt die Fläche innerhalb von 250 ms, zeigt sie den alten Stand bis zur nächsten Anfrage; stört das im Browser-Teil, ruft `save_script` `refresh_scripts_now`.
- **Pool-Zahlen im Gate:** die Grenzen 100 bis 170 (Flags) und 10 bis 100 (Variablen) sind weit, weil Gutachten A mit einer anderen Namensregel zählte; der Test druckt die Werte, die Gate-Datei hält sie fest.
- **Szenario-Kacheln:** `X_FIGUR`/`Z_FIGUR` und die Kachel nach dem Durchgehen (`expect-at`) werden in der Test-Umgebung an der Karte abgelesen (Vorlage 342, Karten 5 und 6; HeartGold nach `hg-trainer-begegnung.yaml`) und als `--set` dokumentiert.
- **Werkzeugzahl:** `tool_count_is_about_thirty` kann mit vier Werkzeugen die Grenze erreichen; dann hebt Task 5 sie mit Begründung im Commit.
- **Platin-Ende der Flags:** `flags_end: 0xB60` folgt HeartGold (`NUM_FLAGS`) und den gemessenen 364 Byte; die Datenbank nennt Namen bis `0xFFF`, sie stehen in der Liste mit Bereich „außerhalb“ und lassen sich nicht benennen (Spezifikation „Was bewusst nicht entschieden ist“).
- **Parallele Merges:** `eventFields.ts` (3d.2 `from`), `TechnikArea.tsx` (nur bei Änderungen von 4.5 an den Einträgen), `icons.ts`, `dispatch.rs`, `catalog.rs`, `api.ts`, `navigation.ts`, `README.md` der Gestaltung und `ROADMAP.md` werden Konflikte an Anhängepunkten; die Erfahrung aus Phase 4 gilt (gesammelt lösen, `.fig` von main behalten).
