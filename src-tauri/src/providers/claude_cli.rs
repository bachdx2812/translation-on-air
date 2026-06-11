//! Local Claude CLI provider (subscription auth, no API key). Runs
//! `claude -p --output-format json` as a subprocess and parses the JSON envelope.

use super::prompt;
use super::types::{ProviderError, Translated};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ClaudeInfo {
    pub path: PathBuf,
    pub supports_json_schema: bool,
}

/// Detect a usable Claude CLI: a known binary path that exists AND subscription
/// credentials present. GUI apps inherit a minimal PATH, so probe absolute paths.
pub fn detect() -> Option<ClaudeInfo> {
    let home = std::env::var("HOME").ok();
    let mut candidates: Vec<PathBuf> = [
        "/opt/homebrew/bin/claude",
        "/usr/local/bin/claude",
        "/Applications/cmux.app/Contents/Resources/bin/claude",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    if let Some(h) = &home {
        candidates.push(PathBuf::from(format!("{h}/.local/bin/claude")));
    }

    let path = candidates.into_iter().find(|p| p.exists())?;
    if !creds_present(home.as_deref()) {
        return None;
    }
    let supports_json_schema = check_json_schema_support(&path);
    Some(ClaudeInfo {
        path,
        supports_json_schema,
    })
}

/// Subscription creds = `~/.claude/.credentials.json` OR a Keychain item. We only
/// check presence (never read the token) via `security find-generic-password`.
fn creds_present(home: Option<&str>) -> bool {
    if let Some(h) = home {
        if std::path::Path::new(&format!("{h}/.claude/.credentials.json")).exists() {
            return true;
        }
    }
    std::process::Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Feature-detect the `--json-schema` flag (absent on older CLI versions).
fn check_json_schema_support(path: &PathBuf) -> bool {
    std::process::Command::new(path)
        .arg("--help")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("--json-schema"))
        .unwrap_or(false)
}

pub async fn translate(
    info: &ClaudeInfo,
    text: &str,
    target_lang: &str,
) -> Result<Translated, ProviderError> {
    let sys = prompt::system_prompt(target_lang);
    let mut cmd = Command::new(&info.path);
    cmd.args([
        "-p",
        "--output-format",
        "json",
        "--model",
        "haiku",
        "--system-prompt",
        &sys,
    ]);
    if info.supports_json_schema {
        cmd.args(["--json-schema", prompt::FURIGANA_SCHEMA]);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|_| ProviderError::Unavailable)?;
    if let Some(mut stdin) = child.stdin.take() {
        // Feed the user text via stdin to avoid argv length/escaping limits.
        let _ = stdin.write_all(text.as_bytes()).await;
        // stdin dropped here → EOF.
    }

    let output = tokio::time::timeout(Duration::from_secs(60), child.wait_with_output())
        .await
        .map_err(|_| ProviderError::Timeout)?
        .map_err(|_| ProviderError::Unavailable)?;

    parse_envelope(&String::from_utf8_lossy(&output.stdout))
}

/// Parse the `claude -p --output-format json` envelope into a Translated.
fn parse_envelope(stdout: &str) -> Result<Translated, ProviderError> {
    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).map_err(|_| ProviderError::BadResponse)?;

    if v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false) {
        let msg = v.get("result").and_then(|r| r.as_str()).unwrap_or("");
        let lower = msg.to_lowercase();
        if lower.contains("limit") || lower.contains("quota") || lower.contains("credit") {
            return Err(ProviderError::QuotaExhausted);
        }
        return Err(ProviderError::Unavailable);
    }

    // Prefer schema-validated structured_output; fall back to the result string.
    if let Some(so) = v.get("structured_output") {
        if !so.is_null() {
            return serde_json::from_value(so.clone()).map_err(|_| ProviderError::BadResponse);
        }
    }
    let result = v
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or(ProviderError::BadResponse)?;
    serde_json::from_str(strip_fences(result).as_str()).map_err(|_| ProviderError::BadResponse)
}

/// Strip a leading ```json / ``` fence and trailing ``` if the model added them.
fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix("```json").or_else(|| t.strip_prefix("```")).unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_envelope, strip_fences};

    #[test]
    fn strips_json_code_fence() {
        assert_eq!(strip_fences("```json\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_fences("{\"a\":1}"), "{\"a\":1}");
    }

    #[test]
    fn parses_structured_output_envelope() {
        let envelope = r#"{"is_error":false,"structured_output":{"translation":"こんにちは","segments":[]}}"#;
        let out = parse_envelope(envelope).unwrap();
        assert_eq!(out.translation, "こんにちは");
    }

    #[test]
    fn maps_quota_error() {
        let envelope = r#"{"is_error":true,"result":"usage limit reached"}"#;
        assert!(matches!(
            parse_envelope(envelope),
            Err(super::ProviderError::QuotaExhausted)
        ));
    }
}
