# Build record

What happened when [handoff-brief.md](handoff-brief.md) was run, in the
order it happened, with the numbers. Timestamps are local (PDT) and come
from the commit log and the run logs.

## 2026-09-06

- **17:30 to 18:30, reading.** The five documents and every module of
  `z80_python/` in the order the brief lists, plus `cpu.py`, `trace.py`,
  `conformance.py`, `debug.py`, and the three validation runners. One thing
  in the reference did not match its docs: `CPUState` has 28 fields, the
  docs say 29 (reported upstream as z80-python#3).
- **18:37, core transcribed** (commit 31c4f1b): thirteen modules
  mirroring the Python ones, the `Bus` trait standing in for the abstract
  host class, `Fault` standing in for `NotImplementedError`, and the core
  remembering the bytes each instruction consumed so a trace needs no
  disassembler. It compiled on the second attempt (the first failed only
  on the 29-versus-28 array length).
- **18:37, rung 1** (e37a8f0): the trace producer, the manifest loader and
  the two host profiles. Both example manifests diffed identical on the
  first run.
- **18:41, rung 2** (5b7c251): the SingleStepTests runner. 1,604,000 of
  1,604,000 cases on the first run, 44 s cold, 9 s warm. A debug build with
  overflow checks passed the same corpus later.
- **18:43, rung 4** (465541e): the z80test runner. `z80full`, `z80ccf`,
  `z80memptr` all reported "all tests passed" on the first run; the only
  fix on the way was splitting the Spectrum's `\r`-terminated output to
  find the "Result:" line.
- **18:46, checkpoints** (eff4ec4). ZEXALL turned out to be 5,764,169,474
  boundary records, and the reference diff consumes tens of thousands of
  records per second, so a single pipe would have taken days. `z80-trace`
  gained `--checkpoint-every N`, which writes a manifest resuming the run
  every N records (full memory image plus every state field), and
  `scripts/rung3.sh` diffs the segments in parallel. Each segment starts
  from the state the previous one ended in, so a divergence anywhere is
  reported by the segment holding it. Verified first on the example
  manifest split into four segments.
- **18:52 to 01:47, rung 3, ZEXALL.** 116 segments of 50,000,000 records,
  30 PyPy 7.3.20 processes at a time on an i9-13900K that was also running
  other people's jobs: 6 h 55 min, every segment identical.

## 2026-09-07

- **01:48, rung 5.** All ten interrupt manifests identical on the first
  run. README committed with the ladder (f954a8b).
- **01:50 to 08:59, rung 3, ZEXDOC.** Same shape as ZEXALL, same record
  and T-state counts, 7 h 11 min, every segment identical (69bf23b).
- **14:07, first push and CI.** The repository was created and pushed on
  the user's instruction. GitHub's clippy (1.98) flagged one
  `needless_late_init` the local 1.90 had not; the interrupt-accept
  if-chain now yields its T-state total directly, and rungs 2 and 5 were
  re-run clean (26955e7). CI then passed rung 1 against a fresh install of
  z80-python and rung 2 against the corpus fetched at the pinned revision.
- **14:23, upstream.** z80-python#3 (the ten manifests with reference
  traces, and the 28-field fix) and z80-python#4 (the reference's own CI
  workflow had failed to parse since a commit before this work; two lines
  were mis-indented). Both green.
- **Speed.** A plain `step()` loop runs ZEXALL in 116 s, about 50 million
  instructions per second.

## 2026-09-07, second piece: prefix runs

- The user asked which gaps remained. Two were real behavior: runs of
  DD/FD prefixes (and DD/FD before ED), which both cores refused, and IM 0
  with a non-RST byte. The first was closed in the reference
  (z80-python#5, merged as `cab1598`) from Sean Young's *The Undocumented
  Z80 Documented* v0.91 with the evidence tier stated, then transcribed
  here. Rungs 1 (now three manifests), 2, 4, and 5 re-run clean at
  `cab1598`, and the ZEX lockstep re-run at `cab1598` finished on
  2026-09-08 at 05:52: ZEXALL and ZEXDOC every segment identical, 6 h 58 min
  and 6 h 43 min. The second gap changes the trace schema and waits for a
  decision.
- One transcription detail: the bytes an instruction occupies became a
  vector, because a prefix run has no length bound and the trace reader
  rejected a five-byte `DD DD 21 34 12` cut to four.

## 2026-09-07, third piece: FUSE

- FUSE 1.6.0's Z80 core test set (1,356 emulator-derived cases) was run
  against the reference first: 1,350 agree, and each of the six that do not
  is explained by a hardware-derived source the reference follows
  (z80-python#6). The same runner transcribed here is rung 6; it requires
  exactly those six to diverge. CI now runs rungs 1, 2, 5, and 6.

## What was not needed

No divergence from the reference was found at any rung, so neither core
was changed for behavior. The only edits after the transcription were the
lint above, the `\r` split in the z80test runner, and tooling.
