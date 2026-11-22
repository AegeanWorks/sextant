//! Pure execution engine: transition selection, replay, static verification,
//! bounded exploration and seeded event generation.
//!
//! Every function in this module is total with respect to its inputs: no I/O,
//! no clock, no ambient state. Given the same `Machine` and events, the result
//! is byte-for-byte identical on every machine.

use std::collections::{BTreeSet, VecDeque};

use crate::core::error::{Error, Finding};
use crate::core::format::format_env;
use crate::core::model::{Env, Event, Machine, State, Trace, TraceStep};

/// A deterministic 64-bit PRNG (xorshift64*). It is a pure value: the seed is
/// the only source of variation, so generated scripts replay exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x243F_6A88_85A3_08D3 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() % bound as u64) as usize
    }
}

/// Pick the single enabled transition for `(state, event, env)`.
///
/// `None` means no guard holds. Two simultaneously holding guards are a real
/// nondeterminism and are reported as an error rather than resolved silently.
pub fn choose_transition<'a>(
    machine: &'a Machine,
    state: &State,
    env: &Env,
    event: &Event,
) -> Result<Option<(usize, &'a crate::core::model::Transition, Env)>, Error> {
    let mut enabled: Vec<(usize, &'a crate::core::model::Transition)> = machine
        .transitions
        .iter()
        .enumerate()
        .filter(|(_, transition)| &transition.from == state && &transition.event == event)
        .filter(|(_, transition)| {
            transition
                .guard
                .as_ref()
                .map_or(true, |condition| condition.holds(env))
        })
        .collect();

    if enabled.len() > 1 {
        return Err(Error::Ambiguous {
            state: state.clone(),
            event: event.clone(),
        });
    }

    match enabled.pop() {
        None => Ok(None),
        Some((index, transition)) => {
            let mut next_env = env.clone();
            for effect in &transition.effects {
                effect.apply(&mut next_env);
            }
            Ok(Some((index, transition, next_env)))
        }
    }
}

fn check_invariants(machine: &Machine, step: usize, state: &State, env: &Env) -> Result<(), Error> {
    for invariant in &machine.invariants {
        if !invariant.condition.holds(env) {
            return Err(Error::Invariant {
                name: invariant.name.clone(),
                step,
                state: state.clone(),
                env: format_env(env),
            });
        }
    }
    Ok(())
}

/// Replay an event script from the machine's initial state.
pub fn replay(machine: &Machine, events: &[Event]) -> Result<Trace, Error> {
    let mut state = machine.initial.clone();
    let mut env = Env::new();
    check_invariants(machine, 0, &state, &env)?;

    let mut steps = Vec::with_capacity(events.len());
    for (offset, event) in events.iter().enumerate() {
        let step_number = offset + 1;
        let (index, transition, next_env) = choose_transition(machine, &state, &env, event)?
            .ok_or_else(|| Error::Stuck {
                state: state.clone(),
                event: event.clone(),
            })?;
        steps.push(TraceStep {
            index: step_number,
            event: event.clone(),
            from: state.clone(),
            from_env: env.clone(),
            to: transition.to.clone(),
            to_env: next_env.clone(),
            transition_index: index,
        });
        state = transition.to.clone();
        env = next_env;
        check_invariants(machine, step_number, &state, &env)?;
    }

    Ok(Trace {
        machine: machine.name.clone(),
        steps,
        final_state: state,
        final_env: env,
    })
}

/// Static well-formedness checks. An empty result means the machine is sound.
pub fn verify(machine: &Machine) -> Vec<Finding> {
    let mut findings = Vec::new();

    if !machine.has_state(&machine.initial) {
        findings.push(Finding::UnknownInitial(machine.initial.0.clone()));
    }

    for (offset, transition) in machine.transitions.iter().enumerate() {
        let context = format!("transition {}", offset + 1);
        if !machine.has_state(&transition.from) {
            findings.push(Finding::UndeclaredState {
                state: transition.from.0.clone(),
                context: context.clone(),
            });
        }
        if !machine.has_state(&transition.to) {
            findings.push(Finding::UndeclaredState {
                state: transition.to.0.clone(),
                context: context.clone(),
            });
        }
        if !machine.has_event(&transition.event) {
            findings.push(Finding::UndeclaredEvent {
                event: transition.event.0.clone(),
                context,
            });
        }
    }

    // Two definitions with the same (from, event, guard) always conflict.
    for left in 0..machine.transitions.len() {
        for right in (left + 1)..machine.transitions.len() {
            let a = &machine.transitions[left];
            let b = &machine.transitions[right];
            if a.from == b.from && a.event == b.event && a.guard == b.guard {
                findings.push(Finding::AmbiguousTransition {
                    state: a.from.0.clone(),
                    event: a.event.0.clone(),
                });
            }
        }
    }

    // Guard-agnostic reachability is a sound over-approximation: a state it
    // calls unreachable is genuinely unreachable for every possible guard.
    let mut reachable: BTreeSet<State> = BTreeSet::new();
    let mut queue: VecDeque<State> = VecDeque::new();
    if machine.has_state(&machine.initial) {
        reachable.insert(machine.initial.clone());
        queue.push_back(machine.initial.clone());
    }
    while let Some(state) = queue.pop_front() {
        for transition in &machine.transitions {
            if transition.from == state
                && machine.has_state(&transition.to)
                && reachable.insert(transition.to.clone())
            {
                queue.push_back(transition.to.clone());
            }
        }
    }
    for state in &machine.declared_states {
        if !reachable.contains(state) {
            findings.push(Finding::UnreachableState(state.0.clone()));
        }
        if !machine.transitions.iter().any(|item| &item.from == state) {
            findings.push(Finding::DeadEndState(state.0.clone()));
        }
    }

    findings.sort();
    findings.dedup();
    findings
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub path: Vec<(State, Event)>,
    pub invariant: String,
    pub state: State,
    pub env: Env,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExploreResult {
    pub configurations: usize,
    pub transitions_taken: usize,
    pub violations: Vec<Violation>,
    pub truncated: bool,
}

/// Breadth-first enumeration of reachable `(state, env)` configurations up to
/// `max_depth`. Visited configurations are deduplicated, so cyclic machines
/// terminate. `max_configs` bounds the work and sets `truncated` when hit.
pub fn explore(machine: &Machine, max_depth: usize, max_configs: usize) -> ExploreResult {
    let mut visited: BTreeSet<(State, Env)> = BTreeSet::new();
    let mut queue: VecDeque<(State, Env, usize, Vec<(State, Event)>)> = VecDeque::new();
    queue.push_back((machine.initial.clone(), Env::new(), 0, Vec::new()));

    let mut configurations = 0usize;
    let mut transitions_taken = 0usize;
    let mut violations = Vec::new();
    let mut truncated = false;
    let mut events: Vec<Event> = machine.declared_events.clone();
    events.sort();

    while let Some((state, env, depth, path)) = queue.pop_front() {
        if !visited.insert((state.clone(), env.clone())) {
            continue;
        }
        configurations += 1;
        if configurations > max_configs {
            truncated = true;
            break;
        }
        for invariant in &machine.invariants {
            if !invariant.condition.holds(&env) {
                violations.push(Violation {
                    path: path.clone(),
                    invariant: invariant.name.clone(),
                    state: state.clone(),
                    env: env.clone(),
                });
            }
        }
        if depth >= max_depth {
            continue;
        }
        for event in &events {
            if let Ok(Some((_, transition, next_env))) =
                choose_transition(machine, &state, &env, event)
            {
                transitions_taken += 1;
                let mut next_path = path.clone();
                next_path.push((state.clone(), event.clone()));
                queue.push_back((transition.to.clone(), next_env, depth + 1, next_path));
            }
        }
    }

    ExploreResult {
        configurations,
        transitions_taken,
        violations,
        truncated,
    }
}

/// Generate a deterministic event script from a seed and the machine's events.
pub fn generate_events(machine: &Machine, seed: u64, count: usize) -> Vec<Event> {
    let mut events: Vec<Event> = machine.declared_events.clone();
    events.sort();
    if events.is_empty() {
        return Vec::new();
    }
    let mut rng = Rng::new(seed);
    (0..count)
        .map(|_| events[rng.below(events.len())].clone())
        .collect()
}
