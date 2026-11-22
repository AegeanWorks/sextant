//! Explicit state machine for the CLI session lifecycle.
//!
//! This is the second state machine in Sextant (the first is the user's model).
//! It is a pure enum transition table: every command drives it, illegal
//! transitions are rejected, and the table is directly inspectable in tests.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Loaded,
    Verified,
    Replayed,
    Compared,
    Complete,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    Load,
    Verify,
    Replay,
    Compare,
    Finish,
    Fail,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IllegalTransition {
    pub from: SessionState,
    pub event: SessionEvent,
}

impl std::fmt::Display for IllegalTransition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "illegal transition {:?} --{:?}-->", self.from, self.event)
    }
}

/// The complete, closed transition table for a CLI session.
pub const TRANSITIONS: &[(SessionState, SessionEvent, SessionState)] = &[
    (SessionState::Idle, SessionEvent::Load, SessionState::Loaded),
    (SessionState::Loaded, SessionEvent::Verify, SessionState::Verified),
    (SessionState::Verified, SessionEvent::Replay, SessionState::Replayed),
    (SessionState::Replayed, SessionEvent::Compare, SessionState::Compared),
    (SessionState::Idle, SessionEvent::Finish, SessionState::Complete),
    (SessionState::Loaded, SessionEvent::Finish, SessionState::Complete),
    (SessionState::Verified, SessionEvent::Finish, SessionState::Complete),
    (SessionState::Replayed, SessionEvent::Finish, SessionState::Complete),
    (SessionState::Compared, SessionEvent::Finish, SessionState::Complete),
    (SessionState::Loaded, SessionEvent::Fail, SessionState::Failed),
    (SessionState::Verified, SessionEvent::Fail, SessionState::Failed),
    (SessionState::Replayed, SessionEvent::Fail, SessionState::Failed),
    (SessionState::Compared, SessionEvent::Fail, SessionState::Failed),
    (SessionState::Failed, SessionEvent::Reset, SessionState::Idle),
];

pub fn transition(
    state: SessionState,
    event: SessionEvent,
) -> Result<SessionState, IllegalTransition> {
    TRANSITIONS
        .iter()
        .find(|(from, candidate, _)| *from == state && *candidate == event)
        .map(|(_, _, to)| *to)
        .ok_or(IllegalTransition { from: state, event })
}
