---

description: "Task list for the Deterministic Excel Formula Evaluator"
---

# Tasks: Deterministic Excel Formula Evaluator

**Input**: Design documents from `/specs/001-excel-formula-evaluator/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/rust-api.md, quickstart.md; constitution at `.specify/memory/constitution.md`

**Tests**: REQUIRED. Constitution Principle IV (test-first, user-scenario tests) is NON-NEGOTIABLE. Within every story, write the test tasks first and watch them fail (red) before the implementation tasks.

**Organization**: Grouped by user story. US1-US3 are all P1 and share one pipeline, so a minimal pipeline lives in Foundational and each story deepens it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no dependency on an incomplete task)
- **[Story]**: US1..US6 from spec.md
- All paths are relative to repo root. Crate root: `crates/agent-canvas/`. Module root: `crates/agent-canvas/src/math/excel/` (written `excel/` below).

## Global rules for every task (from the constitution and plan)

- Every new file under `excel/` starts with `#![forbid(unsafe_code)]`. The lint gate `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]` is declared once in `excel/mod.rs` and covers all submodules. Test files may use `#![allow(...)]` for these lints with a reason comment.
- No `unwrap`/`expect`/`[i]` indexing/`as` casts that can wrap on user data. Use `get`, iterators, pattern matching, saturating or checked arithmetic.
- No `HashMap`, time, randomness, env, or locale in any output path.
- No new runtime dependencies. `proptest` is dev-only.
- Errors are `Value::Error(ExcelError::..)` values. Never return `Result::Err` from `eval`/`eval_with` and never panic.
- Mapping: syntax error, limit violation, wrong arity, bad coercion, text `IF` condition -> `Value`; unknown function, unresolved reference -> `Name`; NaN/inf/overflow -> `Num`; division by zero -> `DivZero`.

---

## Phase 1: Setup

**Purpose**: Module skeleton and dev dependency.

- [X] T001 Add `proptest = { version = "1", default-features = false, features = ["std"] }` under a new `[dev-dependencies]` section in `crates/agent-canvas/Cargo.toml` (research D12). No other Cargo.toml change.
- [X] T002 Create the empty module skeleton: `excel/mod.rs`, `excel/value.rs`, `excel/lexer.rs`, `excel/parser.rs`, `excel/eval.rs`, `excel/functions.rs`, `excel/compare.rs` under `crates/agent-canvas/src/math/`. Each file has `#![forbid(unsafe_code)]` and a one-line `//!` doc. `excel/mod.rs` declares the six submodules, the `deny(clippy::...)` lint attrs, and consts `pub(crate) const MAX_INPUT_CHARS: usize = 8192;` and `pub(crate) const MAX_DEPTH: usize = 64;`.
- [X] T003 Wire the module in `crates/agent-canvas/src/math/mod.rs`: add `pub mod excel;` only. Run `cargo check -p agent-canvas`; it must compile (the skeleton files are empty). The `pub use excel::{Cell, ExcelError, ExcelEvaluator, RangeProvider, Value};` re-export (contract rust-api.md) is added in T016, once those types exist.
- [X] T004 [P] Create empty test files `crates/agent-canvas/tests/excel_tests.rs` and `crates/agent-canvas/tests/excel_proptest.rs` with `use agent_canvas::math::{ExcelEvaluator, Value, ExcelError, Cell, RangeProvider};` and a header comment saying relaxed lints are allowed in tests.

---

## Phase 2: Foundational (blocks all stories)

**Purpose**: Public types, display, coercion, lexer, parser, and an evaluator skeleton that every story depends on.

**CRITICAL**: No user story starts until this phase is complete. Tests first.

### Tests (write first, confirm red)

- [X] T005 [P] Unit tests in `excel/value.rs` (`#[cfg(test)] mod tests`) for display (`Value::as_str`): whole number -> no `.0` (`3.0`->`3`); `-0.0`->`0`; `0.5`->`0.5`; non-integers and whole values with |x| >= 1e15 print plain decimal while the decimal exponent is within [-7, 20] (`1e15` -> `1000000000000000`, `1e20` plain) and scientific `d.dddE+XX` (at least 2 exponent digits, e.g. `1E+21`, `1E-08`) outside it (research D5); Bool -> `TRUE`/`FALSE`; Text -> itself; Error -> `#DIV/0!`/`#VALUE!`/`#NAME?`/`#NUM!`; `ExcelError::code()` matches. Also coercion tests (research D7): `"3"`, `" 3 "`, `"1E3"`, `".5"` coerce; `""`, `"abc"`, `"inf"`, `"nan"`, `"0x10"`, `"1,000"` do NOT (-> `Value` error); Bool->1/0.
- [X] T006 [P] Unit tests in `excel/lexer.rs` for sanitize + tokenize: strips U+200B-U+200D, U+2060, U+FEFF, U+034F, U+E0000-U+E007F; numbers (`12`, `1.5`, `.5`, `1E3`, `1e-2`); strings with `""` escape (`"a""b"` -> `a"b`); whitespace/newlines skipped between tokens but preserved inside strings; `TRUE`/`false` bool literals case-insensitive; idents; references (`A1`, `a1:B3` uppercased); all operators including two-char `<>`, `<=`, `>=`; `%`; an unterminated string and an illegal character produce a lexer error (internal `Result`, mapped to `Value` later).
- [X] T007 [P] Unit tests in `excel/parser.rs` for AST shape of simple inputs: `1+2*3` (Mul under Add), `(1+2)*3`, `f(1,2)`, `f()`, `-5`, `50%`, `A1:B3`; failures for `1+`, `(1`, `SUM(`, `)`, `1 2`, `5% 3`, empty input.

### Implementation

- [X] T008 Implement `ExcelError` (`#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]`, `#[non_exhaustive]`, variants `DivZero, Value, Name, Num`, each with a `#[error("#DIV/0!")]`-style message, plus `pub fn code(&self) -> &'static str`) and `Value` (`#[derive(Debug, Clone, PartialEq)]`, variants `Number(f64), Bool(bool), Text(String), Error(ExcelError)`) in `excel/value.rs`. Invariant: a `Number` is never NaN/inf; add `pub(crate) fn finite_or_num(f64) -> Value` (NaN/+-inf -> `Error(Num)`).
- [X] T009 Implement `Value::as_str(&self) -> Cow<'_, str>` per research D5 in `excel/value.rs`: `Text` borrows; whole `|x| < 1e15` prints as integer (`-0` -> `0`); other finite numbers use Rust shortest round-trip `{}` formatting, plain decimal when decimal exponent is in [-7, 20], else Excel-style scientific `d.dddE+XX`. Do not use `format!("{:e}")` output directly without normalizing exponent to a sign plus at least two digits. Makes T005 display tests green.
- [X] T010 Implement coercion helpers in `excel/value.rs`: `pub(crate) fn parse_number_text(&str) -> Option<f64>` with a mandatory pre-validation of the character grammar `[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?` after trimming whitespace (Rust's `f64::from_str` accepts `inf`/`nan`, so it must never see unvalidated text), result must be finite; `pub(crate) fn to_number(&Value) -> Result<f64, ExcelError>` (Number as is, Bool 1/0, Text via `parse_number_text` else `Value` error, Error propagates its own code); `pub(crate) fn to_text(&Value) -> Result<Cow<str>, ExcelError>` (error propagates). Makes the coercion tests in T005 green.
- [X] T011 Define `Cell` (`#[derive(Debug, Clone, PartialEq)] pub enum Cell { Empty, Value(Value) }`) and `pub trait RangeProvider { fn resolve(&self, reference: &str) -> Option<Vec<Cell>>; }` in `excel/value.rs` (data-model.md; resolves the research D3 vs contract mismatch in favor of `Vec<Cell>`).
- [X] T012 Implement sanitize + lexer in `excel/lexer.rs`: `pub(crate) fn sanitize(&str) -> String` (strip code points listed in research D9) and `pub(crate) fn tokenize(&str) -> Result<Vec<Token>, ()>` producing the `Token` set from data-model.md (`Number, Str, Bool, Ident, Ref, Op, LParen, RParen, Comma, Percent, Eof`). Reference syntax `[A-Za-z]{1,3}[0-9]{1,7}(:[A-Za-z]{1,3}[0-9]{1,7})?` uppercased; an identifier directly followed by `(` is an `Ident` (function), `TRUE`/`FALSE` not followed by `(` are `Bool`; `$` anchors and sheet names are rejected. Iterate with `chars()` only (no byte slicing). Cap token count implicitly by input length. Makes T006 green.
- [X] T013 Define the `Expr` enum in `excel/parser.rs` per data-model.md (`Number, Text, Bool, Ref, Unary{op,operand}, Percent, Binary{op,lhs,rhs}, Call{name,args}` with `Box` children, `UnOp {Neg,Pos}`, `BinOp {Add,Sub,Mul,Div,Pow,Concat,Eq,Ne,Lt,Gt,Le,Ge}`). Call names are stored uppercased.
- [X] T014 Implement the Pratt parser in `excel/parser.rs`: `pub(crate) fn parse(tokens: Vec<Token>) -> Result<Expr, ()>` with the binding-power table from research D1 (comparison 1, `&` 2, `+ -` 3, `* /` 4, `^` 5 LEFT-assoc, postfix `%` 6 stackable, prefix `+ -` 7, so the prefix operand is parsed at level 7 and `-3^2` = `(-3)^2`). A single depth counter increments on each `(`, each call argument list, and each prefix-operator step; depth > `MAX_DEPTH` returns Err (depth exactly 64 is accepted). Postfix `%` followed by an operand start is an Err. All tokens must be consumed (trailing garbage -> Err). Makes T007 green.
- [X] T015 Implement the evaluator skeleton in `excel/eval.rs`: `pub(crate) struct Ctx<'a> { provider: Option<&'a dyn RangeProvider>, depth: usize }` and `pub(crate) fn eval_expr(&Expr, &mut Ctx) -> Value` handling literals, Unary, Percent, and Binary arithmetic (`+ - * / ^`) with coercion via `to_number`, `finite_or_num`, division-by-zero check BEFORE dividing (including `0/0` -> `DivZero`), and `^` rules from research D8 (`0^0` -> `Num`, `0^negative` -> `DivZero`, negative base with fractional exponent -> `Num`, non-finite -> `Num`). Left-associative binary chains MUST be evaluated by iterating down the left spine into a `Vec` and folding (research D2), not by recursing on the left child. Calls and Refs return `Error(Name)` for now. Depth counter guards recursion on right/nesting children at `MAX_DEPTH`.
- [X] T016 Implement `ExcelEvaluator` in `excel/mod.rs`: `pub struct ExcelEvaluator;` with `pub fn eval(formula: &str) -> Value` and `pub fn eval_with(formula: &str, provider: &dyn RangeProvider) -> Value`. Pipeline: `formula.chars().count() > MAX_INPUT_CHARS` -> `Error(Value)` BEFORE any other work (use `chars().take(MAX+1).count()` to stay O(limit)); sanitize; trim; strip one optional leading `=`; trim again; empty remainder -> `Error(Value)`; tokenize; parse; evaluate. Any stage failure maps per the global mapping rule. Re-export `Value, ExcelError, Cell, RangeProvider` from `excel/mod.rs`, and add `pub use excel::{Cell, ExcelError, ExcelEvaluator, RangeProvider, Value};` to `crates/agent-canvas/src/math/mod.rs`.

**Checkpoint**: `cargo test -p agent-canvas excel` green for T005-T007; `eval("=1+2*3")` works end to end.

---

## Phase 3: User Story 1 - Evaluate a formula string to a result (P1) MVP

**Goal**: Text in, `Value` out, with a headline `IF/SUM` example, optional `=`, display strings, determinism.

**Independent Test**: `=IF(SUM(10, 20) > 25, "Profit", "Loss")` -> text `Profit`; `=SUM(1,2,3)*2` -> `12`.

### Tests (write first, confirm red)

- [X] T017 [P] [US1] User-scenario tests in `crates/agent-canvas/tests/excel_tests.rs` (module `story1`): headline formula -> `Value::Text("Profit")` and `as_str()` == `Profit`; `=1+2*3` -> `Number(7.0)`; `1+2*3` and `=1+2*3` equal; `=SUM(1,2,3)*2` -> `12`; display cases (`=4/2` -> `2`, `=1/4` -> `0.25`, `=TRUE` -> `TRUE`, `="x"` -> `x`, `=1/0` -> `#DIV/0!`); same formula evaluated twice yields identical `Value`.
- [X] T018 [P] [US1] Unit tests in `excel/compare.rs` for `compare(&Value, &Value) -> Result<Ordering, ExcelError>` basics needed by `>`: number vs number, error propagates (full quirk coverage is in US2).

### Implementation

- [X] T019 [US1] Implement `compare` (numbers only plus error propagation for now) in `excel/compare.rs` and comparison operators `= <> < > <= >=` in `excel/eval.rs` returning `Bool`. Errors in either operand propagate before comparing (spec edge case).
- [X] T020 [US1] Implement `IF` (lazy: evaluate only the selected branch; 2 or 3 args; missing else branch with false condition -> `Bool(false)`) and `SUM` (left-to-right accumulation, no Kahan, `finite_or_num` on result) in `excel/functions.rs`; dispatch from `Expr::Call` in `excel/eval.rs` via a `match` on the uppercased name (no HashMap). `IF` condition: Bool, Number (non-zero = true); Text -> `Error(Value)` (spec edge case, literal reading; numeric-text conditions also `#VALUE!`, resolving the ambiguity in research D7). Unknown names -> `Error(Name)` without evaluating args. Makes T017/T018 green.

**Checkpoint**: US1 independently demonstrable.

---

## Phase 4: User Story 2 - Match Excel's operator and precedence behavior (P1)

**Goal**: Every quirk in the spec's quirk table matches Excel.

**Independent Test**: Run the quirk table; every row passes.

### Tests (write first, confirm red)

- [X] T021 [P] [US2] Quirk-table scenario tests in `crates/agent-canvas/tests/excel_tests.rs` (module `story2`): `=-3^2` -> `9`; `=2^3^2` -> `64`; `=50%` -> `0.5`; `=200*10%` -> `20`; `=200%%` -> `0.02`; `=--5` -> `5`; `=+5` -> `5`; `="a"&"b"&1` -> `ab1`; `=1&2+3` -> `15` (`&` looser than `+`); `=1&2="12"` -> `TRUE` (`&` tighter than `=`; text vs text) and `=1&2=12` -> `FALSE` (text `"12"` vs number 12 are different types); `="ABC"="abc"` -> `TRUE`; `<>`, `<`, `>`, `<=`, `>=` on text case-insensitively (`="a"<"B"` -> `TRUE`); `=0.1+0.2=0.3` -> `TRUE`; `=1=1.000000000000002` -> `FALSE`; mixed types: `=1<"a"` -> `TRUE`, `="a"<TRUE` -> `TRUE`, `=1<TRUE` -> `TRUE`, `="a"=1` -> `FALSE` (no error); `=(1+2)*3` -> `9`; `=5% 3` -> `#VALUE!`.
- [X] T022 [P] [US2] Unit tests in `excel/compare.rs`: 15-digit equality (`0.1+0.2` vs `0.3` equal; `1.0` vs `1.000000000000002` not equal; exact zero handling `0.0 == -0.0`; large magnitudes), Number < Text < Bool ordering, text compare uses `char::to_lowercase` then code-point order, same-type Bool `FALSE < TRUE`.

### Implementation

- [X] T023 [US2] Complete `excel/compare.rs` per research D6: number equality by `|a-b| <= 4.0 * f64::EPSILON * max(|a|,|b|)` with exact-zero handling (keep the string-rounding fallback only if T022/T021 boundary cases fail), cross-type rank Number < Text < Bool, case-insensitive text compare. If the tolerance form fails a boundary test, switch to rounding both values to 15 significant digits and note it in the code comment.
- [X] T024 [US2] Implement `&` concatenation in `excel/eval.rs` (both operands through `to_text` / display rules: numbers per `as_str`, booleans `TRUE`/`FALSE`, errors propagate) and verify the precedence table end to end against T021; fix parser binding powers in `excel/parser.rs` only if a quirk row fails.

**Checkpoint**: US1 + US2 pass independently.

---

## Phase 5: User Story 3 - Errors are values, not failures (P1)

**Goal**: Every failure mode is an `Error` value with the correct code; errors propagate.

**Independent Test**: `=1/0`, `="x"+1`, `=FOO(1)`, `=10^1000`, `=1+` all return error values, no panic.

### Tests (write first, confirm red)

- [X] T025 [P] [US3] Scenario tests in `crates/agent-canvas/tests/excel_tests.rs` (module `story3`): `=1/0` -> `#DIV/0!`; `=0/0` -> `#DIV/0!`; `="abc"+1` -> `#VALUE!`; `=NOSUCH(1)` -> `#NAME?`; `=10^1000` -> `#NUM!`; `=0^0` -> `#NUM!`; `=(-8)^0.5` -> `#NUM!`; `=0^-1` -> `#DIV/0!`; `=1+(1/0)` -> `#DIV/0!`; `=(1/0)=1` -> `#DIV/0!`; `=NOSUCH(1/0)` -> `#NAME?` (args not evaluated); `=NOSUCH(1+)` -> `#VALUE!` (args must still parse); syntax errors `=1+`, `=(1`, `=SUM(`, ``, `=`, `   ` -> `#VALUE!`; volatile `=NOW()`, `=RAND()`, `=TODAY()` -> `#NAME?`; `A1` with no provider -> `#NAME?`. Each asserts the result is a `Value::Error` (no panic, no `Result`).
- [X] T026 [P] [US3] Batch test (SC-007): a `Vec<&str>` of 20 mixed valid/invalid formulas mapped through `ExcelEvaluator::eval` in one pass; assert length 20 and the exact expected display for each; a failing one never interrupts the rest.

### Implementation

- [X] T027 [US3] Audit and complete error handling across `excel/eval.rs`, `excel/lexer.rs`, `excel/parser.rs`, `excel/mod.rs` so every row in T025 passes: ensure unknown function dispatch skips arg evaluation but the arg expressions were already parsed; `Ref` without provider or with `resolve` -> `None` -> `Name`; no stage returns `Result::Err` out of the public API. Update the error-mapping doc comment on `ExcelEvaluator` to list the mapping.

**Checkpoint**: P1 stories (US1-US3) complete. MVP is shippable here.

---

## Phase 6: User Story 4 - Use the Phase-1 function set (P2)

**Goal**: All 13 functions behave per spec FR-019 to FR-023.

**Independent Test**: One formula per function matching Excel-documented results.

### Tests (write first, confirm red)

- [X] T028 [P] [US4] Per-function scenario tests in `crates/agent-canvas/tests/excel_tests.rs` (module `story4`): `ROUND(2.5,0)`=3, `ROUND(-2.5,0)`=-3, `ROUND(1234,-2)`=1200, `ROUND(2.675,2)`=2.68 (Excel parity; see research D8), `ROUND(1.5)` -> `#VALUE!` (arity), `ROUND(1,1000)` and `ROUND(1,-1000)` terminate without panic; `IFERROR(1/0,"n/a")`=`n/a`; `IFERROR(5,1/0)`=5; `IF(TRUE,1,1/0)`=1; `IF(FALSE,1/0,2)`=2; `IF(1)` -> `#VALUE!`; `IF("x",1,2)` -> `#VALUE!`; `AND(TRUE,1,0)`=FALSE; `OR(FALSE,0)`=FALSE; `NOT(0)`=TRUE; `AND()` / `OR()` / `NOT()` -> `#VALUE!`; `CONCAT("a",1,TRUE)`=`a1TRUE`; `SUM()`=0; `COUNT()`=0; `AVERAGE()` -> `#DIV/0!`; `MIN()`=0; `MAX()`=0; `AVERAGE(1,2,3)`=2; `MIN(3,1,2)`=1; `MAX(3,1,2)`=3; `ABS(-4.5)`=4.5; `ABS()` and `ABS(1,2)` -> `#VALUE!`; function names case-insensitive (`sum(1,2)`, `Sum(1,2)`); `SUM(1/0)` -> `#DIV/0!`.
- [X] T029 [P] [US4] Unit tests in `excel/functions.rs` for the per-function arity table (fixed-arity: `NOT`=1, `ABS`=1, `ROUND`=2, `IFERROR`=2, `IF`=2..3; variadic: `SUM AVERAGE MIN MAX COUNT AND OR CONCAT`; zero arguments are allowed for `SUM AVERAGE MIN MAX COUNT` per spec Story 4.6, while `AND()`/`OR()` with no logical values yield `Error(Value)` and `CONCAT()` yields empty text) and `ROUND` helper edge cases (digits clamp to +-400, truncation toward zero of fractional digit counts, overflow -> `Num`, huge digits return `x` unchanged).

### Implementation

- [X] T030 [US4] In `excel/functions.rs` implement `AVERAGE`, `MIN`, `MAX`, `COUNT` over direct arguments (direct-arg coercion: Bool->1/0, numeric text, non-numeric text -> `Value` error; empty `AVERAGE` -> `DivZero`, empty `MIN`/`MAX` -> `Number(0)`, empty `COUNT` -> `Number(0)`; `COUNT` counts direct args coercible to number). Accumulate left-to-right.
- [X] T031 [US4] In `excel/functions.rs` implement `AND`, `OR`, `NOT`, `ABS`, `CONCAT` (direct args: Bool as is, numbers non-zero = true, numeric text coerced, non-numeric text -> `Value` error; `CONCAT` uses display text, errors propagate).
- [X] T032 [US4] In `excel/functions.rs` implement `IFERROR` (lazy: evaluate arg 2 only if arg 1 is an `Error`) and `ROUND` per research D8 (digits truncated toward zero then clamped to +-400 using saturating conversion, `f64::round` for half-away-from-zero, `powi` scaling, overflow -> `Num`; add the minimal pre-adjustment only if the `ROUND(2.675,2)` test demands it). Enforce arity for all 13 functions with a single table-driven check that yields `Error(Value)` for wrong arity. Makes T028/T029 green.

**Checkpoint**: US1-US4 pass independently.

---

## Phase 7: User Story 5 - Direct arguments vs. ranges coerce differently (P2)

**Goal**: Range values coerce by range rules; direct arguments by direct rules; references resolve via `RangeProvider`.

**Independent Test**: `=SUM("3",TRUE)` -> `4` while `=SUM(A1:A3)` over `"3"`, `TRUE`, `2` -> `2`.

### Tests (write first, confirm red)

- [X] T033 [P] [US5] Scenario tests in `crates/agent-canvas/tests/excel_tests.rs` (module `story5`) using a stub `RangeProvider` (a struct with a `Vec<(&'static str, Vec<Cell>)>` looked up linearly, deterministic): `=SUM("3",TRUE,2)`=6; `=SUM("abc")` -> `#VALUE!`; range `[Text("3"), Bool(true), Number(2)]` with `=SUM(A1:A3)`=2; `=COUNT("3",TRUE,5)`=3 vs `=COUNT(A1:A3)`=1; range with `Cell::Empty` skipped by `SUM`, `COUNT`, `AVERAGE` (average over numbers only); range containing `Error(Num)` propagates from `SUM`/`AVERAGE`/`MIN`/`MAX`/`COUNT`; `=AVERAGE(A1:A3)` over only text -> `#DIV/0!`; `AND`/`OR` over range ignore text/empty and yield `#VALUE!` when no logical values; `CONCAT(A1:A3)` flattens, `Empty` -> `""`, errors propagate; provider returning `None` -> `#NAME?`; `$A$1` and `Sheet1!A1` -> `#NAME?` even with a provider; a single-cell reference used as a scalar (`=A1+1`) with a one-element range works and a multi-cell range used as a scalar -> `#VALUE!`; lowercase reference `a1` passed to provider as `A1`.

### Implementation

- [X] T034 [US5] In `excel/eval.rs` evaluate `Expr::Ref` through `ctx.provider`: resolve the uppercased reference; `None` or no provider -> `Error(Name)`. Introduce an internal `Arg` representation (`Scalar(Value)` | `Range(Vec<Cell>)`) so function dispatch in `excel/functions.rs` can tell direct arguments from range arguments. Scalar contexts accept a one-element range (Empty -> `0`/`""` per Excel) and reject longer ranges with `Error(Value)`. Guard range size: cap total cells processed per formula at a constant (e.g. 1_000_000) returning `Error(Value)` beyond it, to keep SC-005/SC-004 hang-free with hostile providers.
- [X] T035 [US5] Apply range rules in `excel/functions.rs` for `SUM AVERAGE MIN MAX COUNT AND OR CONCAT`: range elements count only for real `Number` (aggregates) / Bool-and-number logicals (`AND`/`OR`); Text, Bool, Empty skipped in numeric aggregates; the first `Error` in row-major order propagates; direct args keep T030/T031 rules. Makes T033 green.

**Checkpoint**: US1-US5 pass independently.

---

## Phase 8: User Story 6 - Safe on hostile or oversized input (P2)

**Goal**: Hard bounds on size and depth, zero panics on any input.

**Independent Test**: boundary lengths 8192/8193, depth 64/65, random input fuzz.

### Tests (write first, confirm red)

- [X] T036 [P] [US6] Boundary scenario tests in `crates/agent-canvas/tests/excel_tests.rs` (module `story6`): an 8192-char formula (e.g. `=` + `1` padded with `+0` to exactly 8192 chars) evaluates to a non-error number; 8193 chars -> `#VALUE!`; the 8192 limit counts chars not bytes (8192 copies of a multi-byte char inside a string literal accepted, 8193 rejected); nesting `((((...1...))))` at depth 64 -> `1`, depth 65 -> `#VALUE!`; the same at 64/65 for nested calls `ABS(ABS(...))` and prefix chains `-----1`; a ~4000-term chain `1+1+...+1` evaluates to 4000 without stack overflow (D2); right-nested exponent/paren mix does not overflow; huge arithmetic `=10^308*10` -> `#NUM!`; `=ROUND(1,1000000000)` terminates; input full of tag chars (U+E0000 range) and zero-width chars behaves identically to the cleaned formula.
- [X] T037 [P] [US6] Timing test in `crates/agent-canvas/tests/excel_tests.rs`: a maximum-size valid formula evaluates in under a generous bound (e.g. 500 ms, to avoid flakiness in debug builds; SC-005 target of 10 ms is verified in release per quickstart).
- [X] T038 [P] [US6] Property tests in `crates/agent-canvas/tests/excel_proptest.rs` with `proptest` (relaxed lints allowed): (a) arbitrary `String` input (any chars incl. control, Unicode, tags) never panics and returns a `Value`; (b) a structured generator over the grammar tokens (numbers, strings, ops, parens, commas, the 13 function names, references) never panics, run through both `eval` and `eval_with` using a stub provider; (c) generated lengths around 8192/8193 and nesting around 64/65 hold the documented boundary behavior; (d) idempotence: evaluating the same input twice yields equal `Value`s. Configure `ProptestConfig::with_cases(10_000)` by default, overridable via `PROPTEST_CASES` (quickstart runs 100_000).

### Implementation

- [X] T039 [US6] Make T036-T038 green by tightening limits in `excel/mod.rs`, `excel/lexer.rs`, `excel/parser.rs`, `excel/eval.rs`: confirm the char-count check is before sanitize and is O(limit); depth accounting in parser matches Phase-2 rules exactly at 64/65 for parens, call arg lists, and prefix chains; evaluator depth counter mirrors it; operator chains use the loop/fold from T015; fix any panic or stack overflow the property tests uncover (add a regression test for each before fixing, per Principle IV).

**Checkpoint**: All six stories pass.

---

## Phase 9: Polish and Cross-Cutting

- [X] T040 [P] Add the 200+ formula determinism corpus test (SC-006) in `crates/agent-canvas/tests/excel_tests.rs` (module `corpus`): a `const CORPUS: &[(&str, &str)]` of at least 200 `(formula, expected display)` pairs covering every operator, every function, every error code, coercion and range cases (stub provider), and boundary inputs; the test evaluates the full corpus twice and asserts both runs equal each other and equal the expected strings. Every expected string must be derived from the spec/Excel documentation, not from running the implementation.
- [X] T041 [P] Add rustdoc to `excel/mod.rs` (module-level overview with the headline example as a doctest, precedence table, error mapping, limits, determinism and "not bit-for-bit Excel beyond 15 digits" note) and doc comments on every public item. Add a short Excel evaluator section to `crates/agent-canvas/README.md` showing `ExcelEvaluator::eval` and `eval_with`.
- [X] T042 [P] Add a runnable example `crates/agent-canvas/examples/issue-009-demo/main.rs` plus a `[[example]] name = "issue_009_demo"` / `path = "examples/issue-009-demo/main.rs"` entry in `crates/agent-canvas/Cargo.toml` (matching the existing `issue_00X_demo` pattern) that prints several formulas and their `as_str()` results.
- [ ] T043 Run the full quality gate from the repo root and fix findings: `cargo fmt --check`; `cargo clippy -p agent-canvas --all-targets -- -D warnings` (the `deny` attrs from the constitution must produce zero violations in `excel/`); `cargo test -p agent-canvas`; confirm existing tests in other modules are untouched and still pass.
- [X] T044 Run the SC-004 full fuzz: `PROPTEST_CASES=100000 cargo test -p agent-canvas --test excel_proptest --release`; record pass/fail output. Also confirm the max-size formula timing under 10 ms in release (SC-005) with a one-off timing print or the T037 test in `--release`.
- [ ] T045 Walk through `specs/001-excel-formula-evaluator/quickstart.md` scenarios 1-8 against the implementation and correct any quickstart or spec mismatch found (research D3 vs the contract on `Vec<Cell>` is already reconciled). Prepare the PR description stating which constitution principles are touched (I, II, III, IV, V, VI) per the Development Workflow, including: the Principle II scoped exception (lints applied at `excel/` module level, not crate level, with the reason from plan.md), and the Principle VI justification for the dev-only `proptest` dependency (SC-004 needs 100k generated inputs with shrinking; test-only, never reaches downstream lockfiles).

---

## Dependencies and Execution Order

### Phase dependencies

- **Setup (1)**: none.
- **Foundational (2)**: depends on Setup; BLOCKS all stories.
- **US1 (3)** -> depends on Phase 2. **US2 (4)** depends on US1 (uses `compare` and comparison ops from T019). **US3 (5)** depends on Phase 2 (and uses US1 function dispatch). **US4 (6)** depends on US1 (function dispatch, `IF`, `SUM`). **US5 (7)** depends on US4 (functions to apply range rules to). **US6 (8)** depends on Phase 2 and is best verified last, after US4/US5, because the property tests exercise all functions.
- **Polish (9)**: depends on all stories.

### Within a story

- Tests first and failing, then implementation. Within a phase, tasks touching the same file (`eval.rs`, `functions.rs`, `compare.rs`, `excel_tests.rs`) are sequential even if listed adjacent; the `[P]` marks above only apply across different files (or across separate test modules written by different people in `excel_tests.rs` if merged carefully).

### Parallel opportunities

- Phase 1: T004 alongside T001-T003.
- Phase 2 tests: T005 (value.rs), T006 (lexer.rs), T007 (parser.rs) in parallel; then T008-T011 (value.rs, sequential), T012 (lexer.rs) and T013-T014 (parser.rs) can proceed in parallel with each other once T008 exists.
- US1: T017 (tests/excel_tests.rs) and T018 (compare.rs) in parallel.
- US2: T021 and T022 in parallel.
- US3: T025 and T026 in parallel.
- US4: T028 and T029 in parallel.
- US6: T036, T037, T038 (T038 in a different file) in parallel.
- Polish: T040, T041, T042 in parallel.

### Parallel example: Foundational tests

```text
Task: "T005 unit tests in crates/agent-canvas/src/math/excel/value.rs"
Task: "T006 unit tests in crates/agent-canvas/src/math/excel/lexer.rs"
Task: "T007 unit tests in crates/agent-canvas/src/math/excel/parser.rs"
```

---

## Implementation Strategy

### MVP first

1. Phase 1 Setup, Phase 2 Foundational.
2. Phase 3 (US1): verify the headline example. This alone is a demoable evaluator.
3. Add US2 and US3 (all P1) to reach a trustworthy core; stop and validate against the quirk table and error cases.

### Incremental delivery

4. US4 (functions), then US5 (ranges), then US6 (hardening and fuzz), validating each independently.
5. Polish: corpus, docs, example, full quality gate, 100k fuzz.

### Notes

- Commit after each task or logical group; keep red/green visible in history where practical.
- Out of scope: LaTeX, volatile functions (they stay `#NAME?`), other parts of issue #9, `#REF!`/`#N/A`/`#NULL!`, `$` anchors, sheet names, semicolon separators.
