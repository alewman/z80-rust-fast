//! Rotate, shift, and bit-operation implementation.
//!
//! Transcribed from `z80_python/_rotate.py`. The rotates keep the reference's
//! shift-and-or form rather than `rotate_left`, so the two read the same.
#![allow(clippy::manual_rotate)]

use crate::core::{Bus, Z80};
use crate::flags::{Flags, FLAG_C, FLAG_H, FLAG_PV, FLAG_S, FLAG_X, FLAG_Y, FLAG_Z, SZP};

impl<B: Bus> Z80<B> {
    // The CB rotates and shifts write all eight flags: C is the bit shifted
    // out, N = H = 0, PV = parity of the result, S/Z/X/Y from the result.
    fn rot_flags(&mut self, result: u8, carry: u8) {
        self.f = Flags::new(SZP[usize::from(result)] | (carry & FLAG_C));
    }

    pub(crate) fn rlc(&mut self, x: u8) -> u8 {
        let x = (x << 1) | (x >> 7);
        self.rot_flags(x, x & 1);
        x
    }

    pub(crate) fn rrc(&mut self, x: u8) -> u8 {
        let x = (x >> 1) | (x << 7);
        self.rot_flags(x, (x >> 7) & 1);
        x
    }

    pub(crate) fn rl(&mut self, x: u8) -> u8 {
        let carry = (x >> 7) & 1;
        let x = (x << 1) | self.f.c();
        self.rot_flags(x, carry);
        x
    }

    pub(crate) fn rr(&mut self, x: u8) -> u8 {
        let carry = x & 1;
        let x = (x >> 1) | (self.f.c() << 7);
        self.rot_flags(x, carry);
        x
    }

    pub(crate) fn sla(&mut self, x: u8) -> u8 {
        let carry = (x >> 7) & 1;
        let x = x << 1;
        self.rot_flags(x, carry);
        x
    }

    pub(crate) fn sra(&mut self, x: u8) -> u8 {
        let carry = x & 1;
        let x = (x & 0x80) | (x >> 1);
        self.rot_flags(x, carry);
        x
    }

    pub(crate) fn sll(&mut self, x: u8) -> u8 {
        let carry = (x >> 7) & 1;
        let x = (x << 1) | 1;
        self.rot_flags(x, carry);
        x
    }

    pub(crate) fn srl(&mut self, x: u8) -> u8 {
        let carry = x & 1;
        let x = x >> 1;
        self.rot_flags(x, carry);
        x
    }

    pub(crate) fn rot_apply(&mut self, group: u8, value: u8) -> u8 {
        match group {
            0 => self.rlc(value),
            1 => self.rrc(value),
            2 => self.rl(value),
            3 => self.rr(value),
            4 => self.sla(value),
            5 => self.sra(value),
            6 => self.sll(value),
            _ => self.srl(value),
        }
    }

    /// RLC/RRC/RL/RR/SLA/SRA/SLL/SRL r -- CB-prefixed rotates and shifts, including (HL).
    pub(crate) fn op_rot(&mut self, sub_opcode: u8) -> u32 {
        let group = (sub_opcode >> 3) & 0x07;
        let dest = sub_opcode & 0x07;
        if dest == 6 {
            let addr = self.hl();
            let value = self.bus.read_byte(addr);
            let value = self.rot_apply(group, value);
            self.bus.write_byte(addr, value);
            self.update_q(true);
            return 15;
        }
        let value = self.rot_apply(group, self.read_reg(dest));
        self.write_reg(dest, value);
        self.update_q(true);
        8
    }

    // The accumulator rotates change only C, N, H and X/Y; S, Z and PV are
    // preserved.
    fn acc_rot_flags(&mut self, carry: u8) {
        let f = (self.f.byte() & (FLAG_S | FLAG_Z | FLAG_PV))
            | (self.a & (FLAG_X | FLAG_Y))
            | (carry & FLAG_C);
        self.f = Flags::new(f);
    }

    /// RLCA
    pub(crate) fn op_rlca(&mut self) -> u32 {
        self.a = (self.a << 1) | (self.a >> 7);
        self.acc_rot_flags(self.a & 1);
        self.update_q(true);
        4
    }

    /// RRCA
    pub(crate) fn op_rrca(&mut self) -> u32 {
        let carry = self.a & 1;
        self.a = (self.a >> 1) | (self.a << 7);
        self.acc_rot_flags(carry);
        self.update_q(true);
        4
    }

    /// RLA
    pub(crate) fn op_rla(&mut self) -> u32 {
        let carry_in = self.f.c();
        let carry = (self.a >> 7) & 1;
        self.a = (self.a << 1) | carry_in;
        self.acc_rot_flags(carry);
        self.update_q(true);
        4
    }

    /// RRA
    pub(crate) fn op_rra(&mut self) -> u32 {
        let carry_in = self.f.c();
        let carry = self.a & 1;
        self.a = (self.a >> 1) | (carry_in << 7);
        self.acc_rot_flags(carry);
        self.update_q(true);
        4
    }

    /// BIT's flags: C preserved; N = 0, H = 1; Z and PV set when the bit is
    /// clear; S set only for bit 7 when it is set; X/Y from `xy_source`.
    pub(crate) fn bit_flags(&mut self, value: u8, bit_index: u8, xy_source: u8) {
        let bit_set = (value >> bit_index) & 1;
        let f = (self.f.byte() & FLAG_C)
            | FLAG_H
            | if bit_set == 0 { FLAG_Z | FLAG_PV } else { 0 }
            | if bit_index == 7 && bit_set != 0 {
                FLAG_S
            } else {
                0
            }
            | (xy_source & (FLAG_X | FLAG_Y));
        self.f = Flags::new(f);
    }

    /// BIT b,r -- includes BIT b,(HL).
    pub(crate) fn op_bit(&mut self, sub_opcode: u8) -> u32 {
        let bit_index = (sub_opcode >> 3) & 0x07;
        let src = sub_opcode & 0x07;
        let (value, xy_source, t_states) = if src == 6 {
            let value = self.bus.read_byte(self.hl());
            // X/Y sample the internal address bus during the read: WZ's high byte, not
            // the tested byte. Register forms below take them from the value itself.
            (value, (self.wz >> 8) as u8, 12)
        } else {
            let value = self.read_reg(src);
            (value, value, 8)
        };
        self.bit_flags(value, bit_index, xy_source);
        self.update_q(true);
        t_states
    }

    /// RES b,r -- includes RES b,(HL).
    pub(crate) fn op_res(&mut self, sub_opcode: u8) -> u32 {
        let bit_index = (sub_opcode >> 3) & 0x07;
        let dest = sub_opcode & 0x07;
        let mask = !(1u8 << bit_index);
        let t_states = if dest == 6 {
            let addr = self.hl();
            let value = self.bus.read_byte(addr);
            self.bus.write_byte(addr, value & mask);
            15
        } else {
            self.write_reg(dest, self.read_reg(dest) & mask);
            8
        };
        self.update_q(false);
        t_states
    }

    /// SET b,r -- includes SET b,(HL).
    pub(crate) fn op_set(&mut self, sub_opcode: u8) -> u32 {
        let bit_index = (sub_opcode >> 3) & 0x07;
        let dest = sub_opcode & 0x07;
        let mask = 1u8 << bit_index;
        let t_states = if dest == 6 {
            let addr = self.hl();
            let value = self.bus.read_byte(addr);
            self.bus.write_byte(addr, value | mask);
            15
        } else {
            self.write_reg(dest, self.read_reg(dest) | mask);
            8
        };
        self.update_q(false);
        t_states
    }

    /// RRD -- rotate the BCD digit chain A[3:0] -> (HL)[7:4] -> (HL)[3:0] -> A[3:0] right.
    pub(crate) fn op_rrd(&mut self) -> u32 {
        let addr = self.hl();
        self.wz = addr.wrapping_add(1);
        let data = self.bus.read_byte(addr);
        self.bus.write_byte(addr, (data >> 4) | (self.a << 4));
        self.a = (self.a & 0xF0) | (data & 0x0F);
        // C is preserved; N = H = 0; the rest from A.
        self.f = Flags::new((self.f.byte() & FLAG_C) | SZP[usize::from(self.a)]);
        self.update_q(true);
        18
    }

    /// RLD -- rotate the BCD digit chain A[3:0] -> (HL)[3:0] -> (HL)[7:4] -> A[3:0] left.
    pub(crate) fn op_rld(&mut self) -> u32 {
        let addr = self.hl();
        self.wz = addr.wrapping_add(1);
        let data = self.bus.read_byte(addr);
        self.bus.write_byte(addr, (data << 4) | (self.a & 0x0F));
        self.a = (self.a & 0xF0) | (data >> 4);
        // C is preserved; N = H = 0; the rest from A.
        self.f = Flags::new((self.f.byte() & FLAG_C) | SZP[usize::from(self.a)]);
        self.update_q(true);
        18
    }
}
