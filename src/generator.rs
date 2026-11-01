use crate::error::{PdfError, Result};
use std::fs;
use std::path::Path;

/// High-level PDF generator (HTML/CSS -> PDF).
/// Wraps fulgur without leaking any fulgur types.
pub struct PdfGenerator;

impl PdfGenerator {
    /// Render HTML/CSS to PDF byte vector.
    /// Injects standard headless fallback fonts if not declared to prevent tofu glyphs.
    pub fn from_html(html: &str) -> Result<Vec<u8>> {
        let processed_html = if !html.contains("font-family") {
            format!(
                "<style>html, body {{ font-family: 'DejaVu Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; }}</style>\n{}",
                html
            )
        } else {
            html.to_string()
        };

        fulgur::convert_html(&processed_html)
            .map_err(|e| PdfError::Generate(format!("Failed to generate PDF from HTML: {}", e)))
    }

    /// Render HTML/CSS to a destination PDF file path.
    pub fn from_html_to_file<P: AsRef<Path>>(html: &str, path: P) -> Result<()> {
        let pdf_bytes = Self::from_html(html)?;
        fs::write(path, pdf_bytes)?;
        Ok(())
    }
}
