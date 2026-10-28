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
    /// The base ROM of a project: check and assign it.
    #[command(subcommand)]
    Base(BaseCommand),
    /// Patches of the built project against its base ROM.
    #[command(subcommand)]
    Patch(PatchCommand),
    /// The text project: the game data as JSON under `data/` with the manifest
    /// `g4-project.json`.
    #[command(subcommand)]
    Text(TextCommand),
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
    write_problems(&mut std::io::stderr().lock(), problems);
}

/// One line per problem: `<severity>: <file>:<line>:<col>: <message>`.
fn write_problems(out: &mut impl std::io::Write, problems: &[Problem]) {
    for problem in problems {
        // Like `eprintln!`, a failed write has nowhere else to go; unlike it, no panic.
        let _ = writeln!(
            out,
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
        Command::Base(BaseCommand::Check { project, rom, json }) => match rom {
            Some(rom) => base_check_rom(&project, &rom, json),
            None => base_status(&project, json),
        },
        Command::Base(BaseCommand::Assign { project, nds }) => base_assign(&project, &nds),
        Command::Patch(PatchCommand::Create {
            project,
            base_rom,
            out,
            json,
        }) => patch_create(&project, base_rom.as_deref(), out.as_deref(), json),
        Command::Text(command) => text(command),
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
    let ids = g4_kernel::textproject::ids::Resolver::for_project(&project)?;
    let ignore = IgnoreSet::load(&project.root, &ids)?;
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

#[derive(Subcommand)]
enum BaseCommand {
    /// Without `--rom`: the project's base mark (`mark=none|extracted|dspre|unreadable|newer`),
    /// its SHA-1, the sibling `.nds` DSPRE unpacked it from (without a mark) and where the base
    /// ROM is (`--base-rom` aside: sibling, then `G4_BASE_ROM_DIR`). With `--rom`: checks that
    /// `.nds` as the base ROM and prints one `warning:` line per warning; exit 1 when it is
    /// refused (another game). Writes nothing.
    Check {
        project: PathBuf,
        #[arg(long)]
        rom: Option<PathBuf>,
        /// Print the result as JSON (with `--rom`: the whole check).
        #[arg(long)]
        json: bool,
    },
    /// Assign a `.nds` as the base ROM of a DSPRE folder: writes the base mark `.g4/base.json`
    /// (`origin: dspre`). Prints the warnings, then `assigned sha1=… label=…`; a refusal (another
    /// game, a fixed or unreadable mark) goes to stderr with exit 1.
    Assign { project: PathBuf, nds: PathBuf },
}

#[derive(Subcommand)]
enum PatchCommand {
    /// Build the project to `build/<Name>.nds` and write an xdelta patch against its base ROM
    /// (default `build/<Name>.xdelta`, with the notes `<patch>.txt`), checked before writing.
    /// The base ROM: `--base-rom`, else the sibling `.nds`, else `G4_BASE_ROM_DIR`.
    Create {
        project: PathBuf,
        /// The base ROM; another SHA-1 than the mark's is refused.
        #[arg(long)]
        base_rom: Option<PathBuf>,
        /// Where to write the patch instead of `build/<Name>.xdelta`.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Print the patch report as JSON.
        #[arg(long)]
        json: bool,
    },
}

fn json_line(value: &impl serde::Serialize) -> Result<String, Failure> {
    serde_json::to_string(value).map_err(|e| Failure::Message(e.to_string()))
}

/// Says so on stderr when `G4_BASE_ROM_DIR` is set but names no folder: the search then finds
/// nothing there, and a typo should not go unnoticed.
fn hint_base_dir(dir: Option<&Path>) {
    if let Some(dir) = dir
        && !dir.is_dir()
    {
        eprintln!(
            "hint: {} is not a folder: {}",
            g4_kernel::base::BASE_ROM_DIR_VAR,
            dir.display()
        );
    }
}

/// `g4k base check` without `--rom`: the mark, the suggestion and where the base ROM is.
fn base_status(root: &Path, json: bool) -> Result<(), Failure> {
    use g4_kernel::start::MarkState;

    let dir = g4_kernel::base::env_base_dir();
    hint_base_dir(dir.as_deref());
    let state = g4_kernel::start::mark_state(root);
    let (kind, reason) = match &state {
        MarkState::Missing => ("none".to_owned(), None),
        MarkState::Unreadable(why) => ("unreadable".to_owned(), Some(why.clone())),
        MarkState::Newer(v) => ("newer".to_owned(), Some(format!("version {v}"))),
        MarkState::Present(mark) => (mark.origin.clone(), None),
    };
    let mark = match state {
        MarkState::Present(mark) => Some(mark),
        _ => None,
    };
    let suggestion = match mark {
        Some(_) => None,
        None => g4_kernel::start::suggest_base(root),
    };
    let located = match &mark {
        Some(mark) => g4_kernel::base::locate(
            mark,
            &g4_kernel::base::Search {
                root: Some(root),
                dir: dir.as_deref(),
                ..Default::default()
            },
        )?,
        None => None,
    };
    if json {
        let base = located
            .as_ref()
            .map(|l| serde_json::json!({ "path": l.path, "source": l.source }));
        let value = serde_json::json!({
            "mark": kind,
            "reason": reason,
            "sha1": mark.as_ref().map(|m| &m.sha1),
            "suggestion": suggestion,
            "base": base,
        });
        println!("{}", json_line(&value)?);
        return Ok(());
    }
    println!("mark={kind}");
    if let Some(reason) = reason {
        println!("reason={reason}");
    }
    if let Some(mark) = &mark {
        println!("sha1={}", mark.sha1);
    }
    if let Some(nds) = suggestion {
        println!("suggestion={}", nds.display());
    }
    if mark.is_some() {
        match located {
            Some(l) => {
                let source = serde_json::to_value(l.source)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                println!("base={} ({source})", l.path.display());
            }
            None => println!("base=missing"),
        }
    }
    Ok(())
}

/// `g4k base check --rom`: [`g4_kernel::start::check_base`], refused → exit 1.
fn base_check_rom(root: &Path, nds: &Path, json: bool) -> Result<(), Failure> {
    let check = g4_kernel::start::check_base(root, nds)?;
    if json {
        println!("{}", json_line(&check)?);
    } else {
        println!(
            "sha1={} game_code={} title={}",
            check.info.sha1, check.info.game_code, check.info.title
        );
        for w in &check.warnings {
            println!("warning: {}", w.message);
        }
    }
    match check.refused {
        Some(reason) => Err(Failure::Message(reason)),
        None => Ok(()),
    }
}

/// `g4k base assign`: [`g4_kernel::start::assign_base`].
fn base_assign(root: &Path, nds: &Path) -> Result<(), Failure> {
    let (check, mark) = g4_kernel::start::assign_base(root, nds)?;
    for w in &check.warnings {
        println!("warning: {}", w.message);
    }
    println!(
        "assigned sha1={} label={}",
        mark.sha1,
        g4_kernel::base::base_label(&mark)
    );
    Ok(())
}

/// `g4k patch create`: finds the base ROM by the mark's SHA-1, builds and writes the patch.
fn patch_create(
    root: &Path,
    base_rom: Option<&Path>,
    out: Option<&Path>,
    json: bool,
) -> Result<(), Failure> {
    let mark = required_mark(root)?;
    let project = Project::open(root)?;
    let base = required_base(root, &mark, base_rom)?;
    let (build, report) = g4_kernel::patch::build_and_create(&project, &base, out)?;
    print_problems(&build.problems);
    if json {
        println!("{}", json_line(&report)?);
    } else {
        println!(
            "patch={} size={} encode_ms={} verify_ms={} base={} target={}",
            report.patch.display(),
            report.size,
            report.encode_ms,
            report.verify_ms,
            report.base_sha1,
            report.target_sha1
        );
        println!(
            "notes={} rom={}",
            report.notes.display(),
            report.rom.display()
        );
    }
    Ok(())
}

/// The project's base mark, or the refusal in the app's words; the no-mark one adds how to
/// assign one here.
fn required_mark(root: &Path) -> Result<g4_kernel::start::BaseMark, Failure> {
    use g4_kernel::start::MarkState;

    match g4_kernel::start::mark_state(root) {
        MarkState::Present(mark) => Ok(mark),
        MarkState::Missing => Err(Failure::Message(format!(
            "{} Ordne sie mit `g4k base assign <Projekt> <nds>` zu.",
            g4_kernel::patch::NO_MARK
        ))),
        state @ (MarkState::Unreadable(_) | MarkState::Newer(_)) => {
            Err(Failure::Message(state.refusal().unwrap_or_default()))
        }
    }
}

/// The base ROM of `mark`: `explicit` (another SHA-1 is refused), else the sibling `.nds`,
/// else `G4_BASE_ROM_DIR`; `None` (after the hint at a base folder that is none) when it is
/// nowhere.
fn located_base(
    root: &Path,
    mark: &g4_kernel::start::BaseMark,
    explicit: Option<&Path>,
) -> Result<Option<g4_kernel::base::BaseRom>, Failure> {
    let dir = g4_kernel::base::env_base_dir();
    let located = g4_kernel::base::locate(
        mark,
        &g4_kernel::base::Search {
            explicit,
            root: Some(root),
            dir: dir.as_deref(),
            ..Default::default()
        },
    )?;
    let Some(located) = located else {
        hint_base_dir(dir.as_deref());
        return Ok(None);
    };
    Ok(Some(g4_kernel::base::BaseRom::open(
        &located.path,
        Some(&mark.sha1),
    )?))
}

/// [`located_base`], a missing base refused with [`g4_kernel::base::missing_text`].
fn required_base(
    root: &Path,
    mark: &g4_kernel::start::BaseMark,
    explicit: Option<&Path>,
) -> Result<g4_kernel::base::BaseRom, Failure> {
    located_base(root, mark, explicit)?
        .ok_or_else(|| Failure::Message(g4_kernel::base::missing_text(mark)))
}

#[derive(Subcommand)]
enum TextCommand {
    /// Write the project's members as the text project (manifest, `data/`), adopting the
    /// scripts first when they are not. Needs the base mark and the base ROM: `--base-rom`,
    /// else the sibling `.nds`, else `G4_BASE_ROM_DIR`. Prints `adopted scripts=N` when it
    /// adopted, one line per art, then the totals; exit 1 on an error.
    Export {
        project: PathBuf,
        /// The base ROM; another SHA-1 than the mark's is refused.
        #[arg(long)]
        base_rom: Option<PathBuf>,
        /// Print the export report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Write the text project back into the members after checking every file; an error
    /// names file, line and column and nothing is written (exit 1). The base ROM (found like
    /// `export`) is needed only for parts that reference it.
    Import {
        project: PathBuf,
        /// The base ROM; another SHA-1 than the mark's is refused.
        #[arg(long)]
        base_rom: Option<PathBuf>,
        /// Print the import report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// What an import would find and write, without writing: `schema=…`, one line per
    /// problem, `differs=N` and the first 20 paths. Exit 1 on an error or another schema
    /// version.
    Check {
        project: PathBuf,
        /// Print the check report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// The files of the text project, one path per line.
    Sources { project: PathBuf },
    /// Lift an older text project to the current schema version, with a snapshot of the
    /// files it changes; a newer version is refused (exit 1).
    Migrate {
        project: PathBuf,
        /// Print the migration report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Set the DSPRE folder up as text project: adopt the scripts (when they are not), export,
    /// add the include root to `rotom.toml`, record the sync. Needs the base mark and the
    /// base ROM (found like `export`). Prints `phase=<phase> ms=N` per phase, then the
    /// totals.
    Adopt {
        project: PathBuf,
        /// The base ROM; another SHA-1 than the mark's is refused.
        #[arg(long)]
        base_rom: Option<PathBuf>,
        /// Print the adoption report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Take the text project back to a DSPRE folder: moves `g4-project.json`, `data/` and the
    /// id files for rotom (`.rotom/command_database/.g4-ids-*.json`) to
    /// `.g4/backup/textproject-<time>/`. Refused (exit 1) while `text sync` finds something
    /// pending, or while a script source uses an id only the id files define.
    Revert { project: PathBuf },
    /// Take over what changed on one side since the last record (`dspre <key> <label>`,
    /// `files <key> <label>`), list conflicts and broken files (`conflict <key> <label>`,
    /// `broken <key> <label>`, the problems on stderr); exit 1 while a conflict or a broken
    /// file is left (both are not settled). With `--keep studio|dspre <key>`: resolve that
    /// conflict instead.
    Sync {
        project: PathBuf,
        /// Resolve one conflict: the side to keep (`studio` or `dspre`) and the unit's key.
        #[arg(long, num_args = 2, value_names = ["SIDE", "KEY"])]
        keep: Option<Vec<String>>,
    },
    /// Rename a register id (`MAP_`, `TRAINER_`, `TRAINER_CLASS_`, `PLACE_`) with every
    /// reference: the register, the JSON under `data/`, `g4-world.json`, `g4-checks.json`, the
    /// script sources, then the id files for rotom. Prints `<file> <count>` per file and
    /// `renamed <old> -> <new> kind=… number=… files=… refs=… id_files=… ms=…`; a refusal
    /// (taken, a fallback of another number, malformed, …) writes nothing (exit 1). No undo.
    RenameId {
        project: PathBuf,
        old: String,
        new: String,
        /// Print the plan as JSON.
        #[arg(long)]
        json: bool,
    },
}

/// The paths `g4k text check` lists after `differs=N`.
const CHECK_PATHS: usize = 20;

fn text(command: TextCommand) -> Result<(), Failure> {
    match command {
        TextCommand::Export {
            project,
            base_rom,
            json,
        } => text_export(&project, base_rom.as_deref(), json),
        TextCommand::Import {
            project,
            base_rom,
            json,
        } => text_import(&project, base_rom.as_deref(), json),
        TextCommand::Check { project, json } => text_check(&project, json),
        TextCommand::Sources { project } => {
            for path in g4_kernel::textproject::sources(&Project::open(project)?)? {
                println!("{}", path.display());
            }
            Ok(())
        }
        TextCommand::Migrate { project, json } => text_migrate(&project, json),
        TextCommand::Adopt {
            project,
            base_rom,
            json,
        } => text_adopt(&project, base_rom.as_deref(), json),
        TextCommand::RenameId {
            project,
            old,
            new,
            json,
        } => text_rename_id(&project, &old, &new, json),
        TextCommand::Revert { project } => {
            let report = g4_kernel::textproject::adopt::revert(&Project::open(project)?)?;
            println!(
                "backup={} rewritten={} ms={}",
                report.backup.display(),
                report.rewritten.len(),
                report.ms
            );
            Ok(())
        }
        TextCommand::Sync { project, keep } => text_sync(&project, keep.as_deref()),
    }
}

/// Exit 1 when one of `problems` is an error (they are printed already).
fn fail_on_errors(problems: &[Problem], what: &str) -> Result<(), Failure> {
    let errors = problems
        .iter()
        .filter(|p| p.severity == g4_kernel::text::Severity::Error)
        .count();
    if errors == 0 {
        Ok(())
    } else {
        Err(Failure::Message(format!("{what}: {errors} error(s)")))
    }
}

/// `g4k text export`: [`g4_kernel::textproject::export`], adopting the scripts.
fn text_export(root: &Path, base_rom: Option<&Path>, json: bool) -> Result<(), Failure> {
    let mark = required_mark(root)?;
    let project = Project::open(root)?;
    let base = required_base(root, &mark, base_rom)?;
    let report = g4_kernel::textproject::export(
        &project,
        &base,
        &g4_kernel::textproject::ExportOptions {
            adopt_scripts: true,
        },
    )?;
    print_problems(&report.problems);
    if json {
        println!("{}", json_line(&report)?);
    } else {
        if let Some(adopted) = report.adopted {
            println!("adopted scripts={adopted}");
        }
        for a in &report.arts {
            println!(
                "art={} entities={} json={} side={} written={} removed={} ms={}",
                a.art, a.entities, a.json_files, a.side_files, a.written, a.removed, a.ms
            );
        }
        println!(
            "raw={} texts={} scripts={} base_layer_ms={} created={} files={} bytes={} \
             written={} removed={} ms={}",
            report.raw_blobs,
            report.texts,
            report.scripts,
            report.base_layer_ms,
            report.base_layer_created,
            report.files,
            report.bytes,
            report.written,
            report.removed,
            report.ms
        );
    }
    fail_on_errors(&report.problems, "text export")
}

/// `g4k text import`: [`g4_kernel::textproject::import`], with the base ROM when it is found.
fn text_import(root: &Path, base_rom: Option<&Path>, json: bool) -> Result<(), Failure> {
    use g4_kernel::start::MarkState;

    // A broken or newer mark is refused like `export` and `patch create`; only without a
    // mark does the import run without a located base.
    let base = match g4_kernel::start::mark_state(root) {
        MarkState::Present(mark) => located_base(root, &mark, base_rom)?,
        MarkState::Missing => match base_rom {
            Some(path) => Some(g4_kernel::base::BaseRom::open(path, None)?),
            None => None,
        },
        state @ (MarkState::Unreadable(_) | MarkState::Newer(_)) => {
            return Err(Failure::Message(state.refusal().unwrap_or_default()));
        }
    };
    let project = Project::open(root)?;
    let report = g4_kernel::textproject::import(&project, base.as_ref())?;
    print_problems(&report.problems);
    if json {
        println!("{}", json_line(&report)?);
    } else {
        println!(
            "members={} raw={} quirks={} compiled={} encoded={} ms={}",
            report.members_written,
            report.raw_written,
            report.quirks_seeded,
            report.compiled.len(),
            report.encoded.len(),
            report.ms
        );
    }
    fail_on_errors(&report.problems, "text import")
}

/// `g4k text check`: [`g4_kernel::textproject::check`]; exit 1 on an error or another schema
/// version than the current one.
fn text_check(root: &Path, json: bool) -> Result<(), Failure> {
    use g4_kernel::textproject::manifest::SchemaState;

    let project = Project::open(root)?;
    let report = g4_kernel::textproject::check(&project)?;
    if json {
        println!("{}", json_line(&report)?);
    } else {
        let schema = match report.schema {
            SchemaState::Current => "current".to_owned(),
            SchemaState::Older(v) => format!("older {v}"),
            SchemaState::Newer(v) => format!("newer {v}"),
            SchemaState::Missing => "missing".to_owned(),
        };
        println!("schema={schema}");
        write_problems(&mut std::io::stdout().lock(), &report.problems);
        println!("differs={}", report.differs.len());
        for path in report.differs.iter().take(CHECK_PATHS) {
            println!("  {}", path.display());
        }
        if report.differs.len() > CHECK_PATHS {
            println!("  ... {} more", report.differs.len() - CHECK_PATHS);
        }
    }
    if report.schema != SchemaState::Current {
        // The kernel's sentence for this state (`g4k text migrate`, newer, no manifest).
        return Err(
            match g4_kernel::textproject::manifest::read_manifest(root) {
                Err(e) => e.into(),
                // Defensive: `read_manifest` refuses every state but the current one.
                Ok(_) => Failure::Message(format!("text check: schema {:?}", report.schema)),
            },
        );
    }
    fail_on_errors(&report.problems, "text check")
}

/// `g4k text adopt`: [`g4_kernel::textproject::adopt::adopt`], timing each phase.
fn text_adopt(root: &Path, base_rom: Option<&Path>, json: bool) -> Result<(), Failure> {
    use g4_kernel::textproject::adopt::AdoptPhase;

    let mark = required_mark(root)?;
    let project = Project::open(root)?;
    let base = required_base(root, &mark, base_rom)?;
    let mut current: Option<(AdoptPhase, std::time::Instant)> = None;
    let mut timed: Vec<(AdoptPhase, u128)> = Vec::new();
    let report = g4_kernel::textproject::adopt::adopt(&project, &base, &mut |p| {
        if current.is_none_or(|(phase, _)| phase != p.phase)
            && let Some((phase, at)) = current.replace((p.phase, std::time::Instant::now()))
        {
            timed.push((phase, at.elapsed().as_millis()));
        }
    })?;
    if let Some((phase, at)) = current {
        timed.push((phase, at.elapsed().as_millis()));
    }
    print_problems(&report.export.problems);
    if json {
        println!("{}", json_line(&report)?);
    } else {
        for (phase, ms) in &timed {
            println!("phase={} ms={ms}", json_line(phase)?.trim_matches('"'));
        }
        println!(
            "adopted={} files={} written={} removed={} include={} sync_files={} ms={}",
            report
                .export
                .adopted
                .map_or_else(|| "already".to_owned(), |n| format!("scripts:{n}")),
            report.export.files,
            report.export.written,
            report.export.removed,
            report.include_written,
            report.sync_files,
            report.ms
        );
    }
    fail_on_errors(&report.export.problems, "text adopt")
}

/// `g4k text sync`: scan and take over the one-sided units, or with `keep` resolve one
/// conflict; exit 1 while conflicts or broken files are left.
fn text_sync(root: &Path, keep: Option<&[String]>) -> Result<(), Failure> {
    use g4_kernel::textproject::sync::{self, Keep, UnitSide};

    let project = Project::open(root)?;
    if let Some([side, key]) = keep {
        let keep = match side.as_str() {
            "studio" => Keep::Studio,
            "dspre" => Keep::Dspre,
            other => {
                return Err(Failure::Message(format!(
                    "--keep {other}: studio oder dspre"
                )));
            }
        };
        sync::resolve_files(&project, key, keep)?;
        println!("kept {side} {key}");
        return Ok(());
    }
    let scan = sync::scan(&project, &mut |_| {})?;
    let applied = sync::apply_one_sided(&project, &scan, &|_| false)?;
    for unit in &applied.dspre {
        println!("dspre {} {}", unit.key, unit.label);
    }
    for unit in &applied.files {
        println!("files {} {}", unit.key, unit.label);
    }
    let (mut conflicts, mut broken) = (0, 0);
    for unit in &scan.units {
        match unit.side {
            UnitSide::Conflict => {
                conflicts += 1;
                println!("conflict {} {}", unit.key, unit.label);
            }
            UnitSide::Broken => {
                broken += 1;
                println!("broken {} {}", unit.key, unit.label);
                print_problems(&unit.problems);
            }
            _ => {}
        }
    }
    println!(
        "stat={} hashed={} written={} ms={}",
        scan.stat_files,
        scan.hashed,
        applied.written.len(),
        scan.ms + applied.ms
    );
    if conflicts > 0 || broken > 0 {
        return Err(Failure::Message(format!(
            "text sync: {conflicts} conflict(s) (g4k text sync --keep studio|dspre <key>), \
             {broken} broken file(s) (fix them, then sync again)"
        )));
    }
    Ok(())
}

/// `g4k text rename-id`: [`g4_kernel::textproject::rename::rename_id_files`].
fn text_rename_id(root: &Path, old: &str, new: &str, json: bool) -> Result<(), Failure> {
    let result = g4_kernel::textproject::rename::rename_id_files(&Project::open(root)?, old, new)?;
    for warning in &result.warnings {
        eprintln!("warning: {warning}");
    }
    let plan = &result.plan;
    if json {
        println!("{}", json_line(plan)?);
        return Ok(());
    }
    for file in &plan.files {
        println!("{} {}", file.path.display(), file.count);
    }
    let kind = plan
        .kind
        .map_or_else(|| "?".to_owned(), |k| k.prefix().to_lowercase());
    println!(
        "renamed {old} -> {new} kind={kind} number={} files={} refs={} id_files={} ms={}",
        plan.number
            .map_or_else(|| "?".to_owned(), |n| n.to_string()),
        plan.files.len(),
        plan.total,
        plan.id_files,
        result.ms
    );
    Ok(())
}

/// `g4k text migrate`: [`g4_kernel::textproject::migrate::migrate`].
fn text_migrate(root: &Path, json: bool) -> Result<(), Failure> {
    let report = g4_kernel::textproject::migrate::migrate(root)?;
    if json {
        println!("{}", json_line(&report)?);
    } else if report.from == report.to {
        println!("already current");
    } else {
        println!(
            "migrated {} -> {} changed={} snapshot={}",
            report.from,
            report.to,
            report.changed.len(),
            report
                .snapshot
                .as_ref()
                .map_or_else(|| "none".to_owned(), |p| p.display().to_string())
        );
    }
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
