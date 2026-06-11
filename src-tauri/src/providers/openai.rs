//! OpenAI provider. Calls chat completions with a strict json_schema response
//! format so the model output always matches the furigana schema.

use super::prompt;
use super::types::{ProviderError, Translated};
use serde_json::json;
use std::sync::OnceLock;
use std::time::Duration;

/// One reused client (connection pool + 30s timeout) for all requests.
fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client builds")
    })
}

pub async fn translate(
    key: &str,
    model: &str,
    text: &str,
    target_lang: &str,
) -> Result<Translated, ProviderError> {
    let sys = prompt::system_prompt(target_lang);
    let schema: serde_json::Value =
        serde_json::from_str(prompt::FURIGANA_SCHEMA).map_err(|_| ProviderError::BadResponse)?;

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": sys },
            { "role": "user", "content": text }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": { "name": "translation", "strict": true, "schema": schema }
        }
    });

    let resp = client()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                ProviderError::Timeout
            } else {
                ProviderError::Unavailable
            }
        })?;

    match resp.status().as_u16() {
        200 => {}
        401 => return Err(ProviderError::InvalidKey),
        429 => return Err(ProviderError::RateLimited),
        500..=599 => return Err(ProviderError::Unavailable),
        _ => return Err(ProviderError::BadResponse),
    }

    let v: serde_json::Value = resp.json().await.map_err(|_| ProviderError::BadResponse)?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or(ProviderError::BadResponse)?;
    serde_json::from_str(content).map_err(|_| ProviderError::BadResponse)
}
