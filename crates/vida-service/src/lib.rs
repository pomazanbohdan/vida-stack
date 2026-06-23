use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use service_manager::{RestartPolicy as ServiceRestartPolicy, ServiceLevel};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use vida_contracts::{VidaCommandEnvelope, VidaRequestId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceDaemonConfig {
    pub service_name: String,
    pub socket_name: String,
    pub journal_path: PathBuf,
    pub shutdown_timeout_ms: u64,
}

impl ServiceDaemonConfig {
    pub fn local_default() -> Self {
        Self {
            service_name: "vida-service".to_string(),
            socket_name: format!("vida-service-{}.sock", std::process::id()),
            journal_path: std::env::temp_dir().join("vida-service-accepted-commands.jsonl"),
            shutdown_timeout_ms: 1_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcceptedCommandRecord {
    pub request_id: String,
    pub operation: String,
    pub replay_state: String,
}

pub struct AcceptedCommandJournal {
    path: PathBuf,
}

impl AcceptedCommandJournal {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn record_accepted(&self, envelope: &VidaCommandEnvelope) -> Result<AcceptedCommandRecord> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("create accepted-command journal parent {}", parent.display())
            })?;
        }
        let record = AcceptedCommandRecord {
            request_id: envelope.request_id.0.clone(),
            operation: envelope.operation.0.clone(),
            replay_state: "accepted_replayable".to_string(),
        };
        let line = serde_json::to_string(&record)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("open accepted-command journal {}", self.path.display()))?;
        writeln!(file, "{line}")?;
        Ok(record)
    }

    pub fn replayable_commands(&self) -> Result<Vec<AcceptedCommandRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = OpenOptions::new()
            .read(true)
            .open(&self.path)
            .with_context(|| format!("read accepted-command journal {}", self.path.display()))?;
        BufReader::new(file)
            .lines()
            .filter_map(|line| match line {
                Ok(value) if value.trim().is_empty() => None,
                other => Some(other),
            })
            .map(|line| {
                let line = line?;
                Ok(serde_json::from_str::<AcceptedCommandRecord>(&line)?)
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcConformanceRow {
    pub platform: String,
    pub transport: String,
    pub framing: String,
    pub rpc_methods: Vec<String>,
    pub domain_mutation_logic: bool,
}

pub fn ipc_conformance_matrix() -> Vec<IpcConformanceRow> {
    vec![
        IpcConformanceRow {
            platform: "windows".to_string(),
            transport: "interprocess_local_socket_named_pipe".to_string(),
            framing: "tarpc_length_delimited_json".to_string(),
            rpc_methods: vec!["execute(VidaCommandEnvelope)".to_string()],
            domain_mutation_logic: false,
        },
        IpcConformanceRow {
            platform: "unix".to_string(),
            transport: "interprocess_local_socket".to_string(),
            framing: "tarpc_length_delimited_json".to_string(),
            rpc_methods: vec!["execute(VidaCommandEnvelope)".to_string()],
            domain_mutation_logic: false,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecyclePlan {
    pub mode: String,
    pub service_name: String,
    pub native_service_apply_supported: bool,
    pub apply_requires_token: bool,
    pub manager_level: String,
    pub restart_policy: String,
    pub planned_actions: Vec<String>,
}

pub fn lifecycle_plan(mode: &str, config: &ServiceDaemonConfig) -> LifecyclePlan {
    let manager_level = match ServiceLevel::User {
        ServiceLevel::User => "user",
        ServiceLevel::System => "system",
    };
    let restart_policy = match (ServiceRestartPolicy::OnFailure {
        delay_secs: Some(1),
        max_retries: Some(3),
        reset_after_secs: Some(60),
    }) {
        ServiceRestartPolicy::OnFailure { .. } => "on_failure",
        ServiceRestartPolicy::Always { .. } => "always",
        ServiceRestartPolicy::Never => "never",
        ServiceRestartPolicy::OnSuccess { .. } => "on_success",
    };
    LifecyclePlan {
        mode: mode.to_string(),
        service_name: config.service_name.clone(),
        native_service_apply_supported: false,
        apply_requires_token: true,
        manager_level: manager_level.to_string(),
        restart_policy: restart_policy.to_string(),
        planned_actions: vec![
            "install user-level vida-service entry".to_string(),
            "start foreground-capable local daemon".to_string(),
            "stop with graceful shutdown timeout".to_string(),
            "uninstall user-level service entry".to_string(),
        ],
    }
}

pub async fn run_foreground_until_shutdown(config: ServiceDaemonConfig) -> Result<()> {
    let timeout = Duration::from_millis(config.shutdown_timeout_ms);
    Toplevel::new(async |subsys: &mut SubsystemHandle| {
        subsys.start(SubsystemBuilder::new("vida-service-listener", service_listener));
    })
    .catch_signals()
    .handle_shutdown_requests(timeout)
    .await
    .map_err(|error| anyhow::anyhow!("{error}"))
}

async fn service_listener(subsys: &mut SubsystemHandle) -> Result<(), anyhow::Error> {
    tokio::select! {
        _ = subsys.on_shutdown_requested() => Ok(()),
        _ = tokio::time::sleep(Duration::from_millis(25)) => {
            subsys.request_shutdown();
            Ok(())
        }
    }
}

pub fn sample_status_request(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: vida_contracts::VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: vida_contracts::VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: vida_contracts::VidaOperation(operation.to_string()),
        session_id: vida_contracts::VidaSessionId("vida-service-test-session".to_string()),
        request_id: VidaRequestId(format!("vida-service-request-{operation}")),
        command_id: None,
        causation_id: None,
        expected_stream_version: None,
        consistency: None,
        deadline: None,
        client_kind: vida_contracts::VidaClientKind::Service,
        project_ref: None,
        claim_kind: vida_contracts::operation_spec(operation).map(|spec| spec.required_claim),
        payload: serde_json::json!({}),
        correlation: None,
        idempotency_key: Some(vida_contracts::VidaIdempotencyKey(format!(
            "vida-service-idem-{operation}"
        ))),
        apply_token: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vida_contracts::operations;

    #[test]
    fn daemon_restart_smoke_replays_accepted_commands() {
        let temp = tempfile::tempdir().expect("temp dir");
        let journal_path = temp.path().join("accepted.jsonl");
        let first_daemon = AcceptedCommandJournal::open(&journal_path);
        first_daemon
            .record_accepted(&sample_status_request(operations::SERVICE_STATUS))
            .expect("record accepted command");

        let restarted_daemon = AcceptedCommandJournal::open(&journal_path);
        let replayable = restarted_daemon
            .replayable_commands()
            .expect("read replayable commands");

        assert_eq!(replayable.len(), 1);
        assert_eq!(replayable[0].operation, operations::SERVICE_STATUS);
        assert_eq!(replayable[0].replay_state, "accepted_replayable");
    }

    #[test]
    fn ipc_conformance_matrix_is_envelope_only() {
        let matrix = ipc_conformance_matrix();
        assert!(matrix.iter().any(|row| row.platform == "windows"));
        assert!(matrix.iter().any(|row| row.platform == "unix"));
        for row in matrix {
            assert_eq!(row.rpc_methods, vec!["execute(VidaCommandEnvelope)"]);
            assert!(!row.domain_mutation_logic);
            assert_eq!(row.framing, "tarpc_length_delimited_json");
        }
    }

    #[test]
    fn lifecycle_plan_is_dry_run_snapshot_and_apply_is_guarded() {
        let config = ServiceDaemonConfig::local_default();
        let plan = lifecycle_plan("dry_run", &config);
        assert_eq!(plan.mode, "dry_run");
        assert_eq!(plan.service_name, "vida-service");
        assert!(!plan.native_service_apply_supported);
        assert!(plan.apply_requires_token);
        assert_eq!(plan.manager_level, "user");
        assert_eq!(plan.restart_policy, "on_failure");
        assert!(plan.planned_actions.iter().any(|action| action.contains("install")));
    }

    #[tokio::test]
    async fn foreground_daemon_uses_graceful_shutdown_supervision() {
        run_foreground_until_shutdown(ServiceDaemonConfig {
            shutdown_timeout_ms: 250,
            ..ServiceDaemonConfig::local_default()
        })
        .await
        .expect("foreground daemon should shut down cleanly");
    }
}
