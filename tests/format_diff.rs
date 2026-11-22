//! Tests for canonical formatting and structural machine diff.

use sextant::core::{
    canonicalize, diff_json, is_canonical, json_to_string, machine_diff, parse_machine,
    render_diff,
};

const RETRY: &str = include_str!("../examples/retry.sex");
const RENAMED: &str = include_str!("../examples/retry_renamed.sex");

#[test]
fn canonical_form_is_sorted_and_idempotent() {
    let messy = "# comment\n\
                 event START\nevent TICK\n\
                 state waiting\nstate idle\n\
                 machine messy\n\
                 initial idle\n\
                 transition idle START -> waiting\n\
                 transition waiting TICK when tries <= 2 do tries += 1 -> waiting\n";
    let first = canonicalize(messy).unwrap();
    let second = canonicalize(&first).unwrap();
    assert_eq!(first, second);
    assert!(first.starts_with("machine messy\ninitial idle\n"));
    assert!(first.contains("state idle\nstate waiting\n"));
    assert!(first.contains("event START\nevent TICK\n"));
    assert!(first.lines().all(|line| !line.starts_with('#')));
    assert!(is_canonical(&first).unwrap());
    assert!(!is_canonical(messy).unwrap());
}

#[test]
fn semantic_change_survives_reordering() {
    let a = canonicalize(RETRY).unwrap();
    let b = canonicalize(&RETRY.replace("event START\nevent TICK", "event TICK\nevent START"))
        .unwrap();
    assert_eq!(a, b, "declaration order must not change the canonical form");
}

#[test]
fn diff_names_added_and_removed_elements() {
    let left = parse_machine(RETRY).unwrap();
    let right = parse_machine(RENAMED).unwrap();
    let report = machine_diff(&left, &right);
    assert_eq!(report.states_added, vec!["finished", "pending", "resting"]);
    assert_eq!(report.states_removed, vec!["done", "idle", "waiting"]);
    assert_eq!(report.initial_changed, Some(("idle".to_string(), "resting".to_string())));
    assert!(report.transitions_added.len() == 4);
    assert!(report.transitions_removed.len() == 4);
    assert_eq!(report.total_changes(), 3 + 3 + 1 + 4 + 4);
    let rendered = render_diff(&report);
    assert!(rendered.starts_with("# sextant-diff v1\n"));
    assert!(rendered.contains("initial: idle -> resting"));
    assert!(rendered.contains("  + resting"));
    assert_eq!(rendered, render_diff(&machine_diff(&left, &right)));
}

#[test]
fn identical_machines_have_no_changes() {
    let machine = parse_machine(RETRY).unwrap();
    let report = machine_diff(&machine, &machine);
    assert_eq!(report.total_changes(), 0);
    assert!(render_diff(&report).contains("changes: 0"));
    let json = json_to_string(&diff_json(&report));
    assert!(json.contains("\"changes\":0"));
    assert!(json.contains("\"initial\":null"));
}
