# The brief that produced this repository

This is the prompt the repository was built from, verbatim. It was handed
to a Claude Code session (Claude Fable 5.1) started in `/data/emu`, beside
the z80-python clone, on 2026-09-06. The only instructions the user added
during the session are listed after it, and [build-record.md](build-record.md)
says what happened. It is kept here because the brief is the specification
the code was written to, and because it shows how much of the result the
reference's own documentation carried: the brief points at five documents
and the source, in order, and asks for a transcription rather than a design.

---

# Handoff: start `z80-rust`, a Rust Z80 core proven equivalent to z80-python

## Context you are inheriting

`z80-python` (github alewman/z80-python, local clone at `/data/emu/z80-python`, currently at commit `530cad3` on `main`) is a pure-Python Z80 instruction core whose identity is "the code is the spec": readable, oracle-verified, embeddable. It is certified under CPython 3.14 and PyPy 7.3.20 against SingleStepTests (1,604,000 cases including T-states), ZEXDOC/ZEXALL, raxoft/z80test (real-silicon flag vectors), and an interrupt cross-check. It now ships a conformance kit so a core in another language can be proven equivalent to it after every instruction. Read these, in this order, before writing any Rust:

1. `docs/start-here.md` — register file, F byte, opcode bit fields, prefix model, WZ/Q/R.
2. `docs/conformance.md` — what "the same CPU" means, the manifest, the one host both cores must implement, the certification ladder.
3. `docs/trace-schema.md` — the JSON Lines record your core must emit.
4. `docs/z80-undocumented-behavior.md` — mechanisms first, rules with source locations.
5. `src/z80_python/` — the reference itself, module by module. Every handler docstring starts with its Zilog mnemonic; hardware reasons are comments on the lines that encode them.

## Your task

Create the repository `alewman/z80-rust` (the user chose this name; the crate name on crates.io is decided later and may differ) and build a Rust Z80 core that passes the conformance ladder in `docs/conformance.md`, in order. The goal is **behavioral equivalence with z80-python at every processor boundary**: kind, T-states, instruction bytes, and all 29 `CPUState` fields, not "passes ZEX at the end".

### Milestones, each with its acceptance test

1. **Skeleton + trace producer.** A core that steps one boundary at a time and emits the trace schema (omit `mnemonic`/`operands`; give `address` and every byte). A host implementing the `flat` and `cpm-minimal` profiles exactly as `docs/conformance.md` specifies (the BDOS traps act outside the step and produce no record). A manifest loader for the JSON form. Acceptance: `python -m z80_python.conformance diff examples/conformance/flags-and-branches.json yours.jsonl` and the same for `interrupts.json` both print "traces are identical".
2. **Native SingleStepTests runner.** Register/RAM/port-order/T-state (`len(cycles)`) comparison across all 1,604 files. The corpus is already fetched at `/data/emu/z80-python/tests/z80_test_vectors/v1/` (pinned rev `ebe1875d…`); the JSON shape is documented in `docs/start-here.md` (ignore keys `ei` and `p`). Acceptance: 1,604,000 / 1,604,000.
3. **ZEX lockstep.** A `cpm-minimal` manifest for `zexall.com` (binary at `/data/emu/z80-python/tests/zex/`, load at 0x0100, SP 0xF000, word 0x00F0 at address 6) diffed in lockstep against the reference through a pipe. Acceptance: identical for the full run, or the first divergence reported and fixed until it is.
4. **z80test natively** (`/data/emu/z80-python/validation/z80test_data/*.tap`; see `validation/z80test_runner.py` for the two ROM stubs). Acceptance: `z80full`, `z80ccf`, `z80memptr` all report "all tests passed".
5. **Interrupt scenarios.** Translate the ten scenarios in `validation/interrupt_crosscheck.py` into manifests with `events` and diff them. If you find these belong in z80-python as shipped manifests, propose that as a z80-python PR rather than keeping them only in z80-rust.

Do not skip a rung or reorder them; each one's failure is cheapest to diagnose before the next.

## Constraints

- **Do not modify z80-python's instruction core.** If you believe the reference is wrong, stop, show the manifest and divergence, and ask; the vectors and hardware suites are the authority, not either implementation.
- Install z80-python from git at commit `530cad3` (it is not on PyPI at this revision): `pip install "git+https://github.com/alewman/z80-python@530cad3"`, or use the local checkout's `.venv/bin/python` / `.venv-pypy/bin/pypy3`. PyPy is 5-15x faster for anything that runs the reference at length.
- Mirror z80-python's module layout in Rust (`alu`, `blocks`, `control`, `core`, `dispatch`, `index`, `io`, `loads`, `rotate`) so a reader can hold the two side by side. Transcribe; do not redesign. Keep the explicit dispatch shape.
- Same discipline the Python project uses: every claim pinned to a commit hash and reproduced; state exactly what was and was not run; "the whole suite" means every rung, so do not say it when external suites were skipped. Report failures with their output.
- Commit in reviewable units with clear messages. Push and open PRs only when the user asks. Attribution trailer on commits: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Cargo and rustc are installed at `/usr/bin`; `gh` is authenticated. No sudo.

## What done looks like

A `z80-rust` README that states, with the four numbers `docs/conformance.md` requires (z80-python version/commit, trace schema version, SingleStepTests revision, z80test release), which rungs of the ladder pass, and a CI job that reproduces rungs 1 and 2 on every push.

Before starting, tell the user in a few sentences how you read this brief and what you will do first.

---

## Instructions added during the session

1. After the session stated its reading of the brief: "lets keep it local
   commits only for now. We can push later. You have a good take on this.
   Please start."
2. Once every rung had passed: create the GitHub repository under the
   user's own account, open the two z80-python PRs the session had
   proposed (the interrupt manifests and the 28-field doc fix), and report
   how long a ZEXALL run takes on the core.
3. Keep this brief in the repository.
