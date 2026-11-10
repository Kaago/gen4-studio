//! The free numbers: the pool new flags and variables are taken from.
//!
//! A number of the general area ([`Area::General`]; the hidden items inside it are their own
//! area) is free when the reference index knows no reference to it (any access counts), the
//! project gives it no own name, its pret name is missing or says it is unused
//! ([`name_is_unused`]) and no recipe reserves it ([`reserved`]: the `done_var` of
//! `metadata/templates.yaml`). The name rule only narrows: `FLAG_UNUSED_0x0096` at an object is
//! not free, and a named progress flag without a script reference (the engine sets it) never is.

use std::collections::BTreeMap;

use g4_formats::game::Family;
use serde::Serialize;

use super::{Area, FlagKind, Range, area_of, name_is_unused, names, own_names, pool_range};
use crate::data::refs::Usage;
use crate::error::KernelError;
use crate::project::Project;

/// Why "take a free flag" has nothing to give.
pub fn pool_empty() -> String {
    t!("flags.pool_empty").into_owned()
}

/// Why "take a free variable" has nothing to give.
pub fn pool_empty_var() -> String {
    t!("flags.pool_empty_var").into_owned()
}

/// The word for a variable the empty-region recipe reserves.
pub fn reserved_by_empty_region() -> String {
    t!("flags.reserved_by_empty_region").into_owned()
}

/// The pool of one kind: the general range, its free numbers (ascending), how many numbers the
/// general area has (`total`) and which of them a recipe reserves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pool {
    pub kind: FlagKind,
    pub range: Range,
    pub free: Vec<u16>,
    pub total: u32,
    pub reserved: Vec<u16>,
}

/// The pool of `kind`: every number of the general range that is unused in `usage`, has no own
/// name, whose pret name is unused ([`name_is_unused`]) and is not reserved (the template's
/// `done_var`); ascending. A register that does not read is its error.
pub fn pool(
    project: &Project,
    usage: &BTreeMap<u16, Usage>,
    kind: FlagKind,
) -> Result<Pool, KernelError> {
    let family = project.family;
    let range = pool_range(family, kind).ok_or_else(|| {
        KernelError::InvalidProject(format!(
            "flag_ranges.yaml: no general range of {} for {family:?}",
            kind.folder()
        ))
    })?;
    let own = own_names(project, kind)?;
    let pret = names(family, kind);
    let reserved: Vec<u16> = match kind {
        FlagKind::Flag => Vec::new(),
        FlagKind::Var => reserved(family)
            .into_iter()
            .map(|(n, _)| n)
            .filter(|n| (range.start..=range.end).contains(n))
            .collect(),
    };
    let general: Vec<u16> = (range.start..=range.end)
        .filter(|&n| area_of(family, kind, n) == Area::General)
        .collect();
    let free = general
        .iter()
        .copied()
        .filter(|n| {
            !usage.contains_key(n)
                && !own.contains_key(n)
                && name_is_unused(pret.name(*n))
                && !reserved.contains(n)
        })
        .collect();
    Ok(Pool {
        kind,
        range,
        free,
        // The general range has at most 65536 numbers.
        total: general.len() as u32,
        reserved,
    })
}

/// The smallest free flag ([`pool`]); `None` when the pool is empty ([`pool_empty`]).
pub fn free_flag(
    project: &Project,
    usage: &BTreeMap<u16, Usage>,
) -> Result<Option<u16>, KernelError> {
    Ok(pool(project, usage, FlagKind::Flag)?.free.first().copied())
}

/// The smallest free variable ([`pool`]); `None` when the pool is empty ([`pool_empty_var`]).
pub fn free_var(
    project: &Project,
    usage: &BTreeMap<u16, Usage>,
) -> Result<Option<u16>, KernelError> {
    Ok(pool(project, usage, FlagKind::Var)?.free.first().copied())
}

/// The variables the recipes of `metadata/templates.yaml` reserve for `family` (`done_var`),
/// with the recipe's word; none for Diamond and Pearl (no recipe) or a `done_var` the command
/// database does not name.
pub fn reserved(family: Family) -> Vec<(u16, String)> {
    crate::templates::region_recipe(family)
        .ok()
        .and_then(|recipe| crate::templates::variable_id(family, &recipe.done_var))
        .map(|n| (n, reserved_by_empty_region()))
        .into_iter()
        .collect()
}

/// Why the pool of `kind` has nothing to give: [`pool_empty`] or [`pool_empty_var`].
pub fn pool_empty_text(kind: FlagKind) -> String {
    match kind {
        FlagKind::Flag => pool_empty(),
        FlagKind::Var => pool_empty_var(),
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use g4_formats::game::Family;

    use super::*;
    use crate::data::refs::{RefIndex, Usage};
    use crate::flags::{Area, FlagKind, NameRequest, area_of, name_flag_files, names};
    use crate::store::{ObjectEvent, Store};
    use crate::test_support::TextFixture;

    fn used() -> Usage {
        Usage {
            read: 1,
            write: 0,
            unknown: 0,
            sources: 1,
        }
    }

    /// Every number of `kind`'s general range below `below` as used.
    fn used_below(kind: FlagKind, below: u16) -> BTreeMap<u16, Usage> {
        let range = crate::flags::pool_range(Family::Pt, kind).unwrap();
        (range.start..below).map(|n| (n, used())).collect()
    }

    fn object(hidden_flag: u16) -> ObjectEvent {
        ObjectEvent {
            local_id: 0,
            graphics_id: 0,
            movement_type: 0,
            trainer_type: 0,
            hidden_flag,
            script: 0,
            dir: 0,
            data: [0; 3],
            movement_range_x: 0,
            movement_range_z: 0,
            x: 0,
            z: 0,
            y: 0,
        }
    }

    #[test]
    fn the_pool_is_the_general_range_minus_used_named_and_reserved() {
        let fx = TextFixture::exported();
        let mut st = Store::new(Arc::new(fx.project()));
        st.edit_event(0, "Objekt", |e| {
            e.object_events.push(object(0x96));
            Ok(())
        })
        .unwrap();
        let usage = RefIndex::new().usage(&st, FlagKind::Flag).unwrap();
        let p = fx.project();
        let pool = pool(&p, &usage, FlagKind::Flag).unwrap();
        assert_eq!(pool.kind, FlagKind::Flag);
        assert_eq!((pool.range.start, pool.range.end), (0x41, 0x54F));
        assert_eq!(pool.range.area, Area::General);
        // The general area without the hidden items it holds.
        assert_eq!(pool.total, 0x54F - 0x41 + 1 - 284);
        assert!(pool.reserved.is_empty());
        assert!(pool.free.windows(2).all(|w| w[0] < w[1]));
        assert!(
            pool.free
                .iter()
                .all(|&n| area_of(Family::Pt, FlagKind::Flag, n) == Area::General)
        );
        // Used by the object: not free. Unused and named so: free.
        assert!(!pool.free.contains(&0x96));
        assert_eq!(names(Family::Pt, FlagKind::Flag).name(0x94), Some("FLAG_UNUSED_0x0094"));
        assert!(pool.free.contains(&0x94));
        // A pret name that is no "unused" name, without a reference: never free (the engine
        // may set it).
        let (hide, _) = names(Family::Pt, FlagKind::Flag)
            .iter()
            .find(|&(n, name)| {
                name.starts_with("FLAG_HIDE_")
                    && area_of(Family::Pt, FlagKind::Flag, n) == Area::General
                    && !usage.contains_key(&n)
            })
            .unwrap();
        assert!(!pool.free.contains(&hide));
        // An own name takes the number out of the pool.
        name_flag_files(
            &p,
            &NameRequest {
                kind: FlagKind::Flag,
                number: 148,
                id: Some("FLAG_HAT_MEDAILLE".to_owned()),
                description: None,
            },
        )
        .unwrap();
        let after = super::pool(&p, &usage, FlagKind::Flag).unwrap();
        assert!(!after.free.contains(&148));
        assert_eq!(after.free.len(), pool.free.len() - 1);
    }

    #[test]
    fn a_used_unused_flag_is_not_free() {
        let fx = TextFixture::exported();
        let p = fx.project();
        assert!(crate::flags::name_is_unused(
            names(Family::Pt, FlagKind::Flag).name(0x96)
        ));
        // Everything below 0x96 is used, so 0x96 would be the smallest free number.
        let mut usage = used_below(FlagKind::Flag, 0x96);
        assert_eq!(free_flag(&p, &usage).unwrap(), Some(0x96));
        usage.insert(0x96, used());
        let next = free_flag(&p, &usage).unwrap().unwrap();
        assert!(next > 0x96, "{next:#X}");
        assert!(
            !pool(&p, &usage, FlagKind::Flag)
                .unwrap()
                .free
                .contains(&0x96)
        );
    }

    #[test]
    fn free_var_skips_the_templates_done_var() {
        let fx = TextFixture::exported();
        let p = fx.project();
        assert_eq!(
            reserved(Family::Pt),
            [(0x406C, "Template “Empty region”".to_owned())]
        );
        assert_eq!(reserved(Family::Hgss), [(0x4031, "Template “Empty region”".to_owned())]);
        assert!(reserved(Family::Dp).is_empty());
        // Everything below 0x406C is used, so the done variable would be the smallest.
        let usage = used_below(FlagKind::Var, 0x406C);
        let free = free_var(&p, &usage).unwrap().unwrap();
        assert_ne!(free, 0x406C);
        assert!(free > 0x406C, "{free:#X}");
        let pool = pool(&p, &usage, FlagKind::Var).unwrap();
        assert_eq!(pool.reserved, [0x406C]);
        assert!(!pool.free.contains(&0x406C));
    }

    #[test]
    fn map_local_and_trainer_flags_are_never_free() {
        let fx = TextFixture::exported();
        let p = fx.project();
        let usage = BTreeMap::new();
        let flags = pool(&p, &usage, FlagKind::Flag).unwrap();
        for n in [0x01, 0x20, 0x40, 0x2DA, 0x3F5, 0x550, 0x95F, 0x960, 0xAA0, 0xB60] {
            assert!(!flags.free.contains(&n), "{n:#X}");
        }
        let vars = pool(&p, &usage, FlagKind::Var).unwrap();
        for n in [0x4000, 0x401F, 0x4020, 0x402F, 0x4120, 0x8000] {
            assert!(!vars.free.contains(&n), "{n:#X}");
        }
        assert!(
            vars.free
                .iter()
                .all(|&n| area_of(Family::Pt, FlagKind::Var, n) == Area::General)
        );
    }

    #[test]
    fn an_empty_pool_is_none() {
        let fx = TextFixture::exported();
        let p = fx.project();
        for kind in [FlagKind::Flag, FlagKind::Var] {
            let range = crate::flags::pool_range(Family::Pt, kind).unwrap();
            let usage: BTreeMap<u16, Usage> =
                (range.start..=range.end).map(|n| (n, used())).collect();
            let pool = pool(&p, &usage, kind).unwrap();
            assert!(pool.free.is_empty());
            assert!(pool.total > 0);
            let free = match kind {
                FlagKind::Flag => free_flag(&p, &usage),
                FlagKind::Var => free_var(&p, &usage),
            };
            assert_eq!(free.unwrap(), None);
        }
        assert_eq!(pool_empty_text(FlagKind::Flag), pool_empty());
        assert_eq!(pool_empty(), "No free flag in the general area.");
        assert_eq!(
            pool_empty_text(FlagKind::Var),
            "No free variable in the general area."
        );
    }
}
