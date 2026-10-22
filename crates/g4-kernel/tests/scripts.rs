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
