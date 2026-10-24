//! Text checks (`docs/research/2026-10-23-3b4/checks-report.md` §1.5): lines wider than the
//! field text window and pages with more lines than it shows, both only in dialog archives
//! (those a header uses as its `text_archive_id`); messages the encoder warns about and text
//! archives that cannot be read, in every archive.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chatot::{default_glyph_widths, get_default_charmap, measure_line_width};

use super::{Finding, FixId, Group, Limits, Severity, Skipped, Source, SourceCheck, Subject};
use crate::project::AssetKind;

/// Where the text archive JSONs live in a DSPRE project.
const TEXT_DIR: &str = "expanded/textArchives";

/// The JSON of text archive `archive`, relative to the project.
fn json_file(archive: u16) -> PathBuf {
    Path::new(TEXT_DIR).join(format!("{archive:04}.json"))
}

fn idx(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The dialog archives: those a header uses as its text archive.
fn dialog_archives(src: &dyn Source) -> Result<BTreeSet<u16>, Skipped> {
    let headers = src.headers().ok_or_else(|| Skipped {
        why: "the header table cannot be read (it names the dialog archives)".into(),
    })?;
    Ok(headers.iter().map(|h| h.text_archive_id()).collect())
}

/// How a line of a message ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    /// `\n`: the next line of the window.
    Line,
    /// `\r` (clear) or `\f` (scroll): a new page.
    Page,
    /// The end of the message.
    Message,
}

/// The lines of a message in chatot text form, split at the hard breaks `\n`, `\r` and
/// `\f` (backslash and letter) the way chatot's tokenizer reads them: a backslash takes the
/// next character with it, so `\\n` is a backslash and an `n`.
fn lines(text: &str) -> Vec<(String, End)> {
    let mut out = Vec::new();
    let mut line = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            line.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push((std::mem::take(&mut line), End::Line)),
            Some('r' | 'f') => out.push((std::mem::take(&mut line), End::Page)),
            Some(other) => {
                line.push('\\');
                line.push(other);
            }
            None => line.push('\\'),
        }
    }
    out.push((line, End::Message));
    out
}

/// Every readable text archive, by index.
fn archives(
    src: &dyn Source,
) -> impl Iterator<Item = (u16, std::sync::Arc<super::TextEntry>)> + '_ {
    (0..src.count(AssetKind::TextArchive)).filter_map(move |a| {
        let a = idx(a);
        Some((a, src.text(a)?))
    })
}

/// `text.too-wide` (warning): in a dialog archive, a line (between hard breaks) wider than
/// the field text window (`dialog_line_px`, 216 px), measured with chatot's glyph widths
/// (a placeholder `{…}` counts 0 px). A line chatot cannot read is left to `text.encode`.
/// Subject: text archive; key `message:<m>:line:<l>` (line 1-based, counting every hard
/// break). Args: `[archive, message, line, width px, window px]`. Fixes: `Wrap`.
/// Fingerprint: the line's text.
pub(crate) struct TextTooWide;

impl SourceCheck for TextTooWide {
    const CODE: &'static str = "text.too-wide";
    const SEVERITY: Severity = Severity::Warning;
    const GROUP: Group = Group::PerArchive;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let dialogs = dialog_archives(src)?;
        let max = limits.dialog_line_px.value;
        let (charmap, widths) = (get_default_charmap(), default_glyph_widths());
        for &archive in &dialogs {
            let Some(entry) = src.text(archive) else {
                continue;
            };
            for (m, message) in entry.doc.messages.iter().enumerate() {
                for (l, (line, _)) in lines(&message.text).iter().enumerate() {
                    // chatot v0.8.1: src/format.rs measure_line_width (the widest line of
                    // the input; one line here)
                    let Ok((_, px)) = measure_line_width(line, charmap, widths) else {
                        continue;
                    };
                    if px <= max {
                        continue;
                    }
                    let l = l + 1;
                    out.push(
                        Finding::new(
                            Self::CODE,
                            Self::SEVERITY,
                            Subject::TextArchive(archive),
                            format!("message:{m}:line:{l}"),
                            format!(
                                "Text archive {archive:04}, message {m}: line {l} is {px} px wide, the text window fits {max}."
                            ),
                            line.as_bytes(),
                        )
                        .with_args([
                            archive.to_string(),
                            m.to_string(),
                            l.to_string(),
                            px.to_string(),
                            max.to_string(),
                        ])
                        .with_fixes([FixId::Wrap]),
                    );
                }
            }
        }
        Ok(())
    }
}

/// `text.three-lines` (warning): in a dialog archive, a page (the lines between `\r` or
/// `\f`) with more lines than the text window shows (`dialog_lines_max`, 2). A `\n` after
/// the second line opens a third line even when nothing follows it (a message ending in
/// `\n`; on vanilla Platin DE all 8 findings are such, the count of checks-report §1.5).
/// One finding per message, for its first such page. Subject: text archive; key `message:<m>`. Args:
/// `[archive, message, page (1-based), lines on it, lines the window shows]`. Fingerprint:
/// the message's text.
pub(crate) struct TextThreeLines;

impl SourceCheck for TextThreeLines {
    const CODE: &'static str = "text.three-lines";
    const SEVERITY: Severity = Severity::Warning;
    const GROUP: Group = Group::PerArchive;

    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped> {
        let dialogs = dialog_archives(src)?;
        let max = limits.dialog_lines_max.value as usize;
        for &archive in &dialogs {
            let Some(entry) = src.text(archive) else {
                continue;
            };
            for (m, message) in entry.doc.messages.iter().enumerate() {
                let Some((page, count)) = first_full_page(&message.text, max) else {
                    continue;
                };
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::TextArchive(archive),
                        format!("message:{m}"),
                        format!(
                            "Text archive {archive:04}, message {m}: page {page} has {count} lines, the text window shows {max}."
                        ),
                        message.text.as_bytes(),
                    )
                    .with_args([
                        archive.to_string(),
                        m.to_string(),
                        page.to_string(),
                        count.to_string(),
                        max.to_string(),
                    ]),
                );
            }
        }
        Ok(())
    }
}

/// The first page (1-based) of `text` with more than `max` lines, and its line count.
fn first_full_page(text: &str, max: usize) -> Option<(usize, usize)> {
    let mut page = 1;
    let mut on_page: Vec<&str> = Vec::new();
    for (line, end) in &lines(text) {
        on_page.push(line);
        if *end == End::Line {
            continue;
        }
        let count = on_page.len();
        if count > max {
            return Some((page, count));
        }
        on_page.clear();
        page += 1;
    }
    None
}

/// `text.encode` (error): chatot's encoder warns about a message (an unknown character or
/// control code); the build stops on it. Every archive. One finding per message. Subject:
/// text archive; key `message:<m>`; file: the archive's JSON with the line and column of the
/// first problem. Args: `[archive, message, first problem, number of problems]`.
/// Fingerprint: the message's text.
pub(crate) struct TextEncode;

impl SourceCheck for TextEncode {
    const CODE: &'static str = "text.encode";
    const SEVERITY: Severity = Severity::Error;
    const GROUP: Group = Group::PerArchive;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        for (archive, entry) in archives(src) {
            let json = json_file(archive);
            // Cached with the archive: unchanged archives are not validated again.
            let problems = entry.encode_problems.get_or_init(|| {
                entry
                    .doc
                    .messages
                    .iter()
                    .enumerate()
                    .map(|(m, message)| (m, crate::text::validate(&message.text, &json)))
                    .filter(|(_, problems)| !problems.is_empty())
                    .collect()
            });
            for (m, problems) in problems {
                let (m, message) = (*m, &entry.doc.messages[*m]);
                let Some(first) = problems.first() else {
                    continue;
                };
                out.push(
                    Finding::new(
                        Self::CODE,
                        Self::SEVERITY,
                        Subject::TextArchive(archive),
                        format!("message:{m}"),
                        format!(
                            "Text archive {archive:04}, message {m}: {} The archive cannot be built like this.",
                            first.message
                        ),
                        message.text.as_bytes(),
                    )
                    .with_args([
                        archive.to_string(),
                        m.to_string(),
                        first.message.clone(),
                        problems.len().to_string(),
                    ])
                    .with_file(json.clone(), first.line, first.col),
                );
            }
        }
        Ok(())
    }
}

/// `text.json-broken` (error): a text archive cannot be read: its JSON
/// (`expanded/textArchives/NNNN.json`) does not parse, or its member does not decode (the
/// snapshot reads the newer of the two). Subject: text archive; key `unreadable`. Args:
/// `[archive, reason]`. Fingerprint: the reason.
pub(crate) struct TextJsonBroken;

impl SourceCheck for TextJsonBroken {
    const CODE: &'static str = "text.json-broken";
    const SEVERITY: Severity = Severity::Error;
    const GROUP: Group = Group::PerArchive;

    fn check(&self, src: &dyn Source, _: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        // Reading every archive first names every one that cannot be read.
        archives(src).for_each(drop);
        for (archive, why) in src.text_unreadable() {
            out.push(
                Finding::new(
                    Self::CODE,
                    Self::SEVERITY,
                    Subject::TextArchive(archive),
                    "unreadable",
                    format!(
                        "Text archive {archive:04} cannot be read: {why}. Repair the file or load it from the ROM again."
                    ),
                    why.as_bytes(),
                )
                .with_args([archive.to_string(), why]),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use chatot::{default_glyph_widths, get_default_charmap, measure_line_width};
    use uxie::map_header::MapHeader;

    use super::*;
    use crate::checks::fake::{Fake, texts};
    use crate::checks::{FixId, Subject};

    fn width(text: &str) -> u32 {
        measure_line_width(text, get_default_charmap(), default_glyph_widths())
            .unwrap()
            .1
    }

    /// A line of exactly `px` pixels of one repeated character.
    fn line_of(px: u32) -> String {
        for c in ["a", "m", "i", "e", "0", "W", "x"] {
            let w = width(c);
            if w > 0 && px.is_multiple_of(w) {
                let line = c.repeat((px / w) as usize);
                assert_eq!(width(&line), px);
                return line;
            }
        }
        panic!("no character divides {px}");
    }

    fn keys(findings: &[Finding]) -> Vec<(Subject, &str)> {
        findings
            .iter()
            .map(|f| (f.subject, f.key.as_str()))
            .collect()
    }

    /// Headers 0 and 1 both use text archive 0; archive 1 is no dialog archive.
    fn world() -> Fake {
        let mut f = Fake::world();
        for h in 0..2 {
            let MapHeader::Pt(h) = f.header_mut(h) else {
                unreachable!()
            };
            h.text_archive_id = 0;
        }
        f
    }

    #[test]
    fn text_too_wide_only_in_dialog_archives_at_216() {
        let fits = line_of(216);
        let wide = &(fits.clone() + &fits[..1]);
        assert!(width(wide) > 216);
        let mut f = world();
        let message = format!("{fits}\\n{wide}\\r{fits}");
        f.texts = vec![
            Some(texts(&[&fits, &message])),
            // Not a dialog archive: never measured.
            Some(texts(&[wide])),
        ];
        let found = f.run(&TextTooWide);
        assert_eq!(
            keys(&found),
            [(Subject::TextArchive(0), "message:1:line:2")]
        );
        let w = &found[0];
        assert_eq!(w.severity, Severity::Warning);
        assert_eq!(w.args, ["0", "1", "2", &width(wide).to_string(), "216"]);
        assert_eq!(w.fixes, [FixId::Wrap]);
        assert!(w.message.contains("216"), "{}", w.message);

        // A placeholder counts 0 px (chatot), so it never makes a line too wide.
        f.texts[0] = Some(texts(&[&format!("{fits}{{STRVAR_1 3, 0, 0}}")]));
        assert!(f.run(&TextTooWide).is_empty());

        // Without a header table the dialog archives are not known.
        f.headers = None;
        assert!(!f.skip_reason(&TextTooWide).is_empty());
    }

    #[test]
    fn text_three_lines_counts_lines_between_page_breaks() {
        let mut f = world();
        f.texts = vec![
            Some(texts(&[
                "a\\nb\\rc\\nd",
                "a\\nb\\nc",
                "a\\nb\\fc\\nd",
                "a\\nb\\rc\\nd\\ne\\r",
                // A \n after the second line opens a third, empty one.
                "a\\nb\\n",
                "a\\nb\\r",
            ])),
            Some(texts(&["a\\nb\\nc"])),
        ];
        let found = f.run(&TextThreeLines);
        assert_eq!(
            keys(&found),
            [
                (Subject::TextArchive(0), "message:1"),
                (Subject::TextArchive(0), "message:3"),
                (Subject::TextArchive(0), "message:4"),
            ]
        );
        // [archive, message, page, lines on it, lines the window shows]
        assert_eq!(found[1].args, ["0", "3", "2", "3", "2"]);
        assert_eq!(found[1].severity, Severity::Warning);
    }

    #[test]
    fn text_encode_reports_what_the_encoder_warns_about_in_every_archive() {
        let mut f = world();
        f.texts = vec![
            Some(texts(&["Hallo", "Hallo\\n[NOPE]"])),
            Some(texts(&["[NOPE]"])),
        ];
        let found = f.run(&TextEncode);
        assert_eq!(
            keys(&found),
            [
                (Subject::TextArchive(0), "message:1"),
                (Subject::TextArchive(1), "message:0"),
            ]
        );
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(found[0].args[..2], ["0", "1"]);
        assert_eq!(
            found[0].file,
            Some((
                std::path::PathBuf::from("expanded/textArchives/0000.json"),
                2,
                1
            ))
        );
    }

    #[test]
    fn text_json_broken_names_unreadable_archives() {
        let mut f = world();
        f.texts = vec![Some(texts(&["a"])), None];
        f.text_unreadable = vec![(1, "expected value at line 1".into())];
        let found = f.run(&TextJsonBroken);
        assert_eq!(keys(&found), [(Subject::TextArchive(1), "unreadable")]);
        assert_eq!(found[0].args, ["1", "expected value at line 1"]);
        assert_eq!(found[0].severity, Severity::Error);
    }
}
