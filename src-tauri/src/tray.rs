use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

const TRAY_ID: &str = "main-tray";

/// Version (disabled), an update item while a newer release exists, then
/// Settings and Quit. `Quit` exits the whole process: the app has no dock icon,
/// so the tray is the only way to quit normally.
fn build_menu(app: &AppHandle, update: Option<&str>) -> tauri::Result<Menu<tauri::Wry>> {
    let item = |id: &str, text: String, enabled: bool| {
        MenuItem::with_id(app, id, text, enabled, None::<&str>)
    };
    let menu = Menu::new(app)?;
    let version = format!("Phiên bản {}", app.package_info().version);
    menu.append(&item("version", version, false)?)?;
    if let Some(latest) = update {
        menu.append(&item("update", format!("Có bản mới {latest}…"), true)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&item("settings", "Cài đặt…".into(), true)?)?;
    menu.append(&item("quit", "Thoát".into(), true)?)?;
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, None)?;

    // Reuse the bundled app icon for now. icon_as_template renders it monochrome
    // so it adapts to light/dark menubars; a dedicated template asset comes later.
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("default window icon configured in tauri.conf.json");

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            // Settings shows the update details and how to install it.
            "settings" | "update" => {
                let _ = crate::windows::show_settings_inner(app);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Show (`Some(version)`) or hide (`None`) the "new version" item.
pub fn set_update(app: &AppHandle, update: Option<&str>) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(build_menu(app, update)?))?;
    }
    Ok(())
}
