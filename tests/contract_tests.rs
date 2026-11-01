use std::fs;
use std::path::Path;
use vizier_pdf::{PdfGenerator, PdfRasterizer, PdfReader};

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
    assert!(pdf_bytes.starts_with(b"%PDF-"), "Must be a valid PDF format");
}

#[test]
fn test_pdf_reading_markdown_contract() {
    let corpus_path = Path::new("/srv/hermes/workspace/pdf-benchmarks/corpus/table.pdf");
    if corpus_path.exists() {
        let bytes = fs::read(corpus_path).expect("Read table.pdf");
        let pages = PdfReader::page_count(&bytes).expect("Page count");
        assert!(pages >= 1);

        let md = PdfReader::extract_markdown(&bytes).expect("Extract markdown");
        assert!(!md.is_empty());
        // Verify markdown table syntax
        assert!(md.contains('|'), "Should reconstruct markdown table with pipe characters");
    }
}

#[test]
fn test_pdf_rasterization_contract() {
    let corpus_path = Path::new("/srv/hermes/workspace/pdf-benchmarks/corpus/table.pdf");
    if corpus_path.exists() {
        let bytes = fs::read(corpus_path).expect("Read table.pdf");
        let image = PdfRasterizer::render_page(&bytes, 0, 1.0).expect("Render page 0");
        assert!(image.width > 0);
        assert!(image.height > 0);
        assert!(!image.raw_rgba.is_empty());

        let png_bytes = image.to_png_bytes().expect("Encode PNG");
        assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "Must be valid PNG header");
    }
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
    let png = PdfRasterizer::render_page_to_png(&pdf_bytes, 0, 1.5).expect("Rasterize PNG");
    assert!(png.starts_with(b"\x89PNG"));
}
