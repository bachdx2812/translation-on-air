use serde::{Deserialize, Serialize};

/// One token of a Japanese translation. `reading` is hiragana for kanji-bearing
/// tokens, empty otherwise. For vi/en translations `segments` is empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub surface: String,
    pub reading: String,
}

/// A completed translation returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translated {
    pub translation: String,
    /// Furigana of the output (populated only when target = ja).
    #[serde(default)]
    pub segments: Vec<Segment>,
    /// Furigana of the source text (populated whenever the source is Japanese).
    #[serde(default)]
    pub source_segments: Vec<Segment>,
}

/// Translation failures, mapped to stable string codes the frontend renders as
/// human messages. The Display impl IS the code (used by `code()`).
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("not-configured")]
    NotConfigured,
    #[error("invalid-key")]
    InvalidKey,
    #[error("rate-limited")]
    RateLimited,
    #[error("quota-exhausted")]
    QuotaExhausted,
    #[error("unavailable")]
    Unavailable,
    #[error("bad-response")]
    BadResponse,
    #[error("timeout")]
    Timeout,
}

impl ProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            ProviderError::NotConfigured => "not-configured",
            ProviderError::InvalidKey => "invalid-key",
            ProviderError::RateLimited => "rate-limited",
            ProviderError::QuotaExhausted => "quota-exhausted",
            ProviderError::Unavailable => "unavailable",
            ProviderError::BadResponse => "bad-response",
            ProviderError::Timeout => "timeout",
        }
    }
}
