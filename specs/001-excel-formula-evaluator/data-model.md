# Data Model: Excel Formula Evaluator

## Public types (exported from `agent_canvas::math`)

### `Value`
`enum Value { Number(f64), Bool(bool), Text(String), Error(ExcelError) }`

- Never contains NaN or infinity as `Number` (such results become `Error(Num)`).
- `as_str() -> Cow<'_, str>`-like display (see contract): Number whole → integer text; Bool → `TRUE`/`FALSE`; Text → itself; Error → code.
- Derives `Debug, Clone, PartialEq`. (`PartialEq` is structural, not Excel equality.)

### `ExcelError`
`#[non_exhaustive] enum ExcelError { DivZero, Value, Name, Num }`

| Variant | Code | Produced by |
|---|---|---|
| `DivZero` | `#DIV/0!` | division by zero, `0^-n`, empty `AVERAGE` |
| `Value` | `#VALUE!` | bad coercion, syntax error, limit violation, wrong arity, text `IF` condition |
| `Name` | `#NAME?` | unknown function (incl. volatile), unresolved reference |
| `Num` | `#NUM!` | NaN/infinity/overflow, `0^0`, negative base fractional power |

### `ExcelEvaluator`
Stateless unit struct (or zero-field struct). Methods: `eval(&str) -> Value`, `eval_with(&str, &dyn RangeProvider) -> Value`.

### `RangeProvider` (trait)
`fn resolve(&self, reference: &str) -> Option<Vec<Cell>>`

### `Cell`
`enum Cell { Empty, Value(Value) }` — range element; `Empty` exists only here so the public `Value` stays four-kind.

## Internal types

### `Token`
`Number(f64) | Str(String) | Bool(bool) | Ident(String) | Ref(String) /* includes the `:` of a range like A1:B3 */ | Op(OpKind) | LParen | RParen | Comma | Percent | Eof`, each with a char offset (for debugging only; not exposed).

### `Expr`
```
Number(f64) | Text(String) | Bool(bool)
Ref(String)
Unary { op: Neg|Pos, operand }
Percent(operand)
Binary { op: Add|Sub|Mul|Div|Pow|Concat|Eq|Ne|Lt|Gt|Le|Ge, lhs, rhs }
Call { name: String /* uppercased */, args: Vec<Expr> }
```
Boxed children; depth ≤ 64 constructs guaranteed by the parser (plus spine handling, see research D2).

### Limits (constants in `excel/mod.rs`)
`MAX_INPUT_CHARS = 8192`, `MAX_DEPTH = 64`.

## Validation rules (from spec)
- Input char count > 8192 → `Error(Value)` before tokenizing; exactly 8192 accepted.
- Depth > 64 → `Error(Value)`; exactly 64 accepted.
- Optional leading `=` stripped after sanitize and trim; empty remainder → `Error(Value)`.
- Function names matched case-insensitively against the 13 Phase-1 names; anything else → `Error(Name)`.

## Evaluation state transitions
`&str` → (length check) → sanitized `String` → `Vec<Token>` → `Expr` → `Value`. Any stage failing yields `Value::Error`; there is no other outcome. The evaluator holds only a depth counter and an optional provider reference; no shared or mutable global state.
