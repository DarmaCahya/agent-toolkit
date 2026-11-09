# Quickstart: validating spacing support

Prerequisites: Rust toolchain; run from the repo root.

1. Tests first (constitution IV): `cargo test -p agent-canvas --test latex_spacing` must fail before implementation.
2. After implementation:
   - `cargo test -p agent-canvas` (all pass; existing snapshots unchanged)
   - `cargo fmt --check && cargo clippy -p agent-canvas --all-targets -- -D warnings`
3. Manual check with `MathTypesetter::new().render_to_svg(r"\int_a^b f(x)\,dx")`:
   - returns `Ok(svg)`; width of `f(x)\,dx` exceeds `f(x)dx` by 3/18 em
   - a second render is byte-identical
   - `\,` alone returns `EmptyFormula`; `\@` returns `Unsupported` with the new hint
4. Review the new integral snapshot (glyph `<use>` elements only, no `<text>`) with `cargo insta review` before accepting.

Contract: [contracts/latex-spacing-grammar.md](contracts/latex-spacing-grammar.md). Types: [data-model.md](data-model.md).
