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
            0xDD | 0xED | 0xFD => {
                return Err(Fault::UnhandledIndexOpcode {
                    prefix,
                    opcode: sub_opcode,
                    pc: self.pc.wrapping_sub(2),
                })
            }

            // Base-set instructions the prefix does not modify: their own
            // handler plus the prefix's 4 T-states.
            0x00 => self.op_nop() + 4,
            0x01 | 0x11 | 0x31 => self.op_ld_rr_nn(sub_opcode) + 4,
            0x02 => self.op_ld_irr_a(self.bc()) + 4,
            0x12 => self.op_ld_irr_a(self.de()) + 4,
            0x03 | 0x13 | 0x33 => self.op_inc_rr(sub_opcode) + 4,
            0x04 | 0x0C | 0x14 | 0x1C | 0x3C => self.op_inc_r(sub_opcode) + 4,
            0x05 | 0x0D | 0x15 | 0x1D | 0x3D => self.op_dec_r(sub_opcode) + 4,
            0x06 | 0x0E | 0x16 | 0x1E | 0x3E => self.op_ld_r_n(sub_opcode) + 4,
            0x07 => self.op_rlca() + 4,
            0x08 => self.op_ex_af_af() + 4,
            0x0A => self.op_ld_a_irr(self.bc()) + 4,
            0x1A => self.op_ld_a_irr(self.de()) + 4,
            0x0B | 0x1B | 0x3B => self.op_dec_rr(sub_opcode) + 4,
            0x0F => self.op_rrca() + 4,
            0x10 => self.op_djnz() + 4,
            0x17 => self.op_rla() + 4,
            0x18 | 0x20 | 0x28 | 0x30 | 0x38 => self.op_jr(sub_opcode) + 4,
            0x1F => self.op_rra() + 4,
            0x27 => self.op_daa() + 4,
            0x2F => self.op_cpl() + 4,
            0x32 => self.op_ld_inn_a() + 4,
            0x37 | 0x3F => self.op_scf_ccf(sub_opcode, true) + 4,
            0x3A => self.op_ld_a_inn() + 4,
            0x76 => self.op_halt() + 4,
            0x40..=0x43 | 0x47..=0x4B | 0x4F..=0x53 | 0x57..=0x5B | 0x5F | 0x78..=0x7B | 0x7F => {
                self.op_ld_r_r(sub_opcode) + 4
            }
            // plain_alu: every 8-bit ALU form whose operand is not H, L, or (HL).
            0x80..=0x83
            | 0x87
            | 0x88..=0x8B
            | 0x8F
            | 0x90..=0x93
            | 0x97
            | 0x98..=0x9B
            | 0x9F
            | 0xA0..=0xA3
            | 0xA7
            | 0xA8..=0xAB
            | 0xAF
            | 0xB0..=0xB3
            | 0xB7
            | 0xB8..=0xBB
            | 0xBF => self.op_alu_r(sub_opcode) + 4,
            0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xE0 | 0xE8 | 0xF0 | 0xF8 => self.op_ret_cc(sub_opcode) + 4,
            0xC1 | 0xD1 => self.op_pop_rr(sub_opcode) + 4,
            0xC2 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA | 0xC3 => {
                self.op_jp(sub_opcode) + 4
            }
            0xC4 | 0xCC | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC | 0xCD => {
                self.op_call(sub_opcode) + 4
            }
            0xC5 | 0xD5 => self.op_push_rr(sub_opcode) + 4,
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => self.op_alu_n(sub_opcode) + 4,
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => self.op_rst(sub_opcode) + 4,
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
            0x09 | 0x19 | 0x29 | 0x39 => self.op_add_index_rr(prefix, sub_opcode),
            0x21 => self.op_ld_index_nn(prefix),
            0x22 => self.op_ld_nn_index(prefix),
            0x23 => self.op_inc_index(prefix),
            0x24 | 0x25 | 0x2C | 0x2D => self.op_inc_dec_index_byte(prefix, sub_opcode) + 4,
            0x26 | 0x2E => self.op_ld_index_byte_n(prefix, sub_opcode) + 4,
            0x2A => self.op_ld_index_nn_from_mem(prefix),
            0x2B => self.op_dec_index(prefix),
            0x34 => self.op_inc_index_mem(prefix),
            0x35 => self.op_dec_index_mem(prefix),
            0x36 => self.op_ld_index_mem_n(prefix),
            0x44 | 0x45 | 0x4C | 0x4D | 0x54 | 0x55 | 0x5C | 0x5D => {
                self.op_ld_r_index_byte(prefix, sub_opcode) + 4
            }
            0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x7E => {
                self.op_ld_r_index_mem(prefix, sub_opcode)
            }
            0x60..=0x63 | 0x67..=0x6B | 0x6F => self.op_ld_index_byte_r(prefix, sub_opcode) + 4,
            0x64 | 0x65 | 0x6C | 0x6D => self.op_ld_index_byte_index_byte(prefix, sub_opcode) + 4,
            0x70..=0x75 | 0x77 => self.op_ld_index_mem_r(prefix, sub_opcode),
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
