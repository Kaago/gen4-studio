#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, and a `git` CLI): "Git einrichten" on a copy of a vanilla DSPRE
//! folder. The status stays empty after the setup and after a build, the packed repository
//! stays within the gate's limit, no game data file falls under an end-of-line rule, and a
//! clone with `core.autocrlf=true` builds byte-equal. The gate of phase 4: the same copy set up
//! as text project builds the base ROM byte-equal, also from a clone completed from the base
//! ROM (`textproject::restore`). A trainer and a text changed through the store show by entity
//! and field name against `HEAD` and the base ROM. "Stand sichern" after a release commits the
//! release list and the changed member, nothing under `build/`. Each run prints one line for the
//! gate document. Git runs in an isolated environment (`GitFixture::cli`) with an identity set
//! through the environment; repositories only in temp folders, never a remote.
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use g4_kernel::base::BaseRom;
use g4_kernel::build::build;
use g4_kernel::diff::{
    Against, DiffSources, DiffState, DiffSubject, EntityDiff, EntityDiffReport, SpanOp,
    diff_entity, head_entries,
};
use g4_kernel::git::cli::REPO_VARS;
use g4_kernel::git::{GitCli, GitRepo, InitOptions, InitPhase, git_init};
use g4_kernel::original::{ResetTarget, reset};
use g4_kernel::project::Project;
use g4_kernel::start::assign_base;
use g4_kernel::store::Store;
use g4_kernel::textproject::{
    self,
    adopt::adopt,
    restore::{RestorePhase, needs_restore, restore_from_base},
    sync,
};
use sha1::{Digest, Sha1};

/// The isolated `git` with an identity from the environment; `None` (the test skips) without
/// one. No automatic maintenance (tests only): a commit would otherwise start
/// `git maintenance run --auto --detach` in the background, racing the test's `repack`.
fn git_cli(home: &Path) -> Option<GitCli> {
    let mut cli = common::GitFixture::cli(home)?;
    for (key, value) in [
        ("GIT_AUTHOR_NAME", "Gen4 Studio Gate"),
        ("GIT_AUTHOR_EMAIL", "gate@gen4.invalid"),
        ("GIT_COMMITTER_NAME", "Gen4 Studio Gate"),
        ("GIT_COMMITTER_EMAIL", "gate@gen4.invalid"),
        ("GIT_CONFIG_COUNT", "2"),
        ("GIT_CONFIG_KEY_0", "maintenance.auto"),
        ("GIT_CONFIG_VALUE_0", "false"),
        ("GIT_CONFIG_KEY_1", "gc.auto"),
        ("GIT_CONFIG_VALUE_1", "0"),
    ] {
        cli.env.push((key.to_owned(), value.to_owned()));
    }
    Some(cli)
}

/// Runs `git -C <dir> <args>` with the isolated environment (test code only: the Studio itself
/// runs only its own subcommands) and returns stdout; panics with stderr when it fails. Without
/// the variables of [`REPO_VARS`], so a run from inside a Git hook never acts on the
/// developer's repository.
fn git(cli: &GitCli, dir: &Path, args: &[&str]) -> String {
    let mut command = Command::new(&cli.path);
    for var in REPO_VARS {
        command.env_remove(var);
    }
    let out = command
        .arg("-C")
        .arg(dir)
        .args(args)
        .envs(cli.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// A clone of `root` without `git clone`: `git init`, `core.autocrlf=true`, `fetch <root>
/// HEAD`, `checkout FETCH_HEAD`.
fn clone_autocrlf(cli: &GitCli, root: &Path, clone: &Path) {
    std::fs::create_dir(clone).unwrap();
    git(cli, clone, &["init", "-q"]);
    git(cli, clone, &["config", "core.autocrlf", "true"]);
    git(cli, clone, &["fetch", "-q", root.to_str().unwrap(), "HEAD"]);
    git(cli, clone, &["checkout", "-q", "FETCH_HEAD"]);
}

fn no_remote(root: &Path) {
    let config = std::fs::read_to_string(root.join(".git/config")).unwrap();
    assert!(!config.contains("[remote"), "{config}");
}

fn changes(root: &Path) -> (Vec<String>, u64) {
    let status = GitRepo::discover(root)
        .unwrap()
        .expect("a repository")
        .status()
        .unwrap();
    let paths = status
        .paths
        .iter()
        .map(|p| format!("{:?} {}", p.state, p.path))
        .collect();
    (paths, status.ms)
}

fn sha1_of(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    Sha1::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A vanilla copy set up with "Git einrichten" (committed), with its report.
fn set_up(cli: &GitCli) -> Option<(tempfile::TempDir, PathBuf, g4_kernel::git::InitReport)> {
    let (tmp, root) = common::vanilla_copy()?;
    let project = Project::open(&root).unwrap();
    let report = git_init(&project, Some(cli), &InitOptions::default(), &mut |_| {}).unwrap();
    assert!(report.committed, "{report:?}");
    assert!(!report.studio_identity);
    no_remote(&root);
    Some((tmp, root, report))
}

#[test]
fn git_setup_on_the_dspre_folder_keeps_status_empty_after_build() {
    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        return;
    };
    let Some((_tmp, root, report)) = set_up(&cli) else {
        return;
    };
    let (after_setup, _) = changes(&root);
    assert!(after_setup.is_empty(), "after the setup: {after_setup:?}");

    let project = Project::open(&root).unwrap();
    build(&project, &root.join("build/out.nds")).unwrap();
    let (after_build, status_ms) = changes(&root);
    assert!(after_build.is_empty(), "after the build: {after_build:?}");

    let packed_kib = packed_kib(&cli, &root);

    let index = GitRepo::discover(&root)
        .unwrap()
        .unwrap()
        .index_paths()
        .unwrap();
    assert_eq!(index.len(), report.files as usize);
    for path in &index {
        assert!(!path.ends_with(".nds"), "{path}");
        if let Some(rest) = path.strip_prefix(".g4/") {
            assert!(["base.json", "preview.png"].contains(&rest), "{path}");
        }
    }
    let ms = |phase| report.phase_ms.get(&phase).copied().unwrap_or(0);
    println!(
        "git {} dspre files={} ignored_archives={} unpack_ms={} add_ms={} commit_ms={} status_ms={} packed_kib={}",
        project.game_code,
        report.files,
        report.templates.ignored_archives,
        ms(InitPhase::Unpack),
        ms(InitPhase::Add),
        ms(InitPhase::Commit),
        status_ms,
        packed_kib
    );
    assert!(packed_kib <= 65 * 1024, "packed_kib={packed_kib}");
    assert!(report.files <= 20_027, "files={}", report.files);
}

/// Every file below `dir`, recursively.
fn files_below(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_below(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn no_game_data_file_falls_under_an_eol_rule() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    // What "Git einrichten" unpacks first, without a repository (no git needed).
    let project = Project::open(&root).unwrap();
    let opts = InitOptions {
        templates_only: true,
    };
    git_init(&project, None, &opts, &mut |_| {}).unwrap();
    let text = ["json", "rotom", "yaml", "yml", "toml", "md"];
    let under_rule = |f: &PathBuf| {
        f.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| text.contains(&e.to_ascii_lowercase().as_str()))
    };
    let mut files = Vec::new();
    for dir in ["files", "unpacked"] {
        files_below(&root.join(dir), &mut files);
    }
    assert!(files.len() > 1000, "{}", files.len());
    let hits: Vec<_> = files.iter().filter(|f| under_rule(f)).collect();
    assert!(hits.is_empty(), "{hits:?}");

    // Below the code folders only ds-rom's own descriptions (`arm9.yaml`, `overlays.yaml`, …)
    // are text; ds-rom writes them with `\n`, so `eol=lf` leaves their bytes as they are.
    let mut code = Vec::new();
    for dir in ["arm9", "arm9_overlays", "arm7"] {
        files_below(&root.join(dir), &mut code);
    }
    let mut yaml = 0;
    for file in code.iter().filter(|f| under_rule(f)) {
        assert_eq!(
            file.extension().and_then(|e| e.to_str()),
            Some("yaml"),
            "{}",
            file.display()
        );
        let bytes = std::fs::read(file).unwrap();
        assert!(!bytes.contains(&b'\r'), "{} has a CR", file.display());
        yaml += 1;
    }
    println!(
        "eol {} dspre game_files={} code_files={} dsrom_yaml={yaml}",
        project.game_code,
        files.len(),
        code.len()
    );
}

#[test]
fn a_clone_with_autocrlf_builds_byte_equal_dspre() {
    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        return;
    };
    let Some((_tmp, root, _)) = set_up(&cli) else {
        return;
    };
    let project = Project::open(&root).unwrap();
    let rom_a = root.join("build/a.nds");
    build(&project, &rom_a).unwrap();

    let clone_dir = tempfile::tempdir().unwrap();
    let clone = clone_dir.path().join("klon");
    clone_autocrlf(&cli, &root, &clone);

    let started = Instant::now();
    let cloned = Project::open(&clone).unwrap();
    let rom_b = clone.join("build/b.nds");
    build(&cloned, &rom_b).unwrap();
    let build_ms = started.elapsed().as_millis();
    assert_eq!(sha1_of(&rom_b), sha1_of(&rom_a));
    let (paths, _) = changes(&clone);
    assert!(paths.is_empty(), "the clone after its build: {paths:?}");
    no_remote(&root);
    no_remote(&clone);
    println!(
        "clone {} dspre build_ms={build_ms} equal",
        project.game_code
    );
}

fn packed_kib(cli: &GitCli, root: &Path) -> u64 {
    git(cli, root, &["repack", "-a", "-d", "-q"]);
    let counted = git(cli, root, &["count-objects", "-v"]);
    counted
        .lines()
        .find_map(|l| l.strip_prefix("size-pack: "))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

/// The gate of phase 4: a text project in Git builds the base ROM byte-equal, from the
/// project and from a clone with `core.autocrlf=true` completed from the base ROM.
#[test]
fn text_project_in_git_builds_from_base_byte_equal() {
    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let base_sha1 = sha1_of(&nds);
    assign_base(&root, &nds).unwrap();
    let base = BaseRom::open(&nds, None).unwrap();
    let started = Instant::now();
    adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
    let adopt_ms = started.elapsed().as_millis();
    let project = Project::open(&root).unwrap();

    let report = git_init(&project, Some(&cli), &InitOptions::default(), &mut |_| {}).unwrap();
    assert!(report.committed, "{report:?}");
    no_remote(&root);
    let (after_setup, _) = changes(&root);
    assert!(after_setup.is_empty(), "after the setup: {after_setup:?}");

    // The index holds every source; anything else is a template or below `expanded/`.
    let index = GitRepo::discover(&root)
        .unwrap()
        .unwrap()
        .index_paths()
        .unwrap();
    let sources: Vec<String> = textproject::sources(&project)
        .unwrap()
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    for source in &sources {
        assert!(index.contains(source), "{source} is not in the index");
    }
    for path in &index {
        assert!(
            sources.contains(path)
                || [".gitignore", ".gitattributes"].contains(&path.as_str())
                || path.starts_with("expanded/"),
            "{path}"
        );
    }

    build(&project, &root.join("build/out.nds")).unwrap();
    assert_eq!(sha1_of(&root.join("build/out.nds")), base_sha1);
    let (after_build, _) = changes(&root);
    assert!(after_build.is_empty(), "after the build: {after_build:?}");
    let packed_kib = packed_kib(&cli, &root);
    let ms = |phase| report.phase_ms.get(&phase).copied().unwrap_or(0);
    println!(
        "git {} text files={} add_ms={} commit_ms={} packed_kib={packed_kib}",
        project.game_code,
        report.files,
        ms(InitPhase::Add),
        ms(InitPhase::Commit),
    );
    assert!(packed_kib <= 10 * 1024, "packed_kib={packed_kib}");

    let clone_dir = tempfile::tempdir().unwrap();
    let clone = clone_dir.path().join("klon");
    clone_autocrlf(&cli, &root, &clone);
    assert!(needs_restore(&clone));
    let started = Instant::now();
    let mut phase_ms: Vec<(RestorePhase, u128)> = Vec::new();
    let mut current: Option<(RestorePhase, Instant)> = None;
    let restored = restore_from_base(&clone, &base, &mut |p| {
        if current.is_none_or(|(phase, _)| phase != p.phase)
            && let Some((phase, at)) = current.replace((p.phase, Instant::now()))
        {
            phase_ms.push((phase, at.elapsed().as_millis()));
        }
    })
    .unwrap();
    if let Some((phase, at)) = current {
        phase_ms.push((phase, at.elapsed().as_millis()));
    }
    let restore_ms = started.elapsed().as_millis();
    same_as_fresh_extract(&clone, &nds);
    // The import recorded the sync: nothing to take over in the clone.
    let scan = sync::scan(&Project::open(&clone).unwrap(), &mut |_| {}).unwrap();
    let pending: Vec<_> = scan
        .units
        .iter()
        .filter(|u| u.side != sync::UnitSide::InSync)
        .collect();
    assert!(pending.is_empty(), "{pending:?}");
    assert!(!needs_restore(&clone));
    let started = Instant::now();
    build(
        &Project::open(&clone).unwrap(),
        &clone.join("build/out.nds"),
    )
    .unwrap();
    let build_ms = started.elapsed().as_millis();
    assert_eq!(sha1_of(&clone.join("build/out.nds")), base_sha1);
    let (paths, _) = changes(&clone);
    assert!(
        paths.is_empty(),
        "the clone after restore and build: {paths:?}"
    );
    no_remote(&root);
    no_remote(&clone);
    println!(
        "clone {} text restore_ms={restore_ms} build_ms={build_ms} equal",
        project.game_code
    );
    let phases: Vec<String> = phase_ms
        .iter()
        .map(|(phase, ms)| {
            format!(
                "{}={ms}",
                serde_json::to_string(phase).unwrap().trim_matches('"')
            )
        })
        .collect();
    println!("restore_phases {} {}", project.game_code, phases.join(" "));
    println!(
        "text {} adopt_ms={adopt_ms} moved={} members={} sync_files={}",
        project.game_code, restored.moved, restored.import.members_written, restored.sync_files
    );

    // `g4k text restore` on a second clone (the synthetic ROM of `tests/cli.rs` cannot be
    // extracted).
    let second = clone_dir.path().join("klon2");
    clone_autocrlf(&cli, &root, &second);
    let out = assert_cmd::Command::cargo_bin("g4k")
        .unwrap()
        .args(["text", "restore"])
        .arg(&second)
        .arg("--base-rom")
        .arg(&nds)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    let last = out.lines().last().unwrap_or_default();
    assert!(last.starts_with("restored moved="), "{out}");
    assert!(!needs_restore(&second));
    let (paths, _) = changes(&second);
    assert!(
        paths.is_empty(),
        "the second clone after g4k text restore: {paths:?}"
    );
}

/// The game files of `root`: every file of the DSPRE part and below `unpacked/`, relative,
/// with `/`.
fn game_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for name in g4_kernel::git::templates::DSPRE_PART
        .iter()
        .chain(&["unpacked/"])
    {
        let path = root.join(name.trim_end_matches('/'));
        if path.is_dir() {
            files_below(&path, &mut files);
        } else if path.is_file() {
            files.push(path);
        }
    }
    let mut rel: Vec<String> = files
        .iter()
        .map(|f| {
            f.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    rel.sort();
    rel
}

/// The restored `clone` has exactly the game files `build::extract_rom` and unpacking every
/// archive (but the Dynamic Headers container) leave, and the DSPRE part byte-equal.
fn same_as_fresh_extract(clone: &Path, nds: &Path) {
    let tmp = tempfile::tempdir().unwrap();
    let fresh = tmp.path().join("frisch");
    let project = g4_kernel::build::extract_rom(nds, &fresh).unwrap();
    let kinds: Vec<_> = project.archives.kinds().collect();
    for kind in kinds {
        if kind == g4_kernel::project::AssetKind::DynamicHeaders {
            continue;
        }
        if project
            .archives
            .packed(kind)
            .is_some_and(|p| fresh.join(p).is_file())
        {
            project.ensure_unpacked(kind).unwrap();
        }
    }
    let ours: std::collections::BTreeSet<String> = game_files(clone).into_iter().collect();
    let theirs: std::collections::BTreeSet<String> = game_files(&fresh).into_iter().collect();
    let only_ours: Vec<_> = ours.difference(&theirs).collect();
    let only_theirs: Vec<_> = theirs.difference(&ours).collect();
    assert!(
        only_ours.is_empty() && only_theirs.is_empty(),
        "only in the clone: {only_ours:?}, only in the extract: {only_theirs:?}"
    );
    for rel in ours.iter().filter(|f| !f.starts_with("unpacked/")) {
        assert!(
            std::fs::read(clone.join(rel)).unwrap() == std::fs::read(fresh.join(rel)).unwrap(),
            "{rel} differs"
        );
    }
}

/// Trainer 1's diff: one field, the level of its first team member.
fn assert_trainer_level(report: &EntityDiffReport) {
    assert_eq!(report.state, DiffState::Changed, "{report:?}");
    let EntityDiff::Fields { changes, more } = &report.diff else {
        panic!("expected fields: {report:?}");
    };
    assert_eq!(*more, 0);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert!(changes[0].path.ends_with("party[0].level"), "{changes:?}");
}

/// Text archive 1's diff: message 0 changed, „ Studio“ added at its end.
fn assert_text_studio(report: &EntityDiffReport) {
    assert_eq!(report.state, DiffState::Changed, "{report:?}");
    let EntityDiff::Messages { messages, more } = &report.diff else {
        panic!("expected messages: {report:?}");
    };
    assert_eq!(*more, 0);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0].index, 0);
    assert_eq!(messages[0].state, DiffState::Changed);
    let last = messages[0].new.last().unwrap();
    assert_eq!(
        (last.op, last.text.as_str()),
        (SpanOp::Added, " Studio"),
        "{messages:?}"
    );
}

#[test]
fn trainer_and_text_changes_show_with_field_names_against_head_and_base() {
    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        eprintln!("skipped: git not found");
        return;
    };
    let Some(nds) = common::vanilla_base_rom() else {
        eprintln!("skipped: no base ROM next to the vanilla project");
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
        eprintln!("skipped: no vanilla project to copy");
        return;
    };
    assign_base(&root, &nds).unwrap();
    let project = Project::open(&root).unwrap();
    let report = git_init(&project, Some(&cli), &InitOptions::default(), &mut |_| {}).unwrap();
    assert!(report.committed, "{report:?}");
    no_remote(&root);

    let mut store = Store::new(std::sync::Arc::new(Project::open(&root).unwrap()));
    store
        .edit_trainer(1, "Level", |t| t.party[0].level += 1)
        .unwrap()
        .unwrap();
    store
        .edit_text_archive(1, "Text", |d| d.messages[0].text.push_str(" Studio"))
        .unwrap()
        .unwrap();
    store.save().unwrap();

    let project = Project::open(&root).unwrap();
    let started = Instant::now();
    let repo = GitRepo::discover(&root).unwrap().unwrap();
    let status = repo.status().unwrap();
    let status_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let entries = head_entries(&project, &repo, &status).unwrap();
    let entries_ms = started.elapsed().as_millis();
    let labels: Vec<_> = entries.iter().map(|e| e.label.as_str()).collect();
    assert!(
        labels.contains(&"Trainer 0001"),
        "{labels:?} from {status:?}"
    );
    assert!(
        labels.contains(&"Textarchiv 0001"),
        "{labels:?} from {status:?}"
    );
    eprintln!("entries {labels:?} from {} paths", status.paths.len());

    let base = BaseRom::open(&nds, None).unwrap();
    let sources = DiffSources {
        original: Some(&base),
        base_label: Some("Basis".to_owned()),
        repo: Some(&repo),
    };
    let trainer = DiffSubject::Trainer { index: 1 };
    let text = DiffSubject::TextArchive { index: 1 };
    let diff = |store: Option<&mut Store>, subject: &DiffSubject, against| {
        diff_entity(&project, store, &sources, subject, against).unwrap()
    };
    let started = Instant::now();
    let head = [
        diff(None, &trainer, Against::Head),
        diff(None, &text, Against::Head),
    ];
    let head_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let against_base = [
        diff(None, &trainer, Against::Base),
        diff(None, &text, Against::Base),
    ];
    let base_ms = started.elapsed().as_millis();
    for [trainer_report, text_report] in [&head, &against_base] {
        assert_trainer_level(trainer_report);
        assert_text_studio(text_report);
    }
    assert_eq!(head[0].diff, against_base[0].diff, "the same rows");
    assert_eq!(head[1].diff, against_base[1].diff, "the same rows");
    for subject in [&trainer, &text] {
        let saved = diff(Some(&mut store), subject, Against::Saved);
        assert_eq!(saved.state, DiffState::Same, "{saved:?}");
    }

    reset(&mut store, &base, &ResetTarget::Trainer { index: 1 }, None).unwrap();
    store.save().unwrap();
    let after = diff(None, &trainer, Against::Base);
    assert_eq!(after.state, DiffState::Same, "{after:?}");

    println!(
        "diff {} status_ms={status_ms} entries_ms={entries_ms} head_ms={head_ms} base_ms={base_ms}",
        project.game_code
    );
    if std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1") {
        assert!(status_ms <= 500, "status_ms={status_ms}");
    }
}

/// "Stand sichern" after a release: the release list and the changed map member are
/// committed, the patch and its texts under `build/` are not; the status is empty afterwards
/// and there is no remote.
#[test]
fn publish_then_commit_all_leaves_status_empty_and_no_remote() {
    use g4_kernel::git::{CommitOptions, CommitPhase, commit_all};
    use g4_kernel::patch::{PublishOptions, build_and_publish};
    use g4_kernel::project::AssetKind;

    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        return;
    };
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let (_, mark) = assign_base(&root, &nds).unwrap();
    let base = BaseRom::open(&nds, Some(&mark.sha1)).unwrap();
    let project = Project::open(&root).unwrap();
    let init = git_init(&project, Some(&cli), &InitOptions::default(), &mut |_| {}).unwrap();
    assert!(init.committed, "{init:?}");

    // One permission byte of map 3, as in `publish_after_a_map_edit_writes_a_small_patch_…`.
    project.ensure_unpacked(AssetKind::Map).unwrap();
    let map = project.member_path(AssetKind::Map, 3);
    let mut bytes = std::fs::read(&map).unwrap();
    bytes[0x20] ^= 0x80;
    std::fs::write(&map, bytes).unwrap();

    let opts = PublishOptions {
        version: "0.1.0".into(),
        notes: "Erste Fassung".into(),
        dir: None,
        file: None,
        readme: true,
        overwrite: false,
    };
    let published = build_and_publish(&project, &base, &opts, &mut |_| {}).unwrap();
    assert!(root.join(".g4/releases.json").is_file());
    assert!(published.patch.patch.starts_with(root.join("build")));

    let report = commit_all(
        &project,
        Some(&cli),
        &CommitOptions {
            message: Some("Veröffentlicht v0.1.0".into()),
            notes: Some("Erste Fassung".into()),
        },
        &mut |_| {},
    )
    .unwrap();
    assert!(report.committed, "{report:?}");
    assert_eq!(
        report.head.as_ref().unwrap().summary,
        "Veröffentlicht v0.1.0"
    );
    assert!(report.files >= 2, "files={}", report.files);

    let (after, status_ms) = changes(&root);
    assert!(after.is_empty(), "after the commit: {after:?}");
    let index = GitRepo::discover(&root)
        .unwrap()
        .unwrap()
        .index_paths()
        .unwrap();
    assert!(
        index.iter().any(|p| p == ".g4/releases.json"),
        "the release list"
    );
    assert!(index.iter().any(|p| p == ".g4/base.json"), "the base mark");
    assert!(
        !index.iter().any(|p| p.starts_with("build/")),
        "no patch, no texts, no ROM"
    );
    no_remote(&root);

    let ms = |phase| report.phase_ms.get(&phase).copied().unwrap_or(0);
    println!(
        "commit {} files={} add_ms={} commit_ms={} status_ms={}",
        project.game_code,
        report.files,
        ms(CommitPhase::Add),
        ms(CommitPhase::Commit),
        status_ms
    );
}

/// The gate of 3d.4: a trainer created in a text project in Git and committed; on the branch
/// `ohne` it is deleted, its number freed and that committed. `git switch` back brings its JSON
/// back, and the build appends its members again; `git switch ohne` leaves the members without
/// JSON, and the build step removes them (`textproject.member-removed`, the import's rule: at
/// the end, unchanged since the sync's record, not in the base). Both builds succeed and the
/// status stays empty.
#[test]
fn a_branch_without_the_freed_file_builds_after_git_switch() {
    use std::sync::Arc;

    use g4_kernel::data::names::NameTables;
    use g4_kernel::data::refs::RefIndex;
    use g4_kernel::data::trainer_create::create_trainer;
    use g4_kernel::delete::{DeleteTarget, delete, free_number};
    use g4_kernel::project::AssetKind;

    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let Some(nds) = common::vanilla_base_rom() else {
        return;
    };
    assign_base(&root, &nds).unwrap();
    let base = BaseRom::open(&nds, None).unwrap();
    adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
    let project = Arc::new(Project::open(&root).unwrap());
    let report = git_init(&project, Some(&cli), &InitOptions::default(), &mut |_| {}).unwrap();
    assert!(report.committed, "{report:?}");
    let main = git(&cli, &root, &["branch", "--show-current"])
        .trim()
        .to_owned();

    // A new trainer at the end, saved and committed on the main branch.
    let mut store = Store::new(project.clone());
    let mut names = NameTables::new();
    let id = create_trainer(&project, &mut store, &mut names, 2, "Jens")
        .unwrap()
        .id;
    store.save().unwrap();
    let json = root.join(format!("data/trainers/{id:04}.json"));
    assert!(json.is_file());
    let members = [
        project.member_path(AssetKind::TrainerProperties, id),
        project.member_path(AssetKind::TrainerParty, id),
    ];
    git(&cli, &root, &["add", "-A"]);
    git(&cli, &root, &["commit", "-q", "-m", "Trainer angelegt"]);

    // Branch `ohne`: deleted and freed.
    git(&cli, &root, &["switch", "-q", "-c", "ohne"]);
    let target = DeleteTarget::Trainer { index: id };
    let mut refs = RefIndex::new();
    delete(&mut store, &mut refs, &base, target).unwrap();
    let freed = free_number(&mut store, &mut refs, &base, target).unwrap();
    assert_eq!(freed.freed.map(|f| f.count), Some(1));
    store.save().unwrap();
    drop(store);
    assert!(!json.exists());
    for member in &members {
        assert!(!member.exists(), "{}", member.display());
    }
    git(&cli, &root, &["add", "-A"]);
    git(&cli, &root, &["commit", "-q", "-m", "Trainer freigegeben"]);

    // Back on the main branch the JSON is there: the build appends the members.
    git(&cli, &root, &["switch", "-q", &main]);
    assert!(json.is_file());
    let started = Instant::now();
    build(&Project::open(&root).unwrap(), &root.join("build/main.nds")).unwrap();
    let main_ms = started.elapsed().as_millis();
    for member in &members {
        assert!(member.is_file(), "{}", member.display());
    }
    let (paths, _) = changes(&root);
    assert!(paths.is_empty(), "after the build on {main}: {paths:?}");

    // On `ohne` the JSON is gone: the build step removes the members.
    git(&cli, &root, &["switch", "-q", "ohne"]);
    assert!(!json.exists());
    let started = Instant::now();
    let built = build(&Project::open(&root).unwrap(), &root.join("build/ohne.nds")).unwrap();
    let ohne_ms = started.elapsed().as_millis();
    for member in &members {
        assert!(!member.exists(), "{}", member.display());
    }
    assert!(
        built
            .problems
            .iter()
            .any(|p| p.message.starts_with("textproject.member-removed:")),
        "{:?}",
        built.problems
    );
    let (paths, _) = changes(&root);
    assert!(paths.is_empty(), "after the build on ohne: {paths:?}");

    // The freed number is free on `ohne`: the next trainer created takes it.
    let project = Arc::new(Project::open(&root).unwrap());
    let mut store = Store::new(project.clone());
    let again = create_trainer(&project, &mut store, &mut NameTables::new(), 2, "Mia")
        .unwrap()
        .id;
    assert_eq!(again, id);
    println!(
        "branch-free {} trainer={id} build_main_ms={main_ms} build_ohne_ms={ohne_ms} removed",
        project.game_code
    );
}
