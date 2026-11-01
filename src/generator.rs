use crate::error::{PdfError, Result};
use std::path::Path;

/// Maximum allowable HTML input length (5 MB) to prevent pathological AST nesting/DoS (H4)
const MAX_HTML_INPUT_BYTES: usize = 5 * 1024 * 1024;

const FALLBACK_STYLE: &str = "<style>html, body { font-family: 'DejaVu Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; }</style>";

/// High-level PDF generator (HTML/CSS -> PDF).
/// Wraps fulgur without leaking any fulgur types.
pub struct PdfGenerator;

impl PdfGenerator {
    /// Render HTML/CSS to PDF byte vector.
    /// Injects standard headless fallback fonts properly inside `<head>` or `<html>` if not declared (T6).
    pub fn from_html(html: &str) -> Result<Vec<u8>> {
        if html.len() > MAX_HTML_INPUT_BYTES {
            return Err(PdfError::LimitExceeded(format!(
                "HTML input ({} bytes) exceeds maximum allowable limit of {} bytes (5 MB)",
                html.len(),
                MAX_HTML_INPUT_BYTES
            )));
        }

        let processed_html = if !html.contains("font-family") {
            if let Some(pos) = html.find("<head>") {
                let (before, after) = html.split_at(pos + 6);
                format!("{}{}{}", before, FALLBACK_STYLE, after)
            } else if let Some(pos) = html.find("<html>") {
                let (before, after) = html.split_at(pos + 6);
                format!("{}<head>{}</head>{}", before, FALLBACK_STYLE, after)
            } else {
                format!("<!DOCTYPE html><html><head>{}</head><body>{}</body></html>", FALLBACK_STYLE, html)
            }
        } else {
            html.to_string()
        };

        crate::sandbox::guarded("generator", || {
            fulgur::convert_html(&processed_html)
                .map_err(|e| PdfError::Generate(format!("Failed to generate PDF from HTML: {}", e)))
        })
    }

    /// Render HTML/CSS to a destination PDF file path with sandbox path verification and atomic write (K2, C3, S8).
    pub fn from_html_to_file<P: AsRef<Path>>(html: &str, path: P) -> Result<()> {
        let p = path.as_ref();
        let safe_path = crate::sandbox::validate_relative_path(p.to_string_lossy().as_ref(), &["pdf"])?;

        let pdf_bytes = Self::from_html(html)?;
        crate::sandbox::atomic_write_bytes(&safe_path, &pdf_bytes)?;
        Ok(())
    }
}
