//! The words of the checks live in the frontend: titles and sentences in the catalogs
//! (`apps/studio/src/i18n/locales/{de,en}/checks.json`), the check's group and its highest severity
//! in `apps/studio/src/checks/messages.ts` (`CHECK_MESSAGES`, one line per code); the catalogue
//! lives in the kernel. This test holds the one against the other: every code of
//! `Registry::all()` has its line in `messages.ts` and its entry in both catalogs, in catalogue
//! order, with the check's group and its highest severity (a check that reports errors cannot be
//! switched off), and there is no entry for a code the kernel does not have.

use g4_kernel::checks::{Group, Registry, Severity};
use serde_json::Value;

const MESSAGES_TS: &str = include_str!("../../../../src/checks/messages.ts");

/// The catalogs, by language; `serde_json` keeps their key order (`preserve_order`).
const CATALOGS: [(&str, &str); 2] = [
    (
        "de",
        include_str!("../../../../src/i18n/locales/de/checks.json"),
    ),
    (
        "en",
        include_str!("../../../../src/i18n/locales/en/checks.json"),
    ),
];

/// One line of `CHECK_MESSAGES` as the file writes it.
#[derive(Debug, PartialEq, Eq)]
struct Entry {
    code: String,
    group: String,
    severity: String,
}

/// The value of `field: "…"` on `line`, if it has one.
fn value(line: &str, field: &str) -> Option<String> {
    let pattern = format!("{field}: \"");
    let start = line.find(&pattern)? + pattern.len();
    let rest = &line[start..];
    Some(rest[..rest.find('"')?].to_owned())
}

/// The entries of `CHECK_MESSAGES`, in file order: one per line, `  "<code>": { group: "…",
/// severity: "…" },`.
fn entries() -> Vec<Entry> {
    let start = MESSAGES_TS
        .find("export const CHECK_MESSAGES")
        .expect("messages.ts declares CHECK_MESSAGES");
    let body = &MESSAGES_TS[start..];
    let body = &body[..body.find("\n};").expect("CHECK_MESSAGES ends with `};`")];
    body.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("  \"")?;
            let (code, fields) = rest.split_once("\": {")?;
            Some(Entry {
                code: code.to_owned(),
                group: value(fields, "group").unwrap_or_default(),
                severity: value(fields, "severity").unwrap_or_default(),
            })
        })
        .collect()
}

/// The codes of a catalog in file order; every entry must carry a title and a sentence. A key
/// starting with `_` is no check code but the words the checks' frontend shares (`_words`).
fn catalog_codes(lang: &str, json: &str) -> Vec<String> {
    let Value::Object(catalog) = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("{lang}/checks.json is not JSON: {e}"))
    else {
        panic!("{lang}/checks.json is not an object keyed by check code");
    };
    let catalog: Vec<(&String, &Value)> = catalog
        .iter()
        .filter(|(key, _)| !key.starts_with('_'))
        .collect();
    for (code, entry) in &catalog {
        for field in ["title", "text"] {
            let word = entry.get(field).and_then(Value::as_str).unwrap_or("");
            assert!(
                !word.trim().is_empty(),
                "{lang}/checks.json: `{code}` has no non-empty `{field}`"
            );
        }
    }
    catalog.into_iter().map(|(code, _)| code.clone()).collect()
}

fn group_name(group: Group) -> &'static str {
    match group {
        Group::Cheap => "cheap",
        Group::PerScript => "per_script",
        Group::PerArchive => "per_archive",
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

#[test]
fn every_check_has_its_group_and_severity_in_messages_ts() {
    let expected: Vec<Entry> = Registry::all()
        .iter()
        .map(|check| Entry {
            code: check.code().to_owned(),
            group: group_name(check.group()).to_owned(),
            severity: severity_name(check.severity()).to_owned(),
        })
        .collect();
    let found = entries();
    let missing: Vec<&str> = expected
        .iter()
        .filter(|e| !found.iter().any(|f| f.code == e.code))
        .map(|e| e.code.as_str())
        .collect();
    assert!(
        missing.is_empty(),
        "codes without a line in messages.ts: {missing:?}"
    );
    let unknown: Vec<&str> = found
        .iter()
        .filter(|f| !expected.iter().any(|e| e.code == f.code))
        .map(|f| f.code.as_str())
        .collect();
    assert!(
        unknown.is_empty(),
        "codes in messages.ts the kernel does not have: {unknown:?}"
    );
    assert_eq!(
        found, expected,
        "messages.ts: catalogue order, group and severity of every code"
    );
}

#[test]
fn catalogs_hold_every_code() {
    let expected: Vec<String> = Registry::all()
        .iter()
        .map(|check| check.code().to_owned())
        .collect();
    for (lang, json) in CATALOGS {
        let found = catalog_codes(lang, json);
        let missing: Vec<&String> = expected.iter().filter(|c| !found.contains(c)).collect();
        assert!(
            missing.is_empty(),
            "codes without an entry in {lang}/checks.json: {missing:?}"
        );
        let unknown: Vec<&String> = found.iter().filter(|c| !expected.contains(c)).collect();
        assert!(
            unknown.is_empty(),
            "codes in {lang}/checks.json the kernel does not have: {unknown:?}"
        );
        assert_eq!(
            found, expected,
            "{lang}/checks.json: catalogue order of every code"
        );
    }
}
