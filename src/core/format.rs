//! Pure rendering and diffing of traces and findings.

use crate::core::error::Finding;
use crate::core::model::{Condition, Effect, Env, Operand, Trace};

fn format_operand(operand: &Operand) -> String {
    match operand {
        Operand::Literal(value) => value.to_string(),
        Operand::Variable(name) => name.clone(),
    }
}

pub fn format_condition(condition: &Condition) -> String {
    format!(
        "{} {} {}",
        format_operand(&condition.left),
        condition.comparison.symbol(),
        format_operand(&condition.right)
    )
}

pub fn format_effect(effect: &Effect) -> String {
    match effect {
        Effect::Set(name, value) => format!("{} = {}", name, value),
        Effect::Add(name, value) => format!("{} += {}", name, value),
        Effect::Subtract(name, value) => format!("{} -= {}", name, value),
    }
}

pub fn format_env(env: &Env) -> String {
    if env.is_empty() {
        return "{}".to_string();
    }
    let body = env
        .iter()
        .map(|(name, value)| format!("{}={}", name, value))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{}}}", body)
}

/// The versioned trace format is the contract golden files depend on.
pub fn render_trace(trace: &Trace) -> String {
    let mut lines = vec![
        "# sextant-trace v1".to_string(),
        format!("machine: {}", trace.machine),
        format!("initial: {}", format_env(&Env::new())),
    ];
    for step in &trace.steps {
        lines.push(format!(
            "step {:03}: {} | {} {} -> {} {}",
            step.index,
            step.event.as_str(),
            step.from.as_str(),
            format_env(&step.from_env),
            step.to.as_str(),
            format_env(&step.to_env)
        ));
    }
    lines.push(format!(
        "final: {} {}",
        trace.final_state.as_str(),
        format_env(&trace.final_env)
    ));
    let mut output = lines.join("\n");
    output.push('\n');
    output
}

pub fn render_findings(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "ok: no findings".to_string();
    }
    let mut lines = vec![format!("{} finding(s):", findings.len())];
    for finding in findings {
        lines.push(format!("  - {}", finding.message()));
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceDiff {
    pub line: usize,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

/// First differing line, ignoring a trailing newline only.
pub fn diff_traces(expected: &str, actual: &str) -> Option<TraceDiff> {
    let left: Vec<&str> = expected.lines().collect();
    let right: Vec<&str> = actual.lines().collect();
    let longer = left.len().max(right.len());
    for index in 0..longer {
        let expected_line = left.get(index).map(|line| (*line).to_string());
        let actual_line = right.get(index).map(|line| (*line).to_string());
        if expected_line != actual_line {
            return Some(TraceDiff {
                line: index + 1,
                expected: expected_line,
                actual: actual_line,
            });
        }
    }
    None
}
