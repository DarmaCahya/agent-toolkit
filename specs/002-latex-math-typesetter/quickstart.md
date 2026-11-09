# Quickstart: Validating the LaTeX Typesetter

Prerequisites: Rust stable; for font regeneration only, `uvx` (Python fonttools) and the upstream STIX Two Math font (`STIXTwoMath-Regular.otf` from CTAN `stix2-otf` 2021-04-02; SHA-256 pinned in `scripts/subset-font.sh`).

## 1. Build and lint
```bash
cargo fmt -p agent-canvas -- --check   # (examples/enterprise-suite is formatted separately)
cargo clippy -p agent-canvas --all-targets -- -D warnings
cargo build -p agent-canvas
```
Expect: clean; `Cargo.lock` gains no new runtime package (only dev-dep `insta` and its deps).

## 2. Run the tests
```bash
cargo test -p agent-canvas
```
Expect: scenario tests (quadratic formula, scripts, `\sum` limits, `\sqrt[3]{x}`), `test_math_nested_fractions`, `test_math_sqrt_stretch`, error-case table, no-text/no-font scan, determinism (100 renders equal), proptest (≥10,000 cases) all pass. Confirm tests were red before implementation.

## 3. Render the headline formula
```rust
// or: cargo run -p agent-canvas --example latex_quadratic
use agent_canvas::math::MathTypesetter;
let svg = MathTypesetter::new().render_to_svg(r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}")?;
std::fs::write("quadratic.svg", svg)?;
```
Open `quadratic.svg` in a browser: fraction with centred numerator/denominator, radical over `b² − 4ac`, tight unary minus. `grep -c "<text\|font-family\|<script" quadratic.svg` prints `0`.

## 4. Error behaviour
Render `\frac{a}{`, `\foo`, `\left( x \right)`, a 9000-char string, 65 nested braces: each returns `Err` with the documented variant and `position()`.

## 5. Font asset checks
```bash
ls -l crates/agent-canvas/assets/fonts/        # subset < 250 KB, OFL-1.1.txt present
grep -n "AgentCanvas Math\|STIX" THIRD_PARTY_NOTICES.md
scripts/subset-font.sh <path-to-STIXTwoMath-Regular.otf>   # prints the output SHA-256; run twice: equal, and equal to sha256sum of the committed asset
```
