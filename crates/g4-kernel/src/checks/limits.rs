//! The limits and exceptions of the checks, read from `metadata/check_limits.yaml`.
//!
//! Every entry carries its `source` and whether it is `verified` against the games' code (see
//! `docs/formats/check_limits.md`). The file is embedded and parsed once per process.

use std::sync::OnceLock;

use g4_formats::game::Family;
use serde::Deserialize;

use crate::project::Project;

const CHECK_LIMITS_YAML: &str = include_str!("../../metadata/check_limits.yaml");

/// One limit: a value, whether it was verified against the games' code, and where it comes from.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limit<T> {
    pub value: T,
    pub verified: bool,
    pub source: String,
}

/// The warp target pair the game reads from the save game: header `0x0FFF` with warp `256`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicWarp {
    pub header: u16,
    pub warp: u16,
    pub verified: bool,
    pub source: String,
}

/// The header value for "no encounters" per family.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterNone {
    pub pt: u16,
    pub hgss: u16,
    pub verified: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpLimits {
    pub dynamic: DynamicWarp,
}

/// The trainer ID of the sight script slot per family (none for Diamond/Pearl).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SightSlot {
    pub pt: u16,
    pub hgss: u16,
    pub verified: bool,
    pub source: String,
}

impl SightSlot {
    /// The slot of `family`; `None` for Diamond/Pearl, whose battle script is not known.
    pub fn of(&self, family: Family) -> Option<u16> {
        match family {
            Family::Dp => None,
            Family::Pt => Some(self.pt),
            Family::Hgss => Some(self.hgss),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerLimits {
    pub dummy: Limit<u16>,
    /// The trainer ID of the sight script slot, never a real trainer (the placeholder of
    /// `create_trainer`); the trainer checks skip it.
    pub sight_slot: SightSlot,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventFileLimits {
    pub shared: Limit<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptLimits {
    pub decoration: Limit<u16>,
    /// The commands whose argument is a message of the script's text archive.
    pub message_params: MessageParams,
    /// The commands that jump to a label of the same file.
    pub jump_params: JumpParams,
}

/// A script command and the position (0-based) of its message argument.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageParam {
    pub command: String,
    pub arg: usize,
}

/// The message commands per family (`script.message-missing`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageParams {
    pub dp: Vec<MessageParam>,
    pub pt: Vec<MessageParam>,
    pub hgss: Vec<MessageParam>,
    pub verified: bool,
    pub source: String,
}

impl MessageParams {
    /// The list of `family`.
    pub fn of(&self, family: Family) -> &[MessageParam] {
        match family {
            Family::Dp => &self.dp,
            Family::Pt => &self.pt,
            Family::Hgss => &self.hgss,
        }
    }
}

/// The jump commands per family (`dialog_of`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JumpParams {
    pub dp: Vec<String>,
    pub pt: Vec<String>,
    pub hgss: Vec<String>,
    pub verified: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterLimits {
    pub none: EncounterNone,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixLimits {
    pub header_none: Limit<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteLimits {
    pub max_bytes: Limit<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionLimits {
    pub bytes: Limit<usize>,
}

/// The buildings of a map the game loads.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingLimits {
    /// At most this many buildings per map are loaded; the rest is skipped silently
    /// (`map.buildings-over-limit`).
    pub max_loaded: Limit<usize>,
}

/// The largest texture VRAM (map plus building textures of an area) on a vanilla project, per
/// family.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VanillaVram {
    pub pt: usize,
    pub hgss: usize,
    pub verified: bool,
    pub source: String,
}

impl VanillaVram {
    /// The value of `family` (Diamond/Pearl as Platinum).
    pub fn of(&self, family: Family) -> usize {
        match family {
            Family::Dp | Family::Pt => self.pt,
            Family::Hgss => self.hgss,
        }
    }
}

/// The texture VRAM of the field: the bank, the vanilla maximum and the estimated full mark.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VramLimits {
    /// Bytes of the texture banks the field maps (shared with the characters).
    pub bank: Limit<usize>,
    /// Above this the budget says „Knapp“.
    pub vanilla_max: VanillaVram,
    /// Above this the budget says „Voll“ (an estimate: the characters' share is not measured).
    pub full_estimate: Limit<usize>,
}

/// All limits of `check_limits.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Width of the field text window in pixels.
    pub dialog_line_px: Limit<u32>,
    /// Lines the text window shows before a page break.
    pub dialog_lines_max: Limit<u32>,
    pub warp: WarpLimits,
    pub trainer: TrainerLimits,
    pub event_file: EventFileLimits,
    pub script: ScriptLimits,
    pub encounter: EncounterLimits,
    pub matrix: MatrixLimits,
    /// Largest map model in bytes.
    pub model: ByteLimits,
    /// Largest terrain payload in bytes, counted after the 16-byte BDHC header.
    pub bdhc: ByteLimits,
    pub permissions: PermissionLimits,
    pub buildings: BuildingLimits,
    pub vram: VramLimits,
}

/// One row of [`Limits::entries`]: the YAML key, the verified flag and the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitEntry<'a> {
    pub key: &'static str,
    pub verified: bool,
    pub source: &'a str,
}

impl Limits {
    /// The embedded limits, parsed once per process.
    pub fn get() -> &'static Limits {
        static LIMITS: OnceLock<Limits> = OnceLock::new();
        LIMITS.get_or_init(|| {
            // The file is embedded and parsed by a test, so a parse error cannot reach a user.
            serde_yaml::from_str(CHECK_LIMITS_YAML)
                .unwrap_or_else(|e| panic!("metadata/check_limits.yaml: {e}"))
        })
    }

    /// Every entry with its key, verified flag and source, in file order.
    pub fn entries(&self) -> Vec<LimitEntry<'_>> {
        fn row<'a, T>(key: &'static str, l: &'a Limit<T>) -> LimitEntry<'a> {
            LimitEntry {
                key,
                verified: l.verified,
                source: &l.source,
            }
        }
        vec![
            row("dialog_line_px", &self.dialog_line_px),
            row("dialog_lines_max", &self.dialog_lines_max),
            LimitEntry {
                key: "warp.dynamic",
                verified: self.warp.dynamic.verified,
                source: &self.warp.dynamic.source,
            },
            row("trainer.dummy", &self.trainer.dummy),
            LimitEntry {
                key: "trainer.sight_slot",
                verified: self.trainer.sight_slot.verified,
                source: &self.trainer.sight_slot.source,
            },
            row("event_file.shared", &self.event_file.shared),
            row("script.decoration", &self.script.decoration),
            LimitEntry {
                key: "script.message_params",
                verified: self.script.message_params.verified,
                source: &self.script.message_params.source,
            },
            LimitEntry {
                key: "script.jump_params",
                verified: self.script.jump_params.verified,
                source: &self.script.jump_params.source,
            },
            LimitEntry {
                key: "encounter.none",
                verified: self.encounter.none.verified,
                source: &self.encounter.none.source,
            },
            row("matrix.header_none", &self.matrix.header_none),
            row("model.max_bytes", &self.model.max_bytes),
            row("bdhc.max_bytes", &self.bdhc.max_bytes),
            row("permissions.bytes", &self.permissions.bytes),
            row("buildings.max_loaded", &self.buildings.max_loaded),
            row("vram.bank", &self.vram.bank),
            LimitEntry {
                key: "vram.vanilla_max",
                verified: self.vram.vanilla_max.verified,
                source: &self.vram.vanilla_max.source,
            },
            row("vram.full_estimate", &self.vram.full_estimate),
        ]
    }

    /// The commands whose label argument (an identifier naming a script or label of the same
    /// file) is a jump there (`script.jump_params`), as written in the file: a name is matched
    /// through rotom's database, so legacy and canonical names count alike.
    pub fn jump_commands(&self, family: Family) -> &[String] {
        let jumps = &self.script.jump_params;
        match family {
            Family::Dp => &jumps.dp,
            Family::Pt => &jumps.pt,
            Family::Hgss => &jumps.hgss,
        }
    }
}

impl Project {
    /// The limits and exceptions of the checks (`metadata/check_limits.yaml`). They do not depend
    /// on the project, so this is an associated function, cached for the process.
    pub fn limits() -> &'static Limits {
        Limits::get()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn limits_yaml_parses_with_sources() {
        // Parsed fresh (not through the cache), so a broken file fails here.
        let l: Limits = serde_yaml::from_str(CHECK_LIMITS_YAML).unwrap();
        assert_eq!(l.dialog_line_px.value, 216);
        assert_eq!(l.dialog_lines_max.value, 2);
        assert_eq!((l.warp.dynamic.header, l.warp.dynamic.warp), (0x0FFF, 256));
        assert_eq!(l.trainer.dummy.value, 0);
        // The sight slot is the one `create_trainer` keeps for its placeholder.
        assert_eq!(
            (l.trainer.sight_slot.pt, l.trainer.sight_slot.hgss),
            (929, 740)
        );
        for family in [Family::Pt, Family::Hgss] {
            assert_eq!(
                l.trainer.sight_slot.of(family),
                crate::scripts::trainer_script_meta(family).map(|m| m.seen_id()),
                "{family:?}"
            );
        }
        assert_eq!(l.trainer.sight_slot.of(Family::Dp), None);
        assert_eq!(l.event_file.shared.value, 0);
        assert_eq!(l.script.decoration.value, 0);
        assert_eq!((l.encounter.none.pt, l.encounter.none.hgss), (0xFFFF, 0xFF));
        assert_eq!(l.matrix.header_none.value, 0);
        assert_eq!(l.model.max_bytes.value, 0xF000);
        assert_eq!(l.bdhc.max_bytes.value, 0x9000);
        assert_eq!(l.permissions.bytes.value, 2048);
        // Phase 6.2: buildings the game loads, the texture VRAM of the field.
        assert_eq!(l.buildings.max_loaded.value, 32);
        assert!(l.buildings.max_loaded.verified);
        assert!(l.buildings.max_loaded.source.contains("map_prop.h:13"));
        assert_eq!(l.vram.bank.value, 262_144);
        assert!(l.vram.bank.verified);
        assert_eq!(l.vram.vanilla_max.of(Family::Pt), 110_640);
        assert_eq!(l.vram.vanilla_max.of(Family::Dp), 110_640);
        assert_eq!(l.vram.vanilla_max.of(Family::Hgss), 87_424);
        assert!(!l.vram.vanilla_max.verified);
        assert_eq!(l.vram.full_estimate.value, 204_800);
        assert!(!l.vram.full_estimate.verified);

        // The verified ones, with their pret sources named.
        assert!(l.dialog_line_px.verified);
        assert!(l.dialog_line_px.source.contains("field_message.c:30"));
        assert!(l.dialog_line_px.source.contains("dialog_box.c:12"));
        assert!(l.warp.dynamic.verified);
        assert!(l.model.max_bytes.verified && l.bdhc.max_bytes.verified);

        // Every entry names its source.
        let entries = l.entries();
        assert_eq!(entries.len(), 18);
        // The message commands per family (task0 section 4: Pt 15).
        let params = &l.script.message_params;
        assert_eq!(params.of(Family::Pt).len(), 15);
        assert_eq!(params.of(Family::Hgss).len(), 11);
        assert_eq!(params.of(Family::Dp).len(), 8);
        assert!(params.source.contains("platinum") && params.source.contains("hgss"));
        for e in &entries {
            assert!(!e.source.trim().is_empty(), "{} has no source", e.key);
        }
        // The cached copy is the same.
        assert_eq!(Project::limits(), &l);
    }

    #[test]
    fn jump_params_name_existing_commands() {
        let l: Limits = serde_yaml::from_str(CHECK_LIMITS_YAML).unwrap();
        assert!(l.script.jump_params.source.contains("legacy_name"));
        for family in [Family::Dp, Family::Pt, Family::Hgss] {
            let db = crate::checks::scripts::database(family).unwrap();
            let names = l.jump_commands(family);
            assert!(names.iter().any(|n| n == "Call"), "{family:?}");
            for name in names {
                // rotom v0.7.0: src/database.rs DatabaseV2::get_command (canonical, legacy, alias)
                let command = db
                    .get_command(name)
                    .unwrap_or_else(|_| panic!("{family:?}: {name} is not in the database"));
                // A jump: one parameter is a label or the relative jump of the binary command.
                assert!(
                    command.params.iter().any(|p| {
                        p.param_type == rotom::database::ParamType::Label
                            || p.name == "relative_jump"
                    }),
                    "{family:?}: {name} names no label"
                );
            }
        }
    }
}
