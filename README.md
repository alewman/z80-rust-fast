# z80-rust-fast

A fast Rust Z80 instruction core, forked from
[z80-rust](https://github.com/alewman/z80-rust) and kept provably equivalent
to [z80-python](https://github.com/alewman/z80-python) at every processor
boundary: boundary kind, T-states, instruction bytes, and all 28 `CpuState`
fields.

z80-rust is a module-for-module transcription of the reference, deliberately
kept readable beside the Python and deliberately not fast. This repository
starts from that transcription (commit `43c5122`, the first commit here that
touches `src/`) and changes how the core does things without changing what
it does. The conformance ladder is the definition of "what it does": a
change that fails a rung is not an optimization, and every change is one
commit with the measurement that justifies it.

The library keeps the crate name `z80_rust`, so a host written for z80-rust
(`use z80_rust::{Bus, Z80}`) compiles against this crate unchanged. The
`Bus` trait, `Z80<B>`, `step()`, `capture_state()`/`restore_state()`, the
lifecycle requests, and `Fault` are the frozen surface; so are the trace
producer, the manifests, and the rung scripts.

## Certification

Pinned oracles, as `docs/conformance.md` in z80-python requires:

| What | Version |
| --- | --- |
| z80-python (reference core and conformance kit) | 0.4.0.dev0 at commit `cab1598` (the local checkout is `d1ec9c7`, whose instruction core is byte-identical: only `conformance.py` and `trace.py` differ) |
| Trace schema | version 1 |
| SingleStepTests/z80 corpus | revision `ebe1875d48f374bcfd4b505d8eb8ee751568b5f7` |
| raxoft/z80test | release 1.2a |
| FUSE Z80 core tests | release 1.6.0 (`fuse-1.6.0.tar.gz`, SHA-256 `3a8fedf2…047096`) |

Ladder status, each rung reproduced with the script named (`scripts/`) on
Linux x86_64 (i9-13900K) with rustc 1.93.1, CPython 3.14.4 and PyPy 7.3.20:

| Rung | What | Result | Last run at | Script |
| --- | --- | --- | --- | --- |
| 1 | The three example manifests diff clean against the reference | `traces are identical` ×3 | `52192d1` | `rung1.sh` |
| 2 | SingleStepTests, 1,604 files | `TOTAL: 1604000 passed, 0 failed, 0 not implemented / 1604000 cases` | `52192d1` | `rung2.sh` |
| 3 | ZEXALL and ZEXDOC in lockstep against the reference, 116 segments each | `every segment identical`, 5,764,169,474 records each | z80-rust `43c5122`; the instruction modules are unchanged since. Re-run before the first tag and after any change to dispatch or block instructions | `rung3.sh` |
| 4 | z80test natively: `z80full`, `z80ccf`, `z80memptr` | all three `Result: all tests passed.` | `52192d1` | `rung4.sh` |
| 5 | The ten interrupt scenarios as manifests with events | all ten `traces are identical` | `52192d1` | `rung5.sh` |
| 6 | FUSE 1.6.0's core test set, 1,356 cases, six explained divergences pinned | `1350 agree, 6 expected divergences, 0 unexpected` | `52192d1` | `rung6.sh` |

Rungs 1, 2, 4, 5, and 6 are re-run on every optimization commit; the commit
message carries the bench number before and after. CI
(`.github/workflows/ci.yml`) reproduces rungs 1, 2, 5, and 6 on every push,
builds the core without `std`, and runs `cargo fmt --check`, `cargo clippy
-D warnings`, and `cargo test`.

### Reproducing

```text
scripts/fetch_z80python.sh           # z80-python at cab1598 into external/, with a venv
scripts/fetch_test_vectors.sh        # SingleStepTests/z80 at the pinned revision into external/
scripts/rung1.sh
scripts/rung2.sh
# place zexall.com / zexdoc.com in conformance/zex/ (SHA-256 in conformance/README.md)
JOBS=30 scripts/rung3.sh zexall path/to/pypy3
JOBS=30 scripts/rung3.sh zexdoc path/to/pypy3
scripts/rung4.sh path/to/z80test-1.2a   # directory holding the .tap files
scripts/rung5.sh
scripts/fetch_fuse_tests.sh            # FUSE 1.6.0 z80/tests into external/
scripts/rung6.sh
scripts/bench.sh                       # the speed number, pinned to one core
target/release/z80-bench bench/ix-loop.json   # secondary workloads, see bench/README.md
```

## Speed

The measure is `z80-bench`: a plain `step()` loop over ZEXALL with the
`cpm-minimal` traps, no trace records and no state capture, 5,764,169,474
instructions and 46,734,975,782 T-states. `scripts/bench.sh` builds release
and runs it pinned to one logical CPU (`CPU`, default 4, a P-core on the
i9-13900K; the E-cores are about 1.45x slower on this loop and an unpinned
run may land on either).

**Code placement matters as much as small optimizations here.** Two builds
whose `step()` code was byte-identical differed by 9% (7.50 s against 8.17 s
per 1,000,000,000 instructions) because an unrelated function changed size
and moved the hot code 0x5e0 bytes. With LLVM's
`-align-all-nofallthru-blocks=5` or `=6` (branch-target blocks on 32- or
64-byte boundaries) the same two builds agree within 1-3%, in both
directions. So `scripts/bench.sh` builds three layouts, the default and
those two, runs each, and prints the minimum; the minimum over layouts is
the figure that says whether a change helped, and the default-build figure
is kept because it is what `cargo build --release` produces. Run-to-run
noise on one binary is about 0.2%. Since the alignment commit the default
build is the 64-byte-aligned layout and the sweep is `default`, `noalign`
(the flag overridden away, the layout LLVM picks on its own) and `nf5`.
Measurements taken while rung 3's 22 PyPy processes were running were
10-30% slower and are not in the table; the rows from `6fb0786` on were
taken with those processes paused.

| Commit | Change | Default build | Min over layouts | M instructions/s (min) |
| --- | --- | --- | --- | --- |
| `52192d1` | Baseline: the transcription as forked | 69.3 s | 60.1 s (nf6; 62.3 nf5) | 95.8 |
| `d27af51` | Main dispatch: one exhaustive `match` over the opcode byte, prefixes included, instead of the if-chain | 43.0 s | 43.0 s (default; 43.2 nf6, 45.8 nf5) | 134.0 |
| `5814b1f` | DD/FD dispatch: the same `match` for the byte after the prefix (`bench/ix-loop.json` 3.86 s to 2.99 s) | 43.3 s | 43.0 s (nf5; 43.6 nf6) | 134.1 |
| `4f0dbe7` | ED dispatch: the same `match` for the byte after ED (`bench/ed-loop.json` 2.27 s to 2.04 s) | 47.1 s | 43.2 s (nf6; 43.6 nf5) | 133.3 |
| `a19edfc` | Lifecycle requests as one byte: `step()` tests RESET, NMI, INT with one load and takes a cold path only when a line is up | 43.7 s | 40.6 s (nf6; 41.9 nf5) | 141.9 |
| `60b843c` | Fetched bytes in an inline 8-byte buffer; a `Vec` only for prefix runs longer than that | 38.6 s | 38.6 s (default; 40.3 nf6, 41.4 nf5) | 149.2 |
| `94bd13a` | Dispatch arms expanded to one per opcode with the opcode as a literal argument (`tools/expand_dispatch.py`) | 38.6 s | 37.8 s (nf5; 38.8 nf6) | 152.3 |
| `dc1e28b` | `#[inline]` on the conformance host's `Bus` methods and the CP/M trap check, which were real calls from the binaries' crate on every read and every step | 21.3 s | 20.5 s (nf6; 22.3 nf5) | 281.0 |
| `653281c` | 8-bit ALU and CB rotate flags composed as one byte from a compile-time S/Z/X/Y/parity table instead of five to seven setter calls | 20.0 s | 19.8 s (nf6 19.75, nf5 19.76) | 291.8 |
| `2a5eac5` | The remaining flag writers the same way: ADC/SBC HL, ADD HL/IX/IY, RLCA/RRCA/RLA/RRA, BIT, IN r,(C), RRD/RLD, LD A,I/R (no measurable ZEXALL change) | 20.5 s | 19.7 s (nf6 19.69, nf5 19.72) | 292.8 |
| `6fb0786` | Build profile: `lto = "fat"`, `codegen-units = 1` | 17.9 s | 17.0 s (nf6; 17.8 nf5) | 338.8 |
| next | 64-byte branch-target alignment becomes the default build (`.cargo/config.toml`); sweep is now default / noalign / nf5 | 16.8 s | 16.8 s (default; 17.0 nf5, 17.5 noalign) | 343.8 |
| (optional) | `scripts/pgo.sh`: profile-guided build trained on ZEXALL and the two loops, output under `target/pgo-use/` | 15.1 s | 15.1 s (one layout) | 382.8 |

### Profile of the baseline

`perf` is not available on the build machine (`perf_event_paranoid` = 4),
so `tools/sample-profile.py` samples the instruction pointer of the running
bench over ptrace at 2 ms and attributes each sample with `nm` and
`addr2line -i`. A full ZEXALL run under it (33,840 samples, 813 distinct
addresses) at `52192d1`:

| Self time | Innermost inlined frame | What it is |
| --- | --- | --- |
| 22.2% | `execute_main` | the if-chain over the unprefixed opcode |
| 12.8% | `handle_cpm_trap` | the two PC compares in the bench loop (inlined into `main`) |
| 10.8% | `step` | reset/NMI/INT checks, `halted`, `ei_delay` |
| 8.3% | `z80_bench::main` | the loop itself |
| 7.9% | `usize::from(u16)` | the memory index in `read_byte`, i.e. the load and its bounds check |
| 3.8% | `read_operand_byte` | |
| 3.8% | `ConformanceHost::read_byte` | |
| 2.6% | `op_ld_r_r` | |
| 2.4% | `decode_and_execute` | the prefix tests before `execute_main` |
| 2.1% | `Vec::push` | `note_fetched`, the per-byte trace bookkeeping |
| 2.0% | `can_accept_maskable_interrupt` | |
| 1.4% each | `pop_word`, `read_pair`, `fetch_byte`, `push_word`, `inc_r` | |

By outlined function, `step` holds 43.9% (nearly every handler is inlined
into it), `handle_cpm_trap` 12.8%, `read_byte` 11.1%, `main` 8.3%,
`read_operand_word` 5.5%, `op_ld_r_r` 2.9%, `alu_a` 2.1%. The flag setters
are individually small (`set_h` 0.6%, `set_z` 0.4%) but are spread over
every ALU handler.

What this says: dispatch shape first (the if-chain and the prefix tests are
a quarter of the time), then the per-step lifecycle checks, then the
fetched-bytes bookkeeping; the flag computation is diffuse and comes after.

### Optimizations

Each row above is one commit; its message states what the profile showed,
what changed, and the number before and after. Rungs 1, 2, 4, 5, and 6 were
re-run clean on each.

1. **Main dispatch as a jump table.** `execute_main` was the reference's
   if-chain, up to forty tests per instruction and 22% of self time; it is
   now one `match` over all 256 opcode bytes with no `_` arm, the CB/ED/
   DD/FD prefix tests folded in as arms, each arm calling the same handler
   with the same argument. `execute_cb` likewise. 69.3 s to 43.0 s in the
   default build; 60.1 s to 43.0 s with layouts controlled.
2. **DD/FD dispatch as a jump table.** `execute_index_opcode` was a
   sixty-test if-chain; now one exhaustive `match`. ZEXALL has too few
   prefixed instructions to move (43.2 s, within noise), so the change is
   measured on `bench/ix-loop.json`, a loop of five IX instructions and a
   JR: 3.86 s to 2.99 s, 78 to 100 M instructions/s.
3. **ED dispatch as a jump table.** `execute_ed` was a `match` on the
   block instructions followed by an if-chain; now one `match` whose `_`
   arm is the reference's ED no-op rule. `bench/ed-loop.json` (NEG, ADC
   HL,BC, LD A,I, LD A,R, JR): 2.27 s to 2.04 s. The default build of
   ZEXALL got 9% slower (43.3 s to 47.1 s) and this is the change that
   exposed the layout sensitivity described above: `step()` was
   byte-identical and had moved; layout-controlled, 43.2 s against the
   previous commit's 43.0 s.
4. **Lifecycle lines as one byte.** `step()` began with three tests
   (`reset_pending`, `non_maskable_interrupt_pending`, `iff1 && ei_delay
   == 0 && pending_maskable_interrupt.is_some()`), 11% of self time with
   `can_accept_maskable_interrupt` in the profile. The three requests are
   now bits of one `pending_lines` byte plus the INT vector byte; `step()`
   tests the byte and calls a `#[cold]` function that applies the
   reference's priority order only when something is pending.
   `capture_state`/`restore_state` and the request/clear API translate, so
   the 28 `CpuState` fields are unchanged. 43.2 s to 40.6 s.
5. **Fetched bytes inline.** Every fetched byte was pushed to a `Vec`
   (2% in `Vec::push`, plus the `clear` per instruction). The bytes now go
   to an 8-byte array with a length; only a DD/FD run longer than eight
   bytes moves to a `Vec`, on a `#[cold]` path. `last_instruction_bytes()`
   returns the same slice either way, and `tests/fetched_bytes.rs` covers
   the spill, since no manifest records an instruction longer than five
   bytes. 40.6 s to 38.6 s.
6. **One arm per opcode.** The grouped arms (`0x40..=0x75 | 0x77..=0x7F
   => self.op_ld_r_r(opcode)`) share one body, so the handler decodes the
   register fields at run time. `tools/expand_dispatch.py` rewrote every
   table so each opcode has its own arm with the opcode as a literal
   (`0x41 => self.op_ld_r_r(0x41)`), the mapping otherwise untouched.
   38.6 s to 37.8 s; `ix-loop` 2.99 s to 2.42 s, `ed-loop` 2.04 s to
   about 1.8 s. The larger gain needs the handlers inlined so the literal
   folds, which is the next change.
7. **Forcing the handlers inline was a regression (not committed).**
   `#[inline(always)]` on the 28 opcode-parameterized handlers so the
   literal would fold: `execute_main` grew to 5.8 KB and was no longer
   inlined into `step`, and ZEXALL went from 37.8 s to 39.0 s, `ix-loop`
   from 2.42 s to 2.76 s. Reverted; recorded so it is not tried again the
   same way.
8. **Inline the host.** The re-profile at `94bd13a` showed
   `ConformanceHost::read_byte` (14.5%), `handle_cpm_trap` (13%) and
   `write_byte` (3.5%) as *outlined* functions: they are non-generic
   functions in the library crate, and the bench, `z80-trace`, and the
   rung binaries are separate crates that cannot inline them without a
   hint. Every memory read and every step was a call. `#[inline]` on the
   four `Bus` methods and on the trap test (its BDOS body moved to a
   `#[cold]` function) changes nothing the host does. 37.8 s to 20.5 s;
   `ix-loop` 2.42 s to 1.77 s, `ed-loop` 1.8 s to 1.4 s; `z80-trace
   --no-trace` over ZEXALL 151 s (z80-rust) to 128 s. A host in its own
   crate never paid this, so the gain is real for the kit's binaries and
   for the numbers in this table, but not for every embedding. For a
   core-only comparison the baseline `52192d1` was re-measured with the
   same two attributes applied: 47.7 s default, **40.1 s** min over
   layouts (nf6; 42.1 nf5). Against that, the core changes alone are
   40.1 s to 20.5 s at this row.
9. **Flags as one byte.** `add`, `sub`, `cp`, `and`, `or`, `xor`, `inc`,
   `dec` and the eight CB rotates/shifts each wrote F through five to
   seven `Flags` setters (read-modify-write each, plus a three-step parity
   fold). Each now builds F in one expression: a `const` table `SZP[256]`
   holds S, Z, X, Y and parity for every result byte, and C, H, N and
   overflow are ORed in. Every primitive still writes exactly the flags
   the setter sequence wrote and preserves the rest (`inc`/`dec` keep C).
   Rung 2's 1,604,000 vectors and z80full are the check. 20.5 s to 19.8 s.
10. **The rest of the flag writers.** `add16`/`sub16`, `ADD HL,rr` and
    `ADD IX/IY,rr` (which preserve S, Z, PV), the accumulator rotates
    (same), `BIT` (preserves C), `IN r,(C)`, `RRD`/`RLD`, and `LD A,I`/
    `LD A,R` (preserve C) composed the same way. ZEXALL barely runs them:
    19.75 s to 19.69 s, within noise. Done for consistency, so every F
    write in the core is one assignment with its preserved bits named.
11. **Fat LTO and one codegen unit.** The first build-profile change, after
    the source-level work so each earlier number stands on the default
    profile. 19.7 s to 17.0 s.
12. **64-byte branch-target alignment as the default build.**
    `.cargo/config.toml` now sets `-C llvm-args=-align-all-nofallthru-blocks=6`,
    the layout that was best or equal-best on every commit above, so
    `cargo build --release` gives the measured layout rather than a lucky or
    unlucky one. Code size grows about 6%. `scripts/bench.sh` now sweeps
    `default` (aligned), `noalign` (the flag overridden away) and `nf5`.
    Quiet measurement (rung 3's PyPy processes paused, nothing else
    running): aligned default 16.8 s, `noalign` 17.5 s, `nf5` 17.0 s.
13. **`target-cpu=native` was not adopted.** On a 1,500,000,000-instruction
    slice with LTO: 8% slower than the default without alignment, 2% faster
    with it, i.e. inside the layout noise, and it would make the binaries
    non-portable. Not used.
14. **Profile-guided optimization, as an optional build.** `scripts/pgo.sh`
    instruments, trains on ZEXALL and the two secondary loops, merges the
    profile with the toolchain's `llvm-profdata`, and rebuilds into
    `target/pgo-use/`. Same quiet conditions: **15.1 s** (15.06, 15.11), 382 M
    instructions/s, a 3.1 GHz Z80; `ix-loop` 1.53 s to 0.98 s, `ed-loop`
    1.21 s to 0.86 s. The default build does not use it, because
    a committed profile is tied to one compiler version.

## Using the core

```rust
use z80_rust::{Bus, Z80};

struct Flat([u8; 0x10000]);

impl Bus for Flat {
    fn read_byte(&mut self, addr: u16) -> u8 { self.0[usize::from(addr)] }
    fn write_byte(&mut self, addr: u16, value: u8) { self.0[usize::from(addr)] = value; }
    fn read_port(&mut self, _addr: u16) -> u8 { 0xFF }
    fn write_port(&mut self, _addr: u16, _value: u8) {}
}

let mut cpu = Z80::new(Flat([0; 0x10000]));
cpu.bus.0[..2].copy_from_slice(&[0x3E, 0x2A]); // LD A,0x2A
let t_states = cpu.step().unwrap();
assert_eq!((t_states, cpu.a), (7, 0x2A));
```

`step()` services an asserted RESET, a latched NMI, or an acceptable
maskable interrupt before fetching, exactly as the reference's `step()`
does; `request_*` / `clear_*` are the lifecycle API; `capture_state()` and
`restore_state()` move the complete processor state. `step()` returns
`Err(Fault)` in exactly the situation where the reference raises
`NotImplementedError` (IM 0 with a non-RST vector byte) and leaves the
state as the reference leaves it.

### Features

- `std` (default): the trace producer (`trace`), the conformance kit
  (`conformance`), serde, the binaries, and the rung 1 test.
- Without it (`cargo build --no-default-features`) the crate is `no_std`
  plus `alloc` and contains the instruction core alone, for the WebAssembly
  build that follows this project.

### Binaries

- `z80-bench [manifest] [--limit N] [--show-output]`: the speed measure above.
- `z80-trace <manifest>`: run a conformance manifest and stream its trace
  (`docs/trace-schema.md` in z80-python) to stdout or `--out`;
  `--checkpoint-every N` splits a long run into resumable segments.
- `z80-vectors <dir>`: the SingleStepTests runner (rung 2).
- `z80-z80test <tap>...`: the z80test runner (rung 4).
- `z80-fuse <dir>`: the FUSE core-test runner (rung 6).

## Stated limitations

Inherited from the reference and z80-rust, unchanged:

- **IM 0 accepts only the eight RST opcodes.** Both parent cores stop with
  an error for a non-RST byte and leave the request pending; no oracle in
  the ladder exercises it, and supporting it changes the trace schema.
- Bus-level timing, memory contention, WAIT states, interrupt-acknowledge
  callbacks, and daisy chains are outside every core's claims.

## Where the brief is

[docs/handoff-brief.md](docs/handoff-brief.md) is the prompt this
repository is built from, verbatim, and [docs/build-record.md](docs/build-record.md)
is what happened when it was run. The transcription's own brief and record
are [docs/z80-rust-handoff-brief.md](docs/z80-rust-handoff-brief.md) and
[docs/z80-rust-build-record.md](docs/z80-rust-build-record.md).

## License

MIT, like both parents. The SingleStepTests corpus (MIT), the ZEX
exercisers (GPL-2.0), z80test (MIT), and FUSE's tests (GPL-2.0) are
external and not bundled.
