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
            0x00 => self.op_rot(0x00),
            0x01 => self.op_rot(0x01),
            0x02 => self.op_rot(0x02),
            0x03 => self.op_rot(0x03),
            0x04 => self.op_rot(0x04),
            0x05 => self.op_rot(0x05),
            0x06 => self.op_rot(0x06),
            0x07 => self.op_rot(0x07),
            0x08 => self.op_rot(0x08),
            0x09 => self.op_rot(0x09),
            0x0A => self.op_rot(0x0A),
            0x0B => self.op_rot(0x0B),
            0x0C => self.op_rot(0x0C),
            0x0D => self.op_rot(0x0D),
            0x0E => self.op_rot(0x0E),
            0x0F => self.op_rot(0x0F),
            0x10 => self.op_rot(0x10),
            0x11 => self.op_rot(0x11),
            0x12 => self.op_rot(0x12),
            0x13 => self.op_rot(0x13),
            0x14 => self.op_rot(0x14),
            0x15 => self.op_rot(0x15),
            0x16 => self.op_rot(0x16),
            0x17 => self.op_rot(0x17),
            0x18 => self.op_rot(0x18),
            0x19 => self.op_rot(0x19),
            0x1A => self.op_rot(0x1A),
            0x1B => self.op_rot(0x1B),
            0x1C => self.op_rot(0x1C),
            0x1D => self.op_rot(0x1D),
            0x1E => self.op_rot(0x1E),
            0x1F => self.op_rot(0x1F),
            0x20 => self.op_rot(0x20),
            0x21 => self.op_rot(0x21),
            0x22 => self.op_rot(0x22),
            0x23 => self.op_rot(0x23),
            0x24 => self.op_rot(0x24),
            0x25 => self.op_rot(0x25),
            0x26 => self.op_rot(0x26),
            0x27 => self.op_rot(0x27),
            0x28 => self.op_rot(0x28),
            0x29 => self.op_rot(0x29),
            0x2A => self.op_rot(0x2A),
            0x2B => self.op_rot(0x2B),
            0x2C => self.op_rot(0x2C),
            0x2D => self.op_rot(0x2D),
            0x2E => self.op_rot(0x2E),
            0x2F => self.op_rot(0x2F),
            0x30 => self.op_rot(0x30),
            0x31 => self.op_rot(0x31),
            0x32 => self.op_rot(0x32),
            0x33 => self.op_rot(0x33),
            0x34 => self.op_rot(0x34),
            0x35 => self.op_rot(0x35),
            0x36 => self.op_rot(0x36),
            0x37 => self.op_rot(0x37),
            0x38 => self.op_rot(0x38),
            0x39 => self.op_rot(0x39),
            0x3A => self.op_rot(0x3A),
            0x3B => self.op_rot(0x3B),
            0x3C => self.op_rot(0x3C),
            0x3D => self.op_rot(0x3D),
            0x3E => self.op_rot(0x3E),
            0x3F => self.op_rot(0x3F),

            0x40 => self.op_bit(0x40),
            0x41 => self.op_bit(0x41),
            0x42 => self.op_bit(0x42),
            0x43 => self.op_bit(0x43),
            0x44 => self.op_bit(0x44),
            0x45 => self.op_bit(0x45),
            0x46 => self.op_bit(0x46),
            0x47 => self.op_bit(0x47),
            0x48 => self.op_bit(0x48),
            0x49 => self.op_bit(0x49),
            0x4A => self.op_bit(0x4A),
            0x4B => self.op_bit(0x4B),
            0x4C => self.op_bit(0x4C),
            0x4D => self.op_bit(0x4D),
            0x4E => self.op_bit(0x4E),
            0x4F => self.op_bit(0x4F),
            0x50 => self.op_bit(0x50),
            0x51 => self.op_bit(0x51),
            0x52 => self.op_bit(0x52),
            0x53 => self.op_bit(0x53),
            0x54 => self.op_bit(0x54),
            0x55 => self.op_bit(0x55),
            0x56 => self.op_bit(0x56),
            0x57 => self.op_bit(0x57),
            0x58 => self.op_bit(0x58),
            0x59 => self.op_bit(0x59),
            0x5A => self.op_bit(0x5A),
            0x5B => self.op_bit(0x5B),
            0x5C => self.op_bit(0x5C),
            0x5D => self.op_bit(0x5D),
            0x5E => self.op_bit(0x5E),
            0x5F => self.op_bit(0x5F),
            0x60 => self.op_bit(0x60),
            0x61 => self.op_bit(0x61),
            0x62 => self.op_bit(0x62),
            0x63 => self.op_bit(0x63),
            0x64 => self.op_bit(0x64),
            0x65 => self.op_bit(0x65),
            0x66 => self.op_bit(0x66),
            0x67 => self.op_bit(0x67),
            0x68 => self.op_bit(0x68),
            0x69 => self.op_bit(0x69),
            0x6A => self.op_bit(0x6A),
            0x6B => self.op_bit(0x6B),
            0x6C => self.op_bit(0x6C),
            0x6D => self.op_bit(0x6D),
            0x6E => self.op_bit(0x6E),
            0x6F => self.op_bit(0x6F),
            0x70 => self.op_bit(0x70),
            0x71 => self.op_bit(0x71),
            0x72 => self.op_bit(0x72),
            0x73 => self.op_bit(0x73),
            0x74 => self.op_bit(0x74),
            0x75 => self.op_bit(0x75),
            0x76 => self.op_bit(0x76),
            0x77 => self.op_bit(0x77),
            0x78 => self.op_bit(0x78),
            0x79 => self.op_bit(0x79),
            0x7A => self.op_bit(0x7A),
            0x7B => self.op_bit(0x7B),
            0x7C => self.op_bit(0x7C),
            0x7D => self.op_bit(0x7D),
            0x7E => self.op_bit(0x7E),
            0x7F => self.op_bit(0x7F),

            0x80 => self.op_res(0x80),
            0x81 => self.op_res(0x81),
            0x82 => self.op_res(0x82),
            0x83 => self.op_res(0x83),
            0x84 => self.op_res(0x84),
            0x85 => self.op_res(0x85),
            0x86 => self.op_res(0x86),
            0x87 => self.op_res(0x87),
            0x88 => self.op_res(0x88),
            0x89 => self.op_res(0x89),
            0x8A => self.op_res(0x8A),
            0x8B => self.op_res(0x8B),
            0x8C => self.op_res(0x8C),
            0x8D => self.op_res(0x8D),
            0x8E => self.op_res(0x8E),
            0x8F => self.op_res(0x8F),
            0x90 => self.op_res(0x90),
            0x91 => self.op_res(0x91),
            0x92 => self.op_res(0x92),
            0x93 => self.op_res(0x93),
            0x94 => self.op_res(0x94),
            0x95 => self.op_res(0x95),
            0x96 => self.op_res(0x96),
            0x97 => self.op_res(0x97),
            0x98 => self.op_res(0x98),
            0x99 => self.op_res(0x99),
            0x9A => self.op_res(0x9A),
            0x9B => self.op_res(0x9B),
            0x9C => self.op_res(0x9C),
            0x9D => self.op_res(0x9D),
            0x9E => self.op_res(0x9E),
            0x9F => self.op_res(0x9F),
            0xA0 => self.op_res(0xA0),
            0xA1 => self.op_res(0xA1),
            0xA2 => self.op_res(0xA2),
            0xA3 => self.op_res(0xA3),
            0xA4 => self.op_res(0xA4),
            0xA5 => self.op_res(0xA5),
            0xA6 => self.op_res(0xA6),
            0xA7 => self.op_res(0xA7),
            0xA8 => self.op_res(0xA8),
            0xA9 => self.op_res(0xA9),
            0xAA => self.op_res(0xAA),
            0xAB => self.op_res(0xAB),
            0xAC => self.op_res(0xAC),
            0xAD => self.op_res(0xAD),
            0xAE => self.op_res(0xAE),
            0xAF => self.op_res(0xAF),
            0xB0 => self.op_res(0xB0),
            0xB1 => self.op_res(0xB1),
            0xB2 => self.op_res(0xB2),
            0xB3 => self.op_res(0xB3),
            0xB4 => self.op_res(0xB4),
            0xB5 => self.op_res(0xB5),
            0xB6 => self.op_res(0xB6),
            0xB7 => self.op_res(0xB7),
            0xB8 => self.op_res(0xB8),
            0xB9 => self.op_res(0xB9),
            0xBA => self.op_res(0xBA),
            0xBB => self.op_res(0xBB),
            0xBC => self.op_res(0xBC),
            0xBD => self.op_res(0xBD),
            0xBE => self.op_res(0xBE),
            0xBF => self.op_res(0xBF),

            0xC0 => self.op_set(0xC0),
            0xC1 => self.op_set(0xC1),
            0xC2 => self.op_set(0xC2),
            0xC3 => self.op_set(0xC3),
            0xC4 => self.op_set(0xC4),
            0xC5 => self.op_set(0xC5),
            0xC6 => self.op_set(0xC6),
            0xC7 => self.op_set(0xC7),
            0xC8 => self.op_set(0xC8),
            0xC9 => self.op_set(0xC9),
            0xCA => self.op_set(0xCA),
            0xCB => self.op_set(0xCB),
            0xCC => self.op_set(0xCC),
            0xCD => self.op_set(0xCD),
            0xCE => self.op_set(0xCE),
            0xCF => self.op_set(0xCF),
            0xD0 => self.op_set(0xD0),
            0xD1 => self.op_set(0xD1),
            0xD2 => self.op_set(0xD2),
            0xD3 => self.op_set(0xD3),
            0xD4 => self.op_set(0xD4),
            0xD5 => self.op_set(0xD5),
            0xD6 => self.op_set(0xD6),
            0xD7 => self.op_set(0xD7),
            0xD8 => self.op_set(0xD8),
            0xD9 => self.op_set(0xD9),
            0xDA => self.op_set(0xDA),
            0xDB => self.op_set(0xDB),
            0xDC => self.op_set(0xDC),
            0xDD => self.op_set(0xDD),
            0xDE => self.op_set(0xDE),
            0xDF => self.op_set(0xDF),
            0xE0 => self.op_set(0xE0),
            0xE1 => self.op_set(0xE1),
            0xE2 => self.op_set(0xE2),
            0xE3 => self.op_set(0xE3),
            0xE4 => self.op_set(0xE4),
            0xE5 => self.op_set(0xE5),
            0xE6 => self.op_set(0xE6),
            0xE7 => self.op_set(0xE7),
            0xE8 => self.op_set(0xE8),
            0xE9 => self.op_set(0xE9),
            0xEA => self.op_set(0xEA),
            0xEB => self.op_set(0xEB),
            0xEC => self.op_set(0xEC),
            0xED => self.op_set(0xED),
            0xEE => self.op_set(0xEE),
            0xEF => self.op_set(0xEF),
            0xF0 => self.op_set(0xF0),
            0xF1 => self.op_set(0xF1),
            0xF2 => self.op_set(0xF2),
            0xF3 => self.op_set(0xF3),
            0xF4 => self.op_set(0xF4),
            0xF5 => self.op_set(0xF5),
            0xF6 => self.op_set(0xF6),
            0xF7 => self.op_set(0xF7),
            0xF8 => self.op_set(0xF8),
            0xF9 => self.op_set(0xF9),
            0xFA => self.op_set(0xFA),
            0xFB => self.op_set(0xFB),
            0xFC => self.op_set(0xFC),
            0xFD => self.op_set(0xFD),
            0xFE => self.op_set(0xFE),
            0xFF => self.op_set(0xFF),
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

            0xDD => {
                let sub_opcode = self.fetch_byte();
                return self.execute_index(0xDD, sub_opcode);
            }
            0xFD => {
                let sub_opcode = self.fetch_byte();
                return self.execute_index(0xFD, sub_opcode);
            }

            0x00 => self.op_nop(),

            0x01 => self.op_ld_rr_nn(0x01),
            0x11 => self.op_ld_rr_nn(0x11),
            0x21 => self.op_ld_rr_nn(0x21),
            0x31 => self.op_ld_rr_nn(0x31),

            0x02 => self.op_ld_irr_a(self.bc()),

            0x12 => self.op_ld_irr_a(self.de()),

            0x03 => self.op_inc_rr(0x03),
            0x13 => self.op_inc_rr(0x13),
            0x23 => self.op_inc_rr(0x23),
            0x33 => self.op_inc_rr(0x33),

            0x04 => self.op_inc_r(0x04),
            0x0C => self.op_inc_r(0x0C),
            0x14 => self.op_inc_r(0x14),
            0x1C => self.op_inc_r(0x1C),
            0x24 => self.op_inc_r(0x24),
            0x2C => self.op_inc_r(0x2C),
            0x3C => self.op_inc_r(0x3C),

            0x05 => self.op_dec_r(0x05),
            0x0D => self.op_dec_r(0x0D),
            0x15 => self.op_dec_r(0x15),
            0x1D => self.op_dec_r(0x1D),
            0x25 => self.op_dec_r(0x25),
            0x2D => self.op_dec_r(0x2D),
            0x3D => self.op_dec_r(0x3D),

            0x06 => self.op_ld_r_n(0x06),
            0x0E => self.op_ld_r_n(0x0E),
            0x16 => self.op_ld_r_n(0x16),
            0x1E => self.op_ld_r_n(0x1E),
            0x26 => self.op_ld_r_n(0x26),
            0x2E => self.op_ld_r_n(0x2E),
            0x36 => self.op_ld_r_n(0x36),
            0x3E => self.op_ld_r_n(0x3E),

            0x07 => self.op_rlca(),

            0x08 => self.op_ex_af_af(),

            0x09 => self.op_add_hl_rr(0x09),
            0x19 => self.op_add_hl_rr(0x19),
            0x29 => self.op_add_hl_rr(0x29),
            0x39 => self.op_add_hl_rr(0x39),

            0x0A => self.op_ld_a_irr(self.bc()),

            0x1A => self.op_ld_a_irr(self.de()),

            0x0B => self.op_dec_rr(0x0B),
            0x1B => self.op_dec_rr(0x1B),
            0x2B => self.op_dec_rr(0x2B),
            0x3B => self.op_dec_rr(0x3B),

            0x0F => self.op_rrca(),

            0x10 => self.op_djnz(),

            0x17 => self.op_rla(),

            0x18 => self.op_jr(0x18),
            0x20 => self.op_jr(0x20),
            0x28 => self.op_jr(0x28),
            0x30 => self.op_jr(0x30),
            0x38 => self.op_jr(0x38),

            0x1F => self.op_rra(),

            0x22 => self.op_ld_nn_hl(),

            0x27 => self.op_daa(),

            0x2A => self.op_ld_hl_nn_from_mem(),

            0x2F => self.op_cpl(),

            0x32 => self.op_ld_inn_a(),

            0x34 => self.op_inc_hl(),

            0x35 => self.op_dec_hl(),

            0x37 => self.op_scf_ccf(0x37, false),
            0x3F => self.op_scf_ccf(0x3F, false),

            0x3A => self.op_ld_a_inn(),

            0x76 => self.op_halt(),

            0x40 => self.op_ld_r_r(0x40),
            0x41 => self.op_ld_r_r(0x41),
            0x42 => self.op_ld_r_r(0x42),
            0x43 => self.op_ld_r_r(0x43),
            0x44 => self.op_ld_r_r(0x44),
            0x45 => self.op_ld_r_r(0x45),
            0x46 => self.op_ld_r_r(0x46),
            0x47 => self.op_ld_r_r(0x47),
            0x48 => self.op_ld_r_r(0x48),
            0x49 => self.op_ld_r_r(0x49),
            0x4A => self.op_ld_r_r(0x4A),
            0x4B => self.op_ld_r_r(0x4B),
            0x4C => self.op_ld_r_r(0x4C),
            0x4D => self.op_ld_r_r(0x4D),
            0x4E => self.op_ld_r_r(0x4E),
            0x4F => self.op_ld_r_r(0x4F),
            0x50 => self.op_ld_r_r(0x50),
            0x51 => self.op_ld_r_r(0x51),
            0x52 => self.op_ld_r_r(0x52),
            0x53 => self.op_ld_r_r(0x53),
            0x54 => self.op_ld_r_r(0x54),
            0x55 => self.op_ld_r_r(0x55),
            0x56 => self.op_ld_r_r(0x56),
            0x57 => self.op_ld_r_r(0x57),
            0x58 => self.op_ld_r_r(0x58),
            0x59 => self.op_ld_r_r(0x59),
            0x5A => self.op_ld_r_r(0x5A),
            0x5B => self.op_ld_r_r(0x5B),
            0x5C => self.op_ld_r_r(0x5C),
            0x5D => self.op_ld_r_r(0x5D),
            0x5E => self.op_ld_r_r(0x5E),
            0x5F => self.op_ld_r_r(0x5F),
            0x60 => self.op_ld_r_r(0x60),
            0x61 => self.op_ld_r_r(0x61),
            0x62 => self.op_ld_r_r(0x62),
            0x63 => self.op_ld_r_r(0x63),
            0x64 => self.op_ld_r_r(0x64),
            0x65 => self.op_ld_r_r(0x65),
            0x66 => self.op_ld_r_r(0x66),
            0x67 => self.op_ld_r_r(0x67),
            0x68 => self.op_ld_r_r(0x68),
            0x69 => self.op_ld_r_r(0x69),
            0x6A => self.op_ld_r_r(0x6A),
            0x6B => self.op_ld_r_r(0x6B),
            0x6C => self.op_ld_r_r(0x6C),
            0x6D => self.op_ld_r_r(0x6D),
            0x6E => self.op_ld_r_r(0x6E),
            0x6F => self.op_ld_r_r(0x6F),
            0x70 => self.op_ld_r_r(0x70),
            0x71 => self.op_ld_r_r(0x71),
            0x72 => self.op_ld_r_r(0x72),
            0x73 => self.op_ld_r_r(0x73),
            0x74 => self.op_ld_r_r(0x74),
            0x75 => self.op_ld_r_r(0x75),
            0x77 => self.op_ld_r_r(0x77),
            0x78 => self.op_ld_r_r(0x78),
            0x79 => self.op_ld_r_r(0x79),
            0x7A => self.op_ld_r_r(0x7A),
            0x7B => self.op_ld_r_r(0x7B),
            0x7C => self.op_ld_r_r(0x7C),
            0x7D => self.op_ld_r_r(0x7D),
            0x7E => self.op_ld_r_r(0x7E),
            0x7F => self.op_ld_r_r(0x7F),

            0x80 => self.op_alu_r(0x80),
            0x81 => self.op_alu_r(0x81),
            0x82 => self.op_alu_r(0x82),
            0x83 => self.op_alu_r(0x83),
            0x84 => self.op_alu_r(0x84),
            0x85 => self.op_alu_r(0x85),
            0x86 => self.op_alu_r(0x86),
            0x87 => self.op_alu_r(0x87),
            0x88 => self.op_alu_r(0x88),
            0x89 => self.op_alu_r(0x89),
            0x8A => self.op_alu_r(0x8A),
            0x8B => self.op_alu_r(0x8B),
            0x8C => self.op_alu_r(0x8C),
            0x8D => self.op_alu_r(0x8D),
            0x8E => self.op_alu_r(0x8E),
            0x8F => self.op_alu_r(0x8F),
            0x90 => self.op_alu_r(0x90),
            0x91 => self.op_alu_r(0x91),
            0x92 => self.op_alu_r(0x92),
            0x93 => self.op_alu_r(0x93),
            0x94 => self.op_alu_r(0x94),
            0x95 => self.op_alu_r(0x95),
            0x96 => self.op_alu_r(0x96),
            0x97 => self.op_alu_r(0x97),
            0x98 => self.op_alu_r(0x98),
            0x99 => self.op_alu_r(0x99),
            0x9A => self.op_alu_r(0x9A),
            0x9B => self.op_alu_r(0x9B),
            0x9C => self.op_alu_r(0x9C),
            0x9D => self.op_alu_r(0x9D),
            0x9E => self.op_alu_r(0x9E),
            0x9F => self.op_alu_r(0x9F),
            0xA0 => self.op_alu_r(0xA0),
            0xA1 => self.op_alu_r(0xA1),
            0xA2 => self.op_alu_r(0xA2),
            0xA3 => self.op_alu_r(0xA3),
            0xA4 => self.op_alu_r(0xA4),
            0xA5 => self.op_alu_r(0xA5),
            0xA6 => self.op_alu_r(0xA6),
            0xA7 => self.op_alu_r(0xA7),
            0xA8 => self.op_alu_r(0xA8),
            0xA9 => self.op_alu_r(0xA9),
            0xAA => self.op_alu_r(0xAA),
            0xAB => self.op_alu_r(0xAB),
            0xAC => self.op_alu_r(0xAC),
            0xAD => self.op_alu_r(0xAD),
            0xAE => self.op_alu_r(0xAE),
            0xAF => self.op_alu_r(0xAF),
            0xB0 => self.op_alu_r(0xB0),
            0xB1 => self.op_alu_r(0xB1),
            0xB2 => self.op_alu_r(0xB2),
            0xB3 => self.op_alu_r(0xB3),
            0xB4 => self.op_alu_r(0xB4),
            0xB5 => self.op_alu_r(0xB5),
            0xB6 => self.op_alu_r(0xB6),
            0xB7 => self.op_alu_r(0xB7),
            0xB8 => self.op_alu_r(0xB8),
            0xB9 => self.op_alu_r(0xB9),
            0xBA => self.op_alu_r(0xBA),
            0xBB => self.op_alu_r(0xBB),
            0xBC => self.op_alu_r(0xBC),
            0xBD => self.op_alu_r(0xBD),
            0xBE => self.op_alu_r(0xBE),
            0xBF => self.op_alu_r(0xBF),

            0xC0 => self.op_ret_cc(0xC0),
            0xC8 => self.op_ret_cc(0xC8),
            0xD0 => self.op_ret_cc(0xD0),
            0xD8 => self.op_ret_cc(0xD8),
            0xE0 => self.op_ret_cc(0xE0),
            0xE8 => self.op_ret_cc(0xE8),
            0xF0 => self.op_ret_cc(0xF0),
            0xF8 => self.op_ret_cc(0xF8),

            0xC1 => self.op_pop_rr(0xC1),
            0xD1 => self.op_pop_rr(0xD1),
            0xE1 => self.op_pop_rr(0xE1),

            0xC2 => self.op_jp(0xC2),
            0xCA => self.op_jp(0xCA),
            0xD2 => self.op_jp(0xD2),
            0xDA => self.op_jp(0xDA),
            0xE2 => self.op_jp(0xE2),
            0xEA => self.op_jp(0xEA),
            0xF2 => self.op_jp(0xF2),
            0xFA => self.op_jp(0xFA),
            0xC3 => self.op_jp(0xC3),

            0xC4 => self.op_call(0xC4),
            0xCC => self.op_call(0xCC),
            0xD4 => self.op_call(0xD4),
            0xDC => self.op_call(0xDC),
            0xE4 => self.op_call(0xE4),
            0xEC => self.op_call(0xEC),
            0xF4 => self.op_call(0xF4),
            0xFC => self.op_call(0xFC),
            0xCD => self.op_call(0xCD),

            0xC5 => self.op_push_rr(0xC5),
            0xD5 => self.op_push_rr(0xD5),
            0xE5 => self.op_push_rr(0xE5),

            0xC6 => self.op_alu_n(0xC6),
            0xCE => self.op_alu_n(0xCE),
            0xD6 => self.op_alu_n(0xD6),
            0xDE => self.op_alu_n(0xDE),
            0xE6 => self.op_alu_n(0xE6),
            0xEE => self.op_alu_n(0xEE),
            0xF6 => self.op_alu_n(0xF6),
            0xFE => self.op_alu_n(0xFE),

            0xC7 => self.op_rst(0xC7),
            0xCF => self.op_rst(0xCF),
            0xD7 => self.op_rst(0xD7),
            0xDF => self.op_rst(0xDF),
            0xE7 => self.op_rst(0xE7),
            0xEF => self.op_rst(0xEF),
            0xF7 => self.op_rst(0xF7),
            0xFF => self.op_rst(0xFF),

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
            0x40 => self.op_in_r_c(0x40),
            0x48 => self.op_in_r_c(0x48),
            0x50 => self.op_in_r_c(0x50),
            0x58 => self.op_in_r_c(0x58),
            0x60 => self.op_in_r_c(0x60),
            0x68 => self.op_in_r_c(0x68),
            0x70 => self.op_in_r_c(0x70),
            0x78 => self.op_in_r_c(0x78),

            0x41 => self.op_out_c_r(0x41),
            0x49 => self.op_out_c_r(0x49),
            0x51 => self.op_out_c_r(0x51),
            0x59 => self.op_out_c_r(0x59),
            0x61 => self.op_out_c_r(0x61),
            0x69 => self.op_out_c_r(0x69),
            0x71 => self.op_out_c_r(0x71),
            0x79 => self.op_out_c_r(0x79),

            0x42 => self.op_sbc_hl_rr(0x42),
            0x52 => self.op_sbc_hl_rr(0x52),
            0x62 => self.op_sbc_hl_rr(0x62),
            0x72 => self.op_sbc_hl_rr(0x72),

            0x43 => self.op_ld_nn_rr((0x43 >> 4) & 0x03),
            0x53 => self.op_ld_nn_rr((0x53 >> 4) & 0x03),
            0x63 => self.op_ld_nn_rr((0x63 >> 4) & 0x03),
            0x73 => self.op_ld_nn_rr((0x73 >> 4) & 0x03),

            0x44 => self.op_neg(),
            0x4C => self.op_neg(),
            0x54 => self.op_neg(),
            0x5C => self.op_neg(),
            0x64 => self.op_neg(),
            0x6C => self.op_neg(),
            0x74 => self.op_neg(),
            0x7C => self.op_neg(),

            0x45 => self.op_retn(),
            0x55 => self.op_retn(),
            0x65 => self.op_retn(),
            0x75 => self.op_retn(),

            0x46 => self.op_im(0),
            0x4E => self.op_im(0),
            0x66 => self.op_im(0),
            0x6E => self.op_im(0),

            0x47 => self.op_ld_i_a(),

            0x4A => self.op_adc_hl_rr(0x4A),
            0x5A => self.op_adc_hl_rr(0x5A),
            0x6A => self.op_adc_hl_rr(0x6A),
            0x7A => self.op_adc_hl_rr(0x7A),

            0x4B => self.op_ld_rr_nn_from_mem((0x4B >> 4) & 0x03),
            0x5B => self.op_ld_rr_nn_from_mem((0x5B >> 4) & 0x03),
            0x6B => self.op_ld_rr_nn_from_mem((0x6B >> 4) & 0x03),
            0x7B => self.op_ld_rr_nn_from_mem((0x7B >> 4) & 0x03),

            0x4D => self.op_reti(),
            0x5D => self.op_reti(),
            0x6D => self.op_reti(),
            0x7D => self.op_reti(),

            0x4F => self.op_ld_r_a(),

            0x56 => self.op_im(1),
            0x76 => self.op_im(1),

            0x57 => self.op_ld_a_i(),

            0x5E => self.op_im(2),
            0x7E => self.op_im(2),

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
