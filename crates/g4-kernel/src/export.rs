//! Model export through the external `apicula` converter.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use xxhash_rust::xxh3::Xxh3;

use crate::error::{KernelError, io_at};

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
    std::fs::create_dir_all(out_dir).map_err(io_at(out_dir))?;
    loop {
        let n = WORK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = out_dir.join(format!("apicula-{}-{n}", std::process::id()));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(io_at(&dir)(e)),
        }
    }
}

/// Collects every `.glb` below `dir` (apicula writes `<outdir>/<model name>.glb`).
fn collect_glbs(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), KernelError> {
    for entry in std::fs::read_dir(dir).map_err(io_at(dir))? {
        let path = entry.map_err(io_at(dir))?.path();
        if path.is_dir() {
            collect_glbs(&path, found)?;
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("glb"))
        {
            found.push(path);
        }
    }
    Ok(())
}

/// Writes `nsbmd` (and `nsbtx`, when given) to temp files and runs
/// `apicula convert -f=glb <nsbmd> [<nsbtx>] -o <dir>` into a fresh subdirectory of `out_dir`. Returns the path of the single `.glb` it produced; the subdirectory
/// stays below `out_dir`, so removing `out_dir` cleans up.
///
/// apicula names its output after the model inside the NSBMD (not the input file), refuses an
/// existing output directory and exits 0 even when it could not parse the input, hence the
/// explicit check for exactly one produced file.
pub fn nsbmd_to_glb(
    apicula: &Path,
    nsbmd: &[u8],
    nsbtx: Option<&[u8]>,
    out_dir: &Path,
) -> Result<PathBuf, KernelError> {
    if !apicula.exists() {
        return Err(tool_error(format!("not found: {}", apicula.display())));
    }
    let work = fresh_work_dir(out_dir)?;
    let input = work.join("input.nsbmd");
    std::fs::write(&input, nsbmd).map_err(io_at(&input))?;
    let converted = work.join("converted");

    let mut command = Command::new(apicula);
    command.arg("convert").arg("-f=glb").arg(&input);
    if let Some(nsbtx) = nsbtx {
        let textures = work.join("input.nsbtx");
        std::fs::write(&textures, nsbtx).map_err(io_at(&textures))?;
        command.arg(&textures);
    }
    let output = command
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

/// Cache key of a converted model: xxh3_64 over `nsbmd` followed by `nsbtx` (nothing when
/// `None`) as 16 lower-case hex digits. The cached file is `<cache_dir>/<key>.glb`.
pub fn glb_cache_key(nsbmd: &[u8], nsbtx: Option<&[u8]>) -> String {
    let mut hasher = Xxh3::new();
    hasher.update(nsbmd);
    if let Some(nsbtx) = nsbtx {
        hasher.update(nsbtx);
    }
    format!("{:016x}", hasher.digest())
}

/// Converts into a fresh work directory below `cache_dir` and publishes the result under
/// `target` (temp file in the same directory, then rename), so a concurrent reader never sees
/// a half-written cache file.
fn convert_into_cache(
    cache_dir: &Path,
    apicula: &Path,
    nsbmd: &[u8],
    nsbtx: Option<&[u8]>,
    target: &Path,
) -> Result<Vec<u8>, KernelError> {
    let work = fresh_work_dir(cache_dir)?;
    let result = (|| {
        let glb = nsbmd_to_glb(apicula, nsbmd, nsbtx, &work)?;
        let bytes = std::fs::read(&glb).map_err(io_at(&glb))?;
        let tmp = work.join("cache.glb");
        std::fs::write(&tmp, &bytes).map_err(io_at(&tmp))?;
        std::fs::rename(&tmp, target).map_err(io_at(target))?;
        Ok(bytes)
    })();
    // Best effort: a leftover work directory is harmless but must not hide the real result.
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// The cached GLB for this model and tileset, if conversion already happened.
pub fn glb_cache_get(
    cache_dir: &Path,
    nsbmd: &[u8],
    nsbtx: Option<&[u8]>,
) -> Result<Option<Vec<u8>>, KernelError> {
    let target = cache_dir.join(format!("{}.glb", glb_cache_key(nsbmd, nsbtx)));
    match std::fs::read(&target) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(&target)(e)),
    }
}

/// GLB bytes of a model from the content-hash cache in `cache_dir`, or converted with apicula
/// (NSBMD plus the optional NSBTX texture set in one call) and stored there. apicula is only
/// needed on a cache miss.
pub fn glb_cached(
    cache_dir: &Path,
    apicula: &Path,
    nsbmd: &[u8],
    nsbtx: Option<&[u8]>,
) -> Result<Vec<u8>, KernelError> {
    if let Some(bytes) = glb_cache_get(cache_dir, nsbmd, nsbtx)? {
        return Ok(bytes);
    }
    let target = cache_dir.join(format!("{}.glb", glb_cache_key(nsbmd, nsbtx)));
    convert_into_cache(cache_dir, apicula, nsbmd, nsbtx, &target)
}
