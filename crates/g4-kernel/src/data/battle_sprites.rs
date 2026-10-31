//! Pokémon and trainer battle sprites as PNG, decoded from the ROM's own archives
//! (`pokemonBattleSprites`, `otherPokemonBattleSprites`, `trainerGraphics`). Nothing is
//! written; the members come from the unpacked folder or the packed NARC like the icons
//! (`icons::members`). All three kinds are read-only in the studio.
//!
//! Every image is an encrypted linear NCGR of 160 x 80 pixels at 4 bpp: two 80 x 80 frames
//! side by side. The decryption direction is a property of the archive and the family
//! (`metadata/battle_sprites.yaml`, measured in gutachten C §3.2); a family marked
//! `unverified` (Diamond/Pearl) has no sprites at all, neither an image nor an error.
//!
//! `pokemonBattleSprites` holds six members per species: back female (0), back male or only
//! (1), front female (2), front male or only (3), the normal (4) and the shiny palette (5). An
//! empty member means "no image of its own": the other gender's slot of the same side is shown.
//! `otherPokemonBattleSprites` holds the other forms and the egg in an order of its own: the
//! form table of `battle_sprites.yaml` (per family, checked by eye) names their members.
//! `trainerGraphics` holds five members per trainer class: the image at `5·k + 4`, its 16
//! palettes at `5·k + 1` (palette 0 is the battle palette).

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use g4_formats::game::Family;
use g4_formats::ncgr::{Direction, Ncgr, render_rgba};
use g4_formats::nclr::Nclr;
use serde::{Deserialize, Serialize};

use super::icons::{members, png};
use crate::error::KernelError;
use crate::project::{AssetKind, Project};

const BATTLE_SPRITES_YAML: &str = include_str!("../../metadata/battle_sprites.yaml");

/// Width and height of one frame; an image holds two frames side by side.
pub const FRAME: usize = 80;
/// Members per species in `pokemonBattleSprites`.
pub const SLOTS_PER_SPECIES: u16 = 6;
/// Members per trainer class in `trainerGraphics`.
const SLOTS_PER_CLASS: u16 = 5;
/// The first personal member that is no plain species (the egg; 496 to 507 are forms).
const FIRST_FORM_PERSONAL: u16 = 494;

/// Which side of the Pokémon an image shows: the front faces the player (the opponent's view),
/// the back is the player's own Pokémon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Front,
    Back,
}

/// One battle sprite of a Pokémon. `form` is the game's form index (0 is the base form);
/// `frame` 0 is the left, 1 the right (animation) half of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpriteRequest {
    pub species: u16,
    pub form: u8,
    pub side: Side,
    pub female: bool,
    pub shiny: bool,
    pub frame: u8,
}

/// The decryption direction of each battle sprite archive of one family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Directions {
    pub pokemon: Direction,
    pub other: Direction,
    pub trainer: Direction,
}

/// One entry of the form table: the members of a form's images in `otherPokemonBattleSprites`.
/// `personal` is the personal member of the form (496 to 507, the egg 494), if it has one.
/// A missing image means "no image of this side" (the egg has no back); a missing female
/// image shows the male one; without `shiny` (the egg) the normal palette is used.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormSprite {
    pub species: u16,
    pub form: u8,
    pub personal: Option<u16>,
    pub back: Option<u16>,
    pub front: Option<u16>,
    pub female_back: Option<u16>,
    pub female_front: Option<u16>,
    pub palette: u16,
    pub shiny: Option<u16>,
    /// Not checked by eye: the base form is shown instead.
    #[serde(default)]
    pub unverified: bool,
}

/// A direction as `battle_sprites.yaml` spells it (g4-formats has no serde).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawDirection {
    Forward,
    Backward,
}

impl From<RawDirection> for Direction {
    fn from(raw: RawDirection) -> Direction {
        match raw {
            RawDirection::Forward => Direction::Forward,
            RawDirection::Backward => Direction::Backward,
        }
    }
}

/// One family's entry of `direction` in `battle_sprites.yaml`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDirections {
    #[serde(default)]
    unverified: bool,
    pokemon: RawDirection,
    other: RawDirection,
    trainer: RawDirection,
}

/// `battle_sprites.yaml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    /// Keyed by family: `dp`, `pt`, `hgss`.
    direction: HashMap<String, RawDirections>,
    /// Keyed by family like `direction`; a family without a table has no forms.
    #[serde(default)]
    forms: HashMap<String, Vec<FormSprite>>,
}

/// The parsed `battle_sprites.yaml`, read once per process.
fn metadata() -> &'static Metadata {
    static METADATA: OnceLock<Metadata> = OnceLock::new();
    // The file is embedded and parsed by a test, so a parse error cannot reach a user.
    METADATA.get_or_init(|| serde_yaml::from_str(BATTLE_SPRITES_YAML).unwrap_or_default())
}

/// The key of `family` in `battle_sprites.yaml`.
fn family_key(family: Family) -> &'static str {
    match family {
        Family::Dp => "dp",
        Family::Pt => "pt",
        Family::Hgss => "hgss",
    }
}

/// The decryption directions of `family`, from `battle_sprites.yaml`; `None` for a family
/// marked `unverified` (Diamond/Pearl).
pub fn directions(family: Family) -> Option<Directions> {
    let raw = metadata().direction.get(family_key(family))?;
    (!raw.unverified).then(|| Directions {
        pokemon: raw.pokemon.into(),
        other: raw.other.into(),
        trainer: raw.trainer.into(),
    })
}

/// The form table of `family`, unverified entries included (empty for Diamond/Pearl).
pub fn forms(family: Family) -> &'static [FormSprite] {
    forms_in(metadata(), family)
}

fn forms_in(metadata: &Metadata, family: Family) -> &[FormSprite] {
    metadata
        .forms
        .get(family_key(family))
        .map_or(&[], Vec::as_slice)
}

/// The verified form table entry of `form` of `species` in `family`, if there is one. An
/// entry marked `unverified` is not given: the base form is shown instead.
pub fn form_sprite(family: Family, species: u16, form: u8) -> Option<&'static FormSprite> {
    verified(forms(family), species, form)
}

fn verified(table: &[FormSprite], species: u16, form: u8) -> Option<&FormSprite> {
    table
        .iter()
        .find(|f| !f.unverified && f.species == species && f.form == form)
}

/// The species and form of personal member `personal` (496 → Deoxys, form 1; the egg 494 →
/// species 494, form 0), from the form tables; `None` below 494 or for a member no table
/// names. The same in every family (`species_forms.yaml`), so no family is asked. Unverified
/// entries count too: what is unverified is the images, not which form a personal member is,
/// and [`form_sprite`] then shows the base form of the species.
pub fn form_of_personal(personal: u16) -> Option<(u16, u8)> {
    if personal < FIRST_FORM_PERSONAL {
        return None;
    }
    metadata()
        .forms
        .values()
        .find_map(|table| table.iter().find(|f| f.personal == Some(personal)))
        .map(|f| (f.species, f.form))
}

/// Member `6·species + slot` of `pokemonBattleSprites`: (Back, female) 0, (Back, male) 1,
/// (Front, female) 2, (Front, male) 3. Saturates for a species beyond any archive.
pub fn pokemon_member(species: u16, side: Side, female: bool) -> u16 {
    let slot = match (side, female) {
        (Side::Back, true) => 0,
        (Side::Back, false) => 1,
        (Side::Front, true) => 2,
        (Side::Front, false) => 3,
    };
    species
        .saturating_mul(SLOTS_PER_SPECIES)
        .saturating_add(slot)
}

/// Whether a species has images of its own in the female slots (0 and 2) and in the male or
/// only slots (1 and 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpriteSlots {
    pub female: bool,
    pub male: bool,
}

/// Whether `species` (a personal member) has images of its own per gender. A member of 494
/// and above is resolved like [`battle_sprite`] resolves it: a verified form table entry
/// gives the slots of its images in `otherPokemonBattleSprites` (no form has female images,
/// so a form with an image is male only); a form without an image there shows its base form,
/// whose slots are given. `None` when the family is unverified, the project has no archive to
/// draw from, or the archive has no member of the species.
pub fn sprite_slots(project: &Project, species: u16) -> Result<Option<SpriteSlots>, KernelError> {
    if directions(project.family).is_none() {
        return Ok(None);
    }
    let (species, form) = form_of_personal(species).unwrap_or((species, 0));
    if let Some(entry) = form_sprite(project.family, species, form) {
        let slots = form_slots(project, entry)?;
        // The egg has no base form: its images are only in the form archive.
        if slots.is_some() || species >= FIRST_FORM_PERSONAL {
            return Ok(slots);
        }
    }
    base_slots(project, species)
}

/// The slots of `species` in `pokemonBattleSprites` (members `6·species` to `6·species + 3`).
fn base_slots(project: &Project, species: u16) -> Result<Option<SpriteSlots>, KernelError> {
    let kind = AssetKind::PokemonBattleSprites;
    if !has_archive_data(project, kind) {
        return Ok(None);
    }
    let Some(first) = first_member(species) else {
        return Ok(None);
    };
    let slots = members(project, kind, [first, first + 1, first + 2, first + 3])?;
    if slots.iter().all(Option::is_none) {
        return Ok(None);
    }
    let filled = |slot: usize| slots[slot].as_ref().is_some_and(|b| !b.is_empty());
    Ok(Some(SpriteSlots {
        female: filled(0) || filled(2),
        male: filled(1) || filled(3),
    }))
}

/// The slots of form `entry` in `otherPokemonBattleSprites`: male (or only) from `back` and
/// `front`, female from `female_back` and `female_front`. `None` when the archive is missing
/// or none of the entry's images is a non-empty member.
fn form_slots(project: &Project, entry: &FormSprite) -> Result<Option<SpriteSlots>, KernelError> {
    let kind = AssetKind::OtherPokemonBattleSprites;
    if !has_archive_data(project, kind) {
        return Ok(None);
    }
    let named = [
        entry.back,
        entry.front,
        entry.female_back,
        entry.female_front,
    ];
    // An image the entry does not name is asked as the last member index and never counted.
    let read = members(project, kind, named.map(|m| m.unwrap_or(u16::MAX)))?;
    let filled = |i: usize| named[i].is_some() && read[i].as_ref().is_some_and(|b| !b.is_empty());
    let male = filled(0) || filled(1);
    let female = filled(2) || filled(3);
    Ok((male || female).then_some(SpriteSlots { female, male }))
}

/// The requested half of a Pokémon's battle sprite as an 80 x 80 PNG (palette index 0
/// transparent); an image of another size than 160 x 80 is given whole.
///
/// A species of 494 and above is a personal member: it is resolved to its species and form by
/// [`form_of_personal`] (503 is Rotom in form 1, the egg 494 stays itself), and the request's
/// own `form` is ignored. A species and form with a verified entry in the form table of the
/// family ([`form_sprite`]) is drawn from `otherPokemonBattleSprites`; every other form, and a
/// form whose image cannot be read there (no archive, no member, no palette), shows the base
/// form of `pokemonBattleSprites` (the egg has none). There an empty slot falls back to the
/// other gender's slot of the same side.
///
/// `None` when the family is unverified, the frame is neither 0 nor 1, both slots are empty,
/// a side has no image (the egg's back), or the archive lacks the species or its palette. A
/// member that is not an NCGR or NCLR is an error.
pub fn battle_sprite(
    project: &Project,
    req: SpriteRequest,
) -> Result<Option<Vec<u8>>, KernelError> {
    let Some(dirs) = directions(project.family) else {
        return Ok(None);
    };
    if req.frame > 1 {
        return Ok(None);
    }
    let (species, form) = match form_of_personal(req.species) {
        Some(resolved) => resolved,
        None => (req.species, req.form),
    };
    if let Some(entry) = form_sprite(project.family, species, form) {
        let png = form_image(project, entry, req, dirs.other)?;
        // The egg has no base form: its images are only in the form archive.
        if png.is_some() || species >= FIRST_FORM_PERSONAL {
            return Ok(png);
        }
    }
    base_image(project, species, req, dirs.pokemon)
}

/// The image of `req` in `pokemonBattleSprites` (members `6·species + slot`).
fn base_image(
    project: &Project,
    species: u16,
    req: SpriteRequest,
    direction: Direction,
) -> Result<Option<Vec<u8>>, KernelError> {
    let kind = AssetKind::PokemonBattleSprites;
    if !has_archive_data(project, kind) {
        return Ok(None);
    }
    let Some(first) = first_member(species) else {
        return Ok(None);
    };
    let own = pokemon_member(species, req.side, req.female);
    let other = pokemon_member(species, req.side, !req.female);
    let palette = first + 4 + u16::from(req.shiny);
    let [own, other, colours] = members(project, kind, [own, other, palette])?;
    let Some(graphic) = [own, other].into_iter().flatten().find(|b| !b.is_empty()) else {
        return Ok(None);
    };
    render(&graphic, colours.as_deref(), direction, req.frame)
}

/// The image of `req` as form `entry` in `otherPokemonBattleSprites`; `None` when the side
/// has no image or the archive lacks the member or the palette.
fn form_image(
    project: &Project,
    entry: &FormSprite,
    req: SpriteRequest,
    direction: Direction,
) -> Result<Option<Vec<u8>>, KernelError> {
    let kind = AssetKind::OtherPokemonBattleSprites;
    let (image, female_image) = match req.side {
        Side::Front => (entry.front, entry.female_front),
        Side::Back => (entry.back, entry.female_back),
    };
    let image = if req.female {
        female_image.or(image)
    } else {
        image
    };
    let Some(image) = image else {
        return Ok(None);
    };
    if !has_archive_data(project, kind) {
        return Ok(None);
    }
    let palette = match entry.shiny {
        Some(shiny) if req.shiny => shiny,
        _ => entry.palette,
    };
    let [graphic, colours] = members(project, kind, [image, palette])?;
    let Some(graphic) = graphic.filter(|b| !b.is_empty()) else {
        return Ok(None);
    };
    render(&graphic, colours.as_deref(), direction, req.frame)
}

/// The battle image of trainer class `class` as an 80 x 80 PNG (the left half of member
/// `5·k + 4`, with palette 0 of member `5·k + 1`). `None` when the family is unverified or
/// the archive has no such class.
pub fn trainer_sprite(project: &Project, class: u8) -> Result<Option<Vec<u8>>, KernelError> {
    let kind = AssetKind::TrainerGraphics;
    let Some(dirs) = directions(project.family) else {
        return Ok(None);
    };
    if !has_archive_data(project, kind) {
        return Ok(None);
    }
    let first = SLOTS_PER_CLASS * u16::from(class);
    let [graphic, colours] = members(project, kind, [first + 4, first + 1])?;
    let Some(graphic) = graphic.filter(|b| !b.is_empty()) else {
        return Ok(None);
    };
    render(&graphic, colours.as_deref(), dirs.trainer, 0)
}

/// The first member of `species` in `pokemonBattleSprites`; `None` when its six members would
/// lie beyond the largest member index.
fn first_member(species: u16) -> Option<u16> {
    let first = species.checked_mul(SLOTS_PER_SPECIES)?;
    first.checked_add(SLOTS_PER_SPECIES - 1)?;
    Some(first)
}

/// Whether the project has the archive of `kind`, unpacked or packed.
fn has_archive_data(project: &Project, kind: AssetKind) -> bool {
    project.unpacked_dir(kind).is_dir()
        || project
            .archives
            .packed(kind)
            .is_some_and(|packed| project.root.join(packed).is_file())
}

/// Decrypts `graphic`, draws it with the first palette of `colours` and cuts out `frame`.
/// `None` without a palette or for an image without pixels.
fn render(
    graphic: &[u8],
    colours: Option<&[u8]>,
    direction: Direction,
    frame: u8,
) -> Result<Option<Vec<u8>>, KernelError> {
    let mut ncgr = Ncgr::read(graphic)?;
    let Some(colours) = colours.filter(|b| !b.is_empty()) else {
        return Ok(None);
    };
    let nclr = Nclr::read(colours)?;
    let Some(palette) = nclr.palettes.first() else {
        return Ok(None);
    };
    if ncgr.width == 0 || ncgr.height == 0 {
        return Ok(None);
    }
    ncgr.decrypt(direction);
    let rgba = render_rgba(&ncgr, palette);
    if (ncgr.width, ncgr.height) != (2 * FRAME, FRAME) {
        return png(&rgba, ncgr.width, ncgr.height).map(Some);
    }
    png(&crop(&rgba, ncgr.width, usize::from(frame)), FRAME, FRAME).map(Some)
}

/// Frame `frame` (`FRAME` x `FRAME` pixels at `frame · FRAME`) of an RGBA image `width`
/// pixels wide and `FRAME` high.
fn crop(rgba: &[u8], width: usize, frame: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(FRAME * FRAME * 4);
    for row in rgba.chunks_exact(width * 4).take(FRAME) {
        let start = frame * FRAME * 4;
        out.extend_from_slice(&row[start..start + FRAME * 4]);
    }
    out
}

/// Decoded battle sprites of one session, as PNG. An image (or "no image") is read once; an
/// error is not remembered.
#[derive(Default)]
pub struct BattleSpriteCache {
    sprites: HashMap<SpriteRequest, Option<Arc<Vec<u8>>>>,
    trainers: HashMap<u8, Option<Arc<Vec<u8>>>>,
}

impl BattleSpriteCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// [`battle_sprite`], read at most once per request.
    pub fn battle_sprite(
        &mut self,
        project: &Project,
        req: SpriteRequest,
    ) -> Result<Option<Arc<Vec<u8>>>, KernelError> {
        if let Some(cached) = self.sprites.get(&req) {
            return Ok(cached.clone());
        }
        let png = battle_sprite(project, req)?.map(Arc::new);
        self.sprites.insert(req, png.clone());
        Ok(png)
    }

    /// [`trainer_sprite`], read at most once per class.
    pub fn trainer_sprite(
        &mut self,
        project: &Project,
        class: u8,
    ) -> Result<Option<Arc<Vec<u8>>>, KernelError> {
        if let Some(cached) = self.trainers.get(&class) {
            return Ok(cached.clone());
        }
        let png = trainer_sprite(project, class)?.map(Arc::new);
        self.trainers.insert(class, png.clone());
        Ok(png)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn battle_sprites_yaml_parses_with_three_families() {
        let parsed: Metadata = serde_yaml::from_str(BATTLE_SPRITES_YAML).unwrap();
        let mut keys: Vec<&str> = parsed.direction.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["dp", "hgss", "pt"]);
        assert!(parsed.direction["dp"].unverified);
    }

    /// A form entry with a personal member 496 to 507 belongs to the species
    /// `species_forms.yaml` names as that member's base.
    #[derive(Deserialize)]
    struct BaseOf {
        base: u16,
    }

    #[test]
    fn form_entries_agree_with_species_forms() {
        let bases: BTreeMap<u16, BaseOf> =
            serde_yaml::from_str(include_str!("../../metadata/species_forms.yaml")).unwrap();
        let parsed: Metadata = serde_yaml::from_str(BATTLE_SPRITES_YAML).unwrap();
        let mut families: Vec<&str> = parsed.forms.keys().map(String::as_str).collect();
        families.sort_unstable();
        assert_eq!(families, ["hgss", "pt"]);
        for form in parsed.forms.values().flatten() {
            if let Some(base) = form.personal.and_then(|p| bases.get(&p)) {
                assert_eq!(form.species, base.base, "{form:?}");
            }
        }
        // Every personal member 496 to 507 is named, in every family alike.
        for personal in bases.keys() {
            for table in parsed.forms.values() {
                assert_eq!(
                    table
                        .iter()
                        .filter(|f| f.personal == Some(*personal))
                        .count(),
                    1,
                    "{personal}"
                );
            }
        }
        let personal_of = |family: &str| -> Vec<(Option<u16>, u16, u8)> {
            let mut out: Vec<_> = parsed.forms[family]
                .iter()
                .filter(|f| f.personal.is_some())
                .map(|f| (f.personal, f.species, f.form))
                .collect();
            out.sort_unstable();
            out
        };
        assert_eq!(personal_of("pt"), personal_of("hgss"));
    }

    #[test]
    fn form_of_personal_is_none_below_the_egg() {
        assert_eq!(form_of_personal(0), None);
        assert_eq!(form_of_personal(493), None);
        assert_eq!(form_sprite(Family::Pt, 0, 0), None);
    }

    /// An entry marked `unverified` is in the table but never given out: the base form is
    /// shown (the real table has none, so a table of its own is read).
    #[test]
    fn an_unverified_form_falls_back_too() {
        let parsed: Metadata = serde_yaml::from_str(
            "direction: {}\n\
             forms:\n  \
               pt:\n    \
                 - { species: 201, form: 1, back: 10, front: 11, palette: 156, shiny: 157, unverified: true }\n    \
                 - { species: 201, form: 2, back: 12, front: 13, palette: 156, shiny: 157 }\n",
        )
        .unwrap();
        let table = forms_in(&parsed, Family::Pt);
        assert_eq!(table.len(), 2);
        assert_eq!(verified(table, 201, 1), None);
        assert_eq!(verified(table, 201, 2).map(|f| f.front), Some(Some(13)));
        assert!(forms_in(&parsed, Family::Hgss).is_empty());
    }

    #[test]
    fn first_member_never_overflows() {
        assert_eq!(first_member(1), Some(6));
        assert_eq!(first_member(u16::MAX / 6), None);
        assert_eq!(first_member(u16::MAX), None);
        assert_eq!(pokemon_member(u16::MAX, Side::Front, false), u16::MAX);
    }

    #[test]
    fn crop_takes_one_frame_per_row() {
        let width = 2 * FRAME;
        let rgba: Vec<u8> = (0..width * FRAME)
            .flat_map(|i| [u8::from(i % width >= FRAME); 4])
            .collect();
        assert!(crop(&rgba, width, 0).iter().all(|&b| b == 0));
        assert!(crop(&rgba, width, 1).iter().all(|&b| b == 1));
        assert_eq!(crop(&rgba, width, 1).len(), FRAME * FRAME * 4);
    }
}
