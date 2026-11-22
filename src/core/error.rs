//! Pure error and finding types. Formatting is a pure string operation; the
//! shell decides how and where to display these values.

use crate::core::model::{Event, State};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Parse { line: usize, message: String },
    Duplicate { what: &'static str, name: String },
    UnknownState(String),
    UnknownEvent(String),
    MissingInitial,
    MissingMachineName,
    Stuck { state: State, event: Event },
    Ambiguous { state: State, event: Event },
    NoFailure,
    Invariant { name: String, step: usize, state: State, env: String },
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            Self::Parse { line, message } => format!("line {}: {}", line, message),
            Self::Duplicate { what, name } => format!("duplicate {}: '{}'", what, name),
            Self::UnknownState(name) => format!("unknown state: '{}'", name),
            Self::UnknownEvent(name) => format!("unknown event: '{}'", name),
            Self::MissingInitial => "missing 'initial' declaration".to_string(),
            Self::MissingMachineName => "missing 'machine' declaration".to_string(),
            Self::Stuck { state, event } => {
                format!("no enabled transition for event '{}' in state '{}'", event.as_str(), state.as_str())
            }
            Self::Ambiguous { state, event } => format!(
                "ambiguous transition for event '{}' in state '{}': more than one guard holds",
                event.as_str(),
                state.as_str()
            ),
            Self::NoFailure => "the event script does not fail; nothing to minimize".to_string(),
            Self::Invariant { name, step, state, env } => format!(
                "invariant '{}' violated at step {} in state '{}' with env {}",
                name, step, state.as_str(), env
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Finding {
    UnknownInitial(String),
    UndeclaredState { state: String, context: String },
    UndeclaredEvent { event: String, context: String },
    AmbiguousTransition { state: String, event: String },
    ShadowedTransition {
        state: String,
        event: String,
        shadowed: usize,
        by: usize,
    },
    UnreachableState(String),
    DeadEndState(String),
}

impl Finding {
    pub fn message(&self) -> String {
        match self {
            Self::UnknownInitial(name) => format!("initial state '{}' is not declared", name),
            Self::UndeclaredState { state, context } => {
                format!("{} uses undeclared state '{}'", context, state)
            }
            Self::UndeclaredEvent { event, context } => {
                format!("{} uses undeclared event '{}'", context, event)
            }
            Self::AmbiguousTransition { state, event } => format!(
                "transition ('{}', '{}') has two identically guarded definitions",
                state, event
            ),
            Self::ShadowedTransition {
                state,
                event,
                shadowed,
                by,
            } => format!(
                "transition {} ('{}', '{}') is shadowed by earlier transition {}",
                shadowed + 1,
                state,
                event,
                by + 1
            ),
            Self::UnreachableState(name) => format!("state '{}' is unreachable from the initial state", name),
            Self::DeadEndState(name) => format!("state '{}' has no outgoing transitions", name),
        }
    }
}
