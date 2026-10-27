//! The German before/after lines of a reset plan: one line per entry, the same function for
//! both sides, so equal values give equal lines. Format: `docs/formats/project_base.md`,
//! section „Auf Original zurücksetzen“.

use std::io::Cursor;

use g4_formats::data::encounter::Encounters;
use g4_formats::data::evolution::{EVO_ENTRIES, Evolution};
use g4_formats::data::learnset::Learnset;
use g4_formats::data::personal::Personal;
use g4_formats::data::record::{FieldKind, FieldSpan, RawRecord};
use g4_formats::data::trainer::TrainerProps;
use g4_formats::data::trainer_doc::TrainerDoc;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use serde_json::json;
use uxie::event_file::EventFile;
use xxhash_rust::xxh3::xxh3_64;

use super::ResetTarget;
use crate::error::KernelError;
use crate::project::AssetKind;
use crate::text::TextArchiveDoc;

/// The lines of asset bytes of `kind` (in the asset's own byte form) for `target`, joined with
/// `\n`. Bytes that do not decode give one line with their size and the error.
pub(crate) fn lines(
    target: &ResetTarget,
    kind: AssetKind,
    family: Family,
    lang: &str,
    bytes: &[u8],
) -> String {
    let lines = match (target, kind) {
        (ResetTarget::TextMessage { message, .. }, AssetKind::TextArchive) => {
            message_line(bytes, lang, *message)
        }
        (_, AssetKind::Map) => map_lines(bytes, family),
        (_, AssetKind::Event) => event_lines(bytes),
        (_, AssetKind::TextArchive) => archive_lines(bytes, lang),
        (_, AssetKind::Trainer) => trainer_lines(bytes, family),
        (_, AssetKind::Encounter) => Encounters::read(bytes, family)
            .map(|e| match e {
                Encounters::Pt(e) => record_lines(&e, ""),
                Encounters::Hgss(e) => record_lines(&e, ""),
            })
            .map_err(KernelError::from),
        (_, AssetKind::Personal) => Personal::read(bytes)
            .map(|p| record_lines(&p, ""))
            .map_err(KernelError::from),
        (_, AssetKind::Learnset) => learnset_lines(bytes),
        (_, AssetKind::Evolution) => evolution_lines(bytes),
        _ => Ok(vec![format!("{} Byte {}", bytes.len(), short_hash(bytes))]),
    };
    match lines {
        Ok(lines) => lines.join("\n"),
        Err(e) => format!("{} Byte, nicht lesbar: {e}", bytes.len()),
    }
}

/// The first eight hex digits of the xxh3 of `bytes`.
fn short_hash(bytes: &[u8]) -> String {
    let mut hex = format!("{:016x}", xxh3_64(bytes));
    hex.truncate(8);
    hex
}

/// `0A FF 00`
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn map_lines(bytes: &[u8], family: Family) -> Result<Vec<String>, KernelError> {
    let map = MapFile::read(bytes, family)?;
    let cells: Vec<u8> = map
        .permissions
        .iter()
        .flat_map(|c| [c.kind, c.collision])
        .collect();
    Ok(vec![
        format!("Bewegungsrechte {}", short_hash(&cells)),
        format!("Gebäude {}", map.buildings.len()),
        format!("Modell {} Byte {}", map.model.len(), short_hash(&map.model)),
        format!("BDHC {} Byte {}", map.bdhc.len(), short_hash(&map.bdhc)),
    ])
}

/// One line per event: objects („Figur“), warps („Tür“), triggers („Auslöser“), then signs
/// („Schild“), each with every field of uxie's event as compact JSON.
fn event_lines(bytes: &[u8]) -> Result<Vec<String>, KernelError> {
    let file = EventFile::from_binary(&mut Cursor::new(bytes))
        .map_err(|e| KernelError::Uxie(e.to_string()))?;
    let mut out = Vec::new();
    for (i, e) in file.object_events.iter().enumerate() {
        let value = json!({
            "local_id": e.local_id, "graphics_id": e.graphics_id,
            "movement_type": e.movement_type, "trainer_type": e.trainer_type,
            "hidden_flag": e.hidden_flag, "script": e.script, "dir": e.dir, "data": e.data,
            "movement_range_x": e.movement_range_x, "movement_range_z": e.movement_range_z,
            "x": e.x, "z": e.z, "y": e.y,
        });
        out.push(format!("Figur {i}: {value}"));
    }
    for (i, e) in file.warp_events.iter().enumerate() {
        let value = json!({
            "x": e.x, "z": e.z, "dest_header_id": e.dest_header_id,
            "dest_warp_id": e.dest_warp_id, "height": e.height,
        });
        out.push(format!("Tür {i}: {value}"));
    }
    for (i, e) in file.coord_events.iter().enumerate() {
        let value = json!({
            "script": e.script, "x": e.x, "z": e.z, "width": e.width, "length": e.length,
            "y": e.y, "value": e.value, "var": e.var,
        });
        out.push(format!("Auslöser {i}: {value}"));
    }
    for (i, e) in file.bg_events.iter().enumerate() {
        let value = json!({
            "script": e.script, "event_type": e.event_type, "x": e.x, "z": e.z, "y": e.y,
            "player_facing_dir": e.player_facing_dir,
        });
        out.push(format!("Schild {i}: {value}"));
    }
    if out.is_empty() {
        out.push("Keine Events".to_owned());
    }
    Ok(out)
}

fn archive_lines(bytes: &[u8], lang: &str) -> Result<Vec<String>, KernelError> {
    let doc = TextArchiveDoc::from_json(bytes, lang)?;
    let mut out: Vec<String> = doc
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{i}: {}", m.text))
        .collect();
    if out.is_empty() {
        out.push("Keine Nachrichten".to_owned());
    }
    Ok(out)
}

/// The text of message `message`, in the chatot form of the JSON (`\n` as backslash and `n`).
fn message_line(bytes: &[u8], lang: &str, message: u16) -> Result<Vec<String>, KernelError> {
    let doc = TextArchiveDoc::from_json(bytes, lang)?;
    Ok(vec![doc.messages.get(usize::from(message)).map_or_else(
        || format!("Keine Nachricht {message}"),
        |m| m.text.clone(),
    )])
}

/// `{prefix}{name}: {value}` for every field of the record, `Bytes` as hex.
fn record_lines<R: RawRecord>(record: &R, prefix: &str) -> Vec<String> {
    R::FIELDS
        .iter()
        .map(|f| format!("{prefix}{}: {}", f.name, field_value(record, f)))
        .collect()
}

fn field_value<R: RawRecord>(record: &R, field: &FieldSpan) -> String {
    match field.kind {
        FieldKind::U8 => record.get_u8(field.offset).to_string(),
        FieldKind::U16 => record.get_u16(field.offset).to_string(),
        FieldKind::U32 => record.get_u32(field.offset).to_string(),
        FieldKind::I8 => record.get_i8(field.offset).to_string(),
        FieldKind::Bits { mask } => record.bits(field.offset, mask).to_string(),
        FieldKind::Bytes => hex(&record.raw()[field.offset..field.offset + field.len]),
    }
}

/// The fields of the properties, then per party member (`Team 1`, …) its values.
fn trainer_lines(bytes: &[u8], family: Family) -> Result<Vec<String>, KernelError> {
    let malformed =
        || KernelError::InvalidProject(format!("{} bytes are no trainer pair", bytes.len()));
    let (len, rest) = bytes.split_first_chunk::<4>().ok_or_else(malformed)?;
    let len = usize::try_from(u32::from_le_bytes(*len)).map_err(|_| malformed())?;
    if len > rest.len() {
        return Err(malformed());
    }
    let (props, party) = rest.split_at(len);
    let mut out = record_lines(&TrainerProps::read(props)?, "");
    let doc = TrainerDoc::decode(props, party, family)?;
    for (n, mon) in doc.party.iter().enumerate() {
        let team = format!("Team {} ", n + 1);
        out.push(format!("{team}iv_or_difficulty: {}", mon.iv_or_difficulty));
        out.push(format!("{team}byte1: {}", mon.byte1));
        out.push(format!("{team}level: {}", mon.level));
        out.push(format!("{team}species: {}", mon.species()));
        out.push(format!("{team}form: {}", mon.form()));
        if let Some(item) = mon.item {
            out.push(format!("{team}item: {item}"));
        }
        if let Some(moves) = mon.moves {
            let moves: Vec<String> = moves.iter().map(u16::to_string).collect();
            out.push(format!("{team}moves: {}", moves.join(", ")));
        }
        if let Some(capsule) = mon.ball_capsule {
            out.push(format!("{team}ball_capsule: {capsule}"));
        }
    }
    Ok(out)
}

fn learnset_lines(bytes: &[u8]) -> Result<Vec<String>, KernelError> {
    let learnset = Learnset::read(bytes)?;
    let mut out: Vec<String> = learnset
        .entries
        .iter()
        .map(|e| format!("Lv {}: Attacke {}", e.level, e.move_id))
        .collect();
    if out.is_empty() {
        out.push("Keine Attacken".to_owned());
    }
    Ok(out)
}

/// `{method} {param} → {target}` for each entry that is not all zero.
fn evolution_lines(bytes: &[u8]) -> Result<Vec<String>, KernelError> {
    let evolution = Evolution::read(bytes)?;
    let mut out: Vec<String> = (0..EVO_ENTRIES)
        .map(|i| evolution.entry(i))
        .filter(|e| e.method != 0 || e.param != 0 || e.target != 0)
        .map(|e| format!("{} {} → {}", e.method, e.param, e.target))
        .collect();
    if out.is_empty() {
        out.push("Keine Entwicklung".to_owned());
    }
    Ok(out)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;

    fn line_of(target: ResetTarget, kind: AssetKind, bytes: &[u8]) -> String {
        lines(&target, kind, Family::Pt, "en_US", bytes)
    }

    #[test]
    fn data_lines_follow_the_table() {
        let species = ResetTarget::Species { index: 1 };
        let learnset = [0x01, 0x02, 0x02, 0x0A, 0xFF, 0xFF, 0x00, 0x00];
        assert_eq!(
            line_of(species, AssetKind::Learnset, &learnset),
            "Lv 1: Attacke 1\nLv 5: Attacke 2"
        );
        let mut evolution = [0u8; 44];
        evolution[..6].copy_from_slice(&[4, 0, 16, 0, 2, 0]);
        assert_eq!(
            line_of(species, AssetKind::Evolution, &evolution),
            "4 16 → 2"
        );
        assert_eq!(
            line_of(species, AssetKind::Evolution, &[0; 44]),
            "Keine Entwicklung"
        );
        let mut personal = [0u8; 44];
        personal[0] = 45;
        let text = line_of(species, AssetKind::Personal, &personal);
        assert!(text.starts_with("hp: 45\n"), "{text}");
        let mut encounter = vec![0u8; 424];
        encounter[0] = 10;
        let text = line_of(
            ResetTarget::Encounter { index: 0 },
            AssetKind::Encounter,
            &encounter,
        );
        assert!(
            text.starts_with("grass_rate: 10\ngrass_00_level: 0\n"),
            "{text}"
        );
        // Not decodable: the size and why.
        assert!(line_of(species, AssetKind::Personal, &[0; 3]).starts_with("3 Byte, nicht lesbar"));
    }

    #[test]
    fn trainer_lines_list_the_properties_then_each_team_member() {
        let mut props = [0u8; 20];
        props[0] = 2; // items
        props[1] = 7; // class
        props[3] = 1; // one Pokémon
        let party = [10, 0, 5, 0, 1, 0, 25, 0, 0, 0, 0, 0];
        let text = line_of(
            ResetTarget::Trainer { index: 1 },
            AssetKind::Trainer,
            &crate::store::trainer_pair_bytes(&props, &party),
        );
        assert!(text.contains("\nclass: 7\n"), "{text}");
        assert!(
            text.contains("\nitems: 00 00 00 00 00 00 00 00\n"),
            "{text}"
        );
        assert!(
            text.contains("\nTeam 1 level: 5\nTeam 1 species: 1\n"),
            "{text}"
        );
        assert!(
            text.contains("\nTeam 1 item: 25\nTeam 1 ball_capsule: 0"),
            "{text}"
        );
    }

    #[test]
    fn text_and_map_lines_follow_the_table() {
        let doc = TextArchiveDoc {
            key: 1,
            lang: "en_US".into(),
            messages: ["Hallo", "Zwei\\nZeilen"]
                .iter()
                .enumerate()
                .map(|(i, t)| crate::text::TextMessage {
                    id: crate::text::default_message_id(i),
                    text: (*t).to_owned(),
                    other: Default::default(),
                })
                .collect(),
        };
        let json = doc.to_json();
        assert_eq!(
            line_of(
                ResetTarget::TextArchive { index: 0 },
                AssetKind::TextArchive,
                &json
            ),
            "0: Hallo\n1: Zwei\\nZeilen"
        );
        assert_eq!(
            line_of(
                ResetTarget::TextMessage {
                    archive: 0,
                    message: 1
                },
                AssetKind::TextArchive,
                &json
            ),
            "Zwei\\nZeilen"
        );
        let map = crate::test_support::synthetic_map_bytes(1);
        let text = line_of(ResetTarget::Map { index: 0 }, AssetKind::Map, &map);
        let parts: Vec<&str> = text.split('\n').collect();
        assert_eq!(parts.len(), 4, "{text}");
        assert!(parts[0].starts_with("Bewegungsrechte ") && parts[0].len() == 24);
        assert_eq!(parts[1], "Gebäude 1");
        assert!(parts[2].starts_with("Modell 6 Byte "), "{text}");
        assert!(parts[3].starts_with("BDHC 3 Byte "), "{text}");
        assert_eq!(
            line_of(
                ResetTarget::EventFile { index: 0 },
                AssetKind::Event,
                &[0; 16]
            ),
            "Keine Events"
        );
    }
}
