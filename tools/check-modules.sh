#!/usr/bin/env bash
# Every module next to this repository, the way check.sh checks the shell: no warnings, unit tests,
# i18n, every state rendered in both looks and every language, scripts parse.
# Usage: tools/check-modules.sh [nemo-files …]    the modules live in NEMO_MODULES_DIR (../nemo-modules)
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODS="${NEMO_MODULES_DIR:-$ROOT/../nemo-modules}"
# shellcheck source=/dev/null
[[ -f $MODS/env.sh ]] && . "$MODS/env.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
fail=0
bad() { printf '\e[31m  ✗ %s\e[0m\n' "$1"; fail=1; }
ok() { printf '\e[32m  ✓ %s\e[0m\n' "$1"; }
DEV="$ROOT/shell/target/release/nemo-dev"
[[ -x $DEV ]] || cargo build --release --locked -p nemo-dev --manifest-path "$ROOT/shell/Cargo.toml" || exit 1
printf '{"preset":"plain"}\n' >"$TMP/plain.json"
printf '{"preset":"nemo","palette":"ash"}\n' >"$TMP/nemo.json"
list=("$@")
# nemo-px and nemo-agents are checked with the shell (tools/check.sh): they live in shell/px and shell/agents
((${#list[@]})) || list=(nemo-files nemo-view nemo-play nemo-shot nemo-greet)
for m in "${list[@]}"; do
  d="$MODS/$m"
  [[ -d $d ]] || { bad "$m: no $d"; continue; }
  printf '\e[1m── %s\e[0m\n' "$m"
  if out="$(cd "$d" && cargo build --release --message-format short 2>&1)"; then
    if grep -q 'warning:' <<<"$out"; then
      grep 'warning:' <<<"$out" | grep -v 'generated [0-9]* warning' | head
      bad "warnings"
    else
      ok "build"
    fi
  else
    grep -E '^(error|.*: error)' <<<"$out" | head
    bad "build"
    continue
  fi
  (cd "$d" && cargo test --release --quiet >"$TMP/test.log" 2>&1) && ok "tests" || { tail -20 "$TMP/test.log"; bad "tests"; }
  for f in "$d"/*.sh; do [[ -f $f ]] && { bash -n "$f" || bad "syntax: $f"; }; done
  "$DEV" check-i18n "$d/src" >/dev/null && ok "i18n" || { "$DEV" check-i18n "$d/src" | head; bad "i18n"; }
  exe="$d/target/release/$m"
  for look in plain nemo; do
    for lang in en ru pt-BR es de fr uk pl; do
      # --fit: a text cut by an ellipsis or running into another fails too
      if NEMO_STYLE="$TMP/$look.json" "$exe" shot --fit --lang "$lang" -o "$TMP/$m-$look-$lang" >"$TMP/shot.log" 2>&1; then :; else
        tail -5 "$TMP/shot.log"
        bad "shot: $look, $lang"
      fi
    done
  done
  ok "renders: plain and nemo, every language, every text fits"
  if [[ $m != nemo-greet ]]; then
    h="$TMP/home-$m"
    mkdir -p "$h"
    box=(env -u XDG_DATA_HOME -u XDG_CONFIG_HOME -u XDG_STATE_HOME -u NEMO_STYLE -u PREFIX HOME="$h")
    if "${box[@]}" "$exe" install </dev/null >/dev/null 2>&1 || [[ -e $h/.local/bin/$m ]]; then bad "install without a yes"; fi
    if "${box[@]}" "$exe" install --yes --all >/dev/null 2>&1 && [[ -x $h/.local/bin/$m ]] && "${box[@]}" "$exe" uninstall --yes >/dev/null 2>&1 && [[ ! -e $h/.local/bin/$m ]]; then ok "install, uninstall"; else bad "install, uninstall"; fi
  fi
done
echo
if ((fail)); then
  printf '\e[31mcheck-modules: FAILED\e[0m\n'
  exit 1
fi
printf '\e[32mcheck-modules: all good\e[0m\n'
