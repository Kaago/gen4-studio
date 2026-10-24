//! Project checks ("Prüfen und Beheben"): findings about references, limits and texts that
//! would break the game, each with a code, a severity, a subject and optional fixes.
//!
//! This module holds the shared types, the limits metadata ([`limits`]) and the registry of
//! checks. The snapshot a run reads is built in [`snapshot`], the [`Runner`] runs the checks
//! that register in [`Registry::all`]. See `docs/formats/check_limits.md` for the limits.

use std::path::PathBuf;
use std::sync::Arc;

use g4_formats::area_data::AreaData;
use g4_formats::data::encounter::Encounters;
use g4_formats::data::trainer_doc::TrainerDoc;
use g4_formats::game::Family;
use g4_formats::map::MapFile;
use g4_formats::matrix::Matrix;
use serde::{Deserialize, Serialize};
use uxie::event_file::EventFile;
use uxie::map_header::MapHeader;

use crate::project::{AssetId, AssetKind};

mod data;
#[cfg(test)]
mod fake;
mod ignore;
pub mod limits;
pub mod runner;
mod scripts;
pub mod snapshot;
mod text;
mod world;

pub(crate) use scripts::ScriptFacts;

pub use ignore::{
    IGNORE_FILE, IgnoreFile, IgnoreKey, IgnoreSet, fingerprint_text, parse_fingerprint,
};
pub use limits::Limits;
pub use runner::{ChecksReport, MapCell, Runner};
pub use snapshot::{Overlay, ScriptEntry, Snapshot, SnapshotCache, TextEntry, Unreadable};

pub use crate::text::Severity;

/// What a finding is about. The UI turns it into a label ("Eventdatei 0411") and a way to open it.
/// Written in JSON (reports, the ignore file, the UI) as `{kind, index}`, the kind in camelCase
/// (`eventFile`); the project has index 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "SubjectRepr", from = "SubjectRepr")]
pub enum Subject {
    Header(u16),
    EventFile(u16),
    ScriptFile(u16),
    TextArchive(u16),
    Trainer(u16),
    Encounter(u16),
    Matrix(u16),
    Map(u16),
    Project,
    AreaData(u16),
}

/// The JSON form of a [`Subject`].
#[derive(Serialize, Deserialize)]
struct SubjectRepr {
    kind: SubjectKind,
    #[serde(default)]
    index: u16,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SubjectKind {
    Header,
    EventFile,
    ScriptFile,
    TextArchive,
    Trainer,
    Encounter,
    Matrix,
    Map,
    Project,
    AreaData,
}

impl From<Subject> for SubjectRepr {
    fn from(subject: Subject) -> SubjectRepr {
        let (kind, index) = match subject {
            Subject::Header(n) => (SubjectKind::Header, n),
            Subject::EventFile(n) => (SubjectKind::EventFile, n),
            Subject::ScriptFile(n) => (SubjectKind::ScriptFile, n),
            Subject::TextArchive(n) => (SubjectKind::TextArchive, n),
            Subject::Trainer(n) => (SubjectKind::Trainer, n),
            Subject::Encounter(n) => (SubjectKind::Encounter, n),
            Subject::Matrix(n) => (SubjectKind::Matrix, n),
            Subject::Map(n) => (SubjectKind::Map, n),
            Subject::Project => (SubjectKind::Project, 0),
            Subject::AreaData(n) => (SubjectKind::AreaData, n),
        };
        SubjectRepr { kind, index }
    }
}

impl From<SubjectRepr> for Subject {
    fn from(SubjectRepr { kind, index: n }: SubjectRepr) -> Subject {
        match kind {
            SubjectKind::Header => Subject::Header(n),
            SubjectKind::EventFile => Subject::EventFile(n),
            SubjectKind::ScriptFile => Subject::ScriptFile(n),
            SubjectKind::TextArchive => Subject::TextArchive(n),
            SubjectKind::Trainer => Subject::Trainer(n),
            SubjectKind::Encounter => Subject::Encounter(n),
            SubjectKind::Matrix => Subject::Matrix(n),
            SubjectKind::Map => Subject::Map(n),
            SubjectKind::Project => Subject::Project,
            SubjectKind::AreaData => Subject::AreaData(n),
        }
    }
}

impl Subject {
    /// A stable number per kind, mixed into [`fingerprint`] (never reorder or reuse).
    fn tag(self) -> (u8, u16) {
        match self {
            Subject::Header(n) => (0, n),
            Subject::EventFile(n) => (1, n),
            Subject::ScriptFile(n) => (2, n),
            Subject::TextArchive(n) => (3, n),
            Subject::Trainer(n) => (4, n),
            Subject::Encounter(n) => (5, n),
            Subject::Matrix(n) => (6, n),
            Subject::Map(n) => (7, n),
            Subject::Project => (8, 0),
            Subject::AreaData(n) => (9, n),
        }
    }
}

/// A fix the user can apply to a finding (planned and applied by `plan_fix` / `apply_fix`).
/// Written in snake_case (`create_script`) everywhere: reports, the UI, the commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixId {
    /// Wrap a message at the text window width.
    Wrap,
    /// Append the missing message to the text archive.
    AppendMessage,
    /// Swap the minimum and maximum level of an encounter slot.
    SwapMinMax,
    /// Create the missing script.
    CreateScript,
}

/// How a check is scheduled: cheap ones run on every change, the others per script file or
/// text archive that changed. Written in snake_case (`per_script`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Cheap,
    PerScript,
    PerArchive,
}

impl Group {
    /// Every group, in the order a run takes them.
    pub const ALL: [Group; 3] = [Group::Cheap, Group::PerScript, Group::PerArchive];
}

/// One result of a check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    /// Catalogue code, e.g. `warp.dest-header`.
    pub code: &'static str,
    pub severity: Severity,
    pub subject: Subject,
    /// Identifies the spot within the subject (`warp 2`, `message 12`); with `code` and
    /// `subject` it names the finding for ignoring.
    pub key: String,
    /// English sentence with the numbers, the fallback for `g4k check`; the UI builds German
    /// sentences from `code` and `args`.
    pub message: String,
    /// The message arguments (numbers, names) the UI substitutes into its sentence.
    pub args: Vec<String>,
    /// The field of the subject the finding points at, if any.
    pub field: Option<String>,
    /// Further subjects the finding touches (the script a warp's event calls, say).
    pub also: Vec<Subject>,
    pub fixes: Vec<FixId>,
    /// `(file relative to the project, line, column)` for findings in script and text sources.
    pub file: Option<(PathBuf, u32, u32)>,
    /// See [`fingerprint`]; an ignored finding stays ignored while it does not change.
    /// Written as [`fingerprint_text`].
    #[serde(serialize_with = "ignore::fingerprint_serde::serialize")]
    pub fingerprint: u64,
}

impl Finding {
    /// A finding without arguments, field, fixes or file; the fingerprint is computed from
    /// `subject`, `key` and the checked bytes `relevant`.
    pub fn new(
        code: &'static str,
        severity: Severity,
        subject: Subject,
        key: impl Into<String>,
        message: impl Into<String>,
        relevant: &[u8],
    ) -> Finding {
        let key = key.into();
        let fingerprint = fingerprint(subject, &key, relevant);
        Finding {
            code,
            severity,
            subject,
            key,
            message: message.into(),
            args: Vec::new(),
            field: None,
            also: Vec::new(),
            fixes: Vec::new(),
            file: None,
            fingerprint,
        }
    }

    /// With the message arguments (each check documents their order).
    pub fn with_args<S: ToString>(mut self, args: impl IntoIterator<Item = S>) -> Finding {
        self.args = args.into_iter().map(|a| a.to_string()).collect();
        self
    }

    /// With the field of the subject the finding points at.
    pub fn with_field(mut self, field: impl Into<String>) -> Finding {
        self.field = Some(field.into());
        self
    }

    /// With further subjects the finding touches.
    pub fn with_also(mut self, also: impl IntoIterator<Item = Subject>) -> Finding {
        self.also = also.into_iter().collect();
        self
    }

    /// With the fixes that apply.
    pub fn with_fixes(mut self, fixes: impl IntoIterator<Item = FixId>) -> Finding {
        self.fixes = fixes.into_iter().collect();
        self
    }

    /// With the source file (relative to the project) and the 1-based line and column.
    pub fn with_file(mut self, path: impl Into<PathBuf>, line: u32, col: u32) -> Finding {
        self.file = Some((path.into(), line, col));
        self
    }
}

/// Why a check did not run (a missing input, say); reported next to the findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    pub why: String,
}

/// One check of the catalogue.
pub trait Check: Sync {
    fn code(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn group(&self) -> Group;
    /// Runs the check on `snap` and appends its findings to `out`. `Err` means the check could
    /// not run (and found nothing).
    fn run(&self, snap: &Snapshot, limits: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped>;
}

/// What the world and data checks read: the [`Snapshot`] of a run, plain values in the unit
/// tests. Every accessor returns `None` for a value that does not exist or cannot be read
/// (`asset.unreadable` names the latter).
pub(crate) trait Source {
    fn family(&self) -> Family;
    /// Every map header; `None` when the table cannot be read.
    fn headers(&self) -> Option<&[MapHeader]>;
    /// Members of the archive of `kind` ([`Snapshot::member_count`]; text archives:
    /// [`Snapshot::text_archive_count`]), 0 when it cannot be counted.
    fn count(&self, kind: AssetKind) -> usize;
    fn event(&self, index: u16) -> Option<&EventFile>;
    fn matrix(&self, index: u16) -> Option<&Matrix>;
    fn map(&self, index: u16) -> Option<&MapFile>;
    fn area_data(&self, index: u16) -> Option<&AreaData>;
    fn trainer(&self, index: u16) -> Option<&TrainerDoc>;
    fn encounter(&self, index: u16) -> Option<&Encounters>;
    /// Entries of the name bank `bank` of `metadata/text_banks.yaml` (`species`,
    /// `trainer_classes`); `None` when the bank is unknown or cannot be read.
    fn bank_len(&self, bank: &str) -> Option<usize>;
    /// Number of item ids: the arm9 item table, else the item data members.
    fn item_count(&self) -> usize;
    /// Reads the header table and every member the cheap checks use (never the text
    /// archives or script sources), then returns what could not be read so far and the
    /// locked assets.
    fn unreadable_all(&self) -> (Vec<Unreadable>, Vec<(AssetId, String)>);
    /// Whether the project's scripts are adopted (rotom sources exist).
    fn scripts_adopted(&self) -> bool;
    /// What script file `file` holds (its slots, level script, message uses), from its
    /// source when the scripts are adopted, else from the member; `None` past the last file.
    fn script_facts(&self, file: u16) -> Option<Arc<ScriptFacts>>;
    /// Text archive `archive` (an unsaved document wins over the disk); `None` when it does
    /// not exist or cannot be read.
    fn text(&self, archive: u16) -> Option<Arc<TextEntry>>;
    /// The text archives that could not be read so far, with the reason.
    fn text_unreadable(&self) -> Vec<(u16, String)>;
    /// Notes that the running check left part of its work undone (a script file it cannot
    /// decode, say); the runner reports it in `skipped` next to the check's findings.
    fn note_skip(&self, why: String);
}

impl Source for Snapshot {
    fn family(&self) -> Family {
        Snapshot::family(self)
    }
    fn headers(&self) -> Option<&[MapHeader]> {
        self.header_table()
    }
    fn count(&self, kind: AssetKind) -> usize {
        match kind {
            // A text archive the store created and has not saved counts too.
            AssetKind::TextArchive => self.text_archive_count(),
            _ => self.member_count(kind),
        }
    }
    fn event(&self, index: u16) -> Option<&EventFile> {
        Snapshot::event(self, index)
    }
    fn matrix(&self, index: u16) -> Option<&Matrix> {
        Snapshot::matrix(self, index)
    }
    fn map(&self, index: u16) -> Option<&MapFile> {
        Snapshot::map(self, index)
    }
    fn area_data(&self, index: u16) -> Option<&AreaData> {
        Snapshot::area_data(self, index)
    }
    fn trainer(&self, index: u16) -> Option<&TrainerDoc> {
        Snapshot::trainer(self, index)
    }
    fn encounter(&self, index: u16) -> Option<&Encounters> {
        Snapshot::encounter(self, index)
    }
    fn bank_len(&self, bank: &str) -> Option<usize> {
        let project = self.project();
        let archive =
            crate::data::text_banks::text_banks_ref(project.family, project.language).get(bank)?;
        Some(self.text_archive(archive)?.doc.messages.len())
    }
    fn item_count(&self) -> usize {
        self.project()
            .item_table()
            .map_or_else(|| self.member_count(AssetKind::Item), |t| t.entries.len())
    }
    fn unreadable_all(&self) -> (Vec<Unreadable>, Vec<(AssetId, String)>) {
        // Cheap: the members only. Text archives and script sources are read before the
        // per-archive and per-script groups, whose checks report them.
        self.load_members(&mut |_, _| {});
        let locked = self
            .locked_assets()
            .map(|(id, why)| (id, why.to_owned()))
            .collect();
        (self.unreadable(), locked)
    }
    fn scripts_adopted(&self) -> bool {
        Snapshot::scripts_adopted(self)
    }
    fn script_facts(&self, file: u16) -> Option<Arc<ScriptFacts>> {
        Snapshot::script_facts(self, file)
    }
    fn text(&self, archive: u16) -> Option<Arc<TextEntry>> {
        self.text_archive(archive)
    }
    fn text_unreadable(&self) -> Vec<(u16, String)> {
        self.unreadable()
            .into_iter()
            .filter(|u| u.kind == AssetKind::TextArchive)
            .filter_map(|u| Some((u.index?, u.why)))
            .collect()
    }
    fn note_skip(&self, why: String) {
        Snapshot::note_skip(self, why);
    }
}

/// A check of the catalogue: runs on a [`Source`] (the world and data checks in the cheap
/// group, the script and text checks per script file and text archive).
pub(crate) trait SourceCheck: Sync {
    const CODE: &'static str;
    /// The highest severity the check reports (`trainer.no-party` also reports warnings).
    const SEVERITY: Severity;
    const GROUP: Group = Group::Cheap;
    fn check(
        &self,
        src: &dyn Source,
        limits: &Limits,
        out: &mut Vec<Finding>,
    ) -> Result<(), Skipped>;
}

impl<T: SourceCheck> Check for T {
    fn code(&self) -> &'static str {
        T::CODE
    }
    fn severity(&self) -> Severity {
        T::SEVERITY
    }
    fn group(&self) -> Group {
        T::GROUP
    }
    fn run(&self, snap: &Snapshot, limits: &Limits, out: &mut Vec<Finding>) -> Result<(), Skipped> {
        self.check(snap, limits, out)
    }
}

/// The checks that exist.
pub struct Registry;

impl Registry {
    /// The check of `code`, if one is registered.
    pub fn get(code: &str) -> Option<&'static dyn Check> {
        Registry::all().iter().copied().find(|c| c.code() == code)
    }

    /// Where `code` stands in [`Registry::all`] (catalogue order).
    pub fn position(code: &str) -> Option<usize> {
        Registry::all().iter().position(|c| c.code() == code)
    }

    /// Every registered check, in catalogue order: the world checks, the data checks,
    /// `asset.unreadable` (the last cheap one, it reads every member the cheap checks use
    /// first), then the script checks (per script file) and the text checks (per text
    /// archive).
    pub fn all() -> &'static [&'static dyn Check] {
        static ALL: [&dyn Check; 39] = [
            &world::WarpDestHeader,
            &world::WarpDestWarp,
            &world::WarpOffMap,
            &world::WarpNeverFires,
            &world::EventTrainerMissing,
            &world::EventOffMap,
            &world::EventTrainerBlind,
            &world::EventTriggerEmpty,
            &world::EventSameSpot,
            &world::HeaderRefRange,
            &world::HeaderWildDangling,
            &world::HeaderTextSplit,
            &world::MatrixMapRange,
            &world::MatrixHeaderRange,
            &world::MatrixHeaderElsewhere,
            &world::MapModelInvalid,
            &world::MapBdhcSize,
            &world::MapBuildingRange,
            &world::AreaTilesetRange,
            &data::TrainerNoParty,
            &data::TrainerEmptySpecies,
            &data::TrainerIdRange,
            &data::TrainerNoMoves,
            &data::TrainerDoubleOne,
            &data::EncounterEmptySlot,
            &data::EncounterLevelZero,
            &data::EncounterLevelHigh,
            &data::EncounterMinMax,
            &data::EncounterRateByte,
            &world::AssetUnreadable,
            &scripts::EventScriptMissing,
            &scripts::HeaderLevelTarget,
            &scripts::HeaderScriptKind,
            &scripts::ScriptMessageMissing,
            &scripts::ScriptUnreadable,
            &text::TextTooWide,
            &text::TextThreeLines,
            &text::TextEncode,
            &text::TextJsonBroken,
        ];
        &ALL
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// FNV-1a over the subject, the key and the bytes the check looked at (the fields it
/// judged, not the whole file), so editing something else in the same file does not
/// bring an ignored finding back. Stable across runs and versions: it is stored in ignore lists.
pub fn fingerprint(subject: Subject, key: &str, relevant_bytes: &[u8]) -> u64 {
    let (kind, number) = subject.tag();
    let mut hash = fnv(FNV_OFFSET, &[kind]);
    hash = fnv(hash, &number.to_le_bytes());
    // Length prefixes keep ("ab", "c") apart from ("a", "bc").
    hash = fnv(hash, &(key.len() as u64).to_le_bytes());
    hash = fnv(hash, key.as_bytes());
    hash = fnv(hash, &(relevant_bytes.len() as u64).to_le_bytes());
    fnv(hash, relevant_bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn fingerprint_changes_only_with_relevant_bytes() {
        let file = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let subject = Subject::EventFile(411);
        let base = fingerprint(subject, "warp 2", &file[2..6]);

        // Same subject, key and checked bytes: same value, however the rest of the file reads.
        let mut edited = file;
        edited[0] = 99;
        edited[7] = 99;
        assert_eq!(base, fingerprint(subject, "warp 2", &edited[2..6]));

        // A checked byte, the key, the subject's number or kind change it.
        edited[3] = 99;
        assert_ne!(base, fingerprint(subject, "warp 2", &edited[2..6]));
        assert_ne!(base, fingerprint(subject, "warp 3", &file[2..6]));
        assert_ne!(
            base,
            fingerprint(Subject::EventFile(412), "warp 2", &file[2..6])
        );
        assert_ne!(
            base,
            fingerprint(Subject::Header(411), "warp 2", &file[2..6])
        );
        // The split between key and bytes matters.
        assert_ne!(
            fingerprint(subject, "ab", b"c"),
            fingerprint(subject, "a", b"bc")
        );
        // Stored in ignore lists, so the value is fixed.
        assert_eq!(
            fingerprint(Subject::Project, "", &[]),
            0x0c79_e202_f9cc_d0cf
        );

        let f = Finding::new("x.y", Severity::Info, subject, "warp 2", "m", &file[2..6]);
        assert_eq!(f.fingerprint, base);
        assert!(f.args.is_empty() && f.fixes.is_empty() && f.file.is_none());
    }

    #[test]
    fn subjects_fixes_and_groups_are_written_for_the_ui() {
        assert_eq!(
            serde_json::to_value(Subject::EventFile(411)).unwrap(),
            serde_json::json!({ "kind": "eventFile", "index": 411 })
        );
        assert_eq!(
            serde_json::to_value(Subject::Project).unwrap(),
            serde_json::json!({ "kind": "project", "index": 0 })
        );
        for subject in [
            Subject::Header(1),
            Subject::EventFile(2),
            Subject::ScriptFile(3),
            Subject::TextArchive(4),
            Subject::Trainer(5),
            Subject::Encounter(6),
            Subject::Matrix(7),
            Subject::Map(8),
            Subject::Project,
            Subject::AreaData(9),
        ] {
            let text = serde_json::to_string(&subject).unwrap();
            assert_eq!(serde_json::from_str::<Subject>(&text).unwrap(), subject);
        }
        // The project may come without an index; an unknown kind is refused.
        assert_eq!(
            serde_json::from_str::<Subject>(r#"{ "kind": "project" }"#).unwrap(),
            Subject::Project
        );
        assert!(serde_json::from_str::<Subject>(r#"{ "kind": "warp", "index": 1 }"#).is_err());
        assert_eq!(
            serde_json::to_value(FixId::CreateScript).unwrap(),
            "create_script"
        );
        assert_eq!(
            serde_json::from_str::<FixId>(r#""swap_min_max""#).unwrap(),
            FixId::SwapMinMax
        );
        assert_eq!(
            serde_json::to_value(Group::PerScript).unwrap(),
            "per_script"
        );
        let f = Finding::new("x.y", Severity::Info, Subject::Project, "", "m", &[]);
        assert_eq!(
            serde_json::to_value(&f).unwrap()["fingerprint"],
            "0c79e202f9ccd0cf"
        );
    }

    #[test]
    fn registry_lists_each_code_once_in_its_group() {
        let all = Registry::all();
        let mut codes: Vec<&str> = all.iter().map(|c| c.code()).collect();
        assert_eq!(codes.len(), 39);
        let in_group = |group: Group| -> Vec<&str> {
            all.iter()
                .filter(|c| c.group() == group)
                .map(|c| c.code())
                .collect()
        };
        let cheap = in_group(Group::Cheap);
        assert_eq!(cheap.len(), 30);
        assert_eq!(cheap.last(), Some(&"asset.unreadable"));
        assert_eq!(
            in_group(Group::PerScript),
            [
                "event.script-missing",
                "header.level-target",
                "header.script-kind",
                "script.message-missing",
                "script.unreadable",
            ]
        );
        assert_eq!(
            in_group(Group::PerArchive),
            [
                "text.too-wide",
                "text.three-lines",
                "text.encode",
                "text.json-broken"
            ]
        );
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 39, "a code is registered twice");
        assert_eq!(
            Registry::get("warp.dest-header").map(|c| c.code()),
            Some("warp.dest-header")
        );
        assert_eq!(Registry::position("warp.dest-header"), Some(0));
        assert_eq!(Registry::position("text.json-broken"), Some(38));
        assert!(Registry::get("warp.no-such-check").is_none());
    }
}
