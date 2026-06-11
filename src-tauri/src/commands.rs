//! Tauri commands: translation, provider detection, and OpenAI key storage.
//! The OpenAI key is read here inside Rust and never returned to the frontend.

use crate::providers::{self, types::Translated, ProviderMode};
use crate::{keychain, settings};
use tauri::State;

#[derive(serde::Serialize)]
pub struct ProviderStatus {
    claude_detected: bool,
    claude_path: Option<String>,
    claude_supports_json_schema: bool,
    has_openai_key: bool,
    resolved: String, // "openai" | "claude" | "none"
}

#[tauri::command]
pub async fn translate(
    app: tauri::AppHandle,
    cache: State<'_, providers::ProviderCache>,
    text: String,
    target_lang: String,
) -> Result<Translated, String> {
    let text = text.trim().to_string();
    let mode = ProviderMode::from_str(&settings::provider_mode(&app));
    let model = settings::model(&app);
    let claude = cache.claude_info();
    let has_key = keychain::has_key();

    let provider = providers::resolve(mode, claude, has_key).map_err(|e| e.code().to_string())?;

    let result = match provider {
        providers::Provider::OpenAi => {
            let key = keychain::get_key().ok_or_else(|| "not-configured".to_string())?;
            providers::openai::translate(&key, &model, &text, &target_lang).await
        }
        providers::Provider::ClaudeCli(info) => {
            providers::claude_cli::translate(&info, &text, &target_lang).await
        }
    }
    .map_err(|e| e.code().to_string())?;

    // Output furigana only when translating INTO Japanese.
    let segments = if target_lang == "ja" {
        providers::furigana::validate_segments(&result.translation, result.segments)
    } else {
        Vec::new()
    };
    // Source furigana whenever the source text is Japanese (e.g. reading JA→VI).
    let source_segments = if providers::furigana::contains_japanese(&text) {
        providers::furigana::validate_segments(&text, result.source_segments)
    } else {
        Vec::new()
    };

    Ok(Translated {
        translation: result.translation,
        segments,
        source_segments,
    })
}

#[tauri::command]
pub fn detect_providers(
    app: tauri::AppHandle,
    cache: State<'_, providers::ProviderCache>,
) -> ProviderStatus {
    let claude = cache.claude_info();
    let has_key = keychain::has_key();
    let mode = ProviderMode::from_str(&settings::provider_mode(&app));
    let resolved = match providers::resolve(mode, claude.clone(), has_key) {
        Ok(providers::Provider::OpenAi) => "openai",
        Ok(providers::Provider::ClaudeCli(_)) => "claude",
        Err(_) => "none",
    }
    .to_string();

    ProviderStatus {
        claude_detected: claude.is_some(),
        claude_path: claude.as_ref().map(|c| c.path.display().to_string()),
        claude_supports_json_schema: claude
            .as_ref()
            .map(|c| c.supports_json_schema)
            .unwrap_or(false),
        has_openai_key: has_key,
        resolved,
    }
}

#[tauri::command]
pub fn set_openai_key(key: String) -> Result<(), String> {
    keychain::set_key(&key)
}

#[tauri::command]
pub fn delete_openai_key() -> Result<(), String> {
    keychain::delete_key()
}

#[tauri::command]
pub fn has_openai_key() -> bool {
    keychain::has_key()
}

/// Copy text to the clipboard (popup Copy button). Keeps clipboard writes behind
/// a Rust command instead of granting the webview clipboard permission.
#[tauri::command]
pub fn copy_text(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}
