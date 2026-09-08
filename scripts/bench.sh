#!/usr/bin/env bash
# Time the plain step() loop over ZEXALL, pinned to one core, under several
# code layouts, and report each plus the minimum.
#
# Why layouts: the hot loop is sensitive to where the linker places it. Two
# builds whose step() code was byte-identical differed by 9% because one
# landed 0x5e0 bytes later; with branch-target blocks aligned to 32 or 64
# bytes (LLVM's -align-all-nofallthru-blocks) the same two builds agree
# within 1-3%. So every commit is measured under three layouts and the
# minimum over them is the number that says whether a change helped:
#   default  what `cargo build --release` gives (.cargo/config.toml aligns
#            branch-target blocks to 64 bytes)
#   noalign  the same build with no alignment flag (RUSTFLAGS overrides the
#            config), i.e. the layout LLVM picks on its own
#   nf5      32-byte alignment
#
# Usage: scripts/bench.sh [z80-bench options...]
#   env: CPU (logical core to pin to, default 4), RUNS (per layout, default 1),
#        LAYOUTS (default "default noalign nf5"), ROOT (checkout to measure)
set -euo pipefail
ROOT=${ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
CPU=${CPU:-4}
RUNS=${RUNS:-1}
LAYOUTS=${LAYOUTS:-"default noalign nf5"}
# Setting RUSTFLAGS replaces .cargo/config.toml's rustflags entirely, which
# is what "noalign" relies on; an empty string is not "unset", so it is
# passed explicitly.
declare -A FLAGS=(
    [noalign]="-C opt-level=3"
    [nf5]="-C llvm-args=-align-all-nofallthru-blocks=5"
    [nf6]="-C llvm-args=-align-all-nofallthru-blocks=6"
)
dirty=$(git -C "$ROOT" diff --quiet HEAD -- . ':!README.md' ':!docs' 2>/dev/null || echo " (dirty)")
echo "commit $(git -C "$ROOT" rev-parse --short HEAD)$dirty, cpu $CPU, $(rustc --version | cut -d' ' -f1-2)"
best=""
best_layout=""
for layout in $LAYOUTS; do
    if [ "$layout" = default ]; then
        cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
        bin="$ROOT/target/release/z80-bench"
    else
        RUSTFLAGS="${FLAGS[$layout]}" CARGO_TARGET_DIR="$ROOT/target/layout-$layout" \
            cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
        bin="$ROOT/target/layout-$layout/release/z80-bench"
    fi
    for _ in $(seq "$RUNS"); do
        line=$(taskset -c "$CPU" "$bin" "$ROOT/conformance/zex/zexall.json" "$@")
        echo "$layout: $line"
        seconds=$(sed -E 's/.* T-states, ([0-9.]+) s,.*/\1/' <<<"$line")
        if [ -z "$best" ] || awk "BEGIN{exit !($seconds < $best)}"; then
            best=$seconds
            best_layout=$layout
        fi
    done
done
echo "min over layouts: $best s ($best_layout)"
