#!/usr/bin/env bash
# Build the release binaries and time the plain step() loop over ZEXALL,
# pinned to one core so the number is comparable between commits.
# Usage: scripts/bench.sh [z80-bench options...]
#   env: CPU (logical core to pin to, default 4), RUNS (repetitions, default 1)
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
CPU=${CPU:-4}
RUNS=${RUNS:-1}
cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
dirty=$(git -C "$ROOT" diff --quiet HEAD -- . ':!README.md' ':!docs' 2>/dev/null || echo " (dirty)")
echo "commit $(git -C "$ROOT" rev-parse --short HEAD)$dirty, cpu $CPU, $(rustc --version)"
for _ in $(seq "$RUNS"); do
    taskset -c "$CPU" "$ROOT/target/release/z80-bench" "$ROOT/conformance/zex/zexall.json" "$@"
done
