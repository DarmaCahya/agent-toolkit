use crate::canvas::{Point, Rect, Size, SvgCanvas, ThemeMode};
use crate::error::Result;
use crate::math::{CategoryScale, LinearScale, TickGenerator};

/// Low-level frame for Cartesian (X, Y) charts.
/// Computes margins, draws axes, gridlines, and tick labels.
#[derive(Debug, Clone)]
pub struct CartesianPlot {
    pub width: f32,
    pub height: f32,
    pub title: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub theme: ThemeMode,
    pub x_categories: Vec<String>,
    pub y_min: Option<f32>,
    pub y_max: Option<f32>,
    pub margin_left: f32,
    pub margin_right: f32,
    pub margin_top: f32,
    pub margin_bottom: f32,
}

impl CartesianPlot {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: if width.is_finite() && width > 50.0 { width } else { 600.0 },
            height: if height.is_finite() && height > 50.0 { height } else { 400.0 },
            title: None,
            x_label: None,
            y_label: None,
            theme: ThemeMode::Light,
            x_categories: Vec::new(),
            y_min: None,
            y_max: None,
            margin_left: 70.0,
            margin_right: 30.0,
            margin_top: 60.0,
            margin_bottom: 50.0,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        self.x_label = Some(label.into());
        self
    }

    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        self.y_label = Some(label.into());
        self
    }

    pub fn with_theme(mut self, mode: ThemeMode) -> Self {
        self.theme = mode;
        self
    }

    pub fn x_categories(mut self, categories: &[impl AsRef<str>]) -> Self {
        self.x_categories = categories.iter().map(|s| s.as_ref().to_string()).collect();
        self
    }

    pub fn y_range(mut self, min: f32, max: f32) -> Self {
        if min.is_finite() && max.is_finite() && min < max {
            self.y_min = Some(min);
            self.y_max = Some(max);
        }
        self
    }

    /// Computes the usable plot area rectangle.
    pub fn plot_area(&self) -> Rect {
        let w = (self.width - self.margin_left - self.margin_right).max(10.0);
        let h = (self.height - self.margin_top - self.margin_bottom).max(10.0);
        Rect::new(Point::new(self.margin_left, self.margin_top), Size::new(w, h))
    }

    /// Renders frame, title, gridlines, and axis labels onto the canvas.
    /// Returns `(plot_area, y_scale, cat_scale)`.
    pub fn render_frame(
        &self,
        canvas: &mut SvgCanvas,
        data_y_min: f32,
        data_y_max: f32,
    ) -> Result<(Rect, LinearScale, CategoryScale)> {
        let palette = crate::canvas::Palette::new(self.theme);
        let plot_rect = self.plot_area();

        // Determine Y domain
        let eff_min = self.y_min.unwrap_or(data_y_min);
        let eff_max = self.y_max.unwrap_or(data_y_max);
        let ticks = TickGenerator::generate_ticks(eff_min, eff_max, 5);
        let tick_min = *ticks.first().unwrap_or(&eff_min);
        let tick_max = *ticks.last().unwrap_or(&eff_max);

        // Y scale maps tick_min to plot_rect bottom, tick_max to plot_rect top
        let y_scale = LinearScale::new(
            tick_min,
            tick_max,
            plot_rect.origin.y + plot_rect.size.height,
            plot_rect.origin.y,
        );

        let cat_scale = CategoryScale::new(
            self.x_categories.clone(),
            plot_rect.origin.x,
            plot_rect.origin.x + plot_rect.size.width,
            0.2,
        );

        // 1. Draw Title
        if let Some(ref t) = self.title {
            canvas.draw_text(
                Point::new(self.margin_left, self.margin_top - 25.0),
                t,
                18.0,
                palette.text_primary(),
                "start",
            );
        }

        // 2. Draw Horizontal Gridlines & Y-axis Ticks
        for tick in &ticks {
            let y_pos = y_scale.map(*tick);
            if y_pos.is_finite() {
                // Subtle gridline
                canvas.draw_line(
                    Point::new(plot_rect.origin.x, y_pos),
                    Point::new(plot_rect.origin.x + plot_rect.size.width, y_pos),
                    crate::canvas::Stroke::new(palette.gridline(), 1.0),
                );

                // Tick label
                let formatted = TickGenerator::format_tick(*tick);
                canvas.draw_text(
                    Point::new(plot_rect.origin.x - 10.0, y_pos + 4.0),
                    &formatted,
                    11.0,
                    palette.text_secondary(),
                    "end",
                );
            }
        }

        // 3. Draw Category Labels on X-axis
        for (i, cat) in self.x_categories.iter().enumerate() {
            let x_pos = cat_scale.map_center(i);
            if x_pos.is_finite() {
                canvas.draw_text(
                    Point::new(x_pos, plot_rect.origin.y + plot_rect.size.height + 20.0),
                    cat,
                    12.0,
                    palette.text_secondary(),
                    "middle",
                );
            }
        }

        // 4. Draw Axis border
        canvas.draw_line(
            Point::new(plot_rect.origin.x, plot_rect.origin.y + plot_rect.size.height),
            Point::new(plot_rect.origin.x + plot_rect.size.width, plot_rect.origin.y + plot_rect.size.height),
            crate::canvas::Stroke::new(palette.border(), 1.5),
        );

        Ok((plot_rect, y_scale, cat_scale))
    }
}
