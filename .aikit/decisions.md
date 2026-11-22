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
