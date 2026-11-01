use crate::error::{PdfError, Result};
use crate::types::PageImage;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;

/// High-level PDF rasterizer (PDF -> Image/PNG).
/// Wraps hayro (pure-Rust CPU rasterizer) without leaking any hayro types.
pub struct PdfRasterizer;

impl PdfRasterizer {
    /// Render a single page (0-indexed) at given scale (e.g. 2.0 for 2x scale) to `PageImage`.
    pub fn render_page(pdf_bytes: &[u8], page_index: usize, scale: f32) -> Result<PageImage> {
        let pdf = Pdf::new(pdf_bytes.to_vec())
            .map_err(|e| PdfError::Render(format!("Failed to parse PDF for rendering: {:?}", e)))?;

        let total = pdf.pages().len();
        if page_index >= total {
            return Err(PdfError::InvalidPage {
                page: page_index,
                total,
            });
        }

        let settings = InterpreterSettings::default();
        let page_range = Some(page_index..=page_index);

        let pixmaps = hayro::render_pdf(&pdf, scale, settings, page_range)
            .ok_or_else(|| PdfError::Render(format!("Failed to rasterize page {}", page_index)))?;

        let pixmap = pixmaps.first().ok_or_else(|| {
            PdfError::Render(format!("No pixmap returned for page {}", page_index))
        })?;

        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;
        let raw_slice: &[u8] = bytemuck::cast_slice(pixmap.data());

        Ok(PageImage::new(width, height, raw_slice.to_vec()))
    }

    /// Render a page and immediately return PNG encoded bytes.
    pub fn render_page_to_png(pdf_bytes: &[u8], page_index: usize, scale: f32) -> Result<Vec<u8>> {
        let image = Self::render_page(pdf_bytes, page_index, scale)?;
        image.to_png_bytes()
    }
}
