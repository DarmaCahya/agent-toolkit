use pdf_agent::{PdfGenerator, PdfRasterizer, PdfReadOptions, PdfReader};
use std::fs;
use std::path::Path;

fn get_corpus_file(name: &str) -> Vec<u8> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let manifest_path = Path::new(manifest_dir).join("corpus").join(name);
    if manifest_path.exists() {
        return fs::read(&manifest_path).unwrap();
    }
    let local = Path::new("corpus").join(name);
    if local.exists() {
        return fs::read(&local).unwrap();
    }
    let crate_local = Path::new("crates/pdf-agent/corpus").join(name);
    if crate_local.exists() {
        return fs::read(&crate_local).unwrap();
    }
    let global = Path::new("/srv/hermes/workspace/pdf-benchmarks/corpus").join(name);
    if global.exists() {
        return fs::read(&global).unwrap();
    }
    panic!("Corpus file not found: {}", name);
}

#[test]
fn test_pdf_generation_contract() {
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head><style>body { font-family: sans-serif; }</style></head>
    <body>
        <h1>Vizier PDF Engine Contract Test</h1>
        <p>Testing HTML to PDF generation without leaking upstream types.</p>
    </body>
    </html>
    "#;

    let pdf_bytes = PdfGenerator::from_html(html).expect("Generation should succeed");
    assert!(!pdf_bytes.is_empty());
    assert!(
        pdf_bytes.starts_with(b"%PDF-"),
        "Must be a valid PDF format"
    );
}

#[test]
fn test_pdf_reading_markdown_contract() {
    let bytes = get_corpus_file("table.pdf");
    let pages = PdfReader::page_count(&bytes).expect("Page count");
    assert!(pages >= 1);

    let md = PdfReader::extract_markdown(&bytes).expect("Extract markdown");
    assert!(!md.is_empty());
    assert!(
        md.contains('|'),
        "Should reconstruct markdown table with pipe characters"
    );
}

#[test]
fn test_structured_reading_with_agent_guards() {
    let bytes = get_corpus_file("500_pages.pdf");
    let total = PdfReader::page_count(&bytes).expect("Page count");
    assert_eq!(total, 434);

    // Test 1: Windowed read (only first 3 pages)
    let opts = PdfReadOptions::new()
        .with_pages(1, 3)
        .with_max_chars(100_000);
    let res = PdfReader::read(&bytes, Some(opts)).expect("Windowed read");
    assert_eq!(res.page_start, 1);
    assert_eq!(res.page_end, 3);
    assert_eq!(res.total_pages, 434);
    assert!(
        res.truncated,
        "Must be truncated because only 3 of 434 pages read"
    );
    assert!(res
        .note
        .as_ref()
        .unwrap()
        .contains("Showing pages 1-3 of 434"));
    assert!(!res.scanned_likely, "500_pages has real text");

    // Test 2: Character budget cap (e.g. 200 chars)
    let budget_opts = PdfReadOptions::new().with_pages(1, 5).with_max_chars(200);
    let budget_res = PdfReader::read(&bytes, Some(budget_opts)).expect("Budget read");
    assert!(budget_res.markdown.chars().count() <= 200);
    assert!(budget_res.truncated);
    assert!(budget_res
        .note
        .as_ref()
        .unwrap()
        .contains("character limit"));
}

#[test]
fn test_scanned_pdf_detection_heuristics() {
    // Generate a minimal PDF that has an image or almost zero text
    let empty_html = "<html><body><div style='width:100px; height:100px;'></div></body></html>";
    let empty_pdf = PdfGenerator::from_html(empty_html).expect("Generate empty PDF");

    let res = PdfReader::read(&empty_pdf, None).expect("Read empty PDF");
    assert!(
        res.scanned_likely,
        "Should detect empty text layer as scanned/image-only"
    );
    assert!(res.note.as_ref().unwrap().contains("pdf_render"));
}

#[test]
fn test_full_document_file_offload() {
    let bytes = get_corpus_file("table.pdf");
    let out_file = "tests/table_extracted.md";
    let _ = fs::remove_file(out_file);

    let opts = PdfReadOptions::new().with_output_file(out_file);

    let res = PdfReader::read(&bytes, Some(opts)).expect("Read with file offload");
    assert!(!res.truncated);
    assert_eq!(res.saved_to.as_deref(), Some(out_file));
    assert!(res.total_chars.unwrap_or(0) > 0);
    assert!(Path::new(out_file).exists());

    let content = fs::read_to_string(out_file).expect("Read back offloaded file");
    assert!(
        content.contains('|'),
        "Extracted file must contain table pipes"
    );

    let _ = fs::remove_file(out_file);
}

#[test]
fn test_pdf_rasterization_contract() {
    let bytes = get_corpus_file("table.pdf");
    let image = PdfRasterizer::render_page(&bytes, 1, 1.0).expect("Render page 1");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
    assert!(!image.as_rgba().is_empty());

    let png_bytes = image.to_png_bytes().expect("Encode PNG");
    assert!(
        png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "Must be valid PNG header"
    );
}

#[test]
fn test_full_roundtrip_contract() {
    let sample_html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <style>
            table, th, td { border: 1px solid black; border-collapse: collapse; padding: 4px; }
        </style>
    </head>
    <body>
        <h2>Quarterly Summary</h2>
        <table>
            <tr><th>Metric</th><th>Score</th></tr>
            <tr><td>Alpha</td><td>95%</td></tr>
            <tr><td>Beta</td><td>98%</td></tr>
        </table>
    </body>
    </html>
    "#;

    // 1. Generate
    let pdf_bytes = PdfGenerator::from_html(sample_html).expect("Generate PDF");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    // 2. Read back
    let md = PdfReader::extract_markdown(&pdf_bytes).expect("Read Markdown");
    assert!(md.contains("Quarterly Summary") || md.contains("Alpha"));

    // 3. Rasterize
    let png = PdfRasterizer::render_page_to_png(&pdf_bytes, 1, 1.5).expect("Rasterize PNG");
    assert!(png.starts_with(b"\x89PNG"));
}
