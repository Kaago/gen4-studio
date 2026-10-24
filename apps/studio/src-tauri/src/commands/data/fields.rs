//! Reading and writing one named field of a fixed record by its [`FieldSpan`], so the data
//! commands work on any record through its field map instead of one accessor per field.

use g4_formats::data::record::{FieldKind, FieldSpan, RawRecord};
use serde::Serialize;

/// One field of a record as the editor shows it: its value, its width in bits and whether it
/// is signed (the range a `NumberField` allows).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldValueDto {
    pub name: String,
    pub value: i64,
    pub bits: u8,
    pub signed: bool,
}

/// A field span on the wire: `kind` in lower case, `mask` only for `bits`. The frontend's
/// `HexFieldSpan` (ui/primitives/HexBlock.tsx) is the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldSpanDto {
    pub name: String,
    pub offset: usize,
    pub len: usize,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<u32>,
}

impl From<&FieldSpan> for FieldSpanDto {
    fn from(f: &FieldSpan) -> Self {
        let (kind, mask) = match f.kind {
            FieldKind::U8 => ("u8", None),
            FieldKind::U16 => ("u16", None),
            FieldKind::U32 => ("u32", None),
            FieldKind::I8 => ("i8", None),
            FieldKind::Bits { mask } => ("bits", Some(mask)),
            FieldKind::Bytes => ("bytes", None),
        };
        FieldSpanDto {
            name: f.name.to_owned(),
            offset: f.offset,
            len: f.len,
            kind,
            mask,
        }
    }
}

/// The span named `name` in `fields`; `None` for an unknown name and for a `Bytes` span (raw
/// bytes are no number).
pub fn numeric_span<'f>(fields: &'f [FieldSpan], name: &str) -> Option<&'f FieldSpan> {
    fields
        .iter()
        .find(|f| f.name == name && f.kind != FieldKind::Bytes)
}

/// Width in bits and signedness of a numeric span.
pub fn width(span: &FieldSpan) -> (u8, bool) {
    match span.kind {
        FieldKind::U8 => (8, false),
        FieldKind::I8 => (8, true),
        FieldKind::U16 => (16, false),
        FieldKind::U32 => (32, false),
        // A mask has at most 32 bits.
        FieldKind::Bits { mask } => (mask.count_ones() as u8, false),
        FieldKind::Bytes => (0, false),
    }
}

/// The value of a numeric span.
pub fn get<R: RawRecord>(record: &R, span: &FieldSpan) -> i64 {
    match span.kind {
        FieldKind::U8 => i64::from(record.get_u8(span.offset)),
        FieldKind::I8 => i64::from(record.get_i8(span.offset)),
        FieldKind::U16 => i64::from(record.get_u16(span.offset)),
        FieldKind::U32 => i64::from(record.get_u32(span.offset)),
        FieldKind::Bits { mask } => i64::from(record.bits(span.offset, mask)),
        FieldKind::Bytes => 0,
    }
}

/// Whether `value` fits the span (`0..2^bits`, or `-128..=127` for an `i8`).
pub fn fits(span: &FieldSpan, value: i64) -> bool {
    let (bits, signed) = width(span);
    if bits == 0 {
        return false;
    }
    if signed {
        let half = 1i64 << (bits - 1);
        (-half..half).contains(&value)
    } else {
        (0..(1i64 << bits)).contains(&value)
    }
}

/// Writes `value` into a numeric span; the caller checked it with [`fits`].
pub fn set<R: RawRecord>(record: &mut R, span: &FieldSpan, value: i64) {
    // `fits` bounds the value to the field's width, so the casts are exact.
    match span.kind {
        FieldKind::U8 => record.set_u8(span.offset, value as u8),
        FieldKind::I8 => record.set_i8(span.offset, value as i8),
        FieldKind::U16 => record.set_u16(span.offset, value as u16),
        FieldKind::U32 => record.set_u32(span.offset, value as u32),
        FieldKind::Bits { mask } => record.set_bits(span.offset, mask, value as u32),
        FieldKind::Bytes => {}
    }
}

/// The fields of `record` named in `names` (the editable ones, in table order) with their
/// values.
pub fn values<R: RawRecord>(record: &R, names: &[&str]) -> Vec<FieldValueDto> {
    names
        .iter()
        .filter_map(|&name| {
            let span = numeric_span(R::FIELDS, name)?;
            let (bits, signed) = width(span);
            Some(FieldValueDto {
                name: name.to_owned(),
                value: get(record, span),
                bits,
                signed,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use g4_formats::data::item::Item;
    use g4_formats::data::moves::Move;

    #[test]
    fn spans_serialize_like_the_hex_block_expects() {
        let price = numeric_span(Item::FIELDS, "price").unwrap();
        let pocket = numeric_span(Item::FIELDS, "field_pocket").unwrap();
        assert_eq!(
            serde_json::to_value(FieldSpanDto::from(price)).unwrap(),
            serde_json::json!({ "name": "price", "offset": 0, "len": 2, "kind": "u16" })
        );
        assert_eq!(
            serde_json::to_value(FieldSpanDto::from(pocket)).unwrap(),
            serde_json::json!({ "name": "field_pocket", "offset": 8, "len": 2, "kind": "bits", "mask": 0x0780 })
        );
        assert!(numeric_span(Item::FIELDS, "pad_20").is_none(), "bytes");
        assert!(numeric_span(Item::FIELDS, "no_such").is_none());
    }

    #[test]
    fn get_set_and_fits_follow_the_kind() {
        let mut m = Move::read(&[0; 16]).unwrap();
        let priority = numeric_span(Move::FIELDS, "priority").unwrap();
        assert!(fits(priority, -7) && fits(priority, 127) && !fits(priority, 128));
        set(&mut m, priority, -3);
        assert_eq!(get(&m, priority), -3);
        assert_eq!(m.priority(), -3);
        let mut item = Item::read(&[0; 34]).unwrap();
        let pocket = numeric_span(Item::FIELDS, "field_pocket").unwrap();
        assert_eq!(width(pocket), (4, false));
        assert!(fits(pocket, 15) && !fits(pocket, 16) && !fits(pocket, -1));
        set(&mut item, pocket, 5);
        assert_eq!(item.field_pocket(), 5);
        assert_eq!(item.battle_pocket(), 0, "neighbour bits stay");
        let v = values(&item, &["price", "field_pocket", "pad_20"]);
        assert_eq!(
            v,
            [
                FieldValueDto {
                    name: "price".into(),
                    value: 0,
                    bits: 16,
                    signed: false
                },
                FieldValueDto {
                    name: "field_pocket".into(),
                    value: 5,
                    bits: 4,
                    signed: false
                },
            ]
        );
    }
}
