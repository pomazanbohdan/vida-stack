use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use surrealdb::types::SurrealValue;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

pub(crate) const ENV_SESSION_ID: &str = "VIDA_ORCHESTRATOR_SESSION_ID";
pub(crate) const ENV_LEASE_ID: &str = "VIDA_ORCHESTRATOR_LEASE_ID";
pub(crate) const ENV_HOST_APP: &str = "VIDA_HOST_APP";
pub(crate) const ENV_HOST_THREAD_ID: &str = "VIDA_HOST_THREAD_ID";
pub(crate) const ENV_ACTIVE_BOUNDED_UNIT: &str = "VIDA_ACTIVE_BOUNDED_UNIT";
pub(crate) const ENV_PUBLICATION_REPOSITORY: &str = "VIDA_OWNER_PUBLICATION_REPOSITORY";
pub(crate) const ENV_PUBLICATION_ISSUE: &str = "VIDA_OWNER_PUBLICATION_ISSUE";
pub(crate) const DEFAULT_LEASE_TTL_SECONDS: i64 = 2 * 60 * 60;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue, PartialEq, Eq)]
pub(crate) struct OrchestratorSessionRecord {
    pub(crate) session_id: String,
    pub(crate) lease_id: String,
    pub(crate) state_root: String,
    pub(crate) project_root: String,
    pub(crate) workspace_fingerprint: String,
    pub(crate) execution_context_id: String,
    pub(crate) publication_context_id: String,
    pub(crate) host_app: String,
    pub(crate) host_thread_id: String,
    pub(crate) process_id: u32,
    pub(crate) active_bounded_unit: String,
    pub(crate) started_at: String,
    pub(crate) heartbeat_at: String,
    pub(crate) lease_expires_at: String,
    pub(crate) status: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct ExecutionContextIdentity {
    pub(crate) execution_context_id: String,
    pub(crate) project_root: String,
    pub(crate) state_root: String,
    pub(crate) current_executable: String,
    pub(crate) process_id: u32,
    pub(crate) workspace_fingerprint: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct OwnerPublicationContext {
    pub(crate) publication_context_id: String,
    pub(crate) repository: String,
    pub(crate) issue: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone)]
pub(crate) struct OrchestratorSessionDerivationInputs {
    pub(crate) state_root: PathBuf,
    pub(crate) project_root: PathBuf,
    pub(crate) current_executable: PathBuf,
    pub(crate) process_id: u32,
    pub(crate) now_utc: OffsetDateTime,
    pub(crate) lease_ttl_seconds: i64,
    pub(crate) env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct OrchestratorSessionDerivation {
    pub(crate) record: OrchestratorSessionRecord,
    pub(crate) execution_context_identity: ExecutionContextIdentity,
    pub(crate) owner_publication_context: OwnerPublicationContext,
}

pub(crate) fn derive_current_orchestrator_session(
    state_root: &Path,
    project_root: &Path,
) -> Result<OrchestratorSessionDerivation, String> {
    let env = std::env::vars().collect::<BTreeMap<_, _>>();
    let current_executable = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    derive_orchestrator_session(OrchestratorSessionDerivationInputs {
        state_root: state_root.to_path_buf(),
        project_root: project_root.to_path_buf(),
        current_executable,
        process_id: std::process::id(),
        now_utc: OffsetDateTime::now_utc(),
        lease_ttl_seconds: DEFAULT_LEASE_TTL_SECONDS,
        env,
    })
}

pub(crate) fn derive_orchestrator_session(
    inputs: OrchestratorSessionDerivationInputs,
) -> Result<OrchestratorSessionDerivation, String> {
    let state_root = normalized_path_string(&inputs.state_root);
    let project_root = normalized_path_string(&inputs.project_root);
    let current_executable = normalized_path_string(&inputs.current_executable);
    let host_app = env_value(&inputs.env, ENV_HOST_APP).unwrap_or("unknown_host_app");
    let host_thread_id =
        env_value(&inputs.env, ENV_HOST_THREAD_ID).unwrap_or("unknown_host_thread");
    let active_bounded_unit = env_value(&inputs.env, ENV_ACTIVE_BOUNDED_UNIT).unwrap_or("unknown");
    let workspace_fingerprint = prefixed_hash("workspace", &[&state_root, &project_root]);
    let execution_context_id = prefixed_hash(
        "execution-context",
        &[&project_root, &state_root, &current_executable],
    );
    let repository = env_value(&inputs.env, ENV_PUBLICATION_REPOSITORY).unwrap_or("unknown");
    let issue = env_value(&inputs.env, ENV_PUBLICATION_ISSUE).unwrap_or("unknown");
    let publication_source = if repository == "unknown" && issue == "unknown" {
        "default_unknown"
    } else {
        "environment"
    };
    let publication_context_id = prefixed_hash("publication-context", &[repository, issue]);
    let session_id = env_value(&inputs.env, ENV_SESSION_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            prefixed_hash(
                "orchestrator-session",
                &[
                    &workspace_fingerprint,
                    host_app,
                    host_thread_id,
                    active_bounded_unit,
                ],
            )
        });
    let lease_id = env_value(&inputs.env, ENV_LEASE_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            prefixed_hash(
                "orchestrator-lease",
                &[
                    &session_id,
                    &execution_context_id,
                    &inputs.process_id.to_string(),
                ],
            )
        });
    let heartbeat_at = format_rfc3339(inputs.now_utc)?;
    let lease_expires_at =
        format_rfc3339(inputs.now_utc + Duration::seconds(inputs.lease_ttl_seconds.max(1)))?;
    let record = OrchestratorSessionRecord {
        session_id,
        lease_id,
        state_root: state_root.clone(),
        project_root: project_root.clone(),
        workspace_fingerprint: workspace_fingerprint.clone(),
        execution_context_id: execution_context_id.clone(),
        publication_context_id: publication_context_id.clone(),
        host_app: host_app.to_string(),
        host_thread_id: host_thread_id.to_string(),
        process_id: inputs.process_id,
        active_bounded_unit: active_bounded_unit.to_string(),
        started_at: heartbeat_at.clone(),
        heartbeat_at: heartbeat_at.clone(),
        lease_expires_at,
        status: "active".to_string(),
    };
    Ok(OrchestratorSessionDerivation {
        record,
        execution_context_identity: ExecutionContextIdentity {
            execution_context_id,
            project_root,
            state_root,
            current_executable,
            process_id: inputs.process_id,
            workspace_fingerprint,
        },
        owner_publication_context: OwnerPublicationContext {
            publication_context_id,
            repository: repository.to_string(),
            issue: issue.to_string(),
            source: publication_source.to_string(),
        },
    })
}

pub(crate) fn build_orchestrator_session_surface(
    derivation: &OrchestratorSessionDerivation,
    records: &[OrchestratorSessionRecord],
    now_utc: OffsetDateTime,
) -> serde_json::Value {
    let mut active_sessions = Vec::new();
    let mut stale_sessions = Vec::new();
    for record in records {
        let summary = session_summary(record, now_utc);
        if record.session_id == derivation.record.session_id {
            continue;
        }
        if summary["lease_status"].as_str() == Some("other_owner_stale") {
            stale_sessions.push(summary);
        } else {
            active_sessions.push(summary);
        }
    }
    serde_json::json!({
        "status": "current_owner",
        "lease_status": "current_owner",
        "heartbeat_recorded": records.iter().any(|record| {
            record.session_id == derivation.record.session_id
                && record.lease_id == derivation.record.lease_id
        }),
        "current_owner": derivation.record,
        "selected_owner_evidence": crate::state_store::RuntimeOwnerEvidence::current_session(
            &derivation.record.session_id,
            &derivation.record.lease_id,
            &derivation.record.execution_context_id,
            &derivation.record.publication_context_id,
            &derivation.record.heartbeat_at,
        ),
        "execution_context_identity": derivation.execution_context_identity,
        "owner_publication_context": derivation.owner_publication_context,
        "active_orchestrator_sessions": active_sessions,
        "stale_orchestrator_sessions": stale_sessions,
        "legacy_owner_default": crate::state_store::RuntimeOwnerEvidence::legacy_global_owner_unknown(),
        "next_actions": [
            "Phase 1 reports session/lease identity only; session-aware latest gating, reclaim, and transfer commands are Phase 2/3 work."
        ],
    })
}

pub(crate) fn lease_status(record: &OrchestratorSessionRecord, now_utc: OffsetDateTime) -> String {
    match OffsetDateTime::parse(&record.lease_expires_at, &Rfc3339) {
        Ok(expires_at) if expires_at < now_utc => "other_owner_stale".to_string(),
        Ok(_) => "other_owner_live".to_string(),
        Err(_) => "other_owner_lease_unknown".to_string(),
    }
}

fn session_summary(
    record: &OrchestratorSessionRecord,
    now_utc: OffsetDateTime,
) -> serde_json::Value {
    serde_json::json!({
        "session_id": record.session_id,
        "lease_id": record.lease_id,
        "lease_status": lease_status(record, now_utc),
        "host_app": record.host_app,
        "host_thread_id": record.host_thread_id,
        "process_id": record.process_id,
        "active_bounded_unit": record.active_bounded_unit,
        "heartbeat_at": record.heartbeat_at,
        "lease_expires_at": record.lease_expires_at,
        "status": record.status,
    })
}

fn env_value<'a>(env: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    env.get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn normalized_path_string(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn prefixed_hash(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    let digest = hasher.finalize().to_hex().to_string();
    format!("{prefix}-{}", &digest[..16])
}

fn format_rfc3339(value: OffsetDateTime) -> Result<String, String> {
    value
        .format(&Rfc3339)
        .map_err(|error| format!("failed to format orchestrator session timestamp: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_time() -> OffsetDateTime {
        OffsetDateTime::parse("2026-05-07T00:00:00Z", &Rfc3339).expect("fixed time parses")
    }

    #[test]
    fn orchestrator_session_identity_derivation_uses_env_and_clock_overrides() {
        let mut env = BTreeMap::new();
        env.insert(ENV_SESSION_ID.to_string(), "session-test".to_string());
        env.insert(ENV_LEASE_ID.to_string(), "lease-test".to_string());
        env.insert(ENV_HOST_APP.to_string(), "codex-desktop".to_string());
        env.insert(ENV_HOST_THREAD_ID.to_string(), "thread-123".to_string());
        env.insert(
            ENV_ACTIVE_BOUNDED_UNIT.to_string(),
            "github-116-orchestrator-session-identity".to_string(),
        );
        env.insert(
            ENV_PUBLICATION_REPOSITORY.to_string(),
            "pomazanbohdan/vida-stack".to_string(),
        );
        env.insert(ENV_PUBLICATION_ISSUE.to_string(), "116".to_string());

        let derivation = derive_orchestrator_session(OrchestratorSessionDerivationInputs {
            state_root: PathBuf::from("C:/tmp/state"),
            project_root: PathBuf::from("C:/tmp/project"),
            current_executable: PathBuf::from("C:/tmp/vida.exe"),
            process_id: 42,
            now_utc: fixed_time(),
            lease_ttl_seconds: 60,
            env,
        })
        .expect("identity should derive");

        assert_eq!(derivation.record.session_id, "session-test");
        assert_eq!(derivation.record.lease_id, "lease-test");
        assert_eq!(derivation.record.host_app, "codex-desktop");
        assert_eq!(derivation.record.host_thread_id, "thread-123");
        assert_eq!(
            derivation.record.active_bounded_unit,
            "github-116-orchestrator-session-identity"
        );
        assert_eq!(derivation.record.heartbeat_at, "2026-05-07T00:00:00Z");
        assert_eq!(derivation.record.lease_expires_at, "2026-05-07T00:01:00Z");
        assert_eq!(
            derivation.owner_publication_context.repository,
            "pomazanbohdan/vida-stack"
        );
        assert_eq!(derivation.owner_publication_context.issue, "116");
    }

    #[test]
    fn orchestrator_session_identity_derivation_is_deterministic_without_env_session() {
        let inputs = OrchestratorSessionDerivationInputs {
            state_root: PathBuf::from("C:/tmp/state"),
            project_root: PathBuf::from("C:/tmp/project"),
            current_executable: PathBuf::from("C:/tmp/vida.exe"),
            process_id: 42,
            now_utc: fixed_time(),
            lease_ttl_seconds: 60,
            env: BTreeMap::new(),
        };
        let first = derive_orchestrator_session(inputs.clone()).expect("first derives");
        let second = derive_orchestrator_session(inputs).expect("second derives");

        assert_eq!(first.record.session_id, second.record.session_id);
        assert_eq!(first.record.lease_id, second.record.lease_id);
        assert!(first.record.session_id.starts_with("orchestrator-session-"));
        assert!(first.record.lease_id.starts_with("orchestrator-lease-"));
    }

    #[test]
    fn orchestrator_session_surface_classifies_sibling_leases() {
        let derivation = derive_orchestrator_session(OrchestratorSessionDerivationInputs {
            state_root: PathBuf::from("C:/tmp/state"),
            project_root: PathBuf::from("C:/tmp/project"),
            current_executable: PathBuf::from("C:/tmp/vida.exe"),
            process_id: 42,
            now_utc: fixed_time(),
            lease_ttl_seconds: 60,
            env: BTreeMap::new(),
        })
        .expect("identity should derive");
        let mut stale = derivation.record.clone();
        stale.session_id = "other-stale".to_string();
        stale.lease_id = "lease-stale".to_string();
        stale.lease_expires_at = "2026-05-06T23:59:00Z".to_string();
        let mut live = derivation.record.clone();
        live.session_id = "other-live".to_string();
        live.lease_id = "lease-live".to_string();
        live.lease_expires_at = "2026-05-07T00:10:00Z".to_string();

        let surface = build_orchestrator_session_surface(
            &derivation,
            &[derivation.record.clone(), stale, live],
            fixed_time(),
        );

        assert_eq!(surface["status"], "current_owner");
        assert_eq!(surface["heartbeat_recorded"], true);
        assert_eq!(
            surface["active_orchestrator_sessions"][0]["lease_status"],
            "other_owner_live"
        );
        assert_eq!(
            surface["stale_orchestrator_sessions"][0]["lease_status"],
            "other_owner_stale"
        );
    }
}
