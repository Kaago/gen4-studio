//! Publishing a release: the patch of the built ROM against the base ROM at a chosen place,
//! its notes with version, date and change notes, optionally `README.txt`, and the entry in
//! `.g4/releases.json`. See `docs/formats/xdelta_patch.md`, section „Veröffentlichen“.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::base::{BaseRom, other_rom};
use crate::error::{KernelError, io_at};
use crate::project::Project;
use crate::start::{BaseMark, now_rfc3339, read_base_mark};

use super::{
    PatchMeta, PatchReport, Release, ReleaseInfo, Releases, Run, append_release, apply_line,
    check_version, create_patch_with, decode, default_targets, elapsed_ms, meta, no_mark,
    notes_path, read_releases, release_file_entry, release_file_name, write_tmp,
};

/// What to publish and where.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishOptions {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    /// The folder; relative to the project root; `None`: `build/`.
    #[serde(default)]
    pub dir: Option<PathBuf>,
    /// The patch's file name; `.xdelta` is added when missing; `None`: `release_file_name`.
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub readme: bool,
    #[serde(default)]
    pub overwrite: bool,
}

/// The files a release writes, and the built ROM the patch produces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishTargets {
    pub dir: PathBuf,
    pub patch: PathBuf,
    pub notes: PathBuf,
    pub readme: Option<PathBuf>,
    /// `build/<patch_name>.nds`, wherever the patch goes.
    pub rom: PathBuf,
}

/// The phases of a release, in their order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishPhase {
    Build,
    Encode,
    Verify,
    Write,
    Record,
}

/// Sent when a phase starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PublishProgress {
    pub phase: PublishPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishedKind {
    Patch,
    Notes,
    Readme,
}

/// A file a release wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishedFile {
    pub kind: PublishedKind,
    pub path: PathBuf,
    pub size: u64,
}

/// What a release did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishReport {
    /// The entry appended to the list.
    pub release: Release,
    pub patch: PatchReport,
    /// Patch, notes, then `README.txt` when written.
    pub files: Vec<PublishedFile>,
    /// The list after the entry.
    pub releases: Releases,
    pub phase_ms: BTreeMap<PublishPhase, u64>,
    pub ms: u64,
}

/// A target exists and `overwrite` is false; follows `<file name> `.
pub fn exists() -> String {
    t!("patch.exists").into_owned()
}
/// The built ROM is the base ROM.
pub fn no_changes() -> String {
    t!("patch.no_changes").into_owned()
}
/// The patch's file name is empty or has a character a file name cannot.
pub fn bad_file() -> String {
    t!("patch.bad_file").into_owned()
}
/// What a release hands on, for the wizard's preview.
pub fn no_rom_content() -> String {
    t!("patch.no_rom_content").into_owned()
}

/// The folder of a release, or the first path above it that exists, is a file; follows
/// `: <path>`.
pub fn not_a_folder() -> String {
    t!("patch.not_a_folder").into_owned()
}

/// The patch's suffix.
const XDELTA: &str = ".xdelta";

/// The targets of `opts` (`Err(Refused(bad_file()))` for a bad name); creates nothing.
pub fn targets_for(
    project: &Project,
    opts: &PublishOptions,
) -> Result<PublishTargets, KernelError> {
    let dir = match &opts.dir {
        None => project.root.join("build"),
        Some(dir) => project.root.join(dir),
    };
    let name = match &opts.file {
        None => release_file_name(project, &opts.version),
        Some(file) => file_name(file)?,
    };
    let patch = dir.join(name);
    Ok(PublishTargets {
        notes: notes_path(&patch),
        readme: opts.readme.then(|| dir.join(README)),
        rom: default_targets(project).rom,
        patch,
        dir,
    })
}

const README: &str = "README.txt";

/// `file` with `.xdelta` (added when missing), or `Refused(bad_file())` for a name that is empty
/// (also before the suffix, after cutting spaces and dots) or has a character a file name
/// cannot.
fn file_name(file: &str) -> Result<String, KernelError> {
    let bad = |c: char| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    };
    let stem = file.strip_suffix(XDELTA).unwrap_or(file);
    if stem.trim_matches([' ', '.']).is_empty() || file.contains(bad) {
        return Err(KernelError::Refused(bad_file()));
    }
    Ok(format!("{stem}{XDELTA}"))
}

/// `Refused("<name> " + exists())` when the patch or its notes exist and `overwrite` is false.
/// `README.txt` never counts: it describes the newest release in its folder and is always
/// replaced.
pub fn check_targets(targets: &PublishTargets, overwrite: bool) -> Result<(), KernelError> {
    // The first existing path of the folder and those above it must be a folder.
    for ancestor in targets.dir.ancestors() {
        match std::fs::metadata(ancestor) {
            Ok(meta) if meta.is_dir() => break,
            Ok(_) => {
                return Err(KernelError::Refused(format!(
                    "{}: {}",
                    not_a_folder(),
                    ancestor.display()
                )));
            }
            Err(_) => {}
        }
    }
    if overwrite {
        return Ok(());
    }
    for file in [&targets.patch, &targets.notes] {
        if std::fs::symlink_metadata(file).is_ok() {
            let name = file.file_name().unwrap_or(file.as_os_str());
            return Err(KernelError::Refused(format!(
                "{} {}",
                name.to_string_lossy(),
                exists()
            )));
        }
    }
    Ok(())
}

/// Version and name checked (`check_version`, `targets_for`, `check_targets`), nothing
/// written; the app and MCP call it before asking or building.
pub fn check_publish(
    project: &Project,
    opts: &PublishOptions,
) -> Result<PublishTargets, KernelError> {
    let releases = read_releases(&project.root)?;
    check_version(&opts.version, &releases).map_err(KernelError::Refused)?;
    let targets = targets_for(project, opts)?;
    check_targets(&targets, opts.overwrite)?;
    Ok(targets)
}

/// `README.txt`: `{project} v{version} ({day})`, a blank line, the notes (or „No change
/// notes.“), a blank line, `Base ROM SHA-1: …` and the line `To apply: …` of the notes, in
/// the current language. `{project}` is `meta.name` without `-v{version}` at its end, `{day}`
/// the day of `release.date`.
pub fn readme_text(meta: &PatchMeta, release: &ReleaseInfo<'_>, base_sha1: &str) -> String {
    let suffix = format!("-v{}", release.version);
    let project = meta
        .name
        .strip_suffix(&suffix)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(&meta.name);
    let day = release.date.get(..10).unwrap_or(release.date);
    let notes = release.notes.trim_end();
    let no_notes;
    let notes = if notes.trim().is_empty() {
        no_notes = t!("patch.no_notes");
        no_notes.as_ref()
    } else {
        notes
    };
    format!(
        "{project} v{} ({day})\n\n{notes}\n\n{}\n{}",
        release.version,
        t!("patch.readme_base_sha1", base_sha1 = base_sha1),
        apply_line(&meta.name)
    )
}

/// Encode, verify, write (patch, notes, readme), record; `built_rom` is the ROM the app or
/// `build_and_publish` built.
pub fn publish_built(
    project: &Project,
    base: &BaseRom,
    built_rom: &Path,
    opts: &PublishOptions,
    progress: &mut dyn FnMut(PublishProgress),
) -> Result<PublishReport, KernelError> {
    let started = Instant::now();
    let targets = check_publish(project, opts)?;
    let mark = required_mark(project, base)?;

    let date = now_rfc3339();
    let info = ReleaseInfo {
        version: &opts.version,
        date: &date,
        notes: &opts.notes,
    };
    let stem = targets
        .patch
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let meta = PatchMeta {
        name: stem.strip_suffix(XDELTA).unwrap_or(&stem).to_owned(),
        ..meta(project, &mark)
    };

    let mut write_started = None;
    let report = create_patch_with(
        base,
        built_rom,
        &targets.patch,
        &meta,
        Some(&info),
        &decode,
        Run {
            on_phase: &mut |phase| {
                if phase == PublishPhase::Write {
                    write_started = Some(Instant::now());
                }
                progress(PublishProgress { phase });
            },
            refuse_unchanged: true,
        },
    )?;
    let mut files = vec![
        published(PublishedKind::Patch, &report.patch)?,
        published(PublishedKind::Notes, &report.notes)?,
    ];
    if let Some(readme) = &targets.readme {
        let text = readme_text(&meta, &info, base.sha1());
        let tmp = write_tmp(readme, text.as_bytes())?;
        if let Err(e) = std::fs::rename(&tmp, readme).map_err(io_at(readme)) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        files.push(published(PublishedKind::Readme, readme)?);
    }
    let write_ms = write_started.map_or(0, elapsed_ms);

    progress(PublishProgress {
        phase: PublishPhase::Record,
    });
    let record_started = Instant::now();
    let release = Release {
        version: opts.version.clone(),
        date: date.clone(),
        file: release_file_entry(&project.root, &targets.patch),
        notes: opts.notes.clone(),
        base_sha1: report.base_sha1.clone(),
        size: report.size,
    };
    let releases = append_release(&project.root, release.clone())?;
    let record_ms = elapsed_ms(record_started);

    let phase_ms = BTreeMap::from([
        (PublishPhase::Encode, report.encode_ms),
        (PublishPhase::Verify, report.verify_ms),
        (PublishPhase::Write, write_ms),
        (PublishPhase::Record, record_ms),
    ]);
    Ok(PublishReport {
        release,
        patch: report,
        files,
        releases,
        phase_ms,
        ms: elapsed_ms(started),
    })
}

/// The project's base mark, `Refused(no_mark())` without one, `Refused(other_rom …)` when
/// `base` is not its ROM.
fn required_mark(project: &Project, base: &BaseRom) -> Result<BaseMark, KernelError> {
    let mark = read_base_mark(&project.root).ok_or_else(|| KernelError::Refused(no_mark()))?;
    if !mark.sha1.eq_ignore_ascii_case(base.sha1()) {
        return Err(KernelError::Refused(other_rom(base.sha1(), &mark)));
    }
    Ok(mark)
}

fn published(kind: PublishedKind, path: &Path) -> Result<PublishedFile, KernelError> {
    let size = std::fs::metadata(path).map_err(io_at(path))?.len();
    Ok(PublishedFile {
        kind,
        path: path.to_path_buf(),
        size,
    })
}

/// `check_publish`, then `build::build` to `default_targets().rom` (`Build`), then
/// `publish_built`; without a mark or with another base `Refused` before anything is built.
pub fn build_and_publish(
    project: &Project,
    base: &BaseRom,
    opts: &PublishOptions,
    progress: &mut dyn FnMut(PublishProgress),
) -> Result<PublishReport, KernelError> {
    build_and_publish_with(project, base, opts, progress, &|project, out| {
        crate::build::build(project, out).map(|report| report.rom)
    })
}

/// The build of [`build_and_publish_with`]: project and output path in, the built ROM out.
pub(crate) type Build = dyn Fn(&Project, &Path) -> Result<PathBuf, KernelError>;

/// [`build_and_publish`] with the build replaced, for tests (the synthetic project does not
/// build to a ROM).
pub(crate) fn build_and_publish_with(
    project: &Project,
    base: &BaseRom,
    opts: &PublishOptions,
    progress: &mut dyn FnMut(PublishProgress),
    build: &Build,
) -> Result<PublishReport, KernelError> {
    let started = Instant::now();
    let targets = check_publish(project, opts)?;
    required_mark(project, base)?;

    progress(PublishProgress {
        phase: PublishPhase::Build,
    });
    let build_started = Instant::now();
    let built = build(project, &targets.rom)?;
    let build_ms = elapsed_ms(build_started);

    let mut report = publish_built(project, base, &built, opts, progress)?;
    report.phase_ms.insert(PublishPhase::Build, build_ms);
    report.ms = elapsed_ms(started);
    Ok(report)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::patch::{
        RELEASES_FILE, default_targets, next_version, notes_path, project_stem, read_releases,
        version_taken,
    };
    use crate::test_support::TextFixture;

    fn opts(version: &str) -> PublishOptions {
        PublishOptions {
            version: version.into(),
            notes: String::new(),
            dir: None,
            file: None,
            readme: false,
            overwrite: false,
        }
    }

    /// The synthetic ROM with one byte flipped and 300 bytes appended, as a build would
    /// write it.
    fn built_bytes(fx: &TextFixture) -> Vec<u8> {
        let mut bytes = std::fs::read(fx.rom()).unwrap();
        bytes[0x7000] ^= 0x55;
        bytes.extend((0..300u32).map(|i| (i % 13) as u8));
        bytes
    }

    /// A build that writes [`built_bytes`] to its output and counts its calls.
    fn fake_build(fx: &TextFixture) -> (Box<Build>, Rc<Cell<usize>>) {
        let bytes = built_bytes(fx);
        let calls = Rc::new(Cell::new(0usize));
        let counted = calls.clone();
        let build = move |_: &Project, out: &Path| {
            counted.set(counted.get() + 1);
            std::fs::create_dir_all(out.parent().unwrap()).unwrap();
            std::fs::write(out, &bytes).unwrap();
            Ok(out.to_path_buf())
        };
        (Box::new(build), calls)
    }

    fn publish(
        fx: &TextFixture,
        opts: &PublishOptions,
    ) -> (Result<PublishReport, KernelError>, Vec<PublishPhase>) {
        let (build, _) = fake_build(fx);
        let mut phases = Vec::new();
        let result = build_and_publish_with(
            &fx.project(),
            &fx.base(),
            opts,
            &mut |p| phases.push(p.phase),
            &build,
        );
        (result, phases)
    }

    fn sha1_hex(bytes: &[u8]) -> String {
        super::super::sha1_hex(bytes)
    }

    fn refused(err: &KernelError) -> &str {
        match err {
            KernelError::Refused(message) => message,
            other => panic!("not refused: {other:?}"),
        }
    }

    #[test]
    fn targets_default_to_build_and_the_release_name() {
        let fx = TextFixture::dspre();
        let project = fx.project();
        let root = project.root.clone();
        let stem = project_stem(&project);

        let targets = targets_for(&project, &opts("0.1.0")).unwrap();
        assert_eq!(targets.dir, root.join("build"));
        assert_eq!(
            targets.patch,
            root.join("build").join(format!("{stem}-v0.1.0.xdelta"))
        );
        assert!(
            targets.patch.to_string_lossy().ends_with("-v0.1.0.xdelta"),
            "{targets:?}"
        );
        assert_eq!(targets.notes, notes_path(&targets.patch));
        assert!(targets.notes.to_string_lossy().ends_with(".xdelta.txt"));
        assert_eq!(targets.readme, None);
        assert_eq!(targets.rom, default_targets(&project).rom);

        let with_readme = PublishOptions {
            readme: true,
            ..opts("0.1.0")
        };
        let targets = targets_for(&project, &with_readme).unwrap();
        assert_eq!(targets.readme, Some(root.join("build").join("README.txt")));

        let named = PublishOptions {
            file: Some("mein".into()),
            ..opts("0.1.0")
        };
        let targets = targets_for(&project, &named).unwrap();
        assert_eq!(targets.patch, root.join("build").join("mein.xdelta"));
        assert_eq!(targets.notes, root.join("build").join("mein.xdelta.txt"));
        // A name with the suffix keeps it once.
        let suffixed = PublishOptions {
            file: Some("mein.xdelta".into()),
            ..opts("0.1.0")
        };
        let targets = targets_for(&project, &suffixed).unwrap();
        assert_eq!(targets.patch, root.join("build").join("mein.xdelta"));

        let relative = PublishOptions {
            dir: Some("aus".into()),
            readme: true,
            ..opts("0.1.0")
        };
        let targets = targets_for(&project, &relative).unwrap();
        assert_eq!(targets.dir, root.join("aus"));
        assert_eq!(targets.readme, Some(root.join("aus").join("README.txt")));
        // The ROM stays in build/.
        assert_eq!(targets.rom, default_targets(&project).rom);
        // Nothing was created.
        assert!(!root.join("build").exists());
        assert!(!root.join("aus").exists());
    }

    #[test]
    fn a_bad_file_name_is_refused() {
        let fx = TextFixture::dspre();
        let project = fx.project();
        for bad in [
            "a/b", "", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b",
        ] {
            let named = PublishOptions {
                file: Some(bad.into()),
                ..opts("0.1.0")
            };
            let err = targets_for(&project, &named).unwrap_err();
            assert_eq!(refused(&err), bad_file(), "{bad:?}");
            let err = check_publish(&project, &named).unwrap_err();
            assert_eq!(refused(&err), bad_file(), "{bad:?}");
        }
        for bad in ["a\nb", "\u{1}", "  ", ".xdelta", " .xdelta"] {
            let named = PublishOptions {
                file: Some(bad.into()),
                ..opts("0.1.0")
            };
            let err = targets_for(&project, &named).unwrap_err();
            assert_eq!(refused(&err), bad_file(), "{bad:?}");
        }
        // Spaces and dots inside are fine.
        let fine = PublishOptions {
            file: Some("Mein Hack v1.0".into()),
            ..opts("0.1.0")
        };
        assert!(targets_for(&project, &fine).is_ok());
    }

    #[test]
    fn an_existing_target_is_refused_before_the_build() {
        let fx = TextFixture::dspre();
        let project = fx.project();
        let targets = targets_for(&project, &opts("0.1.0")).unwrap();
        std::fs::create_dir_all(&targets.dir).unwrap();
        std::fs::write(&targets.patch, b"old patch").unwrap();
        let name = targets
            .patch
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let err = check_publish(&project, &opts("0.1.0")).unwrap_err();
        let message = refused(&err);
        assert!(message.contains(&exists()), "{message}");
        assert_eq!(message, format!("{name} {}", exists()));

        // The real build: refused before it, no phase reported, nothing recorded.
        let mut phases = 0usize;
        let err = build_and_publish(&project, &fx.base(), &opts("0.1.0"), &mut |_| phases += 1)
            .unwrap_err();
        assert_eq!(refused(&err), format!("{name} {}", exists()));
        assert_eq!(phases, 0);
        assert!(!fx.root().join(RELEASES_FILE).exists());
        assert!(!targets.rom.exists());
        assert_eq!(std::fs::read(&targets.patch).unwrap(), b"old patch");

        // Only the notes count as well; README.txt never does (it is always replaced).
        std::fs::remove_file(&targets.patch).unwrap();
        std::fs::write(&targets.notes, b"old notes").unwrap();
        let err = check_publish(&project, &opts("0.1.0")).unwrap_err();
        let notes_name = targets.notes.file_name().unwrap().to_string_lossy();
        assert_eq!(refused(&err), format!("{notes_name} {}", exists()));
        std::fs::remove_file(&targets.notes).unwrap();
        std::fs::write(targets.dir.join("README.txt"), b"old readme").unwrap();
        assert!(check_publish(&project, &opts("0.1.0")).is_ok());
        let with_readme = PublishOptions {
            readme: true,
            ..opts("0.1.0")
        };
        assert!(check_publish(&project, &with_readme).is_ok());

        // With overwrite it runs through and replaces the files.
        std::fs::write(&targets.patch, b"old patch").unwrap();
        let replace = PublishOptions {
            readme: true,
            overwrite: true,
            ..opts("0.1.0")
        };
        let (report, phases) = publish(&fx, &replace);
        let report = report.unwrap();
        assert_eq!(phases.len(), 5);
        assert_ne!(std::fs::read(&targets.patch).unwrap(), b"old patch");
        assert_ne!(
            std::fs::read(targets.dir.join("README.txt")).unwrap(),
            b"old readme"
        );
        assert_eq!(report.releases.releases.len(), 1);
    }

    #[test]
    fn a_second_release_in_the_same_folder_replaces_readme() {
        let fx = TextFixture::dspre();
        let stem = project_stem(&fx.project());
        let first = PublishOptions {
            notes: "Erste Fassung".into(),
            readme: true,
            ..opts("0.1.0")
        };
        publish(&fx, &first).0.unwrap();
        let readme = fx.root().join("build").join("README.txt");
        let text = std::fs::read_to_string(&readme).unwrap();
        assert!(text.starts_with(&format!("{stem} v0.1.0 (")), "{text}");

        // Without overwrite: README.txt of the first release is no collision.
        let second = PublishOptions {
            notes: "Zweite Fassung".into(),
            readme: true,
            ..opts("0.1.1")
        };
        let report = publish(&fx, &second).0.unwrap();
        assert_eq!(report.releases.releases.len(), 2);
        let text = std::fs::read_to_string(&readme).unwrap();
        assert!(text.starts_with(&format!("{stem} v0.1.1 (")), "{text}");
        assert!(text.contains("Zweite Fassung\n"), "{text}");
        assert!(!text.contains("Erste Fassung"), "{text}");
        // Both patches stay.
        let build = fx.root().join("build");
        assert!(build.join(format!("{stem}-v0.1.0.xdelta")).is_file());
        assert!(build.join(format!("{stem}-v0.1.1.xdelta")).is_file());
    }

    #[test]
    fn publish_writes_patch_notes_readme_and_records_the_release() {
        let fx = TextFixture::dspre();
        let project = fx.project();
        let stem = project_stem(&project);
        let release_opts = PublishOptions {
            notes: "Neue Karte\nZweite Zeile".into(),
            readme: true,
            ..opts("0.1.0")
        };
        let (report, phases) = publish(&fx, &release_opts);
        let report = report.unwrap();

        // The phases in their order, each once, and all five measured.
        assert_eq!(
            phases,
            [
                PublishPhase::Build,
                PublishPhase::Encode,
                PublishPhase::Verify,
                PublishPhase::Write,
                PublishPhase::Record
            ]
        );
        let measured: Vec<PublishPhase> = report.phase_ms.keys().copied().collect();
        assert_eq!(measured, phases);
        assert!(report.ms >= report.phase_ms.values().copied().max().unwrap());

        // Three files, in order, with their sizes.
        let build = fx.root().join("build");
        let patch = build.join(format!("{stem}-v0.1.0.xdelta"));
        let kinds: Vec<PublishedKind> = report.files.iter().map(|f| f.kind).collect();
        assert_eq!(
            kinds,
            [
                PublishedKind::Patch,
                PublishedKind::Notes,
                PublishedKind::Readme
            ]
        );
        let paths: Vec<PathBuf> = report.files.iter().map(|f| f.path.clone()).collect();
        assert_eq!(
            paths,
            [patch.clone(), notes_path(&patch), build.join("README.txt")]
        );
        for file in &report.files {
            assert!(file.size > 0, "{file:?}");
            assert_eq!(std::fs::metadata(&file.path).unwrap().len(), file.size);
        }
        assert_eq!(report.patch.patch, patch);
        assert_eq!(report.patch.size, report.files[0].size);

        let notes = std::fs::read_to_string(notes_path(&patch)).unwrap();
        assert!(notes.contains("Version: 0.1.0\n"), "{notes}");
        assert!(notes.contains("Neue Karte\n"), "{notes}");
        assert!(notes.contains("Zweite Zeile\n"), "{notes}");
        // The app header and the notes carry the release's name.
        let bytes = std::fs::read(&patch).unwrap();
        let header = String::from_utf8(g4_xdelta::app_header(&bytes).unwrap().to_vec()).unwrap();
        assert!(
            header.starts_with(&format!("{stem}-v0.1.0.nds//")),
            "{header}"
        );
        assert!(
            notes.contains(&format!("{stem}-v0.1.0.xdelta {stem}-v0.1.0.nds")),
            "{notes}"
        );
        let readme = std::fs::read_to_string(build.join("README.txt")).unwrap();
        assert!(readme.starts_with(&format!("{stem} v0.1.0 (")), "{readme}");
        assert!(readme.contains("Neue Karte\nZweite Zeile\n"), "{readme}");

        // The list has the entry.
        let releases = read_releases(fx.root()).unwrap();
        assert_eq!(releases, report.releases);
        assert_eq!(releases.releases.len(), 1);
        let entry = &releases.releases[0];
        assert_eq!(entry, &report.release);
        assert_eq!(entry.version, "0.1.0");
        assert_eq!(entry.file, format!("build/{stem}-v0.1.0.xdelta"));
        assert_eq!(entry.size, report.files[0].size);
        assert_eq!(entry.base_sha1, fx.sha1());
        assert_eq!(entry.notes, "Neue Karte\nZweite Zeile");
        assert!(
            notes.contains(&format!("Date: {}\n", entry.date)),
            "{notes}"
        );

        // The patch applied to the base gives the built ROM.
        let built = built_bytes(&fx);
        assert_eq!(g4_xdelta::decode(fx.base().bytes(), &bytes).unwrap(), built);
        assert_eq!(report.patch.target_sha1, sha1_hex(&built));
        assert_eq!(report.patch.rom, default_targets(&project).rom);
        // No temp file is left.
        let tmp: Vec<_> = std::fs::read_dir(&build)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(tmp.is_empty(), "{tmp:?}");
    }

    #[test]
    fn a_second_release_proposes_the_next_version_and_refuses_the_taken_one() {
        let fx = TextFixture::dspre();
        publish(&fx, &opts("0.1.0")).0.unwrap();
        let releases = read_releases(fx.root()).unwrap();
        assert_eq!(next_version(&releases), "0.1.1");

        let (result, phases) = publish(&fx, &opts("0.1.0"));
        assert_eq!(refused(&result.unwrap_err()), version_taken());
        assert!(phases.is_empty(), "{phases:?}");
        let err = check_publish(&fx.project(), &opts("0.1.0")).unwrap_err();
        assert_eq!(refused(&err), version_taken());
        let err = check_publish(&fx.project(), &opts("1 0")).unwrap_err();
        assert_eq!(refused(&err), crate::patch::bad_version());

        let report = publish(&fx, &opts("0.1.1")).0.unwrap();
        assert_eq!(report.releases.releases.len(), 2);
        assert_eq!(next_version(&report.releases), "0.1.2");
    }

    #[test]
    fn no_difference_to_the_base_is_refused_and_writes_nothing() {
        let fx = TextFixture::dspre();
        let mut phases = Vec::new();
        let err = publish_built(
            &fx.project(),
            &fx.base(),
            fx.rom(),
            &opts("0.1.0"),
            &mut |p| phases.push(p.phase),
        )
        .unwrap_err();
        assert_eq!(refused(&err), no_changes());
        // Refused before encoding.
        assert!(phases.is_empty(), "{phases:?}");
        assert!(!fx.root().join("build").exists());
        assert!(!fx.root().join(RELEASES_FILE).exists());
    }

    #[test]
    fn an_unwritable_dir_writes_nothing_and_records_nothing() {
        let fx = TextFixture::dspre();
        let file = fx.root().join("eine-datei");
        std::fs::write(&file, b"x").unwrap();
        for dir in [file.clone(), file.join("darunter")] {
            let into_file = PublishOptions {
                dir: Some(dir.clone()),
                readme: true,
                ..opts("0.1.0")
            };
            let (result, _) = publish(&fx, &into_file);
            let err = result.unwrap_err();
            let message = err.to_string();
            assert!(
                message.contains(&file.display().to_string()),
                "{dir:?}: {message}"
            );
            assert!(!fx.root().join(RELEASES_FILE).exists());
            assert_eq!(std::fs::read(&file).unwrap(), b"x");
        }
        // No half patch next to the file either.
        let stray: Vec<_> = std::fs::read_dir(fx.root())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("xdelta") || n.ends_with(".tmp"))
            .collect();
        assert!(stray.is_empty(), "{stray:?}");

        // A second run into a valid folder succeeds.
        let report = publish(&fx, &opts("0.1.0")).0.unwrap();
        assert_eq!(report.releases.releases.len(), 1);
    }

    #[test]
    fn an_absolute_dir_outside_the_project_is_recorded_absolute() {
        let fx = TextFixture::dspre();
        let outside = tempfile::tempdir().unwrap();
        let dir = outside.path().join("ausgabe");
        let elsewhere = PublishOptions {
            dir: Some(dir.clone()),
            ..opts("0.1.0")
        };
        let report = publish(&fx, &elsewhere).0.unwrap();
        assert!(
            Path::new(&report.release.file).is_absolute(),
            "{}",
            report.release.file
        );
        assert_eq!(
            PathBuf::from(&report.release.file),
            dir.join(format!("{}-v0.1.0.xdelta", project_stem(&fx.project())))
        );
        assert!(Path::new(&report.release.file).is_file());
        assert_eq!(report.files.len(), 2);
        // The built ROM stays in the project.
        assert_eq!(report.patch.rom, default_targets(&fx.project()).rom);
    }

    #[test]
    fn readme_text_has_title_notes_sha1_and_the_instruction() {
        let meta = PatchMeta {
            name: "Mein Hack-v0.1.0".into(),
            game_code: "CPUE".into(),
            base_label: "POKEMON PL (CPUE)".into(),
            header_label: "POKEMON PL (CPUE)".into(),
            studio_version: "0.1.0".into(),
        };
        let sha1 = "d40f19ac24d67c721a4ab25e1c013154443d75b4";
        let info = ReleaseInfo {
            version: "0.1.0",
            date: "2026-10-31T10:00:00Z",
            notes: "Neue Karte\nZweite Zeile\n\n",
        };
        assert_eq!(
            readme_text(&meta, &info, sha1),
            "Mein Hack v0.1.0 (2026-10-31)\n\
             \n\
             Neue Karte\n\
             Zweite Zeile\n\
             \n\
             Base ROM SHA-1: d40f19ac24d67c721a4ab25e1c013154443d75b4\n\
             To apply: with xdelta3 (“xdelta3 -d -s <base ROM> Mein Hack-v0.1.0.xdelta \
             Mein Hack-v0.1.0.nds”) or with Rom Patcher JS, on exactly the base ROM with \
             this SHA-1.\n"
        );
        let empty = ReleaseInfo {
            notes: " \n",
            ..info
        };
        let text = readme_text(&meta, &empty, sha1);
        assert!(
            text.starts_with("Mein Hack v0.1.0 (2026-10-31)\n\nNo change notes.\n\n"),
            "{text}"
        );
        // A name of the user's own stays whole.
        let own = PatchMeta {
            name: "mein".into(),
            ..meta
        };
        assert!(readme_text(&own, &info, sha1).starts_with("mein v0.1.0 (2026-10-31)\n"));
    }

    #[test]
    fn a_failing_readme_keeps_patch_and_notes_and_records_nothing() {
        let fx = TextFixture::dspre();
        let project = fx.project();
        let targets = targets_for(&project, &opts("0.1.0")).unwrap();
        // README.txt is a folder: renaming the temp file onto it fails.
        let readme = targets.dir.join("README.txt");
        std::fs::create_dir_all(readme.join("inhalt")).unwrap();
        let replace = PublishOptions {
            readme: true,
            overwrite: true,
            ..opts("0.1.0")
        };
        let err = publish(&fx, &replace).0.unwrap_err();
        assert!(
            err.to_string().contains(&readme.display().to_string()),
            "{err}"
        );
        assert!(targets.patch.is_file());
        assert!(targets.notes.is_file());
        assert!(!fx.root().join(RELEASES_FILE).exists());
        assert!(!targets.dir.join(".README.txt.tmp").exists());
    }

    #[test]
    fn publish_built_needs_the_marks_base_rom() {
        let fx = TextFixture::dspre();
        let built = fx.root().join("built.nds");
        std::fs::write(&built, built_bytes(&fx)).unwrap();
        fx.write_mark(&"cd".repeat(20));
        let mut phases = 0usize;
        let err = publish_built(
            &fx.project(),
            &fx.base(),
            &built,
            &opts("0.1.0"),
            &mut |_| phases += 1,
        )
        .unwrap_err();
        let mark = read_base_mark(fx.root()).unwrap();
        assert_eq!(refused(&err), other_rom(fx.sha1(), &mark));
        assert_eq!(phases, 0);
        assert!(!fx.root().join("build").exists());
        let (build, calls) = fake_build(&fx);
        let err = build_and_publish_with(
            &fx.project(),
            &fx.base(),
            &opts("0.1.0"),
            &mut |_| phases += 1,
            &build,
        )
        .unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err:?}");
        assert_eq!(calls.get(), 0);
        assert_eq!(phases, 0);

        std::fs::remove_file(fx.root().join(".g4/base.json")).unwrap();
        let err = build_and_publish_with(
            &fx.project(),
            &fx.base(),
            &opts("0.1.0"),
            &mut |_| phases += 1,
            &build,
        )
        .unwrap_err();
        assert_eq!(refused(&err), crate::patch::no_mark());
        assert_eq!(calls.get(), 0);
        assert_eq!(phases, 0);
        assert!(!fx.root().join(RELEASES_FILE).exists());
    }

    #[test]
    fn options_read_from_json_with_defaults() {
        let parsed: PublishOptions = serde_json::from_str(r#"{"version":"0.1.0"}"#).unwrap();
        assert_eq!(parsed, opts("0.1.0"));
        let parsed: PublishOptions = serde_json::from_str(
            r#"{"version":"1","notes":"n","dir":"aus","file":"f","readme":true,"overwrite":true}"#,
        )
        .unwrap();
        assert_eq!(parsed.dir, Some(PathBuf::from("aus")));
        assert!(parsed.readme && parsed.overwrite);
        assert!(serde_json::from_str::<PublishOptions>(r#"{"version":"1","x":1}"#).is_err());
        // Phases are snake_case keys.
        let mut ms = BTreeMap::new();
        ms.insert(PublishPhase::Encode, 3u64);
        assert_eq!(serde_json::to_string(&ms).unwrap(), r#"{"encode":3}"#);
    }
}
