//! Item and Pokémon menu icons as PNG, decoded from the ROM's own archives. Nothing is
//! written: the members come from the unpacked folder when DSPRE made one and from the packed
//! NARC otherwise (it is not unpacked for this). Both icon kinds are read-only in the studio.
//!
//! The item table in `arm9.bin` names the NCGR member (the graphic) and the NCLR member (the
//! palette) of every item; the palette table names palette 0, 1 or 2 of `monIcons` member 0
//! for every Pokémon icon, indexed by the icon (NCGR member − 7), see `docs/formats/ncgr_nclr.md`.

use g4_formats::ncgr::{Ncgr, render_rgba};
use g4_formats::nclr::Nclr;

use crate::error::{KernelError, io_at};
use crate::project::{AssetKind, Project};

/// First NCGR member of `monIcons`: member 0 is the NCLR, 1 to 6 are animation data.
const FIRST_MON_ICON_MEMBER: u16 = 7;
/// Personal members 0..=493 (Bulbasaur to Arceus, and "?" at 0) have their icon at
/// `member + 7`, with the palette entry of the same number.
const PLAIN_SPECIES: u16 = 494;

/// The icon of the personal members 494 to 507, which do not follow `member + 7`: Egg, Bad
/// Egg and the forms of Deoxys, Burmy's evolution, Giratina, Shaymin and Rotom. Each entry is
/// the NCGR member in `monIcons` and the index into the palette table (the palette table is
/// indexed by icon, NCGR member − 7). Bad Egg shows the "?" icon, as the games do for every
/// species above 493. Checked on Platinum DE, HeartGold DE and HeartGold US.
pub const FORM_ICON_MEMBERS: [(u16, usize); 14] = [
    (501, 494), // 494 Egg
    (7, 0),     // 495 Bad Egg
    (503, 496), // 496 Deoxys, Attack
    (504, 497), // 497 Deoxys, Defense
    (505, 498), // 498 Deoxys, Speed
    (536, 529), // 499 Wormadam, Sandy Cloak
    (537, 530), // 500 Wormadam, Trash Cloak
    (540, 533), // 501 Giratina, Origin
    (541, 534), // 502 Shaymin, Sky
    (542, 535), // 503 Rotom, Heat
    (543, 536), // 504 Rotom, Wash
    (544, 537), // 505 Rotom, Frost
    (545, 538), // 506 Rotom, Fan
    (546, 539), // 507 Rotom, Mow
];

/// The NCGR member of `monIcons` and the palette table index of personal member `species`;
/// `None` above 507.
pub fn mon_icon_source(species: u16) -> Option<(u16, usize)> {
    if species < PLAIN_SPECIES {
        return Some((species + FIRST_MON_ICON_MEMBER, usize::from(species)));
    }
    FORM_ICON_MEMBERS
        .get(usize::from(species - PLAIN_SPECIES))
        .copied()
}

/// Encodes `width` x `height` RGBA pixels (8 bit, `width * height * 4` bytes) as a PNG.
pub fn png(rgba: &[u8], width: usize, height: usize) -> Result<Vec<u8>, KernelError> {
    let dimension = |n: usize| {
        u32::try_from(n).map_err(|_| KernelError::Io(std::io::Error::other("image too large")))
    };
    let mut out = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut out, dimension(width)?, dimension(height)?);
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(std::io::Error::from)?;
    writer
        .write_image_data(rgba)
        .map_err(std::io::Error::from)?;
    writer.finish().map_err(std::io::Error::from)?;
    Ok(out)
}

/// The bytes of two members of one archive.
type MemberPair = (Vec<u8>, Vec<u8>);

/// Members `first` and `second` of the icon archive `kind`, or `None` when the archive lacks
/// either. See [`members`].
fn member_pair(
    project: &Project,
    kind: AssetKind,
    first: u16,
    second: u16,
) -> Result<Option<MemberPair>, KernelError> {
    let [first, second] = members(project, kind, [first, second])?;
    Ok(first.zip(second))
}

/// Members `indices` of the archive `kind`, each `None` when the archive lacks it. Reads the
/// unpacked folder if there is one, else the packed NARC (parsed once for all members); never
/// unpacks.
pub(crate) fn members<const N: usize>(
    project: &Project,
    kind: AssetKind,
    indices: [u16; N],
) -> Result<[Option<Vec<u8>>; N], KernelError> {
    let dir = project.unpacked_dir(kind);
    if dir.is_dir() {
        let mut out = [const { None }; N];
        for (slot, index) in out.iter_mut().zip(indices) {
            let path = dir.join(format!("{index:04}"));
            *slot = match std::fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(io_at(&path)(e)),
            };
        }
        return Ok(out);
    }
    let narc = project.read_packed(kind)?;
    Ok(indices.map(|index| narc.members.get(usize::from(index)).cloned()))
}

/// The bag icon of item `item_id` as a 32 x 32 PNG (index 0 of the palette transparent).
/// `None` when the arm9 has no usable item table, the id lies outside it, or the table points
/// at a member the archive does not have. A member that is not an NCGR or NCLR is an error.
pub fn item_icon(project: &Project, item_id: u16) -> Result<Option<Vec<u8>>, KernelError> {
    let Some(entry) = project
        .item_table()
        .and_then(|table| table.entries.get(usize::from(item_id)))
    else {
        return Ok(None);
    };
    let Some((graphic, colours)) =
        member_pair(project, AssetKind::ItemIcons, entry.icon, entry.palette)?
    else {
        return Ok(None);
    };
    let ncgr = Ncgr::read(&graphic)?;
    let nclr = Nclr::read(&colours)?;
    // An item's palette member holds 16 palettes; the item uses the first.
    let Some(palette) = nclr.palettes.first() else {
        return Ok(None);
    };
    png(&render_rgba(&ncgr, palette), ncgr.width, ncgr.height).map(Some)
}

/// The menu icon of personal member `species` (0 to 507, see [`mon_icon_source`]) as a
/// 32 x 32 PNG: the first of the two frames the NCGR holds below each other (the second is
/// the bobbing frame). `None` when the arm9 has no usable palette table, the species has no
/// icon, or the archive lacks the member or the palette. A member that is not an NCGR or
/// NCLR is an error.
pub fn mon_icon(project: &Project, species: u16) -> Result<Option<Vec<u8>>, KernelError> {
    let Some(palettes) = project.mon_icon_palettes() else {
        return Ok(None);
    };
    let Some((graphic, table_index)) = mon_icon_source(species) else {
        return Ok(None);
    };
    let Some(&palette_number) = palettes.values.get(table_index) else {
        return Ok(None);
    };
    let Some((graphic, colours)) = member_pair(project, AssetKind::MonIcons, graphic, 0)? else {
        return Ok(None);
    };
    let ncgr = Ncgr::read(&graphic)?;
    let nclr = Nclr::read(&colours)?;
    let Some(palette) = nclr.palettes.get(usize::from(palette_number)) else {
        return Ok(None);
    };
    let mut rgba = render_rgba(&ncgr, palette);
    let mut height = ncgr.height;
    if height == 2 * ncgr.width {
        height = ncgr.width;
        rgba.truncate(ncgr.width * height * 4);
    }
    png(&rgba, ncgr.width, height).map(Some)
}
