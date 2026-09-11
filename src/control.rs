//! Jump, call, return, restart, exchange, and CPU-control (NOP/HALT/DI/EI/IM) group.
//!
//! Transcribed from `z80_python/_control.py`.

use crate::core::{Bus, Z80};

impl<B: Bus> Z80<B> {
    pub(crate) fn cond_true(&self, cond: u8) -> bool {
        if cond == 0 {
            return self.f.z() == 0;
        }
        if cond == 1 {
            return self.f.z() == 1;
        }
        if cond == 2 {
            return self.f.c() == 0;
        }
        if cond == 3 {
            return self.f.c() == 1;
        }
        if cond == 4 {
            return self.f.pv() == 0;
        }
        if cond == 5 {
            return self.f.pv() == 1;
        }
        if cond == 6 {
            return self.f.s() == 0;
        }
        self.f.s() == 1
    }

    /// JR cc,e -- relative jump; JR e is the always-taken form. 12 T-states taken, 7 not.
    pub(crate) fn op_jr(&mut self, opcode: u8) -> u32 {
        let displacement = self.read_operand_byte() as i8;
        let taken = if opcode == 0x18 {
            true
        } else if opcode == 0x20 {
            self.f.z() == 0
        } else if opcode == 0x28 {
            self.f.z() == 1
        } else if opcode == 0x30 {
            self.f.c() == 0
        } else {
            self.f.c() == 1
        };
        if !taken {
            self.update_q(false);
            return 7;
        }
        // Unlike JP, the target is only computed (and latched into WZ) when the
        // branch is taken; a not-taken JR leaves WZ untouched.
        self.wz = self.pc.wrapping_add(displacement as i16 as u16);
        self.pc = self.wz;
        self.update_q(false);
        12
    }

    /// JP cc,nn -- absolute jump; JP nn is the always-taken form. 10 T-states either way.
    pub(crate) fn op_jp(&mut self, opcode: u8) -> u32 {
        // The operand fetch itself latches nn into WZ, so WZ changes even when the
        // condition fails and the jump is not taken.
        self.wz = self.read_operand_word();
        if opcode == 0xC3 || self.cond_true((opcode >> 3) & 0x07) {
            self.pc = self.wz;
        }
        self.update_q(false);
        10
    }

    /// JP (HL)
    pub(crate) fn op_jp_hl(&mut self) -> u32 {
        self.pc = self.hl();
        self.update_q(false);
        4
    }

    /// EX DE,HL
    pub(crate) fn op_ex_de_hl(&mut self) -> u32 {
        let de = self.de();
        self.d = self.h;
        self.e = self.l;
        self.h = (de >> 8) as u8;
        self.l = de as u8;
        self.update_q(false);
        4
    }

    /// DI/EI -- update both interrupt-enable flip-flops.
    pub(crate) fn op_interrupt_enable(&mut self, enabled: bool) -> u32 {
        self.iff1 = enabled;
        self.iff2 = enabled;
        self.ei_delay = if enabled { 1 } else { 0 };
        self.update_q(false);
        4
    }

    /// CALL cc,nn -- CALL nn is the always-taken form. 17 T-states taken, 10 not.
    pub(crate) fn op_call(&mut self, opcode: u8) -> u32 {
        // The operand fetch itself latches nn into WZ, so WZ changes even when the
        // condition fails and the jump is not taken.
        self.wz = self.read_operand_word();
        if opcode != 0xCD && !self.cond_true((opcode >> 3) & 0x07) {
            self.update_q(false);
            return 10;
        }
        self.push_word(self.pc);
        self.pc = self.wz;
        self.update_q(false);
        17
    }

    /// RET
    pub(crate) fn op_ret(&mut self) -> u32 {
        self.wz = self.pop_word();
        self.pc = self.wz;
        self.update_q(false);
        10
    }

    /// RET cc -- 11 T-states taken, 5 not.
    pub(crate) fn op_ret_cc(&mut self, opcode: u8) -> u32 {
        if !self.cond_true((opcode >> 3) & 0x07) {
            self.update_q(false);
            return 5;
        }
        self.wz = self.pop_word();
        self.pc = self.wz;
        self.update_q(false);
        11
    }

    pub(crate) fn ret_iff(&mut self) -> u32 {
        self.wz = self.pop_word();
        self.pc = self.wz;
        self.iff1 = self.iff2;
        self.update_q(false);
        14
    }

    /// RETN -- return from NMI; restores IFF1 from IFF2.
    pub(crate) fn op_retn(&mut self) -> u32 {
        self.ret_iff()
    }

    /// RETI -- return from maskable interrupt; restores IFF1 from IFF2 exactly like RETN.
    pub(crate) fn op_reti(&mut self) -> u32 {
        self.ret_iff()
    }

    /// RST p
    pub(crate) fn op_rst(&mut self, opcode: u8) -> u32 {
        self.push_word(self.pc);
        self.wz = u16::from(((opcode >> 3) & 0x07) << 3);
        self.pc = self.wz;
        self.update_q(false);
        11
    }

    /// IM n -- select Z80 interrupt mode 0, 1, or 2.
    pub(crate) fn op_im(&mut self, mode: u8) -> u32 {
        self.im = mode;
        self.update_q(false);
        8
    }

    /// NOP (undocumented ED-prefixed form) -- 8 T-states, no state change.
    pub(crate) fn op_ed_nop(&mut self) -> u32 {
        self.update_q(false);
        8
    }

    /// EXX
    pub(crate) fn op_exx(&mut self) -> u32 {
        let (bc, de, hl) = (self.bc(), self.de(), self.hl());
        self.write_pair(0, self.bc_);
        self.write_pair(1, self.de_);
        self.write_pair(2, self.hl_);
        self.bc_ = bc;
        self.de_ = de;
        self.hl_ = hl;
        self.update_q(false);
        4
    }

    /// EX (SP),HL
    pub(crate) fn op_ex_sp_hl(&mut self) -> u32 {
        let value = u16::from(self.bus.read_byte(self.sp))
            | (u16::from(self.bus.read_byte(self.sp.wrapping_add(1))) << 8);
        let hl = self.hl();
        self.bus
            .write_byte(self.sp.wrapping_add(1), (hl >> 8) as u8);
        self.bus.write_byte(self.sp, hl as u8);
        self.h = (value >> 8) as u8;
        self.l = value as u8;
        self.wz = value;
        self.update_q(false);
        19
    }

    /// HALT
    pub(crate) fn op_halt(&mut self) -> u32 {
        self.halted = true;
        self.update_q(false);
        4
    }

    /// NOP
    pub(crate) fn op_nop(&mut self) -> u32 {
        self.update_q(false);
        4
    }

    /// EX AF,AF'
    pub(crate) fn op_ex_af_af(&mut self) -> u32 {
        let af = (u16::from(self.a) << 8) | u16::from(self.f.byte());
        self.a = (self.af_ >> 8) as u8;
        self.f.set_byte(self.af_ as u8);
        self.af_ = af;
        self.update_q(false);
        4
    }

    /// DJNZ e -- decrement B and branch if it is not zero; 13 T-states taken, 8 not.
    pub(crate) fn op_djnz(&mut self) -> u32 {
        let displacement = self.read_operand_byte() as i8;
        self.b = self.b.wrapping_sub(1);
        if self.b != 0 {
            self.wz = self.pc.wrapping_add(displacement as i16 as u16);
            self.pc = self.wz;
            self.update_q(false);
            return 13;
        }
        self.update_q(false);
        8
    }
}
