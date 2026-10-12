//! Native SingleStepTests/z80 runner: rung 2 of the conformance ladder.
//!
//! Usage: `z80-vectors <corpus-dir> [--quiet]`
//!
//! Runs every `*.json` file in the directory (one per opcode or prefixed
//! variant, 1,000 cases each). For each case it loads `initial`, queues the
//! port reads from `ports`, calls `step()` once, and compares against
//! `final`: every register field vector_utils.py compares (a b c d e f h l
//! i r ix iy sp pc wz af_ bc_ de_ hl_ im iff1 iff2 q), RAM at the addresses
//! `final.ram` lists, port writes in order, every supplied port read
//! consumed, and `len(cycles)` against the T-states `step()` returned. The
//! generator's `ei` and `p` keys are ignored, as the reference runner does.
//!
//! It also checks the core against the access-timing table (`timing`), as
//! z80-python's tests/test_access_timing.py does, but on every case rather
//! than one per shape: the kinds of the accesses `step()` makes are the
//! table's for the bytes at PC, in order, and `(T-states, accesses made)` is
//! one of the table's shapes.
//!
//! Prints one line per failing case (up to a cap per file) and the same
//! FILES/TOTAL summary the reference gate prints. Exit status 0 only when
//! every case in every file passed.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Deserialize;
use z80_rust::timing::{instruction_timing, AccessKind};
use z80_rust::{Bus, Fault, Z80};

#[derive(Deserialize)]
struct Case {
    name: String,
    initial: State,
    #[serde(rename = "final")]
    expected: State,
    #[serde(default)]
    cycles: Vec<serde::de::IgnoredAny>,
    #[serde(default)]
    ports: Vec<(u16, u8, String)>,
}

#[derive(Deserialize)]
struct State {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    i: u8,
    r: u8,
    ix: u16,
    iy: u16,
    sp: u16,
    pc: u16,
    wz: u16,
    af_: u16,
    bc_: u16,
    de_: u16,
    hl_: u16,
    im: u8,
    iff1: u8,
    iff2: u8,
    q: u8,
    ram: Vec<(u16, u8)>,
}

/// Flat 64 KiB memory plus the vector's port script, like `VectorCPU`.
struct VectorBus {
    memory: Box<[u8; 0x10000]>,
    port_inputs: Vec<(u16, u8)>,
    port_input_next: usize,
    port_outputs: Vec<(u16, u8)>,
    port_errors: Vec<String>,
    /// The kind of every access, in order, for the access-timing check.
    kinds: Vec<AccessKind>,
}

impl Bus for VectorBus {
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.kinds.push(AccessKind::Read);
        self.memory[usize::from(addr)]
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        self.kinds.push(AccessKind::Write);
        self.memory[usize::from(addr)] = value;
    }

    fn read_port(&mut self, addr: u16) -> u8 {
        self.kinds.push(AccessKind::In);
        match self.port_inputs.get(self.port_input_next) {
            None => {
                self.port_errors.push(format!(
                    "unexpected I/O port read at 0x{addr:04X}: the vector provides no port input"
                ));
                0xFF
            }
            Some(&(expected_addr, value)) => {
                self.port_input_next += 1;
                if expected_addr != addr {
                    self.port_errors.push(format!(
                        "I/O port read address mismatch: expected 0x{expected_addr:04X} \
                         (vector order), CPU read 0x{addr:04X}"
                    ));
                }
                value
            }
        }
    }

    fn write_port(&mut self, addr: u16, value: u8) {
        self.kinds.push(AccessKind::Out);
        self.port_outputs.push((addr, value));
    }
}

fn setup_cpu(initial: &State, ports: &[(u16, u8, String)]) -> Z80<VectorBus> {
    let mut bus = VectorBus {
        memory: Box::new([0; 0x10000]),
        port_inputs: ports
            .iter()
            .filter(|(_, _, direction)| direction == "r")
            .map(|&(addr, value, _)| (addr, value))
            .collect(),
        port_input_next: 0,
        port_outputs: Vec::new(),
        port_errors: Vec::new(),
        kinds: Vec::new(),
    };
    for &(addr, value) in &initial.ram {
        bus.memory[usize::from(addr)] = value;
    }
    let mut cpu = Z80::new(bus);
    cpu.a = initial.a;
    cpu.b = initial.b;
    cpu.c = initial.c;
    cpu.d = initial.d;
    cpu.e = initial.e;
    cpu.f.set_byte(initial.f);
    cpu.h = initial.h;
    cpu.l = initial.l;
    cpu.i = initial.i;
    cpu.r = initial.r;
    cpu.ix = initial.ix;
    cpu.iy = initial.iy;
    cpu.sp = initial.sp;
    cpu.pc = initial.pc;
    cpu.wz = initial.wz;
    cpu.af_ = initial.af_;
    cpu.bc_ = initial.bc_;
    cpu.de_ = initial.de_;
    cpu.hl_ = initial.hl_;
    cpu.im = initial.im;
    cpu.iff1 = initial.iff1 != 0;
    cpu.iff2 = initial.iff2 != 0;
    cpu.q = initial.q;
    cpu
}

/// Run one case; return the list of differences (empty means pass).
fn run_case(case: &Case) -> Vec<String> {
    let mut cpu = setup_cpu(&case.initial, &case.ports);
    let mut diffs = Vec::new();
    // The instruction's bytes, as a host peeks them at PC before step().
    let peeked: Vec<u8> = (0..8u16)
        .map(|offset| cpu.bus.memory[usize::from(cpu.pc.wrapping_add(offset))])
        .collect();
    let timing = instruction_timing(&peeked);
    let t_states = match cpu.step() {
        Ok(t_states) => t_states,
        Err(fault @ Fault::UnhandledIndexOpcode { .. })
        | Err(fault @ Fault::UnhandledOpcode { .. }) => {
            diffs.push(format!("not implemented: {fault}"));
            return diffs;
        }
        Err(fault) => {
            diffs.push(format!("fault: {fault}"));
            return diffs;
        }
    };
    let expected = &case.expected;
    macro_rules! check {
        ($name:literal, $actual:expr, $expected:expr) => {
            if $actual != $expected {
                diffs.push(format!(
                    "{}: expected 0x{:X}, got 0x{:X}",
                    $name, $expected, $actual
                ));
            }
        };
    }
    check!("a", cpu.a, expected.a);
    check!("b", cpu.b, expected.b);
    check!("c", cpu.c, expected.c);
    check!("d", cpu.d, expected.d);
    check!("e", cpu.e, expected.e);
    check!("f", cpu.f.byte(), expected.f);
    check!("h", cpu.h, expected.h);
    check!("l", cpu.l, expected.l);
    check!("i", cpu.i, expected.i);
    check!("r", cpu.r, expected.r);
    check!("ix", cpu.ix, expected.ix);
    check!("iy", cpu.iy, expected.iy);
    check!("sp", cpu.sp, expected.sp);
    check!("pc", cpu.pc, expected.pc);
    check!("wz", cpu.wz, expected.wz);
    check!("af_", cpu.af_, expected.af_);
    check!("bc_", cpu.bc_, expected.bc_);
    check!("de_", cpu.de_, expected.de_);
    check!("hl_", cpu.hl_, expected.hl_);
    check!("im", cpu.im, expected.im);
    check!("iff1", u8::from(cpu.iff1), expected.iff1);
    check!("iff2", u8::from(cpu.iff2), expected.iff2);
    check!("q", cpu.q, expected.q);
    for &(addr, value) in &expected.ram {
        let actual = cpu.bus.memory[usize::from(addr)];
        if actual != value {
            diffs.push(format!(
                "ram[0x{addr:04X}]: expected 0x{value:02X}, got 0x{actual:02X}"
            ));
        }
    }
    let expected_writes: Vec<(u16, u8)> = case
        .ports
        .iter()
        .filter(|(_, _, direction)| direction == "w")
        .map(|&(addr, value, _)| (addr, value))
        .collect();
    if cpu.bus.port_outputs != expected_writes {
        diffs.push(format!(
            "port writes: expected {:?}, got {:?}",
            expected_writes, cpu.bus.port_outputs
        ));
    }
    if cpu.bus.port_input_next < cpu.bus.port_inputs.len() {
        let (addr, _) = cpu.bus.port_inputs[cpu.bus.port_input_next];
        diffs.push(format!(
            "missing I/O port read: the vector supplies a port input at 0x{addr:04X} \
             but the CPU never read it"
        ));
    }
    diffs.append(&mut cpu.bus.port_errors);
    let kinds = &cpu.bus.kinds;
    if !timing.shapes.contains(&(t_states, kinds.len())) {
        diffs.push(format!(
            "access timing: ({t_states} T-states, {} accesses) is not one of the table's shapes {:?}",
            kinds.len(),
            timing.shapes
        ));
    }
    let table_kinds: Vec<AccessKind> = timing.accesses.iter().map(|a| a.kind).collect();
    if !table_kinds.starts_with(kinds) {
        diffs.push(format!(
            "access timing: accesses {kinds:?} are not the table's {table_kinds:?} in order"
        ));
    }
    if !case.cycles.is_empty() {
        let expected_t = case.cycles.len() as u32;
        if t_states != expected_t {
            diffs.push(format!("t_states: expected {expected_t}, got {t_states}"));
        }
    }
    diffs
}

struct FileResult {
    passed: usize,
    failed: usize,
    not_implemented: usize,
}

fn run_file(path: &Path, quiet: bool) -> Result<FileResult, String> {
    let text = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let cases: Vec<Case> = serde_json::from_slice(&text)
        .map_err(|error| format!("{}: not a valid vector file: {error}", path.display()))?;
    if cases.is_empty() {
        return Err(format!("{}: no cases", path.display()));
    }
    let mut result = FileResult {
        passed: 0,
        failed: 0,
        not_implemented: 0,
    };
    let mut reported = 0;
    for case in &cases {
        let diffs = run_case(case);
        if diffs.is_empty() {
            result.passed += 1;
            continue;
        }
        if diffs.iter().any(|diff| diff.starts_with("not implemented")) {
            result.not_implemented += 1;
        } else {
            result.failed += 1;
        }
        if !quiet && reported < 5 {
            reported += 1;
            let mut line = format!("FAIL {} [{}]", path.display(), case.name);
            for diff in &diffs {
                let _ = write!(line, "\n    {diff}");
            }
            println!("{line}");
        }
    }
    Ok(result)
}

fn main() -> ExitCode {
    let mut quiet = false;
    let mut dir: Option<PathBuf> = None;
    for arg in std::env::args().skip(1) {
        if arg == "--quiet" {
            quiet = true;
        } else if dir.is_none() {
            dir = Some(PathBuf::from(arg));
        } else {
            eprintln!("usage: z80-vectors <corpus-dir> [--quiet]");
            return ExitCode::from(2);
        }
    }
    let Some(dir) = dir else {
        eprintln!("usage: z80-vectors <corpus-dir> [--quiet]");
        return ExitCode::from(2);
    };
    let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect(),
        Err(error) => {
            eprintln!("error: {}: {error}", dir.display());
            return ExitCode::from(2);
        }
    };
    files.sort();
    if files.is_empty() {
        eprintln!("error: no .json files in {}", dir.display());
        return ExitCode::from(2);
    }
    let started = std::time::Instant::now();
    let (mut files_passed, mut files_failed, mut files_skipped) = (0usize, 0usize, 0usize);
    let (mut passed, mut failed, mut not_implemented) = (0usize, 0usize, 0usize);
    for path in &files {
        match run_file(path, quiet) {
            Ok(result) => {
                passed += result.passed;
                failed += result.failed;
                not_implemented += result.not_implemented;
                if result.failed > 0 {
                    files_failed += 1;
                } else if result.not_implemented > 0 {
                    files_skipped += 1;
                } else {
                    files_passed += 1;
                }
            }
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::from(2);
            }
        }
    }
    let total = passed + failed + not_implemented;
    println!(
        "FILES: {files_passed} passed, {files_skipped} not implemented (skipped), {files_failed} failed"
    );
    println!(
        "TOTAL: {passed} passed, {failed} failed, {not_implemented} not implemented / {total} cases"
    );
    println!("elapsed: {:.1} s", started.elapsed().as_secs_f64());
    if failed == 0 && not_implemented == 0 && files_failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
