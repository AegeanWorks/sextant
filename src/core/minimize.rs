//! Pure reduction of a failing event script to a 1-minimal reproducer.
//!
//! The reduction loop is itself an explicit state machine: its states and
//! transition table are public and checked exhaustively by the test suite.

use crate::core::engine::replay;
use crate::core::error::Error;
use crate::core::model::{Event, Machine};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimizerState {
    Trail,
    Shrinking,
    Minimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimizerEvent {
    FailureConfirmed,
    CandidateFails,
    CandidatePasses,
    NoCandidatesRemaining,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IllegalMinimizerTransition {
    pub from: MinimizerState,
    pub event: MinimizerEvent,
}

pub const TRANSITIONS: &[(MinimizerState, MinimizerEvent, MinimizerState)] = &[
    (MinimizerState::Trail, MinimizerEvent::FailureConfirmed, MinimizerState::Shrinking),
    (MinimizerState::Shrinking, MinimizerEvent::CandidateFails, MinimizerState::Shrinking),
    (MinimizerState::Shrinking, MinimizerEvent::CandidatePasses, MinimizerState::Shrinking),
    (MinimizerState::Shrinking, MinimizerEvent::NoCandidatesRemaining, MinimizerState::Minimal),
];

pub fn minimizer_transition(
    state: MinimizerState,
    event: MinimizerEvent,
) -> Result<MinimizerState, IllegalMinimizerTransition> {
    TRANSITIONS
        .iter()
        .find(|(from, candidate, _)| *from == state && *candidate == event)
        .map(|(_, _, to)| *to)
        .ok_or(IllegalMinimizerTransition { from: state, event })
}

/// Whether a script fails. With `invariant`, only that invariant counts and a
/// stuck replay is not a failure; without it, any replay error counts.
pub fn fails(machine: &Machine, events: &[Event], invariant: Option<&str>) -> bool {
    match replay(machine, events) {
        Err(Error::Invariant { name, .. }) => invariant.map_or(true, |wanted| wanted == name),
        Err(_) => invariant.is_none(),
        Ok(_) => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinimizeResult {
    pub original_len: usize,
    pub minimized: Vec<Event>,
    pub replays: usize,
    pub state: MinimizerState,
}

/// Greedy 1-minimality: no single event can be removed while keeping the failure.
/// Deterministic because candidates are always tried in their original order.
pub fn minimize(
    machine: &Machine,
    events: &[Event],
    invariant: Option<&str>,
) -> Result<MinimizeResult, Error> {
    if !fails(machine, events, invariant) {
        return Err(Error::NoFailure);
    }

    let mut state = minimizer_transition(MinimizerState::Trail, MinimizerEvent::FailureConfirmed)
        .expect("Trail accepts FailureConfirmed");
    let mut current = events.to_vec();
    let mut replays = 1usize;
    let mut removed_any = true;

    while removed_any {
        removed_any = false;
        let mut index = 0usize;
        while index < current.len() {
            let mut candidate = current.clone();
            candidate.remove(index);
            replays += 1;
            if fails(machine, &candidate, invariant) {
                current = candidate;
                removed_any = true;
                state = minimizer_transition(state, MinimizerEvent::CandidateFails)
                    .expect("Shrinking accepts CandidateFails");
            } else {
                index += 1;
                state = minimizer_transition(state, MinimizerEvent::CandidatePasses)
                    .expect("Shrinking accepts CandidatePasses");
            }
        }
    }

    state = minimizer_transition(state, MinimizerEvent::NoCandidatesRemaining)
        .expect("Shrinking accepts NoCandidatesRemaining");
    Ok(MinimizeResult {
        original_len: events.len(),
        minimized: current,
        replays,
        state,
    })
}

pub fn render_minimize(report: &MinimizeResult) -> String {
    let mut lines = vec![
        "# sextant-minimize v1".to_string(),
        format!("original: {} event(s)", report.original_len),
        format!("minimal: {} event(s)", report.minimized.len()),
        format!("replays: {}", report.replays),
        format!("state: {:?}", report.state),
        "events:".to_string(),
    ];
    for event in &report.minimized {
        lines.push(format!("  - {}", event.as_str()));
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
