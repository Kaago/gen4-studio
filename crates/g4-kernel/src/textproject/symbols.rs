//! Symbolic ids: the fixed English tables of species (with forms), moves, items, abilities and
//! types from `metadata/symbols/`, the fallback `PREFIX_NNNN` for numbers without an entry,
//! and the name normalization that christens maps, trainers, trainer classes and places.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const SPECIES_YAML: &str = include_str!("../../metadata/symbols/species.yaml");
const FORMS_YAML: &str = include_str!("../../metadata/symbols/forms.yaml");
const MOVES_YAML: &str = include_str!("../../metadata/symbols/moves.yaml");
const ITEMS_YAML: &str = include_str!("../../metadata/symbols/items.yaml");
const ABILITIES_YAML: &str = include_str!("../../metadata/symbols/abilities.yaml");
const TYPES_YAML: &str = include_str!("../../metadata/symbols/types.yaml");

/// What an id names; its prefix starts every id of the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum SymbolKind {
    Species,
    Move,
    Item,
    Ability,
    Type,
    Map,
    Trainer,
    TrainerClass,
    Place,
}

impl SymbolKind {
    /// `SPECIES`, `MOVE`, `ITEM`, `ABILITY`, `TYPE`, `MAP`, `TRAINER`, `TRAINER_CLASS`, `PLACE`.
    pub fn prefix(self) -> &'static str {
        match self {
            SymbolKind::Species => "SPECIES",
            SymbolKind::Move => "MOVE",
            SymbolKind::Item => "ITEM",
            SymbolKind::Ability => "ABILITY",
            SymbolKind::Type => "TYPE",
            SymbolKind::Map => "MAP",
            SymbolKind::Trainer => "TRAINER",
            SymbolKind::TrainerClass => "TRAINER_CLASS",
            SymbolKind::Place => "PLACE",
        }
    }
}

/// One fixed table: number → id and id → number.
#[derive(Debug, Default)]
pub struct SymbolTable {
    ids: BTreeMap<u32, String>,
    numbers: HashMap<String, u32>,
}

impl SymbolTable {
    /// The id of `n`, if the table names it.
    pub fn id(&self, n: u32) -> Option<&str> {
        self.ids.get(&n).map(String::as_str)
    }

    /// The number of `id`, if the table has it.
    pub fn number(&self, id: &str) -> Option<u32> {
        self.numbers.get(id).copied()
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the table has no entry.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

/// The fixed table of `kind`: `Some` for species (forms included), moves, items, abilities and
/// types; the other kinds are christened per project and live in its registers.
pub fn table(kind: SymbolKind) -> Option<&'static SymbolTable> {
    static SPECIES: OnceLock<SymbolTable> = OnceLock::new();
    static MOVES: OnceLock<SymbolTable> = OnceLock::new();
    static ITEMS: OnceLock<SymbolTable> = OnceLock::new();
    static ABILITIES: OnceLock<SymbolTable> = OnceLock::new();
    static TYPES: OnceLock<SymbolTable> = OnceLock::new();
    let cell = match kind {
        SymbolKind::Species => &SPECIES,
        SymbolKind::Move => &MOVES,
        SymbolKind::Item => &ITEMS,
        SymbolKind::Ability => &ABILITIES,
        SymbolKind::Type => &TYPES,
        SymbolKind::Map | SymbolKind::Trainer | SymbolKind::TrainerClass | SymbolKind::Place => {
            return None;
        }
    };
    // The files are embedded and parsed by the tests, so a parse error cannot reach a user.
    Some(cell.get_or_init(|| load(kind).unwrap_or_default()))
}

/// The embedded files of a fixed table, by name; the forms (496–507) join the species.
fn files(kind: SymbolKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        SymbolKind::Species => &[("species.yaml", SPECIES_YAML), ("forms.yaml", FORMS_YAML)],
        SymbolKind::Move => &[("moves.yaml", MOVES_YAML)],
        SymbolKind::Item => &[("items.yaml", ITEMS_YAML)],
        SymbolKind::Ability => &[("abilities.yaml", ABILITIES_YAML)],
        SymbolKind::Type => &[("types.yaml", TYPES_YAML)],
        SymbolKind::Map | SymbolKind::Trainer | SymbolKind::TrainerClass | SymbolKind::Place => &[],
    }
}

/// One file of `metadata/symbols/`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TableFile {
    prefix: String,
    ids: BTreeMap<u32, String>,
}

/// Parses and checks the files of `kind`: the prefix of the kind, every id under it, no
/// number and no id twice, no id in the fallback form.
fn load(kind: SymbolKind) -> Result<SymbolTable, String> {
    let mut table = SymbolTable::default();
    for (name, yaml) in files(kind) {
        let file: TableFile = serde_yaml::from_str(yaml).map_err(|e| format!("{name}: {e}"))?;
        if file.prefix != kind.prefix() {
            return Err(format!("{name}: prefix {} for {kind:?}", file.prefix));
        }
        for (n, id) in file.ids {
            let under_prefix = id
                .strip_prefix(kind.prefix())
                .is_some_and(|rest| rest.len() > 1 && rest.starts_with('_'));
            if !under_prefix || parse_fallback(kind, &id).is_some() {
                return Err(format!("{name}: {n} has the id {id}"));
            }
            if let Some(other) = table.numbers.insert(id.clone(), n) {
                return Err(format!("{name}: {id} for {other} and {n}"));
            }
            if table.ids.insert(n, id).is_some() {
                return Err(format!("{name}: {n} twice"));
            }
        }
    }
    Ok(table)
}

/// The id of a number without an entry: the prefix and at least four digits (`ITEM_0113`).
pub fn fallback_id(kind: SymbolKind, n: u32) -> String {
    format!("{}_{n:04}", kind.prefix())
}

/// The number of a fallback id of `kind` (`ITEM_0113` → 113, `ITEM_12345` → 12345); `None` for
/// anything else, also for the fallback of another kind.
pub fn parse_fallback(kind: SymbolKind, id: &str) -> Option<u32> {
    let digits = id.strip_prefix(kind.prefix())?.strip_prefix('_')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The id body of a game name (without prefix): markup removed, `♂`/`♀` as `_M`/`_F`, German
/// umlauts as `AE`/`OE`/`UE` when `german` (else plain vowels), `ß` as `SS`, other diacritics
/// dropped (own table, no Unicode crate), `&` as `AND`, apostrophes and dots dropped, every
/// run of other characters as `_`, upper case. `None` when nothing or only digits remain.
pub fn normalize(name: &str, german: bool) -> Option<String> {
    let mut mapped = String::with_capacity(name.len());
    for c in strip_markup(name).chars() {
        match c {
            '♂' => mapped.push_str("_M"),
            '♀' => mapped.push_str("_F"),
            '\'' | '’' | '.' => {}
            '&' => mapped.push_str("_AND_"),
            'Ä' | 'ä' if german => mapped.push_str("AE"),
            'Ö' | 'ö' if german => mapped.push_str("OE"),
            'Ü' | 'ü' if german => mapped.push_str("UE"),
            c if c.is_ascii_alphanumeric() => mapped.push(c.to_ascii_uppercase()),
            c => mapped.push_str(fold(c).unwrap_or("_")),
        }
    }
    let id = mapped
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(id)
}

/// The game's markup out of a name: `{TRAINER_NAME:Kai}` keeps the name, other `{…}` go,
/// `[PK][MN]` become `PKMN`, the escapes `\n`, `\r`, `\f` a space.
fn strip_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let inner = &rest[open + 1..];
        let Some(close) = inner.find('}') else {
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        if let Some(name) = inner[..close].strip_prefix("TRAINER_NAME:") {
            out.push_str(name);
        }
        rest = &inner[close + 1..];
    }
    out.push_str(rest);
    out.replace("[PK]", "PK")
        .replace("[MN]", "MN")
        .replace("\\n", " ")
        .replace("\\r", " ")
        .replace("\\f", " ")
}

/// A letter with a diacritic (or a ligature) as plain upper-case ASCII; our own table for the
/// Latin letters of the game languages, instead of a Unicode dependency.
fn fold(c: char) -> Option<&'static str> {
    Some(match c {
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å'
        | 'ā' | 'ă' | 'ą' => "A",
        'Æ' | 'æ' => "AE",
        'Ç' | 'Ć' | 'Č' | 'ç' | 'ć' | 'č' => "C",
        'Ď' | 'Đ' | 'Ð' | 'ď' | 'đ' | 'ð' => "D",
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ė' | 'Ę' | 'Ě' | 'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę'
        | 'ě' => "E",
        'Ğ' | 'ğ' => "G",
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' | 'Į' | 'İ' | 'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => {
            "I"
        }
        'Ł' | 'ł' => "L",
        'Ñ' | 'Ń' | 'Ň' | 'ñ' | 'ń' | 'ň' => "N",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' | 'Ő' | 'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō'
        | 'ő' => "O",
        'Œ' | 'œ' => "OE",
        'Ř' | 'ř' => "R",
        'Ś' | 'Š' | 'Ş' | 'ś' | 'š' | 'ş' => "S",
        'ß' | 'ẞ' => "SS",
        'Ť' | 'Ţ' | 'ť' | 'ţ' => "T",
        'Þ' | 'þ' => "TH",
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' | 'Ů' | 'Ű' | 'Ų' | 'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű'
        | 'ų' => "U",
        'Ý' | 'Ÿ' | 'ý' | 'ÿ' => "Y",
        'Ź' | 'Ż' | 'Ž' | 'ź' | 'ż' | 'ž' => "Z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXED: [SymbolKind; 5] = [
        SymbolKind::Species,
        SymbolKind::Move,
        SymbolKind::Item,
        SymbolKind::Ability,
        SymbolKind::Type,
    ];

    #[test]
    fn embedded_tables_load_without_error() {
        for kind in FIXED {
            assert!(load(kind).is_ok(), "{kind:?}: {:?}", load(kind).err());
        }
    }

    #[test]
    fn tables_cover_the_gen4_ranges() {
        for (kind, last) in [
            (SymbolKind::Species, 507),
            (SymbolKind::Move, 467),
            (SymbolKind::Ability, 123),
            (SymbolKind::Type, 17),
        ] {
            let t = table(kind).unwrap_or_else(|| panic!("{kind:?} has a table"));
            for n in 0..=last {
                assert!(t.id(n).is_some(), "{kind:?} {n} missing");
            }
            assert_eq!(
                t.len(),
                last as usize + 1,
                "{kind:?} has nothing past {last}"
            );
        }
        assert!(table(SymbolKind::Item).unwrap().len() >= 400);
        for kind in [
            SymbolKind::Map,
            SymbolKind::Trainer,
            SymbolKind::TrainerClass,
            SymbolKind::Place,
        ] {
            assert!(table(kind).is_none(), "{kind:?} is christened per project");
        }
    }

    #[test]
    fn ids_are_unique_and_never_look_like_a_fallback() {
        for kind in FIXED {
            let t = table(kind).unwrap();
            let mut seen = std::collections::HashSet::new();
            for n in 0..=600 {
                let Some(id) = t.id(n) else { continue };
                assert!(seen.insert(id.to_string()), "{id} twice");
                assert!(id.starts_with(&format!("{}_", kind.prefix())), "{id}");
                assert_eq!(parse_fallback(kind, id), None, "{id}");
                assert_eq!(t.number(id), Some(n), "{id}");
            }
            assert_eq!(seen.len(), t.len());
        }
    }

    #[test]
    fn well_known_ids() {
        let species = table(SymbolKind::Species).unwrap();
        assert_eq!(species.number("SPECIES_PIKACHU"), Some(25));
        assert_eq!(species.number("SPECIES_ROTOM_MOW"), Some(507));
        assert_eq!(species.id(0), Some("SPECIES_NONE"));
        assert_eq!(species.id(494), Some("SPECIES_EGG"));
        assert_eq!(species.id(495), Some("SPECIES_BAD_EGG"));
        assert_eq!(species.id(496), Some("SPECIES_DEOXYS_ATTACK"));
        assert_eq!(
            table(SymbolKind::Move)
                .unwrap()
                .number("MOVE_THUNDER_SHOCK"),
            Some(84)
        );
        assert_eq!(
            table(SymbolKind::Item).unwrap().number("ITEM_POTION"),
            Some(17)
        );
        assert_eq!(
            table(SymbolKind::Type).unwrap().number("TYPE_FIRE"),
            Some(10)
        );
        assert_eq!(
            table(SymbolKind::Ability)
                .unwrap()
                .number("ABILITY_LEVITATE"),
            Some(26)
        );
        assert_eq!(table(SymbolKind::Item).unwrap().number("ITEM_0017"), None);
    }

    #[test]
    fn fallback_ids_round_trip() {
        assert_eq!(fallback_id(SymbolKind::Item, 113), "ITEM_0113");
        assert_eq!(
            fallback_id(SymbolKind::TrainerClass, 7),
            "TRAINER_CLASS_0007"
        );
        assert_eq!(fallback_id(SymbolKind::Map, 12345), "MAP_12345");
        assert_eq!(parse_fallback(SymbolKind::Item, "ITEM_0113"), Some(113));
        assert_eq!(parse_fallback(SymbolKind::Item, "ITEM_12345"), Some(12345));
        assert_eq!(parse_fallback(SymbolKind::Item, "MOVE_0001"), None);
        assert_eq!(parse_fallback(SymbolKind::Item, "ITEM_"), None);
        assert_eq!(parse_fallback(SymbolKind::Item, "ITEM_POTION"), None);
        assert_eq!(parse_fallback(SymbolKind::Item, "ITEM_+1"), None);
        assert_eq!(parse_fallback(SymbolKind::Item, "ITEM_99999999999"), None);
        assert_eq!(
            parse_fallback(SymbolKind::Trainer, "TRAINER_CLASS_0001"),
            None
        );
        assert_eq!(
            parse_fallback(SymbolKind::TrainerClass, "TRAINER_CLASS_0001"),
            Some(1)
        );
        for kind in FIXED {
            assert_eq!(parse_fallback(kind, &fallback_id(kind, 600)), Some(600));
        }
    }

    #[test]
    fn normalize_follows_the_gutachten_rules() {
        assert_eq!(
            normalize("{TRAINER_NAME:Kai}", false).as_deref(),
            Some("KAI")
        );
        assert_eq!(normalize("Nidoran♀", false).as_deref(), Some("NIDORAN_F"));
        assert_eq!(normalize("Nidoran♂", true).as_deref(), Some("NIDORAN_M"));
        assert_eq!(
            normalize("Jungspund Jörg", true).as_deref(),
            Some("JUNGSPUND_JOERG")
        );
        assert_eq!(
            normalize("Jungspund Jörg", false).as_deref(),
            Some("JUNGSPUND_JORG")
        );
        assert_eq!(normalize("Pokémon", false).as_deref(), Some("POKEMON"));
        assert_eq!(normalize("Mr. Mime", false).as_deref(), Some("MR_MIME"));
        assert_eq!(normalize("???", false), None);
        assert_eq!(normalize("0042", false), None);
        assert_eq!(normalize("", true), None);
        assert_eq!(
            normalize("Rüpel & Co", true).as_deref(),
            Some("RUEPEL_AND_CO")
        );
        assert_eq!(normalize("Straße", true).as_deref(), Some("STRASSE"));
        assert_eq!(
            normalize("{COLOR 1}Team{COLOR 0} [PK][MN]\\nBoss", false).as_deref(),
            Some("TEAM_PKMN_BOSS")
        );
        assert_eq!(normalize("Farfetch’d", false).as_deref(), Some("FARFETCHD"));
        assert_eq!(normalize("Ça  va", false).as_deref(), Some("CA_VA"));
        assert_eq!(normalize("Route 201", false).as_deref(), Some("ROUTE_201"));
    }
}
