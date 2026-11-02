//! Gated: the message references of the scripts on a copy of a vanilla project (needs
//! `G4_TEST_PROJECT`). The scripts are adopted, then the index is built twice (the second,
//! warm build is timed and printed, judged nowhere). A message the start town shows
//! (Platinum Zweiblattdorf, header 411; HeartGold Neuborkia, header 60) has a reference at
//! the line its dialog names; the game holds about as many numbered message uses as Gutachten
//! A counted (4 999 Platinum, 4 407 HeartGold), within 20 %: the files no header names
//! (common scripts) have no archive and no message references.
#![cfg(feature = "test-support")]
mod common;

use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::data::refs::{RefIndex, Target};
use g4_kernel::dialog::{DialogKind, DialogSource, dialog_of};
use g4_kernel::project::{AssetKind, Project};
use g4_kernel::scripts;
use g4_kernel::store::Store;

#[test]
fn vanilla_message_refs_count_the_script_messages() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let (header, counted) = match project.family {
        Family::Pt => (411, 4999),
        Family::Hgss => (60, 4407),
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return;
        }
    };
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let mut store = Store::new(project.clone());
    let game = &project.game_code;

    let mut index = RefIndex::new();
    index.rebuild_full(&store).unwrap();
    let started = Instant::now();
    index.rebuild_full(&store).unwrap();
    let warm_ms = started.elapsed().as_millis();
    let messages = index
        .refs_where(&store, |t| matches!(t, Target::Message { .. }))
        .unwrap();
    eprintln!("refs {game} messages={} warm_ms={warm_ms}", messages.len());
    assert!(messages.iter().all(|r| r.source.kind == AssetKind::Script));
    let (low, high) = (counted * 8 / 10, counted * 12 / 10);
    assert!(
        (low..=high).contains(&messages.len()),
        "{} message references, expected {low} to {high}",
        messages.len()
    );

    // The first message a figure of the start town shows.
    let h = &store.header_table().unwrap()[usize::from(header)];
    let (events, archive) = (h.event_file_id(), h.text_archive_id());
    let objects = u16::try_from(store.event(events).unwrap().object_events.len()).unwrap();
    let (file, m) = (0..objects)
        .find_map(|i| {
            let dialog = dialog_of(&mut store, header, events, DialogKind::Object, i).unwrap();
            let DialogSource::Local { file, .. } = dialog.source else {
                return None;
            };
            Some((file, dialog.messages.into_iter().next()?))
        })
        .expect("a figure of the start town shows a message");
    assert_eq!(m.archive, archive);
    let refs = index
        .refs(
            &store,
            Target::Message {
                archive,
                index: m.index,
            },
        )
        .unwrap();
    eprintln!(
        "refs {game} message {archive}/{} refs={} (file {file:04} line {})",
        m.index,
        refs.len(),
        m.line
    );
    assert!(
        refs.iter().any(|r| r.source.index == file
            && r.slot == u16::try_from(m.line).ok()
            && r.field == m.command),
        "{refs:?}"
    );
}
