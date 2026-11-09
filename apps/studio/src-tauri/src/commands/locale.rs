//! The language of the messages: the webview tells the backend which one the user chose.
//! The setting is process-wide (`g4_kernel::i18n`), so it also reaches the kernel, the MCP
//! answers and the dev bridge that run in the same process.

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
