# Data Model

## ParseError (new)
| Field | Type | Notes |
|---|---|---|
| kind | `ParseErrorKind` | |
| span | `Range<usize>` | half-open byte offsets into the original input; empty at end of input |
| expected | `Vec<&'static str>` | deterministic order; empty when not applicable |

Traits: `Debug, Clone, PartialEq, Eq, thiserror::Error`; `Send + Sync`; `#[non_exhaustive]`.
Display: kind, position and expected items, e.g. `unexpected end of input at 5, expected expression or )`.

## ParseErrorKind (new, `#[non_exhaustive]`)
- `UnexpectedEof`
- `UnexpectedToken`
- `WrongArgCount { name: String, min: usize, max: Option<usize>, got: usize }`
- `LimitExceeded` (input over 8192 chars, or nesting over 64)

## Value (changed, now `#[non_exhaustive]`)
`Number(f64) | Bool(bool) | Text(String) | Error(ExcelError) | Blank`
- `as_str(&self) -> Option<&str>`: `Some` only for `Text`.
- `to_text(&self) -> Cow<str>`: the old `as_str` output; `Blank` gives `"0"`.
- `impl Display`: writes `to_text()`.

## Cell (unchanged)
`Empty | Value(Value)`. `Empty` and `Value(Blank)` are skipped inside ranges and read as `Blank` as a scalar.

## Arity table (internal)
| Function | min | max |
|---|---|---|
| SUM AVERAGE MIN MAX COUNT AND OR CONCAT | 1 | none |
| IF | 2 | 3 |
| ROUND IFERROR | 2 | 2 |
| NOT ABS | 1 | 1 |

No state transitions: evaluation is a pure function of input and provider.
