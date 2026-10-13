//! Local Claude CLI provider (subscription auth, no API key). Runs
//! `claude -p --output-format json` as a subprocess and parses the JSON envelope.

use super::prompt;
use super::types::{ProviderError, Translated};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ClaudeInfo {
    pub path: PathBuf,
    pub supports_json_schema: bool,
    /// `--safe-mode` skips the user's CLAUDE.md, hooks, plugins and MCP servers
    /// but keeps subscription auth (unlike `--bare`), so a translation neither
    /// pays for that context nor gets hook output mixed into stdout.
    pub supports_safe_mode: bool,
}

/// Detect a usable Claude CLI: a known binary path that exists AND subscription
/// credentials present. GUI apps inherit a minimal PATH, so probe absolute paths.
pub fn detect() -> Option<ClaudeInfo> {
    let home = home_dir();
    let path = candidates(home.as_deref())
        .into_iter()
        .find(|p| p.exists())?;
    if !creds_present(home.as_deref()) {
        return None;
    }
    let help = help_text(&path);
    Some(ClaudeInfo {
        supports_json_schema: help.contains("--json-schema"),
        supports_safe_mode: help.contains("--safe-mode"),
        path,
    })
}

/// Where to look for `claude`, real installs first. cmux's bundled `claude` is
/// a wrapper that re-resolves the real binary through PATH, so it is only a
/// last resort (and works only thanks to `child_path`).
fn candidates(home: Option<&str>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(h) = home {
        paths.push(PathBuf::from(format!("{h}/.local/bin/claude")));
    }
    paths.extend(
        [
            "/opt/homebrew/bin/claude",
            "/usr/local/bin/claude",
            "/Applications/cmux.app/Contents/Resources/bin/claude",
        ]
        .map(PathBuf::from),
    );
    paths
}

/// PATH for the CLI subprocess. GUI apps inherit a minimal PATH, and an npm
/// install of `claude` is a `#!/usr/bin/env node` script, so the directory it
/// lives in (where npm, nvm and Homebrew also put `node`) must be searchable.
fn child_path(cli: &Path, home: Option<&str>) -> String {
    let mut dirs: Vec<String> = Vec::new();
    let mut push = |d: String| {
        if !d.is_empty() && !dirs.contains(&d) {
            dirs.push(d);
        }
    };
    if let Some(dir) = cli.parent() {
        push(dir.display().to_string());
    }
    push("/opt/homebrew/bin".into());
    push("/usr/local/bin".into());
    if let Some(h) = home {
        push(format!("{h}/.local/bin"));
    }
    for d in ["/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
        push(d.into());
    }
    dirs.join(":")
}

fn home_dir() -> Option<String> {
    std::env::var("HOME").ok()
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

/// `claude --help`, used to feature-detect flags older CLI versions lack.
fn help_text(path: &Path) -> String {
    std::process::Command::new(path)
        .arg("--help")
        .env("PATH", child_path(path, home_dir().as_deref()))
        .stderr(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// Translate `text` using a caller-built system prompt (shared by plain
/// translation and contextual replies).
pub async fn translate(
    info: &ClaudeInfo,
    sys: &str,
    text: &str,
) -> Result<Translated, ProviderError> {
    let mut cmd = Command::new(&info.path);
    cmd.args([
        "-p",
        "--output-format",
        "json",
        "--model",
        "haiku",
        "--system-prompt",
        sys,
    ]);
    if info.supports_json_schema {
        cmd.args(["--json-schema", prompt::FURIGANA_SCHEMA]);
    }
    if info.supports_safe_mode {
        cmd.arg("--safe-mode");
    }
    cmd.env("PATH", child_path(&info.path, home_dir().as_deref()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result = parse_envelope(&stdout);
    if result.is_err() {
        eprintln!(
            "[claude_cli] {:?} (exit {:?}); stderr: {}",
            result.as_ref().err(),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    result
}

/// The `"type": "result"` object in the CLI's stdout. Normally stdout is just
/// that object, but a `verbose` setting turns it into an array of every
/// message, and hooks or plugins may print extra lines around it.
fn find_result(stdout: &str) -> Option<serde_json::Value> {
    fn is_result(v: &serde_json::Value) -> bool {
        v.get("type").and_then(|t| t.as_str()) == Some("result")
    }
    fn pick(v: serde_json::Value) -> Option<serde_json::Value> {
        match v {
            serde_json::Value::Array(items) => items.into_iter().rev().find(is_result),
            v if v.is_object() => Some(v),
            _ => None,
        }
    }
    if let Ok(v) = serde_json::from_str(stdout.trim()) {
        return pick(v);
    }
    stdout
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .find_map(|v| pick(v).filter(is_result))
}

/// Parse the `claude -p --output-format json` envelope into a Translated.
fn parse_envelope(stdout: &str) -> Result<Translated, ProviderError> {
    if stdout.trim().is_empty() {
        // The CLI died before answering (e.g. `node` not found for an npm
        // install): the service is unavailable, not the answer unreadable.
        return Err(ProviderError::Unavailable);
    }
    let v = find_result(stdout).ok_or(ProviderError::BadResponse)?;

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
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{candidates, child_path, parse_envelope, strip_fences};
    use std::path::Path;

    #[test]
    fn strips_json_code_fence() {
        assert_eq!(strip_fences("```json\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_fences("{\"a\":1}"), "{\"a\":1}");
    }

    #[test]
    fn parses_structured_output_envelope() {
        let envelope =
            r#"{"is_error":false,"structured_output":{"translation":"こんにちは","segments":[]}}"#;
        let out = parse_envelope(envelope).unwrap();
        assert_eq!(out.translation, "こんにちは");
    }

    #[test]
    fn finds_result_in_verbose_message_array() {
        let stdout = r#"[{"type":"system","subtype":"init"},{"type":"assistant","message":{}},{"type":"result","is_error":false,"structured_output":{"translation":"Cảm ơn","segments":[],"source_segments":[]}}]"#;
        assert_eq!(parse_envelope(stdout).unwrap().translation, "Cảm ơn");
    }

    #[test]
    fn ignores_lines_printed_around_the_result() {
        let stdout = "hook: session started\n{\"type\":\"result\",\"is_error\":false,\"structured_output\":{\"translation\":\"Cảm ơn\",\"segments\":[],\"source_segments\":[]}}\nSessionEnd hook done\n";
        assert_eq!(parse_envelope(stdout).unwrap().translation, "Cảm ơn");
    }

    #[test]
    fn empty_output_means_unavailable() {
        assert!(matches!(
            parse_envelope("  \n"),
            Err(super::ProviderError::Unavailable)
        ));
        assert!(matches!(
            parse_envelope("not json at all"),
            Err(super::ProviderError::BadResponse)
        ));
    }

    #[test]
    fn child_path_puts_the_cli_dir_first_without_duplicates() {
        let path = child_path(Path::new("/opt/homebrew/bin/claude"), Some("/Users/me"));
        assert_eq!(
            path,
            "/opt/homebrew/bin:/usr/local/bin:/Users/me/.local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
        );
        let nvm = child_path(
            Path::new("/Users/me/.nvm/versions/node/v22/bin/claude"),
            None,
        );
        assert!(nvm.starts_with("/Users/me/.nvm/versions/node/v22/bin:/opt/homebrew/bin:"));
    }

    #[test]
    fn prefers_real_installs_over_the_cmux_wrapper() {
        let paths = candidates(Some("/Users/me"));
        assert_eq!(
            paths.first().unwrap(),
            Path::new("/Users/me/.local/bin/claude")
        );
        assert!(paths.last().unwrap().starts_with("/Applications/cmux.app"));
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
