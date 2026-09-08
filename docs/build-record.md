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
