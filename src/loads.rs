//! 8-bit and 16-bit load group, including PUSH/POP (Zilog files the stack transfers here).
//!
//! Transcribed from `z80_python/_loads.py`.

use crate::core::{Bus, Z80};
use crate::flags::{Flags, FLAG_C, FLAG_PV, SZP};

impl<B: Bus> Z80<B> {
    /// LD r,r' -- includes the (HL) source and destination forms.
    pub(crate) fn op_ld_r_r(&mut self, opcode: u8) -> u32 {
        let dest = (opcode >> 3) & 0x07;
        let src = opcode & 0x07;
        let mut t_states = 4;
        let value = if src == 6 {
            t_states = 7;
            self.bus.read_byte(self.hl())
        } else {
            self.read_reg(src)
        };
        if dest == 6 {
            self.bus.write_byte(self.hl(), value);
            t_states = 7;
        } else {
            self.write_reg(dest, value);
        }
        self.update_q(false);
        t_states
    }

    /// LD r,n -- includes LD (HL),n.
    pub(crate) fn op_ld_r_n(&mut self, opcode: u8) -> u32 {
        let value = self.read_operand_byte();
        let dest = (opcode >> 3) & 0x07;
        let t_states = if dest == 6 {
            self.bus.write_byte(self.hl(), value);
            10
        } else {
            self.write_reg(dest, value);
            7
        };
        self.update_q(false);
        t_states
    }

    /// LD A,(BC)/(DE)
    pub(crate) fn op_ld_a_irr(&mut self, reg16: u16) -> u32 {
        self.wz = reg16;
        self.a = self.bus.read_byte(self.wz);
        self.wz = self.wz.wrapping_add(1);
        self.update_q(false);
        7
    }

    /// LD (BC)/(DE),A
    pub(crate) fn op_ld_irr_a(&mut self, reg16: u16) -> u32 {
        self.wz = reg16;
        self.bus.write_byte(self.wz, self.a);
        // After the write the address latch increments its low byte only, and its
        // high byte is overwritten by A (the data bus value): WZ = A:(addr+1)&FF.
        self.wz = (u16::from(self.a) << 8) | u16::from(self.wz.wrapping_add(1) as u8);
        self.update_q(false);
        7
    }

    /// LD A,(nn)
    pub(crate) fn op_ld_a_inn(&mut self) -> u32 {
        self.wz = self.read_operand_word();
        self.a = self.bus.read_byte(self.wz);
        self.wz = self.wz.wrapping_add(1);
        self.update_q(false);
        13
    }

    /// LD (nn),A
    pub(crate) fn op_ld_inn_a(&mut self) -> u32 {
        self.wz = self.read_operand_word();
        self.bus.write_byte(self.wz, self.a);
        // After the write the address latch increments its low byte only, and its
        // high byte is overwritten by A (the data bus value): WZ = A:(addr+1)&FF.
        self.wz = (u16::from(self.a) << 8) | u16::from(self.wz.wrapping_add(1) as u8);
        self.update_q(false);
        13
    }

    /// LD I,A
    pub(crate) fn op_ld_i_a(&mut self) -> u32 {
        self.i = self.a;
        self.update_q(false);
        9
    }

    /// LD R,A
    pub(crate) fn op_ld_r_a(&mut self) -> u32 {
        self.r = self.a;
        self.update_q(false);
        9
    }

    /// LD A,I -- the only load that sets flags; PV mirrors IFF2.
    pub(crate) fn op_ld_a_i(&mut self) -> u32 {
        let value = self.i;
        self.a = value;
        // C is preserved; N = H = 0; S, Z, X, Y from the value; PV reports
        // IFF2, the only way software can read the interrupt-enable state.
        let f = (self.f.byte() & FLAG_C)
            | (SZP[usize::from(value)] & !FLAG_PV)
            | if self.iff2 { FLAG_PV } else { 0 };
        self.f = Flags::new(f);
        self.update_q(true);
        9
    }

    /// LD A,R -- the only load that sets flags; PV mirrors IFF2.
    pub(crate) fn op_ld_a_r(&mut self) -> u32 {
        let value = self.r;
        self.a = value;
        // C is preserved; N = H = 0; S, Z, X, Y from the value; PV reports
        // IFF2, the only way software can read the interrupt-enable state.
        let f = (self.f.byte() & FLAG_C)
            | (SZP[usize::from(value)] & !FLAG_PV)
            | if self.iff2 { FLAG_PV } else { 0 };
        self.f = Flags::new(f);
        self.update_q(true);
        9
    }

    /// LD (nn),HL
    pub(crate) fn op_ld_nn_hl(&mut self) -> u32 {
        let addr = self.read_operand_word();
        self.bus.write_byte(addr, self.l);
        self.wz = addr.wrapping_add(1);
        self.bus.write_byte(self.wz, self.h);
        self.update_q(false);
        16
    }

    /// LD HL,(nn)
    pub(crate) fn op_ld_hl_nn_from_mem(&mut self) -> u32 {
        let addr = self.read_operand_word();
        self.l = self.bus.read_byte(addr);
        self.wz = addr.wrapping_add(1);
        self.h = self.bus.read_byte(self.wz);
        self.update_q(false);
        16
    }

    /// LD (nn),rr for the ED-prefixed register-pair transfer forms.
    pub(crate) fn op_ld_nn_rr(&mut self, pair_index: u8) -> u32 {
        let addr = self.read_operand_word();
        let value = self.read_pair(pair_index);
        self.bus.write_byte(addr, value as u8);
        self.wz = addr.wrapping_add(1);
        self.bus.write_byte(self.wz, (value >> 8) as u8);
        self.update_q(false);
        20
    }

    /// LD rr,(nn) for the ED-prefixed register-pair transfer forms.
    pub(crate) fn op_ld_rr_nn_from_mem(&mut self, pair_index: u8) -> u32 {
        let addr = self.read_operand_word();
        let low = self.bus.read_byte(addr);
        self.wz = addr.wrapping_add(1);
        let high = self.bus.read_byte(self.wz);
        self.write_pair(pair_index, u16::from(low) | (u16::from(high) << 8));
        self.update_q(false);
        20
    }

    /// LD SP,HL -- copy HL into SP.
    pub(crate) fn op_ld_sp_hl(&mut self) -> u32 {
        self.sp = self.hl();
        self.update_q(false);
        6
    }

    /// POP rr
    pub(crate) fn op_pop_rr(&mut self, sub_opcode: u8) -> u32 {
        let value = self.pop_word();
        self.write_pair((sub_opcode >> 4) & 0x03, value);
        self.update_q(false);
        10
    }

    /// PUSH rr
    pub(crate) fn op_push_rr(&mut self, sub_opcode: u8) -> u32 {
        self.push_word(self.read_pair((sub_opcode >> 4) & 0x03));
        self.update_q(false);
        11
    }

    /// POP AF
    pub(crate) fn op_pop_af(&mut self) -> u32 {
        let value = self.pop_word();
        self.a = (value >> 8) as u8;
        self.f.set_byte(value as u8);
        self.update_q(false);
        10
    }

    /// PUSH AF
    pub(crate) fn op_push_af(&mut self) -> u32 {
        self.push_word((u16::from(self.a) << 8) | u16::from(self.f.byte()));
        self.update_q(false);
        11
    }

    /// LD rr,nn -- BC, DE, HL, or SP from a 16-bit immediate.
    pub(crate) fn op_ld_rr_nn(&mut self, opcode: u8) -> u32 {
        let value = self.read_operand_word();
        self.write_pair((opcode >> 4) & 0x03, value);
        self.update_q(false);
        10
    }
}
