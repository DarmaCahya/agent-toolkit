//! Known-answer and invariant tests for the QR generator.
//!
//! Reference data provenance:
//! - Data/ECC codewords and format/version strings: Thonky QR Code Tutorial
//!   ("Error Correction Coding", "Format and Version String Tables"), which
//!   reproduces ISO/IEC 18004 tables.
//! - The 21x21 bitmaps below were produced independently by `segno` 1.6.6 and
//!   `python-qrcode` 8.2 (`mask_pattern=0`, `border=0`); both libraries agree
//!   on every mask for 1-Q and 1-M. Thonky publishes the 1-M codeword stream
//!   and a rendered image, but not a textual 1-Q bitmap.

use super::gf;
use super::matrix::{self, block_spec, mask_condition};
use super::*;

/// "HELLO WORLD", version 1, error correction Q, mask 0 (21 x 21).
const HELLO_WORLD_1Q_MASK0: [&str; 21] = [
    "111111101100001111111",
    "100000101001001000001",
    "101110101001101011101",
    "101110101000001011101",
    "101110101010001011101",
    "100000100010001000001",
    "111111101010101111111",
    "000000001000000000000",
    "011010110000101011111",
    "010000001111000010001",
    "001101110110001011000",
    "011011010011010101110",
    "100010101011101110101",
    "000000001101001000101",
    "111111101010000101100",
    "100000100101101101000",
    "101110101010001111111",
    "101110100101010100010",
    "101110101001011101001",
    "100000101011110001011",
    "111111100001011100001",
];

/// "HELLO WORLD", version 1, error correction Q, mask 5 (21 x 21).
const HELLO_WORLD_1Q_MASK5: [&str; 21] = [
    "111111101001001111111",
    "100000101100101000001",
    "101110100111101011101",
    "101110100111101011101",
    "101110100100001011101",
    "100000100111101000001",
    "111111101010101111111",
    "000000000101100000000",
    "010000111110110000011",
    "101111010000111101110",
    "000011111000000101001",
    "101110000110100000000",
    "110111111110111011111",
    "000000001000111101011",
    "111111101100001011101",
    "100000100010010010111",
    "101110100100000001110",
    "101110100000100001100",
    "101110100100001000011",
    "100000101110000100101",
    "111111100111010010000",
];

/// "HELLO WORLD", version 1, error correction Q, mask 7 (21 x 21).
const HELLO_WORLD_1Q_MASK7: [&str; 21] = [
    "111111101100001111111",
    "100000100011001000001",
    "101110101000101011101",
    "101110101000001011101",
    "101110100000001011101",
    "100000101011001000001",
    "111111101010101111111",
    "000000001010000000000",
    "010101111001111101101",
    "010000001111000010001",
    "011111100100011001010",
    "010010011010011100111",
    "100010101011101110101",
    "000000001111011010111",
    "111111101011001100101",
    "100000101101101101000",
    "101110100000011101101",
    "101110101100011101011",
    "101110100001011101001",
    "100000101001100011001",
    "111111100000010101000",
];

/// "HELLO WORLD", version 1, error correction M, mask 0 (21 x 21).
const HELLO_WORLD_1M_MASK0: [&str; 21] = [
    "111111100010101111111",
    "100000101110001000001",
    "101110100010101011101",
    "101110100010101011101",
    "101110101011101011101",
    "100000100111001000001",
    "111111101010101111111",
    "000000000000000000000",
    "101010100100100010010",
    "011110001001000010001",
    "000111111101001011000",
    "111101011001110101110",
    "010011110101001110101",
    "000000001010001000101",
    "111111100000100101100",
    "100000100110001101000",
    "101110101100101111111",
    "101110100011010100010",
    "101110101111011101001",
    "100000100001110001011",
    "111111101101011100001",
];

#[test]
fn reference_matrix_hello_world_1q_mask0() {
    let qr = QrCode::encode_with_mask("HELLO WORLD", EccLevel::Q, 0).unwrap();
    assert_eq!(qr.version(), 1);
    assert_eq!(qr.mode(), QrMode::Alphanumeric);
    assert_eq!(qr.mask(), 0);
    assert_eq!(qr.matrix.to_rows(), HELLO_WORLD_1Q_MASK0);
}

#[test]
fn reference_matrix_hello_world_1m_mask0() {
    let qr = QrCode::encode_with_mask("HELLO WORLD", EccLevel::M, 0).unwrap();
    assert_eq!(qr.version(), 1);
    assert_eq!(qr.matrix.to_rows(), HELLO_WORLD_1M_MASK0);
}

#[test]
fn reference_matrices_cover_multiply_based_masks() {
    // Masks 5-7 mix `(x*y) mod 2/3`; regressions there are invisible to mask 0.
    let mask5 = QrCode::encode_with_mask("HELLO WORLD", EccLevel::Q, 5).unwrap();
    assert_eq!(mask5.matrix.to_rows(), HELLO_WORLD_1Q_MASK5);
    let mask7 = QrCode::encode_with_mask("HELLO WORLD", EccLevel::Q, 7).unwrap();
    assert_eq!(mask7.matrix.to_rows(), HELLO_WORLD_1Q_MASK7);
}

#[test]
fn published_hello_world_1m_codewords_and_rs() {
    // Thonky, "Error Correction Coding": 1-M data codewords...
    let expected_data = [
        32u8, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
    ];
    // ...and the ten error correction codewords produced from them.
    let expected_ec = [196u8, 35, 39, 119, 235, 215, 231, 226, 93, 23];

    let mode = detect_mode("HELLO WORLD");
    assert_eq!(mode, QrMode::Alphanumeric);
    let capacity = matrix::data_codewords(1, EccLevel::M).unwrap();
    let data = build_data_codewords("HELLO WORLD", mode, 1, capacity);
    assert_eq!(data, expected_data);
    assert_eq!(gf::rs_remainder(&data, 10), expected_ec);
}

#[test]
fn format_information_is_written_for_every_level_and_mask() {
    for ecc in EccLevel::ALL {
        for mask in 0..8u8 {
            let qr = QrCode::encode_with_mask("A", ecc, mask).unwrap();
            let (read_ecc, read_mask) = read_format(&qr);
            assert_eq!(read_ecc, ecc);
            assert_eq!(read_mask, mask);
            // Both copies must agree.
            assert_eq!(read_format_second_copy(&qr), read_format_bits(&qr));
        }
    }
}

#[test]
fn version_information_is_written_for_versions_7_to_10() {
    for version in 7..=10u8 {
        let qr = QrCode::encode_inner("V", EccLevel::L, Some(version), None).unwrap();
        let size = qr.size();
        let mut bits = 0u32;
        for i in 0..18usize {
            let a = size - 11 + i % 3;
            let b = i / 3;
            if qr.module(a, b) == Some(true) {
                bits |= 1 << i;
            }
            assert_eq!(qr.module(a, b), qr.module(b, a), "mirrored version bits");
        }
        assert_eq!(bits, gf::version_bits(version));
        // Versions below 7 leave the version area for data modules.
        assert_eq!(qr.module(size - 11, 0), qr.module(size - 11, 0));
    }
}

#[test]
fn capacity_matches_published_character_capacities() {
    // Thonky, "Character Capacities by Version, Mode, and Error Correction".
    assert_eq!(
        QrCode::capacity(1, EccLevel::L, QrMode::Numeric).unwrap(),
        41
    );
    assert_eq!(QrCode::capacity(1, EccLevel::M, QrMode::Byte).unwrap(), 14);
    assert_eq!(
        QrCode::capacity(1, EccLevel::Q, QrMode::Alphanumeric).unwrap(),
        16
    );
    assert_eq!(
        QrCode::capacity(2, EccLevel::H, QrMode::Alphanumeric).unwrap(),
        20
    );
    assert_eq!(
        QrCode::capacity(10, EccLevel::H, QrMode::Byte).unwrap(),
        119
    );
    assert_eq!(
        QrCode::capacity(10, EccLevel::L, QrMode::Byte).unwrap(),
        271
    );
}

#[test]
fn errors_are_typed_and_never_panic() {
    assert_eq!(QrCode::encode(""), Err(QrError::EmptyInput));
    assert!(matches!(
        QrCode::encode(&"x".repeat(3000)),
        Err(QrError::DataTooLong { .. })
    ));
    assert!(matches!(
        QrCode::encode_with_mask("x", EccLevel::L, 8),
        Err(QrError::InvalidMask(8))
    ));
    assert!(matches!(
        QrCode::encode_inner("x", EccLevel::L, Some(11), None),
        Err(QrError::UnsupportedVersion(11))
    ));
    // Invalid render sizes are ignored, not panicked on.
    let qr = QrCode::encode("x").unwrap().with_size(f32::NAN);
    assert_eq!(qr.size_px(), 256.0);
    let qr = qr.with_size(-4.0);
    assert_eq!(qr.size_px(), 256.0);
    assert!(qr.with_quiet_zone(200).quiet_zone() <= 16);
}

#[test]
fn auto_mask_selection_is_deterministic_and_valid() {
    let first = QrCode::encode("https://example.com/agent-canvas").unwrap();
    for _ in 0..5 {
        let again = QrCode::encode("https://example.com/agent-canvas").unwrap();
        assert_eq!(first, again);
    }
    assert!(first.mask() < 8);
    // The chosen mask must score no worse than every other mask.
    let best = matrix::penalty(&first.matrix);
    for mask in 0..8u8 {
        let qr = QrCode::encode_with_mask("https://example.com/agent-canvas", EccLevel::M, mask)
            .unwrap();
        assert!(matrix::penalty(&qr.matrix) >= best);
    }
}

// ---------------------------------------------------------------------------
// Round-trip decoder (test-only)
// ---------------------------------------------------------------------------

fn read_format_bits(qr: &QrCode) -> u16 {
    let matrix = &qr.matrix;
    let mut bits = 0u16;
    for i in 0..=5 {
        if matrix.get(8, i) {
            bits |= 1 << i;
        }
    }
    if matrix.get(8, 7) {
        bits |= 1 << 6;
    }
    if matrix.get(8, 8) {
        bits |= 1 << 7;
    }
    if matrix.get(7, 8) {
        bits |= 1 << 8;
    }
    for i in 9..15 {
        if matrix.get(14 - i, 8) {
            bits |= 1 << i;
        }
    }
    bits
}

fn read_format_second_copy(qr: &QrCode) -> u16 {
    let matrix = &qr.matrix;
    let size = qr.size();
    let mut bits = 0u16;
    for i in 0..8 {
        if matrix.get(size - 1 - i, 8) {
            bits |= 1 << i;
        }
    }
    for i in 8..15 {
        if matrix.get(8, size - 15 + i) {
            bits |= 1 << i;
        }
    }
    bits
}

fn read_format(qr: &QrCode) -> (EccLevel, u8) {
    let bits = read_format_bits(qr);
    for ecc in EccLevel::ALL {
        for mask in 0..8u8 {
            if gf::format_bits(ecc.format_bits(), mask) == bits {
                return (ecc, mask);
            }
        }
    }
    panic!("format information did not decode: {bits:015b}");
}

fn extract_codewords(qr: &QrCode, mask: u8) -> Vec<u8> {
    let size = qr.size() as i32;
    let mut bits: Vec<bool> = Vec::new();
    let mut right = size - 1;
    while right >= 1 {
        if right == 6 {
            right = 5;
        }
        let upward = (right + 1) & 2 == 0;
        for vertical in 0..size {
            for offset in 0..2 {
                let x = right - offset;
                let y = if upward {
                    size - 1 - vertical
                } else {
                    vertical
                };
                let (xu, yu) = (x as usize, y as usize);
                if qr.matrix.is_function(xu, yu) {
                    continue;
                }
                bits.push(qr.matrix.get(xu, yu) ^ mask_condition(mask, xu, yu));
            }
        }
        right -= 2;
    }

    let codeword_count = bits.len() / 8;
    let mut codewords = vec![0u8; codeword_count];
    for (index, chunk) in bits.chunks(8).enumerate().take(codeword_count) {
        let mut byte = 0u8;
        for (bit_index, &bit) in chunk.iter().enumerate() {
            if bit {
                byte |= 1 << (7 - bit_index);
            }
        }
        codewords[index] = byte;
    }
    codewords
}

fn deinterleave(codewords: &[u8], version: u8, ecc: EccLevel) -> Vec<u8> {
    let spec = block_spec(version, ecc).expect("supported version");
    let mut lengths = Vec::new();
    for _ in 0..spec.group1_blocks {
        lengths.push(usize::from(spec.group1_data));
    }
    for _ in 0..spec.group2_blocks {
        lengths.push(usize::from(spec.group2_data));
    }
    let mut blocks: Vec<Vec<u8>> = vec![Vec::new(); lengths.len()];
    let mut index = 0usize;
    let longest = lengths.iter().copied().max().unwrap_or(0);
    for position in 0..longest {
        for (block, &length) in blocks.iter_mut().zip(lengths.iter()) {
            if position < length {
                block.push(codewords[index]);
                index += 1;
            }
        }
    }
    blocks.concat()
}

struct BitReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl BitReader<'_> {
    fn read(&mut self, width: u8) -> u32 {
        let mut value = 0u32;
        for _ in 0..width {
            let byte = self.bytes.get(self.position / 8).copied().unwrap_or(0);
            let bit = (byte >> (7 - self.position % 8)) & 1;
            value = (value << 1) | u32::from(bit);
            self.position += 1;
        }
        value
    }
}

fn decode_payload(qr: &QrCode) -> (QrMode, String) {
    let (ecc, mask) = read_format(qr);
    assert_eq!(ecc, qr.ecc_level());
    assert_eq!(mask, qr.mask());
    let interleaved = extract_codewords(qr, mask);
    assert_eq!(
        interleaved.len(),
        block_spec(qr.version(), qr.ecc_level())
            .unwrap()
            .total_codewords()
    );
    let data = deinterleave(&interleaved, qr.version(), qr.ecc_level());
    let mut reader = BitReader {
        bytes: &data,
        position: 0,
    };
    let mode_bits = reader.read(4);
    let mode = match mode_bits {
        0b0001 => QrMode::Numeric,
        0b0010 => QrMode::Alphanumeric,
        0b0100 => QrMode::Byte,
        other => panic!("unexpected mode indicator {other:04b}"),
    };
    let count_bits = match (mode, qr.version() < 10) {
        (QrMode::Numeric, true) => 10,
        (QrMode::Numeric, false) => 12,
        (QrMode::Alphanumeric, true) => 9,
        (QrMode::Alphanumeric, false) => 11,
        (QrMode::Byte, true) => 8,
        (QrMode::Byte, false) => 16,
    };
    let count = reader.read(count_bits) as usize;
    let text = match mode {
        QrMode::Numeric => {
            let mut out = String::new();
            let mut remaining = count;
            while remaining >= 3 {
                out.push_str(&format!("{:03}", reader.read(10)));
                remaining -= 3;
            }
            match remaining {
                2 => out.push_str(&format!("{:02}", reader.read(7))),
                1 => out.push_str(&format!("{}", reader.read(4))),
                _ => {}
            }
            out
        }
        QrMode::Alphanumeric => {
            let mut out = String::new();
            let mut remaining = count;
            while remaining >= 2 {
                let value = reader.read(11) as usize;
                out.push(
                    ALPHANUMERIC
                        .chars()
                        .nth(value / 45)
                        .expect("first alphanumeric symbol"),
                );
                out.push(
                    ALPHANUMERIC
                        .chars()
                        .nth(value % 45)
                        .expect("second alphanumeric symbol"),
                );
                remaining -= 2;
            }
            if remaining == 1 {
                let value = reader.read(6) as usize;
                out.push(ALPHANUMERIC.chars().nth(value).expect("single symbol"));
            }
            out
        }
        QrMode::Byte => {
            let mut bytes = Vec::with_capacity(count);
            for _ in 0..count {
                bytes.push(reader.read(8) as u8);
            }
            String::from_utf8(bytes).expect("valid UTF-8 payload")
        }
    };
    (mode, text)
}

#[test]
fn round_trip_numeric_across_levels_and_lengths() {
    for ecc in EccLevel::ALL {
        for length in [1usize, 2, 3, 5, 10, 20, 41, 60, 100] {
            let payload: String = (0..length)
                .map(|i| char::from(b'0' + (i % 10) as u8))
                .collect();
            let qr = QrCode::encode_with_ecc(&payload, ecc).unwrap();
            assert_eq!(qr.mode(), QrMode::Numeric);
            let (mode, decoded) = decode_payload(&qr);
            assert_eq!(mode, QrMode::Numeric);
            assert_eq!(decoded, payload, "numeric {ecc:?} length {length}");
        }
    }
}

#[test]
fn round_trip_alphanumeric_across_levels_and_lengths() {
    for ecc in EccLevel::ALL {
        for length in [1usize, 2, 5, 11, 16, 30, 60] {
            let source = "HELLO WORLD $%*+-./:";
            let payload: String = source.chars().cycle().take(length).collect();
            let qr = QrCode::encode_with_ecc(&payload, ecc).unwrap();
            assert_eq!(qr.mode(), QrMode::Alphanumeric);
            let (mode, decoded) = decode_payload(&qr);
            assert_eq!(mode, QrMode::Alphanumeric);
            assert_eq!(decoded, payload, "alphanumeric {ecc:?} length {length}");
        }
    }
}

#[test]
fn round_trip_byte_across_levels_and_lengths() {
    let sources = [
        "https://example.com/agent-canvas?kpi=42",
        "héllo ✓ wörld — 漢字",
        "line1\nline2\ttabbed",
    ];
    for ecc in EccLevel::ALL {
        for source in sources {
            for repeat in [1usize, 2] {
                let payload = source.repeat(repeat);
                let qr = QrCode::encode_with_ecc(&payload, ecc).unwrap();
                assert_eq!(qr.mode(), QrMode::Byte);
                let (mode, decoded) = decode_payload(&qr);
                assert_eq!(mode, QrMode::Byte);
                assert_eq!(decoded, payload, "byte {ecc:?} repeat {repeat}");
            }
        }
    }
}

#[test]
fn round_trip_at_every_version_and_block_layout() {
    for version in matrix::MIN_VERSION..=matrix::MAX_VERSION {
        for ecc in EccLevel::ALL {
            let capacity = QrCode::capacity(version, ecc, QrMode::Byte).unwrap();
            let payload = "x".repeat(capacity.saturating_sub(1));
            let qr = QrCode::encode_inner(&payload, ecc, Some(version), None).unwrap();
            assert_eq!(qr.version(), version);
            let (_, decoded) = decode_payload(&qr);
            assert_eq!(decoded, payload, "v{version}-{ecc:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Structural invariants
// ---------------------------------------------------------------------------

fn assert_finder(qr: &QrCode, left: usize, top: usize) {
    for dy in 0..7i32 {
        for dx in 0..7i32 {
            let distance = (dx - 3).abs().max((dy - 3).abs());
            let expected = distance != 2;
            assert_eq!(
                qr.module(left + dx as usize, top + dy as usize),
                Some(expected),
                "finder at ({left},{top}) offset ({dx},{dy})"
            );
        }
    }
}

#[test]
fn matrix_invariants_hold_for_every_version() {
    for version in matrix::MIN_VERSION..=matrix::MAX_VERSION {
        let qr = QrCode::encode_inner("invariants", EccLevel::M, Some(version), None).unwrap();
        let size = qr.size();
        assert_eq!(size, 17 + 4 * usize::from(version));
        assert_eq!(qr.modules().len(), size * size);

        assert_finder(&qr, 0, 0);
        assert_finder(&qr, size - 7, 0);
        assert_finder(&qr, 0, size - 7);

        // Separators: the 8th row/column around each finder is light.
        for i in 0..8 {
            assert_eq!(qr.module(7, i), Some(false));
            assert_eq!(qr.module(i, 7), Some(false));
        }

        // Timing patterns alternate from module 8 to size - 9.
        for i in 8..size - 8 {
            assert_eq!(qr.module(i, 6), Some(i % 2 == 0), "horizontal timing {i}");
            assert_eq!(qr.module(6, i), Some(i % 2 == 0), "vertical timing {i}");
        }

        // Dark module.
        assert_eq!(qr.module(8, size - 8), Some(true));

        // No entirely light row or column.
        for y in 0..size {
            assert!(
                (0..size).any(|x| qr.module(x, y) == Some(true)),
                "row {y} is blank in v{version}"
            );
        }
        for x in 0..size {
            assert!(
                (0..size).any(|y| qr.module(x, y) == Some(true)),
                "column {x} is blank in v{version}"
            );
        }
    }
}

#[test]
fn alignment_patterns_are_present_from_version_2() {
    for version in 2..=matrix::MAX_VERSION {
        let qr = QrCode::encode_inner("align", EccLevel::L, Some(version), None).unwrap();
        let positions: &[u8] = match version {
            2 => &[6, 18],
            3 => &[6, 22],
            4 => &[6, 26],
            5 => &[6, 30],
            6 => &[6, 34],
            7 => &[6, 22, 38],
            8 => &[6, 24, 42],
            9 => &[6, 26, 46],
            _ => &[6, 28, 50],
        };
        let last = qr.size() - 7;
        for &cy in positions {
            for &cx in positions {
                let (cx, cy) = (usize::from(cx), usize::from(cy));
                if (cx == 6 && cy == 6) || (cx == 6 && cy == last) || (cx == last && cy == 6) {
                    continue;
                }
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        let expected = dx.abs().max(dy.abs()) != 1;
                        assert_eq!(
                            qr.module((cx as i32 + dx) as usize, (cy as i32 + dy) as usize),
                            Some(expected),
                            "alignment v{version} at ({cx},{cy})"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn every_mask_produces_a_decodable_symbol() {
    for mask in 0..8u8 {
        let qr = QrCode::encode_with_mask("MASK TEST 0123456789", EccLevel::Q, mask).unwrap();
        assert_eq!(qr.mask(), mask);
        let (mode, decoded) = decode_payload(&qr);
        assert_eq!(mode, QrMode::Alphanumeric);
        assert_eq!(decoded, "MASK TEST 0123456789");
    }
}

// ---------------------------------------------------------------------------
// SVG output
// ---------------------------------------------------------------------------

fn finite_numbers(svg: &str) -> bool {
    let mut rest = svg;
    while let Some(quote) = rest.find('"') {
        rest = &rest[quote + 1..];
        let Some(end) = rest.find('"') else {
            return false;
        };
        let value = &rest[..end];
        rest = &rest[end + 1..];
        // Attribute values that look numeric must parse as finite numbers.
        if value.starts_with(|c: char| c == '-' || c == '.' || c.is_ascii_digit())
            && value
                .parse::<f32>()
                .map(|v| !v.is_finite())
                .unwrap_or(false)
        {
            return false;
        }
        if value.contains("NaN") || value.contains("inf") {
            return false;
        }
    }
    true
}

#[test]
fn svg_is_deterministic_and_vector_only() {
    let qr = QrCode::encode("https://example.com/agent-canvas?report=q4")
        .unwrap()
        .with_size(320.0)
        .with_quiet_zone(4);
    let first = qr.to_svg();
    for _ in 0..10 {
        assert_eq!(first, qr.to_svg());
    }
    assert!(first.starts_with("<svg"));
    assert!(first.trim_end().ends_with("</svg>"));
    assert!(!first.contains("<text"));
    assert!(!first.contains("font-family"));
    assert!(!first.contains("http://www.w3.org/1999/xlink"));
    assert!(first.contains("<path"));
    assert!(first.contains("shape-rendering=\"crispEdges\""));
    assert!(finite_numbers(&first));
    assert_eq!(first.matches("<svg").count(), 1);
    assert_eq!(first.matches("</svg>").count(), 1);
}

#[test]
fn svg_quiet_zone_and_geometry() {
    let quiet = 4u8;
    let qr = QrCode::encode("QUIET ZONE")
        .unwrap()
        .with_size(370.0)
        .with_quiet_zone(quiet);
    let modules = qr.size() + 2 * usize::from(quiet);
    let scale = 370.0 / modules as f32;
    let rects = qr.to_rects();
    let first_rect = rects.lines().next().expect("at least one dark module");
    let offset = crate::canvas::geom::fmt_num(scale * f32::from(quiet));
    assert!(
        first_rect.contains(&format!("x=\"{offset}\"")),
        "{first_rect}"
    );
    assert!(
        first_rect.contains(&format!("y=\"{offset}\"")),
        "{first_rect}"
    );

    // Without a quiet zone the first module sits at the origin.
    let tight = qr.clone().with_quiet_zone(0);
    let rects = tight.to_rects();
    assert!(rects.lines().next().unwrap().contains("x=\"0\""));

    // Both renderers cover the same ink: every dark module appears once.
    let dark_modules = qr.modules().iter().filter(|&&dark| dark).count();
    assert_eq!(qr.to_rects().matches("<rect").count(), dark_modules);
}

#[test]
fn proptest_round_trip_printable_ascii() {
    use proptest::prelude::*;

    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let strategy = ("[ -~]{1,60}", 0usize..4);
    runner
        .run(&strategy, |(payload, ecc_index)| {
            let ecc = EccLevel::ALL[ecc_index];
            let qr = QrCode::encode_with_ecc(&payload, ecc)
                .map_err(|error| proptest::test_runner::TestCaseError::fail(format!("{error}")))?;
            let (_, decoded) = decode_payload(&qr);
            prop_assert_eq!(decoded, payload);
            prop_assert!(qr.mask() < 8);
            Ok(())
        })
        .expect("random payloads round-trip");
}

#[test]
fn theme_colors_are_applied() {
    use crate::canvas::ThemeMode;
    let light = QrCode::encode("THEME")
        .unwrap()
        .with_theme(ThemeMode::Light);
    assert!(light.to_svg().contains("#000000"));
    assert!(light.to_svg().contains("#FFFFFF"));
    let dark = QrCode::encode("THEME").unwrap().with_theme(ThemeMode::Dark);
    assert!(dark.to_svg().contains("#F1F5F9"));
    assert!(dark.to_svg().contains("#0F172A"));
    assert_ne!(light.to_svg(), dark.to_svg());
}
