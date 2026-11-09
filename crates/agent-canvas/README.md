# agent-canvas

[![crates.io](https://img.shields.io/crates/v/agent-canvas.svg)](https://crates.io/crates/agent-canvas)
[![Documentation](https://docs.rs/agent-canvas/badge.svg)](https://docs.rs/agent-canvas)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Pure-Rust visual computing & calculation artifacts for AI Agents and LLM workflows.
Zero external runtime dependencies (no Node.js, no Chromium, no Python).

## Core Capabilities

1. **Flowcharts & Diagrams (`graph`)**: Decision trees, sequence lifelines, and system architecture blocks with Manhattan 90° routing.
2. **Executive Business Charts (`chart`)**: Donut/Pie, 2x2 Quadrant Matrix (BCG/Eisenhower), Sales Funnel, and Stacked/Grouped Bars.
3. **Math & Formulae (`math`)**: LaTeX math formula typesetting to SVG path curves + Pure-Rust Excel formula evaluator (`=SUM`, `=IF`, etc.).
4. **Business Components (`component`)**: Pure-Rust QR Code matrix generator + KPI executive metric cards.

## Excel formula evaluator

`ExcelEvaluator` evaluates Excel-style formulas deterministically, with Excel's operator quirks
(`=-3^2` is `9`, `=2^3^2` is `64`, `="ABC"="abc"` is `TRUE`). It never panics and `eval` never
returns `Err`: every failure is a `Value::Error` (`#DIV/0!`, `#VALUE!`, `#NAME?`, `#NUM!`).
Use `try_eval` to get a located `ParseError` (kind, byte span, expected items) for malformed input.

`Value::as_str()` returns the text of a `Text` value only (`Option<&str>`); use `to_text()` or
`Display` for the display string of any value. A reference to an empty cell evaluates to
`Value::Blank`, which reads as `0`, `""` or `FALSE` depending on context.

```rust
use agent_canvas::math::{Cell, ExcelEvaluator, RangeProvider, Value};
use agent_canvas::math::excel::ParseErrorKind;

let v = ExcelEvaluator::eval(r#"=IF(SUM(10, 20) > 25, "Profit", "Loss")"#);
assert_eq!(v.as_str(), Some("Profit"));
assert_eq!(ExcelEvaluator::eval("=1+1").to_text(), "2");

// Malformed input is reported with its position.
let e = ExcelEvaluator::try_eval("=SUM(").unwrap_err();
assert_eq!(e.kind, ParseErrorKind::UnexpectedEof);
assert_eq!(e.span, 5..5);

// References such as A1:A3 resolve through a caller-supplied provider.
struct Column;
impl RangeProvider for Column {
    fn resolve(&self, reference: &str) -> Option<Vec<Cell>> {
        (reference == "A1:A3").then(|| {
            vec![Cell::Value(Value::Number(4.0)), Cell::Empty, Cell::Value(Value::Number(6.0))]
        })
    }
}
let total = ExcelEvaluator::eval_with("=SUM(A1:A3)", &Column);
assert_eq!(total, Value::Number(10.0));
```

Supported functions: `SUM AVERAGE MIN MAX COUNT IF AND OR NOT ABS ROUND IFERROR CONCAT`.
Argument counts are checked at parse time (`SUM()` is a `WrongArgCount` error; variadic
functions need at least one argument). Input is limited to 8192 characters and 64 nesting levels.

## Part of the Agent Workspace Monorepo
Maintained by Darma Cahya (`darmady6@gmail.com`).
