//! Example prompts: three recipes of `docs/agent-gate.md` as MCP prompts. Each one returns a
//! user message with a numbered plan that names the curated tools, so an agent (or a user
//! who picks the prompt in the client) starts from a working order of steps.

use serde_json::{Map, Value};

/// One prompt: name, title, description, its arguments (name, description, required) and the
/// message it returns for the given arguments (the required ones are there). The names (of the
/// prompt and of its arguments) are codes, the same in every language; the title, the
/// descriptions and the message come from the catalog in the language of the server.
pub struct PromptDef {
    pub name: &'static str,
    /// The catalog key of the title (`mcp.prompts.<name>.title`).
    title_key: &'static str,
    /// The catalog key of the description (`mcp.prompts.<name>.description`).
    description_key: &'static str,
    /// Each argument: its name, the catalog key of its description
    /// (`mcp.prompts.<name>.arg_<argument>`) and whether it is required.
    pub arguments: &'static [(&'static str, &'static str, bool)],
    pub text: fn(&Map<String, Value>) -> String,
}

/// The prompts of the server, in the order they are listed.
pub static PROMPTS: &[PromptDef] = &[
    PromptDef {
        name: "new_route",
        title_key: "mcp.prompts.new_route.title",
        description_key: "mcp.prompts.new_route.description",
        arguments: &[("name", "mcp.prompts.new_route.arg_name", true)],
        text: new_route,
    },
    PromptDef {
        name: "npc_with_item",
        title_key: "mcp.prompts.npc_with_item.title",
        description_key: "mcp.prompts.npc_with_item.description",
        arguments: &[
            ("header", "mcp.prompts.npc_with_item.arg_header", true),
            ("item", "mcp.prompts.npc_with_item.arg_item", false),
        ],
        text: npc_with_item,
    },
    PromptDef {
        name: "place_trainer",
        title_key: "mcp.prompts.place_trainer.title",
        description_key: "mcp.prompts.place_trainer.description",
        arguments: &[
            ("header", "mcp.prompts.place_trainer.arg_header", true),
            (
                "trainer_class",
                "mcp.prompts.place_trainer.arg_trainer_class",
                false,
            ),
        ],
        text: place_trainer,
    },
];

impl PromptDef {
    /// The title, in the language of the server.
    pub fn title(&self) -> String {
        t!(self.title_key).into_owned()
    }

    /// The description, in the language of the server.
    pub fn description(&self) -> String {
        t!(self.description_key).into_owned()
    }

    /// The description of the argument whose catalog key is `key` (the second part of an entry
    /// of [`PromptDef::arguments`]), in the language of the server.
    pub fn argument_description(key: &str) -> String {
        t!(key).into_owned()
    }
}

/// The prompt `name`.
pub fn find(name: &str) -> Option<&'static PromptDef> {
    PROMPTS.iter().find(|p| p.name == name)
}

/// What an argument of a prompt came as, as text: a string (not empty), or a number.
fn argument(args: &Map<String, Value>, name: &str) -> Option<String> {
    match args.get(name)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// The message of the prompt `name` for `args`: an error message (for the client) when the
/// prompt is unknown or a required argument is missing.
pub fn render(
    name: &str,
    args: &Map<String, Value>,
) -> Result<(&'static PromptDef, String), String> {
    let def = find(name).ok_or_else(|| t!("mcp.prompts.unknown", name = name).into_owned())?;
    for (arg, _, required) in def.arguments {
        if *required && argument(args, arg).is_none() {
            return Err(t!("mcp.prompts.missing_argument", name = name, arg = arg).into_owned());
        }
    }
    Ok((def, (def.text)(args)))
}

/// A plan: its text, then the closing note every plan ends with (what asks the user and what
/// to do when a call is refused).
fn with_closing_note(plan: impl std::fmt::Display) -> String {
    format!("{plan}\n\n{}", t!("mcp.prompts.irreversible"))
}

fn new_route(args: &Map<String, Value>) -> String {
    let name = argument(args, "name").unwrap_or_default();
    with_closing_note(t!("mcp.prompts.new_route.text", name = name))
}

fn npc_with_item(args: &Map<String, Value>) -> String {
    let header = argument(args, "header").unwrap_or_default();
    let item = match argument(args, "item") {
        Some(item) => t!("mcp.prompts.npc_with_item.item_named", item = item).into_owned(),
        None => t!("mcp.prompts.npc_with_item.item_ask").into_owned(),
    };
    with_closing_note(t!(
        "mcp.prompts.npc_with_item.text",
        header = header,
        item = item
    ))
}

fn place_trainer(args: &Map<String, Value>) -> String {
    let header = argument(args, "header").unwrap_or_default();
    let class = match argument(args, "trainer_class") {
        Some(class) => t!("mcp.prompts.place_trainer.class_named", class = class).into_owned(),
        None => t!("mcp.prompts.place_trainer.class_ask").into_owned(),
    };
    with_closing_note(t!(
        "mcp.prompts.place_trainer.text",
        header = header,
        class = class
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn prompts_are_listed_and_filled() {
        let names: Vec<&str> = PROMPTS.iter().map(|p| p.name).collect();
        assert_eq!(names, ["new_route", "npc_with_item", "place_trainer"]);
        let route = find("new_route").unwrap();
        let text = (route.text)(&args(json!({ "name": "Route 301" })));
        assert!(text.contains("Route 301"), "{text}");
        assert!(text.contains("create_area"), "{text}");
        assert!(text.contains("1."), "{text}");

        let (_, npc) = render(
            "npc_with_item",
            &args(json!({ "header": "415", "item": "Potion" })),
        )
        .unwrap();
        assert!(npc.contains("415") && npc.contains("Potion"), "{npc}");
        assert!(
            npc.contains("add_event") && npc.contains("edit_text"),
            "{npc}"
        );
        assert!(npc.contains("save_script"), "{npc}");

        let (_, trainer) = render("place_trainer", &args(json!({ "header": 1 }))).unwrap();
        assert!(trainer.contains("create_trainer"), "{trainer}");
        assert!(trainer.contains("cannot be undone"), "{trainer}");
        assert!(trainer.contains("index = the new trainer id"), "{trainer}");
        assert!(trainer.contains("--allow-irreversible"), "{trainer}");
        assert!(trainer.contains("needs_confirmation"), "{trainer}");
    }

    #[test]
    fn unknown_prompt_or_missing_argument_is_an_error() {
        assert!(render("nope", &args(json!({}))).is_err());
        for bad in [
            json!({}),
            json!({ "name": "" }),
            json!({ "name": "  " }),
            json!({ "name": [1] }),
        ] {
            let error = render("new_route", &args(bad)).err().unwrap();
            assert!(error.contains("name"), "{error}");
        }
        assert!(render("place_trainer", &args(json!({ "trainer_class": "x" }))).is_err());
        assert!(render("place_trainer", &args(json!({ "header": "x" }))).is_ok());
    }

    #[test]
    fn plans_only_name_tools_of_the_catalog() {
        use crate::mcp::catalog::catalog;
        let names: Vec<&str> = catalog().iter().map(|t| t.name).collect();
        let all = args(json!({ "name": "R", "header": "1", "item": "I", "trainer_class": "C" }));
        for def in PROMPTS {
            let text = (def.text)(&all);
            // Every word that looks like a tool (snake_case with an underscore) is one.
            for word in text.split(|c: char| !(c.is_ascii_lowercase() || c == '_')) {
                if word.contains('_') && !word.starts_with('_') && !word.ends_with('_') {
                    let known = names.contains(&word)
                        || [
                            "event_file_id",
                            "text_archive_id",
                            "script_file_id",
                            "matrix_id",
                            "source_map",
                            "with_headers",
                            "new_header",
                            "internal_name",
                            "trainer_type",
                            "trainer_class",
                            "needs_confirmation",
                            "not_permitted",
                            "npc_with_item",
                            "place_trainer",
                            "read_only",
                        ]
                        .contains(&word);
                    assert!(known, "{}: {word}", def.name);
                }
            }
        }
    }
}
