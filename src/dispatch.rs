//! Top-level, base, CB, and ED opcode dispatch.
//!
//! Transcribed from `z80_python/_dispatch.py`, then reshaped: the reference
//! dispatches with an if-chain so every opcode is one grep away; here each
//! level is one exhaustive `match`, which keeps that property (every opcode
//! byte is named) and compiles to a jump table.

use crate::core::{Bus, Fault, Z80};

impl<B: Bus> Z80<B> {
    /// Fetch and execute one instruction, returning its documented T-states.
    ///
    /// This is the instruction-only entry point; [`Z80::step`] is the host
    /// entry point that services lifecycle requests first.
    pub fn decode_and_execute(&mut self) -> Result<u32, Fault> {
        self.fetched_len = 0;
        let opcode = self.fetch_byte();
        self.execute_main(opcode)
    }

    pub(crate) fn execute_cb(&mut self, sub_opcode: u8) -> u32 {
        match sub_opcode {
            0x00..=0x3F => self.op_rot(sub_opcode),
            0x40..=0x7F => self.op_bit(sub_opcode),
            0x80..=0xBF => self.op_res(sub_opcode),
            0xC0..=0xFF => self.op_set(sub_opcode),
        }
    }

    /// Dispatch one fetched opcode byte, prefixes included.
    ///
    /// One exhaustive `match` over all 256 values, so the compiler emits a
    /// jump table instead of the reference's if-chain (which tested up to
    /// forty conditions per instruction). Every arm calls the same handler
    /// with the same argument the if-chain did; the prefix arms are the
    /// tests `decode_and_execute` used to make first. No `_` arm: adding an
    /// opcode anywhere is a compile error until it is routed.
    pub(crate) fn execute_main(&mut self, opcode: u8) -> Result<u32, Fault> {
        let t_states = match opcode {
            // Prefixes.
            0xCB => {
                let sub_opcode = self.fetch_byte();
                self.execute_cb(sub_opcode)
            }
            0xED => {
                let sub_opcode = self.fetch_byte();
                self.execute_ed(sub_opcode)
            }
            0xDD | 0xFD => {
                let sub_opcode = self.fetch_byte();
                return self.execute_index(opcode, sub_opcode);
            }

            0x00 => self.op_nop(),
            0x01 | 0x11 | 0x21 | 0x31 => self.op_ld_rr_nn(opcode),
            0x02 => self.op_ld_irr_a(self.bc()),
            0x12 => self.op_ld_irr_a(self.de()),
            0x03 | 0x13 | 0x23 | 0x33 => self.op_inc_rr(opcode),
            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x3C => self.op_inc_r(opcode),
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x3D => self.op_dec_r(opcode),
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => self.op_ld_r_n(opcode),
            0x07 => self.op_rlca(),
            0x08 => self.op_ex_af_af(),
            0x09 | 0x19 | 0x29 | 0x39 => self.op_add_hl_rr(opcode),
            0x0A => self.op_ld_a_irr(self.bc()),
            0x1A => self.op_ld_a_irr(self.de()),
            0x0B | 0x1B | 0x2B | 0x3B => self.op_dec_rr(opcode),
            0x0F => self.op_rrca(),
            0x10 => self.op_djnz(),
            0x17 => self.op_rla(),
            0x18 | 0x20 | 0x28 | 0x30 | 0x38 => self.op_jr(opcode),
            0x1F => self.op_rra(),
            0x22 => self.op_ld_nn_hl(),
            0x27 => self.op_daa(),
            0x2A => self.op_ld_hl_nn_from_mem(),
            0x2F => self.op_cpl(),
            0x32 => self.op_ld_inn_a(),
            0x34 => self.op_inc_hl(),
            0x35 => self.op_dec_hl(),
            0x37 | 0x3F => self.op_scf_ccf(opcode, false),
            0x3A => self.op_ld_a_inn(),

            0x76 => self.op_halt(),
            0x40..=0x75 | 0x77..=0x7F => self.op_ld_r_r(opcode),
            0x80..=0xBF => self.op_alu_r(opcode),

            0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xE0 | 0xE8 | 0xF0 | 0xF8 => self.op_ret_cc(opcode),
            0xC1 | 0xD1 | 0xE1 => self.op_pop_rr(opcode),
            0xC2 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA | 0xC3 => self.op_jp(opcode),
            0xC4 | 0xCC | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC | 0xCD => self.op_call(opcode),
            0xC5 | 0xD5 | 0xE5 => self.op_push_rr(opcode),
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => self.op_alu_n(opcode),
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => self.op_rst(opcode),
            0xC9 => self.op_ret(),
            0xD3 => self.op_out_n_a(),
            0xD9 => self.op_exx(),
            0xDB => self.op_in_a_n(),
            0xE3 => self.op_ex_sp_hl(),
            0xE9 => self.op_jp_hl(),
            0xEB => self.op_ex_de_hl(),
            0xF1 => self.op_pop_af(),
            0xF3 => self.op_interrupt_enable(false),
            0xF5 => self.op_push_af(),
            0xF9 => self.op_ld_sp_hl(),
            0xFB => self.op_interrupt_enable(true),
        };
        Ok(t_states)
    }

    /// Dispatch the byte after an ED prefix: one `match`, every defined
    /// opcode named, and the `_` arm is the reference's rule that every
    /// other ED byte is a 2-byte, 8 T-state no-op on real silicon.
    #[inline(never)]
    pub(crate) fn execute_ed(&mut self, opcode: u8) -> u32 {
        match opcode {
            0x40 | 0x48 | 0x50 | 0x58 | 0x60 | 0x68 | 0x70 | 0x78 => self.op_in_r_c(opcode),
            0x41 | 0x49 | 0x51 | 0x59 | 0x61 | 0x69 | 0x71 | 0x79 => self.op_out_c_r(opcode),
            0x42 | 0x52 | 0x62 | 0x72 => self.op_sbc_hl_rr(opcode),
            0x43 | 0x53 | 0x63 | 0x73 => self.op_ld_nn_rr((opcode >> 4) & 0x03),
            0x44 | 0x4C | 0x54 | 0x5C | 0x64 | 0x6C | 0x74 | 0x7C => self.op_neg(),
            0x45 | 0x55 | 0x65 | 0x75 => self.op_retn(),
            0x46 | 0x4E | 0x66 | 0x6E => self.op_im(0),
            0x47 => self.op_ld_i_a(),
            0x4A | 0x5A | 0x6A | 0x7A => self.op_adc_hl_rr(opcode),
            0x4B | 0x5B | 0x6B | 0x7B => self.op_ld_rr_nn_from_mem((opcode >> 4) & 0x03),
            0x4D | 0x5D | 0x6D | 0x7D => self.op_reti(),
            0x4F => self.op_ld_r_a(),
            0x56 | 0x76 => self.op_im(1),
            0x57 => self.op_ld_a_i(),
            0x5E | 0x7E => self.op_im(2),
            0x5F => self.op_ld_a_r(),
            0x67 => self.op_rrd(),
            0x6F => self.op_rld(),
            0xA0 => self.op_ldi(),
            0xA1 => self.op_cpi(),
            0xA2 => self.op_ini(),
            0xA3 => self.op_outi(),
            0xA8 => self.op_ldd(),
            0xA9 => self.op_cpd(),
            0xAA => self.op_ind(),
            0xAB => self.op_outd(),
            0xB0 => self.op_ldir(),
            0xB1 => self.op_cpir(),
            0xB2 => self.op_inir(),
            0xB3 => self.op_otir(),
            0xB8 => self.op_lddr(),
            0xB9 => self.op_cpdr(),
            0xBA => self.op_indr(),
            0xBB => self.op_otdr(),
            // Every ED-prefixed byte not otherwise defined is a genuine Z80
            // instruction on real silicon: a 2-byte, 8 T-state no-op. That
            // includes 0x77 and 0x7F inside the documented 0x40-0x7F block.
            _ => self.op_ed_nop(),
        }
    }
}
