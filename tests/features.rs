//! Tests for the v0.2 pure additions: coverage, minimization and graph export.

use sextant::core::{
    coverage, minimizer_transition, minimize, parse_events, parse_graph_format, parse_machine,
    render_coverage, render_graph, render_minimize, replay, Error, GraphFormat, MinimizerEvent,
    MinimizerState, MINIMIZER_TRANSITIONS,
};

const RETRY: &str = include_str!("../examples/retry.sex");
const STRICT: &str = include_str!("../examples/strict.sex");
const FULL_EVENTS: &str = include_str!("../examples/retry.events");
const PARTIAL_EVENTS: &str = include_str!("../examples/partial.events");
const FAILING_EVENTS: &str = include_str!("../examples/failing.events");

#[test]
fn coverage_reports_full_and_partial_scripts_exactly() {
    let machine = parse_machine(RETRY).unwrap();
    let full = coverage(&machine, &replay(&machine, &parse_events(FULL_EVENTS).unwrap()).unwrap());
    assert_eq!(full.state_percent(), 100);
    assert_eq!(full.event_percent(), 75);
    assert_eq!(full.transition_percent(), 75);
    assert_eq!(full.transitions_hit, vec![0, 1, 2]);

    let partial =
        coverage(&machine, &replay(&machine, &parse_events(PARTIAL_EVENTS).unwrap()).unwrap());
    assert_eq!(partial.state_percent(), 67);
    assert_eq!(partial.event_percent(), 50);
    assert_eq!(partial.transition_percent(), 50);
    assert_eq!(partial.transitions_hit, vec![0, 1]);
}

#[test]
fn coverage_render_is_versioned_and_names_missed_edges() {
    let machine = parse_machine(RETRY).unwrap();
    let report = coverage(&machine, &replay(&machine, &parse_events(PARTIAL_EVENTS).unwrap()).unwrap());
    let text = render_coverage(&machine, &report);
    assert!(text.starts_with("# sextant-coverage v1\n"));
    assert!(text.contains("transitions: 2/4 (50%)"));
    assert!(text.contains("missed: 2 transition(s)"));
    assert!(text.contains("waiting --OK--> done"));
    assert!(text.contains("done --RESET--> idle"));
    assert_eq!(render_coverage(&machine, &report), text);
}

#[test]
fn minimizer_state_machine_is_explicit_and_closed() {
    let states = [
        MinimizerState::Trail,
        MinimizerState::Shrinking,
        MinimizerState::Minimal,
    ];
    let events = [
        MinimizerEvent::FailureConfirmed,
        MinimizerEvent::CandidateFails,
        MinimizerEvent::CandidatePasses,
        MinimizerEvent::NoCandidatesRemaining,
    ];
    for (from, event, to) in MINIMIZER_TRANSITIONS {
        assert_eq!(minimizer_transition(*from, *event), Ok(*to));
    }
    for from in states {
        for event in events {
            let declared = MINIMIZER_TRANSITIONS
                .iter()
                .any(|(a, b, _)| *a == from && *b == event);
            assert_eq!(minimizer_transition(from, event).is_ok(), declared);
        }
    }
    assert!(minimizer_transition(MinimizerState::Minimal, MinimizerEvent::CandidateFails).is_err());
    assert!(minimizer_transition(MinimizerState::Trail, MinimizerEvent::CandidateFails).is_err());
}

#[test]
fn minimize_keeps_only_the_events_needed_for_the_named_invariant() {
    let machine = parse_machine(STRICT).unwrap();
    let events = parse_events(FAILING_EVENTS).unwrap();
    let result = minimize(&machine, &events, Some("tries_bound")).unwrap();
    assert_eq!(result.original_len, 3);
    assert_eq!(result.minimized, events);
    assert_eq!(result.state, MinimizerState::Minimal);
    assert!(result.replays >= 3);
    assert_eq!(render_minimize(&result), render_minimize(&result));
    assert!(render_minimize(&result).starts_with("# sextant-minimize v1\n"));
}

#[test]
fn minimize_without_a_named_invariant_accepts_a_stuck_script() {
    let machine = parse_machine(STRICT).unwrap();
    let events = parse_events(FAILING_EVENTS).unwrap();
    let result = minimize(&machine, &events, None).unwrap();
    assert_eq!(result.minimized, vec![sextant::core::Event("TICK".to_string())]);
    assert!(result.replays > 3);
}

#[test]
fn minimize_refuses_a_script_that_does_not_fail() {
    let machine = parse_machine(RETRY).unwrap();
    let events = parse_events(FULL_EVENTS).unwrap();
    assert_eq!(minimize(&machine, &events, None).unwrap_err(), Error::NoFailure);
}

#[test]
fn graph_exports_are_deterministic_and_parse_their_format() {
    let machine = parse_machine(RETRY).unwrap();
    assert_eq!(parse_graph_format("dot"), Some(GraphFormat::Dot));
    assert_eq!(parse_graph_format("mermaid"), Some(GraphFormat::Mermaid));
    assert_eq!(parse_graph_format("png"), None);

    let dot = render_graph(&machine, GraphFormat::Dot);
    assert!(dot.starts_with("digraph retry {\n"));
    assert!(dot.contains("\"waiting\" -> \"done\" [label=\"OK\"];"));
    assert!(dot.contains("\"waiting\" -> \"waiting\" [label=\"TICK when tries <= 2 / tries += 1\"];"));
    assert_eq!(dot, render_graph(&machine, GraphFormat::Dot));

    let mermaid = render_graph(&machine, GraphFormat::Mermaid);
    assert!(mermaid.starts_with("stateDiagram-v2\n    [*] --> idle\n"));
    assert!(mermaid.contains("    idle --> waiting: START"));
    assert_eq!(mermaid, render_graph(&machine, GraphFormat::Mermaid));
}
