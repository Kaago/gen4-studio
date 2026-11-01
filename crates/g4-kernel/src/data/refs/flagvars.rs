//! Flags and variables in script sources: which command parameters name a flag, a variable or
//! the "defeated" flag of a trainer, and what a command does with it, declared per command
//! database family in `metadata/script_flagvars.yaml`; plus the conditions of `if`, `while` and
//! `match`, which read what they test.
//!
//! Which value counts (Gutachten B §3.2):
//! - a variable parameter: `0x4000..0x8000` is a [`Target::Var`]; from `0x8000` on it is local to
//!   the script and only counted ([`ScriptCounts::script_local`]); below `0x4000` it is a value;
//! - a flag parameter: `1..0x4000` is a [`Target::Flag`], `0` no flag; from `0x4000` on the flag
//!   is the value of a variable and only counted ([`ScriptCounts::dynamic`]), except when the
//!   command takes the flag as a plain number (a parameter of type `u16` or `flag`, as
//!   `SetFlag`) and the argument is a name that resolves as a flag of the family: a pret flag
//!   name (HeartGold's temporary `FLAG_TEMP_x4000`…`x403F`, Platinum's
//!   `FLAG_GALACTIC_HQ_CONTROL_ROOM_STATE` = `0x410D`), an own flag name or an alias of the file
//!   for one of these; that one is a [`Target::Flag`] too. A plain number or a variable name from
//!   `0x4000` on stays dynamic, and so does any argument of a parameter of type `var` or `flex`
//!   (`SetFlagVar`, `SetFlagFromVar`): the engine reads such a parameter as a variable, whatever
//!   name a decompiled source gives it;
//! - a trainer flag parameter: trainer `n` is the flag [`trainer_flag`] gives (`0x550 + n`),
//!   from `0x4000` on counted like a flag;
//! - a condition: a bare identifier whose name starts with `FLAG_` and resolves is a flag read
//!   (from `0x4000` on by the rule of `SetFlag`, as rotom lowers it to `CheckFlag`);
//!   an operand of a comparison (or a bare operand, as rotom lowers it to `CompareVarValue x, 0`;
//!   the head of `match`) that resolves from `0x4000` on is a variable read. A call in a
//!   condition (`CheckItem(ITEM_POTION, 1) == 1`) is a command like any other; its implicit
//!   result variable is no reference.
//!
//! The value of a variable reference ([`Ref::value`]): the constant a command sets the variable
//! to or compares it with (the parameter the family's `values` names for the command, as
//! `SetVar`'s `valueOrVarID`), or the other operand of a comparison in a condition; only a
//! number below `0x4000` (a variable from there on, as `SetVar VAR_X, VAR_Y`), none where it
//! does not resolve, for the head of `match`, a bare operand and every flag.
//!
//! `field` of a reference is rotom's command name (`if`, `while`, `match` for conditions),
//! `slot` the line of the argument or operand.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use g4_formats::game::Family;
use rotom::compiler::ast::{Expression, ExpressionKind, ScriptFile};
use rotom::compiler::token::TokenType;
use serde::Deserialize;

use super::{Access, FIRST_VARIABLE, Ref, ScriptCounts, ScriptNames, Target, constant, script_id};
use crate::flags::trainer_flag;
use crate::scripts::{self, AstItem};

/// What a parameter names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamRole {
    Flag,
    Var,
    /// A trainer number: the flag is the trainer's "defeated" flag.
    TrainerFlag,
}

/// A parameter of a command that names a flag, a variable or a trainer flag: its name, its
/// argument position in the command database, what it names, what the command does with it and
/// whether the engine takes it as a plain number (`direct`: its type is neither `var` nor
/// `flex`, which the engine reads as a variable from `0x4000` on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlagVarParam {
    pub name: String,
    pub arg: usize,
    pub role: ParamRole,
    pub access: Access,
    pub direct: bool,
    /// The argument that carries the value of this variable parameter (`values` of the family),
    /// when there is one.
    pub value_arg: Option<usize>,
}

/// The commands of one family whose parameters name a flag, a variable or a trainer flag, from
/// `metadata/script_flagvars.yaml` and the database (type, rotom's name rule, `access`),
/// positioned by the bundled database; `params(name)` resolves legacy names and aliases like
/// `ScriptRefTable::params`.
#[derive(Debug, Default)]
pub(crate) struct FlagVarTable {
    family: Option<Family>,
    commands: BTreeMap<String, Vec<FlagVarParam>>,
}

impl FlagVarTable {
    /// rotom's name of command `name` (itself, or the command a legacy name or an alias stands
    /// for) and its parameters that name a flag or a variable; `None` for other commands.
    pub(crate) fn params(&self, name: &str) -> Option<(&str, &[FlagVarParam])> {
        if let Some((key, params)) = self.commands.get_key_value(name) {
            return Some((key, params));
        }
        let db = crate::checks::scripts::database(self.family?)?;
        // A canonical name of another command: nothing to look for (most commands of a source).
        if db.commands.contains_key(name) {
            return None;
        }
        // rotom v0.7.0: src/database.rs DatabaseV2::get_command (canonical, legacy, alias)
        let command = db.get_command(name).ok()?;
        self.commands.iter().find_map(|(key, params)| {
            db.commands
                .get(key)
                .is_some_and(|c| std::ptr::eq(c, command))
                .then_some((key.as_str(), params.as_slice()))
        })
    }

    /// The parameter `param` of command `command` (rotom's name) in the table.
    #[cfg(test)]
    fn param(&self, command: &str, param: &str) -> Option<&FlagVarParam> {
        self.commands.get(command)?.iter().find(|p| p.name == param)
    }
}

/// How a variable parameter without an entry gets its access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VarDefault {
    /// The parameter's `access` in the database, through `access_words`.
    Database,
    /// The access of the Platinum command of the same name, parameter at the same position.
    Borrow,
}

/// Command → parameter → access.
type Entries = BTreeMap<String, BTreeMap<String, Access>>;

/// The rules of one family in `metadata/script_flagvars.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFamily {
    flag_default: Access,
    var_default: VarDefault,
    #[serde(default)]
    flags: Entries,
    #[serde(default)]
    vars: Entries,
    #[serde(default)]
    trainer_flags: Entries,
    /// Command → the parameter that carries the value its variable parameter is set to or
    /// compared with.
    #[serde(default)]
    values: BTreeMap<String, String>,
}

/// rotom's parameter names of a flag and of a variable (lower case).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NameRule {
    flag: Vec<String>,
    var: Vec<String>,
}

/// `metadata/script_flagvars.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFlagVars {
    name_rule: NameRule,
    access_words: BTreeMap<String, Access>,
    pt: RawFamily,
    hgss: RawFamily,
}

impl RawFlagVars {
    fn family(&self, family: Family) -> Option<&RawFamily> {
        match family {
            Family::Pt => Some(&self.pt),
            Family::Hgss => Some(&self.hgss),
            Family::Dp => None,
        }
    }
}

/// The parsed `metadata/script_flagvars.yaml` (embedded and parsed by the tests, so a parse
/// error cannot reach a user: then there are no rules).
fn raw_flagvars() -> Option<RawFlagVars> {
    serde_yaml::from_str(include_str!("../../../metadata/script_flagvars.yaml")).ok()
}

/// A parameter as the bundled database's JSON has it: rotom's own `DatabaseV2` reads neither
/// `access` nor the type `flex`.
#[derive(Debug, Deserialize)]
struct RawParam {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    access: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCommand {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    params: Vec<RawParam>,
}

#[derive(Debug, Deserialize)]
struct RawDatabase {
    commands: HashMap<String, RawCommand>,
}

/// The bundled database of `family` read for types and `access` (`None` when it does not parse:
/// a test parses each).
fn raw_database(family: Family) -> Option<RawDatabase> {
    serde_json::from_str(crate::scripts::scrcmd_database(family).1).ok()
}

/// The [`FlagVarTable`] of `family`, made once per process.
pub(crate) fn flagvar_table(family: Family) -> &'static FlagVarTable {
    static TABLES: [OnceLock<FlagVarTable>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match family {
        Family::Dp => 0,
        Family::Pt => 1,
        Family::Hgss => 2,
    };
    TABLES[slot].get_or_init(|| make_table(family).unwrap_or_default())
}

/// The table of `family` from the rules and the database; `None` without either.
fn make_table(family: Family) -> Option<FlagVarTable> {
    let raw = raw_flagvars()?;
    let rules = raw.family(family)?;
    let db = raw_database(family)?;
    let mut commands = BTreeMap::new();
    for (name, command) in db.commands {
        // Movements share the map but take no flags or variables.
        if command.kind == "movement" {
            continue;
        }
        let value_at = rules
            .values
            .get(&name)
            .and_then(|value| command.params.iter().position(|p| p.name == *value));
        let params: Vec<FlagVarParam> = command
            .params
            .iter()
            .enumerate()
            .filter_map(|(arg, param)| {
                let (role, access) = classify(&raw, rules, family, &name, arg, param)?;
                Some(FlagVarParam {
                    name: param.name.clone(),
                    arg,
                    role,
                    access,
                    direct: !matches!(param.ty.as_str(), "var" | "flex"),
                    value_arg: value_at.filter(|&at| role == ParamRole::Var && at != arg),
                })
            })
            .collect();
        if !params.is_empty() {
            commands.insert(name, params);
        }
    }
    Some(FlagVarTable {
        family: Some(family),
        commands,
    })
}

/// Parameter types that hold a 16-bit word: only these can name a flag or a variable by name.
const WORD_TYPES: [&str; 4] = ["u16", "var", "flex", "flag"];

/// What parameter `param` (position `arg`) of command `command` names and what the command
/// does with it, by the rules of `family`: the entries first, then type and name rule.
fn classify(
    raw: &RawFlagVars,
    rules: &RawFamily,
    family: Family,
    command: &str,
    arg: usize,
    param: &RawParam,
) -> Option<(ParamRole, Access)> {
    let entry = |entries: &Entries| entries.get(command)?.get(&param.name).copied();
    if let Some(access) = entry(&rules.trainer_flags) {
        return Some((ParamRole::TrainerFlag, access));
    }
    if let Some(access) = entry(&rules.flags) {
        return Some((ParamRole::Flag, access));
    }
    if let Some(access) = entry(&rules.vars) {
        return Some((ParamRole::Var, access));
    }
    let lower = param.name.to_ascii_lowercase();
    let word = WORD_TYPES.contains(&param.ty.as_str());
    if param.ty == "flag" || (word && raw.name_rule.flag.contains(&lower)) {
        return Some((ParamRole::Flag, rules.flag_default));
    }
    if matches!(param.ty.as_str(), "var" | "flex") || (word && raw.name_rule.var.contains(&lower)) {
        let access = match rules.var_default {
            VarDefault::Database => param
                .access
                .as_ref()
                .and_then(|word| raw.access_words.get(word).copied())
                .unwrap_or(Access::Unknown),
            // Platinum cannot borrow from itself (its table is being made).
            VarDefault::Borrow if family == Family::Pt => Access::Unknown,
            VarDefault::Borrow => flagvar_table(Family::Pt)
                .commands
                .get(command)
                .and_then(|params| params.iter().find(|p| p.arg == arg))
                .filter(|p| p.role == ParamRole::Var)
                .map_or(Access::Unknown, |p| p.access),
        };
        return Some((ParamRole::Var, access));
    }
    None
}

/// What a value of a parameter or an operand stands for.
enum Seen {
    Target(Target),
    /// A variable from `0x8000` on.
    ScriptLocal,
    /// A flag named through a variable.
    Dynamic,
    Nothing,
}

/// A flag argument: `1..0x4000` a flag, from `0x4000` on the value of a variable.
fn flag_value(value: i32) -> Seen {
    match u16::try_from(value) {
        Ok(0) | Err(_) => Seen::Nothing,
        Ok(v) if i32::from(v) < FIRST_VARIABLE => Seen::Target(Target::Flag(v)),
        Ok(_) => Seen::Dynamic,
    }
}

/// A flag argument the engine takes as a plain number, `named` when it is a name that resolves
/// as a flag: like [`flag_value`], but a named flag from `0x4000` on is a flag as well.
fn direct_flag_value(value: i32, named: bool) -> Seen {
    match (flag_value(value), u16::try_from(value)) {
        (Seen::Dynamic, Ok(v)) if named => Seen::Target(Target::Flag(v)),
        (seen, _) => seen,
    }
}

/// A variable argument: `0x4000..0x8000` a variable, from `0x8000` on local to the script,
/// below `0x4000` a value.
fn var_value(value: i32) -> Seen {
    match u16::try_from(value) {
        Ok(v) if i32::from(v) < FIRST_VARIABLE => Seen::Nothing,
        Ok(v) if v < SCRIPT_LOCAL => Seen::Target(Target::Var(v)),
        Ok(_) => Seen::ScriptLocal,
        Err(_) => Seen::Nothing,
    }
}

/// A trainer argument of a trainer flag: trainer `n` is its "defeated" flag (none for 0 or a
/// number past the range), from `0x4000` on the value of a variable.
fn trainer_value(family: Family, value: i32) -> Seen {
    match flag_value(value) {
        Seen::Target(Target::Flag(n)) => {
            trainer_flag(family, n).map_or(Seen::Nothing, |flag| Seen::Target(Target::Flag(flag)))
        }
        other => other,
    }
}

/// Variables from here on are local to a script.
const SCRIPT_LOCAL: u16 = 0x8000;

/// Collects the flag and variable references of one source.
struct FlagVarRefs<'s, 'n> {
    file: u16,
    family: Family,
    text: &'s str,
    names: &'n ScriptNames<'n>,
    refs: Vec<Ref>,
    counts: ScriptCounts,
}

impl FlagVarRefs<'_, '_> {
    /// Notes what `seen` (at byte `at`) stands for: a reference of `field` with `access` (and
    /// `value`, for a variable), or a count.
    fn note(
        &mut self,
        field: &'static str,
        at: usize,
        seen: Seen,
        access: Access,
        value: Option<u16>,
    ) {
        match seen {
            Seen::Target(target) => {
                let (line, _) = scripts::position_of(self.text, at);
                self.refs.push(Ref {
                    source: script_id(self.file),
                    field,
                    // Lines are far below 65536.
                    slot: Some(u16::try_from(line).unwrap_or(u16::MAX)),
                    target,
                    access,
                    value: value.filter(|_| matches!(target, Target::Var(_))),
                });
            }
            Seen::ScriptLocal => self.counts.script_local += 1,
            Seen::Dynamic => self.counts.dynamic += 1,
            Seen::Nothing => {}
        }
    }

    /// The number `expression` stands for, as rotom's lowering resolves it.
    fn value(&self, expression: &Expression) -> Option<i32> {
        self.names.value(expression, expression.span.start, 0)
    }

    /// Whether `expression` is a name that resolves as a flag of the family.
    fn names_a_flag(&self, expression: &Expression) -> bool {
        self.names
            .names_a_flag(expression, expression.span.start, 0, self.family)
    }

    /// The constant `expression` stands for as the value of a variable: a number below
    /// `0x4000` it resolves to; `None` for a variable, a call or what does not resolve.
    fn constant(&self, expression: &Expression) -> Option<u16> {
        constant(self.value(expression)?)
    }

    fn command(&mut self, command: &'static str, params: &[FlagVarParam], args: &[Expression]) {
        for param in params {
            let Some(arg) = args.get(param.arg) else {
                continue;
            };
            let Some(value) = self.value(arg) else {
                continue;
            };
            let seen = match param.role {
                ParamRole::Flag if param.direct => direct_flag_value(value, self.names_a_flag(arg)),
                ParamRole::Flag => flag_value(value),
                ParamRole::Var => var_value(value),
                ParamRole::TrainerFlag => trainer_value(self.family, value),
            };
            let constant = param
                .value_arg
                .and_then(|at| args.get(at))
                .and_then(|arg| self.constant(arg));
            self.note(command, arg.span.start, seen, param.access, constant);
        }
    }

    /// The condition of `if` or `while`: `!`, `&&` and `||` combine conditions, a comparison
    /// reads its operands (each with the other as its value), a bare `FLAG_…` is a flag.
    fn condition(&mut self, keyword: &'static str, expression: &Expression) {
        match &expression.node {
            ExpressionKind::Prefix {
                operator: TokenType::Not,
                id,
            } => self.condition(keyword, id),
            ExpressionKind::Infix {
                left,
                operator: TokenType::And | TokenType::Or,
                right,
            } => {
                self.condition(keyword, left);
                self.condition(keyword, right);
            }
            ExpressionKind::Infix {
                left,
                operator,
                right,
            } => {
                // Only a comparison gives each operand the other as its value.
                let compares = matches!(
                    operator,
                    TokenType::Equal
                        | TokenType::NotEqual
                        | TokenType::LesserThan
                        | TokenType::LesserEqual
                        | TokenType::GreaterThan
                        | TokenType::GreaterEqual
                );
                let value_of =
                    |other: &Expression| compares.then(|| self.constant(other)).flatten();
                let (left_value, right_value) = (value_of(right), value_of(left));
                self.operand(keyword, left, left_value);
                self.operand(keyword, right, right_value);
            }
            // rotom v0.7.0: src/compiler/ir/lowerer.rs flag_id_of (a bare flag constant).
            ExpressionKind::Identifier(name) if name.starts_with("FLAG_") => {
                if let Some(value) = self.value(expression) {
                    let seen = direct_flag_value(value, self.names_a_flag(expression));
                    self.note(keyword, expression.span.start, seen, Access::Read, None);
                }
            }
            _ => self.operand(keyword, expression, None),
        }
    }

    /// An operand of a comparison (or the head of `match`): from `0x4000` on a variable it
    /// reads, compared with `value`. A call is a command; its result variable is no reference.
    fn operand(&mut self, keyword: &'static str, expression: &Expression, value: Option<u16>) {
        if matches!(expression.node, ExpressionKind::Call { .. }) {
            return;
        }
        if let Some(n) = self.value(expression) {
            let seen = var_value(n);
            self.note(keyword, expression.span.start, seen, Access::Read, value);
        }
    }
}

/// The flag and variable references of one parsed source (commands and conditions) and its counts.
pub(crate) fn script_flagvar_refs(
    file: u16,
    text: &str,
    ast: &ScriptFile,
    names: &ScriptNames<'_>,
    table: &'static FlagVarTable,
    family: Family,
) -> (Vec<Ref>, ScriptCounts) {
    let mut out = FlagVarRefs {
        file,
        family,
        text,
        names,
        refs: Vec::new(),
        counts: ScriptCounts::default(),
    };
    scripts::visit(ast, &mut |item| match item {
        AstItem::Command { name, args } => {
            if let Some((command, params)) = table.params(name) {
                out.command(command, params, args);
            }
        }
        AstItem::Condition {
            keyword: "match",
            expression,
        } => out.operand("match", expression, None),
        AstItem::Condition {
            keyword,
            expression,
        } => out.condition(keyword, expression),
        AstItem::Alias { .. } | AstItem::Declared(_) | AstItem::Expression(_) => {}
    });
    (out.refs, out.counts)
}

#[cfg(test)]
mod tests;
