# Research 3b.3 "Daten-Editoren": uxie 0.9.7, g4-kernel pattern, gaps, vanilla facts

Date: 2026-10-23. Everything under `Roms/` was only read. Abbreviations:
`U` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/uxie-0.9.7/src`,
`K` = `gen4-studio/crates/g4-kernel/src`, `D` = `$HOME/Projekte/lab/editor/DSPRE/DS_Map` (DSPRE main, commit e751187, 2026-10-13).
PtDE = Platin DE (CPUD), HGDE = HeartGold DE (IPKD), HGUS = HeartGold US (IPKE).

## How this was verified

Beyond reading the code, I ran every uxie codec over every vanilla member of all three projects
(read-only), with a small scratch crate pinned to `uxie =0.9.7` / `chatot =0.8.1` (gen4-studio's
Cargo.lock): parse, write, compare byte for byte. Tools are in
`/private/tmp/claude-502/-Users-POLTEP-Projekte-lab-editor/e1c6e331-c44c-4ea7-ac35-509b47402793/scratchpad/research-3b3/tools/`
(`rt/` Rust round trip, `inv.py` NARC inventory, `enc.py` encounter/trainer layout checks,
`itemtbl.py`, `tm.py`). Build needs `CFLAGS=-D_POSIX_C_SOURCE=200809L` (same as `gen4-studio/.cargo/config.toml`)
and `PATH=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH` (no `cargo` shim on PATH).

### Round-trip results (uxie parse -> uxie write -> compare with vanilla member)

| Family / codec | PtDE | HGDE | HGUS |
|---|---|---|---|
| Encounters `BinaryEncounterFile` | 42/183 identical, 141 write errors | 0/142: 111 wrong length (196 -> 204), 31 write errors | same as HGDE |
| Trainer properties `TrainerProperties` | 928/928 | 738/738 | 738/738 |
| Trainer party `PartyPokemon` (via `TrainerData::to_binary_parts`) | 885/928, 43 too short | 703/738, 35 too short | same as HGDE |
| Personal `PersonalData` | 508/508 | 508/508 | 508/508 |
| Evolutions `EvolutionData` | 0/508 (44 -> 42 bytes) | 0/508 | 0/508 |
| Learnsets `LearnsetData` | 243/508 (rest 2 bytes short) | 240/508 | 240/508 |
| Learnsets + zero pad to 4 bytes | 508/508 | 508/508 | 508/508 |
| Moves `MoveData` | 0/471: 172 parse errors, 299 written as 14 of 16 bytes | same | same |
| Items `ItemData` | 446/446 | 514/514 | 514/514 |
| Egg moves `EggMoveData` (HGSS kowaza) | n/a (overlay) | 1/1 | 1/1 |

## 1. uxie 0.9.7: data types

General: all modules are re-exported from `U/lib.rs:140-205`. uxie has **no library-level
"rominfo"/paths module for these NARCs**. The only per-game paths are:
- `U/cli/paths.rs:13-78`: part of the **binary** (`use uxie::...`, not exported by the lib). Its HGSS paths
  are partly wrong for ROM data (`data/pbr/personal.narc`, `data/pbr/item_data.narc`, `data/pbr/pms.narc`,
  `data/data/kowaza.narc` for moves); all use the DSPRE-legacy `data/` prefix, not ds-rom `files/`.
- `U/decomp_data/paths.rs:117-148` `DecompPaths::dspre_*` (Platinum only, `data/...` prefix).
- Hard-coded fallbacks in loaders/tests: encounters `data/fielddata/encountdata/{d,pl}_enc_data.narc` / `data/a/0/3/7`
  (`U/encounter_file/tests.rs:18-24`), trainers `data/poketool/trainer/tr{data,poke}.narc` / `data/a/0/5/{5,6}`
  (`U/trainer_data/binary.rs:28-34`).
None of these matches the ds-rom layout (`files/...`) that g4-kernel uses, so gen4-studio must keep its own paths
(it already does, in `K/../metadata/archives.yaml`).

Text bank ids (names): `U/text_bank/game_strings.rs:13-131` `TextBankIds { species, items, moves, abilities,
types, locations, trainers, trainer_classes, sounds }` and `TextBankIds::for_game(GameFamily, GameLanguage)`;
documented in `uxie-0.9.7/docs/text_archives.md`. `GameStrings::load_from_dspre` (`:197-245`) only reads
`expanded/textArchives/NNNN.json` and returns an empty table when that folder is missing (our test projects have no
`expanded/` folder), and only species/items/moves/abilities/types; prefers `en_US`. Not usable as is; use `TextBankIds`
plus g4-kernel's own decoding (`Project::location_names` pattern, `K/project.rs:739-753`).

`GameFamily::null_encounter_id()` (`U/game.rs:111-116`): 0xFFFF DP/Pt, 0xFF HGSS. Map header field
`wild_pokemon`: `u16` in `MapHeaderDP`/`MapHeaderPt` (`U/map_header/types.rs:15`, `:35`), `u8` in `MapHeaderHGSS` (`:48`).

### 1.1 Wild encounters: `uxie::encounter_file::BinaryEncounterFile` (`U/encounter_file/binary.rs`)

One struct for all families (`:38-65`), DPPt and HGSS fields mixed:
`walking_rate: u32`, `grass_encounters: [EncounterEntry; 12]` (`EncounterEntry { level: u8, species: u32 }`, `:25-29`),
`swarm_encounters: [u32; 4]` (DPPt uses 2), `day_encounters: [u32; 12]`, `night_encounters: [u32; 12]` (DPPt uses 2 each),
`radar_encounters: [u32; 4]`, `music_encounters: [u32; 4]` (HGSS), `form_encounter_rates: [u32; 5]`, `unown_table_id: u32`,
`dual_slot_{ruby,sapphire,emerald,firered,leafgreen}: [u32; 2]`, `surf_rate: u32`,
`surf_encounters / old_rod / good_rod / super_rod: [WaterEncounterEntry; 5]` (`{ min_level: u8, max_level: u8, species: u32 }`, `:31-36`),
`rock_smash_rate: u32`, `rock_smash_encounters: [WaterEncounterEntry; 2]` (HGSS), `morning_encounters: [EncounterEntry; 12]` (HGSS).

- Reader: `from_binary<R: Read+Seek>(reader, GameFamily)` (`:68-73`) -> `from_binary_dppt` (`:77-250`), `from_binary_hgss` (`:254-405`).
- Writer: `to_binary<W: Write>(&self, writer, GameFamily)` (`:407-413`), validates first (`:415-507`).
- JSON form with symbolic species: `JsonEncounterFile` (`U/encounter_file/json.rs:20-56`), needs a `SymbolTable` (decomp constants); not useful for DSPRE projects.
- Tests (`U/encounter_file/tests.rs`): only struct -> bytes -> struct round trips of synthetic fixtures (`:160-184`, proptest `:552-567`);
  the real-ROM tests are `#[ignore]` and compare struct -> JSON -> struct, **never bytes** (`:304-341`).

**Defects found (verified on vanilla, see table):**
1. DPPt validation rejects empty slots: `ensure_level_range` requires 1..=100 (`:660-672`), applied to every grass level (`:429-432`)
   and to min/max of every surf/rod slot (`:456-459`, `validate_water_levels :674-689`), even when species is 0 and the rate is 0.
   141 of 183 PtDE files fail to write. HGSS: morning levels (`:485-488`) -> 31 of 142 fail.
2. HGSS reader is misaligned. Real layout (196 bytes, confirmed by DSPRE `D/ROMFiles/EncounterFile.cs:478-700` and vanilla data):
   `0x00` six u8 rates (walk, surf, rock smash, old, good, super) + 2 pad; `0x08` 12 x u8 levels; `0x14` morning[12] u16;
   `0x2C` day[12] u16; `0x44` night[12] u16; `0x5C` Hoenn radio[2] u16 + Sinnoh radio[2] u16; `0x64` surf[5] (min u8, max u8, species u16);
   `0x78` rock smash[2]; `0x80` old rod[5]; `0x94` good rod[5]; `0xA8` super rod[5]; `0xBC` swarm[4] u16; end `0xC4`.
   uxie reads `swarm` right after night (`:288-291`, so it gets the radio species, e.g. 311/312/403/403 in HG file 1),
   then `music` (`:293-296`) from the first surf slots, and every water table 8 bytes late; the last 8 bytes are never read.
   The writer writes swarm after night **and** again at the end (`:605-607`, `:637-639`) -> 204 bytes.
3. DPPt reader drops data: grass level read as u32 and cut to u8 (`:85`), 2 pad bytes per water slot skipped (`:151`),
   the 44 bytes `0xF8..0x124` skipped (`:160`) and written as zeros (`:557`). In vanilla PtDE all of these are 0 (checked all 183 files),
   so it is lossless for vanilla, but a hacked file would lose bytes (g4-kernel's lock check would catch it).
4. No DP vs Pt distinction (same layout assumed; DSPRE does the same, `D/ROMFiles/EncounterFile.cs:160-340`). DP unverified (no DP project).

Not modelled at all: HGSS headbutt (`a/2/5/2`), HGSS rock smash item/aux table (`a/2/5/3`), HGSS Safari Zone (`a/2/3/0`),
Pt `encdata_ex.narc` (honey trees etc., semantics unverified), HGSS swarm/radio tables in overlays, Pt Great Marsh/Trophy Garden tables.

### 1.2 Trainers: `uxie::trainer_data` (`U/trainer_data/types.rs`, `binary.rs`)

- `TrainerProperties` (`types.rs:32-41`), 20 bytes (`TRAINER_PROPERTIES_SIZE`, `binary.rs:7`):
  `flags: TrainerFlags` (bitflags u8: `HAS_MOVES=1`, `HAS_ITEMS=2`, `types.rs:4-10`), `trainer_class: u8`, `unknown: u8`,
  `party_count: u8`, `items: [u16; 4]`, `ai_flags: AiFlags` (bitflags u32, `types.rs:12-30`), `double_battle: u32`.
  Reader `TrainerProperties::from_binary` (`binary.rs:122-157`) is **strict**: unknown flag bits or AI bits are an error (`:124-129`, `:137-143`).
  Writer `to_binary` / `to_bytes` (`:159-177`).
- `PartyPokemon` (`types.rs:43-54`): `difficulty: u16` (DP/Pt u16; HGSS u8), `gender_ability_override: u8` (HGSS only),
  `level: u16`, `species: u16` (low 10 bits), `form: u8` (bits 10-15 of the species word), `held_item: Option<u16>`,
  `moves: Option<[u16; 4]>`, `ball_seal: Option<u16>` (Pt and HGSS, not DP). Reader `from_binary(reader, TrainerFlags, GameFamily)` (`binary.rs:180-230`),
  writer `to_binary(writer, flags, family)` (`:232-293`) errors when a flagged field is `None`. `binary_size(flags, family)` (`:295-307`).
- `TrainerData { properties, party: Vec<PartyPokemon> }` (`types.rs:74-78`); `from_binary_parts(props_reader, party_reader, family)` /
  `to_binary_parts(props_writer, party_writer, family)` (`binary.rs:310-347`).
- Loaders `load_dspre_trainer(root, family, id)` / `load_all_dspre_trainers(root, family)` (`binary.rs:14-118`): read
  `unpacked/trainerProperties/NNNN` and `unpacked/trainerParty/NNNN` (matches DSPRE/ds-rom projects), fallback NARCs with the wrong prefix.
- Tests: real-ROM byte comparison exists but `#[ignore]` and env-gated (`binary.rs:924-1027`, first 400 trainers only).
- **Verified:** properties 100 % identical in all three projects; AI flags strict parse passes on vanilla.
  Party writer is **not** byte-identical: vanilla trpoke members are zero-padded to a multiple of 4 bytes, and an empty party
  (trainer 0, `party_count` 0) is 8 zero bytes; uxie writes the unpadded size (e.g. 54 instead of 56) and 0 bytes for trainer 0.
  Rule verified on all members of all three projects: `len = round_up_4(party_count * entry_size)`, and `8` when that is 0;
  padding bytes are always 0. A thin wrapper fixes it.
- Trainer classes: there is **no** trainer class type in uxie (only the `u8` id and the name bank id `TextBankIds::trainer_classes`).
  No trainer graphics, trainer text tables (`trtbl`/`trtblofs`), class gender table, or battle music.

### 1.3 Pokémon base data: `uxie::PersonalData` (`U/personal_data/types.rs:3-31`)

binrw struct, 44 bytes (`PERSONAL_DATA_SIZE`, `binary.rs:5`): `hp, attack, defense, speed, sp_attack, sp_defense, type1, type2,
catch_rate, base_exp: u8`, `ev_yield: u16` (2 bits per stat, accessors `:34-56`), `item1, item2: u16`, `gender_ratio, egg_cycles,
base_friendship, growth_rate, egg_group1, egg_group2, ability1, ability2, safari_flee_rate, color_flip: u8`, 2 pad bytes
(`#[brw(pad_before = 2)]`, `:29`), `tm_compatibility: [u8; 16]` (`can_learn_tm(index)`, `:58-65`).
`from_binary` / `to_binary` / `to_bytes` (`binary.rs:7-21`). Real-ROM test skips members that are not 44 bytes and the HGSS
candidates do not include `a/0/0/2` (`binary.rs:231-299`). **Verified 508/508 identical in all three projects** (pad bytes are 0).

### 1.4 Evolutions: `uxie::EvolutionData` (`U/evolution_data/types.rs:111-129`)

`entries: [EvolutionEntry; 7]`, `EvolutionEntry { method: u16, param: u16, target_species: u16 }`; `EvolutionMethod` enum 0..=26 plus
`Unknown(u16)` (`:8-74`, used only for display). `EVOLUTION_FILE_SIZE = 42` (`:6`). **Vanilla members are 44 bytes**
(2 trailing zero bytes, checked: all zero). The writer emits 42 bytes -> 0/508 identical. The real-ROM test skips members
whose length is not 42 (`binary.rs:155-158`), so it never tests vanilla. Fix: append 2 zero bytes (and lock if they are non-zero).

### 1.5 Learnsets: `uxie::LearnsetData` (`U/learnset_data/types.rs:3-30`, `binary.rs:5-30`)

`entries: Vec<LearnsetEntry { move_id: u16 (9 bits), level: u8 (7 bits) }>`, packed u16 per entry, terminator 0xFFFF.
Writer has no padding; vanilla members are padded with 0x0000 to a multiple of 4 (e.g. member 0 = `ffff0000`).
uxie-only round trip 243/508 (Pt), **508/508 after zero-padding to 4** in all three projects. The real test (`binary.rs:148-175`)
compares whole members and would fail on vanilla. Move ids > 511 cannot be stored (format limit).

### 1.6 Moves: `uxie::MoveData` (`U/move_data/types.rs:47-64`)

binrw, `MOVE_DATA_SIZE = 14` (`binary.rs:7`): `battle_effect: u16`, `split: MoveSplit` (u8 enum 0..2), `power, move_type, accuracy, pp,
side_effect_chance: u8`, `target: u16`, `priority: i8`, `flags: MoveFlags` (bitflags u8 with 6 bits only, `:24-34`, parsed strictly
via `from_bits_strict`, `:59-61`), `contest_appeal, contest_condition: u8`.
**Vanilla members are 16 bytes** (last 2 bytes zero in all 471). And 172 of 471 vanilla moves have flag bit 0x40 (119 moves)
or 0x80 (53 moves) set -> uxie parse error "Invalid move flags bits". Also `MoveSplit::from(u8)` maps anything > 1 to Status (lossy,
but binrw read of `MoveSplit` errors on > 2). The real test skips members that are not 14 bytes (`binary.rs:205-207`), so it is vacuous.
**uxie's move codec is unusable for vanilla**; an own 16-byte codec (or a patched uxie) is required.

### 1.7 Items: `uxie::ItemData` (`U/item_data/types.rs:180-201`, `binary.rs:31-104`)

34 bytes (`ITEM_DATA_SIZE`, `binary.rs:7`): `price: u16`, `hold_effect, hold_effect_param, pluck_effect, fling_effect, fling_power,
natural_gift_power: u8`, 16-bit bitfield (`natural_gift_type:5, prevent_toss:1, is_selectable:1, field_pocket:4, battle_pocket:5`,
`types.rs:48-61`), `field_use_func, battle_use_func, party_use: u8` + 1 pad, party-use flags (7 bytes, `ItemPartyUseFlagsBits`,
`types.rs:63-118`), party-use values (11 bytes `ItemPartyUseValues`, `:120-135`) + 2 pad. `FieldPocket` 0..7 and `BattlePocket` bits
are validated strictly on read (`types.rs:221-240`). **Verified 446/446 (Pt) and 514/514 (HGSS) identical.**
API gap: `ItemPartyUseParam`'s `flags`/`values` are private and `ItemPartyUseFlagsBits`/`ItemPartyUseValues` are only
`pub(crate)` re-exported (`U/item_data/mod.rs:6-8`); external code can call `to_parts()` / `from_parts()` but cannot name the types
(serde of `ItemPartyUseParam` works: flags as `[u8; 7]`). Editing party-use effects field by field from g4-kernel is awkward.

**Item id != item data index:** the item -> (data file, icon, palette, gen-3 id) table is in `arm9.bin` (8 bytes per item,
DSPRE `itemTableOffset`, `D/RomInfo.cs:860-940`; PtDE 0xF0D1C, HGDE 0x100148, HGUS 0x100194). Verified: PtDE 468 entries map onto 446
data files (dataId 0..445), HGSS 537 entries onto 514 files. uxie has no type for this table.

### 1.8 TM/HM list

Not in uxie (only the 128-bit `tm_compatibility` in `PersonalData`). TM/HM -> move table: 100 x u16 in `arm9.bin`
(DSPRE `GetMachineMoveOffset`, `D/RomInfo.cs:1934-1996`; PtDE 0xF0C54, HGDE 0x100080, HGUS 0x1000CC). Verified: TM01 = 264 (Focus Punch),
TM02 = 337; HM05 = 432 (Defog) in Pt, 250 (Whirlpool) in HGSS.

### 1.9 Other relevant uxie pieces

- Egg moves `EggMoveData { entries: Vec<EggMoveEntry { species_id, move_ids }> }` (`U/egg_move_data/types.rs:6-45`); `from_binary`/`to_binary`
  (species entries `20000 + species`, terminator 0xFFFF, `binary.rs:6-50`) and `*_simple` variants (`:53-80`). HGSS `a/2/2/9` member 0: 1/1 identical.
  Platinum egg moves live in overlay 5 at 0x29222 (`binary.rs:331`); in ds-rom projects that is `arm9_overlays/ov005.bin` (path unverified).
- `Narc` reader and writer (`U/narc.rs:94-300`, `to_bytes` `:239`); g4-kernel uses its own `g4_formats::narc` instead.
- `decomp_data::{pokemon, trainers, moves, items}`: readers for pokeplatinum JSON/CSV sources, irrelevant for DSPRE/ds-rom projects.
- Nothing for: trainer classes, trainer text, Pokédex/height, sprites, trade data, Battle Tower, item table, TM table, abilities data.

## 2. How g4-kernel wraps uxie today (pattern for a new asset kind)

### 2.1 Pieces

- **`AssetKind`** (`K/project.rs:25-48`) with `ALL: [AssetKind; 12]` (`:53-66`), `label()` (`:69-84`, English, error messages),
  `slot()` (`:87-102`, index into `ALL`, also sort key of `Store::dirty`, `K/store.rs:1162`). `AssetId { kind, index: u16 }` (`:105-109`).
- **Archive map**: `metadata/archives.yaml` (per family `dp`/`pt`/`hgss`, entry `{ packed, packed_jp?, unpacked }`, embedded at `K/project.rs:16`),
  deserialized into `FamilyArchives` (`:130-145`, one named field per kind; optional ones are `Option`), resolved by `ArchiveMap::for_game(family, language)`
  (`:181-216`) into `entries: [Option<(PathBuf, String)>; 12]` (`:177`). `packed()`, `unpacked_name()`, `kinds()` (`:220-244`).
  The test `embedded_metadata_parses_and_resolves_every_family` asserts the number of kinds per family (9/10/11) (`:864-901`).
- **Location** (`K/project.rs:153-169`): `Member(path)`, `Slice{path,offset,len}` (header table in arm9), `HeaderFiles{dir,count}`,
  `TextSource{json,member}`. `Project::asset_location(kind, index)` (`:412-446`): every archive kind is `Member(unpacked/<name>/NNNN)`
  unless special-cased.
- **Unpack / count**: `Project::ensure_unpacked(kind)` (`:557-600`) unpacks the packed NARC into `unpacked/<name>` only if the folder is missing
  (temp folder + rename); `member_count(kind)` (`:612-625`) counts `NNNN` files in the folder, else the NARC. `Store::ensure_unpacked`
  caches per kind (`K/store.rs:279-285`). The app's `member_counts_impl` iterates `AssetKind::ALL` (`apps/studio/src-tauri/src/commands/project.rs:71-82`).
- **Codec dispatch** (`K/store.rs`): `enum Loaded` (`:68-77`) has one variant per editable kind; `Loaded::is_editable` (`:81-91`),
  `Loaded::from_bytes(kind, bytes, family, lang)` (`:94-133`, exhaustive match; non-editable kinds -> `KernelError::NotEditable`),
  `Loaded::to_bytes(family)` (`:135-151`). uxie errors are wrapped as `KernelError::Uxie(String)`; own g4-formats codecs return `FormatError`
  (`KernelError::Format`). Family conversion `uxie_family` (`K/store.rs:58-64`, also `Project::uxie_family` `K/project.rs:717-723`).
- **Locked assets**: on load, `unwritable_reason(value, bytes, family)` (`K/store.rs:154-168`) re-serializes; if the bytes differ or writing
  fails, `Asset.locked = Some(reason)` (`:173-182`, set in `load` `:329-345` and `reload_from_disk` `:1035-1055`). Locked: shown read-only,
  `apply_edit` refuses with `KernelError::ReadOnly` (`:440-445`), never dirty (`is_dirty` `:1141-1152`), never saved. Tests:
  `crates/g4-kernel/tests/store.rs:651-720` (event file with trailing bytes / Pt warp height).
  Consequence for 3b.3: with uxie as is, **every** HGSS encounter file, every evolution file, ~half the learnsets and ~5 % of trainer parties
  would load as locked, and 172 moves would not load at all.
- **Edits, undo, dirty**: typed getters (`Store::event` `:406-415`, pattern `match &self.load(id)?.current`), typed editors
  (`edit_event` `:533-547`: `apply_edit(id, label, |loaded| match loaded { Loaded::Event(e) => Ok(edit(e)?), _ => Err(mismatch(id)) })`).
  `apply_edit` (`:432-467`): clone, run closure, serialize; unchanged bytes record nothing; else push `Changeset { asset, before, after, label }`
  (`:184-190`, **one asset per changeset**) and clear redo. `undo`/`redo` (`:1109-1130`) restore via `Loaded::from_bytes` (`restore` `:1093-1105`).
  Dirty = serialized current != `saved_bytes` (`:1141-1152`); `dirty()` sorted by `(slot, index)` (`:1155-1164`).
- **Save**: `Store::save` (`:1172-1226`): for each dirty asset re-read the disk (`read_base`, `:1291-1302`) and compare with `saved_bytes`;
  any mismatch -> `KernelError::Stale(id)` and nothing is written. Then per `Location`: `Member` -> `write_atomic(project.root, path, bytes)`
  (`K/fsutil.rs:10-38`: temp file under `.g4/tmp/`, rename; never inside the member folder because DSPRE packs every file there).
  `reload_from_disk` (`:1035`) and `accept_disk_as_base` (`:1066-1090`) handle external changes.
- **New members**: `append_member(kind, bytes)` / `next_member(kind)` (`:601-628`): index = member count, refuses an existing file and
  counts >= 0xFFFF, writes at once with `write_atomic`, **not undoable**. Examples: `create_event_file` (`:701-719`, also inserts the asset
  unlocked), `duplicate_event_file` (`:724-731`), `create_text_archive` (`:905-937`). `member_bytes` (`:1024-1030`) gives the shown bytes.
- **Build**: `build_prepared` packs every `project.archives.kinds()` whose `unpacked/<name>` folder exists (`K/build.rs:188-205`) and writes
  only changed NARCs. Adding a kind to `archives.yaml` therefore makes the build repack that DSPRE folder (the DSPRE test projects already have
  `unpacked/encounters`, `trainerProperties`, ... and they are byte-identical to the packed NARCs, see 4.1).
- **Text as the newest store kind**: `AssetKind::TextArchive` uses `Location::TextSource` (`K/project.rs:413-418`), loads JSON or the member
  whichever is newer (`read_text_doc` `K/store.rs:1329-1345`, `text_winner` `K/text.rs:252`), its canonical bytes are JSON, save writes JSON and
  member (`K/text.rs:345` `write_archive`), problems are reported in `SaveReport.problems`. `edit_text_archive` (`K/store.rs:884-900`) returns
  `Option<ChangeId>`.
- **Scripts as an out-of-store kind**: `AssetKind::Script` is not editable in the store (`K/store.rs:126-131`); `K/scripts.rs` writes rotom sources
  directly (`write_source :790`, `compile :584`) with `write_atomic`. Pattern for data that is not a NARC member.
- **Names from text banks**: `Project::location_names` (`K/project.rs:739-753`) = `TextBankIds::for_game(...).locations`, member decoded with
  `TextArchiveDoc::from_member` (`K/text.rs:225`). Same pattern works for species/moves/items/trainer classes/trainers (reads the member,
  so unsaved text edits and a newer JSON are not reflected; consider going through the store/`read_text_doc`).
- **Frontend touch points** (main; note the phase-3c.1 worktree reworks the UI): `apps/studio/src/api.ts:372-383` (`AssetKind` union),
  `apps/studio/src/assets.ts:3-15` (`KIND_LABELS`, German), `apps/studio/src/reload.ts:44-80` (`reloadPlan` per kind),
  `apps/studio/src/assets.test.ts:6-20` (asserts every kind has a label), Tauri commands in `apps/studio/src-tauri/src/commands/*.rs`
  (e.g. `events.rs:256` calls `store.edit_event`).

### 2.2 Checklist for a new store-editable kind (e.g. `Encounter`)

1. `K/project.rs`: add variant to `AssetKind`, to `ALL` (length +1), `label()`, `slot()`; add field to `FamilyArchives`; extend
   `ArchiveMap.entries` array length and `for_game`; update the kinds-per-family test (`:864-901`). If the kind is not a plain member,
   add a `Location` variant and extend every `match` on `Location` (`K/store.rs` `read_member :311-326`, `accept_disk_as_base :1070-1082`,
   `save :1190-1217`, `read_location :1266-1285`; `K/project.rs` `read_headers :783`, `header_count :796`).
2. `metadata/archives.yaml`: packed/unpacked per family. **Gotcha:** paths can differ per game within a family, which the yaml cannot express today
   (only `packed_jp`): SoulSilver encounters `a/1/3/6` vs HeartGold `a/0/3/7` (`D/RomInfo.cs:2514`); Diamond `d_enc_data.narc` vs Pearl
   `p_enc_data.narc` (`D/RomInfo.cs:2339`), Pearl personal `poketool/personal_pearl/personal.narc` (`:2368-2376`). Needs e.g. a `packed_ss`/per-game-code
   override using `Project.game_code` (IPG* = SoulSilver, APA* = Pearl).
3. Codec: own one in `crates/g4-formats/src/<name>.rs` (pattern `area_data.rs`: `read(bytes, family) -> Result<_, FormatError>`, `write() -> Vec<u8>`,
   format doc in `docs/formats/<name>.md`, vanilla test in `crates/g4-formats/tests/vanilla.rs` with `G4_TEST_PROJECT`) or a uxie wrapper.
4. `K/store.rs`: `Loaded` variant, `is_editable`, `from_bytes`, `to_bytes`; typed getter + `edit_<kind>` (+ `create_<kind>`/`duplicate_<kind>` via
   `append_member` if needed). Locking comes for free through `unwritable_reason`.
5. Kernel tests in `crates/g4-kernel/tests/store.rs` (byte-identical vanilla load, edit/undo/dirty/save, locked file).
6. App: Tauri commands + `api.ts` union + `assets.ts` label + `reload.ts` plan.

### 2.3 Design points the spec must decide

- **Granularity**: per-member assets (fits `Location::Member`, undo/dirty per member; a list view of 508 Pokémon or 928 trainers then reads
  summaries from disk like `location_names`, or loads hundreds of assets) vs. a whole-archive table asset like `HeaderTable`
  (one changeset per edit holds the whole table; needs a new `Location`).
- **Trainers span two NARCs plus text**: properties (`party_count`, `flags`) and party must change together; `Changeset` covers one asset,
  so either a combined asset (new `Location`, e.g. props+party pair) or two kinds with grouped undo (not supported today). A new trainer
  also needs a name message appended to bank 618 (Pt) / 729 (HGSS) and possibly entries in the trainer text tables
  (`trtbl.narc`/`trtblofs.narc`, HGSS `a/0/5/7`/`a/1/3/1`, one member each). Existing create functions are "not undoable".
- **Encounter linkage**: header `wild_pokemon` (u16, 0xFFFF none in DP/Pt; u8, 0xFF none in HGSS -> at most 255 usable HGSS encounter files).
- **Arm9 tables** (item table, TM list, class gender table) are `Slice`-like assets inside `arm9.bin` (the header table already uses `Location::Slice`
  + `write_slice`, `K/store.rs:1356-1385`). Offsets are per game code (DSPRE values above).

## 3. Writer gaps (what uxie lacks or gets wrong)

| Family | uxie reader | uxie writer | Gap for byte-identical vanilla |
|---|---|---|---|
| Encounters DPPt | ok for vanilla (drops gap/pad/upper level bytes, all 0 in vanilla) | rejects empty slots (level 0) -> 141/183 Pt files fail | **own codec needed** (or relax validation); DP unverified |
| Encounters HGSS | **misaligned** (swarm read at radio offset, water tables 8 bytes late) | **broken** (swarm written twice, 204 bytes) + level-0 rejection | **own codec needed** (roadmap's original plan stands) |
| Trainer properties | ok, strict flags | ok, 100 % | none |
| Trainer party | ok | no 4-byte padding, empty party not 8 bytes | wrapper: pad to 4, min 8 |
| Personal | ok | ok, 100 % | none |
| Evolutions | ok | 42 instead of 44 bytes | wrapper: append 2 zero bytes |
| Learnsets | ok | no 4-byte padding | wrapper: pad to 4 |
| Moves | **fails on 172/471** (flag bits 0x40/0x80) | 14 instead of 16 bytes | **own codec needed** (or patch uxie) |
| Items | ok | ok, 100 % | none; party-use types not nameable outside uxie |
| Egg moves HGSS | ok | ok | none |
| Item table, TM list, trainer class gender table, trainer text tables, headbutt/safari/rock-smash (HGSS), encdata_ex (Pt) | none | none | own code if in scope |
| Names (species/moves/items/classes/trainers) | `TextBankIds` ok; `GameStrings` needs `expanded/` JSON | n/a | use g4-kernel text module |

The memory note "uxie 0.9.7 already has encounter `to_binary`" is true but that writer is not usable: it fails on 77 % of Pt
files and corrupts every HGSS file. The roadmap's original plan (own encounter codec in `g4-formats`) should stay.
Moves also need an own codec. Trainers, evolutions and learnsets work with a few-line padding wrapper; personal and items work as is.

## 4. Vanilla facts (read-only check of the three projects)

### 4.1 NARC paths and member counts

ds-rom paths (relative to the project root). In all three projects the DSPRE folder `unpacked/<name>` exists, has the same member count
and is byte-identical to the packed NARC (checked member by member).

| Data | Pt (PtDE) | count | HGSS (HGDE = HGUS) | count | member size |
|---|---|---|---|---|---|
| Encounters (`unpacked/encounters`) | `files/fielddata/encountdata/pl_enc_data.narc` | 183 | `files/a/0/3/7` (HG; SS uses `a/1/3/6`, also present in HG ROMs, 142 members) | 142 | Pt 424, HGSS 196 |
| Trainer properties (`trainerProperties`) | `files/poketool/trainer/trdata.narc` | 928 | `files/a/0/5/5` | 738 | 20 |
| Trainer party (`trainerParty`) | `files/poketool/trainer/trpoke.narc` | 928 | `files/a/0/5/6` | 738 | 8..n, multiple of 4 |
| Personal (`personalPokeData`) | `files/poketool/personal/pl_personal.narc` | 508 | `files/a/0/0/2` | 508 | 44 |
| Moves (`moveData`) | `files/poketool/waza/pl_waza_tbl.narc` | 471 | `files/a/0/1/1` | 471 | 16 |
| Items (`itemData`) | `files/itemtool/itemdata/pl_item_data.narc` | 446 | `files/a/0/1/7` | 514 | 34 |
| Learnsets (`learnsets`) | `files/poketool/personal/wotbl.narc` | 508 | `files/a/0/3/3` | 508 | 4..n, multiple of 4 |
| Evolutions (`evolutions`) | `files/poketool/personal/evo.narc` | 508 | `files/a/0/3/4` | 508 | 44 |
| Egg moves (`eggMoves`) | overlay 5 (DSPRE custom NARC optional) | - | `files/a/2/2/9` | 1 | 4092 |
| Pt extended encounters (`encounterExtended`) | `files/arc/encdata_ex.narc` | 12 | - | - | mixed |
| HGSS headbutt / rock smash / safari | - | - | `files/a/2/5/2` / `a/2/5/3` / `a/2/3/0` | 540 / 540 / 12 | mixed / 4 / 912 |
| Trainer text table / offsets | `poketool/trmsg/trtbl.narc` / `trtblofs.narc` (DSPRE, `D/RomInfo.cs:2441-2443`) | 1 / 1 | `files/a/0/5/7` / `a/1/3/1` | 1 / 1 | - |
| Text archives | `files/msgdata/pl_msg.narc` | 724 | `files/a/0/2/7` | 829 | - |

DSPRE sources for the paths: `D/RomInfo.cs:2332-2366` (DP), `:2390`, `:2421-2444` (Pt), `:2457-2514` (HGSS). Vanilla quirks seen:
trainer 0 is all zero (props) with 8 zero party bytes; trainer flags in Pt: 672 none, 198 moves, 4 items, 54 both; `unknown` byte always 0;
`double_battle` in {0, 2}; max trainer class id Pt 102, HGSS 124; HGSS encounter files with walk rate 0 have level 0 slots.
Personal has 508 members vs 496 species names (extra members are alternate forms; inference, not checked).
Moves 471 members vs 468 move names. Items: data index comes from the arm9 item table (see 1.7), not the item id.

### 4.2 Names in the text archives (decoded with chatot, first entries shown)

Bank = member index in `pl_msg.narc` / `a/0/2/7`. Same ids in uxie `TextBankIds` (Western languages) and DSPRE `RomInfo` (`D/RomInfo.cs:1369-1500`).

| Names | Pt (PtDE) | msgs | HGSS (HGDE and HGUS) | msgs | Sample |
|---|---|---|---|---|---|
| Species | 412 | 496 | 237 | 496 | "-----", "BISASAM" / "BULBASAUR" |
| Items | 392 | 468 | 222 | 537 | "Keines", "Meisterball" / "None", "Master Ball" |
| Moves | 647 | 468 | 750 | 468 | "-", "Pfund" / "Pound" |
| Abilities | 610 | 124 | 720 | 124 | " -", "Duftnote" / "Stench" |
| Types | 624 | 18 | 735 | 18 | "NORMAL", "KAMPF" / "FIGHTING" |
| Trainer names | 618 | 928 | 729 | 738 | "{TRAINER_NAME:Gunther}" (chatot shows a `TRAINER_NAME` command around each name) |
| Trainer classes | 619 | 105 | 730 | 129 | "[PK][MN]-Trainer", "Teenager" / "Youngster" |
| Locations | 433 | 126 | 279 | 235 | "Zweiblattdorf" / "Twinleaf Town" |

Further banks per DSPRE (not decoded here): Pt alternate species name banks 413, 712-716; HGSS 238, 817-821
(`D/RomInfo.cs:1440-1453`, DSPRE marks 413/238 with "?"); HGSS item descriptions 221 (`:1424`); Pt EN trainer class descriptions 620 (`:1475-1478`).
Japanese and Korean use other ids (uxie table). Trainer name count equals trdata count in both families (928 / 738).

### 4.3 arm9 tables (verified offsets)

| Table | PtDE | HGDE | HGUS | Layout |
|---|---|---|---|---|
| Item table (item -> data, icon, palette, gen3) | 0xF0D1C | 0x100148 | 0x100194 | 8 bytes x item count (468 Pt, 537 HGSS) |
| TM/HM moves | 0xF0C54 | 0x100080 | 0x1000CC | 100 x u16 (TM01-92, HM01-08) |
| Trainer class gender table (DSPRE, not checked) | 0xF076C | 0xFFB44 | 0xFFB90 | vanilla count 105 Pt / 128 HGSS (`D/RomInfo.cs:1503-1545`) |

## Unverified / open

- DP (Diamond/Pearl) everything: no DP project at hand.
- Semantics of Pt `encdata_ex.narc`, HGSS `a/2/5/2`, `a/2/5/3`, `a/2/3/0` layouts.
- Trainer class gender table offsets (taken from DSPRE only).
- Personal members 496..507 being forms (inferred from counts).
- Pt egg-move overlay path in ds-rom layout (`arm9_overlays/ov005.bin` assumed).
