use crate::error::{PdfError, Result};
use std::fmt;
#[cfg(feature = "render")]
use std::io::Cursor;
#[cfg(feature = "render")]
use std::path::Path;

/// Options for reading and extracting Markdown from PDF documents.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PdfReadOptions {
    /// 1-indexed starting page. Default: 1
    pub page_start: Option<usize>,
    /// Number of pages to read from page_start. Default: 10
    pub page_limit: Option<usize>,
    /// Maximum character budget for output markdown in context. Default: 100_000
    pub max_chars: Option<usize>,
    /// Optional password for encrypted/protected PDF files.
    #[serde(skip_serializing)]
    pub password: Option<String>,
    /// If specified, writes 100% full markdown extraction to this file path,
    /// offloading large documents completely from the LLM context window.
    pub output_file: Option<String>,
}

impl Default for PdfReadOptions {
    fn default() -> Self {
        Self {
            page_start: Some(1),
            page_limit: Some(10),
            max_chars: Some(100_000),
            password: None,
            output_file: None,
        }
    }
}

impl PdfReadOptions {
    /// Create new default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set 1-indexed start page and page limit.
    pub fn with_pages(mut self, start: usize, limit: usize) -> Self {
        self.page_start = Some(start);
        self.page_limit = Some(limit);
        self
    }

    /// Set max character budget for LLM context.
    pub fn with_max_chars(mut self, max: usize) -> Self {
        self.max_chars = Some(max);
        self
    }

    /// Set decryption password.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set safe relative file path to offload entire document extraction to disk.
    pub fn with_output_file(mut self, path: impl Into<String>) -> Self {
        self.output_file = Some(path.into());
        self
    }
}

// Redact password in Debug output to prevent accidental leak in agent trace logs (K6)
impl fmt::Debug for PdfReadOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PdfReadOptions")
            .field("page_start", &self.page_start)
            .field("page_limit", &self.page_limit)
            .field("max_chars", &self.max_chars)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("output_file", &self.output_file)
            .finish()
    }
}

/// Structured result of PDF reading designed specifically for AI Agent consumption.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct PdfReadResult {
    /// Extracted markdown text (or preview if offloaded to file).
    pub markdown: String,
    /// Total number of pages in the PDF.
    pub total_pages: usize,
    /// 1-indexed start page of the extracted slice.
    pub page_start: usize,
    /// 1-indexed end page of the extracted slice.
    pub page_end: usize,
    /// Suggested 1-indexed start page for the next turn if truncated.
    pub next_page_start: Option<usize>,
    /// True if the output was cut by character budget or page limit.
    pub truncated: bool,
    /// List of 1-indexed page numbers that failed to parse (empty if all succeeded).
    pub failed_pages: Vec<usize>,
    /// Total characters of full extraction when offloaded to file.
    pub total_chars: Option<usize>,
    /// File path where full extraction was saved, if offload was requested.
    pub saved_to: Option<String>,
    /// Helpful note/context for the LLM agent.
    pub note: Option<String>,
    /// True if document appears to be scanned images without text layer.
    pub scanned_likely: bool,
}

/// Represents a rasterized PDF page image (RGBA 8-bit).
#[derive(Clone)]
pub struct PageImage {
    width: u32,
    height: u32,
    raw_rgba: Vec<u8>,
}

impl fmt::Debug for PageImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("raw_rgba_bytes", &self.raw_rgba.len())
            .finish()
    }
}

impl PageImage {
    /// Create a new `PageImage`, strictly validating buffer dimensions invariant (`width * height * 4 == raw_rgba.len()`).
    pub fn new(width: u32, height: u32, raw_rgba: Vec<u8>) -> Result<Self> {
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or_else(|| PdfError::Render("Pixel dimensions overflow integer bounds".into()))?;

        if raw_rgba.len() != expected_len {
            return Err(PdfError::Render(format!(
                "Invalid RGBA buffer length: expected {} bytes for {}x{}, got {}",
                expected_len,
                width,
                height,
                raw_rgba.len()
            )));
        }

        Ok(Self {
            width,
            height,
            raw_rgba,
        })
    }

    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw RGBA 8-bit byte slice.
    pub fn as_rgba(&self) -> &[u8] {
        &self.raw_rgba
    }

    /// Consume into owned RGBA bytes.
    pub fn into_rgba(self) -> Vec<u8> {
        self.raw_rgba
    }

    /// Encode the raw RGBA pixels into PNG format bytes.
    #[cfg(feature = "render")]
    pub fn to_png_bytes(&self) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());
        image::write_buffer_with_format(
            &mut cursor,
            &self.raw_rgba,
            self.width,
            self.height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|e| PdfError::Render(format!("Failed to encode PNG bytes: {}", e)))?;

        Ok(cursor.into_inner())
    }

    /// Save the rendered image to a file as PNG, ensuring sandbox path validation and atomic write (K1, K5, T5).
    #[cfg(feature = "render")]
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let p = path.as_ref();
        let path_str = p.to_str().ok_or_else(|| {
            PdfError::InvalidArgument("Target PNG path must be valid UTF-8".into())
        })?;
        let safe_path = crate::sandbox::validate_relative_path(path_str, &["png"])?;

        let png_bytes = self.to_png_bytes()?;
        crate::sandbox::atomic_write_bytes(&safe_path, &png_bytes)?;
        Ok(())
    }
}
