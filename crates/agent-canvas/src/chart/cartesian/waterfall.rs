use super::plot::CartesianPlot;
use crate::canvas::{Color, Fill, Palette, Point, Rect, Size, Stroke, SvgCanvas, ThemeMode};
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct WaterfallItem {
    pub label: String,
    pub value: f32,
    pub is_total: bool,
}

impl WaterfallItem {
    pub fn step(label: impl Into<String>, value: f32) -> Self {
        Self {
            label: label.into(),
            value,
            is_total: false,
        }
    }

    pub fn total(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: 0.0,
            is_total: true,
        }
    }
}

/// Waterfall Chart: Financial bridge explaining revenue -> expenses -> profit.
#[derive(Debug, Clone)]
pub struct WaterfallChart {
    pub title: Option<String>,
    pub items: Vec<WaterfallItem>,
    pub width: f32,
    pub height: f32,
    pub theme: ThemeMode,
}

impl WaterfallChart {
    pub fn new() -> Self {
        Self {
            title: None,
            items: Vec::new(),
            width: 750.0,
            height: 450.0,
            theme: ThemeMode::Light,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn add_item(mut self, item: WaterfallItem) -> Self {
        self.items.push(item);
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

        let categories: Vec<String> = self.items.iter().map(|it| it.label.clone()).collect();
        let mut plot = CartesianPlot::new(self.width, self.height)
            .with_theme(self.theme)
            .x_categories(&categories);

        if let Some(ref t) = self.title {
            plot = plot.title(t);
        }

        // Calculate running balances and min/max
        let mut running = 0.0f32;
        let mut min_val = 0.0f32;
        let mut max_val = 0.0f32;
        let mut steps: Vec<(f32, f32, bool)> = Vec::new(); // (start, end, is_total)

        for item in &self.items {
            if item.is_total {
                steps.push((0.0, running, true));
                min_val = min_val.min(running);
                max_val = max_val.max(running);
            } else {
                let start = running;
                running += item.value;
                steps.push((start, running, false));
                min_val = min_val.min(start).min(running);
                max_val = max_val.max(start).max(running);
            }
        }

        if max_val == min_val {
            max_val = min_val + 10.0;
        }

        let (_plot_rect, y_scale, cat_scale) = plot.render_frame(&mut canvas, min_val, max_val)?;
        let band_w = cat_scale.bandwidth();
        let bar_w = band_w * 0.75;

        // Colors
        let green = Color::from_hex("#10B981").unwrap();
        let red = Color::from_hex("#EF4444").unwrap();
        let blue = Color::from_hex("#2563EB").unwrap();

        for (i, &(start, end, is_total)) in steps.iter().enumerate() {
            let center_x = cat_scale.map_center(i);
            let bar_left = center_x - bar_w / 2.0;

            let start_y = y_scale.map(start);
            let end_y = y_scale.map(end);

            let top_y = start_y.min(end_y);
            let bar_h = (start_y - end_y).abs().max(2.0);

            let color = if is_total {
                blue
            } else if end >= start {
                green
            } else {
                red
            };

            let rect = Rect::new(Point::new(bar_left, top_y), Size::new(bar_w, bar_h)).with_radius(3.0, 3.0);
            canvas.draw_rect(rect, None, Some(Fill::new(color, 0.95)));

            // Draw horizontal connector line to next bar
            if i + 1 < steps.len() {
                let next_center_x = cat_scale.map_center(i + 1);
                let next_left = next_center_x - bar_w / 2.0;
                let right_edge = bar_left + bar_w;
                canvas.draw_line(
                    Point::new(right_edge, end_y),
                    Point::new(next_left, end_y),
                    Stroke::new(palette.border(), 1.0),
                );
            }
        }

        Ok(canvas.to_svg())
    }
}

impl Default for WaterfallChart {
    fn default() -> Self {
        Self::new()
    }
}
