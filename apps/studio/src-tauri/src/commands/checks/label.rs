//! The label of a finding: its subject and the spot its key names ("Event file 0411 · Warp 2";
//! "Eventdatei 0411 · Warp 2" in German), in the words of the workbench (`assets.ts`,
//! `terms.ts`, the breadcrumb) and in the language of the process.

use g4_kernel::checks::Subject;
use g4_kernel::project::{AssetId, AssetKind};

use crate::commands::data::labels;

/// The subject, then the spot its `key` names. A key nobody words yet gives the subject alone,
/// never the raw key.
pub fn subject_label(subject: Subject, key: &str) -> String {
    let name = subject_name(subject);
    match spot(subject, key) {
        Some(spot) => format!("{name} · {spot}"),
        None => name,
    }
}

/// The name of an asset a fix changes ("Text archive 0433", "Script file 0005"): that of its
/// subject; a kind no fix changes keeps the kernel's name.
pub fn asset_label(asset: AssetId) -> String {
    let n = asset.index;
    let subject = match asset.kind {
        AssetKind::TextArchive => Subject::TextArchive(n),
        AssetKind::Script => Subject::ScriptFile(n),
        AssetKind::Encounter => Subject::Encounter(n),
        AssetKind::Event => Subject::EventFile(n),
        _ => return asset.to_string(),
    };
    subject_name(subject)
}

/// "Event file 0411", "Project".
fn subject_name(subject: Subject) -> String {
    let (key, n) = match subject {
        Subject::Header(n) => ("commands.checks.subject_header", n),
        Subject::EventFile(n) => ("commands.checks.subject_event_file", n),
        Subject::ScriptFile(n) => ("commands.checks.subject_script_file", n),
        Subject::TextArchive(n) => ("commands.checks.subject_text_archive", n),
        Subject::Trainer(n) => ("commands.checks.subject_trainer", n),
        Subject::Encounter(n) => ("commands.checks.subject_encounters", n),
        Subject::Matrix(n) => ("commands.checks.subject_matrix", n),
        Subject::Map(n) => ("commands.checks.subject_map", n),
        Subject::AreaData(n) => ("commands.checks.subject_area_data", n),
        Subject::Project => return t!("commands.checks.subject_project").into_owned(),
    };
    t!(key, n = n : {:04}).into_owned()
}

/// A number of a key as it is ("Line 7").
fn number(text: &str) -> Option<u32> {
    text.parse().ok()
}

/// "slot 3" for the 0-based position 2.
fn place(text: &str) -> Option<String> {
    Some(t!("commands.checks.place", n = number(text)? + 1).into_owned())
}

/// The name of an encounter method or replacement list ("Grass", "Swarm"), in the language of
/// the process.
fn method(key: &str) -> Option<String> {
    labels::label(labels::ENCOUNTER, key)
}

/// The spot `key` names in `subject` (the keys each check documents).
fn spot(subject: Subject, key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split(':').collect();
    // `asset.unreadable` on any subject.
    match parts[0] {
        "unreadable" => return Some(t!("commands.checks.spot_unreadable").into_owned()),
        "locked" => return Some(t!("commands.checks.spot_locked").into_owned()),
        _ => {}
    }
    let word = |key: &str| Some(t!(key).into_owned());
    match (subject, parts.as_slice()) {
        (Subject::EventFile(_), [kind, i]) => {
            let key = match *kind {
                "warp" => "commands.checks.spot_warp",
                "object" => "commands.checks.spot_object",
                "sign" => "commands.checks.spot_sign",
                "trigger" => "commands.checks.spot_trigger",
                _ => return None,
            };
            Some(t!(key, n = number(i)?).into_owned())
        }
        (Subject::Header(_), ["level", n]) => {
            Some(t!("commands.checks.spot_level_script", n = number(n)?).into_owned())
        }
        (Subject::Header(_), [field]) => word(match *field {
            "matrix_id" => "commands.checks.word_matrix",
            "event_file_id" => "commands.checks.word_event_file",
            "script_file_id" => "commands.checks.word_script_file",
            "level_script_id" => "commands.checks.word_level_script",
            "text_archive_id" => "commands.checks.word_text_archive",
            "area_data_id" => "commands.checks.word_area_data",
            "wild_pokemon" => "commands.checks.word_encounters",
            _ => return None,
        }),
        (Subject::ScriptFile(_), ["text_archive"]) => word("commands.checks.word_text_archive"),
        (Subject::ScriptFile(_), ["line", l, "message", m]) => Some(
            t!(
                "commands.checks.spot_line_message",
                line = number(l)?,
                message = number(m)?
            )
            .into_owned(),
        ),
        (Subject::TextArchive(_), ["message", m]) => {
            Some(t!("commands.checks.spot_message", n = number(m)?).into_owned())
        }
        (Subject::TextArchive(_), ["message", m, "line", l]) => Some(
            t!(
                "commands.checks.spot_message_line",
                message = number(m)?,
                line = number(l)?
            )
            .into_owned(),
        ),
        (Subject::Trainer(_), ["party"]) => word("commands.checks.word_party"),
        (Subject::Trainer(_), ["class"]) => word("commands.checks.word_class"),
        (Subject::Trainer(_), ["battle_type"]) => word("commands.checks.word_battle_type"),
        (Subject::Trainer(_), ["item", j]) => {
            Some(t!("commands.checks.spot_item", place = place(j)?).into_owned())
        }
        (Subject::Trainer(_), ["slot", k]) => {
            Some(t!("commands.checks.spot_party_slot", place = place(k)?).into_owned())
        }
        (Subject::Trainer(_), ["slot", k, what]) => {
            let key = match *what {
                "species" => "commands.checks.spot_party_species",
                "item" => "commands.checks.spot_party_item",
                "moves" => "commands.checks.spot_party_moves",
                _ => return None,
            };
            Some(t!(key, place = place(k)?).into_owned())
        }
        (Subject::Trainer(_), ["slot", k, "move", j]) => Some(
            t!(
                "commands.checks.spot_party_move",
                place = place(k)?,
                n = number(j)? + 1
            )
            .into_owned(),
        ),
        (Subject::Encounter(_), ["slot", m, i]) => Some(format!("{} {}", method(m)?, place(i)?)),
        (Subject::Encounter(_), ["slot", m, i, list, j]) => Some(format!(
            "{} {} · {} {}",
            method(m)?,
            place(i)?,
            method(list)?,
            place(j)?
        )),
        // HGSS: the species of one time of day (`encounter.empty-slot`).
        (Subject::Encounter(_), ["slot", m, i, when]) => {
            let when = match *when {
                "morning" => t!("commands.checks.when_morning"),
                "day" => t!("commands.checks.when_day"),
                "night" => t!("commands.checks.when_night"),
                _ => return None,
            };
            Some(format!("{} {} · {when}", method(m)?, place(i)?))
        }
        (Subject::Encounter(_), ["rate", m]) => {
            Some(t!("commands.checks.spot_rate", method = method(m)?).into_owned())
        }
        (Subject::Matrix(_), ["cell", xy]) => {
            let (x, y) = xy.split_once(',')?;
            Some(t!("commands.checks.spot_cell", x = number(x)?, y = number(y)?).into_owned())
        }
        (Subject::Map(_), ["model"]) => word("commands.checks.word_model"),
        (Subject::Map(_), ["bdhc"]) => word("commands.checks.word_terrain"),
        (Subject::Map(_), ["building", b]) => {
            Some(t!("commands.checks.spot_building", n = number(b)? + 1).into_owned())
        }
        (Subject::AreaData(_), [field]) => word(match *field {
            "map_tileset" => "commands.checks.word_map_textures",
            "buildings_tileset" => "commands.checks.word_building_textures",
            _ => return None,
        }),
        _ => None,
    }
}
