#!/usr/bin/env bash
# Rung 7: instruction_timing answers as the reference's does, for every form of
# instruction (scripts/timing_reference.py lists them).
# Usage: scripts/rung7.sh [python]
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
PYTHON=${1:-"$ROOT/external/z80-python/.venv/bin/python"}
cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
WORK=$(mktemp -d)
trap 'rm -r "$WORK"' EXIT
"$PYTHON" -I "$ROOT/scripts/timing_reference.py" inputs > "$WORK/inputs"
"$PYTHON" -I "$ROOT/scripts/timing_reference.py" answers < "$WORK/inputs" > "$WORK/reference"
"$ROOT/target/release/z80-timing" < "$WORK/inputs" > "$WORK/port"
if cmp -s "$WORK/reference" "$WORK/port"; then
    echo "instruction_timing: $(wc -l < "$WORK/inputs") forms, every answer identical"
else
    diff "$WORK/reference" "$WORK/port" | head -20
    exit 1
fi
