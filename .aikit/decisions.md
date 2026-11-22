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
