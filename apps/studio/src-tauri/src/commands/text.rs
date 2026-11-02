//! Text archives: reading and editing messages, creating archives, searching, and checking
//! one text against the encoder and the text box width.

use std::path::Path;

use g4_kernel::store::{ChangeId, TextHit};
use g4_kernel::text::{default_message_id, validate};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{ProblemDto, lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct TextMessageDto {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct TextArchiveDto {
    pub index: u16,
    pub key: u16,
    pub lang: String,
    pub messages: Vec<TextMessageDto>,
    /// Ids of the map headers that use this archive.
    pub users: Vec<u16>,
}

/// One edit of an archive's message list.
#[derive(Debug, Deserialize)]
pub enum TextEdit {
    /// Replaces the text of message `index`.
    Set { index: u16, text: String },
    /// Appends a message with the next default id and an empty text.
    Append,
    /// Removes the last message; an archive keeps at least one.
    RemoveLast,
}

pub fn get_text_archive_impl(state: &AppState, index: u16) -> Result<TextArchiveDto, CmdError> {
    let mut guard = lock(state)?;
    let store = &mut session(&mut guard)?.store;
    let doc = store.text_archive(index)?;
    let (key, lang) = (doc.key, doc.lang.clone());
    let messages = doc
        .messages
        .iter()
        .map(|m| TextMessageDto {
            id: m.id.clone(),
            text: m.text.clone(),
        })
        .collect();
    Ok(TextArchiveDto {
        index,
        key,
        lang,
        messages,
        users: store.text_archive_users(index)?,
    })
}

/// Applies `edit`; `None` when it left the archive as it was (nothing to undo then). The step
/// is named `label` ("Dialog geändert" from the dialog editor), else after the edit ("edit
/// text", "add message", "remove message").
pub fn edit_text_archive_impl(
    state: &AppState,
    index: u16,
    edit: TextEdit,
    label: Option<String>,
) -> Result<Option<ChangeId>, CmdError> {
    let mut guard = lock(state)?;
    let store = &mut session(&mut guard)?.store;
    let len = store.text_archive(index)?.messages.len();
    let change = match edit {
        TextEdit::Set {
            index: message,
            text,
        } => {
            if usize::from(message) >= len {
                return Err(
                    format!("text archive {index} has {len} messages, none at {message}").into(),
                );
            }
            store.edit_text_archive(index, label.as_deref().unwrap_or("edit text"), |doc| {
                doc.messages[usize::from(message)].text = text;
            })?
        }
        TextEdit::Append => {
            // The count is a u16 on disk: one more message would not fit.
            if len >= usize::from(u16::MAX) {
                return Err(format!("text archive {index} is full").into());
            }
            store.edit_text_archive(index, label.as_deref().unwrap_or("add message"), |doc| {
                doc.messages.push(g4_kernel::text::TextMessage {
                    id: default_message_id(len),
                    text: String::new(),
                    other: Default::default(),
                });
            })?
        }
        TextEdit::RemoveLast => {
            if len <= 1 {
                return Err(format!(
                    "text archive {index} has {len} message, an archive keeps at least one"
                )
                .into());
            }
            store.edit_text_archive(index, label.as_deref().unwrap_or("remove message"), |doc| {
                doc.messages.pop();
            })?
        }
    };
    Ok(change)
}

/// Appends a new archive with one message and returns its index. Not undoable.
pub fn create_text_archive_impl(state: &AppState) -> Result<u16, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.create_text_archive()?)
}

pub fn search_text_impl(
    state: &AppState,
    query: String,
    limit: usize,
) -> Result<Vec<TextHit>, CmdError> {
    let mut guard = lock(state)?;
    Ok(session(&mut guard)?.store.search_text(&query, limit)?)
}

/// Warnings for one message text. Positions are inside that text (line 1 is its first line);
/// `file` is empty because the text belongs to no file yet.
pub fn validate_text_impl(text: String) -> Vec<ProblemDto> {
    validate(&text, Path::new(""))
        .iter()
        .map(|p| ProblemDto::from_kernel(p, Path::new(""), "text"))
        .collect()
}

#[tauri::command]
pub fn get_text_archive(state: State<AppState>, index: u16) -> Result<TextArchiveDto, CmdError> {
    get_text_archive_impl(&state, index)
}

#[tauri::command]
pub fn edit_text_archive(
    state: State<AppState>,
    index: u16,
    edit: TextEdit,
    label: Option<String>,
) -> Result<Option<ChangeId>, CmdError> {
    edit_text_archive_impl(&state, index, edit, label)
}

#[tauri::command]
pub fn create_text_archive(state: State<AppState>) -> Result<u16, CmdError> {
    create_text_archive_impl(&state)
}

#[tauri::command]
pub fn search_text(
    state: State<AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<TextHit>, CmdError> {
    search_text_impl(&state, query, limit)
}

#[tauri::command]
pub fn validate_text(text: String) -> Vec<ProblemDto> {
    validate_text_impl(text)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::{dirty_impl, open_synthetic, save_impl, set_header_impl, undo_impl};
    use super::*;
    use g4_kernel::project::{AssetId, AssetKind};
    use g4_kernel::store::MapHeader;

    const ARCHIVE: u16 = 433;

    fn archive_id() -> AssetId {
        AssetId {
            kind: AssetKind::TextArchive,
            index: ARCHIVE,
        }
    }

    #[test]
    fn get_text_archive_returns_messages_and_users() {
        let (_f, state) = open_synthetic();
        let before = get_text_archive_impl(&state, ARCHIVE).unwrap();
        let texts: Vec<_> = before.messages.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["Nowhere", "Twinleaf Town", "Sandgem Town"]);
        assert_eq!(before.messages[1].id, "msg_00001");
        assert_eq!(before.index, ARCHIVE);
        assert!(!before.lang.is_empty());
        // The fixture's header 0 uses the archive; header 1 does not.
        assert_eq!(before.users, [0]);

        let mut header = super::super::get_header_impl(&state, 1).unwrap();
        if let MapHeader::Pt(h) = &mut header {
            h.text_archive_id = ARCHIVE;
        }
        set_header_impl(&state, 1, header, "use text").unwrap();
        assert_eq!(
            get_text_archive_impl(&state, ARCHIVE).unwrap().users,
            [0, 1]
        );
        // A garbage archive is an error, not a panic.
        assert!(get_text_archive_impl(&state, 0).is_err());
        assert!(get_text_archive_impl(&AppState::default(), ARCHIVE).is_err());
    }

    #[test]
    fn edit_set_then_save_writes_json() {
        let (f, state) = open_synthetic();
        let change = edit_text_archive_impl(
            &state,
            ARCHIVE,
            TextEdit::Set {
                index: 1,
                text: "Neu".to_owned(),
            },
            None,
        )
        .unwrap();
        assert!(change.is_some());
        assert_eq!(dirty_impl(&state).unwrap(), [archive_id()]);
        assert_eq!(
            get_text_archive_impl(&state, ARCHIVE).unwrap().messages[1].text,
            "Neu"
        );
        // Setting the same text again records nothing.
        let same = edit_text_archive_impl(
            &state,
            ARCHIVE,
            TextEdit::Set {
                index: 1,
                text: "Neu".to_owned(),
            },
            None,
        )
        .unwrap();
        assert!(same.is_none());

        let report = save_impl(&state).unwrap();
        assert_eq!(report.written, [archive_id()]);
        assert!(report.problems.is_empty());
        let json =
            std::fs::read_to_string(f.dir.path().join("expanded/textArchives/0433.json")).unwrap();
        assert!(json.contains("Neu"), "{json}");
        assert!(dirty_impl(&state).unwrap().is_empty());

        // Undo brings the old text back.
        assert_eq!(undo_impl(&state).unwrap(), [archive_id()]);
        assert_eq!(
            get_text_archive_impl(&state, ARCHIVE).unwrap().messages[1].text,
            "Twinleaf Town"
        );
    }

    #[test]
    fn set_refuses_a_message_that_does_not_exist() {
        let (_f, state) = open_synthetic();
        let err = edit_text_archive_impl(
            &state,
            ARCHIVE,
            TextEdit::Set {
                index: 3,
                text: "x".to_owned(),
            },
            None,
        )
        .unwrap_err();
        assert_eq!(err.code, "error");
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn append_and_remove_last_change_count() {
        let (_f, state) = open_synthetic();
        let count = |state: &AppState| {
            get_text_archive_impl(state, ARCHIVE)
                .unwrap()
                .messages
                .len()
        };
        assert_eq!(count(&state), 3);
        assert!(
            edit_text_archive_impl(&state, ARCHIVE, TextEdit::Append, None)
                .unwrap()
                .is_some()
        );
        let doc = get_text_archive_impl(&state, ARCHIVE).unwrap();
        assert_eq!(doc.messages.len(), 4);
        assert_eq!(
            (doc.messages[3].id.as_str(), doc.messages[3].text.as_str()),
            ("msg_00003", "")
        );
        edit_text_archive_impl(&state, ARCHIVE, TextEdit::RemoveLast, None).unwrap();
        assert_eq!(count(&state), 3);
        // Undo of the removal restores the appended message.
        undo_impl(&state).unwrap();
        assert_eq!(count(&state), 4);

        // The last message of an archive stays.
        let (_f, state) = open_synthetic();
        let created = create_text_archive_impl(&state).unwrap();
        assert_eq!(created, 698);
        let err = edit_text_archive_impl(&state, created, TextEdit::RemoveLast, None).unwrap_err();
        assert_eq!(err.code, "error");
        assert_eq!(
            get_text_archive_impl(&state, created)
                .unwrap()
                .messages
                .len(),
            1
        );
    }

    #[test]
    fn create_text_archive_returns_the_next_index() {
        let (_f, state) = open_synthetic();
        let index = create_text_archive_impl(&state).unwrap();
        assert_eq!(index, 698);
        let doc = get_text_archive_impl(&state, index).unwrap();
        assert_eq!(doc.messages[0].text, "Neuer Text");
        assert!(dirty_impl(&state).unwrap().is_empty());
    }

    #[test]
    fn search_text_hits() {
        let (_f, state) = open_synthetic();
        let hits = search_text_impl(&state, "twinleaf".to_owned(), 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].archive, hits[0].index), (433, 1));
        assert!(hits[0].excerpt.contains("Twinleaf Town"));
        assert!(
            search_text_impl(&state, String::new(), 10)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            search_text_impl(&state, "town".to_owned(), 1)
                .unwrap()
                .len(),
            1
        );
        // Serialized for the frontend with the kernel's field names.
        let json = serde_json::to_value(&hits[0]).unwrap();
        assert_eq!(json["archive"], 433);
    }

    #[test]
    fn validate_text_warns_on_encoder_problems_only() {
        assert!(validate_text_impl("Hello".to_owned()).is_empty());
        // The line width is the `text.too-wide` check's, not this live validation's.
        assert!(validate_text_impl("W".repeat(60)).is_empty());
        let problems = validate_text_impl("[NOPE]".to_owned());
        assert!(!problems.is_empty());
        assert_eq!(problems[0].severity, "warning");
        assert_eq!(problems[0].source, "text");
        assert!(
            problems[0].message.contains("NOPE"),
            "{}",
            problems[0].message
        );
        assert_eq!((problems[0].line, problems[0].col), (1, 1));
    }

    #[test]
    fn save_reports_text_warnings_with_a_relative_file() {
        let (_f, state) = open_synthetic();
        edit_text_archive_impl(
            &state,
            ARCHIVE,
            TextEdit::Set {
                index: 2,
                text: "[NOPE]".to_owned(),
            },
            None,
        )
        .unwrap();
        let report = save_impl(&state).unwrap();
        assert_eq!(report.written, [archive_id()]);
        assert_eq!(report.problems.len(), 1);
        let problem = &report.problems[0];
        assert_eq!(problem.file, "expanded/textArchives/0433.json");
        assert_eq!((problem.severity, problem.source), ("warning", "text"));
        assert!(
            problem.message.starts_with("msg_00002: "),
            "{}",
            problem.message
        );
        // The shape the frontend reads.
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["written"][0]["kind"], "TextArchive");
        assert_eq!(json["problems"][0]["source"], "text");
    }

    #[test]
    fn edit_text_archive_takes_a_label() {
        let (_f, state) = open_synthetic();
        let set = |text: &str, label: Option<String>| {
            edit_text_archive_impl(
                &state,
                ARCHIVE,
                TextEdit::Set {
                    index: 1,
                    text: text.to_owned(),
                },
                label,
            )
            .unwrap()
        };
        assert!(set("Neu", Some("Dialog geändert".to_owned())).is_some());
        assert!(set("Neuer", None).is_some());
        let history = super::super::history_impl(&state).unwrap();
        // Without a label the step keeps the name it had.
        assert_eq!(history.undo, ["Dialog geändert", "edit text"]);
    }
}
