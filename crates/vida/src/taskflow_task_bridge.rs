use std::path::{Path, PathBuf};

use crate::release1_contracts::{
    blocker_code_str, canonical_release1_contract_status_str, BlockerCode,
};
use crate::state_store::TaskRecord;

#[cfg(test)]
thread_local! {
    static TEST_PROXY_STATE_DIR_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

pub(crate) fn taskflow_native_state_root(project_root: &Path) -> PathBuf {
    project_root.join(crate::state_store::default_state_dir())
}

pub(crate) fn proxy_state_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = TEST_PROXY_STATE_DIR_OVERRIDE.with_borrow(|path| path.clone()) {
        return path;
    }
    std::env::var_os("VIDA_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            crate::resolve_runtime_project_root()
                .map(|project_root| taskflow_native_state_root(&project_root))
                .unwrap_or_else(|_| crate::state_store::default_state_dir())
        })
}

#[cfg(test)]
pub(crate) fn set_test_proxy_state_dir_override(path: Option<PathBuf>) {
    TEST_PROXY_STATE_DIR_OVERRIDE.with_borrow_mut(|current| {
        *current = path;
    });
}

pub(crate) fn infer_project_root_from_state_root(state_root: &Path) -> Option<PathBuf> {
    state_root
        .ancestors()
        .find(|path| super::looks_like_project_root(path))
        .map(|path| {
            if path.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                path.to_path_buf()
            }
        })
}

fn read_runtime_consumption_snapshot(state_root: &Path) -> Result<serde_json::Value, String> {
    let snapshot_path = crate::latest_final_runtime_consumption_snapshot_path(state_root)?
        .ok_or_else(|| {
            "execution_preparation_gate_blocked: latest runtime-consumption snapshot is not `final`"
                .to_string()
        })?;
    let snapshot_body = std::fs::read_to_string(&snapshot_path).map_err(|error| {
        format!(
            "execution_preparation_gate_blocked: failed to read runtime-consumption snapshot: {error}"
        )
    })?;
    serde_json::from_str::<serde_json::Value>(&snapshot_body).map_err(|error| {
        format!(
            "execution_preparation_gate_blocked: failed to parse runtime-consumption snapshot: {error}"
        )
    })
}

fn has_execution_preparation_blocker(snapshot: &serde_json::Value) -> bool {
    let pending_execution_preparation_evidence =
        blocker_code_str(BlockerCode::PendingExecutionPreparationEvidence);
    let pending_design_packet = blocker_code_str(BlockerCode::PendingDesignPacket);
    let pending_developer_handoff_packet =
        blocker_code_str(BlockerCode::PendingDeveloperHandoffPacket);
    let missing_execution_preparation_contract =
        blocker_code_str(BlockerCode::MissingExecutionPreparationContract);
    let mut blockers: Vec<&str> = Vec::new();
    if let Some(rows) = snapshot["closure_admission"]["blockers"].as_array() {
        blockers.extend(rows.iter().filter_map(serde_json::Value::as_str));
    }
    if let Some(rows) = snapshot["operator_contracts"]["blocker_codes"].as_array() {
        blockers.extend(rows.iter().filter_map(serde_json::Value::as_str));
    }
    if let Some(code) = snapshot["dispatch_receipt"]["blocker_code"].as_str() {
        blockers.push(code);
    }
    blockers.iter().any(|value| {
        *value == pending_execution_preparation_evidence
            || *value == pending_design_packet
            || *value == pending_developer_handoff_packet
            || *value == missing_execution_preparation_contract
    })
}

pub(crate) fn enforce_execution_preparation_contract_gate(state_root: &Path) -> Result<(), String> {
    let snapshot = read_runtime_consumption_snapshot(state_root)?;
    let contract = &snapshot["operator_contracts"];
    let contract_ready = contract["contract_id"].as_str() == Some("release-1-operator-contracts")
        && contract["schema_version"].as_str() == Some("release-1-v1")
        && contract["status"].is_string()
        && contract["blocker_codes"].is_array()
        && contract["next_actions"].is_array()
        && contract["artifact_refs"].is_object();
    if !contract_ready {
        return Err(
            "execution_preparation_gate_blocked: missing or invalid release-1 operator contract"
                .to_string(),
        );
    }
    if contract["status"]
        .as_str()
        .and_then(canonical_release1_contract_status_str)
        .is_none()
    {
        return Err(
            "execution_preparation_gate_blocked: release-1 operator contract has invalid status"
                .to_string(),
        );
    }
    if has_execution_preparation_blocker(&snapshot) {
        return Err(format!(
            "execution_preparation_gate_blocked: {}",
            blocker_code_str(BlockerCode::PendingExecutionPreparationEvidence)
        ));
    }
    Ok(())
}

pub(crate) fn task_record_json(task: &TaskRecord) -> serde_json::Value {
    serde_json::to_value(task).expect("task record should serialize")
}

fn parse_display_path(display_id: &str) -> Option<(String, Vec<u32>)> {
    let trimmed = display_id.trim();
    if !trimmed.starts_with("vida-") {
        return None;
    }
    let parts = trimmed.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts[0].len() <= 5 {
        return None;
    }
    let mut levels = Vec::new();
    for part in parts.iter().skip(1) {
        levels.push(part.parse::<u32>().ok()?);
    }
    Some((parts[0].to_string(), levels))
}

pub(crate) fn next_display_id_payload(
    rows: &[serde_json::Value],
    parent_display_id: &str,
) -> serde_json::Value {
    let Some((parent_root, parent_levels)) = parse_display_path(parent_display_id) else {
        return serde_json::json!({
            "valid": false,
            "reason": "invalid_parent_display_id",
            "parent_display_id": parent_display_id,
        });
    };

    let mut max_child = 0u32;
    for row in rows {
        let display_id = row
            .get("display_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| row.get("id").and_then(serde_json::Value::as_str))
            .unwrap_or_default();
        let Some((child_root, child_levels)) = parse_display_path(display_id) else {
            continue;
        };
        if child_root != parent_root || child_levels.len() != parent_levels.len() + 1 {
            continue;
        }
        if !parent_levels.is_empty() && child_levels[..parent_levels.len()] != parent_levels[..] {
            continue;
        }
        max_child = max_child.max(*child_levels.last().unwrap_or(&0));
    }

    let next_index = max_child + 1;
    serde_json::json!({
        "valid": true,
        "parent_display_id": parent_display_id,
        "next_display_id": format!("{parent_display_id}.{next_index}"),
        "next_index": next_index,
    })
}

pub(crate) fn resolve_task_id_by_display_id(
    rows: &[serde_json::Value],
    display_id: &str,
) -> serde_json::Value {
    for row in rows {
        let current = row
            .get("display_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| row.get("id").and_then(serde_json::Value::as_str))
            .unwrap_or_default();
        if current == display_id {
            return serde_json::json!({
                "found": true,
                "display_id": display_id,
                "task_id": row.get("id").and_then(serde_json::Value::as_str).unwrap_or_default(),
            });
        }
    }
    serde_json::json!({
        "found": false,
        "display_id": display_id,
        "reason": "parent_display_id_not_found",
    })
}

#[cfg(test)]
mod tests {
    use super::{enforce_execution_preparation_contract_gate, has_execution_preparation_blocker};
    use crate::release1_contracts::canonical_release1_contract_status_str;
    use std::fs;

    #[test]
    fn execution_preparation_blocker_ignores_unrelated_operator_contract_blockers() {
        let snapshot = serde_json::json!({
            "closure_admission": {
                "blockers": [],
            },
            "operator_contracts": {
                "blocker_codes": [
                    "migration_required",
                    "protocol_binding_blocking_issues",
                ],
            },
            "dispatch_receipt": {},
        });

        assert!(!has_execution_preparation_blocker(&snapshot));
    }

    #[test]
    fn execution_preparation_blocker_detects_pending_execution_preparation_evidence() {
        let snapshot = serde_json::json!({
            "closure_admission": {
                "blockers": [],
            },
            "operator_contracts": {
                "blocker_codes": [
                    "pending_execution_preparation_evidence",
                ],
            },
            "dispatch_receipt": {},
        });

        assert!(has_execution_preparation_blocker(&snapshot));
    }

    #[test]
    fn execution_preparation_blocker_detects_pending_developer_handoff_packet() {
        let snapshot = serde_json::json!({
            "closure_admission": {
                "blockers": ["pending_developer_handoff_packet"],
            },
            "operator_contracts": {
                "blocker_codes": [],
            },
            "dispatch_receipt": {},
        });

        assert!(has_execution_preparation_blocker(&snapshot));
    }

    #[test]
    fn execution_preparation_blocker_detects_pending_design_packet() {
        let snapshot = serde_json::json!({
            "closure_admission": {
                "blockers": ["pending_design_packet"],
            },
            "operator_contracts": {
                "blocker_codes": [],
            },
            "dispatch_receipt": {},
        });

        assert!(has_execution_preparation_blocker(&snapshot));
    }

    #[test]
    fn release1_operator_contract_status_compatibility_normalizes_to_canonical_vocabulary() {
        assert_eq!(canonical_release1_contract_status_str("pass"), Some("pass"));
        assert_eq!(
            canonical_release1_contract_status_str(" blocked "),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str("ok"), Some("pass"));
        assert_eq!(
            canonical_release1_contract_status_str("block"),
            Some("blocked")
        );
        assert_eq!(canonical_release1_contract_status_str("unknown"), None);
    }

    #[test]
    fn execution_preparation_contract_gate_accepts_release1_canonical_and_compat_statuses() {
        let cases = [
            ("pass", "final-pass.json", Vec::new(), Vec::new()),
            (
                "blocked",
                "final-blocked.json",
                vec!["migration_required".to_string()],
                vec!["Complete required migration before normal operation.".to_string()],
            ),
            ("ok", "final-ok.json", Vec::new(), Vec::new()),
            (
                "block",
                "final-block.json",
                vec!["migration_required".to_string()],
                vec!["Complete required migration before normal operation.".to_string()],
            ),
        ];

        for (status, file_name, blocker_codes, next_actions) in cases {
            let root = std::env::temp_dir().join(format!(
                "vida-taskflow-bridge-release1-operator-contract-gate-{}-{}-{}",
                std::process::id(),
                status,
                file_name
            ));
            let snapshot_dir = root.join("runtime-consumption");
            fs::create_dir_all(&snapshot_dir).expect("create snapshot dir");
            let operator_contracts = crate::build_release1_operator_contracts_envelope(
                status,
                blocker_codes,
                next_actions,
                serde_json::json!({
                    "retrieval_trust_signal": {
                        "source": "runtime_consumption_snapshot_index",
                        "citation": format!("runtime-consumption/{file_name}"),
                        "freshness": "final",
                        "acl": "protocol-binding-receipt-id"
                    }
                }),
            );
            let shared_fields = serde_json::json!({
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
            });
            let snapshot = serde_json::json!({
                "status": operator_contracts["status"].clone(),
                "blocker_codes": operator_contracts["blocker_codes"].clone(),
                "next_actions": operator_contracts["next_actions"].clone(),
                "artifact_refs": operator_contracts["artifact_refs"].clone(),
                "shared_fields": shared_fields,
                "closure_admission": {
                    "blockers": [],
                },
                "operator_contracts": operator_contracts,
                "dispatch_receipt": {
                    "blocker_code": null,
                },
            });
            let snapshot_path = snapshot_dir.join(file_name);
            fs::write(
                &snapshot_path,
                serde_json::to_string_pretty(&snapshot).expect("serialize snapshot"),
            )
            .expect("write runtime consumption snapshot");
            assert_eq!(
                enforce_execution_preparation_contract_gate(root.as_path()),
                Ok(())
            );

            let _ = fs::remove_dir_all(&root);
        }
    }
}
