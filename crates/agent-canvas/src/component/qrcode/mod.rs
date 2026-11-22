#![forbid(unsafe_code)]

//! Pure-Rust QR Code generator with deterministic SVG output.
//!
//! Implements ISO/IEC 18004 byte, alphanumeric and numeric encoding for
//! versions 1 through 10, error correction levels L/M/Q/H, Reed-Solomon block
//! interleaving, all eight data masks with standard penalty scoring, and
//! vector-only SVG rendering (no `<text>`, no fonts, no external resources).
//!
//! ```
//! use agent_canvas::component::QrCode;
//!
//! let svg = QrCode::encode("https://example.com/agent-canvas")?
//!     .with_size(320.0)
//!     .to_svg();
//! assert!(svg.starts_with("<svg"));
//! # Ok::<(), agent_canvas::component::qrcode::QrError>(())
//! ```

mod error;
mod gf;
mod matrix;
mod render;

#[cfg(test)]
mod tests;

pub use error::QrError;

use crate::canvas::{Color, ThemeMode};
use std::fmt;

/// Encoding mode selected for a payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QrMode {
    /// Digits `0-9`, three characters per 10 bits.
    Numeric,
    /// `0-9 A-Z $%*+-./:` and space, two characters per 11 bits.
    Alphanumeric,
    /// Arbitrary bytes (the input is UTF-8 encoded).
    Byte,
}

impl QrMode {
    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Numeric => "numeric",
            Self::Alphanumeric => "alphanumeric",
            Self::Byte => "byte",
        }
    }

    fn indicator(self) -> u32 {
        match self {
            Self::Numeric => 0b0001,
            Self::Alphanumeric => 0b0010,
            Self::Byte => 0b0100,
        }
    }

    /// Width of the character count indicator for `version` (versions 1..=10).
    fn count_bits(self, version: u8) -> u8 {
        match (self, version < 10) {
            (Self::Numeric, true) => 10,
            (Self::Numeric, false) => 12,
            (Self::Alphanumeric, true) => 9,
            (Self::Alphanumeric, false) => 11,
            (Self::Byte, true) => 8,
            (Self::Byte, false) => 16,
        }
    }

    /// Number of characters (bytes for byte mode) that the payload uses.
    fn count(self, data: &str) -> usize {
        match self {
            Self::Byte => data.len(),
            _ => data.chars().count(),
        }
    }

    fn payload_bits(self, data: &str) -> usize {
        let count = self.count(data);
        match self {
            Self::Numeric => {
                (count / 3) * 10
                    + match count % 3 {
                        1 => 4,
                        2 => 7,
                        _ => 0,
                    }
            }
            Self::Alphanumeric => (count / 2) * 11 + (count % 2) * 6,
            Self::Byte => count * 8,
        }
    }

    fn write_payload(self, data: &str, writer: &mut BitWriter) {
        match self {
            Self::Numeric => {
                let bytes = data.as_bytes();
                for chunk in bytes.chunks(3) {
                    let mut value = 0u32;
                    for &byte in chunk {
                        value = value * 10 + u32::from(byte - b'0');
                    }
                    let bits = match chunk.len() {
                        3 => 10,
                        2 => 7,
                        _ => 4,
                    };
                    writer.push(value, bits);
                }
            }
            Self::Alphanumeric => {
                let indices: Vec<u32> = data
                    .chars()
                    .filter_map(|c| ALPHANUMERIC.find(c))
                    .map(|index| index as u32)
                    .collect();
                for pair in indices.chunks(2) {
                    match pair {
                        [a, b] => writer.push(a * 45 + b, 11),
                        [a] => writer.push(*a, 6),
                        _ => {}
                    }
                }
            }
            Self::Byte => {
                for &byte in data.as_bytes() {
                    writer.push(u32::from(byte), 8);
                }
            }
        }
    }
}

impl fmt::Display for QrMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Error correction level (ISO/IEC 18004 levels L, M, Q and H).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EccLevel {
    /// Recovers about 7% of codewords.
    L,
    /// Recovers about 15% of codewords.
    #[default]
    M,
    /// Recovers about 25% of codewords.
    Q,
    /// Recovers about 30% of codewords.
    H,
}

impl EccLevel {
    /// All levels, weakest to strongest.
    pub const ALL: [EccLevel; 4] = [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H];

    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            Self::L => "L",
            Self::M => "M",
            Self::Q => "Q",
            Self::H => "H",
        }
    }

    /// The two-bit value written in the format information field.
    pub(crate) fn format_bits(self) -> u8 {
        match self {
            Self::L => 0b01,
            Self::M => 0b00,
            Self::Q => 0b11,
            Self::H => 0b10,
        }
    }
}

impl fmt::Display for EccLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// The alphanumeric character set, in code order.
const ALPHANUMERIC: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// A generated QR Code symbol and its render settings.
///
/// Create one with [`QrCode::encode`], tune it with the builder methods, then
/// render it with [`QrCode::to_svg`], [`QrCode::to_path`] or
/// [`QrCode::to_rects`].
#[derive(Debug, Clone, PartialEq)]
pub struct QrCode {
    version: u8,
    ecc: EccLevel,
    mode: QrMode,
    mask: u8,
    matrix: matrix::Matrix,
    quiet_zone: u8,
    size_px: f32,
    dark: Color,
    light: Color,
}

impl QrCode {
    /// Encodes `data` with error correction level M, choosing the smallest
    /// version that fits and the lowest-penalty mask.
    ///
    /// The encoding mode is chosen automatically: numeric when the input is
    /// all digits, alphanumeric when every character is in the QR alphanumeric
    /// set, byte otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`QrError::EmptyInput`] for an empty string and
    /// [`QrError::DataTooLong`] when the payload exceeds version 10 capacity at
    /// the requested error correction level.
    ///
    /// ```
    /// use agent_canvas::component::QrCode;
    ///
    /// let qr = QrCode::encode("HELLO WORLD")?;
    /// assert_eq!(qr.version(), 1);
    /// assert_eq!(qr.size(), 21);
    /// assert!(qr.to_svg().contains("<path"));
    /// # Ok::<(), agent_canvas::component::qrcode::QrError>(())
    /// ```
    pub fn encode(data: &str) -> Result<Self, QrError> {
        Self::encode_inner(data, EccLevel::default(), None, None)
    }

    /// Encodes `data` with an explicit error correction level.
    ///
    /// # Errors
    ///
    /// See [`QrCode::encode`].
    pub fn encode_with_ecc(data: &str, ecc: EccLevel) -> Result<Self, QrError> {
        Self::encode_inner(data, ecc, None, None)
    }

    /// Encodes `data` with an explicit error correction level and mask.
    ///
    /// This is mainly useful for reproducible fixtures and known-answer tests;
    /// normal callers should let the encoder choose the mask.
    ///
    /// # Errors
    ///
    /// See [`QrCode::encode`], plus [`QrError::InvalidMask`] for `mask > 7`.
    pub fn encode_with_mask(data: &str, ecc: EccLevel, mask: u8) -> Result<Self, QrError> {
        if mask > 7 {
            return Err(QrError::InvalidMask(mask));
        }
        Self::encode_inner(data, ecc, None, Some(mask))
    }

    fn encode_inner(
        data: &str,
        ecc: EccLevel,
        version: Option<u8>,
        mask: Option<u8>,
    ) -> Result<Self, QrError> {
        if data.is_empty() {
            return Err(QrError::EmptyInput);
        }
        if let Some(requested) = version {
            if !(matrix::MIN_VERSION..=matrix::MAX_VERSION).contains(&requested) {
                return Err(QrError::UnsupportedVersion(requested));
            }
        }

        let mode = detect_mode(data);
        let chosen_version = match version {
            Some(requested) => requested,
            None => select_version(data, mode, ecc).ok_or(QrError::DataTooLong {
                len: mode.count(data),
                mode,
                max_version: matrix::MAX_VERSION,
                ecc,
            })?,
        };

        let data_capacity = matrix::data_codewords(chosen_version, ecc)
            .ok_or(QrError::UnsupportedVersion(chosen_version))?;
        let required = 4 + usize::from(mode.count_bits(chosen_version)) + mode.payload_bits(data);
        if required > data_capacity * 8 {
            return Err(QrError::DataTooLong {
                len: mode.count(data),
                mode,
                max_version: matrix::MAX_VERSION,
                ecc,
            });
        }

        let data_codewords = build_data_codewords(data, mode, chosen_version, data_capacity);
        let codewords = interleave(&data_codewords, chosen_version, ecc);

        let (matrix, used_mask) = match mask {
            Some(forced) => (
                matrix::build(chosen_version, ecc, forced, &codewords),
                forced,
            ),
            None => matrix::build_best(chosen_version, ecc, &codewords),
        };

        Ok(Self {
            version: chosen_version,
            ecc,
            mode,
            mask: used_mask,
            matrix,
            quiet_zone: 4,
            size_px: 256.0,
            dark: Color::from_rgb(0, 0, 0),
            light: Color::from_rgb(255, 255, 255),
        })
    }

    /// Sets the square render size in pixels.
    ///
    /// Non-finite or non-positive values are ignored, so the previous size is
    /// kept. The default is 256 px.
    pub fn with_size(mut self, px: f32) -> Self {
        if px.is_finite() && px > 0.0 {
            self.size_px = px;
        }
        self
    }

    /// Sets the quiet zone width in modules (default 4, clamped to 0..=16).
    pub fn with_quiet_zone(mut self, modules: u8) -> Self {
        self.quiet_zone = modules.min(16);
        self
    }

    /// Sets the dark (foreground) and light (background) colours.
    pub fn with_colors(mut self, dark: Color, light: Color) -> Self {
        self.dark = dark;
        self.light = light;
        self
    }

    /// Sets the dark module colour, keeping the background.
    pub fn with_dark_color(mut self, dark: Color) -> Self {
        self.dark = dark;
        self
    }

    /// Sets the background colour.
    pub fn with_light_color(mut self, light: Color) -> Self {
        self.light = light;
        self
    }

    /// Applies the light or dark canvas theme.
    ///
    /// Light renders black modules on white; dark renders near-white modules on
    /// the dark canvas background (both keep QR contrast).
    pub fn with_theme(mut self, mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => {
                self.dark = Color::from_rgb(0x00, 0x00, 0x00);
                self.light = Color::from_rgb(0xFF, 0xFF, 0xFF);
            }
            ThemeMode::Dark => {
                self.dark = Color::from_rgb(0xF1, 0xF5, 0xF9);
                self.light = Color::from_rgb(0x0F, 0x17, 0x2A);
            }
        }
        self
    }

    /// Version number (1..=10).
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Error correction level.
    pub fn ecc_level(&self) -> EccLevel {
        self.ecc
    }

    /// Encoding mode selected for the payload.
    pub fn mode(&self) -> QrMode {
        self.mode
    }

    /// Mask pattern used (0..=7).
    pub fn mask(&self) -> u8 {
        self.mask
    }

    /// Symbol edge length in modules (without the quiet zone).
    pub fn size(&self) -> usize {
        self.matrix.size()
    }

    /// Quiet zone in modules.
    pub fn quiet_zone(&self) -> u8 {
        self.quiet_zone
    }

    /// Render size in pixels.
    pub fn size_px(&self) -> f32 {
        self.size_px
    }

    /// Module colour, row-major, `size() * size()` entries.
    pub fn modules(&self) -> &[bool] {
        self.matrix.modules()
    }

    /// Module at column `x`, row `y`, or `None` when out of bounds.
    pub fn module(&self, x: usize, y: usize) -> Option<bool> {
        if x < self.matrix.size() && y < self.matrix.size() {
            Some(self.matrix.get(x, y))
        } else {
            None
        }
    }

    /// Largest number of characters that fit a version / level / mode.
    ///
    /// Uses the ISO/IEC 18004 character capacity tables, derived from the data
    /// codeword counts.
    ///
    /// ```
    /// use agent_canvas::component::{EccLevel, QrCode, QrMode};
    ///
    /// assert_eq!(QrCode::capacity(1, EccLevel::Q, QrMode::Alphanumeric)?, 16);
    /// assert_eq!(QrCode::capacity(1, EccLevel::M, QrMode::Byte)?, 14);
    /// # Ok::<(), agent_canvas::component::qrcode::QrError>(())
    /// ```
    pub fn capacity(version: u8, ecc: EccLevel, mode: QrMode) -> Result<usize, QrError> {
        let codewords =
            matrix::data_codewords(version, ecc).ok_or(QrError::UnsupportedVersion(version))?;
        let available = codewords * 8;
        let header = 4 + usize::from(mode.count_bits(version));
        let payload_bits = available.saturating_sub(header);
        let capacity = match mode {
            QrMode::Numeric => {
                let groups = payload_bits / 10;
                let remainder = payload_bits % 10;
                groups * 3
                    + if remainder >= 7 {
                        2
                    } else if remainder >= 4 {
                        1
                    } else {
                        0
                    }
            }
            QrMode::Alphanumeric => {
                let groups = payload_bits / 11;
                groups * 2 + usize::from(payload_bits % 11 >= 6)
            }
            QrMode::Byte => payload_bits / 8,
        };
        Ok(capacity)
    }
}

fn detect_mode(data: &str) -> QrMode {
    if data.bytes().all(|byte| byte.is_ascii_digit()) {
        QrMode::Numeric
    } else if data
        .chars()
        .all(|c| c.is_ascii() && ALPHANUMERIC.contains(c))
    {
        QrMode::Alphanumeric
    } else {
        QrMode::Byte
    }
}

fn select_version(data: &str, mode: QrMode, ecc: EccLevel) -> Option<u8> {
    for version in matrix::MIN_VERSION..=matrix::MAX_VERSION {
        let capacity = matrix::data_codewords(version, ecc)?;
        let required = 4 + usize::from(mode.count_bits(version)) + mode.payload_bits(data);
        if required <= capacity * 8 {
            return Some(version);
        }
    }
    None
}

#[derive(Default)]
struct BitWriter {
    bits: Vec<bool>,
}

impl BitWriter {
    fn push(&mut self, value: u32, width: u8) {
        for shift in (0..width).rev() {
            self.bits.push((value >> shift) & 1 == 1);
        }
    }

    fn len(&self) -> usize {
        self.bits.len()
    }

    /// Pads with zero bits to a whole number of codewords and packs them.
    fn into_codewords(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.bits.len().div_ceil(8));
        for chunk in self.bits.chunks(8) {
            let mut byte = 0u8;
            for (index, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << (7 - index);
                }
            }
            bytes.push(byte);
        }
        bytes
    }
}

fn build_data_codewords(data: &str, mode: QrMode, version: u8, capacity: usize) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.push(mode.indicator(), 4);
    let count = mode.count(data) as u32;
    writer.push(count, mode.count_bits(version));
    mode.write_payload(data, &mut writer);

    let total_bits = capacity * 8;
    let terminator = (total_bits.saturating_sub(writer.len())).min(4);
    writer.push(0, terminator as u8);

    let mut codewords = writer.into_codewords();
    let mut pad = true;
    while codewords.len() < capacity {
        codewords.push(if pad { 0xEC } else { 0x11 });
        pad = !pad;
    }
    codewords.truncate(capacity);
    codewords
}

/// Splits data into blocks, appends Reed-Solomon codewords and interleaves both.
fn interleave(data_codewords: &[u8], version: u8, ecc: EccLevel) -> Vec<u8> {
    let Some(spec) = matrix::block_spec(version, ecc) else {
        return Vec::new();
    };

    struct Block {
        data: Vec<u8>,
        ec: Vec<u8>,
    }

    let mut blocks: Vec<Block> =
        Vec::with_capacity(usize::from(spec.group1_blocks) + usize::from(spec.group2_blocks));
    let mut offset = 0usize;
    let mut push_block = |length: usize, blocks: &mut Vec<Block>| {
        let end = offset.saturating_add(length);
        let data = data_codewords.get(offset..end).unwrap_or(&[]).to_vec();
        let ec = gf::rs_remainder(&data, usize::from(spec.ec_per_block));
        blocks.push(Block { data, ec });
        offset = end;
    };
    for _ in 0..spec.group1_blocks {
        push_block(usize::from(spec.group1_data), &mut blocks);
    }
    for _ in 0..spec.group2_blocks {
        push_block(usize::from(spec.group2_data), &mut blocks);
    }

    let mut out = Vec::with_capacity(spec.total_codewords());
    let max_data = blocks
        .iter()
        .map(|block| block.data.len())
        .max()
        .unwrap_or(0);
    for index in 0..max_data {
        for block in &blocks {
            if let Some(&byte) = block.data.get(index) {
                out.push(byte);
            }
        }
    }
    for index in 0..usize::from(spec.ec_per_block) {
        for block in &blocks {
            if let Some(&byte) = block.ec.get(index) {
                out.push(byte);
            }
        }
    }
    out
}
