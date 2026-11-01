use crate::error::{PdfError, Result};
use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::path::Path;

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
