use crate::canvas::{Color, Fill, Palette, Path, Point, Rect, Size, Stroke, SvgCanvas, ThemeMode};
use crate::error::Result;
use crate::math::{calculate_slice_angles, deg_to_rad, polar_to_cartesian};

#[derive(Debug, Clone)]
pub struct DonutSlice {
    pub label: String,
    pub value: f32,
    pub color: Option<Color>,
}

impl DonutSlice {
    pub fn new(label: impl Into<String>, value: f32) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Executive Donut & Pie Chart with center metrics and auto-calculated percentage legend.
#[derive(Debug, Clone)]
pub struct DonutChart {
    pub title: Option<String>,
    pub center_text_primary: Option<String>,
    pub center_text_secondary: Option<String>,
    pub slices: Vec<DonutSlice>,
    pub width: f32,
    pub height: f32,
    pub hole_ratio: f32, // 0.55 for donut, 0.0 for pie
    pub theme: ThemeMode,
}

impl DonutChart {
    pub fn new() -> Self {
        Self {
            title: None,
            center_text_primary: None,
            center_text_secondary: None,
            slices: Vec::new(),
            width: 650.0,
            height: 400.0,
            hole_ratio: 0.58,
            theme: ThemeMode::Light,
        }
    }

    pub fn pie() -> Self {
        let mut d = Self::new();
        d.hole_ratio = 0.0;
        d
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn center_text(mut self, primary: impl Into<String>, secondary: impl Into<String>) -> Self {
        self.center_text_primary = Some(primary.into());
        self.center_text_secondary = Some(secondary.into());
        self
    }

    pub fn add_slice(mut self, label: impl Into<String>, value: f32) -> Self {
        self.slices.push(DonutSlice::new(label, value));
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

        // 1. Title
        if let Some(ref t) = self.title {
            canvas.draw_text(
                Point::new(40.0, 45.0),
                t,
                18.0,
                palette.text_primary(),
                "start",
            );
        }

        // Layout: Ring center on left, Legend on right
        let center_x = self.width * 0.35;
        let center_y = self.height * 0.55;
        let outer_r = (self.height * 0.35).min(center_x - 30.0);
        let inner_r = outer_r * self.hole_ratio;

        let values: Vec<f32> = self.slices.iter().map(|s| if s.value.is_finite() && s.value > 0.0 { s.value } else { 0.0 }).collect();
        let total_val: f32 = values.iter().sum();

        if total_val <= 0.0 || values.is_empty() {
            // Draw empty "No Data" placeholder ring
            let empty_color = Color::from_hex("#94A3B8").unwrap();
            let mut empty_canvas = SvgCanvas::new(self.width, self.height)
                .with_view_box(0.0, 0.0, self.width, self.height)
                .with_background(palette.background());
            empty_canvas.draw_circle(
                Point::new(center_x, center_y),
                outer_r,
                Some(Stroke::new(empty_color, 2.0)),
                None,
            );
            empty_canvas.draw_text(
                Point::new(center_x, center_y + 5.0),
                "No Data",
                14.0,
                palette.text_secondary(),
                "middle",
            );
            return Ok(empty_canvas.to_svg());
        }

        // Start from -90 degrees (12 o clock)
        let start_rad = deg_to_rad(-90.0);
        let angles = calculate_slice_angles(&values, start_rad);

        for (i, &(s_rad, e_rad, pct)) in angles.iter().enumerate() {
            let slice = &self.slices[i];
            let color = slice.color.unwrap_or_else(|| palette.get_categorical(i));

            // To avoid zero-length arc collapse on 100% full circle, split into 2 arcs if delta >= 2pi
            let delta = e_rad - s_rad;
            let is_full_circle = delta >= std::f32::consts::TAU - 0.001;

            if is_full_circle {
                let mid_rad = s_rad + std::f32::consts::PI;
                let (p1_x, p1_y) = polar_to_cartesian(center_x, center_y, outer_r, s_rad);
                let (p2_x, p2_y) = polar_to_cartesian(center_x, center_y, outer_r, mid_rad);

                let mut path = Path::new();
                path = path.move_to(Point::new(p1_x, p1_y));
                path = path.arc_to(outer_r, outer_r, 0.0, false, true, Point::new(p2_x, p2_y));
                path = path.arc_to(outer_r, outer_r, 0.0, false, true, Point::new(p1_x, p1_y));

                if inner_r > 0.0 {
                    let (in1_x, in1_y) = polar_to_cartesian(center_x, center_y, inner_r, s_rad);
                    let (in2_x, in2_y) = polar_to_cartesian(center_x, center_y, inner_r, mid_rad);
                    path = path.move_to(Point::new(in1_x, in1_y));
                    path = path.arc_to(inner_r, inner_r, 0.0, false, false, Point::new(in2_x, in2_y));
                    path = path.arc_to(inner_r, inner_r, 0.0, false, false, Point::new(in1_x, in1_y));
                }
                path = path.close();
                canvas.draw_path(&path, None, Some(Fill::solid(color)));
            } else {
                let (out_start_x, out_start_y) = polar_to_cartesian(center_x, center_y, outer_r, s_rad);
                let (out_end_x, out_end_y) = polar_to_cartesian(center_x, center_y, outer_r, e_rad);
                let large_arc = delta > std::f32::consts::PI;

                let mut path = Path::new();
                path = path.move_to(Point::new(out_start_x, out_start_y));
                path = path.arc_to(
                    outer_r,
                    outer_r,
                    0.0,
                    large_arc,
                    true,
                    Point::new(out_end_x, out_end_y),
                );

                if inner_r > 0.0 {
                    let (in_end_x, in_end_y) = polar_to_cartesian(center_x, center_y, inner_r, e_rad);
                    let (in_start_x, in_start_y) = polar_to_cartesian(center_x, center_y, inner_r, s_rad);
                    path = path.line_to(Point::new(in_end_x, in_end_y));
                    path = path.arc_to(
                        inner_r,
                        inner_r,
                        0.0,
                        large_arc,
                        false,
                        Point::new(in_start_x, in_start_y),
                    );
                } else {
                    path = path.line_to(Point::new(center_x, center_y));
                }

                path = path.close();
                canvas.draw_path(
                    &path,
                    Some(Stroke::new(palette.background(), 2.0)),
                    Some(Fill::solid(color)),
                );
            }

            // Draw Legend Entry on the right
            let legend_x = self.width * 0.65;
            let legend_y = center_y - (angles.len() as f32 * 15.0) + (i as f32 * 32.0);

            // Color box
            canvas.draw_rect(
                Rect::new(Point::new(legend_x, legend_y), Size::new(14.0, 14.0)).with_radius(3.0, 3.0),
                None,
                Some(Fill::solid(color)),
            );

            // Label & Percentage
            let pct_text = format!("{} ({:.1}%)", slice.label, pct * 100.0);
            canvas.draw_text(
                Point::new(legend_x + 24.0, legend_y + 11.0),
                &pct_text,
                13.0,
                palette.text_primary(),
                "start",
            );
        }

        // Center Metric Text (if donut)
        if self.hole_ratio > 0.0 {
            if let Some(ref p) = self.center_text_primary {
                let offset_y = if self.center_text_secondary.is_some() { -6.0 } else { 5.0 };
                canvas.draw_text(
                    Point::new(center_x, center_y + offset_y),
                    p,
                    17.0,
                    palette.text_primary(),
                    "middle",
                );
            }
            if let Some(ref s) = self.center_text_secondary {
                canvas.draw_text(
                    Point::new(center_x, center_y + 16.0),
                    s,
                    12.0,
                    palette.text_secondary(),
                    "middle",
                );
            }
        }

        Ok(canvas.to_svg())
    }
}

impl Default for DonutChart {
    fn default() -> Self {
        Self::new()
    }
}
