# Research: Excel Evaluator Polish

No external unknowns; decisions come from reading `crates/agent-canvas/src/math/excel`.

## R1 Where do offsets come from?
- Finding: tokens have no positions; the lexer indexes a `Vec<char>`; input is sanitized (invisible chars removed), trimmed and stripped of `=` before lexing, so lexer offsets differ from caller offsets.
- Decision: lexer records byte spans in the sanitized body; `evaluate()` builds a body-to-original byte-offset table and translates spans.
- Rationale: spec requires byte offsets into the original input. Alternatives: report offsets in sanitized text (wrong when invisible chars exist); sanitize lazily inside the lexer (larger refactor).

## R2 Error type shape
- Decision: `#[non_exhaustive] struct ParseError { kind, span, expected }` with `#[non_exhaustive] enum ParseErrorKind { UnexpectedEof, UnexpectedToken, WrongArgCount { name, min, max: Option<usize>, got }, LimitExceeded }`, derived with `thiserror`. All fields owned or `'static`, so `Send + Sync`.
- Alternatives: one enum with embedded spans (no uniform span access); `String` message (violates Principle V).

## R3 Fallible API naming
- Decision: `ExcelEvaluator::try_eval` and `try_eval_with`, following the `std` `try_*` convention; existing names untouched.

## R4 `expected` labels
- Decision: `&'static str` labels in deterministic order. Directly after `(` at end of input: `["expression", ")"]`; after `,`: `["expression"]`.

## R5 Blank coercion matrix
| Context | Blank reads as |
|---|---|
| arithmetic, numeric compare, `%`, unary `-`, ROUND, ABS | 0 |
| `&`, CONCAT, text compare | "" |
| logical (`IF`, `AND`, `OR`, `NOT`), bool compare | FALSE |
| final result `to_text()` / `Display` | "0" |
| inside ranges | skipped (unchanged) |

Blank vs Number compares as 0, vs Text as "", vs Bool as FALSE, vs Blank equal; an Error operand still dominates. This needs two text conversions: the public one ("0") and an internal one for `&`/CONCAT ("").

## R6 Arity location
- `functions::arity` currently returns `(0, MAX)` for variadics and is enforced only at runtime. Decision: change to `(1, MAX)`, call it from the parser when building a `Call`, and keep the runtime guard (returns `#VALUE!`, cannot panic). Tests relying on `MAX()` returning 0 (module test `empty_variadics`, integration tests) are updated per FR-016.

## R7 AVERAGE/MAX/MIN over ranges with no numbers (FR-015)
- Already implemented in `numeric()` (`AVERAGE` gives `#DIV/0!`, `MIN`/`MAX` give 0). Work is regression tests only.

## R8 Dependencies
- `thiserror 2` is already present; nothing added, satisfying "no new dependencies".
