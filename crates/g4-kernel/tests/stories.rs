#![cfg(feature = "test-support")]
//! Stories in the text project: export and import leave them alone, the check and the source
//! list know their files, deleting one is one undo step that frees its number (synthetic
//! Platinum fixture). Gated (`G4_TEST_PROJECT`, the base ROM
//! next to it, and a `git` CLI): a vanilla copy set up as text project in Git with one story
//! builds the base ROM byte-equal, and the story's files are in the index. Prints one line
//! (`story-git …`) for the gate document.
mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use g4_kernel::base::BaseRom;
use g4_kernel::build::build;
use g4_kernel::error::KernelError;
use g4_kernel::flags::{FlagKind, names};
use g4_kernel::git::{CommitOptions, GitCli, GitRepo, InitOptions, commit_all, git_init};
use g4_kernel::project::Project;
use g4_kernel::start::assign_base;
use g4_kernel::store::Store;
use g4_kernel::stories::{
    Layout, Position, Progress, StoryPatch, StoryRequest, create_story, create_story_files,
    delete_story, delete_story_files, list_stories, update_story_files,
};
use g4_kernel::textproject::ids::unknown_id_text;
use g4_kernel::textproject::{self, ExportOptions, adopt::adopt, check, export, import, sources};
use sha1::{Digest, Sha1};

use common::TextFixture;

const STORY_FILES: [&str; 3] = [
    "data/ids/stories.json",
    "data/stories/0000.json",
    "data/stories/0000.layout.json",
];

fn testfaden(var: &str) -> StoryRequest {
    StoryRequest {
        name: "Testfaden".to_owned(),
        progress: Progress::Var(var.to_owned()),
        notes: String::new(),
        steps: Vec::new(),
        flags: Vec::new(),
    }
}

/// The bytes and the modification time of each file of [`STORY_FILES`].
fn story_files(fx: &TextFixture) -> BTreeMap<&'static str, (Vec<u8>, SystemTime)> {
    STORY_FILES
        .iter()
        .map(|rel| {
            let path = fx.root().join(rel);
            let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
            (*rel, (std::fs::read(&path).unwrap(), mtime))
        })
        .collect()
}

#[test]
fn export_and_import_leave_stories_alone() {
    let fx = TextFixture::exported();
    create_story_files(&fx.project(), &testfaden("VAR_UNUSED_0x406C")).unwrap();
    let before = story_files(&fx);

    // An export after making the story writes nothing, the story's files least of all.
    let report = export(
        &fx.project(),
        &fx.base(),
        &ExportOptions {
            adopt_scripts: true,
            force: false,
        },
    )
    .unwrap();
    assert_eq!(report.written, 0, "{report:?}");
    assert_eq!(report.removed, 0);
    assert_eq!(report.registries_written, 0);
    assert!(report.problems.is_empty(), "{:#?}", report.problems);
    assert_eq!(story_files(&fx), before);

    // The import does not read them: a story file that does not read is no import error.
    fx.write("data/stories/0001.json", b"keine Geschichte");
    import(&fx.project(), Some(&fx.base())).unwrap();
    assert_eq!(story_files(&fx), before);
    assert_eq!(fx.read("data/stories/0001.json"), b"keine Geschichte");
}

#[test]
fn check_lists_the_story_files_as_sources() {
    let fx = TextFixture::exported();
    create_story_files(&fx.project(), &testfaden("VAR_UNUSED_0x406C")).unwrap();
    let list = sources(&fx.project()).unwrap();
    for rel in STORY_FILES {
        assert!(list.contains(&PathBuf::from(rel)), "{rel} in {list:?}");
    }
    let report = check(&fx.project()).unwrap();
    assert!(report.problems.is_empty(), "{:#?}", report.problems);
    assert!(report.differs.is_empty(), "{:?}", report.differs);

    // A story file that does not read is the check's problem, with its file and line.
    fx.edit(
        "data/stories/0000.json",
        "\"name\": \"Testfaden\",",
        "\"name\": \"Testfaden\",\n  \"farbe\": \"rot\",",
    );
    let report = check(&fx.project()).unwrap();
    assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
    let problem = &report.problems[0];
    assert!(
        problem.message.starts_with("textproject.schema:"),
        "{problem:?}"
    );
    assert_eq!(problem.file, Path::new("data/stories/0000.json"));
    assert_eq!(problem.line, 4);
    assert!(report.differs.is_empty(), "{:?}", report.differs);
}

/// The bytes of `rel`, `None` when the file is missing.
fn bytes_of(fx: &TextFixture, rel: &str) -> Option<Vec<u8>> {
    std::fs::read(fx.root().join(rel)).ok()
}

#[test]
fn delete_removes_both_files_and_the_register_entry_as_one_undo_step() {
    let fx = TextFixture::exported();
    let mut store = Store::new(Arc::new(fx.project()));
    create_story(&mut store, &testfaden("VAR_UNUSED_0x406C")).unwrap();
    // A second story stays in the register; the layout of the first has positions.
    create_story_files(&fx.project(), &testfaden("VAR_UNUSED_0x406C")).unwrap();
    let layout = Layout {
        nodes: [
            ("s1".to_owned(), Position { x: 120.0, y: -40.5 }),
            ("s2".to_owned(), Position { x: 0.25, y: 300.0 }),
        ]
        .into_iter()
        .collect(),
    };
    update_story_files(
        &fx.project(),
        "STORY_TESTFADEN",
        &StoryPatch {
            layout: Some(layout),
            ..StoryPatch::default()
        },
    )
    .unwrap();
    let before = story_files(&fx);
    let register = fx.read("data/ids/stories.json");
    let undo_before = store.history().0.len();

    let report = delete_story(&mut store, "STORY_TESTFADEN").unwrap();
    assert_eq!((report.id.as_str(), report.number), ("STORY_TESTFADEN", 0));
    assert!(report.change.is_some());
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        report.files,
        STORY_FILES.iter().map(PathBuf::from).collect::<Vec<_>>()
    );
    assert_eq!(
        store.history().0.last().map(String::as_str),
        Some("Story “Testfaden” deleted")
    );
    assert_eq!(store.history().0.len(), undo_before + 1);
    assert_eq!(bytes_of(&fx, "data/stories/0000.json"), None);
    assert_eq!(bytes_of(&fx, "data/stories/0000.layout.json"), None);
    assert_eq!(
        fx.text("data/ids/stories.json"),
        "{\n  \"STORY_TESTFADEN_2\": 1\n}\n"
    );
    let left: Vec<u16> = list_stories(&fx.project())
        .unwrap()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(left, [1]);
    // The check finds nothing: no file without its register entry, no entry without a file.
    let checked = textproject::check(&fx.project()).unwrap();
    assert!(checked.problems.is_empty(), "{:#?}", checked.problems);

    // One undo brings all three back, byte-equal (the layout with its positions too).
    store.undo().unwrap();
    let after: BTreeMap<_, _> = story_files(&fx)
        .into_iter()
        .map(|(rel, (bytes, _))| (rel, bytes))
        .collect();
    let expected: BTreeMap<_, _> = before
        .into_iter()
        .map(|(rel, (bytes, _))| (rel, bytes))
        .collect();
    assert_eq!(after, expected);
    assert_eq!(fx.read("data/ids/stories.json"), register);
    assert_eq!(store.history().0.len(), undo_before);

    // Redo removes them again.
    store.redo().unwrap();
    assert_eq!(bytes_of(&fx, "data/stories/0000.json"), None);
    assert_eq!(bytes_of(&fx, "data/stories/0000.layout.json"), None);
}

#[test]
fn delete_of_an_unknown_id_is_unknown_id() {
    let fx = TextFixture::exported();
    let mut store = Store::new(Arc::new(fx.project()));
    create_story(&mut store, &testfaden("VAR_UNUSED_0x406C")).unwrap();
    let before = story_files(&fx);
    let undo = store.history().0.len();
    for result in [
        delete_story(&mut store, "STORY_NOPE"),
        delete_story_files(&fx.project(), "STORY_NOPE"),
    ] {
        match result {
            Err(KernelError::Refused(why)) => assert_eq!(why, unknown_id_text("STORY_NOPE")),
            other => panic!("expected unknown id, got {other:?}"),
        }
    }
    assert_eq!(story_files(&fx), before);
    assert_eq!(store.history().0.len(), undo);
}

#[test]
fn a_deleted_number_is_free_again() {
    let fx = TextFixture::exported();
    let p = fx.project();
    create_story_files(&p, &testfaden("VAR_UNUSED_0x406C")).unwrap();
    let second = create_story_files(&p, &testfaden("VAR_UNUSED_0x406C")).unwrap();
    assert_eq!(second.number, 1);
    let report = delete_story_files(&p, "STORY_TESTFADEN").unwrap();
    assert_eq!((report.number, report.change), (0, None));
    assert_eq!(bytes_of(&fx, "data/stories/0000.json"), None);

    // Nothing points at a story: the next one takes number 0000 and the id again.
    let next = create_story_files(&p, &testfaden("VAR_UNUSED_0x406C")).unwrap();
    assert_eq!((next.id.as_str(), next.number), ("STORY_TESTFADEN", 0));
    assert!(fx.root().join("data/stories/0000.json").is_file());
    assert!(fx.root().join("data/stories/0000.layout.json").is_file());
}

/// The isolated `git` with an identity from the environment (as in `tests/git.rs`); `None`
/// (the test skips) without one.
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

/// The paths `git status` shows, sorted.
fn changed(root: &Path) -> Vec<String> {
    let status = GitRepo::discover(root)
        .unwrap()
        .expect("a repository")
        .status()
        .unwrap();
    let mut paths: Vec<String> = status.paths.iter().map(|p| p.path.clone()).collect();
    paths.sort();
    paths
}

fn sha1_of(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    Sha1::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The base ROM of the vanilla project: `<Name>.nds` next to `<Name>_DSPRE_contents`, or
/// `<folder>.nds` next to a folder without the suffix (the test copies of a phase), as in
/// `tests/flags.rs`.
fn base_rom(project: &Path) -> Option<PathBuf> {
    if let Some(nds) = g4_kernel::start::suggest_base(project) {
        return Some(nds);
    }
    let nds = project
        .parent()?
        .join(format!("{}.nds", project.file_name()?.to_str()?));
    if !nds.is_file() {
        eprintln!("skipped: no base ROM next to {}", project.display());
        return None;
    }
    Some(nds)
}

/// The gate of 3d.6: a text project in Git with one story builds the base ROM byte-equal; the
/// story is three files (register, story, side file), all of them sources and in the index.
#[test]
fn text_project_with_a_story_builds_from_base_byte_equal() {
    let home = tempfile::tempdir().unwrap();
    let Some(cli) = git_cli(home.path()) else {
        return;
    };
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let Some(nds) = common::vanilla_project().and_then(|src| base_rom(&src)) else {
        return;
    };
    let base_sha1 = sha1_of(&nds);
    let (assigned, _) = assign_base(&root, &nds).unwrap();
    assert_eq!(assigned.refused, None);
    let base = BaseRom::open(&nds, None).unwrap();
    adopt(&Project::open(&root).unwrap(), &base, &mut |_| {}).unwrap();
    let project = Project::open(&root).unwrap();
    let init = git_init(&project, Some(&cli), &InitOptions::default(), &mut |_| {}).unwrap();
    assert!(init.committed, "{init:?}");
    assert!(changed(&root).is_empty(), "{:?}", changed(&root));

    // The pret name of variable 0x406C in this game (its fallback id where it has none).
    let var = names(project.family, FlagKind::Var)
        .name(0x406C)
        .map_or_else(|| "VAR_16492".to_owned(), str::to_owned);
    let report = create_story_files(&project, &testfaden(&var)).unwrap();
    assert_eq!((report.id.as_str(), report.number), ("STORY_TESTFADEN", 0));
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(changed(&root), STORY_FILES);

    let commit = commit_all(
        &project,
        Some(&cli),
        &CommitOptions {
            message: Some("Story “Testfaden” created".to_owned()),
            notes: None,
        },
        &mut |_| {},
    )
    .unwrap();
    assert!(commit.committed, "{commit:?}");
    assert!(changed(&root).is_empty(), "{:?}", changed(&root));
    let index = GitRepo::discover(&root)
        .unwrap()
        .unwrap()
        .index_paths()
        .unwrap();
    for rel in STORY_FILES {
        assert!(index.iter().any(|p| p == rel), "{rel} is not in the index");
    }
    let list: Vec<String> = textproject::sources(&project)
        .unwrap()
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    for path in &index {
        assert!(
            list.contains(path)
                || [".gitignore", ".gitattributes"].contains(&path.as_str())
                || path.starts_with("expanded/"),
            "{path}"
        );
    }

    let started = Instant::now();
    build(&project, &root.join("build/out.nds")).unwrap();
    let build_ms = started.elapsed().as_millis();
    assert_eq!(sha1_of(&root.join("build/out.nds")), base_sha1);
    assert!(
        changed(&root).is_empty(),
        "after the build: {:?}",
        changed(&root)
    );
    println!("story-git {} build_ms={build_ms}", project.game_code);
}
