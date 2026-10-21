use std::path::Path;

use g4_kernel::error::KernelError;
use g4_kernel::export::{glb_cache_key, glb_cached, nsbmd_to_glb};

#[test]
fn missing_apicula_is_tool_missing() {
    let out = tempfile::tempdir().unwrap();
    let err =
        nsbmd_to_glb(Path::new("/nonexistent/apicula"), b"BMD0", None, out.path()).unwrap_err();
    assert!(matches!(
        err,
        KernelError::ToolMissing {
            name: "apicula",
            ..
        }
    ));
}

#[test]
fn cache_key_depends_on_model_and_tileset() {
    assert_eq!(glb_cache_key(b"BMD0", None).len(), 16);
    assert_ne!(
        glb_cache_key(b"BMD0", None),
        glb_cache_key(b"BMD0", Some(b"BTX0"))
    );
    assert_eq!(glb_cache_key(b"BMD0", None), glb_cache_key(b"BMD0", None));
}

#[cfg(unix)]
mod fake_tool {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    /// Writes an executable shell script standing in for apicula.
    fn fake(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("fake-apicula");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    // apicula's argv: convert -f=glb <input> [<nsbtx>] -o <dir>; the script finds the directory
    // after `-o` and creates <dir>/<name>/<name>.glb.
    const FIND_OUT: &str = r#"while [ $# -gt 0 ]; do [ "$1" = "-o" ] && out="$2"; shift; done"#;
    const PRODUCE: &str = r#"while [ $# -gt 0 ]; do [ "$1" = "-o" ] && out="$2"; shift; done
mkdir -p "$out/model"; printf 'glTFxxxx' > "$out/model/model.glb""#;

    /// A fake apicula that counts its runs in a file; returns the script and the counter file.
    fn fake_apicula_counting(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let counter = dir.join("count");
        let body = format!(
            "{FIND_OUT}\necho x >> '{c}'\nmkdir -p \"$out/model\"; printf 'glTFxxxx' > \"$out/model/model.glb\"",
            c = counter.display()
        );
        (fake(dir, &body), counter)
    }

    fn runs(counter: &Path) -> usize {
        std::fs::read_to_string(counter).map_or(0, |t| t.lines().count())
    }

    #[test]
    fn returns_the_single_glb_the_tool_produced() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let tool = fake(tools.path(), PRODUCE);
        let glb = nsbmd_to_glb(&tool, b"BMD0", None, out.path()).unwrap();
        assert_eq!(glb.extension().unwrap(), "glb");
        assert!(glb.starts_with(out.path()));
        assert_eq!(&std::fs::read(&glb).unwrap()[0..4], b"glTF");
    }

    #[test]
    fn non_zero_exit_reports_stderr() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let tool = fake(tools.path(), "echo boom >&2; exit 3");
        let err = nsbmd_to_glb(&tool, b"BMD0", None, out.path()).unwrap_err();
        match err {
            KernelError::ToolMissing { name, reason } => {
                assert_eq!(name, "apicula");
                assert!(reason.contains("boom"), "{reason}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn no_glb_or_several_glbs_is_an_error() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let none = fake(tools.path(), "exit 0");
        assert!(nsbmd_to_glb(&none, b"BMD0", None, out.path()).is_err());
        let two = fake(
            tools.path(),
            &format!(
                r#"{FIND_OUT}; mkdir -p "$out/a" "$out/b"; : > "$out/a/a.glb"; : > "$out/b/b.glb""#
            ),
        );
        assert!(nsbmd_to_glb(&two, b"BMD0", None, out.path()).is_err());
    }

    #[test]
    fn tileset_is_passed_as_second_input_file() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        // Fails unless argv is: convert -f=glb <x>.nsbmd <y>.nsbtx -o <dir> with both files present.
        let tool = fake(
            tools.path(),
            &format!(
                r#"case "$3" in *.nsbmd) ;; *) echo "bad model arg $3" >&2; exit 2;; esac
case "$4" in *.nsbtx) ;; *) echo "bad tileset arg $4" >&2; exit 2;; esac
[ "$(cat "$4")" = "BTX0tiles" ] || {{ echo "bad tileset content" >&2; exit 2; }}
[ "$5" = "-o" ] || {{ echo "bad -o" >&2; exit 2; }}
{FIND_OUT}
mkdir -p "$out/model"; printf 'glTFxxxx' > "$out/model/model.glb""#
            ),
        );
        nsbmd_to_glb(&tool, b"BMD0", Some(b"BTX0tiles"), out.path()).unwrap();
        // Without a tileset the same script must be rejected (no fourth argument).
        assert!(nsbmd_to_glb(&tool, b"BMD0", None, out.path()).is_err());
    }

    #[test]
    fn glb_cached_hits_the_cache_second_time() {
        let tools = tempfile::tempdir().unwrap();
        let (apicula, counter) = fake_apicula_counting(tools.path());
        let dir = tempfile::tempdir().unwrap();
        let a = glb_cached(dir.path(), &apicula, b"BMD0one", None).unwrap();
        let b = glb_cached(dir.path(), &apicula, b"BMD0one", None).unwrap();
        assert_eq!(a, b);
        assert_eq!(runs(&counter), 1);
        // Only the cache file remains: no work directories or temp files.
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, [format!("{}.glb", glb_cache_key(b"BMD0one", None))]);
        // A different tileset is a different cache entry.
        glb_cached(dir.path(), &apicula, b"BMD0one", Some(b"BTX0")).unwrap();
        assert_eq!(runs(&counter), 2);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn glb_cached_serves_hits_without_the_tool() {
        let dir = tempfile::tempdir().unwrap();
        let key = glb_cache_key(b"BMD0", Some(b"BTX0"));
        std::fs::write(dir.path().join(format!("{key}.glb")), b"glTFcached").unwrap();
        let glb = glb_cached(
            dir.path(),
            Path::new("/nonexistent/apicula"),
            b"BMD0",
            Some(b"BTX0"),
        )
        .unwrap();
        assert_eq!(glb, b"glTFcached");
        // A miss needs the tool.
        let err =
            glb_cached(dir.path(), Path::new("/nonexistent/apicula"), b"BMD1", None).unwrap_err();
        assert!(matches!(err, KernelError::ToolMissing { .. }));
    }

    #[test]
    fn failed_conversion_leaves_no_files_in_the_cache_dir() {
        let tools = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let failing = fake(tools.path(), "echo boom >&2; exit 3");
        assert!(glb_cached(dir.path(), &failing, b"BMD0", Some(b"BTX0")).is_err());
        let none = fake(tools.path(), "exit 0");
        assert!(glb_cached(dir.path(), &none, b"BMD0", None).is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[cfg(feature = "test-support")]
mod project_tileset {
    use g4_formats::map::MapFile;
    use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
    use g4_kernel::project::{AssetKind, Project};

    use super::*;

    fn glb_json(glb: &[u8]) -> serde_json::Value {
        assert_eq!(&glb[0..4], b"glTF");
        let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&glb[20..20 + json_len]).unwrap()
    }

    fn image_count(glb: &[u8]) -> usize {
        glb_json(glb)["images"].as_array().map_or(0, |a| a.len())
    }

    #[test]
    fn map_tileset_bytes_follows_area_data_and_tolerates_out_of_range() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let p = Project::open(fixture.dir.path()).unwrap();
        // Area data 0 selects map tileset 0 (the only texture set); area data 1 selects
        // tileset 2, which does not exist; area data 5 does not exist either.
        assert_eq!(
            p.map_tileset_bytes(0).unwrap().as_deref(),
            Some(&b"BTX0\0\0\0\0"[..])
        );
        assert_eq!(p.map_tileset_bytes(1).unwrap(), None);
        assert_eq!(p.map_tileset_bytes(5).unwrap(), None);
    }

    /// Map model and texture set of a vanilla header, working on a copy of the project
    /// (unpacking writes `unpacked/`; the original is never touched).
    fn vanilla_model_and_tileset(
        header: usize,
    ) -> Option<(tempfile::TempDir, Vec<u8>, Option<Vec<u8>>)> {
        let (copy, root) = g4_kernel::test_support::vanilla_copy()?;
        let p = Project::open(&root).unwrap();
        let headers = p.headers().unwrap();
        let h = &headers[header.min(headers.len() - 1)];
        p.ensure_unpacked(AssetKind::Matrix).unwrap();
        p.ensure_unpacked(AssetKind::Map).unwrap();
        let matrix =
            Matrix::read(&std::fs::read(p.member_path(AssetKind::Matrix, h.matrix_id)).unwrap())
                .unwrap();
        let map_index = *matrix.maps.iter().find(|&&m| m != MATRIX_EMPTY).unwrap();
        let map = MapFile::read(
            &std::fs::read(p.member_path(AssetKind::Map, map_index)).unwrap(),
            p.family,
        )
        .unwrap();
        let tex = p.map_tileset_bytes(h.area_data_id).unwrap();
        Some((copy, map.model, tex))
    }

    #[test]
    fn vanilla_textured_map_glb_has_images() {
        let Some(apicula) = std::env::var_os("G4_APICULA") else {
            eprintln!("skipped: needs G4_APICULA");
            return;
        };
        let Some((_copy, model, tex)) = vanilla_model_and_tileset(415) else {
            return;
        };
        assert!(tex.is_some());
        let cache = tempfile::tempdir().unwrap();
        let glb = glb_cached(cache.path(), Path::new(&apicula), &model, tex.as_deref()).unwrap();
        assert!(image_count(&glb) > 0);
    }

    #[test]
    fn glb_cached_without_tileset_still_converts() {
        let Some(apicula) = std::env::var_os("G4_APICULA") else {
            eprintln!("skipped: needs G4_APICULA");
            return;
        };
        let Some((_copy, model, _tex)) = vanilla_model_and_tileset(415) else {
            return;
        };
        let cache = tempfile::tempdir().unwrap();
        let glb = glb_cached(cache.path(), Path::new(&apicula), &model, None).unwrap();
        assert_eq!(image_count(&glb), 0);
    }
}

#[test]
fn vanilla_map_model_converts_to_glb() {
    let (Some(root), Some(apicula)) = (
        std::env::var_os("G4_TEST_PROJECT"),
        std::env::var_os("G4_APICULA"),
    ) else {
        eprintln!("skipped: needs G4_TEST_PROJECT and G4_APICULA");
        return;
    };
    // Read-only: unpack into a copy so the original project is never written.
    let copy = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("cp")
        .arg("-R")
        .arg(&root)
        .arg(copy.path().join("p"))
        .status()
        .unwrap();
    assert!(status.success());
    let p = g4_kernel::project::Project::open(copy.path().join("p")).unwrap();
    p.ensure_unpacked(g4_kernel::project::AssetKind::Map)
        .unwrap();
    let bytes = std::fs::read(p.member_path(g4_kernel::project::AssetKind::Map, 0)).unwrap();
    let map = g4_formats::map::MapFile::read(&bytes, p.family).unwrap();
    let out = tempfile::tempdir().unwrap();
    let glb = nsbmd_to_glb(Path::new(&apicula), &map.model, None, out.path()).unwrap();
    assert_eq!(&std::fs::read(&glb).unwrap()[0..4], b"glTF");
}
