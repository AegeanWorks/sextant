//! Pure structural diff between two machines. Elements are compared by their
//! canonical semantic text, so formatting and declaration order are ignored.

use std::collections::BTreeSet;

use crate::core::canonical::{render_property, render_transition};
use crate::core::format::format_condition;
use crate::core::json::Json;
use crate::core::model::Machine;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineDiff {
    pub left: String,
    pub right: String,
    pub initial_changed: Option<(String, String)>,
    pub states_added: Vec<String>,
    pub states_removed: Vec<String>,
    pub events_added: Vec<String>,
    pub events_removed: Vec<String>,
    pub transitions_added: Vec<String>,
    pub transitions_removed: Vec<String>,
    pub invariants_added: Vec<String>,
    pub invariants_removed: Vec<String>,
    pub properties_added: Vec<String>,
    pub properties_removed: Vec<String>,
}

impl MachineDiff {
    pub fn total_changes(&self) -> usize {
        let mut total = usize::from(self.initial_changed.is_some());
        total += self.states_added.len() + self.states_removed.len();
        total += self.events_added.len() + self.events_removed.len();
        total += self.transitions_added.len() + self.transitions_removed.len();
        total += self.invariants_added.len() + self.invariants_removed.len();
        total += self.properties_added.len() + self.properties_removed.len();
        total
    }
}

fn set(values: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

fn changes(left: BTreeSet<String>, right: BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    let added: Vec<String> = right.difference(&left).cloned().collect();
    let removed: Vec<String> = left.difference(&right).cloned().collect();
    (added, removed)
}

pub fn diff(left: &Machine, right: &Machine) -> MachineDiff {
    let (states_added, states_removed) = changes(
        set(left.declared_states.iter().map(|item| item.0.clone())),
        set(right.declared_states.iter().map(|item| item.0.clone())),
    );
    let (events_added, events_removed) = changes(
        set(left.declared_events.iter().map(|item| item.0.clone())),
        set(right.declared_events.iter().map(|item| item.0.clone())),
    );
    let (transitions_added, transitions_removed) = changes(
        set(left.transitions.iter().map(render_transition)),
        set(right.transitions.iter().map(render_transition)),
    );
    let (invariants_added, invariants_removed) = changes(
        set(
            left.invariants
                .iter()
                .map(|item| format!("{}: {}", item.name, format_condition(&item.condition))),
        ),
        set(
            right
                .invariants
                .iter()
                .map(|item| format!("{}: {}", item.name, format_condition(&item.condition))),
        ),
    );
    let (properties_added, properties_removed) = changes(
        set(left.properties.iter().map(render_property)),
        set(right.properties.iter().map(render_property)),
    );
    let initial_changed = if left.initial == right.initial {
        None
    } else {
        Some((left.initial.0.clone(), right.initial.0.clone()))
    };
    MachineDiff {
        left: left.name.clone(),
        right: right.name.clone(),
        initial_changed,
        states_added,
        states_removed,
        events_added,
        events_removed,
        transitions_added,
        transitions_removed,
        invariants_added,
        invariants_removed,
        properties_added,
        properties_removed,
    }
}

fn section(lines: &mut Vec<String>, label: &str, added: &[String], removed: &[String]) {
    if added.is_empty() && removed.is_empty() {
        return;
    }
    lines.push(format!("{}: {} added, {} removed", label, added.len(), removed.len()));
    for item in added {
        lines.push(format!("  + {}", item));
    }
    for item in removed {
        lines.push(format!("  - {}", item));
    }
}

pub fn render_diff(report: &MachineDiff) -> String {
    let mut lines = vec![
        "# sextant-diff v1".to_string(),
        format!("left: {}", report.left),
        format!("right: {}", report.right),
        format!("changes: {}", report.total_changes()),
    ];
    if let Some((old, new)) = &report.initial_changed {
        lines.push(format!("initial: {} -> {}", old, new));
    }
    section(&mut lines, "states", &report.states_added, &report.states_removed);
    section(&mut lines, "events", &report.events_added, &report.events_removed);
    section(
        &mut lines,
        "transitions",
        &report.transitions_added,
        &report.transitions_removed,
    );
    section(
        &mut lines,
        "invariants",
        &report.invariants_added,
        &report.invariants_removed,
    );
    section(
        &mut lines,
        "properties",
        &report.properties_added,
        &report.properties_removed,
    );
    let mut output = lines.join("\n");
    output.push('\n');
    output
}

fn names(values: &[String]) -> Json {
    Json::array(values.iter().map(|value| Json::string(value)).collect())
}

pub fn diff_json(report: &MachineDiff) -> Json {
    let initial = match &report.initial_changed {
        Some((old, new)) => Json::object(vec![
            ("old", Json::string(old)),
            ("new", Json::string(new)),
        ]),
        None => Json::Null,
    };
    Json::object(vec![
        ("left", Json::string(&report.left)),
        ("right", Json::string(&report.right)),
        ("changes", Json::number(report.total_changes())),
        ("initial", initial),
        ("states_added", names(&report.states_added)),
        ("states_removed", names(&report.states_removed)),
        ("events_added", names(&report.events_added)),
        ("events_removed", names(&report.events_removed)),
        ("transitions_added", names(&report.transitions_added)),
        ("transitions_removed", names(&report.transitions_removed)),
        ("invariants_added", names(&report.invariants_added)),
        ("invariants_removed", names(&report.invariants_removed)),
        ("properties_added", names(&report.properties_added)),
        ("properties_removed", names(&report.properties_removed)),
    ])
}
