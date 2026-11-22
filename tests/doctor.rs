//! The self-check must be pure, complete and deterministic.

use sextant::core::{doctor_json, json_to_string, render_doctor, self_check};

#[test]
fn every_builtin_check_passes() {
    let results = self_check();
    assert_eq!(results.len(), 11);
    assert!(
        results.iter().all(|item| item.ok),
        "{:?}",
        results.iter().filter(|item| !item.ok).collect::<Vec<_>>()
    );
}

#[test]
fn doctor_output_is_byte_stable_in_both_formats() {
    let results = self_check();
    let text = render_doctor(&results);
    assert!(text.starts_with("# sextant-doctor v1\n"));
    assert!(text.contains("11/11 checks passed"));
    assert_eq!(text, render_doctor(&self_check()));

    let json = json_to_string(&doctor_json(&results));
    assert!(json.starts_with("{\"ok\":true,\"passed\":11,\"total\":11,"));
    assert_eq!(json, json_to_string(&doctor_json(&self_check())));
}
