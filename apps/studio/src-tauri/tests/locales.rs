//! The message catalogs of the Tauri crate: `en.yml` and `de.yml` have the same keys, the same
//! `%{…}` placeholders per key and no empty values; the `set_locale` command switches the
//! language of the process, and the field labels and command errors follow it.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use g4_kernel::i18n::{check_catalogs, locale, set_locale};
use g4_kernel::test_support::synthetic_project;
use serde_json::json;
use studio_lib::AppState;
use studio_lib::dispatch::{DispatchError, dispatch, field_label};
use studio_lib::policy::UserPolicy;

#[test]
fn catalogs_agree() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let n = check_catalogs(&dir).unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert!(n >= 1, "the catalogs have no keys");
}

/// The locale is process-wide and the tests of this file run in parallel: every test that
/// changes it holds this lock and leaves the locale at `en` (also when it fails).
static LOCALE_LOCK: Mutex<()> = Mutex::new(());

/// Holds [`LOCALE_LOCK`]; sets the locale back to `en` when dropped.
struct LocaleGuard(#[allow(dead_code)] MutexGuard<'static, ()>);

impl LocaleGuard {
    fn take() -> Self {
        Self(LOCALE_LOCK.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Drop for LocaleGuard {
    fn drop(&mut self) {
        set_locale("en");
    }
}

#[test]
fn field_labels_translate() {
    let _locale = LocaleGuard::take();
    set_locale("de");
    assert_eq!(
        field_label("species", "egg_cycles").as_deref(),
        Some("Schlüpfzyklen")
    );
    set_locale("en");
    assert_eq!(
        field_label("species", "egg_cycles").as_deref(),
        Some("Egg cycles")
    );
    // A field the kind does not list has no label (and cannot be edited).
    assert_eq!(field_label("species", "nonsense"), None);
}

#[test]
fn command_errors_follow_the_locale() {
    let _locale = LocaleGuard::take();
    let project = synthetic_project();
    let policy = UserPolicy::new(project.dir.path(), &[], None).unwrap();
    let state = AppState::default();
    let message = |lang: &str| {
        set_locale(lang);
        match dispatch(&state, &policy, "lsp_send", json!({ "body": "{}" })) {
            Err(DispatchError::Cmd(e)) => (e.code, e.message),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(message("de"), ("error", "rotom-lsp läuft nicht".to_owned()));
    assert_eq!(
        message("en"),
        ("error", "rotom-lsp is not running".to_owned())
    );
}

#[test]
fn doctor_report_follows_the_locale() {
    let _locale = LocaleGuard::take();
    let empty = tempfile::tempdir().unwrap();
    let report = |lang: &str| {
        set_locale(lang);
        studio_lib::doctor::probe_all(empty.path()).text()
    };
    let german = report("de");
    assert!(german.contains("apicula: fehlt"), "{german}");
    assert!(
        german.ends_with("doctor: Sidecar fehlt oder startet nicht"),
        "{german}"
    );
    let english = report("en");
    assert!(english.contains("apicula: missing"), "{english}");
    assert!(
        english.ends_with("doctor: a sidecar is missing or does not start"),
        "{english}"
    );
}

#[test]
fn set_locale_command_sets_the_process_locale() {
    let _locale = LocaleGuard::take();
    let project = synthetic_project();
    let policy = UserPolicy::new(project.dir.path(), &[], None).unwrap();
    let state = AppState::default();
    for (lang, expected) in [("de-AT", "de"), ("xx", "en"), ("de", "de"), ("en", "en")] {
        dispatch(&state, &policy, "set_locale", json!({ "lang": lang })).unwrap();
        assert_eq!(locale(), expected, "{lang}");
    }
}
