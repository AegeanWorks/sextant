//! Tests for behavioural properties and bounded machine equivalence.

use sextant::core::{
    equivalent, explore, parse_machine, render_equivalence, verify, PropertyKind, RuleType,
};

const RETRY: &str = include_str!("../examples/retry.sex");
const RENAMED: &str = include_str!("../examples/retry_renamed.sex");
const PROPERTIES: &str = include_str!("../examples/properties.sex");
const STRICT: &str = include_str!("../examples/strict.sex");

#[test]
fn properties_parse_and_refer_to_declared_symbols() {
    let machine = parse_machine(PROPERTIES).unwrap();
    assert_eq!(machine.properties.len(), 3);
    assert_eq!(machine.properties[0].name, "reaches_done");
    assert!(matches!(machine.properties[0].kind, PropertyKind::Reaches(_)));
    assert!(matches!(machine.properties[1].kind, PropertyKind::Avoids(_)));
    assert!(matches!(machine.properties[2].kind, PropertyKind::Emits(_)));
    assert!(verify(&machine).is_empty());
}

#[test]
fn verify_rejects_a_property_that_names_an_undeclared_state() {
    let broken = PROPERTIES.replace("reaches done", "reaches nowhere");
    let machine = parse_machine(&broken).unwrap();
    let findings = verify(&machine);
    assert!(findings
        .iter()
        .any(|finding| finding.message().contains("nowhere")));
}

#[test]
fn exploration_checks_properties_and_names_the_violation() {
    let machine = parse_machine(PROPERTIES).unwrap();
    let result = explore(&machine, 6, 10_000);
    assert_eq!(result.rules_checked, 4);
    let property_violations: Vec<&str> = result
        .violations
        .iter()
        .filter(|violation| violation.rule_type == RuleType::Property)
        .map(|violation| violation.rule.as_str())
        .collect();
    assert_eq!(property_violations, vec!["avoids_waiting"]);
    assert!(result
        .violations
        .iter()
        .any(|violation| violation.rule == "tries_bound"
            && violation.rule_type == RuleType::Invariant) == false);
    assert_eq!(result, explore(&machine, 6, 10_000));
}

#[test]
fn renamed_states_are_behaviourally_equivalent() {
    let left = parse_machine(RETRY).unwrap();
    let right = parse_machine(RENAMED).unwrap();
    let result = equivalent(&left, &right, 4, 100_000);
    assert!(result.differences.is_empty());
    assert_eq!(result.sequences, 341);
    assert!(!result.truncated);
    assert_eq!(result, equivalent(&left, &right, 4, 100_000));
    assert!(render_equivalence(&result, "retry", "retry_renamed")
        .contains("differences: 0"));
}

#[test]
fn a_tighter_invariant_is_a_behavioural_difference() {
    let left = parse_machine(RETRY).unwrap();
    let right = parse_machine(STRICT).unwrap();
    let result = equivalent(&left, &right, 4, 100_000);
    assert!(!result.differences.is_empty());
    assert!(result.differences.iter().all(|difference| difference.left != difference.right));
    let rendered = render_equivalence(&result, "retry", "strict");
    assert!(rendered.starts_with("# sextant-equivalence v1\n"));
    assert!(rendered.contains("err:invariant:tries_bound"));
}
