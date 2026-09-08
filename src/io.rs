//! Input/output instruction implementation.
//!
//! Transcribed from `z80_python/_io.py`.

use crate::core::{Bus, Z80};
use crate::flags::{Flags, FLAG_C, SZP};

impl<B: Bus> Z80<B> {
    pub(crate) fn in_flags(&mut self, value: u8) {
        // C is preserved; N = H = 0; S, Z, X, Y and parity from the byte.
        self.f = Flags::new((self.f.byte() & FLAG_C) | SZP[usize::from(value)]);
    }

    /// IN A,(n) -- port address is A:n.
    pub(crate) fn op_in_a_n(&mut self) -> u32 {
        let low = self.read_operand_byte();
        self.wz = (u16::from(self.a) << 8) | u16::from(low);
        self.a = self.bus.read_port(self.wz);
        self.wz = self.wz.wrapping_add(1);
        self.update_q(false);
        11
    }

    /// OUT (n),A -- port address is A:n.
    pub(crate) fn op_out_n_a(&mut self) -> u32 {
        let low = self.read_operand_byte();
        self.wz = (u16::from(self.a) << 8) | u16::from(low);
        self.bus.write_port(self.wz, self.a);
        // Same latch behavior as LD (nn),A: the port's high byte (A) is kept and only
        // the low byte of the address increments.
        self.wz = (self.wz & 0xFF00) | u16::from(self.wz.wrapping_add(1) as u8);
        self.update_q(false);
        11
    }

    /// IN r,(C) -- includes the undocumented IN (C) / IN F,(C) form that only sets flags.
    pub(crate) fn op_in_r_c(&mut self, opcode: u8) -> u32 {
        let dest = (opcode >> 3) & 0x07;
        self.wz = self.bc().wrapping_add(1);
        let value = self.bus.read_port(self.bc());
        self.in_flags(value);
        if dest != 6 {
            self.write_reg(dest, value);
        }
        self.update_q(true);
        12
    }

    /// OUT (C),r -- the undocumented OUT (C),0 form writes zero on NMOS parts.
    pub(crate) fn op_out_c_r(&mut self, opcode: u8) -> u32 {
        let src = (opcode >> 3) & 0x07;
        let addr = self.bc();
        // OUT (C),0: the undocumented (HL) slot has no register, NMOS parts drive 0
        // (CMOS parts drive 0xFF; this core models NMOS, as the vectors do).
        let value = if src == 6 { 0 } else { self.read_reg(src) };
        self.bus.write_port(addr, value);
        self.wz = addr.wrapping_add(1);
        self.update_q(false);
        12
    }

    pub(crate) fn block_ini(&mut self, increment: bool) -> bool {
        let (data, adj) = if increment {
            self.wz = self.bc().wrapping_add(1);
            let data = self.bus.read_port(self.wz.wrapping_sub(1));
            (data, self.c.wrapping_add(1))
        } else {
            self.wz = self.bc().wrapping_sub(1);
            let data = self.bus.read_port(self.wz.wrapping_add(1));
            (data, self.c.wrapping_sub(1))
        };
        self.io_data = data;
        self.b = self.b.wrapping_sub(1);
        self.bus.write_byte(self.hl(), data);
        let hl = if increment {
            self.hl().wrapping_add(1)
        } else {
            self.hl().wrapping_sub(1)
        };
        self.h = (hl >> 8) as u8;
        self.l = hl as u8;
        let sum = u16::from(adj) + u16::from(data);
        let carry = if sum > 0xFF { 1 } else { 0 };
        self.f.set_c(carry);
        // Block I/O flags (Undocumented Z80 Documented, 4.3): N = bit 7 of the byte,
        // H = C = carry of (C+/-1) + byte for INI/IND (L + byte for OUTI/OUTD), and
        // PV = parity of ((that sum & 7) ^ B). S/Z/X/Y come from the new B.
        self.f.set_n((data >> 7) & 1);
        self.set_parity(((sum & 7) as u8) ^ self.b);
        self.set_xysz(self.b);
        self.f.set_h(carry);
        self.b != 0
    }

    pub(crate) fn block_outi(&mut self, increment: bool) -> bool {
        let hl = self.hl();
        let data = self.bus.read_byte(hl);
        self.io_data = data;
        let hl = if increment {
            hl.wrapping_add(1)
        } else {
            hl.wrapping_sub(1)
        };
        self.h = (hl >> 8) as u8;
        self.l = hl as u8;
        self.b = self.b.wrapping_sub(1);
        let addr = self.bc();
        self.bus.write_port(addr, data);
        self.wz = if increment {
            addr.wrapping_add(1)
        } else {
            addr.wrapping_sub(1)
        };
        let sum = u16::from(self.l) + u16::from(data);
        let carry = if sum > 0xFF { 1 } else { 0 };
        self.f.set_c(carry);
        // Block I/O flags (Undocumented Z80 Documented, 4.3): N = bit 7 of the byte,
        // H = C = carry of (C+/-1) + byte for INI/IND (L + byte for OUTI/OUTD), and
        // PV = parity of ((that sum & 7) ^ B). S/Z/X/Y come from the new B.
        self.f.set_n((data >> 7) & 1);
        self.set_parity(((sum & 7) as u8) ^ self.b);
        self.set_xysz(self.b);
        self.f.set_h(carry);
        self.b != 0
    }

    // Repeating block I/O adjusts flags once more after the PC rewind. This is
    // newer than Young's 2005 text and is encoded by the SingleStepTests
    // generator: X/Y from PC high byte bits 3/5, and when C is set, PV and H
    // are corrected from B+/-1 depending on bit 7 of the byte moved.
    pub(crate) fn post_in_o_r(&mut self) {
        self.f.set_x(((self.pc >> 11) & 1) as u8);
        self.f.set_y(((self.pc >> 13) & 1) as u8);
        if self.f.c() != 0 {
            if self.io_data & 0x80 != 0 {
                let pv = self.f.pv() ^ (1 - Self::parity(self.b.wrapping_sub(1) & 7));
                self.f.set_pv(pv);
                self.f.set_h(if (self.b & 0x0F) == 0 { 1 } else { 0 });
            } else {
                let pv = self.f.pv() ^ (1 - Self::parity(self.b.wrapping_add(1) & 7));
                self.f.set_pv(pv);
                self.f.set_h(if (self.b & 0x0F) == 0x0F { 1 } else { 0 });
            }
        } else {
            let pv = self.f.pv() ^ (1 - Self::parity(self.b & 7));
            self.f.set_pv(pv);
        }
    }

    /// INI -- (HL) <- port BC; HL++, B--.
    pub(crate) fn op_ini(&mut self) -> u32 {
        self.block_ini(true);
        self.update_q(true);
        16
    }

    /// IND -- (HL) <- port BC; HL--, B--.
    pub(crate) fn op_ind(&mut self) -> u32 {
        self.block_ini(false);
        self.update_q(true);
        16
    }

    /// OUTI -- B--, then port BC <- (HL); HL++.
    pub(crate) fn op_outi(&mut self) -> u32 {
        self.block_outi(true);
        self.update_q(true);
        16
    }

    /// OUTD -- B--, then port BC <- (HL); HL--.
    pub(crate) fn op_outd(&mut self) -> u32 {
        self.block_outi(false);
        self.update_q(true);
        16
    }

    /// INIR -- INI repeated while B != 0; 21 T-states per repeat, 16 on the last.
    pub(crate) fn op_inir(&mut self) -> u32 {
        let repeat = self.block_ini(true);
        if repeat {
            self.block_repeat();
            self.post_in_o_r();
        }
        self.update_q(true);
        if repeat {
            21
        } else {
            16
        }
    }

    /// INDR -- IND repeated while B != 0; 21 T-states per repeat, 16 on the last.
    pub(crate) fn op_indr(&mut self) -> u32 {
        let repeat = self.block_ini(false);
        if repeat {
            self.block_repeat();
            self.post_in_o_r();
        }
        self.update_q(true);
        if repeat {
            21
        } else {
            16
        }
    }

    /// OTIR -- OUTI repeated while B != 0; 21 T-states per repeat, 16 on the last.
    pub(crate) fn op_otir(&mut self) -> u32 {
        let repeat = self.block_outi(true);
        if repeat {
            self.block_repeat();
            self.post_in_o_r();
        }
        self.update_q(true);
        if repeat {
            21
        } else {
            16
        }
    }

    /// OTDR -- OUTD repeated while B != 0; 21 T-states per repeat, 16 on the last.
    pub(crate) fn op_otdr(&mut self) -> u32 {
        let repeat = self.block_outi(false);
        if repeat {
            self.block_repeat();
            self.post_in_o_r();
        }
        self.update_q(true);
        if repeat {
            21
        } else {
            16
        }
    }
}
