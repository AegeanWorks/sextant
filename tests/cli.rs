//! End-to-end CLI tests: the imperative shell at the process edge.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MACHINE: &str = include_str!("../examples/retry.sex");
const EVENTS: &str = include_str!("../examples/retry.events");

struct Fixture {
    directory: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "sextant-cli-{}-{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("temporary fixture directory");
        Self { directory }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, contents).expect("fixture file written");
        path
    }

    fn example_machine(&self) -> PathBuf {
        self.write("retry.sex", MACHINE)
    }

    fn example_events(&self) -> PathBuf {
        self.write("retry.events", EVENTS)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn run(arguments: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_sextant"))
        .args(arguments)
        .output()
        .expect("sextant binary runs");
    (
        output.status.code().unwrap_or(99),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn as_text(path: &Path) -> &str {
    path.to_str().expect("UTF-8 fixture path")
}

#[test]
fn check_accepts_a_well_formed_machine() {
    let fixture = Fixture::new("check-ok");
    let machine = fixture.example_machine();
    let (code, stdout, stderr) = run(&["check", as_text(&machine)]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(stdout.contains("well-formed"));
}

#[test]
fn check_reports_structural_findings() {
    let fixture = Fixture::new("check-bad");
    let machine = fixture.write(
        "broken.sex",
        "machine broken\ninitial idle\nstate idle\nstate orphan\nevent GO\ntransition idle GO -> orphan\n",
    );
    let (code, stdout, stderr) = run(&["check", as_text(&machine)]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains("orphan"));
}

#[test]
fn replay_output_is_byte_identical_across_runs() {
    let fixture = Fixture::new("replay");
    let machine = fixture.example_machine();
    let events = fixture.example_events();
    let first = run(&["replay", as_text(&machine), as_text(&events)]);
    let second = run(&["replay", as_text(&machine), as_text(&events)]);
    assert_eq!(first.0, 0);
    assert_eq!(first.1, second.1);
    assert!(first.1.starts_with("# sextant-trace v1\n"));
    assert!(first.1.contains("step 004: OK | waiting {tries=2} -> done {tries=2}"));
    assert!(first.1.contains("final: done {tries=2}"));
}

#[test]
fn golden_regression_detects_a_trace_change() {
    let fixture = Fixture::new("golden");
    let machine = fixture.example_machine();
    let events = fixture.example_events();
    let golden = fixture.path("retry.golden");

    let updated = run(&[
        "golden",
        as_text(&machine),
        as_text(&events),
        as_text(&golden),
        "--update",
    ]);
    assert_eq!(updated.0, 0, "stderr: {}", updated.2);

    let matching = run(&[
        "golden",
        as_text(&machine),
        as_text(&events),
        as_text(&golden),
    ]);
    assert_eq!(matching.0, 0, "stderr: {}", matching.2);

    fs::write(&golden, "changed line\n").expect("golden tampered");
    let mismatch = run(&[
        "golden",
        as_text(&machine),
        as_text(&events),
        as_text(&golden),
    ]);
    assert_eq!(mismatch.0, 1);
    assert!(mismatch.2.contains("differs at line 1"));
}

#[test]
fn explore_reports_invariant_violations() {
    let fixture = Fixture::new("explore");
    let machine = fixture.write(
        "strict.sex",
        &MACHINE.replace("tries <= 3", "tries <= 1"),
    );
    let (code, stdout, stderr) = run(&["explore", as_text(&machine), "--depth", "6"]);
    assert_eq!(code, 1);
    assert!(stdout.contains("configuration(s)"));
    assert!(stderr.contains("violation: invariant 'tries_bound'"));
}

#[test]
fn generated_scripts_are_reproducible_from_their_seed() {
    let fixture = Fixture::new("generate");
    let machine = fixture.example_machine();
    let first = run(&[
        "generate",
        as_text(&machine),
        "--seed",
        "11",
        "--count",
        "12",
    ]);
    let second = run(&[
        "generate",
        as_text(&machine),
        "--seed",
        "11",
        "--count",
        "12",
    ]);
    let different = run(&[
        "generate",
        as_text(&machine),
        "--seed",
        "12",
        "--count",
        "12",
    ]);
    assert_eq!(first.0, 0);
    assert_eq!(first.1, second.1);
    assert_ne!(first.1, different.1);
    assert_eq!(first.1.lines().count(), 12);
}

#[test]
fn inventory_replaces_the_python_starter() {
    let (code, stdout, stderr) = run(&["inventory"]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert_eq!(stdout, "cli|demo|--scripted\ncli|inventory|inventory\nhandler|run|demo-run\n");
}

#[test]
fn scripted_demo_replaces_the_python_starter() {
    let first = run(&["demo", "--scripted", "--seed", "5"]);
    let second = run(&["demo", "--scripted", "--seed", "5"]);
    assert_eq!(first.0, 0);
    assert_eq!(first.1, second.1);
    assert!(first.1.contains("WORLD-DELTA: counter=5->6"));
    assert!(first.1.contains("OK: scripted drive complete"));
}

#[test]
fn coverage_reports_missed_transitions_through_the_cli() {
    let fixture = Fixture::new("coverage");
    let machine = fixture.example_machine();
    let events = fixture.write("partial.events", include_str!("../examples/partial.events"));
    let (code, stdout, stderr) = run(&[
        "coverage",
        as_text(&machine),
        as_text(&events),
    ]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(stdout.contains("transitions: 2/4 (50%)"));
    assert!(stdout.contains("waiting --OK--> done"));
}

#[test]
fn minimize_reduces_a_failing_script_through_the_cli() {
    let fixture = Fixture::new("minimize");
    let machine = fixture.write("strict.sex", include_str!("../examples/strict.sex"));
    let events = fixture.write("failing.events", include_str!("../examples/failing.events"));
    let (code, stdout, stderr) = run(&[
        "minimize",
        as_text(&machine),
        as_text(&events),
        "--invariant",
        "tries_bound",
    ]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(stdout.contains("original: 3 event(s)"));
    assert!(stdout.contains("minimal: 3 event(s)"));
    assert!(stdout.contains("state: Minimal"));

    let passing = run(&[
        "minimize",
        as_text(&fixture.example_machine()),
        as_text(&fixture.example_events()),
    ]);
    assert_eq!(passing.0, 1);
    assert!(passing.2.contains("does not fail"));
}

#[test]
fn graph_renders_both_formats_through_the_cli() {
    let fixture = Fixture::new("graph");
    let machine = fixture.example_machine();
    let dot = run(&["graph", as_text(&machine), "--format", "dot"]);
    assert_eq!(dot.0, 0, "stderr: {}", dot.2);
    assert!(dot.1.starts_with("digraph retry {"));
    let mermaid = run(&["graph", as_text(&machine), "--format", "mermaid"]);
    assert_eq!(mermaid.0, 0, "stderr: {}", mermaid.2);
    assert!(mermaid.1.starts_with("stateDiagram-v2"));
    let bad = run(&["graph", as_text(&machine), "--format", "png"]);
    assert_eq!(bad.0, 2);
    assert!(bad.2.contains("unknown graph format"));
}

#[test]
fn explore_reports_property_violations() {
    let fixture = Fixture::new("properties");
    let machine = fixture.write("properties.sex", include_str!("../examples/properties.sex"));
    let (code, stdout, stderr) = run(&["explore", as_text(&machine), "--depth", "6"]);
    assert_eq!(code, 1);
    assert!(stdout.contains("4 rule(s)"));
    assert!(stderr.contains("property 'avoids_waiting'"));
}

#[test]
fn equiv_accepts_renamed_states_and_detects_a_tighter_invariant() {
    let fixture = Fixture::new("equiv");
    let left = fixture.example_machine();
    let renamed = fixture.write("renamed.sex", include_str!("../examples/retry_renamed.sex"));
    let strict = fixture.write("strict.sex", include_str!("../examples/strict.sex"));

    let same = run(&["equiv", as_text(&left), as_text(&renamed)]);
    assert_eq!(same.0, 0, "stderr: {}", same.2);
    assert!(same.1.contains("differences: 0"));

    let different = run(&["equiv", as_text(&left), as_text(&strict)]);
    assert_eq!(different.0, 1);
    assert!(different.1.contains("differences:"));
    assert!(different.2.contains("behavioural difference(s)"));
}

#[test]
fn usage_errors_exit_two() {
    let missing = run(&["check"]);
    assert_eq!(missing.0, 2);
    assert!(missing.2.contains("error:"));

    let unknown = run(&["teleport"]);
    assert_eq!(unknown.0, 2);
    assert!(unknown.2.contains("unknown command"));
}

#[test]
fn help_and_version_exit_zero() {
    assert_eq!(run(&["--help"]).0, 0);
    let version = run(&["--version"]);
    assert_eq!(version.0, 0);
    assert!(version.1.contains("sextant 0.1.0"));
}
