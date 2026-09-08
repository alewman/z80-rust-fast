# Handoff: start `z80-rust-fast`, a fast Z80 core that stays provably equivalent to z80-python

## Context you are inheriting

Two repositories exist and are done, in the sense that every claim in them is reproduced and recorded:

- `z80-python` (github alewman/z80-python, local clone `/data/emu/z80-python`, `main` at `d1ec9c7`): the reference core, "the code is the spec". Certified against SingleStepTests (1,604,000 cases with T-states), ZEXDOC/ZEXALL, raxoft/z80test (real silicon), FUSE's core tests (1,350 of 1,356 agree, six explained), and an interrupt cross-check. It ships a conformance kit: manifests, one shared host, reference traces, a lockstep `diff`, and since #7 a `checkpoints` subcommand that splits a multi-billion-record run into parallel segments.
- `z80-rust` (github alewman/z80-rust, private, local clone `/data/emu/z80-rust`, `main` at `43c5122`): a module-for-module transcription of z80-python, proven equivalent at every processor boundary. Six rungs pass at z80-python `cab1598` (whose instruction core is byte-identical to `d1ec9c7`): the three example manifests, all 1,604,000 vectors, ZEXALL and ZEXDOC in lockstep (116 segments each, every segment identical), z80test, the ten interrupt manifests, and FUSE. CI reproduces rungs 1, 2, 5, and 6 on every push. Read its `README.md`, `docs/handoff-brief.md` (the prompt that produced it) and `docs/build-record.md` (what happened when it ran) first.

z80-rust is deliberately not fast. It keeps the reference's if-chain dispatch, flag setters, and per-fetch bookkeeping so a reader can hold it beside the Python. Its measured speed on this machine (i9-13900K, one core, release profile, no tuning): a plain `step()` loop runs all of ZEXALL, 5,764,169,474 instructions, in **116 s**, about 50 M instructions/s or a 400 MHz Z80; `z80-trace --no-trace`, which also captures state before and after every boundary, takes 151 s; the vector corpus runs in 9 s warm.

## Your task

Create `alewman/z80-rust-fast` as a fork of z80-rust (clone with history, so the transcription is the first commit and every optimization is a diff against it) and make the core as fast as a well-built Rust core can be, **without changing what it does**. The ladder is the definition of "what it does". A fast core that fails any rung is not a result; a fast core that passes all of them at a stated commit of z80-python is.

### What must not change

- The `Bus` trait, `Z80<B>`, `step()`, `capture_state()`/`restore_state()`, the lifecycle API, and `Fault`. Hosts written for z80-rust must compile against z80-rust-fast unchanged. Additions are fine; removals and renames are not.
- The trace producer (`z80-trace`), the manifests under `conformance/`, `tests/data/`, and every `scripts/rung*.sh`. They are how equivalence is proven and they must keep working unchanged. Trace capture may be gated behind a Cargo feature or a const generic if the per-fetch bookkeeping costs measurable time, but the default build of the binaries must still produce traces.
- Behavior at every boundary: kind, T-states, instruction bytes, all 28 `CpuState` fields. In particular every repeat of a block instruction is its own boundary, and a run of DD/FD prefixes is one. If you believe the reference is wrong, stop, show the manifest and the divergence, and ask; the vectors and hardware suites are the authority, not either implementation.

### What may change

Everything else: dispatch (jump tables, `match`, decoded-instruction caches), flag computation (lookup tables for S/Z/P, computing F as a byte instead of through setters), register representation, memory access patterns, inlining, `#[cold]` paths, build profile (LTO, `codegen-units = 1`, `target-cpu`, PGO), and the module layout. Readability against the Python is no longer a requirement; each optimization should still be explained in its commit message with its measurement.

### Milestones, each with its acceptance test

1. **Fork, baseline, and harness.** Clone z80-rust with history; add a `bench` binary that runs ZEXALL with a plain `step()` loop and the `cpm-minimal` traps (the loop in z80-rust's README "Speed" section is the model) and prints instructions, T-states, wall time, instructions/s, and equivalent MHz; add a `no_std` split (the core modules need nothing from `std`; `trace` and `conformance` go behind a default `std` feature) because the next project is a WebAssembly build. Acceptance: `cargo build --no-default-features` succeeds for the core; every rung passes unchanged; the baseline number is recorded in the README with the commit hash and machine.
2. **Profile before touching anything.** `perf record` on the bench; record the top symbols in the README. Every optimization after this cites what it targeted.
3. **Optimize, one change per commit, ladder-gated.** For each commit: the bench number before and after, and rungs 1, 2, 4, 5, and 6 re-run clean (seconds). Rung 3 (about 7 hours per exerciser with 30 PyPy processes on this machine; `scripts/rung3.sh` or z80-python's `checkpoints` subcommand) is required before any tag or release and after any change to dispatch or block instructions. Suggested order, to be revised by the profile: dispatch shape; flags; the per-fetch trace bookkeeping; build profile last, because it changes every earlier number.
4. **Done.** A README stating, with the four pinning numbers z80-python's `docs/conformance.md` requires (z80-python version/commit, trace schema version, SingleStepTests revision, z80test release) plus the FUSE release, which rungs pass at which commit; a table of every optimization with its measured effect; and CI reproducing rungs 1, 2, 5, and 6 on every push as z80-rust does.

Do not skip a rung or reorder them; a divergence is cheapest to diagnose at the lowest rung that shows it.

## Constraints

- Same discipline as both parents: every claim pinned to a commit hash and reproduced; state exactly what was and was not run; "the whole ladder" means every rung, so do not say it when rung 3 was skipped. Report failures with their output.
- Commit in reviewable units with clear messages. Create the GitHub repository private under the user's account (`gh` is authenticated as alewman) and push when the user asks. Attribution trailer on commits: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Environment: cargo and rustc 1.93 at `/usr/bin`; `cargo clippy` needs the rustup stable toolchain and its own target directory (see z80-rust's memory notes: `CARGO_TARGET_DIR=target/clippy RUSTC=~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc rustup run stable cargo clippy`). The reference's CPython venv is `/data/emu/z80-python/.venv/bin/python` and PyPy is `/data/emu/z80-python/.venv-pypy/bin/pypy3`; PyPy is the one to use for rung 3. The corpus is at `/data/emu/z80-python/tests/z80_test_vectors/v1`, the ZEX binaries at `/data/emu/z80-python/tests/zex/`, z80test at `/data/emu/z80-python/validation/z80test_data/`, FUSE at `/data/emu/z80-python/validation/fuse_data/`; z80-rust's `scripts/fetch_*.sh` fetch the same pinned inputs elsewhere. No sudo. The machine is shared; check `uptime` before starting a 30-process run.
- Start the session in `/data/emu`, not inside either existing clone, so the new repository is created beside them.

## What done looks like

`z80-rust-fast` passes the same six rungs as z80-rust at a stated z80-python commit, runs ZEXALL in a plain `step()` loop in a fraction of 116 s with each contributing change measured and explained, builds its core without `std`, and leaves a host written for z80-rust compiling unchanged. The next project after it is a WebAssembly build for the website that runs the rung 1 and rung 5 manifests in the browser against the committed reference traces before it runs a game; keep that in mind when choosing what to gate behind features, but do not start it.

Before starting, tell the user in a few sentences how you read this brief and what you will do first.
