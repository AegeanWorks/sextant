//! Tests for the std-only deterministic JSON renderer.

use sextant::core::{
    check_json, explore_json, fuzz_json, json_to_string, parse_machine, verify, explore, fuzz, Json,
};

const RETRY: &str = include_str!("../examples/retry.sex");
const STRICT: &str = include_str!("../examples/strict.sex");

#[test]
fn escaping_is_exact_and_deterministic() {
    let value = Json::string("quote \" backslash \\ newline \n tab \t control \u{01}");
    assert_eq!(
        json_to_string(&value),
        "\"quote \\\" backslash \\\\ newline \\n tab \\t control \\u0001\""
    );
}

#[test]
fn object_key_order_is_preserved() {
    let value = Json::object(vec![
        ("z", Json::number(1)),
        ("a", Json::boolean(true)),
        ("m", Json::Null),
    ]);
    assert_eq!(json_to_string(&value), "{\"z\":1,\"a\":true,\"m\":null}");
}

#[test]
fn check_json_reports_counts_and_findings() {
    let machine = parse_machine(RETRY).unwrap();
    let findings = verify(&machine);
    let rendered = json_to_string(&check_json(&machine, &findings));
    assert!(rendered.starts_with("{\"machine\":\"retry\",\"ok\":true,"));
    assert!(rendered.contains("\"transitions\":4"));
    assert!(rendered.contains("\"findings\":[]"));

    let broken = parse_machine(&RETRY.replace("state done", "state done\nstate orphan")).unwrap();
    let findings = verify(&broken);
    let rendered = json_to_string(&check_json(&broken, &findings));
    assert!(rendered.contains("\"ok\":false"));
    assert!(rendered.contains("UnreachableState"));
    assert_eq!(rendered, json_to_string(&check_json(&broken, &verify(&broken))));
}

#[test]
fn explore_and_fuzz_json_are_stable() {
    let strict = parse_machine(STRICT).unwrap();
    let exploration = explore(&strict, 6, 10_000);
    let rendered = json_to_string(&explore_json(&exploration));
    assert!(rendered.contains("\"ok\":false"));
    assert!(rendered.contains("\"rules_checked\":1"));
    assert!(rendered.contains("\"type\":\"invariant\""));
    assert_eq!(rendered, json_to_string(&explore_json(&explore(&strict, 6, 10_000))));

    let retry = parse_machine(RETRY).unwrap();
    let fuzz_result = fuzz(&retry, 8, 12, Some("tries_bound"));
    let rendered = json_to_string(&fuzz_json(&fuzz_result));
    assert!(rendered.contains("\"failures\":0"));
    assert!(rendered.contains("\"events_per_seed\":12"));
    assert_eq!(
        rendered,
        json_to_string(&fuzz_json(&fuzz(&retry, 8, 12, Some("tries_bound"))))
    );
}
