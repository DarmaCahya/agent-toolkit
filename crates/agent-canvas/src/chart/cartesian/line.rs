use super::plot::CartesianPlot;
use crate::canvas::{Color, Fill, Palette, Path, Point, Rect, Stroke, SvgCanvas, ThemeMode};
use crate::error::Result;
use crate::math::{CategoryScale, LinearScale};

#[derive(Debug, Clone)]
pub struct LineSeries {
    pub name: String,
    pub values: Vec<f32>,
    pub color: Option<Color>,
    pub area_fill: bool,
    pub stroke_width: f32,
}

impl LineSeries {
    pub fn new(name: impl Into<String>, values: &[f32]) -> Self {
        Self {
            name: name.into(),
            values: values.to_vec(),
            color: None,
            area_fill: false,
            stroke_width: 2.5,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_area_fill(mut self, fill: bool) -> Self {
        self.area_fill = fill;
        self
    }

    pub fn with_stroke_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.stroke_width = width;
        }
        self
    }

    pub fn render_on(
        &self,
        canvas: &mut SvgCanvas,
        plot_area: &Rect,
        cat_scale: &CategoryScale,
        y_scale: &LinearScale,
        fallback_color: Color,
    ) {
        if self.values.is_empty() {
            return;
        }
        let line_color = self.color.unwrap_or(fallback_color);
        let mut points: Vec<Point> = Vec::new();

        for (i, &v) in self.values.iter().enumerate() {
            if v.is_finite() {
                let x = cat_scale.map_center(i);
                let y = y_scale.map(v);
                if x.is_finite() && y.is_finite() {
                    points.push(Point::new(x, y));
                }
            }
        }

        if points.is_empty() {
            return;
        }

        // Draw area fill if requested
        if self.area_fill && points.len() >= 2 {
            let zero_y = y_scale.map(0.0).clamp(plot_area.origin.y, plot_area.origin.y + plot_area.size.height);
            let mut area_path = Path::new();
            area_path = area_path.move_to(Point::new(points[0].x, zero_y));
            for p in &points {
                area_path = area_path.line_to(*p);
            }
            area_path = area_path.line_to(Point::new(points.last().unwrap().x, zero_y));
            area_path = area_path.close();
            canvas.draw_path(&area_path, None, Some(Fill::new(line_color, 0.15)));
        }

        // Draw line curve
        let mut line_path = Path::new();
        line_path = line_path.move_to(points[0]);
        for p in &points[1..] {
            line_path = line_path.line_to(*p);
        }
        canvas.draw_path(
            &line_path,
            Some(Stroke::new(line_color, self.stroke_width)),
            None,
        );

        // Draw marker dots at data points
        for p in &points {
            canvas.draw_circle(
                *p,
                3.5,
                Some(Stroke::new(Color::from_rgb(255, 255, 255), 1.5)),
                Some(Fill::solid(line_color)),
            );
        }
    }
}

/// Shorthand LineChart builder for AI agents.
#[derive(Debug, Clone)]
pub struct LineChart {
    pub title: Option<String>,
    pub categories: Vec<String>,
    pub series: Vec<LineSeries>,
    pub width: f32,
    pub height: f32,
    pub theme: ThemeMode,
}

impl LineChart {
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

    pub fn add_series(mut self, s: LineSeries) -> Self {
        self.series.push(s);
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

        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;
        for s in &self.series {
            for &v in &s.values {
                if v.is_finite() {
                    min_val = min_val.min(v);
                    max_val = max_val.max(v);
                }
            }
        }
        if min_val >= max_val {
            min_val = 0.0;
            max_val = 10.0;
        }

        let (plot_rect, y_scale, cat_scale) = plot.render_frame(&mut canvas, min_val, max_val)?;

        for (i, s) in self.series.iter().enumerate() {
            let color = palette.get_categorical(i);
            s.render_on(&mut canvas, &plot_rect, &cat_scale, &y_scale, color);
        }

        Ok(canvas.to_svg())
    }
}

impl Default for LineChart {
    fn default() -> Self {
        Self::new()
    }
}
