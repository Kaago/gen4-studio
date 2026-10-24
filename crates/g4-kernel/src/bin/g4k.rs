//! `g4k`: headless command line over the kernel, for scripting, CI and debugging.

use std::collections::HashMap;
use std::fmt;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use ds_rom::rom::{FileSystem, raw};
use g4_formats::data::learnset::Learnset;
use g4_formats::data::moves::Move;
use g4_formats::data::record::{FieldKind, RawRecord};
use g4_formats::game::{Family, Language};
use g4_formats::map::Cell;
use g4_formats::narc::Narc;
use g4_kernel::build::{build, extract_rom};
use g4_kernel::error::KernelError;
use g4_kernel::project::{ArchiveMap, AssetKind, Project};
use g4_kernel::store::Store;
use g4_kernel::text::Problem;

#[derive(Parser)]
#[command(name = "g4k", about = "Gen-4 world editor kernel CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print game code, family, language and header count of a project.
    Info { project: PathBuf },
    /// List the map headers: id, location name, internal name and matrix.
    Headers { project: PathBuf },
    /// Extract a .nds ROM into a new ds-rom project folder.
    Extract { rom: PathBuf, dest: PathBuf },
    /// Map inspection and editing.
    #[command(subcommand)]
    Map(MapCommand),
    /// Pack the unpacked archives and build a ROM.
    Build {
        project: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Read data members from a built ROM's archives and compare them with expected values.
    /// Personal data and items are read with uxie, moves and learnsets with this project's
    /// codecs (uxie rejects 172 of 471 vanilla moves). Exit 1 when any expectation fails.
    CheckData {
        rom: PathBuf,
        /// `KIND:MEMBER:FIELD=VALUE`, for example `species:397:attack=85`. KIND is species,
        /// move, item or learnset; MEMBER the member index in the archive (item: the data
        /// member, not the item id); a learnset has the field `len` (number of entries).
        #[arg(long = "expect", required = true, value_parser = parse_expectation)]
        expect: Vec<Expectation>,
    },
}

#[derive(Subcommand)]
enum MapCommand {
    /// Print a map's buildings and permissions as JSON.
    Dump {
        project: PathBuf,
        index: u16,
        #[arg(long)]
        json: bool,
    },
    /// Set the permission cell at (x, y) and save.
    SetPerm {
        project: PathBuf,
        index: u16,
        x: usize,
        y: usize,
        #[arg(long)]
        kind: u8,
        #[arg(long)]
        collision: u8,
    },
}

/// Why a command failed: a kernel error, or a message of the command itself.
enum Failure {
    Kernel(KernelError),
    Message(String),
}

impl From<KernelError> for Failure {
    fn from(e: KernelError) -> Self {
        Failure::Kernel(e)
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Kernel(e)) => {
            if let KernelError::Compile(problems) = &e {
                print_problems(problems);
            }
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
        Err(Failure::Message(m)) => {
            eprintln!("error: {m}");
            ExitCode::from(1)
        }
    }
}

fn print_problems(problems: &[Problem]) {
    for problem in problems {
        eprintln!(
            "{:?}: {}:{}:{}: {}",
            problem.severity,
            problem.file.display(),
            problem.line,
            problem.col,
            problem.message
        );
    }
}

fn run(cli: Cli) -> Result<(), Failure> {
    match cli.command {
        Command::Info { project } => Ok(info(&project)?),
        Command::Headers { project } => Ok(headers(&project)?),
        Command::Extract { rom, dest } => {
            let p = extract_rom(&rom, &dest)?;
            println!("extracted {} to {}", p.game_code, p.root.display());
            Ok(())
        }
        Command::Map(MapCommand::Dump {
            project,
            index,
            json: _,
        }) => Ok(dump(&project, index)?),
        Command::Map(MapCommand::SetPerm {
            project,
            index,
            x,
            y,
            kind,
            collision,
        }) => Ok(set_perm(&project, index, x, y, Cell { kind, collision })?),
        Command::Build { project, out } => {
            let project = Project::open(project)?;
            let report = build(&project, &out)?;
            let mut line = String::from("packed");
            for (kind, count) in &report.packed {
                line.push_str(&format!(
                    " {}={count}",
                    project.archives.unpacked_name(*kind)
                ));
            }
            println!("{line}");
            if !report.compiled.is_empty() {
                println!("compiled={:?}", report.compiled);
            }
            if !report.encoded.is_empty() {
                println!("encoded={:?}", report.encoded);
            }
            print_problems(&report.problems);
            println!("rom={}", report.rom.display());
            Ok(())
        }
        Command::CheckData { rom, expect } => check_data(&rom, &expect),
    }
}

fn info(path: &PathBuf) -> Result<(), KernelError> {
    let project = Project::open(path)?;
    let headers = match project.headers() {
        Ok(h) => h.len().to_string(),
        Err(e) => {
            if let KernelError::Compile(problems) = &e {
                print_problems(problems);
            }
            eprintln!("error: {e}");
            "?".to_owned()
        }
    };
    println!(
        "game_code={} family={:?} language={:?} headers={headers}",
        project.game_code, project.family, project.language
    );
    Ok(())
}

fn headers(path: &PathBuf) -> Result<(), KernelError> {
    let project = Project::open(path)?;
    let listing = project.header_listing()?;
    if let Some(e) = &listing.names_error {
        eprintln!("error: location names unreadable: {e}");
    }
    for h in listing.headers {
        println!(
            "{:04}  {}  {}  M{}",
            h.id, h.location_name, h.internal_name, h.matrix_id
        );
    }
    Ok(())
}

fn dump(path: &PathBuf, index: u16) -> Result<(), KernelError> {
    let project = Project::open(path)?;
    let mut store = Store::new(Arc::new(project));
    let map = store.map(index)?;
    let buildings: Vec<_> = map
        .buildings
        .iter()
        .map(|b| {
            serde_json::json!({
                "model_id": b.model_id,
                "position": b.position,
                "rotation": b.rotation,
                "scale": b.scale,
            })
        })
        .collect();
    let permissions: Vec<_> = map
        .permissions
        .iter()
        .map(|c| [c.kind, c.collision])
        .collect();
    let value = serde_json::json!({
        "index": index,
        "buildings": buildings,
        "permissions": permissions,
    });
    println!("{value}");
    Ok(())
}

fn set_perm(path: &PathBuf, index: u16, x: usize, y: usize, cell: Cell) -> Result<(), KernelError> {
    let project = Project::open(path)?;
    let mut store = Store::new(Arc::new(project));
    store.edit_map(index, "set permission", |m| m.set_cell(x, y, cell))?;
    let report = store.save()?;
    println!("saved {} file(s)", report.written.len());
    Ok(())
}

/// The data archives `check-data` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DataKind {
    Species,
    Move,
    Item,
    Learnset,
}

/// Fields of a species, read with `uxie::PersonalData` (names as in `Personal::FIELDS`).
const SPECIES_FIELDS: &[&str] = &[
    "hp",
    "attack",
    "defense",
    "speed",
    "sp_attack",
    "sp_defense",
    "type1",
    "type2",
    "catch_rate",
    "base_exp",
    "ev_yield",
    "item1",
    "item2",
    "gender_ratio",
    "egg_cycles",
    "base_friendship",
    "growth_rate",
    "egg_group1",
    "egg_group2",
    "ability1",
    "ability2",
    "flee_rate",
    "color_flip",
];

/// Fields of an item, read with `uxie::ItemData` (names as in `Item::FIELDS`).
const ITEM_FIELDS: &[&str] = &[
    "price",
    "hold_effect",
    "hold_param",
    "pluck_effect",
    "fling_effect",
    "fling_power",
    "natural_gift_power",
    "natural_gift_type",
    "prevent_toss",
    "can_register",
    "field_pocket",
    "battle_pocket",
    "field_use_func",
    "battle_use_func",
    "party_use",
];

impl DataKind {
    fn name(self) -> &'static str {
        match self {
            DataKind::Species => "species",
            DataKind::Move => "move",
            DataKind::Item => "item",
            DataKind::Learnset => "learnset",
        }
    }

    fn asset_kind(self) -> AssetKind {
        match self {
            DataKind::Species => AssetKind::Personal,
            DataKind::Move => AssetKind::Move,
            DataKind::Item => AssetKind::Item,
            DataKind::Learnset => AssetKind::Learnset,
        }
    }

    /// The fields an expectation may name.
    fn fields(self) -> Vec<&'static str> {
        match self {
            DataKind::Species => SPECIES_FIELDS.to_vec(),
            DataKind::Item => ITEM_FIELDS.to_vec(),
            DataKind::Move => Move::FIELDS
                .iter()
                .filter(|f| f.kind != FieldKind::Bytes)
                .map(|f| f.name)
                .collect(),
            DataKind::Learnset => vec!["len"],
        }
    }

    /// The value of `field` in the member `bytes`.
    fn read(self, bytes: &[u8], field: &str) -> Result<i64, String> {
        match self {
            DataKind::Species => species_field(bytes, field),
            DataKind::Item => item_field(bytes, field),
            DataKind::Move => move_field(bytes, field),
            DataKind::Learnset => {
                let learnset = Learnset::read(bytes).map_err(|e| e.to_string())?;
                match field {
                    "len" => Ok(learnset.entries.len() as i64),
                    _ => Err(format!("learnset has no field {field}")),
                }
            }
        }
    }
}

/// One `--expect KIND:MEMBER:FIELD=VALUE`.
#[derive(Clone, Debug)]
struct Expectation {
    kind: DataKind,
    member: u16,
    field: String,
    value: i64,
}

impl fmt::Display for Expectation {
    /// "species 397 attack", the prefix of every line about this expectation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.kind.name(), self.member, self.field)
    }
}

fn parse_expectation(text: &str) -> Result<Expectation, String> {
    let usage = || format!("{text}: expected KIND:MEMBER:FIELD=VALUE");
    let (target, value) = text.split_once('=').ok_or_else(usage)?;
    let mut parts = target.split(':');
    let (Some(kind), Some(member), Some(field), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(usage());
    };
    let kind = match kind {
        "species" => DataKind::Species,
        "move" => DataKind::Move,
        "item" => DataKind::Item,
        "learnset" => DataKind::Learnset,
        _ => {
            return Err(format!(
                "{text}: unknown kind {kind} (species, move, item, learnset)"
            ));
        }
    };
    let fields = kind.fields();
    if !fields.contains(&field) {
        return Err(format!(
            "{} has no field {field} (fields: {})",
            kind.name(),
            fields.join(", ")
        ));
    }
    Ok(Expectation {
        kind,
        member: member
            .parse()
            .map_err(|_| format!("{text}: member {member} is not a number"))?,
        field: field.to_owned(),
        value: value
            .parse()
            .map_err(|_| format!("{text}: value {value} is not a number"))?,
    })
}

fn species_field(bytes: &[u8], field: &str) -> Result<i64, String> {
    let p = uxie::PersonalData::from_binary(&mut Cursor::new(bytes))
        .map_err(|e| format!("uxie rejects the member: {e}"))?;
    let value: i64 = match field {
        "hp" => p.hp.into(),
        "attack" => p.attack.into(),
        "defense" => p.defense.into(),
        "speed" => p.speed.into(),
        "sp_attack" => p.sp_attack.into(),
        "sp_defense" => p.sp_defense.into(),
        "type1" => p.type1.into(),
        "type2" => p.type2.into(),
        "catch_rate" => p.catch_rate.into(),
        "base_exp" => p.base_exp.into(),
        "ev_yield" => p.ev_yield.into(),
        "item1" => p.item1.into(),
        "item2" => p.item2.into(),
        "gender_ratio" => p.gender_ratio.into(),
        "egg_cycles" => p.egg_cycles.into(),
        "base_friendship" => p.base_friendship.into(),
        "growth_rate" => p.growth_rate.into(),
        "egg_group1" => p.egg_group1.into(),
        "egg_group2" => p.egg_group2.into(),
        "ability1" => p.ability1.into(),
        "ability2" => p.ability2.into(),
        "flee_rate" => p.safari_flee_rate.into(),
        "color_flip" => p.color_flip.into(),
        _ => return Err(format!("species has no field {field}")),
    };
    Ok(value)
}

fn item_field(bytes: &[u8], field: &str) -> Result<i64, String> {
    let i = uxie::ItemData::from_binary(&mut Cursor::new(bytes))
        .map_err(|e| format!("uxie rejects the member: {e}"))?;
    let value: i64 = match field {
        "price" => i.price.into(),
        "hold_effect" => i.hold_effect.into(),
        "hold_param" => i.hold_effect_param.into(),
        "pluck_effect" => i.pluck_effect.into(),
        "fling_effect" => i.fling_effect.into(),
        "fling_power" => i.fling_power.into(),
        "natural_gift_power" => i.natural_gift_power.into(),
        "natural_gift_type" => i.natural_gift_type.into(),
        "prevent_toss" => i.prevent_toss.into(),
        "can_register" => i.is_selectable.into(),
        "field_pocket" => (i.field_pocket as u8).into(),
        "battle_pocket" => i.battle_pocket.bits().into(),
        "field_use_func" => i.field_use_func.into(),
        "battle_use_func" => i.battle_use_func.into(),
        "party_use" => i.party_use.into(),
        _ => return Err(format!("item has no field {field}")),
    };
    Ok(value)
}

fn move_field(bytes: &[u8], field: &str) -> Result<i64, String> {
    let m = Move::read(bytes).map_err(|e| e.to_string())?;
    let span = Move::FIELDS
        .iter()
        .find(|f| f.name == field)
        .ok_or_else(|| format!("move has no field {field}"))?;
    match span.kind {
        FieldKind::U8 => Ok(m.get_u8(span.offset).into()),
        FieldKind::U16 => Ok(m.get_u16(span.offset).into()),
        FieldKind::U32 => Ok(m.get_u32(span.offset).into()),
        FieldKind::I8 => Ok(m.get_i8(span.offset).into()),
        FieldKind::Bits { mask } => Ok(m.bits(span.offset, mask).into()),
        FieldKind::Bytes => Err(format!("move field {field} is not a number")),
    }
}

/// `g4k check-data`: reads the archives the expectations name from the ROM's file system
/// (packed paths from `archives.yaml` for the ROM's game code) and compares each value.
/// Matches go to stdout, mismatches and unreadable members to stderr; any of those fails.
fn check_data(rom: &Path, expectations: &[Expectation]) -> Result<(), Failure> {
    let at_rom = |e: &dyn fmt::Display| Failure::Message(format!("{}: {e}", rom.display()));
    let raw = raw::Rom::from_file(rom).map_err(|e| at_rom(&e))?;
    let header = raw.header().map_err(|e| at_rom(&e))?;
    let game_code = String::from_utf8_lossy(&header.gamecode.0).into_owned();
    let unknown = || KernelError::UnknownGame(game_code.clone());
    let family = Family::from_game_code(&game_code).ok_or_else(unknown)?;
    let language = Language::from_game_code(&game_code).ok_or_else(unknown)?;
    let archives = ArchiveMap::for_game(family, language, &game_code)?;

    // Paths inside the ROM's file system: the packed paths without the project's `files/`.
    let mut paths: HashMap<DataKind, String> = HashMap::new();
    for e in expectations {
        if paths.contains_key(&e.kind) {
            continue;
        }
        let packed = archives.packed(e.kind.asset_kind()).ok_or_else(|| {
            Failure::Message(format!("{game_code} has no {} archive", e.kind.name()))
        })?;
        let inside = packed.strip_prefix("files").unwrap_or(packed);
        let inside = inside
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        paths.insert(e.kind, inside);
    }
    let fnt = raw.fnt().map_err(|e| at_rom(&e))?;
    let fat = raw.fat().map_err(|e| at_rom(&e))?;
    let files = FileSystem::parse(&fnt, fat, &raw).map_err(|e| at_rom(&e))?;
    let mut narcs: HashMap<DataKind, Narc> = HashMap::new();
    for (kind, path) in &paths {
        let mut contents = None;
        files.traverse_files([path.as_str()], |file, _| {
            contents = Some(file.contents().to_vec())
        });
        let contents = contents
            .ok_or_else(|| Failure::Message(format!("{}: no file {path}", rom.display())))?;
        let narc = Narc::read(&contents).map_err(|e| Failure::Message(format!("{path}: {e}")))?;
        narcs.insert(*kind, narc);
    }

    let mut failed = 0;
    for e in expectations {
        let members = &narcs[&e.kind].members;
        let found = match members.get(usize::from(e.member)) {
            Some(bytes) => e.kind.read(bytes, &e.field),
            None => Err(format!("the archive has {} members", members.len())),
        };
        match found {
            Ok(v) if v == e.value => println!("{e}: {v}"),
            Ok(v) => {
                eprintln!("{e}: expected {}, found {v}", e.value);
                failed += 1;
            }
            Err(why) => {
                eprintln!("{} {}: {why}", e.kind.name(), e.member);
                failed += 1;
            }
        }
    }
    if failed > 0 {
        return Err(Failure::Message(format!(
            "{failed} of {} expectation(s) failed",
            expectations.len()
        )));
    }
    println!("{} expectation(s) met", expectations.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_field_reads_and_parses() {
        for (kind, len) in [
            (DataKind::Species, 44),
            (DataKind::Item, 34),
            (DataKind::Move, 16),
            (DataKind::Learnset, 4),
        ] {
            let mut bytes = vec![0u8; len];
            if kind == DataKind::Learnset {
                bytes[..2].copy_from_slice(&0xFFFFu16.to_le_bytes());
            }
            for field in kind.fields() {
                assert_eq!(kind.read(&bytes, field), Ok(0), "{} {field}", kind.name());
                let e = parse_expectation(&format!("{}:7:{field}=-3", kind.name())).unwrap();
                assert_eq!((e.kind, e.member, e.value), (kind, 7, -3));
            }
        }
    }

    #[test]
    fn malformed_expectations_are_refused() {
        for text in [
            "species:1:attack",
            "species:1=85",
            "species:1:attack:x=85",
            "pokemon:1:attack=85",
            "species:x:attack=85",
            "species:1:attack=x",
            "learnset:1:entries=3",
        ] {
            assert!(parse_expectation(text).is_err(), "{text}");
        }
    }
}
