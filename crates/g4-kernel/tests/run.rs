use std::path::Path;

use g4_kernel::error::KernelError;
use g4_kernel::run::launch_emulator;

#[test]
fn missing_emulator_is_tool_missing() {
    let err =
        launch_emulator(Path::new("/nonexistent/melonDS"), Path::new("/tmp/x.nds")).unwrap_err();
    assert!(matches!(
        err,
        KernelError::ToolMissing {
            name: "emulator",
            ..
        }
    ));
}

#[cfg(unix)]
mod fake_tool {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    use super::*;

    fn fake(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("fake-emulator");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn wait_for(path: &Path) -> String {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            if let Ok(s) = std::fs::read_to_string(path)
                && !s.is_empty()
            {
                return s;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("{} was never written", path.display());
    }

    #[test]
    fn rom_path_is_passed_as_the_only_argument() {
        let dir = tempfile::tempdir().unwrap();
        let argv = dir.path().join("argv.txt");
        let tool = fake(
            dir.path(),
            &format!("printf '%s|%s' \"$#\" \"$1\" > '{}'", argv.display()),
        );
        let pid = launch_emulator(&tool, Path::new("/some/dir/out.nds")).unwrap();
        assert!(pid > 0);
        assert_eq!(wait_for(&argv), "1|/some/dir/out.nds");
    }

    #[test]
    fn does_not_wait_for_the_emulator() {
        let dir = tempfile::tempdir().unwrap();
        let tool = fake(dir.path(), "sleep 5");
        let start = Instant::now();
        launch_emulator(&tool, Path::new("/x.nds")).unwrap();
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn non_executable_file_is_tool_missing() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain");
        std::fs::write(&plain, "not a program").unwrap();
        let err = launch_emulator(&plain, Path::new("/x.nds")).unwrap_err();
        assert!(matches!(
            err,
            KernelError::ToolMissing {
                name: "emulator",
                ..
            }
        ));
    }
}
