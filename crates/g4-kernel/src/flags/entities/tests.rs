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
    match result {
        Err(KernelError::Refused(why)) => why,
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
            NOTHING_TO_CHANGE
        );
    }
}

#[test]
fn naming_refuses_a_number_outside_the_save() {
    let fx = TextFixture::exported();
    let family = fx.project().family;
    assert_eq!(
        not_in_save_text(FlagKind::Flag, 0xFFF, family),
        "Nummer 0x0FFF liegt außerhalb des Spielstands (Flags 0x0001 bis 0x0B5F)."
    );
    assert_eq!(
        not_in_save_text(FlagKind::Var, 0x3FFF, family),
        "Nummer 0x3FFF liegt außerhalb des Spielstands (Variablen 0x4000 bis 0x411F)."
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
    let format = "Die neue ID muss mit FLAG_ beginnen und darf danach nur A–Z, 0–9 und _ \
                  enthalten (höchstens 64 Zeichen).";
    let cases: Vec<(NameRequest, String)> = vec![
        (
            req(FlagKind::Flag, 148, Some("FLAG_UNUSED_0x0095"), None),
            "FLAG_UNUSED_0x0095 ist der pret-Name der Nummer 149.".to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("flag_x"), None),
            format.to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("VAR_X"), None),
            format.to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("FLAG_"), None),
            format.to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("FLAG_0149"), None),
            "FLAG_0149 ist die Ersatz-ID der Nummer 149.".to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, Some("FLAG_SEVEN"), None),
            "FLAG_SEVEN kennt rotom schon als Nummer 7.".to_owned(),
        ),
        (
            req(FlagKind::Var, 0x406C, Some("VAR_RESULT"), None),
            "VAR_RESULT ist der pret-Name der Nummer 32780.".to_owned(),
        ),
        (
            req(FlagKind::Flag, 148, None, Some("Ohne Namen")),
            ID_REQUIRED.to_owned(),
        ),
    ];
    for (request, why) in &cases {
        assert_eq!(
            &refused(name_flag_files(&fx.project(), request)),
            why,
            "{request:?}"
        );
    }
    assert_eq!(fx.text(FLAG_REGISTER), "{}\n");
    assert!(!fx.root().join("data/flags").exists());

    assert_eq!(taken_text("FLAG_X", TakenBy::Register(148)), "FLAG_X gibt es schon (Nummer 148).");
    name_medaille(&fx);
    // Taken in the register by another number; a second own name for 148.
    assert_eq!(
        refused(name_flag_files(
            &fx.project(),
            &req(FlagKind::Flag, 150, Some(MEDAILLE), None)
        )),
        "FLAG_HAT_MEDAILLE gibt es schon (Nummer 148)."
    );
    assert_eq!(
        refused(name_flag_files(
            &fx.project(),
            &req(FlagKind::Flag, 148, Some("FLAG_ZWEITER_NAME"), None)
        )),
        HAS_OWN_NAME
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
        refused(name_flag_files(&fx.project(), &request)),
        NOT_A_TEXT_PROJECT
    );
    let mut store = Store::new(Arc::new(fx.project()));
    assert_eq!(refused(name_flag(&mut store, &request)), NOT_A_TEXT_PROJECT);
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
        ["Flag benannt: FLAG_HAT_MEDAILLE (0x0094)"]
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
            "Flag benannt: FLAG_HAT_MEDAILLE (0x0094)",
            "Beschreibung geändert: FLAG_HAT_MEDAILLE",
            "Variable benannt: VAR_MEIN_ZAEHLER (0x406C)",
        ]
    );
}
