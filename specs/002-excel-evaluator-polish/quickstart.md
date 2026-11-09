# Quickstart: validating the feature

Prereqs: Rust stable; run from the worktree root. No network or new crates needed.

```bash
cargo test -p agent-canvas --test excel_tests --test excel_proptest   # scenario + property tests
cargo test -p agent-canvas --lib math::excel                          # module unit tests
cargo test -p agent-canvas --doc                                      # README and crate doc examples
cargo run -p agent-canvas --example issue_009_demo                    # example uses the new API
cargo clippy -p agent-canvas --all-targets -- -D warnings
cargo fmt --check
```

Scenarios to observe (API in [contracts/public-api.md](contracts/public-api.md)):
1. `try_eval("=SUM(")` gives `UnexpectedEof` at `5..5`, expected `expression` and `)`; `eval("=SUM(")` gives `#VALUE!`.
2. `eval(r#"=IF(SUM(10,20)>25,"Profit","Loss")"#).as_str() == Some("Profit")`; `eval("=1+1").as_str() == None` while `to_text() == "2"`.
3. Provider with empty `A1`: `=A1=0` TRUE, `=A1=""` TRUE, `=A1&"x"` gives `x`, `=A1+1` gives 1, `=A1` is Blank and displays `0`.
4. `try_eval` of `=ROUND(1)`, `=IF(1)`, `=SUM()`, `=NOT(1,2)`, `=IF(TRUE,1,ROUND(1))` each give `WrongArgCount`.
5. All-empty range: `AVERAGE` gives `#DIV/0!`; `MAX` and `MIN` give 0.

Expected: everything passes, and `grep -rn "as_str()" crates/agent-canvas/README.md crates/agent-canvas/examples crates/agent-canvas/tests` shows no use of the old coercing meaning.
