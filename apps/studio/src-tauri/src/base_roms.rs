//! The base ROMs the app remembers, in `<app data>/base-roms.json` (pattern `agents.json`):
//! one entry per SHA-1 with the path, size and mtime it was last seen with, so a project
//! finds its base ROM again without hashing every time.
//!
//! Headless (`studio mcp --project`, `g4k`) the file is not read: there `--base-rom` and
//! `G4_BASE_ROM_DIR` apply.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use g4_kernel::base::{
    BaseSource, KnownRom, Located, Search, base_label, env_base_dir, locate, mtime_ms,
};
use g4_kernel::error::KernelError;
use g4_kernel::start::{BaseMark, RomInfo, rfc3339_utc};
use serde::{Deserialize, Serialize};

use crate::commands::policy::{is_original, originals_dir};
use crate::mcp::settings::write_private;

/// The settings file in the app data folder.
pub const BASE_ROMS_FILE: &str = "base-roms.json";

/// Version of the settings file.
pub const BASE_ROMS_VERSION: u32 = 1;

/// One remembered base ROM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseRomEntry {
    /// SHA-1 of the whole file, 40 lowercase hex digits.
    pub sha1: String,
    pub path: String,
    pub size: u64,
    /// Modification time in milliseconds since the Unix epoch, when last seen.
    pub mtime_ms: u64,
    pub game_code: String,
    /// The label shown for the ROM (`base_roms.yaml`, else `<title> (<game_code>)`).
    pub label: Option<String>,
    /// RFC 3339, UTC: when the SHA-1 of the file was last computed.
    pub verified: String,
}

/// The content of `base-roms.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BaseRomSettings {
    pub version: u32,
    pub roms: Vec<BaseRomEntry>,
}

impl Default for BaseRomSettings {
    fn default() -> BaseRomSettings {
        BaseRomSettings {
            version: BASE_ROMS_VERSION,
            roms: Vec::new(),
        }
    }
}

impl BaseRomSettings {
    /// The settings in `dir`; a missing or unreadable file gives the defaults.
    pub fn load(dir: &Path) -> BaseRomSettings {
        std::fs::read_to_string(dir.join(BASE_ROMS_FILE))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes the settings to `dir` (created if missing), 0600 and atomically
    /// ([`write_private`]).
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Ordner nicht anlegbar: {}: {e}", dir.display()))?;
        let mut text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        text.push('\n');
        write_private(&dir.join(BASE_ROMS_FILE), text.as_bytes())
    }

    /// The entry of the ROM with SHA-1 `sha1` (case ignored).
    pub fn find(&self, sha1: &str) -> Option<&BaseRomEntry> {
        self.roms.iter().find(|r| r.sha1.eq_ignore_ascii_case(sha1))
    }

    /// Adds `entry`, replacing the entry of the same SHA-1.
    pub fn upsert(&mut self, entry: BaseRomEntry) {
        match self
            .roms
            .iter_mut()
            .find(|r| r.sha1.eq_ignore_ascii_case(&entry.sha1))
        {
            Some(old) => *old = entry,
            None => self.roms.push(entry),
        }
    }

    /// Removes the entry of the ROM with SHA-1 `sha1`.
    pub fn remove(&mut self, sha1: &str) {
        self.roms.retain(|r| !r.sha1.eq_ignore_ascii_case(sha1));
    }
}

#[derive(Default)]
struct Inner {
    /// The app data folder; without one the settings live in memory only.
    dir: Option<PathBuf>,
    settings: BaseRomSettings,
    /// `--base-rom` (headless): searched first, never remembered.
    explicit: Option<PathBuf>,
    /// `G4_BASE_ROM_DIR`.
    search_dir: Option<PathBuf>,
    /// `G4_ROMS_DIR`, canonical: no ROM under it is a candidate (see [`is_original`]).
    originals: Option<PathBuf>,
}

impl Inner {
    /// Saves to the app data folder, if there is one. A failed save only costs a hash on
    /// the next search, so it is reported and otherwise ignored.
    fn save(&self) {
        if let Some(dir) = &self.dir
            && let Err(e) = self.settings.save(dir)
        {
            eprintln!("{BASE_ROMS_FILE} nicht gespeichert: {e}");
        }
    }
}

/// The base ROMs of the app (in `AppState`). Without [`BaseRoms::configure`] nothing is
/// remembered beyond memory and no folder is searched.
#[derive(Default)]
pub struct BaseRoms {
    inner: Mutex<Inner>,
}

impl BaseRoms {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Reads the settings from the app data folder `dir` (`None`: in memory only, as
    /// headless) and saves them there from now on; the search folder is
    /// `G4_BASE_ROM_DIR`, the originals are those of `G4_ROMS_DIR`.
    pub fn configure(&self, dir: Option<PathBuf>) {
        self.configure_with(dir, env_base_dir(), originals_dir());
    }

    /// [`BaseRoms::configure`] with the search folder and the originals folder given.
    pub(crate) fn configure_with(
        &self,
        dir: Option<PathBuf>,
        search_dir: Option<PathBuf>,
        originals: Option<PathBuf>,
    ) {
        let mut inner = self.lock();
        inner.settings = dir
            .as_deref()
            .map(BaseRomSettings::load)
            .unwrap_or_default();
        inner.dir = dir;
        inner.search_dir = search_dir;
        inner.originals = originals.map(|o| o.canonicalize().unwrap_or(o));
    }

    /// The ROM `--base-rom` names: searched before anything else.
    pub fn set_explicit(&self, path: Option<PathBuf>) {
        self.lock().explicit = path;
    }

    /// Remembers the ROM at `path` (inspected as `info`) with its canonical path, its size
    /// and mtime now, and saves. An original under `G4_ROMS_DIR` is not remembered (the search
    /// skips it anyway). Errors: the file cannot be read, or its path is no valid text.
    pub fn register(&self, path: &Path, info: &RomInfo) -> Result<(), String> {
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("ROM nicht lesbar: {}: {e}", path.display()))?;
        if self.is_skipped(&canonical) {
            return Ok(());
        }
        let meta = std::fs::metadata(&canonical)
            .map_err(|e| format!("ROM nicht lesbar: {}: {e}", path.display()))?;
        let entry = BaseRomEntry {
            sha1: info.sha1.to_ascii_lowercase(),
            path: path_text(&canonical)?,
            size: meta.len(),
            mtime_ms: mtime_ms(&meta),
            game_code: info.game_code.clone(),
            label: Some(
                info.known_base
                    .clone()
                    .unwrap_or_else(|| format!("{} ({})", info.title, info.game_code)),
            ),
            verified: now(),
        };
        let mut inner = self.lock();
        inner.settings.upsert(entry);
        inner.save();
        Ok(())
    }

    /// Searches as `base::locate` (explicit, known entry, sibling of `root`, search dir;
    /// originals skipped); a rehashed known entry gets its new mtime, a known entry that no
    /// longer matches is removed, a hit from sibling or dir is registered. Saves when
    /// anything changed.
    ///
    /// A known entry whose file is gone (a disk not plugged in) is no other ROM and stays.
    /// The search runs without the lock (it may hash a whole ROM).
    pub fn locate(&self, root: &Path, mark: &BaseMark) -> Result<Option<Located>, KernelError> {
        let (explicit, known, search_dir, originals) = {
            let inner = self.lock();
            let known = inner.settings.find(&mark.sha1).map(|e| KnownRom {
                path: PathBuf::from(&e.path),
                size: e.size,
                mtime_ms: e.mtime_ms,
            });
            (
                inner.explicit.clone(),
                known,
                inner.search_dir.clone(),
                inner.originals.clone(),
            )
        };
        let skip = |path: &Path| {
            originals.as_deref().is_some_and(|roms| {
                let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                is_original(&canonical, roms)
            })
        };
        let found = locate(
            mark,
            &Search {
                explicit: explicit.as_deref(),
                known: known.as_ref(),
                root: Some(root),
                dir: search_dir.as_deref(),
                skip: &skip,
            },
        )?;
        let change = match &found {
            Some(hit) => match hit.source {
                BaseSource::Explicit => None,
                BaseSource::Known if !hit.hashed => None,
                BaseSource::Known | BaseSource::Sibling | BaseSource::Dir => {
                    Some(Change::Upsert(hit))
                }
            },
            None => known
                .as_ref()
                .filter(|k| k.path.is_file())
                .map(Change::Remove),
        };
        if let Some(change) = change {
            let mut inner = self.lock();
            match change {
                Change::Upsert(hit) => {
                    // A remembered path stays as it was written; a new hit is kept canonical.
                    let path = if hit.source == BaseSource::Known {
                        hit.path.clone()
                    } else {
                        hit.path.canonicalize().unwrap_or_else(|_| hit.path.clone())
                    };
                    // Stored as text, like `register`: a path that is none is not remembered.
                    let Some(path) = path.to_str().map(str::to_owned) else {
                        return Ok(found);
                    };
                    let label = inner
                        .settings
                        .find(&mark.sha1)
                        .filter(|_| hit.source == BaseSource::Known)
                        .and_then(|e| e.label.clone())
                        .unwrap_or_else(|| base_label(mark));
                    inner.settings.upsert(BaseRomEntry {
                        sha1: mark.sha1.to_ascii_lowercase(),
                        path,
                        size: hit.size,
                        mtime_ms: hit.mtime_ms,
                        game_code: mark.game_code.clone(),
                        label: Some(label),
                        verified: now(),
                    });
                }
                // Only the entry that was searched: one registered meanwhile stays.
                Change::Remove(gone) => {
                    let unchanged = inner.settings.find(&mark.sha1).is_some_and(|e| {
                        Path::new(&e.path) == gone.path
                            && e.size == gone.size
                            && e.mtime_ms == gone.mtime_ms
                    });
                    if !unchanged {
                        return Ok(found);
                    }
                    inner.settings.remove(&mark.sha1);
                }
            }
            inner.save();
        }
        Ok(found)
    }

    pub fn settings(&self) -> BaseRomSettings {
        self.lock().settings.clone()
    }

    /// Whether `canonical` is an original under the configured `G4_ROMS_DIR`.
    fn is_skipped(&self, canonical: &Path) -> bool {
        self.lock()
            .originals
            .as_deref()
            .is_some_and(|roms| is_original(canonical, roms))
    }
}

/// What a search changes in the remembered entries.
enum Change<'a> {
    Upsert(&'a Located),
    Remove(&'a KnownRom),
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("Pfad ist kein gültiger Text: {}", path.display()))
}

/// The time now as RFC 3339, UTC.
fn now() -> String {
    rfc3339_utc(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// Size of the test ROMs: a header and one byte that tells them apart.
    const ROM_LEN: usize = 0x400;

    /// A header-only Platinum (US) ROM; `variant` is its last byte.
    fn rom_bytes(variant: u8) -> Vec<u8> {
        let mut bytes = vec![0u8; ROM_LEN];
        bytes[0x00..0x0A].copy_from_slice(b"POKEMON PL");
        bytes[0x0C..0x10].copy_from_slice(b"CPUE");
        bytes[ROM_LEN - 1] = variant;
        bytes
    }

    fn write_rom(path: &Path, variant: u8) -> String {
        let bytes = rom_bytes(variant);
        std::fs::write(path, &bytes).unwrap();
        g4_kernel::start::inspect_rom(path).unwrap().sha1
    }

    /// Moves the file's mtime one minute away from what it was.
    fn touch(path: &Path) {
        let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        let then = std::fs::metadata(path).unwrap().modified().unwrap();
        file.set_modified(then + Duration::from_secs(60)).unwrap();
    }

    fn mark(sha1: &str) -> BaseMark {
        BaseMark {
            version: 2,
            origin: "dspre".into(),
            sha1: sha1.into(),
            game_code: "CPUE".into(),
            title: "POKEMON PL".into(),
            revision: 0,
            size: ROM_LEN as u64,
            template: None,
            created: "2026-10-27T10:00:00Z".into(),
            assigned: Some("2026-10-27T10:00:00Z".into()),
            studio_version: "0.0.0".into(),
            secure_area_crc: 0,
            secure_area_sha1: String::new(),
        }
    }

    fn entry(sha1: &str, path: &str) -> BaseRomEntry {
        BaseRomEntry {
            sha1: sha1.into(),
            path: path.into(),
            size: 1,
            mtime_ms: 2,
            game_code: "CPUE".into(),
            label: None,
            verified: "2026-10-27T10:00:00Z".into(),
        }
    }

    /// The entry the app would have written for the ROM at `path` now.
    fn remembered(sha1: &str, path: &Path) -> BaseRomEntry {
        let meta = std::fs::metadata(path).unwrap();
        BaseRomEntry {
            size: meta.len(),
            mtime_ms: mtime_ms(&meta),
            ..entry(sha1, path.to_str().unwrap())
        }
    }

    /// A `BaseRoms` saving to `<tmp>/app`, no search folder, no originals, holding `roms`.
    fn base_roms(tmp: &Path, roms: Vec<BaseRomEntry>) -> (BaseRoms, PathBuf) {
        let app = tmp.join("app");
        BaseRomSettings {
            version: BASE_ROMS_VERSION,
            roms,
        }
        .save(&app)
        .unwrap();
        let base = BaseRoms::default();
        base.configure_with(Some(app.clone()), None, None);
        (base, app)
    }

    /// `<tmp>/Spiel_DSPRE_contents` with a `header.yaml` of game code CPUE.
    fn dspre_root(tmp: &Path) -> PathBuf {
        let root = tmp.join("Spiel_DSPRE_contents");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("header.yaml"), "gamecode: CPUE\n").unwrap();
        root
    }

    #[test]
    fn missing_or_broken_file_gives_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let settings = BaseRomSettings::load(dir.path());
        assert_eq!(settings.version, BASE_ROMS_VERSION);
        assert!(settings.roms.is_empty());
        std::fs::write(dir.path().join(BASE_ROMS_FILE), "{ kaputt").unwrap();
        assert_eq!(
            BaseRomSettings::load(dir.path()),
            BaseRomSettings::default()
        );
    }

    #[test]
    fn save_is_private_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("app-data");
        let settings = BaseRomSettings {
            version: BASE_ROMS_VERSION,
            roms: vec![BaseRomEntry {
                label: Some("Platin DE (Testprojekt)".into()),
                ..entry("d40f19ac24d67c721a4ab25e1c013154443d75b4", "/roms/pt.nds")
            }],
        };
        settings.save(&app).unwrap();
        let file = app.join(BASE_ROMS_FILE);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(BaseRomSettings::load(&app), settings);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "version": 1,
                "roms": [{
                    "sha1": "d40f19ac24d67c721a4ab25e1c013154443d75b4",
                    "path": "/roms/pt.nds",
                    "size": 1,
                    "mtime_ms": 2,
                    "game_code": "CPUE",
                    "label": "Platin DE (Testprojekt)",
                    "verified": "2026-10-27T10:00:00Z",
                }],
            })
        );
        let names: Vec<_> = std::fs::read_dir(&app)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, [BASE_ROMS_FILE]);
    }

    #[test]
    fn upsert_replaces_the_entry_of_the_same_sha1() {
        let mut settings = BaseRomSettings::default();
        settings.upsert(entry("aaa", "/a.nds"));
        settings.upsert(entry("bbb", "/b.nds"));
        settings.upsert(entry("aaa", "/neu/a.nds"));
        assert_eq!(settings.roms.len(), 2);
        assert_eq!(settings.find("aaa").unwrap().path, "/neu/a.nds");
        assert_eq!(settings.find("bbb").unwrap().path, "/b.nds");
        assert_eq!(settings.find("ccc"), None);
        settings.remove("aaa");
        assert_eq!(settings.find("aaa"), None);
        assert_eq!(settings.roms.len(), 1);
        // Removing what is not there changes nothing.
        settings.remove("ccc");
        assert_eq!(settings.roms.len(), 1);
    }

    #[test]
    fn register_writes_size_mtime_and_label() {
        let tmp = tempfile::tempdir().unwrap();
        let (base, app) = base_roms(tmp.path(), Vec::new());
        let (_rom_tmp, rom) = g4_kernel::test_support::synthetic_rom();
        let mut info = g4_kernel::start::inspect_rom(&rom).unwrap();
        info.known_base = Some("Platin US (Test)".into());
        base.register(&rom, &info).unwrap();
        let meta = std::fs::metadata(&rom).unwrap();
        let saved = BaseRomSettings::load(&app);
        assert_eq!(saved, base.settings());
        let got = saved.find(&info.sha1).unwrap();
        assert_eq!(
            got.path,
            std::fs::canonicalize(&rom).unwrap().to_str().unwrap()
        );
        assert_eq!(got.size, meta.len());
        assert_eq!(got.mtime_ms, mtime_ms(&meta));
        assert_eq!(got.game_code, "CPUE");
        assert_eq!(got.label.as_deref(), Some("Platin US (Test)"));
        assert_eq!(got.verified.len(), "2026-10-27T10:00:00Z".len());
        assert!(got.verified.ends_with('Z'));

        // Without a known label the entry is named by title and game code; registering the
        // same SHA-1 again replaces the entry.
        info.known_base = None;
        base.register(&rom, &info).unwrap();
        let saved = BaseRomSettings::load(&app);
        assert_eq!(saved.roms.len(), 1);
        assert_eq!(
            saved.find(&info.sha1).unwrap().label.as_deref(),
            Some("POKEMON PL (CPUE)")
        );
        // A file that is not there is not registered.
        assert!(base.register(&tmp.path().join("fehlt.nds"), &info).is_err());
    }

    #[test]
    fn locate_updates_a_rehashed_entry_and_drops_a_stale_one() {
        let tmp = tempfile::tempdir().unwrap();
        let rom = tmp.path().join("basis.nds");
        let sha = write_rom(&rom, 1);
        let old = remembered(&sha, &rom);
        let (base, app) = base_roms(tmp.path(), vec![old.clone()]);
        let root = tmp.path().join("projekt");
        std::fs::create_dir(&root).unwrap();

        // Unchanged: trusted without a hash, nothing saved.
        let found = base.locate(&root, &mark(&sha)).unwrap().unwrap();
        assert_eq!(found.source, BaseSource::Known);
        assert!(!found.hashed);
        assert_eq!(BaseRomSettings::load(&app).roms, std::slice::from_ref(&old));

        // Written again with the same bytes: hashed, the entry gets the new mtime.
        write_rom(&rom, 1);
        touch(&rom);
        let now = mtime_ms(&std::fs::metadata(&rom).unwrap());
        assert_ne!(now, old.mtime_ms);
        let found = base.locate(&root, &mark(&sha)).unwrap().unwrap();
        assert_eq!(found.source, BaseSource::Known);
        assert!(found.hashed);
        let saved = BaseRomSettings::load(&app);
        assert_eq!(saved, base.settings());
        assert_eq!(saved.find(&sha).unwrap().mtime_ms, now);
        assert_eq!(saved.find(&sha).unwrap().path, old.path);

        // Replaced in place by another ROM of the same size: not found, the entry goes.
        write_rom(&rom, 2);
        touch(&rom);
        assert_eq!(base.locate(&root, &mark(&sha)).unwrap(), None);
        assert_eq!(base.settings().find(&sha), None);
        assert_eq!(BaseRomSettings::load(&app).find(&sha), None);
    }

    #[test]
    fn locate_keeps_the_entry_of_a_rom_that_is_not_there() {
        // A ROM on a disk that is not plugged in is no other ROM: the entry stays.
        let tmp = tempfile::tempdir().unwrap();
        let rom = tmp.path().join("basis.nds");
        let sha = write_rom(&rom, 1);
        let old = remembered(&sha, &rom);
        std::fs::remove_file(&rom).unwrap();
        let (base, app) = base_roms(tmp.path(), vec![old.clone()]);
        assert_eq!(base.locate(tmp.path(), &mark(&sha)).unwrap(), None);
        assert_eq!(BaseRomSettings::load(&app).roms, [old]);
    }

    #[test]
    fn locate_registers_a_sibling_hit() {
        let tmp = tempfile::tempdir().unwrap();
        let root = dspre_root(tmp.path());
        let nds = tmp.path().join("Spiel.nds");
        let sha = write_rom(&nds, 1);
        let (base, app) = base_roms(tmp.path(), vec![entry("other", "/x.nds")]);
        let found = base.locate(&root, &mark(&sha)).unwrap().unwrap();
        assert_eq!(found.source, BaseSource::Sibling);
        let saved = BaseRomSettings::load(&app);
        assert_eq!(saved, base.settings());
        assert_eq!(saved.roms.len(), 2);
        let got = saved.find(&sha).unwrap();
        let meta = std::fs::metadata(&nds).unwrap();
        assert_eq!(
            got.path,
            std::fs::canonicalize(&nds).unwrap().to_str().unwrap()
        );
        assert_eq!((got.size, got.mtime_ms), (meta.len(), mtime_ms(&meta)));
        assert_eq!(got.game_code, "CPUE");
        assert_eq!(got.label.as_deref(), Some("POKEMON PL (CPUE)"));

        // The next search takes the remembered entry, without a hash.
        let found = base.locate(&root, &mark(&sha)).unwrap().unwrap();
        assert_eq!(found.source, BaseSource::Known);
        assert!(!found.hashed);
    }

    #[test]
    fn locate_registers_a_hit_in_the_search_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("roms");
        std::fs::create_dir(&dir).unwrap();
        let sha = write_rom(&dir.join("a.nds"), 1);
        let app = tmp.path().join("app");
        let base = BaseRoms::default();
        base.configure_with(Some(app.clone()), Some(dir.clone()), None);
        let found = base.locate(tmp.path(), &mark(&sha)).unwrap().unwrap();
        assert_eq!(found.source, BaseSource::Dir);
        assert!(BaseRomSettings::load(&app).find(&sha).is_some());
    }

    #[test]
    fn locate_takes_the_explicit_rom_first_and_does_not_remember_it() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.nds");
        let b = tmp.path().join("b.nds");
        let sha = write_rom(&a, 1);
        let other = write_rom(&b, 2);
        let base = BaseRoms::default();
        base.configure_with(None, None, None);
        base.set_explicit(Some(a.clone()));
        let found = base.locate(tmp.path(), &mark(&sha)).unwrap().unwrap();
        assert_eq!(found.source, BaseSource::Explicit);
        assert_eq!(found.path, a);
        assert!(base.settings().roms.is_empty());
        // Another ROM there is an error.
        base.set_explicit(Some(b));
        assert!(matches!(
            base.locate(tmp.path(), &mark(&sha)),
            Err(KernelError::Refused(_))
        ));
        let _ = other;
    }

    #[test]
    fn register_skips_an_original_under_g4_roms_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let roms = tmp.path().join("Roms");
        let copies = roms.join("_testkopien");
        std::fs::create_dir_all(&copies).unwrap();
        let original = roms.join("x.nds");
        let copy = copies.join("x.nds");
        write_rom(&original, 1);
        write_rom(&copy, 1);
        let info = g4_kernel::start::inspect_rom(&original).unwrap();
        let app = tmp.path().join("app");
        let base = BaseRoms::default();
        base.configure_with(Some(app.clone()), None, Some(roms));
        base.register(&original, &info).unwrap();
        assert!(base.settings().roms.is_empty());
        assert!(!app.join(BASE_ROMS_FILE).exists());
        // The copy is remembered.
        base.register(&copy, &info).unwrap();
        assert_eq!(
            BaseRomSettings::load(&app).find(&info.sha1).unwrap().path,
            std::fs::canonicalize(&copy).unwrap().to_str().unwrap()
        );
    }

    #[test]
    fn locate_skips_an_original_under_g4_roms_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let roms = tmp.path().join("Roms");
        let copies = roms.join("_testkopien");
        std::fs::create_dir_all(&copies).unwrap();
        let original = roms.join("x.nds");
        let copy = copies.join("x.nds");
        let sha = write_rom(&original, 1);
        write_rom(&copy, 1);
        let app = tmp.path().join("app");
        let root = tmp.path().join("projekt");
        std::fs::create_dir(&root).unwrap();

        // A remembered original: skipped, not found.
        let base = BaseRoms::default();
        BaseRomSettings {
            version: BASE_ROMS_VERSION,
            roms: vec![remembered(&sha, &original)],
        }
        .save(&app)
        .unwrap();
        base.configure_with(Some(app.clone()), None, Some(roms.clone()));
        assert_eq!(base.locate(&root, &mark(&sha)).unwrap(), None);
        // In the search folder as well.
        base.configure_with(Some(app.clone()), Some(roms.clone()), Some(roms.clone()));
        assert_eq!(base.locate(&root, &mark(&sha)).unwrap(), None);

        // The copy under `_testkopien` is found.
        BaseRomSettings {
            version: BASE_ROMS_VERSION,
            roms: vec![remembered(&sha, &copy)],
        }
        .save(&app)
        .unwrap();
        base.configure_with(Some(app.clone()), None, Some(roms.clone()));
        let found = base.locate(&root, &mark(&sha)).unwrap().unwrap();
        assert_eq!(found.path, copy);
        assert_eq!(found.source, BaseSource::Known);
    }
}
