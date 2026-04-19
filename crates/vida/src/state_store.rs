use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "state_store_boot_summary.rs"]
mod state_store_boot_summary;
#[path = "state_store_core_utils.rs"]
mod state_store_core_utils;
#[path = "state_store_instruction_bundle.rs"]
mod state_store_instruction_bundle;
#[path = "state_store_launcher_activation.rs"]
mod state_store_launcher_activation;
#[path = "state_store_open.rs"]
mod state_store_open;
#[path = "state_store_patching.rs"]
mod state_store_patching;
#[path = "state_store_protocol_binding.rs"]
mod state_store_protocol_binding;
#[path = "state_store_run_graph_state.rs"]
mod state_store_run_graph_state;
#[path = "state_store_run_graph_summary.rs"]
mod state_store_run_graph_summary;
#[path = "state_store_source_scan.rs"]
mod state_store_source_scan;
#[path = "state_store_task_graph.rs"]
mod state_store_task_graph;
#[path = "state_store_task_models.rs"]
mod state_store_task_models;
#[path = "state_store_task_store.rs"]
mod state_store_task_store;
#[path = "state_store_taskflow_snapshot_bridge.rs"]
mod state_store_taskflow_snapshot_bridge;
#[path = "state_store_taskflow_snapshot_codec.rs"]
mod state_store_taskflow_snapshot_codec;

use crate::release1_contracts::{
    canonical_blocker_code_str, canonical_compatibility_class_str, canonical_lane_status_str,
    canonical_release1_contract_type_str, canonical_release1_schema_version_str,
    derive_lane_status, BlockerCode, CompatibilityClass, LaneStatus, Release1ContractType,
    Release1SchemaVersion,
};
#[cfg(test)]
use state_store_boot_summary::StorageMetaRow;
pub(crate) use state_store_boot_summary::{
    BootCompatibilitySummary, MigrationPreflightSummary, MigrationReceiptSummary,
    StateSpineSummary, StorageMetadataSummary,
};
use state_store_core_utils::{
    compare_task_paths, escape_surql_literal, sanitize_record_id, task_ready_sort_key,
    task_sort_key, unix_timestamp, unix_timestamp_nanos,
};
pub use state_store_core_utils::{default_state_dir, repo_root};
#[allow(unused_imports)]
pub use state_store_instruction_bundle::{
    EffectiveBundleReceiptSummary, EffectiveInstructionArtifact, EffectiveInstructionBundle,
    InstructionDiffPatchContent, InstructionIngestSummary, InstructionPatchOperation,
    InstructionProjection,
};
#[allow(unused_imports)]
pub(crate) use state_store_instruction_bundle::{
    EffectiveInstructionBundleReceiptContent, InstructionArtifactContent, InstructionArtifactRow,
    InstructionDependencyEdgeContent, InstructionDependencyEdgeRow, InstructionDiffPatchRow,
    InstructionIngestReceiptContent, InstructionProjectionReceiptContent,
    InstructionRuntimeStateRow, SourceArtifactContent, SourceArtifactRow, SourceTreeConfigRow,
};
pub use state_store_launcher_activation::LauncherActivationSnapshot;
use state_store_patching::{
    apply_patch_operation, collect_patch_ids, join_lines, split_lines, validate_patch_bindings,
    validate_patch_conflicts,
};
pub use state_store_protocol_binding::{ProtocolBindingState, ProtocolBindingSummary};
#[allow(unused_imports)]
pub(crate) use state_store_run_graph_state::{
    ExecutionPlanStateRow, GovernanceStateRow, ResumabilityCapsuleRow, RoutedRunStateRow,
    RunGraphDispatchReceiptStored, RunGraphLatestRow,
};
#[allow(unused_imports)]
pub use state_store_run_graph_state::{
    RunGraphContinuationBinding, RunGraphDispatchContext, RunGraphDispatchReceipt, RunGraphStatus,
    RunGraphSummary,
};
pub(crate) use state_store_run_graph_summary::{
    default_run_graph_lane_status, deserialize_run_graph_lane_status,
    handoff_state_links_consent_ttl, latest_run_graph_dispatch_receipt_matches_status,
    latest_run_graph_dispatch_receipt_signal_is_ambiguous,
    latest_run_graph_dispatch_receipt_summary_is_inconsistent,
    latest_run_graph_evidence_snapshot_is_consistent, normalize_run_graph_lane_status,
    requires_memory_governance_enforcement, RunGraphApprovalDelegationReceipt,
    RunGraphCheckpointSummary, RunGraphDelegationGateSummary, RunGraphDispatchReceiptSummary,
    RunGraphGateSummary, RunGraphRecoverySummary,
};
use state_store_source_scan::{
    artifact_id_from_path, collect_markdown_files, hierarchy_from_path, infer_artifact_kind,
    infer_mutability_class, infer_ownership_class, normalize_path, parse_source_metadata,
    record_id_for_slice_source,
};
pub use state_store_task_models::{
    BlockedTaskRecord, CreateTaskRequest, TaskCriticalPath, TaskCriticalPathNode,
    TaskDependencyStatus, TaskDependencyTreeChild, TaskDependencyTreeEdge, TaskDependencyTreeNode,
    TaskExecutionSemantics, TaskGraphIssue, TaskImportSummary, TaskProgressSummary, TaskRecord,
    TaskRelease1ContractStep, TaskSchedulingCandidate, TaskSchedulingProjection, TaskStoreSummary,
    UpdateTaskRequest,
};
pub(crate) use state_store_task_models::{TaskContent, TaskJsonlRecord, TaskStorageRow};
#[cfg(test)]
use state_store_taskflow_snapshot_codec::{
    canonical_issue_type_label, canonical_task_status_label, canonical_timestamp_label,
};
use state_store_taskflow_snapshot_codec::{
    task_dependency_to_canonical_edge, task_record_to_canonical_snapshot_row,
    task_records_from_canonical_snapshot, task_records_from_canonical_snapshot_for_additive_import,
};
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::types::SurrealValue;
use surrealdb::Surreal;
use taskflow_contracts::{
    DependencyEdge as CanonicalDependencyEdge, TaskRecord as CanonicalTaskRecord,
};
use taskflow_core::{
    IssueType as CanonicalIssueType, TaskId as CanonicalTaskId, TaskStatus as CanonicalTaskStatus,
    Timestamp as CanonicalTimestamp,
};
use taskflow_state::InMemoryTaskStore;
use taskflow_state_fs::{
    read_snapshot_into_memory as read_canonical_snapshot_into_memory,
    restore_in_memory_store as restore_canonical_in_memory_store,
    write_snapshot as write_canonical_snapshot, TaskSnapshot,
};
use taskflow_state_surreal::{StateSpineManifestContract, SurrealStoreTarget};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const DEFAULT_STATE_DIR: &str = ".vida/data/state";
const STATE_STORE_RECOVERY_HINT: &str = "hint: use VIDA_STATE_DIR=<temp-dir> for a fresh proof run, or reinitialize the long-lived local state root instead of deleting datastore subdirectories by hand";
pub const STATE_NAMESPACE: &str = "vida";
pub const STATE_DATABASE: &str = "primary";
pub const DEFAULT_INSTRUCTION_SOURCE_ROOT: &str =
    "vida/config/instructions/bundles/framework-source";
pub const DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT: &str =
    "vida/config/instructions/bundles/framework-memory-source";
const VIDA_PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const INSTRUCTION_STATE_SCHEMA: &str = r#"
DEFINE TABLE instruction_artifact SCHEMALESS;
DEFINE TABLE instruction_dependency_edge SCHEMALESS;
DEFINE TABLE instruction_sidecar SCHEMALESS;
DEFINE TABLE instruction_diff_patch SCHEMALESS;
DEFINE TABLE instruction_migration_receipt SCHEMALESS;
DEFINE TABLE instruction_projection_receipt SCHEMALESS;
DEFINE TABLE effective_instruction_bundle_receipt SCHEMALESS;
DEFINE TABLE instruction_runtime_state SCHEMALESS;
DEFINE TABLE instruction_source_artifact SCHEMALESS;
DEFINE TABLE instruction_ingest_receipt SCHEMALESS;
DEFINE TABLE source_tree_config SCHEMALESS;
DEFINE TABLE protocol_binding_state SCHEMALESS;
DEFINE TABLE protocol_binding_receipt SCHEMALESS;
DEFINE TABLE launcher_activation_snapshot SCHEMALESS;
DEFINE TABLE run_graph_approval_delegation_receipt SCHEMALESS;
DEFINE TABLE run_graph_continuation_binding SCHEMALESS;
DEFINE TABLE run_graph_dispatch_context SCHEMALESS;
"#;

fn state_store_recovery_hint_for_message(message: &str) -> Option<&'static str> {
    if message.contains("Failed to load manifest")
        || message.contains("authoritative state spine manifest")
        || message.contains("No such file or directory")
    {
        return Some(STATE_STORE_RECOVERY_HINT);
    }
    if message.contains("LOCK") || message.contains("lock") {
        return Some(
            "hint: another VIDA process still holds the authoritative datastore lock; wait for that lane to finish, reclaim the stuck lane through VIDA recovery flow, or retry after the holder exits instead of deleting datastore files by hand",
        );
    }
    None
}

#[path = "state_store_task_reconciliation.rs"]
mod state_store_task_reconciliation;

pub(crate) use state_store_task_reconciliation::{
    count_snapshot_bridge_rows, TaskReconciliationRollup, TaskReconciliationRollupRow,
    TaskReconciliationSummary, TaskReconciliationSummaryInput, TaskReconciliationSummaryRow,
    TaskflowSnapshotBridgeSummary,
};

#[derive(Debug)]
pub struct StateStore {
    db: Surreal<Db>,
    root: PathBuf,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
pub struct TaskDependencyRecord {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub created_at: String,
    pub created_by: String,
    pub metadata: String,
    pub thread_id: String,
}

#[derive(Debug)]
pub enum StateStoreError {
    Io(io::Error),
    Db(surrealdb::Error),
    MissingStateDir(PathBuf),
    InvalidSourcePath(PathBuf),
    MissingMetadata,
    MissingTask {
        task_id: String,
    },
    InvalidTaskJsonLine {
        line: usize,
        reason: String,
    },
    InvalidTaskRecord {
        reason: String,
    },
    MissingSourceTreeConfig,
    MissingStateSpineManifest,
    InvalidStorageMetadata {
        reason: String,
    },
    InvalidStateSpineManifest {
        reason: String,
    },
    #[allow(dead_code)]
    InvalidCanonicalTaskflowExport {
        reason: String,
    },
    MissingInstructionRuntimeState,
    InvalidInstructionRuntimeState {
        reason: String,
    },
    InvalidProtocolBinding {
        reason: String,
    },
    MissingLauncherActivationSnapshot,
    InvalidLauncherActivationSnapshot {
        reason: String,
    },
    MissingSourceRoot {
        slice: String,
        path: PathBuf,
    },
    #[allow(dead_code)]
    MissingInstructionArtifact {
        artifact_id: String,
    },
    #[allow(dead_code)]
    InvalidPatchOperation {
        reason: String,
    },
    #[allow(dead_code)]
    PatchConflict {
        reason: String,
    },
    #[allow(dead_code)]
    InstructionDependencyCycle {
        cycle_path: String,
    },
}

impl std::fmt::Display for StateStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Db(error) => {
                let message = error.to_string();
                write!(f, "{message}")?;
                if let Some(hint) = state_store_recovery_hint_for_message(&message) {
                    write!(f, "; {hint}")?;
                }
                Ok(())
            }
            Self::MissingStateDir(path) => {
                write!(
                    f,
                    "authoritative state directory is missing: {}; {}",
                    path.display(),
                    STATE_STORE_RECOVERY_HINT
                )
            }
            Self::InvalidSourcePath(path) => {
                write!(
                    f,
                    "invalid source path outside instruction root: {}",
                    path.display()
                )
            }
            Self::MissingMetadata => write!(f, "storage metadata record is missing"),
            Self::MissingTask { task_id } => write!(f, "task is missing: {task_id}"),
            Self::InvalidTaskJsonLine { line, reason } => {
                write!(f, "invalid task JSONL at line {line}: {reason}")
            }
            Self::InvalidTaskRecord { reason } => write!(f, "invalid task record: {reason}"),
            Self::MissingSourceTreeConfig => write!(f, "source tree config record is missing"),
            Self::InvalidStorageMetadata { reason } => {
                write!(f, "storage metadata record is invalid: {reason}")
            }
            Self::MissingStateSpineManifest => {
                write!(
                    f,
                    "authoritative state spine manifest is missing; {}",
                    STATE_STORE_RECOVERY_HINT
                )
            }
            Self::InvalidStateSpineManifest { reason } => {
                write!(
                    f,
                    "authoritative state spine manifest is invalid: {reason}; {}",
                    STATE_STORE_RECOVERY_HINT
                )
            }
            Self::InvalidCanonicalTaskflowExport { reason } => {
                write!(f, "canonical taskflow export is invalid: {reason}")
            }
            Self::MissingInstructionRuntimeState => {
                write!(f, "instruction runtime state record is missing")
            }
            Self::InvalidInstructionRuntimeState { reason } => {
                write!(f, "instruction runtime state is invalid: {reason}")
            }
            Self::InvalidProtocolBinding { reason } => {
                write!(f, "protocol binding state is invalid: {reason}")
            }
            Self::MissingLauncherActivationSnapshot => {
                write!(f, "launcher activation snapshot is missing")
            }
            Self::InvalidLauncherActivationSnapshot { reason } => {
                write!(f, "launcher activation snapshot is invalid: {reason}")
            }
            Self::MissingSourceRoot { slice, path } => {
                write!(f, "source root for {slice} is missing: {}", path.display())
            }
            Self::MissingInstructionArtifact { artifact_id } => {
                write!(f, "instruction artifact is missing: {artifact_id}")
            }
            Self::InvalidPatchOperation { reason } => {
                write!(f, "invalid patch operation: {reason}")
            }
            Self::PatchConflict { reason } => write!(f, "patch conflict: {reason}"),
            Self::InstructionDependencyCycle { cycle_path } => {
                write!(f, "instruction dependency cycle detected: {cycle_path}")
            }
        }
    }
}

impl std::error::Error for StateStoreError {}

impl From<io::Error> for StateStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<surrealdb::Error> for StateStoreError {
    fn from(error: surrealdb::Error) -> Self {
        Self::Db(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::state_store_boot_summary::StateSpineManifestContent;
    fn sample_tasks_jsonl() -> String {
        [
            r#"{"id":"vida-root","title":"Root epic","description":"epic","status":"open","priority":1,"issue_type":"epic","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["wave"],"dependencies":[]}"#,
            r#"{"id":"vida-a","title":"Task A","description":"first","status":"open","priority":2,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[{"issue_id":"vida-a","depends_on_id":"vida-root","type":"parent-child","created_at":"2026-03-08T00:00:00Z","created_by":"tester","metadata":"{}","thread_id":""}]}"#,
            r#"{"id":"vida-b","title":"Task B","description":"second","status":"open","priority":3,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[{"issue_id":"vida-b","depends_on_id":"vida-a","type":"blocks","created_at":"2026-03-08T00:00:00Z","created_by":"tester","metadata":"{}","thread_id":""}]}"#,
            r#"{"id":"vida-c","title":"Task C","description":"active","status":"in_progress","priority":4,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[]}"#,
            r#"{"id":"vida-d","title":"Task D","description":"done","status":"closed","priority":5,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","closed_at":"2026-03-08T00:10:00Z","close_reason":"done","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[]}"#,
        ]
        .join("\n")
    }

    #[test]
    fn parse_source_metadata_extracts_extended_fields() {
        let body = r#"
artifact_id: sample-artifact
artifact_kind: instruction_contract
version: 7
ownership_class: framework
mutability_class: immutable
activation_class: always_on
required_follow_on: next-one,next-two
hierarchy: framework,contracts
"#;

        let metadata = parse_source_metadata(body);
        assert_eq!(metadata.artifact_id.as_deref(), Some("sample-artifact"));
        assert_eq!(
            metadata.artifact_kind.as_deref(),
            Some("instruction_contract")
        );
        assert_eq!(metadata.version, Some(7));
        assert_eq!(metadata.activation_class.as_deref(), Some("always_on"));
        assert_eq!(metadata.required_follow_on, vec!["next-one", "next-two"]);
        assert_eq!(metadata.hierarchy, vec!["framework", "contracts"]);
    }

    #[test]
    fn runtime_state_schema_contains_surreal_adapter_bootstrap_document() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");
        let bootstrap_document = target.bootstrap_schema_document();

        assert!(state_store_open::state_schema_document().contains(&bootstrap_document));
    }

    #[tokio::test]
    async fn launcher_activation_snapshot_write_accepts_empty_source_config_path_as_provenance_only(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-launcher-activation-provenance-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let snapshot = LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: String::new(),
            source_config_digest: "digest-123".to_string(),
            captured_at: "2026-03-08T00:00:00Z".to_string(),
            compiled_bundle: serde_json::json!({
                "role_selection": {
                    "fallback_role": "worker",
                    "mode": "native"
                },
                "agent_system": {}
            }),
            pack_router_keywords: serde_json::json!({}),
        };

        store
            .write_launcher_activation_snapshot(&snapshot)
            .await
            .expect("write launcher activation snapshot");

        let read_back = store
            .read_launcher_activation_snapshot()
            .await
            .expect("read launcher activation snapshot");
        assert_eq!(read_back.source, "state_store");
        assert_eq!(read_back.source_config_path, "");
        assert_eq!(read_back.source_config_digest, "digest-123");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_continuation_binding_and_dispatch_context_round_trip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-binding-roundtrip-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let binding = RunGraphContinuationBinding {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "run_graph_task",
                "task_id": "task-1",
                "run_id": "run-1",
                "active_node": "pm",
            }),
            binding_source: "test".to_string(),
            why_this_unit: "because".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only".to_string(),
            request_text: Some("req".to_string()),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };
        let context = RunGraphDispatchContext {
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            request_text: "req".to_string(),
            role_selection: serde_json::json!({
                "ok": true,
                "activation_source": "test",
                "selection_mode": "fixed",
                "fallback_role": "worker",
                "request": "req",
                "selected_role": "worker",
                "conversational_mode": null,
                "single_task_only": false,
                "tracked_flow_entry": null,
                "allow_freeform_chat": false,
                "confidence": "high",
                "matched_terms": [],
                "compiled_bundle": null,
                "execution_plan": {},
                "reason": "test"
            }),
            recorded_at: "2026-04-10T10:00:00Z".to_string(),
        };

        store
            .record_run_graph_continuation_binding(&binding)
            .await
            .expect("record binding");
        store
            .record_run_graph_dispatch_context(&context)
            .await
            .expect("record context");

        let stored_binding = store
            .run_graph_continuation_binding("run-1")
            .await
            .expect("read binding")
            .expect("binding present");
        let stored_context = store
            .run_graph_dispatch_context("run-1")
            .await
            .expect("read context")
            .expect("context present");

        assert_eq!(stored_binding.binding_source, "test");
        assert_eq!(stored_binding.active_bounded_unit["active_node"], "pm");
        assert_eq!(stored_context.request_text, "req");
        assert_eq!(
            stored_context
                .role_selection()
                .expect("role selection should decode")
                .selected_role,
            "worker"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn runtime_state_spine_manifest_defaults_match_surreal_adapter_contract() {
        let contract = SurrealStoreTarget::new("/tmp/vida-state").state_spine_manifest_contract();
        let content =
            StateSpineManifestContent::from_contract(contract.clone(), "123456789".to_string());

        assert_eq!(content.manifest_id, contract.manifest_id);
        assert_eq!(content.state_schema_version, contract.state_schema_version);
        assert_eq!(
            content.authoritative_mutation_root,
            contract.authoritative_mutation_root
        );
        assert_eq!(content.entity_surfaces, contract.entity_surfaces);
        assert_eq!(content.initialized_at, "123456789");
    }

    #[tokio::test]
    async fn task_import_and_ready_surface_work_from_jsonl() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("vida-task-import-{}-{}", std::process::id(), nanos));
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&source, sample_tasks_jsonl()).expect("write sample jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        let summary = store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks");
        assert_eq!(summary.imported_count, 5);
        assert_eq!(summary.updated_count, 0);

        let listed = store.list_tasks(None, false).await.expect("list tasks");
        assert_eq!(listed.len(), 4);
        assert_eq!(
            listed.first().map(|task| task.id.as_str()),
            Some("vida-root")
        );

        let shown = store.show_task("vida-b").await.expect("show task");
        assert_eq!(shown.dependencies.len(), 1);
        assert_eq!(shown.dependencies[0].depends_on_id, "vida-a");

        let ready = store.ready_tasks().await.expect("ready tasks");
        let ready_ids = ready.into_iter().map(|task| task.id).collect::<Vec<_>>();
        assert_eq!(ready_ids, vec!["vida-c", "vida-a"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn task_dependency_tree_surfaces_recursive_parent_child_and_blocking_edges() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-dependency-tree-{}-{}",
            std::process::id(),
            nanos
        ));
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&source, sample_tasks_jsonl()).expect("write sample jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks");

        let tree = store
            .task_dependency_tree("vida-b")
            .await
            .expect("dependency tree");
        assert_eq!(tree.task.id, "vida-b");
        assert_eq!(tree.dependencies.len(), 1);
        assert!(tree.children.is_empty());
        let blocks = &tree.dependencies[0];
        assert_eq!(blocks.edge_type, "blocks");
        assert_eq!(blocks.depends_on_id, "vida-a");
        let a_node = blocks.node.as_ref().expect("task a node");
        assert_eq!(a_node.task.id, "vida-a");
        assert_eq!(a_node.dependencies.len(), 1);
        assert!(a_node.children.is_empty());
        let parent = &a_node.dependencies[0];
        assert_eq!(parent.edge_type, "parent-child");
        assert_eq!(parent.depends_on_id, "vida-root");
        assert!(parent.node.is_some());
        assert_eq!(parent.node.as_ref().unwrap().task.id, "vida-root");

        let root_tree = store
            .task_dependency_tree("vida-root")
            .await
            .expect("root dependency tree");
        assert!(root_tree.dependencies.is_empty());
        assert_eq!(root_tree.children.len(), 1);
        let child_ids = root_tree
            .children
            .iter()
            .map(|child| child.child_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(child_ids, vec!["vida-a"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn task_progress_summary_reports_descendant_status_totals() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-progress-summary-{}-{}",
            std::process::id(),
            nanos
        ));
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&source, sample_tasks_jsonl()).expect("write sample jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks");

        let summary = store
            .task_progress_summary("vida-root")
            .await
            .expect("progress summary");
        assert_eq!(summary.root_task.id, "vida-root");
        assert_eq!(summary.progress_basis, "descendants_excluding_root");
        assert_eq!(summary.direct_child_count, 1);
        assert_eq!(summary.descendant_count, 1);
        assert_eq!(summary.open_count, 1);
        assert_eq!(summary.in_progress_count, 0);
        assert_eq!(summary.closed_count, 0);
        assert_eq!(summary.epic_count, 0);
        assert_eq!(summary.status_counts.get("open"), Some(&1));
        assert_eq!(summary.percent_closed, 0.0);

        let b_summary = store
            .task_progress_summary("vida-b")
            .await
            .expect("task summary without descendants");
        assert_eq!(b_summary.direct_child_count, 0);
        assert_eq!(b_summary.descendant_count, 0);
        assert!(b_summary.status_counts.is_empty());
        assert_eq!(b_summary.percent_closed, 0.0);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn critical_path_includes_release1_contract_steps_surface() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-critical-path-{}-{}",
            std::process::id(),
            nanos
        ));
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        fs::write(&source, sample_tasks_jsonl()).expect("write sample jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks");

        let path = store.critical_path().await.expect("critical path");
        assert_eq!(path.length, 2);
        assert_eq!(path.root_task_id.as_deref(), Some("vida-a"));
        assert_eq!(path.terminal_task_id.as_deref(), Some("vida-b"));

        assert_eq!(path.release_1_contract_steps.len(), 1);
        let step = &path.release_1_contract_steps[0];
        assert_eq!(step.id, "doctor_run_graph_negative_control");
        assert_eq!(step.mode, "fail_closed");
        assert_eq!(
            step.blocker_code,
            "missing_run_graph_dispatch_receipt_operator_evidence"
        );
        assert_eq!(
            step.next_action,
            "Run `vida taskflow consume continue --json` to materialize or refresh run-graph dispatch receipt evidence before operator handoff."
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn close_task_fails_closed_when_open_child_tasks_exist() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-close-task-open-child-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels: Vec<String> = Vec::new();

        store
            .create_task(CreateTaskRequest {
                task_id: "vida-root",
                title: "Root",
                display_id: None,
                description: "root",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create root task");
        store
            .create_task(CreateTaskRequest {
                task_id: "vida-child",
                title: "Child",
                display_id: None,
                description: "child",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: Some("vida-root"),
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create child task");

        let error = store
            .close_task("vida-root", "done")
            .await
            .expect_err("closing parent with open child should fail");
        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("cannot close task `vida-root`"));
                assert!(reason.contains("vida-child"));
            }
            other => panic!("expected InvalidTaskRecord, got {other}"),
        }

        store
            .close_task("vida-child", "done")
            .await
            .expect("child close should succeed");
        let closed_parent = store
            .close_task("vida-root", "done")
            .await
            .expect("parent close should succeed after child closure");
        assert_eq!(closed_parent.status, "closed");

        drop(store);
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_applies_set_and_delta_labels() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root =
            std::env::temp_dir().join(format!("vida-update-task-{}-{}", std::process::id(), nanos));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = vec!["framework".to_string()];

        store
            .create_task(CreateTaskRequest {
                task_id: "vida-root",
                title: "Root",
                display_id: None,
                description: "root",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create root task");

        let set_labels_vec = vec!["alpha".to_string(), "beta ".to_string()];
        let updated = store
            .update_task(UpdateTaskRequest {
                task_id: "vida-root",
                status: Some("in_progress"),
                notes: Some("steady"),
                description: Some("adjusted"),
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: Some(&set_labels_vec),
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
            })
            .await
            .expect("apply set labels");

        assert_eq!(updated.status, "in_progress");
        assert_eq!(updated.notes.as_deref(), Some("steady"));
        assert_eq!(updated.description, "adjusted");
        assert_eq!(
            updated.labels,
            vec!["alpha".to_string(), "beta".to_string()]
        );

        let add_labels = vec!["gamma".to_string(), "alpha".to_string()];
        let remove_labels = vec!["beta".to_string()];
        let updated_again = store
            .update_task(UpdateTaskRequest {
                task_id: "vida-root",
                status: Some("open"),
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &add_labels,
                remove_labels: &remove_labels,
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
            })
            .await
            .expect("apply delta labels");

        assert_eq!(updated_again.status, "open");
        assert_eq!(
            updated_again.labels,
            vec!["alpha".to_string(), "gamma".to_string()]
        );
        assert!(updated_again.closed_at.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_reparents_without_losing_non_parent_dependencies() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-update-task-reparent-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            ("root-a", "Root A", "epic", None),
            ("root-b", "Root B", "epic", None),
            ("dep-task", "Dependency", "task", None),
            ("child-task", "Child", "task", Some("root-a")),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status: "open",
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    created_by: "tester",
                    source_repo: ".",
                })
                .await
                .expect("create task");
        }

        store
            .add_task_dependency("child-task", "dep-task", "blocks", "tester")
            .await
            .expect("add non-parent dependency");

        let reparented = store
            .update_task(UpdateTaskRequest {
                task_id: "child-task",
                status: None,
                notes: None,
                description: None,
                parent_id: Some(Some("root-b")),
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
            })
            .await
            .expect("reparent child task");

        let parent_edges = reparented
            .dependencies
            .iter()
            .filter(|dependency| dependency.edge_type == "parent-child")
            .collect::<Vec<_>>();
        assert_eq!(parent_edges.len(), 1);
        assert_eq!(parent_edges[0].depends_on_id, "root-b");
        assert!(reparented
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "blocks"
                && dependency.depends_on_id == "dep-task"));

        let detached = store
            .update_task(UpdateTaskRequest {
                task_id: "child-task",
                status: None,
                notes: None,
                description: None,
                parent_id: Some(None),
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
            })
            .await
            .expect("clear parent");

        assert!(detached
            .dependencies
            .iter()
            .all(|dependency| dependency.edge_type != "parent-child"));
        assert!(detached
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "blocks"
                && dependency.depends_on_id == "dep-task"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn ingest_is_idempotent_within_same_store() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-state-store-idempotent-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed bundle");
        let first = store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("first ingest");
        assert_eq!(first.imported_count, 3);

        let mut count_query = store
            .db
            .query("SELECT count() AS count FROM instruction_source_artifact GROUP ALL;")
            .await
            .expect("count source artifacts");
        #[derive(Debug, serde::Deserialize, SurrealValue)]
        struct CountRow {
            count: i64,
        }
        let count_rows: Vec<CountRow> = count_query.take(0).expect("take count rows");
        assert_eq!(count_rows.first().map(|row| row.count), Some(3));

        let one: Option<SourceArtifactRow> = store
            .db
            .select((
                "instruction_source_artifact",
                "instruction_memory-framework-agent-definition-source",
            ))
            .await
            .expect("select one source artifact");
        assert!(one.is_some());

        let second = store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("second ingest");
        assert_eq!(second.imported_count, 0);
        assert_eq!(second.unchanged_count, 3);
        assert_eq!(second.updated_count, 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn state_spine_manifest_is_idempotent_across_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-state-spine-idempotent-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let first: Option<StateSpineManifestContent> = store
            .db
            .select(("state_spine_manifest", "primary"))
            .await
            .expect("select first manifest");
        let first = first.expect("first manifest should exist");

        store
            .ensure_minimal_authoritative_state_spine()
            .await
            .expect("repeat ensure should succeed");
        let second: Option<StateSpineManifestContent> = store
            .db
            .select(("state_spine_manifest", "primary"))
            .await
            .expect("select second manifest");
        let second = second.expect("second manifest should exist");

        assert_eq!(first.initialized_at, second.initialized_at);

        drop(store);

        let mut existing = None;
        for _ in 0..10 {
            match StateStore::open_existing(root.clone()).await {
                Ok(store) => {
                    existing = Some(store);
                    break;
                }
                Err(StateStoreError::Db(_)) => {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(other) => panic!("open existing store: {other}"),
            }
        }
        let existing = existing.expect("open existing store");
        let summary = existing
            .state_spine_summary()
            .await
            .expect("state spine summary should load from existing store");
        assert_eq!(summary.entity_surface_count, 8);
        assert_eq!(summary.authoritative_mutation_root, "vida task");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn state_spine_summary_fails_closed_on_contract_drift() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-state-spine-drift-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let _: Option<StateSpineManifestContent> = store
            .db
            .upsert(("state_spine_manifest", "primary"))
            .content(StateSpineManifestContent {
                manifest_id: "primary".to_string(),
                state_schema_version: 1,
                authoritative_mutation_root: "legacy task".to_string(),
                entity_surfaces: vec![
                    "task".to_string(),
                    "task_dependency".to_string(),
                    "task_blocker".to_string(),
                ],
                initialized_at: "123".to_string(),
            })
            .await
            .expect("update state spine manifest");

        let error = store
            .state_spine_summary()
            .await
            .expect_err("state spine contract drift should fail");
        match error {
            StateStoreError::InvalidStateSpineManifest { reason } => {
                assert!(reason.contains("expected manifest_id=primary"));
                assert!(reason.contains("authoritative_mutation_root=vida task"));
                assert!(reason.contains("got manifest_id=primary"));
                assert!(reason.contains("authoritative_mutation_root=legacy task"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn backend_summary_fails_closed_on_storage_metadata_drift() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-storage-meta-drift-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let _: Option<StorageMetaRow> = store
            .db
            .upsert(("storage_meta", "primary"))
            .content(StorageMetaRow {
                engine: "surrealdb".to_string(),
                backend: "sqlite".to_string(),
                namespace: "vida".to_string(),
                database: "primary".to_string(),
                state_schema_version: 1,
                instruction_schema_version: 1,
            })
            .await
            .expect("update storage metadata");

        let error = store
            .backend_summary()
            .await
            .expect_err("storage metadata drift should fail");
        match error {
            StateStoreError::InvalidStorageMetadata { reason } => {
                assert!(reason.contains("expected engine=surrealdb backend=kv-surrealkv"));
                assert!(reason.contains("namespace=vida"));
                assert!(reason.contains("database=primary"));
                assert!(reason.contains("backend=sqlite"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn backend_summary_fails_closed_on_storage_metadata_namespace_drift() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-storage-meta-namespace-drift-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let _: Option<StorageMetaRow> = store
            .db
            .upsert(("storage_meta", "primary"))
            .content(StorageMetaRow {
                engine: "surrealdb".to_string(),
                backend: "kv-surrealkv".to_string(),
                namespace: "other".to_string(),
                database: "secondary".to_string(),
                state_schema_version: 1,
                instruction_schema_version: 1,
            })
            .await
            .expect("update storage metadata");

        let error = store
            .backend_summary()
            .await
            .expect_err("storage metadata namespace drift should fail");
        match error {
            StateStoreError::InvalidStorageMetadata { reason } => {
                assert!(reason.contains("expected engine=surrealdb backend=kv-surrealkv"));
                assert!(reason.contains("namespace=vida"));
                assert!(reason.contains("database=primary"));
                assert!(reason.contains("engine=surrealdb"));
                assert!(reason.contains("namespace=other"));
                assert!(reason.contains("database=secondary"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn storage_metadata_summary_matches_canonical_surreal_contract() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-storage-meta-summary-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let summary = store
            .storage_metadata_summary()
            .await
            .expect("storage metadata summary should load");

        assert_eq!(summary.engine, "surrealdb");
        assert_eq!(summary.backend, "kv-surrealkv");
        assert_eq!(summary.namespace, "vida");
        assert_eq!(summary.database, "primary");
        assert_eq!(summary.state_schema_version, 1);
        assert_eq!(summary.instruction_schema_version, 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn state_spine_summary_fails_closed_on_missing_manifest() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-state-spine-missing-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let _: Option<StateSpineManifestContent> = store
            .db
            .delete(("state_spine_manifest", "primary"))
            .await
            .expect("delete manifest");

        let error = store
            .state_spine_summary()
            .await
            .expect_err("missing manifest should fail");
        assert!(matches!(error, StateStoreError::MissingStateSpineManifest));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_state_dir_error_includes_recovery_hint() {
        let error = StateStoreError::MissingStateDir(PathBuf::from("/tmp/vida-missing"));
        let rendered = error.to_string();
        assert!(rendered.contains("authoritative state directory is missing"));
        assert!(rendered.contains("VIDA_STATE_DIR=<temp-dir>"));
        assert!(rendered.contains("reinitialize the long-lived local state root"));
    }

    #[test]
    fn missing_state_spine_manifest_error_includes_recovery_hint() {
        let rendered = StateStoreError::MissingStateSpineManifest.to_string();
        assert!(rendered.contains("authoritative state spine manifest is missing"));
        assert!(rendered.contains("VIDA_STATE_DIR=<temp-dir>"));
    }

    #[tokio::test]
    async fn active_instruction_root_loads_from_runtime_state() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-instruction-runtime-state-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");

        let active_root = store
            .active_instruction_root()
            .await
            .expect("active root should load");
        assert_eq!(active_root, "framework-agent-definition");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn boot_compatibility_is_incompatible_when_runtime_state_is_missing() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-boot-compatibility-missing-runtime-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");
        let _: Option<InstructionRuntimeStateRow> = store
            .db
            .delete(("instruction_runtime_state", "primary"))
            .await
            .expect("delete runtime state");

        let compatibility = store
            .evaluate_boot_compatibility()
            .await
            .expect("compatibility evaluation should succeed");
        assert_eq!(
            compatibility.classification,
            CompatibilityClass::ReaderUpgradeRequired.as_str()
        );
        assert!(compatibility
            .reasons
            .iter()
            .any(|reason| reason.contains("instruction runtime state missing")));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn boot_compatibility_reports_storage_metadata_drift_reason() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-boot-compatibility-storage-drift-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");
        let _: Option<StorageMetaRow> = store
            .db
            .upsert(("storage_meta", "primary"))
            .content(StorageMetaRow {
                engine: "surrealdb".to_string(),
                backend: "sqlite".to_string(),
                namespace: "vida".to_string(),
                database: "primary".to_string(),
                state_schema_version: 1,
                instruction_schema_version: 1,
            })
            .await
            .expect("update storage metadata");

        let compatibility = store
            .evaluate_boot_compatibility()
            .await
            .expect("compatibility evaluation should succeed");
        assert_eq!(
            compatibility.classification,
            CompatibilityClass::ReaderUpgradeRequired.as_str()
        );
        assert!(compatibility
            .reasons
            .iter()
            .any(|reason| reason.contains("storage metadata record is invalid")));
        assert!(compatibility
            .reasons
            .iter()
            .any(|reason| reason.contains("backend=sqlite")));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn boot_compatibility_summary_persists_across_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-boot-compatibility-reopen-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");

        let compatibility = store
            .evaluate_boot_compatibility()
            .await
            .expect("compatibility evaluation should succeed");
        assert_eq!(
            compatibility.classification,
            CompatibilityClass::BackwardCompatible.as_str()
        );
        assert!(compatibility.reasons.is_empty());
        assert_eq!(compatibility.next_step, "normal_boot_allowed");

        drop(store);

        let mut reopened = None;
        for _ in 0..10 {
            match StateStore::open_existing(root.clone()).await {
                Ok(store) => {
                    reopened = Some(store);
                    break;
                }
                Err(StateStoreError::Db(_)) => {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(other) => panic!("open existing store: {other}"),
            }
        }
        let reopened = reopened.expect("open existing store");

        let persisted = reopened
            .latest_boot_compatibility_summary()
            .await
            .expect("latest boot compatibility should load")
            .expect("persisted boot compatibility should exist");
        assert_eq!(
            persisted.classification,
            CompatibilityClass::BackwardCompatible.as_str()
        );
        assert!(persisted.reasons.is_empty());
        assert_eq!(persisted.next_step, "normal_boot_allowed");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_round_trips_and_persists_across_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-reopen-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let status = RunGraphStatus {
            run_id: "run-vida-a".to_string(),
            task_id: "vida-a".to_string(),
            task_class: "writer".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "analysis".to_string(),
            selected_backend: "codex".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "active".to_string(),
            policy_gate: "policy_gate_required".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            recovery_ready: true,
        };

        store
            .record_run_graph_status(&status)
            .await
            .expect("run graph status should record");
        let loaded = store
            .run_graph_status("run-vida-a")
            .await
            .expect("run graph status should load");
        assert_eq!(loaded.run_id, "run-vida-a");
        assert_eq!(loaded.task_id, "vida-a");
        assert_eq!(loaded.active_node, "writer");
        assert_eq!(loaded.next_node.as_deref(), Some("coach"));
        assert_eq!(loaded.route_task_class, "analysis");
        assert_eq!(loaded.selected_backend, "codex");
        assert_eq!(loaded.policy_gate, "policy_gate_required");
        assert_eq!(loaded.handoff_state, "awaiting_coach");
        assert_eq!(loaded.context_state, "sealed");
        assert_eq!(loaded.checkpoint_kind, "execution_cursor");
        assert_eq!(loaded.resume_target, "dispatch.writer_lane");
        assert!(loaded.recovery_ready);
        let recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("recovery summary should load")
            .expect("recovery summary should exist");
        assert_eq!(recovery.run_id, "run-vida-a");
        assert_eq!(recovery.task_id, "vida-a");
        assert_eq!(recovery.resume_node.as_deref(), Some("coach"));
        assert_eq!(recovery.resume_status, "ready");
        assert_eq!(recovery.checkpoint_kind, "execution_cursor");
        assert_eq!(recovery.resume_target, "dispatch.writer_lane");
        assert_eq!(recovery.policy_gate, "policy_gate_required");
        assert_eq!(recovery.handoff_state, "awaiting_coach");
        assert!(recovery.recovery_ready);
        assert!(recovery.delegation_gate.delegated_cycle_open);
        assert_eq!(
            recovery.delegation_gate.delegated_cycle_state,
            "handoff_pending"
        );
        assert_eq!(
            recovery.delegation_gate.local_exception_takeover_gate,
            "blocked_open_delegated_cycle"
        );
        let direct_recovery = store
            .run_graph_recovery_summary("run-vida-a")
            .await
            .expect("direct recovery summary should load");
        assert_eq!(direct_recovery, recovery);
        let checkpoint = store
            .latest_run_graph_checkpoint_summary()
            .await
            .expect("checkpoint summary should load")
            .expect("checkpoint summary should exist");
        assert_eq!(checkpoint.run_id, "run-vida-a");
        assert_eq!(checkpoint.task_id, "vida-a");
        assert_eq!(checkpoint.checkpoint_kind, "execution_cursor");
        assert_eq!(checkpoint.resume_target, "dispatch.writer_lane");
        assert!(checkpoint.recovery_ready);
        let direct_checkpoint = store
            .run_graph_checkpoint_summary("run-vida-a")
            .await
            .expect("direct checkpoint summary should load");
        assert_eq!(direct_checkpoint, checkpoint);
        let gate = store
            .latest_run_graph_gate_summary()
            .await
            .expect("gate summary should load")
            .expect("gate summary should exist");
        assert_eq!(gate.run_id, "run-vida-a");
        assert_eq!(gate.task_id, "vida-a");
        assert_eq!(gate.policy_gate, "policy_gate_required");
        assert_eq!(gate.handoff_state, "awaiting_coach");
        assert_eq!(gate.context_state, "sealed");
        assert!(gate.delegation_gate.delegated_cycle_open);
        assert_eq!(
            gate.delegation_gate.local_exception_takeover_gate,
            "blocked_open_delegated_cycle"
        );
        let direct_gate = store
            .run_graph_gate_summary("run-vida-a")
            .await
            .expect("direct gate summary should load");
        assert_eq!(direct_gate, gate);

        let summary = store
            .run_graph_summary()
            .await
            .expect("run graph summary should load");
        assert_eq!(summary.execution_plan_count, 1);
        assert_eq!(summary.routed_run_count, 1);
        assert_eq!(summary.governance_count, 1);
        assert_eq!(summary.resumability_count, 1);

        drop(store);

        let mut reopened = None;
        for _ in 0..10 {
            match StateStore::open_existing(root.clone()).await {
                Ok(store) => {
                    reopened = Some(store);
                    break;
                }
                Err(StateStoreError::Db(_)) => {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(other) => panic!("open existing store: {other}"),
            }
        }
        let reopened = reopened.expect("open existing store");
        let loaded = reopened
            .run_graph_status("run-vida-a")
            .await
            .expect("reopened run graph status should load");
        assert_eq!(loaded.task_id, "vida-a");
        assert_eq!(loaded.active_node, "writer");
        assert_eq!(loaded.next_node.as_deref(), Some("coach"));
        assert_eq!(loaded.lifecycle_stage, "active");
        assert_eq!(loaded.handoff_state, "awaiting_coach");
        assert_eq!(loaded.resume_target, "dispatch.writer_lane");
        assert!(loaded.recovery_ready);
        let recovery = reopened
            .latest_run_graph_recovery_summary()
            .await
            .expect("reopened recovery summary should load")
            .expect("reopened recovery summary should exist");
        assert_eq!(recovery.resume_node.as_deref(), Some("coach"));
        assert_eq!(recovery.resume_status, "ready");
        assert_eq!(recovery.policy_gate, "policy_gate_required");
        assert_eq!(recovery.handoff_state, "awaiting_coach");
        assert!(recovery.recovery_ready);
        assert!(recovery.delegation_gate.delegated_cycle_open);
        let direct_recovery = reopened
            .run_graph_recovery_summary("run-vida-a")
            .await
            .expect("reopened direct recovery summary should load");
        assert_eq!(direct_recovery, recovery);
        let checkpoint = reopened
            .latest_run_graph_checkpoint_summary()
            .await
            .expect("reopened checkpoint summary should load")
            .expect("reopened checkpoint summary should exist");
        assert_eq!(checkpoint.checkpoint_kind, "execution_cursor");
        assert_eq!(checkpoint.resume_target, "dispatch.writer_lane");
        assert!(checkpoint.recovery_ready);
        let direct_checkpoint = reopened
            .run_graph_checkpoint_summary("run-vida-a")
            .await
            .expect("reopened direct checkpoint summary should load");
        assert_eq!(direct_checkpoint, checkpoint);
        let gate = reopened
            .latest_run_graph_gate_summary()
            .await
            .expect("reopened gate summary should load")
            .expect("reopened gate summary should exist");
        assert_eq!(gate.policy_gate, "policy_gate_required");
        assert_eq!(gate.handoff_state, "awaiting_coach");
        assert_eq!(gate.context_state, "sealed");
        assert!(gate.delegation_gate.delegated_cycle_open);
        let direct_gate = reopened
            .run_graph_gate_summary("run-vida-a")
            .await
            .expect("reopened direct gate summary should load");
        assert_eq!(direct_gate, gate);

        let summary = reopened
            .run_graph_summary()
            .await
            .expect("reopened run graph summary should load");
        assert_eq!(summary.execution_plan_count, 1);
        assert_eq!(summary.routed_run_count, 1);
        assert_eq!(summary.governance_count, 1);
        assert_eq!(summary.resumability_count, 1);

        let _ = fs::remove_dir_all(&root);
    }

    fn sample_run_graph_status() -> RunGraphStatus {
        RunGraphStatus {
            run_id: "run-vida-a".to_string(),
            task_id: "vida-a".to_string(),
            task_class: "writer".to_string(),
            active_node: "writer".to_string(),
            next_node: Some("coach".to_string()),
            status: "ready".to_string(),
            route_task_class: "analysis".to_string(),
            selected_backend: "codex".to_string(),
            lane_id: "writer_lane".to_string(),
            lifecycle_stage: "active".to_string(),
            policy_gate: "policy_gate_required".to_string(),
            handoff_state: "awaiting_coach".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "execution_cursor".to_string(),
            resume_target: "dispatch.writer_lane".to_string(),
            recovery_ready: true,
        }
    }

    #[tokio::test]
    async fn record_run_graph_status_persists_route_bound_approval_delegation_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-approval-delegation-receipt-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut awaiting_approval = sample_run_graph_status();
        awaiting_approval.task_class = "implementation".to_string();
        awaiting_approval.route_task_class = "implementation".to_string();
        awaiting_approval.active_node = "verification".to_string();
        awaiting_approval.next_node = Some("approval".to_string());
        awaiting_approval.status = "awaiting_approval".to_string();
        awaiting_approval.lifecycle_stage = "approval_wait".to_string();
        awaiting_approval.policy_gate = crate::release1_contracts::ApprovalStatus::ApprovalRequired
            .as_str()
            .to_string();
        awaiting_approval.handoff_state = "awaiting_approval".to_string();
        awaiting_approval.resume_target = "dispatch.approval".to_string();

        store
            .record_run_graph_status(&awaiting_approval)
            .await
            .expect("persist approval wait run graph status");

        let receipt = store
            .run_graph_approval_delegation_receipt("run-vida-a")
            .await
            .expect("load approval wait receipt")
            .expect("approval wait receipt should exist");
        assert_eq!(receipt.transition_kind, "approval_wait");
        assert_eq!(receipt.status, "awaiting_approval");
        assert_eq!(receipt.lifecycle_stage, "approval_wait");
        assert_eq!(receipt.policy_gate, "approval_required");
        assert_eq!(receipt.handoff_state, "awaiting_approval");
        assert_eq!(receipt.resume_target, "dispatch.approval");
        assert_eq!(receipt.next_node.as_deref(), Some("approval"));

        let mut completed = awaiting_approval;
        completed.status = "completed".to_string();
        completed.next_node = None;
        completed.lifecycle_stage = "implementation_complete".to_string();
        completed.policy_gate = "not_required".to_string();
        completed.handoff_state = "none".to_string();
        completed.resume_target = "none".to_string();

        store
            .record_run_graph_status(&completed)
            .await
            .expect("persist approval complete run graph status");

        let receipt = store
            .run_graph_approval_delegation_receipt("run-vida-a")
            .await
            .expect("load approval complete receipt")
            .expect("approval complete receipt should exist");
        assert_eq!(receipt.transition_kind, "approval_complete");
        assert_eq!(receipt.status, "completed");
        assert_eq!(receipt.lifecycle_stage, "implementation_complete");
        assert_eq!(receipt.policy_gate, "not_required");
        assert_eq!(receipt.handoff_state, "none");
        assert_eq!(receipt.resume_target, "none");
        assert!(receipt.next_node.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_fails_closed_when_correction_requires_sealed_evidence_context() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-governance-fail-closed-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.policy_gate = "memory_correction_required".to_string();
        status.context_state = "open".to_string();

        let error = store
            .record_run_graph_status(&status)
            .await
            .expect_err("unsealed evidence context should fail closed");
        assert!(error
            .to_string()
            .contains("memory governance evidence shaping required"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_fails_closed_when_memory_governance_linkage_is_missing() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-governance-linkage-fail-closed-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.policy_gate = "memory_delete_required".to_string();
        status.context_state = "sealed".to_string();
        status.handoff_state = "awaiting_coach".to_string();

        let error = store
            .record_run_graph_status(&status)
            .await
            .expect_err("missing consent/ttl linkage should fail closed");
        assert!(error
            .to_string()
            .contains("memory governance linkage required"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_fails_closed_when_persisted_governance_state_breaks_validation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-governance-read-fail-closed-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let status = sample_run_graph_status();
        store
            .record_run_graph_status(&status)
            .await
            .expect("seed valid run graph status");
        let _: Option<GovernanceStateRow> = store
            .db
            .upsert(("governance_state", "run-vida-a"))
            .content(GovernanceStateRow {
                run_id: "run-vida-a".to_string(),
                policy_gate: "memory_correction_required".to_string(),
                handoff_state: "awaiting_coach".to_string(),
                context_state: "open".to_string(),
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("corrupt governance state in place");

        let error = store
            .run_graph_status("run-vida-a")
            .await
            .expect_err("persisted invalid governance state should fail closed on read");
        assert!(error
            .to_string()
            .contains("memory governance evidence shaping required"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_status_prefers_highest_run_id_when_updated_at_ties() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-latest-tie-break-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut first = sample_run_graph_status();
        first.run_id = "run-aaa".to_string();
        first.task_id = "task-aaa".to_string();
        first.lane_id = "lane-aaa".to_string();
        first.resume_target = "dispatch.aaa".to_string();
        store
            .record_run_graph_status(&first)
            .await
            .expect("seed first run graph status");

        let mut second = sample_run_graph_status();
        second.run_id = "run-bbb".to_string();
        second.task_id = "task-bbb".to_string();
        second.lane_id = "lane-bbb".to_string();
        second.resume_target = "dispatch.bbb".to_string();
        store
            .record_run_graph_status(&second)
            .await
            .expect("seed second run graph status");

        store
            .db
            .query("UPDATE execution_plan_state SET updated_at = '0000000000000000000';")
            .await
            .expect("normalize updated_at tie");

        let latest = store
            .latest_run_graph_status()
            .await
            .expect("load latest run graph status")
            .expect("latest run graph status should exist");
        assert_eq!(latest.run_id, "run-bbb");

        let recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("load latest recovery summary")
            .expect("latest recovery summary should exist");
        assert_eq!(recovery.run_id, "run-bbb");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_recovery_checkpoint_and_gate_summaries_fail_closed_when_latest_checkpoint_row_is_reordered_by_timestamp_drift(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-latest-checkpoint-drift-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-current".to_string();
        status.task_id = "task-current".to_string();
        status.lane_id = "lane-current".to_string();
        status.resume_target = "dispatch.current".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("seed latest run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-older"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-older".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.older".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("seed reordered checkpoint row");

        let recovery_error = store
            .latest_run_graph_recovery_summary()
            .await
            .expect_err("timestamp drifted checkpoint row should fail closed for recovery summary");
        assert!(recovery_error
            .to_string()
            .contains("latest checkpoint evidence must share the same run_id"));

        let checkpoint_error = store
            .latest_run_graph_checkpoint_summary()
            .await
            .expect_err(
                "timestamp drifted checkpoint row should fail closed for checkpoint summary",
            );
        assert!(checkpoint_error
            .to_string()
            .contains("latest checkpoint evidence must share the same run_id"));

        let gate_error = store
            .latest_run_graph_gate_summary()
            .await
            .expect_err("timestamp drifted checkpoint row should fail closed for gate summary");
        assert!(gate_error
            .to_string()
            .contains("latest checkpoint evidence must share the same run_id"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_recovery_and_gate_summary_fail_closed_on_partial_governance_corruption(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-summary-inconsistent-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let status = sample_run_graph_status();
        store
            .record_run_graph_status(&status)
            .await
            .expect("seed valid run graph status");
        let _: Option<GovernanceStateRow> = store
            .db
            .upsert(("governance_state", "run-vida-a"))
            .content(GovernanceStateRow {
                run_id: "run-vida-a".to_string(),
                policy_gate: String::new(),
                handoff_state: "none".to_string(),
                context_state: "sealed".to_string(),
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("corrupt governance state in place");

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("load latest run graph status")
            .expect("latest run graph status should exist");
        assert_eq!(latest_status.run_id, "run-vida-a");
        assert_eq!(latest_status.resume_target, "dispatch.writer_lane");
        assert_eq!(latest_status.policy_gate, "");
        assert_eq!(latest_status.handoff_state, "none");

        let recovery_error = store
            .latest_run_graph_recovery_summary()
            .await
            .expect_err("partial governance corruption should fail closed for recovery summary");
        assert!(recovery_error
            .to_string()
            .contains("run-graph recovery/gate summary is inconsistent"));

        let gate_error = store
            .latest_run_graph_gate_summary()
            .await
            .expect_err("partial governance corruption should fail closed for gate summary");
        assert!(gate_error
            .to_string()
            .contains("run-graph recovery/gate summary is inconsistent"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_recovery_checkpoint_and_gate_summaries_fail_closed_when_one_surface_row_is_missing_and_an_older_row_exists(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-partial-summary-row-drift-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let status = sample_run_graph_status();
        store
            .record_run_graph_status(&status)
            .await
            .expect("seed valid run graph status");

        let _old_governance: Option<GovernanceStateRow> = store
            .db
            .upsert(("governance_state", "run-older"))
            .content(GovernanceStateRow {
                run_id: "run-older".to_string(),
                policy_gate: "policy_gate_required".to_string(),
                handoff_state: "awaiting_coach".to_string(),
                context_state: "sealed".to_string(),
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("seed older governance row");
        let _: Option<GovernanceStateRow> = store
            .db
            .delete(("governance_state", "run-vida-a"))
            .await
            .expect("remove latest governance row");

        let latest_run_id = store
            .latest_run_graph_run_id()
            .await
            .expect("load latest run id")
            .expect("latest run id should exist");
        assert_eq!(latest_run_id, "run-vida-a");

        let recovery_error = store
            .latest_run_graph_recovery_summary()
            .await
            .expect_err("missing latest governance row should fail closed for recovery summary");
        assert!(recovery_error
            .to_string()
            .contains("recovery/checkpoint summary is inconsistent"));

        let checkpoint_error = store
            .latest_run_graph_checkpoint_summary()
            .await
            .expect_err("missing latest governance row should fail closed for checkpoint summary");
        assert!(checkpoint_error
            .to_string()
            .contains("recovery/checkpoint summary is inconsistent"));

        let gate_error = store
            .latest_run_graph_gate_summary()
            .await
            .expect_err("missing latest governance row should fail closed for gate summary");
        assert!(gate_error
            .to_string()
            .contains("recovery/checkpoint summary is inconsistent"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn delegation_gate_marks_handoff_pending_when_resume_target_is_open() {
        let status = sample_run_graph_status();

        let gate = status.delegation_gate();

        assert!(gate.delegated_cycle_open);
        assert_eq!(gate.delegated_cycle_state, "handoff_pending");
        assert_eq!(
            gate.local_exception_takeover_gate,
            "blocked_open_delegated_cycle"
        );
        assert_eq!(gate.blocker_code.as_deref(), Some("open_delegated_cycle"));
        assert_eq!(gate.reporting_pause_gate, "non_blocking_only");
        assert_eq!(gate.continuation_signal, "continue_routing_non_blocking");
    }

    #[test]
    fn delegation_gate_marks_active_lane_without_handoff_as_delegated_lane_active() {
        let mut status = sample_run_graph_status();
        status.active_node = "review_ensemble".to_string();
        status.next_node = None;
        status.status = "in_progress".to_string();
        status.lane_id = "review_ensemble_lane".to_string();
        status.lifecycle_stage = "review_active".to_string();
        status.policy_gate = "review_findings".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;

        let gate = status.delegation_gate();

        assert!(gate.delegated_cycle_open);
        assert_eq!(gate.delegated_cycle_state, "delegated_lane_active");
        assert_eq!(
            gate.local_exception_takeover_gate,
            "blocked_open_delegated_cycle"
        );
        assert_eq!(gate.blocker_code.as_deref(), Some("open_delegated_cycle"));
        assert_eq!(gate.reporting_pause_gate, "non_blocking_only");
        assert_eq!(gate.continuation_signal, "continue_routing_non_blocking");
    }

    #[test]
    fn delegation_gate_marks_completed_cycle_as_clear_and_closure_candidate() {
        let mut status = sample_run_graph_status();
        status.active_node = "review_ensemble".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lane_id = "review_ensemble_lane".to_string();
        status.lifecycle_stage = "implementation_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;

        let gate = status.delegation_gate();

        assert!(!gate.delegated_cycle_open);
        assert_eq!(gate.delegated_cycle_state, "clear");
        assert_eq!(gate.local_exception_takeover_gate, "delegated_cycle_clear");
        assert_eq!(gate.blocker_code, None);
        assert_eq!(gate.reporting_pause_gate, "closure_candidate");
        assert_eq!(gate.continuation_signal, "continue_after_reports");
    }

    #[test]
    fn run_graph_recovery_summary_reports_blocked_delegated_cycle_takeover_gate() {
        let status = sample_run_graph_status();
        let summary = RunGraphRecoverySummary::from_status(status);
        assert_eq!(
            summary.delegation_gate.local_exception_takeover_gate,
            "blocked_open_delegated_cycle"
        );
        assert_eq!(
            summary.delegation_gate.blocker_code.as_deref(),
            Some("open_delegated_cycle")
        );
        assert_eq!(
            summary.delegation_gate.continuation_signal,
            "continue_routing_non_blocking"
        );
    }

    #[test]
    fn run_graph_recovery_summary_reports_clear_delegated_cycle_takeover_gate() {
        let mut status = sample_run_graph_status();
        status.active_node = "review_ensemble".to_string();
        status.next_node = None;
        status.status = "completed".to_string();
        status.lane_id = "review_ensemble_lane".to_string();
        status.lifecycle_stage = "implementation_complete".to_string();
        status.policy_gate = "not_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = false;

        let summary = RunGraphRecoverySummary::from_status(status);
        assert_eq!(
            summary.delegation_gate.local_exception_takeover_gate,
            "delegated_cycle_clear"
        );
        assert_eq!(summary.delegation_gate.blocker_code, None);
        assert_eq!(
            summary.delegation_gate.continuation_signal,
            "continue_after_reports"
        );
    }

    fn sample_dispatch_receipt_with_status(dispatch_status: &str) -> RunGraphDispatchReceipt {
        RunGraphDispatchReceipt {
            run_id: "run-vida-a".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: dispatch_status.to_string(),
            lane_status: String::new(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("vida agent-init".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: None,
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
            recorded_at: "2026-03-15T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn run_graph_dispatch_receipt_summary_uses_recorded_exception_lane_status_until_takeover_is_explicit(
    ) {
        let mut receipt = sample_dispatch_receipt_with_status("executed");
        receipt.exception_path_receipt_id = Some("receipt-exception-1".to_string());
        receipt.supersedes_receipt_id = Some("receipt-superseded-1".to_string());

        let summary = RunGraphDispatchReceiptSummary::from_receipt(receipt);

        assert_eq!(summary.lane_status, "lane_exception_recorded");
    }

    #[test]
    fn run_graph_dispatch_receipt_summary_prefers_superseded_lane_status_over_dispatch_mapping() {
        let mut receipt = sample_dispatch_receipt_with_status("routed");
        receipt.supersedes_receipt_id = Some("receipt-superseded-2".to_string());

        let summary = RunGraphDispatchReceiptSummary::from_receipt(receipt);

        assert_eq!(summary.lane_status, "lane_superseded");
    }

    #[test]
    fn run_graph_dispatch_receipt_deserialize_tolerates_null_lane_status() {
        let receipt = sample_dispatch_receipt_with_status("executed");
        let mut value = serde_json::to_value(receipt).expect("serialize receipt");
        value["lane_status"] = serde_json::Value::Null;

        let parsed: RunGraphDispatchReceipt =
            serde_json::from_value(value).expect("deserialize receipt with null lane_status");

        assert_eq!(parsed.lane_status, "lane_open");
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_tolerates_persisted_null_lane_status() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-null-lane-status-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "runnulllanestatus".to_string();
        status.task_id = "task-nulllanestatus".to_string();
        status.resume_target = "dispatch.null.lane.status".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "runnulllanestatus"))
            .content(ResumabilityCapsuleRow {
                run_id: "runnulllanestatus".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.null.lane.status".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let mut receipt = sample_dispatch_receipt_with_status("executed");
        receipt.run_id = "runnulllanestatus".to_string();
        receipt.recorded_at = "2026-03-16T00:00:00Z".to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist dispatch receipt");
        store
            .db
            .query("UPDATE run_graph_dispatch_receipt:runnulllanestatus SET lane_status = NONE;")
            .await
            .expect("set persisted lane_status to NONE");

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("load summary")
            .expect("summary exists");
        assert_eq!(summary.lane_status, "lane_running");

        let receipt = store
            .latest_run_graph_dispatch_receipt()
            .await
            .expect("load receipt")
            .expect("receipt exists");
        assert_eq!(receipt.lane_status, "lane_running");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_uses_shared_contract_validation_flow() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-shared-contract-flow-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-shared-contract-flow".to_string();
        status.task_id = "task-shared-contract-flow".to_string();
        status.resume_target = "dispatch.shared.contract.flow".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");
        store
            .record_run_graph_dispatch_context(&RunGraphDispatchContext {
                run_id: "run-shared-contract-flow".to_string(),
                task_id: "task-shared-contract-flow".to_string(),
                request_text: "continue development".to_string(),
                role_selection: serde_json::json!({
                    "ok": true,
                    "activation_source": "test",
                    "selection_mode": "fixed",
                    "fallback_role": "orchestrator",
                    "request": "continue development",
                    "selected_role": "worker",
                    "conversational_mode": null,
                    "single_task_only": false,
                    "tracked_flow_entry": null,
                    "allow_freeform_chat": false,
                    "confidence": "high",
                    "matched_terms": [],
                    "compiled_bundle": null,
                    "execution_plan": {
                        "development_flow": {
                            "dispatch_contract": {
                                "lane_catalog": {
                                    "implementer": {
                                        "executor_backend": "opencode_cli",
                                        "fallback_executor_backend": "internal_subagents",
                                        "fanout_executor_backends": ["hermes_cli", "junior"],
                                        "activation": {
                                            "activation_agent_type": "junior",
                                            "activation_runtime_role": "worker"
                                        }
                                    }
                                }
                            }
                        },
                        "backend_admissibility_matrix": [
                            { "backend_id": "opencode_cli", "backend_class": "external_cli" },
                            { "backend_id": "hermes_cli", "backend_class": "external_cli" },
                            { "backend_id": "internal_subagents", "backend_class": "internal" },
                            { "backend_id": "junior", "backend_class": "internal" }
                        ]
                    },
                    "reason": "test"
                }),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist run graph dispatch context");

        let mut receipt = sample_dispatch_receipt_with_status("executed");
        receipt.run_id = "run-shared-contract-flow".to_string();
        receipt.lane_status = "lane_running".to_string();
        receipt.downstream_dispatch_target = Some("docflow".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers = vec![
            "pending_execution_preparation_evidence".to_string(),
            "pending_review_findings".to_string(),
        ];
        receipt.downstream_dispatch_status = Some("executed".to_string());
        receipt.downstream_dispatch_active_target = Some("docflow".to_string());
        receipt.downstream_dispatch_last_target = Some("closure".to_string());
        receipt.recorded_at = "2026-03-18T00:00:00Z".to_string();

        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist canonical dispatch receipt");

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("load latest dispatch receipt summary")
            .expect("summary exists");
        assert_eq!(summary.run_id, "run-shared-contract-flow");
        assert_eq!(summary.dispatch_status, "executed");
        assert_eq!(
            summary.downstream_dispatch_status,
            Some("executed".to_string())
        );
        assert_eq!(
            summary.downstream_dispatch_blockers,
            vec![
                "pending_execution_preparation_evidence".to_string(),
                "pending_review_findings".to_string(),
            ]
        );
        assert_eq!(
            summary.effective_execution_posture["selected_backend"],
            "junior"
        );
        assert_eq!(
            summary.effective_execution_posture["route_primary_backend"],
            "opencode_cli"
        );
        assert_eq!(
            summary.effective_execution_posture["fallback_backend"],
            "internal_subagents"
        );
        assert_eq!(
            summary.effective_execution_posture["fanout_backends"],
            serde_json::json!(["hermes_cli", "junior"])
        );
        assert_eq!(
            summary.effective_execution_posture["mixed_route_backends"],
            true
        );
        assert_eq!(
            summary.effective_execution_posture["activation_evidence_state"],
            "execution_evidence"
        );
        assert_eq!(
            summary.effective_execution_posture["receipt_backed_execution_evidence"],
            true
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn record_run_graph_dispatch_receipt_rejects_noncanonical_downstream_blockers_before_persist(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-contract-write-guard-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-contract-write-guard".to_string();
        status.task_id = "task-contract-write-guard".to_string();
        status.resume_target = "dispatch.contract.write.guard".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_dispatch_receipt_with_status("executed");
        receipt.run_id = "run-contract-write-guard".to_string();
        receipt.lane_status = "lane_running".to_string();
        receipt.downstream_dispatch_target = Some("docflow".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers = vec![
            "pending_execution_preparation_evidence".to_string(),
            "".to_string(),
        ];
        receipt.downstream_dispatch_status = Some("executed".to_string());
        receipt.downstream_dispatch_active_target = Some("docflow".to_string());
        receipt.downstream_dispatch_last_target = Some("closure".to_string());
        receipt.recorded_at = "2026-03-18T00:00:00Z".to_string();

        let error = store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect_err("noncanonical downstream blockers should be rejected before persist");
        assert!(error.to_string().contains(
            "downstream_dispatch_blockers must contain only non-empty ASCII lowercase canonical entries without whitespace, case, internal spacing, or unicode drift when downstream_dispatch_status `executed` is present"
        ));

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("load latest dispatch receipt summary after rejected write");
        assert!(summary.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_tracks_latest_status_and_derives_stale_lane_status(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-latest-status-consistency-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut older_status = sample_run_graph_status();
        older_status.run_id = "run-aaa".to_string();
        older_status.task_id = "task-aaa".to_string();
        older_status.lane_id = "lane-aaa".to_string();
        older_status.resume_target = "dispatch.aaa".to_string();
        store
            .record_run_graph_status(&older_status)
            .await
            .expect("persist older run graph status");

        let mut latest_status = sample_run_graph_status();
        latest_status.run_id = "run-bbb".to_string();
        latest_status.task_id = "task-bbb".to_string();
        latest_status.lane_id = "lane-bbb".to_string();
        latest_status.resume_target = "dispatch.bbb".to_string();
        store
            .record_run_graph_status(&latest_status)
            .await
            .expect("persist latest run graph status");

        store
            .db
            .query("UPDATE execution_plan_state SET updated_at = '0000000000000000000';")
            .await
            .expect("normalize execution_plan_state tie");

        let mut latest_status_receipt = sample_dispatch_receipt_with_status("executed");
        latest_status_receipt.run_id = "run-bbb".to_string();
        latest_status_receipt.lane_status = "lane_open".to_string();
        latest_status_receipt.exception_path_receipt_id = Some("receipt-exception-bbb".to_string());
        latest_status_receipt.recorded_at = "2026-03-16T00:00:00Z".to_string();
        store
            .record_run_graph_dispatch_receipt(&latest_status_receipt)
            .await
            .expect("persist latest-status dispatch receipt");

        let mut newer_foreign_receipt = sample_dispatch_receipt_with_status("executed");
        newer_foreign_receipt.run_id = "run-aaa".to_string();
        newer_foreign_receipt.recorded_at = "2026-03-17T00:00:00Z".to_string();
        store
            .record_run_graph_dispatch_receipt(&newer_foreign_receipt)
            .await
            .expect("persist newer foreign dispatch receipt");

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("load latest run graph status")
            .expect("latest run graph status should exist");
        assert_eq!(latest_status.run_id, "run-bbb");

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("load latest dispatch receipt summary")
            .expect("latest dispatch receipt summary should exist");
        assert_eq!(summary.run_id, "run-bbb");
        assert_eq!(summary.lane_status, "lane_exception_recorded");

        let receipt = store
            .latest_run_graph_dispatch_receipt()
            .await
            .expect("load latest dispatch receipt")
            .expect("latest dispatch receipt should exist");
        assert_eq!(receipt.run_id, "run-bbb");
        assert_eq!(receipt.lane_status, "lane_exception_recorded");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_downstream_lane_drift() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-downstream-lane-drift-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-drift".to_string();
        status.task_id = "task-drift".to_string();
        status.resume_target = "dispatch.drift".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_dispatch_receipt_with_status("executed");
        receipt.run_id = "run-drift".to_string();
        receipt.dispatch_status = "executed".to_string();
        receipt.lane_status = "lane_open".to_string();
        receipt.downstream_dispatch_target = Some("docflow".to_string());
        receipt.downstream_dispatch_ready = true;
        receipt.downstream_dispatch_blockers =
            vec!["pending_execution_preparation_evidence".to_string()];
        receipt.downstream_dispatch_status = Some("executed".to_string());
        receipt.downstream_dispatch_active_target = Some("docflow".to_string());
        receipt.downstream_dispatch_last_target = Some("closure".to_string());
        receipt.recorded_at = "2026-03-18T00:00:00Z".to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist drifted dispatch receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("drifted downstream lane signal should fail closed");
        assert!(error
            .to_string()
            .contains("run-graph dispatch receipt summary is inconsistent"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_whitespace_only_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-whitespace-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-downstream-blockers".to_string();
        status.task_id = "task-downstream-blockers".to_string();
        status.resume_target = "dispatch.downstream".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-downstream-blockers"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.downstream".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert(("run_graph_dispatch_receipt", "run-downstream-blockers"))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec!["   ".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist whitespace downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("whitespace-only downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("downstream_dispatch_blockers must contain only non-empty ASCII lowercase canonical entries without whitespace, case, internal spacing, or unicode drift when downstream_dispatch_status `executed` is present"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_mixed_canonical_and_whitespace_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-mixed-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-mixed-downstream-blockers".to_string();
        status.task_id = "task-mixed-downstream-blockers".to_string();
        status.resume_target = "dispatch.mixed".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-mixed-downstream-blockers"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-mixed-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.mixed".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-mixed-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-mixed-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending_execution_preparation_evidence".to_string(),
                    " pending_execution_preparation_evidence ".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist mixed downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("mixed canonical and whitespace downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("without whitespace, case, internal spacing, or unicode drift"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_empty_string_and_canonical_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-empty-string-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-empty-string-downstream-blockers".to_string();
        status.task_id = "task-empty-string-downstream-blockers".to_string();
        status.resume_target = "dispatch.empty.string".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert((
                "resumability_capsule",
                "run-empty-string-downstream-blockers",
            ))
            .content(ResumabilityCapsuleRow {
                run_id: "run-empty-string-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.empty.string".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-empty-string-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-empty-string-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "".to_string(),
                    "pending_execution_preparation_evidence".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist empty-string downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("empty-string downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("non-empty ASCII lowercase canonical entries"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_tab_and_newline_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-tab-newline-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-tab-newline-downstream-blockers".to_string();
        status.task_id = "task-tab-newline-downstream-blockers".to_string();
        status.resume_target = "dispatch.tab.newline".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert((
                "resumability_capsule",
                "run-tab-newline-downstream-blockers",
            ))
            .content(ResumabilityCapsuleRow {
                run_id: "run-tab-newline-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.tab.newline".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-tab-newline-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-tab-newline-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "\tpending_execution_preparation_evidence\n".to_string(),
                    "pending_review_findings".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist tab/newline downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("tab/newline downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("non-empty ASCII lowercase canonical entries"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_trailing_empty_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-trailing-empty-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-trailing-empty-downstream-blockers".to_string();
        status.task_id = "task-trailing-empty-downstream-blockers".to_string();
        status.resume_target = "dispatch.trailing.empty".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert((
                "resumability_capsule",
                "run-trailing-empty-downstream-blockers",
            ))
            .content(ResumabilityCapsuleRow {
                run_id: "run-trailing-empty-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.trailing.empty".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-trailing-empty-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-trailing-empty-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending_execution_preparation_evidence".to_string(),
                    "".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist trailing empty downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("trailing empty downstream blockers should fail closed");
        let expected_fragment = "downstream_dispatch_blockers must contain only non-empty ASCII lowercase canonical entries without whitespace, case, internal spacing, or unicode drift when downstream_dispatch_status `executed` is present";
        assert!(error.to_string().contains(expected_fragment));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_duplicate_canonical_and_whitespace_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-duplicate-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-duplicate-downstream-blockers".to_string();
        status.task_id = "task-duplicate-downstream-blockers".to_string();
        status.resume_target = "dispatch.duplicate".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-duplicate-downstream-blockers"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-duplicate-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.duplicate".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-duplicate-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-duplicate-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending_execution_preparation_evidence".to_string(),
                    "pending_execution_preparation_evidence".to_string(),
                    " pending_execution_preparation_evidence ".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist duplicate canonical downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err(
                "duplicate canonical and whitespace downstream blockers should fail closed",
            );
        assert!(error
            .to_string()
            .contains("without whitespace, case, internal spacing, or unicode drift"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_repeated_canonical_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-repeated-canonical-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-repeated-canonical-downstream-blockers".to_string();
        status.task_id = "task-repeated-canonical-downstream-blockers".to_string();
        status.resume_target = "dispatch.repeated.canonical".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert((
                "resumability_capsule",
                "run-repeated-canonical-downstream-blockers",
            ))
            .content(ResumabilityCapsuleRow {
                run_id: "run-repeated-canonical-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.repeated.canonical".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-repeated-canonical-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-repeated-canonical-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending_execution_preparation_evidence".to_string(),
                    "pending_execution_preparation_evidence".to_string(),
                    "pending_execution_preparation_evidence".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist repeated canonical downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("repeated canonical downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("duplicate canonical entries after lowercase canonicalization"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_large_repeated_canonical_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-large-repeated-canonical-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-large-repeated-canonical-downstream-blockers".to_string();
        status.task_id = "task-large-repeated-canonical-downstream-blockers".to_string();
        status.resume_target = "dispatch.large.repeated.canonical".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert((
                "resumability_capsule",
                "run-large-repeated-canonical-downstream-blockers",
            ))
            .content(ResumabilityCapsuleRow {
                run_id: "run-large-repeated-canonical-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.large.repeated.canonical".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let repeated_blocker = "pending_execution_preparation_evidence".to_string();
        let large_repeated_blockers = vec![repeated_blocker; 2048];
        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-large-repeated-canonical-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-large-repeated-canonical-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: large_repeated_blockers,
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist large repeated canonical downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("large repeated canonical downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("duplicate canonical entries after lowercase canonicalization"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_mixed_case_duplicate_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-mixed-case-duplicate-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-mixed-case-downstream-blockers".to_string();
        status.task_id = "task-mixed-case-downstream-blockers".to_string();
        status.resume_target = "dispatch.mixed.case".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-mixed-case-downstream-blockers"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-mixed-case-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.mixed.case".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-mixed-case-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-mixed-case-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending_execution_preparation_evidence".to_string(),
                    "PENDING_EXECUTION_PREPARATION_EVIDENCE".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist mixed-case duplicate downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("mixed-case duplicate downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("without whitespace, case, internal spacing, or unicode drift"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_internal_repeated_space_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-internal-space-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-internal-space-downstream-blockers".to_string();
        status.task_id = "task-internal-space-downstream-blockers".to_string();
        status.resume_target = "dispatch.internal.space".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert((
                "resumability_capsule",
                "run-internal-space-downstream-blockers",
            ))
            .content(ResumabilityCapsuleRow {
                run_id: "run-internal-space-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.internal.space".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-internal-space-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-internal-space-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending review findings".to_string(),
                    "pending  review findings".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist internal-space downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("internal repeated spaces in downstream blockers should fail closed");
        assert!(error
            .to_string()
            .contains("without whitespace, case, internal spacing, or unicode drift"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_unicode_zero_width_downstream_blockers(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-unicode-zero-width-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-unicode-zero-width-downstream-blockers".to_string();
        status.task_id = "task-unicode-zero-width-downstream-blockers".to_string();
        status.resume_target = "dispatch.unicode.zero.width".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert((
                "resumability_capsule",
                "run-unicode-zero-width-downstream-blockers",
            ))
            .content(ResumabilityCapsuleRow {
                run_id: "run-unicode-zero-width-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.unicode.zero.width".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-unicode-zero-width-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-unicode-zero-width-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending_execution_preparation_evidence".to_string(),
                    "pending_execution_preparation_evidence\u{200B}".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist unicode zero-width downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("unicode zero-width downstream blockers should fail closed");
        assert!(error.to_string().contains("unicode drift"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_missing_downstream_blockers_fallback(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-missing-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-missing-downstream-blockers".to_string();
        status.task_id = "task-missing-downstream-blockers".to_string();
        status.resume_target = "dispatch.missing.blockers".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-missing-downstream-blockers"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-missing-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.missing.blockers".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-missing-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-missing-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: Vec::new(),
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("blocked".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist downstream blockers fallback receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("missing downstream blockers fallback should fail closed");
        assert!(error
            .to_string()
            .contains("downstream_dispatch_blockers must be present and non-empty"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_normalizes_canonical_downstream_blocker_order(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-sorted-downstream-blockers-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-sorted-downstream-blockers".to_string();
        status.task_id = "task-sorted-downstream-blockers".to_string();
        status.resume_target = "dispatch.sorted".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-sorted-downstream-blockers"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-sorted-downstream-blockers".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.sorted".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("persist matching checkpoint row");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert((
                "run_graph_dispatch_receipt",
                "run-sorted-downstream-blockers",
            ))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-sorted-downstream-blockers".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("lane_running".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
                blocker_code: None,
                downstream_dispatch_target: Some("docflow".to_string()),
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: true,
                downstream_dispatch_blockers: vec![
                    "pending_review_findings".to_string(),
                    "pending_execution_preparation_evidence".to_string(),
                ],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: Some("executed".to_string()),
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 1,
                downstream_dispatch_active_target: Some("docflow".to_string()),
                downstream_dispatch_last_target: Some("closure".to_string()),
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("junior".to_string()),
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist unsorted canonical downstream blockers receipt");

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("load latest dispatch receipt summary")
            .expect("latest dispatch receipt summary should exist");
        assert_eq!(
            summary.downstream_dispatch_blockers,
            vec![
                "pending_execution_preparation_evidence".to_string(),
                "pending_review_findings".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_when_latest_checkpoint_row_leaks_from_older_run(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-checkpoint-leak-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-current".to_string();
        status.task_id = "task-current".to_string();
        status.lane_id = "lane-current".to_string();
        status.resume_target = "dispatch.current".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist current run graph status");

        let mut receipt = sample_dispatch_receipt_with_status("executed");
        receipt.run_id = "run-current".to_string();
        receipt.recorded_at = "2026-03-18T00:00:00Z".to_string();
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist current dispatch receipt");

        let _: Option<ResumabilityCapsuleRow> = store
            .db
            .upsert(("resumability_capsule", "run-older"))
            .content(ResumabilityCapsuleRow {
                run_id: "run-older".to_string(),
                checkpoint_kind: "execution_cursor".to_string(),
                resume_target: "dispatch.older".to_string(),
                recovery_ready: true,
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await
            .expect("seed leaked older checkpoint row");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err(
                "checkpoint leakage should fail closed for latest dispatch receipt summary",
            );
        assert!(error
            .to_string()
            .contains("latest checkpoint evidence must share the same run_id"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_fails_closed_on_whitespace_only_critical_fields(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-whitespace-critical-fields-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-whitespace".to_string();
        status.task_id = "task-whitespace".to_string();
        status.lane_id = "lane-whitespace".to_string();
        status.resume_target = "dispatch.whitespace".to_string();
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert(("run_graph_dispatch_receipt", "run-whitespace"))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-whitespace".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "   ".to_string(),
                lane_status: Some("lane_open".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
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
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist whitespace dispatch_status receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("whitespace-only dispatch_status should fail closed");
        assert!(error
            .to_string()
            .contains("dispatch_status must be non-empty"));

        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert(("run_graph_dispatch_receipt", "run-whitespace"))
            .content(RunGraphDispatchReceiptStored {
                run_id: "run-whitespace".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "executed".to_string(),
                lane_status: Some("   ".to_string()),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: None,
                dispatch_packet_path: None,
                dispatch_result_path: None,
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
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist whitespace lane_status receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("whitespace-only lane_status should fail closed");
        assert!(error.to_string().contains("lane_status must be non-empty"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn migration_preflight_reports_no_migration_required_for_seeded_runtime() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-migration-preflight-seeded-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");

        let summary = store
            .evaluate_migration_preflight()
            .await
            .expect("migration preflight should succeed");
        assert_eq!(summary.contract_type, "release-1-operator-contracts");
        assert_eq!(summary.schema_version, "release-1-v1");
        assert_eq!(summary.compatibility_classification, "backward_compatible");
        assert_eq!(summary.migration_state, "no_migration_required");
        assert!(summary.blockers.is_empty());
        assert_eq!(
            summary.source_version_tuple,
            vec![
                "framework-agent-definition@v1".to_string(),
                "framework-instruction-contract@v1".to_string(),
                "framework-prompt-template-config@v1".to_string()
            ]
        );
        let receipt_summary = store
            .migration_receipt_summary()
            .await
            .expect("migration receipt summary should load");
        assert_eq!(receipt_summary.compatibility_receipts, 1);
        assert_eq!(receipt_summary.application_receipts, 0);
        assert_eq!(receipt_summary.verification_receipts, 0);
        assert_eq!(receipt_summary.cutover_readiness_receipts, 0);
        assert_eq!(receipt_summary.rollback_notes, 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn migration_preflight_blocks_when_runtime_root_is_missing() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-migration-preflight-missing-runtime-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");
        let _: Option<InstructionRuntimeStateRow> = store
            .db
            .delete(("instruction_runtime_state", "primary"))
            .await
            .expect("delete runtime state");

        let summary = store
            .evaluate_migration_preflight()
            .await
            .expect("migration preflight should succeed");
        assert_eq!(
            summary.compatibility_classification,
            "reader_upgrade_required"
        );
        assert_eq!(summary.migration_state, "migration_blocked");
        assert!(summary
            .blockers
            .iter()
            .any(|blocker| blocker.contains("instruction runtime root unresolved")));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn migration_preflight_blocks_on_state_spine_contract_drift() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-migration-preflight-state-spine-drift-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");
        let _: Option<StateSpineManifestContent> = store
            .db
            .upsert(("state_spine_manifest", "primary"))
            .content(StateSpineManifestContent {
                manifest_id: "primary".to_string(),
                state_schema_version: 1,
                authoritative_mutation_root: "legacy task".to_string(),
                entity_surfaces: vec![
                    "task".to_string(),
                    "task_dependency".to_string(),
                    "task_blocker".to_string(),
                ],
                initialized_at: "123".to_string(),
            })
            .await
            .expect("update state spine manifest");

        let summary = store
            .evaluate_migration_preflight()
            .await
            .expect("migration preflight should succeed");
        assert_eq!(
            summary.compatibility_classification,
            "reader_upgrade_required"
        );
        assert_eq!(summary.migration_state, "migration_blocked");
        assert!(summary
            .blockers
            .iter()
            .any(|blocker| blocker.contains("authoritative state spine manifest is invalid")));
        assert!(summary
            .blockers
            .iter()
            .any(|blocker| blocker.contains("authoritative_mutation_root=legacy task")));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn migration_preflight_summary_and_receipts_persist_across_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-migration-preflight-reopen-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");

        let summary = store
            .evaluate_migration_preflight()
            .await
            .expect("migration preflight should succeed");
        assert_eq!(summary.contract_type, "release-1-operator-contracts");
        assert_eq!(summary.schema_version, "release-1-v1");
        assert_eq!(summary.compatibility_classification, "backward_compatible");
        assert_eq!(summary.migration_state, "no_migration_required");

        drop(store);

        let mut reopened = None;
        for _ in 0..10 {
            match StateStore::open_existing(root.clone()).await {
                Ok(store) => {
                    reopened = Some(store);
                    break;
                }
                Err(StateStoreError::Db(_)) => {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(other) => panic!("open existing store: {other}"),
            }
        }
        let reopened = reopened.expect("open existing store");

        let persisted = reopened
            .latest_migration_preflight_summary()
            .await
            .expect("latest migration preflight should load")
            .expect("persisted migration preflight should exist");
        assert_eq!(persisted.contract_type, "release-1-operator-contracts");
        assert_eq!(persisted.schema_version, "release-1-v1");
        assert_eq!(
            persisted.compatibility_classification,
            "backward_compatible"
        );
        assert_eq!(persisted.migration_state, "no_migration_required");
        assert!(persisted.blockers.is_empty());
        assert_eq!(
            persisted.source_version_tuple,
            vec![
                "framework-agent-definition@v1".to_string(),
                "framework-instruction-contract@v1".to_string(),
                "framework-prompt-template-config@v1".to_string()
            ]
        );
        assert_eq!(persisted.next_step, "normal_boot_allowed");

        let receipts = reopened
            .migration_receipt_summary()
            .await
            .expect("migration receipt summary should load");
        assert_eq!(receipts.compatibility_receipts, 1);
        assert_eq!(receipts.application_receipts, 0);
        assert_eq!(receipts.verification_receipts, 0);
        assert_eq!(receipts.cutover_readiness_receipts, 0);
        assert_eq!(receipts.rollback_notes, 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn seed_framework_instruction_bundle_preserves_existing_active_root() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-instruction-runtime-preserve-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("initial seed should succeed");

        let _: Option<InstructionRuntimeStateRow> = store
            .db
            .upsert(("instruction_runtime_state", "primary"))
            .content(InstructionRuntimeStateRow {
                state_id: "primary".to_string(),
                active_root_artifact_id: "custom-root".to_string(),
                runtime_mode: "test_override".to_string(),
            })
            .await
            .expect("override runtime state");

        store
            .seed_framework_instruction_bundle()
            .await
            .expect("reseed should preserve runtime state");

        let row: Option<InstructionRuntimeStateRow> = store
            .db
            .select(("instruction_runtime_state", "primary"))
            .await
            .expect("select runtime state");
        let row = row.expect("runtime state should exist");
        assert_eq!(row.active_root_artifact_id, "custom-root");
        assert_eq!(row.runtime_mode, "test_override");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn project_instruction_artifact_applies_minimal_sidecar_ops() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-test-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");

        store
            .upsert_instruction_diff_patch(InstructionDiffPatchContent {
                patch_id: "test-projection-patch".to_string(),
                target_artifact_id: "framework-instruction-contract".to_string(),
                target_artifact_version: 1,
                target_artifact_hash: blake3::hash(
                    fs::read_to_string(
                        repo_root()
                            .join(DEFAULT_INSTRUCTION_SOURCE_ROOT)
                            .join("framework/instruction-contract.md"),
                    )
                    .expect("read instruction contract source")
                    .as_bytes(),
                )
                .to_hex()
                .to_string(),
                patch_precedence: 10,
                author_class: "test".to_string(),
                applies_if: "always".to_string(),
                created_at: "2026-03-08T00:00:00Z".to_string(),
                active: true,
                operations: vec![
                    InstructionPatchOperation {
                        op: "replace_range".to_string(),
                        target_mode: "exact_text".to_string(),
                        target: "artifact_kind: instruction_contract".to_string(),
                        with_lines: vec!["artifact_kind: instruction_contract_patched".to_string()],
                    },
                    InstructionPatchOperation {
                        op: "insert_after".to_string(),
                        target_mode: "exact_text".to_string(),
                        target: "ownership_class: framework".to_string(),
                        with_lines: vec!["clarification: sidecar-added-line".to_string()],
                    },
                    InstructionPatchOperation {
                        op: "delete_range".to_string(),
                        target_mode: "exact_text".to_string(),
                        target: "hierarchy: framework".to_string(),
                        with_lines: vec![],
                    },
                    InstructionPatchOperation {
                        op: "append_block".to_string(),
                        target_mode: "exact_text".to_string(),
                        target: "mutability_class: immutable".to_string(),
                        with_lines: vec![
                            "appendix: extra guidance".to_string(),
                            "appendix: follow patched runtime".to_string(),
                        ],
                    },
                ],
            })
            .await
            .expect("upsert diff patch");

        let projection = store
            .project_instruction_artifact("framework-instruction-contract")
            .await
            .expect("project artifact");

        assert_eq!(projection.artifact_id, "framework-instruction-contract");
        assert_eq!(projection.applied_patch_ids, vec!["test-projection-patch"]);
        assert!(projection
            .body
            .contains("artifact_kind: instruction_contract_patched"));
        assert!(projection
            .body
            .contains("clarification: sidecar-added-line"));
        assert!(!projection.body.contains("hierarchy: framework"));
        assert!(projection.body.contains("appendix: extra guidance"));
        assert!(!projection.projected_hash.is_empty());

        let mut receipt_query = store
            .db
            .query("SELECT count() AS count FROM instruction_projection_receipt GROUP ALL;")
            .await
            .expect("count projection receipts");
        #[derive(Debug, serde::Deserialize, SurrealValue)]
        struct CountRow {
            count: i64,
        }
        let receipt_rows: Vec<CountRow> = receipt_query.take(0).expect("take receipt count");
        assert_eq!(receipt_rows.first().map(|row| row.count), Some(1));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_patch_operation_supports_line_span_targeting() {
        let mut lines = vec!["one".to_string(), "two".to_string(), "three".to_string()];

        apply_patch_operation(
            &mut lines,
            &InstructionPatchOperation {
                op: "insert_before".to_string(),
                target_mode: "line_span".to_string(),
                target: "2".to_string(),
                with_lines: vec!["between".to_string()],
            },
        )
        .expect("line_span op should succeed");

        assert_eq!(lines, vec!["one", "between", "two", "three"]);
    }

    #[test]
    fn apply_patch_operation_supports_anchor_hash_targeting() {
        let mut lines = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let anchor = format!("blake3:{}", blake3::hash("two".as_bytes()).to_hex());

        apply_patch_operation(
            &mut lines,
            &InstructionPatchOperation {
                op: "insert_after".to_string(),
                target_mode: "anchor_hash".to_string(),
                target: anchor,
                with_lines: vec!["after-two".to_string()],
            },
        )
        .expect("anchor_hash op should succeed");

        assert_eq!(lines, vec!["one", "two", "after-two", "three"]);
    }

    #[test]
    fn apply_patch_operation_fails_closed_on_stale_anchor_hash() {
        let mut lines = vec!["one".to_string(), "two".to_string()];

        let error = apply_patch_operation(
            &mut lines,
            &InstructionPatchOperation {
                op: "replace_range".to_string(),
                target_mode: "anchor_hash".to_string(),
                target: format!("blake3:{}", blake3::hash("stale".as_bytes()).to_hex()),
                with_lines: vec!["new".to_string()],
            },
        )
        .expect_err("stale anchor hash should fail");

        assert!(matches!(
            error,
            StateStoreError::InvalidPatchOperation { .. }
        ));
    }

    #[test]
    fn validate_patch_conflicts_fails_on_equal_precedence_same_anchor() {
        let patches = vec![
            InstructionDiffPatchRow {
                patch_id: "patch-a".to_string(),
                target_artifact_id: "artifact".to_string(),
                target_artifact_version: 1,
                target_artifact_hash: "base-hash".to_string(),
                patch_precedence: 10,
                active: true,
                operations: vec![InstructionPatchOperation {
                    op: "replace_range".to_string(),
                    target_mode: "exact_text".to_string(),
                    target: "anchor".to_string(),
                    with_lines: vec!["a".to_string()],
                }],
            },
            InstructionDiffPatchRow {
                patch_id: "patch-b".to_string(),
                target_artifact_id: "artifact".to_string(),
                target_artifact_version: 1,
                target_artifact_hash: "base-hash".to_string(),
                patch_precedence: 10,
                active: true,
                operations: vec![InstructionPatchOperation {
                    op: "delete_range".to_string(),
                    target_mode: "exact_text".to_string(),
                    target: "anchor".to_string(),
                    with_lines: vec![],
                }],
            },
        ];

        let error = validate_patch_conflicts(&patches).expect_err("conflict should fail");
        assert!(matches!(error, StateStoreError::PatchConflict { .. }));
    }

    #[test]
    fn apply_patch_operation_fails_closed_on_missing_anchor() {
        let mut lines = vec!["one".to_string()];

        let error = apply_patch_operation(
            &mut lines,
            &InstructionPatchOperation {
                op: "replace_range".to_string(),
                target_mode: "exact_text".to_string(),
                target: "missing".to_string(),
                with_lines: vec!["new".to_string()],
            },
        )
        .expect_err("missing anchor should fail");

        assert!(matches!(
            error,
            StateStoreError::InvalidPatchOperation { .. }
        ));
    }

    #[test]
    fn validate_patch_conflicts_fails_on_equal_precedence_same_anchor_hash() {
        let anchor = format!("blake3:{}", blake3::hash("anchor".as_bytes()).to_hex());
        let patches = vec![
            InstructionDiffPatchRow {
                patch_id: "patch-a".to_string(),
                target_artifact_id: "artifact".to_string(),
                target_artifact_version: 1,
                target_artifact_hash: "base-hash".to_string(),
                patch_precedence: 10,
                active: true,
                operations: vec![InstructionPatchOperation {
                    op: "replace_range".to_string(),
                    target_mode: "anchor_hash".to_string(),
                    target: anchor.clone(),
                    with_lines: vec!["a".to_string()],
                }],
            },
            InstructionDiffPatchRow {
                patch_id: "patch-b".to_string(),
                target_artifact_id: "artifact".to_string(),
                target_artifact_version: 1,
                target_artifact_hash: "base-hash".to_string(),
                patch_precedence: 10,
                active: true,
                operations: vec![InstructionPatchOperation {
                    op: "delete_range".to_string(),
                    target_mode: "anchor_hash".to_string(),
                    target: anchor,
                    with_lines: vec![],
                }],
            },
        ];

        let error =
            validate_patch_conflicts(&patches).expect_err("anchor_hash conflict should fail");
        assert!(matches!(error, StateStoreError::PatchConflict { .. }));
    }

    #[tokio::test]
    async fn project_instruction_artifact_fails_on_stale_patch_binding() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-projection-binding-test-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");

        store
            .upsert_instruction_diff_patch(InstructionDiffPatchContent {
                patch_id: "stale-binding-patch".to_string(),
                target_artifact_id: "framework-instruction-contract".to_string(),
                target_artifact_version: 1,
                target_artifact_hash: "stale-hash".to_string(),
                patch_precedence: 10,
                author_class: "test".to_string(),
                applies_if: "always".to_string(),
                created_at: "2026-03-08T00:00:00Z".to_string(),
                active: true,
                operations: vec![InstructionPatchOperation {
                    op: "replace_range".to_string(),
                    target_mode: "exact_text".to_string(),
                    target: "artifact_kind: instruction_contract".to_string(),
                    with_lines: vec!["artifact_kind: changed".to_string()],
                }],
            })
            .await
            .expect("upsert stale patch");

        let error = store
            .project_instruction_artifact("framework-instruction-contract")
            .await
            .expect_err("stale binding should fail");
        assert!(matches!(
            error,
            StateStoreError::InvalidPatchOperation { .. }
        ));

        let mut receipt_query = store
            .db
            .query("SELECT * FROM instruction_projection_receipt;")
            .await
            .expect("query projection receipts");
        let receipts: Vec<InstructionProjectionReceiptContent> =
            receipt_query.take(0).expect("take receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].applied_patch_ids.len(), 0);
        assert_eq!(receipts[0].skipped_patch_ids, vec!["stale-binding-patch"]);
        assert!(receipts[0].failed_reason.contains("targets artifact hash"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_effective_instruction_bundle_returns_mandatory_chain_in_order() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-effective-bundle-test-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed bundle");
        store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree");

        let bundle = store
            .resolve_effective_instruction_bundle("framework-agent-definition")
            .await
            .expect("resolve bundle");

        assert_eq!(
            bundle.mandatory_chain_order,
            vec![
                "framework-agent-definition",
                "framework-instruction-contract",
                "framework-prompt-template-config",
            ]
        );
        assert_eq!(
            bundle.source_version_tuple,
            vec![
                "framework-agent-definition@v1",
                "framework-instruction-contract@v1",
                "framework-prompt-template-config@v1",
            ]
        );
        assert_eq!(bundle.projected_artifacts.len(), 3);
        assert_eq!(
            bundle.projected_artifacts[0].artifact_id,
            "framework-agent-definition"
        );
        assert!(bundle
            .receipt_id
            .starts_with("effective-bundle-framework-agent-definition-"));

        let mut receipt_query = store
            .db
            .query("SELECT * FROM effective_instruction_bundle_receipt;")
            .await
            .expect("query bundle receipts");
        let receipts: Vec<EffectiveInstructionBundleReceiptContent> =
            receipt_query.take(0).expect("take bundle receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            receipts[0].mandatory_chain_order,
            bundle.mandatory_chain_order
        );
        assert_eq!(
            receipts[0].source_version_tuple,
            bundle.source_version_tuple
        );
        assert_eq!(receipts[0].optional_triggered_reads, Vec::<String>::new());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_mandatory_chain_handles_diamond_graph_topologically() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-diamond-graph-test-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        for artifact_id in ["test-a", "test-b", "test-c", "test-d"] {
            let _: Option<InstructionArtifactContent> = store
                .db
                .upsert(("instruction_artifact", artifact_id))
                .content(InstructionArtifactContent {
                    artifact_id: artifact_id.to_string(),
                    artifact_kind: "instruction_contract".to_string(),
                    version: 1,
                    ownership_class: "framework".to_string(),
                    mutability_class: "immutable".to_string(),
                    activation_class: "always_on".to_string(),
                    source_hash: format!("hash-{artifact_id}"),
                    body: artifact_id.to_string(),
                    hierarchy: vec!["framework".to_string()],
                    required_follow_on: vec![],
                })
                .await
                .expect("insert diamond artifact");
        }
        for (edge_id, from_artifact, to_artifact) in [
            ("test-a__test-b", "test-a", "test-b"),
            ("test-a__test-c", "test-a", "test-c"),
            ("test-b__test-d", "test-b", "test-d"),
            ("test-c__test-d", "test-c", "test-d"),
        ] {
            let _: Option<InstructionDependencyEdgeContent> = store
                .db
                .upsert(("instruction_dependency_edge", edge_id))
                .content(InstructionDependencyEdgeContent {
                    from_artifact: from_artifact.to_string(),
                    to_artifact: to_artifact.to_string(),
                    edge_kind: "mandatory_follow_on".to_string(),
                })
                .await
                .expect("insert diamond edge");
        }

        let ordered = store
            .resolve_mandatory_chain("test-a")
            .await
            .expect("resolve diamond graph");

        let pos_a = ordered
            .iter()
            .position(|id| id == "test-a")
            .expect("a present");
        let pos_b = ordered
            .iter()
            .position(|id| id == "test-b")
            .expect("b present");
        let pos_c = ordered
            .iter()
            .position(|id| id == "test-c")
            .expect("c present");
        let pos_d = ordered
            .iter()
            .position(|id| id == "test-d")
            .expect("d present");
        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_mandatory_chain_fails_closed_on_cycle() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-cycle-graph-test-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        for artifact_id in ["test-a", "test-b", "test-c"] {
            let _: Option<InstructionArtifactContent> = store
                .db
                .upsert(("instruction_artifact", artifact_id))
                .content(InstructionArtifactContent {
                    artifact_id: artifact_id.to_string(),
                    artifact_kind: "instruction_contract".to_string(),
                    version: 1,
                    ownership_class: "framework".to_string(),
                    mutability_class: "immutable".to_string(),
                    activation_class: "always_on".to_string(),
                    source_hash: format!("hash-{artifact_id}"),
                    body: artifact_id.to_string(),
                    hierarchy: vec!["framework".to_string()],
                    required_follow_on: vec![],
                })
                .await
                .expect("insert cycle artifact");
        }
        for (edge_id, from_artifact, to_artifact) in [
            ("test-a__test-b", "test-a", "test-b"),
            ("test-b__test-c", "test-b", "test-c"),
            ("test-c__test-a", "test-c", "test-a"),
        ] {
            let _: Option<InstructionDependencyEdgeContent> = store
                .db
                .upsert(("instruction_dependency_edge", edge_id))
                .content(InstructionDependencyEdgeContent {
                    from_artifact: from_artifact.to_string(),
                    to_artifact: to_artifact.to_string(),
                    edge_kind: "mandatory_follow_on".to_string(),
                })
                .await
                .expect("insert cycle edge");
        }

        let error = store
            .resolve_mandatory_chain("test-a")
            .await
            .expect_err("cycle should fail");
        match error {
            StateStoreError::InstructionDependencyCycle { cycle_path } => {
                assert!(cycle_path.contains("test-a"));
                assert!(cycle_path.contains("test-b"));
                assert!(cycle_path.contains("test-c"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn resolve_mandatory_chain_fails_on_missing_dependency_target() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-missing-dependency-test-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let _: Option<InstructionArtifactContent> = store
            .db
            .upsert(("instruction_artifact", "test-a"))
            .content(InstructionArtifactContent {
                artifact_id: "test-a".to_string(),
                artifact_kind: "agent_definition".to_string(),
                version: 1,
                ownership_class: "framework".to_string(),
                mutability_class: "immutable".to_string(),
                activation_class: "always_on".to_string(),
                source_hash: "hash-a".to_string(),
                body: "test-a".to_string(),
                hierarchy: vec!["framework".to_string()],
                required_follow_on: vec!["missing-b".to_string()],
            })
            .await
            .expect("insert root artifact");
        let _: Option<InstructionDependencyEdgeContent> = store
            .db
            .upsert(("instruction_dependency_edge", "test-a__missing-b"))
            .content(InstructionDependencyEdgeContent {
                from_artifact: "test-a".to_string(),
                to_artifact: "missing-b".to_string(),
                edge_kind: "mandatory_follow_on".to_string(),
            })
            .await
            .expect("insert missing edge");

        let error = store
            .resolve_mandatory_chain("test-a")
            .await
            .expect_err("missing dependency should fail");
        match error {
            StateStoreError::MissingInstructionArtifact { artifact_id } => {
                assert_eq!(artifact_id, "missing-b");
            }
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn authoritative_store_exports_taskflow_snapshot_and_round_trips_to_memory() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-export-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"in_progress\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"closed\",\"priority\":3,\"issue_type\":\"bug\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write task jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks should succeed");

        let snapshot = store
            .export_taskflow_snapshot()
            .await
            .expect("canonical snapshot export should succeed");
        assert_eq!(snapshot.tasks.len(), 3);
        assert_eq!(snapshot.tasks[0].id.0, "vida-a");
        assert_eq!(snapshot.tasks[1].id.0, "vida-b");
        assert_eq!(snapshot.tasks[2].id.0, "vida-root");
        assert!(matches!(
            snapshot.tasks[0].status,
            CanonicalTaskStatus::InProgress
        ));
        assert!(matches!(
            snapshot.tasks[1].status,
            CanonicalTaskStatus::Closed
        ));
        assert!(matches!(
            snapshot.tasks[1].issue_type,
            CanonicalIssueType::Bug
        ));
        assert_eq!(snapshot.dependencies.len(), 2);
        assert_eq!(snapshot.dependencies[0].issue_id.0, "vida-a");
        assert_eq!(snapshot.dependencies[0].depends_on_id.0, "vida-root");
        assert_eq!(snapshot.dependencies[1].issue_id.0, "vida-b");
        assert_eq!(snapshot.dependencies[1].dependency_type, "blocks");

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "export_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_object");
        assert_eq!(latest.task_count, 3);
        assert_eq!(latest.dependency_count, 2);
        assert_eq!(latest.stale_removed_count, 0);

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 1);
        assert_eq!(rollup.by_operation.get("export_snapshot"), Some(&1));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_object"),
            Some(&1)
        );

        let memory = store
            .export_taskflow_in_memory_store()
            .await
            .expect("memory export should succeed");
        let runtime = taskflow_state::TaskStore::get_task(&memory, &CanonicalTaskId::new("vida-b"))
            .expect("task should exist in memory export");
        assert_eq!(runtime.title, "Task B");
        assert!(matches!(runtime.status, CanonicalTaskStatus::Closed));
        let runtime_dependencies =
            taskflow_state::TaskStore::list_dependencies(&memory, &CanonicalTaskId::new("vida-b"));
        assert_eq!(runtime_dependencies.len(), 1);
        assert_eq!(runtime_dependencies[0].depends_on_id.0, "vida-a");

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "export_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 3);
        assert_eq!(latest.dependency_count, 2);
        assert_eq!(latest.stale_removed_count, 0);

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 2);
        assert_eq!(rollup.by_operation.get("export_snapshot"), Some(&2));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_memory"),
            Some(&1)
        );
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_object"),
            Some(&1)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn authoritative_store_taskflow_snapshot_export_fails_closed_on_unsupported_issue_type() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-export-invalid-issue-type-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let _: Option<TaskStorageRow> = store
            .db
            .upsert(("task", "vida-weird"))
            .content(TaskStorageRow {
                task_id: "vida-weird".to_string(),
                display_id: None,
                title: "Weird".to_string(),
                description: "unsupported issue type".to_string(),
                status: "open".to_string(),
                priority: 1,
                issue_type: "chore".to_string(),
                created_at: "2026-03-08T00:00:00Z".to_string(),
                created_by: "tester".to_string(),
                updated_at: "2026-03-08T00:00:00Z".to_string(),
                closed_at: None,
                close_reason: None,
                source_repo: ".".to_string(),
                compaction_level: 0,
                original_size: 0,
                notes: None,
                labels: Vec::new(),
                execution_semantics: TaskExecutionSemantics::default(),
                dependencies: Vec::new(),
            })
            .await
            .expect("insert weird task");

        let error = store
            .export_taskflow_snapshot()
            .await
            .expect_err("unsupported issue type should fail");
        match error {
            StateStoreError::InvalidCanonicalTaskflowExport { reason } => {
                assert!(reason.contains("unsupported taskflow-core issue_type mapping: chore"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn authoritative_store_writes_taskflow_snapshot_to_disk_and_restores_it() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-file-export-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        let snapshot_path = root.join("taskflow-snapshot.json");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write task jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks should succeed");

        store
            .write_taskflow_snapshot(&snapshot_path)
            .await
            .expect("snapshot should write to disk");
        assert!(snapshot_path.is_file());

        let restored = StateStore::read_taskflow_snapshot_into_memory(&snapshot_path)
            .expect("snapshot should restore from disk");
        let restored_task =
            taskflow_state::TaskStore::get_task(&restored, &CanonicalTaskId::new("vida-a"))
                .expect("task should restore from snapshot");
        assert_eq!(restored_task.title, "Task A");
        let restored_dependencies = taskflow_state::TaskStore::list_dependencies(
            &restored,
            &CanonicalTaskId::new("vida-a"),
        );
        assert_eq!(restored_dependencies.len(), 1);
        assert_eq!(restored_dependencies[0].depends_on_id.0, "vida-root");

        let mut receipt_query = store
            .db
            .query("SELECT * FROM task_reconciliation_summary ORDER BY recorded_at DESC LIMIT 1;")
            .await
            .expect("query reconciliation receipts");
        let receipts: Vec<TaskReconciliationSummaryRow> =
            receipt_query.take(0).expect("take reconciliation receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].operation, "export_snapshot");
        assert_eq!(receipts[0].source_kind, "canonical_snapshot_file");
        assert_eq!(
            receipts[0].source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );
        assert_eq!(receipts[0].task_count, 2);
        assert_eq!(receipts[0].dependency_count, 1);
        assert_eq!(receipts[0].stale_removed_count, 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_task_reconciliation_summary_synthesizes_runtime_consumption_final_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-reconciliation-final-summary-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let snapshot_path = root
            .join("runtime-consumption")
            .join("final-2026-03-08T00-00-00Z.json");
        fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("snapshot parent should exist"),
        )
        .expect("create runtime-consumption directory");
        fs::write(
            &snapshot_path,
            r#"{"surface":"vida taskflow consume final","status":"pass","operator_contracts":{"status":"pass"},"payload":{"closure_admission":{"status":"pass","admitted":true,"blockers":[],"proof_surfaces":[]}}}"#,
        )
        .expect("write final snapshot");

        let summary = store
            .record_runtime_consumption_final_task_reconciliation_summary(Some(
                snapshot_path.display().to_string(),
            ))
            .await
            .expect("synthetic reconciliation summary should persist");
        let snapshot_path_string = snapshot_path.display().to_string();
        assert_eq!(summary.operation, "consume_final");
        assert_eq!(summary.source_kind, "runtime_consumption_final_snapshot");
        assert_eq!(
            summary.source_path.as_deref(),
            Some(snapshot_path_string.as_str())
        );
        assert_eq!(summary.task_count, 0);
        assert_eq!(summary.dependency_count, 0);
        assert_eq!(summary.stale_removed_count, 0);

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.receipt_id, summary.receipt_id);
        assert_eq!(latest.operation, "consume_final");
        assert_eq!(latest.source_kind, "runtime_consumption_final_snapshot");
        assert_eq!(
            latest.source_path.as_deref(),
            Some(snapshot_path_string.as_str())
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn task_reconciliation_rollup_synthesizes_runtime_consumption_final_receipt() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-task-reconciliation-final-rollup-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let snapshot_path = root
            .join("runtime-consumption")
            .join("final-2026-03-08T00-00-01Z.json");
        fs::create_dir_all(
            snapshot_path
                .parent()
                .expect("snapshot parent should exist"),
        )
        .expect("create runtime-consumption directory");
        fs::write(
            &snapshot_path,
            r#"{"surface":"vida taskflow consume final","status":"pass","operator_contracts":{"status":"pass"},"payload":{"closure_admission":{"status":"pass","admitted":true,"blockers":[],"proof_surfaces":[]}}}"#,
        )
        .expect("write final snapshot");

        store
            .record_runtime_consumption_final_task_reconciliation_summary(Some(
                snapshot_path.display().to_string(),
            ))
            .await
            .expect("synthetic reconciliation summary should persist");
        let snapshot_path_string = snapshot_path.display().to_string();

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 1);
        assert_eq!(rollup.total_task_rows, 0);
        assert_eq!(rollup.total_dependency_rows, 0);
        assert_eq!(rollup.total_stale_removed, 0);
        assert_eq!(rollup.by_operation.get("consume_final"), Some(&1));
        assert_eq!(
            rollup
                .by_source_kind
                .get("runtime_consumption_final_snapshot"),
            Some(&1)
        );
        assert_eq!(
            rollup.latest_source_path.as_deref(),
            Some(snapshot_path_string.as_str())
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn authoritative_store_imports_canonical_taskflow_snapshot() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let snapshot = TaskSnapshot {
            tasks: vec![
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-root"),
                    title: "Root".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-a"),
                    title: "Task A".to_string(),
                    status: CanonicalTaskStatus::InProgress,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:01Z", &Rfc3339)
                            .expect("parse task timestamp"),
                    ),
                },
            ],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-a"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };

        store
            .import_taskflow_snapshot(&snapshot)
            .await
            .expect("snapshot import should succeed");

        let imported = store.show_task("vida-a").await.expect("task should import");
        assert_eq!(imported.title, "Task A");
        assert_eq!(imported.status, "in_progress");
        assert_eq!(imported.issue_type, "task");
        assert_eq!(imported.created_by, "taskflow-state-fs");
        assert_eq!(imported.source_repo, "taskflow-state-fs");
        assert_eq!(imported.dependencies.len(), 1);
        assert_eq!(imported.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(imported.dependencies[0].created_by, "taskflow-state-fs");
        assert_eq!(
            imported.dependencies[0].created_at,
            "canonical-taskflow-snapshot"
        );

        let ready = store.ready_tasks().await.expect("ready tasks should load");
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "vida-a");

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "import_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 2);
        assert_eq!(latest.dependency_count, 1);
        assert_eq!(latest.stale_removed_count, 0);

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 1);
        assert_eq!(rollup.by_operation.get("import_snapshot"), Some(&1));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_memory"),
            Some(&1)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn authoritative_store_imports_canonical_taskflow_snapshot_from_disk() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-file-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let snapshot_path = root.join("snapshot.json");
        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-b"),
                title: "Task B".to_string(),
                status: CanonicalTaskStatus::Closed,
                issue_type: CanonicalIssueType::Bug,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:02Z", &Rfc3339)
                        .expect("parse task timestamp"),
                ),
            }],
            dependencies: Vec::new(),
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &snapshot)
            .expect("snapshot should write");

        store
            .import_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect("snapshot file import should succeed");

        let imported = store.show_task("vida-b").await.expect("task should import");
        assert_eq!(imported.status, "closed");
        assert_eq!(imported.issue_type, "bug");
        assert_eq!(imported.created_by, "taskflow-state-fs");
        assert_eq!(imported.source_repo, "taskflow-state-fs");
        assert_eq!(imported.closed_at.as_deref(), Some("2026-03-08T00:00:02Z"));
        assert_eq!(
            imported.close_reason.as_deref(),
            Some("imported_from_canonical_taskflow_snapshot")
        );

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "import_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_file");
        assert_eq!(
            latest.source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );
        assert_eq!(latest.task_count, 1);
        assert_eq!(latest.dependency_count, 0);
        assert_eq!(latest.stale_removed_count, 0);

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 1);
        assert_eq!(rollup.by_operation.get("import_snapshot"), Some(&1));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_file"),
            Some(&1)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn replace_with_taskflow_snapshot_removes_stale_authoritative_tasks() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-replace-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep old\",\"description\":\"keep\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-keep"),
                title: "Keep new".to_string(),
                status: CanonicalTaskStatus::Closed,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse timestamp"),
                ),
            }],
            dependencies: Vec::new(),
        };

        store
            .replace_with_taskflow_snapshot(&snapshot)
            .await
            .expect("replacement import should succeed");

        let kept = store
            .show_task("vida-keep")
            .await
            .expect("keep task should remain");
        assert_eq!(kept.title, "Keep new");
        assert_eq!(kept.status, "closed");
        let missing = store
            .show_task("vida-stale")
            .await
            .expect_err("stale task should be removed");
        assert!(matches!(missing, StateStoreError::MissingTask { .. }));

        let mut receipt_query = store
            .db
            .query("SELECT * FROM task_reconciliation_summary ORDER BY recorded_at DESC LIMIT 1;")
            .await
            .expect("query reconciliation receipts");
        let receipts: Vec<TaskReconciliationSummaryRow> =
            receipt_query.take(0).expect("take reconciliation receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].operation, "replace_snapshot");
        assert_eq!(receipts[0].source_kind, "canonical_snapshot_memory");
        assert_eq!(receipts[0].task_count, 1);
        assert_eq!(receipts[0].stale_removed_count, 1);

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "replace_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 1);
        assert_eq!(latest.stale_removed_count, 1);
        assert!(latest
            .as_display()
            .contains("replace_snapshot via canonical_snapshot_memory"));

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 1);
        assert_eq!(rollup.by_operation.get("replace_snapshot"), Some(&1));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_memory"),
            Some(&1)
        );
        assert!(rollup.latest_recorded_at.is_some());
        assert!(
            rollup
                .as_display()
                .contains("1 receipts (tasks=1, dependencies=0, stale_removed=1, operations: replace_snapshot=1; source_kinds: canonical_snapshot_memory=1;")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_file_fails_closed_before_mutation_on_post_merge_parent_conflict(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-file-import-parent-conflict-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        let snapshot_path = root.join("snapshot.json");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root-a\",\"title\":\"Root A\",\"description\":\"root a\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-root-b\",\"title\":\"Root B\",\"description\":\"root b\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-child\",\"title\":\"Child old\",\"description\":\"child\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-child\",\"depends_on_id\":\"vida-root-a\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-child"),
                title: "Child new".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse child timestamp"),
                ),
            }],
            dependencies: vec![
                CanonicalDependencyEdge {
                    issue_id: CanonicalTaskId::new("vida-child"),
                    depends_on_id: CanonicalTaskId::new("vida-root-a"),
                    dependency_type: "parent-child".to_string(),
                },
                CanonicalDependencyEdge {
                    issue_id: CanonicalTaskId::new("vida-child"),
                    depends_on_id: CanonicalTaskId::new("vida-root-b"),
                    dependency_type: "parent-child".to_string(),
                },
            ],
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &snapshot).expect("write snapshot");

        let error = store
            .import_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect_err("post-merge multiple-parent conflict should fail");
        match error {
            StateStoreError::InvalidCanonicalTaskflowExport { reason } => {
                assert!(reason.contains("snapshot graph is invalid after additive merge"));
                assert!(reason.contains("multiple_parent_edges"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let after_child = store
            .show_task("vida-child")
            .await
            .expect("child should still exist after rejected file import");
        assert_eq!(after_child.title, "Child old");
        assert_eq!(after_child.dependencies.len(), 1);
        assert_eq!(after_child.dependencies[0].depends_on_id, "vida-root-a");

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load");
        assert!(
            latest.is_none(),
            "rejected file import must not emit reconciliation receipt"
        );

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 0);
        assert_eq!(bridge.import_receipts, 0);
        assert_eq!(bridge.memory_import_receipts, 0);
        assert_eq!(bridge.file_import_receipts, 0);
        assert!(bridge.latest_operation.is_none());
        assert!(bridge.latest_source_kind.is_none());
        assert!(bridge.latest_source_path.is_none());

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn additive_imports_accumulate_mixed_memory_and_file_rollup_totals() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-mixed-rollup-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        let snapshot_path = root.join("snapshot.json");
        fs::write(
            &source,
            "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let memory_snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-a"),
                title: "Task A".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse task a timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-a"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        store
            .import_taskflow_snapshot(&memory_snapshot)
            .await
            .expect("memory additive import should succeed");

        let file_snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-b"),
                title: "Task B".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:06Z", &Rfc3339)
                        .expect("parse task b timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-b"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &file_snapshot).expect("write snapshot");
        store
            .import_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect("file additive import should succeed");

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 2);
        assert_eq!(rollup.by_operation.get("import_snapshot"), Some(&2));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_memory"),
            Some(&1)
        );
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_file"),
            Some(&1)
        );
        assert_eq!(rollup.total_task_rows, 2);
        assert_eq!(rollup.total_dependency_rows, 2);
        assert_eq!(rollup.total_stale_removed, 0);
        assert_eq!(
            rollup.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 2);
        assert_eq!(bridge.import_receipts, 2);
        assert_eq!(bridge.memory_import_receipts, 1);
        assert_eq!(bridge.file_import_receipts, 1);
        assert_eq!(bridge.total_task_rows, 2);
        assert_eq!(bridge.total_dependency_rows, 2);
        assert_eq!(bridge.total_stale_removed, 0);
        assert_eq!(bridge.latest_operation.as_deref(), Some("import_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_file")
        );
        assert_eq!(
            bridge.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let tasks = store.all_tasks().await.expect("tasks should load");
        assert_eq!(tasks.len(), 3);

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_and_replace_accumulate_cross_operation_rollup_totals() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-replace-rollup-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        let snapshot_path = root.join("replace-snapshot.json");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let memory_snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-a"),
                title: "Task A".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse task a timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-a"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        store
            .import_taskflow_snapshot(&memory_snapshot)
            .await
            .expect("memory additive import should succeed");

        let replace_snapshot = TaskSnapshot {
            tasks: vec![
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-root"),
                    title: "Root".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-a"),
                    title: "Task A replaced".to_string(),
                    status: CanonicalTaskStatus::Closed,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:06Z", &Rfc3339)
                            .expect("parse task a replace timestamp"),
                    ),
                },
            ],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-a"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &replace_snapshot)
            .expect("write replace snapshot");
        store
            .replace_with_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect("file-backed replace should succeed");

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 2);
        assert_eq!(rollup.by_operation.get("import_snapshot"), Some(&1));
        assert_eq!(rollup.by_operation.get("replace_snapshot"), Some(&1));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_memory"),
            Some(&1)
        );
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_file"),
            Some(&1)
        );
        assert_eq!(rollup.total_task_rows, 3);
        assert_eq!(rollup.total_dependency_rows, 2);
        assert_eq!(rollup.total_stale_removed, 1);
        assert_eq!(
            rollup.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 2);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.replace_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 1);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.memory_replace_receipts, 0);
        assert_eq!(bridge.file_replace_receipts, 1);
        assert_eq!(bridge.total_task_rows, 3);
        assert_eq!(bridge.total_dependency_rows, 2);
        assert_eq!(bridge.total_stale_removed, 1);
        assert_eq!(bridge.latest_operation.as_deref(), Some("replace_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_file")
        );
        assert_eq!(
            bridge.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let stale = store
            .show_task("vida-stale")
            .await
            .expect_err("stale task should be removed");
        assert!(matches!(stale, StateStoreError::MissingTask { .. }));
        let replaced = store
            .show_task("vida-a")
            .await
            .expect("task a should remain");
        assert_eq!(replaced.title, "Task A replaced");
        assert_eq!(replaced.status, "closed");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn reconciliation_receipts_and_summaries_persist_across_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-reconciliation-reopen-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        let snapshot_path = root.join("replace-snapshot.json");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let memory_snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-a"),
                title: "Task A".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse task a timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-a"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        store
            .import_taskflow_snapshot(&memory_snapshot)
            .await
            .expect("memory additive import should succeed");

        let replace_snapshot = TaskSnapshot {
            tasks: vec![
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-root"),
                    title: "Root".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-a"),
                    title: "Task A replaced".to_string(),
                    status: CanonicalTaskStatus::Closed,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:06Z", &Rfc3339)
                            .expect("parse task a replace timestamp"),
                    ),
                },
            ],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-a"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &replace_snapshot)
            .expect("write replace snapshot");
        store
            .replace_with_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect("file-backed replace should succeed");

        drop(store);

        let mut reopened = None;
        for _ in 0..10 {
            match StateStore::open_existing(root.clone()).await {
                Ok(store) => {
                    reopened = Some(store);
                    break;
                }
                Err(StateStoreError::Db(_)) => {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(other) => panic!("open existing store: {other}"),
            }
        }
        let reopened = reopened.expect("open existing store");

        let latest = reopened
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "replace_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_file");
        assert_eq!(
            latest.source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );
        assert_eq!(latest.stale_removed_count, 1);

        let rollup = reopened
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 2);
        assert_eq!(rollup.by_operation.get("import_snapshot"), Some(&1));
        assert_eq!(rollup.by_operation.get("replace_snapshot"), Some(&1));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_memory"),
            Some(&1)
        );
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_file"),
            Some(&1)
        );
        assert_eq!(rollup.total_task_rows, 3);
        assert_eq!(rollup.total_dependency_rows, 2);
        assert_eq!(rollup.total_stale_removed, 1);
        assert_eq!(
            rollup.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let bridge = reopened
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 2);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.replace_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 1);
        assert_eq!(bridge.file_replace_receipts, 1);
        assert_eq!(bridge.total_task_rows, 3);
        assert_eq!(bridge.total_dependency_rows, 2);
        assert_eq!(bridge.total_stale_removed, 1);
        assert_eq!(bridge.latest_operation.as_deref(), Some("replace_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_file")
        );
        assert_eq!(
            bridge.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let replaced = reopened
            .show_task("vida-a")
            .await
            .expect("task a should remain");
        assert_eq!(replaced.title, "Task A replaced");
        let stale = reopened
            .show_task("vida-stale")
            .await
            .expect_err("stale task should remain removed after reopen");
        assert!(matches!(stale, StateStoreError::MissingTask { .. }));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn replace_with_taskflow_snapshot_file_records_file_receipt_and_rollup() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-replace-file-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        let snapshot_path = root.join("snapshot.json");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep old\",\"description\":\"keep\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-keep"),
                title: "Keep replacement".to_string(),
                status: CanonicalTaskStatus::Closed,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse timestamp"),
                ),
            }],
            dependencies: Vec::new(),
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &snapshot)
            .expect("snapshot should write");

        store
            .replace_with_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect("replacement file import should succeed");

        let kept = store
            .show_task("vida-keep")
            .await
            .expect("keep task should remain");
        assert_eq!(kept.title, "Keep replacement");
        let missing = store
            .show_task("vida-stale")
            .await
            .expect_err("stale task should be removed");
        assert!(matches!(missing, StateStoreError::MissingTask { .. }));

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "replace_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_file");
        assert_eq!(
            latest.source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );
        assert_eq!(latest.task_count, 1);
        assert_eq!(latest.dependency_count, 0);
        assert_eq!(latest.stale_removed_count, 1);

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 1);
        assert_eq!(rollup.total_task_rows, 1);
        assert_eq!(rollup.total_dependency_rows, 0);
        assert_eq!(rollup.total_stale_removed, 1);
        assert_eq!(rollup.by_operation.get("replace_snapshot"), Some(&1));
        assert_eq!(
            rollup.by_source_kind.get("canonical_snapshot_file"),
            Some(&1)
        );
        assert_eq!(
            rollup.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 1);
        assert_eq!(bridge.export_receipts, 0);
        assert_eq!(bridge.import_receipts, 0);
        assert_eq!(bridge.replace_receipts, 1);
        assert_eq!(bridge.object_export_receipts, 0);
        assert_eq!(bridge.memory_export_receipts, 0);
        assert_eq!(bridge.memory_import_receipts, 0);
        assert_eq!(bridge.memory_replace_receipts, 0);
        assert_eq!(bridge.file_export_receipts, 0);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.file_replace_receipts, 1);
        assert_eq!(bridge.total_task_rows, 1);
        assert_eq!(bridge.total_dependency_rows, 0);
        assert_eq!(bridge.total_stale_removed, 1);
        assert_eq!(bridge.latest_operation.as_deref(), Some("replace_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_file")
        );
        assert_eq!(
            bridge.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
        );
        assert!(
            bridge
                .as_display()
                .contains("receipts=1 export=0 import=0 replace=1 object=0 memory=0 file=1 tasks=1 dependencies=0 stale_removed=1 latest=replace_snapshot via canonical_snapshot_file")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn canonical_snapshot_bridge_round_trips_across_authoritative_stores() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let source_root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-bridge-source-{}-{}",
            std::process::id(),
            nanos
        ));
        let destination_root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-bridge-destination-{}-{}",
            std::process::id(),
            nanos
        ));

        let source_store = StateStore::open(source_root.clone())
            .await
            .expect("open source store");
        let destination_store = StateStore::open(destination_root.clone())
            .await
            .expect("open destination store");
        let source = source_root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"in_progress\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:01Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"bug\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:02Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write task jsonl");
        source_store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import source tasks should succeed");

        let exported = source_store
            .export_taskflow_snapshot()
            .await
            .expect("source snapshot export should succeed");
        destination_store
            .replace_with_taskflow_snapshot(&exported)
            .await
            .expect("destination replace should succeed");
        let re_exported = destination_store
            .export_taskflow_snapshot()
            .await
            .expect("destination snapshot export should succeed");

        let exported_task_rows = exported
            .tasks
            .iter()
            .map(|task| {
                (
                    task.id.0.clone(),
                    task.title.clone(),
                    canonical_task_status_label(task.status).to_string(),
                    canonical_issue_type_label(task.issue_type).to_string(),
                    canonical_timestamp_label(&task.updated_at),
                )
            })
            .collect::<Vec<_>>();
        let re_exported_task_rows = re_exported
            .tasks
            .iter()
            .map(|task| {
                (
                    task.id.0.clone(),
                    task.title.clone(),
                    canonical_task_status_label(task.status).to_string(),
                    canonical_issue_type_label(task.issue_type).to_string(),
                    canonical_timestamp_label(&task.updated_at),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(re_exported_task_rows, exported_task_rows);

        let exported_dependency_rows = exported
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.issue_id.0.clone(),
                    dependency.depends_on_id.0.clone(),
                    dependency.dependency_type.clone(),
                )
            })
            .collect::<Vec<_>>();
        let re_exported_dependency_rows = re_exported
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.issue_id.0.clone(),
                    dependency.depends_on_id.0.clone(),
                    dependency.dependency_type.clone(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(re_exported_dependency_rows, exported_dependency_rows);

        let destination_ready = destination_store
            .ready_tasks()
            .await
            .expect("destination ready tasks should load");
        assert_eq!(destination_ready.len(), 1);
        assert_eq!(destination_ready[0].id, "vida-a");

        let bridge = destination_store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 2);
        assert_eq!(bridge.export_receipts, 1);
        assert_eq!(bridge.replace_receipts, 1);
        assert_eq!(bridge.import_receipts, 0);
        assert_eq!(bridge.object_export_receipts, 1);
        assert_eq!(bridge.memory_export_receipts, 0);
        assert_eq!(bridge.memory_import_receipts, 0);
        assert_eq!(bridge.memory_replace_receipts, 1);
        assert_eq!(bridge.file_export_receipts, 0);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.file_replace_receipts, 0);

        let _ = fs::remove_dir_all(&source_root);
        let _ = fs::remove_dir_all(&destination_root);
    }

    #[tokio::test]
    async fn file_backed_snapshot_bridge_round_trips_across_authoritative_stores() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let source_root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-file-bridge-source-{}-{}",
            std::process::id(),
            nanos
        ));
        let destination_root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-file-bridge-destination-{}-{}",
            std::process::id(),
            nanos
        ));

        let source_store = StateStore::open(source_root.clone())
            .await
            .expect("open source store");
        let destination_store = StateStore::open(destination_root.clone())
            .await
            .expect("open destination store");
        let source = source_root.join("tasks.jsonl");
        let snapshot_path = source_root.join("bridge-snapshot.json");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root epic\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:01Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-a\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"closed\",\"priority\":3,\"issue_type\":\"bug\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:02Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write task jsonl");
        source_store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import source tasks should succeed");

        source_store
            .write_taskflow_snapshot(&snapshot_path)
            .await
            .expect("file-backed snapshot export should succeed");
        destination_store
            .replace_with_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect("destination file-backed replace should succeed");
        let re_exported = destination_store
            .export_taskflow_snapshot()
            .await
            .expect("destination snapshot export should succeed");

        let re_exported_task_rows = re_exported
            .tasks
            .iter()
            .map(|task| {
                (
                    task.id.0.clone(),
                    task.title.clone(),
                    canonical_task_status_label(task.status).to_string(),
                    canonical_issue_type_label(task.issue_type).to_string(),
                    canonical_timestamp_label(&task.updated_at),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            re_exported_task_rows,
            vec![
                (
                    "vida-a".to_string(),
                    "Task A".to_string(),
                    "open".to_string(),
                    "task".to_string(),
                    "2026-03-08T00:00:01Z".to_string(),
                ),
                (
                    "vida-b".to_string(),
                    "Task B".to_string(),
                    "closed".to_string(),
                    "bug".to_string(),
                    "2026-03-08T00:00:02Z".to_string(),
                ),
                (
                    "vida-root".to_string(),
                    "Root epic".to_string(),
                    "open".to_string(),
                    "epic".to_string(),
                    "2026-03-08T00:00:00Z".to_string(),
                ),
            ]
        );

        let re_exported_dependency_rows = re_exported
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.issue_id.0.clone(),
                    dependency.depends_on_id.0.clone(),
                    dependency.dependency_type.clone(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            re_exported_dependency_rows,
            vec![
                (
                    "vida-a".to_string(),
                    "vida-root".to_string(),
                    "parent-child".to_string(),
                ),
                (
                    "vida-b".to_string(),
                    "vida-a".to_string(),
                    "blocks".to_string(),
                ),
            ]
        );

        let destination_ready = destination_store
            .ready_tasks()
            .await
            .expect("destination ready tasks should load");
        assert_eq!(destination_ready.len(), 1);
        assert_eq!(destination_ready[0].id, "vida-a");

        let bridge = destination_store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 2);
        assert_eq!(bridge.export_receipts, 1);
        assert_eq!(bridge.replace_receipts, 1);
        assert_eq!(bridge.import_receipts, 0);
        assert_eq!(bridge.object_export_receipts, 1);
        assert_eq!(bridge.memory_export_receipts, 0);
        assert_eq!(bridge.memory_import_receipts, 0);
        assert_eq!(bridge.memory_replace_receipts, 0);
        assert_eq!(bridge.file_export_receipts, 0);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.file_replace_receipts, 1);
        assert_eq!(bridge.latest_operation.as_deref(), Some("export_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_object")
        );

        let _ = fs::remove_dir_all(&source_root);
        let _ = fs::remove_dir_all(&destination_root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_fails_closed_before_mutation_on_invalid_graph() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-invalid-graph-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-child"),
                title: "Child".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                        .expect("parse timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-child"),
                depends_on_id: CanonicalTaskId::new("vida-missing"),
                dependency_type: "blocks".to_string(),
            }],
        };

        let error = store
            .import_taskflow_snapshot(&snapshot)
            .await
            .expect_err("invalid graph should fail");
        match error {
            StateStoreError::InvalidCanonicalTaskflowExport { reason } => {
                assert!(reason.contains("snapshot graph is invalid"));
                assert!(reason.contains("missing_dependency"));
            }
            other => panic!("unexpected error: {other}"),
        }

        let tasks = store.all_tasks().await.expect("tasks should still load");
        assert!(tasks.is_empty(), "invalid import must not mutate store");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_reconciles_closure_ready_downstream_receipt_into_closure_candidate() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-status-closure-candidate-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-closure-ready".to_string();
        status.task_id = "task-closure-ready".to_string();
        status.active_node = "dev-pack".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lane_id = "dev_pack_direct".to_string();
        status.lifecycle_stage = "dev_pack_active".to_string();
        status.policy_gate = "single_task_scope_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-closure-ready".to_string(),
            dispatch_target: "work-pool-pack".to_string(),
            dispatch_status: "executed".to_string(),
            lane_status: "lane_running".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "taskflow_pack".to_string(),
            dispatch_surface: Some("vida task ensure".to_string()),
            dispatch_command: None,
            dispatch_packet_path: None,
            dispatch_result_path: Some("/tmp/result.json".to_string()),
            blocker_code: None,
            downstream_dispatch_target: Some("closure".to_string()),
            downstream_dispatch_command: None,
            downstream_dispatch_note: Some(
                "no additional downstream lane is required by the current execution plan after this handoff"
                    .to_string(),
            ),
            downstream_dispatch_ready: true,
            downstream_dispatch_blockers: Vec::new(),
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: Some("packet_ready".to_string()),
            downstream_dispatch_result_path: Some("/tmp/downstream-result.json".to_string()),
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 1,
            downstream_dispatch_active_target: Some("verification".to_string()),
            downstream_dispatch_last_target: Some("verification".to_string()),
            activation_agent_type: None,
            activation_runtime_role: None,
            selected_backend: Some("taskflow_state_store".to_string()),
            recorded_at: "2026-03-18T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist closure-ready dispatch receipt");

        let reconciled = store
            .run_graph_status("run-closure-ready")
            .await
            .expect("reconciled run graph status should load");
        assert_eq!(reconciled.active_node, "verification");
        assert_eq!(reconciled.status, "completed");
        assert_eq!(reconciled.lifecycle_stage, "implementation_complete");
        assert_eq!(reconciled.policy_gate, "not_required");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(!reconciled.recovery_ready);

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("latest reconciled run graph status should load")
            .expect("latest run graph status should exist");
        assert_eq!(latest_status.active_node, "verification");
        assert_eq!(latest_status.status, "completed");
        assert_eq!(latest_status.lifecycle_stage, "implementation_complete");
        assert_eq!(latest_status.policy_gate, "not_required");
        assert_eq!(latest_status.handoff_state, "none");
        assert_eq!(latest_status.resume_target, "none");
        assert!(!latest_status.recovery_ready);

        let recovery = store
            .run_graph_recovery_summary("run-closure-ready")
            .await
            .expect("reconciled recovery summary should load");
        assert_eq!(recovery.active_node, "verification");
        assert_eq!(recovery.resume_status, "completed");
        assert_eq!(recovery.lifecycle_stage, "implementation_complete");
        assert_eq!(recovery.delegation_gate.blocker_code, None);
        assert_eq!(
            recovery.delegation_gate.reporting_pause_gate,
            "closure_candidate"
        );

        let latest_recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("latest reconciled recovery summary should load")
            .expect("latest run graph recovery summary should exist");
        assert_eq!(latest_recovery.active_node, "verification");
        assert_eq!(latest_recovery.resume_status, "completed");
        assert_eq!(latest_recovery.lifecycle_stage, "implementation_complete");
        assert_eq!(latest_recovery.delegation_gate.blocker_code, None);
        assert_eq!(
            latest_recovery.delegation_gate.reporting_pause_gate,
            "closure_candidate"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_recovery_summary_fails_closed_when_latest_dispatch_receipt_is_blocked() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-recovery-blocked-receipt-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-blocked-recovery".to_string();
        status.task_id = "task-blocked-recovery".to_string();
        status.active_node = "coach".to_string();
        status.next_node = None;
        status.status = "ready".to_string();
        status.lane_id = "coach_lane".to_string();
        status.lifecycle_stage = "coach_active".to_string();
        status.policy_gate = "validation_report_required".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "none".to_string();
        status.recovery_ready = true;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let receipt = RunGraphDispatchReceipt {
            run_id: "run-blocked-recovery".to_string(),
            dispatch_target: "coach".to_string(),
            dispatch_status: "blocked".to_string(),
            lane_status: "lane_blocked".to_string(),
            supersedes_receipt_id: None,
            exception_path_receipt_id: None,
            dispatch_kind: "agent_lane".to_string(),
            dispatch_surface: Some("external_cli:hermes_cli".to_string()),
            dispatch_command: Some("hermes ...".to_string()),
            dispatch_packet_path: Some("/tmp/dispatch-packet.json".to_string()),
            dispatch_result_path: Some("/tmp/dispatch-result.json".to_string()),
            blocker_code: Some("timeout_without_takeover_authority".to_string()),
            downstream_dispatch_target: Some("verification".to_string()),
            downstream_dispatch_command: Some("vida agent-init".to_string()),
            downstream_dispatch_note: Some("after review".to_string()),
            downstream_dispatch_ready: false,
            downstream_dispatch_blockers: vec!["pending_review_clean_evidence".to_string()],
            downstream_dispatch_packet_path: None,
            downstream_dispatch_status: None,
            downstream_dispatch_result_path: None,
            downstream_dispatch_trace_path: None,
            downstream_dispatch_executed_count: 0,
            downstream_dispatch_active_target: Some("coach".to_string()),
            downstream_dispatch_last_target: Some("coach".to_string()),
            activation_agent_type: Some("middle".to_string()),
            activation_runtime_role: Some("coach".to_string()),
            selected_backend: Some("hermes_cli".to_string()),
            recorded_at: "2026-04-16T00:00:00Z".to_string(),
        };
        store
            .record_run_graph_dispatch_receipt(&receipt)
            .await
            .expect("persist blocked dispatch receipt");

        let reconciled = store
            .run_graph_status("run-blocked-recovery")
            .await
            .expect("reconciled run graph status should load");
        assert_eq!(reconciled.status, "blocked");
        assert_eq!(reconciled.selected_backend, "hermes_cli");
        assert!(!reconciled.recovery_ready);

        let recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("recovery summary should load")
            .expect("recovery summary should exist");
        assert_eq!(recovery.run_id, "run-blocked-recovery");
        assert_eq!(recovery.resume_status, "blocked");
        assert!(!recovery.recovery_ready);
        assert!(recovery.delegation_gate.delegated_cycle_open);
        assert_eq!(
            recovery.delegation_gate.blocker_code.as_deref(),
            Some("open_delegated_cycle")
        );

        let _ = fs::remove_dir_all(&root);
    }
}
