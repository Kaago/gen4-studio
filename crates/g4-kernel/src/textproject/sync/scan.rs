//! The scan: every watched file against its record, the changed ones grouped into units and
//! judged by the rule table of the module documentation. Writes nothing.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::build::fingerprint::{Fp, Seen, hash, is_racy, now_ns, same_stat, stat};
use crate::build::state::BuildState;
use crate::error::KernelError;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts;
use crate::start::read_base_mark;
use crate::text::{Problem, Severity, TextArchiveDoc, TextSide, TextSync, json_matches_member};

use super::super::base_layer::{BaseLayer, check_raw_blob, owned_paths, same_blob};
use super::super::canon::problem;
use super::super::codes::{MEMBER_WITHOUT_JSON_CODE, RAW_PATH_CODE, SCHEMA_CODE};
use super::super::import::plan_quirks_alone;
use super::super::manifest::{refusal, schema_state};
use super::super::registry::Symbols;
use super::super::source::EntitySource;
use super::watch::{Of, Unit, Watch, label};
use super::{ScanReport, SyncProgress, SyncUnit, UnitSide, key};

/// How often the scan reports its progress, in files.
const PROGRESS_EVERY: usize = 256;

/// How file `path` compares with its `recorded` fingerprint (of the scan at
/// `recorded_scan_ns`), the fingerprint seen now and whether the file was hashed: like
/// [`compare`](crate::build::fingerprint::compare), but a file without a record is not hashed
/// (the record hashes it), and the hashing is counted.
fn judge(
    path: &Path,
    recorded: Option<&Fp>,
    recorded_scan_ns: i128,
) -> Result<(Seen, Option<Fp>, bool), KernelError> {
    let Some(mut now) = stat(path)? else {
        let seen = if recorded.is_some() {
            Seen::Removed
        } else {
            Seen::Same
        };
        return Ok((seen, None, false));
    };
    let Some(recorded) = recorded else {
        return Ok((Seen::Added, Some(now), false));
    };
    if now.size != recorded.size {
        return Ok((Seen::Changed, Some(now), false));
    }
    if same_stat(&now, recorded) && !is_racy(recorded, recorded_scan_ns) {
        return Ok((Seen::Same, Some(*recorded), false));
    }
    now.xxh3 = Some(hash(path)?);
    let seen = if recorded.xxh3.is_some() && recorded.xxh3 == now.xxh3 {
        Seen::Same
    } else {
        Seen::Changed
    };
    Ok((seen, Some(now), true))
}

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub(super) fn scan(
    project: &Project,
    progress: &mut dyn FnMut(SyncProgress),
) -> Result<ScanReport, KernelError> {
    let start = Instant::now();
    if let Some(why) = refusal(schema_state(&project.root)?) {
        return Err(KernelError::Refused(why));
    }
    let at = now_ns();
    let state = BuildState::load_shared(&project.root);
    let record = state.sync.as_ref();
    let full = record.is_none();
    let recorded_scan = record.map_or(0, |r| r.scanned_at_ns);
    let watch = Watch::new(project)?;

    // Every watched file there is, and every recorded one that is gone.
    let mut files = watch.files()?;
    if let Some(record) = record {
        let present: HashSet<String> = files.iter().map(|rel| key(rel)).collect();
        for k in record.files.keys() {
            let rel = PathBuf::from(k);
            if !present.contains(k) && watch.watches(&rel) {
                files.push(rel);
            }
        }
    }
    let total = u32::try_from(files.len()).unwrap_or(u32::MAX);
    let mut seen: HashMap<String, (Seen, Option<Fp>)> = HashMap::with_capacity(files.len());
    let mut changed = Vec::new();
    let mut hashed = 0u32;
    for (i, rel) in files.iter().enumerate() {
        if i % PROGRESS_EVERY == 0 {
            progress(SyncProgress {
                done: u32::try_from(i).unwrap_or(u32::MAX),
                total,
            });
        }
        let k = key(rel);
        let recorded = record.and_then(|r| r.files.get(&k));
        let (verdict, fp, was_hashed) = judge(&project.root.join(rel), recorded, recorded_scan)?;
        hashed += u32::from(was_hashed);
        if verdict != Seen::Same || fp.as_ref() != recorded {
            changed.push(rel);
        }
        seen.insert(k, (verdict, fp));
    }
    progress(SyncProgress { done: total, total });

    // The units of the changed files; a changed manifest or register puts every entity's
    // unit up for judging.
    let mut ofs: Vec<Of> = changed.iter().flat_map(|rel| watch.of(rel)).collect();
    let global = !full && ofs.iter().any(|of| matches!(of, Of::Global(_)));
    if global {
        for rel in &files {
            ofs.extend(
                watch
                    .of(rel)
                    .into_iter()
                    .filter(|of| matches!(of, Of::Asset(_))),
            );
        }
    }
    let mut units: BTreeMap<String, Unit> = BTreeMap::new();
    let mut done: HashSet<String> = HashSet::new();
    for of in ofs {
        let id = format!("{of:?}");
        if !done.insert(id) {
            continue;
        }
        if let Some(unit) = watch.unit(&of)? {
            units.entry(unit.key.clone()).or_insert(unit);
        }
    }

    let mut judge = Judge {
        project,
        watch: &watch,
        seen: &seen,
        full,
        global,
        source: None,
        texts: TextSync::load(project),
        layer: OnceCell::new(),
        owned: OnceCell::new(),
        rerecord: Vec::new(),
    };
    let symbols: OnceCell<Option<Symbols>> = OnceCell::new();
    let mut report = ScanReport {
        stat_files: total,
        hashed,
        full,
        scanned_at_ns: at,
        ..ScanReport::default()
    };
    for unit in units.into_values() {
        let (side, problems) = judge.side(&unit)?;
        for rel in unit.sources.iter().chain(&unit.members) {
            let k = key(rel);
            if let Some((_, fp)) = seen.get(&k) {
                report.seen.insert(k, *fp);
            }
        }
        let symbols = symbols.get_or_init(|| Symbols::load(&project.root).ok());
        report.units.push(SyncUnit {
            label: label(project, &unit, symbols.as_ref()),
            key: unit.key,
            side,
            assets: unit.assets,
            sources: unit.sources,
            members: unit.members,
            problems,
        });
    }
    report.texts = judge.rerecord;
    report.ms = ms_since(start);
    Ok(report)
}

/// What judging the units needs, read once.
struct Judge<'a> {
    project: &'a Project,
    watch: &'a Watch<'a>,
    seen: &'a HashMap<String, (Seen, Option<Fp>)>,
    full: bool,
    global: bool,
    source: Option<EntitySource>,
    texts: TextSync,
    layer: OnceCell<Option<BaseLayer>>,
    owned: OnceCell<Option<BTreeSet<PathBuf>>>,
    /// Text archives in step whose record is to be taken anew.
    rerecord: Vec<u16>,
}

type Judged = (UnitSide, Vec<Problem>);

/// The side of a unit whose bytes differ, by which side changed (both unchanged is in step).
fn by_change(source: bool, member: bool) -> UnitSide {
    match (source, member) {
        (false, false) => UnitSide::InSync,
        (false, true) => UnitSide::Dspre,
        (true, false) => UnitSide::Files,
        (true, true) => UnitSide::Conflict,
    }
}

fn broken(problems: Vec<Problem>) -> Result<Judged, KernelError> {
    Ok((UnitSide::Broken, problems))
}

/// A problem without position (the whole file).
fn plain(file: &Path, message: String) -> Problem {
    Problem {
        severity: Severity::Error,
        file: file.to_path_buf(),
        line: 1,
        col: 1,
        message,
    }
}

fn shown(path: &Path) -> String {
    super::super::arts::slash(path)
        .to_string_lossy()
        .into_owned()
}

/// Whether `kind` is one of the binary kinds (no JSON, `data/<art>/NNNN.<ext>`).
fn is_binary(kind: AssetKind) -> bool {
    matches!(
        kind,
        AssetKind::MapTexture
            | AssetKind::BuildingTexture
            | AssetKind::BuildingModel
            | AssetKind::InteriorBuildingModel
    )
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, KernelError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(crate::error::io_at(path)(e)),
    }
}

impl Judge<'_> {
    fn seen(&self, rel: &Path) -> Seen {
        self.seen.get(&key(rel)).map_or(Seen::Same, |(s, _)| *s)
    }

    fn exists(&self, rel: &Path) -> bool {
        self.project.root.join(rel).is_file()
    }

    fn layer(&self) -> Option<&BaseLayer> {
        self.layer
            .get_or_init(|| {
                read_base_mark(&self.project.root)
                    .and_then(|mark| BaseLayer::open(&self.project.root, &mark.sha1))
            })
            .as_ref()
    }

    fn side(&mut self, unit: &Unit) -> Result<Judged, KernelError> {
        let source = self.full
            || unit.sources.iter().any(|s| self.seen(s) != Seen::Same)
            || (self.global && matches!(unit.of, Of::Asset(_)));
        let member = self.full || unit.members.iter().any(|m| self.seen(m) != Seen::Same);
        if !self.full
            && let Some(gone) = unit.members.iter().find(|m| self.seen(m) == Seen::Removed)
        {
            let json = unit.sources.first().map(|s| shown(s)).unwrap_or_default();
            return broken(vec![problem(
                gone,
                SCHEMA_CODE,
                format!(
                    "{} fehlt, {json} ist da; der Abgleich löscht keine Entität, Löschen kommt \
                     mit der Referenzsperre.",
                    shown(gone)
                ),
            )]);
        }
        if !source && !member {
            // Only the `stat` moved: the content is what was recorded.
            if let Of::Text(n) = unit.of {
                self.rerecord.push(n);
            }
            return Ok((UnitSide::InSync, Vec::new()));
        }
        match unit.of.clone() {
            Of::Asset(id) => self.asset(unit, id, source, member),
            Of::Text(n) => self.text(unit, n),
            Of::Script(n) => self.script(unit, n, source, member),
            Of::Blob(file) => self.blob(unit, &file, source, member),
            Of::Quirks => self.quirks(),
            Of::Global(_) | Of::Other(_) => Ok((UnitSide::InSync, Vec::new())),
        }
    }

    /// A JSON source of `unit` deleted since the record while its members stay: deleting is
    /// not the sync's.
    fn json_deleted(&self, unit: &Unit) -> Option<Problem> {
        if self.full {
            return None;
        }
        let gone = unit.sources.iter().find(|s| {
            s.extension().is_some_and(|e| e == "json") && self.seen(s) == Seen::Removed
        })?;
        let member = unit.members.iter().find(|m| self.exists(m))?;
        Some(problem(
            gone,
            MEMBER_WITHOUT_JSON_CODE,
            format!(
                "die Datei fehlt, {} ist da; der Abgleich löscht keine Entität, Löschen kommt \
                 mit der Referenzsperre.",
                shown(member)
            ),
        ))
    }

    fn asset(
        &mut self,
        unit: &Unit,
        id: AssetId,
        source: bool,
        member: bool,
    ) -> Result<Judged, KernelError> {
        if let Some(found) = self.json_deleted(unit) {
            return broken(vec![found]);
        }
        let has_member = unit.members.iter().any(|m| self.exists(m));
        if is_binary(id.kind) {
            // Without its file the member is the base's: no file is no deletion.
            if !has_member && unit.sources.iter().any(|s| self.exists(s)) {
                return Ok((UnitSide::Files, Vec::new()));
            }
        } else {
            let has_json = unit
                .sources
                .iter()
                .any(|s| s.extension().is_some_and(|e| e == "json") && self.exists(s));
            match (has_json, has_member) {
                // A member DSPRE appended: its JSON comes from it.
                (false, true) => return Ok((UnitSide::Dspre, Vec::new())),
                // A JSON without a member: the member comes from it.
                (true, false) => return Ok((UnitSide::Files, Vec::new())),
                (false, false) => return Ok((UnitSide::InSync, Vec::new())),
                (true, true) => {}
            }
        }
        if self.source.is_none() {
            self.source = Some(EntitySource::open(self.project)?);
        }
        let entities = self.source.as_mut().expect("opened");
        match entities.matches_members(self.project, &unit.assets) {
            Ok(true) => Ok((UnitSide::InSync, Vec::new())),
            Ok(false) => Ok((by_change(source, member), Vec::new())),
            Err(KernelError::Compile(problems)) => broken(problems),
            Err(e) => Err(e),
        }
    }

    fn text(&mut self, unit: &Unit, n: u16) -> Result<Judged, KernelError> {
        if let Some(found) = self.json_deleted(unit) {
            return broken(vec![found]);
        }
        let json = &unit.sources[0];
        let member = &unit.members[0];
        let lang = self.project.language.locale_key();
        let (side, rerecord) = self.texts.check(self.project, n)?;
        Ok(match side {
            TextSide::InSync | TextSide::Neither => {
                if rerecord {
                    self.rerecord.push(n);
                }
                (UnitSide::InSync, Vec::new())
            }
            TextSide::Unknown => {
                let root = &self.project.root;
                if json_matches_member(self.project, &root.join(json), &root.join(member)) {
                    self.rerecord.push(n);
                    (UnitSide::InSync, Vec::new())
                } else {
                    (UnitSide::Conflict, Vec::new())
                }
            }
            TextSide::JsonNewer | TextSide::OnlyJson => {
                let bytes = read_optional(&self.project.root.join(json))?.unwrap_or_default();
                match TextArchiveDoc::from_json(&bytes, lang) {
                    Ok(_) => (UnitSide::Files, Vec::new()),
                    Err(e) => (UnitSide::Broken, vec![plain(json, e.to_string())]),
                }
            }
            TextSide::MemberNewer | TextSide::OnlyMember => {
                let bytes = read_optional(&self.project.root.join(member))?.unwrap_or_default();
                match TextArchiveDoc::from_member(&bytes, lang) {
                    Ok(_) => (UnitSide::Dspre, Vec::new()),
                    Err(e) => (UnitSide::Broken, vec![plain(member, e.to_string())]),
                }
            }
            TextSide::Conflict => (UnitSide::Conflict, Vec::new()),
        })
    }

    /// A script file: in step where rotom's compile state names the hashes of its source and
    /// member as they are now; a member changed past rotom (DSPRE) is taken over by
    /// decompiling it, a changed source is rotom's to compile.
    fn script(
        &mut self,
        unit: &Unit,
        n: u16,
        source: bool,
        member: bool,
    ) -> Result<Judged, KernelError> {
        let root = &self.project.root;
        let present = unit.sources.iter().rev().find(|s| self.exists(s));
        let Some(present) = present else {
            if !self.full
                && let Some(gone) = unit.sources.iter().find(|s| self.seen(s) == Seen::Removed)
                && unit.members.iter().any(|m| self.exists(m))
            {
                return broken(vec![problem(
                    gone,
                    MEMBER_WITHOUT_JSON_CODE,
                    "die Quelle fehlt, das Member ist da; der Abgleich löscht keine Scriptdatei."
                        .to_owned(),
                )]);
            }
            return Ok((UnitSide::InSync, Vec::new()));
        };
        let member_rel = &unit.members[0];
        let Some(bytes) = read_optional(&root.join(member_rel))? else {
            return Ok((UnitSide::Files, Vec::new()));
        };
        let source_bytes =
            std::fs::read(root.join(present)).map_err(crate::error::io_at(&root.join(present)))?;
        let entry = scripts::state_entry(self.project, present)?;
        let hash_of = |field: &str| entry.as_ref().and_then(|e| e.get(field)?.as_u64());
        let (member_hash, source_hash) = (
            xxhash_rust::xxh3::xxh3_64(&bytes),
            xxhash_rust::xxh3::xxh3_64(&source_bytes),
        );
        if hash_of("output_hash") == Some(member_hash)
            && hash_of("source_hash") == Some(source_hash)
        {
            return Ok((UnitSide::InSync, Vec::new()));
        }
        // A member rotom wrote (the build compiled the source) is no change from outside.
        let member = member && hash_of("output_hash") != Some(member_hash);
        let side = by_change(source, member);
        if side != UnitSide::Dspre {
            return Ok((side, Vec::new()));
        }
        Ok(match scripts::decompile_member(self.project, n, &bytes) {
            Ok(Some(_)) => (UnitSide::Dspre, Vec::new()),
            Ok(None) => (
                UnitSide::Broken,
                vec![plain(
                    member_rel,
                    format!(
                        "rotom dekompiliert {} nicht; {} bleibt, wie sie ist.",
                        shown(member_rel),
                        shown(present)
                    ),
                )],
            ),
            Err(e) => (
                UnitSide::Broken,
                vec![plain(
                    member_rel,
                    format!("rotom dekompiliert {} nicht: {e}", shown(member_rel)),
                )],
            ),
        })
    }

    /// A blob: the file `file` against `data/raw/<file>`, or the base layer's file when there
    /// is no blob (no layer and no blob: unknown, in step on a full scan).
    fn blob(
        &mut self,
        unit: &Unit,
        file: &Path,
        source: bool,
        member: bool,
    ) -> Result<Judged, KernelError> {
        let project = self.project;
        let raw_rel = &unit.sources[0];
        let ours = read_optional(&project.root.join(file))?;
        let raw = read_optional(&project.root.join(raw_rel))?;
        if let Some(bytes) = &raw
            && source
        {
            let owned = self
                .owned
                .get_or_init(|| owned_paths(project).ok())
                .clone()
                .unwrap_or_default();
            if let Err(why) = check_raw_blob(project, &owned, file, bytes) {
                return broken(vec![problem(raw_rel, RAW_PATH_CODE, why)]);
            }
        }
        let Some(ours) = ours else {
            // The file is not there (and was not recorded): the blob makes it.
            return Ok(match raw {
                Some(_) => (UnitSide::Files, Vec::new()),
                None => (UnitSide::InSync, Vec::new()),
            });
        };
        let equal = match &raw {
            Some(raw) => Some(same_blob(project, file, &ours, raw)),
            None => match self.layer() {
                Some(layer) => Some(self.watch.range.layer_has_same(project, layer, file)?),
                None => None,
            },
        };
        if raw.is_none() && !self.full && self.seen(raw_rel) == Seen::Removed {
            // The blob went: the file goes back to the base's.
            let base = self.layer().map(|layer| layer.file(file)).transpose()?;
            match base {
                None => {
                    return broken(vec![plain(
                        raw_rel,
                        format!(
                            "{} fehlt; ohne Basisschicht ist die Fassung der Basis von {} nicht \
                             bekannt.",
                            shown(raw_rel),
                            shown(file)
                        ),
                    )]);
                }
                Some(None) => {
                    return broken(vec![plain(
                        raw_rel,
                        format!(
                            "{} fehlt und die Basis hat {} nicht; der Abgleich löscht keine \
                             Datei.",
                            shown(raw_rel),
                            shown(file)
                        ),
                    )]);
                }
                Some(Some(_)) => {}
            }
        }
        Ok(match equal {
            Some(true) => (UnitSide::InSync, Vec::new()),
            Some(false) => (by_change(source, member), Vec::new()),
            None if self.full => (UnitSide::InSync, Vec::new()),
            None => (by_change(source, member), Vec::new()),
        })
    }

    /// The quirks file: in step when seeding it would not change rotom's compile state.
    fn quirks(&mut self) -> Result<Judged, KernelError> {
        if !scripts::is_adopted(self.project) {
            return Ok((UnitSide::InSync, Vec::new()));
        }
        let plan = plan_quirks_alone(self.project)?;
        let errors: Vec<Problem> = plan
            .problems
            .into_iter()
            .filter(|p| p.severity == Severity::Error)
            .collect();
        if !errors.is_empty() {
            return broken(errors);
        }
        Ok(if plan.quirks_path.is_some() {
            (UnitSide::Files, Vec::new())
        } else {
            (UnitSide::InSync, Vec::new())
        })
    }
}
