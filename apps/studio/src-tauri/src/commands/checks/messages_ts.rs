//! The German sentences of the checks live in the frontend (`apps/studio/src/checks/messages.ts`);
//! the catalogue lives in the kernel. This test holds the one against the other: every code of
//! `Registry::all()` has its entry there, in catalogue order, with the check's group and its
//! highest severity (the UI groups a cheap run's report and offers switches by them), and there
//! is no entry for a code the kernel does not have.

use g4_kernel::checks::{Group, Registry, Severity};

const MESSAGES_TS: &str = include_str!("../../../../src/checks/messages.ts");

/// One entry of `CHECK_MESSAGES` as the file writes it.
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

/// The entries of `CHECK_MESSAGES`, in file order: each starts with `  "<code>": {` and has its
/// fields on the lines below.
fn entries() -> Vec<Entry> {
    let start = MESSAGES_TS
        .find("export const CHECK_MESSAGES")
        .expect("messages.ts declares CHECK_MESSAGES");
    let body = &MESSAGES_TS[start..];
    let body = &body[..body.find("\n};").expect("CHECK_MESSAGES ends with `};`")];
    let mut out: Vec<Entry> = Vec::new();
    for line in body.lines() {
        if let Some(code) = line
            .strip_prefix("  \"")
            .and_then(|rest| rest.strip_suffix("\": {"))
        {
            out.push(Entry {
                code: code.to_owned(),
                group: String::new(),
                severity: String::new(),
            });
        } else if let Some(entry) = out.last_mut() {
            if let Some(group) = value(line, "group") {
                entry.group = group;
            }
            if let Some(severity) = value(line, "severity") {
                entry.severity = severity;
            }
        }
    }
    out
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
fn every_check_has_its_german_sentence_in_messages_ts() {
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
        "codes without a sentence in messages.ts: {missing:?}"
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
