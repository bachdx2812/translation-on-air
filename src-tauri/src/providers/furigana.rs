//! Pure furigana validation. Runs in Rust before the UI sees segments so the
//! frontend never has to re-validate or guard against malformed model output.

use super::types::Segment;

/// CJK unified ideographs (common + extension A).
fn is_kanji(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
}

/// True if the text contains any Japanese: hiragana, katakana, or kanji.
/// Used to decide whether to annotate the SOURCE text with furigana.
pub fn contains_japanese(s: &str) -> bool {
    s.chars().any(|c| {
        let u = c as u32;
        (0x3040..=0x30FF).contains(&u) || matches!(u, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
    })
}

/// Normalize katakana readings to hiragana (U+30A1..=U+30F6 → −0x60).
fn kata_to_hira(s: &str) -> String {
    s.chars()
        .map(|c| {
            let u = c as u32;
            if (0x30A1..=0x30F6).contains(&u) {
                char::from_u32(u - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// Validate + clean segments. If the surfaces don't reconcatenate to the
/// translation, return an empty vec (the UI falls back to plain text). Readings
/// are normalized to hiragana and dropped for non-kanji tokens.
pub fn validate_segments(translation: &str, segments: Vec<Segment>) -> Vec<Segment> {
    let concat: String = segments.iter().map(|s| s.surface.as_str()).collect();
    if concat != translation {
        return Vec::new();
    }
    segments
        .into_iter()
        .map(|s| {
            let reading = if s.surface.chars().any(is_kanji) {
                kata_to_hira(&s.reading)
            } else {
                String::new()
            };
            Segment {
                surface: s.surface,
                reading,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(surface: &str, reading: &str) -> Segment {
        Segment {
            surface: surface.to_string(),
            reading: reading.to_string(),
        }
    }

    #[test]
    fn keeps_kanji_reading_drops_kana_reading() {
        let out = validate_segments("日本語", vec![seg("日本語", "にほんご")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].reading, "にほんご");
    }

    #[test]
    fn drops_all_when_reconcat_mismatch() {
        let out = validate_segments("日本", vec![seg("日", "に")]);
        assert!(out.is_empty());
    }

    #[test]
    fn normalizes_katakana_reading_to_hiragana() {
        let out = validate_segments("水", vec![seg("水", "ミズ")]);
        assert_eq!(out[0].reading, "みず");
    }

    #[test]
    fn blanks_reading_for_non_kanji_token() {
        let out = validate_segments("です", vec![seg("です", "です")]);
        assert_eq!(out[0].reading, "");
    }

    #[test]
    fn detects_japanese_source() {
        assert!(contains_japanese("今日は")); // kanji + kana
        assert!(contains_japanese("テスト")); // katakana
        assert!(!contains_japanese("hello world"));
        assert!(!contains_japanese("Xin chào")); // Vietnamese, no Japanese
    }
}
