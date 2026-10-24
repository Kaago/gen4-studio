//! Creating trainers and writing their battle texts.
//!
//! A new trainer is three member files and three table entries: its properties and party
//! (`trainerProperties`, `trainerParty`), its name at index ID of the name bank, and its
//! offset in `trtblofs`. Trainers above the battle slots of the shared battle script need a
//! slot there, or they freeze the game when talked to; the ID of the "seen" slot is never
//! given out but gets a locked placeholder. Sources:
//! `docs/research/2026-10-23-3b3/trainer-script-extension-3b3b.md` §2.2,
//! `trainer-mechanics-3b3b.md` §1, §3, §4, §7.

use std::path::PathBuf;

use g4_formats::data::trainer::PartyFormat;
use g4_formats::data::trainer_doc::TrainerDoc;

use crate::data::arm9::arm9_layout;
use crate::data::names::{Bank, NameTables, bank_index, trainer_name};
use crate::data::trainer_rules::{NAME_MAX_CHARS, name_units};
use crate::error::KernelError;
use crate::project::{AssetId, AssetKind, Project};
use crate::scripts::{self, TrainerScriptMeta};
use crate::store::{ChangeId, Store};
use crate::text::{Problem, Severity, TextMessage, default_message_id};

pub use crate::store::PLACEHOLDER_REASON;

/// The refusal when the project's scripts are not adopted (the UI shows it as it is).
pub const ADOPT_FIRST: &str = "Erst Scripts übernehmen (Bereich Welt, Panel Script)";

/// Trainer IDs with a "defeated" flag of their own in Platinum and HGSS: flags `0x550 + ID`
/// up to `0x95F` (trainer-mechanics-3b3b.md §4), the fallback where `arm9_tables.yaml` has
/// no `trainer_flag_capacity`.
pub const TRAINER_FLAG_CAPACITY: u16 = 1040;

/// The name the placeholder trainer of the sight slot gets.
const PLACEHOLDER_NAME: &str = "-";

/// Battle text kind "before the battle"; its text should end with `\r` (the box waits for a
/// button press), trainer-mechanics-3b3b.md §3.
pub const TEXT_BEFORE_BATTLE: u16 = 0;

/// AI flags of a new trainer: 1, the basic AI, as the trainers measured in the spike.
const NEW_TRAINER_AI: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct CreateTrainerReport {
    /// The new trainer.
    pub id: u16,
    /// The placeholder written for the sight slot before it, if this call wrote one.
    pub placeholder: Option<u16>,
    /// Whether the battle script got new slots.
    pub script_extended: bool,
}

/// One battle text of a trainer: its kind (0 before the battle, 1 lost, 2 after the battle,
/// ...) and the text in chatot form.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrainerText {
    pub kind: u16,
    pub text: String,
}

/// What [`set_trainer_texts`] did: the undo step (`None` when nothing changed) and the
/// warnings about the trainer's texts.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct TrainerTextsOutcome {
    pub change: Option<ChangeId>,
    pub problems: Vec<Problem>,
}

fn refused(text: impl Into<String>) -> KernelError {
    KernelError::Refused(text.into())
}

/// Creates a trainer of class `class` named `name` at the next ID, see
/// [`create_trainer_with`]; the battle script and the sight slot come from
/// `trainer_scripts.yaml`, the flag capacity from `arm9_tables.yaml` (else 1040).
pub fn create_trainer(
    project: &Project,
    store: &mut Store,
    names: &mut NameTables,
    class: u8,
    name: &str,
) -> Result<CreateTrainerReport, KernelError> {
    if !scripts::is_adopted(project) {
        return Err(refused(ADOPT_FIRST));
    }
    let meta = scripts::project_trainer_script_meta(project)?;
    let capacity = arm9_layout(&project.game_code)
        .and_then(|l| l.trainer_flag_capacity)
        .unwrap_or(TRAINER_FLAG_CAPACITY);
    create_trainer_with(project, store, names, &meta, capacity, class, name)
}

/// Creates a trainer at the next ID (the trainer count). Refused, with nothing written,
/// unless: the scripts are adopted, the name bank has one name per trainer, the battle
/// script's sight slot is the one of `meta`, the ID is below `flag_capacity`, and the
/// name takes at most 8 UTF-16 units. When the next ID is the sight slot, a locked
/// placeholder (properties 0, party of 8 zero bytes, name "-") takes it first. An ID above
/// the battle slots extends them first ([`scripts::extend_battle_slots_with`]). The
/// trainer gets class `class`, no Pokémon, a party without moves and items and AI 1. The
/// members are written at once and are not undoable; the name (at index ID) and
/// `trtblofs[ID] = len(trtbl)` (gaps 0) are one undo step "Trainer <id> angelegt".
/// A failure after the script extension is [`KernelError::Partial`] naming what stays.
pub fn create_trainer_with(
    project: &Project,
    store: &mut Store,
    names: &mut NameTables,
    meta: &TrainerScriptMeta,
    flag_capacity: u16,
    class: u8,
    name: &str,
) -> Result<CreateTrainerReport, KernelError> {
    // Check and compute everything first: nothing is written before the plan stands.
    if !scripts::is_adopted(project) {
        return Err(refused(ADOPT_FIRST));
    }
    let units = name_units(name);
    if units > NAME_MAX_CHARS + 1 {
        return Err(refused(format!(
            "the trainer name has {} characters, at most {NAME_MAX_CHARS} fit",
            units - 1
        )));
    }
    store.set_placeholder_trainer(Some(meta.seen_id()))?;
    let props_count = store.member_count(AssetKind::TrainerProperties)?;
    let party_count = store.member_count(AssetKind::TrainerParty)?;
    if props_count != party_count {
        return Err(refused(format!(
            "{props_count} trainer properties but {party_count} trainer parties"
        )));
    }
    let family = project.family;
    let names_archive = bank_index(family, project.language, Bank::TrainerNames)
        .ok_or_else(|| refused("the game has no trainer name bank"))?;
    let name_count = names.names(store, Bank::TrainerNames)?.len();
    if name_count != props_count {
        return Err(refused(format!(
            "the trainer name bank has {name_count} names for {props_count} trainers"
        )));
    }
    let table_len = store.trainer_text_table(0)?.entries.len();
    store.trainer_text_offsets(0)?;
    for asset in [
        text_table_id(),
        text_offsets_id(),
        AssetId {
            kind: AssetKind::TextArchive,
            index: names_archive,
        },
    ] {
        if let Some(reason) = store.locked(asset) {
            return Err(KernelError::ReadOnly {
                asset,
                reason: reason.to_owned(),
            });
        }
    }
    // `trtblofs` holds byte offsets as u16; a new trainer points at the end of `trtbl`.
    let text_offset = u16::try_from(table_len * 4).map_err(|_| {
        refused(format!(
            "the trainer text table has {table_len} entries; a new trainer's offset does not \
             fit 16 bits"
        ))
    })?;
    let mut id = u16::try_from(props_count)
        .map_err(|_| refused(format!("{props_count} trainers, no ID is left")))?;
    let placeholder = (id == meta.seen_id()).then_some(id);
    if placeholder.is_some() {
        id += 1;
    }
    if id >= flag_capacity {
        return Err(refused(format!(
            "trainer {id} has no defeated flag: the game has flags for trainers 0 to {}",
            flag_capacity.saturating_sub(1)
        )));
    }
    let slots = scripts::battle_slots(project, meta)?;
    let extend = u32::from(id) > slots.max();
    for index in placeholder.iter().copied().chain([id]) {
        for kind in [AssetKind::TrainerProperties, AssetKind::TrainerParty] {
            let path = project.member_path(kind, index);
            if path.exists() {
                return Err(refused(format!(
                    "{} already exists, refusing to overwrite it",
                    path.display()
                )));
            }
        }
    }
    let doc = TrainerDoc {
        class,
        format: PartyFormat {
            moves: false,
            items: false,
        },
        flags_rest: 0,
        unknown: 0,
        items: [0; 4],
        ai: NEW_TRAINER_AI,
        battle_type: 0,
        party: Vec::new(),
    };
    let (props, party) = doc.encode(family);

    // The script first: if it fails, its source is back and nothing else was written.
    let mut progress = Progress {
        id,
        extended: None,
        created: Vec::new(),
    };
    if extend {
        let touched = scripts::extend_battle_slots_with(project, meta, id)?;
        progress.extended = Some(meta.battle_file);
        for index in touched {
            store
                .reload_from_disk(AssetId {
                    kind: AssetKind::TextArchive,
                    index,
                })
                .map_err(|e| progress.failed("reloading a text archive its compile changed", e))?;
        }
    }
    if let Some(slot) = placeholder {
        let index = store
            .append_trainer(&[0; 20], &[0; 8])
            .map_err(|e| progress.failed("writing the placeholder", e))?;
        progress.created.push(index);
        if index != slot {
            return Err(progress.failed(
                "writing the placeholder",
                KernelError::InvalidProject(format!("it became trainer {index}, not {slot}")),
            ));
        }
    }
    let index = store
        .append_trainer(&props, &party)
        .map_err(|e| progress.failed("writing the trainer", e))?;
    progress.created.push(index);
    if index != id {
        return Err(progress.failed(
            "writing the trainer",
            KernelError::InvalidProject(format!("it became trainer {index}, not {id}")),
        ));
    }
    let new_names: Vec<String> = placeholder
        .iter()
        .map(|_| trainer_name::wrap(PLACEHOLDER_NAME))
        .chain([trainer_name::wrap(name)])
        .collect();
    store
        .transaction(&format!("Trainer {id} angelegt"), |tx| {
            tx.edit_text_archive(names_archive, |doc| {
                for text in new_names {
                    let n = doc.messages.len();
                    doc.messages.push(TextMessage {
                        id: default_message_id(n),
                        text,
                        other: Default::default(),
                    });
                }
            })?;
            tx.edit_trainer_text_offsets(0, |o| {
                let len = (usize::from(id) + 1).max(o.offsets.len());
                o.offsets.resize(len, 0);
                o.offsets[usize::from(id)] = text_offset;
            })
        })
        .map_err(|e| progress.failed("setting its name and text offset", e))?;
    Ok(CreateTrainerReport {
        id,
        placeholder,
        script_extended: extend,
    })
}

/// Sets battle texts of trainer `id`. A kind the trainer has (in its block of `trtbl` from
/// `trtblofs[id]`) gets the new text; a new kind appends an entry to `trtbl` and a message to
/// the battle text archive, keeping the trainer's entries together: a trainer without a
/// block gets one at the end and `trtblofs[id]` (gaps 0), a block that is not at the end
/// moves there. A new kind with an empty text is skipped. One undo step. Refused as
/// [`KernelError::ReadOnly`] when `trtbl` and the archive differ in length; warns when the
/// text before the battle does not end with `\r`.
pub fn set_trainer_texts(
    store: &mut Store,
    names: &mut NameTables,
    id: u16,
    texts: Vec<TrainerText>,
) -> Result<TrainerTextsOutcome, KernelError> {
    let project = store.project();
    let archive = bank_index(project.family, project.language, Bank::BattleTexts)
        .ok_or_else(|| refused("the game has no battle text bank"))?;
    let trainers = store.member_count(AssetKind::Trainer)?;
    if usize::from(id) >= trainers {
        return Err(refused(format!(
            "trainer {id} does not exist ({trainers} trainers)"
        )));
    }
    if store.placeholder_trainer() == Some(id) {
        return Err(KernelError::ReadOnly {
            asset: AssetId {
                kind: AssetKind::Trainer,
                index: id,
            },
            reason: PLACEHOLDER_REASON.to_owned(),
        });
    }
    let messages = names.names(store, Bank::BattleTexts)?.to_vec();
    let table = store.trainer_text_table(0)?.entries.clone();
    let offsets = store.trainer_text_offsets(0)?.offsets.clone();
    if table.len() != messages.len() {
        return Err(KernelError::ReadOnly {
            asset: text_table_id(),
            reason: "trainer texts do not match the table".to_owned(),
        });
    }
    let block = own_block(&table, &offsets, id);

    // One text per kind, the last one given wins.
    let mut wanted: Vec<(u16, String)> = Vec::new();
    for TrainerText { kind, text } in texts {
        match wanted.iter_mut().find(|(k, _)| *k == kind) {
            Some(slot) => slot.1 = text,
            None => wanted.push((kind, text)),
        }
    }
    let in_block = |kind: u16| {
        block
            .clone()
            .and_then(|mut r| r.find(|&i| table[i].1 == kind))
    };
    let mut updates: Vec<(usize, String)> = Vec::new();
    let mut added: Vec<(u16, String)> = Vec::new();
    for (kind, text) in wanted {
        match in_block(kind) {
            Some(i) => updates.push((i, text)),
            None if text.is_empty() => {}
            None => added.push((kind, text)),
        }
    }
    // New kinds go to the end of `trtbl`, where the trainer's block must end: a block
    // elsewhere is copied there first, a trainer without one starts one there.
    let mut appended: Vec<((u16, u16), String)> = Vec::new();
    let mut new_offset = None;
    if !added.is_empty() {
        match &block {
            Some(r) if r.end == table.len() => {}
            other => {
                new_offset = Some(table.len() * 4);
                for i in other.clone().into_iter().flatten() {
                    let text = updates
                        .iter()
                        .find(|(u, _)| *u == i)
                        .map_or_else(|| messages[i].clone(), |(_, t)| t.clone());
                    appended.push((table[i], text));
                }
            }
        }
        appended.extend(added.into_iter().map(|(kind, text)| ((id, kind), text)));
    }
    let new_offset = new_offset
        .map(|offset| {
            u16::try_from(offset).map_err(|_| {
                refused(format!(
                    "the trainer text table has {} entries; the offset of trainer {id} does \
                     not fit 16 bits",
                    table.len()
                ))
            })
        })
        .transpose()?;

    // The text before the battle as it will be: given now, or kept.
    let before_battle = appended
        .iter()
        .rev()
        .find(|((_, kind), _)| *kind == TEXT_BEFORE_BATTLE)
        .map(|(_, t)| t.clone())
        .or_else(|| {
            updates
                .iter()
                .find(|(i, _)| table[*i].1 == TEXT_BEFORE_BATTLE)
                .map(|(_, t)| t.clone())
        })
        .or_else(|| in_block(TEXT_BEFORE_BATTLE).map(|i| messages[i].clone()));
    let problems = before_battle
        .filter(|text| !text.is_empty() && !text.ends_with("\\r"))
        .map(|_| Problem {
            severity: Severity::Warning,
            file: PathBuf::from(format!("trainers/{id:04}")),
            line: 1,
            col: 1,
            message: format!(
                "trainer {id}: the text before the battle does not end with \\r, so its box \
                 closes at once"
            ),
        })
        .into_iter()
        .collect();

    let (_, change) = store.transaction(&format!("Kampftexte Trainer {id}"), |tx| {
        tx.edit_text_archive(archive, |doc| {
            for (i, text) in &updates {
                if let Some(message) = doc.messages.get_mut(*i) {
                    message.text.clone_from(text);
                }
            }
            for (_, text) in &appended {
                let n = doc.messages.len();
                doc.messages.push(TextMessage {
                    id: default_message_id(n),
                    text: text.clone(),
                    other: Default::default(),
                });
            }
        })?;
        tx.edit_trainer_text_table(0, |t| {
            t.entries.extend(appended.iter().map(|(entry, _)| *entry));
        })?;
        tx.edit_trainer_text_offsets(0, |o| {
            if let Some(offset) = new_offset {
                let len = (usize::from(id) + 1).max(o.offsets.len());
                o.offsets.resize(len, 0);
                o.offsets[usize::from(id)] = offset;
            }
        })
    })?;
    Ok(TrainerTextsOutcome { change, problems })
}

/// The entries of trainer `id` in `table`: the run of its entries from `trtblofs[id]`, as
/// the game reads them; `None` when it has no offset or the offset starts no entry of its.
fn own_block(table: &[(u16, u16)], offsets: &[u16], id: u16) -> Option<std::ops::Range<usize>> {
    let offset = usize::from(*offsets.get(usize::from(id))?);
    if !offset.is_multiple_of(4) {
        return None;
    }
    let start = offset / 4;
    let len = table
        .get(start..)?
        .iter()
        .take_while(|(trainer, _)| *trainer == id)
        .count();
    (len > 0).then_some(start..start + len)
}

fn text_table_id() -> AssetId {
    AssetId {
        kind: AssetKind::TrainerTextTable,
        index: 0,
    }
}

fn text_offsets_id() -> AssetId {
    AssetId {
        kind: AssetKind::TrainerTextOffsets,
        index: 0,
    }
}

/// What `create_trainer_with` has written so far, for the error of a later step.
struct Progress {
    id: u16,
    /// The battle script file, once it has the new slots.
    extended: Option<u16>,
    /// Trainers written to disk.
    created: Vec<u16>,
}

impl Progress {
    /// `error` of `step` as it is when nothing was written yet, else
    /// [`KernelError::Partial`] naming what stays.
    fn failed(&self, step: &str, error: KernelError) -> KernelError {
        let mut done: Vec<String> = self
            .extended
            .iter()
            .map(|file| {
                format!(
                    "battle script file {file:04} has the slots up to trainer {}",
                    self.id
                )
            })
            .collect();
        done.extend(
            self.created
                .iter()
                .map(|t| format!("trainer {t} was written")),
        );
        if done.is_empty() {
            return error;
        }
        KernelError::Partial(format!(
            "creating trainer {}: {}; {step} failed: {error}",
            self.id,
            done.join(", ")
        ))
    }
}

#[cfg(all(test, feature = "test-support"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::scripts::{ScriptSource, SourceKind};
    use crate::test_support::{Synthetic, synthetic_project};

    /// A battle script like Platinum's file 1114, for three trainers: slots 1 to 3 share the
    /// battle code, slot 4 is the sight script.
    const BATTLE: &str = "script script_1 #[1-3]:\n    LockAll\n    FacePlayer\n    \
                          ReleaseAll\n    End\nscript script_4 #4:\n    LockAll\n    \
                          ReleaseAll\n    End\n";
    /// The game starts slot 4 for a trainer that sees the player.
    const SEEN_SCRIPT: u16 = 3003;
    /// The fixture's battle text archive (617) and trainer name bank (618).
    const BATTLE_TEXTS: u16 = 617;
    const TRAINER_NAMES: u16 = 618;

    struct Fixture {
        _s: Synthetic,
        project: Arc<Project>,
        store: Store,
        names: NameTables,
        meta: TrainerScriptMeta,
    }

    fn fixture(adopt: bool) -> Fixture {
        let s = synthetic_project();
        let project = Arc::new(Project::open(s.dir.path()).unwrap());
        let mut battle_file = 3;
        if adopt {
            scripts::adopt(&project, &mut |_| {}).unwrap();
            battle_file = scripts::new_script_file(&project, SourceKind::Script).unwrap();
            scripts::write_source(&project, battle_file, &ScriptSource::Rotom(BATTLE.into()))
                .unwrap();
            let report = scripts::compile(&project).unwrap();
            assert!(report.problems.is_empty(), "{:?}", report.problems);
        }
        let store = Store::new(project.clone());
        Fixture {
            _s: s,
            project,
            store,
            names: NameTables::new(),
            meta: TrainerScriptMeta {
                battle_file,
                seen_script: SEEN_SCRIPT,
            },
        }
    }

    fn trainer_id(index: u16) -> AssetId {
        AssetId {
            kind: AssetKind::Trainer,
            index,
        }
    }

    impl Fixture {
        fn create_capped(
            &mut self,
            capacity: u16,
            class: u8,
            name: &str,
        ) -> Result<CreateTrainerReport, KernelError> {
            create_trainer_with(
                &self.project,
                &mut self.store,
                &mut self.names,
                &self.meta,
                capacity,
                class,
                name,
            )
        }

        fn create(&mut self, class: u8, name: &str) -> Result<CreateTrainerReport, KernelError> {
            self.create_capped(TRAINER_FLAG_CAPACITY, class, name)
        }

        fn member(&self, kind: AssetKind, index: u16) -> Vec<u8> {
            std::fs::read(self.project.member_path(kind, index)).unwrap()
        }

        fn count(&self) -> usize {
            self.store.member_count(AssetKind::Trainer).unwrap()
        }

        fn texts(&mut self, archive: u16) -> Vec<String> {
            let doc = self.store.text_archive(archive).unwrap();
            doc.messages.iter().map(|m| m.text.clone()).collect()
        }

        fn offsets(&mut self) -> Vec<u16> {
            self.store.trainer_text_offsets(0).unwrap().offsets.clone()
        }

        fn table(&mut self) -> Vec<(u16, u16)> {
            self.store.trainer_text_table(0).unwrap().entries.clone()
        }

        fn source(&self) -> String {
            match scripts::read_source(&self.project, self.meta.battle_file).unwrap() {
                ScriptSource::Rotom(text) => text,
                other => panic!("{other:?}"),
            }
        }

        fn set_texts(
            &mut self,
            id: u16,
            texts: &[(u16, &str)],
        ) -> Result<TrainerTextsOutcome, KernelError> {
            let texts = texts
                .iter()
                .map(|&(kind, text)| TrainerText {
                    kind,
                    text: text.to_owned(),
                })
                .collect();
            set_trainer_texts(&mut self.store, &mut self.names, id, texts)
        }
    }

    /// Offset `k` of the jump table of a script member: the target of entry `k`.
    fn target(member: &[u8], k: usize) -> usize {
        let rel = i32::from_le_bytes(member[4 * k..4 * k + 4].try_into().unwrap());
        (4 * k as i64 + 4 + i64::from(rel)) as usize
    }

    #[test]
    fn create_trainer_appends_props_party_name_and_offset() {
        let mut f = fixture(true);
        let report = f.create(1, "Jens").unwrap();
        assert_eq!(
            report,
            CreateTrainerReport {
                id: 3,
                placeholder: None,
                script_extended: false
            }
        );
        assert_eq!(f.count(), 4);
        // Class 1, no Pokémon, no moves or items, AI 1.
        let mut props = [0u8; 20];
        props[1] = 1;
        props[12] = 1;
        assert_eq!(f.member(AssetKind::TrainerProperties, 3), props);
        assert_eq!(f.member(AssetKind::TrainerParty, 3), [0; 8]);
        assert_eq!(f.texts(TRAINER_NAMES)[3], "{TRAINER_NAME:Jens}");
        assert_eq!(f.texts(TRAINER_NAMES).len(), 4);
        // trtbl has two entries (8 bytes); the gap at trainer 2 is 0.
        assert_eq!(f.offsets(), [0, 0, 0, 8]);
        assert_eq!(
            f.store.history(),
            (vec!["Trainer 3 angelegt".to_owned()], vec![])
        );
        assert_eq!(f.store.trainer(3).unwrap().class, 1);
        assert_eq!(f.store.locked(trainer_id(3)), None);
        // Within the battle slots: the battle script stays as it was.
        assert_eq!(f.source(), BATTLE);
        // Undo takes the name and the offset back; the members stay.
        f.store.undo().unwrap();
        assert_eq!(f.texts(TRAINER_NAMES).len(), 3);
        assert_eq!(f.offsets(), [0, 0]);
        assert_eq!(f.count(), 4);
    }

    #[test]
    fn create_trainer_refuses_a_name_longer_than_seven_characters() {
        let mut f = fixture(true);
        let err = f.create(1, "Bartholomeus").unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert!(
            err.to_string().contains(&NAME_MAX_CHARS.to_string()),
            "{err}"
        );
        assert_eq!(f.count(), 3);
        assert_eq!(f.create(1, "Maximil").unwrap().id, 3);
    }

    #[test]
    fn create_trainer_skips_the_seen_slot_with_a_locked_placeholder() {
        let mut f = fixture(true);
        f.create(1, "Jens").unwrap();
        let report = f.create(1, "Lars").unwrap();
        assert_eq!(
            report,
            CreateTrainerReport {
                id: 5,
                placeholder: Some(4),
                script_extended: true
            }
        );
        assert_eq!(f.count(), 6);
        assert_eq!(f.member(AssetKind::TrainerProperties, 4), [0; 20]);
        assert_eq!(f.member(AssetKind::TrainerParty, 4), [0; 8]);
        let names = f.texts(TRAINER_NAMES);
        assert_eq!(names[4], "{TRAINER_NAME:-}");
        assert_eq!(names[5], "{TRAINER_NAME:Lars}");
        assert_eq!(f.offsets(), [0, 0, 0, 8, 0, 8]);
        assert_eq!(
            f.store.history().0,
            ["Trainer 3 angelegt", "Trainer 5 angelegt"]
        );
        // The placeholder is shown, but locked; the new trainer is not.
        f.store.trainer(4).unwrap();
        assert_eq!(f.store.locked(trainer_id(4)), Some(PLACEHOLDER_REASON));
        let err = f.store.edit_trainer(4, "x", |t| t.class = 9).unwrap_err();
        assert!(matches!(err, KernelError::ReadOnly { .. }), "{err}");
        f.store.trainer(5).unwrap();
        assert_eq!(f.store.locked(trainer_id(5)), None);
        // A store opened anew derives the lock from the sight slot ID when it loads the
        // trainer (in a real project from `trainer_scripts.yaml`: Platinum 929).
        let mut again = Store::new(f.project.clone());
        assert_eq!(again.placeholder_trainer(), Some(929));
        again.set_placeholder_trainer(Some(4)).unwrap();
        again.trainer(4).unwrap();
        assert_eq!(again.locked(trainer_id(4)), Some(PLACEHOLDER_REASON));
        again.trainer(5).unwrap();
        assert_eq!(again.locked(trainer_id(5)), None);
    }

    #[test]
    fn create_trainer_extends_the_battle_script_slots_and_keeps_the_code() {
        let mut f = fixture(true);
        let old = f.member(AssetKind::Script, f.meta.battle_file);
        f.create(1, "Jens").unwrap();
        f.create(1, "Lars").unwrap();
        assert_eq!(f.source(), BATTLE.replace("#[1-3]", "#[1-3, 5]"));
        let new = f.member(AssetKind::Script, f.meta.battle_file);
        // One more jump table entry (4 bytes); the rest, end marker and code, is unchanged.
        assert_eq!(new.len(), old.len() + 4);
        assert_eq!(old[16..], new[20..]);
        // Slot 5 (entry 4) runs the battle code, slot 4 (entry 3) still the sight code.
        assert_eq!(target(&new, 4), target(&new, 0));
        assert_ne!(target(&new, 3), target(&new, 0));
        assert_eq!(target(&old, 3) + 4, target(&new, 3));
        // The next trainer appends without a gap.
        let report = f.create(2, "Mia").unwrap();
        assert_eq!(
            report,
            CreateTrainerReport {
                id: 6,
                placeholder: None,
                script_extended: true
            }
        );
        assert_eq!(f.source(), BATTLE.replace("#[1-3]", "#[1-3, 5-6]"));
        assert_eq!(
            f.member(AssetKind::Script, f.meta.battle_file)[24..],
            old[16..]
        );
        let compiled = scripts::compile(&f.project).unwrap();
        assert!(compiled.problems.is_empty(), "{:?}", compiled.problems);
    }

    #[test]
    fn create_trainer_refuses_without_adopted_scripts() {
        let mut f = fixture(false);
        let err = f.create(1, "Jens").unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert_eq!(err.to_string(), ADOPT_FIRST);
        assert_eq!(f.count(), 3);
        assert_eq!(f.store.history(), (vec![], vec![]));
    }

    #[test]
    fn create_trainer_refuses_at_flag_capacity() {
        let mut f = fixture(true);
        let err = f.create_capped(3, 1, "Jens").unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert!(err.to_string().contains('3'), "{err}");
        assert_eq!(f.count(), 3);
        assert_eq!(f.source(), BATTLE);
        assert_eq!(f.store.history(), (vec![], vec![]));
        assert_eq!(f.create_capped(4, 1, "Jens").unwrap().id, 3);
    }

    #[test]
    fn create_trainer_refuses_a_battle_script_whose_sight_slot_moved() {
        let mut f = fixture(true);
        // The game would start slot 5 for the sight, the file has the sight script at 4.
        f.meta.seen_script = 3004;
        let err = f.create(1, "Jens").unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert!(err.to_string().contains("slot 4"), "{err}");
        assert_eq!(f.count(), 3);
        assert_eq!(f.source(), BATTLE);
    }

    #[test]
    fn create_trainer_refuses_when_the_name_bank_does_not_match() {
        let mut f = fixture(true);
        f.store
            .edit_text_archive(TRAINER_NAMES, "x", |d| d.messages.truncate(2))
            .unwrap();
        let err = f.create(1, "Jens").unwrap_err();
        assert!(matches!(err, KernelError::Refused(_)), "{err}");
        assert_eq!(f.count(), 3);
    }

    #[test]
    fn set_trainer_texts_appends_a_contiguous_block_and_fills_gaps_with_zero() {
        let mut f = fixture(false);
        let base = f.texts(BATTLE_TEXTS);
        let (lets, lost) = (base[0].as_str(), base[1].as_str());
        // Trainer 2 has no texts and no entry in trtblofs (two entries).
        let out = f
            .set_texts(2, &[(0, "Hallo!\\r"), (2, "Gut gemacht.")])
            .unwrap();
        assert!(out.change.is_some());
        assert!(out.problems.is_empty(), "{:?}", out.problems);
        assert_eq!(f.table(), [(1, 0), (1, 2), (2, 0), (2, 2)]);
        assert_eq!(f.offsets(), [0, 0, 8]);
        assert_eq!(
            f.texts(BATTLE_TEXTS),
            [lets, lost, "Hallo!\\r", "Gut gemacht."]
        );
        assert_eq!(f.store.history().0.len(), 1);
        // A kind it has changes in place; a new one joins the block at the end.
        f.set_texts(2, &[(0, "Los!\\r"), (1, "Mist.")]).unwrap();
        assert_eq!(f.table(), [(1, 0), (1, 2), (2, 0), (2, 2), (2, 1)]);
        assert_eq!(
            f.texts(BATTLE_TEXTS),
            [lets, lost, "Los!\\r", "Gut gemacht.", "Mist."]
        );
        assert_eq!(f.offsets(), [0, 0, 8]);
        // Trainer 1's block is not at the end: a new kind moves the block there.
        f.set_texts(1, &[(1, "Oh.")]).unwrap();
        assert_eq!(f.table()[5..], [(1, 0), (1, 2), (1, 1)]);
        assert_eq!(f.offsets(), [0, 20, 8]);
        assert_eq!(f.texts(BATTLE_TEXTS)[5..], [lets, lost, "Oh."]);
        // Nothing new, nothing recorded; an empty new kind is skipped.
        let lets = lets.to_owned();
        let out = f.set_texts(1, &[(0, &lets), (5, "")]).unwrap();
        assert_eq!(out.change, None);
        // Each call is one step.
        assert_eq!(f.store.history().0.len(), 3);
        f.store.undo().unwrap();
        f.store.undo().unwrap();
        f.store.undo().unwrap();
        assert_eq!(f.table(), [(1, 0), (1, 2)]);
        assert_eq!(f.offsets(), [0, 0]);
        assert_eq!(f.texts(BATTLE_TEXTS), base);
    }

    #[test]
    fn set_trainer_texts_warns_without_carriage_return() {
        let mut f = fixture(false);
        let out = f.set_texts(2, &[(0, "Hallo!")]).unwrap();
        assert_eq!(out.problems.len(), 1, "{:?}", out.problems);
        let problem = &out.problems[0];
        assert_eq!(problem.severity, Severity::Warning);
        assert!(problem.message.contains("trainer 2"), "{problem:?}");
        // The text is written as it is, not changed silently.
        assert_eq!(f.texts(BATTLE_TEXTS)[2], "Hallo!");
        let out = f.set_texts(2, &[(0, "Hallo!\\r")]).unwrap();
        assert!(out.problems.is_empty(), "{:?}", out.problems);
        // Another kind without `\r` is fine.
        let out = f.set_texts(2, &[(2, "Tschüss")]).unwrap();
        assert!(out.problems.is_empty(), "{:?}", out.problems);
    }

    #[test]
    fn set_trainer_texts_refuses_when_table_and_archive_differ() {
        let mut f = fixture(false);
        f.store
            .edit_text_archive(BATTLE_TEXTS, "x", |d| {
                d.messages.push(TextMessage {
                    id: default_message_id(2),
                    text: "extra".into(),
                    other: Default::default(),
                })
            })
            .unwrap();
        let err = f.set_texts(2, &[(0, "Hallo!\\r")]).unwrap_err();
        match err {
            KernelError::ReadOnly { reason, .. } => {
                assert_eq!(reason, "trainer texts do not match the table")
            }
            other => panic!("{other}"),
        }
        assert_eq!(f.table(), [(1, 0), (1, 2)]);
    }
}
