# Contract: Public API and Input Grammar

## Rust API (exported from `agent_canvas::math`)

```rust
pub struct MathTypesetter { /* private */ }
impl MathTypesetter {
    pub fn new() -> Self;
    pub fn render_to_svg(&self, latex: &str) -> Result<String, MathError>;
}
impl Default for MathTypesetter { /* new() */ }

#[non_exhaustive]
pub enum MathError { /* variants: see research.md R8 */ }
impl MathError { pub fn position(&self) -> Option<usize>; }
```

Guarantees: never panics; deterministic; input ≤ 8192 chars; nesting ≤ 64; no I/O.

## Input grammar (v1)

```text
formula  := item*
item     := atom ( '^' arg | '_' arg )*      // at most one of each per atom
atom     := LETTER | DIGIT | '+' | '-' | '=' | '<' | '>' | '(' | ')' | ',' | symbol-cmd
          | group | '\frac' group group
          | '\sqrt' ('[' formula ']')? group
          | ('\sum' | '\int' | '\prod')
arg      := single-token-atom | group
group    := '{' formula '}'
symbol-cmd := \alpha … \omega | \Gamma … \Omega | \pm | \cdot | \times | \leq | \geq | \neq | \infty
```

Whitespace is ignored. Anything else → `MathError` with position.

## SVG output contract
- Root `<svg>` with `xmlns`, `viewBox`, `fill="currentColor"`.
- Allowed elements: `svg`, `defs`, `path`, `use`, `rect`. Forbidden anywhere: `text`, `tspan`, `style`, `script`, `foreignObject`, `image`, `font-family`, `on*=` attributes, any `href` not starting with `#`.
- `use` references only local ids. Numbers: ≤ 2 decimals, no exponent, no NaN/inf.

## Error cases (each carries `position()` where applicable)
| Input | Variant |
|---|---|
| `` / whitespace | `EmptyFormula` |
| len > 8192 | `InputTooLong` |
| `\frac{a}{` | `UnexpectedEof` |
| `\foo` | `UnknownCommand` |
| `\left`, `\text`, `\begin`, `\hat`, `\newcommand` | `Unsupported` (names it) |
| `x^2^3`, `x_1_2` | `DoubleScript` |
| `\frac{a}`, `\sqrt`, `x^` | `MissingArgument` / `UnexpectedEof` |
| 65 nested `{` | `NestingTooDeep` |
| `$x$`, non-ASCII, tag chars | `UnexpectedChar` |
