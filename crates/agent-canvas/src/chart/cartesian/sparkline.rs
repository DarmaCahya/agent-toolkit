use crate::canvas::{Color, Fill, Path, Point, Stroke, SvgCanvas};
use crate::error::Result;
use crate::math::LinearScale;

/// Ultra-compact, frameless mini-trend line for embedding in tables or KPI metric cards.
#[derive(Debug, Clone)]
pub struct Sparkline {
    pub values: Vec<f32>,
    pub width: f32,
    pub height: f32,
    pub color: Color,
    pub area_fill: bool,
}

impl Sparkline {
    pub fn new(values: &[f32]) -> Self {
        Self {
            values: values.to_vec(),
            width: 120.0,
            height: 32.0,
            color: Color::from_hex("#2563EB").unwrap(),
            area_fill: true,
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && width > 10.0 {
            self.width = width;
        }
        if height.is_finite() && height > 5.0 {
            self.height = height;
        }
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_fill(mut self, fill: bool) -> Self {
        self.area_fill = fill;
        self
    }

    pub fn to_svg(&self) -> Result<String> {
        let mut canvas = SvgCanvas::new(self.width, self.height)
            .with_view_box(0.0, 0.0, self.width, self.height);

        if self.values.len() < 2 {
            return Ok(canvas.to_svg());
        }

        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in &self.values {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min >= max {
            max = min + 1.0;
        }

        let pad_x = 4.0;
        let pad_y = 3.0;
        let y_scale = LinearScale::new(min, max, self.height - pad_y, pad_y);
        let x_step = (self.width - pad_x * 2.0) / ((self.values.len() - 1) as f32).max(1.0);

        let mut points: Vec<Point> = Vec::new();
        for (i, &v) in self.values.iter().enumerate() {
            if v.is_finite() {
                let x = pad_x + i as f32 * x_step;
                let y = y_scale.map(v);
                points.push(Point::new(x, y));
            }
        }

        if points.len() < 2 {
            return Ok(canvas.to_svg());
        }

        // Draw area fill
        if self.area_fill {
            let mut area = Path::new();
            area = area.move_to(Point::new(points[0].x, self.height));
            for p in &points {
                area = area.line_to(*p);
            }
            area = area.line_to(Point::new(points.last().unwrap().x, self.height));
            area = area.close();
            canvas.draw_path(&area, None, Some(Fill::new(self.color, 0.15)));
        }

        // Draw line
        let mut line = Path::new();
        line = line.move_to(points[0]);
        for p in &points[1..] {
            line = line.line_to(*p);
        }
        canvas.draw_path(&line, Some(Stroke::new(self.color, 2.0)), None);

        // Highlight latest point
        if let Some(&last_p) = points.last() {
            canvas.draw_circle(
                last_p,
                2.5,
                Some(Stroke::new(Color::from_rgb(255, 255, 255), 1.0)),
                Some(Fill::solid(self.color)),
            );
        }

        Ok(canvas.to_svg())
    }
}
