#![forbid(unsafe_code)]

//! GF(2^8) arithmetic, Reed-Solomon generator polynomials and BCH codes used
//! by QR Code error correction.
//!
//! The field is GF(2^8) modulo the primitive polynomial
//! `x^8 + x^4 + x^3 + x^2 + 1` (`0x11D`) with `α = 2`, as required by
//! ISO/IEC 18004.

/// Low eight bits of the primitive polynomial `0x11D`.
const PRIMITIVE_LOW: u8 = 0x1D;

struct Tables {
    exp: [u8; 255],
    log: [u8; 256],
}

const fn build_tables() -> Tables {
    let mut exp = [0u8; 255];
    let mut log = [0u8; 256];
    let mut x: u8 = 1;
    let mut i = 0usize;
    while i < 255 {
        exp[i] = x;
        log[x as usize] = i as u8;
        let carry = x & 0x80 != 0;
        x <<= 1;
        if carry {
            x ^= PRIMITIVE_LOW;
        }
        i += 1;
    }
    Tables { exp, log }
}

const TABLES: Tables = build_tables();

/// Multiplies two field elements.
pub(crate) fn mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let sum = TABLES.log[a as usize] as usize + TABLES.log[b as usize] as usize;
    TABLES.exp[sum % 255]
}

/// The Reed-Solomon generator polynomial of the given degree, highest power
/// first with a leading coefficient of 1.
pub(crate) fn generator(degree: usize) -> Vec<u8> {
    let mut poly = vec![1u8];
    for &root in TABLES.exp.iter().take(degree) {
        let mut next = vec![0u8; poly.len() + 1];
        for (j, &coefficient) in poly.iter().enumerate() {
            next[j] ^= coefficient;
            next[j + 1] ^= mul(coefficient, root);
        }
        poly = next;
    }
    poly
}

/// Remainder of `data * x^degree` divided by the generator polynomial, i.e.
/// the `degree` Reed-Solomon error correction codewords for `data`.
pub(crate) fn rs_remainder(data: &[u8], degree: usize) -> Vec<u8> {
    if degree == 0 {
        return Vec::new();
    }
    let poly = generator(degree);
    let mut remainder = vec![0u8; degree];
    for &byte in data {
        let factor = byte ^ remainder[0];
        remainder.copy_within(1.., 0);
        // `degree >= 1`, so the last slot always exists.
        if let Some(last) = remainder.last_mut() {
            *last = 0;
        }
        for (slot, &coefficient) in remainder.iter_mut().zip(poly.iter().skip(1)) {
            *slot ^= mul(coefficient, factor);
        }
    }
    remainder
}

/// BCH(15,5) format information: error correction level bits and mask number,
/// with the mandatory XOR mask `0x5412` applied.
pub(crate) fn format_bits(ecc_format_bits: u8, mask: u8) -> u16 {
    let data = (u32::from(ecc_format_bits & 0b11) << 3) | u32::from(mask & 0b111);
    let mut remainder = data;
    for _ in 0..10 {
        remainder = (remainder << 1) ^ (((remainder >> 9) & 1) * 0x537);
    }
    let bits = (data << 10) | (remainder & 0x3FF);
    (bits ^ 0x5412) as u16
}

/// BCH(18,6) version information for versions 7 and above.
pub(crate) fn version_bits(version: u8) -> u32 {
    let data = u32::from(version);
    let mut remainder = data;
    for _ in 0..12 {
        remainder = (remainder << 1) ^ (((remainder >> 11) & 1) * 0x1F25);
    }
    (data << 12) | (remainder & 0xFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_multiplication_matches_powers_of_alpha() {
        assert_eq!(mul(1, 1), 1);
        assert_eq!(mul(0, 42), 0);
        assert_eq!(mul(42, 0), 0);
        // α^8 = 0x1D because x^8 = x^4 + x^3 + x^2 + 1 in the QR field.
        assert_eq!(mul(0x80, 0x02), 0x1D);
        // Commutativity and inverse spot checks over the whole field.
        for a in 0..=255u8 {
            for b in [0u8, 1, 2, 3, 17, 128, 255] {
                assert_eq!(mul(a, b), mul(b, a));
            }
        }
    }

    #[test]
    fn generator_polynomial_matches_published_1m_coefficients() {
        // Thonky, "Error Correction Coding": the degree-10 generator for a
        // 1-M symbol has these α exponents.
        let expected_exponents = [0u8, 251, 67, 46, 61, 118, 70, 64, 94, 32, 45];
        let poly = generator(10);
        assert_eq!(poly.len(), 11);
        for (coefficient, &exponent) in poly.iter().zip(expected_exponents.iter()) {
            assert_eq!(*coefficient, TABLES.exp[exponent as usize]);
        }
    }

    #[test]
    fn rs_remainder_matches_published_hello_world_1m_codewords() {
        // Thonky, "Error Correction Coding": data codewords and the resulting
        // ten EC codewords for "HELLO WORLD" as a 1-M symbol.
        let data = [
            32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
        ];
        assert_eq!(
            rs_remainder(&data, 10),
            vec![196, 35, 39, 119, 235, 215, 231, 226, 93, 23]
        );
    }

    #[test]
    fn published_format_strings() {
        // Thonky, "Format and Version String Tables" (15-bit strings).
        let table: [(char, u8, u16); 32] = [
            ('L', 0, 0b111011111000100),
            ('L', 1, 0b111001011110011),
            ('L', 2, 0b111110110101010),
            ('L', 3, 0b111100010011101),
            ('L', 4, 0b110011000101111),
            ('L', 5, 0b110001100011000),
            ('L', 6, 0b110110001000001),
            ('L', 7, 0b110100101110110),
            ('M', 0, 0b101010000010010),
            ('M', 1, 0b101000100100101),
            ('M', 2, 0b101111001111100),
            ('M', 3, 0b101101101001011),
            ('M', 4, 0b100010111111001),
            ('M', 5, 0b100000011001110),
            ('M', 6, 0b100111110010111),
            ('M', 7, 0b100101010100000),
            ('Q', 0, 0b011010101011111),
            ('Q', 1, 0b011000001101000),
            ('Q', 2, 0b011111100110001),
            ('Q', 3, 0b011101000000110),
            ('Q', 4, 0b010010010110100),
            ('Q', 5, 0b010000110000011),
            ('Q', 6, 0b010111011011010),
            ('Q', 7, 0b010101111101101),
            ('H', 0, 0b001011010001001),
            ('H', 1, 0b001001110111110),
            ('H', 2, 0b001110011100111),
            ('H', 3, 0b001100111010000),
            ('H', 4, 0b000011101100010),
            ('H', 5, 0b000001001010101),
            ('H', 6, 0b000110100001100),
            ('H', 7, 0b000100000111011),
        ];
        for (label, mask, bits) in table {
            let format_bits_value = match label {
                'L' => 0b01,
                'M' => 0b00,
                'Q' => 0b11,
                _ => 0b10,
            };
            assert_eq!(
                format_bits(format_bits_value, mask),
                bits,
                "format info for {label}/{mask}"
            );
        }
    }

    #[test]
    fn published_version_information_strings() {
        // Thonky, "Format and Version String Tables".
        let table = [
            (7u8, 0b000111110010010100u32),
            (8, 0b001000010110111100),
            (9, 0b001001101010011001),
            (10, 0b001010010011010011),
        ];
        for (version, bits) in table {
            assert_eq!(version_bits(version), bits, "version info for {version}");
        }
        // Versions below 7 still expose a well-formed (unused) BCH value.
        assert_eq!(version_bits(7) & 0xFFF, 0b110010010100);
    }
}
