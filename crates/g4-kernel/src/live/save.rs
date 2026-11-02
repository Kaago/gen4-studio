//! Where flags and variables of the running game lie in RAM, and a snapshot of their values.
//!
//! The save block is found through a pointer (`metadata/save_layout.yaml`): the u32 at
//! [`SaveBlock::pointer`] points at the general block; the variables and flags lie at fixed
//! offsets after its header. Before the save is loaded the pointer is not valid.

use crate::flags::{FlagKind, SaveBlock};

use super::gdb::LiveError;

/// Start of the DS main RAM.
pub const RAM_START: u32 = 0x0200_0000;
/// End (exclusive) of the DS main RAM.
pub const RAM_END: u32 = 0x0240_0000;
/// Where the game code of the running cartridge lies (four ASCII bytes, `CPUD`).
pub const GAME_CODE_ADDR: u32 = 0x023F_FE0C;

/// The first variable number; variable `v` is the u16 at `2 * (v - VAR_BASE)`.
const VAR_BASE: u16 = 0x4000;

/// A flag or a variable of the save.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    Flag { number: u16 },
    Var { number: u16 },
}

impl Target {
    /// Flag or variable.
    pub fn kind(self) -> FlagKind {
        match self {
            Target::Flag { .. } => FlagKind::Flag,
            Target::Var { .. } => FlagKind::Var,
        }
    }

    /// The flag or variable number.
    pub fn number(self) -> u16 {
        match self {
            Target::Flag { number } | Target::Var { number } => number,
        }
    }

    /// [`LiveError::OutsideSave`] for this target.
    pub fn outside_save(self) -> LiveError {
        LiveError::OutsideSave {
            kind: self.kind(),
            number: self.number(),
        }
    }
}

/// Offset of flag `n` after the header (byte offset, bit); `None` past `flags_bytes * 8`.
pub(crate) fn flag_offset(block: &SaveBlock, n: u16) -> Option<(u32, u8)> {
    let n = u32::from(n);
    (n < block.flags_bytes * 8).then(|| (block.flags + n / 8, (n % 8) as u8))
}

/// Offset of variable `v` after the header; `None` outside `0x4000..0x4000 + vars_count`.
pub(crate) fn var_offset(block: &SaveBlock, v: u16) -> Option<u32> {
    let index = u32::from(v.checked_sub(VAR_BASE)?);
    (index < block.vars_count).then(|| block.vars + 2 * index)
}

/// Whether `target` lies inside the save of `block`.
pub(crate) fn in_block(block: &SaveBlock, target: Target) -> bool {
    match target {
        Target::Flag { number } => flag_offset(block, number).is_some(),
        Target::Var { number } => var_offset(block, number).is_some(),
    }
}

/// The save block of the running game, resolved through its pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveView {
    pub block: SaveBlock,
    /// The pointer target plus the header: the offsets of the layout count from here.
    pub base: u32,
}

impl SaveView {
    /// The view for the pointer value read from RAM; `None` when the pointer is outside RAM, not
    /// word-aligned, or the block would run past the end of RAM (the save is not loaded yet).
    pub fn resolve(block: SaveBlock, pointer_value: u32) -> Option<SaveView> {
        if !(RAM_START..RAM_END).contains(&pointer_value) || !pointer_value.is_multiple_of(4) {
            return None;
        }
        let base = pointer_value.checked_add(block.header)?;
        let flags_end = block.flags.checked_add(block.flags_bytes)?;
        let vars_end = block.vars.checked_add(block.vars_count.checked_mul(2)?)?;
        let end = base.checked_add(flags_end.max(vars_end))?;
        (end <= RAM_END).then_some(SaveView { block, base })
    }

    /// (byte address, bit) of flag `n`; `None` outside `flags_bytes * 8`.
    pub fn flag_addr(&self, n: u16) -> Option<(u32, u8)> {
        flag_offset(&self.block, n).map(|(offset, bit)| (self.base + offset, bit))
    }

    /// Address of the u16 of variable `v`; `None` outside `0x4000..0x4000 + vars_count`.
    pub fn var_addr(&self, v: u16) -> Option<u32> {
        var_offset(&self.block, v).map(|offset| self.base + offset)
    }

    /// (address, length in bytes) of the flags.
    pub fn flags_range(&self) -> (u32, usize) {
        (
            self.base + self.block.flags,
            self.block.flags_bytes as usize,
        )
    }

    /// (address, length in bytes) of the variables (two bytes each).
    pub fn vars_range(&self) -> (u32, usize) {
        (
            self.base + self.block.vars,
            self.block.vars_count as usize * 2,
        )
    }
}

/// The flags and variables of the save at one moment (read while the game was halted).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Snapshot {
    /// Counts the snapshots of a session from 1.
    pub revision: u64,
    /// `flags_bytes` bytes; flag `n` is bit `n % 8` of byte `n / 8`.
    pub flags: Vec<u8>,
    /// `vars_count` values; variable `v` is `vars[v - 0x4000]`.
    pub vars: Vec<u16>,
    /// How long the halt-read-continue cycle took.
    pub taken_ms: u32,
}

impl Snapshot {
    /// Whether flag `n` is set; `None` outside the snapshot.
    pub fn flag(&self, n: u16) -> Option<bool> {
        let byte = self.flags.get(usize::from(n / 8))?;
        Some(byte & (1 << (n % 8)) != 0)
    }

    /// The value of variable `v`; `None` outside the snapshot.
    pub fn var(&self, v: u16) -> Option<u16> {
        self.vars.get(usize::from(v.checked_sub(VAR_BASE)?)).copied()
    }

    /// The value of `target` as u16 (a flag is 0 or 1); `None` outside the snapshot.
    pub fn value(&self, target: Target) -> Option<u16> {
        match target {
            Target::Flag { number } => self.flag(number).map(u16::from),
            Target::Var { number } => self.var(number),
        }
    }
}
