use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use vida_runtime_tools::ToolAuditEnvelope;

pub const PROVIDER_ID: &str = "vida-coder";
pub const RECEIPT_SCHEMA_VERSION: &str = "vida-coder-receipt-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAuthRef {
    pub profile_ref: String,
    pub source: AuthRefSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRefSource {
    EnvRef,
    SecretRef,
    RuntimeProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RigProviderAdapterConfig {
    pub provider: String,
    pub model_ref: String,
    pub model_profile_id: String,
    pub reasoning_effort: Option<String>,
    pub auth_ref: ProviderAuthRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderReadiness {
    pub status: ReadinessStatus,
    pub provider: String,
    pub model_ref: String,
    pub model_profile_id: String,
    pub blocker_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VidaRuntimeKnowledgePack {
    pub project_root: String,
    pub state_dir: String,
    pub session_id: String,
    pub packet_id: String,
    pub task_id: String,
    pub runtime_role: String,
    pub selected_backend_id: String,
    pub selected_model_provider: String,
    pub selected_model_ref: String,
    pub selected_model_profile_id: String,
    pub selected_reasoning_effort: Option<String>,
    pub owned_paths: Vec<String>,
    pub read_only_paths: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub verification_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceProjectRuntime {
    pub project_id: String,
    pub project_root: String,
    pub state_dir: String,
    pub vida_binary_fingerprint: String,
    pub config_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderSession {
    pub session_id: String,
    pub project_id: String,
    pub packet_id: String,
    pub runtime_role: String,
    pub backend_id: String,
    pub model_profile_id: String,
    pub owned_paths: Vec<String>,
    pub read_only_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceWorkerLease {
    pub project_id: String,
    pub task_id: String,
    pub packet_id: String,
    pub conflict_domain: Option<String>,
    pub parallel_group: Option<String>,
    pub expires_at_epoch_ms: u64,
    pub heartbeat_at_epoch_ms: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ServiceScheduler {
    projects: Vec<ServiceProjectRuntime>,
    leases: Vec<ServiceWorkerLease>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolAuditRecord {
    pub tool_name: String,
    pub status: String,
    pub touched_paths: Vec<String>,
}

impl From<ToolAuditEnvelope> for ToolAuditRecord {
    fn from(value: ToolAuditEnvelope) -> Self {
        Self {
            tool_name: value.tool_name,
            status: value.status,
            touched_paths: value.touched_paths,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEvidence {
    pub command: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoderReceipt {
    pub schema_version: String,
    pub status: String,
    pub packet_id: String,
    pub session_id: String,
    pub provider: String,
    pub model_ref: String,
    pub model_profile_id: String,
    pub tool_audit: Vec<ToolAuditRecord>,
    pub touched_paths: Vec<String>,
    pub verification: Vec<VerificationEvidence>,
    pub blockers: Vec<String>,
    pub raw_provider: Value,
    pub handoff_summary: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoderContractError {
    #[error("provider id must be vida-coder")]
    UnsupportedProvider,
    #[error("model_ref is required")]
    MissingModelRef,
    #[error("model_profile_id is required")]
    MissingModelProfile,
    #[error("auth profile reference is required")]
    MissingAuthRef,
    #[error("auth profile reference must not contain secret material")]
    SecretMaterialInAuthRef,
    #[error("receipt touched_paths contains a path outside the packet owned scope: {0}")]
    TouchedPathOutsideScope(String),
    #[error("project is already registered: {0}")]
    ProjectAlreadyRegistered(String),
    #[error("project is not registered: {0}")]
    ProjectNotRegistered(String),
    #[error("conflict domain already leased: {0}")]
    ConflictDomainAlreadyLeased(String),
}

pub fn provider_readiness(config: &RigProviderAdapterConfig) -> ProviderReadiness {
    let mut blocker_codes = Vec::new();

    if config.provider != PROVIDER_ID {
        blocker_codes.push("provider_id_not_vida_coder".to_string());
    }
    if config.model_ref.trim().is_empty() {
        blocker_codes.push("selected_model_ref_missing".to_string());
    }
    if config.model_profile_id.trim().is_empty() {
        blocker_codes.push("selected_model_profile_id_missing".to_string());
    }
    if config.auth_ref.profile_ref.trim().is_empty() {
        blocker_codes.push("provider_auth_ref_missing".to_string());
    }
    if auth_ref_contains_secret_material(&config.auth_ref.profile_ref) {
        blocker_codes.push("provider_auth_ref_contains_secret_material".to_string());
    }

    let status = if blocker_codes.is_empty() {
        ReadinessStatus::Ready
    } else {
        ReadinessStatus::Blocked
    };

    ProviderReadiness {
        status,
        provider: config.provider.clone(),
        model_ref: config.model_ref.clone(),
        model_profile_id: config.model_profile_id.clone(),
        blocker_codes,
    }
}

pub fn validate_provider_config(
    config: &RigProviderAdapterConfig,
) -> Result<ProviderReadiness, CoderContractError> {
    if config.provider != PROVIDER_ID {
        return Err(CoderContractError::UnsupportedProvider);
    }
    if config.model_ref.trim().is_empty() {
        return Err(CoderContractError::MissingModelRef);
    }
    if config.model_profile_id.trim().is_empty() {
        return Err(CoderContractError::MissingModelProfile);
    }
    if config.auth_ref.profile_ref.trim().is_empty() {
        return Err(CoderContractError::MissingAuthRef);
    }
    if auth_ref_contains_secret_material(&config.auth_ref.profile_ref) {
        return Err(CoderContractError::SecretMaterialInAuthRef);
    }

    Ok(provider_readiness(config))
}

pub fn config_from_knowledge_pack(
    pack: &VidaRuntimeKnowledgePack,
    auth_ref: ProviderAuthRef,
) -> RigProviderAdapterConfig {
    RigProviderAdapterConfig {
        provider: pack.selected_model_provider.clone(),
        model_ref: pack.selected_model_ref.clone(),
        model_profile_id: pack.selected_model_profile_id.clone(),
        reasoning_effort: pack.selected_reasoning_effort.clone(),
        auth_ref,
    }
}

pub fn build_receipt(
    pack: &VidaRuntimeKnowledgePack,
    config: &RigProviderAdapterConfig,
    tool_audit: Vec<ToolAuditRecord>,
    verification: Vec<VerificationEvidence>,
    blockers: Vec<String>,
    raw_provider: Value,
    handoff_summary: impl Into<String>,
) -> Result<CoderReceipt, CoderContractError> {
    validate_provider_config(config)?;

    let mut touched_paths = Vec::new();
    for audit in &tool_audit {
        for path in &audit.touched_paths {
            if !pack
                .owned_paths
                .iter()
                .any(|owned| path == owned || path.starts_with(&format!("{owned}/")))
            {
                return Err(CoderContractError::TouchedPathOutsideScope(path.clone()));
            }
            if !touched_paths.contains(path) {
                touched_paths.push(path.clone());
            }
        }
    }

    let status = if blockers.is_empty() {
        "pass"
    } else {
        "blocked"
    };

    Ok(CoderReceipt {
        schema_version: RECEIPT_SCHEMA_VERSION.to_string(),
        status: status.to_string(),
        packet_id: pack.packet_id.clone(),
        session_id: pack.session_id.clone(),
        provider: config.provider.clone(),
        model_ref: config.model_ref.clone(),
        model_profile_id: config.model_profile_id.clone(),
        tool_audit,
        touched_paths,
        verification,
        blockers,
        raw_provider,
        handoff_summary: handoff_summary.into(),
    })
}

fn auth_ref_contains_secret_material(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("sk-")
        || normalized.contains("api_key=")
        || normalized.contains("apikey=")
        || normalized.contains("token=")
        || normalized.contains("secret=")
}

pub fn redacted_provider_probe(config: &RigProviderAdapterConfig) -> Value {
    let readiness = provider_readiness(config);
    json!({
        "provider": readiness.provider,
        "model_ref": readiness.model_ref,
        "model_profile_id": readiness.model_profile_id,
        "auth_ref_source": config.auth_ref.source,
        "auth_ref_present": !config.auth_ref.profile_ref.trim().is_empty(),
        "status": readiness.status,
        "blocker_codes": readiness.blocker_codes,
    })
}

impl ServiceScheduler {
    pub fn register_project(
        &mut self,
        project: ServiceProjectRuntime,
    ) -> Result<(), CoderContractError> {
        if self
            .projects
            .iter()
            .any(|existing| existing.project_id == project.project_id)
        {
            return Err(CoderContractError::ProjectAlreadyRegistered(
                project.project_id,
            ));
        }
        self.projects.push(project);
        Ok(())
    }

    pub fn claim_worker(&mut self, lease: ServiceWorkerLease) -> Result<(), CoderContractError> {
        if !self
            .projects
            .iter()
            .any(|project| project.project_id == lease.project_id)
        {
            return Err(CoderContractError::ProjectNotRegistered(lease.project_id));
        }

        if let Some(domain) = lease.conflict_domain.as_deref()
            && self.leases.iter().any(|existing| {
                existing.project_id == lease.project_id
                    && existing.conflict_domain.as_deref() == Some(domain)
            })
        {
            return Err(CoderContractError::ConflictDomainAlreadyLeased(
                domain.to_string(),
            ));
        }

        self.leases.push(lease);
        Ok(())
    }

    pub fn project_count(&self) -> usize {
        self.projects.len()
    }

    pub fn active_lease_count(&self) -> usize {
        self.leases.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> VidaRuntimeKnowledgePack {
        VidaRuntimeKnowledgePack {
            project_root: "C:/project/vida-stack".to_string(),
            state_dir: "C:/project/vida-stack/.vida/data/state".to_string(),
            session_id: "session-1".to_string(),
            packet_id: "packet-1".to_string(),
            task_id: "task-1".to_string(),
            runtime_role: "worker".to_string(),
            selected_backend_id: "vida_coder".to_string(),
            selected_model_provider: PROVIDER_ID.to_string(),
            selected_model_ref: "vida-coder/provider-configured".to_string(),
            selected_model_profile_id: "vida_coder_medium_guarded".to_string(),
            selected_reasoning_effort: Some("medium".to_string()),
            owned_paths: vec!["crates/vida-coder".to_string(), "Cargo.toml".to_string()],
            read_only_paths: vec!["docs/product/spec".to_string()],
            allowed_tools: vec!["vida_current_packet".to_string()],
            verification_target: Some("cargo test -p vida-coder".to_string()),
        }
    }

    fn auth_ref() -> ProviderAuthRef {
        ProviderAuthRef {
            profile_ref: "env:VIDA_CODER_PROVIDER_AUTH".to_string(),
            source: AuthRefSource::EnvRef,
        }
    }

    #[test]
    fn provider_config_resolves_model_and_auth_without_secret_values() {
        let config = config_from_knowledge_pack(&pack(), auth_ref());
        let readiness = validate_provider_config(&config).expect("config should be valid");
        assert_eq!(readiness.status, ReadinessStatus::Ready);
        assert_eq!(readiness.provider, PROVIDER_ID);
        assert_eq!(readiness.model_ref, "vida-coder/provider-configured");
        assert_eq!(readiness.model_profile_id, "vida_coder_medium_guarded");

        let probe = redacted_provider_probe(&config);
        assert_eq!(probe["auth_ref_present"], true);
        assert!(probe.to_string().contains("EnvRef") || probe.to_string().contains("env_ref"));
        assert!(!probe.to_string().contains("VIDA_CODER_PROVIDER_AUTH"));
    }

    #[test]
    fn provider_config_blocks_secret_material_in_auth_reference() {
        let mut config = config_from_knowledge_pack(&pack(), auth_ref());
        config.auth_ref.profile_ref = "api_key=sk-test-secret".to_string();
        assert_eq!(
            validate_provider_config(&config),
            Err(CoderContractError::SecretMaterialInAuthRef)
        );
        let readiness = provider_readiness(&config);
        assert_eq!(readiness.status, ReadinessStatus::Blocked);
        assert!(
            readiness
                .blocker_codes
                .contains(&"provider_auth_ref_contains_secret_material".to_string())
        );
    }

    #[test]
    fn provider_config_validation_fails_closed_for_each_required_field() {
        let mut unsupported = config_from_knowledge_pack(&pack(), auth_ref());
        unsupported.provider = "other-provider".to_string();
        assert_eq!(
            validate_provider_config(&unsupported),
            Err(CoderContractError::UnsupportedProvider)
        );

        let mut missing_model = config_from_knowledge_pack(&pack(), auth_ref());
        missing_model.model_ref = "  ".to_string();
        assert_eq!(
            validate_provider_config(&missing_model),
            Err(CoderContractError::MissingModelRef)
        );

        let mut missing_profile = config_from_knowledge_pack(&pack(), auth_ref());
        missing_profile.model_profile_id = "".to_string();
        assert_eq!(
            validate_provider_config(&missing_profile),
            Err(CoderContractError::MissingModelProfile)
        );

        let mut missing_auth = config_from_knowledge_pack(&pack(), auth_ref());
        missing_auth.auth_ref.profile_ref = "\t".to_string();
        assert_eq!(
            validate_provider_config(&missing_auth),
            Err(CoderContractError::MissingAuthRef)
        );
    }

    #[test]
    fn provider_readiness_reports_independent_blockers_and_all_secret_markers() {
        let mut invalid = config_from_knowledge_pack(&pack(), auth_ref());
        invalid.provider = "other-provider".to_string();
        invalid.model_ref = " \t".to_string();
        invalid.model_profile_id.clear();
        invalid.auth_ref.profile_ref = "ToKeN=redacted".to_string();

        let readiness = provider_readiness(&invalid);
        assert_eq!(readiness.status, ReadinessStatus::Blocked);
        assert_eq!(
            readiness.blocker_codes,
            vec![
                "provider_id_not_vida_coder",
                "selected_model_ref_missing",
                "selected_model_profile_id_missing",
                "provider_auth_ref_contains_secret_material",
            ]
        );

        for marker in [
            "sk-live",
            "api_key=redacted",
            "apikey=redacted",
            "token=redacted",
            "secret=redacted",
        ] {
            let mut config = config_from_knowledge_pack(&pack(), auth_ref());
            config.auth_ref.profile_ref = marker.to_string();
            assert_eq!(
                validate_provider_config(&config),
                Err(CoderContractError::SecretMaterialInAuthRef),
                "marker must remain blocked: {marker}"
            );
        }
    }

    #[test]
    fn receipt_builder_records_scope_checked_touched_paths() {
        let pack = pack();
        let config = config_from_knowledge_pack(&pack, auth_ref());
        let receipt = build_receipt(
            &pack,
            &config,
            vec![ToolAuditRecord {
                tool_name: "guarded_patch".to_string(),
                status: "pass".to_string(),
                touched_paths: vec![
                    "crates/vida-coder/src/lib.rs".to_string(),
                    "Cargo.toml".to_string(),
                ],
            }],
            vec![VerificationEvidence {
                command: "cargo test -p vida-coder".to_string(),
                status: "pass".to_string(),
            }],
            Vec::new(),
            json!({"provider_receipt_id": "provider-run-1"}),
            "provider adapter contract passed",
        )
        .expect("receipt should build");

        assert_eq!(receipt.schema_version, RECEIPT_SCHEMA_VERSION);
        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.provider, PROVIDER_ID);
        assert_eq!(receipt.touched_paths.len(), 2);
        assert_eq!(receipt.verification[0].status, "pass");
    }

    #[test]
    fn config_and_receipt_preserve_reasoning_auth_blockers_and_deduplicated_paths() {
        let pack = pack();
        let auth = auth_ref();
        let config = config_from_knowledge_pack(&pack, auth.clone());
        assert_eq!(config.reasoning_effort.as_deref(), Some("medium"));
        assert_eq!(config.auth_ref, auth);

        let receipt = build_receipt(
            &pack,
            &config,
            vec![ToolAuditRecord {
                tool_name: "guarded_patch".to_string(),
                status: "blocked".to_string(),
                touched_paths: vec![
                    "Cargo.toml".to_string(),
                    "crates/vida-coder/src/lib.rs".to_string(),
                    "Cargo.toml".to_string(),
                ],
            }],
            vec![VerificationEvidence {
                command: "cargo test -p vida-coder".to_string(),
                status: "blocked".to_string(),
            }],
            vec!["verification_blocked".to_string()],
            json!({"provider_receipt_id": "provider-run-blocked"}),
            "blocked by provider verification",
        )
        .expect("receipt should preserve blocked contract");

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.touched_paths,
            vec![
                "Cargo.toml".to_string(),
                "crates/vida-coder/src/lib.rs".to_string()
            ]
        );
        assert_eq!(receipt.blockers, vec!["verification_blocked"]);
        assert_eq!(
            receipt.raw_provider["provider_receipt_id"],
            "provider-run-blocked"
        );
        assert_eq!(receipt.handoff_summary, "blocked by provider verification");
    }

    #[test]
    fn receipt_builder_rejects_out_of_scope_touched_paths() {
        let pack = pack();
        let config = config_from_knowledge_pack(&pack, auth_ref());
        let err = build_receipt(
            &pack,
            &config,
            vec![ToolAuditRecord {
                tool_name: "guarded_patch".to_string(),
                status: "blocked".to_string(),
                touched_paths: vec!["crates/vida/src/lib.rs".to_string()],
            }],
            Vec::new(),
            vec!["out_of_scope_write".to_string()],
            Value::Null,
            "blocked",
        )
        .expect_err("receipt should reject out-of-scope path");

        assert_eq!(
            err,
            CoderContractError::TouchedPathOutsideScope("crates/vida/src/lib.rs".to_string())
        );
    }

    #[test]
    fn runtime_tool_audit_envelope_feeds_coder_receipt() {
        let pack = pack();
        let config = config_from_knowledge_pack(&pack, auth_ref());
        let policy = vida_runtime_tools::PacketToolPolicy {
            owned_paths: pack.owned_paths.clone(),
            read_only_paths: pack.read_only_paths.clone(),
            allowed_tools: vec![vida_runtime_tools::TypedVidaTool::GuardedPatch],
        };
        let audit = vida_runtime_tools::validate_tool_request(
            &policy,
            "guarded_patch",
            &["crates/vida-coder/src/lib.rs".to_string()],
        )
        .expect("guarded patch should pass");

        let receipt = build_receipt(
            &pack,
            &config,
            vec![audit.into()],
            Vec::new(),
            Vec::new(),
            Value::Null,
            "runtime tool audit accepted",
        )
        .expect("runtime tool audit should feed receipt");

        assert_eq!(receipt.tool_audit[0].tool_name, "guarded_patch");
        assert_eq!(receipt.touched_paths, vec!["crates/vida-coder/src/lib.rs"]);
    }

    #[test]
    fn service_scheduler_represents_multiple_projects() {
        let mut scheduler = ServiceScheduler::default();
        scheduler
            .register_project(ServiceProjectRuntime {
                project_id: "vida-stack".to_string(),
                project_root: "C:/project/vida-stack".to_string(),
                state_dir: "C:/project/vida-stack/.vida/data/state".to_string(),
                vida_binary_fingerprint: "vida-fp-1".to_string(),
                config_hash: "cfg-1".to_string(),
            })
            .expect("first project should register");
        scheduler
            .register_project(ServiceProjectRuntime {
                project_id: "vida-mobile".to_string(),
                project_root: "C:/project/vida_mobile".to_string(),
                state_dir: "C:/project/vida_mobile/.vida/data/state".to_string(),
                vida_binary_fingerprint: "vida-fp-1".to_string(),
                config_hash: "cfg-2".to_string(),
            })
            .expect("second project should register");

        assert_eq!(scheduler.project_count(), 2);
    }

    #[test]
    fn service_scheduler_rejects_overlapping_conflict_domain() {
        let mut scheduler = ServiceScheduler::default();
        scheduler
            .register_project(ServiceProjectRuntime {
                project_id: "vida-stack".to_string(),
                project_root: "C:/project/vida-stack".to_string(),
                state_dir: "C:/project/vida-stack/.vida/data/state".to_string(),
                vida_binary_fingerprint: "vida-fp-1".to_string(),
                config_hash: "cfg-1".to_string(),
            })
            .expect("project should register");

        let first = ServiceWorkerLease {
            project_id: "vida-stack".to_string(),
            task_id: "task-a".to_string(),
            packet_id: "packet-a".to_string(),
            conflict_domain: Some("vida-service-scheduler".to_string()),
            parallel_group: None,
            expires_at_epoch_ms: 2_000,
            heartbeat_at_epoch_ms: 1_000,
        };
        scheduler
            .claim_worker(first)
            .expect("first domain lease should pass");

        let second = ServiceWorkerLease {
            project_id: "vida-stack".to_string(),
            task_id: "task-b".to_string(),
            packet_id: "packet-b".to_string(),
            conflict_domain: Some("vida-service-scheduler".to_string()),
            parallel_group: None,
            expires_at_epoch_ms: 2_000,
            heartbeat_at_epoch_ms: 1_000,
        };

        assert_eq!(
            scheduler.claim_worker(second),
            Err(CoderContractError::ConflictDomainAlreadyLeased(
                "vida-service-scheduler".to_string()
            ))
        );
        assert_eq!(scheduler.active_lease_count(), 1);
    }

    #[test]
    fn service_scheduler_rejects_duplicate_projects_and_unknown_claims() {
        let project = ServiceProjectRuntime {
            project_id: "vida-stack".to_string(),
            project_root: "C:/project/vida-stack".to_string(),
            state_dir: "C:/project/vida-stack/.vida/data/state".to_string(),
            vida_binary_fingerprint: "vida-fp-1".to_string(),
            config_hash: "cfg-1".to_string(),
        };
        let mut scheduler = ServiceScheduler::default();
        scheduler
            .register_project(project.clone())
            .expect("first project should register");
        assert_eq!(
            scheduler.register_project(project),
            Err(CoderContractError::ProjectAlreadyRegistered(
                "vida-stack".to_string()
            ))
        );

        let unknown_claim = ServiceWorkerLease {
            project_id: "unknown-project".to_string(),
            task_id: "task-unknown".to_string(),
            packet_id: "packet-unknown".to_string(),
            conflict_domain: None,
            parallel_group: None,
            expires_at_epoch_ms: 2_000,
            heartbeat_at_epoch_ms: 1_000,
        };
        assert_eq!(
            scheduler.claim_worker(unknown_claim),
            Err(CoderContractError::ProjectNotRegistered(
                "unknown-project".to_string()
            ))
        );
        assert_eq!(scheduler.active_lease_count(), 0);
    }
}
