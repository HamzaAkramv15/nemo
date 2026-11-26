# Sourced by tools/budget.sh and tools/power-bench.sh: a second niri running as a window inside the current session, with its own
# nemo-shell, a throwaway home, a private D-Bus session and its own runtime folder, so measurements do not depend on what the live desktop
# is doing and nothing the fixture does can reach the real one (no shared IPC socket, no shared notification or tray names, no second
# polkit agent: nemo-agents is left off its PATH). It needs a running Wayland session; without one `nested_available` fails.
#
#   nested_available               returns 0 when a nested session can be started
#   nested_start FIXTURE           FIXTURE: minimal (default settings) | realistic (dock, 3 designed widgets, the shipped reef wallpaper) | bare (no widgets) | many (eight widgets)
#                                  sets NESTED_DIR NESTED_NIRI_PID NESTED_SHELL_PID
#   nested_stop                    stops it and removes the folder
# The nested output is niri's default window (1280x720), not the real 1920x1080: compare fixtures with fixtures, never with the live shell.

NESTED_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

nested_niri_bin() {
  local b
  for b in "$HOME/.local/lib/nemo-niri/current/niri" "$(command -v niri 2>/dev/null)"; do [[ -n $b && -x $b ]] && { echo "$b"; return 0; }; done
  return 1
}

nested_host_socket() {
  local d="${WAYLAND_DISPLAY:-}"
  [[ -n $d ]] || return 1
  [[ $d == /* ]] && { echo "$d"; return 0; }
  echo "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$d"
}

nested_available() {
  local s; s="$(nested_host_socket)" && [[ -S $s ]] && nested_niri_bin >/dev/null && command -v dbus-run-session >/dev/null &&
    [[ -x $NESTED_ROOT/shell/target/release/nemo-shell ]]
}

nested_fixture_settings() {
  # NESTED_SETTINGS_FILE=/path/shell.json: a settings file of your own (for reproducing a report); the fixture name is then only a label
  if [[ -n ${NESTED_SETTINGS_FILE:-} ]]; then cat "$NESTED_SETTINGS_FILE"; return; fi
  case "$1" in
    minimal) echo '{}' ;;
    realistic)
      cat <<'JSON'
{"wallpaper":"~/.local/share/nemo/wallpapers/nemo-reef.png",
 "widgets":[
  {"id":1,"kind":"sky","on":true,"x":700,"y":60,"size":"m","frame":true,"date":true},
  {"id":2,"kind":"week","on":true,"x":40,"y":60,"size":"m","frame":true,"date":true},
  {"id":3,"kind":"vitals","on":true,"x":900,"y":420,"size":"m","frame":true,"date":true}]}
JSON
      ;;
    # for measuring what widgets cost: no widgets at all, and eight of them (not part of the default budget)
    bare) echo '{"widgets":[]}' ;;
    many)
      python3 - <<'PY'
import json
kinds = ["sky", "week", "vitals", "clock", "cal", "graph", "net", "weather"]
print(json.dumps({"widgets": [{"id": i + 1, "kind": k, "on": True, "x": 20 + (i % 4) * 300, "y": 40 + (i // 4) * 260, "size": "m", "frame": True, "date": True} for i, k in enumerate(kinds)]}))
PY
      ;;
    *) echo "nested_fixture_settings: unknown fixture $1" >&2; return 1 ;;
  esac
}

nested_start() {
  local fixture="${1:-minimal}" host niri t i
  host="$(nested_host_socket)" || return 1
  niri="$(nested_niri_bin)" || return 1
  t="$(mktemp -d "${TMPDIR:-/tmp}/nemo-nested.XXXXXX")"
  NESTED_DIR="$t"
  mkdir -p "$t/home/.config/nemo" "$t/home/.local/share/nemo/wallpapers" "$t/run" "$t/bin"
  chmod 700 "$t/run"
  # only the shell on the PATH: it must not find nemo-agents (polkit agent, automount) or a second launcher to start
  ln -s "$NESTED_ROOT/shell/target/release/nemo-shell" "$t/bin/nemo-shell"
  # NESTED_EXTRA_BINS="nemo-shot ...": programs a test needs and that are safe in a nested session (tools/e2e.sh record)
  for b in ${NESTED_EXTRA_BINS:-}; do p="$(command -v "$b")" && ln -s "$p" "$t/bin/$b"; done
  nested_fixture_settings "$fixture" >"$t/home/.config/nemo/shell.json" || return 1
  cp "$NESTED_ROOT/config/wallpapers/nemo-reef.png" "$t/home/.local/share/nemo/wallpapers/" 2>/dev/null
  printf 'spawn-at-startup "nemo-shell"\n' >"$t/niri.kdl"
  # GIO_USE_VFS=local: no gvfs daemon (it leaves a FUSE mount inside the folder); GSETTINGS_BACKEND=memory: nothing reaches dconf
  mkdir -p "$t/greet-look"
  # свой каталог вида экрана входа: иначе вложенный шелл пишет в настоящий /var/lib/nemo-greet/look и подменяет обои на экране входа
  setsid env -i NEMO_GREET_LOOK="$t/greet-look" HOME="$t/home" XDG_CONFIG_HOME="$t/home/.config" XDG_STATE_HOME="$t/home/.local/state" XDG_DATA_HOME="$t/home/.local/share" \
    XDG_CACHE_HOME="$t/home/.cache" XDG_RUNTIME_DIR="$t/run" WAYLAND_DISPLAY="$host" PATH="$t/bin:/usr/bin:/bin" LANG=C.UTF-8 \
    GIO_USE_VFS=local GSETTINGS_BACKEND=memory NEMO_NO_KBD=1 NEMO_TEST_IPC=1 dbus-run-session -- "$niri" -c "$t/niri.kdl" >"$t/niri.log" 2>&1 &
  NESTED_NIRI_PID=$!
  NESTED_SHELL_PID=""
  for i in $(seq 1 40); do
    NESTED_SHELL_PID="$(nested_find_shell)"
    [[ -n $NESTED_SHELL_PID ]] && break
    sleep 0.5
  done
  [[ -n $NESTED_SHELL_PID ]] || { echo "nested_start: the shell did not start (see $t/niri.log)" >&2; return 1; }
  sleep "${NESTED_SETTLE:-35}"   # startup work (fonts, icons, first frames) is not idle
}

# the nemo-shell whose HOME is ours (the live one has the real HOME)
nested_find_shell() {
  local p
  for p in $(pgrep -x nemo-shell); do
    tr '\0' '\n' <"/proc/$p/environ" 2>/dev/null | grep -qx "HOME=$NESTED_DIR/home" && { echo "$p"; return 0; }
  done
  return 1
}

nested_stop() {
  [[ -n ${NESTED_NIRI_PID:-} ]] || return 0
  kill -TERM -- "-$NESTED_NIRI_PID" 2>/dev/null || kill -TERM "$NESTED_NIRI_PID" 2>/dev/null
  local i; for i in 1 2 3 4 5 6; do kill -0 "$NESTED_NIRI_PID" 2>/dev/null || break; sleep 0.5; done
  kill -KILL -- "-$NESTED_NIRI_PID" 2>/dev/null
  [[ -n ${NESTED_DIR:-} ]] && { fusermount3 -u "$NESTED_DIR/run/gvfs" 2>/dev/null; rm -rf "$NESTED_DIR"; }
  NESTED_NIRI_PID="" NESTED_SHELL_PID="" NESTED_DIR=""
}
