# Quickstart: validating glyph assembly and math kerning

Run from the repository root. Contracts: [contracts/stretchy-glyph.md](contracts/stretchy-glyph.md). Types: [data-model.md](data-model.md).

## Prerequisites

- Rust toolchain from the workspace.
- Only for rebuilding the font: `uvx` (fonttools) and the pinned `STIXTwoMath-Regular.otf` (SHA-256 `95bc2729e41faf93b0bcae9e96c4dc4da45855067fd0581e621e30734fe8d90b`). Not needed for build or test.

## 1. Gates

```bash
cargo fmt --check -p agent-canvas
cargo clippy -p agent-canvas --all-targets -- -D warnings
cargo test -p agent-canvas
```

Expected: all pass, no new `#[allow]` in library code.

## 2. Tall radical (User Story 1)

```bash
cargo test -p agent-canvas --test latex_layout -- sqrt_nested_fractions
```

Expected: `\sqrt` over 12 nested `\frac` renders; no `ScaledGlyph` (no non-uniform `scale(x y)` on the radical); radical height ≥ radicand height + gap; adjacent parts overlap ≥ `min_connector_overlap`; two renders are byte-identical.

## 3. Stretchy builder (User Story 2)

```bash
cargo test -p agent-canvas stretchy
cargo test -p agent-canvas --test latex_proptest
```

Expected: parenthesis, bracket, brace, bar, integral and radical at heights from 0 to 100× base: no panic, finite result, extender count ≤ 64, and result ≥ request whenever the request is within the cap-limited maximum (SC-003); zero, negative, NaN and ∞ inputs return a bounded result.

## 4. Kerning (User Story 3)

```bash
cargo test -p agent-canvas kern
```

Expected: a script on a kerned glyph differs from the kern-ignored placement by exactly the font's kern value; a nucleus without kern data is unchanged; sup and sub are kerned independently.

## 5. Snapshots (SC-005)

```bash
cargo test -p agent-canvas --test latex_layout --test latex_spacing --test latex_quadratic
```

Expected: no pending `.snap.new`. If any snapshot changes, inspect the SVG and record the justification in `specs/004-latex-glyph-assembly-mathkern/snapshot-notes.md` (and the PR notes) before accepting.

## 6. Font subset (User Story 4)

```bash
scripts/subset-font.sh /path/to/STIXTwoMath-Regular.otf   # run twice, compare the printed sha256
ls -l crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf
cargo test -p agent-canvas --test latex_font
```

Expected: identical hashes on both runs; size < 250 KB (currently 21,804 bytes); closure, assembly-part, kern-presence and family-name tests pass.
