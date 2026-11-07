//! The build step `world` (phase 6.3b) and the bake of world maps into their members
//! ([`bake_world_maps`]): it runs right after `textproject` (whose units leave world maps out),
//! before `rotom`, only in a text project. Each world map is decided by its record
//! ([`world_side`]): a map in step is left alone, a changed world (its JSON, tileset, package
//! or area) or a missing member is baked and written, a member changed outside the Studio is
//! `world.member-changed` and stops the step before anything is written. So the step is
//! incremental per map, whatever the runner hands it. A map whose JSON carries a tombstone is
//! not baked.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;

use super::bake::{Verbatim, bake_world};
use super::check::{WORLD_BAKE_CODE, WORLD_MEMBER_CHANGED_CODE};
use super::keep::world_field;
use super::record::{
    WorldSide, map_json_rel, member_hash, record_world, side_and_input, world_salt,
};
use super::tileset::TILESETS_DIR;
use crate::bake::Level;
use crate::build::state::WorldRecord;
use crate::build::step::{Dirty, Step, StepRun};
use crate::dynamic_headers;
use crate::error::KernelError;
use crate::fsutil::Journal;
use crate::project::{AssetKind, Project};
use crate::store::Store;
use crate::text::{Problem, Severity};
use crate::textproject::Art;
use crate::textproject::arts::numbered;
use crate::textproject::canon::{problem, write_if_changed};
use crate::textproject::sync;
use crate::textproject::tombstone::DELETED_KEY;

/// The step's id, its key in `.g4/build/state.json`.
pub const WORLD_STEP_ID: &str = "world";

/// Which world maps [`bake_world_maps`] decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakeScope {
    All,
    Maps(Vec<u16>),
}

/// How [`bake_world_maps`] treats a member changed outside the Studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BakeMode {
    /// The build step: a changed member is `world.member-changed`.
    Step,
    /// The text project's import: as the step.
    Import,
    /// „Neu backen“ over a changed member: the world wins.
    Rebake,
}

/// What [`bake_world_maps`] did: the maps it baked and wrote (in order), how many were in step
/// and left alone, and its problems (errors: nothing was written).
#[derive(Debug, Clone, Default, Serialize)]
pub struct WorldBuild {
    pub baked: Vec<u16>,
    pub skipped: usize,
    pub problems: Vec<Problem>,
}

/// „Karte 0123: Der Member wurde außerhalb des Studios geändert (DSPRE?). In Technik ›
/// Textprojekt „Neu backen“ oder „Als binäre Karte übernehmen“.“
pub(crate) fn member_changed_text(map: u16) -> String {
    format!(
        "Karte {map:04}: Der Member wurde außerhalb des Studios geändert (DSPRE?). In Technik › \
         Textprojekt „Neu backen“ oder „Als binäre Karte übernehmen“."
    )
}

/// Whether the entity file `bytes` carries the tombstone `"deleted": true`.
fn is_deleted(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| v.get(DELETED_KEY).and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

/// The world maps of the project: every `data/maps/NNNN.json` with `source: "world"` and a
/// `world` that reads, without tombstone, in order.
pub(crate) fn world_maps(project: &Project) -> Result<Vec<u16>, KernelError> {
    let dir = project.root.join(Art::Maps.dir());
    let mut maps = Vec::new();
    for n in numbered(&dir, "json")? {
        let Ok(map) = u16::try_from(n) else { continue };
        let Ok(bytes) = std::fs::read(project.root.join(map_json_rel(map))) else {
            continue;
        };
        if matches!(world_field(&bytes), Ok(Some(_))) && !is_deleted(&bytes) {
            maps.push(map);
        }
    }
    Ok(maps)
}

/// The `world.member-changed` errors of `maps` (those that are world maps without
/// tombstone): each whose member changed outside the Studio since its record (MemberChanged or
/// Both), decided without writing anything; the import asks before it writes.
pub(crate) fn member_changed_problems(
    project: &Project,
    maps: &[u16],
) -> Result<Vec<Problem>, KernelError> {
    let listed = world_maps(project)?;
    let maps: Vec<u16> = maps
        .iter()
        .copied()
        .filter(|m| listed.contains(m))
        .collect();
    let mut problems = Vec::new();
    if maps.is_empty() {
        return Ok(problems);
    }
    let mut store = Store::new(Arc::new(Project::open(&project.root)?));
    for map in maps {
        match side_and_input(&mut store, map) {
            Ok(Some((WorldSide::MemberChanged | WorldSide::Both, _))) => {
                problems.push(member_changed(map));
            }
            Ok(_) | Err(KernelError::Compile(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(problems)
}

/// The `world.member-changed` error of `map`, at its JSON.
fn member_changed(map: u16) -> Problem {
    problem(
        &map_json_rel(map),
        WORLD_MEMBER_CHANGED_CODE,
        member_changed_text(map),
    )
}

/// Opens a store on `project`, decides every world map of `scope` by [`world_side`]
/// (Step/Import: MemberChanged or Both → `world.member-changed` error for that map, nothing
/// written for any map; Rebake: bakes over), bakes the others whose side is not InStep, writes
/// their members and records them. A refused bake is a `world.bake` error. Errors come back in
/// `problems`; nothing is written when one is an error.
///
/// A map of `scope` that is no world map (any more), carries a tombstone, or whose JSON does
/// not read or has derived keys is passed over: the last is the error of the import and of the
/// step `textproject`. The members written are recorded for the text project's sync too, so a
/// save that follows does not take them for DSPRE's.
///
/// [`world_side`]: super::record::world_side
pub fn bake_world_maps(
    project: &Project,
    scope: &BakeScope,
    mode: BakeMode,
) -> Result<WorldBuild, KernelError> {
    let listed = world_maps(project)?;
    let maps: Vec<u16> = match scope {
        BakeScope::All => listed,
        BakeScope::Maps(maps) => maps
            .iter()
            .copied()
            .filter(|m| listed.contains(m))
            .collect(),
    };
    let mut out = WorldBuild::default();
    if maps.is_empty() {
        return Ok(out);
    }
    let mut store = Store::new(Arc::new(Project::open(&project.root)?));
    let family = project.family;
    let mut bakes: Vec<(u16, Vec<u8>, String)> = Vec::new();
    for map in maps {
        let rel = map_json_rel(map);
        let (side, input) = match side_and_input(&mut store, map) {
            Ok(Some(found)) => found,
            Ok(None) | Err(KernelError::Compile(_)) => continue,
            Err(e) => return Err(e),
        };
        match (side, mode) {
            (WorldSide::InStep, _) => {
                out.skipped += 1;
                continue;
            }
            (WorldSide::MemberChanged | WorldSide::Both, BakeMode::Step | BakeMode::Import) => {
                out.problems.push(member_changed(map));
                continue;
            }
            _ => {}
        }
        let Some(world) = store.world(map)?.cloned() else {
            continue;
        };
        let base = store.map(map)?.clone();
        let verbatim = Verbatim::of(&world, &base);
        let baked = match bake_world(&mut store, map, &world, &base, &verbatim) {
            Ok(baked) => baked,
            Err(KernelError::Refused(text)) => {
                out.problems
                    .push(problem(&rel, WORLD_BAKE_CODE, format!("world: {text}")));
                continue;
            }
            Err(e) => return Err(e),
        };
        if !baked.report.ok {
            for m in baked
                .report
                .messages
                .iter()
                .filter(|m| m.level == Level::Error)
            {
                out.problems.push(problem(
                    &rel,
                    WORLD_BAKE_CODE,
                    format!("world: {}: {}", m.code, m.message),
                ));
            }
            continue;
        }
        bakes.push((map, baked.file.write(family), input));
    }
    if out.problems.iter().any(|p| p.severity == Severity::Error) {
        return Ok(out);
    }
    if bakes.is_empty() {
        return Ok(out);
    }
    project.ensure_unpacked(AssetKind::Map)?;
    let journal = Journal::start();
    let mut entries = Vec::new();
    let written: Result<(), KernelError> = bakes.iter().try_for_each(|(map, bytes, input)| {
        let member = relative(project, &project.member_path(AssetKind::Map, *map));
        write_if_changed(&project.root, &member, bytes)?;
        entries.push((
            *map,
            WorldRecord {
                input: input.clone(),
                member: member_hash(bytes),
            },
        ));
        out.baked.push(*map);
        Ok(())
    });
    let journal = journal.finish();
    // What was written is recorded even when a later write failed.
    let recorded = record_world(project, &entries);
    sync::before_record("world");
    let _ = sync::record_known(project, &sync::noted(project, &journal));
    written?;
    recorded?;
    Ok(out)
}

/// `path` relative to the project root (as it is when it lies outside).
fn relative(project: &Project, path: &Path) -> PathBuf {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_path_buf()
}

/// The build step `world`, see the module documentation. The problems of its last run
/// (warnings when it succeeded, everything when it failed with [`KernelError::Compile`]) are in
/// `problems`.
#[derive(Default)]
pub(crate) struct WorldStep {
    pub(crate) problems: RefCell<Vec<Problem>>,
}

impl Step for WorldStep {
    fn id(&self) -> String {
        WORLD_STEP_ID.to_owned()
    }

    fn salt(&self) -> String {
        world_salt()
    }

    /// The map files, the tileset files, the members of the map textures, area data and
    /// matrices, `arm9.bin` (the header table) and the Dynamic Headers files when applied.
    fn inputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        let mut inputs = vec![Art::Maps.dir(), PathBuf::from(TILESETS_DIR)];
        for kind in [
            AssetKind::MapTexture,
            AssetKind::AreaData,
            AssetKind::Matrix,
        ] {
            if p.has_archive(kind) {
                inputs.push(relative(p, &p.unpacked_dir(kind)));
            }
        }
        inputs.push(relative(p, &p.arm9_path()));
        if dynamic_headers::is_applied(p)? {
            inputs.push(relative(p, &dynamic_headers::unpacked_dir(p)));
        }
        Ok(inputs)
    }

    /// The members of the world maps.
    fn outputs(&self, p: &Project) -> Result<Vec<PathBuf>, KernelError> {
        if !p.has_archive(AssetKind::Map) {
            return Ok(Vec::new());
        }
        Ok(world_maps(p)?
            .into_iter()
            .map(|map| relative(p, &p.member_path(AssetKind::Map, map)))
            .collect())
    }

    fn run(&self, p: &Project, _: &Dirty) -> Result<StepRun, KernelError> {
        let total = world_maps(p)?.len();
        let build = bake_world_maps(p, &BakeScope::All, BakeMode::Step)?;
        let failed = build.problems.iter().any(|p| p.severity == Severity::Error);
        *self.problems.borrow_mut() = build.problems.clone();
        if failed {
            return Err(KernelError::Compile(build.problems));
        }
        Ok(StepRun {
            done: u32::try_from(build.baked.len()).unwrap_or(u32::MAX),
            total: u32::try_from(total).unwrap_or(u32::MAX),
            ..StepRun::default()
        })
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
