use crate::error::{PdfError, Result};
use crate::types::{PdfReadOptions, PdfReadResult};
use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::PdfDocument;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

/// Maximum allowable PDF file size (256 MB) to prevent infinite streams/memory exhaustion (K3)
const MAX_PDF_BYTES: usize = 256 * 1024 * 1024;

/// Maximum allowable character budget per single read turn (500k chars ~ 125k tokens) (K4)
const MAX_ALLOWED_CHAR_BUDGET: usize = 500_000;

/// Maximum allowable output file size for offload (64 MB) (T4)
const MAX_OFFLOAD_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// Safe wrapper around upstream calls that catches panics and downcasts payload into actionable error (T1 & S15)
fn guarded<T, F>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    crate::sandbox::guarded("reader", f)
}

/// High-level PDF reader and text/Markdown extractor.
/// Wraps pdf_oxide internally without leaking any pdf_oxide types.
pub struct PdfReader;

impl PdfReader {
    /// Internal helper to safely open and authenticate PDF documents (T5 & K3)
    fn open_doc(pdf_bytes: &[u8], password: Option<&str>) -> Result<PdfDocument> {
        if pdf_bytes.len() > MAX_PDF_BYTES {
            return Err(PdfError::LimitExceeded(format!(
                "PDF size ({} bytes) exceeds maximum allowable limit of {} bytes (256 MB)",
                pdf_bytes.len(),
                MAX_PDF_BYTES
            )));
        }

        let doc = PdfDocument::from_bytes(pdf_bytes.to_vec())
            .map_err(|e| PdfError::Read(format!("Failed to parse PDF bytes: {}", e)))?;

        if doc.is_encrypted() {
            if let Some(pwd) = password {
                let ok = doc
                    .authenticate(pwd.as_bytes())
                    .map_err(|e| PdfError::Encrypted(format!("Password authentication failed: {}", e)))?;
                if !ok {
                    return Err(PdfError::Encrypted("Incorrect password for protected PDF".into()));
                }
            } else if !doc.is_authenticated() {
                return Err(PdfError::Encrypted(
                    "Document is encrypted/password-protected. Provide `password` in PdfReadOptions.".into(),
                ));
            }
        }

        Ok(doc)
    }

    /// Validates that output_file path is a safe relative workspace path (K2 & K5 Sandbox)
    fn validate_output_path(path_str: &str) -> Result<std::path::PathBuf> {
        crate::sandbox::validate_relative_path(path_str, &["md", "markdown", "txt"])
    }

    /// Strips invisible zero-width and Unicode tag characters (U+E0000 - U+E007F) from extracted text.
    pub fn clean_extracted_text(text: &str) -> String {
        text.chars()
            .filter(|&c| !matches!(c, '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}' | '\u{034F}' | '\u{E0000}'..='\u{E007F}'))
            .collect()
    }

    /// Read a file safely bounded by MAX_PDF_BYTES avoiding TOCTOU & FIFO blocking (T2 & T6)
    fn read_bounded_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
        let p = path.as_ref();
        let meta = fs::metadata(p)?;
        if !meta.is_file() {
            return Err(PdfError::InvalidArgument(format!(
                "Path '{}' is not a regular file (device files and FIFOs rejected)",
                p.display()
            )));
        }
        let file = File::open(p)?;
        let mut buf = Vec::new();
        file.take(MAX_PDF_BYTES as u64 + 1).read_to_end(&mut buf)?;
        if buf.len() > MAX_PDF_BYTES {
            return Err(PdfError::LimitExceeded(format!(
                "File '{}' exceeds maximum 256 MB size limit",
                p.display()
            )));
        }
        Ok(buf)
    }

    /// Agent-focused structured reading with pagination, character budget guard, scanned document detection,
    /// and full-file offload option (`output_file`). All page numbers are 1-indexed.
    pub fn read(pdf_bytes: &[u8], options: Option<PdfReadOptions>) -> Result<PdfReadResult> {
        let opts = options.unwrap_or_default();

        // Validate max_chars budget (K4 & S12)
        if let Some(chars) = opts.max_chars {
            if chars == 0 {
                return Err(PdfError::InvalidArgument(
                    "max_chars must be greater than 0. Omit option to use default budget (100,000).".into(),
                ));
            }
            if chars > MAX_ALLOWED_CHAR_BUDGET {
                return Err(PdfError::LimitExceeded(format!(
                    "max_chars ({}) exceeds safe single-turn budget of {} characters. Use `output_file` to extract whole document.",
                    chars, MAX_ALLOWED_CHAR_BUDGET
                )));
            }
        }

        if let Some(limit) = opts.page_limit {
            if limit == 0 {
                return Err(PdfError::InvalidArgument("page_limit must be greater than 0".into()));
            }
        }

        // Validate output_file path sandbox (K2 & K5)
        if let Some(ref out_path) = opts.output_file {
            Self::validate_output_path(out_path)?;
        }

        // Catch unhandled panics from upstream parser safely (T1)
        guarded(|| {
            let doc = Self::open_doc(pdf_bytes, opts.password.as_deref())?;

            let total = doc
                .page_count()
                .map_err(|e| PdfError::Read(format!("Failed to get page count: {}", e)))?;

            if total == 0 {
                let saved_to = if let Some(ref out_path) = opts.output_file {
                    let safe_path = Self::validate_output_path(out_path)?;
                    crate::sandbox::atomic_write_bytes(&safe_path, &[])?;
                    Some(safe_path.display().to_string())
                } else {
                    None
                };

                return Ok(PdfReadResult {
                    markdown: String::new(),
                    total_pages: 0,
                    page_start: 0,
                    page_end: 0,
                    next_page_start: None,
                    truncated: false,
                    failed_pages: Vec::new(),
                    total_chars: Some(0),
                    saved_to,
                    note: Some("Empty PDF document (0 pages)".to_string()),
                    scanned_likely: false,
                });
            }

            let p_start = opts.page_start.unwrap_or(1);
            if p_start == 0 || p_start > total {
                return Err(PdfError::InvalidPage {
                    page: p_start,
                    total,
                });
            }

            let conv_opts = ConversionOptions::default();

            // 1. File Offload Scenario: User wants markdown extraction written streaming to disk (T4 & T5)
            if let Some(ref out_path) = opts.output_file {
                let safe_path = Self::validate_output_path(out_path)?;
                if let Some(parent) = safe_path.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)?;
                    }
                }

                // Race-free & atomic offload write via unique temp file (K2 & T4)
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let tmp_path = safe_path.with_extension(format!("tmp_{}_{}.tmp", std::process::id(), nanos));

                let p_limit = opts.page_limit.unwrap_or(total).max(1);
                let start_idx = p_start - 1;
                let end_idx = start_idx.saturating_add(p_limit).min(total);

                let run_offload = (|| -> Result<(usize, Vec<usize>, String)> {
                    let file = File::create(&tmp_path)?;
                    let mut writer = BufWriter::new(file);

                    let mut total_chars = 0;
                    let mut total_bytes_written = 0;
                    let mut failed_pages = Vec::new();
                    let mut preview = String::new();

                    for p_idx in start_idx..end_idx {
                        match doc.to_markdown(p_idx, &conv_opts) {
                            Ok(page_md) => {
                                let cleaned = Self::clean_extracted_text(&page_md);
                                let trimmed = cleaned.trim();
                                if !trimmed.is_empty() {
                                    if total_chars > 0 {
                                        write!(writer, "\n\n---\n\n")?;
                                        total_chars += 7;
                                        total_bytes_written += 7;
                                    }
                                    write!(writer, "{}", trimmed)?;
                                    total_chars += trimmed.chars().count();
                                    total_bytes_written += trimmed.len();

                                    if preview.len() < 400 {
                                        let rem = 400 - preview.len();
                                        preview.extend(trimmed.chars().take(rem));
                                    }
                                }

                                if total_bytes_written > MAX_OFFLOAD_OUTPUT_BYTES {
                                    return Err(PdfError::LimitExceeded(format!(
                                        "Extracted markdown exceeds safety output cap ({} bytes)",
                                        MAX_OFFLOAD_OUTPUT_BYTES
                                    )));
                                }
                            }
                            Err(_) => {
                                failed_pages.push(p_idx + 1);
                            }
                        }
                    }

                    writer.flush()?;
                    let inner = writer.into_inner().map_err(|e| PdfError::Io(e.into_error()))?;
                    inner.sync_all()?;
                    Ok((total_chars, failed_pages, preview))
                })();

                let (total_chars, failed_pages, preview) = match run_offload {
                    Ok(val) => {
                        if let Err(e) = fs::rename(&tmp_path, &safe_path) {
                            let _ = fs::remove_file(&tmp_path);
                            return Err(PdfError::Io(e));
                        }
                        val
                    }
                    Err(e) => {
                        let _ = fs::remove_file(&tmp_path);
                        return Err(e);
                    }
                };

                let pages_count = end_idx.saturating_sub(start_idx);
                let scanned_likely = pages_count > 0 && (total_chars / pages_count < 25);
                let is_full_doc = start_idx == 0 && end_idx == total;

                let note = if is_full_doc {
                    if scanned_likely {
                        format!(
                            "Extracted {total} pages ({total_chars} chars) and saved to {}. Scanned/image-only document suspected. Use `pdf_render` for visual inspection.",
                            safe_path.display()
                        )
                    } else {
                        format!(
                            "Extracted full document ({total} pages, {total_chars} chars) and saved to {}. Use search or read_file with offset to inspect.",
                            safe_path.display()
                        )
                    }
                } else if scanned_likely {
                    format!(
                        "Extracted pages {}-{} of {total} ({total_chars} chars) and saved to {}. Scanned/image-only document suspected. Use `pdf_render` for visual inspection.",
                        start_idx + 1,
                        end_idx,
                        safe_path.display()
                    )
                } else {
                    format!(
                        "Extracted pages {}-{} of {total} ({total_chars} chars) and saved to {}. Use search or read_file with offset to inspect.",
                        start_idx + 1,
                        end_idx,
                        safe_path.display()
                    )
                };

                let preview_cut: String = preview.chars().take(400).collect();
                let truncated = end_idx < total;
                let next_page_start = if end_idx < total {
                    Some(end_idx + 1)
                } else {
                    None
                };

                return Ok(PdfReadResult {
                    markdown: preview_cut,
                    total_pages: total,
                    page_start: start_idx + 1,
                    page_end: end_idx,
                    next_page_start,
                    truncated,
                    failed_pages,
                    total_chars: Some(total_chars),
                    saved_to: Some(safe_path.display().to_string()),
                    note: Some(note),
                    scanned_likely,
                });
            }

            // 2. In-Memory Windowed Context Scenario
            let p_limit = opts.page_limit.unwrap_or(10).max(1);
            let max_chars = opts.max_chars.unwrap_or(100_000);

            let mut collected_md = String::new();
            let mut truncated = false;
            let mut notes = Vec::new();
            let mut failed_pages = Vec::new();

            let start_idx = p_start - 1;
            // Fix C1: Prevent integer overflow using saturating_add
            let end_idx = start_idx.saturating_add(p_limit).min(total);

            let mut last_processed_page = start_idx;
            let mut next_page_start = None;
            let mut current_char_count: usize = 0;

            for p_idx in start_idx..end_idx {
                match doc.to_markdown(p_idx, &conv_opts) {
                    Ok(page_md) => {
                        let cleaned = Self::clean_extracted_text(&page_md);
                        let trimmed = cleaned.trim();
                        if !trimmed.is_empty() {
                            if !collected_md.is_empty() {
                                collected_md.push_str("\n\n---\n\n");
                                current_char_count = current_char_count.saturating_add(7);
                            }
                            collected_md.push_str(trimmed);
                            current_char_count = current_char_count.saturating_add(trimmed.chars().count());
                        }
                        last_processed_page = p_idx + 1;

                        // Fix C2, K3 & K4: Advance page so agent never gets stuck in an infinite loop
                        if current_char_count > max_chars {
                            let kept: String = collected_md.chars().take(max_chars).collect();
                            collected_md = kept;
                            truncated = true;
                            // Set next_page_start to next page only if there are pages left
                            next_page_start = (last_processed_page < total).then_some(last_processed_page + 1);
                            notes.push(format!(
                                "Output cut at {max_chars} character limit on page {last_processed_page}."
                            ));
                            break;
                        }
                    }
                    Err(e) => {
                        failed_pages.push(p_idx + 1);
                        notes.push(format!("Failed to parse page {}: {}", p_idx + 1, e));
                    }
                }
            }

            if end_idx < total && !truncated {
                truncated = true;
                next_page_start = Some(end_idx + 1);
                notes.push(format!(
                    "Showing pages {}-{} of {total}. Set `page_start={}` to read further.",
                    start_idx + 1,
                    end_idx,
                    end_idx + 1
                ));
            }

            let pages_read_count = last_processed_page.saturating_sub(start_idx);
            let scanned_likely = pages_read_count > 0 && (current_char_count / pages_read_count < 25);

            if scanned_likely {
                notes.push(
                    "No extractable text layer found (document appears to be scanned images). Use `pdf_render` for vision/image analysis.".to_string(),
                );
            }

            let note = (!notes.is_empty()).then(|| notes.join(" "));

            Ok(PdfReadResult {
                markdown: collected_md,
                total_pages: total,
                page_start: start_idx + 1,
                page_end: last_processed_page.max(start_idx + 1),
                next_page_start,
                truncated,
                failed_pages,
                total_chars: Some(current_char_count),
                saved_to: None,
                note,
                scanned_likely,
            })
        })
    }

    /// Read a PDF file from a path with structured agent options. All page numbers are 1-indexed.
    pub fn read_file<P: AsRef<Path>>(path: P, options: Option<PdfReadOptions>) -> Result<PdfReadResult> {
        let bytes = Self::read_bounded_file(path)?;
        Self::read(&bytes, options)
    }

    /// Extract full structured Markdown from in-memory PDF bytes.
    /// Strictly guarded by MAX_ALLOWED_CHAR_BUDGET (500k chars) to prevent context overflow or decompression bombs (D1).
    pub fn extract_markdown(pdf_bytes: &[u8]) -> Result<String> {
        guarded(|| {
            let doc = Self::open_doc(pdf_bytes, None)?;
            let options = ConversionOptions::default();
            let md = doc
                .to_markdown_all(&options)
                .map_err(|e| PdfError::Read(format!("Failed to extract Markdown: {}", e)))?;

            let count = md.chars().count();
            if count > MAX_ALLOWED_CHAR_BUDGET {
                return Err(PdfError::LimitExceeded(format!(
                    "Extracted Markdown ({} chars) exceeds single-turn budget of {} chars. Use PdfReader::read() with `output_file` to offload to disk.",
                    count,
                    MAX_ALLOWED_CHAR_BUDGET
                )));
            }
            Ok(Self::clean_extracted_text(&md))
        })
    }

    /// Extract full structured Markdown from a PDF file path.
    pub fn extract_markdown_file<P: AsRef<Path>>(path: P) -> Result<String> {
        let bytes = Self::read_bounded_file(path)?;
        Self::extract_markdown(&bytes)
    }

    /// Extract Markdown from a specific page (1-indexed page_number).
    pub fn extract_page_markdown(pdf_bytes: &[u8], page_number: usize) -> Result<String> {
        guarded(|| {
            let doc = Self::open_doc(pdf_bytes, None)?;
            let total = doc
                .page_count()
                .map_err(|e| PdfError::Read(format!("Failed to get page count: {}", e)))?;

            if page_number == 0 || page_number > total {
                return Err(PdfError::InvalidPage {
                    page: page_number,
                    total,
                });
            }

            let options = ConversionOptions::default();
            let md = doc
                .to_markdown(page_number - 1, &options)
                .map_err(|e| PdfError::Read(format!("Failed to extract page Markdown: {}", e)))?;

            let count = md.chars().count();
            if count > MAX_ALLOWED_CHAR_BUDGET {
                return Err(PdfError::LimitExceeded(format!(
                    "Extracted page {} ({} chars) exceeds safety budget of {} chars.",
                    page_number, count, MAX_ALLOWED_CHAR_BUDGET
                )));
            }
            Ok(Self::clean_extracted_text(&md))
        })
    }

    /// Get the total number of pages in the PDF document.
    pub fn page_count(pdf_bytes: &[u8]) -> Result<usize> {
        Self::page_count_with_password(pdf_bytes, None)
    }

    /// Get the total number of pages in the PDF document with optional decryption password.
    pub fn page_count_with_password(pdf_bytes: &[u8], password: Option<&str>) -> Result<usize> {
        guarded(|| {
            let doc = Self::open_doc(pdf_bytes, password)?;
            doc.page_count()
                .map_err(|e| PdfError::Read(format!("Failed to count pages: {}", e)))
        })
    }

    /// Search for a keyword across all pages. Returns a list of 1-indexed page numbers where the query appears.
    /// This allows AI agents to instantly locate relevant pages without reading hundreds of pages into context.
    pub fn search_keyword(pdf_bytes: &[u8], query: &str) -> Result<Vec<usize>> {
        Self::search_keyword_with_password(pdf_bytes, query, None)
    }

    /// Search for a keyword across all pages with optional decryption password.
    pub fn search_keyword_with_password(
        pdf_bytes: &[u8],
        query: &str,
        password: Option<&str>,
    ) -> Result<Vec<usize>> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Err(PdfError::InvalidArgument("Search query cannot be empty".to_string()));
        }

        guarded(|| {
            let doc = Self::open_doc(pdf_bytes, password)?;
            let total = doc
                .page_count()
                .map_err(|e| PdfError::Read(format!("Failed to get page count: {}", e)))?;

            let q_lower = trimmed_query.to_lowercase();
            let mut matched_pages = Vec::new();
            let conv_opts = ConversionOptions::default();

            for p_idx in 0..total {
                if let Ok(text) = doc.to_markdown(p_idx, &conv_opts) {
                    if text.to_lowercase().contains(&q_lower) {
                        matched_pages.push(p_idx + 1);
                    }
                }
            }

            Ok(matched_pages)
        })
    }

    /// Extract document structure and headings (`# Heading 1`, `## Heading 2`, etc.) with their 1-indexed page numbers.
    /// Returns `Vec<(page_number, heading_text)>`.
    pub fn extract_headings(pdf_bytes: &[u8]) -> Result<Vec<(usize, String)>> {
        Self::extract_headings_with_password(pdf_bytes, None)
    }

    /// Extract document structure and headings with optional decryption password.
    pub fn extract_headings_with_password(
        pdf_bytes: &[u8],
        password: Option<&str>,
    ) -> Result<Vec<(usize, String)>> {
        guarded(|| {
            let doc = Self::open_doc(pdf_bytes, password)?;
            let total = doc
                .page_count()
                .map_err(|e| PdfError::Read(format!("Failed to get page count: {}", e)))?;

            let mut headings = Vec::new();
            let conv_opts = ConversionOptions {
                detect_headings: true,
                ..Default::default()
            };

            for p_idx in 0..total {
                if let Ok(md) = doc.to_markdown(p_idx, &conv_opts) {
                    let mut in_code_fence = false;
                    for line in md.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("```") {
                            in_code_fence = !in_code_fence;
                            continue;
                        }
                        if !in_code_fence
                            && (trimmed.starts_with("# ")
                                || trimmed.starts_with("## ")
                                || trimmed.starts_with("### ")
                                || trimmed.starts_with("#### "))
                        {
                            headings.push((p_idx + 1, trimmed.to_string()));
                        }
                    }
                }
            }

            Ok(headings)
        })
    }
}
