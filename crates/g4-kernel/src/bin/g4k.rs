//! `g4k`: headless command line over the kernel, for scripting, CI and debugging.

use std::collections::HashMap;
use std::fmt;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use ds_rom::rom::{FileSystem, raw};
use g4_formats::data::encounter::{Encounters, Method};
use g4_formats::data::learnset::Learnset;
use g4_formats::data::moves::Move;
use g4_formats::data::record::{FieldKind, RawRecord};
use g4_formats::game::{Family, Language};
use g4_formats::map::{Cell, MapFile};
use g4_formats::narc::Narc;
use g4_kernel::build::{build, extract_rom};
use g4_kernel::checks::{
    ChecksReport, Group, IgnoreSet, Registry, Runner, Severity, Snapshot, SnapshotCache, Subject,
};
use g4_kernel::error::KernelError;
use g4_kernel::project::{ArchiveMap, AssetKind, Project};
use g4_kernel::start::{Template, create_project, inspect_rom};
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
    /// Print what a .nds ROM is (title, game code, family, language, SHA-1, support) as JSON.
    /// Reads the file only.
    InspectRom { rom: PathBuf },
    /// Create a project from a .nds ROM: extract it into `dest` (missing or empty) and write
    /// the base mark `.g4/base.json`. Prints the root and the phases with their milliseconds.
    Create {
        rom: PathBuf,
        dest: PathBuf,
        /// `original` (default), `data-only` or `empty-region` (adopts the scripts, applies
        /// the empty region and saves).
        #[arg(long, default_value = "original", value_parser = parse_template)]
        template: Template,
    },
    /// Apply a world template to a project and save: `empty-region` adopts the scripts when
    /// they are not yet, applies the empty region with the start town and prints its report.
    /// The project must have no unsaved changes elsewhere (nothing else is open here).
    Template {
        project: PathBuf,
        /// `empty-region`.
        #[arg(value_parser = parse_world_template)]
        template: WorldTemplate,
    },
    /// Map inspection and editing.
    #[command(subcommand)]
    Map(MapCommand),
    /// Pack the unpacked archives and build a ROM.
    Build {
        project: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Print one JSON line `{ "ms", "rom", "steps" }` (a row per build step) instead of
        /// the text lines.
        #[arg(long)]
        timings: bool,
        /// Delete the build cache (`.g4/build/`, all but the text sync) first, so every step
        /// runs fully.
        #[arg(long)]
        no_cache: bool,
    },
    /// Read data members from a built ROM's archives and compare them with expected values.
    /// Personal data and items are read with uxie, moves and learnsets with this project's
    /// codecs (uxie rejects 172 of 471 vanilla moves). Exit 1 when any expectation fails.
    CheckData {
        rom: PathBuf,
        /// `KIND:MEMBER:FIELD=VALUE`, for example `species:397:attack=85`. KIND is species,
        /// move, item, learnset or encounter; MEMBER the member index in the archive (item: the
        /// data member, not the item id); a learnset has the field `len` (number of entries); an
        /// encounter file `grass_rate`, `grassN` (species of grass slot N, 0 to 11; HeartGold:
        /// the day column), `grassN_level` and in HeartGold `grassN_morning`, `grassN_night`.
        #[arg(long = "expect", required = true, value_parser = parse_expectation)]
        expect: Vec<Expectation>,
    },
    /// Run every project check on the saved project and print the findings, the ignored ones
    /// and the checks that did not run. Honours the project's ignore file `g4-checks.json`
    /// like the app: ignored findings are listed apart and counted as ignored, checks it
    /// switches off do not run (errors are never left out). Exit 1 only when an expectation
    /// fails; findings alone never fail.
    Check {
        project: PathBuf,
        /// Print the whole report as JSON (expectation results then go to stderr).
        #[arg(long)]
        json: bool,
        /// `CODE=N`: the project has exactly N findings of CODE that are not ignored, for
        /// example `warp.dest-warp=1`.
        #[arg(long = "expect", value_parser = parse_count)]
        expect: Vec<CountExpectation>,
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

fn parse_template(text: &str) -> Result<Template, String> {
    match text {
        "original" => Ok(Template::Original),
        "data-only" | "data_only" => Ok(Template::DataOnly),
        "empty-region" | "empty_region" => Ok(Template::EmptyRegion),
        other => Err(format!(
            "unknown template {other:?}: original, data-only or empty-region"
        )),
    }
}

/// A template `g4k template` applies to an existing project.
#[derive(Clone, Copy, Debug)]
enum WorldTemplate {
    EmptyRegion,
}

fn parse_world_template(text: &str) -> Result<WorldTemplate, String> {
    match text {
        "empty-region" | "empty_region" => Ok(WorldTemplate::EmptyRegion),
        other => Err(format!("unknown template {other:?}: empty-region")),
    }
}

/// `g4k template`: checks what refuses the template without adopted scripts (Diamond/Pearl,
/// the plan, a world that is already empty), then adopts the scripts when needed, applies the
/// template, saves and prints what changed. A refused run writes nothing.
fn apply_template(path: &Path, template: WorldTemplate) -> Result<(), Failure> {
    let project = Arc::new(Project::open(path)?);
    match template {
        WorldTemplate::EmptyRegion => {
            // The refusals of `apply_empty_region` that come before it writes, with its
            // messages, but before adopting: that writes rotom.toml and the sources.
            g4_kernel::templates::region_recipe(project.family)?;
            let plan =
                g4_kernel::templates::plan_empty_region(&mut Store::new(Arc::clone(&project)))?;
            if plan.already_empty {
                return Err(KernelError::Refused("the world is already empty".into()).into());
            }
            if !g4_kernel::scripts::is_adopted(&project) {
                let adopted = g4_kernel::scripts::adopt(&project, &mut |_| {})?;
                println!(
                    "adopted scripts: {} decompiled, {} empty, {} failed",
                    adopted.decompiled,
                    adopted.skipped_empty,
                    adopted.failed.len()
                );
            }
            let mut store = Store::new(Arc::clone(&project));
            let report = g4_kernel::templates::apply_empty_region(&mut store)?;
            let saved = store.save()?;
            print_problems(&saved.problems);
            let plan = &report.plan;
            println!(
                "start town: header {} ({}), cells {:?}, event file {}, script file {}",
                plan.start_header,
                plan.town_name,
                plan.town_cells,
                plan.town_event_file,
                plan.town_script_file
            );
            println!(
                "kept={:?} emptied={} border_cells={} dropped_warps={:?} removed_objects={} \
                 removed_triggers={}",
                plan.kept,
                plan.emptied.len(),
                plan.border_cells,
                plan.dropped_warps,
                plan.removed_objects,
                plan.removed_triggers
            );
            println!(
                "border_map={} level_script={} start_script={}#{}",
                report.border_map,
                report.level_script,
                report.start_script.0,
                report.start_script.1
            );
            println!("saved {} asset(s)", saved.written.len());
        }
    }
    Ok(())
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
        Command::InspectRom { rom } => {
            let info = inspect_rom(&rom)?;
            let json = serde_json::to_string_pretty(&info)
                .map_err(|e| Failure::Message(format!("inspect-rom: {e}")))?;
            println!("{json}");
            Ok(())
        }
        Command::Create {
            rom,
            dest,
            template,
        } => {
            let created = create_project(&rom, &dest, template, &mut |_| {})?;
            println!("root={}", created.root.display());
            for (phase, ms) in &created.phases {
                println!(
                    "{}: {ms} ms",
                    serde_json::json!(phase).as_str().unwrap_or("?")
                );
            }
            Ok(())
        }
        Command::Template { project, template } => apply_template(&project, template),
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
        Command::Build {
            project,
            out,
            timings,
            no_cache,
        } => {
            let project = Project::open(project)?;
            if no_cache {
                g4_kernel::build::clear_cache(&project)?;
            }
            let report = build(&project, &out)?;
            if timings {
                print_problems(&report.problems);
                let line = serde_json::json!({
                    "ms": report.ms,
                    "rom": report.rom.display().to_string(),
                    "steps": report.steps,
                });
                println!("{line}");
                return Ok(());
            }
            // The kinds whose pack step ran; skipped ones are missing.
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
        Command::Check {
            project,
            json,
            expect,
        } => check(&project, json, &expect),
    }
}

/// One `--expect CODE=N` of `g4k check`.
#[derive(Clone, Debug)]
struct CountExpectation {
    code: &'static str,
    count: usize,
}

fn parse_count(text: &str) -> Result<CountExpectation, String> {
    let (code, count) = text
        .split_once('=')
        .ok_or_else(|| format!("{text}: expected CODE=N"))?;
    let code = Registry::get(code)
        .ok_or_else(|| format!("{text}: unknown check code {code}"))?
        .code();
    let count = count
        .parse()
        .map_err(|_| format!("{text}: count {count} is not a number"))?;
    Ok(CountExpectation { code, count })
}

/// "event file 0411": a subject in the kernel's words.
fn subject_text(subject: Subject) -> String {
    let (kind, n) = match subject {
        Subject::Header(n) => ("header", n),
        Subject::EventFile(n) => ("event file", n),
        Subject::ScriptFile(n) => ("script file", n),
        Subject::TextArchive(n) => ("text archive", n),
        Subject::Trainer(n) => ("trainer", n),
        Subject::Encounter(n) => ("encounter file", n),
        Subject::Matrix(n) => ("matrix", n),
        Subject::Map(n) => ("map", n),
        Subject::AreaData(n) => ("area data", n),
        Subject::Project => return "project".to_owned(),
    };
    format!("{kind} {n:04}")
}

/// `g4k check`: every group on the saved project (no store edits), with its ignore file.
fn check(path: &Path, json: bool, expectations: &[CountExpectation]) -> Result<(), Failure> {
    let project = Arc::new(Project::open(path)?);
    let ignore = IgnoreSet::load(&project.root)?;
    let overlay = Store::new(Arc::clone(&project)).snapshot_overlay();
    let snap = Snapshot::build(Arc::clone(&project), overlay, SnapshotCache::default());
    let report = Runner::run(&snap, Project::limits(), &ignore, &Group::ALL, |_, _, _| {});
    if json {
        let text = serde_json::to_string(&report).map_err(|e| Failure::Message(e.to_string()))?;
        println!("{text}");
    } else {
        print_report(&report);
    }

    let mut failed = 0;
    for e in expectations {
        let found = report.findings.iter().filter(|f| f.code == e.code).count();
        if found == e.count {
            if json {
                eprintln!("{}: {found}", e.code);
            } else {
                println!("{}: {found}", e.code);
            }
        } else {
            eprintln!("{}: expected {}, found {found}", e.code, e.count);
            failed += 1;
        }
    }
    if failed > 0 {
        return Err(Failure::Message(format!(
            "{failed} of {} expectation(s) failed",
            expectations.len()
        )));
    }
    if !expectations.is_empty() && !json {
        println!("{} expectation(s) met", expectations.len());
    }
    Ok(())
}

fn print_report(report: &ChecksReport) {
    for f in &report.findings {
        let severity = match f.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        println!(
            "{severity} {} {} {}: {}",
            f.code,
            subject_text(f.subject),
            f.key,
            f.message
        );
    }
    for (code, why) in &report.skipped {
        println!("skipped {code}: {why}");
    }
    let count = |severity| {
        report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    };
    println!(
        "{} finding(s): {} error(s), {} warning(s), {} info(s); {} ignored; {} ms",
        report.findings.len(),
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info),
        report.ignored.len(),
        report.ran_ms
    );
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
    Encounter,
    /// A map file: SHA-1 of its model or permissions (hex).
    Map,
}

/// Fields of a map file: SHA-1 of a part, lowercase hex.
const MAP_FIELDS: &[&str] = &["model_sha1", "per_sha1"];

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
            DataKind::Encounter => "encounter",
            DataKind::Map => "map",
        }
    }

    fn asset_kind(self) -> AssetKind {
        match self {
            DataKind::Species => AssetKind::Personal,
            DataKind::Move => AssetKind::Move,
            DataKind::Item => AssetKind::Item,
            DataKind::Learnset => AssetKind::Learnset,
            DataKind::Encounter => AssetKind::Encounter,
            DataKind::Map => AssetKind::Map,
        }
    }

    /// The fields an expectation may name.
    fn fields(self) -> Vec<String> {
        let names = |list: &[&str]| list.iter().map(|f| f.to_string()).collect();
        match self {
            DataKind::Species => names(SPECIES_FIELDS),
            DataKind::Item => names(ITEM_FIELDS),
            DataKind::Move => Move::FIELDS
                .iter()
                .filter(|f| f.kind != FieldKind::Bytes)
                .map(|f| f.name.to_string())
                .collect(),
            DataKind::Learnset => names(&["len"]),
            DataKind::Encounter => std::iter::once("grass_rate".to_string())
                .chain((0..GRASS_SLOTS).flat_map(|i| {
                    ["", "_level", "_morning", "_night"].map(|column| format!("grass{i}{column}"))
                }))
                .collect(),
            DataKind::Map => names(MAP_FIELDS),
        }
    }

    /// The value of `field` in the member `bytes` of a ROM of `family`, as text (a number or
    /// a hex digest).
    fn read(self, bytes: &[u8], field: &str, family: Family) -> Result<String, String> {
        let number = match self {
            DataKind::Species => species_field(bytes, field),
            DataKind::Item => item_field(bytes, field),
            DataKind::Move => move_field(bytes, field),
            DataKind::Encounter => encounter_field(bytes, family, field),
            DataKind::Learnset => {
                let learnset = Learnset::read(bytes).map_err(|e| e.to_string())?;
                match field {
                    "len" => Ok(learnset.entries.len() as i64),
                    _ => Err(format!("learnset has no field {field}")),
                }
            }
            DataKind::Map => return map_field(bytes, field, family),
        };
        number.map(|v| v.to_string())
    }

    /// Whether `value` is a valid expected value for this kind.
    fn parse_value(self, value: &str) -> Option<String> {
        match self {
            DataKind::Map => (value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| value.to_ascii_lowercase()),
            _ => value.parse::<i64>().ok().map(|v| v.to_string()),
        }
    }
}

/// One `--expect KIND:MEMBER:FIELD=VALUE`.
#[derive(Clone, Debug)]
struct Expectation {
    kind: DataKind,
    member: u16,
    field: String,
    /// A number, or 40 hex digits for a map part digest.
    value: String,
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
        "encounter" => DataKind::Encounter,
        "map" => DataKind::Map,
        _ => {
            return Err(format!(
                "{text}: unknown kind {kind} (species, move, item, learnset, encounter, map)"
            ));
        }
    };
    let fields = kind.fields();
    if !fields.iter().any(|f| f == field) {
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
        value: kind.parse_value(value).ok_or_else(|| match kind {
            DataKind::Map => format!("{text}: value {value} is not 40 hex digits"),
            _ => format!("{text}: value {value} is not a number"),
        })?,
    })
}

/// SHA-1 (lowercase hex) of the model or the permissions of a map file.
fn map_field(bytes: &[u8], field: &str, family: Family) -> Result<String, String> {
    use sha1::{Digest, Sha1};
    let map = MapFile::read(bytes, family).map_err(|e| e.to_string())?;
    let part: Vec<u8> = match field {
        "model_sha1" => map.model,
        "per_sha1" => map
            .permissions
            .iter()
            .flat_map(|c| [c.kind, c.collision])
            .collect(),
        _ => return Err(format!("map has no field {field}")),
    };
    Ok(Sha1::digest(&part)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
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

/// Grass slots of an encounter file (both families).
const GRASS_SLOTS: usize = 12;

/// `grass_rate`, `grassN` (the species; HeartGold: the day column), `grassN_level`,
/// `grassN_morning` and `grassN_night` (HeartGold only) of an encounter file.
fn encounter_field(bytes: &[u8], family: Family, field: &str) -> Result<i64, String> {
    let encounters = Encounters::read(bytes, family).map_err(|e| e.to_string())?;
    let grass = encounters
        .methods()
        .into_iter()
        .find(|m| m.method == Method::Grass)
        .ok_or("the file has no grass method")?;
    if field == "grass_rate" {
        return Ok(grass.rate.into());
    }
    let unknown = || format!("encounter has no field {field}");
    let rest = field.strip_prefix("grass").ok_or_else(unknown)?;
    let (slot, column) = match rest.split_once('_') {
        Some((slot, column)) => (slot, Some(column)),
        None => (rest, None),
    };
    let slot = slot
        .parse::<usize>()
        .ok()
        .and_then(|i| grass.slots.get(i))
        .ok_or_else(unknown)?;
    // HeartGold's (morning, night) species next to the day species; Platinum has one column.
    let other = |i: usize| {
        slot.alt_species
            .map(|alt| i64::from(alt[i]))
            .ok_or_else(|| format!("{family:?} encounter files have no {field}"))
    };
    match column {
        None => Ok(slot.species.into()),
        Some("level") => Ok(slot.min.into()),
        Some("morning") => other(0),
        Some("night") => other(1),
        Some(_) => Err(unknown()),
    }
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
            Some(bytes) => e.kind.read(bytes, &e.field, family),
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
                let field = field.as_str();
                assert_eq!(
                    kind.read(&bytes, field, Family::Pt),
                    Ok("0".to_owned()),
                    "{} {field}",
                    kind.name()
                );
                let e = parse_expectation(&format!("{}:7:{field}=-3", kind.name())).unwrap();
                assert_eq!((e.kind, e.member, e.value.as_str()), (kind, 7, "-3"));
            }
        }
    }

    #[test]
    fn encounter_fields_read_the_grass_slots_of_both_families() {
        // Platinum: rate u32 at 0, slot i = level u32 at 4 + 8i, species u32 at 8 + 8i.
        let mut pt = vec![0u8; 424];
        pt[0] = 100;
        pt[0x04 + 8 * 3] = 5;
        pt[0x08 + 8 * 3..0x0A + 8 * 3].copy_from_slice(&150u16.to_le_bytes());
        let read = |bytes: &[u8], family, field: &str| {
            DataKind::Encounter
                .read(bytes, field, family)
                .map(|v| v.parse::<i64>().expect("encounter values are numbers"))
        };
        assert_eq!(read(&pt, Family::Pt, "grass_rate"), Ok(100));
        assert_eq!(read(&pt, Family::Pt, "grass3"), Ok(150));
        assert_eq!(read(&pt, Family::Pt, "grass3_level"), Ok(5));
        assert_eq!(read(&pt, Family::Pt, "grass0"), Ok(0));
        assert!(
            read(&pt, Family::Pt, "grass3_morning").is_err(),
            "Platinum has one column"
        );
        // HeartGold: levels u8 at 8, morning/day/night species u16 at 0x14/0x2C/0x44.
        let mut hg = vec![0u8; 196];
        hg[0] = 100;
        hg[0x08 + 11] = 7;
        hg[0x14 + 22..0x16 + 22].copy_from_slice(&151u16.to_le_bytes());
        hg[0x2C + 22..0x2E + 22].copy_from_slice(&152u16.to_le_bytes());
        hg[0x44 + 22..0x46 + 22].copy_from_slice(&153u16.to_le_bytes());
        assert_eq!(read(&hg, Family::Hgss, "grass_rate"), Ok(100));
        assert_eq!(
            read(&hg, Family::Hgss, "grass11"),
            Ok(152),
            "the day column"
        );
        assert_eq!(read(&hg, Family::Hgss, "grass11_morning"), Ok(151));
        assert_eq!(read(&hg, Family::Hgss, "grass11_night"), Ok(153));
        assert_eq!(read(&hg, Family::Hgss, "grass11_level"), Ok(7));
        // a member of the wrong length is reported, not read
        assert!(read(&hg, Family::Pt, "grass0").is_err());
        // every listed field reads (HeartGold has all columns) and parses
        let fields = DataKind::Encounter.fields();
        assert_eq!(fields.len(), 1 + 12 * 4);
        for field in &fields {
            assert_eq!(read(&[0u8; 196], Family::Hgss, field), Ok(0), "{field}");
            let e = parse_expectation(&format!("encounter:7:{field}=-3")).unwrap();
            assert_eq!(
                (e.kind, e.member, e.value.as_str()),
                (DataKind::Encounter, 7, "-3")
            );
        }
    }

    #[test]
    fn encounter_expectations_parse() {
        let e = parse_expectation("encounter:140:grass0=150").unwrap();
        assert_eq!(
            (e.kind, e.member, e.field.as_str(), e.value.as_str()),
            (DataKind::Encounter, 140, "grass0", "150")
        );
        for text in [
            "encounter:1:grass12=1",
            "encounter:1:grass=1",
            "encounter:1:surf0=1",
        ] {
            assert!(parse_expectation(text).is_err(), "{text}");
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
            "map:1:model_sha1=abc",
            "map:1:model_sha1=-3",
            "map:1:bdhc_sha1=0000000000000000000000000000000000000000",
        ] {
            assert!(parse_expectation(text).is_err(), "{text}");
        }
    }
}
