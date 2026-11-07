//! The textures of the map texture packs: what the baker binds, the VRAM of an area and
//! „Textur an Paket anhängen“ as one undo file step.
//!
//! A map texture pack is shared by every area whose `map_tileset` names it, so appending
//! judges the VRAM (map pack + buildings pack) of each of them. Every check runs before the
//! member is written; a refusal is [`KernelError::Refused`] with `<code>: <message>`.

use std::collections::HashSet;

use g4_formats::error::FormatError;
use g4_formats::game::Family;
use g4_formats::nitro_dict::{self, MAX_ENTRIES, Name};
use g4_formats::nsbtx::{self, Btx0, NewTexture, Nsbtx, PALETTE_BANK, TEXEL_BANK};
use g4_formats::texconv::palette_name;

use super::{
    AreaStyle, BakeCode, BakeInput, BakeMessage, BakeReport, Face, PackTexture, bake,
    vanilla_vram_max,
};
use crate::error::{KernelError, io_at};
use crate::map_import::map_areas;
use crate::project::{AssetId, AssetKind, Project};
use crate::store::{FileChange, FileStep, FileStepKind, Store, write_file_step};

/// Textures of a pack with their palette: the palette named like the texture, else
/// `palette_name(name)`, else the only palette of the pack; a texture without one is left
/// out (bake reports it missing).
pub fn pack_textures(pack: &[u8]) -> Result<Vec<PackTexture>, FormatError> {
    let btx = Nsbtx::read(pack)?;
    let palettes: Vec<&str> = btx.palettes.iter().map(|p| p.name.as_str()).collect();
    let only = match palettes.as_slice() {
        [only] => Some(*only),
        _ => None,
    };
    Ok(btx
        .textures
        .iter()
        .filter_map(|t| {
            let suffixed = palette_name(&t.name);
            let palette = palettes
                .iter()
                .copied()
                .find(|&p| p == t.name)
                .or_else(|| palettes.iter().copied().find(|&p| p == suffixed))
                .or(only)?;
            Some(PackTexture {
                name: t.name.clone(),
                palette: palette.to_string(),
                width: u16::try_from(t.width).ok()?,
                height: u16::try_from(t.height).ok()?,
            })
        })
        .collect())
}

/// Magic of a texture pack; a building member without it is a dummy (see [`area_vram`]).
const BTX0_MAGIC: &[u8] = b"BTX0";

/// Bytes `pack` takes in VRAM: texels, 4x4 texels, 4x4 index and palettes.
pub fn pack_vram(pack: &[u8]) -> Result<usize, FormatError> {
    Ok(Btx0::read(pack)?.vram_bytes())
}

/// VRAM of area `area`: its map pack (`map_pack` when given, else the member on disk) plus
/// its building pack; a pack outside its archive counts 0, and so does a building member that
/// is no BTX0 (HeartGold/SoulSilver keep 4-byte dummies there, the game loads no textures
/// from them). A map pack must read. The area data as the store holds it; an area outside
/// the archive is refused.
pub fn area_vram(
    store: &mut Store,
    area: u8,
    map_pack: Option<&[u8]>,
) -> Result<usize, KernelError> {
    let data = area_data(store, area)?;
    let project = store.project();
    let map = match map_pack {
        Some(bytes) => pack_vram(bytes)?,
        None => match project.map_tileset_member(data.map_tileset())? {
            Some(bytes) => pack_vram(&bytes)?,
            None => 0,
        },
    };
    let buildings = match project.building_tileset_member(data.buildings_tileset())? {
        Some(bytes) if bytes.starts_with(BTX0_MAGIC) => pack_vram(&bytes)?,
        _ => 0,
    };
    Ok(map + buildings)
}

/// The area data whose `map_tileset` is `pack`, as the store holds them.
pub fn areas_of_pack(store: &mut Store, pack: u16) -> Result<Vec<u8>, KernelError> {
    let mut areas = Vec::new();
    for area in 0..=u8::MAX {
        match store.map_tileset_index(area)? {
            Some(tileset) if tileset == pack => areas.push(area),
            Some(_) => {}
            // The area data are numbered without gaps: the first missing one ends them.
            None => break,
        }
    }
    Ok(areas)
}

fn area_data(store: &mut Store, area: u8) -> Result<g4_formats::area_data::AreaData, KernelError> {
    store
        .area_data_checked(area)?
        .ok_or_else(|| KernelError::Refused(format!("area data {area} is not in the archive")))
}

/// What [`append_textures`] did: the pack, the names appended (in order), the member's size
/// before and after, `vram_bytes` = [`pack_vram`] of the pack after appending (the pack
/// alone: it is shared by several areas, unlike [`BakeReport::vram_bytes`], which is one
/// area's map pack plus buildings pack), and the warnings: `bake.vram-high` once per area of
/// the pack whose [`area_vram`] with the new pack is above the vanilla maximum, args `[area,
/// area VRAM, maximum]` (`bake`'s own `bake.vram-high` has no area: `[area VRAM, maximum]`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppendReport {
    pub pack: u16,
    pub appended: Vec<String>,
    pub size_before: usize,
    pub size_after: usize,
    pub vram_bytes: usize,
    pub messages: Vec<BakeMessage>,
}

/// The label of appending `count` textures to map texture pack `pack`:
/// „Kartentexturen 0020: 2 Texturen angehängt“.
pub fn append_label(pack: u16, count: usize) -> String {
    let noun = if count == 1 { "Textur" } else { "Texturen" };
    format!("Kartentexturen {pack:04}: {count} {noun} angehängt")
}

/// Appends `textures` to map texture pack `pack` as one file step labelled `label`; every
/// check before anything is written (errors: `KernelError::Refused("<code>: <message>")`);
/// Diamond/Pearl refused.
///
/// The codes: `texture.side` (a side no power of two 8..=1024), `texture.name` (a texture or
/// palette name `nitro_dict::name` refuses), `texture.taken` (a texture or palette name in the
/// pack or twice in the request), `texture.pack-full` (over 255 entries, texture data over
/// [`TEXEL_BANK`], palettes over [`PALETTE_BANK`], an offset past `0xFFFF · 8`). A format
/// other than 2 and 3 or a wrong texel or colour count is the caller's error
/// ([`KernelError::Format`]). No texture: nothing is written or recorded.
pub fn append_textures(
    store: &mut Store,
    pack: u16,
    textures: &[NewTexture],
    label: &str,
) -> Result<AppendReport, KernelError> {
    if store.project().family == Family::Dp {
        return Err(KernelError::Refused(
            "Diamond/Pearl: textures are not appended without a test project".into(),
        ));
    }
    if let Some(why) = store.busy() {
        return Err(KernelError::Busy(why.to_owned()));
    }
    store.project().editable()?;
    let before = current_pack(store.project(), pack)?.ok_or_else(|| {
        KernelError::Refused(format!("map texture pack {pack:04} is not in the archive"))
    })?;
    let btx = appended(Btx0::read(&before)?, textures)?.map_err(refusal)?;
    let after = nsbtx::write(&btx)?;
    let mut messages = Vec::new();
    let limit = vanilla_vram_max(store.project().family);
    for area in areas_of_pack(store, pack)? {
        let vram = area_vram(store, area, Some(&after))?;
        if vram > limit {
            messages.push(BakeMessage::warning(
                BakeCode::VramHigh,
                &[&area, &vram, &limit],
                format!(
                    "area {area} takes {vram} bytes of VRAM with map texture pack {pack:04}; \
                     the largest vanilla area takes {limit}"
                ),
            ));
        }
    }
    let report = AppendReport {
        pack,
        appended: textures.iter().map(|t| t.name.clone()).collect(),
        size_before: before.len(),
        size_after: after.len(),
        vram_bytes: btx.vram_bytes(),
        messages,
    };
    if textures.is_empty() {
        return Ok(report);
    }
    let project = store.project();
    let step = FileStep {
        asset: AssetId {
            kind: AssetKind::MapTexture,
            index: pack,
        },
        files: vec![FileChange {
            path: project.member_path(AssetKind::MapTexture, pack),
            before: Some(before),
            after: Some(after),
        }],
        rotom_entry: None,
        kind: FileStepKind::Reset,
    };
    project.ensure_unpacked(AssetKind::MapTexture)?;
    write_file_step(project, &step)?;
    store.record_file_step(label, step);
    Ok(report)
}

/// The pack `before` with `textures` appended, as [`append_textures`] would write it, or its
/// refusal; nothing is written (for callers that judge the result before appending).
pub(crate) fn appended_pack(
    before: &[u8],
    textures: &[NewTexture],
) -> Result<Vec<u8>, KernelError> {
    appended_pack_or_refusal(before, textures)?.map_err(refusal)
}

/// The pack `before` with `textures` appended, or the refusal as a message (texture.side,
/// .name, .taken, .pack-full); the outer error is a pack that cannot be read (or a texture the
/// caller built wrongly: a format other than 2 and 3, a wrong texel or colour count).
pub(crate) fn appended_pack_or_refusal(
    before: &[u8],
    textures: &[NewTexture],
) -> Result<Result<Vec<u8>, BakeMessage>, KernelError> {
    match appended(Btx0::read(before)?, textures)? {
        Ok(pack) => Ok(Ok(nsbtx::write(&pack)?)),
        Err(m) => Ok(Err(m)),
    }
}

/// Member `pack` of the map texture archive as it is now: its file, or the member of the
/// packed archive while the folder is not unpacked yet (nothing is written before the
/// checks). `None`: no such member.
fn current_pack(project: &Project, pack: u16) -> Result<Option<Vec<u8>>, KernelError> {
    let kind = AssetKind::MapTexture;
    if project.unpacked_dir(kind).is_dir() {
        if usize::from(pack) >= project.member_count(kind)? {
            return Ok(None);
        }
        let path = project.member_path(kind, pack);
        return std::fs::read(&path).map(Some).map_err(io_at(&path));
    }
    Ok(project
        .read_packed(kind)?
        .members
        .into_iter()
        .nth(usize::from(pack)))
}

/// A refusal of appending as the error of [`append_textures`]: `<code>: <message>`.
fn refusal(m: BakeMessage) -> KernelError {
    KernelError::Refused(format!("{}: {}", m.code.as_str(), m.message))
}

fn pack_full(what: &str, value: usize, max: usize) -> BakeMessage {
    BakeMessage::error(
        BakeCode::PackFull,
        &[&what, &value, &max],
        format!("the map texture pack is full: {what} {value}, at most {max}"),
    )
}

/// `pack` with `textures` pushed, or the first refusal: per texture its sides, its names,
/// whether they are taken; then the entries, the offsets and the banks. The outer error is a
/// texture the caller built wrongly.
fn appended(
    mut pack: Btx0,
    textures: &[NewTexture],
) -> Result<Result<Btx0, BakeMessage>, KernelError> {
    let in_pack: HashSet<Name> = pack.textures.iter().map(|e| e.name).collect();
    let palettes_in_pack: HashSet<Name> = pack.palettes.iter().map(|e| e.name).collect();
    let (mut names, mut palettes) = (HashSet::new(), HashSet::new());
    for t in textures {
        for side in [t.width, t.height] {
            if !(side.is_power_of_two() && (8..=1024).contains(&side)) {
                return Ok(Err(BakeMessage::error(
                    BakeCode::TextureSide,
                    &[&t.name, &t.width, &t.height],
                    format!(
                        "texture {:?} is {} × {} texels; each side must be a power of two \
                         from 8 to 1024",
                        t.name, t.width, t.height
                    ),
                )));
            }
        }
        let name = match nitro_name(&t.name) {
            Ok(name) => name,
            Err(m) => return Ok(Err(m)),
        };
        let palette = match nitro_name(&t.palette) {
            Ok(name) => name,
            Err(m) => return Ok(Err(m)),
        };
        for (text, name, have, asked, what) in [
            (&t.name, name, &in_pack, &mut names, "texture"),
            (
                &t.palette,
                palette,
                &palettes_in_pack,
                &mut palettes,
                "palette",
            ),
        ] {
            let place = if have.contains(&name) {
                "is already in the pack"
            } else if !asked.insert(name) {
                "is asked for twice"
            } else {
                continue;
            };
            return Ok(Err(BakeMessage::error(
                BakeCode::TextureTaken,
                &[&what, text],
                format!("the {what} name {text:?} {place}"),
            )));
        }
    }
    let entries = pack.textures.len().max(pack.palettes.len()) + textures.len();
    if entries > MAX_ENTRIES {
        return Ok(Err(pack_full("entries", entries, MAX_ENTRIES)));
    }
    for t in textures {
        match pack.push(t) {
            Ok(()) => {}
            Err(FormatError::OutOfRange { what, value, max })
                if what == "texel offset" || what == "palette offset" =>
            {
                return Ok(Err(pack_full(what, value, max)));
            }
            Err(e) => return Err(KernelError::Format(e)),
        }
    }
    let texture_data = pack.texels.len() + pack.compressed.len() + pack.compressed_index.len();
    if texture_data > TEXEL_BANK {
        return Ok(Err(pack_full("texture bytes", texture_data, TEXEL_BANK)));
    }
    if pack.palette_data.len() > PALETTE_BANK {
        return Ok(Err(pack_full(
            "palette bytes",
            pack.palette_data.len(),
            PALETTE_BANK,
        )));
    }
    Ok(Ok(pack))
}

/// `text` as a Nitro name, or `texture.name`.
fn nitro_name(text: &str) -> Result<Name, BakeMessage> {
    nitro_dict::name(text).map_err(|e| {
        BakeMessage::error(
            BakeCode::TextureName,
            &[&text],
            format!("{text:?} cannot be a name in the pack: {e}"),
        )
    })
}

/// What `bake` gets for a map: the first area that shows it, that area's map texture pack,
/// style, pack textures ([`pack_textures`]), VRAM ([`area_vram`]) and the vanilla maximum.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BakeContext {
    pub area: u8,
    pub pack: u16,
    pub style: AreaStyle,
    pub textures: Vec<PackTexture>,
    pub vram_bytes: usize,
    pub vram_limit: usize,
}

impl BakeContext {
    pub fn input(&self) -> BakeInput<'_> {
        BakeInput {
            style: self.style,
            textures: &self.textures,
            vram_bytes: self.vram_bytes,
            vram_limit: self.vram_limit,
        }
    }
}

/// The [`BakeContext`] of map `map`; refused when no header shows the map or its area is not
/// in the archive. A pack outside its archive has no textures.
pub fn bake_context(store: &mut Store, map: u16) -> Result<BakeContext, KernelError> {
    let Some(&area) = map_areas(store, map)?.first() else {
        return Err(KernelError::Refused(format!(
            "no map header shows map {map:04}"
        )));
    };
    let data = area_data(store, area)?;
    let pack = data.map_tileset();
    let bytes = store.project().map_tileset_member(pack)?;
    let textures = match &bytes {
        Some(b) => pack_textures(b)?,
        None => Vec::new(),
    };
    let vram_bytes = area_vram(store, area, bytes.as_deref())?;
    Ok(BakeContext {
        area,
        pack,
        style: AreaStyle::of(&data),
        textures,
        vram_bytes,
        vram_limit: vanilla_vram_max(store.project().family),
    })
}

/// `bake` for map `map`: style, pack textures and VRAM of the first area that shows the map.
pub fn bake_for_map(
    store: &mut Store,
    map: u16,
    faces: &[Face],
) -> Result<BakeReport, KernelError> {
    Ok(bake(faces, &bake_context(store, map)?.input()))
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use std::sync::Arc;

    use g4_formats::area_data::AreaData;
    use g4_formats::nsbtx;
    use g4_formats::texconv::palette_name;

    use super::*;
    use crate::bake::{BakeCode, Level, VANILLA_VRAM_MAX_PT};
    use crate::project::{AssetId, AssetKind, Project};
    use crate::test_support::{Synthetic, TextFixture, synthetic_project};

    /// A `width` × `height` texture of `format` (2: 4 colours, 3: 16) with palette
    /// `palette_name(name)`.
    fn tex_wh(name: &str, width: u32, height: u32, format: u8) -> NewTexture {
        let (colors, bits) = if format == 2 { (4u16, 2) } else { (16, 4) };
        NewTexture {
            name: name.to_string(),
            palette: palette_name(name),
            width,
            height,
            format,
            color0: false,
            texels: vec![0x5A; (width * height * bits / 8) as usize],
            colors: (0..colors).map(|c| c * 0x421).collect(),
        }
    }

    fn tex(name: &str, side: u32, format: u8) -> NewTexture {
        tex_wh(name, side, side, format)
    }

    /// Map texture pack 0: `gras` 16 × 16, `weg` 8 × 8 four colours.
    fn map_pack() -> Vec<u8> {
        nsbtx::build(&[tex("gras", 16, 3), tex("weg", 8, 2)]).unwrap()
    }

    /// Building texture pack 0: `haus` 32 × 32.
    fn building_pack() -> Vec<u8> {
        nsbtx::build(&[tex("haus", 32, 3)]).unwrap()
    }

    /// Writes [`map_pack`] and [`building_pack`] as member 0 of their archives (area data 0
    /// shows both).
    fn prepare(project: &Project) {
        project.ensure_unpacked(AssetKind::MapTexture).unwrap();
        project.ensure_unpacked(AssetKind::BuildingTexture).unwrap();
        let write = |kind, bytes: Vec<u8>| std::fs::write(project.member_path(kind, 0), bytes);
        write(AssetKind::MapTexture, map_pack()).unwrap();
        write(AssetKind::BuildingTexture, building_pack()).unwrap();
    }

    struct Fixture {
        _synth: Synthetic,
        store: Store,
    }

    impl Fixture {
        fn member(&self, kind: AssetKind, index: u16) -> Vec<u8> {
            std::fs::read(self.store.project().member_path(kind, index)).unwrap()
        }

        fn write_member(&self, kind: AssetKind, index: u16, bytes: &[u8]) {
            std::fs::write(self.store.project().member_path(kind, index), bytes).unwrap();
        }

        fn pack(&self) -> Vec<u8> {
            self.member(AssetKind::MapTexture, 0)
        }
    }

    fn fixture() -> Fixture {
        let synth = synthetic_project();
        let project = Project::open(synth.dir.path()).unwrap();
        prepare(&project);
        Fixture {
            _synth: synth,
            store: Store::new(Arc::new(project)),
        }
    }

    /// The code of a refusal `<code>: <message>`.
    fn code_of(err: KernelError) -> String {
        match err {
            KernelError::Refused(text) => text
                .split_once(": ")
                .map_or(text.clone(), |(code, _)| code.to_string()),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    fn map_texture(index: u16) -> AssetId {
        AssetId {
            kind: AssetKind::MapTexture,
            index,
        }
    }

    #[test]
    fn append_writes_the_member_and_undo_restores_it() {
        let mut f = fixture();
        let before = f.pack();
        let new = [tex("neu_a", 16, 3), tex("neu_b", 16, 2)];
        let label = append_label(0, 2);
        assert_eq!(label, "Kartentexturen 0000: 2 Texturen angehängt");
        assert_eq!(
            append_label(20, 1),
            "Kartentexturen 0020: 1 Textur angehängt"
        );
        let report = append_textures(&mut f.store, 0, &new, &label).unwrap();
        let after = f.pack();
        assert_ne!(after, before);
        assert_eq!(after, nsbtx::append(&before, &new).unwrap());
        assert_eq!(report.pack, 0);
        assert_eq!(report.appended, ["neu_a", "neu_b"]);
        assert_eq!(report.size_before, before.len());
        assert_eq!(report.size_after, after.len());
        assert_eq!(report.vram_bytes, pack_vram(&after).unwrap());
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        let (undo, _) = f.store.history();
        assert_eq!(undo, std::slice::from_ref(&label));

        assert_eq!(f.store.undo().unwrap(), [map_texture(0)]);
        assert_eq!(f.pack(), before);
        assert_eq!(f.store.redo().unwrap(), [map_texture(0)]);
        assert_eq!(f.pack(), after);
    }

    #[test]
    fn append_refuses_side_name_taken_and_full() {
        let mut f = fixture();
        let before = f.pack();
        let mut wrong_palette = tex("neu", 16, 3);
        wrong_palette.palette = "gras_pl".into();
        let cases = [
            (vec![tex_wh("seite", 24, 16, 3)], "texture.side"),
            (vec![tex_wh("seite", 16, 2048, 3)], "texture.side"),
            (vec![tex("abcdefghijklmnopq", 16, 3)], "texture.name"),
            (vec![tex("", 16, 3)], "texture.name"),
            (vec![tex("a b", 16, 3)], "texture.name"),
            (vec![tex("gras", 16, 3)], "texture.taken"),
            (vec![wrong_palette], "texture.taken"),
            (vec![tex("neu", 16, 3), tex("neu", 8, 3)], "texture.taken"),
        ];
        for (new, code) in cases {
            let err = append_textures(&mut f.store, 0, &new, "x").unwrap_err();
            assert_eq!(code_of(err), code, "{:?}", new[0].name);
            assert_eq!(f.pack(), before);
            assert!(f.store.history().0.is_empty());
        }

        // Entry 256.
        let full: Vec<NewTexture> = (0..255).map(|i| tex(&format!("t{i:03}"), 8, 2)).collect();
        let full = nsbtx::build(&full).unwrap();
        f.write_member(AssetKind::MapTexture, 0, &full);
        let err = append_textures(&mut f.store, 0, &[tex("neu", 8, 2)], "x").unwrap_err();
        assert_eq!(code_of(err), "texture.pack-full");
        assert_eq!(f.pack(), full);

        // Texels over the 256 KiB bank: 1024 × 512 at 4 bits are the whole bank.
        let bank = nsbtx::build(&[tex_wh("riesig", 1024, 512, 3)]).unwrap();
        f.write_member(AssetKind::MapTexture, 0, &bank);
        let err = append_textures(&mut f.store, 0, &[tex("neu", 8, 2)], "x").unwrap_err();
        assert_eq!(code_of(err), "texture.pack-full");
        assert_eq!(f.pack(), bank);

        // Palettes over the 64 KiB bank: a palette block of 65 536 bytes takes the whole bank.
        let mut btx = nsbtx::Btx0::read(&map_pack()).unwrap();
        btx.palette_data.resize(nsbtx::PALETTE_BANK, 0);
        let palettes = nsbtx::write(&btx).unwrap();
        f.write_member(AssetKind::MapTexture, 0, &palettes);
        let err = append_textures(&mut f.store, 0, &[tex("neu", 8, 2)], "x").unwrap_err();
        assert_eq!(code_of(err), "texture.pack-full");
        assert_eq!(f.pack(), palettes);
        assert!(f.store.history().0.is_empty());
    }

    #[test]
    fn appended_pack_or_refusal_gives_the_message() {
        let before = map_pack();
        // A name already in the pack: the refusal as a message, with its args.
        let m = appended_pack_or_refusal(&before, &[tex("gras", 16, 3)])
            .unwrap()
            .unwrap_err();
        assert_eq!((m.code, m.level), (BakeCode::TextureTaken, Level::Error));
        assert_eq!(m.args, ["texture", "gras"]);
        let m = appended_pack_or_refusal(&before, &[tex_wh("seite", 24, 16, 3)])
            .unwrap()
            .unwrap_err();
        assert_eq!(
            (m.code, m.args.as_slice()),
            (
                BakeCode::TextureSide,
                &["seite".to_string(), "24".into(), "16".into()][..]
            )
        );
        // Accepted: the same bytes as `appended_pack`.
        let new = [tex("neu", 16, 3)];
        let after = appended_pack_or_refusal(&before, &new).unwrap().unwrap();
        assert_eq!(after, appended_pack(&before, &new).unwrap());
        let names: Vec<String> = Nsbtx::read(&after)
            .unwrap()
            .textures
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(names, ["gras", "weg", "neu"]);
        // A pack that cannot be read is the outer error; `appended_pack` keeps `<code>: …`.
        assert!(appended_pack_or_refusal(b"nope", &new).is_err());
        let Err(KernelError::Refused(text)) = appended_pack(&before, &[tex("gras", 16, 3)]) else {
            panic!("refused");
        };
        assert!(text.starts_with("texture.taken: "), "{text}");
    }

    /// HeartGold/SoulSilver keep 4-byte dummies among the building texture members: they
    /// count 0 VRAM, so appending and baking work in their areas; a map pack must still read.
    #[test]
    fn dummy_building_packs_count_as_zero_vram() {
        let mut f = fixture();
        f.write_member(AssetKind::BuildingTexture, 0, &[0; 4]);
        let map = pack_vram(&map_pack()).unwrap();
        assert_eq!(area_vram(&mut f.store, 0, None).unwrap(), map);

        let new = [tex("neu", 16, 3)];
        let report = append_textures(&mut f.store, 0, &new, &append_label(0, 1)).unwrap();
        assert_eq!(report.appended, ["neu"]);
        assert!(report.messages.is_empty(), "{:?}", report.messages);
        let pack = f.pack();
        assert_eq!(
            bake_context(&mut f.store, 0).unwrap().vram_bytes,
            pack_vram(&pack).unwrap()
        );
        let face = Face {
            corners: [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]],
            uv: [[0, 0], [0, 16], [16, 16], [16, 0]],
            texture: "neu".into(),
            layer: 0,
        };
        let report = bake_for_map(&mut f.store, 0, &[face]).unwrap();
        assert!(report.refusal().is_none(), "{:?}", report.messages);
        assert_eq!(report.vram_bytes, pack_vram(&pack).unwrap());

        // A map pack that is no BTX0 is refused, nothing written.
        f.write_member(AssetKind::MapTexture, 0, &[0; 4]);
        let err = append_textures(&mut f.store, 0, &new, "x").unwrap_err();
        assert!(matches!(err, KernelError::Format(_)), "{err:?}");
        assert_eq!(f.pack(), [0; 4]);
        assert_eq!(f.store.history().0.len(), 1);
    }

    #[test]
    fn palette_names_that_collide_after_cutting_are_taken() {
        let mut f = fixture();
        let before = f.pack();
        // `palette_name` gives a 15-byte name only `_`: it equals a 16-byte name ending in
        // `_`, whose palette is its own name.
        let a = tex("abcdefghijklmnX", 16, 3);
        let b = tex("abcdefghijklmnX_", 16, 3);
        assert_eq!(a.palette, "abcdefghijklmnX_");
        assert_eq!(a.palette, b.palette);
        let err = append_textures(&mut f.store, 0, &[a, b], "x").unwrap_err();
        assert_eq!(code_of(err), "texture.taken");
        assert_eq!(f.pack(), before);

        // In the pack: `abcdefghijklmn` holds palette `abcdefghijklmn_p`.
        let c = tex("abcdefghijklmn", 16, 3);
        append_textures(&mut f.store, 0, &[c], "c").unwrap();
        let with_c = f.pack();
        let d = tex("abcdefghijklmn_p", 16, 3);
        assert_eq!(d.palette, "abcdefghijklmn_p");
        let err = append_textures(&mut f.store, 0, &[d], "x").unwrap_err();
        assert_eq!(code_of(err), "texture.taken");
        assert_eq!(f.pack(), with_c);
        assert_eq!(f.store.history().0, ["c"]);

        // Two 15-byte names that differ in their last byte keep different palettes.
        let x = tex("abcdefghijklmnX", 16, 3);
        let y = tex("abcdefghijklmnY", 16, 3);
        assert_ne!(x.palette, y.palette);
        append_textures(&mut f.store, 0, &[x, y], "xy").unwrap();
    }

    #[test]
    fn vram_is_judged_for_every_area_of_the_pack() {
        let mut f = fixture();
        // Building pack 1: 2 × 1024 × 128 at 4 bits, 131 072 texel bytes.
        let big = nsbtx::build(&[
            tex_wh("gross_a", 1024, 128, 3),
            tex_wh("gross_b", 1024, 128, 3),
        ])
        .unwrap();
        f.write_member(AssetKind::BuildingTexture, 1, &big);
        assert_eq!(areas_of_pack(&mut f.store, 0).unwrap(), [0]);
        f.store
            .edit_area_data(1, "Gebiet 1", |a| {
                *a = AreaData::Dppt {
                    buildings_tileset: 1,
                    map_tileset: 0,
                    unknown: 0,
                    light_type: 0,
                };
                Ok(())
            })
            .unwrap();
        assert_eq!(areas_of_pack(&mut f.store, 0).unwrap(), [0, 1]);
        assert_eq!(
            area_vram(&mut f.store, 1, None).unwrap(),
            pack_vram(&map_pack()).unwrap() + pack_vram(&big).unwrap()
        );

        let new = [tex("neu", 16, 3)];
        let label = append_label(0, 1);
        let report = append_textures(&mut f.store, 0, &new, &label).unwrap();
        assert_eq!(report.appended, ["neu"]);
        let high: Vec<&BakeMessage> = report
            .messages
            .iter()
            .filter(|m| m.code == BakeCode::VramHigh)
            .collect();
        assert_eq!(high.len(), 1, "{:?}", report.messages);
        assert_eq!(high[0].level, Level::Warning);
        assert_eq!(high[0].args[0], "1");
        let pack = f.pack();
        let vram1 = pack_vram(&pack).unwrap() + pack_vram(&big).unwrap();
        assert_eq!(high[0].args[1], vram1.to_string());
        assert_eq!(high[0].args[2], VANILLA_VRAM_MAX_PT.to_string());
        assert!(area_vram(&mut f.store, 0, None).unwrap() <= VANILLA_VRAM_MAX_PT);
        assert_eq!(f.store.history().0.last(), Some(&label));
    }

    #[test]
    fn pack_textures_resolve_palettes_by_name_then_pl_then_single() {
        // `e` has `e_pl` and a palette named `e` (of `f`): the name wins. `f` and `c` find
        // none of theirs among several palettes and are left out.
        let e = tex("e", 8, 3);
        let mut f = tex("f", 8, 3);
        f.palette = "e".into();
        let mut c = tex("c", 16, 2);
        c.palette = "zz".into();
        let b = tex("b", 16, 2);
        let pack = nsbtx::build(&[e, f, c, b]).unwrap();
        let got = pack_textures(&pack).unwrap();
        let want = [("e", "e", 8), ("b", "b_pl", 16)].map(|(name, palette, side)| PackTexture {
            name: name.into(),
            palette: palette.into(),
            width: side,
            height: side,
        });
        assert_eq!(got, want);

        // The only palette of a pack binds every texture.
        let mut d = tex_wh("d", 32, 16, 2);
        d.palette = "only".into();
        let single = nsbtx::build(&[d]).unwrap();
        assert_eq!(
            pack_textures(&single).unwrap(),
            [PackTexture {
                name: "d".into(),
                palette: "only".into(),
                width: 32,
                height: 16,
            }]
        );
        assert_eq!(
            pack_vram(&single).unwrap(),
            nsbtx::Btx0::read(&single).unwrap().vram_bytes()
        );
        assert_eq!(pack_vram(&single).unwrap(), 32 * 16 / 4 + 8);
    }

    #[test]
    fn bake_for_map_takes_style_and_textures_of_the_area() {
        let mut f = fixture();
        let ctx = bake_context(&mut f.store, 0).unwrap();
        let vram = pack_vram(&map_pack()).unwrap() + pack_vram(&building_pack()).unwrap();
        assert_eq!(
            ctx,
            BakeContext {
                area: 0,
                pack: 0,
                style: AreaStyle::Outdoor,
                textures: pack_textures(&map_pack()).unwrap(),
                vram_bytes: vram,
                vram_limit: VANILLA_VRAM_MAX_PT,
            }
        );
        assert_eq!(ctx.textures.len(), 2);
        let face = Face {
            corners: [[0, 0, 0], [0, 0, 16], [16, 0, 16], [16, 0, 0]],
            uv: [[0, 0], [0, 16], [16, 16], [16, 0]],
            texture: "gras".into(),
            layer: 0,
        };
        let report = bake_for_map(&mut f.store, 0, std::slice::from_ref(&face)).unwrap();
        assert!(report.refusal().is_none(), "{:?}", report.messages);
        assert_eq!(report.textures, ["gras"]);
        assert_eq!(report.vram_bytes, vram);
        assert!(report.size > 0);

        // Light type 2 is indoor on Platinum; map 2 is shown by no header.
        f.store
            .edit_area_data(0, "Licht", |a| {
                if let AreaData::Dppt { light_type, .. } = a {
                    *light_type = 2;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(
            bake_context(&mut f.store, 0).unwrap().style,
            AreaStyle::Indoor
        );
        assert!(matches!(
            bake_context(&mut f.store, 2),
            Err(KernelError::Refused(_))
        ));
    }

    #[test]
    fn diamond_pearl_is_refused() {
        let mut f = fixture();
        let mut project = Project::open(f.store.project().root.as_path()).unwrap();
        project.family = g4_formats::game::Family::Dp;
        f.store = Store::new(Arc::new(project));
        let before = f.pack();
        let err = append_textures(&mut f.store, 0, &[tex("neu", 8, 3)], "x").unwrap_err();
        assert!(err.to_string().contains("Diamond/Pearl"), "{err}");
        assert_eq!(f.pack(), before);
    }

    /// Plan „Offen für die Umsetzung“, Textprojekt: the existing sync takes a pack changed by
    /// `append_textures` into `data/map_textures/NNNN.nsbtx`.
    #[test]
    fn the_text_project_sync_takes_the_appended_pack_into_data() {
        let fx = TextFixture::with(prepare);
        crate::textproject::export(
            &fx.project(),
            &fx.base(),
            &crate::textproject::ExportOptions {
                adopt_scripts: true,
                force: false,
            },
        )
        .unwrap();
        let rel = "data/map_textures/0000.nsbtx";
        println!(
            "text project: {rel} before the append: {}",
            fx.root().join(rel).exists()
        );
        let mut store = Store::new(Arc::new(fx.project()));
        append_textures(&mut store, 0, &[tex("neu", 16, 3)], &append_label(0, 1)).unwrap();
        let member = std::fs::read(store.project().member_path(AssetKind::MapTexture, 0)).unwrap();
        let found = crate::textproject::sync::scan(store.project(), &mut |_| {}).unwrap();
        println!(
            "text project: units {:?}",
            found
                .units
                .iter()
                .map(|u| (&u.key, u.side))
                .collect::<Vec<_>>()
        );
        crate::textproject::sync::apply_one_sided(store.project(), &found, &|_| false).unwrap();
        assert!(fx.root().join(rel).is_file(), "{rel} is not written");
        assert_eq!(fx.read(rel), member);
    }
}
