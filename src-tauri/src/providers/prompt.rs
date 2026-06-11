//! System prompts + the shared JSON schema. One schema works for both providers
//! (OpenAI strict json_schema and Claude --json-schema).
//!
//! Two furigana arrays:
//! - `segments`       → furigana of the OUTPUT translation (only when target = ja)
//! - `source_segments`→ furigana of the SOURCE text (whenever the source is Japanese)

/// JSON schema for `{ translation, segments[], source_segments[] }`.
/// strict-mode friendly: every property required, additionalProperties false.
pub const FURIGANA_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "translation": { "type": "string" },
    "segments": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": { "surface": { "type": "string" }, "reading": { "type": "string" } },
        "required": ["surface", "reading"],
        "additionalProperties": false
      }
    },
    "source_segments": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": { "surface": { "type": "string" }, "reading": { "type": "string" } },
        "required": ["surface", "reading"],
        "additionalProperties": false
      }
    }
  },
  "required": ["translation", "segments", "source_segments"],
  "additionalProperties": false
}"#;

const RULES: &str = "Tokenization rule for any furigana array: split the target text into consecutive \
tokens that EXACTLY reconcatenate to it (no gaps or overlaps). Whitespace, line breaks and punctuation \
are each their own token. For each token, 'surface' = the token text and 'reading' = its pronunciation \
in HIRAGANA only if the token contains kanji, otherwise an empty string. Never romanize.";

/// Build the system prompt for a target language. `segments` carries furigana
/// only when the output is Japanese; `source_segments` carries furigana whenever
/// the user's source text is Japanese (so reading JA→VI still shows furigana).
pub fn system_prompt(target_lang: &str) -> String {
    let (lang_name, output_is_ja) = match target_lang {
        "ja" => ("Japanese", true),
        "en" => ("English", false),
        _ => ("Vietnamese", false),
    };
    let segments_rule = if output_is_ja {
        "'segments' = furigana tokens of 'translation' (the Japanese output)."
    } else {
        "'segments' = [] (the output is not Japanese)."
    };
    format!(
        "You are a translation engine. Translate the user's text into natural {lang}. Return ONLY JSON \
matching the schema (no markdown fences). 'translation' = the full {lang} translation. {seg} \
'source_segments' = furigana tokens of the SOURCE text (the user's input) if it contains Japanese, \
otherwise []. {rules}",
        lang = lang_name,
        seg = segments_rule,
        rules = RULES
    )
}
