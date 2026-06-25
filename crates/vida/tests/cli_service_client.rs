use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use taskflow_contracts::{
    VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef,
    VidaEventRef, VidaIdempotencyKey, VidaOperation, VidaSchemaId, VidaSchemaVersion,
    VidaStreamRef, VidaStreamVersion, VidaTimestamp,
};
use taskflow_state::{JournalAppendRequest, OperationalJournal};
use taskflow_state_redb::RedbOperationalJournal;

fn vida() -> Command {
    vida_test_support::bounded_binary_command(env!("CARGO_BIN_EXE_vida"))
}

fn run_json(args: &[&str]) -> serde_json::Value {
    let output = vida()
        .args(args)
        .output()
        .expect("vida command should execute");
    assert!(
        output.status.success(),
        "command {args:?} should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "command {args:?} should emit JSON ({error}): stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn run_json_with_env(args: &[&str], key: &str, value: &PathBuf) -> serde_json::Value {
    let output = vida()
        .env(key, value)
        .args(args)
        .output()
        .expect("vida command should execute");
    assert!(
        output.status.success(),
        "command {args:?} should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "command {args:?} should emit JSON ({error}): stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn cli_service_first_families_emit_vida_client_operations() {
    let cases = [
        (
            &["service", "status", "--json"][..],
            "service",
            "vida.service.status",
        ),
        (
            &["service", "hello", "--json"][..],
            "service",
            "vida.service.hello",
        ),
        (
            &["service", "endpoints", "--json"][..],
            "service",
            "vida.service.endpoint.status",
        ),
        (
            &["service", "endpoint-status", "--json"][..],
            "service",
            "vida.service.endpoint.status",
        ),
        (
            &["service", "capabilities", "--json"][..],
            "service",
            "vida.service.capabilities",
        ),
        (
            &["service", "lifecycle-plan", "--json"][..],
            "service",
            "vida.service.lifecycle.plan",
        ),
        (
            &["service", "lifecycle-status", "--json"][..],
            "service",
            "vida.service.lifecycle.status",
        ),
        (
            &["service", "events", "--json"][..],
            "service",
            "vida.events.since",
        ),
        (
            &["project", "list", "--json"][..],
            "project",
            "vida.project.registry.list",
        ),
        (
            &["project", "resolve", "--project", "vida-stack", "--json"][..],
            "project",
            "vida.project.resolve",
        ),
        (
            &["project", "status", "--project", "vida-stack", "--json"][..],
            "project",
            "vida.project.status",
        ),
        (
            &["wizard", "inspect", "--project", "vida-stack", "--json"][..],
            "wizard",
            "vida.wizard.schema.get",
        ),
        (
            &["wizard", "draft", "--project", "vida-stack", "--json"][..],
            "wizard",
            "vida.wizard.session.start",
        ),
        (
            &["wizard", "validate", "--project", "vida-stack", "--json"][..],
            "wizard",
            "vida.wizard.session.validate",
        ),
        (
            &["wizard", "diff", "--project", "vida-stack", "--json"][..],
            "wizard",
            "vida.wizard.session.diff",
        ),
        (&["job", "status", "--json"][..], "job", "vida.jobs.get"),
        (
            &["receipt", "get", "--project", "vida-stack", "--json"][..],
            "receipt",
            "vida.receipts.get",
        ),
    ];

    for (args, family, operation) in cases {
        let payload = run_json(args);
        assert_eq!(payload["family"], family);
        assert_eq!(payload["operation"], operation);
        assert_eq!(payload["status"], "pass");
    }
}

#[test]
fn cli_job_status_default_output_is_actionable_without_json() {
    let journal_path = persisted_outbox_journal("cli-job-status");
    let output = vida()
        .env("VIDA_JOB_JOURNAL_PATH", &journal_path)
        .args(["job", "status"])
        .output()
        .expect("job status command should execute");
    assert!(
        output.status.success(),
        "job status should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida job status"));
    assert!(stdout.contains("job_id: vida-effect-effect-1"));
    assert!(stdout.contains("job_status: retryable"));
    assert!(stdout.contains("authority: redb_outbox"));
    assert!(stdout.contains("runner: effectum"));
    assert!(stdout.contains("next_action: schedule_retry_from_redb_outbox"));
    assert!(!stdout.contains("--json"));

    let json = run_json_with_env(
        &["job", "status", "--json"],
        "VIDA_JOB_JOURNAL_PATH",
        &journal_path,
    );
    assert_eq!(json["response"]["result"]["status"], "retryable");
    assert_eq!(
        json["response"]["result"]["job"]["next_action"],
        "schedule_retry_from_redb_outbox"
    );
    assert_eq!(
        json["response"]["result"]["job"]["authority"],
        "redb_outbox"
    );
}

#[test]
fn cli_job_status_default_output_escapes_blocker_repair_control_characters() {
    let journal_path = std::env::temp_dir().join("missing-vida-job-\u{1b}]52;c;SGFja2Vk\u{7}.redb");
    let output = vida()
        .env("VIDA_JOB_JOURNAL_PATH", &journal_path)
        .args(["job", "status"])
        .output()
        .expect("job status command should execute");
    assert!(
        output.status.success(),
        "job status should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("job_status: unavailable"));
    assert!(stdout.contains("repair_action: redb outbox journal"));
    assert!(stdout.contains("\\u{1b}]52;c;SGFja2Vk\\u{7}"));
    assert!(!stdout.contains('\u{1b}'));
    assert!(!stdout.contains('\u{7}'));
}

#[test]
fn cli_job_status_default_output_exposes_blocker_repair_without_json() {
    let output = vida()
        .env(
            "VIDA_JOB_JOURNAL_PATH",
            std::env::temp_dir().join("missing-vida-job-journal.redb"),
        )
        .args(["job", "status"])
        .output()
        .expect("job status command should execute");
    assert!(
        output.status.success(),
        "job status should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("job_status: unavailable"));
    assert!(stdout.contains("code: vida_job_journal_unavailable"));
    assert!(stdout.contains("repair_action: redb outbox journal"));
    assert!(!stdout.contains("--json"));
}

#[test]
fn cli_service_capabilities_default_output_is_actionable_without_json() {
    let output = vida()
        .args(["service", "capabilities"])
        .output()
        .expect("service capabilities command should execute");
    assert!(
        output.status.success(),
        "service capabilities should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vida service capabilities"));
    assert!(stdout.contains("engine_id: vida-runtime-local"));
    assert!(stdout.contains("engine_kind: local_redb_effectum"));
    assert!(stdout.contains("engine_contract: vida-runtime-engine-v1"));
    assert!(stdout.contains("capabilities[7]{capability,supported,mode,blocker_code}:"));
    assert!(stdout.contains("jobs,true,redb_outbox_effectum,"));
    assert!(stdout.contains("durable_timers,false,unsupported,unsupported_engine_capability"));
    assert!(!stdout.contains("--json"));

    let json = run_json(&["service", "capabilities", "--json"]);
    assert_eq!(
        json["response"]["result"]["engine_capabilities"]["engine_id"],
        "vida-runtime-local"
    );
    assert_eq!(
        json["response"]["result"]["engine_capabilities"]["contract_version"],
        "vida-runtime-engine-v1"
    );
    assert_eq!(
        json["response"]["result"]["operation_catalog"][0]["input_schema"]["fields"]
            .as_array()
            .expect("catalog entry should expose input fields")
            .len(),
        0
    );
}

#[test]
fn cli_operation_help_matches_contract_schema_catalog() {
    let help = vida()
        .args(["wizard", "inspect", "--help"])
        .output()
        .expect("wizard inspect help should execute");
    assert!(
        help.status.success(),
        "wizard inspect help should succeed: stderr={}",
        String::from_utf8_lossy(&help.stderr)
    );
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("vida wizard inspect"));
    assert!(stdout.contains("operation: vida.wizard.schema.get"));
    assert!(stdout.contains("inputs[2]{field_id,label,required,default,cli,control}:"));
    assert!(stdout.contains("project,Project,true,vida-stack,--project,text_input"));
    assert!(stdout.contains("wizard_kind,Wizard kind,false,project_init,--kind,select"));

    let endpoints = run_json(&["service", "endpoints", "--json"]);
    let endpoints = endpoints["response"]["result"]["endpoints"]
        .as_array()
        .expect("endpoints array");
    let wizard = endpoints
        .iter()
        .find(|entry| entry["operation"] == "vida.wizard.schema.get")
        .expect("wizard endpoint catalog entry");
    let fields = wizard["input_schema"]["fields"]
        .as_array()
        .expect("input schema fields");
    assert_eq!(fields[0]["field_id"], "project");
    assert_eq!(fields[0]["cli_flag"], "--project");
    assert_eq!(fields[0]["tui_control"], "text_input");
    assert_eq!(fields[1]["field_id"], "wizard_kind");
    assert_eq!(fields[1]["cli_flag"], "--kind");
    assert_eq!(fields[1]["tui_control"], "select");
}

#[test]
fn cli_help_description_inventory_covers_service_first_proxy_commands() {
    for (args, expected) in [
        (
            &["service", "--help"][..],
            "vida service endpoint-status --json",
        ),
        (&["service", "--help"][..], "vida service hello --json"),
        (
            &["project", "--help"][..],
            "vida project resolve --project <project-id> --json",
        ),
        (&["wizard", "--help"][..], "vida wizard validate --json"),
        (&["job", "--help"][..], "vida job status"),
        (&["receipt", "--help"][..], "vida receipt get --json"),
    ] {
        let output = vida()
            .args(args)
            .output()
            .expect("help command should execute");
        assert!(
            output.status.success(),
            "help command {args:?} should succeed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected),
            "help command {args:?} should describe `{expected}`:\n{stdout}"
        );
        assert!(
            stdout.contains("--json"),
            "help command {args:?} should describe --json:\n{stdout}"
        );
    }
}

fn persisted_outbox_journal(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("vida-{label}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create journal fixture dir");
    let path = dir.join("journal.redb");
    let mut journal = RedbOperationalJournal::create(&path).expect("create redb journal");
    journal
        .append(JournalAppendRequest {
            stream_id: VidaStreamRef("stream-1".to_string()),
            expected_stream_version: Some(VidaStreamVersion(0)),
            command_id: VidaCommandRef("command-1".to_string()),
            idempotency_key: VidaIdempotencyKey("idem-1".to_string()),
            causation_id: Some(VidaCommandRef("command-1".to_string())),
            correlation_id: Some("correlation-1".to_string()),
            events: vec![VidaDomainEventEnvelope {
                schema_id: VidaSchemaId("schema.task.updated".to_string()),
                event_version: VidaSchemaVersion(1),
                event_id: VidaEventRef("event-1".to_string()),
                command_id: Some(VidaCommandRef("command-1".to_string())),
                causation_id: Some(VidaCommandRef("command-1".to_string())),
                stream_id: VidaStreamRef("stream-1".to_string()),
                stream_version: VidaStreamVersion(1),
                aggregate_id: VidaAggregateRef("task-1".to_string()),
                occurred_at: VidaTimestamp("2026-06-23T00:00:00Z".to_string()),
                payload: serde_json::json!({ "stream_version": 1 }),
                trace: serde_json::json!({ "correlation_id": "correlation-1" }),
            }],
            effect_intents: vec![VidaEffectIntent {
                effect_id: VidaEffectRef("effect-1".to_string()),
                operation: VidaOperation("vida.effect.dispatch".to_string()),
                command_id: VidaCommandRef("command-1".to_string()),
                stream_id: VidaStreamRef("stream-1".to_string()),
                payload: serde_json::json!({ "effect_id": "effect-1" }),
            }],
        })
        .expect("append effect intent");
    let claimed = journal.claim_outbox_batch("effectum-worker-1", 1);
    journal
        .mark_outbox_failed(&claimed[0].outbox_id, "transport failure".to_string())
        .expect("mark failed");
    path
}

#[test]
fn cli_taskflow_and_docflow_remain_direct_proxy_families() {
    let taskflow = vida()
        .args(["taskflow", "help"])
        .output()
        .expect("taskflow help should execute");
    assert!(
        taskflow.status.success(),
        "taskflow help should remain direct: stderr={}",
        String::from_utf8_lossy(&taskflow.stderr)
    );
    let taskflow_stdout = String::from_utf8_lossy(&taskflow.stdout);
    assert!(
        taskflow_stdout.contains("TaskFlow"),
        "taskflow help should render TaskFlow output, got {taskflow_stdout}"
    );

    let docflow = vida()
        .args(["docflow", "help"])
        .output()
        .expect("docflow help should execute");
    assert!(
        docflow.status.success(),
        "docflow help should remain direct: stderr={}",
        String::from_utf8_lossy(&docflow.stderr)
    );
    let docflow_stdout = String::from_utf8_lossy(&docflow.stdout);
    assert!(
        docflow_stdout.contains("DocFlow") || docflow_stdout.contains("docflow"),
        "docflow help should render DocFlow output, got {docflow_stdout}"
    );
}
