//! Local translation history, grouped as conversations. Persisted via
//! tauri-plugin-store (`history.json`); never leaves the machine. The frontend
//! owns the conversation shape (id, turns, …) and timestamps — Rust just stores
//! the array, upserting by id and capping the total.

use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "history.json";
const KEY: &str = "conversations";
/// Keep history bounded so the store file can't grow without limit.
const MAX_CONVERSATIONS: usize = 200;

fn read_all(app: &AppHandle) -> Vec<Value> {
    app.store(STORE_FILE)
        .ok()
        .and_then(|s| s.get(KEY))
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
}

/// All conversations, newest first.
#[tauri::command]
pub fn history_get(app: AppHandle) -> Vec<Value> {
    read_all(&app)
}

/// Insert or update a conversation (matched by its `id`). Updated conversations
/// move to the front so the list stays newest-first.
#[tauri::command]
pub fn history_save(app: AppHandle, conversation: Value) -> Result<(), String> {
    let Some(id) = conversation.get("id").and_then(|v| v.as_str()) else {
        return Err("conversation missing id".into());
    };
    let id = id.to_string();

    let mut all = read_all(&app);
    all.retain(|c| c.get("id").and_then(|v| v.as_str()) != Some(id.as_str()));
    all.insert(0, conversation);
    all.truncate(MAX_CONVERSATIONS);

    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(KEY, json!(all));
    store.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_clear(app: AppHandle) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(KEY, json!([]));
    store.save().map_err(|e| e.to_string())
}
