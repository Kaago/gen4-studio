# Phase 3g.2 „Leere Region mit Startstadt“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Das Rezept „Leere Region mit Startstadt“ als ein Store-Schritt (`templates::apply`): alle Header-IDs bleiben, Matrix 0 behält die Startstadt und füllt jede andere Zelle mit einer gesperrten Rand-Karte, alle übrigen Header bekommen die leere Eventdatei 0 und ein leeres Level-Script, Scripts und Texte bleiben nur unverwiesen liegen; Startstadt und Spielerhaus mit ihren Innenräumen bleiben; ein Start-Script (rotom-Vorlage) gibt den Starter und setzt Pokédex, Laufschuhe und den Starter im Startmenü. Verfügbar als Vorlage im Assistenten „Neues Spiel aus ROM“ und als Assistent „Welt leeren…“ (Technik › Werkzeuge). Danach meldet `run_checks` 0 Fehler, und die Region startet in `g4emu` auf Platin DE und HeartGold DE.

**Architecture:** Kernel-Modul `crates/g4-kernel/src/templates.rs` mit `plan_empty_region` (rechnet aus Header-Tabelle, Matrix 0 und Warps, was bleibt) und `apply_empty_region` (zuerst die unumkehrbaren Dateien: Rand-Karte per `duplicate_map_with`, neues Level-Script per `new_script_file`, Start-Script an die Scriptdatei der Stadt angehängt und kompiliert; dann eine `Store::transaction` „Vorlage: Leere Region mit Startstadt“ über Header-Tabelle, Matrix 0, Rand-Karte und die Eventdateien der Stadt). Rezept als Metadatei `metadata/templates.yaml` mit Namen aus `mapname.bin` (T01, T20) statt Nummern; Start-Script als Vorlagen `metadata/templates/start_<familie>.rotom`. `create_project` bekommt `Template::EmptyRegion` (Phase Vorlage = Scripts übernehmen + Rezept + Speichern). App-Befehle `plan_empty_region`, `apply_empty_region`; Assistent „Welt leeren…“ als neue Art im vorhandenen `WizardHost`. `g4emu` bekommt neue Spiele ohne Spielstand (Intro-Folge je Spielcode in `games.yaml`, Schritt `new-game`, Szenario-Schlüssel `save: none`) und den Schritt `walk-until-blocked`.

**Tech Stack:** Rust (g4-kernel Store, scripts, rotom), Tauri 2, React 19, Python (g4emu, uv, pytest).

**Spec:** [docs/ROADMAP.md](../../ROADMAP.md) „Phase 3g: Projektstart“ (3g.2, Gate); [entscheidungen.md](../../research/2026-10-25-ausbau/entscheidungen.md) „Projektstart“ („Leere Welt“, „Welt leeren…“); [gutachten-c-projektstart-mcp.md](../../research/2026-10-25-ausbau/gutachten-c-projektstart-mcp.md) A.3, A.4, A.6, A.7, A.8 (3e.2 dort = 3g.2).

## Befunde vor dem Plan (25.10.2026)

- Platin DE: Zweiblattdorf ist Header 411 (`T01`), genau eine Zelle von Matrix 0 (3, 27), Karte 0; 411: Scriptdatei 1052, Level-Script 982 (nur `on_transition #1`), Text 554, Eventdatei 390 (8 Figuren, 4 Warps nach 412, 414, 416, 417, 2 Auslöser, 1 Schild). Spielerhaus 414 (Matrix 128, Eventdatei 393, Tür (6, 10) → 411/1, Treppe (10, 3) → 415) und 415 (Matrix 129). Header 0: Eventdatei 0 (16 Nullbytes), Level-Script 897 (leer).
- HeartGold DE: Neuborkia ist Header 60 (`T20`), eine Zelle (21, 12), Karte 0; Warps nach 61, 63, 65, 66, 62; Haus 63/64; Matrix 0 enthält auch Kanto (47 × 17). Level-Script „leer“ = 4 Nullbytes (Platin 191, HeartGold 165 solche Member; Header 0 nutzt 897 bzw. 399). `level_script_id = 0` ist ein Fehler (`header.script-kind`).
- Versuch auf einer Platin-Kopie (Spielstand `CPUD-zimmer.sav`): ein neues Level-Script `{header_entries: [on_var_condition], var_conditions: [{var: 0x406C, value: 0, script_id: 10}]}` an Header 411 und Script `#10` in Scriptdatei 1052 laufen beim ersten Betreten der Stadt; `GivePokemon 387, 5, 0, VAR_RESULT` gibt Chelast Lv. 5 (Team-Menü belegt). Das Startmenü zeigt „POKéMON“ nur, wenn `VAR_PLAYER_STARTER` gesetzt ist (pret `pokeplatinum` `start_menu.c`); HeartGold prüft `FLAG_GOT_STARTER`, `FLAG_GOT_POKEDEX`, `FLAG_GOT_POKEGEAR` (pret `pokeheartgold`).
- Freie Variablen (in keinem übernommenen Script benutzt): Platin `VAR_UNUSED_0x406C`, HeartGold `VAR_UNK_4031`.
- Warps aus nicht behaltenen Headern in geleerte Außen-Header: Platin 264, HeartGold 254 (wären `warp.dest-warp`-Fehler) → alle nicht behaltenen Header bekommen Eventdatei 0, nicht nur die Außen-Header.
- Neues Spiel ohne Spielstand ist mit `g4emu` gescriptet (Intro-Folgen in `target/agent/probe-intro/INTRO.md`): Platin ab Bild ~10150 steuerbar in 415 (4, 6), HeartGold ab ~4560 in 64 (6, 6); Spieler „AAAAA“, Junge. In Platin hält die Mutter den Spieler an der Haustür mit einem Text auf (`mash b 400`, dann erneut hinaus).

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Was bleibt | Startstadt (Header mit dem internen Namen aus dem Rezept und `matrix_id 0`) und alle Header, die man von ihr über Warps erreicht, ohne einen anderen Header mit `matrix_id 0` zu betreten (Innenräume, Spielerhaus); ihre Eventdateien, Scripts und Level-Scripts bleiben, außer der Stadt selbst | „Start-Header und Spielerhaus bleiben“; die Innenräume der Stadt sind Teil der Stadt |
| Was geleert wird | Jeder andere Header: `event_file_id = 0`, `level_script_id` = das leere Level-Script, das Header 0 nutzt (sonst das erste Member mit genau 4 Nullbytes; fehlt beides: Ablehnung). Scriptdatei, Text, Begegnungen, Musik und alle IDs bleiben. Warps behaltener Eventdateien in geleerte Header werden entfernt. | Spec „Außen-Header bekommen leere Eventdateien und keine Level-Scripts“; ohne die Innenräume blieben 264/254 Fehler; Eventdatei 0 ist leer und von der Prüfung `event.script-missing` ausgenommen |
| Die Stadt | Karte und Warps bleiben, Schilder bleiben; Figuren und Auslöser werden entfernt (ihre Story-Scripts würden fehlende Figuren bewegen); das alte Level-Script wird durch ein neues ersetzt, das nur das Start-Script per Variablenbedingung startet | „originale Stadt ohne Figuren“ (Gutachten A.3); Versuch belegt |
| Rand-Karte | Kopie der Stadtkarte ohne Gebäude (`duplicate_map_with(town_map, true)`), alle 1024 Kollisionszellen `{kind: 0, collision: 0x80}`; jede Zelle von Matrix 0 mit Karte (≠ `0xFFFF`), die nicht zur Stadt gehört, bekommt diese Karte, den Header der Stadt und die Höhe der ersten Stadtzelle; leere Zellen bleiben leer | gleiche Texturen wie die Stadt (Gebietsdaten des Headers), kein Weg hinaus, keine Ortswechsel-Einblendung; „alle Header-IDs bleiben“ betrifft die Header-Tabelle |
| Start-Script | Vorlage je Familie in `metadata/templates/start_pt.rotom` und `start_hgss.rotom` mit Platzhaltern `{species}`, `{level}`, `{done_var}`, `{message}`; angehängt als neues Script `#N` (N = größte Nummer der Datei + 1) an die Scriptdatei der Stadt; Text als Literal (landet im Textarchiv der Stadt). Platin: `GivePokemon`, `GivePokedex`, `SetFlag FLAG_HAS_POKEDEX`, `GiveRunningShoes`, `SetVarFromValue VAR_PLAYER_STARTER, {species}`, `SetVarFromValue {done_var}, 1`. HeartGold: `GiveMon {species}, {level}, 0, 0, 0, VAR_SPECIAL_RESULT`, `GivePokedex`, `SetFlag FLAG_GOT_POKEDEX`, `SetFlag FLAG_GOT_STARTER`, `GiveRunningShoes`, `SetVarFromValue VAR_PLAYER_STARTER, {species}`, `SetVarFromValue {done_var}, 1`. Starter: Platin 387 (Chelast) Lv. 5, HeartGold 152 (Endivie) Lv. 5. Text: Deutsch „Willkommen in deiner leeren Region! Hier ist dein erstes Pokémon.“, sonst Englisch „Welcome to your empty region! Here is your first Pokémon.“ | Versuch; Spec „gibt den Starter und setzt Pokédex- und Laufschuh-Flags“; bis 3d keine Flag-Auswahl |
| Ein Store-Schritt | Header-Tabelle, Matrix 0, Rand-Karte und Eventdateien der Stadt in **einer** `Store::transaction` (Label „Vorlage: Leere Region mit Startstadt“); ⌘Z holt die Welt zurück. Rand-Karte, Level-Script-Datei und Start-Script entstehen vorher sofort auf der Platte (Plakette „Nicht rückgängig“ im Assistenten, unverwiesen nach ⌘Z) | Spec „als ein Store-Schritt“; Muster der übrigen Assistenten |
| Vorbedingungen | Familie Pt oder HGSS (DP: „Diamant/Perl: ohne Testprojekt nicht freigegeben“); Scripts übernommen (sonst `Refused("scripts are not adopted")`, der Assistent bietet „Scripts übernehmen“); nichts ungespeichert (sonst `Refused("save first")`, Assistent: „Erst speichern“); Startstadt im Rezept auflösbar | Text-Literal braucht gespeicherte Header und Archive |
| Neues Projekt mit dieser Vorlage | `create_project(.., Template::EmptyRegion, ..)`: Phase Vorlage = Scripts übernehmen (`scripts::adopt`, Fortschritt done/total) + `apply_empty_region` + `store.save()`; die Marke trägt `template: "empty_region"` | eine Vorlage, zwei Wege |
| Zahlen im Rezept | `templates.yaml` nennt die Stadt per internem Namen (`T01`, `T20`) und Variablen/Flags per Namen der Befehlsdatenbank; Nummern rechnet der Kernel | Gutachten A.4 (Namen statt Zahlen) |
| g4emu ohne Spielstand | `games.yaml` bekommt je Spielcode `intro:` (die gemessene Folge, Platin und HeartGold DE; IPKE ohne); Schritt `new-game` spielt sie ab Einschalten; Szenarien mit `save: none` starten ohne Spielstand; Schritt `walk-until-blocked RICHTUNG N` (ok, sobald ein Schritt blockiert, scheitert, wenn alle N gehen) | Gate verlangt Intro und Aufwachen zu Hause |

## Global Constraints

- Wie Plan 3g.1 (ROM-Regeln, Originale, Brücke nur am Ende ergänzen, Design-System nur „Nachtrag 3g“, grüne Prüfungen vor jedem Commit, Commit-Zeile `Co-Authored-By:` mit dem eigenen Modell).
- Neue Brückenbefehle `plan_empty_region`, `apply_empty_region`: am Ende von `COMMANDS`, des `match` und von `generate_handler!`.
- Nach dem Rezept: `run_checks` (alle drei Gruppen) mit 0 Fehlern auf Platin DE und HeartGold DE (gated).

## Review Focus

- Projekt mit ungespeicherten Änderungen oder ohne übernommene Scripts: Ablehnung vor jeder Plattenänderung (keine neue Karte, kein neues Level-Script) (Task 2).
- ⌘Z nach dem Rezept: Header-Tabelle, Matrix 0 und Eventdatei der Stadt wie vorher; `dirty` danach sauber nach Speichern; die drei neuen Dateien bleiben unverwiesen; `run_checks` wieder wie vorher (Task 2).
- Zweimal anwenden: der zweite Lauf findet die Stadt wieder (Header-Name), legt aber eine zweite Rand-Karte an — Ablehnung, wenn Matrix 0 schon nur Stadt und Rand trägt („schon leer“) (Task 2).
- HeartGold: Kanto-Zellen und die `kanto_flag`-Header werden mitgeleert, Neuborkias Innenräume (Labor) bleiben (Task 2, gated).
- Eine behaltene Eventdatei mit Warp in einen geleerten Header: der Warp verschwindet, kein `warp.dest-warp` (Task 1, synthetisch).

---

### Task 1: Rezept planen (`templates.rs`, `templates.yaml`)

**Files:**
- Create: `crates/g4-kernel/src/templates.rs` (in `lib.rs` `pub mod templates;`), `crates/g4-kernel/metadata/templates.yaml`, `crates/g4-kernel/tests/templates.rs`

**Interfaces:**
- Consumes: `Store` (Header-Tabelle, Matrix 0, Eventdateien über die vorhandenen Lesewege), `Project::headers() -> Vec<HeaderSummary>` (mit `internal_name`, `matrix_id`, `event_file_id`, `level_script_id`, `script_file_id`), Befehlsdatenbank `scripts::scrcmd_database(family)` für Namen von Variablen und Flags.
- Produces:
  - `pub struct RegionRecipe { pub start_town: String, pub starter_species: u16, pub starter_level: u8, pub done_var: String, pub message_de: String, pub message_en: String }` und `pub fn region_recipe(family: Family) -> Result<RegionRecipe, KernelError>` (DP → `Refused(DP_LOCKED)`).
  - `pub struct RegionPlan { pub start_header: u16, pub town_name: String /* Ortsname, z. B. „Zweiblattdorf“ */, pub town_cells: Vec<(u8, u8)>, pub town_map: u16, pub town_altitude: u8, pub town_event_file: u16, pub town_script_file: u16, pub kept: Vec<u16>, pub emptied: Vec<u16>, pub border_cells: usize, pub empty_level_script: u16, pub dropped_warps: Vec<(u16, u16)> /* (Eventdatei, Warp-Index) */, pub removed_objects: usize, pub removed_triggers: usize, pub already_empty: bool }` (`Serialize`, snake_case)
  - `pub fn plan_empty_region(store: &mut Store) -> Result<RegionPlan, KernelError>`: Stadt = Header mit `internal_name == recipe.start_town` und `matrix_id == 0` (fehlt → `Refused("start town T01 not found")`); Stadtzellen = Zellen von Matrix 0 mit dieser Header-ID und Karte ≠ `0xFFFF`; behalten = Stadt plus Breitensuche über die Warps der Eventdateien zu Headern mit `matrix_id != 0` (weiter über deren Warps, nie in einen anderen Header mit `matrix_id 0`); geleert = alle anderen Header (auch Header 0; Header, die schon Eventdatei 0 und das leere Level-Script haben, zählen nicht als Änderung); leeres Level-Script = das von Header 0, wenn sein Member genau 4 Nullbytes hat, sonst das erste solche Member (keines → `Refused`); `dropped_warps` = Warps behaltener Eventdateien in geleerte Header; `already_empty` = jede Zelle von Matrix 0 mit Karte trägt schon die Stadt-Header-ID und außerhalb der Stadtzellen dieselbe Karte, und die Stadt hat keine Figuren.
- `templates.yaml`: `empty_region: { pt: { start_town: T01, starter: { species: 387, level: 5 }, done_var: VAR_UNUSED_0x406C }, hgss: { start_town: T20, starter: { species: 152, level: 5 }, done_var: VAR_UNK_4031 }, message: { de: "Willkommen in deiner leeren Region! Hier ist dein erstes Pokémon.", en: "Welcome to your empty region! Here is your first Pokémon." } }`.

- [ ] **Step 1: Failing tests:** synthetisch (Projekt aus `test_support::synthetic_project()`; Header, Matrix und Warps per Store-Änderungen im Test zurechtgelegt): `plan_keeps_the_town_and_its_rooms` (Stadt mit zwei Innenräumen, ein Innenraum mit Warp in einen zweiten Außen-Header → dieser geleert, der Warp in `dropped_warps`), `plan_refuses_a_missing_town`, `plan_picks_the_empty_level_script_of_header_0`, `plan_refuses_diamond`. Gated (`G4_TEST_PROJECT`): `vanilla_plan_empty_region` → Platin DE: `start_header 411`, `town_cells == [(3, 27)]`, `kept` enthält 411, 412, 414, 415, 416, 417, `border_cells == 467`, `empty_level_script == 897`; HeartGold DE: `start_header 60`, `town_cells == [(21, 12)]`, `kept` enthält 60, 61, 62, 63, 64, 65, 66, `border_cells == 491`, `empty_level_script == 399` (Werte je Projekt per `game_code` gewählt; IPKE nur `start_header == 60`).
- [ ] **Step 2: Run** `cargo test --workspace --all-features -- templates` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS; gated mit `G4_TEST_PROJECT` auf eine **Kopie** je Spiel (`vanilla_copy`), nie auf das Original.
- [ ] **Step 5: Commit** `feat(kernel): plan the empty-region template (town, kept rooms, emptied headers, border cells)`

---

### Task 2: Rezept anwenden, Start-Script, `create_project` mit der Vorlage, `g4k template`

**Files:**
- Modify: `crates/g4-kernel/src/templates.rs`, `crates/g4-kernel/src/start.rs` (`Template::EmptyRegion` = `"empty_region"`; Phase Vorlage), `crates/g4-kernel/src/bin/g4k.rs` (`g4k template <projekt> empty-region` übernimmt Scripts, wenn nötig, wendet an, speichert, druckt den Bericht; `g4k create … --template empty-region`), `crates/g4-kernel/tests/templates.rs`, `crates/g4-kernel/tests/cli.rs`
- Create: `crates/g4-kernel/metadata/templates/start_pt.rotom`, `start_hgss.rotom` (Inhalt wie in den Entscheidungen, mit `{n}`, `{species}`, `{level}`, `{done_var}`, `{message}`), `docs/formats/templates.md`; Abschnitt in `docs/dev-setup.md`

**Interfaces:**
- Consumes: Task 1 (`plan_empty_region`, `RegionPlan`, `region_recipe`); `Store::duplicate_map_with`, `Store::transaction` (je Asset nur einmal: Header-Tabelle über `tx.edit(HEADER_TABLE, …)`, Matrix 0 über `tx.edit_matrix(0, …)`, Rand-Karte über `tx.edit(AssetId { kind: Map, index }, …)`, Eventdateien über `tx.edit_event`), `scripts::{new_script_file, write_source, compile, read_source}`.
- Produces:
  - `pub struct RegionReport { pub plan: RegionPlan, pub border_map: u16, pub level_script: u16, pub start_script: (u16, u16) /* Datei, Nummer */, pub change: Option<ChangeId>, pub changed: Vec<AssetId> }`
  - `pub fn apply_empty_region(store: &mut Store) -> Result<RegionReport, KernelError>`: Vorbedingungen in dieser Reihenfolge, jede vor jeder Plattenänderung: Familie (DP → `DP_LOCKED`), `already_empty` → `Refused("the world is already empty")`, Scripts übernommen (`Refused("scripts are not adopted")`), `store.dirty().is_empty()` (`Refused("save first")`); dann Rand-Karte (`duplicate_map_with(town_map, true)`), Level-Script-Datei (`new_script_file(LevelScript)` + `write_source` mit `{header_entries: [on_var_condition], var_conditions: [{var: done_var, value: 0, script_id: n}]}`), Start-Script (Quelle der Stadt-Scriptdatei lesen, Vorlage mit `n = größte Scriptnummer + 1` anhängen, `write_source`, `compile`; Text nach Sprache `de` sonst `en`); dann die Transaktion „Vorlage: Leere Region mit Startstadt“ (Header-Tabelle, Matrix 0, Rand-Karte mit 1024 × `{kind 0, collision 0x80}`, Stadt-Eventdatei ohne Figuren und Auslöser, `dropped_warps` entfernt).
  - `create_project(.., Template::EmptyRegion, ..)`: Phase Vorlage meldet `done/total` (Scripts übernehmen, dann ein Schritt für das Rezept), danach `store.save()`.

- [ ] **Step 1: Failing tests:** synthetisch: `apply_refuses_unsaved_changes_before_touching_the_disk` (Member-Zahlen von Karten und Scripts unverändert), `apply_refuses_without_adopted_scripts`, `apply_is_one_undo_step` (Verlauf hat genau einen neuen Eintrag mit dem Label; `undo` stellt Header-Tabelle, Matrix 0 und Stadt-Eventdatei byte-gleich wieder her), `apply_twice_is_refused`, `start_script_template_fills_every_placeholder` (keine `{` mehr im Ergebnis). Gated (Kopie, `G4_TEST_PROJECT`): `vanilla_empty_region_has_no_check_errors` (übernehmen, anwenden, speichern, alle Prüfgruppen: 0 Fehler, `groups.len() == 3`), `vanilla_empty_region_undo_restores_the_world`, `vanilla_empty_region_builds` (`build()` ohne Fehler). Gated `G4_TEST_ROM`: `project_start_empty_region_from_the_rom` (`create_project` mit `EmptyRegion`, Marke `empty_region`, 0 Fehler). `tests/cli.rs`: `g4k template` ohne Projekt → Exit 1 mit Meldung.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS, gated auf Platin DE und HeartGold DE (Kopien).
- [ ] **Step 5: Commit** `feat(kernel): apply the empty-region template as one store step with a start script; create_project and g4k template`

---

### Task 3: App-Befehle und Brücke

**Files:**
- Create: `apps/studio/src-tauri/src/commands/templates.rs`, `apps/studio/src/project/api.templates.ts` (+ Test; in `api.ts` am Ende re-exportiert)
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `lib.rs` (`generate_handler!` am Ende), `bridge/dispatch.rs` (am Ende von `COMMANDS` und `match`, Test `every_command_accepts_the_keys_api_ts_sends`), `apps/studio/src/project/api.start.ts` (`Template` um `"empty_region"`)

**Interfaces:**
- Produces: `plan_empty_region_impl(state) -> Result<RegionPlan, CmdError>`, `apply_empty_region_impl(state) -> Result<RegionReport, CmdError>` (danach `session.files_written()`; die Antwort trägt `changed`); Tauri-Befehle `plan_empty_region`, `apply_empty_region` (async, `spawn_blocking`); Brücke ohne Pfadargumente. TS: `planEmptyRegion()`, `applyEmptyRegion()`, Typen `RegionPlan`, `RegionReport`.

- [ ] **Step 1: Failing tests:** `dispatch.rs`: beide Befehle im Schlüssel-Test, `apply_empty_region_refuses_unsaved_changes` (synthetisch); gated `project_start_bridge_empty_region` (`G4_TEST_ROM` → `create_project` mit `empty_region`, öffnen, `run_checks` 0 Fehler). `api.templates.test.ts`: Aufrufnamen.
- [ ] **Step 2–4** wie gewohnt. **Step 5: Commit** `feat(studio): plan_empty_region and apply_empty_region in the app and the bridge`

---

### Task 4: Oberfläche: Vorlage frei, Assistent „Welt leeren…“

**Files:**
- Create: `apps/studio/src/project/ClearWorldWizard.tsx` (+ `.module.css`, Test)
- Modify: `apps/studio/src/project/NewProjectWizard.tsx` (Karte „Leere Region mit Startstadt“ frei: „Nur die Startstadt mit Spielerhaus bleibt, der Rest ist eine leere Region. Ein Start-Script gibt dir deinen ersten Partner.“; bei `untested` die Plakette; Zusammenfassung nennt „Scripts werden übernommen“), `apps/studio/src/technik/TechnikArea.tsx` (+ Test; in Werkzeuge ein Eintrag `data-tool="welt-leeren"`, Titel „Welt leeren“, Satz „Macht aus der Welt eine leere Region mit Startstadt. Die Welt holst du mit ⌘Z zurück.“, Taste „Welt leeren…“, ohne Projekt gesperrt mit „Erst ein Projekt öffnen“), `apps/studio/src/workbench/WorkbenchContext.tsx` (`WizardRequest` um `{ kind: "clearWorld" }`), `apps/studio/src/app/WizardHost.tsx` (ein `case`), `apps/studio/src/App.tsx` (Technik bekommt `onClearWorld`; Palette „Welt leeren…“ in `paletteEntries.ts`)

Assistent „Welt leeren“ (Titel „Welt leeren“), Schritte:
1. **Was bleibt** – lädt `planEmptyRegion`: „Zweiblattdorf bleibt mit 6 Räumen.“ (Name und Zahl aus dem Plan), „N Orte werden geleert: ihre Figuren, Türen, Auslöser und Level-Scripts verschwinden. Scripts und Texte bleiben liegen.“, „M Felder der Weltkarte werden zu einer gesperrten Randkarte.“, „Ein Start-Script gibt dir <Art> auf Level 5, den Pokédex und die Laufschuhe.“ (Artname über `name_tables`, sonst „Art 387“). Vorbedingungen als `Notice`: Scripts nicht übernommen → „Dafür müssen die Scripts übernommen sein.“ mit Taste „Scripts übernehmen“ (`adoptScripts` mit Fortschritt); ungespeichert → „Speichere zuerst, das Rezept schreibt Scripts direkt auf die Platte.“ mit Taste „Speichern“; schon leer → „Die Welt ist schon leer.“ `canNext` nur ohne offene Vorbedingung.
2. **Zusammenfassung** – `summary`: „Leert die Welt bis auf Zweiblattdorf. Die Welt holst du mit ⌘Z zurück; Randkarte, Level-Script und Start-Script bleiben als Dateien.“, `irreversible` (Plakette „Nicht rückgängig“ für die Dateien), `submitLabel` „Welt leeren“.
- Ergebnis: `DoneResult` „Welt geleert“, Zeilen „Randkarte 0NNN, Level-Script NNNN, Start-Script NNNN #n“, Als Nächstes „Mit „Hier spielen“: Intro, zu Hause aufwachen, draußen wartet dein erstes Pokémon.“; `onShow` zeigt die Stadt in der Welt, `onPlay` wie bei den anderen Assistenten. Danach lädt die Welt neu (Header, Matrix 0, Eventdateien, Karten), wie nach ⌘Z (`reloadPlan`).

- [ ] **Step 1: Failing tests:** `ClearWorldWizard.test.tsx` (`walks_plan_and_summary_then_applies`, `offers_to_adopt_scripts_first`, `asks_to_save_first`, `already_empty_blocks`, `shows_the_result_with_show_and_play`), `TechnikArea.test.tsx` (`clear_world_needs_a_project`, `clear_world_opens_the_wizard`), `NewProjectWizard.test.tsx` (`empty_region_card_is_selectable`), `paletteEntries.test.ts`.
- [ ] **Step 2–4** mit `frontend-design:frontend-design`, nur Primitives und Tokens; `pnpm -C apps/studio exec vitest run`, Build, `check-styles`, `check-tokens`.
- [ ] **Step 5: Commit** `feat(studio): template "Leere Region mit Startstadt" in the new-game wizard and the wizard "Welt leeren…" in Technik`

---

### Task 5: g4emu: neues Spiel ohne Spielstand, `walk-until-blocked`, Szenarien

**Files:**
- Modify: `scripts/g4emu/g4emu/games.yaml` (`intro:` für CPUD und IPKD aus `target/agent/probe-intro/INTRO.md`, `intro_ends: {header, x, y}`), `games.py`, `steps.py` (Schritte `new-game` und `walk-until-blocked RICHTUNG N`), `scenario.py` (Schlüssel `save: none` → `g4emu run` ohne `--save`; die Kopie bleibt Pflicht), Tests in `scripts/g4emu/tests/`
- Create: `scripts/agent/scenarios/pt-leere-region.yaml`, `hg-leere-region.yaml`; Abschnitt in `docs/agent-gate.md`

Szenario (Platin; HeartGold entsprechend mit Neuborkia): `edits: [[adopt_scripts, {}], [apply_empty_region, {}, {expect: [{path: plan.start_header, equals: 411}]}], [run_checks, {scope: all}, {expect: [{path: "findings[severity=error].length", equals: 0}, {path: groups.length, equals: 3}]}]]`, `build: out.nds`, `save: none`, `play:` `new-game`, `expect-map 415`, `shot zimmer`, Weg zur Treppe, `expect-map 414`, zur Tür, Text der Mutter, `expect-map 411`, `shot ankunft`, Text des Start-Scripts, `shot start-script`, Menü und Team (`shot team`), `walk-until-blocked up 40`, `where`, `expect-map 411`, `shot rand`. Wege und Zahlen erst gemessen, dann eingetragen.

- [ ] **Step 1: Failing tests** (pytest, ohne Emulator): `test_parse_new_game_and_walk_until_blocked`, `test_games_have_intro_for_cpud_and_ipkd`, `test_scenario_save_none_runs_without_save` (der Befehl hat kein `--save`), `test_walk_until_blocked_fails_when_never_blocked` (mit dem Fake-Emu der bestehenden Tests).
- [ ] **Step 2–4:** `cd scripts/g4emu && uv run pytest -m "not integration" -q`; dann die Szenarien auf frischen Kopien `_testkopien/3g-gate-b/pt` und `…/hg` (aus `create_project` mit Vorlage `original` über `g4k create`) laufen lassen und die Wege eintragen, bis beide mit `"ok": true` enden.
- [ ] **Step 5: Commit** `feat(g4emu): new game without a save (intro per game code), walk-until-blocked; scenarios for the empty region`

---

### Task 6: Gate 3g.2, Gesamtlauf, Doku, Nachtrag, Roadmap

**Files:**
- Modify: `docs/phase-3g-gate.md` (Teil B, Ergebnis), `design-system/README.md` (nur „Nachtrag 3g“ ergänzen: Assistent „Welt leeren“, Vorlage frei), `design-system/openpencil/NACHTRAG-3g.md` (+ `beispiele/welt-leeren.body.js`), `docs/ROADMAP.md` (Status 3g: „umgesetzt, Abnahme offen“), `docs/agent-gate.md`

Gate-Punkte Teil B:
| (B1) gated Kernel- und Brückentests der Vorlage auf Platin DE und HeartGold DE: 0 Fehler, Undo, Build | Agent |
| (B2) Szenarien `pt-leere-region.yaml`, `hg-leere-region.yaml`: Intro, Aufwachen im Zimmer, Start-Script gibt den Starter (Team-Bild), Weg aus der Stadt blockiert | Agent + Blick |
| (B3) Browser: „Welt leeren…“ in Technik › Werkzeuge auf einer Kopie, Ergebnis, ⌘Z holt die Welt zurück; Vorlage im Assistenten „Neues Spiel aus ROM“ | Agent + Blick |
| (B4) `scripts/gate.sh` grün | Agent |
| (B5) Gegenprobe: eine Original-Kopie hat in Zweiblattdorf Figuren und Wege hinaus | Agent |

- [ ] **Step 1–4:** Durchläufe, `G4_ROMS_DIR=/Users/POLTEP/Projekte/lab/editor/Roms bash scripts/gate.sh > target/agent/gate.log 2>&1` im Hintergrund, Ergebnisse eintragen, Bilder nach `target/agent/gate-2026-10-25/`.
- [ ] **Step 5: Commit** `docs(3g.2): gate part B, design-system addendum, roadmap status`
