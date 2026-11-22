//! Tests for trace recovery and machine slicing.

use sextant::core::{
    parse_events, parse_machine, render_events, render_trace, replay, slice, untrace, verify,
};

const RETRY: &str = include_str!("../examples/retry.sex");
const PROPERTIES: &str = include_str!("../examples/properties.sex");
const EVENTS: &str = include_str!("../examples/retry.events");

#[test]
fn a_trace_round_trips_to_its_event_script() {
    let machine = parse_machine(RETRY).unwrap();
    let events = parse_events(EVENTS).unwrap();
    let trace = replay(&machine, &events).unwrap();
    let recovered = untrace(&render_trace(&trace)).unwrap();
    assert_eq!(recovered, events);
    assert_eq!(render_events(&recovered), EVENTS);
}

#[test]
fn malformed_traces_report_line_and_reason() {
    let bad_header = untrace("not a trace\nstep 001: START | a -> b\n").unwrap_err();
    assert_eq!(bad_header.message(), "line 1: expected trace header '# sextant-trace v1', got 'not a trace'");
    let bad_step = untrace("# sextant-trace v1\nstep 001 START | a -> b\n").unwrap_err();
    assert!(bad_step.message().contains("line 2"));
}

#[test]
fn sliced_machine_replays_the_same_script() {
    let machine = parse_machine(RETRY).unwrap();
    let events = parse_events(EVENTS).unwrap();
    let trace = replay(&machine, &events).unwrap();
    let report = slice(&machine, &trace);
    assert_eq!(report.removed_transitions, 1);
    assert_eq!(report.removed_events, 1);
    assert_eq!(report.removed_states, 0);
    assert!(report.dropped_properties.is_empty());
    // The slice declares its final state terminal, so a dead end is not a finding.
    assert_eq!(report.machine.terminals, vec![sextant::core::State("done".to_string())]);
    assert!(verify(&report.machine).is_empty());
    let sliced_trace = replay(&report.machine, &events).unwrap();
    assert_eq!(sliced_trace.steps, trace.steps);
    assert_eq!(sliced_trace.final_env, trace.final_env);
}

#[test]
fn slicing_names_properties_it_must_drop() {
    let machine = parse_machine(PROPERTIES).unwrap();
    let events = parse_events("START\n").unwrap();
    let trace = replay(&machine, &events).unwrap();
    let report = slice(&machine, &trace);
    assert_eq!(
        report.dropped_properties,
        vec![
            "property emits_ok: emits OK".to_string(),
            "property reaches_done: reaches done".to_string(),
        ]
    );
    assert_eq!(report.machine.properties.len(), 1);
}
