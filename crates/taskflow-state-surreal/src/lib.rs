use std::path::{Path, PathBuf};

use thiserror::Error;

pub const DEFAULT_NAMESPACE: &str = "vida";
pub const DEFAULT_DATABASE: &str = "primary";
pub const DEFAULT_BACKEND: &str = "kv-surrealkv";
pub const DEFAULT_ENGINE: &str = "surrealdb";
pub const DEFAULT_STATE_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_INSTRUCTION_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_STATE_TABLES: &[&str] = &[
    "task",
    "task_dependency",
    "task_blocker",
    "execution_plan_state",
    "routed_run_state",
    "governance_state",
    "resumability_capsule",
    "task_reconciliation_summary",
    "state_spine_manifest",
    "boot_compatibility_state",
    "migration_runtime_state",
    "migration_compatibility_receipt",
    "migration_application_receipt",
    "migration_verification_receipt",
    "migration_cutover_readiness_receipt",
    "migration_rollback_note",
    "storage_meta",
];
pub const DEFAULT_AUTHORITATIVE_MUTATION_ROOT: &str = "vida task";
pub const DEFAULT_STATE_ENTITY_SURFACES: &[&str] = &[
    "task",
    "task_dependency",
    "task_blocker",
    "execution_plan_state",
    "routed_run_state",
    "governance_state",
    "resumability_capsule",
    "task_reconciliation_summary",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurrealStorageMeta {
    pub engine: &'static str,
    pub backend: &'static str,
    pub namespace: String,
    pub database: String,
    pub state_schema_version: u32,
    pub instruction_schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurrealBootstrapPayload {
    pub target: SurrealStoreTarget,
    pub storage_meta: SurrealStorageMeta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSpineManifestContract {
    pub manifest_id: String,
    pub state_schema_version: u32,
    pub authoritative_mutation_root: String,
    pub entity_surfaces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurrealStoreTarget {
    pub root: PathBuf,
    pub namespace: String,
    pub database: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SurrealStoreTargetError {
    #[error("surreal root path must not be empty")]
    EmptyRoot,
    #[error("surreal namespace must not be empty")]
    EmptyNamespace,
    #[error("surreal database must not be empty")]
    EmptyDatabase,
}

impl SurrealStoreTarget {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            namespace: DEFAULT_NAMESPACE.to_string(),
            database: DEFAULT_DATABASE.to_string(),
        }
    }

    pub fn validate(&self) -> Result<(), SurrealStoreTargetError> {
        if self.root.as_os_str().is_empty() {
            return Err(SurrealStoreTargetError::EmptyRoot);
        }
        if self.namespace.trim().is_empty() {
            return Err(SurrealStoreTargetError::EmptyNamespace);
        }
        if self.database.trim().is_empty() {
            return Err(SurrealStoreTargetError::EmptyDatabase);
        }
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn backend(&self) -> &'static str {
        DEFAULT_BACKEND
    }

    pub fn namespace_root(&self) -> PathBuf {
        self.root.join(&self.namespace)
    }

    pub fn database_root(&self) -> PathBuf {
        self.namespace_root().join(&self.database)
    }

    pub fn storage_meta(&self) -> SurrealStorageMeta {
        SurrealStorageMeta {
            engine: DEFAULT_ENGINE,
            backend: self.backend(),
            namespace: self.namespace.clone(),
            database: self.database.clone(),
            state_schema_version: DEFAULT_STATE_SCHEMA_VERSION,
            instruction_schema_version: DEFAULT_INSTRUCTION_SCHEMA_VERSION,
        }
    }

    pub fn bootstrap_payload(&self) -> SurrealBootstrapPayload {
        SurrealBootstrapPayload {
            target: self.clone(),
            storage_meta: self.storage_meta(),
        }
    }

    pub fn storage_meta_upsert_statement(&self) -> String {
        let meta = self.storage_meta();
        format!(
            "UPSERT storage_meta:primary CONTENT {{ engine: '{engine}', backend: '{backend}', namespace: '{namespace}', database: '{database}', state_schema_version: {state_schema_version}, instruction_schema_version: {instruction_schema_version} }};",
            engine = meta.engine,
            backend = meta.backend,
            namespace = meta.namespace,
            database = meta.database,
            state_schema_version = meta.state_schema_version,
            instruction_schema_version = meta.instruction_schema_version,
        )
    }

    pub fn bootstrap_schema_statements(&self) -> Vec<String> {
        let mut statements = DEFAULT_STATE_TABLES
            .iter()
            .map(|table| format!("DEFINE TABLE {table} SCHEMALESS;"))
            .collect::<Vec<_>>();
        statements.push(self.storage_meta_upsert_statement());
        statements
    }

    pub fn bootstrap_schema_document(&self) -> String {
        self.bootstrap_schema_statements().join("\n")
    }

    pub fn state_spine_manifest_contract(&self) -> StateSpineManifestContract {
        StateSpineManifestContract {
            manifest_id: "primary".to_string(),
            state_schema_version: DEFAULT_STATE_SCHEMA_VERSION,
            authoritative_mutation_root: DEFAULT_AUTHORITATIVE_MUTATION_ROOT.to_string(),
            entity_surfaces: DEFAULT_STATE_ENTITY_SURFACES
                .iter()
                .map(|surface| (*surface).to_string())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_BACKEND, DEFAULT_DATABASE, DEFAULT_ENGINE, DEFAULT_INSTRUCTION_SCHEMA_VERSION,
        DEFAULT_NAMESPACE, DEFAULT_STATE_ENTITY_SURFACES, DEFAULT_STATE_SCHEMA_VERSION,
        DEFAULT_STATE_TABLES, StateSpineManifestContract, SurrealBootstrapPayload,
        SurrealStorageMeta, SurrealStoreTarget, SurrealStoreTargetError,
    };
    use std::path::PathBuf;

    #[test]
    fn uses_canonical_surreal_defaults() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");
        assert_eq!(target.namespace, DEFAULT_NAMESPACE);
        assert_eq!(target.database, DEFAULT_DATABASE);
        assert_eq!(target.backend(), DEFAULT_BACKEND);
        assert_eq!(target.root(), PathBuf::from("/tmp/vida-state").as_path());
    }

    #[test]
    fn rejects_empty_root() {
        let target = SurrealStoreTarget::new("");
        let error = target.validate().expect_err("empty root should fail");
        assert_eq!(error, SurrealStoreTargetError::EmptyRoot);
    }

    #[test]
    fn validates_explicit_target() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");
        assert!(target.validate().is_ok());
    }

    #[test]
    fn derives_deterministic_namespace_and_database_roots() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");

        assert_eq!(target.namespace_root(), PathBuf::from("/tmp/vida-state/vida"));
        assert_eq!(
            target.database_root(),
            PathBuf::from("/tmp/vida-state/vida/primary")
        );
    }

    #[test]
    fn exposes_canonical_storage_metadata() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");

        assert_eq!(
            target.storage_meta(),
            SurrealStorageMeta {
                engine: DEFAULT_ENGINE,
                backend: DEFAULT_BACKEND,
                namespace: DEFAULT_NAMESPACE.to_string(),
                database: DEFAULT_DATABASE.to_string(),
                state_schema_version: DEFAULT_STATE_SCHEMA_VERSION,
                instruction_schema_version: DEFAULT_INSTRUCTION_SCHEMA_VERSION,
            }
        );
    }

    #[test]
    fn exposes_bootstrap_payload_with_target_and_storage_meta() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");

        assert_eq!(
            target.bootstrap_payload(),
            SurrealBootstrapPayload {
                target: target.clone(),
                storage_meta: target.storage_meta(),
            }
        );
    }

    #[test]
    fn renders_canonical_storage_meta_upsert_statement() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");

        assert_eq!(
            target.storage_meta_upsert_statement(),
            "UPSERT storage_meta:primary CONTENT { engine: 'surrealdb', backend: 'kv-surrealkv', namespace: 'vida', database: 'primary', state_schema_version: 1, instruction_schema_version: 1 };"
        );
    }

    #[test]
    fn renders_bootstrap_schema_bundle_for_state_tables() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");
        let statements = target.bootstrap_schema_statements();

        assert_eq!(statements.len(), DEFAULT_STATE_TABLES.len() + 1);
        assert_eq!(statements.first().expect("first statement"), "DEFINE TABLE task SCHEMALESS;");
        assert_eq!(
            statements.last().expect("last statement"),
            "UPSERT storage_meta:primary CONTENT { engine: 'surrealdb', backend: 'kv-surrealkv', namespace: 'vida', database: 'primary', state_schema_version: 1, instruction_schema_version: 1 };"
        );
    }

    #[test]
    fn renders_bootstrap_schema_document() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");
        let document = target.bootstrap_schema_document();

        assert!(document.starts_with("DEFINE TABLE task SCHEMALESS;"));
        assert!(document.contains("\nDEFINE TABLE storage_meta SCHEMALESS;\n"));
        assert!(document.ends_with(
            "UPSERT storage_meta:primary CONTENT { engine: 'surrealdb', backend: 'kv-surrealkv', namespace: 'vida', database: 'primary', state_schema_version: 1, instruction_schema_version: 1 };"
        ));
    }

    #[test]
    fn exposes_canonical_state_spine_manifest_contract() {
        let target = SurrealStoreTarget::new("/tmp/vida-state");

        assert_eq!(
            target.state_spine_manifest_contract(),
            StateSpineManifestContract {
                manifest_id: "primary".to_string(),
                state_schema_version: DEFAULT_STATE_SCHEMA_VERSION,
                authoritative_mutation_root: "vida task".to_string(),
                entity_surfaces: DEFAULT_STATE_ENTITY_SURFACES
                    .iter()
                    .map(|surface| (*surface).to_string())
                    .collect(),
            }
        );
    }
}
