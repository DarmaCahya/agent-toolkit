#![forbid(unsafe_code)]

//! Vector SVG rendering for [`QrCode`](super::QrCode).
//!
//! Both renderers emit pure geometry: a single merged `<path>` or one
//! `<rect>` per dark module. No `<text>`, font family or external resource is
//! ever produced, so the output is stable across platforms.

use super::QrCode;
use crate::canvas::geom::fmt_num;
use crate::canvas::{Path, Point, SvgCanvas};

impl QrCode {
    /// Edge length of one module in output pixels (quiet zone included).
    fn module_scale(&self) -> f32 {
        let modules = self.size() + 2 * usize::from(self.quiet_zone);
        if modules == 0 {
            return 0.0;
        }
        self.size_px / modules as f32
    }

    fn fill_attributes(&self) -> String {
        let mut attributes = format!(" fill=\"{}\"", self.dark.to_rgb_hex());
        if self.dark.a < 1.0 {
            attributes.push_str(&format!(" fill-opacity=\"{}\"", fmt_num(self.dark.a)));
        }
        attributes
    }

    /// Renders the symbol as one `<path>` element whose dark modules are merged
    /// into horizontal runs.
    ///
    /// The element includes the quiet zone and is positioned at the SVG origin.
    pub fn to_path(&self) -> String {
        let scale = self.module_scale();
        let quiet = f32::from(self.quiet_zone);
        let size = self.size();
        let mut path = Path::new();

        for y in 0..size {
            let mut x = 0usize;
            while x < size {
                if !self.matrix.get(x, y) {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < size && self.matrix.get(x, y) {
                    x += 1;
                }
                let left = (quiet + start as f32) * scale;
                let right = (quiet + x as f32) * scale;
                let top = (quiet + y as f32) * scale;
                let bottom = (quiet + (y + 1) as f32) * scale;
                path = path
                    .move_to(Point::new(left, top))
                    .line_to(Point::new(right, top))
                    .line_to(Point::new(right, bottom))
                    .line_to(Point::new(left, bottom))
                    .close();
            }
        }

        format!(
            "<path d=\"{}\"{} shape-rendering=\"crispEdges\"/>",
            path.to_svg_d(),
            self.fill_attributes()
        )
    }

    /// Renders the symbol as one `<rect>` element per dark module.
    ///
    /// Useful for editors that prefer individual shapes over a merged path.
    /// The elements include the quiet zone and are positioned at the SVG origin.
    pub fn to_rects(&self) -> String {
        let scale = self.module_scale();
        let quiet = f32::from(self.quiet_zone);
        let size = self.size();
        let fill = self.fill_attributes();
        let mut rects = String::new();

        for y in 0..size {
            for x in 0..size {
                if !self.matrix.get(x, y) {
                    continue;
                }
                let left = (quiet + x as f32) * scale;
                let top = (quiet + y as f32) * scale;
                rects.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>\n",
                    fmt_num(left),
                    fmt_num(top),
                    fmt_num(scale),
                    fmt_num(scale),
                    fill
                ));
            }
        }
        rects
    }

    /// Renders a complete, deterministic SVG document.
    ///
    /// The background rectangle uses the light colour; the symbol is emitted as
    /// a single merged path. The document has no `<text>`, `font-family`, or
    /// external references.
    ///
    /// ```
    /// use agent_canvas::component::QrCode;
    ///
    /// let svg = QrCode::encode("KDX-2049")?.with_size(256.0).to_svg();
    /// assert!(svg.starts_with("<svg"));
    /// assert!(!svg.contains("<text"));
    /// # Ok::<(), agent_canvas::component::qrcode::QrError>(())
    /// ```
    pub fn to_svg(&self) -> String {
        let mut canvas = SvgCanvas::new(self.size_px, self.size_px)
            .with_view_box(0.0, 0.0, self.size_px, self.size_px)
            .with_background(self.light);
        canvas.elements.push(self.to_path());
        canvas.to_svg()
    }
}
