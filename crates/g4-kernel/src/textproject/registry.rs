//! The registers `data/ids/*.json` of the kinds without a fixed table (maps, trainers,
//! trainer classes, places) and their baptism; [`Symbols`] resolves the ids of every kind.
//!
//! A register is one JSON object id → number, keys sorted, in the canonical form and without
//! a version field of its own (the manifest carries the version). Ids are christened once in
//! the project language and then frozen: an entry stays when its name changes and also when
//! its number is gone (Task 9 reports that as `textproject.id-dangling`).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::error::{KernelError, io_at};
use crate::text::Problem;

use super::canon::{json_problem, problem, to_canonical, write_if_changed};
use super::locate::key_position;
use super::manifest::SCHEMA_CODE;
use super::symbols::{SymbolKind, fallback_id, normalize, parse_fallback, table};

/// The folder of the registers, relative to the project root.
pub const IDS_DIR: &str = "data/ids";

/// The kinds with a register, in the order [`Symbols`] loads and saves them.
const REGISTRY_KINDS: [SymbolKind; 4] = [
    SymbolKind::Map,
    SymbolKind::Trainer,
    SymbolKind::TrainerClass,
    SymbolKind::Place,
];

/// The register file of `kind` in [`IDS_DIR`]: `maps.json`, `trainers.json`,
/// `trainer_classes.json`, `places.json`; `None` for the kinds with a fixed table.
pub fn registry_file(kind: SymbolKind) -> Option<&'static str> {
    match kind {
        SymbolKind::Map => Some("maps.json"),
        SymbolKind::Trainer => Some("trainers.json"),
        SymbolKind::TrainerClass => Some("trainer_classes.json"),
        SymbolKind::Place => Some("places.json"),
        SymbolKind::Species
        | SymbolKind::Move
        | SymbolKind::Item
        | SymbolKind::Ability
        | SymbolKind::Type => None,
    }
}

/// The register path of `kind` relative to the project root.
fn registry_rel(kind: SymbolKind) -> Option<PathBuf> {
    registry_file(kind).map(|file| Path::new(IDS_DIR).join(file))
}

/// One register: id → number. Every id is the kind's prefix, `_` and a body; a fallback id
/// (`TRAINER_0005`) only ever names its own number, and no number has two ids.
#[derive(Debug, Clone, PartialEq)]
pub struct Registry {
    pub kind: SymbolKind,
    pub ids: BTreeMap<String, u32>,
}

impl Registry {
    /// An empty register of `kind`.
    pub fn new(kind: SymbolKind) -> Registry {
        Registry {
            kind,
            ids: BTreeMap::new(),
        }
    }

    /// The register of `kind` under `root`: empty when the file is missing (or the kind has
    /// none); a file that is no JSON object id → number, or breaks the rules of [`Registry`],
    /// is a [`KernelError::Compile`] naming the file (with line and column for JSON errors).
    pub fn load(root: &Path, kind: SymbolKind) -> Result<Registry, KernelError> {
        let Some(rel) = registry_rel(kind) else {
            return Ok(Registry::new(kind));
        };
        let path = root.join(&rel);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Registry::new(kind));
            }
            Err(e) => return Err(io_at(&path)(e)),
        };
        let ids: BTreeMap<String, u32> = serde_json::from_slice(&bytes)
            .map_err(|e| KernelError::Compile(vec![json_problem(&rel, &bytes, &e, SCHEMA_CODE)]))?;
        let mut problems = duplicate_keys(&rel, &bytes);
        let registry = Registry { kind, ids };
        problems.extend(registry.check(&rel, &bytes));
        if problems.is_empty() {
            Ok(registry)
        } else {
            Err(KernelError::Compile(problems))
        }
    }

    /// The rule breaks of a loaded register `rel` (its text `bytes`), one problem each, at
    /// the id's key.
    fn check(&self, rel: &Path, bytes: &[u8]) -> Vec<Problem> {
        let prefix = self.kind.prefix();
        let mut problems = Vec::new();
        let mut owners: BTreeMap<u32, &str> = BTreeMap::new();
        let at_key = |id: &str, code: &str, message: String| {
            let (line, col) = key_position(bytes, id, 0);
            Problem {
                line,
                col,
                ..problem(rel, code, message)
            }
        };
        for (id, &n) in &self.ids {
            let under_prefix = id
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.len() > 1 && rest.starts_with('_'));
            if !under_prefix {
                problems.push(at_key(
                    id,
                    SCHEMA_CODE,
                    format!("Die ID `{id}` beginnt nicht mit `{prefix}_`."),
                ));
            } else if let Some(own) = parse_fallback(self.kind, id).filter(|&own| own != n) {
                problems.push(at_key(
                    id,
                    SCHEMA_CODE,
                    format!("Die ID `{id}` gehört der Nummer {own}, nicht {n}."),
                ));
            } else if let Some(first) = owners.insert(n, id) {
                // Restore the first owner, so a third id names the first again.
                owners.insert(n, first);
                problems.push(at_key(
                    id,
                    SCHEMA_CODE,
                    format!("Die Nummer {n} hat die IDs `{first}` und `{id}`."),
                ));
            }
        }
        problems
    }

    /// Writes the register in the canonical form (keys sorted) unless the file already holds
    /// exactly that; `true` when it wrote. A kind without a register file writes nothing.
    pub fn save(&self, root: &Path) -> Result<bool, KernelError> {
        let Some(rel) = registry_rel(self.kind) else {
            return Ok(false);
        };
        write_if_changed(root, &rel, &to_canonical(&self.ids))
    }

    /// The id of number `n`, if the register has one.
    pub fn id_of(&self, n: u32) -> Option<&str> {
        self.ids
            .iter()
            .find(|&(_, &m)| m == n)
            .map(|(id, _)| id.as_str())
    }

    /// Gives every number of `names` (index = number) without an entry an id, in number order:
    /// the prefix and the normalized name (`german`: the project language is German), or the
    /// fallback `PREFIX_NNNN` when the name is missing, normalizes to nothing or spells a
    /// fallback id. An id already in the register (frozen) or given earlier in this run is
    /// taken; a taken candidate gets the first free suffix `_2`, `_3`, ….
    pub fn baptize(&mut self, names: &[Option<String>], german: bool) {
        let kind = self.kind;
        let prefix = kind.prefix();
        let numbered: BTreeSet<u32> = self.ids.values().copied().collect();
        let mut taken: BTreeSet<String> = self.ids.keys().cloned().collect();
        for (n, name) in names.iter().enumerate() {
            let Ok(n) = u32::try_from(n) else { break };
            if numbered.contains(&n) {
                continue;
            }
            let candidate = name
                .as_deref()
                .and_then(|name| normalize(name, german))
                .map(|body| format!("{prefix}_{body}"))
                .filter(|id| parse_fallback(kind, id).is_none())
                .unwrap_or_else(|| fallback_id(kind, n));
            let id = if taken.contains(&candidate) {
                (2u32..)
                    .map(|suffix| format!("{candidate}_{suffix}"))
                    .find(|id| !taken.contains(id))
                    .expect("a free suffix exists")
            } else {
                candidate
            };
            taken.insert(id.clone());
            self.ids.insert(id, n);
        }
    }
}

/// A key that appears twice in the JSON object `bytes` (serde keeps the last one silently),
/// one problem per repetition at its key.
fn duplicate_keys(rel: &Path, bytes: &[u8]) -> Vec<Problem> {
    struct Keys(Vec<String>);
    struct KeysVisitor;
    impl<'de> serde::de::Visitor<'de> for KeysVisitor {
        type Value = Keys;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a JSON object")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Keys, A::Error> {
            let mut keys = Vec::new();
            while let Some(key) = map.next_key::<String>()? {
                map.next_value::<serde::de::IgnoredAny>()?;
                keys.push(key);
            }
            Ok(Keys(keys))
        }
    }
    impl<'de> serde::Deserialize<'de> for Keys {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Keys, D::Error> {
            d.deserialize_map(KeysVisitor)
        }
    }
    let Ok(Keys(keys)) = serde_json::from_slice::<Keys>(bytes) else {
        return Vec::new();
    };
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut problems = Vec::new();
    for key in &keys {
        let count = seen.entry(key.as_str()).or_insert(0);
        *count += 1;
        if *count > 1 {
            let (line, col) = key_position(bytes, key, *count - 1);
            problems.push(Problem {
                line,
                col,
                ..problem(
                    rel,
                    SCHEMA_CODE,
                    format!("Die ID `{key}` steht mehrfach in der Datei."),
                )
            });
        }
    }
    problems
}

/// Resolves ids for every kind: the fixed tables of species, moves, items, abilities and types,
/// the registers of maps, trainers, trainer classes and places, and the fallbacks.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbols {
    registries: HashMap<SymbolKind, Registry>,
}

impl Symbols {
    /// The four registers under `root` (missing files are empty); the problems of every broken
    /// register together as one [`KernelError::Compile`].
    pub fn load(root: &Path) -> Result<Symbols, KernelError> {
        let mut registries = HashMap::new();
        let mut problems = Vec::new();
        for kind in REGISTRY_KINDS {
            match Registry::load(root, kind) {
                Ok(registry) => {
                    registries.insert(kind, registry);
                }
                Err(KernelError::Compile(found)) => problems.extend(found),
                Err(e) => return Err(e),
            }
        }
        if problems.is_empty() {
            Ok(Symbols { registries })
        } else {
            Err(KernelError::Compile(problems))
        }
    }

    /// The register of `kind`, to baptize. A kind with a fixed table has none: its register
    /// stays empty in effect, as [`Symbols::id`], [`Symbols::number`] and [`Symbols::save`]
    /// never look at it.
    pub fn registry_mut(&mut self, kind: SymbolKind) -> &mut Registry {
        debug_assert!(
            registry_file(kind).is_some(),
            "{kind:?} has a fixed table, no register"
        );
        self.registries
            .entry(kind)
            .or_insert_with(|| Registry::new(kind))
    }

    /// The id of number `n` of `kind` from its table or register; `None` without an entry
    /// (the caller writes the number then).
    pub fn id(&self, kind: SymbolKind, n: u32) -> Option<String> {
        match table(kind) {
            Some(table) => table.id(n).map(str::to_owned),
            None => self
                .registries
                .get(&kind)
                .and_then(|r| r.id_of(n))
                .map(str::to_owned),
        }
    }

    /// The number of `id` of `kind`: from its table or register, else from the fallback form
    /// `PREFIX_NNNN`; `None` for an unknown id.
    pub fn number(&self, kind: SymbolKind, id: &str) -> Option<u32> {
        let known = match table(kind) {
            Some(table) => table.number(id),
            None => self
                .registries
                .get(&kind)
                .and_then(|r| r.ids.get(id).copied()),
        };
        known.or_else(|| parse_fallback(kind, id))
    }

    /// Writes every register (missing ones too, empty as `{}`) unless unchanged; the number of
    /// files written.
    pub fn save(&self, root: &Path) -> Result<u32, KernelError> {
        let mut written = 0;
        for kind in REGISTRY_KINDS {
            let saved = match self.registries.get(&kind) {
                Some(registry) => registry.save(root)?,
                None => Registry::new(kind).save(root)?,
            };
            if saved {
                written += 1;
            }
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::text::Severity;

    fn names(list: &[Option<&str>]) -> Vec<Option<String>> {
        list.iter().map(|n| n.map(str::to_owned)).collect()
    }

    fn registry(kind: SymbolKind, entries: &[(&str, u32)]) -> Registry {
        Registry {
            kind,
            ids: entries.iter().map(|(id, n)| (id.to_string(), *n)).collect(),
        }
    }

    fn compile_problems(err: KernelError) -> Vec<crate::text::Problem> {
        match err {
            KernelError::Compile(problems) => problems,
            other => panic!("expected Compile, got {other:?}"),
        }
    }

    #[test]
    fn register_files_per_kind() {
        assert_eq!(registry_file(SymbolKind::Map), Some("maps.json"));
        assert_eq!(registry_file(SymbolKind::Trainer), Some("trainers.json"));
        assert_eq!(
            registry_file(SymbolKind::TrainerClass),
            Some("trainer_classes.json")
        );
        assert_eq!(registry_file(SymbolKind::Place), Some("places.json"));
        assert_eq!(registry_file(SymbolKind::Species), None);
        assert_eq!(registry_file(SymbolKind::Type), None);
    }

    #[test]
    fn baptism_suffixes_collisions_in_number_order() {
        let mut reg = Registry::new(SymbolKind::Trainer);
        reg.baptize(&names(&[Some("Kai"), Some("Kai"), None, Some("Kai")]), true);
        assert_eq!(reg.id_of(0), Some("TRAINER_KAI"));
        assert_eq!(reg.id_of(1), Some("TRAINER_KAI_2"));
        assert_eq!(reg.id_of(2), Some("TRAINER_0002"));
        assert_eq!(reg.id_of(3), Some("TRAINER_KAI_3"));
        assert_eq!(reg.ids.len(), 4);
        assert_eq!(reg.id_of(4), None);
    }

    #[test]
    fn rebaptism_keeps_frozen_ids_and_suffixes_new_ones() {
        let mut reg = registry(
            SymbolKind::Trainer,
            &[("TRAINER_KAI", 0), ("TRAINER_KAI_2", 1)],
        );
        reg.baptize(&names(&[Some("Udo"), Some("Kai"), Some("Kai")]), true);
        assert_eq!(
            reg.id_of(0),
            Some("TRAINER_KAI"),
            "frozen despite the new name"
        );
        assert_eq!(reg.id_of(1), Some("TRAINER_KAI_2"));
        assert_eq!(reg.id_of(2), Some("TRAINER_KAI_3"));
        assert_eq!(reg.ids.len(), 3, "no id for Udo");

        // An entry of a vanished number stays and keeps its id taken.
        let mut reg = registry(SymbolKind::Trainer, &[("TRAINER_KAI", 9)]);
        reg.baptize(&names(&[Some("Kai")]), true);
        assert_eq!(reg.id_of(9), Some("TRAINER_KAI"));
        assert_eq!(reg.id_of(0), Some("TRAINER_KAI_2"));
    }

    #[test]
    fn a_name_that_equals_a_fallback_is_treated_as_empty() {
        let mut list = vec![None; 8];
        list[5] = Some("0007".to_owned());
        let mut reg = Registry::new(SymbolKind::Trainer);
        reg.baptize(&list, true);
        assert_eq!(reg.id_of(5), Some("TRAINER_0005"));
        assert_eq!(reg.id_of(7), Some("TRAINER_0007"));

        // Only digits: the fallback of its own number, never the one the name spells.
        let mut reg = Registry::new(SymbolKind::Map);
        reg.baptize(&names(&[Some("0001"), None]), false);
        assert_eq!(reg.id_of(0), Some("MAP_0000"));
        assert_eq!(reg.id_of(1), Some("MAP_0001"));
    }

    #[test]
    fn baptism_uses_the_project_language() {
        let mut reg = Registry::new(SymbolKind::Place);
        reg.baptize(&names(&[Some("Mühlental Süd")]), true);
        assert_eq!(reg.id_of(0), Some("PLACE_MUEHLENTAL_SUED"));
        let mut reg = Registry::new(SymbolKind::Place);
        reg.baptize(&names(&[Some("Mühlental Süd")]), false);
        assert_eq!(reg.id_of(0), Some("PLACE_MUHLENTAL_SUD"));
    }

    #[test]
    fn registry_file_is_sorted_canonical_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let reg = registry(
            SymbolKind::Map,
            &[("MAP_T01", 3), ("MAP_C01", 10), ("MAP_0000", 0)],
        );
        assert!(reg.save(root).unwrap());
        let bytes = std::fs::read(root.join("data/ids/maps.json")).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\n  \"MAP_0000\": 0,\n  \"MAP_C01\": 10,\n  \"MAP_T01\": 3\n}\n"
        );
        assert!(!reg.save(root).unwrap(), "unchanged: not written");
        assert_eq!(Registry::load(root, SymbolKind::Map).unwrap(), reg);

        // A missing file is an empty register; an empty one is written as `{}`.
        let empty = Registry::load(root, SymbolKind::Place).unwrap();
        assert_eq!(empty, Registry::new(SymbolKind::Place));
        assert!(empty.save(root).unwrap());
        assert_eq!(
            std::fs::read(root.join("data/ids/places.json")).unwrap(),
            b"{}\n"
        );

        // A hand-written file in another key order loads to the same register.
        std::fs::write(
            root.join("data/ids/maps.json"),
            "{\"MAP_T01\": 3, \"MAP_0000\": 0, \"MAP_C01\": 10}",
        )
        .unwrap();
        assert_eq!(Registry::load(root, SymbolKind::Map).unwrap(), reg);
    }

    #[test]
    fn broken_registry_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(IDS_DIR)).unwrap();
        std::fs::write(
            root.join("data/ids/trainers.json"),
            "{\n  \"TRAINER_KAI\": 0,\n  \"TRAINER_UDO\": x\n}\n",
        )
        .unwrap();
        let problems = compile_problems(Registry::load(root, SymbolKind::Trainer).unwrap_err());
        assert_eq!(problems.len(), 1);
        let p = &problems[0];
        assert_eq!(p.severity, Severity::Error);
        assert_eq!(p.file, PathBuf::from("data/ids/trainers.json"));
        assert_eq!((p.line, p.col), (3, 18));
        assert!(
            p.message.starts_with("textproject.schema: "),
            "{}",
            p.message
        );

        // A number out of range, a number given twice, a foreign prefix and a fallback of
        // another number are refused too, each with the file.
        for (text, needle) in [
            ("{\"TRAINER_KAI\": -1}", "expected u32"),
            ("{\"TRAINER_KAI\": 1, \"TRAINER_UDO\": 1}", "TRAINER_UDO"),
            ("{\"MAP_T01\": 1}", "MAP_T01"),
            ("{\"TRAINER_0005\": 7}", "TRAINER_0005"),
        ] {
            std::fs::write(root.join("data/ids/trainers.json"), text).unwrap();
            let problems = compile_problems(Registry::load(root, SymbolKind::Trainer).unwrap_err());
            assert_eq!(problems.len(), 1, "{text}");
            assert_eq!(problems[0].file, PathBuf::from("data/ids/trainers.json"));
            assert!(
                problems[0].message.starts_with("textproject.schema: ")
                    && problems[0].message.contains(needle),
                "{text}: {}",
                problems[0].message
            );
        }

        // A rule break points at its id; an id given twice (serde would keep the last) is
        // refused at its second key.
        std::fs::write(
            root.join("data/ids/trainers.json"),
            "{\n  \"TRAINER_KAI\": 0,\n  \"MAP_T01\": 1\n}\n",
        )
        .unwrap();
        let problems = compile_problems(Registry::load(root, SymbolKind::Trainer).unwrap_err());
        assert_eq!((problems[0].line, problems[0].col), (3, 3));
        std::fs::write(
            root.join("data/ids/trainers.json"),
            "{\n  \"TRAINER_KAI\": 0,\n  \"TRAINER_KAI\": 1\n}\n",
        )
        .unwrap();
        let problems = compile_problems(Registry::load(root, SymbolKind::Trainer).unwrap_err());
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].message.contains("mehrfach"));
        assert_eq!((problems[0].line, problems[0].col), (3, 3));

        // Symbols::load collects the problems of every register.
        std::fs::write(root.join("data/ids/trainers.json"), "{\"TRAINER_KAI\": -1}").unwrap();
        std::fs::write(root.join("data/ids/maps.json"), "[").unwrap();
        let problems = compile_problems(Symbols::load(root).err().unwrap());
        let files: Vec<_> = problems.iter().map(|p| p.file.clone()).collect();
        assert_eq!(
            files,
            vec![
                PathBuf::from("data/ids/maps.json"),
                PathBuf::from("data/ids/trainers.json")
            ]
        );
    }

    #[test]
    fn symbols_resolve_ids_registries_and_fallbacks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(IDS_DIR)).unwrap();
        std::fs::write(root.join("data/ids/maps.json"), "{\"MAP_T01\": 3}\n").unwrap();
        let mut symbols = Symbols::load(root).unwrap();

        assert_eq!(
            symbols.number(SymbolKind::Species, "SPECIES_PIKACHU"),
            Some(25)
        );
        assert_eq!(
            symbols.number(SymbolKind::Species, "SPECIES_0600"),
            Some(600)
        );
        assert_eq!(symbols.number(SymbolKind::Map, "MAP_T01"), Some(3));
        assert_eq!(symbols.number(SymbolKind::Map, "MAP_0005"), Some(5));
        assert_eq!(symbols.number(SymbolKind::Map, "MAP_NOWHERE"), None);
        assert_eq!(symbols.number(SymbolKind::Species, "SPECIES_NOBODY"), None);
        assert_eq!(symbols.number(SymbolKind::Species, "MOVE_0001"), None);

        assert_eq!(
            symbols.id(SymbolKind::Species, 25).as_deref(),
            Some("SPECIES_PIKACHU")
        );
        assert_eq!(symbols.id(SymbolKind::Species, 600), None);
        assert_eq!(symbols.id(SymbolKind::Map, 3).as_deref(), Some("MAP_T01"));
        assert_eq!(symbols.id(SymbolKind::Map, 4), None);
        assert_eq!(symbols.id(SymbolKind::Place, 0), None);

        symbols
            .registry_mut(SymbolKind::Place)
            .baptize(&names(&[Some("Kleindorf")]), true);
        assert_eq!(
            symbols.id(SymbolKind::Place, 0).as_deref(),
            Some("PLACE_KLEINDORF")
        );
        assert_eq!(
            symbols.number(SymbolKind::Place, "PLACE_KLEINDORF"),
            Some(0)
        );

        // Every register is written; only the changed or missing ones count. The hand-written
        // `maps.json` is not canonical, so it is rewritten too.
        assert_eq!(
            symbols.save(root).unwrap(),
            4,
            "maps, places, trainers, classes"
        );
        assert_eq!(symbols.save(root).unwrap(), 0);
        assert_eq!(
            std::fs::read(root.join("data/ids/maps.json")).unwrap(),
            b"{\n  \"MAP_T01\": 3\n}\n"
        );
        assert_eq!(
            std::fs::read(root.join("data/ids/trainers.json")).unwrap(),
            b"{}\n"
        );
        assert_eq!(
            std::fs::read(root.join("data/ids/places.json")).unwrap(),
            b"{\n  \"PLACE_KLEINDORF\": 0\n}\n"
        );
        let again = Symbols::load(root).unwrap();
        assert_eq!(again.number(SymbolKind::Place, "PLACE_KLEINDORF"), Some(0));
        assert_eq!(again.number(SymbolKind::Map, "MAP_T01"), Some(3));
    }
}
