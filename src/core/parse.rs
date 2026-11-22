//! Pure parsers for the machine definition and event scripts.
//!
//! The grammar is line oriented so it can be reviewed in a diff:
//!
//! ```text
//! machine retry
//! initial idle
//! state idle
//! state waiting
//! event START
//! transition idle START -> waiting
//! transition waiting TICK when tries <= 2 do tries += 1 -> waiting
//! invariant tries_bound: tries <= 3
//! ```

use crate::core::error::Error;
use crate::core::model::{
    Comparison, Condition, Effect, Event, Invariant, Machine, Operand, State, Transition,
};

fn parse_operand(token: &str) -> Operand {
    match token.parse::<i64>() {
        Ok(value) => Operand::Literal(value),
        Err(_) => Operand::Variable(token.to_string()),
    }
}

fn parse_condition(parts: &[&str], line: usize) -> Result<Condition, Error> {
    if parts.len() != 3 {
        return Err(Error::Parse {
            line,
            message: format!(
                "a condition needs 3 tokens like 'tries <= 3', got {}",
                parts.len()
            ),
        });
    }
    let comparison = Comparison::parse(parts[1]).ok_or_else(|| Error::Parse {
        line,
        message: format!("unknown comparison '{}' (use < <= > >= == !=)", parts[1]),
    })?;
    Ok(Condition {
        left: parse_operand(parts[0]),
        comparison,
        right: parse_operand(parts[2]),
    })
}

fn parse_effects(spec: &str, line: usize) -> Result<Vec<Effect>, Error> {
    let mut effects = Vec::new();
    for piece in spec.split(',') {
        let piece = piece.trim();
        if piece.is_empty() {
            return Err(Error::Parse {
                line,
                message: "empty effect in 'do' clause".to_string(),
            });
        }
        let (name, op, value) = if let Some((name, value)) = piece.split_once("+=") {
            (name, '+', value)
        } else if let Some((name, value)) = piece.split_once("-=") {
            (name, '-', value)
        } else if let Some((name, value)) = piece.split_once('=') {
            (name, '=', value)
        } else {
            return Err(Error::Parse {
                line,
                message: format!("effect '{}' must use =, += or -=", piece),
            });
        };
        let name = name.trim();
        let value = value.trim().parse::<i64>().map_err(|_| Error::Parse {
            line,
            message: format!("effect value '{}' is not an integer", value),
        })?;
        if name.is_empty() {
            return Err(Error::Parse {
                line,
                message: "effect has an empty variable name".to_string(),
            });
        }
        effects.push(match op {
            '+' => Effect::Add(name.to_string(), value),
            '-' => Effect::Subtract(name.to_string(), value),
            _ => Effect::Set(name.to_string(), value),
        });
    }
    Ok(effects)
}

/// Strip comments and trailing whitespace. `#` starts a comment.
fn content(line: &str) -> &str {
    let without_comment = match line.find('#') {
        Some(index) => &line[..index],
        None => line,
    };
    without_comment.trim()
}

pub fn parse_machine(input: &str) -> Result<Machine, Error> {
    let mut name: Option<String> = None;
    let mut initial: Option<State> = None;
    let mut declared_states: Vec<State> = Vec::new();
    let mut declared_events: Vec<Event> = Vec::new();
    let mut transitions: Vec<Transition> = Vec::new();
    let mut invariants: Vec<Invariant> = Vec::new();

    for (offset, raw) in input.lines().enumerate() {
        let line_no = offset + 1;
        let text = content(raw);
        if text.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = text.split_whitespace().collect();
        match tokens[0] {
            "machine" => {
                if tokens.len() != 2 {
                    return Err(Error::Parse {
                        line: line_no,
                        message: "'machine' needs exactly one name".to_string(),
                    });
                }
                if name.is_some() {
                    return Err(Error::Duplicate {
                        what: "machine name",
                        name: tokens[1].to_string(),
                    });
                }
                name = Some(tokens[1].to_string());
            }
            "initial" => {
                if tokens.len() != 2 {
                    return Err(Error::Parse {
                        line: line_no,
                        message: "'initial' needs exactly one state".to_string(),
                    });
                }
                if initial.is_some() {
                    return Err(Error::Duplicate {
                        what: "initial state",
                        name: tokens[1].to_string(),
                    });
                }
                initial = Some(State(tokens[1].to_string()));
            }
            "state" => {
                if tokens.len() != 2 {
                    return Err(Error::Parse {
                        line: line_no,
                        message: "'state' needs exactly one name".to_string(),
                    });
                }
                let state = State(tokens[1].to_string());
                if declared_states.contains(&state) {
                    return Err(Error::Duplicate {
                        what: "state",
                        name: state.0,
                    });
                }
                declared_states.push(state);
            }
            "event" => {
                if tokens.len() != 2 {
                    return Err(Error::Parse {
                        line: line_no,
                        message: "'event' needs exactly one name".to_string(),
                    });
                }
                let event = Event(tokens[1].to_string());
                if declared_events.contains(&event) {
                    return Err(Error::Duplicate {
                        what: "event",
                        name: event.0,
                    });
                }
                declared_events.push(event);
            }
            "transition" => {
                let arrow = tokens.iter().position(|token| *token == "->").ok_or_else(|| {
                    Error::Parse {
                        line: line_no,
                        message: "a transition needs '-> <to>'".to_string(),
                    }
                })?;
                if tokens.len() < 5 || arrow + 2 != tokens.len() {
                    return Err(Error::Parse {
                        line: line_no,
                        message: "transition syntax is 'transition <from> <event> [when <cond>] [do <effects>] -> <to>'".to_string(),
                    });
                }
                let mut cursor = 3usize;
                let mut guard: Option<Condition> = None;
                let mut effects: Vec<Effect> = Vec::new();
                while cursor < arrow {
                    match tokens[cursor] {
                        "when" => {
                            if cursor + 4 > arrow {
                                return Err(Error::Parse {
                                    line: line_no,
                                    message: "'when' needs a 3-token condition".to_string(),
                                });
                            }
                            guard = Some(parse_condition(&tokens[cursor + 1..cursor + 4], line_no)?);
                            cursor += 4;
                        }
                        "do" => {
                            if cursor + 2 > arrow {
                                return Err(Error::Parse {
                                    line: line_no,
                                    message: "'do' needs at least one effect".to_string(),
                                });
                            }
                            let spec = tokens[cursor + 1..arrow].join(" ");
                            effects = parse_effects(&spec, line_no)?;
                            cursor = arrow;
                        }
                        other => {
                            return Err(Error::Parse {
                                line: line_no,
                                message: format!("unexpected token '{}' in transition", other),
                            });
                        }
                    }
                }
                transitions.push(Transition {
                    from: State(tokens[1].to_string()),
                    event: Event(tokens[2].to_string()),
                    guard,
                    to: State(tokens[arrow + 1].to_string()),
                    effects,
                });
            }
            "invariant" => {
                let colon = tokens
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, token)| token.ends_with(':'));
                let (position, token) = colon.ok_or_else(|| Error::Parse {
                    line: line_no,
                    message: "an invariant needs a name followed by ':'".to_string(),
                })?;
                let invariant_name = token.trim_end_matches(':').to_string();
                let condition = parse_condition(&tokens[position + 1..], line_no)?;
                if invariants.iter().any(|item| item.name == invariant_name) {
                    return Err(Error::Duplicate {
                        what: "invariant",
                        name: invariant_name,
                    });
                }
                invariants.push(Invariant {
                    name: invariant_name,
                    condition,
                });
            }
            other => {
                return Err(Error::Parse {
                    line: line_no,
                    message: format!("unknown directive '{}'", other),
                });
            }
        }
    }

    let machine = Machine {
        name: name.ok_or(Error::MissingMachineName)?,
        initial: initial.ok_or(Error::MissingInitial)?,
        declared_states,
        declared_events,
        transitions,
        invariants,
    };
    Ok(machine)
}

/// An event script holds one declared event name per line.
pub fn parse_events(input: &str) -> Result<Vec<Event>, Error> {
    let mut events = Vec::new();
    for (offset, raw) in input.lines().enumerate() {
        let line_no = offset + 1;
        let text = content(raw);
        if text.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.len() != 1 {
            return Err(Error::Parse {
                line: line_no,
                message: "an event script holds one event name per line".to_string(),
            });
        }
        events.push(Event(tokens[0].to_string()));
    }
    Ok(events)
}
