//! The i18n foundation of the kernel: the shipped catalogs agree, `check_catalogs` names what
//! is wrong with a pair of catalogs, the locale normalizes and the probe key translates.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use g4_kernel::i18n::{check_catalogs, locale, set_locale};
use rust_i18n::t;

// `t!` forwards to `crate::_rust_i18n_t!`, which `i18n!` generates: this test crate loads the
// same catalogs as the kernel (the kernel's own backend is checked through `g4_kernel::`).
rust_i18n::i18n!("locales", fallback = "en");

/// The locale is process-wide, so the tests that change it take turns.
static LOCALE_LOCK: Mutex<()> = Mutex::new(());

fn locale_lock() -> MutexGuard<'static, ()> {
    LOCALE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A folder with `en.yml` and `de.yml` of the given contents.
fn catalogs(en: &str, de: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("en.yml"), en).unwrap();
    std::fs::write(dir.path().join("de.yml"), de).unwrap();
    dir
}

#[test]
fn catalogs_agree() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let n = check_catalogs(&dir).unwrap_or_else(|errors| panic!("{errors:#?}"));
    assert!(n >= 1, "the catalogs have no keys");
}

#[test]
fn check_catalogs_reports_missing_key() {
    let dir = catalogs("a: x\nb: y\n", "a: x\n");
    let errors = check_catalogs(dir.path()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`b`") && e.contains("de.yml")),
        "{errors:?}"
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn check_catalogs_reports_key_missing_in_english() {
    let dir = catalogs("a: x\n", "a: x\nb: y\n");
    let errors = check_catalogs(dir.path()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`b`") && e.contains("en.yml")),
        "{errors:?}"
    );
}

#[test]
fn check_catalogs_reports_variable_mismatch() {
    let dir = catalogs("a: \"%{n} files\"\n", "a: \"Dateien\"\n");
    let errors = check_catalogs(dir.path()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`a`") && e.contains("%{n}")),
        "{errors:?}"
    );
}

#[test]
fn check_catalogs_reports_empty_and_non_string_values() {
    let dir = catalogs("a: \"\"\nb: 3\n", "a: x\nb: y\n");
    let errors = check_catalogs(dir.path()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`a`") && e.contains("en.yml")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`b`") && e.contains("en.yml")),
        "{errors:?}"
    );
}

#[test]
fn check_catalogs_flattens_nested_maps_and_skips_the_version() {
    let en = "_version: 1\nfiles:\n  open:\n    title: \"Open %{name}\"\nplain: p\n";
    let de = "files:\n  open:\n    title: \"%{name} (de)\"\nfiles.save: s\nplain: q\n";
    // `files.save` exists in de only: nested keys compare by their dotted path.
    let dir = catalogs(en, de);
    let errors = check_catalogs(dir.path()).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`files.save`"), "{errors:?}");
    let dir = catalogs(
        en,
        "files:\n  open:\n    title: \"%{name} (de)\"\nplain: q\n",
    );
    assert_eq!(check_catalogs(dir.path()), Ok(2));
}

#[test]
fn set_locale_normalizes() {
    let _turn = locale_lock();
    set_locale("de-AT");
    assert_eq!(locale(), "de");
    set_locale("DE_ch");
    assert_eq!(locale(), "de");
    set_locale("xx");
    assert_eq!(locale(), "en");
    set_locale("en");
}

#[test]
fn probe_translates() {
    let _turn = locale_lock();
    set_locale("de");
    assert_eq!(t!("i18n.probe"), "Probe");
    assert_eq!(g4_kernel::_rust_i18n_translate("de", "i18n.probe"), "Probe");
    set_locale("en");
    assert_eq!(t!("i18n.probe"), "probe");
    assert_eq!(g4_kernel::_rust_i18n_translate("en", "i18n.probe"), "probe");
}

/// A real key of the world module with two variables (`world.cell_outside`, the refusal of a
/// cell off the map): both values land in the sentence, in German and in English.
#[test]
fn world_message_fills_variables() {
    let _turn = locale_lock();
    set_locale("de");
    let de = t!("world.cell_outside", x = 3, z = 7, max = 31);
    assert_eq!(de, "Zelle (3, 7) liegt außerhalb der Karte (0 bis 31).");
    assert!(de.contains('3') && de.contains('7'), "{de}");
    set_locale("en");
    let en = t!("world.cell_outside", x = 3, z = 7, max = 31);
    assert_eq!(en, "Cell (3, 7) is outside the map (0 to 31).");
    assert!(en.contains('3') && en.contains('7'), "{en}");
}

/// A real key of the git module with two variables (`git.default_subject`, the subject of
/// „Save a snapshot“ without a message): both values land in the sentence, in German and in
/// English.
#[test]
fn git_message_fills_variables() {
    let _turn = locale_lock();
    set_locale("de");
    let de = t!("git.default_subject", date = "2026-11-10", time = "12:34");
    assert_eq!(de, "Stand 2026-11-10 12:34 UTC");
    assert!(de.contains("2026-11-10") && de.contains("12:34"), "{de}");
    set_locale("en");
    let en = t!("git.default_subject", date = "2026-11-10", time = "12:34");
    assert_eq!(en, "Snapshot 2026-11-10 12:34 UTC");
    assert!(en.contains("2026-11-10") && en.contains("12:34"), "{en}");
}

/// The five sentences the frontend mirrors (its English catalogs hold them byte for byte:
/// `git:gitWords.notInstalled`, `publish:publishWords.kernel.*`): a change here must show there.
#[test]
fn mirror_sentences_match_the_frontend_in_english() {
    let _turn = locale_lock();
    set_locale("en");
    assert_eq!(
        t!("git.not_installed"),
        "Git is not installed. The templates are written; you set up the repository as soon as \
         Git is there (git-scm.com/downloads)."
    );
    assert_eq!(
        t!("patch.bad_version"),
        "The version must not be empty, must be at most 32 characters long and must contain \
         neither spaces nor / \\ : * ? \" < > |."
    );
    assert_eq!(t!("patch.version_taken"), "This version already exists.");
    assert_eq!(
        t!("patch.bad_file"),
        "The file name must not be empty and must contain neither / \\ : * ? \" < > | nor \
         control characters."
    );
    assert_eq!(
        t!("patch.no_rom_content"),
        "No ROM content, only the patch: anyone who applies it needs the same base ROM."
    );
}
