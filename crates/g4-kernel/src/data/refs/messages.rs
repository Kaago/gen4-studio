//! Messages in script sources: every numbered argument of a message command
//! (`script.message_params` in `metadata/check_limits.yaml`, matched through rotom's database
//! like `script.message-missing`) is a [`Target::Message`] of the file's text archive, read
//! (field the command, slot the line). The archive of a script file is that of the first header
//! that names the file (as `script.message-missing` and `dialog::dialog_of` read it); a file no
//! header names has no message references. An alias or a constant as the argument is left to
//! rotom (the check counts numbers only), and so is a text literal.
//!
//! The archives come from the store's header table when it is loaded (unsaved edits included),
//! else from the disk; a change of a file's archive reads that file again
//! (`ScriptSources::refresh`).

use std::collections::BTreeMap;

use g4_formats::game::Family;
use rotom::compiler::ast::ScriptFile;
use uxie::map_header::MapHeader;

use super::{Access, HEADER_TABLE, Ref, Target, script_id};
use crate::checks::Limits;
use crate::checks::scripts::{MessageUse, database, message_uses_of};
use crate::store::{Loaded, Store};

/// The text archive of each script file a header names: that of the first such header.
pub(crate) type Archives = BTreeMap<u16, u16>;

/// The message references of one parsed source: a `Ref` per `MessageUse` (field the command,
/// slot the line, `Access::Read`), `archive` the file's archive (the first header naming it);
/// `None` archive: nothing.
pub(crate) fn script_message_refs(
    file: u16,
    uses: &[MessageUse],
    archive: Option<u16>,
) -> Vec<Ref> {
    let Some(archive) = archive else {
        return Vec::new();
    };
    uses.iter()
        .filter_map(|u| {
            Some(Ref {
                source: script_id(file),
                field: command_name(&u.command),
                // Lines are far below 65536.
                slot: Some(u16::try_from(u.line).unwrap_or(u16::MAX)),
                target: Target::Message {
                    archive,
                    // Message numbers are below 0x4000 (`MessageUse`).
                    index: u16::try_from(u.message).ok()?,
                },
                access: Access::Read,
            })
        })
        .collect()
}

/// The message uses of the parsed source `text` of a game of `family`: every numbered argument
/// of a message command in the file, in source order.
pub(crate) fn message_uses(family: Family, text: &str, ast: &ScriptFile) -> Vec<MessageUse> {
    let params = Limits::get().script.message_params.of(family);
    message_uses_of(family, params, &ast.items, None, text)
}

/// The name a reference carries for message command `written`: the command of
/// `script.message_params` it is (by name, else through rotom's database for a legacy name or
/// an alias), the first family that knows it; `"Message"` for none (a use is only made for a
/// listed command, so it does not happen).
fn command_name(written: &str) -> &'static str {
    let params = &Limits::get().script.message_params;
    let families = [Family::Pt, Family::Hgss, Family::Dp];
    if let Some(p) = families
        .iter()
        .flat_map(|&f| params.of(f))
        .find(|p| p.command == written)
    {
        return p.command.as_str();
    }
    for family in families {
        let Some(db) = database(family) else {
            continue;
        };
        // rotom v0.7.0: src/database.rs DatabaseV2::get_command (canonical, legacy, alias)
        let Ok(command) = db.get_command(written) else {
            continue;
        };
        if let Some(p) = params.of(family).iter().find(|p| {
            db.get_command(&p.command)
                .is_ok_and(|c| std::ptr::eq(c, command))
        }) {
            return p.command.as_str();
        }
    }
    "Message"
}

/// The archive of each script file of `headers` (the first header naming it wins).
pub(crate) fn file_archives(headers: &[MapHeader]) -> Archives {
    let mut out = Archives::new();
    for h in headers {
        out.entry(h.script_file_id())
            .or_insert_with(|| h.text_archive_id());
    }
    out
}

/// The archives of the script files now: from the store's header table when it holds one,
/// else `kept` (what the index read before; the table on disk changes only through the
/// store), else the table on disk. A table that does not read gives none.
pub(crate) fn archives_now(store: &Store, kept: Option<&Archives>) -> Archives {
    if let Some(Loaded::HeaderTable(headers)) = store.peek(HEADER_TABLE) {
        return file_archives(headers);
    }
    if let Some(kept) = kept {
        return kept.clone();
    }
    store
        .project()
        .read_headers()
        .map(|headers| file_archives(&headers))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::project::AssetKind;

    fn used(command: &str, message: u32, line: u32) -> MessageUse {
        MessageUse {
            script: Some(1),
            command: command.into(),
            message,
            line,
            col: 13,
        }
    }

    #[test]
    fn message_uses_become_read_refs_with_line_and_command() {
        let refs = script_message_refs(4, &[used("Message", 5, 3), used("NPCMsg", 0, 7)], Some(9));
        let shown: Vec<_> = refs
            .iter()
            .map(|r| (r.source, r.field, r.slot, r.target, r.access))
            .collect();
        let source = crate::project::AssetId {
            kind: AssetKind::Script,
            index: 4,
        };
        assert_eq!(
            shown,
            [
                (
                    source,
                    "Message",
                    Some(3),
                    Target::Message {
                        archive: 9,
                        index: 5
                    },
                    Access::Read
                ),
                (
                    source,
                    "NPCMsg",
                    Some(7),
                    Target::Message {
                        archive: 9,
                        index: 0
                    },
                    Access::Read
                ),
            ]
        );
    }

    #[test]
    fn a_file_without_header_has_no_message_refs() {
        assert!(script_message_refs(4, &[used("Message", 5, 3)], None).is_empty());
    }

    #[test]
    fn the_first_header_naming_a_file_gives_its_archive() {
        use uxie::map_header::MapHeaderPt;
        let header = |script: u16, archive: u16| {
            MapHeader::Pt(MapHeaderPt {
                script_file_id: script,
                text_archive_id: archive,
                ..Default::default()
            })
        };
        let archives = file_archives(&[header(3, 10), header(4, 11), header(3, 12)]);
        assert_eq!(archives, Archives::from([(3, 10), (4, 11)]));
    }
}

#[cfg(all(test, feature = "test-support"))]
mod index_tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Arc;

    use crate::data::refs::{RefIndex, Target};
    use crate::project::AssetKind;
    use crate::scripts::{self, ScriptSource};
    use crate::store::Store;
    use crate::test_support::TextFixture;

    /// The text archive of header 0 of the fixture (its script file is 0).
    const ARCHIVE: u16 = 433;

    fn message_refs(
        index: &mut RefIndex,
        st: &Store,
        archive: u16,
        n: u16,
    ) -> Vec<(u16, &'static str, Option<u16>)> {
        index
            .refs(st, Target::Message { archive, index: n })
            .unwrap()
            .into_iter()
            .map(|r| {
                assert_eq!(r.source.kind, AssetKind::Script);
                (r.source.index, r.field, r.slot)
            })
            .collect()
    }

    #[test]
    fn the_index_finds_a_message_of_a_script_and_follows_the_header() {
        let fx = TextFixture::exported();
        let project = fx.project();
        scripts::write_source(
            &project,
            0,
            &ScriptSource::Rotom(
                "script eins #1:\n    Message 5\n    End\nscript zwei #2:\n    Message 5\n    \
                 Message 6\n    End\n"
                    .into(),
            ),
        )
        .unwrap();
        let mut st = Store::new(Arc::new(project));
        let mut index = RefIndex::new();
        assert_eq!(
            message_refs(&mut index, &st, ARCHIVE, 5),
            [(0, "Message", Some(2)), (0, "Message", Some(5))]
        );
        assert_eq!(message_refs(&mut index, &st, ARCHIVE, 6).len(), 1);
        assert!(
            index
                .refs(
                    &st,
                    Target::Message {
                        archive: ARCHIVE,
                        index: 7
                    }
                )
                .unwrap()
                .is_empty()
        );

        // Header 0 takes another archive (unsaved): the file's messages move with it.
        st.transaction("archive", |tx| {
            tx.edit_header(0, |h| {
                if let uxie::map_header::MapHeader::Pt(h) = h {
                    h.text_archive_id = 500;
                }
                Ok(())
            })
        })
        .unwrap();
        index.refresh_scripts_now(&st).unwrap();
        assert!(message_refs(&mut index, &st, ARCHIVE, 5).is_empty());
        assert_eq!(message_refs(&mut index, &st, 500, 5).len(), 2);
    }
}
