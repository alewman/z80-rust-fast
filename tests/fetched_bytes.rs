//! `last_instruction_bytes()` for instructions longer than the inline
//! buffer: a run of DD/FD prefixes has no length bound, and no conformance
//! manifest records an instruction longer than five bytes, so the spill
//! path is checked here.

use z80_rust::{Bus, Z80};

struct Flat(Box<[u8; 0x10000]>);

impl Bus for Flat {
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.0[usize::from(addr)]
    }
    fn write_byte(&mut self, addr: u16, value: u8) {
        self.0[usize::from(addr)] = value;
    }
    fn read_port(&mut self, _addr: u16) -> u8 {
        0xFF
    }
    fn write_port(&mut self, _addr: u16, _value: u8) {}
}

fn cpu_with(program: &[u8]) -> Z80<Flat> {
    let mut memory = Box::new([0u8; 0x10000]);
    memory[..program.len()].copy_from_slice(program);
    Z80::new(Flat(memory))
}

#[test]
fn a_long_prefix_run_is_recorded_whole() {
    // Ten DDs then LD IX,0x1234: 13 bytes, nine stray prefixes at 4 T-states
    // each on top of the instruction's 14.
    let mut program = vec![0xDD; 10];
    program.extend_from_slice(&[0x21, 0x34, 0x12]);
    program.extend_from_slice(&[0x3E, 0x2A]); // LD A,0x2A
    let mut cpu = cpu_with(&program);

    assert_eq!(cpu.step(), Ok(14 + 9 * 4));
    assert_eq!(cpu.ix, 0x1234);
    assert_eq!(cpu.last_instruction_bytes(), &program[..13]);

    // The next instruction is back on the inline path.
    assert_eq!(cpu.step(), Ok(7));
    assert_eq!(cpu.a, 0x2A);
    assert_eq!(cpu.last_instruction_bytes(), &[0x3E, 0x2A]);
}

#[test]
fn consecutive_long_runs_do_not_accumulate() {
    // Two runs in a row: the second must not carry the first's bytes.
    let mut program = vec![0xFD; 9];
    program.push(0x00); // NOP after eight strays: 10 bytes
    program.extend(vec![0xDD; 12]);
    program.push(0x23); // INC IX: 13 bytes
    let mut cpu = cpu_with(&program);

    assert_eq!(cpu.step(), Ok(4 + 4 + 8 * 4));
    assert_eq!(cpu.last_instruction_bytes(), &program[..10]);
    assert_eq!(cpu.step(), Ok(10 + 11 * 4));
    assert_eq!(cpu.last_instruction_bytes(), &program[10..23]);
    assert_eq!(cpu.ix, 1);
}

#[test]
fn exactly_the_inline_capacity_stays_inline_and_one_more_spills() {
    // 8 bytes: seven strays and NOP.
    let mut program = vec![0xDD; 7];
    program.push(0x00);
    // 9 bytes: eight strays and NOP.
    program.extend(vec![0xDD; 8]);
    program.push(0x00);
    let mut cpu = cpu_with(&program);
    assert_eq!(cpu.step(), Ok(8 + 6 * 4));
    assert_eq!(cpu.last_instruction_bytes(), &program[..8]);
    assert_eq!(cpu.step(), Ok(8 + 7 * 4));
    assert_eq!(cpu.last_instruction_bytes(), &program[8..17]);
}
