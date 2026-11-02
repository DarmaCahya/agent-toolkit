# pdf-agent

[![crates.io](https://img.shields.io/crates/v/pdf-agent.svg)](https://crates.io/crates/pdf-agent)
[![Documentation](https://docs.rs/pdf-agent/badge.svg)](https://docs.rs/pdf-agent)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

A pure-Rust, memory-guarded PDF facade engineered specifically for **AI Agent Frameworks & LLM Pipelines**.

## Why pdf-agent?

Feeding raw PDFs into LLM context windows frequently leads to **context blowouts**, **out-of-memory (OOM) crashes**, and **high latency**. Traditional tools either depend on heavy C++ runtimes (`Poppler`, `PDFium`) or headless Chromium (~800MB+).

`pdf-agent` provides a lightweight, pure-Rust facade combining three world-class engines with hardened **Agent Protection Guards**:

1. **Structured Reading (`pdf_oxide`)**: Fast Markdown & GFM table reconstruction with built-in token budgets and windowing.
2. **Visual Rasterization (`hayro`)**: Pure-Rust CPU rasterizer to render pages directly to RGBA/PNG for Vision/Multimodal LLMs without Poppler.
3. **Document Generation (`fulgur`)**: Full modern HTML/CSS layout engine generating crisp vector PDFs.

---

## Agent-Specific Protections

- 🛡️ **Context Window Guard (`page_start`, `page_limit`, `max_chars`)**: Read 10-20 pages at a time to prevent overflowing LLM context.
- 💾 **File Offload Pattern (`output_file`)**: Stream 100% of large documents (400+ pages) directly to disk in seconds, returning only a compact metadata preview (~400 chars) to the LLM.
- 🔍 **Scanned Document Detection (`scanned_likely`)**: Automatically alerts the agent when a PDF is image-only so it can switch to multimodal vision analysis.
- 🔒 **Sandboxed & Race-Free Execution**: Path validation blocking directory traversal (`..`) and symlink attacks, with atomic writes safe for concurrent multi-agent environments.
- ⚡ **Pre-Render Memory Checks**: Guards against decompression bombs and runaway allocations before invoking image rasterization.

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
pdf-agent = "0.1.0"
```

### Feature Flags

Customize which engines are compiled to keep binaries lean:

```toml
# Default includes all features: reader, render, writer
pdf-agent = { version = "0.1.0" }

# Read-only (minimal dependencies & lightning-fast compile)
pdf-agent = { version = "0.1.0", default-features = false, features = ["reader"] }
```

---

## Quick Start

### 1. Windowed Reading (Safe for LLM Context)

```rust
use pdf_agent::{PdfReader, PdfReadOptions};

let pdf_bytes = std::fs::read("annual_report.pdf")?;

// Windowed read: pages 1 to 10, max 100,000 characters
let options = PdfReadOptions::new()
    .with_pages(1, 10)
    .with_max_chars(100_000);

let result = PdfReader::read(&pdf_bytes, Some(options))?;
println!("Extracted Markdown:\n{}", result.markdown);

if result.truncated {
    println!("Next page to read: {:?}", result.next_page_start);
}
```

### 2. File Offload (For 100% Full Document Extraction)

```rust
use pdf_agent::{PdfReader, PdfReadOptions};

let pdf_bytes = std::fs::read("huge_500_pages.pdf")?;

// Offload full text directly to disk
let options = PdfReadOptions::new()
    .with_output_file("extracted_docs/huge.md");

let result = PdfReader::read(&pdf_bytes, Some(options))?;
// result.markdown contains only a lightweight ~400 char preview
println!("Full document saved safely to: {:?}", result.saved_to);
```

### 3. Rendering Pages to PNG for Vision Models

```rust
use pdf_agent::PdfRasterizer;

let pdf_bytes = std::fs::read("diagram.pdf")?;
// Render page 1 at 2.0x scale directly to PNG bytes
let png_bytes = PdfRasterizer::render_page_to_png(&pdf_bytes, 1, 2.0)?;
std::fs::write("page_1.png", png_bytes)?;
```

### 4. HTML/CSS to Vector PDF

```rust
use pdf_agent::PdfGenerator;

let html = r#"
    <html>
      <body style="font-family: sans-serif; padding: 20px;">
        <h1 style="color: #2563eb;">Invoice #1042</h1>
        <p>Generated autonomously by AI Agent.</p>
      </body>
    </html>
"#;

let pdf_bytes = PdfGenerator::from_html(html)?;
std::fs::write("invoice.pdf", pdf_bytes)?;
```

---

## Acknowledgements

`pdf-agent` is a high-level facade built on top of excellent pure-Rust libraries:
- [`pdf_oxide`](https://crates.io/crates/pdf_oxide) by the pdf_oxide team.
- [`hayro`](https://crates.io/crates/hayro) by the vigia / typst community.
- [`fulgur`](https://crates.io/crates/fulgur) (powered by Blitz & Krilla).

---

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.
