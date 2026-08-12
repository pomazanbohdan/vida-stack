use std::collections::BTreeSet;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectionDifference {
    pub path: String,
    pub management: Option<String>,
    pub dispatch: Option<String>,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectionComparison {
    pub parity_gate: &'static str,
    pub differences: Vec<ProjectionDifference>,
    pub authoritative_write_count: u64,
    pub external_effect_count: u64,
    pub metadata_complete: bool,
}

/// Compare two read-only projections using an explicit difference ledger.
/// Missing/extra fields are differences; no runtime surface is invoked.
pub fn compare_management_dispatch_projection(
    management: &serde_json::Value,
    dispatch: &serde_json::Value,
    allowed_paths: &[&str],
) -> ProjectionComparison {
    let allowed = allowed_paths.iter().copied().collect::<BTreeSet<_>>();
    let mut differences = Vec::new();
    collect_differences(
        Some(management),
        Some(dispatch),
        "$",
        &allowed,
        &mut differences,
    );

    let (management_writes, management_writes_complete) =
        required_counter(management, "authoritative_write_count");
    let (dispatch_writes, dispatch_writes_complete) =
        required_counter(dispatch, "authoritative_write_count");
    let (management_effects, management_effects_complete) =
        required_counter(management, "external_effect_count");
    let (dispatch_effects, dispatch_effects_complete) =
        required_counter(dispatch, "external_effect_count");
    let authoritative_write_count = management_writes
        .checked_add(dispatch_writes)
        .unwrap_or(u64::MAX);
    let external_effect_count = management_effects
        .checked_add(dispatch_effects)
        .unwrap_or(u64::MAX);
    let metadata_complete = management_writes_complete
        && dispatch_writes_complete
        && management_effects_complete
        && dispatch_effects_complete;
    let unexplained = differences.iter().any(|difference| !difference.allowed);
    let parity_gate = if !metadata_complete
        || unexplained
        || authoritative_write_count > 0
        || external_effect_count > 0
    {
        "blocked"
    } else {
        "pass"
    };

    ProjectionComparison {
        parity_gate,
        differences,
        authoritative_write_count,
        external_effect_count,
        metadata_complete,
    }
}

fn required_counter(value: &serde_json::Value, key: &str) -> (u64, bool) {
    match value.get(key).and_then(serde_json::Value::as_u64) {
        Some(value) => (value, true),
        None => (0, false),
    }
}

fn collect_differences(
    management: Option<&serde_json::Value>,
    dispatch: Option<&serde_json::Value>,
    path: &str,
    allowed: &BTreeSet<&str>,
    differences: &mut Vec<ProjectionDifference>,
) {
    if management == dispatch {
        return;
    }

    match (management, dispatch) {
        (Some(serde_json::Value::Object(left)), Some(serde_json::Value::Object(right))) => {
            let keys = left
                .keys()
                .chain(right.keys())
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            for key in keys {
                let child_path = if path == "$" {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                collect_differences(
                    left.get(key),
                    right.get(key),
                    &child_path,
                    allowed,
                    differences,
                );
            }
        }
        (Some(serde_json::Value::Array(left)), Some(serde_json::Value::Array(right))) => {
            for index in 0..left.len().max(right.len()) {
                let child_path = format!("{path}[{index}]");
                collect_differences(
                    left.get(index),
                    right.get(index),
                    &child_path,
                    allowed,
                    differences,
                );
            }
        }
        _ => differences.push(ProjectionDifference {
            allowed: allowed.contains(path),
            path: path.to_string(),
            management: management.map(ToString::to_string),
            dispatch: dispatch.map(ToString::to_string),
        }),
    }
}

pub fn assert_shadow_report_clean(report: &serde_json::Value) {
    assert_eq!(report["unexplained_difference_count"], 0);
    assert_eq!(report["authoritative_write_count"], 0);
    assert_eq!(report["external_effect_count"], 0);
    assert_eq!(report["parity_gate"], "pass");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_shadow_report_assertion_accepts_zero_diff_no_write_report() {
        let report = serde_json::json!({
            "unexplained_difference_count": 0,
            "authoritative_write_count": 0,
            "external_effect_count": 0,
            "parity_gate": "pass"
        });

        assert_shadow_report_clean(&report);
    }

    #[test]
    fn clean_shadow_report_assertion_rejects_each_non_clean_gate_field() {
        let fields = [
            ("unexplained_difference_count", serde_json::json!(1)),
            ("authoritative_write_count", serde_json::json!(1)),
            ("external_effect_count", serde_json::json!(1)),
            ("parity_gate", serde_json::json!("blocked")),
        ];

        for (field, value) in fields {
            let mut report = serde_json::json!({
                "unexplained_difference_count": 0,
                "authoritative_write_count": 0,
                "external_effect_count": 0,
                "parity_gate": "pass"
            });
            report[field] = value;
            assert!(
                std::panic::catch_unwind(|| assert_shadow_report_clean(&report)).is_err(),
                "non-clean shadow field `{field}` must fail closed"
            );
        }
    }

    #[test]
    fn projection_comparator_accepts_only_declared_differences_without_writes() {
        let management = serde_json::json!({"state":"active","projection":"management","mode":"read","authoritative_write_count":0,"external_effect_count":0});
        let dispatch = serde_json::json!({"state":"active","projection":"dispatch","mode":"read","authoritative_write_count":0,"external_effect_count":0});
        let comparison =
            compare_management_dispatch_projection(&management, &dispatch, &["projection"]);
        assert_eq!(comparison.parity_gate, "pass");
        assert_eq!(comparison.differences.len(), 1);
        assert!(comparison.differences[0].allowed);
        assert!(comparison.metadata_complete);
    }

    #[test]
    fn projection_comparator_blocks_unexplained_differences_and_effects() {
        let management = serde_json::json!({"state":"active","external_effect_count":1,"authoritative_write_count":0});
        let dispatch = serde_json::json!({"state":"completed","external_effect_count":0,"authoritative_write_count":0});
        let comparison = compare_management_dispatch_projection(&management, &dispatch, &[]);
        assert_eq!(comparison.parity_gate, "blocked");
        assert!(comparison
            .differences
            .iter()
            .any(|difference| !difference.allowed));
        assert_eq!(comparison.external_effect_count, 1);
    }

    #[test]
    fn projection_comparator_blocks_missing_metadata_and_root_shape_differences() {
        let management = serde_json::json!([]);
        let dispatch = serde_json::json!({});
        let comparison = compare_management_dispatch_projection(&management, &dispatch, &[]);
        assert_eq!(comparison.parity_gate, "blocked");
        assert!(!comparison.metadata_complete);
        assert_eq!(comparison.differences[0].path, "$");
    }
}
