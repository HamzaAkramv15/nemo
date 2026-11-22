#!/usr/bin/env bash
# Battery benchmark wrapper: see tools/power-bench.py (no root needed).
exec python3 -I "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/power-bench.py" "$@"
