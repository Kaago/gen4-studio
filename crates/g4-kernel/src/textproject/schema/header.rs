//! Map headers: one file per header with its id (`MAP_<internal name>`), the internal name
//! from `mapname.bin` and every field of uxie's header of the project's family, the location
//! name as a place id. The table as a whole comes from the slice of `arm9.bin` or, with the
//! Dynamic Headers patch, from the files of `unpacked/dynamicHeaders`, and goes back there.
//!
//! Each of the 24 bytes of a header is in exactly one field (uxie splits the bit fields
//! completely), so every header decodes and writes back byte for byte: a header has no locked
//! form.

use std::path::{Path, PathBuf};

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};
use uxie::GameFamily;
use uxie::map_header::{
    MapHeader, MapHeaderHGSS, MapHeaderPt, read_map_header_from_bytes, write_map_header_to_bytes,
};

use crate::dynamic_headers;
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Location, Project};
use crate::store::MAX_HEADERS;
use crate::text::Problem;

use super::super::DATA_DIR;
use super::super::canon::{hex, parse_entity, problem, to_canonical, unhex, write_if_changed};
use super::super::manifest::SCHEMA_CODE;
use super::super::part::{BasePart, PartKey};
use super::super::registry::Symbols;
use super::super::symbols::{SymbolKind, fallback_id};
use super::{
    EntitySchema, ExportCtx, ImportCtx, ROOT, SymRef, check_id, entity_id, required, resolve_max,
};

/// A header count that grows without the Dynamic Headers patch, or header files with a gap.
pub(crate) use super::super::codes::HEADER_COUNT_CODE;
/// A header table whose place in the project cannot be read (`arm9.bin` too short, no
/// `mapname.bin`, …).
pub(crate) use super::super::codes::HEADER_TABLE_CODE;
/// A member (here: a header) the import would have to delete because no JSON names it;
/// deleting is 3d.
pub(crate) use super::super::codes::MEMBER_WITHOUT_JSON_CODE;
/// The folder of the header files under `data/`.
pub(crate) const HEADERS_ART: &str = "headers";
/// Bytes of one `mapname.bin` record.
const NAME_LEN: usize = 16;
/// Bytes of one header.
const HEADER_LEN: usize = uxie::map_header::MAP_HEADER_SIZE;
/// The refusal for Diamond/Pearl.
pub const NO_DP: &str = "Kein Textprojekt für Diamant/Perl in Phase 4.";

/// A Platinum header (uxie `MapHeaderPt`, same fields in the same order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderPtEntity {
    pub id: String,
    pub internal_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_name_bytes: Option<String>,
    pub area_data_id: u8,
    pub unknown1: u8,
    pub matrix_id: u16,
    pub script_file_id: u16,
    pub level_script_id: u16,
    pub text_archive_id: u16,
    pub music_day_id: u16,
    pub music_night_id: u16,
    pub wild_pokemon: u16,
    pub event_file_id: u16,
    pub location_name: SymRef,
    pub area_icon: u8,
    pub weather_id: u8,
    pub camera_angle_id: u8,
    pub location_specifier: u8,
    pub battle_background: u8,
    pub flags: u8,
}

/// A HeartGold/SoulSilver header (uxie `MapHeaderHGSS`, same fields in the same order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderHgssEntity {
    pub id: String,
    pub internal_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_name_bytes: Option<String>,
    pub wild_pokemon: u8,
    pub area_data_id: u8,
    pub unknown0: u8,
    pub worldmap_x: u8,
    pub worldmap_y: u8,
    pub matrix_id: u16,
    pub script_file_id: u16,
    pub level_script_id: u16,
    pub text_archive_id: u16,
    pub music_day_id: u16,
    pub music_night_id: u16,
    pub event_file_id: u16,
    pub location_name: SymRef,
    pub area_icon: u8,
    pub unknown1: u8,
    pub kanto_flag: bool,
    pub weather_id: u8,
    pub location_type: u8,
    pub camera_angle_id: u8,
    pub follow_mode: u8,
    pub battle_background: u8,
    pub flags: u8,
}

impl EntitySchema for HeaderPtEntity {
    const ART: &'static str = HEADERS_ART;
    const LOCKABLE: bool = false;

    /// `members` are the header's 24 bytes and its `mapname.bin` record.
    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let (header, record) = header_members(members, index)?;
        let MapHeader::Pt(h) = decode(header, GameFamily::Platinum, index)? else {
            return Err(not_family(index, "Platin"));
        };
        let (internal_name, internal_name_bytes) = name_json(record);
        Ok(HeaderPtEntity {
            id: entity_id(ctx.symbols, SymbolKind::Map, index),
            internal_name,
            internal_name_bytes,
            area_data_id: h.area_data_id,
            unknown1: h.unknown1,
            matrix_id: h.matrix_id,
            script_file_id: h.script_file_id,
            level_script_id: h.level_script_id,
            text_archive_id: h.text_archive_id,
            music_day_id: h.music_day_id,
            music_night_id: h.music_night_id,
            wild_pokemon: h.wild_pokemon,
            event_file_id: h.event_file_id,
            location_name: place_ref(ctx.symbols, h.location_name),
            area_icon: h.area_icon,
            weather_id: h.weather_id,
            camera_angle_id: h.camera_angle_id,
            location_specifier: h.location_specifier,
            battle_background: h.battle_background,
            flags: h.flags,
        })
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        check_id(ctx.symbols, SymbolKind::Map, index, &self.id)?;
        let record = name_record(&self.internal_name, self.internal_name_bytes.as_deref())?;
        let location_name = place_of(ctx.symbols, &self.location_name)?;
        bits(self.location_specifier, 0x7F, "location_specifier")?;
        bits(self.battle_background, 0x1F, "battle_background")?;
        bits(self.flags, 0x0F, "flags")?;
        let h = MapHeader::Pt(MapHeaderPt {
            area_data_id: self.area_data_id,
            unknown1: self.unknown1,
            matrix_id: self.matrix_id,
            script_file_id: self.script_file_id,
            level_script_id: self.level_script_id,
            text_archive_id: self.text_archive_id,
            music_day_id: self.music_day_id,
            music_night_id: self.music_night_id,
            wild_pokemon: self.wild_pokemon,
            event_file_id: self.event_file_id,
            location_name,
            area_icon: self.area_icon,
            weather_id: self.weather_id,
            camera_angle_id: self.camera_angle_id,
            location_specifier: self.location_specifier,
            battle_background: self.battle_background,
            flags: self.flags,
        });
        Ok(vec![Some(write_map_header_to_bytes(&h)), Some(record)])
    }

    fn sample() -> Self {
        sample_pt(true)
    }

    fn samples() -> Vec<(String, Self)> {
        vec![(format!("{HEADERS_ART}[pt]"), sample_pt(true))]
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        vec![(format!("{HEADERS_ART}[pt]"), sample_pt(false))]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        Vec::new()
    }
}

impl EntitySchema for HeaderHgssEntity {
    const ART: &'static str = HEADERS_ART;
    const LOCKABLE: bool = false;

    /// `members` are the header's 24 bytes and its `mapname.bin` record.
    fn export(
        index: u32,
        members: &[Option<Vec<u8>>],
        ctx: &mut ExportCtx<'_>,
    ) -> Result<Self, KernelError> {
        let (header, record) = header_members(members, index)?;
        let MapHeader::HGSS(h) = decode(header, GameFamily::HGSS, index)? else {
            return Err(not_family(index, "HeartGold/SoulSilver"));
        };
        let (internal_name, internal_name_bytes) = name_json(record);
        Ok(HeaderHgssEntity {
            id: entity_id(ctx.symbols, SymbolKind::Map, index),
            internal_name,
            internal_name_bytes,
            wild_pokemon: h.wild_pokemon,
            area_data_id: h.area_data_id,
            unknown0: h.unknown0,
            worldmap_x: h.worldmap_x,
            worldmap_y: h.worldmap_y,
            matrix_id: h.matrix_id,
            script_file_id: h.script_file_id,
            level_script_id: h.level_script_id,
            text_archive_id: h.text_archive_id,
            music_day_id: h.music_day_id,
            music_night_id: h.music_night_id,
            event_file_id: h.event_file_id,
            location_name: place_ref(ctx.symbols, h.location_name),
            area_icon: h.area_icon,
            unknown1: h.unknown1,
            kanto_flag: h.kanto_flag,
            weather_id: h.weather_id,
            location_type: h.location_type,
            camera_angle_id: h.camera_angle_id,
            follow_mode: h.follow_mode,
            battle_background: h.battle_background,
            flags: h.flags,
        })
    }

    fn import(&self, index: u32, ctx: &ImportCtx<'_>) -> Result<Vec<Option<Vec<u8>>>, String> {
        check_id(ctx.symbols, SymbolKind::Map, index, &self.id)?;
        let record = name_record(&self.internal_name, self.internal_name_bytes.as_deref())?;
        let location_name = place_of(ctx.symbols, &self.location_name)?;
        for (value, max, path) in [
            (self.unknown0, 0x0F, "unknown0"),
            (self.worldmap_x, 0x3F, "worldmap_x"),
            (self.worldmap_y, 0x3F, "worldmap_y"),
            (self.area_icon, 0x0F, "area_icon"),
            (self.unknown1, 0x0F, "unknown1"),
            (self.weather_id, 0x7F, "weather_id"),
            (self.location_type, 0x0F, "location_type"),
            (self.camera_angle_id, 0x3F, "camera_angle_id"),
            (self.follow_mode, 0x03, "follow_mode"),
            (self.battle_background, 0x1F, "battle_background"),
            (self.flags, 0x7F, "flags"),
        ] {
            bits(value, max, path)?;
        }
        let h = MapHeader::HGSS(MapHeaderHGSS {
            wild_pokemon: self.wild_pokemon,
            area_data_id: self.area_data_id,
            unknown0: self.unknown0,
            worldmap_x: self.worldmap_x,
            worldmap_y: self.worldmap_y,
            matrix_id: self.matrix_id,
            script_file_id: self.script_file_id,
            level_script_id: self.level_script_id,
            text_archive_id: self.text_archive_id,
            music_day_id: self.music_day_id,
            music_night_id: self.music_night_id,
            event_file_id: self.event_file_id,
            location_name,
            area_icon: self.area_icon,
            unknown1: self.unknown1,
            kanto_flag: self.kanto_flag,
            weather_id: self.weather_id,
            location_type: self.location_type,
            camera_angle_id: self.camera_angle_id,
            follow_mode: self.follow_mode,
            battle_background: self.battle_background,
            flags: self.flags,
        });
        Ok(vec![Some(write_map_header_to_bytes(&h)), Some(record)])
    }

    fn sample() -> Self {
        sample_hgss(true)
    }

    fn samples() -> Vec<(String, Self)> {
        vec![(format!("{HEADERS_ART}[hgss]"), sample_hgss(true))]
    }

    fn minimal_samples() -> Vec<(String, Self)> {
        vec![(format!("{HEADERS_ART}[hgss]"), sample_hgss(false))]
    }

    fn locked_samples() -> Vec<(String, Self)> {
        Vec::new()
    }
}

/// The header's 24 bytes and its 16-byte name record of entity `index`.
fn header_members(members: &[Option<Vec<u8>>], index: u32) -> Result<(&[u8], &[u8]), KernelError> {
    let header = required(members, 0, HEADERS_ART, index)?;
    let record = required(members, 1, HEADERS_ART, index)?;
    if header.len() != HEADER_LEN || record.len() != NAME_LEN {
        return Err(KernelError::InvalidProject(format!(
            "headers {index:04}: {} Byte Header und {} Byte Name, erwartet {HEADER_LEN} und \
             {NAME_LEN}.",
            header.len(),
            record.len()
        )));
    }
    Ok((header, record))
}

fn decode(header: &[u8], family: GameFamily, index: u32) -> Result<MapHeader, KernelError> {
    read_map_header_from_bytes(header, family)
        .map_err(|e| KernelError::Uxie(format!("headers {index:04}: {e}")))
}

fn not_family(index: u32, family: &str) -> KernelError {
    KernelError::Uxie(format!(
        "headers {index:04}: uxie las keinen Header von {family}."
    ))
}

fn place_ref(sym: &Symbols, n: u8) -> SymRef {
    super::super::record::sym_ref(sym, SymbolKind::Place, u32::from(n))
}

/// The number of a `location_name` reference, at most 255.
fn place_of(sym: &Symbols, r: &SymRef) -> Result<u8, String> {
    resolve_max(sym, SymbolKind::Place, r, "location_name", 0xFF).map(|n| n as u8)
}

/// Whether bit field `path` fits `max`.
fn bits(value: u8, max: u8, path: &str) -> Result<(), String> {
    if value > max {
        return Err(format!(
            "{SCHEMA_CODE}: {path}: {value} liegt nicht in 0..={max}."
        ));
    }
    Ok(())
}

/// The internal name of a `mapname.bin` record (the bytes before the first NUL as Latin-1)
/// and the record as hex when the name and NUL fill do not give it.
fn name_json(record: &[u8]) -> (String, Option<String>) {
    let end = record.iter().position(|&b| b == 0).unwrap_or(record.len());
    let name = record[..end].iter().map(|&b| char::from(b)).collect();
    let filled = record[end..].iter().all(|&b| b == 0);
    (name, (!filled).then(|| hex(record)))
}

/// The `mapname.bin` record of `name` (Latin-1, at most 16 characters, no NUL), NUL filled,
/// or `bytes` when given; then `name` must be what `bytes` hold before their first NUL.
fn name_record(name: &str, bytes: Option<&str>) -> Result<Vec<u8>, String> {
    let mut record = Vec::with_capacity(NAME_LEN);
    for c in name.chars() {
        let byte = u8::try_from(u32::from(c))
            .ok()
            .filter(|&b| b != 0)
            .ok_or_else(|| {
                format!(
                    "{SCHEMA_CODE}: internal_name: {c:?} ist kein Zeichen von Latin-1 (ohne \
                     NUL); mapname.bin hat ein Byte je Zeichen."
                )
            })?;
        record.push(byte);
    }
    if record.len() > NAME_LEN {
        return Err(format!(
            "{SCHEMA_CODE}: internal_name: {} Zeichen, höchstens {NAME_LEN}.",
            record.len()
        ));
    }
    let Some(text) = bytes else {
        record.resize(NAME_LEN, 0);
        return Ok(record);
    };
    let raw = unhex(text).map_err(|e| format!("{SCHEMA_CODE}: internal_name_bytes: {e}"))?;
    if raw.len() != NAME_LEN {
        return Err(format!(
            "{SCHEMA_CODE}: internal_name_bytes: {} Byte statt {NAME_LEN}.",
            raw.len()
        ));
    }
    let (held, _) = name_json(&raw);
    if held != name {
        return Err(format!(
            "{SCHEMA_CODE}: internal_name: „{name}“ passt nicht zu internal_name_bytes \
             („{held}“); eines von beiden anpassen oder internal_name_bytes löschen."
        ));
    }
    Ok(raw)
}

fn sample_name(full: bool) -> (String, String, Option<String>) {
    (
        fallback_id(SymbolKind::Map, 0),
        "name".into(),
        full.then(|| "00".repeat(NAME_LEN)),
    )
}

fn sample_pt(full: bool) -> HeaderPtEntity {
    let (id, internal_name, internal_name_bytes) = sample_name(full);
    HeaderPtEntity {
        id,
        internal_name,
        internal_name_bytes,
        area_data_id: 0,
        unknown1: 0,
        matrix_id: 0,
        script_file_id: 0,
        level_script_id: 0,
        text_archive_id: 0,
        music_day_id: 0,
        music_night_id: 0,
        wild_pokemon: 0,
        event_file_id: 0,
        location_name: SymRef::Id(fallback_id(SymbolKind::Place, 0)),
        area_icon: 0,
        weather_id: 0,
        camera_angle_id: 0,
        location_specifier: 0,
        battle_background: 0,
        flags: 0,
    }
}

fn sample_hgss(full: bool) -> HeaderHgssEntity {
    let (id, internal_name, internal_name_bytes) = sample_name(full);
    HeaderHgssEntity {
        id,
        internal_name,
        internal_name_bytes,
        wild_pokemon: 0,
        area_data_id: 0,
        unknown0: 0,
        worldmap_x: 0,
        worldmap_y: 0,
        matrix_id: 0,
        script_file_id: 0,
        level_script_id: 0,
        text_archive_id: 0,
        music_day_id: 0,
        music_night_id: 0,
        event_file_id: 0,
        location_name: SymRef::Id(fallback_id(SymbolKind::Place, 0)),
        area_icon: 0,
        unknown1: 0,
        kanto_flag: false,
        weather_id: 0,
        location_type: 0,
        camera_angle_id: 0,
        follow_mode: 0,
        battle_background: 0,
        flags: 0,
    }
}

/// The header table as `import_headers` built it: the 24 bytes of every header in order and
/// the matching `mapname.bin`; `remove`: the headers past it go (the import's rule for headers
/// at the end without JSON, Dynamic Headers patch applied: their files and records).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HeaderWrite {
    pub table: Vec<u8>,
    pub mapname: Vec<u8>,
    pub remove: bool,
}

/// What the import says of the headers without JSON it does not remove.
const REMOVES_ONLY_HEADERS: &str = "der Import entfernt nur Header am Ende, die seit der \
                                    Aufzeichnung unverändert sind (textproject.member-removed).";

/// The header table as a whole: one canonical JSON per header (index, bytes), from the slice
/// of `arm9.bin` or the Dynamic Headers files, with the names of `mapname.bin`.
pub(crate) fn export_headers(
    project: &Project,
    ctx: &mut ExportCtx<'_>,
) -> Result<Vec<(u32, Vec<u8>)>, KernelError> {
    if project.family == Family::Dp {
        return Err(KernelError::InvalidProject(NO_DP.to_owned()));
    }
    let (table, names) = read_table(project)?;
    let mut out = Vec::with_capacity(names.len() / NAME_LEN);
    for (i, (header, record)) in table
        .chunks(HEADER_LEN)
        .zip(names.chunks(NAME_LEN))
        .enumerate()
    {
        let index = i as u32;
        let members = [Some(header.to_vec()), Some(record.to_vec())];
        let json = match project.family {
            Family::Hgss => to_canonical(&HeaderHgssEntity::export(index, &members, ctx)?),
            Family::Pt | Family::Dp => to_canonical(&HeaderPtEntity::export(index, &members, ctx)?),
        };
        out.push((index, json));
    }
    Ok(out)
}

/// The header table as on disk (the `arm9.bin` slice or the Dynamic Headers files) and
/// `mapname.bin`, one record per header.
fn read_table(project: &Project) -> Result<(Vec<u8>, Vec<u8>), KernelError> {
    let names_path = project.mapname_path();
    let names = std::fs::read(&names_path).map_err(io_at(&names_path))?;
    if !names.len().is_multiple_of(NAME_LEN) {
        return Err(KernelError::InvalidProject(format!(
            "{} hat {} Byte, kein Vielfaches von {NAME_LEN}.",
            names_path.display(),
            names.len()
        )));
    }
    Ok((project.read_header_table_bytes()?, names))
}

/// The base layer of the headers, which have no parts.
struct NoParts;

impl BasePart for NoParts {
    fn base_bytes(&self, _: PartKey) -> Result<Option<Vec<u8>>, KernelError> {
        Ok(None)
    }
}

/// An import error `<code>: <path>: <sentence>` of `file` as a [`Problem`] (line 1, column 1;
/// the caller finds the position from the path).
fn coded_problem(file: &Path, message: &str) -> Problem {
    let (code, rest) = message.split_once(": ").unwrap_or((SCHEMA_CODE, message));
    problem(file, code, rest.to_owned())
}

/// Header `index` of the file `bytes` as its members (header, name record).
fn import_one<E: EntitySchema>(
    index: u32,
    bytes: &[u8],
    file: &Path,
    ctx: &ImportCtx<'_>,
) -> Result<Vec<Option<Vec<u8>>>, Problem> {
    let (e, _deleted): (E, bool) = parse_entity(file, bytes, SCHEMA_CODE)?;
    e.import(index, ctx)
        .map_err(|message| coded_problem(file, &message))
}

/// The header table and `mapname.bin` from the header files `files` (index, JSON), checked
/// all together, nothing written: the problems name `data/headers/NNNN.json`.
pub(crate) fn import_headers(
    project: &Project,
    files: &[(u32, Vec<u8>)],
    symbols: &Symbols,
) -> Result<HeaderWrite, Vec<Problem>> {
    import_headers_with(project, files, symbols, false)
}

/// [`import_headers`]; with `remove` (the import's rule found the headers past the files
/// removable, see `import::header_removals`) fewer files than `mapname.bin` has records are no
/// error: the write removes the headers past them ([`HeaderWrite::remove`]).
pub(crate) fn import_headers_with(
    project: &Project,
    files: &[(u32, Vec<u8>)],
    symbols: &Symbols,
    remove: bool,
) -> Result<HeaderWrite, Vec<Problem>> {
    let folder = Path::new(DATA_DIR).join(HEADERS_ART);
    let file_of = |i: u32| folder.join(format!("{i:04}.json"));
    if project.family == Family::Dp {
        return Err(vec![problem(
            &folder,
            SCHEMA_CODE,
            format!("{ROOT}: {NO_DP}"),
        )]);
    }
    let mut problems = Vec::new();
    let mut sorted: Vec<&(u32, Vec<u8>)> = files.iter().collect();
    sorted.sort_by_key(|(i, _)| *i);
    let mut next = 0u32;
    for (i, _) in &sorted {
        if *i < next {
            problems.push(problem(
                &file_of(*i),
                HEADER_COUNT_CODE,
                format!("{ROOT}: die Nummer {i:04} kommt doppelt vor."),
            ));
        } else if *i > next {
            problems.push(problem(
                &file_of(next),
                HEADER_COUNT_CODE,
                format!(
                    "{ROOT}: die Datei fehlt; die Header sind lückenlos von 0000 an nummeriert."
                ),
            ));
        }
        next = next.max(i.saturating_add(1));
    }

    let count = sorted.len();
    let mut removes = false;
    let rel = |p: &Path| p.strip_prefix(&project.root).unwrap_or(p).to_path_buf();
    let state = dynamic_headers::is_applied(project)
        .and_then(|dynamic| Ok((dynamic, project.mapname_count()?)));
    match state {
        Err(e) => problems.push(problem(
            &rel(&project.mapname_path()),
            HEADER_TABLE_CODE,
            format!("{ROOT}: die Zahl der Header ist nicht lesbar ({e})."),
        )),
        Ok((dynamic, current)) => {
            if !dynamic && count != current {
                problems.push(problem(
                    &file_of(count.min(current) as u32),
                    HEADER_COUNT_CODE,
                    format!(
                        "{ROOT}: Die Zahl der Header ändert sich nur mit dem Patch „Dynamische \
                         Header“. mapname.bin hat {current} Einträge, data/headers {count} \
                         Dateien."
                    ),
                ));
            } else if count < current && remove {
                removes = true;
            } else if count < current {
                problems.push(problem(
                    &file_of(count as u32),
                    MEMBER_WITHOUT_JSON_CODE,
                    format!(
                        "{ROOT}: Header {count:04} hat keine Datei (mapname.bin hat {current} \
                         Einträge, data/headers {count} Dateien); {REMOVES_ONLY_HEADERS}"
                    ),
                ));
            } else if dynamic && count > MAX_HEADERS {
                problems.push(problem(
                    &file_of(MAX_HEADERS as u32),
                    HEADER_COUNT_CODE,
                    format!("{ROOT}: {count} Header, höchstens {MAX_HEADERS}."),
                ));
            }
            // Without the patch the table is a slice of arm9.bin, which must hold it. With
            // it, a missing `unpacked/dynamicHeaders` is fine: the headers own that folder
            // and `write_headers` writes every file.
            if !dynamic && let Err(e) = project.member_location(AssetKind::HeaderTable, 0) {
                problems.push(problem(
                    &rel(&project.arm9_path()),
                    HEADER_TABLE_CODE,
                    format!("{ROOT}: die Header-Tabelle ist nicht lesbar ({e})."),
                ));
            }
        }
    }

    let dir = project.root.join(&folder);
    let ctx = ImportCtx {
        family: project.family,
        symbols,
        dir: &dir,
        base: &NoParts,
        current: &[],
    };
    let mut table = Vec::with_capacity(count * HEADER_LEN);
    let mut mapname = Vec::with_capacity(count * NAME_LEN);
    for (i, bytes) in sorted {
        let file = file_of(*i);
        let members = match project.family {
            Family::Hgss => import_one::<HeaderHgssEntity>(*i, bytes, &file, &ctx),
            Family::Pt | Family::Dp => import_one::<HeaderPtEntity>(*i, bytes, &file, &ctx),
        };
        match members {
            Ok(members) => {
                for (out, m) in [&mut table, &mut mapname].into_iter().zip(members) {
                    out.extend_from_slice(m.as_deref().unwrap_or_default());
                }
            }
            Err(p) => problems.push(p),
        }
    }
    if problems.is_empty() {
        Ok(HeaderWrite {
            table,
            mapname,
            remove: removes,
        })
    } else {
        Err(problems)
    }
}

/// Writes `w` where the project keeps its header table and `mapname.bin`, each file only when
/// it changes; the project-relative paths it wrote. With the Dynamic Headers patch a missing
/// `unpacked/dynamicHeaders` is created with every header file, and a table shorter than
/// `mapname.bin` is refused (a header leaves the table only through „Nummer freigeben“, which
/// shortens `mapname.bin` with it, [`crate::delete::free_number`]); without it the table must
/// have the length of the slice in `arm9.bin`.
pub(crate) fn write_headers(
    project: &Project,
    w: &HeaderWrite,
) -> Result<Vec<PathBuf>, KernelError> {
    let count = w.table.len() / HEADER_LEN;
    if w.table.len() != count * HEADER_LEN || w.mapname.len() != count * NAME_LEN {
        return Err(KernelError::InvalidProject(format!(
            "{HEADER_COUNT_CODE}: {} Byte Header-Tabelle und {} Byte mapname.bin passen nicht \
             zusammen.",
            w.table.len(),
            w.mapname.len()
        )));
    }
    let root = &project.root;
    let rel = |p: &Path| p.strip_prefix(root).unwrap_or(p).to_path_buf();
    let names_rel = rel(&project.mapname_path());
    let mut wrote = Vec::new();
    if dynamic_headers::is_applied(project)? {
        let old = project.mapname_count()?;
        if count < old && !w.remove {
            return Err(KernelError::InvalidProject(format!(
                "{MEMBER_WITHOUT_JSON_CODE}: {ROOT}: {count} Header statt {old}; \
                 {REMOVES_ONLY_HEADERS}"
            )));
        }
        let dir_rel = rel(&dynamic_headers::unpacked_dir(project));
        for (i, chunk) in w.table.chunks(HEADER_LEN).enumerate() {
            let file = dir_rel.join(format!("{i:04}"));
            if write_if_changed(root, &file, chunk)? {
                wrote.push(file);
            }
        }
        if write_if_changed(root, &names_rel, &w.mapname)? {
            wrote.push(names_rel);
        }
        // The headers past the table go after `mapname.bin` lost their records.
        for i in count..old {
            let file = dir_rel.join(format!("{i:04}"));
            match std::fs::remove_file(root.join(&file)) {
                Ok(()) => wrote.push(file),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_at(&root.join(&file))(e)),
            }
        }
        return Ok(wrote);
    }
    match project.member_location(AssetKind::HeaderTable, 0)? {
        Location::Slice { path, offset, len } => {
            if len != w.table.len() {
                return Err(KernelError::InvalidProject(format!(
                    "{HEADER_COUNT_CODE}: die Tabelle in arm9.bin hat {} Header, geschrieben \
                     würden {count}.",
                    len / HEADER_LEN
                )));
            }
            let mut arm9 = std::fs::read(&path).map_err(io_at(&path))?;
            let at = usize::try_from(offset).unwrap_or(usize::MAX);
            let slice = arm9.get_mut(at..at.saturating_add(len)).ok_or_else(|| {
                KernelError::InvalidProject(format!(
                    "{} endet vor der Header-Tabelle.",
                    path.display()
                ))
            })?;
            if *slice != *w.table {
                slice.copy_from_slice(&w.table);
                write_atomic(root, &path, &arm9)?;
                wrote.push(rel(&path));
            }
            if write_if_changed(root, &names_rel, &w.mapname)? {
                wrote.push(names_rel);
            }
        }
        Location::HeaderFiles { .. }
        | Location::Member(_)
        | Location::TextSource { .. }
        | Location::MemberPair { .. }
        | Location::Derived { .. }
        | Location::Entity { .. } => {
            return Err(KernelError::InvalidProject(
                "the header table is not a slice of arm9.bin".to_owned(),
            ));
        }
    }
    Ok(wrote)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::super::super::canon::to_canonical;
    use super::super::testkit::{code_and_path, export, export_with, import, import_with, symbols};
    use super::*;

    fn name(text: &[u8]) -> Vec<u8> {
        let mut rec = vec![0u8; NAME_LEN];
        rec[..text.len()].copy_from_slice(text);
        rec
    }

    fn pt_header() -> Vec<u8> {
        uxie::map_header::write_map_header_to_bytes(&MapHeader::Pt(MapHeaderPt {
            area_data_id: 3,
            matrix_id: 0,
            script_file_id: 10,
            level_script_id: 11,
            text_archive_id: 12,
            music_day_id: 1000,
            music_night_id: 1001,
            wild_pokemon: 0xFFFF,
            event_file_id: 9,
            location_name: 5,
            area_icon: 2,
            location_specifier: 100,
            battle_background: 20,
            flags: 9,
            ..MapHeaderPt::default()
        }))
    }

    #[test]
    fn header_location_is_a_place_ref() {
        let mut sym = symbols();
        let places: Vec<Option<String>> =
            (0..6).map(|i| (i == 5).then(|| "X".to_owned())).collect();
        sym.registry_mut(SymbolKind::Place).baptize(&places, false);
        let members = vec![Some(pt_header()), Some(name(b"T01"))];
        let (e, side) = export_with::<HeaderPtEntity>(Family::Pt, 4, &members, &mut sym);
        assert!(side.is_empty());
        assert_eq!(e.location_name, SymRef::Id("PLACE_X".into()));
        assert_eq!(e.id, "MAP_0004");
        assert_eq!(e.internal_name, "T01");
        assert_eq!(e.internal_name_bytes, None);
        let v = serde_json::to_value(&e).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys[..4],
            ["id", "internal_name", "area_data_id", "unknown1"]
        );
        assert_eq!(keys.last().unwrap(), "flags");
        assert_eq!(v["location_specifier"], json!(100));
        assert_eq!(import_with(Family::Pt, &e, 4, &sym).unwrap(), members);

        // A number reads as well; an unknown id or one beyond a byte names the field.
        let mut by_number = e.clone();
        by_number.location_name = SymRef::Num(5);
        assert_eq!(
            import_with(Family::Pt, &by_number, 4, &sym).unwrap(),
            members
        );
        let mut bad = e.clone();
        bad.location_name = SymRef::Id("PLACE_Y".into());
        let err = import_with(Family::Pt, &bad, 4, &sym).unwrap_err();
        assert_eq!(
            code_and_path(&err),
            ("textproject.unknown-id", "location_name")
        );
        bad.location_name = SymRef::Num(256);
        let err = import_with(Family::Pt, &bad, 4, &sym).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", "location_name"));
        // Without the register the place is a number.
        let (e, _) = export::<HeaderPtEntity>(Family::Pt, 4, &members);
        assert_eq!(e.location_name, SymRef::Num(5));
    }

    #[test]
    fn header_import_checks_id_bits_and_name() {
        let members = vec![Some(pt_header()), Some(name(b"T01"))];
        let (good, _) = export::<HeaderPtEntity>(Family::Pt, 4, &members);
        let err_of = |f: &dyn Fn(&mut HeaderPtEntity)| {
            let mut e = good.clone();
            f(&mut e);
            import(Family::Pt, &e, 4).unwrap_err()
        };
        let err = err_of(&|e| e.id = "MAP_0005".into());
        assert_eq!(code_and_path(&err), ("textproject.id-mismatch", "id"));
        let err = err_of(&|e| e.location_specifier = 128);
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "location_specifier")
        );
        let err = err_of(&|e| e.battle_background = 32);
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "battle_background")
        );
        let err = err_of(&|e| e.flags = 16);
        assert_eq!(code_and_path(&err), ("textproject.schema", "flags"));
        let err = err_of(&|e| e.internal_name = "A".repeat(17));
        assert_eq!(code_and_path(&err), ("textproject.schema", "internal_name"));
        let err = err_of(&|e| e.internal_name = "Ā".into());
        assert_eq!(code_and_path(&err), ("textproject.schema", "internal_name"));
        let err = err_of(&|e| e.internal_name = "A\0B".into());
        assert_eq!(code_and_path(&err), ("textproject.schema", "internal_name"));
        let err = err_of(&|e| e.internal_name_bytes = Some("00".repeat(15)));
        assert_eq!(
            code_and_path(&err),
            ("textproject.schema", "internal_name_bytes")
        );
        let err = err_of(&|e| e.internal_name_bytes = Some(hex(&name(b"T02"))));
        assert_eq!(code_and_path(&err), ("textproject.schema", "internal_name"));
        assert!(err.contains("internal_name_bytes"), "{err}");

        // Latin-1 names: 16 characters, each one byte.
        let mut e = good.clone();
        e.internal_name = "ÄBCDEFGHIJKLMNOP".into();
        let back = import(Family::Pt, &e, 4).unwrap();
        let mut rec = b"\xC4BCDEFGHIJKLMNOP".to_vec();
        assert_eq!(back[1].as_deref(), Some(rec.as_slice()));
        rec[3] = 0;
        let (e, _) = export::<HeaderPtEntity>(Family::Pt, 4, &[members[0].clone(), Some(rec)]);
        assert_eq!(e.internal_name, "ÄBC");
        assert!(e.internal_name_bytes.is_some());

        // HeartGold/SoulSilver bit fields have their own widths.
        let hgss = uxie::map_header::write_map_header_to_bytes(&MapHeader::HGSS(MapHeaderHGSS {
            worldmap_x: 63,
            follow_mode: 3,
            kanto_flag: true,
            flags: 127,
            ..MapHeaderHGSS::default()
        }));
        let members = vec![Some(hgss), Some(name(b"R01"))];
        let (e, _) = export::<HeaderHgssEntity>(Family::Hgss, 0, &members);
        assert!(e.kanto_flag);
        assert_eq!(import(Family::Hgss, &e, 0).unwrap(), members);
        let mut bad = e.clone();
        bad.follow_mode = 4;
        let err = import(Family::Hgss, &bad, 0).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", "follow_mode"));
        let mut bad = e.clone();
        bad.worldmap_y = 64;
        let err = import(Family::Hgss, &bad, 0).unwrap_err();
        assert_eq!(code_and_path(&err), ("textproject.schema", "worldmap_y"));
    }

    proptest! {
        #[test]
        fn prop_header_round_trip(
            hgss in any::<bool>(),
            header in proptest::collection::vec(any::<u8>(), HEADER_LEN),
            text in proptest::collection::vec(1u8..=255, 0..=NAME_LEN),
            fill in proptest::collection::vec(any::<u8>(), NAME_LEN),
            garbage in any::<bool>(),
        ) {
            let mut rec = text.clone();
            if rec.len() < NAME_LEN {
                rec.push(0);
                let rest = NAME_LEN - rec.len();
                if garbage {
                    rec.extend_from_slice(&fill[..rest]);
                } else {
                    rec.resize(NAME_LEN, 0);
                }
            }
            let members = vec![Some(header), Some(rec)];
            let back = if hgss {
                let (e, _) = export::<HeaderHgssEntity>(Family::Hgss, 7, &members);
                let again: HeaderHgssEntity = serde_json::from_slice(&to_canonical(&e)).unwrap();
                import(Family::Hgss, &again, 7).unwrap()
            } else {
                let (e, _) = export::<HeaderPtEntity>(Family::Pt, 7, &members);
                let again: HeaderPtEntity = serde_json::from_slice(&to_canonical(&e)).unwrap();
                import(Family::Pt, &again, 7).unwrap()
            };
            prop_assert_eq!(back, members);
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
#[allow(clippy::unwrap_used)]
mod project_tests {
    use serde_json::{Value, json};

    use super::super::super::canon::to_canonical;
    use super::super::super::part::tests::MemBase;
    use super::super::testkit::symbols;
    use super::*;
    use crate::dynamic_headers;
    use crate::test_support::{PT_HEADER_TABLE_CPUE, synthetic_project};

    /// The symbols with the map register baptized from the project's internal names.
    fn symbols_of(project: &Project) -> Symbols {
        let mut sym = symbols();
        let names: Vec<Option<String>> = project
            .internal_names()
            .unwrap()
            .into_iter()
            .map(Some)
            .collect();
        sym.registry_mut(SymbolKind::Map).baptize(&names, false);
        sym
    }

    fn export_all(project: &Project, sym: &mut Symbols) -> Vec<(u32, Vec<u8>)> {
        let base = MemBase::default();
        let mut ctx = ExportCtx {
            family: project.family,
            symbols: sym,
            base: &base,
            side: Vec::new(),
            egg_moves: None,
        };
        let files = export_headers(project, &mut ctx).unwrap();
        assert!(ctx.side.is_empty());
        files
    }

    fn value(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).unwrap()
    }

    /// File `i` of `files` with `f` applied to its JSON.
    fn edit(files: &mut [(u32, Vec<u8>)], i: usize, f: impl FnOnce(&mut Value)) {
        let mut v = value(&files[i].1);
        f(&mut v);
        files[i].1 = to_canonical(&v);
    }

    fn table_slice(project: &Project, count: usize) -> Vec<u8> {
        let arm9 = std::fs::read(project.root.join("arm9/arm9.bin")).unwrap();
        arm9[PT_HEADER_TABLE_CPUE..PT_HEADER_TABLE_CPUE + count * HEADER_LEN].to_vec()
    }

    #[test]
    fn headers_round_trip_through_the_arm9_slice() {
        let s = synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();
        let mut sym = symbols_of(&project);
        let mut files = export_all(&project, &mut sym);
        assert_eq!(files.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [0, 1]);
        let h0 = value(&files[0].1);
        assert_eq!(h0["id"], json!("MAP_T01"));
        assert_eq!(h0["internal_name"], json!("T01"));
        assert!(h0.get("internal_name_bytes").is_none());
        assert_eq!(h0["text_archive_id"], json!(433));
        assert_eq!(h0["location_name"], json!(1));
        assert_eq!(value(&files[1].1)["id"], json!("MAP_T02"));
        assert_eq!(value(&files[1].1)["location_name"], json!(200));
        assert!(files[0].1.ends_with(b"}\n"));

        let mapname_path = project.mapname_path();
        let w = import_headers(&project, &files, &sym).unwrap();
        assert_eq!(w.table, table_slice(&project, 2));
        assert_eq!(w.mapname, std::fs::read(&mapname_path).unwrap());
        assert!(
            write_headers(&project, &w).unwrap().is_empty(),
            "nothing changed"
        );

        // An edited header and a renamed map land in the slice and in mapname.bin.
        let arm9_before = std::fs::read(project.root.join("arm9/arm9.bin")).unwrap();
        edit(&mut files, 1, |v| {
            v["music_day_id"] = json!(1234);
            v["internal_name"] = json!("T02X");
        });
        let w = import_headers(&project, &files, &sym).unwrap();
        let wrote = write_headers(&project, &w).unwrap();
        assert_eq!(
            wrote,
            [
                PathBuf::from("arm9/arm9.bin"),
                PathBuf::from("files/fielddata/maptable/mapname.bin")
            ]
        );
        let headers = project.read_headers().unwrap();
        let MapHeader::Pt(h) = &headers[1] else {
            panic!()
        };
        assert_eq!(h.music_day_id, 1234);
        assert_eq!(project.internal_names().unwrap()[1], "T02X");
        let arm9_after = std::fs::read(project.root.join("arm9/arm9.bin")).unwrap();
        let changed: Vec<usize> = (0..arm9_after.len())
            .filter(|&i| arm9_after[i] != arm9_before[i])
            .collect();
        let at = PT_HEADER_TABLE_CPUE + HEADER_LEN;
        assert!(
            changed.iter().all(|&i| (at..at + HEADER_LEN).contains(&i)),
            "{changed:?}"
        );
        // The id stays: renaming the map does not rename its id.
        let again = export_all(&project, &mut sym);
        assert_eq!(value(&again[1].1)["id"], json!("MAP_T02"));
        assert_eq!(again, files);
    }

    #[test]
    fn headers_round_trip_through_dynamic_header_files() {
        let s = synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();
        dynamic_headers::apply(&project, &project.read_headers().unwrap()).unwrap();
        assert!(dynamic_headers::is_applied(&project).unwrap());
        let arm9 = std::fs::read(project.root.join("arm9/arm9.bin")).unwrap();
        let mut sym = symbols_of(&project);
        let mut files = export_all(&project, &mut sym);
        assert_eq!(files.len(), 2);
        let w = import_headers(&project, &files, &sym).unwrap();
        let dir = dynamic_headers::unpacked_dir(&project);
        assert_eq!(
            w.table[..HEADER_LEN],
            std::fs::read(dir.join("0000")).unwrap()
        );
        assert!(write_headers(&project, &w).unwrap().is_empty());

        // A third header: a new file and a new name, arm9.bin untouched.
        let mut third = value(&files[1].1);
        third["id"] = json!("MAP_0002");
        third["internal_name"] = json!("T03");
        third["music_day_id"] = json!(77);
        files.push((2, to_canonical(&third)));
        let w = import_headers(&project, &files, &sym).unwrap();
        let wrote = write_headers(&project, &w).unwrap();
        assert_eq!(
            wrote,
            [
                PathBuf::from("unpacked/dynamicHeaders/0002"),
                PathBuf::from("files/fielddata/maptable/mapname.bin")
            ]
        );
        assert_eq!(project.header_count().unwrap(), 3);
        let MapHeader::Pt(h) = &project.read_headers().unwrap()[2] else {
            panic!()
        };
        assert_eq!(h.music_day_id, 77);
        assert_eq!(project.internal_names().unwrap()[2], "T03");
        assert_eq!(
            std::fs::read(project.root.join("arm9/arm9.bin")).unwrap(),
            arm9
        );

        // Back to two is refused (only „Nummer freigeben“ shortens the table), and nothing is
        // written.
        let three = import_headers(&project, &files, &sym).unwrap();
        files.pop();
        let problems = import_headers(&project, &files, &sym).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].file, Path::new("data/headers/0002.json"));
        assert!(
            problems[0]
                .message
                .starts_with("textproject.member-without-json: $: "),
            "{}",
            problems[0].message
        );
        assert!(
            problems[0].message.ends_with(REMOVES_ONLY_HEADERS),
            "{}",
            problems[0].message
        );
        let two = HeaderWrite {
            table: three.table[..2 * HEADER_LEN].to_vec(),
            mapname: three.mapname[..2 * NAME_LEN].to_vec(),
            remove: false,
        };
        let err = write_headers(&project, &two).unwrap_err().to_string();
        assert!(err.contains("textproject.member-without-json"), "{err}");
        assert!(dir.join("0002").exists());
        assert_eq!(project.header_count().unwrap(), 3);
    }

    #[test]
    fn missing_dynamic_header_folder_is_recreated() {
        let s = synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();
        dynamic_headers::apply(&project, &project.read_headers().unwrap()).unwrap();
        let mut sym = symbols_of(&project);
        let files = export_all(&project, &mut sym);
        let dir = dynamic_headers::unpacked_dir(&project);
        let before: Vec<Vec<u8>> = ["0000", "0001"]
            .iter()
            .map(|f| std::fs::read(dir.join(f)).unwrap())
            .collect();
        // The state after a clone: the patch is in arm9.bin, the folder is not there.
        std::fs::remove_dir_all(&dir).unwrap();
        let w = import_headers(&project, &files, &sym).unwrap();
        let wrote = write_headers(&project, &w).unwrap();
        assert_eq!(
            wrote,
            [
                PathBuf::from("unpacked/dynamicHeaders/0000"),
                PathBuf::from("unpacked/dynamicHeaders/0001")
            ]
        );
        for (i, bytes) in before.iter().enumerate() {
            assert_eq!(&std::fs::read(dir.join(format!("{i:04}"))).unwrap(), bytes);
        }
        assert_eq!(export_all(&project, &mut sym), files);
    }

    #[test]
    fn an_empty_header_folder_is_refused() {
        for patched in [false, true] {
            let s = synthetic_project();
            let project = Project::open(s.dir.path()).unwrap();
            if patched {
                dynamic_headers::apply(&project, &project.read_headers().unwrap()).unwrap();
            }
            let problems = import_headers(&project, &[], &symbols_of(&project)).unwrap_err();
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!(problems[0].file, Path::new("data/headers/0000.json"));
            let message = &problems[0].message;
            if patched {
                assert!(
                    message.starts_with("textproject.member-without-json: $: ")
                        && message.ends_with(REMOVES_ONLY_HEADERS),
                    "{message}"
                );
            } else {
                assert!(
                    message.starts_with(
                        "textproject.header-count: $: Die Zahl der Header ändert sich nur mit \
                         dem Patch „Dynamische Header“."
                    ),
                    "{message}"
                );
            }
        }
    }

    #[test]
    fn a_header_table_beyond_arm9_is_a_problem() {
        let s = synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();
        let mut sym = symbols_of(&project);
        let files = export_all(&project, &mut sym);
        let arm9 = project.root.join("arm9/arm9.bin");
        let mut bytes = std::fs::read(&arm9).unwrap();
        bytes.truncate(PT_HEADER_TABLE_CPUE + HEADER_LEN);
        std::fs::write(&arm9, &bytes).unwrap();
        let problems = import_headers(&project, &files, &sym).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].file, Path::new("arm9/arm9.bin"));
        assert!(
            problems[0]
                .message
                .starts_with("textproject.header-table: $: "),
            "{}",
            problems[0].message
        );
    }

    #[test]
    fn mapname_garbage_after_the_nul_is_kept() {
        let s = synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();
        let path = project.mapname_path();
        let mut names = std::fs::read(&path).unwrap();
        names[NAME_LEN + 5] = 0xAB;
        names[NAME_LEN + 15] = b'Z';
        std::fs::write(&path, &names).unwrap();
        let mut sym = symbols_of(&project);
        let files = export_all(&project, &mut sym);
        let h1 = value(&files[1].1);
        assert_eq!(h1["internal_name"], json!("T02"));
        assert_eq!(
            h1["internal_name_bytes"],
            json!(hex(&names[NAME_LEN..2 * NAME_LEN]))
        );
        let keys: Vec<_> = h1.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys[..3], ["id", "internal_name", "internal_name_bytes"]);
        assert!(value(&files[0].1).get("internal_name_bytes").is_none());
        let w = import_headers(&project, &files, &sym).unwrap();
        assert_eq!(w.mapname, names);
        assert!(write_headers(&project, &w).unwrap().is_empty());
    }

    #[test]
    fn header_count_change_without_dynamic_headers_is_refused() {
        let s = synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();
        let mut sym = symbols_of(&project);
        let mut files = export_all(&project, &mut sym);
        let mut third = value(&files[1].1);
        third["id"] = json!("MAP_0002");
        files.push((2, to_canonical(&third)));
        let problems = import_headers(&project, &files, &sym).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].message.starts_with(
                "textproject.header-count: $: Die Zahl der Header ändert sich nur mit dem Patch \
                 „Dynamische Header“."
            ),
            "{}",
            problems[0].message
        );

        // Fewer headers are a count change too.
        files.truncate(1);
        let problems = import_headers(&project, &files, &sym).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].message.starts_with(
                "textproject.header-count: $: Die Zahl der Header ändert sich nur mit dem Patch \
                 „Dynamische Header“."
            ),
            "{}",
            problems[0].message
        );
        // The writer refuses a table of another length as well.
        let w = HeaderWrite {
            table: vec![0; HEADER_LEN],
            mapname: vec![0; NAME_LEN],
            remove: false,
        };
        let err = write_headers(&project, &w).unwrap_err().to_string();
        assert!(err.contains("textproject.header-count"), "{err}");

        // A gap in the numbers is refused with the file that is missing.
        let mut gap = export_all(&project, &mut sym);
        gap[1].0 = 2;
        let problems = import_headers(&project, &gap, &sym).unwrap_err();
        assert!(
            problems
                .iter()
                .any(|p| p.file == Path::new("data/headers/0001.json")
                    && p.message.starts_with("textproject.header-count: ")),
            "{problems:?}"
        );
    }

    #[test]
    fn header_import_reports_every_file() {
        let s = synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();
        let mut sym = symbols_of(&project);
        let mut files = export_all(&project, &mut sym);
        edit(&mut files, 0, |v| v["flags"] = json!(16));
        files[1].1 = b"{\n  \"id\": \"MAP_T02\",\n  \"internal_nam\": 1\n}\n".to_vec();
        let problems = import_headers(&project, &files, &sym).unwrap_err();
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert_eq!(problems[0].file, Path::new("data/headers/0000.json"));
        assert!(
            problems[0]
                .message
                .starts_with("textproject.schema: flags: "),
            "{}",
            problems[0].message
        );
        assert_eq!(problems[1].file, Path::new("data/headers/0001.json"));
        assert_eq!(problems[1].line, 3);
        assert!(
            problems[1].message.contains("internal_nam"),
            "{}",
            problems[1].message
        );
    }
}
