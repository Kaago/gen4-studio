//! Gated: the dialogs of the figures on a copy of a vanilla project (needs `G4_TEST_PROJECT`).
//! First the start town, Platinum Zweiblattdorf (header 411) and HeartGold Neuborkia (header
//! 60): every figure's dialog resolves and every message of a local script has its text. Then
//! every object of the game, each event file once with the first header that names it: no
//! dialog is unreadable, and on Platinum figures share messages (Gutachten A counted 66 shared
//! messages without following jumps; Zweiblattdorf itself has none, its archive 554 is its own
//! and each message is shown once). Zweiblattdorf's object 7 runs script 65535: no script.
//! Last the places with the most objects: `dialogs_of_header` lists each within a second
//! (judged with `G4_TIMING_STRICT=1`, else printed).
#![cfg(feature = "test-support")]
mod common;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

use g4_formats::game::Family;
use g4_kernel::dialog::{Dialog, DialogKind, DialogSource, dialog_of, dialogs_of_header};
use g4_kernel::project::Project;
use g4_kernel::scripts::{self, ScriptTarget};
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

    // UI test 3d.5 (U5): object 7 of Zweiblattdorf runs script 65535. The common script table's
    // last range is open upwards, so it lands in that range's file at a slot far past the ones
    // the file declares: no script that exists. Its figure says nothing.
    if project.family == Family::Pt {
        assert_eq!(store.event(file).unwrap().object_events[7].script, 0xFFFF);
        let headers = store.header_table().unwrap().to_vec();
        let target = scripts::resolve_script(&project, &headers, header, 0xFFFF).unwrap();
        if let ScriptTarget::File { file: common, line } = target {
            let source = match scripts::read_source(&project, common).unwrap() {
                scripts::ScriptSource::Rotom(text) => text,
                _ => String::new(),
            };
            let highest = source
                .lines()
                .filter_map(|l| l.trim_start().strip_prefix("script "))
                .filter_map(|l| l.split('#').nth(1))
                .filter_map(|n| n.trim_end_matches(':').trim().parse::<u32>().ok())
                .max()
                .unwrap_or(0);
            eprintln!("script 65535 -> file {common} line {line}, highest slot {highest}");
            // The slot is 65535 minus the range's start (10000 and up) plus 1: past every slot.
            assert!(highest < 55_000, "file {common} declares slot {highest}");
            assert_eq!(line, 1);
        }
        let dialog = dialog_of(&mut store, header, file, DialogKind::Object, 7).unwrap();
        assert_eq!(dialog.source, DialogSource::None);
    }

    // Every object of the game.
    let files: Vec<(u16, u16)> = (0u16..)
        .zip(store.header_table().unwrap())
        .map(|(h, header)| (h, header.event_file_id()))
        .collect();
    let mut seen = BTreeSet::new();
    let (mut objects, mut unreadable) = (0, Vec::new());
    let (mut shared_pairs, mut shared_figures) = (BTreeSet::new(), 0);
    // Messages under a condition (an `if` or a conditional jump, U4) and shared messages a
    // split would take (U2).
    let (mut branched, mut splittable) = (0, 0);
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
            branched += messages.iter().filter(|m| m.branch.is_some()).count();
            splittable += messages
                .iter()
                .filter(|m| m.shared_with >= 1 && m.split_refusal.is_none())
                .count();
        }
    }
    let ms = started.elapsed().as_millis();
    let (bad, pairs) = (unreadable.len(), shared_pairs.len());
    eprintln!(
        "dialog-all {game} objects={objects} unreadable={bad} shared_pairs={pairs} \
         shared_figures={shared_figures} branched={branched} splittable={splittable} ms={ms}"
    );
    assert!(unreadable.is_empty(), "{unreadable:#?}");
    // Vanilla branches with conditional jumps: some messages carry a condition.
    assert!(branched > 0, "no message carries a condition");
    if project.family == Family::Pt {
        assert!(shared_figures >= 1, "no figure shares a message");
    }
}

/// Whether the time limits are judged (`G4_TIMING_STRICT=1`); otherwise they are printed.
fn strict() -> bool {
    std::env::var_os("G4_TIMING_STRICT").is_some_and(|v| v == "1")
}

#[test]
fn vanilla_dialogs_of_the_largest_places_under_a_second() {
    let Some((_tmp, root)) = common::vanilla_copy() else {
        return;
    };
    let project = Arc::new(Project::open(&root).unwrap());
    scripts::adopt(&project, &mut |_| {}).unwrap();
    let game = &project.game_code;

    // The five headers with the most objects, each event file once (its first header).
    let mut store = Store::new(project.clone());
    let files: Vec<(u16, u16)> = (0u16..)
        .zip(store.header_table().unwrap())
        .map(|(h, header)| (h, header.event_file_id()))
        .collect();
    let mut seen = BTreeSet::new();
    let mut largest: Vec<(usize, u16)> = Vec::new();
    for (header, file) in files {
        if !seen.insert(file) {
            continue;
        }
        if let Ok(events) = store.event(file) {
            largest.push((events.object_events.len(), header));
        }
    }
    largest.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    // Figures with a local dialog and its first text (some large places hold only trainers).
    let mut spoken = 0;
    for &(objects, header) in largest.iter().take(5) {
        // A store as a session holds it: the header table read, the place not yet.
        let mut store = Store::new(project.clone());
        store.header_table().unwrap();
        let started = Instant::now();
        let place = dialogs_of_header(&mut store, header).unwrap();
        let ms = started.elapsed().as_millis();
        let figures = place.figures.len();
        eprintln!("dialogs {game} header={header} figures={figures} ms={ms}");
        let events = store.event(place.event_file).unwrap();
        assert_eq!(objects, events.object_events.len());
        assert_eq!(figures, objects + events.bg_events.len());
        for f in &place.figures {
            assert!(
                !matches!(f.source, DialogSource::Unreadable { .. }),
                "header {header}: {f:?}"
            );
            assert_eq!(usize::try_from(f.messages).unwrap(), f.message_ids.len());
        }
        spoken += place
            .figures
            .iter()
            .filter(|f| f.messages > 0 && f.first_text.is_some())
            .count();
        if strict() {
            assert!(ms <= 1_000, "header {header}: {ms} ms");
        }
    }
    assert!(spoken > 0, "nobody speaks in the largest places");
}
