# Phase 3b.3a „Codecs und Pokémon-Daten“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eigene, byte-treue Codecs für Begegnungen, Trainer, Pokémon-Basisdaten, Lernsets, Entwicklungen, Attacken, Gegenstände und Trainer-Texttabellen in `g4-formats`; die editierbaren Arten im Store mit mehrteiligem Changeset, Namenstabellen, arm9-Tabellen (lesend), Referenzindex und Item-Symbolen; die Editoren Pokédex, Attacken und Gegenstände im Doppeldeck-Rahmen.

**Architecture:** `g4-formats/src/data/` hält die Codecs als Rohbyte-Overlays mit Feldkarte (fester Länge) bzw. `entries + tail` (variabler Länge); uxie bleibt Test-Orakel. `g4-kernel` bekommt neue `AssetKind`s mit `Location::Member`, `Store::transaction` für Changesets mit mehreren Teilen, `data/{names,refs,icons}.rs` und Metadaten-YAML. Drei generische Tauri-Befehle (`data_list`, `data_get`, `data_edit`) plus `data_raw`, `name_tables`, `item_table`, `machine_table`, `refs`, `get_item_icon`, alle auch in der Brücke. Das Frontend bekommt neue Primitives (`TypeBadge`, `StatBar`, `Tabs`, `Table`, `FilterChips`, `ToggleGrid`, `HexBlock`, `Sprite`), den Unterbereich-Umschalter in `PanelHead` und die Bereiche Pokémon und Gegenstände als Deck mit Liste, Schau und Hand.

**Tech Stack:** Rust (g4-formats, g4-kernel, Tauri 2, proptest, uxie 0.9.7 als dev-dependency), React 19 + `ui/` aus 3c (Base UI 1.9, CSS Modules), vitest 5, Python `g4emu` für das Backend-Szenario.

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md](../specs/2026-10-23-phase-3b3-data-editors-design.md). Byte-Layouts: [docs/research/2026-10-23-3b3/dspre-report.md](../../research/2026-10-23-3b3/dspre-report.md) §1.1–§1.6, Vanilla-Zahlen und uxie-Typen: [uxie-report.md](../../research/2026-10-23-3b3/uxie-report.md) §1, §4; Beschriftungen der Screens: [design-report.md](../../research/2026-10-23-3b3/design-report.md) §1, §2, §5, §7.

## Global Constraints

- Voraussetzung: 3c.1 und 3c.2 sind auf `main` (Primitives, `PanelHead`, `Picker`, `Reference`, `Deck`, `Hinge`, `shellState`, `history()`, Galerie). Der Plan startet auf einem Branch ab diesem Stand.
- Die Vanilla-Projekte unter `../Roms/*_DSPRE_contents` werden nie beschrieben; schreibende Tests laufen auf `vanilla_copy()`, das GUI nur auf Kopien unter `../Roms/_testkopien`.
- Testbefehle aus der Repo-Wurzel mit `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`. Vor jedem Commit: `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`; in `apps/studio`: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`.
- Gated Tests: `G4_TEST_PROJECT="$PWD/../Roms/<Projekt>_DSPRE_contents"`, Projekte `Pokemon - Platin-Edition (Germany)` (CPUD), `Pokemon - Goldene Edition HeartGold (Germany)` (IPKD), `Pokemon - HeartGold Version (USA)` (IPKE); Soll-Member-Zahlen (Pt / HGSS): encounters 183/142, trainerProperties 928/738, trainerParty 928/738, personalPokeData 508/508, learnsets 508/508, evolutions 508/508, moveData 471/471, itemData 446/514, trainerTextTable 1/1, trainerTextOffset 1/1. Ein gated Test, der 0 Member sieht, schlägt fehl.
- Codecs lehnen nie unbekannte Bits oder Werte ab; nur eine falsche Länge ist ein Fehler. Unbekannte Bits bleiben Rohwerte.
- DSPRE-Layout 1:1: Ordner `unpacked/<name>` aus Spec §2; neue Dateien nur über `fsutil::write_atomic`; in Member-Ordnern liegen nur `NNNN`-Dateien.
- arm9 wird gelesen, nie geschrieben. Offsets nur aus `metadata/arm9_tables.yaml` (CPUD, IPKD, IPKE geprüft; andere Codes `unverified: true`).
- UI-Texte deutsch (Begriffe aus `terms.ts`, Schreibweise „Script“); Code, Kommentare, Commits englisch; Styling nur über `ui/` und `--g4-*`-Tokens, kein `style={{` ohne `/* dyn */`, keine Hex-Farben (Gate-Schritt „styles“).
- Jeder neue Tauri-Befehl steht in `apps/studio/src-tauri/src/bridge/dispatch.rs` (`COMMANDS`, `run!`-Arm, Fixture in `every_command_accepts_the_keys_api_ts_sends`), in `generate_handler!` und in `api.ts`.
- Keine Nintendo-Grafiken im Repo: Tests für NCGR/NCLR nutzen synthetische Bytes; die App rendert nur aus dem geöffneten Projekt.
- `CmdError.code` bleibt `stale | read_only | tool_missing | partial | error | compile`; „nicht verfügbar“ (arm9) ist kein Fehler, sondern `null` im DTO.
- Diamant/Perl: Metadaten kennen die Pfade (`unverified`), `data_list` liefert für `Family::Dp` den Sperrgrund „Diamant/Perl: ohne Testprojekt nicht freigegeben“ und keine Zeilen.

## Review Focus

- Ein Member mit abweichender Länge (Hack-Projekt, 36-B-Item aus DSPRE): das Asset ist gesperrt mit Grund „Gegenstand 0017: 36 Bytes statt 34“, die Liste zeigt es, nichts stürzt (Task 7, Task 14).
- Item-Tabelle unplausibel (arm9-Patch, fremder Spielcode): `item_table` liefert `null`, die Liste zeigt „Daten 0017“ statt Namen, das TM-Raster zeigt Nummern ohne Attackennamen (Task 5, Task 14, Task 16).
- Personal-Member 496–507 (Formen) und Nachrichten 494/495 (Ei): die Liste nennt „Deoxys (Angriff)“ aus `species_forms.yaml`, nie „??? #503“ ohne Kennung; ein Species-Index ohne Namen heißt „Art 0503“ (Task 8, Task 14).
- Lernset mit 20 Einträgen: „Zeile hinzufügen“ ist gesperrt mit Hinweis „20 von 20“; ein Lernset ohne 0xFFFF-Ende wird gelesen und beim Schreiben korrekt terminiert (Task 3, Task 14).
- Mehrteiliger Changeset, dessen zweiter Teil fehlschlägt (gesperrtes Lernset bei „Wie Staralili“): kein Asset ändert sich, der Fehler nennt das gesperrte Asset (Task 6, Task 14).
- Undo über 200 Datenänderungen in Folge (Scrubbing): jede Änderung ist ein Schritt mit sprechendem Label; `history()` mit 200 Schritten bleibt unter 50 ms (Task 6, Task 11).

---

### Task 0: Prüfungen für 3b.3a (Pokémon-Symbol-Paletten, Formnamen, Nachrichtenzahlen)

**Files:**
- Create: `docs/research/2026-10-23-3b3/task0-3b3a.md`
- Create (nur Skripte, Scratch): `docs/research/2026-10-23-3b3/tools/` wird nicht angelegt; Werkzeuge liegen im Scratch und werden nicht committet

**Interfaces:**
- Produces: eine Tabelle mit geprüften Werten je Spielcode für Task 5 (`arm9_tables.yaml`: `mon_icon_palettes` Offset oder `null`), Task 9 (`species_forms.yaml`: Liste der zwölf Formnamen für Member 496–507 je Familie) und Task 11 (`monIcons`-Layout: Member 0 NCLR, ab Member 7 NCGR 32 × 64 4 bpp; Zuordnung Art → NCGR-Member = Art + 7).

- [ ] **Step 1: Pokémon-Symbol-Paletten-Tabelle** (read-only auf den drei Projekten): die Kandidaten Pt DE `0xF07D8`, HG DE `0xFFBC4`, HG US `0xFFC10` in `arm9/arm9.bin` lesen (508 Bytes ab dort); plausibel, wenn jeder Wert ∈ {0, 1, 2} und drei Symbole (Bisasam, Glumanda, Schiggy), mit der Palette ihres Tabellenwerts aus NCLR-Member 0 gerendert, per Augenschein stimmen (PNGs im Scratch, Beschreibung im Bericht). Ergebnis: Offset je Code oder `null`.
- [ ] **Step 2: Formnamen**: aus `pokeplatinum`/`pokeheartgold` (pret, `include/constants/species.h`) die Reihenfolge der Member 496–507 (Deoxys Angriff/Verteidigung/Initiative, Wormadam Sand/Trash, Giratina Ur, Shaymin Himmel, Rotom × 5) je Familie bestätigen; Tabelle mit Member → Basisart → Formname (deutsch).
- [ ] **Step 3: Nachrichtenzahlen** (Spec §2) mit chatot gegen die drei Projekte nachzählen (Species 496, Attacken 468, Items 468/537, Fähigkeiten, Typen, Klassen 105/129) und die Zuordnung Attacken-Nachricht i ↔ Attacke i, Item-Nachricht i ↔ Item-ID i stichprobenartig bestätigen (Flügelschlag = 17, Trank = 17); dazu die Archivnummern der Attacken- und Item-Beschreibungen je Familie bestimmen (Banken `move_descriptions`, `item_descriptions` für Task 5 und Tasks 15–16).
- [ ] **Step 4: Bericht schreiben** (Tabelle „geprüft / offen“), Commit `docs(3b3): task 0 checks for icon palettes, form names and name counts`.

---

### Task 1: Overlay-Grundbaustein in `g4-formats`

**Files:**
- Create: `crates/g4-formats/src/data/mod.rs`, `crates/g4-formats/src/data/overlay.rs`
- Modify: `crates/g4-formats/src/lib.rs` (`pub mod data;`), `crates/g4-formats/Cargo.toml` (`[dev-dependencies] proptest = "1"`, `uxie = "0.9"`)

**Interfaces:**
- Produces: `pub struct FieldSpan { pub name: &'static str, pub offset: usize, pub len: usize, pub kind: FieldKind }`, `pub enum FieldKind { U8, U16, U32, Bits { mask: u32 }, Bytes }`, `pub trait Overlay: Sized { const LEN: usize; const FIELDS: &'static [FieldSpan]; fn raw(&self) -> &[u8]; fn raw_mut(&mut self) -> &mut [u8]; fn read(bytes: &[u8]) -> Result<Self, FormatError>; fn write(&self) -> Vec<u8>; fn get_u8(&self, off: usize) -> u8; fn get_u16(&self, off: usize) -> u16; fn get_u32(&self, off: usize) -> u32; fn set_u8(&mut self, off: usize, v: u8); fn set_u16(&mut self, off: usize, v: u16); fn set_u32(&mut self, off: usize, v: u32); fn bits(&self, off: usize, mask: u32, shift: u32) -> u32; fn set_bits(&mut self, off: usize, mask: u32, shift: u32, v: u32); }` mit Standardimplementierungen über `raw`; ein Makro `fixed_overlay!(Name, LEN, [FieldSpan…])` erzeugt `struct Name { raw: [u8; LEN] }` samt `read` (Längenprüfung: `FormatError::Length { what, expected, got }`) und `write`. `FormatError::Length` wird in `error.rs` ergänzt.

- [ ] **Step 1: Failing tests** (`overlay.rs` Test-Modul): `read_rejects_wrong_length_with_expected_and_got` (`fixed_overlay!(Probe, 4, …)`, `Probe::read(&[0; 3])` → `Err(Length { expected: 4, got: 3 })`), `write_returns_the_raw_bytes_unchanged` (proptest über beliebige 4 Bytes), `setter_changes_only_its_own_bytes` (proptest: `set_u16(1, v)` lässt Byte 0 und 3 unverändert), `bits_round_trip` (`set_bits(0, 0x3F, 2, 21)` → `bits(0, 0x3F, 2) == 21`, andere Bits unverändert).
- [ ] **Step 2: Run, FAIL** (`cargo test -p g4-formats data::overlay`)
- [ ] **Step 3: Implement** nach Interfaces (Little-Endian wie der Rest der Crate).
- [ ] **Step 4: Run, PASS**; `cargo clippy`.
- [ ] **Step 5: Commit** `feat(formats): raw-byte overlay with field map for the data codecs`

---

### Task 2: Codecs fester Länge: Personal, Entwicklung, Attacke, Gegenstand

**Files:**
- Create: `crates/g4-formats/src/data/personal.rs`, `evolution.rs`, `moves.rs`, `item.rs`; `docs/formats/personal.md`, `docs/formats/evolution.md`, `docs/formats/move.md`, `docs/formats/item.md`
- Modify: `crates/g4-formats/src/data/mod.rs`

**Interfaces:**
- Produces: `Personal` (44 B; Getter/Setter `hp, atk, def, spd, spa, spdef: u8`, `type1, type2: u8`, `catch_rate, base_exp: u8`, `ev_yield(stat) -> u8` (2 Bit je Stat aus u16), `item1, item2: u16`, `gender_ratio, egg_cycles, base_friendship, growth_rate, egg_group1, egg_group2, ability1, ability2, flee_rate, color: u8`, `tm(n: u8) -> bool`, `set_tm(n, bool)` für n ∈ 0..100 (TM 1..92, HM 1..8), `tm_bits() -> [u8; 16]`), Layout nach dspre-report §1.4; `Evolution` (44 B; `entry(i: usize) -> EvoEntry { method: u16, param: u16, target: u16 }` für i ∈ 0..7, `set_entry`, 2 Pad-Bytes am Ende erhalten); `Move` (16 B; `effect: u16, category: u8, power: u8, move_type: u8, accuracy: u8, pp: u8, effect_chance: u8, target: u16, priority: i8, flags: u8` (alle 8 Bits, auch 0x40/0x80), `contest: u8, contest_type: u8`), Layout §1.5; `Item` (34 B; Felder nach §1.6: `price: u16, hold_effect, hold_param, pluck_effect, fling_effect, fling_power, natural_gift_power: u8, natural_gift_type: u8 (5 Bit) + flags, pocket: u8, field_pocket, battle_pocket, …, party params…`, als `FieldSpan`-Karte vollständig). Jede Struktur: `const FIELDS` vollständig (jedes Byte durch genau einen Span abgedeckt, Pad-Spans heißen `pad_N`).

- [ ] **Step 1: Failing tests** je Datei: `fields_cover_every_byte_exactly_once` (Summe der Spans == LEN, keine Überlappung), `read_write_round_trip_on_random_bytes` (proptest), ein Feldtest mit eingebetteten Vanilla-Bytes (beim Schreiben des Tests read-only aus Platin DE kopiert: Personal 397 Staravia mit `hp 55, atk 75, def 50, spd 80, spa 40, spdef 40, type1 0, type2 2`; Move 17 Flügelschlag `power 60, pp 35, move_type 2`; Item 17 Trank `price 300`; die übrigen Felder des Tests aus denselben Bytes ablesen), `tm_bits_round_trip` (`set_tm(39, true)` setzt genau ein Bit im TM-Block; Offset und Bit-Reihenfolge nach dspre-report §1.4, der Test fixiert beides).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** mit `fixed_overlay!` → **Step 4: Run, PASS**.
- [ ] **Step 5: Docs** `docs/formats/*.md` nach dem Muster von `docs/formats/narc.md` (Tabelle Offset, Länge, Feld, Quelle).
- [ ] **Step 6: Commit** `feat(formats): personal, evolution, move and item codecs as byte overlays`

---

### Task 3: Codecs variabler Länge: Lernset, Trainer-Eigenschaften, Trainer-Party, Trainer-Texttabellen

**Files:**
- Create: `crates/g4-formats/src/data/learnset.rs`, `trainer.rs`, `trainer_text.rs`; `docs/formats/learnset.md`, `docs/formats/trainer.md`, `docs/formats/trainer_text.md`

**Interfaces:**
- Produces: `Learnset { entries: Vec<LearnEntry { move_id: u16 (9 Bit), level: u8 (7 Bit) }>, tail: Vec<u8> }` mit `read(bytes)` (liest bis 0xFFFF oder Ende; fehlt 0xFFFF, ist `tail` leer und `terminated = false`), `write()` (Einträge, 0xFFFF, dann `tail` wenn Eintragszahl unverändert, sonst 0x0000-Füllung auf Vielfaches von 4), `MAX_ENTRIES = 20`; `TrainerProps` (20 B Overlay: `flags: u8` mit `has_moves() = bit 0`, `has_items() = bit 1`, `class: u8`, `unknown: u8`, `items: [u16; 4]`, `ai: u32`, `battle_type: u32`, `party_count: u8`; Layout §1.3); `PartyFormat { moves: bool, items: bool }` mit `entry_len(family: Family) -> usize` (Pt/HGSS 8/10/16/18, DP 6/8/14/16); `PartyMon { iv_or_difficulty: u8, byte1: u8, level: u16, species: u16 (10 Bit) + form (6 Bit) via `species()`/`form()`, item: Option<u16>, moves: Option<[u16; 4]>, ball_capsule: Option<u16> (nicht DP) }`; `Party { mons: Vec<PartyMon>, tail: Vec<u8> }` mit `read(bytes, format, count, family)` und `write(format, family)` (Füllung mit Nullen auf Vielfaches von 4, mindestens 8 B); `TrainerTextTable { entries: Vec<(u16 trainer, u16 kind)> }`, `TrainerTextOffsets { offsets: Vec<u16> }` (je read/write, Länge frei).

- [ ] **Step 1: Failing tests**: `learnset_reads_vanilla_bytes_and_writes_them_back` (eingebettete Bytes eines Vanilla-Lernsets mit 0xFFFF und Füllung), `learnset_with_changed_count_pads_to_multiple_of_four`, `learnset_without_terminator_is_read_and_written_terminated`, `party_round_trips_every_format` (proptest über `format`, `count` 0..6, `family`: `read(write(d)) == d` und `write` Länge ist Vielfaches von 4, mindestens 8), `empty_party_is_eight_zero_bytes`, `trainer_props_fields_cover_twenty_bytes`, `trainer_text_tables_round_trip`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Docs** → **Step 6: Commit** `feat(formats): learnset, trainer properties, party and trainer text codecs`

---

### Task 4: Codec Begegnungen (Pt und HGSS)

**Files:**
- Create: `crates/g4-formats/src/data/encounter.rs`, `docs/formats/encounter.md`

**Interfaces:**
- Produces: `PtEncounters` (424 B Overlay, Layout §1.1: `grass_rate: u32`, `grass(i) -> Slot { level: u8, species: u16, raw_species_word: u32 }` i ∈ 0..12, Ersatzlisten `day(i)`, `night(i)` i ∈ 0..2, `radar(i)` i ∈ 0..4, `dual(slot: DualSlot, i)` (Rubin, Saphir, Smaragd, Feuerrot, Blattgrün je 2), `swarm(i)` i ∈ 0..2, Wasser `surf(i)`, `old_rod(i)`, `good_rod(i)`, `super_rod(i)` i ∈ 0..5 als `WaterSlot { min: u8, max: u8, species: u16 }` (Byte-Reihenfolge Pt: max vor min; `min()`/`max()` verbergen das), `shellos_form: u8`-Felder, `unown(i)`), Setter je Feld; `HgssEncounters` (196 B, Layout §1.2: Raten `grass_rate, surf_rate, rock_rate, old_rod_rate, good_rod_rate, super_rod_rate: u8`, `grass_level(i): u8`, `grass_morning(i)`, `grass_day(i)`, `grass_night(i): u16`, `radio(i)` i ∈ 0..4 (`hoenn` 2, `sinnoh` 2), Wasser `WaterSlot` min vor max, `rock(i)` i ∈ 0..2, `land_swarm(i)`, `surf_swarm`, `night_fish`, `fish_swarm`); `enum Encounters { Pt(PtEncounters), Hgss(HgssEncounters) }` mit `read(bytes, family)` (DP wie Pt) und `write()`; gemeinsame Sicht `fn methods(&self) -> Vec<MethodView { method: Method, rate: u8/u32, slots: Vec<SlotView { index, species, min, max }> }>` für Kernel und Oberfläche.

- [ ] **Step 1: Failing tests**: `pt_fields_cover_424_bytes_once`, `hgss_fields_cover_196_bytes_once`, `pt_water_slot_min_max_order_is_hidden` (Bytes `[0x1E, 0x14, …]` → `min() == 20`, `max() == 30`; `set_min(25)` schreibt Byte 1), `hgss_water_slot_min_before_max`, `round_trip_random_bytes_both_families` (proptest), `methods_view_lists_grass_surf_and_rods_with_slot_counts` (Pt: 12/5/5/5/5, HGSS: 12/5/2/5/5/5).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Docs** → **Step 6: Commit** `feat(formats): encounter codecs for Platinum and HGSS with a shared method view`

---

### Task 5: Metadaten, arm9-Tabellen, gated Vanilla-Test mit uxie-Orakel

**Files:**
- Create: `crates/g4-formats/src/data/arm9_tables.rs`, `crates/g4-kernel/metadata/text_banks.yaml`, `crates/g4-kernel/metadata/arm9_tables.yaml`, `crates/g4-kernel/metadata/species_forms.yaml`, `crates/g4-formats/tests/vanilla_data.rs`
- Modify: `crates/g4-kernel/metadata/archives.yaml` (elf Arten aus Spec §2 je Familie; `by_game` für `IPG` Begegnungen `files/a/1/3/6`, `APA` Begegnungen `p_enc_data.narc`, `APA` Personal `poketool/personal_pearl/personal.narc`), `crates/g4-kernel/src/project.rs` (`ArchiveEntry.by_game: Option<BTreeMap<String, String>>`, `ArchiveMap::for_game(family, language, game_code)`, `AssetKind` + `ALL` + `label` + `slot` um `Encounter, Personal, Learnset, Evolution, Move, Item, TrainerProperties, TrainerParty, TrainerTextTable, TrainerTextOffsets`), bestehende Metadaten-Tests

**Interfaces:**
- Produces: `ItemTable { entries: Vec<ItemEntry { data: u16, icon: u16, palette: u16, gba: u16 }> }::read(arm9: &[u8], offset: usize, count: usize)`, `MachineTable { moves: [u16; 100] }::read(arm9, offset)`, `fn plausible(item: &ItemTable, machine: &MachineTable, item_members: usize) -> Result<(), String>` (TM01 = 264, HM05 ∈ {432, 250}, jeder `data` < `item_members`); YAML-Schemata: `arm9_tables.yaml`: `CPUD: { item_table: 0xF0D1C, item_count: 468, machine_table: 0xF0C54, mon_icon_palettes: <aus Task 0 oder null>, trainer_name_max: 7, trainer_flag_capacity: null }`, IPKD `0x100148 / 537 / 0x100080 / 734`, IPKE `0x100194 / 537 / 0x1000CC / 734`, andere Codes aus DSPRE mit `unverified: true`; `text_banks.yaml`: `pt: { species: 412, moves: 647, items: 392, abilities: 610, types: 624, trainer_names: 618, trainer_classes: 619, battle_texts: 617 }`, `hgss: { 237, 750, 222, 720, 735, 729, 730, 728 }`, `dp` aus uxie `TextBankIds` mit `unverified`; `species_forms.yaml`: Member → `{ base: u16, name: "Deoxys (Angriff)" }` je Familie (Task 0). `Project::archives.kinds()` zählt die neuen Arten; `Project::member_path(kind, index)` funktioniert für sie.

- [ ] **Step 1: Failing tests**: `embedded_metadata_parses_and_resolves_every_family` erweitert um die elf Arten und um Fälle `("IPG…", Encounter) == "files/a/1/3/6"`, `("APA…", Personal) == "…/personal_pearl/personal.narc"`; `arm9_tables_yaml_has_every_checked_code`; `plausible_rejects_a_data_index_beyond_the_members`; gated `vanilla_data.rs`: je Art alle Member aus `unpacked/<name>/` lesen, `write(read(x)) == x`, Soll-Zahl prüfen; Orakel: für Personal, Item, TrainerProps alle Felder gegen `uxie::PersonalData::from_bytes`, `uxie::ItemData::from_bytes`, `uxie::TrainerData` (Feldnamen aus uxie-report §1.3, §1.7, §1.2) vergleichen; `vanilla_pt_water_slots_have_min_le_max` (1078 belegte Slots); `vanilla_arm9_tables_are_plausible` (Item-Tabelle und TM-Liste je Projekt, TM01 = 264).
- [ ] **Step 2: Run, FAIL** (ohne `G4_TEST_PROJECT` überspringen die gated Tests mit „skipped“; mit Projekt rot, bis implementiert) → **Step 3: Implement** → **Step 4: Run, PASS** auch gated: `G4_TEST_PROJECT=<Platin DE> cargo test -p g4-formats --features test-support --test vanilla_data`, ebenso HG DE und HG US.
- [ ] **Step 5: Commit** `feat(formats,kernel): data archive kinds in the metadata, arm9 tables, gated vanilla round trip with uxie as oracle`

---

### Task 6: Mehrteiliger Changeset im Store

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (`Changeset { parts: Vec<Part>, label }`, `undo`, `redo`, `history`, `apply_edit` als Ein-Teil-Transaktion), `crates/g4-kernel/tests/store.rs`
- Create: `crates/g4-kernel/tests/transaction.rs`

**Interfaces:**
- Produces: `pub struct Part { pub asset: AssetId, before: Vec<u8>, after: Vec<u8> }`; `pub fn transaction<T>(&mut self, label: &str, f: impl FnOnce(&mut Tx<'_>) -> Result<T, KernelError>) -> Result<(T, Option<ChangeId>), KernelError>`; `Tx::edit(&mut self, id: AssetId, edit: impl FnOnce(&mut Loaded) -> Result<(), KernelError>) -> Result<(), KernelError>` (sammelt Parts; ein Asset höchstens einmal je Transaktion, sonst `KernelError::InvalidProject`); schlägt `f` fehl, bleibt jedes Asset wie vorher; ohne Änderung kein Changeset. `undo()`/`redo()` liefern weiter `Option<AssetId>` für Ein-Teil-Schritte und neu `Vec<AssetId>` über `undo_all()`/`redo_all()` (die bestehenden Befehle nutzen die neuen; die Tauri-Antwort wird `Vec<AssetId>`, `reload.ts` plant je Asset).

- [ ] **Step 1: Failing tests** (`transaction.rs`): `transaction_records_one_history_step_for_two_assets` (zwei Edits, `history().undo.len() == 1`, Label), `undo_restores_every_part_byte_for_byte`, `redo_reapplies_every_part`, `failing_closure_changes_nothing` (zweiter Edit auf gesperrtem Asset → Fehler nennt das Asset; beide unverändert; kein History-Eintrag), `same_asset_twice_is_refused`, `history_with_two_hundred_steps_answers_under_50_ms`; proptest `undo_redo_are_inverse_over_mixed_kinds` (zufällige Folgen aus Matrix-, Header- und Text-Edits und Transaktionen).
- [ ] **Step 2: Run, FAIL**; **die bestehenden `tests/store.rs` bleiben unverändert und müssen grün bleiben.**
- [ ] **Step 3: Implement**; `project.rs`-Befehle `undo`/`redo` (Tauri, Brücke) liefern `Vec<AssetId>`; `api.ts` `undo(): Promise<AssetId[]>`, `useHistoryReload.step` plant je Asset.
- [ ] **Step 4: Run, PASS** (`cargo test --workspace`, `pnpm test`).
- [ ] **Step 5: Commit** `feat(kernel): changesets with several parts and a store transaction; undo and redo return every restored asset`

---

### Task 7: Editierbare Datenarten im Store

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (`Loaded::{Encounter(Encounters), Personal(Personal), Learnset(Learnset), Evolution(Evolution), Move(Move), Item(Item), TrainerTextTable(..), TrainerTextOffsets(..)}`, `is_editable`, `from_bytes`, `to_bytes`, typisierte Getter `encounter(index)`, `personal(index)`, …, `edit_encounter`, `edit_personal`, `edit_learnset`, `edit_evolution`, `edit_move`, `edit_item` über `apply_edit`), `crates/g4-kernel/src/build.rs` (packt die neuen Ordner, wenn vorhanden; `TrainerProperties`/`TrainerParty` nur Build), `crates/g4-kernel/tests/store.rs`, `crates/g4-kernel/tests/build.rs`

**Interfaces:**
- Produces: `Store::personal(&mut self, index: u16) -> Result<&Personal, KernelError>` usw.; `edit_personal(&mut self, index, label, edit: impl FnOnce(&mut Personal)) -> Result<Option<ChangeId>, KernelError>` (Muster `edit_text_archive`); `member_count(kind)` für die neuen Arten; Sperrgrund „Gegenstand 0017: 36 Bytes statt 34“ über `unwritable_reason` (Längenfehler des Codecs wird zum Lock, nicht zum Ladefehler: `from_bytes` liefert `Err`, der Store sperrt das Asset mit dem Fehlertext).

- [ ] **Step 1: Failing tests**: je Art in `tests/store.rs` auf der synthetischen Fixture (`test_support`): `loads_byte_identical`, `edit_records_undo_and_dirty`, `save_writes_member_atomically`, `wrong_length_member_is_locked_with_reason`; gated: `vanilla_rebuild_matches_dsrom_cli` bleibt byte-identisch, obwohl die zehn neuen Ordner gepackt werden (`tests/build.rs`).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (gated gegen Platin DE und HeartGold DE).
- [ ] **Step 5: Commit** `feat(kernel): encounter, personal, learnset, evolution, move, item and trainer text assets in the store`

---

### Task 8: Namen (`data/names.rs`)

**Files:**
- Create: `crates/g4-kernel/src/data/mod.rs`, `crates/g4-kernel/src/data/names.rs`, `crates/g4-kernel/tests/names.rs`
- Modify: `crates/g4-kernel/src/lib.rs`, `crates/g4-kernel/src/store.rs` (Generationszähler je Text-Asset: `text_generation(index) -> u64`, steigt bei Edit, Undo, Redo, Reload, Accept)

**Interfaces:**
- Produces: `pub enum Bank { Species, Moves, Items, Abilities, Types, TrainerNames, TrainerClasses, BattleTexts }`, `pub fn bank_index(family: Family, language: Language, bank: Bank) -> Option<u16>` (aus `text_banks.yaml`), `pub struct NameTables` mit `names(&mut self, store: &mut Store, bank: Bank) -> Result<&[String], KernelError>` (Cache je Bank, invalidiert über `text_generation`), `species_name(&mut self, store, index: u16) -> String` (Formen aus `species_forms.yaml`, unbekannt „Art 0503“), `move_name`, `item_name` (Item-ID), `class_name`, `trainer_name` (ohne `{TRAINER_NAME:…}`); `pub mod trainer_name { pub fn unwrap(s: &str) -> String; pub fn wrap(s: &str) -> String }`.

- [ ] **Step 1: Failing tests**: `bank_index_knows_pt_and_hgss` (412/237 usw.), `unknown_species_is_named_by_number` („Art 0503“ ohne Formtabelle), `form_member_gets_its_form_name` („Deoxys (Angriff)“), `cache_invalidates_after_a_text_edit` (Name ändern über `edit_text_archive`, `names()` liefert den neuen), `trainer_name_wrap_unwrap_round_trip` (proptest über Namen ohne `{}`); gated `vanilla_name_counts_match_spec` (Species 496, Moves 468, Items 468/537, Trainernamen 928/738, Klassen 105/129, Kampftexte 2497/1717) und `every_vanilla_trainer_name_round_trips`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(kernel): name tables from the text archives with form names and trainer name wrapping`

---

### Task 9: Referenzindex (`data/refs.rs`)

**Files:**
- Create: `crates/g4-kernel/src/data/refs.rs`, `crates/g4-kernel/tests/refs.rs`
- Modify: `crates/g4-kernel/src/store.rs` (Änderungsmeldung: `Store::changed_since(gen: u64) -> Vec<AssetId>` oder ein `on_change`-Log; der Index fragt nach jedem Befehl), `crates/g4-kernel/src/data/mod.rs`

**Interfaces:**
- Produces: `pub enum Target { Species(u16), Move(u16), Item(u16), Trainer(u16), TrainerClass(u8), Encounter(u16) }`, `pub struct Ref { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub target: Target }`, `pub struct RefIndex` mit `refs(&mut self, store: &mut Store, target: Target) -> Result<Vec<Ref>, KernelError>` (lazy: erster Aufruf baut aus Store-Zustand, geladene Assets aktuell, andere von der Platte), `count(&mut self, store, target) -> usize`, `refresh(&mut self, store, changed: &[AssetId])` (nur Beiträge der Quellen neu), `rebuild_full(store)`; Quellen: HeaderTable `wild_pokemon` → Encounter (Pt u16, 0xFFFF keine; HGSS u8, 0xFF keine), Encounter → Species (alle Slots und Ersatzlisten), TrainerProperties/TrainerParty (über die Codecs, read-only) → TrainerClass, Item, Species, Move, Objekt-Event Script 3000–4999 → Trainer, Personal → Item (`item1`, `item2`), Learnset → Move, Evolution → Species/Item/Move je Methode (Methodentabelle aus §1.4: Item-Methoden 7, 8, 17, 18 …, Move-Methode 20; genaue Nummern aus dem Bericht in den Code als Konstante mit Quelle).

- [ ] **Step 1: Failing tests** (synthetische Fixture mit je zwei Membern je Art): `encounter_slots_reference_species`, `learnset_references_moves`, `evolution_item_method_references_item_not_species`, `header_references_its_encounter_file_and_none_for_ffff`, `event_script_3000_references_trainer_1`, `refresh_after_edit_updates_only_that_source`, proptest `incremental_equals_full_rebuild` über zufällige Edit/Undo/Redo-Folgen; gated `vanilla_index_builds_under_a_second` (Messung, Ziel 300 ms, Fehler erst über 1000 ms) und `vanilla_route_201_encounters_reference_known_species`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(kernel): reference index over data, headers and events with incremental refresh`

---

### Task 10: NCGR/NCLR-Dekoder und Item-Symbole

**Files:**
- Create: `crates/g4-formats/src/ncgr.rs`, `crates/g4-formats/src/nclr.rs`, `crates/g4-kernel/src/data/icons.rs`, `crates/g4-formats/tests/ncgr.rs`, `crates/g4-kernel/tests/icons.rs`, `docs/formats/ncgr_nclr.md`
- Modify: `crates/g4-formats/src/lib.rs`, `crates/g4-kernel/metadata/archives.yaml` (`item_icons`: Pt `files/itemtool/itemdata/item_icon.narc`, HGSS `files/a/0/1/8`, `unpacked: itemIcons`; `mon_icons` analog, Pfade aus uxie-report §4.1 bzw. DSPRE, `unverified` wo nicht geprüft), `project.rs` (`AssetKind::ItemIcons`, `MonIcons`, nicht editierbar, nur Zählung und Pfad)

**Interfaces:**
- Produces: `Nclr::read(bytes) -> Result<Nclr { palettes: Vec<[u16; 16]> }, FormatError>` (RGB555, Abschnitte `RLCM`/`PLTT`, nach öffentlicher Formatbeschreibung), `Ncgr::read(bytes) -> Result<Ncgr { width, height, bpp: 4|8, tiles: Vec<u8> }>` (`RGCN`/`CHAR`, tiled 8 × 8), `fn render_rgba(ncgr: &Ncgr, palette: &[u16; 16]) -> Vec<u8>` (RGBA8, Index 0 transparent), `fn png(rgba, w, h) -> Vec<u8>` (Crate `png`, falls nicht vorhanden hinzufügen; Lizenz MIT/Apache); Kernel `icons::item_icon(project, item_id) -> Result<Option<Vec<u8>>, KernelError>` (Symbol- und Palettenmember aus der Item-Tabelle; `None`, wenn die Tabelle unplausibel ist), `icons::mon_icon(project, species) -> …` nur, wenn Task 0 die Paletten-Tabelle bestätigt hat (sonst `None` und Kommentar mit dem Grund).

- [ ] **Step 1: Failing tests**: `nclr_reads_a_synthetic_palette_section` (selbst gebaute Bytes mit zwei Paletten; Farbe `0x7FFF` → `(255,255,255,255)`), `ncgr_reads_synthetic_4bpp_tiles_and_renders_in_tile_order` (2 × 2 Kacheln, Pixel (9, 0) liegt in Kachel 1), `index_zero_is_transparent`; gated `vanilla_item_icon_of_potion_decodes_to_a_non_empty_png` (Trank = Item 17; der Test prüft die PNG-Signatur und mehr als 0 opake Pixel; die Breite kommt aus dem NCGR).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Docs** → **Step 6: Commit** `feat(formats,kernel): NCGR and NCLR decoders, item icons from the ROM`

---

### Task 11: Befehle: Daten, Namen, Tabellen, Referenzen, Symbole (Tauri und Brücke)

**Files:**
- Create: `apps/studio/src-tauri/src/commands/data.rs`, `apps/studio/src/api.data.ts` (DTO-Typen und Befehle; `api.ts` re-exportiert), `apps/studio/src/api.data.test.ts`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `lib.rs` (`generate_handler!`), `bridge/dispatch.rs` (`COMMANDS`, Arme, Fixture), `apps/studio/src-tauri/src/state.rs` (Session hält `NameTables` und `RefIndex`), `apps/studio/src/assets.ts` (Labels der neuen Arten: „Begegnungen“, „Pokémon-Daten“, „Lernset“, „Entwicklung“, „Attacke“, „Gegenstand“, „Trainer-Eigenschaften“, „Trainer-Team“, „Trainertexte“, „Trainertext-Offsets“, „Item-Symbole“, „Pokémon-Symbole“), `apps/studio/src/reload.ts` (neue Arten → `{ kind: "dataPanel"; asset }`)

**Interfaces:**
- Produces (Rust DTOs, serde): `DataKind` ∈ `species | learnset | evolution | move | item | encounter | trainer` (`trainer` liefert in 3b.3a nur `data_list` lesend aus TrainerProps, kein `data_get`/`data_edit`: Fehler „kommt mit 3b.3b“), `DataRowDto { index: u16, name: String, tech: String, extra: serde_json::Value }` (Species: `{ types: [u8, u8], number: u16 }`; Move: `{ type: u8, category: u8, power, accuracy, pp }`; Item: `{ price, pocket, icon: bool, data_index }`; Encounter: `{ users: [u16] }`; Trainer: `{ class, party_count }`), `DataDocDto` (getaggt `kind`): `Species { personal: PersonalDto, learnset: Vec<LearnEntryDto>, evolution: [EvoEntryDto; 7], tm: Vec<bool> (100), locked: Option<String> }`, `Move { … }`, `Item { … , shared_with: u16 }`, `Encounter { methods: Vec<MethodDto>, extras, locked }`; `DataPatch` (getaggt): `Personal { field: String, value: u32 }`, `Tm { n: u8, on: bool }`, `Learnset { entries: Vec<LearnEntryDto> }`, `Evolution { slot: u8, method, param, target }`, `Move { field, value }`, `Item { field, value }`, `CopyMoves { from: u16 }` (TM-Bits und Lernset von Art `from` in einer Transaktion „Wie <Name>“), `Encounter { method, slot, field, value }`; `data_list(kind, filter?) -> Vec<DataRowDto>`, `data_get(kind, index) -> DataDocDto`, `data_edit(kind, index, patch, label?) -> Vec<AssetId>` (Label-Standard „<Name>: <Feld> <alt> → <neu>“), `data_raw(kind, index) -> RawDto { bytes: Vec<u8>, fields: Vec<FieldSpanDto>, path: String }`, `name_tables() -> NameTablesDto { species, moves, items, abilities, types, classes, forms }`, `item_table() -> Option<Vec<ItemEntryDto>>`, `machine_table() -> Option<[u16; 100]>`, `refs(target: TargetDto) -> Vec<RefDto { source: AssetId, field, slot, label: String, nav: NavTargetDto }>`, `get_item_icon(item) -> Option<Vec<u8>>` (Tauri: `tauri::ipc::Response` mit Bytes wie `get_object_sprite`). TS: gleichnamige Typen und Funktionen in `api.data.ts`; `kindLabel` erweitert.

- [ ] **Step 1: Failing tests**: Rust `commands/data.rs` Tests über `*_impl` auf der synthetischen Fixture (`data_list_species_names_forms`, `data_edit_personal_records_a_speaking_label`, `copy_moves_is_one_history_step`, `data_get_locked_member_reports_reason`, `item_table_is_null_when_implausible`); Dispatch: Fixture-Einträge für jeden Befehl; `commands_match_the_tauri_handler`; TS `api.data.test.ts` (Argumentnamen je Befehl).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (`cargo test`, `pnpm test`) → **Step 5: Commit** `feat(studio): generic data commands, name tables, arm9 tables, references and item icons in Tauri and the bridge`

---

### Task 12: Primitives: TypeBadge, StatBar, Tabs, Table, FilterChips, ToggleGrid, HexBlock, Sprite; `PanelHead.switcher`

**Files:**
- Create: in `apps/studio/src/ui/primitives/`: `TypeBadge.tsx`, `StatBar.tsx`, `Table.tsx`, `FilterChips.tsx`, `ToggleGrid.tsx`, `HexBlock.tsx`, `Sprite.tsx` (+ je `.module.css`, `.test.tsx`), `types.ts` (18 Typen Gen 4 mit deutschem Namen und Schlüssel in `tokens.ts` `typen`)
- Modify: `Tabs.tsx` (+ test; `Tabs` gibt es seit 3c.2, siehe unten: nur erweitern, nicht neu bauen), `PanelHead.tsx` (+ test), `ui/primitives/index.ts`, `ui/gallery/pages/` (Seite „Daten“ mit allen neuen Bausteinen)

**Interfaces:**
- Produces: `<TypeBadge type: number size?>` (Farbe und Name aus `types.ts`, unbekannt „Typ 19“ in `flaeche/deck`), `<StatBar label value max=255 average? deviates? onReset?>` (Strich am Schnitt, Wert rechts, `deviates` Kobalt-Strich), `Tabs` erweitert: seit 3c.2 gibt es `<Tabs items: {id, label, count?, unsaved?, closeLabel?, dataAttrs?}[] value onChange onClose? ariaLabel>children</Tabs>` (Base UI Tabs mit `activateOnFocus`, `role="tablist"`, Kobalt-Punkt bei `unsaved`, × nur mit `onClose`; das Script-Panel benutzt es); 3b.3a ergänzt nur, was die Reiter des Pokédex brauchen (etwa `children` als Funktion der aktiven id), ohne die bestehenden Props und Tests zu ändern, `<Table columns: {key, label, width?, align?}[] rows: {key, cells: Record<string, ReactNode>}[] selected? onSelect? ariaLabel>` (Roving Tabindex über Zeilen, ohne Virtualisierung), `<FilterChips options: {value, label, count?, color?}[] value: Set onChange max?=6 />` („n weitere“ als Popover, „Filter lösen“), `<ToggleGrid cells: {id, label, checked, tooltip, disabled?}[] columns onToggle>` (Raster aus `Checkbox`), `<HexBlock bytes: number[] fields: {name, offset, len}[] highlight?: string />` (16 je Zeile, Offsets mono, markiertes Feld in `auswahl/flaeche`), `<Sprite src: Uint8Array | null fallback: string size>` (Object-URL mit Cache je Schlüssel, Platzhalter Farbfläche mit Buchstabe), `PanelHead` neu `switcher?: { items: {id, label, count?}[], value, onChange }` (Titel wird `Menu`-Knopf „Pokédex ▾“).

- [ ] **Step 1: Failing tests** je Komponente über Rollen (`tab`/`tablist` nur für die Erweiterung von `Tabs`, `grid`/`row`, `checkbox`, `menu`), `TypeBadge` unbekannter Typ, `StatBar` Schnitt-Position (`style` `/* dyn */`), `HexBlock` markiert nur die Bytes des Felds, `PanelHead` mit `switcher` rendert ein Menü mit Zählern und ruft `onChange`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**; Galerie-Seite „Daten“; `bash scripts/check-styles.sh` grün.
- [ ] **Step 5: Commit** `feat(ui): type badge, stat bar, tabs, table, filter chips, toggle grid, hex block, sprite, panel head switcher`

---

### Task 13: Bereiche Pokémon und Gegenstände im Rahmen

**Files:**
- Create: `apps/studio/src/data/DataArea.tsx` (Bereich mit Unterbereich, Liste, Deck), `apps/studio/src/data/useDataArea.ts` (Zustand: `sub`, `selected` je Unterbereich, Suche, Filter; lädt `data_list` und `name_tables`, hört den Bus), `apps/studio/src/data/dataBus.ts` (Bus-Thema `{ topic: "dataChanged"; kind; index }`), `apps/studio/src/data/DataArea.test.tsx`
- Modify: `apps/studio/src/ui/frame/shellState.ts` (`g4.shell.v2`: `sub: Record<AreaId, string>`, `handHeight` je `area/sub`), `Atlas`-Sperren (Pokémon und Gegenstände frei), `workbench/layout.ts` (`DECK_PANELS.pokemon`, `DECK_PANELS.gegenstaende`: Schau `dataView`, Hand `dataEdit` mit `problems` als Reiter), `App.tsx` (Bereich rendert `DataArea`), `ui/frame/breadcrumb.ts` (Brotkrume `Pokémon › Pokédex › Staravia`; das zweite Glied öffnet den Umschalter), `bus/editBus.ts`, `useShortcuts.ts` (⌘1–⌘4 wechseln den Unterbereich im Bereich)

**Interfaces:**
- Consumes: `data_list`, `name_tables`, `PanelHead.switcher`, `Deck`, `Hinge`.
- Produces: `DataAreaContext { sub: DataSub; setSub; selected: number | null; select(index); rows: DataRowDto[]; names: NameTablesDto; reload() }` mit `type DataSub = "pokedex" | "trainer" | "encounters" | "moves" | "items"`; Trainer und Begegnungen sind in 3b.3a im Umschalter sichtbar, aber gesperrt mit Tooltip „Kommt mit 3b.3b“; `ListPane` je Unterbereich mit Suche; Deck-Panels `dataView` (Schau) und `dataEdit` (Hand) rendern je `sub` die Komponenten aus Task 14–16.

- [ ] **Step 1: Failing tests**: `DataArea.test.tsx`: Umschalter wechselt Liste und Brotkrume; ⌘2 wählt den zweiten Unterbereich; gesperrte Einträge rufen `setSub` nicht; Auswahl bleibt je Unterbereich erhalten; `shellState.test.ts`: `v2` liest `v1` nicht (Standard), `sub` je Bereich gespeichert.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** (Platzhalter `EmptyState` „Noch kein Eintrag gewählt.“ in Schau und Hand) → **Step 4: Run, PASS**, Browser: Atlas → Pokémon zeigt die Liste der Arten mit Namen und Nummern.
- [ ] **Step 5: Commit** `feat(studio): Pokémon and items areas with a sub-area switcher, list, view and hand docks`

---

### Task 14: Pokédex-Editor

**Files:**
- Create: `apps/studio/src/data/pokedex/PokedexList.tsx`, `PokedexView.tsx` (Schau), `PokedexHand.tsx` (Reiter Werte · Fähigkeiten und Typen · Lernset · Entwicklung · TM/VM · Rohdaten), `pokedexModel.ts` (+ test: Summen, Schnitt aller Arten, EV-Ertrag ↔ 2-Bit-Felder, Geschlechtsstufen `{0: "nur ♂", 31: "87,5 % ♂", 63: "75 % ♂", 127: "50 %", 191: "75 % ♀", 225: "87,5 % ♀", 254: "nur ♀", 255: "geschlechtslos"}`, Schlüpfschritte = Zyklen × 256, Entwicklungsmethoden mit Parameterart aus §1.4), `useSpecies.ts` (lädt `data_get`, hält Draft, `data_edit` je Feld, hört `dataChanged`), Tests je Komponente
- Modify: `apps/studio/src/data/DataArea.tsx` (rendert den Pokédex für `sub === "pokedex"`)

**Interfaces:**
- Consumes: `DataDocDto::Species`, `DataPatch::{Personal, Tm, Learnset, Evolution, CopyMoves}`, `refs`, `TypeBadge`, `StatBar`, `Tabs`, `Table`, `ToggleGrid`, `FilterChips`, `Picker`, `Reference`, `NumberField`, `SelectField`, `HexBlock`.
- Produces: Beschriftungen nach design-report §1 und §2: Liste „Pokédex“ mit Zähler „508 Arten“, Suche „Name, Nummer oder Kennung“ (Kennung = Formname oder Nummer; es gibt keine generierten `SPECIES_*`-Kennungen, die Zeile zeigt die Nummer vierstellig), Typ-`FilterChips`; Schau: Kopf (Name in `font-titel-s`, `TypeBadge` × 2, Fähigkeiten), sechs `StatBar`s mit Schnitt, Summe mit „vorher“ (Wert der letzten Speicherung: `useSpecies` merkt sich den Stand nach `save`), „Fundorte“ (`refs Species` gruppiert: Begegnungen als Welt-Chips mit Ortsname des nutzenden Headers, Trainer als Pokémon-Chips) und „Verwendet in“; Hand-Reiter: Werte (sechs `NumberField bits=8`, EV-Ertrag sechs `NumberField max=3`, Fangrate mit Satz „höher ist leichter“, Basis-EP, Wachstum `SelectField` (8 Stufen), Geschlecht `SelectField` (Stufen oben), Ei-Gruppen 2 × `SelectField` (15 Gruppen), Schlüpfen „15 Zyklen, 3.840 Schritte“), Fähigkeiten und Typen (2 + 2 `SelectField`), Lernset (`Table` Level · Attacke mit `Picker`, „Zeile hinzufügen“ gesperrt ab 20 mit „20 von 20“, Löschen je Zeile, Sortierung nach Level beim Speichern), Entwicklung (`Table` 7 Zeilen: Methode `SelectField`, Parameter als Level/Item-Picker/Attacke-Picker/Zahl je Methode, Ziel `Picker`), TM/VM (`ToggleGrid` 92 + 8, Tooltip „TM40 Aero-Ass“ aus `machine_table`, ohne Tabelle „TM40“; Taste „Wie …“ öffnet `Picker` und ruft `CopyMoves`), Rohdaten (nur Tüfteln: `HexBlock` je Teil-Asset mit `data_raw`, Dateipfad). Jedes Feld committet über `data_edit` mit Label; Fehler als `Notice` im Reiter. `data-species={index}` an Listenzeilen.

- [ ] **Step 1: Failing tests**: `pokedexModel.test.ts` (Summe, Schnitt, Geschlechtsstufen, Schlüpfschritte, EV-Bits), `PokedexHand.test.tsx` (Reiter; Angriff ändern ruft `data_edit` mit `Personal { field: "atk", value: 85 }` und Label „Staravia: Angriff 75 → 85“; Lernset-Grenze; „Wie …“ ruft `CopyMoves`; gesperrtes Asset zeigt Grund und sperrt Felder), `PokedexView.test.tsx` (StatBar-Werte, „vorher“, Fundort-Chips rufen `goTo`), `PokedexList.test.tsx` (Suche nach Nummer „397“, Typ-Filter, Member 500 heißt „Deoxys (Angriff)“, ohne Tabelle „Art 0500“; ohne `machine_table` zeigt das TM-Raster „TM40“ ohne Attackenname).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser auf einer Platin-Kopie: Staravia wählen, Angriff auf 85, ⌘Z, TM-Raster, „Wie Staralili“, ⌘Z nimmt beides in einem Schritt zurück (`call.sh history`).
- [ ] **Step 5: Commit** `feat(studio): Pokédex editor with values, abilities, learnset, evolution and TM tabs`

---

### Task 15: Attacken-Editor

**Files:**
- Create: `apps/studio/src/data/moves/MovesList.tsx`, `MoveView.tsx`, `MoveHand.tsx`, `moveModel.ts` (+ test: Kategorien `{0: "Physisch", 1: "Speziell", 2: "Status"}`, Zielarten aus §1.5, benannte Flag-Bits 0x01 Kontakt … nach §1.5, unbenannte Bits nur in Tüfteln), `useMove.ts`, Tests
- Modify: `DataArea.tsx`

**Interfaces:**
- Consumes: `DataDocDto::Move`, `DataPatch::Move`, `refs Move`, `TypeBadge`, `DSScreen variant="textbox"` (Beschreibung aus der Bank `move_descriptions`, Nummer aus Task 0 Step 3; fehlt sie, zeigt die Schau „Beschreibung nicht gefunden“).
- Produces: Beschriftungen nach design-report §5: Liste „Attacken“ mit Zähler, Suche, Typ-Chips, Kategorie-`SelectField`; Schau: Karte (Typ, Kategorie, Stärke, Genauigkeit, AP, Merkmale), Beschreibung, „Verwendet in“ (N Lernsets, N Trainer); Hand: Spalten Grundwerte (Stärke, Genauigkeit, AP, Priorität, Effektchance) · Verhalten (Ziel `SelectField`, benannte Flags als `Checkbox`) · Rohdaten (Tüfteln: Effekt-ID, alle Flag-Bits, `HexBlock`). `data-move={index}`.

- [ ] **Step 1: Failing tests** (Muster Task 14): Stärke ändern → `data_edit` Label „Flügelschlag: Stärke 60 → 65“; unbekanntes Flag-Bit 0x40 bleibt beim Setzen von Kontakt erhalten (Patch setzt nur das Bit); Suche; Kategorie-Filter.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): moves editor`

---

### Task 16: Gegenstände-Editor

**Files:**
- Create: `apps/studio/src/data/items/ItemsList.tsx`, `ItemView.tsx`, `ItemHand.tsx`, `itemModel.ts` (+ test: Taschen nach §1.6, Feldgruppen Werte/Effekte/Funktionen, Party-Parameter nur Tüfteln), `useItem.ts`, `useItemIcon.ts` (Cache je Item-ID über `get_item_icon`), Tests
- Modify: `DataArea.tsx`

**Interfaces:**
- Consumes: `DataDocDto::Item`, `DataPatch::Item`, `item_table`, `refs Item`, `Sprite`, `DSScreen textbox` (Beschreibung: Bank `item_descriptions`, Nummer aus Task 0 Step 3).
- Produces: nach design-report §7: Liste „Gegenstände“ nach Item-ID mit Symbol, Name, Preis rechts, Taschen-Chips mit Zähler; ohne Item-Tabelle Zeilen „Daten 0017“ ohne Symbol; Schau: Karte, Beschreibung, „Verwendet in“ (Trainer-Items, Party-Items, Wild-Items), Hinweis „teilt Daten mit 21 Gegenständen“; Hand: Werte (Preis `NumberField bits=16`, Taschen, Effekte, Funktionen als `SelectField`/`Switch`) · Rohdaten. `data-item={id}`.

- [ ] **Step 1: Failing tests**: Preis ändern → Label „Trank: Preis 300 → 250“; Liste ohne Tabelle zeigt „Daten 0017“; Symbol-Cache ruft `get_item_icon` je ID einmal; geteilte Daten Hinweis.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): items editor with icons from the ROM`

---

### Task 17: Szenario `pt-daten.yaml`, Prüfschritt mit uxie, Docs, Gate-Dokument Teil A

**Files:**
- Create: `scripts/agent/scenarios/pt-daten.yaml`, `docs/phase-3b3-gate.md` (Teil A mit Ergebnistabelle)
- Modify: `crates/g4-kernel/src/bin/g4k.rs`: neuer Unterbefehl `g4k check-data <rom.nds> --expect species:397:atk=85 --expect move:17:power=65 --expect item:17:price=250`, der die NARCs aus dem gebauten ROM (ds-rom-Lesepfad des Kernels, `g4_formats::narc`) **mit uxie** liest (`uxie` wird reguläre Abhängigkeit von g4-kernel, ist es schon); `crates/g4-kernel/tests/cli.rs`
- Modify: `scripts/g4emu/g4emu/scenario.py` (optionaler Abschnitt `check: [["g4k", "check-data", …]]` nach `build`, Exit ≠ 0 lässt das Szenario fehlschlagen), `docs/agent-gate.md` (Rezepte Pokédex, Attacken, Gegenstände; `call.sh data_get '{"kind":"species","index":397}'`), `docs/dev-setup.md` (gated `vanilla_data`, uxie-Orakel, `g4k check-data`), `docs/ROADMAP.md` (3b.3 mit Schnitt a/b und Status „3b.3a implementiert, Gate offen“; Abschnitt „Aus dem Design-System, später“ um die „Später“-Tabelle der Spec ergänzt; Hinweis zum eigenen Encounter-Writer als entschieden), `design-system/README.md` („Was die App umsetzt“ + 3b.3a)

- [ ] **Step 1: Failing test** `tests/cli.rs`: `check_data_reads_the_built_narcs_with_uxie_and_fails_on_a_wrong_expectation` (synthetische Fixture bauen, `--expect species:1:atk=<falsch>` → Exit 1 mit Meldung „species 1 atk: expected …, found …“).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** `g4k check-data` und den `check`-Abschnitt in `scenario.py` (+ `test_scenario.py`-Fall).
- [ ] **Step 4: Szenario** `pt-daten.yaml`: `edits`: `data_edit species 397 {Personal: {field: "atk", value: 85}}`, `data_edit species 397 {Learnset: …}` (eine Zeile mehr), `data_edit move 17 {Move: {field: "power", value: 65}}`, `data_edit item 17 {Item: {field: "price", value: 250}}`; `build: out.nds`; `check`: die vier Erwartungen; kein `play`. Laufen lassen auf einer frischen Kopie `pt-3b3a` unter `_testkopien` (Kopie anlegen, nicht löschen).
- [ ] **Step 5: Docs** nach Dateiliste; `docs/phase-3b3-gate.md` Teil A mit Ablauf (Spec §1 Gate 3b.3a) und leerer Ergebnistabelle.
- [ ] **Step 6: Volle Prüfung**: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`, `cargo test --workspace --all-features`, dann `scripts/gate.sh` im Hintergrund (jetzt mit `vanilla_data`, ca. 30 min); Tabelle in den Bericht.
- [ ] **Step 7: Commit** `feat(gate): data scenario with a uxie check step; docs and roadmap for 3b.3a`

---

## Abschluss

Nach Task 17: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`), danach die Abnahme über die Test-Umgebung (Gate 3b.3a Teil A: Screens wiedererkennbar, ⌘Z über zwei Assets, Hexblock, Symbole) in einer eigenen Session. Der Plan 3b.3b (Trainer-Paar-Asset, `create_trainer`, Begegnungen-Editor, Event ↔ Trainer, `PokemonCard`, `g4emu`-Kampfschritte mit RAM-Kalibrierung als Task 1, Szenarien, Gate Teil B) wird nach dem Merge aus derselben Spec geschrieben.
