#!/usr/bin/env bash
# Profile-guided optimization, as an optional build: instrument, run the
# training workloads, merge the profile, rebuild with it, and time the
# result the same way scripts/bench.sh does. The output binaries are under
# target/pgo-use/release/; `cargo build --release` is unchanged, because a
# committed .profdata would be tied to one compiler version.
#
# Needs llvm-profdata from the same LLVM as rustc (rustup's llvm-tools for
# the matching toolchain: `rustup toolchain install <rustc version>
# --component llvm-tools`). Set PROFDATA to its path if it is not found.
#
# Usage: scripts/pgo.sh   env: CPU (default 4), PROFDATA, ROOT
set -euo pipefail
ROOT=${ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
CPU=${CPU:-4}
version=$(rustc --version | awk '{print $2}')
PROFDATA=${PROFDATA:-"$HOME/.rustup/toolchains/$version-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"}
[ -x "$PROFDATA" ] || { echo "llvm-profdata not found at $PROFDATA" >&2; exit 2; }
base_flags=$(sed -n 's/^rustflags = \["-C", "\(.*\)"\]$/-C \1/p' "$ROOT/.cargo/config.toml")
profiles="$ROOT/target/pgo-profiles"
rm -rf "$profiles"
mkdir -p "$profiles"

echo "instrumented build"
RUSTFLAGS="$base_flags -C profile-generate=$profiles" CARGO_TARGET_DIR="$ROOT/target/pgo-gen" \
    cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
echo "training: ZEXALL and the secondary loops"
taskset -c "$CPU" "$ROOT/target/pgo-gen/release/z80-bench" "$ROOT/conformance/zex/zexall.json" >/dev/null
for loop in ix-loop ed-loop; do
    taskset -c "$CPU" "$ROOT/target/pgo-gen/release/z80-bench" "$ROOT/bench/$loop.json" >/dev/null
done
"$PROFDATA" merge -o "$profiles/merged.profdata" "$profiles"/*.profraw

echo "optimized build"
RUSTFLAGS="$base_flags -C profile-use=$profiles/merged.profdata -C llvm-args=-pgo-warn-missing-function" \
    CARGO_TARGET_DIR="$ROOT/target/pgo-use" cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
echo "commit $(git -C "$ROOT" rev-parse --short HEAD), cpu $CPU, $(rustc --version | cut -d' ' -f1-2), pgo"
taskset -c "$CPU" "$ROOT/target/pgo-use/release/z80-bench" "$ROOT/conformance/zex/zexall.json"
