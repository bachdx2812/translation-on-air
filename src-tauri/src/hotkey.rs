//! Global hotkey registration + dynamic rebind. Default Cmd+Shift+T.
//!
//! The plugin's press handler (a single dispatcher) is set up in `lib.rs`; this
//! module owns the accelerator value and the register/rebind logic.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// Register the persisted accelerator at startup, falling back to the default if
/// the stored value is missing or unparseable (never crash on a corrupt store).
pub fn register_from_settings(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let accel = crate::settings::hotkey(app);
    let shortcut: Shortcut = accel
        .parse()
        .or_else(|_| crate::settings::DEFAULT_HOTKEY.parse())?;
    app.global_shortcut().register(shortcut)?;
    Ok(())
}

/// Swap the active accelerator at runtime. If registering the new accelerator
/// fails, the old one is restored (rollback) so the hotkey never silently dies.
/// Reused by the settings window in phase 06.
#[allow(dead_code)] // wired into the settings UI in phase 06
pub fn rebind(app: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    let old_s: Shortcut = old.parse().map_err(|_| format!("bad accelerator: {old}"))?;
    let new_s: Shortcut = new.parse().map_err(|_| format!("bad accelerator: {new}"))?;
    let gs = app.global_shortcut();

    gs.unregister(old_s).map_err(|e| e.to_string())?;
    if let Err(e) = gs.register(new_s) {
        let _ = gs.register(old_s); // rollback to keep a working hotkey
        return Err(e.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tauri_plugin_global_shortcut::Shortcut;

    #[test]
    fn valid_accelerators_parse() {
        assert!("Cmd+Shift+T".parse::<Shortcut>().is_ok());
        assert!(crate::settings::DEFAULT_HOTKEY.parse::<Shortcut>().is_ok());
    }

    #[test]
    fn empty_accelerator_is_rejected() {
        assert!("".parse::<Shortcut>().is_err());
    }
}
