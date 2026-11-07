//! The record of each world map's last bake (phase 6.3b), section `world` of
//! `.g4/build/state.json` ([`WorldRecord`]): the xxh3 of what the bake read ([`input_hash`]) and
//! of the member it wrote ([`member_hash`]). The build step `world`, a save, the import and a
//! conflict resolution write it ([`record_world`]); [`world_side`] reads it to tell a changed
//! world (bake again) from a member changed outside the Studio (`world.member-changed`).

use std::path::PathBuf;

use g4_formats::bdhc::GENERATOR_REV;
use xxhash_rust::xxh3::{Xxh3, xxh3_64};

use super::bake::WORLD_BAKE_REV;
use super::freeform::side_file_rels;
use crate::bake::{AreaStyle, BAKE_REV};
use crate::build::state::{BuildState, WorldRecord};
use crate::error::{KernelError, io_at};
use crate::map_import::map_areas;
use crate::project::{AssetKind, Project};
use crate::relpath::slash;
use crate::store::Store;
use crate::textproject::Art;

/// „world#<WORLD_BAKE_REV>+bake#<BAKE_REV>+bdhc#<GENERATOR_REV>“: the revisions of everything
/// that turns a world into its member. Salts the input hash and the build step `world`.
pub fn world_salt() -> String {
    format!("world#{WORLD_BAKE_REV}+bake#{BAKE_REV}+bdhc#{GENERATOR_REV}")
}

/// `data/maps/NNNN.json` of map `map`, relative to the root.
pub(crate) fn map_json_rel(map: u16) -> PathBuf {
    Art::Maps.dir().join(format!("{map:04}.json"))
}

/// The bytes of `path`, `None` when it is missing.
fn read_optional(path: &std::path::Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(path)(e)),
    }
}

/// Feeds one tagged, length-prefixed piece (or its absence) into `h`.
fn feed(h: &mut Xxh3, tag: &str, bytes: Option<&[u8]>) {
    h.update(tag.as_bytes());
    match bytes {
        Some(bytes) => {
            h.update(&[1]);
            h.update(&(bytes.len() as u64).to_le_bytes());
            h.update(bytes);
        }
        None => h.update(&[0]),
    }
}

/// xxh3 of what baking `map` reads (salt, family, map JSON, resolved verbatim bytes, tileset file
/// or package bytes, package bytes, area id and style, then each side file the bake reads with its
/// path: the free-form models, then the models of the pieces the map places, bytes or missing);
/// `None` when the map is no world map.
///
/// Everything comes from disk as the bake of a fresh store reads it: the JSON as it is on disk
/// (not the store's unsaved world), the tileset file (without one the tileset is the package's
/// textures, which the package bytes cover), the package member, and the first area of the map
/// ([`map_areas`]) with its style. A JSON that does not read or has derived keys is
/// [`KernelError::Compile`], as [`Store::world_map_source`] gives it.
pub fn input_hash(store: &mut Store, map: u16) -> Result<Option<String>, KernelError> {
    let Some(source) = store.world_map_source(map)? else {
        return Ok(None);
    };
    let package = source.world.tileset;
    // The side files of the pieces need the tileset; without pieces none is read. A tileset that
    // does not read fails the bake, whose input its file bytes already are.
    let tileset = match source.world.pieces.is_empty() {
        true => None,
        false => store.tileset(package).ok(),
    };
    let sides = side_file_rels(map, &source.world, tileset.as_deref());
    let project = store.project();
    let root = project.root.clone();
    let family = project.family;
    let json = read_optional(&root.join(map_json_rel(map)))?;
    let tileset = read_optional(&root.join(super::tileset_rel(package)))?;
    let pack = if project.has_archive(AssetKind::MapTexture) {
        project.map_tileset_member(package)?
    } else {
        None
    };
    let area = match map_areas(store, map)?.first() {
        Some(&id) => {
            let style = store
                .area_data_checked(id)?
                .map(|data| AreaStyle::of(&data));
            Some(format!("{id}:{style:?}"))
        }
        None => None,
    };

    let mut h = Xxh3::new();
    feed(&mut h, "salt", Some(world_salt().as_bytes()));
    feed(&mut h, "family", Some(format!("{family:?}").as_bytes()));
    feed(&mut h, "json", json.as_deref());
    feed(&mut h, "model", source.verbatim.model.as_deref());
    feed(&mut h, "bdhc", source.verbatim.bdhc.as_deref());
    feed(&mut h, "tileset", tileset.as_deref());
    feed(&mut h, "package", pack.as_deref());
    feed(&mut h, "area", area.as_deref().map(str::as_bytes));
    for rel in &sides {
        let bytes = read_optional(&root.join(rel))?;
        feed(&mut h, &format!("side {}", slash(rel)), bytes.as_deref());
    }
    Ok(Some(format!("{:016x}", h.digest())))
}

/// xxh3 of a member's bytes, 16 hex digits.
pub fn member_hash(bytes: &[u8]) -> String {
    format!("{:016x}", xxh3_64(bytes))
}

/// Where a world map stands against its record, see [`world_side`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSide {
    /// Input and member as recorded.
    InStep,
    /// The input changed (or the member is gone); the member is as recorded.
    SourceChanged,
    /// The member changed outside the Studio; the input is as recorded.
    MemberChanged,
    /// Both changed.
    Both,
    /// No record: the world applies.
    Unrecorded,
}

/// Against the record of `map`: input hash and member hash equal → InStep; input differs →
/// SourceChanged; member differs → MemberChanged; both → Both; no record → Unrecorded. A
/// **missing member** counts as SourceChanged (an output that is gone is baked again, like the
/// build's `removed`), never as MemberChanged. `None` when the map is no world map.
pub fn world_side(store: &mut Store, map: u16) -> Result<Option<WorldSide>, KernelError> {
    Ok(side_and_input(store, map)?.map(|(side, _)| side))
}

/// [`world_side`] with the input hash it compared.
pub(crate) fn side_and_input(
    store: &mut Store,
    map: u16,
) -> Result<Option<(WorldSide, String)>, KernelError> {
    let Some(input) = input_hash(store, map)? else {
        return Ok(None);
    };
    let project = store.project();
    let Some(record) = BuildState::load_shared(&project.root)
        .world
        .get(&map)
        .cloned()
    else {
        return Ok(Some((WorldSide::Unrecorded, input)));
    };
    let Some(member) = read_optional(&project.member_path(AssetKind::Map, map))? else {
        return Ok(Some((WorldSide::SourceChanged, input)));
    };
    let source = input != record.input;
    let outside = member_hash(&member) != record.member;
    let side = match (source, outside) {
        (false, false) => WorldSide::InStep,
        (true, false) => WorldSide::SourceChanged,
        (false, true) => WorldSide::MemberChanged,
        (true, true) => WorldSide::Both,
    };
    Ok(Some((side, input)))
}

/// Writes the records `entries` into `.g4/build/state.json` (one update, other records kept).
pub fn record_world(project: &Project, entries: &[(u16, WorldRecord)]) -> Result<(), KernelError> {
    if entries.is_empty() {
        return Ok(());
    }
    BuildState::update(&project.root, |state| {
        for (map, record) in entries {
            state.world.insert(*map, record.clone());
        }
    })
}

/// Removes the records of `maps` (no longer world maps, or saved with a refused bake: the next
/// build bakes them and reports), leaving the state alone when it holds none of them.
pub(crate) fn forget_world(project: &Project, maps: &[u16]) -> Result<(), KernelError> {
    let state = BuildState::load_shared(&project.root);
    if !maps.iter().any(|map| state.world.contains_key(map)) {
        return Ok(());
    }
    BuildState::update(&project.root, |state| {
        for map in maps {
            state.world.remove(map);
        }
    })
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
