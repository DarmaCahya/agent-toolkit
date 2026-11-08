use crate::canvas::{Color, Fill, Palette, Path, Point, SvgCanvas, ThemeMode};
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct FunnelStage {
    pub name: String,
    pub value: f32,
    pub color: Option<Color>,
}

impl FunnelStage {
    pub fn new(name: impl Into<String>, value: f32) -> Self {
        Self {
            name: name.into(),
            value,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Sales & Conversion Funnel Chart with automated drop-off calculation.
#[derive(Debug, Clone)]
pub struct FunnelChart {
    pub title: Option<String>,
    pub stages: Vec<FunnelStage>,
    pub width: f32,
    pub height: f32,
    pub theme: ThemeMode,
}

impl FunnelChart {
    pub fn new() -> Self {
        Self {
            title: None,
            stages: Vec::new(),
            width: 700.0,
            height: 480.0,
            theme: ThemeMode::Light,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn add_stage(mut self, name: impl Into<String>, value: f32) -> Self {
        self.stages.push(FunnelStage::new(name, value));
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

        if let Some(ref t) = self.title {
            canvas.draw_text(
                Point::new(40.0, 45.0),
                t,
                18.0,
                palette.text_primary(),
                "start",
            );
        }

        if self.stages.is_empty() {
            return Ok(canvas.to_svg());
        }

        let margin_t = 80.0;
        let margin_b = 40.0;
        let margin_l = 80.0;
        let margin_r = 180.0; // Space for labels & drop-off %

        let usable_h = self.height - margin_t - margin_b;
        let max_w = self.width - margin_l - margin_r;
        let center_x = margin_l + max_w / 2.0;

        let max_val = self
            .stages
            .iter()
            .map(|s| s.value)
            .fold(0.0f32, f32::max)
            .max(1.0);
        let n = self.stages.len();
        let stage_gap = 6.0;
        let stage_h = (usable_h - (n as f32 - 1.0) * stage_gap) / (n as f32).max(1.0);

        for (i, stage) in self.stages.iter().enumerate() {
            let top_y = margin_t + i as f32 * (stage_h + stage_gap);
            let bottom_y = top_y + stage_h;

            let top_val = if stage.value.is_finite() && stage.value > 0.0 {
                stage.value
            } else {
                0.0
            };
            let next_val = if i + 1 < n {
                if self.stages[i + 1].value.is_finite() && self.stages[i + 1].value > 0.0 {
                    self.stages[i + 1].value
                } else {
                    0.0
                }
            } else {
                top_val * 0.75
            };

            let top_half_w = (top_val / max_val * (max_w / 2.0)).clamp(15.0, max_w / 2.0);
            let bot_half_w = (next_val / max_val * (max_w / 2.0)).clamp(15.0, max_w / 2.0);

            let color = stage.color.unwrap_or_else(|| palette.get_categorical(i));

            // Draw trapezoid
            let mut trap = Path::new();
            trap = trap.move_to(Point::new(center_x - top_half_w, top_y));
            trap = trap.line_to(Point::new(center_x + top_half_w, top_y));
            trap = trap.line_to(Point::new(center_x + bot_half_w, bottom_y));
            trap = trap.line_to(Point::new(center_x - bot_half_w, bottom_y));
            trap = trap.close();

            canvas.draw_path(&trap, None, Some(Fill::solid(color)));

            // Text label: placed outside on the left if trapezoid is narrow, or inside if wide enough
            let stage_label = format!("{} ({:.0})", stage.name, stage.value);
            if top_half_w >= 65.0 {
                canvas.draw_text(
                    Point::new(center_x, top_y + stage_h / 2.0 + 4.0),
                    &stage_label,
                    13.0,
                    Color::from_rgb(255, 255, 255),
                    "middle",
                );
            } else {
                canvas.draw_text(
                    Point::new(center_x - top_half_w - 12.0, top_y + stage_h / 2.0 + 4.0),
                    &stage_label,
                    12.0,
                    palette.text_primary(),
                    "end",
                );
            }

            // Conversion percentage from previous stage
            if i > 0 {
                let prev_val = self.stages[i - 1].value;
                let conv_pct = if prev_val > 0.0 {
                    (stage.value / prev_val) * 100.0
                } else {
                    0.0
                };
                let drop_text = format!("{:.1}% conv", conv_pct);
                canvas.draw_text(
                    Point::new(margin_l + max_w + 15.0, top_y + 10.0),
                    &drop_text,
                    12.0,
                    palette.text_secondary(),
                    "start",
                );
            }
        }

        Ok(canvas.to_svg())
    }
}

impl Default for FunnelChart {
    fn default() -> Self {
        Self::new()
    }
}
