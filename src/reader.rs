use crate::error::{PdfError, Result};
use crate::types::{PdfReadOptions, PdfReadResult};
use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::PdfDocument;
use std::fs;
use std::path::Path;

/// High-level PDF reader and text/Markdown extractor.
/// Wraps pdf_oxide internally without leaking any pdf_oxide types.
pub struct PdfReader;

impl PdfReader {
    /// Agent-focused structured reading with pagination, character budget guard, scanned document detection,
    /// and full-file offload option (`output_file`).
    pub fn read(pdf_bytes: &[u8], options: Option<PdfReadOptions>) -> Result<PdfReadResult> {
        let opts = options.unwrap_or_default();
        let doc = PdfDocument::from_bytes(pdf_bytes.to_vec())
            .map_err(|e| PdfError::Read(format!("Failed to parse PDF bytes: {}", e)))?;

        let total = doc
            .page_count()
            .map_err(|e| PdfError::Read(format!("Failed to get page count: {}", e)))?;

        if total == 0 {
            return Ok(PdfReadResult {
                markdown: String::new(),
                total_pages: 0,
                page_start: 0,
                page_end: 0,
                truncated: false,
                total_chars: Some(0),
                saved_to: None,
                note: Some("Empty PDF document (0 pages)".to_string()),
                scanned_likely: false,
            });
        }

        let conv_opts = ConversionOptions::default();

        // 1. File Offload Scenario: User wants 100% full extraction written directly to disk
        if let Some(ref out_path) = opts.output_file {
            let full_md = doc
                .to_markdown_all(&conv_opts)
                .map_err(|e| PdfError::Read(format!("Failed to extract full document: {}", e)))?;
            let total_chars = full_md.chars().count();
            fs::write(out_path, &full_md)?;

            let scanned_likely = total > 0 && (total_chars / total < 25);
            let note = if scanned_likely {
                format!(
                    "Extracted {total} pages ({total_chars} chars) and saved to {out_path}. Scanned/image-only document suspected. Use `pdf_render` for visual inspection."
                )
            } else {
                format!(
                    "Extracted full document ({total} pages, {total_chars} chars) and saved to {out_path}. Use search or read_file with offset to inspect."
                )
            };

            let preview: String = full_md.chars().take(400).collect();

            return Ok(PdfReadResult {
                markdown: preview,
                total_pages: total,
                page_start: 1,
                page_end: total,
                truncated: false,
                total_chars: Some(total_chars),
                saved_to: Some(out_path.clone()),
                note: Some(note),
                scanned_likely,
            });
        }

        // 2. In-Memory Windowed Context Scenario: Protect LLM context window with paging & char limits
        let p_start = opts.page_start.unwrap_or(1).max(1);
        let p_limit = opts.page_limit.unwrap_or(10).max(1);
        let max_chars = opts.max_chars.unwrap_or(100_000);

        let mut collected_md = String::new();
        let mut truncated = false;
        let mut notes = Vec::new();

        let start_idx = (p_start - 1).min(total - 1);
        let end_idx = (start_idx + p_limit).min(total);

        for p_idx in start_idx..end_idx {
            match doc.to_markdown(p_idx, &conv_opts) {
                Ok(page_md) => {
                    let trimmed = page_md.trim();
                    if !trimmed.is_empty() {
                        if !collected_md.is_empty() {
                            collected_md.push_str("\n\n---\n\n");
                        }
                        collected_md.push_str(trimmed);
                    }
                    if collected_md.chars().count() >= max_chars {
                        let kept: String = collected_md.chars().take(max_chars).collect();
                        collected_md = kept;
                        truncated = true;
                        notes.push(format!("Output cut at {max_chars} character limit."));
                        break;
                    }
                }
                Err(e) => {
                    notes.push(format!("Failed to parse page {}: {}", p_idx + 1, e));
                }
            }
        }

        if end_idx < total && !truncated {
            truncated = true;
            notes.push(format!(
                "Showing pages {}-{} of {total}. Set `page_start={}` to read further.",
                start_idx + 1,
                end_idx,
                end_idx + 1
            ));
        }

        // Scanned detection: if average text per read page < 25 chars
        let pages_read_count = end_idx - start_idx;
        let total_chars = collected_md.trim().chars().count();
        let scanned_likely = pages_read_count > 0 && (total_chars / pages_read_count < 25);

        if scanned_likely {
            notes.push(
                "No extractable text layer found (document appears to be scanned images). Use `pdf_render` for vision/image analysis.".to_string()
            );
        }

        let note = (!notes.is_empty()).then(|| notes.join(" "));

        Ok(PdfReadResult {
            markdown: collected_md,
            total_pages: total,
            page_start: start_idx + 1,
            page_end: end_idx,
            truncated,
            total_chars: None,
            saved_to: None,
            note,
            scanned_likely,
        })
    }

    /// Read a PDF file from a path with structured agent options.
    pub fn read_file<P: AsRef<Path>>(path: P, options: Option<PdfReadOptions>) -> Result<PdfReadResult> {
        let bytes = fs::read(path)?;
        Self::read(&bytes, options)
    }

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
