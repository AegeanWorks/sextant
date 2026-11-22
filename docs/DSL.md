# Sextant model and trace languages

Both languages are line oriented, UTF-8, and use `#` to start a comment. There is no
escaping rule and no implicit continuation: one line is one statement. That makes a
machine reviewable in a diff and its parser easy to audit.

## Machine file (`*.sex`)

### Declarations

| Line | Meaning |
|---|---|
| `machine <name>` | Exactly one machine name. Required. |
| `initial <state>` | Exactly one initial state. Required. Must be declared. |
| `state <name>` | Declare a state. Duplicates are rejected. |
| `terminal <state>` | Declare an intentional end state; it is exempt from the dead-end finding and must be declared as a state. |
| `event <name>` | Declare an event. Duplicates are rejected. |
| `invariant <name>: <condition>` | A condition that must hold initially and after every step. |

### Transitions

```
transition <from> <event> [when <condition>] [do <effects>] -> <to>
```

- `when` and `do` are optional and may appear in either order (the parser reads tokens
  up to the `->` marker).
- The arrow `->` and the destination must be the last two tokens.
- A transition with no guard is always enabled.
- Two transitions with the same `(from, event, guard)` are a structural finding. At run
  time, two simultaneously holding guards are an ambiguity error: Sextant never resolves
  nondeterminism by declaration order.

### Conditions

```
<operand> <operator> <operand>
```

Exactly three whitespace-separated tokens.

- `<operator>` is one of `<`, `<=`, `>`, `>=`, `==`, `!=`.
- `<operand>` is an integer literal (optional leading `-`) or a variable name.
- An unbound variable reads as `0`, which keeps evaluation total for every environment.

Examples: `tries <= 2`, `retries != limit`, `tokens > -1`.

### Effects

Effects follow `do` and are separated by commas:

| Form | Meaning |
|---|---|
| `x = 5` | Set `x` to `5`. |
| `x += 2` | Add `2` to `x`. |
| `x -= 1` | Subtract `1` from `x`. |

Effects are applied in written order to a copy of the environment. Arithmetic is
saturating, so a long replay cannot wrap an integer and silently change meaning.

### Full example

```
# retry with bounded attempts
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

## Event script (`*.events`)

One declared event name per line; blank lines and comments are ignored. A line with
anything other than a single token is a parse error.

```
START
TICK
TICK
OK
```

## Trace (`*.golden`)

Versioned output of `replay`; the header makes incompatible future formats obvious.

```
# sextant-trace v1
machine: retry
initial: {}
step 001: START | idle {} -> waiting {}
step 002: TICK | waiting {} -> waiting {tries=1}
step 003: TICK | waiting {tries=1} -> waiting {tries=2}
step 004: OK | waiting {tries=2} -> done {tries=2}
final: done {tries=2}
```

Environments are rendered as `{name=value,name=value}` with names in sorted order, so
two runs on different machines produce identical bytes. `golden` compares line by line
and reports the first divergence.

## Behavioural properties

Properties are checked by `explore`, not by `check` (which is structural only):

```
property reaches_done: reaches done
property avoids_waiting: avoids waiting
property emits_ok: emits OK
```

- `reaches <state>` must be visited by at least one reachable configuration.
- `avoids <state>` must not be visited by any reachable configuration.
- `emits <event>` must appear on at least one enabled transition during exploration.

A property is a finding of type `property`, distinct from an `invariant` violation.
Both are reported with the path that led to them where one exists.

## Equivalence reports

`equiv` enumerates every event sequence over the union of both alphabets up to `--depth`
(default 4) and compares observable outcomes:

```
# sextant-equivalence v1
left: retry
right: retry_renamed
depth: 4
sequences: 341
differences: 0
```

An outcome is `ok:steps=N:env={...}` for a completed replay or a failure category
(`err:stuck`, `err:ambiguous`, `err:invariant:<name>`). State labels are intentionally
not compared, so renaming states is accepted. A sequence cap marks the result partial.

## Fuzz reports

`fuzz` replays generated scripts for seeds `0..N` with `M` events each:

```
# sextant-fuzz v1
machine: retry
seeds: 64
events-per-seed: 12
failures: 3
  - seed=7 reason=invariant:tries_bound events=START,TICK,...
```

Failures are listed in seed order and capped at 20 displayed cases while the total count
remains exact. With `--invariant NAME`, only violations of that invariant count; without
it, `stuck` and `ambiguous` replays count too. A reported seed is reproducible and can be
turned into a short reproducer by writing its events to a script and running `minimize`.

## Shadowed guards

`check` reports `ShadowedTransition` when two guarded transitions on the same `(state,
event)` have single-variable literal-bound intervals where the earlier guard contains the
later one — for example `when tries <= 5` declared before `when tries <= 2`. The later
transition can never be the only enabled one. The analysis is deliberately conservative:
`!=`, comparisons between two variables, and mixed expressions are not modelled and
never produce a finding.

## Trace recovery and slicing

`untrace` reads a versioned trace (`# sextant-trace v1`) and writes the event script that
produced it, one event per line. It validates the header and each step's shape, reporting
the offending line on malformed input.

`slice <machine> <events>` replays the script and emits a canonical machine containing
only the transitions that were exercised, plus the states and events they mention.
Properties whose symbols disappear are dropped and listed in a `# sextant-slice v1`
summary on stderr. The result replays the same script with the same steps and environment.

A slice declares its final state `terminal`, so it verifies cleanly and can be replayed
through every CLI command. It is still a projection for debugging a specific path: other
transitions of the original model are intentionally absent.

## Canonical form and diff

`fmt` parses a machine and re-renders it in a fixed order: machine, initial, sorted
states, sorted events, transitions sorted by semantic text, sorted invariants and sorted
properties. Comments are intentionally dropped. `fmt --check` exits 1 when the file is not
already canonical, which makes it usable as a style gate; `fmt --out F` writes the
canonical text instead of stdout.

`diff` compares two machines by the canonical text of each element, so reordering
declarations or changing whitespace produces zero changes. Its versioned report
(`# sextant-diff v1`) lists added and removed states, events, transitions, invariants and
properties, plus a changed initial state. `--json` emits the same information with stable
keys.

## Self-check

`doctor [--json]` runs eleven pure checks against a model embedded in the binary: parse,
structure, event parse, replay determinism, golden rendering, coverage, exploration,
seeded generation, the minimizer table, JSON serialization and self-equivalence. It reads
no files, so a broken fixture in the working directory cannot affect the verdict.

## JSON output

`check`, `explore` and `fuzz` accept `--json`. Each prints one compact object with fixed
key order and a trailing newline; human status goes to stderr, so stdout can be piped:

```
sextant check retry.sex --json
{"machine":"retry","ok":true,"counts":{"states":3,...},"findings":[]}
```

| Command | Stable top-level keys |
|---|---|
| `check` | `machine`, `ok`, `counts`, `findings` |
| `explore` | `ok`, `configurations`, `transitions`, `rules_checked`, `truncated`, `violations` |
| `fuzz` | `machine`, `ok`, `seeds`, `events_per_seed`, `failures`, `cases` |

A finding object has `rule` and `message`; a violation object has `type`, `rule`, `state`,
`env` and `path`; a fuzz case has `seed`, `reason` and `events`. These keys are a public
contract. The serializer is hand-written with `std` only, escapes control characters and
quotes, and never reorders keys.

## Error semantics

- `check` is structural: it reports undeclared states/events, identical guards,
  unreachable states (ignoring guards, a sound over-approximation) and states with no
  outgoing transitions.
- `replay` is behavioural: a missing enabled transition is `stuck`; an invariant that
  stops holding is a violation; two holding guards are `ambiguous`.
- `explore` is bounded: it deduplicates `(state, env)` configurations, so cycles
  terminate, and a cap (`200_000` configurations) marks results as partial rather than
  running without limit.
