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
            super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER
                .to_string(),
        ));
    };
    runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
        latest_status.run_id.as_str(),
        payload_run_id,
        block_on_state_store(store.latest_run_graph_dispatch_receipt_summary()),
    )
}

pub(crate) fn runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result(
    latest_status_run_id: &str,
    payload_run_id: &str,
    dispatch_receipt_summary: Result<Option<RunGraphDispatchReceiptSummary>, String>,
) -> Result<Option<String>, String> {
    if payload_run_id != latest_status_run_id {
        return Ok(Some(
            super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER
                .to_string(),
        ));
    }

    match dispatch_receipt_summary {
        Ok(Some(summary)) if summary.run_id == latest_status_run_id => Ok(None),
        Ok(_) => Ok(Some(
            super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_SUMMARY_INCONSISTENT_BLOCKER
                .to_string(),
        )),
        Err(error) if error.contains("latest checkpoint evidence must share the same run_id") => {
            Ok(Some(
                super::RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER
                    .to_string(),
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
        .and_then(serde_json::Value::as_object);

    status_ok && operator_status_ok && release_admission.is_some()
}

pub(crate) fn latest_final_runtime_consumption_snapshot_path(
    state_root: &Path,
) -> Result<Option<String>, String> {
    let summary = runtime_consumption_summary(state_root)?;
    if summary.latest_kind.as_deref() == Some("final") {
        Ok(summary.latest_snapshot_path)
    } else {
        Ok(None)
    }
}
