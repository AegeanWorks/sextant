//! Pure, reproducible fuzzing: generated scripts over a seed range.
//!
//! The PRNG is a pure value in `engine.rs`, so a fuzz result depends only on
//! the machine, the seed range and the requested length. A failing seed can be
//! fed to `minimize` to obtain a short reproducer.

use crate::core::engine::{generate_events, replay};
use crate::core::error::Error;
use crate::core::model::{Event, Machine};

const MAX_REPORTED_CASES: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuzzCase {
    pub seed: u64,
    pub reason: String,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuzzResult {
    pub machine: String,
    pub seeds: usize,
    pub events_per_seed: usize,
    pub failures: usize,
    pub cases: Vec<FuzzCase>,
}

fn failure_reason(machine: &Machine, events: &[Event], invariant: Option<&str>) -> Option<String> {
    match replay(machine, events) {
        Err(Error::Invariant { name, .. }) => {
            if invariant.map_or(true, |wanted| wanted == name) {
                Some(format!("invariant:{}", name))
            } else {
                None
            }
        }
        Err(Error::Stuck { .. }) => {
            if invariant.is_none() {
                Some("stuck".to_string())
            } else {
                None
            }
        }
        Err(Error::Ambiguous { .. }) => {
            if invariant.is_none() {
                Some("ambiguous".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn fuzz(machine: &Machine, seeds: usize, count: usize, invariant: Option<&str>) -> FuzzResult {
    let mut cases = Vec::new();
    let mut failures = 0usize;
    for seed in 0..seeds as u64 {
        let events = generate_events(machine, seed, count);
        if let Some(reason) = failure_reason(machine, &events, invariant) {
            failures += 1;
            if cases.len() < MAX_REPORTED_CASES {
                cases.push(FuzzCase {
                    seed,
                    reason,
                    events,
                });
            }
        }
    }
    FuzzResult {
        machine: machine.name.clone(),
        seeds,
        events_per_seed: count,
        failures,
        cases,
    }
}

pub fn render_fuzz(report: &FuzzResult) -> String {
    let mut lines = vec![
        "# sextant-fuzz v1".to_string(),
        format!("machine: {}", report.machine),
        format!("seeds: {}", report.seeds),
        format!("events-per-seed: {}", report.events_per_seed),
        format!("failures: {}", report.failures),
    ];
    if report.cases.len() < report.failures {
        lines.push(format!(
            "showing: {} of {} failing seeds",
            report.cases.len(),
            report.failures
        ));
    }
    for case in &report.cases {
        let events: Vec<&str> = case.events.iter().map(|event| event.as_str()).collect();
        lines.push(format!(
            "  - seed={} reason={} events={}",
            case.seed,
            case.reason,
            events.join(",")
        ));
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
