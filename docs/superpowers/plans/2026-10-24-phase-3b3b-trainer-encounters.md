# Phase 3b.3b „Trainer, Begegnungen, Gate“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Trainer als zusammengesetztes Asset (Eigenschaften, Team, Name, Kampftexte) mit „Neuer Trainer“ und Verbindung zu Objekt-Events in der Welt; Begegnungen mit Anteilen, Ersatzlisten und Warnungen aus dem Kernel, neue Begegnungsdatei und „Für diesen Ort abspalten“; `g4emu`-Kampfschritte und zwei Szenarien, die das Roadmap-Gate 3b.3 beweisen.

**Architecture:** Baut auf 3b.3a auf (`RawRecord`-Codecs in `g4-formats/src/data/`, `Store::transaction` mit typisierten `Tx::edit_*`, `data/{names,refs,icons,arm9}.rs`, Befehle `data_list|data_get|data_edit|data_raw` mit `DataKind::{Trainer, Encounter}` als Platzhalter, Bereich Pokémon mit `PanelHead.switcher` und gesperrten Unterbereichen Trainer und Begegnungen, `useDataDoc`, `dataStore`, `dataBus`). Neu: `AssetKind::Trainer` mit `Location::MemberPair`, Fachlogik `data/encounter_rules.rs` und `data/trainer_rules.rs` im Kernel (die Oberfläche rechnet nichts), `create_trainer` erweitert die gemeinsame Kampf-Scriptdatei über die rotom-Quelle, `expect-battle` erkennt den Kampf über eine RAM-Signatur in beiden Spielen.

**Tech Stack:** Rust (g4-formats, g4-kernel, Tauri 2, proptest, uxie 0.9.7 als Orakel), React 19 + `ui/` (Base UI nur in `ui/primitives`, CSS Modules), vitest 5, Python `g4emu` (`scripts/g4emu`, pytest).

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md](../specs/2026-10-23-phase-3b3-data-editors-design.md) §1 Gate 3b.3b, §6, §7, §8 (Befehle Plan b), §9 (Trainer, Begegnungen), §10, §11, §13, §14. **Recherche (bindend, wo sie der Spec widerspricht):** [encounter-mechanics-3b3b.md](../../research/2026-10-23-3b3/encounter-mechanics-3b3b.md), [trainer-mechanics-3b3b.md](../../research/2026-10-23-3b3/trainer-mechanics-3b3b.md), [trainer-script-extension-3b3b.md](../../research/2026-10-23-3b3/trainer-script-extension-3b3b.md), [task0-3b3a.md](../../research/2026-10-23-3b3/task0-3b3a.md). Byte-Layouts: `docs/formats/trainer.md`, `trainer_text.md`, `encounter.md`.

Stand 24.10.2026, geschrieben gegen `main` 42d3758 (3b.3a gemerged). Pfade relativ zur Repo-Wurzel; Frontend-Pfade relativ zu `apps/studio/src`.

## Entscheidungen aus dem Brainstorming (Recherche und Spike)

| Frage | Entscheidung | Beleg |
|---|---|---|
| Zwei neue Trainer je Spiel trotz fester Kampf-Scriptdatei | `create_trainer` erweitert die Slot-Liste der Kampf-Scriptdatei (Pt 1114, HGSS 0953) in der rotom-Quelle (`script script_1 #[1-928, 930]`) und schreibt sie über den Kernel-Compile; die ID des „gesehen“-Slots (Pt 929, HGSS 740) bekommt einen gesperrten Platzhalter-Datensatz. Gate: Pt 928 und 930 (Erweiterung bewiesen), HGSS 738 und 739; die gated Tests erweitern HGSS bis 741. | Spike §2–§3 |
| Flag-Kapazität | 1040 Trainer-IDs je Spiel (Flag `0x550 + ID` bis `0x95F`); `arm9_tables.yaml` bekommt `trainer_flag_capacity: 1040` für CPUD, IPKD, IPKE; `create_trainer` verweigert ID ≥ 1040. | trainer-mechanics §4 |
| Kampf-Erkennung im Emulator | Signatur im `BattleMon` des Gegners: Art (u16) +0x00, acht Bytes `06` ab +0x18, Level (u8) +0x34; trifft in Pt und HGSS genau einmal und nur im Kampf. Pt zusätzlich Zeiger `u32 @ 0x02101F88` (0 außerhalb des Kampfs) als Plausibilität. Kein Zeiger-Offset-Modell, weil die Heap-Lage je Lauf wandert. | encounter-mechanics §4, trainer-mechanics §6 |
| Deterministische Wildbegegnung | Neuer Ort mit neuem Header (Patch) und leerer Eventdatei, neue Begegnungsdatei, Rate 100, **alle** wirksamen Slots auf die Testart: Pt Gras 0–11 **und** `day`/`night`, HGSS Morgens/Tags/Nachts; hin und her laufen (Pt 40 % je Grasschritt, HGSS 20 %, nach vier Kehrtwenden 80 %, erste vier Schritte nach Kartenwechsel nie). | encounter-mechanics §1, §3, §5 |
| Pokémon für den Sieg | NPC auf dem neuen Ort (nicht im Zimmer: Objekte des Spielstand-Orts erscheinen nach `continue` nicht) mit Script `GivePokemon` (Pt, ID 150) / `GiveMon` (HGSS, 137), Level 100, Art ohne Adlerauge/Bedroher; danach `win-battle`. Ohne Team endet jeder Kampf mit Niederlage und Rückkehr nach Hause (Pt 414, HGSS 63): `expect-battle` reicht für Art und Stufe, der Sieg braucht den Geber. | encounter-mechanics §4, trainer-mechanics „Neue Figuren“ |
| Default-Attacken | Lernset der Grundform bis zum Level durchgehen, schon bekannte überspringen, bei vollen Slots die erste hinausschieben (nicht „die letzten vier“). | trainer-mechanics §2 |
| Trainername | Harte Grenze 7 Zeichen (unkomprimiert 8 UTF-16-Einheiten mit Endzeichen); Hinweis „Höchstens 7 Zeichen“. | trainer-mechanics §7 |
| Kampftexte | `trtblofs` ist ein Byte-Offset in `trtbl`; Einträge eines Trainers müssen zusammenhängen; neue ans Ende, `trtblofs` bis zur ID mit 0 auffüllen. Der Text „vor dem Kampf“ sollte auf `\r` enden: Warnung (Quelle „Trainer“), keine stille Änderung. | trainer-mechanics §3 |
| Begegnungsdatei | Member-Zahl kommt zur Laufzeit aus dem NARC; Index N anhängen funktioniert; Grenzen Pt 65534, HGSS 254; Member genau 424/196 B. | encounter-mechanics §6 |
| Sichtweite | Objekt-Event `data[0]`; `trainer_type` 1 für Sichtlinie; beide über `update_event` setzbar machen (fehlt heute). | task0, trainer-mechanics §1 |

## Global Constraints

- Wie Plan 3b.3a (Global Constraints gelten unverändert): Vanilla nie beschreiben, `vanilla_copy()` für schreibende Tests, PATH-Export, `cargo test --workspace --all-features` mit Filter (kein `-p`, sonst scheitert rotoms Build-Skript am Download), gated Tests mit `G4_TEST_PROJECT` und `G4_ROMS_DIR` absolut, Codecs lehnen nur falsche Längen ab, arm9 nur lesen, Styling nur über `ui/` und Tokens, Base UI nur in `ui/primitives`, Tests über Rollen und `data-*` (`data-trainer`, `data-slot`, `data-encounter`), deutsche UI-Texte mit Verben, englische Commits mit `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Neue Backend-Befehle nur die aus Spec §8 Plan b: `create_trainer`, `set_trainer_texts`, `create_encounter_file`, `split_encounter_file`; dazu die Erweiterung von `update_event` (Felder) und `data_get`/`data_edit` für `trainer` und `encounter`. Jeder Befehl in `bridge/dispatch.rs`, `generate_handler!`, `api.ts`.
- Anlegen (`create_trainer`, `create_encounter_file`) ist wie `append_member` sofort auf der Platte und nicht rückgängig; Dialoge sind Fragen, Plakette „Nicht rückgängig“.
- `create_trainer` setzt übernommene Scripts voraus (`scripts_status`); sonst Ablehnung mit Grund „Erst Scripts übernehmen (Bereich Welt, Panel Script)“.
- DP bleibt gesperrt („Diamant/Perl: ohne Testprojekt nicht freigegeben“).
- Kein Löschen von Trainern oder Begegnungsdateien (3b.4).

## Review Focus

- `create_trainer` auf einem Projekt, dessen Kampf-Scriptdatei schon einmal erweitert wurde (Slot-Liste `#[1-928, 930, 931]`): die nächste ID hängt lückenlos an, der Platzhalter 929 bleibt gesperrt, der Compile schreibt den Code byte-gleich (Task 4).
- Team mit sechs Pokémon, Schalter „Eigene Attacken“ aus und wieder an: Attacken werden nach der pret-Regel gefüllt, Undo stellt die alten Attacken her, `encode(decode(p, q)) == (p, q)` gilt nach jeder Edit-Folge (Task 3, Task 5).
- Begegnungsdatei von 190 Headern genutzt, „Für diesen Ort abspalten“: genau ein neuer Member, nur dieser Header wird umgehängt, ⌘Z hängt den Header zurück und lässt die Kopie liegen (Task 4, Task 7).
- `expect-battle` auf einem Kampf, der 180 Bilder nach dem Schritt beginnt (HGSS), und auf einem Nicht-Kampf: Treffer innerhalb des Limits bzw. sauberer Fehler mit zuletzt gelesener Lage, nie ein falscher Treffer im Feld (Task 1).
- Objekt-Event mit Script 3355 und ein Trainer 356: das Feld „Trainer“ zeigt „Sebastian“, Ändern setzt die Script-Nummer, ein Script außerhalb 3000–4999 zeigt „kein Trainer“ (Task 8).

---

### Task 1: `g4emu`-Kampfschritte und Kalibrierung

**Files:**
- Modify: `scripts/g4emu/g4emu/steps.py` (Argzahlen-Tabelle und Schritt-Tabelle), `scripts/g4emu/g4emu/games.yaml`, `scripts/g4emu/g4emu/emu.py` (RAM-Suche, falls dort die Abzüge entstehen), `scripts/g4emu/tests/test_steps.py` (oder die vorhandene Testdatei für Schritte), `docs/agent-gate.md` (Abschnitt Emulator: neue Schritte)
- Create: `scripts/g4emu/g4emu/battle.py` (Signatur-Suche, reine Funktionen, synthetisch testbar)

**Interfaces:**
- `battle.find_enemy(ram: bytes) -> list[tuple[int, int, int]]` (Offset, Art, Level) für jede Stelle mit `ram[o+0x18:o+0x20] == b"\x06"*8`, Art = u16 LE bei `o`, Level = u8 bei `o+0x34`, Art in 1..=507, Level in 1..=100; `battle.in_battle(ram, game) -> bool` (genau ein Treffer; bei CPUD zusätzlich `u32 @ 0x02101F88 != 0`).
- Schritte: `expect-battle SPECIES LEVEL [TIMEOUT_FRAMES]` (Standard 600; pollt alle 10 Bilder; Erfolg druckt `{"species", "level", "offset", "frame"}`; Fehler Exit 2 mit zuletzt gesehener Signatur oder „kein Kampf“), `walk-until-battle DIR N [back-and-forth]` (läuft bis zu N Schritte, bei `back-and-forth` wechselt die Richtung nach jedem Schritt; bricht ab, sobald `in_battle` wahr ist; Exit 2, wenn nach N Schritten kein Kampf), `mash BUTTON FRAMES` (drückt alle 8 Bilder), `end-battle [TIMEOUT_FRAMES]` (A alle 8 Bilder, bis `in_battle` falsch ist und die Ortsabfrage wieder einen Header liefert; druckt den Header), `win-battle [TIMEOUT_FRAMES]` = `end-battle` plus Prüfung, dass der Header danach nicht der Heim-Header ist (Pt 414, HGSS 63, aus `games.yaml` `home_header`). `games.yaml` je Code: `battle: { signature: "06x8@0x18", species_at: 0, level_at: 0x34 }`, CPUD zusätzlich `battle_pointer: 0x02101F88`, `home_header: 414` (IPKD/IPKE 63).

- [ ] **Step 1: Failing tests** (`test_steps.py`, synthetisch): `find_enemy_hits_exactly_the_signature` (gebauter Puffer mit einer Signatur und einem Fast-Treffer mit sieben `06`), `in_battle_requires_one_hit`, `expect_battle_times_out_cleanly` (gefälschter Emulator-Stub ohne Signatur → Exit 2), `walk_until_battle_alternates_direction`.
- [ ] **Step 2: Run, FAIL** (`cd scripts/g4emu && uv run pytest -m "not integration" -q`) → **Step 3: Implement** → **Step 4: Run, PASS**.
- [ ] **Step 5: Gated Kalibrierung** (Messung, kein Commit von ROM-Daten): auf der Kopie `$HOME/Projekte/lab/editor/Roms/_testkopien/pt-3b3b-enc` (Messdaten unter `_messung/`, ROM `out.nds` mit Mewtu-Gras) `uv run g4emu run --rom … --save … continue expect-map <neuer Header> walk-until-battle right 20 back-and-forth expect-battle 150 5 end-battle` → Erfolg; dasselbe auf `hg-3b3b-enc`. Protokoll in den Bericht.
- [ ] **Step 6: Commit** `feat(g4emu): battle detection by enemy signature, walk-until-battle, end-battle, win-battle, mash`

---

### Task 2: Kernel-Fachlogik: Begegnungsregeln, Trainer-Regeln, Event-Felder

**Files:**
- Create: `crates/g4-kernel/src/data/encounter_rules.rs` (+ Tests), `crates/g4-kernel/src/data/trainer_rules.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/data/mod.rs`, `apps/studio/src-tauri/src/commands/events.rs` (`EventPatch` + `trainer_type: Option<u16>`, `sight: Option<u16>` → `data[0]`; `ObjectEvent`-DTO liefert beide), `apps/studio/src/api.ts`, `workbench/inspector/eventFields.ts` (+ test: Felder „Sichtlinie“ (`trainer_type`, `SelectField` Aus | Trainer sieht dich (1)) und „Sichtweite“ (`sight`, `NumberField max=15`) für Objekt-Events)

**Interfaces:**
- `encounter_rules::shares(method: Method, family) -> &'static [u8]` (Gras 20/20/10/10/10/10/5/5/4/4/1/1; Pt Surfen und Alte Angel 60/30/5/4/1, Profi- und Super-Angel 40/40/15/4/1; HGSS Surfen 60/30/5/4/1, alle Angeln 40/30/15/10/5, Fels 80/20); `encounter_rules::replacements(family) -> Vec<Replacement { list: &'static str, slots: [usize; 2], when: &'static str }>` (Pt: `day`/`night` → 2, 3 „10 bis 3 Uhr“; `swarm` → 0, 1; `radar` → 4, 5, 10, 11; Dual-Slot → 8, 9; Trophäengarten → 6, 7; HGSS: Radio Hoenn/Sinnoh `[0]` → 2, 3 und `[1]` → 4, 5; Schwarm nach fester Header-Liste); `encounter_rules::warnings(enc: &Encounters, family) -> Vec<RuleWarning { method, slot: Option<usize>, text: String }>` (nur bei Rate > 0: Art belegt bei Level 0; min > max „das Spiel tauscht sie“; Level > 100; leerer Ersatz-Slot „Schwarm auf Gras 1 ergibt ein Glitch-Pokémon“; Pt Rate > 255 „wirkt wie <Rate mod 256>“, Rate 256 „nie“); `encounter_rules::effective_species(enc, family, slot) -> Vec<(&'static str, u16)>` („Tags“, „Nachts“, …) für die Schau „Wer hier wartet“ mit Gesamtanteil je Tageszeit.
- `trainer_rules::default_moves(learnset: &Learnset, level: u8) -> [u16; 4]` (pret-Regel aus der Entscheidungstabelle; 0 = leer), `trainer_rules::name_units(name: &str) -> usize` (UTF-16-Einheiten plus Endzeichen), `NAME_MAX_CHARS = 7`.
- Englische Kernel-Texte mit Zahlen; die UI übersetzt (Spec-Regel aus 3b.3a).

- [ ] **Step 1: Failing tests**: `grass_shares_sum_to_100_in_both_families`, `warnings_flag_level_zero_species_and_swapped_levels_only_when_rate_is_positive`, `default_moves_shifts_the_first_out_when_full` (Lucario Level 7 → 193, 197, 232, 68 nach trainer-mechanics §2 mit einem synthetischen Lernset derselben Form), `default_moves_skips_known_moves`, `event_patch_sets_trainer_type_and_sight` (`events.rs` Test über `*_impl`), `event_fields_offer_sight_only_for_objects`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (`cargo test --workspace --all-features data::`, `pnpm test`) → **Step 5: Commit** `feat(kernel,studio): encounter shares, replacements and warnings; default move rule; trainer type and sight on events`

---

### Task 3: Trainer als zusammengesetztes Asset im Store

**Files:**
- Modify: `crates/g4-kernel/src/project.rs` (`AssetKind::Trainer`, `Location::MemberPair { props: AssetKind, party: AssetKind }`, `member_count(Trainer) = min(props, party)`), `crates/g4-kernel/src/store.rs` (`Loaded::Trainer(TrainerDoc)`, kanonische Bytes `u32 len(props) | props | party`; `read_member`, `read_location`, `accept_disk_as_base`, `save` (beide Dateien mit `write_atomic`, Stale-Prüfung über beide), `is_editable`, typisierte `trainer(id)`, `edit_trainer`, `Tx::edit_trainer`, `Tx::trainer`), `crates/g4-kernel/src/test_support.rs` (Fixture: `trainerProperties`/`trainerParty` mit ≥ 3 Membern, einer mit Attacken und Items), `crates/g4-kernel/tests/store.rs`
- Create: `crates/g4-formats/src/data/trainer_doc.rs` (+ Tests)

**Interfaces:**
- `TrainerDoc { class: u8, format: PartyFormat, flags_rest: u8, unknown: u8, items: [u16; 4], ai: u32, battle_type: u32, party: Vec<PartyMon> }` mit `decode(props: &[u8], party: &[u8], family) -> Result<TrainerDoc, FormatError>` und `encode(&self, family) -> (Vec<u8>, Vec<u8>)`; Flag-Byte und `party_count` werden beim Schreiben aus `format` und `party.len()` abgeleitet; `party.len() ≤ 6`. Sperre, wenn `encode(decode(p, q)) != (p, q)`, Grund nennt beide Dateien (`"trainer 0012: party 20 bytes, expected 16"`).
- Proptest-Invariante: nach jeder Edit-Folge gilt Flag = `format`, `party_count = party.len()`.

- [ ] **Step 1: Failing tests**: `trainer_doc_round_trips_every_vanilla_shape` (proptest über `format`, `count` 0..6, `family`), `trainer_loads_byte_identical_from_the_pair`, `edit_trainer_records_one_undo_step_for_both_files`, `save_writes_both_members_atomically_and_stale_checks_both`, `mismatched_pair_is_locked_with_reason`; gated in `vanilla_data.rs`: `vanilla_trainers_decode_and_encode_identically` (928/738/738 Paare).
- [ ] **Step 2: Run, FAIL**; bestehende `tests/store.rs` bleiben grün → **Step 3: Implement** → **Step 4: Run, PASS** (auch gated gegen die drei Projekte) → **Step 5: Commit** `feat(kernel): trainer as a member pair asset with a decoded document`

---

### Task 4: Anlegen: `create_trainer` mit Scripterweiterung, `set_trainer_texts`, Begegnungsdateien

**Files:**
- Create: `crates/g4-kernel/src/data/trainer_create.rs` (+ Tests), `crates/g4-kernel/src/data/encounter_create.rs` (+ Tests)
- Modify: `crates/g4-kernel/src/scripts.rs` (Helfer `battle_script_file(project) -> Result<u16>` aus der Script-Tabelle: Pt 1114, HGSS 953 über Metadaten `metadata/trainer_scripts.yaml`: `pt: { battle_file: 1114, seen_script: 3928 }`, `hgss: { battle_file: 953, seen_script: 3739 }`; `extend_battle_slots(project, up_to_id: u16) -> Result<(), KernelError>`: liest die rotom-Quelle, prüft den „gesehen“-Slot gegen die Konstante, erweitert die Slot-Liste des Kampf-Scripts lückenlos um alle IDs bis `up_to_id` außer dem „gesehen“-Slot, schreibt über `write_source` und `compile`; bei Fehler alte Quelle zurück), `crates/g4-kernel/metadata/arm9_tables.yaml` (`trainer_flag_capacity: 1040` für CPUD, IPKD, IPKE), `crates/g4-kernel/src/store.rs` (`append_member` für `TrainerProperties`, `TrainerParty`, `Encounter`; Texte anfügen an `trtbl`/`trtblofs` über `edit_trainer_text_table`/`edit_trainer_text_offsets` in einer Transaktion), `crates/g4-kernel/tests/trainer_create.rs` (gated)

**Interfaces:**
- `create_trainer(project, store, names, class: u8, name: &str) -> Result<CreateTrainerReport { id: u16, placeholder: Option<u16>, script_extended: bool }, KernelError>`: Vorbedingungen (Ablehnung mit Grund, nichts angelegt): Scripts übernommen; `names.len(TrainerNames) == member_count(TrainerProperties)`; `trtblofs.len() ≥ member_count`; nächste ID < 1040; `name_units(name) ≤ 8`. Ablauf: `id = member_count`; ist `id == seen_id` (`seen_script − 3000 + 1`), erst einen Platzhalter anlegen (Eigenschaften 0, Party 8 Nullbytes, Name `"-"`, gesperrt über `locked_reason = "placeholder for the sight script slot"`) und `id += 1`; ist `id > letzte Slot-Nummer der Kampf-Scriptdatei`, `extend_battle_slots(id)`; dann Eigenschaften (Klasse, 0 Pokémon, Format ohne Attacken/Items), Party (8 Nullbytes), Name an Index `id` in der Namensbank (`trainer_name::wrap`), `trtblofs[id] = len(trtbl)` (Lücken bis `id` mit 0) in **einer** Transaktion „Trainer <id> angelegt“ für die Text-Assets; die Member-Anlagen sind nicht rückgängig (wie `append_member`).
- `set_trainer_texts(store, names, id, texts: Vec<TrainerText { kind: u16, text: String }>) -> Result<Option<ChangeId>>`: vorhandene Einträge des Trainers (zusammenhängender Block ab `trtblofs[id]`) werden geändert; neue Arten hängen Einträge an `trtbl` und Nachrichten an das Kampftext-Archiv an und setzen `trtblofs[id]`, wenn der Trainer noch keinen Block hat; Invariante `len(trtbl)/4 == Nachrichten` wird geprüft, sonst `ReadOnly` „trainer texts do not match the table“; Warnung (Problems, Quelle „Trainer“), wenn der Text der Art „vor dem Kampf“ nicht auf `\r` endet.
- `create_encounter_file(project, store, copy_of: Option<u16>) -> Result<u16>` (neuer Member am Ende: Kopie oder 424/196 Nullbytes; HGSS lehnt Index ≥ 255 ab „HeartGold kann höchstens 255 Begegnungsdateien ansprechen“); `split_encounter_file(project, store, header: u16) -> Result<(u16, Option<ChangeId>)>` (Kopie anlegen, dann im selben Changeset den Header auf die Kopie umhängen; das uxie-`MapHeader`-Feld der Begegnungsdatei heißt im Code wie in `data/refs.rs` benutzt).

- [ ] **Step 1: Failing tests** (Fixture mit übernommenen Scripts und einer kleinen Kampf-Scriptdatei mit Slots `#[1-3]` und „gesehen“ `#[4]`): `create_trainer_appends_props_party_name_and_offset`, `create_trainer_skips_the_seen_slot_with_a_locked_placeholder`, `create_trainer_extends_the_battle_script_slots_and_keeps_the_code` (Member-Bytes bis auf den Kopf gleich), `create_trainer_refuses_without_adopted_scripts`, `create_trainer_refuses_at_flag_capacity`, `set_trainer_texts_appends_a_contiguous_block_and_fills_gaps_with_zero`, `set_trainer_texts_warns_without_carriage_return`, `create_encounter_file_appends_a_member_of_the_right_size`, `split_encounter_file_is_one_history_step_for_the_header`; gated `trainer_create.rs`: auf `vanilla_copy()` von Platin DE zwei Trainer anlegen (928, Platzhalter 929, 930) und von HeartGold DE vier (738, 739, Platzhalter 740, 741); Compile ohne Fehler; `vanilla_adopts_and_compiles_clean` bleibt grün.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (gated gegen Pt DE und HG DE) → **Step 5: Commit** `feat(kernel): create trainers with battle script extension and texts; create and split encounter files`

---

### Task 5: Befehle und DTOs für Trainer und Begegnungen; Referenzen

**Files:**
- Modify: `apps/studio/src-tauri/src/commands/data/dto.rs`, `apps/studio/src-tauri/src/commands/data/*.rs` (`data_get`/`data_edit` für `trainer` und `encounter`; Platzhalter-Fehler „kommt mit 3b.3b“ entfällt), neue Befehle `create_trainer`, `set_trainer_texts`, `create_encounter_file`, `split_encounter_file` in `commands/data/`, `bridge/dispatch.rs`, `lib.rs`, `apps/studio/src/api.data.ts` (+ test), `apps/studio/src/assets.ts` (Label „Trainer“), `apps/studio/src/reload.ts`, `crates/g4-kernel/src/data/refs.rs` (Trainer-Quellen über `Loaded::Trainer` statt Rohcodec; Objekt-Event → Trainer bleibt)

**Interfaces:**
- `DataDocDto::Trainer { class: u8, name: String, format: { moves: bool, items: bool }, ai: u32, battle_type: u32, items: [u16; 4], party: Vec<PartyMonDto { iv_or_difficulty: u8, byte1: u8, level: u16, species: u16, form: u8, item: Option<u16>, moves: Option<[u16; 4]>, ball_capsule: Option<u16>, gender: Option<u8>, ability: Option<u8> }>, texts: Vec<TrainerTextDto { kind: u16, archive: u16, message: u16, text: String }>, events: Vec<RefDto> (Objekt-Events mit diesem Trainer), locked, placeholder: bool }`; `DataPatch::Trainer { field: String, value: i64 }` (`class`, `ai`, `battle_type`, `item0..3`), `DataPatch::TrainerName { name }` (Grenze 7 Zeichen, Ablehnung mit Grund), `DataPatch::TrainerFormat { moves: Option<bool>, items: Option<bool> }` (Einschalten der Attacken füllt per `default_moves` aus dem Lernset der Grundform, Ausschalten verwirft; ein Undo-Schritt; Label „<Name>: eigene Attacken an“), `DataPatch::TrainerParty { party: Vec<PartyMonDto> }` (≤ 6), `DataPatch::TrainerTexts { texts }` → `set_trainer_texts`. `DataRowDto` für `trainer`: `extra: { class, class_name, level_max, party_count, places: Vec<{ header, name }> }` (Orte aus den Event-Referenzen; „Ohne Ort“ bei leer).
- `DataDocDto::Encounter { methods: Vec<MethodDto { method, rate, slots: Vec<SlotDto { index, species, min, max, share: u8, replaced_by: Vec<{ list, when }> }> }>, replacements: Vec<ReplacementListDto { list, label, slots: Vec<SlotDto> }>, extras: { form_rates?, unown_table? } (Pt), warnings: Vec<RuleWarningDto>, users: Vec<{ header, name }>, effective: Vec<{ when, species: Vec<{ species, share }> }>, locked }`; `DataPatch::Encounter { method: String, slot: Option<usize>, field: String, value: u32 }` (`rate`, `species`, `min`, `max`, `level`; Level-Klemmung nur bei Eingabe: min über max zieht max mit und umgekehrt), `DataPatch::EncounterExtra { field, value }`.
- Befehle: `create_trainer { class, name } -> CreateTrainerReport`, `set_trainer_texts { trainer, texts } -> Vec<AssetId>`, `create_encounter_file { copyOf? } -> u16`, `split_encounter_file { header } -> { index, changed: Vec<AssetId> }`. Warnungen der Begegnungen gehen zusätzlich in den `ProblemsStore` mit Quelle „Begegnungen“ (bei `data_get` und nach `data_edit`).

- [ ] **Step 1: Failing tests**: Rust `commands/data` Tests (`data_get_trainer_lists_party_texts_and_events`, `trainer_format_on_fills_default_moves_in_one_step`, `trainer_name_over_seven_chars_is_refused`, `data_get_encounter_carries_shares_and_warnings`, `encounter_min_above_max_pulls_max`, `create_trainer_command_reports_placeholder`); Dispatch-Fixture je Befehl; `commands_match_the_tauri_handler`; `api.data.test.ts`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): trainer and encounter documents, patches and creation commands in Tauri and the bridge`

---

### Task 6: `PokemonCard` und der Unterbereich Trainer

**Files:**
- Create: `ui/primitives/PokemonCard.tsx` (+ css, test), `ui/gallery/pages/DataPage.tsx` (Abschnitt), `data/trainer/TrainerList.tsx`, `TrainerView.tsx`, `TrainerHand.tsx` (Reiter Team · Trainer · Rohdaten), `trainerModel.ts` (+ test: KI-Stufen `einfach 0x1 · klug 0x7 · Experte 0xF · eigene`, benannte KI-Bits (11 nach dspre), `battle_type` Einzel/Doppel (0/2), HGSS Geschlecht/Fähigkeit aus `byte1`, Pt „Stärke (IV-Skala)“), `useTrainer.ts` (auf `useDataDoc`), Tests je Datei
- Modify: `data/PokemonArea.tsx` (Unterbereich `trainer` frei), `data/dataModel.ts`, `data/DataList.tsx` (Baum nach Ort), `ui/frame/shellState.ts` (`selected.trainer`), `app/paletteEntries.ts` (Gruppe „Trainer“), `ui/frame/breadcrumb.ts` (`Pokémon › Trainer › Wanderer Jens`)

**Interfaces:**
- `PokemonCard { state: "idle" | "selected" | "empty"; species?: number; name?: string; level?: number; item?: string; iconUrl?: string | null; onClick; ariaLabel }` (Fläche `flaeche/blatt`, gewählt mit Kobalt-Kante, leer gestrichelt mit „Platz 4 leer“; `role="button"`).
- Liste: `Segmented` „Nach Ort | Nach Klasse“; `TreeRow` Ort → Trainer (Name mit Klasse „Wanderer Jens“, Untertitel Kennung und „Lv. 12“ = höchstes Level), Gruppe „Ohne Ort“; Suche „Name, Klasse oder Nummer“; `+` „Trainer anlegen…“ öffnet `Dialog` „Trainer 928 anlegen?“ mit Klasse (`SelectField` aus der Klassenbank) und Name (`TextField`, Hinweis „Höchstens 7 Zeichen“), Plakette „Nicht rückgängig“; nach dem Anlegen ist der Trainer gewählt; ein Platzhalter-Trainer steht gesperrt in der Liste („Platzhalter für das Sicht-Script“). `data-trainer={id}`.
- Schau: Teamübersicht (bis sechs `PokemonCard`), Kopf mit Klasse, Name, `Badge` Doppelkampf; „Steht in der Welt“: Event-`Reference`s (Ort, Kachel) mit `goTo` Welt, sonst „Noch nicht platziert“ mit Taste „In der Welt platzieren“ (Task 8); `Notice kind="warning"` „Ohne Pokémon stürzt der Kampf ab“ bei leerem Team.
- Hand: Reiter Team (`PokemonCard` × 6, Klick öffnet die Platz-Details: Art per `Picker` mit `TypeBadge`-Vorschau, Level `NumberField min=1 max=100`, Item `Picker`, vier Attacken `Picker` (nur bei „Eigene Attacken“), Stärke/`difficulty` `NumberField bits=8` („IV = Wert × 31 / 255“ in HGSS), HGSS Geschlecht/Fähigkeit `SelectField` (Auto/männlich/weiblich; Auto/1/2), Form nur bei Arten mit Formen, Taste „Pokémon entfernen“), Trainer (Klasse `SelectField`, Name `TextField` mit „5 von 7“, Schalter „Eigene Attacken“ und „Items tragen“ (`Switch`, Hinweis auf die Füllregel), Kampftyp `Segmented` Einzel | Doppel, KI `Segmented` einfach | klug | Experte | eigene, vier Items `Picker`, Kampftexte als drei `TextField`s (vor dem Kampf, Niederlage, danach; fehlende Arten als „Text anlegen“), Warnung „sollte auf ⏎ enden“ aus den Problems), Rohdaten (Tüfteln: `HexBlock` beider Member, alle 32 KI-Bits als `Checkbox`-Liste).

- [ ] **Step 1: Failing tests**: `PokemonCard` Zustände und `role`, `TrainerList` Baum nach Ort und Suche, „Trainer anlegen…“ ruft `create_trainer` und wählt die ID, `TrainerHand`: Level ändern ruft `data_edit` mit `TrainerParty` und Label „Wanderer Jens: Platz 1 Level 12 → 14“, Schalter füllt Attacken (Label), Name mit 8 Zeichen wird abgelehnt und zeigt den Grund, Kampftext ohne `\r` zeigt die Warnung, gesperrter Platzhalter sperrt die Felder.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**; Browser auf einer Platin-Kopie: Trainer anlegen, Team füllen, Texte, ⌘Z.
- [ ] **Step 5: Commit** `feat(studio): trainer sub-area with team cards, properties, texts and creation dialog`

---

### Task 7: Unterbereich Begegnungen und das Header-Feld „Begegnungen“

**Files:**
- Create: `data/encounters/EncounterList.tsx`, `EncounterView.tsx`, `EncounterHand.tsx`, `encounterModel.ts` (+ test: Methodenreiter je Familie Gras · Surfen · Fels (HGSS) · Alte Angel · Profi-Angel · Super-Angel · Ersatz; Spaltenmodell HGSS Morgens/Tags/Nachts), `useEncounter.ts`, Tests
- Modify: `data/PokemonArea.tsx` (Unterbereich `encounters` frei), `workbench/inspector/HeaderEditor.tsx` (+ test: Feld „Begegnungen“ als `Reference area="pokemon"` mit `Picker` der Dateien (Vorschau: Orte, die sie nutzen) und Option „Keine“ (Pt 0xFFFF / HGSS 0xFF); Ändern ist eine rückgängige Header-Änderung), `app/paletteEntries.ts` (Gruppe „Begegnungen“), `ui/frame/breadcrumb.ts` (`Pokémon › Begegnungen › Route 201`), `app/statusContext.ts`

**Interfaces:**
- Liste „Begegnungen“: Dateien beschriftet mit den Orten der nutzenden Header (aus `users`), Untertitel Kennung und „verwendet von N Orten“; Gruppe „Ohne Ort“; `+` „Begegnungsdatei anlegen…“ (`Dialog` mit `Segmented` Leer | Kopie von … (`Picker`), Plakette „Nicht rückgängig“). `data-encounter={index}`.
- Schau „Wer hier wartet“: Arten mit Gesamtanteil je Tageszeit aus `effective` (Zeilen mit `Sprite`, `TypeBadge`, „Tags 40 %“), „Verwendet von N Orten“ als `Reference`s (Welt) mit Taste „Für diesen Ort abspalten“ bei N > 1 (`Dialog` als Frage, Plakette „Nicht rückgängig“ für die Kopie, Hinweis „Das Umhängen des Headers ist rückgängig“); Warnliste aus `warnings` (`Notice kind="warning"`).
- Hand: `Tabs` je Methode mit Zähler belegter Slots; Rate als `NumberField` („0 = nie“; Pt `max=255` mit Hinweis „Das Spiel liest nur das untere Byte“); `Table` Platz · Pokémon (`Picker`) · Level (min–max als zwei `NumberField`, Gras nur Level) · Anteil (Mono „20 %“) · Ersetzt durch (Text aus `replaced_by`, leer gedämpft); HGSS-Gras mit drei Art-Spalten und gemeinsamer Level-Spalte; Reiter „Ersatz“ mit den Ersatzlisten (Tag/Nacht, Radar, Dual-Slot, Schwarm bzw. Radio, Schwarm); Pt-Extras: Formraten als `Switch` West | Ost (Ost schreibt 100), Unown `SelectField`; leere Slots gedämpft, gültig; Rohdaten in Tüfteln.

- [ ] **Step 1: Failing tests**: Liste nach Orten, Anlegen ruft `create_encounter_file`, Abspalten ruft `split_encounter_file` und wählt die Kopie, Rate ändern ruft `data_edit` mit Label „Route 201: Gras-Rate 30 → 100“, Slot-Art ändern, min über max zieht max (DTO-Antwort), HGSS drei Spalten, Warnung erscheint inline und im Prüflicht, Header-Editor-Feld setzt die Begegnungsdatei und ist rückgängig.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**; Browser: Route 201 öffnen, Gras-Slot 0 ändern, Abspalten auf einer Datei mit mehreren Nutzern.
- [ ] **Step 5: Commit** `feat(studio): encounters sub-area with shares, replacements, warnings, new files and split; header field for the encounter file`

---

### Task 8: Event ↔ Trainer in der Welt

**Files:**
- Modify: `workbench/inspector/EventEditor.tsx` (+ test: Feld „Trainer“ als `Reference area="pokemon"` aus `script` 3000–4999 abgeleitet (`script − 3000 + 1`), `Picker` der Trainer setzt `script`, `trainer_type = 1` und `sight = 3`, wenn beide 0 sind; außerhalb des Bereichs „kein Trainer“; `Reference` öffnet Pokémon › Trainer), `workbench/inspector/eventFields.ts` (Felder aus Task 2 angezeigt), `ui/frame/shellState.ts` (flüchtig, nicht gespeichert: `pendingTrainer?: number`), `App.tsx` / `app/` (Taste „In der Welt platzieren“ aus Task 6: `setArea("welt")`, `setTool("figure")`, `pendingTrainer = id`; die nächste per Werkzeug Figur gesetzte Figur bekommt `trainer_type = 1`, `script = 3000 + id − 1`, `sight = 3` in einem Changeset „Trainer Jens platziert“; Statusleiste „Klick setzt Wanderer Jens“; Escape hebt die Bindung auf), `workbench/ContextPanel.tsx` (Trainer-Events zeigen Klasse und Name), `crates/g4-kernel/src/data/refs.rs` (Objekt-Event ↔ Trainer bleibt, Task 5)

- [ ] **Step 1: Failing tests**: `EventEditor` zeigt „Sebastian“ für Script 3355, Picker setzt Script und Sichtlinie, „In der Welt platzieren“ wechselt Bereich und Werkzeug, der nächste Figur-Klick ruft `add_event` und `update_event` mit den drei Feldern in einem Verlaufsschritt (`App.test.tsx`), Escape löst die Bindung.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): trainer field on object events and placing a trainer in the world`

---

### Task 9: Szenarien, Gate-Dokument Teil B, Rezepte, Docs, Roadmap, volle Prüfung

**Files:**
- Create: `scripts/agent/scenarios/pt-trainer-begegnung.yaml`, `scripts/agent/scenarios/hg-trainer-begegnung.yaml`
- Modify: `scripts/g4emu/g4emu/scenario.py` (+ `test_scenario.py`: `edits` dürfen `create_area`, `create_encounter_file`, `create_trainer`, `set_trainer_texts`, `add_event`, `update_event`, `set_header`, `new_script_file`/`save_script` aufrufen; ein `edits`-Eintrag darf das Ergebnis eines vorigen als Platzhalter nutzen: `{$trainer1}` aus der Antwort von `create_trainer`), `docs/phase-3b3-gate.md` (Teil B nach Spec §1 Gate 3b.3b mit Ergebnistabelle, Spalte „Wer“), `docs/agent-gate.md` (Rezepte Trainer, Begegnungen, Platzieren; neue `g4emu`-Schritte), `docs/dev-setup.md`, `docs/ROADMAP.md` (3b.3 „3b.3b implementiert, Gate offen“), `design-system/README.md` („Was die App umsetzt“ + 3b.3b: `PokemonCard`; Abweichungen: keine Hand-Höhe je Unterbereich (bleibt offen, Grund: Deck-Umbau), Kampfsprites später), `docs/formats/trainer_scripts.md` (Struktur der Kampf-Scriptdatei aus dem Spike)

**Szenario `pt-trainer-begegnung.yaml`** (`game: CPUD`; Platzhalter `S_WILD` 150, `L_WILD` 5, `S_T1` 25 Pikachu, `L_T1` 12, `S_T2` 133 Evoli, `L_T2` 14, `S_GIFT` 445 Knakrack): `edits`: (1) Patch anwenden (der Befehl, den der Assistent nutzt; Name in `api.ts` neben `header_patch_status` prüfen), `create_area` aus einer Route-201-Karte mit hohem Gras als neuer Header (`new_header`), leere Eventdatei; (2) `create_encounter_file` leer, `set_header` `wild_pokemon = $enc`, `data_edit encounter` Gras-Rate 100, Slots 0–11 und `day`/`night` auf `S_WILD`/`L_WILD`; zusätzlich die Begegnungsdatei von Route 201 (Header laut `list_headers`, Datei aus `get_header`) Gras-Slot 0 auf `S_WILD` (Prüfung im Build); (3) `create_trainer` × 2 (Klasse Wanderer, Namen „Jens“ und „Mia“; Pt liefert 928 und 930 mit Platzhalter 929), `data_edit trainer` Team je ein Pokémon (`S_T1`/`L_T1`, `S_T2`/`L_T2`), `set_trainer_texts` (vor dem Kampf mit `\r`, Niederlage, danach); (4) `add_event` × 2 Objekte auf dem neuen Ort in Sichtlinie quer zum Weg, `update_event` mit `script = 3000 + id − 1`, `trainer_type = 1`, `sight = 3`, Blickrichtung; (5) Geber: `add_event` Objekt neben der Ankunftskachel, `new_script_file`/`save_script` mit rotom-Quelle (`GivePokemon` 445 Level 100 einmalig über ein Flag, Textliteral „Hier, nimm das.“), Script am Event; (6) `update_event` Treppe des Spielerzimmers (Eventdatei 394, Warp 0) auf den neuen Ort, Ankunfts-Warp im neuen Ort (Werkzeug-Regel aus `agent-gate.md`: Warp-Kachel plus Tür-Verhalten); (7) `save`, `build: out.nds`; `check`: `g4k check-data out.nds --expect encounter:<route201>:grass0=S_WILD`; `play`: `continue`, `expect-map 415`, `walk-to 8 4`, `walk right 1`, `expect-map $NEU`, `wait 150`, `walk-to <Geber>`, `press a`, `mash a 300`, `walk <Richtung> N` in die Sichtlinie von Trainer 1, `expect-battle S_T1 L_T1`, `shot trainer1`, `win-battle`, `expect-map $NEU`, weiter zu Trainer 2, `expect-battle S_T2 L_T2`, `shot trainer2`, `win-battle`, `walk-until-battle up 40 back-and-forth`, `expect-battle S_WILD L_WILD`, `shot wild`. `hg-trainer-begegnung.yaml` entsprechend (`game: IPKD`, Zimmer 64, Treppe (2,4) nach unten, Trainer 738 und 739, `GiveMon` 137, Gras-Spalten Morgens/Tags/Nachts, Heim 63).

- [ ] **Step 1: `scenario.py`** Platzhalter aus Antworten (`$name`) mit Test; dann beide Szenarien auf frischen Kopien (`pt-3b3b`, `hg-3b3b` unter `_testkopien`, `--out target/agent/gate-2026-10-24/<name>`) bis grün; Befunde in Code oder Szenario beheben (TDD für Code).
- [ ] **Step 2: Docs** nach Dateiliste; Gate-Dokument Teil B mit Spalte „Wer“ (Agent: Szenarien, Editoren über die Test-Umgebung, ⌘Z, Abspalten, Tastatur, Dunkel/Kompakt; Hand: Blick auf die Screens „Trainer“ und „Begegnungen“, DSPRE, HeartGold US im Spiel optional).
- [ ] **Step 3: Volle Prüfung**: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `cargo test --workspace --all-features`, `cd scripts/g4emu && uv run pytest -q`, dann `G4_ROMS_DIR=<absolut> scripts/gate.sh` im Hintergrund (rund 35 min); Tabelle in den Bericht.
- [ ] **Step 4: Commit** `feat(gate): trainer and encounter scenarios for Platinum and HeartGold; gate document part B, recipes and roadmap`

---

## Reihenfolge und Parallelität

- Parallel: **1** (g4emu) ∥ **2** (Kernel-Regeln, Event-Felder) ∥ **3** (Trainer-Asset). Dann **4 → 5**. Danach **6 ∥ 7** (getrennte Ordner `data/trainer`, `data/encounters`; `PokemonArea.tsx` vorher mit beiden Einhängepunkten), dann **8 → 9**.

## Abschluss

Nach Task 9: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`), danach die Abnahme über die Test-Umgebung (Gate 3b.3 Teil B) in einer eigenen Session; Befunde werden dort behoben. Danach beginnt 3b.4 (Import und Prüfungen) nach eigener Spec und eigenem Plan.
