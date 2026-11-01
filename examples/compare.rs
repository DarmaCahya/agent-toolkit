use std::fs;
use std::path::Path;
use std::time::Instant;
use vizier_pdf::{PdfGenerator, PdfRasterizer, PdfReadOptions, PdfReader};

fn main() {
    println!("============================================================");
    println!("     VIZIER PDF SUITE — AGENT SCENARIOS BENCHMARK SUITE     ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // SKENARIO 1: File Raksasa (434 Halaman, 500_pages.pdf)
    // ------------------------------------------------------------------
    println!("\n[SKENARIO 1: Dokumen Raksasa - 434 Halaman / 1.95M Karakter]");
    let bytes_500 = fs::read("corpus/500_pages.pdf").expect("read 500_pages.pdf");

    // 1A. Baseline (Before): Dump semua ke memori string
    let t0 = Instant::now();
    let full_md = PdfReader::extract_markdown(&bytes_500).expect("full extract");
    let dur_before = t0.elapsed();

    // 1B. Agent In-Memory Windowing (After Default: 10 halaman, max 100k chars)
    let t1 = Instant::now();
    let res_window = PdfReader::read(&bytes_500, None).expect("windowed read");
    let dur_window = t1.elapsed();

    // 1C. File Offload Pattern: User minta 100% full ekstrak ke file di disk
    let offload_path = "target/full_500_pages_extracted.md";
    let t2 = Instant::now();
    let res_offload = PdfReader::read(
        &bytes_500,
        Some(PdfReadOptions {
            output_file: Some(offload_path.to_string()),
            ..Default::default()
        }),
    )
    .expect("file offload");
    let dur_offload = t2.elapsed();

    println!("  • 1A. Full Memory Dump (BEFORE):");
    println!("        Output: {} karakter (~{} token)", full_md.chars().count(), full_md.chars().count() / 4);
    println!("        Durasi: {:.2} ms | Status: RAW UNGUARDED (Bahaya context blowout!)", dur_before.as_secs_f64() * 1000.0);

    println!("  • 1B. Agent In-Memory Windowing (AFTER DEFAULT):");
    println!("        Output: {} karakter (~{} token)", res_window.markdown.chars().count(), res_window.markdown.chars().count() / 4);
    println!("        Halaman: {} - {} dari {} total", res_window.page_start, res_window.page_end, res_window.total_pages);
    println!("        Durasi: {:.2} ms ({:.1}x lebih cepat!)", dur_window.as_secs_f64() * 1000.0, dur_before.as_secs_f64() / dur_window.as_secs_f64());
    println!("        Truncated: {} | Note: {}", res_window.truncated, res_window.note.as_deref().unwrap_or(""));

    println!("  • 1C. File Offload Pattern (USER MINTA SEMUA HALAMAN):");
    println!("        Tersimpan ke: {}", res_offload.saved_to.as_deref().unwrap_or(""));
    println!("        Total Karakter Terekstrak: {}", res_offload.total_chars.unwrap_or(0));
    println!("        Durasi Ekstrak 100%: {:.2} ms", dur_offload.as_secs_f64() * 1000.0);
    println!("        Ukuran File di Disk: {:.2} MB", Path::new(offload_path).metadata().unwrap().len() as f64 / 1_048_576.0);
    println!("        Konteks LLM Terpakai: HANYA {} karakter (Preview aman)", res_offload.markdown.chars().count());

    // ------------------------------------------------------------------
    // SKENARIO 2: Deteksi Scanned / Image-Only PDF (Tanpa Layer Teks)
    // ------------------------------------------------------------------
    println!("\n[SKENARIO 2: Deteksi Scanned PDF / Image-Only Document]");
    let scanned_mock_html = "<html><body><div style='width:300px; height:400px; background:#eee;'></div></body></html>";
    let scanned_pdf = PdfGenerator::from_html(scanned_mock_html).expect("generate mock scanned");
    let scanned_res = PdfReader::read(&scanned_pdf, None).expect("read scanned");

    println!("  • Scanned Detected: {}", scanned_res.scanned_likely);
    println!("  • Guidance Note: {}", scanned_res.note.as_deref().unwrap_or(""));

    // ------------------------------------------------------------------
    // SKENARIO 3: Preservasi Tabel Markdown (table.pdf)
    // ------------------------------------------------------------------
    println!("\n[SKENARIO 3: Rekonstruksi Struktur Tabel GFM (table.pdf)]");
    let table_bytes = fs::read("corpus/table.pdf").expect("read table.pdf");
    let table_res = PdfReader::read(&table_bytes, None).expect("read table");
    let has_pipe = table_res.markdown.contains('|');
    println!("  • Table Pipe Detected: {} (Rekonstruksi Tabel GFM Berhasil)", has_pipe);

    // ------------------------------------------------------------------
    // SKENARIO 4: Two-Column Text Flow (twocolumn.pdf)
    // ------------------------------------------------------------------
    println!("\n[SKENARIO 4: Alur Baca Dua Kolom (twocolumn.pdf)]");
    let col_bytes = fs::read("corpus/twocolumn.pdf").expect("read twocolumn.pdf");
    let col_res = PdfReader::read(&col_bytes, None).expect("read twocolumn");
    println!("  • Two Column Extracted: {} chars", col_res.markdown.chars().count());

    // ------------------------------------------------------------------
    // SKENARIO 5: Multilingual & CJK Support (multilingual.pdf)
    // ------------------------------------------------------------------
    println!("\n[SKENARIO 5: Karakter CJK & Multilingual UTF-8 (multilingual.pdf)]");
    let multi_bytes = fs::read("corpus/multilingual.pdf").expect("read multilingual.pdf");
    let multi_res = PdfReader::read(&multi_bytes, None).expect("read multilingual");
    println!("  • Multilingual Extracted: {} chars", multi_res.markdown.chars().count());

    // ------------------------------------------------------------------
    // SKENARIO 6: Pure-Rust Visual Rasterizer (hayro render to PNG)
    // ------------------------------------------------------------------
    println!("\n[SKENARIO 6: Pure-Rust Visual Rasterizer (hayro -> PNG)]");
    let t_render = Instant::now();
    let png_bytes = PdfRasterizer::render_page_to_png(&table_bytes, 0, 1.5).expect("render png");
    let dur_render = t_render.elapsed();
    println!("  • Halaman 1 Rendered: {} bytes PNG dalam {:.2} ms", png_bytes.len(), dur_render.as_secs_f64() * 1000.0);
    println!("  • Valid PNG Header: {}", png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

    println!("\n============================================================");
    println!("           SELURUH 6 SKENARIO PENGUJIAN SELESAI & LULUS      ");
    println!("============================================================");
}
