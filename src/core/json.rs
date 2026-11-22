//! Minimal, dependency-free JSON rendering for CI consumers.
//!
//! `Json` is a pure value. `to_string` is compact and deterministic: object
//! keys appear in the order the caller supplied them, so a result is byte-stable.

use std::fmt::Debug;

use crate::core::engine::{ExploreResult, RuleType, Violation};
use crate::core::error::Finding;
use crate::core::fuzz::FuzzResult;
use crate::core::model::{Env, Machine};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub fn number(value: usize) -> Self {
        Self::Number(value as i64)
    }

    pub fn boolean(value: bool) -> Self {
        Self::Bool(value)
    }

    pub fn array(values: Vec<Json>) -> Self {
        Self::Array(values)
    }

    pub fn object(entries: Vec<(&str, Json)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }
}

fn escape(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            other if (other as u32) < 0x20 => {
                output.push_str(&format!("\\u{:04x}", other as u32));
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

fn write_value(value: &Json, output: &mut String) {
    match value {
        Json::Null => output.push_str("null"),
        Json::Bool(flag) => output.push_str(if *flag { "true" } else { "false" }),
        Json::Number(number) => output.push_str(&number.to_string()),
        Json::String(text) => escape(text, output),
        Json::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(item, output);
            }
            output.push(']');
        }
        Json::Object(entries) => {
            output.push('{');
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                escape(key, output);
                output.push(':');
                write_value(value, output);
            }
            output.push('}');
        }
    }
}

pub fn to_string(value: &Json) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output
}

fn variant_name(value: &impl Debug) -> String {
    let text = format!("{:?}", value);
    let head = text
        .split(|character| character == '(' || character == ' ' || character == '{')
        .next()
        .unwrap_or(&text);
    head.to_string()
}

fn env_json(env: &Env) -> Json {
    Json::Object(
        env.iter()
            .map(|(name, value)| (name.clone(), Json::Number(*value)))
            .collect(),
    )
}

pub fn check_json(machine: &Machine, findings: &[Finding]) -> Json {
    Json::object(vec![
        ("machine", Json::string(&machine.name)),
        ("ok", Json::boolean(findings.is_empty())),
        (
            "counts",
            Json::object(vec![
                ("states", Json::number(machine.declared_states.len())),
                ("events", Json::number(machine.declared_events.len())),
                ("transitions", Json::number(machine.transitions.len())),
                ("invariants", Json::number(machine.invariants.len())),
                ("properties", Json::number(machine.properties.len())),
            ]),
        ),
        (
            "findings",
            Json::array(
                findings
                    .iter()
                    .map(|finding| {
                        Json::object(vec![
                            ("rule", Json::string(variant_name(finding))),
                            ("message", Json::string(finding.message())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn violation_json(violation: &Violation) -> Json {
    let kind = match violation.rule_type {
        RuleType::Invariant => "invariant",
        RuleType::Property => "property",
    };
    Json::object(vec![
        ("type", Json::string(kind)),
        ("rule", Json::string(&violation.rule)),
        ("state", Json::string(violation.state.as_str())),
        ("env", env_json(&violation.env)),
        (
            "path",
            Json::array(
                violation
                    .path
                    .iter()
                    .map(|(state, event)| Json::string(format!("{}/{}", state.as_str(), event.as_str())))
                    .collect(),
            ),
        ),
    ])
}

pub fn explore_json(result: &ExploreResult) -> Json {
    Json::object(vec![
        ("ok", Json::boolean(result.violations.is_empty())),
        ("configurations", Json::number(result.configurations)),
        ("transitions", Json::number(result.transitions_taken)),
        ("rules_checked", Json::number(result.rules_checked)),
        ("truncated", Json::boolean(result.truncated)),
        (
            "violations",
            Json::array(result.violations.iter().map(violation_json).collect()),
        ),
    ])
}

pub fn fuzz_json(result: &FuzzResult) -> Json {
    Json::object(vec![
        ("machine", Json::string(&result.machine)),
        ("ok", Json::boolean(result.failures == 0)),
        ("seeds", Json::number(result.seeds)),
        ("events_per_seed", Json::number(result.events_per_seed)),
        ("failures", Json::number(result.failures)),
        (
            "cases",
            Json::array(
                result
                    .cases
                    .iter()
                    .map(|case| {
                        Json::object(vec![
                            ("seed", Json::Number(case.seed as i64)),
                            ("reason", Json::string(&case.reason)),
                            (
                                "events",
                                Json::array(
                                    case.events
                                        .iter()
                                        .map(|event| Json::string(event.as_str()))
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}
