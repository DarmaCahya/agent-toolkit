#![forbid(unsafe_code)]

//! Typed errors for the QR Code generator.

use super::{EccLevel, QrMode};
use thiserror::Error;

/// Errors returned by [`QrCode`](super::QrCode) construction and rendering.
///
/// The enum is `#[non_exhaustive]`: new variants may be added without a
/// breaking change.
#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum QrError {
    /// The input string was empty.
    #[error("cannot encode an empty string")]
    EmptyInput,

    /// The payload does not fit in any supported version at the chosen ECC level.
    #[error(
        "input of {len} characters in {mode} mode does not fit versions 1..={max_version} \
         at error correction level {ecc}"
    )]
    DataTooLong {
        /// Length of the input in characters.
        len: usize,
        /// Encoding mode that was selected for the input.
        mode: QrMode,
        /// Highest supported version at the time of the call.
        max_version: u8,
        /// Requested error correction level.
        ecc: EccLevel,
    },

    /// A version outside `1..=10` was requested.
    #[error("unsupported QR version {0}: this generator supports versions 1..=10")]
    UnsupportedVersion(u8),

    /// A mask outside `0..=7` was requested.
    #[error("invalid mask pattern {0}: expected 0..=7")]
    InvalidMask(u8),
}
