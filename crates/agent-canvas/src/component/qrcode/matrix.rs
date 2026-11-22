#![forbid(unsafe_code)]

//! QR symbol layout: function patterns, data placement, masking, format and
//! version information, and the ISO/IEC 18004 penalty score.
//!
//! Coordinates are `(x, y)` with `x` the column and `y` the row, both starting
//! at the top-left module of the symbol (the quiet zone is not part of the
//! matrix).

use super::EccLevel;

/// Lowest supported version.
pub(crate) const MIN_VERSION: u8 = 1;
/// Highest supported version (37 x 37 modules).
pub(crate) const MAX_VERSION: u8 = 10;

/// Reed-Solomon block layout for one version / error correction level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockSpec {
    pub(crate) ec_per_block: u8,
    pub(crate) group1_blocks: u8,
    pub(crate) group1_data: u8,
    pub(crate) group2_blocks: u8,
    pub(crate) group2_data: u8,
}

impl BlockSpec {
    /// Total number of data codewords.
    pub(crate) fn data_codewords(self) -> usize {
        usize::from(self.group1_blocks) * usize::from(self.group1_data)
            + usize::from(self.group2_blocks) * usize::from(self.group2_data)
    }

    /// Total number of error correction codewords.
    pub(crate) fn ec_codewords(self) -> usize {
        usize::from(self.group1_blocks + self.group2_blocks) * usize::from(self.ec_per_block)
    }

    /// Total number of codewords (data + error correction).
    pub(crate) fn total_codewords(self) -> usize {
        self.data_codewords() + self.ec_codewords()
    }
}

/// Number of codewords and block split for versions 1..=10, indexed by
/// `[version - 1][L, M, Q, H]`.
///
/// Source: Thonky, "Error Correction Code Words and Block Information"
/// (the same table as ISO/IEC 18004 Table 9).
const BLOCKS: [[BlockSpec; 4]; 10] = [
    [
        BlockSpec {
            ec_per_block: 7,
            group1_blocks: 1,
            group1_data: 19,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 10,
            group1_blocks: 1,
            group1_data: 16,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 13,
            group1_blocks: 1,
            group1_data: 13,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 17,
            group1_blocks: 1,
            group1_data: 9,
            group2_blocks: 0,
            group2_data: 0,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 10,
            group1_blocks: 1,
            group1_data: 34,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 16,
            group1_blocks: 1,
            group1_data: 28,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 22,
            group1_blocks: 1,
            group1_data: 22,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 28,
            group1_blocks: 1,
            group1_data: 16,
            group2_blocks: 0,
            group2_data: 0,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 15,
            group1_blocks: 1,
            group1_data: 55,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 26,
            group1_blocks: 1,
            group1_data: 44,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 18,
            group1_blocks: 2,
            group1_data: 17,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 22,
            group1_blocks: 2,
            group1_data: 13,
            group2_blocks: 0,
            group2_data: 0,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 20,
            group1_blocks: 1,
            group1_data: 80,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 18,
            group1_blocks: 2,
            group1_data: 32,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 26,
            group1_blocks: 2,
            group1_data: 24,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 16,
            group1_blocks: 4,
            group1_data: 9,
            group2_blocks: 0,
            group2_data: 0,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 26,
            group1_blocks: 1,
            group1_data: 108,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 24,
            group1_blocks: 2,
            group1_data: 43,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 18,
            group1_blocks: 2,
            group1_data: 15,
            group2_blocks: 2,
            group2_data: 16,
        },
        BlockSpec {
            ec_per_block: 22,
            group1_blocks: 2,
            group1_data: 11,
            group2_blocks: 2,
            group2_data: 12,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 18,
            group1_blocks: 2,
            group1_data: 68,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 16,
            group1_blocks: 4,
            group1_data: 27,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 24,
            group1_blocks: 4,
            group1_data: 19,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 28,
            group1_blocks: 4,
            group1_data: 15,
            group2_blocks: 0,
            group2_data: 0,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 20,
            group1_blocks: 2,
            group1_data: 78,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 18,
            group1_blocks: 4,
            group1_data: 31,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 18,
            group1_blocks: 2,
            group1_data: 14,
            group2_blocks: 4,
            group2_data: 15,
        },
        BlockSpec {
            ec_per_block: 26,
            group1_blocks: 4,
            group1_data: 13,
            group2_blocks: 1,
            group2_data: 14,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 24,
            group1_blocks: 2,
            group1_data: 97,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 22,
            group1_blocks: 2,
            group1_data: 38,
            group2_blocks: 2,
            group2_data: 39,
        },
        BlockSpec {
            ec_per_block: 22,
            group1_blocks: 4,
            group1_data: 18,
            group2_blocks: 2,
            group2_data: 19,
        },
        BlockSpec {
            ec_per_block: 26,
            group1_blocks: 4,
            group1_data: 14,
            group2_blocks: 2,
            group2_data: 15,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 30,
            group1_blocks: 2,
            group1_data: 116,
            group2_blocks: 0,
            group2_data: 0,
        },
        BlockSpec {
            ec_per_block: 22,
            group1_blocks: 3,
            group1_data: 36,
            group2_blocks: 2,
            group2_data: 37,
        },
        BlockSpec {
            ec_per_block: 20,
            group1_blocks: 4,
            group1_data: 16,
            group2_blocks: 4,
            group2_data: 17,
        },
        BlockSpec {
            ec_per_block: 24,
            group1_blocks: 4,
            group1_data: 12,
            group2_blocks: 4,
            group2_data: 13,
        },
    ],
    [
        BlockSpec {
            ec_per_block: 18,
            group1_blocks: 2,
            group1_data: 68,
            group2_blocks: 2,
            group2_data: 69,
        },
        BlockSpec {
            ec_per_block: 26,
            group1_blocks: 4,
            group1_data: 43,
            group2_blocks: 1,
            group2_data: 44,
        },
        BlockSpec {
            ec_per_block: 24,
            group1_blocks: 6,
            group1_data: 19,
            group2_blocks: 2,
            group2_data: 20,
        },
        BlockSpec {
            ec_per_block: 28,
            group1_blocks: 6,
            group1_data: 15,
            group2_blocks: 2,
            group2_data: 16,
        },
    ],
];

/// Alignment pattern centre coordinates per version (version 1 has none).
const ALIGNMENT: [&[u8]; 10] = [
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
];

/// Number of modules in a symbol edge for `version`.
pub(crate) fn size_for(version: u8) -> usize {
    17 + 4 * usize::from(version)
}

/// Block layout for a version and ECC level, or `None` for unsupported versions.
pub(crate) fn block_spec(version: u8, ecc: EccLevel) -> Option<BlockSpec> {
    if !(MIN_VERSION..=MAX_VERSION).contains(&version) {
        return None;
    }
    let index = usize::from(version - MIN_VERSION);
    let level = match ecc {
        EccLevel::L => 0,
        EccLevel::M => 1,
        EccLevel::Q => 2,
        EccLevel::H => 3,
    };
    BLOCKS.get(index).and_then(|row| row.get(level)).copied()
}

/// Number of data codewords available for a version / ECC level.
pub(crate) fn data_codewords(version: u8, ecc: EccLevel) -> Option<usize> {
    block_spec(version, ecc).map(BlockSpec::data_codewords)
}

/// Number of unused "remainder" bits at the end of the data area.
pub(crate) fn remainder_bits(version: u8) -> usize {
    match version {
        1 | 7..=10 => 0,
        2..=6 => 7,
        _ => 0,
    }
}

/// Immutable-ish QR symbol matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Matrix {
    size: usize,
    modules: Vec<bool>,
    function: Vec<bool>,
}

impl Matrix {
    fn new(version: u8) -> Self {
        let size = size_for(version);
        Self {
            size,
            modules: vec![false; size * size],
            function: vec![false; size * size],
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn modules(&self) -> &[bool] {
        &self.modules
    }

    pub(crate) fn get(&self, x: usize, y: usize) -> bool {
        if x >= self.size || y >= self.size {
            return false;
        }
        self.modules
            .get(y * self.size + x)
            .copied()
            .unwrap_or(false)
    }

    fn set(&mut self, x: usize, y: usize, dark: bool) {
        if x < self.size && y < self.size {
            if let Some(slot) = self.modules.get_mut(y * self.size + x) {
                *slot = dark;
            }
        }
    }

    fn set_function(&mut self, x: usize, y: usize, dark: bool) {
        if x < self.size && y < self.size {
            self.set(x, y, dark);
            if let Some(slot) = self.function.get_mut(y * self.size + x) {
                *slot = true;
            }
        }
    }

    pub(crate) fn is_function(&self, x: usize, y: usize) -> bool {
        if x >= self.size || y >= self.size {
            return true;
        }
        self.function
            .get(y * self.size + x)
            .copied()
            .unwrap_or(true)
    }

    /// Renders the matrix as `size` strings of `0`/`1` characters (row major).
    #[cfg(test)]
    pub(crate) fn to_rows(&self) -> Vec<String> {
        (0..self.size)
            .map(|y| {
                (0..self.size)
                    .map(|x| if self.get(x, y) { '1' } else { '0' })
                    .collect()
            })
            .collect()
    }
}

/// Builds the symbol for a specific mask pattern.
///
/// `codewords` must be the final, interleaved data + error correction
/// codewords. A shorter slice leaves the remaining data modules light; longer
/// slices are ignored, so the function never panics.
pub(crate) fn build(version: u8, ecc: EccLevel, mask: u8, codewords: &[u8]) -> Matrix {
    let mut matrix = Matrix::new(version);
    place_function_patterns(&mut matrix, version);
    place_data(&mut matrix, version, codewords);
    apply_mask(&mut matrix, mask);
    write_format_info(&mut matrix, ecc, mask);
    matrix
}

/// Builds the symbol with the lowest-penalty mask, returning the mask used.
pub(crate) fn build_best(version: u8, ecc: EccLevel, codewords: &[u8]) -> (Matrix, u8) {
    let mut best: Option<(u32, u8, Matrix)> = None;
    for mask in 0..8u8 {
        let candidate = build(version, ecc, mask, codewords);
        let score = penalty(&candidate);
        let replace = match &best {
            None => true,
            Some((best_score, _, _)) => score < *best_score,
        };
        if replace {
            best = Some((score, mask, candidate));
        }
    }
    match best {
        Some((_, mask, matrix)) => (matrix, mask),
        // Unreachable: the loop always runs at least once.
        None => (build(version, ecc, 0, codewords), 0),
    }
}

fn place_function_patterns(matrix: &mut Matrix, version: u8) {
    let size = matrix.size;

    // Timing patterns first; the finders overwrite their ends.
    for i in 0..size {
        matrix.set_function(6, i, i % 2 == 0);
        matrix.set_function(i, 6, i % 2 == 0);
    }

    draw_finder(matrix, 3, 3);
    draw_finder(matrix, size.saturating_sub(4), 3);
    draw_finder(matrix, 3, size.saturating_sub(4));

    let positions = ALIGNMENT
        .get(usize::from(version.saturating_sub(1)))
        .copied()
        .unwrap_or(&[]);
    let last = size.saturating_sub(7);
    for &cy in positions {
        for &cx in positions {
            let overlaps_finder = (cx == 6 && cy == 6)
                || (cx == 6 && usize::from(cy) == last)
                || (usize::from(cx) == last && cy == 6);
            if !overlaps_finder {
                draw_alignment(matrix, usize::from(cx), usize::from(cy));
            }
        }
    }

    reserve_format_areas(matrix);
    // The dark module is always set and always a function module.
    matrix.set_function(8, size.saturating_sub(8), true);

    if version >= 7 {
        let bits = super::gf::version_bits(version);
        for i in 0..18usize {
            let dark = (bits >> i) & 1 == 1;
            let a = size - 11 + i % 3;
            let b = i / 3;
            matrix.set_function(a, b, dark);
            matrix.set_function(b, a, dark);
        }
    }
}

fn draw_finder(matrix: &mut Matrix, cx: usize, cy: usize) {
    let size = matrix.size;
    for dy in -4i32..=4 {
        for dx in -4i32..=4 {
            let x = cx as i32 + dx;
            let y = cy as i32 + dy;
            if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
                continue;
            }
            let distance = dx.abs().max(dy.abs());
            let dark = distance != 2 && distance != 4;
            matrix.set_function(x as usize, y as usize, dark);
        }
    }
}

fn draw_alignment(matrix: &mut Matrix, cx: usize, cy: usize) {
    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            let x = cx as i32 + dx;
            let y = cy as i32 + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let dark = dx.abs().max(dy.abs()) != 1;
            matrix.set_function(x as usize, y as usize, dark);
        }
    }
}

/// Marks every module occupied by format information (and the dark module
/// column/row) as a function module so data placement skips it.
fn reserve_format_areas(matrix: &mut Matrix) {
    let size = matrix.size;
    for i in 0..=8 {
        // The timing patterns cross the format area at (8, 6) and (6, 8);
        // those two modules must keep their timing value.
        if i != 6 {
            matrix.set_function(8, i, false);
            matrix.set_function(i, 8, false);
        }
    }
    for i in 0..8 {
        matrix.set_function(size.saturating_sub(1 + i), 8, false);
    }
    for i in 8..15 {
        matrix.set_function(8, size.saturating_sub(15 - i), false);
    }
}

/// Writes the two copies of the 15-bit format information.
fn write_format_info(matrix: &mut Matrix, ecc: EccLevel, mask: u8) {
    let size = matrix.size;
    let bits = super::gf::format_bits(ecc.format_bits(), mask);

    for i in 0..=5 {
        matrix.set_function(8, i, (bits >> i) & 1 == 1);
    }
    matrix.set_function(8, 7, (bits >> 6) & 1 == 1);
    matrix.set_function(8, 8, (bits >> 7) & 1 == 1);
    matrix.set_function(7, 8, (bits >> 8) & 1 == 1);
    for i in 9..15 {
        matrix.set_function(14 - i, 8, (bits >> i) & 1 == 1);
    }

    // Second copy along the bottom-left / top-right edge.
    for i in 0..8 {
        matrix.set_function(size.saturating_sub(1 + i), 8, (bits >> i) & 1 == 1);
    }
    for i in 8..15 {
        matrix.set_function(8, size.saturating_sub(15 - i), (bits >> i) & 1 == 1);
    }
}

fn place_data(matrix: &mut Matrix, version: u8, codewords: &[u8]) {
    let size = matrix.size as i32;
    let total_bits = codewords.len() * 8 + remainder_bits(version);
    let mut bit_index = 0usize;
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
                if x < 0 || y < 0 {
                    continue;
                }
                let (xu, yu) = (x as usize, y as usize);
                if matrix.is_function(xu, yu) {
                    continue;
                }
                let dark = if bit_index < total_bits {
                    bit_at(codewords, bit_index)
                } else {
                    false
                };
                matrix.set(xu, yu, dark);
                bit_index += 1;
            }
        }
        right -= 2;
    }
}

fn bit_at(codewords: &[u8], index: usize) -> bool {
    let byte = index / 8;
    match codewords.get(byte) {
        Some(value) => (value >> (7 - index % 8)) & 1 == 1,
        None => false,
    }
}

/// The ISO/IEC 18004 mask predicate for `mask` (0..=7).
#[allow(clippy::manual_is_multiple_of)] // ISO mask formulas are kept literal for auditability
pub(crate) fn mask_condition(mask: u8, x: usize, y: usize) -> bool {
    let product = x * y;
    match mask & 7 {
        0 => (x + y) % 2 == 0,
        1 => y % 2 == 0,
        2 => x % 3 == 0,
        3 => (x + y) % 3 == 0,
        4 => (y / 2 + x / 3) % 2 == 0,
        5 => product % 2 + product % 3 == 0,
        6 => (product % 2 + product % 3) % 2 == 0,
        _ => ((x + y) % 2 + product % 3) % 2 == 0,
    }
}

fn apply_mask(matrix: &mut Matrix, mask: u8) {
    let size = matrix.size;
    for y in 0..size {
        for x in 0..size {
            if !matrix.is_function(x, y) && mask_condition(mask, x, y) {
                let value = matrix.get(x, y);
                matrix.set(x, y, !value);
            }
        }
    }
}

/// The four penalty rules of ISO/IEC 18004 section 8.8.2.
pub(crate) fn penalty(matrix: &Matrix) -> u32 {
    let size = matrix.size;
    let mut score = 0u32;

    // Rule 1: runs of five or more same-colour modules in a row or column.
    for y in 0..size {
        let mut run = 1u32;
        for x in 1..size {
            if matrix.get(x, y) == matrix.get(x - 1, y) {
                run += 1;
            } else {
                if run >= 5 {
                    score += 3 + (run - 5);
                }
                run = 1;
            }
        }
        if run >= 5 {
            score += 3 + (run - 5);
        }
    }
    for x in 0..size {
        let mut run = 1u32;
        for y in 1..size {
            if matrix.get(x, y) == matrix.get(x, y - 1) {
                run += 1;
            } else {
                if run >= 5 {
                    score += 3 + (run - 5);
                }
                run = 1;
            }
        }
        if run >= 5 {
            score += 3 + (run - 5);
        }
    }

    // Rule 2: 2x2 blocks of a single colour.
    for y in 0..size.saturating_sub(1) {
        for x in 0..size.saturating_sub(1) {
            let value = matrix.get(x, y);
            if value == matrix.get(x + 1, y)
                && value == matrix.get(x, y + 1)
                && value == matrix.get(x + 1, y + 1)
            {
                score += 3;
            }
        }
    }

    // Rule 3: finder-like 1:1:3:1:1 patterns with four light modules on one side.
    const PATTERN: u16 = 0b10111010000;
    const PATTERN_REVERSED: u16 = 0b00001011101;
    for y in 0..size {
        let mut window = 0u16;
        for x in 0..size {
            window = ((window << 1) | u16::from(matrix.get(x, y))) & 0x7FF;
            if x >= 10 && (window == PATTERN || window == PATTERN_REVERSED) {
                score += 40;
            }
        }
    }
    for x in 0..size {
        let mut window = 0u16;
        for y in 0..size {
            window = ((window << 1) | u16::from(matrix.get(x, y))) & 0x7FF;
            if y >= 10 && (window == PATTERN || window == PATTERN_REVERSED) {
                score += 40;
            }
        }
    }

    // Rule 4: deviation of the dark-module proportion from 50%.
    let total = (size * size) as u32;
    let mut dark = 0u32;
    for y in 0..size {
        for x in 0..size {
            if matrix.get(x, y) {
                dark += 1;
            }
        }
    }
    let percent = dark * 100 / total.max(1);
    let deviation = (percent as i32 - 50).unsigned_abs();
    score += (deviation / 5) * 10;

    score
}
