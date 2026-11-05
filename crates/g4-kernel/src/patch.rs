//! An xdelta patch of the built ROM against the base ROM: VCDIFF without secondary
//! compression, with Adler32 and an application header naming both ROMs, a German notes
//! file next to it, and a self-check before anything is written. See
//! `docs/formats/xdelta_patch.md`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;
use sha1::{Digest, Sha1};

use crate::base::{BaseRom, base_label, other_rom};
use crate::build::BuildReport;
use crate::error::{KernelError, io_at};
use crate::project::Project;
use crate::start::{BaseMark, read_base_mark};

pub mod publish;
pub mod release;

pub use publish::{
    BAD_FILE, EXISTS, NO_CHANGES, NO_ROM_CONTENT, NOT_A_FOLDER, PublishOptions, PublishPhase,
    PublishProgress, PublishReport, PublishTargets, PublishedFile, PublishedKind,
    build_and_publish, check_publish, check_targets, publish_built, readme_text, targets_for,
};
pub use release::{
    BAD_VERSION, FIRST_VERSION, RELEASES_BROKEN, RELEASES_FILE, RELEASES_VERSION, Release,
    Releases, VERSION_MAX, VERSION_TAKEN, append_release, check_version, next_version,
    read_releases, release_file_entry, release_file_name,
};

/// The patch's name, its target ROM and the base it applies to, for the app header and the
/// notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchMeta {
    /// `<Projekt>-<Spielcode>`, see [`patch_name`].
    pub name: String,
    pub game_code: String,
    /// [`crate::base::base_label`] of the mark, as it is (the app header cleans it).
    pub base_label: String,
    pub studio_version: String,
}

/// Where the export writes: `build/<Name>.nds` and `build/<Name>.xdelta` in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchTargets {
    pub rom: PathBuf,
    pub patch: PathBuf,
}

/// What [`create_patch`] wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PatchReport {
    pub patch: PathBuf,
    pub notes: PathBuf,
    /// The built ROM the patch produces.
    pub rom: PathBuf,
    /// Size of the patch file in bytes.
    pub size: u64,
    pub base_sha1: String,
    pub target_sha1: String,
    pub encode_ms: u64,
    pub verify_ms: u64,
}

/// The self-check decoded something other than the built ROM; nothing was written.
pub const SELF_CHECK_FAILED: &str = "Selbstprüfung fehlgeschlagen: der Patch ergibt auf der Basis-ROM nicht die gebaute ROM; nichts geschrieben.";

/// A project without a base mark: the refusal of `build_and_create`, the app, MCP and `g4k`.
pub const NO_MARK: &str = "Diesem Projekt ist keine Basis-ROM zugeordnet.";

/// `<Projekt>-<Spielcode>`: the folder name without `_DSPRE_contents`, cleaned to a file name
/// (see [`clean`]), `Projekt` if nothing is left, and the project's game code.
pub fn patch_name(project: &Project) -> String {
    let folder = project
        .root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    name_for(&folder, &project.game_code)
}

fn name_for(folder: &str, game_code: &str) -> String {
    format!("{}-{game_code}", stem_of(folder))
}

/// The folder name without `_DSPRE_contents`, cleaned as [`patch_name`] cleans it (`Projekt`
/// if nothing is left), without the game code.
pub fn project_stem(project: &Project) -> String {
    let folder = project
        .root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    stem_of(&folder)
}

fn stem_of(folder: &str) -> String {
    let stem = folder.strip_suffix(DSPRE_SUFFIX).unwrap_or(folder);
    clean(stem, "Projekt")
}

const DSPRE_SUFFIX: &str = "_DSPRE_contents";

/// `/ \ : * ? " < > |` and control characters become `_`, spaces and dots at both ends go;
/// `fallback` if nothing is left. Keeps names valid as file names on every platform and keeps
/// `/` out of the app header, where it separates the fields.
fn clean(name: &str, fallback: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = replaced.trim_matches([' ', '.']);
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `build/<Name>.nds` and `build/<Name>.xdelta` in the project folder.
pub fn default_targets(project: &Project) -> PatchTargets {
    let name = patch_name(project);
    let dir = project.root.join("build");
    PatchTargets {
        rom: dir.join(format!("{name}.nds")),
        patch: dir.join(format!("{name}.xdelta")),
    }
}

/// The patch's metadata for `project` with base mark `base`; `studio_version` is this
/// kernel's version.
pub fn meta(project: &Project, base: &BaseMark) -> PatchMeta {
    PatchMeta {
        name: patch_name(project),
        game_code: project.game_code.clone(),
        base_label: base_label(base),
        studio_version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// The notes file next to a patch: `<Name>.xdelta` → `<Name>.xdelta.txt`.
pub fn notes_path(patch: &Path) -> PathBuf {
    let mut path = patch.as_os_str().to_owned();
    path.push(".txt");
    PathBuf::from(path)
}

/// The VCDIFF application header in xdelta's form `target/target-compression/source/
/// source-compression/` with empty compression fields: `<Name>.nds//<label> [<sha1>].nds/`.
pub fn app_header(meta: &PatchMeta, base_sha1: &str) -> String {
    format!(
        "{}.nds//{} [{base_sha1}].nds/",
        meta.name,
        clean(&meta.base_label, "Basis-ROM")
    )
}

/// The German notes file `<Name>.xdelta.txt`, lines ending in `\n`.
pub fn notes(meta: &PatchMeta, base_sha1: &str, target_sha1: &str) -> String {
    notes_with(meta, base_sha1, target_sha1, None)
}

/// What a release adds to the notes: the version and date of the list's entry and the
/// change notes the user typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseInfo<'a> {
    pub version: &'a str,
    /// `YYYY-MM-DDTHH:MM:SSZ`, UTC.
    pub date: &'a str,
    pub notes: &'a str,
}

/// [`notes`] plus `Version:` and `Datum:` after `Studio-Version:` and, at the end, the
/// paragraph `Änderungen:` (only when the release has notes, which stand as typed with
/// their trailing white space cut, followed by exactly one `\n`); `None` gives [`notes`].
pub fn notes_with(
    meta: &PatchMeta,
    base_sha1: &str,
    target_sha1: &str,
    release: Option<&ReleaseInfo<'_>>,
) -> String {
    let PatchMeta {
        name,
        game_code,
        base_label,
        studio_version,
    } = meta;
    let mut text = format!(
        "Gen4 Studio: Patch für {base_label}\n\
         Spiel: {game_code}\n\
         Basis-ROM SHA-1: {base_sha1}\n\
         Ziel-ROM SHA-1: {target_sha1}\n\
         Studio-Version: {studio_version}\n"
    );
    if let Some(release) = release {
        text.push_str(&format!(
            "Version: {}\nDatum: {}\n",
            release.version, release.date
        ));
    }
    text.push('\n');
    text.push_str(&apply_line(name));
    if let Some(release) = release {
        let changes = release.notes.trim_end();
        if !changes.trim().is_empty() {
            text.push_str(&format!("\nÄnderungen:\n{changes}\n"));
        }
    }
    text
}

/// The paragraph `Anwenden: …` of the notes and `README.txt` for the patch `<name>.xdelta`,
/// ending in `\n`.
pub(crate) fn apply_line(name: &str) -> String {
    format!(
        "Anwenden: mit xdelta3 („xdelta3 -d -s <Basis-ROM> {name}.xdelta {name}.nds“) oder mit \
         Rom Patcher JS, auf genau die Basis-ROM mit dieser SHA-1.\n"
    )
}

/// Writes a patch from `base` to the ROM at `built_rom` to `out` and its notes next to it.
/// The patch is decoded again before anything is written; if that does not give the built
/// ROM (by SHA-1), the result is `Refused(SELF_CHECK_FAILED)` (shown as it is, without a
/// prefix) and an old patch stays as it was.
pub fn create_patch(
    base: &BaseRom,
    built_rom: &Path,
    out: &Path,
    meta: &PatchMeta,
) -> Result<PatchReport, KernelError> {
    create_patch_with_notes(base, built_rom, out, meta, None)
}

/// [`create_patch`] with the notes of [`notes_with`].
pub fn create_patch_with_notes(
    base: &BaseRom,
    built_rom: &Path,
    out: &Path,
    meta: &PatchMeta,
    release: Option<&ReleaseInfo<'_>>,
) -> Result<PatchReport, KernelError> {
    create_patch_with(
        base,
        built_rom,
        out,
        meta,
        release,
        &decode,
        Run {
            on_phase: &mut |_| {},
            refuse_unchanged: false,
        },
    )
}

/// The decoder of the self-check: xdelta's, a failure is [`SELF_CHECK_FAILED`].
pub(crate) fn decode(source: &[u8], patch: &[u8]) -> Result<Vec<u8>, KernelError> {
    g4_xdelta::decode(source, patch).map_err(|_| KernelError::Refused(SELF_CHECK_FAILED.to_owned()))
}

/// The decoder of the self-check: source and patch in, the decoded target out.
pub(crate) type Verify = dyn Fn(&[u8], &[u8]) -> Result<Vec<u8>, KernelError>;

/// What a release adds to a patch run: the phases `Encode`, `Verify` and `Write` reported as
/// they start, and the refusal of a built ROM equal to the base.
pub(crate) struct Run<'a> {
    pub(crate) on_phase: &'a mut dyn FnMut(PublishPhase),
    /// `Refused(publish::NO_CHANGES)` before encoding when the built ROM is the base ROM.
    pub(crate) refuse_unchanged: bool,
}

/// [`create_patch`] with the decoder of the self-check replaced, for tests.
pub(crate) fn create_patch_with(
    base: &BaseRom,
    built_rom: &Path,
    out: &Path,
    meta: &PatchMeta,
    release: Option<&ReleaseInfo<'_>>,
    verify: &Verify,
    run: Run<'_>,
) -> Result<PatchReport, KernelError> {
    let Run {
        on_phase,
        refuse_unchanged,
    } = run;
    let target = std::fs::read(built_rom).map_err(io_at(built_rom))?;
    let target_sha1 = sha1_hex(&target);
    if refuse_unchanged && target_sha1.eq_ignore_ascii_case(base.sha1()) {
        return Err(KernelError::Refused(publish::NO_CHANGES.to_owned()));
    }
    let header = app_header(meta, base.sha1());

    on_phase(PublishPhase::Encode);
    let started = Instant::now();
    let patch = g4_xdelta::encode(base.bytes(), &target, header.as_bytes())
        .map_err(|e| KernelError::Refused(format!("Patch nicht erstellt: {e}")))?;
    let encode_ms = elapsed_ms(started);
    drop(target);

    on_phase(PublishPhase::Verify);
    let started = Instant::now();
    let decoded = verify(base.bytes(), &patch)?;
    if sha1_hex(&decoded) != target_sha1 {
        return Err(KernelError::Refused(SELF_CHECK_FAILED.to_owned()));
    }
    let verify_ms = elapsed_ms(started);
    drop(decoded);

    on_phase(PublishPhase::Write);
    let notes_file = notes_path(out);
    let text = notes_with(meta, base.sha1(), &target_sha1, release);
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(io_at(parent))?;
    }
    // Both temp files first, then the patch, then the notes: a write error leaves the old
    // pair; only a failing second rename leaves a new patch beside the old notes.
    let patch_tmp = write_tmp(out, &patch)?;
    let notes_tmp = match write_tmp(&notes_file, text.as_bytes()) {
        Ok(tmp) => tmp,
        Err(e) => {
            let _ = std::fs::remove_file(&patch_tmp);
            return Err(e);
        }
    };
    if let Err(e) = std::fs::rename(&patch_tmp, out).map_err(io_at(out)) {
        let _ = std::fs::remove_file(&patch_tmp);
        let _ = std::fs::remove_file(&notes_tmp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&notes_tmp, &notes_file).map_err(io_at(&notes_file)) {
        let _ = std::fs::remove_file(&notes_tmp);
        return Err(e);
    }

    Ok(PatchReport {
        patch: out.to_path_buf(),
        notes: notes_file,
        rom: built_rom.to_path_buf(),
        size: patch.len() as u64,
        base_sha1: base.sha1().to_owned(),
        target_sha1,
        encode_ms,
        verify_ms,
    })
}

/// Writes `bytes` to `.<file name>.tmp` next to `path` and returns that path; removed again
/// on an error.
pub(crate) fn write_tmp(path: &Path, bytes: &[u8]) -> Result<PathBuf, KernelError> {
    let name = path
        .file_name()
        .ok_or_else(|| KernelError::Refused(format!("{}: kein Dateiname", path.display())))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let written = std::fs::File::create(&tmp).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(io_at(&tmp)(e));
    }
    Ok(tmp)
}

pub(crate) fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// For g4k and the gated tests: builds to `default_targets().rom`, then `create_patch` to
/// `out` or the default. Without a base mark, or with a `base` that is not the mark's ROM, it
/// is `Refused` before anything is built.
pub fn build_and_create(
    project: &Project,
    base: &BaseRom,
    out: Option<&Path>,
) -> Result<(BuildReport, PatchReport), KernelError> {
    let mark =
        read_base_mark(&project.root).ok_or_else(|| KernelError::Refused(NO_MARK.to_owned()))?;
    if !mark.sha1.eq_ignore_ascii_case(base.sha1()) {
        return Err(KernelError::Refused(other_rom(base.sha1(), &mark)));
    }
    let targets = default_targets(project);
    let build = crate::build::build(project, &targets.rom)?;
    let report = create_patch(
        base,
        &build.rom,
        out.unwrap_or(&targets.patch),
        &meta(project, &mark),
    )?;
    Ok((build, report))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sha1::{Digest, Sha1};

    use super::*;

    const ROM_LEN: usize = 0x2_0000;

    fn sha1_hex(bytes: &[u8]) -> String {
        Sha1::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// A Platinum (CPUE) header and pseudo-random content; `BaseRom::open` reads no more.
    fn base_bytes() -> Vec<u8> {
        let mut bytes = vec![0u8; ROM_LEN];
        let mut x: u32 = 0x1234_5678;
        for b in bytes.iter_mut() {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *b = x as u8;
        }
        bytes[0x00..0x0C].copy_from_slice(b"POKEMON PL\0\0");
        bytes[0x0C..0x10].copy_from_slice(b"CPUE");
        bytes
    }

    /// The base with one byte flipped, 600 bytes inserted in the middle (everything after
    /// it shifts) and `variant` at the end, as a build after a map, a script and a text edit.
    fn built_bytes(variant: u8) -> Vec<u8> {
        let mut bytes = base_bytes();
        bytes[0x8020] ^= 0x80;
        let insert: Vec<u8> = (0..600u32).map(|i| (i % 7) as u8 + variant).collect();
        bytes.splice(0x1_0000..0x1_0000, insert);
        let end = bytes.len() - 1;
        bytes[end] = variant;
        bytes
    }

    struct Fixture {
        _tmp: tempfile::TempDir,
        dir: PathBuf,
        base: BaseRom,
        built: PathBuf,
        built_bytes: Vec<u8>,
    }

    fn fixture(variant: u8) -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let base_path = dir.join("base.nds");
        std::fs::write(&base_path, base_bytes()).unwrap();
        let base = BaseRom::open(&base_path, None).unwrap();
        let built = dir.join("built.nds");
        let built_bytes = built_bytes(variant);
        std::fs::write(&built, &built_bytes).unwrap();
        Fixture {
            _tmp: tmp,
            dir,
            base,
            built,
            built_bytes,
        }
    }

    fn test_meta() -> PatchMeta {
        PatchMeta {
            name: "Mein Hack-CPUE".into(),
            game_code: "CPUE".into(),
            base_label: "Platin/DE: Basis".into(),
            studio_version: "0.1.0".into(),
        }
    }

    fn tmp_files(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect()
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

    /// A folder `<tmp>/<folder>` that `Project::open` accepts (only `header.yaml`).
    fn minimal_project(folder: &str) -> (tempfile::TempDir, Project) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join(folder);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("header.yaml"), "gamecode: CPUE\n").unwrap();
        let project = Project::open(&root).unwrap();
        (tmp, project)
    }

    #[test]
    fn patch_name_strips_the_dspre_suffix_and_unsafe_characters() {
        assert_eq!(
            name_for("Mein/Hack:Neu_DSPRE_contents", "CPUE"),
            "Mein_Hack_Neu-CPUE"
        );
        assert_eq!(name_for(" .Rom. ", "CPUE"), "Rom-CPUE");
        assert_eq!(name_for("...", "CPUE"), "Projekt-CPUE");
        assert_eq!(name_for("", "CPUE"), "Projekt-CPUE");
        assert_eq!(
            name_for("Pokemon - Platin-Edition (Germany)_DSPRE_contents", "CPUE"),
            "Pokemon - Platin-Edition (Germany)-CPUE"
        );
        assert_eq!(
            name_for("a\\b*c?d\"e<f>g|h\u{1}i\u{7f}j", "IPKD"),
            "a_b_c_d_e_f_g_h_i_j-IPKD"
        );
        // Only the suffix goes, not the same text inside the name.
        assert_eq!(
            name_for("A_DSPRE_contents B_DSPRE_contents", "CPUE"),
            "A_DSPRE_contents B-CPUE"
        );

        let (_tmp, project) = minimal_project("Mein Hack_DSPRE_contents");
        assert_eq!(patch_name(&project), "Mein Hack-CPUE");
        let targets = default_targets(&project);
        assert_eq!(targets.rom, project.root.join("build/Mein Hack-CPUE.nds"));
        assert_eq!(
            targets.patch,
            project.root.join("build/Mein Hack-CPUE.xdelta")
        );
        assert_eq!(
            notes_path(&targets.patch),
            project.root.join("build/Mein Hack-CPUE.xdelta.txt")
        );

        let meta = meta(&project, &mark(&"ab".repeat(20)));
        assert_eq!(
            meta,
            PatchMeta {
                name: "Mein Hack-CPUE".into(),
                game_code: "CPUE".into(),
                base_label: "POKEMON PL (CPUE)".into(),
                studio_version: env!("CARGO_PKG_VERSION").into(),
            }
        );
    }

    #[test]
    fn app_header_names_target_and_base_with_the_full_sha1() {
        let sha1 = "d40f19ac24d67c721a4ab25e1c013154443d75b4";
        assert_eq!(
            app_header(&test_meta(), sha1),
            "Mein Hack-CPUE.nds//Platin_DE_ Basis [d40f19ac24d67c721a4ab25e1c013154443d75b4].nds/"
        );
    }

    #[test]
    fn notes_carry_both_sha1_and_the_instruction() {
        let base = "d40f19ac24d67c721a4ab25e1c013154443d75b4";
        let target = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            notes(&test_meta(), base, target),
            "Gen4 Studio: Patch für Platin/DE: Basis\n\
             Spiel: CPUE\n\
             Basis-ROM SHA-1: d40f19ac24d67c721a4ab25e1c013154443d75b4\n\
             Ziel-ROM SHA-1: 0123456789abcdef0123456789abcdef01234567\n\
             Studio-Version: 0.1.0\n\
             \n\
             Anwenden: mit xdelta3 („xdelta3 -d -s <Basis-ROM> Mein Hack-CPUE.xdelta \
             Mein Hack-CPUE.nds“) oder mit Rom Patcher JS, auf genau die Basis-ROM mit \
             dieser SHA-1.\n"
        );
    }

    #[test]
    fn create_patch_writes_patch_and_notes_that_apply() {
        let f = fixture(1);
        let out = f.dir.join("build").join("Mein Hack-CPUE.xdelta");
        let meta = test_meta();
        let report = create_patch(&f.base, &f.built, &out, &meta).unwrap();

        let patch = std::fs::read(&out).unwrap();
        assert_eq!(
            g4_xdelta::decode(f.base.bytes(), &patch).unwrap(),
            f.built_bytes
        );
        assert_eq!(report.size, patch.len() as u64);
        assert!(report.size < 4096, "{} bytes", report.size);
        assert_eq!(
            g4_xdelta::app_header(&patch),
            Some(app_header(&meta, f.base.sha1()).as_bytes())
        );
        assert!(!g4_xdelta::has_secondary_compression(&patch));

        let target_sha1 = sha1_hex(&f.built_bytes);
        assert_eq!(
            report,
            PatchReport {
                patch: out.clone(),
                notes: notes_path(&out),
                rom: f.built.clone(),
                size: patch.len() as u64,
                base_sha1: f.base.sha1().to_owned(),
                target_sha1: target_sha1.clone(),
                encode_ms: report.encode_ms,
                verify_ms: report.verify_ms,
            }
        );
        assert_eq!(
            std::fs::read_to_string(notes_path(&out)).unwrap(),
            notes(&meta, f.base.sha1(), &target_sha1)
        );
        assert!(tmp_files(&f.dir.join("build")).is_empty());
    }

    #[test]
    fn a_failed_self_check_writes_nothing() {
        let f = fixture(1);
        let out = f.dir.join("Mein Hack-CPUE.xdelta");
        let old_patch = b"old patch".to_vec();
        let old_notes = b"old notes".to_vec();
        std::fs::write(&out, &old_patch).unwrap();
        std::fs::write(notes_path(&out), &old_notes).unwrap();

        let wrong = |source: &[u8], _patch: &[u8]| -> Result<Vec<u8>, KernelError> {
            let mut bytes = source.to_vec();
            bytes[0x100] ^= 1;
            Ok(bytes)
        };
        let err = create_patch_with(
            &f.base,
            &f.built,
            &out,
            &test_meta(),
            None,
            &wrong,
            Run {
                on_phase: &mut |_| {},
                refuse_unchanged: false,
            },
        )
        .unwrap_err();
        assert!(
            matches!(&err, KernelError::Refused(m) if m == SELF_CHECK_FAILED),
            "{err:?}"
        );
        assert_eq!(err.to_string(), SELF_CHECK_FAILED);
        // A verify that cannot decode at all fails the same way.
        let broken = |_: &[u8], _: &[u8]| -> Result<Vec<u8>, KernelError> {
            Err(KernelError::Refused(SELF_CHECK_FAILED.to_owned()))
        };
        let err = create_patch_with(
            &f.base,
            &f.built,
            &out,
            &test_meta(),
            None,
            &broken,
            Run {
                on_phase: &mut |_| {},
                refuse_unchanged: false,
            },
        )
        .unwrap_err();
        assert!(
            matches!(&err, KernelError::Refused(m) if m == SELF_CHECK_FAILED),
            "{err:?}"
        );
        assert_eq!(err.to_string(), SELF_CHECK_FAILED);

        assert_eq!(std::fs::read(&out).unwrap(), old_patch);
        assert_eq!(std::fs::read(notes_path(&out)).unwrap(), old_notes);
        assert!(tmp_files(&f.dir).is_empty(), "{:?}", tmp_files(&f.dir));
    }

    #[test]
    fn create_patch_replaces_an_old_patch() {
        let f = fixture(1);
        let out = f.dir.join("Mein Hack-CPUE.xdelta");
        let first = create_patch(&f.base, &f.built, &out, &test_meta()).unwrap();
        let first_bytes = std::fs::read(&out).unwrap();

        let second_built = built_bytes(9);
        std::fs::write(&f.built, &second_built).unwrap();
        let second = create_patch(&f.base, &f.built, &out, &test_meta()).unwrap();
        let second_bytes = std::fs::read(&out).unwrap();

        assert_ne!(first_bytes, second_bytes);
        assert_ne!(first.target_sha1, second.target_sha1);
        assert_eq!(
            g4_xdelta::decode(f.base.bytes(), &second_bytes).unwrap(),
            second_built
        );
        assert_eq!(
            std::fs::read_to_string(notes_path(&out)).unwrap(),
            notes(&test_meta(), f.base.sha1(), &second.target_sha1)
        );
        assert!(tmp_files(&f.dir).is_empty(), "{:?}", tmp_files(&f.dir));
    }

    #[test]
    fn build_and_create_needs_the_marks_base_rom() {
        let f = fixture(1);
        let (_tmp, project) = minimal_project("X_DSPRE_contents");
        let err = build_and_create(&project, &f.base, None).unwrap_err();
        assert!(
            matches!(&err, KernelError::Refused(m) if m == NO_MARK),
            "{err:?}"
        );

        // A mark of another ROM: refused with the sentence of `base::other_rom`, no build.
        let other = mark(&"cd".repeat(20));
        let dir = project.root.join(".g4");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("base.json"),
            serde_json::to_string(&other).unwrap(),
        )
        .unwrap();
        let err = build_and_create(&project, &f.base, None).unwrap_err();
        let expected = crate::base::other_rom(f.base.sha1(), &other);
        assert!(
            matches!(&err, KernelError::Refused(m) if *m == expected),
            "{err:?}"
        );
        assert!(!project.root.join("build").exists());
    }

    const BASE_SHA1: &str = "d40f19ac24d67c721a4ab25e1c013154443d75b4";
    const TARGET_SHA1: &str = "0123456789abcdef0123456789abcdef01234567";

    fn release_info(notes: &str) -> ReleaseInfo<'_> {
        ReleaseInfo {
            version: "0.1.0",
            date: "2026-10-31T10:00:00Z",
            notes,
        }
    }

    #[test]
    fn notes_with_adds_version_date_and_changes_after_the_old_lines() {
        let meta = test_meta();
        let plain = notes(&meta, BASE_SHA1, TARGET_SHA1);
        let info = release_info("Neue Karte\nZweite Zeile");
        let with = notes_with(&meta, BASE_SHA1, TARGET_SHA1, Some(&info));
        assert_eq!(
            with,
            "Gen4 Studio: Patch für Platin/DE: Basis\n\
             Spiel: CPUE\n\
             Basis-ROM SHA-1: d40f19ac24d67c721a4ab25e1c013154443d75b4\n\
             Ziel-ROM SHA-1: 0123456789abcdef0123456789abcdef01234567\n\
             Studio-Version: 0.1.0\n\
             Version: 0.1.0\n\
             Datum: 2026-10-31T10:00:00Z\n\
             \n\
             Anwenden: mit xdelta3 („xdelta3 -d -s <Basis-ROM> Mein Hack-CPUE.xdelta \
             Mein Hack-CPUE.nds“) oder mit Rom Patcher JS, auf genau die Basis-ROM mit \
             dieser SHA-1.\n\
             \n\
             Änderungen:\n\
             Neue Karte\n\
             Zweite Zeile\n"
        );
        // The first five lines and the paragraph of 4.1a are the same bytes.
        let old: Vec<&str> = plain.split_inclusive('\n').collect();
        let new: Vec<&str> = with.split_inclusive('\n').collect();
        assert_eq!(old[..5], new[..5]);
        assert_eq!(old[5..], new[7..9]);
        // Notes as typed; trailing blank lines give exactly one `\n`.
        let typed = release_info("  Eingerückt\r\n\nZeile  \n\n\n");
        let with = notes_with(&meta, BASE_SHA1, TARGET_SHA1, Some(&typed));
        assert!(
            with.ends_with("Änderungen:\n  Eingerückt\r\n\nZeile\n"),
            "{with:?}"
        );
    }

    #[test]
    fn notes_without_a_release_are_unchanged() {
        let meta = test_meta();
        assert_eq!(
            notes_with(&meta, BASE_SHA1, TARGET_SHA1, None),
            notes(&meta, BASE_SHA1, TARGET_SHA1)
        );
        let plain = notes(&meta, BASE_SHA1, TARGET_SHA1);
        for missing in ["\nVersion:", "\nDatum:", "Änderungen"] {
            assert!(!plain.contains(missing), "{missing}");
        }
    }

    #[test]
    fn notes_without_text_have_no_changes_block() {
        let meta = test_meta();
        let plain = notes(&meta, BASE_SHA1, TARGET_SHA1);
        for empty in ["", " \n\t\n"] {
            let info = release_info(empty);
            let with = notes_with(&meta, BASE_SHA1, TARGET_SHA1, Some(&info));
            assert!(with.contains("Version: 0.1.0\nDatum: 2026-10-31T10:00:00Z\n"));
            assert!(!with.contains("Änderungen"), "{with}");
            assert!(with.ends_with("dieser SHA-1.\n"), "{with}");
            assert_eq!(with.lines().count(), plain.lines().count() + 2);
        }
    }

    #[test]
    fn create_patch_with_notes_writes_the_extended_notes() {
        let f = fixture(1);
        let out = f.dir.join("build").join("Mein Hack-v0.1.0.xdelta");
        let meta = test_meta();
        let info = release_info("Neue Karte");
        let report = create_patch_with_notes(&f.base, &f.built, &out, &meta, Some(&info)).unwrap();

        let text = std::fs::read_to_string(notes_path(&out)).unwrap();
        assert_eq!(
            text,
            notes_with(&meta, f.base.sha1(), &report.target_sha1, Some(&info))
        );
        assert!(text.contains("Version: 0.1.0\n"), "{text}");
        assert!(text.contains("Änderungen:\nNeue Karte\n"), "{text}");
        let patch = std::fs::read(&out).unwrap();
        assert_eq!(
            g4_xdelta::decode(f.base.bytes(), &patch).unwrap(),
            f.built_bytes
        );
        assert!(tmp_files(&f.dir.join("build")).is_empty());
    }

    #[test]
    fn project_stem_is_the_folder_without_suffix_and_game_code() {
        let (_tmp, plain) = minimal_project("Mein Hack_DSPRE_contents");
        assert_eq!(project_stem(&plain), "Mein Hack");
        // The stem is cleaned like the patch name (a folder with `:` exists only outside
        // Windows, so the folder name is given directly).
        assert_eq!(stem_of("Hack:Neu_DSPRE_contents"), "Hack_Neu");
        #[cfg(unix)]
        {
            let (_tmp2, project) = minimal_project("Hack:Neu_DSPRE_contents");
            assert_eq!(project_stem(&project), "Hack_Neu");
        }
        assert_eq!(patch_name(&plain), format!("{}-CPUE", project_stem(&plain)));
        assert_eq!(stem_of("..."), "Projekt");
    }
}
