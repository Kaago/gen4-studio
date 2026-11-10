//! New encounter files: appended at the end of the archive (index N), as a copy of another
//! file or empty, and "split for this place": a header gets a copy of its encounter file of
//! its own. Source: `docs/research/2026-10-23-3b3/encounter-mechanics-3b3b.md` §6 (the game
//! takes the member count from the NARC at run time; a member is exactly 424 bytes in
//! Platinum, 196 in HeartGold/SoulSilver; a HGSS header stores the index in one byte).

use g4_formats::data::encounter::{HgssEncounters, PtEncounters};
use g4_formats::data::record::RawRecord;
use g4_formats::error::FormatError;
use g4_formats::game::Family;

use crate::data::refs::encounter_file;
use crate::error::KernelError;
use crate::project::{AssetKind, Project};
use crate::store::{ChangeId, MapHeader, Store};

/// The refusal for an encounter file a HeartGold/SoulSilver header could not name (the UI
/// shows it as it is).
pub fn hgss_limit() -> String {
    t!("data.hgss_encounter_limit").into_owned()
}

/// Encounter files a header of `family` can name: HGSS stores the index in a byte with 0xFF
/// for "none" (0 to 254); Platinum and DP in a u16 with 0xFFFF for "none".
pub fn encounter_file_limit(family: Family) -> usize {
    match family {
        Family::Hgss => 255,
        Family::Pt | Family::Dp => 0xFFFF,
    }
}

/// Bytes of one encounter file of `family`.
fn encounter_len(family: Family) -> usize {
    match family {
        Family::Hgss => HgssEncounters::LEN,
        Family::Pt | Family::Dp => PtEncounters::LEN,
    }
}

/// Appends an encounter file at the end of the archive and returns its index: a copy of
/// file `copy_of` as the editor shows it (unsaved edits included), or one of zero bytes
/// (424 or 196). Written at once, not undoable. Refused when no header could name the new
/// index ([`hgss_limit`] in HeartGold/SoulSilver) or the file to copy has the wrong length.
pub fn create_encounter_file(
    project: &Project,
    store: &mut Store,
    copy_of: Option<u16>,
) -> Result<u16, KernelError> {
    let family = project.family;
    let count = store.member_count(AssetKind::Encounter)?;
    if count >= encounter_file_limit(family) {
        return Err(KernelError::Refused(match family {
            Family::Hgss => hgss_limit(),
            Family::Pt | Family::Dp => t!("data.encounter_limit", count = count).into_owned(),
        }));
    }
    let len = encounter_len(family);
    let bytes = match copy_of {
        Some(source) => {
            let bytes = store.encounter_bytes(source)?;
            if bytes.len() != len {
                return Err(KernelError::Refused(
                    t!(
                        "data.encounter_wrong_length",
                        source = source : {:04},
                        len = bytes.len(),
                        expected = len
                    )
                    .into_owned(),
                ));
            }
            bytes
        }
        None => vec![0; len],
    };
    store.append_encounter(&bytes)
}

/// Gives header `header` an encounter file of its own: appends a copy of the file it uses
/// ([`create_encounter_file`], not undoable) and points the header at the copy as one undo
/// step. Returns the new file and the step. Refused, with nothing written, when the header
/// does not exist or has no encounter file; a header that cannot be switched after the copy
/// was written is [`KernelError::Partial`].
pub fn split_encounter_file(
    project: &Project,
    store: &mut Store,
    header: u16,
) -> Result<(u16, Option<ChangeId>), KernelError> {
    let headers = store.header_table()?;
    let current = headers.get(usize::from(header)).ok_or_else(|| {
        KernelError::Refused(
            t!("data.no_header", header = header, count = headers.len()).into_owned(),
        )
    })?;
    let file = encounter_file(current).ok_or_else(|| {
        KernelError::Refused(t!("data.header_without_encounters", header = header).into_owned())
    })?;
    let copy = create_encounter_file(project, store, Some(file))?;
    let label = t!("data.encounter_split_label", copy = copy, header = header).into_owned();
    let (_, change) = store
        .transaction(&label, |tx| {
            tx.edit_header(header, |h| set_encounter_file(h, copy))
        })
        .map_err(|e| {
            KernelError::Partial(format!(
                "encounter file {copy} was written as a copy of {file}; switching header \
                 {header} to it failed: {e}"
            ))
        })?;
    Ok((copy, change))
}

/// Points `header` at encounter file `file` (0xFFFF: none); a file the header cannot store
/// is [`FormatError::OutOfRange`].
fn set_encounter_file(header: &mut MapHeader, file: u16) -> Result<(), FormatError> {
    match header {
        MapHeader::DP(h) => h.wild_pokemon = file,
        MapHeader::Pt(h) => h.wild_pokemon = file,
        MapHeader::HGSS(h) => {
            h.wild_pokemon = match file {
                0xFFFF => u8::MAX,
                _ => u8::try_from(file).map_err(|_| FormatError::OutOfRange {
                    what: "encounter file",
                    value: usize::from(file),
                    max: 254,
                })?,
            }
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "test-support"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::project::AssetId;
    use crate::test_support::{PT_ENCOUNTER_MEMBERS, Synthetic, synthetic_project};

    fn fixture() -> (Synthetic, Arc<Project>, Store) {
        let s = synthetic_project();
        let project = Arc::new(Project::open(s.dir.path()).unwrap());
        let store = Store::new(project.clone());
        (s, project, store)
    }

    fn member(project: &Project, index: u16) -> Vec<u8> {
        std::fs::read(project.member_path(AssetKind::Encounter, index)).unwrap()
    }

    fn wild(store: &mut Store, header: u16) -> Option<u16> {
        encounter_file(&store.header_table().unwrap()[usize::from(header)])
    }

    #[test]
    fn create_encounter_file_appends_a_member_of_the_right_size() {
        let (_s, project, mut store) = fixture();
        let first = PT_ENCOUNTER_MEMBERS as u16;
        assert_eq!(
            create_encounter_file(&project, &mut store, None).unwrap(),
            first
        );
        assert_eq!(member(&project, first), [0; 424]);
        // A copy holds the editor's version, its unsaved edit included.
        store
            .edit_encounter(0, "rate", |e| {
                if let g4_formats::data::encounter::Encounters::Pt(p) = e {
                    p.set_u32(0, 30); // grass rate
                }
            })
            .unwrap();
        let copy = create_encounter_file(&project, &mut store, Some(0)).unwrap();
        assert_eq!(copy, first + 1);
        assert_eq!(member(&project, copy), store.encounter_bytes(0).unwrap());
        assert_eq!(member(&project, copy)[0], 30);
        assert_eq!(
            store.member_count(AssetKind::Encounter).unwrap(),
            PT_ENCOUNTER_MEMBERS + 2
        );
        // Creating is not an undo step; the rate edit is the only one.
        assert_eq!(store.history().0, ["rate"]);
        assert!(create_encounter_file(&project, &mut store, Some(99)).is_err());
    }

    #[test]
    fn encounter_file_limit_is_one_byte_in_hgss() {
        assert_eq!(encounter_file_limit(Family::Hgss), 255);
        assert_eq!(encounter_file_limit(Family::Pt), 65535);
        assert_eq!(encounter_len(Family::Hgss), 196);
        assert_eq!(encounter_len(Family::Pt), 424);
    }

    #[test]
    fn split_encounter_file_is_one_history_step_for_the_header() {
        let (_s, project, mut store) = fixture();
        assert_eq!(
            (wild(&mut store, 0), wild(&mut store, 1)),
            (Some(0), Some(0))
        );
        let (copy, change) = split_encounter_file(&project, &mut store, 0).unwrap();
        assert_eq!(copy, PT_ENCOUNTER_MEMBERS as u16);
        assert!(change.is_some());
        assert_eq!(member(&project, copy), member(&project, 0));
        // Only this header is switched, in one step.
        assert_eq!(
            (wild(&mut store, 0), wild(&mut store, 1)),
            (Some(copy), Some(0))
        );
        assert_eq!(
            store.history().0,
            [format!("Encounter file {copy} split off for Header 0")]
        );
        // Undo switches the header back; the copy stays.
        let undone = store.undo().unwrap();
        assert_eq!(
            undone,
            [AssetId {
                kind: AssetKind::HeaderTable,
                index: 0
            }]
        );
        assert_eq!(wild(&mut store, 0), Some(0));
        assert!(project.member_path(AssetKind::Encounter, copy).is_file());
        // A header that does not exist is refused, nothing is created.
        let err = split_encounter_file(&project, &mut store, 99).unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        let headers = store.header_table().unwrap().len();
        assert_eq!(
            err.to_string(),
            format!("Header 99 does not exist (the project has {headers} headers).")
        );
        assert_eq!(
            store.member_count(AssetKind::Encounter).unwrap(),
            PT_ENCOUNTER_MEMBERS + 1
        );
    }

    #[test]
    fn split_refuses_a_header_without_encounters() {
        let (_s, project, mut store) = fixture();
        store
            .edit_header(1, "none", |h| set_encounter_file(h, 0xFFFF))
            .unwrap();
        assert_eq!(wild(&mut store, 1), None);
        let err = split_encounter_file(&project, &mut store, 1).unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert_eq!(
            err.to_string(),
            "Header 1 has no encounter file. Attach one in the header editor (field \
             “Encounters”); then it can be split off."
        );
        assert_eq!(
            store.member_count(AssetKind::Encounter).unwrap(),
            PT_ENCOUNTER_MEMBERS
        );
    }
}
