//! The discovery file `<app data>/mcp.json`: where the running app serves MCP and with which
//! token. The app writes it when "Lokale Agenten zulassen" switches the server on and removes
//! it when the server stops; `studio mcp` without `--project` reads it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mcp::settings::write_private;

/// Name of the discovery file in the app data folder.
pub const DISCOVERY_FILE: &str = "mcp.json";
/// Version of the file's layout.
pub const DISCOVERY_VERSION: u32 = 1;
/// Environment variable naming the app data folder; it wins over the platform's folder.
pub const APP_DATA_ENV: &str = "G4_APP_DATA_DIR";
/// The app's identifier (`tauri.conf.json`), the name of its data folder.
pub const APP_ID: &str = "dev.gen4studio.app";

/// Contents of `mcp.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discovery {
    pub version: u32,
    pub url: String,
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

impl Discovery {
    /// The file for a server of this process on `127.0.0.1:<port>`.
    pub fn new(port: u16, token: &str) -> Discovery {
        Discovery {
            version: DISCOVERY_VERSION,
            url: format!("http://127.0.0.1:{port}/mcp"),
            port,
            token: token.to_owned(),
            pid: std::process::id(),
        }
    }
}

/// Writes `mcp.json` into `dir` (created when missing), mode 0600, atomically.
pub fn write(dir: &Path, discovery: &Discovery) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| {
        t!(
            "mcp.discovery.folder_not_created",
            path = dir.display(),
            error = e
        )
        .into_owned()
    })?;
    let mut text = serde_json::to_string_pretty(discovery).map_err(|e| e.to_string())?;
    text.push('\n');
    write_private(&dir.join(DISCOVERY_FILE), text.as_bytes())
}

/// Reads `mcp.json` from `dir`.
pub fn read(dir: &Path) -> Result<Discovery, String> {
    let path = dir.join(DISCOVERY_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        t!(
            "mcp.discovery.file_unreadable",
            path = path.display(),
            error = e
        )
        .into_owned()
    })?;
    serde_json::from_str(&text).map_err(|e| {
        t!(
            "mcp.discovery.file_invalid",
            path = path.display(),
            error = e
        )
        .into_owned()
    })
}

/// Removes `mcp.json` from `dir` (a missing file is fine).
pub fn remove(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(DISCOVERY_FILE));
}

/// The folder `G4_APP_DATA_DIR` names, when set and not empty.
pub fn env_dir() -> Option<PathBuf> {
    std::env::var_os(APP_DATA_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// The app data folder: `G4_APP_DATA_DIR` first, otherwise the platform's folder for
/// [`APP_ID`] (what Tauri's `app_data_dir()` gives).
pub fn default_dir() -> Option<PathBuf> {
    dir_from(std::env::var_os(APP_DATA_ENV), platform_data_dir())
}

/// [`default_dir`] with the environment value and the platform's data folder given.
fn dir_from(env: Option<OsString>, platform: Option<PathBuf>) -> Option<PathBuf> {
    match env.filter(|value| !value.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => platform.map(|base| base.join(APP_ID)),
    }
}

/// An absolute path from the environment variable `name`.
fn absolute_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

/// The platform's folder for app data, as Tauri's `data_dir()`: macOS
/// `~/Library/Application Support`, Windows `%APPDATA%`, elsewhere `$XDG_DATA_HOME` or
/// `~/.local/share`.
fn platform_data_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        absolute_env("HOME").map(|home| home.join("Library").join("Application Support"))
    } else if cfg!(windows) {
        absolute_env("APPDATA")
    } else {
        absolute_env("XDG_DATA_HOME")
            .or_else(|| absolute_env("HOME").map(|home| home.join(".local").join("share")))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn written_0600_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        // The folder need not exist yet.
        let data = dir.path().join("app-data");
        let discovery = Discovery::new(4711, "ab12");
        assert_eq!(discovery.version, DISCOVERY_VERSION);
        assert_eq!(discovery.url, "http://127.0.0.1:4711/mcp");
        assert_eq!(discovery.pid, std::process::id());
        write(&data, &discovery).unwrap();
        let path = data.join(DISCOVERY_FILE);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let text: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            text,
            serde_json::json!({
                "version": 1,
                "url": "http://127.0.0.1:4711/mcp",
                "port": 4711,
                "token": "ab12",
                "pid": std::process::id(),
            })
        );
        assert_eq!(read(&data).unwrap(), discovery);
        remove(&data);
        assert!(!path.exists());
        assert!(read(&data).is_err());
        // Removing again is fine.
        remove(&data);
    }

    #[test]
    fn default_dir_prefers_env() {
        let platform = Some(PathBuf::from("/base"));
        assert_eq!(
            dir_from(Some("/elsewhere".into()), platform.clone()),
            Some(PathBuf::from("/elsewhere"))
        );
        assert_eq!(
            dir_from(None, platform.clone()),
            Some(PathBuf::from("/base").join(APP_ID))
        );
        // An empty value counts as unset.
        assert_eq!(
            dir_from(Some(OsString::new()), platform),
            Some(PathBuf::from("/base").join(APP_ID))
        );
        assert_eq!(dir_from(None, None), None);
    }
}
