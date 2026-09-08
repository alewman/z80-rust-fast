//! Immutable values for capturing and restoring Z80 processor state.
//!
//! Transcribed from `z80_python/state.py`. The fields, in this order, are
//! the definition of "processor state" for conformance purposes
//! (`docs/trace-schema.md` in z80-python).

use alloc::string::{String, ToString};
use core::fmt::Write as _;

/// Complete CPU-owned state at an instruction boundary.
///
/// Registers, internal execution state, and pending lifecycle requests. It
/// deliberately excludes host memory, ports, devices, scheduling, and
/// counters, so restoring it restores the processor, not a complete machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuState {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub ix: u16,
    pub iy: u16,
    pub sp: u16,
    pub pc: u16,
    pub wz: u16,
    pub i: u8,
    pub r: u8,
    pub iff1: bool,
    pub iff2: bool,
    pub im: u8,
    pub af_alt: u16,
    pub bc_alt: u16,
    pub de_alt: u16,
    pub hl_alt: u16,
    pub q: u8,
    pub halted: bool,
    pub ei_delay: u8,
    pub reset_pending: bool,
    pub maskable_interrupt_vector: Option<u8>,
    pub non_maskable_interrupt_pending: bool,
}

/// Field names in `CPUState` declaration order (the order `z80-python` compares them).
///
/// There are 28; the reference docs say "29 fields" in two places, which is a
/// documentation miscount (see `dataclasses.fields(CPUState)`).
pub const FIELD_NAMES: [&str; 28] = [
    "a",
    "f",
    "b",
    "c",
    "d",
    "e",
    "h",
    "l",
    "ix",
    "iy",
    "sp",
    "pc",
    "wz",
    "i",
    "r",
    "iff1",
    "iff2",
    "im",
    "af_alt",
    "bc_alt",
    "de_alt",
    "hl_alt",
    "q",
    "halted",
    "ei_delay",
    "reset_pending",
    "maskable_interrupt_vector",
    "non_maskable_interrupt_pending",
];

impl CpuState {
    /// Validate the constraints `CPUState.__post_init__` enforces.
    pub fn validate(&self) -> Result<(), String> {
        if self.im > 2 {
            return Err("im must be 0, 1, or 2".to_string());
        }
        if self.ei_delay > 1 {
            return Err("ei_delay must be 0 or 1".to_string());
        }
        Ok(())
    }

    /// Append the state as a JSON object with sorted keys and no whitespace,
    /// the form `z80_python.trace.write_trace` produces.
    pub fn write_json(&self, out: &mut String) {
        let _ = write!(
            out,
            "{{\"a\":{},\"af_alt\":{},\"b\":{},\"bc_alt\":{},\"c\":{},\"d\":{},\"de_alt\":{},\"e\":{},\
             \"ei_delay\":{},\"f\":{},\"h\":{},\"halted\":{},\"hl_alt\":{},\"i\":{},\"iff1\":{},\
             \"iff2\":{},\"im\":{},\"ix\":{},\"iy\":{},\"l\":{},\"maskable_interrupt_vector\":{},\
             \"non_maskable_interrupt_pending\":{},\"pc\":{},\"q\":{},\"r\":{},\"reset_pending\":{},\
             \"sp\":{},\"wz\":{}}}",
            self.a,
            self.af_alt,
            self.b,
            self.bc_alt,
            self.c,
            self.d,
            self.de_alt,
            self.e,
            self.ei_delay,
            self.f,
            self.h,
            self.halted,
            self.hl_alt,
            self.i,
            self.iff1,
            self.iff2,
            self.im,
            self.ix,
            self.iy,
            self.l,
            match self.maskable_interrupt_vector {
                Some(vector) => vector.to_string(),
                None => "null".to_string(),
            },
            self.non_maskable_interrupt_pending,
            self.pc,
            self.q,
            self.r,
            self.reset_pending,
            self.sp,
            self.wz,
        );
    }
}
