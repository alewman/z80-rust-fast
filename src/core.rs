//! CPU state, fetch, stack, and register-selection helpers.
//!
//! Transcribed from `z80_python/_core.py`. The Python core is an abstract
//! class whose host subclass supplies memory and ports; here the host is a
//! [`Bus`] value owned by the [`Z80`] struct, and every instruction module
//! is an `impl<B: Bus> Z80<B>` block, one file per Python mixin.

use alloc::vec::Vec;

use crate::flags::Flags;

/// Memory and I/O supplied by the host. Addresses are already 16-bit and
/// values 8-bit, so a host never has to mask.
pub trait Bus {
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, value: u8);
    fn read_port(&mut self, addr: u16) -> u8;
    fn write_port(&mut self, addr: u16, value: u8);
}

/// The conditions under which the reference core raises instead of executing.
///
/// `z80-python` raises `NotImplementedError` for these and leaves the state
/// exactly as it was at the moment of the raise; this core does the same and
/// returns the error, so a run that hits one ends where the reference's would.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// A DD/FD prefix followed by a byte the index dispatch table does not
    /// handle. Unreachable in practice: runs of DD/FD and DD/FD before ED are
    /// consumed before the table, and the table covers every other byte.
    UnhandledIndexOpcode { prefix: u8, opcode: u8, pc: u16 },
    /// An unprefixed opcode the main dispatcher does not handle. Unreachable
    /// in practice: every one of the 256 is handled.
    UnhandledOpcode { opcode: u8, pc: u16 },
    /// IM 0 acceptance with a device byte that is not an RST opcode. The
    /// request stays pending.
    Im0NonRstVector(u8),
}

impl core::fmt::Display for Fault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Fault::UnhandledIndexOpcode { prefix, opcode, pc } => write!(
                f,
                "unhandled {prefix:02X}-prefixed opcode 0x{opcode:02X} at PC 0x{pc:04X}"
            ),
            Fault::UnhandledOpcode { opcode, pc } => {
                write!(f, "unhandled opcode 0x{opcode:02X} at PC 0x{pc:04X}")
            }
            Fault::Im0NonRstVector(vector) => write!(
                f,
                "IM 0 currently supports only device-supplied RST opcodes (got 0x{vector:02X})"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Fault {}

/// Z80 instruction core with memory and I/O supplied by the host `B`.
///
/// A newly constructed CPU has zeroed processor state, exactly like
/// `Z80CPU.__init__`.
pub struct Z80<B: Bus> {
    pub bus: B,
    pub a: u8,
    pub f: Flags,
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
    pub af_: u16,
    pub bc_: u16,
    pub de_: u16,
    pub hl_: u16,
    pub q: u8,
    pub(crate) io_data: u8,
    pub halted: bool,
    pub(crate) ei_delay: u8,
    pub ei_nmi_iff2_erratum: bool,
    pub(crate) reset_pending: bool,
    pub(crate) pending_maskable_interrupt: Option<u8>,
    pub(crate) non_maskable_interrupt_pending: bool,
    /// Not processor state: the bytes the current instruction has consumed
    /// from PC (opcode, prefixes, operands), so a trace can report exactly
    /// the bytes the instruction occupied without a disassembler. A run of
    /// DD/FD prefixes has no length bound, so this is a vector; it is
    /// cleared, not reallocated, per instruction.
    pub(crate) fetched: Vec<u8>,
}

impl<B: Bus> Z80<B> {
    pub fn new(bus: B) -> Self {
        Z80 {
            bus,
            a: 0,
            f: Flags::new(0),
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            ix: 0,
            iy: 0,
            sp: 0,
            pc: 0,
            wz: 0,
            i: 0,
            r: 0,
            iff1: false,
            iff2: false,
            im: 0,
            af_: 0,
            bc_: 0,
            de_: 0,
            hl_: 0,
            q: 0,
            io_data: 0,
            halted: false,
            ei_delay: 0,
            ei_nmi_iff2_erratum: false,
            reset_pending: false,
            pending_maskable_interrupt: None,
            non_maskable_interrupt_pending: false,
            fetched: Vec::with_capacity(8),
        }
    }

    /// The bytes the most recent instruction occupied, in address order.
    pub fn last_instruction_bytes(&self) -> &[u8] {
        &self.fetched
    }

    pub(crate) fn note_fetched(&mut self, value: u8) {
        self.fetched.push(value);
    }

    pub(crate) fn inc_r(&mut self) {
        // R counts M1 (opcode fetch) cycles in its low 7 bits; bit 7 is only ever
        // written by LD R,A and is preserved across the increment.
        self.r = (self.r & 0x80) | (self.r.wrapping_add(1) & 0x7F);
    }

    pub(crate) fn update_q(&mut self, flags_modified: bool) {
        // Q mirrors the ALU's last flag output: the new F when this instruction wrote
        // flags, else 0. Only SCF/CCF read it (undocumented X/Y). Q=0 from writing
        // F=0 is indistinguishable from 'not written', harmlessly: F's X/Y are 0 too.
        self.q = if flags_modified { self.f.byte() } else { 0 };
    }

    // An opcode or prefix fetch is an M1 cycle and bumps R; operand and
    // displacement bytes (read_operand_byte) are ordinary reads and do not.
    // That is why DD CB d op advances R by 2 (two prefixes) and not 4.
    pub(crate) fn fetch_byte(&mut self) -> u8 {
        let value = self.bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        self.inc_r();
        self.note_fetched(value);
        value
    }

    pub(crate) fn read_operand_byte(&mut self) -> u8 {
        let value = self.bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        self.note_fetched(value);
        value
    }

    pub(crate) fn read_operand_word(&mut self) -> u16 {
        let low = self.read_operand_byte();
        let high = self.read_operand_byte();
        (u16::from(high) << 8) | u16::from(low)
    }

    pub(crate) fn push_word(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, (value >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, value as u8);
    }

    pub(crate) fn pop_word(&mut self) -> u16 {
        let low = self.bus.read_byte(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let high = self.bus.read_byte(self.sp);
        self.sp = self.sp.wrapping_add(1);
        (u16::from(high) << 8) | u16::from(low)
    }

    pub(crate) fn can_accept_maskable_interrupt(&self) -> bool {
        self.iff1 && self.ei_delay == 0 && self.pending_maskable_interrupt.is_some()
    }

    /// Apply the RESET-visible CPU state while the host holds RESET asserted.
    pub(crate) fn accept_reset(&mut self) -> u32 {
        debug_assert!(self.reset_pending, "RESET is not asserted");
        self.pc = 0;
        self.im = 0;
        self.iff1 = false;
        self.iff2 = false;
        self.halted = false;
        self.ei_delay = 0;
        self.update_q(false);
        3
    }

    /// Enter a pending NMI at an instruction boundary.
    pub(crate) fn accept_non_maskable_interrupt(&mut self) -> u32 {
        debug_assert!(
            self.non_maskable_interrupt_pending,
            "no non-maskable interrupt is pending"
        );
        self.non_maskable_interrupt_pending = false;
        self.halted = false;
        if self.ei_nmi_iff2_erratum && self.ei_delay > 0 {
            // Opt-in NMOS quirk: an NMI landing inside EI's one-instruction
            // delay window resets IFF2 as well as IFF1, instead of IFF2
            // preserving the pre-NMI IFF1 value for a later RETN. See
            // docs/interrupt-lifecycle.md in z80-python.
            self.iff2 = false;
        } else {
            self.iff2 = self.iff1;
        }
        self.iff1 = false;
        self.inc_r();
        self.push_word(self.pc);
        self.wz = 0x0066;
        self.pc = self.wz;
        self.update_q(false);
        11
    }

    /// Enter a pending maskable interrupt at an instruction boundary.
    pub(crate) fn accept_maskable_interrupt(&mut self) -> Result<u32, Fault> {
        let vector_byte = self
            .pending_maskable_interrupt
            .expect("no maskable interrupt is pending");
        debug_assert!(
            self.iff1 && self.ei_delay == 0,
            "maskable interrupts cannot currently be accepted"
        );
        if self.im == 0 && (vector_byte & 0xC7) != 0xC7 {
            return Err(Fault::Im0NonRstVector(vector_byte));
        }

        self.pending_maskable_interrupt = None;
        self.halted = false;
        self.iff1 = false;
        self.iff2 = false;
        self.inc_r();
        self.push_word(self.pc);
        let t_states = if self.im == 0 {
            self.wz = u16::from(vector_byte & 0x38);
            self.pc = self.wz;
            13
        } else if self.im == 1 {
            self.wz = 0x0038;
            self.pc = self.wz;
            13
        } else {
            let vector_address = (u16::from(self.i) << 8) | u16::from(vector_byte);
            let low = self.bus.read_byte(vector_address);
            let high = self.bus.read_byte(vector_address.wrapping_add(1));
            self.wz = (u16::from(high) << 8) | u16::from(low);
            self.pc = self.wz;
            19
        };
        self.update_q(false);
        Ok(t_states)
    }

    pub(crate) fn hl(&self) -> u16 {
        (u16::from(self.h) << 8) | u16::from(self.l)
    }

    pub(crate) fn bc(&self) -> u16 {
        (u16::from(self.b) << 8) | u16::from(self.c)
    }

    pub(crate) fn de(&self) -> u16 {
        (u16::from(self.d) << 8) | u16::from(self.e)
    }

    pub(crate) fn read_pair(&self, index: u8) -> u16 {
        if index == 0 {
            return self.bc();
        }
        if index == 1 {
            return self.de();
        }
        if index == 2 {
            return self.hl();
        }
        self.sp
    }

    pub(crate) fn write_pair(&mut self, index: u8, value: u16) {
        if index == 0 {
            self.b = (value >> 8) as u8;
            self.c = value as u8;
        } else if index == 1 {
            self.d = (value >> 8) as u8;
            self.e = value as u8;
        } else if index == 2 {
            self.h = (value >> 8) as u8;
            self.l = value as u8;
        } else {
            self.sp = value;
        }
    }

    pub(crate) fn read_reg(&self, index: u8) -> u8 {
        match index {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            7 => self.a,
            _ => unreachable!("register index 6 ((HL)) has no direct register backing"),
        }
    }

    pub(crate) fn write_reg(&mut self, index: u8, value: u8) {
        match index {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.h = value,
            5 => self.l = value,
            7 => self.a = value,
            _ => unreachable!("register index 6 ((HL)) has no direct register backing"),
        }
    }
}
