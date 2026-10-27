//! The project preview for the start list (phase 4.0 design, §3.3): read and write the
//! 256 × 192 picture under `<project>/.g4/` through `g4_kernel::preview`.
//!
//! Without a `path` both commands work on the open project; with one they touch only the two
//! preview files under `<path>/.g4/`, without opening the project or changing the app's state.
//! The PNG travels as base64, so the answer is the same JSON in the app and the bridge; MCP
//! moves it out of the JSON into an image content block (`mcp::server`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use g4_kernel::preview::{self, Preview, PreviewInput, PreviewSlot, PreviewSource, StandardMap};
use g4_kernel::project::Project;
use serde::Serialize;
use tauri::Manager;

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// Longest base64 `png` taken: the encoding of the kernel's largest image file (32 MiB).
/// Longer text is refused before it is decoded.
const MAX_PNG_BASE64: usize = (32 * 1024 * 1024usize).div_ceil(3) * 4;

/// What both commands answer: the preview as it is now (`source` and `png` both `None`
/// without one) and, for the open project only, the start town's map the app renders the
/// standard picture of.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDto {
    pub source: Option<PreviewSource>,
    /// Base64 of a PNG of 256 × 192.
    pub png: Option<String>,
    pub standard_map: Option<StandardMap>,
}

/// The project folder a call is about, and the open project when it is that one: without
/// `path` the open project (the usual error without one), with `path` that folder, which is
/// the open project when both name the same folder after links are resolved.
fn target(
    state: &AppState,
    path: Option<&Path>,
) -> Result<(PathBuf, Option<Arc<Project>>), CmdError> {
    let mut guard = lock(state)?;
    match path {
        None => {
            let project = Arc::clone(&session(&mut guard)?.project);
            Ok((project.root.clone(), Some(project)))
        }
        Some(path) => {
            let open = guard.as_ref().map(|s| Arc::clone(&s.project));
            drop(guard);
            let open = open.filter(|project| same_folder(&project.root, path));
            Ok((path.to_path_buf(), open))
        }
    }
}

/// Whether `a` and `b` are the same existing folder.
fn same_folder(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// The answer for `preview` of the project at a folder; `open` is the open project when the
/// folder is its (only then is there a standard map). The map is read from disk, outside the
/// state lock.
fn dto(preview: Preview, open: Option<&Project>) -> PreviewDto {
    PreviewDto {
        source: preview.source,
        png: preview.png.map(|png| STANDARD.encode(png)),
        standard_map: open.and_then(preview::standard_map),
    }
}

/// The preview of the open project (`path` `None`) or of the project folder `path`. A folder
/// that is gone or has no valid preview answers `source: None`, never an error.
pub fn project_preview_get_impl(
    state: &AppState,
    path: Option<&Path>,
) -> Result<PreviewDto, CmdError> {
    let (root, open) = target(state, path)?;
    let read = preview::read_preview(&root)?;
    Ok(dto(read, open.as_deref()))
}

/// Writes the picture `png` (base64 of a PNG or JPEG of any size) or the file `file` into
/// `slot` of the open project (`path` `None`) or of the project folder `path`, fitted to
/// 256 × 192; with neither it removes that slot's file. Answers the preview after the write.
/// `png` and `file` together are refused, and so is a `png` that is no base64
/// (`preview.format`, like a picture that is neither PNG nor JPEG) or longer than the base64
/// of the kernel's largest file (`preview.too_large`, before it is decoded).
pub fn project_preview_set_impl(
    state: &AppState,
    path: Option<&Path>,
    slot: PreviewSlot,
    png: Option<&str>,
    file: Option<&Path>,
) -> Result<PreviewDto, CmdError> {
    let bytes =
        match (png, file) {
            (Some(_), Some(_)) => {
                return Err("png und file schließen sich aus; nur eins von beiden angeben".into());
            }
            (Some(text), None) => {
                let text = text.trim();
                if text.len() > MAX_PNG_BASE64 {
                    return Err(CmdError::from(format!(
                        "preview.too_large: png has {} base64 characters, at most {MAX_PNG_BASE64}",
                        text.len()
                    )));
                }
                Some(STANDARD.decode(text).map_err(|e| {
                    CmdError::from(format!("preview.format: png is no base64 ({e})"))
                })?)
            }
            (None, _) => None,
        };
    let input = match (&bytes, file) {
        (Some(bytes), _) => PreviewInput::Bytes(bytes),
        (None, Some(file)) => PreviewInput::File(file),
        (None, None) => PreviewInput::Remove,
    };
    let (root, open) = target(state, path)?;
    let written = preview::write_preview(&root, slot, input)?;
    Ok(dto(written, open.as_deref()))
}

/// Reads two small files (and, for the open project, its headers and matrix 0), so it runs
/// on a blocking worker.
#[tauri::command]
pub async fn project_preview_get(
    app: tauri::AppHandle,
    path: Option<String>,
) -> Result<PreviewDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        project_preview_get_impl(&app.state::<AppState>(), path.as_deref().map(Path::new))
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}

/// Decodes, fits and encodes a picture of up to 8192 × 8192, so it runs on a blocking
/// worker.
#[tauri::command]
pub async fn project_preview_set(
    app: tauri::AppHandle,
    path: Option<String>,
    slot: Option<PreviewSlot>,
    png: Option<String>,
    file: Option<String>,
) -> Result<PreviewDto, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        project_preview_set_impl(
            &app.state::<AppState>(),
            path.as_deref().map(Path::new),
            slot.unwrap_or_default(),
            png.as_deref(),
            file.as_deref().map(Path::new),
        )
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
}
