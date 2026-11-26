#!/usr/bin/env bash
# `cargo test` for the shell workspace in a throwaway home: HOME and every XDG folder point into a temporary directory, so no test can
# touch the real ~/.config/nemo, ~/.local/state/nemo or ~/.cache (finding #6: tests used to write the update keys into the real state
# folder). Afterwards it checks that the real state folder's files did not change under us and that the tests really wrote into the
# throwaway home (so a test that bypasses the environment is noticed).
#   tools/test.sh [cargo test arguments...]     default: --release --locked --workspace --quiet
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REAL_HOME="$HOME"
export CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}"
# the real files that only a test (never the running desktop) would rewrite: the update key list
real_keys="$REAL_HOME/.local/state/nemo/update-keys"
before=""; [[ -e $real_keys ]] && before="$(stat -c '%Y %s' "$real_keys")"
TH="$(mktemp -d)"
trap 'rm -rf "$TH"' EXIT
mkdir -p "$TH/home" "$TH/run"; chmod 700 "$TH/run"
args=("$@"); ((${#args[@]})) || args=(--release --locked --workspace --quiet)
mkdir -p "$TH/greet-look"
env NEMO_GREET_LOOK="$TH/greet-look" HOME="$TH/home" XDG_CONFIG_HOME="$TH/home/.config" XDG_STATE_HOME="$TH/home/.local/state" XDG_DATA_HOME="$TH/home/.local/share" \
  XDG_CACHE_HOME="$TH/home/.cache" XDG_RUNTIME_DIR="$TH/run" cargo test --manifest-path "$ROOT/shell/Cargo.toml" "${args[@]}"
rc=$?
after=""; [[ -e $real_keys ]] && after="$(stat -c '%Y %s' "$real_keys")"
if [[ $before != "$after" ]]; then echo "tools/test.sh: the real $real_keys changed during the tests" >&2; rc=1; fi
echo "tools/test.sh: the tests wrote $(find "$TH/home" -type f 2>/dev/null | wc -l) file(s) into the throwaway home (none into the real one)" >&2
exit $rc
