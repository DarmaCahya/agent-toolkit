# Public API Contract (`agent_canvas::math::excel`)

```rust
impl ExcelEvaluator {
    pub fn eval(formula: &str) -> Value;                                  // total, signature unchanged
    pub fn eval_with(formula: &str, p: &dyn RangeProvider) -> Value;      // total, signature unchanged
    pub fn try_eval(formula: &str) -> Result<Value, ParseError>;          // NEW
    pub fn try_eval_with(formula: &str, p: &dyn RangeProvider) -> Result<Value, ParseError>; // NEW
}

#[non_exhaustive]
pub struct ParseError { pub kind: ParseErrorKind, pub span: Range<usize>, pub expected: Vec<&'static str> }

#[non_exhaustive]
pub enum ParseErrorKind {
    UnexpectedEof,
    UnexpectedToken,
    WrongArgCount { name: String, min: usize, max: Option<usize>, got: usize },
    LimitExceeded,
}

#[non_exhaustive]
pub enum Value { Number(f64), Bool(bool), Text(String), Error(ExcelError), Blank }

impl Value {
    pub fn as_str(&self) -> Option<&str>;     // BREAKING: was Cow<str>
    pub fn to_text(&self) -> Cow<'_, str>;    // old as_str behaviour
}
impl fmt::Display for Value {}                // equals to_text()
```

## Behavioural guarantees
- `eval*` never panics and maps every `ParseError` to `Value::Error(ExcelError::Value)`.
- `try_eval*` returns `Err` only for syntax, arity and limit problems; runtime errors (`#DIV/0!`, `#NAME?`, ...) are `Ok(Value::Error(..))`.
- `try_eval("=SUM(")` is `Err` with `UnexpectedEof`, span `5..5`, expected `["expression", ")"]`.
- `try_eval("=ROUND(1)")` is `WrongArgCount { name: "ROUND", min: 2, max: Some(2), got: 1 }`.
- Unknown functions parse and evaluate to `#NAME?`.

## Migration
Old `v.as_str()` becomes `v.to_text()`. `assert_eq!(v.as_str(), "Profit")` becomes `assert_eq!(v.as_str(), Some("Profit"))`.
