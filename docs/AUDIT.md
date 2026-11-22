# Architecture audit — Sextant v0.1

This document maps each architectural directive to the concrete code that implements it
and the test that proves it. It is deliberately evidence-first: a claim without a
command or a file is not a claim.

## Directive 1 — Functional Core, Imperative Shell

**Claim.** All domain decisions are pure functions; command sequencing lives in the shell.

**Evidence.**

- `src/core/` has five modules plus `mod.rs`: `model.rs`, `parse.rs`, `engine.rs`,
  `format.rs`, `error.rs`. None imports an I/O, clock, process or environment API.
- `src/core/engine.rs::choose_transition` is the single step function. `replay` and
  `explore` both call it; there is no duplicate step logic.
- `src/shell/run.rs` contains the only command orchestration (`run_check`, `run_replay`,
  `run_golden`, `run_explore`, `run_generate`, `run_demo`).
- Test: `tests/architecture.rs::functional_core_has_no_io_or_ambient_state` scans all six
  core source files for forbidden tokens and fails on any hit.

**Command output.**

```
running 3 tests
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Directive 2 — Push I/O to the Edges

**Claim.** Files, stdout and stderr are touched only at the process boundary.

**Evidence.**

- `src/shell/io.rs` is the only file importing `std::fs` or calling `write_all`; it also
  owns `read_text`, `write_text`, `write_stdout`, `out` and `err`.
- `src/main.rs` reads `std::env::args` once and maps the result to an exit code.
- `replay`/`generate` use `finish_quiet` so data and status travel on different streams.
- Test: `tests/architecture.rs::io_is_confined_to_the_edge_module`.
- Test: `tests/cli.rs::generated_scripts_are_reproducible_from_their_seed` asserts a
  `--count 12` generation produces exactly 12 stdout lines.

## Directive 3 — Model Stateful Behavior with Explicit State Machines

**Claim.** Two explicit state machines, both enumerated and transition-driven.

**Evidence.**

- Domain FSM: `Machine`, `Transition`, `Condition`, `Effect` in `src/core/model.rs`;
  transition selection in `src/core/engine.rs`. Guards are evaluated, ambiguity is
  rejected, and effects are applied to a cloned environment.
- CLI FSM: `SessionState` (7 states), `SessionEvent` (7 events), `TRANSITIONS` (14 edges)
  in `src/shell/session.rs`. `transition()` is a pure lookup with a closed result.
- Tests: `cli_session_is_an_explicit_closed_state_machine` checks every pair, and
  `tests/core.rs` checks guarded selection, stuck states, ambiguity, invariants and
  exploration.

## Full regression evidence

Command run from `projects/sextant`:

```
$ cargo test
running 3 tests   (architecture)  -> ok. 3 passed; 0 failed
running 23 tests  (cli)           -> ok. 23 passed; 0 failed
running 12 tests  (core)          -> ok. 12 passed; 0 failed
running 7 tests   (features)      -> ok. 7 passed; 0 failed
running 5 tests   (behavior)      -> ok. 5 passed; 0 failed
running 5 tests   (fuzz)          -> ok. 5 passed; 0 failed
running 4 tests   (json)          -> ok. 4 passed; 0 failed
running 2 tests   (doctor)        -> ok. 2 passed; 0 failed
running 4 tests   (format_diff)   -> ok. 4 passed; 0 failed
running 4 tests   (slice)         -> ok. 4 passed; 0 failed
total: 69 passed, 0 failed, EXIT=0
```

Sample behavioural checks:

```
$ cargo run --quiet -- check examples/retry.sex
ok: 'retry' is well-formed (3 states, 4 events, 4 transitions, 1 invariants)

$ cargo run --quiet -- golden examples/retry.sex examples/retry.events examples/retry.golden
ok: golden trace matches

$ cargo run --quiet -- coverage examples/retry.sex examples/partial.events
transitions: 2/4 (50%)
  - [3] waiting --OK--> done
  - [4] done --RESET--> idle

$ cargo run --quiet -- minimize examples/strict.sex examples/failing.events --invariant tries_bound
minimal: 3 event(s)
state: Minimal

$ cargo run --quiet -- equiv examples/retry.sex examples/retry_renamed.sex
differences: 0
```

## Deviations handled during the build

1. The AIKit `new` scaffold creates Python ADR-315 compatibility files. They were removed;
   the deterministic scripted drive is now `sextant demo --scripted` in Rust.
2. The host has no MSVC `link.exe`. `.cargo/config.toml` selects the GNU target, matching
   the existing `projects/keystone` precedent, so plain `cargo test` works.
3. An early `replay`/`generate` status line polluted machine-readable stdout; it was moved
   to stderr through `finish_quiet` and a regression test was added.

All three are documented in `reports/AIKIT-GAPS-BUGS-FIXES-2026-11-22.md` at the AIKit
workspace root.
