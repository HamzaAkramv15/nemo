#!/usr/bin/env bash
# Every widget kind, framed, at every size: no opaque pixel may lie outside its card (a clipped or overflowing line, like the clock's seconds bar that
# used to be drawn below the card). The shadow (translucent) and the bubbles that rise above the top-right corner are allowed.
# Also renders them so the pictures can be looked at: tools/widget-audit.sh --keep DIR
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/shell/target/release/nemo-shell"; [[ -x $BIN ]] || BIN="$ROOT/shell/target/debug/nemo-shell"
[[ -x $BIN ]] || { echo "widget-audit: no nemo-shell binary" >&2; exit 2; }
KEEP=""; [[ ${1:-} == --keep ]] && KEEP="${2:-}"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/home" "$TMP/out"
targets=(); for k in clock graph net media cal weather bt script sky vitals week; do for v in sf mf lf; do targets+=("widget:$k-$v"); done; done
env -i HOME="$TMP/home" PATH="/usr/bin:/bin" LANG=C.UTF-8 NEMO_FROZEN_NOW=1795000000 "$BIN" shot --defaults --transparent --lang en -o "$TMP/out" "${targets[@]}" >/dev/null 2>&1 || { echo "widget-audit: the screenshot run failed" >&2; exit 2; }
[[ -n $KEEP ]] && { mkdir -p "$KEEP" && cp "$TMP"/out/widget-*.png "$KEEP"/; }
python3 -I - "$TMP/out" <<'PY'
import glob, os, sys
from PIL import Image
M, RISE = 12, 16          # shadow margin on the sides and bottom, room above for the reveal and the bubbles (widgets.rs)
bad = []
files = sorted(glob.glob(sys.argv[1] + "/widget-*-?f.png"))
for f in files:
    im = Image.open(f).convert("RGBA"); w, h = im.size; px = im.load()
    x0, y0, x1, y1 = M, RISE, w - M, h - M
    out, first = 0, None
    for y in range(h):
        for x in range(w):
            if x0 - 1 <= x < x1 + 1 and y0 - 1 <= y < y1 + 1:
                continue
            if y < RISE and x > w - M - 34:
                continue
            if px[x, y][3] > 150:
                out += 1; first = first or (x, y)
    if out:
        bad.append(f"{os.path.basename(f)}: {out} opaque pixel(s) outside the card, first at {first} of {w}x{h}")
if not files:
    print("widget-audit: no pictures rendered"); sys.exit(2)
print(f"widget-audit: {len(files)} cards, " + ("none clipped or overflowing" if not bad else f"{len(bad)} with content outside the card"))
for b in bad: print("  " + b)
sys.exit(1 if bad else 0)
PY
