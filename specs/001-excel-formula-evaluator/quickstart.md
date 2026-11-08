# Quickstart: Validating the Excel Evaluator

## Prerequisites
- Rust toolchain (workspace edition 2021); run from repo root.
- `proptest` added under `[dev-dependencies]` of `crates/agent-canvas/Cargo.toml` (first implementation task).

## Run the checks
```bash
cargo fmt --check
cargo clippy -p agent-canvas --all-targets -- -D warnings
cargo test -p agent-canvas excel            # unit + scenario tests
cargo test -p agent-canvas --test excel_proptest
PROPTEST_CASES=100000 cargo test -p agent-canvas --test excel_proptest --release   # SC-004 full fuzz run
```

## Scenarios that prove the feature (see spec.md for the full list)

| # | Formula | Expected display | Covers |
|---|---|---|---|
| 1 | `=IF(SUM(10, 20) > 25, "Profit", "Loss")` | `Profit` | SC-003, Story 1 |
| 2 | `=-3^2` / `=2^3^2` | `9` / `64` | Story 2 |
| 3 | `=200*10%` | `20` | postfix % |
| 4 | `="ABC"="abc"` / `=0.1+0.2=0.3` | `TRUE` / `TRUE` | compare quirks |
| 5 | `=1/0` / `="abc"+1` / `=NOSUCH(1)` / `=10^1000` | `#DIV/0!` / `#VALUE!` / `#NAME?` / `#NUM!` | Story 3 |
| 6 | `=IF(TRUE,1,1/0)` / `=IFERROR(1/0,"n/a")` | `1` / `n/a` | lazy eval |
| 7 | `=SUM("3",TRUE,2)` vs `SUM(range of "3",TRUE,2)` via stub provider | `6` vs `2` | Story 5 |
| 8 | 8192-char vs 8193-char formula; depth 64 vs 65 | value vs `#VALUE!` | Story 6 |

## Expected outcomes
- All tests pass; zero panics across the property run.
- Running the 200+ formula corpus twice yields identical display strings.
- A max-size formula evaluates well under 10 ms (release build).

Contracts: [contracts/rust-api.md](contracts/rust-api.md). Types: [data-model.md](data-model.md).
