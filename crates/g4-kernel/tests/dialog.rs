//! Gated: the dialogs of the figures on a copy of a vanilla project (needs `G4_TEST_PROJECT`).
//! First the start town, Platinum Zweiblattdorf (header 411) and HeartGold Neuborkia (header
//! 60): every figure's dialog resolves and every message of a local script has its text. Then
//! every object of the game, each event file once with the first header that names it: no
//! dialog is unreadable, and on Platinum figures share messages (Gutachten A counted 66 shared
//! messages without following jumps; Zweiblattdorf itself has none, its archive 554 is its own
//! and each message is shown once).
#![cfg(feature = "test-support")]
mod common;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::dialog::{Dialog, DialogKind, DialogSource, dialog_of};
use g4_kernel::project::Project;
use g4_kernel::scripts;
use g4_kernel::store::Store;

/// The number of objects in event file `file`.
fn objects_in(store: &mut Store, file: u16) -> u16 {
    u16::try_from(store.event(file).unwrap().object_events.len()).unwrap()
}

#[test]
fn vanilla_dialogs_of_a_town_resolve() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    let header = match project.family {
        Family::Pt => 411,
        Family::Hgss => 60,
        Family::Dp => {
            eprintln!("skipped: no Diamond/Pearl expectations");
            return;
        }
    };
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let mut store = Store::new(project.clone());
    let game = &project.game_code;

    // The start town.
    let file = store.header_table().unwrap()[usize::from(header)].event_file_id();
    let objects = objects_in(&mut store, file);
    let started = Instant::now();
    let (mut local, mut messages, mut shared) = (0, 0, 0);
    for index in 0..objects {
        let dialog = dialog_of(&mut store, header, file, DialogKind::Object, index).unwrap();
        match &dialog.source {
            DialogSource::Unreadable { reason } => panic!("object {index}: {reason}"),
            DialogSource::Local { .. } => {
                local += 1;
                for m in &dialog.messages {
                    assert!(m.text.is_some(), "object {index}: {m:?}");
                }
            }
            _ => assert!(dialog.messages.is_empty(), "object {index}: {dialog:?}"),
        }
        messages += dialog.messages.len();
        shared += dialog
            .messages
            .iter()
            .filter(|m| m.shared_with >= 1)
            .count();
    }
    let ms = started.elapsed().as_millis();
    eprintln!(
        "dialog {game} objects={objects} local={local} messages={messages} shared={shared} ms={ms}"
    );
    assert!(local > 0 && messages > 0);

    // Every object of the game.
    let files: Vec<(u16, u16)> = (0u16..)
        .zip(store.header_table().unwrap())
        .map(|(h, header)| (h, header.event_file_id()))
        .collect();
    let mut seen = BTreeSet::new();
    let (mut objects, mut unreadable) = (0, Vec::new());
    let (mut shared_pairs, mut shared_figures) = (BTreeSet::new(), 0);
    let started = Instant::now();
    for (header, file) in files {
        // An event file that does not load (a header pointing past the archive) has no figures.
        if !seen.insert(file) || store.event(file).is_err() {
            continue;
        }
        for index in 0..objects_in(&mut store, file) {
            objects += 1;
            let Dialog {
                source, messages, ..
            } = dialog_of(&mut store, header, file, DialogKind::Object, index).unwrap();
            if let DialogSource::Unreadable { reason } = source {
                unreadable.push(format!(
                    "header {header} file {file} object {index}: {reason}"
                ));
            }
            let mine: Vec<_> = messages
                .iter()
                .filter(|m| m.shared_with >= 1)
                .map(|m| (m.archive, m.index))
                .collect();
            shared_figures += usize::from(!mine.is_empty());
            shared_pairs.extend(mine);
        }
    }
    let ms = started.elapsed().as_millis();
    let (bad, pairs) = (unreadable.len(), shared_pairs.len());
    eprintln!(
        "dialog-all {game} objects={objects} unreadable={bad} shared_pairs={pairs} \
         shared_figures={shared_figures} ms={ms}"
    );
    assert!(unreadable.is_empty(), "{unreadable:#?}");
    if project.family == Family::Pt {
        assert!(shared_figures >= 1, "no figure shares a message");
    }
}
