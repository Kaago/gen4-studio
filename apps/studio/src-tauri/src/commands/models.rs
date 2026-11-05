//! Map terrain models (GLB through apicula) and their texture status.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use g4_kernel::error::KernelError;
use g4_kernel::export::{
    apicula_timeout, glb_cache_get, glb_cache_key, glb_cached_with_timeout, glb_fail_get,
    glb_fail_record, last_tool, remember_tool, tool_fingerprint,
};
use serde::Serialize;
use tauri::{Manager, State};

use super::{lock, session};
use crate::errors::CmdError;
use crate::state::AppState;

/// Finds the apicula executable: `G4_APICULA`, then `apicula` next to the running binary, then
/// (debug builds only) the repository's `tools/apicula/bin/apicula`; `.exe` is added on Windows.
/// A set `G4_APICULA` always wins, even when it points nowhere, so a typo is reported instead of
/// silently ignored. An empty file is the build-time placeholder for the bundled sidecar (see
/// build.rs), not apicula.
pub(super) fn find_apicula() -> Option<PathBuf> {
    crate::sidecar::find_sidecar("G4_APICULA", "apicula", "tools/apicula/bin/apicula")
}

/// Why a map's model has no textures although it should have: the area data or tileset it
/// depends on cannot be used. `tileset` is the tileset index when it is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextureIssue {
    pub tileset: Option<u16>,
    /// First line of the (English) kernel error, for display next to the German note.
    pub reason: String,
}

fn issue(tileset: Option<u16>, reason: &str) -> TextureIssue {
    let line = reason.lines().next().unwrap_or_default();
    TextureIssue {
        tileset,
        reason: line.chars().take(200).collect(),
    }
}

/// What the model conversion needs, read under one short lock acquisition. The tileset follows
/// the store's area data, so an unsaved edit re-textures the model.
struct ModelInputs {
    cache_dir: PathBuf,
    nsbmd: Vec<u8>,
    tileset: Option<Vec<u8>>,
    tileset_id: Option<u16>,
    /// Set when the area data or the tileset could not be read: `tileset` is then `None`.
    problem: Option<TextureIssue>,
}

/// Cache directory, model and tileset come from ONE short lock acquisition, so a project
/// switch in between can never mix two projects. A tileset that cannot be resolved (unreadable
/// area data, unreadable or corrupt texture archive) is not an error: the map is converted
/// without textures and the problem is reported in `problem`.
fn model_inputs(state: &AppState, index: u16, area_data_id: u8) -> Result<ModelInputs, CmdError> {
    let mut guard = lock(state)?;
    let session = session(&mut guard)?;
    let project = Arc::clone(&session.project);
    let nsbmd = session.store.map(index)?.model.clone();
    let mut inputs = ModelInputs {
        cache_dir: project.root.join(".g4").join("models"),
        nsbmd,
        tileset: None,
        tileset_id: None,
        problem: None,
    };
    match session.store.map_tileset_index(area_data_id) {
        Ok(Some(id)) => {
            inputs.tileset_id = Some(id);
            match project.map_tileset_member(id) {
                Ok(bytes) => inputs.tileset = bytes,
                Err(e) => inputs.problem = Some(issue(Some(id), &e.to_string())),
            }
        }
        Ok(None) => {}
        Err(e) => inputs.problem = Some(issue(None, &e.to_string())),
    }
    Ok(inputs)
}

/// The apicula to run, or a `ToolMissing` error naming the fix. Checked before any
/// conversion so a missing tool is never mistaken for a tileset that cannot be converted.
fn require_apicula(apicula: Option<&Path>) -> Result<&Path, KernelError> {
    let missing = |reason: String| KernelError::ToolMissing {
        name: "apicula",
        reason,
    };
    let apicula = apicula.ok_or_else(|| {
        missing("set G4_APICULA or place apicula next to the app (see docs/dev-setup.md)".into())
    })?;
    if !apicula.exists() {
        return Err(missing(format!("not found: {}", apicula.display())));
    }
    Ok(apicula)
}

/// Textured cache keys whose conversion timed out in this process. A timeout leaves no `.fail`
/// marker (apicula may only have been slow), but retrying a hanging tileset on every load would
/// cost the full time limit each time, so for the rest of the process such a model goes straight
/// to its untextured version.
fn timed_out_keys() -> &'static Mutex<HashSet<String>> {
    static KEYS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    KEYS.get_or_init(Default::default)
}

/// The tool hash cache keys are built from. With a usable apicula it is the hash of that binary
/// (remembered as the cache's last used tool); without one, the last remembered hash, so models
/// converted earlier stay visible. With neither, no entry can match (0) and the first
/// conversion reports the missing tool.
fn resolve_tool(cache_dir: &Path, apicula: Option<&Path>) -> Result<u64, KernelError> {
    match apicula.filter(|p| p.exists()) {
        Some(path) => {
            let tool = tool_fingerprint(path)?;
            if last_tool(cache_dir) != Some(tool) {
                // Best effort: without it, cached models are only hidden while apicula is missing.
                let _ = remember_tool(cache_dir, tool);
            }
            Ok(tool)
        }
        None => Ok(last_tool(cache_dir).unwrap_or(0)),
    }
}

/// GLB bytes of one NSBMD with an optional NSBTX texture set, cached by content hash in
/// `cache_dir`; shared by the map terrain and the building models. `apicula` is only needed
/// on a cache miss, and the cache keys carry its hash, so a different apicula starts afresh
/// (see `g4_kernel::export::glb_cache_key`). A missing tileset, or a textured conversion that
/// fails, falls back to the untextured model; a failed textured conversion is recorded next
/// to the cache (`<key>.fail`) so later loads skip it. apicula gets `timeout` per run; a
/// textured run that exceeds it writes no marker but also falls back to the untextured model,
/// and the textured attempt is skipped for the rest of the process (`timed_out_keys`). Only
/// when the untextured run times out as well is `ToolTimeout` returned. Runs apicula, so it
/// must be called without the state lock.
pub(super) fn glb_with_fallback(
    cache_dir: &Path,
    nsbmd: &[u8],
    tileset: Option<&[u8]>,
    apicula: Option<&Path>,
    timeout: Duration,
) -> Result<Vec<u8>, CmdError> {
    let tool = resolve_tool(cache_dir, apicula)?;
    let mut textured_failure = None;
    if let Some(tileset) = tileset {
        if let Some(bytes) = glb_cache_get(cache_dir, tool, nsbmd, Some(tileset))? {
            return Ok(bytes);
        }
        let key = glb_cache_key(tool, nsbmd, Some(tileset));
        let timed_out = timed_out_keys()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&key);
        if !timed_out && glb_fail_get(cache_dir, tool, nsbmd, Some(tileset))?.is_none() {
            let apicula = require_apicula(apicula)?;
            match glb_cached_with_timeout(cache_dir, apicula, tool, nsbmd, Some(tileset), timeout) {
                Ok(bytes) => return Ok(bytes),
                // No marker: apicula may only have been slow. Show the untextured model.
                Err(KernelError::ToolTimeout { .. }) => {
                    timed_out_keys()
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(key);
                }
                Err(e) => textured_failure = Some(e.to_string()),
            }
        }
    }
    let plain = match glb_cache_get(cache_dir, tool, nsbmd, None)? {
        Some(bytes) => bytes,
        None => glb_cached_with_timeout(
            cache_dir,
            require_apicula(apicula)?,
            tool,
            nsbmd,
            None,
            timeout,
        )?,
    };
    // Recorded only now: if the untextured conversion fails too, the tileset is not to blame.
    if let (Some(reason), Some(tileset)) = (textured_failure, tileset) {
        // Best effort: without a marker the next load simply tries the tileset again.
        let _ = glb_fail_record(cache_dir, tool, nsbmd, Some(tileset), &reason);
    }
    Ok(plain)
}

/// GLB bytes of the map's terrain model with the map's texture set (from `area_data_id`); see
/// [`glb_with_fallback`] for caching and the untextured fallback. The reason a model came out
/// untextured is available through [`map_model_texture_status_impl`].
pub fn get_map_model_glb_with(
    state: &AppState,
    index: u16,
    area_data_id: u8,
    apicula: Option<&Path>,
) -> Result<Vec<u8>, CmdError> {
    get_map_model_glb_timed(state, index, area_data_id, apicula, apicula_timeout())
}

/// [`get_map_model_glb_with`] with an explicit apicula time limit instead of the one from
/// `G4_APICULA_TIMEOUT_SECS`.
pub fn get_map_model_glb_timed(
    state: &AppState,
    index: u16,
    area_data_id: u8,
    apicula: Option<&Path>,
    timeout: Duration,
) -> Result<Vec<u8>, CmdError> {
    let ModelInputs {
        cache_dir,
        nsbmd,
        tileset,
        ..
    } = model_inputs(state, index, area_data_id)?;
    glb_with_fallback(&cache_dir, &nsbmd, tileset.as_deref(), apicula, timeout)
}

/// Whether the model of map `index` is shown without textures because of its tileset: the area
/// data or tileset cannot be read, or an earlier conversion with this tileset failed (see
/// [`get_map_model_glb_with`]). `None` when nothing is wrong. Call it after the model was
/// requested; it never runs apicula.
pub fn map_model_texture_status_impl(
    state: &AppState,
    index: u16,
    area_data_id: u8,
) -> Result<Option<TextureIssue>, CmdError> {
    let inputs = model_inputs(state, index, area_data_id)?;
    if inputs.problem.is_some() {
        return Ok(inputs.problem);
    }
    let Some(tileset) = inputs.tileset.as_deref() else {
        return Ok(None);
    };
    // Markers carry the hash of the apicula that wrote them; the model was requested before,
    // so that is the last used one, whether or not apicula is still there.
    let tool = last_tool(&inputs.cache_dir).unwrap_or(0);
    Ok(
        glb_fail_get(&inputs.cache_dir, tool, &inputs.nsbmd, Some(tileset))?
            .map(|reason| issue(inputs.tileset_id, &reason)),
    )
}

pub fn get_map_model_glb_impl(
    state: &AppState,
    index: u16,
    area_data_id: u8,
) -> Result<Vec<u8>, CmdError> {
    get_map_model_glb_with(state, index, area_data_id, find_apicula().as_deref())
}

/// Raw GLB bytes over IPC (no file paths, no asset protocol). The apicula run (up to its time
/// limit) happens on a blocking worker like `build_rom`, not on the async pool.
#[tauri::command]
pub async fn get_map_model_glb(
    app: tauri::AppHandle,
    index: u16,
    area_data_id: u8,
) -> Result<tauri::ipc::Response, CmdError> {
    tauri::async_runtime::spawn_blocking(move || {
        get_map_model_glb_impl(&app.state::<AppState>(), index, area_data_id)
    })
    .await
    .map_err(|e| CmdError::from(e.to_string()))?
    .map(tauri::ipc::Response::new)
}

/// Why a map's model is untextured because of its tileset, if it is (see
/// [`map_model_texture_status_impl`]). Cheap: reads a few small files, never runs apicula.
#[tauri::command]
pub fn map_model_texture_status(
    state: State<AppState>,
    index: u16,
    area_data_id: u8,
) -> Result<Option<TextureIssue>, CmdError> {
    map_model_texture_status_impl(&state, index, area_data_id)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::open_synthetic;
    use super::*;
    use crate::commands::*;
    use g4_formats::map::MapFile;
    use g4_kernel::export::{glb_cache_key, remember_tool};
    #[cfg(unix)]
    use g4_kernel::project::{AssetKind, Project};

    /// Stand-in tool hash for tests that seed the cache by hand: without apicula, hits are
    /// looked up under the last remembered one.
    const TOOL: u64 = 0xabcd;

    #[test]
    fn model_glb_without_apicula_names_the_tool() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let err = get_map_model_glb_with(&state, 0, 0, None).unwrap_err();
        assert!(err.message.contains("apicula"), "{}", err.message);
        let err = get_map_model_glb_with(&state, 0, 0, Some(Path::new("/nonexistent/apicula")))
            .unwrap_err();
        assert!(err.message.contains("apicula"), "{}", err.message);
        let err = get_map_model_glb_impl(&AppState::default(), 0, 0).unwrap_err();
        assert_eq!(err.message, "no project open");
    }

    /// The terrain model of synthetic map 0.
    fn synthetic_model() -> Vec<u8> {
        let bytes = g4_kernel::test_support::synthetic_map_bytes(0);
        MapFile::read(&bytes, g4_formats::game::Family::Pt)
            .unwrap()
            .model
    }

    #[test]
    fn model_glb_cache_is_keyed_by_model_and_tileset_content() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let models = fixture.dir.path().join(".g4/models");
        std::fs::create_dir_all(&models).unwrap();
        let model = g4_kernel::test_support::synthetic_map_bytes(0);
        let model = MapFile::read(&model, g4_formats::game::Family::Pt)
            .unwrap()
            .model;
        // Synthetic area data 0 selects the only tileset, area data 1 a missing one.
        let tileset = b"BTX0\0\0\0\0";
        let textured = glb_cache_key(TOOL, &model, Some(tileset));
        let plain = glb_cache_key(TOOL, &model, None);
        std::fs::write(models.join(format!("{textured}.glb")), b"glTFtextured").unwrap();
        std::fs::write(models.join(format!("{plain}.glb")), b"glTFplain").unwrap();
        remember_tool(&models, TOOL).unwrap();
        // Hits are served without apicula.
        assert_eq!(
            get_map_model_glb_with(&state, 0, 0, None).unwrap(),
            b"glTFtextured"
        );
        assert_eq!(
            get_map_model_glb_with(&state, 0, 1, None).unwrap(),
            b"glTFplain"
        );
        assert_eq!(
            get_map_model_glb_with(&state, 0, 200, None).unwrap(),
            b"glTFplain"
        );
    }

    #[test]
    fn an_unsaved_area_data_edit_changes_the_models_tileset() {
        let (fixture, state) = open_synthetic();
        let models = fixture.dir.path().join(".g4/models");
        std::fs::create_dir_all(&models).unwrap();
        let model = synthetic_model();
        let textured = glb_cache_key(TOOL, &model, Some(b"BTX0\0\0\0\0"));
        let plain = glb_cache_key(TOOL, &model, None);
        std::fs::write(models.join(format!("{textured}.glb")), b"glTFtextured").unwrap();
        std::fs::write(models.join(format!("{plain}.glb")), b"glTFplain").unwrap();
        remember_tool(&models, TOOL).unwrap();
        // Area data 1 selects a tileset the archive does not have: untextured.
        assert_eq!(
            get_map_model_glb_with(&state, 0, 1, None).unwrap(),
            b"glTFplain"
        );
        let mut a = get_area_data_impl(&state, 1).unwrap();
        if let AreaDataDto::Dppt { map_tileset, .. } = &mut a {
            *map_tileset = 0;
        }
        set_area_data_impl(&state, 1, a, "map tileset").unwrap();
        // Nothing was saved, yet the model now follows the edited record ...
        assert_eq!(
            get_map_model_glb_with(&state, 0, 1, None).unwrap(),
            b"glTFtextured"
        );
        // ... and undo takes it back.
        assert!(!undo_impl(&state).unwrap().is_empty());
        assert_eq!(
            get_map_model_glb_with(&state, 0, 1, None).unwrap(),
            b"glTFplain"
        );
    }

    /// Names in the cache directory except the `last-tool` bookkeeping file.
    #[cfg(unix)]
    fn cache_files(models: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(models)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n != "last-tool")
            .collect();
        names.sort();
        names
    }

    #[cfg(unix)]
    #[test]
    fn model_glb_is_converted_once_and_leaves_no_work_files() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let tool = fixture.dir.path().join("fake-apicula");
        std::fs::write(
            &tool,
            "#!/bin/sh\nwhile [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
             mkdir -p \"$out\"; printf 'FAKEGLB' > \"$out/m.glb\"\n"
                .replace("FAKEGLB", crate::commands::FAKE_GLB_PRINTF),
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        let models = fixture.dir.path().join(".g4/models");
        // Besides the model only the remembered tool hash is left: no work directories.
        let names = cache_files(&models);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].ends_with(".glb"), "{names:?}");
        // Second call is a cache hit and does not need the tool.
        assert_eq!(
            get_map_model_glb_with(&state, 1, 0, None).unwrap(),
            crate::commands::fake_glb()
        );
        // A map with a missing area data entry still converts, just without textures.
        let glb = get_map_model_glb_with(&state, 1, 200, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        assert_eq!(cache_files(&models).len(), 2);
        assert!(models.join("last-tool").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn model_glb_falls_back_to_untextured_when_the_textured_conversion_fails() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = g4_kernel::test_support::synthetic_project();
        let state = AppState::default();
        open_project_impl(&state, fixture.dir.path().to_str().unwrap()).unwrap();
        let tool = fixture.dir.path().join("fake-apicula");
        // argv: convert -f=glb <nsbmd> [<nsbtx>] -o <dir>; a fourth argument other than -o is
        // the tileset, which this tool rejects (a "bad tileset").
        std::fs::write(
            &tool,
            "#!/bin/sh\n[ \"$4\" = \"-o\" ] || { echo 'bad tileset' >&2; exit 2; }\n\
             while [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
             mkdir -p \"$out\"; printf 'FAKEGLB' > \"$out/m.glb\"\n"
                .replace("FAKEGLB", crate::commands::FAKE_GLB_PRINTF),
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        // Area data 0 selects the synthetic tileset: the textured run fails, the retry
        // without the tileset yields an untextured model instead of an error.
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        // The untextured result is cached under the plain key.
        let models = fixture.dir.path().join(".g4/models");
        let names: Vec<_> = std::fs::read_dir(&models)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(
            names.iter().filter(|n| n.ends_with(".glb")).count(),
            1,
            "{names:?}"
        );
        // A model that fails without a tileset too is still an error.
        std::fs::write(&tool, "#!/bin/sh\necho boom >&2; exit 3\n").unwrap();
        let err = get_map_model_glb_with(&state, 0, 0, Some(&tool)).unwrap_err();
        assert!(err.message.contains("boom"), "{}", err.message);
    }

    #[cfg(unix)]
    fn counting_tool(dir: &Path, glb: &str, fail_with_tileset: bool) -> (PathBuf, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let tool = dir.join("fake-apicula");
        let calls = dir.join("apicula-calls");
        // argv: convert -f=glb <nsbmd> [<nsbtx>] -o <dir>. Logs "plain" or "textured" per run.
        let fail = if fail_with_tileset {
            "[ \"$4\" = \"-o\" ] || { echo 'bad tileset' >&2; exit 2; }\n"
        } else {
            ""
        };
        std::fs::write(
            &tool,
            format!(
                "#!/bin/sh\nif [ \"$4\" = \"-o\" ]; then echo plain >> '{calls}'; \
                 else echo textured >> '{calls}'; fi\n{fail}\
                 while [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
                 mkdir -p \"$out\"; printf '{glb}' > \"$out/m.glb\"\n",
                calls = calls.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        (tool, calls)
    }

    #[cfg(unix)]
    fn recorded_calls(calls: &Path) -> Vec<String> {
        std::fs::read_to_string(calls)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[cfg(unix)]
    #[test]
    fn failing_tileset_is_remembered_and_not_retried() {
        let (fixture, state) = open_synthetic();
        let (tool, calls) =
            counting_tool(fixture.dir.path(), crate::commands::FAKE_GLB_PRINTF, true);
        let models = fixture.dir.path().join(".g4/models");
        assert_eq!(map_model_texture_status_impl(&state, 1, 0).unwrap(), None);

        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        assert_eq!(recorded_calls(&calls), ["textured", "plain"]);
        let fails: Vec<_> = std::fs::read_dir(&models)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".fail"))
            .collect();
        assert_eq!(fails.len(), 1, "one marker");
        let issue = map_model_texture_status_impl(&state, 1, 0)
            .unwrap()
            .unwrap();
        assert_eq!(issue.tileset, Some(0));
        assert!(issue.reason.contains("bad tileset"), "{}", issue.reason);

        // Even with the untextured result gone, the tileset is not tried again.
        for e in std::fs::read_dir(&models).unwrap() {
            let path = e.unwrap().path();
            if path.extension().is_some_and(|x| x == "glb") {
                std::fs::remove_file(path).unwrap();
            }
        }
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        assert_eq!(recorded_calls(&calls), ["textured", "plain", "plain"]);
        // A map without a tileset problem reports none.
        assert_eq!(map_model_texture_status_impl(&state, 1, 200).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_failure_that_is_not_the_tilesets_leaves_no_marker() {
        let (fixture, state) = open_synthetic();
        let (tool, _calls) = counting_tool(fixture.dir.path(), "x", true);
        std::fs::write(&tool, "#!/bin/sh\necho boom >&2; exit 3\n").unwrap();
        assert!(get_map_model_glb_with(&state, 1, 0, Some(&tool)).is_err());
        let models = fixture.dir.path().join(".g4/models");
        let fails = std::fs::read_dir(&models)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == "fail")
            })
            .count();
        assert_eq!(fails, 0);
        assert_eq!(map_model_texture_status_impl(&state, 1, 0).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_area_data_or_tileset_member_still_converts_untextured() {
        let (fixture, state) = open_synthetic();
        let (tool, calls) =
            counting_tool(fixture.dir.path(), crate::commands::FAKE_GLB_PRINTF, false);
        let project = Project::open(fixture.dir.path()).unwrap();
        project.ensure_unpacked(AssetKind::AreaData).unwrap();
        project.ensure_unpacked(AssetKind::MapTexture).unwrap();

        // A truncated area data member.
        std::fs::write(project.member_path(AssetKind::AreaData, 0), [1, 2, 3]).unwrap();
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        assert_eq!(recorded_calls(&calls), ["plain"]);
        let issue = map_model_texture_status_impl(&state, 1, 0)
            .unwrap()
            .unwrap();
        assert_eq!(issue.tileset, None);
        assert!(!issue.reason.is_empty());

        // A tileset member that cannot be read.
        use std::os::unix::fs::PermissionsExt;
        let member = project.member_path(AssetKind::MapTexture, 0);
        std::fs::set_permissions(&member, std::fs::Permissions::from_mode(0o000)).unwrap();
        std::fs::write(project.member_path(AssetKind::AreaData, 0), [0u8; 8]).unwrap();
        let glb = get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        let issue = map_model_texture_status_impl(&state, 1, 0)
            .unwrap()
            .unwrap();
        assert_eq!(issue.tileset, Some(0));
    }

    /// A fake apicula that logs "plain" or "textured" per run to a counter file and, once the
    /// `slow` marker exists, hangs on the given kind of run (`"textured"`, or `"any"`); a
    /// textured run without the marker fails fast. The script embeds the temp paths, so its
    /// hash is unique to the test. Returns tool, counter and marker paths; the tool has been
    /// run once already (macOS scans a fresh executable on first exec, which can take a second
    /// under load and would eat the whole time limit).
    #[cfg(unix)]
    fn hanging_tool(dir: &Path, hang_on: &str) -> (PathBuf, PathBuf, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let tool = dir.join("fake-apicula");
        let calls = dir.join("apicula-calls");
        let slow = dir.join("slow");
        let hang = if hang_on == "any" {
            "[ -e '{slow}' ] && exec sleep 5\n"
        } else {
            ""
        };
        // argv: convert -f=glb <nsbmd> [<nsbtx>] -o <dir>.
        std::fs::write(
            &tool,
            format!(
                "#!/bin/sh\nif [ \"$4\" = \"-o\" ]; then echo plain >> '{calls}'; kind=plain; \
                 else echo textured >> '{calls}'; kind=textured; fi\n{hang}\
                 if [ \"$kind\" = textured ]; then [ -e '{slow}' ] && exec sleep 5; \
                 echo 'bad tileset' >&2; exit 2; fi\n\
                 while [ $# -gt 0 ]; do [ \"$1\" = \"-o\" ] && out=\"$2\"; shift; done\n\
                 mkdir -p \"$out\"; printf '{glb}' > \"$out/m.glb\"\n",
                calls = calls.display(),
                slow = slow.display(),
                glb = crate::commands::FAKE_GLB_PRINTF,
                hang = hang.replace("{slow}", &slow.display().to_string()),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        // Warm-up run (a textured one, which fails fast without the marker), then reset.
        let _ = std::process::Command::new(&tool)
            .args(["convert", "-f=glb", "a", "b", "-o", "c"])
            .output();
        std::fs::remove_file(&calls).unwrap();
        std::fs::write(&slow, "").unwrap();
        (tool, calls, slow)
    }

    #[cfg(unix)]
    #[test]
    fn a_hanging_tileset_falls_back_to_untextured_without_marker_and_is_not_retried() {
        let (fixture, state) = open_synthetic();
        let (tool, calls, _slow) = hanging_tool(fixture.dir.path(), "textured");
        let models = fixture.dir.path().join(".g4/models");

        // Area data 0 selects the synthetic tileset: the textured run hangs and is killed,
        // the untextured one yields the model. No `.fail` marker is written.
        let started = std::time::Instant::now();
        let glb =
            get_map_model_glb_timed(&state, 1, 0, Some(&tool), Duration::from_secs(1)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "{:?}",
            started.elapsed()
        );
        assert_eq!(recorded_calls(&calls), ["textured", "plain"]);
        let names = cache_files(&models);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].ends_with(".glb"), "{names:?}");
        assert_eq!(map_model_texture_status_impl(&state, 1, 0).unwrap(), None);

        // Later loads skip the textured attempt, also when the untextured model must be
        // converted again.
        get_map_model_glb_timed(&state, 1, 0, Some(&tool), Duration::from_secs(1)).unwrap();
        std::fs::remove_file(models.join(&names[0])).unwrap();
        let glb =
            get_map_model_glb_timed(&state, 1, 0, Some(&tool), Duration::from_secs(1)).unwrap();
        assert_eq!(glb, crate::commands::fake_glb());
        assert_eq!(recorded_calls(&calls), ["textured", "plain", "plain"]);
        assert!(cache_files(&models).iter().all(|n| !n.ends_with(".fail")));
    }

    #[cfg(unix)]
    #[test]
    fn timeout_records_no_fail_marker() {
        let (fixture, state) = open_synthetic();
        let (tool, calls, _slow) = hanging_tool(fixture.dir.path(), "any");

        // Textured and untextured run both hang: the error is the timeout.
        let started = std::time::Instant::now();
        let err =
            get_map_model_glb_timed(&state, 1, 0, Some(&tool), Duration::from_secs(1)).unwrap_err();
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
        assert_eq!(err.code, "error");
        assert_eq!(err.message, "apicula did not finish within 1 s");
        assert_eq!(recorded_calls(&calls), ["textured", "plain"]);
        // No marker, no model, no work directory: only the remembered tool hash is left, and
        // the tileset is not reported as broken.
        let models = fixture.dir.path().join(".g4/models");
        assert_eq!(cache_files(&models), Vec::<String>::new());
        assert_eq!(map_model_texture_status_impl(&state, 1, 0).unwrap(), None);
        // Without tileset the untextured conversion times out the same way.
        let err = get_map_model_glb_timed(&state, 1, 200, Some(&tool), Duration::from_secs(1))
            .unwrap_err();
        assert_eq!(err.message, "apicula did not finish within 1 s");
        assert_eq!(cache_files(&models), Vec::<String>::new());
    }

    #[cfg(unix)]
    #[test]
    fn a_different_apicula_binary_starts_with_a_fresh_cache() {
        let (fixture, state) = open_synthetic();
        let (tool, calls) =
            counting_tool(fixture.dir.path(), crate::commands::FAKE_GLB_PRINTF, true);
        get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(recorded_calls(&calls), ["textured", "plain"]);
        // The same binary: model and `.fail` marker are hits.
        get_map_model_glb_with(&state, 1, 0, Some(&tool)).unwrap();
        assert_eq!(recorded_calls(&calls), ["textured", "plain"]);
        // An updated apicula (other bytes at the same path): the tileset is tried afresh. The
        // tool hash is memoised per path, so the update is a second path here.
        let updated = fixture.dir.path().join("fake-apicula-2");
        let script = std::fs::read_to_string(&tool).unwrap();
        std::fs::write(&updated, format!("{script}# updated\n")).unwrap();
        std::fs::set_permissions(&updated, std::fs::metadata(&tool).unwrap().permissions())
            .unwrap();
        get_map_model_glb_with(&state, 1, 0, Some(&updated)).unwrap();
        assert_eq!(
            recorded_calls(&calls),
            ["textured", "plain", "textured", "plain"]
        );
        // Without apicula the cache of the last used one is what is visible.
        get_map_model_glb_with(&state, 1, 0, None).unwrap();
        assert_eq!(recorded_calls(&calls).len(), 4);
        assert_eq!(
            map_model_texture_status_impl(&state, 1, 0)
                .unwrap()
                .map(|i| i.tileset),
            Some(Some(0))
        );
    }

    /// Timing of the backend part of loading a large matrix, as the frontend drives it: one
    /// `get_map_model_glb` per distinct map, four at a time. Gated (`G4_TEST_PROJECT`, a
    /// vanilla copy is used) and `#[ignore]`d; run with
    /// `G4_APICULA=... cargo test -p studio --release -- --ignored --nocapture bench_matrix_models`.
    /// `G4_BENCH_HEADER` picks the header (default 3: Platinum C01 with its 900-cell matrix).
    #[test]
    #[ignore = "timing run, minutes on a cold cache"]
    fn bench_matrix_models() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Instant;
        let Some((_tmp, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let state = AppState::default();
        open_project_impl(&state, root.to_str().unwrap()).unwrap();
        let header: usize = std::env::var("G4_BENCH_HEADER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let h = list_headers_impl(&state).unwrap().headers[header].clone();
        let matrix = get_matrix_impl(&state, h.matrix_id).unwrap();
        let mut ids: Vec<u16> = matrix.maps.iter().flatten().copied().collect();
        let cells = ids.len();
        ids.sort_unstable();
        ids.dedup();
        let apicula = find_apicula();
        let t = Instant::now();
        for id in &ids {
            get_map_impl(&state, *id).unwrap();
        }
        eprintln!("get_map x{}: {:?}", ids.len(), t.elapsed());
        for pass in ["cold", "warm"] {
            let next = AtomicUsize::new(0);
            let failed = AtomicUsize::new(0);
            let bytes = AtomicUsize::new(0);
            let t = Instant::now();
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    scope.spawn(|| {
                        loop {
                            let i = next.fetch_add(1, Ordering::SeqCst);
                            let Some(id) = ids.get(i) else { break };
                            match get_map_model_glb_with(
                                &state,
                                *id,
                                h.area_data_id,
                                apicula.as_deref(),
                            ) {
                                Ok(glb) => bytes.fetch_add(glb.len(), Ordering::SeqCst),
                                Err(_) => failed.fetch_add(1, Ordering::SeqCst),
                            };
                        }
                    });
                }
            });
            eprintln!(
                "{pass}: {} models ({cells} cells, header {} {}) in {:?}, {} failed, {} MB",
                ids.len(),
                h.id,
                h.internal_name,
                t.elapsed(),
                failed.load(Ordering::SeqCst),
                bytes.load(Ordering::SeqCst) / 1_000_000
            );
        }
    }
}
