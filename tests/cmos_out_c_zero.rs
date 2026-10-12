//! The opt-in CMOS OUT (C),0 (z80-python 80f88a4): ED 71 drives 0 on NMOS
//! parts, the default the oracles check, and 0xFF when the host sets
//! `cmos_out_c_zero`. No manifest can carry the flag, so it is tested here.

use z80_rust::{Bus, Z80};

struct Flat {
    memory: Vec<u8>,
    outs: Vec<(u16, u8)>,
}

impl Bus for Flat {
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.memory[usize::from(addr)]
    }
    fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[usize::from(addr)] = value;
    }
    fn read_port(&mut self, _addr: u16) -> u8 {
        0xFF
    }
    fn write_port(&mut self, addr: u16, value: u8) {
        self.outs.push((addr, value));
    }
}

fn out_c_zero(cmos: bool) -> Vec<(u16, u8)> {
    let mut memory = vec![0; 0x10000];
    memory[..2].copy_from_slice(&[0xED, 0x71]);
    let mut cpu = Z80::new(Flat {
        memory,
        outs: Vec::new(),
    });
    cpu.b = 0x12;
    cpu.c = 0x34;
    cpu.cmos_out_c_zero = cmos;
    assert_eq!(cpu.step(), Ok(12));
    cpu.bus.outs
}

#[test]
fn nmos_drives_zero_by_default_and_cmos_drives_ff_when_asked() {
    assert_eq!(out_c_zero(false), [(0x1234, 0x00)]);
    assert_eq!(out_c_zero(true), [(0x1234, 0xFF)]);
}
