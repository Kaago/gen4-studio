# Phase 3b.3a „Codecs und Pokémon-Daten“ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eigene, byte-treue Codecs für Begegnungen, Trainer, Pokémon-Basisdaten, Lernsets, Entwicklungen, Attacken, Gegenstände und Trainer-Texttabellen in `g4-formats`; die editierbaren Arten im Store mit mehrteiligem Changeset, Namenstabellen, arm9-Tabellen (lesend), Referenzindex und Item-Symbolen; die Editoren Pokédex, Attacken und Gegenstände im Doppeldeck-Rahmen.

**Architecture:** `g4-formats/src/data/` hält die Codecs als Rohbyte-Records mit Feldkarte (fester Länge) bzw. `entries + tail` (variabler Länge); uxie 0.9.7 bleibt Test-Orakel. `g4-kernel` bekommt zehn neue `AssetKind`s mit `Location::Member`, `Store::transaction` für Changesets mit mehreren Teilen, `data/{arm9,names,refs,icons}.rs` und Metadaten-YAML. Generische Tauri-Befehle (`data_list`, `data_get`, `data_edit`, `data_raw`) plus `name_tables`, `item_table`, `machine_table`, `refs`, `get_item_icon`, alle auch in der Brücke. Das Frontend bekommt neue Primitives (`TypeBadge`, `StatBar`, `Table`, `FilterChips`, `ToggleGrid`, `HexBlock`, `Sprite`), den Unterbereich-Umschalter in `PanelHead` und die Bereiche Pokémon und Gegenstände auf der Bereichs-Infrastruktur von 3c.2 (`Shell areas`, `DECK_PANELS`, `NavResult.area`, Palette).

**Tech Stack:** Rust (g4-formats, g4-kernel, Tauri 2, proptest 1 als dev-dependency, uxie 0.9.7 schon reguläre Abhängigkeit von g4-kernel, `png` 0.18.1 schon gelockt), React 19 + `ui/` aus 3c (Base UI 1.9 nur in `ui/primitives`, CSS Modules), vitest 5, Python `g4emu` für das Backend-Szenario.

**Spec:** [docs/superpowers/specs/2026-10-23-phase-3b3-data-editors-design.md](../specs/2026-10-23-phase-3b3-data-editors-design.md). Byte-Layouts: [dspre-report.md](../../research/2026-10-23-3b3/dspre-report.md) §1.1–§1.6; Vanilla-Zahlen und uxie-Typen: [uxie-report.md](../../research/2026-10-23-3b3/uxie-report.md) §1, §4; Beschriftungen: [design-report.md](../../research/2026-10-23-3b3/design-report.md) §1, §2, §5, §7; **geprüfte Werte aus Task 0: [task0-3b3a.md](../../research/2026-10-23-3b3/task0-3b3a.md)** (Paletten-Tabelle, Formnamen, Archivnummern, Stichproben).

Diese Fassung (23.10.2026, nach Task 0 und drei Reviews) ersetzt den Entwurf vom selben Tag. Pfade unten sind relativ zur Repo-Wurzel; Frontend-Pfade relativ zu `apps/studio/src`.

## Global Constraints

- Voraussetzung: 3c.1 und 3c.2 sind auf `main` (Primitives, `Tabs`, `DSScreen`, `Badge`-Arten `success|warning|code|neutral`, `Picker variant="palette"`, `Shell areas`, `DECK_PANELS` mit `PanelSpec[]`, `layoutKey` v8, `NavResult.area`, `resetLayout`, `paletteEntries`, `setTheme`/`setDensity`). Der Plan startet auf einem Branch ab diesem Stand; die Namen aus 3c.2 werden konsumiert, nicht neu gebaut.
- Die Vanilla-Projekte unter `../Roms/*_DSPRE_contents` werden nie beschrieben; schreibende Tests laufen auf `vanilla_copy()`, das GUI nur auf Kopien unter `../Roms/_testkopien`.
- Testbefehle aus der Repo-Wurzel mit `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`. Vor jedem Commit: `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` (auch `private_interfaces`: keine privaten Typen in öffentlichen Signaturen), `cargo test --workspace --all-features`; in `apps/studio`: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`.
- Gated Tests: `G4_TEST_PROJECT="$PWD/../Roms/<Projekt>_DSPRE_contents"`, Projekte `Pokemon - Platin-Edition (Germany)` (CPUD), `Pokemon - Goldene Edition HeartGold (Germany)` (IPKD), `Pokemon - HeartGold Version (USA)` (IPKE); in einem Worktree `G4_ROMS_DIR` absolut setzen. Soll-Member-Zahlen (Pt / HGSS): encounters 183/142, trainerProperties 928/738, trainerParty 928/738, personalPokeData 508/508, learnsets 508/508, evolutions 508/508, moveData 471/471, itemData 446/514, trainerTextTable 1/1, trainerTextOffset 1/1. Ein gated Test, der 0 Member sieht, schlägt fehl.
- Codecs lehnen nie unbekannte Bits oder Werte ab; nur eine falsche Länge ist ein Fehler. Unbekannte Bits bleiben Rohwerte.
- DSPRE-Layout 1:1: Ordner `unpacked/<name>` aus Spec §2; neue Dateien nur über `fsutil::write_atomic`; in Member-Ordnern liegen nur `NNNN`-Dateien.
- arm9 wird gelesen, nie geschrieben. Offsets nur aus `crates/g4-kernel/metadata/arm9_tables.yaml` (CPUD, IPKD, IPKE geprüft; andere Codes `unverified: true`).
- UI-Texte deutsch (Begriffe aus `terms.ts`, Schreibweise „Script“; Tasten sagen, was passiert; keine Abkürzungen wie „Pt“ oder „Sp.-Ang.“; keine Mittelpunkt-Ketten); Code, Kommentare, Commits englisch; Styling nur über `ui/` und `--g4-*`-Tokens, kein `style=` ohne `/* dyn */`, keine Hex-Farben außerhalb `ui/tokens/` (Gate-Schritt „styles“).
- Jeder neue Tauri-Befehl steht in `apps/studio/src-tauri/src/bridge/dispatch.rs` (`COMMANDS`, `run!`-Arm, Fixture in `every_command_accepts_the_keys_api_ts_sends`), in `generate_handler!` und in `api.ts`.
- Keine Nintendo-Grafiken im Repo: Tests für NCGR/NCLR nutzen synthetische Bytes; die App rendert nur aus dem geöffneten Projekt.
- `CmdError.code` bleibt `stale | read_only | tool_missing | partial | error | compile`; „nicht verfügbar“ (arm9) ist kein Fehler, sondern `null` im DTO.
- Diamant/Perl: Metadaten kennen die Pfade (`unverified`), `data_list` liefert für `Family::Dp` den Sperrgrund „Diamant/Perl: ohne Testprojekt nicht freigegeben“ und keine Zeilen.
- Kernel-Texte (Sperrgründe, `AssetId`-Display) bleiben englisch mit den Zahlen; die deutschen Sätze („Gegenstand 0017: 36 Bytes statt 34“) baut die Oberfläche.
- Tests fragen nach Rollen, `aria-label`, Texten und `data-*` (`data-species`, `data-move`, `data-item`, `data-raw`), nie nach CSS-Klassen.

## Review Focus

- Ein Member mit abweichender Länge (Hack-Projekt, 36-B-Item aus DSPRE): das Asset ist gesperrt, `data_get` liefert Rohbytes und Grund, die Liste zeigt die Zeile, die Hand sperrt die Felder, nichts stürzt (Task 7, Task 14).
- Item-Tabelle unplausibel (arm9-Patch, fremder Spielcode): `item_table` liefert `null`, die Liste zeigt „Daten 0017“ statt Namen, das TM-Raster zeigt Nummern ohne Attackennamen (Task 5, Task 14, Task 16).
- Personal-Member 496–507 (Formen): die Liste nennt „Deoxys, Angriffsform“ aus `species_forms.yaml`; ein Index ohne Namen heißt „Art 0520“ (Task 8, Task 14).
- Mehrteiliger Changeset, dessen zweiter Teil fehlschlägt oder beim Undo nicht dekodierbar ist: kein Asset ändert sich, der Fehler nennt das Asset (Task 6).
- Undo über 200 Datenänderungen in Folge (Scrubbing): jede Änderung ist ein Schritt mit sprechendem Label; `history()` mit 200 Schritten bleibt unter 50 ms; `dirty()` bleibt schnell, obwohl der Referenzindex tausende Member gelesen hat (Task 6, Task 9).

---

### Task 0: Prüfungen (erledigt)

Erledigt am 23.10.2026, Commit 4f9be61, Bericht [task0-3b3a.md](../../research/2026-10-23-3b3/task0-3b3a.md). Ergebnisse, die dieser Plan benutzt:
- `mon_icon_palettes`: CPUD `0xF07D8`, IPKD `0xFFBC4`, IPKE `0xFFC10`; Länge 540 / 544 / 544 Bytes, Werte ∈ {0,1,2}; **geordnet nach Symbol-Index (NCGR-Member − 7), nicht nach Personal-Member**. „Art + 7“ gilt für Arten 0–493; Ei, Bad Egg und Formen nach §1.2 des Berichts (Rotom-Formen auf NCGR 542–546).
- Formnamen 496–507 (Pt und HGSS gleich): Deoxys Angriffsform/Verteidigungsform/Initiativeform, Burmadame Sandumhang/Lumpenumhang, Giratina Urform, Shaymin Zenitform, Hitze-/Wasch-/Frost-/Wirbel-/Schneid-Rotom. Formnamen-Archive: Pt 697, HGSS 802 (nicht 712–716 / 817–821, das sind Artnamen in fünf Sprachen).
- Banken: Pt `species 412, moves 647, move_descriptions 646, items 392, item_descriptions 391, abilities 610, types 624, trainer_names 618, trainer_classes 619, battle_texts 617, forms 697`; HGSS `237, 750, 749, 222, 221, 720, 735, 729, 730, 728, 802`. Zahlen: Species 496, Attacken 468, Items 468/537, Fähigkeiten 124, Typen 18, Trainernamen 928/738, Klassen 105/129, Kampftexte 2497/1717.
- Stichproben: Index 17 = Flügelschlag (Stärke 60, Flug, AP 35) und Trank (Preis 300, arm9 Daten 17, Symbol 24, Palette 25).
- HGSS `trtbl` i ↔ Nachricht i in Archiv 728 bestätigt. rotom: Pt `GivePokemon` (150), HGSS `GiveMon` (137). Sichtweite `data[0]` (Pt belegt). Trainer-Flags Pt ab 0x550, 928 Flags.

---

### Task 1: Record-Grundbaustein in `g4-formats`

**Files:**
- Create: `crates/g4-formats/src/data/mod.rs`, `crates/g4-formats/src/data/record.rs`
- Modify: `crates/g4-formats/src/lib.rs` (`pub mod data;`), `crates/g4-formats/Cargo.toml` (`[dev-dependencies] proptest = "1"`; proptest ist nicht im Lock: erster `cargo fetch` braucht Netz, im Bericht nennen)

**Interfaces:**
- Produces: `pub struct FieldSpan { pub name: &'static str, pub offset: usize, pub len: usize, pub kind: FieldKind }`, `pub enum FieldKind { U8, U16, U32, I8, Bits { mask: u32 }, Bytes }`; `pub trait RawRecord: Sized { const LEN: usize; const FIELDS: &'static [FieldSpan]; fn raw(&self) -> &[u8]; fn raw_mut(&mut self) -> &mut [u8]; fn read(bytes: &[u8]) -> Result<Self, FormatError>; fn write(&self) -> Vec<u8>; fn get_u8/get_u16/get_u32/get_i8(&self, off) ; fn set_u8/set_u16/set_u32/set_i8(&mut self, off, v); fn bits(&self, off: usize, mask: u32) -> u32; fn set_bits(&mut self, off: usize, mask: u32, v: u32) }` (`shift = mask.trailing_zeros()`, Little-Endian); Makro `fixed_record!(Name, LEN, [FieldSpan…])` erzeugt `struct Name { raw: [u8; LEN] }` mit `read` (Längenprüfung über das vorhandene `FormatError::BadSize { what: stringify!(Name), expected, found }`) und `write`. (Name `RawRecord`, nicht `Overlay`: `g4_kernel::project::Overlay` sind die arm9-Overlays.)
- Produces für Tests: `pub fn fields_cover_exactly(fields: &[FieldSpan], len: usize) -> Result<(), String>`: Spans ohne `Bits` decken jedes Byte genau einmal; jeder `Bits`-Span liegt in einem Container-Span gleicher Lage, und die Masken je Container sind disjunkt.

- [ ] **Step 1: Failing tests** (`record.rs` Test-Modul): `read_rejects_wrong_length_with_expected_and_found` (`fixed_record!(Probe, 4, …)`, `Probe::read(&[0; 3])` → `Err(BadSize { expected: 4, found: 3, .. })`), `write_returns_the_raw_bytes_unchanged` (proptest), `setter_changes_only_its_own_bytes` (proptest), `bits_round_trip` (`set_bits(0, 0xFC, 21)` → `bits(0, 0xFC) == 21`, Bits 0–1 unverändert), `coverage_accepts_bits_inside_their_container_and_rejects_a_gap`.
- [ ] **Step 2: Run, FAIL** (`cargo test -p g4-formats data::record`) → **Step 3: Implement** → **Step 4: Run, PASS**; `cargo clippy`.
- [ ] **Step 5: Commit** `feat(formats): raw-byte record with field map for the data codecs`

---

### Task 2: Codecs fester Länge: Personal, Entwicklung, Attacke, Gegenstand

**Files:**
- Create: `crates/g4-formats/src/data/personal.rs`, `evolution.rs`, `moves.rs`, `item.rs`; `docs/formats/personal.md`, `evolution.md`, `move.md`, `item.md`
- Modify: `crates/g4-formats/src/data/mod.rs`

**Interfaces:**
- Produces `Personal` (44 B, dspre §1.4): `hp, attack, defense, speed, sp_attack, sp_defense: u8` (0x00–0x05), `type1, type2, catch_rate, base_exp: u8`, `ev_yield(i: usize) -> u8` für i ∈ 0..6 in der Reihenfolge KP, Angriff, Verteidigung, Initiative, Spezial-Angriff, Spezial-Verteidigung (Bits 2i..2i+1 des u16 @0x0A), `set_ev_yield(i, v)`, `item1, item2: u16`, `gender_ratio, egg_cycles, base_friendship, growth_rate, egg_group1, egg_group2, ability1, ability2, flee_rate: u8`, `color() -> u8` (Bits 0–6 @0x19), `flip() -> bool` (Bit 7), `tm(n: usize) -> bool`/`set_tm(n, bool)` für n ∈ 0..100 (TM 1..92, HM 1..8; Bit-Reihenfolge nach §1.4, Test fixiert sie), `tm_bits() -> [u8; 16]`. `Evolution` (44 B): `entry(i) -> EvoEntry { method: u16, param: u16, target: u16 }` für i ∈ 0..7, `set_entry`, 2 Pad-Bytes erhalten. `Move` (16 B, §1.5): `effect: u16, category, power, move_type, accuracy, pp, effect_chance: u8, target: u16, priority: i8, flags: u8, contest, contest_type: u8`. `Item` (34 B, §1.6): `price: u16, hold_effect, hold_param, pluck_effect, fling_effect, fling_power, natural_gift_power: u8`, u16 @0x08 mit `natural_gift_type` (Bits 0–4), `is_selectable` (5), `is_rare`? (Bits nach §1.6 benannt, Rest roh), `field_pocket` (Bits 7–10), `battle_pocket` (Bits 11–15); 0x0A–0x0D `field_use_func, battle_use_func, party_use, pad_0d`, danach die Party-Parameter nach §1.6; **kein** Feld `pocket`. Jede Struktur: `const FIELDS` vollständig nach Task 1 (Pad-Spans `pad_<hex>`).

- [ ] **Step 1: Failing tests** je Datei: `fields_cover_every_byte_exactly_once` (`fields_cover_exactly(FIELDS, LEN)`), `read_write_round_trip_on_random_bytes` (proptest), ein Feldtest mit eingebetteten Vanilla-Bytes (read-only aus Platin DE kopiert: Personal 397 Staravia `hp 55, attack 75, defense 50, speed 80, sp_attack 40, sp_defense 40, type1 0, type2 2`; Move 17 Flügelschlag `power 60, pp 35, move_type 2`; Item 17 Trank `price 300`; übrige Felder aus denselben Bytes ablesen), `tm_bits_round_trip` (`set_tm(39, true)` setzt genau ein Bit), `ev_yield_reads_two_bit_fields`, `item_pockets_are_bit_fields_of_the_word_at_8`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** mit `fixed_record!` → **Step 4: Run, PASS**.
- [ ] **Step 5: Docs** `docs/formats/*.md` nach dem Muster von `docs/formats/narc.md` (Offset, Länge, Feld, Quelle).
- [ ] **Step 6: Commit** `feat(formats): personal, evolution, move and item codecs as byte records`

---

### Task 3: Codecs variabler Länge: Lernset, Trainer-Eigenschaften, Trainer-Party, Trainer-Texttabellen

**Files:**
- Create: `crates/g4-formats/src/data/learnset.rs`, `trainer.rs`, `trainer_text.rs`; `docs/formats/learnset.md`, `trainer.md`, `trainer_text.md`

**Interfaces:**
- Produces `Learnset { entries: Vec<LearnEntry { move_id: u16 (9 Bit), level: u8 (7 Bit) }>, tail: Vec<u8>, terminated: bool, read_count: usize }` mit `read(bytes)` (bis 0xFFFF oder Ende; ohne 0xFFFF `terminated = false`, `tail` leer), `write()` (Einträge, 0xFFFF, dann `tail`, wenn `entries.len() == read_count`, sonst 0x0000-Füllung auf Vielfaches von 4), `MAX_ENTRIES = 20` (alle 3 × 508 Vanilla-Lernsets haben ≤ 20 Einträge und 0x0000-Tail, geprüft). `TrainerProps` (20 B Record, **Reihenfolge geprüft**: `flags: u8` @0 mit `has_moves() = Bit 0`, `has_items() = Bit 1`, `class: u8` @1, `unknown: u8` @2, `party_count: u8` @3, `items: [u16; 4]` @4..12, `ai: u32` @12, `battle_type: u32` @16). `PartyFormat { moves: bool, items: bool }` mit `entry_len(family) -> usize` (Pt/HGSS 8/10/16/18, DP 6/8/14/16); `PartyMon { iv_or_difficulty: u8, byte1: u8, level: u16, species_word: u16 (species() 10 Bit, form() 6 Bit), item: Option<u16>, moves: Option<[u16; 4]>, ball_capsule: Option<u16> (nicht DP) }`; `Party { mons: Vec<PartyMon>, tail: Vec<u8> }` mit `read(bytes, format, count, family)` und `write(format, family)` (Nullen auf Vielfaches von 4, mindestens 8 B; Vanilla: Länge count × len, aufgefüllt, geprüft); `TrainerTextTable { entries: Vec<(u16, u16)> }`, `TrainerTextOffsets { offsets: Vec<u16> }`.

- [ ] **Step 1: Failing tests**: `learnset_reads_vanilla_bytes_and_writes_them_back`, `learnset_with_changed_count_pads_to_multiple_of_four`, `learnset_without_terminator_is_read_and_written_terminated`, `party_round_trips_every_format` (proptest über `format`, `count` 0..6, `family`; der Generator erzeugt `tail` kanonisch, sonst gilt `read(write(d)) != d`), `empty_party_is_eight_zero_bytes`, `trainer_props_fields_cover_twenty_bytes_in_the_checked_order` (Byte 3 ist `party_count`), `trainer_text_tables_round_trip`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Docs** → **Step 6: Commit** `feat(formats): learnset, trainer properties, party and trainer text codecs`

---

### Task 4: Codec Begegnungen (Pt und HGSS)

**Files:**
- Create: `crates/g4-formats/src/data/encounter.rs`, `docs/formats/encounter.md`

**Interfaces:**
- Produces `PtEncounters` (424 B Record, §1.1): `grass_rate: u32`, `grass(i) -> Slot { level: u8, species: u16 }` i ∈ 0..12, Ersatzlisten `day(i)`, `night(i)` i ∈ 0..2, `radar(i)` i ∈ 0..4, `form_rates: [u32; 5]` @0x8C, `unown_table: u32` @0xA0, `dual(slot: DualSlot, i)` (Rubin, Saphir, Smaragd, Feuerrot, Blattgrün je 2), `swarm(i)` i ∈ 0..2, Wasser `surf(i)`, `old_rod(i)`, `good_rod(i)`, `super_rod(i)` i ∈ 0..5 als `WaterSlot` (Pt: Byte 0 max, Byte 1 min; `min()`/`max()` verbergen das), Raten je Wasser-Methode `u32`, Block 0xF8–0x123 als `pad_f8`; Setter je Feld. `HgssEncounters` (196 B, §1.2): `grass_rate, surf_rate, rock_rate, old_rod_rate, good_rod_rate, super_rod_rate: u8`, `grass_level(i)`, `grass_morning(i)`, `grass_day(i)`, `grass_night(i)`, `radio(i)` i ∈ 0..4, Wasser `WaterSlot` min vor max, `rock(i)` i ∈ 0..2, `land_swarm(i)`, `surf_swarm`, `night_fish`, `fish_swarm`. `enum Encounters { Pt(PtEncounters), Hgss(HgssEncounters) }` mit `read(bytes, family)` (DP wie Pt) und `write()`; `fn methods(&self) -> Vec<MethodView { method: Method, rate: u32, slots: Vec<SlotView { index, species, min, max }> }>`.

- [ ] **Step 1: Failing tests**: `pt_fields_cover_424_bytes_once`, `hgss_fields_cover_196_bytes_once`, `pt_water_slot_min_max_order_is_hidden` (Bytes `[0x1E, 0x14, …]` → `min() == 20`, `max() == 30`; `set_min(25)` schreibt Byte 1), `hgss_water_slot_min_before_max`, `round_trip_random_bytes_both_families` (proptest), `methods_view_lists_grass_surf_and_rods_with_slot_counts` (Pt 12/5/5/5/5, HGSS 12/5/2/5/5/5).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Docs** → **Step 6: Commit** `feat(formats): encounter codecs for Platinum and HGSS with a shared method view`

---

### Task 5: Metadaten, Archivarten, arm9-Tabellen, gated Vanilla-Test mit uxie-Orakel

**Files:**
- Create: `crates/g4-formats/src/data/arm9_tables.rs`, `crates/g4-kernel/src/data/mod.rs`, `crates/g4-kernel/src/data/arm9.rs`, `crates/g4-kernel/metadata/text_banks.yaml`, `crates/g4-kernel/metadata/arm9_tables.yaml`, `crates/g4-kernel/metadata/species_forms.yaml`, `crates/g4-formats/tests/vanilla_data.rs`, `crates/g4-kernel/tests/arm9_tables.rs`
- Modify: `crates/g4-kernel/metadata/archives.yaml` (zehn Arten aus Spec §2 je Familie; `by_game` nach Präfix (3 Zeichen, Vorrang vor `packed_jp`): `IPG` Begegnungen `files/a/1/3/6`, `APA` Begegnungen `p_enc_data.narc`, `APA` Personal `poketool/personal_pearl/personal.narc`), `crates/g4-kernel/src/project.rs` (`ArchiveEntry.by_game: Option<BTreeMap<String, String>>`, `ArchiveMap::for_game(family, language, game_code)`; `AssetKind` + `ALL` + `label` + `slot` um `Encounter, Personal, Learnset, Evolution, Move, Item, TrainerProperties, TrainerParty, TrainerTextTable, TrainerTextOffsets` in den Slots 12–21 hinter `Script`; `ArchiveMap.entries` und `ALL` von 12 auf 22; `FamilyArchives` neue Felder), `crates/g4-kernel/src/store.rs` (erschöpfendes `match` in `Loaded::from_bytes`: `NotEditable`-Arme für die neuen Arten, editierbar erst in Task 7), `crates/g4-kernel/src/lib.rs` (`pub mod data;`), `crates/g4-kernel/tests/project.rs` (`asset_kinds_have_twelve_slots` → 22; `embedded_metadata_parses_and_resolves_every_family` erwartet je Familie +10)

**Interfaces:**
- Produces (g4-formats): `ItemTable { entries: Vec<ItemEntry { data: u16, icon: u16, palette: u16, gba: u16 }> }::read(arm9: &[u8], offset, count) -> Option<ItemTable>` (None, wenn arm9 zu kurz), `MachineTable { moves: Vec<u16> /* 100 */ }::read(arm9, offset) -> Option<…>`, `MonIconPalettes { values: Vec<u8> }::read(arm9, offset, len) -> Option<…>`, `fn plausible(item: &ItemTable, machine: &MachineTable, item_members: usize) -> Result<(), String>` (TM01 = 264, HM05 ∈ {432, 250}, jeder `data` < `item_members`).
- Produces (g4-kernel `data/arm9.rs`): `Project::item_table(&self) -> Option<&ItemTable>`, `Project::machine_table(&self) -> Option<&MachineTable>`, `Project::mon_icon_palettes(&self) -> Option<&MonIconPalettes>` (einmal gelesen und gecacht; `None`, wenn Offset fehlt, `unverified`, arm9 zu kurz oder `plausible` fehlschlägt).
- YAML: `arm9_tables.yaml` vollständig je Code: `CPUD: { item_table: 0xF0D1C, item_count: 468, machine_table: 0xF0C54, mon_icon_palettes: 0xF07D8, mon_icon_palettes_len: 540, trainer_name_max: 7, trainer_flag_start: 0x550, trainer_flag_capacity: 928 }`, `IPKD: { item_table: 0x100148, item_count: 537, machine_table: 0x100080, mon_icon_palettes: 0xFFBC4, mon_icon_palettes_len: 544, trainer_name_max: 7, trainer_flag_start: null, trainer_flag_capacity: 734 }`, `IPKE: { item_table: 0x100194, item_count: 537, machine_table: 0x1000CC, mon_icon_palettes: 0xFFC10, mon_icon_palettes_len: 544, … }`, andere Codes aus DSPRE mit `unverified: true`. `text_banks.yaml`: `pt: { species: 412, moves: 647, move_descriptions: 646, items: 392, item_descriptions: 391, abilities: 610, types: 624, trainer_names: 618, trainer_classes: 619, battle_texts: 617, forms: 697 }`, `hgss: { 237, 750, 749, 222, 221, 720, 735, 729, 730, 728, 802 }`, `dp` aus uxie `TextBankIds` mit `unverified` (kein `battle_texts`, kein `forms`). `species_forms.yaml`: Member 496–507 → `{ base: u16, name: "Deoxys, Angriffsform" }` (Task 0), je Familie gleich.
- uxie-Orakel (Zuordnung Plan ↔ uxie): `uxie::PersonalData::from_binary(&mut Cursor)` mit `attack, defense, speed, sp_attack, sp_defense, safari_flee_rate, color_flip, ev_yield: u16, tm_compatibility: [u8;16]`; `uxie::ItemData::from_binary` mit `hold_effect_param, field_pocket` (Enum), `battle_pocket` (Bitflags), `is_selectable, field_use_func, battle_use_func, party_use, party_use_param`; `uxie::TrainerProperties::from_binary` mit `flags` (Bitflags), `trainer_class, unknown, party_count, items, ai_flags` (Bitflags), `double_battle`. Der Test vergleicht Feld für Feld über diese Tabelle (Enums und Bitflags als Zahl).

- [ ] **Step 1: Failing tests**: `embedded_metadata_parses_and_resolves_every_family` (+10 je Familie, `("IPG…", Encounter) == "files/a/1/3/6"`, `("APA…", Personal) == "…/personal_pearl/personal.narc"`), `arm9_tables_yaml_has_every_checked_code`, `plausible_rejects_a_data_index_beyond_the_members`, `item_table_is_none_when_arm9_is_short`; gated `vanilla_data.rs`: je Art alle Member aus `unpacked/<name>/` lesen, `write(read(x)) == x`, Soll-Zahl prüfen; Orakel für Personal, Item, TrainerProps; `vanilla_pt_water_slots_have_min_le_max` (**1080** belegte Slots, 1078 mit min < max); gated `arm9_tables.rs` (g4-kernel): `vanilla_arm9_tables_are_plausible` (TM01 = 264; Item 17 → Daten 17; Paletten-Tabelle Werte ∈ {0,1,2} und Länge aus YAML).
- [ ] **Step 2: Run, FAIL** (ohne `G4_TEST_PROJECT` überspringen die gated Tests mit „skipped“) → **Step 3: Implement** → **Step 4: Run, PASS** auch gated gegen alle drei Projekte: `G4_TEST_PROJECT=<Projekt> cargo test -p g4-formats --features test-support --test vanilla_data` und `cargo test -p g4-kernel --test arm9_tables`.
- [ ] **Step 5: Commit** `feat(formats,kernel): data archive kinds in the metadata, arm9 tables, gated vanilla round trip with uxie as oracle`

---

### Task 6: Mehrteiliger Changeset im Store

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (`Changeset { parts: Vec<Part>, label }`, `undo`, `redo`, `history`, `apply_edit` als Ein-Teil-Transaktion, `reload_from_disk`/`accept_disk_as_base` entfernen nur den betroffenen Part und löschen leere Changesets), `crates/g4-kernel/tests/store.rs`, `apps/studio/src-tauri/src/commands/project.rs` (`undo_impl`/`redo_impl` liefern `Vec<AssetId>`) und die Tests, die deren Rückgabe prüfen (`commands/{area,matrix,headers,events,buildings,models,world,wizards,project,text}.rs`, `bridge/dispatch.rs`), `apps/studio/src/api.ts` (`undo(): Promise<AssetId[]>`), `apps/studio/src/app/useHistoryReload.ts` (`step` plant je Asset)
- Create: `crates/g4-kernel/tests/transaction.rs`

**Interfaces:**
- Produces: `pub struct Part { pub asset: AssetId, before: Vec<u8>, after: Vec<u8> }`; `pub fn transaction<T>(&mut self, label: &str, f: impl FnOnce(&mut Tx<'_>) -> Result<T, KernelError>) -> Result<(T, Option<ChangeId>), KernelError>`; `Tx` mit **öffentlichen typisierten** Methoden `edit_matrix(id, FnOnce(&mut Matrix))`, `edit_header(…)`, `edit_text_archive(…)` (Task 7 ergänzt `edit_personal`, `edit_learnset`, `edit_evolution`, `edit_move`, `edit_item`, `edit_encounter` und lesend `personal(i)`, `learnset(i)`); das generische `edit(&mut self, id, FnOnce(&mut Loaded))` bleibt `pub(crate)`, weil `Loaded` privat ist. Ein Asset höchstens einmal je Transaktion (sonst `KernelError::InvalidProject`); schlägt `f` fehl, bleibt jedes Asset wie vorher; ohne Änderung kein Changeset. Undo/Redo dekodieren **erst alle** Parts, dann wenden sie in umgekehrter Reihenfolge an; `undo()`/`redo()` liefern `Vec<AssetId>` (bisheriges `Option<AssetId>`-Verhalten: erstes Element).

- [ ] **Step 1: Failing tests** (`transaction.rs`): `transaction_records_one_history_step_for_two_assets`, `undo_restores_every_part_byte_for_byte`, `redo_reapplies_every_part`, `failing_closure_changes_nothing` (Fixture: Event-Member 1 durch 20 Bytes ersetzen, dann ist es gesperrt; zweiter Edit darauf → Fehler nennt das Asset; beide unverändert; kein History-Eintrag), `undo_with_undecodable_part_changes_nothing` (Part 2 nach außen beschädigt → Fehler, Part 1 unverändert), `same_asset_twice_is_refused`, `reload_of_one_part_keeps_the_other_in_history`, `history_with_two_hundred_steps_answers_under_50_ms`; proptest `undo_redo_are_inverse_over_mixed_kinds`.
- [ ] **Step 2: Run, FAIL**; die bestehenden `tests/store.rs` bleiben grün (dort `Option` → erstes Element der `Vec`, nur die Assertions anpassen).
- [ ] **Step 3: Implement** → **Step 4: Run, PASS** (`cargo test --workspace`, `pnpm test`).
- [ ] **Step 5: Commit** `feat(kernel): changesets with several parts and a store transaction; undo and redo return every restored asset`

---

### Task 7: Editierbare Datenarten im Store

**Files:**
- Modify: `crates/g4-kernel/src/store.rs` (`Loaded::{Encounter(Encounters), Personal(Personal), Learnset(Learnset), Evolution(Evolution), Move(Move), Item(Item), TrainerTextTable(..), TrainerTextOffsets(..), Raw(Vec<u8>)}`, `is_editable`, `from_bytes`, `to_bytes`, typisierte Getter und `edit_*` über `apply_edit`, `Tx::edit_*`/`Tx::personal`), `crates/g4-kernel/src/test_support.rs` (Fixture CPUE: die neuen NARCs mit Pt-Pfaden, mindestens 508 Personal-Member, ein Item mit 36 Bytes, Textarchive bis Index 697 mit den Banken 391/392/412/610/617/618/619/624/646/647/697 als kleine Archive), `crates/g4-kernel/src/build.rs` (packt die zehn neuen Ordner, wenn vorhanden), `crates/g4-kernel/tests/store.rs`, `crates/g4-kernel/tests/build.rs`

**Interfaces:**
- Produces: `Store::personal(&mut self, index: u16) -> Result<&Personal, KernelError>` usw.; `edit_personal(&mut self, index, label, edit: impl FnOnce(&mut Personal)) -> Result<Option<ChangeId>, KernelError>` (Muster `edit_text_archive`); `member_count(kind)` für die neuen Arten. **Falsche Länge sperrt statt zu scheitern:** `from_bytes` liefert bei `BadSize` `Loaded::Raw(bytes)` und `locked = Some("item 0017: 36 bytes, expected 34")`; typisierte Getter liefern `KernelError::ReadOnly` mit dem Grund; `Store::current_bytes(id) -> Result<Option<Vec<u8>>>` (öffentlich, auch für Task 9 und `data_get`).

- [ ] **Step 1: Failing tests**: je Art in `tests/store.rs`: `loads_byte_identical`, `edit_records_undo_and_dirty`, `save_writes_member_atomically`, `wrong_length_member_is_locked_with_reason_and_raw_bytes` (prüft die Zahlen 36 und 34 und `current_bytes`); gated `tests/build.rs`: `vanilla_rebuild_matches_dsrom_cli` byte-identisch, obwohl die zehn Ordner gepackt werden, **gegen alle drei Projekte**.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (gated gegen Pt DE, HG DE, HG US).
- [ ] **Step 5: Commit** `feat(kernel): encounter, personal, learnset, evolution, move, item and trainer text assets in the store`

---

### Task 8: Namen (`data/names.rs`)

**Files:**
- Create: `crates/g4-kernel/src/data/names.rs`, `crates/g4-kernel/tests/names.rs`
- Modify: `crates/g4-kernel/src/data/mod.rs`, `crates/g4-kernel/src/store.rs` (`text_generation(index) -> u64`, steigt bei Edit, Undo, Redo, `reload_from_disk`, `accept_disk_as_base`, `create_text_archive` und für die `touched_archives` des Builds)

**Interfaces:**
- Produces: `pub enum Bank { Species, Moves, MoveDescriptions, Items, ItemDescriptions, Abilities, Types, TrainerNames, TrainerClasses, BattleTexts, Forms }`, `pub fn bank_index(family, language, bank) -> Option<u16>` (aus `text_banks.yaml`), `pub struct NameTables` mit `names(&mut self, store: &mut Store, bank) -> Result<&[String], KernelError>` (Cache je Bank über `text_generation`), `species_name(&mut self, store, index: u16) -> String` (Formen aus `species_forms.yaml`, Formtabelle injizierbar `NameTables::with_forms(map)`; unbekannt „Art 0520“ – englisch im Kernel: `"species 0520"`, die UI übersetzt), `move_name`, `item_name` (Item-ID), `class_name`, `trainer_name` (ohne `{TRAINER_NAME:…}`), `description(&mut self, store, bank: Bank, index) -> Option<String>`; `pub mod trainer_name { pub fn unwrap(s) -> String; pub fn wrap(s) -> String }`.

- [ ] **Step 1: Failing tests**: `bank_index_knows_pt_and_hgss` (412/237, 646/749, 391/221, 697/802), `unknown_species_is_named_by_number` (Index 520), `form_member_gets_its_form_name` (503 → „Hitze-Rotom“ aus der Tabelle), `cache_invalidates_after_a_text_edit_and_after_reload`, `trainer_name_wrap_unwrap_round_trip` (proptest); gated `vanilla_name_counts_match_task0` (Species 496, Moves 468, Items 468/537, Trainernamen 928/738, Klassen 105/129, Kampftexte 2497/1717, Beschreibungen 468 / 468 bzw. 537) und `every_vanilla_trainer_name_round_trips`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(kernel): name tables from the text archives with form names and trainer name wrapping`

---

### Task 9: Referenzindex (`data/refs.rs`)

**Files:**
- Create: `crates/g4-kernel/src/data/refs.rs`, `crates/g4-kernel/tests/refs.rs`
- Modify: `crates/g4-kernel/src/store.rs` (Änderungslog: `Store::changes_since(gen: u64) -> (u64, Vec<AssetId>)`, gespeist von Edit, Undo, Redo, Reload, Accept, Create), `crates/g4-kernel/src/data/mod.rs`

**Interfaces:**
- Produces: `pub enum Target { Species(u16), Move(u16), Item(u16), Trainer(u16), TrainerClass(u8), Encounter(u16) }`, `pub struct Ref { pub source: AssetId, pub field: &'static str, pub slot: Option<u16>, pub target: Target }`, `pub struct RefIndex` mit `refs(&mut self, store: &mut Store, target) -> Result<Vec<Ref>, KernelError>` (lazy: baut beim ersten Aufruf; geladene Assets aus dem Store, **nicht geladene direkt von der Platte über `Project::member_path`, ohne sie in den Store zu laden**), `count(…) -> usize`, `refresh(&mut self, store)` (fragt `changes_since`, erneuert nur die Beiträge der geänderten Quellen), `rebuild_full(store)`. Quellen: HeaderTable `wild_pokemon` → Encounter (uxie-Header je Variante per `match`; Pt u16, 0xFFFF keine; HGSS u8, 0xFF keine), Encounter → Species, TrainerProperties/TrainerParty (Codecs, read-only) → TrainerClass, Item, Species, Move, Objekt-Event Script 3000–4999 → Trainer, Personal → Item (`item1`, `item2`), Learnset → Move, Evolution → Species (Ziel), Item (Methoden **6, 7, 16, 17, 18, 19**), Move (Methode **20**), Species-Parameter (Methode **21**); Konstanten mit Quelle dspre §1.4 im Code.

- [ ] **Step 1: Failing tests** (Fixture mit je zwei Membern je Art): `encounter_slots_reference_species`, `learnset_references_moves`, `evolution_item_methods_reference_items_not_species` (6, 7, 16–19), `header_references_its_encounter_file_and_none_for_ffff`, `event_script_3000_references_trainer_1`, `refresh_after_edit_updates_only_that_source`, `index_does_not_load_assets_into_the_store` (`store.loaded_count()` unverändert), proptest `incremental_equals_full_rebuild` über Edit/Undo/Redo/Reload/Accept; gated `vanilla_index_builds_under_a_second` (Ziel 300 ms, Fehler über 1000 ms), `vanilla_dirty_stays_fast_after_index_build` (`dirty()` unter 5 ms), `vanilla_route_201_encounters_reference_known_species`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(kernel): reference index over data, headers and events with incremental refresh`

---

### Task 10: NCGR/NCLR-Dekoder, Item- und Pokémon-Symbole

**Files:**
- Create: `crates/g4-formats/src/ncgr.rs`, `crates/g4-formats/src/nclr.rs`, `crates/g4-kernel/src/data/icons.rs`, `crates/g4-formats/tests/ncgr.rs`, `crates/g4-kernel/tests/icons.rs`, `docs/formats/ncgr_nclr.md`
- Modify: `crates/g4-formats/src/lib.rs`, `crates/g4-kernel/Cargo.toml` (`png = "0.18"`), `crates/g4-kernel/metadata/archives.yaml` (`item_icons`: Pt `files/itemtool/itemdata/item_icon.narc`, HGSS `files/a/0/1/8`, `unpacked: itemIcons`; `mon_icons`: Pt `files/poketool/icongra/pl_poke_icon.narc` (547 Member), HGSS `files/a/0/2/0` (551), `unpacked: monIcons`), `project.rs` (`AssetKind::ItemIcons`, `MonIcons` in den Slots 22–23, nicht editierbar, nur Zählung und Pfad; `ALL` auf 24, Tests anpassen)

**Interfaces:**
- Produces: `Nclr::read(bytes) -> Result<Nclr { palettes: Vec<[u16; 16]> }, FormatError>` (Magics **`RLCN`** und Abschnitt **`TTLP`**, RGB555), `Ncgr::read(bytes) -> Result<Ncgr { width, height, bpp: 4|8, tiles: Vec<u8> }>` (**`RGCN`**/**`RAHC`**, tiled 8 × 8), `fn render_rgba(ncgr, palette: &[u16; 16]) -> Vec<u8>` (Index 0 transparent), Kernel `fn png(rgba, w, h) -> Vec<u8>`; `icons::item_icon(project, item_id) -> Result<Option<Vec<u8>>, KernelError>` (Symbol- und Palettenmember aus `Project::item_table()`; `None` ohne Tabelle), `icons::mon_icon(project, species: u16) -> Result<Option<Vec<u8>>, KernelError>` (NCGR-Member = species + 7 für species < 494, Palette = `mon_icon_palettes()[species]`-Wert → NCLR-Member 0, Palette Nr. Wert; Formen 496–507 über die Tabelle aus Task 0 §1.2 als Konstante `FORM_ICON_MEMBERS`; sonst `None`).

- [ ] **Step 1: Failing tests**: `nclr_reads_a_synthetic_palette_section` (`0x7FFF` → `(255,255,255,255)`), `ncgr_reads_synthetic_4bpp_tiles_and_renders_in_tile_order` (2 × 2 Kacheln, Pixel (9, 0) in Kachel 1), `index_zero_is_transparent`; gated `vanilla_item_icon_of_potion_decodes_to_a_non_empty_png` (Item 17: PNG-Signatur, > 0 opake Pixel) und `vanilla_mon_icon_of_bulbasaur_uses_palette_one`.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Docs** → **Step 6: Commit** `feat(formats,kernel): NCGR and NCLR decoders, item and Pokémon icons from the ROM`

---

### Task 11: Befehle: Daten, Namen, Tabellen, Referenzen, Symbole (Tauri und Brücke)

**Files:**
- Create: `apps/studio/src-tauri/src/commands/data.rs`, `apps/studio/src/api.data.ts` (DTOs, Befehle, `fieldLabels`: deutsche Feldnamen je `DataKind`; `api.ts` re-exportiert), `apps/studio/src/api.data.test.ts`
- Modify: `apps/studio/src-tauri/src/commands/mod.rs`, `lib.rs` (`generate_handler!`), `bridge/dispatch.rs` (`COMMANDS`, Arme, Fixture; Byte-Antworten wie bei den GLB-Befehlen), `apps/studio/src-tauri/src/state.rs` (Session hält `NameTables` und `RefIndex`; `refresh` nach jedem schreibenden Befehl), `apps/studio/src/assets.ts` (Labels: „Begegnungen“, „Pokémon-Daten“, „Lernset“, „Entwicklung“, „Attacke“, „Gegenstand“, „Trainer-Eigenschaften“, „Trainer-Team“, „Trainertexte“, „Trainertext-Offsets“, „Gegenstand-Symbole“, „Pokémon-Symbole“), `apps/studio/src/reload.ts` (neue Arten → `{ kind: "dataPanel"; asset }`)

**Interfaces:**
- Produces (serde): `DataKind` ∈ `species | learnset | evolution | move | item | encounter | trainer` (`trainer` nur `data_list` lesend; `data_get`/`data_edit` → Fehler „kommt mit 3b.3b“); `DataRowDto { index: u16, name: String, tech: String, locked: Option<String>, extra: serde_json::Value }` (Species: `{ types: [u8, u8], number: u16, stats: [u8; 6] }`; Move: `{ type, category, power, accuracy, pp }`; Item: `{ price, field_pocket, icon: bool, data_index }`; Encounter: `{ users: [u16] }`; Trainer: `{ class, party_count }`); `DataDocDto` getaggt `kind`: `Species { personal: PersonalDto, saved: PersonalDto (Stand der letzten Speicherung), learnset: Vec<LearnEntryDto>, evolution: [EvoEntryDto; 7], tm: Vec<bool> (100), locked: Option<String>, raw_locked: Option<Vec<u8>> }`, `Move { fields: Vec<FieldValueDto { name, value: i64, bits: u8, signed: bool }>, description: Option<DescriptionDto { archive: u16, message: u16, text: String }>, locked }`, `Item { fields, description, shared_with: Vec<u16>, locked }`, `Encounter { methods: Vec<MethodDto>, extras, locked }`; `DataPatch` getaggt: `Personal { field: String, value: u32 }`, `Tm { n: u8, on: bool }`, `Learnset { entries }`, `Evolution { slot, method, param, target }`, `Move { field, value: i64 }`, `Item { field, value }`, `CopyMoves { from: u16 }` (TM-Bits und Lernset in einer Transaktion „<Name>: TM/VM und Lernset wie <Quelle>“), `Encounter { method, slot, field, value }`; `data_list(kind, filter: Option<DataFilter { query: Option<String>, types: Vec<u8>, pocket: Option<u8>, category: Option<u8> }>) -> Vec<DataRowDto>`, `data_get(kind, index) -> DataDocDto`, `data_edit(kind, index, patch, label: Option<String>) -> Vec<AssetId>` (Label-Standard „<Name>: <Feld> <alt> → <neu>“ mit deutschen Feldnamen aus einer Tabelle im Kernel-Befehl, gespiegelt in `fieldLabels`), `data_raw(kind, index) -> RawDto { bytes: Vec<u8>, fields: Vec<FieldSpanDto>, path: String, member: u16 }`, `name_tables() -> NameTablesDto { species, moves, items, abilities, types, classes, forms }`, `item_table() -> Option<Vec<ItemEntryDto>>`, `machine_table() -> Option<Vec<u16>>`, `refs(target: TargetDto) -> Vec<RefDto { source: AssetId, field, slot, label, nav: NavTargetDto }>` (`NavTargetDto` als Teilmenge der Frontend-`NavTarget`-Union aus Task 13), `get_item_icon(item) -> Bytes` und `get_mon_icon(species) -> Bytes` (Tauri `tauri::ipc::Response`, Brücke `bytes()` wie die GLB-Befehle; **leerer Body = kein Symbol**).

- [ ] **Step 1: Failing tests**: Rust `commands/data.rs` über `*_impl` auf der Fixture (`data_list_species_names_forms`, `data_edit_personal_records_a_speaking_label`, `copy_moves_is_one_history_step`, `data_get_locked_member_reports_reason_and_raw_bytes`, `item_table_is_null_when_implausible`, `get_item_icon_returns_empty_without_table`); Dispatch: Fixture-Einträge je Befehl; `commands_match_the_tauri_handler`; TS `api.data.test.ts` (Argumentnamen je Befehl).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** (`cargo test`, `pnpm test`) → **Step 5: Commit** `feat(studio): generic data commands, name tables, arm9 tables, references and icons in Tauri and the bridge`

---

### Task 12: Typ-Tokens und Primitives: TypeBadge, StatBar, Table, FilterChips, ToggleGrid, HexBlock, Sprite; `PanelHead.switcher`

**Files:**
- Modify: `design-system/tokens/Szene/*.tokens.json` (Gruppe `szene/typ/<schluessel>/{grund,schrift}` für die 18 Typen, Werte aus der bisherigen `typen`-Konstante in `export_tokens.py`, RGB555-treu, Kontrast ≥ 4,5 in `check_tokens.py` geprüft), `design-system/scripts/export_tokens.py` (`typen` aus den Tokens statt fest), generierte `ui/tokens/*` per `pnpm tokens`, `design-system/README.md`
- Create in `ui/primitives/`: `pokemonTypes.ts` (+ test: Gen-4-Reihenfolge 0 Normal, 1 Kampf, 2 Flug, 3 Gift, 4 Boden, 5 Gestein, 6 Käfer, 7 Geist, 8 Stahl, 9 ???, 10 Feuer, 11 Wasser, 12 Pflanze, 13 Elektro, 14 Psycho, 15 Eis, 16 Drache, 17 Unlicht; deutscher Name und Token-Schlüssel), `TypeBadge.tsx`, `StatBar.tsx`, `Table.tsx`, `FilterChips.tsx`, `ToggleGrid.tsx`, `HexBlock.tsx`, `Sprite.tsx` (+ je `.module.css`, `.test.tsx`); `ui/gallery/pages/DataPage.tsx` (+ Eintrag in `pages/index.ts`)
- Modify: `PanelHead.tsx` (+ test), `ui/frame/ListPane.tsx` (reicht `switcher` durch), `ui/primitives/Menu.tsx` (`MenuItemSpec.checked?` als `menuitemradio`, `detail?` für Zähler und Kürzel), `ui/primitives/ListRow.tsx` (`leading?: ReactNode`), `ui/primitives/Tabs.tsx` (falls 3c.2 nur die Reiterleiste liefert: `Tabs.Panel` mit `role="tabpanel"` und `aria-labelledby`), `ui/primitives/index.ts`

**Interfaces:**
- `TypeBadge { type: number; size?: "s" | "m" }`: Klasse je Typ aus `TypeBadge.module.css` (`--g4-szene-typ-feuer-grund`…), Name aus `pokemonTypes.ts`; unbekannt „Typ 19“ in `flaeche/deck` + `text/sekundaer`.
- `StatBar { label; value: number; max?: number (255); average: number; deviates?: boolean }`: `role="meter"` mit `aria-valuenow/min/max`, sichtbarer Text „über Schnitt“ / „unter Schnitt“ / „im Schnitt“; Balken `akzent/grund`, Schnitt-Strich `linie/stark`, Spur `flaeche/deck`; Breite per `/* dyn */`.
- `Table { columns: {key, label, width?, align?}[]; rows: {key, cells: Record<string, ReactNode>, attrs?: Record<string, string>, disabled?}[]; ariaLabel; selected?; onSelect?; onDeleteRow?; footer? }`: `role="grid"`, Roving Tabindex über Zeilen, Enter springt ins erste Feld der Zeile, Escape zurück zur Zeile, Entf ruft `onDeleteRow` nur mit Fokus auf der Zeile.
- `FilterChips { options: {value, label, count?, type?: number}[]; value: Set<string>; onChange; visible?: number (6) }`: aktiv `auswahl/flaeche` + `auswahl/kante` + Haken; „n weitere“ als `Popover`; Taste „Filter lösen“.
- `ToggleGrid { cells: {id, label, name?, checked, disabled?}[]; columns; onToggle; ariaLabel }`: Raster aus `Checkbox`, zugänglicher Name „TM40 Aero-Ass“ (nichts nur im Tooltip), Pfeiltasten in zwei Richtungen; der Zähler „27 von 100“ steht außerhalb.
- `HexBlock { bytes: number[]; fields: FieldSpanDto[]; highlight?: string; path: string; member: number }`: `<table>` 16 je Zeile, Offsets mono in `text/sekundaer`, markiertes Feld `auswahl/flaeche`, Fußzeile „Byte 0 bis 1: Preis, 0x012C = 300“; `data-raw` an den Zellen.
- `Sprite { url: string | null; label: string; size: 24 | 32 | 48; area: "pokemon" | "gegenstaende" }`: `<img>` mit `image-rendering: pixelated`, Platzhalter Anfangsbuchstabe in `bereich/<area>` auf `bereich/<area>-flaeche`; der Blob-URL-Cache liegt in `data/icons.ts` (Task 13) nach dem Muster `world/sprites.ts` (Generation, Reset bei Projektwechsel, `revokeObjectURL`).
- `PanelHead.switcher?: { items: {id, label, count?, shortcut?, disabled?, hint?}[]; value; onChange }`: Titel wird ein `Menu`-Knopf „Pokédex ▾“ mit `menuitemradio`; gesperrte Einträge nennen den Grund im Text („Trainer, kommt mit Phase 3b.3b“).

- [ ] **Step 1: Tokens** anlegen, `python3 design-system/scripts/check_tokens.py`, `cd apps/studio && pnpm tokens`, `bash scripts/check-tokens.sh` grün (`.fig`-Import bleibt beim Nutzer; im Bericht nennen).
- [ ] **Step 2: Failing tests** je Komponente über Rollen (`meter`, `grid`/`row`, `checkbox` mit Namen „TM40 Aero-Ass“, `menu`/`menuitemradio`), `TypeBadge` unbekannter Typ, `pokemonTypes` Reihenfolge, `HexBlock` markiert nur die Bytes des Felds und zeigt die Fußzeile, `PanelHead` mit `switcher` rendert Zähler und ruft `onChange`, gesperrter Eintrag ruft nicht.
- [ ] **Step 3: Run, FAIL** → **Step 4: Implement** → **Step 5: Run, PASS**; Galerie-Seite „Daten“; `bash scripts/check-styles.sh` grün.
- [ ] **Step 6: Commit** `feat(ui,tokens): type tokens, type badge, stat bar, table, filter chips, toggle grid, hex block, sprite, panel head switcher`

---

### Task 13: Bereiche Pokémon und Gegenstände im Rahmen

**Files:**
- Create: `data/PokemonArea.tsx`, `data/ItemsArea.tsx`, `data/useDataArea.ts` (gemeinsamer Zustand: `sub`, `selected` je Unterbereich, Suche, Filter), `data/dataStore.ts` (`useSyncExternalStore`-Store für `name_tables` und die `data_list`-Zeilen je Art; lädt beim Projektöffnen lazy beim ersten Bedarf und nach `dataChanged`; auch für die Palette), `data/dataBus.ts` (Bus-Thema `{ topic: "dataChanged"; kind; index }`), `data/icons.ts` (Blob-URL-Cache für `get_item_icon`/`get_mon_icon`, Schlüssel `item:<icon>:<palette>` bzw. `mon:<species>`), `data/PokemonArea.test.tsx`, `data/ItemsArea.test.tsx`
- Modify: `ui/frame/shellState.ts` (**bleibt `g4.shell.v1`**; neue Felder `sub: { pokemon: PokemonSub }` und `selected: { pokedex?: number; moves?: number; items?: number }`, fehlende Felder werden wie heute mit Standard gefüllt), `App.tsx` (`LOCKED_AREAS = ["geschichten"]`, `LOCKED_HINTS.geschichten = "Kommt mit Phase 3b.4"`; `Shell areas={{ …, pokemon: {list, deck}, gegenstaende: {list, deck} }}`; Puffer `dataTarget` wie `textTarget`), `workbench/layout.ts` (`PanelId` + `pokemonView | pokemonEdit | itemView | itemEdit`; `DECK_PANELS.pokemon = { schau: [pokemonView], hand: [pokemonEdit] }`, `.gegenstaende` analog, **ohne** `problems`; Schlüssel bleiben `g4.layout.v8.<area>.<deck>`), `workbench/panels.tsx` (Registrierung), `workbench/navigation.ts` (`NavTarget` + `{kind:"species", index, tab?}`, `{kind:"move", index}`, `{kind:"item", id}`; `NavResult.area` + `"pokemon" | "gegenstaende"`, `NavResult.data?: { sub, index }`), `workbench/breadcrumb.ts` (`dataCrumbs()`: „Pokémon › Pokédex › Staravia“, zweites Glied mit `menu` = Umschalter; Gegenstände: „Gegenstände › Tasche Items › Trank“, Klick auf die Tasche setzt den Filter), `ui/frame/Hinge.tsx` (`Crumb.menu?: MenuItemSpec[]` als `Menu`), `ui/frame/Deck.tsx` (`DockApis`-Map und `resetLayout` für beide Bereiche), `app/paletteEntries.ts` (+ test: Gruppen „Pokémon“, „Attacken“, „Gegenstände“ nach „Textarchive“ mit `tech` vierstellig und `description` (Typen bzw. Preis); Befehle „Bereich Pokémon“, „Bereich Gegenstände“, „Attacken öffnen“), `world/buildings.ts` (`shortcutAction`: ⌘1 Pokédex, ⌘2 Trainer, ⌘3 Begegnungen, ⌘4 Attacken; Entf und Escape-Abwählen nur bei `area === "welt"`), `app/useShortcuts.ts` (+ test: ⌘1–⌘4 nur bei `area === "pokemon"` und ohne `[data-wizard]`; gesperrter Unterbereich tut nichts), `app/statusContext.ts` (Statusleisten-Kontext je Bereich, z. B. „Staravia, 0397“)

**Interfaces:**
- `type PokemonSub = "pokedex" | "trainer" | "encounters" | "moves"`; Trainer und Begegnungen sind im Umschalter sichtbar, aber gesperrt („Trainer, kommt mit Phase 3b.3b“). Gegenstände hat keinen Umschalter.
- `DataAreaContext { sub; setSub; selected: number | null; select(index); rows: DataRowDto[]; names: NameTablesDto; locked: string | null (DP-Grund) }`; Hand-Höhe je Unterbereich (Spec §9) kommt erst mit 3b.3b (Deck-Umbau), Abweichung in Task 17 notieren.
- Leere Zustände (`EmptyState`): kein Projekt („Kein Projekt geöffnet.“), Diamant/Perl gesperrt (Grund als `why`), nichts gewählt („Kein Pokémon gewählt.“ / „Wähle links eine Art; hier erscheinen Werte und Fundorte.“, Gegenstände entsprechend), Suche ohne Treffer („Nichts gefunden für „…““ mit Taste „Filter lösen“).
- Listen mit 508/471/537 Zeilen: Fenstertechnik wie `TextTable` aus 3c.2, wenn die Messung (Browser, `performance.now()`) über 50 ms liegt.

- [ ] **Step 1: Failing tests**: Umschalter wechselt Liste und Brotkrume; ⌘4 wählt Attacken, ⌘2 tut nichts (gesperrt); Auswahl bleibt je Unterbereich erhalten; `shellState` liest `v1` mit und ohne `sub`; `nextSelection({kind:"item", id: 17})` liefert `area: "gegenstaende"`; `paletteEntries` mit „0397“ trifft die Art; `dataCrumbs` liefert das Menü-Glied; Entf im Bereich Pokémon fragt nicht nach dem Löschen eines Gebäudes.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser: Atlas → Pokémon zeigt die Liste der Arten mit Namen, Nummern und Symbolen; Renderzeit notieren.
- [ ] **Step 5: Commit** `feat(studio): Pokémon and items areas with a sub-area switcher on the 3c.2 area infrastructure`

---

### Task 14: Pokédex-Editor

**Files:**
- Create: `data/pokedex/PokedexList.tsx`, `PokedexView.tsx`, `PokedexHand.tsx` (Reiter Werte · Fähigkeiten und Typen · Lernset · Entwicklung · TM/VM · Rohdaten (nur Tüfteln)), `pokedexModel.ts` (+ test: Summe, Schnitt aller Arten aus `rows[].extra.stats` ohne Member 0 und 494–507, EV-Ertrag ↔ 2-Bit-Felder, Geschlechtsstufen in Worten `{0: "nur männlich", 31: "87,5 % männlich", 63: "75 % männlich", 127: "50 % männlich", 191: "75 % weiblich", 225: "87,5 % weiblich", 254: "nur weiblich", 255: "geschlechtslos"}`, Schlüpfschritte = Zyklen × 256, Wachstum **6 Stufen**, Entwicklungsmethoden mit Parameterart nach §1.4), `useSpecies.ts` (lädt `data_get`, `data_edit` je Feld, hört `dataChanged`), Tests je Komponente
- Modify: `data/PokemonArea.tsx`

**Interfaces:**
- Consumes: `DataDocDto::Species` (mit `saved` für „vorher“), `DataPatch::{Personal, Tm, Learnset, Evolution, CopyMoves}`, `refs`, `TypeBadge`, `StatBar`, `Tabs`, `Table`, `ToggleGrid`, `FilterChips`, `Picker` (mit `preview`: Art mit `TypeBadge`, Attacke mit `TypeBadge` und Kategorie, Gegenstand mit `Sprite`), `Reference`, `NumberField` (`showBase={depth === "tuefteln"}`, `depth` aus `useShell().depth`), `SelectField`, `HexBlock`, `Sprite`.
- Liste „Pokédex“ mit Zähler „493 Arten, 12 Formen“, Suche „Name oder Nummer“, Typ-`FilterChips`, Zeilen mit `Sprite` (`mon`), Ungespeichert-Punkt über `trailing`, `data-species={index}`; Schau: Kopf (Name in `font-titel-s`, `TypeBadge` × 2, Fähigkeiten), sechs `StatBar`s mit Schnitt, Summe mit „vorher“ aus `saved`, „Fundorte“ (`refs Species`: Begegnungen als Welt-`Reference` mit Ortsname des nutzenden Headers; Trainer als `Reference disabled` mit Text „Trainer öffnen kommt mit Phase 3b.3b“) und „Verwendet in“ mit dem Satz „ohne Scripts gezählt“; Hand-Reiter: Werte (sechs `NumberField bits=8`, EV-Ertrag sechs `NumberField max=3`, Fangrate mit Satz „höher ist leichter“, Basis-EP, Basis-Freundschaft `bits=8`, Wachstum `SelectField` (6), Geschlecht `SelectField`, Ei-Gruppen 2 × `SelectField` (15), Schlüpfen „15 Zyklen, 3.840 Schritte“; Fluchtrate und Farbe nur in Tüfteln), Fähigkeiten und Typen (Fähigkeiten über `Picker` mit 124 Einträgen, Typen 2 × `SelectField`, wild getragene Gegenstände `item1`/`item2` über Item-`Picker`), Lernset (`Table` Level `NumberField min=1 max=100` · Attacke `Picker`; „Zeile hinzufügen“ gesperrt ab 20 mit „20 von 20“; Löschen je Zeile; nach jedem Commit stabil nach Level sortiert), Entwicklung (`Table` 7 Zeilen: Methode `SelectField`, Parameter je Methode, Ziel `Picker` plus `Reference area="pokemon"` zum Springen), TM/VM (`ToggleGrid` 92 + 8 mit Namen „TM40 Aero-Ass“ aus `machine_table`, ohne Tabelle „TM40“; Taste „Von anderer Art übernehmen …“ öffnet `Picker` mit Fuß „Ersetzt TM/VM und Lernset von Staravia“ und ruft `CopyMoves` mit Label „Staravia: TM/VM und Lernset wie Staralili“), Rohdaten (Tüfteln: `HexBlock` je Teil-Asset mit `data_raw`, `highlight` folgt dem zuletzt fokussierten Feld). `Tabs.unsaved` je Teil-Asset aus der Dirty-Liste. Jedes Feld committet über `data_edit` mit Label „Staravia: Angriff 75 → 85“; Fehler als `Notice`; gesperrtes Asset zeigt „Pokémon-Daten 0397: 36 Bytes statt 44“ und sperrt die Felder.

- [ ] **Step 1: Failing tests**: `pokedexModel.test.ts`; `PokedexHand.test.tsx` (Reiter; Angriff ändern ruft `data_edit` mit `Personal { field: "attack", value: 85 }` und Label; Lernset-Grenze; Übernehmen ruft `CopyMoves`; gesperrtes Asset; `showBase` nur in Tüfteln); `PokedexView.test.tsx` (StatBar-Werte, „vorher“ aus `saved`, Fundort-`Reference` ruft `goTo`, Trainer-Chip gesperrt mit Text); `PokedexList.test.tsx` (Suche „397“, Typ-Filter, Member 503 heißt „Hitze-Rotom“, Index 520 „Art 0520“; ohne `machine_table` zeigt das Raster „TM40“).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS**, Browser auf einer Platin-Kopie: Staravia wählen, Angriff auf 85, ⌘Z, TM-Raster, „Von anderer Art übernehmen … Staralili“, ⌘Z nimmt beides in einem Schritt zurück (`call.sh history`).
- [ ] **Step 5: Commit** `feat(studio): Pokédex editor with values, abilities, learnset, evolution and TM tabs`

---

### Task 15: Attacken-Editor

**Files:**
- Create: `data/moves/MovesList.tsx`, `MoveView.tsx`, `MoveHand.tsx`, `moveModel.ts` (+ test: Kategorien `{0: "Physisch", 1: "Speziell", 2: "Status"}`, Zielarten aus §1.5, benannte Flag-Bits nach §1.5, unbenannte Bits nur in Tüfteln), `useMove.ts`, Tests
- Modify: `data/PokemonArea.tsx`

**Interfaces:**
- Consumes: `DataDocDto::Move` (`fields`, `description`), `DataPatch::Move`, `refs Move`, `TypeBadge`, `DSScreen variant="textbox" pages={pages(text)} scale={2}` (3c.2) für die Beschreibung; `Reference area="texte"` „Beschreibung bearbeiten“ ruft `goTo({kind:"textArchive", index, message})`; fehlt die Beschreibung, steht „Beschreibung nicht gefunden“.
- Liste „Attacken“ mit Zähler, Suche, Typ-Chips, Kategorie-`SelectField`; Attacke ohne Namen „Attacke 0469“; Name in der Schau als `Reference area="texte"` zur Namensbank; Schau: Karte (Typ, Kategorie, Stärke, Genauigkeit, AP, Merkmale), Beschreibung, „Verwendet in“ (N Lernsets, N Trainer; „ohne Scripts gezählt“); Hand: Spalten Grundwerte (Typ `SelectField` (18), Kategorie `Segmented` Physisch | Speziell | Status, Stärke, Genauigkeit, AP, Priorität `NumberField min=-7 max=5` (i8), Effektchance) · Verhalten (Ziel `SelectField`, benannte Flags als `Checkbox`) · Rohdaten (Tüfteln: Effekt-ID, alle Flag-Bits, `HexBlock`). `data-move={index}`.

- [ ] **Step 1: Failing tests**: Stärke ändern → Label „Flügelschlag: Stärke 60 → 65“; Priorität −1 wird angenommen; unbekanntes Flag-Bit 0x40 bleibt beim Setzen von Kontakt erhalten; Suche; Kategorie-Filter; gesperrtes Member zeigt Grund.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): moves editor`

---

### Task 16: Gegenstände-Editor

**Files:**
- Create: `data/items/ItemsList.tsx`, `ItemView.tsx`, `ItemHand.tsx`, `itemModel.ts` (+ test: Taschen und Feldliste wörtlich nach dspre §1.6, Feldgruppen Werte/Effekte/Funktionen, Party-Parameter nur Tüfteln), `useItem.ts`, Tests
- Modify: `data/ItemsArea.tsx`

**Interfaces:**
- Consumes: `DataDocDto::Item` (`fields`, `description`, `shared_with`), `DataPatch::Item`, `item_table`, `refs Item`, `Sprite` (Cache-Schlüssel `item:<icon>:<palette>` aus `item_table`, nur für sichtbare Zeilen laden), `DSScreen textbox` (Beschreibung), `Reference area="texte"`.
- Liste „Gegenstände“ nach Item-ID mit Symbol, Name, Preis rechts, Taschen-Chips mit Zähler; ohne Item-Tabelle Zeilen „Daten 0017“ ohne Symbol und `Notice kind="info"` „Item-Tabelle nicht lesbar: Namen und Symbole fehlen“; Schau: Karte, Beschreibung, „Verwendet in“ (Trainer-Items, Party-Items, wild getragen); Hand: Werte (Preis `NumberField bits=16`, Taschen, Effekte, Funktionen als `SelectField`/`Switch`) · Rohdaten; bei geteilten Daten in der Hand `Notice kind="warning"` „Preis und Werte gelten auch für 21 andere Gegenstände“ mit Liste, und das Label nennt es („Trank: Preis 300 → 250, gilt für 22 Gegenstände“). `data-item={id}`.

- [ ] **Step 1: Failing tests**: Preis ändern → Label; Liste ohne Tabelle zeigt „Daten 0017“ und die Notiz; Symbol-Cache ruft `get_item_icon` je `icon:palette` einmal; geteilte Daten zeigen die Warnung.
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** → **Step 4: Run, PASS** → **Step 5: Commit** `feat(studio): items editor with icons from the ROM`

---

### Task 17: Szenario `pt-daten.yaml`, Prüfschritt `g4k check-data`, Docs, Gate-Dokument Teil A

**Files:**
- Create: `scripts/agent/scenarios/pt-daten.yaml`, `docs/phase-3b3-gate.md` (Teil A mit leerer Ergebnistabelle)
- Modify: `crates/g4-kernel/src/bin/g4k.rs` (Unterbefehl `g4k check-data <rom.nds> --expect species:397:attack=85 --expect move:17:power=65 --expect item:17:price=250 --expect learnset:397:len=<n+1>`: liest die NARCs aus dem gebauten ROM; Personal und Item **mit uxie**, Attacke und Lernset **roh** über die eigenen Codecs, weil uxie 172 von 471 Vanilla-Attacken ablehnt), `crates/g4-kernel/tests/cli.rs`, `scripts/g4emu/g4emu/scenario.py` (+ `test_scenario.py`: optionaler Abschnitt `check: [["g4k", "check-data", …]]` nach `build`, Exit ≠ 0 lässt das Szenario fehlschlagen), `docs/agent-gate.md` („Doppeldeck: wo was ist“ um Pokémon und Gegenstände; Rezepte Pokédex, Attacken, Gegenstände; `call.sh data_get '{"kind":"species","index":397}'`), `docs/dev-setup.md` (gated `vanilla_data`, uxie-Orakel, `g4k check-data`, proptest), `docs/ROADMAP.md` (3b.3 mit Schnitt a/b, Status „3b.3a implementiert, Gate offen“; „Aus dem Design-System, später“ um die „Später“-Tabelle der Spec; Encounter-Writer als entschieden), `design-system/README.md` („Was die App umsetzt“ + 3b.3a; Abweichungen: Typfarben als Tokens, kein `g4.shell.v2`, keine Hand-Höhe je Unterbereich, Beschreibungen über `data_get`)

- [ ] **Step 1: Failing test** `tests/cli.rs`: `check_data_reads_the_built_narcs_and_fails_on_a_wrong_expectation` (Fixture bauen, `--expect species:1:attack=<falsch>` → Exit 1 mit „species 1 attack: expected …, found …“).
- [ ] **Step 2: Run, FAIL** → **Step 3: Implement** `g4k check-data` und den `check`-Abschnitt in `scenario.py`.
- [ ] **Step 4: Szenario** `pt-daten.yaml`: `edits`: `data_edit species 397 {Personal: {field: "attack", value: 85}}`, `data_edit species 397 {Learnset: …}` (eine Zeile mehr), `data_edit move 17 {Move: {field: "power", value: 65}}`, `data_edit item 17 {Item: {field: "price", value: 250}}`; `build: out.nds`; `check`: die vier Erwartungen; `play`: `continue`, `expect-map 415`, `shot start` als Boot-Test (das ROM startet mit zwölf neu gepackten Ordnern und lädt den Spielstand). Laufen lassen auf einer frischen Kopie `pt-3b3a` unter `_testkopien`.
- [ ] **Step 5: Docs** nach Dateiliste; `docs/phase-3b3-gate.md` Teil A (Spec §1 Gate 3b.3a: Screens wiedererkennbar, ⌘Z über zwei Assets, HexBlock, Symbole, Liste-Renderzeit; Spalte „Wer“ wie in `agent-gate.md`).
- [ ] **Step 6: Volle Prüfung**: `pnpm test`, `pnpm build`, `bash scripts/check-styles.sh`, `bash scripts/check-tokens.sh`, `cargo test --workspace --all-features`, dann `G4_ROMS_DIR=<absolut> scripts/gate.sh` im Hintergrund (jetzt mit `vanilla_data`, rund 35 min); Tabelle in den Bericht.
- [ ] **Step 7: Commit** `feat(gate): data scenario with a check-data step; docs and roadmap for 3b.3a`

---

## Reihenfolge und Parallelität

- Parallel (getrennte Dateien): **1 → 2 ∥ 3 ∥ 4** (Codecs nach dem Grundbaustein), **12** (Frontend-Primitives, unabhängig vom Kernel).
- Seriell danach: **5 → 6 → 7 → 8 → 9 → 10 → 11 → 13 → 14 ∥ 15 ∥ 16 → 17** (14–16 parallel nur, wenn `PokemonArea.tsx`/`ItemsArea.tsx` vorher die Einhängepunkte haben; Commits nacheinander).

## Abschluss

Nach Task 17: `superpowers:finishing-a-development-branch` (Merge nach `main` nach grünem `gate.sh`), danach die Abnahme über die Test-Umgebung (Gate 3b.3a Teil A) in einer eigenen Session. Der Plan 3b.3b (Trainer-Paar-Asset, `create_trainer`, Begegnungen-Editor, Event ↔ Trainer, `PokemonCard`, `g4emu`-Kampfschritte mit RAM-Kalibrierung als Task 1, Hand-Höhe je Unterbereich, Szenarien, Gate Teil B) wird nach dem Merge aus derselben Spec geschrieben.
