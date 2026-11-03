use crate::canvas::{Color, Fill, Palette, Point, Rect, Size, Stroke, SvgCanvas, ThemeMode};
use crate::error::Result;
use crate::math::LinearScale;

#[derive(Debug, Clone)]
pub struct QuadrantItem {
    pub label: String,
    pub x: f32, // 0.0 to 100.0
    pub y: f32, // 0.0 to 100.0
    pub color: Option<Color>,
}

impl QuadrantItem {
    pub fn new(label: impl Into<String>, x: f32, y: f32) -> Self {
        let safe_x = if x.is_finite() { x.clamp(0.0, 100.0) } else { 0.0 };
        let safe_y = if y.is_finite() { y.clamp(0.0, 100.0) } else { 0.0 };
        Self {
            label: label.into(),
            x: safe_x,
            y: safe_y,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Executive 2x2 Decision Matrix (BCG, Eisenhower Priority, Risk vs Impact).
#[derive(Debug, Clone)]
pub struct QuadrantMatrix {
    pub title: Option<String>,
    pub x_axis_label: String,
    pub y_axis_label: String,
    pub quadrant_labels: (String, String, String, String), // (TR, TL, BL, BR)
    pub items: Vec<QuadrantItem>,
    pub width: f32,
    pub height: f32,
    pub theme: ThemeMode,
}

impl QuadrantMatrix {
    pub fn new() -> Self {
        Self {
            title: None,
            x_axis_label: "Effort / Complexity".to_string(),
            y_axis_label: "Impact / Value".to_string(),
            quadrant_labels: (
                "Strategic Bets".to_string(), // High Effort, High Impact
                "Quick Wins".to_string(),     // Low Effort, High Impact
                "Fill-ins".to_string(),       // Low Effort, Low Impact
                "Thankless Tasks".to_string(),// High Effort, Low Impact
            ),
            items: Vec::new(),
            width: 700.0,
            height: 550.0,
            theme: ThemeMode::Light,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn x_axis(mut self, label: impl Into<String>) -> Self {
        self.x_axis_label = label.into();
        self
    }

    pub fn y_axis(mut self, label: impl Into<String>) -> Self {
        self.y_axis_label = label.into();
        self
    }

    pub fn quadrant_labels(mut self, tr: &str, tl: &str, bl: &str, br: &str) -> Self {
        self.quadrant_labels = (tr.to_string(), tl.to_string(), bl.to_string(), br.to_string());
        self
    }

    pub fn add_item(mut self, label: impl Into<String>, x: f32, y: f32) -> Self {
        self.items.push(QuadrantItem::new(label, x, y));
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

        // Margins
        let margin_l = 80.0;
        let margin_r = 40.0;
        let margin_t = 60.0;
        let margin_b = 60.0;

        let grid_w = self.width - margin_l - margin_r;
        let grid_h = self.height - margin_t - margin_b;

        let half_w = grid_w / 2.0;
        let half_h = grid_h / 2.0;

        // Title
        if let Some(ref t) = self.title {
            canvas.draw_text(
                Point::new(margin_l, margin_t - 25.0),
                t,
                18.0,
                palette.text_primary(),
                "start",
            );
        }

        // Draw 4 Quadrants Background
        let q_tl = Rect::new(Point::new(margin_l, margin_t), Size::new(half_w, half_h));
        let q_tr = Rect::new(Point::new(margin_l + half_w, margin_t), Size::new(half_w, half_h));
        let q_bl = Rect::new(Point::new(margin_l, margin_t + half_h), Size::new(half_w, half_h));
        let q_br = Rect::new(Point::new(margin_l + half_w, margin_t + half_h), Size::new(half_w, half_h));

        // Subtle quadrant tints
        let tint_green = Color::from_hex("#10B981").unwrap();
        let tint_blue = Color::from_hex("#2563EB").unwrap();
        let tint_slate = Color::from_hex("#64748B").unwrap();
        let tint_amber = Color::from_hex("#F59E0B").unwrap();

        canvas.draw_rect(q_tl, None, Some(Fill::new(tint_green, 0.08)));
        canvas.draw_rect(q_tr, None, Some(Fill::new(tint_blue, 0.08)));
        canvas.draw_rect(q_bl, None, Some(Fill::new(tint_slate, 0.06)));
        canvas.draw_rect(q_br, None, Some(Fill::new(tint_amber, 0.06)));

        // Outer & Cross Axes Lines
        let center_x = margin_l + half_w;
        let center_y = margin_t + half_h;

        // Horizontal cross line
        canvas.draw_line(
            Point::new(margin_l, center_y),
            Point::new(margin_l + grid_w, center_y),
            Stroke::new(palette.border(), 1.5),
        );

        // Vertical cross line
        canvas.draw_line(
            Point::new(center_x, margin_t),
            Point::new(center_x, margin_t + grid_h),
            Stroke::new(palette.border(), 1.5),
        );

        // Outer border
        let outer_rect = Rect::new(Point::new(margin_l, margin_t), Size::new(grid_w, grid_h));
        canvas.draw_rect(outer_rect, Some(Stroke::new(palette.border(), 1.0)), None);

        // Quadrant Title Badges in corners
        let font_q = 12.0;
        canvas.draw_text(Point::new(margin_l + 12.0, margin_t + 20.0), &self.quadrant_labels.1, font_q, palette.text_secondary(), "start"); // TL
        canvas.draw_text(Point::new(margin_l + grid_w - 12.0, margin_t + 20.0), &self.quadrant_labels.0, font_q, palette.text_secondary(), "end"); // TR
        canvas.draw_text(Point::new(margin_l + 12.0, margin_t + grid_h - 12.0), &self.quadrant_labels.2, font_q, palette.text_secondary(), "start"); // BL
        canvas.draw_text(Point::new(margin_l + grid_w - 12.0, margin_t + grid_h - 12.0), &self.quadrant_labels.3, font_q, palette.text_secondary(), "end"); // BR

        // Axis Titles
        canvas.draw_text(
            Point::new(center_x, margin_t + grid_h + 35.0),
            &format!("{} ➔", self.x_axis_label),
            13.0,
            palette.text_primary(),
            "middle",
        );

        canvas.draw_text(
            Point::new(margin_l - 25.0, center_y),
            &format!("▲ {}", self.y_axis_label),
            13.0,
            palette.text_primary(),
            "middle",
        );

        // Scales
        let scale_x = LinearScale::new(0.0, 100.0, margin_l, margin_l + grid_w);
        let scale_y = LinearScale::new(0.0, 100.0, margin_t + grid_h, margin_t);

        // Plot Items
        for (i, item) in self.items.iter().enumerate() {
            let px = scale_x.map(item.x);
            let py = scale_y.map(item.y);

            let color = item.color.unwrap_or_else(|| palette.get_categorical(i));

            // Dot
            canvas.draw_circle(
                Point::new(px, py),
                6.0,
                Some(Stroke::new(Color::from_rgb(255, 255, 255), 1.5)),
                Some(Fill::solid(color)),
            );

            // Item Label: flip anchor to "end" if near right border to prevent clipping
            let (label_x, anchor) = if item.x > 75.0 {
                (px - 10.0, "end")
            } else {
                (px + 10.0, "start")
            };

            canvas.draw_text(
                Point::new(label_x, py + 4.0),
                &item.label,
                12.0,
                palette.text_primary(),
                anchor,
            );
        }

        Ok(canvas.to_svg())
    }
}

impl Default for QuadrantMatrix {
    fn default() -> Self {
        Self::new()
    }
}
