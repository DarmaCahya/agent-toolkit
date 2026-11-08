// Relaxed lints are allowed in tests: failing loudly is the point.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)] // test code

use agent_canvas::math::{Cell, ExcelError, ExcelEvaluator, RangeProvider, Value};

fn d(formula: &str) -> String {
    ExcelEvaluator::eval(formula).as_str().into_owned()
}

fn err(e: ExcelError) -> Value {
    Value::Error(e)
}

/// Deterministic stub provider: linear lookup over a fixed table.
struct Stub(Vec<(&'static str, Vec<Cell>)>);

impl RangeProvider for Stub {
    fn resolve(&self, reference: &str) -> Option<Vec<Cell>> {
        self.0
            .iter()
            .find(|(k, _)| *k == reference)
            .map(|(_, v)| v.clone())
    }
}

fn n(x: f64) -> Cell {
    Cell::Value(Value::Number(x))
}
fn t(s: &str) -> Cell {
    Cell::Value(Value::Text(s.into()))
}
fn b(x: bool) -> Cell {
    Cell::Value(Value::Bool(x))
}

fn stub() -> Stub {
    Stub(vec![
        ("A1", vec![n(5.0)]),
        ("A1:A3", vec![t("3"), b(true), n(2.0)]),
        ("B1:B4", vec![Cell::Empty, n(4.0), Cell::Empty, n(6.0)]),
        ("C1:C2", vec![n(1.0), Cell::Value(err(ExcelError::Num))]),
        ("D1:D2", vec![t("a"), t("b")]),
        ("E1:E3", vec![t("x"), Cell::Empty, t("y")]),
        ("F1:F3", vec![b(true), n(1.0), t("x")]),
        ("G1:G2", vec![b(false), n(0.0)]),
        ("H1:H3", vec![t("a"), Cell::Empty, n(1.0)]),
        ("I1", vec![Cell::Empty]),
        ("J1:J2", vec![n(1.0), Cell::Value(err(ExcelError::DivZero))]),
    ])
}

fn w(formula: &str) -> Value {
    ExcelEvaluator::eval_with(formula, &stub())
}

mod story1 {
    use super::*;

    #[test]
    fn headline_example() {
        let v = ExcelEvaluator::eval("=IF(SUM(10, 20) > 25, \"Profit\", \"Loss\")");
        assert_eq!(v, Value::Text("Profit".into()));
        assert_eq!(v.as_str(), "Profit");
    }

    #[test]
    fn basics_and_optional_equals() {
        assert_eq!(ExcelEvaluator::eval("=1+2*3"), Value::Number(7.0));
        assert_eq!(
            ExcelEvaluator::eval("1+2*3"),
            ExcelEvaluator::eval("=1+2*3")
        );
        assert_eq!(d("=SUM(1,2,3)*2"), "12");
    }

    #[test]
    fn display_strings() {
        assert_eq!(d("=4/2"), "2");
        assert_eq!(d("=1/4"), "0.25");
        assert_eq!(d("=TRUE"), "TRUE");
        assert_eq!(d("=\"x\""), "x");
        assert_eq!(d("=1/0"), "#DIV/0!");
    }

    #[test]
    fn deterministic() {
        let f = "=IF(SUM(10, 20) > 25, \"Profit\", \"Loss\")&1/3";
        assert_eq!(ExcelEvaluator::eval(f), ExcelEvaluator::eval(f));
    }
}

mod story2 {
    use super::*;

    #[test]
    fn quirk_table() {
        let cases = [
            ("=-3^2", "9"),
            ("=2^3^2", "64"),
            ("=50%", "0.5"),
            ("=200*10%", "20"),
            ("=200%%", "0.02"),
            ("=--5", "5"),
            ("=+5", "5"),
            ("=\"a\"&\"b\"&1", "ab1"),
            ("=1&2+3", "15"),
            ("=1&2=\"12\"", "TRUE"),
            ("=1&2=12", "FALSE"),
            ("=\"ABC\"=\"abc\"", "TRUE"),
            ("=\"a\"<>\"A\"", "FALSE"),
            ("=\"a\"<\"B\"", "TRUE"),
            ("=\"b\">\"A\"", "TRUE"),
            ("=\"a\"<=\"A\"", "TRUE"),
            ("=\"a\">=\"B\"", "FALSE"),
            ("=0.1+0.2=0.3", "TRUE"),
            ("=1=1.000000000000002", "FALSE"),
            ("=1<\"a\"", "TRUE"),
            ("=\"a\"<TRUE", "TRUE"),
            ("=1<TRUE", "TRUE"),
            ("=\"a\"=1", "FALSE"),
            ("=(1+2)*3", "9"),
            ("=5% 3", "#VALUE!"),
            ("=2*-3^2", "18"),
            ("=+\"a\"", "a"),
        ];
        for (f, want) in cases {
            assert_eq!(d(f), want, "{f}");
        }
        // Fractional powers go through libm; only check the leading digits.
        assert!(d("=2^3%").starts_with("1.0210"), "{}", d("=2^3%"));
    }
}

mod story3 {
    use super::*;

    #[test]
    fn error_codes() {
        let cases = [
            ("=1/0", ExcelError::DivZero),
            ("=0/0", ExcelError::DivZero),
            ("=\"abc\"+1", ExcelError::Value),
            ("=NOSUCH(1)", ExcelError::Name),
            ("=10^1000", ExcelError::Num),
            ("=0^0", ExcelError::Num),
            ("=(-8)^0.5", ExcelError::Num),
            ("=0^-1", ExcelError::DivZero),
            ("=1+(1/0)", ExcelError::DivZero),
            ("=(1/0)=1", ExcelError::DivZero),
            ("=NOSUCH(1/0)", ExcelError::Name),
            ("=NOSUCH(1+)", ExcelError::Value),
            ("=1+", ExcelError::Value),
            ("=(1", ExcelError::Value),
            ("=SUM(", ExcelError::Value),
            ("", ExcelError::Value),
            ("=", ExcelError::Value),
            ("   ", ExcelError::Value),
            ("=NOW()", ExcelError::Name),
            ("=RAND()", ExcelError::Name),
            ("=TODAY()", ExcelError::Name),
            ("=A1", ExcelError::Name),
            ("=FOO", ExcelError::Name),
            ("=10^308*10", ExcelError::Num),
            ("=\"abc", ExcelError::Value),
            ("=1 # 2", ExcelError::Value),
        ];
        for (f, e) in cases {
            assert_eq!(ExcelEvaluator::eval(f), Value::Error(e), "{f}");
        }
    }

    #[test]
    fn batch_never_interrupted() {
        let batch: Vec<&str> = vec![
            "=1+1",
            "=1/0",
            "=SUM(1,2,3)",
            "=NOSUCH(1)",
            "=\"a\"&\"b\"",
            "=(",
            "=IF(TRUE,\"y\",\"n\")",
            "=10^1000",
            "=ABS(-3)",
            "=\"x\"+1",
            "=AVERAGE(2,4)",
            "",
            "=NOT(TRUE)",
            "=ROUND(2.5,0)",
            "=1+",
            "=CONCAT(\"a\",1)",
            "=MAX(1,9,3)",
            "=IFERROR(1/0,\"ok\")",
            "=COUNT(1,\"a\")",
            "=1=1",
        ];
        let want = [
            "2", "#DIV/0!", "6", "#NAME?", "ab", "#VALUE!", "y", "#NUM!", "3", "#VALUE!", "3",
            "#VALUE!", "FALSE", "3", "#VALUE!", "a1", "9", "ok", "1", "TRUE",
        ];
        let got: Vec<String> = batch.iter().map(|f| d(f)).collect();
        assert_eq!(got.len(), 20);
        assert_eq!(got, want);
    }
}

mod story4 {
    use super::*;

    #[test]
    fn functions() {
        let cases = [
            ("=ROUND(2.5,0)", "3"),
            ("=ROUND(-2.5,0)", "-3"),
            ("=ROUND(1234,-2)", "1200"),
            ("=ROUND(2.675,2)", "2.68"),
            ("=ROUND(1.5)", "#VALUE!"),
            ("=ROUND(1,1000)", "1"),
            ("=ROUND(1,-1000)", "0"),
            ("=ROUND(3.14159,2.9)", "3.14"),
            ("=IFERROR(1/0,\"n/a\")", "n/a"),
            ("=IFERROR(5,1/0)", "5"),
            ("=IF(TRUE,1,1/0)", "1"),
            ("=IF(FALSE,1/0,2)", "2"),
            ("=IF(FALSE,1)", "FALSE"),
            ("=IF(1)", "#VALUE!"),
            ("=IF(\"x\",1,2)", "#VALUE!"),
            ("=IF(\"1\",1,2)", "#VALUE!"),
            ("=IF(0,1,2)", "2"),
            ("=AND(TRUE,1,0)", "FALSE"),
            ("=AND(TRUE,2)", "TRUE"),
            ("=OR(FALSE,0)", "FALSE"),
            ("=OR(FALSE,\"1\")", "TRUE"),
            ("=NOT(0)", "TRUE"),
            ("=AND()", "#VALUE!"),
            ("=OR()", "#VALUE!"),
            ("=NOT()", "#VALUE!"),
            ("=CONCAT(\"a\",1,TRUE)", "a1TRUE"),
            ("=CONCAT()", ""),
            ("=SUM()", "0"),
            ("=COUNT()", "0"),
            ("=AVERAGE()", "#DIV/0!"),
            ("=MIN()", "0"),
            ("=MAX()", "0"),
            ("=AVERAGE(1,2,3)", "2"),
            ("=MIN(3,1,2)", "1"),
            ("=MAX(3,1,2)", "3"),
            ("=ABS(-4.5)", "4.5"),
            ("=ABS()", "#VALUE!"),
            ("=ABS(1,2)", "#VALUE!"),
            ("=sum(1,2)", "3"),
            ("=Sum(1,2)", "3"),
            ("=SUM(1/0)", "#DIV/0!"),
            ("=COUNT(\"abc\",1)", "1"),
            ("=IFERROR(NOSUCH(1),7)", "7"),
        ];
        for (f, want) in cases {
            assert_eq!(d(f), want, "{f}");
        }
    }
}

mod story5 {
    use super::*;

    #[test]
    fn direct_vs_range_coercion() {
        assert_eq!(d("=SUM(\"3\",TRUE,2)"), "6");
        assert_eq!(d("=SUM(\"abc\")"), "#VALUE!");
        assert_eq!(w("=SUM(A1:A3)"), Value::Number(2.0));
        assert_eq!(d("=COUNT(\"3\",TRUE,5)"), "3");
        assert_eq!(w("=COUNT(A1:A3)"), Value::Number(1.0));
    }

    #[test]
    fn empty_cells_are_skipped() {
        assert_eq!(w("=SUM(B1:B4)"), Value::Number(10.0));
        assert_eq!(w("=COUNT(B1:B4)"), Value::Number(2.0));
        assert_eq!(w("=AVERAGE(B1:B4)"), Value::Number(5.0));
        assert_eq!(w("=MIN(B1:B4)"), Value::Number(4.0));
        assert_eq!(w("=MAX(B1:B4)"), Value::Number(6.0));
    }

    #[test]
    fn range_errors_propagate() {
        for f in ["SUM", "AVERAGE", "MIN", "MAX", "COUNT"] {
            assert_eq!(w(&format!("={f}(C1:C2)")), err(ExcelError::Num), "{f}");
        }
        assert_eq!(w("=SUM(J1:J2)"), err(ExcelError::DivZero));
    }

    #[test]
    fn average_over_text_only() {
        assert_eq!(w("=AVERAGE(D1:D2)"), err(ExcelError::DivZero));
    }

    #[test]
    fn logical_over_ranges() {
        assert_eq!(w("=AND(E1:E3)"), err(ExcelError::Value));
        assert_eq!(w("=OR(E1:E3)"), err(ExcelError::Value));
        assert_eq!(w("=AND(F1:F3)"), Value::Bool(true));
        assert_eq!(w("=OR(G1:G2)"), Value::Bool(false));
        assert_eq!(w("=AND(G1:G2)"), Value::Bool(false));
    }

    #[test]
    fn concat_flattens_ranges() {
        assert_eq!(w("=CONCAT(H1:H3)"), Value::Text("a1".into()));
        assert_eq!(w("=CONCAT(\"<\",H1:H3,\">\")"), Value::Text("<a1>".into()));
        assert_eq!(w("=CONCAT(C1:C2)"), err(ExcelError::Num));
    }

    #[test]
    fn references_and_provider_rules() {
        assert_eq!(w("=Z9"), err(ExcelError::Name));
        assert_eq!(w("=SUM(Z9:Z10)"), err(ExcelError::Name));
        assert_eq!(ExcelEvaluator::eval("=SUM(A1:A3)"), err(ExcelError::Name));
        assert_eq!(w("=$A$1"), err(ExcelError::Name));
        assert_eq!(w("=Sheet1!A1"), err(ExcelError::Name));
        assert_eq!(w("=SUM($A$1:$A$3)"), err(ExcelError::Name));
        assert_eq!(w("=A1+1"), Value::Number(6.0));
        assert_eq!(w("=a1+1"), Value::Number(6.0));
        assert_eq!(w("=a1:a3+1"), err(ExcelError::Value));
        assert_eq!(w("=I1+1"), Value::Number(1.0));
        assert_eq!(w("=SUM(I1)"), Value::Number(0.0));
        assert_eq!(w("=SUM(a1:a3)"), Value::Number(2.0));
    }

    #[test]
    fn hostile_provider_cell_budget() {
        struct Big;
        impl RangeProvider for Big {
            fn resolve(&self, _: &str) -> Option<Vec<Cell>> {
                Some(vec![Cell::Empty; 600_000])
            }
        }
        let v = ExcelEvaluator::eval_with("=SUM(A1:A2,A1:A2)", &Big);
        assert_eq!(v, err(ExcelError::Value));
        let ok = ExcelEvaluator::eval_with("=SUM(A1:A2)", &Big);
        assert_eq!(ok, Value::Number(0.0));
    }
}

/// (formula, expected display). Expectations come from Excel semantics and the spec,
/// not from running the implementation. Evaluated with the stub provider.
const CORPUS: &[(&str, &str)] = &[
    // --- arithmetic and display ---
    ("=1+1", "2"),
    ("=5-8", "-3"),
    ("=3*4", "12"),
    ("=9/4", "2.25"),
    ("=2^10", "1024"),
    ("=2^-1", "0.5"),
    ("=10-2-3", "5"),
    ("=100/10/5", "2"),
    ("=2*3+4", "10"),
    ("=2+3*4", "14"),
    ("=(2+3)*4", "20"),
    ("=-2^2", "4"),
    ("=-(2^2)", "-4"),
    ("=--3", "3"),
    ("=+-3", "-3"),
    ("=7/2", "3.5"),
    ("=1/8", "0.125"),
    ("=0.5+0.25", "0.75"),
    ("=1.5*2", "3"),
    ("=10%", "0.1"),
    ("=50%*2", "1"),
    ("=200%", "2"),
    ("=200%%", "0.02"),
    ("=1%+1%", "0.02"),
    ("=50%+50%", "1"),
    ("=1E3", "1000"),
    ("=1e-2", "0.01"),
    ("=.5+.5", "1"),
    ("=2^0.5*2^0.5=2", "TRUE"),
    ("=4^0.5", "2"),
    ("=(-8)^2", "64"),
    ("=0^2", "0"),
    ("=2^3^2", "64"),
    ("=-3^2", "9"),
    ("=1000000*1000000", "1000000000000"),
    ("=0.1*3", "0.30000000000000004"),
    ("=1-0.9", "0.09999999999999998"),
    ("=3-3", "0"),
    ("=0*-1", "0"),
    ("=10^15", "1000000000000000"),
    ("=10^20", "100000000000000000000"),
    ("=10^21", "1E+21"),
    ("=1/10^8", "1E-08"),
    ("=10^-7", "0.0000001"),
    ("=1.5E-10*1", "1.5E-10"),
    ("=123456789*1000", "123456789000"),
    ("=2^53", "9007199254740992"),
    ("=2^70", "1.1805916207174113E+21"),
    // --- comparisons ---
    ("=1<2", "TRUE"),
    ("=2<1", "FALSE"),
    ("=1<=1", "TRUE"),
    ("=1>=2", "FALSE"),
    ("=1<>1", "FALSE"),
    ("=1<>2", "TRUE"),
    (r#"="a"="a""#, "TRUE"),
    (r#"="a"<>"b""#, "TRUE"),
    (r#"="apple"<"banana""#, "TRUE"),
    (r#"="Z">"a""#, "TRUE"),
    (r#"="abc"<"abd""#, "TRUE"),
    (r#"=""="""#, "TRUE"),
    (r#"=""<"a""#, "TRUE"),
    ("=TRUE=TRUE", "TRUE"),
    ("=TRUE>FALSE", "TRUE"),
    ("=FALSE<TRUE", "TRUE"),
    ("=TRUE=1", "FALSE"),
    (r#"=1<"1""#, "TRUE"),
    (r#"="1"=1"#, "FALSE"),
    (r#"="a">1"#, "TRUE"),
    (r#"=TRUE>"zzz""#, "TRUE"),
    ("=0.1+0.2=0.3", "TRUE"),
    ("=0.1+0.2<>0.3", "FALSE"),
    ("=0.1+0.2>0.3", "FALSE"),
    ("=0.1+0.2<=0.3", "TRUE"),
    ("=1/3*3=1", "TRUE"),
    ("=10^15+0.1=10^15", "TRUE"),
    ("=1=1.000000000000002", "FALSE"),
    ("=-0=0", "TRUE"),
    (r#"="Straße"="STRASSE""#, "FALSE"),
    (r#"="É"="é""#, "TRUE"),
    ("=(1<2)=TRUE", "TRUE"),
    ("=(1<2)+1", "2"),
    ("=TRUE+TRUE", "2"),
    ("=TRUE*5", "5"),
    ("=-TRUE", "-1"),
    // --- concatenation ---
    (r#"="a"&"b""#, "ab"),
    ("=1&1", "11"),
    (r#"=1.5&"x""#, "1.5x"),
    ("=TRUE&FALSE", "TRUEFALSE"),
    (r#"=""&"""#, ""),
    (r#"="a"&1+2"#, "a3"),
    ("=1+2&3+4", "37"),
    (r#"="x"&(1/4)"#, "x0.25"),
    (r#"=1/0&"a""#, "#DIV/0!"),
    (r#"="a"&NOSUCH(1)"#, "#NAME?"),
    (r#"="say ""hi""""#, r#"say "hi""#),
    (r#"="a b"&" c""#, "a b c"),
    (r#"=-1&"""#, "-1"),
    (r#"=1E21&"""#, "1E+21"),
    (r#"=0.1+0.2&"""#, "0.30000000000000004"),
    // --- errors and coercion ---
    ("=1/0", "#DIV/0!"),
    ("=0/0", "#DIV/0!"),
    (r#"="a"*2"#, "#VALUE!"),
    (r#"="3"*2"#, "6"),
    (r#"=" 3 "+1"#, "4"),
    (r#"="1e2"+0"#, "100"),
    (r#"=""+1"#, "#VALUE!"),
    (r#"="inf"+1"#, "#VALUE!"),
    (r#"="0x10"+1"#, "#VALUE!"),
    (r#"="1,000"+1"#, "#VALUE!"),
    (r#"=TRUE+"a""#, "#VALUE!"),
    (r#"=-"a""#, "#VALUE!"),
    ("=10^400", "#NUM!"),
    ("=(-1)^0.5", "#NUM!"),
    ("=0^0", "#NUM!"),
    ("=0^-2", "#DIV/0!"),
    ("=1E308*10", "#NUM!"),
    ("=-1E308*10", "#NUM!"),
    ("=1E308+1E308", "#NUM!"),
    ("=1/(1-1)", "#DIV/0!"),
    ("=1/(1/0)", "#DIV/0!"),
    ("=NOSUCH()", "#NAME?"),
    ("=NOW()", "#NAME?"),
    ("=RAND()", "#NAME?"),
    ("=TODAY()", "#NAME?"),
    ("=VLOOKUP(1,2,3)", "#NAME?"),
    ("=FOO", "#NAME?"),
    ("=1+", "#VALUE!"),
    ("=(1+2", "#VALUE!"),
    ("=1+2)", "#VALUE!"),
    ("=1 2", "#VALUE!"),
    ("=*2", "#VALUE!"),
    ("=1**2", "#VALUE!"),
    ("=SUM(1,)", "#VALUE!"),
    ("=SUM(,1)", "#VALUE!"),
    ("=SUM(1 2)", "#VALUE!"),
    (r#"="abc"#, "#VALUE!"),
    ("=1.2.3", "#VALUE!"),
    ("={1}", "#VALUE!"),
    ("=@A1", "#VALUE!"),
    ("=1;2", "#VALUE!"),
    ("=SUM(1;2)", "#VALUE!"),
    ("=#REF!", "#VALUE!"),
    ("=%", "#VALUE!"),
    ("=5% 3", "#VALUE!"),
    ("==1", "#VALUE!"),
    ("=", "#VALUE!"),
    ("", "#VALUE!"),
    // --- SUM / AVERAGE / MIN / MAX / COUNT ---
    ("=SUM(1,2,3)", "6"),
    ("=SUM(1)", "1"),
    ("=SUM(-1,1)", "0"),
    ("=SUM(0.1,0.2)", "0.30000000000000004"),
    (r#"=SUM("2",3)"#, "5"),
    ("=SUM(TRUE,TRUE)", "2"),
    (r#"=SUM(1,"x")"#, "#VALUE!"),
    ("=SUM(1,2/0)", "#DIV/0!"),
    ("=SUM(SUM(1,2),SUM(3,4))", "10"),
    ("=SUM(1E308,1E308)", "#NUM!"),
    ("=SUM()", "0"),
    ("=AVERAGE(1,2,3,4)", "2.5"),
    ("=AVERAGE(10)", "10"),
    (r#"=AVERAGE("4",TRUE)"#, "2.5"),
    (r#"=AVERAGE(1,"x")"#, "#VALUE!"),
    ("=AVERAGE(0,0)", "0"),
    ("=AVERAGE()", "#DIV/0!"),
    ("=MIN(5,2,8)", "2"),
    ("=MAX(5,2,8)", "8"),
    ("=MIN(-1,-5)", "-5"),
    ("=MAX(-1,-5)", "-1"),
    (r#"=MIN("3",TRUE)"#, "1"),
    (r#"=MAX(1,"a")"#, "#VALUE!"),
    ("=MIN(1/0,2)", "#DIV/0!"),
    ("=MIN()", "0"),
    ("=MAX()", "0"),
    ("=COUNT(1,2,3)", "3"),
    (r#"=COUNT(1,"a",TRUE)"#, "2"),
    (r#"=COUNT("1","x")"#, "1"),
    ("=COUNT(1/0)", "#DIV/0!"),
    ("=COUNT()", "0"),
    // --- IF ---
    (r#"=IF(TRUE,"y","n")"#, "y"),
    (r#"=IF(FALSE,"y","n")"#, "n"),
    ("=IF(1,1,2)", "1"),
    ("=IF(0,1,2)", "2"),
    ("=IF(-1,1,2)", "1"),
    (r#"=IF(1>2,"a","b")"#, "b"),
    ("=IF(TRUE,1)", "1"),
    ("=IF(FALSE,1)", "FALSE"),
    (r#"=IF("",1,2)"#, "#VALUE!"),
    ("=IF(1/0,1,2)", "#DIV/0!"),
    ("=IF(TRUE,1/0,2)", "#DIV/0!"),
    ("=IF(FALSE,NOSUCH(1),5)", "5"),
    ("=IF(TRUE,IF(FALSE,1,2),3)", "2"),
    ("=IF(IF(TRUE,FALSE,TRUE),1,2)", "2"),
    ("=IF(1,2,3,4)", "#VALUE!"),
    ("=IF()", "#VALUE!"),
    (r#"=IF(SUM(10, 20) > 25, "Profit", "Loss")"#, "Profit"),
    (r#"=IF(SUM(1, 2) > 25, "Profit", "Loss")"#, "Loss"),
    // --- AND / OR / NOT ---
    ("=AND(TRUE,TRUE)", "TRUE"),
    ("=AND(TRUE,FALSE)", "FALSE"),
    ("=AND(1,2,3)", "TRUE"),
    ("=AND(1,0)", "FALSE"),
    (r#"=AND("1",TRUE)"#, "TRUE"),
    (r#"=AND("x")"#, "#VALUE!"),
    ("=AND(TRUE,1/0)", "#DIV/0!"),
    ("=AND(FALSE,1/0)", "#DIV/0!"),
    ("=AND(1>0,2>1)", "TRUE"),
    ("=AND()", "#VALUE!"),
    ("=OR(FALSE,FALSE)", "FALSE"),
    ("=OR(FALSE,TRUE)", "TRUE"),
    ("=OR(0,0,1)", "TRUE"),
    (r#"=OR("0")"#, "FALSE"),
    ("=OR(1>2,2>3)", "FALSE"),
    ("=OR()", "#VALUE!"),
    ("=NOT(TRUE)", "FALSE"),
    ("=NOT(FALSE)", "TRUE"),
    ("=NOT(5)", "FALSE"),
    (r#"=NOT("0")"#, "TRUE"),
    (r#"=NOT("x")"#, "#VALUE!"),
    ("=NOT(1/0)", "#DIV/0!"),
    ("=NOT(TRUE,FALSE)", "#VALUE!"),
    ("=NOT()", "#VALUE!"),
    // --- ABS ---
    ("=ABS(-5)", "5"),
    ("=ABS(5)", "5"),
    ("=ABS(0)", "0"),
    ("=ABS(-0.5)", "0.5"),
    (r#"=ABS("-3")"#, "3"),
    ("=ABS(TRUE)", "1"),
    (r#"=ABS("x")"#, "#VALUE!"),
    ("=ABS(-1/0)", "#DIV/0!"),
    ("=ABS(1,2)", "#VALUE!"),
    // --- ROUND ---
    ("=ROUND(3.14159,2)", "3.14"),
    ("=ROUND(3.5,0)", "4"),
    ("=ROUND(-3.5,0)", "-4"),
    ("=ROUND(0.5,0)", "1"),
    ("=ROUND(-0.5,0)", "-1"),
    ("=ROUND(1.4,0)", "1"),
    ("=ROUND(1250,-2)", "1300"),
    ("=ROUND(-1250,-2)", "-1300"),
    ("=ROUND(1249,-2)", "1200"),
    ("=ROUND(5,-1)", "10"),
    ("=ROUND(4,-1)", "0"),
    ("=ROUND(123.456,1)", "123.5"),
    ("=ROUND(123.456,0)", "123"),
    ("=ROUND(0.1+0.2,1)", "0.3"),
    ("=ROUND(2.675,2)", "2.68"),
    ("=ROUND(1.005,2)", "1.01"),
    (r#"=ROUND("2.5","0")"#, "3"),
    ("=ROUND(8.5,0)", "9"),
    ("=ROUND(1e-20,2)", "0"),
    ("=ROUND(1/0,2)", "#DIV/0!"),
    ("=ROUND(1,2,3)", "#VALUE!"),
    ("=ROUND(1)", "#VALUE!"),
    // --- IFERROR ---
    ("=IFERROR(1,2)", "1"),
    ("=IFERROR(1/0,0)", "0"),
    (r#"=IFERROR("a"+1,"bad")"#, "bad"),
    (r#"=IFERROR(NOSUCH(1),"nf")"#, "nf"),
    ("=IFERROR(10^1000,-1)", "-1"),
    ("=IFERROR(IFERROR(1/0,2/0),3)", "3"),
    ("=IFERROR(1/0,1/0)", "#DIV/0!"),
    ("=IFERROR(1)", "#VALUE!"),
    (r#"=IFERROR("x",1/0)"#, "x"),
    // --- CONCAT ---
    (r#"=CONCAT("a","b")"#, "ab"),
    ("=CONCAT(1,2,3)", "123"),
    (r#"=CONCAT("x",TRUE,1.5)"#, "xTRUE1.5"),
    ("=CONCAT(1/0)", "#DIV/0!"),
    (r#"=CONCAT("a",NOSUCH(1))"#, "#NAME?"),
    (r#"=CONCAT("")"#, ""),
    ("=CONCAT()", ""),
    // --- names are case-insensitive; nesting ---
    ("=sum(1,1)", "2"),
    ("=If(true,1,2)", "1"),
    ("=iferror(1/0,0)", "0"),
    (r#"=cOnCaT("a","b")"#, "ab"),
    ("=aBs(-1)", "1"),
    ("=SUM(ABS(-1),ROUND(1.5,0))", "3"),
    (r#"=IF(AND(1<2,2<3),"ok","no")"#, "ok"),
    ("=MAX(MIN(5,3),2)", "3"),
    ("=ROUND(AVERAGE(1,2,4),2)", "2.33"),
    (r#"=CONCAT(IF(TRUE,"a","b"),SUM(1,2))"#, "a3"),
    // --- whitespace, literals, sanitizing ---
    ("1+1", "2"),
    ("  =  1 + 2  ", "3"),
    ("=\n1\n+\n2", "3"),
    ("=\t1", "1"),
    ("= 1", "1"),
    ("=SUM( 1 , 2 )", "3"),
    (r#"=" a ""#, " a "),
    ("=TRUE", "TRUE"),
    ("=false", "FALSE"),
    ("=True", "TRUE"),
    (r#"="hello""#, "hello"),
    (r#"="""#, ""),
    (r#"="a""b""#, r#"a"b"#),
    (r#"="é""#, "é"),
    ("=1\u{200B}+1", "2"),
    ("=\u{FEFF}1+\u{E0041}1", "2"),
    // --- ranges through the stub provider ---
    ("=SUM(A1:A3)", "2"),
    ("=COUNT(A1:A3)", "1"),
    ("=SUM(B1:B4)", "10"),
    ("=AVERAGE(B1:B4)", "5"),
    ("=MIN(B1:B4)", "4"),
    ("=MAX(B1:B4)", "6"),
    ("=COUNT(B1:B4)", "2"),
    ("=SUM(C1:C2)", "#NUM!"),
    ("=AVERAGE(D1:D2)", "#DIV/0!"),
    ("=AND(F1:F3)", "TRUE"),
    ("=OR(G1:G2)", "FALSE"),
    ("=AND(E1:E3)", "#VALUE!"),
    ("=CONCAT(H1:H3)", "a1"),
    ("=A1+1", "6"),
    ("=a1*2", "10"),
    ("=I1+1", "1"),
    ("=A1:A3+1", "#VALUE!"),
    ("=Z9", "#NAME?"),
    ("=SUM(Z9:Z10)", "#NAME?"),
    ("=$A$1", "#NAME?"),
    ("=Sheet1!A1", "#NAME?"),
    ("=SUM(A1,A1)", "10"),
    ("=SUM(A1:A3,1)", "3"),
    (r#"=IF(A1>3,"big","small")"#, "big"),
    ("=IFERROR(SUM(C1:C2),0)", "0"),
    ("=SUM(J1:J2)", "#DIV/0!"),
    (r#"=COUNT(A1:A3,"5",TRUE)"#, "3"),
    (r#"=SUM("3",A1:A3)"#, "5"),
    ("=MAX(A1:A3,10)", "10"),
    ("=AVERAGE(A1:A3)", "2"),
];

mod corpus {
    use super::*;

    fn run_all() -> Vec<String> {
        let p = stub();
        CORPUS
            .iter()
            .map(|(f, _)| ExcelEvaluator::eval_with(f, &p).as_str().into_owned())
            .collect()
    }

    #[test]
    fn corpus_is_large_enough() {
        assert!(CORPUS.len() >= 200, "only {} formulas", CORPUS.len());
    }

    #[test]
    fn two_runs_agree_and_match_expected() {
        let first = run_all();
        let second = run_all();
        assert_eq!(first, second);
        for ((formula, want), got) in CORPUS.iter().zip(&first) {
            assert_eq!(got, want, "{formula}");
        }
    }

    #[test]
    fn references_without_a_provider_are_names() {
        for f in ["=A1", "=SUM(A1:B2)", "=$A$1", "=Sheet1!A1"] {
            assert_eq!(d(f), "#NAME?", "{f}");
        }
    }
}

mod story6 {
    use super::*;

    fn chain_of_len(total: usize) -> String {
        assert!(total >= 2);
        format!("=1{}", "+0".repeat((total - 2) / 2))
    }

    #[test]
    fn length_boundary_counts_chars() {
        let f = chain_of_len(8192);
        assert_eq!(f.chars().count(), 8192);
        assert_eq!(ExcelEvaluator::eval(&f), Value::Number(1.0));
        let too_long = format!("{f} ");
        assert_eq!(too_long.chars().count(), 8193);
        assert_eq!(ExcelEvaluator::eval(&too_long), err(ExcelError::Value));
    }

    #[test]
    fn length_limit_is_chars_not_bytes() {
        let ok = format!("=\"{}\"", "é".repeat(8189));
        assert_eq!(ok.chars().count(), 8192);
        assert!(ok.len() > 8192);
        assert_eq!(ExcelEvaluator::eval(&ok), Value::Text("é".repeat(8189)));
        let bad = format!("=\"{}\"", "é".repeat(8190));
        assert_eq!(ExcelEvaluator::eval(&bad), err(ExcelError::Value));
    }

    #[test]
    fn nesting_boundaries() {
        let parens = |n: usize| format!("={}1{}", "(".repeat(n), ")".repeat(n));
        assert_eq!(ExcelEvaluator::eval(&parens(64)), Value::Number(1.0));
        assert_eq!(ExcelEvaluator::eval(&parens(65)), err(ExcelError::Value));
        let calls = |n: usize| format!("={}1{}", "ABS(".repeat(n), ")".repeat(n));
        assert_eq!(ExcelEvaluator::eval(&calls(64)), Value::Number(1.0));
        assert_eq!(ExcelEvaluator::eval(&calls(65)), err(ExcelError::Value));
        let prefix = |n: usize| format!("={}1", "-".repeat(n));
        assert_eq!(ExcelEvaluator::eval(&prefix(64)), Value::Number(1.0));
        assert_eq!(ExcelEvaluator::eval(&prefix(65)), err(ExcelError::Value));
        let mixed = format!("={}1{}", "-(".repeat(32), ")".repeat(32));
        assert_eq!(ExcelEvaluator::eval(&mixed), Value::Number(1.0));
        let mixed_bad = format!("={}1{}", "-(".repeat(33), ")".repeat(33));
        assert_eq!(ExcelEvaluator::eval(&mixed_bad), err(ExcelError::Value));
    }

    #[test]
    fn long_chains_do_not_overflow_the_stack() {
        let sum = format!("={}1", "1+".repeat(3999));
        assert_eq!(ExcelEvaluator::eval(&sum), Value::Number(4000.0));
        let pow = format!("=1{}", "^1".repeat(4000));
        assert_eq!(ExcelEvaluator::eval(&pow), Value::Number(1.0));
        let pct = format!("=1{}", "%".repeat(8000));
        assert_eq!(d(&pct), "0");
        let cat = format!("={}", "1&".repeat(4000) + "1");
        assert_eq!(d(&cat).chars().count(), 4001);
        let cmp = format!("=1{}", "=1".repeat(4000));
        // (1=1)=1 compares TRUE with 1 (different types), so the chain settles on FALSE.
        assert_eq!(ExcelEvaluator::eval(&cmp), Value::Bool(false));
        let nested_pow = format!("={}1{}", "2^(".repeat(60), ")".repeat(60));
        assert!(matches!(
            ExcelEvaluator::eval(&nested_pow),
            Value::Number(_) | Value::Error(ExcelError::Num)
        ));
        let right = format!("={}1{}", "1+(".repeat(64), ")".repeat(64));
        assert_eq!(ExcelEvaluator::eval(&right), Value::Number(65.0));
    }

    #[test]
    fn huge_numbers_and_digits_terminate() {
        assert_eq!(d("=10^308*10"), "#NUM!");
        assert_eq!(d("=ROUND(1,1000000000)"), "1");
        assert_eq!(d("=ROUND(1e300,-1000000000)"), "0");
        assert_eq!(d("=2^1000000000"), "#NUM!");
        assert_eq!(d("=2^-1000000000"), "0");
        assert!(matches!(
            ExcelEvaluator::eval("=1.0000001^1000000000"),
            Value::Number(_)
        ));
        assert_eq!(d("=1E999"), "#VALUE!");
    }

    #[test]
    fn invisible_code_points_are_ignored() {
        assert_eq!(d("=1+\u{E0001}2"), d("=1+2"));
        assert_eq!(d("=SU\u{200B}M(1,\u{FEFF}2)"), "3");
        let tagged = format!("=1+{}2", "\u{E0041}".repeat(100));
        assert_eq!(d(&tagged), "3");
    }

    #[test]
    fn max_size_formula_is_fast() {
        let f = chain_of_len(8192);
        let start = std::time::Instant::now();
        let v = ExcelEvaluator::eval(&f);
        let elapsed = start.elapsed();
        assert_eq!(v, Value::Number(1.0));
        assert!(elapsed.as_millis() < 500, "took {elapsed:?}");
    }
}
