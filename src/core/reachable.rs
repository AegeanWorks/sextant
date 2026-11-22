//! Pure breadth-first map of reachable configurations with shortest paths.

use std::collections::{BTreeSet, VecDeque};

use crate::core::engine::choose_transition;
use crate::core::format::format_env;
use crate::core::model::{Env, Event, Machine, State};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReachNode {
    pub state: State,
    pub env: Env,
    pub depth: usize,
    pub path: Vec<(State, Event)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reachability {
    pub machine: String,
    pub nodes: Vec<ReachNode>,
    pub truncated: bool,
    pub skipped_ambiguous: usize,
}

pub fn reachable(machine: &Machine, max_depth: usize, max_nodes: usize) -> Reachability {
    let mut visited: BTreeSet<(State, Env)> = BTreeSet::new();
    let mut queue: VecDeque<ReachNode> = VecDeque::new();
    queue.push_back(ReachNode {
        state: machine.initial.clone(),
        env: Env::new(),
        depth: 0,
        path: Vec::new(),
    });
    let mut nodes = Vec::new();
    let mut truncated = false;
    let mut skipped_ambiguous = 0usize;
    let mut events: Vec<Event> = machine.declared_events.clone();
    events.sort();

    while let Some(node) = queue.pop_front() {
        if !visited.insert((node.state.clone(), node.env.clone())) {
            continue;
        }
        if nodes.len() >= max_nodes {
            truncated = true;
            break;
        }
        nodes.push(node.clone());
        if node.depth >= max_depth {
            continue;
        }
        for event in &events {
            match choose_transition(machine, &node.state, &node.env, event) {
                Ok(Some((_, transition, next_env))) => {
                    let mut path = node.path.clone();
                    path.push((node.state.clone(), event.clone()));
                    queue.push_back(ReachNode {
                        state: transition.to.clone(),
                        env: next_env,
                        depth: node.depth + 1,
                        path,
                    });
                }
                Ok(None) => {}
                Err(_) => skipped_ambiguous += 1,
            }
        }
    }

    nodes.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.state.cmp(&right.state))
            .then_with(|| left.env.cmp(&right.env))
    });
    Reachability {
        machine: machine.name.clone(),
        nodes,
        truncated,
        skipped_ambiguous,
    }
}

fn path_text(node: &ReachNode) -> String {
    if node.path.is_empty() {
        return "-".to_string();
    }
    node.path
        .iter()
        .map(|(state, event)| format!("{}/{}", state.as_str(), event.as_str()))
        .collect::<Vec<_>>()
        .join(" -> ")
}

pub fn render_reachable(report: &Reachability) -> String {
    let mut lines = vec![
        "# sextant-reachable v1".to_string(),
        format!("machine: {}", report.machine),
        format!("configurations: {}", report.nodes.len()),
    ];
    if report.truncated {
        lines.push("warning: node cap reached; map is partial".to_string());
    }
    if report.skipped_ambiguous > 0 {
        lines.push(format!(
            "warning: {} ambiguous transition(s) skipped",
            report.skipped_ambiguous
        ));
    }
    for node in &report.nodes {
        lines.push(format!(
            "  - d={} state={} env={} path={}",
            node.depth,
            node.state.as_str(),
            format_env(&node.env),
            path_text(node)
        ));
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
