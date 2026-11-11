//! Updates from GitHub Releases in three steps, each its own command so the webview can ask the
//! user in between: `update_check` looks for a newer signed build, `update_download` fetches
//! and verifies it, `update_install` installs the verified bytes. The unsaved edits are counted
//! at the install, the last moment before the app ends, not before the download that may take
//! minutes. The Tauri updater plugin does the network and the signature check (against
//! `plugins.updater.pubkey` in `tauri.conf.json`); this module decides when a step may start
//! and tells the webview how far it is. The webview only ever calls the commands here, never
//! the plugin's own (no `updater:*` permission is granted).

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

/// The event that reports the progress of the download and the start of the install to the
/// webview.
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

/// How far `update_download` and `update_install` are, sent as [`PROGRESS_EVENT`].
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum UpdateProgress {
    /// `update_download`: bytes downloaded so far, of `total` when the server told the size.
    Downloading { done: u64, total: Option<u64> },
    /// `update_install` has started: the verified bytes are being installed.
    Installing,
}

/// The update `update_check` found, its verified download, and whether a download or an
/// install is running.
#[derive(Default)]
pub struct UpdateState {
    inner: Mutex<Found<Update>>,
}

impl UpdateState {
    /// The state lock, recovered when a panic poisoned it: the state is plain values.
    fn lock(&self) -> MutexGuard<'_, Found<Update>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// What [`UpdateState`] holds, with the update's type open so the tests need no real one.
struct Found<U> {
    /// The update the last check found, with its version; the next check replaces it.
    update: Option<(String, U)>,
    /// The downloaded and verified package of `update`; replaced or dropped with it.
    bytes: Option<Vec<u8>>,
    /// A download or an install is running (an install that succeeds ends the app).
    busy: bool,
}

impl<U> Default for Found<U> {
    fn default() -> Self {
        Self {
            update: None,
            bytes: None,
            busy: false,
        }
    }
}

impl<U: Clone> Found<U> {
    /// Keeps what a check found in place of the older find and its download; no find clears
    /// both.
    fn replace(&mut self, found: Option<(String, U)>) {
        self.update = found;
        self.bytes = None;
    }

    /// The version of the found update.
    fn found_version(&self) -> Option<&str> {
        self.update.as_ref().map(|(version, _)| version.as_str())
    }

    /// Starts the download of `version`: the update to download, with the state marked busy;
    /// or why it may not start ([`download_refusal`]). Check and mark are one step, so two
    /// calls at once start one download. The found update stays where it is.
    fn begin_download(&mut self, version: &str) -> Result<U, CmdError> {
        if let Some(refusal) = download_refusal(self.found_version(), version, self.busy) {
            return Err(refusal);
        }
        let Some((_, update)) = &self.update else {
            return Err(not_found(version));
        };
        let update = update.clone();
        self.busy = true;
        Ok(update)
    }

    /// The download of `version` failed: not busy any more, the found update stays.
    fn download_failed(&mut self) {
        self.busy = false;
    }

    /// The download of `version` is complete and verified: not busy any more, `bytes` kept
    /// for the install. A check that replaced the found update meanwhile made the download
    /// worthless: `Err` then, nothing kept.
    fn downloaded(&mut self, version: &str, bytes: Vec<u8>) -> Result<(), CmdError> {
        self.busy = false;
        if self.found_version() != Some(version) {
            return Err(not_found(version));
        }
        self.bytes = Some(bytes);
        Ok(())
    }

    /// Starts the install of `version`: the update with its verified bytes, the state marked
    /// busy; or why it may not start ([`install_refusal`]). Check and mark are one step, so
    /// two calls at once start one install.
    fn begin_install(
        &mut self,
        version: &str,
        store_dirty: usize,
        discard_unsaved: bool,
    ) -> Result<(U, Vec<u8>), CmdError> {
        let downloaded = self.bytes.as_ref().and(self.found_version());
        if let Some(refusal) =
            install_refusal(downloaded, version, store_dirty, discard_unsaved, self.busy)
        {
            return Err(refusal);
        }
        let (Some((_, update)), Some(bytes)) = (&self.update, self.bytes.take()) else {
            return Err(not_found(version));
        };
        let update = update.clone();
        self.busy = true;
        Ok((update, bytes))
    }

    /// The install of `version` failed: not busy any more, with `bytes` kept again for
    /// another try unless a check replaced the update meanwhile.
    fn install_failed(&mut self, version: &str, bytes: Vec<u8>) {
        self.busy = false;
        if self.found_version() == Some(version) {
            self.bytes = Some(bytes);
        }
    }
}

fn not_found(version: &str) -> CmdError {
    t!("commands.update.not_found", version = version)
        .into_owned()
        .into()
}

fn running() -> CmdError {
    t!("commands.update.running").into_owned().into()
}

/// A failure of the updater plugin (the check, the download, the install) as the webview gets
/// it: the cause alone, in the plugin's words. The webview puts the stage in front of it, once
/// (`updates.checkFailed`, `updates.downloadFailed`, `updates.installFailed`), so a prefix here
/// would show up twice. Refusals that are no plugin failure (running, unsaved, not found) keep
/// their own sentences.
fn plugin_failure<E: std::fmt::Display>(e: E) -> CmdError {
    e.to_string().into()
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

/// Why `update_download(requested)` must not start, in this order: a download or an install
/// is running; no update was found, or another one than `requested`. `found` is the version
/// `update_check` found.
pub fn download_refusal(found: Option<&str>, requested: &str, busy: bool) -> Option<CmdError> {
    if busy {
        return Some(running());
    }
    if found != Some(requested) {
        return Some(not_found(requested));
    }
    None
}

/// Why `update_install(requested, …)` must not start, in this order: a download or an install
/// is running; no verified download for exactly `requested`; unsaved edits that the user did
/// not agree to discard (code `"unsaved"`). `downloaded` is the version whose package
/// `update_download` verified and kept.
pub fn install_refusal(
    downloaded: Option<&str>,
    requested: &str,
    store_dirty: usize,
    discard_unsaved: bool,
    busy: bool,
) -> Option<CmdError> {
    if busy {
        return Some(running());
    }
    if downloaded != Some(requested) {
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
/// `update_download` until the next check replaces it (and drops a download of the older one).
#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    state: State<'_, UpdateState>,
) -> Result<Option<UpdateInfo>, CmdError> {
    let check_failed = plugin_failure::<tauri_plugin_updater::Error>;
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
    state
        .lock()
        .replace(found.map(|update| (update.version.clone(), update)));
    Ok(info)
}

/// Downloads and verifies the update `update_check` found, which must be `version`, and keeps
/// the package for `update_install`. Reports [`PROGRESS_EVENT`] `Downloading` while it runs.
/// Installs nothing and asks nothing about unsaved edits: the user may keep working meanwhile.
/// A failed download keeps the found update.
#[tauri::command]
pub async fn update_download(
    app: AppHandle,
    state: State<'_, UpdateState>,
    version: String,
) -> Result<(), CmdError> {
    let update = state.lock().begin_download(&version)?;
    let mut throttle = ProgressThrottle::default();
    let downloaded = update
        .download(
            |chunk, total| {
                if let Some(progress) = throttle.advance(chunk, total) {
                    // A failed emit only means the webview misses one update.
                    let _ = app.emit(PROGRESS_EVENT, progress);
                }
            },
            || {},
        )
        .await;
    match downloaded {
        Ok(bytes) => state.lock().downloaded(&version, bytes),
        Err(e) => {
            state.lock().download_failed();
            Err(plugin_failure(e))
        }
    }
}

/// Installs the package `update_download` verified for `version`. Refuses while unsaved edits
/// exist unless `discard_unsaved` says the user agreed; the edits are counted here, right
/// before the app ends, so the ones made during the download count too. Reports
/// [`PROGRESS_EVENT`] `Installing` once. Afterwards the app restarts (on Windows the plugin
/// ends it and the installer starts the new version). A failed install keeps the update and
/// its package.
#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    state: State<'_, UpdateState>,
    version: String,
    discard_unsaved: bool,
) -> Result<(), CmdError> {
    let dirty = app.state::<AppState>().unsaved_count();
    let (update, bytes) = state
        .lock()
        .begin_install(&version, dirty, discard_unsaved)?;
    // A failed emit only means the webview misses the update.
    let _ = app.emit(PROGRESS_EVENT, UpdateProgress::Installing);
    match update.install(&bytes) {
        Ok(()) => restart(&app),
        Err(e) => {
            state.lock().install_failed(&version, bytes);
            Err(plugin_failure(e))
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
    fn a_plugin_failure_is_its_cause_alone() {
        // The webview names the stage; a prefix here would be shown twice.
        for (error, cause) in [
            (
                tauri_plugin_updater::Error::EmptyEndpoints,
                "Updater does not have any endpoints set.",
            ),
            (
                tauri_plugin_updater::Error::Network(
                    "Download request failed with status: 404 Not Found".to_owned(),
                ),
                "`Download request failed with status: 404 Not Found`",
            ),
            (
                tauri_plugin_updater::Error::ReleaseNotFound,
                "Could not fetch a valid release JSON from the remote",
            ),
        ] {
            let failure = plugin_failure(error);
            assert_eq!(failure.code, "error");
            assert_eq!(failure.message, cause);
        }
    }

    #[test]
    fn install_needs_the_version_that_was_downloaded() {
        // No download at all.
        let e = install_refusal(None, "1.0.2", 0, false, false).unwrap();
        assert_eq!(e.code, "error");
        assert!(e.message.contains("1.0.2"), "{}", e.message);
        // The download is of another version.
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
        // Without a download the missing package is the answer, not the edits.
        assert_eq!(
            install_refusal(None, "1.0.2", 3, false, false)
                .unwrap()
                .code,
            "error"
        );
    }

    #[test]
    fn a_second_install_is_refused() {
        let e = install_refusal(Some("1.0.2"), "1.0.2", 0, false, true).unwrap();
        assert_eq!(e.code, "error");
        assert!(e.message.contains("already"), "{}", e.message);
        // A running one wins over everything else.
        let e = install_refusal(None, "1.0.2", 3, false, true).unwrap();
        assert!(e.message.contains("already"), "{}", e.message);
    }

    #[test]
    fn a_download_needs_the_version_that_was_found_and_no_other_run() {
        let e = download_refusal(None, "1.0.2", false).unwrap();
        assert_eq!(e.code, "error");
        assert!(e.message.contains("1.0.2"), "{}", e.message);
        assert_eq!(
            download_refusal(Some("1.0.2"), "1.0.3", false)
                .unwrap()
                .code,
            "error"
        );
        assert!(download_refusal(Some("1.0.2"), "1.0.2", false).is_none());
        let e = download_refusal(Some("1.0.2"), "1.0.2", true).unwrap();
        assert_eq!(e.code, "error");
        assert!(e.message.contains("already"), "{}", e.message);
    }

    /// The state with `1.0.2` found (the update itself is a stand-in).
    fn found() -> Found<u8> {
        let mut state = Found::default();
        state.replace(Some(("1.0.2".to_owned(), 7)));
        state
    }

    #[test]
    fn a_download_keeps_the_found_update_and_blocks_other_runs() {
        let mut state = found();
        assert_eq!(state.begin_download("1.0.2").unwrap(), 7);
        assert!(state.busy);
        // A second download, an install, both while it runs.
        for refusal in [
            state.begin_download("1.0.2").unwrap_err(),
            state.begin_install("1.0.2", 0, true).unwrap_err(),
        ] {
            assert!(refusal.message.contains("already"), "{}", refusal.message);
        }
        // It failed: the update is still there for another try.
        state.download_failed();
        assert!(!state.busy);
        assert_eq!(state.begin_download("1.0.2").unwrap(), 7);
        // The retry succeeds: not busy, the package is kept for the install.
        state.downloaded("1.0.2", vec![1, 2, 3]).unwrap();
        assert!(!state.busy);
        assert_eq!(state.bytes.as_deref(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn install_without_a_download_or_for_another_version_is_refused() {
        let mut state = found();
        // Found, not downloaded.
        let e = state.begin_install("1.0.2", 0, true).unwrap_err();
        assert_eq!(e.code, "error");
        assert!(!state.busy, "a refusal marks nothing");
        state.begin_download("1.0.2").unwrap();
        state.downloaded("1.0.2", vec![1]).unwrap();
        // Downloaded, but another version asked for.
        let e = state.begin_install("1.0.3", 0, true).unwrap_err();
        assert_eq!(e.code, "error");
        assert!(e.message.contains("1.0.3"), "{}", e.message);
        assert!(!state.busy);
        assert!(state.bytes.is_some(), "a refusal keeps the package");
        // Unsaved edits without consent, then with it.
        assert_eq!(
            state.begin_install("1.0.2", 2, false).unwrap_err().code,
            "unsaved"
        );
        assert!(!state.busy);
        assert_eq!(state.begin_install("1.0.2", 2, true).unwrap(), (7, vec![1]));
        assert!(state.busy);
        assert!(
            state.begin_install("1.0.2", 0, true).is_err(),
            "one install only"
        );
    }

    #[test]
    fn a_failed_install_keeps_the_update_and_its_package() {
        let mut state = found();
        state.begin_download("1.0.2").unwrap();
        state.downloaded("1.0.2", vec![4, 5]).unwrap();
        let (_, bytes) = state.begin_install("1.0.2", 0, false).unwrap();
        state.install_failed("1.0.2", bytes);
        assert!(!state.busy);
        assert_eq!(state.begin_install("1.0.2", 0, false).unwrap().1, [4, 5]);
    }

    #[test]
    fn a_new_check_replaces_the_update_and_drops_its_package() {
        let mut state = found();
        state.begin_download("1.0.2").unwrap();
        state.downloaded("1.0.2", vec![1]).unwrap();
        state.replace(Some(("1.0.3".to_owned(), 8)));
        assert!(state.bytes.is_none());
        assert_eq!(
            state.begin_install("1.0.2", 0, true).unwrap_err().code,
            "error"
        );
        assert_eq!(
            state.begin_install("1.0.3", 0, true).unwrap_err().code,
            "error"
        );
        // Nothing found drops both as well.
        state.begin_download("1.0.3").unwrap();
        state.downloaded("1.0.3", vec![2]).unwrap();
        state.replace(None);
        assert!(state.bytes.is_none());
        assert!(state.begin_download("1.0.3").is_err());
    }

    #[test]
    fn a_download_that_a_check_overtook_is_not_kept() {
        // The download of 1.0.2 runs, a check finds 1.0.3 meanwhile.
        let mut state = found();
        state.begin_download("1.0.2").unwrap();
        state.replace(Some(("1.0.3".to_owned(), 8)));
        let e = state.downloaded("1.0.2", vec![1]).unwrap_err();
        assert_eq!(e.code, "error");
        assert!(e.message.contains("1.0.2"), "{}", e.message);
        assert!(state.bytes.is_none());
        assert!(!state.busy);
        // An install that fails after a check replaced the update does not bring its package back.
        let mut state = found();
        state.begin_download("1.0.2").unwrap();
        state.downloaded("1.0.2", vec![1]).unwrap();
        let (_, bytes) = state.begin_install("1.0.2", 0, true).unwrap();
        state.replace(Some(("1.0.3".to_owned(), 8)));
        state.install_failed("1.0.2", bytes);
        assert!(state.bytes.is_none());
        assert!(!state.busy);
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
