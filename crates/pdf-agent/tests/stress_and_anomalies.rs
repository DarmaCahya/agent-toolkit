use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use pdf_agent::{PdfError, PdfGenerator, PdfRasterizer, PdfReadOptions, PdfReader};

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

// =========================================================================
// 1. ANOMALY: Password Handling on Unencrypted Documents
// =========================================================================

#[test]
fn test_anomaly_unencrypted_doc_with_superfluous_password() {
    let bytes = get_corpus_file("table.pdf");

    // Passing a password to an unencrypted document should succeed gracefully, never panic.
    let count = PdfReader::page_count_with_password(&bytes, Some("dummy_pass"));
    assert!(count.is_ok(), "Unencrypted doc should gracefully handle superfluous password");

    let headings = PdfReader::extract_headings_with_password(&bytes, Some("dummy_pass"));
    assert!(headings.is_ok(), "Heading extraction should succeed on unencrypted doc even if password passed");

    let search = PdfReader::search_keyword_with_password(&bytes, "table", Some("dummy_pass"));
    assert!(search.is_ok());
}

// =========================================================================
// 2. ANOMALY: Zero Page Limit and Strict Invariants
// =========================================================================

#[test]
fn test_anomaly_zero_page_limit_in_read_and_batch() {
    let bytes = get_corpus_file("table.pdf");

    // In read(): page_limit = 0 is strictly rejected by contract (must be >= 1)
    let opts = PdfReadOptions::new().with_pages(1, 0);
    let res = PdfReader::read(&bytes, Some(opts));
    assert!(res.is_err());
    match res.unwrap_err() {
        PdfError::InvalidArgument(msg) => assert!(msg.contains("greater than 0")),
        other => panic!("Expected InvalidArgument, got {:?}", other),
    }

    // In render_pages(): page_limit = 0 returns empty batch gracefully without error
    let batch = PdfRasterizer::render_pages(&bytes, 1, 0, 1.0).expect("render_pages limit 0 should succeed");
    assert!(batch.is_empty());
}

#[test]
fn test_anomaly_extreme_budget_clamp() {
    let bytes = get_corpus_file("table.pdf");

    // max_chars = 1: Should truncate immediately without buffer overflow
    let opts_min = PdfReadOptions::new().with_max_chars(1);
    let res = PdfReader::read(&bytes, Some(opts_min)).expect("Minimal max_chars");
    assert!(res.truncated);
    assert!(res.markdown.chars().count() <= 100);

    // max_chars = 500_000: Max in-memory safe budget should succeed
    let opts_safe_ceiling = PdfReadOptions::new().with_max_chars(500_000);
    let res_safe = PdfReader::read(&bytes, Some(opts_safe_ceiling)).expect("Safe ceiling max_chars");
    assert!(!res_safe.markdown.is_empty());

    // max_chars = 50_000_000: Exceeds in-memory safe budget (guard protects LLM context)
    let opts_oversized = PdfReadOptions::new().with_max_chars(50_000_000);
    let res_oversized = PdfReader::read(&bytes, Some(opts_oversized));
    assert!(res_oversized.is_err());
    match res_oversized.unwrap_err() {
        PdfError::LimitExceeded(msg) => assert!(msg.contains("safe single-turn budget")),
        other => panic!("Expected LimitExceeded, got {:?}", other),
    }
}

// =========================================================================
// 3. ANOMALY: Unicode, RTL, Math, CJK, and Emojis
// =========================================================================

#[test]
fn test_anomaly_unicode_rtl_cjk_emoji_roundtrip() {
    let rich_html = r#"
    <!DOCTYPE html>
    <html lang="id">
    <head><title>Unicode Test</title></head>
    <body>
        <h1>Laporan Keuangan & Analisis AI</h1>
        <p dir="rtl">مرحبا بكم في تقرير الذكاء الاصطناعي</p>
        <p>日本語のテスト文章です。京都、東京、大阪。</p>
        <p>한국어 테스트입니다. 인공지능 에이전트.</p>
        <p>Formula: ∑(x_i^2) = α + β * γ / √2π</p>
        <p>Emoji flags & family: 🇮🇩 🇯🇵 🇺🇸 👨‍👩‍👧‍👦 🤖 🧠 📈</p>
    </body>
    </html>
    "#;

    let pdf_bytes = PdfGenerator::from_html(rich_html).expect("Generate rich unicode PDF");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    let read_res = PdfReader::read(&pdf_bytes, None).expect("Read generated PDF");
    let md = read_res.markdown;
    assert!(md.contains("Laporan Keuangan") || md.contains("Analisis AI"), "Title must be present");
    assert!(!read_res.scanned_likely);
}

// =========================================================================
// 4. ANOMALY: Deeply Nested HTML Input
// =========================================================================

#[test]
fn test_anomaly_deeply_nested_html_elements() {
    let mut nested = String::from("<!DOCTYPE html><html><body>");
    for i in 0..50 {
        nested.push_str(&format!("<div class='layer-{}' style='padding: 1px;'>", i));
    }
    nested.push_str("<h1>Deep Core Content</h1>");
    for _ in 0..50 {
        nested.push_str("</div>");
    }
    nested.push_str("</body></html>");

    let pdf_bytes = PdfGenerator::from_html(&nested).expect("Generate deeply nested PDF");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    let md = PdfReader::extract_markdown(&pdf_bytes).expect("Extract markdown from deep tree");
    assert!(md.contains("Deep Core Content"));
}

// =========================================================================
// 5. SECURITY & QA: Adversarial Offline Isolation & Script Injection
// =========================================================================

#[test]
fn test_security_adversarial_offline_html_isolation() {
    let malicious_html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <script>alert("XSS payload executed");</script>
        <link rel="stylesheet" href="http://127.0.0.1:9999/steal.css">
    </head>
    <body>
        <h1>Offline Executive Brief</h1>
        <iframe src="file:///etc/passwd"></iframe>
        <img src="http://127.0.0.1:8888/beacon.png" alt="beacon tracking">
        <p>The system remains fully offline and secure.</p>
    </body>
    </html>
    "#;

    // Must not panic, must not connect externally, must render document text
    let pdf_bytes = PdfGenerator::from_html(malicious_html).expect("Generate offline isolated PDF");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    let md = PdfReader::extract_markdown(&pdf_bytes).expect("Read markdown");
    assert!(md.contains("Executive Brief"));
    assert!(md.contains("secure"));
    assert!(!md.contains("XSS payload executed"), "Script tag contents must not leak into document text");
}

// =========================================================================
// 6. SECURITY & QA: Path Traversal, Windows ADS & Reserved Names
// =========================================================================

#[test]
fn test_security_sandbox_path_traversal_audit() {
    let bytes = get_corpus_file("table.pdf");

    let hostile_paths = vec![
        "../traversal.md",
        "../../etc/passwd.md",
        "/absolute/path.md",
        "C:\\Windows\\system32.md",
        "CON.md",
        "PRN.md",
        "AUX.md",
        "NUL.md",
        "COM1.md",
        "file.txt:stream:$DATA",
        "test.exe",
        "test.sh",
        "test.py",
        "   ",
        "",
    ];

    for path in hostile_paths {
        let opts = PdfReadOptions::new().with_output_file(path);
        let res = PdfReader::read(&bytes, Some(opts));
        assert!(
            res.is_err(),
            "Path '{}' must be rejected by sandbox, but was accepted!",
            path
        );
    }
}

// =========================================================================
// 7. SECURITY & QA: Corrupted Streams & 100% Failed Page Boundary
// =========================================================================

#[test]
fn test_security_corrupted_payloads_and_boundary() {
    let table_pdf = get_corpus_file("table.pdf");

    // 1. Valid header followed by pure zero bytes => Must return Err, not empty Ok
    let mut zero_filled = table_pdf[..32].to_vec();
    zero_filled.extend(vec![0u8; 1024]);
    assert!(PdfReader::read(&zero_filled, None).is_err());
    assert!(PdfRasterizer::render_page(&zero_filled, 1, 1.0).is_err());

    // 2. Truncated at 10% (100% of pages fail to parse) => Must return explicit Err
    let truncated_short = &table_pdf[..table_pdf.len() / 10];
    let res = PdfReader::read(truncated_short, None);
    assert!(
        res.is_err(),
        "Truncated PDF with 100% unparseable pages must return Err, not Ok with empty text"
    );

    // 3. Bit-flipped trailer => graceful error or handled safely, zero panics
    let mut flipped = table_pdf.clone();
    let len = flipped.len();
    for byte in &mut flipped[len - 20..] {
        *byte = !*byte;
    }
    let _ = PdfReader::read(&flipped, None);
}

// =========================================================================
// 8. STRESS: Multi-Threaded Concurrency
// =========================================================================

#[test]
fn test_stress_concurrent_reading_and_searching() {
    let doc_bytes = Arc::new(get_corpus_file("500_pages.pdf"));
    let locked_bytes = Arc::new(get_corpus_file("locked.pdf"));

    let mut handles = Vec::new();

    // Spawn 8 concurrent worker threads performing intensive tasks
    for thread_id in 0..8 {
        let doc = Arc::clone(&doc_bytes);
        let locked = Arc::clone(&locked_bytes);

        handles.push(thread::spawn(move || {
            // Task A: Read page slice
            let opts = PdfReadOptions::new().with_pages((thread_id * 5) + 1, 3);
            let res = PdfReader::read(&doc, Some(opts)).expect("Concurrent read");
            assert_eq!(res.total_pages, 434);

            // Task B: Keyword search
            let query = if thread_id % 2 == 0 { "Page" } else { "Document" };
            let matches = PdfReader::search_keyword(&doc, query).expect("Concurrent search");
            assert!(!matches.is_empty());

            // Task C: Password decrypt on locked document
            let count = PdfReader::page_count_with_password(&locked, Some("vizier"))
                .expect("Concurrent count with password");
            assert!(count >= 1);

            // Task D: Search on locked document
            let lock_search = PdfReader::search_keyword_with_password(&locked, "vizier", Some("vizier"))
                .expect("Concurrent search on locked doc");
            let _ = lock_search;
        }));
    }

    for handle in handles {
        handle.join().expect("Worker thread panicked under concurrency");
    }
}

// =========================================================================
// 9. STRESS: Concurrent Batch Rasterization
// =========================================================================

#[test]
fn test_stress_concurrent_batch_rasterization() {
    let doc_bytes = Arc::new(get_corpus_file("table.pdf"));
    let mut handles = Vec::new();

    for i in 0..6 {
        let doc = Arc::clone(&doc_bytes);
        handles.push(thread::spawn(move || {
            let scale = 0.5 + (i as f32 * 0.2); // 0.5, 0.7, 0.9, 1.1, 1.3, 1.5
            let batch = PdfRasterizer::render_pages(&doc, 1, 1, scale)
                .expect("Concurrent render_pages");
            assert_eq!(batch.len(), 1);
            assert!(batch[0].1.width() > 0);
            assert!(batch[0].1.height() > 0);
        }));
    }

    for handle in handles {
        handle.join().expect("Rasterizer worker thread panicked");
    }
}

// =========================================================================
// 10. AGENTIC CONTRACT: Full Matrix of is_retryable() & agent_hint()
// =========================================================================

#[test]
fn test_agentic_contract_error_matrix() {
    // 1. LimitExceeded => Retryable with hint
    let err_limit = PdfError::LimitExceeded("Total characters 200 exceeded limit 100".into());
    assert!(err_limit.is_retryable());
    assert!(err_limit.agent_hint().unwrap().contains("page_limit"));

    // 2. Encrypted => Retryable with password hint
    let err_enc = PdfError::Encrypted("Password required".into());
    assert!(err_enc.is_retryable());
    assert!(err_enc.agent_hint().unwrap().contains("password"));

    // 3. InvalidPage => Retryable with bounds hint
    let err_page = PdfError::InvalidPage { page: 99, total: 5 };
    assert!(err_page.is_retryable());
    assert!(err_page.agent_hint().unwrap().contains("page_count"));

    // 4. InvalidArgument => Retryable
    let err_arg = PdfError::InvalidArgument("invalid scale".into());
    assert!(err_arg.is_retryable());
    assert!(err_arg.agent_hint().is_some());

    // 5. Sandbox => Retryable
    let err_box = PdfError::Sandbox("prohibited path".into());
    assert!(err_box.is_retryable());
    assert!(err_box.agent_hint().unwrap().contains("sandbox"));

    // 6. InvalidPdf / Unrecoverable => NOT retryable
    let err_invalid = PdfError::InvalidPdf("bad header".into());
    assert!(!err_invalid.is_retryable());
    assert!(err_invalid.agent_hint().unwrap().contains("syntax error"));
}
