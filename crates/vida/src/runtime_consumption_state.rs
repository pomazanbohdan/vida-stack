use std::path::Path;
use std::time::SystemTime;

use time::format_description::well_known::Rfc3339;

use super::{block_on_state_store, StateStore};
use crate::state_store::RunGraphDispatchReceiptSummary;

pub(crate) fn runtime_consumption_snapshot_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

struct RuntimeConsumptionSnapshotEntry {
    path: std::path::PathBuf,
    file_name: String,
    modified: SystemTime,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectedFinalRuntimeConsumptionSnapshot {
    pub(crate) path: String,
    pub(crate) payload: Option<serde_json::Value>,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum RuntimeReflexLoopStage {
    Plan,
    Produce,
    Evaluate,
    Critique,
    Refine,
}

impl RuntimeReflexLoopStage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            RuntimeReflexLoopStage::Plan => "PLAN",
            RuntimeReflexLoopStage::Produce => "PRODUCE",
            RuntimeReflexLoopStage::Evaluate => "EVALUATE",
            RuntimeReflexLoopStage::Critique => "CRITIQUE",
            RuntimeReflexLoopStage::Refine => "REFINE",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct RuntimeReflexLoopEvidenceRefs {
    pub(crate) taskflow: Vec<String>,
    pub(crate) docflow: Vec<String>,
    pub(crate) other: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct RuntimeReflexLoopRecord {
    pub(crate) schema_version: u8,
    pub(crate) loop_id: String,
    pub(crate) bounded_unit_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) artifact_id: Option<String>,
    pub(crate) stage: RuntimeReflexLoopStage,
    pub(crate) goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) decision: Option<String>,
    pub(crate) evidence_refs: RuntimeReflexLoopEvidenceRefs,
    pub(crate) source_surface: String,
    pub(crate) created_at: String,
    pub(crate) diagnostic_only: bool,
    pub(crate) grants_write_authority: bool,
    pub(crate) not_closure_proof: bool,
}

#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct RuntimeReflexLoopSummary {
    pub(crate) bounded_unit_id: String,
    pub(crate) total_records: usize,
    pub(crate) latest_stage: Option<RuntimeReflexLoopStage>,
    pub(crate) latest_record_path: Option<String>,
}

pub(crate) fn runtime_reflex_loop_record(
    bounded_unit_id: &str,
    artifact_id: Option<&str>,
    stage: RuntimeReflexLoopStage,
    goal: &str,
    decision: Option<&str>,
    evidence_refs: RuntimeReflexLoopEvidenceRefs,
    source_surface: &str,
) -> Result<RuntimeReflexLoopRecord, String> {
    let bounded_unit_id = non_empty_reflex_loop_field("bounded_unit_id", bounded_unit_id)?;
    let goal = non_empty_reflex_loop_field("goal", goal)?;
    let source_surface = non_empty_reflex_loop_field("source_surface", source_surface)?;
    let artifact_id = artifact_id
        .map(|value| non_empty_reflex_loop_field("artifact_id", value))
        .transpose()?;
    let decision = decision
        .map(|value| non_empty_reflex_loop_field("decision", value))
        .transpose()?;
    let created_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("rfc3339 timestamp should render");
    let loop_id = match artifact_id.as_deref() {
        Some(artifact_id) => format!("runtime-reflex-loop:{bounded_unit_id}:{artifact_id}"),
        None => format!("runtime-reflex-loop:{bounded_unit_id}"),
    };

    Ok(RuntimeReflexLoopRecord {
        schema_version: 1,
        loop_id,
        bounded_unit_id,
        artifact_id,
        stage,
        goal,
        decision,
        evidence_refs,
        source_surface,
        created_at,
        diagnostic_only: true,
        grants_write_authority: false,
        not_closure_proof: true,
    })
}

pub(crate) fn append_runtime_reflex_loop_record(
    state_root: &Path,
    record: &RuntimeReflexLoopRecord,
) -> Result<String, String> {
    validate_runtime_reflex_loop_record(record)?;
    let reflex_dir = runtime_reflex_loop_dir(state_root, &record.bounded_unit_id);
    std::fs::create_dir_all(&reflex_dir)
        .map_err(|error| format!("Failed to create runtime-reflex-loop directory: {error}"))?;
    let ts = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let record_path = reflex_dir.join(format!("reflex-{ts}-{}.json", record.stage.as_str()));
    let body = serde_json::to_string_pretty(record)
        .map_err(|error| format!("Failed to encode runtime-reflex-loop record: {error}"))?;
    std::fs::write(&record_path, body)
        .map_err(|error| format!("Failed to write runtime-reflex-loop record: {error}"))?;
    Ok(runtime_consumption_snapshot_path_string(&record_path))
}

pub(crate) fn latest_runtime_reflex_loop_record(
    state_root: &Path,
    bounded_unit_id: &str,
) -> Result<Option<RuntimeReflexLoopRecord>, String> {
    let bounded_unit_id = non_empty_reflex_loop_field("bounded_unit_id", bounded_unit_id)?;
    for entry in runtime_reflex_loop_entries_newest_first(state_root, &bounded_unit_id)? {
        let payload = match std::fs::read_to_string(&entry.path) {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        let record = match serde_json::from_str::<RuntimeReflexLoopRecord>(&payload) {
            Ok(record) => record,
            Err(_) => continue,
        };
        if record.bounded_unit_id != bounded_unit_id {
            continue;
        }
        if validate_runtime_reflex_loop_record(&record).is_err() {
            continue;
        }
        return Ok(Some(record));
    }

    Ok(None)
}

pub(crate) fn runtime_reflex_loop_summary(
    state_root: &Path,
    bounded_unit_id: &str,
) -> Result<RuntimeReflexLoopSummary, String> {
    let bounded_unit_id = non_empty_reflex_loop_field("bounded_unit_id", bounded_unit_id)?;
    let entries = runtime_reflex_loop_entries_newest_first(state_root, &bounded_unit_id)?;
    let latest_path = entries
        .first()
        .map(|entry| runtime_consumption_snapshot_path_string(&entry.path));
    let latest_stage =
        latest_runtime_reflex_loop_record(state_root, &bounded_unit_id)?.map(|record| record.stage);
    Ok(RuntimeReflexLoopSummary {
        bounded_unit_id,
        total_records: entries.len(),
        latest_stage,
        latest_record_path: latest_path,
    })
}

fn validate_runtime_reflex_loop_record(record: &RuntimeReflexLoopRecord) -> Result<(), String> {
    if record.schema_version != 1 {
        return Err(format!(
            "Unsupported runtime-reflex-loop schema_version `{}`",
            record.schema_version
        ));
    }
    non_empty_reflex_loop_field("loop_id", &record.loop_id)?;
    non_empty_reflex_loop_field("bounded_unit_id", &record.bounded_unit_id)?;
    non_empty_reflex_loop_field("goal", &record.goal)?;
    non_empty_reflex_loop_field("source_surface", &record.source_surface)?;
    non_empty_reflex_loop_field("created_at", &record.created_at)?;
    if !record.diagnostic_only || record.grants_write_authority || !record.not_closure_proof {
        return Err(
            "runtime-reflex-loop records are diagnostic evidence only and cannot grant write authority or closure proof"
                .to_string(),
        );
    }
    Ok(())
}

fn runtime_reflex_loop_dir(state_root: &Path, bounded_unit_id: &str) -> std::path::PathBuf {
    state_root
        .join("runtime-reflex-loop")
        .join(safe_runtime_reflex_loop_path_component(bounded_unit_id))
}

fn runtime_reflex_loop_entries_newest_first(
    state_root: &Path,
    bounded_unit_id: &str,
) -> Result<Vec<RuntimeConsumptionSnapshotEntry>, String> {
    let reflex_dir = runtime_reflex_loop_dir(state_root, bounded_unit_id);
    if !reflex_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&reflex_dir)
        .map_err(|error| format!("Failed to read runtime-reflex-loop directory: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("Failed to inspect runtime-reflex-loop entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        if !file_name.starts_with("reflex-") || !file_name.ends_with(".json") {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push(RuntimeConsumptionSnapshotEntry {
            path,
            file_name,
            modified,
        });
    }
    entries.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| right.file_name.cmp(&left.file_name))
    });
    Ok(entries)
}

fn safe_runtime_reflex_loop_path_component(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn non_empty_reflex_loop_field(field: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("runtime-reflex-loop `{field}` must not be empty"));
    }
    Ok(value.to_string())
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
    &str = "Run `vida status --json` to refresh the latest run-graph dispatch receipt summary, then inspect `vida taskflow recovery latest --json`; rerun consume-final only after latest status and dispatch receipt share the same concrete run_id.";
pub(crate) const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_BLOCKER: &str =
    "run_graph_latest_dispatch_receipt_checkpoint_leakage";
pub(crate) const RUNTIME_CONSUMPTION_LATEST_DISPATCH_RECEIPT_CHECKPOINT_LEAKAGE_NEXT_ACTION: &str = "Refresh the latest checkpoint evidence before rerunning consume-final so the latest status and checkpoint rows share the same run_id.";

pub(crate) fn latest_admissible_retrieval_trust_signal(
    runtime_consumption: &RuntimeConsumptionSummary,
    latest_final_snapshot_path: Option<&str>,
    protocol_binding_latest_receipt_id: Option<&str>,
) -> Option<serde_json::Value> {
    let acl = protocol_binding_latest_receipt_id?.trim();
    if acl.is_empty() {
        return None;
    }

    if let Some(citation) = latest_final_snapshot_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        return Some(serde_json::json!({
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
        }));
    }

    latest_bundle_check_retrieval_trust_signal(runtime_consumption, acl)
}

fn latest_bundle_check_retrieval_trust_signal(
    runtime_consumption: &RuntimeConsumptionSummary,
    current_protocol_binding_receipt_id: &str,
) -> Option<serde_json::Value> {
    if runtime_consumption.latest_kind.as_deref() != Some("bundle-check") {
        return None;
    }
    let snapshot_path = runtime_consumption.latest_snapshot_path.as_deref()?.trim();
    if snapshot_path.is_empty() {
        return None;
    }

    let payload = std::fs::read_to_string(snapshot_path).ok()?;
    let snapshot = serde_json::from_str::<serde_json::Value>(&payload).ok()?;
    if snapshot.get("surface").and_then(serde_json::Value::as_str)
        != Some("vida taskflow consume bundle check")
        || snapshot
            .get("check")
            .and_then(|check| check.get("ok"))
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || snapshot
            .get("operator_contracts")
            .and_then(|contracts| contracts.get("status"))
            .and_then(serde_json::Value::as_str)
            != Some("pass")
        || !snapshot
            .get("blocker_codes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|blockers| blockers.is_empty())
        || !snapshot
            .get("operator_contracts")
            .and_then(|contracts| contracts.get("blocker_codes"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|blockers| blockers.is_empty())
    {
        return None;
    }

    let signal = snapshot
        .get("artifact_refs")
        .and_then(|refs| refs.get("retrieval_trust_signal"))
        .or_else(|| {
            snapshot
                .get("operator_contracts")
                .and_then(|contracts| contracts.get("artifact_refs"))
                .and_then(|refs| refs.get("retrieval_trust_signal"))
        })
        .or_else(|| {
            snapshot
                .get("bundle")
                .and_then(|bundle| bundle.get("cache_delivery_contract"))
                .and_then(|contract| contract.get("retrieval_trust_evidence"))
        })?;

    retrieval_trust_signal_is_complete(signal, current_protocol_binding_receipt_id)
        .then(|| signal.clone())
}

fn retrieval_trust_signal_field<'a>(signal: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    signal
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn retrieval_trust_signal_is_complete(
    signal: &serde_json::Value,
    current_protocol_binding_receipt_id: &str,
) -> bool {
    retrieval_trust_signal_field(signal, "source")
        == Some(RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX)
        && retrieval_trust_signal_field(signal, "source_registry_ref").is_some()
        && retrieval_trust_signal_field(signal, "citation").is_some()
        && retrieval_trust_signal_field(signal, "freshness").is_some()
        && retrieval_trust_signal_field(signal, "freshness_posture").is_some()
        && retrieval_trust_signal_field(signal, "acl") == Some(current_protocol_binding_receipt_id)
        && retrieval_trust_signal_field(signal, "acl_context").is_some()
        && retrieval_trust_signal_field(signal, "acl_propagation").is_some()
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
    Ok(runtime_consumption_snapshot_path_string(&snapshot_path))
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

pub(crate) fn latest_final_runtime_consumption_dispatch_receipt_summary(
    state_root: &Path,
) -> Result<Option<RunGraphDispatchReceiptSummary>, String> {
    let Some(snapshot_path) = latest_recorded_final_runtime_consumption_snapshot_path(state_root)?
    else {
        return Ok(None);
    };
    let raw = std::fs::read_to_string(&snapshot_path).map_err(|error| {
        format!("Failed to read latest final runtime-consumption snapshot: {error}")
    })?;
    let payload = serde_json::from_str::<serde_json::Value>(&raw).map_err(|error| {
        format!("Failed to decode latest final runtime-consumption snapshot: {error}")
    })?;
    let receipt = &payload["payload"]["dispatch_receipt"];
    let Some(run_id) = json_non_empty_string(receipt, "run_id") else {
        return Ok(None);
    };
    let _run_id = run_id;
    // Final runtime-consumption snapshots are fallback context only. Persisted
    // dispatch receipt authority must come from the StateStore caller path.
    Ok(None)
}

fn json_non_empty_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value[key]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn json_optional_string(value: &serde_json::Value, key: &str) -> Option<String> {
    json_non_empty_string(value, key)
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
        Ok(Some(summary))
            if taskflow_authority::final_snapshot::final_snapshot_dispatch_receipt_authority_is_persisted(
                Some(payload_run_id),
                Some(latest_status_run_id),
                Some(summary.run_id.as_str()),
            ) =>
        {
            Ok(None)
        }
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
        let path_display = runtime_consumption_snapshot_path_string(&path);
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
    crate::release1_contracts::release_admission_operator_evidence_snapshot(snapshot)
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

    let authority = taskflow_authority::final_snapshot::FinalSnapshotReleaseAdmission {
        admitted,
        blockers_empty: blockers_clear,
        status,
    };
    taskflow_authority::final_snapshot::final_snapshot_has_admissible_release_admission(Some(
        &authority,
    ))
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

pub(crate) fn release_admission_operator_evidence_incomplete(
    state_root: &Path,
) -> Result<bool, String> {
    let Some(snapshot_path) = latest_recorded_final_runtime_consumption_snapshot_path(state_root)?
    else {
        return Ok(true);
    };

    let payload = std::fs::read_to_string(&snapshot_path).map_err(|error| {
        format!("Failed to read runtime-consumption snapshot `{snapshot_path}`: {error}")
    })?;
    let summary_json = serde_json::from_str::<serde_json::Value>(&payload).map_err(|error| {
        format!("Failed to parse runtime-consumption snapshot `{snapshot_path}`: {error}")
    })?;
    if crate::release1_operator_output::shared_operator_output_contract_parity_error(&summary_json)
        .is_some()
    {
        return Ok(true);
    }

    Ok(!runtime_consumption_snapshot_has_release_admission_evidence(&summary_json))
}

fn runtime_consumption_snapshot_source_run_id(snapshot: &serde_json::Value) -> Option<&str> {
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
}

pub(crate) fn selected_final_runtime_consumption_snapshot_for_run(
    state_root: &Path,
    effective_run_id: Option<&str>,
) -> Result<Option<SelectedFinalRuntimeConsumptionSnapshot>, String> {
    let Some(effective_run_id) = effective_run_id
        .map(str::trim)
        .filter(|run_id| !run_id.is_empty())
    else {
        return Ok(None);
    };
    let snapshot_dir = state_root.join("runtime-consumption");
    let mut newest_malformed_snapshot_path = None;
    for entry in runtime_consumption_snapshot_entries_newest_first(&snapshot_dir)? {
        let payload = match std::fs::read_to_string(&entry.path) {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        let payload = match serde_json::from_str::<serde_json::Value>(&payload) {
            Ok(payload) => payload,
            Err(_) => {
                newest_malformed_snapshot_path
                    .get_or_insert_with(|| runtime_consumption_snapshot_path_string(&entry.path));
                continue;
            }
        };
        if runtime_consumption_snapshot_source_run_id(&payload) != Some(effective_run_id) {
            continue;
        }
        return Ok(Some(SelectedFinalRuntimeConsumptionSnapshot {
            path: runtime_consumption_snapshot_path_string(&entry.path),
            payload: Some(payload),
        }));
    }

    Ok(
        newest_malformed_snapshot_path.map(|path| SelectedFinalRuntimeConsumptionSnapshot {
            path,
            payload: None,
        }),
    )
}

pub(crate) fn retrieval_trust_signal_from_selected_final_snapshot(
    selected_snapshot: Option<&SelectedFinalRuntimeConsumptionSnapshot>,
    protocol_binding_latest_receipt_id: Option<&str>,
) -> Option<serde_json::Value> {
    let acl = protocol_binding_latest_receipt_id?.trim();
    if acl.is_empty() {
        return None;
    }
    let selected_snapshot = selected_snapshot?;
    let payload = selected_snapshot.payload.as_ref()?;
    if !runtime_consumption_snapshot_has_admissible_release_admission(payload) {
        return None;
    }

    Some(serde_json::json!({
        "source": RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX,
        "source_registry_ref": RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL,
        "citation": selected_snapshot.path,
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

pub(crate) fn release_admission_operator_evidence_complete_for_run(
    state_root: &Path,
    run_id: &str,
) -> Result<bool, String> {
    let Some(snapshot_path) = latest_recorded_final_runtime_consumption_snapshot_path(state_root)?
    else {
        return Ok(false);
    };
    let payload = std::fs::read_to_string(&snapshot_path).map_err(|error| {
        format!("Failed to read runtime-consumption snapshot `{snapshot_path}`: {error}")
    })?;
    let summary_json = serde_json::from_str::<serde_json::Value>(&payload).map_err(|error| {
        format!("Failed to parse runtime-consumption snapshot `{snapshot_path}`: {error}")
    })?;
    if crate::release1_operator_output::shared_operator_output_contract_parity_error(&summary_json)
        .is_some()
    {
        return Ok(false);
    }

    if !runtime_consumption_snapshot_has_release_admission_evidence(&summary_json) {
        return Ok(false);
    }

    Ok(runtime_consumption_snapshot_source_run_id(&summary_json) == Some(run_id))
}

pub(crate) fn release_admission_operator_evidence_incomplete_from_latest_snapshot(
    latest_snapshot_path: Option<&str>,
) -> Result<bool, String> {
    let Some(snapshot_path) = latest_snapshot_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return Ok(true);
    };
    let payload = std::fs::read_to_string(snapshot_path).map_err(|error| {
        format!("Failed to read runtime-consumption snapshot `{snapshot_path}`: {error}")
    })?;
    let summary_json = serde_json::from_str::<serde_json::Value>(&payload).map_err(|error| {
        format!("Failed to parse runtime-consumption snapshot `{snapshot_path}`: {error}")
    })?;
    if summary_json
        .get("surface")
        .and_then(serde_json::Value::as_str)
        != Some("vida taskflow consume final")
        && summary_json.get("kind").and_then(serde_json::Value::as_str) != Some("final")
    {
        return Ok(true);
    }
    if crate::release1_operator_output::shared_operator_output_contract_parity_error(&summary_json)
        .is_some()
    {
        return Ok(true);
    }
    Ok(!runtime_consumption_snapshot_has_release_admission_evidence(&summary_json))
}

pub(crate) fn latest_terminal_consume_continue_snapshot_run_id(
    state_root: &Path,
) -> Result<Option<String>, String> {
    let snapshot_dir = state_root.join("runtime-consumption");
    latest_runtime_consumption_snapshot_matching(&snapshot_dir, |file_name, snapshot| {
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
        let deferred_handoff_projection = snapshot
            .get("projection_truth")
            .and_then(|truth| truth.get("projection_source"))
            .and_then(serde_json::Value::as_str)
            == Some("deferred_agent_handoff_receipt");
        let authority_snapshot =
            taskflow_authority::final_snapshot::TerminalConsumeContinueSnapshot {
                file_name,
                surface: snapshot
                    .get("surface")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                status: snapshot
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                top_level_next_actions_empty,
                operator_next_actions_empty,
                blockers_empty,
                deferred_handoff_projection,
                source_run_id: runtime_consumption_snapshot_source_run_id(snapshot),
            };
        taskflow_authority::final_snapshot::terminal_consume_continue_snapshot_run_id(
            &authority_snapshot,
        )
    })
}

fn latest_runtime_consumption_snapshot_matching<F, T>(
    snapshot_dir: &Path,
    mut include: F,
) -> Result<Option<T>, String>
where
    F: FnMut(&str, &serde_json::Value) -> Option<T>,
{
    for entry in runtime_consumption_snapshot_entries_newest_first(snapshot_dir)? {
        let payload = match std::fs::read_to_string(&entry.path) {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        let snapshot = match serde_json::from_str::<serde_json::Value>(&payload) {
            Ok(snapshot) => snapshot,
            Err(_) => continue,
        };
        let Some(value) = include(&entry.file_name, &snapshot) else {
            continue;
        };

        return Ok(Some(value));
    }

    Ok(None)
}

fn latest_runtime_consumption_snapshot_path_matching<F>(
    snapshot_dir: &Path,
    mut include: F,
) -> Result<Option<String>, String>
where
    F: FnMut(&str, &serde_json::Value) -> bool,
{
    for entry in runtime_consumption_snapshot_entries_newest_first(snapshot_dir)? {
        let payload = match std::fs::read_to_string(&entry.path) {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        let snapshot = match serde_json::from_str::<serde_json::Value>(&payload) {
            Ok(snapshot) => snapshot,
            Err(_) => continue,
        };
        if !include(&entry.file_name, &snapshot) {
            continue;
        }

        return Ok(Some(runtime_consumption_snapshot_path_string(&entry.path)));
    }

    Ok(None)
}

fn runtime_consumption_snapshot_entries_newest_first(
    snapshot_dir: &Path,
) -> Result<Vec<RuntimeConsumptionSnapshotEntry>, String> {
    if !snapshot_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
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
            .unwrap_or_default()
            .to_string();
        if !file_name.starts_with("final-") {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push(RuntimeConsumptionSnapshotEntry {
            path,
            file_name,
            modified,
        });
    }

    entries.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| right.file_name.cmp(&left.file_name))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{
        append_runtime_reflex_loop_record,
        apply_runtime_consumption_final_dispatch_receipt_blocker,
        latest_admissible_retrieval_trust_signal,
        latest_final_runtime_consumption_dispatch_receipt_summary,
        latest_final_runtime_consumption_snapshot_path, latest_runtime_reflex_loop_record,
        latest_terminal_consume_continue_snapshot_run_id,
        release_admission_operator_evidence_complete_for_run,
        release_admission_operator_evidence_incomplete,
        runtime_consumption_final_dispatch_receipt_blocker_code,
        runtime_consumption_final_dispatch_receipt_blocker_code_from_summary_result,
        runtime_consumption_snapshot_has_release_admission_evidence,
        runtime_consumption_snapshot_path_string, runtime_reflex_loop_record,
        runtime_reflex_loop_summary, write_runtime_consumption_snapshot, RuntimeConsumptionSummary,
        RuntimeReflexLoopEvidenceRefs, RuntimeReflexLoopStage,
        RETRIEVAL_TRUST_ACL_CONTEXT_PROTOCOL_BINDING_RECEIPT,
        RETRIEVAL_TRUST_ACL_PROPAGATION_PROTOCOL_BINDING_GATE,
        RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_FINAL_SNAPSHOT,
        RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL,
        RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX,
    };
    use crate::state_store::{
        RunGraphDispatchReceiptSummary, RunGraphStatus, TaskExecutionSemantics,
        TaskPlannerMetadata, TaskRecord,
    };
    use std::{fs, path::Path, thread, time::Duration};

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

    fn test_task_record(task_id: &str, status: &str) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: task_id.to_string(),
            status: status.to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: "2026-06-21T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-06-21T00:00:00Z".to_string(),
            closed_at: None,
            close_reason: None,
            source_repo: ".".to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: Vec::new(),
            execution_semantics: TaskExecutionSemantics::default(),
            planner_metadata: TaskPlannerMetadata::default(),
            provider_mapping: None,
            dependencies: Vec::new(),
        }
    }

    fn case10_closure_admission_record() -> serde_json::Value {
        serde_json::json!({
            "status": "pass",
            "admitted": true,
            "closure_decision": "closed",
            "decision_owner": "release-owner",
            "decision_at": "2026-05-19T00:00:00Z",
            "evidence_bundle_refs": ["evidence-bundle-case10"],
            "open_risk_acceptance_ids": ["risk-acceptance-case10"],
            "blockers": [],
            "proof_surfaces": ["vida taskflow consume final"],
            "evidence_table": [
                {
                    "evidence_class": "closure_decision_record",
                    "status": "pass",
                    "evidence_refs": ["closure-record-case10"]
                },
                {
                    "evidence_class": "runtime_consumption_final_snapshot",
                    "status": "pass",
                    "evidence_refs": ["final-snapshot-case10"]
                },
                {
                    "evidence_class": "docflow_readiness_and_proof_receipts",
                    "status": "pass",
                    "evidence_refs": ["docflow-readiness-case10", "docflow-proof-case10"]
                },
                {
                    "evidence_class": "lane_execution_and_handoff_receipts",
                    "status": "pass",
                    "evidence_refs": ["lane-execution-case10", "handoff-case10"]
                },
                {
                    "evidence_class": "replay_checkpoint_lineage_artifacts",
                    "status": "pass",
                    "evidence_refs": ["checkpoint-case10", "replay-case10"]
                },
                {
                    "evidence_class": "risk_acceptance_artifacts",
                    "status": "pass",
                    "evidence_refs": ["risk-acceptance-case10"]
                },
                {
                    "evidence_class": "evidence_bundle_linkage",
                    "status": "pass",
                    "evidence_refs": ["evidence-bundle-case10"]
                }
            ]
        })
    }

    #[test]
    fn runtime_reflex_loop_appends_bounded_records_and_exposes_latest_for_resume() {
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-reflex-loop-append-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let evidence_refs = RuntimeReflexLoopEvidenceRefs {
            taskflow: vec!["taskflow:runtime-reflex-loop-state-model".to_string()],
            docflow: vec!["docflow:runtime.direct-runtime-consumption-protocol".to_string()],
            other: Vec::new(),
        };
        let plan = runtime_reflex_loop_record(
            "runtime-reflex-loop-state-model",
            Some("protocol-generation"),
            RuntimeReflexLoopStage::Plan,
            "Model a bounded protocol generation loop",
            Some("Use append-only runtime state"),
            evidence_refs.clone(),
            "vida taskflow consume continue",
        )
        .expect("plan record should be valid");
        let plan_path =
            append_runtime_reflex_loop_record(&root, &plan).expect("plan record should append");
        thread::sleep(Duration::from_millis(5));
        let critique = runtime_reflex_loop_record(
            "runtime-reflex-loop-state-model",
            Some("protocol-generation"),
            RuntimeReflexLoopStage::Critique,
            "Model a bounded protocol generation loop",
            Some("Critique evidence before refine"),
            evidence_refs,
            "vida docflow proofcheck",
        )
        .expect("critique record should be valid");
        let critique_path =
            append_runtime_reflex_loop_record(&root, &critique).expect("critique should append");

        assert_ne!(plan_path, critique_path);
        assert!(Path::new(&plan_path).exists());
        assert!(Path::new(&critique_path).exists());

        let latest = latest_runtime_reflex_loop_record(&root, "runtime-reflex-loop-state-model")
            .expect("latest lookup should succeed")
            .expect("latest record should exist");
        assert_eq!(latest.stage, RuntimeReflexLoopStage::Critique);
        assert_eq!(latest.diagnostic_only, true);
        assert_eq!(latest.grants_write_authority, false);
        assert_eq!(latest.not_closure_proof, true);

        let summary = runtime_reflex_loop_summary(&root, "runtime-reflex-loop-state-model")
            .expect("summary lookup should succeed");
        assert_eq!(summary.total_records, 2);
        assert_eq!(summary.latest_stage, Some(RuntimeReflexLoopStage::Critique));
        assert_eq!(
            summary.latest_record_path.as_deref(),
            Some(critique_path.as_str())
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_reflex_loop_rejects_write_authority_or_closure_proof_records() {
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-reflex-loop-authority-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let mut record = runtime_reflex_loop_record(
            "runtime-reflex-loop-state-model",
            None,
            RuntimeReflexLoopStage::Refine,
            "Refine without bypassing runtime law",
            None,
            RuntimeReflexLoopEvidenceRefs::default(),
            "vida taskflow consume advance",
        )
        .expect("record should be constructible");

        record.grants_write_authority = true;
        let error = append_runtime_reflex_loop_record(&root, &record)
            .expect_err("authority-bearing records must be rejected");
        assert!(error.contains("cannot grant write authority"));

        record.grants_write_authority = false;
        record.not_closure_proof = false;
        let error = append_runtime_reflex_loop_record(&root, &record)
            .expect_err("closure-proof records must be rejected");
        assert!(error.contains("closure proof"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_consumption_snapshot_path_string_uses_contract_separators() {
        let raw =
            Path::new(r"C:\project\vida-stack\.vida\data\state\runtime-consumption\final.json");
        assert_eq!(
            runtime_consumption_snapshot_path_string(raw),
            "C:/project/vida-stack/.vida/data/state/runtime-consumption/final.json"
        );
    }

    #[test]
    fn latest_final_runtime_consumption_dispatch_receipt_summary_rejects_forged_snapshot_without_persisted_receipt(
    ) {
        let root = std::env::temp_dir().join(format!(
            "vida-final-runtime-consumption-forged-receipt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp state root should exist");
        let dispatch_packet_path = root.join("packets").join("dispatch-packet.json");
        let dispatch_result_path = root.join("results").join("dispatch-result.json");

        write_runtime_consumption_snapshot(
            &root,
            "final",
            &serde_json::json!({
                "kind": "final",
                "payload": {
                    "dispatch_receipt": {
                        "run_id": "forged-final-run",
                        "dispatch_target": "codex",
                        "dispatch_status": "blocked",
                        "lane_status": "lane_exception_takeover",
                        "supersedes_receipt_id": "superseded-receipt-1",
                        "exception_path_receipt_id": "exception-path-receipt-1",
                        "dispatch_kind": "agent_init",
                        "dispatch_packet_path": dispatch_packet_path,
                        "dispatch_result_path": dispatch_result_path,
                        "recorded_at": "2026-06-11T00:00:00Z"
                    }
                }
            }),
        )
        .expect("forged final snapshot should be writable");

        let summary = latest_final_runtime_consumption_dispatch_receipt_summary(&root)
            .expect("forged snapshot lookup should fail closed without error");

        assert_eq!(summary, None);
        let _ = fs::remove_dir_all(root);
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
    fn latest_admissible_retrieval_trust_signal_accepts_latest_bundle_check_signal() {
        let root = std::env::temp_dir().join(format!(
            "vida-bundle-check-retrieval-trust-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let snapshot_path = runtime_dir.join("bundle-check-2026-05-19T00-00-00Z.json");
        fs::write(
            &snapshot_path,
            serde_json::json!({
                "surface": "vida taskflow consume bundle check",
                "check": { "ok": true },
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {
                    "root_artifact_id": "framework-agent-definition",
                    "bundle_artifact_name": "taskflow_runtime_bundle",
                    "surface": "vida taskflow consume bundle check"
                },
                "operator_contracts": {
                    "contract_id": "release-1-operator-contracts",
                    "schema_version": "release-1-v1",
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {
                        "root_artifact_id": "framework-agent-definition",
                        "bundle_artifact_name": "taskflow_runtime_bundle",
                        "surface": "vida taskflow consume bundle check"
                    }
                },
                "bundle": {
                    "cache_delivery_contract": {
                        "retrieval_trust_evidence": {
                            "source": RETRIEVAL_TRUST_SOURCE_RUNTIME_CONSUMPTION_SNAPSHOT_INDEX,
                            "source_registry_ref": RETRIEVAL_TRUST_SOURCE_REGISTRY_REF_RUNTIME_CONSUMPTION_FINAL,
                            "citation": "/tmp/project/runtime-consumption/final-recorded.json",
                            "freshness": "final",
                            "freshness_posture": RETRIEVAL_TRUST_FRESHNESS_POSTURE_LATEST_FINAL_SNAPSHOT,
                            "acl": "protocol-binding-receipt-2",
                            "acl_context": "protocol_binding_receipt:protocol-binding-receipt-2",
                            "acl_propagation": RETRIEVAL_TRUST_ACL_PROPAGATION_PROTOCOL_BINDING_GATE
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("bundle-check snapshot should be writable");
        let runtime_consumption = RuntimeConsumptionSummary {
            total_snapshots: 2,
            bundle_snapshots: 0,
            bundle_check_snapshots: 1,
            final_snapshots: 1,
            latest_kind: Some("bundle-check".to_string()),
            latest_snapshot_path: Some(runtime_consumption_snapshot_path_string(&snapshot_path)),
        };

        let signal = latest_admissible_retrieval_trust_signal(
            &runtime_consumption,
            None,
            Some("protocol-binding-receipt-2"),
        )
        .expect("latest passing bundle-check snapshot should publish retrieval trust");

        assert_eq!(
            signal["citation"],
            "/tmp/project/runtime-consumption/final-recorded.json"
        );
        assert_eq!(signal["acl"], "protocol-binding-receipt-2");

        let stale_signal = latest_admissible_retrieval_trust_signal(
            &runtime_consumption,
            None,
            Some("protocol-binding-receipt-3"),
        );
        assert!(
            stale_signal.is_none(),
            "bundle-check retrieval trust must not survive protocol-binding ACL drift"
        );

        let _ = fs::remove_dir_all(root);
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
                    "closure_admission": case10_closure_admission_record()
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
        assert_eq!(
            selected,
            runtime_consumption_snapshot_path_string(&valid_path)
        );

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
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {},
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": case10_closure_admission_record()
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
                        "proof_surfaces": ["vida taskflow consume final"],
                        "evidence_table": [{
                            "requirement": "closure_admission",
                            "status": "blocked",
                            "evidence_refs": ["vida taskflow consume final"],
                            "blockers": ["missing_retrieval_trust_evidence"]
                        }]
                    }
                }
            })
            .to_string(),
        )
        .expect("blocked final snapshot should be writable");

        let selected = latest_final_runtime_consumption_snapshot_path(&root)
            .expect("latest admissible final snapshot should resolve")
            .expect("one admissible final snapshot should be available");
        assert_eq!(
            selected,
            runtime_consumption_snapshot_path_string(&admissible_path)
        );

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

        thread::sleep(Duration::from_millis(5));

        let deferred_handoff_path = runtime_dir.join("final-deferred-handoff-continue.json");
        fs::write(
            &deferred_handoff_path,
            serde_json::json!({
                "surface": "vida taskflow consume continue",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": ["inspect lane"],
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": ["inspect lane"],
                    "artifact_refs": {
                        "latest_run_graph_dispatch_receipt_id": "run-deferred-handoff"
                    }
                },
                "projection_truth": {
                    "projection_source": "deferred_agent_handoff_receipt"
                }
            })
            .to_string(),
        )
        .expect("deferred handoff continue snapshot should be writable");

        let run_id = latest_terminal_consume_continue_snapshot_run_id(&root)
            .expect("terminal continue lookup should succeed")
            .expect("deferred handoff continue run id should resolve");
        assert_eq!(run_id, "run-deferred-handoff");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_consumption_snapshot_release_admission_accepts_payload_closure_admission() {
        let snapshot = serde_json::json!({
            "surface": "vida taskflow consume final",
            "status": "pass",
            "blocker_codes": [],
            "next_actions": [],
            "shared_fields": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {}
            },
            "operator_contracts": {
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {}
            },
            "payload": {
                "closure_admission": case10_closure_admission_record()
            }
        });

        assert!(runtime_consumption_snapshot_has_release_admission_evidence(
            &snapshot
        ));
    }

    #[test]
    fn release_admission_operator_evidence_incomplete_accepts_recorded_final_with_evidence() {
        let root = std::env::temp_dir().join(format!(
            "vida-release-admission-evidence-complete-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let final_path = runtime_dir.join("final-recorded-with-release-admission.json");
        fs::write(
            &final_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {},
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": case10_closure_admission_record()
                }
            })
            .to_string(),
        )
        .expect("final snapshot should be writable");

        assert!(!release_admission_operator_evidence_incomplete(&root)
            .expect("release-admission evidence check should succeed"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn release_admission_operator_evidence_incomplete_rejects_parity_error_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "vida-release-admission-evidence-parity-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let final_path = runtime_dir.join("final-recorded-with-parity-error.json");
        fs::write(
            &final_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "artifact_refs": {},
                "operator_contracts": {
                    "status": "blocked",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": case10_closure_admission_record()
                }
            })
            .to_string(),
        )
        .expect("final snapshot should be writable");

        assert!(release_admission_operator_evidence_incomplete(&root)
            .expect("release-admission evidence check should succeed"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn release_admission_operator_evidence_incomplete_rejects_newest_final_without_evidence() {
        let root = std::env::temp_dir().join(format!(
            "vida-release-admission-evidence-stale-bypass-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");

        let older_admissible_path = runtime_dir.join("final-2026-05-16T12-00-00Z.json");
        fs::write(
            &older_admissible_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": case10_closure_admission_record()
                }
            })
            .to_string(),
        )
        .expect("older admissible final snapshot should be writable");

        let newer_incomplete_path = runtime_dir.join("final-2026-05-16T12-00-01Z.json");
        fs::write(
            &newer_incomplete_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                }
            })
            .to_string(),
        )
        .expect("newer incomplete final snapshot should be writable");

        assert!(release_admission_operator_evidence_incomplete(&root)
            .expect("release-admission evidence check should succeed"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn release_admission_operator_evidence_complete_for_run_requires_matching_run_id() {
        let root = std::env::temp_dir().join(format!(
            "vida-release-admission-evidence-run-match-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be monotonic enough for test ids")
                .as_nanos()
        ));
        let runtime_dir = root.join("runtime-consumption");
        fs::create_dir_all(&runtime_dir).expect("runtime-consumption dir should exist");
        let final_path = runtime_dir.join("final-recorded-with-release-admission.json");
        fs::write(
            &final_path,
            serde_json::json!({
                "surface": "vida taskflow consume final",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "source_run_id": "run-allowed",
                "operator_contracts": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "shared_fields": {
                    "status": "pass",
                    "blocker_codes": [],
                    "next_actions": [],
                    "artifact_refs": {}
                },
                "payload": {
                    "closure_admission": case10_closure_admission_record()
                }
            })
            .to_string(),
        )
        .expect("final snapshot should be writable");

        assert!(
            release_admission_operator_evidence_complete_for_run(&root, "run-allowed")
                .expect("release-admission run match check should succeed")
        );
        assert!(
            !release_admission_operator_evidence_complete_for_run(&root, "run-other")
                .expect("release-admission run mismatch check should succeed")
        );

        let _ = fs::remove_dir_all(root);
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
            policy_bundle_ref: None,
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
        store
            .persist_task_record(test_task_record("task-final", "open"))
            .await
            .expect("seed task authority");

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
