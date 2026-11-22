#!/usr/bin/env bash
# Size and idle-cost budget.
#   Always: the release binaries must not grow past their limit.
#   Default: idle memory (PSS), wakeups per second (all threads), CPU and threads of a *fixture* shell, a fresh nemo-shell inside a nested niri with a throwaway
#            home (tools/nested-session.sh), for each fixture against its limits in tools/budget.txt. The result does not depend on what the live desktop is doing
#            (browser, wallpaper, widgets, a locked screen), which used to make this check fail or pass for reasons unrelated to the code.
#            Without a Wayland session the runtime part says so and passes.
#   --live:  the same numbers for the shell of the running desktop. A report only: it never fails, because the live desktop's state is the user's.
# Usage: tools/budget.sh [seconds] [--live]   (seconds: sampling window, default 20)
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
B="$ROOT/tools/budget.txt"
lim() { awk -v k="$1" '$1 == k { print $2 }' "$B"; }
fail=0
ok() { printf '\e[32m✓ %s\e[0m\n' "$1"; }
bad() { printf '\e[31m✗ %s\e[0m\n' "$1"; fail=1; }
note() { printf '\e[33m· %s\e[0m\n' "$1"; }
secs=20 live=0
for a in "$@"; do case "$a" in --live) live=1 ;; *[!0-9]*) echo "usage: tools/budget.sh [seconds] [--live]" >&2; exit 2 ;; *) secs="$a" ;; esac; done

rel="$ROOT/shell/target/release"
for bin in nemo-shell nemo-agents; do
  [[ -f $rel/$bin ]] || continue
  kb=$(($(stat -c %s "$rel/$bin") / 1024)); max="$(lim "size_$bin")"
  if ((kb <= max)); then ok "$bin $((kb / 1024)).$((kb % 1024 * 10 / 1024)) MB (limit $((max / 1024)) MB)"; else bad "$bin is $kb KB, the limit is $max KB"; fi
done

# context switches of every thread of a process (the main thread's own counters, which /proc/PID/status shows, miss the workers)
sw() { cat /proc/"$1"/task/*/status 2>/dev/null | awk '/^voluntary_ctxt_switches|^nonvoluntary_ctxt_switches/{s+=$2} END{print s+0}'; }
ticks() { awk '{print $14+$15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
# the shell and the programs it keeps running (playerctl, wlsunset, pactl ...): the cost of the shell *stack*
family() { local p=$1; echo "$p"; pgrep -P "$p" 2>/dev/null; }

# measure PID SECONDS -> prints "pss_kb wakeups_per_s cpu_tenths_percent threads children child_wakeups_per_s child_cpu_tenths"
measure() {
  local pid=$1 s=$2 kids a ta ka tka b tb kb tkb p
  kids="$(pgrep -P "$pid" | tr '\n' ' ')"
  a=$(sw "$pid"); ta=$(ticks "$pid"); ka=0; tka=0
  for p in $kids; do ka=$((ka + $(sw "$p"))); tka=$((tka + $(ticks "$p"))); done
  sleep "$s"
  b=$(sw "$pid"); tb=$(ticks "$pid"); kb=0; tkb=0
  for p in $kids; do kb=$((kb + $(sw "$p"))); tkb=$((tkb + $(ticks "$p"))); done
  printf '%s %s %s %s %s %s %s\n' "$(awk '/^Pss:/{print $2}' "/proc/$pid/smaps_rollup")" $(((b - a) / s)) $(((tb - ta) * 1000 / (s * 100))) \
    "$(ls /proc/"$pid"/task | wc -l)" "$(wc -w <<<"$kids")" $(((kb - ka) / s)) $(((tkb - tka) * 1000 / (s * 100)))
}

tenths() { printf '%d.%d' $(($1 / 10)) $(($1 % 10)); }

# report NAME "numbers" [KEYPREFIX]: with KEYPREFIX each number is checked against its limit, without it the numbers are only printed
report() {
  local name=$1 pss wake cpu thr nk kw kc; read -r pss wake cpu thr nk kw kc <<<"$2"
  local pre="${3:-}"
  if [[ -z $pre ]]; then
    note "$name: PSS $pss kB, wakeups $wake/s, cpu $(tenths "$cpu") %, $thr threads; $nk child programs: $kw wakeups/s, cpu $(tenths "$kc") % (report only)"
    return
  fi
  chk() { local label=$1 val=$2 key=$3 unit=$4 max; max="$(lim "${pre}_$key")"
    [[ -n $max ]] || { note "$name: $label $val $unit (no limit set for ${pre}_$key)"; return; }
    if ((val <= max)); then ok "$name: $label $val $unit (limit $max)"; else bad "$name: $label $val $unit exceeds $max"; fi; }
  chk "idle PSS" "$pss" pss_kb kB
  chk "wakeups" "$wake" wakeups_per_s "/s"
  chk "cpu" "$((cpu / 10))" cpu_percent "%"
  chk "threads" "$thr" threads ""
  note "$name: child programs ($nk): $kw wakeups/s, cpu $(tenths "$kc") %; shell cpu $(tenths "$cpu") %"
}

if ((live)); then
  pid="$(pgrep -x nemo-shell | head -n1 || true)"
  [[ -n $pid ]] || { echo "  (no running nemo-shell)"; exit 0; }
  if [[ -e ${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/nemo-locked ]]; then echo "  (the screen is locked: the lock surface ticks, not an idle shell)"; fi
  report "live desktop shell" "$(measure "$pid" "$secs")"
  exit 0
fi

# shellcheck source=tools/nested-session.sh
source "$ROOT/tools/nested-session.sh"
if ! nested_available; then
  echo "  (no Wayland session, niri or release shell: the fixture shell was not measured; build it, run inside a session, and try again)"
  exit $fail
fi
trap 'nested_stop' EXIT
for fx in minimal realistic; do
  if nested_start "$fx"; then
    report "fixture $fx" "$(measure "$NESTED_SHELL_PID" "$secs")" "$fx"
  else
    bad "fixture $fx did not start"
  fi
  nested_stop
done
exit $fail
