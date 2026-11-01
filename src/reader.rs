use crate::error::{PdfError, Result};
use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::PdfDocument;
use std::path::Path;

/// High-level PDF reader and text/Markdown extractor.
/// Wraps pdf_oxide internally without leaking any pdf_oxide types.
pub struct PdfReader;

impl PdfReader {
    /// Extract full structured Markdown from in-memory PDF bytes.
    pub fn extract_markdown(pdf_bytes: &[u8]) -> Result<String> {
        let doc = PdfDocument::from_bytes(pdf_bytes.to_vec())
            .map_err(|e| PdfError::Read(format!("Failed to parse PDF bytes: {}", e)))?;

        let options = ConversionOptions::default();
        doc.to_markdown_all(&options)
            .map_err(|e| PdfError::Read(format!("Failed to extract Markdown: {}", e)))
    }

    /// Extract full structured Markdown from a PDF file path.
    pub fn extract_markdown_file<P: AsRef<Path>>(path: P) -> Result<String> {
        let doc = PdfDocument::open(path.as_ref())
            .map_err(|e| PdfError::Read(format!("Failed to open PDF file: {}", e)))?;

        let options = ConversionOptions::default();
        doc.to_markdown_all(&options)
            .map_err(|e| PdfError::Read(format!("Failed to extract Markdown: {}", e)))
    }

    /// Extract Markdown from a specific page (0-indexed).
    pub fn extract_page_markdown(pdf_bytes: &[u8], page_index: usize) -> Result<String> {
        let doc = PdfDocument::from_bytes(pdf_bytes.to_vec())
            .map_err(|e| PdfError::Read(format!("Failed to parse PDF bytes: {}", e)))?;

        let total = doc
            .page_count()
            .map_err(|e| PdfError::Read(format!("Failed to get page count: {}", e)))?;

        if page_index >= total {
            return Err(PdfError::InvalidPage {
                page: page_index,
                total,
            });
        }

        let options = ConversionOptions::default();
        doc.to_markdown(page_index, &options)
            .map_err(|e| PdfError::Read(format!("Failed to extract page Markdown: {}", e)))
    }

    /// Get the total number of pages in the PDF document.
    pub fn page_count(pdf_bytes: &[u8]) -> Result<usize> {
        let doc = PdfDocument::from_bytes(pdf_bytes.to_vec())
            .map_err(|e| PdfError::Read(format!("Failed to parse PDF bytes: {}", e)))?;

        doc.page_count()
            .map_err(|e| PdfError::Read(format!("Failed to count pages: {}", e)))
    }
}
