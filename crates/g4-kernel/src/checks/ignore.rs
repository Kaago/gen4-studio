//! What a run leaves out and the ignore file `g4-checks.json` that keeps it with the project
//! (spec 3b.4 §7): findings the user ignored, by code, subject, key and fingerprint, and checks
//! switched off by code. Errors are never left out: an error cannot be ignored, a check that
//! reports errors cannot be switched off, and an entry a hand-written file has for either is
//! not honoured. "Ausgangslage übernehmen" ignores every warning and info of a report at once.
//!
//! The file lives in the project root, not in `.g4/` (a cache that may be deleted), so the
//! baseline moves with the project. It is written whole and atomically on every change.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{ChecksReport, Finding, Registry, Severity, Subject};
use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;

/// Name of the ignore file in the project root.
pub const IGNORE_FILE: &str = "g4-checks.json";

/// A fingerprint as it is written in JSON (the ignore file, `g4k check`, the UI): 16 lowercase
/// hex digits. A JavaScript number holds 53 bits, a fingerprint 64.
pub fn fingerprint_text(fingerprint: u64) -> String {
    format!("{fingerprint:016x}")
}

/// The fingerprint of 1 to 16 hex digits (either case); `None` for anything else.
pub fn parse_fingerprint(text: &str) -> Option<u64> {
    if text.is_empty() || text.len() > 16 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(text, 16).ok()
}

/// Serde of a fingerprint as [`fingerprint_text`].
pub(crate) mod fingerprint_serde {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(fingerprint: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::fingerprint_text(*fingerprint))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let text = String::deserialize(d)?;
        super::parse_fingerprint(&text).ok_or_else(|| {
            D::Error::custom(format!("fingerprint {text:?} is not 1 to 16 hex digits"))
        })
    }
}

/// Identifies an ignored finding: it stays ignored while its code, subject, key and
/// fingerprint are the same (a change of the checked bytes brings it back).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IgnoreKey {
    pub code: String,
    pub subject: Subject,
    pub key: String,
    #[serde(with = "fingerprint_serde")]
    pub fingerprint: u64,
}

impl IgnoreKey {
    pub fn of(finding: &Finding) -> IgnoreKey {
        IgnoreKey {
            code: finding.code.to_owned(),
            subject: finding.subject,
            key: finding.key.clone(),
            fingerprint: finding.fingerprint,
        }
    }
}

/// The ignore file as it is written: both lists sorted, so a change shows as a small diff.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoreFile {
    #[serde(default)]
    pub ignored: Vec<IgnoreKey>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

/// What a run leaves out: checks switched off by code, and findings the user ignored (read
/// from and written to [`IGNORE_FILE`]). Errors are never ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoreSet {
    pub disabled: HashSet<String>,
    pub ignored: HashSet<IgnoreKey>,
}

impl IgnoreSet {
    pub fn is_disabled(&self, code: &str) -> bool {
        self.disabled.contains(code)
    }

    /// Whether `finding` is ignored: on the list and no error.
    pub fn ignores(&self, finding: &Finding) -> bool {
        finding.severity != Severity::Error && self.ignored.contains(&IgnoreKey::of(finding))
    }

    /// The ignore file of the project at `root`; empty when there is none. A file that is no
    /// valid ignore file is [`KernelError::InvalidProject`] naming it (it is never
    /// overwritten unread).
    pub fn load(root: &Path) -> Result<IgnoreSet, KernelError> {
        let path = root.join(IGNORE_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(IgnoreSet::default());
            }
            Err(e) => return Err(io_at(&path)(e)),
        };
        let file: IgnoreFile = serde_json::from_str(&text)
            .map_err(|e| KernelError::InvalidProject(format!("{IGNORE_FILE}: {e}")))?;
        Ok(IgnoreSet {
            disabled: file.disabled.into_iter().collect(),
            ignored: file.ignored.into_iter().collect(),
        })
    }

    /// Writes the ignore file of the project at `root` atomically (pretty JSON, sorted).
    pub fn save(&self, root: &Path) -> Result<(), KernelError> {
        let mut text = serde_json::to_string_pretty(&self.file())
            .map_err(|e| KernelError::InvalidProject(format!("{IGNORE_FILE}: {e}")))?;
        text.push('\n');
        write_atomic(root, &root.join(IGNORE_FILE), text.as_bytes())
    }

    /// The set as it is written.
    pub fn file(&self) -> IgnoreFile {
        let mut ignored: Vec<IgnoreKey> = self.ignored.iter().cloned().collect();
        ignored.sort();
        let mut disabled: Vec<String> = self.disabled.iter().cloned().collect();
        disabled.sort();
        IgnoreFile { ignored, disabled }
    }

    /// Puts the finding `key` of `severity` on the list; `false` when it already was. An
    /// error is refused ([`KernelError::InvalidProject`] naming the code).
    pub fn ignore(&mut self, key: IgnoreKey, severity: Severity) -> Result<bool, KernelError> {
        if severity == Severity::Error {
            return Err(KernelError::InvalidProject(format!(
                "errors cannot be ignored ({})",
                key.code
            )));
        }
        Ok(self.ignored.insert(key))
    }

    /// Takes the finding `key` off the list; `false` when it was not on it.
    pub fn unignore(&mut self, key: &IgnoreKey) -> bool {
        self.ignored.remove(key)
    }

    /// "Ausgangslage übernehmen": puts every warning and info of `findings` on the list
    /// (errors stay); returns how many were not on it yet.
    pub fn accept_baseline<'a>(
        &mut self,
        findings: impl IntoIterator<Item = &'a Finding>,
    ) -> usize {
        findings
            .into_iter()
            .filter(|f| f.severity != Severity::Error)
            .filter(|f| self.ignored.insert(IgnoreKey::of(f)))
            .count()
    }

    /// Switches the check `code` on or off; `false` when it already was. Refused
    /// ([`KernelError::InvalidProject`] naming the code) for a code no check has, and for
    /// switching off a check that reports errors.
    pub fn set_enabled(&mut self, code: &str, enabled: bool) -> Result<bool, KernelError> {
        if enabled {
            return Ok(self.disabled.remove(code));
        }
        let check = Registry::get(code)
            .ok_or_else(|| KernelError::InvalidProject(format!("unknown check code {code}")))?;
        if check.severity() == Severity::Error {
            return Err(KernelError::InvalidProject(format!(
                "{code} reports errors and cannot be switched off"
            )));
        }
        Ok(self.disabled.insert(code.to_owned()))
    }
}

impl ChecksReport {
    /// Whether the report left out a check as `disabled` that `ignore` switches on (again):
    /// it lacks that check's findings, which only a run can add, so it is no report of a
    /// state with this list.
    pub fn lacks_enabled(&self, ignore: &IgnoreSet) -> bool {
        self.skipped
            .iter()
            .any(|(code, why)| why == "disabled" && !ignore.is_disabled(code))
    }

    /// Sorts the findings out anew by `ignore` after it changed, without a run: the findings
    /// of checks now switched off go and the check is listed in `skipped` as `disabled` (as a
    /// run would), the others are split into `findings` and `ignored` again, both in
    /// catalogue order; within a check, what was in `findings` comes before what was in
    /// `ignored` (the run's own order of the two is not known). A check switched on again has
    /// nothing in the report; it needs a run.
    pub fn apply_ignore(&mut self, ignore: &IgnoreSet) {
        let order = |code: &str| Registry::position(code).unwrap_or(usize::MAX);
        // Like `Runner::run_with`: a check that reports errors runs whatever the file says.
        let off = |code: &str| {
            ignore.is_disabled(code)
                && Registry::get(code).is_some_and(|c| c.severity() != Severity::Error)
        };
        let mut all: Vec<Finding> = std::mem::take(&mut self.findings);
        all.append(&mut self.ignored);
        // Stable: the order a check reported its findings in stays.
        all.sort_by_key(|f| order(f.code));
        let mut now_off: BTreeSet<&'static str> = Registry::all()
            .iter()
            .filter(|c| self.groups.contains(&c.group()) && off(c.code()))
            .map(|c| c.code())
            .collect();
        for finding in all {
            if off(finding.code) {
                now_off.insert(finding.code);
            } else if ignore.ignores(&finding) {
                self.ignored.push(finding);
            } else {
                self.findings.push(finding);
            }
        }
        // A check that is off says only that.
        self.skipped
            .retain(|(code, why)| !now_off.contains(code) || why == "disabled");
        for code in now_off {
            if !self.skipped.iter().any(|(c, _)| *c == code) {
                self.skipped.push((code, "disabled".to_owned()));
            }
        }
        self.skipped.sort_by_key(|(code, _)| order(code));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::checks::{ChecksReport, Finding, Severity, Subject};
    use crate::error::KernelError;

    fn finding(
        code: &'static str,
        severity: Severity,
        subject: Subject,
        key: &str,
        bytes: &[u8],
    ) -> Finding {
        Finding::new(code, severity, subject, key, "m", bytes)
    }

    fn hex(fingerprint: u64) -> String {
        format!("{fingerprint:016x}")
    }

    #[test]
    fn ignore_round_trips_the_file() {
        let dir = tempfile::tempdir().unwrap();
        // No file: nothing ignored, nothing switched off.
        assert_eq!(IgnoreSet::load(dir.path()).unwrap(), IgnoreSet::default());

        let never = finding(
            "warp.never-fires",
            Severity::Info,
            Subject::EventFile(411),
            "warp:2",
            &[1, 2],
        );
        let split = finding(
            "header.text-split",
            Severity::Warning,
            Subject::ScriptFile(3),
            "text_archive",
            &[9],
        );
        let mut set = IgnoreSet::default();
        assert!(set.ignore(IgnoreKey::of(&never), never.severity).unwrap());
        assert!(
            !set.ignore(IgnoreKey::of(&never), never.severity).unwrap(),
            "already on the list"
        );
        assert!(set.ignore(IgnoreKey::of(&split), split.severity).unwrap());
        assert!(set.set_enabled("encounter.empty-slot", false).unwrap());
        assert!(!set.set_enabled("encounter.empty-slot", false).unwrap());
        set.save(dir.path()).unwrap();
        assert_eq!(IgnoreSet::load(dir.path()).unwrap(), set);

        // Sorted, the subject as {kind, index}, the fingerprint as 16 hex digits (a u64 does
        // not survive a JavaScript number).
        let text = std::fs::read_to_string(dir.path().join(IGNORE_FILE)).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "ignored": [
                    { "code": "header.text-split", "subject": { "kind": "scriptFile", "index": 3 },
                      "key": "text_archive", "fingerprint": hex(split.fingerprint) },
                    { "code": "warp.never-fires", "subject": { "kind": "eventFile", "index": 411 },
                      "key": "warp:2", "fingerprint": hex(never.fingerprint) },
                ],
                "disabled": ["encounter.empty-slot"],
            })
        );
        // Written through `.g4/tmp`, nothing left behind.
        let tmp = dir.path().join(".g4/tmp");
        assert_eq!(std::fs::read_dir(tmp).unwrap().count(), 0);

        // Back to nothing.
        assert!(set.unignore(&IgnoreKey::of(&never)));
        assert!(!set.unignore(&IgnoreKey::of(&never)));
        assert!(set.unignore(&IgnoreKey::of(&split)));
        assert!(set.set_enabled("encounter.empty-slot", true).unwrap());
        set.save(dir.path()).unwrap();
        assert_eq!(IgnoreSet::load(dir.path()).unwrap(), IgnoreSet::default());

        // A broken file, or a fingerprint that is no hex text, names the file.
        for broken in [
            "{ broken".to_owned(),
            serde_json::json!({ "ignored": [{ "code": "warp.never-fires",
                "subject": { "kind": "eventFile", "index": 1 }, "key": "warp:0",
                "fingerprint": 12 }] })
            .to_string(),
            serde_json::json!({ "ignored": [{ "code": "warp.never-fires",
                "subject": { "kind": "eventFile", "index": 1 }, "key": "warp:0",
                "fingerprint": "+f" }] })
            .to_string(),
        ] {
            std::fs::write(dir.path().join(IGNORE_FILE), &broken).unwrap();
            let err = IgnoreSet::load(dir.path()).unwrap_err();
            assert!(matches!(err, KernelError::InvalidProject(_)), "{err}");
            assert!(err.to_string().contains(IGNORE_FILE), "{err}");
        }
        // A file with only one of the lists is fine.
        std::fs::write(
            dir.path().join(IGNORE_FILE),
            r#"{ "disabled": ["warp.never-fires"] }"#,
        )
        .unwrap();
        assert!(
            IgnoreSet::load(dir.path())
                .unwrap()
                .is_disabled("warp.never-fires")
        );
    }

    #[test]
    fn changed_subject_reappears() {
        let mut set = IgnoreSet::default();
        let warp = |key: &str, bytes: &[u8]| {
            finding(
                "warp.never-fires",
                Severity::Info,
                Subject::EventFile(1),
                key,
                bytes,
            )
        };
        let before = warp("warp:0", &[5, 0, 7, 0]);
        set.ignore(IgnoreKey::of(&before), before.severity).unwrap();
        assert!(set.ignores(&before));
        // The checked bytes changed: another fingerprint, the finding is reported again.
        let changed = warp("warp:0", &[6, 0, 7, 0]);
        assert!(!set.ignores(&changed));
        // The same bytes again: ignored again. Another warp of the same file: not.
        assert!(set.ignores(&warp("warp:0", &[5, 0, 7, 0])));
        assert!(!set.ignores(&warp("warp:1", &[5, 0, 7, 0])));

        // A report sorted out anew moves each to its list.
        let mut report = ChecksReport {
            findings: vec![before.clone(), changed.clone()],
            ..ChecksReport::default()
        };
        report.apply_ignore(&set);
        assert_eq!(report.findings, std::slice::from_ref(&changed));
        assert_eq!(report.ignored, std::slice::from_ref(&before));
        set.unignore(&IgnoreKey::of(&before));
        report.apply_ignore(&set);
        // Within a check, what was not ignored comes first (a run's order is not kept).
        assert_eq!(report.findings, [changed, before]);
        assert!(report.ignored.is_empty());
    }

    #[test]
    fn errors_cannot_be_ignored() {
        let mut set = IgnoreSet::default();
        let error = finding(
            "warp.dest-header",
            Severity::Error,
            Subject::EventFile(0),
            "warp:0",
            &[1],
        );
        let err = set
            .ignore(IgnoreKey::of(&error), Severity::Error)
            .unwrap_err();
        assert!(matches!(err, KernelError::InvalidProject(_)), "{err}");
        assert!(err.to_string().contains("warp.dest-header"), "{err}");
        assert_eq!(set, IgnoreSet::default());
        // Not even when a hand-written file lists it.
        set.ignored.insert(IgnoreKey::of(&error));
        assert!(!set.ignores(&error));
        let mut report = ChecksReport {
            findings: vec![error.clone()],
            ..ChecksReport::default()
        };
        report.apply_ignore(&set);
        assert_eq!(report.findings, [error]);

        // A check that reports errors cannot be switched off; an unknown code is refused.
        for code in ["warp.dest-header", "trainer.no-party", "warp.no-such-check"] {
            let err = set.set_enabled(code, false).unwrap_err();
            assert!(err.to_string().contains(code), "{err}");
        }
        assert!(set.disabled.is_empty());
        // Switching one on is always possible (a hand-written file may list it).
        set.disabled.insert("warp.dest-header".into());
        assert!(set.set_enabled("warp.dest-header", true).unwrap());
    }

    #[test]
    fn accept_baseline_ignores_every_warning_and_info() {
        let error = finding(
            "warp.dest-header",
            Severity::Error,
            Subject::EventFile(0),
            "warp:0",
            &[1],
        );
        let warning = finding(
            "trainer.no-party",
            Severity::Warning,
            Subject::Trainer(3),
            "party",
            &[],
        );
        let info = finding(
            "warp.never-fires",
            Severity::Info,
            Subject::EventFile(0),
            "warp:1",
            &[2],
        );
        let info2 = finding(
            "encounter.min-max",
            Severity::Info,
            Subject::Encounter(7),
            "slot:surf:1",
            &[3],
        );
        let findings = vec![error.clone(), warning.clone(), info.clone(), info2.clone()];
        let mut set = IgnoreSet::default();
        assert_eq!(set.accept_baseline(&findings), 3);
        for f in &findings {
            assert_eq!(set.ignores(f), f.severity != Severity::Error, "{}", f.code);
        }
        assert_eq!(set.accept_baseline(&findings), 0, "nothing new");
        assert_eq!(set.ignored.len(), 3);

        let mut report = ChecksReport {
            findings,
            ..ChecksReport::default()
        };
        report.apply_ignore(&set);
        assert_eq!(report.findings, [error]);
        // Catalogue order: warp.never-fires, trainer.no-party, encounter.min-max.
        assert_eq!(report.ignored, [info, warning, info2]);
    }

    #[test]
    fn a_report_sorted_out_anew_keeps_catalogue_order_and_drops_disabled_checks() {
        // Catalogue order: warp.dest-header, warp.never-fires, ..., encounter.empty-slot.
        let never = finding(
            "warp.never-fires",
            Severity::Info,
            Subject::EventFile(0),
            "warp:1",
            &[2],
        );
        let empty = finding(
            "encounter.empty-slot",
            Severity::Warning,
            Subject::Encounter(0),
            "slot:grass:0:swarm:0",
            &[4],
        );
        let dest = finding(
            "warp.dest-header",
            Severity::Error,
            Subject::EventFile(0),
            "warp:0",
            &[1],
        );
        let mut report = ChecksReport {
            findings: vec![dest.clone(), empty.clone()],
            ignored: vec![never.clone()],
            skipped: vec![("script.message-missing", "scripts not adopted".into())],
            ..ChecksReport::default()
        };
        let mut set = IgnoreSet::default();
        report.apply_ignore(&set);
        assert_eq!(report.findings, [dest.clone(), never.clone(), empty]);
        assert!(report.ignored.is_empty());

        set.set_enabled("encounter.empty-slot", false).unwrap();
        set.ignore(IgnoreKey::of(&never), never.severity).unwrap();
        report.apply_ignore(&set);
        assert_eq!(report.findings, [dest]);
        assert_eq!(report.ignored, [never]);
        assert_eq!(
            report.skipped,
            [
                ("encounter.empty-slot", "disabled".to_owned()),
                ("script.message-missing", "scripts not adopted".to_owned()),
            ]
        );
        // Twice changes nothing.
        let once = report.clone();
        report.apply_ignore(&set);
        assert_eq!(report.skipped, once.skipped);
        assert_eq!(report.findings, once.findings);

        // Switched on again: the report lacks the check's findings until a run.
        assert!(!report.lacks_enabled(&set));
        set.set_enabled("encounter.empty-slot", true).unwrap();
        assert!(report.lacks_enabled(&set));
    }

    #[test]
    fn fingerprints_are_written_as_sixteen_hex_digits() {
        assert_eq!(fingerprint_text(0x0c79_e202_f9cc_d0cf), "0c79e202f9ccd0cf");
        assert_eq!(fingerprint_text(1), "0000000000000001");
        assert_eq!(
            parse_fingerprint("0c79e202f9ccd0cf"),
            Some(0x0c79_e202_f9cc_d0cf)
        );
        assert_eq!(parse_fingerprint("ff"), Some(255));
        assert_eq!(parse_fingerprint("FFFFFFFFFFFFFFFF"), Some(u64::MAX));
        for bad in ["", "+ff", "-1", "0x1f", "1ffffffffffffffff", "g"] {
            assert_eq!(parse_fingerprint(bad), None, "{bad}");
        }
    }
}
