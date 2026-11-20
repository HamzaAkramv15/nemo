#!/usr/bin/env bash
# The README pictures (.github/assets), made without touching your desktop: every picture is the real drawing code of the shell and
# the apps (`shot` in each program) with made-up data, so no window, title or file of yours gets in. The desktop picture is
# those panels placed on a plain pixel wallpaper. English and Russian. Needs the release builds of nemo-shell, nemo-files and
# the modules next to this tree (NEMO_MODULES_DIR, default ../nemo-modules).
#   tools/readme-shots-offline.sh
# (tools/readme-shots.sh does the same from a live session and needs the ash palette; this one does not.)
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODS="${NEMO_MODULES_DIR:-$ROOT/../nemo-modules}"
SH="$ROOT/shell/target/release/nemo-shell"
FILES="${NEMO_FILES_BIN:-$ROOT/../nemo-files/target/release/nemo-files}"
bin() { local m=$1; local p="$MODS/$m/target/release/$m"; [[ -x $p ]] || p="$ROOT/../$m/target/release/$m"; [[ -x $p ]] && printf '%s' "$p" || { echo "readme-shots-offline: no $m binary: build it first" >&2; exit 1; }; }
for b in "$SH" "$FILES"; do [[ -x $b ]] || { echo "readme-shots-offline: no $b: build it first" >&2; exit 1; }; done
VIEW="$(bin nemo-view)" SHOT="$(bin nemo-shot)" GREET="$(bin nemo-greet)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/home"
export HOME="$TMP/home"
for L in en ru; do
  d="$TMP/$L"; mkdir -p "$d"
  "$SH" shot --defaults --lang "$L" -o "$d" settings:color settings:keys settings:updates-new settings:agents popup:sound >/dev/null
  # the panels of the desktop picture keep their transparency, so they sit on the wallpaper
  "$SH" shot --defaults --transparent --lang "$L" -o "$d" launcher bar dock widget:clock widget:cal widget:graph widget:media >/dev/null
  (cd "$d" && "$FILES" shot grid --lang "$L" -o . >/dev/null && "$VIEW" shot --lang "$L" -o . >/dev/null && "$SHOT" shot --lang "$L" -o . >/dev/null && "$GREET" shot --lang "$L" -o . >/dev/null)
done
python3 "$ROOT/tools/readme-compose.py" "$TMP" "$ROOT/.github/assets"
