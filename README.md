# Unified PDF Crate (`pdf-suite` / `vizier_pdf`)

> **Pure-Rust, browserless PDF engine designed specifically for autonomous AI agents.**  
> Unified facade combining high-speed Markdown extraction (`pdf_oxide`), pure CPU rasterization (`hayro`), and headless HTML-to-PDF compilation (`fulgur`).

---

## 🌟 Key Features

1. **Zero Upstream Type Leakage (Facade Pattern):**
   * Encapsulates all third-party internal types behind a clean public API (`PdfReader`, `PdfRasterizer`, `PdfGenerator`, `PageImage`, `PdfError`).
2. **Context Window Protection (Windowing & Token Guards):**
   * Paginates huge documents with `page_start`, `page_limit`, and `max_chars` constraints to prevent LLM context blowouts.
3. **Smart Scanned PDF Detection:**
   * Automatically detects image-only / scanned documents (`scanned_likely: true`) and directs agents to visual inspection via `pdf_render`.
4. **Headless Font Fallback:**
   * Injects system-neutral fallback font declarations to eliminate tofu/empty boxes in headless Docker or musl scratch containers.
5. **Feature-Gated Modularity:**
   * Fine-grained Cargo features (`reader`, `render`, `writer`) enable compiling reader-only or rasterizer-only binaries in seconds without full CSS engine bloat.

---

## 🗄️ Large Document Handling & Cache Lifecycle

When handling massive documents (100–500+ pages), the engine enforces the **Dual-Tier Storage Pattern**:

```text
User / Agent Request
         │
         ├──> [A] Small/Targeted Read ──> In-Memory Paging (10 pages / max 100k chars) ──> LLM Context (Aman)
         │
         └──> [B] Full 100% Extraction (output_file)
                     │
                     ├──> User Request ("simpan ke buku.md")
                     │       └──> User Artifact (Permanent storage in workspace / vault)
                     │
                     └──> System Offload (Auto-downgraded due to size)
                             └──> Ephemeral Cache (/tmp/ or scratch session)
                                     ├── Auto-prune saat sesi chat ditutup
                                     ├── TTL Expiry: 24 jam tanpa akses
                                     └── Emergency purge saat disk VPS < 1 GB
```

### Storage Distinction:
1. **User Artifacts (Permanent):**
   * Triggered when the user explicitly requests an output file path.
   * Saved permanently in the user's project workspace or Obsidian Vault.
2. **Ephemeral System Cache (Auto-Pruned):**
   * Created when reading large files to keep LLM context light.
   * Stored in `/tmp/` or session scratch directories.
   * Auto-deleted on session close, subject to a 24-hour TTL, and immediately swept if host disk capacity falls below 1 GB.

---

## 📊 Empirical Benchmarks (Tested on 434-Page Corpus / 1.95M Chars)

| Metric | Before (Raw Dump) | After (Windowed Paging) | After (File Offload) |
| :--- | :--- | :--- | :--- |
| **Output Characters** | 1,952,440 chars | **39,929 chars** | **1,952,440 chars** (in file) |
| **LLM Token Burden** | ~488,110 tokens 💥 | **~9,982 tokens** 🛡️ | **~100 tokens** (metadata preview) |
| **Execution Duration** | 17.0 seconds | **1.21 seconds (14x faster)** | 17.6 seconds (100% full extract) |
| **Disk File Created** | None | None | 1.86 MB clean Markdown |
| **Context Safety** | Out-of-Memory / Blowout | **Safe within standard turn** | **Zero context impact** |

---

## 📦 Quick Start & Usage

```rust
use vizier_pdf::{PdfReader, PdfReadOptions, PdfRasterizer, PdfGenerator};

// 1. Structured In-Memory Reading (Agent Default)
let bytes = std::fs::read("report.pdf")?;
let res = PdfReader::read(&bytes, Some(PdfReadOptions {
    page_start: Some(1),
    page_limit: Some(10),
    max_chars: Some(100_000),
    ..Default::default()
}))?;

println!("Pages {}-{} of {}", res.page_start, res.page_end, res.total_pages);
if res.scanned_likely {
    println!("Scanned document detected. Rendering visual image...");
    let png = PdfRasterizer::render_page_to_png(&bytes, 0, 1.5)?;
    std::fs::write("page_1.png", png)?;
}

// 2. Full Document Offload to File
let full_res = PdfReader::read(&bytes, Some(PdfReadOptions {
    output_file: Some("extracted_full.md".to_string()),
    ..Default::default()
}))?;
println!("Saved {} chars to {}", full_res.total_chars.unwrap(), full_res.saved_to.unwrap());

// 3. Generate PDF from HTML
let html = "<h1>Official Report</h1><p>Generated in pure Rust.</p>";
let pdf_bytes = PdfGenerator::from_html(html)?;
std::fs::write("output.pdf", pdf_bytes)?;
```

---

## ⚖️ License
Dual-licensed under **MIT** or **Apache-2.0**.
*(Note: Fulgur incorporates Blitz components under MPL-2.0. Ensure `licenses.allow` includes MPL-2.0 in CI).*
