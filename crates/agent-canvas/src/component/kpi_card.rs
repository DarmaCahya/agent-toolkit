#![forbid(unsafe_code)]

//! Executive KPI metric card with a semantic delta badge.
//!
//! The card is pure vector output: title, value, delta and caption are glyph
//! outlines (no `<text>`, no `font-family`), so it renders identically on any
//! viewer. Colours come from the shared [`Palette`].

use super::text::{self, Weight};
use crate::canvas::{Color, Fill, Palette, Rect, SvgCanvas, ThemeMode};
use crate::canvas::{Point, Size, Stroke};

/// Design-space card size; the SVG scales this to the requested pixel size.
const DESIGN_WIDTH: f32 = 320.0;
const DESIGN_HEIGHT: f32 = 160.0;
const MARGIN: f32 = 24.0;
const CORNER_RADIUS: f32 = 14.0;
const BADGE_HEIGHT: f32 = 26.0;
const BADGE_RADIUS: f32 = 13.0;
const BADGE_PADDING: f32 = 10.0;
const BADGE_TEXT_SIZE: f32 = 13.0;

/// Sentiment carried by a delta badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaTrend {
    /// Improvement: green badge.
    Positive,
    /// Regression: red badge.
    Negative,
    /// No meaningful change: neutral badge using the theme's secondary text colour.
    Neutral,
}

impl DeltaTrend {
    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Delta {
    text: String,
    trend: DeltaTrend,
}

/// An executive KPI card: title, big value and an optional delta badge.
///
/// ```
/// use agent_canvas::component::KpiCard;
/// use agent_canvas::canvas::ThemeMode;
///
/// let svg = KpiCard::new("Monthly Revenue", "Rp 1.24B")
///     .with_delta("+12.4%", true)
///     .with_caption("vs. last month")
///     .with_theme(ThemeMode::Dark)
///     .to_svg();
/// assert!(svg.starts_with("<svg"));
/// assert!(!svg.contains("<text"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct KpiCard {
    title: String,
    value: String,
    delta: Option<Delta>,
    caption: Option<String>,
    theme: ThemeMode,
    width: f32,
    height: f32,
}

impl KpiCard {
    /// Creates a card with a title and a value, using the light theme.
    pub fn new(title: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
            delta: None,
            caption: None,
            theme: ThemeMode::Light,
            width: DESIGN_WIDTH,
            height: DESIGN_HEIGHT,
        }
    }

    /// Adds a delta badge. `positive` selects green (`true`) or red (`false`).
    ///
    /// The delta text is rendered verbatim; pass the sign yourself
    /// (for example `"+12.4%"`).
    pub fn with_delta(mut self, delta: impl Into<String>, positive: bool) -> Self {
        let trend = if positive {
            DeltaTrend::Positive
        } else {
            DeltaTrend::Negative
        };
        self.delta = Some(Delta {
            text: delta.into(),
            trend,
        });
        self
    }

    /// Adds a delta badge with an explicit sentiment (including neutral).
    pub fn with_delta_trend(mut self, delta: impl Into<String>, trend: DeltaTrend) -> Self {
        self.delta = Some(Delta {
            text: delta.into(),
            trend,
        });
        self
    }

    /// Adds a neutral delta badge (grey, no implied direction).
    pub fn with_neutral_delta(self, delta: impl Into<String>) -> Self {
        self.with_delta_trend(delta, DeltaTrend::Neutral)
    }

    /// Adds a small caption to the right of the delta badge, e.g. "vs. last month".
    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Selects the light or dark canvas theme.
    pub fn with_theme(mut self, theme: ThemeMode) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the rendered pixel size. Non-finite or non-positive values are ignored.
    ///
    /// The card keeps its 2:1 design proportions through the SVG view box.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
        if height.is_finite() && height > 0.0 {
            self.height = height;
        }
        self
    }

    /// Renders the card as a self-contained SVG document.
    ///
    /// Output is deterministic for identical inputs and contains only
    /// `<rect>`, `<path>` and `<svg>` elements.
    pub fn to_svg(&self) -> String {
        let palette = Palette::new(self.theme);
        let mut canvas = SvgCanvas::new(self.width, self.height)
            .with_view_box(0.0, 0.0, DESIGN_WIDTH, DESIGN_HEIGHT)
            .with_background(palette.background());

        canvas.draw_rect(
            Rect::new(
                Point::new(0.5, 0.5),
                Size::new(DESIGN_WIDTH - 1.0, DESIGN_HEIGHT - 1.0),
            )
            .with_radius(CORNER_RADIUS, CORNER_RADIUS),
            Some(Stroke::new(palette.border(), 1.0)),
            Some(Fill::solid(palette.surface())),
        );

        text::draw(
            &mut canvas,
            &self.title,
            MARGIN,
            38.0,
            13.0,
            Weight::Regular,
            palette.text_secondary(),
        );

        let value_size = self.fit_value_size();
        text::draw(
            &mut canvas,
            &self.value,
            MARGIN,
            88.0,
            value_size,
            Weight::Bold,
            palette.text_primary(),
        );

        let mut caption_x = MARGIN;
        if let Some(delta) = &self.delta {
            if !delta.text.is_empty() {
                let (color, badge_background) = badge_colors(delta.trend, &palette);
                let badge_width =
                    text::measure(&delta.text, BADGE_TEXT_SIZE, Weight::Bold) + BADGE_PADDING * 2.0;
                let badge_top = 104.0;

                canvas.draw_rect(
                    Rect::new(
                        Point::new(MARGIN, badge_top),
                        Size::new(badge_width, BADGE_HEIGHT),
                    )
                    .with_radius(BADGE_RADIUS, BADGE_RADIUS),
                    None,
                    Some(Fill::solid(badge_background)),
                );
                text::draw(
                    &mut canvas,
                    &delta.text,
                    MARGIN + BADGE_PADDING,
                    badge_top + 18.0,
                    BADGE_TEXT_SIZE,
                    Weight::Bold,
                    color,
                );
                caption_x = MARGIN + badge_width + 10.0;
            }
        }

        if let Some(caption) = &self.caption {
            text::draw(
                &mut canvas,
                caption,
                caption_x,
                122.0,
                12.0,
                Weight::Regular,
                palette.text_secondary(),
            );
        }

        canvas.to_svg()
    }

    /// Largest value font size (40 down to 18) that fits the card width.
    fn fit_value_size(&self) -> f32 {
        let available = DESIGN_WIDTH - MARGIN * 2.0;
        let mut size = 40.0f32;
        while size > 18.0 && text::measure(&self.value, size, Weight::Bold) > available {
            size -= 1.0;
        }
        size
    }
}

fn trend_color(trend: DeltaTrend, palette: &Palette) -> Color {
    match trend {
        DeltaTrend::Positive => Color::from_rgb(0x05, 0x96, 0x69),
        DeltaTrend::Negative => Color::from_rgb(0xDC, 0x26, 0x26),
        DeltaTrend::Neutral => palette.text_secondary(),
    }
}

/// Badge text and background for the active theme.
///
/// Dark cards need a lighter text tint: the raw accent on a dark tinted pill
/// drops below a comfortable contrast ratio.
fn badge_colors(trend: DeltaTrend, palette: &Palette) -> (Color, Color) {
    let accent = trend_color(trend, palette);
    match palette.mode {
        ThemeMode::Light => (accent, blend(accent, palette.surface(), 0.12)),
        ThemeMode::Dark => {
            let text = blend(accent, Color::from_rgb(0xFF, 0xFF, 0xFF), 0.55);
            (text, blend(accent, palette.surface(), 0.28))
        }
    }
}

/// Linear blend from `base` toward `accent` by `amount` (0..=1).
fn blend(accent: Color, base: Color, amount: f32) -> Color {
    let t = if amount.is_finite() {
        amount.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mix = |a: u8, b: u8| -> u8 {
        let value = f32::from(a) * t + f32::from(b) * (1.0 - t);
        value.round().clamp(0.0, 255.0) as u8
    };
    Color::from_rgb(
        mix(accent.r, base.r),
        mix(accent.g, base.g),
        mix(accent.b, base.b),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finite_numbers(svg: &str) -> bool {
        let mut rest = svg;
        while let Some(quote) = rest.find('"') {
            rest = &rest[quote + 1..];
            let Some(end) = rest.find('"') else {
                return false;
            };
            let value = &rest[..end];
            rest = &rest[end + 1..];
            if value.starts_with(|c: char| c == '-' || c == '.' || c.is_ascii_digit())
                && value
                    .parse::<f32>()
                    .map(|number| !number.is_finite())
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
    fn renders_light_and_dark_themes_without_text_elements() {
        for theme in [ThemeMode::Light, ThemeMode::Dark] {
            let svg = KpiCard::new("Monthly Revenue", "Rp 1.24B")
                .with_delta("+12.4%", true)
                .with_caption("vs. last month")
                .with_theme(theme)
                .to_svg();
            assert!(svg.starts_with("<svg"));
            assert!(svg.trim_end().ends_with("</svg>"));
            assert!(!svg.contains("<text"));
            assert!(!svg.contains("font-family"));
            assert!(svg.contains("<rect"));
            assert!(svg.contains("<path"));
            assert!(finite_numbers(&svg));
        }
        let light = KpiCard::new("Revenue", "42")
            .with_theme(ThemeMode::Light)
            .to_svg();
        let dark = KpiCard::new("Revenue", "42")
            .with_theme(ThemeMode::Dark)
            .to_svg();
        assert!(light.contains("#FFFFFF"));
        assert!(dark.contains("#0F172A"));
        assert_ne!(light, dark);
    }

    #[test]
    fn delta_colors_follow_sentiment() {
        let positive = KpiCard::new("Conversion", "4.8%")
            .with_delta("+0.6pp", true)
            .to_svg();
        assert!(positive.contains("#059669"), "positive badge must be green");

        let negative = KpiCard::new("Churn", "2.1%")
            .with_delta("+0.4pp", false)
            .to_svg();
        assert!(negative.contains("#DC2626"), "negative badge must be red");

        let neutral = KpiCard::new("Headcount", "128")
            .with_neutral_delta("0.0%")
            .to_svg();
        assert!(!neutral.contains("#059669"));
        assert!(!neutral.contains("#DC2626"));
        assert!(
            neutral.contains("#64748B"),
            "neutral uses secondary text colour"
        );
    }

    #[test]
    fn output_is_deterministic() {
        let card = KpiCard::new("EBITDA", "Rp 2.4B")
            .with_delta("-3.1%", false)
            .with_caption("QoQ");
        let first = card.to_svg();
        for _ in 0..10 {
            assert_eq!(first, card.to_svg());
        }
    }

    #[test]
    fn long_values_shrink_instead_of_overflowing() {
        let card = KpiCard::new("Total", "Rp 1,234,567,890,123.45");
        let size = card.fit_value_size();
        assert!((18.0..40.0).contains(&size));
        assert!(
            text::measure(&card.value, size, Weight::Bold) <= DESIGN_WIDTH - MARGIN * 2.0 + 1.0
        );
    }

    #[test]
    fn invalid_size_is_ignored() {
        let card = KpiCard::new("A", "1").with_size(f32::NAN, -10.0);
        assert_eq!(card.width, DESIGN_WIDTH);
        assert_eq!(card.height, DESIGN_HEIGHT);
        let card = card.with_size(640.0, 320.0);
        let svg = card.to_svg();
        assert!(svg.contains("width=\"640\""));
        assert!(svg.contains("height=\"320\""));
        assert!(svg.contains("viewBox=\"0 0 320 160\""));
    }

    #[test]
    fn empty_delta_and_caption_are_skipped() {
        let svg = KpiCard::new("Title", "Value").with_delta("", true).to_svg();
        assert!(!svg.contains("#059669"));
    }
}
