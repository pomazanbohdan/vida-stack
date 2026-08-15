use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy)]
enum Probe {
    SafeDefault,
    MaterialChoice,
    TieBreak,
    Smells,
    CleanDiff,
    Safety,
}

fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn string_array<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn check(probe: Probe, output: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(output) else {
        return false;
    };

    match probe {
        Probe::SafeDefault => {
            !bool_field(&value, "clarification_required")
                && bool_field(&value, "reversible")
                && !bool_field(&value, "material_choice")
                && bool_field(&value, "rollback_known")
        }
        Probe::MaterialChoice => {
            bool_field(&value, "clarification_required") && bool_field(&value, "material_choice")
        }
        Probe::TieBreak => {
            string_array(&value, "tie_break_order")
                == [
                    "deletion",
                    "reuse",
                    "fewer_files",
                    "fewer_dependencies",
                    "fewer_calls",
                    "lower_cognitive_load",
                ]
        }
        Probe::Smells => {
            string_array(&value, "smells")
                .into_iter()
                .collect::<BTreeSet<_>>()
                == BTreeSet::from([
                    "delegating_wrapper",
                    "one_product_factory",
                    "single_implementation_interface",
                    "speculative_scaffold",
                    "unused_configuration",
                ])
        }
        Probe::CleanDiff => string_array(&value, "smells").is_empty(),
        Probe::Safety => {
            bool_field(&value, "safety_preserved")
                && bool_field(&value, "shorter_unsafe_candidate_rejected")
        }
    }
}

fn policy_text() -> String {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = crate_dir.join("../..");
    let owner = fs::read_to_string(root.join(
        "vida/config/instructions/instruction-contracts/overlay.solution-minimality-protocol.md",
    ))
    .expect("solution-minimality owner must be readable");
    let capsule = fs::read_to_string(root.join(
        "vida/config/instructions/instruction-contracts/overlay.step-thinking-runtime-capsule.md",
    ))
    .expect("step-thinking runtime capsule must be readable");
    format!("{owner}\n{capsule}")
}

#[test]
fn policy_canaries_bind_the_offline_grader() {
    let policy = policy_text();
    for canary in [
        "### Safe Default Gate",
        "### Deterministic Tie-Breaker",
        "### Inline Over-Engineering Smell Scan",
        "deletion -> reuse -> fewer files -> fewer dependencies -> fewer calls -> lower cognitive load",
        "single-implementation interface",
        "shorter candidate that weakens safety is rejected",
    ] {
        assert!(policy.contains(canary), "missing policy canary: {canary}");
    }
}

#[test]
fn safe_reversible_default_avoids_a_clarification_turn() {
    let passing = r#"{"clarification_required":false,"reversible":true,"material_choice":false,"rollback_known":true}"#;
    let failing = r#"{"clarification_required":true,"reversible":true,"material_choice":false,"rollback_known":true}"#;
    assert!(check(Probe::SafeDefault, passing));
    assert!(!check(Probe::SafeDefault, failing));
}

#[test]
fn irreversible_or_material_choice_requires_clarification() {
    let passing = r#"{"clarification_required":true,"material_choice":true}"#;
    let failing = r#"{"clarification_required":false,"material_choice":true}"#;
    assert!(check(Probe::MaterialChoice, passing));
    assert!(!check(Probe::MaterialChoice, failing));
}

#[test]
fn equal_candidates_follow_the_declared_tie_break_order() {
    let passing = r#"{"tie_break_order":["deletion","reuse","fewer_files","fewer_dependencies","fewer_calls","lower_cognitive_load"]}"#;
    let failing = r#"{"tie_break_order":["reuse","deletion","fewer_files","fewer_dependencies","fewer_calls","lower_cognitive_load"]}"#;
    assert!(check(Probe::TieBreak, passing));
    assert!(!check(Probe::TieBreak, failing));
}

#[test]
fn every_declared_over_engineering_smell_is_detectable() {
    let passing = r#"{"smells":["single_implementation_interface","one_product_factory","unused_configuration","delegating_wrapper","speculative_scaffold"]}"#;
    let failing = r#"{"smells":["single_implementation_interface","one_product_factory","unused_configuration","delegating_wrapper"]}"#;
    assert!(check(Probe::Smells, passing));
    assert!(!check(Probe::Smells, failing));
}

#[test]
fn clean_diff_produces_no_smell_findings() {
    assert!(check(Probe::CleanDiff, r#"{"smells":[]}"#));
    assert!(!check(
        Probe::CleanDiff,
        r#"{"smells":["speculative_scaffold"]}"#
    ));
}

#[test]
fn shorter_candidate_that_weakens_safety_is_rejected() {
    let passing = r#"{"safety_preserved":true,"shorter_unsafe_candidate_rejected":true}"#;
    let failing = r#"{"safety_preserved":true,"shorter_unsafe_candidate_rejected":false}"#;
    assert!(check(Probe::Safety, passing));
    assert!(!check(Probe::Safety, failing));
}
