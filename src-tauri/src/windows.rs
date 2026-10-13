use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalRect, PhysicalSize};

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

/// Resize the popup to fit its content (the frontend measures and clamps).
/// The window grows/shrinks around its CURRENT center rather than the screen's,
/// so a popup the user dragged aside stays there, while an undragged one stays
/// screen-centered (the window config starts it centered).
#[tauri::command]
pub fn resize_popup(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let Some(win) = app.get_webview_window("popup") else {
        return Ok(());
    };
    let pos = win.outer_position().map_err(|e| e.to_string())?;
    let old_size = win.outer_size().map_err(|e| e.to_string())?;
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let size = LogicalSize::new(width, height);

    win.set_size(size).map_err(|e| e.to_string())?;
    let work_area = win.current_monitor().ok().flatten().map(|m| *m.work_area());
    let origin = recentered_origin(pos, old_size, size.to_physical(scale), work_area);
    win.set_position(origin).map_err(|e| e.to_string())
}

/// Top-left for a window resized from `old` to `new` that keeps its center,
/// clamped into `work_area` so a growing popup never spills off-screen.
fn recentered_origin(
    pos: PhysicalPosition<i32>,
    old: PhysicalSize<u32>,
    new: PhysicalSize<u32>,
    work_area: Option<PhysicalRect<i32, u32>>,
) -> PhysicalPosition<i32> {
    let mut x = pos.x + (old.width as i32 - new.width as i32) / 2;
    let mut y = pos.y + (old.height as i32 - new.height as i32) / 2;
    if let Some(wa) = work_area {
        // min-then-max (not clamp): a window taller than the work area pins to
        // its top instead of panicking on min > max.
        x = x
            .min(wa.position.x + wa.size.width as i32 - new.width as i32)
            .max(wa.position.x);
        y = y
            .min(wa.position.y + wa.size.height as i32 - new.height as i32)
            .max(wa.position.y);
    }
    PhysicalPosition::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::recentered_origin;
    use tauri::{PhysicalPosition, PhysicalRect, PhysicalSize};

    const WORK: PhysicalRect<i32, u32> = PhysicalRect {
        position: PhysicalPosition { x: 0, y: 50 },
        size: PhysicalSize {
            width: 2000,
            height: 1000,
        },
    };

    #[test]
    fn grows_around_current_center() {
        let o = recentered_origin(
            PhysicalPosition::new(100, 300),
            PhysicalSize::new(920, 360),
            PhysicalSize::new(920, 600),
            Some(WORK),
        );
        assert_eq!(o, PhysicalPosition::new(100, 180));
    }

    #[test]
    fn clamps_into_work_area() {
        let o = recentered_origin(
            PhysicalPosition::new(1800, 900),
            PhysicalSize::new(920, 100),
            PhysicalSize::new(920, 400),
            Some(WORK),
        );
        assert_eq!(o, PhysicalPosition::new(1080, 650));
    }

    #[test]
    fn pins_oversized_window_to_work_area_top_left() {
        let o = recentered_origin(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(920, 100),
            PhysicalSize::new(3000, 1500),
            Some(WORK),
        );
        assert_eq!(o, PhysicalPosition::new(0, 50));
    }
}
