# Agent Toolkit Monorepo

> **Pure-Rust, memory-guarded, and browserless artifact generation engines designed specifically for autonomous AI agents.**

This repository is a **Cargo Workspace Monorepo** hosting high-performance, zero-runtime crates for agentic workflows:

| Crate | Crates.io | Status | Description |
| :--- | :---: | :---: | :--- |
| [**`pdf-agent`**](crates/pdf-agent) | [![crates.io](https://img.shields.io/crates/v/pdf-agent.svg)](https://crates.io/crates/pdf-agent) | 🟢 **v0.1.1 (Live)** | Memory-guarded PDF facade: structured Markdown extraction, pure CPU visual rasterization, and HTML-to-PDF vector generation. |
| [**`agent-canvas`**](crates/agent-canvas) | [![crates.io](https://img.shields.io/crates/v/agent-canvas.svg)](https://crates.io/crates/agent-canvas) | 🟡 **v0.1.0 (source)** | Visual computing & calculation artifacts: flowcharts, business charts, LaTeX math typesetting to vector SVG, Excel formula evaluation, QR codes and KPI cards. |

---

## 🤖 For AI agents and LLM crawlers

- **[`llms.txt`](llms.txt)** — one-page index: capability → API → "why this over the usual alternative".
- **[`llms-full.txt`](llms-full.txt)** — full context: architecture, dependencies, API surface, hard limits, FAQ.
- **Ground rules the artifacts guarantee:** deterministic byte-identical SVG, `<path>`/`<rect>`/`<use>` only (no `<text>`, no `font-family`, no external URLs), typed `thiserror` errors instead of panics, hard input/nesting caps, and no runtime dependency on a browser, Node.js, Python, system fonts or a C toolchain.

---

## 🏛️ Workspace Architecture

```text
.
├── Cargo.toml                  <-- Root Cargo Workspace
├── llms.txt / llms-full.txt    <-- machine-readable context for LLM crawlers
├── crates/
│   ├── pdf-agent/              <-- [Crate: pdf-agent v0.1.1]
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── tests/
│   │
│   └── agent-canvas/           <-- [Crate: agent-canvas v0.1.0]
│       ├── assets/fonts/       <-- bundled, licenced glyph subsets
│       ├── Cargo.toml
│       ├── src/
│       └── tests/
└── scripts/                    <-- reproducible font subsetting
```

---

## 📦 Crates Overview

### 1. `pdf-agent`
- **Why it exists:** Raw PDF extraction frequently causes LLM context overflows and OOM errors. `pdf-agent` introduces built-in **Agent Windowing** (`page_start`, `page_limit`, `max_chars`) and the **File Offload Pattern** (`output_file`), alongside pure-Rust visual page rasterization (`hayro`) and modern HTML/CSS vector PDF generation (`fulgur`).
- **Install:** `cargo add pdf-agent`
- **Documentation:** See [`crates/pdf-agent/README.md`](crates/pdf-agent/README.md) or [docs.rs/pdf-agent](https://docs.rs/pdf-agent).

### 2. `agent-canvas`
- **Why it exists:** AI agents frequently need to generate visual artifacts (decision trees, sales funnels, portfolio donuts, LaTeX formulas, QR codes, KPI cards) without depending on headless browsers (Chromium), Python (matplotlib), Node.js (Mermaid/KaTeX) or C libraries.
- **Capabilities:** `graph` (flowchart DSL + Manhattan routing), `chart` (donut, 2×2 quadrant, funnel, bar, line, waterfall, sparkline), `math` (LaTeX → SVG typesetter and Excel formula evaluator), `component` (QR codes, KPI cards).
- **Install:** `cargo add agent-canvas` (upcoming release)
- **Documentation:** See [`crates/agent-canvas/README.md`](crates/agent-canvas/README.md).

---

## 🛠️ Building & Testing the Workspace

Compile the entire workspace:
```bash
cargo build --workspace
```

Run all unit, contract, snapshot, property and adversarial tests across all crates:
```bash
cargo test --workspace
```

---

## ⚖️ License

Dual-licensed under **MIT** or **Apache-2.0**. Bundled font subsets are third-party assets under their own licences — see [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
Maintained by Darma Cahya (`darmady6@gmail.com`).
