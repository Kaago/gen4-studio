//! The message catalogs of the Tauri crate: `en.yml` and `de.yml` have the same keys, the same
//! `%{…}` placeholders per key and no empty values; the `set_locale` command switches the
//! language of the process, and the field labels, command errors, the MCP glossary, prompts and
//! access texts follow it. `system_language` answers the OS locale for "System", and the macOS
//! bundle declares both languages.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use g4_kernel::i18n::{check_catalogs, locale, set_locale};
use g4_kernel::test_support::synthetic_project;
use serde_json::{Map, Value, json};
use studio_lib::AppState;
use studio_lib::dispatch::{DispatchError, Reply, dispatch, field_label};
use studio_lib::mcp::access::{Access, Class, admit, classify};
use studio_lib::mcp::prompts::PromptDef;
use studio_lib::mcp::{docs, prompts};
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

#[test]
fn system_language_command_answers_a_locale_or_null_and_changes_nothing() {
    let _locale = LocaleGuard::take();
    let project = synthetic_project();
    let policy = UserPolicy::new(project.dir.path(), &[], None).unwrap();
    let state = AppState::default();
    set_locale("de");
    let reply = dispatch(&state, &policy, "system_language", json!({})).unwrap();
    let Reply::Json(value) = reply else {
        panic!("system_language answers JSON");
    };
    match value {
        Value::Null => {}
        Value::String(locale) => assert!(!locale.trim().is_empty(), "{locale:?}"),
        other => panic!("not a locale: {other}"),
    }
    // Asking is no choice: the language of the process stays.
    assert_eq!(locale(), "de");
}

/// The macOS bundle names English and German as its localizations (`Info.plist`, which Tauri
/// merges): WKWebView's `navigator.language` and WebKit's own texts follow only those.
#[test]
fn the_macos_bundle_declares_english_and_german() {
    let plist =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Info.plist")).unwrap();
    let after = |key: &str| {
        let at = plist
            .find(&format!("<key>{key}</key>"))
            .unwrap_or_else(|| panic!("{key} missing"));
        plist[at..].to_owned()
    };
    assert!(
        after("CFBundleDevelopmentRegion")
            .split("</string>")
            .next()
            .unwrap()
            .ends_with("<string>en"),
        "{plist}"
    );
    let localizations = after("CFBundleLocalizations");
    let array = &localizations[..localizations.find("</array>").unwrap()];
    for lang in ["en", "de"] {
        assert!(
            array.contains(&format!("<string>{lang}</string>")),
            "{plist}"
        );
    }
}

#[test]
fn glossary_follows_locale() {
    let _locale = LocaleGuard::take();
    set_locale("de");
    let german = docs::glossary();
    assert!(german.starts_with("# Glossar"), "{german}");
    assert!(german.contains("Gebiet"), "{german}");
    assert!(german.contains("Weltkarte"), "{german}");
    set_locale("en");
    let english = docs::glossary();
    assert!(english.starts_with("# Glossary"), "{english}");
    assert!(english.contains("Area"), "{english}");
    assert!(english.contains("World map"), "{english}");
    assert!(english.contains("**Matrix**"), "{english}");
    assert!(!english.contains("**Gebiet**"), "{english}");
    // Any other language is English, too.
    set_locale("xx");
    assert_eq!(docs::glossary(), english);
    // The guide to the server comes in the same two languages.
    set_locale("de");
    assert!(docs::mcp_guide().contains("# Gen4 Studio für Coding-Agenten"));
    set_locale("en");
    assert!(docs::mcp_guide().contains("# Gen4 Studio for coding agents"));
}

#[test]
fn prompts_and_access_texts_follow_locale() {
    let _locale = LocaleGuard::take();
    let project = synthetic_project();
    let policy = UserPolicy::new(project.dir.path(), &[], None).unwrap();
    let args: Map<String, Value> = json!({ "name": "Route 301" }).as_object().cloned().unwrap();
    let read = Access {
        write: false,
        irreversible: false,
        expert: false,
    };
    let refused = |lang: &str| {
        set_locale(lang);
        let class = classify("create_trainer", &json!({}), &policy);
        let error = admit("create_trainer", &class, &read).unwrap_err();
        (class, error.code, error.message)
    };
    let (class, code, message) = refused("de");
    assert_eq!(class, Class::Irreversible("Trainer anlegen".to_owned()));
    assert_eq!(code, "read_only");
    assert_eq!(message, "Nur lesen: create_trainer ändert das Projekt");
    let (class, code, message) = refused("en");
    assert_eq!(class, Class::Irreversible("Create trainer".to_owned()));
    assert_eq!(code, "read_only");
    assert_eq!(message, "Read only: create_trainer changes the project");

    set_locale("de");
    let (def, plan) = prompts::render("new_route", &args).unwrap();
    assert_eq!(def.title(), "Neue Route anlegen");
    assert!(
        def.description()
            .starts_with("Plan, um eine neue Route im Freien (Gebiet)")
    );
    assert_eq!(
        PromptDef::argument_description(def.arguments[0].1),
        "Name der neuen Route, zum Beispiel „Route 301“"
    );
    assert!(plan.starts_with("Lege in Gen4 Studio eine neue Route „Route 301“ an"));
    assert!(plan.contains("create_area"), "{plan}");
    set_locale("en");
    let (def, plan) = prompts::render("new_route", &args).unwrap();
    assert_eq!(def.name, "new_route");
    assert_eq!(def.title(), "Create a new route");
    assert!(
        def.description()
            .starts_with("Plan to create a new outdoor route (an area)")
    );
    assert_eq!(
        PromptDef::argument_description(def.arguments[0].1),
        "name of the new route, for example \"Route 301\""
    );
    assert!(plan.starts_with("Create a new route \"Route 301\" in Gen4 Studio"));
    assert!(plan.contains("create_area"), "{plan}");
    // Prompt and argument names are codes: the same in both languages.
    assert_eq!(def.arguments[0].0, "name");
    assert!(prompts::render("new_route", &Map::new()).is_err());
}
