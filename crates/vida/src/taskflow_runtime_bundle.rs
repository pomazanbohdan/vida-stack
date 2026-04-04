use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

use crate::{
    build_project_activator_view, doctor_launcher_summary_for_root,
    merge_project_activation_into_init_view, read_or_sync_launcher_activation_snapshot,
    DoctorLauncherSummary, StateStore, TaskflowConsumeBundleCheck, TaskflowConsumeBundlePayload,
    TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
};

use super::activation_status::{activation_status_is_pending, canonical_activation_status};

const PROJECT_STARTUP_PROTOCOL_SURFACES: [(&str, &str, &str, &str); 4] = [
    (
        "project_orchestrator_startup_bundle",
        "docs/process/project-orchestrator-startup-bundle.md",
        "always_on_core",
        "orchestrator-init",
    ),
    (
        "project_packet_and_lane_runtime_capsule",
        "docs/process/project-packet-and-lane-runtime-capsule.md",
        "lane_bundle",
        "orchestrator-init",
    ),
    (
        "project_start_readiness_runtime_capsule",
        "docs/process/project-start-readiness-runtime-capsule.md",
        "lane_bundle",
        "orchestrator-init",
    ),
    (
        "project_packet_rendering_runtime_capsule",
        "docs/process/project-packet-rendering-runtime-capsule.md",
        "lane_bundle",
        "orchestrator-init",
    ),
];
const RETRIEVAL_OPTIONAL_CONTEXT_BOUNDARY_REQUIRED: [&str; 3] = [
    "full_project_owner_protocols",
    "non_promoted_project_docs",
    "broad_repo_manual_scan",
];

pub(crate) async fn build_taskflow_consume_bundle_payload(
    store: &StateStore,
) -> Result<TaskflowConsumeBundlePayload, String> {
    let activation_snapshot = read_or_sync_launcher_activation_snapshot(store).await?;
    let config_path = PathBuf::from(&activation_snapshot.source_config_path);
    let vida_root = config_path.parent().ok_or_else(|| {
        format!(
            "Launcher activation snapshot config path has no parent: {}",
            activation_snapshot.source_config_path
        )
    })?;
    let launcher_runtime_paths = doctor_launcher_summary_for_root(vida_root)
        .map_err(|error| format!("Failed to resolve launcher/runtime paths: {error}"))?;
    let root_artifact_id = store
        .active_instruction_root()
        .await
        .map_err(|error| format!("Failed to read active instruction root: {error}"))?;
    let effective_instruction_bundle = store
        .inspect_effective_instruction_bundle(&root_artifact_id)
        .await
        .map_err(|error| format!("Failed to inspect effective instruction bundle: {error}"))?;
    let boot_compatibility = store
        .latest_boot_compatibility_summary()
        .await
        .map_err(|error| format!("Failed to read boot compatibility summary: {error}"))?
        .ok_or_else(|| {
            "Boot compatibility summary is missing from the authoritative state store.".to_string()
        })?;
    let migration_preflight = store
        .latest_migration_preflight_summary()
        .await
        .map_err(|error| format!("Failed to read migration preflight summary: {error}"))?
        .ok_or_else(|| {
            "Migration preflight summary is missing from the authoritative state store.".to_string()
        })?;
    let task_store = store
        .task_store_summary()
        .await
        .map_err(|error| format!("Failed to read task store summary: {error}"))?;
    let run_graph = store
        .run_graph_summary()
        .await
        .map_err(|error| format!("Failed to read run graph summary: {error}"))?;
    let protocol_binding_receipt = store
        .latest_protocol_binding_receipt()
        .await
        .map_err(|error| format!("Failed to read protocol binding receipt: {error}"))?;
    let protocol_binding_rows = store
        .latest_protocol_binding_rows()
        .await
        .map_err(|error| format!("Failed to read protocol binding rows: {error}"))?;
    let protocol_binding_cache_token = store
        .latest_protocol_binding_cache_token()
        .await
        .map_err(|error| format!("Failed to read protocol binding cache token: {error}"))?
        .unwrap_or_default();
    let compiled_payload_import_evidence =
        crate::taskflow_protocol_binding::protocol_binding_compiled_payload_import_evidence(store)
            .await;
    let protocol_binding_ready = protocol_binding_receipt.is_some()
        && !protocol_binding_rows.is_empty()
        && compiled_payload_import_evidence.imported
        && compiled_payload_import_evidence.trusted
        && protocol_binding_rows
            .iter()
            .all(|row| row.binding_status == "fully-runtime-bound" && row.blockers.is_empty());
    let protocol_binding_registry = serde_json::json!({
        "primary_state_authority": protocol_binding_receipt
            .as_ref()
            .map(|receipt| receipt.primary_state_authority.clone())
            .unwrap_or_else(|| TASKFLOW_PROTOCOL_BINDING_AUTHORITY.to_string()),
        "receipt_id": protocol_binding_receipt
            .as_ref()
            .map(|receipt| receipt.receipt_id.clone())
            .unwrap_or_default(),
        "binding_status": if protocol_binding_ready {
            "bound"
        } else {
            "blocked"
        },
        "compiled_payload_import_evidence": compiled_payload_import_evidence,
        "protocols": protocol_binding_rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "protocol_id": row.protocol_id,
                    "activation_class": row.activation_class,
                    "runtime_owner": row.runtime_owner,
                    "enforcement_type": row.enforcement_type,
                    "proof_surface": row.proof_surface,
                    "binding_status": row.binding_status,
                    "blockers": row.blockers,
                })
            })
            .collect::<Vec<_>>(),
    });
    let control_core = serde_json::json!({
        "root_artifact_id": effective_instruction_bundle.root_artifact_id,
        "mandatory_chain_order": effective_instruction_bundle.mandatory_chain_order,
        "source_version_tuple": effective_instruction_bundle.source_version_tuple,
        "receipt_id": effective_instruction_bundle.receipt_id,
        "artifact_count": effective_instruction_bundle.projected_artifacts.len(),
    });
    let project_protocol_projections = build_project_protocol_projections(vida_root);
    let project_activation_view = build_project_activator_view(vida_root);
    let mut activation_bundle = activation_snapshot.compiled_bundle.clone();
    if let serde_json::Value::Object(bundle) = &mut activation_bundle {
        bundle.insert(
            "project_protocol_projections".to_string(),
            project_protocol_projections.clone(),
        );
    }
    let metadata = serde_json::json!({
        "bundle_id": format!(
            "taskflow-runtime-bundle-{}",
            activation_snapshot.source_config_digest
        ),
        "bundle_schema_version": "release-1-v1",
        "framework_revision": control_core["source_version_tuple"]
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default(),
        "project_activation_revision": activation_snapshot.source_config_digest,
        "protocol_binding_revision": protocol_binding_receipt
            .as_ref()
            .map(|receipt| receipt.recorded_at.clone())
            .unwrap_or_default(),
        "protocol_binding_cache_token": protocol_binding_cache_token,
        "compiled_at": activation_snapshot.captured_at,
        "binding_status": protocol_binding_registry["binding_status"]
            .as_str()
            .unwrap_or("blocked"),
    });
    let startup_bundle_revision = project_protocol_projections["startup_bundle"]
        .get("artifact_revision")
        .cloned()
        .unwrap_or(serde_json::Value::String(String::new()));
    let cache_delivery_contract = serde_json::json!({
        "always_on_core": control_core["mandatory_chain_order"],
        "project_startup_bundle": ["activation_bundle.project_protocol_projections.startup_bundle"],
        "project_runtime_capsules": ["activation_bundle.project_protocol_projections.startup_capsules"],
        "lane_bundle": ["activation_bundle"],
        "triggered_domain_bundle": ["protocol_binding_registry"],
        "task_specific_dynamic_context": ["task_store", "run_graph"],
        "invalidation_tuple": {
            "framework_revision": metadata["framework_revision"],
            "project_activation_revision": activation_snapshot.source_config_digest,
            "protocol_binding_revision": metadata["protocol_binding_revision"],
            "protocol_binding_cache_token": metadata["protocol_binding_cache_token"],
            "startup_bundle_revision": startup_bundle_revision,
        },
        "retrieval_only_optional_context_boundary": [
            "full_project_owner_protocols",
            "non_promoted_project_docs",
            "broad_repo_manual_scan"
        ],
        "cache_key_inputs": {
            "source_version_tuple": control_core["source_version_tuple"],
            "project_activation_revision": activation_snapshot.source_config_digest,
            "protocol_binding_revision": metadata["protocol_binding_revision"],
            "protocol_binding_cache_token": metadata["protocol_binding_cache_token"],
            "startup_bundle_revision": startup_bundle_revision,
        },
        "retrieval_trust_evidence": {
            "source": "taskflow_runtime_bundle",
            "citation": activation_snapshot.source_config_path,
            "freshness": metadata["compiled_at"],
            "acl": protocol_binding_registry["receipt_id"],
        },
    });
    let orchestrator_init_view = merge_project_activation_into_init_view(
        build_orchestrator_init_view(
            vida_root,
            &control_core,
            &project_protocol_projections,
            &protocol_binding_registry,
            &cache_delivery_contract,
            &boot_compatibility.classification,
            &migration_preflight.migration_state,
        ),
        &project_activation_view,
    );
    let agent_init_view = merge_project_activation_into_init_view(
        build_agent_init_view(
            vida_root,
            &activation_bundle,
            &project_protocol_projections,
            &protocol_binding_registry,
            &boot_compatibility.classification,
            &migration_preflight.migration_state,
        ),
        &project_activation_view,
    );

    Ok(TaskflowConsumeBundlePayload {
        artifact_name: "taskflow_runtime_bundle".to_string(),
        artifact_type: "runtime_bundle".to_string(),
        generated_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        vida_root: vida_root.display().to_string(),
        config_path: activation_snapshot.source_config_path.clone(),
        activation_source: activation_snapshot.source.clone(),
        launcher_runtime_paths,
        metadata,
        control_core,
        activation_bundle,
        protocol_binding_registry,
        cache_delivery_contract,
        orchestrator_init_view,
        agent_init_view,
        boot_compatibility: serde_json::json!({
            "classification": super::release1_contracts::canonical_compatibility_class_str(
                &boot_compatibility.classification
            )
            .unwrap_or(super::release1_contracts::CompatibilityClass::ReaderUpgradeRequired.as_str()),
            "reasons": boot_compatibility.reasons,
            "next_step": boot_compatibility.next_step,
        }),
        migration_preflight: serde_json::json!({
            "compatibility_classification": super::release1_contracts::canonical_compatibility_class_str(
                &migration_preflight.compatibility_classification
            )
            .unwrap_or(super::release1_contracts::CompatibilityClass::ReaderUpgradeRequired.as_str()),
            "migration_state": migration_preflight.migration_state,
            "blockers": migration_preflight.blockers,
            "source_version_tuple": migration_preflight.source_version_tuple,
            "next_step": migration_preflight.next_step,
        }),
        task_store: serde_json::json!({
            "total_count": task_store.total_count,
            "open_count": task_store.open_count,
            "in_progress_count": task_store.in_progress_count,
            "closed_count": task_store.closed_count,
            "epic_count": task_store.epic_count,
            "ready_count": task_store.ready_count,
        }),
        run_graph: serde_json::json!({
            "execution_plan_count": run_graph.execution_plan_count,
            "routed_run_count": run_graph.routed_run_count,
            "governance_count": run_graph.governance_count,
            "resumability_count": run_graph.resumability_count,
            "reconciliation_count": run_graph.reconciliation_count,
        }),
    })
}

pub(crate) fn taskflow_consume_bundle_check(
    payload: &TaskflowConsumeBundlePayload,
) -> TaskflowConsumeBundleCheck {
    let mut blockers = Vec::new();
    let root_artifact_id = payload
        .control_core
        .get("root_artifact_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let artifact_count = payload
        .control_core
        .get("artifact_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize;
    let boot_classification = payload
        .boot_compatibility
        .get("classification")
        .and_then(serde_json::Value::as_str)
        .and_then(super::release1_contracts::canonical_compatibility_class_str)
        .unwrap_or(super::release1_contracts::CompatibilityClass::ReaderUpgradeRequired.as_str())
        .to_string();
    let migration_state = payload
        .migration_preflight
        .get("migration_state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let next_step = payload
        .migration_preflight
        .get("next_step")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let bundle_order = payload
        .control_core
        .get("mandatory_chain_order")
        .and_then(serde_json::Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    if !payload
        .control_core
        .as_object()
        .is_some_and(|control_core| {
            control_core
                .keys()
                .all(|key| is_canonical_release1_control_core_key(key))
        })
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::InvalidControlCoreKeys,
            )
            .to_string(),
        );
    }
    let metadata = &payload.metadata;

    if root_artifact_id.is_empty() {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingRootArtifactId,
            )
            .to_string(),
        );
    }
    if bundle_order == 0 {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingMandatoryChainOrder,
            )
            .to_string(),
        );
    }
    if artifact_count == 0 {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingEffectiveBundleArtifacts,
            )
            .to_string(),
        );
    }
    if metadata
        .get("bundle_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .is_empty()
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingBundleId,
            )
            .to_string(),
        );
    }
    if metadata
        .get("bundle_schema_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .is_empty()
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingBundleSchemaVersion,
            )
            .to_string(),
        );
    }
    if boot_classification
        != super::release1_contracts::CompatibilityClass::BackwardCompatible.as_str()
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::BootCompatibilityNotCompatible,
            )
            .to_string(),
        );
    }
    if migration_state != "no_migration_required" || next_step != "normal_boot_allowed" {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MigrationPreflightNotReady,
            )
            .to_string(),
        );
    }
    if payload.vida_root != payload.launcher_runtime_paths.project_root {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MixedRuntimeRoot,
            )
            .to_string(),
        );
    }
    if payload.config_path != expected_config_path(&payload.vida_root) {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MixedConfigPath,
            )
            .to_string(),
        );
    }
    for (value, family) in [
        (&payload.metadata, "metadata"),
        (&payload.control_core, "control_core"),
        (&payload.activation_bundle, "activation_bundle"),
        (
            &payload.protocol_binding_registry,
            "protocol_binding_registry",
        ),
        (&payload.cache_delivery_contract, "cache_delivery_contract"),
        (&payload.orchestrator_init_view, "orchestrator_init_view"),
        (&payload.agent_init_view, "agent_init_view"),
    ] {
        if !value.is_object() {
            blockers.push(format!("missing_{family}_family"));
        }
    }
    let protocol_rows = payload
        .protocol_binding_registry
        .get("protocols")
        .and_then(serde_json::Value::as_array);
    if protocol_rows.map(|rows| rows.is_empty()).unwrap_or(true) {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingProtocolBindingRows,
            )
            .to_string(),
        );
    }
    blockers.extend(protocol_binding_registry_contract_blockers(
        &payload.protocol_binding_registry,
    ));
    if !payload
        .protocol_binding_registry
        .as_object()
        .is_some_and(|protocol_binding_registry| {
            protocol_binding_registry
                .keys()
                .all(|key| is_canonical_release1_protocol_binding_registry_key(key))
        })
    {
        blockers.push("invalid_protocol_binding_registry_keys".to_string());
    }
    let compiled_payload_import_evidence =
        &payload.protocol_binding_registry["compiled_payload_import_evidence"];
    if !compiled_payload_import_evidence
        .get("imported")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || !compiled_payload_import_evidence
            .get("trusted")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingAuthoritativeProtocolBindingImportEvidence,
            )
            .to_string(),
        );
    }
    if protocol_rows.map(|rows| {
        rows.iter().all(|row| {
            row.get("binding_status")
                .and_then(serde_json::Value::as_str)
                == Some("fully-runtime-bound")
                && row
                    .get("blockers")
                    .and_then(serde_json::Value::as_array)
                    .map(|blockers| blockers.is_empty())
                    .unwrap_or(false)
        })
    }) != Some(true)
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::ProtocolBindingRowsNotRuntimeTrusted,
            )
            .to_string(),
        );
    }
    if payload
        .cache_delivery_contract
        .get("cache_key_inputs")
        .and_then(serde_json::Value::as_object)
        .is_none()
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingCacheKeyInputs,
            )
            .to_string(),
        );
    }
    if payload
        .cache_delivery_contract
        .get("invalidation_tuple")
        .and_then(serde_json::Value::as_object)
        .is_none()
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::MissingInvalidationTuple,
            )
            .to_string(),
        );
    }
    blockers.extend(cache_contract_consistency_blockers(payload));
    blockers.extend(cache_registry_contract_blockers(payload));
    blockers.extend(retrieval_optional_context_boundary_blockers(
        &payload.cache_delivery_contract,
    ));
    blockers.extend(retrieval_trust_evidence_blockers(
        &payload.cache_delivery_contract,
    ));
    let orchestrator_activation_pending =
        init_view_activation_is_pending(&payload.orchestrator_init_view);
    let agent_activation_pending = init_view_activation_is_pending(&payload.agent_init_view);
    if orchestrator_activation_pending || agent_activation_pending {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::ActivationPending,
            )
            .to_string(),
        );
    }
    if payload
        .orchestrator_init_view
        .get("execution_gate")
        .and_then(|value| value.get("taskflow_admitted"))
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        blockers.push(
            super::release1_contracts::blocker_code_str(
                super::release1_contracts::BlockerCode::TaskflowBlockedDuringPendingActivation,
            )
            .to_string(),
        );
    }
    let activation_pending = orchestrator_activation_pending
        || agent_activation_pending
        || project_activation_truth_is_pending(payload).unwrap_or(false);
    let activation_status = canonical_activation_status(None, activation_pending).to_string();

    TaskflowConsumeBundleCheck {
        ok: blockers.is_empty(),
        blockers,
        root_artifact_id,
        artifact_count,
        boot_classification,
        migration_state,
        activation_status,
    }
}

fn project_activation_truth_is_pending(payload: &TaskflowConsumeBundlePayload) -> Option<bool> {
    let project_root = activation_truth_project_root(payload, std::env::var("VIDA_ROOT").ok());
    let truth = crate::project_activator_surface::canonical_project_activation_status_truth(
        Path::new(&project_root),
    );
    Some(truth.activation_pending || truth.status == "pending")
}

fn activation_truth_project_root(
    payload: &TaskflowConsumeBundlePayload,
    vida_root_override: Option<String>,
) -> String {
    let launcher_root = payload.launcher_runtime_paths.project_root.trim();
    let payload_root = payload.vida_root.trim();
    let fallback_root = if launcher_root.is_empty() {
        payload_root
    } else {
        launcher_root
    };

    let Some(override_root) = vida_root_override
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return fallback_root.to_string();
    };

    if runtime_roots_equivalent(fallback_root, &override_root) {
        return override_root;
    }

    fallback_root.to_string()
}

fn runtime_roots_equivalent(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    match (
        std::fs::canonicalize(Path::new(left)),
        std::fs::canonicalize(Path::new(right)),
    ) {
        (Ok(left_canonical), Ok(right_canonical)) => left_canonical == right_canonical,
        _ => false,
    }
}

fn cache_contract_consistency_blockers(payload: &TaskflowConsumeBundlePayload) -> Vec<String> {
    let mut blockers = Vec::new();
    let cache_key_inputs = payload
        .cache_delivery_contract
        .get("cache_key_inputs")
        .and_then(serde_json::Value::as_object);
    let invalidation_tuple = payload
        .cache_delivery_contract
        .get("invalidation_tuple")
        .and_then(serde_json::Value::as_object);

    let Some(cache_key_inputs) = cache_key_inputs else {
        return blockers;
    };
    let Some(invalidation_tuple) = invalidation_tuple else {
        return blockers;
    };

    if !cache_key_inputs
        .keys()
        .all(|key| is_canonical_release1_cache_key_input_key(key))
    {
        blockers.push("invalid_cache_key_inputs_keys".to_string());
    }
    if !invalidation_tuple
        .keys()
        .all(|key| is_canonical_release1_invalidation_tuple_key(key))
    {
        blockers.push("invalid_invalidation_tuple_keys".to_string());
    }

    let cache_required = [
        "source_version_tuple",
        "project_activation_revision",
        "protocol_binding_revision",
        "protocol_binding_cache_token",
        "startup_bundle_revision",
    ];
    for key in cache_required {
        if !cache_key_inputs.contains_key(key) {
            blockers.push(format!("missing_cache_key_input:{key}"));
        }
    }

    let invalidation_required = [
        "framework_revision",
        "project_activation_revision",
        "protocol_binding_revision",
        "protocol_binding_cache_token",
        "startup_bundle_revision",
    ];
    for key in invalidation_required {
        if !invalidation_tuple.contains_key(key) {
            blockers.push(format!("missing_invalidation_tuple_key:{key}"));
        }
    }

    if cache_key_inputs
        .get("source_version_tuple")
        .and_then(serde_json::Value::as_array)
        .map(|rows| rows.is_empty() || !rows.iter().all(is_canonical_release1_tuple_entry))
        .unwrap_or(true)
    {
        blockers.push("invalid_cache_key_input:source_version_tuple".to_string());
    }
    for key in [
        "project_activation_revision",
        "protocol_binding_revision",
        "protocol_binding_cache_token",
        "startup_bundle_revision",
    ] {
        if !cache_key_inputs
            .get(key)
            .is_some_and(is_canonical_nonempty_string)
        {
            blockers.push(format!("invalid_cache_key_input:{key}"));
        }
    }
    for key in [
        "framework_revision",
        "project_activation_revision",
        "protocol_binding_revision",
        "protocol_binding_cache_token",
        "startup_bundle_revision",
    ] {
        if !invalidation_tuple
            .get(key)
            .is_some_and(is_canonical_nonempty_string)
        {
            blockers.push(format!("invalid_invalidation_tuple_key:{key}"));
        }
    }

    let metadata = payload.metadata.as_object();
    if let Some(metadata) = metadata {
        if !metadata
            .keys()
            .all(|key| is_canonical_release1_metadata_key(key))
        {
            blockers.push("invalid_metadata_tuple_keys".to_string());
        }
        for key in [
            "framework_revision",
            "project_activation_revision",
            "protocol_binding_revision",
            "protocol_binding_cache_token",
        ] {
            if !metadata.contains_key(key) {
                blockers.push(format!("missing_metadata_tuple_key:{key}"));
            }
            if !metadata.get(key).is_some_and(is_canonical_nonempty_string) {
                blockers.push(format!("invalid_metadata_tuple_key:{key}"));
            }
        }
        for (left, right, blocker_code) in [
            (
                metadata.get("project_activation_revision"),
                cache_key_inputs.get("project_activation_revision"),
                "cache_key_mismatch:project_activation_revision",
            ),
            (
                metadata.get("protocol_binding_revision"),
                cache_key_inputs.get("protocol_binding_revision"),
                "cache_key_mismatch:protocol_binding_revision",
            ),
            (
                metadata.get("protocol_binding_cache_token"),
                cache_key_inputs.get("protocol_binding_cache_token"),
                "cache_key_mismatch:protocol_binding_cache_token",
            ),
            (
                metadata.get("framework_revision"),
                invalidation_tuple.get("framework_revision"),
                "invalidation_tuple_mismatch:framework_revision",
            ),
            (
                cache_key_inputs.get("project_activation_revision"),
                invalidation_tuple.get("project_activation_revision"),
                "invalidation_tuple_mismatch:project_activation_revision",
            ),
            (
                cache_key_inputs.get("protocol_binding_revision"),
                invalidation_tuple.get("protocol_binding_revision"),
                "invalidation_tuple_mismatch:protocol_binding_revision",
            ),
            (
                cache_key_inputs.get("protocol_binding_cache_token"),
                invalidation_tuple.get("protocol_binding_cache_token"),
                "invalidation_tuple_mismatch:protocol_binding_cache_token",
            ),
            (
                cache_key_inputs.get("startup_bundle_revision"),
                invalidation_tuple.get("startup_bundle_revision"),
                "invalidation_tuple_mismatch:startup_bundle_revision",
            ),
        ] {
            if left.is_some() && right.is_some() && left != right {
                blockers.push(blocker_code.to_string());
            }
        }
    }
    let protocol_binding_revision = cache_key_inputs
        .get("protocol_binding_revision")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let protocol_binding_cache_token = cache_key_inputs
        .get("protocol_binding_cache_token")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let protocol_binding_receipt_id = payload
        .protocol_binding_registry
        .get("receipt_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let protocol_binding_status = payload
        .protocol_binding_registry
        .get("binding_status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("blocked");
    if !protocol_binding_revision.is_empty()
        && (protocol_binding_receipt_id.is_empty() || protocol_binding_status != "bound")
    {
        blockers.push("cache_tuple_protocol_binding_evidence_untrusted".to_string());
    }
    if !protocol_binding_revision.is_empty()
        && !protocol_binding_cache_token.contains(protocol_binding_receipt_id)
    {
        blockers.push("cache_tuple_protocol_binding_token_mismatch".to_string());
    }

    blockers.sort();
    blockers.dedup();
    blockers
}

fn cache_registry_contract_blockers(payload: &TaskflowConsumeBundlePayload) -> Vec<String> {
    let Some(triggered_domain_bundle) = payload
        .cache_delivery_contract
        .get("triggered_domain_bundle")
        .and_then(serde_json::Value::as_array)
    else {
        return vec!["missing_triggered_domain_bundle_partition".to_string()];
    };

    let includes_registry_contract = triggered_domain_bundle
        .iter()
        .any(|entry| entry.as_str() == Some("protocol_binding_registry"));
    if includes_registry_contract {
        Vec::new()
    } else {
        vec!["cache_registry_contract_missing_triggered_domain_binding".to_string()]
    }
}

fn retrieval_optional_context_boundary_blockers(
    cache_delivery_contract: &serde_json::Value,
) -> Vec<String> {
    let Some(rows) = cache_delivery_contract
        .get("retrieval_only_optional_context_boundary")
        .and_then(serde_json::Value::as_array)
    else {
        return vec!["missing_retrieval_only_optional_context_boundary".to_string()];
    };
    if rows.is_empty() {
        return vec!["missing_retrieval_only_optional_context_boundary".to_string()];
    }
    let values = rows
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let mut blockers = RETRIEVAL_OPTIONAL_CONTEXT_BOUNDARY_REQUIRED
        .iter()
        .filter(|required| !values.contains(**required))
        .map(|required| format!("missing_retrieval_optional_boundary_entry:{required}"))
        .collect::<Vec<_>>();
    blockers.sort();
    blockers
}

fn retrieval_trust_evidence_blockers(cache_delivery_contract: &serde_json::Value) -> Vec<String> {
    let Some(evidence) = cache_delivery_contract
        .get("retrieval_trust_evidence")
        .and_then(serde_json::Value::as_object)
    else {
        return vec!["missing_retrieval_trust_evidence".to_string()];
    };

    let mut blockers = Vec::new();
    for key in ["source", "citation", "freshness", "acl"] {
        if !evidence.get(key).is_some_and(is_canonical_nonempty_string) {
            blockers.push(format!("missing_retrieval_trust_evidence_field:{key}"));
        }
    }
    blockers.sort();
    blockers
}

fn canonical_project_protocol_projection_status(status: Option<&str>) -> &'static str {
    match status.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "compiled" => "compiled",
        Some(value) if value == "blocked" => "blocked",
        Some(value) if value == "validated" => "validated",
        Some(value) if value == "executable" => "executable",
        Some(value) if value == "present" => "present",
        Some(value) if value == "missing" => "missing",
        _ => "blocked",
    }
}

fn is_canonical_nonempty_string(value: &serde_json::Value) -> bool {
    value
        .as_str()
        .map(|entry| {
            let trimmed = entry.trim();
            !trimmed.is_empty() && trimmed == entry
        })
        .unwrap_or(false)
}

enum Release1StringFieldRule {
    CanonicalNonempty,
    ExactTrimmed(&'static str),
    CanonicalEnum(&'static [&'static str]),
}

fn release1_string_field_matches(value: Option<&str>, rule: Release1StringFieldRule) -> bool {
    let Some(value) = value.map(str::trim) else {
        return false;
    };
    match rule {
        Release1StringFieldRule::CanonicalNonempty => !value.is_empty(),
        Release1StringFieldRule::ExactTrimmed(expected) => value == expected,
        Release1StringFieldRule::CanonicalEnum(allowed) => {
            let normalized = value.to_ascii_lowercase();
            !normalized.is_empty() && allowed.contains(&normalized.as_str())
        }
    }
}

fn protocol_binding_registry_contract_blockers(
    protocol_binding_registry: &serde_json::Value,
) -> Vec<String> {
    let Some(protocol_binding_registry) = protocol_binding_registry.as_object() else {
        return Vec::new();
    };
    let mut blockers = Vec::new();
    if !release1_string_field_matches(
        protocol_binding_registry
            .get("receipt_id")
            .and_then(serde_json::Value::as_str),
        Release1StringFieldRule::CanonicalNonempty,
    ) {
        if let Some(code) = super::release1_contracts::blocker_code_value(
            super::release1_contracts::BlockerCode::MissingProtocolBindingReceipt,
        ) {
            blockers.push(code);
        }
    }
    if !release1_string_field_matches(
        protocol_binding_registry
            .get("primary_state_authority")
            .and_then(serde_json::Value::as_str),
        Release1StringFieldRule::ExactTrimmed(TASKFLOW_PROTOCOL_BINDING_AUTHORITY),
    ) {
        if let Some(code) = super::release1_contracts::blocker_code_value(
            super::release1_contracts::BlockerCode::OwnerSurfaceContradiction,
        ) {
            blockers.push(code);
        }
    }
    if !release1_string_field_matches(
        protocol_binding_registry
            .get("binding_status")
            .and_then(serde_json::Value::as_str),
        Release1StringFieldRule::CanonicalEnum(&["bound"]),
    ) {
        if let Some(code) = super::release1_contracts::blocker_code_value(
            super::release1_contracts::BlockerCode::ProtocolBindingNotRuntimeReady,
        ) {
            blockers.push(code);
        }
    }
    blockers
}

fn is_canonical_release1_cache_key_input_key(key: &str) -> bool {
    matches!(
        key,
        "source_version_tuple"
            | "project_activation_revision"
            | "protocol_binding_revision"
            | "protocol_binding_cache_token"
            | "startup_bundle_revision"
    )
}

fn is_canonical_release1_invalidation_tuple_key(key: &str) -> bool {
    matches!(
        key,
        "framework_revision"
            | "project_activation_revision"
            | "protocol_binding_revision"
            | "protocol_binding_cache_token"
            | "startup_bundle_revision"
    )
}

fn is_canonical_release1_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "bundle_id"
            | "bundle_schema_version"
            | "framework_revision"
            | "project_activation_revision"
            | "protocol_binding_revision"
            | "protocol_binding_cache_token"
            | "compiled_at"
            | "binding_status"
            | "error"
    )
}

fn is_canonical_release1_control_core_key(key: &str) -> bool {
    matches!(
        key,
        "root_artifact_id"
            | "mandatory_chain_order"
            | "source_version_tuple"
            | "receipt_id"
            | "artifact_count"
            | "error"
    )
}

fn is_canonical_release1_protocol_binding_registry_key(key: &str) -> bool {
    matches!(
        key,
        "primary_state_authority"
            | "receipt_id"
            | "binding_status"
            | "compiled_payload_import_evidence"
            | "protocols"
            | "error"
    )
}

fn is_canonical_release1_tuple_entry(value: &serde_json::Value) -> bool {
    value
        .as_str()
        .map(|entry| {
            let trimmed = entry.trim();
            !trimmed.is_empty() && trimmed == entry && trimmed == trimmed.to_ascii_lowercase()
        })
        .unwrap_or(false)
}

fn init_view_activation_is_pending(init_view: &serde_json::Value) -> bool {
    init_view
        .get("activation_pending")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || activation_status_is_pending(init_view.get("status").and_then(serde_json::Value::as_str))
}

pub(crate) fn blocking_runtime_bundle(error: &str) -> TaskflowConsumeBundlePayload {
    let current_exe = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "unresolved".to_string());
    let vida_root = std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "unresolved".to_string());
    TaskflowConsumeBundlePayload {
        artifact_name: "taskflow_runtime_bundle".to_string(),
        artifact_type: "runtime_bundle".to_string(),
        generated_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        vida_root: vida_root.clone(),
        config_path: expected_config_path(&vida_root),
        activation_source: "state_store".to_string(),
        launcher_runtime_paths: DoctorLauncherSummary {
            vida: current_exe,
            project_root: vida_root,
            taskflow_surface: "vida taskflow".to_string(),
        },
        metadata: serde_json::json!({
            "bundle_id": "",
            "bundle_schema_version": "release-1-v1",
            "framework_revision": "",
            "project_activation_revision": "",
            "protocol_binding_revision": "",
            "protocol_binding_cache_token": "",
            "compiled_at": time::OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .expect("rfc3339 timestamp should render"),
            "binding_status": "blocked",
            "error": error,
        }),
        control_core: serde_json::json!({
            "root_artifact_id": "",
            "mandatory_chain_order": [],
            "source_version_tuple": [],
            "receipt_id": "",
            "artifact_count": 0,
            "error": error,
        }),
        activation_bundle: serde_json::Value::Null,
        protocol_binding_registry: serde_json::json!({
            "primary_state_authority": TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
            "receipt_id": "",
            "binding_status": "blocked",
            "protocols": [],
            "error": error,
        }),
        cache_delivery_contract: serde_json::json!({
            "always_on_core": [],
            "project_startup_bundle": [],
            "project_runtime_capsules": [],
            "lane_bundle": [],
            "triggered_domain_bundle": [],
            "task_specific_dynamic_context": [],
            "invalidation_tuple": {},
            "retrieval_only_optional_context_boundary": [],
            "cache_key_inputs": {},
            "retrieval_trust_evidence": {},
            "error": error,
        }),
        orchestrator_init_view: serde_json::json!({
            "surface": "vida orchestrator-init",
            "status": "blocked",
            "error": error,
        }),
        agent_init_view: serde_json::json!({
            "surface": "vida agent-init",
            "status": "blocked",
            "error": error,
        }),
        boot_compatibility: serde_json::json!({
            "classification": "reader_upgrade_required",
            "reasons": [error],
            "next_step": "restore_runtime_bundle",
        }),
        migration_preflight: serde_json::json!({
            "compatibility_classification": "reader_upgrade_required",
            "migration_state": "blocked",
            "blockers": [error],
            "source_version_tuple": [],
            "next_step": "restore_runtime_bundle",
        }),
        task_store: serde_json::json!({
            "total_count": 0,
            "open_count": 0,
            "in_progress_count": 0,
            "closed_count": 0,
            "epic_count": 0,
            "ready_count": 0,
        }),
        run_graph: serde_json::json!({
            "execution_plan_count": 0,
            "routed_run_count": 0,
            "governance_count": 0,
            "resumability_count": 0,
            "reconciliation_count": 0,
        }),
    }
}

fn expected_config_path(vida_root: &str) -> String {
    Path::new(vida_root)
        .join("vida.config.yaml")
        .display()
        .to_string()
}

fn build_project_protocol_projections(vida_root: &Path) -> serde_json::Value {
    let protocols = PROJECT_STARTUP_PROTOCOL_SURFACES
        .iter()
        .map(
            |(protocol_id, relative_path, cache_partition, runtime_use_point)| {
                read_project_protocol_projection(
                    vida_root,
                    protocol_id,
                    relative_path,
                    cache_partition,
                    runtime_use_point,
                )
            },
        )
        .collect::<Vec<_>>();
    let startup_bundle = protocols
        .iter()
        .find(|row| row["protocol_id"] == "project_orchestrator_startup_bundle")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let startup_capsules = protocols
        .iter()
        .filter(|row| row["protocol_id"] != "project_orchestrator_startup_bundle")
        .cloned()
        .collect::<Vec<_>>();
    let compiled = protocols
        .iter()
        .filter(|row| row["promotion_state"]["stage"] == "executable")
        .cloned()
        .collect::<Vec<_>>();
    serde_json::json!({
        "status": if compiled.len() == protocols.len() { "compiled" } else { "blocked" },
        "known_project_protocols": protocols,
        "compiled_executable_project_protocols": compiled,
        "startup_bundle": startup_bundle,
        "startup_capsules": startup_capsules,
    })
}

fn read_project_protocol_projection(
    vida_root: &Path,
    protocol_id: &str,
    relative_path: &str,
    cache_partition: &str,
    runtime_use_point: &str,
) -> serde_json::Value {
    let path = vida_root.join(relative_path);
    let raw = std::fs::read_to_string(&path).ok();
    let metadata = raw
        .as_deref()
        .map(parse_markdown_artifact_metadata)
        .unwrap_or_default();
    let validated = raw.is_some()
        && metadata.contains_key("artifact_path")
        && metadata.contains_key("artifact_revision")
        && metadata
            .get("status")
            .map(|status| status == "canonical")
            .unwrap_or(false);
    let stage = if validated { "executable" } else { "validated" };
    serde_json::json!({
        "protocol_id": protocol_id,
        "source_path": relative_path,
        "artifact_path": metadata.get("artifact_path").cloned().unwrap_or_default(),
        "artifact_type": metadata.get("artifact_type").cloned().unwrap_or_default(),
        "artifact_revision": metadata.get("artifact_revision").cloned().unwrap_or_default(),
        "status": canonical_project_protocol_projection_status(
            metadata.get("status").map(String::as_str).or_else(|| if raw.is_some() { Some("present") } else { Some("missing") })
        ),
        "cache_partition": cache_partition,
        "runtime_use_point": runtime_use_point,
        "promotion_state": {
            "registered": raw.is_some(),
            "mapped": raw.is_some(),
            "validated": validated,
            "bound": validated,
            "compiled": validated,
            "executable": validated,
            "stage": stage,
        }
    })
}
fn parse_markdown_artifact_metadata(raw: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let Some((_, metadata)) = raw.split_once("\n-----\n") else {
        return map;
    };
    for line in metadata.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        map.insert(
            key.trim().to_string(),
            value.trim().trim_matches('\'').to_string(),
        );
    }
    map
}

pub(crate) fn build_orchestrator_init_view(
    vida_root: &Path,
    control_core: &serde_json::Value,
    project_protocol_projections: &serde_json::Value,
    protocol_binding_registry: &serde_json::Value,
    cache_delivery_contract: &serde_json::Value,
    boot_classification: &str,
    migration_state: &str,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida orchestrator-init",
        "status": init_status(boot_classification, migration_state, protocol_binding_registry),
        "local_runtime_surface": "vida orchestrator-init",
        "boot_surface": "vida boot",
        "framework_bootstrap": [
            "AGENTS.md",
            "AGENTS.sidecar.md",
            "vida/config/instructions/agent-definitions/entry.orchestrator-entry.md"
        ],
        "thinking_runtime_surface": "vida/config/instructions/instruction-contracts/overlay.step-thinking-runtime-capsule.md",
        "thinking_protocol_targets": [
            "instruction-contracts/overlay.step-thinking-runtime-capsule",
            "instruction-contracts/overlay.step-thinking-protocol#section-algorithm-selector"
        ],
        "allowed_thinking_modes": [
            "STC",
            "PR-CoT",
            "MAR",
            "5-SOL",
            "META"
        ],
        "mode_selection_rule": "select one thinking mode per step after request-intent classification; do not freeze one mode at bootstrap",
        "reporting_contract": {
            "required": true,
            "scope": "user-facing orchestrator progress and closure reports",
            "thinking_mode_prefix": "Thinking mode: <STC|PR-CoT|MAR|5-SOL|META>.",
            "request_counters_prefix": "Requests: active=<n> | in_work=<n> | blocked=<n>",
            "task_counters_prefix": "Tasks: active=<n> | in_work=<n> | blocked=<n>",
            "agent_counters_prefix": "Agents: active=<n> | working=<n> | waiting=<n>",
            "mode_selection_note": "the reporting label must reflect the selected per-step thinking mode but must not expose hidden reasoning"
        },
        "protocol_view_targets": [
            "bootstrap/router",
            "agent-definitions/entry.orchestrator-entry",
            "instruction-contracts/overlay.step-thinking-runtime-capsule",
            "system-maps/bootstrap.orchestrator-boot-flow"
        ],
        "project_startup_bundle": project_protocol_projections["startup_bundle"],
        "project_startup_capsules": project_protocol_projections["startup_capsules"],
        "required_cache_partitions": {
            "always_on_core": cache_delivery_contract["always_on_core"],
            "project_startup_bundle": cache_delivery_contract["project_startup_bundle"],
            "project_runtime_capsules": cache_delivery_contract["project_runtime_capsules"],
            "task_specific_dynamic_context": cache_delivery_contract["task_specific_dynamic_context"],
        },
        "minimum_commands": [
            "vida boot",
            "vida orchestrator-init --json",
            "vida protocol view bootstrap/router",
            "vida protocol view agent-definitions/entry.orchestrator-entry",
            "vida protocol view instruction-contracts/overlay.step-thinking-runtime-capsule",
            "vida task ready --json",
            "vida taskflow consume bundle check --json",
            "vida docflow protocol-coverage-check --profile active-canon"
        ],
        "feature_delivery_default_flow": {
            "documentation_first": true,
            "intake_runtime": "vida taskflow consume final <request> --json",
            "design_template_path": "docs/product/spec/templates/feature-design-document.template.md",
            "tracked_flow_order": [
                "open epic in vida taskflow",
                "open spec-pack task in vida taskflow",
                "initialize/finalize/check bounded design doc through vida docflow",
                "close spec-pack and shape work-pool/dev packet in vida taskflow",
                "delegate implementation through the configured development team"
            ],
            "design_flow_commands": [
                "vida docflow init docs/product/spec/<feature>-design.md product/spec/<feature>-design product_spec \"initialize feature design\"",
                "vida docflow finalize-edit docs/product/spec/<feature>-design.md \"record bounded feature design\"",
                "vida docflow check --root . docs/product/spec/<feature>-design.md"
            ],
            "post_design_execution_posture": [
                "shape one bounded execution packet from the design document",
                "delegate normal write-producing work through the configured development team",
                "keep the root session in orchestrator posture unless an explicit exception path is recorded"
            ]
        },
        "project_root": vida_root.display().to_string(),
        "root_artifact_id": control_core["root_artifact_id"],
    })
}

pub(crate) fn build_agent_init_view(
    vida_root: &Path,
    activation_bundle: &serde_json::Value,
    project_protocol_projections: &serde_json::Value,
    protocol_binding_registry: &serde_json::Value,
    boot_classification: &str,
    migration_state: &str,
) -> serde_json::Value {
    serde_json::json!({
        "surface": "vida agent-init",
        "status": init_status(boot_classification, migration_state, protocol_binding_registry),
        "local_runtime_surface": "vida agent-init",
        "worker_entry_contract": "vida/config/instructions/agent-definitions/entry.worker-entry.md",
        "worker_thinking_subset": "vida/config/instructions/instruction-contracts/role.worker-thinking.md",
        "thinking_protocol_targets": [
            "instruction-contracts/role.worker-thinking"
        ],
        "allowed_thinking_modes": [
            "STC",
            "PR-CoT",
            "MAR"
        ],
        "mode_selection_rule": "select one worker-safe thinking mode per step inside the assigned bounded scope; do not widen into orchestrator/meta reasoning without an explicit packet trigger",
        "reporting_contract": {
            "required": true,
            "scope": "worker-facing bounded status and completion reports",
            "thinking_mode_prefix": "Thinking mode: <STC|PR-CoT|MAR>.",
            "task_counters_prefix": "Tasks: active=<n> | in_work=<n> | blocked=<n>",
            "agent_counters_prefix": "Agents: active=<n> | working=<n> | waiting=<n>",
            "mode_selection_note": "the reporting label must reflect the selected worker-safe per-step thinking mode without exposing hidden reasoning"
        },
        "protocol_view_targets": [
            "agent-definitions/entry.worker-entry",
            "instruction-contracts/role.worker-thinking",
            "system-maps/bootstrap.worker-boot-flow"
        ],
        "minimum_commands": [
            "vida agent-init --role worker --json",
            "vida protocol view agent-definitions/entry.worker-entry",
            "vida protocol view instruction-contracts/role.worker-thinking",
            "vida task show <task-id> --json",
            "vida taskflow consume bundle check --json"
        ],
        "allowed_non_orchestrator_roles": non_orchestrator_roles(activation_bundle),
        "worker_lane_markers": [
            "worker_lane_confirmed: true",
            "lane_identity: worker"
        ],
        "project_runtime_capsules": project_protocol_projections["startup_capsules"],
        "project_root": vida_root.display().to_string(),
    })
}

fn init_status(
    boot_classification: &str,
    migration_state: &str,
    protocol_binding_registry: &serde_json::Value,
) -> &'static str {
    if super::release1_contracts::canonical_compatibility_class_str(boot_classification)
        == Some(super::release1_contracts::CompatibilityClass::BackwardCompatible.as_str())
        && migration_state == "no_migration_required"
        && protocol_binding_registry["binding_status"] == "bound"
    {
        "ready"
    } else {
        "degraded"
    }
}

fn non_orchestrator_roles(activation_bundle: &serde_json::Value) -> Vec<String> {
    let mut roles = activation_bundle["enabled_framework_roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .filter(|role| *role != "orchestrator")
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    roles.extend(
        activation_bundle["project_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["role_id"].as_str())
            .map(ToOwned::to_owned),
    );
    roles.sort();
    roles.dedup();
    roles
}

#[cfg(test)]
mod tests {
    use super::{
        activation_status_is_pending, activation_truth_project_root,
        cache_contract_consistency_blockers, canonical_project_protocol_projection_status,
        init_view_activation_is_pending, retrieval_optional_context_boundary_blockers,
        retrieval_trust_evidence_blockers, taskflow_consume_bundle_check,
        TaskflowConsumeBundlePayload,
    };
    use crate::TASKFLOW_PROTOCOL_BINDING_AUTHORITY;

    #[test]
    fn retrieval_optional_boundary_requires_all_canonical_entries() {
        let contract = serde_json::json!({
            "retrieval_only_optional_context_boundary": ["full_project_owner_protocols"]
        });
        let blockers = retrieval_optional_context_boundary_blockers(&contract);
        assert!(blockers.iter().any(
            |row| row == "missing_retrieval_optional_boundary_entry:non_promoted_project_docs"
        ));
        assert!(blockers
            .iter()
            .any(|row| row == "missing_retrieval_optional_boundary_entry:broad_repo_manual_scan"));
    }

    #[test]
    fn retrieval_optional_boundary_passes_with_canonical_entries() {
        let contract = serde_json::json!({
            "retrieval_only_optional_context_boundary": [
                "non_promoted_project_docs",
                "full_project_owner_protocols",
                "broad_repo_manual_scan"
            ]
        });
        let blockers = retrieval_optional_context_boundary_blockers(&contract);
        assert!(blockers.is_empty());
    }

    #[test]
    fn cache_contract_consistency_blocks_missing_required_keys() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.cache_delivery_contract = serde_json::json!({
            "cache_key_inputs": {
                "source_version_tuple": ["release-1"],
                "project_activation_revision": "pa-1"
            },
            "invalidation_tuple": {
                "framework_revision": "release-1",
                "project_activation_revision": "pa-1"
            }
        });

        let blockers = cache_contract_consistency_blockers(&payload);
        assert!(blockers
            .iter()
            .any(|row| row == "missing_cache_key_input:protocol_binding_revision"));
        assert!(blockers
            .iter()
            .any(|row| row == "missing_invalidation_tuple_key:startup_bundle_revision"));
    }

    #[test]
    fn cache_contract_consistency_blocks_mismatched_revisions() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.metadata = serde_json::json!({
            "framework_revision": "release-1",
            "project_activation_revision": "pa-1",
            "protocol_binding_revision": "pb-1",
        });
        payload.cache_delivery_contract = serde_json::json!({
            "cache_key_inputs": {
                "source_version_tuple": ["release-1"],
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-2",
                "startup_bundle_revision": "sb-1"
            },
            "invalidation_tuple": {
                "framework_revision": "release-2",
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-2",
                "startup_bundle_revision": "sb-2"
            }
        });

        let blockers = cache_contract_consistency_blockers(&payload);
        assert!(blockers
            .iter()
            .any(|row| row == "cache_key_mismatch:protocol_binding_revision"));
        assert!(blockers
            .iter()
            .any(|row| row == "invalidation_tuple_mismatch:framework_revision"));
        assert!(blockers
            .iter()
            .any(|row| row == "invalidation_tuple_mismatch:startup_bundle_revision"));
    }

    #[test]
    fn cache_contract_consistency_blocks_whitespace_padded_shared_tuple_entries() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.metadata = serde_json::json!({
            "framework_revision": " release-1 ",
            "project_activation_revision": " pa-1 ",
            "protocol_binding_revision": " pb-1 ",
            "protocol_binding_cache_token": " token-pb-1 ",
        });
        payload.cache_delivery_contract = serde_json::json!({
            "cache_key_inputs": {
                "source_version_tuple": ["release-1"],
                "project_activation_revision": " pa-1 ",
                "protocol_binding_revision": " pb-1 ",
                "protocol_binding_cache_token": " token-pb-1 ",
                "startup_bundle_revision": " sb-1 "
            },
            "invalidation_tuple": {
                "framework_revision": " release-1 ",
                "project_activation_revision": " pa-1 ",
                "protocol_binding_revision": " pb-1 ",
                "protocol_binding_cache_token": " token-pb-1 ",
                "startup_bundle_revision": " sb-1 "
            }
        });

        let blockers = cache_contract_consistency_blockers(&payload);
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_metadata_tuple_key:protocol_binding_revision"));
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_cache_key_input:protocol_binding_revision"));
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_invalidation_tuple_key:protocol_binding_revision"));
    }

    #[test]
    fn cache_contract_consistency_blocks_mixed_case_source_version_tuple_entries() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.cache_delivery_contract = serde_json::json!({
            "cache_key_inputs": {
                "source_version_tuple": ["Release-1"],
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-1",
                "protocol_binding_cache_token": "token-pb-1",
                "startup_bundle_revision": "sb-1"
            },
            "invalidation_tuple": {
                "framework_revision": "release-1",
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-1",
                "protocol_binding_cache_token": "token-pb-1",
                "startup_bundle_revision": "sb-1"
            }
        });

        let blockers = cache_contract_consistency_blockers(&payload);
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_cache_key_input:source_version_tuple"));
    }

    #[test]
    fn cache_contract_consistency_blocks_unknown_and_mixed_case_tuple_keys() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.cache_delivery_contract = serde_json::json!({
            "cache_key_inputs": {
                "source_version_tuple": ["release-1"],
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-1",
                "protocol_binding_cache_token": "token-pb-1",
                "startup_bundle_revision": "sb-1",
                "Protocol_Binding_Revision": "pb-1"
            },
            "invalidation_tuple": {
                "framework_revision": "release-1",
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-1",
                "protocol_binding_cache_token": "token-pb-1",
                "startup_bundle_revision": "sb-1",
                "Framework_Revision": "release-1"
            }
        });

        let blockers = cache_contract_consistency_blockers(&payload);
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_cache_key_inputs_keys"));
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_invalidation_tuple_keys"));
    }

    #[test]
    fn cache_contract_consistency_blocks_unknown_and_mixed_case_metadata_keys() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.metadata = serde_json::json!({
            "bundle_id": "taskflow-runtime-bundle-1",
            "bundle_schema_version": "release-1-v1",
            "framework_revision": "release-1",
            "project_activation_revision": "pa-1",
            "protocol_binding_revision": "pb-1",
            "protocol_binding_cache_token": "token-pb-1",
            "compiled_at": "2026-03-17T00:00:00Z",
            "binding_status": "blocked",
            "error": "blocked",
            "Bundle_Id": "taskflow-runtime-bundle-1"
        });
        payload.cache_delivery_contract = serde_json::json!({
            "cache_key_inputs": {
                "source_version_tuple": ["release-1"],
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-1",
                "protocol_binding_cache_token": "token-pb-1",
                "startup_bundle_revision": "sb-1"
            },
            "invalidation_tuple": {
                "framework_revision": "release-1",
                "project_activation_revision": "pa-1",
                "protocol_binding_revision": "pb-1",
                "protocol_binding_cache_token": "token-pb-1",
                "startup_bundle_revision": "sb-1"
            }
        });

        let blockers = cache_contract_consistency_blockers(&payload);
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_metadata_tuple_keys"));
    }

    #[test]
    fn cache_contract_consistency_blocks_unknown_and_mixed_case_control_core_keys() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.control_core = serde_json::json!({
            "root_artifact_id": "root-1",
            "mandatory_chain_order": ["core"],
            "source_version_tuple": ["release-1"],
            "receipt_id": "receipt-1",
            "artifact_count": 1,
            "Error": "boom"
        });

        let blockers = taskflow_consume_bundle_check(&payload).blockers;
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_control_core_keys"));
    }

    #[test]
    fn cache_contract_consistency_blocks_unknown_and_mixed_case_protocol_binding_registry_keys() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.protocol_binding_registry = serde_json::json!({
            "primary_state_authority": TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
            "receipt_id": "receipt-1",
            "binding_status": "bound",
            "compiled_payload_import_evidence": {
                "imported": true,
                "trusted": true,
                "source": "state_store",
                "source_config_path": "/tmp/project/vida.config.yaml",
                "source_config_digest": "digest-1",
                "captured_at": "2026-03-17T00:00:00Z",
                "effective_bundle_receipt_id": "receipt-1",
                "effective_bundle_root_artifact_id": "root-1",
                "effective_bundle_artifact_count": 1,
                "compiled_payload_summary": {},
                "blockers": []
            },
            "protocols": [
                {
                    "protocol_id": "instruction-contracts/bridge.instruction-activation-protocol",
                    "activation_class": "always_on",
                    "runtime_owner": "vida::taskflow::protocol_binding::activation_bridge",
                    "enforcement_type": "activation-resolution",
                    "proof_surface": "vida docflow activation-check --profile active-canon",
                    "binding_status": "fully-runtime-bound",
                    "blockers": []
                }
            ],
            "Protocols": []
        });

        let blockers = taskflow_consume_bundle_check(&payload).blockers;
        assert!(blockers
            .iter()
            .any(|row| row == "invalid_protocol_binding_registry_keys"));
    }

    #[test]
    fn protocol_binding_registry_whitespace_only_receipt_id_fails_closed() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.protocol_binding_registry = serde_json::json!({
            "primary_state_authority": TASKFLOW_PROTOCOL_BINDING_AUTHORITY,
            "receipt_id": "   ",
            "binding_status": "bound",
            "compiled_payload_import_evidence": {
                "imported": true,
                "trusted": true,
                "source": "state_store",
                "source_config_path": "/tmp/project/vida.config.yaml",
                "source_config_digest": "digest-1",
                "captured_at": "2026-03-17T00:00:00Z",
                "effective_bundle_receipt_id": "receipt-1",
                "effective_bundle_root_artifact_id": "root-1",
                "effective_bundle_artifact_count": 1,
                "compiled_payload_summary": {},
                "blockers": []
            },
            "protocols": [
                {
                    "protocol_id": "instruction-contracts/bridge.instruction-activation-protocol",
                    "activation_class": "always_on",
                    "runtime_owner": "vida::taskflow::protocol_binding::activation_bridge",
                    "enforcement_type": "activation-resolution",
                    "proof_surface": "vida docflow activation-check --profile active-canon",
                    "binding_status": "fully-runtime-bound",
                    "blockers": []
                }
            ]
        });

        let blockers = taskflow_consume_bundle_check(&payload).blockers;
        assert!(blockers
            .iter()
            .any(|row| row == "missing_protocol_binding_receipt"));
    }

    #[test]
    fn retrieval_trust_evidence_requires_all_fields() {
        let contract = serde_json::json!({
            "retrieval_trust_evidence": {
                "source": "taskflow_runtime_bundle",
                "citation": "/tmp/project/vida.config.yaml",
                "freshness": "2026-03-17T00:00:00Z"
            }
        });
        let blockers = retrieval_trust_evidence_blockers(&contract);
        assert!(blockers
            .iter()
            .any(|row| row == "missing_retrieval_trust_evidence_field:acl"));
    }

    #[test]
    fn retrieval_trust_evidence_blocks_whitespace_only_fields() {
        let contract = serde_json::json!({
            "retrieval_trust_evidence": {
                "source": " taskflow_runtime_bundle ",
                "citation": "   ",
                "freshness": "2026-03-17T00:00:00Z",
                "acl": "\t"
            }
        });
        let blockers = retrieval_trust_evidence_blockers(&contract);
        assert!(blockers
            .iter()
            .any(|row| row == "missing_retrieval_trust_evidence_field:citation"));
        assert!(blockers
            .iter()
            .any(|row| row == "missing_retrieval_trust_evidence_field:acl"));
        assert!(blockers
            .iter()
            .any(|row| row == "missing_retrieval_trust_evidence_field:source"));
    }

    #[test]
    fn activation_status_pending_compat_accepts_pending_and_pending_activation() {
        assert!(activation_status_is_pending(Some("pending")));
        assert!(activation_status_is_pending(Some("pending_activation")));
        assert!(!activation_status_is_pending(Some(
            "ready_enough_for_normal_work"
        )));
        assert!(!activation_status_is_pending(None));
    }

    #[test]
    fn bundle_check_activation_status_fails_closed_for_case_and_whitespace_drift_in_view_status() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.orchestrator_init_view = serde_json::json!({
            "status": " PENDING_ACTIVATION ",
            "activation_pending": false,
        });
        payload.agent_init_view = serde_json::json!({
            "status": "ready_enough_for_normal_work",
            "activation_pending": false,
        });

        let check = taskflow_consume_bundle_check(&payload);
        assert_eq!(check.activation_status, "pending");
    }

    #[test]
    fn bundle_check_canonicalizes_legacy_pending_activation_inputs() {
        let mut payload = minimal_payload_for_cache_checks();
        payload.orchestrator_init_view = serde_json::json!({
            "status": "pending_activation",
            "activation_pending": true,
        });
        payload.agent_init_view = serde_json::json!({
            "status": "pending_activation",
            "activation_pending": true,
        });

        let check = taskflow_consume_bundle_check(&payload);
        assert_eq!(check.activation_status, "pending");
    }

    #[test]
    fn init_view_activation_pending_is_fail_closed_for_status_and_boolean_drift() {
        let pending_by_boolean = serde_json::json!({
            "status": "ready_enough_for_normal_work",
            "activation_pending": true
        });
        assert!(init_view_activation_is_pending(&pending_by_boolean));

        let pending_by_status = serde_json::json!({
            "status": "pending_activation",
            "activation_pending": false
        });
        assert!(init_view_activation_is_pending(&pending_by_status));

        let ready_view = serde_json::json!({
            "status": "ready_enough_for_normal_work",
            "activation_pending": false
        });
        assert!(!init_view_activation_is_pending(&ready_view));
    }

    #[test]
    fn project_protocol_projection_status_normalizes_case_and_whitespace_drift() {
        assert_eq!(
            canonical_project_protocol_projection_status(Some(" COMPILED ")),
            "compiled"
        );
        assert_eq!(
            canonical_project_protocol_projection_status(Some(" validated ")),
            "validated"
        );
        assert_eq!(
            canonical_project_protocol_projection_status(Some(" present ")),
            "present"
        );
        assert_eq!(
            canonical_project_protocol_projection_status(Some(" missing ")),
            "missing"
        );
        assert_eq!(
            canonical_project_protocol_projection_status(Some("unknown")),
            "blocked"
        );
    }

    fn minimal_payload_for_cache_checks() -> TaskflowConsumeBundlePayload {
        TaskflowConsumeBundlePayload {
            artifact_name: "taskflow_runtime_bundle".to_string(),
            artifact_type: "runtime_bundle".to_string(),
            generated_at: "2026-03-17T00:00:00Z".to_string(),
            vida_root: "/tmp/project".to_string(),
            config_path: "/tmp/project/vida.config.yaml".to_string(),
            activation_source: "state_store".to_string(),
            launcher_runtime_paths: crate::DoctorLauncherSummary {
                vida: "vida".to_string(),
                project_root: "/tmp/project".to_string(),
                taskflow_surface: "vida taskflow".to_string(),
            },
            metadata: serde_json::json!({}),
            control_core: serde_json::json!({}),
            activation_bundle: serde_json::json!({}),
            protocol_binding_registry: serde_json::json!({}),
            cache_delivery_contract: serde_json::json!({}),
            orchestrator_init_view: serde_json::json!({}),
            agent_init_view: serde_json::json!({}),
            boot_compatibility: serde_json::json!({}),
            migration_preflight: serde_json::json!({}),
            task_store: serde_json::json!({}),
            run_graph: serde_json::json!({}),
        }
    }

    #[test]
    fn activation_truth_project_root_ignores_foreign_vida_root_override() {
        let payload = minimal_payload_for_cache_checks();
        let selected =
            activation_truth_project_root(&payload, Some("/tmp/foreign-runtime-root".to_string()));
        assert_eq!(selected, "/tmp/project");
    }

    #[test]
    fn activation_truth_project_root_accepts_equivalent_vida_root_override() {
        let payload = minimal_payload_for_cache_checks();
        let selected = activation_truth_project_root(&payload, Some("/tmp/project".to_string()));
        assert_eq!(selected, "/tmp/project");
    }
}
