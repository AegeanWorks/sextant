//! Pure built-in self-check. No files, clock or process state are consulted.

use crate::core::coverage::coverage;
use crate::core::engine::{explore, generate_events, replay};
use crate::core::equivalence::equivalent;
use crate::core::json::{to_string as json_to_string, Json};
use crate::core::minimize::{minimizer_transition, MinimizerEvent, MinimizerState};
use crate::core::parse::{parse_events, parse_machine};

const MODEL: &str = "machine doctor\n\
initial idle\n\
state idle\n\
state waiting\n\
state done\n\
event START\n\
event TICK\n\
event OK\n\
event RESET\n\
transition idle START -> waiting\n\
transition waiting TICK when tries <= 1 do tries += 1 -> waiting\n\
transition waiting OK -> done\n\
transition done RESET do tries = 0 -> idle\n\
invariant tries_bound: tries <= 2\n";

const SCRIPT: &str = "START\nTICK\nOK\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckResult {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

fn result(name: &str, ok: bool, detail: impl Into<String>) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        ok,
        detail: detail.into(),
    }
}

/// Run every core engine against the embedded fixture and return named results.
pub fn self_check() -> Vec<CheckResult> {
    let machine = match parse_machine(MODEL) {
        Ok(machine) => machine,
        Err(error) => {
            return vec![result("model parse", false, error.message())];
        }
    };
    let mut results = vec![result(
        "model parse",
        true,
        format!(
            "{} states, {} transitions",
            machine.declared_states.len(),
            machine.transitions.len()
        ),
    )];

    let findings = crate::core::engine::verify(&machine);
    results.push(result(
        "structure",
        findings.is_empty(),
        format!("{} finding(s)", findings.len()),
    ));

    let events = match parse_events(SCRIPT) {
        Ok(events) => events,
        Err(error) => {
            results.push(result("event parse", false, error.message()));
            return results;
        }
    };
    results.push(result(
        "event parse",
        true,
        format!("{} event(s)", events.len()),
    ));

    let first = replay(&machine, &events);
    let second = replay(&machine, &events);
    results.push(result(
        "deterministic replay",
        first == second && first.is_ok(),
        format!("final: {:?}", first.as_ref().map(|trace| trace.final_state.clone())),
    ));

    if let Ok(trace) = &first {
        let rendered = crate::core::format::render_trace(trace);
        results.push(result(
            "golden render",
            rendered.contains("final: done {tries=1}"),
            "versioned trace rendered",
        ));
        let report = coverage(&machine, trace);
        results.push(result(
            "coverage",
            report.transition_percent() == 75,
            format!("{}% of transitions", report.transition_percent()),
        ));
    }

    let exploration = explore(&machine, 4, 1_000);
    results.push(result(
        "explore",
        exploration.violations.is_empty(),
        format!("{} configuration(s)", exploration.configurations),
    ));

    let generated = generate_events(&machine, 7, 10);
    results.push(result(
        "seeded generation",
        generated == generate_events(&machine, 7, 10),
        format!("{} event(s)", generated.len()),
    ));

    let table_ok = minimizer_transition(MinimizerState::Trail, MinimizerEvent::FailureConfirmed)
        == Ok(MinimizerState::Shrinking)
        && minimizer_transition(MinimizerState::Minimal, MinimizerEvent::CandidateFails).is_err();
    results.push(result(
        "minimizer table",
        table_ok,
        "closed transitions confirmed",
    ));

    let json = json_to_string(&Json::object(vec![("ok", Json::boolean(true))]));
    results.push(result(
        "json serializer",
        json == "{\"ok\":true}",
        json,
    ));

    let comparison = equivalent(&machine, &machine, 2, 1_000);
    results.push(result(
        "equivalence",
        comparison.differences.is_empty(),
        format!("{} sequence(s)", comparison.sequences),
    ));

    results
}

pub fn render_doctor(results: &[CheckResult]) -> String {
    let passed = results.iter().filter(|item| item.ok).count();
    let mut lines = vec!["# sextant-doctor v1".to_string()];
    for item in results {
        lines.push(format!(
            "{} {}: {}",
            if item.ok { "ok" } else { "FAIL" },
            item.name,
            item.detail
        ));
    }
    lines.push(format!("{}/{} checks passed", passed, results.len()));
    let mut output = lines.join("\n");
    output.push('\n');
    output
}

pub fn doctor_json(results: &[CheckResult]) -> Json {
    let passed = results.iter().filter(|item| item.ok).count();
    Json::object(vec![
        ("ok", Json::boolean(passed == results.len())),
        ("passed", Json::number(passed)),
        ("total", Json::number(results.len())),
        (
            "checks",
            Json::array(
                results
                    .iter()
                    .map(|item| {
                        Json::object(vec![
                            ("name", Json::string(&item.name)),
                            ("ok", Json::boolean(item.ok)),
                            ("detail", Json::string(&item.detail)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}
