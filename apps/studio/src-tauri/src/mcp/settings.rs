//! The agent settings of the app: whether local agents may connect, the expert tools and
//! the grant of every project, in `<app data>/agents.json` (never in a project folder).
//!
//! A grant belongs to a project by its canonical path and its base id (`.g4/base.json`):
//! another project copied to the same place does not inherit it.

use std::io::Write;
use std::path::Path;

use g4_kernel::start::read_base_mark;
use serde::{Deserialize, Serialize};

/// The settings file in the app data folder.
pub const SETTINGS_FILE: &str = "agents.json";

/// Version of the settings file.
pub const SETTINGS_VERSION: u32 = 1;

/// How far agents may go in one project: not at all, read, or read and edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantLevel {
    None,
    Read,
    Write,
}

/// The grant of one project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// The canonical project folder.
    pub path: String,
    /// `<sha1>/<created>` of the project's base mark; `None` for a project without one.
    pub base_id: Option<String>,
    pub level: GrantLevel,
}

/// The content of `agents.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentSettings {
    pub version: u32,
    /// "Lokale Agenten zulassen".
    pub enabled: bool,
    /// "Expertenwerkzeuge (g4_call)".
    pub expert: bool,
    pub grants: Vec<Grant>,
}

impl Default for AgentSettings {
    /// Agents off, expert tools off, no grants.
    fn default() -> AgentSettings {
        AgentSettings {
            version: SETTINGS_VERSION,
            enabled: false,
            expert: false,
            grants: Vec::new(),
        }
    }
}

/// Which project a grant is for: its canonical path and base id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectKey {
    pub path: String,
    pub base_id: Option<String>,
}

impl ProjectKey {
    /// The key of the project folder `root`: symlinks resolved, base id from
    /// `.g4/base.json` (none for a folder without it, e.g. a DSPRE folder).
    pub fn of(root: &Path) -> Result<ProjectKey, String> {
        let canonical = std::fs::canonicalize(root)
            .map_err(|e| format!("Projektordner nicht lesbar: {}: {e}", root.display()))?;
        let path = canonical
            .to_str()
            .ok_or_else(|| format!("Pfad ist kein gültiger Text: {}", canonical.display()))?
            .to_owned();
        let base_id =
            read_base_mark(&canonical).map(|mark| format!("{}/{}", mark.sha1, mark.created));
        Ok(ProjectKey { path, base_id })
    }
}

impl AgentSettings {
    /// The settings in `dir`; a missing or unreadable file gives the defaults.
    pub fn load(dir: &Path) -> AgentSettings {
        std::fs::read_to_string(dir.join(SETTINGS_FILE))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes the settings to `dir` (created if missing): readable only by the user (0600),
    /// atomically over a temporary file in the same folder.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Ordner nicht anlegbar: {}: {e}", dir.display()))?;
        let mut text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        text.push('\n');
        write_private(&dir.join(SETTINGS_FILE), text.as_bytes())
    }

    /// The grant of `key`: same canonical path and same base id.
    pub fn grant_for(&self, key: &ProjectKey) -> Option<GrantLevel> {
        self.grants
            .iter()
            .find(|g| g.path == key.path && g.base_id == key.base_id)
            .map(|g| g.level)
    }

    /// Sets the grant of `key`, replacing any grant for the same path.
    pub fn set_grant(&mut self, key: ProjectKey, level: GrantLevel) {
        let grant = Grant {
            path: key.path,
            base_id: key.base_id,
            level,
        };
        match self.grants.iter_mut().find(|g| g.path == grant.path) {
            Some(old) => *old = grant,
            None => self.grants.push(grant),
        }
    }

    /// Removes every grant for `path`.
    pub fn remove_grant(&mut self, path: &str) {
        self.grants.retain(|g| g.path != path);
    }

    /// Moves the grant of `from` (same path and base id) to `to`, after a base ROM was
    /// assigned and the project's base id changed. Every other grant stays. Whether there
    /// was a grant to move.
    pub fn move_grant(&mut self, from: &ProjectKey, to: &ProjectKey) -> bool {
        let Some(index) = self
            .grants
            .iter()
            .position(|g| g.path == from.path && g.base_id == from.base_id)
        else {
            return false;
        };
        let grant = &mut self.grants[index];
        grant.path.clone_from(&to.path);
        grant.base_id.clone_from(&to.base_id);
        // A grant that already had the new key gives way to the moved one.
        let mut position = 0;
        self.grants.retain(|g| {
            let keep = position == index || g.path != to.path || g.base_id != to.base_id;
            position += 1;
            keep
        });
        true
    }
}

/// Writes `bytes` to `path` with mode 0600 (on unix): a temporary file in the same folder,
/// then a rename over the target. The temporary file goes again when anything fails.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("Kein Dateiname: {}", path.display()))?;
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".{}.tmp", std::process::id()));
    let temp = path.with_file_name(temp_name);
    let written = write_new_private(&temp, bytes)
        .and_then(|()| std::fs::rename(&temp, path))
        .map_err(|e| format!("Datei nicht schreibbar: {}: {e}", path.display()));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

/// Creates (or truncates) `path` with mode 0600 and writes `bytes`, synced to disk.
fn write_new_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    // A file left over from an earlier run keeps its old mode; set it again.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn key(path: &str, base_id: Option<&str>) -> ProjectKey {
        ProjectKey {
            path: path.to_owned(),
            base_id: base_id.map(str::to_owned),
        }
    }

    #[test]
    fn missing_file_is_default_off() {
        let dir = tempfile::tempdir().unwrap();
        let settings = AgentSettings::load(dir.path());
        assert!(!settings.enabled);
        assert!(!settings.expert);
        assert!(settings.grants.is_empty());
        assert_eq!(settings.version, SETTINGS_VERSION);
        // A broken file reads as the defaults too.
        std::fs::write(dir.path().join(SETTINGS_FILE), "{ kaputt").unwrap();
        assert_eq!(AgentSettings::load(dir.path()), AgentSettings::default());
    }

    #[test]
    fn save_writes_0600_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("app-data");
        let mut settings = AgentSettings {
            enabled: true,
            ..AgentSettings::default()
        };
        settings.set_grant(
            key("/projekt", Some("abc/2026-10-25T14:50:00Z")),
            GrantLevel::Write,
        );
        settings.set_grant(key("/dspre", None), GrantLevel::Read);
        settings.save(&app).unwrap();
        let file = app.join(SETTINGS_FILE);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(AgentSettings::load(&app), settings);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "version": 1,
                "enabled": true,
                "expert": false,
                "grants": [
                    {"path": "/projekt", "base_id": "abc/2026-10-25T14:50:00Z", "level": "write"},
                    {"path": "/dspre", "base_id": null, "level": "read"},
                ],
            })
        );
        // No temporary file is left behind.
        let names: Vec<_> = std::fs::read_dir(&app)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, [SETTINGS_FILE]);
    }

    #[test]
    fn grant_needs_same_path_and_base_id() {
        let mut settings = AgentSettings::default();
        settings.set_grant(key("/p", Some("a/1")), GrantLevel::Write);
        assert_eq!(
            settings.grant_for(&key("/p", Some("a/1"))),
            Some(GrantLevel::Write)
        );
        assert_eq!(settings.grant_for(&key("/p", Some("b/2"))), None);
        assert_eq!(settings.grant_for(&key("/p", None)), None);
        assert_eq!(settings.grant_for(&key("/q", Some("a/1"))), None);
        // A new grant for the same path replaces the old one.
        settings.set_grant(key("/p", Some("b/2")), GrantLevel::Read);
        assert_eq!(settings.grants.len(), 1);
        assert_eq!(
            settings.grant_for(&key("/p", Some("b/2"))),
            Some(GrantLevel::Read)
        );
        settings.remove_grant("/p");
        assert!(settings.grants.is_empty());
    }

    #[test]
    fn move_grant_rewrites_the_grant_of_the_old_key() {
        let mut settings = AgentSettings::default();
        settings.set_grant(key("/dspre", None), GrantLevel::Write);
        assert!(settings.move_grant(&key("/dspre", None), &key("/dspre", Some("abc/1"))));
        assert_eq!(settings.grants.len(), 1);
        assert_eq!(settings.grant_for(&key("/dspre", None)), None);
        assert_eq!(
            settings.grant_for(&key("/dspre", Some("abc/1"))),
            Some(GrantLevel::Write)
        );
        // After a change of the ROM the grant moves on to the next base id.
        assert!(settings.move_grant(&key("/dspre", Some("abc/1")), &key("/dspre", Some("def/2"))));
        assert_eq!(
            settings.grant_for(&key("/dspre", Some("def/2"))),
            Some(GrantLevel::Write)
        );
        assert_eq!(settings.grants.len(), 1);
    }

    #[test]
    fn move_grant_leaves_other_projects_and_other_base_ids_alone() {
        let mut settings = AgentSettings {
            grants: vec![
                Grant {
                    path: "/dspre".into(),
                    base_id: None,
                    level: GrantLevel::Read,
                },
                Grant {
                    path: "/zweites".into(),
                    base_id: None,
                    level: GrantLevel::Write,
                },
                Grant {
                    path: "/drittes".into(),
                    base_id: Some("abc/1".into()),
                    level: GrantLevel::Read,
                },
                // Another base id of the same folder (written by an older app or by hand).
                Grant {
                    path: "/dspre".into(),
                    base_id: Some("alt/0".into()),
                    level: GrantLevel::Write,
                },
            ],
            ..AgentSettings::default()
        };
        let before = settings.grants.clone();
        assert!(settings.move_grant(&key("/dspre", None), &key("/dspre", Some("abc/1"))));
        assert_eq!(
            settings.grants,
            [
                Grant {
                    path: "/dspre".into(),
                    base_id: Some("abc/1".into()),
                    level: GrantLevel::Read,
                },
                before[1].clone(),
                before[2].clone(),
                before[3].clone(),
            ]
        );
    }

    #[test]
    fn move_grant_without_a_grant_changes_nothing() {
        let mut settings = AgentSettings::default();
        settings.set_grant(key("/p", Some("a/1")), GrantLevel::Write);
        settings.set_grant(key("/q", None), GrantLevel::Read);
        let before = settings.clone();
        // Another base id, another path, nothing at all.
        assert!(!settings.move_grant(&key("/p", None), &key("/p", Some("b/2"))));
        assert!(!settings.move_grant(&key("/r", None), &key("/r", Some("b/2"))));
        assert_eq!(settings, before);
        assert!(!AgentSettings::default().move_grant(&key("/p", None), &key("/p", Some("b/2"))));
    }

    #[test]
    fn key_resolves_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("projekt");
        std::fs::create_dir_all(project.join(".g4")).unwrap();
        let canonical = std::fs::canonicalize(&project).unwrap();
        let plain = ProjectKey::of(&project).unwrap();
        assert_eq!(plain.path, canonical.to_str().unwrap());
        assert_eq!(plain.base_id, None);
        #[cfg(unix)]
        {
            let link = dir.path().join("verweis");
            std::os::unix::fs::symlink(&project, &link).unwrap();
            assert_eq!(ProjectKey::of(&link).unwrap(), plain);
        }
        // With a base mark the key carries `<sha1>/<created>`.
        let mark = g4_kernel::start::BaseMark {
            version: g4_kernel::start::BASE_MARK_VERSION,
            origin: g4_kernel::start::ORIGIN_EXTRACTED.into(),
            sha1: "abc".into(),
            game_code: "CPUE".into(),
            title: "POKEMON PL".into(),
            revision: 0,
            size: 1,
            template: Some(g4_kernel::start::Template::Original),
            created: "2026-10-25T14:50:00Z".into(),
            assigned: None,
            studio_version: "0.1.0".into(),
            secure_area_crc: 0,
            secure_area_sha1: String::new(),
        };
        std::fs::write(
            project.join(".g4/base.json"),
            serde_json::to_string(&mark).unwrap(),
        )
        .unwrap();
        assert_eq!(
            ProjectKey::of(&project).unwrap().base_id.as_deref(),
            Some("abc/2026-10-25T14:50:00Z")
        );
        assert!(ProjectKey::of(&dir.path().join("fehlt")).is_err());
    }
}
