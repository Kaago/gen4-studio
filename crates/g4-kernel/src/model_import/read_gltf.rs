//! The glTF reader: `gltf` parses `.gltf` and `.glb` (without its `import` feature, so it
//! touches no file); buffers and images are resolved here: the BIN chunk, `data:` URIs
//! (base64) and files only from inside the model's folder ([`resolve_inside`]).
//!
//! `gltf` trusts the byte ranges of a validated document (it slices, asserts and subtracts
//! without checks), so every accessor is checked before it is read ([`check_accessor`]) and
//! the parsing and reading run under `catch_unwind` as a last guard.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use base64::Engine as _;
use gltf::accessor::{DataType, Dimensions};
use gltf::mesh::Mode;
use gltf::{Document, Semantic};

use super::mesh::{Mesh, MeshImage, MeshMaterial, Polygon};
use super::{SourceFormat, decode_image, parts_ignored, resolve_inside, source_unreadable};
use crate::map_import::PartMessage;

/// The reason when the parser panics anyway.
const PANICKED: &str = "the glTF parser failed on this file";

/// How deep nodes may nest; deeper is taken for a cycle.
const MAX_DEPTH: usize = 128;
/// How many node instances the scene may place (a mesh shared along many paths multiplies).
const MAX_INSTANCES: usize = 65_536;
/// How many triangles a model may bring in all (far above what a map holds).
const MAX_POLYGONS: usize = 1_000_000;

/// A node transform, column first (`m[column][row]`) as glTF writes it.
type Matrix = [[f64; 4]; 4];

const IDENTITY: Matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn unreadable(reason: &str) -> PartMessage {
    source_unreadable(SourceFormat::Gltf, reason)
}

/// The mesh of the glTF or GLB at `path` with its notes (model.parts-ignored).
///
/// Scene: the default scene, else scene 0, without a scene every root node. Node matrices are
/// applied (a mesh at two nodes comes twice); a matrix with a negative determinant turns the
/// corner order of its triangles. Primitives `Triangles`, `TriangleStrip` and `TriangleFan`
/// are unfolded to triangles in their winding; points and lines are left out, as are
/// animations, skins, cameras, lights, vertex colours and texture transforms (each named in
/// the hint). uv: the set the base colour texture names, t = v as written. A material's image
/// is its base colour texture (stem: the file's stem, else the image's name, else the
/// material's). Refuses with `model.source-unreadable` (not glTF, a required extension, a
/// primitive without `POSITION`, data that does not fit), `model.file-missing`,
/// `model.file-outside` or `texture.unreadable`.
pub(crate) fn read_gltf(path: &Path) -> Result<(Mesh, Vec<PartMessage>), PartMessage> {
    // `a.gltf` has the parent "" (the current folder).
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let root = folder
        .canonicalize()
        .map_err(|e| unreadable(&e.to_string()))?;
    let bytes = std::fs::read(path).map_err(|e| unreadable(&e.to_string()))?;
    check_glb_header(&bytes).map_err(|reason| unreadable(&reason))?;
    let parsed = catch_unwind(|| gltf::Gltf::from_slice_without_validation(&bytes));
    let gltf::Gltf { document, blob } = match parsed {
        Ok(Ok(gltf)) => gltf,
        Ok(Err(e)) => return Err(unreadable(&e.to_string())),
        Err(_) => return Err(unreadable(PANICKED)),
    };
    // Before validating: the validation would refuse it as well, but without saying why.
    if let Some(name) = document.extensions_required().next() {
        return Err(unreadable(&format!(
            "it requires the extension {name}, which the reader does not know"
        )));
    }
    let validated = catch_unwind(AssertUnwindSafe(|| {
        Document::from_json(document.into_json())
    }));
    let document = match validated {
        Ok(Ok(document)) => document,
        Ok(Err(e)) => return Err(unreadable(&validation_reason(&e))),
        Err(_) => return Err(unreadable(PANICKED)),
    };
    let read = catch_unwind(AssertUnwindSafe(|| {
        let buffers = load_buffers(&document, blob, &root)?;
        Reader::new(&document, &buffers, &root).read()
    }));
    read.unwrap_or_else(|_| Err(unreadable(PANICKED)))
}

/// A GLB header whose length is below its own 12 bytes (`gltf` subtracts it unchecked).
fn check_glb_header(bytes: &[u8]) -> Result<(), String> {
    if !bytes.starts_with(b"glTF") {
        return Ok(());
    }
    let Some(length) = bytes.get(8..12) else {
        return Err("the GLB header is cut short".to_string());
    };
    let length = u32::from_le_bytes([length[0], length[1], length[2], length[3]]);
    if length < 12 {
        return Err(format!(
            "the GLB header names {length} bytes, less than itself"
        ));
    }
    Ok(())
}

/// The first finding of the validation (and how many more), or the error as it says itself.
fn validation_reason(e: &gltf::Error) -> String {
    match e {
        gltf::Error::Validation(findings) => match findings.split_first() {
            Some(((path, error), [])) => format!("{path}: {error}"),
            Some(((path, error), rest)) => {
                format!("{path}: {error} (and {} more)", rest.len())
            }
            None => e.to_string(),
        },
        _ => e.to_string(),
    }
}

/// The bytes a URI names, and the file read (`None` for a `data:` URI) with its name as
/// written (percent-decoded).
struct Fetched {
    bytes: Vec<u8>,
    file: Option<(PathBuf, String)>,
}

/// A `data:` URI decoded, or a file inside `root` read (relative URIs, percent-decoded, are
/// relative to the model's folder).
fn fetch(uri: &str, root: &Path) -> Result<Fetched, PartMessage> {
    if let Some(rest) = strip_prefix_ignoring_case(uri, "data:") {
        let bytes = decode_data(rest).map_err(|reason| unreadable(&reason))?;
        return Ok(Fetched { bytes, file: None });
    }
    let written = percent_decode(uri).map_err(|reason| unreadable(&reason))?;
    let file = resolve_inside(root, root, &written)?;
    let bytes = std::fs::read(&file).map_err(|e| unreadable(&format!("{written}: {e}")))?;
    Ok(Fetched {
        bytes,
        file: Some((file, written)),
    })
}

fn strip_prefix_ignoring_case<'s>(text: &'s str, prefix: &str) -> Option<&'s str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// The payload of a `data:` URI after `data:` (`<type>;base64,<data>`).
fn decode_data(rest: &str) -> Result<Vec<u8>, String> {
    let (header, payload) = rest
        .split_once(',')
        .ok_or_else(|| "a data URI without a comma".to_string())?;
    if !header.to_ascii_lowercase().ends_with(";base64") {
        return Err("a data URI that is not base64".to_string());
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|e| format!("a data URI that is no valid base64 ({e})"))
}

/// `%XX` decoded (glTF URIs are URI-encoded: `tex%20ordner/a.png`); other bytes as they are.
fn percent_decode(uri: &str) -> Result<String, String> {
    let bytes = uri.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[at], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                at += 3;
            }
            (byte, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| format!("the URI {uri} is no UTF-8 once decoded"))
}

/// The data of every buffer, cut to its `byteLength`: the BIN chunk, a `data:` URI or a file.
fn load_buffers(
    document: &Document,
    blob: Option<Vec<u8>>,
    root: &Path,
) -> Result<Vec<Vec<u8>>, PartMessage> {
    let mut out = Vec::new();
    for buffer in document.buffers() {
        let index = buffer.index();
        let mut data = match buffer.source() {
            gltf::buffer::Source::Bin => blob.clone().ok_or_else(|| {
                unreadable(&format!(
                    "buffer {index} has no uri and the file no BIN chunk"
                ))
            })?,
            gltf::buffer::Source::Uri(uri) => fetch(uri, root)?.bytes,
        };
        let length = buffer.length();
        if data.len() < length {
            return Err(unreadable(&format!(
                "buffer {index} holds {} bytes, it names {length}",
                data.len()
            )));
        }
        data.truncate(length);
        out.push(data);
    }
    Ok(out)
}

/// `count` elements of `size` bytes at `offset` in `view` (with its stride): inside the view,
/// the view inside its buffer, no overflow on the way (the formula `gltf` slices with).
fn check_range(
    view: &gltf::buffer::View,
    offset: usize,
    size: usize,
    count: usize,
    buffers: &[Vec<u8>],
) -> Result<(), String> {
    let stride = view.stride().unwrap_or(size);
    if stride < size {
        return Err(format!(
            "buffer view {} has a stride of {stride} bytes for elements of {size}",
            view.index()
        ));
    }
    let past = || format!("its data runs past buffer view {}", view.index());
    let end = count
        .checked_sub(1)
        .and_then(|n| n.checked_mul(stride))
        .and_then(|n| n.checked_add(offset))
        .and_then(|n| n.checked_add(size))
        .ok_or_else(past)?;
    if end > view.length() {
        return Err(past());
    }
    let view_end = view.offset().checked_add(view.length());
    let buffer_len = buffers.get(view.buffer().index()).map(Vec::len);
    match (view_end, buffer_len) {
        (Some(view_end), Some(len)) if view_end <= len => Ok(()),
        _ => Err(format!("buffer view {} runs past its buffer", view.index())),
    }
}

/// An accessor `gltf` may read as `dimensions` of one of `types`: not empty, of that type, its
/// data (and that of its sparse part) inside the buffers. Errors are reasons.
fn check_accessor(
    accessor: &gltf::Accessor,
    types: &[DataType],
    dimensions: Dimensions,
    buffers: &[Vec<u8>],
) -> Result<(), String> {
    let index = accessor.index();
    let count = accessor.count();
    if count == 0 {
        return Err(format!("accessor {index} is empty"));
    }
    if !types.contains(&accessor.data_type()) || accessor.dimensions() != dimensions {
        return Err(format!(
            "accessor {index} holds {:?} of {:?}, expected {dimensions:?} of {types:?}",
            accessor.dimensions(),
            accessor.data_type()
        ));
    }
    let size = accessor.size();
    let at = |reason: String| format!("accessor {index}: {reason}");
    match accessor.sparse() {
        None => {
            let view = accessor
                .view()
                .ok_or_else(|| format!("accessor {index} has no buffer view"))?;
            check_range(&view, accessor.offset(), size, count, buffers).map_err(at)?;
        }
        Some(sparse) => {
            let n = sparse.count();
            if n == 0 || n > count {
                return Err(at(format!("{n} sparse values for {count} elements")));
            }
            if let Some(view) = accessor.view() {
                check_range(&view, accessor.offset(), size, count, buffers).map_err(at)?;
            }
            let indices = sparse.indices();
            let index_size = indices.index_type().size();
            check_range(&indices.view(), indices.offset(), index_size, n, buffers).map_err(at)?;
            let values = sparse.values();
            check_range(&values.view(), values.offset(), size, n, buffers).map_err(at)?;
        }
    }
    Ok(())
}

/// `a · b` (both column first).
fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = [[0.0; 4]; 4];
    for (column, out_column) in out.iter_mut().enumerate() {
        for (row, value) in out_column.iter_mut().enumerate() {
            *value = (0..4).map(|k| a[k][row] * b[column][k]).sum();
        }
    }
    out
}

/// The point `p` moved by `m`.
fn transform(m: &Matrix, p: [f32; 3]) -> [f32; 3] {
    let v = [f64::from(p[0]), f64::from(p[1]), f64::from(p[2]), 1.0];
    let row = |r: usize| (0..4).map(|c| m[c][r] * v[c]).sum::<f64>() as f32;
    [row(0), row(1), row(2)]
}

/// The determinant of the 3 × 3 part (negative: the transform mirrors).
fn determinant(m: &Matrix) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2])
        - m[1][0] * (m[0][1] * m[2][2] - m[2][1] * m[0][2])
        + m[2][0] * (m[0][1] * m[1][2] - m[1][1] * m[0][2])
}

/// The triangles of a primitive's corner list, in its winding (glTF: strip triangle i is
/// `i, i+1, i+2` for even i and `i, i+2, i+1` for odd i; fan triangle i is `i+1, i+2, 0`).
fn unfold(mode: Mode, corners: &[u32]) -> Vec<[u32; 3]> {
    match mode {
        Mode::Triangles => corners.as_chunks::<3>().0.to_vec(),
        Mode::TriangleStrip => corners
            .windows(3)
            .enumerate()
            .map(|(i, w)| {
                if i % 2 == 0 {
                    [w[0], w[1], w[2]]
                } else {
                    [w[0], w[2], w[1]]
                }
            })
            .collect(),
        Mode::TriangleFan => match corners.split_first() {
            Some((&first, rest)) => rest.windows(2).map(|w| [w[0], w[1], first]).collect(),
            None => Vec::new(),
        },
        Mode::Points | Mode::Lines | Mode::LineLoop | Mode::LineStrip => Vec::new(),
    }
}

/// The mesh being filled from one document.
struct Reader<'a> {
    document: &'a Document,
    buffers: &'a [Vec<u8>],
    root: &'a Path,
    mesh: Mesh,
    /// glTF material index → index in `mesh.materials`, given on first use.
    materials: Vec<Option<usize>>,
    /// Tokens of model.parts-ignored.
    ignored: Vec<&'static str>,
}

impl<'a> Reader<'a> {
    fn new(document: &'a Document, buffers: &'a [Vec<u8>], root: &'a Path) -> Reader<'a> {
        Reader {
            document,
            buffers,
            root,
            mesh: Mesh::default(),
            materials: vec![None; document.materials().len()],
            ignored: Vec::new(),
        }
    }

    fn read(mut self) -> Result<(Mesh, Vec<PartMessage>), PartMessage> {
        let document = self.document;
        let scene = document
            .default_scene()
            .or_else(|| document.scenes().next());
        let roots: Vec<gltf::Node> = match scene {
            Some(scene) => scene.nodes().collect(),
            None => {
                let mut is_child = vec![false; document.nodes().len()];
                for node in document.nodes() {
                    for child in node.children() {
                        if let Some(flag) = is_child.get_mut(child.index()) {
                            *flag = true;
                        }
                    }
                }
                document
                    .nodes()
                    .filter(|n| !is_child.get(n.index()).copied().unwrap_or(false))
                    .collect()
            }
        };
        let mut stack: Vec<(gltf::Node, Matrix, usize)> = roots
            .into_iter()
            .rev()
            .map(|node| (node, IDENTITY, 1))
            .collect();
        let mut instances = 0usize;
        while let Some((node, parent, depth)) = stack.pop() {
            instances += 1;
            if instances > MAX_INSTANCES {
                return Err(unreadable(&format!(
                    "the scene places more than {MAX_INSTANCES} nodes"
                )));
            }
            if depth > MAX_DEPTH {
                return Err(unreadable(&format!(
                    "nodes nest deeper than {MAX_DEPTH} levels (a cycle?)"
                )));
            }
            let local = node.transform().matrix().map(|c| c.map(f64::from));
            let matrix = multiply(&parent, &local);
            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    match primitive.mode() {
                        Mode::Points => self.ignore("points"),
                        Mode::Lines | Mode::LineLoop | Mode::LineStrip => self.ignore("lines"),
                        Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan => {
                            self.read_primitive(&mesh, &primitive, &matrix)?;
                        }
                    }
                }
            }
            let children: Vec<gltf::Node> = node.children().collect();
            stack.extend(children.into_iter().rev().map(|c| (c, matrix, depth + 1)));
        }
        if document.animations().next().is_some() {
            self.ignore("animations");
        }
        if document.skins().next().is_some() {
            self.ignore("skins");
        }
        if document.cameras().next().is_some() {
            self.ignore("cameras");
        }
        if document
            .extensions_used()
            .any(|e| e == "KHR_lights_punctual")
        {
            self.ignore("lights");
        }
        if document
            .extensions_used()
            .any(|e| e == "KHR_texture_transform")
        {
            self.ignore("texture-options");
        }
        let notes = parts_ignored(&self.ignored).into_iter().collect();
        Ok((self.mesh, notes))
    }

    fn ignore(&mut self, token: &'static str) {
        if !self.ignored.contains(&token) {
            self.ignored.push(token);
        }
    }

    /// Appends the triangles of one primitive placed by `matrix`.
    fn read_primitive(
        &mut self,
        mesh: &gltf::Mesh,
        primitive: &gltf::Primitive,
        matrix: &Matrix,
    ) -> Result<(), PartMessage> {
        let bad = |reason: String| {
            unreadable(&format!(
                "mesh {} primitive {}: {reason}",
                mesh.index(),
                primitive.index()
            ))
        };
        let buffers = self.buffers;
        let positions = primitive
            .get(&Semantic::Positions)
            .ok_or_else(|| bad("no POSITION".to_string()))?;
        check_accessor(&positions, &[DataType::F32], Dimensions::Vec3, buffers).map_err(bad)?;
        let reader = primitive.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
        let points: Vec<[f32; 3]> = reader
            .read_positions()
            .ok_or_else(|| bad("POSITION has no data".to_string()))?
            .collect();
        let n = points.len();
        if n != positions.count() {
            return Err(bad(format!(
                "POSITION gives {n} of {} positions",
                positions.count()
            )));
        }
        let material = primitive.material();
        let set = material
            .pbr_metallic_roughness()
            .base_color_texture()
            .map_or(0, |info| info.tex_coord());
        let uvs: Option<Vec<[f32; 2]>> = match primitive.get(&Semantic::TexCoords(set)) {
            None => None,
            Some(accessor) => {
                let types = [DataType::F32, DataType::U8, DataType::U16];
                check_accessor(&accessor, &types, Dimensions::Vec2, buffers).map_err(bad)?;
                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(set)
                    .ok_or_else(|| bad(format!("TEXCOORD_{set} has no data")))?
                    .into_f32()
                    .collect();
                if uvs.len() != n {
                    return Err(bad(format!(
                        "TEXCOORD_{set} has {} corners, POSITION {n}",
                        uvs.len()
                    )));
                }
                Some(uvs)
            }
        };
        if primitive.get(&Semantic::Colors(0)).is_some() {
            self.ignore("vertex-colours");
        }
        let corners: Vec<u32> = match primitive.indices() {
            None => {
                let n = u32::try_from(n).map_err(|_| bad("too many positions".to_string()))?;
                (0..n).collect()
            }
            Some(accessor) => {
                let types = [DataType::U8, DataType::U16, DataType::U32];
                check_accessor(&accessor, &types, Dimensions::Scalar, buffers).map_err(bad)?;
                let corners: Vec<u32> = reader
                    .read_indices()
                    .ok_or_else(|| bad("the indices have no data".to_string()))?
                    .into_u32()
                    .collect();
                if corners.len() != accessor.count() {
                    return Err(bad("the indices are cut short".to_string()));
                }
                corners
            }
        };
        if let Some(&index) = corners.iter().find(|&&i| i as usize >= n) {
            return Err(bad(format!("index {index} past the {n} positions")));
        }
        let triangles = unfold(primitive.mode(), &corners);
        if self.mesh.polygons.len().saturating_add(triangles.len()) > MAX_POLYGONS {
            return Err(unreadable(&format!(
                "the model has more than {MAX_POLYGONS} triangles"
            )));
        }
        let material = self.material(&material)?;

        let too_many = || bad("more corners than an index holds".to_string());
        let base = u32::try_from(self.mesh.positions.len()).map_err(|_| too_many())?;
        for point in points {
            let placed = transform(matrix, point);
            if !placed.iter().all(|v| v.is_finite()) {
                return Err(bad(
                    "a position is no finite number once the nodes are applied".to_string(),
                ));
            }
            self.mesh.positions.push(placed);
        }
        let uv_base = match uvs {
            Some(uvs) => {
                let first = u32::try_from(self.mesh.uvs.len()).map_err(|_| too_many())?;
                self.mesh.uvs.extend(uvs);
                Some(first)
            }
            None => None,
        };
        let mirrored = determinant(matrix) < 0.0;
        for triangle in triangles {
            let mut corners = Vec::with_capacity(3);
            for index in triangle {
                let position = base.checked_add(index).ok_or_else(too_many)?;
                let uv = match uv_base {
                    Some(first) => Some(first.checked_add(index).ok_or_else(too_many)?),
                    None => None,
                };
                corners.push((position, uv));
            }
            if mirrored {
                corners.reverse();
            }
            self.mesh.polygons.push(Polygon { material, corners });
        }
        Ok(())
    }

    /// The index in `mesh.materials` of a glTF material (added on first use); `None` for the
    /// default material of a primitive without one.
    fn material(&mut self, material: &gltf::Material) -> Result<Option<usize>, PartMessage> {
        let Some(index) = material.index() else {
            return Ok(None);
        };
        if let Some(Some(at)) = self.materials.get(index) {
            return Ok(Some(*at));
        }
        let name = material.name().unwrap_or_default().to_string();
        let image = match material.pbr_metallic_roughness().base_color_texture() {
            Some(info) => Some(self.image(info.texture().source().index(), &name)?),
            None => None,
        };
        self.mesh.materials.push(MeshMaterial { name, image });
        let at = self.mesh.materials.len() - 1;
        if let Some(slot) = self.materials.get_mut(index) {
            *slot = Some(at);
        }
        Ok(Some(at))
    }

    /// Image `index` decoded: from its uri (file or `data:`) or its buffer view.
    fn image(&self, index: usize, material: &str) -> Result<MeshImage, PartMessage> {
        let json = self
            .document
            .as_json()
            .images
            .get(index)
            .ok_or_else(|| unreadable(&format!("no image {index}")))?;
        let name = json.name.clone();
        let (bytes, file) = match (&json.uri, &json.buffer_view) {
            (Some(uri), _) => {
                let fetched = fetch(uri, self.root)?;
                (fetched.bytes, fetched.file)
            }
            (None, Some(view)) => {
                if json.mime_type.is_none() {
                    return Err(unreadable(&format!(
                        "image {index} lies in a buffer view without mimeType"
                    )));
                }
                (self.view_bytes(view.value(), index)?, None)
            }
            (None, None) => {
                return Err(unreadable(&format!(
                    "image {index} has neither uri nor bufferView"
                )));
            }
        };
        let label = match (&file, &name) {
            (Some((_, written)), _) => written.clone(),
            (None, Some(name)) => name.clone(),
            (None, None) => format!("image {index}"),
        };
        let rgba = decode_image(&bytes, &label)?;
        let stem = match (&file, name) {
            (Some((_, written)), _) => Path::new(&written.replace('\\', "/"))
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            (None, Some(name)) => name,
            (None, None) => material.to_string(),
        };
        Ok(MeshImage {
            stem,
            file: file.map(|(path, _)| path),
            rgba,
        })
    }

    /// The bytes of buffer view `view` (holding image `image`).
    fn view_bytes(&self, view: usize, image: usize) -> Result<Vec<u8>, PartMessage> {
        let outside = || unreadable(&format!("image {image} runs past its buffer"));
        let view = self.document.views().nth(view).ok_or_else(outside)?;
        let end = view
            .offset()
            .checked_add(view.length())
            .ok_or_else(outside)?;
        self.buffers
            .get(view.buffer().index())
            .and_then(|data| data.get(view.offset()..end))
            .map(<[u8]>::to_vec)
            .ok_or_else(outside)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::*;
    use crate::map_import::PartCode;
    use crate::model_import::mesh::{Binding, Polygon, to_faces};
    use crate::model_import::read_model;
    use crate::model_import::samples::{PLANE_GLB, PLANE_GLTF, write_samples};

    /// The samples in a fresh folder: (its guard, the folder).
    fn samples() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("modell");
        write_samples(&root).unwrap();
        (dir, root)
    }

    /// The JSON of the plane `.gltf` (naming `g4_imp_plane.bin` and `g4_imp_plane.png`).
    fn plane_json(root: &Path) -> Value {
        serde_json::from_slice(&std::fs::read(root.join(PLANE_GLTF)).unwrap()).unwrap()
    }

    fn write_json(dir: &Path, name: &str, document: &Value) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, serde_json::to_vec(document).unwrap()).unwrap();
        path
    }

    fn read(path: &Path) -> (Mesh, Vec<PartMessage>) {
        match read_gltf(path) {
            Ok(read) => read,
            Err(e) => panic!("{} refused: {e:?}", path.display()),
        }
    }

    fn refusal(path: &Path) -> PartMessage {
        match read_gltf(path) {
            Ok((mesh, _)) => panic!("read {} polygons", mesh.polygons.len()),
            Err(e) => e,
        }
    }

    fn positions_of(mesh: &Mesh, polygon: &Polygon) -> Vec<[f32; 3]> {
        polygon
            .corners
            .iter()
            .map(|c| mesh.positions[c.0 as usize])
            .collect()
    }

    /// `(b − a) × (c − a)` of the first three corners.
    fn normal_of(p: &[[f32; 3]]) -> [f32; 3] {
        let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
        let v = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    fn data_uri(mime: &str, bytes: &[u8]) -> String {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        format!("data:{mime};base64,{encoded}")
    }

    /// A document with one primitive of `mode` over `positions` (in a `data:` buffer), with
    /// `indices` (u16) if given; no material, no uv.
    fn geometry(positions: &[[f32; 3]], mode: u32, indices: Option<&[u16]>) -> Value {
        let mut bytes: Vec<u8> = positions
            .iter()
            .flatten()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let positions_len = bytes.len();
        let mut views = vec![json!({"buffer": 0, "byteOffset": 0, "byteLength": positions_len})];
        let mut accessors = vec![json!({
            "bufferView": 0,
            "componentType": 5126,
            "count": positions.len(),
            "type": "VEC3",
            "min": [-10.0, -10.0, -10.0],
            "max": [10.0, 10.0, 10.0]
        })];
        let mut primitive = json!({"attributes": {"POSITION": 0}, "mode": mode});
        if let Some(indices) = indices {
            bytes.extend(indices.iter().flat_map(|i| i.to_le_bytes()));
            views.push(json!({
                "buffer": 0,
                "byteOffset": positions_len,
                "byteLength": indices.len() * 2
            }));
            accessors.push(json!({
                "bufferView": 1,
                "componentType": 5123,
                "count": indices.len(),
                "type": "SCALAR"
            }));
            primitive["indices"] = json!(1);
        }
        json!({
            "asset": {"version": "2.0"},
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [primitive]}],
            "accessors": accessors,
            "bufferViews": views,
            "buffers": [{
                "uri": data_uri("application/octet-stream", &bytes),
                "byteLength": bytes.len()
            }]
        })
    }

    #[test]
    fn the_plane_glb_reads_as_two_triangles_with_its_image() {
        let (_guard, root) = samples();
        let (mesh, notes) = read(&root.join(PLANE_GLB));
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(mesh.positions.len(), 4);
        assert_eq!(mesh.polygons.len(), 2);
        for polygon in &mesh.polygons {
            assert_eq!(polygon.corners.len(), 3);
            assert_eq!(polygon.material, Some(0));
            assert!(polygon.corners.iter().all(|c| c.1.is_some()));
        }
        // glTF t is v as written (t down already): (0, 0) .. (4, 4).
        assert_eq!(mesh.uvs, [[0.0, 0.0], [0.0, 4.0], [4.0, 4.0], [4.0, 0.0]]);
        assert_eq!(mesh.materials.len(), 1);
        assert_eq!(mesh.materials[0].name, "g4_imp_plane_mat");
        let image = mesh.materials[0].image.as_ref().unwrap();
        assert_eq!(image.stem, "g4_imp_plane");
        assert_eq!(image.file, None);
        assert_eq!((image.rgba.width, image.rgba.height), (16, 16));
        assert_eq!(image.rgba.pixels.len(), 16 * 16 * 4);
        // read_model takes the same way for `.glb`.
        assert_eq!(read_model(&root.join(PLANE_GLB)).unwrap().0, mesh);
    }

    #[test]
    fn the_plane_gltf_with_files_equals_the_glb() {
        let (_guard, root) = samples();
        let (glb, _) = read(&root.join(PLANE_GLB));
        let (mut gltf, notes) = read(&root.join(PLANE_GLTF));
        assert!(notes.is_empty(), "{notes:?}");
        let image = gltf.materials[0].image.as_mut().unwrap();
        let file = image.file.take().unwrap();
        assert!(file.ends_with("modell/g4_imp_plane.png"), "{file:?}");
        assert_eq!(gltf, glb);
    }

    #[test]
    fn data_uris_are_read() {
        let (_guard, root) = samples();
        let (glb, _) = read(&root.join(PLANE_GLB));
        let mut document = plane_json(&root);
        let bin = std::fs::read(root.join("g4_imp_plane.bin")).unwrap();
        let png = std::fs::read(root.join("g4_imp_plane.png")).unwrap();
        document["buffers"][0]["uri"] = json!(data_uri("application/octet-stream", &bin));
        document["images"][0]["uri"] = json!(data_uri("image/png", &png));
        // A folder of its own: nothing but the `.gltf` is there.
        let dir = tempfile::tempdir().unwrap();
        let (mesh, notes) = read(&write_json(dir.path(), "daten.gltf", &document));
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(mesh, glb);
    }

    #[test]
    fn node_transforms_are_applied_and_a_mirror_flips_the_winding() {
        let (_guard, root) = samples();
        let (plain, _) = read(&root.join(PLANE_GLTF));
        let mut document = plane_json(&root);
        document["nodes"][0]["translation"] = json!([1.0, 0.0, 2.0]);
        document["nodes"][0]["scale"] = json!([2.0, 2.0, 2.0]);
        let (moved, _) = read(&write_json(&root, "verschoben.gltf", &document));
        let expected: Vec<[f32; 3]> = plain
            .positions
            .iter()
            .map(|p| [2.0 * p[0] + 1.0, 2.0 * p[1], 2.0 * p[2] + 2.0])
            .collect();
        assert_eq!(moved.positions, expected);
        assert_eq!(moved.polygons, plain.polygons);

        // A child node multiplies with its parent: the same placement in two steps.
        document["nodes"][0] = json!({"translation": [1.0, 0.0, 2.0], "children": [1]});
        let child = json!({"name": "kind", "mesh": 0, "scale": [2.0, 2.0, 2.0]});
        document["nodes"].as_array_mut().unwrap().push(child);
        let (nested, _) = read(&write_json(&root, "verschachtelt.gltf", &document));
        assert_eq!(nested.positions, expected);

        let mut document = plane_json(&root);
        document["nodes"][0]["scale"] = json!([-1.0, 1.0, 1.0]);
        let (mirrored, _) = read(&write_json(&root, "gespiegelt.gltf", &document));
        assert_eq!(mirrored.polygons.len(), plain.polygons.len());
        for (m, p) in mirrored.polygons.iter().zip(&plain.polygons) {
            let mut reversed = positions_of(&plain, p)
                .into_iter()
                .map(|[x, y, z]| [-x, y, z])
                .collect::<Vec<_>>();
            reversed.reverse();
            assert_eq!(positions_of(&mirrored, m), reversed);
            assert!(normal_of(&positions_of(&mirrored, m))[1] > 0.0);
        }
        // After to_faces the front still faces up.
        let binding = Binding {
            texture: "g4_imp_plane".into(),
            width: 16,
            height: 16,
        };
        let converted = to_faces(&mirrored, 16.0, &[Some(binding)]).unwrap();
        assert!(!converted.faces.is_empty());
        for face in &converted.faces {
            let corners: Vec<[f32; 3]> = face.corners.iter().map(|c| c.map(|v| v as f32)).collect();
            assert!(normal_of(&corners)[1] > 0.0, "{face:?}");
        }
    }

    #[test]
    fn a_mesh_at_two_nodes_is_imported_twice() {
        let (_guard, root) = samples();
        let mut document = plane_json(&root);
        document["nodes"] = json!([
            {"mesh": 0},
            {"mesh": 0, "translation": [5.0, 0.0, 0.0]}
        ]);
        document["scenes"] = json!([{"nodes": [0, 1]}]);
        let (mesh, _) = read(&write_json(&root, "zweimal.gltf", &document));
        assert_eq!(mesh.polygons.len(), 4);
        assert_eq!(mesh.materials.len(), 1, "one material, used twice");
        let second = positions_of(&mesh, &mesh.polygons[2]);
        let first = positions_of(&mesh, &mesh.polygons[0]);
        for (a, b) in first.iter().zip(&second) {
            assert_eq!([a[0] + 5.0, a[1], a[2]], *b);
        }
    }

    #[test]
    fn without_a_scene_every_root_node_is_imported() {
        let (_guard, root) = samples();
        let mut document = plane_json(&root);
        document.as_object_mut().unwrap().remove("scene");
        document.as_object_mut().unwrap().remove("scenes");
        document["nodes"] = json!([
            {"children": [1]},
            {"mesh": 0},
            {"mesh": 0, "translation": [5.0, 0.0, 0.0]}
        ]);
        let (mesh, _) = read(&write_json(&root, "ohne-szene.gltf", &document));
        // Node 0 (with its child 1) and node 2 are roots: two planes.
        assert_eq!(mesh.polygons.len(), 4);
        // With a default scene only its nodes count.
        document["scenes"] = json!([{"nodes": [2]}, {"nodes": [0, 2]}]);
        document["scene"] = json!(1);
        let (mesh, _) = read(&write_json(&root, "szene-1.gltf", &document));
        assert_eq!(mesh.polygons.len(), 4);
        document.as_object_mut().unwrap().remove("scene");
        let (mesh, _) = read(&write_json(&root, "szene-0.gltf", &document));
        assert_eq!(mesh.polygons.len(), 2, "scene 0 without a default scene");
    }

    #[test]
    fn a_sparse_accessor_is_read() {
        let (_guard, root) = samples();
        let (plain, _) = read(&root.join(PLANE_GLTF));
        // One sparse value: index 0 (the first u16 of the index view) becomes the second
        // position (12 bytes into the position view).
        let mut document = plane_json(&root);
        document["accessors"][0]["sparse"] = json!({
            "count": 1,
            "indices": {"bufferView": 2, "componentType": 5123},
            "values": {"bufferView": 0, "byteOffset": 12}
        });
        let (mesh, _) = read(&write_json(&root, "sparse.gltf", &document));
        assert_eq!(mesh.positions[0], plain.positions[1]);
        assert_eq!(mesh.positions[1..], plain.positions[1..]);
    }

    #[test]
    fn strips_and_fans_are_unfolded() {
        let dir = tempfile::tempdir().unwrap();
        let square = [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
        ];
        let (mesh, _) = read(&write_json(
            dir.path(),
            "strip.gltf",
            &geometry(&square, 5, None),
        ));
        assert_eq!(mesh.polygons.len(), 2);
        let normals: Vec<[f32; 3]> = mesh
            .polygons
            .iter()
            .map(|p| normal_of(&positions_of(&mesh, p)))
            .collect();
        assert!(normals.iter().all(|n| n[1] > 0.0), "{normals:?}");
        // Triangle 1 of a strip is (1, 3, 2): the winding of triangle 0.
        let corners: Vec<u32> = mesh.polygons[1].corners.iter().map(|c| c.0).collect();
        assert_eq!(corners, [1, 3, 2]);
        assert!(mesh.polygons.iter().all(|p| p.material.is_none()));

        // A fan of 5 around its first corner, with indices.
        let fan = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, -1.0],
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, -1.0],
        ];
        let (mesh, _) = read(&write_json(
            dir.path(),
            "fan.gltf",
            &geometry(&fan, 6, Some(&[0, 1, 2, 3, 4])),
        ));
        assert_eq!(mesh.polygons.len(), 3);
        let normals: Vec<[f32; 3]> = mesh
            .polygons
            .iter()
            .map(|p| normal_of(&positions_of(&mesh, p)))
            .collect();
        assert!(normals.iter().all(|n| n[1] > 0.0), "{normals:?}");
        let corners: Vec<u32> = mesh.polygons[0].corners.iter().map(|c| c.0).collect();
        assert_eq!(corners, [1, 2, 0]);
    }

    #[test]
    fn lines_and_points_are_ignored_with_a_note() {
        let (_guard, root) = samples();
        let mut document = plane_json(&root);
        let lines = json!({"attributes": {"POSITION": 0}, "indices": 2, "mode": 1});
        document["meshes"][0]["primitives"]
            .as_array_mut()
            .unwrap()
            .push(lines);
        let (mesh, notes) = read(&write_json(&root, "linien.gltf", &document));
        assert_eq!(mesh.polygons.len(), 2, "the plane stays");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].code, PartCode::ModelPartsIgnored);
        assert_eq!(notes[0].args, ["lines"]);

        // An animation as well (its times: two floats of the uv view).
        document["accessors"]
            .as_array_mut()
            .unwrap()
            .push(json!({"bufferView": 1, "componentType": 5126, "count": 2, "type": "SCALAR"}));
        document["animations"] = json!([{
            "channels": [{"sampler": 0, "target": {"node": 0, "path": "translation"}}],
            "samplers": [{"input": 3, "output": 0}]
        }]);
        let (mesh, notes) = read(&write_json(&root, "animiert.gltf", &document));
        assert_eq!(mesh.polygons.len(), 2);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].args, ["lines,animations"]);

        // Points, vertex colours, skins, cameras, lights, texture transforms: in table order.
        let points = json!({"attributes": {"POSITION": 0}, "mode": 0});
        document["meshes"][0]["primitives"]
            .as_array_mut()
            .unwrap()
            .push(points);
        document["meshes"][0]["primitives"][0]["attributes"]["COLOR_0"] = json!(0);
        document["cameras"] =
            json!([{"type": "perspective", "perspective": {"yfov": 1.0, "znear": 0.1}}]);
        document["skins"] = json!([{"joints": [0]}]);
        document["extensionsUsed"] = json!(["KHR_lights_punctual", "KHR_texture_transform"]);
        let (mesh, notes) = read(&write_json(&root, "alles.gltf", &document));
        assert_eq!(mesh.polygons.len(), 2);
        assert_eq!(
            notes[0].args,
            ["lines,points,animations,skins,cameras,lights,vertex-colours,texture-options"]
        );
    }

    #[test]
    fn gltf_uris_outside_the_folder_or_with_a_scheme_are_refused() {
        let (guard, root) = samples();
        // Files that would be read if the reader followed the references.
        std::fs::copy(root.join("g4_imp_plane.bin"), guard.path().join("a.bin")).unwrap();
        std::fs::copy(root.join("g4_imp_plane.png"), guard.path().join("a.png")).unwrap();
        let outside_png = guard.path().join("a.png").display().to_string();
        let cases: [(&str, &str); 6] = [
            ("buffers", "../a.bin"),
            ("buffers", "https://example.com/a.bin"),
            ("buffers", "file:///etc/passwd"),
            ("images", "/abs/a.png"),
            ("images", "..%2Fa.png"),
            ("images", outside_png.as_str()),
        ];
        for (list, uri) in cases {
            let mut document = plane_json(&root);
            document[list][0]["uri"] = json!(uri);
            let e = refusal(&write_json(&root, "draussen.gltf", &document));
            assert_eq!(e.code, PartCode::ModelFileOutside, "{uri}: {e:?}");
        }
        let mut document = plane_json(&root);
        document["buffers"][0]["uri"] = json!("../a.bin");
        let e = refusal(&write_json(&root, "draussen.gltf", &document));
        assert_eq!(e.args, ["../a.bin"]);

        // A symlink in the folder to a file outside.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(guard.path().join("a.bin"), root.join("link.bin")).unwrap();
            let mut document = plane_json(&root);
            document["buffers"][0]["uri"] = json!("link.bin");
            let e = refusal(&write_json(&root, "link.gltf", &document));
            assert_eq!(e.code, PartCode::ModelFileOutside);
        }

        // Missing, and inside in a subfolder (percent-encoded as glTF URIs are).
        let mut document = plane_json(&root);
        document["images"][0]["uri"] = json!("fehlt.png");
        let e = refusal(&write_json(&root, "fehlt.gltf", &document));
        assert_eq!(
            (e.code, e.args.as_slice()),
            (
                PartCode::ModelFileMissing,
                ["fehlt.png".to_string()].as_slice()
            )
        );
        std::fs::create_dir(root.join("tex ordner")).unwrap();
        std::fs::copy(
            root.join("g4_imp_plane.png"),
            root.join("tex ordner/stein.png"),
        )
        .unwrap();
        document["images"][0]["uri"] = json!("tex%20ordner/stein.png");
        let (mesh, _) = read(&write_json(&root, "unterordner.gltf", &document));
        assert_eq!(mesh.materials[0].image.as_ref().unwrap().stem, "stein");
    }

    #[test]
    fn required_extensions_and_missing_positions_are_source_unreadable() {
        let (_guard, root) = samples();
        let mut document = plane_json(&root);
        document["extensionsUsed"] = json!(["KHR_draco_mesh_compression"]);
        document["extensionsRequired"] = json!(["KHR_draco_mesh_compression"]);
        let e = refusal(&write_json(&root, "draco.gltf", &document));
        assert_eq!(e.code, PartCode::ModelSourceUnreadable);
        assert_eq!(e.args[0], "glTF");
        assert!(e.args[1].contains("KHR_draco_mesh_compression"), "{e:?}");

        let mut document = plane_json(&root);
        document["meshes"][0]["primitives"][0]["attributes"] = json!({"TEXCOORD_0": 1});
        let e = refusal(&write_json(&root, "ohne-position.gltf", &document));
        assert_eq!(e.code, PartCode::ModelSourceUnreadable);
        assert_eq!(e.args[0], "glTF");
    }

    /// A change to a document.
    type Edit = Box<dyn Fn(&mut Value)>;

    /// Edits of the plane that a reader must refuse with model.source-unreadable, not a panic.
    #[test]
    fn broken_documents_are_source_unreadable_not_a_panic() {
        let (_guard, root) = samples();
        let edits: Vec<(&str, Edit)> = vec![
            (
                "count past the view",
                Box::new(|d| d["accessors"][0]["count"] = json!(1000)),
            ),
            (
                "huge count",
                Box::new(|d| d["accessors"][0]["count"] = json!(u64::MAX / 2)),
            ),
            (
                "count 0",
                Box::new(|d| d["accessors"][2]["count"] = json!(0)),
            ),
            (
                "u16 positions",
                Box::new(|d| d["accessors"][0]["componentType"] = json!(5123)),
            ),
            (
                "vec2 positions",
                Box::new(|d| d["accessors"][0]["type"] = json!("VEC2")),
            ),
            (
                "float indices",
                Box::new(|d| d["accessors"][2]["componentType"] = json!(5126)),
            ),
            (
                "vec3 uv",
                Box::new(|d| d["accessors"][1]["type"] = json!("VEC3")),
            ),
            (
                "offset past",
                Box::new(|d| d["accessors"][0]["byteOffset"] = json!(4000)),
            ),
            (
                "huge offset",
                Box::new(|d| d["bufferViews"][0]["byteOffset"] = json!(u64::MAX / 2)),
            ),
            (
                "view past buffer",
                Box::new(|d| d["bufferViews"][2]["byteLength"] = json!(400)),
            ),
            (
                "short stride",
                Box::new(|d| d["bufferViews"][0]["byteStride"] = json!(4)),
            ),
            (
                "buffer longer than its file",
                Box::new(|d| d["buffers"][0]["byteLength"] = json!(500)),
            ),
            (
                "no accessor data",
                Box::new(|d| {
                    d["accessors"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("bufferView");
                }),
            ),
            (
                "index past positions",
                Box::new(|d| {
                    d["accessors"][2]["count"] = json!(4);
                    d["bufferViews"][1]["byteLength"] = json!(32);
                    d["accessors"][2]["bufferView"] = json!(1);
                    d["accessors"][2]["componentType"] = json!(5125);
                }),
            ),
            (
                "uv count differs",
                Box::new(|d| d["accessors"][1]["count"] = json!(3)),
            ),
            (
                "sparse with 0",
                Box::new(|d| {
                    d["accessors"][0]["sparse"] = json!({
                        "count": 0,
                        "indices": {"bufferView": 2, "componentType": 5123},
                        "values": {"bufferView": 0}
                    });
                }),
            ),
            (
                "sparse past",
                Box::new(|d| {
                    d["accessors"][0]["sparse"] = json!({
                        "count": 50,
                        "indices": {"bufferView": 2, "componentType": 5123},
                        "values": {"bufferView": 0}
                    });
                }),
            ),
            (
                "sparse values past their view",
                Box::new(|d| {
                    d["accessors"][0]["sparse"] = json!({
                        "count": 2,
                        "indices": {"bufferView": 2, "componentType": 5123},
                        "values": {"bufferView": 2}
                    });
                }),
            ),
            (
                "node cycle",
                Box::new(|d| d["nodes"][0]["children"] = json!([0])),
            ),
            (
                "image view without mime type",
                Box::new(|d| {
                    d["images"][0] = json!({"bufferView": 0});
                }),
            ),
            (
                "image without source",
                Box::new(|d| d["images"][0] = json!({"name": "leer"})),
            ),
            (
                "data uri not base64",
                Box::new(|d| {
                    d["buffers"][0]["uri"] = json!("data:application/octet-stream,abc");
                }),
            ),
            (
                "data uri broken",
                Box::new(|d| {
                    d["buffers"][0]["uri"] = json!("data:application/octet-stream;base64,@@@");
                }),
            ),
            (
                "no BIN chunk",
                Box::new(|d| {
                    d["buffers"][0].as_object_mut().unwrap().remove("uri");
                }),
            ),
            (
                "bad mode",
                Box::new(|d| d["meshes"][0]["primitives"][0]["mode"] = json!(9)),
            ),
            (
                "not finite",
                Box::new(|d| d["nodes"][0]["scale"] = json!([3e38, 3e38, 3e38])),
            ),
        ];
        for (what, edit) in edits {
            let mut document = plane_json(&root);
            edit(&mut document);
            let e = refusal(&write_json(&root, "kaputt.gltf", &document));
            assert_eq!(e.code, PartCode::ModelSourceUnreadable, "{what}: {e:?}");
            assert_eq!(e.args[0], "glTF", "{what}");
            assert_ne!(e.args[1], PANICKED, "{what}: the parser panicked");
        }
        // Not JSON at all, and a file that is not there.
        std::fs::write(root.join("kein.gltf"), b"{ kein json").unwrap();
        assert_eq!(
            refusal(&root.join("kein.gltf")).code,
            PartCode::ModelSourceUnreadable
        );
        assert_eq!(
            refusal(&root.join("weg.glb")).code,
            PartCode::ModelSourceUnreadable
        );
    }

    /// Every cut of the GLB and every byte flipped in its headers: refused (or read), never a
    /// panic inside `gltf`.
    #[test]
    fn a_cut_or_damaged_glb_is_source_unreadable_not_a_panic() {
        let (_guard, root) = samples();
        let glb = std::fs::read(root.join(PLANE_GLB)).unwrap();
        let path = root.join("kaputt.glb");
        let check = |bytes: &[u8], what: &str| {
            std::fs::write(&path, bytes).unwrap();
            if let Err(e) = read_gltf(&path) {
                assert_ne!(e.args.get(1).map(String::as_str), Some(PANICKED), "{what}");
            }
        };
        for cut in 0..glb.len() {
            check(&glb[..cut], &format!("cut at {cut}"));
        }
        for at in (0..28).chain(glb.len() - 400..glb.len()) {
            for value in [0u8, 0xFF, 0x0B] {
                let mut bytes = glb.clone();
                bytes[at] = value;
                check(&bytes, &format!("byte {at} = {value}"));
            }
        }
        // Header lengths below the header itself.
        let mut bytes = glb.clone();
        bytes[8..12].copy_from_slice(&3u32.to_le_bytes());
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(refusal(&path).code, PartCode::ModelSourceUnreadable);
    }
}
