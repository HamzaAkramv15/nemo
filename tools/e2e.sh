#!/usr/bin/env bash
# End-to-end checks of the real shell in a nested niri with a throwaway home and a private bus (nothing touches your desktop):
# it clicks through the real settings pages, widgets and bar by the shell's own click path and checks what the shell wrote.
#   tools/e2e.sh [CASE...]   CASE: panels tiles layouts gallery bar island wallpapers reset record undo keyboard barmods rtl groups dnd (default: all)   pictures: E2E_OUT=DIR
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tools/nested-session.sh"
nested_available || { echo "e2e: no Wayland session, niri or release shell: nothing to run"; exit 0; }
trap nested_stop EXIT
SETTINGS="$(mktemp)"; trap 'nested_stop; rm -f "$SETTINGS"' EXIT
cat >"$SETTINGS" <<'JSON'
{"island":true,"wallDir":"~/Pictures","barLeft":["workspaces"],"barCenter":[],"barRight":["clock"],"widgets":[{"id":1,"kind":"sky","on":true,"x":60,"y":60,"size":"m","frame":true,"date":true},{"id":2,"kind":"quick","on":true,"x":60,"y":330,"size":"m","frame":true}]}
JSON
NESTED_EXTRA_BINS="nemo-shot" NESTED_SETTINGS_FILE="$SETTINGS" NESTED_SETTLE="${NESTED_SETTLE:-8}" nested_start e2e || exit 1
export E2E_BUS="$(tr '\0' '\n' </proc/$NESTED_SHELL_PID/environ | grep ^DBUS_SESSION_BUS_ADDRESS= | cut -d= -f2-)" E2E_RUN="$NESTED_DIR/run" E2E_HOME="$NESTED_DIR/home" E2E_WL="$(cd "$NESTED_DIR/run" && ls wayland-* 2>/dev/null | grep -v lock | head -1)" E2E_OUT="${E2E_OUT:-$(mktemp -d)}"
mkdir -p "$E2E_OUT"
cd "$ROOT/tools" && python3 -I -c "import sys; sys.path.insert(0, '.'); import e2e, e2e_cases; sys.exit(e2e_cases.run(e2e, sys.argv[1:] or ['all']))" "$@"
rc=$?
echo "pictures: $E2E_OUT"
exit $rc
