//! IX/IY instruction handlers: every handler here names an IX or IY operand.
//!
//! Transcribed from `z80_python/_index.py`. Base-set instructions that a
//! DD/FD prefix merely decorates live with their own instruction group;
//! `index_dispatch.rs` routes them and adds the prefix cost.

use crate::core::{Bus, Z80};
use crate::flags::{Flags, FLAG_C, FLAG_H, FLAG_PV, FLAG_S, FLAG_X, FLAG_Y, FLAG_Z};

impl<B: Bus> Z80<B> {
    pub(crate) fn get_index(&self, prefix: u8) -> u16 {
        if prefix == 0xDD {
            self.ix
        } else {
            self.iy
        }
    }

    pub(crate) fn set_index(&mut self, prefix: u8, value: u16) {
        if prefix == 0xDD {
            self.ix = value;
        } else {
            self.iy = value;
        }
    }

    pub(crate) fn index_high_byte(&self, prefix: u8) -> u8 {
        (self.get_index(prefix) >> 8) as u8
    }

    pub(crate) fn index_low_byte(&self, prefix: u8) -> u8 {
        self.get_index(prefix) as u8
    }

    pub(crate) fn set_index_high_byte(&mut self, prefix: u8, value: u8) {
        let low = self.index_low_byte(prefix);
        self.set_index(prefix, (u16::from(value) << 8) | u16::from(low));
    }

    pub(crate) fn set_index_low_byte(&mut self, prefix: u8, value: u8) {
        let high = self.index_high_byte(prefix);
        self.set_index(prefix, (u16::from(high) << 8) | u16::from(value));
    }

    /// ADD IX/IY,rr -- rr = BC, DE, the index register itself, or SP.
    pub(crate) fn op_add_index_rr(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let pair_index = (sub_opcode >> 4) & 0x03;
        let index = self.get_index(prefix);
        let value = if pair_index == 2 {
            index
        } else {
            self.read_pair(pair_index)
        };
        self.wz = index.wrapping_add(1);
        let wide = u32::from(index) + u32::from(value);
        let result = wide as u16;
        // Only H, N, C and X/Y change; S, Z and PV are preserved.
        let f = (self.f.byte() & (FLAG_S | FLAG_Z | FLAG_PV))
            | ((result >> 8) as u8 & (FLAG_X | FLAG_Y))
            | (((index ^ value ^ result) >> 8) as u8 & FLAG_H)
            | ((wide >> 16) as u8 & FLAG_C);
        self.f = Flags::new(f);
        self.set_index(prefix, result);
        self.update_q(true);
        15
    }

    /// LD IX/IY,nn
    pub(crate) fn op_ld_index_nn(&mut self, prefix: u8) -> u32 {
        let value = self.read_operand_word();
        self.set_index(prefix, value);
        self.update_q(false);
        14
    }

    /// LD (nn),IX/IY
    pub(crate) fn op_ld_nn_index(&mut self, prefix: u8) -> u32 {
        let addr = self.read_operand_word();
        let index = self.get_index(prefix);
        self.bus.write_byte(addr, index as u8);
        self.wz = addr.wrapping_add(1);
        self.bus.write_byte(self.wz, (index >> 8) as u8);
        self.update_q(false);
        20
    }

    /// LD IX/IY,(nn)
    pub(crate) fn op_ld_index_nn_from_mem(&mut self, prefix: u8) -> u32 {
        let addr = self.read_operand_word();
        let low = self.bus.read_byte(addr);
        self.wz = addr.wrapping_add(1);
        let high = self.bus.read_byte(self.wz);
        self.set_index(prefix, u16::from(low) | (u16::from(high) << 8));
        self.update_q(false);
        20
    }

    /// LD A,IXH/IYH
    pub(crate) fn op_ld_a_index_h(&mut self, prefix: u8) -> u32 {
        self.a = self.index_high_byte(prefix);
        self.update_q(false);
        4
    }

    /// LD A,IXL/IYL
    pub(crate) fn op_ld_a_index_l(&mut self, prefix: u8) -> u32 {
        self.a = self.index_low_byte(prefix);
        self.update_q(false);
        4
    }

    /// LD r,IXH/IXL/IYH/IYL
    pub(crate) fn op_ld_r_index_byte(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let value = if (sub_opcode & 0x07) == 4 {
            self.index_high_byte(prefix)
        } else {
            self.index_low_byte(prefix)
        };
        self.write_reg((sub_opcode >> 3) & 0x07, value);
        self.update_q(false);
        4
    }

    /// LD IXH/IXL,IXH/IXL (and the IY forms)
    pub(crate) fn op_ld_index_byte_index_byte(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let value = if (sub_opcode & 0x07) == 4 {
            self.index_high_byte(prefix)
        } else {
            self.index_low_byte(prefix)
        };
        if ((sub_opcode >> 3) & 0x07) == 4 {
            self.set_index_high_byte(prefix, value);
        } else {
            self.set_index_low_byte(prefix, value);
        }
        self.update_q(false);
        4
    }

    /// LD IXH/IXL/IYH/IYL,r
    pub(crate) fn op_ld_index_byte_r(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let value = self.read_reg(sub_opcode & 0x07);
        if ((sub_opcode >> 3) & 0x07) == 4 {
            self.set_index_high_byte(prefix, value);
        } else {
            self.set_index_low_byte(prefix, value);
        }
        self.update_q(false);
        4
    }

    /// LD IXH/IXL/IYH/IYL,n
    pub(crate) fn op_ld_index_byte_n(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let value = self.read_operand_byte();
        if ((sub_opcode >> 3) & 0x07) == 4 {
            self.set_index_high_byte(prefix, value);
        } else {
            self.set_index_low_byte(prefix, value);
        }
        self.update_q(false);
        7
    }

    /// INC/DEC IXH/IXL/IYH/IYL
    pub(crate) fn op_inc_dec_index_byte(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let is_high_byte = ((sub_opcode >> 3) & 0x07) == 4;
        let value = if is_high_byte {
            self.index_high_byte(prefix)
        } else {
            self.index_low_byte(prefix)
        };
        let result = if (sub_opcode & 0x07) == 4 {
            self.inc(value)
        } else {
            self.dec(value)
        };
        if is_high_byte {
            self.set_index_high_byte(prefix, result);
        } else {
            self.set_index_low_byte(prefix, result);
        }
        self.update_q(true);
        4
    }

    /// ADD A,IXH/IYH
    pub(crate) fn op_add_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(0, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// ADD A,IXL/IYL
    pub(crate) fn op_add_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(0, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// ADC A,IXH/IYH
    pub(crate) fn op_adc_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(1, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// ADC A,IXL/IYL
    pub(crate) fn op_adc_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(1, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// SUB A,IXH/IYH
    pub(crate) fn op_sub_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(2, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// SUB A,IXL/IYL
    pub(crate) fn op_sub_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(2, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// SBC A,IXH/IYH
    pub(crate) fn op_sbc_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(3, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// SBC A,IXL/IYL
    pub(crate) fn op_sbc_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(3, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// AND A,IXH/IYH
    pub(crate) fn op_and_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(4, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// AND A,IXL/IYL
    pub(crate) fn op_and_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(4, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// XOR A,IXH/IYH
    pub(crate) fn op_xor_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(5, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// XOR A,IXL/IYL
    pub(crate) fn op_xor_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(5, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// OR A,IXH/IYH
    pub(crate) fn op_or_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(6, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// OR A,IXL/IYL
    pub(crate) fn op_or_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(6, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// CP A,IXH/IYH
    pub(crate) fn op_cp_a_index_h(&mut self, prefix: u8) -> u32 {
        self.alu_a(7, self.index_high_byte(prefix));
        self.update_q(true);
        4
    }

    /// CP A,IXL/IYL
    pub(crate) fn op_cp_a_index_l(&mut self, prefix: u8) -> u32 {
        self.alu_a(7, self.index_low_byte(prefix));
        self.update_q(true);
        4
    }

    /// INC IX/IY -- no flags.
    pub(crate) fn op_inc_index(&mut self, prefix: u8) -> u32 {
        self.set_index(prefix, self.get_index(prefix).wrapping_add(1));
        self.update_q(false);
        10
    }

    /// DEC IX/IY -- no flags.
    pub(crate) fn op_dec_index(&mut self, prefix: u8) -> u32 {
        self.set_index(prefix, self.get_index(prefix).wrapping_sub(1));
        self.update_q(false);
        10
    }

    /// POP IX/IY
    pub(crate) fn op_pop_index(&mut self, prefix: u8) -> u32 {
        let value = self.pop_word();
        self.set_index(prefix, value);
        self.update_q(false);
        14
    }

    /// PUSH IX/IY
    pub(crate) fn op_push_index(&mut self, prefix: u8) -> u32 {
        self.push_word(self.get_index(prefix));
        self.update_q(false);
        15
    }

    /// EX (SP),IX/IY
    pub(crate) fn op_ex_sp_index(&mut self, prefix: u8) -> u32 {
        let value = u16::from(self.bus.read_byte(self.sp))
            | (u16::from(self.bus.read_byte(self.sp.wrapping_add(1))) << 8);
        let index = self.get_index(prefix);
        self.bus.write_byte(self.sp, index as u8);
        self.bus
            .write_byte(self.sp.wrapping_add(1), (index >> 8) as u8);
        self.set_index(prefix, value);
        self.wz = value;
        self.update_q(false);
        23
    }

    /// JP (IX)/(IY)
    pub(crate) fn op_jp_index(&mut self, prefix: u8) -> u32 {
        self.pc = self.get_index(prefix);
        self.update_q(false);
        8
    }

    /// LD SP,IX/IY
    pub(crate) fn op_ld_sp_index(&mut self, prefix: u8) -> u32 {
        self.sp = self.get_index(prefix);
        self.update_q(false);
        10
    }

    pub(crate) fn index_displacement_addr(&mut self, prefix: u8) -> u16 {
        let displacement = self.read_operand_byte() as i8;
        let addr = self
            .get_index(prefix)
            .wrapping_add(displacement as i16 as u16);
        self.wz = addr;
        addr
    }

    /// BIT b,(IX+d)/(IY+d)
    pub(crate) fn op_index_bit(&mut self, prefix: u8, displacement: u8, sub_opcode: u8) -> u32 {
        let displacement = displacement as i8;
        self.wz = self
            .get_index(prefix)
            .wrapping_add(displacement as i16 as u16);
        let value = self.bus.read_byte(self.wz);
        let bit_index = (sub_opcode >> 3) & 0x07;
        self.bit_flags(value, bit_index, (self.wz >> 8) as u8);
        self.update_q(true);
        20
    }

    /// RLC/RRC/RL/RR/SLA/SRA/SLL/SRL (IX+d)/(IY+d) -- undocumented forms also copy into r.
    pub(crate) fn op_index_rot(&mut self, prefix: u8, displacement: u8, sub_opcode: u8) -> u32 {
        let displacement = displacement as i8;
        self.wz = self
            .get_index(prefix)
            .wrapping_add(displacement as i16 as u16);
        let value = self.bus.read_byte(self.wz);
        let value = self.rot_apply((sub_opcode >> 3) & 0x07, value);
        self.bus.write_byte(self.wz, value);
        let dest = sub_opcode & 0x07;
        if dest != 6 {
            self.write_reg(dest, value);
        }
        self.update_q(true);
        23
    }

    /// RES/SET b,(IX+d)/(IY+d) -- the undocumented forms also copy the result into r.
    pub(crate) fn op_index_res_set(
        &mut self,
        prefix: u8,
        displacement: u8,
        sub_opcode: u8,
        set_bit: bool,
    ) -> u32 {
        let displacement = displacement as i8;
        self.wz = self
            .get_index(prefix)
            .wrapping_add(displacement as i16 as u16);
        let mask = 1u8 << ((sub_opcode >> 3) & 0x07);
        let value = self.bus.read_byte(self.wz);
        let value = if set_bit { value | mask } else { value & !mask };
        self.bus.write_byte(self.wz, value);
        let dest = sub_opcode & 0x07;
        if dest != 6 {
            self.write_reg(dest, value);
        }
        self.update_q(false);
        23
    }

    /// INC (IX+d)/(IY+d)
    pub(crate) fn op_inc_index_mem(&mut self, prefix: u8) -> u32 {
        let addr = self.index_displacement_addr(prefix);
        let value = self.bus.read_byte(addr);
        let value = self.inc(value);
        self.bus.write_byte(addr, value);
        self.update_q(true);
        23
    }

    /// DEC (IX+d)/(IY+d)
    pub(crate) fn op_dec_index_mem(&mut self, prefix: u8) -> u32 {
        let addr = self.index_displacement_addr(prefix);
        let value = self.bus.read_byte(addr);
        let value = self.dec(value);
        self.bus.write_byte(addr, value);
        self.update_q(true);
        23
    }

    /// LD r,(IX+d)/(IY+d)
    pub(crate) fn op_ld_r_index_mem(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let dest = (sub_opcode >> 3) & 0x07;
        let addr = self.index_displacement_addr(prefix);
        let value = self.bus.read_byte(addr);
        self.write_reg(dest, value);
        self.update_q(false);
        19
    }

    /// LD (IX+d)/(IY+d),r
    pub(crate) fn op_ld_index_mem_r(&mut self, prefix: u8, sub_opcode: u8) -> u32 {
        let src = sub_opcode & 0x07;
        let addr = self.index_displacement_addr(prefix);
        self.bus.write_byte(addr, self.read_reg(src));
        self.update_q(false);
        19
    }

    /// LD (IX+d)/(IY+d),n
    pub(crate) fn op_ld_index_mem_n(&mut self, prefix: u8) -> u32 {
        let addr = self.index_displacement_addr(prefix);
        let value = self.read_operand_byte();
        self.bus.write_byte(addr, value);
        self.update_q(false);
        19
    }

    pub(crate) fn indexed_alu(&mut self, prefix: u8, group: u8) -> u32 {
        let addr = self.index_displacement_addr(prefix);
        let value = self.bus.read_byte(addr);
        self.alu_a(group, value);
        self.update_q(true);
        19
    }

    /// ADD A,(IX+d)/(IY+d)
    pub(crate) fn op_add_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 0)
    }

    /// ADC A,(IX+d)/(IY+d)
    pub(crate) fn op_adc_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 1)
    }

    /// SUB A,(IX+d)/(IY+d)
    pub(crate) fn op_sub_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 2)
    }

    /// SBC A,(IX+d)/(IY+d)
    pub(crate) fn op_sbc_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 3)
    }

    /// AND A,(IX+d)/(IY+d)
    pub(crate) fn op_and_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 4)
    }

    /// XOR A,(IX+d)/(IY+d)
    pub(crate) fn op_xor_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 5)
    }

    /// OR A,(IX+d)/(IY+d)
    pub(crate) fn op_or_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 6)
    }

    /// CP A,(IX+d)/(IY+d)
    pub(crate) fn op_cp_a_index_mem(&mut self, prefix: u8) -> u32 {
        self.indexed_alu(prefix, 7)
    }
}
