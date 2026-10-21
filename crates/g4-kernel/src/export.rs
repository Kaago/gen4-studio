//! Model export through the external `apicula` converter.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::KernelError;

static WORK_COUNTER: AtomicU32 = AtomicU32::new(0);

fn tool_error(reason: impl Into<String>) -> KernelError {
    KernelError::ToolMissing {
        name: "apicula",
        reason: reason.into(),
    }
}

/// Creates a fresh, empty working directory below `out_dir` (apicula insists on creating its
/// own output directory, so each call needs a name that does not exist yet).
fn fresh_work_dir(out_dir: &Path) -> Result<PathBuf, KernelError> {
    std::fs::create_dir_all(out_dir)?;
    loop {
        let n = WORK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = out_dir.join(format!("apicula-{}-{n}", std::process::id()));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
}

/// Collects every `.glb` below `dir` (apicula writes `<outdir>/<model name>.glb`).
fn collect_glbs(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), KernelError> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_glbs(&path, found)?;
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb")) {
            found.push(path);
        }
    }
    Ok(())
}

/// Writes `nsbmd` to a temp file and runs `apicula convert -f=glb <file> -o <dir>` into a fresh
/// subdirectory of `out_dir`. Returns the path of the single `.glb` it produced; the subdirectory
/// stays below `out_dir`, so removing `out_dir` cleans up.
///
/// apicula names its output after the model inside the NSBMD (not the input file), refuses an
/// existing output directory and exits 0 even when it could not parse the input, hence the
/// explicit check for exactly one produced file.
pub fn nsbmd_to_glb(apicula: &Path, nsbmd: &[u8], out_dir: &Path) -> Result<PathBuf, KernelError> {
    if !apicula.exists() {
        return Err(tool_error(format!("not found: {}", apicula.display())));
    }
    let work = fresh_work_dir(out_dir)?;
    let input = work.join("input.nsbmd");
    std::fs::write(&input, nsbmd)?;
    let converted = work.join("converted");

    let output = Command::new(apicula)
        .arg("convert")
        .arg("-f=glb")
        .arg(&input)
        .arg("-o")
        .arg(&converted)
        .output()
        .map_err(|e| tool_error(format!("cannot run {}: {e}", apicula.display())))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let reason = if stderr.trim().is_empty() {
            format!("{} ({})", output.status, stdout.trim())
        } else {
            stderr.trim().to_string()
        };
        return Err(tool_error(reason));
    }

    let mut glbs = Vec::new();
    if converted.is_dir() {
        collect_glbs(&converted, &mut glbs)?;
    }
    match glbs.len() {
        1 => Ok(glbs.remove(0)),
        n => Err(KernelError::Build(format!(
            "apicula produced {n} .glb files, expected 1: {} {}",
            stdout.trim(),
            stderr.trim()
        ))),
    }
}
