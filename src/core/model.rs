//! Pure data types for the Sextant domain. No I/O, no clock, no randomness.
//!
//! A `Machine` is a finite state machine with a small integer environment.
//! Transitions may be guarded by a `Condition` and may mutate the environment
//! with `Effect`s. Every type here is `Clone + Debug + PartialEq + Eq` so that
//! complete machine runs can be compared and stored as golden fixtures.

use std::collections::BTreeMap;

/// Environment of named integers. `BTreeMap` keeps rendering deterministic.
pub type Env = BTreeMap<String, i64>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State(pub String);

impl State {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Event(pub String);

impl Event {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Comparison {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl Comparison {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Eq => "==",
            Self::Ne => "!=",
        }
    }

    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "<" => Some(Self::Lt),
            "<=" => Some(Self::Le),
            ">" => Some(Self::Gt),
            ">=" => Some(Self::Ge),
            "==" => Some(Self::Eq),
            "!=" => Some(Self::Ne),
            _ => None,
        }
    }

    pub fn test(self, left: i64, right: i64) -> bool {
        match self {
            Self::Lt => left < right,
            Self::Le => left <= right,
            Self::Gt => left > right,
            Self::Ge => left >= right,
            Self::Eq => left == right,
            Self::Ne => left != right,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Literal(i64),
    Variable(String),
}

impl Operand {
    /// An unbound variable reads as zero. This keeps every evaluation total.
    pub fn value(&self, env: &Env) -> i64 {
        match self {
            Self::Literal(value) => *value,
            Self::Variable(name) => env.get(name).copied().unwrap_or(0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Condition {
    pub left: Operand,
    pub comparison: Comparison,
    pub right: Operand,
}

impl Condition {
    pub fn holds(&self, env: &Env) -> bool {
        self.comparison.test(self.left.value(env), self.right.value(env))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Set(String, i64),
    Add(String, i64),
    Subtract(String, i64),
}

impl Effect {
    pub fn apply(&self, env: &mut Env) {
        match self {
            Self::Set(name, value) => {
                env.insert(name.clone(), *value);
            }
            Self::Add(name, delta) => {
                let slot = env.entry(name.clone()).or_insert(0);
                *slot = slot.saturating_add(*delta);
            }
            Self::Subtract(name, delta) => {
                let slot = env.entry(name.clone()).or_insert(0);
                *slot = slot.saturating_sub(*delta);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub from: State,
    pub event: Event,
    pub guard: Option<Condition>,
    pub to: State,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invariant {
    pub name: String,
    pub condition: Condition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    pub name: String,
    pub initial: State,
    pub declared_states: Vec<State>,
    pub declared_events: Vec<Event>,
    pub transitions: Vec<Transition>,
    pub invariants: Vec<Invariant>,
}

impl Machine {
    pub fn has_state(&self, state: &State) -> bool {
        self.declared_states.contains(state)
    }

    pub fn has_event(&self, event: &Event) -> bool {
        self.declared_events.contains(event)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceStep {
    pub index: usize,
    pub event: Event,
    pub from: State,
    pub from_env: Env,
    pub to: State,
    pub to_env: Env,
    pub transition_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    pub machine: String,
    pub steps: Vec<TraceStep>,
    pub final_state: State,
    pub final_env: Env,
}
