use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    OrchestratorSessionArgs, OrchestratorSessionCommand, OrchestratorSessionReclaimArgs,
    OrchestratorSessionShowArgs, OrchestratorSessionTransferArgs,
};

const SESSION_TTL_SECONDS: i64 = 2 * 60 * 60;
const UPSTREAM_VIDA_ISSUE_OWNER: &str = "pomazanbohdan/vida-stack";

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn session_store_path(state_dir: &Path) -> PathBuf {
    state_dir
        .join("orchestrator-sessions")
        .join("sessions.json")
}

fn sanitized_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn current_session_id() -> String {
    sanitized_env("VIDA_ORCHESTRATOR_SESSION_ID")
        .or_else(|| sanitized_env("CODEX_SESSION_ID"))
        .or_else(|| sanitized_env("CODEX_THREAD_ID"))
        .unwrap_or_else(|| format!("local-pid-{}", std::process::id()))
}

fn host_tool_identity() -> String {
    sanitized_env("VIDA_HOST_TOOL")
        .or_else(|| sanitized_env("CODEX_APP"))
        .unwrap_or_else(|| "codex_desktop_or_local_cli".to_string())
}

fn canonicalized_current_dir() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|path| std::fs::canonicalize(path).ok())
        .or_else(|| std::env::current_dir().ok())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_sessions(path: &Path) -> Vec<serde_json::Value> {
    let Ok(body) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("sessions")
                .and_then(|sessions| sessions.as_array())
                .cloned()
        })
        .unwrap_or_default()
}

fn write_sessions(path: &Path, sessions: &[serde_json::Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create orchestrator session dir: {error}"))?;
    }
    let payload = serde_json::json!({
        "schema_version": "runtime-owner-evidence-v1",
        "updated_at_epoch_seconds": now_epoch_seconds(),
        "sessions": sessions,
    });
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("serialize orchestrator sessions: {error}"))?;
    std::fs::write(path, body).map_err(|error| format!("write orchestrator sessions: {error}"))
}

fn current_session_record(state_dir: &Path) -> serde_json::Value {
    let now = now_epoch_seconds();
    serde_json::json!({
        "session_id": current_session_id(),
        "owner_kind": "orchestrator",
        "state": "live",
        "host_tool": host_tool_identity(),
        "process_id": std::process::id(),
        "project_root": canonicalized_current_dir(),
        "state_dir": state_dir.display().to_string(),
        "worktree_environment_id": sanitized_env("VIDA_WORKTREE_ID")
            .or_else(|| sanitized_env("GIT_DIR"))
            .unwrap_or_else(|| canonicalized_current_dir()),
        "started_at_epoch_seconds": now,
        "last_heartbeat_epoch_seconds": now,
        "owner_annotation": "current_session",
    })
}

fn merge_current_session(
    mut sessions: Vec<serde_json::Value>,
    current: serde_json::Value,
) -> Vec<serde_json::Value> {
    let current_id = current["session_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let mut replaced = false;
    for session in &mut sessions {
        if session["session_id"].as_str() == Some(current_id.as_str()) {
            let started_at = session["started_at_epoch_seconds"].clone();
            *session = current.clone();
            if !started_at.is_null() {
                session["started_at_epoch_seconds"] = started_at;
            }
            replaced = true;
        }
    }
    if !replaced {
        sessions.push(current);
    }
    sessions
}

fn classify_sessions(
    sessions: &[serde_json::Value],
    current_id: &str,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let now = now_epoch_seconds();
    let mut live_other = Vec::new();
    let mut stale = Vec::new();
    for session in sessions {
        let session_id = session["session_id"].as_str().unwrap_or_default();
        if session_id == current_id {
            continue;
        }
        let heartbeat = session["last_heartbeat_epoch_seconds"]
            .as_i64()
            .unwrap_or(0);
        let state = session["state"]
            .as_str()
            .unwrap_or("legacy_global_owner_unknown");
        if state == "live" && now.saturating_sub(heartbeat) <= SESSION_TTL_SECONDS {
            live_other.push(session.clone());
        } else {
            let mut cloned = session.clone();
            cloned["state"] = serde_json::Value::String(if state.trim().is_empty() {
                "legacy_global_owner_unknown".to_string()
            } else {
                "stale".to_string()
            });
            stale.push(cloned);
        }
    }
    (live_other, stale)
}

pub(crate) fn build_runtime_owner_evidence(
    state_dir: &Path,
    persist_current: bool,
) -> Result<serde_json::Value, String> {
    let path = session_store_path(state_dir);
    let current = current_session_record(state_dir);
    let current_id = current["session_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let mut sessions = read_sessions(&path);
    sessions = merge_current_session(sessions, current.clone());
    if persist_current {
        write_sessions(&path, &sessions)?;
    }
    let (live_other_sessions, stale_sessions) = classify_sessions(&sessions, &current_id);
    let mutation_gate = if live_other_sessions.is_empty() {
        "current_session_allowed"
    } else {
        "blocked_live_other_orchestrator"
    };
    let blocker_codes = if live_other_sessions.is_empty() {
        Vec::<String>::new()
    } else {
        vec!["live_other_orchestrator_owner".to_string()]
    };
    let next_actions = if live_other_sessions.is_empty() {
        Vec::<String>::new()
    } else {
        vec![
            "Inspect `vida orchestrator-session show --json`; reclaim only stale sessions with `vida orchestrator-session reclaim <session-id> --json`.".to_string(),
        ]
    };
    Ok(serde_json::json!({
        "schema_version": "runtime-owner-evidence-v1",
        "current_session": current,
        "live_other_sessions": live_other_sessions,
        "stale_sessions": stale_sessions,
        "legacy_ownerless_rows": {
            "annotation": "legacy_global_owner_unknown",
            "admissible_for_backward_compatibility": true,
        },
        "downstream_execution_context": {
            "project_root": canonicalized_current_dir(),
            "state_dir": state_dir.display().to_string(),
            "git_remote_context_is_publication_owner": false,
        },
        "upstream_vida_owner_publication_context": {
            "issue_owner": UPSTREAM_VIDA_ISSUE_OWNER,
            "issue_tracker_url": format!("https://github.com/{UPSTREAM_VIDA_ISSUE_OWNER}/issues"),
            "source": "canonical_runtime_self_diagnostic_policy",
        },
        "mutation_gate": mutation_gate,
        "blocker_codes": blocker_codes,
        "next_actions": next_actions,
        "session_store_path": path.display().to_string(),
    }))
}

fn print_or_plain(payload: &serde_json::Value, as_json: bool) {
    if as_json {
        crate::print_json_pretty(payload);
    } else {
        println!("VIDA orchestrator session");
        println!("status: {}", payload["status"].as_str().unwrap_or("pass"));
        println!(
            "current_session: {}",
            payload["runtime_owner_evidence"]["current_session"]["session_id"]
                .as_str()
                .unwrap_or("unknown")
        );
        println!(
            "mutation_gate: {}",
            payload["runtime_owner_evidence"]["mutation_gate"]
                .as_str()
                .unwrap_or("unknown")
        );
    }
}

fn run_show(args: OrchestratorSessionShowArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(crate::state_store::default_state_dir);
    match build_runtime_owner_evidence(&state_dir, true) {
        Ok(evidence) => {
            let blocked = evidence["mutation_gate"] == "blocked_live_other_orchestrator";
            let payload = serde_json::json!({
                "surface": "vida orchestrator-session show",
                "status": if blocked { "blocked" } else { "pass" },
                "blocker_codes": evidence["blocker_codes"].clone(),
                "next_actions": evidence["next_actions"].clone(),
                "runtime_owner_evidence": evidence,
            });
            print_or_plain(&payload, args.json);
            if blocked {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn mutate_session(
    state_dir: PathBuf,
    session_id: &str,
    mutation: &str,
) -> Result<serde_json::Value, String> {
    let path = session_store_path(&state_dir);
    let mut sessions =
        merge_current_session(read_sessions(&path), current_session_record(&state_dir));
    let mut found = false;
    for session in &mut sessions {
        if session["session_id"].as_str() == Some(session_id) {
            session["state"] = serde_json::Value::String(mutation.to_string());
            session["reclaimed_by_session_id"] = serde_json::Value::String(current_session_id());
            session["reclaimed_at_epoch_seconds"] =
                serde_json::Value::Number(now_epoch_seconds().into());
            found = true;
        }
    }
    if !found {
        return Err(format!("orchestrator session `{session_id}` is missing"));
    }
    write_sessions(&path, &sessions)?;
    build_runtime_owner_evidence(&state_dir, true)
}

fn run_reclaim(args: OrchestratorSessionReclaimArgs) -> ExitCode {
    let state_dir = args
        .state_dir
        .unwrap_or_else(crate::state_store::default_state_dir);
    match mutate_session(state_dir, &args.session_id, "reclaimed") {
        Ok(evidence) => {
            let payload = serde_json::json!({
                "surface": "vida orchestrator-session reclaim",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "reclaimed_session_id": args.session_id,
                "runtime_owner_evidence": evidence,
            });
            print_or_plain(&payload, args.json);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run_transfer(args: OrchestratorSessionTransferArgs) -> ExitCode {
    if !args.to_current {
        eprintln!("transfer requires --to-current");
        return ExitCode::from(1);
    }
    let state_dir = args
        .state_dir
        .unwrap_or_else(crate::state_store::default_state_dir);
    match mutate_session(state_dir, &args.session_id, "transferred_to_current") {
        Ok(evidence) => {
            let payload = serde_json::json!({
                "surface": "vida orchestrator-session transfer",
                "status": "pass",
                "blocker_codes": [],
                "next_actions": [],
                "transferred_session_id": args.session_id,
                "runtime_owner_evidence": evidence,
            });
            print_or_plain(&payload, args.json);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

pub(crate) async fn run_orchestrator_session(args: OrchestratorSessionArgs) -> ExitCode {
    match args.command {
        OrchestratorSessionCommand::Show(args) => run_show(args),
        OrchestratorSessionCommand::Reclaim(args) => run_reclaim(args),
        OrchestratorSessionCommand::Transfer(args) => run_transfer(args),
    }
}

pub(crate) fn issue_owner() -> &'static str {
    UPSTREAM_VIDA_ISSUE_OWNER
}

pub(crate) fn context_summary_map(state_dir: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert("project_root".to_string(), canonicalized_current_dir());
    map.insert("state_dir".to_string(), state_dir.display().to_string());
    map.insert(
        "upstream_issue_owner".to_string(),
        issue_owner().to_string(),
    );
    map
}

#[cfg(test)]
mod tests {
    use super::{build_runtime_owner_evidence, context_summary_map};
    use crate::temp_state::TempStateHarness;

    #[test]
    fn orchestrator_session_evidence_distinguishes_upstream_issue_owner() {
        let harness = TempStateHarness::new().expect("temp state should initialize");
        let evidence = build_runtime_owner_evidence(harness.path(), false)
            .expect("owner evidence should build");
        assert_eq!(
            evidence["upstream_vida_owner_publication_context"]["issue_owner"],
            "pomazanbohdan/vida-stack"
        );
        assert_eq!(
            evidence["downstream_execution_context"]["git_remote_context_is_publication_owner"],
            false
        );
        assert_eq!(
            context_summary_map(harness.path())
                .get("upstream_issue_owner")
                .map(String::as_str),
            Some("pomazanbohdan/vida-stack")
        );
    }
}
