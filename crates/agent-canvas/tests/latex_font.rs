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
