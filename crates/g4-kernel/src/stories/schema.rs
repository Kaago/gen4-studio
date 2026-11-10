//! The story file `data/stories/NNNN.json`, its side file `NNNN.layout.json`, and the check of
//! one story file ([`check_file`]).
//!
//! Every type refuses unknown fields and is written in the canonical form
//! ([`to_canonical`](crate::textproject::canon::to_canonical)), keys in the order of the type.
//! A step's `key` is unique in its story; `value` is the value of the story's progress variable
//! (or flag) the step stands for. The references in `progress`, `uses` and `flags` are weak:
//! the check warns about one that names nothing or a deleted entity ([`DANGLING_REF_CODE`]).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::project::Project;
use crate::text::{Problem, Severity};
use crate::textproject::arts::file_names;
use crate::textproject::canon::{parse_json, problem};
use crate::textproject::codes::{ID_DANGLING_CODE, ID_MISMATCH_CODE, SCHEMA_CODE};
use crate::textproject::ids::Resolver;
use crate::textproject::locate::{key_position, position};
use crate::textproject::registry::{IDS_DIR, Registry, registry_file};
use crate::textproject::symbols::SymbolKind;

use super::uses::{OnDisk, UseTarget, number, resolve};

/// The folder of the stories, relative to the project root.
pub const STORIES_DIR: &str = "data/stories";

/// The code of a reference of a story that names nothing in the project (a warning).
pub const DANGLING_REF_CODE: &str = "story.dangling-ref";

/// What a step of a story is; the user interface says it in its language ([`StepKind::word`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Start,
    Dialog,
    Condition,
    Battle,
    Reward,
    Progress,
    /// Appearing or disappearing; the step's name says which.
    Appearance,
}

impl StepKind {
    /// Every kind, in the order of the user interface.
    pub fn all() -> &'static [StepKind] {
        &[
            StepKind::Start,
            StepKind::Dialog,
            StepKind::Condition,
            StepKind::Battle,
            StepKind::Reward,
            StepKind::Progress,
            StepKind::Appearance,
        ]
    }

    /// `Start`, `Dialog`, `Condition`, `Battle`, `Reward`, `Progress`, `Appear/disappear`, in
    /// the current language.
    pub fn word(self) -> String {
        let key = match self {
            StepKind::Start => "stories.step_start",
            StepKind::Dialog => "stories.step_dialog",
            StepKind::Condition => "stories.step_condition",
            StepKind::Battle => "stories.step_battle",
            StepKind::Reward => "stories.step_reward",
            StepKind::Progress => "stories.step_progress",
            StepKind::Appearance => "stories.step_appearance",
        };
        t!(key).into_owned()
    }
}

/// The number a story's progress is kept in: `{ "var": "VAR_…" }` or `{ "flag": "FLAG_…" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum Progress {
    Var(String),
    Flag(String),
}

/// One thing of the game a step uses, exactly one key each: an id (`trainer`, `map`, `flag`,
/// `var`) or a place by numbers (`object`, `script`, `message`, `levelScript`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum Use {
    Trainer(String),
    Map(String),
    Flag(String),
    Var(String),
    Object { file: u16, local_id: u16 },
    Script { file: u16, slot: u16 },
    Message { archive: u16, index: u16 },
    LevelScript { file: u16, condition: u16 },
}

/// Which recipe made a step, and in which Studio version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Made {
    pub recipe: String,
    pub studio: String,
}

/// One step of a story: a value of its progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub key: String,
    pub value: u16,
    pub kind: StepKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made: Option<Made>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<Use>,
}

/// A story: `data/stories/NNNN.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Story {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    pub progress: Progress,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
}

/// Where a node sits in the graph.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// The side file `data/stories/NNNN.layout.json`: the position of each node by the key of its
/// step. A missing file is no error (the graph lays itself out).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub nodes: BTreeMap<String, Position>,
}

/// The story file of number `n`, relative to the root: `data/stories/0000.json`.
pub fn story_rel(n: u16) -> PathBuf {
    Path::new(STORIES_DIR).join(format!("{n:04}.json"))
}

/// The side file of story `n`, relative to the root: `data/stories/0000.layout.json`.
pub fn layout_rel(n: u16) -> PathBuf {
    Path::new(STORIES_DIR).join(format!("{n:04}.layout.json"))
}

/// Reads and judges one story file `bytes` of number `n`: not JSON or not the schema →
/// `textproject.schema` (file, line, col via `parse_json`); `id` without the `STORY_` prefix or
/// ≠ the register's id of `n` → `textproject.id-mismatch`; a duplicate step `key` →
/// `textproject.schema`; `progress`/`uses`/`flags` that name no known id (resolver), a use of
/// what the project on disk does not have (a number past the count of its art, a trainer or
/// header whose file is gone) or of an entity whose JSON carries a tombstone (3d.4) → warning
/// [`DANGLING_REF_CODE`]. The id is judged even when the rest does not read.
pub fn check_file(
    project: &Project,
    registry: &Registry,
    resolver: &Resolver,
    n: u16,
    bytes: &[u8],
) -> Vec<Problem> {
    check_with(&OnDisk::new(project), registry, resolver, n, bytes)
}

/// [`check_file`] with what is on disk read once for every story of a check.
fn check_with(
    disk: &OnDisk<'_>,
    registry: &Registry,
    resolver: &Resolver,
    n: u16,
    bytes: &[u8],
) -> Vec<Problem> {
    let (story, mut problems) = judge(registry, n, bytes);
    if let Some(story) = story {
        problems.extend(dangling(&story, resolver, disk, &story_rel(n), bytes));
    }
    problems
}

/// The errors of story file `bytes` of number `n` (what [`check_file`] says without the
/// resolver: id, schema, duplicate keys) and the story when it reads.
pub(crate) fn judge(registry: &Registry, n: u16, bytes: &[u8]) -> (Option<Story>, Vec<Problem>) {
    let rel = story_rel(n);
    let mut problems: Vec<Problem> = id_problem(registry, n, &rel, bytes).into_iter().collect();
    let story: Story = match parse_json(&rel, bytes, SCHEMA_CODE) {
        Ok(story) => story,
        Err(found) => {
            problems.push(found);
            return (None, problems);
        }
    };
    let mut first: HashMap<&str, usize> = HashMap::new();
    for (i, step) in story.steps.iter().enumerate() {
        match first.get(step.key.as_str()) {
            Some(&j) => {
                let path = format!("steps[{i}].key");
                problems.push(at(
                    &rel,
                    bytes,
                    &path,
                    Severity::Error,
                    SCHEMA_CODE,
                    t!("stories.key_taken", key = step.key, step = j).into_owned(),
                ));
            }
            None => {
                first.insert(&step.key, i);
            }
        }
    }
    (Some(story), problems)
}

/// The mismatch of the `id` of story file `bytes` (any JSON object with a string `id`) with the
/// register's id of `n`, at the id's key.
fn id_problem(registry: &Registry, n: u16, rel: &Path, bytes: &[u8]) -> Option<Problem> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let id = value.get("id")?.as_str()?;
    let prefixed = id
        .strip_prefix(SymbolKind::Story.prefix())
        .is_some_and(|rest| rest.len() > 1 && rest.starts_with('_'));
    let message = if !prefixed {
        t!("stories.id_no_prefix", id = id).into_owned()
    } else {
        let own = registry.id_of(u32::from(n));
        if own == Some(id) {
            return None;
        }
        let owner = own.map_or_else(
            || t!("stories.id_owner_none").into_owned(),
            |own| format!("`{own}`"),
        );
        t!("stories.id_mismatch", id = id, n = n, owner = owner).into_owned()
    };
    let (line, col) = key_position(bytes, "id", 0);
    Some(Problem {
        line,
        col,
        ..problem(rel, ID_MISMATCH_CODE, message)
    })
}

/// The warnings of the references in `progress`, `uses` and `flags` of `story` that name
/// nothing: an id that resolves nowhere (the resolver's sentence), a use of what `disk` does
/// not have („Scriptdatei 0101 gibt es nicht.“) or of an entity whose JSON carries a tombstone
/// („Trainer 0001 ist gelöscht.“), each at its JSON path.
fn dangling(
    story: &Story,
    resolver: &Resolver,
    disk: &OnDisk<'_>,
    rel: &Path,
    bytes: &[u8],
) -> Vec<Problem> {
    let mut refs: Vec<(String, Result<UseTarget, String>)> = Vec::new();
    match &story.progress {
        Progress::Var(id) => refs.push((
            "progress.var".to_owned(),
            number(resolver, SymbolKind::Var, id).map(UseTarget::Var),
        )),
        Progress::Flag(id) => refs.push((
            "progress.flag".to_owned(),
            number(resolver, SymbolKind::Flag, id).map(UseTarget::Flag),
        )),
    }
    for (i, step) in story.steps.iter().enumerate() {
        for (j, used) in step.uses.iter().enumerate() {
            let key = match used {
                Use::Trainer(_) => "trainer",
                Use::Map(_) => "map",
                Use::Flag(_) => "flag",
                Use::Var(_) => "var",
                Use::Object { .. } => "object",
                Use::Script { .. } => "script",
                Use::Message { .. } => "message",
                Use::LevelScript { .. } => "levelScript",
            };
            refs.push((
                format!("steps[{i}].uses[{j}].{key}"),
                resolve(used, resolver),
            ));
        }
    }
    for (k, id) in story.flags.iter().enumerate() {
        refs.push((
            format!("flags[{k}]"),
            number(resolver, SymbolKind::Flag, id).map(UseTarget::Flag),
        ));
    }
    refs.into_iter()
        .filter_map(|(path, target)| {
            let why = match target {
                Err(why) => why,
                Ok(target) => disk.open(&target)?,
            };
            Some(at(
                rel,
                bytes,
                &path,
                Severity::Warning,
                DANGLING_REF_CODE,
                why,
            ))
        })
        .collect()
}

/// A problem of `severity` with `code` at JSON `path` of `bytes`: `<code>: <path>: <message>`.
fn at(
    rel: &Path,
    bytes: &[u8],
    path: &str,
    severity: Severity,
    code: &str,
    message: String,
) -> Problem {
    let (line, col) = position(bytes, path).unwrap_or((1, 1));
    Problem {
        severity,
        line,
        col,
        ..problem(rel, code, format!("{path}: {message}"))
    }
}

/// The problems of the folder `data/stories/` and of the register (the data kind `stories` of
/// [`check_files`](crate::textproject::kinds::check_files)): a file name that is no
/// `NNNN.json` or `NNNN.layout.json` (`textproject.schema`), each story file by
/// [`check_file`], a side file that is no [`Layout`] (`textproject.schema`), a register entry
/// beyond 16 bits (`textproject.schema`) or without its file (`textproject.id-dangling`,
/// warning). A register or resolver that does not read gives none here: the import plan
/// reports it.
pub(crate) fn check_folder(
    project: &Project,
    problems: &mut Vec<Problem>,
) -> Result<(), KernelError> {
    let root = &project.root;
    let Ok(registry) = Registry::load(root, SymbolKind::Story) else {
        return Ok(());
    };
    let dir = Path::new(STORIES_DIR);
    let names = file_names(&root.join(dir))?;
    let resolver = if names.is_empty() {
        None
    } else {
        match Resolver::for_project(project) {
            Ok(resolver) => Some(resolver),
            Err(_) => return Ok(()),
        }
    };
    let disk = OnDisk::new(project);
    let mut files: BTreeSet<u16> = BTreeSet::new();
    for name in names {
        let rel = dir.join(&name);
        let Some((n, side)) = file_number(&name) else {
            problems.push(problem(
                &rel,
                SCHEMA_CODE,
                t!("stories.file_not_numbered").into_owned(),
            ));
            continue;
        };
        let path = root.join(&rel);
        let bytes = std::fs::read(&path).map_err(io_at(&path))?;
        if side {
            if let Err(found) = parse_json::<Layout>(&rel, &bytes, SCHEMA_CODE) {
                problems.push(found);
            }
            continue;
        }
        files.insert(n);
        if let Some(resolver) = &resolver {
            problems.extend(check_with(&disk, &registry, resolver, n, &bytes));
        }
    }
    for (id, &n) in &registry.ids {
        match u16::try_from(n) {
            Ok(n16) if files.contains(&n16) => {}
            Ok(n16) => problems.push(unfiled(root, id, n16)),
            Err(_) => {
                let mut found = register_problem(
                    root,
                    id,
                    SCHEMA_CODE,
                    t!("stories.number_too_high", id = id, n = n).into_owned(),
                );
                found.severity = Severity::Error;
                problems.push(found);
            }
        }
    }
    Ok(())
}

/// The warning for register entry `id` of story `n` without its file, at the id's key in the
/// register.
pub(crate) fn unfiled(root: &Path, id: &str, n: u16) -> Problem {
    register_problem(
        root,
        id,
        ID_DANGLING_CODE,
        t!(
            "stories.unfiled",
            id = id,
            n = n,
            file = story_rel(n).to_string_lossy().replace('\\', "/")
        )
        .into_owned(),
    )
}

/// A warning with `code` at the key `id` of the story register.
fn register_problem(root: &Path, id: &str, code: &str, message: String) -> Problem {
    let rel = Path::new(IDS_DIR).join(registry_file(SymbolKind::Story).unwrap_or_default());
    let bytes = std::fs::read(root.join(&rel)).unwrap_or_default();
    let (line, col) = key_position(&bytes, id, 0);
    Problem {
        severity: Severity::Warning,
        line,
        col,
        ..problem(&rel, code, message)
    }
}

/// The number of a file name of the folder and whether it is a side file: `0003.json` →
/// `(3, false)`, `0003.layout.json` → `(3, true)` (decimal, at least four digits, no leading
/// zero beyond them).
fn file_number(name: &str) -> Option<(u16, bool)> {
    let (digits, side) = match name.strip_suffix(".layout.json") {
        Some(digits) => (digits, true),
        None => (name.strip_suffix(".json")?, false),
    };
    let n: u16 = digits.parse().ok()?;
    (digits.bytes().all(|b| b.is_ascii_digit()) && format!("{n:04}") == digits).then_some((n, side))
}

/// Story `n`; `None` without its file. A file that does not read is [`KernelError::Compile`].
pub fn read_story(project: &Project, n: u16) -> Result<Option<Story>, KernelError> {
    read_json(project, &story_rel(n))
}

/// The side file of story `n`; `None` without it. A file that does not read is
/// [`KernelError::Compile`].
pub fn read_layout(project: &Project, n: u16) -> Result<Option<Layout>, KernelError> {
    read_json(project, &layout_rel(n))
}

/// `T` from file `rel` of the project; `None` without the file.
fn read_json<T: DeserializeOwned>(project: &Project, rel: &Path) -> Result<Option<T>, KernelError> {
    let path = project.root.join(rel);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_at(&path)(e)),
    };
    parse_json(rel, &bytes, SCHEMA_CODE)
        .map(Some)
        .map_err(|found| KernelError::Compile(vec![found]))
}

/// The number of `id` from the register; `None` for an unknown id.
pub fn number_of(project: &Project, id: &str) -> Result<Option<u16>, KernelError> {
    let registry = Registry::load(&project.root, SymbolKind::Story)?;
    Ok(registry.ids.get(id).and_then(|&n| u16::try_from(n).ok()))
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::test_support::TextFixture;
    use crate::textproject::canon::to_canonical;
    use crate::textproject::tombstone::with_envelope;

    /// Gutachten A §3.2 (c), grown to seven steps (one of each kind) with every form of `uses`.
    const EXAMPLE: &str = r#"{
  "id": "STORY_TESTFADEN",
  "name": "Testfaden",
  "notes": "Ein Faden zum Testen.",
  "progress": {
    "var": "VAR_STORY_TESTFADEN"
  },
  "steps": [
    {
      "key": "s0",
      "value": 0,
      "kind": "start",
      "name": "Ankunft",
      "uses": [
        {
          "map": "MAP_T01"
        },
        {
          "levelScript": {
            "file": 502,
            "condition": 0
          }
        }
      ]
    },
    {
      "key": "s1",
      "value": 1,
      "kind": "dialog",
      "name": "Gespräch mit dem Wächter",
      "notes": "Der Wächter erzählt vom Tor.",
      "made": {
        "recipe": "story.talk_advances",
        "studio": "0.3.0"
      },
      "uses": [
        {
          "object": {
            "file": 2,
            "local_id": 4
          }
        },
        {
          "script": {
            "file": 2,
            "slot": 7
          }
        },
        {
          "message": {
            "archive": 23,
            "index": 5
          }
        }
      ]
    },
    {
      "key": "s2",
      "value": 2,
      "kind": "condition",
      "name": "Orden geprüft",
      "uses": [
        {
          "flag": "FLAG_STORY_TESTFADEN_WAECHTER"
        }
      ]
    },
    {
      "key": "s3",
      "value": 3,
      "kind": "battle",
      "name": "Herausforderung",
      "uses": [
        {
          "trainer": "TRAINER_0042"
        }
      ]
    },
    {
      "key": "s4",
      "value": 4,
      "kind": "reward",
      "name": "Belohnung",
      "uses": [
        {
          "var": "VAR_UNUSED_0x406C"
        }
      ]
    },
    {
      "key": "s5",
      "value": 5,
      "kind": "progress",
      "name": "Weiter zum Tor"
    },
    {
      "key": "s6",
      "value": 65535,
      "kind": "appearance",
      "name": "Der Wächter verschwindet",
      "uses": [
        {
          "object": {
            "file": 2,
            "local_id": 4
          }
        }
      ]
    }
  ],
  "flags": [
    "FLAG_STORY_TESTFADEN_WAECHTER"
  ]
}
"#;

    fn parse(text: &str) -> Result<Story, Problem> {
        parse_json(&story_rel(0), text.as_bytes(), SCHEMA_CODE)
    }

    /// `(code, severity, line)` of each problem, in order.
    fn found(problems: &[Problem]) -> Vec<(String, Severity, u32)> {
        problems
            .iter()
            .map(|p| {
                (
                    p.message.split(':').next().unwrap().to_owned(),
                    p.severity,
                    p.line,
                )
            })
            .collect()
    }

    #[test]
    fn a_story_round_trips_canonically() {
        let story = parse(EXAMPLE).unwrap();
        assert_eq!(story.steps.len(), 7);
        let kinds: Vec<StepKind> = story.steps.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, StepKind::all());
        assert_eq!(
            story.progress,
            Progress::Var("VAR_STORY_TESTFADEN".to_owned())
        );
        assert_eq!(
            story.steps[0].uses[1],
            Use::LevelScript {
                file: 502,
                condition: 0
            }
        );
        assert_eq!(
            story.steps[1].made,
            Some(Made {
                recipe: "story.talk_advances".to_owned(),
                studio: "0.3.0".to_owned()
            })
        );
        assert_eq!(story.steps[6].value, u16::MAX);
        assert_eq!(String::from_utf8(to_canonical(&story)).unwrap(), EXAMPLE);

        // The smallest story: no notes, no steps, no flags are written.
        let small = Story {
            id: "STORY_A".to_owned(),
            name: "A".to_owned(),
            notes: String::new(),
            progress: Progress::Flag("FLAG_0001".to_owned()),
            steps: Vec::new(),
            flags: Vec::new(),
        };
        let text = String::from_utf8(to_canonical(&small)).unwrap();
        assert_eq!(
            text,
            "{\n  \"id\": \"STORY_A\",\n  \"name\": \"A\",\n  \"progress\": {\n    \"flag\": \
             \"FLAG_0001\"\n  }\n}\n"
        );
        assert_eq!(parse(&text).unwrap(), small);

        // The layout: empty, and with a node.
        assert_eq!(
            String::from_utf8(to_canonical(&Layout::default())).unwrap(),
            "{\n  \"nodes\": {}\n}\n"
        );
        let layout: Layout = parse_json(
            &layout_rel(0),
            br#"{"nodes": {"s0": {"x": 10, "y": -2.5}}}"#,
            SCHEMA_CODE,
        )
        .unwrap();
        assert_eq!(layout.nodes["s0"], Position { x: 10.0, y: -2.5 });
        assert_eq!(story_rel(0), Path::new("data/stories/0000.json"));
        assert_eq!(layout_rel(12), Path::new("data/stories/0012.layout.json"));
        assert_eq!(story_rel(12345), Path::new("data/stories/12345.json"));
    }

    #[test]
    fn unknown_fields_and_duplicate_keys_are_schema_problems() {
        let bad = [
            // An unknown field of the story, of a step, of a use.
            EXAMPLE.replacen("\"notes\": \"Ein", "\"colour\": 1,\n  \"notes\": \"Ein", 1),
            EXAMPLE.replacen("\"value\": 5,", "\"value\": 5,\n      \"x\": 1,", 1),
            EXAMPLE.replacen("\"slot\": 7", "\"slot\": 7,\n            \"line\": 1", 1),
            // A use of an unknown kind, a use with two keys, progress with both.
            EXAMPLE.replacen("\"map\": \"MAP_T01\"", "\"event\": 3", 1),
            EXAMPLE.replacen(
                "\"map\": \"MAP_T01\"",
                "\"map\": \"MAP_T01\", \"trainer\": \"TRAINER_0042\"",
                1,
            ),
            EXAMPLE.replacen(
                "\"var\": \"VAR_STORY_TESTFADEN\"",
                "\"var\": \"VAR_STORY_TESTFADEN\", \"flag\": \"FLAG_0001\"",
                1,
            ),
            // A kind in German, a value above 16 bits, no progress.
            EXAMPLE.replacen("\"kind\": \"battle\"", "\"kind\": \"kampf\"", 1),
            EXAMPLE.replacen("\"value\": 65535", "\"value\": 65536", 1),
            EXAMPLE.replacen(
                "\"progress\": {\n    \"var\": \"VAR_STORY_TESTFADEN\"\n  },\n",
                "",
                1,
            ),
        ];
        for text in &bad {
            assert_ne!(text, EXAMPLE);
            let problem = parse(text).unwrap_err();
            assert!(problem.message.starts_with(SCHEMA_CODE), "{problem:?}");
            assert_eq!(problem.severity, Severity::Error);
        }
        // The unknown field is named at its line.
        let problem = parse(&bad[0]).unwrap_err();
        assert_eq!(problem.line, 4, "{problem:?}");
        assert!(problem.message.contains("colour"), "{problem:?}");

        // A key twice: the schema reads it, the check names the second at its line.
        let fx = TextFixture::exported();
        fx.write("data/ids/stories.json", b"{\n  \"STORY_TESTFADEN\": 0\n}\n");
        let p = fx.project();
        let registry = Registry::load(fx.root(), SymbolKind::Story).unwrap();
        let resolver = Resolver::for_project(&p).unwrap();
        let twice = EXAMPLE.replacen("\"key\": \"s3\"", "\"key\": \"s1\"", 1);
        assert!(parse(&twice).is_ok());
        let problems: Vec<Problem> = check_file(&p, &registry, &resolver, 0, twice.as_bytes())
            .into_iter()
            .filter(|p| p.severity == Severity::Error)
            .collect();
        let line = 1 + twice[..twice.find("\"key\": \"s1\"").unwrap()]
            .matches('\n')
            .count();
        let second = 1 + twice[..twice.rfind("\"key\": \"s1\"").unwrap()]
            .matches('\n')
            .count();
        assert!(second > line);
        assert_eq!(
            found(&problems),
            [(
                SCHEMA_CODE.to_owned(),
                Severity::Error,
                u32::try_from(second).unwrap()
            )],
            "{problems:#?}"
        );
        assert_eq!(
            problems[0].message,
            "textproject.schema: steps[3].key: `s1` already has step 1; each key appears only \
             once in a story."
        );
    }

    #[test]
    fn check_file_reports_schema_and_id_problems() {
        let fx = TextFixture::exported();
        fx.write("data/ids/stories.json", b"{\n  \"STORY_A\": 0\n}\n");
        let p = fx.project();
        let registry = Registry::load(fx.root(), SymbolKind::Story).unwrap();
        let resolver = Resolver::for_project(&p).unwrap();
        let check = |n: u16, text: &str| check_file(&p, &registry, &resolver, n, text.as_bytes());
        let story = |id: &str, progress: &str| {
            format!(
                "{{\n  \"id\": \"{id}\",\n  \"name\": \"A\",\n  \"progress\": {{\n    {progress}\n  }}\n}}\n"
            )
        };

        // A story whose ids all resolve: nothing.
        let good = story("STORY_A", "\"var\": \"VAR_UNUSED_0x406C\"");
        assert!(check(0, &good).is_empty(), "{:#?}", check(0, &good));
        // The register names another id: a mismatch at the id's line, with the schema's
        // problem when the rest does not read either.
        let problems = check(0, "{\n  \"id\": \"STORY_B\"\n}\n");
        let mut codes: Vec<(String, Severity)> = found(&problems)
            .into_iter()
            .map(|(c, s, _)| (c, s))
            .collect();
        codes.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            codes,
            [
                (ID_MISMATCH_CODE.to_owned(), Severity::Error),
                (SCHEMA_CODE.to_owned(), Severity::Error),
            ],
            "{problems:#?}"
        );
        let mismatch = problems
            .iter()
            .find(|p| p.message.starts_with(ID_MISMATCH_CODE))
            .unwrap();
        assert_eq!(
            mismatch.message,
            "textproject.id-mismatch: id: `STORY_B` is not the ID of story 0 (`STORY_A`)."
        );
        assert_eq!(mismatch.file, Path::new("data/stories/0000.json"));
        assert_eq!(mismatch.line, 2);
        // An id without the prefix.
        let problems = check(0, &story("A", "\"var\": \"VAR_UNUSED_0x406C\""));
        assert_eq!(
            found(&problems),
            [(ID_MISMATCH_CODE.to_owned(), Severity::Error, 2)]
        );
        assert_eq!(
            problems[0].message,
            "textproject.id-mismatch: id: `A` does not start with `STORY_`."
        );
        // A number the register does not name.
        let problems = check(1, &story("STORY_A", "\"var\": \"VAR_UNUSED_0x406C\""));
        assert_eq!(
            problems[0].message,
            "textproject.id-mismatch: id: `STORY_A` is not the ID of story 1 (the register \
             names none)."
        );
        // No JSON.
        let problems = check(0, "nicht json");
        assert_eq!(
            found(&problems),
            [(SCHEMA_CODE.to_owned(), Severity::Error, 1)]
        );

        // A progress, a use and a flag naming nothing: warnings at their lines.
        let problems = check(0, &story("STORY_A", "\"var\": \"VAR_NOPE\""));
        assert_eq!(
            found(&problems),
            [(DANGLING_REF_CODE.to_owned(), Severity::Warning, 5)]
        );
        assert_eq!(
            problems[0].message,
            "story.dangling-ref: progress.var: Unknown ID: VAR_NOPE."
        );
        let text = "{\n  \"id\": \"STORY_A\",\n  \"name\": \"A\",\n  \"progress\": {\n    \"flag\": \
                    \"FLAG_0001\"\n  },\n  \"steps\": [\n    {\n      \"key\": \"s0\",\n      \
                    \"value\": 0,\n      \"kind\": \"battle\",\n      \"name\": \"Kampf\",\n      \
                    \"uses\": [\n        {\n          \"trainer\": \"TRAINER_NIEMAND\"\n        },\n        \
                    {\n          \"script\": {\n            \"file\": 1,\n            \"slot\": 1\n          \
                    }\n        }\n      ]\n    }\n  ],\n  \"flags\": [\n    \"FLAG_NIRGENDS\"\n  ]\n}\n";
        assert!(parse(text).is_ok());
        let problems = check(0, text);
        assert_eq!(
            found(&problems),
            [
                (DANGLING_REF_CODE.to_owned(), Severity::Warning, 15),
                (DANGLING_REF_CODE.to_owned(), Severity::Warning, 27),
            ],
            "{problems:#?}"
        );
        assert_eq!(
            problems[0].message,
            "story.dangling-ref: steps[0].uses[0].trainer: Unknown ID: TRAINER_NIEMAND."
        );
        assert_eq!(
            problems[1].message,
            "story.dangling-ref: flags[0]: Unknown ID: FLAG_NIRGENDS."
        );
    }

    #[test]
    fn a_use_of_a_deleted_entity_is_dangling() {
        let fx = TextFixture::exported();
        fx.write("data/ids/stories.json", b"{\n  \"STORY_A\": 0\n}\n");
        let p = fx.project();
        let registry = Registry::load(fx.root(), SymbolKind::Story).unwrap();
        let resolver = Resolver::for_project(&p).unwrap();
        let text = "{\n  \"id\": \"STORY_A\",\n  \"name\": \"A\",\n  \"progress\": {\n    \"flag\": \
                    \"FLAG_0001\"\n  },\n  \"steps\": [\n    {\n      \"key\": \"s0\",\n      \
                    \"value\": 0,\n      \"kind\": \"battle\",\n      \"name\": \"Kampf\",\n      \
                    \"uses\": [\n        {\n          \"trainer\": \"TRAINER_YOUNGSTER_KAI\"\n        },\n        \
                    {\n          \"object\": {\n            \"file\": 0,\n            \"local_id\": 1\n          \
                    }\n        },\n        {\n          \"map\": \"MAP_T01\"\n        }\n      ]\n    }\n  ]\n}\n";
        let check = || check_file(&p, &registry, &resolver, 0, text.as_bytes());
        // All live: nothing.
        assert!(check().is_empty(), "{:#?}", check());

        // Trainer 1, event file 0 and header 0 (`MAP_T01`) carry a tombstone: each use is open,
        // at its line.
        for rel in [
            "data/trainers/0001.json",
            "data/events/0000.json",
            "data/headers/0000.json",
        ] {
            fx.write(rel, &with_envelope(fx.read(rel)));
        }
        let problems = check();
        assert_eq!(
            found(&problems),
            [
                (DANGLING_REF_CODE.to_owned(), Severity::Warning, 15),
                (DANGLING_REF_CODE.to_owned(), Severity::Warning, 18),
                (DANGLING_REF_CODE.to_owned(), Severity::Warning, 24),
            ],
            "{problems:#?}"
        );
        assert_eq!(
            problems[0].message,
            "story.dangling-ref: steps[0].uses[0].trainer: Trainer 0001 is deleted."
        );
        assert_eq!(
            problems[1].message,
            "story.dangling-ref: steps[0].uses[1].object: Event file 0000 is deleted."
        );
        assert_eq!(
            problems[2].message,
            "story.dangling-ref: steps[0].uses[2].map: Header 0000 is deleted."
        );
    }

    /// The fixture has script files 0 and 1 as sources and 0 to 2 as members, text archives 0
    /// to 697, event files 0 and 1, trainers 0 to 2 (2 is `TRAINER_POKEMON_TRAINER_MIA`) and
    /// headers 0 and 1 (`MAP_T01`, `MAP_T02`).
    #[test]
    fn a_use_of_what_is_gone_is_dangling() {
        let fx = TextFixture::exported();
        fx.write("data/ids/stories.json", b"{\n  \"STORY_A\": 0\n}\n");
        let p = fx.project();
        let registry = Registry::load(fx.root(), SymbolKind::Story).unwrap();
        let resolver = Resolver::for_project(&p).unwrap();
        let text = r#"{
  "id": "STORY_A",
  "name": "A",
  "progress": { "flag": "FLAG_0001" },
  "steps": [
    {
      "key": "s0", "value": 0, "kind": "battle", "name": "Kampf",
      "uses": [
        { "script": { "file": 1, "slot": 0 } },
        { "levelScript": { "file": 2, "condition": 0 } },
        { "script": { "file": 3, "slot": 0 } },
        { "message": { "archive": 697, "index": 0 } },
        { "message": { "archive": 698, "index": 0 } },
        { "object": { "file": 1, "local_id": 0 } },
        { "object": { "file": 2, "local_id": 0 } },
        { "trainer": "TRAINER_POKEMON_TRAINER_MIA" },
        { "map": "MAP_T02" }
      ]
    }
  ]
}
"#;
        assert!(parse(text).is_ok());
        let check = || check_file(&p, &registry, &resolver, 0, text.as_bytes());
        let warning = |line: u32| (DANGLING_REF_CODE.to_owned(), Severity::Warning, line);
        let messages = |problems: &[Problem]| -> Vec<String> {
            problems.iter().map(|p| p.message.clone()).collect()
        };

        // Past the count of its art: a script, a text archive and an event file. What exists
        // (script 1, member 2 as a level script, archive 697, event file 1, trainer 2, header 1)
        // is no problem.
        let problems = check();
        assert_eq!(
            found(&problems),
            [warning(11), warning(13), warning(15)],
            "{problems:#?}"
        );
        assert_eq!(
            messages(&problems),
            [
                "story.dangling-ref: steps[0].uses[2].script: Script file 0003 does not exist.",
                "story.dangling-ref: steps[0].uses[4].message: Text archive 0698 does not exist.",
                "story.dangling-ref: steps[0].uses[6].object: Event file 0002 does not exist.",
            ]
        );

        // „Nummer freigeben“ removed trainer 2 and header 1: their ids still resolve (the
        // register keeps them), the files are gone.
        for rel in ["data/trainers/0002.json", "data/headers/0001.json"] {
            std::fs::remove_file(fx.root().join(rel)).unwrap();
        }
        let problems = check();
        assert_eq!(
            found(&problems),
            [
                warning(11),
                warning(13),
                warning(15),
                warning(16),
                warning(17),
            ],
            "{problems:#?}"
        );
        assert_eq!(
            messages(&problems)[3..],
            [
                "story.dangling-ref: steps[0].uses[7].trainer: Trainer 0002 does not exist.",
                "story.dangling-ref: steps[0].uses[8].map: Header 0001 does not exist.",
            ]
        );
    }

    /// A text project without the sources and members of an art (no `unpacked/`, no script
    /// sources, no text archives, no event files) does not know how many there are: its uses
    /// of scripts, messages and objects are no problem, never „… gibt es nicht.“ for each.
    #[test]
    fn an_art_without_sources_or_members_counts_nothing() {
        use crate::project::AssetKind;
        let fx = TextFixture::exported();
        fx.write("data/ids/stories.json", b"{\n  \"STORY_A\": 0\n}\n");
        let p = fx.project();
        for dir in [
            p.expanded_dir(AssetKind::Script),
            p.unpacked_dir(AssetKind::Script),
            p.expanded_dir(AssetKind::TextArchive),
            p.unpacked_dir(AssetKind::TextArchive),
            fx.root().join(crate::textproject::Art::Events.dir()),
        ] {
            if dir.exists() {
                std::fs::remove_dir_all(&dir).unwrap();
            }
        }
        let registry = Registry::load(fx.root(), SymbolKind::Story).unwrap();
        let resolver = Resolver::for_project(&p).unwrap();
        let text = r#"{
  "id": "STORY_A",
  "name": "A",
  "progress": { "flag": "FLAG_0001" },
  "steps": [
    {
      "key": "s0", "value": 0, "kind": "battle", "name": "Kampf",
      "uses": [
        { "script": { "file": 1, "slot": 0 } },
        { "levelScript": { "file": 2, "condition": 0 } },
        { "message": { "archive": 697, "index": 0 } },
        { "object": { "file": 1, "local_id": 0 } }
      ]
    }
  ]
}
"#;
        let problems = check_file(&p, &registry, &resolver, 0, text.as_bytes());
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn kinds_have_seven_words() {
        assert_eq!(StepKind::all().len(), 7);
        let words: Vec<String> = StepKind::all().iter().map(|k| k.word()).collect();
        assert_eq!(
            words,
            [
                "Start",
                "Dialog",
                "Condition",
                "Battle",
                "Reward",
                "Progress",
                "Appear/disappear"
            ]
        );
        let names: Vec<serde_json::Value> = StepKind::all()
            .iter()
            .map(|k| serde_json::to_value(k).unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "start",
                "dialog",
                "condition",
                "battle",
                "reward",
                "progress",
                "appearance"
            ]
        );
    }

    #[test]
    fn read_story_layout_and_number() {
        let fx = TextFixture::exported();
        let p = fx.project();
        assert_eq!(read_story(&p, 0).unwrap(), None);
        assert_eq!(read_layout(&p, 0).unwrap(), None);
        assert_eq!(number_of(&p, "STORY_TESTFADEN").unwrap(), None);
        fx.write("data/ids/stories.json", b"{\n  \"STORY_TESTFADEN\": 3\n}\n");
        fx.write("data/stories/0003.json", EXAMPLE.as_bytes());
        fx.write("data/stories/0003.layout.json", b"{\"nodes\": {}}");
        assert_eq!(number_of(&p, "STORY_TESTFADEN").unwrap(), Some(3));
        assert_eq!(read_story(&p, 3).unwrap(), Some(parse(EXAMPLE).unwrap()));
        assert_eq!(read_layout(&p, 3).unwrap(), Some(Layout::default()));
        fx.write("data/stories/0003.json", b"{}");
        assert!(matches!(read_story(&p, 3), Err(KernelError::Compile(_))));
    }
}
