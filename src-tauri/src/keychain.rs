//! OpenAI API key storage in the macOS Keychain via the `keyring` crate.
//! The key is read only inside Rust at call time — it never reaches the frontend.

use std::sync::Mutex;

const SERVICE: &str = "translate-on-air";
const ACCOUNT: &str = "openai_api_key";

/// In-process cache of the key (outer Option: loaded yet; inner: key present).
/// Every uncached Keychain read can trigger a system password prompt when the
/// app's ad-hoc signature no longer matches the item's ACL (each build/update),
/// and settings + provider detection + every translation all read the key. The
/// Mutex also serializes concurrent first reads at startup, so the user sees at
/// most ONE prompt per launch instead of one per call site.
static KEY_CACHE: Mutex<Option<Option<String>>> = Mutex::new(None);

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
}

pub fn set_key(key: &str) -> Result<(), String> {
    let mut cache = KEY_CACHE.lock().unwrap();
    entry()?.set_password(key).map_err(|e| e.to_string())?;
    *cache = Some(Some(key.to_string()));
    Ok(())
}

pub fn get_key() -> Option<String> {
    let mut cache = KEY_CACHE.lock().unwrap();
    if let Some(loaded) = cache.as_ref() {
        return loaded.clone();
    }
    let key = entry().ok().and_then(|e| e.get_password().ok());
    *cache = Some(key.clone());
    key
}

pub fn delete_key() -> Result<(), String> {
    let mut cache = KEY_CACHE.lock().unwrap();
    entry()?.delete_credential().map_err(|e| e.to_string())?;
    *cache = Some(None);
    Ok(())
}

pub fn has_key() -> bool {
    get_key().is_some()
}
