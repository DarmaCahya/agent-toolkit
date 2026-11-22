//! Integration tests for the bundled math font asset (task T007).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ttf_parser::{Face, GlyphId};

const FONT: &[u8] = include_bytes!("../assets/fonts/AgentCanvasMath-subset.otf");

fn face() -> Face<'static> {
    Face::parse(FONT, 0).expect("font parses")
}

#[test]
fn size_under_250_kib() {
    assert!(FONT.len() < 250 * 1024, "font is {} bytes", FONT.len());
}

#[test]
fn has_math_table_and_constants() {
    let f = face();
    let math = f.tables().math.expect("MATH table");
    assert!(math.constants.is_some(), "MATH constants");
}

#[test]
fn names_are_renamed() {
    let f = face();
    let mut family = 0;
    for n in f.names() {
        let text = n.to_string().unwrap_or_default();
        if n.name_id == 1 || n.name_id == 16 {
            assert_eq!(text, "AgentCanvas Math");
            family += 1;
        }
        // Attribution is only allowed in copyright (0) and licence (13/14).
        if !matches!(n.name_id, 0 | 13 | 14) {
            assert!(
                !text.contains("STIX"),
                "name id {} mentions STIX: {text}",
                n.name_id
            );
        }
    }
    assert!(family > 0, "family name records present");
}

fn required_chars() -> Vec<char> {
    let mut v: Vec<char> = Vec::new();
    v.extend('\u{1D434}'..='\u{1D467}'); // math italic A-z (h is a hole, see below)
    v.retain(|&c| c != '\u{1D455}');
    v.push('\u{210E}');
    v.extend('0'..='9');
    v.extend(['+', '(', ')', ',', '=', '<', '>', '\u{2212}']);
    v.extend('\u{1D6FC}'..='\u{1D714}'); // italic lowercase Greek
    v.extend('\u{0391}'..='\u{03A9}'); // upright uppercase Greek
    v.retain(|&c| c != '\u{03A2}'); // unassigned
    v.extend(['±', '·', '×', '≤', '≥', '≠', '∞', '√', '∑', '∫', '∏']);
    // Auto-sizing delimiter glyphs for `\left ... \right`: ( ) [ ] { } | are
    // above, these are the double bar and the angle brackets.
    v.extend(['\u{2016}', '\u{27E8}', '\u{27E9}']);
    v
}

#[test]
fn required_code_points_have_glyphs() {
    let f = face();
    for c in required_chars() {
        assert!(
            f.glyph_index(c).is_some(),
            "missing glyph for U+{:04X}",
            c as u32
        );
    }
}

#[test]
fn big_operators_and_radical_have_vertical_variants() {
    let f = face();
    let variants = f.tables().math.unwrap().variants.expect("MathVariants");
    for c in ['√', '∑', '∫', '∏'] {
        let gid = f.glyph_index(c).unwrap();
        let n = variants
            .vertical_constructions
            .get(gid)
            .map_or(0, |cons| cons.variants.into_iter().count());
        assert!(n > 0, "no vertical variants for {c}");
    }
}

/// Characters usable as `\left` / `\right` delimiters.
fn delimiter_chars() -> Vec<char> {
    vec![
        '(', ')', '[', ']', '{', '}', '|', '\u{2016}', '\u{27E8}', '\u{27E9}',
    ]
}

#[test]
fn every_delimiter_glyph_has_a_vertical_construction() {
    let f = face();
    let variants = f.tables().math.unwrap().variants.expect("MathVariants");
    for c in delimiter_chars() {
        let gid = f
            .glyph_index(c)
            .unwrap_or_else(|| panic!("no glyph for U+{:04X}", c as u32));
        let cons = variants
            .vertical_constructions
            .get(gid)
            .unwrap_or_else(|| panic!("no vertical construction for U+{:04X}", c as u32));
        let sizes: Vec<_> = cons.variants.into_iter().collect();
        assert!(
            !sizes.is_empty(),
            "no vertical size variant for U+{:04X}",
            c as u32
        );
        for v in sizes {
            assert!(
                f.glyph_bounding_box(v.variant_glyph).is_some(),
                "variant {} of U+{:04X} has no outline",
                v.variant_glyph.0,
                c as u32
            );
        }
        if let Some(asm) = cons.assembly {
            for p in asm.parts {
                assert!(
                    f.glyph_bounding_box(p.glyph_id).is_some(),
                    "assembly part {} of U+{:04X} is missing",
                    p.glyph_id.0,
                    c as u32
                );
            }
        }
    }
}

#[test]
fn angle_brackets_and_double_bar_grow_tall_enough() {
    let f = face();
    let variants = f.tables().math.unwrap().variants.expect("MathVariants");
    let upem = f64::from(f.units_per_em());
    for c in ['\u{27E8}', '\u{27E9}', '\u{2016}'] {
        let gid = f.glyph_index(c).unwrap();
        let cons = variants.vertical_constructions.get(gid).unwrap();
        // Growth capacity: the tallest variant (or an assembly) must reach at
        // least 1.5 em so it can cover a fraction.
        let tallest = cons
            .variants
            .into_iter()
            .map(|v| f64::from(v.advance_measurement) / upem)
            .fold(0.0_f64, f64::max);
        assert!(
            tallest >= 1.5 || cons.assembly.is_some(),
            "U+{:04X} cannot grow past {tallest:.2} em",
            c as u32
        );
    }
}

#[test]
fn italic_correction_exists() {
    let f = face();
    let info = f.tables().math.unwrap().glyph_info.expect("glyph info");
    let ic = info.italic_corrections.expect("italic corrections");
    let any = required_chars()
        .into_iter()
        .filter_map(|c| f.glyph_index(c))
        .any(|g: GlyphId| ic.get(g).is_some());
    assert!(any, "no italic correction for any required glyph");
}

// ---- Glyph assembly and MathKern data (spec 004, task T003) ----

fn stretchy_chars() -> Vec<char> {
    vec!['√', '(', ')', '[', ']', '{', '}', '|', '∫']
}

#[test]
fn stretchy_glyphs_have_vertical_assembly() {
    let f = face();
    let variants = f.tables().math.unwrap().variants.expect("MathVariants");
    for c in stretchy_chars() {
        let gid = f.glyph_index(c).unwrap_or_else(|| panic!("no glyph {c}"));
        let cons = variants
            .vertical_constructions
            .get(gid)
            .unwrap_or_else(|| panic!("no vertical construction for {c}"));
        let asm = cons
            .assembly
            .unwrap_or_else(|| panic!("no assembly for {c}"));
        assert!(asm.parts.into_iter().count() > 0, "empty assembly for {c}");
    }
}

#[test]
fn assembly_and_variant_glyphs_are_retained() {
    let f = face();
    let variants = f.tables().math.unwrap().variants.expect("MathVariants");
    let count = f.number_of_glyphs();
    for c in stretchy_chars() {
        let gid = f.glyph_index(c).unwrap();
        let cons = variants.vertical_constructions.get(gid).unwrap();
        for v in cons.variants {
            assert!(
                v.variant_glyph.0 < count && f.glyph_bounding_box(v.variant_glyph).is_some(),
                "variant glyph {} of {c} missing",
                v.variant_glyph.0
            );
        }
        for p in cons.assembly.unwrap().parts {
            assert!(
                p.glyph_id.0 < count && f.glyph_bounding_box(p.glyph_id).is_some(),
                "assembly part {} of {c} has no outline",
                p.glyph_id.0
            );
        }
    }
}

#[test]
fn some_retained_glyph_has_corner_kerns() {
    let f = face();
    let infos = f
        .tables()
        .math
        .unwrap()
        .glyph_info
        .expect("glyph info")
        .kern_infos
        .expect("kern infos");
    let (mut top, mut bottom) = (false, false);
    for gid in (0..f.number_of_glyphs()).map(GlyphId) {
        if let Some(k) = infos.get(gid) {
            top |= k.top_right.is_some();
            bottom |= k.bottom_right.is_some();
        }
    }
    assert!(top, "no TopRight kern data retained");
    assert!(bottom, "no BottomRight kern data retained");
}

#[test]
fn subset_stays_small_with_assembly_data() {
    assert!(FONT.len() < 250_000, "font is {} bytes", FONT.len());
}

#[test]
fn licence_and_notices_present() {
    let ofl = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fonts/OFL-1.1.txt"
    ))
    .unwrap();
    assert!(ofl.contains("SIL OPEN FONT LICENSE"));
    let notices = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../THIRD_PARTY_NOTICES.md"
    ))
    .unwrap();
    assert!(notices.contains("STIX Two") && notices.contains("OFL"));
}
