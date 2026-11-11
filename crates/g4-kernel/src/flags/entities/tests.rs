#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use super::*;
use crate::store::Store;
use crate::test_support::TextFixture;
use crate::textproject::ids::Resolver;
use crate::textproject::symbols::SymbolKind;

const FLAG_REGISTER: &str = "data/ids/flags.json";
const VAR_REGISTER: &str = "data/ids/vars.json";
const FLAG_FILE: &str = "data/flags/0148.json";
const FLAG_IDS: &str = ".rotom/command_database/.g4-ids-flags.json";
const VAR_IDS: &str = ".rotom/command_database/.g4-ids-vars.json";
const MEDAILLE: &str = "FLAG_HAT_MEDAILLE";

fn req(kind: FlagKind, number: u16, id: Option<&str>, description: Option<&str>) -> NameRequest {
    NameRequest {
        kind,
        number,
        id: id.map(str::to_owned),
        description: description.map(str::to_owned),
    }
}

fn refused(result: Result<NameReport, KernelError>) -> String {
    refusal(result).1
}

/// The code (`None` for a plain refusal) and the sentence of a refusal.
fn refusal(result: Result<NameReport, KernelError>) -> (Option<&'static str>, String) {
    match result {
        Err(KernelError::Refused(why)) => (None, why),
        Err(KernelError::Coded { code, message }) => (Some(code), message),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

/// Names flag 148 `FLAG_HAT_MEDAILLE` with a description, without a store.
fn name_medaille(fx: &TextFixture) -> NameReport {
    name_flag_files(
        &fx.project(),
        &req(FlagKind::Flag, 148, Some(MEDAILLE), Some("Orden erhalten")),
    )
    .unwrap()
}

#[test]
fn naming_writes_register_entity_and_id_file() {
    let fx = TextFixture::exported();
    // The export writes both registers empty.
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
    assert_eq!(fx.text(VAR_REGISTER), "{}\n");
    assert!(!fx.root().join(FLAG_IDS).exists());

    let report = name_medaille(&fx);
    assert!(report.created);
    assert_eq!(report.kind, FlagKind::Flag);
    assert_eq!(report.number, 148);
    assert_eq!(report.id, MEDAILLE);
    assert_eq!(report.change, None);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(report.files, paths(&[FLAG_REGISTER, FLAG_FILE]));

    assert_eq!(fx.text(FLAG_REGISTER), "{\n  \"FLAG_HAT_MEDAILLE\": 148\n}\n");
    assert_eq!(
        fx.text(FLAG_FILE),
        "{\n  \"id\": \"FLAG_HAT_MEDAILLE\",\n  \"description\": \"Orden erhalten\"\n}\n"
    );
    assert_eq!(fx.text(FLAG_IDS), "{\n  \"148\": \"FLAG_HAT_MEDAILLE\"\n}\n");
    assert_eq!(fx.text(VAR_REGISTER), "{}\n");

    let p = fx.project();
    let entity = FlagEntity {
        id: MEDAILLE.to_owned(),
        description: "Orden erhalten".to_owned(),
    };
    assert_eq!(
        read_entity(&p, FlagKind::Flag, 148).unwrap(),
        Some(entity.clone())
    );
    assert_eq!(read_entity(&p, FlagKind::Flag, 149).unwrap(), None);
    assert_eq!(
        own_names(&p, FlagKind::Flag).unwrap(),
        BTreeMap::from([(148, entity)])
    );
    assert!(own_names(&p, FlagKind::Var).unwrap().is_empty());

    // The own name is a second name: the pret name still resolves, the own one wins as id.
    let ids = Resolver::for_project(&p).unwrap();
    assert_eq!(ids.parse(SymbolKind::Flag, MEDAILLE), Ok(148));
    assert_eq!(ids.parse(SymbolKind::Flag, "FLAG_UNUSED_0x0094"), Ok(148));
    assert_eq!(ids.id(SymbolKind::Flag, 148).as_deref(), Some(MEDAILLE));
    assert_eq!(
        ids.id(SymbolKind::Flag, 149).as_deref(),
        Some("FLAG_UNUSED_0x0095")
    );

    // The check finds nothing to say about it.
    let report = crate::textproject::check(&p).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert!(report.differs.is_empty(), "{:?}", report.differs);
}

#[test]
fn naming_a_variable_uses_five_digits() {
    let fx = TextFixture::exported();
    let report = name_flag_files(
        &fx.project(),
        &req(FlagKind::Var, 0x406C, Some("VAR_MEIN_ZAEHLER"), None),
    )
    .unwrap();
    assert!(report.created);
    assert_eq!(report.files, paths(&[VAR_REGISTER, "data/vars/16492.json"]));
    assert_eq!(
        entity_rel(FlagKind::Var, 0x406C),
        PathBuf::from("data/vars/16492.json")
    );
    assert_eq!(
        entity_rel(FlagKind::Flag, 7),
        PathBuf::from("data/flags/0007.json")
    );
    assert_eq!(fx.text(VAR_REGISTER), "{\n  \"VAR_MEIN_ZAEHLER\": 16492\n}\n");
    // Without a description the entity holds the id alone.
    assert_eq!(
        fx.text("data/vars/16492.json"),
        "{\n  \"id\": \"VAR_MEIN_ZAEHLER\"\n}\n"
    );
    assert_eq!(fx.text(VAR_IDS), "{\n  \"16492\": \"VAR_MEIN_ZAEHLER\"\n}\n");
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
}

#[test]
fn description_only_keeps_the_id() {
    let fx = TextFixture::exported();
    name_medaille(&fx);
    let register = fx.text(FLAG_REGISTER);

    let report = name_flag_files(
        &fx.project(),
        &req(FlagKind::Flag, 148, None, Some("Den ersten Orden erhalten")),
    )
    .unwrap();
    assert!(!report.created);
    assert_eq!(report.id, MEDAILLE);
    assert_eq!(report.files, paths(&[FLAG_FILE]));
    assert_eq!(fx.text(FLAG_REGISTER), register);
    assert!(
        fx.text(FLAG_FILE)
            .contains("\"description\": \"Den ersten Orden erhalten\"")
    );

    // The same id with a description is fine too; an empty description drops the key.
    let report = name_flag_files(
        &fx.project(),
        &req(FlagKind::Flag, 148, Some(MEDAILLE), Some("")),
    )
    .unwrap();
    assert!(!report.created);
    assert_eq!(fx.text(FLAG_FILE), "{\n  \"id\": \"FLAG_HAT_MEDAILLE\"\n}\n");

    // Nothing to do: the same again, or no description at all.
    for request in [
        req(FlagKind::Flag, 148, Some(MEDAILLE), Some("")),
        req(FlagKind::Flag, 148, Some(MEDAILLE), None),
        req(FlagKind::Flag, 148, None, None),
    ] {
        assert_eq!(
            refused(name_flag_files(&fx.project(), &request)),
            nothing_to_change()
        );
    }
}

#[test]
fn naming_refuses_a_number_outside_the_save() {
    let fx = TextFixture::exported();
    let family = fx.project().family;
    assert_eq!(
        not_in_save_text(FlagKind::Flag, 0xFFF, family),
        "Number 0x0FFF is outside the save game (flags 0x0001 to 0x0B5F)."
    );
    assert_eq!(
        not_in_save_text(FlagKind::Var, 0x3FFF, family),
        "Number 0x3FFF is outside the save game (variables 0x4000 to 0x411F)."
    );
    for (kind, n, id) in [
        (FlagKind::Flag, 0xFFF, "FLAG_AUSSEN"),
        (FlagKind::Flag, 0, "FLAG_NULL"),
        (FlagKind::Flag, 0xB60, "FLAG_AUSSEN"),
        (FlagKind::Var, 0x3FFF, "VAR_AUSSEN"),
        (FlagKind::Var, 0x4120, "VAR_AUSSEN"),
        (FlagKind::Var, 0x8000, "VAR_AUSSEN"),
    ] {
        assert_eq!(
            refused(name_flag_files(
                &fx.project(),
                &req(kind, n, Some(id), None)
            )),
            not_in_save_text(kind, n, family),
            "{kind:?} {n:#X}"
        );
    }
    // Nothing written.
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
    assert_eq!(fx.text(VAR_REGISTER), "{}\n");
    assert!(!fx.root().join("data/flags").exists());
    assert!(!fx.root().join("data/vars").exists());
}

#[test]
fn naming_refuses_taken_and_malformed_ids() {
    let fx = TextFixture::exported();
    // A constant of rotom's own under the flags' prefix.
    fx.write(
        ".rotom/command_database/extra.json",
        b"{\"7\": \"FLAG_SEVEN\"}\n",
    );
    let format = "The new ID must start with FLAG_ and may then contain only A–Z, 0–9 and _ (at most 64 characters).";
    // With the code the app answers them by: a taken id `taken`, a malformed one `invalid_id`.
    let (taken, invalid) = (Some("taken"), Some("invalid_id"));
    let cases: Vec<(NameRequest, Option<&str>, String)> = vec![
        (
            req(FlagKind::Flag, 148, Some("FLAG_UNUSED_0x0095"), None),
            taken,
            "FLAG_UNUSED_0x0095 is the pret name of number 149.".to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("flag_x"), None),
            invalid,
            format.to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("VAR_X"), None),
            invalid,
            format.to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("FLAG_"), None),
            invalid,
            format.to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("FLAG_0149"), None),
            taken,
            "FLAG_0149 is the fallback ID of number 149.".to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("FLAG_SEVEN"), None),
            taken,
            "FLAG_SEVEN is already known to rotom as number 7.".to_owned(),
        ),
        (
            req(FlagKind::Var, 0x406C, Some("VAR_RESULT"), None),
            taken,
            "VAR_RESULT is the pret name of number 32780.".to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, None, Some("Ohne Namen")),
            None,
            id_required(),
        ),
    ];
    for (request, code, why) in &cases {
        assert_eq!(
            refusal(name_flag_files(&fx.project(), request)),
            (*code, why.clone()),
            "{request:?}"
        );
    }
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
    assert!(!fx.root().join("data/flags").exists());

    assert_eq!(taken_text("FLAG_X", TakenBy::Register(148)), "FLAG_X already exists (number 148).");
    name_medaille(&fx);
    // Taken in the register by another number; a second own name for 148.
    assert_eq!(
        refusal(name_flag_files(
            &fx.project(),
            &req(FlagKind::Flag, 150, Some(MEDAILLE), None)
        )),
        (
            Some("taken"),
            "FLAG_HAT_MEDAILLE already exists (number 148).".to_owned()
        )
    );
    assert_eq!(
        refused(name_flag_files(
            &fx.project(),
            &req(FlagKind::Flag, 148, Some("FLAG_ZWEITER_NAME"), None)
        )),
        has_own_name()
    );
    // The pret name of its own number (when it has the form of an id) and the own fallback
    // are fine; a pret name with a lower-case `0x` is no id.
    assert_eq!(
        refused(name_flag_files(
            &fx.project(),
            &req(FlagKind::Flag, 149, Some("FLAG_UNUSED_0x0095"), None)
        )),
        format
    );
    for (n, id) in [(0x997, "FLAG_VILLA_FURNITURE_TABLE"), (150, "FLAG_0150")] {
        let report =
            name_flag_files(&fx.project(), &req(FlagKind::Flag, n, Some(id), None)).unwrap();
        assert!(report.created, "{id}");
    }
}

#[test]
fn naming_refuses_the_dspre_layout() {
    let fx = TextFixture::dspre();
    let request = req(FlagKind::Flag, 148, Some(MEDAILLE), None);
    assert_eq!(
        refusal(name_flag_files(&fx.project(), &request)),
        (Some("not_text_project"), not_a_text_project())
    );
    let mut store = Store::new(Arc::new(fx.project()));
    assert_eq!(
        refusal(name_flag(&mut store, &request)),
        (Some("not_text_project"), not_a_text_project())
    );
    assert!(store.history().0.is_empty());
    assert!(!fx.root().join("data").exists());
    assert!(
        own_names(&fx.project(), FlagKind::Flag)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn name_flag_over_the_store_is_one_undo_step() {
    let fx = TextFixture::exported();
    let mut store = Store::new(Arc::new(fx.project()));
    let report = name_flag(
        &mut store,
        &req(FlagKind::Flag, 148, Some(MEDAILLE), Some("Orden erhalten")),
    )
    .unwrap();
    assert!(report.change.is_some());
    assert!(report.created);
    assert_eq!(
        store.history().0,
        ["Flag named: FLAG_HAT_MEDAILLE (0x0094)"]
    );
    assert!(fx.text(FLAG_IDS).contains(MEDAILLE));

    // One undo takes register and entity back, and the name out of the id file.
    assert_eq!(store.undo().unwrap(), [HEADER_TABLE]);
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
    assert!(!fx.root().join(FLAG_FILE).exists());
    assert!(
        std::fs::read_to_string(fx.root().join(FLAG_IDS))
            .map_or(true, |text| !text.contains(MEDAILLE))
    );

    // Redo puts both back, with the id file.
    store.redo().unwrap();
    assert_eq!(fx.text(FLAG_REGISTER), "{\n  \"FLAG_HAT_MEDAILLE\": 148\n}\n");
    assert!(fx.text(FLAG_FILE).contains("Orden erhalten"));
    assert!(fx.text(FLAG_IDS).contains(MEDAILLE));

    // A description, and a variable: their labels.
    name_flag(
        &mut store,
        &req(FlagKind::Flag, 148, None, Some("Neu")),
    )
    .unwrap();
    name_flag(
        &mut store,
        &req(FlagKind::Var, 0x406C, Some("VAR_MEIN_ZAEHLER"), None),
    )
    .unwrap();
    assert_eq!(
        store.history().0,
        [
            "Flag named: FLAG_HAT_MEDAILLE (0x0094)",
            "Description changed: FLAG_HAT_MEDAILLE",
            "Variable named: VAR_MEIN_ZAEHLER (0x406C)",
        ]
    );
}

const EVENT_JSON_0: &str = "data/events/0000.json";

/// Event file 0 gets a figure hidden by flag 148, as one undo step.
fn hide_figure_by_148(store: &mut Store) {
    store
        .edit_event(0, "Figur", |e| {
            e.object_events.push(crate::store::ObjectEvent {
                local_id: 0,
                graphics_id: 0,
                movement_type: 0,
                trainer_type: 0,
                hidden_flag: 148,
                script: 0,
                dir: 0,
                data: [0; 3],
                movement_range_x: 0,
                movement_range_z: 0,
                x: 0,
                z: 0,
                y: 0,
            });
            Ok(())
        })
        .unwrap();
}

/// No error in the import of the whole text project.
fn imports_cleanly(fx: &TextFixture) {
    let report = crate::textproject::check(&fx.project()).unwrap();
    assert!(
        report
            .problems
            .iter()
            .all(|p| p.severity != crate::text::Severity::Error),
        "{:?}",
        report.problems
    );
}

#[test]
fn undoing_a_name_the_saved_event_json_uses_names_the_number_again() {
    // A recipe's chain in one store (`studio mcp`, the kernel tests): name the flag, hide a
    // figure by it, save (the event JSON names it), undo both, save.
    let fx = TextFixture::exported();
    let event_before = fx.read(EVENT_JSON_0);
    let mut store = Store::new(Arc::new(fx.project()));
    name_flag(&mut store, &req(FlagKind::Flag, 148, Some(MEDAILLE), None)).unwrap();
    hide_figure_by_148(&mut store);
    store.save().unwrap();
    let event_named = fx.read(EVENT_JSON_0);
    assert!(fx.text(EVENT_JSON_0).contains("\"hidden_flag\": \"FLAG_HAT_MEDAILLE\""));

    store.undo().unwrap();
    store.undo().unwrap();
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
    // The event file the store saved names the number again: the register no longer knows
    // the name, and the number means the same flag.
    let text = fx.text(EVENT_JSON_0);
    assert!(!text.contains(MEDAILLE), "{text}");
    assert!(text.contains("\"hidden_flag\": 148"), "{text}");
    imports_cleanly(&fx);
    // The store's own event is no stale file: the save writes it back as it was.
    store.save().unwrap();
    assert_eq!(fx.read(EVENT_JSON_0), event_before);

    // Redo brings name and figure back; the save names the flag again.
    store.redo().unwrap();
    store.redo().unwrap();
    store.save().unwrap();
    assert_eq!(fx.read(EVENT_JSON_0), event_named);
    imports_cleanly(&fx);
}

#[test]
fn undoing_a_rename_the_saved_event_json_uses_names_the_number_again() {
    // The same with a rename: the event JSON saved after it names the new id, which the undo
    // of the rename drops from the register.
    let fx = TextFixture::exported();
    let mut store = Store::new(Arc::new(fx.project()));
    name_flag(&mut store, &req(FlagKind::Flag, 148, Some(MEDAILLE), None)).unwrap();
    crate::textproject::rename::rename_id(&mut store, MEDAILLE, "FLAG_HAT_ORDEN").unwrap();
    hide_figure_by_148(&mut store);
    store.save().unwrap();
    assert!(fx.text(EVENT_JSON_0).contains("\"hidden_flag\": \"FLAG_HAT_ORDEN\""));

    store.undo().unwrap();
    store.undo().unwrap();
    assert_eq!(fx.text(FLAG_REGISTER), "{\n  \"FLAG_HAT_MEDAILLE\": 148\n}\n");
    let text = fx.text(EVENT_JSON_0);
    assert!(!text.contains("FLAG_HAT_ORDEN"), "{text}");
    imports_cleanly(&fx);
    store.save().unwrap();
    imports_cleanly(&fx);
}
