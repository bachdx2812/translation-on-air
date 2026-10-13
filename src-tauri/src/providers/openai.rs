//! OpenAI provider. Calls chat completions with a strict json_schema response
//! format so the model output always matches the furigana schema, streaming the
//! response so the translation can be shown while the rest is still generating.

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
///
/// `on_partial` receives the translation decoded so far each time it grows.
pub async fn translate(
    key: &str,
    model: &str,
    effort: &str,
    sys: &str,
    text: &str,
    on_partial: &(dyn Fn(&str) + Send + Sync),
) -> Result<Translated, ProviderError> {
    let schema: serde_json::Value =
        serde_json::from_str(prompt::FURIGANA_SCHEMA).map_err(|_| ProviderError::BadResponse)?;

    let body = json!({
        "model": model,
        "stream": true,
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

    let mut resp = if send_effort {
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

    let mut lines = SseLines::default();
    let mut content = String::new();
    let mut translation_done = false;
    let mut sent_len = 0;
    while let Some(chunk) = resp.chunk().await.map_err(transport_error)? {
        for data in lines.push(&chunk) {
            if data == "[DONE]" {
                continue;
            }
            let event: serde_json::Value =
                serde_json::from_str(&data).map_err(|_| ProviderError::BadResponse)?;
            let Some(delta) = event["choices"][0]["delta"]["content"].as_str() else {
                continue;
            };
            content.push_str(delta);
            // Once the translation string closes, only furigana arrays follow.
            if !translation_done {
                if let Some((partial, done)) = partial_translation(&content) {
                    translation_done = done;
                    // Skip chunks that added nothing visible (e.g. half an escape).
                    if partial.len() > sent_len {
                        sent_len = partial.len();
                        on_partial(&partial);
                    }
                }
            }
        }
    }
    serde_json::from_str(&content).map_err(|_| ProviderError::BadResponse)
}

async fn send(key: &str, body: &serde_json::Value) -> Result<reqwest::Response, ProviderError> {
    client()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(body)
        .send()
        .await
        .map_err(transport_error)
}

fn transport_error(e: reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Unavailable
    }
}

/// Splits a server-sent-events byte stream into `data:` payloads. Bytes are
/// buffered until a full line arrives, so a UTF-8 character split across
/// network chunks is never decoded in halves.
#[derive(Default)]
struct SseLines {
    buf: Vec<u8>,
}

impl SseLines {
    fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        while let Some(nl) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=nl).collect();
            let line = String::from_utf8_lossy(&line);
            if let Some(data) = line.trim_end().strip_prefix("data:") {
                out.push(data.trim_start().to_string());
            }
        }
        out
    }
}

/// The `translation` value of a possibly-truncated JSON object, plus whether
/// its closing quote has arrived. None before the value starts, or while its
/// tail is an escape sequence that has not fully arrived yet.
fn partial_translation(json: &str) -> Option<(String, bool)> {
    let rest = json.split_once("\"translation\"")?.1.trim_start();
    let value = rest.strip_prefix(':')?.trim_start().strip_prefix('"')?;

    let mut escaped = false;
    let mut last_escape = None;
    for (i, c) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => {
                escaped = true;
                last_escape = Some(i);
            }
            '"' => return decode_json_string(&value[..i]).map(|s| (s, true)),
            _ => {}
        }
    }
    let end = match last_escape {
        Some(i)
            if &value[i..] == "\\" || (value[i..].starts_with("\\u") && value.len() - i < 6) =>
        {
            i
        }
        _ => value.len(),
    };
    decode_json_string(&value[..end]).map(|s| (s, false))
}

fn decode_json_string(raw: &str) -> Option<String> {
    serde_json::from_str(&format!("\"{raw}\"")).ok()
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
    use super::{partial_translation, rejects_reasoning_effort, SseLines};

    #[test]
    fn sse_lines_buffer_split_utf8_and_partial_lines() {
        let mut lines = SseLines::default();
        let event = "data: {\"a\":\"問\"}\n\n".as_bytes();
        // Split inside the 3-byte UTF-8 sequence of 問.
        let cut = event.iter().position(|&b| b >= 0x80).unwrap() + 1;
        assert!(lines.push(&event[..cut]).is_empty());
        assert_eq!(lines.push(&event[cut..]), vec![r#"{"a":"問"}"#.to_string()]);
        assert_eq!(
            lines.push(b": keep-alive\ndata: [DONE]\n"),
            vec!["[DONE]".to_string()]
        );
    }

    #[test]
    fn partial_translation_waits_for_the_value() {
        assert_eq!(partial_translation(""), None);
        assert_eq!(partial_translation(r#"{"transl"#), None);
        assert_eq!(partial_translation(r#"{"translation":"#), None);
    }

    #[test]
    fn partial_translation_decodes_growing_and_closed_values() {
        assert_eq!(
            partial_translation(r#"{"translation":"Vâng, chúng"#),
            Some(("Vâng, chúng".to_string(), false))
        );
        assert_eq!(
            partial_translation(r#"{"translation":"Dòng 1\nDòng \"2\"","segments":[{"#),
            Some(("Dòng 1\nDòng \"2\"".to_string(), true))
        );
    }

    #[test]
    fn partial_translation_holds_back_half_received_escapes() {
        assert_eq!(
            partial_translation(r#"{"translation":"a\"#),
            Some(("a".to_string(), false))
        );
        assert_eq!(
            partial_translation(r#"{"translation":"a\u00"#),
            Some(("a".to_string(), false))
        );
        assert_eq!(
            partial_translation(r#"{"translation":"a\\"#),
            Some(("a\\".to_string(), false))
        );
        assert_eq!(
            partial_translation(r#"{"translation":"a\u00e9"#),
            Some(("aé".to_string(), false))
        );
    }

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
