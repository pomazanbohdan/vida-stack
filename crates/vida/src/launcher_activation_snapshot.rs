use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;

use crate::state_store::LauncherActivationSnapshot;
use crate::{
    build_compiled_agent_extension_bundle_for_root, config_file_path, config_file_path_for_root,
    load_project_overlay_yaml, load_project_overlay_yaml_for_root, split_csv_like, yaml_lookup,
    yaml_string, StateStore, StateStoreError,
};

pub(crate) fn pack_router_keywords_json(config: &serde_yaml::Value) -> serde_json::Value {
    serde_json::json!({
        "research": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "research"])).unwrap_or_default()),
        "spec": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "spec"])).unwrap_or_default()),
        "pool": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "pool"])).unwrap_or_default()),
        "pool_strong": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "pool_strong"])).unwrap_or_default()),
        "pool_dependency": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "pool_dependency"])).unwrap_or_default()),
        "dev": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "dev"])).unwrap_or_default()),
        "bug": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "bug"])).unwrap_or_default()),
        "reflect": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "reflect"])).unwrap_or_default()),
        "reflect_strong": split_csv_like(&yaml_string(yaml_lookup(config, &["pack_router_keywords", "reflect_strong"])).unwrap_or_default()),
    })
}

pub(crate) fn config_file_digest(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "Failed to read config for digest at {}: {error}",
            path.display()
        )
    })?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub(crate) fn capture_launcher_activation_snapshot() -> Result<LauncherActivationSnapshot, String> {
    let project_root = crate::resolve_runtime_project_root()?;
    capture_launcher_activation_snapshot_for_root(&project_root)
}

pub(crate) fn capture_launcher_activation_snapshot_for_root(
    project_root: &Path,
) -> Result<LauncherActivationSnapshot, String> {
    let config = load_project_overlay_yaml_for_root(project_root)?;
    let config_path = config_file_path_for_root(project_root);
    let config_digest = config_file_digest(&config_path)?;
    let compiled_bundle = build_compiled_agent_extension_bundle_for_root(&config, project_root)?;
    Ok(LauncherActivationSnapshot {
        source: "state_store".to_string(),
        source_config_path: config_path.display().to_string(),
        source_config_digest: config_digest,
        captured_at: time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("rfc3339 timestamp should render"),
        compiled_bundle,
        pack_router_keywords: pack_router_keywords_json(&config),
    })
}

pub(crate) async fn sync_launcher_activation_snapshot(
    store: &StateStore,
) -> Result<LauncherActivationSnapshot, String> {
    let project_root = launcher_activation_project_root(store)?;
    sync_launcher_activation_snapshot_for_root(store, &project_root).await
}

pub(crate) async fn sync_launcher_activation_snapshot_for_root(
    store: &StateStore,
    project_root: &Path,
) -> Result<LauncherActivationSnapshot, String> {
    let snapshot = capture_launcher_activation_snapshot_for_root(project_root)?;
    store
        .write_launcher_activation_snapshot(&snapshot)
        .await
        .map_err(|error| format!("Failed to write launcher activation snapshot: {error}"))?;
    Ok(snapshot)
}

pub(crate) async fn read_or_sync_launcher_activation_snapshot(
    store: &StateStore,
) -> Result<LauncherActivationSnapshot, String> {
    let project_root = launcher_activation_project_root(store)?;
    match store.read_launcher_activation_snapshot().await {
        Ok(snapshot) => {
            let config_path = config_file_path_for_root(&project_root);
            let current_digest = config_file_digest(&config_path)?;
            let current_config_path = config_path.display().to_string();
            if snapshot.source_config_digest == current_digest
                && snapshot.source_config_path == current_config_path
            {
                Ok(snapshot)
            } else {
                sync_launcher_activation_snapshot_for_root(store, &project_root).await
            }
        }
        Err(StateStoreError::MissingLauncherActivationSnapshot) => {
            sync_launcher_activation_snapshot_for_root(store, &project_root).await
        }
        Err(error) => Err(format!(
            "Failed to read launcher activation snapshot: {error}"
        )),
    }
}

fn launcher_activation_project_root(store: &StateStore) -> Result<PathBuf, String> {
    crate::taskflow_task_bridge::infer_project_root_from_state_root(store.root())
        .or_else(|| crate::resolve_runtime_project_root().ok())
        .ok_or_else(|| {
            format!(
                "Unable to resolve launcher activation project root from state dir {}",
                store.root().display()
            )
        })
}

fn normalize_root_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(crate) async fn ensure_launcher_bootstrap(
    store: &StateStore,
    instruction_source_root: &Path,
    framework_memory_source_root: &Path,
) -> Result<(), String> {
    store
        .seed_framework_instruction_bundle()
        .await
        .map_err(|error| format!("Failed to seed framework instruction bundle: {error}"))?;
    store
        .source_tree_summary()
        .await
        .map_err(|error| format!("Failed to read source tree metadata: {error}"))?;
    store
        .ingest_instruction_source_tree(&normalize_root_arg(instruction_source_root))
        .await
        .map_err(|error| format!("Failed to ingest instruction source tree: {error}"))?;
    let compatibility = store
        .evaluate_boot_compatibility()
        .await
        .map_err(|error| format!("Failed to evaluate boot compatibility: {error}"))?;
    if !crate::contract_profile_adapter::boot_compatibility_is_backward_compatible(
        &compatibility.classification,
    ) {
        return Err(format!(
            "Boot compatibility check failed: {}",
            compatibility.reasons.join(", ")
        ));
    }
    let migration = store
        .evaluate_migration_preflight()
        .await
        .map_err(|error| format!("Failed to evaluate migration preflight: {error}"))?;
    if !migration.blockers.is_empty() {
        return Err(format!(
            "Migration preflight failed: {}",
            migration.blockers.join(", ")
        ));
    }
    let root_artifact_id = store
        .active_instruction_root()
        .await
        .map_err(|error| format!("Failed to read active instruction root: {error}"))?;
    store
        .resolve_effective_instruction_bundle(&root_artifact_id)
        .await
        .map_err(|error| format!("Failed to resolve effective instruction bundle: {error}"))?;
    store
        .ingest_framework_memory_source_tree(&normalize_root_arg(framework_memory_source_root))
        .await
        .map_err(|error| format!("Failed to ingest framework memory source tree: {error}"))?;
    sync_launcher_activation_snapshot(store)
        .await
        .map_err(|error| format!("Failed to persist launcher activation snapshot: {error}"))?;
    crate::taskflow_protocol_binding::sync_taskflow_protocol_binding_snapshot(store).await?;
    Ok(())
}
