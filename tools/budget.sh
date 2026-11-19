#!/usr/bin/env bash
# Size and idle-cost budget. Always: the release binaries must not grow past their limit. When a shell is running
# (a session, or a nested compositor): idle memory (PSS), context switches per second, CPU and threads over a
# sampling window, each against tools/budget.txt. Without a running shell the runtime part says so and passes.
# Usage: tools/budget.sh [seconds]   (default 20)
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
B="$ROOT/tools/budget.txt"
lim() { awk -v k="$1" '$1 == k { print $2 }' "$B"; }
fail=0
ok() { printf '\e[32m✓ %s\e[0m\n' "$1"; }
bad() { printf '\e[31m✗ %s\e[0m\n' "$1"; fail=1; }
rel="$ROOT/shell/target/release"
for bin in yami-shell yami-agents; do
  [[ -f $rel/$bin ]] || continue
  kb=$(($(stat -c %s "$rel/$bin") / 1024)); max="$(lim "size_$bin")"
  if ((kb <= max)); then ok "$bin $((kb / 1024)).$((kb % 1024 * 10 / 1024)) MB (limit $((max / 1024)) MB)"; else bad "$bin is $kb KB, the limit is $max KB"; fi
done
pid="$(pgrep -x yami-shell | head -n1 || true)"
if [[ -z $pid ]]; then echo "  (no running yami-shell: idle memory and wakeups were not measured)"; exit $fail; fi
secs="${1:-20}"
sw() { awk '/^voluntary_ctxt_switches/{v=$2} /^nonvoluntary_ctxt_switches/{n=$2} END{print v+n}' "/proc/$pid/status"; }
ticks() { awk '{print $14+$15}' "/proc/$pid/stat"; }
a=$(sw); ta=$(ticks); sleep "$secs"; b=$(sw); tb=$(ticks)
pss=$(awk '/^Pss:/{print $2}' "/proc/$pid/smaps_rollup")
wake=$(((b - a) / secs)); cpu=$(((tb - ta) * 100 / (secs * 100)))  # whole percent of one CPU
threads=$(awk '/^Threads/{print $2}' "/proc/$pid/status")
chk() { local name=$1 val=$2 key=$3 unit=$4 max; max="$(lim "$key")"; if ((val <= max)); then ok "$name $val $unit (limit $max)"; else bad "$name $val $unit exceeds $max"; fi; }
chk "idle PSS" "$pss" pss_kb kB
chk "wakeups" "$wake" wakeups_per_s "/s"
chk "cpu" "$cpu" cpu_percent "%"
chk "threads" "$threads" threads ""
exit $fail
