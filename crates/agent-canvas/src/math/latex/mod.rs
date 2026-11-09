//! LaTeX math-mode to vector SVG typesetter (work in progress; see
//! `specs/002-latex-math-typesetter`).
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod atoms;
mod box_model;
mod error;
mod font;
mod lexer;
mod parser;
mod render;

pub use error::MathError;

use atoms::Style;
use box_model::{LBox, Layout};
use font::MathFont;

/// Maximum accepted input length in characters.
pub const MAX_INPUT_CHARS: usize = 8192;
/// Maximum accepted nesting depth.
pub const MAX_DEPTH: usize = 64;

/// Typesets a supported LaTeX math subset into a self-contained, vector-only SVG.
///
/// Layout follows TeX's Appendix G rules (fractions, radicals, scripts, large
/// operators) using the OpenType MATH constants of a bundled, renamed subset of
/// STIX Two Math (OFL-1.1, see `THIRD_PARTY_NOTICES.md`). Glyphs are emitted as
/// `<path>` outlines referenced through `<use>`; no `<text>`, fonts, scripts or
/// external references appear in the output.
///
/// # Supported grammar (v1)
///
/// ```text
/// formula  := item*
/// item     := atom ( '^' arg | '_' arg )*      // at most one of each per atom
/// atom     := LETTER | DIGIT | '+' | '-' | '=' | '<' | '>' | '(' | ')' | ','
///           | symbol-cmd | group | '\frac' group group
///           | '\sqrt' ('[' formula ']')? group
///           | '\sum' | '\int' | '\prod'
/// arg      := single-token-atom | group
/// group    := '{' formula '}'
/// symbol-cmd := \alpha … \omega | \Gamma … \Omega | \pm | \cdot | \times
///             | \leq | \geq | \neq | \infty
/// ```
///
/// Whitespace is ignored. Everything else (for example `\left`, `\text`,
/// matrices, accents, font commands) is rejected with a [`MathError`] that
/// carries the character position and a hint.
///
/// # Limits and guarantees
///
/// * Input is limited to [`MAX_INPUT_CHARS`] characters and nesting to
///   [`MAX_DEPTH`]; both are checked before any layout work.
/// * Never panics on any input; no I/O; no global mutable state.
/// * Deterministic: the same input yields byte-identical output on every call
///   and platform (numbers are rounded to two decimals by integer arithmetic).
///
/// # Examples
///
/// ```
/// use agent_canvas::math::MathTypesetter;
///
/// let svg = MathTypesetter::new().render_to_svg("x^2").unwrap();
/// assert!(svg.starts_with("<svg"));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct MathTypesetter;

impl MathTypesetter {
    /// Creates a typesetter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Renders `latex` to SVG.
    ///
    /// # Errors
    /// Returns a [`MathError`] for empty, over-long, malformed or unsupported
    /// input.
    pub fn render_to_svg(&self, latex: &str) -> Result<String, MathError> {
        let font = MathFont::shared()?;
        let root = layout(font, latex)?;
        render::render(font, &root)
    }

    /// Layout measurements (em) for tests; shares the pipeline of
    /// [`render_to_svg`](Self::render_to_svg). Not part of the stable API.
    ///
    /// # Errors
    /// Same as [`render_to_svg`](Self::render_to_svg).
    #[doc(hidden)]
    pub fn layout_metrics(&self, latex: &str) -> Result<BoxMetrics, MathError> {
        let font = MathFont::shared()?;
        let root = layout(font, latex)?;
        let items = root
            .items
            .iter()
            .map(|item| match *item {
                box_model::Item::Glyph { gid, x, y, scale } => ItemMetrics {
                    kind: ItemKind::Glyph,
                    gid: gid.0,
                    x,
                    y,
                    w: font.advance(gid) * scale,
                    h: 0.0,
                    sx: scale,
                    sy: scale,
                },
                box_model::Item::ScaledGlyph { gid, x, y, sx, sy } => ItemMetrics {
                    kind: ItemKind::ScaledGlyph,
                    gid: gid.0,
                    x,
                    y,
                    w: font.advance(gid) * sx,
                    h: 0.0,
                    sx,
                    sy,
                },
                box_model::Item::Rule { x, y, w, h } => ItemMetrics {
                    kind: ItemKind::Rule,
                    gid: 0,
                    x,
                    y,
                    w,
                    h,
                    sx: 1.0,
                    sy: 1.0,
                },
            })
            .collect();
        Ok(BoxMetrics {
            width: root.width,
            height: root.height,
            depth: root.depth,
            items,
        })
    }

    /// Font facts for tests (em). Not part of the stable API.
    ///
    /// # Errors
    /// `MissingGlyph` when the bundled font lacks `ch`.
    #[doc(hidden)]
    pub fn glyph_facts(&self, ch: char) -> Result<GlyphFacts, MathError> {
        let font = MathFont::shared()?;
        let gid = font.glyph(atoms::math_char(ch))?;
        let c = font.consts();
        Ok(GlyphFacts {
            gid: gid.0,
            advance: font.advance(gid),
            italic_correction: font.italic_correction(gid),
            script_scale: c.script_percent_scale_down / 100.0,
            script_script_scale: c.script_script_percent_scale_down / 100.0,
            axis_height: c.axis_height,
            fraction_rule_thickness: c.fraction_rule_thickness,
            radical_display_vertical_gap: c.radical_display_vertical_gap,
            radical_rule_thickness: c.radical_rule_thickness,
        })
    }
}

/// Kind of a placed item (test hook).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// Uniformly scaled glyph.
    Glyph,
    /// Non-uniformly scaled glyph (stretched radical).
    ScaledGlyph,
    /// Filled rectangle.
    Rule,
}

/// A placed item in em, y up from the baseline (test hook).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemMetrics {
    /// Item kind.
    pub kind: ItemKind,
    /// Glyph id (0 for rules).
    pub gid: u16,
    /// Origin x (rule: left).
    pub x: f64,
    /// Origin y (rule: bottom).
    pub y: f64,
    /// Glyph advance after scaling, or rule width.
    pub w: f64,
    /// Rule height (0 for glyphs).
    pub h: f64,
    /// Horizontal scale (1.0 = 1 em).
    pub sx: f64,
    /// Vertical scale.
    pub sy: f64,
}

/// Overall layout measurements in em (test hook).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct BoxMetrics {
    /// Width.
    pub width: f64,
    /// Height above the baseline.
    pub height: f64,
    /// Depth below the baseline.
    pub depth: f64,
    /// Placed items in layout order.
    pub items: Vec<ItemMetrics>,
}

/// Font facts for one character (test hook).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphFacts {
    /// Glyph id.
    pub gid: u16,
    /// Advance in em.
    pub advance: f64,
    /// Italic correction in em.
    pub italic_correction: f64,
    /// Script size factor.
    pub script_scale: f64,
    /// Scriptscript size factor.
    pub script_script_scale: f64,
    /// Math axis height in em.
    pub axis_height: f64,
    /// Fraction bar thickness in em.
    pub fraction_rule_thickness: f64,
    /// Display-style radical vertical gap in em.
    pub radical_display_vertical_gap: f64,
    /// Radical rule thickness in em.
    pub radical_rule_thickness: f64,
}

/// Shared pipeline: validate, lex, parse, convert to atoms and lay out.
fn layout(font: &MathFont, latex: &str) -> Result<LBox, MathError> {
    let len = latex
        .chars()
        .take(MAX_INPUT_CHARS.saturating_add(1))
        .count();
    if len > MAX_INPUT_CHARS {
        return Err(MathError::InputTooLong {
            len,
            max: MAX_INPUT_CHARS,
        });
    }
    if latex.trim().is_empty() {
        return Err(MathError::EmptyFormula);
    }
    let tokens = lexer::lex(latex)?;
    let ast = parser::parse(&tokens, len)?;
    let atoms = match &ast.kind {
        parser::NodeKind::Group(items) => atoms::build_atoms(items),
        _ => atoms::build_atoms(std::slice::from_ref(&ast)),
    };
    let root = Layout::new(font).layout_list(&atoms, Style::DISPLAY)?;
    root.finite()?;
    Ok(root)
}
