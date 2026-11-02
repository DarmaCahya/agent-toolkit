# Agent Toolkit Monorepo

> **Pure-Rust, memory-guarded, and browserless artifact generation engines designed specifically for autonomous AI agents.**

This repository is a **Cargo Workspace Monorepo** hosting high-performance, zero-runtime crates for agentic workflows:

| Crate | Crates.io | Status | Description |
| :--- | :---: | :---: | :--- |
| [**`pdf-agent`**](crates/pdf-agent) | [![crates.io](https://img.shields.io/crates/v/pdf-agent.svg)](https://crates.io/crates/pdf-agent) | 🟢 **v0.1.0 (Live)** | Memory-guarded PDF facade: structured Markdown extraction, pure CPU visual rasterization, and HTML-to-PDF vector generation. |
| [**`agent-canvas`**](crates/agent-canvas) | [![crates.io](https://img.shields.io/crates/v/agent-canvas.svg)](https://crates.io/crates/agent-canvas) | 🟡 **v0.1.0 (In Progress)** | Visual computing & calculation artifacts: flowcharts, business charts, LaTeX math typesetting, and Excel formula evaluation. |

---

## 🏛️ Workspace Architecture

```text
.
├── Cargo.toml                  <-- Root Cargo Workspace
├── crates/
│   ├── pdf-agent/              <-- [Crate: pdf-agent v0.1.0]
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── tests/
│   │
│   └── agent-canvas/           <-- [Crate: agent-canvas v0.1.0]
│       ├── Cargo.toml
│       ├── src/
│       └── tests/
```

---

## 📦 Crates Overview

### 1. `pdf-agent`
- **Why it exists:** Raw PDF extraction frequently causes LLM context overflows and OOM errors. `pdf-agent` introduces built-in **Agent Windowing** (`page_start`, `page_limit`, `max_chars`) and the **File Offload Pattern** (`output_file`), alongside pure-Rust visual page rasterization (`hayro`) and modern HTML/CSS vector PDF generation (`fulgur`).
- **Install:** `cargo add pdf-agent`
- **Documentation:** See [`crates/pdf-agent/README.md`](crates/pdf-agent/README.md) or [docs.rs/pdf-agent](https://docs.rs/pdf-agent).

### 2. `agent-canvas`
- **Why it exists:** AI agents frequently need to generate visual artifacts (decision trees, sales funnels, portfolio donuts, LaTeX formulas) without depending on heavy headless browsers (Chromium), Python (matplotlib), or Node.js. `agent-canvas` is built from first principles in 100% pure Rust.
- **Install:** `cargo add agent-canvas` (upcoming)
- **Documentation:** See [`crates/agent-canvas/README.md`](crates/agent-canvas/README.md).

---

## 🛠️ Building & Testing the Workspace

Compile the entire workspace:
```bash
cargo build --workspace
```

Run all unit, contract, and adversarial tests across all crates:
```bash
cargo test --workspace
```

---

## ⚖️ License

Dual-licensed under **MIT** or **Apache-2.0**.
Maintained by Darma Cahya (`darmady6@gmail.com`).
