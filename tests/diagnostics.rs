//! Tests for guard explanations and the reachable-state map.

use sextant::core::{
    explain, parse_events, parse_machine, reachable, render_explain, render_reachable, Event,
};

const RETRY: &str = include_str!("../examples/retry.sex");

#[test]
fn explain_shows_the_guard_and_its_actual_value() {
    let machine = parse_machine(RETRY).unwrap();
    let prefix = parse_events("START\nTICK\nTICK\n").unwrap();
    let report = explain(&machine, &prefix, &Event("TICK".to_string())).unwrap();
    assert_eq!(report.state, sextant::core::State("waiting".to_string()));
    assert_eq!(report.env.get("tries"), Some(&2));
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.enabled_count(), 1);
    assert_eq!(report.candidates[0].condition.as_deref(), Some("tries=2 <= 2"));
    let rendered = render_explain(&report);
    assert!(rendered.starts_with("# sextant-explain v1\n"));
    assert!(rendered.contains("when tries=2 <= 2 -> enabled"));
    assert_eq!(rendered, render_explain(&explain(&machine, &prefix, &Event("TICK".to_string())).unwrap()));
}

#[test]
fn explain_reports_no_candidate_for_an_unrelated_event() {
    let machine = parse_machine(RETRY).unwrap();
    let prefix = parse_events("START\n").unwrap();
    let report = explain(&machine, &prefix, &Event("RESET".to_string())).unwrap();
    assert_eq!(report.candidates.len(), 0);
    assert_eq!(report.enabled_count(), 0);
    assert!(render_explain(&report).contains("enabled: 0"));
}

#[test]
fn explain_reports_a_stuck_prefix_without_panicking() {
    let machine = parse_machine(RETRY).unwrap();
    let prefix = parse_events("TICK\n").unwrap();
    assert!(explain(&machine, &prefix, &Event("START".to_string())).is_err());
}

#[test]
fn reachable_map_is_deterministic_and_contains_the_initial_state() {
    let machine = parse_machine(RETRY).unwrap();
    let first = reachable(&machine, 5, 10_000);
    let second = reachable(&machine, 5, 10_000);
    assert_eq!(first, second);
    assert!(!first.nodes.is_empty());
    let initial = &first.nodes[0];
    assert_eq!(initial.depth, 0);
    assert!(initial.path.is_empty());
    let rendered = render_reachable(&first);
    assert!(rendered.starts_with("# sextant-reachable v1\n"));
    assert!(rendered.contains("state=idle env={} path=-"));
    assert_eq!(rendered, render_reachable(&second));
}
