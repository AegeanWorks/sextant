//! Sextant: deterministic finite-state-machine verification, replay and
//! golden-trace regression.
//!
//! Architecture (hard constraints, see G-001):
//! - `core` is a pure functional core with no I/O;
//! - `shell` is the imperative shell and owns the CLI session state machine;
//! - all I/O is confined to `shell::io` and the `main` process edge.

pub mod core;
pub mod shell;
