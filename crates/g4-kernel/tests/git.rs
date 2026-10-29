#![cfg(feature = "test-support")]
//! Gated (`G4_TEST_PROJECT`, and a `git` CLI): "Git einrichten" on a copy of a vanilla DSPRE
//! folder. The status stays empty after the setup and after a build, the packed repository
//! stays within the gate's limit, no game data file falls under an end-of-line rule, and a
//! clone with `core.autocrlf=true` builds byte-equal. The gate of phase 4: the same copy set up
//! as text project builds the base ROM byte-equal, also from a clone completed from the base
//! ROM (`textproject::restore`). Each run prints one line for the gate document. Git runs in an isolated environment (`GitFixture::cli`) with an identity set
//! through the environment; repositories only in temp folders, never a remote.
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use g4_kernel::base::BaseRom;
use g4_kernel::build::build;
use g4_kernel::git::cli::REPO_VARS;
use g4_kernel::git::{GitCli, GitRepo, InitOptions, InitPhase, git_init};
use g4_kernel::project::Project;
use g4_kernel::start::assign_base;
use g4_kernel::textproject::{
    self,
    adopt::adopt,
    restore::{needs_restore, restore_from_base},
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
    let restored = restore_from_base(&clone, &base, &mut |_| {}).unwrap();
    let restore_ms = started.elapsed().as_millis();
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
