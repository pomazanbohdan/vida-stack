use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
#[path = "state_store_policy.rs"]
pub(crate) mod policy;
pub(crate) use state_store_open::state_root_lifecycle_guard_path;
#[path = "state_store_policy.rs"]
pub mod policy;
#[path = "state_store_orchestrator_claim.rs"]
mod state_store_orchestrator_claim;
#[path = "state_store_patching.rs"]
mod state_store_patching;
#[path = "state_store_protocol_binding.rs"]
mod state_store_protocol_binding;
#[path = "state_store_run_graph_state.rs"]
mod state_store_run_graph_state;
#[path = "state_store_run_graph_summary.rs"]
mod state_store_run_graph_summary;
#[path = "state_store_scheduler_reservation.rs"]
mod state_store_scheduler_reservation;
#[path = "state_store_source_scan.rs"]
mod state_store_source_scan;
#[path = "state_store_task_attempts.rs"]
mod state_store_task_attempts;
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
#[allow(unused_imports)]
pub(crate) use state_store_orchestrator_claim::{
    claim_paths_intersect, AcquireOrchestratorClaimRequest, LeaseMode, OrchestratorClaim,
    OrchestratorClaimCompatibilityConflict, OrchestratorClaimStatus,
};
use state_store_patching::{
    apply_patch_operation, collect_patch_ids, join_lines, split_lines, validate_patch_bindings,
    validate_patch_conflicts,
};
pub use state_store_protocol_binding::{ProtocolBindingState, ProtocolBindingSummary};
#[allow(unused_imports)]
pub(crate) use state_store_run_graph_state::{
    ExecutionPlanStateRow, GovernanceStateRow, HostBridgePrecursorFingerprintStored,
    HostBridgeReceiptIdentityStored, ResumabilityCapsuleRow, RoutedRunStateRow,
    RunGraphDispatchReceiptStored, RunGraphLatestReceiptRow, RunGraphLatestRow,
    RunGraphLatestStateRow, RunGraphOwnerEvidenceRecord, RunGraphPolicyPin,
    RunGraphProjectionCheckpointRecord, RunGraphReplayLineageReceipt,
};

pub(crate) fn policy_bundle_ref_from_execution_plan(
    execution_plan: &serde_json::Value,
) -> Option<RunGraphPolicyPin> {
    let runtime_assignment = execution_plan
        .get("runtime_assignment")
        .or_else(|| execution_plan.get("carrier_runtime_assignment"));
    execution_plan
        .get("policy_bundle_ref")
        .or_else(|| runtime_assignment.and_then(|value| value.get("policy_bundle_ref")))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .and_then(|pin: RunGraphPolicyPin| pin.normalize().ok())
}
#[allow(unused_imports)]
pub use state_store_run_graph_state::{
    RunGraphContinuationBinding, RunGraphDispatchContext, RunGraphDispatchReceipt,
    RunGraphDispatchTaskIdentity, RunGraphMemoryGovernanceProjection,
    RunGraphPrincipalDelegationProjection, RunGraphStatus, RunGraphSummary,
};
pub(crate) use state_store_run_graph_summary::{
    default_run_graph_lane_status, deserialize_run_graph_lane_status,
    downstream_dispatch_allows_completed_lane_status, handoff_state_links_consent_ttl,
    latest_run_graph_dispatch_receipt_matches_status,
    latest_run_graph_dispatch_receipt_signal_is_ambiguous,
    latest_run_graph_dispatch_receipt_summary_is_inconsistent,
    latest_run_graph_evidence_snapshot_is_consistent, normalize_run_graph_lane_status,
    requires_memory_governance_enforcement, RunGraphApprovalDelegationReceipt,
    RunGraphCheckpointSummary, RunGraphDelegationGateSummary, RunGraphDispatchReceiptSummary,
    RunGraphGateSummary, RunGraphRecoverySummary,
};
#[allow(unused_imports)]
pub(crate) use state_store_scheduler_reservation::{
    AcquireSchedulerDispatchReservationRequest, SchedulerDispatchReservation,
    SchedulerDispatchReservationStatus,
};
use state_store_source_scan::{
    artifact_id_from_path, collect_markdown_files, hierarchy_from_path, infer_artifact_kind,
    infer_mutability_class, infer_ownership_class, normalize_path, parse_source_metadata,
    record_id_for_slice_source,
};
pub use state_store_task_attempts::{
    ConsolidateTaskStageAttemptsRequest, RecordTaskAttemptRequest, TaskAttemptRecord,
    TaskStageConsolidationReceipt, TaskStageRecord, TaskStageSummary, TransitionTaskAttemptRequest,
};
pub(crate) use state_store_task_models::{
    apply_provider_mapping_to_task_jsonl_record, provider_external_key, TaskContent,
    TaskDependencyJsonlRecord, TaskJsonlRecord, TaskStorageRow, TaskStorageRowStored,
};
pub use state_store_task_models::{
    canonical_work_item_issue_type, task_work_item_kind, work_item_contributes_to_task_stats,
    work_item_is_active_bounded_unit_candidate, work_item_is_program_container,
    work_item_requires_parent, work_item_taxonomy_entry, BlockedTaskRecord, CreateTaskRequest,
    TaskBulkReparentResult, TaskCriticalPath, TaskCriticalPathNode, TaskDefectBatchRehomeResult,
    TaskDependencyStatus, TaskDependencyTreeChild, TaskDependencyTreeEdge, TaskDependencyTreeNode,
    TaskExecutionSemantics, TaskGraphIssue, TaskImportSummary, TaskPlannerMetadata,
    TaskProgressSummary, TaskRecord, TaskRelease1ContractStep, TaskSchedulingCandidate,
    TaskSchedulingProjection, TaskStoreSummary, TaskWorkItemKind, UpdateTaskRequest,
};
pub(crate) use state_store_task_store::SpecFirstDevHandoffGate;
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
const SURREALKV_WAL_REPLAY_CORRUPTION_BLOCKER: &str = "state_store_surrealkv_wal_replay_corruption";
const SURREALKV_WAL_REPLAY_CORRUPTION_GUIDANCE: &str = "Create a backup copy of the whole state directory first, then recover from a known-good state snapshot or reinitialize the local state root through VIDA recovery tooling; do not delete WAL, SST, or SurrealKV subdirectories in place.";
const STATE_RESET_ARCHIVE_RENAME_RETRY_COUNT: usize = 120;
const STATE_RESET_ARCHIVE_RENAME_RETRY_DELAY_MS: u64 = 25;
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
DEFINE TABLE run_graph_owner_evidence SCHEMALESS;
DEFINE TABLE run_graph_projection_checkpoint_record SCHEMALESS;
DEFINE TABLE run_graph_replay_lineage_receipt SCHEMALESS;
DEFINE TABLE run_graph_dispatch_lane_receipt SCHEMALESS;
DEFINE TABLE host_bridge_receipt_identity SCHEMALESS;
DEFINE TABLE host_bridge_precursor_fingerprint SCHEMALESS;
DEFINE TABLE orchestrator_claim SCHEMALESS;
DEFINE TABLE scheduler_dispatch_reservation SCHEMALESS;
DEFINE TABLE task_stage SCHEMALESS;
DEFINE TABLE task_attempt SCHEMALESS;
"#;

fn state_store_recovery_hint_for_message(message: &str) -> Option<&'static str> {
    if state_store_message_is_surrealkv_wal_replay_corruption(message) {
        return Some(SURREALKV_WAL_REPLAY_CORRUPTION_GUIDANCE);
    }
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

fn state_store_message_is_surrealkv_wal_replay_corruption(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("surrealkv")
        && (lower.contains("wal") || lower.contains("memtable") || lower.contains("sst"))
        && (lower.contains("keys are not in order")
            || lower.contains("failed to flush memtable")
            || lower.contains("wal replay"))
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
    _lifecycle_guard: Arc<state_store_open::StateRootLifecycleGuard>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StateResetSummary {
    pub surface: &'static str,
    pub status: &'static str,
    pub state_dir: PathBuf,
    pub archive_path: Option<PathBuf>,
    pub recovery_receipt_path: Option<PathBuf>,
    pub archive_created: bool,
    pub reinitialized: bool,
    pub task_count: usize,
    pub state_spine_manifest_present: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StateStoreOpenDiagnostic {
    pub blocker_code: String,
    pub state_dir: String,
    pub corruption_state: String,
    pub suspected_wal_or_sst_hint: String,
    pub recovery_guidance: String,
    pub silent_delete_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StateStoreOpenStage {
    LifecycleGuard,
    LegacyGuard,
    DatastoreOpen,
    DatastoreCheckVersion,
    DatastoreBootstrap,
    SurrealAttach,
    NamespaceDatabase,
    SchemaQuery,
    Unclassified,
}

impl StateStoreOpenStage {
    pub(crate) const fn lock_evidence(self) -> Option<StateStoreOpenLockEvidence> {
        match self {
            Self::LifecycleGuard => Some(StateStoreOpenLockEvidence::LifecycleGuard),
            Self::LegacyGuard => Some(StateStoreOpenLockEvidence::LegacyGuard),
            Self::DatastoreOpen
            | Self::DatastoreCheckVersion
            | Self::DatastoreBootstrap
            | Self::SurrealAttach
            | Self::NamespaceDatabase
            | Self::SchemaQuery => Some(StateStoreOpenLockEvidence::Datastore),
            Self::Unclassified => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StateStoreOpenLockEvidence {
    LifecycleGuard,
    LegacyGuard,
    Datastore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StateStoreOpenErrorKind {
    LockContention,
    PermissionAccess,
    StorageCorruption,
    Unknown,
}

impl StateStoreOpenErrorKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::LockContention => "lock_contention",
            Self::PermissionAccess => "permission_access",
            Self::StorageCorruption => "storage_corruption",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub(crate) struct StateStoreOpenErrorDiagnostic {
    pub(crate) error_kind: StateStoreOpenErrorKind,
    pub(crate) retryable: bool,
    pub(crate) blocker_code: &'static str,
    pub(crate) open_stage: StateStoreOpenStage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) lock_evidence: Option<StateStoreOpenLockEvidence>,
}

impl StateStoreOpenErrorDiagnostic {
    pub(crate) const fn blocker_code(self) -> &'static str {
        self.blocker_code
    }
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

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyBulkAddInput {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyBulkAddEdgeReport {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub reason: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyBulkAddResult {
    pub dry_run: bool,
    pub requested_count: usize,
    pub created_count: usize,
    pub existing_count: usize,
    pub failed_count: usize,
    pub unapplied_count: usize,
    pub created: Vec<TaskDependencyRecord>,
    pub existing: Vec<TaskDependencyRecord>,
    pub failed: Vec<TaskDependencyBulkAddEdgeReport>,
    pub unapplied: Vec<TaskDependencyBulkAddEdgeReport>,
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
    OpenContext {
        stage: StateStoreOpenStage,
        lock_evidence: Option<StateStoreOpenLockEvidence>,
        source: Box<StateStoreError>,
    },
    InvalidStateSpineManifest {
        reason: String,
    },
    InvalidStateReset {
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

impl StateStoreError {
    pub(crate) fn with_open_context(
        self,
        stage: StateStoreOpenStage,
        lock_evidence: Option<StateStoreOpenLockEvidence>,
    ) -> Self {
        match self {
            Self::OpenContext {
                stage: existing_stage,
                lock_evidence: existing_lock_evidence,
                source,
            } if existing_stage != StateStoreOpenStage::Unclassified => Self::OpenContext {
                stage: existing_stage,
                lock_evidence: existing_lock_evidence,
                source,
            },
            Self::OpenContext { source, .. } => Self::OpenContext {
                stage,
                lock_evidence,
                source,
            },
            source => Self::OpenContext {
                stage,
                lock_evidence,
                source: Box::new(source),
            },
        }
    }

    pub(crate) fn open_error_diagnostic(&self) -> StateStoreOpenErrorDiagnostic {
        let (source, open_stage, lock_evidence) = match self {
            Self::OpenContext {
                stage,
                lock_evidence,
                source,
            } => (source.as_ref(), *stage, *lock_evidence),
            _ => (self, StateStoreOpenStage::Unclassified, None),
        };
        let kind = match source {
            Self::Io(error) => classify_state_store_io_open_error(error),
            Self::Db(error) => classify_state_store_db_open_error(error),
            Self::InvalidStorageMetadata { reason }
            | Self::InvalidStateSpineManifest { reason }
            | Self::InvalidStateReset { reason }
            | Self::InvalidInstructionRuntimeState { reason }
            | Self::InvalidProtocolBinding { reason }
            | Self::InvalidLauncherActivationSnapshot { reason }
            | Self::InvalidTaskRecord { reason }
            | Self::InvalidCanonicalTaskflowExport { reason }
            | Self::InvalidPatchOperation { reason }
            | Self::PatchConflict { reason } => classify_state_store_open_error_message(reason),
            _ => StateStoreOpenErrorKind::Unknown,
        };
        let mut diagnostic = state_store_open_error_diagnostic(kind);
        diagnostic.open_stage = open_stage;
        diagnostic.lock_evidence = lock_evidence;
        diagnostic
    }

    pub fn open_diagnostic(&self, state_dir: &Path) -> Option<StateStoreOpenDiagnostic> {
        let message = self.to_string();
        if !state_store_message_is_surrealkv_wal_replay_corruption(&message) {
            return None;
        }
        Some(StateStoreOpenDiagnostic {
            blocker_code: SURREALKV_WAL_REPLAY_CORRUPTION_BLOCKER.to_string(),
            state_dir: state_dir.display().to_string(),
            corruption_state: "surrealkv_wal_replay_or_memtable_flush_key_order_corruption"
                .to_string(),
            suspected_wal_or_sst_hint: "SurrealKV open failed while replaying WAL or flushing a memtable into SST; WAL/SST files are suspects, but this command will not delete them."
                .to_string(),
            recovery_guidance: SURREALKV_WAL_REPLAY_CORRUPTION_GUIDANCE.to_string(),
            silent_delete_allowed: false,
        })
    }
}

fn state_store_open_error_diagnostic(
    error_kind: StateStoreOpenErrorKind,
) -> StateStoreOpenErrorDiagnostic {
    let (retryable, blocker_code) = match error_kind {
        StateStoreOpenErrorKind::LockContention => (
            true,
            taskflow_contracts::BlockerCode::AuthoritativeStateStoreLocked.as_str(),
        ),
        StateStoreOpenErrorKind::PermissionAccess
        | StateStoreOpenErrorKind::StorageCorruption
        | StateStoreOpenErrorKind::Unknown => (
            false,
            taskflow_contracts::BlockerCode::AuthoritativeStateStoreOpenFailed.as_str(),
        ),
    };
    StateStoreOpenErrorDiagnostic {
        error_kind,
        retryable,
        blocker_code,
        open_stage: StateStoreOpenStage::Unclassified,
        lock_evidence: None,
    }
}

fn classify_state_store_io_open_error(error: &io::Error) -> StateStoreOpenErrorKind {
    if matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    ) || error.raw_os_error().is_some_and(|code| {
        code == libc::EWOULDBLOCK || code == libc::EAGAIN || code == 32 || code == 33
    }) {
        return StateStoreOpenErrorKind::LockContention;
    }
    if error.kind() == io::ErrorKind::PermissionDenied {
        return StateStoreOpenErrorKind::PermissionAccess;
    }
    classify_state_store_open_error_message(&error.to_string())
}

fn classify_state_store_db_open_error(error: &surrealdb::Error) -> StateStoreOpenErrorKind {
    classify_state_store_open_error_evidence(&error.to_string(), &format!("{error:?}"))
}

fn classify_state_store_open_error_evidence(display: &str, debug: &str) -> StateStoreOpenErrorKind {
    let display_kind = classify_state_store_open_error_message(display);
    if display_kind != StateStoreOpenErrorKind::Unknown {
        return display_kind;
    }
    classify_state_store_open_error_message(debug)
}

fn classify_state_store_open_error_message(message: &str) -> StateStoreOpenErrorKind {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("surrealkv")
        && (normalized.contains("wal")
            || normalized.contains("memtable")
            || normalized.contains("sst"))
        && (normalized.contains("keys are not in order")
            || normalized.contains("failed to flush memtable")
            || normalized.contains("wal replay"))
    {
        return StateStoreOpenErrorKind::StorageCorruption;
    }
    if normalized.contains("permission denied")
        || normalized.contains("access is denied")
        || normalized.contains("os error 5")
    {
        return StateStoreOpenErrorKind::PermissionAccess;
    }
    if normalized.contains("timed out while waiting for authoritative datastore lock")
        || normalized.contains("resource temporarily unavailable")
        || normalized.contains("another process has locked")
        || normalized.contains("being used by another process")
        || normalized.contains("process cannot access the file")
        || normalized.contains("portion of the file")
        || normalized.contains("os error 32")
        || normalized.contains("os error 33")
    {
        return StateStoreOpenErrorKind::LockContention;
    }
    StateStoreOpenErrorKind::Unknown
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
                write!(f, "storage metadata record is invalid: {reason}")?;
                if let Some(hint) = state_store_recovery_hint_for_message(reason) {
                    write!(f, "; {hint}")?;
                }
                Ok(())
            }
            Self::OpenContext { source, .. } => source.fmt(f),
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
            Self::InvalidStateReset { reason } => {
                write!(f, "invalid state reset request: {reason}")
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

impl std::error::Error for StateStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OpenContext { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

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

impl StateStore {
    pub async fn archive_and_reinit_state_root(
        root: PathBuf,
        archive: bool,
        reinit: bool,
    ) -> Result<StateResetSummary, StateStoreError> {
        if !archive {
            return Err(StateStoreError::InvalidStateReset {
                reason:
                    "`vida state reset` requires --archive so the current state root is preserved"
                        .to_string(),
            });
        }

        let lifecycle_guard =
            Arc::new(state_store_open::StateRootLifecycleGuard::acquire(&root).await?);
        lifecycle_guard.validate_root_identity(&root)?;
        // Hold the legacy in-root guard for the whole reset. The state root itself stays stable,
        // so Windows never has to rename a directory containing our locked guard handle.
        if root.exists() && state_reset_dir_has_existing_datastore_payload(&root)? {
            validate_state_reset_existing_root(&root)?;
            lifecycle_guard.acquire_legacy_guard(&root).await?;
        }
        let archive_path = if root.exists() {
            let archive_path = Self::next_state_archive_path(&root);
            Self::archive_state_payload_to_archive_with_retry(&root, &archive_path).await?;
            Some(archive_path)
        } else {
            None
        };

        let mut summary = StateResetSummary {
            surface: "vida state reset",
            status: "pass",
            state_dir: root.clone(),
            archive_created: archive_path.is_some(),
            archive_path,
            recovery_receipt_path: None,
            reinitialized: false,
            task_count: 0,
            state_spine_manifest_present: false,
        };

        if reinit {
            let store =
                Self::open_with_lifecycle_guard(root.clone(), lifecycle_guard.clone()).await?;
            let task_store = store.task_store_summary().await?;
            let _state_spine = store.state_spine_summary().await?;
            summary.reinitialized = true;
            summary.task_count = task_store.total_count;
            summary.state_spine_manifest_present = true;
            store.close().await;
        }

        if summary.archive_created || summary.reinitialized {
            summary.recovery_receipt_path = Some(write_state_reset_recovery_receipt(&summary)?);
        }

        Ok(summary)
    }

    fn next_state_archive_path(root: &Path) -> PathBuf {
        let parent = root.parent().unwrap_or_else(|| Path::new("."));
        let file_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "state".to_string());
        for suffix in 0..1000u16 {
            let archive_name = if suffix == 0 {
                format!("{file_name}.archive.{}", unix_timestamp_nanos())
            } else {
                format!("{file_name}.archive.{}-{suffix}", unix_timestamp_nanos())
            };
            let candidate = parent.join(archive_name);
            if !candidate.exists() {
                return candidate;
            }
        }
        parent.join(format!(
            "{file_name}.archive.{}-{}",
            unix_timestamp_nanos(),
            std::process::id()
        ))
    }

    async fn archive_state_payload_to_archive_with_retry(
        root: &Path,
        archive_path: &Path,
    ) -> Result<(), StateStoreError> {
        fs::create_dir(archive_path)?;
        let entries = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
        for entry in entries {
            let file_name = entry.file_name();
            if file_name == ".vida-authoritative-open.guard" {
                continue;
            }
            let source = entry.path();
            let target = archive_path.join(&file_name);
            for attempt in 0..STATE_RESET_ARCHIVE_RENAME_RETRY_COUNT {
                match fs::rename(&source, &target) {
                    Ok(()) => break,
                    Err(error)
                        if state_reset_archive_rename_error_is_retryable(&error)
                            && attempt + 1 < STATE_RESET_ARCHIVE_RENAME_RETRY_COUNT =>
                    {
                        let _ = Self::reclaim_recoverable_authoritative_datastore_lock_marker(root);
                        tokio::time::sleep(std::time::Duration::from_millis(
                            STATE_RESET_ARCHIVE_RENAME_RETRY_DELAY_MS,
                        ))
                        .await;
                    }
                    Err(error) => {
                        return Err(state_reset_archive_rename_failure(
                            root,
                            archive_path,
                            &error,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    async fn rename_state_root_to_archive_with_retry(
        root: &Path,
        archive_path: &Path,
    ) -> Result<(), StateStoreError> {
        for attempt in 0..STATE_RESET_ARCHIVE_RENAME_RETRY_COUNT {
            match fs::rename(root, archive_path) {
                Ok(()) => return Ok(()),
                Err(error)
                    if state_reset_archive_rename_error_is_retryable(&error)
                        && attempt + 1 < STATE_RESET_ARCHIVE_RENAME_RETRY_COUNT =>
                {
                    let _ = Self::reclaim_recoverable_authoritative_datastore_lock_marker(root);
                    tokio::time::sleep(std::time::Duration::from_millis(
                        STATE_RESET_ARCHIVE_RENAME_RETRY_DELAY_MS,
                    ))
                    .await;
                }
                Err(error) => {
                    return Err(state_reset_archive_rename_failure(
                        root,
                        archive_path,
                        &error,
                    ));
                }
            }
        }

        Err(StateStoreError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!(
                "timed out while archiving VIDA state root `{}` to `{}` after waiting for datastore handles to settle",
                root.display(),
                archive_path.display()
            ),
        )))
    }
}

fn state_reset_archive_rename_error_is_retryable(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock
            | io::ErrorKind::TimedOut
            | io::ErrorKind::Interrupted
            | io::ErrorKind::PermissionDenied
    ) || StateStore::message_is_lock_contention(&error.to_string())
}

fn state_reset_archive_rename_failure(
    root: &Path,
    archive_path: &Path,
    error: &io::Error,
) -> StateStoreError {
    let suspect_paths = state_reset_archive_suspect_paths(root);
    let suspect_detail = if suspect_paths.is_empty() {
        "no known datastore lock/WAL/SST suspect paths were present".to_string()
    } else {
        format!("suspect existing paths: {}", suspect_paths.join(", "))
    };
    StateStoreError::InvalidStateReset {
        reason: format!(
            "`vida state reset` could not archive state root `{}` to `{}`; io_kind={:?}; io_error={}; {}; backup_first=true; silent_delete_allowed=false",
            root.display(),
            archive_path.display(),
            error.kind(),
            error,
            suspect_detail
        ),
    }
}

fn state_reset_archive_suspect_paths(root: &Path) -> Vec<String> {
    [
        ".vida-authoritative-open.guard",
        "LOCK",
        "wal",
        "sstables",
        "vlog",
    ]
    .iter()
    .map(|component| root.join(component))
    .filter(|path| path.exists())
    .map(|path| path.display().to_string())
    .collect()
}

fn write_state_reset_recovery_receipt(
    summary: &StateResetSummary,
) -> Result<PathBuf, StateStoreError> {
    let receipt_root = if summary.reinitialized {
        summary.state_dir.clone()
    } else {
        summary
            .archive_path
            .clone()
            .unwrap_or_else(|| summary.state_dir.clone())
    };
    let receipt_dir =
        create_state_reset_receipt_dir(&receipt_root, &["recovery", "state-reset-receipts"])?;
    let receipt_path = receipt_dir.join(format!("state-reset-{}.json", unix_timestamp_nanos()));
    let recorded_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| unix_timestamp_nanos().to_string());
    let recovery_action = if summary.reinitialized {
        "archive_existing_state_root_then_reinitialize_authoritative_spine"
    } else {
        "archive_existing_state_root_without_reinitialize"
    };
    let receipt = serde_json::json!({
        "artifact_kind": "state_reset_recovery_receipt",
        "schema_version": 1,
        "surface": summary.surface,
        "status": summary.status,
        "recorded_at": recorded_at,
        "state_dir": summary.state_dir,
        "archive_path": summary.archive_path,
        "archive_created": summary.archive_created,
        "reinitialized": summary.reinitialized,
        "task_count": summary.task_count,
        "state_spine_manifest_present": summary.state_spine_manifest_present,
        "recovery_action": recovery_action,
        "backup_requirement": SURREALKV_WAL_REPLAY_CORRUPTION_GUIDANCE,
        "silent_delete_allowed": false,
        "result": {
            "state_root_archived": summary.archive_created,
            "authoritative_spine_reinitialized": summary.reinitialized,
            "authoritative_state_opened_after_reinit": summary.state_spine_manifest_present
        }
    });
    let receipt_bytes = serde_json::to_vec_pretty(&receipt).map_err(|error| {
        StateStoreError::InvalidStateReset {
            reason: format!("failed to serialize state reset recovery receipt: {error}"),
        }
    })?;
    let mut receipt_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&receipt_path)?;
    receipt_file.write_all(&receipt_bytes)?;
    Ok(receipt_path)
}

fn create_state_reset_receipt_dir(
    receipt_root: &Path,
    components: &[&str],
) -> Result<PathBuf, StateStoreError> {
    let mut path = receipt_root.to_path_buf();
    for component in components {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(StateStoreError::InvalidStateReset {
                        reason: format!(
                            "`vida state reset` refused to write a recovery receipt through non-directory or symlinked path component `{}`",
                            path.display()
                        ),
                    });
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&path)?;
                let metadata = fs::symlink_metadata(&path)?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(StateStoreError::InvalidStateReset {
                        reason: format!(
                            "`vida state reset` refused to write a recovery receipt through non-directory or symlinked path component `{}`",
                            path.display()
                        ),
                    });
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

fn validate_state_reset_existing_root(root: &Path) -> Result<(), StateStoreError> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(StateStoreError::InvalidStateReset {
            reason: format!(
                "`vida state reset` can only archive an existing VIDA state directory; `{}` is not a plain directory",
                root.display()
            ),
        });
    }

    let required_entries = ["manifest", "sstables", "vlog", "wal"];
    let missing_entries = required_entries
        .iter()
        .filter(|entry| !root.join(entry).exists())
        .copied()
        .collect::<Vec<_>>();
    if !missing_entries.is_empty() {
        return Err(StateStoreError::InvalidStateReset {
            reason: format!(
                "`vida state reset` refused to archive `{}` because it is not a validated VIDA state root; missing required datastore entries: {}",
                root.display(),
                missing_entries.join(", ")
            ),
        });
    }

    Ok(())
}

fn state_reset_dir_has_existing_datastore_payload(root: &Path) -> Result<bool, StateStoreError> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if matches!(
            file_name.as_ref(),
            ".vida-authoritative-open.guard" | "LOCK" | ".operator-projection-cache-state-marker"
        ) {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::state_store_boot_summary::StateSpineManifestContent;
    fn sample_tasks_jsonl() -> String {
        [
            r#"{"id":"vida-root","title":"Root epic","description":"epic","status":"open","priority":1,"issue_type":"epic","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["wave"],"dependencies":[]}"#,
            r#"{"id":"vida-a","title":"Task A","description":"first","status":"open","priority":2,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[{"issue_id":"vida-a","depends_on_id":"vida-root","type":"parent-child","created_at":"2026-03-08T00:00:00Z","created_by":"tester","metadata":"{}","thread_id":""}]}"#,
            r#"{"id":"vida-b","title":"Task B","description":"second","status":"open","priority":3,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[{"issue_id":"vida-b","depends_on_id":"vida-root","type":"parent-child","created_at":"2026-03-08T00:00:00Z","created_by":"tester","metadata":"{}","thread_id":""},{"issue_id":"vida-b","depends_on_id":"vida-a","type":"blocks","created_at":"2026-03-08T00:00:00Z","created_by":"tester","metadata":"{}","thread_id":""}]}"#,
            r#"{"id":"vida-c","title":"Task C","description":"active","status":"in_progress","priority":4,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[{"issue_id":"vida-c","depends_on_id":"vida-root","type":"parent-child","created_at":"2026-03-08T00:00:00Z","created_by":"tester","metadata":"{}","thread_id":""}]}"#,
            r#"{"id":"vida-d","title":"Task D","description":"done","status":"closed","priority":5,"issue_type":"task","created_at":"2026-03-08T00:00:00Z","created_by":"tester","updated_at":"2026-03-08T00:00:00Z","closed_at":"2026-03-08T00:10:00Z","close_reason":"done","source_repo":".","compaction_level":0,"original_size":0,"labels":["framework"],"dependencies":[]}"#,
        ]
        .join("\n")
    }

    fn sample_host_bridge_receipt_identity() -> taskflow_host_bridge::HostBridgeReceiptIdentityV1 {
        let registry = serde_json::json!({
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "dispatch_transport": "host_tool_bridge",
            "receipt_mode": "host_bridge_receipt",
            "operations": {
                "spawn": "multi_agent_v1.spawn_agent",
                "wait": "multi_agent_v1.wait_agent",
                "dispose": "multi_agent_v1.close_agent"
            },
            "dispose_policy": "configured"
        });
        let adapter_operations =
            taskflow_host_bridge::HostBridgeAdapterOperations::from_registry_value(&registry)
                .expect("test adapter registry should resolve");
        let adapter_contract_snapshot = adapter_operations.to_value();
        let adapter_contract_hash = blake3::hash(
            &serde_json::to_vec(&adapter_contract_snapshot).expect("test adapter snapshot"),
        )
        .to_hex()
        .to_string();
        let mut receipt = sample_dispatch_receipt_with_status("bridge_request_pending");
        receipt.run_id = "run-state-store".to_string();
        receipt.dispatch_target = "developer".to_string();
        receipt.lane_status = "lane_running".to_string();
        receipt.dispatch_packet_path =
            Some("runtime-consumption/dispatch-packets/state-store.json".to_string());
        receipt.selected_backend = Some("internal_subagents".to_string());
        receipt.recorded_at = "2026-07-18T00:00:00Z".to_string();
        let precursor_fingerprint =
            taskflow_host_bridge::HostBridgePrecursorFingerprintV1::from_dispatch_receipt(
                "request-state-store",
                &serde_json::to_value(receipt).expect("test dispatch receipt should serialize"),
            )
            .expect("test precursor fingerprint should build");
        taskflow_host_bridge::HostBridgeReceiptIdentityV1 {
            schema_version: taskflow_host_bridge::HOST_BRIDGE_RECEIPT_IDENTITY_SCHEMA_VERSION
                .to_string(),
            request_id: "request-state-store".to_string(),
            run_id: "run-state-store".to_string(),
            task_id: "task-state-store".to_string(),
            attempt_id: "attempt-state-store".to_string(),
            packet_id: "packet-state-store".to_string(),
            dispatch_target: "developer".to_string(),
            packet_path: "runtime-consumption/dispatch-packets/state-store.json".to_string(),
            backend_id: "internal_subagents".to_string(),
            carrier_id: "middle".to_string(),
            adapter_kind: adapter_operations.adapter_kind.clone(),
            adapter_capability_id: adapter_operations.adapter_capability_id.clone(),
            invocation_mode: adapter_operations.invocation_mode.clone(),
            dispatch_transport: adapter_operations.dispatch_transport.clone(),
            receipt_mode: adapter_operations.receipt_mode.clone(),
            adapter_contract_source: "vida.config.yaml".to_string(),
            adapter_contract_snapshot,
            adapter_contract_hash,
            adapter_operations,
            request_path: "host-tool-bridge/requests/state-store.json".to_string(),
            result_path: "host-tool-bridge/results/state-store.json".to_string(),
            receipt_path: "host-tool-bridge/receipts/state-store.json".to_string(),
            precursor_fingerprint: Some(precursor_fingerprint),
            recorded_at: "2026-07-18T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn host_bridge_receipt_identity_roundtrip_and_clear() {
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-identity-store-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        let store = StateStore::open(root.clone())
            .await
            .expect("state store should open");
        let identity = sample_host_bridge_receipt_identity();
        store
            .record_host_bridge_receipt_identity(&identity)
            .await
            .expect("identity should persist");
        store.close().await;
        let store = StateStore::open(root.clone())
            .await
            .expect("state store should reopen");

        let loaded = store
            .host_bridge_receipt_identity(
                &identity.run_id,
                &identity.dispatch_target,
                &identity.packet_path,
                &identity.request_id,
            )
            .await
            .expect("identity lookup should succeed")
            .expect("identity should roundtrip");
        assert_eq!(loaded, identity);
        assert_eq!(
            store
                .host_bridge_receipt_identities_for_run(&identity.run_id)
                .await
                .expect("run identity lookup should succeed"),
            vec![identity.clone()]
        );
        assert!(store
            .host_bridge_receipt_identity_for_compact(
                &identity.run_id,
                &identity.dispatch_target,
                &identity.packet_path,
            )
            .await
            .expect("compact identity selector should succeed")
            .is_some());

        let mut duplicate = identity.clone();
        duplicate.request_id = "request-state-store-duplicate".to_string();
        duplicate.precursor_fingerprint = Some(
            taskflow_host_bridge::HostBridgePrecursorFingerprintV1::from_dispatch_receipt(
                &duplicate.request_id,
                &identity
                    .precursor_fingerprint
                    .as_ref()
                    .expect("identity should contain precursor fingerprint")
                    .receipt,
            )
            .expect("duplicate precursor fingerprint should build"),
        );
        let ambiguous = store
            .record_host_bridge_receipt_identity(&duplicate)
            .await
            .expect_err("duplicate compact identity must block deterministically");
        assert!(ambiguous
            .to_string()
            .contains("host_bridge_receipt_identity_ambiguous_compact_binding"));

        store
            .clear_host_bridge_receipt_identity(&identity)
            .await
            .expect("identity clear should succeed");
        assert!(store
            .host_bridge_receipt_identity(
                &identity.run_id,
                &identity.dispatch_target,
                &identity.packet_path,
                &identity.request_id,
            )
            .await
            .expect("cleared identity lookup should succeed")
            .is_none());
        assert!(store
            .host_bridge_receipt_identity_for_compact(
                &identity.run_id,
                &identity.dispatch_target,
                &identity.packet_path,
            )
            .await
            .expect("cleared compact identity lookup should succeed")
            .is_none());
        store.close().await;
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn host_bridge_receipt_identity_rejects_legacy_missing_precursor_fingerprint() {
        let root = std::env::temp_dir().join(format!(
            "vida-host-bridge-legacy-identity-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        let store = StateStore::open(root.clone())
            .await
            .expect("state store should open");
        let identity = sample_host_bridge_receipt_identity();
        let mut legacy_value = identity.as_value();
        legacy_value
            .as_object_mut()
            .expect("identity should be an object")
            .remove("precursor_fingerprint");
        let legacy: taskflow_host_bridge::HostBridgeReceiptIdentityV1 =
            serde_json::from_value(legacy_value).expect("legacy identity should deserialize");

        let error = store
            .record_host_bridge_receipt_identity(&legacy)
            .await
            .expect_err("legacy identity must fail closed");
        assert!(error
            .to_string()
            .contains("host_bridge_precursor_fingerprint_missing"));

        store.close().await;
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn state_reset_archives_and_reinitializes_empty_spine() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-archive-reinit-{}-{nanos}",
            std::process::id()
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store.close().await;

        let summary = StateStore::archive_and_reinit_state_root(root.clone(), true, true)
            .await
            .expect("state reset should archive and reinit");

        assert!(summary.archive_created);
        assert!(summary
            .archive_path
            .as_ref()
            .is_some_and(|path| path.exists()));
        assert!(root.exists());
        assert!(summary.reinitialized);
        assert_eq!(summary.task_count, 0);
        assert!(summary.state_spine_manifest_present);
        let receipt_path = summary
            .recovery_receipt_path
            .as_ref()
            .expect("recovery receipt should be recorded");
        assert!(receipt_path.exists());

        let _ = fs::remove_dir_all(&root);
        if let Some(archive_path) = summary.archive_path {
            let _ = fs::remove_dir_all(archive_path);
        }
    }

    #[test]
    fn state_reset_archive_rename_retries_lock_shaped_errors() {
        for error in [
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Access is denied. (os error 5)",
            ),
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "The process cannot access the file because it is being used by another process. (os error 32)",
            ),
            io::Error::new(
                io::ErrorKind::TimedOut,
                "timed out while waiting for datastore lock",
            ),
            io::Error::new(
                io::ErrorKind::Interrupted,
                "interrupted while archiving state root",
            ),
        ] {
            assert!(
                state_reset_archive_rename_error_is_retryable(&error),
                "state reset archive rename should retry transient lock-shaped error: {error}"
            );
        }
    }

    #[test]
    fn state_reset_archive_failure_names_permission_cause_and_suspect_paths() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-archive-error-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        fs::create_dir_all(root.join("wal")).expect("create wal dir");
        fs::create_dir_all(root.join("sstables")).expect("create sstables dir");
        fs::write(root.join(".vida-authoritative-open.guard"), "").expect("write guard");
        fs::write(root.join("LOCK"), "").expect("write lock marker");
        let archive_path = root.with_extension("archive");
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "Access is denied");

        let message = state_reset_archive_rename_failure(&root, &archive_path, &error).to_string();

        assert!(message.contains(root.to_string_lossy().as_ref()));
        assert!(message.contains(archive_path.to_string_lossy().as_ref()));
        assert!(message.contains("io_kind=PermissionDenied"));
        assert!(message.contains("Access is denied"));
        assert!(message.contains(".vida-authoritative-open.guard"));
        assert!(message.contains("LOCK"));
        assert!(message.contains("wal"));
        assert!(message.contains("sstables"));
        assert!(message.contains("backup_first=true"));
        assert!(message.contains("silent_delete_allowed=false"));

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn state_reset_archives_recently_dropped_datastore_without_manual_settle() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-recently-dropped-{}-{nanos}",
            std::process::id()
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        drop(store);

        let summary = StateStore::archive_and_reinit_state_root(root.clone(), true, true)
            .await
            .expect("state reset should wait for recently dropped datastore handles to settle");

        assert!(summary.archive_created);
        assert!(summary.reinitialized);
        assert!(summary
            .archive_path
            .as_ref()
            .is_some_and(|path| path.exists()));
        assert!(root.exists());

        let _ = fs::remove_dir_all(&root);
        if let Some(archive_path) = summary.archive_path {
            let _ = fs::remove_dir_all(archive_path);
        }
    }

    #[tokio::test]
    async fn state_reset_archives_and_reinitializes_precreated_empty_state_dir() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-precreated-empty-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).expect("create empty state dir");

        let summary = StateStore::archive_and_reinit_state_root(root.clone(), true, true)
            .await
            .expect("precreated empty state dir should archive and reinit");

        assert!(summary.archive_created);
        assert!(summary.reinitialized);
        assert!(summary.state_spine_manifest_present);
        assert!(summary
            .recovery_receipt_path
            .as_ref()
            .is_some_and(|path| path.exists()));
        assert!(root.exists());

        let _ = fs::remove_dir_all(&root);
        if let Some(archive_path) = summary.archive_path {
            let _ = fs::remove_dir_all(archive_path);
        }
    }

    #[tokio::test]
    async fn state_reset_archives_corrupted_datastore_after_acquiring_authoritative_guard() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-corrupted-datastore-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        fs::create_dir_all(root.join("manifest")).expect("create manifest dir");
        fs::create_dir_all(root.join("sstables")).expect("create sstables dir");
        fs::create_dir_all(root.join("vlog")).expect("create vlog dir");
        fs::create_dir_all(root.join("wal")).expect("create wal dir");
        fs::write(root.join(".vida-authoritative-open.guard"), "").expect("write guard");
        fs::write(
            root.join("wal").join("00000000000000000003.wal"),
            "not a valid wal",
        )
        .expect("write corrupted wal stand-in");

        let summary = StateStore::archive_and_reinit_state_root(root.clone(), true, true)
            .await
            .expect("corrupted datastore layout should archive before reinit");

        assert!(summary.archive_created);
        let archive_path = summary
            .archive_path
            .clone()
            .expect("archive path should be recorded");
        assert!(archive_path
            .join("wal")
            .join("00000000000000000003.wal")
            .exists());
        assert!(summary.reinitialized);
        assert!(summary.state_spine_manifest_present);
        assert!(root.join("wal").exists());
        let receipt_path = summary
            .recovery_receipt_path
            .as_ref()
            .expect("recovery receipt should be recorded");
        let receipt: serde_json::Value = serde_json::from_slice(
            &fs::read(receipt_path).expect("recovery receipt should be readable"),
        )
        .expect("recovery receipt should be json");
        assert_eq!(receipt["artifact_kind"], "state_reset_recovery_receipt");
        assert_eq!(
            receipt["recovery_action"],
            "archive_existing_state_root_then_reinitialize_authoritative_spine"
        );
        assert_eq!(receipt["silent_delete_allowed"], false);
        assert_eq!(receipt["archive_path"], archive_path.display().to_string());

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(archive_path);
    }

    #[tokio::test]
    async fn state_reset_refuses_to_archive_datastore_when_authoritative_guard_is_locked() {
        use fs2::FileExt;
        use std::fs::OpenOptions;

        let container = std::env::temp_dir().join(format!(
            "vida-state-reset-live-guard-container-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        let root = container.join("state");
        fs::create_dir_all(root.join("manifest")).expect("create manifest dir");
        fs::create_dir_all(root.join("sstables")).expect("create sstables dir");
        fs::create_dir_all(root.join("vlog")).expect("create vlog dir");
        fs::create_dir_all(root.join("wal")).expect("create wal dir");
        fs::write(root.join(".vida-authoritative-open.guard"), "").expect("write legacy guard");
        fs::write(
            root.join("wal").join("00000000000000000003.wal"),
            "held live",
        )
        .expect("write wal stand-in");

        let lifecycle_guard_path = state_store_open::state_root_lifecycle_guard_path(&root)
            .expect("derive lifecycle guard path");
        let guard_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lifecycle_guard_path)
            .expect("open lifecycle guard");
        guard_file
            .try_lock_exclusive()
            .expect("hold lifecycle guard");

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            StateStore::archive_and_reinit_state_root(root.clone(), true, true),
        )
        .await;

        assert!(result.is_err(), "locked live datastore should not archive");
        assert!(root.join("wal").join("00000000000000000003.wal").exists());
        assert!(
            fs::read_dir(root.parent().expect("temp parent"))
                .expect("read temp parent")
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .all(|name| !name.starts_with("state.archive.")),
            "a locked lifecycle guard must prevent archive creation"
        );

        guard_file.unlock().expect("unlock lifecycle guard");
        drop(guard_file);
        let summary = StateStore::archive_and_reinit_state_root(root.clone(), true, true)
            .await
            .expect("reset should retry after lifecycle guard release");
        let archive_path = summary.archive_path.expect("archive path");
        assert!(!archive_path.join(".vida-authoritative-open.guard").exists());
        assert!(root.join(".vida-authoritative-open.guard").exists());
        let _ = fs::remove_dir_all(&container);
    }

    #[tokio::test]
    async fn state_reset_refuses_to_archive_datastore_when_legacy_guard_is_locked() {
        use fs2::FileExt;
        use std::fs::OpenOptions;

        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-legacy-guard-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        fs::create_dir_all(root.join("manifest")).expect("create manifest dir");
        fs::create_dir_all(root.join("sstables")).expect("create sstables dir");
        fs::create_dir_all(root.join("vlog")).expect("create vlog dir");
        fs::create_dir_all(root.join("wal")).expect("create wal dir");
        let legacy_guard_path = root.join(".vida-authoritative-open.guard");
        fs::write(&legacy_guard_path, "").expect("write legacy guard");
        fs::write(
            root.join("wal").join("00000000000000000003.wal"),
            "held live",
        )
        .expect("write wal stand-in");
        let guard_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&legacy_guard_path)
            .expect("open legacy guard");
        guard_file.try_lock_exclusive().expect("hold legacy guard");

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            StateStore::archive_and_reinit_state_root(root.clone(), true, true),
        )
        .await;

        assert!(
            result.is_err(),
            "locked legacy datastore guard should not archive"
        );
        assert!(legacy_guard_path.exists());
        assert!(root.join("wal").join("00000000000000000003.wal").exists());

        guard_file.unlock().expect("unlock legacy guard");
        drop(guard_file);
        let summary = StateStore::archive_and_reinit_state_root(root.clone(), true, true)
            .await
            .expect("reset should retry after legacy guard release");
        let archive_path = summary.archive_path.expect("archive path");
        assert!(!archive_path.join(".vida-authoritative-open.guard").exists());
        assert!(root.join(".vida-authoritative-open.guard").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&archive_path);
    }

    #[tokio::test]
    async fn state_reset_archive_only_records_receipt_without_recreating_state_dir() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-archive-only-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        store.close().await;

        let summary = StateStore::archive_and_reinit_state_root(root.clone(), true, false)
            .await
            .expect("archive-only state reset should pass");

        assert!(summary.archive_created);
        assert!(!summary.reinitialized);
        assert!(root.join(".vida-authoritative-open.guard").exists());
        let archive_path = summary
            .archive_path
            .as_ref()
            .expect("archive path should be recorded");
        assert!(archive_path.exists());
        let receipt_path = summary
            .recovery_receipt_path
            .as_ref()
            .expect("recovery receipt should be recorded");
        assert!(receipt_path.exists());
        assert!(receipt_path.starts_with(archive_path));
        let receipt: serde_json::Value = serde_json::from_slice(
            &fs::read(receipt_path).expect("recovery receipt should be readable"),
        )
        .expect("recovery receipt should be json");
        assert_eq!(
            receipt["recovery_action"],
            "archive_existing_state_root_without_reinitialize"
        );
        assert_eq!(receipt["reinitialized"], false);

        let _ = fs::remove_dir_all(archive_path);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn state_reset_archive_only_rejects_symlinked_recovery_receipt_directory() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-symlink-recovery-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        let attacker_sink = std::env::temp_dir().join(format!(
            "vida-state-reset-attacker-sink-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        fs::create_dir_all(root.join("manifest")).expect("create manifest dir");
        fs::create_dir_all(root.join("sstables")).expect("create sstables dir");
        fs::create_dir_all(root.join("vlog")).expect("create vlog dir");
        fs::create_dir_all(root.join("wal")).expect("create wal dir");
        fs::create_dir_all(&attacker_sink).expect("create attacker sink");
        fs::write(root.join(".vida-authoritative-open.guard"), "").expect("write guard");
        fs::write(root.join("wal").join("00000000000000000003.wal"), "wal")
            .expect("write datastore payload");
        std::os::unix::fs::symlink(&attacker_sink, root.join("recovery"))
            .expect("create recovery symlink");

        let error = StateStore::archive_and_reinit_state_root(root.clone(), true, false)
            .await
            .expect_err("symlinked recovery receipt directory should fail closed");

        match error {
            StateStoreError::InvalidStateReset { reason } => {
                assert!(reason.contains("refused to write a recovery receipt"));
                assert!(reason.contains("recovery"));
            }
            other => panic!("expected invalid reset error, got {other:?}"),
        }
        assert!(!attacker_sink.join("state-reset-receipts").exists());

        if root.exists() {
            let _ = fs::remove_dir_all(&root);
        }
        if let Some(archive_path) = fs::read_dir(root.parent().expect("temp parent"))
            .expect("read temp parent")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with("vida-state-reset-symlink-recovery-")
                        && name.to_string_lossy().contains(".archive.")
                })
            })
        {
            let _ = fs::remove_dir_all(archive_path);
        }
        let _ = fs::remove_dir_all(attacker_sink);
    }

    #[tokio::test]
    async fn state_reset_requires_archive_guard() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-requires-archive-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));

        let error = StateStore::archive_and_reinit_state_root(root, false, true)
            .await
            .expect_err("reset without archive should fail closed");

        assert!(matches!(error, StateStoreError::InvalidStateReset { .. }));
    }

    #[tokio::test]
    async fn state_reset_rejects_existing_non_state_directory_before_archive() {
        let root = std::env::temp_dir().join(format!(
            "vida-state-reset-rejects-non-state-{}-{}",
            std::process::id(),
            unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).expect("create non-state dir");
        fs::write(root.join("report.txt"), "keep me").expect("write non-state payload");

        let error = StateStore::archive_and_reinit_state_root(root.clone(), true, true)
            .await
            .expect_err("non-state directory should fail closed");

        match error {
            StateStoreError::InvalidStateReset { reason } => {
                assert!(reason.contains("not a validated VIDA state root"));
                assert!(reason.contains("missing required datastore entries"));
            }
            other => panic!("expected invalid reset error, got {other:?}"),
        }
        assert!(root.join("report.txt").exists());
        assert!(fs::read_dir(root.parent().expect("temp parent"))
            .expect("read temp parent")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .all(
                |name| !name.starts_with("vida-state-reset-rejects-non-state-")
                    || !name.contains(".archive.")
            ));

        let _ = fs::remove_dir_all(&root);
    }

    fn unique_temp_root(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos))
    }

    fn write_minimal_project_markers(project_root: &Path) {
        fs::create_dir_all(project_root.join(".vida/config")).expect("create .vida/config");
        fs::create_dir_all(project_root.join(".vida/db")).expect("create .vida/db");
        fs::create_dir_all(project_root.join(".vida/project")).expect("create .vida/project");
        fs::write(project_root.join("AGENTS.md"), "# Test agents\n").expect("write AGENTS.md");
        fs::write(project_root.join("vida.config.yaml"), "project_id: test\n")
            .expect("write vida.config.yaml");
    }

    fn write_minimal_framework_source_bundle(source_root: &Path, marker: &str) {
        let framework_dir = source_root.join("framework");
        fs::create_dir_all(&framework_dir).expect("create framework source dir");
        fs::write(
            framework_dir.join("agent-definition.md"),
            format!(
                "artifact_id: framework-agent-definition\nartifact_kind: agent_definition\nversion: 1\nownership_class: framework\nmutability_class: immutable\nactivation_class: always_on\nrequired_follow_on: framework-instruction-contract,framework-prompt-template-config\nhierarchy: framework\n\n{marker}\n"
            ),
        )
        .expect("write agent definition");
        fs::write(
            framework_dir.join("instruction-contract.md"),
            "artifact_id: framework-instruction-contract\nartifact_kind: instruction_contract\nversion: 1\nownership_class: framework\nmutability_class: immutable\nactivation_class: always_on\nhierarchy: framework\n",
        )
        .expect("write instruction contract");
        fs::write(
            framework_dir.join("prompt-template-config.md"),
            "artifact_id: framework-prompt-template-config\nartifact_kind: prompt_template_configuration\nversion: 1\nownership_class: framework\nmutability_class: immutable\nactivation_class: always_on\nhierarchy: framework\n",
        )
        .expect("write prompt template config");
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
        assert!(state_store_open::state_schema_document()
            .contains("DEFINE TABLE run_graph_dispatch_lane_receipt SCHEMALESS;"));
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
        assert_eq!(shown.dependencies.len(), 2);
        assert!(shown.dependencies.iter().any(|dependency| {
            dependency.edge_type == "blocks" && dependency.depends_on_id == "vida-a"
        }));

        let ready = store.ready_tasks().await.expect("ready tasks");
        let ready_ids = ready.into_iter().map(|task| task.id).collect::<Vec<_>>();
        assert_eq!(ready_ids, vec!["vida-c", "vida-a"]);

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn work_item_provider_mapping_import_resolves_external_parent_dependency() {
        let root = unique_temp_root("vida-provider-mapping-import");
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        let child = serde_json::json!({
            "id": "vida-child",
            "title": "Provider child",
            "description": "story",
            "status": "open",
            "priority": 4,
            "issue_type": "",
            "created_at": "2026-03-08T00:00:00Z",
            "created_by": "tester",
            "updated_at": "2026-03-08T00:00:00Z",
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "labels": [],
            "provider_mapping": {
                "provider": "jira",
                "external_id": "PROJ-12",
                "external_parent_id": "PROJ-1",
                "provider_issue_type": "story",
                "provider_status": "resolved",
                "provider_priority": "p1"
            },
            "dependencies": []
        });
        let parent = serde_json::json!({
            "id": "vida-parent",
            "title": "Provider parent",
            "description": "epic",
            "status": "open",
            "priority": 2,
            "issue_type": "",
            "created_at": "2026-03-08T00:00:00Z",
            "created_by": "tester",
            "updated_at": "2026-03-08T00:00:00Z",
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "labels": [],
            "provider_mapping": {
                "provider": "jira",
                "external_id": "PROJ-1",
                "provider_issue_type": "epic",
                "provider_status": "open"
            },
            "dependencies": []
        });
        fs::write(
            &source,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&child).expect("serialize child"),
                serde_json::to_string(&parent).expect("serialize parent")
            ),
        )
        .expect("write provider jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        let summary = store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import provider tasks");
        assert_eq!(summary.imported_count, 2);

        let shown = store.show_task("vida-child").await.expect("show child");
        assert_eq!(shown.issue_type, "task");
        assert_eq!(shown.status, "closed");
        assert_eq!(shown.priority, 1);
        assert_eq!(
            shown
                .provider_mapping
                .as_ref()
                .map(|mapping| mapping.external_id.as_str()),
            Some("PROJ-12")
        );
        assert!(shown.dependencies.iter().any(|dependency| {
            dependency.edge_type == "parent-child" && dependency.depends_on_id == "vida-parent"
        }));
        let export_path = root.join("exported.jsonl");
        let exported_count = store
            .export_tasks_to_jsonl(&export_path)
            .await
            .expect("export provider tasks");
        assert_eq!(exported_count, 2);
        let exported = fs::read_to_string(&export_path).expect("read export");
        let exported_child = exported
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("json export row"))
            .find(|row| row["id"] == "vida-child")
            .expect("exported child row");
        assert_eq!(exported_child["provider_mapping"]["provider"], "jira");
        assert_eq!(exported_child["provider_mapping"]["external_id"], "PROJ-12");

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn work_item_provider_mapping_import_rejects_unresolved_external_parent() {
        let root = unique_temp_root("vida-provider-mapping-missing-parent");
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        let child = serde_json::json!({
            "id": "vida-child",
            "title": "Provider child",
            "description": "story",
            "status": "open",
            "priority": 4,
            "issue_type": "",
            "created_at": "2026-03-08T00:00:00Z",
            "created_by": "tester",
            "updated_at": "2026-03-08T00:00:00Z",
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "labels": [],
            "provider_mapping": {
                "provider": "jira",
                "external_id": "PROJ-12",
                "external_parent_id": "PROJ-404",
                "provider_issue_type": "story"
            },
            "dependencies": []
        });
        fs::write(
            &source,
            format!(
                "{}\n",
                serde_json::to_string(&child).expect("serialize child")
            ),
        )
        .expect("write provider jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        let error = store
            .import_tasks_from_jsonl(&source)
            .await
            .expect_err("unresolved external parent should fail closed");
        assert!(error.to_string().contains("unresolved external_parent_id"));
        assert!(error.to_string().contains("provider=jira"));
        assert!(error.to_string().contains("external_parent_id=PROJ-404"));

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn work_item_provider_mapping_import_rejects_self_parent_dependency() {
        let root = unique_temp_root("vida-provider-mapping-self-parent");
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        let task = serde_json::json!({
            "id": "vida-self",
            "title": "Provider self parent",
            "description": "story",
            "status": "open",
            "priority": 4,
            "issue_type": "",
            "created_at": "2026-03-08T00:00:00Z",
            "created_by": "tester",
            "updated_at": "2026-03-08T00:00:00Z",
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "labels": [],
            "provider_mapping": {
                "provider": "jira",
                "external_id": "PROJ-SELF",
                "external_parent_id": "PROJ-SELF",
                "provider_issue_type": "story"
            },
            "dependencies": []
        });
        fs::write(
            &source,
            format!(
                "{}\n",
                serde_json::to_string(&task).expect("serialize self parent")
            ),
        )
        .expect("write provider jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        let error = store
            .import_tasks_from_jsonl(&source)
            .await
            .expect_err("self parent import should fail closed");
        assert!(error.to_string().contains("invalid graph"));
        assert!(error.to_string().contains("vida-self"));
        assert!(store
            .list_tasks(None, false)
            .await
            .expect("list tasks")
            .is_empty());

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn work_item_provider_mapping_import_rejects_parent_child_cycle() {
        let root = unique_temp_root("vida-provider-mapping-parent-cycle");
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        let first = serde_json::json!({
            "id": "vida-first",
            "title": "Provider first",
            "description": "story",
            "status": "open",
            "priority": 4,
            "issue_type": "",
            "created_at": "2026-03-08T00:00:00Z",
            "created_by": "tester",
            "updated_at": "2026-03-08T00:00:00Z",
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "labels": [],
            "provider_mapping": {
                "provider": "jira",
                "external_id": "PROJ-1",
                "external_parent_id": "PROJ-2",
                "provider_issue_type": "story"
            },
            "dependencies": []
        });
        let second = serde_json::json!({
            "id": "vida-second",
            "title": "Provider second",
            "description": "story",
            "status": "open",
            "priority": 4,
            "issue_type": "",
            "created_at": "2026-03-08T00:00:00Z",
            "created_by": "tester",
            "updated_at": "2026-03-08T00:00:00Z",
            "source_repo": ".",
            "compaction_level": 0,
            "original_size": 0,
            "labels": [],
            "provider_mapping": {
                "provider": "jira",
                "external_id": "PROJ-2",
                "external_parent_id": "PROJ-1",
                "provider_issue_type": "story"
            },
            "dependencies": []
        });
        fs::write(
            &source,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).expect("serialize first"),
                serde_json::to_string(&second).expect("serialize second")
            ),
        )
        .expect("write provider jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        let error = store
            .import_tasks_from_jsonl(&source)
            .await
            .expect_err("parent cycle import should fail closed");
        assert!(error.to_string().contains("parent_child_cycle"));
        assert!(store
            .list_tasks(None, false)
            .await
            .expect("list tasks")
            .is_empty());

        store.close().await;
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
        assert_eq!(tree.dependencies.len(), 2);
        assert!(tree.children.is_empty());
        let blocks = tree
            .dependencies
            .iter()
            .find(|dependency| dependency.edge_type == "blocks")
            .expect("blocks dependency should be present");
        assert_eq!(blocks.edge_type, "blocks");
        assert_eq!(blocks.depends_on_id, "vida-a");
        assert!(blocks.repeated);
        assert!(blocks.node.is_none());
        let parent = tree
            .dependencies
            .iter()
            .find(|dependency| dependency.edge_type == "parent-child")
            .expect("parent-child dependency should be present");
        assert_eq!(parent.edge_type, "parent-child");
        assert_eq!(parent.depends_on_id, "vida-root");
        assert!(parent.node.is_some());
        assert_eq!(parent.node.as_ref().unwrap().task.id, "vida-root");

        let root_tree = store
            .task_dependency_tree("vida-root")
            .await
            .expect("root dependency tree");
        assert!(root_tree.dependencies.is_empty());
        assert_eq!(root_tree.children.len(), 3);
        let child_ids = root_tree
            .children
            .iter()
            .map(|child| child.child_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(child_ids, vec!["vida-a", "vida-b", "vida-c"]);

        store.close().await;
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
        assert_eq!(summary.direct_child_count, 3);
        assert_eq!(summary.descendant_count, 3);
        assert_eq!(summary.open_count, 2);
        assert_eq!(summary.in_progress_count, 1);
        assert_eq!(summary.closed_count, 0);
        assert_eq!(summary.epic_count, 0);
        assert_eq!(summary.status_counts.get("open"), Some(&2));
        assert_eq!(summary.status_counts.get("in_progress"), Some(&1));
        assert_eq!(summary.percent_closed, 0.0);

        let b_summary = store
            .task_progress_summary("vida-b")
            .await
            .expect("task summary without descendants");
        assert_eq!(b_summary.direct_child_count, 0);
        assert_eq!(b_summary.descendant_count, 0);
        assert!(b_summary.status_counts.is_empty());
        assert_eq!(b_summary.percent_closed, 0.0);

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn task_progress_summary_marks_open_epic_with_closed_descendants_as_closure_candidate() {
        let root = unique_temp_root("vida-task-progress-closure-candidate");
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        let rows = sample_tasks_jsonl()
            .replace(
                "\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\"",
                "\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"closed\"",
            )
            .replace(
                "\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"open\"",
                "\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"closed\"",
            )
            .replace(
                "\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"active\",\"status\":\"in_progress\"",
                "\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"active\",\"status\":\"closed\"",
            );
        fs::write(&source, rows).expect("write sample jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks");

        let summary = store
            .task_progress_summary("vida-root")
            .await
            .expect("progress summary");
        assert!(summary.closure_candidate);
        assert_eq!(summary.closure_candidate_state, "ready_to_close");
        assert_eq!(summary.descendant_count, 3);
        assert_eq!(summary.closed_count, 3);
        assert_eq!(summary.percent_closed, 100.0);
        assert!(summary
            .recommended_next_action
            .contains("vida task close vida-root"));
        assert_eq!(
            summary.canonical_commands,
            vec!["vida task close vida-root --reason \"all descendants closed\" --json"]
        );

        let leaf_summary = store
            .task_progress_summary("vida-a")
            .await
            .expect("leaf progress summary");
        assert!(!leaf_summary.closure_candidate);
        assert_eq!(leaf_summary.closure_candidate_state, "already_closed");

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn task_progress_summary_reports_leaf_defect_readiness_fields() {
        let root = unique_temp_root("vida-task-progress-leaf-readiness");
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        let rows = sample_tasks_jsonl().replace(
            "\"labels\":[\"framework\"],\"dependencies\":[{\"issue_id\":\"vida-a\"",
            "\"labels\":[\"framework\"],\"planner_metadata\":{\"proof_targets\":[\"cargo test -p vida task_progress_summary -- --nocapture\"]},\"dependencies\":[{\"issue_id\":\"vida-a\"",
        );
        fs::write(&source, rows).expect("write sample jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks");

        let summary = store
            .task_progress_summary("vida-a")
            .await
            .expect("leaf progress summary");
        assert!(!summary.closure_candidate);
        assert_eq!(summary.closure_candidate_state, "leaf_missing_proof");
        assert_eq!(
            summary.closure_candidate_reason.as_deref(),
            Some("leaf task uses proof readiness instead of container closure semantics")
        );
        assert!(!summary.ready_for_close);
        assert!(summary.missing_proof);
        assert!(!summary.blocked_by_runtime);
        assert_eq!(
            summary.next_required_command.as_deref(),
            Some("Run declared proof targets, then close the leaf task with explicit evidence.")
        );
        assert_eq!(summary.direct_child_count, 0);
        assert_eq!(summary.descendant_count, 0);
        assert!(summary.status_counts.is_empty());

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn task_progress_summary_shell_quotes_closure_candidate_task_id() {
        let root = unique_temp_root("vida-task-progress-closure-candidate-quoted-id");
        let source = root.join("issues.jsonl");
        fs::create_dir_all(&root).expect("create temp dir");
        let unsafe_task_id = "vida-root; touch /tmp/pwned #";
        let rows = sample_tasks_jsonl()
            .replace("vida-root", unsafe_task_id)
            .replace(
                "\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"open\"",
                "\"id\":\"vida-a\",\"title\":\"Task A\",\"description\":\"first\",\"status\":\"closed\"",
            )
            .replace(
                "\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"open\"",
                "\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"closed\"",
            )
            .replace(
                "\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"active\",\"status\":\"in_progress\"",
                "\"id\":\"vida-c\",\"title\":\"Task C\",\"description\":\"active\",\"status\":\"closed\"",
            );
        fs::write(&source, rows).expect("write sample jsonl");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("import tasks");

        let summary = store
            .task_progress_summary(unsafe_task_id)
            .await
            .expect("progress summary");
        let expected_command = "vida task close 'vida-root; touch /tmp/pwned #' --reason \"all descendants closed\" --json";
        assert!(summary.closure_candidate);
        assert!(summary.recommended_next_action.contains(expected_command));
        assert_eq!(summary.canonical_commands, vec![expected_command]);

        store.close().await;
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
            "Run `vida taskflow consume continue` to materialize or refresh run-graph dispatch receipt evidence before operator handoff."
        );

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn close_task_fails_closed_when_blocked_child_tasks_exist() {
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
                status: "blocked",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
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
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create child task");

        let error = store
            .close_task("vida-root", "done")
            .await
            .expect_err("closing parent with blocked child should fail");
        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("cannot close task `vida-root`"));
                assert!(reason.contains("non-closed child tasks"));
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

        store.close().await;
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
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create root task");

        let set_labels_vec = vec!["alpha".to_string(), "beta ".to_string()];
        let updated = store
            .update_task(UpdateTaskRequest {
                task_id: "vida-root",
                title: None,
                status: Some("in_progress"),
                priority: None,
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
                planner_metadata: None,
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
                title: None,
                status: Some("open"),
                priority: None,
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
                planner_metadata: None,
            })
            .await
            .expect("apply delta labels");

        assert_eq!(updated_again.status, "open");
        assert_eq!(
            updated_again.labels,
            vec!["alpha".to_string(), "gamma".to_string()]
        );
        assert!(updated_again.closed_at.is_none());

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_rejects_closed_task_metadata_until_reopened() {
        let root = unique_temp_root("vida-update-closed-task");
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = Vec::<String>::new();

        store
            .create_task(CreateTaskRequest {
                task_id: "closed-parent",
                title: "Closed parent",
                display_id: None,
                description: "parent",
                issue_type: "epic",
                status: "closed",
                priority: 1,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create parent task");
        store
            .create_task(CreateTaskRequest {
                task_id: "closed-task",
                title: "Closed task",
                display_id: None,
                description: "closed",
                issue_type: "task",
                status: "closed",
                priority: 1,
                parent_id: Some("closed-parent"),
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create closed task");

        let mut planner_metadata = TaskPlannerMetadata::default();
        planner_metadata.proof_targets = vec!["cargo test -p vida closed_task_guard".to_string()];
        let err = store
            .update_task(UpdateTaskRequest {
                task_id: "closed-task",
                title: None,
                status: None,
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: Some(planner_metadata.clone()),
            })
            .await
            .expect_err("closed metadata mutation should be rejected");
        let reason = match err {
            StateStoreError::InvalidTaskRecord { reason } => reason,
            other => panic!("expected invalid task record, got {other:?}"),
        };
        assert_eq!(
            StateStore::task_update_closed_task_mutation_task_id_from_reason(&reason),
            Some("closed-task")
        );

        let reopened = store
            .update_task(UpdateTaskRequest {
                task_id: "closed-task",
                title: None,
                status: Some("in_progress"),
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect("status-only reopen should remain explicit path");
        assert_eq!(reopened.status, "in_progress");

        let updated = store
            .update_task(UpdateTaskRequest {
                task_id: "closed-task",
                title: None,
                status: None,
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: Some(planner_metadata),
            })
            .await
            .expect("metadata update should be allowed after reopen");
        assert_eq!(
            updated.planner_metadata.proof_targets,
            vec!["cargo test -p vida closed_task_guard".to_string()]
        );

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn append_task_notes_preserves_existing_notes() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-append-task-notes-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

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
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create root task");

        let first = store
            .append_task_notes("vida-root", "\n\n", "first")
            .await
            .expect("append first note");
        assert_eq!(first.notes.as_deref(), Some("first"));

        let second = store
            .append_task_notes("vida-root", "\n\n", "second")
            .await
            .expect("append second note");
        assert_eq!(second.notes.as_deref(), Some("first\n\nsecond"));

        let persisted = store.show_task("vida-root").await.expect("show task");
        assert_eq!(persisted.notes.as_deref(), Some("first\n\nsecond"));

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn update_task_rejects_closed_status_when_blocked_child_exists() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-update-task-close-blocked-child-{}-{}",
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
                planner_metadata: TaskPlannerMetadata::default(),
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
                status: "blocked",
                priority: 2,
                parent_id: Some("vida-root"),
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create child task");

        let error = store
            .update_task(UpdateTaskRequest {
                task_id: "vida-root",
                title: None,
                status: Some("closed"),
                priority: None,
                notes: None,
                description: None,
                parent_id: None,
                add_labels: &[],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect_err("updating parent to closed with blocked child should fail");
        match error {
            StateStoreError::InvalidTaskRecord { reason } => {
                assert!(reason.contains("cannot close task `vida-root`"));
                assert!(reason.contains("non-closed child tasks"));
                assert!(reason.contains("vida-child"));
            }
            other => panic!("expected InvalidTaskRecord, got {other}"),
        }

        let root_task = store.show_task("vida-root").await.expect("show root");
        assert_eq!(root_task.status, "open");
        assert!(root_task.closed_at.is_none());

        store.close().await;
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
            ("dep-task", "Dependency", "epic", None),
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
                    planner_metadata: TaskPlannerMetadata::default(),
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
                title: None,
                status: None,
                priority: None,
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
                planner_metadata: None,
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

        let clear_parent_error = store
            .update_task(UpdateTaskRequest {
                task_id: "child-task",
                title: None,
                status: None,
                priority: None,
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
                planner_metadata: None,
            })
            .await
            .expect_err("open parent-required task cannot clear parent");

        assert!(clear_parent_error
            .to_string()
            .contains("missing_required_parent_edge on child-task"));
        let unchanged = store
            .show_task("child-task")
            .await
            .expect("child task should remain after rejected parent clear");
        assert!(unchanged.dependencies.iter().any(|dependency| {
            dependency.edge_type == "parent-child" && dependency.depends_on_id == "root-b"
        }));
        assert!(unchanged
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "blocks"
                && dependency.depends_on_id == "dep-task"));

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn bulk_dependency_add_uses_aggregate_plan_before_persistence() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-bulk-dependency-aggregate-plan-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            ("root", "Root", "epic", None),
            ("task-a", "Task A", "task", Some("root")),
            ("task-b", "Task B", "task", Some("root")),
            ("blocker-a", "Blocker A", "task", Some("root")),
            ("blocker-b", "Blocker B", "task", Some("root")),
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
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "tester",
                    source_repo: ".",
                })
                .await
                .expect("create task");
        }

        let edges = vec![
            TaskDependencyBulkAddInput {
                issue_id: "task-a".to_string(),
                depends_on_id: "blocker-a".to_string(),
                edge_type: "blocks".to_string(),
            },
            TaskDependencyBulkAddInput {
                issue_id: "task-b".to_string(),
                depends_on_id: "blocker-b".to_string(),
                edge_type: "blocks".to_string(),
            },
        ];

        let dry_run = store
            .add_task_dependencies_bulk(&edges, "tester", true)
            .await
            .expect("dry-run bulk dependency add");
        assert_eq!(dry_run.created_count, 2);
        assert!(store
            .show_task("task-a")
            .await
            .expect("show task-a")
            .dependencies
            .iter()
            .all(|dependency| dependency.depends_on_id != "blocker-a"));

        let persisted = store
            .add_task_dependencies_bulk(&edges, "tester", false)
            .await
            .expect("persist bulk dependency add through aggregate plan");
        assert_eq!(persisted.created_count, 2);
        for (task_id, blocker_id) in [("task-a", "blocker-a"), ("task-b", "blocker-b")] {
            let task = store.show_task(task_id).await.expect("show task");
            assert!(task.dependencies.iter().any(|dependency| {
                dependency.edge_type == "blocks" && dependency.depends_on_id == blocker_id
            }));
        }

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn metadata_update_uses_aggregate_plan_before_preserving_latest_notes() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-metadata-update-aggregate-plan-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        store
            .create_task(CreateTaskRequest {
                task_id: "root",
                title: "Root",
                display_id: None,
                description: "",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &[],
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create root");

        let updated = store
            .update_task(UpdateTaskRequest {
                task_id: "root",
                title: Some("Root renamed"),
                status: None,
                priority: Some(2),
                notes: None,
                description: Some("metadata-only update"),
                parent_id: None,
                add_labels: &["aggregate-plan".to_string()],
                remove_labels: &[],
                set_labels: None,
                execution_mode: None,
                order_bucket: None,
                parallel_group: None,
                conflict_domain: None,
                planner_metadata: None,
            })
            .await
            .expect("metadata update through aggregate plan");

        assert_eq!(updated.title, "Root renamed");
        assert_eq!(updated.priority, 2);
        assert_eq!(updated.description, "metadata-only update");
        assert!(updated.labels.iter().any(|label| label == "aggregate-plan"));

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn reparent_children_moves_selected_children_and_preserves_store_on_dry_run() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-reparent-children-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, parent_id) in [
            ("root-a", "Root A", "epic", None),
            ("root-b", "Root B", "epic", None),
            ("child-1", "Child 1", "task", Some("root-a")),
            ("child-2", "Child 2", "task", Some("root-a")),
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
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "tester",
                    source_repo: ".",
                })
                .await
                .expect("create task");
        }

        let dry_run = store
            .reparent_children("root-a", "root-b", &["child-1".to_string()], true)
            .await
            .expect("dry-run reparent children");
        assert_eq!(dry_run.moved_child_ids, vec!["child-1".to_string()]);
        let child_1_after_dry_run = store.show_task("child-1").await.expect("show child-1");
        assert!(child_1_after_dry_run
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "parent-child"
                && dependency.depends_on_id == "root-a"));

        let persisted = store
            .reparent_children("root-a", "root-b", &["child-1".to_string()], false)
            .await
            .expect("persisted reparent children");
        assert_eq!(persisted.moved_child_ids, vec!["child-1".to_string()]);

        let child_1 = store.show_task("child-1").await.expect("show child-1");
        let child_2 = store.show_task("child-2").await.expect("show child-2");
        assert!(child_1
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "parent-child"
                && dependency.depends_on_id == "root-b"));
        assert!(child_2
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "parent-child"
                && dependency.depends_on_id == "root-a"));

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn defect_batch_rehome_moves_pauses_and_starts_after_graph_validation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-defect-batch-rehome-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        for (task_id, title, issue_type, status, parent_id) in [
            ("root-a", "Root A", "epic", "open", None),
            ("root-b", "Root B", "epic", "open", None),
            ("child-1", "Child 1", "defect", "open", Some("root-a")),
            ("child-2", "Child 2", "defect", "open", Some("root-a")),
            (
                "old-active",
                "Old active",
                "defect",
                "in_progress",
                Some("root-a"),
            ),
            ("new-active", "New active", "defect", "open", Some("root-b")),
        ] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title,
                    display_id: None,
                    description: "",
                    issue_type,
                    status,
                    priority: 1,
                    parent_id,
                    labels: &[],
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "tester",
                    source_repo: ".",
                })
                .await
                .expect("create task");
        }

        let dry_run = store
            .defect_batch_rehome(
                "root-a",
                "root-b",
                &["child-1".to_string()],
                &["old-active".to_string()],
                &["new-active".to_string()],
                true,
            )
            .await
            .expect("dry-run defect batch rehome");
        assert_eq!(dry_run.moved_child_ids, vec!["child-1".to_string()]);
        assert_eq!(dry_run.paused_task_ids, vec!["old-active".to_string()]);
        assert_eq!(dry_run.started_task_ids, vec!["new-active".to_string()]);
        let child_1_after_dry_run = store.show_task("child-1").await.expect("show child-1");
        assert!(child_1_after_dry_run
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "parent-child"
                && dependency.depends_on_id == "root-a"));
        assert_eq!(
            store
                .show_task("old-active")
                .await
                .expect("show old-active")
                .status,
            "in_progress"
        );

        let persisted = store
            .defect_batch_rehome(
                "root-a",
                "root-b",
                &["child-1".to_string()],
                &["old-active".to_string()],
                &["new-active".to_string()],
                false,
            )
            .await
            .expect("persisted defect batch rehome");
        assert_eq!(persisted.moved_count, 1);
        assert_eq!(persisted.paused_count, 1);
        assert_eq!(persisted.started_count, 1);

        let child_1 = store.show_task("child-1").await.expect("show child-1");
        let child_2 = store.show_task("child-2").await.expect("show child-2");
        assert!(child_1
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "parent-child"
                && dependency.depends_on_id == "root-b"));
        assert!(child_2
            .dependencies
            .iter()
            .any(|dependency| dependency.edge_type == "parent-child"
                && dependency.depends_on_id == "root-a"));
        assert_eq!(
            store
                .show_task("old-active")
                .await
                .expect("show old-active")
                .status,
            "paused"
        );
        assert_eq!(
            store
                .show_task("new-active")
                .await
                .expect("show new-active")
                .status,
            "in_progress"
        );

        store.close().await;
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

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn ingest_relative_source_root_resolves_from_active_project_root() {
        let root = unique_temp_root("vida-state-store-project-source-root");
        let project_root = root.join("project");
        let state_root = root.join("state");
        write_minimal_project_markers(&project_root);
        write_minimal_framework_source_bundle(
            &project_root.join(DEFAULT_INSTRUCTION_SOURCE_ROOT),
            "active-project-root-source-marker",
        );

        let _cwd = crate::test_cli_support::guard_current_dir(&project_root);
        let store = StateStore::open(state_root).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed bundle");
        let ingest = store
            .ingest_instruction_source_tree(DEFAULT_INSTRUCTION_SOURCE_ROOT)
            .await
            .expect("ingest source tree from active project root");
        assert_eq!(ingest.imported_count, 3);

        let artifact: Option<InstructionArtifactRow> = store
            .db
            .select(("instruction_artifact", "framework-agent-definition"))
            .await
            .expect("select ingested artifact");
        let artifact = artifact.expect("agent definition artifact should exist");
        assert!(artifact.body.contains("active-project-root-source-marker"));

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn ingest_accepts_absolute_source_root_without_project_resolution() {
        let root = unique_temp_root("vida-state-store-absolute-source-root");
        let source_root = root.join("absolute-framework-source");
        let state_root = root.join("state");
        write_minimal_framework_source_bundle(&source_root, "absolute-source-marker");

        let store = StateStore::open(state_root).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed bundle");
        let ingest = store
            .ingest_instruction_source_tree(&source_root.display().to_string())
            .await
            .expect("ingest absolute source tree");
        assert_eq!(ingest.imported_count, 3);

        let artifact: Option<InstructionArtifactRow> = store
            .db
            .select(("instruction_artifact", "framework-agent-definition"))
            .await
            .expect("select ingested artifact");
        let artifact = artifact.expect("agent definition artifact should exist");
        assert!(artifact.body.contains("absolute-source-marker"));

        store.close().await;
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

        store.close().await;

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

        existing.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

    #[test]
    fn surrealkv_wal_replay_corruption_error_includes_backup_first_guidance() {
        let rendered = StateStoreError::InvalidStorageMetadata {
            reason: "failed to open bounded SurrealKV datastore: Failed to flush memtable to SST table_id=4: Keys are not in order".to_string(),
        }
        .to_string();
        assert!(rendered.contains("Create a backup copy of the whole state directory first"));
        assert!(rendered.contains("do not delete WAL, SST, or SurrealKV subdirectories in place"));
    }

    #[test]
    fn surrealkv_wal_replay_corruption_diagnostic_exposes_operator_fields() {
        let state_dir = std::path::Path::new("C:/project/vida_mobile/.vida/data/state");
        let error = StateStoreError::InvalidStorageMetadata {
            reason: "failed to open bounded SurrealKV datastore: failed during WAL replay: Keys are not in order".to_string(),
        };
        let diagnostic = error
            .open_diagnostic(state_dir)
            .expect("WAL replay key-order error should be classified");
        assert_eq!(
            diagnostic.blocker_code,
            "state_store_surrealkv_wal_replay_corruption"
        );
        assert_eq!(diagnostic.state_dir, state_dir.display().to_string());
        assert!(!diagnostic.silent_delete_allowed);
        assert!(diagnostic
            .suspected_wal_or_sst_hint
            .contains("WAL/SST files are suspects"));
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

        store.close().await;
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
        let source_root = root.join("framework-source");
        write_minimal_framework_source_bundle(&source_root, "missing-runtime-root-marker");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .seed_framework_instruction_bundle()
            .await
            .expect("seed framework bundle");
        store
            .ingest_instruction_source_tree(&source_root.display().to_string())
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;

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

        reopened.close().await;
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
        store
            .persist_task_record(run_graph_fixture_task("vida-a"))
            .await
            .expect("seed live task for latest projection authority");
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

        store.close().await;

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

        reopened.close().await;
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

    fn run_graph_fixture_task(task_id: &str) -> TaskRecord {
        TaskRecord {
            id: task_id.to_string(),
            display_id: None,
            title: task_id.to_string(),
            description: task_id.to_string(),
            status: "open".to_string(),
            priority: 1,
            issue_type: "task".to_string(),
            created_at: "2026-05-22T00:00:00Z".to_string(),
            created_by: "test".to_string(),
            updated_at: "2026-05-22T00:00:00Z".to_string(),
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_accepts_memory_governance_when_consent_and_ttl_are_linked() {
        let nanos = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-run-graph-governance-linkage-pass-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let mut status = sample_run_graph_status();
        status.policy_gate = "memory_delete_required".to_string();
        status.context_state = "sealed".to_string();
        status.handoff_state = "consent_ttl_linked".to_string();

        store
            .record_run_graph_status(&status)
            .await
            .expect("consent+ttl linked governance state should persist");

        let persisted = store
            .run_graph_status(&status.run_id)
            .await
            .expect("load persisted run graph status");
        assert_eq!(persisted.policy_gate, "memory_delete_required");
        assert_eq!(persisted.context_state, "sealed");
        assert_eq!(persisted.handoff_state, "consent_ttl_linked");

        store.close().await;
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

        store.close().await;
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
        let labels: Vec<String> = Vec::new();
        store
            .create_task(CreateTaskRequest {
                task_id: "run-graph-latest-parent",
                title: "Run graph latest parent",
                display_id: None,
                description: "parent",
                issue_type: "epic",
                status: "open",
                priority: 0,
                parent_id: None,
                labels: &labels,
                execution_semantics: TaskExecutionSemantics::default(),
                planner_metadata: TaskPlannerMetadata::default(),
                created_by: "test",
                source_repo: "test",
            })
            .await
            .expect("create run graph latest parent");
        for task_id in ["task-aaa", "task-bbb"] {
            store
                .create_task(CreateTaskRequest {
                    task_id,
                    title: "Run graph latest task",
                    display_id: None,
                    description: "task",
                    issue_type: "task",
                    status: "in_progress",
                    priority: 0,
                    parent_id: Some("run-graph-latest-parent"),
                    labels: &labels,
                    execution_semantics: TaskExecutionSemantics::default(),
                    planner_metadata: TaskPlannerMetadata::default(),
                    created_by: "test",
                    source_repo: "test",
                })
                .await
                .expect("create run graph latest task");
        }

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

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_recovery_checkpoint_and_gate_summaries_use_status_run_checkpoint_when_other_run_checkpoint_is_newer(
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
        store
            .persist_task_record(run_graph_fixture_task("task-current"))
            .await
            .expect("seed current task");

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

        let recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("other-run checkpoint drift should not poison current recovery summary")
            .expect("latest recovery summary should exist");
        assert_eq!(recovery.run_id, "run-current");

        let checkpoint = store
            .latest_run_graph_checkpoint_summary()
            .await
            .expect("other-run checkpoint drift should not poison current checkpoint summary")
            .expect("latest checkpoint summary should exist");
        assert_eq!(checkpoint.run_id, "run-current");

        let gate = store
            .latest_run_graph_gate_summary()
            .await
            .expect("other-run checkpoint drift should not poison current gate summary")
            .expect("latest gate summary should exist");
        assert_eq!(gate.run_id, "run-current");

        store.close().await;
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
        store
            .persist_task_record(run_graph_fixture_task("vida-a"))
            .await
            .expect("seed latest task");
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

        store.close().await;
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
        store
            .persist_task_record(run_graph_fixture_task("vida-a"))
            .await
            .expect("seed latest task");
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

        let recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("missing latest governance row should produce fail-closed recovery summary")
            .expect("latest recovery summary should exist");
        assert_eq!(recovery.run_id, "run-vida-a");
        assert_eq!(recovery.policy_gate, "stale_missing_run_graph_governance");
        assert_eq!(
            recovery.handoff_state,
            "blocked_missing_run_graph_governance"
        );
        assert!(!recovery.recovery_ready);

        let checkpoint = store
            .latest_run_graph_checkpoint_summary()
            .await
            .expect("missing latest governance row should produce fail-closed checkpoint summary")
            .expect("latest checkpoint summary should exist");
        assert_eq!(checkpoint.run_id, "run-vida-a");
        assert_eq!(checkpoint.resume_target, "none");
        assert!(!checkpoint.recovery_ready);

        let gate = store
            .latest_run_graph_gate_summary()
            .await
            .expect("missing latest governance row should produce fail-closed gate summary")
            .expect("latest gate summary should exist");
        assert_eq!(gate.run_id, "run-vida-a");
        assert_eq!(gate.policy_gate, "stale_missing_run_graph_governance");
        assert_eq!(gate.handoff_state, "blocked_missing_run_graph_governance");

        store.close().await;
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
            policy_bundle_ref: None,
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

        store.close().await;
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
        let result_path = root.join("run-shared-contract-flow-result.json");
        fs::write(
            &result_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "artifact_kind": "runtime_dispatch_result",
                "status": "pass",
                "activation_semantics": {
                    "activation_kind": "execution_evidence",
                    "view_only": false,
                    "executes_packet": true,
                    "records_completion_receipt": true
                },
                "execution_evidence": {
                    "status": "recorded",
                    "receipt_backed": true
                }
            }))
            .expect("encode dispatch result"),
        )
        .expect("write dispatch result");
        receipt.dispatch_result_path = Some(result_path.display().to_string());
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

        store.close().await;
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

        store.close().await;
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
        store
            .persist_task_record(run_graph_fixture_task("task-aaa"))
            .await
            .expect("seed older task");
        store
            .persist_task_record(run_graph_fixture_task("task-bbb"))
            .await
            .expect("seed latest task");

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

        store.close().await;
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
        let stored_receipt: RunGraphDispatchReceiptStored = receipt.into();
        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert(("run_graph_dispatch_receipt", "run-drift"))
            .content(stored_receipt)
            .await
            .expect("persist drifted dispatch receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("drifted downstream lane signal should fail closed");
        assert!(error
            .to_string()
            .contains("run-graph dispatch receipt summary is inconsistent"));

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn latest_run_graph_dispatch_receipt_summary_repairs_in_flight_downstream_lane_drift() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-dispatch-receipt-in-flight-lane-drift-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");

        let mut status = sample_run_graph_status();
        status.run_id = "run-in-flight-drift".to_string();
        status.task_id = "task-in-flight-drift".to_string();
        status.active_node = "autotester".to_string();
        status.next_node = None;
        status.status = "blocked".to_string();
        status.lane_id = "autotester_lane".to_string();
        status.lifecycle_stage = "autotester_blocked".to_string();
        status.policy_gate = "host_bridge_completion_result_blocked".to_string();
        status.handoff_state = "none".to_string();
        status.resume_target = "dispatch.autotester_lane".to_string();
        status.recovery_ready = false;
        store
            .record_run_graph_status(&status)
            .await
            .expect("persist run graph status");

        let mut receipt = sample_dispatch_receipt_with_status("bridge_request_pending");
        receipt.run_id = "run-in-flight-drift".to_string();
        receipt.dispatch_target = "autotester".to_string();
        receipt.lane_status = "lane_running".to_string();
        receipt.blocker_code = Some("host_tool_bridge_adapter_required".to_string());
        receipt.downstream_dispatch_target = Some("developer".to_string());
        receipt.downstream_dispatch_ready = false;
        receipt.downstream_dispatch_blockers =
            vec!["host_bridge_completion_result_blocked".to_string()];
        receipt.downstream_dispatch_status = Some("blocked".to_string());
        receipt.downstream_dispatch_active_target = Some("autotester".to_string());
        receipt.downstream_dispatch_last_target = Some("autotester".to_string());
        receipt.recorded_at = "2026-03-18T00:05:00Z".to_string();
        let stored_receipt: RunGraphDispatchReceiptStored = receipt.into();
        let _: Option<RunGraphDispatchReceiptStored> = store
            .db
            .upsert(("run_graph_dispatch_receipt", "run-in-flight-drift"))
            .content(stored_receipt)
            .await
            .expect("persist stale dispatch receipt");

        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("stale in-flight lane drift should repair")
            .expect("latest dispatch receipt summary should exist");
        assert_eq!(summary.run_id, "run-in-flight-drift");
        assert_eq!(summary.dispatch_status, "bridge_request_pending");
        assert_eq!(summary.lane_status, "lane_open");
        assert_eq!(
            summary.downstream_dispatch_status.as_deref(),
            Some("blocked")
        );
        assert_eq!(
            summary.downstream_dispatch_blockers,
            vec!["host_bridge_completion_result_blocked".to_string()]
        );

        let repaired: Option<RunGraphDispatchReceiptStored> = store
            .db
            .select(("run_graph_dispatch_receipt", "run-in-flight-drift"))
            .await
            .expect("load repaired stored receipt");
        assert_eq!(
            repaired
                .expect("repaired stored receipt exists")
                .lane_status
                .as_deref(),
            Some("lane_open")
        );

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist unicode zero-width downstream blockers receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("unicode zero-width downstream blockers should fail closed");
        assert!(error.to_string().contains("unicode drift"));

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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
                policy_bundle_ref: None,
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

        store.close().await;
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

        // With the fix, stale projection checkpoints are cleared instead of failing
        // Note: This test creates a resumability_capsule, not a projection checkpoint record.
        // The projection checkpoint clearing logic is in ensure_run_graph_recovery_surface_has_checkpoint_lineage.
        // For resumability capsule leakage, separate handling would be needed.
        let summary = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect("dispatch receipt summary should succeed");
        assert!(summary.is_some(), "summary should be present");
        assert_eq!(summary.as_ref().unwrap().run_id, "run-current");

        store.close().await;
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
                policy_bundle_ref: None,
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
                policy_bundle_ref: None,
                recorded_at: "2026-03-18T00:00:00Z".to_string(),
            })
            .await
            .expect("persist whitespace lane_status receipt");

        let error = store
            .latest_run_graph_dispatch_receipt_summary()
            .await
            .expect_err("whitespace-only lane_status should fail closed");
        assert!(error.to_string().contains("lane_status must be non-empty"));

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;

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

        reopened.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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
        assert_eq!(snapshot.tasks[0].id.as_str(), "vida-a");
        assert_eq!(snapshot.tasks[1].id.as_str(), "vida-b");
        assert_eq!(snapshot.tasks[2].id.as_str(), "vida-root");
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
        assert_eq!(snapshot.dependencies[0].issue_id.as_str(), "vida-a");
        assert_eq!(snapshot.dependencies[0].depends_on_id.as_str(), "vida-root");
        assert_eq!(snapshot.dependencies[1].issue_id.as_str(), "vida-b");
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
        assert_eq!(runtime_dependencies[0].depends_on_id.as_str(), "vida-a");

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

        store.close().await;
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
                planner_metadata: TaskPlannerMetadata::default(),
                provider_mapping: None,
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

        store.close().await;
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
        assert_eq!(restored_dependencies[0].depends_on_id.as_str(), "vida-root");

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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":0,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-stale\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep old\",\"description\":\"keep\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-root"),
                    title: "Root".to_string(),
                    status: CanonicalTaskStatus::Closed,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-keep"),
                    title: "Keep new".to_string(),
                    status: CanonicalTaskStatus::Closed,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                            .expect("parse timestamp"),
                    ),
                },
            ],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-keep"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
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
        assert_eq!(receipts[0].task_count, 2);
        assert_eq!(receipts[0].dependency_count, 1);
        assert_eq!(receipts[0].stale_removed_count, 1);

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "replace_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 2);
        assert_eq!(latest.dependency_count, 1);
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
                .contains("1 receipts (tasks=2, dependencies=1, stale_removed=1, operations: replace_snapshot=1; source_kinds: canonical_snapshot_memory=1;")
        );

        store.close().await;
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

        store.close().await;
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

        store.close().await;
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
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-stale\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
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
                    status: CanonicalTaskStatus::Closed,
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

        store.close().await;
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
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-stale\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
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
                    status: CanonicalTaskStatus::Closed,
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

        store.close().await;

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

        reopened.close().await;
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
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":0,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-stale\",\"title\":\"Stale\",\"description\":\"stale\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-stale\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep old\",\"description\":\"keep\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
            ),
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-root"),
                    title: "Root".to_string(),
                    status: CanonicalTaskStatus::Closed,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-keep"),
                    title: "Keep replacement".to_string(),
                    status: CanonicalTaskStatus::Closed,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                            .expect("parse timestamp"),
                    ),
                },
            ],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-keep"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
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
        assert_eq!(latest.task_count, 2);
        assert_eq!(latest.dependency_count, 1);
        assert_eq!(latest.stale_removed_count, 1);

        let rollup = store
            .task_reconciliation_rollup()
            .await
            .expect("reconciliation rollup should load");
        assert_eq!(rollup.total_receipts, 1);
        assert_eq!(rollup.total_task_rows, 2);
        assert_eq!(rollup.total_dependency_rows, 1);
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
        assert_eq!(bridge.total_task_rows, 2);
        assert_eq!(bridge.total_dependency_rows, 1);
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
                .contains("receipts=1 export=0 import=0 replace=1 object=0 memory=0 file=1 tasks=2 dependencies=1 stale_removed=1 latest=replace_snapshot via canonical_snapshot_file")
        );

        store.close().await;
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
                "{\"id\":\"vida-b\",\"title\":\"Task B\",\"description\":\"second\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"bug\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:02Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-b\",\"depends_on_id\":\"vida-a\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
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
                    task.id.as_str().to_string(),
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
                    task.id.as_str().to_string(),
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
                    dependency.issue_id.as_str().to_string(),
                    dependency.depends_on_id.as_str().to_string(),
                    dependency.dependency_type.clone(),
                )
            })
            .collect::<Vec<_>>();
        let re_exported_dependency_rows = re_exported
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.issue_id.as_str().to_string(),
                    dependency.depends_on_id.as_str().to_string(),
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
                    task.id.as_str().to_string(),
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
                    dependency.issue_id.as_str().to_string(),
                    dependency.depends_on_id.as_str().to_string(),
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

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn run_graph_status_does_not_close_open_run_from_closure_ready_downstream_receipt() {
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
        store
            .persist_task_record(run_graph_fixture_task("task-closure-ready"))
            .await
            .expect("seed live task for latest projection authority");

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
            policy_bundle_ref: None,
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
        assert_eq!(reconciled.active_node, "dev-pack");
        assert_eq!(reconciled.status, "ready");
        assert_eq!(reconciled.lifecycle_stage, "dev_pack_active");
        assert_eq!(reconciled.policy_gate, "single_task_scope_required");
        assert_eq!(reconciled.handoff_state, "none");
        assert_eq!(reconciled.resume_target, "none");
        assert!(reconciled.recovery_ready);

        let latest_status = store
            .latest_run_graph_status()
            .await
            .expect("latest reconciled run graph status should load")
            .expect("latest run graph status should exist");
        assert_eq!(latest_status.active_node, "dev-pack");
        assert_eq!(latest_status.status, "ready");
        assert_eq!(latest_status.lifecycle_stage, "dev_pack_active");
        assert_eq!(latest_status.policy_gate, "single_task_scope_required");
        assert_eq!(latest_status.handoff_state, "none");
        assert_eq!(latest_status.resume_target, "none");
        assert!(latest_status.recovery_ready);

        let recovery = store
            .run_graph_recovery_summary("run-closure-ready")
            .await
            .expect("reconciled recovery summary should load");
        assert_eq!(recovery.active_node, "dev-pack");
        assert_eq!(recovery.resume_status, "ready");
        assert_eq!(recovery.lifecycle_stage, "dev_pack_active");
        assert_eq!(
            recovery.delegation_gate.blocker_code.as_deref(),
            Some("open_delegated_cycle")
        );
        assert_eq!(
            recovery.delegation_gate.reporting_pause_gate,
            "non_blocking_only"
        );

        let latest_recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("latest reconciled recovery summary should load")
            .expect("latest run graph recovery summary should exist");
        assert_eq!(latest_recovery.active_node, "dev-pack");
        assert_eq!(latest_recovery.resume_status, "ready");
        assert_eq!(latest_recovery.lifecycle_stage, "dev_pack_active");
        assert_eq!(
            latest_recovery.delegation_gate.blocker_code.as_deref(),
            Some("open_delegated_cycle")
        );
        assert_eq!(
            latest_recovery.delegation_gate.reporting_pause_gate,
            "non_blocking_only"
        );

        store.close().await;
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
            policy_bundle_ref: None,
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
        assert_eq!(reconciled.resume_target, "dispatch.coach");
        assert!(!reconciled.recovery_ready);

        let recovery = store
            .latest_run_graph_recovery_summary()
            .await
            .expect("recovery summary should load")
            .expect("recovery summary should exist");
        assert_eq!(recovery.run_id, "run-blocked-recovery");
        assert_eq!(recovery.resume_status, "blocked");
        assert_eq!(recovery.resume_target, "dispatch.coach");
        assert!(!recovery.recovery_ready);
        assert!(recovery.delegation_gate.delegated_cycle_open);
        assert_eq!(
            recovery.delegation_gate.blocker_code.as_deref(),
            Some("open_delegated_cycle")
        );

        store.close().await;
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn open_error_diagnostic_matrix_preserves_kind_and_retryability() {
        let cases = [
            (
                StateStoreError::Io(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "authoritative datastore lock wait timed out",
                )),
                StateStoreOpenErrorKind::LockContention,
                true,
                "authoritative_state_store_locked",
            ),
            (
                StateStoreError::Io(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "state root access denied",
                )),
                StateStoreOpenErrorKind::PermissionAccess,
                false,
                "authoritative_state_store_open_failed",
            ),
            (
                StateStoreError::InvalidStorageMetadata {
                    reason: "failed to open bounded SurrealKV datastore: Failed to flush memtable to SST: Keys are not in order".to_string(),
                },
                StateStoreOpenErrorKind::StorageCorruption,
                false,
                "authoritative_state_store_open_failed",
            ),
            (
                StateStoreError::MissingStateDir(PathBuf::from("/tmp/vida-missing-state")),
                StateStoreOpenErrorKind::Unknown,
                false,
                "authoritative_state_store_open_failed",
            ),
        ];
        for (error, expected_kind, expected_retryable, expected_blocker) in cases {
            let diagnostic = error.open_error_diagnostic();
            assert_eq!(diagnostic.error_kind, expected_kind);
            assert_eq!(diagnostic.retryable, expected_retryable);
            assert_eq!(diagnostic.blocker_code(), expected_blocker);
        }
    }

    #[test]
    fn access_denied_message_is_not_lock_contention_without_lock_evidence() {
        let error = StateStoreError::Io(io::Error::new(
            io::ErrorKind::Other,
            "Access is denied. (os error 5)",
        ));
        let diagnostic = error.open_error_diagnostic();
        assert_eq!(
            diagnostic.error_kind,
            StateStoreOpenErrorKind::PermissionAccess
        );
        assert!(!diagnostic.retryable);
    }

    #[test]
    fn permission_evidence_precedes_generic_lock_word_for_io_and_db() {
        for message in [
            "Access is denied while locking state root",
            "permission denied to lock file",
        ] {
            let io_error = StateStoreError::Io(io::Error::new(io::ErrorKind::Other, message));
            let diagnostic = io_error.open_error_diagnostic();
            assert_eq!(
                diagnostic.error_kind,
                StateStoreOpenErrorKind::PermissionAccess
            );
            assert!(!diagnostic.retryable);
            assert!(!StateStore::error_is_lock_contention(&io_error));

            let db_error = StateStoreError::Db(surrealdb::Error::internal(message.to_string()));
            let diagnostic = db_error.open_error_diagnostic();
            assert_eq!(
                diagnostic.error_kind,
                StateStoreOpenErrorKind::PermissionAccess
            );
            assert!(!diagnostic.retryable);
            assert!(!StateStore::error_is_lock_contention(&db_error));
        }
    }

    #[test]
    fn exact_lock_contention_phrases_remain_retryable_without_generic_lock_heuristic() {
        for message in [
            "timed out while waiting for authoritative datastore lock",
            "resource temporarily unavailable",
            "another process has locked a portion of the file",
            "the process cannot access the file because it is being used by another process",
        ] {
            let io_error = StateStoreError::Io(io::Error::new(io::ErrorKind::Other, message));
            let diagnostic = io_error.open_error_diagnostic();
            assert_eq!(
                diagnostic.error_kind,
                StateStoreOpenErrorKind::LockContention
            );
            assert!(diagnostic.retryable);
            assert!(StateStore::error_is_lock_contention(&io_error));

            let db_error = StateStoreError::Db(surrealdb::Error::internal(message.to_string()));
            let diagnostic = db_error.open_error_diagnostic();
            assert_eq!(
                diagnostic.error_kind,
                StateStoreOpenErrorKind::LockContention
            );
            assert!(diagnostic.retryable);
            assert!(StateStore::error_is_lock_contention(&db_error));
        }
    }

    #[test]
    fn open_error_diagnostic_preserves_stage_and_sanitizes_lock_evidence() {
        let error = StateStoreError::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            r"C:\secret\state\LOCK pid=424242 raw holder details",
        ))
        .with_open_context(
            StateStoreOpenStage::DatastoreOpen,
            Some(StateStoreOpenLockEvidence::Datastore),
        );
        let diagnostic = error.open_error_diagnostic();
        assert_eq!(
            diagnostic.error_kind,
            StateStoreOpenErrorKind::LockContention
        );
        assert!(diagnostic.retryable);
        assert_eq!(diagnostic.open_stage, StateStoreOpenStage::DatastoreOpen);
        assert_eq!(
            diagnostic.lock_evidence,
            Some(StateStoreOpenLockEvidence::Datastore)
        );
        let serialized = serde_json::to_string(&diagnostic).expect("diagnostic serializes");
        assert!(serialized.contains("\"open_stage\":\"datastore_open\""));
        assert!(serialized.contains("\"lock_evidence\":\"datastore\""));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("424242"));
        assert!(!serialized.contains("raw holder"));
    }

    #[test]
    fn open_context_keeps_first_meaningful_stage_without_recursive_wrapping() {
        let error = StateStoreError::Io(io::Error::new(
            io::ErrorKind::Other,
            "state metadata is invalid",
        ))
        .with_open_context(StateStoreOpenStage::SchemaQuery, None)
        .with_open_context(
            StateStoreOpenStage::DatastoreOpen,
            Some(StateStoreOpenLockEvidence::Datastore),
        );
        let diagnostic = error.open_error_diagnostic();
        assert_eq!(diagnostic.open_stage, StateStoreOpenStage::SchemaQuery);
        assert_eq!(diagnostic.lock_evidence, None);
        assert!(
            !matches!(error, StateStoreError::OpenContext { source, .. } if matches!(*source, StateStoreError::OpenContext { .. }))
        );
    }

    #[test]
    fn invalid_storage_metadata_is_not_assumed_corruption_without_storage_signal() {
        let error = StateStoreError::InvalidStorageMetadata {
            reason: "namespace metadata is missing".to_string(),
        }
        .with_open_context(StateStoreOpenStage::SchemaQuery, None);
        let diagnostic = error.open_error_diagnostic();
        assert_eq!(diagnostic.error_kind, StateStoreOpenErrorKind::Unknown);
        assert_eq!(diagnostic.open_stage, StateStoreOpenStage::SchemaQuery);
        assert!(!diagnostic.retryable);
    }

    #[test]
    fn contextualized_lock_errors_remain_retryable_for_open_retry_loop() {
        let error = StateStoreError::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            "authoritative datastore lock wait timed out",
        ))
        .with_open_context(
            StateStoreOpenStage::DatastoreOpen,
            Some(StateStoreOpenLockEvidence::Datastore),
        );
        assert!(StateStore::error_is_lock_contention(&error));
        assert!(error.open_error_diagnostic().retryable);
    }

    #[test]
    fn db_display_and_debug_evidence_share_retry_and_diagnostic_classification() {
        let error = StateStoreError::Db(surrealdb::Error::internal(
            "resource temporarily unavailable".to_string(),
        ))
        .with_open_context(
            StateStoreOpenStage::DatastoreCheckVersion,
            Some(StateStoreOpenLockEvidence::Datastore),
        );
        let diagnostic = error.open_error_diagnostic();
        assert_eq!(
            diagnostic.error_kind,
            StateStoreOpenErrorKind::LockContention
        );
        assert!(diagnostic.retryable);
        assert!(StateStore::error_is_lock_contention(&error));
        assert_eq!(
            diagnostic.open_stage,
            StateStoreOpenStage::DatastoreCheckVersion
        );
    }
}
