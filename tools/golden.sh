#!/usr/bin/env bash
# Visual regression: every screenshot target of `nemo-shell shot` is rendered with default settings and its SHA-256 is compared with
# tools/golden.txt. A change in the shared drawing code, a theme, a font or one app's layout shows up as the list of targets whose
# picture changed, and the new pictures are kept for a look. The clock is frozen (NEMO_FROZEN_NOW), so pictures with the time and date are compared too; tools/golden-skip.txt lists any target that is still unstable.
#   tools/golden.sh            compare (exit 1 and the list on a difference)
#   tools/golden.sh --update   after looking at the changes: take the current pictures as the reference
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/shell/target/release/nemo-shell"
# --update records what the binary draws: a binary older than the sources would record stale pictures (it did, more than once),
# so --update rebuilds the release shell first
if [[ ${1:-} == --update && -z ${NEMO_GOLDEN_NO_BUILD:-} ]]; then
  echo "golden: building the release shell first (NEMO_GOLDEN_NO_BUILD=1 skips)" >&2
  cargo build --release --locked -p nemo-shell --manifest-path "$ROOT/shell/Cargo.toml" -q || { echo "golden: the build failed" >&2; exit 2; }
fi
[[ -x $BIN ]] || BIN="$ROOT/shell/target/debug/nemo-shell"
[[ -x $BIN ]] || { echo "golden: no nemo-shell binary: build the shell first" >&2; exit 2; }
# a comparison against a stale binary reports differences that are not real: say so
if [[ -x $BIN ]] && [[ -n "$(find "$ROOT/shell/src" "$ROOT/shell/px/src" -name '*.rs' -newer "$BIN" -print -quit 2>/dev/null)" ]]; then
  echo "golden: warning: sources are newer than $BIN; differences may only mean it needs a rebuild" >&2
fi
GOLD="$ROOT/tools/golden.txt" SKIP="$ROOT/tools/golden-skip.txt"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/home" "$TMP/out"
# an empty environment: user, host, locale and the check's own variables must not leak into a picture
env -i HOME="$TMP/home" PATH="/usr/bin:/bin" LANG=C.UTF-8 NEMO_FROZEN_NOW=1795000000 NEMO_NO_KBD=1 "$BIN" shot --defaults -o "$TMP/out" >/dev/null 2>&1 || { echo "golden: the screenshot run failed" >&2; exit 2; }
now="$TMP/now.txt"
(cd "$TMP/out" && for f in *.png; do grep -qxF "$f" "$SKIP" 2>/dev/null || sha256sum "$f"; done) >"$now"
if [[ ${1:-} == --update ]]; then
  cp "$now" "$GOLD"
  echo "golden: $(wc -l <"$GOLD") pictures recorded"
  exit 0
fi
[[ -f $GOLD ]] || { echo "golden: no tools/golden.txt: run tools/golden.sh --update" >&2; exit 2; }
changed="$(diff <(sort -k2 "$GOLD") <(sort -k2 "$now") | grep '^[<>]' | awk '{print $3}' | sort -u)"
if [[ -z $changed ]]; then
  echo "golden: $(wc -l <"$GOLD") pictures unchanged"
  exit 0
fi
keep="${XDG_CACHE_HOME:-$HOME/.cache}/nemo/golden-changed"
rm -rf "$keep" && mkdir -p "$keep"
n=0
while read -r f; do [[ -n $f && -f $TMP/out/$f ]] && cp "$TMP/out/$f" "$keep/" && n=$((n + 1)); done <<<"$changed"
echo "golden: $(wc -l <<<"$changed") pictures differ from the reference:"
sed 's/^/  /' <<<"$changed" | head -40
echo "the new pictures are in $keep; if the change is intended: tools/golden.sh --update"
exit 1
