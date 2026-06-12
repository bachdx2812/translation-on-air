//! macOS Accessibility permission. Posting synthetic key events (the Cmd+C used
//! to capture a selection) requires the app to be trusted for Accessibility.

#[cfg(target_os = "macos")]
pub fn is_trusted(prompt: bool) -> bool {
    use macos_accessibility_client::accessibility::{
        application_is_trusted, application_is_trusted_with_prompt,
    };
    if prompt {
        application_is_trusted_with_prompt()
    } else {
        application_is_trusted()
    }
}

#[cfg(not(target_os = "macos"))]
pub fn is_trusted(_prompt: bool) -> bool {
    true
}

/// Whether the app currently holds Accessibility permission (no system prompt).
#[tauri::command]
pub fn check_accessibility() -> bool {
    is_trusted(false)
}

/// Trigger the system Accessibility prompt (registers the app in the
/// Privacy & Security list). Only called from an explicit user click so the
/// system dialog never stacks on top of the in-app guidance.
#[tauri::command]
pub fn request_accessibility() -> bool {
    is_trusted(true)
}

/// Open System Settings → Privacy & Security → Accessibility so the user can
/// grant permission.
#[tauri::command]
pub fn open_accessibility_settings(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            None::<&str>,
        )
        .map_err(|e| e.to_string())
}
