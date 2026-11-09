# Feature Specification: Excel Evaluator Polish

**Feature Branch**: `fix/excel-evaluator-polish` (existing branch; none created — spec directory `002-excel-evaluator-polish`)

**Created**: 2026-11-09

**Status**: Draft

**Input**: User description: "Polish the existing Excel evaluator in crates/agent-canvas/src/math/excel (already merged): (1) structured parse errors and a fallible entry point, keeping the infallible one total; (2) `Value::as_str()` returns text only for Text values, with the old coercing behaviour renamed and `Display` added; (3) first-class blank cells with Excel-style coercion; (4) function arity validated at parse time, plus defined runtime results for AVERAGE/MAX/MIN over ranges with no numbers."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Diagnose a malformed formula (Priority: P1)

An agent or developer submits a syntactically wrong formula and needs to know *where* and *why* it failed, so it can repair the formula instead of seeing a generic `#VALUE!`.

**Why this priority**: Actionable errors are the core of agent self-correction; today a typo and a legitimate `#VALUE!` result are indistinguishable.

**Independent Test**: Call the fallible entry point with `=SUM(` and inspect the error; call the infallible entry point with the same text and confirm it still returns a value.

**Acceptance Scenarios**:

1. **Given** `=SUM(`, **When** evaluated through the fallible entry point, **Then** it fails with an "unexpected end of input" error at offset 5 whose expected items are "expression" and ")".
2. **Given** `=1+2`, **When** evaluated through the fallible entry point, **Then** it succeeds with the number 3.
3. **Given** any malformed formula, **When** evaluated through the infallible entry point, **Then** the result is the `#VALUE!` error value and nothing panics.
4. **Given** a parse error, **When** printed or passed across threads or error-handling layers, **Then** it renders a readable message (kind, position, expected items) and is safe to share between threads.
5. **Given** a future release adds new error kinds, **When** downstream code matches on the error or its kind, **Then** it is not broken.

---

### User Story 2 - Distinguish "is text" from "render as text" (Priority: P1)

A developer wants to know whether a result is literally a text value (e.g. `"Profit"`), separately from wanting a display string for any value.

**Why this priority**: The issue #9 example asserts `val.as_str() == Some("Profit")`; the current API contradicts it and conflates two intents.

**Independent Test**: Evaluate `=IF(SUM(10,20)>25,"Profit","Loss")` and assert `as_str()` is `Some("Profit")`; evaluate `=1+1` and assert `as_str()` is `None` while its display text is `2`.

**Acceptance Scenarios**:

1. **Given** a Text value, **When** `as_str()` is called, **Then** it returns the text; for Number, Bool, Error and Blank it returns nothing.
2. **Given** any value, **When** `to_text()` is called, **Then** it returns exactly what the former `as_str()` returned.
3. **Given** any value, **When** formatted for display, **Then** the output equals `to_text()`.
4. **Given** the README, crate docs, examples and tests, **When** built and run, **Then** all use the new semantics.

---

### User Story 3 - Blank cells behave like Excel (Priority: P2)

A user evaluates formulas that reference empty cells and gets Excel-faithful results instead of a silent `0` that loses the "was empty" distinction.

**Why this priority**: Empty cells are extremely common; a blank must satisfy both `=A1=0` and `=A1=""`.

**Independent Test**: With a provider returning an empty cell for `A1`, evaluate `=A1=0`, `=A1=""`, `=A1&"x"`, `=A1+1`, `=A1`.

**Acceptance Scenarios**:

1. **Given** blank `A1`, **When** `=A1=0`, **Then** TRUE.
2. **Given** blank `A1`, **When** `=A1=""`, **Then** TRUE.
3. **Given** blank `A1`, **When** `=A1&"x"`, **Then** text `x`.
4. **Given** blank `A1`, **When** `=A1+1`, **Then** 1; and `=A1<1` is TRUE.
5. **Given** blank `A1`, **When** `=A1` is the whole formula, **Then** the result is blank and its display text is `0`.
6. **Given** a single-cell reference to an empty cell used as a scalar, **Then** it yields blank, not the number 0.

---

### User Story 4 - Catch wrong argument counts before evaluation (Priority: P2)

A developer or agent calls a function with the wrong number of arguments and is told so up front, with function name, allowed range and actual count.

**Why this priority**: Turns silent runtime results (such as `MAX()==0`) into precise, diagnosable errors.

**Independent Test**: Parse `=ROUND(1)`, `=IF(1)`, `=SUM()`, `=NOT(1,2)`; each reports a wrong-count error with correct name/min/max/got.

**Acceptance Scenarios**:

1. **Given** `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT`, `AND`, `OR`, `CONCAT` with zero arguments, **Then** the error reports min 1, no maximum, got 0.
2. **Given** `IF` with 1 or 4+ arguments, **Then** error with min 2, max 3.
3. **Given** `ROUND` or `IFERROR` with ≠2 arguments, or `NOT`/`ABS` with ≠1, **Then** error with min = max = required count.
4. **Given** valid counts, **Then** parsing succeeds and evaluation is unchanged.
5. **Given** a violation inside an unevaluated branch (`=IF(TRUE,1,ROUND(1))`), **Then** it is still reported, because validation is at parse time.
6. **Given** the infallible entry point and an arity violation, **Then** the result is `#VALUE!`.

---

### User Story 5 - Defined results for ranges with no numbers (Priority: P3)

A user aggregates a range that is entirely empty or non-numeric and gets Excel's defined answers.

**Independent Test**: With an all-empty range, evaluate `AVERAGE`, `MAX`, `MIN`.

**Acceptance Scenarios**:

1. **Given** a range with no numeric cells, **When** `AVERAGE` runs, **Then** `#DIV/0!`.
2. **Given** such a range, **When** `MAX` or `MIN` runs, **Then** 0.
3. **Given** existing tests asserting that a zero-argument variadic call (`MAX()`, `MIN()`, `SUM()`, `AVERAGE()`, `COUNT()`, `AND()`, `OR()`, `CONCAT()`) returns a value, **Then** they are updated to expect the arity error (`#VALUE!` via the infallible entry point).

### Edge Cases

- `=SUM(`, `=SUM(1,`, `=IF(` — end of input at differing offsets.
- Empty input, input of only `=`, or input beyond the existing length/nesting limits: never panic; fallible path returns an error (limit violations get their own kind), infallible path returns `#VALUE!`.
- Unknown function names remain `#NAME?` values, not parse errors; arity is checked only for known functions.
- Function names are case-insensitive for arity checks (`sum()` fails like `SUM()`).
- Blank inside a range keeps existing range semantics (empty cells ignored); blank passed directly as a scalar follows scalar coercion (0 / "").
- Blank in a logical context (`=IF(A1,...)`, `AND(A1)`) is FALSE; blank compared with a Bool reads as FALSE.
- Blank flowing through `ROUND`, `ABS`, `NOT`, `IFERROR`, `CONCAT` coerces appropriately and never panics.
- Error values still dominate blank (an error operand yields that error).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The library MUST expose a structured parse-error type carrying a kind, the offending source span, and the list of expected items; it MUST implement readable display and the standard error trait, be safe to send/share across threads, and be extensible without breaking downstream code.
- **FR-002**: The library MUST expose a fallible evaluation entry point returning either the computed value or the parse error.
- **FR-003**: The existing infallible entry point MUST remain total: every parse error maps to `#VALUE!`, and no input may cause a panic.
- **FR-004**: For `=SUM(`, the fallible entry point MUST report an unexpected-end-of-input error at offset 5 with expected items "expression" and ")".
- **FR-005**: `Value::as_str()` MUST return text only for Text values and nothing for every other variant.
- **FR-006**: The previous coercing conversion MUST be available as `to_text()` with output identical to the old `as_str()`.
- **FR-007**: `Value` MUST implement display formatting equal to `to_text()`.
- **FR-008**: All callers, tests, doc examples, README snippets and example programs MUST be migrated and pass.
- **FR-009**: A distinct blank value MUST exist; the value type MUST be extensible so adding it does not constitute a further break for future additions.
- **FR-010**: Blank MUST coerce to 0 in arithmetic and numeric comparison, and to the empty string in concatenation and text comparison; `=blank=0` and `=blank=""` MUST both be TRUE; `blank&"x"` MUST be `x`.
- **FR-011**: A formula whose final result is blank MUST display as `0`.
- **FR-012**: A single-cell reference to an empty cell used as a scalar MUST evaluate to blank, not the number 0.
- **FR-013**: Function arity MUST be validated at parse time and reported with a wrong-arg-count error kind containing function name, minimum, optional maximum, and actual count.
- **FR-014**: Required arities: `MAX`, `MIN`, `SUM`, `AVERAGE`, `COUNT`, `AND`, `OR`, `CONCAT` ≥ 1; `IF` 2–3; `ROUND` exactly 2; `NOT`, `ABS` exactly 1; `IFERROR` exactly 2.
- **FR-015**: `AVERAGE` over a range with no numeric cells MUST yield `#DIV/0!`; `MAX`/`MIN` over such a range MUST yield 0.
- **FR-016**: Existing tests relying on a zero-argument variadic call (`MAX()` and the other FR-014 variadics) returning a value MUST be updated to expect the arity error; all other existing behaviour MUST be preserved.
- **FR-017**: All new behaviour MUST remain deterministic and respect existing input-length and nesting limits.

### Key Entities

- **Parse error**: kind (unexpected end of input, unexpected token, wrong argument count, limit exceeded), source span, expected items; renders as a message.
- **Value**: Number, Bool, Text, Error, and now Blank; offers a text-only view (`as_str`), coerced text (`to_text`), and display.
- **Function arity table**: per known function, minimum and optional maximum argument counts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of malformed-formula test cases (unterminated call, missing operand, stray token, bad arity) return a located error naming what was expected; the `=SUM(` case matches offset 5 exactly.
- **SC-002**: The infallible entry point returns a value for 100% of inputs in the existing property tests, with zero panics.
- **SC-003**: The issue #9 example (`as_str()` equals `Some("Profit")`) runs as a passing test.
- **SC-004**: All blank-cell examples (`=0` TRUE, `=""` TRUE, `&"x"`, `+1`, final-blank displays `0`) pass as scenario tests.
- **SC-005**: Every function in FR-014 has a passing test at each boundary (one below minimum, at minimum, above maximum where applicable).
- **SC-006**: The workspace builds, all tests (with the documented zero-argument variadic updates, FR-016) pass, doc tests and examples compile, and no documentation snippet uses the old `as_str()` meaning.

## Assumptions

- **No new branch**: work stays on `fix/excel-evaluator-polish`; the `002` spec number is independent of the branch name.
- **Offset unit**: byte offsets into the original input (equal to character offsets for the `=SUM(` example); span is half-open, empty at end of input.
- **Expected-items wording**: static labels `expression` and `)` for the `=SUM(` case; other labels chosen consistently by the parser.
- **Unknown functions**: not a parse error; remain `#NAME?` at evaluation.
- **Limit violations** (length, nesting) become a parse-error kind on the fallible path and `#VALUE!` on the infallible path, matching today's observable behaviour.
- **Value extensibility**: the value type is made extensible when Blank is added; the `as_str()` signature change is inherently breaking, so a version bump and changelog note follow project semver policy. Both edit files outside this feature's change surface (`Cargo.toml`; no CHANGELOG exists) and are deferred for user decision (see plan.md Open Items).
- **Blank in logical context** is FALSE; blank compared with blank is equal.
- **Blank display vs coercion**: blank renders as `0` as a final result (display / `to_text()`), but participates as the empty string in `&` and text comparison.
- **Dependencies**: no new third-party dependencies beyond the project's existing typed-error convention; tests are written first per the constitution.
- **Out of scope**: new functions, localisation, cross-sheet references, volatile functions, numeric formatting changes.
