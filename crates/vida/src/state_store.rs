use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::release1_contracts::{
    canonical_blocker_code_str, canonical_compatibility_class_str, canonical_lane_status_str,
    canonical_release1_contract_type_str, canonical_release1_schema_version_str,
    derive_lane_status, BlockerCode, CompatibilityClass, LaneStatus, Release1ContractType,
    Release1SchemaVersion,
};
use crate::taskflow_run_graph::is_dispatch_resume_handoff_complete;
use serde_json::Deserializer;
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
"#;

fn state_schema_document() -> String {
    let storage_schema = SurrealStoreTarget::new(DEFAULT_STATE_DIR).bootstrap_schema_document();
    format!("{storage_schema}\n\n{INSTRUCTION_STATE_SCHEMA}")
}

#[derive(Debug)]
pub struct StateStore {
    db: Surreal<Db>,
    root: PathBuf,
}

const LAUNCHER_ACTIVATION_SNAPSHOT_ID: &str = "launcher_live";

impl StateStore {
    pub async fn open(root: PathBuf) -> Result<Self, StateStoreError> {
        fs::create_dir_all(&root)?;

        let db: Surreal<Db> = Surreal::new::<SurrealKv>(root.clone()).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        db.query(state_schema_document()).await?;

        let store = Self { db, root };
        store.ensure_minimal_authoritative_state_spine().await?;
        Ok(store)
    }

    pub async fn open_existing(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }

        for attempt in 0..80 {
            match Self::open_existing_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error)) if attempt < 79 => {
                    let message = error.to_string();
                    if message.contains("LOCK") || message.contains("lock") {
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                        continue;
                    }
                    return Err(StateStoreError::Db(error));
                }
                Err(error) => return Err(error),
            }
        }

        Self::open_existing_once(root).await
    }

    async fn open_existing_once(root: PathBuf) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(root.clone()).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        db.query(state_schema_document()).await?;

        let store = Self { db, root };
        store.ensure_minimal_authoritative_state_spine().await?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn write_launcher_activation_snapshot(
        &self,
        snapshot: &LauncherActivationSnapshot,
    ) -> Result<(), StateStoreError> {
        snapshot.validate()?;
        let _: Option<LauncherActivationSnapshot> = self
            .db
            .upsert((
                "launcher_activation_snapshot",
                LAUNCHER_ACTIVATION_SNAPSHOT_ID,
            ))
            .content(snapshot.clone())
            .await?;
        Ok(())
    }

    pub async fn read_launcher_activation_snapshot(
        &self,
    ) -> Result<LauncherActivationSnapshot, StateStoreError> {
        let row: Option<LauncherActivationSnapshot> = self
            .db
            .select((
                "launcher_activation_snapshot",
                LAUNCHER_ACTIVATION_SNAPSHOT_ID,
            ))
            .await?;
        let row = row.ok_or(StateStoreError::MissingLauncherActivationSnapshot)?;
        row.validate()?;
        Ok(row)
    }

    pub async fn storage_metadata_summary(
        &self,
    ) -> Result<StorageMetadataSummary, StateStoreError> {
        let row: Option<StorageMetaRow> = self.db.select(("storage_meta", "primary")).await?;
        let row = row.ok_or(StateStoreError::MissingMetadata)?;
        let expected = SurrealStoreTarget::new(DEFAULT_STATE_DIR).storage_meta();
        if row.engine != expected.engine
            || row.backend != expected.backend
            || row.namespace != expected.namespace
            || row.database != expected.database
            || row.state_schema_version != expected.state_schema_version
            || row.instruction_schema_version != expected.instruction_schema_version
        {
            return Err(StateStoreError::InvalidStorageMetadata {
                reason: format!(
                    "expected engine={} backend={} namespace={} database={} state_schema_version={} instruction_schema_version={}, got engine={} backend={} namespace={} database={} state_schema_version={} instruction_schema_version={}",
                    expected.engine,
                    expected.backend,
                    expected.namespace,
                    expected.database,
                    expected.state_schema_version,
                    expected.instruction_schema_version,
                    row.engine,
                    row.backend,
                    row.namespace,
                    row.database,
                    row.state_schema_version,
                    row.instruction_schema_version
                ),
            });
        }
        Ok(StorageMetadataSummary {
            engine: row.engine,
            backend: row.backend,
            namespace: row.namespace,
            database: row.database,
            state_schema_version: row.state_schema_version,
            instruction_schema_version: row.instruction_schema_version,
        })
    }

    pub async fn backend_summary(&self) -> Result<String, StateStoreError> {
        let summary = self.storage_metadata_summary().await?;
        Ok(format!(
            "{} state-v{} instruction-v{}",
            summary.backend, summary.state_schema_version, summary.instruction_schema_version
        ))
    }

    pub async fn ensure_minimal_authoritative_state_spine(&self) -> Result<(), StateStoreError> {
        let existing: Option<StateSpineManifestContent> =
            self.db.select(("state_spine_manifest", "primary")).await?;
        let initialized_at = existing
            .map(|row| row.initialized_at)
            .unwrap_or_else(|| unix_timestamp_nanos().to_string());

        let contract = SurrealStoreTarget::new(DEFAULT_STATE_DIR).state_spine_manifest_contract();
        let content = StateSpineManifestContent::from_contract(contract, initialized_at);

        let _: Option<StateSpineManifestContent> = self
            .db
            .upsert(("state_spine_manifest", "primary"))
            .content(content)
            .await?;
        Ok(())
    }

    pub async fn state_spine_summary(&self) -> Result<StateSpineSummary, StateStoreError> {
        let row: Option<StateSpineManifestContent> =
            self.db.select(("state_spine_manifest", "primary")).await?;
        let row = row.ok_or(StateStoreError::MissingStateSpineManifest)?;
        let expected = SurrealStoreTarget::new(DEFAULT_STATE_DIR).state_spine_manifest_contract();
        if row.manifest_id != expected.manifest_id
            || row.state_schema_version != expected.state_schema_version
            || row.authoritative_mutation_root != expected.authoritative_mutation_root
            || row.entity_surfaces != expected.entity_surfaces
        {
            return Err(StateStoreError::InvalidStateSpineManifest {
                reason: format!(
                    "expected manifest_id={} state_schema_version={} authoritative_mutation_root={} entity_surfaces={:?}, got manifest_id={} state_schema_version={} authoritative_mutation_root={} entity_surfaces={:?}",
                    expected.manifest_id,
                    expected.state_schema_version,
                    expected.authoritative_mutation_root,
                    expected.entity_surfaces,
                    row.manifest_id,
                    row.state_schema_version,
                    row.authoritative_mutation_root,
                    row.entity_surfaces,
                ),
            });
        }
        if row.authoritative_mutation_root.trim().is_empty() {
            return Err(StateStoreError::InvalidStateSpineManifest {
                reason: "authoritative mutation root is empty".to_string(),
            });
        }
        if row.entity_surfaces.is_empty() {
            return Err(StateStoreError::InvalidStateSpineManifest {
                reason: "entity surface list is empty".to_string(),
            });
        }
        Ok(StateSpineSummary {
            authoritative_mutation_root: row.authoritative_mutation_root,
            entity_surface_count: row.entity_surfaces.len(),
            state_schema_version: row.state_schema_version,
        })
    }

    pub async fn import_tasks_from_jsonl(
        &self,
        source_path: &Path,
    ) -> Result<TaskImportSummary, StateStoreError> {
        let raw = fs::read_to_string(source_path)?;
        let mut imported = 0usize;
        let mut unchanged = 0usize;
        let mut updated = 0usize;

        for (index, record) in Deserializer::from_str(&raw)
            .into_iter::<TaskJsonlRecord>()
            .enumerate()
        {
            let record = record.map_err(|error| StateStoreError::InvalidTaskJsonLine {
                line: index + 1,
                reason: error.to_string(),
            })?;
            let task_id = record.id.trim().to_string();
            if task_id.is_empty() {
                return Err(StateStoreError::InvalidTaskRecord {
                    reason: format!("line {} is missing task id", index + 1),
                });
            }

            let content = TaskContent::from(record);
            let existing: Option<TaskStorageRow> =
                self.db.select(("task", task_id.as_str())).await?;
            match existing {
                None => imported += 1,
                Some(current) if current == TaskStorageRow::from(content.clone()) => unchanged += 1,
                Some(_) => updated += 1,
            }

            let _: Option<TaskStorageRow> = self
                .db
                .upsert(("task", task_id.as_str()))
                .content(content.clone())
                .await?;

            let _ = self
                .db
                .query(format!(
                    "DELETE task_dependency WHERE issue_id = '{}';",
                    escape_surql_literal(&task_id)
                ))
                .await?;

            for dependency in &content.dependencies {
                let dep_id = format!(
                    "{}--{}--{}",
                    sanitize_record_id(&task_id),
                    sanitize_record_id(&dependency.depends_on_id),
                    sanitize_record_id(&dependency.edge_type)
                );
                let _: Option<TaskDependencyRecord> = self
                    .db
                    .upsert(("task_dependency", dep_id.as_str()))
                    .content(dependency.clone())
                    .await?;
            }
        }

        Ok(TaskImportSummary {
            source_path: source_path.display().to_string(),
            imported_count: imported,
            unchanged_count: unchanged,
            updated_count: updated,
        })
    }

    pub async fn export_tasks_to_jsonl(
        &self,
        target_path: &Path,
    ) -> Result<usize, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let mut body = String::new();
        for task in tasks {
            body.push_str(&serde_json::to_string(&task).map_err(|error| {
                StateStoreError::InvalidTaskRecord {
                    reason: format!("failed to serialize task export row: {error}"),
                }
            })?);
            body.push('\n');
        }
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, body)?;
        Ok(self.all_tasks().await?.len())
    }

    pub async fn list_tasks(
        &self,
        status: Option<&str>,
        include_closed: bool,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut rows = self.all_tasks().await?;
        rows.retain(|task| {
            if !include_closed && task.status == "closed" {
                return false;
            }
            match status {
                Some(expected) => task.status == expected,
                None => true,
            }
        });
        rows.sort_by(task_sort_key);
        Ok(rows)
    }

    pub async fn show_task(&self, task_id: &str) -> Result<TaskRecord, StateStoreError> {
        let row: Option<TaskStorageRow> = self.db.select(("task", task_id)).await?;
        row.map(TaskRecord::from)
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })
    }

    pub async fn ready_tasks(&self) -> Result<Vec<TaskRecord>, StateStoreError> {
        self.ready_tasks_scoped(None).await
    }

    pub async fn ready_tasks_scoped(
        &self,
        scope_task_id: Option<&str>,
    ) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut rows = self.all_tasks().await?;
        rows.sort_by(task_sort_key);

        let by_id = rows
            .iter()
            .map(|task| (task.id.clone(), task.status.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let scope_ids = if let Some(scope_task_id) = scope_task_id {
            Some(self.ready_scope_ids(&rows, scope_task_id)?)
        } else {
            None
        };

        let mut ready = rows
            .into_iter()
            .filter(|task| {
                scope_ids
                    .as_ref()
                    .map(|ids| ids.contains(&task.id))
                    .unwrap_or(true)
            })
            .filter(|task| task.status == "open" || task.status == "in_progress")
            .filter(|task| task.issue_type != "epic")
            .filter(|task| {
                task.dependencies.iter().all(|dependency| {
                    if dependency.edge_type == "parent-child" {
                        return true;
                    }
                    matches!(
                        by_id.get(&dependency.depends_on_id).map(String::as_str),
                        Some("closed")
                    )
                })
            })
            .collect::<Vec<_>>();

        ready.sort_by(task_ready_sort_key);
        Ok(ready)
    }

    fn ready_scope_ids(
        &self,
        rows: &[TaskRecord],
        scope_task_id: &str,
    ) -> Result<BTreeSet<String>, StateStoreError> {
        if !rows.iter().any(|task| task.id == scope_task_id) {
            return Err(StateStoreError::MissingTask {
                task_id: scope_task_id.to_string(),
            });
        }

        let mut children = BTreeMap::<String, Vec<String>>::new();
        for task in rows {
            for dependency in &task.dependencies {
                if dependency.edge_type != "parent-child" {
                    continue;
                }
                children
                    .entry(dependency.depends_on_id.clone())
                    .or_default()
                    .push(task.id.clone());
            }
        }

        let mut scope_ids = BTreeSet::new();
        let mut stack = vec![scope_task_id.to_string()];
        while let Some(current) = stack.pop() {
            if !scope_ids.insert(current.clone()) {
                continue;
            }
            if let Some(descendants) = children.get(&current) {
                stack.extend(descendants.iter().cloned());
            }
        }

        Ok(scope_ids)
    }

    pub async fn task_dependencies(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskDependencyStatus>, StateStoreError> {
        let task = self.show_task(task_id).await?;
        let by_id = self
            .all_tasks()
            .await?
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut dependencies = task
            .dependencies
            .into_iter()
            .map(|dependency| {
                let depends_on_id = dependency.depends_on_id.clone();
                let dependency_status = by_id
                    .get(&depends_on_id)
                    .map(|task| task.status.clone())
                    .unwrap_or_else(|| "missing".to_string());
                TaskDependencyStatus {
                    issue_id: dependency.issue_id,
                    depends_on_id,
                    edge_type: dependency.edge_type,
                    dependency_status,
                    dependency_issue_type: by_id
                        .get(&dependency.depends_on_id)
                        .map(|task| task.issue_type.clone()),
                }
            })
            .collect::<Vec<_>>();

        dependencies.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });
        Ok(dependencies)
    }

    pub async fn reverse_dependencies(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskDependencyStatus>, StateStoreError> {
        let _ = self.show_task(task_id).await?;
        let tasks = self.all_tasks().await?;
        let by_id = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut reverse = tasks
            .into_iter()
            .flat_map(|task| {
                let issue_id = task.id.clone();
                let issue_status = task.status.clone();
                let issue_type = task.issue_type.clone();
                task.dependencies
                    .into_iter()
                    .filter(move |dependency| dependency.depends_on_id == task_id)
                    .map(move |dependency| TaskDependencyStatus {
                        issue_id: issue_id.clone(),
                        depends_on_id: dependency.depends_on_id,
                        edge_type: dependency.edge_type,
                        dependency_status: issue_status.clone(),
                        dependency_issue_type: Some(issue_type.clone()),
                    })
            })
            .collect::<Vec<_>>();

        reverse.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.issue_id.cmp(&right.issue_id))
        });

        for item in &mut reverse {
            item.dependency_issue_type = by_id
                .get(&item.issue_id)
                .map(|task| task.issue_type.clone());
            item.dependency_status = by_id
                .get(&item.issue_id)
                .map(|task| task.status.clone())
                .unwrap_or_else(|| "missing".to_string());
        }

        Ok(reverse)
    }

    pub async fn blocked_tasks(&self) -> Result<Vec<BlockedTaskRecord>, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let by_id = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::BTreeMap<_, _>>();

        let mut blocked = tasks
            .into_iter()
            .filter(|task| task.status == "open" || task.status == "in_progress")
            .filter(|task| task.issue_type != "epic")
            .filter_map(|task| {
                let blockers = task
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.edge_type != "parent-child")
                    .filter_map(|dependency| {
                        let blocker_task = by_id.get(&dependency.depends_on_id)?;
                        if blocker_task.status == "closed" {
                            return None;
                        }
                        Some(TaskDependencyStatus {
                            issue_id: dependency.issue_id.clone(),
                            depends_on_id: dependency.depends_on_id.clone(),
                            edge_type: dependency.edge_type.clone(),
                            dependency_status: blocker_task.status.clone(),
                            dependency_issue_type: Some(blocker_task.issue_type.clone()),
                        })
                    })
                    .collect::<Vec<_>>();

                (!blockers.is_empty()).then_some(BlockedTaskRecord { task, blockers })
            })
            .collect::<Vec<_>>();

        blocked.sort_by(|left, right| task_ready_sort_key(&left.task, &right.task));
        Ok(blocked)
    }

    pub async fn task_dependency_tree(
        &self,
        task_id: &str,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let by_id = tasks
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let mut active = BTreeSet::new();
        Self::build_task_dependency_tree(&by_id, task_id, &mut active)
    }

    fn build_task_dependency_tree(
        by_id: &BTreeMap<String, TaskRecord>,
        task_id: &str,
        active: &mut BTreeSet<String>,
    ) -> Result<TaskDependencyTreeNode, StateStoreError> {
        let task = by_id
            .get(task_id)
            .cloned()
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;

        active.insert(task.id.clone());
        let mut dependencies = Vec::new();
        for dependency in &task.dependencies {
            let mut edge = TaskDependencyTreeEdge {
                issue_id: dependency.issue_id.clone(),
                depends_on_id: dependency.depends_on_id.clone(),
                edge_type: dependency.edge_type.clone(),
                dependency_status: "missing".to_string(),
                dependency_issue_type: None,
                node: None,
                cycle: false,
                missing: false,
            };

            if active.contains(&dependency.depends_on_id) {
                edge.cycle = true;
            } else if let Some(child) = by_id.get(&dependency.depends_on_id) {
                edge.dependency_status = child.status.clone();
                edge.dependency_issue_type = Some(child.issue_type.clone());
                let child_id = child.id.clone();
                let child_node = Self::build_task_dependency_tree(by_id, &child_id, active)?;
                edge.node = Some(Box::new(child_node));
            } else {
                edge.missing = true;
            }

            dependencies.push(edge);
        }
        active.remove(&task.id);

        dependencies.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });

        Ok(TaskDependencyTreeNode { task, dependencies })
    }

    pub async fn validate_task_graph(&self) -> Result<Vec<TaskGraphIssue>, StateStoreError> {
        let tasks = self.all_tasks().await?;
        Ok(Self::validate_task_graph_rows(&tasks))
    }

    pub async fn add_task_dependency(
        &self,
        issue_id: &str,
        depends_on_id: &str,
        edge_type: &str,
        created_by: &str,
    ) -> Result<TaskDependencyRecord, StateStoreError> {
        let mut tasks = self.all_tasks().await?;
        let target_exists = tasks.iter().any(|task| task.id == depends_on_id);
        if !target_exists {
            return Err(StateStoreError::MissingTask {
                task_id: depends_on_id.to_string(),
            });
        }

        let task_index = tasks
            .iter()
            .position(|task| task.id == issue_id)
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: issue_id.to_string(),
            })?;

        if tasks[task_index].dependencies.iter().any(|dependency| {
            dependency.depends_on_id == depends_on_id && dependency.edge_type == edge_type
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "dependency already exists: {} -> {} ({})",
                    issue_id, depends_on_id, edge_type
                ),
            });
        }

        let dependency = TaskDependencyRecord {
            issue_id: issue_id.to_string(),
            depends_on_id: depends_on_id.to_string(),
            edge_type: edge_type.to_string(),
            created_at: unix_timestamp_nanos().to_string(),
            created_by: created_by.to_string(),
            metadata: "{}".to_string(),
            thread_id: String::new(),
        };
        tasks[task_index].dependencies.push(dependency.clone());
        tasks[task_index].updated_at = unix_timestamp_nanos().to_string();
        tasks[task_index].dependencies.sort_by(|left, right| {
            left.edge_type
                .cmp(&right.edge_type)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });

        let issues = Self::validate_task_graph_rows(&tasks);
        if !issues.is_empty() {
            let first = issues.first().expect("issues is not empty");
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "dependency mutation would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        self.persist_task_record(tasks[task_index].clone()).await?;
        Ok(dependency)
    }

    pub async fn remove_task_dependency(
        &self,
        issue_id: &str,
        depends_on_id: &str,
        edge_type: &str,
    ) -> Result<TaskDependencyRecord, StateStoreError> {
        let task = self.show_task(issue_id).await?;
        let removed = task
            .dependencies
            .iter()
            .find(|dependency| {
                dependency.depends_on_id == depends_on_id && dependency.edge_type == edge_type
            })
            .cloned()
            .ok_or_else(|| StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "dependency does not exist: {} -> {} ({})",
                    issue_id, depends_on_id, edge_type
                ),
            })?;

        let mut updated = task;
        updated.dependencies.retain(|dependency| {
            !(dependency.depends_on_id == depends_on_id && dependency.edge_type == edge_type)
        });
        updated.updated_at = unix_timestamp_nanos().to_string();

        self.persist_task_record(updated).await?;
        Ok(removed)
    }

    pub async fn create_task(
        &self,
        request: CreateTaskRequest<'_>,
    ) -> Result<TaskRecord, StateStoreError> {
        let CreateTaskRequest {
            task_id,
            title,
            description,
            issue_type,
            status,
            priority,
            parent_id,
            labels,
            created_by,
            source_repo,
        } = request;

        let task_id = task_id.trim();
        let title = title.trim();
        if task_id.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "task id is empty".to_string(),
            });
        }
        if title.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("task `{task_id}` title is empty"),
            });
        }
        if self.show_task(task_id).await.is_ok() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("task already exists: {task_id}"),
            });
        }
        let normalized_parent_id = parent_id.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        if let Some(parent_id) = normalized_parent_id.as_deref() {
            if self.show_task(parent_id).await.is_err() {
                return Err(StateStoreError::MissingTask {
                    task_id: parent_id.to_string(),
                });
            }
        }

        let now = unix_timestamp_nanos().to_string();
        let mut normalized_labels = labels
            .iter()
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty())
            .collect::<Vec<_>>();
        normalized_labels.sort();
        normalized_labels.dedup();

        let mut dependencies = Vec::new();
        if let Some(parent_id) = normalized_parent_id.clone() {
            dependencies.push(TaskDependencyRecord {
                issue_id: task_id.to_string(),
                depends_on_id: parent_id.to_string(),
                edge_type: "parent-child".to_string(),
                created_at: now.clone(),
                created_by: created_by.to_string(),
                metadata: "{}".to_string(),
                thread_id: String::new(),
            });
        }

        let mut task = TaskRecord {
            id: task_id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            status: status.to_string(),
            priority,
            issue_type: issue_type.to_string(),
            created_at: now.clone(),
            created_by: created_by.to_string(),
            updated_at: now.clone(),
            closed_at: None,
            close_reason: None,
            source_repo: source_repo.to_string(),
            compaction_level: 0,
            original_size: 0,
            notes: None,
            labels: normalized_labels,
            dependencies,
        };
        if status == "closed" {
            task.closed_at = Some(now);
        }

        let mut tasks = self.all_tasks().await?;
        tasks.push(task.clone());
        let issues = Self::validate_task_graph_rows(&tasks);
        if let Some(first) = issues.first() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "task creation would create invalid graph: {} on {}",
                    first.issue_type, first.issue_id
                ),
            });
        }

        self.persist_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn update_task(
        &self,
        request: UpdateTaskRequest<'_>,
    ) -> Result<TaskRecord, StateStoreError> {
        let UpdateTaskRequest {
            task_id,
            status,
            notes,
            description,
            add_labels,
            remove_labels,
            set_labels,
        } = request;
        let mut task = self.show_task(task_id).await?;
        if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
            task.status = status.to_string();
            if status == "closed" {
                if task.closed_at.is_none() {
                    task.closed_at = Some(unix_timestamp_nanos().to_string());
                }
            } else {
                task.closed_at = None;
                task.close_reason = None;
            }
        }
        if let Some(notes) = notes {
            task.notes = Some(notes.to_string());
        }
        if let Some(description) = description {
            task.description = description.to_string();
        }
        if let Some(set_labels) = set_labels {
            task.labels = set_labels
                .iter()
                .map(|label| label.trim().to_string())
                .filter(|label| !label.is_empty())
                .collect::<Vec<_>>();
        }
        for label in add_labels {
            let label = label.trim();
            if label.is_empty() || task.labels.iter().any(|existing| existing == label) {
                continue;
            }
            task.labels.push(label.to_string());
        }
        if !remove_labels.is_empty() {
            task.labels
                .retain(|label| !remove_labels.iter().any(|remove| remove == label));
        }
        task.labels.sort();
        task.labels.dedup();
        task.updated_at = unix_timestamp_nanos().to_string();
        self.persist_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn close_task(
        &self,
        task_id: &str,
        reason: &str,
    ) -> Result<TaskRecord, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let open_children = tasks
            .iter()
            .filter(|task| {
                task.id != task_id
                    && task.status != "closed"
                    && task.dependencies.iter().any(|dependency| {
                        dependency.edge_type == "parent-child"
                            && dependency.depends_on_id == task_id
                    })
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        if !open_children.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "cannot close task `{task_id}` while open child tasks exist: {}",
                    open_children.join(", ")
                ),
            });
        }

        let mut task = self.show_task(task_id).await?;
        let now = unix_timestamp_nanos().to_string();
        task.status = "closed".to_string();
        task.updated_at = now.clone();
        task.closed_at = Some(now);
        task.close_reason = Some(reason.to_string());
        self.persist_task_record(task.clone()).await?;
        Ok(task)
    }

    pub async fn critical_path(&self) -> Result<TaskCriticalPath, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let issues = Self::validate_task_graph_rows(&tasks);
        if !issues.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: "task graph is invalid; run `vida task validate-graph` first".to_string(),
            });
        }

        let by_id = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let active_ids = tasks
            .iter()
            .filter(|task| {
                (task.status == "open" || task.status == "in_progress") && task.issue_type != "epic"
            })
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();

        let mut memo = BTreeMap::<String, Vec<String>>::new();
        let mut active = BTreeSet::new();
        let mut best = Vec::new();
        for task_id in active_ids {
            let path = Self::critical_path_for_task(&by_id, &task_id, &mut memo, &mut active)?;
            if compare_task_paths(&path, &best).is_gt() {
                best = path;
            }
        }

        let nodes = best
            .into_iter()
            .filter_map(|task_id| by_id.get(&task_id))
            .map(|task| TaskCriticalPathNode {
                id: task.id.clone(),
                title: task.title.clone(),
                status: task.status.clone(),
                issue_type: task.issue_type.clone(),
                priority: task.priority,
            })
            .collect::<Vec<_>>();

        Ok(TaskCriticalPath {
            length: nodes.len(),
            root_task_id: nodes.first().map(|node| node.id.clone()),
            terminal_task_id: nodes.last().map(|node| node.id.clone()),
            release_1_contract_steps: vec![TaskRelease1ContractStep {
                id: "doctor_run_graph_negative_control".to_string(),
                mode: "fail_closed".to_string(),
                blocker_code: "missing_run_graph_dispatch_receipt_operator_evidence".to_string(),
                next_action: "Run `vida taskflow run-graph dispatch --json` to materialize run-graph dispatch receipt evidence before operator handoff.".to_string(),
            }],
            nodes,
        })
    }

    fn validate_task_graph_rows(tasks: &[TaskRecord]) -> Vec<TaskGraphIssue> {
        let by_id = tasks
            .iter()
            .map(|task| (task.id.clone(), task))
            .collect::<BTreeMap<_, _>>();
        let mut issues = Vec::new();

        for task in tasks {
            let parent_edges = task
                .dependencies
                .iter()
                .filter(|dependency| dependency.edge_type == "parent-child")
                .collect::<Vec<_>>();
            if parent_edges.len() > 1 {
                issues.push(TaskGraphIssue {
                    issue_type: "multiple_parent_edges".to_string(),
                    issue_id: task.id.clone(),
                    depends_on_id: None,
                    edge_type: Some("parent-child".to_string()),
                    detail: format!(
                        "task has {} parent-child edges; only one parent is allowed",
                        parent_edges.len()
                    ),
                });
            }

            for dependency in &task.dependencies {
                if !by_id.contains_key(&dependency.depends_on_id) {
                    issues.push(TaskGraphIssue {
                        issue_type: "missing_dependency_target".to_string(),
                        issue_id: task.id.clone(),
                        depends_on_id: Some(dependency.depends_on_id.clone()),
                        edge_type: Some(dependency.edge_type.clone()),
                        detail: "dependency target is missing from the authoritative runtime store"
                            .to_string(),
                    });
                }
                if dependency.depends_on_id == task.id {
                    issues.push(TaskGraphIssue {
                        issue_type: "self_dependency".to_string(),
                        issue_id: task.id.clone(),
                        depends_on_id: Some(dependency.depends_on_id.clone()),
                        edge_type: Some(dependency.edge_type.clone()),
                        detail: "task must not depend on itself".to_string(),
                    });
                }
            }
        }

        let mut parent_children = BTreeMap::<String, Vec<String>>::new();
        for task in tasks {
            for dependency in &task.dependencies {
                if dependency.edge_type == "parent-child" {
                    parent_children
                        .entry(dependency.depends_on_id.clone())
                        .or_default()
                        .push(task.id.clone());
                }
            }
        }

        let mut visited = BTreeSet::new();
        let mut active = BTreeSet::new();
        for task in tasks {
            Self::validate_parent_child_cycles(
                &task.id,
                &parent_children,
                &mut visited,
                &mut active,
                &mut issues,
            );
        }

        issues.sort_by(|left, right| {
            left.issue_type
                .cmp(&right.issue_type)
                .then_with(|| left.issue_id.cmp(&right.issue_id))
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
        });
        issues.dedup();
        issues
    }

    fn critical_path_for_task(
        by_id: &BTreeMap<String, TaskRecord>,
        task_id: &str,
        memo: &mut BTreeMap<String, Vec<String>>,
        active: &mut BTreeSet<String>,
    ) -> Result<Vec<String>, StateStoreError> {
        if let Some(path) = memo.get(task_id) {
            return Ok(path.clone());
        }
        if !active.insert(task_id.to_string()) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!("critical-path cycle detected at {task_id}"),
            });
        }

        let task = by_id
            .get(task_id)
            .ok_or_else(|| StateStoreError::MissingTask {
                task_id: task_id.to_string(),
            })?;
        let mut best_dependency_path = Vec::new();
        for dependency in &task.dependencies {
            if dependency.edge_type == "parent-child" {
                continue;
            }
            let Some(dep_task) = by_id.get(&dependency.depends_on_id) else {
                continue;
            };
            if dep_task.status == "closed" || dep_task.issue_type == "epic" {
                continue;
            }

            let candidate = Self::critical_path_for_task(by_id, &dep_task.id, memo, active)?;
            if compare_task_paths(&candidate, &best_dependency_path).is_gt() {
                best_dependency_path = candidate;
            }
        }

        active.remove(task_id);
        best_dependency_path.push(task_id.to_string());
        memo.insert(task_id.to_string(), best_dependency_path.clone());
        Ok(best_dependency_path)
    }

    fn validate_parent_child_cycles(
        task_id: &str,
        parent_children: &BTreeMap<String, Vec<String>>,
        visited: &mut BTreeSet<String>,
        active: &mut BTreeSet<String>,
        issues: &mut Vec<TaskGraphIssue>,
    ) {
        if active.contains(task_id) {
            issues.push(TaskGraphIssue {
                issue_type: "parent_child_cycle".to_string(),
                issue_id: task_id.to_string(),
                depends_on_id: Some(task_id.to_string()),
                edge_type: Some("parent-child".to_string()),
                detail: "parent-child ancestry contains a cycle".to_string(),
            });
            return;
        }
        if visited.contains(task_id) {
            return;
        }

        visited.insert(task_id.to_string());
        active.insert(task_id.to_string());
        if let Some(children) = parent_children.get(task_id) {
            for child in children {
                Self::validate_parent_child_cycles(child, parent_children, visited, active, issues);
            }
        }
        active.remove(task_id);
    }

    async fn persist_task_record(&self, task: TaskRecord) -> Result<(), StateStoreError> {
        let task_id = task.id.clone();
        let row = TaskStorageRow::from(task.clone());
        let _: Option<TaskStorageRow> = self
            .db
            .upsert(("task", task_id.as_str()))
            .content(row)
            .await?;
        self.replace_task_dependency_rows(&task_id, &task.dependencies)
            .await?;
        Ok(())
    }

    async fn replace_task_dependency_rows(
        &self,
        task_id: &str,
        dependencies: &[TaskDependencyRecord],
    ) -> Result<(), StateStoreError> {
        let _ = self
            .db
            .query(format!(
                "DELETE task_dependency WHERE issue_id = '{}';",
                escape_surql_literal(task_id)
            ))
            .await?;

        for dependency in dependencies {
            let dep_id = format!(
                "{}--{}--{}",
                sanitize_record_id(task_id),
                sanitize_record_id(&dependency.depends_on_id),
                sanitize_record_id(&dependency.edge_type)
            );
            let _: Option<TaskDependencyRecord> = self
                .db
                .upsert(("task_dependency", dep_id.as_str()))
                .content(dependency.clone())
                .await?;
        }

        Ok(())
    }

    async fn delete_task_record(&self, task_id: &str) -> Result<(), StateStoreError> {
        let _: Option<TaskStorageRow> = self.db.delete(("task", task_id)).await?;
        let _ = self
            .db
            .query(format!(
                "DELETE task_dependency WHERE issue_id = '{}';",
                escape_surql_literal(task_id)
            ))
            .await?;
        Ok(())
    }

    async fn write_task_reconciliation_summary(
        &self,
        input: TaskReconciliationSummaryInput,
    ) -> Result<(), StateStoreError> {
        let receipt_id = format!("task-reconciliation-{}", unix_timestamp_nanos());
        let _: Option<TaskReconciliationSummaryRow> = self
            .db
            .upsert(("task_reconciliation_summary", receipt_id.as_str()))
            .content(TaskReconciliationSummaryRow {
                receipt_id,
                operation: input.operation,
                source_kind: input.source_kind,
                source_path: input.source_path,
                task_count: input.task_count,
                dependency_count: input.dependency_count,
                stale_removed_count: input.stale_removed_count,
                recorded_at: unix_timestamp_nanos().to_string(),
            })
            .await?;
        Ok(())
    }

    async fn all_tasks(&self) -> Result<Vec<TaskRecord>, StateStoreError> {
        let mut query = self
            .db
            .query("SELECT * FROM task ORDER BY priority ASC, id ASC;")
            .await?;
        let rows: Vec<TaskStorageRow> = query.take(0)?;
        Ok(rows.into_iter().map(TaskRecord::from).collect())
    }

    pub async fn evaluate_boot_compatibility(
        &self,
    ) -> Result<BootCompatibilitySummary, StateStoreError> {
        let mut reasons = Vec::new();
        let mut hard_failures = 0usize;

        if let Err(error) = self.storage_metadata_summary().await {
            reasons.push(error.to_string());
            hard_failures += 1;
        }
        if let Err(error) = self.state_spine_summary().await {
            reasons.push(error.to_string());
            hard_failures += 1;
        }
        let active_root = match self.active_instruction_root().await {
            Ok(value) => Some(value),
            Err(_) => {
                reasons.push("instruction runtime state missing".to_string());
                hard_failures += 1;
                None
            }
        };
        if let Some(root_artifact_id) = active_root.as_deref() {
            if self
                .inspect_effective_instruction_bundle(root_artifact_id)
                .await
                .is_err()
            {
                reasons.push("effective instruction bundle unresolved".to_string());
                hard_failures += 1;
            }
        }

        let classification = if reasons.is_empty() {
            "compatible"
        } else if hard_failures > 0 {
            "incompatible"
        } else {
            "insufficient_evidence"
        };

        let summary = BootCompatibilitySummary {
            classification: classification.to_string(),
            reasons,
            next_step: if classification == "compatible" {
                "normal_boot_allowed".to_string()
            } else {
                "stop_and_repair_prerequisites".to_string()
            },
        };

        let _: Option<BootCompatibilityStateRow> = self
            .db
            .upsert(("boot_compatibility_state", "primary"))
            .content(BootCompatibilityStateRow {
                state_id: "primary".to_string(),
                classification: summary.classification.clone(),
                reasons: summary.reasons.clone(),
                next_step: summary.next_step.clone(),
                evaluated_at: unix_timestamp_nanos().to_string(),
            })
            .await?;

        Ok(summary)
    }

    pub async fn latest_boot_compatibility_summary(
        &self,
    ) -> Result<Option<BootCompatibilitySummary>, StateStoreError> {
        let row: Option<BootCompatibilityStateRow> = self
            .db
            .select(("boot_compatibility_state", "primary"))
            .await?;
        Ok(row.map(|row| BootCompatibilitySummary {
            classification: row.classification,
            reasons: row.reasons,
            next_step: row.next_step,
        }))
    }

    pub async fn evaluate_migration_preflight(
        &self,
    ) -> Result<MigrationPreflightSummary, StateStoreError> {
        let mut blockers = Vec::new();
        if let Err(error) = self.storage_metadata_summary().await {
            blockers.push(error.to_string());
        }
        if let Err(error) = self.state_spine_summary().await {
            blockers.push(error.to_string());
        }
        let source_version_tuple = match self.active_instruction_root().await {
            Ok(root_artifact_id) => match self
                .inspect_effective_instruction_bundle(&root_artifact_id)
                .await
            {
                Ok(bundle) => bundle.source_version_tuple,
                Err(error) => {
                    blockers.push(format!("effective instruction bundle unresolved: {error}"));
                    Vec::new()
                }
            },
            Err(error) => {
                blockers.push(format!("instruction runtime root unresolved: {error}"));
                Vec::new()
            }
        };

        let compatibility_classification = if blockers.is_empty() {
            CompatibilityClass::Compatible
        } else {
            CompatibilityClass::Incompatible
        };
        let migration_state = if blockers.is_empty() {
            "no_migration_required"
        } else {
            "migration_blocked"
        };
        let next_step = if blockers.is_empty() {
            "normal_boot_allowed"
        } else {
            "stop_and_repair_migration_inputs"
        };

        let summary = MigrationPreflightSummary {
            contract_type: canonical_release1_contract_type_str(
                Release1ContractType::OperatorContracts.as_str(),
            )
            .unwrap_or(Release1ContractType::OperatorContracts.as_str())
            .to_string(),
            schema_version: canonical_release1_schema_version_str(
                Release1SchemaVersion::V1.as_str(),
            )
            .unwrap_or(Release1SchemaVersion::V1.as_str())
            .to_string(),
            compatibility_classification: compatibility_classification.as_str().to_string(),
            migration_state: migration_state.to_string(),
            blockers,
            source_version_tuple,
            next_step: next_step.to_string(),
        };

        let _: Option<MigrationRuntimeStateRow> = self
            .db
            .upsert(("migration_runtime_state", "primary"))
            .content(MigrationRuntimeStateRow {
                state_id: "primary".to_string(),
                contract_type: summary.contract_type.clone(),
                schema_version: summary.schema_version.clone(),
                migration_state: summary.migration_state.clone(),
                compatibility_classification: summary.compatibility_classification.clone(),
                blockers: summary.blockers.clone(),
                source_version_tuple: summary.source_version_tuple.clone(),
                next_step: summary.next_step.clone(),
                evaluated_at: unix_timestamp_nanos().to_string(),
            })
            .await?;

        let _: Option<MigrationCompatibilityReceiptRow> = self
            .db
            .upsert(("migration_compatibility_receipt", "primary"))
            .content(MigrationCompatibilityReceiptRow {
                receipt_id: "primary".to_string(),
                contract_type: summary.contract_type.clone(),
                schema_version: summary.schema_version.clone(),
                compatibility_classification: summary.compatibility_classification.clone(),
                migration_state: summary.migration_state.clone(),
                blockers: summary.blockers.clone(),
                source_version_tuple: summary.source_version_tuple.clone(),
                next_step: summary.next_step.clone(),
                evaluated_at: unix_timestamp_nanos().to_string(),
            })
            .await?;

        Ok(summary)
    }

    pub async fn latest_migration_preflight_summary(
        &self,
    ) -> Result<Option<MigrationPreflightSummary>, StateStoreError> {
        let row: Option<MigrationRuntimeStateRow> = self
            .db
            .select(("migration_runtime_state", "primary"))
            .await?;
        Ok(row.map(|row| MigrationPreflightSummary {
            contract_type: canonical_release1_contract_type_str(&row.contract_type)
                .unwrap_or(Release1ContractType::OperatorContracts.as_str())
                .to_string(),
            schema_version: canonical_release1_schema_version_str(&row.schema_version)
                .unwrap_or(Release1SchemaVersion::V1.as_str())
                .to_string(),
            compatibility_classification: canonical_compatibility_class_str(
                &row.compatibility_classification,
            )
            .unwrap_or(CompatibilityClass::Incompatible.as_str())
            .to_string(),
            migration_state: row.migration_state,
            blockers: row.blockers,
            source_version_tuple: row.source_version_tuple,
            next_step: row.next_step,
        }))
    }

    pub async fn migration_receipt_summary(
        &self,
    ) -> Result<MigrationReceiptSummary, StateStoreError> {
        Ok(MigrationReceiptSummary {
            compatibility_receipts: self
                .count_table_rows("migration_compatibility_receipt")
                .await?,
            application_receipts: self
                .count_table_rows("migration_application_receipt")
                .await?,
            verification_receipts: self
                .count_table_rows("migration_verification_receipt")
                .await?,
            cutover_readiness_receipts: self
                .count_table_rows("migration_cutover_readiness_receipt")
                .await?,
            rollback_notes: self.count_table_rows("migration_rollback_note").await?,
        })
    }

    pub async fn task_store_summary(&self) -> Result<TaskStoreSummary, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let open_count = tasks.iter().filter(|task| task.status == "open").count();
        let in_progress_count = tasks
            .iter()
            .filter(|task| task.status == "in_progress")
            .count();
        let closed_count = tasks.iter().filter(|task| task.status == "closed").count();
        let epic_count = tasks
            .iter()
            .filter(|task| task.issue_type == "epic")
            .count();
        let ready_count = self.ready_tasks().await?.len();

        Ok(TaskStoreSummary {
            total_count: tasks.len(),
            open_count,
            in_progress_count,
            closed_count,
            epic_count,
            ready_count,
        })
    }

    #[allow(dead_code)]
    async fn build_taskflow_snapshot(&self) -> Result<TaskSnapshot, StateStoreError> {
        let tasks = self.all_tasks().await?;
        let mut snapshot_tasks = Vec::with_capacity(tasks.len());
        let mut snapshot_dependencies = Vec::new();

        for task in tasks {
            snapshot_dependencies.extend(
                task.dependencies
                    .iter()
                    .map(task_dependency_to_canonical_edge),
            );
            snapshot_tasks.push(task_record_to_canonical_snapshot_row(&task)?);
        }

        snapshot_tasks.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        snapshot_dependencies.sort_by(|left, right| {
            left.issue_id
                .0
                .cmp(&right.issue_id.0)
                .then_with(|| left.depends_on_id.0.cmp(&right.depends_on_id.0))
                .then_with(|| left.dependency_type.cmp(&right.dependency_type))
        });

        Ok(TaskSnapshot {
            tasks: snapshot_tasks,
            dependencies: snapshot_dependencies,
        })
    }

    #[allow(dead_code)]
    pub async fn export_taskflow_snapshot(&self) -> Result<TaskSnapshot, StateStoreError> {
        let snapshot = self.build_taskflow_snapshot().await?;
        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: "export_snapshot".to_string(),
            source_kind: "canonical_snapshot_object".to_string(),
            source_path: None,
            task_count: snapshot.tasks.len(),
            dependency_count: snapshot.dependencies.len(),
            stale_removed_count: 0,
        })
        .await?;
        Ok(snapshot)
    }

    #[allow(dead_code)]
    pub async fn export_taskflow_in_memory_store(
        &self,
    ) -> Result<InMemoryTaskStore, StateStoreError> {
        let snapshot = self.build_taskflow_snapshot().await?;
        let restored = restore_canonical_in_memory_store(&snapshot);
        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: "export_snapshot".to_string(),
            source_kind: "canonical_snapshot_memory".to_string(),
            source_path: None,
            task_count: snapshot.tasks.len(),
            dependency_count: snapshot.dependencies.len(),
            stale_removed_count: 0,
        })
        .await?;
        Ok(restored)
    }

    #[allow(dead_code)]
    pub async fn write_taskflow_snapshot(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), StateStoreError> {
        let snapshot = self.build_taskflow_snapshot().await?;
        let source_path = path.as_ref().display().to_string();
        write_canonical_snapshot(path, &snapshot)?;
        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: "export_snapshot".to_string(),
            source_kind: "canonical_snapshot_file".to_string(),
            source_path: Some(source_path),
            task_count: snapshot.tasks.len(),
            dependency_count: snapshot.dependencies.len(),
            stale_removed_count: 0,
        })
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn read_taskflow_snapshot_into_memory(
        path: impl AsRef<Path>,
    ) -> Result<InMemoryTaskStore, StateStoreError> {
        Ok(read_canonical_snapshot_into_memory(path)?)
    }

    #[allow(dead_code)]
    pub async fn import_taskflow_snapshot(
        &self,
        snapshot: &TaskSnapshot,
    ) -> Result<(), StateStoreError> {
        let task_records = task_records_from_canonical_snapshot_for_additive_import(
            snapshot,
            &self.all_tasks().await?,
        )?;
        for task in task_records {
            self.persist_task_record(task).await?;
        }
        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: "import_snapshot".to_string(),
            source_kind: "canonical_snapshot_memory".to_string(),
            source_path: None,
            task_count: snapshot.tasks.len(),
            dependency_count: snapshot.dependencies.len(),
            stale_removed_count: 0,
        })
        .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn import_taskflow_snapshot_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), StateStoreError> {
        let source_path = path.as_ref().display().to_string();
        let snapshot = taskflow_state_fs::read_snapshot(path)?;
        let task_records = task_records_from_canonical_snapshot_for_additive_import(
            &snapshot,
            &self.all_tasks().await?,
        )?;
        for task in task_records {
            self.persist_task_record(task).await?;
        }
        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: "import_snapshot".to_string(),
            source_kind: "canonical_snapshot_file".to_string(),
            source_path: Some(source_path),
            task_count: snapshot.tasks.len(),
            dependency_count: snapshot.dependencies.len(),
            stale_removed_count: 0,
        })
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn replace_with_taskflow_snapshot(
        &self,
        snapshot: &TaskSnapshot,
    ) -> Result<(), StateStoreError> {
        let task_records = task_records_from_canonical_snapshot(snapshot)?;
        let keep_ids = task_records
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();

        for task in task_records {
            self.persist_task_record(task).await?;
        }

        let mut stale_removed_count = 0usize;
        for task_id in self
            .all_tasks()
            .await?
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
        {
            if !keep_ids.contains(&task_id) {
                self.delete_task_record(&task_id).await?;
                stale_removed_count += 1;
            }
        }

        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: "replace_snapshot".to_string(),
            source_kind: "canonical_snapshot_memory".to_string(),
            source_path: None,
            task_count: snapshot.tasks.len(),
            dependency_count: snapshot.dependencies.len(),
            stale_removed_count,
        })
        .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn replace_with_taskflow_snapshot_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), StateStoreError> {
        let source_path = path.as_ref().display().to_string();
        let snapshot = taskflow_state_fs::read_snapshot(path)?;
        let task_records = task_records_from_canonical_snapshot(&snapshot)?;
        let keep_ids = task_records
            .iter()
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        for task in task_records {
            self.persist_task_record(task).await?;
        }
        let mut stale_removed_count = 0usize;
        for task_id in self
            .all_tasks()
            .await?
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
        {
            if !keep_ids.contains(&task_id) {
                self.delete_task_record(&task_id).await?;
                stale_removed_count += 1;
            }
        }
        self.write_task_reconciliation_summary(TaskReconciliationSummaryInput {
            operation: "replace_snapshot".to_string(),
            source_kind: "canonical_snapshot_file".to_string(),
            source_path: Some(source_path),
            task_count: snapshot.tasks.len(),
            dependency_count: snapshot.dependencies.len(),
            stale_removed_count,
        })
        .await?;
        Ok(())
    }

    pub async fn run_graph_summary(&self) -> Result<RunGraphSummary, StateStoreError> {
        Ok(RunGraphSummary {
            execution_plan_count: self.count_table_rows("execution_plan_state").await?,
            routed_run_count: self.count_table_rows("routed_run_state").await?,
            governance_count: self.count_table_rows("governance_state").await?,
            resumability_count: self.count_table_rows("resumability_capsule").await?,
            reconciliation_count: self.count_table_rows("task_reconciliation_summary").await?,
        })
    }

    #[allow(dead_code)]
    pub async fn record_run_graph_status(
        &self,
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        status.validate_memory_governance()?;
        let updated_at = unix_timestamp_nanos().to_string();
        let _: Option<RoutedRunStateRow> = self
            .db
            .upsert(("routed_run_state", status.run_id.as_str()))
            .content(RoutedRunStateRow {
                run_id: status.run_id.clone(),
                route_task_class: status.route_task_class.clone(),
                selected_backend: status.selected_backend.clone(),
                lane_id: status.lane_id.clone(),
                lifecycle_stage: status.lifecycle_stage.clone(),
                updated_at: updated_at.clone(),
            })
            .await?;
        let _: Option<GovernanceStateRow> = self
            .db
            .upsert(("governance_state", status.run_id.as_str()))
            .content(GovernanceStateRow {
                run_id: status.run_id.clone(),
                policy_gate: status.policy_gate.clone(),
                handoff_state: status.handoff_state.clone(),
                context_state: status.context_state.clone(),
                updated_at: updated_at.clone(),
            })
            .await?;
        let _: Option<ResumabilityCapsuleRow> = self
            .db
            .upsert(("resumability_capsule", status.run_id.as_str()))
            .content(ResumabilityCapsuleRow {
                run_id: status.run_id.clone(),
                checkpoint_kind: status.checkpoint_kind.clone(),
                resume_target: status.resume_target.clone(),
                recovery_ready: status.recovery_ready,
                updated_at,
            })
            .await?;
        let _: Option<ExecutionPlanStateRow> = self
            .db
            .upsert(("execution_plan_state", status.run_id.as_str()))
            .content(ExecutionPlanStateRow {
                run_id: status.run_id.clone(),
                task_id: status.task_id.clone(),
                task_class: status.task_class.clone(),
                active_node: status.active_node.clone(),
                next_node: status.next_node.clone(),
                status: status.status.clone(),
                updated_at: unix_timestamp_nanos().to_string(),
            })
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn record_run_graph_dispatch_receipt(
        &self,
        receipt: &RunGraphDispatchReceipt,
    ) -> Result<(), StateStoreError> {
        let receipt: RunGraphDispatchReceiptStored = receipt.clone().into();
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        let _: Option<RunGraphDispatchReceiptStored> = self
            .db
            .upsert(("run_graph_dispatch_receipt", receipt.run_id.as_str()))
            .content(receipt)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn run_graph_status(&self, run_id: &str) -> Result<RunGraphStatus, StateStoreError> {
        let execution: Option<ExecutionPlanStateRow> =
            self.db.select(("execution_plan_state", run_id)).await?;
        let execution = execution.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph:{run_id}"),
        })?;
        let routed: Option<RoutedRunStateRow> =
            self.db.select(("routed_run_state", run_id)).await?;
        let routed = routed.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph_route:{run_id}"),
        })?;
        let governance: Option<GovernanceStateRow> =
            self.db.select(("governance_state", run_id)).await?;
        let governance = governance.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph_governance:{run_id}"),
        })?;
        let resumability: Option<ResumabilityCapsuleRow> =
            self.db.select(("resumability_capsule", run_id)).await?;
        let resumability = resumability.ok_or_else(|| StateStoreError::MissingTask {
            task_id: format!("run_graph_resumability:{run_id}"),
        })?;

        let status = RunGraphStatus {
            run_id: execution.run_id,
            task_id: execution.task_id,
            task_class: execution.task_class,
            active_node: execution.active_node,
            next_node: execution.next_node,
            status: execution.status,
            route_task_class: routed.route_task_class,
            selected_backend: routed.selected_backend,
            lane_id: routed.lane_id,
            lifecycle_stage: routed.lifecycle_stage,
            policy_gate: governance.policy_gate,
            handoff_state: governance.handoff_state,
            context_state: governance.context_state,
            checkpoint_kind: resumability.checkpoint_kind,
            resume_target: resumability.resume_target,
            recovery_ready: resumability.recovery_ready,
        };
        status.validate_memory_governance()?;
        Ok(status)
    }

    pub async fn latest_run_graph_status(&self) -> Result<Option<RunGraphStatus>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        Ok(Some(self.run_graph_status(&run_id).await?))
    }

    async fn latest_run_graph_run_id(&self) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, updated_at FROM execution_plan_state ORDER BY updated_at DESC, run_id DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<RunGraphLatestRow> = query.take(0)?;
        Ok(rows.into_iter().next().map(|latest| latest.run_id))
    }

    async fn ensure_run_graph_recovery_surface_rows_present(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let governance: Option<GovernanceStateRow> =
            self.db.select(("governance_state", run_id)).await?;
        let resumability: Option<ResumabilityCapsuleRow> =
            self.db.select(("resumability_capsule", run_id)).await?;
        if governance.is_none() || resumability.is_none() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/checkpoint summary is inconsistent for `{run_id}`: latest status requires both governance and resumability rows (governance_present={}, resumability_present={})",
                    governance.is_some(),
                    resumability.is_some()
                ),
            });
        }
        Ok(())
    }

    async fn latest_run_graph_checkpoint_run_id(&self) -> Result<Option<String>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT run_id, updated_at FROM resumability_capsule ORDER BY updated_at DESC, run_id DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<RunGraphLatestRow> = query.take(0)?;
        Ok(rows.into_iter().next().map(|latest| latest.run_id))
    }

    async fn ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(
        &self,
        run_id: &str,
    ) -> Result<(), StateStoreError> {
        let latest_checkpoint_run_id = self.latest_run_graph_checkpoint_run_id().await?;
        if latest_checkpoint_run_id.as_deref() != Some(run_id) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/checkpoint summary is inconsistent for `{run_id}`: latest checkpoint evidence must share the same run_id (latest_checkpoint_run_id={})",
                    latest_checkpoint_run_id.as_deref().unwrap_or("none")
                ),
            });
        }
        Ok(())
    }

    fn ensure_run_graph_recovery_surface_consistency(
        status: &RunGraphStatus,
    ) -> Result<(), StateStoreError> {
        if status.resume_target.starts_with("dispatch.")
            && !is_dispatch_resume_handoff_complete(status)
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph recovery/gate summary is inconsistent for `{}`: dispatch resume target `{}` requires complete handoff metadata (next_node={}, policy_gate=`{}`, handoff=`{}`)",
                    status.run_id,
                    status.resume_target,
                    status.next_node.as_deref().unwrap_or("none"),
                    status.policy_gate,
                    status.handoff_state
                ),
            });
        }
        Ok(())
    }

    pub async fn ensure_memory_governance_guard(&self) -> Result<(), StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(());
        };
        status.validate_memory_governance()
    }

    pub async fn latest_run_graph_dispatch_receipt_summary(
        &self,
    ) -> Result<Option<RunGraphDispatchReceiptSummary>, StateStoreError> {
        let Some(status) = self.latest_run_graph_status().await? else {
            return Ok(None);
        };
        let latest_checkpoint_run_id = self.latest_run_graph_checkpoint_run_id().await?;
        if latest_checkpoint_run_id.as_deref() != Some(status.run_id.as_str()) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: latest checkpoint evidence must share the same run_id (latest_checkpoint_run_id={})",
                    status.run_id,
                    latest_checkpoint_run_id.as_deref().unwrap_or("none")
                ),
            });
        }
        let Some(receipt) = self
            .run_graph_dispatch_receipt_stored(&status.run_id)
            .await?
        else {
            return Ok(None);
        };
        let receipt = Self::validate_run_graph_dispatch_receipt_contract(receipt)?;
        Ok(Some(RunGraphDispatchReceiptSummary::from_receipt(
            receipt.into(),
        )))
    }

    pub async fn latest_run_graph_dispatch_receipt(
        &self,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT * FROM run_graph_dispatch_receipt ORDER BY recorded_at DESC, run_id DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<RunGraphDispatchReceiptStored> = query.take(0)?;
        Ok(rows.into_iter().next().map(Into::into))
    }

    pub async fn run_graph_dispatch_receipt(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceipt>, StateStoreError> {
        self.run_graph_dispatch_receipt_stored(run_id)
            .await
            .map(|row| row.map(Into::into))
    }

    async fn run_graph_dispatch_receipt_stored(
        &self,
        run_id: &str,
    ) -> Result<Option<RunGraphDispatchReceiptStored>, StateStoreError> {
        self.db
            .select(("run_graph_dispatch_receipt", run_id))
            .await
            .map_err(Into::into)
    }

    pub async fn latest_run_graph_recovery_summary(
        &self,
    ) -> Result<Option<RunGraphRecoverySummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        Ok(Some(RunGraphRecoverySummary::from_status(status)))
    }

    pub async fn latest_run_graph_checkpoint_summary(
        &self,
    ) -> Result<Option<RunGraphCheckpointSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        Ok(Some(RunGraphCheckpointSummary::from_status(status)))
    }

    pub async fn latest_run_graph_gate_summary(
        &self,
    ) -> Result<Option<RunGraphGateSummary>, StateStoreError> {
        let Some(run_id) = self.latest_run_graph_run_id().await? else {
            return Ok(None);
        };
        let status = self.load_consistent_run_graph_status(&run_id).await?;
        Ok(Some(RunGraphGateSummary::from_status(status)))
    }

    pub async fn run_graph_recovery_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphRecoverySummary, StateStoreError> {
        let status = self
            .load_consistent_run_graph_status(run_id)
            .await?;
        Ok(RunGraphRecoverySummary::from_status(status))
    }

    async fn load_consistent_run_graph_status(
        &self,
        run_id: &str,
    ) -> Result<RunGraphStatus, StateStoreError> {
        self.ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(run_id)
            .await?;
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(status)
    }

    pub async fn run_graph_checkpoint_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphCheckpointSummary, StateStoreError> {
        self.ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(run_id)
            .await?;
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphCheckpointSummary::from_status(status))
    }

    pub async fn run_graph_gate_summary(
        &self,
        run_id: &str,
    ) -> Result<RunGraphGateSummary, StateStoreError> {
        self.ensure_run_graph_recovery_surface_latest_checkpoint_matches_run_id(run_id)
            .await?;
        self.ensure_run_graph_recovery_surface_rows_present(run_id)
            .await?;
        let status = self.run_graph_status(run_id).await?;
        Self::ensure_run_graph_recovery_surface_consistency(&status)?;
        Ok(RunGraphGateSummary::from_status(status))
    }

    fn ensure_run_graph_dispatch_receipt_summary_consistency(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        if receipt.dispatch_status.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: dispatch_status must be non-empty",
                    receipt.run_id
                ),
            });
        }
        let Some(raw_lane_status) = receipt.lane_status.as_deref() else {
            return Ok(());
        };
        if raw_lane_status.trim().is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: lane_status must be non-empty when present",
                    receipt.run_id
                ),
            });
        }
        let raw_lane_status = raw_lane_status.trim();
        let derived_lane_status = derive_lane_status(
            &receipt.dispatch_status,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str();
        let canonical_lane_status =
            canonical_lane_status_str(raw_lane_status).unwrap_or(raw_lane_status);
        if receipt.downstream_dispatch_status.is_some()
            && canonical_lane_status != derived_lane_status
        {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_status `{}` with lane_status `{}` conflicts with derived lane_status `{}` from dispatch_status `{}`",
                    receipt.run_id,
                    receipt.downstream_dispatch_status.as_deref().unwrap_or("none"),
                    canonical_lane_status,
                    derived_lane_status,
                    receipt.dispatch_status
                ),
            });
        }
        Ok(())
    }

    fn ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(
        receipt: &RunGraphDispatchReceiptStored,
    ) -> Result<(), StateStoreError> {
        let Some(downstream_status) = receipt.downstream_dispatch_status.as_deref() else {
            return Ok(());
        };
        let downstream_status = downstream_status.trim().to_ascii_lowercase();
        let requires_blockers = downstream_status == "blocked";
        if requires_blockers && receipt.downstream_dispatch_blockers.is_empty() {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must be present and non-empty when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt.downstream_dispatch_status.as_deref().unwrap_or("none")
                ),
            });
        }
        if receipt.downstream_dispatch_blockers.is_empty() {
            return Ok(());
        }
        let mut canonical_blockers = std::collections::HashSet::new();
        if receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            let raw_blocker = blocker.as_str();
            let blocker = blocker.trim();
            let collapsed = blocker.split_whitespace().collect::<Vec<_>>().join(" ");
            raw_blocker != blocker
                || blocker.is_empty()
                || !blocker.is_ascii()
                || blocker.to_ascii_lowercase() != blocker
                || collapsed != blocker
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must contain only non-empty ASCII lowercase canonical entries without whitespace, case, internal spacing, or unicode drift when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt.downstream_dispatch_status.as_deref().unwrap_or("none")
                ),
            });
        }
        if receipt.downstream_dispatch_blockers.iter().any(|blocker| {
            let canonical_blocker = blocker.trim().to_ascii_lowercase();
            !canonical_blockers.insert(canonical_blocker)
        }) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "run-graph dispatch receipt summary is inconsistent for `{}`: downstream_dispatch_blockers must not contain duplicate canonical entries after lowercase canonicalization when downstream_dispatch_status `{}` is present",
                    receipt.run_id,
                    receipt.downstream_dispatch_status.as_deref().unwrap_or("none")
                ),
            });
        }
        Ok(())
    }

    fn validate_run_graph_dispatch_receipt_contract(
        receipt: RunGraphDispatchReceiptStored,
    ) -> Result<RunGraphDispatchReceiptStored, StateStoreError> {
        Self::ensure_run_graph_dispatch_receipt_summary_consistency(&receipt)?;
        Self::ensure_run_graph_dispatch_receipt_summary_downstream_blockers_canonical(&receipt)?;
        Ok(receipt)
    }

    pub async fn latest_task_reconciliation_summary(
        &self,
    ) -> Result<Option<TaskReconciliationSummary>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT receipt_id, operation, source_kind, source_path, task_count, dependency_count, stale_removed_count, recorded_at FROM task_reconciliation_summary ORDER BY recorded_at DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<TaskReconciliationSummary> = query.take(0)?;
        Ok(rows.into_iter().next())
    }

    pub async fn task_reconciliation_rollup(
        &self,
    ) -> Result<TaskReconciliationRollup, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT operation, source_kind, source_path, task_count, dependency_count, stale_removed_count, recorded_at FROM task_reconciliation_summary ORDER BY recorded_at DESC;",
            )
            .await?;
        let rows: Vec<TaskReconciliationRollupRow> = query.take(0)?;
        let mut by_operation = BTreeMap::<String, usize>::new();
        let mut by_source_kind = BTreeMap::<String, usize>::new();
        let latest_recorded_at = rows.first().map(|row| row.recorded_at.clone());
        let latest_source_path = rows.first().and_then(|row| row.source_path.clone());
        let mut total_task_rows = 0usize;
        let mut total_dependency_rows = 0usize;
        let mut total_stale_removed = 0usize;

        for row in &rows {
            *by_operation.entry(row.operation.clone()).or_insert(0) += 1;
            *by_source_kind.entry(row.source_kind.clone()).or_insert(0) += 1;
            total_task_rows += row.task_count;
            total_dependency_rows += row.dependency_count;
            total_stale_removed += row.stale_removed_count;
        }

        Ok(TaskReconciliationRollup {
            total_receipts: by_operation.values().sum(),
            latest_recorded_at,
            latest_source_path,
            total_task_rows,
            total_dependency_rows,
            total_stale_removed,
            by_operation,
            by_source_kind,
            rows,
        })
    }

    pub async fn taskflow_snapshot_bridge_summary(
        &self,
    ) -> Result<TaskflowSnapshotBridgeSummary, StateStoreError> {
        let latest_receipt = self.latest_task_reconciliation_summary().await?;
        let rollup = self.task_reconciliation_rollup().await?;
        Ok(TaskflowSnapshotBridgeSummary {
            total_receipts: rollup.total_receipts,
            export_receipts: *rollup.by_operation.get("export_snapshot").unwrap_or(&0),
            import_receipts: *rollup.by_operation.get("import_snapshot").unwrap_or(&0),
            replace_receipts: *rollup.by_operation.get("replace_snapshot").unwrap_or(&0),
            object_export_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("export_snapshot"),
                Some("canonical_snapshot_object"),
            ),
            memory_export_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("export_snapshot"),
                Some("canonical_snapshot_memory"),
            ),
            memory_import_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("import_snapshot"),
                Some("canonical_snapshot_memory"),
            ),
            memory_replace_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("replace_snapshot"),
                Some("canonical_snapshot_memory"),
            ),
            file_export_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("export_snapshot"),
                Some("canonical_snapshot_file"),
            ),
            file_import_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("import_snapshot"),
                Some("canonical_snapshot_file"),
            ),
            file_replace_receipts: count_snapshot_bridge_rows(
                &rollup.rows,
                Some("replace_snapshot"),
                Some("canonical_snapshot_file"),
            ),
            total_task_rows: rollup.total_task_rows,
            total_dependency_rows: rollup.total_dependency_rows,
            total_stale_removed: rollup.total_stale_removed,
            latest_operation: latest_receipt
                .as_ref()
                .map(|receipt| receipt.operation.clone()),
            latest_source_kind: latest_receipt
                .as_ref()
                .map(|receipt| receipt.source_kind.clone()),
            latest_source_path: rollup.latest_source_path,
            latest_recorded_at: rollup.latest_recorded_at,
        })
    }

    pub async fn latest_effective_bundle_receipt_summary(
        &self,
    ) -> Result<Option<EffectiveBundleReceiptSummary>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT receipt_id, root_artifact_id, artifact_count FROM effective_instruction_bundle_receipt ORDER BY receipt_id DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<EffectiveBundleReceiptSummary> = query.take(0)?;
        Ok(rows.into_iter().next())
    }

    pub async fn record_protocol_binding_snapshot(
        &self,
        scenario: &str,
        primary_state_authority: &str,
        bindings: &[ProtocolBindingState],
    ) -> Result<ProtocolBindingReceipt, StateStoreError> {
        if scenario.trim().is_empty() {
            return Err(StateStoreError::InvalidProtocolBinding {
                reason: "scenario is required".to_string(),
            });
        }
        if primary_state_authority.trim().is_empty() {
            return Err(StateStoreError::InvalidProtocolBinding {
                reason: "primary_state_authority is required".to_string(),
            });
        }
        if bindings.is_empty() {
            return Err(StateStoreError::InvalidProtocolBinding {
                reason: "at least one protocol binding row is required".to_string(),
            });
        }

        let recorded_at = unix_timestamp_nanos().to_string();
        let receipt_id = format!("protocol-binding-{recorded_at}");
        let scenario_literal = escape_surql_literal(scenario);
        self.db
            .query(format!(
                "DELETE protocol_binding_state WHERE scenario = '{scenario_literal}';"
            ))
            .await?;

        let mut active_bindings = 0usize;
        let mut script_bound_count = 0usize;
        let mut rust_bound_count = 0usize;
        let mut fully_runtime_bound_count = 0usize;
        let mut unbound_count = 0usize;
        let mut blocking_issue_count = 0usize;

        for binding in bindings {
            let record = ProtocolBindingStateRow::from_state(
                scenario,
                primary_state_authority,
                recorded_at.clone(),
                binding.clone(),
            );
            if record.active {
                active_bindings += 1;
            }
            match record.binding_status.as_str() {
                "script-bound" => script_bound_count += 1,
                "rust-bound" => rust_bound_count += 1,
                "fully-runtime-bound" => fully_runtime_bound_count += 1,
                _ => unbound_count += 1,
            }
            blocking_issue_count += record.blockers.len();
            let row_id = format!(
                "{}--{}",
                sanitize_record_id(scenario),
                sanitize_record_id(&record.protocol_id)
            );
            let _: Option<ProtocolBindingStateRow> = self
                .db
                .upsert(("protocol_binding_state", row_id.as_str()))
                .content(record)
                .await?;
        }

        let receipt = ProtocolBindingReceipt {
            receipt_id,
            scenario: scenario.to_string(),
            total_bindings: bindings.len(),
            active_bindings,
            script_bound_count,
            rust_bound_count,
            fully_runtime_bound_count,
            unbound_count,
            blocking_issue_count,
            primary_state_authority: primary_state_authority.to_string(),
            recorded_at,
        };
        let _: Option<ProtocolBindingReceipt> = self
            .db
            .upsert(("protocol_binding_receipt", receipt.receipt_id.as_str()))
            .content(receipt.clone())
            .await?;
        Ok(receipt)
    }

    pub async fn latest_protocol_binding_receipt(
        &self,
    ) -> Result<Option<ProtocolBindingReceipt>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT receipt_id, scenario, total_bindings, active_bindings, script_bound_count, rust_bound_count, fully_runtime_bound_count, unbound_count, blocking_issue_count, primary_state_authority, recorded_at FROM protocol_binding_receipt ORDER BY recorded_at DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<ProtocolBindingReceipt> = query.take(0)?;
        Ok(rows.into_iter().next())
    }

    pub async fn latest_protocol_binding_cache_token(
        &self,
    ) -> Result<Option<String>, StateStoreError> {
        let Some(receipt) = self.latest_protocol_binding_receipt().await? else {
            return Ok(None);
        };
        if receipt.receipt_id.trim().is_empty()
            || receipt.recorded_at.trim().is_empty()
            || receipt.primary_state_authority.trim().is_empty()
        {
            return Ok(None);
        }
        Ok(Some(format!(
            "{}::{}::{}",
            receipt.primary_state_authority, receipt.receipt_id, receipt.recorded_at
        )))
    }

    pub async fn latest_protocol_binding_rows(
        &self,
    ) -> Result<Vec<ProtocolBindingState>, StateStoreError> {
        let Some(receipt) = self.latest_protocol_binding_receipt().await? else {
            return Ok(Vec::new());
        };
        let mut query = self
            .db
            .query(format!(
                "SELECT protocol_id, source_path, activation_class, runtime_owner, enforcement_type, proof_surface, primary_state_authority, binding_status, active, blockers, scenario, synced_at FROM protocol_binding_state WHERE scenario = '{}' ORDER BY protocol_id ASC;",
                escape_surql_literal(&receipt.scenario)
            ))
            .await?;
        let rows: Vec<ProtocolBindingStateRow> = query.take(0)?;
        Ok(rows
            .into_iter()
            .map(ProtocolBindingState::from_row)
            .collect())
    }

    pub async fn protocol_binding_summary(
        &self,
    ) -> Result<ProtocolBindingSummary, StateStoreError> {
        let latest_receipt = self.latest_protocol_binding_receipt().await?;
        Ok(match latest_receipt {
            Some(receipt) => ProtocolBindingSummary {
                total_receipts: self.count_table_rows("protocol_binding_receipt").await?,
                total_bindings: receipt.total_bindings,
                active_bindings: receipt.active_bindings,
                script_bound_count: receipt.script_bound_count,
                rust_bound_count: receipt.rust_bound_count,
                fully_runtime_bound_count: receipt.fully_runtime_bound_count,
                unbound_count: receipt.unbound_count,
                blocking_issue_count: receipt.blocking_issue_count,
                latest_receipt_id: Some(receipt.receipt_id),
                latest_scenario: Some(receipt.scenario),
                latest_recorded_at: Some(receipt.recorded_at),
                primary_state_authority: Some(receipt.primary_state_authority),
            },
            None => ProtocolBindingSummary {
                total_receipts: 0,
                total_bindings: 0,
                active_bindings: 0,
                script_bound_count: 0,
                rust_bound_count: 0,
                fully_runtime_bound_count: 0,
                unbound_count: 0,
                blocking_issue_count: 0,
                latest_receipt_id: None,
                latest_scenario: None,
                latest_recorded_at: None,
                primary_state_authority: None,
            },
        })
    }

    async fn count_table_rows(&self, table: &str) -> Result<usize, StateStoreError> {
        let query = format!("SELECT count() AS total FROM {table} GROUP ALL;");
        let mut response = self.db.query(query).await?;
        let rows: Vec<CountRow> = response.take(0)?;
        Ok(rows.into_iter().next().map(|row| row.total).unwrap_or(0))
    }

    pub async fn seed_framework_instruction_bundle(&self) -> Result<(), StateStoreError> {
        let existing_runtime_state: Option<InstructionRuntimeStateRow> = self
            .db
            .select(("instruction_runtime_state", "primary"))
            .await?;
        let active_root_artifact_id = existing_runtime_state
            .as_ref()
            .map(|row| row.active_root_artifact_id.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "framework-agent-definition".to_string());
        let runtime_mode = existing_runtime_state
            .as_ref()
            .map(|row| row.runtime_mode.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "framework_seed".to_string());

        let query = r#"
UPSERT instruction_artifact:framework-agent-definition CONTENT {
  artifact_id: 'framework-agent-definition',
  artifact_kind: 'agent_definition',
  version: 1,
  ownership_class: 'framework',
  mutability_class: 'immutable',
  source_hash: 'seed-framework-agent-definition-v1',
  activation_class: 'always_on',
  required_follow_on: ['framework-instruction-contract', 'framework-prompt-template-config']
};

UPSERT instruction_artifact:framework-instruction-contract CONTENT {
  artifact_id: 'framework-instruction-contract',
  artifact_kind: 'instruction_contract',
  version: 1,
  ownership_class: 'framework',
  mutability_class: 'immutable',
  source_hash: 'seed-framework-instruction-contract-v1',
  activation_class: 'always_on',
  required_follow_on: []
};

UPSERT instruction_artifact:framework-prompt-template-config CONTENT {
  artifact_id: 'framework-prompt-template-config',
  artifact_kind: 'prompt_template_configuration',
  version: 1,
  ownership_class: 'framework',
  mutability_class: 'immutable',
  source_hash: 'seed-framework-prompt-template-config-v1',
  activation_class: 'always_on',
  required_follow_on: []
};

UPSERT instruction_dependency_edge:framework-agent-definition__framework-instruction-contract CONTENT {
  from_artifact: 'framework-agent-definition',
  to_artifact: 'framework-instruction-contract',
  edge_kind: 'mandatory_follow_on'
};

UPSERT instruction_dependency_edge:framework-agent-definition__framework-prompt-template-config CONTENT {
  from_artifact: 'framework-agent-definition',
  to_artifact: 'framework-prompt-template-config',
  edge_kind: 'mandatory_follow_on'
};

UPSERT instruction_migration_receipt:framework-bundle-v1 CONTENT {
  receipt_id: 'framework-bundle-v1',
  bundle_version: 1,
  state_schema_version: 1,
  instruction_schema_version: 1,
  receipt_kind: 'seed',
  applied: true
};

UPSERT instruction_sidecar:framework-sidecar-substrate CONTENT {
  sidecar_id: 'framework-sidecar-substrate',
  target_artifact_id: 'framework-instruction-contract',
  patch_format: 'structured_diff',
  active: false,
  sidecar_kind: 'seed_substrate'
};

UPSERT instruction_diff_patch:framework-diff-substrate CONTENT {
  patch_id: 'framework-diff-substrate',
  target_artifact_id: 'framework-instruction-contract',
  active: false,
  patch_kind: 'seed_substrate',
  operations: []
};

UPSERT source_tree_config:instruction CONTENT {
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  ingest_kind: 'git_tree',
  runtime_owner: 'db_mirror',
  nested_directories_supported: true,
  markdown_supported: true
};

UPSERT source_tree_config:project CONTENT {
  slice: 'project_memory',
  source_root: 'docs/project-memory',
  ingest_kind: 'git_tree',
  runtime_owner: 'db_mirror',
  nested_directories_supported: true,
  markdown_supported: true
};

UPSERT source_tree_config:framework CONTENT {
  slice: 'framework_memory',
  source_root: 'vida/config/instructions/bundles/framework-memory-source',
  ingest_kind: 'git_tree',
  runtime_owner: 'db_mirror',
  nested_directories_supported: true,
  markdown_supported: true
};

UPSERT instruction_source_artifact:framework-agent-definition-source CONTENT {
  source_artifact_id: 'framework-agent-definition-source',
  artifact_id: 'framework-agent-definition',
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  source_path: 'vida/config/instructions/bundles/framework-source/framework/agent-definition.md',
  content_hash: 'seed-framework-agent-definition-v1',
  ingest_status: 'seeded',
  hierarchy: ['framework']
};

UPSERT instruction_source_artifact:framework-instruction-contract-source CONTENT {
  source_artifact_id: 'framework-instruction-contract-source',
  artifact_id: 'framework-instruction-contract',
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  source_path: 'vida/config/instructions/bundles/framework-source/framework/instruction-contract.md',
  content_hash: 'seed-framework-instruction-contract-v1',
  ingest_status: 'seeded',
  hierarchy: ['framework']
};

UPSERT instruction_source_artifact:framework-prompt-template-config-source CONTENT {
  source_artifact_id: 'framework-prompt-template-config-source',
  artifact_id: 'framework-prompt-template-config',
  slice: 'instruction_memory',
  source_root: 'vida/config/instructions/bundles/framework-source',
  source_path: 'vida/config/instructions/bundles/framework-source/framework/prompt-template-config.md',
  content_hash: 'seed-framework-prompt-template-config-v1',
  ingest_status: 'seeded',
  hierarchy: ['framework']
};

UPSERT instruction_ingest_receipt:framework-bundle-seed CONTENT {
  receipt_id: 'framework-bundle-seed',
  source_root: 'vida/config/instructions/bundles/framework-source',
  product_version: '__VIDA_PRODUCT_VERSION__',
  ingest_kind: 'seed',
  applied: true
};

"#
        .replace("__VIDA_PRODUCT_VERSION__", VIDA_PRODUCT_VERSION);

        self.db.query(query).await?;
        let _: Option<InstructionRuntimeStateRow> = self
            .db
            .upsert(("instruction_runtime_state", "primary"))
            .content(InstructionRuntimeStateRow {
                state_id: "primary".to_string(),
                active_root_artifact_id,
                runtime_mode,
            })
            .await?;
        Ok(())
    }

    pub async fn source_tree_summary(&self) -> Result<String, StateStoreError> {
        let row: Option<SourceTreeConfigRow> = self
            .db
            .select(("source_tree_config", "instruction"))
            .await?;
        let row = row.ok_or(StateStoreError::MissingSourceTreeConfig)?;
        Ok(format!("{} -> {}", row.source_root, row.slice))
    }

    pub async fn active_instruction_root(&self) -> Result<String, StateStoreError> {
        let row: Option<InstructionRuntimeStateRow> = self
            .db
            .select(("instruction_runtime_state", "primary"))
            .await?;
        let row = row.ok_or(StateStoreError::MissingInstructionRuntimeState)?;
        if row.active_root_artifact_id.trim().is_empty() {
            return Err(StateStoreError::InvalidInstructionRuntimeState {
                reason: "active root artifact id is empty".to_string(),
            });
        }
        Ok(row.active_root_artifact_id)
    }

    pub async fn ingest_instruction_source_tree(
        &self,
        source_root: &str,
    ) -> Result<InstructionIngestSummary, StateStoreError> {
        self.ingest_tree("instruction_memory", source_root).await
    }

    pub async fn ingest_framework_memory_source_tree(
        &self,
        source_root: &str,
    ) -> Result<InstructionIngestSummary, StateStoreError> {
        self.ingest_tree("framework_memory", source_root).await
    }

    async fn ingest_tree(
        &self,
        slice: &str,
        source_root: &str,
    ) -> Result<InstructionIngestSummary, StateStoreError> {
        let root = repo_root().join(source_root);
        if !root.exists() {
            return Err(StateStoreError::MissingSourceRoot {
                slice: slice.to_string(),
                path: root,
            });
        }

        let files = collect_markdown_files(&root)?;
        let mut imported = 0usize;
        let mut unchanged = 0usize;
        let mut updated = 0usize;

        for path in files {
            let relative = path
                .strip_prefix(&root)
                .map_err(|_| StateStoreError::InvalidSourcePath(path.clone()))?;
            let body = fs::read_to_string(&path)?;
            let hash = blake3::hash(body.as_bytes()).to_hex().to_string();
            let metadata = parse_source_metadata(&body);
            let artifact_id = metadata
                .artifact_id
                .clone()
                .unwrap_or_else(|| artifact_id_from_path(relative));
            let hierarchy = if metadata.hierarchy.is_empty() {
                hierarchy_from_path(relative)
            } else {
                metadata.hierarchy.clone()
            };
            let artifact_kind = metadata
                .artifact_kind
                .clone()
                .unwrap_or_else(|| infer_artifact_kind(slice, relative));
            let version = metadata.version.unwrap_or(1);
            let ownership_class = metadata
                .ownership_class
                .clone()
                .unwrap_or_else(|| infer_ownership_class(slice).to_string());
            let mutability_class = metadata
                .mutability_class
                .clone()
                .unwrap_or_else(|| infer_mutability_class(slice).to_string());
            let activation_class = metadata.activation_class.clone().unwrap_or_default();
            let source_record_id = record_id_for_slice_source(slice, relative);

            let existing: Option<SourceArtifactRow> = self
                .db
                .select(("instruction_source_artifact", source_record_id.as_str()))
                .await?;

            let status = match existing {
                Some(existing_row) if existing_row.content_hash == hash => {
                    unchanged += 1;
                    "unchanged"
                }
                Some(_) => {
                    updated += 1;
                    "updated"
                }
                None => {
                    imported += 1;
                    "imported"
                }
            };

            let _: Option<SourceArtifactContent> = self
                .db
                .upsert(("instruction_source_artifact", source_record_id.as_str()))
                .content(SourceArtifactContent {
                    source_artifact_id: source_record_id.clone(),
                    artifact_id: artifact_id.clone(),
                    artifact_kind: artifact_kind.clone(),
                    version,
                    ownership_class: ownership_class.clone(),
                    mutability_class: mutability_class.clone(),
                    slice: slice.to_string(),
                    source_root: source_root.to_string(),
                    source_path: normalize_path(path.as_path()),
                    content_hash: hash.clone(),
                    ingest_status: status.to_string(),
                    hierarchy: hierarchy.clone(),
                })
                .await?;

            let _: Option<InstructionArtifactContent> = self
                .db
                .upsert(("instruction_artifact", artifact_id.as_str()))
                .content(InstructionArtifactContent {
                    artifact_id: artifact_id.clone(),
                    artifact_kind: artifact_kind.clone(),
                    version,
                    ownership_class: ownership_class.clone(),
                    mutability_class: mutability_class.clone(),
                    activation_class,
                    source_hash: hash,
                    body: body.clone(),
                    hierarchy: hierarchy.clone(),
                    required_follow_on: metadata.required_follow_on.clone(),
                })
                .await?;

            self.replace_dependency_edges(&artifact_id, &metadata.required_follow_on)
                .await?;
        }

        let receipt_id = format!("{}-ingest-{}", slice, unix_timestamp());
        let _: Option<InstructionIngestReceiptContent> = self
            .db
            .upsert(("instruction_ingest_receipt", receipt_id.as_str()))
            .content(InstructionIngestReceiptContent {
                receipt_id,
                slice: slice.to_string(),
                source_root: source_root.to_string(),
                product_version: VIDA_PRODUCT_VERSION.to_string(),
                ingest_kind: "scan".to_string(),
                applied: true,
                imported_count: imported,
                unchanged_count: unchanged,
                updated_count: updated,
            })
            .await?;

        Ok(InstructionIngestSummary {
            source_root: source_root.to_string(),
            imported_count: imported,
            unchanged_count: unchanged,
            updated_count: updated,
        })
    }

    async fn replace_dependency_edges(
        &self,
        artifact_id: &str,
        required_follow_on: &[String],
    ) -> Result<(), StateStoreError> {
        self.db
            .query(format!(
                "DELETE instruction_dependency_edge WHERE from_artifact = '{}';",
                artifact_id
            ))
            .await?;

        for target in required_follow_on {
            let edge_id = format!("{}__{}", artifact_id, target);
            let _: Option<InstructionDependencyEdgeContent> = self
                .db
                .upsert(("instruction_dependency_edge", edge_id.as_str()))
                .content(InstructionDependencyEdgeContent {
                    from_artifact: artifact_id.to_string(),
                    to_artifact: target.clone(),
                    edge_kind: "mandatory_follow_on".to_string(),
                })
                .await?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn project_instruction_artifact(
        &self,
        artifact_id: &str,
    ) -> Result<InstructionProjection, StateStoreError> {
        self.project_instruction_artifact_internal(artifact_id, true)
            .await
    }

    #[allow(dead_code)]
    pub async fn inspect_instruction_artifact(
        &self,
        artifact_id: &str,
    ) -> Result<InstructionProjection, StateStoreError> {
        self.project_instruction_artifact_internal(artifact_id, false)
            .await
    }

    async fn project_instruction_artifact_internal(
        &self,
        artifact_id: &str,
        persist_receipt: bool,
    ) -> Result<InstructionProjection, StateStoreError> {
        let base: Option<InstructionArtifactRow> = self
            .db
            .select(("instruction_artifact", artifact_id))
            .await?;
        let base = base.ok_or_else(|| StateStoreError::MissingInstructionArtifact {
            artifact_id: artifact_id.to_string(),
        })?;

        let mut sidecar_query = self
            .db
            .query(format!(
                "SELECT * FROM instruction_diff_patch WHERE target_artifact_id = '{}' AND active = true ORDER BY patch_precedence ASC, patch_id ASC;",
                artifact_id
            ))
            .await?;
        let patches: Vec<InstructionDiffPatchRow> = sidecar_query.take(0)?;

        if let Err(error) = validate_patch_bindings(&base, &patches) {
            if persist_receipt {
                self.write_projection_receipt(
                    artifact_id,
                    &base,
                    &base.body,
                    &[],
                    collect_patch_ids(&patches),
                    error.to_string(),
                )
                .await?;
            }
            return Err(error);
        }
        if let Err(error) = validate_patch_conflicts(&patches) {
            if persist_receipt {
                self.write_projection_receipt(
                    artifact_id,
                    &base,
                    &base.body,
                    &[],
                    collect_patch_ids(&patches),
                    error.to_string(),
                )
                .await?;
            }
            return Err(error);
        }

        let mut lines = split_lines(&base.body);
        let mut applied_patch_ids = Vec::new();
        let mut skipped_patch_ids = Vec::new();
        let mut failed_reason = String::new();

        for (index, patch) in patches.iter().enumerate() {
            for operation in &patch.operations {
                if let Err(error) = apply_patch_operation(&mut lines, operation) {
                    failed_reason = error.to_string();
                    skipped_patch_ids.extend(
                        patches
                            .iter()
                            .skip(index)
                            .map(|remaining| remaining.patch_id.clone()),
                    );

                    let projected_body = join_lines(&lines);
                    if persist_receipt {
                        self.write_projection_receipt(
                            artifact_id,
                            &base,
                            &projected_body,
                            &applied_patch_ids,
                            skipped_patch_ids.clone(),
                            failed_reason,
                        )
                        .await?;
                    }

                    return Err(error);
                }
            }
            applied_patch_ids.push(patch.patch_id.clone());
        }

        let projected_body = join_lines(&lines);
        let projection_hash = if persist_receipt {
            self.write_projection_receipt(
                artifact_id,
                &base,
                &projected_body,
                &applied_patch_ids,
                skipped_patch_ids.clone(),
                failed_reason,
            )
            .await?
        } else {
            blake3::hash(projected_body.as_bytes()).to_hex().to_string()
        };

        Ok(InstructionProjection {
            artifact_id: artifact_id.to_string(),
            body: projected_body,
            projected_hash: projection_hash,
            applied_patch_ids,
            skipped_patch_ids,
        })
    }

    #[allow(dead_code)]
    pub async fn upsert_instruction_diff_patch(
        &self,
        patch: InstructionDiffPatchContent,
    ) -> Result<(), StateStoreError> {
        let _: Option<InstructionDiffPatchContent> = self
            .db
            .upsert(("instruction_diff_patch", patch.patch_id.as_str()))
            .content(patch)
            .await?;
        Ok(())
    }

    async fn write_projection_receipt(
        &self,
        artifact_id: &str,
        base: &InstructionArtifactRow,
        projected_body: &str,
        applied_patch_ids: &[String],
        skipped_patch_ids: Vec<String>,
        failed_reason: String,
    ) -> Result<String, StateStoreError> {
        let projected_hash = blake3::hash(projected_body.as_bytes()).to_hex().to_string();
        let receipt_id = format!("projection-{}-{}", artifact_id, unix_timestamp());

        let _: Option<InstructionProjectionReceiptContent> = self
            .db
            .upsert(("instruction_projection_receipt", receipt_id.as_str()))
            .content(InstructionProjectionReceiptContent {
                receipt_id,
                artifact_id: artifact_id.to_string(),
                base_version: base.version,
                base_hash: base.source_hash.clone(),
                projected_hash: projected_hash.clone(),
                applied_patch_ids: applied_patch_ids.to_vec(),
                skipped_patch_ids,
                failed_reason,
                line_count: split_lines(projected_body).len(),
            })
            .await?;

        Ok(projected_hash)
    }

    #[allow(dead_code)]
    pub async fn resolve_effective_instruction_bundle(
        &self,
        root_artifact_id: &str,
    ) -> Result<EffectiveInstructionBundle, StateStoreError> {
        self.resolve_effective_instruction_bundle_internal(root_artifact_id, true)
            .await
    }

    #[allow(dead_code)]
    pub async fn inspect_effective_instruction_bundle(
        &self,
        root_artifact_id: &str,
    ) -> Result<EffectiveInstructionBundle, StateStoreError> {
        self.resolve_effective_instruction_bundle_internal(root_artifact_id, false)
            .await
    }

    async fn resolve_effective_instruction_bundle_internal(
        &self,
        root_artifact_id: &str,
        persist_receipt: bool,
    ) -> Result<EffectiveInstructionBundle, StateStoreError> {
        let ordered_ids = self.resolve_mandatory_chain(root_artifact_id).await?;
        let mut projected_artifacts = Vec::new();
        let mut source_version_tuple = Vec::new();

        for artifact_id in &ordered_ids {
            let projection = if persist_receipt {
                self.project_instruction_artifact(artifact_id).await?
            } else {
                self.inspect_instruction_artifact(artifact_id).await?
            };
            let base: Option<InstructionArtifactRow> = self
                .db
                .select(("instruction_artifact", artifact_id.as_str()))
                .await?;
            let base = base.ok_or_else(|| StateStoreError::MissingInstructionArtifact {
                artifact_id: artifact_id.clone(),
            })?;

            projected_artifacts.push(EffectiveInstructionArtifact {
                artifact_id: artifact_id.clone(),
                version: base.version,
                source_hash: base.source_hash,
                projected_hash: projection.projected_hash,
                body: projection.body,
            });
            source_version_tuple.push(format!("{}@v{}", artifact_id, base.version));
        }

        let receipt_id = if persist_receipt {
            let receipt_id = format!(
                "effective-bundle-{}-{}",
                root_artifact_id,
                unix_timestamp_nanos()
            );
            let _: Option<EffectiveInstructionBundleReceiptContent> = self
                .db
                .upsert(("effective_instruction_bundle_receipt", receipt_id.as_str()))
                .content(EffectiveInstructionBundleReceiptContent {
                    receipt_id: receipt_id.clone(),
                    root_artifact_id: root_artifact_id.to_string(),
                    mandatory_chain_order: ordered_ids.clone(),
                    source_version_tuple: source_version_tuple.clone(),
                    // Reserved for later trigger-matrix runtime work; B04/B05 keeps this explicit and empty.
                    optional_triggered_reads: Vec::new(),
                    artifact_count: projected_artifacts.len(),
                })
                .await?;
            receipt_id
        } else {
            "not-persisted".to_string()
        };

        Ok(EffectiveInstructionBundle {
            root_artifact_id: root_artifact_id.to_string(),
            mandatory_chain_order: ordered_ids,
            source_version_tuple,
            projected_artifacts,
            receipt_id,
        })
    }

    async fn resolve_mandatory_chain(
        &self,
        root_artifact_id: &str,
    ) -> Result<Vec<String>, StateStoreError> {
        use std::collections::{BTreeMap, BTreeSet};

        let root_exists: Option<InstructionArtifactRow> = self
            .db
            .select(("instruction_artifact", root_artifact_id))
            .await?;
        if root_exists.is_none() {
            return Err(StateStoreError::MissingInstructionArtifact {
                artifact_id: root_artifact_id.to_string(),
            });
        }

        let mut reachable = BTreeSet::new();
        let mut frontier = vec![root_artifact_id.to_string()];

        while let Some(current) = frontier.pop() {
            if !reachable.insert(current.clone()) {
                continue;
            }

            let mut query = self
                .db
                .query(format!(
                    "SELECT to_artifact, edge_kind FROM instruction_dependency_edge WHERE from_artifact = '{}' ORDER BY to_artifact ASC;",
                    current
                ))
                .await?;
            let edges: Vec<InstructionDependencyEdgeRow> = query.take(0)?;
            for edge in edges {
                if edge.edge_kind == "mandatory_follow_on" {
                    let target_exists: Option<InstructionArtifactRow> = self
                        .db
                        .select(("instruction_artifact", edge.to_artifact.as_str()))
                        .await?;
                    if target_exists.is_none() {
                        return Err(StateStoreError::MissingInstructionArtifact {
                            artifact_id: edge.to_artifact,
                        });
                    }
                    frontier.push(edge.to_artifact);
                }
            }
        }

        let mut indegree: BTreeMap<String, usize> =
            reachable.iter().map(|id| (id.clone(), 0usize)).collect();
        let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for from in &reachable {
            let mut query = self
                .db
                .query(format!(
                    "SELECT to_artifact, edge_kind FROM instruction_dependency_edge WHERE from_artifact = '{}' ORDER BY to_artifact ASC;",
                    from
                ))
                .await?;
            let edges: Vec<InstructionDependencyEdgeRow> = query.take(0)?;
            for edge in edges {
                if edge.edge_kind == "mandatory_follow_on" && reachable.contains(&edge.to_artifact)
                {
                    adjacency
                        .entry(from.clone())
                        .or_default()
                        .push(edge.to_artifact.clone());
                    *indegree.entry(edge.to_artifact).or_default() += 1;
                }
            }
        }

        let mut ready: Vec<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();
        ready.sort();

        let mut ordered = Vec::new();
        while let Some(current) = ready.first().cloned() {
            ready.remove(0);
            ordered.push(current.clone());

            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if let Some(degree) = indegree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            ready.push(neighbor.clone());
                        }
                    }
                }
                ready.sort();
            }
        }

        if ordered.len() != reachable.len() {
            let cycle_nodes: Vec<String> = indegree
                .into_iter()
                .filter_map(|(id, degree)| if degree > 0 { Some(id) } else { None })
                .collect();
            return Err(StateStoreError::InstructionDependencyCycle {
                cycle_path: cycle_nodes.join(" -> "),
            });
        }

        Ok(ordered)
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct StorageMetaRow {
    engine: String,
    backend: String,
    namespace: String,
    database: String,
    state_schema_version: u32,
    instruction_schema_version: u32,
}

#[derive(Debug, serde::Deserialize)]
struct TaskJsonlRecord {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    priority: u32,
    #[serde(default)]
    issue_type: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    created_by: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    close_reason: Option<String>,
    #[serde(default)]
    source_repo: String,
    #[serde(default)]
    compaction_level: u32,
    #[serde(default)]
    original_size: u32,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    dependencies: Vec<TaskDependencyJsonlRecord>,
}

#[derive(Debug, serde::Deserialize)]
struct TaskDependencyJsonlRecord {
    issue_id: String,
    depends_on_id: String,
    #[serde(rename = "type")]
    edge_type: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    created_by: String,
    #[serde(default)]
    metadata: String,
    #[serde(default)]
    thread_id: String,
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

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone)]
struct TaskContent {
    task_id: String,
    title: String,
    description: String,
    status: String,
    priority: u32,
    issue_type: String,
    created_at: String,
    created_by: String,
    updated_at: String,
    closed_at: Option<String>,
    close_reason: Option<String>,
    source_repo: String,
    compaction_level: u32,
    original_size: u32,
    notes: Option<String>,
    labels: Vec<String>,
    dependencies: Vec<TaskDependencyRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
struct TaskStorageRow {
    task_id: String,
    title: String,
    description: String,
    status: String,
    priority: u32,
    issue_type: String,
    created_at: String,
    created_by: String,
    updated_at: String,
    closed_at: Option<String>,
    close_reason: Option<String>,
    source_repo: String,
    compaction_level: u32,
    original_size: u32,
    notes: Option<String>,
    labels: Vec<String>,
    dependencies: Vec<TaskDependencyRecord>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SurrealValue, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: u32,
    pub issue_type: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub close_reason: Option<String>,
    pub source_repo: String,
    pub compaction_level: u32,
    pub original_size: u32,
    pub notes: Option<String>,
    pub labels: Vec<String>,
    pub dependencies: Vec<TaskDependencyRecord>,
}

#[derive(Debug)]
pub struct CreateTaskRequest<'a> {
    pub task_id: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub issue_type: &'a str,
    pub status: &'a str,
    pub priority: u32,
    pub parent_id: Option<&'a str>,
    pub labels: &'a [String],
    pub created_by: &'a str,
    pub source_repo: &'a str,
}

#[derive(Debug)]
pub struct UpdateTaskRequest<'a> {
    pub task_id: &'a str,
    pub status: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub description: Option<&'a str>,
    pub add_labels: &'a [String],
    pub remove_labels: &'a [String],
    pub set_labels: Option<&'a [String]>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyStatus {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub dependency_status: String,
    pub dependency_issue_type: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct BlockedTaskRecord {
    pub task: TaskRecord,
    pub blockers: Vec<TaskDependencyStatus>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyTreeNode {
    pub task: TaskRecord,
    pub dependencies: Vec<TaskDependencyTreeEdge>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskDependencyTreeEdge {
    pub issue_id: String,
    pub depends_on_id: String,
    pub edge_type: String,
    pub dependency_status: String,
    pub dependency_issue_type: Option<String>,
    pub node: Option<Box<TaskDependencyTreeNode>>,
    pub cycle: bool,
    pub missing: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskGraphIssue {
    pub issue_type: String,
    pub issue_id: String,
    pub depends_on_id: Option<String>,
    pub edge_type: Option<String>,
    pub detail: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskCriticalPath {
    pub length: usize,
    pub root_task_id: Option<String>,
    pub terminal_task_id: Option<String>,
    pub release_1_contract_steps: Vec<TaskRelease1ContractStep>,
    pub nodes: Vec<TaskCriticalPathNode>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskRelease1ContractStep {
    pub id: String,
    pub mode: String,
    pub blocker_code: String,
    pub next_action: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct TaskCriticalPathNode {
    pub id: String,
    pub title: String,
    pub status: String,
    pub issue_type: String,
    pub priority: u32,
}

#[derive(Debug)]
pub struct TaskImportSummary {
    pub source_path: String,
    pub imported_count: usize,
    pub unchanged_count: usize,
    pub updated_count: usize,
}

impl TaskImportSummary {
    pub fn as_display(&self) -> String {
        format!(
            "{} imported, {} unchanged, {} updated from {}",
            self.imported_count, self.unchanged_count, self.updated_count, self.source_path
        )
    }
}

impl From<TaskJsonlRecord> for TaskContent {
    fn from(value: TaskJsonlRecord) -> Self {
        Self {
            task_id: value.id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            dependencies: value
                .dependencies
                .into_iter()
                .map(TaskDependencyRecord::from)
                .collect(),
        }
    }
}

impl From<TaskContent> for TaskStorageRow {
    fn from(value: TaskContent) -> Self {
        Self {
            task_id: value.task_id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            dependencies: value.dependencies,
        }
    }
}

impl From<TaskStorageRow> for TaskRecord {
    fn from(value: TaskStorageRow) -> Self {
        Self {
            id: value.task_id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            dependencies: value.dependencies,
        }
    }
}

impl From<TaskRecord> for TaskStorageRow {
    fn from(value: TaskRecord) -> Self {
        Self {
            task_id: value.id,
            title: value.title,
            description: value.description,
            status: value.status,
            priority: value.priority,
            issue_type: value.issue_type,
            created_at: value.created_at,
            created_by: value.created_by,
            updated_at: value.updated_at,
            closed_at: value.closed_at,
            close_reason: value.close_reason,
            source_repo: value.source_repo,
            compaction_level: value.compaction_level,
            original_size: value.original_size,
            notes: value.notes,
            labels: value.labels,
            dependencies: value.dependencies,
        }
    }
}

impl From<TaskDependencyJsonlRecord> for TaskDependencyRecord {
    fn from(value: TaskDependencyJsonlRecord) -> Self {
        Self {
            issue_id: value.issue_id,
            depends_on_id: value.depends_on_id,
            edge_type: value.edge_type,
            created_at: value.created_at,
            created_by: value.created_by,
            metadata: value.metadata,
            thread_id: value.thread_id,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct SourceTreeConfigRow {
    slice: String,
    source_root: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct SourceArtifactRow {
    content_hash: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
#[allow(dead_code)]
struct InstructionArtifactRow {
    artifact_id: String,
    version: u32,
    source_hash: String,
    body: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue, Clone)]
#[allow(dead_code)]
struct InstructionDiffPatchRow {
    patch_id: String,
    target_artifact_id: String,
    target_artifact_version: u32,
    target_artifact_hash: String,
    patch_precedence: u32,
    active: bool,
    operations: Vec<InstructionPatchOperation>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct InstructionDependencyEdgeRow {
    to_artifact: String,
    edge_kind: String,
}

#[derive(Debug, serde::Serialize, SurrealValue)]
struct SourceArtifactContent {
    source_artifact_id: String,
    artifact_id: String,
    artifact_kind: String,
    version: u32,
    ownership_class: String,
    mutability_class: String,
    slice: String,
    source_root: String,
    source_path: String,
    content_hash: String,
    ingest_status: String,
    hierarchy: Vec<String>,
}

#[derive(Debug, serde::Serialize, SurrealValue)]
struct InstructionArtifactContent {
    artifact_id: String,
    artifact_kind: String,
    version: u32,
    ownership_class: String,
    mutability_class: String,
    activation_class: String,
    source_hash: String,
    body: String,
    hierarchy: Vec<String>,
    required_follow_on: Vec<String>,
}

#[derive(Debug, serde::Serialize, SurrealValue)]
struct InstructionIngestReceiptContent {
    receipt_id: String,
    slice: String,
    source_root: String,
    product_version: String,
    ingest_kind: String,
    applied: bool,
    imported_count: usize,
    unchanged_count: usize,
    updated_count: usize,
}

#[derive(Debug, serde::Serialize, SurrealValue)]
struct InstructionDependencyEdgeContent {
    from_artifact: String,
    to_artifact: String,
    edge_kind: String,
}

#[derive(Debug, serde::Serialize, SurrealValue, Clone)]
#[allow(dead_code)]
pub struct InstructionDiffPatchContent {
    pub patch_id: String,
    pub target_artifact_id: String,
    pub target_artifact_version: u32,
    pub target_artifact_hash: String,
    pub patch_precedence: u32,
    pub author_class: String,
    pub applies_if: String,
    pub created_at: String,
    pub active: bool,
    pub operations: Vec<InstructionPatchOperation>,
}

#[derive(Debug, serde::Serialize, SurrealValue)]
#[allow(dead_code)]
struct InstructionProjectionReceiptContent {
    receipt_id: String,
    artifact_id: String,
    base_version: u32,
    base_hash: String,
    projected_hash: String,
    applied_patch_ids: Vec<String>,
    skipped_patch_ids: Vec<String>,
    failed_reason: String,
    line_count: usize,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
#[allow(dead_code)]
pub struct InstructionPatchOperation {
    pub op: String,
    pub target_mode: String,
    pub target: String,
    pub with_lines: Vec<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct InstructionProjection {
    pub artifact_id: String,
    pub body: String,
    pub projected_hash: String,
    pub applied_patch_ids: Vec<String>,
    pub skipped_patch_ids: Vec<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct EffectiveInstructionBundle {
    pub root_artifact_id: String,
    pub mandatory_chain_order: Vec<String>,
    pub source_version_tuple: Vec<String>,
    pub projected_artifacts: Vec<EffectiveInstructionArtifact>,
    pub receipt_id: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct EffectiveInstructionArtifact {
    pub artifact_id: String,
    pub version: u32,
    pub source_hash: String,
    pub projected_hash: String,
    pub body: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct EffectiveInstructionBundleReceiptContent {
    receipt_id: String,
    root_artifact_id: String,
    mandatory_chain_order: Vec<String>,
    source_version_tuple: Vec<String>,
    optional_triggered_reads: Vec<String>,
    artifact_count: usize,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct EffectiveBundleReceiptSummary {
    pub receipt_id: String,
    pub root_artifact_id: String,
    pub artifact_count: usize,
}

#[derive(Debug)]
pub struct InstructionIngestSummary {
    source_root: String,
    imported_count: usize,
    unchanged_count: usize,
    updated_count: usize,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct StateSpineManifestContent {
    manifest_id: String,
    state_schema_version: u32,
    authoritative_mutation_root: String,
    entity_surfaces: Vec<String>,
    initialized_at: String,
}

impl StateSpineManifestContent {
    fn from_contract(contract: StateSpineManifestContract, initialized_at: String) -> Self {
        Self {
            manifest_id: contract.manifest_id,
            state_schema_version: contract.state_schema_version,
            authoritative_mutation_root: contract.authoritative_mutation_root,
            entity_surfaces: contract.entity_surfaces,
            initialized_at,
        }
    }
}

#[derive(Debug)]
pub struct StateSpineSummary {
    pub authoritative_mutation_root: String,
    pub entity_surface_count: usize,
    pub state_schema_version: u32,
}

#[allow(dead_code)]
pub struct StorageMetadataSummary {
    pub engine: String,
    pub backend: String,
    pub namespace: String,
    pub database: String,
    pub state_schema_version: u32,
    pub instruction_schema_version: u32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct InstructionRuntimeStateRow {
    state_id: String,
    active_root_artifact_id: String,
    runtime_mode: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct BootCompatibilityStateRow {
    state_id: String,
    classification: String,
    reasons: Vec<String>,
    next_step: String,
    evaluated_at: String,
}

#[derive(Debug)]
pub struct BootCompatibilitySummary {
    pub classification: String,
    pub reasons: Vec<String>,
    pub next_step: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct MigrationRuntimeStateRow {
    state_id: String,
    #[serde(default = "default_release1_contract_type")]
    contract_type: String,
    #[serde(default = "default_release1_schema_version")]
    schema_version: String,
    migration_state: String,
    compatibility_classification: String,
    blockers: Vec<String>,
    source_version_tuple: Vec<String>,
    next_step: String,
    evaluated_at: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct MigrationCompatibilityReceiptRow {
    receipt_id: String,
    #[serde(default = "default_release1_contract_type")]
    contract_type: String,
    #[serde(default = "default_release1_schema_version")]
    schema_version: String,
    compatibility_classification: String,
    migration_state: String,
    blockers: Vec<String>,
    source_version_tuple: Vec<String>,
    next_step: String,
    evaluated_at: String,
}

#[derive(Debug)]
pub struct MigrationPreflightSummary {
    pub contract_type: String,
    pub schema_version: String,
    pub compatibility_classification: String,
    pub migration_state: String,
    pub blockers: Vec<String>,
    pub source_version_tuple: Vec<String>,
    pub next_step: String,
}

fn default_release1_contract_type() -> String {
    Release1ContractType::OperatorContracts.as_str().to_string()
}

fn default_release1_schema_version() -> String {
    Release1SchemaVersion::V1.as_str().to_string()
}

#[derive(Debug)]
pub struct MigrationReceiptSummary {
    pub compatibility_receipts: usize,
    pub application_receipts: usize,
    pub verification_receipts: usize,
    pub cutover_readiness_receipts: usize,
    pub rollback_notes: usize,
}

impl MigrationReceiptSummary {
    pub fn as_display(&self) -> String {
        format!(
            "compatibility={}, application={}, verification={}, cutover={}, rollback={}",
            self.compatibility_receipts,
            self.application_receipts,
            self.verification_receipts,
            self.cutover_readiness_receipts,
            self.rollback_notes
        )
    }
}

#[derive(Debug)]
struct TaskReconciliationSummaryInput {
    operation: String,
    source_kind: String,
    source_path: Option<String>,
    task_count: usize,
    dependency_count: usize,
    stale_removed_count: usize,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct TaskReconciliationSummaryRow {
    receipt_id: String,
    operation: String,
    source_kind: String,
    source_path: Option<String>,
    task_count: usize,
    dependency_count: usize,
    stale_removed_count: usize,
    recorded_at: String,
}

#[derive(Debug)]
pub struct TaskStoreSummary {
    pub total_count: usize,
    pub open_count: usize,
    pub in_progress_count: usize,
    pub closed_count: usize,
    pub epic_count: usize,
    pub ready_count: usize,
}

impl TaskStoreSummary {
    pub fn as_display(&self) -> String {
        format!(
            "{} total, {} open, {} in_progress, {} closed, {} epics, {} ready",
            self.total_count,
            self.open_count,
            self.in_progress_count,
            self.closed_count,
            self.epic_count,
            self.ready_count
        )
    }
}

#[derive(Debug)]
pub struct RunGraphSummary {
    pub execution_plan_count: usize,
    pub routed_run_count: usize,
    pub governance_count: usize,
    pub resumability_count: usize,
    pub reconciliation_count: usize,
}

impl RunGraphSummary {
    pub fn as_display(&self) -> String {
        format!(
            "execution_plans={}, routed_runs={}, governance={}, resumability={}, reconciliation={}",
            self.execution_plan_count,
            self.routed_run_count,
            self.governance_count,
            self.resumability_count,
            self.reconciliation_count
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct ExecutionPlanStateRow {
    run_id: String,
    task_id: String,
    task_class: String,
    active_node: String,
    next_node: Option<String>,
    status: String,
    updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct RoutedRunStateRow {
    run_id: String,
    route_task_class: String,
    selected_backend: String,
    lane_id: String,
    lifecycle_stage: String,
    updated_at: String,
}

#[derive(Debug, serde::Deserialize, SurrealValue)]
struct RunGraphLatestRow {
    run_id: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct GovernanceStateRow {
    run_id: String,
    policy_gate: String,
    handoff_state: String,
    context_state: String,
    updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
struct ResumabilityCapsuleRow {
    run_id: String,
    checkpoint_kind: String,
    resume_target: String,
    recovery_ready: bool,
    updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct RunGraphDispatchReceipt {
    pub run_id: String,
    pub dispatch_target: String,
    pub dispatch_status: String,
    #[serde(
        default = "default_run_graph_lane_status",
        deserialize_with = "deserialize_run_graph_lane_status"
    )]
    pub lane_status: String,
    pub supersedes_receipt_id: Option<String>,
    pub exception_path_receipt_id: Option<String>,
    pub dispatch_kind: String,
    pub dispatch_surface: Option<String>,
    pub dispatch_command: Option<String>,
    pub dispatch_packet_path: Option<String>,
    pub dispatch_result_path: Option<String>,
    pub blocker_code: Option<String>,
    pub downstream_dispatch_target: Option<String>,
    pub downstream_dispatch_command: Option<String>,
    pub downstream_dispatch_note: Option<String>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_blockers: Vec<String>,
    pub downstream_dispatch_packet_path: Option<String>,
    pub downstream_dispatch_status: Option<String>,
    pub downstream_dispatch_result_path: Option<String>,
    pub downstream_dispatch_trace_path: Option<String>,
    pub downstream_dispatch_executed_count: u32,
    pub downstream_dispatch_active_target: Option<String>,
    pub downstream_dispatch_last_target: Option<String>,
    pub activation_agent_type: Option<String>,
    pub activation_runtime_role: Option<String>,
    pub selected_backend: Option<String>,
    pub recorded_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
struct RunGraphDispatchReceiptStored {
    run_id: String,
    dispatch_target: String,
    dispatch_status: String,
    lane_status: Option<String>,
    supersedes_receipt_id: Option<String>,
    exception_path_receipt_id: Option<String>,
    dispatch_kind: String,
    dispatch_surface: Option<String>,
    dispatch_command: Option<String>,
    dispatch_packet_path: Option<String>,
    dispatch_result_path: Option<String>,
    blocker_code: Option<String>,
    downstream_dispatch_target: Option<String>,
    downstream_dispatch_command: Option<String>,
    downstream_dispatch_note: Option<String>,
    downstream_dispatch_ready: bool,
    downstream_dispatch_blockers: Vec<String>,
    downstream_dispatch_packet_path: Option<String>,
    downstream_dispatch_status: Option<String>,
    downstream_dispatch_result_path: Option<String>,
    downstream_dispatch_trace_path: Option<String>,
    downstream_dispatch_executed_count: u32,
    downstream_dispatch_active_target: Option<String>,
    downstream_dispatch_last_target: Option<String>,
    activation_agent_type: Option<String>,
    activation_runtime_role: Option<String>,
    selected_backend: Option<String>,
    recorded_at: String,
}

impl From<RunGraphDispatchReceiptStored> for RunGraphDispatchReceipt {
    fn from(stored: RunGraphDispatchReceiptStored) -> Self {
        let normalized_lane_status = normalize_run_graph_lane_status(
            stored.lane_status.as_deref(),
            &stored.dispatch_status,
            stored.supersedes_receipt_id.as_deref(),
            stored.exception_path_receipt_id.as_deref(),
        );
        Self {
            run_id: stored.run_id,
            dispatch_target: stored.dispatch_target,
            dispatch_status: stored.dispatch_status,
            lane_status: normalized_lane_status,
            supersedes_receipt_id: stored.supersedes_receipt_id,
            exception_path_receipt_id: stored.exception_path_receipt_id,
            dispatch_kind: stored.dispatch_kind,
            dispatch_surface: stored.dispatch_surface,
            dispatch_command: stored.dispatch_command,
            dispatch_packet_path: stored.dispatch_packet_path,
            dispatch_result_path: stored.dispatch_result_path,
            blocker_code: stored.blocker_code,
            downstream_dispatch_target: stored.downstream_dispatch_target,
            downstream_dispatch_command: stored.downstream_dispatch_command,
            downstream_dispatch_note: stored.downstream_dispatch_note,
            downstream_dispatch_ready: stored.downstream_dispatch_ready,
            downstream_dispatch_blockers: stored.downstream_dispatch_blockers,
            downstream_dispatch_packet_path: stored.downstream_dispatch_packet_path,
            downstream_dispatch_status: stored.downstream_dispatch_status,
            downstream_dispatch_result_path: stored.downstream_dispatch_result_path,
            downstream_dispatch_trace_path: stored.downstream_dispatch_trace_path,
            downstream_dispatch_executed_count: stored.downstream_dispatch_executed_count,
            downstream_dispatch_active_target: stored.downstream_dispatch_active_target,
            downstream_dispatch_last_target: stored.downstream_dispatch_last_target,
            activation_agent_type: stored.activation_agent_type,
            activation_runtime_role: stored.activation_runtime_role,
            selected_backend: stored.selected_backend,
            recorded_at: stored.recorded_at,
        }
    }
}

impl From<RunGraphDispatchReceipt> for RunGraphDispatchReceiptStored {
    fn from(receipt: RunGraphDispatchReceipt) -> Self {
        let lane_status = if receipt.lane_status.is_empty() {
            None
        } else {
            Some(receipt.lane_status)
        };
        Self {
            run_id: receipt.run_id,
            dispatch_target: receipt.dispatch_target,
            dispatch_status: receipt.dispatch_status,
            lane_status,
            supersedes_receipt_id: receipt.supersedes_receipt_id,
            exception_path_receipt_id: receipt.exception_path_receipt_id,
            dispatch_kind: receipt.dispatch_kind,
            dispatch_surface: receipt.dispatch_surface,
            dispatch_command: receipt.dispatch_command,
            dispatch_packet_path: receipt.dispatch_packet_path,
            dispatch_result_path: receipt.dispatch_result_path,
            blocker_code: receipt.blocker_code,
            downstream_dispatch_target: receipt.downstream_dispatch_target,
            downstream_dispatch_command: receipt.downstream_dispatch_command,
            downstream_dispatch_note: receipt.downstream_dispatch_note,
            downstream_dispatch_ready: receipt.downstream_dispatch_ready,
            downstream_dispatch_blockers: receipt.downstream_dispatch_blockers,
            downstream_dispatch_packet_path: receipt.downstream_dispatch_packet_path,
            downstream_dispatch_status: receipt.downstream_dispatch_status,
            downstream_dispatch_result_path: receipt.downstream_dispatch_result_path,
            downstream_dispatch_trace_path: receipt.downstream_dispatch_trace_path,
            downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
            downstream_dispatch_active_target: receipt.downstream_dispatch_active_target,
            downstream_dispatch_last_target: receipt.downstream_dispatch_last_target,
            activation_agent_type: receipt.activation_agent_type,
            activation_runtime_role: receipt.activation_runtime_role,
            selected_backend: receipt.selected_backend,
            recorded_at: receipt.recorded_at,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, serde::Serialize)]
pub struct RunGraphStatus {
    pub run_id: String,
    pub task_id: String,
    pub task_class: String,
    pub active_node: String,
    pub next_node: Option<String>,
    pub status: String,
    pub route_task_class: String,
    pub selected_backend: String,
    pub lane_id: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub context_state: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

#[allow(dead_code)]
impl RunGraphStatus {
    fn validate_memory_governance(&self) -> Result<(), StateStoreError> {
        if !requires_memory_governance_enforcement(&self.policy_gate) {
            return Ok(());
        }
        if self.context_state != "sealed" {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "memory governance evidence shaping required for policy_gate `{}`: context_state must be `sealed`, got `{}`",
                    self.policy_gate, self.context_state
                ),
            });
        }
        if !handoff_state_links_consent_ttl(&self.handoff_state) {
            return Err(StateStoreError::InvalidTaskRecord {
                reason: format!(
                    "memory governance linkage required for policy_gate `{}`: handoff_state must link consent+ttl, got `{}`",
                    self.policy_gate, self.handoff_state
                ),
            });
        }
        Ok(())
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} class={} node={} status={} next={} route={} backend={} lane={} lifecycle={} gate={} handoff={} context={} checkpoint={} resume_target={} recovery_ready={}",
            self.run_id,
            self.task_id,
            self.task_class,
            self.active_node,
            self.status,
            self.next_node.as_deref().unwrap_or("none"),
            self.route_task_class,
            self.selected_backend,
            self.lane_id,
            self.lifecycle_stage,
            self.policy_gate,
            self.handoff_state,
            self.context_state,
            self.checkpoint_kind,
            self.resume_target,
            self.recovery_ready
        )
    }

    pub fn delegation_gate(&self) -> RunGraphDelegationGateSummary {
        RunGraphDelegationGateSummary::from_status(self)
    }
}

fn requires_memory_governance_enforcement(policy_gate: &str) -> bool {
    let normalized = policy_gate.trim().to_ascii_lowercase();
    normalized.contains("consent")
        || normalized.contains("ttl")
        || normalized.contains("correction")
        || normalized.contains("delete")
        || normalized.contains("deletion")
}

fn handoff_state_links_consent_ttl(handoff_state: &str) -> bool {
    let normalized = handoff_state.trim().to_ascii_lowercase();
    normalized.contains("consent") && normalized.contains("ttl")
}

#[derive(Debug, serde::Serialize, PartialEq, Eq, Clone)]
pub struct RunGraphDelegationGateSummary {
    pub active_node: String,
    pub lifecycle_stage: String,
    pub delegated_cycle_open: bool,
    pub delegated_cycle_state: String,
    pub local_exception_takeover_gate: String,
    pub blocker_code: Option<String>,
    pub reporting_pause_gate: String,
    pub continuation_signal: String,
}

impl RunGraphDelegationGateSummary {
    fn from_status(status: &RunGraphStatus) -> Self {
        let handoff_pending = status.next_node.is_some()
            || status.handoff_state != "none"
            || status.resume_target != "none";
        let delegated_lane_active = !handoff_pending
            && status.status != "completed"
            && status.active_node != "planning"
            && status.lifecycle_stage.ends_with("_active");
        let (delegated_cycle_open, delegated_cycle_state) = if handoff_pending {
            (true, "handoff_pending".to_string())
        } else if delegated_lane_active {
            (true, "delegated_lane_active".to_string())
        } else {
            (false, "clear".to_string())
        };
        let local_exception_takeover_gate = if delegated_cycle_open {
            "blocked_open_delegated_cycle".to_string()
        } else {
            "delegated_cycle_clear".to_string()
        };
        let blocker_code = if local_exception_takeover_gate == "blocked_open_delegated_cycle" {
            Some(
                canonical_blocker_code_str(BlockerCode::OpenDelegatedCycle.as_str())
                    .unwrap_or(BlockerCode::OpenDelegatedCycle.as_str())
                    .to_string(),
            )
        } else {
            None
        };
        let reporting_pause_gate = if delegated_cycle_open {
            "non_blocking_only".to_string()
        } else if status.status == "completed" {
            "closure_candidate".to_string()
        } else {
            "continuation_check_required".to_string()
        };
        let continuation_signal = if delegated_cycle_open {
            "continue_routing_non_blocking".to_string()
        } else if status.status == "completed" {
            "continue_after_reports".to_string()
        } else {
            "continuation_check_required".to_string()
        };

        Self {
            active_node: status.active_node.clone(),
            lifecycle_stage: status.lifecycle_stage.clone(),
            delegated_cycle_open,
            delegated_cycle_state,
            local_exception_takeover_gate,
            blocker_code,
            reporting_pause_gate,
            continuation_signal,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "node={} lifecycle={} delegated_cycle_open={} delegated_cycle_state={} local_exception_takeover_gate={} blocker_code={} reporting_pause_gate={} continuation_signal={}",
            self.active_node,
            self.lifecycle_stage,
            self.delegated_cycle_open,
            self.delegated_cycle_state,
            self.local_exception_takeover_gate,
            self.blocker_code.as_deref().unwrap_or("none"),
            self.reporting_pause_gate,
            self.continuation_signal
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphRecoverySummary {
    pub run_id: String,
    pub task_id: String,
    pub active_node: String,
    pub lifecycle_stage: String,
    pub resume_node: Option<String>,
    pub resume_status: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub recovery_ready: bool,
    pub delegation_gate: RunGraphDelegationGateSummary,
}

impl RunGraphRecoverySummary {
    fn from_status(status: RunGraphStatus) -> Self {
        let delegation_gate = status.delegation_gate();
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            active_node: status.active_node,
            lifecycle_stage: status.lifecycle_stage,
            resume_node: status.next_node,
            resume_status: status.status,
            checkpoint_kind: status.checkpoint_kind,
            resume_target: status.resume_target,
            policy_gate: status.policy_gate,
            handoff_state: status.handoff_state,
            recovery_ready: status.recovery_ready,
            delegation_gate,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} active_node={} lifecycle={} resume_node={} resume_status={} checkpoint={} resume_target={} gate={} handoff={} recovery_ready={} takeover_gate={} report_pause_gate={} continuation_signal={}",
            self.run_id,
            self.task_id,
            self.active_node,
            self.lifecycle_stage,
            self.resume_node.as_deref().unwrap_or("none"),
            self.resume_status,
            self.checkpoint_kind,
            self.resume_target,
            self.policy_gate,
            self.handoff_state,
            self.recovery_ready,
            self.delegation_gate.local_exception_takeover_gate,
            self.delegation_gate.reporting_pause_gate,
            self.delegation_gate.continuation_signal
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphCheckpointSummary {
    pub run_id: String,
    pub task_id: String,
    pub checkpoint_kind: String,
    pub resume_target: String,
    pub recovery_ready: bool,
}

impl RunGraphCheckpointSummary {
    fn from_status(status: RunGraphStatus) -> Self {
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            checkpoint_kind: status.checkpoint_kind,
            resume_target: status.resume_target,
            recovery_ready: status.recovery_ready,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} checkpoint={} resume_target={} recovery_ready={}",
            self.run_id,
            self.task_id,
            self.checkpoint_kind,
            self.resume_target,
            self.recovery_ready
        )
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphDispatchReceiptSummary {
    pub run_id: String,
    pub dispatch_target: String,
    pub dispatch_status: String,
    pub lane_status: String,
    pub supersedes_receipt_id: Option<String>,
    pub exception_path_receipt_id: Option<String>,
    pub dispatch_kind: String,
    pub dispatch_surface: Option<String>,
    pub dispatch_command: Option<String>,
    pub dispatch_packet_path: Option<String>,
    pub dispatch_result_path: Option<String>,
    pub blocker_code: Option<String>,
    pub downstream_dispatch_target: Option<String>,
    pub downstream_dispatch_command: Option<String>,
    pub downstream_dispatch_note: Option<String>,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_blockers: Vec<String>,
    pub downstream_dispatch_packet_path: Option<String>,
    pub downstream_dispatch_status: Option<String>,
    pub downstream_dispatch_result_path: Option<String>,
    pub downstream_dispatch_trace_path: Option<String>,
    pub downstream_dispatch_executed_count: u32,
    pub downstream_dispatch_active_target: Option<String>,
    pub downstream_dispatch_last_target: Option<String>,
    pub activation_agent_type: Option<String>,
    pub activation_runtime_role: Option<String>,
    pub selected_backend: Option<String>,
    pub recorded_at: String,
}

#[allow(dead_code)]
impl RunGraphDispatchReceiptSummary {
    fn from_receipt(receipt: RunGraphDispatchReceipt) -> Self {
        let lane_status = if receipt.lane_status.trim().is_empty() {
            derive_lane_status(
                &receipt.dispatch_status,
                receipt.supersedes_receipt_id.as_deref(),
                receipt.exception_path_receipt_id.as_deref(),
            )
            .as_str()
            .to_string()
        } else {
            canonical_lane_status_str(&receipt.lane_status)
                .unwrap_or(receipt.lane_status.as_str())
                .to_string()
        };
        let blocker_code = receipt
            .blocker_code
            .as_deref()
            .and_then(canonical_blocker_code_str)
            .map(str::to_string)
            .or(receipt.blocker_code.clone());
        let mut downstream_dispatch_blockers = receipt.downstream_dispatch_blockers;
        downstream_dispatch_blockers.sort_unstable();
        Self {
            run_id: receipt.run_id,
            dispatch_target: receipt.dispatch_target,
            dispatch_status: receipt.dispatch_status,
            lane_status,
            supersedes_receipt_id: receipt.supersedes_receipt_id,
            exception_path_receipt_id: receipt.exception_path_receipt_id,
            dispatch_kind: receipt.dispatch_kind,
            dispatch_surface: receipt.dispatch_surface,
            dispatch_command: receipt.dispatch_command,
            dispatch_packet_path: receipt.dispatch_packet_path,
            dispatch_result_path: receipt.dispatch_result_path,
            blocker_code,
            downstream_dispatch_target: receipt.downstream_dispatch_target,
            downstream_dispatch_command: receipt.downstream_dispatch_command,
            downstream_dispatch_note: receipt.downstream_dispatch_note,
            downstream_dispatch_ready: receipt.downstream_dispatch_ready,
            downstream_dispatch_blockers,
            downstream_dispatch_packet_path: receipt.downstream_dispatch_packet_path,
            downstream_dispatch_status: receipt.downstream_dispatch_status,
            downstream_dispatch_result_path: receipt.downstream_dispatch_result_path,
            downstream_dispatch_trace_path: receipt.downstream_dispatch_trace_path,
            downstream_dispatch_executed_count: receipt.downstream_dispatch_executed_count,
            downstream_dispatch_active_target: receipt.downstream_dispatch_active_target,
            downstream_dispatch_last_target: receipt.downstream_dispatch_last_target,
            activation_agent_type: receipt.activation_agent_type,
            activation_runtime_role: receipt.activation_runtime_role,
            selected_backend: receipt.selected_backend,
            recorded_at: receipt.recorded_at,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} target={} status={} lane_status={} supersedes_receipt_id={} exception_path_receipt_id={} blocker_code={} kind={} surface={} command={} packet={} result={} next_target={} next_command={} next_note={} next_ready={} next_blockers={} next_packet={} next_status={} next_result={} next_trace={} next_count={} next_last_target={} agent={} runtime_role={} backend={} recorded_at={}",
            self.run_id,
            self.dispatch_target,
            self.dispatch_status,
            self.lane_status,
            self.supersedes_receipt_id.as_deref().unwrap_or("none"),
            self.exception_path_receipt_id.as_deref().unwrap_or("none"),
            self.blocker_code.as_deref().unwrap_or("none"),
            self.dispatch_kind,
            self.dispatch_surface.as_deref().unwrap_or("none"),
            self.dispatch_command.as_deref().unwrap_or("none"),
            self.dispatch_packet_path.as_deref().unwrap_or("none"),
            self.dispatch_result_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_target.as_deref().unwrap_or("none"),
            self.downstream_dispatch_command.as_deref().unwrap_or("none"),
            self.downstream_dispatch_note.as_deref().unwrap_or("none"),
            self.downstream_dispatch_ready,
            if self.downstream_dispatch_blockers.is_empty() {
                "none".to_string()
            } else {
                self.downstream_dispatch_blockers.join("|")
            },
            self.downstream_dispatch_packet_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_status.as_deref().unwrap_or("none"),
            self.downstream_dispatch_result_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_trace_path.as_deref().unwrap_or("none"),
            self.downstream_dispatch_executed_count,
            self.downstream_dispatch_last_target.as_deref().unwrap_or("none"),
            self.activation_agent_type.as_deref().unwrap_or("none"),
            self.activation_runtime_role.as_deref().unwrap_or("none"),
            self.selected_backend.as_deref().unwrap_or("none"),
            self.recorded_at
        )
    }
}

pub(crate) fn latest_run_graph_dispatch_receipt_matches_status(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    matches!(
        (
            latest_run_graph_status_run_id,
            latest_run_graph_dispatch_receipt_run_id
        ),
        (Some(status_run_id), Some(receipt_run_id)) if status_run_id == receipt_run_id
    )
}

pub(crate) fn latest_run_graph_dispatch_receipt_summary_is_inconsistent(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    latest_run_graph_status_run_id.is_some()
        && !latest_run_graph_dispatch_receipt_matches_status(
            latest_run_graph_status_run_id,
            latest_run_graph_dispatch_receipt_run_id,
        )
}

pub(crate) fn latest_run_graph_dispatch_receipt_signal_is_ambiguous(
    receipt: &RunGraphDispatchReceiptSummary,
) -> bool {
    matches!(
        receipt.dispatch_status.as_str(),
        "packet_ready" | "routed" | "executed" | "blocked"
    ) && receipt.lane_status.as_str()
        != derive_lane_status(
            &receipt.dispatch_status,
            receipt.supersedes_receipt_id.as_deref(),
            receipt.exception_path_receipt_id.as_deref(),
        )
        .as_str()
        || !matches!(
            receipt.dispatch_status.as_str(),
            "packet_ready" | "routed" | "executed" | "blocked"
        )
}

pub(crate) fn latest_run_graph_evidence_snapshot_is_consistent(
    latest_run_graph_status_run_id: Option<&str>,
    latest_run_graph_recovery_run_id: Option<&str>,
    latest_run_graph_checkpoint_run_id: Option<&str>,
    latest_run_graph_gate_run_id: Option<&str>,
    latest_run_graph_dispatch_receipt_run_id: Option<&str>,
) -> bool {
    let Some(latest_run_graph_status_run_id) = latest_run_graph_status_run_id else {
        return latest_run_graph_recovery_run_id.is_none()
            && latest_run_graph_checkpoint_run_id.is_none()
            && latest_run_graph_gate_run_id.is_none()
            && latest_run_graph_dispatch_receipt_run_id.is_none();
    };
    [
        latest_run_graph_recovery_run_id,
        latest_run_graph_checkpoint_run_id,
        latest_run_graph_gate_run_id,
        latest_run_graph_dispatch_receipt_run_id,
    ]
    .into_iter()
    .flatten()
    .all(|run_id| run_id == latest_run_graph_status_run_id)
}

fn default_run_graph_lane_status() -> String {
    LaneStatus::LaneOpen.as_str().to_string()
}

fn normalize_run_graph_lane_status(
    value: Option<&str>,
    dispatch_status: &str,
    supersedes_receipt_id: Option<&str>,
    exception_path_receipt_id: Option<&str>,
) -> String {
    let derived_lane_status = derive_lane_status(
        dispatch_status,
        supersedes_receipt_id,
        exception_path_receipt_id,
    )
    .as_str()
    .to_string();
    match value {
        Some(raw) if !raw.trim().is_empty() => {
            let canonical_lane_status = canonical_lane_status_str(raw).unwrap_or(raw);
            if canonical_lane_status == derived_lane_status {
                canonical_lane_status.to_string()
            } else {
                derived_lane_status
            }
        }
        _ => derived_lane_status,
    }
}

fn deserialize_run_graph_lane_status<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <Option<String> as serde::Deserialize>::deserialize(deserializer)?;
    match value.as_deref() {
        Some(raw) if !raw.trim().is_empty() => {
            Ok(canonical_lane_status_str(raw).unwrap_or(raw).to_string())
        }
        _ => Ok(default_run_graph_lane_status()),
    }
}

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct RunGraphGateSummary {
    pub run_id: String,
    pub task_id: String,
    pub active_node: String,
    pub lifecycle_stage: String,
    pub policy_gate: String,
    pub handoff_state: String,
    pub context_state: String,
    pub delegation_gate: RunGraphDelegationGateSummary,
}

impl RunGraphGateSummary {
    fn from_status(status: RunGraphStatus) -> Self {
        let delegation_gate = status.delegation_gate();
        Self {
            run_id: status.run_id,
            task_id: status.task_id,
            active_node: status.active_node,
            lifecycle_stage: status.lifecycle_stage,
            policy_gate: status.policy_gate,
            handoff_state: status.handoff_state,
            context_state: status.context_state,
            delegation_gate,
        }
    }

    pub fn as_display(&self) -> String {
        format!(
            "run={} task={} active_node={} lifecycle={} gate={} handoff={} context={} takeover_gate={} report_pause_gate={} continuation_signal={}",
            self.run_id,
            self.task_id,
            self.active_node,
            self.lifecycle_stage,
            self.policy_gate,
            self.handoff_state,
            self.context_state,
            self.delegation_gate.local_exception_takeover_gate,
            self.delegation_gate.reporting_pause_gate,
            self.delegation_gate.continuation_signal
        )
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct TaskReconciliationSummary {
    pub receipt_id: String,
    pub operation: String,
    pub source_kind: String,
    pub source_path: Option<String>,
    pub task_count: usize,
    pub dependency_count: usize,
    pub stale_removed_count: usize,
    pub recorded_at: String,
}

impl TaskReconciliationSummary {
    pub fn as_display(&self) -> String {
        let source_path = self.source_path.as_deref().unwrap_or("none");
        format!(
            "{} via {} (tasks={}, dependencies={}, stale_removed={}, source_path={})",
            self.operation,
            self.source_kind,
            self.task_count,
            self.dependency_count,
            self.stale_removed_count,
            source_path
        )
    }
}

#[derive(Debug, serde::Deserialize, SurrealValue)]
struct TaskReconciliationRollupRow {
    operation: String,
    source_kind: String,
    source_path: Option<String>,
    task_count: usize,
    dependency_count: usize,
    stale_removed_count: usize,
    recorded_at: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TaskReconciliationRollup {
    pub total_receipts: usize,
    pub latest_recorded_at: Option<String>,
    pub latest_source_path: Option<String>,
    pub total_task_rows: usize,
    pub total_dependency_rows: usize,
    pub total_stale_removed: usize,
    pub by_operation: BTreeMap<String, usize>,
    pub by_source_kind: BTreeMap<String, usize>,
    #[serde(skip)]
    rows: Vec<TaskReconciliationRollupRow>,
}

impl TaskReconciliationRollup {
    pub fn as_display(&self) -> String {
        if self.total_receipts == 0 {
            return "0 receipts".to_string();
        }

        let operations = self
            .by_operation
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        let source_kinds = self
            .by_source_kind
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        let latest_recorded_at = self.latest_recorded_at.as_deref().unwrap_or("none");
        let latest_source_path = self.latest_source_path.as_deref().unwrap_or("none");

        format!(
            "{} receipts (tasks={}, dependencies={}, stale_removed={}, operations: {}; source_kinds: {}; latest_recorded_at={}; latest_source_path={})",
            self.total_receipts,
            self.total_task_rows,
            self.total_dependency_rows,
            self.total_stale_removed,
            operations,
            source_kinds,
            latest_recorded_at,
            latest_source_path
        )
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TaskflowSnapshotBridgeSummary {
    pub total_receipts: usize,
    pub export_receipts: usize,
    pub import_receipts: usize,
    pub replace_receipts: usize,
    pub object_export_receipts: usize,
    pub memory_export_receipts: usize,
    pub memory_import_receipts: usize,
    pub memory_replace_receipts: usize,
    pub file_export_receipts: usize,
    pub file_import_receipts: usize,
    pub file_replace_receipts: usize,
    pub total_task_rows: usize,
    pub total_dependency_rows: usize,
    pub total_stale_removed: usize,
    pub latest_operation: Option<String>,
    pub latest_source_kind: Option<String>,
    pub latest_source_path: Option<String>,
    pub latest_recorded_at: Option<String>,
}

impl TaskflowSnapshotBridgeSummary {
    pub fn as_display(&self) -> String {
        if self.total_receipts == 0 {
            return "idle (no snapshot bridge receipts)".to_string();
        }

        format!(
            "receipts={} export={} import={} replace={} object={} memory={} file={} tasks={} dependencies={} stale_removed={} latest={} via {} source_path={}",
            self.total_receipts,
            self.export_receipts,
            self.import_receipts,
            self.replace_receipts,
            self.object_export_receipts,
            self.memory_export_receipts
                + self.memory_import_receipts
                + self.memory_replace_receipts,
            self.file_export_receipts + self.file_import_receipts + self.file_replace_receipts,
            self.total_task_rows,
            self.total_dependency_rows,
            self.total_stale_removed,
            self.latest_operation.as_deref().unwrap_or("none"),
            self.latest_source_kind.as_deref().unwrap_or("none"),
            self.latest_source_path.as_deref().unwrap_or("none"),
        )
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
struct ProtocolBindingStateRow {
    protocol_id: String,
    source_path: String,
    activation_class: String,
    runtime_owner: String,
    enforcement_type: String,
    proof_surface: String,
    primary_state_authority: String,
    binding_status: String,
    active: bool,
    blockers: Vec<String>,
    scenario: String,
    synced_at: String,
}

impl ProtocolBindingStateRow {
    fn from_state(
        scenario: &str,
        primary_state_authority: &str,
        synced_at: String,
        state: ProtocolBindingState,
    ) -> Self {
        Self {
            protocol_id: state.protocol_id,
            source_path: state.source_path,
            activation_class: state.activation_class,
            runtime_owner: state.runtime_owner,
            enforcement_type: state.enforcement_type,
            proof_surface: state.proof_surface,
            primary_state_authority: if state.primary_state_authority.trim().is_empty() {
                primary_state_authority.to_string()
            } else {
                state.primary_state_authority
            },
            binding_status: state.binding_status,
            active: state.active,
            blockers: state.blockers,
            scenario: scenario.to_string(),
            synced_at,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProtocolBindingState {
    pub protocol_id: String,
    pub source_path: String,
    pub activation_class: String,
    pub runtime_owner: String,
    pub enforcement_type: String,
    pub proof_surface: String,
    pub primary_state_authority: String,
    pub binding_status: String,
    pub active: bool,
    pub blockers: Vec<String>,
    pub scenario: String,
    pub synced_at: String,
}

impl ProtocolBindingState {
    fn from_row(row: ProtocolBindingStateRow) -> Self {
        Self {
            protocol_id: row.protocol_id,
            source_path: row.source_path,
            activation_class: row.activation_class,
            runtime_owner: row.runtime_owner,
            enforcement_type: row.enforcement_type,
            proof_surface: row.proof_surface,
            primary_state_authority: row.primary_state_authority,
            binding_status: row.binding_status,
            active: row.active,
            blockers: row.blockers,
            scenario: row.scenario,
            synced_at: row.synced_at,
        }
    }

    pub fn as_display(&self) -> String {
        let blockers = if self.blockers.is_empty() {
            "none".to_string()
        } else {
            self.blockers.join(",")
        };
        format!(
            "{} status={} active={} activation={} enforcement={} owner={} blockers={}",
            self.protocol_id,
            self.binding_status,
            self.active,
            self.activation_class,
            self.enforcement_type,
            self.runtime_owner,
            blockers
        )
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct ProtocolBindingReceipt {
    pub receipt_id: String,
    pub scenario: String,
    pub total_bindings: usize,
    pub active_bindings: usize,
    pub script_bound_count: usize,
    pub rust_bound_count: usize,
    pub fully_runtime_bound_count: usize,
    pub unbound_count: usize,
    pub blocking_issue_count: usize,
    pub primary_state_authority: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProtocolBindingSummary {
    pub total_receipts: usize,
    pub total_bindings: usize,
    pub active_bindings: usize,
    pub script_bound_count: usize,
    pub rust_bound_count: usize,
    pub fully_runtime_bound_count: usize,
    pub unbound_count: usize,
    pub blocking_issue_count: usize,
    pub latest_receipt_id: Option<String>,
    pub latest_scenario: Option<String>,
    pub latest_recorded_at: Option<String>,
    pub primary_state_authority: Option<String>,
}

impl ProtocolBindingSummary {
    pub fn as_display(&self) -> String {
        if self.total_receipts == 0 {
            return "idle (no protocol-binding receipts)".to_string();
        }
        format!(
            "receipts={} bindings={} active={} script_bound={} rust_bound={} fully_runtime_bound={} unbound={} blocking_issues={} latest_scenario={} authority={}",
            self.total_receipts,
            self.total_bindings,
            self.active_bindings,
            self.script_bound_count,
            self.rust_bound_count,
            self.fully_runtime_bound_count,
            self.unbound_count,
            self.blocking_issue_count,
            self.latest_scenario.as_deref().unwrap_or("none"),
            self.primary_state_authority.as_deref().unwrap_or("none"),
        )
    }
}

#[derive(Debug, serde::Deserialize, SurrealValue)]
struct CountRow {
    total: usize,
}

fn count_snapshot_bridge_rows(
    rows: &[TaskReconciliationRollupRow],
    operation: Option<&str>,
    source_kind: Option<&str>,
) -> usize {
    rows.iter()
        .filter(|row| {
            operation
                .map(|expected| row.operation == expected)
                .unwrap_or(true)
        })
        .filter(|row| {
            source_kind
                .map(|expected| row.source_kind == expected)
                .unwrap_or(true)
        })
        .count()
}

impl InstructionIngestSummary {
    pub fn as_display(&self) -> String {
        format!(
            "{} imported, {} unchanged, {} updated from {}",
            self.imported_count, self.unchanged_count, self.updated_count, self.source_root
        )
    }
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
            Self::Db(error) => write!(f, "{error}"),
            Self::MissingStateDir(path) => {
                write!(
                    f,
                    "authoritative state directory is missing: {}",
                    path.display()
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
                write!(f, "authoritative state spine manifest is missing")
            }
            Self::InvalidStateSpineManifest { reason } => {
                write!(f, "authoritative state spine manifest is invalid: {reason}")
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub struct LauncherActivationSnapshot {
    pub source: String,
    pub source_config_path: String,
    pub source_config_digest: String,
    pub captured_at: String,
    pub compiled_bundle: serde_json::Value,
    pub pack_router_keywords: serde_json::Value,
}

impl LauncherActivationSnapshot {
    fn validate(&self) -> Result<(), StateStoreError> {
        if self.source != "state_store" {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: format!("unsupported source `{}`", self.source),
            });
        }
        if self.source_config_path.trim().is_empty() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "source_config_path is empty".to_string(),
            });
        }
        if self.source_config_digest.trim().is_empty() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "source_config_digest is empty".to_string(),
            });
        }
        if !self.compiled_bundle.is_object() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "compiled_bundle must be an object".to_string(),
            });
        }
        if !self.pack_router_keywords.is_object() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "pack_router_keywords must be an object".to_string(),
            });
        }
        let fallback_role = self.compiled_bundle["role_selection"]["fallback_role"]
            .as_str()
            .unwrap_or_default();
        if fallback_role.is_empty() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "role_selection.fallback_role is empty".to_string(),
            });
        }
        let selection_mode = self.compiled_bundle["role_selection"]["mode"]
            .as_str()
            .unwrap_or_default();
        if selection_mode.is_empty() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "role_selection.mode is empty".to_string(),
            });
        }
        if !self.compiled_bundle["agent_system"].is_object() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "compiled_bundle.agent_system must be an object".to_string(),
            });
        }
        Ok(())
    }
}

fn collect_markdown_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_markdown_files_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_markdown_files_inner(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files_inner(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    Ok(())
}

fn artifact_id_from_path(relative: &Path) -> String {
    relative
        .with_extension("")
        .to_string_lossy()
        .replace(['/', '\\'], "-")
}

fn escape_surql_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn sanitize_record_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[allow(dead_code)]
fn parse_canonical_timestamp(value: &str) -> Result<CanonicalTimestamp, StateStoreError> {
    if let Ok(parsed) = OffsetDateTime::parse(value, &Rfc3339) {
        return Ok(CanonicalTimestamp(parsed));
    }

    let nanos =
        value
            .parse::<i128>()
            .map_err(|_| StateStoreError::InvalidCanonicalTaskflowExport {
                reason: format!("updated_at is not RFC3339 or unix nanos: {value}"),
            })?;
    let parsed = OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|error| {
        StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!("updated_at unix nanos is invalid ({value}): {error}"),
        }
    })?;
    Ok(CanonicalTimestamp(parsed))
}

#[allow(dead_code)]
fn parse_canonical_task_status(value: &str) -> Result<CanonicalTaskStatus, StateStoreError> {
    match value {
        "open" => Ok(CanonicalTaskStatus::Open),
        "in_progress" => Ok(CanonicalTaskStatus::InProgress),
        "closed" => Ok(CanonicalTaskStatus::Closed),
        "blocked" => Ok(CanonicalTaskStatus::Blocked),
        other => Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!("unsupported taskflow-core status mapping: {other}"),
        }),
    }
}

#[allow(dead_code)]
fn parse_canonical_issue_type(value: &str) -> Result<CanonicalIssueType, StateStoreError> {
    match value {
        "epic" => Ok(CanonicalIssueType::Epic),
        "task" => Ok(CanonicalIssueType::Task),
        "bug" => Ok(CanonicalIssueType::Bug),
        "spike" => Ok(CanonicalIssueType::Spike),
        other => Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!("unsupported taskflow-core issue_type mapping: {other}"),
        }),
    }
}

#[allow(dead_code)]
fn task_dependency_to_canonical_edge(dependency: &TaskDependencyRecord) -> CanonicalDependencyEdge {
    CanonicalDependencyEdge {
        issue_id: CanonicalTaskId::new(&dependency.issue_id),
        depends_on_id: CanonicalTaskId::new(&dependency.depends_on_id),
        dependency_type: dependency.edge_type.clone(),
    }
}

#[allow(dead_code)]
fn task_record_to_canonical_snapshot_row(
    task: &TaskRecord,
) -> Result<CanonicalTaskRecord, StateStoreError> {
    Ok(CanonicalTaskRecord {
        id: CanonicalTaskId::new(&task.id),
        title: task.title.clone(),
        status: parse_canonical_task_status(&task.status)?,
        issue_type: parse_canonical_issue_type(&task.issue_type)?,
        updated_at: parse_canonical_timestamp(&task.updated_at)?,
    })
}

fn canonical_task_status_label(status: CanonicalTaskStatus) -> &'static str {
    match status {
        CanonicalTaskStatus::Open => "open",
        CanonicalTaskStatus::InProgress => "in_progress",
        CanonicalTaskStatus::Closed => "closed",
        CanonicalTaskStatus::Blocked => "blocked",
    }
}

fn canonical_issue_type_label(issue_type: CanonicalIssueType) -> &'static str {
    match issue_type {
        CanonicalIssueType::Epic => "epic",
        CanonicalIssueType::Task => "task",
        CanonicalIssueType::Bug => "bug",
        CanonicalIssueType::Spike => "spike",
    }
}

fn canonical_timestamp_label(timestamp: &CanonicalTimestamp) -> String {
    timestamp
        .0
        .format(&Rfc3339)
        .unwrap_or_else(|_| timestamp.0.unix_timestamp_nanos().to_string())
}

fn canonical_edge_to_task_dependency_record(
    dependency: &CanonicalDependencyEdge,
) -> TaskDependencyRecord {
    TaskDependencyRecord {
        issue_id: dependency.issue_id.0.clone(),
        depends_on_id: dependency.depends_on_id.0.clone(),
        edge_type: dependency.dependency_type.clone(),
        created_at: "canonical-taskflow-snapshot".to_string(),
        created_by: "taskflow-state-fs".to_string(),
        metadata: "{}".to_string(),
        thread_id: String::new(),
    }
}

fn canonical_snapshot_row_to_task_record(
    task: &CanonicalTaskRecord,
) -> Result<TaskRecord, StateStoreError> {
    let task_id = task.id.0.trim().to_string();
    if task_id.is_empty() {
        return Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: "canonical taskflow snapshot task id is empty".to_string(),
        });
    }

    let updated_at = canonical_timestamp_label(&task.updated_at);
    let status = canonical_task_status_label(task.status).to_string();
    let (closed_at, close_reason) = if matches!(task.status, CanonicalTaskStatus::Closed) {
        (
            Some(updated_at.clone()),
            Some("imported_from_canonical_taskflow_snapshot".to_string()),
        )
    } else {
        (None, None)
    };
    Ok(TaskRecord {
        id: task_id,
        title: task.title.clone(),
        description: String::new(),
        status,
        priority: 0,
        issue_type: canonical_issue_type_label(task.issue_type).to_string(),
        created_at: updated_at.clone(),
        created_by: "taskflow-state-fs".to_string(),
        updated_at,
        closed_at,
        close_reason,
        source_repo: "taskflow-state-fs".to_string(),
        compaction_level: 0,
        original_size: 0,
        notes: None,
        labels: Vec::new(),
        dependencies: Vec::new(),
    })
}

fn task_records_from_canonical_snapshot(
    snapshot: &TaskSnapshot,
) -> Result<Vec<TaskRecord>, StateStoreError> {
    let task_records = task_records_from_canonical_snapshot_rows(snapshot)?;
    let issues = StateStore::validate_task_graph_rows(&task_records);
    if let Some(first) = issues.first() {
        return Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!(
                "snapshot graph is invalid: {} on {}",
                first.issue_type, first.issue_id
            ),
        });
    }

    Ok(task_records)
}

fn task_records_from_canonical_snapshot_for_additive_import(
    snapshot: &TaskSnapshot,
    existing_tasks: &[TaskRecord],
) -> Result<Vec<TaskRecord>, StateStoreError> {
    let imported_tasks = task_records_from_canonical_snapshot_rows(snapshot)?;
    let mut merged_tasks = existing_tasks
        .iter()
        .cloned()
        .map(|task| (task.id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    for task in &imported_tasks {
        merged_tasks.insert(task.id.clone(), task.clone());
    }

    let merged_rows = merged_tasks.into_values().collect::<Vec<_>>();
    let issues = StateStore::validate_task_graph_rows(&merged_rows);
    if let Some(first) = issues.first() {
        return Err(StateStoreError::InvalidCanonicalTaskflowExport {
            reason: format!(
                "snapshot graph is invalid after additive merge: {} on {}",
                first.issue_type, first.issue_id
            ),
        });
    }

    Ok(imported_tasks)
}

fn task_records_from_canonical_snapshot_rows(
    snapshot: &TaskSnapshot,
) -> Result<Vec<TaskRecord>, StateStoreError> {
    let mut dependencies_by_issue = BTreeMap::<String, Vec<TaskDependencyRecord>>::new();
    for dependency in &snapshot.dependencies {
        dependencies_by_issue
            .entry(dependency.issue_id.0.clone())
            .or_default()
            .push(canonical_edge_to_task_dependency_record(dependency));
    }

    let mut task_records = Vec::with_capacity(snapshot.tasks.len());
    for task in &snapshot.tasks {
        let mut task_record = canonical_snapshot_row_to_task_record(task)?;
        if let Some(dependencies) = dependencies_by_issue.remove(&task.id.0) {
            task_record.dependencies = dependencies;
        }
        task_records.push(task_record);
    }

    Ok(task_records)
}

fn task_sort_key(left: &TaskRecord, right: &TaskRecord) -> std::cmp::Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| left.id.cmp(&right.id))
}

fn task_ready_sort_key(left: &TaskRecord, right: &TaskRecord) -> std::cmp::Ordering {
    let left_rank = if left.status == "in_progress" {
        0u8
    } else {
        1u8
    };
    let right_rank = if right.status == "in_progress" {
        0u8
    } else {
        1u8
    };
    left_rank
        .cmp(&right_rank)
        .then_with(|| left.priority.cmp(&right.priority))
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_task_paths(left: &[String], right: &[String]) -> std::cmp::Ordering {
    left.len()
        .cmp(&right.len())
        .then_with(|| left.join("->").cmp(&right.join("->")))
}

#[derive(Default)]
struct SourceMetadata {
    artifact_id: Option<String>,
    artifact_kind: Option<String>,
    version: Option<u32>,
    ownership_class: Option<String>,
    mutability_class: Option<String>,
    activation_class: Option<String>,
    required_follow_on: Vec<String>,
    hierarchy: Vec<String>,
}

fn parse_source_metadata(body: &str) -> SourceMetadata {
    let mut metadata = SourceMetadata::default();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "artifact_id" => metadata.artifact_id = Some(value),
            "artifact_kind" => metadata.artifact_kind = Some(value),
            "version" => metadata.version = value.parse::<u32>().ok(),
            "ownership_class" => metadata.ownership_class = Some(value),
            "mutability_class" => metadata.mutability_class = Some(value),
            "activation_class" => metadata.activation_class = Some(value),
            "required_follow_on" => {
                metadata.required_follow_on = value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            "hierarchy" => {
                metadata.hierarchy = value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            _ => {}
        }
    }
    metadata
}

fn infer_artifact_kind(slice: &str, relative: &Path) -> String {
    if slice == "framework_memory" {
        return "framework_memory_entry".to_string();
    }

    let normalized = relative.with_extension("").to_string_lossy().to_string();
    if normalized.ends_with("agent-definition") {
        "agent_definition".to_string()
    } else if normalized.ends_with("instruction-contract") {
        "instruction_contract".to_string()
    } else if normalized.ends_with("prompt-template-config") {
        "prompt_template_configuration".to_string()
    } else {
        "instruction_source".to_string()
    }
}

fn infer_ownership_class(slice: &str) -> &'static str {
    match slice {
        "framework_memory" => "framework",
        "instruction_memory" => "framework",
        _ => "project",
    }
}

fn infer_mutability_class(slice: &str) -> &'static str {
    match slice {
        "instruction_memory" => "immutable",
        "framework_memory" => "mutable",
        _ => "mutable",
    }
}

fn record_id_for_slice_source(slice: &str, relative: &Path) -> String {
    format!("{}-{}-source", slice, artifact_id_from_path(relative))
}

fn hierarchy_from_path(relative: &Path) -> Vec<String> {
    relative
        .parent()
        .map(|parent| {
            parent
                .iter()
                .map(|part| part.to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn default_state_dir() -> PathBuf {
    PathBuf::from(DEFAULT_STATE_DIR)
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(REPO_ROOT)
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn unix_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[allow(dead_code)]
fn split_lines(body: &str) -> Vec<String> {
    body.lines().map(|line| line.to_string()).collect()
}

#[allow(dead_code)]
fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[allow(dead_code)]
fn apply_patch_operation(
    lines: &mut Vec<String>,
    operation: &InstructionPatchOperation,
) -> Result<(), StateStoreError> {
    let index = resolve_operation_target(lines, operation)?;

    match operation.op.as_str() {
        "replace_range" => {
            lines.splice(index..=index, operation.with_lines.clone());
        }
        "replace_with_many" => {
            lines.splice(index..=index, operation.with_lines.clone());
        }
        "delete_range" => {
            lines.remove(index);
        }
        "insert_before" => {
            lines.splice(index..index, operation.with_lines.clone());
        }
        "insert_after" => {
            lines.splice(index + 1..index + 1, operation.with_lines.clone());
        }
        "append_block" => {
            lines.extend(operation.with_lines.clone());
        }
        other => {
            return Err(StateStoreError::InvalidPatchOperation {
                reason: format!("unsupported op: {other}"),
            });
        }
    }

    Ok(())
}

fn resolve_operation_target(
    lines: &[String],
    operation: &InstructionPatchOperation,
) -> Result<usize, StateStoreError> {
    match operation.target_mode.as_str() {
        "exact_text" => lines
            .iter()
            .position(|line| line == &operation.target)
            .ok_or_else(|| StateStoreError::InvalidPatchOperation {
                reason: format!(
                    "anchor not found for op {}: {}",
                    operation.op, operation.target
                ),
            }),
        "line_span" => {
            let line_number = operation.target.parse::<usize>().map_err(|_| {
                StateStoreError::InvalidPatchOperation {
                    reason: format!("invalid line_span target: {}", operation.target),
                }
            })?;
            if line_number == 0 || line_number > lines.len() {
                return Err(StateStoreError::InvalidPatchOperation {
                    reason: format!("line_span out of bounds: {}", operation.target),
                });
            }
            Ok(line_number - 1)
        }
        "anchor_hash" => {
            let target_hash = operation.target.strip_prefix("blake3:").ok_or_else(|| {
                StateStoreError::InvalidPatchOperation {
                    reason: format!("invalid anchor_hash target format: {}", operation.target),
                }
            })?;

            lines
                .iter()
                .position(|line| blake3::hash(line.as_bytes()).to_hex().as_str() == target_hash)
                .ok_or_else(|| StateStoreError::InvalidPatchOperation {
                    reason: format!("anchor hash not found for op {}", operation.op),
                })
        }
        other => Err(StateStoreError::InvalidPatchOperation {
            reason: format!("unsupported target_mode: {other}"),
        }),
    }
}

fn validate_patch_conflicts(patches: &[InstructionDiffPatchRow]) -> Result<(), StateStoreError> {
    use std::collections::HashMap;

    let mut claimed: HashMap<(String, String), (u32, String)> = HashMap::new();

    for patch in patches {
        for operation in &patch.operations {
            if matches!(
                operation.op.as_str(),
                "replace_range" | "replace_with_many" | "delete_range"
            ) {
                let key = (operation.target_mode.clone(), operation.target.clone());
                if let Some((existing_precedence, existing_patch_id)) = claimed.get(&key) {
                    if *existing_precedence == patch.patch_precedence {
                        return Err(StateStoreError::PatchConflict {
                            reason: format!(
                                "patches {} and {} target the same anchor with equal precedence",
                                existing_patch_id, patch.patch_id
                            ),
                        });
                    }
                } else {
                    claimed.insert(key, (patch.patch_precedence, patch.patch_id.clone()));
                }
            }
        }
    }

    Ok(())
}

fn validate_patch_bindings(
    base: &InstructionArtifactRow,
    patches: &[InstructionDiffPatchRow],
) -> Result<(), StateStoreError> {
    for patch in patches {
        if patch.target_artifact_version != base.version {
            return Err(StateStoreError::InvalidPatchOperation {
                reason: format!(
                    "patch {} targets artifact version {} but base version is {}",
                    patch.patch_id, patch.target_artifact_version, base.version
                ),
            });
        }

        if patch.target_artifact_hash != base.source_hash {
            return Err(StateStoreError::InvalidPatchOperation {
                reason: format!(
                    "patch {} targets artifact hash {} but base hash is {}",
                    patch.patch_id, patch.target_artifact_hash, base.source_hash
                ),
            });
        }
    }

    Ok(())
}

fn collect_patch_ids(patches: &[InstructionDiffPatchRow]) -> Vec<String> {
    patches.iter().map(|patch| patch.patch_id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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

        assert!(state_schema_document().contains(&bootstrap_document));
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
            "Run `vida taskflow run-graph dispatch --json` to materialize run-graph dispatch receipt evidence before operator handoff."
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
                description: "root",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &labels,
                created_by: "tester",
                source_repo: ".",
            })
            .await
            .expect("create root task");
        store
            .create_task(CreateTaskRequest {
                task_id: "vida-child",
                title: "Child",
                description: "child",
                issue_type: "task",
                status: "open",
                priority: 2,
                parent_id: Some("vida-root"),
                labels: &labels,
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
        let root = std::env::temp_dir().join(format!(
            "vida-update-task-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let labels = vec!["framework".to_string()];

        store
            .create_task(CreateTaskRequest {
                task_id: "vida-root",
                title: "Root",
                description: "root",
                issue_type: "epic",
                status: "open",
                priority: 1,
                parent_id: None,
                labels: &labels,
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
                add_labels: &[],
                remove_labels: &[],
                set_labels: Some(&set_labels_vec),
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
                add_labels: &add_labels,
                remove_labels: &remove_labels,
                set_labels: None,
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
        assert_eq!(compatibility.classification, "incompatible");
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
        assert_eq!(compatibility.classification, "incompatible");
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
        assert_eq!(compatibility.classification, "compatible");
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
        assert_eq!(persisted.classification, "compatible");
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
    fn run_graph_dispatch_receipt_summary_prefers_exception_lane_status_over_dispatch_mapping() {
        let mut receipt = sample_dispatch_receipt_with_status("executed");
        receipt.exception_path_receipt_id = Some("receipt-exception-1".to_string());
        receipt.supersedes_receipt_id = Some("receipt-superseded-1".to_string());

        let summary = RunGraphDispatchReceiptSummary::from_receipt(receipt);

        assert_eq!(summary.lane_status, "lane_exception_takeover");
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
        assert_eq!(summary.lane_status, "lane_exception_takeover");

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
        assert_eq!(summary.compatibility_classification, "compatible");
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
        assert_eq!(summary.compatibility_classification, "incompatible");
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
        assert_eq!(summary.compatibility_classification, "incompatible");
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
        assert_eq!(summary.compatibility_classification, "compatible");
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
        assert_eq!(persisted.compatibility_classification, "compatible");
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
    async fn replace_with_taskflow_snapshot_removes_stale_dependencies_for_kept_tasks() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-replace-deps-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-blocker\",\"title\":\"Blocker\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep\",\"description\":\"keep\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-blocker\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n"
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
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-keep"),
                    title: "Keep".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                            .expect("parse keep timestamp"),
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

        let keep = store
            .show_task("vida-keep")
            .await
            .expect("keep task should remain");
        assert_eq!(keep.dependencies.len(), 1);
        assert_eq!(keep.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(keep.dependencies[0].edge_type, "parent-child");

        let blockers = store
            .task_dependencies("vida-keep")
            .await
            .expect("dependencies should load");
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].depends_on_id, "vida-root");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_replaces_dependencies_for_updated_tasks_without_removing_unrelated_tasks(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-deps-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            concat!(
                "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-blocker\",\"title\":\"Blocker\",\"description\":\"blocker\",\"status\":\"open\",\"priority\":2,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
                "{\"id\":\"vida-keep\",\"title\":\"Keep\",\"description\":\"keep\",\"status\":\"open\",\"priority\":3,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-root\",\"type\":\"parent-child\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"},{\"issue_id\":\"vida-keep\",\"depends_on_id\":\"vida-blocker\",\"type\":\"blocks\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"metadata\":\"{}\",\"thread_id\":\"\"}]}\n",
                "{\"id\":\"vida-unrelated\",\"title\":\"Unrelated\",\"description\":\"unrelated\",\"status\":\"open\",\"priority\":4,\"issue_type\":\"task\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n"
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
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Epic,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:00Z", &Rfc3339)
                            .expect("parse root timestamp"),
                    ),
                },
                CanonicalTaskRecord {
                    id: CanonicalTaskId::new("vida-keep"),
                    title: "Keep".to_string(),
                    status: CanonicalTaskStatus::Open,
                    issue_type: CanonicalIssueType::Task,
                    updated_at: CanonicalTimestamp(
                        OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                            .expect("parse keep timestamp"),
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
            .import_taskflow_snapshot(&snapshot)
            .await
            .expect("additive import should succeed");

        let keep = store
            .show_task("vida-keep")
            .await
            .expect("keep task should remain");
        assert_eq!(keep.dependencies.len(), 1);
        assert_eq!(keep.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(keep.dependencies[0].edge_type, "parent-child");

        let unrelated = store
            .show_task("vida-unrelated")
            .await
            .expect("unrelated task should remain after additive import");
        assert_eq!(unrelated.title, "Unrelated");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "import_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 2);
        assert_eq!(latest.dependency_count, 1);

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 1);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 1);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.latest_operation.as_deref(), Some("import_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_memory")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_allows_dependencies_on_existing_authoritative_tasks_outside_payload(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-existing-target-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
        fs::write(
            &source,
            "{\"id\":\"vida-root\",\"title\":\"Root\",\"description\":\"root\",\"status\":\"open\",\"priority\":1,\"issue_type\":\"epic\",\"created_at\":\"2026-03-08T00:00:00Z\",\"created_by\":\"tester\",\"updated_at\":\"2026-03-08T00:00:00Z\",\"source_repo\":\".\",\"compaction_level\":0,\"original_size\":0,\"labels\":[],\"dependencies\":[]}\n",
        )
        .expect("write initial jsonl");
        store
            .import_tasks_from_jsonl(&source)
            .await
            .expect("initial import should succeed");

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-child"),
                title: "Child".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse child timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-child"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };

        store
            .import_taskflow_snapshot(&snapshot)
            .await
            .expect("additive import should accept existing authoritative dependency target");

        let child = store
            .show_task("vida-child")
            .await
            .expect("child task should be imported");
        assert_eq!(child.dependencies.len(), 1);
        assert_eq!(child.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(child.dependencies[0].edge_type, "parent-child");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load")
            .expect("latest reconciliation receipt should exist");
        assert_eq!(latest.operation, "import_snapshot");
        assert_eq!(latest.source_kind, "canonical_snapshot_memory");
        assert_eq!(latest.task_count, 1);
        assert_eq!(latest.dependency_count, 1);

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 1);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 1);
        assert_eq!(bridge.file_import_receipts, 0);
        assert_eq!(bridge.latest_operation.as_deref(), Some("import_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_memory")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_fails_closed_before_mutation_on_post_merge_parent_conflict() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-import-parent-conflict-{}-{}",
            std::process::id(),
            nanos
        ));

        let store = StateStore::open(root.clone()).await.expect("open store");
        let source = root.join("tasks.jsonl");
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

        let before_child = store
            .show_task("vida-child")
            .await
            .expect("child should exist before conflicting import");
        assert_eq!(before_child.title, "Child old");
        assert_eq!(before_child.dependencies.len(), 1);
        assert_eq!(before_child.dependencies[0].depends_on_id, "vida-root-a");

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

        let error = store
            .import_taskflow_snapshot(&snapshot)
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
            .expect("child should still exist after rejected import");
        assert_eq!(after_child.title, "Child old");
        assert_eq!(after_child.dependencies.len(), 1);
        assert_eq!(after_child.dependencies[0].depends_on_id, "vida-root-a");

        let latest = store
            .latest_task_reconciliation_summary()
            .await
            .expect("latest reconciliation summary should load");
        assert!(
            latest.is_none(),
            "rejected import must not emit reconciliation receipt"
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

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn import_taskflow_snapshot_file_allows_dependencies_on_existing_authoritative_tasks_outside_payload(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-taskflow-snapshot-file-import-existing-target-{}-{}",
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

        let snapshot = TaskSnapshot {
            tasks: vec![CanonicalTaskRecord {
                id: CanonicalTaskId::new("vida-child"),
                title: "Child".to_string(),
                status: CanonicalTaskStatus::Open,
                issue_type: CanonicalIssueType::Task,
                updated_at: CanonicalTimestamp(
                    OffsetDateTime::parse("2026-03-08T00:00:05Z", &Rfc3339)
                        .expect("parse child timestamp"),
                ),
            }],
            dependencies: vec![CanonicalDependencyEdge {
                issue_id: CanonicalTaskId::new("vida-child"),
                depends_on_id: CanonicalTaskId::new("vida-root"),
                dependency_type: "parent-child".to_string(),
            }],
        };
        taskflow_state_fs::write_snapshot(&snapshot_path, &snapshot).expect("write snapshot");

        store
            .import_taskflow_snapshot_file(&snapshot_path)
            .await
            .expect(
            "file-backed additive import should accept existing authoritative dependency target",
        );

        let child = store
            .show_task("vida-child")
            .await
            .expect("child task should be imported");
        assert_eq!(child.dependencies.len(), 1);
        assert_eq!(child.dependencies[0].depends_on_id, "vida-root");
        assert_eq!(child.dependencies[0].edge_type, "parent-child");

        let graph_issues = store
            .validate_task_graph()
            .await
            .expect("graph validation should succeed");
        assert!(graph_issues.is_empty());

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
        assert_eq!(latest.dependency_count, 1);

        let bridge = store
            .taskflow_snapshot_bridge_summary()
            .await
            .expect("snapshot bridge summary should load");
        assert_eq!(bridge.total_receipts, 1);
        assert_eq!(bridge.import_receipts, 1);
        assert_eq!(bridge.memory_import_receipts, 0);
        assert_eq!(bridge.file_import_receipts, 1);
        assert_eq!(bridge.latest_operation.as_deref(), Some("import_snapshot"));
        assert_eq!(
            bridge.latest_source_kind.as_deref(),
            Some("canonical_snapshot_file")
        );
        assert_eq!(
            bridge.latest_source_path.as_deref(),
            Some(snapshot_path.to_string_lossy().as_ref())
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
}
