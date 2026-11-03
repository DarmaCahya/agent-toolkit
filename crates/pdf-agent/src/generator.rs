use crate::error::{PdfError, Result};
use std::path::Path;

/// Maximum allowable HTML input length (5 MB) to prevent pathological AST nesting/DoS (H4)
const MAX_HTML_INPUT_BYTES: usize = 5 * 1024 * 1024;

const FALLBACK_STYLE: &str = "<style>:where(html, body) { font-family: 'DejaVu Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; font-variant-ligatures: none; }</style>";

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

        let processed_html = process_html(html);

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

/// Find the opening HTML tag with the given `tag_name` case-insensitively,
/// accounting for attributes and comments. Returns `(start_index, end_index)`
/// where `start_index` points to `<` and `end_index` points immediately after `>`.
fn find_opening_tag(html: &str, tag_name: &str) -> Option<(usize, usize)> {
    let bytes = html.as_bytes();
    let tag_len = tag_name.len();
    let mut i = 0;

    while i < bytes.len() {
        // Skip comments <!-- ... -->
        if bytes[i..].starts_with(b"<!--") {
            if let Some(pos) = bytes[i + 4..].windows(3).position(|w| w == b"-->") {
                i += 4 + pos + 3;
            } else {
                break;
            }
            continue;
        }

        // Skip doctypes, closing tags, and processing instructions
        if bytes[i..].starts_with(b"<!") || bytes[i..].starts_with(b"</") || bytes[i..].starts_with(b"<?") {
            if let Some(pos) = bytes[i..].iter().position(|&b| b == b'>') {
                i += pos + 1;
            } else {
                break;
            }
            continue;
        }

        if bytes[i] == b'<' && i + 1 + tag_len <= bytes.len() {
            let candidate = &bytes[i + 1..i + 1 + tag_len];
            if candidate.eq_ignore_ascii_case(tag_name.as_bytes()) {
                let next_byte = bytes.get(i + 1 + tag_len);
                if matches!(
                    next_byte,
                    None | Some(b' ' | b'\t' | b'\r' | b'\n' | b'\x0C' | b'>' | b'/')
                ) {
                    let tag_start = i;
                    let mut j = i + 1 + tag_len;
                    let mut in_quote: Option<u8> = None;
                    while j < bytes.len() {
                        let b = bytes[j];
                        if let Some(quote) = in_quote {
                            if b == quote {
                                in_quote = None;
                            }
                        } else if b == b'"' || b == b'\'' {
                            in_quote = Some(b);
                        } else if b == b'>' {
                            return Some((tag_start, j + 1));
                        }
                        j += 1;
                    }
                    return Some((tag_start, j));
                }
            }
        }

        i += 1;
    }

    None
}

/// Check if a closing tag `</tag_name>` exists case-insensitively.
fn has_closing_tag(html: &str, tag_name: &str) -> bool {
    let bytes = html.as_bytes();
    let tag_len = tag_name.len();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(b"</") {
            let start = i + 2;
            if start + tag_len <= bytes.len()
                && bytes[start..start + tag_len].eq_ignore_ascii_case(tag_name.as_bytes())
            {
                let next = bytes.get(start + tag_len);
                if matches!(
                    next,
                    None | Some(b' ' | b'\t' | b'\r' | b'\n' | b'\x0C' | b'>')
                ) {
                    return true;
                }
            }
        }
        i += 1;
    }

    false
}

/// Process HTML input by sanitizing invisible code points and injecting font fallback non-invasively if `font-family` is not declared.
pub(crate) fn process_html(html: &str) -> String {
    // Sanitize invisible zero-width and Unicode tag characters (U+E0000 - U+E007F)
    let sanitized: String = html
        .chars()
        .filter(|&c| !matches!(c, '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}' | '\u{034F}' | '\u{E0000}'..='\u{E007F}'))
        .collect();

    let html = &sanitized;

    // If font-family is already explicitly declared in input HTML, do not override.
    if html.to_ascii_lowercase().contains("font-family") {
        return html.to_string();
    }

    // 1. If <head> exists (case-insensitive, with any attributes):
    if let Some((_tag_start, tag_end)) = find_opening_tag(html, "head") {
        let (before, after) = html.split_at(tag_end);
        return format!("{}{}{}", before, FALLBACK_STYLE, after);
    }

    // 2. If <html> exists (case-insensitive, with any attributes), but no <head>:
    if let Some((_tag_start, tag_end)) = find_opening_tag(html, "html") {
        let (before, after) = html.split_at(tag_end);
        return format!("{}<head>{}</head>{}", before, FALLBACK_STYLE, after);
    }

    // 3. If <body> exists (case-insensitive, with any attributes), but no <html>:
    if let Some((tag_start, _tag_end)) = find_opening_tag(html, "body") {
        let (before, after) = html.split_at(tag_start);
        let has_doctype = before.to_ascii_lowercase().contains("<!doctype");
        let prefix = if has_doctype {
            format!("{}<html><head>{}</head>", before, FALLBACK_STYLE)
        } else {
            format!("<!DOCTYPE html>{}<html><head>{}</head>", before, FALLBACK_STYLE)
        };
        let suffix = if has_closing_tag(html, "html") { "" } else { "</html>" };
        return format!("{}{}{}", prefix, after, suffix);
    }

    // 4. Raw body fragment without <html> or <body>:
    format!(
        "<!DOCTYPE html><html><head>{}</head><body>{}</body></html>",
        FALLBACK_STYLE, html
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_uppercase_head_tags() {
        let html = r#"
        <!DOCTYPE html>
        <HTML>
        <HEAD>
            <TITLE>Uppercase Head Test</TITLE>
        </HEAD>
        <BODY>
            <H1>Uppercase Tags</H1>
            <P>Testing case-insensitive tag handling in PdfGenerator.</P>
        </BODY>
        </HTML>
        "#;

        let processed = process_html(html);
        assert!(
            processed.contains(":where(html, body)"),
            "Fallback style must be injected"
        );
        let html_count = processed.to_ascii_lowercase().matches("<html").count();
        let body_count = processed.to_ascii_lowercase().matches("<body").count();
        assert_eq!(html_count, 1, "Should not duplicate <html> tag");
        assert_eq!(body_count, 1, "Should not duplicate <body> tag");

        let pdf = PdfGenerator::from_html(html).expect("Generation should succeed with uppercase tags");
        assert!(pdf.starts_with(b"%PDF-"), "Must produce valid PDF");
    }

    #[test]
    fn test_generator_html_attributes() {
        let html_with_head_attrs = r#"
        <!DOCTYPE html>
        <html lang="en" class="dark-theme">
        <head profile="http://www.w3.org/2005/10/profile" class="head-class">
            <title>HTML Attributes Test</title>
        </head>
        <body class="container mx-auto" data-env="test">
            <h1>Document with Attributes</h1>
            <p>Testing attributes on html, head, and body tags.</p>
        </body>
        </html>
        "#;

        let processed_head = process_html(html_with_head_attrs);
        assert!(processed_head.contains(":where(html, body)"));
        assert_eq!(
            processed_head.to_ascii_lowercase().matches("<html").count(),
            1,
            "No duplicate <html>"
        );
        assert_eq!(
            processed_head.to_ascii_lowercase().matches("<body").count(),
            1,
            "No duplicate <body>"
        );

        let pdf1 = PdfGenerator::from_html(html_with_head_attrs).expect("Should succeed with attributes");
        assert!(pdf1.starts_with(b"%PDF-"));

        let html_no_head = r#"
        <html lang="fr" dir="ltr">
        <body id="main">
            <h1>No Head Tag With Attributes</h1>
        </body>
        </html>
        "#;

        let processed_no_head = process_html(html_no_head);
        assert!(processed_no_head.contains(":where(html, body)"));
        assert_eq!(
            processed_no_head.to_ascii_lowercase().matches("<html").count(),
            1,
            "No duplicate <html>"
        );
        assert_eq!(
            processed_no_head.to_ascii_lowercase().matches("<body").count(),
            1,
            "No duplicate <body>"
        );

        let pdf2 = PdfGenerator::from_html(html_no_head).expect("Should succeed without head and with attributes");
        assert!(pdf2.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_generator_raw_body_fragment() {
        // Fragment without <html> or <body>
        let raw_fragment = r#"
        <div class="card">
            <h2>Card Fragment</h2>
            <p>This is a raw fragment without html or body tags.</p>
        </div>
        "#;

        let processed_fragment = process_html(raw_fragment);
        assert!(processed_fragment.contains(":where(html, body)"));
        assert!(processed_fragment.contains("<!DOCTYPE html><html><head>"));
        assert!(processed_fragment.contains("<body>"));
        assert!(processed_fragment.ends_with("</body></html>"));

        let pdf1 = PdfGenerator::from_html(raw_fragment).expect("Should generate PDF from raw fragment");
        assert!(pdf1.starts_with(b"%PDF-"));

        // Fragment that contains <body> but no <html> or <head>
        let raw_body = r#"<body class="standalone-body"><h1>Body Fragment</h1><p>Has body tag</p></body>"#;
        let processed_body = process_html(raw_body);
        assert!(processed_body.contains(":where(html, body)"));
        let body_count = processed_body.to_ascii_lowercase().matches("<body").count();
        assert_eq!(body_count, 1, "Must avoid duplicate <body> wrapping");

        let pdf2 = PdfGenerator::from_html(raw_body).expect("Should generate PDF from body fragment");
        assert!(pdf2.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_generator_strips_invisible_unicode_characters() {
        let dirty_html = "<p>Clean\u{200B}Text\u{034F}With\u{E0041}Secret\u{2060}Tags</p>";
        let processed = process_html(dirty_html);
        assert!(!processed.contains('\u{200B}'));
        assert!(!processed.contains('\u{034F}'));
        assert!(!processed.contains('\u{E0041}'));
        assert!(!processed.contains('\u{2060}'));
        assert!(processed.contains("CleanTextWithSecretTags"));
    }

    #[test]
    fn test_generator_explicit_font_family_not_overridden() {
        let html = r#"
        <html>
        <head><style>body { font-family: monospace; }</style></head>
        <body><p>Code block</p></body>
        </html>
        "#;

        let processed = process_html(html);
        assert!(
            !processed.contains(":where(html, body)"),
            "Should not inject fallback font when font-family is declared"
        );
        assert!(processed.contains("font-family: monospace;"));
    }
}
