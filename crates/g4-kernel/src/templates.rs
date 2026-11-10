//! World templates: recipes that reshape a vanilla world. The empty region keeps the start
//! town and the rooms reached from it, empties every other header and fills matrix 0 around
//! the town with one blocked border map; a start script in the town gives the starter. This
//! module reads the recipe and plans it ([`plan_empty_region`], changes nothing); the recipe
//! `recipes::empty_region` applies it as one undo step ([`apply_empty_region`]), its scripts
//! and text archives included. See `docs/formats/templates.md`.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::OnceLock;

use g4_formats::game::Family;
use g4_formats::matrix::MATRIX_EMPTY;
use serde::{Deserialize, Serialize};

pub use crate::recipes::empty_region::apply_empty_region;

use crate::error::{KernelError, guard_panic, io_at};
use crate::places;
use crate::project::{AssetId, Project};
use crate::scripts;
use crate::start::dp_locked;
use crate::store::{ChangeId, Store};
use crate::textproject::ids::{RefInput, Resolver};
use crate::textproject::symbols::SymbolKind;

const TEMPLATES_YAML: &str = include_str!("../metadata/templates.yaml");
/// The start script of each family, with the placeholders `{n}` (script number), `{species}`,
/// `{level}`, `{done_var}` and `{message}`.
const START_PT: &str = include_str!("../metadata/templates/start_pt.rotom");
const START_HGSS: &str = include_str!("../metadata/templates/start_hgss.rotom");

/// The label of the store step [`apply_empty_region`] records.
pub fn region_label() -> String {
    t!("templates.region_label").into_owned()
}

/// Collision byte of a blocked cell; a map whose cells all carry it (with kind 0) is a border
/// map of the empty region.
pub(crate) const BLOCKED: u8 = 0x80;
/// Collision cells of a map (32 × 32).
const MAP_CELLS: usize = 1024;

/// The empty-region recipe of one family, as `templates.yaml` names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegionRecipe {
    /// Internal name (`mapname.bin`) of the start town's header.
    pub start_town: String,
    pub starter_species: u16,
    pub starter_level: u8,
    /// Variable (command database name) the start script sets once it ran.
    pub done_var: String,
    /// The start script's message for German games.
    pub message_de: String,
    /// The start script's message for every other language.
    pub message_en: String,
}

#[derive(Debug, Deserialize)]
struct TemplatesYaml {
    empty_region: EmptyRegionYaml,
}

#[derive(Debug, Deserialize)]
struct EmptyRegionYaml {
    pt: FamilyYaml,
    hgss: FamilyYaml,
    message: MessageYaml,
}

#[derive(Debug, Deserialize)]
struct FamilyYaml {
    /// `MAP_<internal name>`, or the bare internal name (before 4.3b).
    start_town: String,
    starter: StarterYaml,
    done_var: String,
}

impl FamilyYaml {
    /// The internal name of the start town (`MAP_T01` and `T01` are `T01`).
    fn town(&self) -> &str {
        self.start_town
            .strip_prefix("MAP_")
            .unwrap_or(&self.start_town)
    }
}

#[derive(Debug, Deserialize)]
struct StarterYaml {
    /// `SPECIES_TURTWIG`, or the number (before 4.3b).
    species: RefInput,
    level: u8,
}

#[derive(Debug, Deserialize)]
struct MessageYaml {
    de: String,
    en: String,
}

/// A recipe file in the form of `templates.yaml`.
fn parse_templates(text: &str) -> Result<TemplatesYaml, String> {
    serde_yaml::from_str(text).map_err(|e| format!("templates.yaml: {e}"))
}

/// `templates.yaml`, parsed once.
fn templates() -> Result<&'static TemplatesYaml, KernelError> {
    static TABLE: OnceLock<Result<TemplatesYaml, String>> = OnceLock::new();
    TABLE
        .get_or_init(|| parse_templates(TEMPLATES_YAML))
        .as_ref()
        .map_err(|e| KernelError::InvalidProject(e.clone()))
}

#[derive(Debug, Deserialize)]
struct VarEntry {
    id: u16,
}

#[derive(Debug, Deserialize)]
struct VarTable {
    vars: HashMap<String, VarEntry>,
}

/// Number of the variable `name` in the bundled command database of `family`
/// (`scripts::scrcmd_database`); `None` when the database does not name it.
pub(crate) fn variable_id(family: Family, name: &str) -> Option<u16> {
    static TABLES: [OnceLock<Option<VarTable>>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match family {
        Family::Dp => 0,
        Family::Pt => 1,
        Family::Hgss => 2,
    };
    TABLES[slot]
        .get_or_init(|| serde_json::from_str(scripts::scrcmd_database(family).1).ok())
        .as_ref()?
        .vars
        .get(name)
        .map(|v| v.id)
}

/// The empty-region recipe of `family`. Diamond and Pearl are refused with [`dp_locked`]; a
/// `done_var` the command database does not know is a metadata error.
pub fn region_recipe(family: Family) -> Result<RegionRecipe, KernelError> {
    recipe_of(templates()?, family)
}

/// The empty-region recipe of `family` in the recipe file `yaml`: the start town as its
/// internal name, the starter's species by number (an id through the fixed table).
fn recipe_of(yaml: &TemplatesYaml, family: Family) -> Result<RegionRecipe, KernelError> {
    let yaml = &yaml.empty_region;
    let game = match family {
        Family::Dp => return Err(KernelError::Refused(dp_locked())),
        Family::Pt => &yaml.pt,
        Family::Hgss => &yaml.hgss,
    };
    if variable_id(family, &game.done_var).is_none() {
        return Err(KernelError::InvalidProject(format!(
            "templates.yaml: the command database has no variable {}",
            game.done_var
        )));
    }
    let invalid = |why: String| KernelError::InvalidProject(format!("templates.yaml: {why}"));
    let species = Resolver::fixed()
        .number(SymbolKind::Species, &game.starter.species)
        .map_err(invalid)?;
    let starter_species = u16::try_from(species)
        .map_err(|_| invalid(format!("species {species} is no species number")))?;
    Ok(RegionRecipe {
        start_town: game.town().to_owned(),
        starter_species,
        starter_level: game.starter.level,
        done_var: game.done_var.clone(),
        message_de: yaml.message.de.clone(),
        message_en: yaml.message.en.clone(),
    })
}

/// Internal name (`mapname.bin`) of the start town's header in the recipe of `family`, for
/// example `T01` (Platinum) or `T20` (HeartGold/SoulSilver); `None` for Diamond/Pearl, which
/// has no recipe, or when `templates.yaml` cannot be read.
pub fn start_town(family: Family) -> Option<&'static str> {
    let yaml = &templates().ok()?.empty_region;
    match family {
        Family::Dp => None,
        Family::Pt => Some(yaml.pt.town()),
        Family::Hgss => Some(yaml.hgss.town()),
    }
}

/// What the empty-region recipe would do to the store's world (unsaved edits included).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RegionPlan {
    /// The start town's header.
    pub start_header: u16,
    /// The town's in-game location name, for example "Zweiblattdorf" (empty when the text
    /// bank does not name it).
    pub town_name: String,
    /// `(x, y)` of the town's cells in matrix 0, row by row.
    pub town_cells: Vec<(u8, u8)>,
    /// Map of the first town cell: the source of the border map.
    pub town_map: u16,
    /// Altitude of the first town cell, which every border cell gets.
    pub town_altitude: u8,
    pub town_event_file: u16,
    pub town_script_file: u16,
    /// The town and the headers reached from it through warps without entering another
    /// header of matrix 0, ascending.
    pub kept: Vec<u16>,
    /// Headers that get event file 0 and the empty level script, ascending; headers that
    /// already have both are not listed.
    pub emptied: Vec<u16>,
    /// Cells of matrix 0 with a map outside the town: they get the border map.
    pub border_cells: usize,
    /// The level script every emptied header gets.
    pub empty_level_script: u16,
    /// `(event file, warp index)` of the warps of kept event files that lead into a header
    /// that is not kept; they are removed.
    pub dropped_warps: Vec<(u16, u16)>,
    /// Figures (object events) of the town's event file, removed.
    pub removed_objects: usize,
    /// Triggers (coord events) of the town's event file, removed.
    pub removed_triggers: usize,
    /// Species the start script gives (the recipe's starter).
    pub starter_species: u16,
    /// Level of that Pokémon.
    pub starter_level: u8,
    /// The world already looks like the recipe's result: every cell of matrix 0 with a map
    /// carries the town's header, the cells outside the town share one map, and the town has
    /// no figures.
    pub already_empty: bool,
}

/// Plans the empty region on the store's world; reads, never edits (maps, matrix 0, event
/// files and the header table may be loaded). Refused: Diamond/Pearl ([`dp_locked`]), a
/// start town that is no header of matrix 0 or has no cell there, and a project without an
/// empty level script (a script member of exactly four zero bytes).
///
/// Town cells are the cells of matrix 0 with the town's header and a map, except cells whose
/// map is a border map (every collision cell `{kind 0, collision 0x80}`), so a world the recipe
/// already emptied still shows the town apart from its border.
pub fn plan_empty_region(store: &mut Store) -> Result<RegionPlan, KernelError> {
    let recipe = region_recipe(store.project().family)?;
    let headers = store.header_table()?.to_vec();
    // The names come from the store, so a rename not yet saved is the town's name too.
    let names = places::location_names(store).map_err(|e| e.to_string());
    let listing = store
        .project()
        .header_listing_with(&headers, names)?
        .headers;
    let town = listing
        .iter()
        .find(|h| h.internal_name == recipe.start_town && h.matrix_id == 0)
        .ok_or_else(|| {
            KernelError::Refused(format!("start town {} not found", recipe.start_town))
        })?;
    let start_header = town.id;
    let town_name = town.location_name.clone();
    let town_event_file = town.event_file_id;
    let town_script_file = town.script_file_id;

    // Cells of matrix 0: (x, y, map, header) of every cell with a map.
    let matrix = store.matrix(0)?.clone();
    let width = usize::from(matrix.width);
    let cells: Vec<(u8, u8, u16, Option<u16>)> = matrix
        .maps
        .iter()
        .enumerate()
        .filter(|&(_, &map)| map != MATRIX_EMPTY)
        .map(|(i, &map)| {
            let header = matrix.headers.as_ref().and_then(|h| h.get(i).copied());
            // A matrix is at most 255 × 255 cells, so both coordinates fit in u8.
            ((i % width) as u8, (i / width) as u8, map, header)
        })
        .collect();
    let with_town: Vec<_> = cells.iter().filter(|c| c.3 == Some(start_header)).collect();
    if with_town.is_empty() {
        return Err(KernelError::Refused(format!(
            "start town {} has no cell in matrix 0",
            recipe.start_town
        )));
    }
    let mut border_maps = BTreeSet::new();
    for map in with_town.iter().map(|c| c.2).collect::<BTreeSet<_>>() {
        let blocked = is_border_map(&store.map(map)?.permissions);
        if blocked {
            border_maps.insert(map);
        }
    }
    let mut town_part: Vec<_> = with_town
        .iter()
        .filter(|c| !border_maps.contains(&c.2))
        .collect();
    if town_part.is_empty() {
        // Only border maps carry the town: they are the town.
        town_part = with_town.iter().collect();
    }
    let town_cells: Vec<(u8, u8)> = town_part.iter().map(|c| (c.0, c.1)).collect();
    let first = town_part[0];
    let town_map = first.2;
    let town_altitude = matrix
        .altitudes
        .as_ref()
        .and_then(|a| {
            a.get(usize::from(first.1) * width + usize::from(first.0))
                .copied()
        })
        .unwrap_or(0);
    let outside: Vec<_> = cells
        .iter()
        .filter(|c| !town_cells.contains(&(c.0, c.1)))
        .collect();
    let border_cells = outside.len();

    let empty_level_script = empty_level_script(store, &headers)?;

    // Kept: the town plus a breadth-first walk over warps into headers off matrix 0.
    let mut kept = BTreeSet::from([start_header]);
    let mut queue = VecDeque::from([start_header]);
    while let Some(id) = queue.pop_front() {
        let event = headers[usize::from(id)].event_file_id();
        let dests: Vec<u16> = store
            .event(event)?
            .warp_events
            .iter()
            .map(|w| w.dest_header_id)
            .collect();
        for dest in dests {
            let Some(header) = headers.get(usize::from(dest)) else {
                continue;
            };
            if header.matrix_id() != 0 && kept.insert(dest) {
                queue.push_back(dest);
            }
        }
    }

    let emptied: Vec<u16> = (0..headers.len())
        // The table holds at most 65536 headers (checked when it is read).
        .map(|i| i as u16)
        .filter(|id| !kept.contains(id))
        .filter(|&id| {
            let h = &headers[usize::from(id)];
            h.event_file_id() != 0 || h.level_script_id() != empty_level_script
        })
        .collect();

    let kept_events: BTreeSet<u16> = kept
        .iter()
        .map(|&id| headers[usize::from(id)].event_file_id())
        .collect();
    let mut dropped_warps = Vec::new();
    for event in kept_events {
        for (i, warp) in store.event(event)?.warp_events.iter().enumerate() {
            let into_dropped = usize::from(warp.dest_header_id) < headers.len()
                && !kept.contains(&warp.dest_header_id);
            if into_dropped {
                // An event file holds at most u16::MAX warps (its counter is 16 bits wide).
                dropped_warps.push((event, i as u16));
            }
        }
    }

    let town_events = store.event(town_event_file)?;
    let removed_objects = town_events.object_events.len();
    let removed_triggers = town_events.coord_events.len();
    let all_town = cells.iter().all(|c| c.3 == Some(start_header));
    let one_border = outside.windows(2).all(|w| w[0].2 == w[1].2);
    let already_empty = all_town && one_border && removed_objects == 0;

    Ok(RegionPlan {
        start_header,
        town_name,
        town_cells,
        town_map,
        town_altitude,
        town_event_file,
        town_script_file,
        kept: kept.into_iter().collect(),
        emptied,
        border_cells,
        empty_level_script,
        dropped_warps,
        removed_objects,
        removed_triggers,
        starter_species: recipe.starter_species,
        starter_level: recipe.starter_level,
        already_empty,
    })
}

/// Whether a map's collision cells make it a border map: all [`MAP_CELLS`] of them
/// `{kind 0, collision 0x80}` (an empty or short list is none).
fn is_border_map(permissions: &[g4_formats::map::Cell]) -> bool {
    permissions.len() == MAP_CELLS
        && permissions
            .iter()
            .all(|c| c.kind == 0 && c.collision == BLOCKED)
}

/// Whether a level script member is empty: exactly four zero bytes.
fn is_empty_level_script(bytes: &[u8]) -> bool {
    bytes == [0, 0, 0, 0]
}

/// The level script of header 0 when it is empty, else the first empty member of the script
/// archive; refused when there is none.
fn empty_level_script(
    store: &Store,
    headers: &[crate::store::MapHeader],
) -> Result<u16, KernelError> {
    let dir = scripts::ensure_members(store.project())?;
    let read = |n: u16| -> Result<Option<Vec<u8>>, KernelError> {
        let path = dir.join(format!("{n:04}"));
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_at(&path)(e)),
        }
    };
    if let Some(header) = headers.first() {
        let n = header.level_script_id();
        if read(n)?.is_some_and(|b| is_empty_level_script(&b)) {
            return Ok(n);
        }
    }
    for n in scripts::numbered_files(&dir, None)? {
        if read(n)?.is_some_and(|b| is_empty_level_script(&b)) {
            return Ok(n);
        }
    }
    Err(KernelError::Refused(
        "no empty level script (a script member of four zero bytes)".into(),
    ))
}

/// What [`apply_empty_region`] did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RegionReport {
    /// The plan it carried out.
    pub plan: RegionPlan,
    /// The new map of every cell outside the town (written at once, not undoable).
    pub border_map: u16,
    /// The new level script of the town (written at once, not undoable).
    pub level_script: u16,
    /// `(script file, number)` of the start script appended to the town's script file
    /// (written at once, not undoable).
    pub start_script: (u16, u16),
    /// The store step; `None` only when it changed nothing.
    pub change: Option<ChangeId>,
    /// Every asset it changed, in kind and index order: the header table, matrix 0, the
    /// border map and event files of the step, the two script files and the text archives
    /// the compile rewrote (the store loaded them again).
    pub changed: Vec<AssetId>,
}

/// The start script of `family` as script `#n`, filled from `recipe` with `message` as its
/// text literal (a `"` in it is escaped). Diamond and Pearl are refused with [`dp_locked`].
pub fn start_script(
    recipe: &RegionRecipe,
    family: Family,
    n: u16,
    message: &str,
) -> Result<String, KernelError> {
    let template = match family {
        Family::Dp => return Err(KernelError::Refused(dp_locked())),
        Family::Pt => START_PT,
        Family::Hgss => START_HGSS,
    };
    // `{message}` goes last, so a brace in the message is never taken for a placeholder.
    Ok(template
        .replace("{n}", &n.to_string())
        .replace("{species}", &recipe.starter_species.to_string())
        .replace("{level}", &recipe.starter_level.to_string())
        .replace("{done_var}", &recipe.done_var)
        .replace("{message}", &message.replace('"', "\\\"")))
}

/// `message` wrapped like the `text.too-wide` fix does (chatot's `word_wrap` at the text
/// window width, `dialog_line_px`): the game breaks no line by itself, and the recipe's text is
/// one line wider than the window. The `\r` chatot appends at the end is left out.
pub fn wrap_message(message: &str) -> Result<String, KernelError> {
    use chatot::{default_glyph_widths, get_default_charmap, word_wrap};
    let max = Project::limits().dialog_line_px.value;
    // chatot v0.8.1: src/format.rs word_wrap
    let wrapped = guard_panic("chatot", || {
        word_wrap(message, get_default_charmap(), default_glyph_widths(), max)
    })?
    .map_err(|e| KernelError::InvalidProject(format!("start script message: {e}")))?;
    Ok(match wrapped.strip_suffix("\\r") {
        Some(kept) if !message.ends_with("\\r") => kept.to_owned(),
        _ => wrapped,
    })
}

#[cfg(test)]
mod tests {
    use g4_formats::map::Cell;

    use super::*;

    /// Compiles `source` on its own with rotom against the bundled command database of
    /// `family` and the flags and variables it names: an unknown command, a wrong number of
    /// parameters or an unknown name is a compile error. (In a project the names come from
    /// uxie's symbols; rotom's database type leaves the `flags` and `vars` of the JSON out,
    /// so they go in as constants files, `{"<id>": "<name>"}`, one name per id and file.)
    fn compile_with_database(family: Family, source: &str) -> Result<Vec<u8>, String> {
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let (name, json) = scripts::scrcmd_database(family);
        let path = tmp.path().join(name);
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        // rotom v0.7.0: src/database.rs DatabaseV2::load, ConstantDb::new, load_from_db,
        // load_json
        let db = rotom::DatabaseV2::load(&path).map_err(|e| e.to_string())?;
        let mut constants = rotom::ConstantDb::new();
        constants.load_from_db(&db);
        let full: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let mut files: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
        for section in ["flags", "vars"] {
            let entries = full[section].as_object().ok_or(section)?;
            for (symbol, entry) in entries {
                let id = entry["id"].to_string();
                match files.iter_mut().find(|f| !f.contains_key(&id)) {
                    Some(file) => {
                        file.insert(id, symbol.clone().into());
                    }
                    None => files.push(serde_json::Map::from_iter([(id, symbol.clone().into())])),
                }
            }
        }
        for (i, file) in files.into_iter().enumerate() {
            let at = tmp.path().join(format!("symbols{i}.json"));
            std::fs::write(&at, serde_json::Value::Object(file).to_string())
                .map_err(|e| e.to_string())?;
            constants.load_json(&at).map_err(|e| e.to_string())?;
        }
        // rotom v0.7.0: src/lib.rs compile (standalone, no project)
        rotom::compile(source, &db, &constants, rotom::BinaryQuirk::default())
            .map(|compiled| compiled.bytes)
            .map_err(|e| e.to_string())
    }

    /// The start script of `family` filled like `apply_empty_region` fills it (a free slot,
    /// the wrapped message), checked to parse as it is, then with its text literal swapped for
    /// message 0: outside a project rotom cannot lower a literal (it adds it to the map's
    /// text archive), and in one the literal becomes that message's number.
    fn filled_start_script(family: Family, message: &str) -> String {
        let recipe = region_recipe(family).unwrap();
        let wrapped = wrap_message(message).unwrap();
        let source = start_script(&recipe, family, 7, &wrapped).unwrap();
        // rotom v0.7.0: src/compiler/parser.rs Parser::new (strict), parse_script_file
        let parsed = rotom::Parser::new(rotom::Lexer::new(&source)).parse_script_file();
        assert!(parsed.is_ok(), "{family:?}: {parsed:?}\n{source}");
        let literal = format!("\"{}\"", wrapped.replace('"', "\\\""));
        assert_eq!(source.matches(&literal).count(), 1, "{source}");
        source.replace(&literal, "0")
    }

    #[test]
    fn the_start_scripts_compile_against_their_command_database() {
        for family in [Family::Pt, Family::Hgss] {
            let recipe = region_recipe(family).unwrap();
            for message in [&recipe.message_de, &recipe.message_en] {
                let source = filled_start_script(family, message);
                let bytes = compile_with_database(family, &source)
                    .unwrap_or_else(|e| panic!("{family:?}: {e}\n{source}"));
                assert!(!bytes.is_empty(), "{family:?}");
            }
            // The check bites: a command the database does not know, one parameter too many
            // and an unknown variable are refused.
            let source = filled_start_script(family, &recipe.message_de);
            for broken in [
                source.replace("GivePokedex", "GivePokedexTwice"),
                source.replace("GiveRunningShoes", "GiveRunningShoes 1"),
                source.replace("VAR_PLAYER_STARTER", "VAR_NO_SUCH_VARIABLE"),
            ] {
                assert_ne!(broken, source);
                assert!(
                    compile_with_database(family, &broken).is_err(),
                    "{family:?}: {broken}"
                );
            }
        }
    }

    /// A recipe file with `start_town` and `species` as given for Platinum.
    fn recipe_yaml(start_town: &str, species: &str) -> String {
        TEMPLATES_YAML
            .replacen(
                "start_town: MAP_T01",
                &format!("start_town: {start_town}"),
                1,
            )
            .replacen(
                "species: SPECIES_TURTWIG",
                &format!("species: {species}"),
                1,
            )
    }

    #[test]
    fn recipe_reads_ids_and_the_old_numbers() {
        for (town, species) in [
            ("MAP_T01", "SPECIES_TURTWIG"),
            ("T01", "387"),
            ("T01", "SPECIES_0387"),
            ("MAP_T01", "\"387\""),
        ] {
            let yaml = parse_templates(&recipe_yaml(town, species)).unwrap();
            let recipe = recipe_of(&yaml, Family::Pt).unwrap();
            assert_eq!(recipe.start_town, "T01", "{town}");
            assert_eq!(recipe.starter_species, 387, "{species}");
        }
        // An unknown species, or one out of range, is an error of the recipe file.
        for species in ["SPECIES_TURTWIGG", "70000"] {
            let yaml = parse_templates(&recipe_yaml("T01", species)).unwrap();
            match recipe_of(&yaml, Family::Pt) {
                Err(KernelError::InvalidProject(why)) => {
                    assert!(why.starts_with("templates.yaml: "), "{why}");
                    assert!(why.contains(species), "{why}");
                }
                other => panic!("expected an error, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_bundled_recipe_names_ids() {
        for line in [
            "start_town: MAP_T01",
            "start_town: MAP_T20",
            "species: SPECIES_TURTWIG",
            "species: SPECIES_CHIKORITA",
        ] {
            assert!(TEMPLATES_YAML.contains(line), "{line}");
        }
        let pt = region_recipe(Family::Pt).unwrap();
        assert_eq!((pt.start_town.as_str(), pt.starter_species), ("T01", 387));
        let hgss = region_recipe(Family::Hgss).unwrap();
        assert_eq!(
            (hgss.start_town.as_str(), hgss.starter_species),
            ("T20", 152)
        );
        // The internal name, as the header listing and the preview compare it.
        assert_eq!(start_town(Family::Pt), Some("T01"));
        assert_eq!(start_town(Family::Hgss), Some("T20"));
        assert_eq!(start_town(Family::Dp), None);
    }

    #[test]
    fn a_border_map_has_every_cell_blocked() {
        let blocked = Cell {
            kind: 0,
            collision: BLOCKED,
        };
        assert!(is_border_map(&[blocked; MAP_CELLS]));
        // No cells, or too few, is no border map (all of none would be vacuously blocked).
        assert!(!is_border_map(&[]));
        assert!(!is_border_map(&[blocked; MAP_CELLS - 1]));
        let mut one_open = [blocked; MAP_CELLS];
        one_open[5].collision = 0;
        assert!(!is_border_map(&one_open));
        one_open[5] = Cell {
            kind: 1,
            collision: BLOCKED,
        };
        assert!(!is_border_map(&one_open));
    }
}
