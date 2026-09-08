//! Speed benchmark: a plain `step()` loop over a manifest (ZEXALL by
//! default) with the `cpm-minimal` traps, no trace records and no state
//! capture. This is the loop z80-rust's README "Speed" section describes,
//! and the number every optimization in this repository is measured by.
//!
//! Usage: `z80-bench [manifest.json] [--limit <instructions>] [--show-output]`
//!
//! Prints one line to stdout: instructions, T-states, wall time,
//! instructions per second, the equivalent Z80 clock (T-states per second),
//! and why the run stopped. `--show-output` writes the CP/M console output
//! the program produced to stderr, so a ZEX run can be seen to have finished
//! with every test reporting OK. Exit status 2 on a bad manifest or an
//! unsupported BDOS call, 1 if the core faulted.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use z80_rust::conformance::{handle_cpm_trap, load_manifest, ConformanceHost, HostProfile};
use z80_rust::Z80;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut manifest_path = PathBuf::from("conformance/zex/zexall.json");
    let mut limit: Option<u64> = None;
    let mut show_output = false;
    while let Some(arg) = args.next() {
        if arg == "--limit" {
            limit = args.next().and_then(|value| value.parse().ok());
            if limit.is_none() {
                eprintln!("error: --limit requires an instruction count");
                return ExitCode::from(2);
            }
        } else if arg == "--show-output" {
            show_output = true;
        } else if arg.starts_with("--") {
            eprintln!("usage: z80-bench [manifest.json] [--limit <instructions>] [--show-output]");
            return ExitCode::from(2);
        } else {
            manifest_path = PathBuf::from(arg);
        }
    }
    let manifest = match load_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };

    let mut cpu = Z80::new(ConformanceHost::new(&manifest));
    cpu.restore_state(&manifest.initial);
    let cpm = manifest.host == HostProfile::CpmMinimal;
    let stop_on_halt = manifest.stop.on_halt;
    let budget = limit.unwrap_or(u64::MAX).min(manifest.stop.max_steps);
    let mut instructions: u64 = 0;
    let mut t_states: u64 = 0;

    let start = Instant::now();
    let reason = loop {
        if instructions == budget {
            break if limit.is_some() {
                "limit"
            } else {
                "max_steps"
            };
        }
        if cpm {
            match handle_cpm_trap(&mut cpu) {
                Ok(true) => break "cpm_exit",
                Ok(false) => {}
                Err(error) => {
                    eprintln!("error: {error}");
                    return ExitCode::from(2);
                }
            }
        }
        if stop_on_halt && cpu.halted {
            break "halted";
        }
        match cpu.step() {
            Ok(t) => {
                t_states += u64::from(t);
                instructions += 1;
            }
            Err(fault) => {
                eprintln!("fault after {instructions} instructions: {fault}");
                return ExitCode::from(1);
            }
        }
    };
    let elapsed = start.elapsed().as_secs_f64();

    if show_output {
        eprintln!("{}", String::from_utf8_lossy(&cpu.bus.output));
    }
    println!(
        "{}: {} instructions, {} T-states, {:.3} s, {:.2} M instructions/s, {:.1} MHz, stopped on {}",
        manifest.name,
        instructions,
        t_states,
        elapsed,
        instructions as f64 / elapsed / 1e6,
        t_states as f64 / elapsed / 1e6,
        reason
    );
    ExitCode::SUCCESS
}
