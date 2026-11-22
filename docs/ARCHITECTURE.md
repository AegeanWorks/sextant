# Sextant architecture

## The three constraints

The user's Architectural Engineering Directive is recorded as project guideline G-001 and
treated as a constraint on every source file, not as a style preference.

### 1. Functional Core, Imperative Shell

`src/core/` contains only data and total functions:

- `model.rs` — `State`, `Event`, `Condition`, `Effect`, `Transition`, `Machine`, `Env`,
  `Trace`. All types are `Clone + Debug + PartialEq + Eq`; keys use `BTreeMap`/`BTreeSet`
  for deterministic ordering.
- `parse.rs` — `parse_machine` and `parse_events` turn text into data or a structured
  `Error` carrying a line number.
- `engine.rs` — `choose_transition`, `replay`, `verify`, `explore`, `generate_events`,
  `Rng`. No function reads a file, a clock, an environment variable or ambient state.
- `format.rs` — `render_trace`, `render_findings`, `diff_traces` and condition/effect renderers. Pure string transforms.
- `coverage.rs` — derives state/event/transition coverage from a trace and renders a versioned report.
- `minimize.rs` — deterministic 1-minimal reduction of a failing script, driven by its own `MinimizerState` transition table.
- `graph.rs` — DOT and Mermaid export of the machine.
- `error.rs` — `Error` and `Finding` with pure message formatting.

`src/shell/` is the imperative shell: `cli.rs` parses arguments, `run.rs` sequences
operations and drives the session state machine, `io.rs` performs the actual I/O, and
`main.rs` maps results to process exit codes.

### 2. I/O at the Edges

The boundary is explicit and tiny:

- `src/shell/io.rs` is the only module that imports `std::fs` or calls `write_all`.
- `src/main.rs` reads `std::env::args` once at the process edge.
- Data-producing commands write only data to stdout; status goes to stderr
  (`finish_quiet`), so `sextant generate ... > events.sex` produces exactly the script.
- `write_text` flushes and fsyncs generated files before returning.

`tests/architecture.rs::io_is_confined_to_the_edge_module` enforces this by scanning
source, and `functional_core_has_no_io_or_ambient_state` rejects `std::fs`, `std::io`,
`std::env`, `std::process`, `std::time`, `std::net`, `println!`, `eprintln!` and
`Command::new` anywhere under `src/core/`.

### 3. Explicit State Machines

There are three, all real and all inspectable:

**User model.** A `Machine` is states + events + transitions + invariants. The single
transition selector `choose_transition(machine, state, env, event)` filters by state and
event, evaluates guards, rejects simultaneous guards as ambiguous, applies effects to a
clone of the environment, and returns the chosen edge. Replay and exploration call this
one function; there is no second, divergent notion of a step. Guards use a total
environment (unbound variable = 0), and effects use saturating arithmetic.

**CLI lifecycle.** `SessionState` is `Idle → Loaded → Verified → Replayed → Compared →
Complete`, with `Failed` and `Reset`. The complete transition table is a `const`
slice of 14 edges in `src/shell/session.rs`. Every command drives it through one helper;
illegal transitions are errors, not no-ops. The test enumerates all 7×7 state/event pairs
and asserts the transition function agrees with the table exactly, and that no pair has
two destinations.

**Minimizer lifecycle.** `MinimizerState` is `Trail → Shrinking → Minimal`, with events
`FailureConfirmed`, `CandidateFails`, `CandidatePasses` and `NoCandidatesRemaining`. The
closed table has four edges in `src/core/minimize.rs`; the reduction loop drives it and
the test enumerates all 3×4 pairs. This is the third explicit state machine.

## Why this shape

- **Determinism is a contract.** Golden files are only useful if a trace is identical on
  every run and machine. Sorted maps, ordered transitions, no clock and a pure PRNG make
  that true by construction.
- **Verification is separable.** Static structure (`verify`), bounded dynamics
  (`explore`) and a concrete run (`replay`) are three different questions. Keeping them
  as separate pure functions lets the shell choose which one a command needs.
- **The edge can stay dumb.** Because core results are data, the shell only reads text,
  calls a pure function and writes text. There is no business logic in `io.rs`.

## Boundaries and limitations (v0.2)

- The environment holds integers only; no strings, lists or floats.
- `verify` ignores guards for reachability. This over-approximates reachability, so an
  "unreachable" finding is sound, but a state reported reachable may still be infeasible
  under its guards.
- `explore` bound is 200,000 configurations; hitting it sets `truncated` rather than
  claiming completeness.
- Effects use saturating arithmetic to avoid silent wraparound; this is a deliberate
  semantic choice, not an error condition.
- `minimize` guarantees 1-minimality (no single event can be removed), not global minimum
  length; a different deletion order could produce a different but equally 1-minimal script.
  The order is fixed, so results are reproducible.
- `coverage` counts each transition once even when exercised repeatedly; it answers "was
  this edge reached?", not "how often?".
- No third-party crates. The only build prerequisite is a Rust toolchain with a working
  linker. On this Windows host the project follows `projects/keystone/.cargo/config.toml`
  and selects `x86_64-pc-windows-gnu`.
