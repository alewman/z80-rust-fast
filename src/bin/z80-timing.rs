//! Answer `instruction_timing` for byte strings on stdin: rung 7.
//!
//! Usage: `z80-timing < inputs`
//!
//! Each input line is an instruction's bytes in hex (`ddcb0006`); each
//! output line is that line, a tab, and the answer in the form
//! `scripts/timing_reference.py` prints for the reference: the accesses as
//! `kind@t` joined by spaces, ` | `, and the shapes as `t_states/made`.

use std::io::{self, BufRead, BufWriter, Write};

use z80_rust::timing::instruction_timing;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut out = BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line?;
        let hex = line.trim();
        if hex.is_empty() {
            continue;
        }
        let data: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex bytes"))
            .collect();
        let timing = instruction_timing(&data);
        let accesses: Vec<String> = timing
            .accesses
            .iter()
            .map(|a| format!("{}@{}", a.kind.name(), a.t))
            .collect();
        let shapes: Vec<String> = timing
            .shapes
            .iter()
            .map(|(t_states, made)| format!("{t_states}/{made}"))
            .collect();
        writeln!(out, "{hex}\t{} | {}", accesses.join(" "), shapes.join(" "))?;
    }
    out.flush()
}
