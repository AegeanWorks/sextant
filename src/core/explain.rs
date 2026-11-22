//! Pure guard diagnostics. The current state and environment are obtained by
//! replaying a prefix script, so the explanation describes a real execution.

use crate::core::engine::replay;
use crate::core::error::Error;
use crate::core::format::{format_effect, format_env};
use crate::core::model::{Condition, Env, Event, Machine, Operand, State};

fn operand_text(operand: &Operand, env: &Env) -> String {
    match operand {
        Operand::Literal(value) => value.to_string(),
        Operand::Variable(name) => format!("{}={}", name, env.get(name).copied().unwrap_or(0)),
    }
}

fn condition_text(condition: &Condition, env: &Env) -> String {
    format!(
        "{} {} {}",
        operand_text(&condition.left, env),
        condition.comparison.symbol(),
        operand_text(&condition.right, env)
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub index: usize,
    pub condition: Option<String>,
    pub enabled: bool,
    pub to: State,
    pub effects: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    pub machine: String,
    pub state: State,
    pub env: Env,
    pub event: Event,
    pub candidates: Vec<Candidate>,
}

impl Explanation {
    pub fn enabled_count(&self) -> usize {
        self.candidates.iter().filter(|item| item.enabled).count()
    }
}

pub fn explain(machine: &Machine, prefix: &[Event], event: &Event) -> Result<Explanation, Error> {
    let trace = replay(machine, prefix)?;
    let state = trace.final_state;
    let env = trace.final_env;
    let mut candidates = Vec::new();
    for (index, transition) in machine.transitions.iter().enumerate() {
        if &transition.from != &state || &transition.event != event {
            continue;
        }
        let condition = transition.guard.as_ref().map(|guard| condition_text(guard, &env));
        let enabled = transition
            .guard
            .as_ref()
            .map_or(true, |guard| guard.holds(&env));
        candidates.push(Candidate {
            index,
            condition,
            enabled,
            to: transition.to.clone(),
            effects: transition.effects.iter().map(format_effect).collect(),
        });
    }
    Ok(Explanation {
        machine: machine.name.clone(),
        state,
        env,
        event: event.clone(),
        candidates,
    })
}

pub fn render_explain(explanation: &Explanation) -> String {
    let mut lines = vec![
        "# sextant-explain v1".to_string(),
        format!("machine: {}", explanation.machine),
        format!("state: {}", explanation.state.as_str()),
        format!("env: {}", format_env(&explanation.env)),
        format!("event: {}", explanation.event.as_str()),
        format!("candidates: {}", explanation.candidates.len()),
    ];
    for candidate in &explanation.candidates {
        let guard = candidate.condition.as_deref().unwrap_or("always");
        lines.push(format!(
            "  - [{}] when {} -> {} ; to={} ; effects={}",
            candidate.index + 1,
            guard,
            if candidate.enabled { "enabled" } else { "disabled" },
            candidate.to.as_str(),
            if candidate.effects.is_empty() {
                "none".to_string()
            } else {
                candidate.effects.join(", ")
            }
        ));
    }
    lines.push(format!("enabled: {}", explanation.enabled_count()));
    let mut output = lines.join("\n");
    output.push('\n');
    output
}
