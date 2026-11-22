//! Pure recovery of an event script from a versioned trace.

use crate::core::error::Error;
use crate::core::model::Event;

const HEADER: &str = "# sextant-trace v1";

pub fn untrace(input: &str) -> Result<Vec<Event>, Error> {
    let mut lines = input.lines().enumerate();
    match lines.next() {
        Some((_, header)) if header.trim_end() == HEADER => {}
        Some((_, other)) => {
            return Err(Error::Parse {
                line: 1,
                message: format!("expected trace header '{}', got '{}'", HEADER, other),
            });
        }
        None => {
            return Err(Error::Parse {
                line: 1,
                message: "empty trace".to_string(),
            });
        }
    }

    let mut events = Vec::new();
    for (offset, raw) in lines {
        let line_no = offset + 1;
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with("machine:") || line.starts_with("initial:")
            || line.starts_with("final:")
        {
            continue;
        }
        if !line.starts_with("step ") {
            return Err(Error::Parse {
                line: line_no,
                message: format!("unexpected trace line '{}'", line),
            });
        }
        let after_step = &line["step ".len()..];
        let colon = after_step.find(": ").ok_or_else(|| Error::Parse {
            line: line_no,
            message: "a step needs ': ' after its number".to_string(),
        })?;
        let index = &after_step[..colon];
        if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Error::Parse {
                line: line_no,
                message: format!("invalid step number '{}'", index),
            });
        }
        let rest = &after_step[colon + 2..];
        let separator = rest.find(" | ").ok_or_else(|| Error::Parse {
            line: line_no,
            message: "a step needs ' | ' between the event and the state".to_string(),
        })?;
        let event = &rest[..separator];
        if event.is_empty() {
            return Err(Error::Parse {
                line: line_no,
                message: "empty event name in step".to_string(),
            });
        }
        events.push(Event(event.to_string()));
    }
    Ok(events)
}

pub fn render_events(events: &[Event]) -> String {
    let mut output = String::new();
    for event in events {
        output.push_str(event.as_str());
        output.push('\n');
    }
    output
}
