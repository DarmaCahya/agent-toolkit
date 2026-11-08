use agent_canvas::math::{Cell, ExcelEvaluator, RangeProvider, Value};

/// A one-column sheet: A1:A3 holds a number, an empty cell, and text.
struct Sheet;

impl RangeProvider for Sheet {
    fn resolve(&self, reference: &str) -> Option<Vec<Cell>> {
        match reference {
            "A1:A3" => Some(vec![
                Cell::Value(Value::Number(4.0)),
                Cell::Empty,
                Cell::Value(Value::Text("3".into())),
            ]),
            _ => None,
        }
    }
}

fn main() {
    println!("=== AGENT-CANVAS: ISSUE #9 EXCEL FORMULA EVALUATOR ===");

    let formulas = [
        r#"=IF(SUM(10, 20) > 25, "Profit", "Loss")"#,
        "=-3^2",
        "=2^3^2",
        "=200*10%",
        r#"="ABC"="abc""#,
        "=0.1+0.2=0.3",
        r#"="a"&"b"&1"#,
        "=ROUND(2.675,2)",
        r#"=IFERROR(1/0,"n/a")"#,
        "=IF(TRUE,1,1/0)",
        r#"=SUM("3",TRUE,2)"#,
        "=1/0",
        r#"="abc"+1"#,
        "=NOSUCH(1)",
        "=10^1000",
        "=1+",
    ];
    for f in formulas {
        println!("{f:<45} -> {}", ExcelEvaluator::eval(f).as_str());
    }

    println!("--- with a range provider ---");
    for f in ["=SUM(A1:A3)", "=COUNT(A1:A3)", "=SUM(\"3\",A1:A3)", "=B1"] {
        println!(
            "{f:<45} -> {}",
            ExcelEvaluator::eval_with(f, &Sheet).as_str()
        );
    }
}
