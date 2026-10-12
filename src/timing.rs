//! Where inside an instruction each of its memory and I/O accesses falls.
//!
//! Transcribed from `z80_python/timing.py`.
//!
//! `step()` reports an instruction's total T-states. A board that shares its
//! bus with another processor (the Mega Drive's arbiter, for one) also needs
//! to know *when* inside the instruction each access happens. The core does
//! not count T-states as it goes, so this module answers from a table
//! instead: [`instruction_timing`] takes the instruction's bytes and returns
//! every access it can make, in order, each with its T-state offset.
//!
//! The table (`access_timing.rs`) is the reference's, transcribed by
//! `scripts/gen_access_timing.py`; the reference derives it from the
//! SingleStepTests corpus and refuses to write one unless every case of an
//! instruction agrees. That corpus is emulator-derived (z80-python
//! docs/validation.md, "Oracle tiers"), and so is this table. Rung 2 checks
//! this core against it on every corpus case: the accesses `step()` makes
//! are the table's, in the same order and of the same kinds, and its
//! T-states are one of the table's lengths. Rung 7 checks this module
//! answers as the reference's does.
//!
//! **Offsets.** The corpus samples the bus once per T-state, between
//! T-states. An access's `t` is the index of the first sample that shows it
//! (RD or WR with MREQ or IORQ), counted from the instruction's first
//! T-state; every opcode fetch has `t == 1`, and an ordinary memory read
//! after it `t == 5`.
//!
//! **Conditional and repeating instructions** have two shapes: `CALL cc`
//! makes five accesses when taken and three when not; `LDIR` takes 21
//! T-states when it repeats and 16 when it stops. The two shapes always
//! agree on every access they both make, so [`InstructionTiming::accesses`]
//! lists the longest, and the k-th access a board sees falls at
//! `accesses[k].t` whichever way the instruction goes.
//! [`InstructionTiming::shapes`] gives each shape's length and how many of
//! the accesses it makes.
//!
//! **Not in the corpus**, and derived here by rule:
//!
//! * The 176 undefined ED opcodes, which execute as 8-T-state no-ops (Young
//!   3.4): the ED prefix and opcode fetches, at the offsets every ED
//!   instruction in the corpus shows.
//! * A DD or FD followed by DD, FD or ED: the first byte is a stray prefix,
//!   its own 4-T-state opcode fetch (Young 3.7), and the instruction after it
//!   is timed as usual, 4 T-states later.
//!
//! Interrupt acknowledge cycles (RESET, NMI, IM 0/1/2), and the 4-T-state
//! HALT idle cycle, are not instructions and are not covered.

use alloc::vec;
use alloc::vec::Vec;

use crate::access_timing::{Entry, TABLE};

/// The kind of a bus access: the reference's `"read"`, `"write"`, `"in"`,
/// `"out"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
    In,
    Out,
}

impl AccessKind {
    /// The reference's name for this kind.
    pub fn name(self) -> &'static str {
        match self {
            AccessKind::Read => "read",
            AccessKind::Write => "write",
            AccessKind::In => "in",
            AccessKind::Out => "out",
        }
    }
}

/// One bus access: its kind and offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessTiming {
    pub kind: AccessKind,
    pub t: u32,
}

/// Every access an instruction can make, and the ways it can execute.
///
/// `shapes` holds `(t_states, accesses_made)` for each way: the instruction
/// takes `t_states` and makes the first `accesses_made` of `accesses`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionTiming {
    pub accesses: Vec<AccessTiming>,
    pub shapes: Vec<(u32, usize)>,
}

fn find(key: &[u8]) -> Option<&'static Entry> {
    TABLE
        .binary_search_by(|(entry_key, _, _)| (*entry_key).cmp(key))
        .ok()
        .map(|index| &TABLE[index])
}

fn entry(key: &[u8]) -> InstructionTiming {
    let (_, accesses, shapes) =
        find(key).unwrap_or_else(|| panic!("no access timing for opcode bytes {key:02X?}"));
    InstructionTiming {
        accesses: accesses
            .iter()
            .map(|&(kind, t)| AccessTiming {
                kind,
                t: u32::from(t),
            })
            .collect(),
        shapes: shapes
            .iter()
            .map(|&(t_states, made)| (u32::from(t_states), usize::from(made)))
            .collect(),
    }
}

/// The fetches every ED instruction begins with; an undefined ED opcode is
/// only these.
fn ed_nop() -> InstructionTiming {
    InstructionTiming {
        accesses: vec![
            AccessTiming {
                kind: AccessKind::Read,
                t: 1,
            },
            AccessTiming {
                kind: AccessKind::Read,
                t: 5,
            },
        ],
        shapes: vec![(8, 2)],
    }
}

const STRAY_PREFIX: AccessTiming = AccessTiming {
    kind: AccessKind::Read,
    t: 1,
};

/// The access timing of the instruction whose bytes begin `data`.
///
/// `data` starts at the instruction's first byte, as a host would peek it at
/// PC before `step()`; it needs as many bytes as the instruction has opcode
/// and prefix bytes (four for `DD CB d op`, more for a run of prefixes), and
/// panics if it has fewer. Operands and displacements are not looked at.
pub fn instruction_timing(data: &[u8]) -> InstructionTiming {
    let first = data[0];
    if first == 0xCB || first == 0xED {
        let key = [first, data[1]];
        if first == 0xED && find(&key).is_none() {
            return ed_nop();
        }
        return entry(&key);
    }
    if first == 0xDD || first == 0xFD {
        let second = data[1];
        if matches!(second, 0xDD | 0xED | 0xFD) {
            let rest = instruction_timing(&data[1..]);
            let mut accesses = Vec::with_capacity(rest.accesses.len() + 1);
            accesses.push(STRAY_PREFIX);
            accesses.extend(rest.accesses.iter().map(|a| AccessTiming {
                kind: a.kind,
                t: a.t + 4,
            }));
            return InstructionTiming {
                accesses,
                shapes: rest
                    .shapes
                    .iter()
                    .map(|&(t_states, made)| (t_states + 4, made + 1))
                    .collect(),
            };
        }
        if second == 0xCB {
            return entry(&[first, 0xCB, data[3]]);
        }
        return entry(&[first, second]);
    }
    entry(&[first])
}

#[cfg(test)]
mod tests {
    //! The reference's fast tests (tests/test_access_timing.py), transcribed.

    use super::*;
    use AccessKind::{Read, Write};

    fn at(kind: AccessKind, t: u32) -> AccessTiming {
        AccessTiming { kind, t }
    }

    #[test]
    fn every_corpus_instruction_has_an_entry_and_every_entry_starts_with_its_fetch() {
        assert_eq!(TABLE.len(), 1604);
        assert!(TABLE.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (key, accesses, shapes) in TABLE {
            assert_eq!(accesses[0], (Read, 1), "{key:02X?}");
            assert!(accesses.windows(2).all(|p| p[0].1 <= p[1].1), "{key:02X?}");
            assert!(!shapes.is_empty(), "{key:02X?}");
            assert!(
                shapes
                    .iter()
                    .all(|&(_, made)| usize::from(made) <= accesses.len()),
                "{key:02X?}"
            );
        }
    }

    #[test]
    fn plain_cb_ed_and_index_instructions_are_found_by_their_opcode_bytes() {
        assert_eq!(
            instruction_timing(&[0x00]),
            InstructionTiming {
                accesses: vec![at(Read, 1)],
                shapes: vec![(4, 1)]
            }
        );
        assert_eq!(instruction_timing(&[0xCB, 0x06]).shapes, [(15, 4)]);
        assert_eq!(instruction_timing(&[0xED, 0xB0]).shapes, [(16, 4), (21, 4)]);
        assert_eq!(instruction_timing(&[0xDD, 0x7E, 0x05]).shapes, [(19, 4)]);
        // POP: the two stack reads, three T-states apart (issue #16's Mega Drive case).
        let pop = instruction_timing(&[0xE1]);
        assert_eq!(pop.accesses, [at(Read, 1), at(Read, 5), at(Read, 8)]);
    }

    #[test]
    fn index_cb_is_keyed_by_the_last_byte_not_the_displacement() {
        assert_eq!(
            instruction_timing(&[0xDD, 0xCB, 0x80, 0x06]),
            instruction_timing(&[0xDD, 0xCB, 0x00, 0x06])
        );
        assert_eq!(
            instruction_timing(&[0xFD, 0xCB, 0x05, 0x46]).shapes,
            [(20, 5)]
        );
    }

    #[test]
    fn conditional_instructions_list_the_accesses_of_their_longest_shape() {
        let call_nz = instruction_timing(&[0xC4, 0x00, 0x00]);
        let kinds: Vec<_> = call_nz.accesses.iter().map(|a| a.kind).collect();
        assert_eq!(kinds, [Read, Read, Read, Write, Write]);
        assert_eq!(call_nz.shapes, [(10, 3), (17, 5)]);
    }

    #[test]
    fn an_undefined_ed_opcode_is_its_two_fetches() {
        let timing = instruction_timing(&[0xED, 0x00]);
        assert_eq!(timing.accesses, [at(Read, 1), at(Read, 5)]);
        assert_eq!(timing.shapes, [(8, 2)]);
    }

    #[test]
    fn a_stray_prefix_is_one_more_fetch_and_shifts_the_rest_by_four() {
        let plain = instruction_timing(&[0xDD, 0x7E, 0x05]);
        let stray = instruction_timing(&[0xFD, 0xDD, 0x7E, 0x05]);
        assert_eq!(stray.accesses[0], at(Read, 1));
        let shifted: Vec<_> = plain.accesses.iter().map(|a| at(a.kind, a.t + 4)).collect();
        assert_eq!(stray.accesses[1..], shifted[..]);
        assert_eq!(stray.shapes, [(23, 5)]);
        let before_ed = instruction_timing(&[0xDD, 0xED, 0xB0]);
        assert_eq!(before_ed.shapes, [(20, 5), (25, 5)]);
    }
}
