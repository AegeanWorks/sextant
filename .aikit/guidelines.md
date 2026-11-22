# Project Guidelines

Rules for this project only, written by people or AI agents. They add to AIKit's principles and
rank below them: a guideline never overrides a principle (PRINCIPLE-000 R6, PRINCIPLE-001 R4).
Add one with `python aikit.py guideline <name> "<rule>" --source <user|ADR-NNN|person>`, or by
hand as one line below in the form `- **G-NNN** (source): rule`; IDs are never reused.

Persisted state carries a format version. A loader refuses a version it does not support,
without modifying or quarantining that file; older versions migrate when a migration exists.

## Rules
- **G-001** (user): Functional Core, Imperative Shell, I/O at the Edges and Explicit State Machines are hard architectural constraints, not style: src/core is pure and contains no filesystem, stdout, clock, process or network access; only src/shell performs I/O at process edges; both the user-modelled finite state machine and the CLI session lifecycle are explicit enum state machines with enumerated transitions.
