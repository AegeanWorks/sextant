//! Tests for seed fuzzing and conservative guard-shadow detection.

use sextant::core::{fuzz, parse_machine, render_fuzz, verify, Finding};

const RETRY: &str = include_str!("../examples/retry.sex");
const STRICT: &str = include_str!("../examples/strict.sex");

#[test]
fn a_broader_guard_declared_first_shadows_a_narrower_one() {
    let text = "machine shadow\ninitial idle\nstate idle\nstate done\nevent GO\n\
                transition idle GO when tries <= 5 -> done\n\
                transition idle GO when tries <= 2 -> done\n";
    let machine = parse_machine(text).unwrap();
    let findings = verify(&machine);
    assert!(findings.iter().any(|finding| matches!(
        finding,
        Finding::ShadowedTransition {
            shadowed: 1,
            by: 0,
            ..
        }
    )));
}

#[test]
fn reversed_comparisons_are_normalized_before_dominance() {
    let text = "machine shadow\ninitial idle\nstate idle\nstate done\nevent GO\n\
                transition idle GO when 5 >= tries -> done\n\
                transition idle GO when 2 >= tries -> done\n";
    let machine = parse_machine(text).unwrap();
    assert!(verify(&machine)
        .iter()
        .any(|finding| matches!(finding, Finding::ShadowedTransition { .. })));
}

#[test]
fn disjoint_guards_and_non_interval_guards_are_not_reported() {
    let disjoint = "machine fine\ninitial idle\nstate idle\nstate done\nevent GO\nevent RESET\n\
                    transition idle GO when tries <= 0 -> done\n\
                    transition idle GO when tries >= 2 -> done\n\
                    transition done RESET -> idle\n";
    assert!(verify(&parse_machine(disjoint).unwrap()).is_empty());

    let non_interval = "machine fine\ninitial idle\nstate idle\nstate done\nevent GO\nevent RESET\n\
                        transition idle GO when tries != limit -> done\n\
                        transition idle GO when tries == 1 -> done\n\
                        transition done RESET -> idle\n";
    assert!(verify(&parse_machine(non_interval).unwrap()).is_empty());
}

#[test]
fn fuzz_is_reproducible_and_reports_failures_by_seed() {
    let machine = parse_machine(STRICT).unwrap();
    let first = fuzz(&machine, 32, 12, Some("tries_bound"));
    let second = fuzz(&machine, 32, 12, Some("tries_bound"));
    assert_eq!(first, second);
    assert!(first.failures > 0);
    assert_eq!(first.seeds, 32);
    assert_eq!(first.events_per_seed, 12);
    let seeds: Vec<u64> = first.cases.iter().map(|case| case.seed).collect();
    assert!(seeds.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(first
        .cases
        .iter()
        .all(|case| case.reason.starts_with("invariant:")));
    let rendered = render_fuzz(&first);
    assert!(rendered.starts_with("# sextant-fuzz v1\n"));
    assert!(rendered.contains("failures: "));
}

#[test]
fn invariant_filter_ignores_stuck_replays() {
    let machine = parse_machine(RETRY).unwrap();
    let filtered = fuzz(&machine, 32, 12, Some("tries_bound"));
    assert_eq!(filtered.failures, 0);
    let unfiltered = fuzz(&machine, 32, 12, None);
    assert!(unfiltered.failures > 0);
    assert!(unfiltered.cases.iter().any(|case| case.reason == "stuck"));
}
