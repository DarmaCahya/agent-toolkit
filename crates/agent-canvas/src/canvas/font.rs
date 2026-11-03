#![forbid(unsafe_code)]

//! Heuristic text metrics for proportional sans-serif fonts (Inter/Roboto/Arial-like).
//!
//! Widths are estimates expressed as a fraction of the font size (em); they are
//! intended for layout, not for pixel-exact typesetting.

const NARROW_EM: f32 = 0.28;
const WIDE_EM: f32 = 0.85;
const UPPERCASE_EM: f32 = 0.65;
const LOWERCASE_EM: f32 = 0.52;
const DIGIT_EM: f32 = 0.55;
const SPACE_EM: f32 = 0.30;
const CJK_EM: f32 = 1.0;
const FALLBACK_EM: f32 = 0.55;
const LINE_HEIGHT_RATIO: f32 = 1.2;

const NARROW_CHARS: [char; 8] = ['i', 'l', 'j', '!', '.', ',', ':', ';'];
const WIDE_CHARS: [char; 6] = ['m', 'w', 'M', 'W', '@', '%'];

/// Namespace for text measurement helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FontMetrics;

impl FontMetrics {
    /// Width of a single character as a fraction of the font size.
    /// Control characters (including newlines) have zero width.
    pub fn char_width_ratio(c: char) -> f32 {
        if NARROW_CHARS.contains(&c) {
            NARROW_EM
        } else if WIDE_CHARS.contains(&c) {
            WIDE_EM
        } else if c == ' ' {
            SPACE_EM
        } else if c.is_ascii_digit() {
            DIGIT_EM
        } else if is_full_width(c) {
            CJK_EM
        } else if c.is_uppercase() {
            UPPERCASE_EM
        } else if c.is_lowercase() {
            LOWERCASE_EM
        } else if c.is_control() {
            0.0
        } else {
            FALLBACK_EM
        }
    }

    /// Estimated rendered width in the same unit as `font_size`.
    /// Returns `0.0` for non-finite or non-positive font sizes.
    pub fn estimate_text_width(text: &str, font_size: f32) -> f32 {
        if !font_size.is_finite() || font_size <= 0.0 {
            return 0.0;
        }
        text.chars().map(Self::char_width_ratio).sum::<f32>() * font_size
    }

    /// Estimated line height (~1.2 × font size).
    pub fn estimate_text_height(font_size: f32) -> f32 {
        if !font_size.is_finite() || font_size <= 0.0 {
            return 0.0;
        }
        font_size * LINE_HEIGHT_RATIO
    }
}

fn is_full_width(c: char) -> bool {
    matches!(u32::from(c),
        0x1100..=0x11FF          // Hangul Jamo
        | 0x2E80..=0x303F        // CJK radicals, symbols and punctuation
        | 0x3040..=0x30FF        // Hiragana, Katakana
        | 0x3100..=0x312F        // Bopomofo
        | 0x3130..=0x318F        // Hangul compatibility Jamo
        | 0x3400..=0x4DBF        // CJK extension A
        | 0x4E00..=0x9FFF        // CJK unified ideographs
        | 0xAC00..=0xD7AF        // Hangul syllables
        | 0xF900..=0xFAFF        // CJK compatibility ideographs
        | 0xFF00..=0xFF60        // Fullwidth forms
        | 0xFFE0..=0xFFE6        // Fullwidth signs
        | 0x20000..=0x2FA1F      // CJK extensions B–F, compatibility supplement
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes() {
        assert_eq!(FontMetrics::char_width_ratio('i'), 0.28);
        assert_eq!(FontMetrics::char_width_ratio('W'), 0.85);
        assert_eq!(FontMetrics::char_width_ratio('A'), 0.65);
        assert_eq!(FontMetrics::char_width_ratio('a'), 0.52);
        assert_eq!(FontMetrics::char_width_ratio('7'), 0.55);
        assert_eq!(FontMetrics::char_width_ratio(' '), 0.30);
        assert_eq!(FontMetrics::char_width_ratio('漢'), 1.0);
        assert_eq!(FontMetrics::char_width_ratio('#'), 0.55);
        assert_eq!(FontMetrics::char_width_ratio('\n'), 0.0);
    }

    #[test]
    fn width_and_height() {
        let w = FontMetrics::estimate_text_width("Hi", 10.0);
        assert!((w - (0.65 + 0.28) * 10.0).abs() < 1e-4);
        assert_eq!(FontMetrics::estimate_text_width("abc", -1.0), 0.0);
        assert_eq!(FontMetrics::estimate_text_width("abc", f32::NAN), 0.0);
        assert!((FontMetrics::estimate_text_height(10.0) - 12.0).abs() < 1e-5);
    }
}
