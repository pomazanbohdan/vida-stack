use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::host_runtime_registry::looks_like_host_runtime_source_root;
use crate::state_store::StateStore;
use crate::surface_render::print_compact_command_families;

use super::{
    build_runtime_lane_selection_with_store, ensure_launcher_bootstrap, normalize_root_arg,
    print_surface_header, print_surface_line, role_exists_in_lane_bundle, state_store,
    sync_launcher_activation_snapshot, AgentInitArgs, BootArgs, InitArgs, RenderMode,
};
use crate::taskflow_runtime_bundle::build_taskflow_consume_bundle_payload;

const DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS: u64 = 10;

async fn best_effort_record_agent_init_dispatch_timeout_receipt(
    state_root: &Path,
    run_graph_bootstrap: &serde_json::Value,
    receipt: &crate::state_store::RunGraphDispatchReceipt,
    execute_dispatch_timeout_seconds: u64,
) -> Option<String> {
    let store = match tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
        StateStore::open_existing(state_root.to_path_buf()),
    )
    .await
    {
        Ok(Ok(store)) => store,
        Ok(Err(error)) => {
            return Some(format!(
                "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to reopen authoritative state store: {error}"
            ));
        }
        Err(_) => {
            return Some(format!(
                "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: timed out reopening authoritative state store after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
            ));
        }
    };
    if let Err(error) = store.record_run_graph_dispatch_receipt(receipt).await {
        return Some(format!(
            "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to persist timeout-blocked dispatch receipt: {error}"
        ));
    }
    if let Some(run_id) = run_graph_bootstrap
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    {
        match store.run_graph_status(run_id).await {
            Ok(status) => {
                if let Err(error) =
                    crate::taskflow_continuation::sync_run_graph_continuation_binding(
                        &store,
                        &status,
                        "agent_init_execute_dispatch_timeout",
                    )
                    .await
                {
                    return Some(format!(
                        "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to synchronize continuation binding after timeout: {error}"
                    ));
                }
            }
            Err(error) => {
                return Some(format!(
                    "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion; authoritative timeout reconciliation deferred until next safe reopen: failed to read run-graph status after timeout: {error}"
                ));
            }
        }
    }
    None
}

pub(crate) fn resolve_init_bootstrap_source_root() -> PathBuf {
    if let Some(installed_root) = resolve_installed_runtime_root() {
        for candidate in installed_runtime_source_root_candidates(&installed_root) {
            if looks_like_init_bootstrap_source_root(&candidate) {
                return candidate;
            }
        }
    }
    super::repo_runtime_root()
}

pub(crate) fn resolve_installed_runtime_root() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;
    let root = bin_dir.parent()?;
    taskflow_binary_candidates_for_root(root)
        .into_iter()
        .next()
        .map(|_| root.to_path_buf())
}

pub(crate) fn installed_runtime_source_root_candidates(root: &Path) -> Vec<PathBuf> {
    let current_root = root.join("current");
    if current_root == root {
        vec![root.to_path_buf()]
    } else {
        vec![current_root, root.to_path_buf()]
    }
}

pub(crate) fn taskflow_binary_candidates_for_root(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let bin_dir = root.join("bin");
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_binary = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_file() && is_taskflow_binary {
                candidates.push(path);
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_taskflow_runtime_dir = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("taskflow"))
                .unwrap_or(false);
            if path.is_dir() && is_taskflow_runtime_dir {
                let candidate = path.join("src/vida");
                if candidate.exists() {
                    candidates.push(candidate);
                }
            }
        }
    }

    candidates
}

pub(crate) fn looks_like_init_bootstrap_source_root(root: &Path) -> bool {
    resolve_init_agents_source(root).is_ok()
        && resolve_init_sidecar_source(root).is_ok()
        && resolve_init_config_template_source(root).is_ok()
        && looks_like_host_runtime_source_root(root)
}

pub(crate) fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

pub(crate) fn resolve_init_agents_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/AGENTS.scaffold.md"),
        root.join("AGENTS.md"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve generated AGENTS scaffold. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_init_sidecar_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [root.join("AGENTS.sidecar.md")];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve project sidecar source. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub(crate) fn resolve_init_config_template_source(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        root.join("install/assets/vida.config.yaml.template"),
        root.join("docs/framework/templates/vida.config.yaml.template"),
    ];
    first_existing_path(&candidates).ok_or_else(|| {
        format!(
            "Unable to resolve vida.config.yaml template. Checked: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run;
    use crate::runtime_dispatch_state::{
        write_runtime_dispatch_packet, RuntimeDispatchPacketContext,
    };
    use crate::state_store::{RunGraphDispatchReceipt, RunGraphStatus, StateStore};
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir};
    use clap::CommandFactory;
    use serde_json::json;
    use std::fs;
    use std::process::ExitCode;
    use std::time::{Duration, Instant};

    fn wait_for_state_unlock(state_dir: &Path) {
        let direct_lock_path = state_dir.join("LOCK");
        let nested_lock_path = state_dir
            .join(".vida")
            .join("data")
            .join("state")
            .join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while (direct_lock_path.exists() || nested_lock_path.exists()) && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn agent_lane_test_execution_plan(executor_backend: &str) -> serde_json::Value {
        json!({
            "backend_admissibility_matrix": [
                {
                    "backend_id": "junior",
                    "backend_class": "internal",
                    "lane_admissibility": {
                        "implementation": true
                    }
                }
            ],
            "development_flow": {
                "implementer": {
                    "executor_backend": executor_backend
                }
            }
        })
    }

    #[test]
    fn boot_command_succeeds() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        assert_eq!(
            runtime.block_on(run(super::super::Cli {
                command: Some(super::super::Command::Boot(BootArgs {
                    state_dir: Some(harness.path().to_path_buf()),
                    render: RenderMode::Plain,
                    instruction_source_root: None,
                    framework_memory_source_root: None,
                    extra_args: Vec::new(),
                })),
            })),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn boot_with_extra_argument_fails_closed() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(run(cli(&["boot", "unexpected"]))),
            ExitCode::from(2)
        );
    }

    #[test]
    fn clap_help_lists_project_activator() {
        let mut command = crate::Cli::command();
        let help = command.render_long_help().to_string();
        assert!(
            help.contains("project-activator"),
            "project-activator should be present in help"
        );
    }

    #[test]
    fn init_bootstrap_source_requires_bootstrap_markers() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let root = harness.path();
        fs::create_dir_all(root.join("bin")).expect("bin dir should exist");
        fs::write(root.join("bin/taskflow"), "#!/bin/sh\n").expect("taskflow marker should exist");
        assert!(
            !looks_like_init_bootstrap_source_root(root),
            "taskflow binary alone should not qualify as an init bootstrap source"
        );

        fs::create_dir_all(root.join("install/assets")).expect("install assets dir should exist");
        fs::create_dir_all(root.join(".codex")).expect(".codex dir should exist");
        fs::write(
            root.join("install/assets/AGENTS.scaffold.md"),
            "# scaffold\n",
        )
        .expect("generated AGENTS scaffold should exist");
        fs::write(root.join("AGENTS.sidecar.md"), "# sidecar\n")
            .expect("project sidecar should exist");
        fs::write(
            root.join("install/assets/vida.config.yaml.template"),
            concat!(
                "project:\n",
                "  id: demo\n",
                "host_environment:\n",
                "  systems:\n",
                "    codex:\n",
                "      template_root: .codex\n",
                "      runtime_root: .codex\n",
            ),
        )
        .expect("config template should exist");
        assert!(
            looks_like_init_bootstrap_source_root(root),
            "bootstrap source should require actual init assets rather than runtime-only markers"
        );
    }

    #[test]
    fn installed_runtime_source_root_candidates_prefer_current_layout() {
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let install_root = harness.path().join("vida-install");
        let current_root = install_root.join("current");
        fs::create_dir_all(current_root.join("bin")).expect("current bin dir should exist");
        fs::write(current_root.join("bin/taskflow"), "#!/bin/sh\n")
            .expect("current taskflow marker should exist");
        fs::create_dir_all(current_root.join("install/assets"))
            .expect("current install assets dir should exist");
        fs::create_dir_all(current_root.join(".codex")).expect("current .codex dir should exist");
        fs::write(
            current_root.join("install/assets/AGENTS.scaffold.md"),
            "# scaffold\n",
        )
        .expect("current generated AGENTS scaffold should exist");
        fs::write(current_root.join("AGENTS.sidecar.md"), "# sidecar\n")
            .expect("current sidecar should exist");
        fs::write(
            current_root.join("install/assets/vida.config.yaml.template"),
            concat!(
                "project:\n",
                "  id: demo\n",
                "host_environment:\n",
                "  systems:\n",
                "    codex:\n",
                "      template_root: .codex\n",
                "      runtime_root: .codex\n",
            ),
        )
        .expect("current config template should exist");

        let candidates = installed_runtime_source_root_candidates(&install_root);
        assert_eq!(candidates[0], current_root);
        assert!(
            looks_like_init_bootstrap_source_root(&candidates[0]),
            "installed `current/` layout should be recognized as the bootstrap source root"
        );
    }

    #[test]
    fn init_preserves_existing_agents_as_sidecar_when_missing() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());
        fs::write(
            harness.path().join("AGENTS.md"),
            "project documentation: docs/\n",
        )
        .expect("existing agents should be written");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            fs::read_to_string(harness.path().join("AGENTS.sidecar.md"))
                .expect("sidecar should exist"),
            "project documentation: docs/\n"
        );
        let framework_agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("framework agents should exist");
        assert!(
            framework_agents.contains("VIDA Project Bootstrap Carrier"),
            "generated bootstrap carrier should replace root AGENTS.md"
        );
    }

    #[test]
    fn init_replaces_agents_template_and_keeps_existing_sidecar_with_backup() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        fs::write(
            harness.path().join("AGENTS.md"),
            "project-specific bootstrap notes\n",
        )
        .expect("existing agents should be written");
        fs::write(
            harness.path().join("AGENTS.sidecar.md"),
            "current sidecar content\n",
        )
        .expect("existing sidecar should be written");

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);

        let framework_agents = fs::read_to_string(harness.path().join("AGENTS.md"))
            .expect("framework agents should exist");
        assert!(
            framework_agents.contains("VIDA Project Bootstrap Carrier"),
            "generated bootstrap carrier should replace root AGENTS.md"
        );

        let sidecar = fs::read_to_string(harness.path().join("AGENTS.sidecar.md"))
            .expect("sidecar should still exist");
        assert_eq!(sidecar, "current sidecar content\n");

        let backup = fs::read_to_string(
            harness
                .path()
                .join(".vida/receipts/AGENTS.pre-init.backup.md"),
        )
        .expect("agents backup should be written");
        assert_eq!(backup, "project-specific bootstrap notes\n");
    }

    #[test]
    fn init_with_extra_argument_fails_closed() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        assert_eq!(
            runtime.block_on(run(cli(&["init", "unexpected"]))),
            ExitCode::from(2)
        );
    }

    #[test]
    fn agent_init_timeout_reconciliation_defers_when_reopen_is_contended() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-agent-init-timeout-reopen-{}-{}",
            std::process::id(),
            nanos
        ));
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        runtime.block_on(async {
            let store = StateStore::open(root.clone()).await.expect("open store");
            let receipt = RunGraphDispatchReceipt {
                run_id: "run-agent-init-timeout-reopen".to_string(),
                dispatch_target: "implementer".to_string(),
                dispatch_status: "blocked".to_string(),
                lane_status: "lane_blocked".to_string(),
                supersedes_receipt_id: None,
                exception_path_receipt_id: None,
                dispatch_kind: "agent_lane".to_string(),
                dispatch_surface: Some("vida agent-init".to_string()),
                dispatch_command: Some("vida agent-init --execute-dispatch".to_string()),
                dispatch_packet_path: Some("/tmp/packet.json".to_string()),
                dispatch_result_path: Some("/tmp/result.json".to_string()),
                blocker_code: Some("internal_dispatch_timeout_without_receipt".to_string()),
                downstream_dispatch_target: None,
                downstream_dispatch_command: None,
                downstream_dispatch_note: None,
                downstream_dispatch_ready: false,
                downstream_dispatch_blockers: vec!["pending_implementation_evidence".to_string()],
                downstream_dispatch_packet_path: None,
                downstream_dispatch_status: None,
                downstream_dispatch_result_path: None,
                downstream_dispatch_trace_path: None,
                downstream_dispatch_executed_count: 0,
                downstream_dispatch_active_target: Some("implementer".to_string()),
                downstream_dispatch_last_target: None,
                activation_agent_type: Some("junior".to_string()),
                activation_runtime_role: Some("worker".to_string()),
                selected_backend: Some("internal_subagents".to_string()),
                recorded_at: "2026-04-22T00:00:00Z".to_string(),
            };
            let warning = best_effort_record_agent_init_dispatch_timeout_receipt(
                &root,
                &json!({ "run_id": "run-agent-init-timeout-reopen" }),
                &receipt,
                12,
            )
            .await
            .expect("reopen contention should return a deferral warning");
            assert!(
                warning.contains(
                    "authoritative timeout reconciliation deferred until next safe reopen"
                ),
                "expected deferred reconciliation warning, got {warning}"
            );
            drop(store);
            let _ = fs::remove_dir_all(&root);
        });
    }

    #[test]
    fn agent_init_execute_dispatch_timeout_materializes_internal_timeout_receipt() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&[
                "project-activator",
                "--project-id",
                "test-project",
                "--language",
                "english",
                "--host-cli-system",
                "codex",
                "--json"
            ]))),
            ExitCode::SUCCESS
        );
        wait_for_state_unlock(harness.path());
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let config_path = harness.path().join("vida.config.yaml");
        let config = fs::read_to_string(&config_path).expect("config should exist");
        let updated = config.replace(
            "      execution_class: internal\n",
            "      execution_class: internal\n      max_runtime_seconds: 1\n",
        );
        fs::write(&config_path, updated).expect("config should update");

        let fake_bin = harness.path().join("fake-bin");
        fs::create_dir_all(&fake_bin).expect("fake bin dir should exist");
        let fake_codex = fake_bin.join("codex");
        fs::write(
            &fake_codex,
            "#!/bin/sh\nsleep 11\nprintf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"too-late\"}}'\n",
        )
        .expect("fake codex should write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_codex)
                .expect("fake codex metadata should load")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_codex, perms).expect("fake codex should be executable");
        }
        let original_path = std::env::var("PATH").ok();
        let patched_path = if original_path.as_deref().unwrap_or_default().is_empty() {
            fake_bin.display().to_string()
        } else {
            format!(
                "{}:{}",
                fake_bin.display(),
                original_path.as_deref().unwrap_or_default()
            )
        };
        std::env::set_var("PATH", &patched_path);

        let state_root = harness.path().join(".vida").join("data").join("state");
        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should open");
        let role_selection = crate::RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "Implement the bounded fix in crates/vida/src/init_surfaces.rs with regression tests."
                .to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec![
                "implementation".to_string(),
                "crates/vida/src/init_surfaces.rs".to_string(),
            ],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: agent_lane_test_execution_plan("junior"),
            reason: "test".to_string(),
        };
        let run_graph_bootstrap = json!({
            "run_id": "run-agent-init-timeout"
        });
        let status = RunGraphStatus {
            run_id: "run-agent-init-timeout".to_string(),
            task_id: "run-agent-init-timeout".to_string(),
            task_class: "implementation".to_string(),
            active_node: "planning".to_string(),
            next_node: Some("worker".to_string()),
            status: "ready".to_string(),
            route_task_class: "implementation".to_string(),
            selected_backend: "junior".to_string(),
            lane_id: "worker_lane".to_string(),
            lifecycle_stage: "dispatch_ready".to_string(),
            policy_gate: "single_task_scope_required".to_string(),
            handoff_state: "awaiting_worker".to_string(),
            context_state: "sealed".to_string(),
            checkpoint_kind: "conversation_cursor".to_string(),
            resume_target: "dispatch.worker_lane".to_string(),
            recovery_ready: true,
        };
        runtime
            .block_on(store.record_run_graph_status(&status))
            .expect("run graph status should record");
        let receipt = RunGraphDispatchReceipt {
            run_id: "run-agent-init-timeout".to_string(),
            dispatch_target: "implementer".to_string(),
            dispatch_status: "routed".to_string(),
            lane_status: "lane_running".to_string(),
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
            recorded_at: "2026-04-17T00:00:00Z".to_string(),
        };
        let handoff_plan = json!({});
        let ctx = RuntimeDispatchPacketContext::new(
            &state_root,
            &role_selection,
            &receipt,
            &handoff_plan,
            &run_graph_bootstrap,
        );
        let dispatch_packet_path =
            write_runtime_dispatch_packet(&ctx).expect("dispatch packet should render");
        let mut persisted_receipt = receipt.clone();
        persisted_receipt.dispatch_packet_path = Some(dispatch_packet_path.clone());
        runtime
            .block_on(store.record_run_graph_dispatch_receipt(&persisted_receipt))
            .expect("dispatch receipt should record");
        drop(store);

        assert_eq!(
            runtime.block_on(run(cli(&[
                "agent-init",
                "--dispatch-packet",
                dispatch_packet_path.as_str(),
                "--execute-dispatch",
                "--json",
            ]))),
            ExitCode::from(1)
        );
        wait_for_state_unlock(harness.path());

        let store = runtime
            .block_on(StateStore::open(state_root.clone()))
            .expect("state store should reopen");
        let recorded_receipt = runtime
            .block_on(store.latest_run_graph_dispatch_receipt())
            .expect("latest dispatch receipt should load")
            .expect("latest dispatch receipt should exist");
        assert_eq!(recorded_receipt.dispatch_status, "blocked");
        assert_eq!(
            recorded_receipt.blocker_code.as_deref(),
            Some("internal_dispatch_timeout_without_receipt")
        );
        let dispatch_result_path = recorded_receipt
            .dispatch_result_path
            .as_deref()
            .expect("dispatch result path should record");
        let rendered =
            fs::read_to_string(dispatch_result_path).expect("dispatch result artifact should load");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("execute-dispatch json should parse");
        assert_eq!(parsed["status"], "blocked");
        assert_eq!(parsed["execution_state"], "blocked");
        assert_eq!(
            parsed["blocker_code"],
            "internal_dispatch_timeout_without_receipt"
        );
        assert!(parsed["provider_error"]
            .as_str()
            .expect("provider error should render")
            .contains("timed out after 1s"));

        if let Some(original_path) = original_path {
            std::env::set_var("PATH", original_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

fn ensure_runtime_home(project_root: &Path) -> Result<(), String> {
    for relative in [
        ".vida/config",
        ".vida/db",
        ".vida/cache",
        ".vida/framework",
        ".vida/project",
        ".vida/project/agent-extensions",
        ".vida/receipts",
        ".vida/runtime",
        ".vida/scratchpad",
    ] {
        super::ensure_dir(&project_root.join(relative))?;
    }
    Ok(())
}

fn copy_file_if_missing(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    std::fs::copy(source, target).map_err(|error| {
        format!(
            "Failed to copy {} -> {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn write_file_if_missing(target: &Path, contents: &str) -> Result<(), String> {
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        super::ensure_dir(parent)?;
    }
    std::fs::write(target, contents)
        .map_err(|error| format!("Failed to write {}: {error}", target.display()))
}

fn write_runtime_agent_extension_projections(project_root: &Path) -> Result<(), String> {
    let root = super::project_activator_surface::runtime_agent_extensions_root(project_root);
    super::ensure_dir(&root)?;
    write_file_if_missing(
        &root.join("README.md"),
        super::DEFAULT_RUNTIME_AGENT_EXTENSIONS_README,
    )?;
    write_file_if_missing(
        &root.join("roles.yaml"),
        super::DEFAULT_AGENT_EXTENSION_ROLES_YAML,
    )?;
    write_file_if_missing(
        &root.join("skills.yaml"),
        super::DEFAULT_AGENT_EXTENSION_SKILLS_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.yaml"),
        super::DEFAULT_AGENT_EXTENSION_PROFILES_YAML,
    )?;
    write_file_if_missing(
        &root.join("flows.yaml"),
        super::DEFAULT_AGENT_EXTENSION_FLOWS_YAML,
    )?;
    write_file_if_missing(
        &root.join("dispatch-aliases.yaml"),
        super::DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_YAML,
    )?;
    write_file_if_missing(
        &root.join("roles.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_ROLES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("skills.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_SKILLS_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("profiles.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_PROFILES_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("flows.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_FLOWS_SIDECAR_YAML,
    )?;
    write_file_if_missing(
        &root.join("dispatch-aliases.sidecar.yaml"),
        super::DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_SIDECAR_YAML,
    )?;

    let receipt_path = project_root.join(".vida/receipts/agent-extensions-bootstrap.json");
    if !receipt_path.exists() {
        let generated_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("rfc3339 timestamp should render");
        let receipt = serde_json::json!({
            "receipt_kind": "agent_extensions_bootstrap",
            "generated_at": generated_at,
            "project_root": project_root.display().to_string(),
            "runtime_projection_root": root.display().to_string(),
            "base_projection_files": [
                ".vida/project/agent-extensions/README.md",
                ".vida/project/agent-extensions/roles.yaml",
                ".vida/project/agent-extensions/skills.yaml",
                ".vida/project/agent-extensions/profiles.yaml",
                ".vida/project/agent-extensions/flows.yaml",
                ".vida/project/agent-extensions/dispatch-aliases.yaml"
            ],
            "sidecar_projection_files": [
                ".vida/project/agent-extensions/roles.sidecar.yaml",
                ".vida/project/agent-extensions/skills.sidecar.yaml",
                ".vida/project/agent-extensions/profiles.sidecar.yaml",
                ".vida/project/agent-extensions/flows.sidecar.yaml",
                ".vida/project/agent-extensions/dispatch-aliases.sidecar.yaml"
            ],
            "source": "vida init default runtime projection bootstrap"
        });
        write_file_if_missing(
            &receipt_path,
            &serde_json::to_string_pretty(&receipt)
                .expect("agent extension bootstrap receipt should render"),
        )?;
    }

    Ok(())
}

pub(crate) async fn run_init(args: super::BootArgs) -> ExitCode {
    if let Some(arg) = args.extra_args.first() {
        eprintln!("Unsupported `vida init` argument `{arg}` in Binary Foundation.");
        return ExitCode::from(2);
    }

    let project_root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to resolve current directory: {error}");
            return ExitCode::from(1);
        }
    };
    let source_root = resolve_init_bootstrap_source_root();
    let framework_agents = match resolve_init_agents_source(&source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let sidecar_scaffold = match resolve_init_sidecar_source(&source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let config_template = match resolve_init_config_template_source(&source_root) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    if !framework_agents.is_file() {
        eprintln!(
            "Missing framework bootstrap carrier: {}",
            framework_agents.display()
        );
        return ExitCode::from(1);
    }

    if let Err(error) = materialize_framework_agents_and_sidecar(
        &project_root,
        &framework_agents,
        &sidecar_scaffold,
    )
    .and_then(|()| copy_file_if_missing(&config_template, &project_root.join("vida.config.yaml")))
    .and_then(|()| materialize_project_docs_scaffold(&project_root))
    .and_then(|()| ensure_runtime_home(&project_root))
    .and_then(|()| write_runtime_agent_extension_projections(&project_root))
    {
        eprintln!("{error}");
        return ExitCode::from(1);
    }

    let activation_view =
        super::project_activator_surface::build_project_activator_view(&project_root);
    print_init_summary(&project_root, &activation_view);
    ExitCode::SUCCESS
}

fn materialize_project_docs_scaffold(project_root: &Path) -> Result<(), String> {
    let project_id = super::project_activator_surface::inferred_project_id_candidate(project_root);
    let project_title = super::inferred_project_title(&project_id, None);
    let source_root = resolve_init_bootstrap_source_root();
    let feature_template_source =
        source_root.join("docs/product/spec/templates/feature-design-document.template.md");
    let feature_template = std::fs::read_to_string(&feature_template_source).map_err(|error| {
        format!(
            "Failed to read framework feature-design template source {}: {error}",
            feature_template_source.display()
        )
    })?;

    let generated_files = vec![
        (
            project_root.join("README.md"),
            render_project_readme(&project_title),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ROOT_MAP),
            render_project_root_map(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PRODUCT_INDEX),
            render_project_product_index(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PRODUCT_SPEC_README),
            render_project_product_spec_readme(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE),
            feature_template,
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ARCHITECTURE_DOC),
            render_project_architecture_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_PROCESS_README),
            render_project_process_readme(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_DECISIONS_DOC),
            with_scaffold_footer(
                "# Decisions\n\nRecord bounded architecture and product decisions here.\n",
                "process/decisions",
                "process_doc",
                "docs/process/decisions.md",
            ),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_ENVIRONMENTS_DOC),
            render_project_environments_doc(project_root),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_OPERATIONS_DOC),
            render_project_operations_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_AGENT_SYSTEM_DOC),
            render_project_agent_system_doc(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_DOC_TOOLING_DOC),
            render_project_doc_tooling_map(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC),
            render_project_host_agent_guide(),
        ),
        (
            project_root.join(super::DEFAULT_PROJECT_RESEARCH_README),
            render_project_research_readme(),
        ),
    ];

    for (path, content) in generated_files {
        write_file_if_missing(&path, &content)?;
        if let Ok(relative_source_path) = path.strip_prefix(project_root) {
            write_scaffold_changelog_if_missing(
                &path,
                relative_source_path,
                scaffold_artifact_path_for(relative_source_path),
                scaffold_artifact_type_for(relative_source_path),
            )?;
        }
    }

    Ok(())
}

pub(crate) fn render_project_readme(project_title: &str) -> String {
    with_scaffold_footer(
        &format!(
            "# {project_title}\n\n\
This repository contains a VIDA-initialized project scaffold.\n\n\
Use `AGENTS.md` for framework bootstrap, `AGENTS.sidecar.md` for project docs routing, and `docs/` for project-owned operating context.\n"
        ),
        "project/readme",
        "document",
        "README.md",
    )
}

pub(crate) fn render_project_root_map() -> String {
    with_scaffold_footer(
        &format!(
            "# Project Root Map\n\n\
This project uses the following canonical documentation roots:\n\n\
- `docs/product/` for product-facing intent and architecture notes\n\
- `docs/process/` for project operations and working agreements\n\
- `docs/research/` for research notes and discovery artifacts\n\n\
Primary pointers:\n\n\
- Product index: `{}`\n\
- Product spec/readiness guide: `{}`\n\
- Feature design template: `{}`\n\
- Process index: `{}`\n\
- Documentation tooling: `{}`\n\
- Host agent guide: `{}`\n\
- Research index: `{}`\n\
- Repository overview: `README.md`\n",
            super::DEFAULT_PROJECT_PRODUCT_INDEX,
            super::DEFAULT_PROJECT_PRODUCT_SPEC_README,
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE,
            super::DEFAULT_PROJECT_PROCESS_README,
            super::DEFAULT_PROJECT_DOC_TOOLING_DOC,
            super::DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC,
            super::DEFAULT_PROJECT_RESEARCH_README
        ),
        "project/root-map",
        "document",
        "docs/project-root-map.md",
    )
}

pub(crate) fn render_project_product_index() -> String {
    with_scaffold_footer(
        &format!(
            "# Product Index\n\n\
Product documentation currently contains:\n\n\
- `{}` for the initial project architecture outline\n\
- `{}` for bounded feature/change design and ADR routing\n",
            super::DEFAULT_PROJECT_ARCHITECTURE_DOC,
            super::DEFAULT_PROJECT_PRODUCT_SPEC_README
        ),
        "product/index",
        "product_index",
        "docs/product/index.md",
    )
}

pub(crate) fn render_project_product_spec_readme() -> String {
    with_scaffold_footer(
        &format!(
            "# Product Spec Guide\n\n\
Use this directory for bounded product-facing feature/change design documents and linked ADRs.\n\n\
Default rule:\n\n\
1. If a request asks for research, detailed specifications, implementation planning, and then code, create or update one bounded design document before implementation.\n\
2. Start from the local template at `{}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before normal implementation work begins.\n\
4. Use `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check` to keep the document canonical.\n\
5. Close the spec-pack task only after the design artifact is finalized and validated, then hand off through the next TaskFlow packet.\n\
6. When one major decision needs durable standalone recording, add a linked ADR instead of overloading the design document.\n\
\n\
Suggested homes:\n\n\
- `docs/product/spec/<feature>-design.md` for committed feature/change designs\n\
- `docs/research/<topic>.md` for exploratory research before design closure\n",
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "product/spec/readme",
        "product_spec",
        "docs/product/spec/README.md",
    )
}

pub(crate) fn render_project_architecture_doc() -> String {
    with_scaffold_footer(
        "# Architecture\n\nCurrent project posture:\n\n- VIDA bootstrap scaffold is initialized\n- project documentation roots are materialized\n- project-specific implementation modules are not yet defined\n",
        "product/architecture",
        "document",
        "docs/product/architecture.md",
    )
}

pub(crate) fn render_project_process_readme() -> String {
    with_scaffold_footer(
        "# Process Docs\n\nThis directory contains the minimum process documentation expected by VIDA activation.\n\nAvailable process docs:\n\n- `decisions.md`\n- `environments.md`\n- `project-operations.md`\n- `agent-system.md`\n- `documentation-tooling-map.md`\n- `codex-agent-configuration-guide.md` (current host-agent guide filename)\n",
        "process/readme",
        "process_doc",
        "docs/process/README.md",
    )
}

pub(crate) fn render_project_decisions_doc(answers: &super::ProjectActivationAnswers) -> String {
    format!(
        "# Decisions\n\n\
Initial activation decisions:\n\n\
- project id: `{}`\n\
- host CLI system: selected through `vida project-activator`\n\
- language policy:\n  - user communication: `{}`\n  - reasoning: `{}`\n  - documentation: `{}`\n  - todo protocol: `{}`\n",
        answers.project_id,
        answers.user_communication_language,
        answers.reasoning_language,
        answers.documentation_language,
        answers.todo_protocol_language
    )
}

pub(crate) fn render_project_environments_doc(project_root: &Path) -> String {
    with_scaffold_footer(
        &format!(
            "# Environments\n\n\
Initial environment assumptions:\n\n\
- local project root: `{}`\n\
- VIDA runtime directories are managed under `.vida/`\n\
- host CLI agent template is selected through `vida project-activator`\n",
            project_root.display()
        ),
        "process/environments",
        "process_doc",
        "docs/process/environments.md",
    )
}

pub(crate) fn render_project_operations_doc() -> String {
    with_scaffold_footer(
        &format!(
            "# Project Operations\n\n\
Current operating baseline:\n\n\
- bootstrap through `AGENTS.md` followed by the bounded VIDA init surfaces\n\
- use `AGENTS.sidecar.md` as the project documentation map\n\
- while project activation is pending, do not enter TaskFlow execution; use `vida project-activator` and `vida docflow`\n\
\n\
Default feature-delivery flow:\n\n\
1. If the request asks for research, specifications, a plan, and then implementation, start with a bounded design document.\n\
2. Use the local template at `{}`.\n\
3. Open one feature epic and one spec-pack task in `vida taskflow` before code execution.\n\
4. Keep the design artifact canonical through `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check`.\n\
5. Close the spec-pack task and shape the next work-pool/dev packet in `vida taskflow` after the design document names the bounded file set, proof targets, and rollout.\n\
6. When the selected host runtime surface is materialized, use the delegated host team surface instead of collapsing the root session directly into coding.\n\
7. Treat `vida.config.yaml` as the owner of carrier tiers, host-system inventory, and any optional internal aliases; project-visible activation should still use the selected carrier tier plus explicit runtime role.\n\
8. Let runtime map the current packet role into the cheapest capable carrier tier with a healthy local score from `.vida/state/worker-strategy.json`.\n\
9. For normal write-producing work, treat project agent-first execution as the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional executor details and not the canonical project control surface.\n\
10. Keep the root session in orchestration posture unless an explicit exception path is recorded.\n\
11. Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; if the root-session write guard is still active, continue through packet shaping or `vida agent-init` dispatch instead of local coding.\n\
12. Host-local shell/edit capability is not a lane-change receipt and does not authorize root-session coding.\n\
13. If the user explicitly orders agent-first or parallel-agent execution, keep that routing intent sticky; do not silently substitute root-session coding.\n\
14. Finding the patch location, reproducing a runtime defect, hitting a worker timeout, or tripping a thread-limit/`not_found` lane failure does not authorize root-session coding; recover delegated lanes, wait, reroute, or record the exception path first.\n\
15. If delegated execution returns only an activation view without execution evidence and a bounded read-only diagnostic path still exists, continue diagnosis to a code-level blocker or next bounded fix before asking the user to choose a route.\n\
16. Saturation recovery means: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered.\n\
17. Under continued-development intent, stay in commentary/progress mode until the user explicitly asks to stop; do not emit final closure wording while a next lawful TaskFlow continuation item is already known.\n\
18. Do not treat commentary, status output, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\n\
19. If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.\n\
20. After any bounded result, green test, successful build, runtime handoff, or delegated handoff, immediately bind the next lawful continuation item in the same cycle instead of pausing at a summary.\n\
21. Sticky continuation intent is not permission to self-select `ready_head[0]`, the first ready backlog item, or any adjacent slice; fail closed unless the active bounded unit is explicit from user wording or runtime evidence.\n\
22. If continued-development intent is active but `vida status --json` or `vida orchestrator-init --json` cannot state `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, publish an ambiguity report instead of continuing implementation.\n\
23. When recording progress into the backlog from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.\n",
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "process/project-operations",
        "process_doc",
        "docs/process/project-operations.md",
    )
}

pub(crate) fn render_project_agent_system_doc() -> String {
    with_scaffold_footer(
        "# Agent System\n\nProject activation owns host CLI agent-template selection and runtime admission.\n\n- default framework host templates become available only after the selected host CLI template is materialized\n- supported and active host CLI systems are config-driven under `vida.config.yaml -> host_environment.systems`\n- framework template inventory may be broader than the enabled active list in project config\n- carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers`; for the current internal Codex adapter, `vida.config.yaml -> host_environment.codex.agents` remains the rendered tier-catalog source\n- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model\n- the selected runtime surface is rendered under the configured runtime root and is not the owner of tier/rate/task-class policy\n- project activation materializes the selected host template using the configured `materialization_mode`; the current internal Codex adapter renders the configured TOML catalog root, while external CLI systems use their configured runtime roots\n- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`\n- project-local agent extensions remain under `.vida/project/agent-extensions/`\n- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists\n- project \"agent-first\" development means the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional carrier mechanics and not the canonical execution contract\n- host-local shell/edit capability is an executor affordance only and must not be interpreted as lawful root-session write ownership\n- when the selected host execution class is internal, optional external CLI subagents remain auxiliary carrier details and do not make the whole session externally gated by default\n- patch localization, runtime-defect diagnosis, or other read-only findings feed the next delegated packet and do not transfer write ownership back to the root session\n",
        "process/agent-system",
        "process_doc",
        "docs/process/agent-system.md",
    )
}

pub(crate) fn render_project_doc_tooling_map() -> String {
    with_scaffold_footer(
        &format!(
            "# Documentation Tooling Map\n\n\
Use `vida docflow` for documentation inventory, mutation, validation, and readiness checks.\n\n\
Design-document rule:\n\n\
1. For bounded feature/change work that requires research, detailed specifications, planning, and implementation, begin with one design document before code execution.\n\
2. Start from `{}`.\n\
3. Open one epic and one spec-pack task in `vida taskflow` before writing code.\n\
4. Suggested command sequence:\n\
   - `vida docflow init docs/product/spec/<feature>-design.md product/spec/<feature>-design product_spec \"initialize feature design\"`\n\
   - edit the document using the local template shape\n\
   - `vida docflow finalize-edit docs/product/spec/<feature>-design.md \"record bounded feature design\"`\n\
   - `vida docflow check --root . docs/product/spec/<feature>-design.md`\n\
   - `vida task close <spec-task-id> --reason \"design packet finalized and handed off\" --json`\n\
\n\
Activation rule:\n\n\
1. During project activation, `vida project-activator` owns bounded config/doc materialization.\n\
2. `vida taskflow` and any non-canonical external TaskFlow runtime are not lawful activation-entry surfaces while activation is pending.\n\
3. After activation writes, prefer `vida docflow` for documentation-oriented inspection and proof before multi-step implementation.\n",
            super::DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE
        ),
        "process/documentation-tooling-map",
        "process_doc",
        "docs/process/documentation-tooling-map.md",
    )
}

pub(crate) fn render_project_research_readme() -> String {
    with_scaffold_footer(
        "# Research Notes\n\nUse this directory for research artifacts, discovery notes, and external references that support future project work.\n",
        "research/readme",
        "document",
        "docs/research/README.md",
    )
}

pub(crate) fn render_project_host_agent_guide() -> String {
    with_scaffold_footer(
        "# Host Agent Configuration Guide\n\nThis project uses framework-materialized host runtime surfaces; the active internal Codex surface currently renders under `.codex/**`.\n\nSource-of-truth rule:\n\n- `vida.config.yaml -> host_environment.codex.agents` owns carrier-tier metadata, rates, runtime-role fit, and task-class fit\n- `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` owns the dispatch-alias registry for executor-local overlays\n- `.codex/**` is the rendered executor surface used by the current internal Codex adapter after activation\n- `.codex/config.toml` should expose the carrier tiers materialized from overlay\n\nCarrier rule:\n\n- the primary visible agent model is the configured carrier catalog rendered from `vida.config.yaml`, not a Rust-hardcoded role list\n- runtime role remains explicit activation state such as `worker`, `coach`, `verifier`, or `solution_architect`\n- internal alias ids may exist in registry state, but they must not replace the carrier-tier model at the project surface\n\nWorking rule:\n\n1. The root session stays the orchestrator.\n2. Documentation/specification work should complete the bounded design document first.\n3. Before delegated implementation starts, open the feature epic/spec task in `vida taskflow` and close the spec task only after the design artifact is finalized.\n4. After a bounded packet exists, route research, specification, planning, implementation, review, and verification through the configured carrier catalog instead of collapsing into root-session coding.\n5. Let runtime choose the cheapest capable configured carrier tier with a healthy local score from `.vida/state/worker-strategy.json` and pass the lawful runtime role explicitly.\n6. Canonical delegated execution still dispatches through `vida agent-init`; host-tool-specific subagent APIs are optional executor details and not the primary project delegation surface.\n7. Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; an active root-session write guard still means orchestration-only.\n8. If the user explicitly orders agent-first or parallel-agent execution, keep that routing sticky; do not silently substitute root-session coding because a host tool offers local write access.\n9. Finding the patch location, reproducing a runtime defect, hitting a worker timeout, or tripping a thread-limit/`not_found` lane failure is not a lane-change receipt and does not authorize root-session coding.\n10. Recover delegated-lane saturation first: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered.\n11. Under continued-development intent, stay in commentary/progress mode and continue routing; do not emit final closure wording while a next lawful continuation item is already known.\n12. Do not treat commentary, status output, an intermediate status update, or “I have explained the result” as a lawful pause boundary.\n13. If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.\n14. After any bounded result, successful build, runtime handoff, or delegated handoff, immediately bind the next lawful continuation item in the same cycle instead of pausing at a summary.\n15. Sticky continuation intent does not authorize choosing the first ready task or an adjacent slice by plausibility; continue only when the active bounded unit is explicit from user wording or runtime evidence.\n16. If `vida status --json` or `vida orchestrator-init --json` does not expose explicit `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, fail closed to an ambiguity report instead of continuing implementation.\n17. When recording task progress from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.\n18. Use `.vida/project/agent-extensions/**` for project-local role and skill overlays; do not treat `.codex/**` as the owner of framework or product law.\n",
        "process/codex-agent-configuration-guide",
        "process_doc",
        "docs/process/codex-agent-configuration-guide.md",
    )
}

fn with_scaffold_footer(
    body: &str,
    artifact_path: &str,
    artifact_type: &str,
    source_path: &str,
) -> String {
    let changelog_ref = scaffold_changelog_ref_for(source_path);
    format!(
        "{body}\n-----\nartifact_path: {artifact_path}\nartifact_type: {artifact_type}\nartifact_version: '1'\nartifact_revision: '2026-04-04'\nschema_version: '1'\nstatus: scaffold\nsource_path: {source_path}\ncreated_at: '2026-04-04T00:00:00Z'\nupdated_at: '2026-04-04T00:00:00Z'\nchangelog_ref: {changelog_ref}\n"
    )
}

fn scaffold_changelog_ref_for(source_path: &str) -> String {
    let source_path = Path::new(source_path);
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    format!("{stem}.changelog.jsonl")
}

fn scaffold_artifact_path_for(relative_source_path: &Path) -> &'static str {
    match relative_source_path.to_string_lossy().as_ref() {
        "README.md" => "project/readme",
        "docs/project-root-map.md" => "project/root-map",
        "docs/product/index.md" => "product/index",
        "docs/product/architecture.md" => "product/architecture",
        "docs/product/spec/README.md" => "product/spec/readme",
        "docs/product/spec/templates/feature-design-document.template.md" => {
            "product/spec/templates/feature-design-document.template"
        }
        "docs/process/README.md" => "process/readme",
        "docs/process/agent-system.md" => "process/agent-system",
        "docs/process/codex-agent-configuration-guide.md" => {
            "process/codex-agent-configuration-guide"
        }
        "docs/process/decisions.md" => "process/decisions",
        "docs/process/documentation-tooling-map.md" => "process/documentation-tooling-map",
        "docs/process/environments.md" => "process/environments",
        "docs/process/project-operations.md" => "process/project-operations",
        "docs/research/README.md" => "research/readme",
        _ => "project/scaffold-doc",
    }
}

fn scaffold_artifact_type_for(relative_source_path: &Path) -> &'static str {
    match relative_source_path.to_string_lossy().as_ref() {
        "docs/process/README.md"
        | "docs/process/agent-system.md"
        | "docs/process/codex-agent-configuration-guide.md"
        | "docs/process/decisions.md"
        | "docs/process/documentation-tooling-map.md"
        | "docs/process/environments.md"
        | "docs/process/project-operations.md" => "process_doc",
        "docs/product/index.md" => "product_index",
        "docs/product/spec/README.md"
        | "docs/product/spec/templates/feature-design-document.template.md" => "product_spec",
        _ => "document",
    }
}

fn write_scaffold_changelog_if_missing(
    absolute_source_path: &Path,
    relative_source_path: &Path,
    artifact_path: &str,
    artifact_type: &str,
) -> Result<(), String> {
    let parent = absolute_source_path.parent().ok_or_else(|| {
        format!(
            "Failed to determine scaffold parent directory for {}",
            absolute_source_path.display()
        )
    })?;
    let stem = absolute_source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("artifact");
    let changelog_path = parent.join(format!("{stem}.changelog.jsonl"));
    let entry = format!(
        "{{\"ts\":\"2026-04-04T00:00:00Z\",\"event\":\"metadata_initialized\",\"artifact_path\":\"{artifact_path}\",\"artifact_type\":\"{artifact_type}\",\"artifact_version\":\"1\",\"artifact_revision\":\"2026-04-04\",\"source_path\":\"{}\",\"reason\":\"initialize scaffold metadata for docflow-ready project bootstrap\",\"actor\":\"vida\",\"scope\":\"scaffold-init\",\"tags\":[\"scaffold\",\"docflow\"]}}\n",
        relative_source_path.display()
    );
    write_file_if_missing(&changelog_path, &entry)
}

fn materialize_framework_agents_and_sidecar(
    project_root: &Path,
    framework_agents: &Path,
    sidecar_scaffold: &Path,
) -> Result<(), String> {
    let agents = project_root.join("AGENTS.md");
    let sidecar = project_root.join("AGENTS.sidecar.md");
    let framework_contents = std::fs::read_to_string(framework_agents)
        .map_err(|error| format!("Failed to read {}: {error}", framework_agents.display()))?;

    if agents.is_file() {
        let existing_agents = std::fs::read_to_string(&agents)
            .map_err(|error| format!("Failed to read {}: {error}", agents.display()))?;
        if existing_agents != framework_contents {
            if !sidecar.is_file()
                || super::project_activator_surface::file_contains_placeholder(&sidecar)
            {
                if let Some(parent) = sidecar.parent() {
                    super::ensure_dir(parent)?;
                }
                std::fs::write(&sidecar, existing_agents).map_err(|error| {
                    format!(
                        "Failed to preserve existing {} as {}: {error}",
                        agents.display(),
                        sidecar.display()
                    )
                })?;
            } else {
                let backup_path = project_root.join(".vida/receipts/AGENTS.pre-init.backup.md");
                if let Some(parent) = backup_path.parent() {
                    super::ensure_dir(parent)?;
                }
                if !backup_path.exists() {
                    std::fs::write(&backup_path, existing_agents).map_err(|error| {
                        format!("Failed to write {} backup: {error}", backup_path.display())
                    })?;
                }
            }
        }
    }

    copy_file_if_missing(sidecar_scaffold, &sidecar)?;
    std::fs::write(&agents, framework_contents)
        .map_err(|error| format!("Failed to write {}: {error}", agents.display()))
}

fn print_init_summary(project_root: &Path, activation_view: &serde_json::Value) {
    println!("vida init project bootstrap ready");
    println!("project root: {}", project_root.display());
    println!(
        "materialized: AGENTS.md, AGENTS.sidecar.md, vida.config.yaml, README.md, docs/project-root-map.md, docs/product/**, docs/process/**, docs/research/README.md, .vida/config, .vida/db, .vida/cache, .vida/framework, .vida/project, .vida/project/agent-extensions/*, .vida/project/agent-extensions/*.sidecar.yaml, .vida/receipts, .vida/runtime, .vida/scratchpad"
    );
    println!(
        "activation status: {}",
        activation_view["status"].as_str().unwrap_or("unknown")
    );
    if activation_view["activation_pending"]
        .as_bool()
        .unwrap_or(true)
    {
        println!("next step: vida project-activator --json");
        if let Some(example) = activation_view["interview"]["one_shot_example"].as_str() {
            println!("activation example: {example}");
        }
        if let Some(step) = activation_view["next_steps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .next()
        {
            println!("activation note: {step}");
        }
        println!(
            "activation rule: while activation is pending, use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime"
        );
    }
}

pub(crate) async fn run_boot(args: BootArgs) -> ExitCode {
    if let Some(arg) = args.extra_args.first() {
        eprintln!("Unsupported `vida boot` argument `{arg}` in Binary Foundation.");
        return ExitCode::from(2);
    }

    let render = args.render;
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let instruction_source_root = args
        .instruction_source_root
        .unwrap_or_else(|| PathBuf::from(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT));
    let framework_memory_source_root = args
        .framework_memory_source_root
        .unwrap_or_else(|| PathBuf::from(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT));

    match tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
        StateStore::open(state_dir),
    )
    .await
    {
        Ok(Ok(store)) => match store.seed_framework_instruction_bundle().await {
            Ok(()) => match store.backend_summary().await {
                Ok(summary) => match store.source_tree_summary().await {
                    Ok(source_tree) => match store
                        .ingest_instruction_source_tree(&normalize_root_arg(
                            &instruction_source_root,
                        ))
                        .await
                    {
                        Ok(ingest) => {
                            print_surface_header(render, "vida boot scaffold ready");
                            print_surface_line(render, "authoritative state store", &summary);
                            match store.state_spine_summary().await {
                                Ok(state_spine) => print_surface_line(
                                    render,
                                    "authoritative state spine",
                                    &format!(
                                        "initialized (state-v{}, {} entity surfaces, mutation root {})",
                                        state_spine.state_schema_version,
                                        state_spine.entity_surface_count,
                                        state_spine.authoritative_mutation_root
                                    ),
                                ),
                                Err(error) => {
                                    eprintln!(
                                        "Failed to read authoritative state spine summary: {error}"
                                    );
                                    return ExitCode::from(1);
                                }
                            }
                            print_surface_line(render, "framework instruction bundle", "seeded");
                            print_surface_line(render, "instruction source tree", &source_tree);
                            print_surface_line(render, "instruction ingest", &ingest.as_display());
                            match store.evaluate_boot_compatibility().await {
                                Ok(compatibility) => {
                                    print_surface_line(
                                        render,
                                        "boot compatibility",
                                        &format!(
                                            "{} ({})",
                                            compatibility.classification, compatibility.next_step
                                        ),
                                    );
                                    if crate::release1_contracts::canonical_compatibility_class_str(
                                        &compatibility.classification,
                                    ) != Some(
                                        crate::release1_contracts::CompatibilityClass::BackwardCompatible
                                            .as_str(),
                                    ) {
                                        eprintln!(
                                            "Boot compatibility check failed: {}",
                                            compatibility.reasons.join(", ")
                                        );
                                        return ExitCode::from(1);
                                    }
                                }
                                Err(error) => {
                                    eprintln!("Failed to evaluate boot compatibility: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store.evaluate_migration_preflight().await {
                                Ok(migration) => {
                                    print_surface_line(
                                        render,
                                        "migration preflight",
                                        &format!(
                                            "{} / {} ({})",
                                            migration.compatibility_classification,
                                            migration.migration_state,
                                            migration.next_step
                                        ),
                                    );
                                    if !migration.blockers.is_empty() {
                                        eprintln!(
                                            "Migration preflight failed: {}",
                                            migration.blockers.join(", ")
                                        );
                                        return ExitCode::from(1);
                                    }
                                }
                                Err(error) => {
                                    eprintln!("Failed to evaluate migration preflight: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store.migration_receipt_summary().await {
                                Ok(summary) => {
                                    print_surface_line(
                                        render,
                                        "migration receipts",
                                        &summary.as_display(),
                                    );
                                }
                                Err(error) => {
                                    eprintln!("Failed to read migration receipt summary: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store.active_instruction_root().await {
                                Ok(root_artifact_id) => match store
                                    .resolve_effective_instruction_bundle(&root_artifact_id)
                                    .await
                                {
                                    Ok(bundle) => {
                                        print_surface_line(
                                            render,
                                            "effective instruction bundle",
                                            &bundle.mandatory_chain_order.join(" -> "),
                                        );
                                        print_surface_line(
                                            render,
                                            "effective instruction bundle receipt",
                                            &bundle.receipt_id,
                                        );
                                    }
                                    Err(error) => {
                                        eprintln!(
                                            "Failed to resolve effective instruction bundle: {error}"
                                        );
                                        return ExitCode::from(1);
                                    }
                                },
                                Err(error) => {
                                    eprintln!("Failed to read active instruction root: {error}");
                                    return ExitCode::from(1);
                                }
                            }
                            match store
                                .ingest_framework_memory_source_tree(&normalize_root_arg(
                                    &framework_memory_source_root,
                                ))
                                .await
                            {
                                Ok(framework_ingest) => {
                                    if let Err(error) =
                                        sync_launcher_activation_snapshot(&store).await
                                    {
                                        eprintln!(
                                            "Failed to persist launcher activation snapshot: {error}"
                                        );
                                        return ExitCode::from(1);
                                    }
                                    print_surface_line(
                                        render,
                                        "framework memory ingest",
                                        &framework_ingest.as_display(),
                                    );
                                    print_surface_line(
                                        render,
                                        "state dir",
                                        &store.root().display().to_string(),
                                    );
                                    ExitCode::SUCCESS
                                }
                                Err(error) => {
                                    eprintln!(
                                        "Failed to ingest framework memory source tree: {error}"
                                    );
                                    ExitCode::from(1)
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!("Failed to ingest instruction source tree: {error}");
                            ExitCode::from(1)
                        }
                    },
                    Err(error) => {
                        eprintln!("Failed to read source tree metadata: {error}");
                        ExitCode::from(1)
                    }
                },
                Err(error) => {
                    eprintln!("Failed to read storage metadata: {error}");
                    ExitCode::from(1)
                }
            },
            Err(error) => {
                eprintln!("Failed to seed framework instruction bundle: {error}");
                ExitCode::from(1)
            }
        },
        Ok(Err(error)) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!(
                "Timed out opening authoritative state store for `vida boot` after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
            );
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_orchestrator_init(args: InitArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);
    let instruction_source_root = PathBuf::from(state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT);
    let framework_memory_source_root =
        PathBuf::from(state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT);

    match tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
        StateStore::open(state_dir),
    )
    .await
    {
        Ok(Ok(store)) => {
            match tokio::time::timeout(
                std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
                ensure_launcher_bootstrap(
                    &store,
                    &instruction_source_root,
                    &framework_memory_source_root,
                ),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
                Err(_) => {
                    eprintln!(
                        "Timed out ensuring launcher bootstrap for `vida orchestrator-init` after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
                    );
                    return ExitCode::from(1);
                }
            }
            match tokio::time::timeout(
                std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
                build_taskflow_consume_bundle_payload(&store),
            )
            .await
            {
                Ok(Ok(bundle)) => {
                    let project_activation_view = match std::env::current_dir() {
                        Ok(path) => {
                            super::project_activator_surface::build_project_activator_view(&path)
                        }
                        Err(error) => {
                            eprintln!("Failed to resolve current directory: {error}");
                            return ExitCode::from(1);
                        }
                    };
                    let init_view =
                        super::project_activator_surface::merge_project_activation_into_init_view(
                            bundle.orchestrator_init_view,
                            &project_activation_view,
                        );
                    let dev_team_readiness =
                        super::taskflow_consume_bundle::build_dev_team_readiness(
                            &bundle.config_path,
                            &bundle.activation_bundle,
                        );
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surface": "vida orchestrator-init",
                                "init": init_view,
                                "dev_team_readiness": dev_team_readiness,
                                "runtime_bundle_summary": {
                                    "bundle_id": bundle.metadata["bundle_id"],
                                    "root_artifact_id": bundle.control_core["root_artifact_id"],
                                    "activation_source": bundle.activation_source,
                                    "vida_root": bundle.vida_root,
                                    "state_dir": store.root().display().to_string(),
                                    "launcher_runtime_paths": bundle.launcher_runtime_paths,
                                },
                            }))
                            .expect("orchestrator-init json should render")
                        );
                    } else {
                        print_surface_header(RenderMode::Plain, "vida orchestrator-init");
                        print_surface_line(
                            RenderMode::Plain,
                            "status",
                            init_view["status"].as_str().unwrap_or("unknown"),
                        );
                        print_surface_line(RenderMode::Plain, "boot surface", "vida boot");
                        print_surface_line(
                            RenderMode::Plain,
                            "bundle id",
                            bundle.metadata["bundle_id"].as_str().unwrap_or(""),
                        );
                        print_surface_line(
                            RenderMode::Plain,
                            "state dir",
                            &store.root().display().to_string(),
                        );
                        print_compact_command_families(RenderMode::Plain, "vida orchestrator-init");
                        if init_view["project_activation"]["activation_pending"]
                            .as_bool()
                            .unwrap_or(false)
                        {
                            print_surface_line(
                                RenderMode::Plain,
                                "next step",
                                "vida project-activator --json",
                            );
                            if let Some(example) = init_view["project_activation"]["interview"]
                                ["one_shot_example"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "activation example",
                                    example,
                                );
                            }
                            print_surface_line(
                                RenderMode::Plain,
                                "activation runtime",
                                "use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime while activation is pending",
                            );
                        } else if init_view["project_activation"]["normal_work_defaults"]
                            ["documentation_first_for_feature_requests"]
                            .as_bool()
                            .unwrap_or(false)
                        {
                            print_surface_line(
                                RenderMode::Plain,
                                "feature flow",
                                "for requests that combine research/specification/planning and implementation, start with one bounded design document before code execution",
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "feature intake",
                                init_view["project_activation"]["normal_work_defaults"]
                                    ["intake_runtime"]
                                    .as_str()
                                    .unwrap_or("vida taskflow consume final <request> --json"),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "design template",
                                init_view["project_activation"]["normal_work_defaults"]
                                    ["local_feature_design_template"]
                                    .as_str()
                                    .unwrap_or("docs/product/spec/templates/feature-design-document.template.md"),
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "documentation runtime",
                                "open one feature epic and one spec-pack task in `vida taskflow`, then use `vida docflow` to initialize, finalize, and validate the design document before shaping the execution packet",
                            );
                            print_surface_line(
                                RenderMode::Plain,
                                "execution posture",
                                "after the bounded design document is ready, delegate normal write-producing work through the configured development team instead of collapsing directly into root-session coding",
                            );
                            if let Some(rule) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["selection_rule"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "agent model",
                                    "agent=execution carrier; role=runtime activation state",
                                );
                                print_surface_line(RenderMode::Plain, "carrier selection", rule);
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["snapshot"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "agent snapshot cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["carrier_catalog"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "carrier catalog cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["runtime_roles"]
                                .as_str()
                            {
                                print_surface_line(RenderMode::Plain, "runtime roles cmd", command);
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["scores"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "carrier scores cmd",
                                    command,
                                );
                            }
                            if let Some(command) = init_view["project_activation"]
                                ["normal_work_defaults"]["execution_carrier_model"]
                                ["inspect_commands"]["selection_preview"]
                                .as_str()
                            {
                                print_surface_line(
                                    RenderMode::Plain,
                                    "selection preview cmd",
                                    command,
                                );
                            }
                        }
                    }
                    ExitCode::SUCCESS
                }
                Ok(Err(error)) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
                Err(_) => {
                    eprintln!(
                        "Timed out building taskflow consume bundle for `vida orchestrator-init` after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
                    );
                    ExitCode::from(1)
                }
            }
        }
        Ok(Err(error)) => {
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!(
                "Timed out opening authoritative state store for `vida orchestrator-init` after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
            );
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_agent_init(args: AgentInitArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(state_store::default_state_dir);

    match tokio::time::timeout(
        std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
        async {
            match StateStore::open_existing_read_only(state_dir.clone()).await {
                Ok(store) => Ok(store),
                Err(crate::state_store::StateStoreError::MissingStateDir(_)) => {
                    StateStore::open(state_dir.clone()).await
                }
                Err(error) => Err(error),
            }
        },
    )
    .await
    {
        Ok(Ok(store)) => {
            let bundle = match tokio::time::timeout(
                std::time::Duration::from_secs(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS),
                build_taskflow_consume_bundle_payload(&store),
            )
            .await
            {
                Ok(Ok(bundle)) => bundle,
                Ok(Err(error)) => {
                    eprintln!("{error}");
                    return ExitCode::from(1);
                }
                Err(_) => {
                    eprintln!(
                        "Timed out building taskflow consume bundle for `vida agent-init` after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
                    );
                    return ExitCode::from(1);
                }
            };
            let packet_arg_count = usize::from(args.dispatch_packet.is_some())
                + usize::from(args.downstream_packet.is_some());
            if packet_arg_count > 1 {
                eprintln!(
                    "Agent init accepts at most one packet source: use either `--dispatch-packet` or `--downstream-packet`."
                );
                return ExitCode::from(2);
            }
            let selection = if let Some(packet_path) = args.dispatch_packet.as_deref() {
                if args.role.is_some() || args.request_text.is_some() {
                    eprintln!(
                        "Agent init packet activation is exclusive: do not combine packet flags with `--role` or request text."
                    );
                    return ExitCode::from(2);
                }
                let packet = match super::taskflow_consume_resume::read_dispatch_packet(packet_path)
                {
                    Ok(packet) => packet,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                match agent_init_packet_selection(packet_path, packet, false) {
                    Ok(selection) => selection,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                }
            } else if let Some(packet_path) = args.downstream_packet.as_deref() {
                if args.role.is_some() || args.request_text.is_some() {
                    eprintln!(
                        "Agent init packet activation is exclusive: do not combine packet flags with `--role` or request text."
                    );
                    return ExitCode::from(2);
                }
                let packet = match super::taskflow_consume_resume::read_dispatch_packet(packet_path)
                {
                    Ok(packet) => packet,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                };
                match agent_init_packet_selection(packet_path, packet, true) {
                    Ok(selection) => selection,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(2);
                    }
                }
            } else if let Some(role) = args.role.clone() {
                let compiled_bundle = &bundle.activation_bundle;
                if !role_exists_in_lane_bundle(compiled_bundle, &role) || role == "orchestrator" {
                    eprintln!(
                        "Agent init requires a non-orchestrator lane role present in the compiled activation bundle."
                    );
                    return ExitCode::from(2);
                }
                serde_json::json!({
                    "mode": "explicit_role",
                    "selected_role": role,
                    "request_text": args.request_text.clone().unwrap_or_default(),
                })
            } else {
                let request = match args.request_text.as_deref() {
                    Some(request) if !request.trim().is_empty() => request,
                    _ => {
                        eprintln!(
                            "Agent init requires either a non-orchestrator `--role` or a bounded request text."
                        );
                        return ExitCode::from(2);
                    }
                };
                match build_runtime_lane_selection_with_store(&store, request).await {
                    Ok(selection) => {
                        if selection.selected_role == "orchestrator" {
                            eprintln!(
                                "Agent init resolved to orchestrator posture; provide a non-orchestrator `--role` or a bounded worker request."
                            );
                            return ExitCode::from(2);
                        }
                        serde_json::to_value(selection).expect("lane selection should serialize")
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::from(1);
                    }
                }
            };

            let project_activation_view = match std::env::current_dir() {
                Ok(path) => super::project_activator_surface::build_project_activator_view(&path),
                Err(error) => {
                    eprintln!("Failed to resolve current directory: {error}");
                    return ExitCode::from(1);
                }
            };
            let init_view =
                super::project_activator_surface::merge_project_activation_into_init_view(
                    bundle.agent_init_view,
                    &project_activation_view,
                );
            let dev_team_readiness = super::taskflow_consume_bundle::build_dev_team_readiness(
                &bundle.config_path,
                &bundle.activation_bundle,
            );
            let activation_semantics = agent_init_activation_semantics(&selection);
            let surface_payload = build_agent_init_surface_payload(
                init_view.clone(),
                selection.clone(),
                activation_semantics.clone(),
                serde_json::json!({
                    "bundle_id": bundle.metadata["bundle_id"],
                    "activation_source": bundle.activation_source,
                    "vida_root": bundle.vida_root,
                    "state_dir": store.root().display().to_string(),
                    "launcher_runtime_paths": bundle.launcher_runtime_paths,
                }),
                dev_team_readiness,
            );

            if args.execute_dispatch {
                if packet_arg_count == 0 {
                    eprintln!(
                        "Agent init execute-dispatch requires either `--dispatch-packet` or `--downstream-packet`."
                    );
                    return ExitCode::from(2);
                }

                let mut resume_inputs =
                    match super::taskflow_consume_resume::resolve_runtime_consumption_resume_inputs(
                        &store,
                        None,
                        args.dispatch_packet.as_deref(),
                        args.downstream_packet.as_deref(),
                    )
                    .await
                    {
                        Ok(inputs) => inputs,
                        Err(error) => {
                            eprintln!("{error}");
                            return ExitCode::from(1);
                        }
                    };
                let state_root = store.root().to_path_buf();
                drop(store);
                let dispatch_handoff_timeout_seconds =
                    super::dispatch_handoff_timeout_seconds_for_state_root(
                        &state_root,
                        &resume_inputs.role_selection,
                        &resume_inputs.dispatch_receipt,
                    );
                let execute_dispatch_timeout_seconds = dispatch_handoff_timeout_seconds
                    .saturating_add(DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS);
                match tokio::time::timeout(
                    std::time::Duration::from_secs(execute_dispatch_timeout_seconds),
                    super::execute_and_record_dispatch_receipt(
                        &state_root,
                        &resume_inputs.role_selection,
                        &resume_inputs.run_graph_bootstrap,
                        &mut resume_inputs.dispatch_receipt,
                    ),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        eprintln!("Failed to execute agent-init dispatch packet: {error}");
                        return ExitCode::from(1);
                    }
                    Err(_) => {
                        if let Err(error) =
                            super::apply_dispatch_handoff_timeout_to_receipt_for_state_root(
                                &state_root,
                                &resume_inputs.role_selection,
                                &mut resume_inputs.dispatch_receipt,
                                dispatch_handoff_timeout_seconds,
                            )
                        {
                            eprintln!(
                                "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion, and failed to materialize timeout receipt: {error}"
                            );
                            return ExitCode::from(1);
                        }
                        if let Some(warning) =
                            best_effort_record_agent_init_dispatch_timeout_receipt(
                                &state_root,
                                &resume_inputs.run_graph_bootstrap,
                                &resume_inputs.dispatch_receipt,
                                execute_dispatch_timeout_seconds,
                            )
                            .await
                        {
                            eprintln!("{warning}");
                        }
                        eprintln!(
                            "Timed out executing agent-init dispatch packet after {execute_dispatch_timeout_seconds}s total without receipt-backed completion"
                        );
                        return ExitCode::from(1);
                    }
                }
                let Some(dispatch_result_path) = resume_inputs
                    .dispatch_receipt
                    .dispatch_result_path
                    .as_deref()
                else {
                    eprintln!(
                        "Agent init execute-dispatch did not produce a dispatch result artifact."
                    );
                    return ExitCode::from(1);
                };
                let result_body = match std::fs::read_to_string(dispatch_result_path) {
                    Ok(body) => body,
                    Err(error) => {
                        eprintln!(
                            "Failed to read agent-init dispatch result `{dispatch_result_path}`: {error}"
                        );
                        return ExitCode::from(1);
                    }
                };
                let result_json = match serde_json::from_str::<serde_json::Value>(&result_body) {
                    Ok(json) => json,
                    Err(error) => {
                        eprintln!(
                            "Failed to parse agent-init dispatch result `{dispatch_result_path}`: {error}"
                        );
                        return ExitCode::from(1);
                    }
                };
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&result_json)
                            .expect("agent-init dispatch result json should render")
                    );
                } else {
                    crate::print_json_pretty(&result_json);
                }
                return if resume_inputs.dispatch_receipt.dispatch_status == "blocked" {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                };
            }

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&surface_payload)
                        .expect("agent-init json should render")
                );
            } else {
                print_surface_header(RenderMode::Plain, "vida agent-init");
                print_surface_line(
                    RenderMode::Plain,
                    "status",
                    init_view["status"].as_str().unwrap_or("unknown"),
                );
                print_surface_line(
                    RenderMode::Plain,
                    "selected role",
                    selection["selected_role"].as_str().unwrap_or("unknown"),
                );
                if let Some(mode) = selection["mode"].as_str() {
                    print_surface_line(RenderMode::Plain, "mode", mode);
                }
                if let Some(path) = selection["dispatch_packet_path"].as_str() {
                    print_surface_line(RenderMode::Plain, "dispatch packet", path);
                }
                if let Some(path) = selection["downstream_packet_path"].as_str() {
                    print_surface_line(RenderMode::Plain, "downstream packet", path);
                }
                print_compact_command_families(RenderMode::Plain, "vida agent-init");
                if let Some(backend) = surface_payload["backend_truth"]["selected_backend"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "selected backend", backend);
                }
                if let Some(source) =
                    surface_payload["backend_truth"]["selected_backend_source"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "backend source", source);
                }
                if let Some(carrier_id) =
                    surface_payload["backend_truth"]["selected_carrier_id"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "selected carrier", carrier_id);
                }
                if let Some(profile_id) =
                    surface_payload["backend_truth"]["selected_model_profile_id"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "selected model profile", profile_id);
                }
                if let Some(backend) =
                    surface_payload["backend_truth"]["route_primary_backend"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "route primary backend", backend);
                }
                if let Some(backend) =
                    surface_payload["backend_truth"]["route_fallback_backend"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "fallback backend", backend);
                }
                if let Some(posture) =
                    surface_payload["backend_truth"]["effective_execution_posture"].as_str()
                {
                    print_surface_line(RenderMode::Plain, "execution posture", posture);
                }
                if let Some(status) = surface_payload["backend_truth"]["override_status"].as_str() {
                    print_surface_line(RenderMode::Plain, "lawful override", status);
                }
                print_surface_line(
                    RenderMode::Plain,
                    "activation semantics",
                    activation_semantics["activation_kind"]
                        .as_str()
                        .unwrap_or("activation_view"),
                );
                if let Some(next_step) = activation_semantics["next_lawful_action"].as_str() {
                    print_surface_line(RenderMode::Plain, "next lawful action", next_step);
                }
                if let Some(fallback_surface) = init_view["source_mode_fallback_surface"].as_str() {
                    print_surface_line(RenderMode::Plain, "fallback surface", fallback_surface);
                }
                if init_view["project_activation"]["activation_pending"]
                    .as_bool()
                    .unwrap_or(false)
                {
                    print_surface_line(
                        RenderMode::Plain,
                        "next step",
                        "vida project-activator --json",
                    );
                    if let Some(example) =
                        init_view["project_activation"]["interview"]["one_shot_example"].as_str()
                    {
                        print_surface_line(RenderMode::Plain, "activation example", example);
                    }
                    print_surface_line(
                        RenderMode::Plain,
                        "activation runtime",
                        "use `vida project-activator` and `vida docflow`; do not enter `vida taskflow` or any non-canonical external TaskFlow runtime while activation is pending",
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Ok(Err(error)) => {
            if StateStore::message_is_lock_contention(&error.to_string()) {
                return crate::status_surface::emit_degraded_read_lock_surface(
                    "vida agent-init",
                    &state_dir,
                    RenderMode::Plain,
                    args.json,
                    &error.to_string(),
                );
            }
            eprintln!("Failed to open authoritative state store: {error}");
            ExitCode::from(1)
        }
        Err(_) => {
            eprintln!(
                "Timed out opening authoritative state store for `vida agent-init` after {DEFAULT_INIT_SURFACE_TIMEOUT_SECONDS}s"
            );
            ExitCode::from(1)
        }
    }
}

fn agent_init_activation_semantics(selection: &serde_json::Value) -> serde_json::Value {
    let mode = selection["mode"].as_str().unwrap_or("unknown");
    let packet_template_kind = selection["packet_template_kind"]
        .as_str()
        .unwrap_or_default();
    let tracked_flow_shaping_only = packet_template_kind == "tracked_flow_packet";
    let next_lawful_action = match mode {
        "dispatch_packet" | "downstream_packet" if tracked_flow_shaping_only => {
            "complete only the tracked-flow/task-shaping handoff; this activation does not itself execute implementation and does not authorize root-session writing"
        }
        "dispatch_packet" | "downstream_packet" => {
            "use this activation view to execute only the bounded packet owned by the selected lane; completion still requires receipt-backed evidence and does not transfer root-session write authority; if execution evidence is still missing, continue bounded diagnosis/reroute rather than treating this view as completion"
        }
        "explicit_role" => {
            "use this bounded startup view to initialize the selected non-orchestrator lane; execution still requires a lawful packet or bounded worker request"
        }
        _ => {
            "treat this surface as activation/view-only runtime context; it does not by itself execute work or transfer root-session write authority"
        }
    };

    serde_json::json!({
        "activation_kind": "activation_view",
        "view_only": true,
        "executes_packet": false,
        "records_completion_receipt": false,
        "transfers_root_session_write_authority": false,
        "root_session_write_guard_remains_authoritative": true,
        "tracked_flow_shaping_only": tracked_flow_shaping_only,
        "next_lawful_action": next_lawful_action,
    })
}

fn agent_init_packet_selection(
    packet_path: &str,
    packet: serde_json::Value,
    downstream: bool,
) -> Result<serde_json::Value, String> {
    let selected_role = packet
        .get("activation_runtime_role")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            packet
                .get("role_selection")
                .and_then(|value| value.get("selected_role"))
                .and_then(serde_json::Value::as_str)
        })
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    if selected_role == "orchestrator" || selected_role == "unknown" {
        return Err(
            "Packet activation requires a non-orchestrator runtime role in the dispatch packet."
                .to_string(),
        );
    }

    let dispatch_target_key = if downstream {
        "downstream_dispatch_target"
    } else {
        "dispatch_target"
    };
    let packet_path_key = if downstream {
        "downstream_packet_path"
    } else {
        "dispatch_packet_path"
    };

    Ok(serde_json::json!({
        "mode": if downstream { "downstream_packet" } else { "dispatch_packet" },
        "selected_role": selected_role,
        "request_text": packet.get("request_text").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "dispatch_target": packet.get(dispatch_target_key).and_then(serde_json::Value::as_str).unwrap_or_default(),
        packet_path_key: packet_path,
        "packet_kind": packet.get("packet_kind").cloned().unwrap_or(serde_json::Value::Null),
        "packet_template_kind": packet.get("packet_template_kind").cloned().unwrap_or(serde_json::Value::Null),
        "packet": packet,
    }))
}

fn agent_init_execution_truth(selection: &serde_json::Value) -> serde_json::Value {
    let packet = selection
        .get("packet")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if packet.is_null() {
        return serde_json::Value::Null;
    }

    packet
        .get("execution_truth")
        .cloned()
        .or_else(|| {
            let role_selection = packet
                .get("role_selection_full")
                .cloned()
                .and_then(|value| {
                    serde_json::from_value::<super::RuntimeConsumptionLaneSelection>(value).ok()
                })?;
            let dispatch_target = selection
                .get("dispatch_target")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            Some(
                super::runtime_dispatch_state::dispatch_execution_route_summary(
                    &role_selection,
                    dispatch_target,
                    packet
                        .get("selected_backend")
                        .and_then(serde_json::Value::as_str),
                    packet
                        .get("selected_backend_override")
                        .and_then(serde_json::Value::as_str),
                ),
            )
        })
        .unwrap_or(serde_json::Value::Null)
}

fn agent_init_backend_truth(
    selection: &serde_json::Value,
    execution_truth: &serde_json::Value,
) -> serde_json::Value {
    let Some(execution_truth) = execution_truth.as_object() else {
        return serde_json::Value::Null;
    };
    let role_selection = selection
        .get("packet")
        .and_then(|packet| packet.get("role_selection_full"))
        .cloned()
        .and_then(|value| {
            serde_json::from_value::<super::RuntimeConsumptionLaneSelection>(value).ok()
        });
    let runtime_assignment = role_selection
        .as_ref()
        .map(|role_selection| {
            super::runtime_assignment_from_execution_plan(&role_selection.execution_plan).clone()
        })
        .unwrap_or(serde_json::Value::Null);
    let selected_backend = execution_truth
        .get("effective_selected_backend")
        .or_else(|| execution_truth.get("selected_backend"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_primary_backend = execution_truth
        .get("route_primary_backend")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let route_fallback_backend = execution_truth
        .get("route_fallback_backend")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_backend_source = execution_truth
        .get("selected_backend_source")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let effective_execution_posture = execution_truth
        .get("effective_execution_posture")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_backend_class = execution_truth
        .get("selected_backend_class")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_carrier_id = runtime_assignment
        .get("selected_carrier_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_model_profile_id = runtime_assignment
        .get("selected_model_profile_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let selected_backend_str = selected_backend.as_str().filter(|value| !value.is_empty());
    let route_primary_backend_str = route_primary_backend
        .as_str()
        .filter(|value| !value.is_empty());
    let selected_carrier_str = selected_carrier_id
        .as_str()
        .filter(|value| !value.is_empty());
    let override_active = matches!(
        (selected_backend_str, route_primary_backend_str),
        (Some(selected), Some(primary)) if selected != primary
    );
    let dynamic_carrier_matches_effective_backend =
        match (selected_carrier_str, selected_backend_str) {
            (Some(selected_carrier), Some(effective_backend)) => {
                serde_json::Value::Bool(selected_carrier == effective_backend)
            }
            _ => serde_json::Value::Null,
        };
    let dynamic_carrier_matches_route_primary_backend =
        match (selected_carrier_str, route_primary_backend_str) {
            (Some(selected_carrier), Some(route_primary_backend)) => {
                serde_json::Value::Bool(selected_carrier == route_primary_backend)
            }
            _ => serde_json::Value::Null,
        };

    let lawful_override = if override_active {
        role_selection
            .as_ref()
            .and_then(|role_selection| {
                let dispatch_target = selection
                    .get("dispatch_target")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                selected_backend_str.map(|backend_id| {
                    super::runtime_dispatch_state::backend_is_admissible_for_dispatch_target(
                        &role_selection.execution_plan,
                        backend_id,
                        dispatch_target,
                    )
                })
            })
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Bool(false)
    };
    let override_status = if !override_active {
        "not_needed"
    } else if lawful_override == serde_json::Value::Bool(true) {
        "lawful"
    } else if lawful_override == serde_json::Value::Bool(false) {
        "inadmissible"
    } else {
        "unknown"
    };

    serde_json::json!({
        "selected_backend": selected_backend,
        "selected_backend_source": selected_backend_source,
        "backend_selection_source": selected_backend_source,
        "selected_backend_class": selected_backend_class,
        "selected_carrier_id": selected_carrier_id,
        "selected_model_profile_id": selected_model_profile_id,
        "dynamic_carrier_matches_effective_backend": dynamic_carrier_matches_effective_backend,
        "dynamic_carrier_matches_route_primary_backend": dynamic_carrier_matches_route_primary_backend,
        "route_primary_backend": route_primary_backend,
        "route_fallback_backend": route_fallback_backend,
        "effective_execution_posture": effective_execution_posture,
        "override_active": override_active,
        "lawful_override": lawful_override,
        "override_status": override_status,
    })
}

fn build_agent_init_surface_payload(
    init_view: serde_json::Value,
    selection: serde_json::Value,
    activation_semantics: serde_json::Value,
    runtime_bundle_summary: serde_json::Value,
    dev_team_readiness: serde_json::Value,
) -> serde_json::Value {
    let execution_truth = agent_init_execution_truth(&selection);
    let backend_truth = agent_init_backend_truth(&selection, &execution_truth);
    let packet_activation_evidence = selection
        .get("packet")
        .and_then(|packet| packet.get("activation_evidence"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let dev_team_readiness = enrich_dev_team_readiness_with_agent_selection(
        dev_team_readiness,
        &selection,
        &backend_truth,
    );

    serde_json::json!({
        "surface": "vida agent-init",
        "init": init_view,
        "selection": selection,
        "activation_semantics": activation_semantics,
        "execution_truth": execution_truth,
        "backend_truth": backend_truth,
        "dev_team_readiness": dev_team_readiness,
        "packet_activation_evidence": packet_activation_evidence,
        "runtime_bundle_summary": runtime_bundle_summary,
    })
}

fn enrich_dev_team_readiness_with_agent_selection(
    mut dev_team_readiness: serde_json::Value,
    selection: &serde_json::Value,
    backend_truth: &serde_json::Value,
) -> serde_json::Value {
    let Some(dev_team_object) = dev_team_readiness.as_object_mut() else {
        return dev_team_readiness;
    };
    let selected_runtime_role = selection
        .get("selected_role")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty());
    let selected_dev_team_role = dev_team_object
        .get("roles")
        .and_then(serde_json::Value::as_array)
        .and_then(|roles| {
            roles
                .iter()
                .find(|role| role["runtime_role"].as_str() == selected_runtime_role)
        })
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let selected_model = selected_dev_team_role
        .get("selected_model")
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "model_ref": selected_dev_team_role
                    .get("default_model")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            })
        });
    let selected_cost_units = selected_dev_team_role
        .get("cost_policy")
        .and_then(|cost| cost.get("budget_units"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    dev_team_object.insert(
        "active_selection".to_string(),
        serde_json::json!({
            "selected_runtime_role": selected_runtime_role,
            "selected_dev_team_role": selected_dev_team_role.get("role_id").cloned().unwrap_or(serde_json::Value::Null),
            "selected_backend": backend_truth.get("selected_backend").cloned().unwrap_or(serde_json::Value::Null),
            "selected_carrier_id": backend_truth.get("selected_carrier_id").cloned().unwrap_or(serde_json::Value::Null),
            "selected_model_profile_id": backend_truth.get("selected_model_profile_id").cloned().unwrap_or(serde_json::Value::Null),
            "selected_model": selected_model,
            "selected_cost_units": selected_cost_units,
        }),
    );
    dev_team_readiness
}

pub(crate) async fn render_agent_init_packet_activation_with_store(
    store: &super::StateStore,
    project_root: &Path,
    packet_path: &str,
    downstream: bool,
) -> Result<serde_json::Value, String> {
    let instruction_source_root =
        PathBuf::from(super::state_store::DEFAULT_INSTRUCTION_SOURCE_ROOT);
    let framework_memory_source_root =
        PathBuf::from(super::state_store::DEFAULT_FRAMEWORK_MEMORY_SOURCE_ROOT);
    super::ensure_launcher_bootstrap(
        store,
        &instruction_source_root,
        &framework_memory_source_root,
    )
    .await?;
    let bundle = build_taskflow_consume_bundle_payload(store).await?;
    let packet = super::taskflow_consume_resume::read_dispatch_packet(packet_path)?;
    let selection = agent_init_packet_selection(packet_path, packet, downstream)?;

    let project_activation_view =
        super::project_activator_surface::build_project_activator_view(project_root);
    let init_view = super::project_activator_surface::merge_project_activation_into_init_view(
        bundle.agent_init_view,
        &project_activation_view,
    );
    let activation_semantics = agent_init_activation_semantics(&selection);

    Ok(build_agent_init_surface_payload(
        init_view,
        selection,
        activation_semantics,
        serde_json::json!({
            "bundle_id": bundle.metadata["bundle_id"],
            "activation_source": bundle.activation_source,
            "vida_root": bundle.vida_root,
            "state_dir": store.root().display().to_string(),
            "launcher_runtime_paths": bundle.launcher_runtime_paths,
        }),
        super::taskflow_consume_bundle::build_dev_team_readiness(
            &bundle.config_path,
            &bundle.activation_bundle,
        ),
    ))
}

#[cfg(test)]
mod agent_init_surface_tests {
    use super::*;
    use crate::run;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::{cli, guard_current_dir};
    use crate::RuntimeConsumptionLaneSelection;
    use std::path::Path;
    use std::process::ExitCode;
    use std::time::{Duration, Instant};

    fn wait_for_state_unlock(state_dir: &Path) {
        let direct_lock_path = state_dir.join("LOCK");
        let nested_lock_path = state_dir
            .join(".vida")
            .join("data")
            .join("state")
            .join("LOCK");
        let deadline = Instant::now() + Duration::from_secs(2);
        while (direct_lock_path.exists() || nested_lock_path.exists()) && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn test_role_selection() -> RuntimeConsumptionLaneSelection {
        RuntimeConsumptionLaneSelection {
            ok: true,
            activation_source: "test".to_string(),
            selection_mode: "fixed".to_string(),
            fallback_role: "orchestrator".to_string(),
            request: "continue development".to_string(),
            selected_role: "worker".to_string(),
            conversational_mode: None,
            single_task_only: true,
            tracked_flow_entry: Some("dev-pack".to_string()),
            allow_freeform_chat: false,
            confidence: "high".to_string(),
            matched_terms: vec!["development".to_string()],
            compiled_bundle: serde_json::Value::Null,
            execution_plan: serde_json::json!({
                "development_flow": {
                    "implementer": {
                        "executor_backend": "hermes_cli",
                        "fallback_executor_backend": "internal_subagents"
                    }
                },
                "runtime_assignment": {
                    "selected_carrier_id": "junior",
                    "selected_backend_id": "junior",
                    "selected_model_profile_id": "codex_gpt54_mini_impl",
                    "selected_tier": "junior",
                    "activation_agent_type": "junior",
                    "activation_runtime_role": "worker"
                },
                "backend_admissibility_matrix": [
                    {
                        "backend_id": "hermes_cli",
                        "backend_class": "external_cli",
                        "lane_admissibility": {
                            "implementation": false
                        }
                    },
                    {
                        "backend_id": "internal_subagents",
                        "backend_class": "internal",
                        "lane_admissibility": {
                            "implementation": true
                        }
                    }
                ]
            }),
            reason: "test".to_string(),
        }
    }

    #[test]
    fn agent_init_surface_payload_exposes_execution_truth_selected_backend() {
        let role_selection = test_role_selection();
        let selection = agent_init_packet_selection(
            "/tmp/dispatch.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "fix runtime handoff",
                "dispatch_target": "implementer",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "selected_backend": "internal_subagents",
                "role_selection_full": role_selection,
            }),
            false,
        )
        .expect("packet selection should build");
        let payload = build_agent_init_surface_payload(
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            serde_json::json!({
                "status": "ready",
                "roles": [
                    {
                        "role_id": "developer",
                        "runtime_role": "worker",
                        "default_model": "gpt-5.4",
                        "cost_policy": {
                            "budget_units": 1
                        }
                    }
                ]
            }),
        );

        assert_eq!(
            payload["execution_truth"]["effective_selected_backend"],
            "internal_subagents"
        );
        assert_eq!(
            payload["backend_truth"]["selected_backend"],
            "internal_subagents"
        );
        assert_eq!(payload["backend_truth"]["selected_carrier_id"], "junior");
        assert_eq!(
            payload["backend_truth"]["selected_model_profile_id"],
            "codex_gpt54_mini_impl"
        );
        assert_eq!(
            payload["backend_truth"]["backend_selection_source"],
            "dynamic_runtime_selection"
        );
        assert_eq!(
            payload["backend_truth"]["dynamic_carrier_matches_effective_backend"],
            false
        );
        assert_eq!(
            payload["backend_truth"]["dynamic_carrier_matches_route_primary_backend"],
            false
        );
        assert_eq!(
            payload["backend_truth"]["route_primary_backend"],
            "hermes_cli"
        );
        assert_eq!(
            payload["backend_truth"]["route_fallback_backend"],
            "internal_subagents"
        );
        assert_eq!(payload["backend_truth"]["override_active"], true);
        assert_eq!(payload["backend_truth"]["lawful_override"], true);
        assert_eq!(payload["backend_truth"]["override_status"], "lawful");
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_dev_team_role"],
            "developer"
        );
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_carrier_id"],
            "junior"
        );
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_cost_units"],
            1
        );
        assert_eq!(
            payload["dev_team_readiness"]["active_selection"]["selected_model"]["model_ref"],
            "gpt-5.4"
        );
    }

    #[test]
    fn agent_init_surface_payload_exposes_route_fallback_backend_for_downstream_packet() {
        let selection = agent_init_packet_selection(
            "/tmp/downstream.json",
            serde_json::json!({
                "activation_runtime_role": "worker",
                "request_text": "fix runtime handoff",
                "downstream_dispatch_target": "implementer",
                "packet_kind": "runtime_dispatch_packet",
                "packet_template_kind": "delivery_task_packet",
                "role_selection_full": test_role_selection(),
            }),
            true,
        )
        .expect("downstream packet selection should build");
        let payload = build_agent_init_surface_payload(
            serde_json::json!({ "status": "ready" }),
            selection,
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert_eq!(payload["selection"]["mode"], "downstream_packet");
        assert_eq!(
            payload["execution_truth"]["route_primary_backend"],
            "hermes_cli"
        );
        assert_eq!(
            payload["execution_truth"]["route_fallback_backend"],
            "internal_subagents"
        );
        assert_eq!(payload["backend_truth"]["override_status"], "lawful");
    }

    #[test]
    fn agent_init_surface_payload_keeps_backend_truth_null_without_packet_mode() {
        let payload = build_agent_init_surface_payload(
            serde_json::json!({ "status": "ready" }),
            serde_json::json!({
                "mode": "explicit_role",
                "selected_role": "worker",
                "request_text": "repair"
            }),
            serde_json::json!({ "activation_kind": "activation_view" }),
            serde_json::json!({ "bundle_id": "bundle-test" }),
            serde_json::json!({ "status": "ready", "roles": [] }),
        );

        assert!(payload["execution_truth"].is_null());
        assert!(payload["backend_truth"].is_null());
    }

    #[test]
    fn orchestrator_init_succeeds_after_init_scaffold() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(
            runtime.block_on(run(cli(&["orchestrator-init", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn agent_init_succeeds_after_init_scaffold() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());
        assert_eq!(
            runtime.block_on(run(cli(&["agent-init", "--role", "worker", "--json"]))),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn parallel_agent_init_role_views_do_not_contend_on_write_open() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
        let harness = TempStateHarness::new().expect("temp state harness should initialize");
        let _cwd = guard_current_dir(harness.path());

        assert_eq!(runtime.block_on(run(cli(&["init"]))), ExitCode::SUCCESS);
        assert_eq!(runtime.block_on(run(cli(&["boot"]))), ExitCode::SUCCESS);
        wait_for_state_unlock(harness.path());

        let results = runtime.block_on(async {
            tokio::join!(
                run(cli(&["agent-init", "--role", "worker", "--json"])),
                run(cli(&["agent-init", "--role", "worker", "--json"])),
                run(cli(&["agent-init", "--role", "worker", "--json"])),
                run(cli(&["agent-init", "--role", "worker", "--json"]))
            )
        });

        assert_eq!(results.0, ExitCode::SUCCESS);
        assert_eq!(results.1, ExitCode::SUCCESS);
        assert_eq!(results.2, ExitCode::SUCCESS);
        assert_eq!(results.3, ExitCode::SUCCESS);
    }
}
