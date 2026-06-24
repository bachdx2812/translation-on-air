use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

/// Build the menubar tray icon with a minimal Settings + Quit menu.
///
/// `Settings` is a stub here; phase 02 wires it to show the settings window.
/// `Quit` exits the whole process (the app has no dock icon, so the tray is the
/// only way to quit normally).
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

    // Reuse the bundled app icon for now. icon_as_template renders it monochrome
    // so it adapts to light/dark menubars; a dedicated template asset comes later.
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("default window icon configured in tauri.conf.json");

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "settings" => {
                let _ = crate::windows::show_settings_inner(app);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
