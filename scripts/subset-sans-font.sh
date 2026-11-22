#!/usr/bin/env bash
# Regenerates the bundled text faces used by `agent_canvas::component`:
#   crates/agent-canvas/assets/fonts/AgentCanvasSans-subset.ttf
#   crates/agent-canvas/assets/fonts/AgentCanvasSansBold-subset.ttf
#
# Source : DejaVu Sans / DejaVu Sans Bold 2.37 (Debian fonts-dejavu-core)
# License: Bitstream Vera + public-domain DejaVu changes; see
#          crates/agent-canvas/assets/fonts/DejaVu-LICENSE.txt
# Usage  : scripts/subset-sans-font.sh /path/to/DejaVuSans.ttf /path/to/DejaVuSans-Bold.ttf
# Needs  : fonttools (`pip install fonttools` or uvx), developer-only; never
#          required to build or test the crate.
set -euo pipefail

REG_SHA="b4c632e3cdf9acc7f28758fb5a323c8524d7fc6660d46904d9b6cbe2809c419c"
BOLD_SHA="e1d733afbbfce842a1b43dced0c9885a76a4e67290dc7e690c4e9b76ebfc7d23"
REG_SRC="${1:?usage: $0 <DejaVuSans.ttf> <DejaVuSans-Bold.ttf>}"
BOLD_SRC="${2:?usage: $0 <DejaVuSans.ttf> <DejaVuSans-Bold.ttf>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT/crates/agent-canvas/assets/fonts"

check_sha() {
  local file="$1" expected="$2" actual
  actual="$(sha256sum "$file" | cut -d' ' -f1)"
  if [ "$actual" != "$expected" ]; then
    echo "error: source SHA-256 mismatch for $file ($actual)" >&2
    exit 1
  fi
}
check_sha "$REG_SRC" "$REG_SHA"
check_sha "$BOLD_SRC" "$BOLD_SHA"

# Latin-1 printable text plus the symbols component cards use.
REG_UNICODES="0020-007E,00A0,00B1,2013,2014,2018,2019,201C,201D,2022,2026,20AC,2212,25B2,25BC"
BOLD_UNICODES="0020-007E,00B1,2212,2026,25B2,25BC"

SUBSET=(pyftsubset)
if ! command -v pyftsubset >/dev/null 2>&1; then
  SUBSET=(uvx --from fonttools pyftsubset)
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"${SUBSET[@]}" "$REG_SRC" \
  --unicodes="$REG_UNICODES" \
  --layout-features= \
  --no-hinting \
  --desubroutinize \
  --name-IDs='*' \
  --output-file="$TMP/regular.ttf"

"${SUBSET[@]}" "$BOLD_SRC" \
  --unicodes="$BOLD_UNICODES" \
  --layout-features= \
  --no-hinting \
  --desubroutinize \
  --name-IDs='*' \
  --output-file="$TMP/bold.ttf"

# Constant timestamps (2016-01-01, Mac epoch) make the subset bytes reproducible.
PY=(python3)
if command -v uvx >/dev/null 2>&1; then
  PY=(uvx --from fonttools python)
fi
"${PY[@]}" - "$TMP/regular.ttf" "$OUT_DIR/AgentCanvasSans-subset.ttf" \
  "$TMP/bold.ttf" "$OUT_DIR/AgentCanvasSansBold-subset.ttf" <<'PY'
import sys
from fontTools.ttLib import TTFont

FIXED = 3676320000  # 2016-01-01T00:00:00Z in the Mac 1904 epoch
for src, dst in zip(sys.argv[1::2], sys.argv[2::2]):
    font = TTFont(src, recalcTimestamp=False)
    font["head"].created = FIXED
    font["head"].modified = FIXED
    font.save(dst)
    print(f"wrote {dst}")
PY

sha256sum "$OUT_DIR/AgentCanvasSans-subset.ttf" "$OUT_DIR/AgentCanvasSansBold-subset.ttf"
