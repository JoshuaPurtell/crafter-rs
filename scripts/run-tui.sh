#!/usr/bin/env bash
set -euo pipefail

ZIG_ROOT="/Users/joshpurtell/Documents/GitHub/mission-control/vendor/zig-0.14.0"
if [[ ! -x "${ZIG_ROOT}/zig" ]]; then
  echo "Expected Zig at ${ZIG_ROOT}/zig. Update scripts/run-tui.sh if your Zig 0.14.x lives elsewhere." >&2
  exit 1
fi

PATH="${ZIG_ROOT}:$PATH" cargo run -p crafter-tui
