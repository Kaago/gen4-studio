//! The lexer the two catalog tests share (`crates/g4-kernel/tests/catalog_keys.rs` includes it as
//! `support/catalog_lex.rs`, `apps/studio/src-tauri/tests/catalog_keys.rs` by its path from
//! there): it splits a Rust source into code and literals, finds the test modules (behind
//! `#[cfg(test)]`, also in files of their own) and names the keys of the `t!("…")` calls and the
//! string literals of the production code.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A source file taken apart: `code` is the text with the inside of every comment, string and
/// character literal blanked out (same byte offsets), so braces, attributes and `t!(` found in
/// it are code; `strings` are the ranges of the string literals' contents in the text.
pub struct Lexed {
    pub code: String,
    pub strings: Vec<std::ops::Range<usize>>,
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Splits `text` into code and literals. Knows line and nested block comments, strings with
/// escapes, raw strings (`r"…"`, `r#"…"#`, also with `b`/`c`), character literals (`'{'`,
/// `'\''`, `'\u{7b}'`) and lifetimes, which are none.
pub fn lex(text: &str) -> Lexed {
    let src = text.as_bytes();
    let mut code = src.to_vec();
    let mut strings = Vec::new();
    let blank = |code: &mut Vec<u8>, from: usize, to: usize| {
        for b in &mut code[from..to] {
            *b = b' ';
        }
    };
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        let after_ident = i > 0 && is_ident(src[i - 1]);
        if rest.starts_with(b"//") {
            let end = rest
                .iter()
                .position(|&b| b == b'\n')
                .map_or(src.len(), |n| i + n);
            blank(&mut code, i, end);
            i = end;
        } else if rest.starts_with(b"/*") {
            let (mut depth, mut j) = (1, i + 2);
            while j < src.len() && depth > 0 {
                if src[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if src[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&mut code, i, j);
            i = j;
        } else if !after_ident && let Some((open, hashes)) = raw_string_start(rest) {
            let start = i + open;
            let mut close = vec![b'"'];
            close.extend(std::iter::repeat_n(b'#', hashes));
            let len = src[start..]
                .windows(close.len())
                .position(|w| w == close.as_slice())
                .expect("a closed raw string");
            strings.push(start..start + len);
            blank(&mut code, start, start + len);
            i = start + len + close.len();
        } else if src[i] == b'"' || (!after_ident && matches!(rest, [b'b' | b'c', b'"', ..])) {
            let start = i + if src[i] == b'"' { 1 } else { 2 };
            let mut j = start;
            while src[j] != b'"' {
                j += if src[j] == b'\\' { 2 } else { 1 };
            }
            strings.push(start..j);
            blank(&mut code, start, j);
            i = j + 1;
        } else if src[i] == b'\'' {
            i = char_literal_end(src, i).map_or(i + 1, |end| {
                blank(&mut code, i + 1, end - 1);
                end
            });
        } else {
            i += 1;
        }
    }
    Lexed {
        code: String::from_utf8(code).expect("only whole characters are blanked"),
        strings,
    }
}

/// For a raw string at the start of `rest` (`r"`, `r#"`, `br##"`, `cr"`): the offset of its
/// content and the number of `#`.
fn raw_string_start(rest: &[u8]) -> Option<(usize, usize)> {
    let prefix = match rest {
        [b'r', ..] => 1,
        [b'b' | b'c', b'r', ..] => 2,
        _ => return None,
    };
    let hashes = rest[prefix..].iter().take_while(|&&b| b == b'#').count();
    (rest.get(prefix + hashes) == Some(&b'"')).then_some((prefix + hashes + 1, hashes))
}

/// The end (after the closing `'`) of the character literal opening at `at`, `None` for a
/// lifetime or a label.
fn char_literal_end(src: &[u8], at: usize) -> Option<usize> {
    if src.get(at + 1) == Some(&b'\\') {
        // `'\''`, `'\n'`, `'\u{7b}'`: the escaped character, then up to the closing quote.
        let close = src[at + 3..].iter().position(|&b| b == b'\'')?;
        return Some(at + 3 + close + 1);
    }
    let len = std::str::from_utf8(&src[at + 1..(at + 5).min(src.len())])
        .or_else(|e| std::str::from_utf8(&src[at + 1..at + 1 + e.valid_up_to()]))
        .ok()?
        .chars()
        .next()?
        .len_utf8();
    (src.get(at + 1 + len) == Some(&b'\'')).then_some(at + 1 + len + 1)
}

/// The byte ranges of `code` that belong to a module behind `#[cfg(test)]` or
/// `#[cfg(all(test, …))]` with a body (`mod tests { … }`), and the names of such modules kept
/// in files of their own (`mod tests;`).
pub fn test_modules(code: &str) -> (Vec<std::ops::Range<usize>>, Vec<String>) {
    let mut ranges = Vec::new();
    let mut files = Vec::new();
    let mut from = 0;
    while let Some(found) = ["#[cfg(test)]", "#[cfg(all(test"]
        .iter()
        .filter_map(|attr| code[from..].find(attr).map(|n| from + n))
        .min()
    {
        let attr_end = found + code[found..].find(']').expect("a closed attribute") + 1;
        from = attr_end;
        // Further attributes may follow before the item.
        let mut item = code[attr_end..].trim_start();
        while item.starts_with("#[") {
            item = item[item.find(']').expect("a closed attribute") + 1..].trim_start();
        }
        let item = item
            .strip_prefix("pub(crate) ")
            .or_else(|| item.strip_prefix("pub "))
            .unwrap_or(item);
        let Some(module) = item.strip_prefix("mod ") else {
            continue;
        };
        let name: String = module
            .chars()
            .take_while(|&c| c == '_' || c.is_ascii_alphanumeric())
            .collect();
        let after_name = module[name.len()..].trim_start();
        if after_name.starts_with(';') {
            files.push(name);
            continue;
        }
        let open = code.len() - after_name.len();
        assert!(
            after_name.starts_with('{'),
            "mod {name}: neither `;` nor `{{`"
        );
        let mut depth = 0usize;
        let mut close = open;
        for (n, c) in code[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = open + n + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        assert!(close > open, "mod {name}: no closing brace");
        ranges.push(found..close);
        from = close;
    }
    (ranges, files)
}

/// Where the module `name` declared in `file` lives: `<dir>/name.rs` or `<dir>/name/mod.rs`,
/// `<dir>` being the folder of `file` for `lib.rs`, `main.rs` and `mod.rs`, else the folder
/// named like `file`.
pub fn module_paths(file: &Path, name: &str) -> [PathBuf; 2] {
    let stem = file.file_stem().unwrap_or_default();
    let parent = file.parent().unwrap_or(Path::new(""));
    let dir = if ["lib", "main", "mod"].iter().any(|s| stem == *s) {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    };
    [dir.join(format!("{name}.rs")), dir.join(name)]
}

/// What the production code of one file names: the keys of its `t!("…")` and every string
/// literal outside the test modules.
pub struct Named {
    pub t_keys: BTreeSet<String>,
    pub literals: BTreeSet<String>,
}

pub fn named(text: &str, lexed: &Lexed, tests: &[std::ops::Range<usize>]) -> Named {
    let in_tests = |at: usize| tests.iter().any(|r| r.contains(&at));
    let literals: Vec<_> = lexed
        .strings
        .iter()
        .filter(|r| !in_tests(r.start))
        .collect();
    let mut t_keys = BTreeSet::new();
    let code = lexed.code.as_bytes();
    let mut from = 0;
    while let Some(n) = lexed.code[from..].find("t!(") {
        let at = from + n;
        from = at + 3;
        if (at > 0 && is_ident(code[at - 1])) || in_tests(at) {
            continue;
        }
        let first = lexed.code[at + 3..].len() - lexed.code[at + 3..].trim_start().len();
        let quote = at + 3 + first;
        if code[quote] == b'"'
            && let Some(key) = literals.iter().find(|r| r.start == quote + 1)
        {
            t_keys.insert(text[(*key).clone()].to_owned());
        }
    }
    Named {
        t_keys,
        literals: literals
            .iter()
            .map(|r| text[(*r).clone()].to_owned())
            .collect(),
    }
}
