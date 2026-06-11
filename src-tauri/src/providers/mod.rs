pub mod claude_cli;
pub mod furigana;
pub mod openai;
pub mod prompt;
pub mod types;

use claude_cli::ClaudeInfo;
use std::sync::Mutex;
use types::ProviderError;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProviderMode {
    Auto,
    Claude,
    OpenAi,
}

impl ProviderMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "claude" => ProviderMode::Claude,
            "openai" => ProviderMode::OpenAi,
            _ => ProviderMode::Auto,
        }
    }
}

pub enum Provider {
    ClaudeCli(ClaudeInfo),
    OpenAi,
}

/// Resolve the provider to use. Auto prefers OpenAI (≈1–2s, instant-popup UX),
/// then falls back to the local Claude CLI for zero-config subscribers. Forced
/// modes error if their dependency is missing.
pub fn resolve(
    mode: ProviderMode,
    claude: Option<ClaudeInfo>,
    has_openai_key: bool,
) -> Result<Provider, ProviderError> {
    match mode {
        ProviderMode::OpenAi => {
            if has_openai_key {
                Ok(Provider::OpenAi)
            } else {
                Err(ProviderError::NotConfigured)
            }
        }
        ProviderMode::Claude => claude.map(Provider::ClaudeCli).ok_or(ProviderError::NotConfigured),
        ProviderMode::Auto => {
            if has_openai_key {
                Ok(Provider::OpenAi)
            } else if let Some(c) = claude {
                Ok(Provider::ClaudeCli(c))
            } else {
                Err(ProviderError::NotConfigured)
            }
        }
    }
}

/// Caches the Claude CLI detection (filesystem + creds probe) for the app's
/// lifetime. Outer Option = "have we probed yet"; inner = the probe result.
#[derive(Default)]
pub struct ProviderCache {
    claude: Mutex<Option<Option<ClaudeInfo>>>,
}

impl ProviderCache {
    pub fn claude_info(&self) -> Option<ClaudeInfo> {
        let mut guard = self.claude.lock().unwrap();
        if guard.is_none() {
            *guard = Some(claude_cli::detect());
        }
        (*guard).clone().flatten()
    }

    #[allow(dead_code)] // invoked by settings changes in phase 06
    pub fn invalidate(&self) {
        *self.claude.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> ClaudeInfo {
        ClaudeInfo {
            path: std::path::PathBuf::from("/x/claude"),
            supports_json_schema: true,
        }
    }

    #[test]
    fn auto_prefers_openai_key_over_claude() {
        assert!(matches!(
            resolve(ProviderMode::Auto, Some(info()), true),
            Ok(Provider::OpenAi)
        ));
    }

    #[test]
    fn auto_falls_back_to_claude_without_key() {
        assert!(matches!(
            resolve(ProviderMode::Auto, Some(info()), false),
            Ok(Provider::ClaudeCli(_))
        ));
    }

    #[test]
    fn auto_not_configured_when_nothing_available() {
        assert!(matches!(
            resolve(ProviderMode::Auto, None, false),
            Err(ProviderError::NotConfigured)
        ));
    }

    #[test]
    fn forced_openai_without_key_errors() {
        assert!(matches!(
            resolve(ProviderMode::OpenAi, Some(info()), false),
            Err(ProviderError::NotConfigured)
        ));
    }
}
