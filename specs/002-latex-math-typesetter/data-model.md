# Data Model: LaTeX Math Typesetter

All types are crate-private except `MathTypesetter` and `MathError`. Positions are char indices into the input.

## Lexer — `lexer.rs`
- `Token { kind: TokenKind, pos: usize }`
- `TokenKind`: `Letter(char)`, `Digit(char)`, `Symbol(char)` (`+ - = < > ( ) ,`), `Command(String)` (name after `\`), `LBrace`, `RBrace`, `LBracket`, `RBracket`, `Caret`, `Underscore`.
- Rules: whitespace skipped; any other char (incl. non-ASCII, control, tag/non-rendering code points, `$`, `%`, `#`, `&`, `~`) → `UnexpectedChar{pos}`. A lone trailing `\` → `UnexpectedEof`.

## AST — `parser.rs`
- `Node { kind: NodeKind, pos: usize }`
- `NodeKind`: `Ord(char)`-style leaf `Symbol(Sym)`, `Group(Vec<Node>)`, `Scripts{ base: Box<Node>, sup: Option<Box<Node>>, sub: Option<Box<Node>> }` (empty-group base when script has no base), `Frac{num, den}`, `Sqrt{index: Option<Box<Node>>, body}`, `BigOp{op: BigOpKind, sup, sub}`.
- `Sym` = resolved symbol: code point, TeX class, face (italic letter / upright / Greek), see atoms.
- Invariants: depth ≤ 64; a base takes at most one `^` and one `_` (second → `DoubleScript`); `\sqrt[` index parsed in brackets; `\frac`/`\sqrt` require braced args.

## Atoms — `atoms.rs`
- `AtomClass`: `Ord, Op, Bin, Rel, Open, Close, Punct` (+ internal `Inner` omitted in v1).
- `Style { size: StyleSize (D,T,S,SS), cramped: bool }` with `sup()`, `sub()`, `frac_num()`, `frac_den()`, `cramp()`, `is_script()`, `mu_scale()`.
- `Atom { class, content: AtomContent }` where content is a glyph (with code point), a nucleus box request, or a nested construct.
- Transformations: `Bin→Ord` demotion (start, after Bin/Rel/Open/Punct/Op, before Rel/Close/Punct, at end); spacing function `space(left, right, style) -> Mu` from the TeX table (0/3/4/5 mu; 4 and 5 become 0 in script styles).
- Letter mapping: Latin → Mathematical Italic block (`h` → U+210E); Greek lowercase → italic block, uppercase → upright; digits/operators upright.

## Font — `font.rs`
- `MathFont` (singleton built from `include_bytes!`, held in a `OnceLock` or constructed per call; no global mutable state beyond that): wraps `ttf_parser::Face`.
- `MathConsts`: units-per-em plus the MATH constants used (script scale-down %, script-script %, axis height, fraction rule thickness, numerator/denominator shifts & gaps (display/text), subscript/superscript shifts and gaps, radical constants, upper/lower limit gaps, display operator min height).
- Lookups: `glyph(char) -> Result<GlyphId, MathError>`, `advance(gid)`, `italic_correction(gid)`, `vertical_variants(gid) -> &[(GlyphId, advance)]`, `display_variant(gid)`, `outline(gid) -> PathData`, `bbox(gid)`.

## Box model — `box_model.rs`
- `LBox { width, height, depth: f64 (em units), items: Vec<Item> }`
- `Item`: `Glyph{ gid, x, y, scale }`, `Rule{ x, y, w, h }`, `Scaled radical glyph{ gid, x, y, sx, sy }`.
- Operations: `hbox(children with kerning)`, `vstack`, `shift`, `frac(...)`, `sqrt(...)`, `scripts(...)`, `limits(...)`.
- Invariants: all numbers finite; checked by a final `finite()` pass before rendering.

## Render — `render.rs`
- Input: root `LBox`. Output: `String`.
- Structure: `<svg xmlns=… viewBox="minx miny w h" fill="currentColor">` → `<defs>` (unique `<path id="g{gid}" d="…"/>` ordered by gid) → placed `<use href="#g{gid}" transform="translate(x y) scale(s)"/>` and rule `<rect>`s.
- Padding: fixed 20 units (0.02 em ×1000). 1 em = 1000 units, 2 dp.

## Errors — `error.rs`
`MathError` as in research R8; derives `Debug, Clone, PartialEq, Eq, thiserror::Error`; `#[non_exhaustive]`.

## Public façade — `mod.rs`
`MathTypesetter` (unit/zero-field struct with `new()`, `Default`), `render_to_svg(&self, latex: &str) -> Result<String, MathError>`; constants `MAX_INPUT_CHARS = 8192`, `MAX_DEPTH = 64`.

## State flow
`&str → (len check) → Vec<Token> → Node tree (depth check) → Atom list per level (class assignment, demotion) → LBox (layout, style flows down) → SVG String`. No state persists between calls.
