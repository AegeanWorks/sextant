//! Audit tests for the three architectural constraints.
//!
//! These tests are allowed to read source files; the core modules themselves
//! are not allowed to perform I/O. This is the executable form of G-001.

use std::fs;
use std::path::{Path, PathBuf};

fn project_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn functional_core_has_no_io_or_ambient_state() {
    let forbidden = [
        "std::fs",
        "std::io",
        "std::process",
        "std::time",
        "std::net",
        "std::env",
        "println!",
        "eprintln!",
        "File::open",
        "File::create",
        "Command::new",
    ];
    let mut checked = 0usize;
    for entry in fs::read_dir(project_file("src/core")).expect("core directory exists") {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("core source is UTF-8");
        for token in forbidden {
            assert!(
                !source.contains(token),
                "core module {} uses forbidden token '{}'",
                path.display(),
                token
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 15, "expected the fifteen core source files (fourteen modules plus mod.rs)");
}

#[test]
fn io_is_confined_to_the_edge_module() {
    let io_source = fs::read_to_string(project_file("src/shell/io.rs")).expect("io.rs exists");
    assert!(io_source.contains("std::fs"), "edge module owns file I/O");
    assert!(io_source.contains("write_all"), "edge module owns stdout writes");

    let shell_dir = fs::read_dir(project_file("src/shell")).expect("shell directory exists");
    for entry in shell_dir {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs")
            || path.ends_with("io.rs")
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("shell source is UTF-8");
        assert!(
            !source.contains("std::fs"),
            "{} bypasses the I/O edge",
            path.display()
        );
        assert!(
            !source.contains("write_all"),
            "{} bypasses the I/O edge",
            path.display()
        );
    }
}

#[test]
fn cli_session_is_an_explicit_closed_state_machine() {
    use sextant::shell::{
        session_transition, SessionEvent, SessionState, TRANSITIONS,
    };

    let states = [
        SessionState::Idle,
        SessionState::Loaded,
        SessionState::Verified,
        SessionState::Replayed,
        SessionState::Compared,
        SessionState::Complete,
        SessionState::Failed,
    ];
    let events = [
        SessionEvent::Load,
        SessionEvent::Verify,
        SessionEvent::Replay,
        SessionEvent::Compare,
        SessionEvent::Finish,
        SessionEvent::Fail,
        SessionEvent::Reset,
    ];

    // The table is closed: no (state, event) pair has two destinations.
    for left in 0..TRANSITIONS.len() {
        for right in (left + 1)..TRANSITIONS.len() {
            let (from_a, event_a, _) = TRANSITIONS[left];
            let (from_b, event_b, _) = TRANSITIONS[right];
            assert!(
                !(from_a == from_b && event_a == event_b),
                "duplicate transition for {:?}/{:?}",
                from_a,
                event_a
            );
        }
    }

    // Every legal pair resolves, every other pair is rejected.
    for (from, event, to) in TRANSITIONS {
        assert_eq!(session_transition(*from, *event), Ok(*to));
    }
    for from in states {
        for event in events {
            let declared = TRANSITIONS
                .iter()
                .any(|(a, b, _)| *a == from && *b == event);
            assert_eq!(session_transition(from, event).is_ok(), declared);
        }
    }

    // The happy path and the failure path are both reachable and closed.
    let happy = [
        (SessionState::Idle, SessionEvent::Load, SessionState::Loaded),
        (SessionState::Loaded, SessionEvent::Verify, SessionState::Verified),
        (
            SessionState::Verified,
            SessionEvent::Replay,
            SessionState::Replayed,
        ),
        (
            SessionState::Replayed,
            SessionEvent::Compare,
            SessionState::Compared,
        ),
        (
            SessionState::Compared,
            SessionEvent::Finish,
            SessionState::Complete,
        ),
    ];
    for (from, event, to) in happy {
        assert_eq!(session_transition(from, event), Ok(to));
    }
    assert_eq!(
        session_transition(SessionState::Replayed, SessionEvent::Fail),
        Ok(SessionState::Failed)
    );
    assert_eq!(
        session_transition(SessionState::Failed, SessionEvent::Reset),
        Ok(SessionState::Idle)
    );
    assert!(session_transition(SessionState::Complete, SessionEvent::Load).is_err());
    assert!(session_transition(SessionState::Failed, SessionEvent::Replay).is_err());
}
