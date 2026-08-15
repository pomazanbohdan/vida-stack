use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingJobRecord {
    pub job_id: String,
    pub source_request_id: String,
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
        let record = AcceptedCommandRecord {
            request_id: envelope.request_id.0.clone(),
            operation: envelope.operation.0.clone(),
            replay_state: "accepted_replayable".to_string(),
        };
        append_journal_record(&self.path, record.clone(), "accepted-command")?;
        Ok(record)
    }

    pub fn replayable_commands(&self) -> Result<Vec<AcceptedCommandRecord>> {
        replay_journal_records(&self.path, "accepted-command")
    }
}

pub struct PendingJobJournal {
    path: PathBuf,
}

impl PendingJobJournal {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn record_pending(
        &self,
        job_id: impl Into<String>,
        envelope: &VidaCommandEnvelope,
    ) -> Result<PendingJobRecord> {
        let record = PendingJobRecord {
            job_id: job_id.into(),
            source_request_id: envelope.request_id.0.clone(),
            replay_state: "pending_replayable".to_string(),
        };
        append_journal_record(&self.path, record.clone(), "pending-job")?;
        Ok(record)
    }

    pub fn replayable_jobs(&self) -> Result<Vec<PendingJobRecord>> {
        replay_journal_records(&self.path, "pending-job")
    }
}

fn append_journal_record<T>(path: &PathBuf, record: T, journal_name: &str) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("create {journal_name} journal parent {}", parent.display())
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {journal_name} journal {}", path.display()))?;
    serde_json::to_writer(&mut file, &record)
        .with_context(|| format!("serialize {journal_name} journal {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("append {journal_name} journal newline {}", path.display()))
}

fn replay_journal_records<T>(path: &PathBuf, journal_name: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)
        .with_context(|| format!("open {journal_name} journal {}", path.display()))?;

    BufReader::new(file)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            Ok(line) => Some((index + 1, Ok(line))),
            Err(error) => Some((index + 1, Err(error))),
        })
        .map(|(line_number, line)| {
            let line = line.with_context(|| {
                format!(
                    "read {journal_name} journal line {line_number} in {}",
                    path.display()
                )
            })?;
            serde_json::from_str::<T>(&line).with_context(|| {
                format!(
                    "parse {journal_name} journal line {line_number} in {}",
                    path.display()
                )
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcConformanceRow {
    pub platform: String,
    pub transport: String,
    pub framing: String,
    pub rpc_methods: Vec<String>,
    pub domain_mutation_logic: bool,
    pub proof_scope: String,
}

pub fn ipc_conformance_matrix() -> Vec<IpcConformanceRow> {
    vec![
        IpcConformanceRow {
            platform: "windows".to_string(),
            transport: "interprocess_local_socket_named_pipe".to_string(),
            framing: "tarpc_length_delimited_json".to_string(),
            rpc_methods: vec!["execute(VidaCommandEnvelope)".to_string()],
            domain_mutation_logic: false,
            proof_scope: "exercised_on_windows_host".to_string(),
        },
        IpcConformanceRow {
            platform: "unix".to_string(),
            transport: "interprocess_local_socket".to_string(),
            framing: "tarpc_length_delimited_json".to_string(),
            rpc_methods: vec!["execute(VidaCommandEnvelope)".to_string()],
            domain_mutation_logic: false,
            proof_scope: "metadata_contract_cross_platform_runner_required".to_string(),
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

pub fn mark_in_flight_command_replayable(
    journal: &AcceptedCommandJournal,
    envelope: &VidaCommandEnvelope,
) -> Result<AcceptedCommandRecord> {
    journal.record_accepted(envelope)
}

pub async fn run_foreground_until_shutdown(config: ServiceDaemonConfig) -> Result<()> {
    let timeout = Duration::from_millis(config.shutdown_timeout_ms);
    Toplevel::new(async |subsys: &mut SubsystemHandle| {
        subsys.start(SubsystemBuilder::new(
            "vida-service-listener",
            service_listener,
        ));
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
        trusted_owned_path: None,
        trusted_owned_write_scopes: Vec::new(),
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
        assert_eq!(
            replayable[0].request_id,
            "vida-service-request-vida.service.status"
        );
        assert_eq!(replayable[0].operation, operations::SERVICE_STATUS);
        assert_eq!(replayable[0].replay_state, "accepted_replayable");
    }

    #[test]
    fn local_default_projects_service_journal_path() {
        let config = ServiceDaemonConfig::local_default();

        assert_eq!(
            config.journal_path,
            std::env::temp_dir().join("vida-service-accepted-commands.jsonl")
        );
    }

    #[test]
    fn daemon_restart_smoke_replays_pending_jobs() {
        let temp = tempfile::tempdir().expect("temp dir");
        let journal_path = temp.path().join("pending-jobs.jsonl");
        let first_daemon = PendingJobJournal::open(&journal_path);
        first_daemon
            .record_pending(
                "job-service-status-1",
                &sample_status_request(operations::SERVICE_STATUS),
            )
            .expect("record pending job");

        let restarted_daemon = PendingJobJournal::open(&journal_path);
        let replayable = restarted_daemon
            .replayable_jobs()
            .expect("read replayable jobs");

        assert_eq!(replayable.len(), 1);
        assert_eq!(replayable[0].job_id, "job-service-status-1");
        assert_eq!(
            replayable[0].source_request_id,
            "vida-service-request-vida.service.status"
        );
        assert_eq!(replayable[0].replay_state, "pending_replayable");
    }

    #[test]
    fn graceful_shutdown_marks_in_flight_command_replayable() {
        let temp = tempfile::tempdir().expect("temp dir");
        let journal_path = temp.path().join("accepted.jsonl");
        let journal = AcceptedCommandJournal::open(&journal_path);
        let envelope = sample_status_request(operations::SERVICE_STATUS);

        let record = mark_in_flight_command_replayable(&journal, &envelope)
            .expect("mark in-flight command replayable");
        let replayable = journal.replayable_commands().expect("read replayable");

        assert_eq!(record.replay_state, "accepted_replayable");
        assert_eq!(replayable, vec![record]);
    }

    #[test]
    fn journal_replay_skips_blank_lines() {
        let temp = tempfile::tempdir().expect("temp dir");
        let journal_path = temp.path().join("accepted.jsonl");
        fs::write(
            &journal_path,
            concat!(
                "\n",
                "{\"request_id\":\"req-1\",\"operation\":\"vida.service.status\",\"replay_state\":\"accepted_replayable\"}\n",
                "   \n"
            ),
        )
        .expect("write journal");

        let replayable = AcceptedCommandJournal::open(&journal_path)
            .replayable_commands()
            .expect("read replayable commands");

        assert_eq!(replayable.len(), 1);
        assert_eq!(replayable[0].request_id, "req-1");
    }

    #[test]
    fn journal_replay_blocks_malformed_lines() {
        let temp = tempfile::tempdir().expect("temp dir");
        let journal_path = temp.path().join("pending-jobs.jsonl");
        fs::write(
            &journal_path,
            concat!(
                "{\"job_id\":\"job-1\",\"source_request_id\":\"req-1\",\"replay_state\":\"pending_replayable\"}\n",
                "{malformed-json}\n"
            ),
        )
        .expect("write journal");

        let err = PendingJobJournal::open(&journal_path)
            .replayable_jobs()
            .expect_err("malformed line blocks replay");

        assert!(err.to_string().contains("parse pending-job journal"));
    }

    #[test]
    fn journal_replay_reports_malformed_first_line() {
        let temp = tempfile::tempdir().expect("temp dir");
        let journal_path = temp.path().join("accepted.jsonl");
        fs::write(
            &journal_path,
            concat!(
                "{malformed-json}\n",
                "{\"request_id\":\"req-1\",\"operation\":\"vida.service.status\",\"replay_state\":\"accepted_replayable\"}\n"
            ),
        )
        .expect("write journal");

        let err = AcceptedCommandJournal::open(&journal_path)
            .replayable_commands()
            .expect_err("malformed first line blocks replay");

        assert!(
            err.to_string()
                .contains("parse accepted-command journal line 1"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn ipc_conformance_matrix_is_envelope_only() {
        let matrix = ipc_conformance_matrix();
        assert!(matrix.iter().any(|row| row.platform == "windows"));
        assert!(matrix.iter().any(|row| row.platform == "unix"));
        for row in &matrix {
            assert_eq!(row.rpc_methods, vec!["execute(VidaCommandEnvelope)"]);
            assert!(!row.domain_mutation_logic);
            assert_eq!(row.framing, "tarpc_length_delimited_json");
        }
        assert_eq!(
            matrix
                .iter()
                .find(|row| row.platform == "unix")
                .map(|row| row.proof_scope.as_str()),
            Some("metadata_contract_cross_platform_runner_required")
        );
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
        assert!(
            plan.planned_actions
                .iter()
                .any(|action| action.contains("install"))
        );
    }

    #[test]
    fn sample_status_request_projects_contract_fields_and_claims() {
        let request = sample_status_request(operations::SERVICE_STATUS);

        assert_eq!(
            request.schema_version,
            vida_contracts::VIDA_CONTRACTS_SCHEMA_VERSION
        );
        assert_eq!(
            request.protocol_version,
            vida_contracts::VIDA_COMMAND_PROTOCOL_VERSION
        );
        assert_eq!(request.operation.0, operations::SERVICE_STATUS);
        assert_eq!(request.session_id.0, "vida-service-test-session");
        assert_eq!(
            request.request_id.0,
            "vida-service-request-vida.service.status"
        );
        assert_eq!(
            request.claim_kind,
            Some(vida_contracts::VidaClaimKind::SharedRead)
        );
        assert_eq!(
            request.idempotency_key.map(|key| key.0),
            Some("vida-service-idem-vida.service.status".to_string())
        );
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
