//! The script texts of the story recipes of 3d.7, filled from the templates
//! `metadata/recipes/story_*.rotom` (see [`template`]): the branch of a talk, the branch of a
//! reward, the scene on entering and the visibility block of a place with its end marker.
//!
//! A talk or reward script is a hull a figure points at; it ends with a [`Tail`], a `Jump` to
//! the figure's script so far or `End`. The visibility block is one script per place and
//! story, `sichtbarkeit_<story>_<header>`, closed by a marker line ([`visibility_end_marker`])
//! the next rule is inserted above, so no source is parsed ([`insert_rule`]).

#![allow(
    dead_code,
    reason = "the step recipes of 3d.7 (talk_advances, figure_appears, scene_on_enter, \
              reward_once) are the callers"
)]

use g4_formats::game::Family;

use super::story_common::{fill, template};
use super::types::refused;
use crate::error::KernelError;

/// How a hull script ends: it continues with the figure's script so far (`Jump <name>`) or
/// ends (`End`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tail {
    Jump(String),
    End,
}

impl Tail {
    /// The line (indented like the script's body, without the line break).
    fn line(&self) -> String {
        match self {
            Tail::Jump(name) => format!("    Jump {name}"),
            Tail::End => "    End".to_owned(),
        }
    }
}

/// The name of the talk hull of figure `local_id` in slot `#n`: `gespraech_<local_id>_<n>`,
/// unique in its script file by the slot (two stories may advance through the same figure
/// from the same value).
pub(crate) fn talk_name(local_id: u16, n: u16) -> String {
    format!("gespraech_{local_id}_{n}")
}

/// The talk branch [`talk_name`] `#n` of figure `local_id`: when `var_id` is `from`, it shows
/// message `message` and sets the variable to `from + 1`; otherwise the script goes on with
/// `tail`.
pub(crate) fn talk_script(
    family: Family,
    n: u16,
    local_id: u16,
    var_id: &str,
    from: u16,
    message: u16,
    tail: &Tail,
) -> String {
    fill(
        template("story_talk", family),
        &[
            ("name", talk_name(local_id, n)),
            ("n", n.to_string()),
            ("value", (u32::from(from) + 1).to_string()),
            ("var", var_id.to_owned()),
            ("from", from.to_string()),
            ("message", message.to_string()),
            ("tail", tail.line()),
        ],
    )
}

/// The name of the reward hull of figure `local_id` in slot `#n`: `belohnung_<local_id>_<n>`,
/// unique in its script file by the slot, like [`talk_name`].
pub(crate) fn reward_name(local_id: u16, n: u16) -> String {
    format!("belohnung_{local_id}_{n}")
}

/// The reward branch [`reward_name`] `#n` of figure `local_id`: from step `from` on
/// and while `flag_id` is clear it shows message `message`, gives `item` once and sets the
/// flag; otherwise the script goes on with `tail`.
#[expect(
    clippy::too_many_arguments,
    reason = "one argument per placeholder of the template"
)]
pub(crate) fn reward_script(
    family: Family,
    n: u16,
    local_id: u16,
    var_id: &str,
    from: u16,
    flag_id: &str,
    item: u16,
    message: u16,
    tail: &Tail,
) -> String {
    fill(
        template("story_reward", family),
        &[
            ("name", reward_name(local_id, n)),
            ("n", n.to_string()),
            ("var", var_id.to_owned()),
            ("from", from.to_string()),
            ("flag", flag_id.to_owned()),
            ("item", item.to_string()),
            ("message", message.to_string()),
            ("tail", tail.line()),
        ],
    )
}

/// The scene `szene_<n> #n`: it shows message `message` and sets `var_id` to `next`.
pub(crate) fn scene_script(
    family: Family,
    n: u16,
    var_id: &str,
    next: u16,
    message: u16,
) -> String {
    fill(
        template("story_scene", family),
        &[
            ("n", n.to_string()),
            ("var", var_id.to_owned()),
            ("value", next.to_string()),
            ("message", message.to_string()),
        ],
    )
}

/// One rule of a visibility block: from `step` on the figure with visibility flag `flag_id`
/// appears (the flag is cleared) or vanishes (the flag is set), before `step` the other way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisibilityRule {
    pub var_id: String,
    pub step: u16,
    pub flag_id: String,
    pub appears: bool,
}

/// The name of the block of story `story` at header `header`.
pub(crate) fn visibility_name(story: u16, header: u16) -> String {
    format!("sichtbarkeit_{story}_{header}")
}

/// The line that ends the block ([`insert_rule`] finds the block by it).
pub(crate) fn visibility_end_marker(story: u16, header: u16) -> String {
    format!("// ende {}", visibility_name(story, header))
}

/// The lines of `rule`, indented like the block's body (each ends with a line break).
pub(crate) fn visibility_rule(rule: &VisibilityRule) -> String {
    let name = if rule.appears {
        "story_visibility_rule_appear"
    } else {
        "story_visibility_rule_vanish"
    };
    fill(
        template(name, Family::Pt),
        &[
            ("var", rule.var_id.clone()),
            ("step", rule.step.to_string()),
            ("flag", rule.flag_id.clone()),
        ],
    )
}

/// The whole block as script `#n`: the head, the `rules`, the `tail` and the end marker.
pub(crate) fn visibility_block(
    story: u16,
    header: u16,
    n: u16,
    rules: &[VisibilityRule],
    tail: &Tail,
) -> String {
    let mut text = fill(
        template("story_visibility_head", Family::Pt),
        &[
            ("story", story.to_string()),
            ("header", header.to_string()),
            ("n", n.to_string()),
        ],
    );
    for rule in rules {
        text.push_str(&visibility_rule(rule));
    }
    text.push_str(&tail.line());
    text.push('\n');
    text.push_str(&visibility_end_marker(story, header));
    text.push('\n');
    text
}

/// `source` with `rule` (as [`visibility_rule`] gives it) inserted above the `Jump`/`End` line
/// that closes the block of story `story` at header `header`, the marker stays below it.
/// Refused when the block's marker is not in `source` (`file`, the script file, names the
/// place in the text), or no `Jump`/`End` line stands above it.
pub(crate) fn insert_rule(
    source: &str,
    story: u16,
    header: u16,
    rule: &str,
    file: u16,
) -> Result<String, KernelError> {
    let missing = || {
        refused(format!(
            "Der Sichtbarkeits-Block von Geschichte {story} fehlt in Scriptdatei {file}."
        ))
    };
    let marker = visibility_end_marker(story, header);
    let mut at = 0;
    let mut tail_at = None;
    let mut previous = None;
    for line in source.split_inclusive('\n') {
        if line.trim_end() == marker {
            tail_at = previous;
            break;
        }
        if !line.trim().is_empty() {
            previous = Some((at, line));
        }
        at += line.len();
    }
    let Some((at, line)) = tail_at else {
        return Err(missing());
    };
    let line = line.trim();
    if line != "End" && !line.starts_with("Jump ") {
        return Err(missing());
    }
    let mut out = String::with_capacity(source.len() + rule.len());
    out.push_str(&source[..at]);
    out.push_str(rule);
    out.push_str(&source[at..]);
    Ok(out)
}

#[cfg(test)]
mod tests;
