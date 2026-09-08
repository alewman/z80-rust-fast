//! Rotate, shift, and bit-operation implementation.
//!
//! Transcribed from `z80_python/_rotate.py`. The rotates keep the reference's
//! shift-and-or form rather than `rotate_left`, so the two read the same.
#![allow(clippy::manual_rotate)]

use crate::core::{Bus, Z80};
use crate::flags::{Flags, FLAG_C, SZP};

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

    /// RLCA
    pub(crate) fn op_rlca(&mut self) -> u32 {
        self.a = (self.a << 1) | (self.a >> 7);
        self.f.set_c(self.a & 1);
        self.f.set_n(0);
        self.f.set_h(0);
        self.f.set_xy(self.a);
        self.update_q(true);
        4
    }

    /// RRCA
    pub(crate) fn op_rrca(&mut self) -> u32 {
        self.f.set_c(self.a & 1);
        self.a = (self.a >> 1) | (self.a << 7);
        self.f.set_n(0);
        self.f.set_h(0);
        self.f.set_xy(self.a);
        self.update_q(true);
        4
    }

    /// RLA
    pub(crate) fn op_rla(&mut self) -> u32 {
        let carry_in = self.f.c();
        self.f.set_c((self.a >> 7) & 1);
        self.a = (self.a << 1) | carry_in;
        self.f.set_n(0);
        self.f.set_h(0);
        self.f.set_xy(self.a);
        self.update_q(true);
        4
    }

    /// RRA
    pub(crate) fn op_rra(&mut self) -> u32 {
        let carry_in = self.f.c();
        self.f.set_c(self.a & 1);
        self.a = (self.a >> 1) | (carry_in << 7);
        self.f.set_n(0);
        self.f.set_h(0);
        self.f.set_xy(self.a);
        self.update_q(true);
        4
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
        let bit_set = (value >> bit_index) & 1;
        self.f.set_n(0);
        self.f.set_h(1);
        self.f.set_z(if bit_set != 0 { 0 } else { 1 });
        self.f.set_pv(self.f.z());
        self.f
            .set_s(if bit_index == 7 && bit_set != 0 { 1 } else { 0 });
        self.f.set_xy(xy_source);
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
        self.f.set_n(0);
        self.f.set_h(0);
        self.set_parity(self.a);
        self.set_xysz(self.a);
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
        self.f.set_n(0);
        self.f.set_h(0);
        self.set_parity(self.a);
        self.set_xysz(self.a);
        self.update_q(true);
        18
    }
}
