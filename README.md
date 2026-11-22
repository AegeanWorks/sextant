# Sextant

[![Built with AIKit](https://img.shields.io/badge/built%20with-AIKit-6f42c1)](https://github.com/AegeanWorks/AIKit)

Sextant is a deterministic, zero-dependency Rust CLI that verifies finite state machines, replays event scripts into auditable traces, and guards stateful behavior with golden-trace regression.

Event-driven code — protocol handlers, retry loops, UI flows, embedded controllers — is easy to reason about one transition at a time and hard to regression-test as a whole. Sextant gives that behavior a small textual model, executes it deterministically, and turns the result into a diffable trace. No network, no runtime, no telemetry; the model and its traces are plain text you can review in a diff.

## ⚡ 30-Second Cold-Start Quick Reference

- **Runtime Prerequisites**: Rust 1.70+ with Cargo. On Windows the MSVC linker is not
  required: the repository selects the GNU target in `.cargo/config.toml` and uses an
  available mingw-w64 `gcc`. No third-party crates.
- **Canonical Regression Shield Gate**:
  ```bash
  cargo test
  ```
- **Try it** (from this folder):
  ```bash
  cargo run --quiet -- check examples/retry.sex
  cargo run --quiet -- replay examples/retry.sex examples/retry.events
  cargo run --quiet -- golden examples/retry.sex examples/retry.events examples/retry.golden
  cargo run --quiet -- explore examples/retry.sex --depth 6
  cargo run --quiet -- coverage examples/retry.sex examples/partial.events
  cargo run --quiet -- minimize examples/strict.sex examples/failing.events --invariant tries_bound
  cargo run --quiet -- graph examples/retry.sex --format mermaid
  cargo run --quiet -- equiv examples/retry.sex examples/retry_renamed.sex
  cargo run --quiet -- fuzz examples/strict.sex --seeds 64 --invariant tries_bound
  ```

### 🛠️ AIKit Commands (run from the AIKit folder)

| Role | Command |
|---|---|
| Status, task and gates | `python aikit.py status sextant` |
| Phase 4: every gate | `python aikit.py check sextant` |
| Repair memory and map, then check | `python aikit.py fix sextant` |
| Resume an interrupted task | `python aikit.py resume sextant` |

## What it does

| Command | Purpose |
|---|---|
| `check <machine.sex>` | Validate structure: undeclared states/events, duplicate guarded transitions, unreachable and dead-end states. |
| `replay <machine.sex> <events.sex>` | Produce a deterministic, human-readable trace of every transition and environment change. |
| `golden <machine.sex> <events.sex> <golden.txt> [--update]` | Guard stateful behavior against a committed trace; report the first differing line. |
| `explore <machine.sex> [--depth N]` | Enumerate reachable `(state, env)` configurations up to a depth and check invariants with a bounded cap. |
| `coverage <machine.sex> <events.sex>` | Report which states, events and transitions a script actually exercised, and name the missed edges. |
| `minimize <machine.sex> <events.sex> [--invariant NAME]` | Reduce a failing script to a 1-minimal reproducer; with `--invariant`, only that violation counts as failure. |
| `graph <machine.sex> [--format dot\|mermaid]` | Export the machine as a reviewable DOT or Mermaid state diagram. |
| `equiv <left.sex> <right.sex> [--depth N]` | Compare observable behaviour up to a depth; state renames do not count as differences. |
| `fuzz <machine.sex> [--seeds N] [--count M] [--invariant NAME]` | Replay generated scripts over a seed range; report reproducible failures that `minimize` can shrink. |
| `generate <machine.sex> --seed N [--count M]` | Emit a reproducible event script from a pure seeded PRNG (no clock, no `rand` crate). |
| `demo --scripted [--seed N]` | Deterministic scripted drive (`SEED:` / `WORLD-DELTA:` / `OK:`), the Rust replacement for the AIKit Python starter. |
| `inventory` | Machine-readable `kind|name|locator` inventory consumed by AIKit's compat gate. |

Exit codes: `0` success, `1` verification or behaviour failure, `2` usage error.
`replay` and `generate` write only machine-readable data to stdout; status goes to stderr
so the output can be piped directly into a file. `check`, `explore` and `fuzz` accept
`--json` for a single stable JSON object on stdout (keys in fixed order, no dependencies).

## The model language

A machine is a line-oriented text file. `#` starts a comment.

```text
machine retry
initial idle

state idle
state waiting
state done

event START
event TICK
event OK
event RESET

transition idle START -> waiting
transition waiting TICK when tries <= 2 do tries += 1 -> waiting
transition waiting OK -> done
transition done RESET do tries = 0 -> idle

invariant tries_bound: tries <= 3
```

- `when <left> <op> <right>` compares an integer literal or a variable; an unbound
  variable reads as zero. Operators: `< <= > >= == !=`.
- `do <name> = N`, `<name> += N`, `<name> -= N` mutate the integer environment.
  Effects are comma separated and applied in order.
- `invariant <name>: <condition>` must hold initially and after every step.
- Two transitions for the same state and event are allowed only while at most one
  guard holds; otherwise replay reports ambiguity instead of choosing arbitrarily.
- An event script has one event name per line. See [docs/DSL.md](docs/DSL.md).

A replayed trace is stable byte-for-byte:

```text
# sextant-trace v1
machine: retry
initial: {}
step 001: START | idle {} -> waiting {}
step 002: TICK | waiting {} -> waiting {tries=1}
step 003: TICK | waiting {tries=1} -> waiting {tries=2}
step 004: OK | waiting {tries=2} -> done {tries=2}
final: done {tries=2}
```

## Architecture

Three constraints are treated as hard rules, recorded as guideline G-001 and checked by
`tests/architecture.rs` on every `cargo test`:

1. **Functional Core, Imperative Shell.** `src/core/` is pure data and total functions.
   `src/shell/` orchestrates commands, parses arguments and maps outcomes to exit codes.
2. **I/O at the Edges.** Only `src/shell/io.rs` touches files, stdout and stderr; `main`
   reads process arguments at the boundary. No core module imports `std::fs`, `std::io`,
   `std::env`, a clock or a process API.
3. **Explicit State Machines.** Both levels are enums with transition functions: the
   user's FSM (`Machine` / `Transition` / `Condition` / `Effect`, with a single pure
   transition selector) and the CLI session lifecycle (`SessionState` with a closed const
   transition table of 14 edges).

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design and
[docs/AUDIT.md](docs/AUDIT.md) for the principle-by-principle audit with test evidence.

## 📦 What's Inside

```text
sextant/
├── .cargo/                   <- Cargo build configuration
│   └── config.toml           <- GNU target on this Windows host (MSVC linker absent)
├── .gitattributes            <- LF line endings on every platform
├── .gitignore                <- transient AIKit/Rust state excluded
├── Cargo.toml                <- zero dependencies
├── Cargo.lock                <- pinned dependency graph (empty)
├── LICENSE                   <- Aegean License 1.1 (AL-1.1): free personal use
├── COMMERCIAL.md             <- Commercial licensing terms
├── LICENSES/                 <- REUSE license texts
├── REUSE.toml                <- Machine-readable SPDX metadata
├── README.md
├── src/
│   ├── main.rs               <- process edge: args in, exit code out
│   ├── lib.rs
│   ├── core/                 <- pure functional core (no I/O)
│   │   ├── mod.rs            <- module exports
│   │   ├── model.rs          <- State, Event, Condition, Effect, Machine, Trace
│   │   ├── parse.rs          <- pure machine and event-script parsers
│   │   ├── engine.rs         <- transition, replay, verify, explore, seeded PRNG
│   │   ├── format.rs         <- deterministic trace rendering and diffing
│   │   ├── coverage.rs       <- state/event/transition coverage from a trace
│   │   ├── minimize.rs       <- 1-minimal reproducer with an explicit state machine
│   │   ├── graph.rs          <- DOT and Mermaid rendering
│   │   ├── equivalence.rs    <- bounded, state-label-independent comparison
│   │   ├── fuzz.rs           <- reproducible seed-range failure search
│   │   ├── json.rs           <- std-only deterministic JSON value and serializer
│   │   └── error.rs          <- pure error/finding types
│   └── shell/                <- imperative shell
│       ├── mod.rs            <- module exports
│       ├── cli.rs            <- argument parsing and usage text
│       ├── session.rs        <- explicit CLI SessionState transition table
│       ├── io.rs             <- the only file/stdout/stderr boundary
│       └── run.rs            <- command orchestration
├── tests/
│   ├── core.rs               <- 11 pure-core tests
│   ├── cli.rs                <- 18 end-to-end process tests
│   ├── features.rs           <- 7 coverage/minimize/graph tests
│   ├── behavior.rs           <- 5 property/equivalence tests
│   ├── fuzz.rs               <- 5 fuzz/shadowing tests
│   ├── json.rs               <- 4 JSON serializer tests
│   └── architecture.rs       <- 3 architectural audit tests
├── examples/
│   ├── retry.sex             <- example machine
│   ├── retry.events          <- example event script
│   ├── retry.golden          <- committed golden trace
│   ├── partial.events        <- coverage example (2 of 4 transitions)
│   ├── strict.sex            <- minimized failing invariant example
│   ├── failing.events        <- failing script for the strict example
│   ├── properties.sex        <- reaches/avoids/emits example
│   └── retry_renamed.sex     <- renamed-state twin for `equiv`
├── docs/
│   ├── DSL.md                <- grammar and semantics
│   ├── ARCHITECTURE.md       <- design and constraint rationale
│   ├── AUDIT.md              <- directive-to-evidence audit
│   └── manual.html           <- standalone static reference (no external assets)
└── .aikit/                   <- this project's memory: guidelines, ADR, ledger
```

## 📜 License

Free for personal use under the Aegean License 1.1 (AL-1.1); see [LICENSE](LICENSE). Any commercial use needs the Aegean Commercial License 1.0 (ACL-1.0); see [COMMERCIAL.md](COMMERCIAL.md).
