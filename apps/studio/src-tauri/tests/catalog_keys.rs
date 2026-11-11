//! The message keys of the Tauri crate's catalog and its code agree: every key the code names
//! (`t!("commands.git.init_unsaved")`, `("hp", "labels.hp")`, `Kind::Irreversible("mcp.access.run_rom")`)
//! is in `locales/en.yml`, and every
//! key of the sections below is named somewhere in the code. A typo in a key would otherwise
//! show as the key itself in the app (and tests that compare a message with the function that
//! builds it would not notice). The test modules of `src/` do not count (a test may name a
//! missing key on purpose), whatever they are called and also when they live in files of their
//! own: the lexer is the kernel test's, shared.

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[path = "../../../../crates/g4-kernel/tests/support/catalog_lex.rs"]
mod catalog_lex;

use catalog_lex::{Lexed, lex, module_paths, named, test_modules};

/// The top-level keys of the command layer, the MCP server (`mcp`), the dev bridge (`bridge`)
/// and the files around them.
const SECTIONS: [&str; 10] = [
    "base_roms",
    "bridge",
    "commands",
    "doctor",
    "errors",
    "labels",
    "live_hub",
    "lsp",
    "mcp",
    "menu",
];

/// String literals that look like a key of a section and are none: the name of the file
/// `studio mcp` and the app share (`mcp.json`).
const NOT_KEYS: [&str; 1] = ["mcp.json"];

/// The dotted keys of the catalog `text` (nested maps, two-space indentation, one
/// `key: "value"` per line; comments and blank lines skipped).
fn catalog_keys(text: &str) -> BTreeSet<String> {
    let mut path: Vec<(usize, String)> = Vec::new();
    let mut keys = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        let (key, rest) = trimmed.split_once(':').expect("a `key:` line");
        while path.last().is_some_and(|(i, _)| *i >= indent) {
            path.pop();
        }
        if rest.trim().is_empty() {
            path.push((indent, key.to_owned()));
        } else {
            let mut dotted: Vec<&str> = path.iter().map(|(_, k)| k.as_str()).collect();
            dotted.push(key);
            keys.insert(dotted.join("."));
        }
    }
    keys
}

/// Every `.rs` file below `dir`.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Whether `literal` looks like a key of one of the [`SECTIONS`] (`section.name`,
/// `section.module.name` or `section.module.name.part`, lower case and underscores).
fn looks_like_key(literal: &str) -> bool {
    let mut parts = literal.split('.');
    let first = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    let word = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    SECTIONS.contains(&first)
        && !NOT_KEYS.contains(&literal)
        && (1..=3).contains(&rest.len())
        && rest.iter().all(|s| word(s))
}

/// The keys the production code of a source names: those of its `t!("…")` calls, whatever
/// they start with, and its string literals that look like a key of a section. The code in
/// the `tests` ranges does not count.
fn keys_named(text: &str, lexed: &Lexed, tests: &[Range<usize>]) -> BTreeSet<String> {
    let found = named(text, lexed, tests);
    let literals = found.literals.into_iter().filter(|l| looks_like_key(l));
    found.t_keys.into_iter().chain(literals).collect()
}

/// The keys the production code of `text` names, test modules (behind `#[cfg(test)]`, with a
/// body or as `mod name;`) left out.
fn production_keys(text: &str) -> BTreeSet<String> {
    let lexed = lex(text);
    let (tests, _) = test_modules(&lexed.code);
    keys_named(text, &lexed, &tests)
}

#[test]
fn code_and_catalog_name_the_same_keys() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let en = catalog_keys(&std::fs::read_to_string(root.join("locales/en.yml")).unwrap());
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    files.sort();

    // First the test modules, also those in files of their own, then the rest.
    let mut lexed = Vec::new();
    let mut test_files: Vec<PathBuf> = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let tokens = lex(&text);
        let (ranges, modules) = test_modules(&tokens.code);
        for name in modules {
            test_files.extend(module_paths(&file, &name));
        }
        lexed.push((file, text, tokens, ranges));
    }
    let mut named = BTreeSet::new();
    for (file, text, tokens, ranges) in &lexed {
        if test_files.iter().any(|t| file.starts_with(t)) {
            continue;
        }
        named.extend(keys_named(text, tokens, ranges));
    }
    let missing: Vec<_> = named.difference(&en).collect();
    assert!(
        missing.is_empty(),
        "keys the code names, not in en.yml: {missing:#?}"
    );
    let unused: Vec<_> = en
        .iter()
        .filter(|key| SECTIONS.contains(&key.split('.').next().unwrap_or_default()))
        .filter(|key| !named.contains(*key))
        .collect();
    assert!(
        unused.is_empty(),
        "keys of en.yml no code names: {unused:#?}"
    );
}

#[test]
fn a_test_module_in_its_own_file_does_not_hide_the_code_after_it() {
    let text = r#"
#[cfg(test)]
mod tests;

fn code() { t!("commands.git.init_unsaved"); let k = ("hp", "labels.hp"); }

#[cfg(test)]
mod more_tests {
    fn on_purpose() { t!("mcp.access.not_a_key"); }
}
"#;
    assert_eq!(
        production_keys(text),
        BTreeSet::from([
            "commands.git.init_unsaved".to_owned(),
            "labels.hp".to_owned()
        ])
    );
}
