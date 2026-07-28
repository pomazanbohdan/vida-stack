use serde_json::json;
use vida_policy_rhai::{build_policy_engine, Limits, PolicyBundle, PolicyBundleError, PolicyError};

const DYNAMIC_EVAL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/policy-runtime/hard-boundaries/zombies-dynamic-eval.json"
));
const IMPORT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/policy-runtime/hard-boundaries/r-import.json"
));
const UNKNOWN_CONTEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/policy-runtime/hard-boundaries/p-unknown-context.json"
));
const SCRIPT_OVER_LIMIT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/policy-runtime/hard-boundaries/c-script-over-limit.json"
));
const UNKNOWN_FIELD: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/policy-runtime/hard-boundaries/invalid-unknown-field.json"
));

fn hard_boundary_engine() -> vida_policy_rhai::PolicyEngine {
    build_policy_engine(Limits {
        max_operations: 128,
        max_call_levels: 4,
        max_expr_depth: 32,
        max_string_size: 128,
        max_array_size: 8,
        max_map_size: 8,
        max_script_size: 64,
        max_context_size: 128,
    })
}

fn assert_rejected_before_effective_decision(name: &str, raw: &str) {
    let bundle = PolicyBundle::from_json(raw).unwrap_or_else(|error| {
        panic!("{name} fixture must be a structurally valid bundle: {error}")
    });
    let result = hard_boundary_engine().evaluate(
        &bundle.source,
        json!({
            "requested_decision": "allow",
            "sentinel": "effective-decision-must-not-run",
        }),
    );
    assert!(
        matches!(
            result,
            Err(PolicyError::Evaluation(_)) | Err(PolicyError::ScriptTooLarge { .. })
        ),
        "{name} must fail before producing an effective decision, got {result:?}"
    );
}

#[test]
fn policy_hard_boundaries_reject_unsafe_inputs_before_decision() {
    assert_rejected_before_effective_decision("zombies-dynamic-eval", DYNAMIC_EVAL);
    assert_rejected_before_effective_decision("r-import", IMPORT);
    assert_rejected_before_effective_decision("p-unknown-context", UNKNOWN_CONTEXT);
    assert_rejected_before_effective_decision("c-script-over-limit", SCRIPT_OVER_LIMIT);
}

#[test]
fn policy_hard_boundaries_reject_unknown_bundle_fields_before_decision() {
    assert!(matches!(
        PolicyBundle::from_json(UNKNOWN_FIELD),
        Err(PolicyBundleError::Malformed(_))
    ));
}
