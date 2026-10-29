//! Word and line diffs through gix's `blob-diff` (imara-diff, Histogram): a message's words
//! as spans of both sides, a source's lines as hunks with context.

use gix::diff::blob::{Algorithm, Diff, InternedInput, diff_with_slider_heuristics};

use super::{LineHunk, SpanOp, WordSpan};

/// The spans of the old and of the new side of a word diff of `old` and `new`. Words are runs
/// of letters and digits (Unicode), runs of white space and every other character alone; a
/// control sequence of the texts (`\n`, any `\` with the character after it, `{…}`) is one
/// word. Neighbouring words of the same kind form one span; a change slides as far down as it
/// can („Hallo schöne Welt“ adds `schöne `).
pub fn word_spans(old: &str, new: &str) -> (Vec<WordSpan>, Vec<WordSpan>) {
    let (a, b) = (words(old), words(new));
    let mut input = InternedInput::default();
    input.update_before(a.iter().copied());
    input.update_after(b.iter().copied());
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_no_heuristic(&input);
    let at = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
    (
        spans(&a, SpanOp::Removed, |i| diff.is_removed(at(i))),
        spans(&b, SpanOp::Added, |i| diff.is_added(at(i))),
    )
}

/// The words of `text` (see [`word_spans`]).
fn words(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let len = if c == '\\' {
            // The backslash and the character after it (`\n`, `\r`, `\f`).
            rest[1..]
                .chars()
                .next()
                .map_or(1, |next| 1 + next.len_utf8())
        } else if c == '{' {
            rest.find('}').map_or(1, |end| end + 1)
        } else if c.is_alphanumeric() {
            rest.find(|x: char| !x.is_alphanumeric())
                .unwrap_or(rest.len())
        } else if c.is_whitespace() {
            rest.find(|x: char| !x.is_whitespace())
                .unwrap_or(rest.len())
        } else {
            c.len_utf8()
        };
        out.push(&rest[..len]);
        rest = &rest[len..];
    }
    out
}

/// The `tokens` of one side as spans: the changed ones (`changed(i)`) as `op`, the others as
/// [`SpanOp::Same`], neighbours of the same kind joined.
fn spans(tokens: &[&str], op: SpanOp, changed: impl Fn(usize) -> bool) -> Vec<WordSpan> {
    let mut out: Vec<WordSpan> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let this = if changed(i) { op } else { SpanOp::Same };
        match out.last_mut() {
            Some(last) if last.op == this => last.text.push_str(token),
            _ => out.push(WordSpan {
                op: this,
                text: (*token).to_owned(),
            }),
        }
    }
    out
}

/// The changed lines of `old` and `new` as hunks with `context` lines around them (hunks whose
/// context meets are one), at most `cap` lines in all and the number of the rest.
pub fn line_hunks(old: &str, new: &str, context: usize, cap: usize) -> (Vec<LineHunk>, u32) {
    let input = InternedInput::new(old, new);
    let diff = diff_with_slider_heuristics(Algorithm::Histogram, &input);
    let a: Vec<&str> = input.before.iter().map(|&t| input.interner[t]).collect();
    let b: Vec<&str> = input.after.iter().map(|&t| input.interner[t]).collect();
    let changes: Vec<_> = diff.hunks().collect();
    let mut out: Vec<LineHunk> = Vec::new();
    let mut shown = 0usize;
    let mut more = 0u32;
    let mut push = |hunk: &mut LineHunk, op: SpanOp, line: &str| {
        if shown < cap {
            let line = line.strip_suffix('\n').unwrap_or(line);
            let line = line.strip_suffix('\r').unwrap_or(line);
            hunk.lines.push((op, line.to_owned()));
            shown += 1;
        } else {
            more = more.saturating_add(1);
        }
    };
    let mut k = 0;
    while k < changes.len() {
        // The changes whose context meets form one hunk.
        let mut last = k;
        while last + 1 < changes.len()
            && to(changes[last + 1].before.start) <= to(changes[last].before.end) + 2 * context
        {
            last += 1;
        }
        let first = &changes[k];
        let start = to(first.before.start).saturating_sub(context);
        // Unchanged lines have the same offset on both sides up to the first change.
        let new_start = to(first.after.start) - (to(first.before.start) - start);
        let mut hunk = LineHunk {
            old_start: from(start + 1),
            new_start: from(new_start + 1),
            lines: Vec::new(),
        };
        let mut at = start;
        for change in &changes[k..=last] {
            for line in &a[at..to(change.before.start)] {
                push(&mut hunk, SpanOp::Same, line);
            }
            for line in &a[to(change.before.start)..to(change.before.end)] {
                push(&mut hunk, SpanOp::Removed, line);
            }
            for line in &b[to(change.after.start)..to(change.after.end)] {
                push(&mut hunk, SpanOp::Added, line);
            }
            at = to(change.before.end);
        }
        let end = (at + context).min(a.len());
        for line in &a[at..end] {
            push(&mut hunk, SpanOp::Same, line);
        }
        if !hunk.lines.is_empty() {
            out.push(hunk);
        }
        k = last + 1;
    }
    (out, more)
}

fn to(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

fn from(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::SpanOp;

    fn span(op: SpanOp, text: &str) -> WordSpan {
        WordSpan {
            op,
            text: text.to_owned(),
        }
    }

    #[test]
    fn word_spans_mark_the_changed_words() {
        let (old, new) = word_spans("Hallo Welt", "Hallo schöne Welt");
        assert_eq!(
            new,
            [
                span(SpanOp::Same, "Hallo "),
                span(SpanOp::Added, "schöne "),
                span(SpanOp::Same, "Welt"),
            ]
        );
        assert_eq!(old, [span(SpanOp::Same, "Hallo Welt")]);
        let (old, new) = word_spans("Gut.\\nJa", "Gut!\\nJa");
        assert_eq!(
            old,
            [
                span(SpanOp::Same, "Gut"),
                span(SpanOp::Removed, "."),
                span(SpanOp::Same, "\\nJa"),
            ]
        );
        assert_eq!(new[1], span(SpanOp::Added, "!"));
        // A control sequence is one word.
        let (_, new) = word_spans("{COLOR, 1} a", "{COLOR, 2} a");
        assert_eq!(new[0], span(SpanOp::Added, "{COLOR, 2}"));
    }

    #[test]
    fn line_hunks_keep_three_lines_of_context() {
        let old: String = (1..=20).map(|n| format!("{n}\n")).collect();
        let new = old.replace("10\n", "zehn\n").replace("20\n", "20\n21\n");
        let (hunks, more) = line_hunks(&old, &new, 3, 400);
        assert_eq!(more, 0);
        assert_eq!(hunks.len(), 2, "{hunks:?}");
        let first = &hunks[0];
        assert_eq!((first.old_start, first.new_start), (7, 7));
        let ops: Vec<SpanOp> = first.lines.iter().map(|(op, _)| *op).collect();
        assert_eq!(
            ops,
            [
                SpanOp::Same,
                SpanOp::Same,
                SpanOp::Same,
                SpanOp::Removed,
                SpanOp::Added,
                SpanOp::Same,
                SpanOp::Same,
                SpanOp::Same,
            ]
        );
        assert_eq!(first.lines[3], (SpanOp::Removed, "10".to_owned()));
        assert_eq!(first.lines[4], (SpanOp::Added, "zehn".to_owned()));
        let second = &hunks[1];
        assert_eq!((second.old_start, second.new_start), (18, 18));
        assert_eq!(second.lines.last(), Some(&(SpanOp::Added, "21".to_owned())));
        // Hunks whose context meets are one; the cap counts the rest.
        let near = old.replace("10\n", "x\n").replace("14\n", "y\n");
        assert_eq!(line_hunks(&old, &near, 3, 400).0.len(), 1);
        let (hunks, more) = line_hunks(&old, &new, 3, 5);
        assert_eq!(hunks.iter().map(|h| h.lines.len()).sum::<usize>(), 5);
        assert_eq!(more, 3 + 4);
    }
}
