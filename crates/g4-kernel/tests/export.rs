use std::path::Path;

use g4_kernel::error::KernelError;
use g4_kernel::export::nsbmd_to_glb;

#[test]
fn missing_apicula_is_tool_missing() {
    let out = tempfile::tempdir().unwrap();
    let err = nsbmd_to_glb(Path::new("/nonexistent/apicula"), b"BMD0", out.path()).unwrap_err();
    assert!(matches!(
        err,
        KernelError::ToolMissing {
            name: "apicula",
            ..
        }
    ));
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

    // apicula's argv: convert -f=glb <input> -o <dir>; the script creates <dir>/<name>/<name>.glb.
    const PRODUCE: &str =
        r#"out="$5"; mkdir -p "$out/model"; printf 'glTFxxxx' > "$out/model/model.glb""#;

    #[test]
    fn returns_the_single_glb_the_tool_produced() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let tool = fake(tools.path(), PRODUCE);
        let glb = nsbmd_to_glb(&tool, b"BMD0", out.path()).unwrap();
        assert_eq!(glb.extension().unwrap(), "glb");
        assert!(glb.starts_with(out.path()));
        assert_eq!(&std::fs::read(&glb).unwrap()[0..4], b"glTF");
    }

    #[test]
    fn non_zero_exit_reports_stderr() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let tool = fake(tools.path(), "echo boom >&2; exit 3");
        let err = nsbmd_to_glb(&tool, b"BMD0", out.path()).unwrap_err();
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
        assert!(nsbmd_to_glb(&none, b"BMD0", out.path()).is_err());
        let two = fake(
            tools.path(),
            r#"out="$5"; mkdir -p "$out/a" "$out/b"; : > "$out/a/a.glb"; : > "$out/b/b.glb""#,
        );
        assert!(nsbmd_to_glb(&two, b"BMD0", out.path()).is_err());
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
    let glb = nsbmd_to_glb(Path::new(&apicula), &map.model, out.path()).unwrap();
    assert_eq!(&std::fs::read(&glb).unwrap()[0..4], b"glTF");
}
