//! The manifest `g4-project.json`: format, schema version, game and Studio version, plus the
//! reserved `schemas` and `plugins`, kept as they are and not evaluated in phase 4.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

use g4_formats::game::Family;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::canon::{json_problem, parse_json, problem};
use super::{MANIFEST_FILE, SCHEMA_VERSION};
use crate::error::{KernelError, io_at};
use crate::project::Project;
use crate::text::Problem;

/// The value of `format` in every manifest.
pub const FORMAT: &str = "g4-project";
/// The code of a file that does not match its schema (no JSON, unknown key, wrong type).
pub(crate) use super::codes::SCHEMA_CODE;

/// `g4-project.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub format: String,
    pub schema_version: u32,
    pub game: GameInfo,
    pub studio_version: String,
    #[serde(default)]
    pub schemas: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub plugins: Vec<serde_json::Value>,
}

/// The game of the project: its four-letter code, `pt` or `hgss`, and the locale key of its
/// language (`de_DE`, `en_US`, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameInfo {
    pub code: String,
    pub family: String,
    pub language: String,
}

/// The schema version of a folder against [`SCHEMA_VERSION`](super::SCHEMA_VERSION).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase", tag = "state", content = "version")]
pub enum SchemaState {
    /// No manifest.
    Missing,
    Current,
    /// Written by an older Studio: `g4k text migrate` lifts it.
    Older(u32),
    /// Written by a newer Studio: refused everywhere.
    Newer(u32),
}

/// Reads the schema version of the manifest at `root`: `schemaVersion`, or `schema` in a v0
/// manifest. A manifest that is no JSON or has no version is a [`KernelError::Compile`]
/// naming file, line, column and key.
pub fn schema_state(root: &Path) -> Result<SchemaState, KernelError> {
    match read_bytes(root)? {
        None => Ok(SchemaState::Missing),
        Some(bytes) => state_of(&bytes),
    }
}

/// The manifest at `root` if it has the current schema version. Otherwise
/// [`KernelError::Refused`] with the sentence for a missing, older or newer manifest; a
/// manifest that does not match the schema (an unknown or misspelt key, a wrong type, another
/// `format`) is a [`KernelError::Compile`].
pub fn read_manifest(root: &Path) -> Result<Manifest, KernelError> {
    let Some(bytes) = read_bytes(root)? else {
        return Err(KernelError::Refused(
            refusal(SchemaState::Missing).unwrap_or_default(),
        ));
    };
    let state = state_of(&bytes)?;
    if let Some(text) = refusal(state) {
        return Err(KernelError::Refused(text));
    }
    let manifest: Manifest = parse_json(manifest_rel(), &bytes, SCHEMA_CODE)
        .map_err(|problem| KernelError::Compile(vec![problem]))?;
    if manifest.format != FORMAT {
        return Err(KernelError::Compile(vec![problem(
            manifest_rel(),
            SCHEMA_CODE,
            format!("`format` ist „{}“, erwartet „{FORMAT}“.", manifest.format),
        )]));
    }
    Ok(manifest)
}

/// The manifest for `project` in this Studio version; `schemas` and `plugins` come unchanged
/// from `keep` (the manifest on disk), empty without one.
pub fn manifest_for(project: &Project, keep: Option<&Manifest>) -> Manifest {
    Manifest {
        format: FORMAT.to_owned(),
        schema_version: SCHEMA_VERSION,
        game: GameInfo {
            code: project.game_code.clone(),
            family: family_key(project.family).to_owned(),
            language: project.language.locale_key().to_owned(),
        },
        studio_version: env!("CARGO_PKG_VERSION").to_owned(),
        schemas: keep.map(|m| m.schemas.clone()).unwrap_or_default(),
        plugins: keep.map(|m| m.plugins.clone()).unwrap_or_default(),
    }
}

/// The sentence a command refuses with when the schema state is not [`SchemaState::Current`];
/// `None` when it is.
pub(crate) fn refusal(state: SchemaState) -> Option<String> {
    match state {
        SchemaState::Current => None,
        SchemaState::Missing => Some(format!("Kein Textprojekt: {MANIFEST_FILE} fehlt.")),
        SchemaState::Older(v) => Some(format!(
            "Das Textprojekt hat Schema-Version {v}, diese Fassung kennt {SCHEMA_VERSION}: \
             `g4k text migrate` ausführen."
        )),
        SchemaState::Newer(v) => Some(format!(
            "Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio \
             (Schema-Version {v})."
        )),
    }
}

/// The schema state of a manifest's bytes.
pub(crate) fn state_of(bytes: &[u8]) -> Result<SchemaState, KernelError> {
    let version = version_of(bytes).map_err(|p| KernelError::Compile(vec![p]))?;
    Ok(match version.cmp(&SCHEMA_VERSION) {
        Ordering::Equal => SchemaState::Current,
        Ordering::Less => SchemaState::Older(version),
        Ordering::Greater => SchemaState::Newer(version),
    })
}

/// `schemaVersion`, or `schema` (v0), of a manifest's bytes.
fn version_of(bytes: &[u8]) -> Result<u32, Problem> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|e| json_problem(manifest_rel(), bytes, &e, SCHEMA_CODE))?;
    let found = ["schemaVersion", "schema"]
        .into_iter()
        .find_map(|key| value.get(key).map(|v| (key, v)));
    let Some((key, version)) = found else {
        // serde names the misspelt or missing key with its position.
        return Err(
            match parse_json::<Manifest>(manifest_rel(), bytes, SCHEMA_CODE) {
                Err(problem) => problem,
                Ok(_) => problem(
                    manifest_rel(),
                    SCHEMA_CODE,
                    "Der Schlüssel `schemaVersion` fehlt.".to_owned(),
                ),
            },
        );
    };
    version
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| {
            problem(
                manifest_rel(),
                SCHEMA_CODE,
                format!("`{key}` ist keine Schema-Version (ganze Zahl ab 0): {version}."),
            )
        })
}

/// The manifest's bytes, `None` when there is none.
fn read_bytes(root: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    let path = root.join(MANIFEST_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(&path)(e)),
    }
}

fn manifest_rel() -> &'static Path {
    Path::new(MANIFEST_FILE)
}

/// `pt`, `hgss` (and `dp`, which the export refuses before it writes a manifest).
fn family_key(family: Family) -> &'static str {
    match family {
        Family::Dp => "dp",
        Family::Pt => "pt",
        Family::Hgss => "hgss",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: &str = r#"{
  "format": "g4-project",
  "schemaVersion": 1,
  "game": {
    "code": "CPUE",
    "family": "pt",
    "language": "en_US"
  },
  "studioVersion": "0.1.0",
  "schemas": {},
  "plugins": []
}
"#;

    #[test]
    fn manifest_round_trips_and_refuses_unknown_keys() {
        let manifest: Manifest = serde_json::from_str(V1).unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.game.family, "pt");
        assert_eq!(
            crate::textproject::canon::to_canonical(&manifest),
            V1.as_bytes()
        );

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(MANIFEST_FILE), V1).unwrap();
        assert_eq!(read_manifest(dir.path()).unwrap(), manifest);

        // A typo in a key: the schema state cannot be read, the error names the key.
        let typo = V1.replace("\"schemaVersion\"", "\"schemaVersio\"");
        std::fs::write(dir.path().join(MANIFEST_FILE), &typo).unwrap();
        let problems = match read_manifest(dir.path()).unwrap_err() {
            KernelError::Compile(problems) => problems,
            other => panic!("expected Compile, got {other:?}"),
        };
        assert_eq!(problems.len(), 1);
        assert!(problems[0].message.contains("schemaVersio"), "{problems:?}");
        assert_eq!(problems[0].file, Path::new(MANIFEST_FILE));

        // An unknown key next to a valid version: serde names it with line and column.
        let extra = V1.replace("\"plugins\"", "\"plugin\"");
        std::fs::write(dir.path().join(MANIFEST_FILE), &extra).unwrap();
        let problems = match read_manifest(dir.path()).unwrap_err() {
            KernelError::Compile(problems) => problems,
            other => panic!("expected Compile, got {other:?}"),
        };
        assert_eq!(problems.len(), 1);
        let p = &problems[0];
        assert!(
            p.message
                .starts_with("textproject.schema: plugin: unbekannter Schlüssel"),
            "{}",
            p.message
        );
        assert_eq!(p.line, 11);
    }

    #[test]
    fn read_manifest_refuses_other_versions_with_the_sentences() {
        let dir = tempfile::tempdir().unwrap();
        let refused = |root: &Path| match read_manifest(root).unwrap_err() {
            KernelError::Refused(text) => text,
            other => panic!("expected Refused, got {other:?}"),
        };
        assert_eq!(
            refused(dir.path()),
            "Kein Textprojekt: g4-project.json fehlt."
        );
        let older = V1.replace("\"schemaVersion\": 1", "\"schemaVersion\": 0");
        std::fs::write(dir.path().join(MANIFEST_FILE), older).unwrap();
        assert_eq!(
            refused(dir.path()),
            format!(
                "Das Textprojekt hat Schema-Version 0, diese Fassung kennt {SCHEMA_VERSION}: \
                 `g4k text migrate` ausführen."
            )
        );
        let newer = V1.replace("\"schemaVersion\": 1", "\"schemaVersion\": 3");
        std::fs::write(dir.path().join(MANIFEST_FILE), newer).unwrap();
        assert_eq!(
            refused(dir.path()),
            "Das Textprojekt stammt aus einer neueren Fassung von Gen4 Studio (Schema-Version 3)."
        );
    }

    #[test]
    fn schema_state_reports_a_manifest_without_version_as_a_problem() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            "{\"format\": \"g4-project\"}",
        )
        .unwrap();
        match schema_state(dir.path()).unwrap_err() {
            KernelError::Compile(problems) => {
                assert!(
                    problems[0].message.contains("schemaVersion"),
                    "{problems:?}"
                );
            }
            other => panic!("expected Compile, got {other:?}"),
        }
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn manifest_keeps_reserved_schemas_and_plugins() {
        let s = crate::test_support::synthetic_project();
        let project = Project::open(s.dir.path()).unwrap();

        let fresh = manifest_for(&project, None);
        assert_eq!(fresh.format, "g4-project");
        assert_eq!(fresh.schema_version, SCHEMA_VERSION);
        assert_eq!(
            fresh.game,
            GameInfo {
                code: "CPUE".into(),
                family: "pt".into(),
                language: "en_US".into(),
            }
        );
        assert_eq!(fresh.studio_version, env!("CARGO_PKG_VERSION"));
        assert!(fresh.schemas.is_empty());
        assert!(fresh.plugins.is_empty());

        let mut old = fresh.clone();
        old.studio_version = "0.0.1".into();
        old.schemas.insert(
            "maps".into(),
            serde_json::json!({"version": 2, "extra": [1, 2]}),
        );
        old.plugins
            .push(serde_json::json!({"name": "x", "enabled": true}));
        let kept = manifest_for(&project, Some(&old));
        assert_eq!(kept.schemas, old.schemas);
        assert_eq!(kept.plugins, old.plugins);
        assert_eq!(kept.studio_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(kept.game, fresh.game);
    }
}
