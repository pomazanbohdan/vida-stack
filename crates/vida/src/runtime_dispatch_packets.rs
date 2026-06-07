use crate::runtime_contract_vocab::{
    RUNTIME_ROLE_COACH, RUNTIME_ROLE_SOLUTION_ARCHITECT, RUNTIME_ROLE_VERIFIER,
    TASK_CLASS_ARCHITECTURE, TASK_CLASS_COACH, TASK_CLASS_IMPLEMENTATION, TASK_CLASS_SPECIFICATION,
    TASK_CLASS_VERIFICATION,
};

pub(crate) const RUNTIME_CONSUMPTION_FALLBACK_OWNED_PATH: &str =
    ".vida/data/state/runtime-consumption";

pub(crate) fn is_runtime_consumption_fallback_owned_path(path: &str) -> bool {
    path == RUNTIME_CONSUMPTION_FALLBACK_OWNED_PATH
}

fn runtime_delivery_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::delivery")
}

fn runtime_execution_block_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::execution-block")
}

fn runtime_coach_review_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::coach-review")
}

fn runtime_verifier_proof_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::verifier-proof")
}

fn runtime_escalation_packet_id(run_id: &str, dispatch_target: &str) -> String {
    format!("{run_id}::{dispatch_target}::escalation")
}

fn runtime_delivery_source_packet_id(run_id: &str, dispatch_target: &str) -> String {
    runtime_delivery_packet_id(run_id, dispatch_target)
}

fn runtime_review_source_target<'a>(
    dispatch_target: &'a str,
    source_dispatch_target: Option<&'a str>,
) -> &'a str {
    let source_dispatch_target = source_dispatch_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| *value != dispatch_target);
    if let Some(source_dispatch_target) = source_dispatch_target {
        return source_dispatch_target;
    }
    if matches!(dispatch_target, "coach" | "review") || dispatch_target.contains("coach") {
        "implementer"
    } else if matches!(dispatch_target, "verification" | "verifier" | "prover")
        || dispatch_target.contains("verification")
    {
        "coach"
    } else {
        dispatch_target
    }
}

fn trim_owned_scope_path_candidate(segment: &str) -> String {
    segment
        .trim()
        .trim_matches(|ch: char| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    '`' | '"' | '\'' | ',' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}'
                )
        })
        .trim_end_matches('.')
        .trim_end_matches('/')
        .to_string()
}

pub(crate) fn normalize_safe_owned_scope_path_candidate(candidate: &str) -> Option<String> {
    let normalized = trim_owned_scope_path_candidate(candidate);
    if normalized.is_empty()
        || !normalized.contains('/')
        || !normalized.contains('.')
        || normalized.starts_with('/')
        || normalized.starts_with("./")
        || normalized.starts_with("../")
    {
        return None;
    }
    Some(normalized)
}

fn trim_move_request_scope_path(segment: &str) -> String {
    segment
        .trim()
        .trim_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, '`' | '"' | '\'' | ',' | ';' | ':')
        })
        .trim_end_matches('.')
        .to_string()
}

pub(crate) fn single_task_move_scope_paths(request_text: &str) -> Option<Vec<String>> {
    let lowered = request_text.to_ascii_lowercase();
    let move_start = lowered.find("move ")?;
    let from_start = lowered[move_start + "move ".len()..]
        .find(" from ")
        .map(|offset| move_start + "move ".len() + offset)?;
    let source_start = from_start + " from ".len();
    let into_start = lowered[source_start..]
        .find(" into ")
        .map(|offset| source_start + offset)?;
    let source_path = trim_move_request_scope_path(&request_text[source_start..into_start]);
    if source_path.is_empty() {
        return None;
    }

    let destination_start = into_start + " into ".len();
    let destination_segment = &request_text[destination_start..];
    let destination_lowered = &lowered[destination_start..];
    let mut destination_end = destination_segment.len();
    for marker in [
        "\n",
        ";",
        " keep scope",
        " keep ",
        " proof target",
        " proof ",
        " after ",
    ] {
        if let Some(offset) = destination_lowered.find(marker) {
            destination_end = destination_end.min(offset);
        }
    }
    let destination_path = trim_move_request_scope_path(&destination_segment[..destination_end]);
    if destination_path.is_empty() {
        return None;
    }

    Some(vec![source_path, destination_path])
}

pub(crate) fn explicit_request_scope_paths(request_text: &str) -> Vec<String> {
    let mut owned_paths = Vec::new();
    let mut push_path = |candidate: &str| {
        let Some(normalized) = normalize_safe_owned_scope_path_candidate(candidate) else {
            return;
        };
        if !owned_paths.iter().any(|existing| existing == &normalized) {
            owned_paths.push(normalized);
        }
    };

    if let Some(paths) = single_task_move_scope_paths(request_text) {
        for path in paths {
            push_path(&path);
        }
    }

    for token in request_text.split_whitespace() {
        push_path(token);
    }

    owned_paths
}

pub(crate) fn request_has_explicit_owned_scope(request_text: &str) -> bool {
    !explicit_request_scope_paths(request_text).is_empty()
}

pub(crate) fn tracked_design_doc_owned_paths(tracked_design_doc_path: Option<&str>) -> Vec<String> {
    let mut owned_paths = Vec::new();
    if let Some(path) = tracked_design_doc_path {
        let normalized = trim_owned_scope_path_candidate(path);
        if !normalized.is_empty()
            && normalized.contains('/')
            && normalized.contains('.')
            && !normalized.starts_with('/')
            && !normalized.starts_with("./")
            && !normalized.starts_with("../")
        {
            owned_paths.push(normalized);
        }
    }
    owned_paths
}

pub(crate) fn tracked_design_doc_bounded_file_set_paths(
    tracked_design_doc_path: Option<&str>,
) -> Vec<String> {
    const MAX_TRACKED_DESIGN_DOC_BYTES: u64 = 1_048_576;
    let Some(path) = tracked_design_doc_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };
    let Ok(metadata) = std::fs::metadata(path) else {
        return Vec::new();
    };
    if !metadata.is_file() || metadata.len() > MAX_TRACKED_DESIGN_DOC_BYTES {
        return Vec::new();
    }
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut in_bounded_file_set = false;
    let mut owned_paths = Vec::new();
    let mut push_path = |candidate: &str| {
        let Some(normalized) = normalize_safe_owned_scope_path_candidate(candidate) else {
            return;
        };
        if !owned_paths.iter().any(|existing| existing == &normalized) {
            owned_paths.push(normalized);
        }
    };

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            in_bounded_file_set = trimmed
                .trim_start_matches('#')
                .trim()
                .eq_ignore_ascii_case("Bounded File Set");
            continue;
        }
        if !in_bounded_file_set {
            continue;
        }
        let mut rest = trimmed;
        while let Some(start) = rest.find('`') {
            let after_start = &rest[start + 1..];
            let Some(end) = after_start.find('`') else {
                break;
            };
            push_path(&after_start[..end]);
            rest = &after_start[end + 1..];
        }
    }

    owned_paths
}

pub(crate) fn delivery_packet_owned_paths(
    handoff_task_class: &str,
    request_text: &str,
    tracked_design_doc_path: Option<&str>,
) -> Vec<String> {
    if delivery_packet_task_class_requires_owned_paths(handoff_task_class) {
        let explicit_paths = explicit_request_scope_paths(request_text);
        if !explicit_paths.is_empty() {
            explicit_paths
        } else {
            let design_paths = tracked_design_doc_bounded_file_set_paths(tracked_design_doc_path);
            if design_paths.is_empty() {
                vec![RUNTIME_CONSUMPTION_FALLBACK_OWNED_PATH.to_string()]
            } else {
                design_paths
            }
        }
    } else {
        match handoff_task_class {
            TASK_CLASS_SPECIFICATION => tracked_design_doc_owned_paths(tracked_design_doc_path),
            _ => Vec::new(),
        }
    }
}

pub(crate) fn delivery_packet_task_class_requires_owned_paths(handoff_task_class: &str) -> bool {
    matches!(
        handoff_task_class,
        TASK_CLASS_IMPLEMENTATION
            | "implementation_medium"
            | "test_authoring"
            | "regression_test"
            | "delivery_task"
    )
}

pub(crate) fn delivery_packet_task_class_requires_implementation_isolation(
    handoff_task_class: &str,
) -> bool {
    matches!(
        handoff_task_class,
        TASK_CLASS_IMPLEMENTATION | "implementation_medium" | "delivery_task"
    )
}

pub(crate) fn implementation_isolation_contract(
    handoff_task_class: &str,
    owned_paths: &[String],
) -> serde_json::Value {
    if !delivery_packet_task_class_requires_implementation_isolation(handoff_task_class) {
        return serde_json::Value::Null;
    }
    serde_json::json!({
        "schema_version": "implementation-isolation-v1",
        "canonical_worktree_writes_allowed": false,
        "default_mode": "patch_proposal",
        "allowed_modes": ["patch_proposal", "isolated_worktree"],
        "artifact_contract": "stage_attempt_implementation_artifact_v1",
        "owned_paths": owned_paths,
        "required_result_fields": [
            "artifact_kind",
            "attempt_id",
            "task_id",
            "stage_id",
            "changed_files"
        ],
        "scope_policy": {
            "changed_files_must_be_subset_of_owned_paths": true,
            "patch_paths_must_be_subset_of_owned_paths": true
        }
    })
}

pub(crate) struct ImplementationArtifactAuthority<'a> {
    pub task_id: &'a str,
    pub task_updated_at: &'a str,
}

pub(crate) fn implementation_artifact_scope_validation(
    owned_paths: &[String],
    artifacts: &serde_json::Value,
    authority: ImplementationArtifactAuthority<'_>,
) -> serde_json::Value {
    const IMPLEMENTATION_STAGE_ID: &str = "implementation";
    let normalized_owned_paths = owned_paths
        .iter()
        .filter_map(|path| normalize_scope_path_for_compare(path))
        .collect::<Vec<_>>();
    let mut blocker_codes = Vec::new();
    let mut reported_changed_files = Vec::new();
    let mut out_of_scope_paths = Vec::new();
    let mut contract_invalid = false;
    let mut authority_invalid = false;
    let mut receipt_missing = false;

    let Some(rows) = artifacts.as_array() else {
        return serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["implementation_artifact_contract_invalid"],
            "owned_paths": normalized_owned_paths,
            "reported_changed_files": [],
            "out_of_scope_paths": []
        });
    };

    if rows.is_empty() {
        blocker_codes.push("implementation_artifacts_missing".to_string());
    }

    for artifact in rows {
        let Some(object) = artifact.as_object() else {
            contract_invalid = true;
            continue;
        };
        let artifact_kind = object
            .get("artifact_kind")
            .and_then(serde_json::Value::as_str)
            .map(str::trim);
        if !matches!(
            artifact_kind,
            Some("patch_proposal") | Some("isolated_worktree_manifest")
        ) {
            contract_invalid = true;
        }
        for field in ["attempt_id", "task_id", "stage_id", "changed_files"] {
            if !object.contains_key(field) {
                contract_invalid = true;
            }
        }
        let artifact_task_id = object
            .get("task_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim);
        let artifact_stage_id = object
            .get("stage_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim);
        let artifact_freshness = object
            .get("freshness")
            .and_then(serde_json::Value::as_str)
            .map(str::trim);
        if artifact_task_id != Some(authority.task_id)
            || artifact_stage_id != Some(IMPLEMENTATION_STAGE_ID)
            || artifact_freshness != Some(authority.task_updated_at)
        {
            authority_invalid = true;
        }
        let receipt_backed = object
            .get("receipt_backed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let consolidation_receipt_id = object
            .get("consolidation_receipt_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if !receipt_backed || consolidation_receipt_id.is_none() {
            receipt_missing = true;
        }
        collect_implementation_changed_files(artifact, &mut reported_changed_files);
    }

    reported_changed_files.sort();
    reported_changed_files.dedup();
    for changed_file in &reported_changed_files {
        match normalize_scope_path_for_compare(changed_file) {
            Some(path) if path_is_in_owned_scope(&path, &normalized_owned_paths) => {}
            Some(path) => out_of_scope_paths.push(path),
            None => out_of_scope_paths.push(changed_file.clone()),
        }
    }
    out_of_scope_paths.sort();
    out_of_scope_paths.dedup();

    if contract_invalid {
        blocker_codes.push("implementation_artifact_contract_invalid".to_string());
    }
    if authority_invalid {
        blocker_codes.push("implementation_artifact_authority_invalid".to_string());
    }
    if receipt_missing {
        blocker_codes.push("implementation_artifact_receipt_missing".to_string());
    }
    if !rows.is_empty() && reported_changed_files.is_empty() {
        blocker_codes.push("implementation_artifact_changed_files_missing".to_string());
    }
    if !out_of_scope_paths.is_empty() {
        blocker_codes.push("implementation_attempt_scope_guard_violation".to_string());
    }
    blocker_codes.sort();
    blocker_codes.dedup();

    serde_json::json!({
        "status": if blocker_codes.is_empty() { "pass" } else { "blocked" },
        "blocker_codes": blocker_codes,
        "owned_paths": normalized_owned_paths,
        "reported_changed_files": reported_changed_files,
        "out_of_scope_paths": out_of_scope_paths
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TaskflowImplementationArtifacts {
    pub artifacts: Vec<serde_json::Value>,
    pub artifact_refs: Vec<String>,
    pub authority_keys: Vec<TaskflowImplementationArtifactAuthority>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskflowImplementationArtifactAuthority {
    pub attempt_id: String,
    pub task_id: String,
    pub stage_id: String,
    pub freshness: String,
    pub consolidation_receipt_id: String,
}

pub(crate) fn taskflow_attempt_implementation_artifacts(
    attempts: &[crate::state_store::TaskAttemptRecord],
    task_updated_at: &str,
) -> TaskflowImplementationArtifacts {
    const IMPLEMENTATION_STAGE_ID: &str = "implementation";
    let mut collected = TaskflowImplementationArtifacts::default();
    for attempt in attempts.iter().filter(|attempt| {
        attempt.stage_id.trim() == IMPLEMENTATION_STAGE_ID
            && attempt.freshness == task_updated_at
            && attempt
                .consolidation_receipt_id
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            && taskflow_attempt_status_can_supply_implementation_artifact(&attempt.status)
    }) {
        for artifact_ref in &attempt.artifact_refs {
            let path = std::path::Path::new(artifact_ref);
            if !path.exists() {
                continue;
            }
            let Ok(raw) = std::fs::read_to_string(path) else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            if let Some(artifact) =
                normalize_taskflow_attempt_implementation_artifact(attempt, artifact_ref, &json)
            {
                push_unique_string(&mut collected.artifact_refs, artifact_ref);
                if let Some(authority_key) = taskflow_attempt_implementation_authority(attempt) {
                    push_unique_implementation_authority(
                        &mut collected.authority_keys,
                        authority_key,
                    );
                }
                collected.artifacts.push(artifact);
            }
        }
    }
    collected
}

fn taskflow_attempt_implementation_authority(
    attempt: &crate::state_store::TaskAttemptRecord,
) -> Option<TaskflowImplementationArtifactAuthority> {
    Some(TaskflowImplementationArtifactAuthority {
        attempt_id: attempt.attempt_id.clone(),
        task_id: attempt.task_id.clone(),
        stage_id: attempt.stage_id.clone(),
        freshness: attempt.freshness.clone(),
        consolidation_receipt_id: attempt
            .consolidation_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())?
            .to_string(),
    })
}

fn taskflow_attempt_status_can_supply_implementation_artifact(status: &str) -> bool {
    matches!(
        status.trim(),
        "accepted" | "partially_accepted" | "consumed"
    )
}

fn normalize_taskflow_attempt_implementation_artifact(
    attempt: &crate::state_store::TaskAttemptRecord,
    artifact_ref: &str,
    json: &serde_json::Value,
) -> Option<serde_json::Value> {
    let changed_files = json
        .get("changed_files")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if changed_files.is_empty() {
        return None;
    }
    if json_identity_conflicts_attempt(json, attempt) {
        return None;
    }
    let consolidation_receipt_id = attempt
        .consolidation_receipt_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let artifact_kind = json
        .get("artifact_kind")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| {
            matches!(
                *value,
                "patch_proposal" | "isolated_worktree_manifest" | "task_handoff_accept_receipt"
            )
        })
        .unwrap_or("patch_proposal");
    let normalized_kind = if artifact_kind == "task_handoff_accept_receipt" {
        "patch_proposal"
    } else {
        artifact_kind
    };
    Some(serde_json::json!({
        "artifact_kind": normalized_kind,
        "schema_version": "stage-attempt-implementation-artifact-v1",
        "attempt_id": attempt.attempt_id,
        "task_id": attempt.task_id,
        "stage_id": attempt.stage_id,
        "freshness": attempt.freshness,
        "consolidation_receipt_id": consolidation_receipt_id,
        "changed_files": changed_files,
        "source_artifact_ref": artifact_ref,
        "source_artifact_kind": artifact_kind,
        "receipt_backed": true
    }))
}

fn json_identity_conflicts_attempt(
    json: &serde_json::Value,
    attempt: &crate::state_store::TaskAttemptRecord,
) -> bool {
    [
        ("attempt_id", attempt.attempt_id.as_str()),
        ("task_id", attempt.task_id.as_str()),
        ("stage_id", attempt.stage_id.as_str()),
    ]
    .into_iter()
    .any(|(field, expected)| {
        json.get(field)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|actual| !actual.is_empty())
            .is_some_and(|actual| actual != expected)
    })
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn push_unique_implementation_authority(
    values: &mut Vec<TaskflowImplementationArtifactAuthority>,
    value: TaskflowImplementationArtifactAuthority,
) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn collect_implementation_changed_files(value: &serde_json::Value, files: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(changed_files) = object.get("changed_files") {
                collect_scope_path_values(changed_files, files);
            }
            for nested in object.values() {
                collect_implementation_changed_files(nested, files);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_implementation_changed_files(nested, files);
            }
        }
        _ => {}
    }
}

fn collect_scope_path_values(value: &serde_json::Value, files: &mut Vec<String>) {
    match value {
        serde_json::Value::String(path) => {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                files.push(trimmed.to_string());
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_scope_path_values(value, files);
            }
        }
        serde_json::Value::Object(object) => {
            for key in ["path", "file", "filename"] {
                if let Some(path) = object.get(key).and_then(serde_json::Value::as_str) {
                    let trimmed = path.trim();
                    if !trimmed.is_empty() {
                        files.push(trimmed.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

fn normalize_scope_path_for_compare(path: &str) -> Option<String> {
    let mut normalized = path.trim().replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains(':')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return None;
    }
    Some(normalized)
}

fn path_is_in_owned_scope(path: &str, owned_paths: &[String]) -> bool {
    owned_paths
        .iter()
        .any(|owned_path| path == owned_path || path.starts_with(&format!("{owned_path}/")))
}

#[cfg(test)]
pub(crate) fn runtime_delivery_task_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
    request_text: &str,
) -> serde_json::Value {
    runtime_delivery_task_packet_with_scope_context(
        run_id,
        dispatch_target,
        handoff_runtime_role,
        handoff_task_class,
        closure_class,
        request_text,
        None,
    )
}

pub(crate) fn runtime_delivery_task_packet_with_scope_context(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
    request_text: &str,
    tracked_design_doc_path: Option<&str>,
) -> serde_json::Value {
    let owned_paths =
        delivery_packet_owned_paths(handoff_task_class, request_text, tracked_design_doc_path);
    let implementation_isolation =
        implementation_isolation_contract(handoff_task_class, &owned_paths);
    serde_json::json!({
        "packet_id": runtime_delivery_packet_id(run_id, dispatch_target),
        "backlog_id": run_id,
        "release_slice": "none",
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Execute bounded `{dispatch_target}` handoff for the active runtime request"),
        "non_goals": [
            "unbounded repository-wide rewrites",
            "out-of-scope taskflow state mutation"
        ],
        "scope_in": [
            format!("dispatch_target:{dispatch_target}"),
            format!("runtime_role:{handoff_runtime_role}")
        ],
        "scope_out": [
            "mutation outside bounded packet scope",
            "closure without recorded handoff evidence"
        ],
        "owned_paths": owned_paths,
        "implementation_isolation": implementation_isolation,
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "inputs": [
            "role_selection_full",
            "run_graph_bootstrap",
            "taskflow_handoff_plan"
        ],
        "outputs": [
            "dispatch_result_artifact",
            "updated_run_graph_dispatch_receipt"
        ],
        "definition_of_done": [
            format!("`{dispatch_target}` handoff produces a bounded runtime result artifact"),
            "dispatch receipt and downstream preview are refreshed consistently"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime dispatch result artifact plus updated dispatch receipt",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop after writing bounded dispatch result or explicit blocker",
            "do not widen scope beyond the active packet target"
        ],
        "blocking_question": format!("What is the next bounded action required for `{dispatch_target}`?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier",
        "request_excerpt": request_text.chars().take(240).collect::<String>(),
    })
}

pub(crate) fn runtime_execution_block_packet(
    run_id: &str,
    dispatch_target: &str,
    handoff_runtime_role: &str,
    handoff_task_class: &str,
    closure_class: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_execution_block_packet_id(run_id, dispatch_target),
        "parent_packet_id": runtime_delivery_packet_id(run_id, dispatch_target),
        "backlog_id": run_id,
        "owner": "taskflow",
        "closure_class": closure_class,
        "goal": format!("Resolve bounded execution blocker for `{dispatch_target}`"),
        "scope_in": [
            format!("dispatch_target:{dispatch_target}")
        ],
        "scope_out": [
            "new feature scope without bounded packet update"
        ],
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "definition_of_done": [
            "bounded blocker is resolved with receipt-backed evidence"
        ],
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": "runtime receipt evidence that blocker is resolved or escalated",
        "active_skills": "no_applicable_skill",
        "stop_rules": [
            "stop once blocker resolution evidence is recorded"
        ],
        "blocking_question": format!("Which explicit blocker prevents closing `{dispatch_target}` now?"),
        "handoff_runtime_role": handoff_runtime_role,
        "handoff_task_class": handoff_task_class,
        "handoff_selection": "runtime_selected_tier"
    })
}

pub(crate) fn runtime_coach_review_packet(
    run_id: &str,
    dispatch_target: &str,
    source_dispatch_target: Option<&str>,
    proof_target: &str,
) -> serde_json::Value {
    let reviewed_dispatch_target =
        runtime_review_source_target(dispatch_target, source_dispatch_target);
    serde_json::json!({
        "packet_id": runtime_coach_review_packet_id(run_id, dispatch_target),
        "source_packet_id": runtime_delivery_source_packet_id(run_id, reviewed_dispatch_target),
        "reviewed_dispatch_target": reviewed_dispatch_target,
        "review_subject": format!("bounded `{reviewed_dispatch_target}` delivery/result"),
        "review_goal": format!("Judge whether bounded `{reviewed_dispatch_target}` delivery/result remains aligned with the approved packet, acceptance criteria, and definition of done"),
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "definition_of_done": [
            "coach review returns bounded approval-forward or bounded rework evidence"
        ],
        "proof_target": proof_target,
        "active_skills": "no_applicable_skill",
        "review_focus": [
            "spec_conformance",
            "acceptance_criteria_alignment",
            "bounded_scope_drift"
        ],
        "expected_output": [
            "decision=approve|rework|blocker",
            "checked_evidence",
            "findings",
            "risks",
            "next_required_action"
        ],
        "blocking_question": format!("Does bounded `{reviewed_dispatch_target}` delivery/result match the approved bounded contract cleanly enough to proceed?"),
        "handoff_runtime_role": RUNTIME_ROLE_COACH,
        "handoff_task_class": TASK_CLASS_COACH,
        "handoff_selection": "runtime_selected_tier",
    })
}

pub(crate) fn runtime_verifier_proof_packet(
    run_id: &str,
    dispatch_target: &str,
    proof_target: &str,
) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_verifier_proof_packet_id(run_id, dispatch_target),
        "source_packet_id": runtime_delivery_source_packet_id(run_id, "coach"),
        "proof_goal": format!("Independently verify bounded closure readiness for `{dispatch_target}`"),
        "verification_command": format!("vida taskflow consume continue --run-id {run_id} --json"),
        "proof_target": proof_target,
        "owned_paths": [],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("What proof is still missing before `{dispatch_target}` can close?"),
        "handoff_runtime_role": RUNTIME_ROLE_VERIFIER,
        "handoff_task_class": TASK_CLASS_VERIFICATION,
        "handoff_selection": "runtime_selected_tier",
    })
}

pub(crate) fn runtime_escalation_packet(run_id: &str, dispatch_target: &str) -> serde_json::Value {
    serde_json::json!({
        "packet_id": runtime_escalation_packet_id(run_id, dispatch_target),
        "source_packet_id": runtime_delivery_source_packet_id(run_id, dispatch_target),
        "conflict_type": "architecture",
        "decision_needed": format!("Resolve the bounded architecture-preparation or escalation decision for `{dispatch_target}`"),
        "options": [
            "approve current bounded route",
            "reshape bounded handoff",
            "block execution pending architectural clarification"
        ],
        "constraints": [
            "preserve one bounded packet owner",
            "do not widen scope without a new bounded packet"
        ],
        "read_only_paths": [
            ".vida/data/state/runtime-consumption",
            "docs/product/spec",
            "docs/process"
        ],
        "active_skills": "no_applicable_skill",
        "blocking_question": format!("Which architectural decision is required before `{dispatch_target}` can proceed coherently?"),
        "handoff_runtime_role": RUNTIME_ROLE_SOLUTION_ARCHITECT,
        "handoff_task_class": TASK_CLASS_ARCHITECTURE,
        "handoff_selection": "runtime_selected_tier",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        delivery_packet_owned_paths, implementation_artifact_scope_validation,
        runtime_coach_review_packet, tracked_design_doc_owned_paths,
        ImplementationArtifactAuthority, RUNTIME_CONSUMPTION_FALLBACK_OWNED_PATH,
    };

    #[test]
    fn tracked_design_doc_owned_paths_rejects_absolute_and_traversal_paths() {
        assert!(tracked_design_doc_owned_paths(Some("/tmp/evil.md")).is_empty());
        assert!(tracked_design_doc_owned_paths(Some("../outside.md")).is_empty());
        assert!(tracked_design_doc_owned_paths(Some("./local.md")).is_empty());
    }

    #[test]
    fn tracked_design_doc_owned_paths_accepts_project_relative_doc_path() {
        assert_eq!(
            tracked_design_doc_owned_paths(Some(" docs/product/spec/example.md ")),
            vec!["docs/product/spec/example.md".to_string()]
        );
    }

    #[test]
    fn test_authoring_delivery_packet_receives_owned_paths() {
        assert_eq!(
            delivery_packet_owned_paths("test_authoring", "write the regression test", None),
            vec![RUNTIME_CONSUMPTION_FALLBACK_OWNED_PATH.to_string()]
        );
        assert_eq!(
            delivery_packet_owned_paths(
                "test_authoring",
                "write regression in crates/vida/src/runtime_dispatch_packets.rs",
                None,
            ),
            vec!["crates/vida/src/runtime_dispatch_packets.rs".to_string()]
        );
    }

    #[test]
    fn coach_review_packet_reviews_source_delivery_not_coach_itself() {
        let packet = runtime_coach_review_packet(
            "run-1",
            "coach",
            Some("implementer"),
            "bounded implementation result versus approved spec",
        );

        assert_eq!(packet["reviewed_dispatch_target"], "implementer");
        assert_eq!(packet["source_packet_id"], "run-1::implementer::delivery");
        assert!(packet["review_goal"]
            .as_str()
            .expect("review goal")
            .contains("bounded `implementer` delivery/result"));
        assert!(packet["blocking_question"]
            .as_str()
            .expect("blocking question")
            .contains("bounded `implementer` delivery/result"));
        assert!(packet["expected_output"]
            .as_array()
            .expect("expected output")
            .iter()
            .any(|value| value.as_str() == Some("decision=approve|rework|blocker")));
    }

    fn implementation_artifact(
        task_id: &str,
        stage_id: &str,
        freshness: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "artifact_kind": "patch_proposal",
            "attempt_id": "attempt-1",
            "task_id": task_id,
            "stage_id": stage_id,
            "freshness": freshness,
            "consolidation_receipt_id": "receipt-1",
            "receipt_backed": true,
            "changed_files": ["crates/vida/src/lib.rs"]
        })
    }

    fn implementation_authority() -> ImplementationArtifactAuthority<'static> {
        ImplementationArtifactAuthority {
            task_id: "task-1",
            task_updated_at: "task-updated-at-1",
        }
    }

    #[test]
    fn implementation_artifact_validation_accepts_current_receipt_backed_implementation_artifact() {
        let validation = implementation_artifact_scope_validation(
            &["crates/vida/src/lib.rs".to_string()],
            &serde_json::json!([implementation_artifact(
                "task-1",
                "implementation",
                "task-updated-at-1"
            )]),
            implementation_authority(),
        );

        assert_eq!(validation["status"], "pass");
        assert_eq!(validation["blocker_codes"], serde_json::json!([]));
    }

    #[test]
    fn implementation_artifact_validation_rejects_wrong_task_stage_or_stale_freshness() {
        for artifact in [
            implementation_artifact("other-task", "implementation", "task-updated-at-1"),
            implementation_artifact("task-1", "coach", "task-updated-at-1"),
            implementation_artifact("task-1", "implementation", "old-task-updated-at"),
        ] {
            let validation = implementation_artifact_scope_validation(
                &["crates/vida/src/lib.rs".to_string()],
                &serde_json::json!([artifact]),
                implementation_authority(),
            );

            assert_eq!(validation["status"], "blocked");
            assert!(validation["blocker_codes"]
                .as_array()
                .expect("blocker codes")
                .iter()
                .any(|code| code == "implementation_artifact_authority_invalid"));
        }
    }

    #[test]
    fn implementation_artifact_validation_rejects_receiptless_artifacts() {
        let mut artifact = implementation_artifact("task-1", "implementation", "task-updated-at-1");
        artifact
            .as_object_mut()
            .expect("artifact object")
            .remove("consolidation_receipt_id");
        let validation = implementation_artifact_scope_validation(
            &["crates/vida/src/lib.rs".to_string()],
            &serde_json::json!([artifact]),
            implementation_authority(),
        );

        assert_eq!(validation["status"], "blocked");
        assert!(validation["blocker_codes"]
            .as_array()
            .expect("blocker codes")
            .iter()
            .any(|code| code == "implementation_artifact_receipt_missing"));
    }
}
