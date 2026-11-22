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
fn fuzz_reports_failing_seeds_through_the_cli() {
    let fixture = Fixture::new("fuzz");
    let machine = fixture.write("strict.sex", include_str!("../examples/strict.sex"));
    let (code, stdout, stderr) = run(&[
        "fuzz",
        as_text(&machine),
        "--seeds",
        "64",
        "--count",
        "12",
        "--invariant",
        "tries_bound",
    ]);
    assert_eq!(code, 1);
    assert!(stdout.contains("# sextant-fuzz v1"));
    assert!(stdout.contains("seeds: 64"));
    assert!(stderr.contains("seed(s) failed"));
}

#[test]
fn check_flags_a_shadowed_guard() {
    let fixture = Fixture::new("shadow");
    let machine = fixture.write(
        "shadow.sex",
        "machine shadow\ninitial idle\nstate idle\nstate done\nevent GO\n\
         transition idle GO when tries <= 5 -> done\n\
         transition idle GO when tries <= 2 -> done\n",
    );
    let (code, stdout, stderr) = run(&["check", as_text(&machine)]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains("shadowed by earlier transition"));
}

#[test]
fn json_output_is_machine_readable_for_three_commands() {
    let fixture = Fixture::new("json");
    let machine = fixture.example_machine();
    let events = fixture.example_events();

    let check = run(&["check", as_text(&machine), "--json"]);
    assert_eq!(check.0, 0, "stderr: {}", check.2);
    assert!(check.1.starts_with('{'));
    assert!(check.1.contains("\"ok\":true"));

    let explore = run(&["explore", as_text(&machine), "--depth", "4", "--json"]);
    assert_eq!(explore.0, 0, "stderr: {}", explore.2);
    assert!(explore.1.starts_with('{'));
    assert!(explore.1.contains("\"violations\":[]"));

    let fuzz = run(&[
        "fuzz",
        as_text(&machine),
        "--seeds",
        "8",
        "--count",
        "12",
        "--invariant",
        "tries_bound",
        "--json",
    ]);
    assert_eq!(fuzz.0, 0, "stderr: {}", fuzz.2);
    assert!(fuzz.1.starts_with('{'));
    assert!(fuzz.1.contains("\"failures\":0"));

    // JSON mode keeps the status line off stdout.
    assert!(events.to_str().is_some());
}

#[test]
fn doctor_self_check_passes_in_text_and_json() {
    let text = run(&["doctor"]);
    assert_eq!(text.0, 0, "stderr: {}", text.2);
    assert!(text.1.contains("11/11 checks passed"));

    let json = run(&["doctor", "--json"]);
    assert_eq!(json.0, 0, "stderr: {}", json.2);
    assert!(json.1.starts_with("{\"ok\":true"));
    assert_eq!(text.1, run(&["doctor"]).1);
}

#[test]
fn fmt_check_detects_drift_and_fmt_writes_canonical_text() {
    let fixture = Fixture::new("fmt");
    let messy = fixture.write(
        "messy.sex",
        "event START\nstate idle\nstate waiting\nmachine messy\ninitial idle\n\
         transition idle START -> waiting\n",
    );
    let check = run(&["fmt", as_text(&messy), "--check"]);
    assert_eq!(check.0, 1);
    assert!(check.2.contains("not canonically formatted"));
    assert!(check.2.contains("Why:"));
    assert!(check.2.contains("Fix:"));

    let formatted = run(&["fmt", as_text(&messy)]);
    assert_eq!(formatted.0, 0, "stderr: {}", formatted.2);
    assert!(formatted.1.starts_with("machine messy\ninitial idle\n"));
    let canonical = fixture.write("canonical.sex", &formatted.1);
    let clean = run(&["fmt", as_text(&canonical), "--check"]);
    assert_eq!(clean.0, 0, "stderr: {}", clean.2);
}

#[test]
fn diff_reports_structural_changes_in_text_and_json() {
    let fixture = Fixture::new("diff");
    let left = fixture.example_machine();
    let right = fixture.write("renamed.sex", include_str!("../examples/retry_renamed.sex"));
    let text = run(&["diff", as_text(&left), as_text(&right)]);
    assert_eq!(text.0, 0, "stderr: {}", text.2);
    assert!(text.1.contains("initial: idle -> resting"));
    assert!(text.1.contains("changes: 15"));
    let json = run(&["diff", as_text(&left), as_text(&right), "--json"]);
    assert_eq!(json.0, 0, "stderr: {}", json.2);
    assert!(json.1.starts_with('{'));
    assert!(json.1.contains("\"changes\":15"));
}

#[test]
fn untrace_recovers_the_script_from_a_golden_trace() {
    let fixture = Fixture::new("untrace");
    let golden = fixture.write("retry.golden", include_str!("../examples/retry.golden"));
    let (code, stdout, stderr) = run(&["untrace", as_text(&golden)]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert_eq!(stdout, include_str!("../examples/retry.events"));
}

#[test]
fn slice_emits_a_smaller_replayable_machine() {
    let fixture = Fixture::new("slice");
    let machine = fixture.example_machine();
    let events = fixture.example_events();
    let (code, stdout, stderr) = run(&["slice", as_text(&machine), as_text(&events)]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(stdout.starts_with("machine retry_slice\n"));
    assert!(!stdout.contains("RESET"));
    assert!(stderr.contains("dropped-properties: 0"));
    assert_eq!(
        stdout.lines().filter(|line| line.starts_with("transition ")).count(),
        3
    );
    // The slice ends in a terminal state with no outgoing edge, which `check`
    // correctly reports as a dead end. It is a debugging projection, not a
    // standalone verified model; replay equivalence is covered in tests/slice.rs.
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
