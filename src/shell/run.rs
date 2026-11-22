//! Imperative shell: orchestrates commands by driving the session state
//! machine, calling pure core functions and performing I/O only through
//! `shell::io`.

use std::path::Path;

use crate::core::engine::explore;
use crate::core::{
    canonicalize, check_json, coverage, diff_json, diff_traces, doctor_json, equivalent,
    explore_json, fuzz, fuzz_json, generate_events, json_to_string, machine_diff, minimize,
    parse_events, parse_graph_format, parse_machine, render_coverage, render_diff, render_doctor,
    render_equivalence, render_events, render_findings, render_fuzz, render_graph, render_machine,
    render_minimize, render_slice_report, render_trace, replay, self_check, slice, untrace, verify,
    Event, Machine, RuleType,
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

fn run_check(machine_path: &str, json: bool) -> u8 {
    let mut state = SessionState::Idle;
    let machine = match load_machine(machine_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if step(&mut state, SessionEvent::Verify).is_err() {
        return 2;
    }
    let findings = verify(&machine);
    if json {
        let rendered = format!("{}\n", json_to_string(&check_json(&machine, &findings)));
        io::write_stdout(&rendered);
        if findings.is_empty() {
            return finish_quiet(&mut state, "ok: check complete");
        }
        io::err(&format!("error: {} finding(s)", findings.len()));
        fail(&mut state);
        return 1;
    }
    if !findings.is_empty() {
        io::err(&render_findings(&findings));
        fail(&mut state);
        return 1;
    }
    let summary = format!(
        "ok: '{}' is well-formed ({} states, {} events, {} transitions, {} invariants, {} properties)",
        machine.name,
        machine.declared_states.len(),
        machine.declared_events.len(),
        machine.transitions.len(),
        machine.invariants.len(),
        machine.properties.len()
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

fn run_explore(machine_path: &str, depth: usize, json: bool) -> u8 {
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
    if json {
        io::write_stdout(&format!("{}\n", json_to_string(&explore_json(&result))));
        if result.violations.is_empty() {
            return finish_quiet(&mut state, "ok: exploration complete");
        }
        io::err(&format!(
            "error: {} rule violation(s)",
            result.violations.len()
        ));
        fail(&mut state);
        return 1;
    }
    io::out(&format!(
        "explored {} configuration(s), {} transition(s), {} rule(s) at depth {}",
        result.configurations, result.transitions_taken, result.rules_checked, depth
    ));
    if result.truncated {
        io::out("warning: configuration cap reached; exploration is partial");
    }
    if result.violations.is_empty() {
        return finish(&mut state, "ok: no rule violations found");
    }
    for violation in &result.violations {
        let path = violation
            .path
            .iter()
            .map(|(state, event)| format!("{}/{}", state.as_str(), event.as_str()))
            .collect::<Vec<_>>()
            .join(" -> ");
        let kind = match violation.rule_type {
            RuleType::Invariant => "invariant",
            RuleType::Property => "property",
        };
        io::err(&format!(
            "violation: {} '{}' at {} [env {:?}] path {}",
            kind,
            violation.rule,
            violation.state.as_str(),
            violation.env,
            path
        ));
    }
    fail(&mut state);
    1
}

fn run_fuzz(
    machine_path: &str,
    seeds: usize,
    count: usize,
    invariant: Option<&str>,
    out_path: Option<&str>,
    json: bool,
) -> u8 {
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
    let result = fuzz(&machine, seeds, count, invariant);
    let rendered = if json {
        format!("{}\n", json_to_string(&fuzz_json(&result)))
    } else {
        render_fuzz(&result)
    };
    if emit_data(&rendered, out_path, &mut state).is_err() {
        return 1;
    }
    if result.failures == 0 {
        finish_quiet(
            &mut state,
            &format!("ok: no failures across {} deterministic seed(s)", seeds),
        )
    } else {
        io::err(&format!("error: {} seed(s) failed", result.failures));
        fail(&mut state);
        1
    }
}

fn run_equiv(left_path: &str, right_path: &str, depth: usize, out_path: Option<&str>) -> u8 {
    let mut state = SessionState::Idle;
    let left = match load_machine(left_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if let Err(code) = verify_machine(&left, &mut state) {
        return code;
    }
    let right = match io::read_text(Path::new(right_path)) {
        Ok(text) => match parse_machine(&text) {
            Ok(machine) => machine,
            Err(error) => {
                io::err(&format!("error: {}: {}", right_path, error.message()));
                fail(&mut state);
                return 1;
            }
        },
        Err(error) => {
            io::err(&format!("error: cannot read {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    // The session is already Verified; check the second machine without a second
    // Verify event, which the lifecycle correctly rejects.
    let findings = verify(&right);
    if !findings.is_empty() {
        io::err(&render_findings(&findings));
        fail(&mut state);
        return 1;
    }
    if step(&mut state, SessionEvent::Replay).is_err() {
        return 2;
    }
    let result = equivalent(&left, &right, depth, 100_000);
    let rendered = render_equivalence(&result, &left.name, &right.name);
    if emit_data(&rendered, out_path, &mut state).is_err() {
        return 1;
    }
    if result.differences.is_empty() {
        finish_quiet(&mut state, "ok: machines are behaviourally equivalent")
    } else {
        io::err(&format!(
            "error: {} behavioural difference(s)",
            result.differences.len()
        ));
        fail(&mut state);
        1
    }
}

fn emit_data(rendered: &str, out_path: Option<&str>, state: &mut SessionState) -> Result<(), u8> {
    if let Some(path) = out_path {
        if let Err(error) = io::write_text(Path::new(path), rendered) {
            io::err(&format!("error: cannot write {}", error.message()));
            fail(state);
            return Err(1);
        }
    }
    io::write_stdout(rendered);
    Ok(())
}

fn run_coverage(machine_path: &str, events_path: &str, out_path: Option<&str>) -> u8 {
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
    let rendered = render_coverage(&machine, &coverage(&machine, &trace));
    if emit_data(&rendered, out_path, &mut state).is_err() {
        return 1;
    }
    finish_quiet(&mut state, "ok: coverage report complete")
}

fn run_minimize(
    machine_path: &str,
    events_path: &str,
    invariant: Option<&str>,
    out_path: Option<&str>,
) -> u8 {
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
    let report = match minimize(&machine, &events, invariant) {
        Ok(report) => report,
        Err(error) => {
            io::err(&format!("error: {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    let rendered = render_minimize(&report);
    if emit_data(&rendered, out_path, &mut state).is_err() {
        return 1;
    }
    finish_quiet(&mut state, "ok: minimization complete")
}

fn run_graph(machine_path: &str, format_name: &str, out_path: Option<&str>) -> u8 {
    let mut state = SessionState::Idle;
    let machine = match load_machine(machine_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    if let Err(code) = verify_machine(&machine, &mut state) {
        return code;
    }
    let format = match parse_graph_format(format_name) {
        Some(format) => format,
        None => {
            io::err(&format!("error: unknown graph format '{}'", format_name));
            fail(&mut state);
            return 2;
        }
    };
    let rendered = render_graph(&machine, format);
    if emit_data(&rendered, out_path, &mut state).is_err() {
        return 1;
    }
    finish_quiet(&mut state, "ok: graph rendered")
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

fn run_untrace(trace_path: &str, out_path: Option<&str>) -> u8 {
    let mut state = SessionState::Idle;
    if step(&mut state, SessionEvent::Load).is_err() {
        return 2;
    }
    let text = match io::read_text(Path::new(trace_path)) {
        Ok(text) => text,
        Err(error) => {
            io::err(&format!("error: cannot read {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    let events = match untrace(&text) {
        Ok(events) => events,
        Err(error) => {
            io::err(&format!("error: {}: {}", trace_path, error.message()));
            fail(&mut state);
            return 1;
        }
    };
    let rendered = render_events(&events);
    if emit_data(&rendered, out_path, &mut state).is_err() {
        return 1;
    }
    finish_quiet(
        &mut state,
        &format!("ok: recovered {} event(s) from the trace", events.len()),
    )
}

fn run_slice(machine_path: &str, events_path: &str, out_path: Option<&str>) -> u8 {
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
    let report = slice(&machine, &trace);
    let rendered = render_machine(&report.machine);
    if emit_data(&rendered, out_path, &mut state).is_err() {
        return 1;
    }
    let summary = render_slice_report(&report);
    io::err(&summary);
    finish_quiet(&mut state, "ok: sliced machine emitted")
}

fn run_fmt(machine_path: &str, out_path: Option<&str>, check: bool) -> u8 {
    let mut state = SessionState::Idle;
    if step(&mut state, SessionEvent::Load).is_err() {
        return 2;
    }
    let text = match io::read_text(Path::new(machine_path)) {
        Ok(text) => text,
        Err(error) => {
            io::err(&format!("error: cannot read {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    let canonical = match canonicalize(&text) {
        Ok(canonical) => canonical,
        Err(error) => {
            io::err(&format!("error: {}: {}", machine_path, error.message()));
            fail(&mut state);
            return 1;
        }
    };
    if step(&mut state, SessionEvent::Verify).is_err() {
        return 2;
    }
    if check {
        if canonical == text {
            finish_quiet(&mut state, "ok: machine is canonically formatted")
        } else {
            io::err(&format!("error: '{}' is not canonically formatted", machine_path));
            io::err("Why: drift in spacing and declaration order hides real changes in a model diff.");
            io::err(&format!(
                "Fix: run `sextant fmt {}` to rewrite it, or `sextant fmt {} --out F` to preview.",
                machine_path, machine_path
            ));
            fail(&mut state);
            1
        }
    } else {
        if let Some(path) = out_path {
            if let Err(error) = io::write_text(Path::new(path), &canonical) {
                io::err(&format!("error: cannot write {}", error.message()));
                fail(&mut state);
                return 1;
            }
        }
        io::write_stdout(&canonical);
        finish_quiet(&mut state, "ok: canonical form emitted")
    }
}

fn run_diff(left_path: &str, right_path: &str, json: bool) -> u8 {
    let mut state = SessionState::Idle;
    let left = match load_machine(left_path, &mut state) {
        Ok(machine) => machine,
        Err(code) => return code,
    };
    let right = match io::read_text(Path::new(right_path)) {
        Ok(text) => match parse_machine(&text) {
            Ok(machine) => machine,
            Err(error) => {
                io::err(&format!("error: {}: {}", right_path, error.message()));
                fail(&mut state);
                return 1;
            }
        },
        Err(error) => {
            io::err(&format!("error: cannot read {}", error.message()));
            fail(&mut state);
            return 1;
        }
    };
    let findings = verify(&right);
    if !findings.is_empty() {
        io::err(&render_findings(&findings));
        fail(&mut state);
        return 1;
    }
    let report = machine_diff(&left, &right);
    let rendered = if json {
        format!("{}\n", json_to_string(&diff_json(&report)))
    } else {
        render_diff(&report)
    };
    io::write_stdout(&rendered);
    finish_quiet(
        &mut state,
        &format!("ok: {} change(s) between machines", report.total_changes()),
    )
}

fn run_doctor(json: bool) -> u8 {
    let mut state = SessionState::Idle;
    if step(&mut state, SessionEvent::Finish).is_err() {
        return 2;
    }
    let results = self_check();
    let rendered = if json {
        format!("{}\n", json_to_string(&doctor_json(&results)))
    } else {
        render_doctor(&results)
    };
    io::write_stdout(&rendered);
    if results.iter().all(|item| item.ok) {
        0
    } else {
        io::err("error: doctor found a failing check");
        1
    }
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
        Command::Doctor { json } => run_doctor(json),
        Command::Fmt {
            machine,
            out,
            check,
        } => run_fmt(&machine, out.as_deref(), check),
        Command::Diff { left, right, json } => run_diff(&left, &right, json),
        Command::Untrace { trace, out } => run_untrace(&trace, out.as_deref()),
        Command::Slice { machine, events, out } => run_slice(&machine, &events, out.as_deref()),
        Command::Check { machine, json } => run_check(&machine, json),
        Command::Replay { machine, events, out } => run_replay(&machine, &events, out.as_deref()),
        Command::Golden {
            machine,
            events,
            golden,
            update,
        } => run_golden(&machine, &events, &golden, update),
        Command::Explore { machine, depth, json } => run_explore(&machine, depth, json),
        Command::Equiv {
            left,
            right,
            depth,
            out,
        } => run_equiv(&left, &right, depth, out.as_deref()),
        Command::Fuzz {
            machine,
            seeds,
            count,
            invariant,
            out,
            json,
        } => run_fuzz(
            &machine,
            seeds,
            count,
            invariant.as_deref(),
            out.as_deref(),
            json,
        ),
        Command::Coverage { machine, events, out } => {
            run_coverage(&machine, &events, out.as_deref())
        }
        Command::Minimize {
            machine,
            events,
            invariant,
            out,
        } => run_minimize(&machine, &events, invariant.as_deref(), out.as_deref()),
        Command::Graph { machine, format, out } => run_graph(&machine, &format, out.as_deref()),
        Command::Generate {
            machine,
            seed,
            count,
            out,
        } => run_generate(&machine, seed, count, out.as_deref()),
        Command::Demo { seed } => run_demo(seed),
    }
}
