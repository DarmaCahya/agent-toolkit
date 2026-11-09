//! Typed errors for the LaTeX typesetter.

use thiserror::Error;

/// Error returned when a formula cannot be typeset.
///
/// Positions are character (not byte) indices into the input, available through
/// [`MathError::position`]. Every message names the offending command or
/// character and ends with a hint. The enum is `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum MathError {
    /// The formula is empty or whitespace only.
    #[error("formula is empty; hint: provide at least one math token")]
    EmptyFormula,
    /// The formula exceeds the length limit.
    #[error("formula too long: at least {len} characters (max {max}); hint: shorten the input")]
    InputTooLong {
        /// Observed length (at least).
        len: usize,
        /// Allowed maximum.
        max: usize,
    },
    /// Groups are nested deeper than allowed.
    #[error("nesting too deep at position {pos} (max {max}); hint: flatten nested groups")]
    NestingTooDeep {
        /// Char position.
        pos: usize,
        /// Allowed maximum.
        max: usize,
    },
    /// Input ended while more was required.
    #[error("unexpected end of input at position {pos}, expected {expected}; hint: close open groups or complete the command")]
    UnexpectedEof {
        /// Char position.
        pos: usize,
        /// What was expected.
        expected: String,
    },
    /// A character outside the supported set.
    #[error("unexpected character {ch:?} at position {pos}; hint: only letters, digits, + - = < > ( ) , and supported commands are allowed")]
    UnexpectedChar {
        /// Char position.
        pos: usize,
        /// Offending character.
        ch: char,
    },
    /// A command that does not exist.
    #[error("unknown command \\{name} at position {pos}; hint: check the spelling")]
    UnknownCommand {
        /// Char position.
        pos: usize,
        /// Command name without the backslash.
        name: String,
    },
    /// A recognised LaTeX command that this version does not support.
    #[error("unsupported command \\{name} at position {pos}; hint: {hint}")]
    Unsupported {
        /// Char position.
        pos: usize,
        /// Command name without the backslash.
        name: String,
        /// Suggested alternative.
        hint: String,
    },
    /// A command is missing a required argument.
    #[error("missing argument for \\{command} at position {pos}; hint: supply a braced argument")]
    MissingArgument {
        /// Char position.
        pos: usize,
        /// Command name.
        command: String,
    },
    /// Two superscripts or two subscripts on one base.
    #[error("double {kind} at position {pos}; hint: group the base, e.g. {{x^2}}^3")]
    DoubleScript {
        /// Char position of the second script marker.
        pos: usize,
        /// `"superscript"` or `"subscript"`.
        kind: &'static str,
    },
    /// The bundled font lacks a glyph.
    #[error("no glyph for {ch:?} in the bundled math font; hint: use a supported symbol")]
    MissingGlyph {
        /// Character without a glyph.
        ch: char,
    },
    /// Font data or layout arithmetic failed.
    #[error("font error: {reason}; hint: this is an internal limitation")]
    Font {
        /// Reason.
        reason: String,
    },
}

impl MathError {
    /// Char position of the error, when it has one.
    #[must_use]
    pub fn position(&self) -> Option<usize> {
        match self {
            Self::EmptyFormula
            | Self::InputTooLong { .. }
            | Self::MissingGlyph { .. }
            | Self::Font { .. } => None,
            Self::NestingTooDeep { pos, .. }
            | Self::UnexpectedEof { pos, .. }
            | Self::UnexpectedChar { pos, .. }
            | Self::UnknownCommand { pos, .. }
            | Self::Unsupported { pos, .. }
            | Self::MissingArgument { pos, .. }
            | Self::DoubleScript { pos, .. } => Some(*pos),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn position_per_variant() {
        assert_eq!(MathError::EmptyFormula.position(), None);
        assert_eq!(MathError::InputTooLong { len: 9, max: 8 }.position(), None);
        assert_eq!(MathError::MissingGlyph { ch: 'x' }.position(), None);
        assert_eq!(MathError::Font { reason: "r".into() }.position(), None);
        assert_eq!(
            MathError::NestingTooDeep { pos: 3, max: 64 }.position(),
            Some(3)
        );
        assert_eq!(
            MathError::UnexpectedChar { pos: 2, ch: '$' }.position(),
            Some(2)
        );
        assert_eq!(
            MathError::DoubleScript {
                pos: 5,
                kind: "superscript"
            }
            .position(),
            Some(5)
        );
    }

    #[test]
    fn display_names_item_and_hint() {
        let e = MathError::UnknownCommand {
            pos: 0,
            name: "foo".into(),
        };
        assert!(e.to_string().contains("\\foo") && e.to_string().contains("hint"));
        let e = MathError::UnexpectedChar { pos: 1, ch: '$' };
        assert!(e.to_string().contains('$') && e.to_string().contains("hint"));
    }

    #[test]
    fn traits() {
        fn assert_traits<T: Clone + PartialEq + Eq + std::fmt::Debug>() {}
        assert_traits::<MathError>();
    }
}
