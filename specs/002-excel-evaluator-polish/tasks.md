# Tasks: Excel Evaluator Polish

**Input**: Design documents from `specs/002-excel-evaluator-polish/` (plan.md, spec.md, research.md, data-model.md, contracts/public-api.md, quickstart.md)

**Prerequisites**: plan.md, spec.md (both present)

**Tests**: INCLUDED. Constitution Principle IV (test-first) is mandatory: within every story, write the failing tests first, run them and confirm they FAIL (or do not compile) before implementing.

**Organization**: Grouped by user story. All paths are relative to the repo root; crate root is `crates/agent-canvas/` (abbreviated `CRATE/`).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on incomplete tasks)
- **[Story]**: US1..US5, from spec.md

## Constraints (apply to every task)

- Allowed change surface: `CRATE/src/math/excel/**`, `CRATE/tests/**`, `CRATE/README.md`, `CRATE/examples/**`. Do NOT edit `CRATE/src/math/mod.rs` or `CRATE/Cargo.toml` (see Open Items in plan.md; deferred pending user approval).
- The `excel` module denies `unwrap`/`expect`/indexing/`panic`: use `get`, saturating ops, `?`, and `Result`.
- No new dependencies (`thiserror 2` already present). Output must be deterministic.
- Build/test commands: `cargo test -p agent-canvas`, `cargo clippy -p agent-canvas --all-targets -- -D warnings`, `cargo fmt --check`.

---

## Phase 1: Setup

**Purpose**: Establish a green baseline before changing anything.

- [X] T001 Run `cargo test -p agent-canvas` and `cargo clippy -p agent-canvas --all-targets -- -D warnings` from the repo root; record that the baseline is green (stop and report if not).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The `ParseError` type, span-carrying tokens, and `#[non_exhaustive]` `Value::Blank` variant are shared by all stories.

**⚠️ CRITICAL**: No user story work starts until this phase is complete.

**Test-first note (Principle IV)**: T002 writes its unit test before the type's Display impl. T004–T006 are behaviour-preserving refactors guarded by the existing green suite (T001 baseline); the new behaviour they enable is driven by the failing tests in US1–US4, which are written before the story implementation tasks.

- [X] T002 Create `CRATE/src/math/excel/error.rs` with `ParseError { pub kind: ParseErrorKind, pub span: Range<usize>, pub expected: Vec<&'static str> }` and `ParseErrorKind { UnexpectedEof, UnexpectedToken, WrongArgCount { name: String, min: usize, max: Option<usize>, got: usize }, LimitExceeded }`. Both `#[non_exhaustive]`, derive `Debug, Clone, PartialEq, Eq` and `thiserror::Error`. Display must read like `unexpected end of input at 5, expected expression or )` (kind, position, expected items joined with ", " and a final " or "). Add a constructor (e.g. `ParseError::new(kind, span, expected)`) since the struct is non-exhaustive. Add a unit test in the same file asserting `ParseError: Send + Sync + std::error::Error` and the Display string.
- [X] T003 Wire `mod error;` and `pub use error::{ParseError, ParseErrorKind};` in `CRATE/src/math/excel/mod.rs` (T002 first).
- [X] T004 Add span tracking to the lexer in `CRATE/src/math/excel/lexer.rs` (depends on T002, T003): each token carries a half-open byte span into the sanitized body; lexer failures return `ParseError` (`UnexpectedToken`, or `LimitExceeded` for the existing length limit) instead of its current error form. Existing lexer unit tests must still pass (adjust only the assertions needed for the new return type).
- [X] T005 Change the parser in `CRATE/src/math/excel/parser.rs` to return `Result<_, ParseError>`: unexpected end of input → `UnexpectedEof` with an empty span at end of input; wrong token → `UnexpectedToken` with the token span; nesting beyond the existing depth limit (64) → `LimitExceeded`. Populate `expected` with static labels in fixed order (`"expression"`, `")"`, `","`, ...): directly after `(` at end of input `["expression", ")"]`; after `,` `["expression"]`. Depends on T002, T004.
- [X] T006 Add `Value::Blank` and `#[non_exhaustive]` to `Value` in `CRATE/src/math/excel/value.rs`. Make every existing `match` over `Value` in `value.rs`, `compare.rs`, `eval.rs`, `functions.rs` compile (temporary behaviour: Blank treated like the number 0 / no new semantics yet; real semantics land in US3). Confirm `cargo build -p agent-canvas` and existing tests pass.

**Checkpoint**: `cargo test -p agent-canvas` green with typed parse errors internally and `Value::Blank` present but not yet produced.

---

## Phase 3: User Story 1 - Diagnose a malformed formula (Priority: P1) 🎯 MVP

**Goal**: A fallible entry point returns located, structured errors; the infallible one stays total.

**Independent Test**: `try_eval("=SUM(")` is `Err` with `UnexpectedEof`, span `5..5`, expected `["expression", ")"]`; `eval("=SUM(")` is `#VALUE!`; `try_eval("=1+2")` is `Ok(Number(3.0))`.

### Tests for User Story 1 (write first, confirm FAIL)

- [X] T007 [P] [US1] In `CRATE/tests/excel_tests.rs` add scenario tests: `=SUM(` (kind `UnexpectedEof`, span `5..5`, expected `["expression", ")"]`); `=SUM(1,` (EOF, expected `["expression"]`); `=IF(`; `=1+2` Ok(3); `eval` of each malformed input yields `Value::Error(ExcelError::Value)`; empty input and bare `=` never panic on both paths; input over 8192 chars and nesting over 64 give `LimitExceeded` on `try_eval` and `#VALUE!` on `eval`; a stray token (e.g. `=1 2`) gives `UnexpectedToken` with the offending span; an input with a leading space / invisible Unicode character before the error still reports offsets relative to the ORIGINAL string; `ParseError` Display contains kind, position, and expected items.
- [X] T008 [P] [US1] In `CRATE/tests/excel_proptest.rs` add a property: for arbitrary strings (including unicode), `eval` returns a value without panic, and `try_eval` is `Ok` iff `eval` is not the parse-failure `#VALUE!` case where distinguishable (at minimum: never panics, and any `Err` has `span.start <= span.end <= input.len()`).

### Implementation for User Story 1

- [X] T009 [US1] In `CRATE/src/math/excel/mod.rs` implement `ExcelEvaluator::try_eval(formula) -> Result<Value, ParseError>` and `try_eval_with(formula, &dyn RangeProvider)`. Make the internal `evaluate()` return `Result`, and build a body-offset → original-byte-offset table (accounting for stripped invisible chars, trimming, and the leading `=`) so every reported span refers to the caller's string. Redefine `eval`/`eval_with` as `try_*` with any `Err` mapped to `Value::Error(ExcelError::Value)`; their signatures are unchanged. Depends on T005.
- [X] T010 [US1] Add rustdoc with a `try_eval` example (`=SUM(` error) to `CRATE/src/math/excel/mod.rs`, and run T007/T008 until green. Note: `as_str` doc lines in this file are migrated in US2 (T014), not here.

**Checkpoint**: US1 independently verifiable via T007/T008.

---

## Phase 4: User Story 2 - `as_str` vs `to_text` (Priority: P1)

**Goal**: `as_str()` returns `Option<&str>` for Text only; the old coercing behaviour is `to_text()`; `Display` added.

**Independent Test**: `eval(r#"=IF(SUM(10,20)>25,"Profit","Loss")"#).as_str() == Some("Profit")`; `eval("=1+1").as_str() == None` and `.to_text() == "2"`.

### Tests for User Story 2 (write first, confirm FAIL)

- [X] T011 [P] [US2] In `CRATE/src/math/excel/value.rs` unit tests: `as_str()` is `Some` for Text only and `None` for Number, Bool, Error, Blank; `to_text()` for each variant equals the pre-change `as_str()` output (numbers, `TRUE`/`FALSE`, error codes like `#DIV/0!`, Blank → `"0"`); `format!("{v}") == v.to_text()` for every variant.
- [X] T012 [P] [US2] In `CRATE/tests/excel_tests.rs` add the issue #9 scenario: `as_str() == Some("Profit")`, and `=1+1` gives `as_str() == None` with `to_text() == "2"`.

### Implementation for User Story 2

- [X] T013 [US2] In `CRATE/src/math/excel/value.rs`: rename the current `as_str(&self) -> Cow<str>` to `to_text(&self) -> Cow<'_, str>` (output unchanged, Blank → `"0"`), add `as_str(&self) -> Option<&str>` (Text only), and `impl fmt::Display for Value` writing `to_text()`. Fix any internal callers of the old `as_str` in `src/math/excel/**` to use `to_text()`.
- [X] T014 [P] [US2] Migrate docs: in `CRATE/src/math/excel/mod.rs` doc examples change `assert_eq!(v.as_str(), "Profit")` to `Some("Profit")` and the `"9"` / `"#DIV/0!"` assertions to `.to_text()`; in `CRATE/README.md` (line ~27) change the Profit assertion to `Some("Profit")` and adjust any other snippet using the old meaning.
- [X] T015 [P] [US2] Migrate `CRATE/examples/issue-009-demo/main.rs` (lines ~41 and ~48): use `to_text()` (or `{}` Display) instead of `as_str()`.
- [X] T016 [P] [US2] Migrate `CRATE/tests/excel_tests.rs`: helper at line ~12 (`.as_str().into_owned()` → `.to_text().into_owned()`), line ~68 (`Some("Profit")`), line ~697 (`.to_text().into_owned()`). Run `grep -rn "as_str()" CRATE/README.md CRATE/examples CRATE/tests CRATE/src/math/excel` and confirm no remaining use of the old coercing meaning.

**Checkpoint**: `cargo test -p agent-canvas` (incl. `--doc`) green; example compiles (`cargo build -p agent-canvas --examples`).

---

## Phase 5: User Story 3 - Blank cells behave like Excel (Priority: P2)

**Goal**: Single empty-cell references evaluate to `Blank`, with the coercion matrix from research.md R5.

**Independent Test**: With a provider returning an empty `A1`: `=A1=0` TRUE, `=A1=""` TRUE, `=A1&"x"` → `x`, `=A1+1` → 1, `=A1<1` TRUE, `=A1` → Blank displaying `0`.

### Tests for User Story 3 (write first, confirm FAIL)

- [X] T017 [P] [US3] In `CRATE/tests/excel_tests.rs` add a provider-backed scenario test (reuse the existing provider/`Sheet` helper pattern in that file) covering every acceptance scenario in spec.md US3, plus: `=A1=A1`-style blank vs blank equal; `=IF(A1,1,2)` → 2; `=AND(A1)` FALSE; `=OR(A1)` FALSE; `=NOT(A1)` TRUE; `=ROUND(A1,0)` → 0; `=ABS(A1)` → 0; `=CONCAT(A1,"x")` → `x`; `=IFERROR(A1,5)` → Blank; unary `-A1` and `A1%` → 0; error dominance (`=A1+#DIV/0!`-style via `1/0` operand) still yields the error; a range containing empty cells still skips them (`=SUM(A1:A3)` unchanged); `=A1` result has `as_str() == None` and `to_text() == "0"`.
- [X] T018 [P] [US3] In `CRATE/src/math/excel/compare.rs` unit tests: Blank vs Number (as 0), vs Text (as ""), vs Bool (as FALSE), vs Blank (equal), ordering comparisons with Blank, Error operand dominates.

### Implementation for User Story 3

- [X] T019 [US3] In `CRATE/src/math/excel/value.rs` add internal coercion helpers: numeric (Blank → 0), concat/text (Blank → ""), logical (Blank → FALSE). Keep public `to_text()`/`Display` giving `"0"` for Blank. Never panic.
- [X] T020 [US3] In `CRATE/src/math/excel/compare.rs` implement Blank comparisons per research.md R5 (replace the temporary behaviour from T006). Depends on T019.
- [X] T021 [US3] In `CRATE/src/math/excel/eval.rs` make a single-cell reference to an empty cell (`Cell::Empty` or `Cell::Value(Blank)`) evaluate to `Value::Blank` when used as a scalar; ranges keep skipping empty cells. Route arithmetic, `&`, unary `-`, `%`, and logical contexts through the T019 helpers. Depends on T019.
- [X] T022 [US3] In `CRATE/src/math/excel/functions.rs` route `IF`/`AND`/`OR`/`NOT`/`ROUND`/`ABS`/`IFERROR`/`CONCAT` scalar arguments through the T019 helpers so Blank coerces per the matrix (SUM/AVERAGE/MIN/MAX/COUNT range handling unchanged). Depends on T019. Run T017/T018 until green.

**Checkpoint**: US3 independently verifiable; US1/US2 still green.

---

## Phase 6: User Story 4 - Arity validated at parse time (Priority: P2)

**Goal**: Wrong argument counts for known functions fail at parse time with `WrongArgCount`.

**Independent Test**: `try_eval` of `=ROUND(1)`, `=IF(1)`, `=SUM()`, `=NOT(1,2)`, `=IF(TRUE,1,ROUND(1))` each returns `WrongArgCount` with correct name/min/max/got.

### Tests for User Story 4 (write first, confirm FAIL)

- [X] T023 [P] [US4] In `CRATE/tests/excel_tests.rs` add a table-driven boundary test (SC-005) for every function: `SUM, AVERAGE, MIN, MAX, COUNT, AND, OR, CONCAT` with 0 args → `WrongArgCount { min: 1, max: None, got: 0 }` and 1 arg OK; `IF` with 1 and 4 args → `{min: 2, max: Some(3)}`, 2 and 3 OK; `ROUND` and `IFERROR` with 1 and 3 args → `{min: 2, max: Some(2)}`, 2 OK; `NOT` and `ABS` with 0 and 2 args → `{min: 1, max: Some(1)}`, 1 OK. Also: `name` is the canonical uppercase name; lowercase `sum()` fails like `SUM()`; `=IF(TRUE,1,ROUND(1))` is rejected; nested calls (`=SUM(1,ROUND(2))`) are rejected; unknown function `=FOO()` still parses and evaluates to `#NAME?`; `eval` of every arity violation is `#VALUE!`.
- [X] T024 [P] [US4] In `CRATE/src/math/excel/functions.rs` unit tests: arity table values match data-model.md (variadics `(1, MAX)`, etc.).

### Implementation for User Story 4

- [X] T025 [US4] In `CRATE/src/math/excel/functions.rs` change the arity table so variadic functions have minimum 1 (`SUM AVERAGE MIN MAX COUNT AND OR CONCAT`), keeping `IF` 2–3, `ROUND`/`IFERROR` 2–2, `NOT`/`ABS` 1–1; expose a lookup usable by the parser (case-insensitive, `None` for unknown names). Keep the runtime arity guard (returns `#VALUE!`, never panics) as defence in depth.
- [X] T026 [US4] In `CRATE/src/math/excel/parser.rs` validate arity when a `Call` node is built: known names only, case-insensitive, otherwise `ParseErrorKind::WrongArgCount { name (uppercase), min, max, got }` with the span of the call (function name through closing parenthesis; if unavailable, the closing-paren/name span) and empty `expected`. Depends on T005, T025. Run T023/T024 until green.
- [X] T027 [US4] Update tests that relied on zero-argument variadics returning a value: in `CRATE/src/math/excel/functions.rs` `empty_variadics` (line ~305) call the runtime function helper directly or assert the new behaviour; in `CRATE/tests/excel_tests.rs` lines ~244–248 and ~513–528 the entries `=SUM()`, `=AVERAGE()`, `=MIN()`, `=MAX()` (and any other zero-arg variadic such as `=COUNT()`, `=AND()`, `=OR()`, `=CONCAT()`) now expect `#VALUE!` via `eval` (and `WrongArgCount` via `try_eval`). NOTE: the spec names only `MAX()` (FR-016), but arity rule FR-014 makes all zero-arg variadics invalid, so those expectations change too; list every changed expectation in the final report.

**Checkpoint**: US4 independently verifiable.

---

## Phase 7: User Story 5 - Ranges with no numbers (Priority: P3)

**Goal**: Lock in the already-implemented behaviour (research.md R7) with regression tests; no production change expected.

**Independent Test**: All-empty or all-text range: `AVERAGE` → `#DIV/0!`, `MAX` → 0, `MIN` → 0.

- [X] T028 [P] [US5] In `CRATE/tests/excel_tests.rs` add provider-backed tests: an entirely empty range and an all-text range each give `AVERAGE` → `#DIV/0!`, `MAX` → 0, `MIN` → 0; a range of only blanks via `Cell::Value(Blank)` behaves the same. Run them; they should pass without production changes. If any fails, fix it minimally in `CRATE/src/math/excel/functions.rs` (`numeric()`).

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T029 Run `cargo fmt`, then `cargo clippy -p agent-canvas --all-targets -- -D warnings`, `cargo test -p agent-canvas` (lib, integration, proptest, doc tests), and `cargo build -p agent-canvas --examples`; fix any failure.
- [X] T030 Run the quickstart scenarios from `specs/002-excel-evaluator-polish/quickstart.md` (including `cargo run -p agent-canvas --example issue_009_demo`; the example name is declared in `CRATE/Cargo.toml`, the directory is `issue-009-demo`) and confirm all five scenarios and the final `grep -rn "as_str()"` check pass.
- [X] T031 Verify the diff touches only the allowed surface: `git status --short` and `git diff --stat` must list nothing outside `CRATE/src/math/excel/**`, `CRATE/tests/**`, `CRATE/README.md`, `CRATE/examples/**`, `specs/**`. Report the deferred items (re-export in `src/math/mod.rs`, version bump + changelog in `Cargo.toml`) to the user for a decision rather than doing them.

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (P1) → Foundational (P2) → all user stories → Polish (P8).
- **US1 (P1)**: after Phase 2. No story dependencies. 🎯 MVP.
- **US2 (P1)**: after Phase 2; independent of US1 except both edit `mod.rs` (T010 vs T014) and `excel_tests.rs`, so do not run them concurrently on the same files.
- **US3 (P2)**: after Phase 2 and ideally after US2 (needs `to_text()` for the `"0"` display checks).
- **US4 (P2)**: after Phase 2 (needs the parser's `ParseError` from T005); independent of US3.
- **US5 (P3)**: after Phase 2; best after US3 (blank-only range case).

### Within Each Story

Failing tests → implementation → run tests green. Tasks in the same file are sequential even when logically independent.

### Parallel Opportunities

- Phase 2: T004 returns `ParseError`, so it needs T002 (and the `mod error;` wiring from T003) first; T005 needs T002 and T004; T006 does not depend on T002 and can run in parallel with T004/T005 (it touches `value.rs`, `compare.rs`, `eval.rs`, `functions.rs`, not the parser/lexer).
- US1 tests T007 ‖ T008 (different files). US2 T011 ‖ T012; T014 ‖ T015 ‖ T016 (different files). US3 T017 ‖ T018. US4 T023 ‖ T024.
- Different stories can proceed in parallel only if they avoid simultaneous edits to `excel_tests.rs`, `mod.rs`, `functions.rs` (US3 and US4 both touch `functions.rs`: sequence T022 before T025/T027).

### Parallel Example: User Story 2

```text
Task: "T014 Migrate docs in CRATE/src/math/excel/mod.rs and CRATE/README.md"
Task: "T015 Migrate CRATE/examples/issue-009-demo/main.rs"
Task: "T016 Migrate CRATE/tests/excel_tests.rs call sites"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 → Phase 2 → Phase 3 (US1). 2. STOP and validate with T007/T008. 3. Note that US2 is also P1 and is small; deliver US1 + US2 together as the first reviewable increment because the issue #9 example depends on US2.

### Incremental Delivery

Foundation → US1 → US2 → US3 → US4 → US5 → Polish. Each step leaves `cargo test -p agent-canvas` green; commit after each story.

## Notes

- Spans are byte offsets into the ORIGINAL input, half-open, empty at end of input.
- Do not add dependencies or touch files outside the allowed surface without asking.
