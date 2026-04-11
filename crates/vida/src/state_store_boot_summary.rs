use super::{
    canonical_compatibility_class_str, canonical_release1_contract_type_str,
    canonical_release1_schema_version_str, unix_timestamp_nanos, CompatibilityClass,
    EffectiveBundleReceiptSummary, Release1ContractType, Release1SchemaVersion,
    StateSpineManifestContract, StateStore, StateStoreError, SurrealStoreTarget, SurrealValue,
    DEFAULT_STATE_DIR,
};

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct StorageMetaRow {
    pub(crate) engine: String,
    pub(crate) backend: String,
    pub(crate) namespace: String,
    pub(crate) database: String,
    pub(crate) state_schema_version: u32,
    pub(crate) instruction_schema_version: u32,
}

#[derive(Debug, serde::Deserialize, SurrealValue)]
pub(crate) struct CountRow {
    pub(crate) total: usize,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct StateSpineManifestContent {
    pub(crate) manifest_id: String,
    pub(crate) state_schema_version: u32,
    pub(crate) authoritative_mutation_root: String,
    pub(crate) entity_surfaces: Vec<String>,
    pub(crate) initialized_at: String,
}

impl StateSpineManifestContent {
    pub(crate) fn from_contract(
        contract: StateSpineManifestContract,
        initialized_at: String,
    ) -> Self {
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

#[derive(Debug)]
pub struct StorageMetadataSummary {
    pub engine: String,
    pub backend: String,
    pub namespace: String,
    pub database: String,
    pub state_schema_version: u32,
    pub instruction_schema_version: u32,
}

impl StorageMetadataSummary {
    pub fn as_display(&self) -> String {
        format!(
            "{}:{} {}.{}",
            self.engine, self.backend, self.state_schema_version, self.instruction_schema_version
        )
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, SurrealValue)]
pub(crate) struct BootCompatibilityStateRow {
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
pub(crate) struct MigrationRuntimeStateRow {
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
pub(crate) struct MigrationCompatibilityReceiptRow {
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

impl StateStore {
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

    pub(crate) async fn count_table_rows(&self, table: &str) -> Result<usize, StateStoreError> {
        let query = format!("SELECT count() AS total FROM {table} GROUP ALL;");
        let mut response = self.db.query(query).await?;
        let rows: Vec<CountRow> = response.take(0)?;
        Ok(rows.into_iter().next().map(|row| row.total).unwrap_or(0))
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
            CompatibilityClass::BackwardCompatible.as_str()
        } else if hard_failures > 0 {
            CompatibilityClass::ReaderUpgradeRequired.as_str()
        } else {
            "insufficient_evidence"
        };

        let summary = BootCompatibilitySummary {
            classification: classification.to_string(),
            reasons,
            next_step: if classification == CompatibilityClass::BackwardCompatible.as_str() {
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
            CompatibilityClass::BackwardCompatible
        } else {
            CompatibilityClass::ReaderUpgradeRequired
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
            .unwrap_or(CompatibilityClass::ReaderUpgradeRequired.as_str())
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
}
