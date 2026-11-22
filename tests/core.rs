//! Tests for the pure functional core. Every assertion here runs without I/O.

use sextant::core::{
    choose_transition, diff_traces, explore, generate_events, parse_events, parse_machine, replay,
    verify, Comparison, Event, State,
};
use std::collections::BTreeMap;

const RETRY: &str = include_str!("../examples/retry.sex");
const EVENTS: &str = include_str!("../examples/retry.events");

fn machine() -> sextant::core::Machine {
    parse_machine(RETRY).expect("example machine parses")
}

fn script() -> Vec<Event> {
    parse_events(EVENTS).expect("example event script parses")
}

#[test]
fn parses_the_example_machine() {
    let parsed = machine();
    assert_eq!(parsed.name, "retry");
    assert_eq!(parsed.initial, State("idle".to_string()));
    assert_eq!(parsed.declared_states.len(), 3);
    assert_eq!(parsed.declared_events.len(), 4);
    assert_eq!(parsed.transitions.len(), 4);
    assert_eq!(parsed.invariants.len(), 1);
    assert_eq!(parsed.invariants[0].name, "tries_bound");
    assert_eq!(parsed.invariants[0].condition.comparison, Comparison::Le);
}

#[test]
fn reports_parse_errors_with_line_numbers() {
    let error = parse_machine("machine broken\nstate idle\nbogus x\n").unwrap_err();
    assert_eq!(error.message(), "line 3: unknown directive 'bogus'");
}

#[test]
fn well_formed_machine_has_no_findings() {
    assert!(verify(&machine()).is_empty());
}

#[test]
fn verification_finds_unreachable_and_dead_end_states() {
    let text = "machine broken\ninitial idle\nstate idle\nstate waiting\nstate orphan\nevent GO\ntransition idle GO -> waiting\n";
    let parsed = parse_machine(text).unwrap();
    let findings = verify(&parsed);
    let messages: Vec<String> = findings.iter().map(|finding| finding.message()).collect();
    assert!(messages.iter().any(|message| message.contains("orphan")));
    assert!(messages.iter().any(|message| message.contains("no outgoing transitions")));
}

#[test]
fn verification_finds_identically_guarded_transitions() {
    let text = "machine dup\ninitial idle\nstate idle\nstate done\nevent GO\ntransition idle GO -> done\ntransition idle GO when tries == 0 -> done\ntransition idle GO when tries == 0 do tries += 1 -> done\n";
    let parsed = parse_machine(text).unwrap();
    assert!(verify(&parsed).iter().any(|finding| format!("{:?}", finding).contains("Ambiguous")));
}

#[test]
fn replay_is_deterministic_and_matches_expected_states() {
    let parsed = machine();
    let first = replay(&parsed, &script()).unwrap();
    let second = replay(&parsed, &script()).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.steps.len(), 4);
    assert_eq!(first.steps[0].to, State("waiting".to_string()));
    assert_eq!(first.steps[1].to_env.get("tries"), Some(&1));
    assert_eq!(first.steps[2].to_env.get("tries"), Some(&2));
    assert_eq!(first.final_state, State("done".to_string()));
    assert_eq!(first.final_env.get("tries"), Some(&2));
}

#[test]
fn guarded_transition_does_not_enable_when_its_condition_is_false() {
    let parsed = machine();
    let mut env = BTreeMap::new();
    env.insert("tries".to_string(), 3);
    let chosen = choose_transition(
        &parsed,
        &State("waiting".to_string()),
        &env,
        &Event("TICK".to_string()),
    )
    .unwrap();
    assert!(chosen.is_none());
}

#[test]
fn replay_reports_stuck_and_invariant_failures() {
    let parsed = machine();
    let stuck = replay(&parsed, &[Event("TICK".to_string())]).unwrap_err();
    assert!(stuck.message().contains("no enabled transition"));

    let strict = parse_machine(&RETRY.replace("tries <= 3", "tries <= 1")).unwrap();
    let violated = replay(&strict, &script()).unwrap_err();
    assert!(violated.message().contains("tries_bound"));
}

#[test]
fn exploration_finds_invariant_violations_deterministically() {
    let strict = parse_machine(&RETRY.replace("tries <= 3", "tries <= 1")).unwrap();
    let first = explore(&strict, 6, 10_000);
    let second = explore(&strict, 6, 10_000);
    assert_eq!(first, second);
    assert!(!first.violations.is_empty());
    assert_eq!(first.violations[0].invariant, "tries_bound");

    let safe = explore(&machine(), 6, 10_000);
    assert!(safe.violations.is_empty());
    assert!(safe.configurations > 0);
}

#[test]
fn seeded_generation_is_reproducible() {
    let parsed = machine();
    let first = generate_events(&parsed, 42, 30);
    let second = generate_events(&parsed, 42, 30);
    let other = generate_events(&parsed, 7, 30);
    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(first.len(), 30);
}

#[test]
fn trace_diff_reports_the_first_changed_line() {
    let difference = diff_traces("a\nb\nc\n", "a\nx\nc\n").unwrap();
    assert_eq!(difference.line, 2);
    assert_eq!(difference.expected.as_deref(), Some("b"));
    assert_eq!(difference.actual.as_deref(), Some("x"));
    assert!(diff_traces("a\n", "a\n").is_none());
}
