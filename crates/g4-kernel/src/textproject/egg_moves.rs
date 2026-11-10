//! The egg moves in the text project: the field `egg_moves` of every species file
//! (`data/species/NNNN.json`), one unit over all of them ([`EGG_MOVES_UNIT_KEY`]) with the
//! table's one member (the NARC member, DSPRE's patch NARC member or the slice of overlay 5,
//! see [`egg_moves_place`]).
//!
//! The table a set of species files stands for is the table at its place with the field of
//! every file applied in ascending species number through [`EggMoves::set_moves`] (which keeps
//! the end and its padding): a species without the field loses its entry, an empty list is a
//! mark without moves, an entry of a species without a file stays. A text project where no
//! species file has the field (exported before it existed) leaves the table alone.
//!
//! [`EGG_MOVES_UNIT_KEY`]: super::source::EGG_MOVES_UNIT_KEY

use std::path::{Path, PathBuf};

use g4_formats::data::egg_moves::{EggMoveEntry, EggMoves, SPECIES_BASE};
use serde::Deserialize;

use crate::data::egg_moves::{
    EggMovesPlace, EggMovesSource, budget_full, egg_moves_place, signature_lost,
};
use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Location, Project};

use super::arts::{Art, numbered};
use super::codes::SCHEMA_CODE;
use super::record::{SymRef, resolve, sym_ref};
use super::registry::Symbols;
use super::source::disk_member;
use super::symbols::SymbolKind;

/// The key of the field in a species file.
pub(crate) const FIELD: &str = "egg_moves";

/// A species file read for its egg moves alone (every other key skipped, unchecked).
#[derive(Deserialize)]
struct EggField {
    #[serde(default)]
    egg_moves: Option<Vec<SymRef>>,
}

/// The field `egg_moves` of species file `bytes` (`None`: not there), read without checking
/// the rest of the file.
pub(crate) fn egg_field(bytes: &[u8]) -> Result<Option<Vec<SymRef>>, serde_json::Error> {
    Ok(serde_json::from_slice::<EggField>(bytes)?.egg_moves)
}

/// The field of species `index` from `table`: its first entry's moves as ids, `None` without
/// an entry.
pub(crate) fn field_of(table: &EggMoves, index: u32, symbols: &Symbols) -> Option<Vec<SymRef>> {
    let species = u16::try_from(index).ok()?;
    table.moves_of(species).map(|moves| {
        moves
            .iter()
            .map(|&m| sym_ref(symbols, SymbolKind::Move, u32::from(m)))
            .collect()
    })
}

/// The move numbers of field `refs`, as `data_edit` takes egg moves: a move from 1 to below
/// `moves` (the number of moves of the project; without it below the species marks). An
/// unknown id is the error `textproject.unknown-id: egg_moves[i]: …`, another number
/// `textproject.schema: egg_moves[i]: …`.
pub(crate) fn resolve_field(
    refs: &[SymRef],
    symbols: &Symbols,
    moves: Option<usize>,
) -> Result<Vec<u16>, String> {
    let limit = moves.map_or(usize::from(SPECIES_BASE), |m| {
        m.min(usize::from(SPECIES_BASE))
    });
    refs.iter()
        .enumerate()
        .map(|(i, r)| {
            let path = format!("{FIELD}[{i}]");
            let n = resolve(symbols, SymbolKind::Move, r, &path)?;
            u16::try_from(n)
                .ok()
                .filter(|&m| m != 0 && usize::from(m) < limit)
                .ok_or_else(|| {
                    t!(
                        "textproject.egg_move_invalid",
                        code = SCHEMA_CODE,
                        path = path,
                        n = n,
                        max = limit.saturating_sub(1)
                    )
                    .into_owned()
                })
        })
        .collect()
}

/// The number of moves of `project` (members of its move archive), for [`resolve_field`].
pub(crate) fn move_count(project: &Project) -> Option<usize> {
    project
        .member_count(AssetKind::Move)
        .ok()
        .filter(|&n| n > 0)
}

/// Sets the entry of `species` to `moves` (`None`: no entry) through [`EggMoves::set_moves`];
/// an empty list is a mark without moves, which `set_moves` would remove, so it goes in at
/// the same place with the same end and padding rule.
pub(crate) fn put_entry(table: &mut EggMoves, species: u16, moves: Option<&[u16]>) {
    match moves {
        None => {
            while table.moves_of(species).is_some() {
                table.set_moves(species, Vec::new());
            }
        }
        Some(moves) if !moves.is_empty() => table.set_moves(species, moves.to_vec()),
        Some(_) => {
            if table.moves_of(species).is_some_and(|m| m.is_empty()) {
                return;
            }
            // Removed first (when it has moves), then the mark alone.
            table.set_moves(species, Vec::new());
            let at = table
                .entries
                .iter()
                .position(|e| e.species > species)
                .unwrap_or(table.entries.len());
            table.entries.insert(
                at,
                EggMoveEntry {
                    species,
                    moves: Vec::new(),
                },
            );
            table.terminated = true;
            table.tail = table.padding();
        }
    }
}

/// `table` with the field of every species in `fields` (ascending numbers) applied, see the
/// module docs.
pub(crate) fn assemble(mut table: EggMoves, fields: &[(u16, Option<Vec<u16>>)]) -> EggMoves {
    for (species, moves) in fields {
        if table.moves_of(*species) != moves.as_deref() {
            put_entry(&mut table, *species, moves.as_deref());
        }
    }
    table
}

/// An empty table of `place` (a member that is not there yet).
pub(crate) fn empty_table(place: &EggMovesPlace) -> EggMoves {
    EggMoves {
        entries: Vec::new(),
        terminated: true,
        tail: Vec::new(),
        budget: place.budget,
    }
}

/// The table at `place` as it is on disk (with the slice's budget): `None` when the member is
/// not there or holds no table.
pub(crate) fn disk_table(
    project: &Project,
    place: &EggMovesPlace,
) -> Result<Option<EggMoves>, KernelError> {
    let Some(bytes) = place_bytes(project, place)? else {
        return Ok(None);
    };
    Ok(EggMoves::read(&bytes).ok().map(|mut table| {
        table.set_budget(place.budget);
        table
    }))
}

/// The table on disk where it has a place, as the export reads it once: `None` when it has
/// none (locked), or its member is missing or holds no table.
pub(crate) fn readable_table(project: &Project) -> Result<Option<EggMoves>, KernelError> {
    match place(project) {
        Some(place) => disk_table(project, &place),
        None => Ok(None),
    }
}

/// The bytes of the table at `place` on disk in the store's form: the member (from the packed
/// NARC while its folder is not unpacked) or the slice; `None` when they are not there.
pub(crate) fn place_bytes(
    project: &Project,
    place: &EggMovesPlace,
) -> Result<Option<Vec<u8>>, KernelError> {
    match &place.location {
        Location::Member(path) if !path.is_file() => disk_member(project, AssetKind::EggMoves, 0),
        location => location_bytes(location),
    }
}

/// The bytes at `location` (a member or a slice); `None` when the file is missing or too short.
/// The file is read noted ([`crate::fsutil::read_noted`]), so an export or import records it.
pub(crate) fn location_bytes(location: &Location) -> Result<Option<Vec<u8>>, KernelError> {
    let (path, slice) = match location {
        Location::Member(path) => (path, None),
        Location::Slice { path, offset, len } => (path, Some((*offset, *len))),
        other => {
            return Err(KernelError::InvalidProject(
                t!(
                    "textproject.egg_moves_location",
                    location = format!("{other:?}")
                )
                .into_owned(),
            ));
        }
    };
    let bytes = match crate::fsutil::read_noted(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(path)(e)),
    };
    Ok(match slice {
        None => Some(bytes),
        Some((offset, len)) => {
            let start = usize::try_from(offset).ok();
            start
                .and_then(|s| bytes.get(s..s.checked_add(len)?))
                .map(<[u8]>::to_vec)
        }
    })
}

/// The bytes of `table` in the store's form; over the budget the sentence of
/// [`budget_full`], a fixed table that would lose its signature that of [`signature_lost`].
pub(crate) fn table_bytes(table: &EggMoves) -> Result<Vec<u8>, String> {
    let bytes = table
        .write()
        .map_err(|_| budget_full(table.budget_words().unwrap_or_default()))?;
    if table.budget.is_some() && !EggMoves::signature_ok(&bytes) {
        return Err(signature_lost());
    }
    Ok(bytes)
}

/// The place of the table when it has one (`None`: locked or no table for the game).
pub(crate) fn place(project: &Project) -> Option<EggMovesPlace> {
    egg_moves_place(project).ok()?.ok()
}

/// The place the import writes the table to: the table's place, but on Platinum with
/// `patch` (the manifest's `eggMovesPatch`: the text project holds DSPRE's patch) the patch
/// NARC's member `unpacked/eggMoves/0000` also when neither the NARC nor its folder is there
/// (a clone): written there, the folder marks the patch again and the build packs the NARC;
/// overlay 5 stays as it is. `Err(reason)` when the table has no place.
pub(crate) fn import_place(
    project: &Project,
    patch: bool,
) -> Result<Result<EggMovesPlace, String>, KernelError> {
    let place = egg_moves_place(project)?;
    if patch
        && project.family == g4_formats::game::Family::Pt
        && place
            .as_ref()
            .map_or(true, |p| p.source != EggMovesSource::Patch)
    {
        return Ok(Ok(EggMovesPlace {
            source: EggMovesSource::Patch,
            location: Location::Member(project.member_path(AssetKind::EggMoves, 0)),
            budget: None,
        }));
    }
    Ok(place)
}

/// Whether the table lives in DSPRE's patch NARC.
pub(crate) fn is_patch(project: &Project) -> bool {
    place(project).is_some_and(|p| p.source == EggMovesSource::Patch)
}

/// The overlay slice of the table, relative to the root (file, offset, length): `None` unless
/// the table lies in an overlay (Platinum without DSPRE's patch). Read only for a path below
/// `arm9_overlays/` (`rel`), so a caller asking for every file stays cheap.
pub(crate) fn overlay_slice(project: &Project, rel: &Path) -> Option<(usize, usize)> {
    if !rel.starts_with("arm9_overlays") {
        return None;
    }
    let place = place(project)?;
    let Location::Slice { path, offset, len } = place.location else {
        return None;
    };
    (path.strip_prefix(&project.root).unwrap_or(&path) == rel)
        .then(|| usize::try_from(offset).ok().map(|offset| (offset, len)))
        .flatten()
}

/// The member of the table relative to the root: the NARC member or the overlay file.
pub(crate) fn member_rel(project: &Project, place: &EggMovesPlace) -> Option<PathBuf> {
    match &place.location {
        Location::Member(path) | Location::Slice { path, .. } => Some(
            path.strip_prefix(&project.root)
                .unwrap_or(path)
                .to_path_buf(),
        ),
        _ => None,
    }
}

/// The numbers of the species files on disk, ascending.
pub(crate) fn species_numbers(project: &Project) -> Result<Vec<usize>, KernelError> {
    numbered(&project.root.join(Art::Species.dir()), "json")
}

/// `data/species/NNNN.json`.
pub(crate) fn species_rel(n: usize) -> PathBuf {
    Art::Species.dir().join(format!("{n:04}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(words: &[u16]) -> EggMoves {
        EggMoves::read(
            &words
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<u8>>(),
        )
        .unwrap()
    }

    fn words(table: &EggMoves) -> Vec<u16> {
        table
            .write()
            .unwrap()
            .chunks(2)
            .map(|w| u16::from_le_bytes([w[0], w[1]]))
            .collect()
    }

    #[test]
    fn assemble_applies_the_fields_in_species_order_and_keeps_the_rest() {
        let t = table(&[20001, 5, 7, 20003, 20004, 9, 0xFFFF]);
        // Nothing differs: the same table, byte for byte.
        let same = assemble(
            t.clone(),
            &[
                (1, Some(vec![5, 7])),
                (2, None),
                (3, Some(vec![])),
                (4, Some(vec![9])),
            ],
        );
        assert_eq!(same, t);
        let changed = assemble(
            t.clone(),
            &[
                (1, Some(vec![7, 5])),
                (2, Some(vec![20])),
                (3, Some(vec![])),
                (4, None),
                (6, Some(vec![])),
            ],
        );
        assert_eq!(
            words(&changed),
            [20001, 7, 5, 20002, 20, 20003, 20006, 0xFFFF]
        );
        // An entry of a species without a field in the list stays.
        let kept = assemble(t, &[(1, Some(vec![8]))]);
        assert_eq!(words(&kept), [20001, 8, 20003, 20004, 9, 0xFFFF]);
        // A mark without moves replaces an entry with moves.
        let mut t = table(&[20001, 5, 0xFFFF]);
        put_entry(&mut t, 1, Some(&[]));
        assert_eq!(words(&t), [20001, 0xFFFF]);
    }

    #[test]
    fn field_numbers_must_stay_below_the_marks() {
        let sym = Symbols::load(Path::new("/nonexistent/g4-textproject")).unwrap();
        assert_eq!(
            resolve_field(
                &[SymRef::Num(7), SymRef::Id("MOVE_POUND".into())],
                &sym,
                None
            ),
            Ok(vec![7, 1])
        );
        let err = resolve_field(&[SymRef::Num(1), SymRef::Num(20000)], &sym, None).unwrap_err();
        assert!(
            err.starts_with("textproject.schema: egg_moves[1]: 20000"),
            "{err}"
        );
        let err = resolve_field(&[SymRef::Id("MOVE_FOO".into())], &sym, None).unwrap_err();
        assert!(
            err.starts_with("textproject.unknown-id: egg_moves[0]:"),
            "{err}"
        );
        // As `data_edit`: no move 0, none beyond the project's moves.
        let err = resolve_field(&[SymRef::Num(0)], &sym, Some(468)).unwrap_err();
        assert_eq!(
            err,
            "textproject.schema: egg_moves[0]: 0 is not an egg move (moves 1 to 467)."
        );
        let err = resolve_field(&[SymRef::Num(5), SymRef::Num(468)], &sym, Some(468)).unwrap_err();
        assert!(
            err.starts_with("textproject.schema: egg_moves[1]: 468 is not an"),
            "{err}"
        );
        assert_eq!(
            resolve_field(&[SymRef::Num(467)], &sym, Some(468)),
            Ok(vec![467])
        );
    }
}
