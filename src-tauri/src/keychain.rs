//! OpenAI API key storage in the macOS Keychain via the `keyring` crate.
//! The key is read only inside Rust at call time — it never reaches the frontend.

const SERVICE: &str = "translate-on-air";
const ACCOUNT: &str = "openai_api_key";

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
}

pub fn set_key(key: &str) -> Result<(), String> {
    entry()?.set_password(key).map_err(|e| e.to_string())
}

pub fn get_key() -> Option<String> {
    entry().ok()?.get_password().ok()
}

pub fn delete_key() -> Result<(), String> {
    entry()?.delete_credential().map_err(|e| e.to_string())
}

pub fn has_key() -> bool {
    get_key().is_some()
}
