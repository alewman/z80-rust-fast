//! DD/FD index-prefix dispatcher.
//!
//! Transcribed from `z80_python/_index_dispatch.py`. A DD or FD prefix is its
//! own M1 fetch on real hardware: it costs 4 T-states and bumps R before the
//! opcode after it is fetched. Base-set opcodes that the prefix does not
//! modify therefore run their ordinary handler and add that 4 here; the
//! handlers that take a `prefix` argument already include the prefix cost in
//! their documented totals.
//!
//! A DD or FD is not an instruction but a flag saying "use IX (or IY) instead
//! of HL" for the opcode that follows. That is why a run of them is legal:
//! each one is a stray M1 that costs 4 T-states and bumps R, the last one
//! decides the register, and an ED after any of them starts an ED instruction
//! that the flag cannot touch. Sean Young, *The Undocumented Z80 Documented*
//! v0.91, sections 3.7 and 6.1; FUSE's `ddfd00` test case encodes the same
//! costs.

use crate::core::{Bus, Fault, Z80};

impl<B: Bus> Z80<B> {
    pub(crate) fn execute_index(&mut self, prefix: u8, sub_opcode: u8) -> Result<u32, Fault> {
        let mut prefix = prefix;
        let mut sub_opcode = sub_opcode;
        let mut stray_t_states = 0;
        while matches!(sub_opcode, 0xDD | 0xFD) {
            // A DD or FD before another DD or FD is a stray prefix: its own M1
            // (4 T-states, R+1, already counted by fetch_byte) and nothing else.
            // "In a large sequence of DD and FD bytes, it is the last one that
            // counts" (Young 3.7). No interrupt is accepted inside the run,
            // because the run is one instruction to step() (Young, chapter 5).
            prefix = sub_opcode;
            sub_opcode = self.fetch_byte();
            stray_t_states += 4;
        }
        if sub_opcode == 0xED {
            // "If CB or ED is encountered, that byte plus the next make up an
            // instruction" (Young 3.7), and ED instructions never use the IX/IY
            // substitution (Young 3.2), so the DD/FD is a 4-T-state stray.
            let ed_opcode = self.fetch_byte();
            return Ok(stray_t_states + 4 + self.execute_ed(ed_opcode));
        }
        Ok(stray_t_states + self.execute_index_opcode(prefix, sub_opcode)?)
    }

    /// Dispatch the byte after the last DD/FD prefix: one exhaustive `match`
    /// (a jump table) with every arm calling the handler and adding the
    /// prefix cost exactly as the reference's if-chain did.
    pub(crate) fn execute_index_opcode(
        &mut self,
        prefix: u8,
        sub_opcode: u8,
    ) -> Result<u32, Fault> {
        let t_states = match sub_opcode {
            0xCB => {
                let displacement = self.read_operand_byte();
                // The final DDCB opcode byte is read as an operand, not an M1 fetch, so R
                // advances only for the DD and CB prefixes.
                let cb_opcode = self.read_operand_byte();
                match cb_opcode {
                    0x00..=0x3F => self.op_index_rot(prefix, displacement, cb_opcode),
                    0x40..=0x7F => self.op_index_bit(prefix, displacement, cb_opcode),
                    0x80..=0xBF => self.op_index_res_set(prefix, displacement, cb_opcode, false),
                    0xC0..=0xFF => self.op_index_res_set(prefix, displacement, cb_opcode, true),
                }
            }

            // Consumed by execute_index before this table; kept so the table
            // stays complete and a direct call behaves as the if-chain did.
            0xDD => {
                return Err(Fault::UnhandledIndexOpcode {
                    prefix,
                    opcode: 0xDD,
                    pc: self.pc.wrapping_sub(2),
                })
            }
            0xED => {
                return Err(Fault::UnhandledIndexOpcode {
                    prefix,
                    opcode: 0xED,
                    pc: self.pc.wrapping_sub(2),
                })
            }
            0xFD => {
                return Err(Fault::UnhandledIndexOpcode {
                    prefix,
                    opcode: 0xFD,
                    pc: self.pc.wrapping_sub(2),
                })
            }

            // Base-set instructions the prefix does not modify: their own
            // handler plus the prefix's 4 T-states.
            0x00 => self.op_nop() + 4,

            0x01 => self.op_ld_rr_nn(0x01) + 4,
            0x11 => self.op_ld_rr_nn(0x11) + 4,
            0x31 => self.op_ld_rr_nn(0x31) + 4,

            0x02 => self.op_ld_irr_a(self.bc()) + 4,

            0x12 => self.op_ld_irr_a(self.de()) + 4,

            0x03 => self.op_inc_rr(0x03) + 4,
            0x13 => self.op_inc_rr(0x13) + 4,
            0x33 => self.op_inc_rr(0x33) + 4,

            0x04 => self.op_inc_r(0x04) + 4,
            0x0C => self.op_inc_r(0x0C) + 4,
            0x14 => self.op_inc_r(0x14) + 4,
            0x1C => self.op_inc_r(0x1C) + 4,
            0x3C => self.op_inc_r(0x3C) + 4,

            0x05 => self.op_dec_r(0x05) + 4,
            0x0D => self.op_dec_r(0x0D) + 4,
            0x15 => self.op_dec_r(0x15) + 4,
            0x1D => self.op_dec_r(0x1D) + 4,
            0x3D => self.op_dec_r(0x3D) + 4,

            0x06 => self.op_ld_r_n(0x06) + 4,
            0x0E => self.op_ld_r_n(0x0E) + 4,
            0x16 => self.op_ld_r_n(0x16) + 4,
            0x1E => self.op_ld_r_n(0x1E) + 4,
            0x3E => self.op_ld_r_n(0x3E) + 4,

            0x07 => self.op_rlca() + 4,

            0x08 => self.op_ex_af_af() + 4,

            0x0A => self.op_ld_a_irr(self.bc()) + 4,

            0x1A => self.op_ld_a_irr(self.de()) + 4,

            0x0B => self.op_dec_rr(0x0B) + 4,
            0x1B => self.op_dec_rr(0x1B) + 4,
            0x3B => self.op_dec_rr(0x3B) + 4,

            0x0F => self.op_rrca() + 4,

            0x10 => self.op_djnz() + 4,

            0x17 => self.op_rla() + 4,

            0x18 => self.op_jr(0x18) + 4,
            0x20 => self.op_jr(0x20) + 4,
            0x28 => self.op_jr(0x28) + 4,
            0x30 => self.op_jr(0x30) + 4,
            0x38 => self.op_jr(0x38) + 4,

            0x1F => self.op_rra() + 4,

            0x27 => self.op_daa() + 4,

            0x2F => self.op_cpl() + 4,

            0x32 => self.op_ld_inn_a() + 4,

            0x37 => self.op_scf_ccf(0x37, true) + 4,
            0x3F => self.op_scf_ccf(0x3F, true) + 4,

            0x3A => self.op_ld_a_inn() + 4,

            0x76 => self.op_halt() + 4,

            0x40 => self.op_ld_r_r(0x40) + 4,
            0x41 => self.op_ld_r_r(0x41) + 4,
            0x42 => self.op_ld_r_r(0x42) + 4,
            0x43 => self.op_ld_r_r(0x43) + 4,
            0x47 => self.op_ld_r_r(0x47) + 4,
            0x48 => self.op_ld_r_r(0x48) + 4,
            0x49 => self.op_ld_r_r(0x49) + 4,
            0x4A => self.op_ld_r_r(0x4A) + 4,
            0x4B => self.op_ld_r_r(0x4B) + 4,
            0x4F => self.op_ld_r_r(0x4F) + 4,
            0x50 => self.op_ld_r_r(0x50) + 4,
            0x51 => self.op_ld_r_r(0x51) + 4,
            0x52 => self.op_ld_r_r(0x52) + 4,
            0x53 => self.op_ld_r_r(0x53) + 4,
            0x57 => self.op_ld_r_r(0x57) + 4,
            0x58 => self.op_ld_r_r(0x58) + 4,
            0x59 => self.op_ld_r_r(0x59) + 4,
            0x5A => self.op_ld_r_r(0x5A) + 4,
            0x5B => self.op_ld_r_r(0x5B) + 4,
            0x5F => self.op_ld_r_r(0x5F) + 4,
            0x78 => self.op_ld_r_r(0x78) + 4,
            0x79 => self.op_ld_r_r(0x79) + 4,
            0x7A => self.op_ld_r_r(0x7A) + 4,
            0x7B => self.op_ld_r_r(0x7B) + 4,
            0x7F => self.op_ld_r_r(0x7F) + 4,

            // plain_alu: every 8-bit ALU form whose operand is not H, L, or (HL).
            0x80 => self.op_alu_r(0x80) + 4,
            0x81 => self.op_alu_r(0x81) + 4,
            0x82 => self.op_alu_r(0x82) + 4,
            0x83 => self.op_alu_r(0x83) + 4,
            0x87 => self.op_alu_r(0x87) + 4,
            0x88 => self.op_alu_r(0x88) + 4,
            0x89 => self.op_alu_r(0x89) + 4,
            0x8A => self.op_alu_r(0x8A) + 4,
            0x8B => self.op_alu_r(0x8B) + 4,
            0x8F => self.op_alu_r(0x8F) + 4,
            0x90 => self.op_alu_r(0x90) + 4,
            0x91 => self.op_alu_r(0x91) + 4,
            0x92 => self.op_alu_r(0x92) + 4,
            0x93 => self.op_alu_r(0x93) + 4,
            0x97 => self.op_alu_r(0x97) + 4,
            0x98 => self.op_alu_r(0x98) + 4,
            0x99 => self.op_alu_r(0x99) + 4,
            0x9A => self.op_alu_r(0x9A) + 4,
            0x9B => self.op_alu_r(0x9B) + 4,
            0x9F => self.op_alu_r(0x9F) + 4,
            0xA0 => self.op_alu_r(0xA0) + 4,
            0xA1 => self.op_alu_r(0xA1) + 4,
            0xA2 => self.op_alu_r(0xA2) + 4,
            0xA3 => self.op_alu_r(0xA3) + 4,
            0xA7 => self.op_alu_r(0xA7) + 4,
            0xA8 => self.op_alu_r(0xA8) + 4,
            0xA9 => self.op_alu_r(0xA9) + 4,
            0xAA => self.op_alu_r(0xAA) + 4,
            0xAB => self.op_alu_r(0xAB) + 4,
            0xAF => self.op_alu_r(0xAF) + 4,
            0xB0 => self.op_alu_r(0xB0) + 4,
            0xB1 => self.op_alu_r(0xB1) + 4,
            0xB2 => self.op_alu_r(0xB2) + 4,
            0xB3 => self.op_alu_r(0xB3) + 4,
            0xB7 => self.op_alu_r(0xB7) + 4,
            0xB8 => self.op_alu_r(0xB8) + 4,
            0xB9 => self.op_alu_r(0xB9) + 4,
            0xBA => self.op_alu_r(0xBA) + 4,
            0xBB => self.op_alu_r(0xBB) + 4,
            0xBF => self.op_alu_r(0xBF) + 4,

            0xC0 => self.op_ret_cc(0xC0) + 4,
            0xC8 => self.op_ret_cc(0xC8) + 4,
            0xD0 => self.op_ret_cc(0xD0) + 4,
            0xD8 => self.op_ret_cc(0xD8) + 4,
            0xE0 => self.op_ret_cc(0xE0) + 4,
            0xE8 => self.op_ret_cc(0xE8) + 4,
            0xF0 => self.op_ret_cc(0xF0) + 4,
            0xF8 => self.op_ret_cc(0xF8) + 4,

            0xC1 => self.op_pop_rr(0xC1) + 4,
            0xD1 => self.op_pop_rr(0xD1) + 4,

            0xC2 => self.op_jp(0xC2) + 4,
            0xCA => self.op_jp(0xCA) + 4,
            0xD2 => self.op_jp(0xD2) + 4,
            0xDA => self.op_jp(0xDA) + 4,
            0xE2 => self.op_jp(0xE2) + 4,
            0xEA => self.op_jp(0xEA) + 4,
            0xF2 => self.op_jp(0xF2) + 4,
            0xFA => self.op_jp(0xFA) + 4,
            0xC3 => self.op_jp(0xC3) + 4,

            0xC4 => self.op_call(0xC4) + 4,
            0xCC => self.op_call(0xCC) + 4,
            0xD4 => self.op_call(0xD4) + 4,
            0xDC => self.op_call(0xDC) + 4,
            0xE4 => self.op_call(0xE4) + 4,
            0xEC => self.op_call(0xEC) + 4,
            0xF4 => self.op_call(0xF4) + 4,
            0xFC => self.op_call(0xFC) + 4,
            0xCD => self.op_call(0xCD) + 4,

            0xC5 => self.op_push_rr(0xC5) + 4,
            0xD5 => self.op_push_rr(0xD5) + 4,

            0xC6 => self.op_alu_n(0xC6) + 4,
            0xCE => self.op_alu_n(0xCE) + 4,
            0xD6 => self.op_alu_n(0xD6) + 4,
            0xDE => self.op_alu_n(0xDE) + 4,
            0xE6 => self.op_alu_n(0xE6) + 4,
            0xEE => self.op_alu_n(0xEE) + 4,
            0xF6 => self.op_alu_n(0xF6) + 4,
            0xFE => self.op_alu_n(0xFE) + 4,

            0xC7 => self.op_rst(0xC7) + 4,
            0xCF => self.op_rst(0xCF) + 4,
            0xD7 => self.op_rst(0xD7) + 4,
            0xDF => self.op_rst(0xDF) + 4,
            0xE7 => self.op_rst(0xE7) + 4,
            0xEF => self.op_rst(0xEF) + 4,
            0xF7 => self.op_rst(0xF7) + 4,
            0xFF => self.op_rst(0xFF) + 4,

            0xC9 => self.op_ret() + 4,

            0xD3 => self.op_out_n_a() + 4,

            0xD9 => self.op_exx() + 4,

            0xDB => self.op_in_a_n() + 4,

            0xEB => self.op_ex_de_hl() + 4,

            0xF1 => self.op_pop_af() + 4,

            0xF3 => self.op_interrupt_enable(false) + 4,

            0xF5 => self.op_push_af() + 4,

            0xFB => self.op_interrupt_enable(true) + 4,

            // Instructions that name IX/IY: the handler's total already
            // includes the prefix.
            0x09 => self.op_add_index_rr(prefix, 0x09),
            0x19 => self.op_add_index_rr(prefix, 0x19),
            0x29 => self.op_add_index_rr(prefix, 0x29),
            0x39 => self.op_add_index_rr(prefix, 0x39),

            0x21 => self.op_ld_index_nn(prefix),

            0x22 => self.op_ld_nn_index(prefix),

            0x23 => self.op_inc_index(prefix),

            0x24 => self.op_inc_dec_index_byte(prefix, 0x24) + 4,
            0x25 => self.op_inc_dec_index_byte(prefix, 0x25) + 4,
            0x2C => self.op_inc_dec_index_byte(prefix, 0x2C) + 4,
            0x2D => self.op_inc_dec_index_byte(prefix, 0x2D) + 4,

            0x26 => self.op_ld_index_byte_n(prefix, 0x26) + 4,
            0x2E => self.op_ld_index_byte_n(prefix, 0x2E) + 4,

            0x2A => self.op_ld_index_nn_from_mem(prefix),

            0x2B => self.op_dec_index(prefix),

            0x34 => self.op_inc_index_mem(prefix),

            0x35 => self.op_dec_index_mem(prefix),

            0x36 => self.op_ld_index_mem_n(prefix),

            0x44 => self.op_ld_r_index_byte(prefix, 0x44) + 4,
            0x45 => self.op_ld_r_index_byte(prefix, 0x45) + 4,
            0x4C => self.op_ld_r_index_byte(prefix, 0x4C) + 4,
            0x4D => self.op_ld_r_index_byte(prefix, 0x4D) + 4,
            0x54 => self.op_ld_r_index_byte(prefix, 0x54) + 4,
            0x55 => self.op_ld_r_index_byte(prefix, 0x55) + 4,
            0x5C => self.op_ld_r_index_byte(prefix, 0x5C) + 4,
            0x5D => self.op_ld_r_index_byte(prefix, 0x5D) + 4,

            0x46 => self.op_ld_r_index_mem(prefix, 0x46),
            0x4E => self.op_ld_r_index_mem(prefix, 0x4E),
            0x56 => self.op_ld_r_index_mem(prefix, 0x56),
            0x5E => self.op_ld_r_index_mem(prefix, 0x5E),
            0x66 => self.op_ld_r_index_mem(prefix, 0x66),
            0x6E => self.op_ld_r_index_mem(prefix, 0x6E),
            0x7E => self.op_ld_r_index_mem(prefix, 0x7E),

            0x60 => self.op_ld_index_byte_r(prefix, 0x60) + 4,
            0x61 => self.op_ld_index_byte_r(prefix, 0x61) + 4,
            0x62 => self.op_ld_index_byte_r(prefix, 0x62) + 4,
            0x63 => self.op_ld_index_byte_r(prefix, 0x63) + 4,
            0x67 => self.op_ld_index_byte_r(prefix, 0x67) + 4,
            0x68 => self.op_ld_index_byte_r(prefix, 0x68) + 4,
            0x69 => self.op_ld_index_byte_r(prefix, 0x69) + 4,
            0x6A => self.op_ld_index_byte_r(prefix, 0x6A) + 4,
            0x6B => self.op_ld_index_byte_r(prefix, 0x6B) + 4,
            0x6F => self.op_ld_index_byte_r(prefix, 0x6F) + 4,

            0x64 => self.op_ld_index_byte_index_byte(prefix, 0x64) + 4,
            0x65 => self.op_ld_index_byte_index_byte(prefix, 0x65) + 4,
            0x6C => self.op_ld_index_byte_index_byte(prefix, 0x6C) + 4,
            0x6D => self.op_ld_index_byte_index_byte(prefix, 0x6D) + 4,

            0x70 => self.op_ld_index_mem_r(prefix, 0x70),
            0x71 => self.op_ld_index_mem_r(prefix, 0x71),
            0x72 => self.op_ld_index_mem_r(prefix, 0x72),
            0x73 => self.op_ld_index_mem_r(prefix, 0x73),
            0x74 => self.op_ld_index_mem_r(prefix, 0x74),
            0x75 => self.op_ld_index_mem_r(prefix, 0x75),
            0x77 => self.op_ld_index_mem_r(prefix, 0x77),

            0x7C => self.op_ld_a_index_h(prefix) + 4,

            0x7D => self.op_ld_a_index_l(prefix) + 4,

            0x84 => self.op_add_a_index_h(prefix) + 4,

            0x85 => self.op_add_a_index_l(prefix) + 4,

            0x86 => self.op_add_a_index_mem(prefix),

            0x8C => self.op_adc_a_index_h(prefix) + 4,

            0x8D => self.op_adc_a_index_l(prefix) + 4,

            0x8E => self.op_adc_a_index_mem(prefix),

            0x94 => self.op_sub_a_index_h(prefix) + 4,

            0x95 => self.op_sub_a_index_l(prefix) + 4,

            0x96 => self.op_sub_a_index_mem(prefix),

            0x9C => self.op_sbc_a_index_h(prefix) + 4,

            0x9D => self.op_sbc_a_index_l(prefix) + 4,

            0x9E => self.op_sbc_a_index_mem(prefix),

            0xA4 => self.op_and_a_index_h(prefix) + 4,

            0xA5 => self.op_and_a_index_l(prefix) + 4,

            0xA6 => self.op_and_a_index_mem(prefix),

            0xAC => self.op_xor_a_index_h(prefix) + 4,

            0xAD => self.op_xor_a_index_l(prefix) + 4,

            0xAE => self.op_xor_a_index_mem(prefix),

            0xB4 => self.op_or_a_index_h(prefix) + 4,

            0xB5 => self.op_or_a_index_l(prefix) + 4,

            0xB6 => self.op_or_a_index_mem(prefix),

            0xBC => self.op_cp_a_index_h(prefix) + 4,

            0xBD => self.op_cp_a_index_l(prefix) + 4,

            0xBE => self.op_cp_a_index_mem(prefix),

            0xE1 => self.op_pop_index(prefix),

            0xE3 => self.op_ex_sp_index(prefix),

            0xE5 => self.op_push_index(prefix),

            0xE9 => self.op_jp_index(prefix),

            0xF9 => self.op_ld_sp_index(prefix),
        };
        Ok(t_states)
    }
}
