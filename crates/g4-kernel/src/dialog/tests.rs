#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use super::*;
use crate::data::trainer_create::adopt_first;
use crate::scripts::ScriptSource;
use crate::store::{BgEvent, ObjectEvent};
use crate::test_support::{TextFixture, synthetic_hgss_project};
use crate::text::{TextMessage, default_message_id};

/// The text archive of header 0 of the fixture (its script file and event file are 0).
const ARCHIVE: u16 = 433;

fn object(local_id: u16, script: u16) -> ObjectEvent {
    ObjectEvent {
        local_id,
        graphics_id: 0,
        movement_type: 0,
        trainer_type: 0,
        hidden_flag: 0,
        script,
        dir: 0,
        data: [0; 3],
        movement_range_x: 0,
        movement_range_z: 0,
        x: local_id,
        z: 0,
        y: 0,
    }
}

/// Header 0 of `fx`: `source` as script file 0 (none when empty), `messages` as its text
/// archive and one object per entry of `scripts`, the entry its script number.
fn store_with(fx: &TextFixture, source: &str, messages: &[&str], scripts: &[u16]) -> Store {
    let project = fx.project();
    if !source.is_empty() {
        scripts::write_source(&project, 0, &ScriptSource::Rotom(source.into())).unwrap();
    }
    let mut st = Store::new(Arc::new(project));
    st.edit_text_archive(ARCHIVE, "messages", |doc| {
        doc.messages = messages
            .iter()
            .enumerate()
            .map(|(i, text)| TextMessage {
                id: default_message_id(i),
                text: (*text).into(),
                other: Default::default(),
            })
            .collect();
    })
    .unwrap();
    st.transaction("objects", |tx| {
        tx.edit_event(0, |e| {
            e.object_events = (0u16..).zip(scripts).map(|(i, s)| object(i, *s)).collect();
            Ok(())
        })
    })
    .unwrap();
    st
}

fn of(st: &mut Store, index: u16) -> Dialog {
    dialog_of(st, 0, 0, DialogKind::Object, index).unwrap()
}

fn indices(dialog: &Dialog) -> Vec<u16> {
    dialog.messages.iter().map(|m| m.index).collect()
}

const TEN: [&str; 10] = [
    "null", "eins", "zwei", "drei", "vier", "fünf", "sechs", "sieben", "acht", "neun",
];

#[test]
fn a_figure_with_one_message_names_archive_index_and_line() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script figur #2:\n    LockAll\n    Message 5\n    End\n",
        &TEN,
        &[2],
    );
    assert_eq!(
        of(&mut st, 0),
        Dialog {
            source: DialogSource::Local {
                file: 0,
                slot: 2,
                line: 1
            },
            messages: vec![DialogMessage {
                archive: ARCHIVE,
                index: 5,
                text: Some("fünf".into()),
                command: "Message".into(),
                line: 3,
                col: 13,
                branch: None,
                shared_with: 0,
                split_refusal: Some(not_shared()),
            }],
            followed: vec![],
        }
    );
}

#[test]
fn jumps_in_the_file_are_followed_once() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script figur #2:\n    Call hilfe\n    End\nhilfe:\n    Message 7\n    GoTo hilfe\n",
        &TEN,
        &[2],
    );
    let dialog = of(&mut st, 0);
    assert_eq!(indices(&dialog), [7]);
    assert_eq!(dialog.messages[0].line, 5);
    assert_eq!(dialog.followed, ["hilfe"]);
}

#[test]
fn the_path_without_the_jump_comes_first() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script figur #1:\n    CheckFlag 148\n    GoToIf EQUAL, spaeter\n    Message 1\n    \
         End\nspaeter:\n    Message 2\n    Call ende\n    End\n\
         ende:\n    Message 3\n    Return\n",
        &TEN,
        &[1],
    );
    let dialog = of(&mut st, 0);
    assert_eq!(indices(&dialog), [1, 2, 3]);
    assert_eq!(dialog.followed, ["spaeter", "ende"]);
}

#[test]
fn a_body_without_end_runs_on_into_the_next_label() {
    // As rotom's decompiler writes a jump target the code also reaches from above.
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script figur #1:\n    CheckFlag 148\n    GoToIf EQUAL, spaeter\n    Message 1\n\
         weiter:\n    Message 2\n    End\n\
         spaeter:\n    if FLAG_UNUSED_0x0094 then\n        Message 3\n        End\n    \
         else\n        Message 4\n        Return\n    endif\n\
         fremd:\n    Message 9\n    End\n",
        &TEN,
        &[1],
    );
    let dialog = of(&mut st, 0);
    // `weiter` comes on from the script, before the jump target; `End` and an `if … else`
    // ending on both sides stop the code, so `fremd` is never reached.
    assert_eq!(indices(&dialog), [1, 2, 3, 4]);
    assert_eq!(dialog.followed, ["spaeter", "weiter"]);
}

#[test]
fn an_archive_that_does_not_load_makes_the_dialog_unreadable() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script spricht #1:\n    Message 1\n    End\nscript stumm #2:\n    End\n",
        &TEN,
        &[1, 2],
    );
    st.transaction("archive", |tx| {
        tx.edit_header(0, |h| {
            if let uxie::map_header::MapHeader::Pt(h) = h {
                h.text_archive_id = 9999;
            }
            Ok(())
        })
    })
    .unwrap();
    let DialogSource::Unreadable { reason } = of(&mut st, 0).source else {
        panic!("not unreadable");
    };
    assert!(reason.starts_with("Text archive 9999"), "{reason}");
    // A script without messages does not need the archive.
    let quiet = of(&mut st, 1);
    assert!(matches!(quiet.source, DialogSource::Local { slot: 2, .. }));
    assert!(quiet.messages.is_empty());
}

#[test]
fn a_shared_jump_target_counts_both_figures() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script drei #3:\n    Message 1\n    Call hilfe\n    End\n\
         script sieben #7:\n    Call hilfe\n    End\n\
         hilfe:\n    Message 7\n    Return\n",
        &TEN,
        &[0, 0, 0, 3, 0, 0, 0, 7],
    );
    let three = of(&mut st, 3);
    let seven = of(&mut st, 7);
    assert_eq!(indices(&three), [1, 7]);
    assert_eq!(indices(&seven), [7]);
    // Message 1 only figure 3 shows; message 7 both.
    assert_eq!(three.messages[0].shared_with, 0);
    assert_eq!(three.messages[1].shared_with, 1);
    assert_eq!(seven.messages[0].shared_with, 1);
    assert_eq!(three.followed, ["hilfe"]);
    assert_eq!(seven.followed, ["hilfe"]);
}

#[test]
fn dialog_names_the_index_not_the_text() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script eins #1:\n    Message 1\n    End\nscript zwei #2:\n    Message 0\n    End\n",
        &["Hallo", "Hallo"],
        &[1, 2],
    );
    let dialog = of(&mut st, 0);
    assert_eq!(indices(&dialog), [1]);
    assert_eq!(dialog.messages[0].text.as_deref(), Some("Hallo"));
    // Figure 1 shows message 0 with the same text: not the same message.
    assert_eq!(dialog.messages[0].shared_with, 0);
}

#[test]
fn branches_carry_their_condition() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script figur #1:\n    if FLAG_UNUSED_0x0094 then\n        Message 1\n    \
         else\n        Message 2\n    endif\n    \
         if FLAG_UNUSED_0x0095 then\n        Message 3\n    \
         else if FLAG_UNUSED_0x0096 then\n        Message 4\n    else\n        Message 5\n    \
         endif\n    Message 6\n    End\n",
        &TEN,
        &[1],
    );
    let dialog = of(&mut st, 0);
    assert_eq!(indices(&dialog), [1, 2, 3, 4, 5, 6]);
    let branches: Vec<Option<&str>> = dialog
        .messages
        .iter()
        .map(|m| m.branch.as_deref())
        .collect();
    assert_eq!(
        branches,
        [
            Some("FLAG_UNUSED_0x0094"),
            Some("else"),
            Some("FLAG_UNUSED_0x0095"),
            Some("FLAG_UNUSED_0x0096"),
            Some("else"),
            None
        ]
    );
}

#[test]
fn a_conditional_jump_names_its_condition() {
    // UI test 3d.5 (U4): decompiled vanilla scripts branch with jump commands, not `if`.
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script figur #1:\n    CheckFlag FLAG_UNUSED_0x0094\n    GoToIf EQUAL, gesetzt\n    \
         GoToIfSet FLAG_UNUSED_0x0095, auch\n    Message 1\n    End\n\
         gesetzt:\n    Message 2\n    GoTo weiter\n\
         weiter:\n    Message 3\n    End\n\
         auch:\n    CompareVarValue VAR_UNUSED_0x4000, 5\n    CallIf 1, hilfe\n    \
         Message 4\n    End\n\
         hilfe:\n    Message 5\n    Return\n",
        &TEN,
        &[1],
    );
    let dialog = of(&mut st, 0);
    let branches: Vec<(u16, Option<&str>)> = dialog
        .messages
        .iter()
        .map(|m| (m.index, m.branch.as_deref()))
        .collect();
    let flag = "CheckFlag FLAG_UNUSED_0x0094; GoToIf EQUAL";
    let set = "GoToIfSet FLAG_UNUSED_0x0095";
    assert_eq!(
        branches,
        [
            // The path without the jumps.
            (1, None),
            (2, Some(flag)),
            // The code after a comparison runs on under the jump that reached it.
            (4, Some(set)),
            // An unconditional jump keeps the condition it was reached under.
            (3, Some(flag)),
            (5, Some("CompareVarValue VAR_UNUSED_0x4000, 5; CallIf 1")),
        ]
    );
}

#[test]
fn hgss_jump_if_names_its_condition() {
    let fixture = synthetic_hgss_project(&["Nowhere"], 2);
    let project = Project::open(fixture.dir.path()).unwrap();
    let path = scripts::source_path(&project, 0, SourceKind::Script);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "script figur #1:\n    CheckFlag 100\n    JumpIf EQUAL, gesetzt\n    \
         NPCMsg 2\n    End\n\
         gesetzt:\n    NPCMsg 3\n    End\n",
    )
    .unwrap();
    let messages = messages_of_slot(&project, Family::Hgss, 0, 1, 9).unwrap();
    let branches: Vec<(u16, Option<&str>)> = messages
        .iter()
        .map(|m| (m.index, m.branch.as_deref()))
        .collect();
    assert_eq!(
        branches,
        [(2, None), (3, Some("CheckFlag 100; JumpIf EQUAL"))]
    );
}

#[test]
fn common_trainer_and_none_scripts_have_no_messages() {
    let fx = TextFixture::exported();
    let mut st = store_with(&fx, "", &TEN, &[2004, 3005, 0, 5001, 65535]);
    for (index, source) in [
        (0, DialogSource::Common { script: 2004 }),
        (1, DialogSource::Trainer { id: 6 }),
        (2, DialogSource::None),
        // The partner range: the same trainer as a double battle partner.
        (3, DialogSource::Trainer { id: 2 }),
        // 0xFFFF: no script (UI test 3d.5, U5), not a common script.
        (4, DialogSource::None),
    ] {
        let dialog = of(&mut st, index);
        assert_eq!(dialog.source, source);
        assert!(dialog.messages.is_empty() && dialog.followed.is_empty());
    }
}

#[test]
fn a_binary_script_file_is_unreadable_with_a_reason() {
    // Not adopted: the reason is the request to adopt.
    let fx = TextFixture::dspre();
    let mut st = Store::new(Arc::new(fx.project()));
    st.transaction("objects", |tx| {
        tx.edit_event(0, |e| {
            e.object_events = vec![object(0, 1)];
            Ok(())
        })
    })
    .unwrap();
    let DialogSource::Unreadable { reason } = of(&mut st, 0).source else {
        panic!("not unreadable");
    };
    assert!(reason.starts_with(&adopt_first()), "{reason}");

    // Adopted, but the file has no rotom source.
    let fx = TextFixture::exported();
    let project = fx.project();
    std::fs::remove_file(scripts::source_path(&project, 0, SourceKind::Script)).unwrap();
    let mut st = store_with(&fx, "", &TEN, &[1]);
    let dialog = of(&mut st, 0);
    let DialogSource::Unreadable { reason } = &dialog.source else {
        panic!("{dialog:?}");
    };
    assert!(reason.contains("0000"), "{reason}");
    assert!(dialog.messages.is_empty());
}

#[test]
fn a_sign_reads_its_script_and_is_not_counted_as_a_figure() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script schild #1:\n    Message 4\n    End\n",
        &TEN,
        &[1],
    );
    st.transaction("bg", |tx| {
        tx.edit_event(0, |e| {
            e.bg_events = vec![BgEvent {
                script: 1,
                event_type: 0,
                x: 0,
                z: 0,
                y: 0,
                player_facing_dir: 0,
            }];
            Ok(())
        })
    })
    .unwrap();
    let sign = dialog_of(&mut st, 0, 0, DialogKind::Bg, 0).unwrap();
    assert_eq!(indices(&sign), [4]);
    // Object 0 runs the same script: one other figure; the sign is none.
    assert_eq!(sign.messages[0].shared_with, 1);
    assert_eq!(of(&mut st, 0).messages[0].shared_with, 0);
    // An event that does not exist is an error.
    assert!(dialog_of(&mut st, 0, 0, DialogKind::Bg, 1).is_err());
}

#[test]
fn hgss_counts_npcmsg_and_jump() {
    let fixture = synthetic_hgss_project(&["Nowhere"], 2);
    let project = Project::open(fixture.dir.path()).unwrap();
    assert_eq!(project.family, Family::Hgss);
    let path = scripts::source_path(&project, 0, SourceKind::Script);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "script figur #1:\n    NPCMsg 2\n    Jump ende\n    End\n\
         ende:\n    NonNPCMsg 3\n    JumpIf 1, weiter\n    End\n\
         weiter:\n    NPCMsg 4\n    End\n",
    )
    .unwrap();
    let messages = messages_of_slot(&project, Family::Hgss, 0, 1, 9).unwrap();
    let shown: Vec<(u16, u16, &str)> = messages
        .iter()
        .map(|m| (m.archive, m.index, m.command.as_str()))
        .collect();
    assert_eq!(
        shown,
        [(9, 2, "NPCMsg"), (9, 3, "NonNPCMsg"), (9, 4, "NPCMsg")]
    );
}

// -- split_message ------------------------------------------------------------------

fn archive_bytes(st: &Store) -> Vec<u8> {
    st.current_bytes(crate::project::AssetId {
        kind: crate::project::AssetKind::TextArchive,
        index: ARCHIVE,
    })
    .unwrap()
    .unwrap()
}

fn source_of(st: &Store) -> String {
    scripts::rotom_source(st.project(), 0).unwrap()
}

fn refused(result: Result<SplitReport, KernelError>) -> String {
    match result {
        Err(KernelError::Refused(reason)) => reason,
        other => panic!("not refused: {other:?}"),
    }
}

#[test]
fn split_copies_the_message_and_rewrites_this_figures_uses_as_one_step() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script drei #3:\n    Message 5\n    WaitButton\n    Message 5\n    End\n\
         script sieben #7:\n    Message 5\n    End\n",
        &TEN,
        &[0, 0, 0, 3, 0, 0, 0, 7],
    );
    let depth = st.undo_depth();
    let (source, archive) = (source_of(&st), archive_bytes(&st));
    let member = scripts::snapshot_script(st.project(), 0).unwrap();

    let report = split_message(&mut st, 0, 0, 3, ARCHIVE, 5).unwrap();
    assert_eq!(
        (report.archive, report.old_index, report.new_index),
        (ARCHIVE, 5, 10)
    );
    assert_eq!(report.lines, [2, 4]);
    assert!(report.change.is_some());
    let doc = st.text_archive(ARCHIVE).unwrap();
    assert_eq!(doc.messages.len(), 11);
    assert_eq!(doc.messages[10].text, "fünf");
    // Figure 3 shows the copy (both uses, the number grew a digit), figure 7 the original.
    let (three, seven) = (of(&mut st, 3), of(&mut st, 7));
    assert_eq!(indices(&three), [10, 10]);
    assert_eq!(indices(&seven), [5]);
    assert!(three.messages.iter().all(|m| m.shared_with == 0));
    assert_eq!(seven.messages[0].shared_with, 0);
    assert_eq!(
        source_of(&st),
        "script drei #3:\n    Message 10\n    WaitButton\n    Message 10\n    End\n\
         script sieben #7:\n    Message 5\n    End\n"
    );
    // One step with the figure's local id.
    assert_eq!(st.undo_depth(), depth + 1);
    assert_eq!(
        st.history().0.last().map(String::as_str),
        Some("Dialog of character 3 split off")
    );
    let (source_after, archive_after) = (source_of(&st), archive_bytes(&st));
    let member_after = scripts::snapshot_script(st.project(), 0).unwrap();
    assert_ne!(member_after.member, member.member);

    st.undo().unwrap();
    assert_eq!(source_of(&st), source);
    assert_eq!(archive_bytes(&st), archive);
    assert_eq!(
        scripts::snapshot_script(st.project(), 0).unwrap().member,
        member.member
    );
    assert_eq!(st.undo_depth(), depth);

    st.redo().unwrap();
    assert_eq!(source_of(&st), source_after);
    assert_eq!(archive_bytes(&st), archive_after);
    assert_eq!(
        scripts::snapshot_script(st.project(), 0).unwrap().member,
        member_after.member
    );
}

#[test]
fn split_refuses_a_message_in_a_shared_label() {
    let fx = TextFixture::exported();
    // `hilfe` is reached from both scripts: its number stays.
    let shared = "script drei #3:\n    Message 1\n    Call hilfe\n    End\n\
                  script sieben #7:\n    Call hilfe\n    End\n\
                  hilfe:\n    Message 7\n    Return\n";
    let mut st = store_with(&fx, shared, &TEN, &[0, 0, 0, 3, 0, 0, 0, 7]);
    let (depth, archive) = (st.undo_depth(), archive_bytes(&st));
    assert_eq!(
        refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 7)),
        shared_label()
    );
    assert_eq!(st.undo_depth(), depth);
    assert_eq!(archive_bytes(&st), archive);
    assert_eq!(source_of(&st), shared);

    // Only figure 3 reaches `hilfe`; figure 7 shows message 7 in its own body: `hilfe`
    // gets the copy, figure 7 keeps the original.
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script drei #3:\n    Call hilfe\n    End\n\
         script sieben #7:\n    Message 7\n    End\n\
         hilfe:\n    Message 7\n    Return\n",
        &TEN,
        &[0, 0, 0, 3, 0, 0, 0, 7],
    );
    let report = split_message(&mut st, 0, 0, 3, ARCHIVE, 7).unwrap();
    assert_eq!((report.new_index, report.lines.as_slice()), (10, &[8][..]));
    assert_eq!(indices(&of(&mut st, 3)), [10]);
    assert_eq!(indices(&of(&mut st, 7)), [7]);

    // A label the code of another script runs on into is shared as well.
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script drei #3:\n    Call hilfe\n    End\n\
         script sieben #7:\n    Message 7\n\
         hilfe:\n    Message 7\n    Return\n",
        &TEN,
        &[0, 0, 0, 3, 0, 0, 0, 7],
    );
    assert_eq!(
        refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 7)),
        shared_label()
    );
}

/// The refusal `dialog_of` names for message `index` of `dialog` (its first use).
fn refusal_of(dialog: &Dialog, index: u16) -> Option<String> {
    dialog
        .messages
        .iter()
        .find(|m| m.index == index)
        .unwrap()
        .split_refusal
        .clone()
}

#[test]
fn the_dialog_says_why_a_split_would_be_refused() {
    // UI test 3d.5 (U2): the editor offered a split the kernel refused; each message now
    // carries the refusal `split_message` gives, found by the same rules.
    let fx = TextFixture::exported();
    let source = "script drei #3:\n    Message 1\n    Message 5\n    Message 12\n    \
                  Call hilfe\n    End\n\
                  script sieben #7:\n    Message 5\n    Message 12\n    Call hilfe\n    End\n\
                  hilfe:\n    Message 2\n    Return\n";
    let mut st = store_with(&fx, source, &TEN, &[0, 0, 0, 3, 0, 0, 0, 7]);
    let three = of(&mut st, 3);
    assert_eq!(refusal_of(&three, 1), Some(not_shared()));
    assert_eq!(refusal_of(&three, 12), Some(missing_message(ARCHIVE, 12)));
    assert_eq!(refusal_of(&three, 2), Some(shared_label()));
    assert_eq!(refusal_of(&three, 5), None);
    for message in [1, 12, 2] {
        assert_eq!(
            Some(refused(split_message(&mut st, 0, 0, 3, ARCHIVE, message))),
            refusal_of(&three, message)
        );
    }
    assert!(split_message(&mut st, 0, 0, 3, ARCHIVE, 5).is_ok());

    // Two figures run the same script.
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script drei #3:\n    Message 5\n    End\n",
        &TEN,
        &[0, 0, 0, 3, 3],
    );
    let three = of(&mut st, 3);
    assert_eq!(refusal_of(&three, 5), Some(shared_script()));
    assert_eq!(
        Some(refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 5))),
        refusal_of(&three, 5)
    );

    // The copy would not fit the command's u8.
    let fx = TextFixture::exported();
    let texts: Vec<String> = (0..256).map(|i| format!("Text {i}")).collect();
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    let source = "script drei #3:\n    Message 5\n    End\n\
                  script sieben #7:\n    Message 5\n    End\n";
    let mut st = store_with(&fx, source, &texts, &[0, 0, 0, 3, 0, 0, 0, 7]);
    let three = of(&mut st, 3);
    let reason = refusal_of(&three, 5).unwrap();
    assert!(reason.starts_with(&number_too_high()), "{reason}");
    assert_eq!(refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 5)), reason);

    // A sign gets no copy.
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script schild #1:\n    Message 4\n    End\n",
        &TEN,
        &[1],
    );
    st.transaction("bg", |tx| {
        tx.edit_event(0, |e| {
            e.bg_events = vec![BgEvent {
                script: 1,
                event_type: 0,
                x: 0,
                z: 0,
                y: 0,
                player_facing_dir: 0,
            }];
            Ok(())
        })
    })
    .unwrap();
    let sign = dialog_of(&mut st, 0, 0, DialogKind::Bg, 0).unwrap();
    assert_eq!(refusal_of(&sign, 4), Some(not_an_object()));
}

#[test]
fn split_refuses_an_unshared_message() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script drei #3:\n    Message 1\n    Message 5\n    End\n\
         script sieben #7:\n    Message 5\n    End\n",
        &TEN,
        &[0, 0, 0, 3, 0, 0, 0, 7],
    );
    let depth = st.undo_depth();
    assert_eq!(
        refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 1)),
        not_shared()
    );
    // A message the figure does not show, another archive and a script two figures run
    // are refused as well; nothing is recorded.
    let reason = refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 4));
    assert!(reason.contains("message 4"), "{reason}");
    let reason = refused(split_message(&mut st, 0, 0, 3, ARCHIVE + 1, 5));
    assert!(reason.contains("message 5"), "{reason}");
    assert_eq!(st.undo_depth(), depth);

    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script drei #3:\n    Message 5\n    End\n",
        &TEN,
        &[0, 0, 0, 3, 3],
    );
    assert_eq!(
        refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 5)),
        shared_script()
    );
}

#[test]
fn split_refuses_a_message_the_archive_does_not_have() {
    let fx = TextFixture::exported();
    // Both figures show message 12; the archive has ten messages.
    let source = "script drei #3:\n    Message 12\n    End\n\
                  script sieben #7:\n    Message 12\n    End\n";
    let mut st = store_with(&fx, source, &TEN, &[0, 0, 0, 3, 0, 0, 0, 7]);
    let three = of(&mut st, 3);
    assert_eq!(three.messages[0].text, None);
    assert_eq!(three.messages[0].shared_with, 1);
    let (depth, archive) = (st.undo_depth(), archive_bytes(&st));
    assert_eq!(
        refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 12)),
        missing_message(ARCHIVE, 12)
    );
    assert_eq!(st.undo_depth(), depth);
    assert_eq!(archive_bytes(&st), archive);
    assert_eq!(source_of(&st), source);
}

#[test]
fn split_refuses_when_another_script_jumps_into_this_figures_body() {
    let fx = TextFixture::exported();
    // Figure 7 jumps into figure 3's own body: rewriting it would change figure 7's dialog.
    let source = "script drei #3:\n    Message 5\n    End\n\
                  script sieben #7:\n    Message 1\n    GoTo drei\n";
    let mut st = store_with(&fx, source, &TEN, &[0, 0, 0, 3, 0, 0, 0, 7]);
    assert_eq!(indices(&of(&mut st, 7)), [1, 5]);
    let (depth, archive) = (st.undo_depth(), archive_bytes(&st));
    assert_eq!(
        refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 5)),
        shared_label()
    );
    assert_eq!(st.undo_depth(), depth);
    assert_eq!(archive_bytes(&st), archive);
    assert_eq!(source_of(&st), source);

    // Code of another script that runs on into figure 3's body shares it as well.
    let fx = TextFixture::exported();
    let source = "script sieben #7:\n    Message 1\n\
                  script drei #3:\n    Message 5\n    End\n";
    let mut st = store_with(&fx, source, &TEN, &[0, 0, 0, 3, 0, 0, 0, 7]);
    assert_eq!(
        refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 5)),
        shared_label()
    );
    assert_eq!(source_of(&st), source);
}

#[test]
fn a_compile_error_leaves_no_half_step() {
    let fx = TextFixture::exported();
    // Script #9 does not compile; the source parses, so the dialogs read.
    let source = "script drei #3:\n    Message 5\n    End\n\
                  script sieben #7:\n    Message 5\n    End\n\
                  script kaputt #9:\n    GibtEsNicht 1\n    End\n";
    let mut st = store_with(&fx, source, &TEN, &[0, 0, 0, 3, 0, 0, 0, 7]);
    let (depth, archive) = (st.undo_depth(), archive_bytes(&st));
    let before = scripts::snapshot_script(st.project(), 0).unwrap();
    let state = scripts::snapshot_compile_state(st.project()).unwrap();
    let err = split_message(&mut st, 0, 0, 3, ARCHIVE, 5).unwrap_err();
    assert!(matches!(err, KernelError::Compile(_)), "{err:?}");
    assert_eq!(st.undo_depth(), depth);
    assert_eq!(st.history().1.len(), 0, "nothing to redo");
    assert_eq!(archive_bytes(&st), archive);
    let after = scripts::snapshot_script(st.project(), 0).unwrap();
    assert_eq!((after.rotom, after.member), (before.rotom, before.member));
    // The failed compile rewrote rotom's compile state (the source dirty): put back too.
    assert_eq!(
        scripts::snapshot_compile_state(st.project()).unwrap(),
        state
    );
}

#[test]
fn a_copy_past_the_commands_message_type_is_refused() {
    let fx = TextFixture::exported();
    // Platinum's `Message` takes its number as a u8 (rotom's database): 0 to 255.
    let texts: Vec<String> = (0..256).map(|i| format!("Text {i}")).collect();
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    let source = "script drei #3:\n    Message 5\n    End\n\
                  script sieben #7:\n    Message 5\n    End\n";
    let mut st = store_with(&fx, source, &texts, &[0, 0, 0, 3, 0, 0, 0, 7]);
    let (depth, archive) = (st.undo_depth(), archive_bytes(&st));
    let reason = refused(split_message(&mut st, 0, 0, 3, ARCHIVE, 5));
    assert!(reason.starts_with(&number_too_high()), "{reason}");
    assert!(reason.contains("255") && reason.contains("256"), "{reason}");
    assert_eq!(st.undo_depth(), depth);
    assert_eq!(archive_bytes(&st), archive);
    assert_eq!(source_of(&st), source);

    // One message fewer: the copy takes 255, the last number that fits.
    st.edit_text_archive(ARCHIVE, "kürzer", |doc| {
        doc.messages.pop();
    })
    .unwrap();
    let report = split_message(&mut st, 0, 0, 3, ARCHIVE, 5).unwrap();
    assert_eq!(report.new_index, 255);
    assert_eq!(indices(&of(&mut st, 3)), [255]);
}

// -- dialogs_of_header ----------------------------------------------------------------

/// A sign running script `script`.
fn sign(script: u16) -> BgEvent {
    BgEvent {
        script,
        event_type: 0,
        x: 0,
        z: 0,
        y: 0,
        player_facing_dir: 0,
    }
}

/// Points header 0 at event file `file`.
fn header_events(st: &mut Store, file: u16) {
    st.transaction("event file", |tx| {
        tx.edit_header(0, |h| {
            if let uxie::map_header::MapHeader::Pt(h) = h {
                h.event_file_id = file;
            }
            Ok(())
        })
    })
    .unwrap();
}

/// A figure without messages: `source` as given, the rest empty.
fn quiet(kind: DialogKind, index: u16, source: DialogSource) -> FigureDialog {
    FigureDialog {
        kind,
        index,
        local_id: (kind == DialogKind::Object).then_some(index),
        graphics_id: (kind == DialogKind::Object).then_some(0),
        trainer: None,
        source,
        first_text: None,
        messages: 0,
        pages: 0,
        message_ids: vec![],
    }
}

#[test]
fn dialogs_of_header_lists_objects_then_signs_with_first_text_and_counts() {
    let fx = TextFixture::exported();
    // Figure 0 shows message 2 twice (counted once), then 3 and 12 (the archive has no 12);
    // the sign shows message 4.
    let mut texts = TEN.to_vec();
    texts[2] = "A\\nB\\rC";
    texts[3] = "Eins\\nZwei\\nDrei";
    let mut st = store_with(
        &fx,
        "script figur #1:\n    Message 2\n    WaitButton\n    Message 3\n    Message 2\n    \
         Message 12\n    End\n\
         script schild #4:\n    Message 4\n    End\n",
        &texts,
        &[1, 0, 2004],
    );
    st.transaction("bg", |tx| {
        tx.edit_event(0, |e| {
            e.object_events[0].graphics_id = 91;
            e.object_events[0].local_id = 5;
            e.bg_events = vec![sign(4)];
            Ok(())
        })
    })
    .unwrap();

    let place = dialogs_of_header(&mut st, 0).unwrap();
    assert_eq!(
        (place.header, place.event_file, place.archive),
        (0, 0, ARCHIVE)
    );
    assert_eq!(
        place.figures,
        [
            FigureDialog {
                kind: DialogKind::Object,
                index: 0,
                local_id: Some(5),
                graphics_id: Some(91),
                trainer: None,
                source: DialogSource::Local {
                    file: 0,
                    slot: 1,
                    line: 1
                },
                first_text: Some("A\\nB\\rC".into()),
                messages: 3,
                // Two pages for message 2, two for message 3 (the third line overflows), none
                // for the missing 12.
                pages: 4,
                message_ids: vec![2, 3, 12],
            },
            quiet(DialogKind::Object, 1, DialogSource::None),
            quiet(DialogKind::Object, 2, DialogSource::Common { script: 2004 }),
            FigureDialog {
                kind: DialogKind::Bg,
                index: 0,
                local_id: None,
                graphics_id: None,
                trainer: None,
                source: DialogSource::Local {
                    file: 0,
                    slot: 4,
                    line: 8
                },
                first_text: Some("vier".into()),
                messages: 1,
                pages: 1,
                message_ids: vec![4],
            },
        ]
    );

    // The same source and messages as `dialog_of` names for each figure.
    for figure in &place.figures {
        let dialog = dialog_of(&mut st, 0, 0, figure.kind, figure.index).unwrap();
        assert_eq!(dialog.source, figure.source);
        let mut ids: Vec<u16> = Vec::new();
        for m in &dialog.messages {
            if !ids.contains(&m.index) {
                ids.push(m.index);
            }
        }
        assert_eq!(ids, figure.message_ids);
    }

    // A header that does not exist is an error.
    assert!(dialogs_of_header(&mut st, 99).is_err());
}

#[test]
fn a_header_without_events_has_no_figures() {
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script figur #1:\n    Message 2\n    End\n",
        &TEN,
        &[1],
    );
    // Event file 1 of the fixture has no events.
    header_events(&mut st, 1);
    let place = dialogs_of_header(&mut st, 0).unwrap();
    assert_eq!((place.event_file, place.figures.len()), (1, 0));

    // An event file that does not exist: no figures, no error.
    header_events(&mut st, 999);
    let place = dialogs_of_header(&mut st, 0).unwrap();
    assert_eq!((place.event_file, place.figures.len()), (999, 0));

    // A locked event file (shown, never written) has none either.
    header_events(&mut st, 0);
    assert_eq!(dialogs_of_header(&mut st, 0).unwrap().figures.len(), 1);
    st.save().unwrap();
    st.lock(
        AssetId {
            kind: AssetKind::Event,
            index: 0,
        },
        "Test",
    )
    .unwrap();
    assert_eq!(st.event(0).unwrap().object_events.len(), 1);
    assert!(dialogs_of_header(&mut st, 0).unwrap().figures.is_empty());
}

#[test]
fn a_trainer_figure_names_its_trainer() {
    let fx = TextFixture::exported();
    let mut st = store_with(&fx, "", &TEN, &[3005, 5001]);
    let place = dialogs_of_header(&mut st, 0).unwrap();
    let trainer = |index: u16, id: u16| FigureDialog {
        trainer: Some(id),
        ..quiet(DialogKind::Object, index, DialogSource::Trainer { id })
    };
    // The partner range names the same trainer as a double battle partner.
    assert_eq!(place.figures, [trainer(0, 6), trainer(1, 2)]);
}

#[test]
fn a_common_script_figure_has_source_common_and_no_text() {
    let fx = TextFixture::exported();
    let mut st = store_with(&fx, "", &TEN, &[2004, 65535]);
    let place = dialogs_of_header(&mut st, 0).unwrap();
    assert_eq!(
        place.figures,
        [
            quiet(DialogKind::Object, 0, DialogSource::Common { script: 2004 }),
            // 0xFFFF: no script, like 0.
            quiet(DialogKind::Object, 1, DialogSource::None),
        ]
    );
}

#[test]
fn an_unreadable_dialog_is_a_figure_not_an_error() {
    // Not adopted: every local figure names the request to adopt.
    let fx = TextFixture::dspre();
    let mut st = Store::new(Arc::new(fx.project()));
    st.transaction("objects", |tx| {
        tx.edit_event(0, |e| {
            e.object_events = vec![object(0, 1), object(1, 3005)];
            Ok(())
        })
    })
    .unwrap();
    let place = dialogs_of_header(&mut st, 0).unwrap();
    let DialogSource::Unreadable { reason } = &place.figures[0].source else {
        panic!("{place:?}");
    };
    assert!(reason.starts_with(&adopt_first()), "{reason}");
    assert_eq!(place.figures[1].trainer, Some(6));

    // A slot the file does not declare, and an archive that does not load for a figure with
    // messages; a figure without messages does not need the archive.
    let fx = TextFixture::exported();
    let mut st = store_with(
        &fx,
        "script spricht #1:\n    Message 1\n    End\nscript stumm #2:\n    End\n",
        &TEN,
        &[1, 2, 7],
    );
    st.transaction("archive", |tx| {
        tx.edit_header(0, |h| {
            if let uxie::map_header::MapHeader::Pt(h) = h {
                h.text_archive_id = 9999;
            }
            Ok(())
        })
    })
    .unwrap();
    let place = dialogs_of_header(&mut st, 0).unwrap();
    let sources: Vec<&DialogSource> = place.figures.iter().map(|f| &f.source).collect();
    assert!(
        matches!(sources[0], DialogSource::Unreadable { reason } if reason.starts_with("Text archive 9999")),
        "{sources:?}"
    );
    assert!(matches!(sources[1], DialogSource::Local { slot: 2, .. }));
    assert!(
        matches!(sources[2], DialogSource::Unreadable { reason } if reason.contains("#7")),
        "{sources:?}"
    );
    assert!(place.figures.iter().all(|f| f.message_ids.is_empty()));
}
