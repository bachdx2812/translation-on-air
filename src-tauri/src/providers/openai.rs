//! OpenAI provider. Calls chat completions with a strict json_schema response
//! format so the model output always matches the furigana schema.

use super::prompt;
use super::types::{ProviderError, Translated};
use serde_json::json;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Settings value meaning "omit `reasoning_effort`, let the model decide".
pub const MODEL_DEFAULT_EFFORT: &str = "default";

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

/// `(model, effort)` pairs OpenAI rejected this session, so later requests go
/// straight to the model default instead of paying for a failing round trip.
fn rejected_efforts() -> &'static Mutex<HashSet<(String, String)>> {
    static REJECTED: OnceLock<Mutex<HashSet<(String, String)>>> = OnceLock::new();
    REJECTED.get_or_init(Default::default)
}

/// Translate `text` using a caller-built system prompt (so both plain
/// translation and contextual replies share one request path).
///
/// `effort` is sent as `reasoning_effort`. Non-reasoning models (gpt-4o-mini)
/// reject the parameter and some models reject specific levels, so a 400 about
/// it falls back to one retry without the parameter.
pub async fn translate(
    key: &str,
    model: &str,
    effort: &str,
    sys: &str,
    text: &str,
) -> Result<Translated, ProviderError> {
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

    let pair = (model.to_string(), effort.to_string());
    let send_effort =
        effort != MODEL_DEFAULT_EFFORT && !rejected_efforts().lock().unwrap().contains(&pair);

    let resp = if send_effort {
        let mut with_effort = body.clone();
        with_effort["reasoning_effort"] = json!(effort);
        let resp = send(key, &with_effort).await?;
        if resp.status().as_u16() == 400 {
            let err = resp.text().await.unwrap_or_default();
            if !rejects_reasoning_effort(&err) {
                return Err(ProviderError::BadResponse);
            }
            rejected_efforts().lock().unwrap().insert(pair);
            send(key, &body).await?
        } else {
            resp
        }
    } else {
        send(key, &body).await?
    };

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

async fn send(key: &str, body: &serde_json::Value) -> Result<reqwest::Response, ProviderError> {
    client()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                ProviderError::Timeout
            } else {
                ProviderError::Unavailable
            }
        })
}

/// True when a 400 body blames `reasoning_effort`: either the whole parameter
/// (`param` is null, named only in the message) or the chosen level.
fn rejects_reasoning_effort(body: &str) -> bool {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let err = &v["error"];
    err["param"] == "reasoning_effort"
        || err["message"]
            .as_str()
            .is_some_and(|m| m.contains("reasoning_effort"))
}

#[cfg(test)]
mod tests {
    use super::rejects_reasoning_effort;

    #[test]
    fn detects_unrecognized_parameter() {
        let body = r#"{"error":{"message":"Unrecognized request argument supplied: reasoning_effort","type":"invalid_request_error","param":null,"code":null}}"#;
        assert!(rejects_reasoning_effort(body));
    }

    #[test]
    fn detects_unsupported_level() {
        let body = r#"{"error":{"message":"Unsupported value: 'reasoning_effort' does not support 'none' with this model.","type":"invalid_request_error","param":"reasoning_effort","code":"unsupported_value"}}"#;
        assert!(rejects_reasoning_effort(body));
    }

    #[test]
    fn ignores_other_bad_requests() {
        let body = r#"{"error":{"message":"Invalid schema for response_format","type":"invalid_request_error","param":"response_format","code":null}}"#;
        assert!(!rejects_reasoning_effort(body));
        assert!(!rejects_reasoning_effort("not json"));
    }
}
