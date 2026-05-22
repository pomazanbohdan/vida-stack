use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    OrchestratorSessionArgs, OrchestratorSessionCommand, OrchestratorSessionReclaimArgs,
    OrchestratorSessionShowArgs, OrchestratorSessionTransferArgs,
};

const SESSION_TTL_SECONDS: i64 = 2 * 60 * 60;
const STALE_SESSION_PURGE_AFTER_SECONDS: i64 = SESSION_TTL_SECONDS * 2;
const UPSTREAM_VIDA_ISSUE_OWNER: &str = "pomazanbohdan/vida-stack";
const MAX_SESSION_STORE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrchestratorSessionLiveness {
    Current,
    LiveOther,
    Stale,
    Unknown,
}

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

fn stable_hash_hex(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn canonicalized_path_string(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn stable_local_session_id(state_dir: &Path) -> String {
    let worktree = canonicalized_current_dir();
    let state_dir = canonicalized_path_string(state_dir);
    format!(
        "local-worktree-{}",
        stable_hash_hex(&format!("{worktree}\n{state_dir}"))
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VidaSessionIdentity {
    session_id: String,
    identity_source: String,
    legacy_stable_worktree_id: Option<String>,
}

fn generated_local_session_id(state_dir: &Path) -> String {
    let worktree = canonicalized_current_dir();
    let state_dir = canonicalized_path_string(state_dir);
    let context_hash = stable_hash_hex(&format!("{worktree}\n{state_dir}"));
    format!("local-session-{context_hash}")
}

fn resolve_vida_session_identity(state_dir: &Path) -> VidaSessionIdentity {
    for (env_name, source) in [
        ("VIDA_SESSION_ID", "VIDA_SESSION_ID"),
        (
            "VIDA_ORCHESTRATOR_SESSION_ID",
            "VIDA_ORCHESTRATOR_SESSION_ID",
        ),
        ("CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_SESSION_ID"),
        (
            "CLAUDE_CODE_REMOTE_SESSION_ID",
            "CLAUDE_CODE_REMOTE_SESSION_ID",
        ),
        ("CODEX_SESSION_ID", "CODEX_SESSION_ID"),
        ("CODEX_THREAD_ID", "CODEX_THREAD_ID"),
    ] {
        if let Some(session_id) = sanitized_env(env_name) {
            return VidaSessionIdentity {
                session_id,
                identity_source: source.to_string(),
                legacy_stable_worktree_id: None,
            };
        }
    }

    VidaSessionIdentity {
        session_id: generated_local_session_id(state_dir),
        identity_source: "generated_local_session_token".to_string(),
        legacy_stable_worktree_id: Some(stable_local_session_id(state_dir)),
    }
}

fn current_session_id(state_dir: &Path) -> String {
    resolve_vida_session_identity(state_dir).session_id
}

fn current_session_identity_source(state_dir: &Path) -> String {
    resolve_vida_session_identity(state_dir).identity_source
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
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Vec::new();
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Vec::new();
    }
    if metadata.len() > MAX_SESSION_STORE_BYTES {
        return Vec::new();
    }
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

fn create_dir_all_without_symlinks(path: &Path) -> Result<(), String> {
    let mut cursor = PathBuf::new();
    for component in path.components() {
        cursor.push(component.as_os_str());
        if cursor.exists() {
            let metadata = std::fs::symlink_metadata(&cursor)
                .map_err(|error| format!("inspect path {}: {error}", cursor.display()))?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "refusing to use symlinked orchestrator session dir segment: {}",
                    cursor.display()
                ));
            }
            if !metadata.is_dir() {
                return Err(format!(
                    "orchestrator session dir segment is not a directory: {}",
                    cursor.display()
                ));
            }
            continue;
        }
        std::fs::create_dir(&cursor).map_err(|error| {
            format!(
                "create orchestrator session dir {}: {error}",
                cursor.display()
            )
        })?;
    }
    Ok(())
}

fn write_sessions(path: &Path, sessions: &[serde_json::Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        create_dir_all_without_symlinks(parent)?;
    }
    if std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(format!(
            "refusing to write orchestrator sessions through symlink: {}",
            path.display()
        ));
    }
    let payload = serde_json::json!({
        "schema_version": "runtime-owner-evidence-v1",
        "updated_at_epoch_seconds": now_epoch_seconds(),
        "sessions": sessions,
    });
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("serialize orchestrator sessions: {error}"))?;
    if body.len() as u64 > MAX_SESSION_STORE_BYTES {
        return Err(format!(
            "orchestrator sessions payload exceeds {MAX_SESSION_STORE_BYTES} bytes"
        ));
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("open orchestrator sessions for write: {error}"))?;
    std::io::Write::write_all(&mut file, body.as_bytes())
        .map_err(|error| format!("write orchestrator sessions: {error}"))
}

fn current_session_record(state_dir: &Path) -> serde_json::Value {
    let now = now_epoch_seconds();
    let identity = resolve_vida_session_identity(state_dir);
    let mut record = serde_json::json!({
        "session_id": identity.session_id,
        "identity_source": identity.identity_source,
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
    });
    if let Some(legacy_id) = identity.legacy_stable_worktree_id {
        record["fallback_replaces_legacy_stable_worktree_state_hash"] =
            serde_json::Value::String(legacy_id);
    }
    record
}

fn merge_current_session(
    mut sessions: Vec<serde_json::Value>,
    current: serde_json::Value,
    state_dir: &Path,
) -> Vec<serde_json::Value> {
    let current_id = current["session_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let stable_fallback_id = stable_local_session_id(state_dir);
    let legacy_synthesized_prefix =
        stable_fallback_id.replacen("local-worktree-", "local-session-", 1);
    sessions.retain(|session| {
        !(session["identity_source"].as_str() == Some("synthesized_local_session_token")
            && (session["fallback_replaces_legacy_stable_worktree_state_hash"].as_str()
                == Some(stable_fallback_id.as_str())
                || session["session_id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with(&format!("{legacy_synthesized_prefix}-")))))
    });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessLiveness {
    Alive,
    Dead,
    Unknown,
}

#[cfg(target_os = "windows")]
fn local_process_liveness(process_id: u32) -> ProcessLiveness {
    if process_id == std::process::id() {
        return ProcessLiveness::Alive;
    }

    static TASKLIST_CACHE: std::sync::OnceLock<
        std::sync::Mutex<Option<(std::time::Instant, std::collections::BTreeSet<u32>)>>,
    > = std::sync::OnceLock::new();

    let cache = TASKLIST_CACHE.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some((cached_at, pids)) = guard.as_ref() {
            if cached_at.elapsed() < std::time::Duration::from_secs(5) {
                return if pids.contains(&process_id) {
                    ProcessLiveness::Alive
                } else {
                    ProcessLiveness::Dead
                };
            }
        }
    }

    let snapshot = match std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let fields = line
                        .trim()
                        .trim_matches('"')
                        .split("\",\"")
                        .collect::<Vec<_>>();
                    fields.get(1).and_then(|value| value.parse::<u32>().ok())
                })
                .collect::<std::collections::BTreeSet<_>>()
        }
        _ => return ProcessLiveness::Unknown,
    };

    if let Ok(mut guard) = cache.lock() {
        *guard = Some((std::time::Instant::now(), snapshot.clone()));
    }

    if snapshot.contains(&process_id) {
        ProcessLiveness::Alive
    } else {
        ProcessLiveness::Dead
    }
}

#[cfg(target_os = "linux")]
fn local_process_liveness(process_id: u32) -> ProcessLiveness {
    if process_id == std::process::id() {
        return ProcessLiveness::Alive;
    }
    if std::path::PathBuf::from(format!("/proc/{process_id}")).exists() {
        ProcessLiveness::Alive
    } else {
        ProcessLiveness::Dead
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn local_process_liveness(process_id: u32) -> ProcessLiveness {
    if process_id == std::process::id() {
        ProcessLiveness::Alive
    } else {
        ProcessLiveness::Unknown
    }
}

fn classify_sessions(
    sessions: &[serde_json::Value],
    current_id: &str,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    classify_sessions_with_liveness(sessions, current_id, local_process_liveness)
}

fn classify_sessions_with_liveness(
    sessions: &[serde_json::Value],
    current_id: &str,
    process_liveness: impl Fn(u32) -> ProcessLiveness,
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
        let process_id = session["process_id"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok());
        let heartbeat_fresh = heartbeat <= now && (now - heartbeat) <= SESSION_TTL_SECONDS;
        if state == "live" && heartbeat_fresh && process_id.is_some() {
            let process_is_dead =
                process_id.is_some_and(|value| process_liveness(value) == ProcessLiveness::Dead);
            if process_is_dead {
                let mut cloned = session.clone();
                cloned["state"] = serde_json::Value::String("stale".to_string());
                stale.push(cloned);
                continue;
            }
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

fn purgeable_stale_session(session: &serde_json::Value, now: i64) -> bool {
    let heartbeat = session["last_heartbeat_epoch_seconds"]
        .as_i64()
        .unwrap_or(0);
    if heartbeat <= 0 || heartbeat > now {
        return false;
    }
    now.saturating_sub(heartbeat) > STALE_SESSION_PURGE_AFTER_SECONDS
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
    sessions = merge_current_session(sessions, current.clone(), state_dir);
    let current = sessions
        .iter()
        .find(|session| session["session_id"].as_str() == Some(current_id.as_str()))
        .cloned()
        .unwrap_or(current);
    let (live_other_sessions, stale_sessions) = classify_sessions(&sessions, &current_id);
    if persist_current {
        let now = now_epoch_seconds();
        let mut normalized_sessions =
            Vec::with_capacity(1 + live_other_sessions.len() + stale_sessions.len());
        normalized_sessions.push(current.clone());
        normalized_sessions.extend(live_other_sessions.iter().cloned());
        normalized_sessions.extend(
            stale_sessions
                .iter()
                .filter(|session| !purgeable_stale_session(session, now))
                .cloned(),
        );
        write_sessions(&path, &normalized_sessions)?;
    }
    let mutation_gate = "current_session_allowed";
    let blocker_codes = Vec::<String>::new();
    let next_actions = Vec::<String>::new();
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

pub(crate) fn compact_runtime_owner_evidence_for_operator(
    mut evidence: serde_json::Value,
) -> serde_json::Value {
    if let Some(object) = evidence.as_object_mut() {
        if let Some(stale_sessions) = object.get("stale_sessions").cloned() {
            let count = stale_sessions
                .as_array()
                .map(|sessions| sessions.len())
                .unwrap_or(0);
            object.insert(
                "stale_sessions".to_string(),
                serde_json::json!({
                    "count": count,
                    "detail": "omitted_from_fast_operator_surface"
                }),
            );
        }
    }
    evidence
}

fn session_array_contains_id(array: &serde_json::Value, session_id: &str) -> bool {
    array
        .as_array()
        .into_iter()
        .flatten()
        .any(|session| session["session_id"].as_str() == Some(session_id))
}

pub(crate) fn stale_orchestrator_session_ids_from_evidence(
    evidence: &serde_json::Value,
) -> std::collections::BTreeSet<String> {
    evidence["stale_sessions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|session| session["session_id"].as_str())
        .map(str::to_string)
        .collect()
}

pub(crate) fn orchestrator_session_liveness(
    state_dir: &Path,
    session_id: &str,
) -> Result<OrchestratorSessionLiveness, String> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Ok(OrchestratorSessionLiveness::Unknown);
    }
    let evidence = build_runtime_owner_evidence(state_dir, false)?;
    if evidence["current_session"]["session_id"].as_str() == Some(session_id) {
        return Ok(OrchestratorSessionLiveness::Current);
    }
    if session_array_contains_id(&evidence["live_other_sessions"], session_id) {
        return Ok(OrchestratorSessionLiveness::LiveOther);
    }
    if session_array_contains_id(&evidence["stale_sessions"], session_id) {
        return Ok(OrchestratorSessionLiveness::Stale);
    }
    Ok(OrchestratorSessionLiveness::Unknown)
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
    let mut sessions = merge_current_session(
        read_sessions(&path),
        current_session_record(&state_dir),
        &state_dir,
    );
    let mut found = false;
    for session in &mut sessions {
        if session["session_id"].as_str() == Some(session_id) {
            session["state"] = serde_json::Value::String(mutation.to_string());
            session["reclaimed_by_session_id"] =
                serde_json::Value::String(current_session_id(&state_dir));
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
    use super::{
        build_runtime_owner_evidence, classify_sessions_with_liveness,
        compact_runtime_owner_evidence_for_operator, context_summary_map, current_session_id,
        current_session_identity_source, current_session_record, generated_local_session_id,
        merge_current_session, now_epoch_seconds, read_sessions, stable_local_session_id,
        OrchestratorSessionLiveness, ProcessLiveness, MAX_SESSION_STORE_BYTES,
        STALE_SESSION_PURGE_AFTER_SECONDS,
    };
    use crate::temp_state::TempStateHarness;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn saved_session_env() -> Vec<(&'static str, Option<String>)> {
        [
            "VIDA_SESSION_ID",
            "VIDA_ORCHESTRATOR_SESSION_ID",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_REMOTE_SESSION_ID",
            "CODEX_SESSION_ID",
            "CODEX_THREAD_ID",
        ]
        .into_iter()
        .map(|name| (name, std::env::var(name).ok()))
        .collect()
    }

    fn restore_session_env(saved: Vec<(&'static str, Option<String>)>) {
        for (name, value) in saved {
            unsafe {
                if let Some(value) = value {
                    std::env::set_var(name, value);
                } else {
                    std::env::remove_var(name);
                }
            }
        }
    }

    #[test]
    fn compact_runtime_owner_evidence_replaces_stale_sessions_with_count() {
        let evidence = serde_json::json!({
            "current_session": {"session_id": "current"},
            "live_other_sessions": [],
            "stale_sessions": [
                {"session_id": "stale-1"},
                {"session_id": "stale-2"}
            ],
            "mutation_gate": "current_session_allowed"
        });

        let compacted = compact_runtime_owner_evidence_for_operator(evidence);

        assert_eq!(compacted["stale_sessions"]["count"], 2);
        assert_eq!(
            compacted["stale_sessions"]["detail"],
            "omitted_from_fast_operator_surface"
        );
        assert_eq!(compacted["current_session"]["session_id"], "current");
    }

    fn clear_session_env() {
        for name in [
            "VIDA_SESSION_ID",
            "VIDA_ORCHESTRATOR_SESSION_ID",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_REMOTE_SESSION_ID",
            "CODEX_SESSION_ID",
            "CODEX_THREAD_ID",
        ] {
            unsafe {
                std::env::remove_var(name);
            }
        }
    }

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

    #[test]
    fn fallback_orchestrator_session_identity_uses_generated_local_session_token() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();

        let harness = TempStateHarness::new().expect("temp state should initialize");
        let first = build_runtime_owner_evidence(harness.path(), true)
            .expect("first owner evidence should build");
        let second = build_runtime_owner_evidence(harness.path(), true)
            .expect("second owner evidence should build");

        let first_id = first["current_session"]["session_id"]
            .as_str()
            .expect("first session id should be present");
        let second_id = second["current_session"]["session_id"]
            .as_str()
            .expect("second session id should be present");
        assert_eq!(first_id, second_id);
        assert!(first_id.starts_with("local-session-"));
        assert!(!first_id.starts_with("local-worktree-"));
        assert_eq!(
            second["current_session"]["identity_source"],
            "generated_local_session_token"
        );
        assert_eq!(
            second["current_session"]["fallback_replaces_legacy_stable_worktree_state_hash"],
            stable_local_session_id(harness.path())
        );
        assert!(second["live_other_sessions"].as_array().unwrap().is_empty());
        assert!(!second["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "live_other_orchestrator_owner"));

        restore_session_env(saved);
    }

    #[test]
    fn fallback_orchestrator_session_identity_is_stable_for_cli_reentry() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();

        let harness = TempStateHarness::new().expect("temp state should initialize");
        let expected = stable_local_session_id(harness.path()).replacen(
            "local-worktree-",
            "local-session-",
            1,
        );

        assert_eq!(generated_local_session_id(harness.path()), expected);
        assert_eq!(current_session_id(harness.path()), expected);

        restore_session_env(saved);
    }

    #[test]
    fn stable_fallback_prunes_legacy_synthesized_session_tokens_for_same_worktree() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();

        let harness = TempStateHarness::new().expect("temp state should initialize");
        let stable_id = stable_local_session_id(harness.path());
        let stale_synthesized_with_companion = serde_json::json!({
            "session_id": "local-session-old-process",
            "identity_source": "synthesized_local_session_token",
            "fallback_replaces_legacy_stable_worktree_state_hash": stable_id,
            "state": "live",
            "process_id": 12345,
            "last_heartbeat_epoch_seconds": now_epoch_seconds(),
        });
        let legacy_synthesized_without_companion = serde_json::json!({
            "session_id": stable_id.replacen("local-worktree-", "local-session-", 1) + "-67890-123",
            "identity_source": "synthesized_local_session_token",
            "state": "live",
            "process_id": 67890,
            "last_heartbeat_epoch_seconds": now_epoch_seconds(),
        });
        let current = current_session_record(harness.path());
        let merged = merge_current_session(
            vec![
                stale_synthesized_with_companion,
                legacy_synthesized_without_companion,
            ],
            current.clone(),
            harness.path(),
        );

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0]["session_id"], current["session_id"]);
        assert_eq!(
            merged[0]["fallback_replaces_legacy_stable_worktree_state_hash"],
            stable_id
        );

        restore_session_env(saved);
    }

    #[test]
    fn live_other_orchestrator_is_visible_without_blocking_mutation_gate() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();

        let harness = TempStateHarness::new().expect("temp state should initialize");
        unsafe {
            std::env::set_var("VIDA_ORCHESTRATOR_SESSION_ID", "session-a");
        }
        let _first = build_runtime_owner_evidence(harness.path(), true)
            .expect("first owner evidence should build");

        unsafe {
            std::env::set_var("VIDA_ORCHESTRATOR_SESSION_ID", "session-b");
        }
        let second = build_runtime_owner_evidence(harness.path(), true)
            .expect("second owner evidence should build");

        assert_eq!(second["current_session"]["session_id"], "session-b");
        assert_eq!(second["mutation_gate"], "current_session_allowed");
        assert!(!second["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "live_other_orchestrator_owner"));
        assert!(second["live_other_sessions"]
            .as_array()
            .expect("live other sessions should be present")
            .iter()
            .any(|session| session["session_id"] == "session-a"));
        assert!(second["next_actions"].as_array().unwrap().is_empty());

        restore_session_env(saved);
    }

    #[test]
    fn canonical_vida_session_env_wins_over_legacy_and_host_aliases() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();
        unsafe {
            std::env::set_var("VIDA_SESSION_ID", "canonical-session");
            std::env::set_var("VIDA_ORCHESTRATOR_SESSION_ID", "legacy-session");
            std::env::set_var("CLAUDE_CODE_SESSION_ID", "claude-session");
            std::env::set_var("CODEX_SESSION_ID", "codex-session");
        }

        let harness = TempStateHarness::new().expect("temp state should initialize");
        assert_eq!(current_session_id(harness.path()), "canonical-session");
        let evidence = build_runtime_owner_evidence(harness.path(), false)
            .expect("owner evidence should build");
        assert_eq!(
            evidence["current_session"]["session_id"],
            "canonical-session"
        );
        assert_eq!(
            evidence["current_session"]["identity_source"],
            "VIDA_SESSION_ID"
        );

        restore_session_env(saved);
    }

    #[test]
    fn legacy_and_host_session_aliases_remain_supported() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();

        let harness = TempStateHarness::new().expect("temp state should initialize");
        unsafe {
            std::env::set_var("VIDA_ORCHESTRATOR_SESSION_ID", "legacy-session");
        }
        assert_eq!(current_session_id(harness.path()), "legacy-session");
        assert_eq!(
            current_session_identity_source(harness.path()),
            "VIDA_ORCHESTRATOR_SESSION_ID"
        );
        unsafe {
            std::env::remove_var("VIDA_ORCHESTRATOR_SESSION_ID");
            std::env::set_var("CLAUDE_CODE_SESSION_ID", "claude-session");
        }
        assert_eq!(current_session_id(harness.path()), "claude-session");
        assert_eq!(
            current_session_identity_source(harness.path()),
            "CLAUDE_CODE_SESSION_ID"
        );
        unsafe {
            std::env::remove_var("CLAUDE_CODE_SESSION_ID");
            std::env::set_var("CODEX_THREAD_ID", "codex-thread");
        }
        assert_eq!(current_session_id(harness.path()), "codex-thread");
        assert_eq!(
            current_session_identity_source(harness.path()),
            "CODEX_THREAD_ID"
        );

        restore_session_env(saved);
    }

    #[test]
    fn live_session_with_dead_local_process_is_stale_not_live_other() {
        let now = now_epoch_seconds();
        let sessions = vec![serde_json::json!({
            "session_id": "local-pid-12345",
            "state": "live",
            "process_id": 12345,
            "last_heartbeat_epoch_seconds": now,
        })];

        let (live_other, stale) =
            classify_sessions_with_liveness(&sessions, "current-session", |process_id| {
                assert_eq!(process_id, 12345);
                ProcessLiveness::Dead
            });

        assert!(live_other.is_empty());
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0]["state"], "stale");
    }

    #[test]
    fn live_session_without_process_id_is_stale_not_live_other() {
        let now = now_epoch_seconds();
        let sessions = vec![serde_json::json!({
            "session_id": "missing-pid",
            "state": "live",
            "last_heartbeat_epoch_seconds": now,
        })];

        let (live_other, stale) =
            classify_sessions_with_liveness(&sessions, "current-session", |_| {
                ProcessLiveness::Alive
            });

        assert!(live_other.is_empty());
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0]["state"], "stale");
    }

    #[test]
    fn live_session_with_future_heartbeat_is_stale_not_live_other() {
        let now = now_epoch_seconds();
        let sessions = vec![serde_json::json!({
            "session_id": "future-heartbeat",
            "state": "live",
            "process_id": 12345,
            "last_heartbeat_epoch_seconds": now + 60,
        })];

        let (live_other, stale) =
            classify_sessions_with_liveness(&sessions, "current-session", |_| {
                ProcessLiveness::Unknown
            });

        assert!(live_other.is_empty());
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0]["state"], "stale");
    }
    #[test]
    fn stale_or_expired_sessions_do_not_run_liveness_probe() {
        let now = now_epoch_seconds();
        let sessions = vec![
            serde_json::json!({
                "session_id": "stale-state",
                "state": "stale",
                "process_id": 12345,
                "last_heartbeat_epoch_seconds": now,
            }),
            serde_json::json!({
                "session_id": "expired-live",
                "state": "live",
                "process_id": 23456,
                "last_heartbeat_epoch_seconds": now.saturating_sub(super::SESSION_TTL_SECONDS + 1),
            }),
        ];

        let (live_other, stale) =
            classify_sessions_with_liveness(&sessions, "current-session", |_| {
                panic!("stale or expired sessions must not run process liveness probes")
            });

        assert!(live_other.is_empty());
        assert_eq!(stale.len(), 2);
    }

    #[test]
    fn live_session_with_unknown_process_liveness_remains_live_other() {
        let now = now_epoch_seconds();
        let sessions = vec![serde_json::json!({
            "session_id": "local-pid-12345",
            "state": "live",
            "process_id": 12345,
            "last_heartbeat_epoch_seconds": now,
        })];

        let (live_other, stale) =
            classify_sessions_with_liveness(&sessions, "current-session", |_| {
                ProcessLiveness::Unknown
            });

        assert_eq!(live_other.len(), 1);
        assert!(stale.is_empty());
    }

    #[test]
    fn read_sessions_rejects_oversized_files() {
        let harness = TempStateHarness::new().expect("temp state should initialize");
        let sessions_path = harness
            .path()
            .join("orchestrator-sessions")
            .join("sessions.json");
        std::fs::create_dir_all(sessions_path.parent().unwrap()).expect("parent should create");
        std::fs::write(
            &sessions_path,
            vec![b'x'; (MAX_SESSION_STORE_BYTES as usize) + 1],
        )
        .expect("oversized file should be written");

        assert!(read_sessions(&sessions_path).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn read_sessions_rejects_symlink_targets() {
        use std::os::unix::fs as unix_fs;
        let harness = TempStateHarness::new().expect("temp state should initialize");
        let sessions_dir = harness.path().join("orchestrator-sessions");
        std::fs::create_dir_all(&sessions_dir).expect("sessions directory should create");
        let target = harness.path().join("external.json");
        std::fs::write(&target, r#"{"sessions":[{"session_id":"leak"}]}"#)
            .expect("target should write");
        let sessions_path = sessions_dir.join("sessions.json");
        unix_fs::symlink(&target, &sessions_path).expect("symlink should create");

        assert!(read_sessions(&sessions_path).is_empty());
    }

    #[test]
    fn persist_current_purges_stale_sessions_after_retention_window() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();
        unsafe {
            std::env::set_var("VIDA_ORCHESTRATOR_SESSION_ID", "current-session");
        }

        let harness = TempStateHarness::new().expect("temp state should initialize");
        let sessions_path = harness
            .path()
            .join("orchestrator-sessions")
            .join("sessions.json");
        std::fs::create_dir_all(sessions_path.parent().unwrap()).expect("parent should create");
        let now = now_epoch_seconds();
        let retained_stale = serde_json::json!({
            "session_id": "recent-stale",
            "state": "stale",
            "process_id": 12345,
            "last_heartbeat_epoch_seconds": now.saturating_sub(super::SESSION_TTL_SECONDS + 1),
        });
        let purged_stale = serde_json::json!({
            "session_id": "old-stale",
            "state": "stale",
            "process_id": 23456,
            "last_heartbeat_epoch_seconds": now.saturating_sub(STALE_SESSION_PURGE_AFTER_SECONDS + 1),
        });
        let payload = serde_json::json!({
            "schema_version": "runtime-owner-evidence-v1",
            "sessions": [retained_stale, purged_stale],
        });
        std::fs::write(
            &sessions_path,
            serde_json::to_string_pretty(&payload).expect("serialize sessions"),
        )
        .expect("write sessions");

        let evidence = build_runtime_owner_evidence(harness.path(), true)
            .expect("owner evidence should persist");
        let sessions = read_sessions(&sessions_path);
        assert!(sessions
            .iter()
            .any(|session| session["session_id"] == "recent-stale"));
        assert!(!sessions
            .iter()
            .any(|session| session["session_id"] == "old-stale"));
        assert!(evidence["stale_sessions"]
            .as_array()
            .expect("stale evidence")
            .iter()
            .any(|session| session["session_id"] == "old-stale"));

        restore_session_env(saved);
    }

    #[test]
    fn session_liveness_reports_stale_owner_from_evidence() {
        let _guard = env_lock().lock().expect("env lock should be available");
        let saved = saved_session_env();
        clear_session_env();
        unsafe {
            std::env::set_var("VIDA_ORCHESTRATOR_SESSION_ID", "current-session");
        }

        let harness = TempStateHarness::new().expect("temp state should initialize");
        let first = build_runtime_owner_evidence(harness.path(), true)
            .expect("owner evidence should persist");
        let session_store_path = first["session_store_path"]
            .as_str()
            .expect("session store path");
        let now = now_epoch_seconds();
        let payload = serde_json::json!({
            "schema_version": "runtime-owner-evidence-v1",
            "sessions": [
                first["current_session"].clone(),
                {
                    "session_id": "stale-owner",
                    "state": "stale",
                    "process_id": 12345,
                    "last_heartbeat_epoch_seconds": now.saturating_sub(super::SESSION_TTL_SECONDS + 1),
                }
            ],
        });
        std::fs::write(
            session_store_path,
            serde_json::to_string_pretty(&payload).expect("serialize sessions"),
        )
        .expect("write sessions");

        assert_eq!(
            super::orchestrator_session_liveness(harness.path(), "stale-owner").expect("liveness"),
            OrchestratorSessionLiveness::Stale
        );

        restore_session_env(saved);
    }
}
