use std::{collections::BTreeSet, path::Path};

use serde_json::{Value, json};

use crate::state_store::TaskRecord;

const GATE_ID: &str = "zombie_d_test_writing";
const REQUIRED_CATEGORIES: [&str; 7] = ["Z", "O", "M", "B", "I", "E", "S"];
const DEFAULT_TASK_CLASSES: [&str; 4] = [
    "test_authoring",
    "regression_test",
    "verification",
    "quality_gate",
];
const DEFAULT_PATH_TOKENS: [&str; 7] = [
    "test",
    "fixture",
    "snapshot",
    "golden",
    "coverage",
    "smoke",
    "integration",
];
const DEFAULT_ENFORCEMENT_POINTS: [&str; 3] = ["dispatch", "handoff", "closure"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZombieDPolicy {
    enabled: bool,
    gate_id: String,
    required_categories: Vec<String>,
    task_classes: BTreeSet<String>,
    path_tokens: Vec<String>,
    enforcement_points: BTreeSet<String>,
    config_blockers: Vec<String>,
}

pub(crate) fn project_policy(
    dev_team: &serde_yaml::Value,
    blockers: &mut Vec<String>,
) -> Value {
    let gate = crate::yaml_lookup(dev_team, &["zombie_d_gate"]);
    let enabled = crate::yaml_bool(
        gate.and_then(|value| crate::yaml_lookup(value, &["enabled"])),
        true,
    );
    let gate_id = crate::yaml_string(
        gate.and_then(|value| crate::yaml_lookup(value, &["gate_id"])),
    )
    .unwrap_or_else(|| GATE_ID.to_string());
    let required_categories = string_list_or_default(
        gate.and_then(|value| crate::yaml_lookup(value, &["required_categories"])),
        &REQUIRED_CATEGORIES,
    );
    let task_classes = string_list_or_default(
        gate.and_then(|value| crate::yaml_lookup(value, &["applies_to", "task_classes"])),
        &DEFAULT_TASK_CLASSES,
    );
    let path_tokens = string_list_or_default(
        gate.and_then(|value| crate::yaml_lookup(value, &["applies_to", "path_tokens"])),
        &DEFAULT_PATH_TOKENS,
    );
    let enforcement_points = string_list_or_default(
        gate.and_then(|value| crate::yaml_lookup(value, &["enforcement_points"])),
        &DEFAULT_ENFORCEMENT_POINTS,
    );
    let mut config_blockers = Vec::new();
    if gate_id.trim().is_empty() {
        config_blockers.push("invalid_zombie_d_gate_id".to_string());
    }
    if required_categories
        != REQUIRED_CATEGORIES
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    {
        config_blockers.push("invalid_zombie_d_required_categories".to_string());
    }
    if task_classes.is_empty() {
        config_blockers.push("empty_zombie_d_task_classes".to_string());
    }
    if path_tokens.is_empty() {
        config_blockers.push("empty_zombie_d_path_tokens".to_string());
    }
    if enforcement_points.is_empty() {
        config_blockers.push("empty_zombie_d_enforcement_points".to_string());
    }
    for blocker in &config_blockers {
        blockers.push(blocker.clone());
    }
    json!({
        "enabled": enabled,
        "gate_id": gate_id,
        "required_categories": required_categories,
        "applies_to": {
            "task_classes": task_classes,
            "path_tokens": path_tokens,
        },
        "enforcement_points": enforcement_points,
        "status": if config_blockers.is_empty() { "ready" } else { "blocked" },
        "blockers": config_blockers,
        "default_enabled": true,
        "configured": gate.is_some(),
    })
}

pub(crate) fn evaluate_from_readiness(
    readiness: &Value,
    task: &TaskRecord,
    task_class: Option<&str>,
    enforcement_point: &str,
) -> Value {
    let policy = policy_from_projection(readiness.get("zombie_d_gate"));
    evaluate(&policy, task, task_class, enforcement_point)
}

pub(crate) fn evaluate_from_project_root(
    project_root: Option<&Path>,
    task: &TaskRecord,
    enforcement_point: &str,
) -> Value {
    let Some(project_root) = project_root else {
        return blocked_result(
            task,
            enforcement_point,
            vec!["zombie_d_gate_config_unavailable".to_string()],
            vec![
                "Resolve the project root before evaluating the ZOMBIE-D gate.".to_string(),
            ],
            Value::Null,
        );
    };
    let config_path = crate::config_file_path_for_root(project_root);
    let config_text = match std::fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let default_dev_team = serde_yaml::Value::Mapping(Default::default());
            let mut config_blockers = Vec::new();
            let projection = project_policy(&default_dev_team, &mut config_blockers);
            let mut result = evaluate_from_readiness(
                &json!({"zombie_d_gate": projection}),
                task,
                None,
                enforcement_point,
            );
            result["artifact_refs"]["config_path"] = json!(config_path.display().to_string());
            return result;
        }
        Err(error) => {
            return blocked_result(
                task,
                enforcement_point,
                vec!["zombie_d_gate_config_unavailable".to_string()],
                vec![format!(
                    "Read `{}` before closing the task: {error}",
                    config_path.display()
                )],
                json!({"config_path": config_path.display().to_string()}),
            );
        }
    };
    let overlay: serde_yaml::Value = match serde_yaml::from_str(&config_text) {
        Ok(value) => value,
        Err(error) => {
            return blocked_result(
                task,
                enforcement_point,
                vec!["zombie_d_gate_config_invalid".to_string()],
                vec![format!(
                    "Repair `{}` before closing the task: {error}",
                    config_path.display()
                )],
                json!({"config_path": config_path.display().to_string()}),
            );
        }
    };
    let default_dev_team = serde_yaml::Value::Mapping(Default::default());
    let dev_team = crate::yaml_lookup(&overlay, &["dev_team"]).unwrap_or(&default_dev_team);
    let mut config_blockers = Vec::new();
    let projection = project_policy(dev_team, &mut config_blockers);
    let mut result = evaluate_from_readiness(
        &json!({"zombie_d_gate": projection}),
        task,
        None,
        enforcement_point,
    );
    if !config_blockers.is_empty() {
        result["status"] = json!("blocked");
    }
    result["artifact_refs"]["config_path"] = json!(config_path.display().to_string());
    result
}

pub(crate) fn close_block_payload(task: &TaskRecord, result: &Value) -> Option<Value> {
    if result["status"].as_str() != Some("blocked") {
        return None;
    }
    let blocker_codes = string_array(result.get("blocker_codes"));
    let operator_blocker_codes = if blocker_codes
        .iter()
        .any(|code| code.starts_with("zombie_d_"))
    {
        vec!["missing_gate_evidence".to_string()]
    } else {
        blocker_codes
    };
    let next_actions = string_array(result.get("next_actions"));
    crate::release1_operator_output::Release1OperatorOutputBuilder::new("vida task close")
            .blocker_codes(operator_blocker_codes)
            .next_actions(next_actions)
            .artifact_refs(result["artifact_refs"].clone())
            .extra_fields(json!({
                "closed": false,
                "continuation_blocked": true,
                "automation_blocked": false,
                "task_id": task.id,
                "reason": "ZOMBIE-D semantic gate is incomplete",
                "zombie_d_gate": result,
            }))
            .build()
            .ok()
}

fn policy_from_projection(value: Option<&Value>) -> ZombieDPolicy {
    let value = value.unwrap_or(&Value::Null);
    ZombieDPolicy {
        enabled: value.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        gate_id: value
            .get("gate_id")
            .and_then(Value::as_str)
            .unwrap_or(GATE_ID)
            .to_string(),
        required_categories: string_array(value.get("required_categories")),
        task_classes: lower_string_array(value.pointer("/applies_to/task_classes"))
            .into_iter()
            .collect(),
        path_tokens: lower_string_array(value.pointer("/applies_to/path_tokens")),
        enforcement_points: lower_string_array(value.get("enforcement_points"))
            .into_iter()
            .collect(),
        config_blockers: string_array(value.get("blockers")),
    }
}

fn evaluate(
    policy: &ZombieDPolicy,
    task: &TaskRecord,
    task_class: Option<&str>,
    enforcement_point: &str,
) -> Value {
    let artifact_refs = json!({
        "task_id": task.id,
        "gate_id": policy.gate_id,
        "enforcement_point": enforcement_point,
        "proof_target": "zombie_d_matrix",
    });
    if !policy.enabled {
        return json!({
            "status": "disabled",
            "enabled": false,
            "applicable": false,
            "gate_id": policy.gate_id,
            "enforcement_point": enforcement_point,
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": artifact_refs,
        });
    }
    if !policy.enforcement_points.contains(enforcement_point) {
        return json!({
            "status": "not_applicable",
            "enabled": true,
            "applicable": false,
            "gate_id": policy.gate_id,
            "enforcement_point": enforcement_point,
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": artifact_refs,
        });
    }
    let applicable = task_is_applicable(policy, task, task_class);
    if !applicable {
        return json!({
            "status": "not_applicable",
            "enabled": true,
            "applicable": false,
            "gate_id": policy.gate_id,
            "enforcement_point": enforcement_point,
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": artifact_refs,
        });
    }
    if !policy.config_blockers.is_empty() {
        return blocked_result(
            task,
            enforcement_point,
            policy.config_blockers.clone(),
            vec!["Repair the configured ZOMBIE-D gate policy before dispatch or closure.".to_string()],
            artifact_refs,
        );
    }
    let Some(matrix) = zombie_d_matrix_from_notes(task.notes.as_deref()) else {
        return blocked_result(
            task,
            enforcement_point,
            vec!["zombie_d_matrix_missing".to_string()],
            vec![format!(
                "Attach structured ZOMBIE-D evidence with `vida task proof attach-evidence {} --proof-target zombie_d_matrix --result pass --evidence '<matrix-json>'`.",
                crate::shell_quote(&task.id)
            )],
            artifact_refs,
        );
    };
    validate_matrix(policy, task, enforcement_point, matrix, artifact_refs)
}

fn task_is_applicable(policy: &ZombieDPolicy, task: &TaskRecord, task_class: Option<&str>) -> bool {
    if task_class
        .map(|value| policy.task_classes.contains(&value.to_ascii_lowercase()))
        .unwrap_or(false)
    {
        return true;
    }
    let owned_paths = task
        .planner_metadata
        .owned_paths
        .join(" ")
        .to_ascii_lowercase();
    let explicit_gate_marker = format!(
        "{} {} {} {}",
        task.title,
        task.description,
        task.labels.join(" "),
        task.notes.as_deref().unwrap_or_default(),
    )
    .to_ascii_lowercase();
    policy
        .path_tokens
        .iter()
        .any(|token| owned_paths.contains(&token.to_ascii_lowercase()))
        || explicit_gate_marker.contains("zombie-d")
        || explicit_gate_marker.contains("zombie_d")
}

fn validate_matrix(
    policy: &ZombieDPolicy,
    task: &TaskRecord,
    enforcement_point: &str,
    matrix: Value,
    artifact_refs: Value,
) -> Value {
    let mut blockers = Vec::new();
    let mut next_actions = Vec::new();
    let Some(object) = matrix.as_object() else {
        return blocked_result(
            task,
            enforcement_point,
            vec!["zombie_d_matrix_invalid".to_string()],
            vec!["Provide the ZOMBIE-D matrix as a JSON object.".to_string()],
            artifact_refs,
        );
    };
    if object.get("schema_version").and_then(Value::as_u64) != Some(1) {
        blockers.push("zombie_d_matrix_schema_invalid".to_string());
    }
    let Some(categories) = object.get("categories").and_then(Value::as_object) else {
        blockers.push("zombie_d_categories_missing".to_string());
        next_actions.push("Add categories Z/O/M/B/I/E/S to the matrix.".to_string());
        return blocked_result(task, enforcement_point, blockers, next_actions, artifact_refs);
    };
    for category in &policy.required_categories {
        let Some(row) = categories.get(category) else {
            blockers.push(format!("zombie_d_category_missing:{category}"));
            continue;
        };
        let status = row
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match status {
            "pass" => {
                let evidence_refs = row
                    .get("evidence_refs")
                    .and_then(Value::as_array)
                    .map(|values| !values.is_empty())
                    .unwrap_or(false);
                if !evidence_refs {
                    blockers.push(format!("zombie_d_evidence_missing:{category}"));
                }
            }
            "na" => {
                if row
                    .get("reason")
                    .and_then(Value::as_str)
                    .map_or(true, |reason| reason.trim().is_empty())
                {
                    blockers.push(format!("zombie_d_na_reason_missing:{category}"));
                }
            }
            _ => blockers.push(format!("zombie_d_category_unproven:{category}")),
        }
    }
    if object
        .get("doubts")
        .and_then(Value::as_array)
        .is_some_and(|doubts| !doubts.is_empty())
    {
        blockers.push("zombie_d_doubt_unresolved".to_string());
        next_actions.push("Resolve every doubt row before continuing.".to_string());
    }
    if blockers.is_empty() {
        json!({
            "status": "pass",
            "enabled": true,
            "applicable": true,
            "gate_id": policy.gate_id,
            "enforcement_point": enforcement_point,
            "matrix": matrix,
            "blocker_codes": [],
            "next_actions": [],
            "artifact_refs": artifact_refs,
        })
    } else {
        next_actions.push(format!(
            "Complete the missing ZOMBIE-D rows and attach structured evidence for task `{}`.",
            task.id
        ));
        blocked_result(task, enforcement_point, blockers, next_actions, artifact_refs)
    }
}

fn blocked_result(
    task: &TaskRecord,
    enforcement_point: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    json!({
        "status": "blocked",
        "enabled": true,
        "applicable": true,
        "gate_id": artifact_refs
            .get("gate_id")
            .and_then(Value::as_str)
            .unwrap_or(GATE_ID),
        "enforcement_point": enforcement_point,
        "task_id": task.id,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "artifact_refs": artifact_refs,
    })
}

fn zombie_d_matrix_from_notes(notes: Option<&str>) -> Option<Value> {
    let notes = notes?;
    let mut in_proof_note = false;
    let mut proof_target = None;
    let mut result = None;
    let mut evidence = None;
    let finish = |proof_target: &mut Option<String>,
                  result: &mut Option<String>,
                  evidence: &mut Option<String>| {
        if proof_target.as_deref() == Some("zombie_d_matrix")
            && result.as_deref() == Some("pass")
        {
            if let Some(raw) = evidence.as_deref() {
                if let Some(value) = parse_matrix_evidence(raw) {
                    return Some(value);
                }
            }
        }
        *proof_target = None;
        *result = None;
        *evidence = None;
        None
    };
    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed == "task_proof_evidence:" {
            if let Some(value) = finish(&mut proof_target, &mut result, &mut evidence) {
                return Some(value);
            }
            in_proof_note = true;
            continue;
        }
        if in_proof_note && !line.starts_with(' ') && !trimmed.is_empty() {
            if let Some(value) = finish(&mut proof_target, &mut result, &mut evidence) {
                return Some(value);
            }
            in_proof_note = false;
        }
        if !in_proof_note {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("proof_target:") {
            proof_target = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("result:") {
            result = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("evidence:") {
            evidence = Some(value.trim().to_string());
        }
    }
    finish(&mut proof_target, &mut result, &mut evidence)
}

fn parse_matrix_evidence(raw: &str) -> Option<Value> {
    serde_json::from_str::<Value>(raw.trim()).ok().or_else(|| {
        serde_yaml::from_str::<serde_yaml::Value>(raw.trim())
            .ok()
            .and_then(|value| serde_json::to_value(value).ok())
    })
}

fn string_list_or_default(value: Option<&serde_yaml::Value>, defaults: &[&str]) -> Vec<String> {
    match value {
        Some(value) => crate::yaml_string_list(Some(value)),
        None => defaults.iter().map(|value| value.to_string()).collect(),
    }
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn lower_string_array(value: Option<&Value>) -> Vec<String> {
    string_array(value)
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

pub(crate) fn string_array_for_operator(value: Option<&Value>) -> Vec<String> {
    string_array(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(notes: Option<&str>, labels: &[&str], owned_paths: &[&str]) -> TaskRecord {
        serde_json::from_value(json!({
            "id": "task-zombie-d",
            "title": "Update runtime tests",
            "description": "Add integration coverage",
            "status": "open",
            "priority": 2,
            "issue_type": "task",
            "created_at": "now",
            "created_by": "test",
            "updated_at": "now",
            "closed_at": null,
            "close_reason": null,
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "notes": notes,
            "labels": labels,
            "planner_metadata": {"owned_paths": owned_paths, "proof_targets": []},
            "dependencies": []
        }))
        .expect("fixture task should deserialize")
    }

    fn readiness(enabled: bool) -> Value {
        json!({
            "zombie_d_gate": {
                "enabled": enabled,
                "gate_id": GATE_ID,
                "required_categories": REQUIRED_CATEGORIES,
                "applies_to": {"task_classes": DEFAULT_TASK_CLASSES, "path_tokens": DEFAULT_PATH_TOKENS},
                "enforcement_points": DEFAULT_ENFORCEMENT_POINTS,
                "status": "ready",
                "blockers": []
            }
        })
    }

    fn matrix_note() -> String {
        let matrix = json!({
            "schema_version": 1,
            "categories": {
                "Z": {"status": "pass", "evidence_refs": ["z"]},
                "O": {"status": "pass", "evidence_refs": ["o"]},
                "M": {"status": "na", "reason": "single fixture contract"},
                "B": {"status": "pass", "evidence_refs": ["b"]},
                "I": {"status": "pass", "evidence_refs": ["i"]},
                "E": {"status": "pass", "evidence_refs": ["e"]},
                "S": {"status": "pass", "evidence_refs": ["s"]}
            },
            "doubts": []
        });
        format!(
            "task_proof_evidence:\n  proof_target: zombie_d_matrix\n  result: pass\n  evidence: {}",
            matrix
        )
    }

    #[test]
    fn default_enabled_applicable_task_blocks_without_matrix() {
        let task = task(None, &[], &["crates/vida/tests/runtime.rs"]);
        let result = evaluate_from_readiness(&readiness(true), &task, Some("implementation"), "dispatch");
        assert_eq!(result["status"], "blocked");
        assert!(result["blocker_codes"]
            .as_array()
            .is_some_and(|codes| codes.iter().any(|code| code == "zombie_d_matrix_missing")));
    }

    #[test]
    fn structured_matrix_passes_all_categories() {
        let notes = matrix_note();
        let task = task(Some(&notes), &[], &["crates/vida/tests/runtime.rs"]);
        let result = evaluate_from_readiness(&readiness(true), &task, Some("implementation"), "handoff");
        assert_eq!(result["status"], "pass");
    }

    #[test]
    fn scalarized_yaml_matrix_passes_after_cli_transport() {
        let notes = "task_proof_evidence:\n  proof_target: zombie_d_matrix\n  result: pass\n  evidence: {schema_version: 1, categories: {Z: {status: pass, evidence_refs: [z]}, O: {status: pass, evidence_refs: [o]}, M: {status: na, reason: single}, B: {status: pass, evidence_refs: [b]}, I: {status: pass, evidence_refs: [i]}, E: {status: pass, evidence_refs: [e]}, S: {status: pass, evidence_refs: [s]}}, doubts: []}";
        let task = task(Some(notes), &[], &["crates/vida/tests/runtime.rs"]);
        let result = evaluate_from_readiness(&readiness(true), &task, Some("implementation"), "closure");
        assert_eq!(result["status"], "pass");
    }

    #[test]
    fn disabled_gate_does_not_block_applicable_task() {
        let task = task(None, &[], &["crates/vida/tests/runtime.rs"]);
        let result = evaluate_from_readiness(&readiness(false), &task, Some("implementation"), "dispatch");
        assert_eq!(result["status"], "disabled");
        assert!(result["blocker_codes"].as_array().is_some_and(|codes| codes.is_empty()));
    }

    #[test]
    fn omitted_config_uses_enabled_default_and_explicit_false_disables_gate() {
        let omitted: serde_yaml::Value = serde_yaml::from_str("{}").expect("empty YAML");
        let mut omitted_blockers = Vec::new();
        let omitted_projection = project_policy(&omitted, &mut omitted_blockers);
        assert_eq!(omitted_projection["enabled"], true);
        assert_eq!(omitted_projection["default_enabled"], true);
        assert_eq!(omitted_projection["configured"], false);
        assert!(omitted_blockers.is_empty());

        let disabled: serde_yaml::Value =
            serde_yaml::from_str("zombie_d_gate:\n  enabled: false\n").expect("disabled YAML");
        let mut disabled_blockers = Vec::new();
        let disabled_projection = project_policy(&disabled, &mut disabled_blockers);
        assert_eq!(disabled_projection["enabled"], false);
        assert_eq!(disabled_projection["configured"], true);
        assert!(disabled_blockers.is_empty());
    }
}
