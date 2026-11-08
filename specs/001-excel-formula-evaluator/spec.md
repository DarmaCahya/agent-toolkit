# Feature Specification: Deterministic Excel Formula Evaluator

**Feature Branch**: `feat/excel-evaluator-issue-9` (spec directory `001-excel-formula-evaluator`)

**Created**: 2026-11-08

**Status**: Draft

**Input**: User description: "Implement GitHub issue #9 part 2 ONLY: pure-Rust deterministic Excel formula evaluator in crates/agent-canvas/src/math/excel (parser.rs Pratt parser, eval.rs). API: ExcelEvaluator::eval(\"=IF(SUM(10, 20) > 25, \\\"Profit\\\", \\\"Loss\\\")\") returns a Value (Number/Bool/Text/Error) with as_str(). Excel quirks must match: -3^2=9, 2^3^2=64, postfix %, & concat, comparisons case-insensitive, 0.1+0.2=0.3 is TRUE, errors (#DIV/0!,#VALUE!,#NAME?,#NUM!) are values not Rust Err, lazy IF/IFERROR, direct-arg coercion differs from range coercion. Phase-1 functions only: SUM AVERAGE MIN MAX COUNT IF AND OR NOT ABS ROUND IFERROR CONCAT. Limits: input<=8192 chars, nesting depth<=64, never panic. Out of scope: LaTeX, volatile functions. Make best-guess decisions, no questions; record assumptions in spec."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Evaluate a formula string to a result (Priority: P1)

An agent or developer has an Excel-style formula (for example, a computed cell from a spreadsheet or a label rule in a chart) and wants the value it would produce, without Excel. They pass the formula text to the evaluator and get back a result they can read as a number, true/false, text, or an error, and can render as a display string.

**Why this priority**: This is the whole feature. Without a working text-in, value-out evaluation of common formulas, nothing else matters.

**Independent Test**: Evaluate `=IF(SUM(10, 20) > 25, "Profit", "Loss")` and confirm the result is the text `Profit`, and that its display string is `Profit`. Evaluate `=SUM(1,2,3)*2` and confirm `12`.

**Acceptance Scenarios**:

1. **Given** the formula `=IF(SUM(10, 20) > 25, "Profit", "Loss")`, **When** evaluated, **Then** the result is text `Profit`.
2. **Given** the formula `=1+2*3`, **When** evaluated, **Then** the result is the number `7`.
3. **Given** a formula with or without the leading `=`, **When** evaluated, **Then** both forms produce the same result.
4. **Given** any result, **When** its display string is requested, **Then** numbers render without a trailing `.0` for whole values, booleans render as `TRUE`/`FALSE`, text renders as itself, and errors render as their Excel code (e.g. `#DIV/0!`).
5. **Given** the same formula evaluated twice (or on different platforms), **When** results are compared, **Then** they are identical.

---

### User Story 2 - Match Excel's operator and precedence behavior (Priority: P1)

A user migrating spreadsheet logic expects the numbers to agree with Excel, including its well-known quirks, so they can trust the output without re-checking against Excel.

**Why this priority**: A calculator that disagrees with Excel on common expressions is worse than none, because wrong answers look authoritative.

**Independent Test**: Run the quirk table below; every row must produce the stated result.

**Acceptance Scenarios**:

1. **Given** `=-3^2`, **When** evaluated, **Then** the result is `9` (unary minus binds tighter than `^`).
2. **Given** `=2^3^2`, **When** evaluated, **Then** the result is `64` (`^` is left-associative).
3. **Given** `=50%`, **When** evaluated, **Then** the result is `0.5`; and `=200*10%` gives `20` (postfix percent).
4. **Given** `="a"&"b"&1`, **When** evaluated, **Then** the result is text `ab1`; and `&` binds looser than `+` but tighter than comparisons.
5. **Given** `="ABC"="abc"`, **When** evaluated, **Then** the result is `TRUE` (text comparison is case-insensitive); likewise `<>`, `<`, `>`, `<=`, `>=` on text.
6. **Given** `=0.1+0.2=0.3`, **When** evaluated, **Then** the result is `TRUE` (comparison tolerates floating-point noise at Excel's 15-significant-digit precision).
7. **Given** a comparison between mixed types, **When** evaluated, **Then** ordering follows Excel: numbers < text < booleans, with no error raised.

---

### User Story 3 - Errors are values, not failures (Priority: P1)

A caller evaluating many formulas (some bad, some adversarial) needs one broken formula to yield an Excel-style error result they can inspect or branch on, instead of a crash or a thrown failure that aborts the batch.

**Why this priority**: Callers are autonomous agents processing untrusted input. Error-as-value is what makes `IFERROR` possible and batch evaluation safe.

**Independent Test**: Evaluate `=1/0`, `="x"+1`, `=FOO(1)`, `=10^1000` (numeric failure), and malformed text such as `=1+`; each returns an error value, none panics, none returns a Rust-level failure.

**Acceptance Scenarios**:

1. **Given** `=1/0`, **When** evaluated, **Then** the result is the error `#DIV/0!`.
2. **Given** `="abc"+1`, **When** evaluated, **Then** the result is `#VALUE!`.
3. **Given** `=NOSUCH(1)`, **When** evaluated, **Then** the result is `#NAME?`.
4. **Given** a numeric result that is out of range or not finite (e.g. `=10^1000`), **When** evaluated, **Then** the result is `#NUM!`.
5. **Given** an error produced inside a larger expression, **When** the expression continues, **Then** the error propagates (e.g. `=1+(1/0)` is `#DIV/0!`), except where a function is defined to catch it.
6. **Given** syntactically invalid text (`=1+`, `=(1`, `=SUM(`, empty input), **When** evaluated, **Then** an error value is returned rather than a failure or panic.

---

### User Story 4 - Use the Phase-1 function set (Priority: P2)

A user composes formulas from the common functions: `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT`, `IF`, `AND`, `OR`, `NOT`, `ABS`, `ROUND`, `IFERROR`, `CONCAT`. Function names are case-insensitive.

**Why this priority**: Operators alone are a calculator; the function set is what makes realistic business formulas possible. It follows P1 because the operator/value core must exist first.

**Independent Test**: Evaluate one formula per function and compare with the Excel-documented result.

**Acceptance Scenarios**:

1. **Given** `=ROUND(2.5,0)`, **When** evaluated, **Then** the result is `3` (half away from zero), and `=ROUND(-2.5,0)` is `-3`; negative digit counts round to tens/hundreds (`=ROUND(1234,-2)` is `1200`).
2. **Given** `=IFERROR(1/0, "n/a")`, **When** evaluated, **Then** the result is `n/a`; and the fallback is not evaluated when the first argument succeeds.
3. **Given** `=IF(TRUE, 1, 1/0)`, **When** evaluated, **Then** the result is `1` (the untaken branch is never evaluated, so its error never occurs).
4. **Given** `=AND(TRUE, 1, 0)` / `=OR(FALSE, 0)` / `=NOT(0)`, **When** evaluated, **Then** the results are `FALSE` / `FALSE` / `TRUE`.
5. **Given** `=CONCAT("a", 1, TRUE)`, **When** evaluated, **Then** the result is text `a1TRUE`.
6. **Given** `=AVERAGE()` with no numeric values, `=MIN()` / `=MAX()` with none, **When** evaluated, **Then** `AVERAGE` yields `#DIV/0!` and `MIN`/`MAX` yield `0`, as in Excel; `=SUM()` yields `0` and `=COUNT()` yields `0`.
7. **Given** `=IF(1)` (missing argument) or too many arguments to a fixed-arity function, **When** evaluated, **Then** the result is an error value (not a failure).

---

### User Story 5 - Direct arguments vs. ranges coerce differently (Priority: P2)

A user passes values both directly (`=SUM("3", TRUE)`) and as cell ranges (`=SUM(A1:A3)`) and expects Excel's distinct rules for each: text and booleans typed directly into a function are coerced to numbers, while text and booleans stored inside a range are ignored by numeric aggregates.

**Why this priority**: This is a documented Excel trap and a frequent source of silent wrong totals. It matters for correctness but only after the core and function set work.

**Independent Test**: Evaluate `=SUM("3", TRUE)` → `4`; with a range containing the same text `"3"` and `TRUE`, `=SUM(range)` → `0`; `=COUNT("3", TRUE, 5)` → `3` while `=COUNT(range)` counts only true numbers.

**Acceptance Scenarios**:

1. **Given** `=SUM("3", TRUE, 2)`, **When** evaluated, **Then** the result is `6` (numeric text and booleans coerced).
2. **Given** `=SUM("abc")`, **When** evaluated, **Then** the result is `#VALUE!` (non-numeric text passed directly).
3. **Given** a range holding `"3"`, `TRUE`, and `2`, **When** `=SUM` is applied to it, **Then** the result is `2` (text and booleans in ranges are skipped).
4. **Given** a range containing an error value, **When** an aggregate is applied to it, **Then** the error propagates.
5. **Given** a range holding empty cells, **When** an aggregate is applied, **Then** empty cells are skipped and not counted.

---

### User Story 6 - Safe on hostile or oversized input (Priority: P2)

A system that evaluates formulas from untrusted sources needs hard guarantees: bounded input size, bounded nesting, and no crash on any input at all.

**Why this priority**: Required by the project's zero-panic and untrusted-input constraints; a violation is an outage, not a wrong answer.

**Independent Test**: Feed inputs of exactly 8192 and 8193 characters, nesting at depth 64 and 65, random byte/Unicode soup, and very long operator chains; confirm bounded behavior and no panic.

**Acceptance Scenarios**:

1. **Given** a formula longer than 8192 characters, **When** evaluated, **Then** an error value is returned without parsing it; a formula of exactly 8192 characters is accepted.
2. **Given** parentheses/function-call nesting deeper than 64, **When** evaluated, **Then** an error value is returned without stack exhaustion; depth exactly 64 is accepted.
3. **Given** any arbitrary string (including control characters, non-rendering Unicode, tag code points, lone operators), **When** evaluated, **Then** no panic occurs and a value is returned.
4. **Given** arithmetic that would overflow or produce NaN/infinity, **When** evaluated, **Then** `#NUM!` is returned.

---

### Edge Cases

- Empty string, whitespace-only, or just `=`: returns an error value.
- Whitespace and line breaks between tokens are ignored; whitespace inside string literals is preserved.
- String literals use double quotes with `""` as an escaped quote (`="a""b"` is `a"b`).
- Boolean literals `TRUE`/`FALSE` are case-insensitive.
- Numeric literals: integers, decimals, leading-dot (`.5`), and scientific notation (`1E3`).
- Non-rendering and tag Unicode code points in input are sanitized before parsing (project-wide rule), so they cannot smuggle tokens.
- `%` followed by operand-start (`=5% 3`) is a syntax error; stacked `%` (`=200%%`) applies twice.
- Unary plus is accepted; chained unary minus (`=--5`) yields `5`.
- Text-to-number coercion for direct arguments accepts surrounding whitespace and plain decimal/scientific forms only; locale-specific separators are not recognized.
- Division and `ROUND` with extreme values (huge digit counts) must not hang or panic.
- A comparison involving an error propagates the error rather than comparing.
- `IF` with a non-boolean, non-numeric condition (e.g. text) yields `#VALUE!`.
- Unknown function names are `#NAME?` even when the call is syntactically fine, and its arguments need not be evaluated.
- Volatile functions (`NOW`, `RAND`, `TODAY`, …) are not recognized and therefore yield `#NAME?`.

## Requirements *(mandatory)*

### Functional Requirements

**Evaluation interface**

- **FR-001**: The system MUST provide a single entry point that takes a formula string and returns a result value; a formula may begin with `=`, and the `=` is optional.
- **FR-002**: The result value MUST be one of four kinds: Number, Boolean, Text, or Error, and MUST offer a display-string form (see Acceptance Scenario 1.4).
- **FR-003**: The entry point MUST NOT return a failure/exception for any input; every problem, including syntax errors, is expressed as an Error value.
- **FR-004**: Error values MUST cover at least `#DIV/0!`, `#VALUE!`, `#NAME?`, and `#NUM!`; syntax errors and limit violations MUST map to one of these (default assumption: `#VALUE!` for syntax and limit violations, `#NAME?` for unknown functions).

**Operators and precedence** (highest to lowest)

- **FR-005**: Postfix `%` MUST divide by 100 and bind tighter than `^`.
- **FR-006**: Unary `-`/`+` MUST bind tighter than `^` (so `-3^2` = 9).
- **FR-007**: `^` MUST be left-associative (so `2^3^2` = 64).
- **FR-008**: `*` and `/` MUST bind tighter than `+` and `-`; `&` MUST bind looser than `+`/`-`; comparison operators `= <> < > <= >=` MUST bind loosest.
- **FR-009**: `&` MUST concatenate the text forms of both operands (numbers rendered per FR-002 display rules, booleans as `TRUE`/`FALSE`).
- **FR-010**: Parentheses MUST override precedence.

**Coercion and comparison**

- **FR-011**: Arithmetic operators MUST coerce operands: booleans to 1/0, numeric text to numbers; non-numeric text yields `#VALUE!`.
- **FR-012**: Text comparison MUST be case-insensitive.
- **FR-013**: Number comparison MUST treat values equal when they agree to 15 significant digits, so `0.1+0.2=0.3` is `TRUE`.
- **FR-014**: Mixed-type comparison MUST follow Excel ordering (numbers < text < booleans) and MUST NOT raise an error by itself.
- **FR-015**: An argument supplied directly to a function MUST be coerced by direct-argument rules (numeric text and booleans become numbers; non-numeric text is `#VALUE!`).
- **FR-016**: Values coming from a range MUST be coerced by range rules (text, booleans, and empty cells are ignored by numeric aggregates; error values propagate). This difference MUST be observable per Story 5.

**Errors**

- **FR-017**: An error in any operand MUST propagate through operators and functions unless the function is defined to handle it (`IFERROR`).
- **FR-018**: Division by zero MUST yield `#DIV/0!`; any non-finite or overflowing numeric result MUST yield `#NUM!`.

**Functions (Phase 1 only)**

- **FR-019**: The system MUST support `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT`, `IF`, `AND`, `OR`, `NOT`, `ABS`, `ROUND`, `IFERROR`, and `CONCAT`, with case-insensitive names and Excel-documented empty/edge results as described in Story 4.
- **FR-020**: `IF` MUST evaluate only the selected branch; `IFERROR` MUST evaluate its fallback only when the first argument is an error. Skipped arguments MUST have no effect.
- **FR-021**: `ROUND` MUST round halves away from zero and accept negative digit counts.
- **FR-022**: Calling a function with the wrong number of arguments MUST yield an Error value.
- **FR-023**: Functions outside the Phase-1 list, including all volatile functions, MUST yield `#NAME?`.

**Limits and safety**

- **FR-024**: Input longer than 8192 characters MUST be rejected with an Error value before parsing; exactly 8192 MUST be accepted.
- **FR-025**: Nesting deeper than 64 levels MUST be rejected with an Error value; exactly 64 MUST be accepted. Both parsing and evaluation MUST be bounded by this limit so deep input cannot exhaust the stack.
- **FR-026**: No input of any content MUST cause a panic, hang, or unbounded memory use.
- **FR-027**: Evaluation MUST be deterministic: same input, same output, on every platform, with no dependence on time, randomness, locale, or environment.
- **FR-028**: Input MUST have non-rendering Unicode and tag code points sanitized before parsing, consistent with the rest of the toolkit.

**Scope boundaries**

- **FR-029**: This feature covers formula evaluation only. LaTeX rendering and all other parts of GitHub issue #9 are out of scope.

### Key Entities

- **Formula**: Text supplied by the caller, up to 8192 characters, optionally beginning with `=`.
- **Value**: The outcome of evaluating a formula or sub-expression; exactly one of Number, Boolean, Text, or Error. Has a canonical display string.
- **Error value**: An Excel error code (`#DIV/0!`, `#VALUE!`, `#NAME?`, `#NUM!`) carried as an ordinary value that can propagate or be caught.
- **Range**: A caller-supplied group of values (numbers, text, booleans, empty, errors) passed to a function, subject to range coercion rules (see Assumptions).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the documented Excel-quirk examples in this spec (`-3^2`=9, `2^3^2`=64, `50%`=0.5, `0.1+0.2=0.3`=TRUE, case-insensitive compare, `&` concat) produce the stated result.
- **SC-002**: Each of the 13 Phase-1 functions has at least one passing example matching Excel-documented behavior, including its empty and error cases.
- **SC-003**: The headline example `=IF(SUM(10, 20) > 25, "Profit", "Loss")` returns `Profit`.
- **SC-004**: Across a fuzz run of at least 100,000 arbitrary inputs (including boundary lengths 8192/8193 and depths 64/65), there are zero panics, zero hangs, and every input returns a value.
- **SC-005**: Evaluating any single formula within the allowed limits completes in under 10 milliseconds on ordinary developer hardware.
- **SC-006**: Repeated and cross-platform evaluation of a fixed corpus of at least 200 formulas yields byte-identical display strings every time.
- **SC-007**: A batch containing valid and invalid formulas is fully evaluated in one pass: invalid ones yield error values and never interrupt the batch.

## Assumptions

- **Scope**: Only part 2 of issue #9 (the evaluator) is in scope. LaTeX and volatile functions are explicitly out of scope; volatile names are treated as unknown functions (`#NAME?`).
- **Ranges and cell references**: Because there is no spreadsheet in this feature, range values reach functions only through a caller-supplied way to provide values, so range-coercion rules are testable. Cell/range references written in a formula text (`A1`, `A1:B3`) resolve through that caller-supplied source; with none supplied they yield `#NAME?`. The exact shape of this hook is decided in planning (see research.md D3 and contracts/rust-api.md: `RangeProvider`).
- **Error codes**: Only the four listed error codes exist in Phase 1. Syntax errors and limit violations map to `#VALUE!` (assumption; Excel would refuse to accept such a formula at all). `#REF!`, `#N/A`, `#NULL!` are deferred.
- **Number formatting**: Whole-number results display without a decimal point; other numbers use the shortest round-trip decimal form, without locale-specific separators. Very large or small values use plain decimal or scientific form consistently, to be fixed in planning.
- **Floating-point equality**: "Equal at 15 significant digits" approximates Excel's comparison; results beyond that precision are not guaranteed to match Excel bit-for-bit.
- **Function names**: Case-insensitive. Excel-style argument separator is the comma; semicolon and locale-specific variants are not supported.
- **Character counting**: The 8192 limit counts Unicode characters, not bytes.
- **Nesting depth**: One level is counted for each open parenthesis and each function-call argument list, plus unary-operator chains, so that both parser and evaluator recursion are bounded.
- **Empty `AVERAGE`/`MIN`/`MAX`**: Follow Excel (`#DIV/0!`, `0`, `0`).
- **Unary minus precedence**: Follows Excel's (tighter than `^`), not mathematical convention, per the request.
- **Project constraints apply**: Pure Rust, no panics in library code, deterministic output, test-first with user-scenario tests, minimal dependencies (target: none). These come from the project constitution and are binding on the later plan, not on this spec's behavior.
- **Dependency**: Lives inside the existing `agent-canvas` crate's math module; no other crate is changed.
