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
