//! App settings, persisted via tauri-plugin-store (non-secrets only). The OpenAI
//! key lives in the Keychain (keychain.rs) and is NEVER stored here or returned
//! to the frontend (only the `has_openai_key` boolean is).

use crate::providers::ProviderCache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "settings.json";

const K_HOTKEY: &str = "hotkey";
const K_TARGET_LANG: &str = "target_lang";
const K_PROVIDER_MODE: &str = "provider_mode";
const K_MODEL: &str = "openai_model";

pub const DEFAULT_HOTKEY: &str = "Cmd+Shift+T";
const DEFAULT_TARGET_LANG: &str = "vi";
const DEFAULT_PROVIDER_MODE: &str = "auto";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

fn read_string(app: &AppHandle, key: &str, default: &str) -> String {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get(key))
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| default.to_string())
}

pub fn hotkey(app: &AppHandle) -> String {
    read_string(app, K_HOTKEY, DEFAULT_HOTKEY)
}
pub fn target_lang(app: &AppHandle) -> String {
    read_string(app, K_TARGET_LANG, DEFAULT_TARGET_LANG)
}
pub fn provider_mode(app: &AppHandle) -> String {
    read_string(app, K_PROVIDER_MODE, DEFAULT_PROVIDER_MODE)
}
pub fn model(app: &AppHandle) -> String {
    read_string(app, K_MODEL, DEFAULT_MODEL)
}

/// Full settings for the settings window. `has_openai_key` is derived from the
/// Keychain; the key value itself is never included.
#[derive(Serialize)]
pub struct Settings {
    hotkey: String,
    target_lang: String,
    provider_mode: String,
    openai_model: String,
    has_openai_key: bool,
}

/// Partial update from the settings UI (hotkey is handled by `set_hotkey`).
#[derive(Deserialize)]
pub struct SettingsPatch {
    target_lang: Option<String>,
    provider_mode: Option<String>,
    openai_model: Option<String>,
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    Settings {
        hotkey: hotkey(&app),
        target_lang: target_lang(&app),
        provider_mode: provider_mode(&app),
        openai_model: model(&app),
        has_openai_key: crate::keychain::has_key(),
    }
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    cache: State<'_, ProviderCache>,
    patch: SettingsPatch,
) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let mut provider_affecting = false;
    if let Some(v) = patch.target_lang {
        store.set(K_TARGET_LANG, json!(v));
    }
    if let Some(v) = patch.provider_mode {
        store.set(K_PROVIDER_MODE, json!(v));
        provider_affecting = true;
    }
    if let Some(v) = patch.openai_model {
        store.set(K_MODEL, json!(v));
        provider_affecting = true;
    }
    store.save().map_err(|e| e.to_string())?;
    if provider_affecting {
        cache.invalidate();
    }
    Ok(())
}

/// Validate + apply a new hotkey: parse server-side, rebind live (rollback on
/// failure keeps the old binding working), then persist.
#[tauri::command]
pub fn set_hotkey(app: AppHandle, accel: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::Shortcut;
    accel
        .parse::<Shortcut>()
        .map_err(|_| "invalid-accelerator".to_string())?;
    let old = hotkey(&app);
    crate::hotkey::rebind(&app, &old, &accel).map_err(|_| "register-failed".to_string())?;
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(K_HOTKEY, json!(accel));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}
