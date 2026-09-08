//! Eight- and sixteen-bit arithmetic instruction implementation.
//!
//! Transcribed from `z80_python/_alu.py`.

use crate::core::{Bus, Z80};
use crate::flags::{Flags, FLAG_C, FLAG_H, FLAG_N, FLAG_PV, FLAG_S, FLAG_X, FLAG_Y, FLAG_Z, SZP};

impl<B: Bus> Z80<B> {
    pub(crate) fn set_sz(&mut self, value: u8) {
        self.f.set_s((value >> 7) & 1);
        self.f.set_z(if value == 0 { 1 } else { 0 });
    }

    pub(crate) fn set_xysz(&mut self, value: u8) {
        self.f.set_xy(value);
        self.set_sz(value);
    }

    pub(crate) fn parity(value: u8) -> u8 {
        let mut value = value;
        value ^= value >> 4;
        value ^= value >> 2;
        value ^= value >> 1;
        if value & 1 != 0 {
            0
        } else {
            1
        }
    }

    pub(crate) fn set_parity(&mut self, value: u8) {
        self.f.set_pv(Self::parity(value));
    }

    // The eight-bit primitives write F as one byte: the result-only flags
    // (S, Z, X, Y, and parity where PV is parity) come from the SZP table
    // and the rest are computed bits. Each writes exactly the flags the
    // reference's setter sequence wrote and preserves the ones it did not.

    pub(crate) fn add(&mut self, x: u8, y: u8, carry: u8) -> u8 {
        let wide = u16::from(x) + u16::from(y) + u16::from(carry);
        let z = wide as u8;
        // All eight flags: C from bit 8, N = 0, PV = overflow, H = half carry.
        let f = (SZP[usize::from(z)] & !FLAG_PV)
            | ((wide >> 8) as u8 & FLAG_C)
            | (((x ^ y ^ 0xFF) & (x ^ z) & 0x80) >> 5)
            | ((x ^ y ^ z) & FLAG_H);
        self.f = Flags::new(f);
        z
    }

    pub(crate) fn sub(&mut self, x: u8, y: u8, carry: u8) -> u8 {
        let wide = (u16::from(x))
            .wrapping_sub(u16::from(y))
            .wrapping_sub(u16::from(carry));
        let z = wide as u8;
        // All eight flags: C = borrow (bit 8 of the 9-bit difference), N = 1.
        let f = (SZP[usize::from(z)] & !FLAG_PV)
            | ((wide >> 8) as u8 & FLAG_C)
            | FLAG_N
            | (((x ^ y) & (x ^ z) & 0x80) >> 5)
            | ((x ^ y ^ z) & FLAG_H);
        self.f = Flags::new(f);
        z
    }

    pub(crate) fn and(&mut self, x: u8, y: u8) -> u8 {
        let z = x & y;
        // All eight flags: C = N = 0, H = 1, PV = parity.
        self.f = Flags::new(SZP[usize::from(z)] | FLAG_H);
        z
    }

    pub(crate) fn or(&mut self, x: u8, y: u8) -> u8 {
        let z = x | y;
        // All eight flags: C = N = H = 0, PV = parity.
        self.f = Flags::new(SZP[usize::from(z)]);
        z
    }

    pub(crate) fn xor(&mut self, x: u8, y: u8) -> u8 {
        let z = x ^ y;
        // All eight flags: C = N = H = 0, PV = parity.
        self.f = Flags::new(SZP[usize::from(z)]);
        z
    }

    pub(crate) fn cp(&mut self, x: u8, y: u8) {
        let wide = (u16::from(x)).wrapping_sub(u16::from(y));
        let z = wide as u8;
        // All eight flags, like sub, except that CP is a discarded SUB: X/Y
        // sample the operand still on the internal bus, not the result.
        let f = (SZP[usize::from(z)] & !(FLAG_PV | FLAG_X | FLAG_Y))
            | (y & (FLAG_X | FLAG_Y))
            | ((wide >> 8) as u8 & FLAG_C)
            | FLAG_N
            | (((x ^ y) & (x ^ z) & 0x80) >> 5)
            | ((x ^ y ^ z) & FLAG_H);
        self.f = Flags::new(f);
    }

    pub(crate) fn inc(&mut self, x: u8) -> u8 {
        let z = x.wrapping_add(1);
        // C is preserved; N = 0; PV = overflow into 0x80; H = low nibble carried.
        let f = (self.f.byte() & FLAG_C)
            | (SZP[usize::from(z)] & !FLAG_PV)
            | if z == 0x80 { FLAG_PV } else { 0 }
            | if (z & 0x0F) == 0 { FLAG_H } else { 0 };
        self.f = Flags::new(f);
        z
    }

    pub(crate) fn dec(&mut self, x: u8) -> u8 {
        let z = x.wrapping_sub(1);
        // C is preserved; N = 1; PV = overflow from 0x80; H = low nibble borrowed.
        let f = (self.f.byte() & FLAG_C)
            | (SZP[usize::from(z)] & !FLAG_PV)
            | FLAG_N
            | if z == 0x7F { FLAG_PV } else { 0 }
            | if (z & 0x0F) == 0x0F { FLAG_H } else { 0 };
        self.f = Flags::new(f);
        z
    }

    /// ADD/ADC/SUB/SBC/AND/XOR/OR/CP A,r -- 8-bit ALU with a register or (HL) operand.
    pub(crate) fn op_alu_r(&mut self, opcode: u8) -> u32 {
        let group = (opcode >> 3) & 0x07;
        let src = opcode & 0x07;
        let (value, t_states) = if src == 6 {
            (self.bus.read_byte(self.hl()), 7)
        } else {
            (self.read_reg(src), 4)
        };
        self.alu_a(group, value);
        self.update_q(true);
        t_states
    }

    /// ADD/ADC/SUB/SBC/AND/XOR/OR/CP A,n -- 8-bit ALU with an immediate operand.
    pub(crate) fn op_alu_n(&mut self, opcode: u8) -> u32 {
        let group = match opcode {
            0xC6 => 0,
            0xCE => 1,
            0xD6 => 2,
            0xDE => 3,
            0xE6 => 4,
            0xEE => 5,
            0xF6 => 6,
            0xFE => 7,
            _ => unreachable!("op_alu_n called with a non-ALU opcode"),
        };
        let value = self.read_operand_byte();
        self.alu_a(group, value);
        self.update_q(true);
        7
    }

    /// NEG -- replace A with its two's-complement negation.
    pub(crate) fn op_neg(&mut self) -> u32 {
        self.a = self.sub(0, self.a, 0);
        self.update_q(true);
        8
    }

    pub(crate) fn alu_a(&mut self, group: u8, value: u8) {
        if group == 0 {
            self.a = self.add(self.a, value, 0);
        } else if group == 1 {
            self.a = self.add(self.a, value, self.f.c());
        } else if group == 2 {
            self.a = self.sub(self.a, value, 0);
        } else if group == 3 {
            self.a = self.sub(self.a, value, self.f.c());
        } else if group == 4 {
            self.a = self.and(self.a, value);
        } else if group == 5 {
            self.a = self.xor(self.a, value);
        } else if group == 6 {
            self.a = self.or(self.a, value);
        } else {
            self.cp(self.a, value);
        }
    }

    /// INC r
    pub(crate) fn op_inc_r(&mut self, opcode: u8) -> u32 {
        let dest = (opcode >> 3) & 0x07;
        let value = self.inc(self.read_reg(dest));
        self.write_reg(dest, value);
        self.update_q(true);
        4
    }

    /// DEC r
    pub(crate) fn op_dec_r(&mut self, opcode: u8) -> u32 {
        let dest = (opcode >> 3) & 0x07;
        let value = self.dec(self.read_reg(dest));
        self.write_reg(dest, value);
        self.update_q(true);
        4
    }

    /// INC (HL)
    pub(crate) fn op_inc_hl(&mut self) -> u32 {
        let addr = self.hl();
        let value = self.bus.read_byte(addr);
        let value = self.inc(value);
        self.bus.write_byte(addr, value);
        self.update_q(true);
        11
    }

    /// DEC (HL)
    pub(crate) fn op_dec_hl(&mut self) -> u32 {
        let addr = self.hl();
        let value = self.bus.read_byte(addr);
        let value = self.dec(value);
        self.bus.write_byte(addr, value);
        self.update_q(true);
        11
    }

    /// DAA -- decimal-adjust A after a BCD ADD/SUB, steered by H, N, and C.
    pub(crate) fn op_daa(&mut self) -> u32 {
        let original = self.a;
        // DAA: a tens digit above 9, or a carry out of it, means the BCD result
        // overflowed 99; correct by 0x60 (direction from N) and force C. The
        // second test does the same for the units digit via H and 0x06.
        if self.f.c() != 0 || self.a > 0x99 {
            self.a = if self.f.n() != 0 {
                self.a.wrapping_sub(0x60)
            } else {
                self.a.wrapping_add(0x60)
            };
            self.f.set_c(1);
        }
        if self.f.h() != 0 || (self.a & 0x0F) > 0x09 {
            self.a = if self.f.n() != 0 {
                self.a.wrapping_sub(0x06)
            } else {
                self.a.wrapping_add(0x06)
            };
        }
        self.set_parity(self.a);
        self.set_xysz(self.a);
        self.f.set_h(((self.a ^ original) & 0x10) >> 4);
        self.update_q(true);
        4
    }

    /// CPL -- complement A, setting H/N and X/Y from the result.
    pub(crate) fn op_cpl(&mut self) -> u32 {
        self.a ^= 0xFF;
        self.f.set_h(1);
        self.f.set_n(1);
        self.f.set_xy(self.a);
        self.update_q(true);
        4
    }

    /// SCF/CCF -- including their Q-sensitive undocumented X/Y behavior.
    pub(crate) fn op_scf_ccf(&mut self, opcode: u8, prefixed: bool) -> u32 {
        // Q holds F only if the previous M1 cycle wrote flags. If it did, F's X/Y
        // are masked and A alone supplies them; otherwise X/Y = (F | A). A DD/FD
        // prefix is its own M1 that writes no flags, so a prefixed SCF/CCF sees Q=0.
        if self.q != 0 && !prefixed {
            self.f.set_xy(0);
        }
        let old_carry = self.f.c();
        if opcode == 0x37 {
            self.f.set_c(1);
            self.f.set_h(0);
        } else {
            self.f.set_c(old_carry ^ 1);
            self.f.set_h(old_carry);
        }
        self.f.set_n(0);
        self.f.set_xy(self.f.byte() | self.a);
        self.update_q(true);
        4
    }

    // ADC/SBC HL,rr write all eight flags: C from bit 16, H from bit 12,
    // PV = 16-bit overflow, S/X/Y from the high byte, Z from the whole word.
    pub(crate) fn add16(&mut self, x: u16, y: u16, carry: u8) -> u16 {
        let wide = u32::from(x) + u32::from(y) + u32::from(carry);
        let result = wide as u16;
        let high = (result >> 8) as u8;
        let f = (high & (FLAG_S | FLAG_X | FLAG_Y))
            | if result == 0 { FLAG_Z } else { 0 }
            | (((x ^ y ^ result) >> 8) as u8 & FLAG_H)
            | ((((x ^ y ^ 0xFFFF) & (x ^ result)) >> 13) as u8 & FLAG_PV)
            | ((wide >> 16) as u8 & FLAG_C);
        self.f = Flags::new(f);
        result
    }

    pub(crate) fn sub16(&mut self, x: u16, y: u16, carry: u8) -> u16 {
        let wide = u32::from(x)
            .wrapping_sub(u32::from(y))
            .wrapping_sub(u32::from(carry));
        let result = wide as u16;
        let high = (result >> 8) as u8;
        let f = (high & (FLAG_S | FLAG_X | FLAG_Y))
            | if result == 0 { FLAG_Z } else { 0 }
            | (((x ^ y ^ result) >> 8) as u8 & FLAG_H)
            | ((((x ^ y) & (x ^ result)) >> 13) as u8 & FLAG_PV)
            | ((wide >> 16) as u8 & FLAG_C)
            | FLAG_N;
        self.f = Flags::new(f);
        result
    }

    /// ADD HL,rr -- only H, N, C and X/Y change; S/Z/PV are preserved.
    pub(crate) fn op_add_hl_rr(&mut self, opcode: u8) -> u32 {
        let pair_index = (opcode >> 4) & 0x03;
        let hl = self.hl();
        let value = self.read_pair(pair_index);
        // 16-bit adds run through the address latch: WZ = HL + 1 (the high-byte pass).
        self.wz = hl.wrapping_add(1);
        let wide = u32::from(hl) + u32::from(value);
        let result = wide as u16;
        // Only H, N, C and X/Y change; S, Z and PV are preserved.
        let f = (self.f.byte() & (FLAG_S | FLAG_Z | FLAG_PV))
            | ((result >> 8) as u8 & (FLAG_X | FLAG_Y))
            | (((hl ^ value ^ result) >> 8) as u8 & FLAG_H)
            | ((wide >> 16) as u8 & FLAG_C);
        self.f = Flags::new(f);
        self.write_pair(2, result);
        self.update_q(true);
        11
    }

    /// ADC HL,rr
    pub(crate) fn op_adc_hl_rr(&mut self, opcode: u8) -> u32 {
        let pair_index = (opcode >> 4) & 0x03;
        let hl = self.hl();
        // 16-bit adds run through the address latch: WZ = HL + 1 (the high-byte pass).
        self.wz = hl.wrapping_add(1);
        let result = self.add16(hl, self.read_pair(pair_index), self.f.c());
        self.write_pair(2, result);
        self.update_q(true);
        15
    }

    /// SBC HL,rr
    pub(crate) fn op_sbc_hl_rr(&mut self, opcode: u8) -> u32 {
        let pair_index = (opcode >> 4) & 0x03;
        let hl = self.hl();
        // 16-bit adds run through the address latch: WZ = HL + 1 (the high-byte pass).
        self.wz = hl.wrapping_add(1);
        let result = self.sub16(hl, self.read_pair(pair_index), self.f.c());
        self.write_pair(2, result);
        self.update_q(true);
        15
    }

    /// INC rr -- no flags.
    pub(crate) fn op_inc_rr(&mut self, opcode: u8) -> u32 {
        let pair_index = (opcode >> 4) & 0x03;
        self.write_pair(pair_index, self.read_pair(pair_index).wrapping_add(1));
        self.update_q(false);
        6
    }

    /// DEC rr -- no flags.
    pub(crate) fn op_dec_rr(&mut self, opcode: u8) -> u32 {
        let pair_index = (opcode >> 4) & 0x03;
        self.write_pair(pair_index, self.read_pair(pair_index).wrapping_sub(1));
        self.update_q(false);
        6
    }
}
