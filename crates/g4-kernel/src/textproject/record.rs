//! The generic mapper between a fixed record ([`RawRecord`]) and a JSON object, and the
//! symbolic references ([`SymRef`]) inside it.
//!
//! The object holds the record's fields under their codec names in `FIELDS` order: a field
//! without bit fields as its value (`U8`/`U16`/`U32` a number, `I8` a signed number, `Bytes`
//! lowercase hex); a container with bit fields only as its bit fields, followed by
//! `<container>_rest` when bits outside every mask are set. Fields that name a species, move,
//! item, ability or type ([`ref_kind`]) are written as the id when the whole value has one,
//! else as the number; reading takes either.

use std::collections::HashMap;
use std::fmt;

use g4_formats::data::record::{FieldKind, FieldSpan, RawRecord};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::canon::{hex, unhex};
use super::manifest::SCHEMA_CODE;
use super::registry::Symbols;
use super::symbols::{SymbolKind, fallback_id};

/// The code of a problem with an id that no table or register knows.
pub(crate) use super::codes::UNKNOWN_ID_CODE;

/// A fixed record as a JSON object, keys in `FIELDS` order (see the module docs). Its own
/// `Serialize`/`Deserialize` keep the order; reading rejects a key that appears twice.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecordJson(pub Vec<(String, Value)>);

/// A reference: an id (`"TYPE_FIRE"`) or a plain number.
#[derive(Clone, PartialEq, Debug)]
pub enum SymRef {
    Id(String),
    Num(u32),
}

/// The kind of symbol `field` of `record` names, if it is a reference: `personal` types,
/// items and abilities, `evolution` targets, `move.move_type`, `item.natural_gift_type` and
/// every species of an `encounter` (Platinum and HeartGold field names).
pub fn ref_kind(record: &str, field: &str) -> Option<SymbolKind> {
    match (record, field) {
        ("personal", "type1" | "type2") => Some(SymbolKind::Type),
        ("personal", "item1" | "item2") => Some(SymbolKind::Item),
        ("personal", "ability1" | "ability2") => Some(SymbolKind::Ability),
        ("evolution", f) if f.strip_prefix("target").is_some_and(|n| n.len() == 1) => {
            Some(SymbolKind::Species)
        }
        ("move", "move_type") => Some(SymbolKind::Type),
        ("item", "natural_gift_type") => Some(SymbolKind::Type),
        ("encounter", f) if encounter_species(f) => Some(SymbolKind::Species),
        _ => None,
    }
}

/// Whether encounter field `f` holds a species: Platinum `grass_NN_species`, `swarm_N`,
/// `day_N`, `night_N`, `radar_N`, `dual_<game>_N` and the water `…_species`; HeartGold
/// `grass_{morning,day,night}_NN`, `radio_N`, the water `…_species` and the four specials.
fn encounter_species(f: &str) -> bool {
    const PREFIXES: [&str; 9] = [
        "swarm_",
        "day_",
        "night_",
        "radar_",
        "dual_",
        "grass_morning_",
        "grass_day_",
        "grass_night_",
        "radio_",
    ];
    f.ends_with("_species")
        || PREFIXES.iter().any(|p| f.starts_with(p))
        || matches!(f, "land_swarm" | "surf_swarm" | "fish_swarm")
}

/// One key group of a record: a field without bit fields, or a container with its bit fields.
enum Entry<'a> {
    Plain(&'a FieldSpan),
    Container(&'a FieldSpan, Vec<&'a FieldSpan>),
}

/// `fields` grouped: each non-bit field with the bit fields inside its bytes.
fn layout(fields: &[FieldSpan]) -> Vec<Entry<'_>> {
    let is_bits = |f: &FieldSpan| matches!(f.kind, FieldKind::Bits { .. });
    fields
        .iter()
        .filter(|f| !is_bits(f))
        .map(|c| {
            let bits: Vec<&FieldSpan> = fields
                .iter()
                .filter(|b| {
                    is_bits(b) && c.offset <= b.offset && b.offset + b.len <= c.offset + c.len
                })
                .collect();
            if bits.is_empty() {
                Entry::Plain(c)
            } else {
                Entry::Container(c, bits)
            }
        })
        .collect()
}

fn mask_of(f: &FieldSpan) -> u32 {
    match f.kind {
        FieldKind::Bits { mask } => mask,
        _ => 0,
    }
}

/// The key of the bits of container `c` outside every mask.
fn rest_key(c: &FieldSpan) -> String {
    format!("{}_rest", c.name)
}

/// The bytes of container `c` with the bits of every mask cleared.
fn rest_bytes(raw: &[u8], c: &FieldSpan, bits: &[&FieldSpan]) -> Vec<u8> {
    let mut bytes = raw[c.offset..c.offset + c.len].to_vec();
    for b in bits {
        let at = b.offset - c.offset;
        for (k, mb) in mask_of(b).to_le_bytes().into_iter().enumerate() {
            if let Some(byte) = bytes.get_mut(at + k) {
                *byte &= !mb;
            }
        }
    }
    bytes
}

/// Little-endian bytes (at most 4) as a number.
fn le(bytes: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    buf[..bytes.len()].copy_from_slice(bytes);
    u32::from_le_bytes(buf)
}

/// The largest value of a numeric field of `kind` (and `len` bytes).
fn max_of(kind: FieldKind, len: usize) -> u64 {
    match kind {
        FieldKind::U8 | FieldKind::I8 => 0xFF,
        FieldKind::U16 => 0xFFFF,
        FieldKind::U32 => 0xFFFF_FFFF,
        FieldKind::Bits { mask } => u64::from(mask >> mask.trailing_zeros()),
        FieldKind::Bytes => (1u64 << (8 * len.min(4))) - 1,
    }
}

/// A number as JSON, or the id when `field` is a reference and the number has one.
fn number_value(n: u32, record: &str, field: &str, sym: &Symbols) -> Value {
    match ref_kind(record, field).map(|kind| sym_ref(sym, kind, n)) {
        Some(SymRef::Id(id)) => Value::String(id),
        _ => Value::from(n),
    }
}

/// `r` as a [`RecordJson`]; `record` is the record's name for [`ref_kind`].
pub fn record_to_json<R: RawRecord>(r: &R, record: &str, sym: &Symbols) -> RecordJson {
    let raw = r.raw();
    let mut out = Vec::with_capacity(R::FIELDS.len());
    for entry in layout(R::FIELDS) {
        match entry {
            Entry::Plain(f) => {
                let v = match f.kind {
                    FieldKind::U8 => {
                        number_value(u32::from(r.get_u8(f.offset)), record, f.name, sym)
                    }
                    FieldKind::U16 => {
                        number_value(u32::from(r.get_u16(f.offset)), record, f.name, sym)
                    }
                    FieldKind::U32 => number_value(r.get_u32(f.offset), record, f.name, sym),
                    FieldKind::I8 => Value::from(r.get_i8(f.offset)),
                    // A `Bits` field always lies in a container.
                    FieldKind::Bytes | FieldKind::Bits { .. } => {
                        Value::String(hex(&raw[f.offset..f.offset + f.len]))
                    }
                };
                out.push((f.name.to_owned(), v));
            }
            Entry::Container(c, bits) => {
                for b in &bits {
                    let n = r.bits(b.offset, mask_of(b));
                    out.push((b.name.to_owned(), number_value(n, record, b.name, sym)));
                }
                let rest = rest_bytes(raw, c, &bits);
                if rest.iter().any(|&b| b != 0) {
                    let v = match c.kind {
                        FieldKind::Bytes => Value::String(hex(&rest)),
                        _ => Value::from(le(&rest)),
                    };
                    out.push((rest_key(c), v));
                }
            }
        }
    }
    RecordJson(out)
}

/// The number in `v` for field `name` of `record` at JSON `path` (a reference may be an id),
/// at most `max`.
fn read_number(
    v: &Value,
    record: &str,
    path: &str,
    name: &str,
    max: u64,
    sym: &Symbols,
) -> Result<u32, String> {
    let n = match (v, ref_kind(record, name)) {
        (Value::String(id), Some(kind)) => u64::from(resolve(
            sym,
            kind,
            &SymRef::Id(id.clone()),
            &format!("{path}.{name}"),
        )?),
        (Value::Number(n), _) => n.as_u64().ok_or_else(|| {
            t!(
                "textproject.number_out_of_range",
                code = SCHEMA_CODE,
                path = format!("{path}.{name}"),
                n = n,
                max = max
            )
            .into_owned()
        })?,
        (other, kind) => {
            let key = if kind.is_some() {
                "textproject.expected_id_or_number"
            } else {
                "textproject.expected_number"
            };
            return Err(t!(
                key,
                code = SCHEMA_CODE,
                path = format!("{path}.{name}"),
                found = other
            )
            .into_owned());
        }
    };
    if n > max {
        return Err(t!(
            "textproject.number_out_of_range",
            code = SCHEMA_CODE,
            path = format!("{path}.{name}"),
            n = n,
            max = max
        )
        .into_owned());
    }
    Ok(n as u32)
}

/// The `len` bytes of hex field `name` at JSON `path`.
fn read_hex(v: &Value, path: &str, name: &str, len: usize) -> Result<Vec<u8>, String> {
    let Value::String(text) = v else {
        return Err(t!(
            "textproject.expected_hex",
            code = SCHEMA_CODE,
            path = format!("{path}.{name}"),
            found = v
        )
        .into_owned());
    };
    let bytes = unhex(text).map_err(|e| format!("{SCHEMA_CODE}: {path}.{name}: {e}"))?;
    if bytes.len() != len {
        return Err(t!(
            "textproject.hex_length",
            code = SCHEMA_CODE,
            path = format!("{path}.{name}"),
            count = bytes.len(),
            len = len
        )
        .into_owned());
    }
    Ok(bytes)
}

/// The record of `j`: every key of the layout exactly once (`<container>_rest` optional), no
/// other key, every value in its range, references as id or number. The error has the form
/// `<code>: <path>: <sentence>` (code `textproject.schema` or `textproject.unknown-id`), the
/// path starting with `record`; [`record_from_json_at`] puts the record at another path.
pub fn record_from_json<R: RawRecord>(
    j: &RecordJson,
    record: &str,
    sym: &Symbols,
) -> Result<R, String> {
    record_from_json_at(j, record, record, sym)
}

/// [`record_from_json`] for a record at JSON `path` of its entity (`data`, `personal`): an
/// error names the key's path (`data.powr`, `personal.type1`), a missing key the record's
/// path (`data: Schlüssel `power` fehlt.`).
pub(crate) fn record_from_json_at<R: RawRecord>(
    j: &RecordJson,
    record: &str,
    path: &str,
    sym: &Symbols,
) -> Result<R, String> {
    let entries = layout(R::FIELDS);
    let mut known: Vec<String> = Vec::new();
    for entry in &entries {
        match entry {
            Entry::Plain(f) => known.push(f.name.to_owned()),
            Entry::Container(c, bits) => {
                known.extend(bits.iter().map(|b| b.name.to_owned()));
                known.push(rest_key(c));
            }
        }
    }
    let mut given: HashMap<&str, &Value> = HashMap::with_capacity(j.0.len());
    for (k, v) in &j.0 {
        if !known.iter().any(|n| n == k) {
            return Err(t!(
                "textproject.record_unknown_key",
                code = SCHEMA_CODE,
                path = format!("{path}.{k}"),
                record = record
            )
            .into_owned());
        }
        if given.insert(k, v).is_some() {
            return Err(t!(
                "textproject.record_duplicate_key",
                code = SCHEMA_CODE,
                path = format!("{path}.{k}")
            )
            .into_owned());
        }
    }
    let take = |name: &str| {
        given.get(name).copied().ok_or_else(|| {
            t!(
                "textproject.record_missing_key",
                code = SCHEMA_CODE,
                path = path,
                name = name
            )
            .into_owned()
        })
    };

    let mut raw = vec![0u8; R::LEN];
    let mut bit_values = Vec::new();
    for entry in &entries {
        match entry {
            Entry::Plain(f) => {
                let v = take(f.name)?;
                let slot = &mut raw[f.offset..f.offset + f.len];
                match f.kind {
                    FieldKind::I8 => {
                        let n =
                            v.as_i64()
                                .filter(|n| (-128..=127).contains(n))
                                .ok_or_else(|| {
                                    t!(
                                        "textproject.i8_out_of_range",
                                        code = SCHEMA_CODE,
                                        path = format!("{path}.{}", f.name),
                                        value = v
                                    )
                                    .into_owned()
                                })?;
                        slot[0] = n as i8 as u8;
                    }
                    FieldKind::Bytes | FieldKind::Bits { .. } => {
                        slot.copy_from_slice(&read_hex(v, path, f.name, f.len)?);
                    }
                    FieldKind::U8 | FieldKind::U16 | FieldKind::U32 => {
                        let n = read_number(v, record, path, f.name, max_of(f.kind, f.len), sym)?;
                        slot.copy_from_slice(&n.to_le_bytes()[..f.len]);
                    }
                }
            }
            Entry::Container(c, bits) => {
                let key = rest_key(c);
                if let Some(v) = given.get(key.as_str()) {
                    let rest = match c.kind {
                        FieldKind::Bytes => read_hex(v, path, &key, c.len)?,
                        _ => {
                            let n = read_number(v, record, path, &key, max_of(c.kind, c.len), sym)?;
                            n.to_le_bytes()[..c.len].to_vec()
                        }
                    };
                    if rest_bytes(&rest_padded(&rest, c), c, bits) != rest {
                        return Err(t!(
                            "textproject.rest_sets_bits",
                            code = SCHEMA_CODE,
                            path = format!("{path}.{key}"),
                            field = c.name
                        )
                        .into_owned());
                    }
                    raw[c.offset..c.offset + c.len].copy_from_slice(&rest);
                }
                for b in bits {
                    let mask = mask_of(b);
                    let n = read_number(
                        take(b.name)?,
                        record,
                        path,
                        b.name,
                        max_of(b.kind, b.len),
                        sym,
                    )?;
                    bit_values.push((b.offset, mask, n));
                }
            }
        }
    }
    let mut r = R::read(&raw).map_err(|e| format!("{SCHEMA_CODE}: {path}: {e}"))?;
    for (offset, mask, n) in bit_values {
        r.set_bits(offset, mask, n);
    }
    Ok(r)
}

/// `rest` placed at container `c`'s offset in a buffer [`rest_bytes`] can read.
fn rest_padded(rest: &[u8], c: &FieldSpan) -> Vec<u8> {
    let mut buf = vec![0u8; c.offset + c.len];
    buf[c.offset..].copy_from_slice(rest);
    buf
}

/// `n` as a reference of `kind`: its id when the table or register has one, else the number.
pub(crate) fn sym_ref(sym: &Symbols, kind: SymbolKind, n: u32) -> SymRef {
    match sym.id(kind, n) {
        Some(id) => SymRef::Id(id),
        None => SymRef::Num(n),
    }
}

/// The number of `r`; an unknown id is a `textproject.unknown-id` error naming `field`.
pub(crate) fn resolve(
    sym: &Symbols,
    kind: SymbolKind,
    r: &SymRef,
    field: &str,
) -> Result<u32, String> {
    match r {
        SymRef::Num(n) => Ok(*n),
        SymRef::Id(id) => sym.number(kind, id).ok_or_else(|| {
            t!(
                "textproject.unknown_id_of_kind",
                code = UNKNOWN_ID_CODE,
                field = field,
                id = id,
                prefix = kind.prefix()
            )
            .into_owned()
        }),
    }
}

/// A [`RecordJson`] of `R` with every key (each `<container>_rest` too), zeros and the
/// fallback id of 0 for references: the snapshot's sample.
pub(crate) fn sample_record<R: RawRecord>(record: &str) -> RecordJson {
    let value = |f: &FieldSpan| match (ref_kind(record, f.name), f.kind) {
        (Some(kind), _) => Value::String(fallback_id(kind, 0)),
        (None, FieldKind::Bytes) => Value::String(hex(&vec![0; f.len])),
        (None, _) => Value::from(0),
    };
    let mut out = Vec::new();
    for entry in layout(R::FIELDS) {
        match entry {
            Entry::Plain(f) => out.push((f.name.to_owned(), value(f))),
            Entry::Container(c, bits) => {
                out.extend(bits.iter().map(|b| (b.name.to_owned(), value(b))));
                out.push((rest_key(c), value(c)));
            }
        }
    }
    RecordJson(out)
}

/// [`sample_record`] without the `<container>_rest` keys, which the export leaves out when
/// they are 0: the snapshot's minimal sample.
pub(crate) fn minimal_record<R: RawRecord>(record: &str) -> RecordJson {
    let rest: Vec<String> = layout(R::FIELDS)
        .iter()
        .filter_map(|entry| match entry {
            Entry::Container(c, _) => Some(rest_key(c)),
            Entry::Plain(_) => None,
        })
        .collect();
    let mut j = sample_record::<R>(record);
    j.0.retain(|(k, _)| !rest.contains(k));
    j
}

impl Serialize for RecordJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for RecordJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RecordVisitor;

        impl<'de> Visitor<'de> for RecordVisitor {
            type Value = RecordJson;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object with the fields of the record")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<RecordJson, A::Error> {
                let mut out: Vec<(String, Value)> = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    if out.iter().any(|(k, _)| *k == key) {
                        return Err(de::Error::custom(format_args!("duplicate field `{key}`")));
                    }
                    let value = map.next_value::<Value>()?;
                    out.push((key, value));
                }
                Ok(RecordJson(out))
            }
        }

        deserializer.deserialize_map(RecordVisitor)
    }
}

impl Serialize for SymRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SymRef::Id(id) => serializer.serialize_str(id),
            SymRef::Num(n) => serializer.serialize_u32(*n),
        }
    }
}

impl<'de> Deserialize<'de> for SymRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RefVisitor;

        impl Visitor<'_> for RefVisitor {
            type Value = SymRef;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an id or a number from 0 to 4294967295")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<SymRef, E> {
                Ok(SymRef::Id(v.to_owned()))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<SymRef, E> {
                u32::try_from(v)
                    .map(SymRef::Num)
                    .map_err(|_| E::invalid_value(de::Unexpected::Unsigned(v), &self))
            }
        }

        deserializer.deserialize_any(RefVisitor)
    }
}

#[cfg(test)]
mod tests {
    use g4_formats::data::encounter::{HgssEncounters, PtEncounters};
    use g4_formats::data::evolution::Evolution;
    use g4_formats::data::item::Item;
    use g4_formats::data::moves::Move;
    use g4_formats::data::personal::Personal;
    use proptest::prelude::*;
    use serde_json::json;

    use super::super::canon::to_canonical;
    use super::*;

    pub(crate) fn symbols() -> Symbols {
        Symbols::load(std::path::Path::new("/nonexistent/g4-textproject")).unwrap()
    }

    fn keys(j: &RecordJson) -> Vec<&str> {
        j.0.iter().map(|(k, _)| k.as_str()).collect()
    }

    fn get<'a>(j: &'a RecordJson, key: &str) -> Option<&'a Value> {
        j.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn set(j: &mut RecordJson, key: &str, value: Value) {
        j.0.iter_mut().find(|(k, _)| k == key).unwrap().1 = value;
    }

    fn personal(bytes: [u8; 44]) -> Personal {
        Personal::read(&bytes).unwrap()
    }

    #[test]
    fn record_json_writes_bits_instead_of_their_container() {
        let sym = symbols();
        let mut raw = [0u8; 44];
        raw[0x19] = 0x85; // color 5, flip
        raw[0x0A] = 0x02; // ev_yield_hp 2
        let j = record_to_json(&personal(raw), "personal", &sym);
        let k = keys(&j);
        assert!(k.contains(&"color") && k.contains(&"flip"));
        assert!(!k.contains(&"color_flip"));
        assert!(!k.contains(&"ev_yield"));
        assert!(!k.contains(&"ev_yield_rest"), "no bit outside the masks");
        assert_eq!(get(&j, "color"), Some(&json!(5)));
        assert_eq!(get(&j, "flip"), Some(&json!(1)));
        assert_eq!(get(&j, "ev_yield_hp"), Some(&json!(2)));
        // Bit fields follow in FIELDS order where their container stood.
        let at = |name| k.iter().position(|&x| x == name).unwrap();
        assert_eq!(at("base_exp") + 1, at("ev_yield_hp"));
        assert_eq!(at("ev_yield_sp_defense") + 1, at("item1"));
        assert_eq!(at("flee_rate") + 1, at("color"));
        assert_eq!(at("flip") + 1, at("pad_1a"));
        assert_eq!(get(&j, "pad_1a"), Some(&json!("0000")));

        // A bit outside every mask of `ev_yield` (bits 12-15) appears as its rest.
        raw[0x0B] = 0x30;
        let j = record_to_json(&personal(raw), "personal", &sym);
        assert_eq!(get(&j, "ev_yield_rest"), Some(&json!(0x3000)));
        let at = |name| keys(&j).iter().position(|&x| x == name).unwrap();
        assert_eq!(at("ev_yield_sp_defense") + 1, at("ev_yield_rest"));
        let back: Personal = record_from_json(&j, "personal", &sym).unwrap();
        assert_eq!(back.raw(), &raw);
    }

    #[test]
    fn record_json_refuses_unknown_missing_and_out_of_range() {
        let sym = symbols();
        let good = record_to_json(&Move::read(&[0; 16]).unwrap(), "move", &sym);

        let mut unknown = good.clone();
        unknown.0.push(("powr".into(), json!(1)));
        let err = record_from_json::<Move>(&unknown, "move", &sym).unwrap_err();
        assert!(err.starts_with("textproject.schema: move.powr: "), "{err}");

        let mut missing = good.clone();
        missing.0.retain(|(k, _)| k != "power");
        let err = record_from_json::<Move>(&missing, "move", &sym).unwrap_err();
        assert!(err.starts_with("textproject.schema: move: "), "{err}");
        assert!(err.contains("`power`"), "{err}");

        let mut range = good.clone();
        set(&mut range, "power", json!(256));
        let err = record_from_json::<Move>(&range, "move", &sym).unwrap_err();
        assert!(err.contains("move.power") && err.contains("256"), "{err}");

        let mut signed = good.clone();
        set(&mut signed, "priority", json!(-129));
        let err = record_from_json::<Move>(&signed, "move", &sym).unwrap_err();
        assert!(err.contains("move.priority"), "{err}");
        set(&mut signed, "priority", json!(-6));
        let m = record_from_json::<Move>(&signed, "move", &sym).unwrap();
        assert_eq!(m.priority(), -6);

        let mut bits = record_to_json(&personal([0; 44]), "personal", &sym);
        set(&mut bits, "color", json!(0x80));
        let err = record_from_json::<Personal>(&bits, "personal", &sym).unwrap_err();
        assert!(
            err.contains("personal.color") && err.contains("127"),
            "{err}"
        );

        let mut rest = record_to_json(&personal([0; 44]), "personal", &sym);
        rest.0.push(("ev_yield_rest".into(), json!(0x0001)));
        let err = record_from_json::<Personal>(&rest, "personal", &sym).unwrap_err();
        assert!(err.contains("personal.ev_yield_rest"), "{err}");

        let mut hex = good.clone();
        set(&mut hex, "pad_0e", json!("00"));
        let err = record_from_json::<Move>(&hex, "move", &sym).unwrap_err();
        assert!(err.contains("move.pad_0e"), "{err}");

        let mut text = good.clone();
        set(&mut text, "power", json!("viel"));
        let err = record_from_json::<Move>(&text, "move", &sym).unwrap_err();
        assert!(err.contains("move.power"), "{err}");

        let mut twice = good;
        twice.0.push(("power".into(), json!(1)));
        let err = record_from_json::<Move>(&twice, "move", &sym).unwrap_err();
        assert!(err.contains("move.power"), "{err}");
        // Reading JSON already refuses a key that appears twice, with its position.
        let err = serde_json::from_str::<RecordJson>("{\"a\": 1,\n \"a\": 2}").unwrap_err();
        assert!(err.to_string().contains("`a`"), "{err}");
        assert_eq!(err.line(), 2);
    }

    #[test]
    fn refs_are_written_as_ids_and_read_as_id_or_number() {
        let sym = symbols();
        let mut raw = [0u8; 44];
        raw[0x06] = 10; // type1 fire
        raw[0x07] = 200; // type2 without an entry
        let j = record_to_json(&personal(raw), "personal", &sym);
        assert_eq!(get(&j, "type1"), Some(&json!("TYPE_FIRE")));
        assert_eq!(get(&j, "type2"), Some(&json!(200)));
        assert_eq!(get(&j, "ability1"), Some(&json!("ABILITY_NONE")));

        let mut by_number = j.clone();
        set(&mut by_number, "type1", json!(10));
        for j in [&j, &by_number] {
            let back: Personal = record_from_json(j, "personal", &sym).unwrap();
            assert_eq!(back.raw(), &raw);
        }

        let mut unknown = j.clone();
        set(&mut unknown, "type1", json!("TYPE_FOO"));
        let err = record_from_json::<Personal>(&unknown, "personal", &sym).unwrap_err();
        assert!(err.starts_with("textproject.unknown-id: "), "{err}");
        assert!(
            err.contains("personal.type1") && err.contains("TYPE_FOO"),
            "{err}"
        );

        // An id of another kind is unknown here; an id outside the field's range too.
        set(&mut unknown, "type1", json!("MOVE_POUND"));
        let err = record_from_json::<Personal>(&unknown, "personal", &sym).unwrap_err();
        assert!(err.starts_with("textproject.unknown-id: "), "{err}");
        set(&mut unknown, "type1", json!("TYPE_0300"));
        let err = record_from_json::<Personal>(&unknown, "personal", &sym).unwrap_err();
        assert!(
            err.contains("personal.type1") && err.contains("300"),
            "{err}"
        );

        // A bit field can be a reference too.
        let mut item = [0u8; 34];
        item[0x08] = 10 | 0x20; // natural gift fire, prevent toss
        let j = record_to_json(&Item::read(&item).unwrap(), "item", &sym);
        assert_eq!(get(&j, "natural_gift_type"), Some(&json!("TYPE_FIRE")));
        let back: Item = record_from_json(&j, "item", &sym).unwrap();
        assert_eq!(back.raw(), &item);
    }

    #[test]
    fn pt_encounter_species_word_with_upper_bits_stays_a_number() {
        let sym = symbols();
        let mut raw = vec![0u8; 424];
        raw[0x008..0x00C].copy_from_slice(&25u32.to_le_bytes());
        raw[0x010..0x014].copy_from_slice(&0x0001_0019u32.to_le_bytes());
        let j = record_to_json(&PtEncounters::read(&raw).unwrap(), "encounter", &sym);
        assert_eq!(get(&j, "grass_00_species"), Some(&json!("SPECIES_PIKACHU")));
        assert_eq!(get(&j, "grass_01_species"), Some(&json!(0x0001_0019)));
        assert_eq!(get(&j, "grass_00_level"), Some(&json!(0)), "no reference");
        let back: PtEncounters = record_from_json(&j, "encounter", &sym).unwrap();
        assert_eq!(back.raw(), &raw[..]);
    }

    #[test]
    fn ref_kinds_cover_every_species_of_both_encounter_layouts() {
        let species = |fields: &[FieldSpan]| {
            fields
                .iter()
                .filter(|f| ref_kind("encounter", f.name) == Some(SymbolKind::Species))
                .count()
        };
        // Pt: 12 grass, 2 swarm, 2 day, 2 night, 4 radar, 10 dual, 5 surf, 15 rods.
        assert_eq!(species(PtEncounters::FIELDS), 52);
        // HGSS: 3 × 12 grass, 4 radio, 5 surf, 2 rock, 15 rods, 4 specials.
        assert_eq!(species(HgssEncounters::FIELDS), 66);
        assert_eq!(ref_kind("encounter", "grass_00_level"), None);
        assert_eq!(ref_kind("encounter", "form_rate_0"), None);
        assert_eq!(ref_kind("encounter", "grass_level_00"), None);
        let targets = Evolution::FIELDS
            .iter()
            .filter(|f| ref_kind("evolution", f.name) == Some(SymbolKind::Species))
            .count();
        assert_eq!(targets, 7);
        assert_eq!(ref_kind("evolution", "param0"), None);
        assert_eq!(ref_kind("move", "move_type"), Some(SymbolKind::Type));
        assert_eq!(ref_kind("personal", "item2"), Some(SymbolKind::Item));
        assert_eq!(ref_kind("personal", "ability2"), Some(SymbolKind::Ability));
        assert_eq!(ref_kind("personal", "hp"), None);
    }

    #[test]
    fn sym_ref_reads_an_id_or_a_number() {
        assert_eq!(
            serde_json::from_str::<SymRef>("\"MOVE_POUND\"").unwrap(),
            SymRef::Id("MOVE_POUND".into())
        );
        assert_eq!(serde_json::from_str::<SymRef>("7").unwrap(), SymRef::Num(7));
        let err = serde_json::from_str::<SymRef>("-1").unwrap_err();
        assert!(err.to_string().contains("an id or a number"), "{err}");
        assert!(serde_json::from_str::<SymRef>("4294967296").is_err());
        assert_eq!(serde_json::to_string(&SymRef::Num(3)).unwrap(), "3");
        assert_eq!(
            serde_json::to_string(&SymRef::Id("X".into())).unwrap(),
            "\"X\""
        );
    }

    fn round_trip<R: RawRecord>(bytes: &[u8], record: &str) -> Vec<u8> {
        let sym = symbols();
        let j = record_to_json(&R::read(bytes).unwrap(), record, &sym);
        let text = to_canonical(&j);
        let again: RecordJson = serde_json::from_slice(&text).unwrap();
        assert_eq!(again, j);
        record_from_json::<R>(&again, record, &sym).unwrap().write()
    }

    proptest! {
        #[test]
        fn prop_record_bytes_round_trip(
            personal in proptest::collection::vec(any::<u8>(), 44),
            evolution in proptest::collection::vec(any::<u8>(), 44),
            moves in proptest::collection::vec(any::<u8>(), 16),
            item in proptest::collection::vec(any::<u8>(), 34),
            pt in proptest::collection::vec(any::<u8>(), 424),
            hgss in proptest::collection::vec(any::<u8>(), 196),
        ) {
            prop_assert_eq!(round_trip::<Personal>(&personal, "personal"), personal);
            prop_assert_eq!(round_trip::<Evolution>(&evolution, "evolution"), evolution);
            prop_assert_eq!(round_trip::<Move>(&moves, "move"), moves);
            prop_assert_eq!(round_trip::<Item>(&item, "item"), item);
            prop_assert_eq!(round_trip::<PtEncounters>(&pt, "encounter"), pt);
            prop_assert_eq!(round_trip::<HgssEncounters>(&hgss, "encounter"), hgss);
        }
    }
}
