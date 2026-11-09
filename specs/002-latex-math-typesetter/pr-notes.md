# PR notes: LaTeX math typesetter (Issue #9, LaTeX part)

Draft description material only; no PR has been opened.

## Constitution principles touched
- **I (pure Rust / minimal tooling)**: runtime is Rust only; Python `fonttools` (via `uvx`) is a developer-only tool used by `scripts/subset-font.sh`, never by build or tests.
- **II (safety)**: `#![forbid(unsafe_code)]` kept; the `latex` module additionally denies `unwrap`/`expect`/indexing/`panic`.
- **III (determinism)**: integer fixed-point number formatting, ordered collections, byte-identical output (tested 100x); font subset reproducible (same SHA-256 on repeated runs).
- **IV (test-first / scenario tests)**: unit tests per stage, scenario tests per user story, 10k-case proptest, insta snapshots.
- **V (clear errors)**: typed `MathError` with char positions and hints.
- **VI (minimal dependencies)**: see below.

## Dependencies
- `ttf-parser` (runtime, `std` + `opentype-layout`): replaces hand-written OpenType glyf/CFF/MATH-table parsing. Already present in `Cargo.lock` transitively, so no new runtime package.
- `insta` (dev-only, `default-features = false`): snapshot tests. Adds `similar`, `tempfile` (+ `rustix`, `errno`, `linux-raw-sys`) to the lockfile, all dev-only.
- Python `fonttools`: dev-only, regeneration script only.

## Licensing
Bundled `AgentCanvasMath-subset.otf` is a modified, renamed subset of STIX Two Math (OFL-1.1); licence text in `crates/agent-canvas/assets/fonts/OFL-1.1.txt`, attribution in `THIRD_PARTY_NOTICES.md`.

## Known limitations
- Layout correctness was checked by rasterising output with resvg and inspecting it; golden extents were recorded from the first approved render, not from an independent TeX render.
- No MathKern (corner kerning) and no glyph-assembly growth (radicals beyond the largest variant are scaled vertically).
- `\sqrt[n]{}` degree can touch the radical's tick for small radicals.
