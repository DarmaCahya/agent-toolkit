# Research: Excel Formula Evaluator

All decisions resolve items the spec deferred to planning or that the plan depends on.

## D1. Parser technique and precedence table

- **Decision**: Hand-written lexer + Pratt (binding-power) parser producing an `Expr` AST; evaluate by tree walk.
- **Rationale**: Pratt handles Excel's unusual precedence (unary minus above `^`, postfix `%`, left-assoc `^`) with a small binding-power table; trivial to bound recursion with a depth counter; no dependency.
- **Alternatives**: parser-combinator crates (new dependency, violates Principle VI); shunting-yard (awkward for function calls and lazy args); evaluating while parsing (breaks lazy `IF`, since untaken branches must still be syntactically skipped).

Binding powers (higher binds tighter):

| Level | Operators | Notes |
|---|---|---|
| 1 | `= <> < > <= >=` | left-assoc |
| 2 | `&` | left-assoc |
| 3 | `+ -` (binary) | left-assoc |
| 4 | `* /` | left-assoc |
| 5 | `^` | **left**-assoc (`2^3^2`=64) |
| 6 | postfix `%` | stackable (`200%%`) |
| 7 | prefix `+ -` | tighter than `^` (`-3^2`=9) |

Prefix operators bind tighter than `^` and `%`: `-3%` parses as `(-3)%` and `-3^2` as `(-3)^2`. The prefix operand is parsed at level 7, then postfix `%` and `^` apply to the result. Postfix `%` followed by an operand start (`5% 3`) is a syntax error because no infix operator sits between.

## D2. Depth bounding

- **Decision**: One counter shared by parser recursion. Each `(`, each function-call argument list, and each prefix-operator step increments depth; exceeding 64 yields `#VALUE!`. The AST therefore has depth ≤ ~64 + binary-operator spine. Binary operator chains (`1+1+1+...`) are built iteratively by the Pratt loop (left-assoc), but produce a left-leaning tree whose evaluation depth equals chain length (up to ~4k for 8192 chars). To keep the evaluator safe, the evaluator also carries a depth counter (limit: 64 for nesting constructs, and left-spine operator chains are folded with an explicit loop rather than recursion on the left child).
- **Rationale**: Satisfies FR-025 ("both parsing and evaluation bounded") without a stack overflow on `1+1+...+1`.
- **Implementation note**: The evaluator evaluates a left-leaning binary chain by iterating down the left spine into a `Vec`, then folding. Alternatively the parser can emit an n-ary `Chain` node for same-precedence left-assoc runs. Task phase picks one; the property test "long operator chain never overflows the stack" decides.
- **Alternatives**: run on a spawned big-stack thread (non-deterministic resource use, rejected); raise nothing and rely on the 8192 limit (≈4k-deep recursion is risky in debug builds, rejected).

## D3. Range / cell-reference hook (spec Assumption, deferred)

- **Decision**: A public trait `RangeProvider { fn resolve(&self, reference: &str) -> Option<Vec<Cell>>; }` (a range is a `Vec<Cell>` flattened row-major; `Cell::Empty` is the only place emptiness exists, see D4, so the public `Value` stays four-kind). `ExcelEvaluator::eval(formula)` uses no provider; `ExcelEvaluator::eval_with(formula, &dyn RangeProvider)` supplies one. Formula text `A1` / `A1:B3` is lexed as a reference token; without a provider (or when `resolve` returns `None`) it evaluates to `#NAME?`. A single-cell reference resolves to a one-element range used as a scalar when not an aggregate argument.
- **Rationale**: Makes FR-016/Story 5 testable without building a spreadsheet; object-safe, no deps; infallible signature keeps panic surface nil.
- **Alternatives**: a `HashMap<String, Value>` parameter (non-deterministic iteration risk if misused, less flexible); generic parameter on `eval` (needless monomorphization); skipping references entirely (Story 5 untestable through formula text).
- **Open-ended detail**: reference syntax accepted is `[A-Za-z]{1,3}[0-9]{1,7}(:[A-Za-z]{1,3}[0-9]{1,7})?`, case-insensitive, no `$` anchors or sheet names in Phase 1 (those yield `#NAME?`).

## D4. Value model and `Empty`

- **Decision**: Public `Value { Number(f64), Bool(bool), Text(String), Error(ExcelError) }` exactly as spec FR-002. Empty cells are represented only inside a range as an internal `Cell` enum (`Cell::Empty | Cell::Value(Value)`) so the public `Value` stays four-kind. A scalar-used empty cell coerces to `0` / `""` per Excel.
- **Rationale**: Keeps the public contract to four kinds while supporting the "empty cells skipped" acceptance scenario.
- **Alternatives**: add `Value::Empty` (violates FR-002's four kinds).

## D5. Number display formatting

- **Decision**: Whole numbers with |x| < 1e15 print as integers (no `.0`, `-0` prints `0`). Other finite numbers use Rust's shortest round-trip `{}` formatting converted to a fixed policy: if the decimal exponent is within [-7, 20], print plain decimal; otherwise print scientific as `d.dddE+XX` (Excel style, at least two exponent digits). Rust's `f64` Display is deterministic across platforms (Grisu/Ryu-style shortest repr in core, not libm).
- **Rationale**: Satisfies the spec's "shortest round-trip, no locale" assumption and SC-006.
- **Alternatives**: Excel's exact 15-sig-digit display (loses round-trip, e.g. `0.1+0.2` would show `0.3`; rejected for fidelity, but comparisons use 15 digits per D6); `{:e}` always (unreadable).

## D6. 15-significant-digit equality and ordering

- **Decision**: Two finite numbers are equal if they are bit-equal, or if `|a-b| <= 4 * f64::EPSILON * max(|a|,|b|)` (relative tolerance) with exact-zero handling (`0.1+0.2` differs from `0.3` by 1 ulp ≈ 5.5e-17, well inside). `<`, `>` etc. derive from "not equal, then partial_cmp". The string-rounding-to-15-digits form (`format!("{:.14e}")` parse) is the fallback only (see Alternatives).
- **Rationale**: The tolerance form avoids string round-trips (faster, no allocation) and is deterministic. 15 digits ≈ relative 1e-15; 4·ε ≈ 8.9e-16 is the closest ulp-based bound and is validated by the corpus test (`0.1+0.2=0.3` TRUE; `1=1.000000000000002` FALSE).
- **Alternatives**: string rounding to 15 digits (exact Excel analogue but allocates; kept as the fallback if corpus tests expose boundary mismatches).
- **Risk**: boundary behavior may differ from Excel by a few ulps; documented as not guaranteed bit-for-bit (spec Assumption).

Cross-type ordering: Number < Text < Bool. Text compare is case-insensitive using Unicode simple lowercase (`char::to_lowercase`, deterministic, locale-free), then by code-point order.

## D7. Coercion rules

- **Arithmetic / direct function args**: Bool→1/0; Text→number if it parses as trimmed decimal/scientific (`[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?`, no thousands separators, no `inf`/`nan`/hex — Rust's `f64::from_str` accepts `inf`/`nan`, so a pre-validation of the character grammar is mandatory) else `#VALUE!`; empty text `""` → `#VALUE!`.
- **Range elements** (aggregates): only `Number` counted; Text/Bool/Empty skipped; Error propagates (first error in row-major order).
- **`COUNT`**: direct args count if coercible to number (Bool, numeric text, numbers); range elements count only real numbers.
- **`AND`/`OR`**: direct args: Bool as is, numbers non-zero = true, numeric text coerced; non-numeric text `#VALUE!`. Range elements: text/empty ignored, if no logical values at all → `#VALUE!` (Excel).
- **`IF` condition**: Bool, Number (non-zero), no text coercion: **all** text, including numeric text like `"1"`, yields `#VALUE!`. Resolved to match the spec edge case literally (decision recorded in tasks.md T020).
- **`CONCAT`**: each arg to text via display form; ranges flattened, Empty → `""`, errors propagate.

## D8. Numeric safety

- Results go through `finite_or_num()`: NaN/±inf → `#NUM!`. `x/0` (including `0/0`) → `#DIV/0!` checked *before* division. `^`: `0^0` → `#NUM!` (Excel), `0^negative` → `#DIV/0!`, negative base with fractional exponent → `#NUM!`, via `powf` result check.
- `ROUND(x, d)`: `d` truncated toward zero (Excel), clamped to ±400 before use; if `d > 15`+ digits beyond precision return `x` unchanged; scaling by `10^d` uses `powi` on the clamped value, overflow → `#NUM!`. Halves round away from zero via `(x * s).abs().round()` copysigned (Rust `f64::round` is half-away-from-zero already, deterministic). Add a small pre-adjustment of 1 ulp-scaled epsilon only if corpus shows `ROUND(2.675,2)`-style divergence from Excel (Excel yields 2.68).
- `SUM` etc. accumulate left-to-right in a fixed order (determinism); no Kahan.

## D9. Unicode sanitization

- **Decision**: Reuse the toolkit rule used by `canvas/svg.rs` and `pdf-agent` (strip U+200B–U+200D, U+2060, U+FEFF, U+034F, U+E0000–U+E007F), implemented as a private function in `excel/lexer.rs` — the existing helpers are private to other modules/crates and cross-crate sharing is out of scope. Length limit (8192 *chars*) is checked on the raw input **before** sanitizing (cheap, prevents work on huge input); depth is checked after.
- **Rationale**: Matches FR-028 and the earlier security commit; avoids touching other modules.
- **Alternatives**: factor out a shared helper (touches unrelated code; do later).

## D10. Error mapping and type

- `#[non_exhaustive] pub enum ExcelError { DivZero, Value, Name, Num }` with `code() -> &'static str` and `Display`/`std::error::Error` via `thiserror`. Syntax errors, limit violations, wrong arity, wrong-kind arguments → `Value`; unknown function → `Name`; unresolved references → `Name`.
- Wrong arity is `#VALUE!` (Excel refuses entry; spec FR-022 only requires an Error value).
- Unknown function: `#NAME?` takes priority and its arguments are not evaluated, but the arguments must still *parse* (a syntax error elsewhere in the formula still yields `#VALUE!`).

## D11. Testing strategy

- Order per constitution IV: write failing tests first. Tiers: (1) unit tests in each file; (2) `tests/excel_tests.rs` user scenarios taken verbatim from the spec (quirk table, per-function, Story 5 with a stub `RangeProvider`); (3) 200+ formula corpus with expected display strings (SC-006), repeated twice per run; (4) `tests/excel_proptest.rs`: arbitrary `String` and structured-grammar generator never panic and always return a `Value`; length 8192/8193 and depth 64/65 boundary cases as explicit tests plus proptest strategies; case count set to 100_000 via `PROPTEST_CASES`/config in a `#[ignore]`-free but time-bounded test (default 10_000 in CI, full 100k documented in quickstart).
- Timing (SC-005) checked by a coarse debug-build assertion in a scenario test on a max-size formula, with a generous bound to avoid flakiness.

## D12. proptest dependency policy

- `[dev-dependencies] proptest = { version = "1", default-features = false, features = ["std"] }`. Dev-only, so it does not enter downstream resolution. If the project later wants the 100k run in CI, set `PROPTEST_CASES`.
