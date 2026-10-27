//! Test helpers shared by the schema modules: an export that reads its entity back through
//! canonical JSON, an import against an empty base layer, and the code and path of an error.

use std::path::Path;

use g4_formats::game::Family;

use super::super::canon::to_canonical;
use super::super::part::tests::MemBase;
use super::super::registry::Symbols;
use super::{EntitySchema, ExportCtx, ImportCtx};

/// The symbols of a project without registers.
pub(crate) fn symbols() -> Symbols {
    Symbols::load(Path::new("/nonexistent/g4-textproject")).unwrap()
}

/// Exports entity `index` of `E` from `members` with `sym` and reads it back through
/// canonical JSON; the side files the export collected.
pub(crate) fn export_with<E: EntitySchema>(
    family: Family,
    index: u32,
    members: &[Option<Vec<u8>>],
    sym: &mut Symbols,
) -> (E, Vec<(String, Vec<u8>)>) {
    let base = MemBase::default();
    let mut ctx = ExportCtx {
        family,
        symbols: sym,
        base: &base,
        side: Vec::new(),
    };
    let e = E::export(index, members, &mut ctx).unwrap();
    let again: E = serde_json::from_slice(&to_canonical(&e)).unwrap();
    (again, ctx.side)
}

/// [`export_with`] without registers.
pub(crate) fn export<E: EntitySchema>(
    family: Family,
    index: u32,
    members: &[Option<Vec<u8>>],
) -> (E, Vec<(String, Vec<u8>)>) {
    export_with(family, index, members, &mut symbols())
}

/// The members of `e` with `sym`, no side files, no current members, an empty base layer.
pub(crate) fn import_with<E: EntitySchema>(
    family: Family,
    e: &E,
    index: u32,
    sym: &Symbols,
) -> Result<Vec<Option<Vec<u8>>>, String> {
    let base = MemBase::default();
    let ctx = ImportCtx {
        family,
        symbols: sym,
        dir: Path::new("/nonexistent"),
        base: &base,
        current: &[],
    };
    e.import(index, &ctx)
}

/// [`import_with`] without registers.
pub(crate) fn import<E: EntitySchema>(
    family: Family,
    e: &E,
    index: u32,
) -> Result<Vec<Option<Vec<u8>>>, String> {
    import_with(family, e, index, &symbols())
}

/// The `<code>` and `<path>` of an error `<code>: <path>: <sentence>`.
pub(crate) fn code_and_path(err: &str) -> (&str, &str) {
    let mut parts = err.splitn(3, ": ");
    (parts.next().unwrap(), parts.next().unwrap_or(""))
}
