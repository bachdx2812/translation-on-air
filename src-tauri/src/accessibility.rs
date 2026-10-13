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
///
/// A grant is bound to the code signature of the build that received it, and
/// ad-hoc signatures change with every build. After an update System Settings
/// still shows the app as allowed while this build is not, and macOS skips its
/// prompt because an entry exists. Clearing our own entry first brings the
/// prompt back, and the next grant matches this build.
#[tauri::command]
pub async fn request_accessibility(app: tauri::AppHandle) -> bool {
    if is_trusted(false) {
        return true;
    }
    reset_own_entry(&app.config().identifier).await;
    is_trusted(true)
}

#[cfg(target_os = "macos")]
async fn reset_own_entry(bundle_id: &str) {
    let status = tokio::process::Command::new("/usr/bin/tccutil")
        .args(["reset", "Accessibility", bundle_id])
        .status()
        .await;
    // Best effort: on failure the user can still remove the stale entry by
    // hand (the popup explains how).
    if !matches!(status, Ok(s) if s.success()) {
        eprintln!("[accessibility] tccutil reset failed: {status:?}");
    }
}

#[cfg(not(target_os = "macos"))]
async fn reset_own_entry(_bundle_id: &str) {}

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
