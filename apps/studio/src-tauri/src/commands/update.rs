//! Updates from GitHub Releases: `update_check` looks for a newer signed build, `update_install`
//! downloads, verifies and installs the one it found. The Tauri updater plugin does the network
//! and the signature check (against `plugins.updater.pubkey` in `tauri.conf.json`); this module
//! decides when an install may start and tells the webview how far it is. The webview only ever
//! calls the two commands here, never the plugin's own (no `updater:*` permission is granted).

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::errors::CmdError;
use crate::state::AppState;

/// The environment variable that replaces the update address (acceptance of a release
/// candidate: `latest.json` of a pre-release). Only an `https` address is taken.
pub const ENDPOINT_VAR: &str = "G4_UPDATE_ENDPOINT";

/// The event that reports the progress of `update_install` to the webview.
pub const PROGRESS_EVENT: &str = "update-progress";

/// How long the check may take before it counts as failed.
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);

/// Without a known size, a progress event goes out each time this many bytes arrived.
const UNKNOWN_SIZE_STEP: u64 = 512 * 1024;

/// What `update_check` tells the webview about a newer build.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// The version that is available.
    pub version: String,
    /// The version that runs.
    pub current: String,
    /// The release notes, if the release has any.
    pub notes: Option<String>,
}

/// How far `update_install` is, sent as [`PROGRESS_EVENT`].
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum UpdateProgress {
    /// Bytes downloaded so far, of `total` when the server told the size.
    Downloading { done: u64, total: Option<u64> },
    /// The download is complete and verified; the installer runs.
    Installing,
}

/// The update `update_check` found, and whether an install is running.
#[derive(Default)]
pub struct UpdateState {
    inner: Mutex<Found>,
}

#[derive(Default)]
struct Found {
    /// The update the last check found; an install takes it, the next check replaces it.
    update: Option<Update>,
    /// An install has started and has not failed (on success the app ends).
    installing: bool,
}

impl UpdateState {
    /// The state lock, recovered when a panic poisoned it: the state is two plain values.
    fn lock(&self) -> MutexGuard<'_, Found> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Keeps what the last check found in place of an older find; no find clears it.
    fn keep(&self, found: Option<Update>) {
        self.lock().update = found;
    }

    /// Starts the install of `version`: the update to install, with the install marked as
    /// running; or why it may not start ([`install_refusal`]). Check and mark are one step, so
    /// two calls at once start one install.
    fn begin_install(
        &self,
        version: &str,
        store_dirty: usize,
        discard_unsaved: bool,
    ) -> Result<Update, CmdError> {
        let mut state = self.lock();
        let pending = state.update.as_ref().map(|u| u.version.as_str());
        if let Some(refusal) = install_refusal(
            pending,
            version,
            store_dirty,
            discard_unsaved,
            state.installing,
        ) {
            return Err(refusal);
        }
        let Some(update) = state.update.take() else {
            return Err(not_found(version));
        };
        state.installing = true;
        Ok(update)
    }

    /// The install failed: another one may start, with `update` again unless a later check
    /// found one meanwhile.
    fn install_failed(&self, update: Update) {
        let mut state = self.lock();
        state.installing = false;
        state.update.get_or_insert(update);
    }
}

fn not_found(version: &str) -> CmdError {
    t!("commands.update.not_found", version = version)
        .into_owned()
        .into()
}

/// The address in [`ENDPOINT_VAR`], if one is set: `Ok(None)` for none (or an empty one), an
/// error naming the value when it is no `https` address.
pub fn endpoint_override(raw: Option<&str>) -> Result<Option<tauri::Url>, String> {
    let Some(raw) = raw.filter(|raw| !raw.is_empty()) else {
        return Ok(None);
    };
    let bad = || t!("commands.update.bad_endpoint", value = raw).into_owned();
    if !raw.starts_with("https://") {
        return Err(bad());
    }
    raw.parse().map(Some).map_err(|_| bad())
}

/// Why `update_install(requested, …)` must not start, in this order: an install is running;
/// no update was found, or another one than `requested`; unsaved edits that the user did not
/// agree to discard (code `"unsaved"`). `pending` is the version `update_check` found.
pub fn install_refusal(
    pending: Option<&str>,
    requested: &str,
    store_dirty: usize,
    discard_unsaved: bool,
    installing: bool,
) -> Option<CmdError> {
    if installing {
        return Some(t!("commands.update.running").into_owned().into());
    }
    if pending != Some(requested) {
        return Some(not_found(requested));
    }
    if store_dirty > 0 && !discard_unsaved {
        return Some(CmdError::refused(
            "unsaved",
            t!("commands.update.unsaved").into_owned(),
        ));
    }
    None
}

/// Decides which download chunks are worth an event: each time the whole percent changes, and
/// without a known size each [`UNKNOWN_SIZE_STEP`] bytes. The first chunk always is.
#[derive(Default)]
struct ProgressThrottle {
    done: u64,
    /// The percent (or step) of the last event; `None` before the first.
    last: Option<u64>,
}

impl ProgressThrottle {
    /// Counts `chunk` bytes; the event to send for it, if any. `total` is the size the server
    /// named (an empty one counts as none).
    fn advance(&mut self, chunk: usize, total: Option<u64>) -> Option<UpdateProgress> {
        self.done = self.done.saturating_add(chunk as u64);
        let total = total.filter(|&total| total > 0);
        let mark = match total {
            // The server's size can fall short of what arrives: never more than 100.
            Some(total) => (u128::from(self.done) * 100 / u128::from(total)).min(100) as u64,
            None => self.done / UNKNOWN_SIZE_STEP,
        };
        if self.last == Some(mark) {
            return None;
        }
        self.last = Some(mark);
        Some(UpdateProgress::Downloading {
            done: self.done,
            total,
        })
    }
}

/// Looks for a newer build. `None`: this one is the newest. A found update is kept for
/// `update_install` until it is installed or the next check replaces it.
#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    state: State<'_, UpdateState>,
) -> Result<Option<UpdateInfo>, CmdError> {
    let check_failed = |e: tauri_plugin_updater::Error| {
        CmdError::from(t!("commands.update.check_failed", error = e).into_owned())
    };
    let mut builder = app.updater_builder().timeout(CHECK_TIMEOUT);
    // Windows ends the app inside the plugin: take the helpers that outlive it down first.
    let exiting = app.clone();
    builder = builder.on_before_exit(move || {
        crate::lsp::stop_on_exit(&exiting);
        crate::live_hub::stop_on_exit(&exiting);
        exiting.cleanup_before_exit();
    });
    if let Some(url) = endpoint_override(std::env::var(ENDPOINT_VAR).ok().as_deref())? {
        builder = builder.endpoints(vec![url]).map_err(check_failed)?;
    }
    let found = builder
        .build()
        .map_err(check_failed)?
        .check()
        .await
        .map_err(check_failed)?;
    let info = found.as_ref().map(|update| UpdateInfo {
        version: update.version.clone(),
        current: update.current_version.clone(),
        notes: update.body.clone(),
    });
    state.keep(found);
    Ok(info)
}

/// Downloads, verifies and installs the update `update_check` found, which must be `version`.
/// Refuses while unsaved edits exist unless `discard_unsaved` says the user agreed. Reports
/// [`PROGRESS_EVENT`]. Afterwards the app restarts (on Windows the plugin ends it and the
/// installer starts the new version).
#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    state: State<'_, UpdateState>,
    version: String,
    discard_unsaved: bool,
) -> Result<(), CmdError> {
    let dirty = app.state::<AppState>().unsaved_count();
    let update = state.begin_install(&version, dirty, discard_unsaved)?;
    // A failed emit only means the webview misses one update.
    let send = |progress: UpdateProgress| {
        let _ = app.emit(PROGRESS_EVENT, progress);
    };
    let mut throttle = ProgressThrottle::default();
    let result = update
        .download_and_install(
            |chunk, total| {
                if let Some(progress) = throttle.advance(chunk, total) {
                    send(progress);
                }
            },
            || send(UpdateProgress::Installing),
        )
        .await;
    match result {
        Ok(()) => restart(&app),
        Err(e) => {
            state.install_failed(update);
            Err(t!("commands.update.install_failed", error = e)
                .into_owned()
                .into())
        }
    }
}

/// The new version is in place: start it. `force_quit` lets the exit request of the restart
/// pass the unsaved-edits guard (the user agreed before the install began).
#[cfg(not(windows))]
fn restart(app: &AppHandle) -> Result<(), CmdError> {
    app.state::<AppState>()
        .force_quit
        .store(true, std::sync::atomic::Ordering::SeqCst);
    app.restart()
}

/// The plugin ends the app itself once the installer runs; only a plugin that returned
/// without doing so gets here.
#[cfg(windows)]
fn restart(_app: &AppHandle) -> Result<(), CmdError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_override_without_the_variable() {
        assert_eq!(endpoint_override(None).unwrap(), None);
        assert_eq!(endpoint_override(Some("")).unwrap(), None);
    }

    #[test]
    fn an_https_override_is_taken() {
        let raw = "https://github.com/Kaago/gen4-studio/releases/download/v1.0.1-rc.2/latest.json";
        assert_eq!(endpoint_override(Some(raw)).unwrap().unwrap().as_str(), raw);
    }

    #[test]
    fn a_plain_http_override_is_refused() {
        let e = endpoint_override(Some("http://localhost:8000/latest.json")).unwrap_err();
        assert!(e.contains("http://localhost:8000/latest.json"), "{e}");
    }

    #[test]
    fn install_needs_the_version_that_was_found() {
        assert_eq!(
            install_refusal(None, "1.0.2", 0, false, false)
                .unwrap()
                .code,
            "error"
        );
        assert_eq!(
            install_refusal(Some("1.0.2"), "1.0.3", 0, false, false)
                .unwrap()
                .code,
            "error"
        );
        assert!(install_refusal(Some("1.0.2"), "1.0.2", 0, false, false).is_none());
    }

    #[test]
    fn unsaved_edits_need_consent() {
        assert_eq!(
            install_refusal(Some("1.0.2"), "1.0.2", 3, false, false)
                .unwrap()
                .code,
            "unsaved"
        );
        assert!(install_refusal(Some("1.0.2"), "1.0.2", 3, true, false).is_none());
    }

    #[test]
    fn a_second_install_is_refused() {
        let e = install_refusal(Some("1.0.2"), "1.0.2", 0, false, true).unwrap();
        assert_eq!(e.code, "error");
        assert!(e.message.contains("already"), "{}", e.message);
    }

    /// The `done` values of the events `chunks` cause, one chunk at a time.
    fn reported(chunks: impl IntoIterator<Item = usize>, total: Option<u64>) -> Vec<u64> {
        let mut throttle = ProgressThrottle::default();
        chunks
            .into_iter()
            .filter_map(|chunk| match throttle.advance(chunk, total) {
                Some(UpdateProgress::Downloading { done, total: sent }) => {
                    assert_eq!(sent, total, "the size is passed on");
                    Some(done)
                }
                Some(other) => panic!("{other:?}"),
                None => None,
            })
            .collect()
    }

    #[test]
    fn progress_is_reported_when_the_whole_percent_changes() {
        // 1000 bytes in chunks of 5: 200 chunks, 101 percent values (0 to 100), one event each.
        let done = reported(std::iter::repeat_n(5, 200), Some(1000));
        assert_eq!(done.first(), Some(&5));
        assert_eq!(done.last(), Some(&1000));
        assert_eq!(done.len(), 101, "{done:?}");
        // Chunks of 100 bytes: every one is a new percent (10 each).
        assert_eq!(
            reported(std::iter::repeat_n(100, 10), Some(1000)),
            [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]
        );
        // Chunks inside one percent are not reported: 1, 2, 3 bytes are 0 %, 11 and 12 are
        // 1 %, 22 is 2 %.
        assert_eq!(reported([1, 1, 1, 8, 1], Some(1000)), [1, 11]);
        assert_eq!(reported([1, 1, 1, 8, 1, 10], Some(1000)), [1, 11, 22]);
    }

    #[test]
    fn progress_without_a_size_is_reported_every_512_kib() {
        const KIB: usize = 1024;
        // 100 KiB chunks: the first chunk, then the chunks that cross 512 KiB and 1024 KiB, ...
        let done = reported(std::iter::repeat_n(100 * KIB, 12), None);
        let kib: Vec<u64> = done.iter().map(|d| d / KIB as u64).collect();
        assert_eq!(kib, [100, 600, 1100]);
    }

    #[test]
    fn an_empty_size_counts_as_no_size_and_an_overrun_stops_at_100() {
        let mut throttle = ProgressThrottle::default();
        assert_eq!(
            throttle.advance(10, Some(0)),
            Some(UpdateProgress::Downloading {
                done: 10,
                total: None
            })
        );
        // More arrives than the server named: one event for 100, none after.
        assert_eq!(reported([60, 60, 60], Some(100)), [60, 120]);
    }

    #[test]
    fn progress_names_its_phase() {
        assert_eq!(
            serde_json::to_value(UpdateProgress::Downloading {
                done: 10,
                total: Some(20)
            })
            .unwrap(),
            serde_json::json!({ "phase": "downloading", "done": 10, "total": 20 })
        );
        assert_eq!(
            serde_json::to_value(UpdateProgress::Installing).unwrap(),
            serde_json::json!({ "phase": "installing" })
        );
    }
}
