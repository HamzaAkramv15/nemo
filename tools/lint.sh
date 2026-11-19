#!/usr/bin/env bash
# Lint ratchet: clippy over the whole workspace must not produce more warnings than the recorded baseline
# (tools/lint-baseline). Lower the baseline when you fix some; never raise it to make a check pass.
# Also runs cargo-deny when it is installed (deny.toml). Usage: tools/lint.sh [--update]
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="$ROOT/tools/lint-baseline"
cd "$ROOT/shell" || exit 1
out="$(cargo clippy --workspace --message-format short 2>&1)"
# one line per distinct warning site
n="$(grep -E '^[^ ]+:[0-9]+:[0-9]+: warning' <<<"$out" | sort -u | wc -l)"
want="$(cat "$BASE" 2>/dev/null || echo 0)"
if [[ ${1:-} == --update ]]; then echo "$n" >"$BASE"; echo "baseline set to $n"; exit 0; fi
rc=0
if ((n > want)); then
  grep -E '^[^ ]+:[0-9]+:[0-9]+: warning' <<<"$out" | sort -u | head -40
  printf '\e[31m✗ clippy: %d warnings, the baseline allows %d\e[0m\n' "$n" "$want"
  rc=1
elif ((n < want)); then
  printf '\e[32m✓ clippy: %d warnings (baseline %d: lower tools/lint-baseline with tools/lint.sh --update)\e[0m\n' "$n" "$want"
else
  printf '\e[32m✓ clippy: %d warnings, as recorded\e[0m\n' "$n"
fi
if command -v cargo-deny >/dev/null 2>&1; then
  if (cd "$ROOT/shell" && cargo deny --manifest-path Cargo.toml check >/dev/null 2>&1); then printf '\e[32m✓ cargo-deny\e[0m\n'; else (cd "$ROOT/shell" && cargo deny check 2>&1 | tail -15); printf '\e[31m✗ cargo-deny\e[0m\n'; rc=1; fi
else
  echo "  (cargo-deny is not installed: advisories, licences and sources were not checked)"
fi
exit $rc
