use super::plot::CartesianPlot;
use crate::canvas::{Color, Fill, Palette, Point, Rect, Size, SvgCanvas, ThemeMode};
use crate::error::Result;
use crate::math::{CategoryScale, LinearScale};

#[derive(Debug, Clone)]
pub struct BarSeries {
    pub name: String,
    pub values: Vec<f32>,
    pub color: Option<Color>,
}

impl BarSeries {
    pub fn new(name: impl Into<String>, values: &[f32]) -> Self {
        Self {
            name: name.into(),
            values: values.to_vec(),
            color: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Renders this bar series onto the canvas.
    /// Handles single or grouped bars automatically using `series_idx` and `total_series`.
    #[allow(clippy::too_many_arguments)]
    pub fn render_on(
        &self,
        canvas: &mut SvgCanvas,
        _plot_area: &Rect,
        cat_scale: &CategoryScale,
        y_scale: &LinearScale,
        series_idx: usize,
        total_series: usize,
        fallback_color: Color,
    ) {
        let bar_color = self.color.unwrap_or(fallback_color);
        let band_w = cat_scale.bandwidth();
        let bar_w = band_w / (total_series as f32).max(1.0);
        let zero_y = y_scale.map(0.0);

        for (cat_idx, &val) in self.values.iter().enumerate() {
            if !val.is_finite() {
                continue;
            }
            let slot_center = cat_scale.map_center(cat_idx);
            let slot_left = slot_center - band_w / 2.0;
            let bar_left = slot_left + (series_idx as f32) * bar_w;

            let val_y = y_scale.map(val);
            let top_y = val_y.min(zero_y);
            let height = (val_y - zero_y).abs();

            if height > 0.0 && bar_w > 0.0 {
                // Top rounded corners rx=3.0, ry=3.0
                let rect = Rect::new(Point::new(bar_left, top_y), Size::new(bar_w * 0.9, height))
                    .with_radius(3.0, 3.0);
                canvas.draw_rect(rect, None, Some(Fill::new(bar_color, 0.95)));
            }
        }
    }
}

/// Shorthand BarChart builder for rapid generation by AI agents.
#[derive(Debug, Clone)]
pub struct BarChart {
    pub title: Option<String>,
    pub categories: Vec<String>,
    pub series: Vec<BarSeries>,
    pub width: f32,
    pub height: f32,
    pub theme: ThemeMode,
}

impl BarChart {
    pub fn new() -> Self {
        Self {
            title: None,
            categories: Vec::new(),
            series: Vec::new(),
            width: 700.0,
            height: 420.0,
            theme: ThemeMode::Light,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn categories(mut self, cats: &[impl AsRef<str>]) -> Self {
        self.categories = cats.iter().map(|s| s.as_ref().to_string()).collect();
        self
    }

    pub fn add_series(mut self, series: BarSeries) -> Self {
        self.series.push(series);
        self
    }

    pub fn with_theme(mut self, mode: ThemeMode) -> Self {
        self.theme = mode;
        self
    }

    pub fn to_svg(&self) -> Result<String> {
        let palette = Palette::new(self.theme);
        let mut canvas = SvgCanvas::new(self.width, self.height)
            .with_view_box(0.0, 0.0, self.width, self.height)
            .with_background(palette.background());

        let mut plot = CartesianPlot::new(self.width, self.height)
            .with_theme(self.theme)
            .x_categories(&self.categories);

        if let Some(ref t) = self.title {
            plot = plot.title(t);
        }

        // Calculate min/max across all series
        let mut min_val = 0.0f32;
        let mut max_val = 0.0f32;
        for s in &self.series {
            for &v in &s.values {
                if v.is_finite() {
                    min_val = min_val.min(v);
                    max_val = max_val.max(v);
                }
            }
        }
        if max_val == min_val {
            max_val = min_val + 10.0;
        }

        let (plot_rect, y_scale, cat_scale) = plot.render_frame(&mut canvas, min_val, max_val)?;

        let total_series = self.series.len();
        for (i, s) in self.series.iter().enumerate() {
            let color = palette.get_categorical(i);
            s.render_on(&mut canvas, &plot_rect, &cat_scale, &y_scale, i, total_series, color);
        }

        Ok(canvas.to_svg())
    }
}

impl Default for BarChart {
    fn default() -> Self {
        Self::new()
    }
}
