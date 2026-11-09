#!/usr/bin/env bash
# Regenerates crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf.
#
# Source : STIXTwoMath-Regular.otf from CTAN `stix2-otf` (2021-04-02), OFL-1.1
# SHA-256: 95bc2729e41faf93b0bcae9e96c4dc4da45855067fd0581e621e30734fe8d90b
# Usage  : scripts/subset-font.sh /path/to/STIXTwoMath-Regular.otf
# Needs  : uvx (fonttools), developer-only; never required to build or test.
set -euo pipefail

EXPECTED_SHA="95bc2729e41faf93b0bcae9e96c4dc4da45855067fd0581e621e30734fe8d90b"
SRC="${1:?usage: $0 <STIXTwoMath-Regular.otf>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf"

ACTUAL_SHA="$(sha256sum "$SRC" | cut -d' ' -f1)"
if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
  echo "error: source SHA-256 mismatch ($ACTUAL_SHA)" >&2
  exit 1
fi

# Code points: ASCII digits/symbols and stretchy delimiters ( ) [ ] { } |,
# math italic Latin (+ U+210E), math italic lowercase Greek, upright uppercase
# Greek, operators and big operators.
UNICODES="0030-0039,0028,0029,005B,005D,007B,007D,007C,002B,002C,003C,003D,003E,2212,00B1,00B7,00D7,2264,2265,2260,221E,221A,2211,222B,220F,210E,1D434-1D467,1D6FC-1D714,0391-0393,0394,0395-03A9"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

uvx --from fonttools pyftsubset "$SRC" \
  --unicodes="$UNICODES" \
  --layout-features= \
  --no-hinting \
  --no-glyph-names \
  --notdef-outline \
  --drop-tables+=DSIG,GDEF,GPOS,GSUB,BASE,JSTF,STAT,gasp,kern,vhea,vmtx,VORG \
  --output-file="$TMP/subset.otf"

uvx --from fonttools python - "$TMP/subset.otf" "$OUT" <<'PY'
import sys
from fontTools.ttLib import TTFont

src, out = sys.argv[1], sys.argv[2]
FAMILY = "AgentCanvas Math"
f = TTFont(src, recalcTimestamp=False)
name = f["name"]
keep = {0, 13, 14}  # copyright + licence records keep STIX attribution
for rec in list(name.names):
    if rec.nameID in keep:
        continue
    if rec.nameID in (1, 16):
        rec.string = FAMILY
    elif rec.nameID in (2, 17):
        rec.string = "Regular"
    elif rec.nameID in (3, 4):
        rec.string = FAMILY
    elif rec.nameID == 6:
        rec.string = "AgentCanvasMath-Regular"
    elif "STIX" in rec.toUnicode():
        name.removeNames(nameID=rec.nameID, platformID=rec.platformID,
                         platEncID=rec.platEncID, langID=rec.langID)
# Closure check: every vertical variant and assembly part referenced by a
# retained construction must itself be retained, or stretchy glyphs would
# silently lose pieces at runtime.
order = set(f.getGlyphOrder())
variants = f["MATH"].table.MathVariants
missing = []
for base, cons in zip(variants.VertGlyphCoverage.glyphs, variants.VertGlyphConstruction):
    ids = [v.VariantGlyph for v in (cons.MathGlyphVariantRecord or [])]
    if cons.GlyphAssembly:
        ids += [p.glyph for p in cons.GlyphAssembly.PartRecords]
    missing += [(base, g) for g in ids if g not in order]
if missing:
    sys.exit(f"error: subset dropped MATH glyphs: {missing[:5]}")
print(f"closure ok: {len(variants.VertGlyphCoverage.glyphs)} vertical constructions", file=sys.stderr)
# Constant timestamps (2021-04-02, Mac epoch) for reproducible output.
f["head"].created = 3700166400
f["head"].modified = 3700166400
f.save(out)
PY
sha256sum "$OUT"
