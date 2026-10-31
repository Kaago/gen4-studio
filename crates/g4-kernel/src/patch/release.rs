//! The list of releases a project has published: `.g4/releases.json`, versioned with the
//! project. Version proposal and check, and the file name of a release's patch. See
//! `docs/formats/xdelta_patch.md`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::fsutil::write_atomic;
use crate::project::Project;

use super::project_stem;

/// The list, relative to the project root.
pub const RELEASES_FILE: &str = ".g4/releases.json";
/// The format version of [`RELEASES_FILE`].
pub const RELEASES_VERSION: u32 = 1;
/// The first version proposed for a project without a release.
pub const FIRST_VERSION: &str = "0.1.0";
/// The longest version in characters.
pub const VERSION_MAX: usize = 32;

/// One published patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Release {
    pub version: String,
    /// `YYYY-MM-DDTHH:MM:SSZ`, UTC.
    pub date: String,
    /// The patch file: relative to the project root with `/` when below it, else absolute
    /// (see [`release_file_entry`]).
    pub file: String,
    pub notes: String,
    pub base_sha1: String,
    /// Size of the patch file in bytes.
    pub size: u64,
}

/// The content of [`RELEASES_FILE`], oldest release first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Releases {
    pub version: u32,
    pub releases: Vec<Release>,
}

impl Releases {
    pub fn empty() -> Releases {
        Releases {
            version: RELEASES_VERSION,
            releases: Vec::new(),
        }
    }

    /// The release published last.
    pub fn last(&self) -> Option<&Release> {
        self.releases.last()
    }

    pub fn has_version(&self, version: &str) -> bool {
        self.releases.iter().any(|r| r.version == version)
    }
}

/// The refusal for a list that does not read; followed by `: <path>: <reason>` or ` (Fassung
/// 2 ist neuer als diese App)`.
pub const RELEASES_BROKEN: &str = "Die Liste der Veröffentlichungen ist nicht lesbar";
/// [`check_version`]: a release with this version is in the list.
pub const VERSION_TAKEN: &str = "Diese Version gibt es schon.";
/// [`check_version`]: the version is empty, too long or has characters a file name cannot.
pub const BAD_VERSION: &str = "Die Version darf nicht leer sein, höchstens 32 Zeichen haben und weder Leerzeichen noch / \\ : * ? \" < > | enthalten.";

/// `.g4/releases.json`; a missing file is the empty list, a broken or newer one
/// `Refused(RELEASES_BROKEN …)`.
pub fn read_releases(root: &Path) -> Result<Releases, KernelError> {
    let path = root.join(RELEASES_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Releases::empty()),
        Err(e) => return Err(broken(&path, &e.to_string())),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| broken(&path, &e.to_string()))?;
    // A newer file may have other fields: its version is read before the strict parse.
    if let Some(version) = value.get("version").and_then(serde_json::Value::as_u64)
        && version > u64::from(RELEASES_VERSION)
    {
        return Err(KernelError::Refused(format!(
            "{RELEASES_BROKEN} (Fassung {version} ist neuer als diese App)"
        )));
    }
    let releases: Releases =
        serde_json::from_value(value).map_err(|e| broken(&path, &e.to_string()))?;
    if releases.version != RELEASES_VERSION {
        return Err(broken(
            &path,
            &format!("unbekannte Fassung {}", releases.version),
        ));
    }
    Ok(releases)
}

fn broken(path: &Path, reason: &str) -> KernelError {
    KernelError::Refused(format!("{RELEASES_BROKEN}: {}: {reason}", path.display()))
}

/// Appends `release` and writes the file atomically (2 spaces, fields in the order of
/// [`Release`]); returns the new list. A broken or newer file is refused and stays as it is.
pub fn append_release(root: &Path, release: Release) -> Result<Releases, KernelError> {
    let mut releases = read_releases(root)?;
    releases.releases.push(release);
    let mut text = serde_json::to_string_pretty(&releases)
        .map_err(|e| KernelError::InvalidProject(format!("{RELEASES_FILE}: {e}")))?;
    text.push('\n');
    write_atomic(root, &root.join(RELEASES_FILE), text.as_bytes())?;
    Ok(releases)
}

/// The proposal for the next version: [`FIRST_VERSION`] for an empty list; else the leading
/// `major.minor.patch` (three numbers separated by dots) of the last release's version with
/// the patch number one higher and everything after it (`-beta`, `rc1`, `.4`) dropped; a
/// version without that prefix stays as it is (the user types the next one).
pub fn next_version(releases: &Releases) -> String {
    let Some(last) = releases.last() else {
        return FIRST_VERSION.to_owned();
    };
    leading_numbers(&last.version)
        .and_then(|[major, minor, patch]| {
            Some(format!("{major}.{minor}.{}", patch.checked_add(1)?))
        })
        .unwrap_or_else(|| last.version.clone())
}

/// The numbers of a leading `\d+.\d+.\d+`, if the version starts with one.
fn leading_numbers(version: &str) -> Option<[u64; 3]> {
    let mut rest = version;
    let mut numbers = [0u64; 3];
    for (i, number) in numbers.iter_mut().enumerate() {
        if i > 0 {
            rest = rest.strip_prefix('.')?;
        }
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        *number = rest[..digits].parse().ok()?;
        rest = &rest[digits..];
    }
    Some(numbers)
}

/// `Err(BAD_VERSION)` for a version that is empty, longer than [`VERSION_MAX`] characters or
/// holds white space, a control character or one of `/ \ : * ? " < > |` (it ends up in the
/// patch's file name); `Err(VERSION_TAKEN)` for one the list has; else `Ok(())`.
pub fn check_version(version: &str, releases: &Releases) -> Result<(), String> {
    let unsafe_char = |c: char| {
        c.is_whitespace()
            || c.is_control()
            || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    };
    if version.is_empty() || version.chars().count() > VERSION_MAX || version.contains(unsafe_char)
    {
        return Err(BAD_VERSION.to_owned());
    }
    if releases.has_version(version) {
        return Err(VERSION_TAKEN.to_owned());
    }
    Ok(())
}

/// `<Projekt>-v<Version>.xdelta` with [`project_stem`].
pub fn release_file_name(project: &Project, version: &str) -> String {
    format!("{}-v{version}.xdelta", project_stem(project))
}

/// `file` as it goes into the list: relative to `root` with `/` when below it, else absolute.
pub fn release_file_entry(root: &Path, file: &Path) -> String {
    match file.strip_prefix(root) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        _ => file.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(version: &str) -> Release {
        Release {
            version: version.into(),
            date: "2026-10-31T10:00:00Z".into(),
            file: format!("build/Mein Hack-v{version}.xdelta"),
            notes: "Neue Karte\nZweite Zeile".into(),
            base_sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4".into(),
            size: 1234,
        }
    }

    fn list(versions: &[&str]) -> Releases {
        Releases {
            version: RELEASES_VERSION,
            releases: versions.iter().map(|v| release(v)).collect(),
        }
    }

    #[test]
    fn a_missing_file_is_the_empty_list() {
        let tmp = tempfile::tempdir().unwrap();
        let releases = read_releases(tmp.path()).unwrap();
        assert_eq!(releases, Releases::empty());
        assert_eq!(releases.last(), None);
        assert_eq!(next_version(&releases), "0.1.0");
    }

    #[test]
    fn append_writes_camel_case_keys_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let first = append_release(root, release("0.1.0")).unwrap();
        assert_eq!(first.releases.len(), 1);
        let second = append_release(root, release("0.1.1")).unwrap();
        assert_eq!(second.releases.len(), 2);
        assert_eq!(second.last().unwrap().version, "0.1.1");

        let text = std::fs::read_to_string(root.join(RELEASES_FILE)).unwrap();
        assert!(text.contains("\"baseSha1\""), "{text}");
        assert!(text.contains("\"releases\""), "{text}");
        assert!(!text.contains("base_sha1"), "{text}");
        assert!(
            text.find("\"0.1.0\"").unwrap() < text.find("\"0.1.1\"").unwrap(),
            "{text}"
        );
        assert!(
            text.starts_with("{\n  \"version\": 1,\n  \"releases\": [\n    {\n"),
            "{text}"
        );
        assert!(text.ends_with("}\n"), "{text}");
        // The fields stand in the order of the struct.
        let at = |k: &str| text.find(&format!("\"{k}\"")).unwrap();
        assert!(at("version") < at("releases"));
        let entry = &text[at("releases")..];
        let keys = ["version", "date", "file", "notes", "baseSha1", "size"];
        let positions: Vec<usize> = keys
            .iter()
            .map(|k| entry.find(&format!("\"{k}\"")).unwrap())
            .collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]), "{text}");

        assert_eq!(read_releases(root).unwrap(), second);
        // The temp file is gone.
        assert_eq!(std::fs::read_dir(root.join(".g4/tmp")).unwrap().count(), 0);
    }

    #[test]
    fn next_version_bumps_the_patch_number_and_drops_a_suffix() {
        let next = |v: &str| next_version(&list(&[v]));
        assert_eq!(next("0.1.0"), "0.1.1");
        assert_eq!(next("1.2.9"), "1.2.10");
        assert_eq!(next("2.0.0-beta"), "2.0.1");
        assert_eq!(next("2.0.0+b5"), "2.0.1");
        assert_eq!(next("1.2.3rc1"), "1.2.4");
        assert_eq!(next("1.2.3_b"), "1.2.4");
        assert_eq!(next("1.2.3.4"), "1.2.4");
        assert_eq!(next("v1.2.3"), "v1.2.3");
        assert_eq!(next(".1.2.3"), ".1.2.3");
        assert_eq!(next("1..3"), "1..3");
        assert_eq!(next("final"), "final");
        assert_eq!(next("1.2"), "1.2");
        assert_eq!(next("1.x.3"), "1.x.3");
        assert_eq!(
            next("1.2.99999999999999999999999"),
            "1.2.99999999999999999999999"
        );
        // The last one counts, not the highest.
        assert_eq!(next_version(&list(&["0.2.0", "0.1.0"])), "0.1.1");
    }

    #[test]
    fn check_version_refuses_taken_and_unsafe_versions() {
        let releases = list(&["0.1.0"]);
        let long = "1".repeat(VERSION_MAX + 1);
        for bad in [
            "",
            "1.0 b",
            " 1.0",
            "1.0\t",
            "1/0",
            "1\\0",
            "a:b",
            "a*b",
            "a?b",
            "a\"b",
            "a<b",
            "a>b",
            "a|b",
            "a\nb",
            "a\u{a0}b",
            long.as_str(),
        ] {
            assert_eq!(
                check_version(bad, &releases),
                Err(BAD_VERSION.to_owned()),
                "{bad:?}"
            );
        }
        assert_eq!(
            check_version("0.1.0", &releases),
            Err(VERSION_TAKEN.to_owned())
        );
        assert_eq!(check_version("0.1.1", &releases), Ok(()));
        assert_eq!(check_version(&"1".repeat(VERSION_MAX), &releases), Ok(()));
        assert_eq!(check_version("2.0.0-beta+1_é", &releases), Ok(()));
        // The first release of an empty list can be anything valid.
        assert_eq!(check_version("0.1.0", &Releases::empty()), Ok(()));
    }

    #[test]
    fn a_broken_releases_file_is_refused_and_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join(RELEASES_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        let newer = "{\"version\": 2, \"releases\": [], \"future\": true}";
        let cases = [
            (newer, "Fassung 2 ist neuer"),
            ("{\"version\": 2, \"releases\": []}", "Fassung 2 ist neuer"),
            ("{", RELEASES_FILE),
            ("{\"version\": 1}", RELEASES_FILE),
            (
                "{\"version\": 1, \"releases\": [], \"x\": 1}",
                RELEASES_FILE,
            ),
            ("{\"version\": 0, \"releases\": []}", "unbekannte Fassung 0"),
        ];
        for (text, expected) in cases {
            std::fs::write(&path, text).unwrap();
            let err = read_releases(root).unwrap_err();
            let KernelError::Refused(message) = &err else {
                panic!("{err:?}");
            };
            assert!(message.starts_with(RELEASES_BROKEN), "{message}");
            assert!(message.contains(expected), "{text}: {message}");
            if expected == RELEASES_FILE {
                assert!(message.contains(&path.display().to_string()), "{message}");
            }
            let err = append_release(root, release("0.1.0")).unwrap_err();
            assert!(matches!(err, KernelError::Refused(_)), "{err:?}");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        }
    }

    #[test]
    fn release_file_name_uses_the_stem_and_the_version() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Mein Hack_DSPRE_contents");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("header.yaml"), "gamecode: CPUE\n").unwrap();
        let project = Project::open(&root).unwrap();
        assert_eq!(project_stem(&project), "Mein Hack");
        assert_eq!(
            release_file_name(&project, "0.1.0"),
            "Mein Hack-v0.1.0.xdelta"
        );
    }

    #[test]
    fn release_file_entry_is_relative_below_the_root() {
        let root = Path::new("/tmp/proj");
        assert_eq!(
            release_file_entry(root, &root.join("build").join("x.xdelta")),
            "build/x.xdelta"
        );
        assert_eq!(release_file_entry(root, &root.join("x.xdelta")), "x.xdelta");
        assert_eq!(
            release_file_entry(root, Path::new("/tmp/a/x.xdelta")),
            "/tmp/a/x.xdelta"
        );
        assert_eq!(
            release_file_entry(root, Path::new("/tmp/proj-other/x.xdelta")),
            "/tmp/proj-other/x.xdelta"
        );
    }
}
