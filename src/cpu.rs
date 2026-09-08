//! Public Z80 CPU API: `step`, state capture/restore, and the lifecycle requests.
//!
//! Transcribed from `z80_python/cpu.py`.

use crate::core::{Bus, Fault, LINE_INT, LINE_NMI, LINE_RESET, Z80};
use crate::state::CpuState;

impl<B: Bus> Z80<B> {
    /// Advance one instruction boundary and return its documented T-state count.
    ///
    /// An asserted RESET takes priority over NMI and an accepted maskable
    /// interrupt; each is serviced before instruction fetch. A halted CPU
    /// consumes a four-T-state idle cycle until an accepted interrupt wakes
    /// it. T-states are instruction/lifecycle totals, not externally
    /// observable bus cycles.
    pub fn step(&mut self) -> Result<u32, Fault> {
        // One byte test covers RESET, NMI, and INT; the priority order and
        // the acceptance rule live in service_pending_lines.
        if self.pending_lines != 0 {
            if let Some(t_states) = self.service_pending_lines()? {
                return Ok(t_states);
            }
        }

        let delay_was_active = self.ei_delay > 0;
        let t_states = if self.halted {
            self.inc_r();
            self.update_q(false);
            4
        } else {
            self.decode_and_execute()?
        };
        if delay_was_active {
            self.ei_delay -= 1;
        }
        Ok(t_states)
    }

    /// Return a snapshot of all CPU-owned execution state.
    ///
    /// Host memory, ports, devices, scheduling, and counters are not included.
    pub fn capture_state(&self) -> CpuState {
        CpuState {
            a: self.a,
            f: self.f.byte(),
            b: self.b,
            c: self.c,
            d: self.d,
            e: self.e,
            h: self.h,
            l: self.l,
            ix: self.ix,
            iy: self.iy,
            sp: self.sp,
            pc: self.pc,
            wz: self.wz,
            i: self.i,
            r: self.r,
            iff1: self.iff1,
            iff2: self.iff2,
            im: self.im,
            af_alt: self.af_,
            bc_alt: self.bc_,
            de_alt: self.de_,
            hl_alt: self.hl_,
            q: self.q,
            halted: self.halted,
            ei_delay: self.ei_delay,
            reset_pending: self.pending_lines & LINE_RESET != 0,
            maskable_interrupt_vector: if self.pending_lines & LINE_INT != 0 {
                Some(self.int_vector)
            } else {
                None
            },
            non_maskable_interrupt_pending: self.pending_lines & LINE_NMI != 0,
        }
    }

    /// Restore a previously captured CPU state without touching the host.
    pub fn restore_state(&mut self, state: &CpuState) {
        self.a = state.a;
        self.f.set_byte(state.f);
        self.b = state.b;
        self.c = state.c;
        self.d = state.d;
        self.e = state.e;
        self.h = state.h;
        self.l = state.l;
        self.ix = state.ix;
        self.iy = state.iy;
        self.sp = state.sp;
        self.pc = state.pc;
        self.wz = state.wz;
        self.i = state.i;
        self.r = state.r;
        self.iff1 = state.iff1;
        self.iff2 = state.iff2;
        self.im = state.im;
        self.af_ = state.af_alt;
        self.bc_ = state.bc_alt;
        self.de_ = state.de_alt;
        self.hl_ = state.hl_alt;
        self.q = state.q;
        self.halted = state.halted;
        self.ei_delay = state.ei_delay;
        self.pending_lines = 0;
        if state.reset_pending {
            self.pending_lines |= LINE_RESET;
        }
        if let Some(vector) = state.maskable_interrupt_vector {
            self.pending_lines |= LINE_INT;
            self.int_vector = vector;
        }
        if state.non_maskable_interrupt_pending {
            self.pending_lines |= LINE_NMI;
        }
    }

    /// Whether the host has asserted RESET.
    pub fn reset_pending(&self) -> bool {
        self.pending_lines & LINE_RESET != 0
    }

    /// Assert RESET for servicing at the next instruction boundary.
    pub fn request_reset(&mut self) {
        self.pending_lines |= LINE_RESET;
    }

    /// Release the host-controlled RESET line.
    pub fn clear_reset(&mut self) {
        self.pending_lines &= !LINE_RESET;
    }

    /// Whether a device has requested a maskable interrupt not yet accepted.
    pub fn maskable_interrupt_pending(&self) -> bool {
        self.pending_lines & LINE_INT != 0
    }

    /// Assert the maskable-interrupt request line between instruction boundaries.
    pub fn request_maskable_interrupt(&mut self, vector_byte: u8) {
        self.pending_lines |= LINE_INT;
        self.int_vector = vector_byte;
    }

    /// Deassert a previously requested but not-yet-accepted interrupt.
    pub fn clear_maskable_interrupt(&mut self) {
        self.pending_lines &= !LINE_INT;
    }

    /// Whether a device has requested a non-maskable interrupt not yet accepted.
    pub fn non_maskable_interrupt_pending(&self) -> bool {
        self.pending_lines & LINE_NMI != 0
    }

    /// Latch an NMI request for service at the next instruction boundary.
    pub fn request_non_maskable_interrupt(&mut self) {
        self.pending_lines |= LINE_NMI;
    }

    /// Cancel a requested NMI that has not yet reached an instruction boundary.
    pub fn clear_non_maskable_interrupt(&mut self) {
        self.pending_lines &= !LINE_NMI;
    }
}
