//! Imperative shell: orchestrates commands by driving the session state
//! machine, calling pure core functions and performing I/O only through
//! `shell::io`.

use std::path::Path;

use crate::core::engine::explore;
use crate::core::{
    diff_traces, generate_events, parse_events, parse_machine, replay, render_findings,
    render_trace, verify, Event, Machine,
};

use super::cli::Command;
use super::io;
use super::session::{transition, SessionEvent, SessionState};

const MAX_EXPLORE_CONFIGURATIONS: usize = 200_000;

fn step(state: &mut SessionState, event: SessionEvent) -> Result<(), u8> {
    match transition(*state, event) {
        Ok(next) => {
            *state = next;
            Ok(())
        }
        Err(problem) => {
            io::err(&format!("internal state error: {}", problem));
            Err(2)
        }
    }
}

fn fail(state: &mut SessionState) {
    let _ = step(state, SessionEvent::Fail);
}

fn load_machine(path: &str, state: &mut SessionState) -> Result<Machine, u8> {
    step(state, SessionEvent::Load)?;
    let text = match io::read_text(Path::new(path)) {
        Ok(text) => text,
        Err(error) => {
            io::err(&format!("error: cannot read {}", error.message()));
            fail(state);
            return Err(1);
        }
    };
    match parse_machine(&text) {
        Ok(machine) => Ok(machine),
        Err(error) => {
            io::err(&format!("error: {}: {}", path, error.message()));
            fail(state);
            Err(1)
        }
    }
}

fn load_events(path: &str) -> Result<Vec<Event>, u8> {
    let text = match io::read_text(Path::new(path)) {
        Ok(text) => text,
        Err(error) => {
            io::err(&format!("error: cannot read {}", error.message()));
            return Err(1);
        }
    };
    match parse_events(&text) {
        Ok(events) => Ok(events),
        Err(error) => {
            io::err(&format!("error: {}: {}", path, error.message()));
            Err(1)
        }
    }
}

fn verify_machine(machine: &Machine, state: &mut SessionState) -> Result<(), u8> {
    step(state, SessionEvent::Verify)?;
    let findings = verify(machine);
    if findings.is_empty() {
        return Ok(());
    }
    io::err(&render_findings(&findings));
    fail(state);
    Err(1)
}

fn finish(state: &mut SessionState, message: &str) -> u8 {
    if step(state, SessionEvent::Finish).is_err() {
        return 2;
    }
    io::out(message);
    0
}

/// Complete a command that emitted machine-readable data on stdout: the human
/// status goes to stderr so the stdout stream stays pipe-safe.
fn finish_quiet(state: &mut SessionState, message: &str) -> u8 {
    if step(state, SessionEvent::Finish).is_err() {
        return 2;
    }
    io::err(message);
    0
}

fn run_check(machine_path: &str) -> u8 {
    let mut state = SessionState::Idle;
    let machine = match load_machine(machine_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if let Err(code) = verify_machine(&machine, &mut state) {
        return code;
    }
    let summary = format!(
        "ok: '{}' is well-formed ({} states, {} events, {} transitions, {} invariants)",
        machine.name,
        machine.declared_states.len(),
        machine.declared_events.len(),
        machine.transitions.len(),
        machine.invariants.len()
    );
    finish(&mut state, &summary)
}

fn run_replay(machine_path: &str, events_path: &str, out_path: Option<&str>) -> u8 {
    let mut state = SessionState::Idle;
    let machine = match load_machine(machine_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if let Err(code) = verify_machine(&machine, &mut state) {
        return code;
    }
    if step(&mut state, SessionEvent::Replay).is_err() {
        return 2;
    }
    let events = match load_events(events_path) {
        Ok(events) => events,
        Err(code) => {
            fail(&mut state);
            return code;
        }
    };
    let trace = match replay(&machine, &events) {
        Ok(trace) => trace,
        Err(error) => {
            io::err(&format!("error: {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    let rendered = render_trace(&trace);
    if let Some(path) = out_path {
        if let Err(error) = io::write_text(Path::new(path), &rendered) {
            io::err(&format!("error: cannot write {}", error.message()));
            fail(&mut state);
            return 1;
        }
    }
    io::write_stdout(&rendered);
    finish_quiet(&mut state, "ok: replay complete")
}

fn run_golden(machine_path: &str, events_path: &str, golden_path: &str, update: bool) -> u8 {
    let mut state = SessionState::Idle;
    let machine = match load_machine(machine_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if let Err(code) = verify_machine(&machine, &mut state) {
        return code;
    }
    if step(&mut state, SessionEvent::Replay).is_err() {
        return 2;
    }
    let events = match load_events(events_path) {
        Ok(events) => events,
        Err(code) => {
            fail(&mut state);
            return code;
        }
    };
    let trace = match replay(&machine, &events) {
        Ok(trace) => trace,
        Err(error) => {
            io::err(&format!("error: {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    let actual = render_trace(&trace);
    if step(&mut state, SessionEvent::Compare).is_err() {
        return 2;
    }

    if update {
        if let Err(error) = io::write_text(Path::new(golden_path), &actual) {
            io::err(&format!("error: cannot write {}", error.message()));
            fail(&mut state);
            return 1;
        }
        return finish(&mut state, &format!("ok: golden trace updated at '{}'", golden_path));
    }

    let expected = match io::read_text(Path::new(golden_path)) {
        Ok(text) => text,
        Err(error) => {
            io::err(&format!("error: cannot read {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    match diff_traces(&expected, &actual) {
        None => finish(&mut state, "ok: golden trace matches"),
        Some(difference) => {
            io::err(&format!("error: golden trace differs at line {}", difference.line));
            io::err(&format!(
                "  expected: {}",
                difference.expected.unwrap_or_else(|| "<end of file>".to_string())
            ));
            io::err(&format!(
                "  actual:   {}",
                difference.actual.unwrap_or_else(|| "<end of file>".to_string())
            ));
            fail(&mut state);
            1
        }
    }
}

fn run_explore(machine_path: &str, depth: usize) -> u8 {
    let mut state = SessionState::Idle;
    let machine = match load_machine(machine_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if let Err(code) = verify_machine(&machine, &mut state) {
        return code;
    }
    if step(&mut state, SessionEvent::Replay).is_err() {
        return 2;
    }
    let result = explore(&machine, depth, MAX_EXPLORE_CONFIGURATIONS);
    io::out(&format!(
        "explored {} configuration(s), {} transition(s) at depth {}",
        result.configurations, result.transitions_taken, depth
    ));
    if result.truncated {
        io::out("warning: configuration cap reached; exploration is partial");
    }
    if result.violations.is_empty() {
        return finish(&mut state, "ok: no invariant violations found");
    }
    for violation in &result.violations {
        let path = violation
            .path
            .iter()
            .map(|(state, event)| format!("{}/{}", state.as_str(), event.as_str()))
            .collect::<Vec<_>>()
            .join(" -> ");
        io::err(&format!(
            "violation: '{}' at {} [env {:?}] path {}",
            violation.invariant,
            violation.state.as_str(),
            violation.env,
            path
        ));
    }
    fail(&mut state);
    1
}

fn run_generate(machine_path: &str, seed: u64, count: usize, out_path: Option<&str>) -> u8 {
    let mut state = SessionState::Idle;
    let machine = match load_machine(machine_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if let Err(code) = verify_machine(&machine, &mut state) {
        return code;
    }
    let events = generate_events(&machine, seed, count);
    let mut rendered = String::new();
    for event in &events {
        rendered.push_str(event.as_str());
        rendered.push('\n');
    }
    if let Some(path) = out_path {
        if let Err(error) = io::write_text(Path::new(path), &rendered) {
            io::err(&format!("error: cannot write {}", error.message()));
            fail(&mut state);
            return 1;
        }
    }
    io::write_stdout(&rendered);
    finish_quiet(
        &mut state,
        &format!("ok: generated {} event(s) from seed {}", events.len(), seed),
    )
}

fn run_inventory() -> u8 {
    let mut state = SessionState::Idle;
    if step(&mut state, SessionEvent::Finish).is_err() {
        return 2;
    }
    for line in [
        "cli|demo|--scripted",
        "cli|inventory|inventory",
        "handler|run|demo-run",
    ] {
        io::out(line);
    }
    0
}

fn run_demo(seed: u64) -> u8 {
    let mut state = SessionState::Idle;
    if step(&mut state, SessionEvent::Finish).is_err() {
        return 2;
    }
    io::out(&format!("SEED: {}", seed));
    io::out(&format!("WORLD-DELTA: counter={}->{}", seed, seed.wrapping_add(1)));
    io::out(&format!(
        "WORLD-DELTA: items={}->{}",
        seed.wrapping_add(2),
        seed.wrapping_add(3)
    ));
    io::out("OK: scripted drive complete");
    0
}

pub fn run(command: Command) -> u8 {
    match command {
        Command::Help => {
            io::out(&super::cli::usage_text());
            0
        }
        Command::Version => {
            io::out("sextant 0.1.0");
            0
        }
        Command::Inventory => run_inventory(),
        Command::Check { machine } => run_check(&machine),
        Command::Replay { machine, events, out } => run_replay(&machine, &events, out.as_deref()),
        Command::Golden {
            machine,
            events,
            golden,
            update,
        } => run_golden(&machine, &events, &golden, update),
        Command::Explore { machine, depth } => run_explore(&machine, depth),
        Command::Generate {
            machine,
            seed,
            count,
            out,
        } => run_generate(&machine, seed, count, out.as_deref()),
        Command::Demo { seed } => run_demo(seed),
    }
}
