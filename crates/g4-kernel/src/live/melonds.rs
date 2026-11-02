//! The melonDS side of play mode: bundle version, GDB-stub configuration and launch.
//!
//! The functions take every path as a parameter; only [`default_config_path`] knows where the
//! user's `melonDS.toml` lives, so tests work in temp directories and never touch it.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use toml::{Table, Value};

use super::gdb::LiveError;

/// The oldest melonDS whose GDB stub the studio talks to (1.1 introduced the stub).
pub const MIN_VERSION: (u32, u32) = (1, 1);
/// The port melonDS listens on for the ARM9 stub unless the user changed it.
pub const DEFAULT_PORT: u16 = 3333;
/// Appended to the config file name for the one-time backup made by [`enable_stub`].
pub const BACKUP_SUFFIX: &str = ".g4-backup";

/// The three settings of `melonDS.toml` that decide whether the GDB stub is reachable.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct StubConfig {
    /// `[Instance0.Gdb] Enabled`.
    pub enabled: bool,
    /// `[Instance0.Gdb.ARM9] BreakOnStartup`.
    pub break_on_startup: bool,
    /// `[Instance0.Gdb.ARM9] Port`.
    pub port: u16,
    /// `[JIT] Enable`; with the JIT on melonDS opens no stub.
    pub jit: bool,
}

/// What [`inspect`] found about an installed melonDS.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct MelonDs {
    pub bundle: PathBuf,
    pub binary: PathBuf,
    /// `CFBundleShortVersionString`, empty when the plist has none (then [`version_ok`] is false).
    pub version: String,
    pub config_path: PathBuf,
    /// `None` when the config file is missing or unreadable.
    pub config: Option<StubConfig>,
}

/// `<bundle>/Contents/MacOS/melonDS` when that file exists.
pub fn bundle_binary(bundle: &Path) -> Option<PathBuf> {
    let binary = bundle.join("Contents/MacOS/melonDS");
    binary.is_file().then_some(binary)
}

/// `CFBundleShortVersionString` of `<bundle>/Contents/Info.plist`: a text scan for the
/// `<string>` that follows the key (melonDS ships an XML plist).
pub fn bundle_version(bundle: &Path) -> Option<String> {
    let plist = fs::read_to_string(bundle.join("Contents/Info.plist")).ok()?;
    let key = "<key>CFBundleShortVersionString</key>";
    let after_key = &plist[plist.find(key)? + key.len()..];
    let start = after_key.find("<string>")? + "<string>".len();
    let end = start + after_key[start..].find("</string>")?;
    let version = after_key[start..end].trim();
    (!version.is_empty()).then(|| version.to_string())
}

/// Whether `version` (`major.minor[.patch]`) is at least [`MIN_VERSION`]; compared as numbers
/// (`1.10` is newer than `1.9`). An empty or non-numeric version is not ok.
pub fn version_ok(version: &str) -> bool {
    let mut parts = version.trim().split('.');
    let number = |part: Option<&str>| part.and_then(|p| p.parse::<u32>().ok());
    let Some(major) = number(parts.next()) else {
        return false;
    };
    // `2` alone means 2.0
    let minor = match parts.next() {
        Some(p) => match p.parse::<u32>() {
            Ok(n) => n,
            Err(_) => return false,
        },
        None => 0,
    };
    (major, minor) >= MIN_VERSION
}

/// `$HOME/Library/Preferences/melonDS/melonDS.toml`, `None` without `$HOME`.
pub fn default_config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty())?;
    Some(PathBuf::from(home).join("Library/Preferences/melonDS/melonDS.toml"))
}

fn invalid(path: &Path, e: impl std::fmt::Display) -> LiveError {
    LiveError::Io(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{}: {e}", path.display()),
    ))
}

fn read_table(path: &Path) -> Result<Table, LiveError> {
    let text = fs::read_to_string(path)?;
    toml::from_str::<Table>(&text).map_err(|e| invalid(path, e))
}

fn lookup<'a>(table: &'a Table, keys: &[&str]) -> Option<&'a Value> {
    let (last, parents) = keys.split_last()?;
    let mut current = table;
    for key in parents {
        current = current.get(*key)?.as_table()?;
    }
    current.get(*last)
}

fn config_of(table: &Table) -> StubConfig {
    let flag = |keys: &[&str], default: bool| {
        lookup(table, keys)
            .and_then(Value::as_bool)
            .unwrap_or(default)
    };
    let port = lookup(table, &["Instance0", "Gdb", "ARM9", "Port"])
        .and_then(Value::as_integer)
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(DEFAULT_PORT);
    StubConfig {
        enabled: flag(&["Instance0", "Gdb", "Enabled"], false),
        break_on_startup: flag(&["Instance0", "Gdb", "ARM9", "BreakOnStartup"], false),
        port,
        jit: flag(&["JIT", "Enable"], true),
    }
}

/// Reads the stub settings from `path`. Missing keys take melonDS's defaults (`enabled` false,
/// `break_on_startup` false, port 3333, `jit` true). A missing or malformed file is an error.
pub fn read_config(path: &Path) -> Result<StubConfig, LiveError> {
    Ok(config_of(&read_table(path)?))
}

/// Whether melonDS will open a stub a client can use: enabled, not halting on start, no JIT.
pub fn stub_ready(c: &StubConfig) -> bool {
    c.enabled && !c.break_on_startup && !c.jit
}

fn set_key(table: &mut Table, keys: &[&str], value: bool) {
    let Some((last, parents)) = keys.split_last() else {
        return;
    };
    let mut current = table;
    for key in parents {
        let entry = current
            .entry((*key).to_string())
            .or_insert_with(|| Value::Table(Table::new()));
        if !entry.is_table() {
            *entry = Value::Table(Table::new());
        }
        let Value::Table(next) = entry else {
            unreachable!("just made a table")
        };
        current = next;
    }
    current.insert((*last).to_string(), Value::Boolean(value));
}

/// Writes `bytes` to `path` through a temp file in the same directory and a rename, so a crash
/// never leaves a half-written config.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "config path has no file name")
    })?;
    let mut temp_name = name.to_os_string();
    temp_name.push(format!(".g4-tmp-{}", std::process::id()));
    let temp = path.with_file_name(temp_name);
    let written = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written
}

/// Switches the GDB stub on in the melonDS config at `path`: `Instance0.Gdb.Enabled = true`,
/// `Instance0.Gdb.ARM9.BreakOnStartup = false`, `JIT.Enable = false`. Every other key (including
/// the port) is kept; missing tables are created.
///
/// An existing file is first copied to `<path>.g4-backup`, but only when no backup exists yet, so
/// the backup stays the user's original. A missing file is created (with its directory) and gets
/// no backup. A malformed file is an error and stays untouched. The new file is written
/// atomically. Returns the resulting settings. melonDS must not be running: it rewrites the file
/// on exit.
pub fn enable_stub(path: &Path) -> Result<StubConfig, LiveError> {
    let mut table = if path.exists() {
        let table = read_table(path)?;
        let mut backup_name = path
            .file_name()
            .ok_or_else(|| invalid(path, "no file name"))?
            .to_os_string();
        backup_name.push(BACKUP_SUFFIX);
        let backup = path.with_file_name(backup_name);
        if !backup.exists() {
            fs::copy(path, &backup)?;
        }
        table
    } else {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        Table::new()
    };
    set_key(&mut table, &["Instance0", "Gdb", "Enabled"], true);
    set_key(
        &mut table,
        &["Instance0", "Gdb", "ARM9", "BreakOnStartup"],
        false,
    );
    set_key(&mut table, &["JIT", "Enable"], false);
    let text = toml::to_string(&table).map_err(|e| invalid(path, e))?;
    write_atomic(path, text.as_bytes())?;
    Ok(config_of(&table))
}

/// Whether a process named `melonDS` runs (`pgrep -x melonDS`; read-only).
pub fn is_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "melonDS"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Looks at the melonDS bundle: `None` when it has no executable. With an executable but no
/// version in the plist, `version` is empty, so [`version_ok`] is false. `config_path` defaults
/// to [`default_config_path`] (without `$HOME` the path is empty and `config` is `None`).
pub fn inspect(bundle: &Path, config_path: Option<&Path>) -> Option<MelonDs> {
    let binary = bundle_binary(bundle)?;
    let config_path = config_path
        .map(Path::to_path_buf)
        .or_else(default_config_path)
        .unwrap_or_default();
    Some(MelonDs {
        bundle: bundle.to_path_buf(),
        binary,
        version: bundle_version(bundle).unwrap_or_default(),
        config: read_config(&config_path).ok(),
        config_path,
    })
}

/// Starts `binary` with the ROM as its argument and null stdio; the caller owns the [`Child`]
/// (wait for it to avoid a zombie). A missing binary is [`LiveError::Io`].
pub fn launch(binary: &Path, rom: &Path) -> Result<Child, LiveError> {
    Ok(Command::new(binary)
        .arg(rom)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::LiveError;
    use std::path::Path;

    fn bundle_with_plist(plist: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Contents")).unwrap();
        if let Some(body) = plist {
            std::fs::write(dir.path().join("Contents/Info.plist"), body).unwrap();
        }
        dir
    }

    #[test]
    fn the_version_is_read_from_the_plist_and_compared_as_numbers() {
        let plist = "<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><dict>\n\
            <key>CFBundleName</key>\n<string>melonDS</string>\n\
            <key>CFBundleShortVersionString</key>\n\t<string>1.1</string>\n\
            </dict></plist>";
        let bundle = bundle_with_plist(Some(plist));
        assert_eq!(bundle_version(bundle.path()), Some("1.1".to_string()));
        assert!(version_ok("1.1"));
        assert!(!version_ok("1.0.1"));
        assert!(version_ok("2.0"));
        assert!(version_ok("1.10"));
        assert!(!version_ok(""));
        assert!(!version_ok("abc"));

        let without = bundle_with_plist(Some(
            "<plist><dict><key>CFBundleName</key><string>melonDS</string></dict></plist>",
        ));
        assert_eq!(bundle_version(without.path()), None);
        assert_eq!(bundle_version(bundle_with_plist(None).path()), None);
    }

    #[test]
    fn read_config_takes_the_three_keys_and_defaults_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("melonDS.toml");
        std::fs::write(
            &path,
            "[Instance0.Gdb]\nEnabled = true\n[Instance0.Gdb.ARM9]\nBreakOnStartup = false\nPort = 3333\n[JIT]\nEnable = false\n",
        )
        .unwrap();
        let config = read_config(&path).unwrap();
        assert_eq!(
            config,
            StubConfig {
                enabled: true,
                break_on_startup: false,
                port: 3333,
                jit: false
            }
        );
        assert!(stub_ready(&config));

        std::fs::write(&path, "").unwrap();
        let config = read_config(&path).unwrap();
        assert_eq!(
            config,
            StubConfig {
                enabled: false,
                break_on_startup: false,
                port: 3333,
                jit: true
            }
        );
        assert!(!stub_ready(&config));

        assert!(read_config(&dir.path().join("missing.toml")).is_err());
        std::fs::write(&path, "this is = = not toml").unwrap();
        assert!(read_config(&path).is_err());
    }

    #[test]
    fn enable_stub_backs_up_once_and_keeps_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("melonDS.toml");
        let original = "[Audio]\nVolume = 128\n\n[JIT]\nEnable = true\nMaxBlockSize = 32\n\n[Instance0.Gdb.ARM9]\nPort = 3400\n";
        std::fs::write(&path, original).unwrap();

        let config = enable_stub(&path).unwrap();
        assert!(stub_ready(&config));
        assert_eq!(config.port, 3400, "the port is the user's choice");
        let backup = dir.path().join(format!("melonDS.toml{BACKUP_SUFFIX}"));
        assert_eq!(std::fs::read(&backup).unwrap(), original.as_bytes());
        assert!(stub_ready(&read_config(&path).unwrap()));
        let table: toml::Table = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(table["Audio"]["Volume"].as_integer(), Some(128));
        assert_eq!(table["JIT"]["MaxBlockSize"].as_integer(), Some(32));

        // a second call leaves the backup of the very first state alone
        std::fs::write(&path, "[Audio]\nVolume = 7\n").unwrap();
        enable_stub(&path).unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(), original.as_bytes());
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            leftovers.len(),
            2,
            "no temp file stays behind: {leftovers:?}"
        );
    }

    #[test]
    fn enable_stub_creates_a_missing_file_without_backup_and_refuses_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/melonDS.toml");
        let config = enable_stub(&path).unwrap();
        assert!(stub_ready(&config));
        assert!(stub_ready(&read_config(&path).unwrap()));
        assert!(
            !dir.path()
                .join(format!("sub/melonDS.toml{BACKUP_SUFFIX}"))
                .exists()
        );

        let bad = dir.path().join("bad.toml");
        std::fs::write(&bad, "= = nope").unwrap();
        assert!(enable_stub(&bad).is_err());
        assert_eq!(std::fs::read_to_string(&bad).unwrap(), "= = nope");
    }

    #[test]
    fn bundle_binary_needs_the_executable() {
        let bundle = bundle_with_plist(None);
        assert_eq!(bundle_binary(bundle.path()), None);
        std::fs::create_dir_all(bundle.path().join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.path().join("Contents/MacOS/melonDS"), b"x").unwrap();
        assert_eq!(
            bundle_binary(bundle.path()),
            Some(bundle.path().join("Contents/MacOS/melonDS"))
        );
    }

    #[test]
    fn inspect_combines_bundle_and_config() {
        let bundle = bundle_with_plist(Some(
            "<dict><key>CFBundleShortVersionString</key><string>1.1</string></dict>",
        ));
        let cfg_dir = tempfile::tempdir().unwrap();
        let cfg = cfg_dir.path().join("melonDS.toml");
        assert_eq!(inspect(bundle.path(), Some(&cfg)), None, "no binary");

        std::fs::create_dir_all(bundle.path().join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.path().join("Contents/MacOS/melonDS"), b"x").unwrap();
        let found = inspect(bundle.path(), Some(&cfg)).unwrap();
        assert_eq!(found.version, "1.1");
        assert_eq!(found.config, None, "file missing");
        assert_eq!(found.config_path, cfg);

        std::fs::write(&cfg, "[Instance0.Gdb]\nEnabled = true\n").unwrap();
        let found = inspect(bundle.path(), Some(&cfg)).unwrap();
        assert_eq!(found.config.map(|c| c.enabled), Some(true));

        std::fs::write(bundle.path().join("Contents/Info.plist"), "<dict/>").unwrap();
        assert_eq!(inspect(bundle.path(), Some(&cfg)).unwrap().version, "");
    }

    #[test]
    fn launch_refuses_a_missing_binary() {
        let result = launch(Path::new("/nonexistent"), Path::new("/nonexistent.nds"));
        assert!(matches!(result, Err(LiveError::Io(_))));
    }

    #[test]
    fn is_running_does_not_panic() {
        let _ = is_running();
    }
}
