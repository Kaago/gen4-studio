//! The textures of an import, planned against the area's map texture pack: images of model
//! materials and PNG files become new palette textures (`texconv::convert`, not scaled), a name
//! the pack already has is reused (model) or refused (PNG file).
//!
//! Nothing is appended here: the plan carries the new textures, the caller appends them
//! (`appended_pack_or_refusal` for the preview, `append_textures` for the import).

use std::path::{Path, PathBuf};

use g4_formats::error::FormatError;
use g4_formats::nsbtx::{NewTexture, Rgba};
use g4_formats::texconv::{self, Colors};

use super::mesh::{Binding, MeshMaterial};
use crate::bake::PackTexture;
use crate::map_import::{PartCode, PartMessage};

/// A texture of the plan as the preview lists it: `new` (converted from `file`, `colors` and
/// `mean_error` of the conversion) or one of the pack (`colors` 0: its format is not read,
/// `mean_error` `None`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct TexturePreview {
    pub name: String,
    #[serde(serialize_with = "crate::relpath::ser_opt")]
    pub file: Option<PathBuf>,
    pub width: u32,
    pub height: u32,
    pub colors: u8,
    pub new: bool,
    pub mean_error: Option<f32>,
}

/// The textures of an import: one binding per material (`None`: untextured), the textures to
/// append, the preview of every texture used (in order of first use) and the hints
/// (`texture.new`, `texture.reused`).
#[derive(Debug, Clone, PartialEq)]
pub struct TexturePlan {
    pub bindings: Vec<Option<Binding>>,
    pub new: Vec<NewTexture>,
    pub previews: Vec<TexturePreview>,
    pub notes: Vec<PartMessage>,
}

/// The longest name of the pack, in bytes.
const NAME_LEN: usize = 16;
/// The name of a texture whose stem keeps nothing.
const FALLBACK_NAME: &str = "g4_tex";

/// Printable ASCII 0x21..=0x7E kept, every other character '_' (one per character, also for a
/// multi-byte UTF-8 character), cut to 16 bytes, empty → "g4_tex".
pub fn texture_name(stem: &str) -> String {
    let mut name: String = stem
        .chars()
        .map(|c| {
            if ('\x21'..='\x7E').contains(&c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    // ASCII only from here: a byte cut is a character cut.
    name.truncate(NAME_LEN);
    if name.is_empty() {
        FALLBACK_NAME.to_string()
    } else {
        name
    }
}

/// The pixels of a PNG or JPEG as RGBA8; texture.unreadable `[file, reason]`, texture.side
/// `[file, width, height]` for a side over 1024 (read from the header, before decoding).
pub fn load_image(bytes: &[u8], file: &str) -> Result<Rgba, PartMessage> {
    super::decode_image(bytes, file)
}

/// One binding per material (rules „Materialien und Texturen“); new textures converted with
/// `colors`.
///
/// A material with an image: its texture is named [`texture_name`] of the image's stem; a name
/// the pack has is that pack texture (hint `texture.reused`, nothing appended); a different
/// image with a name already planned gets `_2`, `_3` … (cut to 16 bytes, never a name of the
/// pack); the same image twice is one new texture. A material without an image binds the pack
/// texture of its name, else it is untextured (`None`; `to_faces` refuses a polygon that uses
/// it). Refused: a side no power of two from 8 to 1024 (`texture.side`).
pub fn plan_textures(
    materials: &[MeshMaterial],
    pack: &[PackTexture],
    colors: Colors,
) -> Result<TexturePlan, PartMessage> {
    let mut planner = Planner::new(pack, colors);
    for material in materials {
        let binding = match &material.image {
            Some(image) => {
                let base = texture_name(&image.stem);
                match planner.pack_texture(&base) {
                    Some(texture) => Some(planner.reuse(texture)),
                    None => {
                        let label = image
                            .file
                            .as_deref()
                            .map_or_else(|| image.stem.clone(), |f| f.display().to_string());
                        Some(planner.convert(base, image.file.clone(), &label, &image.rgba)?)
                    }
                }
            }
            None => planner
                .pack_texture(&material.name)
                .map(|texture| planner.bind_pack(texture)),
        };
        planner.plan.bindings.push(binding);
    }
    Ok(planner.plan)
}

/// The PNG files as new textures (no bindings); a name already in the pack is texture.taken.
///
/// Name [`texture_name`] of the file stem; two different images with one name get `_2`, `_3`
/// … like the images of a model. Refused: a file that cannot be read (`texture.unreadable`), a
/// side no power of two from 8 to 1024 (`texture.side`), a name of the pack (`texture.taken`
/// `["texture", name]`: replacing is not possible).
pub fn png_textures(
    files: &[PathBuf],
    pack: &[PackTexture],
    colors: Colors,
) -> Result<TexturePlan, PartMessage> {
    let mut planner = Planner::new(pack, colors);
    for file in files {
        let label = file_label(file);
        let bytes = std::fs::read(file).map_err(|e| unreadable(&label, &e.to_string()))?;
        let rgba = load_image(&bytes, &label)?;
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let base = texture_name(&stem);
        if planner.pack_texture(&base).is_some() {
            return Err(PartMessage::new(
                PartCode::TextureTaken,
                &[&"texture", &base],
                format!("the texture name {base:?} is already in the area's pack"),
            ));
        }
        planner.convert(base, Some(file.clone()), &label, &rgba)?;
    }
    Ok(planner.plan)
}

/// The file name of `file` as the messages give it.
fn file_label(file: &Path) -> String {
    file.file_name().map_or_else(
        || file.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn unreadable(file: &str, reason: &str) -> PartMessage {
    PartMessage::new(
        PartCode::TextureUnreadable,
        &[&file, &reason],
        format!("the image {file} cannot be read ({reason})"),
    )
}

/// texture.side `[name, width, height]`.
pub(crate) fn side_refused(name: &str, width: u32, height: u32) -> PartMessage {
    PartMessage::new(
        PartCode::TextureSide,
        &[&name, &width, &height],
        format!(
            "the texture {name} is {width} × {height} pixels; each side must be a power of two \
             from 8 to 1024"
        ),
    )
}

fn side_ok(side: u32) -> bool {
    side.is_power_of_two() && (8..=1024).contains(&side)
}

/// The state of one plan while it is made.
struct Planner<'a> {
    pack: &'a [PackTexture],
    colors: Colors,
    plan: TexturePlan,
    /// Per new texture (same order as `plan.new`): the name it was asked for and its pixels,
    /// to find the same image again.
    asked: Vec<(String, Rgba)>,
}

impl<'a> Planner<'a> {
    fn new(pack: &'a [PackTexture], colors: Colors) -> Planner<'a> {
        Planner {
            pack,
            colors,
            plan: TexturePlan {
                bindings: Vec::new(),
                new: Vec::new(),
                previews: Vec::new(),
                notes: Vec::new(),
            },
            asked: Vec::new(),
        }
    }

    fn pack_texture(&self, name: &str) -> Option<&'a PackTexture> {
        self.pack.iter().find(|t| t.name == name)
    }

    /// Whether `name` is free: neither in the pack nor planned.
    fn free(&self, name: &str) -> bool {
        self.pack_texture(name).is_none() && self.plan.new.iter().all(|t| t.name != name)
    }

    /// `base`, else the first free of `base_2`, `base_3` … (base cut so the name has 16 bytes).
    fn free_name(&self, base: &str) -> String {
        if self.free(base) {
            return base.to_string();
        }
        (2usize..)
            .map(|n| {
                let suffix = format!("_{n}");
                let keep = NAME_LEN.saturating_sub(suffix.len()).min(base.len());
                format!("{}{suffix}", &base[..keep])
            })
            .find(|name| self.free(name))
            .unwrap_or_else(|| base.to_string())
    }

    /// The pack texture as the binding of an image of its name: hint `texture.reused` once.
    fn reuse(&mut self, texture: &PackTexture) -> Binding {
        if !self.shown(&texture.name) {
            self.plan.notes.push(PartMessage::new(
                PartCode::TextureReused,
                &[&texture.name],
                format!(
                    "the texture {} is already in the area's pack and is used as it is",
                    texture.name
                ),
            ));
        }
        self.bind_pack(texture)
    }

    /// The binding of a pack texture, listed in the preview once.
    fn bind_pack(&mut self, texture: &PackTexture) -> Binding {
        if !self.shown(&texture.name) {
            self.plan.previews.push(TexturePreview {
                name: texture.name.clone(),
                file: None,
                width: u32::from(texture.width),
                height: u32::from(texture.height),
                colors: 0,
                new: false,
                mean_error: None,
            });
        }
        Binding {
            texture: texture.name.clone(),
            width: texture.width,
            height: texture.height,
        }
    }

    fn shown(&self, name: &str) -> bool {
        self.plan.previews.iter().any(|p| p.name == name)
    }

    /// The image as a new texture asked for as `base` (the same image again: the texture made
    /// for it); `label` names the file in messages.
    fn convert(
        &mut self,
        base: String,
        file: Option<PathBuf>,
        label: &str,
        rgba: &Rgba,
    ) -> Result<Binding, PartMessage> {
        if let Some(i) = self
            .asked
            .iter()
            .position(|(asked, pixels)| *asked == base && pixels == rgba)
            && let Some(t) = self.plan.new.get(i)
        {
            return Ok(binding_of(t));
        }
        let name = self.free_name(&base);
        if !(side_ok(rgba.width) && side_ok(rgba.height)) {
            return Err(side_refused(&name, rgba.width, rgba.height));
        }
        let q = texconv::convert(&name, rgba, self.colors).map_err(|e| match e {
            FormatError::OutOfRange {
                what: "Nitro name", ..
            } => PartMessage::new(
                PartCode::TextureName,
                &[&name],
                format!("{name:?} cannot be a name in the pack: {e}"),
            ),
            e => unreadable(label, &e.to_string()),
        })?;
        let colors: u8 = match self.colors {
            Colors::Four => 4,
            Colors::Sixteen => 16,
        };
        let mean = format!("{:.1}", q.mean_error);
        self.plan.notes.push(PartMessage::new(
            PartCode::TextureNew,
            &[&name, &rgba.width, &rgba.height, &colors, &mean],
            format!(
                "new texture {name}: {} × {}, {colors} colours, mean colour error {mean}",
                rgba.width, rgba.height
            ),
        ));
        self.plan.previews.push(TexturePreview {
            name: name.clone(),
            file,
            width: rgba.width,
            height: rgba.height,
            colors,
            new: true,
            mean_error: Some(q.mean_error),
        });
        let binding = binding_of(&q.texture);
        self.plan.new.push(q.texture);
        self.asked.push((base, rgba.clone()));
        Ok(binding)
    }
}

/// The binding of a new texture (its sides are checked: at most 1024).
fn binding_of(t: &NewTexture) -> Binding {
    Binding {
        texture: t.name.clone(),
        width: u16::try_from(t.width).unwrap_or(u16::MAX),
        height: u16::try_from(t.height).unwrap_or(u16::MAX),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::io::Cursor;

    use g4_formats::nsbtx::{self, Nsbtx};
    use g4_formats::texconv::{palette_name, to_bgr555};

    use super::*;
    use crate::model_import::mesh::MeshImage;
    use crate::model_import::samples::{PNG_40, forty_colours, write_samples};

    /// A `w` × `h` opaque image with colour `pixel(x, y)`.
    fn rgba(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Rgba {
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                pixels.extend(pixel(x, y));
                pixels.push(255);
            }
        }
        Rgba {
            width: w,
            height: h,
            pixels,
        }
    }

    fn png(image: &Rgba) -> Vec<u8> {
        let buffer =
            image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone()).unwrap();
        let mut out = Cursor::new(Vec::new());
        buffer.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn material(name: &str, image: Option<(&str, Rgba)>) -> MeshMaterial {
        MeshMaterial {
            name: name.to_string(),
            image: image.map(|(stem, rgba)| MeshImage {
                stem: stem.to_string(),
                file: Some(PathBuf::from(format!("{stem}.png"))),
                rgba,
            }),
        }
    }

    fn pack_texture(name: &str, width: u16, height: u16) -> PackTexture {
        PackTexture {
            name: name.to_string(),
            palette: palette_name(name),
            width,
            height,
        }
    }

    fn stripes(w: u32, h: u32) -> Rgba {
        rgba(w, h, |x, _| [(x * 16) as u8, 100, 200])
    }

    fn codes(notes: &[PartMessage]) -> Vec<PartCode> {
        notes.iter().map(|n| n.code).collect()
    }

    fn channels(c: u16) -> [i32; 3] {
        [
            i32::from(c & 31),
            i32::from(c >> 5 & 31),
            i32::from(c >> 10 & 31),
        ]
    }

    /// The largest difference of one 5-bit channel between two BGR555 colours.
    fn step(a: u16, b: u16) -> i32 {
        channels(a)
            .iter()
            .zip(channels(b))
            .map(|(x, y)| (x - y).abs())
            .max()
            .unwrap_or(0)
    }

    /// Gate „40 Farben“: the 64 × 64 PNG with 40 colours, converted to 16, written and read
    /// back: every pixel with a palette hit comes back within one step per channel.
    ///
    /// A palette hit: the palette has a colour within one 5-bit step per channel of the
    /// pixel's BGR555 colour `q`. (The median cut gives box means, so none of the 40 colours
    /// is in the palette exactly; measured: 0 of 40.) Over all pixels: at most 3 steps per
    /// channel and a mean colour error of at most 13.0.
    #[test]
    fn a_64x64_png_with_40_colours_becomes_a_16_colour_texture_within_one_step() {
        let dir = tempfile::tempdir().unwrap();
        write_samples(dir.path()).unwrap();
        let file = dir.path().join(PNG_40);
        let plan = png_textures(std::slice::from_ref(&file), &[], Colors::Sixteen).unwrap();
        assert!(plan.bindings.is_empty());
        let [t] = plan.new.as_slice() else {
            panic!("one new texture: {:?}", plan.new.len());
        };
        assert_eq!(
            (t.name.as_str(), t.format, t.colors.len(), t.width, t.height),
            ("g4_imp_40", 3, 16, 64, 64)
        );
        assert_eq!(codes(&plan.notes), [PartCode::TextureNew]);
        let [preview] = plan.previews.as_slice() else {
            panic!("one preview");
        };
        assert_eq!(preview.file.as_deref(), Some(file.as_path()));
        assert_eq!((preview.colors, preview.new), (16, true));
        let mean = preview.mean_error.unwrap();

        let btx = Nsbtx::read(&nsbtx::build(&plan.new).unwrap()).unwrap();
        let decoded = btx.decode(0, 0).unwrap();
        let source = load_image(&std::fs::read(&file).unwrap(), PNG_40).unwrap();
        let palette = &btx.palettes[0].colors;
        let exact = forty_colours()
            .into_iter()
            .filter(|&c| palette.contains(&to_bgr555(c)))
            .count();
        let (mut hit_colours, mut hit_pixels, mut max_step) = (BTreeSet::new(), 0, 0);
        assert_eq!(source.pixels.len(), decoded.pixels.len());
        for (s, d) in source
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(decoded.pixels.as_chunks::<4>().0)
        {
            let q = to_bgr555([s[0], s[1], s[2]]);
            let back = to_bgr555([d[0], d[1], d[2]]);
            let off = step(q, back);
            max_step = max_step.max(off);
            if palette.iter().any(|&p| step(p, q) <= 1) {
                assert!(off <= 1, "{q:#06x} came back as {back:#06x}");
                hit_colours.insert(q);
                hit_pixels += 1;
            }
        }
        let forty: BTreeSet<u16> = forty_colours().into_iter().map(to_bgr555).collect();
        assert_eq!(forty.len(), 40);
        assert!(hit_colours.is_subset(&forty));
        assert!(!hit_colours.is_empty(), "at least one palette hit");
        println!(
            "png40 hits={}/40 pixels={hit_pixels}/4096 max_step={max_step} mean={mean} \
             exact={exact}/40",
            hit_colours.len()
        );
        // The quality of the quantizer over every pixel, so a worse median cut is caught:
        // measured 3 steps and 12.82 (texconv of 6.1, deterministic); the bounds keep that.
        assert!(max_step <= 3, "max_step {max_step}");
        assert!(mean <= 13.0, "mean {mean}");
    }

    #[test]
    fn texture_name_keeps_printable_ascii_and_cuts_to_16() {
        assert_eq!(texture_name("Gras Weg"), "Gras_Weg");
        assert_eq!(texture_name("größe"), "gr__e");
        assert_eq!(texture_name("abcdefghijklmnopqrst"), "abcdefghijklmnop");
        assert_eq!(texture_name(""), "g4_tex");
        assert_eq!(texture_name("tab\there~"), "tab_here~");
        // Every name it gives is a name of the pack.
        for stem in ["Gras Weg", "größe", "", "ééééééééééééééééé", "a/b\\c"] {
            g4_formats::nitro_dict::name(&texture_name(stem)).unwrap();
        }
    }

    #[test]
    fn two_images_with_one_stem_get_numbered_names() {
        let a = stripes(16, 16);
        let b = rgba(16, 16, |_, y| [10, (y * 16) as u8, 10]);
        let materials = [
            material("m1", Some(("kachel", a.clone()))),
            material("m2", Some(("kachel", b.clone()))),
            material("m3", Some(("kachel", a.clone()))),
        ];
        let plan = plan_textures(&materials, &[], Colors::Sixteen).unwrap();
        let names: Vec<&str> = plan.new.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["kachel", "kachel_2"]);
        let bound: Vec<&str> = plan
            .bindings
            .iter()
            .map(|b| b.as_ref().unwrap().texture.as_str())
            .collect();
        assert_eq!(bound, ["kachel", "kachel_2", "kachel"]);
        assert_eq!(
            codes(&plan.notes),
            [PartCode::TextureNew, PartCode::TextureNew]
        );
        assert_eq!(plan.previews.len(), 2);

        // The same image for two materials: one new texture, two bindings on its name.
        let same = [
            material("m1", Some(("kachel", a.clone()))),
            material("m2", Some(("kachel", a.clone()))),
        ];
        let plan = plan_textures(&same, &[], Colors::Sixteen).unwrap();
        assert_eq!(plan.new.len(), 1);
        assert_eq!(plan.bindings[0], plan.bindings[1]);
        assert_eq!(plan.bindings[0].as_ref().unwrap().texture, "kachel");

        // A 16-byte stem is cut for its number; a number the pack has is skipped.
        let long = "abcdefghijklmnop";
        let materials = [
            material("m1", Some((long, a.clone()))),
            material("m2", Some((long, b.clone()))),
        ];
        let plan = plan_textures(
            &materials,
            &[pack_texture("abcdefghijklmn_2", 8, 8)],
            Colors::Sixteen,
        )
        .unwrap();
        let names: Vec<&str> = plan.new.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, [long, "abcdefghijklmn_3"]);
    }

    #[test]
    fn a_pack_texture_of_the_same_name_is_reused_for_a_model_but_taken_for_png() {
        let pack = [pack_texture("g4_imp_cube", 32, 64)];
        let materials = [material("cube", Some(("g4_imp_cube", stripes(16, 16))))];
        let plan = plan_textures(&materials, &pack, Colors::Sixteen).unwrap();
        assert!(plan.new.is_empty());
        assert_eq!(codes(&plan.notes), [PartCode::TextureReused]);
        assert_eq!(plan.notes[0].args, ["g4_imp_cube"]);
        assert_eq!(
            plan.bindings,
            [Some(Binding {
                texture: "g4_imp_cube".into(),
                width: 32,
                height: 64,
            })]
        );
        assert_eq!(
            plan.previews,
            [TexturePreview {
                name: "g4_imp_cube".into(),
                file: None,
                width: 32,
                height: 64,
                colors: 0,
                new: false,
                mean_error: None,
            }]
        );

        let dir = tempfile::tempdir().unwrap();
        write_samples(dir.path()).unwrap();
        let e = png_textures(
            &[dir.path().join("g4_imp_cube.png")],
            &pack,
            Colors::Sixteen,
        )
        .unwrap_err();
        assert_eq!(
            (e.code, e.args.as_slice()),
            (
                PartCode::TextureTaken,
                &["texture".to_string(), "g4_imp_cube".into()][..]
            )
        );
    }

    #[test]
    fn a_material_without_image_binds_the_pack_texture_of_its_name() {
        let pack = [pack_texture("gras", 16, 16), pack_texture("weg", 8, 8)];
        let plan = plan_textures(&[material("weg", None)], &pack, Colors::Sixteen).unwrap();
        assert_eq!(
            plan.bindings,
            [Some(Binding {
                texture: "weg".into(),
                width: 8,
                height: 8,
            })]
        );
        assert!(plan.new.is_empty() && plan.notes.is_empty());
        assert_eq!(plan.previews.len(), 1);
        assert!(!plan.previews[0].new);
    }

    #[test]
    fn an_unknown_material_without_image_is_untextured() {
        let pack = [pack_texture("gras", 16, 16)];
        let plan = plan_textures(
            &[material("stein", None), material("", None)],
            &pack,
            Colors::Sixteen,
        )
        .unwrap();
        assert_eq!(plan.bindings, [None, None]);
        assert!(plan.new.is_empty() && plan.notes.is_empty() && plan.previews.is_empty());
    }

    #[test]
    fn sides_not_power_of_two_are_texture_side() {
        let e = plan_textures(
            &[material("m", Some(("kachel", stripes(24, 16))))],
            &[],
            Colors::Sixteen,
        )
        .unwrap_err();
        assert_eq!(
            (e.code, e.args.as_slice()),
            (
                PartCode::TextureSide,
                &["kachel".to_string(), "24".into(), "16".into()][..]
            )
        );
        // Too small and as a PNG file.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("klein.png");
        std::fs::write(&file, png(&stripes(4, 8))).unwrap();
        let e = png_textures(&[file], &[], Colors::Sixteen).unwrap_err();
        assert_eq!(e.code, PartCode::TextureSide);
        assert_eq!(e.args, ["klein", "4", "8"]);
    }

    /// The CRC-32 of PNG chunks (ISO 3309), for a patched header.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    crc >> 1 ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn an_unreadable_image_is_texture_unreadable() {
        let e = load_image(b"not an image", "kaputt.png").unwrap_err();
        assert_eq!(e.code, PartCode::TextureUnreadable);
        assert_eq!(e.args[0], "kaputt.png");
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("kaputt.png");
        std::fs::write(&file, b"\x89PNG\r\n\x1a\nbroken").unwrap();
        let e = png_textures(&[file], &[], Colors::Sixteen).unwrap_err();
        assert_eq!(e.code, PartCode::TextureUnreadable);
        assert_eq!(e.args[0], "kaputt.png");
        let e = png_textures(&[dir.path().join("fehlt.png")], &[], Colors::Sixteen).unwrap_err();
        assert_eq!(
            (e.code, e.args[0].as_str()),
            (PartCode::TextureUnreadable, "fehlt.png")
        );

        // A header that claims 60 000 × 60 000 pixels: refused from the header, nothing decoded.
        let mut bytes = png(&stripes(8, 8));
        bytes[16..20].copy_from_slice(&60_000u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&60_000u32.to_be_bytes());
        let crc = crc32(&bytes[12..29]);
        bytes[29..33].copy_from_slice(&crc.to_be_bytes());
        let e = load_image(&bytes, "riesig.png").unwrap_err();
        assert_eq!(e.code, PartCode::TextureSide);
        assert_eq!(e.args, ["riesig.png", "60000", "60000"]);
    }

    #[test]
    fn four_colours_when_asked() {
        let materials = [material("m", Some(("kachel", stripes(16, 8))))];
        let plan = plan_textures(&materials, &[], Colors::Four).unwrap();
        let [t] = plan.new.as_slice() else {
            panic!("one texture");
        };
        assert_eq!((t.format, t.colors.len()), (2, 4));
        assert_eq!(plan.previews[0].colors, 4);
        assert_eq!(plan.notes[0].args[..4], ["kachel", "16", "8", "4"]);
        assert_eq!(
            plan.bindings[0],
            Some(Binding {
                texture: "kachel".into(),
                width: 16,
                height: 8,
            })
        );
        // The mean error has one decimal.
        let mean = &plan.notes[0].args[4];
        assert_eq!(
            mean.split_once('.').map(|(_, d)| d.len()),
            Some(1),
            "{mean}"
        );
    }
}
