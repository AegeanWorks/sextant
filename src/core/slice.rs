//! Pure projection of a machine onto the transitions a trace actually used.

use std::collections::BTreeSet;

use crate::core::canonical::render_property;
use crate::core::model::{Event, Machine, PropertyKind, State, Trace};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SliceResult {
    pub machine: Machine,
    pub removed_states: usize,
    pub removed_events: usize,
    pub removed_transitions: usize,
    pub dropped_properties: Vec<String>,
}

/// Keep only transitions recorded in the trace, then keep only the states and
/// events they mention. Properties whose symbols disappear are dropped and named;
/// invariants survive because they constrain the integer environment, not the graph.
pub fn slice(machine: &Machine, trace: &Trace) -> SliceResult {
    let used: BTreeSet<usize> = trace.steps.iter().map(|step| step.transition_index).collect();
    let transitions: Vec<crate::core::model::Transition> = machine
        .transitions
        .iter()
        .enumerate()
        .filter(|(index, _)| used.contains(index))
        .map(|(_, transition)| transition.clone())
        .collect();

    let mut states: BTreeSet<State> = BTreeSet::new();
    states.insert(machine.initial.clone());
    let mut events: BTreeSet<Event> = BTreeSet::new();
    for transition in &transitions {
        states.insert(transition.from.clone());
        states.insert(transition.to.clone());
        events.insert(transition.event.clone());
    }

    let mut dropped_properties = Vec::new();
    let mut properties = Vec::new();
    for property in &machine.properties {
        let keep = match &property.kind {
            PropertyKind::Reaches(state) | PropertyKind::Avoids(state) => states.contains(state),
            PropertyKind::Emits(event) => events.contains(event),
        };
        if keep {
            properties.push(property.clone());
        } else {
            dropped_properties.push(render_property(property));
        }
    }

    dropped_properties.sort();
    // The script stops in this state, so declaring it terminal keeps the slice
    // verifiable without weakening dead-end detection for undeclared sinks.
    let mut terminals = machine.terminals.clone();
    if !terminals.contains(&trace.final_state) {
        terminals.push(trace.final_state.clone());
    }
    terminals.sort();

    let removed_states = machine.declared_states.len().saturating_sub(states.len());
    let removed_events = machine.declared_events.len().saturating_sub(events.len());
    let removed_transitions = machine.transitions.len().saturating_sub(transitions.len());
    let sliced = Machine {
        name: format!("{}_slice", machine.name),
        initial: machine.initial.clone(),
        declared_states: states.into_iter().collect(),
        terminals,
        declared_events: events.into_iter().collect(),
        transitions,
        invariants: machine.invariants.clone(),
        properties,
    };
    SliceResult {
        machine: sliced,
        removed_states,
        removed_events,
        removed_transitions,
        dropped_properties,
    }
}

pub fn render_slice_report(report: &SliceResult) -> String {
    let mut lines = vec![
        "# sextant-slice v1".to_string(),
        format!("machine: {}", report.machine.name),
        format!("removed-states: {}", report.removed_states),
        format!("removed-events: {}", report.removed_events),
        format!("removed-transitions: {}", report.removed_transitions),
        format!("dropped-properties: {}", report.dropped_properties.len()),
    ];
    for property in &report.dropped_properties {
        lines.push(format!("  - {}", property));
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
