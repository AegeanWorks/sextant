//! Pure graph rendering of a machine (DOT or Mermaid).

use crate::core::format::{format_condition, format_effect};
use crate::core::model::{Machine, State, Transition};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphFormat {
    Dot,
    Mermaid,
}

pub fn parse_graph_format(value: &str) -> Option<GraphFormat> {
    match value {
        "dot" => Some(GraphFormat::Dot),
        "mermaid" | "mmd" => Some(GraphFormat::Mermaid),
        _ => None,
    }
}

pub fn format_name(format: GraphFormat) -> &'static str {
    match format {
        GraphFormat::Dot => "dot",
        GraphFormat::Mermaid => "mermaid",
    }
}

fn edge_label(transition: &Transition) -> String {
    let mut label = transition.event.as_str().to_string();
    if let Some(guard) = &transition.guard {
        label.push_str(" when ");
        label.push_str(&format_condition(guard));
    }
    if !transition.effects.is_empty() {
        let effects: Vec<String> = transition.effects.iter().map(format_effect).collect();
        label.push_str(" / ");
        label.push_str(&effects.join(", "));
    }
    label
}

fn escape_dot(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sorted_states(machine: &Machine) -> Vec<&State> {
    let mut states: Vec<&State> = machine.declared_states.iter().collect();
    states.sort();
    states
}

pub fn render_graph(machine: &Machine, format: GraphFormat) -> String {
    match format {
        GraphFormat::Dot => render_dot(machine),
        GraphFormat::Mermaid => render_mermaid(machine),
    }
}

fn render_dot(machine: &Machine) -> String {
    let mut lines = vec![
        format!("digraph {} {{", escape_dot(&machine.name)),
        "  rankdir=LR;".to_string(),
    ];
    for state in sorted_states(machine) {
        lines.push(format!("  \"{}\";", escape_dot(state.as_str())));
    }
    for transition in &machine.transitions {
        lines.push(format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"];",
            escape_dot(transition.from.as_str()),
            escape_dot(transition.to.as_str()),
            escape_dot(&edge_label(transition))
        ));
    }
    lines.push("}".to_string());
    let mut output = lines.join("\n");
    output.push('\n');
    output
}

fn render_mermaid(machine: &Machine) -> String {
    let mut lines = vec!["stateDiagram-v2".to_string()];
    lines.push(format!("    [*] --> {}", machine.initial.as_str()));
    for transition in &machine.transitions {
        lines.push(format!(
            "    {} --> {}: {}",
            transition.from.as_str(),
            transition.to.as_str(),
            edge_label(transition)
        ));
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
