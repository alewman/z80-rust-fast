# Build record

What happened when [handoff-brief.md](handoff-brief.md) was run, in the
order it happened, with the numbers. Timestamps are local (PDT). The
transcription this repository forks has its own record in
[z80-rust-build-record.md](z80-rust-build-record.md).

## 2026-09-08

- **07:30 to 07:40, reading.** The brief, every module of z80-rust, its
  scripts, CI, and build record. Two facts about the machine decided the
  tooling: `kernel.perf_event_paranoid` is 4, so `perf record` is not
  available without root, and neither `valgrind` nor `gdb` is installed.
  `ptrace` of one's own children is allowed (`yama/ptrace_scope` = 1), so
  the profile comes from a small ptrace sampler instead
  (`tools/sample-profile.py`).
- **07:34, fork** (9f51021). `git clone` of z80-rust with history; the
  package renamed, the crate name kept. The oracle inputs are symlinked
  into `external/` and `conformance/zex/` from the z80-python checkout
  (`d1ec9c7`, whose instruction core is byte-identical to the pinned
  `cab1598`: `git diff cab1598 d1ec9c7 -- src/z80_python/` touches only
  `conformance.py` and `trace.py`).
- **07:38, no_std split** (5a340ff). The core needed `Vec`, `fmt`, and
  `String` only; `alloc` and `core` supply them. `trace`, `conformance`, the
  binaries, and the rung 1 test sit behind the default `std` feature.
  `cargo build --no-default-features` builds the core; clippy is clean in
  both configurations.
- **07:38, bench** (52192d1). `z80-bench` and `scripts/bench.sh`.
- **07:40, ladder on the unchanged core.** Rungs 1, 2, 4, 5, 6 with the
  scripts: identical, 1,604,000 of 1,604,000, all three `all tests passed`,
  ten identical, `1350 agree, 6 expected divergences, 0 unexpected`.
- **07:41 to 07:52, baseline.** Two full ZEXALL runs pinned to logical
  CPU 4 (a P-core): 69.39 s and 69.43 s; a third at the committed
  `52192d1` on CPU 8: 69.28 s. 83.1 M instructions/s, 674 MHz.
  The brief quotes 116 s from z80-rust's README for the same loop. The
  difference is the core the loop lands on: a 1,000,000,000-instruction
  slice takes 11.96 s on P-core 4, 12.05 s unpinned, and 17.35 s on E-core
  20; where and under what load the earlier number was taken is not
  recorded. Every number in this repository is therefore taken pinned to one P-core
  with `scripts/bench.sh`, and the baseline is 69.3 s.
- **07:47 to 07:50, profile.** `tools/sample-profile.py` over a full
  ZEXALL run at `52192d1`: 33,840 samples. The `execute_main` if-chain is
  22% of self time, the bench loop and its trap compares 21%, `step`'s
  lifecycle checks 11%, the memory index 8%, `note_fetched` 2%. The
  table is in the README.
- **07:46 to 07:50, dispatch shape** (d27af51, 5814b1f). `execute_main`
  and `execute_index_opcode` as exhaustive matches. ZEXALL 69.3 s to 43.0 s
  on the first; the second moved only an IX loop (`bench/ix-loop.json`,
  3.86 s to 2.99 s).
- **07:50 to 08:08, the layout finding** (4f0dbe7). The ED dispatcher as
  a match made its own loop 10% faster and ZEXALL 9% slower (43.2 s to
  47.1 s). The `step()` code was byte-identical in both binaries; it had
  moved 0x5e0 bytes because the ED function shrank. Aligning branch-target
  blocks (`-C llvm-args=-align-all-nofallthru-blocks=5` or `=6`) brought
  the two builds within 1-3% of each other in both directions; aligning
  whole functions to 64 bytes did not. `scripts/bench.sh` now builds the
  default and the two aligned layouts and reports the minimum, and the
  three earlier commits were re-measured that way in a worktree so the
  table is one measure throughout (baseline 60.1 s, not 69.3 s, on that
  measure).
- **08:11, lifecycle lines as one byte** (a19edfc): 43.2 s to 40.6 s.
- **08:14, fetched bytes inline** (60b843c): 40.6 s to 38.6 s, with
  `tests/fetched_bytes.rs` for the spill path no manifest reaches.
- **08:17, one arm per opcode** (94bd13a): 37.8 s. Forcing the handlers
  inline on top of it was tried and reverted (39.0 s).
- **08:25, the host was never inlined** (dc1e28b). The re-profile showed
  `read_byte` and `handle_cpm_trap` as outlined functions: the binaries
  are separate crates. `#[inline]` on them: 37.8 s to 20.5 s. The baseline
  re-measured with the same attributes is 40.1 s, the core-only reference
  point.
- **08:29 and 08:33, flags as one byte** (653281c, 2a5eac5): 20.5 s to
  19.7 s; the second commit is neutral on ZEXALL and done for consistency.
- **08:35, rung 3 launched** on 2a5eac5 from a worktree
  (`/data/emu/z80-rust-fast-wt`), CPUs 10-31, 22 PyPy jobs, ZEXALL then
  ZEXDOC, so the dispatch and flag changes are proven at the lockstep rung
  while the build-profile work continues on the other cores.
- **08:37 to 08:50, build profile.** Fat LTO and one codegen unit
  (6fb0786): 19.7 s to 17.0 s. `target-cpu=native` measured and rejected
  (-8% to +2% by layout). 64-byte branch-target alignment made the default
  build (c5f04b5): 16.8 s quiet. `scripts/pgo.sh` added; the PGO build
  runs ZEXALL in 15.1 s. The measurement discipline had to tighten here:
  a sweep taken while builds and the rung 3 jobs ran read 20-26 s and was
  discarded; the recorded numbers were taken with the PyPy jobs paused
  (`SIGSTOP`) and nothing else running.
- **09:05, published.** On the user's word the private repository
  `alewman/z80-rust-fast` was created and `main` pushed at 2fb7c98. The
  first CI run passed all four jobs (fmt/clippy/test with the no_std
  build, rung 1 with rung 5, rung 2, rung 6) on the first try.
- **09:20, public.** Both Rust cores made public on the user's
  instruction after a scan for secrets and machine-local paths.
- **13:09, rung 3 ZEXALL identical** at 2a5eac5: 116 segments, every one
  `traces are identical`, 4 h 34 min with 22 PyPy processes confined to
  CPUs 10-31 (the earlier runs took about 7 h with 30 unconfined
  processes; the segment diffs run at the same speed on the fast core
  because the reference side is the bottleneck). ZEXDOC started 13:10.
- **Open when this record was written:** rung 3 ZEXDOC on 2a5eac5 and
  the wasm project, which is next and not started.
