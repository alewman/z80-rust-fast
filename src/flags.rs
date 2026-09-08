//! Z80 flag-register implementation and bit masks.
//!
//! Transcribed from `z80_python/_flags.py`. `Flags` is one byte with named
//! accessors; every accessor returns or takes a 0/1 value exactly as the
//! Python properties do, and `set_xy` copies bits 3 and 5 of its argument.

pub const FLAG_S: u8 = 0b1000_0000;
pub const FLAG_Z: u8 = 0b0100_0000;
pub const FLAG_Y: u8 = 0b0010_0000;
pub const FLAG_H: u8 = 0b0001_0000;
pub const FLAG_X: u8 = 0b0000_1000;
pub const FLAG_PV: u8 = 0b0000_0100;
pub const FLAG_N: u8 = 0b0000_0010;
pub const FLAG_C: u8 = 0b0000_0001;

/// S, Z, X, Y, and PV-as-parity for every 8-bit result: the flags that
/// depend only on the result byte, ready to OR with the ones that do not.
/// `SZP[v] & !FLAG_PV` is the same without parity, for the arithmetic
/// operations whose PV is overflow.
pub const SZP: [u8; 256] = build_szp();

const fn build_szp() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut value = 0usize;
    while value < 256 {
        let byte = value as u8;
        let mut flags = byte & (FLAG_S | FLAG_X | FLAG_Y);
        if byte == 0 {
            flags |= FLAG_Z;
        }
        if (byte.count_ones() & 1) == 0 {
            flags |= FLAG_PV;
        }
        table[value] = flags;
        value += 1;
    }
    table
}

/// The Z80 F register, including the undocumented X/Y bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    byte: u8,
}

impl Flags {
    pub const fn new(value: u8) -> Self {
        Flags { byte: value }
    }

    pub const fn byte(self) -> u8 {
        self.byte
    }

    pub fn set_byte(&mut self, value: u8) {
        self.byte = value;
    }

    pub const fn s(self) -> u8 {
        (self.byte >> 7) & 1
    }

    pub fn set_s(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_S) | ((value & 1) << 7);
    }

    pub const fn z(self) -> u8 {
        (self.byte >> 6) & 1
    }

    pub fn set_z(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_Z) | ((value & 1) << 6);
    }

    pub const fn y(self) -> u8 {
        (self.byte >> 5) & 1
    }

    pub fn set_y(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_Y) | ((value & 1) << 5);
    }

    pub const fn h(self) -> u8 {
        (self.byte >> 4) & 1
    }

    pub fn set_h(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_H) | ((value & 1) << 4);
    }

    pub const fn x(self) -> u8 {
        (self.byte >> 3) & 1
    }

    pub fn set_x(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_X) | ((value & 1) << 3);
    }

    pub const fn pv(self) -> u8 {
        (self.byte >> 2) & 1
    }

    pub fn set_pv(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_PV) | ((value & 1) << 2);
    }

    pub const fn n(self) -> u8 {
        (self.byte >> 1) & 1
    }

    pub fn set_n(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_N) | ((value & 1) << 1);
    }

    pub const fn c(self) -> u8 {
        self.byte & 1
    }

    pub fn set_c(&mut self, value: u8) {
        self.byte = (self.byte & !FLAG_C) | (value & 1);
    }

    /// Copy bits 3 and 5 of `value` into X and Y.
    pub fn set_xy(&mut self, value: u8) {
        self.byte = (self.byte & !(FLAG_X | FLAG_Y)) | (value & (FLAG_X | FLAG_Y));
    }
}
