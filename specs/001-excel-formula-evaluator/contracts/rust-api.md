# Contract: Public Rust API

Exported from `agent_canvas::math` (via `math/mod.rs`: `pub mod excel;` and `pub use excel::{Cell, ExcelError, ExcelEvaluator, RangeProvider, Value};`).

```rust
pub struct ExcelEvaluator;

impl ExcelEvaluator {
    /// Never panics, never returns Err. Leading '=' optional.
    pub fn eval(formula: &str) -> Value;
    /// Same, with cell/range references resolved through `provider`.
    pub fn eval_with(formula: &str, provider: &dyn RangeProvider) -> Value;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value { Number(f64), Bool(bool), Text(String), Error(ExcelError) }

impl Value {
    /// Canonical display string.
    pub fn as_str(&self) -> std::borrow::Cow<'_, str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ExcelError { DivZero, Value, Name, Num }
impl ExcelError { pub fn code(&self) -> &'static str; }

pub trait RangeProvider {
    /// `reference` is the uppercased text, e.g. "A1" or "A1:B3".
    /// None => the reference evaluates to #NAME?.
    fn resolve(&self, reference: &str) -> Option<Vec<Cell>>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell { Empty, Value(Value) }
```

Open API-shape note: the issue's example calls `ExcelEvaluator::eval("=...")` as an associated function, which this contract preserves; `as_str()` returns `Cow` so text values borrow instead of cloning.

## Behavioral contract (summary; full tables in spec.md)

- Display: whole number → no `.0`; `-0` → `0`; Bool → `TRUE`/`FALSE`; Error → `#DIV/0!` / `#VALUE!` / `#NAME?` / `#NUM!`; other numbers per research D5.
- Precedence high→low: prefix `+ -`, postfix `%`, `^` (left), `* /`, `+ -`, `&`, comparisons.
- Functions: `SUM AVERAGE MIN MAX COUNT IF AND OR NOT ABS ROUND IFERROR CONCAT` (case-insensitive); all others `#NAME?`.
- Limits: >8192 chars or depth >64 → `Error(Value)`; 8192 and 64 accepted.
- Guarantees: total function (always returns), deterministic, no I/O.
- Semver: additive new API in `agent-canvas` → minor bump (0.1.x → 0.2.0 or per pre-1.0 policy; decide at release).

## Grammar (informal EBNF)

```
formula    = [ "=" ] expr
expr       = cmp
cmp        = concat { ("="|"<>"|"<"|">"|"<="|">=") concat }
concat     = add { "&" add }
add        = mul { ("+"|"-") mul }
mul        = pow { ("*"|"/") pow }
pow        = post { "^" post }
post       = unary { "%" }
unary      = ("+"|"-") unary | primary
primary    = number | string | bool | ref | call | "(" expr ")"
call       = ident "(" [ expr { "," expr } ] ")"
ref        = cell [ ":" cell ]
```
Note: `unary` sits *below* `post`/`pow` in this grammar only in the sense that prefix operators apply to the operand before `%` and `^` are considered at the same operand (hence `-3^2` = `(-3)^2`); the Pratt table in research D1 is authoritative.
