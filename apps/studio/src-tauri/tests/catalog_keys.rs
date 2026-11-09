//! The message keys of the Tauri crate's catalog and its code agree: every key the code names
//! (`t!("commands.git.init_unsaved")`, `("hp", "labels.hp")`, `Kind::Irreversible("mcp.access.run_rom")`)
//! is in `locales/en.yml`, and every
//! key of the sections below is named somewhere in the code. A typo in a key would otherwise
//! show as the key itself in the app (and tests that compare a message with the function that
//! builds it would not notice).

use std::collections::BTreeSet;
use std::path::Path;

/// The top-level keys of the command layer, the MCP server (`mcp`), the dev bridge (`bridge`)
/// and the files around them.
const SECTIONS: [&str; 9] = [
    "base_roms",
    "bridge",
    "commands",
    "doctor",
    "errors",
    "labels",
    "live_hub",
    "lsp",
    "mcp",
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
fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `text` without its test module: everything from the `#[cfg(test)]` above `mod tests`.
fn without_tests(text: &str) -> &str {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let Some(at) = lines
        .iter()
        .position(|l| l.trim_start_matches("pub(crate) ").starts_with("mod tests"))
    else {
        return text;
    };
    let mut first = at;
    while first > 0 && lines[first - 1].trim_start().starts_with("#[") {
        first -= 1;
    }
    let end: usize = lines[..first].iter().map(|l| l.len()).sum();
    &text[..end]
}

/// The string literals of `text` that look like a key of one of the [`SECTIONS`]
/// (`section.name`, `section.module.name` or `section.module.name.part`, lower case and
/// underscores).
fn named_keys(text: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for literal in text.split('"').skip(1).step_by(2) {
        let mut parts = literal.split('.');
        let first = parts.next().unwrap_or_default();
        let rest: Vec<&str> = parts.collect();
        let word = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        };
        if SECTIONS.contains(&first)
            && !NOT_KEYS.contains(&literal)
            && (1..=3).contains(&rest.len())
            && rest.iter().all(|s| word(s))
        {
            keys.insert(literal.to_owned());
        }
    }
    keys
}

#[test]
fn code_and_catalog_name_the_same_keys() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let en = catalog_keys(&std::fs::read_to_string(root.join("locales/en.yml")).unwrap());
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    let mut named = BTreeSet::new();
    for file in files {
        // The tests of a module may name keys on purpose (a missing one included).
        if file.file_name().is_some_and(|n| n == "tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        named.extend(named_keys(without_tests(&text)));
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
