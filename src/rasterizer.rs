use crate::error::{PdfError, Result};
use crate::types::PageImage;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;

/// Maximum allowable PDF file size (256 MB) to prevent runaway memory usage
const MAX_PDF_BYTES: usize = 256 * 1024 * 1024;

/// Maximum allowable pixel count for a single rasterized page (~25 megapixels / 100MB buffer)
const MAX_PAGE_PIXELS: usize = 25_000_000;

/// High-level PDF rasterizer (PDF -> Image/PNG).
/// Wraps hayro (pure-Rust CPU rasterizer) without leaking any hayro types.
pub struct PdfRasterizer;

impl PdfRasterizer {
    /// Render a single page (1-indexed page_number) at given scale (e.g. 1.0 or 2.0) to `PageImage`.
    pub fn render_page(pdf_bytes: &[u8], page_number: usize, scale: f32) -> Result<PageImage> {
        if pdf_bytes.len() > MAX_PDF_BYTES {
            return Err(PdfError::LimitExceeded(format!(
                "PDF size ({} bytes) exceeds maximum limit of {} bytes (256 MB)",
                pdf_bytes.len(),
                MAX_PDF_BYTES
            )));
        }

        if !scale.is_finite() || scale < 0.1 || scale > 4.0 {
            return Err(PdfError::InvalidArgument(format!(
                "Render scale must be between 0.1 and 4.0, got: {}",
                scale
            )));
        }

        // Catch potential panics from unhandled upstream parser edge cases (T1 & S15)
        crate::sandbox::guarded("rasterizer", || {
            let pdf = Pdf::new(pdf_bytes.to_vec())
                .map_err(|e| PdfError::Render(format!("Failed to parse PDF for rendering: {:?}", e)))?;

            let total = pdf.pages().len();
            if page_number == 0 || page_number > total {
                return Err(PdfError::InvalidPage {
                    page: page_number,
                    total,
                });
            }

            let page_index = page_number - 1;

            // Pre-render dimension calculation to prevent 10GB+ OOM abort (C1 & D5)
            let page = &pdf.pages()[page_index];
            let (w_pt, h_pt) = page.render_dimensions();
            if !w_pt.is_finite() || !h_pt.is_finite() || w_pt <= 0.0 || h_pt <= 0.0 {
                return Err(PdfError::Render(format!(
                    "Invalid page dimensions on page {}: ({}pt x {}pt)",
                    page_number, w_pt, h_pt
                )));
            }

            let est_w = (w_pt as f64 * scale as f64).ceil() as usize;
            let est_h = (h_pt as f64 * scale as f64).ceil() as usize;
            let est_pixels = est_w.saturating_mul(est_h);
            if est_pixels > MAX_PAGE_PIXELS {
                return Err(PdfError::LimitExceeded(format!(
                    "Pre-render page dimensions at scale {:.1}x ({}x{} px = {} pixels) exceed maximum safe budget of {} pixels",
                    scale, est_w, est_h, est_pixels, MAX_PAGE_PIXELS
                )));
            }

            let settings = InterpreterSettings::default();
            let page_range = Some(page_index..=page_index);

            let pixmaps = hayro::render_pdf(&pdf, scale, settings, page_range)
                .ok_or_else(|| PdfError::Render(format!("Failed to rasterize page {}", page_number)))?;

            let pixmap = pixmaps.first().ok_or_else(|| {
                PdfError::Render(format!("No pixmap returned for page {}", page_number))
            })?;

            let width = pixmap.width() as u32;
            let height = pixmap.height() as u32;

            let pixel_count = (width as usize).saturating_mul(height as usize);
            if pixel_count > MAX_PAGE_PIXELS {
                return Err(PdfError::LimitExceeded(format!(
                    "Page dimension exceeds safe pixel budget: {}x{} ({} pixels > max {})",
                    width, height, pixel_count, MAX_PAGE_PIXELS
                )));
            }

            let raw_slice: &[u8] = bytemuck::cast_slice(pixmap.data());
            PageImage::new(width, height, raw_slice.to_vec())
        })
    }

    /// Render a page (1-indexed) and immediately return PNG encoded bytes.
    pub fn render_page_to_png(pdf_bytes: &[u8], page_number: usize, scale: f32) -> Result<Vec<u8>> {
        let image = Self::render_page(pdf_bytes, page_number, scale)?;
        image.to_png_bytes()
    }
}
