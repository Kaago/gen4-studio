//! The German label of a finding: its subject and the spot its key names ("Eventdatei 0411 ·
//! Warp 2"), in the words of the workbench (`assets.ts`, `terms.ts`, the breadcrumb).

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

/// The German name of an asset a fix changes ("Textarchiv 0433", "Scriptdatei 0005"): that of
/// its subject; a kind no fix changes keeps the kernel's name.
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

/// "Eventdatei 0411", "Projekt".
fn subject_name(subject: Subject) -> String {
    let (word, n) = match subject {
        Subject::Header(n) => ("Header", n),
        Subject::EventFile(n) => ("Eventdatei", n),
        Subject::ScriptFile(n) => ("Scriptdatei", n),
        Subject::TextArchive(n) => ("Textarchiv", n),
        Subject::Trainer(n) => ("Trainer", n),
        Subject::Encounter(n) => ("Begegnungen", n),
        Subject::Matrix(n) => ("Matrix", n),
        Subject::Map(n) => ("Karte", n),
        Subject::AreaData(n) => ("Area-Data", n),
        Subject::Project => return "Projekt".to_owned(),
    };
    format!("{word} {n:04}")
}

/// A number of a key as it is ("Zeile 7").
fn number(text: &str) -> Option<u32> {
    text.parse().ok()
}

/// "Platz 3" for the 0-based position 2.
fn place(text: &str) -> Option<String> {
    Some(format!("Platz {}", number(text)? + 1))
}

/// The German name of an encounter method or replacement list ("Gras", "Schwarm").
fn method(key: &str) -> Option<&'static str> {
    labels::label(labels::ENCOUNTER, key)
}

/// The spot `key` names in `subject` (the keys each check documents).
fn spot(subject: Subject, key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split(':').collect();
    // `asset.unreadable` on any subject.
    match parts[0] {
        "unreadable" => return Some("nicht lesbar".to_owned()),
        "locked" => return Some("gesperrt".to_owned()),
        _ => {}
    }
    let word = |w: &str| Some(w.to_owned());
    match (subject, parts.as_slice()) {
        (Subject::EventFile(_), [kind, i]) => {
            let kind = match *kind {
                "warp" => "Warp",
                "object" => "Objekt",
                "sign" => "Schild",
                "trigger" => "Trigger",
                _ => return None,
            };
            Some(format!("{kind} {}", number(i)?))
        }
        (Subject::Header(_), ["level", n]) => Some(format!("Level-Script #{}", number(n)?)),
        (Subject::Header(_), [field]) => word(match *field {
            "matrix_id" => "Matrix",
            "event_file_id" => "Eventdatei",
            "script_file_id" => "Scriptdatei",
            "level_script_id" => "Level-Script",
            "text_archive_id" => "Textarchiv",
            "area_data_id" => "Area-Data",
            "wild_pokemon" => "Begegnungen",
            _ => return None,
        }),
        (Subject::ScriptFile(_), ["text_archive"]) => word("Textarchiv"),
        (Subject::ScriptFile(_), ["line", l, "message", m]) => {
            Some(format!("Zeile {} · Nachricht {}", number(l)?, number(m)?))
        }
        (Subject::TextArchive(_), ["message", m]) => Some(format!("Nachricht {}", number(m)?)),
        (Subject::TextArchive(_), ["message", m, "line", l]) => {
            Some(format!("Nachricht {} · Zeile {}", number(m)?, number(l)?))
        }
        (Subject::Trainer(_), ["party"]) => word("Team"),
        (Subject::Trainer(_), ["class"]) => word("Klasse"),
        (Subject::Trainer(_), ["battle_type"]) => word("Kampfart"),
        (Subject::Trainer(_), ["item", j]) => Some(format!("Gegenstand {}", place(j)?)),
        (Subject::Trainer(_), ["slot", k]) => Some(format!("Team {}", place(k)?)),
        (Subject::Trainer(_), ["slot", k, what]) => {
            let what = match *what {
                "species" => "Pokémon",
                "item" => "Gegenstand",
                "moves" => "Attacken",
                _ => return None,
            };
            Some(format!("Team {} · {what}", place(k)?))
        }
        (Subject::Trainer(_), ["slot", k, "move", j]) => {
            Some(format!("Team {} · Attacke {}", place(k)?, number(j)? + 1))
        }
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
                "morning" => "morgens",
                "day" => "tagsüber",
                "night" => "nachts",
                _ => return None,
            };
            Some(format!("{} {} · {when}", method(m)?, place(i)?))
        }
        (Subject::Encounter(_), ["rate", m]) => Some(format!("{} · Rate", method(m)?)),
        (Subject::Matrix(_), ["cell", xy]) => {
            let (x, y) = xy.split_once(',')?;
            Some(format!("Zelle x {}, y {}", number(x)?, number(y)?))
        }
        (Subject::Map(_), ["model"]) => word("Modell"),
        (Subject::Map(_), ["bdhc"]) => word("Terrain"),
        (Subject::Map(_), ["building", b]) => Some(format!("Gebäude {}", number(b)? + 1)),
        (Subject::AreaData(_), [field]) => word(match *field {
            "map_tileset" => "Karten-Texturen",
            "buildings_tileset" => "Gebäude-Texturen",
            _ => return None,
        }),
        _ => None,
    }
}
