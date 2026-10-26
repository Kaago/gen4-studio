//! Gated: adopting a vanilla project's script archive with rotom (needs `G4_TEST_PROJECT`).
#![cfg(feature = "test-support")]
mod common;

use g4_formats::game::Family;
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::scripts::{self, ScriptTarget};

#[test]
fn vanilla_adopts_and_compiles_clean() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Project::open(&root).unwrap();
    assert!(!scripts::is_adopted(&project));
    let members = project.member_count(AssetKind::Script).unwrap();

    let mut events = Vec::new();
    let report = scripts::adopt(&project, &mut |e| events.push(e)).unwrap();
    assert!(scripts::is_adopted(&project));
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].done, events[1].total);
    assert_eq!(events[1].total as usize, members);
    // Every member is a source, an empty member or (listed here) one rotom cannot read.
    assert_eq!(
        report.decompiled as usize + report.skipped_empty as usize + report.failed.len(),
        members,
        "{report:?}"
    );
    assert!(report.failed.is_empty(), "{:?}", report.failed);

    // Nothing is stale right after adopting, so compile writes no member.
    let compiled = scripts::compile(&project).unwrap();
    assert!(compiled.compiled.is_empty(), "{:?}", compiled.compiled);
    assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);
    assert!(compiled.touched_archives.is_empty());

    // The sources reproduce the members byte for byte: delete every member that has a
    // source, compile, compare. A mismatch names the member (a rotom finding, not a reason
    // to overwrite it). The text JSON rotom needs is written on the way, so this also covers
    // the retry in `compile`.
    let files = scripts::list_files(&project).unwrap();
    let mut original = Vec::new();
    for info in files
        .iter()
        .filter(|i| i.kind != scripts::SourceKind::Binary)
    {
        let path = project.member_path(AssetKind::Script, info.file);
        original.push((info.file, std::fs::read(&path).unwrap()));
        std::fs::remove_file(&path).unwrap();
    }
    let rebuilt = scripts::compile(&project).unwrap();
    assert!(rebuilt.problems.is_empty(), "{:?}", rebuilt.problems);
    assert_eq!(rebuilt.compiled.len(), original.len());
    let different: Vec<u16> = original
        .iter()
        .filter(|(file, bytes)| {
            std::fs::read(project.member_path(AssetKind::Script, *file))
                .ok()
                .as_ref()
                != Some(bytes)
        })
        .map(|(file, _)| *file)
        .collect();
    // Includes the members whose original has an unusual jump table (no end marker, or two):
    // rotom records that as a quirk and `compile` keeps it across compile errors.
    assert!(
        different.is_empty(),
        "members that do not round-trip: {different:?}"
    );

    // The listing agrees with the report.
    let files = scripts::list_files(&project).unwrap();
    assert_eq!(files.len(), members);

    // Event script numbers: common scripts resolve in Platinum and HGSS (uxie's tables), not in DP.
    let headers = project.read_headers().unwrap();
    let common_script = scripts::resolve_script(&project, &headers, 0, 3000).unwrap();
    match project.family {
        Family::Dp => assert_eq!(common_script, ScriptTarget::Unknown),
        Family::Pt | Family::Hgss => {
            assert!(
                matches!(common_script, ScriptTarget::File { file, .. } if usize::from(file) < members),
                "{common_script:?}"
            );
        }
    }
}

/// Before rotom compiles, a text JSON is rewritten from its member only when the member
/// changed since the last recorded sync (DSPRE edited it), never by modification time: a
/// JSON older than its member but unchanged, or changed itself, stays as it is.
#[test]
fn refresh_rewrites_the_json_only_when_the_member_is_newer() {
    use std::time::{Duration, SystemTime};

    use g4_kernel::text::{self, TextArchiveDoc, TextSide, TextSync};

    let s = common::synthetic_project();
    let p = Project::open(s.dir.path()).unwrap();
    scripts::adopt(&p, &mut |_| {}).unwrap();
    p.ensure_unpacked(AssetKind::TextArchive).unwrap();
    let json_path = |n: u16| {
        p.expanded_dir(AssetKind::TextArchive)
            .join(format!("{n:04}.json"))
    };
    let age = |path: &std::path::Path, secs: u64| {
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(secs))
            .unwrap();
    };
    // Each JSON carries a second language the member does not hold, so a rewrite from the
    // member shows.
    let write_json = |n: u16, texts: &[&str]| {
        let messages: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| serde_json::json!({ "id": format!("msg_{i:05}"), "en_US": t, "de_DE": "x" }))
            .collect();
        let path = json_path(n);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            serde_json::json!({ "key": 0, "messages": messages }).to_string(),
        )
        .unwrap();
    };
    write_json(392, &["None", "Master Ball", "Ultra Ball"]);
    write_json(433, &["Nowhere", "Twinleaf Town", "Sandgem Town"]);
    write_json(618, &["", "Kai", "Mia"]);
    for n in [392, 433, 618] {
        age(&p.member_path(AssetKind::TextArchive, n), 3600);
        age(&json_path(n), 3600);
    }
    text::record_text_sync(&p, &[392, 433, 618]).unwrap();

    // 392: nothing changed, but the JSON is older than the member.
    age(&json_path(392), 7200);
    // 433: DSPRE changed the member.
    std::fs::write(
        p.member_path(AssetKind::TextArchive, 433),
        common::text_archive_bytes(&["Nowhere", "Twinleaf Town", "Sandgem Town", "Aus DSPRE"]),
    )
    .unwrap();
    // 618: the JSON changed (a branch switch), older than the member.
    write_json(618, &["", "Kai", "Lea"]);
    age(&json_path(618), 7200);
    let sync = TextSync::load(&p);
    assert_eq!(sync.side(&p, 392).unwrap(), TextSide::InSync);
    assert_eq!(sync.side(&p, 433).unwrap(), TextSide::MemberNewer);
    assert_eq!(sync.side(&p, 618).unwrap(), TextSide::JsonNewer);
    let kept = [392, 618].map(|n| std::fs::read(json_path(n)).unwrap());

    scripts::compile(&p).unwrap();

    assert_eq!(std::fs::read(json_path(392)).unwrap(), kept[0]);
    assert_eq!(std::fs::read(json_path(618)).unwrap(), kept[1]);
    let doc = TextArchiveDoc::from_json(&std::fs::read(json_path(433)).unwrap(), "en_US").unwrap();
    assert_eq!(doc.messages.len(), 4);
    assert_eq!(doc.messages[3].text, "Aus DSPRE");
    // The refreshed JSON is recorded: both sides agree again.
    assert_eq!(TextSync::load(&p).side(&p, 433).unwrap(), TextSide::InSync);
}
