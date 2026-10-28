//! An id or a number for every kind ([`RefInput`]) and the [`Resolver`] that turns one into
//! the other for a project: the files beside the text project (`g4-world.json`,
//! `g4-checks.json`), the recipes of the templates and the MCP arguments read either, and a
//! text project writes ids where a number has one.
//!
//! What resolves, by kind: a number always (its range is the caller's); the fallback
//! `PREFIX_NNNN` always; the fixed tables of species, moves, items, abilities and types
//! everywhere; the registers of maps, trainers, trainer classes and places only in a text
//! project of the current schema; and a map also as `MAP_<internal name>` from `mapname.bin`
//! (also after a rename and without a text project), in [`Resolver::parse`] also as the bare
//! internal name.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::project::Project;

use super::MANIFEST_FILE;
use super::manifest::{SchemaState, schema_state};
use super::registry::{IDS_DIR, Symbols, registry_file};
use super::symbols::{SymbolKind, normalize, parse_fallback, table};

/// A reference as a project file or an argument writes it: a number, or an id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RefInput {
    /// `25`.
    Number(u32),
    /// `"SPECIES_PIKACHU"` (or a number as text, `"25"`).
    Id(String),
}

/// Number 0, as a field that may be left out reads (the project subject of `g4-checks.json`).
impl Default for RefInput {
    fn default() -> Self {
        RefInput::Number(0)
    }
}

/// The error code of an id no table, register or internal name knows.
pub const UNKNOWN_ID_CODE: &str = "unknown_id";

/// The sentence for an id nothing knows: `Unbekannte ID: {id}.`
pub fn unknown_id_text(id: &str) -> String {
    format!("Unbekannte ID: {id}.")
}

/// The kinds with a register; outside a text project they resolve by number and fallback
/// only (maps also by internal name).
const REGISTER_KINDS: [SymbolKind; 4] = [
    SymbolKind::Map,
    SymbolKind::Trainer,
    SymbolKind::TrainerClass,
    SymbolKind::Place,
];

/// Size and modification time of a file, `None` when it is missing.
pub(crate) type Stamp = Option<(u64, SystemTime)>;

/// Resolves ids of every kind for one project; [`Resolver::is_current`] tells when to make
/// a new one.
#[derive(Debug, Clone)]
pub struct Resolver {
    /// The project root, `None` for [`Resolver::fixed`].
    root: Option<PathBuf>,
    /// The registers, only in a text project of the current schema.
    symbols: Option<Symbols>,
    /// Internal name (as `mapname.bin` has it and normalized) → the first header with it.
    internal: HashMap<String, u32>,
    /// The files read, relative to the root, with their stamp from before reading.
    stamps: Vec<(PathBuf, Stamp)>,
}

impl Resolver {
    /// The resolver of `project`: its registers when it is a text project of the current
    /// schema, and the internal names of its headers. A broken manifest or register, or a
    /// `mapname.bin` that is there but cannot be read, is the error of reading it; without
    /// `mapname.bin` there are no internal names (everything else still resolves).
    pub fn for_project(project: &Project) -> Result<Resolver, KernelError> {
        let root = project.root.clone();
        // Stamped before reading, so a change while reading shows as not current.
        let stamps: Vec<(PathBuf, Stamp)> = watched(project)
            .into_iter()
            .map(|rel| {
                let stamp = stamp(&root.join(&rel));
                (rel, stamp)
            })
            .collect();
        let symbols = match schema_state(&root)? {
            SchemaState::Current => Some(Symbols::load(&root)?),
            SchemaState::Missing | SchemaState::Older(_) | SchemaState::Newer(_) => None,
        };
        let names = if project.mapname_path().exists() {
            project.internal_names()?
        } else {
            Vec::new()
        };
        // Every exact name first, so a normalized name never shadows another header's own.
        let mut internal = HashMap::new();
        for (n, name) in (0u32..).zip(&names) {
            if !name.is_empty() {
                internal.entry(name.clone()).or_insert(n);
            }
        }
        for (n, name) in (0u32..).zip(&names) {
            if let Some(body) = normalize(name, false) {
                internal.entry(body).or_insert(n);
            }
        }
        Ok(Resolver {
            root: Some(root),
            symbols,
            internal,
            stamps,
        })
    }

    /// Without a project: the fixed tables and the fallbacks only (recipes, tests).
    pub fn fixed() -> Resolver {
        Resolver {
            root: None,
            symbols: None,
            internal: HashMap::new(),
            stamps: Vec::new(),
        }
    }

    /// The number of `input`: a number as it is, an id (or a number as text) as
    /// [`Resolver::parse`] reads it, but never a bare internal name (`"T01"`).
    pub fn number(&self, kind: SymbolKind, input: &RefInput) -> Result<u32, String> {
        match input {
            RefInput::Number(n) => Ok(*n),
            RefInput::Id(text) => self.resolve(kind, text, false),
        }
    }

    /// `"25"`, `"SPECIES_PIKACHU"`, `"MAP_T01"`, `"T01"` (only for maps: an internal name).
    /// An unknown id is [`unknown_id_text`]; an id of a register kind outside a text project
    /// says that it needs one.
    pub fn parse(&self, kind: SymbolKind, text: &str) -> Result<u32, String> {
        self.resolve(kind, text, true)
    }

    /// [`Resolver::parse`]; `bare`: a map may also be named by its bare internal name.
    fn resolve(&self, kind: SymbolKind, text: &str, bare: bool) -> Result<u32, String> {
        if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
            return text.parse().map_err(|_| unknown_id_text(text));
        }
        let known = match (table(kind), &self.symbols) {
            (Some(table), _) => table.number(text),
            (None, Some(symbols)) => symbols.number(kind, text),
            (None, None) => None,
        };
        if let Some(n) = known.or_else(|| parse_fallback(kind, text)) {
            return Ok(n);
        }
        if kind == SymbolKind::Map {
            let body = match text.strip_prefix("MAP_") {
                Some(body) => Some(body),
                None if bare => Some(text),
                None => None,
            };
            if let Some(&n) = body.and_then(|body| self.internal.get(body)) {
                return Ok(n);
            }
            return Err(unknown_id_text(text));
        }
        if self.symbols.is_none() && REGISTER_KINDS.contains(&kind) {
            return Err(format!("Unbekannte ID: {text} (kein Textprojekt)."));
        }
        Err(unknown_id_text(text))
    }

    /// The id of number `n` of `kind` from its table, or from its register in a text
    /// project; `None` without an entry.
    pub fn id(&self, kind: SymbolKind, n: u32) -> Option<String> {
        match (table(kind), &self.symbols) {
            (Some(table), _) => table.id(n).map(str::to_owned),
            (None, Some(symbols)) => symbols.id(kind, n),
            (None, None) => None,
        }
    }

    /// Whether project files write ids (a text project of the current schema).
    pub fn writes_ids(&self) -> bool {
        self.symbols.is_some()
    }

    /// The form a project file writes: `Id` in a text project when the number has one, else
    /// `Number`.
    pub fn output(&self, kind: SymbolKind, n: u32) -> RefInput {
        if self.writes_ids()
            && let Some(id) = self.id(kind, n)
        {
            return RefInput::Id(id);
        }
        RefInput::Number(n)
    }

    /// Whether the registers changed on disk since this was made (one `stat` per register,
    /// and one each for the manifest and `mapname.bin`, which decide whether the registers
    /// count and what the internal names are). A resolver of another project or without one
    /// is never current.
    pub fn is_current(&self, project: &Project) -> bool {
        self.root.as_deref() == Some(project.root.as_path())
            && self
                .stamps
                .iter()
                .all(|(rel, was)| stamp(&project.root.join(rel)) == *was)
    }
}

/// The files a resolver reads, relative to the root: the manifest, the registers and
/// `mapname.bin`.
pub(crate) fn watched(project: &Project) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from(MANIFEST_FILE)];
    files.extend(
        REGISTER_KINDS
            .into_iter()
            .filter_map(registry_file)
            .map(|file| Path::new(IDS_DIR).join(file)),
    );
    let mapname = project.mapname_path();
    files.push(
        mapname
            .strip_prefix(&project.root)
            .map(Path::to_path_buf)
            .unwrap_or(mapname),
    );
    files
}

pub(crate) fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_support::TextFixture;

    const MAPS: &str = "data/ids/maps.json";

    fn id(text: &str) -> RefInput {
        RefInput::Id(text.to_owned())
    }

    #[test]
    fn numbers_and_fixed_ids_resolve_everywhere() {
        let fixed = Resolver::fixed();
        let dspre = TextFixture::dspre();
        let exported = TextFixture::exported();
        for r in [
            fixed,
            Resolver::for_project(&dspre.project()).unwrap(),
            Resolver::for_project(&exported.project()).unwrap(),
        ] {
            assert_eq!(
                r.number(SymbolKind::Species, &id("SPECIES_PIKACHU")),
                Ok(25)
            );
            assert_eq!(r.number(SymbolKind::Species, &RefInput::Number(25)), Ok(25));
            assert_eq!(r.parse(SymbolKind::Species, "25"), Ok(25));
            assert_eq!(r.parse(SymbolKind::Species, "SPECIES_PIKACHU"), Ok(25));
            assert_eq!(r.number(SymbolKind::Species, &id("SPECIES_0600")), Ok(600));
            assert_eq!(r.number(SymbolKind::Move, &id("MOVE_POUND")), Ok(1));
            // A number is always a number, whatever the kind; the range is the caller's.
            assert_eq!(r.parse(SymbolKind::Trainer, "70000"), Ok(70000));
            // The fallback of every kind, also of the kinds with a register.
            assert_eq!(r.parse(SymbolKind::Trainer, "TRAINER_0001"), Ok(1));
            assert_eq!(r.parse(SymbolKind::Map, "MAP_0001"), Ok(1));
            assert_eq!(
                r.id(SymbolKind::Species, 25).as_deref(),
                Some("SPECIES_PIKACHU")
            );
            assert_eq!(r.id(SymbolKind::Species, 9999), None);
        }
    }

    #[test]
    fn map_ids_resolve_by_register_and_by_internal_name() {
        let fx = TextFixture::exported();
        let r = Resolver::for_project(&fx.project()).unwrap();
        assert_eq!(r.number(SymbolKind::Map, &id("MAP_T01")), Ok(0));
        assert_eq!(r.parse(SymbolKind::Map, "MAP_T02"), Ok(1));
        assert_eq!(r.parse(SymbolKind::Map, "T01"), Ok(0));
        assert_eq!(r.id(SymbolKind::Map, 0).as_deref(), Some("MAP_T01"));
        // Only maps have a bare internal name.
        assert!(r.parse(SymbolKind::Trainer, "T01").is_err());

        // Renamed in the register: the new id holds, and the internal name still does.
        fx.edit(MAPS, "\"MAP_T01\"", "\"MAP_HOME\"");
        let r = Resolver::for_project(&fx.project()).unwrap();
        assert_eq!(r.parse(SymbolKind::Map, "MAP_HOME"), Ok(0));
        assert_eq!(r.parse(SymbolKind::Map, "MAP_T01"), Ok(0));
        assert_eq!(r.parse(SymbolKind::Map, "T01"), Ok(0));
        assert_eq!(r.id(SymbolKind::Map, 0).as_deref(), Some("MAP_HOME"));

        // Without a text project the internal names alone, read from mapname.bin.
        let dspre = TextFixture::dspre();
        let r = Resolver::for_project(&dspre.project()).unwrap();
        assert_eq!(r.parse(SymbolKind::Map, "MAP_T02"), Ok(1));
        assert_eq!(r.parse(SymbolKind::Map, "T02"), Ok(1));
        assert_eq!(
            r.parse(SymbolKind::Map, "MAP_HOME"),
            Err("Unbekannte ID: MAP_HOME.".to_owned())
        );
        // `number` reads ids, never a bare internal name.
        assert_eq!(r.number(SymbolKind::Map, &id("MAP_T01")), Ok(0));
        assert_eq!(
            r.number(SymbolKind::Map, &id("T01")),
            Err("Unbekannte ID: T01.".to_owned())
        );
        // Fixed resolvers know no internal names.
        assert!(Resolver::fixed().parse(SymbolKind::Map, "MAP_T01").is_err());
    }

    #[test]
    fn an_exact_internal_name_wins_over_a_normalized_one() {
        let fx = TextFixture::dspre();
        // Header 0 is `t01` (normalized `T01`), header 1 is exactly `T01`.
        let mut names = Vec::new();
        for name in ["t01", "T01"] {
            let mut record = [0u8; 16];
            record[..name.len()].copy_from_slice(name.as_bytes());
            names.extend_from_slice(&record);
        }
        fx.write("files/fielddata/maptable/mapname.bin", &names);
        let r = Resolver::for_project(&fx.project()).unwrap();
        assert_eq!(r.parse(SymbolKind::Map, "MAP_T01"), Ok(1));
        assert_eq!(r.parse(SymbolKind::Map, "T01"), Ok(1));
        assert_eq!(r.parse(SymbolKind::Map, "MAP_t01"), Ok(0));
    }

    #[test]
    fn without_mapname_bin_everything_but_internal_names_resolves() {
        let fx = TextFixture::exported();
        std::fs::remove_file(fx.project().mapname_path()).unwrap();
        let r = Resolver::for_project(&fx.project()).unwrap();
        assert_eq!(r.parse(SymbolKind::Species, "SPECIES_PIKACHU"), Ok(25));
        assert_eq!(r.parse(SymbolKind::Trainer, "TRAINER_YOUNGSTER_KAI"), Ok(1));
        // The register still names the map; the internal name is gone.
        assert_eq!(r.parse(SymbolKind::Map, "MAP_T01"), Ok(0));
        assert_eq!(
            r.parse(SymbolKind::Map, "T02"),
            Err("Unbekannte ID: T02.".to_owned())
        );
        assert!(r.is_current(&fx.project()));
    }

    #[test]
    fn register_kinds_need_a_text_project() {
        let dspre = TextFixture::dspre();
        let r = Resolver::for_project(&dspre.project()).unwrap();
        for (kind, text) in [
            (SymbolKind::Trainer, "TRAINER_YOUNGSTER_KAI"),
            (SymbolKind::TrainerClass, "TRAINER_CLASS_YOUNGSTER"),
            (SymbolKind::Place, "PLACE_TWINLEAF_TOWN"),
        ] {
            assert_eq!(
                r.parse(kind, text),
                Err(format!("Unbekannte ID: {text} (kein Textprojekt)."))
            );
        }
        let err = Resolver::fixed()
            .number(SymbolKind::Trainer, &id("TRAINER_YOUNGSTER_KAI"))
            .unwrap_err();
        assert!(err.contains("kein Textprojekt"), "{err}");

        let exported = TextFixture::exported();
        let r = Resolver::for_project(&exported.project()).unwrap();
        assert_eq!(
            r.number(SymbolKind::Trainer, &id("TRAINER_YOUNGSTER_KAI")),
            Ok(1)
        );
        assert_eq!(
            r.id(SymbolKind::Trainer, 1).as_deref(),
            Some("TRAINER_YOUNGSTER_KAI")
        );
        assert_eq!(
            r.parse(SymbolKind::TrainerClass, "TRAINER_CLASS_YOUNGSTER"),
            Ok(1)
        );
        // In a text project an id the register lacks is plainly unknown.
        assert_eq!(
            r.parse(SymbolKind::Trainer, "TRAINER_NOBODY"),
            Err("Unbekannte ID: TRAINER_NOBODY.".to_owned())
        );
    }

    #[test]
    fn unknown_ids_name_themselves() {
        assert_eq!(UNKNOWN_ID_CODE, "unknown_id");
        assert_eq!(
            unknown_id_text("SPECIES_PIKACHUU"),
            "Unbekannte ID: SPECIES_PIKACHUU."
        );
        let r = Resolver::fixed();
        for (kind, text) in [
            (SymbolKind::Species, "SPECIES_PIKACHUU"),
            // Another kind's id, and another kind's fallback.
            (SymbolKind::Species, "MOVE_POUND"),
            (SymbolKind::Item, "MOVE_0001"),
            (SymbolKind::Species, "pikachu"),
            (SymbolKind::Species, ""),
            (SymbolKind::Species, "-1"),
            (SymbolKind::Species, "99999999999"),
        ] {
            assert_eq!(
                r.parse(kind, text),
                Err(unknown_id_text(text)),
                "{kind:?} {text}"
            );
        }
        assert_eq!(
            r.number(SymbolKind::Species, &id("SPECIES_PIKACHUU")),
            Err("Unbekannte ID: SPECIES_PIKACHUU.".to_owned())
        );
    }

    #[test]
    fn output_writes_ids_only_in_a_text_project() {
        let fixed = Resolver::fixed();
        assert!(!fixed.writes_ids());
        assert_eq!(fixed.output(SymbolKind::Species, 25), RefInput::Number(25));

        let dspre = TextFixture::dspre();
        let r = Resolver::for_project(&dspre.project()).unwrap();
        assert!(!r.writes_ids());
        assert_eq!(r.output(SymbolKind::Species, 25), RefInput::Number(25));
        assert_eq!(r.output(SymbolKind::Map, 0), RefInput::Number(0));

        let exported = TextFixture::exported();
        let r = Resolver::for_project(&exported.project()).unwrap();
        assert!(r.writes_ids());
        assert_eq!(r.output(SymbolKind::Species, 25), id("SPECIES_PIKACHU"));
        assert_eq!(r.output(SymbolKind::Map, 1), id("MAP_T02"));
        assert_eq!(
            r.output(SymbolKind::Trainer, 1),
            id("TRAINER_YOUNGSTER_KAI")
        );
        // A number without an entry stays a number, also in a text project.
        assert_eq!(r.output(SymbolKind::Species, 9999), RefInput::Number(9999));
        assert_eq!(r.output(SymbolKind::Map, 77), RefInput::Number(77));

        // A manifest of another schema version writes no ids.
        exported.write(
            "g4-project.json",
            b"{\"format\": \"g4-project\", \"schema\": 0}\n",
        );
        let r = Resolver::for_project(&exported.project()).unwrap();
        assert!(!r.writes_ids());
        assert_eq!(r.output(SymbolKind::Species, 25), RefInput::Number(25));
    }

    #[test]
    fn ref_input_reads_number_or_string() {
        let read = |json: &str| serde_json::from_str::<RefInput>(json);
        assert_eq!(read("25").unwrap(), RefInput::Number(25));
        assert_eq!(read("\"SPECIES_PIKACHU\"").unwrap(), id("SPECIES_PIKACHU"));
        assert_eq!(read("\"25\"").unwrap(), id("25"));
        assert!(read("-1").is_err());
        assert!(read("2.5").is_err());
        assert!(read("null").is_err());
        assert_eq!(serde_json::to_string(&RefInput::Number(25)).unwrap(), "25");
        assert_eq!(
            serde_json::to_string(&id("MAP_T01")).unwrap(),
            "\"MAP_T01\""
        );
        // A string holding a number resolves as the number.
        assert_eq!(
            Resolver::fixed().number(SymbolKind::Species, &id("25")),
            Ok(25)
        );
    }

    #[test]
    fn is_current_sees_a_register_change() {
        let fx = TextFixture::exported();
        let r = Resolver::for_project(&fx.project()).unwrap();
        assert!(r.is_current(&fx.project()));

        fx.edit(MAPS, "\"MAP_T01\"", "\"MAP_HOME\"");
        assert!(!r.is_current(&fx.project()));
        let r = Resolver::for_project(&fx.project()).unwrap();
        assert!(r.is_current(&fx.project()));

        // An edit of the same size shows by its modification time.
        let size = fx.read(MAPS).len();
        fx.edit(MAPS, "\"MAP_HOME\"", "\"MAP_T999\"");
        assert_eq!(fx.read(MAPS).len(), size);
        assert!(!r.is_current(&fx.project()));

        // A register that appears (the folder becomes a text project) is a change too.
        let dspre = TextFixture::dspre();
        let r = Resolver::for_project(&dspre.project()).unwrap();
        assert!(r.is_current(&dspre.project()));
        dspre.write("data/ids/trainers.json", b"{}\n");
        assert!(!r.is_current(&dspre.project()));

        // A resolver of another project, or one without a project, is not current.
        assert!(!r.is_current(&fx.project()));
        assert!(!Resolver::fixed().is_current(&fx.project()));
    }
}
