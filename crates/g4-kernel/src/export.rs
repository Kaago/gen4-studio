//! Model export through the external `apicula` converter.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use xxhash_rust::xxh3::Xxh3;

use crate::error::{KernelError, io_at};

/// Format version of the cached GLBs, hashed into every cache key. Bump it when the cached
/// bytes change meaning: v2 embeds the textures (v1 left them as `uri` references the viewer
/// cannot resolve), and old entries and `.fail` markers are never looked at again.
const CACHE_SALT: &[u8] = b"g4glb-v2\0";

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

/// Cache key of a converted model: xxh3_64 over [`CACHE_SALT`], `nsbmd` and then `nsbtx`
/// (nothing when `None`) as 16 lower-case hex digits. The cached file is
/// `<cache_dir>/<key>.glb`.
pub fn glb_cache_key(nsbmd: &[u8], nsbtx: Option<&[u8]>) -> String {
    let mut hasher = Xxh3::new();
    hasher.update(CACHE_SALT);
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
        let raw = std::fs::read(&glb).map_err(io_at(&glb))?;
        // apicula writes the textures as PNG files next to the .glb.
        let dir = glb.parent().unwrap_or(&work);
        let bytes = embed_images(&raw, |name| std::fs::read(dir.join(name)).ok())?;
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

/// Marker file of a textured conversion that failed: `<cache_dir>/<key>.fail`, holding the
/// failure reason. Callers that find it skip the textured attempt instead of running apicula
/// again on every load. The key is a content hash, so changed model or tileset bytes are
/// tried afresh; deleting the cache directory clears all markers.
fn glb_fail_path(cache_dir: &Path, nsbmd: &[u8], nsbtx: Option<&[u8]>) -> PathBuf {
    cache_dir.join(format!("{}.fail", glb_cache_key(nsbmd, nsbtx)))
}

/// The recorded failure reason for this model and tileset, if a conversion failed before.
pub fn glb_fail_get(
    cache_dir: &Path,
    nsbmd: &[u8],
    nsbtx: Option<&[u8]>,
) -> Result<Option<String>, KernelError> {
    let path = glb_fail_path(cache_dir, nsbmd, nsbtx);
    match std::fs::read_to_string(&path) {
        Ok(reason) => Ok(Some(reason)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_at(&path)(e)),
    }
}

/// Records that converting this model with this tileset failed (temp file in the same
/// directory, then rename, so a concurrent reader never sees a half-written marker).
pub fn glb_fail_record(
    cache_dir: &Path,
    nsbmd: &[u8],
    nsbtx: Option<&[u8]>,
    reason: &str,
) -> Result<(), KernelError> {
    std::fs::create_dir_all(cache_dir).map_err(io_at(cache_dir))?;
    let target = glb_fail_path(cache_dir, nsbmd, nsbtx);
    let n = WORK_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = cache_dir.join(format!(".fail-{}-{n}.tmp", std::process::id()));
    std::fs::write(&tmp, reason).map_err(io_at(&tmp))?;
    std::fs::rename(&tmp, &target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        io_at(&target)(e)
    })
}

const GLB_MAGIC: &[u8; 4] = b"glTF";
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;

fn glb_error(reason: impl std::fmt::Display) -> KernelError {
    KernelError::Build(format!("invalid glb: {reason}"))
}

fn pad4(bytes: &mut Vec<u8>, fill: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(fill);
    }
}

/// A `uri` that names a plain file inside apicula's output directory: no path separators, no
/// `..`, nothing absolute or scheme-like.
fn is_plain_file_name(uri: &str) -> bool {
    !uri.is_empty()
        && uri != "."
        && uri != ".."
        && !uri.contains(['/', '\\', ':', '\0'])
        && !uri.contains("..")
}

fn mime_type(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "image/png",
    }
}

/// Embeds every image a GLB references by a relative file `uri` into its BIN chunk, so a viewer
/// that only gets the bytes (three's `GLTFLoader.parse` on an ArrayBuffer) can show the
/// textures.
///
/// `read` returns the bytes of a plain file name from the converter's output directory. Each
/// image becomes `{ bufferView, mimeType, name }` (the file stem) with its bytes appended to
/// the BIN chunk (4-byte aligned, zero padded; a chunk is added when there is none); `buffers[0]`,
/// the chunk lengths and the total length follow, and the JSON chunk is padded with spaces.
/// `data:` URIs and images that already have a `bufferView` stay as they are, and a GLB without
/// any such image is returned unchanged.
///
/// # Errors
/// A GLB that cannot be parsed, a `uri` that is not a plain file name, and a referenced file
/// `read` cannot supply (the error names it).
pub fn embed_images(
    glb: &[u8],
    read: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<Vec<u8>, KernelError> {
    let (mut json, mut bin) = parse_glb(glb)?;

    // Image indices that reference a file, with their uri.
    let pending: Vec<(usize, String)> = json
        .get("images")
        .and_then(|i| i.as_array())
        .map(|images| {
            images
                .iter()
                .enumerate()
                .filter_map(|(i, image)| {
                    let uri = image.get("uri")?.as_str()?;
                    (!uri.starts_with("data:")).then(|| (i, uri.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    if pending.is_empty() {
        return Ok(glb.to_vec());
    }

    if json["buffers"][0].get("uri").is_some() {
        return Err(glb_error(
            "buffer 0 is external, cannot embed images into it",
        ));
    }
    pad4(&mut bin, 0);
    let mut views = json
        .get("bufferViews")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    // A file used by several images is stored once.
    let mut stored: Vec<(String, usize)> = Vec::new();
    for (index, uri) in pending {
        if !is_plain_file_name(&uri) {
            return Err(glb_error(format!(
                "image uri {uri:?} is not a plain file name"
            )));
        }
        let view = if let Some((_, view)) = stored.iter().find(|(name, _)| *name == uri) {
            *view
        } else {
            let bytes = read(&uri)
                .ok_or_else(|| glb_error(format!("referenced image file {uri} is missing")))?;
            let offset = bin.len();
            bin.extend_from_slice(&bytes);
            pad4(&mut bin, 0);
            views.push(serde_json::json!({
                "buffer": 0,
                "byteOffset": offset,
                "byteLength": bytes.len(),
            }));
            stored.push((uri.clone(), views.len() - 1));
            views.len() - 1
        };
        let stem = Path::new(&uri)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&uri)
            .to_string();
        let image = json["images"][index]
            .as_object_mut()
            .expect("image was an object");
        image.remove("uri");
        image.insert("bufferView".into(), view.into());
        image.insert("mimeType".into(), mime_type(&uri).into());
        image.insert("name".into(), stem.into());
    }
    json["bufferViews"] = views.into();
    if json
        .get("buffers")
        .and_then(|b| b.as_array())
        .is_none_or(Vec::is_empty)
    {
        json["buffers"] = serde_json::json!([{}]);
    }
    json["buffers"][0]["byteLength"] = bin.len().into();
    Ok(write_glb(&json, &bin))
}

/// The JSON document and BIN chunk data (empty when absent) of a GLB; other chunks are ignored.
fn parse_glb(glb: &[u8]) -> Result<(serde_json::Value, Vec<u8>), KernelError> {
    let word = |at: usize| -> Result<u32, KernelError> {
        glb.get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
            .ok_or_else(|| glb_error("truncated"))
    };
    if glb.get(0..4) != Some(GLB_MAGIC) {
        return Err(glb_error("missing glTF magic"));
    }
    if word(4)? != 2 {
        return Err(glb_error("not glTF version 2"));
    }
    if word(8)? as usize != glb.len() {
        return Err(glb_error("length in header does not match the data"));
    }
    let mut json = None;
    let mut bin = None;
    let mut at = 12;
    while at < glb.len() {
        let len = word(at)? as usize;
        let kind = word(at + 4)?;
        let data = at
            .checked_add(8)
            .and_then(|start| Some(start..start.checked_add(len)?))
            .and_then(|range| glb.get(range))
            .ok_or_else(|| glb_error("chunk exceeds the data"))?;
        match kind {
            CHUNK_JSON if json.is_none() => {
                json = Some(
                    serde_json::from_slice(data)
                        .map_err(|e| glb_error(format!("JSON chunk: {e}")))?,
                );
            }
            CHUNK_BIN if bin.is_none() => bin = Some(data.to_vec()),
            _ => {}
        }
        at += 8 + len;
    }
    let json = json.ok_or_else(|| glb_error("no JSON chunk"))?;
    Ok((json, bin.unwrap_or_default()))
}

fn write_glb(json: &serde_json::Value, bin: &[u8]) -> Vec<u8> {
    let mut json = serde_json::to_vec(json).expect("a JSON value serializes");
    pad4(&mut json, b' ');
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(GLB_MAGIC);
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    out.extend_from_slice(bin);
    out
}
