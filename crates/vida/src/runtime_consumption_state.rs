use std::path::Path;
use std::time::SystemTime;

use time::format_description::well_known::Rfc3339;

use super::{block_on_state_store, StateStore};
use crate::state_store::RunGraphDispatchReceiptSummary;

#[derive(Debug, serde::Serialize)]
pub(crate) struct RuntimeConsumptionSummary {
    pub(crate) total_snapshots: usize,
    pub(crate) bundle_snapshots: usize,
    pub(crate) bundle_check_snapshots: usize,
    pub(crate) final_snapshots: usize,
    pub(crate) latest_kind: Option<String>,
    pub(crate) latest_snapshot_path: Option<String>,
}

impl RuntimeConsumptionSummary {
    pub(crate) fn as_display(&self) -> String {
        if self.total_snapshots == 0 {
            return "0 snapshots".to_string();
        }

        format!(
            "{} snapshots (bundle={}, bundle_check={}, final={}, latest_kind={}, latest_path={})",
            self.total_snapshots,
            self.bundle_snapshots,
            self.bundle_check_snapshots,
            self.final_snapshots,
            self.latest_kind.as_deref().unwrap_or("none"),
            self.latest_snapshot_path.as_deref().unwrap_or("none")
        )
    }
}

pub(crate) const RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX: &str =
    "runtime_consumption_snapshot_index";
pub(crate) const RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL: &str =
    "runtime_consumption_snapshot_registry:latest_final_release_admission";
pub(crate) const RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_RECORDED_FINAL: &str =
    "runtime_consumption_snapshot_registry:latest_recorded_final_snapshot";
pub(crate) const RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_FINAL_SNAPSHOT: &str =
    "latest_final_release_admission_snapshot";
pub(crate) const RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_RECORDED_FINAL_SNAPSHOT: &str =
    "latest_recorded_final_snapshot";
pub(crate) const RETRIEVAL_TRUST_ACL_CONTEXT_PROTOCOL_BINDING_RECEIPT: &str =
    "protocol_binding_receipt";
pub(crate) const RETRIEVAL_TRUST_ACL_PROPAGATION_PROTOCOL_BINDING_GATE: &str =
    "protocol_binding_receipt_runtime_gate";
pub(crate) const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER: &str =
    "run_graph_latest_dispatch_receipt_summary_inconsistent";
pub(crate) const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_NEXT_ACTION:
    &str = "Refresh the latest run-graph dispatch receipt summary before rerunning consume-final.";
pub(crate) const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER: &str =
    "run_graph_latest_dispatch_receipt_checkpoint_leakage";
pub(crate) const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION: &str = "Refresh the latest checkpoint evidence before rerunning consume-final so the latest status and checkpoint rows share the same run_id.";

pub(crate) fn latest_admissible_retrieval_trust_signal(
    runtime_consumption: &RuntimeConsumptionSummary,
    latest_final_snapshot_path: Option<&str>,
    protocol_binding_latest_receipt_id: Option<&str>,
) -> Option<serde_json::Value> {
    let citation = latest_final_snapshot_path?.trim();
    let acl = protocol_binding_latest_receipt_id?.trim();

    if citation.is_empty() || acl.is_empty() || runtime_consumption.final_snapshots == 0 {
        return None;
    }

    Some(serde_json::json!({
        "source": RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX,
        "source_registry_ref": RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL,
        "citation": citation,
        "freshness": "final",
        "freshness_posture": RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_FINAL_SNAPSHOT,
        "acl": acl,
        "acl_context": format!(
            "{}:{acl}",
            RETRIEVAL_TRUST_ACL_CONTEXT_PROTOCOL_BINDING_RECEIPT
        ),
        "acl_propagation": RETRIEVAL_TRUST_ACL_PROPAGATION_PROTOCOL_BINDING_GATE,
    }))
}

pub(crate) fn write_runtime_consumption_snapshot(
    state_root: &Path,
    prefix: &str,
    payload: &serde_json::Value,
) -> Result<String, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    std::fs::create_dir_all(&snapshot_dir)
        .map_err(|error| format!("Failed to create runtime-consumption directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render")
        .replace(':', "-");
    let snapshot_path = snapshot_dir.join(format!("{prefix}-{ts}.json"));
    let body = serde_json::to_string_pretty(payload)
        .map_err(|error| format!("Failed to encode runtime-consumption snapshot: {error}"))?;
    std::fs::write(&snapshot_path, body)
        .map_err(|error| format!("Failed to write runtime-consumption snapshot: {error}"))?;
    Ok(snapshot_path.display().to_string())
}

pub(crate) fn runtime_consumption_final_dispatch_receipt_blocker_code(
    store: &StateStore,
    payload_json: &serde_json::Value,
) -> Result<Option<String>, String> {
    let Some(latest_status) = block_on_state_store(store.latest_run_graph_status())? else {
        return Ok(None);
    };
    let Some(payload_run_id) = payload_json["dispatch_receipt"]["run_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(Some(
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER.to_string(),
        ));
    };
    runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
        latest_status.run_id.as_str(),
        payload_run_id,
        block_on_state_store(store.latest_run_graph_dispatch_receipt_summary()),
    )
}

pub(crate) fn runtime_consumption_final_dispatch_receipt_blocker_code_for_run(
    store: &StateStore,
    payload_json: &serde_json::Value,
    run_id: &str,
) -> Result<Option<String>, String> {
    let status = block_on_state_store(store.run_graph_status(run_id)).map_err(|error| {
        format!("Failed to read persisted run-graph state for `{run_id}`: {error}")
    })?;
    let Some(payload_run_id) = payload_json["dispatch_receipt"]["run_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(Some(
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER.to_string(),
        ));
    };
    let dispatch_receipt_summary = block_on_state_store(store.run_graph_dispatch_receipt(run_id))
        .map_err(|error| {
            format!("Failed to read persisted run-graph dispatch receipt for `{run_id}`: {error}")
        })
        .map(|receipt| {
            receipt.map(crate::state_store::RunGraphDispatchReceiptSummary::from_receipt)
        });
    runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
        status.run_id.as_str(),
        payload_run_id,
        dispatch_receipt_summary,
    )
}

pub(crate) fn runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
    latest_status_run_id: &str,
    payload_run_id: &str,
    dispatch_receipt_summary: Result<Option<RunGraphDispatchReceiptSummary>, String>,
) -> Result<Option<String>, String> {
    if payload_run_id != latest_status_run_id {
        return Ok(Some(
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER.to_string(),
        ));
    }

    match dispatch_receipt_summary {
        Ok(Some(summary)) if summary.run_id == latest_status_run_id => Ok(None),
        Ok(_) => Ok(Some(
            RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER.to_string(),
        )),
        Err(error) if error.contains("latest checkpoint evidence must share the same run_id") => {
            Ok(Some(
                RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER.to_string(),
            ))
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn apply_runtime_consumption_final_dispatch_receipt_blocker(
    payload_json: &mut serde_json::Value,
    blocker_code: &str,
) {
    if let Some(payload_object) = payload_json.as_object_mut() {
        payload_object.insert(
            "direct_consumption_ready".to_string(),
            serde_json::Value::Bool(false),
        );
    }
    if let Some(dispatch_receipt) = payload_json
        .get_mut("dispatch_receipt")
        .and_then(serde_json::Value::as_object_mut)
    {
        dispatch_receipt.insert(
            "blocker_code".to_string(),
            serde_json::Value::String(blocker_code.to_string()),
        );
    }
}

pub(crate) fn runtime_consumption_summary(
    state_root: &Path,
) -> Result<RuntimeConsumptionSummary, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    if !snapshot_dir.exists() {
        return Ok(RuntimeConsumptionSummary {
            total_snapshots: 0,
            bundle_snapshots: 0,
            bundle_check_snapshots: 0,
            final_snapshots: 0,
            latest_kind: None,
            latest_snapshot_path: None,
        });
    }

    let mut total_snapshots = 0usize;
    let mut bundle_snapshots = 0usize;
    let mut bundle_check_snapshots = 0usize;
    let mut final_snapshots = 0usize;
    let mut latest: Option<(SystemTime, String, String)> = None;

    for entry in std::fs::read_dir(&snapshot_dir)
        .map_err(|error| format!("Failed to read runtime-consumption directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to inspect runtime-consumption entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        total_snapshots += 1;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let kind = if file_name.starts_with("bundle-check-") {
            bundle_check_snapshots += 1;
            "bundle-check".to_string()
        } else if file_name.starts_with("bundle-") {
            bundle_snapshots += 1;
            "bundle".to_string()
        } else if file_name.starts_with("final-") {
            final_snapshots += 1;
            "final".to_string()
        } else {
            "unknown".to_string()
        };

        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let path_display = path.display().to_string();
        match &latest {
            Some((latest_modified, _, _)) if modified <= *latest_modified => {}
            _ => latest = Some((modified, kind, path_display)),
        }
    }

    Ok(RuntimeConsumptionSummary {
        total_snapshots,
        bundle_snapshots,
        bundle_check_snapshots,
        final_snapshots,
        latest_kind: latest.as_ref().map(|(_, kind, _)| kind.clone()),
        latest_snapshot_path: latest.map(|(_, _, path)| path),
    })
}

pub(crate) fn runtime_consumption_snapshot_has_release_admission_evidence(
    snapshot: &serde_json::Value,
) -> bool {
    let operator_contracts = match snapshot.get("operator_contracts") {
        Some(value) => value,
        None => return false,
    };
    let status_ok =
        crate::operator_contracts::canonical_release1_operator_contract_status(&snapshot["status"])
            .is_some();
    let operator_status_ok =
        crate::operator_contracts::canonical_release1_operator_contract_status(
            &operator_contracts["status"],
        )
        .is_some();
    let release_admission = snapshot
        .get("release_admission")
        .and_then(serde_json::Value::as_object)
        .or_else(|| {
            snapshot
                .get("closure_admission")
                .and_then(serde_json::Value::as_object)
        })
        .or_else(|| {
            snapshot
                .get("payload")
                .and_then(|payload| payload.get("closure_admission"))
                .and_then(serde_json::Value::as_object)
        })
        .or_else(|| {
            snapshot
                .get("payload")
                .and_then(|payload| payload.get("release_admission"))
                .and_then(serde_json::Value::as_object)
        });

    status_ok && operator_status_ok && release_admission.is_some()
}

fn runtime_consumption_snapshot_release_admission(
    snapshot: &serde_json::Value,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
    snapshot
        .get("release_admission")
        .and_then(serde_json::Value::as_object)
        .or_else(|| {
            snapshot
                .get("closure_admission")
                .and_then(serde_json::Value::as_object)
        })
        .or_else(|| {
            snapshot
                .get("payload")
                .and_then(|payload| payload.get("closure_admission"))
                .and_then(serde_json::Value::as_object)
        })
        .or_else(|| {
            snapshot
                .get("payload")
                .and_then(|payload| payload.get("release_admission"))
                .and_then(serde_json::Value::as_object)
        })
}

fn runtime_consumption_snapshot_has_admissible_release_admission(
    snapshot: &serde_json::Value,
) -> bool {
    if !runtime_consumption_snapshot_has_release_admission_evidence(snapshot) {
        return false;
    }

    let Some(release_admission) = runtime_consumption_snapshot_release_admission(snapshot) else {
        return false;
    };
    let admitted = release_admission
        .get("admitted")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let blockers_clear = release_admission
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| rows.is_empty());
    let status = release_admission
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    admitted && blockers_clear && !matches!(status, "" | "block" | "blocked")
}

pub(crate) fn latest_final_runtime_consumption_snapshot_path(
    state_root: &Path,
) -> Result<Option<String>, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    latest_runtime_consumption_snapshot_path_matching(&snapshot_dir, |file_name, snapshot| {
        file_name.starts_with("final-")
            && runtime_consumption_snapshot_has_admissible_release_admission(snapshot)
    })
}

pub(crate) fn latest_recorded_final_runtime_consumption_snapshot_path(
    state_root: &Path,
) -> Result<Option<String>, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    latest_runtime_consumption_snapshot_path_matching(&snapshot_dir, |file_name, _| {
        file_name.starts_with("final-")
    })
}

pub(crate) fn latest_terminal_consume_continue_snapshot_run_id(
    state_root: &Path,
) -> Result<Option<String>, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    latest_runtime_consumption_snapshot_matching(&snapshot_dir, |file_name, snapshot| {
        if !file_name.starts_with("final-") {
            return None;
        }
        if snapshot.get("surface").and_then(serde_json::Value::as_str)
            != Some("vida taskflow consume continue")
        {
            return None;
        }
        if snapshot.get("status").and_then(serde_json::Value::as_str) != Some("pass") {
            return None;
        }
        let top_level_next_actions_empty = snapshot
            .get("next_actions")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|actions| actions.is_empty());
        let operator_next_actions_empty = snapshot
            .get("operator_contracts")
            .and_then(|contracts| contracts.get("next_actions"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|actions| actions.is_empty());
        let blockers_empty = snapshot
            .get("blocker_codes")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|blockers| blockers.is_empty());
        if !(top_level_next_actions_empty && operator_next_actions_empty && blockers_empty) {
            return None;
        }
        snapshot
            .get("source_run_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                snapshot
                    .get("artifact_refs")
                    .and_then(|refs| refs.get("latest_run_graph_dispatch_receipt_id"))
                    .and_then(serde_json::Value::as_str)
            })
            .or_else(|| {
                snapshot
                    .get("operator_contracts")
                    .and_then(|contracts| contracts.get("artifact_refs"))
                    .and_then(|refs| refs.get("latest_run_graph_dispatch_receipt_id"))
                    .and_then(serde_json::Value::as_str)
            })
            .or_else(|| {
                snapshot
                    .get("dispatch_receipt")
                    .and_then(|receipt| receipt.get("run_id"))
                    .and_then(serde_json::Value::as_str)
            })
            .map(str::trim)
            .filter(|run_id| !run_id.is_empty())
            .map(str::to_string)
    })
}

fn latest_runtime_consumption_snapshot_matching<F, T>(
    snapshot_dir: &Path,
    mut include: F,
) -> Result<Option<T>, String>
where
    F: FnMut(&str, &serde_json::Value) -> Option<T>,
{
    if !snapshot_dir.exists() {
        return Ok(None);
    }

    let mut latest_valid: Option<(SystemTime, T)> = None;
    for entry in std::fs::read_dir(snapshot_dir)
        .map_err(|error| format!("Failed to read runtime-consumption directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to inspect runtime-consumption entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !file_name.starts_with("final-") {
            continue;
        }

        let payload = match std::fs::read_to_string(&path) {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        let snapshot = match serde_json::from_str::<serde_json::Value>(&payload) {
            Ok(snapshot) => snapshot,
            Err(_) => continue,
        };
        let Some(value) = include(file_name, &snapshot) else {
            continue;
        };

        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        match &latest_valid {
            Some((latest_modified, _)) if modified <= *latest_modified => {}
            _ => latest_valid = Some((modified, value)),
        }
    }

    Ok(latest_valid.map(|(_, value)| value))
}

fn latest_runtime_consumption_snapshot_path_matching<F>(
    snapshot_dir: &Path,
    mut include: F,
) -> Result<Option<String>, String>
where
    F: FnMut(&str, &serde_json::Value) -> bool,
{
    if !snapshot_dir.exists() {
        return Ok(None);
    }

    let mut latest_valid: Option<(SystemTime, String)> = None;
    for entry in std::fs::read_dir(&snapshot_dir)
        .map_err(|error| format!("Failed to read runtime-consumption directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to inspect runtime-consumption entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !file_name.starts_with("final-") {
            continue;
        }

        let payload = match std::fs::read_to_string(&path) {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        let snapshot = match serde_json::from_str::<serde_json::Value>(&payload) {
            Ok(snapshot) => snapshot,
            Err(_) => continue,
        };
        if !include(file_name, &snapshot) {
            continue;
        }

        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let path_display = path.display().to_string();
        match &latest_valid {
            Some((latest_modified, _)) if modified <= *latest_modified => {}
            _ => latest_valid = Some((modified, path_display)),
        }
    }

    Ok(latest_valid.map(|(_, path)| path))
}

#[cfg(test)]
mod tests {
    use super::{
        apply_runtime_consumption_final_dispatch_receipt_blocker,
        latest_admissible_retrieval_trust_signal, latest_final_runtime_consumption_snapshot_path,
        latest_terminal_consume_continue_snapshot_run_id,
        runtime_consumption_final_dispatch_receipt_blocker_code,
        runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result,
        runtime_consumption_snapshot_has_release_admission_evidence, RuntimeConsumptionSummary,
        RETRIEVAL_TRUST_ACL_CONTEXT_PROTOCOL_BINDING_RECEIPT,
        RETRIEVAL_TRUST_ACL_PROPAGATION_PROTOCOL_BINDING_GATE,
        RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_FINAL_SNAPSHOT,
        RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL,
        RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX,
    };
    use crate::state_store::{RunGraphDispatchReceiptSummary, RunGraphStatus};
    use std::{fs, thread, time::Duration};

    fn sample_runtime_consumption_summary(
        latest_kind: Option<&str>,
        latest_snapshot_path: Option<&str>,
    ) -> RuntimeConsumptionSummary {
        RuntimeConsumptionSummary {
            total_snapshots: 2,
            bundle_snapshots: 1,
            bundle_check_snapshots: 0,
            final_snapshots: 1,
            latest_kind: latest_kind.map(str::to_string),
            latest_snapshot_path: latest_snapshot_path.map(str::to_string),
        }
    }

    #[test]
    fn latest_admissible_retrieval_trust_signal_accepts_latest_final_snapshot() {
        let runtime_consumption = sample_runtime_consumption_summary(
            Some("final"),
            Some("/tmp/project/runtime-consumption/final-2.json"),
        );

        let signal = latest_admissible_retrieval_trust_signal(
            &runtime_consumption,
            Some("/tmp/project/runtime-consumption/final-2.json"),
            Some("protocol-binding-receipt-2"),
        )
        .expect("latest admissible evidence should produce a retrieval trust signal");

        assert_eq!(
            signal["source"],
            RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX
        );
        assert_eq!(
            signal["citation"],
            "/tmp/project/runtime-consumption/final-2.json"
        );
        assert_eq!(signal["freshness"], "final");
        assert_eq!(
            signal["source_registry_ref"],
            RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL
        );
        assert_eq!(
            signal["freshness_posture"],
            RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_FINAL_SNAPSHOT
        );
        assert_eq!(signal["acl"], "protocol-binding-receipt-2");
        assert_eq!(
            signal["acl_context"],
            format!(
                "{}:{}",
                RETRIEVAL_TRUST_ACL_CONTEXT_PROTOCOL_BINDING_RECEIPT, "protocol-binding-receipt-2"
            )
        );
        assert_eq!(
            signal["acl_propagation"],
            RETRIEVAL_TRUST_ACL_PROPAGATION_PROTOCOL_BINDING_GATE
        );
    }

    #[test]
    fn latest_admissible_retrieval_trust_signal_blocks_stale_or_non_final_evidence() {
        let non_final_runtime_consumption = sample_runtime_consumption_summary(
            Some("bundle"),
            Some("/tmp/project/runtime-consumption/bundle-3.json"),
        );
        assert!(latest_admissible_retrieval_trust_signal(
            &non_final_runtime_consumption,
            Some("/tmp/project/runtime-consumption/final-2.json"),
            Some("protocol-binding-receipt-2"),
        )
        .is_some());

        let stale_final_runtime_consumption = sample_runtime_consumption_summary(
            Some("final"),
            Some("/tmp/project/runtime-consumption/final-2.json"),
        );
        assert!(latest_admissible_retrieval_trust_signal(
            &stale_final_runtime_consumption,
            Some("/tmp/project/runtime-consumption/final-1.json"),
            Some("protocol-binding-receipt-2"),
        )
        .is_some());

        assert!(latest_admissible_retrieval_trust_signal(
            &stale_final_runtime_consumption,
            Some("/tmp/project/runtime-consumption/final-2.json"),
            None,
        )
        .is_none());
    }

    #[test]
    fn latest_admissible_retrieval_trust_signal_ignores_newer_non_final_snapshot() {
        let runtime_consumption = RuntimeConsumptionSummary {
            total_snapshots: 4,
            bundle_snapshots: 1,
            bundle_check_snapshots: 1,
            final_snapshots: 2,
            latest_kind: Some("bundle-check".to_string()),
            latest_snapshot_path: Some(
                "/tmp/project/runtime-consumption/bundle-check-9.json".to_string(),
            ),
        };

        let signal = latest_admissible_retrieval_trust_signal(
            &runtime_consumption,
            Some("/tmp/project/runtime-consumption/final-8.json"),
            Some("protocol-binding-receipt-2"),
        )
        .expect("latest admissible final snapshot should remain trusted");

        assert_eq!(
            signal["citation"],
            "/tmp/project/runtime-consumption/final-8.json"
        );
        assert_eq!(signal["freshness"], "final");
    }

    #[test]
    fn latest_final_runtime_consumption_snapshot_path_prefers_newest_valid_final_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "vida-valid-final-snapshot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let valid_path = runtime_dir.join("final-valid.json");
        fs::write(
            &valid_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": []
                },
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {
                        "retrieval_trust_signal": {
                            "source": "runtime_consumption_snapshot_index",
                            "citation": "runtime-consumption/final-valid.json",
                            "freshness": "final",
                            "acl": "protocol-binding-receipt-id"
                        }
                    }
                },
                "payload": {
                    "closure_admission": {
                        "status": "pass",
                        "admitted": true,
                        "blockers": [],
                        "proof_surfaces": ["vida taskflow consume final"]
                    }
                }
            })
            .to_string(),
        )
        .expect("valid final snapshot should be writable");

        thread::sleep(Duration::from_millis(5));

        let invalid_path = runtime_dir.join("final-incomplete.json");
        fs::write(
            &invalid_path,
            serde_json::json!({
                "surface": "vida taskflow consume continue",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": []
                },
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                }
            })
            .to_string(),
        )
        .expect("incomplete final snapshot should be writable");

        let selected = latest_final_runtime_consumption_snapshot_path(&root)
            .expect("latest valid final snapshot should resolve")
            .expect("one valid final snapshot should be available");
        assert_eq!(selected, valid_path.display().to_string());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn latest_final_runtime_consumption_snapshot_path_ignores_newer_blocked_final_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "vida-admissible-final-snapshot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let admissible_path = runtime_dir.join("final-admissible.json");
        fs::write(
            &admissible_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": {
                        "status": "admit",
                        "admitted": true,
                        "blockers": [],
                        "proof_surfaces": ["vida taskflow consume final"]
                    }
                }
            })
            .to_string(),
        )
        .expect("admissible final snapshot should be writable");

        thread::sleep(Duration::from_millis(5));

        let blocked_path = runtime_dir.join("final-blocked.json");
        fs::write(
            &blocked_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "blocked",
                "operator_contracts": {
                    "status": "blocked",
                    "blocker_codes": ["missing_retrieval_trust_evidence"],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": {
                        "status": "block",
                        "admitted": false,
                        "blockers": ["missing_retrieval_trust_evidence"],
                        "proof_surfaces": ["vida taskflow consume final"]
                    }
                }
            })
            .to_string(),
        )
        .expect("blocked final snapshot should be writable");

        let selected = latest_final_runtime_consumption_snapshot_path(&root)
            .expect("latest admissible final snapshot should resolve")
            .expect("one admissible final snapshot should be available");
        assert_eq!(selected, admissible_path.display().to_string());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn latest_terminal_consume_continue_snapshot_run_id_prefers_newest_terminal_continue() {
        let root = std::env::temp_dir().join(format!(
            "vida-terminal-consume-continue-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let blocked_path = runtime_dir.join("final-blocked-continue.json");
        fs::write(
            &blocked_path,
            serde_json::json!({
                "surface": "vida taskflow consume continue",
                "status": "blocked",
                "blocker_codes": ["still_blocked"],
                "next_actions": ["retry"],
                "operator_contracts": {
                    "status": "blocked",
                    "blocker_codes": ["still_blocked"],
                    "next_actions": ["retry"],
                    "artifact_refs": {
                        "latest_run_graph_dispatch_receipt_id": "run-blocked"
                    }
                }
            })
            .to_string(),
        )
        .expect("blocked continue snapshot should be writable");

        thread::sleep(Duration::from_millis(5));

        let terminal_path = runtime_dir.join("final-terminal-continue.json");
        fs::write(
            &terminal_path,
            serde_json::json!({
                "surface": "vida taskflow consume continue",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {
                        "latest_run_graph_dispatch_receipt_id": "run-terminal"
                    }
                }
            })
            .to_string(),
        )
        .expect("terminal continue snapshot should be writable");

        let run_id = latest_terminal_consume_continue_snapshot_run_id(&root)
            .expect("terminal continue lookup should succeed")
            .expect("terminal continue run id should resolve");
        assert_eq!(run_id, "run-terminal");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_consumption_snapshot_release_admission_accepts_payload_closure_admission() {
        let snapshot = serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "operator_contracts": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {}
            },
            "payload": {
                "closure_admission": {
                    "status": "pass",
                    "admitted": true,
                    "blockers": [],
                    "proof_surfaces": ["vida taskflow consume final"]
                }
            }
        });

        assert!(runtime_consumption_snapshot_has_release_admission_evidence(
            &snapshot
        ));
    }

    #[test]
    fn runtime_consumption_final_dispatch_receipt_blocker_code_stays_fail_closed_for_latest_run_mismatch(
    ) {
        let summary = RunGraphDispatchReceiptSummary {
            run_id: "run-latest".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: Some("vida agent-init".to_string()),
            dispatch_packet_path: Some("/tmp/latest-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/latest-result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: None,
            downstream_dispatch_command: None,
            downstream_dispatch_note: None,
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: None,
            downstream_dispatch_last_target: None,
            activation_agent_type: Some("junior".to_string()),
            activation_runtime_role: Some("worker".to_string()),
            selected_backend: Some("junior".to_string()),
            effective_execution_posture: serde_json::Value::Null,
            route_policy: serde_json::Value::Null,
            activation_evidence: serde_json::Value::Null,
            recorded_at: "2026-04-15T00:00:00Z".to_string(),
        };

        let blocker = runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
            "run-latest",
            "run-explicit",
            Ok(Some(summary)),
        )
        .expect("mismatch evaluation should succeed");

        assert_eq!(
            blocker.as_deref(),
            Some(crate::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn taskflow_consume_final_fails_closed_when_latest_dispatch_receipt_summary_is_missing() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-consume-final-summary-missing-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = crate::state_store::StateStore::open(root.clone())
            .await
            .expect("open store");

        let latest_status = RunGraphStatus {
            run_id: "run-final".to_string(),
            task_id: "task-final".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("worker".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "taskflow_state_store".to_string(),
            lane_id: "planning_lane".to_string(),
            lifecycle_stage: "runtime_consumption_ready".to_string(),
            policy_gate: "not_required".to_string(),
            handoff_state: "awaiting_worker".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.worker".to_string(),
            recovery_ready: true,
        };
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest status");

        let mut payload = serde_json::json!({
            "dispatch_receipt": {
                "run_id": "run-final",
                "dispatch_status": "executed",
                "lane_status": "lane_running",
                "blocker_code": serde_json::Value::Null,
            },
            "direct_consumption_ready": true,
        });

        let blocker_code =
            runtime_consumption_final_dispatch_receipt_blocker_code(&store, &payload)
                .expect("blocker evaluation should succeed")
                .expect("missing receipt summary should fail closed");
        assert_eq!(
            blocker_code,
            crate::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER
        );

        apply_runtime_consumption_final_dispatch_receipt_blocker(&mut payload, &blocker_code);
        assert_eq!(payload["direct_consumption_ready"], false);
        assert_eq!(payload["dispatch_receipt"]["blocker_code"], blocker_code);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn taskflow_consume_final_propagates_checkpoint_leakage_blocker_code() {
        let payload = serde_json::json!({
            "dispatch_receipt": {
                "run_id": "run-final",
                "dispatch_status": "executed",
                "lane_status": "lane_open",
                "blocker_code": serde_json::Value::Null,
            },
            "direct_consumption_ready": true,
        });

        let blocker_code = runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
            "run-final",
            "run-final",
            Err(
                "invalid task record: run-graph dispatch receipt summary is inconsistent for `run-final`: latest checkpoint evidence must share the same run_id (latest_checkpoint_run_id=run-older)"
                    .to_string(),
            ),
        )
        .expect("blocker evaluation should succeed")
        .expect("checkpoint leakage should fail closed");
        assert_eq!(
            blocker_code,
            crate::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER
        );

        let mut payload = payload;
        apply_runtime_consumption_final_dispatch_receipt_blocker(&mut payload, &blocker_code);
        assert_eq!(payload["direct_consumption_ready"], false);
        assert_eq!(payload["dispatch_receipt"]["blocker_code"], blocker_code);
    }
}
