#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use super::*;
use crate::scripts::ScriptSource;
use crate::test_support::TextFixture;

const ARCHIVE: u16 = 433;

/// Files the compile wrote before their file steps were recorded go back to their state
/// before: the source, the member, the compile state, an archive member and an archive JSON
/// that was not there (removed).
#[test]
fn disk_before_puts_back_what_a_compile_wrote() {
    let fx = TextFixture::exported();
    let project = fx.project();
    scripts::write_source(
        &project,
        0,
        &ScriptSource::Rotom("script main #1:\n    End\n".into()),
    )
    .unwrap();
    let store = Store::new(Arc::new(fx.project()));
    let snapshot = |project: &Project| {
        (
            scripts::snapshot_script(project, 0).unwrap(),
            scripts::snapshot_compile_state(project).unwrap(),
            scripts::snapshot_archive(project, ARCHIVE).unwrap(),
        )
    };
    let before = snapshot(store.project());
    let disk = DiskBefore {
        file: 0,
        script: before.0.clone(),
        state: before.1.clone(),
        archives: [(ARCHIVE, before.2.clone())].into(),
    };

    scripts::append_source(
        &project,
        0,
        "script main #1:\n    End\n".to_owned(),
        "script zwei #2:\n    End\n",
    )
    .unwrap();
    let json = json_path(&project, ARCHIVE);
    let had_json = json.is_file();
    std::fs::write(&json, b"{}").unwrap();
    let member = project.member_path(AssetKind::TextArchive, ARCHIVE);
    std::fs::write(&member, b"kaputt").unwrap();
    assert_ne!(snapshot(&project), before);

    disk.restore(&project).unwrap();
    assert_eq!(snapshot(&project), before);
    assert_eq!(json.is_file(), had_json);
    // Nothing differs: nothing is written.
    disk.restore(&project).unwrap();
    assert_eq!(snapshot(&project), before);
}
