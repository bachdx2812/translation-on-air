use tauri::{AppHandle, LogicalSize, Manager};

/// Show + focus the settings window. Shared by the tray menu and the
/// `show_settings` command so both paths behave identically (DRY).
pub fn show_settings_inner(app: &AppHandle) -> tauri::Result<()> {
    eprintln!("[show_settings] invoked");

    // Accessory (no-dock) apps can't become the active app, so the settings window
    // would open behind the frontmost app and never take focus. Briefly switch to
    // Regular so the app can activate and the window comes to front; reverted to
    // Accessory when settings closes (lib.rs CloseRequested). Dock shows only while
    // settings is open.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    // Hide the popup so the settings window isn't stacked underneath it (both center).
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }

    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.unminimize();
        win.show()?;
        win.set_focus()?;
    }
    Ok(())
}

/// Show + focus the popup. The hotkey pipeline (phase 03) calls this only AFTER
/// capturing the selection, so the synthetic Cmd+C targets the frontmost app.
pub fn show_popup_inner(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("popup") {
        win.show()?;
        win.set_focus()?;
    }
    Ok(())
}

#[tauri::command]
pub fn show_settings(app: AppHandle) -> Result<(), String> {
    show_settings_inner(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn show_popup(app: AppHandle) -> Result<(), String> {
    show_popup_inner(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_popup(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("popup")
        .ok_or("no popup window")?
        .hide()
        .map_err(|e| e.to_string())
}

/// Resize the popup to fit its content (the frontend measures and clamps), then
/// re-center. Keeps the window auto-sizing to the translation instead of scrolling.
#[tauri::command]
pub fn resize_popup(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("popup") {
        win.set_size(LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        let _ = win.center();
    }
    Ok(())
}
