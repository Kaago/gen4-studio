//! The dialog of a figure (phase 3d.5): the messages an object's or a sign's script shows, and
//! giving a figure its own copy of a message it shares with others. The kernel does the work
//! ([`g4_kernel::dialog`]); `split_message` writes and compiles the script file (a file step),
//! so the reference index and the checks' last report are refreshed after it. Since 3d.8 the
//! figures of a place with their dialogs in one call (`dialogs_of_header`).

use g4_kernel::dialog::{self, Dialog, DialogKind, HeaderDialogs, SplitReport};
use tauri::Manager;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// The dialog of event `index` of class `kind` in event file `event_file`, looked up on header
/// `header` (its script file and text archive), under the session lock. Reads only; an event
/// that does not exist is refused.
pub fn dialog_of_impl(
    state: &AppState,
    header: u16,
    event_file: u16,
    kind: DialogKind,
    index: u16,
) -> Result<Dialog, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(dialog::dialog_of(
        &mut session.store,
        header,
        event_file,
        kind,
        index,
    )?)
}

/// Gives object `index` of `event_file` its own copy of message (`archive`, `message`), as
/// one undo step „Dialog von Figur k abgespalten“ (the copy in the archive, the script file
/// written and compiled as a file step). The script source is written past the store, so the
/// reference index (its script sources at once) and the checks' last report are refreshed,
/// also after a refusal past the first write (the kernel rolls back, the refresh is cheap).
pub fn split_message_impl(
    state: &AppState,
    header: u16,
    event_file: u16,
    index: u16,
    archive: u16,
    message: u16,
) -> Result<SplitReport, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    session.writable()?;
    let report = dialog::split_message(
        &mut session.store,
        header,
        event_file,
        index,
        archive,
        message,
    );
    session.after_write();
    session.files_written(&[]);
    session.scripts_written();
    Ok(report?)
}

/// Every figure (objects, then signs) of header `header`'s event file with its dialog's
/// source, first text and counts, under the session lock. Reads only; a header that does not
/// exist is refused.
pub fn dialogs_of_header_impl(state: &AppState, header: u16) -> Result<HeaderDialogs, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    Ok(dialog::dialogs_of_header(&mut session.store, header)?)
}

/// Parses the figure's script (and every event file of the headers using the archive, for
/// `shared_with`), so it runs on a blocking worker.
#[tauri::command]
pub async fn dialog_of(
    app: tauri::AppHandle,
    header: u16,
    event_file: u16,
    kind: DialogKind,
    index: u16,
) -> Result<Dialog, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        dialog_of_impl(&app.state::<AppState>(), header, event_file, kind, index)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

/// Writes and compiles the script file (seconds), so it runs on a blocking worker.
#[tauri::command]
pub async fn split_message(
    app: tauri::AppHandle,
    header: u16,
    event_file: u16,
    index: u16,
    archive: u16,
    message: u16,
) -> Result<SplitReport, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        split_message_impl(
            &app.state::<AppState>(),
            header,
            event_file,
            index,
            archive,
            message,
        )
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

/// Parses the header's script file and reads its event file and text archive, so it runs on
/// a blocking worker.
#[tauri::command]
pub async fn dialogs_of_header(
    app: tauri::AppHandle,
    header: u16,
) -> Result<HeaderDialogs, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        dialogs_of_header_impl(&app.state::<AppState>(), header)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::{history_impl, open_project_impl, undo_impl};
    use super::*;
    use crate::commands::dispatch::{Reply, dispatch};
    use crate::commands::policy::UserPolicy;
    use g4_kernel::dialog::DialogSource;
    use g4_kernel::scripts::{self, ScriptSource};
    use g4_kernel::store::ObjectEvent;
    use g4_kernel::test_support::TextFixture;
    use g4_kernel::text::{TextMessage, default_message_id};
    use serde_json::json;

    /// The text archive of header 0 of the fixture (its script file and event file are 0).
    const ARCHIVE: u16 = 433;

    /// Figure 3 shows message 5 twice, figure 7 once: the message is shared.
    const SOURCE: &str = "script drei #3:\n    Message 5\n    WaitButton\n    Message 5\n    End\n\
                          script sieben #7:\n    Message 5\n    End\n";

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

    /// The synthetic text project (exported, scripts adopted) with [`SOURCE`] as script file
    /// 0, ten messages in archive 433 and objects 0 to 7 in event file 0 (object 3 runs script
    /// 3, object 7 script 7), open in a fresh state. The setup is one undo step each.
    fn open_with_figures() -> (TextFixture, AppState) {
        let fx = TextFixture::exported();
        scripts::write_source(&fx.project(), 0, &ScriptSource::Rotom(SOURCE.into())).unwrap();
        let state = AppState::default();
        open_project_impl(&state, fx.root().to_str().unwrap()).unwrap();
        {
            let mut guard = lock(&state).unwrap();
            let store = &mut session(&mut guard).unwrap().store;
            store
                .edit_text_archive(ARCHIVE, "messages", |doc| {
                    doc.messages = (0..10)
                        .map(|i| TextMessage {
                            id: default_message_id(i),
                            text: format!("Nachricht {i}"),
                            other: Default::default(),
                        })
                        .collect();
                })
                .unwrap();
            store
                .transaction("objects", |tx| {
                    tx.edit_event(0, |e| {
                        e.object_events = [0, 0, 0, 3, 0, 0, 0, 7]
                            .into_iter()
                            .enumerate()
                            .map(|(i, s)| object(i as u16, s))
                            .collect();
                        Ok(())
                    })
                })
                .unwrap();
        }
        (fx, state)
    }

    fn json_of(reply: Reply) -> serde_json::Value {
        match reply {
            Reply::Json(v) => v,
            Reply::Bytes(_) => panic!("expected JSON"),
        }
    }

    #[test]
    fn dialog_of_over_the_command_path() {
        let (fx, state) = open_with_figures();
        let dialog = dialog_of_impl(&state, 0, 0, DialogKind::Object, 3).unwrap();
        assert_eq!(
            dialog.source,
            DialogSource::Local {
                file: 0,
                slot: 3,
                line: 1
            }
        );
        let shown: Vec<(u16, u16, u32, u32)> = dialog
            .messages
            .iter()
            .map(|m| (m.archive, m.index, m.line, m.shared_with))
            .collect();
        assert_eq!(shown, [(ARCHIVE, 5, 2, 1), (ARCHIVE, 5, 4, 1)]);
        assert_eq!(dialog.messages[0].text.as_deref(), Some("Nachricht 5"));
        // A figure without a script says nothing.
        let none = dialog_of_impl(&state, 0, 0, DialogKind::Object, 0).unwrap();
        assert_eq!((none.source, none.messages.len()), (DialogSource::None, 0));

        // The dispatcher takes the keys `api.dialog.ts` sends and answers the documented JSON.
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        let answer = json_of(
            dispatch(
                &state,
                &policy,
                "dialog_of",
                json!({ "header": 0, "eventFile": 0, "kind": "object", "index": 7 }),
            )
            .unwrap(),
        );
        assert_eq!(
            answer["source"],
            json!({ "kind": "local", "file": 0, "slot": 7, "line": 6 }),
            "{answer}"
        );
        assert_eq!(
            answer["messages"],
            json!([{ "archive": ARCHIVE, "index": 5, "text": "Nachricht 5",
                     "command": "Message", "line": 7, "col": 13, "branch": null,
                     "shared_with": 1, "split_refusal": null }]),
            "{answer}"
        );
        assert_eq!(answer["followed"], json!([]));
        // Reading changed nothing.
        assert_eq!(history_impl(&state).unwrap().undo.len(), 2);
    }

    #[test]
    fn dialogs_of_header_over_the_command_path() {
        let (fx, state) = open_with_figures();
        let place = dialogs_of_header_impl(&state, 0).unwrap();
        assert_eq!(place.figures.len(), 8);
        // The dispatcher takes the key `api.dialog.ts` sends and answers the documented JSON.
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        let answer = json_of(
            dispatch(&state, &policy, "dialogs_of_header", json!({ "header": 0 })).unwrap(),
        );
        assert_eq!(
            (&answer["header"], &answer["event_file"], &answer["archive"]),
            (&json!(0), &json!(0), &json!(ARCHIVE)),
            "{answer}"
        );
        assert!(answer["ms"].is_u64(), "{answer}");
        assert_eq!(
            answer["figures"][3],
            json!({ "kind": "object", "index": 3, "local_id": 3, "graphics_id": 0,
                    "trainer": null, "source": { "kind": "local", "file": 0, "slot": 3, "line": 1 },
                    "first_text": "Nachricht 5", "messages": 1, "pages": 1, "message_ids": [5] }),
            "{answer}"
        );
        assert_eq!(answer["figures"][0]["source"], json!({ "kind": "none" }));
        // A header that does not exist is refused; reading changed nothing.
        assert!(dialogs_of_header_impl(&state, 99).is_err());
        assert_eq!(history_impl(&state).unwrap().undo.len(), 2);
    }

    #[test]
    fn split_message_is_one_undo_step_over_the_command_path() {
        let (fx, state) = open_with_figures();
        let undo = history_impl(&state).unwrap().undo.len();
        let policy = UserPolicy::new(fx.root(), &[], None).unwrap();
        let report = json_of(
            dispatch(
                &state,
                &policy,
                "split_message",
                json!({ "header": 0, "eventFile": 0, "index": 3, "archive": ARCHIVE,
                        "message": 5 }),
            )
            .unwrap(),
        );
        assert_eq!(
            (
                &report["archive"],
                &report["old_index"],
                &report["new_index"],
                &report["lines"]
            ),
            (&json!(ARCHIVE), &json!(5), &json!(10), &json!([2, 4])),
            "{report}"
        );
        assert!(report["change"].is_u64(), "{report}");
        let history = history_impl(&state).unwrap();
        assert_eq!(history.undo.len(), undo + 1);
        assert_eq!(
            history.undo.last().map(String::as_str),
            Some("Dialog of character 3 split off")
        );
        let indices = |state: &AppState, figure: u16| -> Vec<u16> {
            dialog_of_impl(state, 0, 0, DialogKind::Object, figure)
                .unwrap()
                .messages
                .iter()
                .map(|m| m.index)
                .collect()
        };
        assert_eq!(indices(&state, 3), [10, 10]);
        assert_eq!(indices(&state, 7), [5]);
        let source = || match scripts::read_source(&fx.project(), 0).unwrap() {
            ScriptSource::Rotom(text) => text,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            source(),
            SOURCE.replacen("Message 5", "Message 10", 2),
            "only figure 3's script is rewritten"
        );

        // One undo takes the copy and the source back.
        undo_impl(&state).unwrap();
        assert_eq!(history_impl(&state).unwrap().undo.len(), undo);
        assert_eq!(indices(&state, 3), [5, 5]);
        assert_eq!(source(), SOURCE);

        // A message the figure does not show is refused with the kernel's sentence; nothing
        // is recorded.
        let refused = split_message_impl(&state, 0, 0, 3, ARCHIVE, 4).unwrap_err();
        assert_eq!(
            refused.message,
            "Object 3 does not show message 4 of text archive 433."
        );
        assert_eq!(history_impl(&state).unwrap().undo.len(), undo);
    }
}
