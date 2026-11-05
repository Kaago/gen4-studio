//! Writes the synthetic test project (`g4_kernel::test_support::synthetic_project`) into a
//! folder that outlives the process, so the packaged-app smoke test can open it. It holds no ROM
//! data: every file is generated.
//!
//!     cargo run -p g4-kernel --example make_synthetic --features test-support -- <folder>
//!
//! The folder must not exist or must be empty. Prints its absolute path.

use std::path::Path;
use std::process::ExitCode;

use g4_kernel::test_support::synthetic_project;

/// Copies the regular files and folders of `from` into `to` (created as needed).
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [folder] = args.as_slice() else {
        eprintln!("usage: make_synthetic <folder>");
        return ExitCode::from(2);
    };
    let folder = Path::new(folder);
    if folder.read_dir().is_ok_and(|mut d| d.next().is_some()) {
        eprintln!("make_synthetic: {} is not empty", folder.display());
        return ExitCode::from(2);
    }
    let fixture = synthetic_project();
    if let Err(e) = copy_tree(fixture.dir.path(), folder) {
        eprintln!("make_synthetic: {}: {e}", folder.display());
        return ExitCode::FAILURE;
    }
    match folder.canonicalize() {
        Ok(path) => println!("{}", path.display()),
        Err(_) => println!("{}", folder.display()),
    }
    ExitCode::SUCCESS
}
