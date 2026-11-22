//! Bounded behavioural equivalence between two machines.
//!
//! State labels are deliberately not compared: a refactor may rename states.
//! Two machines are equivalent when every event sequence up to `depth` produces
//! the same observable outcome — accepted step count, final environment, or a
//! failure category.

use std::collections::{BTreeSet, VecDeque};

use crate::core::engine::replay;
use crate::core::error::Error;
use crate::core::format::format_env;
use crate::core::model::{Event, Machine};

const MAX_REPORTED_DIFFERENCES: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquivalenceDifference {
    pub path: Vec<Event>,
    pub left: String,
    pub right: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquivalenceResult {
    pub depth: usize,
    pub sequences: usize,
    pub differences: Vec<EquivalenceDifference>,
    pub truncated: bool,
}

fn observable(machine: &Machine, events: &[Event]) -> String {
    match replay(machine, events) {
        Ok(trace) => format!(
            "ok:steps={}:env={}",
            trace.steps.len(),
            format_env(&trace.final_env)
        ),
        Err(Error::Invariant { name, .. }) => format!("err:invariant:{}", name),
        Err(Error::Stuck { .. }) => "err:stuck".to_string(),
        Err(Error::Ambiguous { .. }) => "err:ambiguous".to_string(),
        Err(_) => "err:other".to_string(),
    }
}

/// Enumerate every event sequence up to `depth` and compare observable outcomes.
/// `max_sequences` bounds the work and sets `truncated` when hit.
pub fn equivalent(
    left: &Machine,
    right: &Machine,
    depth: usize,
    max_sequences: usize,
) -> EquivalenceResult {
    let alphabet: BTreeSet<Event> = left
        .declared_events
        .iter()
        .chain(right.declared_events.iter())
        .cloned()
        .collect();
    let alphabet: Vec<Event> = alphabet.into_iter().collect();

    let mut queue: VecDeque<Vec<Event>> = VecDeque::new();
    queue.push_back(Vec::new());
    let mut sequences = 0usize;
    let mut differences = Vec::new();
    let mut truncated = false;

    while let Some(path) = queue.pop_front() {
        sequences += 1;
        if sequences > max_sequences {
            truncated = true;
            break;
        }
        let left_outcome = observable(left, &path);
        let right_outcome = observable(right, &path);
        if left_outcome != right_outcome && differences.len() < MAX_REPORTED_DIFFERENCES {
            differences.push(EquivalenceDifference {
                path: path.clone(),
                left: left_outcome,
                right: right_outcome,
            });
        }
        if path.len() >= depth {
            continue;
        }
        for event in &alphabet {
            let mut next = path.clone();
            next.push(event.clone());
            queue.push_back(next);
        }
    }

    EquivalenceResult {
        depth,
        sequences,
        differences,
        truncated,
    }
}

pub fn render_equivalence(report: &EquivalenceResult, left: &str, right: &str) -> String {
    let mut lines = vec![
        "# sextant-equivalence v1".to_string(),
        format!("left: {}", left),
        format!("right: {}", right),
        format!("depth: {}", report.depth),
        format!("sequences: {}", report.sequences),
        format!("differences: {}", report.differences.len()),
    ];
    if report.truncated {
        lines.push("warning: sequence cap reached; comparison is partial".to_string());
    }
    for difference in &report.differences {
        let path: Vec<&str> = difference.path.iter().map(|event| event.as_str()).collect();
        lines.push(format!(
            "  - [{}] left={} right={}",
            path.join(","),
            difference.left,
            difference.right
        ));
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
