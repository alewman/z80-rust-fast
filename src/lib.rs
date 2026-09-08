//! A fast Rust Z80 instruction core, forked from `z80-rust` and kept
//! provably equivalent to `z80-python` at every processor boundary.
//!
//! The crate name stays `z80_rust` so a host written for z80-rust compiles
//! unchanged. The instruction core (`flags`, `state`, `core`, `alu`,
//! `blocks`, `control`, `dispatch`, `index`, `index_dispatch`, `io`, `loads`,
//! `rotate`, `cpu`) needs nothing from `std`; `trace` and `conformance`
//! implement the trace schema and the conformance kit (`docs/trace-schema.md`
//! and `docs/conformance.md` in z80-python) and need the default `std`
//! feature.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod alu;
pub mod blocks;
#[cfg(feature = "std")]
pub mod conformance;
pub mod control;
pub mod core;
pub mod cpu;
pub mod dispatch;
pub mod flags;
pub mod index;
pub mod index_dispatch;
pub mod io;
pub mod loads;
pub mod rotate;
pub mod state;
#[cfg(feature = "std")]
pub mod trace;

pub use crate::core::{Bus, Fault, Z80};
pub use crate::flags::Flags;
pub use crate::state::CpuState;
