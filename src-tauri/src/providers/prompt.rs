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

/// Longest text (in characters) still sent with furigana requested. Furigana
/// tokens cost several times the translation's own output: on long texts the
/// model then either shortens the translation, drops the furigana, or runs for
/// minutes (an 8k-character document took 145s with furigana, 23s without).
pub const FURIGANA_MAX_CHARS: usize = 1000;

pub fn furigana_fits(text: &str) -> bool {
    text.chars().count() <= FURIGANA_MAX_CHARS
}

const COMPLETE: &str = "Translate the whole text: never summarize, shorten or leave out any part.";

fn lang_name(code: &str) -> &'static str {
    match code {
        "ja" => "Japanese",
        "en" => "English",
        _ => "Vietnamese",
    }
}

/// Wrap the text to translate and repeat the target language after it. Named
/// only in the system prompt, the target got lost on long CJK input: gpt-6-luna
/// (reasoning none) turned a 1,182-character Japanese message into Chinese in
/// 4–6 of 6 runs; with the language restated after the text, 6 of 6 came back
/// in Vietnamese. The tags keep that line from being translated as content.
fn wrap_source(text: &str, instruction: &str) -> String {
    format!("<source>\n{text}\n</source>\n{instruction}")
}

/// User message for a translation into `target_lang`.
pub fn source_message(text: &str, target_lang: &str) -> String {
    let lang = lang_name(target_lang);
    wrap_source(
        text,
        &format!("Translate the text inside <source> into {lang}."),
    )
}

/// User message for a reply going back into the original message's language.
pub fn reply_message(reply: &str) -> String {
    wrap_source(
        reply,
        "Translate the text inside <source> into the language of the ORIGINAL message.",
    )
}

const SOURCE_TAGS: &str = "The user message holds the text to translate between <source> and \
</source>; translate only that text, never the tags or the instruction after them.";

/// Build the system prompt for a target language. `output_furigana` asks for
/// furigana of a Japanese translation in `segments`; `source_furigana` asks for
/// furigana of a Japanese source in `source_segments`, which often costs more
/// output tokens than the translation itself.
pub fn system_prompt(target_lang: &str, output_furigana: bool, source_furigana: bool) -> String {
    let lang_name = lang_name(target_lang);
    let output_is_ja = target_lang == "ja";
    let output_furigana = output_furigana && output_is_ja;
    let segments_rule = if output_furigana {
        "'segments' = furigana tokens of 'translation' (the Japanese output)."
    } else if output_is_ja {
        "'segments' = []."
    } else {
        "'segments' = [] (the output is not Japanese)."
    };
    let source_rule = if source_furigana {
        "'source_segments' = furigana tokens of the SOURCE text (inside <source>) if it contains \
Japanese, otherwise []."
    } else {
        "'source_segments' = []."
    };
    let rules = if output_furigana || source_furigana {
        RULES
    } else {
        ""
    };
    format!(
        "You are a translation engine. {SOURCE_TAGS} Translate it into natural {lang_name}. Return \
ONLY JSON matching the schema (no markdown fences). 'translation' = the full {lang_name} \
translation. {COMPLETE} {segments_rule} {source_rule} {rules}"
    )
    .trim_end()
    .to_string()
}

/// Build the system prompt for translating a REPLY back into the original
/// message's language, using the prior exchange as context so the wording stays
/// consistent. The model infers the original language from `original_source`, so
/// no explicit source-language code is needed. `reply_lang` is the language the
/// user typed the reply in (the translation language). The user message is
/// `reply_message`. `furigana` asks for furigana when the output is Japanese.
pub fn reply_prompt(
    original_source: &str,
    original_translation: &str,
    reply_lang: &str,
    furigana: bool,
) -> String {
    let reply_lang_name = lang_name(reply_lang);
    let (segments_rule, rules) = if furigana {
        (
            "If that language is Japanese, 'segments' = furigana tokens of 'translation', otherwise [].",
            RULES,
        )
    } else {
        ("'segments' = [].", "")
    };
    format!(
        "You are a translation engine in an ongoing bilingual conversation. The other party wrote this \
ORIGINAL message:\n\"{src}\"\nwhich was translated for the user as:\n\"{tr}\"\nThe user now writes a \
REPLY in {reply_lang}. {SOURCE_TAGS} Translate the reply into the SAME LANGUAGE as the ORIGINAL message (so the \
other party can read it), keeping terminology and tone consistent with the conversation. Return ONLY \
JSON matching the schema (no markdown fences). 'translation' = the reply rewritten in the original \
message's language. {COMPLETE} {segments_rule} 'source_segments' = []. {rules}",
        src = original_source,
        tr = original_translation,
        reply_lang = reply_lang_name,
    )
    .trim_end()
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        furigana_fits, reply_message, reply_prompt, source_message, system_prompt,
        FURIGANA_MAX_CHARS,
    };

    #[test]
    fn source_message_repeats_the_target_after_the_text() {
        let m = source_message("ありがとう", "vi");
        assert_eq!(
            m,
            "<source>\nありがとう\n</source>\nTranslate the text inside <source> into Vietnamese."
        );
        assert!(reply_message("Cảm ơn").starts_with("<source>\nCảm ơn\n</source>\n"));
    }

    #[test]
    fn prompts_explain_the_source_tags() {
        assert!(system_prompt("en", false, false).contains("between <source> and </source>"));
        assert!(reply_prompt("こんにちは", "Xin chào", "vi", false).contains("<source>"));
    }

    #[test]
    fn source_furigana_off_requests_empty_source_segments() {
        let p = system_prompt("vi", false, false);
        assert!(p.contains("'source_segments' = []."));
        assert!(!p.contains("Tokenization rule"));
    }

    #[test]
    fn source_furigana_on_requests_source_tokens() {
        let p = system_prompt("vi", false, true);
        assert!(p.contains("furigana tokens of the SOURCE text"));
        assert!(p.contains("Tokenization rule"));
    }

    #[test]
    fn japanese_output_keeps_tokenization_rules() {
        let p = system_prompt("ja", true, false);
        assert!(p.contains("'segments' = furigana tokens of 'translation'"));
        assert!(p.contains("Tokenization rule"));
    }

    #[test]
    fn japanese_output_without_furigana_requests_no_tokens() {
        let p = system_prompt("ja", false, false);
        assert!(p.contains("'segments' = []."));
        assert!(!p.contains("Tokenization rule"));
    }

    #[test]
    fn prompts_ask_for_the_complete_text() {
        assert!(system_prompt("vi", false, false).contains("never summarize"));
        assert!(reply_prompt("こんにちは", "Xin chào", "vi", true).contains("never summarize"));
    }

    #[test]
    fn reply_without_furigana_drops_the_rules() {
        let p = reply_prompt("こんにちは", "Xin chào", "vi", false);
        assert!(p.contains("'segments' = []."));
        assert!(!p.contains("Tokenization rule"));
    }

    #[test]
    fn furigana_fits_counts_characters_not_bytes() {
        assert!(furigana_fits(&"あ".repeat(FURIGANA_MAX_CHARS)));
        assert!(!furigana_fits(&"あ".repeat(FURIGANA_MAX_CHARS + 1)));
    }
}
