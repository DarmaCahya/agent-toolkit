# Data Model: Spacing

```text
Token (existing Command variant):
  Command(",") | Command(":") | Command(";") | Command("!") | Command(" ")
  Command("quad") | Command("qquad")

Glue = Mu(f64)   signed math units; 18 mu = 1 quad of the style in effect
     | Em(f64)   signed font-relative em; ignores style

NodeKind    += Space(Glue)      (parser.rs)
AtomContent += Space(Glue)      (atoms.rs)
```

| Command | Glue |
|---|---|
| `\,` | Mu(3.0) |
| `\:` | Mu(4.0) |
| `\;` | Mu(5.0) |
| `\!` | Mu(-3.0) |
| `\quad` | Em(1.0) |
| `\qquad` | Em(2.0) |
| backslash-space | Em(1/3) |

## Rules
- `Atom::is_glue()` is true for `AtomContent::Space`. Its `class` is a placeholder and is never read.
- `demote_bins`: glue is skipped; `prev` and its index refer to the last non-glue atom (FR-005).
- `layout_list`: glue width is `Mu(m) -> m * style.mu_scale(c)` or `Em(e) -> e`, applied with `LBox::advance`; `prev` is unchanged (FR-002..004).
- `parse_item`: no script is consumed after a `Space` nucleus (FR-006).
- Root: `width = width.max(0.0)`; glue present with no drawable items -> `EmptyFormula`.
- No state transitions; values are constants, so output stays deterministic.
