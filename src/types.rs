#[cfg(feature = "render")]
use crate::error::{PdfError, Result};
#[cfg(feature = "render")]
use std::fs::File;
#[cfg(feature = "render")]
use std::io::{BufWriter, Cursor};
#[cfg(feature = "render")]
use std::path::Path;

/// Options for reading and extracting Markdown from PDF documents.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PdfReadOptions {
    /// 1-indexed starting page. Default: 1
    pub page_start: Option<usize>,
    /// Number of pages to read from page_start. Default: 10
    pub page_limit: Option<usize>,
    /// Maximum character budget for output markdown. Default: 100_000
    pub max_chars: Option<usize>,
}

impl Default for PdfReadOptions {
    fn default() -> Self {
        Self {
            page_start: Some(1),
            page_limit: Some(10),
            max_chars: Some(100_000),
        }
    }
}

/// Structured result of PDF reading designed specifically for AI Agent consumption.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PdfReadResult {
    /// Extracted markdown text.
    pub markdown: String,
    /// Total number of pages in the PDF.
    pub total_pages: usize,
    /// 1-indexed start page of the extracted slice.
    pub page_start: usize,
    /// 1-indexed end page of the extracted slice.
    pub page_end: usize,
    /// True if the output was cut by character budget or page limit.
    pub truncated: bool,
    /// Helpful note/context for the LLM agent.
    pub note: Option<String>,
    /// True if document appears to be scanned images without text layer.
    pub scanned_likely: bool,
}

/// Represents a rasterized PDF page image (RGBA 8-bit).
#[derive(Debug, Clone)]
pub struct PageImage {
    pub width: u32,
    pub height: u32,
    pub raw_rgba: Vec<u8>,
}

impl PageImage {
    pub fn new(width: u32, height: u32, raw_rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            raw_rgba,
        }
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

    /// Save the rendered image to a file as PNG.
    #[cfg(feature = "render")]
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        image::write_buffer_with_format(
            &mut writer,
            &self.raw_rgba,
            self.width,
            self.height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|e| PdfError::Render(format!("Failed to save PNG: {}", e)))?;

        Ok(())
    }
}
