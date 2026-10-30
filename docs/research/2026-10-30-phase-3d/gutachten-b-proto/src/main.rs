//! Gutachten B (3d): Messungen am Referenzindex und ein Prototyp der Flag- und
//! Variablen-Verweise aus rotom-Quellen. Liest nur.
//!
//! `proto-3d-b index <projekt>`: `RefIndex::rebuild_full` (kalt, dann 4× warm) und die
//! Verweise je Quellart.
//! `proto-3d-b flagvar <projekt> <scrcmd.json>`: parst alle `expanded/scripts/*.rotom` mit
//! rotom wie der Kernel, findet Flag- und Variablenargumente nach Parametertyp (`flag`, `var`,
//! `flex`) und rotoms Namensregel, dazu nackte Flags und Vergleiche in `if`/`while`, und misst.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use g4_kernel::data::refs::{RefIndex, Target};
use g4_kernel::project::Project;
use g4_kernel::store::Store;
use rotom::compiler::ast::{Expression, ExpressionKind, Statement, StatementKind};
use rotom::compiler::token::TokenType;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("index") => index(&args[2]),
        Some("flagvar") => flagvar(&args[2], &args[3]),
        _ => eprintln!("proto-3d-b index <projekt> | flagvar <projekt> <scrcmd.json>"),
    }
}

fn index(root: &str) {
    let st = Store::new(Arc::new(Project::open(root).unwrap()));
    let mut idx = RefIndex::new();
    let t = Instant::now();
    idx.rebuild_full(&st).unwrap();
    println!("rebuild_full kalt {:?}", t.elapsed());
    let mut warm = Vec::new();
    for _ in 0..4 {
        let t = Instant::now();
        idx.rebuild_full(&st).unwrap();
        warm.push(t.elapsed().as_millis());
    }
    println!("rebuild_full warm ms {warm:?}");
    let all = idx.refs_where(&st, |_| true).unwrap();
    let mut by: HashMap<String, usize> = HashMap::new();
    for r in &all {
        *by.entry(format!("{:?}", r.source.kind)).or_default() += 1;
    }
    let mut by: Vec<_> = by.into_iter().collect();
    by.sort();
    println!("Verweise gesamt {} je Quelle {by:?}", all.len());
    let t = Instant::now();
    for n in 0..200u16 {
        let _ = idx.refs(&st, Target::Item(n)).unwrap();
    }
    println!("200 Abfragen refs(Item) {:?}", t.elapsed());
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Flag,
    Var,
    Flex,
    FlagVar,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Access {
    Read,
    Write,
    ReadWrite,
    Unknown,
}

const FLAG_NAMES: [&str; 3] = ["flag", "flagid", "flag_id"];
const VAR_NAMES: [&str; 22] = [
    "var", "variable", "var_0", "var_1", "var_2", "var_3", "var_4", "var_5", "var_6", "varid",
    "var_id", "destvarid", "dest_var_id", "variable_1", "variable_2", "retvar", "var_dest",
    "var_result", "var_or_addend", "var_or_trno", "countdown_variable", "var_flag",
];

fn classify(cmd: &str, p: &serde_json::Value) -> Option<(Kind, Access)> {
    let name = p["name"].as_str()?.to_ascii_lowercase();
    let ty = p["type"].as_str()?;
    if ["CheckFlagFromVar", "SetFlagFromVar", "SetFlagVar", "ClearFlagVar", "CheckFlagVar"]
        .contains(&cmd)
        && ["flagid", "varid", "flag", "var_flag"].contains(&name.as_str())
    {
        return Some((Kind::FlagVar, Access::Read));
    }
    if ty == "flag" || (FLAG_NAMES.contains(&name.as_str()) && ty == "u16") {
        let w = cmd == "SetFlag" || cmd == "ClearFlag";
        return Some((Kind::Flag, if w { Access::Write } else { Access::Read }));
    }
    if ty == "var" || VAR_NAMES.contains(&name.as_str()) {
        let a = match p.get("access").and_then(|a| a.as_str()) {
            Some("read") => Access::Read,
            Some("must_write" | "may_write") => Access::Write,
            Some("read_must_write" | "read_may_write") => Access::ReadWrite,
            _ => Access::Unknown,
        };
        return Some((Kind::Var, a));
    }
    (ty == "flex").then_some((Kind::Flex, Access::Read))
}

struct Ctx<'a> {
    commands: &'a HashMap<String, Vec<Option<(Kind, Access)>>>,
    names: &'a HashMap<String, i32>,
    flag_names: &'a HashSet<String>,
    aliases: HashMap<&'a str, &'a Expression>,
    out: Vec<(Kind, i32, Access, usize)>,
}

impl<'a> Ctx<'a> {
    fn value(&self, e: &Expression, depth: u8) -> Option<i32> {
        match &e.node {
            ExpressionKind::Number(n) => Some(*n),
            ExpressionKind::Identifier(n) => {
                if let Some(v) = self.aliases.get(n.as_str()) {
                    return (depth < 8).then(|| self.value(v, depth + 1)).flatten();
                }
                self.names.get(n).copied()
            }
            _ => None,
        }
    }

    fn command(&mut self, name: &str, args: &'a [Expression]) {
        let Some(kinds) = self.commands.get(name) else {
            return;
        };
        for (i, k) in kinds.iter().enumerate() {
            let (Some((kind, access)), Some(arg)) = (k, args.get(i)) else {
                continue;
            };
            let Some(v) = self.value(arg, 0) else { continue };
            match kind {
                Kind::Flag => self.out.push((Kind::Flag, v, *access, arg.span.start)),
                Kind::FlagVar | Kind::Var | Kind::Flex if v >= 0x4000 => {
                    self.out.push((Kind::Var, v, *access, arg.span.start))
                }
                _ => {}
            }
        }
    }

    /// Bedingung von `if`/`while`: nacktes Flag, Variable, Vergleich mit Variable.
    fn condition(&mut self, e: &'a Expression) {
        match &e.node {
            ExpressionKind::Identifier(n) if self.flag_names.contains(n) || n.starts_with("FLAG_") => {
                if let Some(v) = self.value(e, 0) {
                    self.out.push((Kind::Flag, v, Access::Read, e.span.start));
                }
            }
            ExpressionKind::Identifier(_) | ExpressionKind::Number(_) => {
                if let Some(v) = self.value(e, 0).filter(|v| *v >= 0x4000) {
                    self.out.push((Kind::Var, v, Access::Read, e.span.start));
                }
            }
            ExpressionKind::Prefix { id, .. } => self.condition(id),
            ExpressionKind::Infix { left, right, operator } => {
                let _ = matches!(operator, TokenType::Equal);
                self.condition(left);
                self.condition(right);
            }
            ExpressionKind::Call { function, args } => {
                if let ExpressionKind::Identifier(n) = &function.node {
                    self.command(n, args);
                }
            }
            _ => {}
        }
    }

    fn statements(&mut self, list: &'a [Statement]) {
        for s in list {
            match &s.node {
                StatementKind::Function { body, .. } | StatementKind::Action { body, .. } => {
                    self.statements(body)
                }
                StatementKind::AliasStatement { value, name } => {
                    self.aliases.insert(name.as_str(), value);
                }
                StatementKind::IfStatement { condition, body, elseblock } => {
                    self.condition(condition);
                    self.statements(body);
                    if let Some(e) = elseblock {
                        self.statements(e);
                    }
                }
                StatementKind::WhileStatement { condition, body } => {
                    self.condition(condition);
                    self.statements(body);
                }
                StatementKind::MatchStatement { subject, cases, default } => {
                    self.condition(subject);
                    for c in cases {
                        self.statements(&c.body);
                    }
                    if let Some(d) = default {
                        self.statements(d);
                    }
                }
                StatementKind::ScriptCommand { command, args } => self.command(command, args),
                _ => {}
            }
        }
    }
}

fn flagvar(root: &str, db_path: &str) {
    let db: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(db_path).unwrap()).unwrap();
    let mut commands = HashMap::new();
    for (name, c) in db["commands"].as_object().unwrap() {
        if c["type"] == "movement" {
            continue;
        }
        let kinds: Vec<_> = c["params"]
            .as_array()
            .map(|ps| ps.iter().map(|p| classify(name, p)).collect())
            .unwrap_or_default();
        if let Some(l) = c["legacy_name"].as_str() {
            commands.entry(l.to_string()).or_insert_with(|| kinds.clone());
        }
        commands.insert(name.clone(), kinds);
    }
    let mut names = HashMap::new();
    let mut flag_names = HashSet::new();
    for tab in ["flags", "vars"] {
        for (k, v) in db[tab].as_object().unwrap() {
            names.entry(k.clone()).or_insert(v["id"].as_i64().unwrap() as i32);
            if tab == "flags" {
                flag_names.insert(k.clone());
            }
        }
    }
    names.insert("VARS_START".into(), 0x4000);
    let dir = Path::new(root).join("expanded/scripts");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "rotom"))
        .collect();
    files.sort();
    let texts: Vec<String> = files.iter().map(|p| std::fs::read_to_string(p).unwrap()).collect();
    let mut parse_ms = Vec::new();
    let mut extract_ms = Vec::new();
    let mut total = Vec::new();
    for round in 0..4 {
        let t = Instant::now();
        let asts: Vec<_> = texts
            .iter()
            .map(|t| rotom::Parser::new_fallible(rotom::Lexer::new(t)).parse_script_file().ok())
            .collect();
        parse_ms.push(t.elapsed().as_millis());
        let t = Instant::now();
        let mut found = Vec::new();
        for ast in asts.iter().flatten() {
            let mut cx = Ctx {
                commands: &commands,
                names: &names,
                flag_names: &flag_names,
                aliases: HashMap::new(),
                out: Vec::new(),
            };
            cx.statements(&ast.aliases);
            cx.statements(&ast.items);
            found.extend(cx.out);
        }
        extract_ms.push(t.elapsed().as_millis());
        if round == 0 {
            total = found;
        }
    }
    println!("Quellen {} parse ms {parse_ms:?} (ein Faden) Erkennen ms {extract_ms:?} ", files.len());
    let flags: Vec<_> = total.iter().filter(|r| r.0 == Kind::Flag).collect();
    let vars: Vec<_> = total.iter().filter(|r| r.0 == Kind::Var).collect();
    let saved: Vec<_> = vars.iter().filter(|r| r.1 < 0x8000).collect();
    let d = |v: &[&&(Kind, i32, Access, usize)]| v.iter().map(|r| r.1).collect::<HashSet<_>>().len();
    let unk = vars.iter().filter(|r| r.2 == Access::Unknown).count();
    println!(
        "Flag-Verweise {} (verschiedene {}), Var-Verweise {} davon < 0x8000: {} (verschiedene {}), access unbekannt {}",
        flags.len(),
        flags.iter().map(|r| r.1).collect::<HashSet<_>>().len(),
        vars.len(),
        saved.len(),
        d(&saved),
        unk
    );
}
