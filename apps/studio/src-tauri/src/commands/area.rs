//! Area data: the tileset references and light of one area.

use g4_formats::area_data::AreaData;
use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_kernel::error::KernelError;
use g4_kernel::project::AssetKind;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::{AppState, Session};

/// One area data record as the frontend sees it: the codec's fields, tagged by layout. The
/// layout follows the project's game family (Diamond/Pearl/Platinum share one).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family")]
pub enum AreaDataDto {
    Dppt {
        buildings_tileset: u16,
        map_tileset: u16,
        unknown: u16,
        light_type: u16,
    },
    Hgss {
        buildings_tileset: u16,
        map_tileset: u16,
        dynamic_texture_type: u16,
        area_type: u8,
        light_type: u8,
    },
}

impl From<AreaData> for AreaDataDto {
    fn from(a: AreaData) -> Self {
        match a {
            AreaData::Dppt {
                buildings_tileset,
                map_tileset,
                unknown,
                light_type,
            } => AreaDataDto::Dppt {
                buildings_tileset,
                map_tileset,
                unknown,
                light_type,
            },
            AreaData::Hgss {
                buildings_tileset,
                map_tileset,
                dynamic_texture_type,
                area_type,
                light_type,
            } => AreaDataDto::Hgss {
                buildings_tileset,
                map_tileset,
                dynamic_texture_type,
                area_type,
                light_type,
            },
        }
    }
}

impl From<AreaDataDto> for AreaData {
    fn from(a: AreaDataDto) -> Self {
        match a {
            AreaDataDto::Dppt {
                buildings_tileset,
                map_tileset,
                unknown,
                light_type,
            } => AreaData::Dppt {
                buildings_tileset,
                map_tileset,
                unknown,
                light_type,
            },
            AreaDataDto::Hgss {
                buildings_tileset,
                map_tileset,
                dynamic_texture_type,
                area_type,
                light_type,
            } => AreaData::Hgss {
                buildings_tileset,
                map_tileset,
                dynamic_texture_type,
                area_type,
                light_type,
            },
        }
    }
}

impl AreaDataDto {
    fn layout(&self) -> &'static str {
        match self {
            AreaDataDto::Dppt { .. } => "Dppt",
            AreaDataDto::Hgss { .. } => "Hgss",
        }
    }
}

fn project_layout(family: Family) -> &'static str {
    match family {
        Family::Hgss => "Hgss",
        Family::Dp | Family::Pt => "Dppt",
    }
}

/// Fails unless area data `id` exists in the project's archive.
fn check_id(session: &Session, id: u16) -> Result<(), CmdError> {
    let count = session.project.member_count(AssetKind::AreaData)?;
    if usize::from(id) >= count {
        return Err(KernelError::Format(FormatError::OutOfRange {
            what: "area data",
            value: usize::from(id),
            max: count.saturating_sub(1),
        })
        .into());
    }
    Ok(())
}

/// Area data `id` as the store holds it, edits included.
pub fn get_area_data_impl(state: &AppState, id: u16) -> Result<AreaDataDto, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    check_id(session, id)?;
    Ok((*session.store.area_data(id).map_err(CmdError::from)?).into())
}

/// Replaces area data `id` as a whole; `label` names the change in the undo history. The
/// record must use the layout of the project's game family; an identical record records no
/// change. A refused call changes nothing.
pub fn set_area_data_impl(
    state: &AppState,
    id: u16,
    area: AreaDataDto,
    label: &str,
) -> Result<(), CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let expected = project_layout(session.project.family);
    if area.layout() != expected {
        return Err(format!(
            "area data family mismatch: got the {} layout, this project uses the {expected} layout",
            area.layout()
        )
        .into());
    }
    check_id(session, id)?;
    let area = AreaData::from(area);
    session.store.edit_area_data(id, label, |slot| {
        *slot = area;
        Ok(())
    })?;
    Ok(())
}

#[tauri::command]
pub fn get_area_data(state: State<AppState>, id: u16) -> Result<AreaDataDto, CmdError> {
    get_area_data_impl(&state, id)
}

#[tauri::command]
pub fn set_area_data(
    state: State<AppState>,
    id: u16,
    area: AreaDataDto,
    label: Option<String>,
) -> Result<(), CmdError> {
    set_area_data_impl(
        &state,
        id,
        area,
        label.as_deref().unwrap_or("edit area data"),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::super::{dirty_impl, redo_impl, save_impl, undo_impl};
    use super::*;
    use g4_kernel::project::{AssetId, AssetKind};

    fn area_id(index: u16) -> AssetId {
        AssetId {
            kind: AssetKind::AreaData,
            index,
        }
    }

    #[test]
    fn area_data_commands_round_trip() {
        let (_s, state) = open_synthetic();
        let mut a = get_area_data_impl(&state, 1).unwrap();
        if let AreaDataDto::Dppt { map_tileset, .. } = &mut a {
            *map_tileset = 3;
        }
        set_area_data_impl(&state, 1, a.clone(), "map tileset").unwrap();
        assert_eq!(get_area_data_impl(&state, 1).unwrap(), a);
        assert!(dirty_impl(&state).unwrap().contains(&area_id(1)));
        // One undoable change; undo and redo move the value.
        assert_eq!(undo_impl(&state).unwrap(), Some(area_id(1)));
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(redo_impl(&state).unwrap().is_some());
        assert_eq!(get_area_data_impl(&state, 1).unwrap(), a);
        let report = save_impl(&state).unwrap();
        assert_eq!(report.written, vec![area_id(1)]);
    }

    #[test]
    fn the_dto_is_tagged_by_family_with_the_codec_field_names() {
        let (_s, state) = open_synthetic();
        let json = serde_json::to_value(get_area_data_impl(&state, 0).unwrap()).unwrap();
        assert_eq!(json["family"], "Dppt");
        for key in ["buildings_tileset", "map_tileset", "unknown", "light_type"] {
            assert!(json.get(key).is_some(), "{key}");
        }
        let back: AreaDataDto = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), json);
        let hgss: AreaDataDto = serde_json::from_value(serde_json::json!({
            "family": "Hgss", "buildings_tileset": 1, "map_tileset": 2,
            "dynamic_texture_type": 3, "area_type": 1, "light_type": 9
        }))
        .unwrap();
        assert!(matches!(hgss, AreaDataDto::Hgss { area_type: 1, .. }));
    }

    #[test]
    fn a_dto_of_the_other_family_is_rejected_and_changes_nothing() {
        let (_s, state) = open_synthetic();
        let before = get_area_data_impl(&state, 0).unwrap();
        let other = AreaDataDto::Hgss {
            buildings_tileset: 1,
            map_tileset: 2,
            dynamic_texture_type: 0,
            area_type: 1,
            light_type: 0,
        };
        let err = set_area_data_impl(&state, 0, other, "x").unwrap_err();
        assert_eq!(err.code, "error");
        assert!(err.message.contains("family"), "{}", err.message);
        assert_eq!(get_area_data_impl(&state, 0).unwrap(), before);
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(undo_impl(&state).unwrap().is_none());
    }

    #[test]
    fn an_identical_area_data_records_no_change() {
        let (_s, state) = open_synthetic();
        let a = get_area_data_impl(&state, 0).unwrap();
        set_area_data_impl(&state, 0, a, "x").unwrap();
        assert!(dirty_impl(&state).unwrap().is_empty());
        assert!(undo_impl(&state).unwrap().is_none());
    }

    #[test]
    fn unknown_area_data_and_a_closed_project_are_errors() {
        let (_s, state) = open_synthetic();
        let err = get_area_data_impl(&state, 2).unwrap_err();
        assert!(err.message.contains("out of range"), "{}", err.message);
        let a = get_area_data_impl(&state, 0).unwrap();
        assert!(set_area_data_impl(&state, 2, a.clone(), "x").is_err());
        assert!(dirty_impl(&state).unwrap().is_empty());
        let closed = AppState::default();
        assert_eq!(
            get_area_data_impl(&closed, 0).unwrap_err().message,
            "no project open"
        );
        assert_eq!(
            set_area_data_impl(&closed, 0, a, "x").unwrap_err().message,
            "no project open"
        );
    }

    /// Gated: on every vanilla family the DTO carries the project's layout and a saved tileset
    /// change survives reopening.
    #[test]
    fn vanilla_area_data_edit_saves_and_reopens() {
        use super::super::open_project_impl;
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        let info = open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let mut a = get_area_data_impl(&state, 1).unwrap();
        let expected = if info.family == "Hgss" {
            "Hgss"
        } else {
            "Dppt"
        };
        assert_eq!(a.layout(), expected);
        let (AreaDataDto::Dppt { map_tileset, .. } | AreaDataDto::Hgss { map_tileset, .. }) =
            &mut a;
        *map_tileset ^= 1;
        let edited = a.clone();
        set_area_data_impl(&state, 1, a, "map tileset").unwrap();
        assert_eq!(save_impl(&state).unwrap().written, vec![area_id(1)]);

        let reopened = AppState::default();
        open_project_impl(&reopened, root.to_str().unwrap()).unwrap();
        assert_eq!(get_area_data_impl(&reopened, 1).unwrap(), edited);
    }
}
