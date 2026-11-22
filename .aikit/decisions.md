# Decisions

Why this project is built the way it is, oldest first. One entry per decision that changes a
contract, a dependency, a data format or the architecture; IDs are consecutive and never reused.

<!-- Entry format:
### ADR-NNN: <decision in one line>
- **Date**: YYYY-MM-DD
- **Status**: Accepted
- **Context**: <what forced the decision>
- **Decision**: <what was decided>
- **Consequences**: <what follows, including what it costs>
-->

### ADR-001: ADR-001: Sextant scope and functional-core architecture
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: The AegeanWorks portfolio (all 68 repositories, Projects, Packages, Stars and the empty Future ideas list were reviewed) has regex search, compilers, a 3D engine, LLM inference and many Python CLIs, but no tool for verifying or replaying stateful behaviour. The user requires a unique programmer-facing tool in Rust only, built under Functional Core / Imperative Shell, I/O at the Edges, Explicit State Machines.
- **Decision**: Build Sextant in Rust (std only, no crates) as a finite-state-machine verifier, event-script replayer, bounded explorer and golden-trace regression CLI. src/core holds pure data and total functions; src/shell alone performs file/stdout/exit I/O; the machine semantics and the SessionState lifecycle are explicit enums with transition tables. The Python starter compat scripts are removed in favour of Rust equivalents.
- **Consequences**: Architecture is auditable by tests that assert core has no I/O imports and that illegal session transitions are rejected. Traces are deterministic byte-for-byte so golden files are stable across machines. No third-party dependency surface; Rust 1.100+ is the only build prerequisite.

### ADR-002: ADR-002: Sextant v0.2 observability and reduction commands
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: v0.1 proves and replays a machine, but a programmer still has three unanswered questions: which transitions did my event script actually exercise, what is the smallest input that still breaks the model, and what does the machine look like? These are pure functions of (machine, events) and fit the existing functional-core boundary.
- **Decision**: Add three commands implemented as pure core functions: coverage (transition/state/event coverage from a trace), minimize (deterministic 1-minimal reduction of a failing script, driven by an explicit MinimizerState machine), and graph (DOT/Mermaid rendering). Keep all I/O in shell::io and reuse the existing CLI session lifecycle.
- **Consequences**: Three new core modules and one test module; README tree must list them before the gate runs; the architecture test's core-file count increases. No new dependencies. Deterministic output keeps golden-style comparison possible.

### ADR-003: ADR-003: Behavioural properties and bounded equivalence
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Invariants express conditions on the environment but cannot say that a state is reachable or that an event is ever emitted; and a refactor that renames states had no mechanical way to prove behaviour unchanged. Both questions are properties of the explored configuration space and can stay pure.
- **Decision**: Add property declarations (reaches, avoids, emits) checked during explore, and a bounded equiv command that compares observable outcomes (accepted step count, final environment, failure category) across every event sequence up to a depth, deliberately ignoring state labels so renames are accepted.
- **Consequences**: Machine gains a properties vector; Violation gains a rule type; explore counts invariants plus properties; a new equivalence core module and test module are added. Equivalence is bounded and reports truncation rather than claiming completeness.

### ADR-004: ADR-004: Seed fuzzing and guard-shadow detection
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Bounded exploration enumerates structured sequences, but a programmer also wants random-looking reproducible scripts that mimic real usage, and check currently misses the common mistake of a broad transition declared before a narrower one on the same event.
- **Decision**: Add a pure uzz core that replays generated scripts over a seed range and reports deterministic failures, and extend erify with a shadowed-transition finding based on interval dominance for guarded transitions on the same (state, event).
- **Consequences**: A new core module and test module; one new Finding variant; a new CLI command with versioned output. Dominance is deliberately limited to one-variable comparisons with literal bounds, so it is conservative and never claims shadowing for non-interval guards.

### ADR-005: ADR-005: Deterministic JSON output for CI
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Sextant's text reports are good for humans but awkward in CI pipelines, which need a stable machine-readable shape. Adding a JSON dependency is unnecessary and would break the zero-dependency contract.
- **Decision**: Add a minimal pure JSON value type and serializer in src/core/json.rs, and a --json switch on check, explore and fuzz. Object key order is fixed by construction, so output is byte-stable. Text output remains the default.
- **Consequences**: Three CLI variants gain a boolean field; a new core module and test module are added; no dependencies. JSON keys are part of the public contract and are documented in DSL.md.

### ADR-006: ADR-006: Built-in doctor self-check
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: A user landing on a new machine needs one command that proves the binary, the parsers, replay determinism and the auxiliary engines actually work before trusting a golden file.
- **Decision**: Add a pure doctor core that runs a fixed embedded model through parse, verify, replay, golden rendering, exploration, seeded generation, minimization state transitions and JSON serialization, returning a list of named results. The shell renders text or JSON and maps any failure to exit 1.
- **Consequences**: A new core module and CLI command; no external files are read by the check itself, so it cannot be fooled by a broken fixture. All checks are pure and deterministic.

### ADR-007: ADR-007: Canonical formatter and structural machine diff
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Models are reviewed in diffs, but hand-written declaration order and spacing make semantic changes hard to see, and there is no way to list what changed between two machine files.
- **Decision**: Add a pure canonical renderer used by mt (with --check for CI) and a pure structural diff of states, events, transitions, invariants, properties and the initial state. Transition identity is its rendered semantic form, so whitespace and declaration order do not create false differences.
- **Consequences**: Two new core modules and CLI commands; no dependencies. Canonical output is idempotent by construction. Diff reports are versioned and can be rendered as JSON.

### ADR-008: ADR-008: Trace recovery and machine slicing
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: A golden trace records what happened, but turning it back into an event script or a reduced model still requires manual transcription. Both are pure transformations of data Sextant already produces.
- **Decision**: Add untrace to recover the event script from a versioned trace, and slice to project a machine down to the transitions actually exercised by a script, dropping unused states/events and properties whose symbols disappear. Both render through existing pure functions.
- **Consequences**: Two new core modules and CLI commands; the sliced machine is canonical and remains replayable with the same script. Dropped properties are reported explicitly so a user never assumes a removed rule was preserved.

### ADR-009: ADR-009: Explicit terminal states
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: Dead-end detection is useful but v0.8 slices always end in a state, which check wrongly treats as a modeling mistake and which makes replay refuse a valid slice.
- **Decision**: Add a terminal state declaration; verify skips the dead-end finding only for declared terminals, slice marks the script final state as terminal, and canonical rendering plus structural diff include terminals.
- **Consequences**: Modelers state intent explicitly while dead-end detection keeps its power for undeclared sinks, and sliced machines replay through the CLI without a bypass flag.

### ADR-010: ADR-010: Guard diagnostics and reachable-state map
- **Date**: 2026-11-22
- **Status**: Accepted
- **Context**: When an event is stuck, replay only says that no transition fired; a modeler needs to see which guards failed and with what values. Exploration also needs a human-readable map of reachable configurations.
- **Decision**: Add explain, which replays a prefix script and evaluates every candidate guard for the next event with actual environment values, and reachable, which lists deduplicated reachable configurations with shortest paths. Both are pure functions using the existing transition selector.
- **Consequences**: Two new core modules and CLI commands; no dependencies. Outputs are versioned and deterministic, and explain never mutates the model or the environment.
