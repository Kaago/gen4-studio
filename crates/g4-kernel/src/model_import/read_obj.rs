//! The OBJ reader: `tobj` parses OBJ and MTL, every file next to the model is read only from
//! inside the model's folder ([`resolve_inside`]).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use super::mesh::{Mesh, MeshImage, MeshMaterial, Polygon};
use super::{SourceFormat, decode_image, parts_ignored, resolve_inside, source_unreadable};
use crate::map_import::{PartCode, PartMessage};

/// A material library as loaded: its folder (images are relative to it) and how many
/// materials it brought.
struct Library {
    folder: PathBuf,
    materials: usize,
}

/// The mesh of the OBJ at `path`, its materials with their images (only those a face uses, in
/// the order of first use); the note model.parts-ignored `texture-options` when a used
/// material's `map_Kd` has options (they are not applied). Refuses with
/// `model.source-unreadable` (OBJ or MTL unreadable), `model.file-missing`,
/// `model.file-outside` (a material library or image) or `texture.unreadable`.
///
/// OBJ: v up is t down (`t = 1 − v`); quads and n-gons stay as written (`face_arities` empty
/// means triangles only); points and lines are left out. The models `tobj` splits the file
/// into are merged, their indices shifted. A model whose corners have uv only in part keeps
/// no uv (`to_faces` then says `model.uv-missing`). A `usemtl` name no library defines is a
/// material of that name without image (the pack texture of the name may draw it).
pub(crate) fn read_obj(path: &Path) -> Result<(Mesh, Vec<PartMessage>), PartMessage> {
    let unreadable = |reason: &str| source_unreadable(SourceFormat::Obj, reason);
    // `a.obj` has the parent "" (the current folder).
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let root = folder
        .canonicalize()
        .map_err(|e| unreadable(&e.to_string()))?;
    let text = std::fs::read_to_string(path).map_err(|e| unreadable(&e.to_string()))?;
    let names = check_obj(&text).map_err(|reason| unreadable(&reason))?;
    // The `usemtl` names come first as a library of their own (no image), so a name without
    // `newmtl` keeps its name; a real library loaded later overrides the name (tobj's merge).
    let text = if names.is_empty() {
        text
    } else {
        format!("mtllib {NAMES_LIBRARY}\n{text}")
    };
    let names_pending = Cell::new(!names.is_empty());
    let refusal: RefCell<Option<PartMessage>> = RefCell::new(None);
    let libraries: RefCell<Vec<Library>> = RefCell::new(Vec::new());
    let options = tobj::LoadOptions {
        single_index: true,
        triangulate: false,
        ignore_points: true,
        ignore_lines: true,
    };
    let loader = |reference: &Path| {
        if names_pending.replace(false) {
            libraries.borrow_mut().push(Library {
                folder: root.clone(),
                materials: names.len(),
            });
            return Ok(names_library(&names));
        }
        let written = reference.to_string_lossy();
        let library = resolve_inside(&root, &root, &written).and_then(|mtl| {
            let text = std::fs::read_to_string(&mtl)
                .map_err(|e| unreadable(&format!("{written}: {e}")))?;
            let text = normalise_mtl(&text);
            let parsed = catch_unwind(AssertUnwindSafe(|| {
                tobj::load_mtl_buf(&mut Cursor::new(text.as_bytes()))
            }));
            let (materials, names) = match parsed {
                Ok(Ok(library)) => library,
                Ok(Err(e)) => return Err(unreadable(&format!("{written}: {e}"))),
                Err(_) => return Err(unreadable(&format!("{written}: {PANICKED}"))),
            };
            let folder = mtl.parent().map_or_else(|| root.clone(), Path::to_path_buf);
            Ok((folder, materials, names))
        });
        match library {
            Ok((folder, materials, names)) => {
                libraries.borrow_mut().push(Library {
                    folder,
                    materials: materials.len(),
                });
                Ok((materials, names))
            }
            Err(e) => {
                refusal.borrow_mut().get_or_insert(e);
                Err(tobj::LoadError::OpenFileFailed)
            }
        }
    };
    // tobj is checked against the inputs it panics on (`check_obj`, `normalise_mtl`); a panic
    // left in it still only refuses the file.
    let loaded = catch_unwind(AssertUnwindSafe(|| {
        tobj::load_obj_buf(&mut Cursor::new(text.as_bytes()), &options, loader)
    }));
    if let Some(e) = refusal.into_inner() {
        return Err(e);
    }
    let loaded = loaded.map_err(|_| unreadable(PANICKED))?;
    let (models, materials) = loaded.map_err(|e| unreadable(&e.to_string()))?;
    let materials = materials.map_err(|e| unreadable(&e.to_string()))?;
    let libraries = libraries.into_inner();

    let mut mesh = Mesh::default();
    // tobj material index → index in `mesh.materials`, given on first use.
    let mut used: Vec<Option<usize>> = vec![None; materials.len()];
    let mut options_ignored = false;
    for model in &models {
        let m = &model.mesh;
        let material = match m.material_id {
            Some(id) if id < materials.len() => {
                let index = match used[id] {
                    Some(index) => index,
                    None => {
                        let folder = folder_of(&libraries, id).unwrap_or(root.as_path());
                        let (material, had_options) = material_of(&materials[id], &root, folder)?;
                        options_ignored |= had_options;
                        mesh.materials.push(material);
                        let index = mesh.materials.len() - 1;
                        used[id] = Some(index);
                        index
                    }
                };
                Some(index)
            }
            _ => None,
        };
        append_model(&mut mesh, m, material).map_err(|reason| unreadable(&reason))?;
    }
    let notes = if options_ignored {
        parts_ignored(&["texture-options"]).into_iter().collect()
    } else {
        Vec::new()
    };
    Ok((mesh, notes))
}

/// The name the library of the `usemtl` names is asked for by (only its first load counts).
const NAMES_LIBRARY: &str = "g4-studio-usemtl-names";

/// The reason when tobj panics anyway.
const PANICKED: &str = "the OBJ parser failed on this file";

/// The `usemtl` names of `text` in first-use order, after checking every corner index of the
/// `f`, `l` and `p` lines against the `v`, `vt`, `vn` lines before it: tobj overflows on an
/// index past them (`f 1/-9223372036854775808 …`) instead of refusing. Errors are reasons for
/// `model.source-unreadable`.
fn check_obj(text: &str) -> Result<Vec<String>, String> {
    let mut counts = [0usize; 3];
    let mut names: Vec<String> = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("v") => counts[0] += 1,
            Some("vt") => counts[1] += 1,
            Some("vn") => counts[2] += 1,
            Some("f" | "l" | "p") => {
                for corner in words {
                    for (kind, part) in corner.split('/').enumerate().take(3) {
                        let Ok(index) = part.parse::<isize>() else {
                            continue; // empty (`1//1`) or not a number: tobj refuses that
                        };
                        let count = counts[kind];
                        if index.unsigned_abs() > count {
                            let what = ["positions", "texture coordinates", "normals"][kind];
                            return Err(format!("corner index {index} past the {count} {what}"));
                        }
                    }
                }
            }
            // As tobj reads the name: everything after the first space.
            Some("usemtl") => {
                let name = line.split_once(' ').unwrap_or_default().1.trim();
                if !name.is_empty() && !names.iter().any(|n| n == name) {
                    names.push(name.to_string());
                }
            }
            _ => {}
        }
    }
    Ok(names)
}

/// A library with one material without image per name.
fn names_library(names: &[String]) -> (Vec<tobj::Material>, HashMap<String, usize>) {
    let materials = names
        .iter()
        .map(|name| tobj::Material {
            name: name.clone(),
            ..tobj::Material::default()
        })
        .collect();
    let map = names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect();
    (materials, map)
}

/// `text` with every colour line (`Ka`, `Kd`, `Ks`, `Ke`) of one or two numbers filled to
/// three (MTL: g and b default to r) and without the ones of no value: tobj panics on fewer
/// than three. Colours are not imported; other lines stay as they are.
fn normalise_mtl(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let keyword = words.next();
        let values: Vec<&str> = words.collect();
        let colour = matches!(keyword, Some("Ka" | "Kd" | "Ks" | "Ke"));
        let numbers = values.iter().all(|v| v.parse::<f32>().is_ok());
        match (keyword, values.len()) {
            (_, 0) if colour => continue,
            (Some(keyword), 1 | 2) if colour && numbers => {
                let r = values[0];
                let g = values.get(1).copied().unwrap_or(r);
                out.push_str(&format!("{keyword} {r} {g} {r}"));
            }
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    out
}

/// The folder of the library tobj material `id` came from (libraries are appended in order).
fn folder_of(libraries: &[Library], id: usize) -> Option<&Path> {
    let mut first = 0;
    for library in libraries {
        if id < first + library.materials {
            return Some(&library.folder);
        }
        first += library.materials;
    }
    None
}

/// A material with its image (`map_Kd`, relative to the folder of its library), and whether
/// the `map_Kd` had options in front of the file.
fn material_of(
    material: &tobj::Material,
    root: &Path,
    folder: &Path,
) -> Result<(MeshMaterial, bool), PartMessage> {
    let mut had_options = false;
    let image = match &material.diffuse_texture {
        Some(texture) => {
            let (written, options) = texture_file(texture);
            had_options = options;
            let file = resolve_inside(root, folder, written)?;
            let bytes = std::fs::read(&file).map_err(|e| {
                let reason = e.to_string();
                PartMessage::new(
                    PartCode::TextureUnreadable,
                    &[&written, &reason],
                    format!("the image {written} cannot be read ({reason})"),
                )
            })?;
            let rgba = decode_image(&bytes, written)?;
            let stem = Path::new(&written.replace('\\', "/"))
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            Some(MeshImage {
                stem,
                file: Some(file),
                rgba,
            })
        }
        None => None,
    };
    let material = MeshMaterial {
        name: material.name.clone(),
        image,
    };
    Ok((material, had_options))
}

/// The file of a `map_Kd` value and whether options stood in front: with options
/// (`-s 1 1 1 bild.png`) the last word, else all of it (a file name may hold spaces). The
/// options themselves are not applied.
fn texture_file(value: &str) -> (&str, bool) {
    let value = value.trim();
    if value.starts_with('-') {
        (value.split_whitespace().last().unwrap_or(value), true)
    } else {
        (value, false)
    }
}

/// Appends the positions, uvs and polygons of one tobj model (single index: one index for
/// position and uv). Errors are reasons for `model.source-unreadable`.
fn append_model(mesh: &mut Mesh, m: &tobj::Mesh, material: Option<usize>) -> Result<(), String> {
    let too_many = || "more corners than an index holds".to_string();
    let vertices = m.positions.len() / 3;
    let base = u32::try_from(mesh.positions.len()).map_err(|_| too_many())?;
    mesh.positions
        .extend(m.positions.as_chunks::<3>().0.iter().copied());
    // uv only when every corner has one (tobj leaves out the missing ones).
    let uv_base = if !m.texcoords.is_empty() && m.texcoords.len() / 2 == vertices {
        let first = u32::try_from(mesh.uvs.len()).map_err(|_| too_many())?;
        mesh.uvs.extend(
            m.texcoords
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[s, t]| [s, 1.0 - t]),
        );
        Some(first)
    } else {
        None
    };
    let arities: Vec<usize> = if m.face_arities.is_empty() {
        if !m.indices.len().is_multiple_of(3) {
            return Err("a face with fewer than 3 corners".to_string());
        }
        vec![3; m.indices.len() / 3]
    } else {
        m.face_arities.iter().map(|&n| n as usize).collect()
    };
    let mut at = 0usize;
    for arity in arities {
        let end = at.checked_add(arity).ok_or_else(too_many)?;
        let indices = m
            .indices
            .get(at..end)
            .ok_or_else(|| "the faces name more corners than there are".to_string())?;
        let mut corners = Vec::with_capacity(arity);
        for &index in indices {
            if index as usize >= vertices {
                return Err(format!("corner {index} past the {vertices} positions"));
            }
            let position = base.checked_add(index).ok_or_else(too_many)?;
            let uv = match uv_base {
                Some(first) => Some(first.checked_add(index).ok_or_else(too_many)?),
                None => None,
            };
            corners.push((position, uv));
        }
        mesh.polygons.push(Polygon { material, corners });
        at = end;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_import::read_model;
    use crate::model_import::samples::{CUBE_OBJ, PDSMS_OBJ, write_samples};

    /// The samples in a fresh folder: (its guard, the folder).
    fn samples() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("modell");
        write_samples(&root).unwrap();
        (dir, root)
    }

    fn refusal(path: &Path) -> PartMessage {
        match read_obj(path) {
            Ok((mesh, _)) => panic!("read {} polygons", mesh.polygons.len()),
            Err(e) => e,
        }
    }

    fn uv_of(mesh: &Mesh, corner: (u32, Option<u32>)) -> [f32; 2] {
        mesh.uvs[corner.1.unwrap() as usize]
    }

    #[test]
    fn the_cube_reads_as_six_quads_with_one_textured_material() {
        let (_guard, root) = samples();
        let (mesh, notes) = read_obj(&root.join(CUBE_OBJ)).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(mesh.polygons.len(), 6);
        for polygon in &mesh.polygons {
            assert_eq!(polygon.corners.len(), 4);
            assert_eq!(polygon.material, Some(0));
            assert!(polygon.corners.iter().all(|c| c.1.is_some()));
        }
        assert_eq!(mesh.materials.len(), 1);
        let material = &mesh.materials[0];
        assert_eq!(material.name, "g4_imp_cube_mat");
        let image = material.image.as_ref().unwrap();
        assert_eq!(image.stem, "g4_imp_cube");
        assert_eq!((image.rgba.width, image.rgba.height), (16, 16));
        assert_eq!(image.rgba.pixels.len(), 16 * 16 * 4);
        assert!(
            image
                .file
                .as_ref()
                .unwrap()
                .ends_with("modell/g4_imp_cube.png")
        );
        // The bottom face `f 1/1/1 2/2/1 …`: corner 0 is `v -0.5 0 -0.5` with `vt 0 0`.
        let first = mesh.polygons[0].corners[0];
        assert_eq!(mesh.positions[first.0 as usize], [-0.5, 0.0, -0.5]);
        assert_eq!(uv_of(&mesh, first), [0.0, 1.0], "t points down");
        assert_eq!(uv_of(&mesh, mesh.polygons[0].corners[2]), [1.0, 0.0]);
        // read_model takes the same way for `.obj`.
        assert_eq!(read_model(&root.join(CUBE_OBJ)).unwrap().0, mesh);
    }

    #[test]
    fn the_pdsms_style_obj_reads_with_two_materials() {
        let (_guard, root) = samples();
        let (mesh, _) = read_obj(&root.join(PDSMS_OBJ)).unwrap();
        assert_eq!(mesh.polygons.len(), 64);
        assert!(mesh.polygons.iter().all(|p| p.corners.len() == 4));
        let names: Vec<&str> = mesh.materials.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["gras", "weg"]);
        for (index, material) in mesh.materials.iter().enumerate() {
            let image = material.image.as_ref().unwrap();
            assert_eq!(image.stem, format!("g4_pds_{}", material.name));
            let count = mesh
                .polygons
                .iter()
                .filter(|p| p.material == Some(index))
                .count();
            assert_eq!(count, 32, "{}", material.name);
        }
        // Indices of the merged models stay inside the merged lists.
        for polygon in &mesh.polygons {
            for &(position, uv) in &polygon.corners {
                assert!((position as usize) < mesh.positions.len());
                assert!((uv.unwrap() as usize) < mesh.uvs.len());
            }
        }
    }

    #[test]
    fn negative_indices_and_missing_uv_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("quad.obj");
        std::fs::write(&path, "v 0 0 0\nv 1 0 0\nv 1 0 1\nv 0 0 1\nf -4 -3 -2 -1\n").unwrap();
        let (mesh, notes) = read_obj(&path).unwrap();
        assert!(notes.is_empty());
        assert_eq!(mesh.polygons.len(), 1);
        let polygon = &mesh.polygons[0];
        assert_eq!(polygon.material, None);
        let positions: Vec<[f32; 3]> = polygon
            .corners
            .iter()
            .map(|c| mesh.positions[c.0 as usize])
            .collect();
        assert_eq!(
            positions,
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0]
            ]
        );
        assert!(polygon.corners.iter().all(|c| c.1.is_none()));
        assert!(mesh.uvs.is_empty() && mesh.materials.is_empty());
    }

    #[test]
    fn triangles_only_and_no_faces_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tri.obj");
        std::fs::write(
            &path,
            "v 0 0 0\nv 1 0 0\nv 0 0 1\nvt 0 0\nf 1/1 3/1 2/1\nf 1/1 2/1 3/1\n",
        )
        .unwrap();
        let (mesh, _) = read_obj(&path).unwrap();
        assert_eq!(mesh.polygons.len(), 2);
        assert!(mesh.polygons.iter().all(|p| p.corners.len() == 3));
        assert_eq!(uv_of(&mesh, mesh.polygons[0].corners[0]), [0.0, 1.0]);
        // No face at all: an empty mesh; `to_faces` says model.no-geometry.
        std::fs::write(&path, "v 0 0 0\nv 1 0 0\nl 1 2\np 1\n").unwrap();
        let (mesh, _) = read_obj(&path).unwrap();
        assert!(mesh.polygons.is_empty());
    }

    #[test]
    fn references_outside_the_folder_are_refused() {
        let (guard, root) = samples();
        let obj = root.join(CUBE_OBJ);
        let mtl = root.join("g4_imp_cube.mtl");
        let cube = std::fs::read_to_string(&obj).unwrap();
        let outside = guard.path().join("x.mtl");
        std::fs::write(&outside, "newmtl g4_imp_cube_mat\nKd 1 1 1\n").unwrap();
        let with_mtllib = |line: &str| cube.replace("mtllib g4_imp_cube.mtl", line);

        let cases: [(&str, PartCode, &str); 6] = [
            ("mtllib ../x.mtl", PartCode::ModelFileOutside, "../x.mtl"),
            ("mtllib ..\\x.mtl", PartCode::ModelFileOutside, "..\\x.mtl"),
            (
                "mtllib file:x.mtl",
                PartCode::ModelFileOutside,
                "file:x.mtl",
            ),
            (
                "mtllib sub/../../x.mtl",
                PartCode::ModelFileOutside,
                "sub/../../x.mtl",
            ),
            ("mtllib fehlt.mtl", PartCode::ModelFileMissing, "fehlt.mtl"),
            // The folder itself is no file.
            ("mtllib .", PartCode::ModelFileMissing, "."),
        ];
        for (line, code, arg) in cases {
            std::fs::write(&obj, with_mtllib(line)).unwrap();
            let e = refusal(&obj);
            assert_eq!(
                (e.code, e.args.as_slice()),
                (code, [arg.to_string()].as_slice()),
                "{line}"
            );
        }
        let absolute = format!("mtllib {}", outside.display());
        std::fs::write(&obj, with_mtllib(&absolute)).unwrap();
        assert_eq!(refusal(&obj).code, PartCode::ModelFileOutside);
        std::fs::write(&obj, &cube).unwrap();

        // Images: relative to the MTL, same rules.
        let image_cases = [
            "map_Kd /tmp/x.png",
            "map_Kd https://example.com/x.png",
            "map_Kd ../x.png",
            "map_Kd C:\\textures\\x.png",
        ];
        for line in image_cases {
            std::fs::write(&mtl, format!("newmtl g4_imp_cube_mat\n{line}\n")).unwrap();
            let e = refusal(&obj);
            assert_eq!(e.code, PartCode::ModelFileOutside, "{line}");
            assert_eq!(e.args, [line.trim_start_matches("map_Kd ")], "{line}");
        }

        // A symlink in the folder to a file outside.
        #[cfg(unix)]
        {
            let png = guard.path().join("x.png");
            std::fs::copy(root.join("g4_imp_cube.png"), &png).unwrap();
            std::os::unix::fs::symlink(&png, root.join("link.png")).unwrap();
            std::fs::write(&mtl, "newmtl g4_imp_cube_mat\nmap_Kd link.png\n").unwrap();
            let e = refusal(&obj);
            assert_eq!(
                (e.code, e.args.as_slice()),
                (
                    PartCode::ModelFileOutside,
                    ["link.png".to_string()].as_slice()
                )
            );
            std::os::unix::fs::symlink(&outside, root.join("link.mtl")).unwrap();
            std::fs::write(&obj, with_mtllib("mtllib link.mtl")).unwrap();
            assert_eq!(refusal(&obj).code, PartCode::ModelFileOutside);
            std::fs::write(&obj, &cube).unwrap();
        }

        // Inside stays allowed: a subfolder, `..` back into the folder, options before the file.
        std::fs::create_dir(root.join("tex")).unwrap();
        std::fs::copy(root.join("g4_imp_cube.png"), root.join("tex/stein.png")).unwrap();
        for (line, stem, options) in [
            ("map_Kd tex/../g4_imp_cube.png", "g4_imp_cube", false),
            ("map_Kd tex\\stein.png", "stein", false),
            ("map_Kd -s 1 1 1 g4_imp_cube.png", "g4_imp_cube", true),
            ("map_Kd -blendu on -o 0 0 tex/stein.png", "stein", true),
        ] {
            std::fs::write(&mtl, format!("newmtl g4_imp_cube_mat\n{line}\n")).unwrap();
            let (mesh, notes) = read_obj(&obj).unwrap();
            let image = mesh.materials[0].image.as_ref().unwrap();
            assert_eq!(image.stem, stem, "{line}");
            // Options in front of the file are not applied: the hint says so, once.
            let expected: Vec<(PartCode, Vec<String>)> = if options {
                vec![(PartCode::ModelPartsIgnored, vec!["texture-options".into()])]
            } else {
                Vec::new()
            };
            let got: Vec<(PartCode, Vec<String>)> =
                notes.into_iter().map(|n| (n.code, n.args)).collect();
            assert_eq!(got, expected, "{line}");
        }
    }

    #[test]
    fn a_broken_obj_is_source_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kaputt.obj");
        std::fs::write(&path, "v 0 0 0\nv 1 0 0\nv 0 0 1\nf 1 2 x\n").unwrap();
        let e = refusal(&path);
        assert_eq!(e.code, PartCode::ModelSourceUnreadable);
        assert_eq!(e.args[0], "OBJ");
        // An index past the positions, and a file that is not there.
        std::fs::write(&path, "v 0 0 0\nf 1 2 3\n").unwrap();
        assert_eq!(refusal(&path).code, PartCode::ModelSourceUnreadable);
        let e = refusal(&dir.path().join("weg.obj"));
        assert_eq!(
            (e.code, e.args[0].as_str()),
            (PartCode::ModelSourceUnreadable, "OBJ")
        );
        // A broken material library is the model's source as well.
        std::fs::write(&path, "mtllib a.mtl\nv 0 0 0\nv 1 0 0\nv 0 0 1\nf 1 2 3\n").unwrap();
        std::fs::write(dir.path().join("a.mtl"), "newmtl a\nKd 1 x\n").unwrap();
        let e = refusal(&path);
        assert_eq!(e.code, PartCode::ModelSourceUnreadable);
        assert!(e.args[1].contains("a.mtl"), "{e:?}");
    }

    #[test]
    fn a_broken_png_is_texture_unreadable() {
        let (_guard, root) = samples();
        std::fs::write(
            root.join("g4_imp_cube.png"),
            [0x13u8, 7, 0, 0xFF, 1, 2, 3, 4],
        )
        .unwrap();
        let e = refusal(&root.join(CUBE_OBJ));
        assert_eq!(e.code, PartCode::TextureUnreadable);
        assert_eq!(e.args[0], "g4_imp_cube.png");
        assert!(!e.args[1].is_empty());
    }

    #[test]
    fn only_used_materials_come_into_the_mesh() {
        let (_guard, root) = samples();
        // A second material whose image is broken but which no face uses.
        let mtl = root.join("g4_imp_cube.mtl");
        let text = std::fs::read_to_string(&mtl).unwrap();
        std::fs::write(&mtl, format!("newmtl unbenutzt\nmap_Kd kaputt.png\n{text}")).unwrap();
        std::fs::write(root.join("kaputt.png"), b"kein Bild").unwrap();
        let (mesh, _) = read_obj(&root.join(CUBE_OBJ)).unwrap();
        let names: Vec<&str> = mesh.materials.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["g4_imp_cube_mat"]);
        assert!(mesh.polygons.iter().all(|p| p.material == Some(0)));
    }

    #[test]
    fn short_colour_lines_in_the_mtl_do_not_panic() {
        let (_guard, root) = samples();
        std::fs::write(
            root.join("g4_imp_cube.mtl"),
            "newmtl g4_imp_cube_mat\nKd 1\nKa 0.2\nKs\nKe 0 0\nmap_Kd g4_imp_cube.png\n",
        )
        .unwrap();
        let (mesh, _) = read_obj(&root.join(CUBE_OBJ)).unwrap();
        assert_eq!(
            mesh.materials[0].image.as_ref().unwrap().stem,
            "g4_imp_cube"
        );
        assert_eq!(
            normalise_mtl("Kd 1\nKs\nKe 0 0.5\nKa 1 x\n"),
            "Kd 1 1 1\nKe 0 0.5 0\nKa 1 x\n"
        );
    }

    #[test]
    fn corner_indices_past_the_lists_are_source_unreadable_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.obj");
        let head = "v 0 0 0\nv 1 0 0\nv 0 0 1\nvt 0 0\nvn 0 1 0\n";
        for face in [
            "f 1/-9223372036854775808 2/1 3/1",
            "f 1/-2 2/1 3/1",
            "f 1/2 2/1 3/1",
            "f 1//9223372036854775807 2//1 3//1",
            "f -4 2 3",
            "f 1 2 4",
            "l 1/-9223372036854775808 2",
        ] {
            std::fs::write(&path, format!("{head}{face}\n")).unwrap();
            let e = refusal(&path);
            assert_eq!(e.code, PartCode::ModelSourceUnreadable, "{face}");
        }
        // Counted at the line: a later `vt` does not make an earlier index valid.
        std::fs::write(&path, "v 0 0 0\nv 1 0 0\nv 0 0 1\nf 1/1 2/1 3/1\nvt 0 0\n").unwrap();
        assert_eq!(refusal(&path).code, PartCode::ModelSourceUnreadable);
        std::fs::write(&path, format!("{head}f -3/-1/-1 -2/1/1 -1/1/1\n")).unwrap();
        assert_eq!(read_obj(&path).unwrap().0.polygons.len(), 1);
    }

    #[test]
    fn a_usemtl_name_without_library_keeps_its_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stein.obj");
        let faces = "v 0 0 0\nv 1 0 0\nv 0 0 1\nvt 0 0\nusemtl stein\nf 1/1 2/1 3/1\n\
                     usemtl fels\nf 1/1 3/1 2/1\nusemtl stein\nf 2/1 1/1 3/1\n";
        std::fs::write(&path, faces).unwrap();
        let (mesh, _) = read_obj(&path).unwrap();
        assert_eq!(
            mesh.materials,
            [
                MeshMaterial {
                    name: "stein".into(),
                    image: None
                },
                MeshMaterial {
                    name: "fels".into(),
                    image: None
                },
            ]
        );
        let materials: Vec<Option<usize>> = mesh.polygons.iter().map(|p| p.material).collect();
        assert_eq!(materials, [Some(0), Some(1), Some(0)]);

        // A library that defines the name wins; the other name stays without image.
        let (_guard, root) = samples();
        std::fs::copy(root.join("g4_imp_cube.png"), dir.path().join("stein.png")).unwrap();
        std::fs::write(dir.path().join("a.mtl"), "newmtl stein\nmap_Kd stein.png\n").unwrap();
        std::fs::write(&path, format!("mtllib a.mtl\n{faces}")).unwrap();
        let (mesh, _) = read_obj(&path).unwrap();
        assert_eq!(mesh.materials.len(), 2);
        assert_eq!(mesh.materials[0].name, "stein");
        assert_eq!(mesh.materials[0].image.as_ref().unwrap().stem, "stein");
        assert_eq!(
            mesh.materials[1],
            MeshMaterial {
                name: "fels".into(),
                image: None
            }
        );
    }
}
