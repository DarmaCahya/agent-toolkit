#![forbid(unsafe_code)]

//! Colors, theme modes and the categorical palette.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Alpha in `0.0..=1.0`.
    pub a: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorParseError {
    /// Digit count (excluding `#`) was not 3, 6 or 8.
    InvalidLength(usize),
    /// A non-hexadecimal character was encountered.
    InvalidDigit(char),
}

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(n) => {
                write!(f, "invalid hex color length {n}: expected 3, 6 or 8 digits")
            }
            Self::InvalidDigit(c) => write!(f, "invalid hex digit {c:?}"),
        }
    }
}

impl Error for ColorParseError {}

impl Color {
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// `a` is clamped to `0.0..=1.0`; NaN becomes `1.0`.
    pub fn from_rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r,
            g,
            b,
            a: sanitize_alpha(a),
        }
    }

    /// Parses `#RGB`, `#RRGGBB` or `#RRGGBBAA` (leading `#` optional).
    pub fn from_hex(hex: &str) -> Result<Self, ColorParseError> {
        let digits = hex.strip_prefix('#').unwrap_or(hex);
        if let Some(bad) = digits.chars().find(|c| !c.is_ascii_hexdigit()) {
            return Err(ColorParseError::InvalidDigit(bad));
        }
        // All chars are ASCII hex digits, so byte slicing below is safe.
        let pair = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).unwrap_or(0);
        let nibble = |i: usize| {
            let v = u8::from_str_radix(&digits[i..i + 1], 16).unwrap_or(0);
            v * 17
        };
        match digits.len() {
            3 => Ok(Self::from_rgb(nibble(0), nibble(1), nibble(2))),
            6 => Ok(Self::from_rgb(pair(0), pair(2), pair(4))),
            8 => Ok(Self::from_rgba(
                pair(0),
                pair(2),
                pair(4),
                f32::from(pair(6)) / 255.0,
            )),
            n => Err(ColorParseError::InvalidLength(n)),
        }
    }

    /// Uppercase hex: `#RRGGBB`, or `#RRGGBBAA` when alpha is below 1.
    pub fn to_hex(&self) -> String {
        if self.a < 1.0 {
            let alpha = (sanitize_alpha(self.a) * 255.0).round() as u8;
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, alpha)
        } else {
            self.to_rgb_hex()
        }
    }

    /// Uppercase `#RRGGBB`, ignoring alpha (for SVG `fill`/`stroke` attributes).
    pub fn to_rgb_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn to_rgba_string(&self) -> String {
        let a = (sanitize_alpha(self.a) * 1000.0).round() / 1000.0;
        format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, a)
    }

    pub fn with_opacity(&self, a: f32) -> Color {
        Color {
            a: sanitize_alpha(a),
            ..*self
        }
    }
}

fn sanitize_alpha(a: f32) -> f32 {
    if a.is_nan() {
        1.0
    } else {
        a.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

const CATEGORICAL: [Color; 8] = [
    Color::from_rgb(0x25, 0x63, 0xEB), // Sapphire
    Color::from_rgb(0x05, 0x96, 0x69), // Emerald
    Color::from_rgb(0xD9, 0x77, 0x06), // Amber
    Color::from_rgb(0xE1, 0x1D, 0x48), // Crimson
    Color::from_rgb(0x4F, 0x46, 0xE5), // Indigo
    Color::from_rgb(0x0D, 0x94, 0x88), // Teal
    Color::from_rgb(0x47, 0x55, 0x69), // Slate
    Color::from_rgb(0x7C, 0x3A, 0xED), // Violet
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Palette {
    pub mode: ThemeMode,
}

impl Palette {
    pub const CATEGORICAL_COUNT: usize = CATEGORICAL.len();

    pub const fn new(mode: ThemeMode) -> Self {
        Self { mode }
    }

    /// Cyclic lookup: indices wrap modulo 8.
    pub fn get_categorical(&self, index: usize) -> Color {
        CATEGORICAL[index % CATEGORICAL.len()]
    }

    pub fn background(&self) -> Color {
        match self.mode {
            ThemeMode::Light => Color::from_rgb(0xFF, 0xFF, 0xFF),
            ThemeMode::Dark => Color::from_rgb(0x0F, 0x17, 0x2A),
        }
    }

    pub fn surface(&self) -> Color {
        match self.mode {
            ThemeMode::Light => Color::from_rgb(0xF1, 0xF5, 0xF9),
            ThemeMode::Dark => Color::from_rgb(0x1E, 0x29, 0x3B),
        }
    }

    pub fn text_primary(&self) -> Color {
        match self.mode {
            ThemeMode::Light => Color::from_rgb(0x0F, 0x17, 0x2A),
            ThemeMode::Dark => Color::from_rgb(0xF1, 0xF5, 0xF9),
        }
    }

    pub fn text_secondary(&self) -> Color {
        match self.mode {
            ThemeMode::Light => Color::from_rgb(0x64, 0x74, 0x8B),
            ThemeMode::Dark => Color::from_rgb(0x94, 0xA3, 0xB8),
        }
    }

    pub fn gridline(&self) -> Color {
        match self.mode {
            ThemeMode::Light => Color::from_rgb(0xE2, 0xE8, 0xF0),
            ThemeMode::Dark => Color::from_rgb(0x1E, 0x29, 0x3B),
        }
    }

    pub fn border(&self) -> Color {
        match self.mode {
            ThemeMode::Light => Color::from_rgb(0xCB, 0xD5, 0xE1),
            ThemeMode::Dark => Color::from_rgb(0x33, 0x41, 0x55),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing() {
        assert_eq!(
            Color::from_hex("#2563EB").unwrap(),
            Color::from_rgb(0x25, 0x63, 0xEB)
        );
        assert_eq!(
            Color::from_hex("#fa0").unwrap(),
            Color::from_rgb(0xFF, 0xAA, 0x00)
        );
        let c = Color::from_hex("#00000080").unwrap();
        assert!((c.a - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(
            Color::from_hex("#12"),
            Err(ColorParseError::InvalidLength(2))
        );
        assert_eq!(
            Color::from_hex("#12G456"),
            Err(ColorParseError::InvalidDigit('G'))
        );
        assert!(Color::from_hex("#é1é").is_err());
    }

    #[test]
    fn formatting() {
        let c = Color::from_rgb(1, 2, 3);
        assert_eq!(c.to_hex(), "#010203");
        assert_eq!(c.with_opacity(0.5).to_rgba_string(), "rgba(1, 2, 3, 0.5)");
        assert_eq!(c.with_opacity(2.0).a, 1.0);
        assert_eq!(c.with_opacity(0.0).to_hex(), "#01020300");
    }

    #[test]
    fn categorical_wraps() {
        let p = Palette::new(ThemeMode::Light);
        assert_eq!(p.get_categorical(0), p.get_categorical(8));
        assert_eq!(p.get_categorical(7).to_hex(), "#7C3AED");
    }
}
