//! The message catalogs of the Tauri crate: `en.yml` and `de.yml` have the same keys, the same
//! `%{…}` placeholders per key and no empty values; the `set_locale` command switches the
//! language of the process.

use std::path::Path;

use g4_kernel::i18n::{check_catalogs, locale};
use g4_kernel::test_support::synthetic_project;
use serde_json::json;
use studio_lib::AppState;
use studio_lib::dispatch::dispatch;
use studio_lib::policy::UserPolicy;

#[test]
fn catalogs_agree() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let n = check_catalogs(&dir).unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert!(n >= 1, "the catalogs have no keys");
}

/// The only test of this file that changes the locale (an own process: no other test sees it).
#[test]
fn set_locale_command_sets_the_process_locale() {
    let project = synthetic_project();
    let policy = UserPolicy::new(project.dir.path(), &[], None).unwrap();
    let state = AppState::default();
    for (lang, expected) in [("de-AT", "de"), ("xx", "en"), ("de", "de"), ("en", "en")] {
        dispatch(&state, &policy, "set_locale", json!({ "lang": lang })).unwrap();
        assert_eq!(locale(), expected, "{lang}");
    }
}
