# Contract: Spacing syntax (`MathTypesetter::render_to_svg`)

No Rust API signature changes. The accepted input grammar grows by:

```text
atom    ::= ... | spacing
spacing ::= "\," | "\:" | "\;" | "\!" | "\" SPACE | "\quad" | "\qquad"
```

Spacing is allowed wherever an atom is: top level, `{}` groups, script, fraction and root
arguments (including the `\sqrt[..]` index). Spacing cannot take scripts itself.

| Input | Result |
|---|---|
| `a\,b` | Ok; width = `ab` + 3/18 em |
| `a\!b` | Ok; width = `ab` - 3/18 em |
| `a\quad b` | Ok; width = `ab` + 1 em (not scaled in scripts) |
| `x^{a\,b}` | Ok; thin space measured at script size |
| `\,`, `\quad`, `{\,}` | `Err(EmptyFormula)` |
| `x\` | `Err(UnexpectedEof)` (unchanged) |
| `\@`, `\%` | `Err(Unsupported)`; hint no longer says spacing is out of scope |
| `\quadx` | `Err(UnknownCommand{name:"quadx"})` |
| `\left`, `\text`, `\hat` | `Err(Unsupported)` (unchanged) |

Output stays pure path SVG; spacing emits no element. Identical input gives byte-identical SVG,
and formulas valid before this change render byte-identically.
