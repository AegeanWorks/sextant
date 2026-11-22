//! Pure coverage analysis: which states, events and transitions did a trace use?

use std::collections::BTreeSet;

use crate::core::format::{format_condition, format_effect};
use crate::core::model::{Event, Machine, State, Trace};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub machine: String,
    pub states_total: usize,
    pub states_hit: Vec<State>,
    pub events_total: usize,
    pub events_hit: Vec<Event>,
    pub transitions_total: usize,
    pub transitions_hit: Vec<usize>,
}

pub fn percent(part: usize, total: usize) -> u64 {
    if total == 0 {
        return 100;
    }
    let numerator = (part as u64).saturating_mul(100);
    let denominator = total as u64;
    (numerator + denominator / 2) / denominator
}

impl Coverage {
    pub fn state_percent(&self) -> u64 {
        percent(self.states_hit.len(), self.states_total)
    }

    pub fn event_percent(&self) -> u64 {
        percent(self.events_hit.len(), self.events_total)
    }

    pub fn transition_percent(&self) -> u64 {
        percent(self.transitions_hit.len(), self.transitions_total)
    }
}

/// Coverage is derived from a trace: every step names the transition it used.
pub fn coverage(machine: &Machine, trace: &Trace) -> Coverage {
    let mut states: BTreeSet<State> = BTreeSet::new();
    let mut events: BTreeSet<Event> = BTreeSet::new();
    let mut transitions: BTreeSet<usize> = BTreeSet::new();
    states.insert(machine.initial.clone());
    for step in &trace.steps {
        states.insert(step.from.clone());
        states.insert(step.to.clone());
        events.insert(step.event.clone());
        transitions.insert(step.transition_index);
    }
    states.insert(trace.final_state.clone());
    Coverage {
        machine: machine.name.clone(),
        states_total: machine.declared_states.len(),
        states_hit: states.into_iter().collect(),
        events_total: machine.declared_events.len(),
        events_hit: events.into_iter().collect(),
        transitions_total: machine.transitions.len(),
        transitions_hit: transitions.into_iter().collect(),
    }
}

fn transition_label(machine: &Machine, index: usize) -> String {
    let transition = &machine.transitions[index];
    let mut label = format!(
        "{} --{}--> {}",
        transition.from.as_str(),
        transition.event.as_str(),
        transition.to.as_str()
    );
    if let Some(guard) = &transition.guard {
        label.push_str(&format!(" [when {}]", format_condition(guard)));
    }
    if !transition.effects.is_empty() {
        let effects: Vec<String> = transition.effects.iter().map(format_effect).collect();
        label.push_str(&format!(" [do {}]", effects.join(", ")));
    }
    label
}

pub fn render_coverage(machine: &Machine, report: &Coverage) -> String {
    let mut lines = vec![
        "# sextant-coverage v1".to_string(),
        format!("machine: {}", report.machine),
        format!(
            "states: {}/{} ({}%)",
            report.states_hit.len(),
            report.states_total,
            report.state_percent()
        ),
        format!(
            "events: {}/{} ({}%)",
            report.events_hit.len(),
            report.events_total,
            report.event_percent()
        ),
        format!(
            "transitions: {}/{} ({}%)",
            report.transitions_hit.len(),
            report.transitions_total,
            report.transition_percent()
        ),
    ];
    for index in &report.transitions_hit {
        lines.push(format!("covered: [{}] {}", index + 1, transition_label(machine, *index)));
    }
    let missed: Vec<usize> = (0..machine.transitions.len())
        .filter(|index| !report.transitions_hit.contains(index))
        .collect();
    if missed.is_empty() {
        lines.push("missed: none".to_string());
    } else {
        lines.push(format!("missed: {} transition(s)", missed.len()));
        for index in missed {
            lines.push(format!("  - [{}] {}", index + 1, transition_label(machine, index)));
        }
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
