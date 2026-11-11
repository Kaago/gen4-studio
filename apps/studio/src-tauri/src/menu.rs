//! The app menu: Tauri's default menu, except that "Quit" is our own item, and "Check for
//! Updates…" is added.
//!
//! The predefined Quit item sends `terminate:` straight to NSApplication, which ends the app
//! without a `RunEvent::ExitRequested`, so the unsaved-edits check never ran on Cmd+Q. Our item
//! carries the same accelerator and goes through `request_quit` in lib.rs instead. "Check for
//! Updates…" sends `menu-check-updates` to the webview, which runs the check (lib.rs). Its text
//! is in the language of the process when the menu is built: at start that is `G4_LANG` or
//! `LANG`, before the webview chooses one, and each time the webview sets the language
//! (`set_locale`) the menu is built anew ([`follow_locale`]), so the item follows the app's
//! language (the other items are English).

use tauri::menu::{
    AboutMetadata, HELP_SUBMENU_ID, Menu, MenuItem, PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID,
};
use tauri::{AppHandle, Runtime};

/// Menu id of our Quit item.
pub const QUIT_ID: &str = "g4-quit";

/// Menu id of "Check for Updates…".
pub const CHECK_UPDATES_ID: &str = "g4-check-updates";

/// Same layout as `Menu::default`, with Quit replaced (see the module docs), in the language
/// of the process.
pub fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    menu_in(app, &g4_kernel::i18n::locale())
}

/// Builds the app menu anew in `lang` and makes it the app's menu (the old one is dropped). The
/// menu ids stay, so the handler in lib.rs (`on_menu_event`) keeps working.
pub fn follow_locale<R: Runtime>(app: &AppHandle<R>, lang: &str) -> tauri::Result<()> {
    app.set_menu(menu_in(app, lang)?)?;
    Ok(())
}

/// The text of "Check for Updates…" in `lang` (`en` or `de`).
fn check_updates_label(lang: &str) -> String {
    t!("menu.check_updates", locale = lang).into_owned()
}

/// The app menu with the texts that are ours in `lang`.
fn menu_in<R: Runtime>(app: &AppHandle<R>, lang: &str) -> tauri::Result<Menu<R>> {
    let pkg_info = app.package_info();
    let config = app.config();
    let about_metadata = AboutMetadata {
        name: Some(pkg_info.name.clone()),
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };
    let quit = MenuItem::with_id(
        app,
        QUIT_ID,
        format!("Quit {}", pkg_info.name),
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    let check_updates = MenuItem::with_id(
        app,
        CHECK_UPDATES_ID,
        check_updates_label(lang),
        true,
        None::<&str>,
    )?;

    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let help_menu = Submenu::with_id_and_items(
        app,
        HELP_SUBMENU_ID,
        "Help",
        true,
        &[
            #[cfg(not(target_os = "macos"))]
            &PredefinedMenuItem::about(app, None, Some(about_metadata.clone()))?,
            #[cfg(not(target_os = "macos"))]
            &check_updates,
        ],
    )?;

    Menu::with_items(
        app,
        &[
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                pkg_info.name.clone(),
                true,
                &[
                    &PredefinedMenuItem::about(app, None, Some(about_metadata))?,
                    &check_updates,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?,
            &Submenu::with_items(
                app,
                "File",
                true,
                &[
                    &PredefinedMenuItem::close_window(app, None)?,
                    #[cfg(not(target_os = "macos"))]
                    &quit,
                ],
            )?,
            &Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?,
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                "View",
                true,
                &[&PredefinedMenuItem::fullscreen(app, None)?],
            )?,
            &window_menu,
            &help_menu,
        ],
    )
}

// No test builds a menu: muda creates its items on the main thread only, which the test harness
// does not run on. The text is the part with a decision in it.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_for_updates_is_in_the_language_asked_for_not_the_one_of_the_process() {
        assert_eq!(check_updates_label("en"), "Check for Updates…");
        assert_eq!(check_updates_label("de"), "Nach Updates suchen…");
    }
}
