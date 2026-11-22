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

## Error semantics

- `check` is structural: it reports undeclared states/events, identical guards,
  unreachable states (ignoring guards, a sound over-approximation) and states with no
  outgoing transitions.
- `replay` is behavioural: a missing enabled transition is `stuck`; an invariant that
  stops holding is a violation; two holding guards are `ambiguous`.
- `explore` is bounded: it deduplicates `(state, env)` configurations, so cycles
  terminate, and a cap (`200_000` configurations) marks results as partial rather than
  running without limit.
