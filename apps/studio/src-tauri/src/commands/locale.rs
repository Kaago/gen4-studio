//! The language of the messages: the webview tells the backend which one the user chose.
//! The setting is process-wide (`g4_kernel::i18n`), so it also reaches the kernel and the MCP
//! answers that run in the same process. The dev bridge (`studio-bridge`) is a process of its
//! own: it reads `G4_LANG` at its start (`init_from_env`) and, in the browser mode, gets
//! `set_locale` from the page like the app does. For the choice "System" the webview asks the
//! backend for the language of the operating system.

/// Sets the language of this process's messages: `de` (also `de-AT`, `de_CH`) selects German,
/// anything else English.
pub fn set_locale_impl(lang: &str) {
    g4_kernel::i18n::set_locale(lang);
}

/// The webview calls this at start and whenever the user changes the language.
#[tauri::command]
pub fn set_locale(lang: String) {
    set_locale_impl(&lang);
}

/// The language of the operating system as it reports it (`de-DE`, `en-US`, on Linux the
/// locale of the environment), `None` when it reports none.
pub fn system_language_impl() -> Option<String> {
    reported(sys_locale::get_locale())
}

/// The locale as the OS reported it, without surrounding white space; nothing or an empty
/// text is no answer.
fn reported(locale: Option<String>) -> Option<String> {
    locale
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
}

/// What "System" in Settings › Language follows inside the app. WKWebView's
/// `navigator.language` may report the bundle's localizations rather than the OS language, so
/// the webview asks here (`null`: it keeps `navigator.language`).
#[tauri::command]
pub fn system_language() -> Option<String> {
    system_language_impl()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reported_locale_is_trimmed_and_nothing_is_no_answer() {
        assert_eq!(reported(Some("de-DE".into())), Some("de-DE".into()));
        assert_eq!(
            reported(Some(" de_AT.UTF-8\n".into())),
            Some("de_AT.UTF-8".into())
        );
        assert_eq!(reported(Some("  ".into())), None);
        assert_eq!(reported(Some(String::new())), None);
        assert_eq!(reported(None), None);
    }

    #[test]
    fn the_system_language_is_a_locale_or_nothing() {
        // Whatever this machine reports: never an empty text.
        if let Some(locale) = system_language_impl() {
            assert!(!locale.trim().is_empty());
            assert_eq!(locale, locale.trim());
        }
    }
}
