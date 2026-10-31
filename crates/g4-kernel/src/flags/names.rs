//! The pret names of the flags and variables: the `flags` and `vars` tables of a family's
//! bundled command database, several names to one number, the first of them the main name.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use g4_formats::game::Family;

use super::FlagKind;
use crate::scripts::scrcmd_database;

/// The names of one kind in one family.
#[derive(Debug, Default)]
pub struct NameTable {
    /// Every name of a number, in the order of the database file; the main name is
    /// `all[main]`.
    all: BTreeMap<u16, Entry>,
    numbers: HashMap<String, u16>,
}

#[derive(Debug)]
struct Entry {
    names: Vec<String>,
    main: usize,
}

impl NameTable {
    /// The main name of `n`: the first name with the prefix that is neither a range
    /// constant (`_START`, `_END`, `_BASE`) nor the hex form (`VAR_0x800C`); else the first with
    /// the prefix; else the first. `VARS_START` and `VAR_MAP_LOCAL_0x00` are both `0x4000`, the
    /// main name is `VAR_MAP_LOCAL_0x00`.
    pub fn name(&self, n: u16) -> Option<&str> {
        self.all.get(&n).map(|e| e.names[e.main].as_str())
    }

    /// Every name of `n`, in the order of the file; empty when there is none.
    pub fn names(&self, n: u16) -> &[String] {
        self.all.get(&n).map_or(&[], |e| e.names.as_slice())
    }

    /// The number of `name`, any of the names of a number (aliases included).
    pub fn number(&self, name: &str) -> Option<u16> {
        self.numbers.get(name).copied()
    }

    /// The main names, ascending by number.
    pub fn iter(&self) -> impl Iterator<Item = (u16, &str)> {
        self.all.iter().map(|(&n, e)| (n, e.names[e.main].as_str()))
    }

    /// The number of distinct numbers that have a name.
    pub fn len(&self) -> usize {
        self.all.len()
    }

    /// Whether no number has a name.
    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }
}

/// The index of the main name among `names` (see [`NameTable::name`]).
fn main_name(names: &[String], prefix: &str) -> usize {
    let under_prefix = |n: &str| {
        n.strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('_'))
    };
    let plain = |n: &str| {
        under_prefix(n)
            && !["_START", "_END", "_BASE"].iter().any(|s| n.ends_with(s))
            && !n[prefix.len()..].starts_with("_0x")
    };
    names
        .iter()
        .position(|n| plain(n))
        .or_else(|| names.iter().position(|n| under_prefix(n)))
        .unwrap_or(0)
}

/// Reads the `section` (`flags` or `vars`) of a database: `{ "<NAME>": { "id": n } }`, in the
/// order of the file. An entry that does not fit is skipped; the tests count the result.
fn read(json: &str, kind: FlagKind) -> NameTable {
    let section = match kind {
        FlagKind::Flag => "flags",
        FlagKind::Var => "vars",
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json) else {
        return NameTable::default();
    };
    let Some(entries) = root.get(section).and_then(|v| v.as_object()) else {
        return NameTable::default();
    };
    let mut by_number: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    let mut numbers = HashMap::new();
    for (name, value) in entries {
        let Some(n) = value
            .get("id")
            .and_then(|v| v.as_u64())
            .and_then(|n| u16::try_from(n).ok())
        else {
            continue;
        };
        numbers.insert(name.clone(), n);
        by_number.entry(n).or_default().push(name.clone());
    }
    let all = by_number
        .into_iter()
        .map(|(n, names)| {
            let main = main_name(&names, kind.prefix());
            (n, Entry { names, main })
        })
        .collect();
    NameTable { all, numbers }
}

/// The pret names of `kind` in `family`'s command database (`flags`/`vars`); empty for Dp,
/// whose database carries no flag names and only a handful of variables.
pub fn names(family: Family, kind: FlagKind) -> &'static NameTable {
    static EMPTY: OnceLock<NameTable> = OnceLock::new();
    static TABLES: [[OnceLock<NameTable>; 2]; 2] = [
        [OnceLock::new(), OnceLock::new()],
        [OnceLock::new(), OnceLock::new()],
    ];
    let row = match family {
        Family::Dp => return EMPTY.get_or_init(NameTable::default),
        Family::Pt => 0,
        Family::Hgss => 1,
    };
    let col = match kind {
        FlagKind::Flag => 0,
        FlagKind::Var => 1,
    };
    // The database is embedded and counted by the tests, so a parse error cannot reach a user.
    TABLES[row][col].get_or_init(|| read(scrcmd_database(family).1, kind))
}

/// Whether a pret name says the number is unused: it is `None` or contains UNUSED, DUMMY or UNK.
pub fn name_is_unused(name: Option<&str>) -> bool {
    name.is_none_or(|n| ["UNUSED", "DUMMY", "UNK"].iter().any(|w| n.contains(w)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use g4_formats::game::Family;

    use crate::flags::FlagKind;

    #[test]
    fn platinum_and_heartgold_name_the_decided_counts() {
        assert_eq!(names(Family::Pt, FlagKind::Flag).len(), 4097);
        assert_eq!(names(Family::Hgss, FlagKind::Flag).len(), 2244);
        assert_eq!(names(Family::Pt, FlagKind::Var).len(), 307);
        assert_eq!(names(Family::Hgss, FlagKind::Var).len(), 386);
        assert!(names(Family::Dp, FlagKind::Flag).is_empty());
        assert!(names(Family::Dp, FlagKind::Var).is_empty());
    }

    #[test]
    fn the_main_name_prefers_a_prefixed_plain_name() {
        let pt_vars = names(Family::Pt, FlagKind::Var);
        assert_eq!(pt_vars.name(0x4000), Some("VAR_MAP_LOCAL_0x00"));
        assert_eq!(pt_vars.name(0x800C), Some("VAR_RESULT"));
        assert_eq!(
            names(Family::Pt, FlagKind::Flag).name(0x997),
            Some("FLAG_VILLA_FURNITURE_TABLE")
        );
        let hgss = names(Family::Hgss, FlagKind::Flag);
        assert_eq!(hgss.name(0x320), Some("FLAG_HIDDENITEM_T20_POTION"));
        assert_eq!(hgss.name(0x40), Some("FLAG_UNK_040"));
        // The only name, a range constant, is still the main name.
        assert_eq!(hgss.name(0x550), Some("TRAINER_FLAG_BASE"));
        assert_eq!(hgss.names(0x550), ["TRAINER_FLAG_BASE"]);
        assert_eq!(hgss.name(0xFFFF), None);
        assert!(hgss.names(0xFFFF).is_empty());
    }

    #[test]
    fn aliases_resolve_to_their_number() {
        let pt = names(Family::Pt, FlagKind::Var);
        assert_eq!(pt.number("VARS_START"), Some(0x4000));
        assert_eq!(pt.number("VAR_MAP_LOCAL_0x00"), Some(0x4000));
        assert_eq!(pt.number("VAR_NO_SUCH"), None);
        assert_eq!(
            pt.names(0x4000),
            ["MAP_LOCAL_VARS_START", "VARS_START", "VAR_MAP_LOCAL_0x00"]
        );
        let hgss = names(Family::Hgss, FlagKind::Flag);
        assert_eq!(hgss.number("HIDDEN_ITEMS_FLAG_BASE"), Some(0x320));
        // The iteration lists the main names, ascending.
        let all: Vec<(u16, &str)> = hgss.iter().collect();
        assert_eq!(all.len(), hgss.len());
        assert!(all.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(all.contains(&(0x320, "FLAG_HIDDENITEM_T20_POTION")));
    }

    #[test]
    fn unused_names_are_unused_dummy_unk_or_none() {
        assert!(name_is_unused(None));
        assert!(name_is_unused(Some("FLAG_UNUSED_0x0096")));
        assert!(name_is_unused(Some("FLAG_UNK_040")));
        assert!(name_is_unused(Some("VAR_DUMMY_4000")));
        assert!(!name_is_unused(Some("FLAG_VILLA_FURNITURE_TABLE")));
        assert!(!name_is_unused(Some("VAR_RESULT")));
    }
}
