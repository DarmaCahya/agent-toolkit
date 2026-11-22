# Third-party notices

## STIX Two Math (bundled font subset)

`crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf` is a **modified,
renamed subset** of *STIX Two Math* (`STIXTwoMath-Regular.otf`, CTAN `stix2-otf`
release 2021-04-02).

- Copyright 2001-2021 The STIX Fonts Project Authors (<https://github.com/stipub/stixfonts>)
- Licensed under the SIL Open Font License, Version 1.1 (**OFL-1.1**).
  Full text: [`crates/agent-canvas/assets/fonts/OFL-1.1.txt`](crates/agent-canvas/assets/fonts/OFL-1.1.txt)
- Modifications: glyph subsetting, hinting removed, font renamed to
  "AgentCanvas Math" (the Reserved Font Names are not used). Regenerate with
  `scripts/subset-font.sh` (the source SHA-256 is pinned in the script).

## DejaVu Sans (bundled text subset)

`crates/agent-canvas/assets/fonts/AgentCanvasSans-subset.ttf` and
`AgentCanvasSansBold-subset.ttf` are **subset** copies of *DejaVu Sans* and
*DejaVu Sans Bold* (DejaVu Fonts 2.37), used by `component::text` to draw KPI card
labels as vector outlines.

- Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved.
- Bitstream Vera is a trademark of Bitstream, Inc.; DejaVu changes are in the public domain.
- Licensed under the Bitstream Vera licence. Full text:
  [`crates/agent-canvas/assets/fonts/DejaVu-LICENSE.txt`](crates/agent-canvas/assets/fonts/DejaVu-LICENSE.txt)
- Modifications: Unicode subsetting to Latin-1 printable text plus a small symbol set,
  hinting removed, fixed timestamps. Regenerate with `scripts/subset-sans-font.sh`
  (source SHA-256 values are pinned in the script).
