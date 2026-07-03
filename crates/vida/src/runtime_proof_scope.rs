use std::path::Path;

const EXPLICIT_PROOF_PATH_FIELDS: &[&str] = &[
    "proof_artifact_paths",
    "proof_artifact_scope",
    "proof_scope",
    "test_owned_paths",
    "proof_owned_paths",
    "verification_artifact_paths",
];

const TEXT_PROOF_FIELDS: &[&str] = &[
    "proof_targets",
    "proof_target",
    "verification_commands",
    "acceptance_targets",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProofArtifactScope {
    pub paths: Vec<String>,
    pub proof_intent_present: bool,
}

impl ProofArtifactScope {
    pub(crate) fn merge(&mut self, mut other: ProofArtifactScope) {
        self.proof_intent_present |= other.proof_intent_present;
        self.paths.append(&mut other.paths);
        normalize_scope_paths(&mut self.paths);
    }
}

pub(crate) fn proof_token_looks_like_artifact_path(value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    normalized.contains("/test/")
        || normalized.contains("/tests/")
        || normalized.starts_with("test/")
        || normalized.starts_with("tests/")
        || normalized.ends_with("_test.rs")
        || normalized.ends_with("_test.dart")
        || normalized.ends_with(".test.ts")
        || normalized.ends_with(".test.tsx")
        || normalized.ends_with(".spec.ts")
        || normalized.ends_with(".spec.tsx")
}

pub(crate) fn proof_intent_text(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("proof")
        || normalized.contains("test")
        || normalized.contains("verification")
        || normalized.contains("verify")
        || normalized.contains("regression")
}

pub(crate) fn collect_test_like_paths_from_text(paths: &mut Vec<String>, value: &str) {
    for token in value_path_tokens(value) {
        if proof_token_looks_like_artifact_path(token) {
            push_unique_proof_artifact_path(paths, token);
        }
    }
}

pub(crate) fn collect_test_like_paths_from_values<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let mut paths = Vec::new();
    for value in values {
        collect_test_like_paths_from_text(&mut paths, value);
    }
    normalize_scope_paths(&mut paths);
    paths
}

pub(crate) fn proof_scope_from_container(value: &serde_json::Value) -> ProofArtifactScope {
    let mut scope = ProofArtifactScope::default();
    collect_proof_scope_from_container(&mut scope, value);
    normalize_scope_paths(&mut scope.paths);
    scope
}

pub(crate) fn proof_scope_from_dispatch_packet(packet: &serde_json::Value) -> ProofArtifactScope {
    let mut scope = proof_scope_from_container(packet);
    for field in [
        "delivery_task_packet",
        "execution_block_packet",
        "coach_review_packet",
        "verifier_proof_packet",
        "tracked_flow_packet",
    ] {
        if let Some(container) = packet.get(field) {
            scope.merge(proof_scope_from_container(container));
        }
    }
    scope
}

pub(crate) fn proof_scope_from_dispatch_packet_path(path: &str) -> ProofArtifactScope {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return ProofArtifactScope::default();
    };
    let Ok(packet) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return ProofArtifactScope::default();
    };
    proof_scope_from_dispatch_packet(&packet)
}

pub(crate) fn proof_scope_from_planner_metadata_and_text(
    execution_plan: &serde_json::Value,
    request_text: &str,
) -> ProofArtifactScope {
    let mut scope = ProofArtifactScope::default();
    for field in EXPLICIT_PROOF_PATH_FIELDS {
        if let Some(value) =
            execution_plan.pointer(&format!("/tracked_flow_bootstrap/dev_task/planner_metadata/{field}"))
        {
            collect_explicit_proof_paths(&mut scope.paths, value);
        }
    }
    for field in TEXT_PROOF_FIELDS {
        if let Some(value) =
            execution_plan.pointer(&format!("/tracked_flow_bootstrap/dev_task/planner_metadata/{field}"))
        {
            scope.proof_intent_present = true;
            collect_test_like_proof_paths_from_value(&mut scope.paths, value);
        }
    }
    if proof_intent_text(request_text) {
        scope.proof_intent_present = true;
    }
    collect_test_like_paths_from_text(&mut scope.paths, request_text);
    normalize_scope_paths(&mut scope.paths);
    scope
}

fn collect_proof_scope_from_container(scope: &mut ProofArtifactScope, value: &serde_json::Value) {
    let serde_json::Value::Object(map) = value else {
        return;
    };
    for (key, value) in map {
        if EXPLICIT_PROOF_PATH_FIELDS.contains(&key.as_str()) {
            scope.proof_intent_present = true;
            collect_explicit_proof_paths(&mut scope.paths, value);
        } else if TEXT_PROOF_FIELDS.contains(&key.as_str()) {
            scope.proof_intent_present = true;
            collect_test_like_proof_paths_from_value(&mut scope.paths, value);
        }
    }
}

fn collect_explicit_proof_paths(paths: &mut Vec<String>, value: &serde_json::Value) {
    match value {
        serde_json::Value::String(value) => push_unique_proof_artifact_path(paths, value),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_explicit_proof_paths(paths, value);
            }
        }
        serde_json::Value::Object(map) => {
            for value in map.values() {
                collect_explicit_proof_paths(paths, value);
            }
        }
        _ => {}
    }
}

fn collect_test_like_proof_paths_from_value(paths: &mut Vec<String>, value: &serde_json::Value) {
    match value {
        serde_json::Value::String(value) => collect_test_like_paths_from_text(paths, value),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_test_like_proof_paths_from_value(paths, value);
            }
        }
        serde_json::Value::Object(map) => {
            for value in map.values() {
                collect_test_like_proof_paths_from_value(paths, value);
            }
        }
        _ => {}
    }
}

fn push_unique_proof_artifact_path(paths: &mut Vec<String>, value: &str) {
    let normalized = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '\'' | '"' | ':' | ')' | '(' | '[' | ']'))
        .replace('\\', "/");
    if normalized.is_empty() || !(normalized.contains('/') || normalized.contains('\\')) {
        return;
    }
    if !paths.iter().any(|path| path == &normalized) {
        paths.push(normalized);
    }
}

fn value_path_tokens(value: &str) -> impl Iterator<Item = &str> {
    value.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '`'))
}

fn normalize_scope_paths(paths: &mut Vec<String>) {
    for path in paths.iter_mut() {
        *path = path.replace('\\', "/");
    }
    paths.sort();
    paths.dedup();
}

pub(crate) fn path_to_proof_scope_string(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_scope_detects_intent_without_concrete_paths() {
        let packet = serde_json::json!({
            "delivery_task_packet": {
                "proof_targets": [
                    "RecordActivityType tests detect meeting from category or label",
                    "Repository tests prove meeting schedule sends partner_ids/calendar.event fields"
                ]
            }
        });
        let scope = proof_scope_from_dispatch_packet(&packet);
        assert!(scope.proof_intent_present);
        assert!(scope.paths.is_empty());
    }

    #[test]
    fn proof_scope_collects_explicit_and_textual_test_paths() {
        let packet = serde_json::json!({
            "proof_artifact_paths": ["src/test/a_test.dart"],
            "delivery_task_packet": {
                "verification_commands": [
                    "flutter test src/test/b_test.dart"
                ]
            }
        });
        let scope = proof_scope_from_dispatch_packet(&packet);
        assert_eq!(scope.paths, vec!["src/test/a_test.dart", "src/test/b_test.dart"]);
        assert!(scope.proof_intent_present);
    }

    #[test]
    fn test_like_paths_from_changed_files_are_concrete_proof_candidates() {
        let paths = collect_test_like_paths_from_values([
            "src/lib/feature.dart",
            "src/test/features/list_view/data/record_chatter_repository_test.dart",
        ]);
        assert_eq!(
            paths,
            vec!["src/test/features/list_view/data/record_chatter_repository_test.dart"]
        );
    }
}
