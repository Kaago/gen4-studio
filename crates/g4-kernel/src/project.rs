use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use g4_formats::area_data::AreaData;
use g4_formats::game::{Family, Language};
use g4_formats::narc::Narc;
use serde::{Deserialize, Serialize};

use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::text::TextArchiveDoc;
use crate::textproject::manifest::{SchemaState, refusal, schema_state};
use crate::textproject::source::{EntityUnit, UnitCache, unit_of};

const ARCHIVES_YAML: &str = include_str!("../metadata/archives.yaml");
const HEADER_TABLES_YAML: &str = include_str!("../metadata/header_tables.yaml");

/// Size in bytes of one `mapname.bin` record (NUL-padded internal map name).
const MAPNAME_RECORD: usize = 16;

/// Size in bytes of one map header in `arm9.bin`.
const MAP_HEADER_SIZE: usize = uxie::map_header::MAP_HEADER_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetKind {
    Map,
    Matrix,
    AreaData,
    Event,
    TextArchive,
    MapTexture,
    BuildingTexture,
    BuildingModel,
    InteriorBuildingModel,
    /// The map header table, a fixed slice of `arm9/arm9.bin`. Store-only: it has no
    /// archive, so it is never unpacked, packed or built.
    HeaderTable,
    /// The NARC the Dynamic Headers patch moves the header table into (Platinum and
    /// HGSS). Store-only like the header table: never editable and never unpacked by the
    /// store (the header table asset reads and writes its folder), but an archive for
    /// `build`, which packs the folder when it exists.
    DynamicHeaders,
    /// The script archive: one member per script file (level scripts included). The store
    /// does not edit it (scripts are rotom sources, see the `scripts` module); `build` packs
    /// `unpacked/scripts`.
    Script,
    /// Wild encounters of one encounter file (`encounters`).
    Encounter,
    /// Base stats and TM/HM bits of one species or form (`personalPokeData`).
    Personal,
    /// Level-up moves of one species or form (`learnsets`).
    Learnset,
    /// Evolutions of one species or form (`evolutions`).
    Evolution,
    /// One move (`moveData`).
    Move,
    /// One item data member (`itemData`); item ids map to members through the arm9 item
    /// table.
    Item,
    /// One trainer's class, items, AI and party size (`trainerProperties`).
    TrainerProperties,
    /// One trainer's party (`trainerParty`); decodable only with its properties.
    TrainerParty,
    /// The trainer message table (`trainerTextTable`, one member).
    TrainerTextTable,
    /// Per-trainer offsets into the trainer message table (`trainerTextOffset`, one member).
    TrainerTextOffsets,
    /// The item bag icons (`itemIcons`): NANR/NCER members, then NCGR and NCLR members the
    /// item table points at. Read only: counted and located, never edited (see `data::icons`).
    ItemIcons,
    /// The Pokémon menu icons (`monIcons`): the NCLR, animation members, then one NCGR per
    /// icon. Read only like the item icons.
    MonIcons,
    /// One trainer: its `TrainerProperties` and `TrainerParty` members as one asset (the
    /// store's `TrainerDoc`, see [`Location::MemberPair`]). Store-only like the header table:
    /// it has no archive of its own; its member count is the smaller of the two archives'.
    Trainer,
    /// The Pokémon battle sprites (`pokemonBattleSprites`): six members per species, four
    /// encrypted linear NCGR (back and front, female and male) and two NCLR (normal and
    /// shiny). Read only like the icons (see `data::battle_sprites`).
    PokemonBattleSprites,
    /// The battle sprites of the forms and the egg (`otherPokemonBattleSprites`), NCGR and
    /// NCLR in an order of their own. Read only.
    OtherPokemonBattleSprites,
    /// The trainer class graphics (`trainerGraphics`): five members per class, the battle
    /// image (an encrypted linear NCGR) at `5·k + 4`, its palettes at `5·k + 1`. Read only.
    TrainerGraphics,
}

impl AssetKind {
    /// Every kind, in the order archives are unpacked, packed and listed: the world kinds,
    /// the store-only kinds and the scripts, then the data kinds, and last the trainer, the
    /// store-only pair of two data kinds.
    pub const ALL: [AssetKind; 28] = [
        AssetKind::Map,
        AssetKind::Matrix,
        AssetKind::AreaData,
        AssetKind::Event,
        AssetKind::TextArchive,
        AssetKind::MapTexture,
        AssetKind::BuildingTexture,
        AssetKind::BuildingModel,
        AssetKind::InteriorBuildingModel,
        AssetKind::HeaderTable,
        AssetKind::DynamicHeaders,
        AssetKind::Script,
        AssetKind::Encounter,
        AssetKind::Personal,
        AssetKind::Learnset,
        AssetKind::Evolution,
        AssetKind::Move,
        AssetKind::Item,
        AssetKind::TrainerProperties,
        AssetKind::TrainerParty,
        AssetKind::TrainerTextTable,
        AssetKind::TrainerTextOffsets,
        AssetKind::ItemIcons,
        AssetKind::MonIcons,
        AssetKind::Trainer,
        AssetKind::PokemonBattleSprites,
        AssetKind::OtherPokemonBattleSprites,
        AssetKind::TrainerGraphics,
    ];

    /// Human-readable name used in error messages.
    pub fn label(self) -> &'static str {
        match self {
            AssetKind::Map => "map",
            AssetKind::Matrix => "matrix",
            AssetKind::AreaData => "area data",
            AssetKind::Event => "event file",
            AssetKind::TextArchive => "text archive",
            AssetKind::MapTexture => "map texture",
            AssetKind::BuildingTexture => "building texture",
            AssetKind::BuildingModel => "building model",
            AssetKind::InteriorBuildingModel => "interior building model",
            AssetKind::HeaderTable => "header table",
            AssetKind::DynamicHeaders => "dynamic headers",
            AssetKind::Script => "script file",
            AssetKind::Encounter => "encounter file",
            AssetKind::Personal => "personal data",
            AssetKind::Learnset => "learnset",
            AssetKind::Evolution => "evolution data",
            AssetKind::Move => "move",
            AssetKind::Item => "item",
            AssetKind::TrainerProperties => "trainer properties",
            AssetKind::TrainerParty => "trainer party",
            AssetKind::TrainerTextTable => "trainer text table",
            AssetKind::TrainerTextOffsets => "trainer text offsets",
            AssetKind::ItemIcons => "item icons",
            AssetKind::MonIcons => "menu icons",
            AssetKind::Trainer => "trainer",
            AssetKind::PokemonBattleSprites => "battle sprites",
            AssetKind::OtherPokemonBattleSprites => "form battle sprites",
            AssetKind::TrainerGraphics => "trainer graphics",
        }
    }

    /// Index into `ALL`.
    pub(crate) fn slot(self) -> usize {
        match self {
            AssetKind::Map => 0,
            AssetKind::Matrix => 1,
            AssetKind::AreaData => 2,
            AssetKind::Event => 3,
            AssetKind::TextArchive => 4,
            AssetKind::MapTexture => 5,
            AssetKind::BuildingTexture => 6,
            AssetKind::BuildingModel => 7,
            AssetKind::InteriorBuildingModel => 8,
            AssetKind::HeaderTable => 9,
            AssetKind::DynamicHeaders => 10,
            AssetKind::Script => 11,
            AssetKind::Encounter => 12,
            AssetKind::Personal => 13,
            AssetKind::Learnset => 14,
            AssetKind::Evolution => 15,
            AssetKind::Move => 16,
            AssetKind::Item => 17,
            AssetKind::TrainerProperties => 18,
            AssetKind::TrainerParty => 19,
            AssetKind::TrainerTextTable => 20,
            AssetKind::TrainerTextOffsets => 21,
            AssetKind::ItemIcons => 22,
            AssetKind::MonIcons => 23,
            AssetKind::Trainer => 24,
            AssetKind::PokemonBattleSprites => 25,
            AssetKind::OtherPokemonBattleSprites => 26,
            AssetKind::TrainerGraphics => 27,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetId {
    pub kind: AssetKind,
    pub index: u16,
}

impl std::fmt::Display for AssetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:04}", self.kind, self.index)
    }
}

#[derive(Debug, Deserialize)]
struct ArchiveEntry {
    packed: String,
    packed_jp: Option<String>,
    /// Paths for single games of the family, keyed by the game code (Platinum DE `CPUD`) or
    /// its first three characters (SoulSilver `IPG`, Pearl `APA`), the full code first; they
    /// take precedence over `packed_jp`.
    by_game: Option<BTreeMap<String, String>>,
    unpacked: String,
}

#[derive(Debug, Deserialize)]
struct FamilyArchives {
    maps: ArchiveEntry,
    matrices: ArchiveEntry,
    area_data: ArchiveEntry,
    events: ArchiveEntry,
    text_archives: ArchiveEntry,
    map_textures: ArchiveEntry,
    building_textures: ArchiveEntry,
    building_models: ArchiveEntry,
    /// Only HGSS has interior building models.
    interior_building_models: Option<ArchiveEntry>,
    /// Only Platinum and HGSS have the Dynamic Headers NARC.
    dynamic_headers: Option<ArchiveEntry>,
    scripts: ArchiveEntry,
    encounters: ArchiveEntry,
    personal: ArchiveEntry,
    learnsets: ArchiveEntry,
    evolutions: ArchiveEntry,
    moves: ArchiveEntry,
    items: ArchiveEntry,
    trainer_properties: ArchiveEntry,
    trainer_party: ArchiveEntry,
    trainer_text_table: ArchiveEntry,
    trainer_text_offsets: ArchiveEntry,
    item_icons: ArchiveEntry,
    mon_icons: ArchiveEntry,
    pokemon_battle_sprites: ArchiveEntry,
    other_pokemon_battle_sprites: ArchiveEntry,
    trainer_graphics: ArchiveEntry,
    /// Folders DSPRE unpacks that no asset kind covers, packed by the build only (see
    /// [`ArchiveMap::pack_only`]). DP has none.
    #[serde(default)]
    pack_only: Vec<ArchiveEntry>,
}

/// DSPRE folder names of the optional archives. They are also reported for families
/// without that archive, so that `unpacked_name` is total.
const INTERIOR_UNPACKED_NAME: &str = "interiorBuildingModels";
const DYNAMIC_HEADERS_UNPACKED_NAME: &str = "dynamicHeaders";
/// What `unpacked_name` reports for the trainer, which has two folders (see
/// [`Location::MemberPair`]).
const TRAINER_PAIR_NAME: &str = "trainerProperties+trainerParty";

/// Where the bytes of one asset live on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// A whole file: an unpacked archive member.
    Member(PathBuf),
    /// `len` bytes at `offset` inside a larger file (the header table in `arm9.bin`).
    Slice {
        path: PathBuf,
        offset: u64,
        len: usize,
    },
    /// `count` files `0000..` of 24 bytes each in `dir` (the header table once the Dynamic
    /// Headers patch is applied); the asset's bytes are their concatenation.
    HeaderFiles { dir: PathBuf, count: usize },
    /// A text archive: its chatot JSON source and its member. The asset's bytes are the
    /// canonical JSON.
    TextSource { json: PathBuf, member: PathBuf },
    /// A trainer: its properties member and its party member (archives
    /// `TrainerProperties` and `TrainerParty`, same index). The asset's bytes are
    /// `u32 len(props) | props | party` (little endian).
    MemberPair { props: PathBuf, party: PathBuf },
    /// An asset of a text project ([`SourceMode::TextProject`]): its unit's JSON under `data/`
    /// is its source, `members` (one of the locations above) the build's intermediate state.
    /// The asset's bytes are those of `members`, read from the JSON
    /// ([`crate::textproject::source::EntitySource::read`]).
    Entity {
        unit: EntityUnit,
        members: Box<Location>,
    },
}

/// Where the store's assets come from, read once from `g4-project.json` when the project is
/// opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SourceMode {
    /// No manifest: the DSPRE folder's members are the source.
    Dspre,
    /// A text project of the current schema: the entity JSON under `data/` is the source.
    TextProject,
    /// A text project of an older schema (to migrate first): read from the members, never
    /// edited.
    Older { schema: u32 },
    /// A text project of a newer Studio: read from the members, never edited.
    Newer { schema: u32 },
    /// A manifest that does not read; `reason` names file, line and column.
    Broken { reason: String },
}

impl SourceMode {
    /// The mode of the folder at `root`.
    fn of(root: &Path) -> SourceMode {
        match schema_state(root) {
            Ok(SchemaState::Missing) => SourceMode::Dspre,
            Ok(SchemaState::Current) => SourceMode::TextProject,
            Ok(SchemaState::Older(schema)) => SourceMode::Older { schema },
            Ok(SchemaState::Newer(schema)) => SourceMode::Newer { schema },
            Err(KernelError::Compile(problems)) => SourceMode::Broken {
                reason: problems
                    .iter()
                    .map(|p| format!("{}:{}:{}: {}", p.file.display(), p.line, p.col, p.message))
                    .collect::<Vec<_>>()
                    .join("\n"),
            },
            Err(e) => SourceMode::Broken {
                reason: e.to_string(),
            },
        }
    }
}

/// Archive locations (packed NARC and unpacked folder name) per asset kind,
/// resolved for one game family and language.
#[derive(Debug, Clone)]
pub struct ArchiveMap {
    /// Indexed by `AssetKind::slot`; `None`: the archive does not exist in this family (or
    /// the kind has none, like the header table).
    entries: [Option<(PathBuf, String)>; AssetKind::ALL.len()],
    /// See [`ArchiveMap::pack_only`].
    pack_only: Vec<(PathBuf, String)>,
}

impl ArchiveMap {
    /// The archives of `game_code` (family and language derived from it by the caller).
    /// Public for tools that read a built ROM without a project (`g4k check-data`).
    pub fn for_game(
        family: Family,
        language: Language,
        game_code: &str,
    ) -> Result<ArchiveMap, KernelError> {
        let mut all: HashMap<String, FamilyArchives> = serde_yaml::from_str(ARCHIVES_YAML)
            .map_err(|e| KernelError::InvalidProject(format!("archives.yaml: {e}")))?;
        let key = match family {
            Family::Dp => "dp",
            Family::Pt => "pt",
            Family::Hgss => "hgss",
        };
        let archives = all.remove(key).ok_or_else(|| {
            KernelError::InvalidProject(format!("archives.yaml: no entry for {key}"))
        })?;
        let japanese = language == Language::Japanese;
        let game = game_code.get(..3).unwrap_or(game_code);
        let resolve = |mut e: ArchiveEntry| {
            let by_game = e
                .by_game
                .as_mut()
                .and_then(|m| m.remove(game_code).or_else(|| m.remove(game)));
            let packed = match (by_game, e.packed_jp) {
                (Some(path), _) => path,
                (None, Some(jp)) if japanese => jp,
                _ => e.packed,
            };
            (PathBuf::from(packed), e.unpacked)
        };
        Ok(ArchiveMap {
            entries: [
                Some(resolve(archives.maps)),
                Some(resolve(archives.matrices)),
                Some(resolve(archives.area_data)),
                Some(resolve(archives.events)),
                Some(resolve(archives.text_archives)),
                Some(resolve(archives.map_textures)),
                Some(resolve(archives.building_textures)),
                Some(resolve(archives.building_models)),
                archives.interior_building_models.map(resolve),
                None,
                archives.dynamic_headers.map(resolve),
                Some(resolve(archives.scripts)),
                Some(resolve(archives.encounters)),
                Some(resolve(archives.personal)),
                Some(resolve(archives.learnsets)),
                Some(resolve(archives.evolutions)),
                Some(resolve(archives.moves)),
                Some(resolve(archives.items)),
                Some(resolve(archives.trainer_properties)),
                Some(resolve(archives.trainer_party)),
                Some(resolve(archives.trainer_text_table)),
                Some(resolve(archives.trainer_text_offsets)),
                Some(resolve(archives.item_icons)),
                Some(resolve(archives.mon_icons)),
                // The trainer is a pair of the two trainer archives, no archive of its own.
                None,
                Some(resolve(archives.pokemon_battle_sprites)),
                Some(resolve(archives.other_pokemon_battle_sprites)),
                Some(resolve(archives.trainer_graphics)),
            ],
            pack_only: archives.pack_only.into_iter().map(resolve).collect(),
        })
    }

    /// Packed archive path, relative to the project root. `None`: this archive does not
    /// exist in the project's family (interior building models outside HGSS).
    pub fn packed(&self, kind: AssetKind) -> Option<&Path> {
        self.entries[kind.slot()]
            .as_ref()
            .map(|(packed, _)| packed.as_path())
    }

    /// Folder name below `unpacked/`, for example `maps` or `eventFiles`. Meaningless for
    /// the header table, which has no folder of its own.
    pub fn unpacked_name(&self, kind: AssetKind) -> &str {
        match &self.entries[kind.slot()] {
            Some((_, name)) => name,
            // Only the optional archives (and the header table) can be missing.
            None if kind == AssetKind::DynamicHeaders => DYNAMIC_HEADERS_UNPACKED_NAME,
            // No folder of its own: a name that never is one, so a path built from it never
            // reaches another archive's members.
            None if kind == AssetKind::Trainer => TRAINER_PAIR_NAME,
            None => INTERIOR_UNPACKED_NAME,
        }
    }

    /// The pack entries of the family (`pack_only` in `archives.yaml`): the packed path
    /// (relative to the project root) and the folder name below `unpacked/` of each folder
    /// DSPRE unpacks that no asset kind covers. The build packs them like the archives; no
    /// editor reads them.
    pub fn pack_only(&self) -> &[(PathBuf, String)] {
        &self.pack_only
    }

    /// All kinds that have an archive in this family, in `AssetKind::ALL` order. The header
    /// table has none, so nothing packs or builds it. `DynamicHeaders` is listed for
    /// Platinum and HGSS, so `build` packs its folder when it exists.
    pub fn kinds(&self) -> impl Iterator<Item = AssetKind> + '_ {
        AssetKind::ALL
            .into_iter()
            .filter(|kind| self.entries[kind.slot()].is_some())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HeaderSummary {
    pub id: u16,
    pub internal_name: String,
    /// In-game location name from the text archive (empty when the header's name index
    /// lies outside the archive).
    pub location_name: String,
    /// Index of that name in the location text bank (0 is "no place"); the name alone cannot
    /// tell two places of the same name apart.
    pub location_index: u16,
    /// HeartGold/SoulSilver `kanto_flag`: the header lies in Kanto (always `false` in DP and
    /// Platinum).
    pub kanto: bool,
    /// Kind of place the game treats the header as (Platinum and Diamond/Pearl
    /// `location_specifier`, HeartGold/SoulSilver `location_type`): 6 is the Underground.
    pub map_type: u8,
    pub matrix_id: u16,
    pub area_data_id: u8,
    /// Member of the event archive with this header's warps, NPCs, triggers and signs.
    pub event_file_id: u16,
    /// Text archive with this header's messages (the text panel lists which headers use an archive).
    pub text_archive_id: u16,
    /// Member of the script archive with this header's scripts (the world explorer lists it under the header).
    pub script_file_id: u16,
    /// Member of the script archive with this header's level script (0: none).
    pub level_script_id: u16,
    /// The header is a tombstone (3d.4, a text project): the listing leaves it `false`, the
    /// app's `list_headers` sets it from the store.
    pub deleted: bool,
}

/// The header list plus the reason why location names are missing, if they are.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeaderListing {
    pub headers: Vec<HeaderSummary>,
    /// Why every `location_name` is empty: the text archive could not be read. Names are
    /// cosmetic, so this never fails the listing.
    pub names_error: Option<String>,
}

/// Index of the header's name in the location text bank (`u16` in DP, `u8` otherwise).
pub(crate) fn location_name_index(header: &uxie::MapHeader) -> usize {
    match header {
        uxie::MapHeader::DP(h) => usize::from(h.location_name),
        uxie::MapHeader::Pt(h) => usize::from(h.location_name),
        uxie::MapHeader::HGSS(h) => usize::from(h.location_name),
    }
}

fn map_type(header: &uxie::MapHeader) -> u8 {
    match header {
        uxie::MapHeader::DP(h) => h.location_specifier,
        uxie::MapHeader::Pt(h) => h.location_specifier,
        uxie::MapHeader::HGSS(h) => h.location_type,
    }
}

/// The error for asking a store-only kind (one the store never unpacks) for archive data.
fn store_only(kind: AssetKind) -> KernelError {
    KernelError::InvalidProject(format!(
        "{kind} is store-only: it has no member count and is never unpacked"
    ))
}

#[derive(Debug, Deserialize)]
struct HeaderYaml {
    gamecode: String,
}

/// One ARM9 overlay of a ds-rom project (`arm9_overlays/overlays.yaml`): where the game
/// loads it and the file holding its (decompressed) bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlay {
    /// RAM address the overlay is loaded at.
    pub base_address: u32,
    /// Size of the code and data in bytes (the file's length without the bss).
    pub code_size: u32,
    /// `arm9_overlays/ovNNN.bin`, absolute (below the project root).
    pub path: PathBuf,
}

/// `overlays.yaml`: only the keys this crate reads; ds-rom writes more.
#[derive(Debug, Deserialize)]
struct OverlaysYaml {
    #[serde(default)]
    overlays: Vec<OverlayYaml>,
}

#[derive(Debug, Deserialize)]
struct OverlayYaml {
    id: u16,
    base_address: u32,
    code_size: u32,
    #[serde(default)]
    file_name: Option<String>,
}

/// Offset of the header table in the decompressed `arm9.bin` of `game_code` (`UnknownGame`
/// without a known offset).
pub(crate) fn header_table_offset_for(game_code: &str) -> Result<u64, KernelError> {
    let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML)
        .map_err(|e| KernelError::InvalidProject(format!("header_tables.yaml: {e}")))?;
    tables
        .get(game_code)
        .copied()
        .ok_or_else(|| KernelError::UnknownGame(game_code.to_owned()))
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub game_code: String,
    pub family: Family,
    pub language: Language,
    pub archives: ArchiveMap,
    /// The arm9 tables, read once on first use (`data::arm9`).
    pub(crate) arm9_tables: crate::data::arm9::Arm9Cache,
    /// See [`Project::source_mode`].
    source_mode: SourceMode,
    /// What [`Project::asset_location`] reads to find an asset's unit in a text project.
    unit_cache: UnitCache,
}

impl Project {
    /// Opens a ds-rom project folder. Identity comes from the `gamecode` key of `header.yaml`.
    pub fn open(root: impl AsRef<Path>) -> Result<Project, KernelError> {
        let root = root.as_ref().to_path_buf();
        let header_path = root.join("header.yaml");
        let text = match std::fs::read_to_string(&header_path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(KernelError::NotAProject(root));
            }
            Err(e) => return Err(io_at(&header_path)(e)),
        };
        let header: HeaderYaml = serde_yaml::from_str(&text)
            .map_err(|e| KernelError::InvalidProject(format!("{}: {e}", header_path.display())))?;
        let game_code = header.gamecode;
        let family = Family::from_game_code(&game_code)
            .ok_or_else(|| KernelError::UnknownGame(game_code.clone()))?;
        let language = Language::from_game_code(&game_code)
            .ok_or_else(|| KernelError::UnknownGame(game_code.clone()))?;
        let archives = ArchiveMap::for_game(family, language, &game_code)?;
        let source_mode = SourceMode::of(&root);
        Ok(Project {
            root,
            game_code,
            family,
            language,
            archives,
            arm9_tables: Default::default(),
            source_mode,
            unit_cache: UnitCache::default(),
        })
    }

    /// Where the store's assets come from: from `g4-project.json`, read once at
    /// [`Project::open`] (a project opened again sees a changed manifest).
    pub fn source_mode(&self) -> &SourceMode {
        &self.source_mode
    }

    /// `Ok` when the project's assets may be edited (a DSPRE folder or a text project of the
    /// current schema); else [`KernelError::Refused`] with the sentence of the manifest for an
    /// older or newer schema, or the problem of a manifest that does not read.
    pub fn editable(&self) -> Result<(), KernelError> {
        let state = match &self.source_mode {
            SourceMode::Dspre | SourceMode::TextProject => return Ok(()),
            SourceMode::Older { schema } => SchemaState::Older(*schema),
            SourceMode::Newer { schema } => SchemaState::Newer(*schema),
            SourceMode::Broken { reason } => return Err(KernelError::Refused(reason.clone())),
        };
        Err(KernelError::Refused(refusal(state).unwrap_or_else(|| {
            "Das Textprojekt hat eine andere Schema-Version; es lässt sich nur lesen.".to_owned()
        })))
    }

    /// The listings and the item table [`Project::asset_location`] finds units with.
    pub(crate) fn unit_cache(&self) -> &UnitCache {
        &self.unit_cache
    }

    /// `root/unpacked/<name>`
    pub fn unpacked_dir(&self, kind: AssetKind) -> PathBuf {
        self.root
            .join("unpacked")
            .join(self.archives.unpacked_name(kind))
    }

    /// `root/expanded/<name>`: the decoded sources DSPRE keeps next to `unpacked/`. Only the
    /// text archives (`.json`) and the scripts use it.
    pub fn expanded_dir(&self, kind: AssetKind) -> PathBuf {
        self.root
            .join("expanded")
            .join(self.archives.unpacked_name(kind))
    }

    /// `root/unpacked/<name>/NNNN`
    pub fn member_path(&self, kind: AssetKind, index: u16) -> PathBuf {
        self.unpacked_dir(kind).join(format!("{index:04}"))
    }

    /// `root/files/fielddata/maptable/mapname.bin`: one 16-byte NUL-padded internal name per
    /// map header.
    pub fn mapname_path(&self) -> PathBuf {
        self.root.join("files/fielddata/maptable/mapname.bin")
    }

    /// Whether DSPRE's Dynamic Headers patch is applied (`dynamic_headers::is_applied`): the
    /// header table then lives in `unpacked/dynamicHeaders`. Always false for Diamond/Pearl.
    pub fn dynamic_headers_applied(&self) -> Result<bool, KernelError> {
        dynamic_headers::is_applied(self)
    }

    /// Where the store reads and writes asset `index` of `kind`: in a text project
    /// ([`SourceMode::TextProject`]) every kind the store edits that has a unit there (all but
    /// the text archives) is a [`Location::Entity`] around its [`Project::member_location`];
    /// everywhere else the member location itself.
    pub fn asset_location(&self, kind: AssetKind, index: u16) -> Result<Location, KernelError> {
        let members = self.member_location(kind, index)?;
        if self.source_mode != SourceMode::TextProject
            || !crate::store::is_store_kind(kind)
            || kind == AssetKind::TextArchive
        {
            return Ok(members);
        }
        Ok(match unit_of(self, AssetId { kind, index })? {
            Some(unit) => Location::Entity {
                unit,
                members: Box::new(members),
            },
            None => members,
        })
    }

    /// Where the bytes of asset `index` of `kind` live in the members. Archive kinds are whole
    /// files (a text archive is its JSON source and its member, see [`Location::TextSource`]);
    /// the header table is the slice of `arm9/arm9.bin` holding every header (its only asset
    /// has index 0), or, once the Dynamic Headers patch is applied, the files of
    /// `unpacked/dynamicHeaders`. Fails with `InvalidProject` for another index, when
    /// `arm9.bin` is too short for the table or the folder is missing, and with
    /// `UnknownGame` when the game has no known table offset.
    pub fn member_location(&self, kind: AssetKind, index: u16) -> Result<Location, KernelError> {
        if kind == AssetKind::TextArchive {
            return Ok(Location::TextSource {
                json: self.expanded_dir(kind).join(format!("{index:04}.json")),
                member: self.member_path(kind, index),
            });
        }
        if kind == AssetKind::Trainer {
            return Ok(Location::MemberPair {
                props: self.member_path(AssetKind::TrainerProperties, index),
                party: self.member_path(AssetKind::TrainerParty, index),
            });
        }
        if kind != AssetKind::HeaderTable {
            return Ok(Location::Member(self.member_path(kind, index)));
        }
        if index != 0 {
            return Err(KernelError::InvalidProject(format!(
                "the header table has one asset, index {index} is out of range"
            )));
        }
        if self.dynamic_headers_applied()? {
            let dir = dynamic_headers::unpacked_dir(self);
            if !dir.is_dir() {
                return Err(KernelError::InvalidProject(format!(
                    "the Dynamic Headers patch is applied but {} is missing",
                    dir.display()
                )));
            }
            return Ok(Location::HeaderFiles {
                dir,
                count: self.mapname_count()?,
            });
        }
        let (offset, count) = self.header_table_extent()?;
        Ok(Location::Slice {
            path: self.arm9_path(),
            offset,
            len: count * MAP_HEADER_SIZE,
        })
    }

    pub(crate) fn arm9_path(&self) -> PathBuf {
        self.root.join("arm9/arm9.bin")
    }

    /// Offset of the header table in `arm9.bin` (`UnknownGame` without a known offset).
    pub(crate) fn header_table_offset(&self) -> Result<u64, KernelError> {
        header_table_offset_for(&self.game_code)
    }

    /// Number of headers: one per `mapname.bin` record.
    pub(crate) fn mapname_count(&self) -> Result<usize, KernelError> {
        let names_path = self.mapname_path();
        let names_len = std::fs::metadata(&names_path)
            .map_err(io_at(&names_path))?
            .len();
        let count = usize::try_from(names_len)
            .map_err(|_| KernelError::Uxie(format!("mapname.bin too large: {names_len} bytes")))?
            / MAPNAME_RECORD;
        if count > usize::from(u16::MAX) + 1 {
            return Err(KernelError::Uxie(format!("too many map headers: {count}")));
        }
        Ok(count)
    }

    /// Offset of the (unpatched) header table in `arm9.bin` and the number of headers,
    /// checked against the size of `arm9.bin`.
    fn header_table_extent(&self) -> Result<(u64, usize), KernelError> {
        let offset = self.header_table_offset()?;
        let count = self.mapname_count()?;
        let arm9_path = self.arm9_path();
        let arm9_len = std::fs::metadata(&arm9_path)
            .map_err(io_at(&arm9_path))?
            .len();
        let end = offset + (count * MAP_HEADER_SIZE) as u64;
        if arm9_len < end {
            return Err(KernelError::InvalidProject(format!(
                "{} has {arm9_len} bytes, the header table needs {end}",
                arm9_path.display()
            )));
        }
        Ok((offset, count))
    }

    /// Overlay `id` from `arm9_overlays/overlays.yaml`. ds-rom keeps overlays decompressed on
    /// disk (the file's `compressed` flag describes the ROM), so `path` can be read directly.
    pub fn overlay(&self, id: u16) -> Result<Overlay, KernelError> {
        let dir = self.root.join("arm9_overlays");
        let yaml_path = dir.join("overlays.yaml");
        let text = std::fs::read_to_string(&yaml_path).map_err(io_at(&yaml_path))?;
        let parsed: OverlaysYaml = serde_yaml::from_str(&text)
            .map_err(|e| KernelError::InvalidProject(format!("{}: {e}", yaml_path.display())))?;
        let entry = parsed
            .overlays
            .into_iter()
            .find(|o| o.id == id)
            .ok_or_else(|| {
                KernelError::InvalidProject(format!("{}: no overlay {id}", yaml_path.display()))
            })?;
        let file_name = entry.file_name.unwrap_or_else(|| format!("ov{id:03}.bin"));
        Ok(Overlay {
            base_address: entry.base_address,
            code_size: entry.code_size,
            path: dir.join(file_name),
        })
    }

    /// Whether the archive of `kind` exists in this project's game family.
    pub fn has_archive(&self, kind: AssetKind) -> bool {
        self.archives.packed(kind).is_some()
    }

    /// Packed path of `kind`, or `InvalidProject` when the family has no such archive.
    fn require_packed(&self, kind: AssetKind) -> Result<&Path, KernelError> {
        self.archives.packed(kind).ok_or_else(|| {
            KernelError::InvalidProject(format!(
                "game {} has no {kind} archive (family {:?})",
                self.game_code, self.family
            ))
        })
    }

    pub(crate) fn read_packed(&self, kind: AssetKind) -> Result<Narc, KernelError> {
        let path = self.root.join(self.require_packed(kind)?);
        let bytes = crate::fsutil::read_noted(&path).map_err(io_at(&path))?;
        Ok(Narc::read(&bytes)?)
    }

    fn count_unpacked(dir: &Path) -> Result<usize, KernelError> {
        let mut n = 0;
        for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
            let entry = entry.map_err(io_at(dir))?;
            let name = entry.file_name();
            let is_member = name
                .to_str()
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
            if is_member && entry.file_type().map_err(io_at(&entry.path()))?.is_file() {
                n += 1;
            }
        }
        Ok(n)
    }

    /// The kinds [`Project::unpack_every_kind`] unpacks: every kind of the archive map but the
    /// Dynamic Headers container (store-only: its folder belongs to the patch, unpacking it
    /// would fake the patch).
    pub(crate) fn unpackable_kinds(&self) -> impl Iterator<Item = AssetKind> + '_ {
        self.archives
            .kinds()
            .filter(|kind| *kind != AssetKind::DynamicHeaders)
    }

    /// Unpacks every archive of the project the Studio knows ([`Project::ensure_unpacked`]
    /// per kind of [`Project::unpackable_kinds`]); a kind with neither its archive nor its
    /// folder on disk is skipped. "Git einrichten" in a DSPRE folder runs it first, so the
    /// `.gitignore` lists every archive the build packs.
    pub(crate) fn unpack_every_kind(&self) -> Result<(), KernelError> {
        let kinds: Vec<AssetKind> = self.unpackable_kinds().collect();
        for kind in kinds {
            let packed = self
                .archives
                .packed(kind)
                .is_some_and(|p| self.root.join(p).is_file());
            let folder = self
                .root
                .join("unpacked")
                .join(self.archives.unpacked_name(kind))
                .is_dir();
            if packed || folder {
                self.ensure_unpacked(kind)?;
            }
        }
        Ok(())
    }

    /// Unpacks the archive of `kind` if its unpacked folder is missing and returns the
    /// member count. An existing folder is never touched.
    pub fn ensure_unpacked(&self, kind: AssetKind) -> Result<usize, KernelError> {
        match kind {
            // Store-only: nothing to unpack, the table is one asset.
            AssetKind::HeaderTable => return Ok(1),
            // Both archives of the pair; a trainer needs a member in each.
            AssetKind::Trainer => {
                let props = self.ensure_unpacked(AssetKind::TrainerProperties)?;
                return Ok(props.min(self.ensure_unpacked(AssetKind::TrainerParty)?));
            }
            // The folder belongs to the Dynamic Headers patch: it exists once the patch is
            // applied (and in DSPRE projects, holding the NARC's vanilla members), and
            // unpacking it would fake that patch.
            AssetKind::DynamicHeaders => return Err(store_only(kind)),
            _ => {}
        }
        let packed = self.require_packed(kind)?.to_path_buf();
        self.unpack_archive(&packed, self.archives.unpacked_name(kind))
    }

    /// Unpacks the NARC at `packed` (relative to the root) into `unpacked/<unpacked_name>` if
    /// that folder is missing and returns the member count. An existing folder is never
    /// touched. Unlike [`Project::ensure_unpacked`] it takes any archive, also the pack
    /// entries and the Dynamic Headers NARC.
    pub(crate) fn unpack_archive(
        &self,
        packed: &Path,
        unpacked_name: &str,
    ) -> Result<usize, KernelError> {
        let dir = self.root.join("unpacked").join(unpacked_name);
        if dir.is_dir() {
            return Self::count_unpacked(&dir);
        }
        let path = self.root.join(packed);
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        let narc = Narc::read(&bytes)?;
        let parent = self.root.join("unpacked");
        std::fs::create_dir_all(&parent).map_err(io_at(&parent))?;
        // Write into a uniquely named sibling folder and rename, so an interrupted unpack
        // never leaves a partial folder that would later be mistaken for a complete one, and
        // overlapping callers never share a temp folder.
        static UNPACK_SEQ: AtomicU64 = AtomicU64::new(0);
        let tmp = parent.join(format!(
            ".{unpacked_name}.unpacking.{}.{}",
            std::process::id(),
            UNPACK_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let result = Self::write_members(&tmp, &narc)
            .and_then(|()| std::fs::rename(&tmp, &dir).map_err(io_at(&dir)));
        match result {
            Ok(()) => Ok(narc.members.len()),
            Err(e) => {
                // Best effort: the temp folder is ours alone, a leftover is harmless but untidy.
                let _ = std::fs::remove_dir_all(&tmp);
                // A concurrent caller won the race: its complete folder is the result.
                if dir.is_dir() {
                    Self::count_unpacked(&dir)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn write_members(tmp: &Path, narc: &Narc) -> Result<(), KernelError> {
        std::fs::create_dir(tmp).map_err(io_at(tmp))?;
        for (i, member) in narc.members.iter().enumerate() {
            let path = tmp.join(format!("{i:04}"));
            std::fs::write(&path, member).map_err(io_at(&path))?;
        }
        Ok(())
    }

    /// Member count of the archive of `kind`, without unpacking anything.
    pub fn member_count(&self, kind: AssetKind) -> Result<usize, KernelError> {
        match kind {
            AssetKind::HeaderTable => return Ok(1),
            AssetKind::DynamicHeaders => return Err(store_only(kind)),
            AssetKind::Trainer => {
                let props = self.member_count(AssetKind::TrainerProperties)?;
                return Ok(props.min(self.member_count(AssetKind::TrainerParty)?));
            }
            _ => {}
        }
        self.require_packed(kind)?;
        let dir = self.unpacked_dir(kind);
        if dir.is_dir() {
            Self::count_unpacked(&dir)
        } else {
            Ok(self.read_packed(kind)?.members.len())
        }
    }

    /// Index of the map texture set that `AreaData[area_data_id]` selects with its
    /// `map_tileset`. `Ok(None)` when the area data index lies outside its archive.
    pub fn map_tileset_index(&self, area_data_id: u8) -> Result<Option<u16>, KernelError> {
        Ok(self
            .area_data_on_disk(area_data_id)?
            .map(|a| a.map_tileset()))
    }

    /// Area data record `area_data_id` as saved on disk; `Ok(None)` when the index lies
    /// outside its archive.
    fn area_data_on_disk(&self, area_data_id: u8) -> Result<Option<AreaData>, KernelError> {
        let area_count = self.ensure_unpacked(AssetKind::AreaData)?;
        if usize::from(area_data_id) >= area_count {
            return Ok(None);
        }
        let area_path = self.member_path(AssetKind::AreaData, u16::from(area_data_id));
        let area_bytes = std::fs::read(&area_path).map_err(io_at(&area_path))?;
        Ok(Some(AreaData::read(&area_bytes, self.family)?))
    }

    /// Bytes of map texture set `tileset` (NSBTX) from the unpacked archive on disk, or
    /// `Ok(None)` when the index lies outside the archive.
    pub fn map_tileset_member(&self, tileset: u16) -> Result<Option<Vec<u8>>, KernelError> {
        let tileset_count = self.ensure_unpacked(AssetKind::MapTexture)?;
        if usize::from(tileset) >= tileset_count {
            return Ok(None);
        }
        let path = self.member_path(AssetKind::MapTexture, tileset);
        std::fs::read(&path).map(Some).map_err(io_at(&path))
    }

    /// Bytes of the map texture set that `AreaData[area_data_id]` selects, from the unpacked
    /// archives on disk. `Ok(None)` when the area data index or the tileset index lies outside
    /// its archive: the map then simply has no textures.
    pub fn map_tileset_bytes(&self, area_data_id: u8) -> Result<Option<Vec<u8>>, KernelError> {
        match self.map_tileset_index(area_data_id)? {
            Some(tileset) => self.map_tileset_member(tileset),
            None => Ok(None),
        }
    }

    /// Bytes of building texture set `tileset` (NSBTX) from the unpacked archive on disk, or
    /// `Ok(None)` when the index lies outside the archive.
    pub fn building_tileset_member(&self, tileset: u16) -> Result<Option<Vec<u8>>, KernelError> {
        self.read_member_if_present(AssetKind::BuildingTexture, tileset)
    }

    /// Bytes of the building texture set that `AreaData[area_data_id]` selects with its
    /// `buildings_tileset`, from the unpacked archives on disk. `Ok(None)` when the area data
    /// index or the tileset index lies outside its archive.
    pub fn building_tileset_bytes(&self, area_data_id: u8) -> Result<Option<Vec<u8>>, KernelError> {
        match self.area_data_on_disk(area_data_id)? {
            Some(area) => self.building_tileset_member(area.buildings_tileset()),
            None => Ok(None),
        }
    }

    /// NSBMD bytes of building model `model_id`, or `Ok(None)` when the index lies outside the
    /// archive. `interior` selects the interior archive, but only where the game has one (HGSS);
    /// everywhere else the one building archive is used.
    pub fn building_model_bytes(
        &self,
        model_id: u32,
        interior: bool,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let kind = if interior && self.has_archive(AssetKind::InteriorBuildingModel) {
            AssetKind::InteriorBuildingModel
        } else {
            AssetKind::BuildingModel
        };
        match u16::try_from(model_id) {
            Ok(index) => self.read_member_if_present(kind, index),
            Err(_) => Ok(None),
        }
    }

    /// Member `index` of the unpacked archive of `kind`, `Ok(None)` past its last member.
    fn read_member_if_present(
        &self,
        kind: AssetKind,
        index: u16,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        let count = self.ensure_unpacked(kind)?;
        if usize::from(index) >= count {
            return Ok(None);
        }
        let path = self.member_path(kind, index);
        crate::fsutil::read_noted(&path)
            .map(Some)
            .map_err(io_at(&path))
    }

    pub(crate) fn uxie_family(&self) -> uxie::GameFamily {
        match self.family {
            Family::Dp => uxie::GameFamily::DP,
            Family::Pt => uxie::GameFamily::Platinum,
            Family::Hgss => uxie::GameFamily::HGSS,
        }
    }

    pub(crate) fn uxie_language(&self) -> uxie::GameLanguage {
        match self.language {
            Language::English => uxie::GameLanguage::English,
            Language::French => uxie::GameLanguage::French,
            Language::German => uxie::GameLanguage::German,
            Language::Italian => uxie::GameLanguage::Italian,
            Language::Spanish => uxie::GameLanguage::Spanish,
            Language::Japanese => uxie::GameLanguage::Japanese,
            Language::Korean => uxie::GameLanguage::Korean,
        }
    }

    /// In-game location names, indexed by the `location_name` of a map header. They come
    /// from the location text bank of the text archive, which is unpacked on demand.
    pub fn location_names(&self) -> Result<Vec<String>, KernelError> {
        let bank = crate::places::location_bank(self);
        let count = self.ensure_unpacked(AssetKind::TextArchive)?;
        if usize::from(bank) >= count {
            return Err(KernelError::InvalidProject(format!(
                "text archive has {count} members, location names are in member {bank}"
            )));
        }
        let path = self.member_path(AssetKind::TextArchive, bank);
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        // The member stays the source of the names: they are read as the game stores them.
        TextArchiveDoc::from_member(&bytes, self.language.locale_key())
            .map(|doc| doc.messages.into_iter().map(|m| m.text).collect())
            .map_err(|e| KernelError::Uxie(format!("{}: {e}", path.display())))
    }

    /// Reads the header table as it is on disk: from `arm9/arm9.bin`, or from the files of
    /// `unpacked/dynamicHeaders` once the Dynamic Headers patch is applied.
    pub fn read_headers(&self) -> Result<Vec<uxie::MapHeader>, KernelError> {
        self.headers_from_table_bytes(&self.read_header_table_bytes()?)
    }

    /// The headers of the header table `table` (24 bytes each, a trailing rest ignored) in
    /// this project's layout: the table on disk, or one read elsewhere (a commit's
    /// `arm9.bin` slice, the base ROM).
    pub(crate) fn headers_from_table_bytes(
        &self,
        table: &[u8],
    ) -> Result<Vec<uxie::MapHeader>, KernelError> {
        table
            .as_chunks::<MAP_HEADER_SIZE>()
            .0
            .iter()
            .map(|chunk| uxie::map_header::read_map_header_from_bytes(chunk, self.uxie_family()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KernelError::Uxie(format!("{e}")))
    }

    /// The bytes of the header table as it is on disk, 24 per header: the slice of
    /// `arm9/arm9.bin`, or the files of `unpacked/dynamicHeaders` concatenated once the
    /// Dynamic Headers patch is applied.
    pub(crate) fn read_header_table_bytes(&self) -> Result<Vec<u8>, KernelError> {
        match self.member_location(AssetKind::HeaderTable, 0)? {
            Location::HeaderFiles { dir, count } => dynamic_headers::read_files(&dir, count)?
                .ok_or_else(|| dynamic_headers::missing_file_error(&dir, count)),
            Location::Slice { path, offset, len } => {
                let mut file = std::fs::File::open(&path).map_err(io_at(&path))?;
                file.seek(SeekFrom::Start(offset)).map_err(io_at(&path))?;
                let mut bytes = vec![0u8; len];
                file.read_exact(&mut bytes).map_err(io_at(&path))?;
                Ok(bytes)
            }
            Location::Member(_)
            | Location::TextSource { .. }
            | Location::MemberPair { .. }
            | Location::Entity { .. } => Err(KernelError::InvalidProject(
                "the header table is not a member file".to_owned(),
            )),
        }
    }

    /// Number of map headers: the `mapname.bin` record count, provided the table can hold
    /// that many (`arm9.bin` is large enough, or `unpacked/dynamicHeaders` exists). Parses
    /// no header and never reads the text archive.
    pub fn header_count(&self) -> Result<usize, KernelError> {
        match self.member_location(AssetKind::HeaderTable, 0)? {
            Location::HeaderFiles { count, .. } => Ok(count),
            Location::Slice { len, .. } => Ok(len / MAP_HEADER_SIZE),
            Location::Member(_)
            | Location::TextSource { .. }
            | Location::MemberPair { .. }
            | Location::Entity { .. } => Err(KernelError::InvalidProject(
                "the header table is not a member file".to_owned(),
            )),
        }
    }

    /// Lists all map headers with their location names. A failure of the header table is
    /// an error; names that cannot be read leave every `location_name` empty and are
    /// reported in `names_error`.
    pub fn header_listing(&self) -> Result<HeaderListing, KernelError> {
        self.header_listing_for(&self.read_headers()?)
    }

    /// Lists `headers` (for example the store's edited table, which may differ from the
    /// one on disk) with the location names of the bank on disk; see
    /// [`Project::header_listing_with`].
    pub fn header_listing_for(
        &self,
        headers: &[uxie::MapHeader],
    ) -> Result<HeaderListing, KernelError> {
        self.header_listing_with(headers, self.location_names().map_err(|e| e.to_string()))
    }

    /// The internal names of the headers, one per whole record of
    /// `files/fielddata/maptable/mapname.bin` (NUL padded, read lossily as UTF-8).
    pub fn internal_names(&self) -> Result<Vec<String>, KernelError> {
        let names_path = self.mapname_path();
        let mapnames = crate::fsutil::read_noted(&names_path).map_err(io_at(&names_path))?;
        Ok(mapnames
            .as_chunks::<MAPNAME_RECORD>()
            .0
            .iter()
            .map(|record| {
                let end = record.iter().position(|&b| b == 0).unwrap_or(record.len());
                String::from_utf8_lossy(&record[..end]).into_owned()
            })
            .collect())
    }

    /// Lists `headers` with the internal names from `files/fielddata/maptable/mapname.bin`
    /// and the location names (`names`, for example [`crate::places::location_names`] of
    /// the store, unsaved names included) their `location_name` indexes select. An index
    /// outside the names gives an empty name; so does every header when the names could not
    /// be read (the error is reported in `names_error`).
    pub fn header_listing_with(
        &self,
        headers: &[uxie::MapHeader],
        names: Result<Vec<String>, String>,
    ) -> Result<HeaderListing, KernelError> {
        let internal_names = self.internal_names()?;
        let (locations, names_error) = match names {
            Ok(names) => (names, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        let headers = headers
            .iter()
            .zip(internal_names)
            .enumerate()
            .map(|(i, (header, internal_name))| {
                HeaderSummary {
                    // The table holds at most 65536 headers (checked when it is read), so `i` fits in u16.
                    id: i as u16,
                    internal_name,
                    location_name: locations
                        .get(location_name_index(header))
                        .cloned()
                        .unwrap_or_default(),
                    // `location_name_index` is at most `u16::MAX` (the DP index is a `u16`).
                    location_index: location_name_index(header) as u16,
                    kanto: matches!(header, uxie::MapHeader::HGSS(h) if h.kanto_flag),
                    map_type: map_type(header),
                    matrix_id: header.matrix_id(),
                    area_data_id: header.area_data_id(),
                    event_file_id: header.event_file_id(),
                    text_archive_id: header.text_archive_id(),
                    script_file_id: header.script_file_id(),
                    level_script_id: header.level_script_id(),
                    deleted: false,
                }
            })
            .collect();
        Ok(HeaderListing {
            headers,
            names_error,
        })
    }

    /// Like [`Project::header_listing`], without the names error.
    pub fn headers(&self) -> Result<Vec<HeaderSummary>, KernelError> {
        self.header_listing().map(|l| l.headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_metadata_parses_and_resolves_every_family() {
        for code in ["ADAE", "APAE", "ADAJ", "CPUD", "IPKE", "IPGD"] {
            let family = Family::from_game_code(code).unwrap();
            let language = Language::from_game_code(code).unwrap();
            let map = ArchiveMap::for_game(family, language, code).unwrap();
            // Only HGSS has interior building models and only Platinum and HGSS the
            // Dynamic Headers NARC; every other archive kind exists everywhere, the ten data
            // kinds, the two icon kinds and the three battle sprite kinds included. The header
            // table is store-only and never has an archive.
            let expected = match family {
                Family::Dp => 9 + 15,
                Family::Pt => 10 + 15,
                Family::Hgss => 11 + 15,
            };
            assert_eq!(map.kinds().count(), expected, "{code}");
            for kind in AssetKind::ALL {
                let exists = map.packed(kind).is_some();
                assert_eq!(
                    exists,
                    match kind {
                        AssetKind::HeaderTable | AssetKind::Trainer => false,
                        AssetKind::InteriorBuildingModel => family == Family::Hgss,
                        AssetKind::DynamicHeaders => family != Family::Dp,
                        _ => true,
                    }
                );
                assert!(!map.unpacked_name(kind).is_empty());
            }
        }
        // Paths of single games (`by_game`, by the first three characters of the code).
        let packed = |code: &str, kind: AssetKind| {
            let family = Family::from_game_code(code).unwrap();
            let language = Language::from_game_code(code).unwrap();
            let map = ArchiveMap::for_game(family, language, code).unwrap();
            map.packed(kind).unwrap().to_str().unwrap().to_owned()
        };
        assert_eq!(packed("IPKE", AssetKind::Encounter), "files/a/0/3/7");
        assert_eq!(packed("IPGE", AssetKind::Encounter), "files/a/1/3/6");
        assert_eq!(packed("IPGJ", AssetKind::Encounter), "files/a/1/3/6");
        assert_eq!(packed("IPGE", AssetKind::Personal), "files/a/0/0/2");
        assert_eq!(
            packed("ADAE", AssetKind::Encounter),
            "files/fielddata/encountdata/d_enc_data.narc"
        );
        assert_eq!(
            packed("APAE", AssetKind::Encounter),
            "files/fielddata/encountdata/p_enc_data.narc"
        );
        assert_eq!(
            packed("APAD", AssetKind::Personal),
            "files/poketool/personal_pearl/personal.narc"
        );
        assert_eq!(
            packed("ADAD", AssetKind::Personal),
            "files/poketool/personal/personal.narc"
        );
        assert_eq!(
            packed("CPUD", AssetKind::Personal),
            "files/poketool/personal/pl_personal.narc"
        );
        assert_eq!(
            packed("CPUD", AssetKind::Item),
            "files/itemtool/itemdata/pl_item_data.narc"
        );
        assert_eq!(
            packed("IPKD", AssetKind::TrainerTextOffsets),
            "files/a/1/3/1"
        );
        // `by_game` wins over `packed_jp`; a Japanese Diamond still gets its `packed_jp` maps.
        assert_eq!(
            packed("APAJ", AssetKind::Encounter),
            "files/fielddata/encountdata/p_enc_data.narc"
        );
        assert_eq!(
            packed("ADAJ", AssetKind::Map),
            "files/fielddata/land_data/land_data.narc"
        );
        let tables: HashMap<String, u64> = serde_yaml::from_str(HEADER_TABLES_YAML).unwrap();
        assert_eq!(tables["CPUE"], 0xE601C);
        assert_eq!(tables["IPKD"], 0xF6B94);
        assert_eq!(tables["IPGJ"], 0xF6390);
        assert_eq!(tables.len(), 30);
    }
}
