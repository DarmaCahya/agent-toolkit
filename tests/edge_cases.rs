use std::fs;
use vizier_pdf::{PdfError, PdfGenerator, PdfRasterizer, PdfReadOptions, PdfReader};

fn get_table_pdf() -> Vec<u8> {
    fs::read("corpus/table.pdf").expect("read table.pdf")
}

#[test]
fn test_edge_corrupt_bytes_graceful_error() {
    let garbage = b"THIS IS NOT A VALID PDF AT ALL 1234567890";
    let res = PdfReader::read(garbage, None);
    assert!(res.is_err(), "Must return Err for garbage input");
    match res.unwrap_err() {
        PdfError::Read(msg) => assert!(!msg.is_empty()),
        other => panic!("Expected PdfError::Read, got {:?}", other),
    }

    let render_res = PdfRasterizer::render_page(garbage, 1, 1.0);
    assert!(render_res.is_err());
}

#[test]
fn test_edge_truncated_pdf_header() {
    let truncated = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\n";
    let res = PdfReader::read(truncated, None);
    assert!(res.is_err(), "Must return Err for truncated PDF stream");
}

#[test]
fn test_edge_zero_bytes_input() {
    let empty = b"";
    let res = PdfReader::read(empty, None);
    assert!(res.is_err());

    let render_res = PdfRasterizer::render_page(empty, 1, 1.0);
    assert!(render_res.is_err());
}

#[test]
fn test_edge_out_of_bounds_page_start_returns_explicit_error() {
    let bytes = get_table_pdf();
    let total = PdfReader::page_count(&bytes).unwrap();
    // table.pdf has 1 page. Requesting page_start = 50 must return explicit InvalidPage error (not silent wrong clamp)
    let opts = PdfReadOptions::new().with_pages(50, 10);
    let res = PdfReader::read(&bytes, Some(opts));
    assert!(res.is_err());
    match res.unwrap_err() {
        PdfError::InvalidPage { page: 50, total: t } => assert_eq!(t, total),
        other => panic!("Expected PdfError::InvalidPage, got {:?}", other),
    }

    // Page 0 is also invalid in 1-indexed convention
    let opts_zero = PdfReadOptions::new().with_pages(0, 5);
    assert!(PdfReader::read(&bytes, Some(opts_zero)).is_err());
}

#[test]
fn test_edge_zero_limits_handling() {
    let bytes = get_table_pdf();
    let opts = PdfReadOptions::new().with_pages(1, 1).with_max_chars(0);
    let res = PdfReader::read(&bytes, Some(opts));
    assert!(res.is_err());
    match res.unwrap_err() {
        PdfError::InvalidArgument(msg) => assert!(msg.contains("max_chars must be greater than 0")),
        other => panic!("Expected PdfError::InvalidArgument, got {:?}", other),
    }
}

#[test]
fn test_edge_unwritable_output_file() {
    let bytes = get_table_pdf();
    // Path where root parent is illegal or contains traversal
    let opts = PdfReadOptions::new().with_output_file("/dev/null/impossible_dir_xyz/out.md");
    let res = PdfReader::read(&bytes, Some(opts));
    assert!(res.is_err());
    match res.unwrap_err() {
        PdfError::InvalidArgument(msg) => assert!(msg.contains("sandbox") || msg.contains("prohibited")),
        other => panic!("Expected InvalidArgument, got {:?}", other),
    }

    // Path traversal rejection
    let opts_traversal = PdfReadOptions::new().with_output_file("../../../etc/shadow.md");
    assert!(PdfReader::read(&bytes, Some(opts_traversal)).is_err());

    // Bad extension rejection
    let opts_ext = PdfReadOptions::new().with_output_file("dangerous.exe");
    assert!(PdfReader::read(&bytes, Some(opts_ext)).is_err());
}

#[test]
fn test_password_redaction_in_debug() {
    let opts = PdfReadOptions::new().with_password("my_super_secret_password_123");
    let debug_str = format!("{:?}", opts);
    assert!(debug_str.contains("<redacted>"), "Debug format must redact password");
    assert!(!debug_str.contains("my_super_secret_password_123"), "Debug must not leak plaintext password");
}

#[test]
fn test_edge_rasterizer_scale_and_nan_safety() {
    let bytes = get_table_pdf();

    // Scale 0.0 or negative or NaN: must return explicit InvalidArgument error, never panic
    assert!(PdfRasterizer::render_page(&bytes, 1, 0.0).is_err());
    assert!(PdfRasterizer::render_page(&bytes, 1, -2.5).is_err());
    assert!(PdfRasterizer::render_page(&bytes, 1, f32::NAN).is_err());
    assert!(PdfRasterizer::render_page(&bytes, 1, f32::INFINITY).is_err());

    // Valid positive scale
    let valid_render = PdfRasterizer::render_page(&bytes, 1, 1.0);
    assert!(valid_render.is_ok());
}

#[test]
fn test_edge_empty_query_rejected() {
    let bytes = get_table_pdf();
    assert!(PdfReader::search_keyword(&bytes, "").is_err(), "Empty query must be rejected");
    assert!(PdfReader::search_keyword(&bytes, "   ").is_err(), "Whitespace query must be rejected");
}

#[test]
fn test_edge_generator_rich_unicode_and_svg() {
    let complex_html = r#"
    <!DOCTYPE html>
    <html>
    <body>
        <h1>Math & Complex Glyphs: ∑(x²) + ∫f(x)dx</h1>
        <p>Emojis: 🚀🦀🔥💻📊</p>
        <p>CJK: 日本語, 中文, 한국어</p>
        <p>Arabic: مرحبا بالعالم</p>
        <svg width="100" height="100">
            <circle cx="50" cy="50" r="40" stroke="green" stroke-width="4" fill="yellow" />
        </svg>
    </body>
    </html>
    "#;

    let pdf_bytes = PdfGenerator::from_html(complex_html).expect("Complex HTML generate");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    let image = PdfRasterizer::render_page(&pdf_bytes, 1, 1.0).expect("Render complex PDF");
    assert!(image.width() > 0);
}

#[test]
fn test_keyword_search_across_pages() {
    let bytes = get_table_pdf();
    let matches = PdfReader::search_keyword(&bytes, "Revenue").expect("Search keyword");
    assert!(!matches.is_empty(), "Should find page containing 'Revenue'");
    assert_eq!(matches[0], 1, "Should be on page 1");

    let non_matches = PdfReader::search_keyword(&bytes, "XYZ_NON_EXISTENT_TERM_9999").unwrap();
    assert!(non_matches.is_empty());
}

#[test]
fn test_extract_headings_outline() {
    let bytes = get_table_pdf();
    let headings = PdfReader::extract_headings(&bytes).unwrap();
    assert!(!headings.is_empty(), "table.pdf contains headings");
    assert!(headings.iter().any(|(_, text)| text.contains("Quarterly") || text.starts_with('#')));
}

#[test]
fn test_edge_extract_markdown_budget_guard_on_500_pages() {
    let bytes = std::fs::read("corpus/500_pages.pdf").expect("500_pages.pdf");
    // 500_pages has 1.95M characters, exceeding the 500,000 char budget for extract_markdown
    let res = PdfReader::extract_markdown(&bytes);
    assert!(res.is_err(), "Raw extract_markdown on 500_pages must be rejected to prevent context explosion");
    match res.unwrap_err() {
        PdfError::LimitExceeded(msg) => {
            assert!(msg.contains("exceeds single-turn budget"));
            assert!(msg.contains("output_file"));
        }
        other => panic!("Expected LimitExceeded, got {:?}", other),
    }
}

#[test]
fn test_edge_rasterizer_scale_limits_rejection() {
    let bytes = get_table_pdf();
    assert!(PdfReader::read(&bytes, None).is_ok());

    // Scale < 0.1 rejected
    assert!(PdfRasterizer::render_page(&bytes, 1, 0.05).is_err());
    // Scale > 4.0 rejected
    assert!(PdfRasterizer::render_page(&bytes, 1, 5.0).is_err());
    // Valid scale accepted
    assert!(PdfRasterizer::render_page(&bytes, 1, 1.5).is_ok());
}
