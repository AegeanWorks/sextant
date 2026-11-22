//! Pure canonical rendering of a machine. Re-rendering is idempotent, and
//! declaration order does not affect the output, so model diffs show semantics.

use crate::core::error::Error;
use crate::core::format::{format_condition, format_effect};
use crate::core::model::{Machine, Property, PropertyKind, Transition};

pub fn render_transition(transition: &Transition) -> String {
    let mut line = format!(
        "transition {} {}",
        transition.from.as_str(),
        transition.event.as_str()
    );
    if let Some(guard) = &transition.guard {
        line.push_str(&format!(" when {}", format_condition(guard)));
    }
    if !transition.effects.is_empty() {
        let effects: Vec<String> = transition.effects.iter().map(format_effect).collect();
        line.push_str(&format!(" do {}", effects.join(", ")));
    }
    line.push_str(&format!(" -> {}", transition.to.as_str()));
    line
}

pub fn render_property(property: &Property) -> String {
    let kind = match &property.kind {
        PropertyKind::Reaches(state) => format!("reaches {}", state.as_str()),
        PropertyKind::Avoids(state) => format!("avoids {}", state.as_str()),
        PropertyKind::Emits(event) => format!("emits {}", event.as_str()),
    };
    format!("property {}: {}", property.name, kind)
}

/// A deterministic layout: fixed section order, sorted declarations, one
/// statement per line. Transition order is sorted by semantic text so two
/// files that differ only in declaration order produce identical output.
pub fn render_machine(machine: &Machine) -> String {
    let mut states: Vec<String> = machine
        .declared_states
        .iter()
        .map(|state| format!("state {}", state.as_str()))
        .collect();
    states.sort();
    let mut events: Vec<String> = machine
        .declared_events
        .iter()
        .map(|event| format!("event {}", event.as_str()))
        .collect();
    events.sort();
    let mut transitions: Vec<String> = machine.transitions.iter().map(render_transition).collect();
    transitions.sort();
    let mut invariants: Vec<String> = machine
        .invariants
        .iter()
        .map(|item| format!("invariant {}: {}", item.name, format_condition(&item.condition)))
        .collect();
    invariants.sort();
    let mut properties: Vec<String> = machine
        .properties
        .iter()
        .map(render_property)
        .collect();
    properties.sort();

    let mut sections = vec![
        format!("machine {}", machine.name),
        format!("initial {}", machine.initial.as_str()),
    ];
    sections.extend(states);
    sections.extend(events);
    sections.extend(transitions);
    sections.extend(invariants);
    sections.extend(properties);
    let mut output = sections.join("\n");
    output.push('\n');
    output
}

pub fn canonicalize(input: &str) -> Result<String, Error> {
    parse(input).map(|machine| render_machine(&machine))
}

pub fn parse(input: &str) -> Result<Machine, Error> {
    crate::core::parse::parse_machine(input)
}

pub fn is_canonical(input: &str) -> Result<bool, Error> {
    canonicalize(input).map(|canonical| canonical == input)
}
