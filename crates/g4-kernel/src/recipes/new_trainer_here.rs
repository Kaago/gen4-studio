//! „Neuer Trainer hier“: a new trainer with a team and battle texts, standing on a tile of a
//! header as an object that sees the player.
//!
//! The trainer is created as `create_trainer` creates it (members at once, its name and text
//! offset an undo barrier „Trainer N angelegt“; the placeholder of the sight slot and the
//! battle script slots as there). Behind the barrier the team, the battle texts and the
//! object event are **one** undo step. The object fights through the battle script
//! `3000 + id − 1` and the game's sight code (trainer type 1, the sight in `data[0]`), not
//! through a script of its own with `StartTrainerBattle`/`TrainerBattle`: such a script would
//! start the battle only when talked to and break the battle from walking past. See
//! `docs/formats/recipes.md`.

use g4_formats::data::trainer::PartyMon as TrainerMon;
use g4_formats::game::Family;
use serde::{Deserialize, Serialize};

use super::doors::{header_event_file, outside_map};
use super::events::next_free_local_id;
use super::new_figure::default_look;
use super::types::{
    Created, Draft, DraftEvent, DraftEventKind, DraftMessage, Outcome, check_header_id,
    check_writable, id, late, next_steps, refused,
};
use crate::data::names::{Bank, NameTables, bank_index};
use crate::data::trainer_create::{
    KIND_AFTER, KIND_LOSE, NextTrainer, TEXT_BEFORE_BATTLE, TrainerText, adopt_first,
    create_trainer_with, name_too_long, next_trainer, set_trainer_texts_labelled,
    trainer_flag_capacity,
};
use crate::error::KernelError;
use crate::project::AssetKind;
use crate::scripts::{self, FIRST_TRAINER_SCRIPT, TrainerScriptMeta};
use crate::store::{ObjectEvent, Store};
use crate::text::Problem;

/// The flag a trainer's defeat sets: `0x550 + id` (Platinum and HGSS, see
/// `trainer_create::TRAINER_FLAG_CAPACITY`).
const FIRST_TRAINER_FLAG: u16 = 0x550;
/// Most Pokémon in a team.
const MAX_PARTY: usize = 6;
/// The sight a trainer may have, in tiles.
const SIGHT: std::ops::RangeInclusive<u16> = 1..=15;
/// The movement type „Blickt nach oben“; `+ dir` looks down (1), left (2), right (3), as the
/// trainers of the gate scenarios stand.
const LOOKS_UP: u16 = 14;
/// Object trainer type: a trainer that sees the player.
const TRAINER_SEES: u16 = 1;

fn three() -> u16 {
    3
}

/// One Pokémon of the team: species and level; no moves and no items (the game gives the
/// moves of the level, `trainer_rules::default_moves`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PartyMon {
    pub species: u16,
    pub level: u8,
}

/// The battle texts: before the battle (ends on `\r`, the box waits for a button), when it
/// lost, after the battle. An empty text is not written.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BattleTexts {
    pub before: String,
    pub lose: String,
    pub after: String,
}

/// The input of „Neuer Trainer hier“: the tile (`x`, `z`, absolute tiles of the header's
/// matrix), where it looks (`dir` 0 up, 1 down, 2 left, 3 right) and how far it sees (1 to
/// 15, default 3), class and name (at most 7 characters), the team (1 to 6) and the battle
/// texts, the look (`None`: the family's [`super::new_figure::DefaultLook::trainer`]).
#[derive(Debug, Clone, Deserialize)]
pub struct NewTrainerHereInput {
    pub header: u16,
    pub x: u16,
    pub z: u16,
    pub dir: i16,
    #[serde(default = "three")]
    pub sight: u16,
    pub class: u8,
    pub name: String,
    pub party: Vec<PartyMon>,
    pub texts: BattleTexts,
    pub graphics_id: Option<u16>,
}

/// What [`apply`] did: trainer `trainer` (after the placeholder `placeholder` of the sight
/// slot, if this call wrote one; `script_extended` when the battle script got new slots),
/// object `object` of `event_file` with battle script `script`, its defeat flag
/// `trainer_flag`, and the warnings about its battle texts.
#[derive(Debug, Clone, Serialize)]
pub struct NewTrainerHereReport {
    pub trainer: u16,
    pub placeholder: Option<u16>,
    pub script_extended: bool,
    pub event_file: u16,
    pub object: u16,
    pub script: u16,
    pub trainer_flag: u16,
    pub text_problems: Vec<Problem>,
}

/// The undo label of the step behind the barrier.
fn label(name: &str) -> String {
    t!("recipes.trainer_placed", name = name).into_owned()
}

/// What [`check`] found, everything [`apply`] needs before its first write.
struct Checked {
    family: Family,
    event_file: u16,
    matrix: u16,
    /// The index the object gets in its event file.
    object: u16,
    /// The id the trainer gets and the placeholder before it, as `create_trainer` gives them.
    trainer: u16,
    placeholder: Option<u16>,
    /// The battle script gets new slots.
    extend: bool,
    battle_texts: u16,
    look: u16,
}

/// The battle script of the game (`trainer_scripts.yaml`, the battle file from the script
/// table) and its defeat flag capacity (`arm9_tables.yaml`).
fn game_setup(store: &Store) -> Result<(TrainerScriptMeta, u16), KernelError> {
    let project = store.project();
    if !scripts::is_adopted(project) {
        return Err(refused(adopt_first()));
    }
    Ok((
        scripts::project_trainer_script_meta(project)?,
        trainer_flag_capacity(project),
    ))
}

/// Every check before the first write, in this order: the header, adopted scripts
/// ([`adopt_first`]), a name that fits, a team of 1 to 6 Pokémon of species the game has at
/// levels 1 to 100, the sight, the direction, a writable event file, a tile of the header's
/// matrix no object of the file stands on, battle texts that match their table, the next id
/// as `create_trainer` gives it (the placeholder of the sight slot skipped, a defeat flag for
/// it) and a battle script whose slots `create_trainer` can extend.
fn check(
    store: &mut Store,
    input: &NewTrainerHereInput,
    meta: &TrainerScriptMeta,
    flag_capacity: u16,
) -> Result<Checked, KernelError> {
    check_header_id(store, input.header)?;
    let project = store.project_arc();
    let family = project.family;
    if !scripts::is_adopted(&project) {
        return Err(refused(adopt_first()));
    }
    if input.name.trim().is_empty() {
        return Err(refused(t!("recipes.trainer_needs_name")));
    }
    if let Some(why) = name_too_long(&input.name) {
        return Err(refused(why));
    }
    if input.party.is_empty() || input.party.len() > MAX_PARTY {
        return Err(refused(t!(
            "recipes.party_size",
            count = input.party.len(),
            max = MAX_PARTY
        )));
    }
    let species = store.member_count(AssetKind::Personal)?;
    for mon in &input.party {
        if mon.species == 0 || usize::from(mon.species) >= species {
            return Err(refused(t!(
                "recipes.species_missing",
                species = mon.species,
                max = species.saturating_sub(1)
            )));
        }
        if !(1..=100).contains(&mon.level) {
            return Err(refused(t!("recipes.level_invalid", level = mon.level)));
        }
    }
    if !SIGHT.contains(&input.sight) {
        return Err(refused(t!(
            "recipes.sight_invalid",
            sight = input.sight,
            min = SIGHT.start(),
            max = SIGHT.end()
        )));
    }
    if !(0..=3).contains(&input.dir) {
        return Err(refused(t!("recipes.direction_invalid", dir = input.dir)));
    }

    let event_file = header_event_file(store, input.header)?;
    let objects = store.event(event_file)?.object_events.clone();
    check_writable(store, id(AssetKind::Event, event_file))?;
    let (matrix, _) = outside_map(store, input.header, input.x, input.z)?;
    if let Some(k) = objects
        .iter()
        .position(|o| (o.x, o.z) == (input.x, input.z))
    {
        return Err(refused(t!(
            "recipes.object_on_tile",
            x = input.x,
            z = input.z,
            object = k
        )));
    }
    next_free_local_id(&objects)?;
    let object = u16::try_from(objects.len())
        .map_err(|_| refused(format!("event file {event_file:04} has too many objects")))?;

    let battle_texts = bank_index(family, project.language, Bank::BattleTexts)
        .ok_or_else(|| refused(t!("recipes.no_battle_texts")))?;
    let messages = NameTables::new().names(store, Bank::BattleTexts)?.len();
    let table = store.trainer_text_table(0)?.entries.len();
    if messages != table {
        return Err(refused(t!(
            "recipes.battle_texts_mismatch",
            archive = battle_texts : {:04},
            messages = messages,
            table = table
        )));
    }

    // The id as `create_trainer_with` gives it (one rule, `trainer_create::next_trainer`).
    let count = store.member_count(AssetKind::TrainerProperties)?;
    let NextTrainer {
        id: trainer,
        placeholder,
        extend,
    } = next_trainer(&project, count, meta, flag_capacity)?;
    if trainer == 0 {
        // Trainer 0 is no trainer in the game; its battle script would be 2999.
        return Err(refused(t!("recipes.no_trainers")));
    }
    let look = match input.graphics_id {
        Some(look) => look,
        None => default_look(family)?.trainer,
    };
    Ok(Checked {
        family,
        event_file,
        matrix,
        object,
        trainer,
        placeholder,
        extend,
        battle_texts,
        look,
    })
}

/// The battle script of trainer `id`: `3000 + id − 1`.
fn battle_script(id: u16) -> u16 {
    FIRST_TRAINER_SCRIPT + id - 1
}

/// The battle texts of `input` as `set_trainer_texts` takes them.
fn trainer_texts(texts: &BattleTexts) -> Vec<TrainerText> {
    [
        (TEXT_BEFORE_BATTLE, &texts.before),
        (KIND_LOSE, &texts.lose),
        (KIND_AFTER, &texts.after),
    ]
    .into_iter()
    .map(|(kind, text)| TrainerText {
        kind,
        text: text.clone(),
    })
    .collect()
}

/// What [`apply`] would do, with the battle script of the game's metadata.
pub fn plan(store: &mut Store, input: &NewTrainerHereInput) -> Result<Draft, KernelError> {
    check_header_id(store, input.header)?;
    let (meta, capacity) = game_setup(store)?;
    plan_with(store, input, &meta, capacity)
}

/// [`plan`] with the battle script `meta` and the flag capacity given (a fixture's own battle
/// script, as `create_trainer_with`): found with the same checks and without writing. The
/// object on the header's matrix with its look, the three battle texts for the battle text
/// bank, the sentences of the wizard (class and name, the id, the team, the defeat flag, a
/// placeholder, a text before the battle without `\r`), and the trainer as irreversible.
pub fn plan_with(
    store: &mut Store,
    input: &NewTrainerHereInput,
    meta: &TrainerScriptMeta,
    flag_capacity: u16,
) -> Result<Draft, KernelError> {
    let checked = check(store, input, meta, flag_capacity)?;
    let id = checked.trainer;
    let class = NameTables::new().class_name(store, u16::from(input.class));
    let mut notes = vec![
        t!(
            "recipes.note_trainer",
            class = class,
            name = input.name,
            id = id,
            party = input.party.len(),
            flag = FIRST_TRAINER_FLAG + id : {:04X}
        )
        .into_owned(),
    ];
    let mut irreversible = Vec::new();
    if let Some(p) = checked.placeholder {
        notes.push(t!("recipes.note_sight_placeholder", id = p).into_owned());
        irreversible.push(t!("recipes.irreversible_placeholder", id = p).into_owned());
    }
    irreversible.push(t!("recipes.irreversible_trainer_created", id = id).into_owned());
    if checked.extend {
        irreversible.push(
            t!(
                "recipes.irreversible_battle_script_slot",
                file = meta.battle_file : {:04},
                id = id
            )
            .into_owned(),
        );
    }
    notes.push(
        t!(
            "recipes.note_trainer_position",
            x = input.x,
            z = input.z,
            script = battle_script(id),
            sight = input.sight
        )
        .into_owned(),
    );
    if !input.texts.before.is_empty() && !input.texts.before.ends_with("\\r") {
        notes.push(t!("recipes.note_text_without_r").into_owned());
    }
    let messages = [&input.texts.before, &input.texts.lose, &input.texts.after]
        .into_iter()
        .filter(|text| !text.is_empty())
        .map(|text| DraftMessage {
            archive: Some(checked.battle_texts),
            text: text.clone(),
        })
        .collect();
    Ok(Draft {
        events: vec![DraftEvent {
            kind: DraftEventKind::Object,
            matrix: Some(checked.matrix),
            x: input.x,
            z: input.z,
            look: Some(checked.look),
            target: None,
        }],
        messages,
        notes,
        irreversible,
        ..Draft::default()
    })
}

/// Applies the recipe with the battle script of the game's metadata, see [`apply_with`].
pub fn apply(
    store: &mut Store,
    input: &NewTrainerHereInput,
) -> Result<Outcome<NewTrainerHereReport>, KernelError> {
    check_header_id(store, input.header)?;
    let (meta, capacity) = game_setup(store)?;
    apply_with(store, input, &meta, capacity)
}

/// Creates the trainer as `create_trainer_with` does (members at once, the barrier „Trainer N
/// angelegt“; [`Outcome::created`] names it and the placeholder), then one undo step
/// „Trainer „…“ hier aufgestellt“: one transaction sets the team and appends the object
/// (next free `local_id`, script `3000 + id − 1`, trainer type 1, sight in `data[0]`, the
/// movement looking in `dir`), then the battle texts go in (`set_trainer_texts_labelled`) and
/// both steps are merged. The name tables are read anew from the store here; a session's own
/// tables follow by the archives' text generation. A failure after the trainer was created
/// rolls the step back (the barrier stays, nothing to redo) and is [`KernelError::Partial`]
/// naming the trainer.
pub fn apply_with(
    store: &mut Store,
    input: &NewTrainerHereInput,
    meta: &TrainerScriptMeta,
    flag_capacity: u16,
) -> Result<Outcome<NewTrainerHereReport>, KernelError> {
    run(store, input, meta, flag_capacity, |_| Ok(()))
}

/// [`apply_with`] with `after_create` run on the store right after the trainer was created,
/// before the step behind the barrier: for tests of a placement that fails then (an event
/// file locked meanwhile).
#[cfg(feature = "test-support")]
pub fn apply_interrupted(
    store: &mut Store,
    input: &NewTrainerHereInput,
    meta: &TrainerScriptMeta,
    flag_capacity: u16,
    after_create: impl FnOnce(&mut Store) -> Result<(), KernelError>,
) -> Result<Outcome<NewTrainerHereReport>, KernelError> {
    run(store, input, meta, flag_capacity, after_create)
}

fn run(
    store: &mut Store,
    input: &NewTrainerHereInput,
    meta: &TrainerScriptMeta,
    flag_capacity: u16,
    after_create: impl FnOnce(&mut Store) -> Result<(), KernelError>,
) -> Result<Outcome<NewTrainerHereReport>, KernelError> {
    let checked = check(store, input, meta, flag_capacity)?;
    let Checked {
        family,
        event_file,
        object,
        look,
        ..
    } = checked;
    let generation = store.change_generation();
    let project = store.project_arc();
    let mut names = NameTables::new();

    let made = create_trainer_with(
        &project,
        store,
        &mut names,
        meta,
        flag_capacity,
        input.class,
        &input.name,
    )?;
    let trainer = made.id;
    let created = Created {
        trainers: made.placeholder.into_iter().chain([trainer]).collect(),
        ..Created::default()
    };
    const STEP: &str = "placing it";
    after_create(store).map_err(|e| late(&created, STEP, e))?;

    let mark = store.undo_depth();
    let label = label(&input.name);
    let script = battle_script(trainer);
    let party: Vec<TrainerMon> = input
        .party
        .iter()
        .map(|mon| TrainerMon {
            iv_or_difficulty: 0,
            byte1: 0,
            level: u16::from(mon.level),
            species_word: mon.species,
            item: None,
            moves: None,
            ball_capsule: (family != Family::Dp).then_some(0),
        })
        .collect();
    let placed = (|| {
        let ((), change) = store.transaction(&label, |tx| {
            tx.edit_trainer(trainer, |doc| doc.party = party)?;
            tx.edit_event(event_file, |f| {
                let local_id = next_free_local_id(&f.object_events)?;
                let mut data = [0; 3];
                data[0] = input.sight;
                f.object_events.push(ObjectEvent {
                    local_id,
                    graphics_id: look,
                    movement_type: LOOKS_UP + input.dir.unsigned_abs(),
                    trainer_type: TRAINER_SEES,
                    hidden_flag: 0,
                    script,
                    dir: input.dir,
                    data,
                    movement_range_x: 0,
                    movement_range_z: 0,
                    x: input.x,
                    z: input.z,
                    y: 0,
                });
                Ok(())
            })
        })?;
        let texts = set_trainer_texts_labelled(
            store,
            &mut names,
            trainer,
            trainer_texts(&input.texts),
            Some(&label),
        )?;
        Ok::<_, KernelError>((texts.change.or(change), texts.problems))
    })();
    let (change, text_problems) = match placed {
        Ok(placed) => placed,
        Err(e) => {
            let e = match store.rollback_to(mark) {
                Ok(_) => e,
                Err(rollback) => KernelError::Partial(format!("{e}; rolling back: {rollback}")),
            };
            return Err(late(&created, STEP, e));
        }
    };
    store.squash_since(mark, &label);

    Ok(Outcome {
        report: NewTrainerHereReport {
            trainer,
            placeholder: made.placeholder,
            script_extended: made.script_extended,
            event_file,
            object,
            script,
            trainer_flag: FIRST_TRAINER_FLAG + trainer,
            text_problems,
        },
        created: created.objects(),
        change,
        changed: store.changes_since(generation).1,
        next: next_steps(&["play_here"]),
    })
}
