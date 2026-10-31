//! Symbolic ids in the arguments of a call (`SPECIES_PIKACHU` for `25`, `MAP_T01` for a
//! header, `TRAINER_…` in a text project): [`resolve_args`] replaces every id at a place
//! [`SYMBOLIC_ARGS`] names by its number before the command runs, so every command keeps
//! taking numbers and every answer stays in numbers. One place for every way in (the curated
//! tools, `g4_call`, the calls of `g4_batch`): `Call::prepare`.

use g4_kernel::textproject::ids::{RefInput, Resolver, UNKNOWN_ID_CODE, unknown_id_text};
use g4_kernel::textproject::ref_kind;
use g4_kernel::textproject::symbols::SymbolKind;
use serde_json::Value;

use crate::errors::CmdError;
use crate::mcp::tools::ToolError;

/// One argument of a command that takes an id or a number: the command, the JSON pointer of
/// the value in its arguments (keyed as `dispatch` takes them; a segment `*` stands for every
/// key of an object) and how the kind of the id is known.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ArgRef {
    pub command: &'static str,
    pub pointer: &'static str,
    pub kind: RefKindOf,
}

/// How the kind of an argument's id is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefKindOf {
    /// Always this kind.
    Fixed(SymbolKind),
    /// From the text at this pointer of the arguments ([`kind_named`]: `species`, `learnset`,
    /// `evolution` → species, `move`, `item`, `trainer`, `trainer_class`, `header` → map,
    /// `flag`, `var` → the pret names and own ids of flags and variables); a
    /// kind without ids (an encounter file, a map) takes numbers only.
    FromKindField(&'static str),
    /// A `data_edit` patch value: by the patch's variant (the key the `*` stands for) and its
    /// `field` (`record::ref_kind` for personal data, moves and items; `species` of an
    /// encounter slot; `class` and `item0`…`item3` of a trainer).
    PatchField,
}

const fn arg(command: &'static str, pointer: &'static str, kind: RefKindOf) -> ArgRef {
    ArgRef {
        command,
        pointer,
        kind,
    }
}

use RefKindOf::{FromKindField, PatchField};

const SPECIES: RefKindOf = RefKindOf::Fixed(SymbolKind::Species);
const ITEM: RefKindOf = RefKindOf::Fixed(SymbolKind::Item);
const MAP: RefKindOf = RefKindOf::Fixed(SymbolKind::Map);
const TRAINER: RefKindOf = RefKindOf::Fixed(SymbolKind::Trainer);
const TRAINER_CLASS: RefKindOf = RefKindOf::Fixed(SymbolKind::TrainerClass);

/// Every argument that takes an id or a number.
pub(crate) const SYMBOLIC_ARGS: &[ArgRef] = &[
    arg("data_get", "/index", FromKindField("/kind")),
    arg("data_edit", "/index", FromKindField("/kind")),
    arg("data_raw", "/index", FromKindField("/kind")),
    arg("data_edit", "/patch/*/value", PatchField),
    arg("data_edit", "/patch/Evolution/target", SPECIES),
    arg("data_edit", "/patch/CopyMoves/from", SPECIES),
    arg("refs", "/target/index", FromKindField("/target/kind")),
    arg("refs", "/target/id", FromKindField("/target/kind")),
    arg("get_mon_icon", "/species", SPECIES),
    arg("get_item_icon", "/item", ITEM),
    arg("create_trainer", "/class", TRAINER_CLASS),
    arg("set_trainer_texts", "/trainer", TRAINER),
    arg("get_header", "/id", MAP),
    arg("set_header", "/id", MAP),
    arg("split_encounter_file", "/header", MAP),
    arg("split_room", "/header", MAP),
    arg("resolve_script", "/header", MAP),
    arg("update_event", "/patch/dest_header_id", MAP),
    arg("plan_reset", "/target/index", FromKindField("/target/kind")),
    arg("plan_reset", "/target/id", FromKindField("/target/kind")),
    arg(
        "reset_to_original",
        "/target/index",
        FromKindField("/target/kind"),
    ),
    arg(
        "reset_to_original",
        "/target/id",
        FromKindField("/target/kind"),
    ),
    arg("diff_entity", "/index", FromKindField("/kind")),
    arg("flag_usage", "/number", FromKindField("/kind")),
    arg("name_flag", "/number", FromKindField("/kind")),
];

/// The kind a `kind` field names (see [`RefKindOf::FromKindField`]).
fn kind_named(name: &str) -> Option<SymbolKind> {
    match name {
        "species" | "learnset" | "evolution" => Some(SymbolKind::Species),
        "move" => Some(SymbolKind::Move),
        "item" => Some(SymbolKind::Item),
        "trainer" => Some(SymbolKind::Trainer),
        "trainer_class" => Some(SymbolKind::TrainerClass),
        "header" => Some(SymbolKind::Map),
        "flag" => Some(SymbolKind::Flag),
        "var" => Some(SymbolKind::Var),
        _ => None,
    }
}

/// The kind of the value of `data_edit` patch variant `variant` with `field`: a reference
/// field of personal data, a move or an item (`record::ref_kind`), the species of an encounter
/// slot, a trainer's class and items.
fn patch_kind(variant: &str, field: &str) -> Option<SymbolKind> {
    match variant {
        "Personal" => ref_kind("personal", field),
        "Move" => ref_kind("move", field),
        "Item" => ref_kind("item", field),
        "Encounter" if field == "species" => Some(SymbolKind::Species),
        "Trainer" => match field {
            "class" => Some(SymbolKind::TrainerClass),
            "item0" | "item1" | "item2" | "item3" => Some(SymbolKind::Item),
            _ => None,
        },
        _ => None,
    }
}

/// A place of an argument: the value, and the key the `*` of its pointer stood for.
struct Place<'v> {
    value: &'v mut Value,
    wildcard: Option<String>,
}

/// Every value at `pointer` in `root` (a `*` segment: every key of an object).
fn places<'v>(root: &'v mut Value, pointer: &str) -> Vec<Place<'v>> {
    let segments: Vec<&str> = pointer.split('/').skip(1).collect();
    let mut out = Vec::new();
    collect(root, &segments, None, &mut out);
    out
}

fn collect<'v>(
    value: &'v mut Value,
    segments: &[&str],
    wildcard: Option<String>,
    out: &mut Vec<Place<'v>>,
) {
    let Some((first, rest)) = segments.split_first() else {
        out.push(Place { value, wildcard });
        return;
    };
    let Value::Object(map) = value else {
        return;
    };
    if *first == "*" {
        for (key, inner) in map.iter_mut() {
            collect(inner, rest, Some(key.clone()), out);
        }
    } else if let Some(inner) = map.get_mut(*first) {
        collect(inner, rest, wildcard, out);
    }
}

/// The kind of the id at a place of `arg` in `args` (`args` before any change; `wildcard`
/// the key the `*` stood for); `None`: the place takes numbers only.
fn kind_at(arg: &ArgRef, args: &Value, wildcard: Option<&str>) -> Option<SymbolKind> {
    match arg.kind {
        RefKindOf::Fixed(kind) => Some(kind),
        FromKindField(pointer) => args
            .pointer(pointer)
            .and_then(Value::as_str)
            .and_then(kind_named),
        PatchField => {
            let variant = wildcard?;
            let field = args
                .pointer(&format!("/patch/{variant}/field"))
                .and_then(Value::as_str)?;
            patch_kind(variant, field)
        }
    }
}

/// The `unknown_id` error with `message`.
fn unknown(message: String) -> ToolError {
    CmdError::refused(UNKNOWN_ID_CODE, message).into()
}

/// `args` of `command` with every id at a place of [`SYMBOLIC_ARGS`] replaced by its number
/// (`ids` resolves it); numbers and every other value stay as they are. An id nothing knows is
/// `unknown_id` with `Unbekannte ID: <id>.` (outside a text project a register id says that it
/// needs one), and so is an id where only a number fits (an encounter file, a map). A
/// `data_edit` patch value of a field that is no reference keeps its text (the command
/// refuses it as it would any wrong value).
pub(crate) fn resolve_args(
    command: &str,
    mut args: Value,
    ids: &Resolver,
) -> Result<Value, ToolError> {
    let original = args.clone();
    for arg in SYMBOLIC_ARGS.iter().filter(|a| a.command == command) {
        for place in places(&mut args, arg.pointer) {
            let Value::String(text) = &*place.value else {
                continue;
            };
            let kind = kind_at(arg, &original, place.wildcard.as_deref());
            let number = match kind {
                Some(kind) => ids
                    .number(kind, &RefInput::Id(text.clone()))
                    .map_err(unknown)?,
                // A patch value of a field that is no reference: left to the command.
                None if arg.kind == PatchField => continue,
                // A number as text is a number; an id where none fits is unknown.
                None => text
                    .parse::<u32>()
                    .ok()
                    .filter(|_| text.bytes().all(|b| b.is_ascii_digit()))
                    .ok_or_else(|| unknown(unknown_id_text(text)))?,
            };
            *place.value = Value::from(number);
        }
    }
    Ok(args)
}

/// Whether `args` of `command` hold text at a place of [`SYMBOLIC_ARGS`] (only then the
/// project's resolver is needed).
pub(crate) fn has_ids(command: &str, args: &Value) -> bool {
    if !SYMBOLIC_ARGS.iter().any(|a| a.command == command) {
        return false;
    }
    let mut args = args.clone();
    SYMBOLIC_ARGS
        .iter()
        .filter(|a| a.command == command)
        .any(|arg| {
            places(&mut args, arg.pointer)
                .iter()
                .any(|place| place.value.is_string())
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::commands::dispatch::COMMANDS;
    use serde_json::json;

    fn fixed() -> Resolver {
        Resolver::fixed()
    }

    #[test]
    fn species_id_and_number_give_the_same_arguments() {
        let by_number = json!({ "kind": "species", "index": 25 });
        let by_id = json!({ "kind": "species", "index": "SPECIES_PIKACHU" });
        for command in ["data_get", "data_raw"] {
            assert!(has_ids(command, &by_id) && !has_ids(command, &by_number));
            assert_eq!(
                resolve_args(command, by_id.clone(), &fixed()).unwrap(),
                by_number,
                "{command}"
            );
            assert_eq!(
                resolve_args(command, by_number.clone(), &fixed()).unwrap(),
                by_number
            );
        }
        // A learnset is a species' too; a number as text is a number.
        assert_eq!(
            resolve_args(
                "data_get",
                json!({ "kind": "learnset", "index": "SPECIES_PIKACHU" }),
                &fixed()
            )
            .unwrap(),
            json!({ "kind": "learnset", "index": 25 })
        );
        assert_eq!(
            resolve_args("get_mon_icon", json!({ "species": "25" }), &fixed()).unwrap(),
            json!({ "species": 25 })
        );
        assert_eq!(
            resolve_args(
                "refs",
                json!({ "target": { "kind": "move", "index": "MOVE_POUND" } }),
                &fixed()
            )
            .unwrap(),
            json!({ "target": { "kind": "move", "index": 1 } })
        );
        assert_eq!(
            resolve_args(
                "plan_reset",
                json!({ "target": { "kind": "species", "index": "SPECIES_BULBASAUR" } }),
                &fixed()
            )
            .unwrap(),
            json!({ "target": { "kind": "species", "index": 1 } })
        );
        // Another command, or another place, stays as it is.
        let other = json!({ "query": "SPECIES_PIKACHU", "limit": 5 });
        assert_eq!(
            resolve_args("search_text", other.clone(), &fixed()).unwrap(),
            other
        );
    }

    #[test]
    fn flag_and_var_numbers_take_pret_names() {
        let synthetic = g4_kernel::test_support::synthetic_project();
        let project = g4_kernel::project::Project::open(synthetic.dir.path()).unwrap();
        let platinum = Resolver::for_project(&project).unwrap();
        for command in ["flag_usage", "name_flag"] {
            assert_eq!(
                resolve_args(
                    command,
                    json!({ "kind": "flag", "number": "FLAG_UNUSED_0x0094" }),
                    &platinum
                )
                .unwrap(),
                json!({ "kind": "flag", "number": 148 }),
                "{command}"
            );
        }
        assert_eq!(
            resolve_args(
                "flag_usage",
                json!({ "kind": "var", "number": "VAR_UNUSED_0x406C" }),
                &platinum
            )
            .unwrap(),
            json!({ "kind": "var", "number": 0x406C })
        );
    }

    #[test]
    fn patch_values_of_reference_fields_are_resolved() {
        let patch = |variant: &str, field: &str, value: Value| {
            json!({ "kind": "species", "index": 1,
                    "patch": { variant: { "field": field, "value": value } } })
        };
        assert_eq!(
            resolve_args(
                "data_edit",
                patch("Personal", "type1", json!("TYPE_FIRE")),
                &fixed()
            )
            .unwrap(),
            patch("Personal", "type1", json!(10))
        );
        assert_eq!(
            resolve_args(
                "data_edit",
                patch("Personal", "item1", json!("ITEM_MASTER_BALL")),
                &fixed()
            )
            .unwrap(),
            patch("Personal", "item1", json!(1))
        );
        // A field that is no reference keeps its text (the command refuses it).
        let attack = patch("Personal", "attack", json!("SPECIES_PIKACHU"));
        assert_eq!(
            resolve_args("data_edit", attack.clone(), &fixed()).unwrap(),
            attack
        );
        // The species of an encounter slot.
        let slot = |value: Value| {
            json!({ "kind": "encounter", "index": 3, "patch": { "Encounter": {
                "method": "grass", "slot": 0, "field": "species", "value": value } } })
        };
        assert_eq!(
            resolve_args("data_edit", slot(json!("SPECIES_PIKACHU")), &fixed()).unwrap(),
            slot(json!(25))
        );
    }

    #[test]
    fn an_unknown_id_is_unknown_id() {
        let err = resolve_args(
            "data_get",
            json!({ "kind": "species", "index": "SPECIES_NOPE" }),
            &fixed(),
        )
        .unwrap_err();
        match err {
            ToolError::Failed { code, message, .. } => {
                assert_eq!(code, "unknown_id");
                assert_eq!(message, unknown_id_text("SPECIES_NOPE"));
            }
            other => panic!("{other:?}"),
        }
        // A kind without ids takes numbers only.
        let err = resolve_args(
            "data_get",
            json!({ "kind": "encounter", "index": "ENCOUNTER_X" }),
            &fixed(),
        )
        .unwrap_err();
        assert!(matches!(err, ToolError::Failed { ref code, .. } if code == "unknown_id"));
        // A trainer outside a text project: unknown, it says why.
        let err = resolve_args(
            "set_trainer_texts",
            json!({ "trainer": "TRAINER_X", "texts": [] }),
            &fixed(),
        )
        .unwrap_err();
        assert!(err.message().contains("TRAINER_X"), "{err}");
    }

    #[test]
    fn every_symbolic_arg_names_a_command_of_the_table() {
        assert!(!SYMBOLIC_ARGS.is_empty());
        for arg in SYMBOLIC_ARGS {
            assert!(COMMANDS.contains(&arg.command), "{}", arg.command);
            assert!(arg.pointer.starts_with('/'), "{}", arg.pointer);
            if let RefKindOf::FromKindField(field) = arg.kind {
                assert!(field.starts_with('/'), "{field}");
            }
        }
        for command in [
            "data_get",
            "data_edit",
            "data_raw",
            "refs",
            "get_mon_icon",
            "get_item_icon",
            "create_trainer",
            "set_trainer_texts",
            "get_header",
            "set_header",
            "split_encounter_file",
            "split_room",
            "resolve_script",
            "update_event",
            "plan_reset",
            "reset_to_original",
            "diff_entity",
            "flag_usage",
            "name_flag",
        ] {
            assert!(
                SYMBOLIC_ARGS.iter().any(|a| a.command == command),
                "{command}"
            );
        }
    }
}
