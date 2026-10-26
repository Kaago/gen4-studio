//! The project preview: a 256 × 192 picture of the project for the start list. The own picture
//! lives in `.g4/preview.png`, the standard picture (the start town's map, rendered by the app)
//! in `.g4/preview-standard.png`; both are PNG RGB8. Reading prefers the own picture, then the
//! standard one, and never fails: a missing folder or a broken file counts as no picture.
//! Writing fits any PNG or JPEG ([`fit_preview`]) and replaces the slot's file atomically.
//! The preview is no game file and has no undo. See the phase 4.0 design, §3.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, ImageReader, Rgb, RgbImage, Rgba};
use serde::{Deserialize, Serialize};

use crate::error::{KernelError, io_at};
use crate::fsutil::write_atomic;
use crate::project::{AssetKind, Project};
use crate::templates;

pub const PREVIEW_W: u32 = 256;
pub const PREVIEW_H: u32 = 192;

/// The DS ground colour any alpha is laid on: the design token `szene/ds/grund` (#182131, in
/// `design-system/tokens/Szene/Wert.tokens.json` and `apps/studio/src/ui/tokens/tokens.ts`).
const DS_GROUND: [u8; 3] = [0x18, 0x21, 0x31];

/// Which file a read preview came from.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PreviewSource {
    Eigen,
    Standard,
}

/// The file a write goes to.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum PreviewSlot {
    #[default]
    Eigen,
    Standard,
}

/// The project's preview: the PNG bytes (256 × 192, RGB8) and where they came from; both
/// `None` when the project has no valid preview file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Preview {
    pub source: Option<PreviewSource>,
    pub png: Option<Vec<u8>>,
}

/// What [`write_preview`] puts into a slot: image bytes, an image file (PNG or JPEG, told
/// apart by content), or nothing (the slot's file is removed).
#[derive(Clone, Copy, Debug)]
pub enum PreviewInput<'a> {
    Bytes(&'a [u8]),
    File(&'a Path),
    Remove,
}

/// The start town's map: its header, the map of the header's first cell in matrix 0 and the
/// header's area data.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StandardMap {
    pub header: u16,
    pub map: u16,
    pub area_data_id: u16,
}

/// Largest image file [`fit_preview`] takes.
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
/// Largest width and height [`fit_preview`] takes.
const MAX_SIDE: u32 = 8192;

fn refused(code: &str, rest: impl std::fmt::Display) -> KernelError {
    KernelError::Refused(format!("{code}: {rest}"))
}

/// Reads the project's preview from `<root>/.g4/`: the own picture, else the standard picture,
/// else none. Only files are read (no `Project::open`); a file that cannot be read or is no
/// PNG of 256 × 192 counts as missing, and so does a missing project folder, so this never
/// fails.
pub fn read_preview(root: &Path) -> Result<Preview, KernelError> {
    for (slot, source) in [
        (PreviewSlot::Eigen, PreviewSource::Eigen),
        (PreviewSlot::Standard, PreviewSource::Standard),
    ] {
        let Ok(bytes) = std::fs::read(slot_path(root, slot)) else {
            continue;
        };
        if is_preview_png(&bytes) {
            return Ok(Preview {
                source: Some(source),
                png: Some(bytes),
            });
        }
    }
    Ok(Preview {
        source: None,
        png: None,
    })
}

/// Whether `bytes` decode as a PNG of exactly 256 × 192.
fn is_preview_png(bytes: &[u8]) -> bool {
    image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .is_ok_and(|img| (img.width(), img.height()) == (PREVIEW_W, PREVIEW_H))
}

/// Writes `input` into the file of `slot` (atomically through a temp file under `.g4/`,
/// creating `.g4/`) or removes that file (`Remove`; a missing file is fine) and returns the
/// preview as [`read_preview`] then reads it. A refused input leaves the slot as it was; a
/// project folder that does not exist is an I/O error (it is never created).
pub fn write_preview(
    root: &Path,
    slot: PreviewSlot,
    input: PreviewInput,
) -> Result<Preview, KernelError> {
    let path = slot_path(root, slot);
    match input {
        PreviewInput::Remove => match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_at(&path)(e)),
        },
        PreviewInput::Bytes(bytes) => store(root, &path, &fit_preview(bytes)?)?,
        PreviewInput::File(file) => {
            let bytes = read_image_file(file)?;
            store(root, &path, &fit_preview(&bytes)?)?;
        }
    }
    read_preview(root)
}

/// Reads an image file for [`write_preview`]: `preview.unreadable` when it is missing or
/// cannot be read, `preview.too_large` over 32 MiB (checked before reading it).
fn read_image_file(file: &Path) -> Result<Vec<u8>, KernelError> {
    let meta = std::fs::metadata(file)
        .map_err(|e| refused("preview.unreadable", format!("{}: {e}", file.display())))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(refused(
            "preview.too_large",
            format!(
                "{} has {} bytes, at most {MAX_FILE_BYTES}",
                file.display(),
                meta.len()
            ),
        ));
    }
    std::fs::read(file)
        .map_err(|e| refused("preview.unreadable", format!("{}: {e}", file.display())))
}

fn store(root: &Path, path: &Path, png: &[u8]) -> Result<(), KernelError> {
    if !root.is_dir() {
        return Err(io_at(root)(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "project folder not found",
        )));
    }
    // `write_atomic` creates `.g4/tmp/`, and with it `.g4/`.
    write_atomic(root, path, png)
}

/// Fits a PNG or JPEG (told apart by content) into the preview: crops it to 4 : 3 around its
/// centre ("cover"), scales it to 256 × 192 (Lanczos3 when shrinking, Nearest when growing by
/// a whole factor, CatmullRom otherwise; a picture of exactly that size is only re-encoded),
/// lays any alpha on the DS ground colour and encodes it as PNG RGB8. Refused:
/// `preview.format` (neither PNG nor JPEG), `preview.too_large` (over 32 MiB or over
/// 8192 × 8192, checked before decoding) and `preview.unreadable` (a broken picture).
pub fn fit_preview(bytes: &[u8]) -> Result<Vec<u8>, KernelError> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(refused(
            "preview.too_large",
            format!(
                "the file has {} bytes, at most {MAX_FILE_BYTES}",
                bytes.len()
            ),
        ));
    }
    let format = match image::guess_format(bytes) {
        Ok(f @ (ImageFormat::Png | ImageFormat::Jpeg)) => f,
        Ok(other) => {
            return Err(refused(
                "preview.format",
                format!("{other:?} is neither PNG nor JPEG"),
            ));
        }
        Err(_) => return Err(refused("preview.format", "neither PNG nor JPEG")),
    };
    let reader = || {
        let mut reader = ImageReader::new(Cursor::new(bytes));
        reader.set_format(format);
        reader
    };
    let (width, height) = reader()
        .into_dimensions()
        .map_err(|e| refused("preview.unreadable", e))?;
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(refused(
            "preview.too_large",
            format!("{width} × {height} pixels, at most {MAX_SIDE} × {MAX_SIDE}"),
        ));
    }
    let img = reader()
        .decode()
        .map_err(|e| refused("preview.unreadable", e))?;
    if img.width() == 0 || img.height() == 0 {
        return Err(refused("preview.unreadable", "the picture is empty"));
    }

    // Cover: the largest 4 : 3 rectangle around the centre.
    let (w, h) = (img.width(), img.height());
    let (crop_w, crop_h) = if u64::from(w) * 3 > u64::from(h) * 4 {
        // Too wide: keep the height. 4 · h / 3 < w here, so it fits in u32.
        (((u64::from(h) * 4) / 3).max(1) as u32, h)
    } else {
        // Too tall (or exact): keep the width; 3 · w / 4 ≤ h.
        (w, ((u64::from(w) * 3) / 4).max(1) as u32)
    };
    let cropped = img.crop_imm((w - crop_w) / 2, (h - crop_h) / 2, crop_w, crop_h);

    // Alpha onto the ground before scaling, so transparent colours never bleed in.
    let rgba = cropped.to_rgba8();
    let rgb = RgbImage::from_fn(crop_w, crop_h, |x, y| {
        let Rgba([r, g, b, a]) = *rgba.get_pixel(x, y);
        let mix = |c: u8, ground: u8| {
            let (c, ground, a) = (u32::from(c), u32::from(ground), u32::from(a));
            // At most (255 · 255 + 127) / 255 = 255.
            ((c * a + ground * (255 - a) + 127) / 255) as u8
        };
        Rgb([
            mix(r, DS_GROUND[0]),
            mix(g, DS_GROUND[1]),
            mix(b, DS_GROUND[2]),
        ])
    });

    let fitted = if (crop_w, crop_h) == (PREVIEW_W, PREVIEW_H) {
        rgb
    } else {
        let filter = if crop_w >= PREVIEW_W && crop_h >= PREVIEW_H {
            FilterType::Lanczos3
        } else if PREVIEW_W.is_multiple_of(crop_w)
            && PREVIEW_H.is_multiple_of(crop_h)
            && PREVIEW_W / crop_w == PREVIEW_H / crop_h
        {
            FilterType::Nearest
        } else {
            FilterType::CatmullRom
        };
        image::imageops::resize(&rgb, PREVIEW_W, PREVIEW_H, filter)
    };

    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(
            fitted.as_raw(),
            PREVIEW_W,
            PREVIEW_H,
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| KernelError::Refused(format!("preview.unreadable: encoding failed: {e}")))?;
    Ok(out)
}

/// The start town's map of the game's world-template recipe ([`templates::start_town`]):
/// the header with that internal name and matrix 0, the map of its first cell in matrix 0 (row
/// by row) and its area data. A matrix 0 without a header layer belongs wholly to the headers
/// that name it, so its first cell with a map counts. Read from disk without unpacking
/// anything. `None` without a recipe (Diamond/Pearl), without such a header, without a cell
/// of it or when a file cannot be read.
pub fn standard_map(project: &Project) -> Option<StandardMap> {
    let town = templates::start_town(project.family)?;
    let names = project.internal_names().ok()?;
    let headers = project.read_headers().ok()?;
    let (header, summary) = headers
        .iter()
        .zip(names)
        .enumerate()
        .find(|(_, (h, name))| name == town && h.matrix_id() == 0)
        .map(|(i, (h, _))| (i, h))?;
    let header = u16::try_from(header).ok()?;
    let matrix = Matrix::read(&member_on_disk(project, AssetKind::Matrix, 0)?).ok()?;
    let map = match &matrix.headers {
        Some(layer) => matrix
            .maps
            .iter()
            .zip(layer)
            .find(|&(&map, &h)| h == header && map != MATRIX_EMPTY)
            .map(|(&map, _)| map),
        None => matrix.maps.iter().copied().find(|&map| map != MATRIX_EMPTY),
    }?;
    Some(StandardMap {
        header,
        map,
        area_data_id: u16::from(summary.area_data_id()),
    })
}

/// Member `index` of `kind` as on disk: from the unpacked folder when it exists (DSPRE keeps
/// it), else from the packed archive. Never unpacks.
fn member_on_disk(project: &Project, kind: AssetKind, index: u16) -> Option<Vec<u8>> {
    if project.unpacked_dir(kind).is_dir() {
        return std::fs::read(project.member_path(kind, index)).ok();
    }
    project
        .read_packed(kind)
        .ok()?
        .members
        .into_iter()
        .nth(usize::from(index))
}

fn slot_path(root: &Path, slot: PreviewSlot) -> PathBuf {
    let name = match slot {
        PreviewSlot::Eigen => "preview.png",
        PreviewSlot::Standard => "preview-standard.png",
    };
    root.join(".g4").join(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    /// `szene/ds/grund` as RGB.
    const GROUND: [u8; 3] = [0x18, 0x21, 0x31];

    fn encode(img: &image::DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    fn png_of(img: RgbaImage) -> Vec<u8> {
        encode(&image::DynamicImage::ImageRgba8(img), ImageFormat::Png)
    }

    fn jpeg_of(img: RgbImage) -> Vec<u8> {
        encode(&image::DynamicImage::ImageRgb8(img), ImageFormat::Jpeg)
    }

    /// Decodes a fitted preview, checking it is a PNG RGB8 of 256 × 192.
    fn decode(png: &[u8]) -> RgbImage {
        assert_eq!(image::guess_format(png).unwrap(), ImageFormat::Png);
        let img = image::load_from_memory_with_format(png, ImageFormat::Png).unwrap();
        assert_eq!(img.color(), image::ColorType::Rgb8);
        assert_eq!((img.width(), img.height()), (PREVIEW_W, PREVIEW_H));
        img.into_rgb8()
    }

    fn code(err: KernelError) -> String {
        match err {
            KernelError::Refused(text) => text.split(':').next().unwrap().to_string(),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    fn fitted_png(colour: [u8; 3]) -> Vec<u8> {
        fit_preview(&png_of(RgbaImage::from_pixel(
            64,
            48,
            Rgba([colour[0], colour[1], colour[2], 255]),
        )))
        .unwrap()
    }

    #[test]
    fn fits_png_4_3() {
        let img = RgbaImage::from_fn(640, 480, |x, _| {
            if x < 320 {
                Rgba([200, 0, 0, 255])
            } else {
                Rgba([0, 0, 200, 255])
            }
        });
        let out = decode(&fit_preview(&png_of(img)).unwrap());
        assert_eq!(out.get_pixel(10, 96).0, [200, 0, 0]);
        assert_eq!(out.get_pixel(245, 96).0, [0, 0, 200]);
    }

    #[test]
    fn crops_square_to_4_3_centre() {
        // 300 × 300 → the centre 300 × 225 (rows 37..262); the red bands above and below it
        // are cut away, the left and right columns stay.
        let img = RgbaImage::from_fn(300, 300, |x, y| {
            if !(37..262).contains(&y) {
                Rgba([255, 0, 0, 255])
            } else if x == 0 || x == 299 {
                Rgba([0, 0, 255, 255])
            } else {
                Rgba([0, 160, 0, 255])
            }
        });
        let out = decode(&fit_preview(&png_of(img)).unwrap());
        assert!(
            out.pixels()
                .all(|p| !(p[0] > 100 && p[1] < 60 && p[2] < 60)),
            "no red is left"
        );
        // The left and right edges keep their blue column.
        assert!(out.get_pixel(0, 96)[2] > 100);
        assert!(out.get_pixel(255, 96)[2] > 100);
    }

    #[test]
    fn fits_jpeg() {
        let out = decode(
            &fit_preview(&jpeg_of(RgbImage::from_pixel(
                400,
                300,
                Rgb([40, 120, 200]),
            )))
            .unwrap(),
        );
        let p = out.get_pixel(128, 96).0;
        for (got, want) in p.iter().zip([40, 120, 200]) {
            assert!(got.abs_diff(want) <= 4, "{p:?}");
        }
    }

    #[test]
    fn fits_grayscale_jpeg() {
        let img = image::DynamicImage::ImageLuma8(image::GrayImage::from_pixel(
            320,
            240,
            image::Luma([90]),
        ));
        let out = decode(&fit_preview(&encode(&img, ImageFormat::Jpeg)).unwrap());
        let p = out.get_pixel(10, 10).0;
        assert!(p.iter().all(|c| c.abs_diff(90) <= 3), "{p:?}");
    }

    #[test]
    fn fits_palette_png() {
        // An indexed PNG with a transparent palette entry (tRNS): the opaque half keeps its
        // colour, the transparent half lies on the ground.
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 4, 3);
            enc.set_color(png::ColorType::Indexed);
            enc.set_depth(png::BitDepth::Eight);
            enc.set_palette(vec![10, 200, 30, 0, 0, 0]);
            enc.set_trns(vec![255, 0]);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1])
                .unwrap();
        }
        let out = decode(&fit_preview(&bytes).unwrap());
        assert_eq!(out.get_pixel(0, 0).0, [10, 200, 30]);
        assert_eq!(out.get_pixel(255, 191).0, GROUND);
    }

    #[test]
    fn detects_by_content_not_extension() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bild.png");
        std::fs::write(
            &file,
            jpeg_of(RgbImage::from_pixel(256, 192, Rgb([0, 0, 0]))),
        )
        .unwrap();
        let preview =
            write_preview(dir.path(), PreviewSlot::Eigen, PreviewInput::File(&file)).unwrap();
        assert_eq!(preview.source, Some(PreviewSource::Eigen));
        decode(&preview.png.unwrap());
    }

    #[test]
    fn alpha_lies_on_ds_ground() {
        let out = decode(
            &fit_preview(&png_of(RgbaImage::from_pixel(
                256,
                192,
                Rgba([255, 255, 255, 0]),
            )))
            .unwrap(),
        );
        assert!(out.pixels().all(|p| p.0 == GROUND));
        assert_eq!(DS_GROUND, GROUND);
    }

    #[test]
    fn upscales_small_image() {
        // 64 × 48 is a whole fraction (×4): every pixel stays sharp.
        let img = RgbaImage::from_fn(64, 48, |x, y| {
            if (x + y) % 2 == 0 {
                Rgba([255, 255, 255, 255])
            } else {
                Rgba([0, 0, 0, 255])
            }
        });
        let out = decode(&fit_preview(&png_of(img)).unwrap());
        assert_eq!(out.get_pixel(0, 0).0, [255, 255, 255]);
        assert_eq!(out.get_pixel(3, 3).0, [255, 255, 255]);
        assert_eq!(out.get_pixel(4, 0).0, [0, 0, 0]);
        // 100 × 75 is no whole fraction and is scaled smoothly, still to 256 × 192.
        decode(
            &fit_preview(&png_of(RgbaImage::from_pixel(
                100,
                75,
                Rgba([1, 2, 3, 255]),
            )))
            .unwrap(),
        );
    }

    #[test]
    fn keeps_exact_size() {
        let img = RgbaImage::from_fn(256, 192, |x, y| Rgba([x as u8, y as u8, 7, 255]));
        let out = decode(&fit_preview(&png_of(img)).unwrap());
        assert_eq!(out.get_pixel(0, 0).0, [0, 0, 7]);
        assert_eq!(out.get_pixel(255, 191).0, [255, 191, 7]);
        assert_eq!(out.get_pixel(17, 42).0, [17, 42, 7]);
    }

    #[test]
    fn refuses_text_as_format() {
        assert_eq!(
            code(fit_preview(b"just some text, no picture").unwrap_err()),
            "preview.format"
        );
        // A format `image` knows, but no PNG or JPEG.
        assert_eq!(
            code(fit_preview(b"GIF89a\x01\x00\x01\x00\x00\x00\x00").unwrap_err()),
            "preview.format"
        );
    }

    #[test]
    fn refuses_too_large_dimensions() {
        // The header of a 9000 × 10 PNG suffices (signature and chunks up to the start of the
        // image data, which the decoder reads its info up to): the size is checked before
        // decoding, so the missing pixels never matter.
        let png = png_of(RgbaImage::new(9000, 10));
        let idat = png.windows(4).position(|w| w == b"IDAT").unwrap();
        let header = &png[..idat + 8];
        assert_eq!(code(fit_preview(header).unwrap_err()), "preview.too_large");
        // A file over 32 MiB is refused before it is read as a picture.
        let big = vec![0u8; 32 * 1024 * 1024 + 1];
        assert_eq!(code(fit_preview(&big).unwrap_err()), "preview.too_large");
        // A broken PNG is unreadable.
        let png = png_of(RgbaImage::new(40, 30));
        assert_eq!(
            code(fit_preview(&png[..png.len() / 2]).unwrap_err()),
            "preview.unreadable"
        );
    }

    #[test]
    fn refuses_missing_file_as_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("fehlt.png");
        let err = write_preview(dir.path(), PreviewSlot::Eigen, PreviewInput::File(&missing))
            .unwrap_err();
        assert_eq!(code(err), "preview.unreadable");
        assert!(!dir.path().join(".g4/preview.png").exists());
    }

    #[test]
    fn read_prefers_eigen_over_standard() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let standard = fitted_png([0, 0, 255]);
        write_preview(root, PreviewSlot::Standard, PreviewInput::Bytes(&standard)).unwrap();
        let read = read_preview(root).unwrap();
        assert_eq!(read.source, Some(PreviewSource::Standard));
        assert_eq!(read.png.as_deref(), Some(standard.as_slice()));

        let own = fitted_png([255, 0, 0]);
        let written = write_preview(root, PreviewSlot::Eigen, PreviewInput::Bytes(&own)).unwrap();
        assert_eq!(written.source, Some(PreviewSource::Eigen));
        assert_eq!(read_preview(root).unwrap(), written);

        // Removing the own picture shows the standard one again.
        let removed = write_preview(root, PreviewSlot::Eigen, PreviewInput::Remove).unwrap();
        assert_eq!(removed.source, Some(PreviewSource::Standard));
        assert!(!root.join(".g4/preview.png").exists());
    }

    #[test]
    fn read_ignores_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".g4")).unwrap();
        std::fs::write(root.join(".g4/preview.png"), b"garbage").unwrap();
        assert_eq!(
            read_preview(root).unwrap(),
            Preview {
                source: None,
                png: None
            }
        );
        // A valid PNG of the wrong size counts as missing too.
        std::fs::write(root.join(".g4/preview.png"), png_of(RgbaImage::new(10, 10))).unwrap();
        assert_eq!(read_preview(root).unwrap().source, None);
        // The standard picture stands in for the broken own one.
        let standard = fitted_png([0, 255, 0]);
        std::fs::write(root.join(".g4/preview-standard.png"), &standard).unwrap();
        let read = read_preview(root).unwrap();
        assert_eq!(read.source, Some(PreviewSource::Standard));
        assert_eq!(read.png, Some(standard));
    }

    #[test]
    fn read_without_g4_dir_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let none = Preview {
            source: None,
            png: None,
        };
        assert_eq!(read_preview(dir.path()).unwrap(), none);
        // A project folder that is gone answers the same.
        assert_eq!(read_preview(&dir.path().join("gone")).unwrap(), none);
    }

    #[test]
    fn write_is_atomic_and_creates_g4() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let bytes = png_of(RgbaImage::from_pixel(512, 384, Rgba([9, 9, 9, 255])));
        let written =
            write_preview(root, PreviewSlot::Standard, PreviewInput::Bytes(&bytes)).unwrap();
        assert_eq!(written.source, Some(PreviewSource::Standard));
        let on_disk = std::fs::read(root.join(".g4/preview-standard.png")).unwrap();
        assert_eq!(written.png, Some(on_disk.clone()));
        decode(&on_disk);
        // Overwriting goes through the same temp file and rename; nothing is left behind.
        let other = fitted_png([1, 1, 1]);
        write_preview(root, PreviewSlot::Standard, PreviewInput::Bytes(&other)).unwrap();
        assert_eq!(
            std::fs::read(root.join(".g4/preview-standard.png")).unwrap(),
            other
        );
        let mut left = Vec::new();
        let mut stack = vec![root.join(".g4")];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(d).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    left.push(path);
                }
            }
        }
        assert_eq!(left, vec![root.join(".g4/preview-standard.png")]);
        // A refused input leaves the file as it was.
        assert!(write_preview(root, PreviewSlot::Standard, PreviewInput::Bytes(b"x")).is_err());
        assert_eq!(
            std::fs::read(root.join(".g4/preview-standard.png")).unwrap(),
            other
        );
    }

    #[test]
    fn write_into_missing_project_folder_fails() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone");
        let bytes = fitted_png([1, 2, 3]);
        assert!(write_preview(&gone, PreviewSlot::Eigen, PreviewInput::Bytes(&bytes)).is_err());
        assert!(!gone.exists());
    }

    #[test]
    fn remove_without_file_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        for slot in [PreviewSlot::Eigen, PreviewSlot::Standard] {
            let read = write_preview(dir.path(), slot, PreviewInput::Remove).unwrap();
            assert_eq!(
                read,
                Preview {
                    source: None,
                    png: None
                }
            );
        }
    }

    #[test]
    fn serde_names() {
        assert_eq!(
            serde_json::to_string(&PreviewSource::Eigen).unwrap(),
            "\"eigen\""
        );
        assert_eq!(
            serde_json::to_string(&PreviewSlot::Standard).unwrap(),
            "\"standard\""
        );
        assert_eq!(PreviewSlot::default(), PreviewSlot::Eigen);
        let map = StandardMap {
            header: 1,
            map: 2,
            area_data_id: 3,
        };
        assert_eq!(
            serde_json::to_string(&map).unwrap(),
            r#"{"header":1,"map":2,"areaDataId":3}"#
        );
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn standard_map_on_synthetic_project() {
        use g4_formats::matrix::Matrix;
        use g4_formats::narc::Narc;

        // Platinum: the recipe names T01, header 0 of matrix 0, area data 0. Matrix 0 of the
        // fixture has no header layer, so every cell is the header's: the first is map 0.
        let fixture = crate::test_support::synthetic_project();
        let root = fixture.dir.path();
        let project = Project::open(root).unwrap();
        assert_eq!(
            standard_map(&project),
            Some(StandardMap {
                header: 0,
                map: 0,
                area_data_id: 0
            })
        );
        // Reading changes nothing on disk: no archive gets unpacked.
        assert!(!root.join("unpacked").exists());

        // With a header layer only the town's own cells count.
        let matrix = Matrix {
            name: "m".into(),
            width: 2,
            height: 1,
            headers: Some(vec![1, 0]),
            altitudes: None,
            maps: vec![2, 1],
        };
        std::fs::write(
            root.join("files/fielddata/mapmatrix/map_matrix.narc"),
            Narc {
                members: vec![matrix.write()],
            }
            .write(),
        )
        .unwrap();
        assert_eq!(
            standard_map(&project),
            Some(StandardMap {
                header: 0,
                map: 1,
                area_data_id: 0
            })
        );
        // An unpacked folder (as DSPRE keeps it) is read in place of the archive.
        let unpacked = project.unpacked_dir(crate::project::AssetKind::Matrix);
        std::fs::create_dir_all(&unpacked).unwrap();
        let matrix = Matrix {
            headers: Some(vec![0, 1]),
            ..matrix
        };
        std::fs::write(unpacked.join("0000"), matrix.write()).unwrap();
        assert_eq!(
            standard_map(&project),
            Some(StandardMap {
                header: 0,
                map: 2,
                area_data_id: 0
            })
        );
        // No cell of the town: none.
        let matrix = Matrix {
            headers: Some(vec![1, 1]),
            ..matrix
        };
        std::fs::write(unpacked.join("0000"), matrix.write()).unwrap();
        assert_eq!(standard_map(&project), None);

        // HeartGold: the fixture's headers have no name T20, so there is no start town.
        let hgss = crate::test_support::synthetic_hgss_project(&["Nowhere"], 2);
        let project = Project::open(hgss.dir.path()).unwrap();
        assert_eq!(standard_map(&project), None);
    }
}
