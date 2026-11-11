//! The message keys of the kernel's catalog and its code agree (the twin of
//! `apps/studio/src-tauri/tests/catalog_keys.rs`): every key a `t!("…")` in `src/` names is in
//! `locales/en.yml`, and every key of the catalog is named somewhere in the code, by a `t!` or
//! as a string literal the code hands to `t!` later (a table, a match). A typo in a key would
//! otherwise show as the key itself (and tests that compare a message with the function that
//! builds it would not notice). The test modules of `src/` do not count: a test may name a
//! missing key on purpose. The lexer that tells them apart is `support/catalog_lex.rs`, which
//! the studio test includes as well.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[path = "support/catalog_lex.rs"]
mod catalog_lex;

use catalog_lex::{lex, module_paths, named, test_modules};

/// Keys that only the tests name: the probe of `tests/i18n.rs`.
const TEST_ONLY: [&str; 1] = ["i18n.probe"];

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
    let mut t_keys = BTreeSet::new();
    let mut literals = BTreeSet::new();
    for (file, text, tokens, ranges) in &lexed {
        if test_files.iter().any(|t| file.starts_with(t)) {
            continue;
        }
        let named = named(text, tokens, ranges);
        t_keys.extend(named.t_keys);
        literals.extend(named.literals);
    }
    assert!(
        t_keys.len() > 500,
        "only {} keys found in t!(…)",
        t_keys.len()
    );

    let missing: Vec<_> = t_keys.difference(&en).collect();
    assert!(
        missing.is_empty(),
        "keys a t!(…) names, not in en.yml: {missing:#?}"
    );
    let unused: Vec<_> = en
        .iter()
        .filter(|key| !literals.contains(*key) && !TEST_ONLY.contains(&key.as_str()))
        .collect();
    assert!(
        unused.is_empty(),
        "keys of en.yml no code names: {unused:#?}"
    );
}

#[test]
fn the_lexer_tells_code_from_literals_and_finds_test_modules() {
    let text = r####"
fn a() { t!("start.not_gen4"); let c = '{'; let q = '\''; }
// t!("commented.out") {
/* t!("block.comment") /* nested } */ { */
fn b<'a>(x: &'a str) -> String { format!(r#"{ "raw": "}" }"#) + "}{" }
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    fn c() { t!("missing.on_purpose"); let s = "}"; }
}
#[cfg(all(test, feature = "test-support"))]
mod store_tests;
fn d() { t!(
    "patch.no_notes"
); }
"####;
    let lexed = lex(text);
    assert_eq!(lexed.code.len(), text.len());
    let (ranges, files) = test_modules(&lexed.code);
    assert_eq!(files, ["store_tests"]);
    assert_eq!(ranges.len(), 1);
    assert!(text[ranges[0].clone()].starts_with("#[cfg(test)]"));
    assert!(text[ranges[0].clone()].ends_with("}\n}"));
    let named = named(text, &lexed, &ranges);
    assert_eq!(
        named.t_keys,
        BTreeSet::from(["patch.no_notes".to_owned(), "start.not_gen4".to_owned()])
    );
    assert!(named.literals.contains(r#"{ "raw": "}" }"#));
    assert!(named.literals.contains("}{"));
    assert!(!named.literals.contains("missing.on_purpose"));
    assert!(!named.literals.iter().any(|l| l.contains("comment")));
    assert_eq!(
        module_paths(Path::new("src/world/tmj.rs"), "import_tests"),
        [
            PathBuf::from("src/world/tmj/import_tests.rs"),
            PathBuf::from("src/world/tmj/import_tests")
        ]
    );
    assert_eq!(
        module_paths(Path::new("src/checks/mod.rs"), "fake"),
        [
            PathBuf::from("src/checks/fake.rs"),
            PathBuf::from("src/checks/fake")
        ]
    );
}
