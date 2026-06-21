use clap::Parser;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(name = "vida-pi-agent")]
#[command(about = "VIDA adapter for one-shot Pi RPC dispatch")]
pub struct Cli {
    #[arg(long = "mode", default_value = "rpc")]
    pub mode: String,

    #[arg(long = "model")]
    pub model: Option<String>,

    #[arg(long = "thinking-level")]
    pub thinking_level: Option<String>,

    #[arg(long = "workdir")]
    pub workdir: Option<PathBuf>,

    #[arg(long = "pi-command", default_value = "pi")]
    pub pi_command: String,

    #[arg(long = "timeout-seconds", default_value_t = 300)]
    pub timeout_seconds: u64,

    #[arg(long = "scope-guard-mode", default_value = "auto")]
    pub scope_guard_mode: String,

    #[arg(long = "owned-path")]
    pub owned_paths: Vec<String>,

    #[arg(long = "no-session")]
    pub no_session: bool,

    #[arg(long = "no-context-files")]
    pub no_context_files: bool,

    #[arg(long = "no-skills")]
    pub no_skills: bool,

    #[arg(long = "no-extensions")]
    pub no_extensions: bool,

    #[arg(long = "no-prompt-templates")]
    pub no_prompt_templates: bool,

    #[arg(long = "no-tools")]
    pub no_tools: bool,

    #[arg(long = "capabilities-json")]
    pub capabilities_json: bool,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub prompt: Vec<String>,
}

#[derive(Debug)]
pub struct AdapterOutput {
    pub exit_code: i32,
    pub payload: Value,
}

#[derive(Debug)]
struct PiRunResult {
    result: String,
}

pub fn run_cli_with_stdin(cli: Cli, mut stdin: impl Read) -> AdapterOutput {
    if cli.capabilities_json {
        return AdapterOutput {
            exit_code: 0,
            payload: capabilities_payload(),
        };
    }

    let scope_guard = match ScopeGuardConfig::from_cli_and_env(&cli) {
        Ok(scope_guard) => scope_guard,
        Err(error) => {
            return AdapterOutput {
                exit_code: 1,
                payload: error_payload_with_scope_guard(&error.message, Some(error.report)),
            };
        }
    };

    if let Err(error) = scope_guard.preflight_write_mode() {
        return AdapterOutput {
            exit_code: 1,
            payload: error_payload_with_scope_guard(&error.message, Some(error.report)),
        };
    }

    match run_adapter(&cli, &scope_guard, &mut stdin) {
        Ok(run_result) => {
            let provider_result_json = parse_provider_result_json(&run_result.result);
            let touched_paths = provider_result_json
                .as_ref()
                .map(extract_reported_touched_paths)
                .unwrap_or_default();
            match scope_guard.validate_touched_paths(&touched_paths) {
                Ok(scope_report) => AdapterOutput {
                    exit_code: 0,
                    payload: success_payload(run_result.result, provider_result_json, scope_report),
                },
                Err(error) => AdapterOutput {
                    exit_code: 1,
                    payload: error_payload_with_scope_guard(&error.message, Some(error.report)),
                },
            }
        }
        Err(error) => AdapterOutput {
            exit_code: 1,
            payload: error_payload(&error),
        },
    }
}

fn run_adapter(
    cli: &Cli,
    scope_guard: &ScopeGuardConfig,
    stdin: &mut impl Read,
) -> Result<PiRunResult, String> {
    if cli.mode != "rpc" {
        return Err(format!(
            "Unsupported Pi adapter mode `{}`; only `rpc` is supported",
            cli.mode
        ));
    }
    if cli.timeout_seconds == 0 {
        return Err("timeout-seconds must be greater than zero".to_string());
    }

    let prompt = resolve_prompt(&cli.prompt, stdin)?;
    if prompt.trim().is_empty() {
        return Err("Pi dispatch prompt must not be empty".to_string());
    }

    let prewrite_guard = PrewriteGuardActivation::prepare(cli, scope_guard)?;
    let mut child = spawn_pi_rpc(cli, prewrite_guard.as_ref())?;
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to open Pi RPC stdin".to_string())?;
    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to open Pi RPC stdout".to_string())?;

    let (line_tx, line_rx) = mpsc::channel::<Result<String, String>>();
    std::thread::spawn(move || {
        let reader = BufReader::new(child_stdout);
        for line in reader.lines() {
            match line {
                Ok(value) => {
                    if line_tx.send(Ok(value)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = line_tx.send(Err(format!("Failed reading Pi RPC stdout: {error}")));
                    break;
                }
            }
        }
    });

    write_rpc_commands(&mut child_stdin, cli, &prompt)?;
    wait_for_agent_end(
        &mut child,
        line_rx,
        Duration::from_secs(cli.timeout_seconds),
    )
    .map(|result| PiRunResult { result })
}

fn resolve_prompt(prompt_args: &[String], stdin: &mut impl Read) -> Result<String, String> {
    if !prompt_args.is_empty() {
        return Ok(prompt_args.join(" "));
    }
    let mut prompt = String::new();
    stdin
        .read_to_string(&mut prompt)
        .map_err(|error| format!("Failed reading prompt from stdin: {error}"))?;
    Ok(prompt)
}

fn spawn_pi_rpc(
    cli: &Cli,
    prewrite_guard: Option<&PrewriteGuardActivation>,
) -> Result<Child, String> {
    let mut command = Command::new(&cli.pi_command);
    command.arg("--mode").arg("rpc");
    if let Some(prewrite_guard) = prewrite_guard {
        command
            .arg("--extension")
            .arg(&prewrite_guard.extension_path);
    }
    if cli.no_session {
        command.arg("--no-session");
    }
    if cli.no_context_files {
        command.arg("--no-context-files");
    }
    if cli.no_skills {
        command.arg("--no-skills");
    }
    if cli.no_extensions {
        command.arg("--no-extensions");
    }
    if cli.no_prompt_templates {
        command.arg("--no-prompt-templates");
    }
    if cli.no_tools {
        command.arg("--no-tools");
    }
    if let Some(workdir) = &cli.workdir {
        command.current_dir(workdir);
    }
    if let Some(prewrite_guard) = prewrite_guard {
        command.env("VIDA_PI_AGENT_SCOPE_GUARD_MODE", "guarded-write");
        command.env("VIDA_PI_AGENT_PROJECT_ROOT", &prewrite_guard.project_root);
        command.env(
            "VIDA_PI_AGENT_OWNED_PATHS_JSON",
            &prewrite_guard.owned_paths_json,
        );
        command.env("VIDA_PI_AGENT_PREWRITE_GUARD_ACTIVE", "true");
        command.env("VIDA_PI_AGENT_PREWRITE_GUARD_VERSION", "1");
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
        .spawn()
        .map_err(|error| format!("Failed to spawn Pi command `{}`: {error}", cli.pi_command))
}

fn write_rpc_commands(mut child_stdin: impl Write, cli: &Cli, prompt: &str) -> Result<(), String> {
    write_json_line(
        &mut child_stdin,
        json!({"id":"vida-get-state","type":"get_state"}),
    )?;
    write_json_line(
        &mut child_stdin,
        json!({"id":"vida-get-models","type":"get_available_models"}),
    )?;
    if let Some(model_ref) = cli.model.as_deref() {
        let (provider, model_id) = split_model_ref(model_ref)?;
        write_json_line(
            &mut child_stdin,
            json!({"id":"vida-set-model","type":"set_model","provider":provider,"modelId":model_id}),
        )?;
    }
    if let Some(thinking_level) = cli.thinking_level.as_deref() {
        write_json_line(
            &mut child_stdin,
            json!({"id":"vida-set-thinking","type":"set_thinking_level","level":thinking_level}),
        )?;
    }
    write_json_line(
        &mut child_stdin,
        json!({"id":"vida-prompt","type":"prompt","message":prompt}),
    )?;
    child_stdin
        .flush()
        .map_err(|error| format!("Failed flushing Pi RPC stdin: {error}"))
}

fn write_json_line(writer: &mut impl Write, value: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, &value)
        .map_err(|error| format!("Failed encoding Pi RPC command: {error}"))?;
    writer
        .write_all(b"\n")
        .map_err(|error| format!("Failed writing Pi RPC command: {error}"))
}

fn split_model_ref(model_ref: &str) -> Result<(&str, &str), String> {
    let Some((provider, model_id)) = model_ref.split_once('/') else {
        return Err(format!(
            "Pi model `{model_ref}` must use provider/model form"
        ));
    };
    if provider.trim().is_empty() || model_id.trim().is_empty() {
        return Err(format!(
            "Pi model `{model_ref}` must include both provider and model id"
        ));
    }
    Ok((provider, model_id))
}

fn wait_for_agent_end(
    child: &mut Child,
    line_rx: mpsc::Receiver<Result<String, String>>,
    timeout: Duration,
) -> Result<String, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let now = Instant::now();
        if now >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Timed out waiting for Pi agent_end event".to_string());
        }

        let wait_for = (deadline - now).min(Duration::from_millis(100));
        match line_rx.recv_timeout(wait_for) {
            Ok(Ok(line)) => {
                let trimmed = line.trim_end_matches('\r').trim();
                if trimmed.is_empty() {
                    continue;
                }
                let value: Value = serde_json::from_str(trimmed)
                    .map_err(|error| format!("Invalid Pi RPC JSONL record: {error}"))?;
                validate_response_record(&value)?;
                if is_agent_end_event(&value) {
                    let result = extract_agent_end_text(&value)?;
                    wait_or_kill_after_terminal(child);
                    return Ok(result);
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(status) = child
                    .try_wait()
                    .map_err(|error| format!("Failed to inspect Pi process state: {error}"))?
                {
                    return Err(format!(
                        "Pi process exited before agent_end with status {status}"
                    ));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(status) = child
                    .try_wait()
                    .map_err(|error| format!("Failed to inspect Pi process state: {error}"))?
                {
                    return Err(format!(
                        "Pi process exited before agent_end with status {status}"
                    ));
                }
                return Err("Pi RPC stdout closed before agent_end".to_string());
            }
        }
    }
}

fn validate_response_record(value: &Value) -> Result<(), String> {
    if value["type"].as_str() == Some("response") && value["success"].as_bool() == Some(false) {
        let command = value["command"].as_str().unwrap_or("unknown");
        let message = value["error"]
            .as_str()
            .or_else(|| value["message"].as_str())
            .or_else(|| value.pointer("/data/error").and_then(Value::as_str))
            .unwrap_or("Pi RPC command failed");
        return Err(format!("Pi RPC `{command}` failed: {message}"));
    }
    Ok(())
}

fn is_agent_end_event(value: &Value) -> bool {
    value["type"].as_str() == Some("agent_end")
        || value["event"].as_str() == Some("agent_end")
        || value["name"].as_str() == Some("agent_end")
        || value["type"].as_str() == Some("event") && value["event"].as_str() == Some("agent_end")
}

fn extract_agent_end_text(value: &Value) -> Result<String, String> {
    let messages = value
        .get("messages")
        .or_else(|| value.pointer("/data/messages"))
        .and_then(Value::as_array)
        .ok_or_else(|| "Pi agent_end event did not include messages".to_string())?;

    for message in messages.iter().rev() {
        if let Some(text) = message_text(message) {
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }
    Err("Pi agent_end messages did not include final text".to_string())
}

fn message_text(value: &Value) -> Option<String> {
    for key in ["text", "content", "message", "result"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            return Some(text.to_string());
        }
    }
    if let Some(parts) = value.get("content").and_then(Value::as_array) {
        let mut rendered = String::new();
        for part in parts {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                if !rendered.is_empty() {
                    rendered.push('\n');
                }
                rendered.push_str(text);
            }
        }
        if !rendered.is_empty() {
            return Some(rendered);
        }
    }
    None
}

fn wait_or_kill_after_terminal(child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

const VIDA_PI_PREWRITE_GUARD_EXTENSION: &str = r#"
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { existsSync, realpathSync } from "node:fs";
import { isAbsolute, join, relative } from "node:path";

function normalizePathForMessage(path: string): string {
  return path.trim().replace(/\\\\/g, "/");
}

function isInside(root: string, candidate: string): boolean {
  const rel = relative(root, candidate);
  return rel === "" || (!!rel && !rel.startsWith("..") && !isAbsolute(rel));
}

function cleanRelativePath(raw: unknown): string[] {
  if (typeof raw !== "string") throw new Error("tool path must be a string");
  const trimmed = raw.trim();
  if (!trimmed) throw new Error("tool path must not be empty");
  if (isAbsolute(trimmed)) throw new Error(`absolute path is not allowed: ${normalizePathForMessage(trimmed)}`);
  const parts = trimmed.split(/[\\/]+/).filter((part) => part.length > 0);
  if (parts.length === 0) throw new Error(`path has no normal relative component: ${normalizePathForMessage(trimmed)}`);
  for (const part of parts) {
    if (part === "." || part === "..") {
      throw new Error(`path contains current-directory or parent-directory escape: ${normalizePathForMessage(trimmed)}`);
    }
  }
  return parts;
}

function canonicalizeUnderRoot(root: string, raw: unknown): string {
  const parts = cleanRelativePath(raw);
  const candidate = join(root, ...parts);
  let existing = candidate;
  const missing: string[] = [];
  while (!existsSync(existing)) {
    const next = existing.replace(/[\\/]+$/, "").replace(/[\\/][^\\/]*$/, "");
    if (!next || next === existing) break;
    missing.push(existing.slice(next.length).replace(/^[\\/]+/, ""));
    existing = next;
    if (existing === root) break;
  }
  const existingReal = realpathSync.native(existing);
  if (!isInside(root, existingReal)) {
    throw new Error(`canonical ancestor escapes project root: ${normalizePathForMessage(String(raw))}`);
  }
  let canonical = existingReal;
  for (const part of missing.reverse()) canonical = join(canonical, part);
  if (!isInside(root, canonical)) {
    throw new Error(`canonical path escapes project root: ${normalizePathForMessage(String(raw))}`);
  }
  return canonical;
}

function parseOwnedPaths(): string[] {
  const raw = process.env.VIDA_PI_AGENT_OWNED_PATHS_JSON || "[]";
  const parsed = JSON.parse(raw);
  if (!Array.isArray(parsed)) throw new Error("VIDA_PI_AGENT_OWNED_PATHS_JSON must be a JSON array");
  return parsed.map((value) => {
    if (typeof value !== "string") throw new Error("owned paths must be strings");
    return value;
  });
}

export default function (pi: ExtensionAPI) {
  const mode = (process.env.VIDA_PI_AGENT_SCOPE_GUARD_MODE || "").trim().toLowerCase();
  if (mode !== "guarded-write") return;

  const rootRaw = process.env.VIDA_PI_AGENT_PROJECT_ROOT;
  if (!rootRaw) throw new Error("VIDA Pi pre-write guard missing VIDA_PI_AGENT_PROJECT_ROOT");
  const projectRoot = realpathSync.native(rootRaw);
  const ownedPaths = parseOwnedPaths().map((path) => canonicalizeUnderRoot(projectRoot, path));
  if (ownedPaths.length === 0) throw new Error("VIDA Pi pre-write guard requires at least one owned path");

  function assertOwned(raw: unknown): string | undefined {
    const canonical = canonicalizeUnderRoot(projectRoot, raw);
    if (!ownedPaths.some((owned) => isInside(owned, canonical))) {
      return `VIDA Pi write-scope guard blocked path outside owned paths: ${normalizePathForMessage(String(raw))}`;
    }
    return undefined;
  }

  function mutatingToolReason(toolName: string, input: Record<string, unknown>): string | undefined {
    if (toolName === "write" || toolName === "edit") {
      return assertOwned(input.path);
    }
    if (toolName === "bash") {
      return "VIDA Pi write-scope guard blocks bash in guarded-write mode to prevent shell write bypass";
    }
    if (/write|edit|patch|create|delete|remove|rename|move/i.test(toolName)) {
      return `VIDA Pi write-scope guard blocked unknown mutating tool: ${toolName}`;
    }
    return undefined;
  }

  pi.on("tool_call", async (event) => {
    const input = (event.input || {}) as Record<string, unknown>;
    const reason = mutatingToolReason(event.toolName, input);
    if (reason) return { block: true, reason };
    return undefined;
  });

  pi.on("user_bash", () => {
    return { result: { output: "VIDA Pi write-scope guard blocks user bash in guarded-write mode", exitCode: 126, cancelled: false, truncated: false } };
  });
}
"#;

#[derive(Debug)]
struct PrewriteGuardActivation {
    extension_path: PathBuf,
    _temp_dir: PathBuf,
    project_root: PathBuf,
    owned_paths_json: String,
}

impl PrewriteGuardActivation {
    fn prepare(cli: &Cli, scope_guard: &ScopeGuardConfig) -> Result<Option<Self>, String> {
        if scope_guard.mode != ScopeGuardMode::GuardedWrite {
            return Ok(None);
        }
        if cli.no_extensions {
            return Err("Pi guarded-write mode requires the VIDA pre-write guard extension; --no-extensions is forbidden".to_string());
        }
        let unique = format!(
            "vida-pi-agent-guard-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| format!("Failed to create guard timestamp: {error}"))?
                .as_nanos()
        );
        let temp_dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&temp_dir).map_err(|error| {
            format!(
                "Failed to create Pi pre-write guard directory `{}`: {error}",
                temp_dir.display()
            )
        })?;
        let extension_path = temp_dir.join("vida-owned-write-scope-guard.ts");
        fs::write(&extension_path, VIDA_PI_PREWRITE_GUARD_EXTENSION).map_err(|error| {
            format!(
                "Failed to write Pi pre-write guard extension `{}`: {error}",
                extension_path.display()
            )
        })?;
        let owned_paths_json = serde_json::to_string(
            &scope_guard
                .owned_paths
                .iter()
                .map(|path| path.raw.clone())
                .collect::<Vec<_>>(),
        )
        .map_err(|error| format!("Failed to encode Pi owned paths for pre-write guard: {error}"))?;
        Ok(Some(Self {
            extension_path,
            _temp_dir: temp_dir,
            project_root: scope_guard.project_root.clone(),
            owned_paths_json,
        }))
    }
}

impl Drop for PrewriteGuardActivation {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self._temp_dir);
    }
}

fn capabilities_payload() -> Value {
    json!({
        "type": "capabilities",
        "adapter": "vida-pi-agent",
        "scope_guard": {
            "pre_write_enforcement": true,
            "guarded_write_mode": true,
            "explicit_extension_arg": true,
            "blocks_tools": ["write", "edit", "bash", "user_bash", "unknown_mutating_tools"],
            "post_execution_validation": true,
            "version": 1
        }
    })
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ScopeGuardMode {
    Off,
    Auto,
    ValidateOnly,
    GuardedWrite,
}

impl ScopeGuardMode {
    fn from_raw(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Ok(Self::Auto),
            "off" | "disabled" | "none" => Ok(Self::Off),
            "validate" | "validate-only" | "validation-only" | "validation_only" => {
                Ok(Self::ValidateOnly)
            }
            "guarded-write"
            | "guarded_write"
            | "write"
            | "guard_required"
            | "guard_required_owned_paths"
            | "guard-required-owned-paths" => Ok(Self::GuardedWrite),
            other => Err(format!(
                "Unsupported Pi scope guard mode `{other}`; expected auto, off, validate-only, or guarded-write"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::ValidateOnly => "validate-only",
            Self::GuardedWrite => "guarded-write",
        }
    }
}

#[derive(Debug, Clone)]
struct OwnedScopePath {
    raw: String,
    canonical: PathBuf,
}

#[derive(Debug)]
struct ScopeGuardConfig {
    mode: ScopeGuardMode,
    project_root: PathBuf,
    owned_paths: Vec<OwnedScopePath>,
    sources: Vec<String>,
}

#[derive(Debug)]
struct ScopeGuardFailure {
    message: String,
    report: Value,
}

impl ScopeGuardConfig {
    fn from_cli_and_env(cli: &Cli) -> Result<Self, ScopeGuardFailure> {
        let raw_mode = std::env::var("VIDA_PI_AGENT_SCOPE_GUARD_MODE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| cli.scope_guard_mode.clone());
        let mode = ScopeGuardMode::from_raw(&raw_mode).map_err(|message| ScopeGuardFailure {
            report: json!({
                "mode": raw_mode,
                "enforcement": "validation_only_fail_closed_for_guarded_write_mode",
                "status": "configuration_error",
                "valid": false,
                "violations": [message.clone()]
            }),
            message,
        })?;
        let project_root = resolve_project_root(cli).map_err(|message| ScopeGuardFailure {
            report: json!({
                "mode": mode.as_str(),
                "enforcement": "validation_only_fail_closed_for_guarded_write_mode",
                "status": "configuration_error",
                "valid": false,
                "violations": [message.clone()]
            }),
            message,
        })?;
        let mut raw_owned_paths = Vec::new();
        let mut sources = Vec::new();
        append_unique_raw_paths(&mut raw_owned_paths, &cli.owned_paths);
        if !cli.owned_paths.is_empty() {
            sources.push("cli:--owned-path".to_string());
        }
        if let Some(paths) = env_owned_paths() {
            append_unique_raw_paths(&mut raw_owned_paths, &paths);
            if !paths.is_empty() {
                sources.push("env:VIDA_PI_AGENT_OWNED_PATHS".to_string());
            }
        }
        if let Some((paths, packet_path)) = dispatch_packet_owned_paths() {
            append_unique_raw_paths(&mut raw_owned_paths, &paths);
            if !paths.is_empty() {
                sources.push(format!("packet:{}", packet_path.display()));
            }
        }

        let mut owned_paths = Vec::new();
        for raw_path in &raw_owned_paths {
            match canonicalize_scope_relative_path(&project_root, raw_path) {
                Ok(canonical) => owned_paths.push(OwnedScopePath {
                    raw: normalize_slashes(&raw_path),
                    canonical,
                }),
                Err(message) => {
                    return Err(ScopeGuardFailure {
                        report: json!({
                            "mode": mode.as_str(),
                            "enforcement": "validation_only_fail_closed_for_guarded_write_mode",
                            "status": "owned_path_invalid",
                            "valid": false,
                            "project_root": project_root.display().to_string(),
                            "owned_paths": raw_owned_paths.clone(),
                            "violations": [message.clone()]
                        }),
                        message,
                    });
                }
            }
        }

        Ok(Self {
            mode,
            project_root,
            owned_paths,
            sources,
        })
    }

    fn preflight_write_mode(&self) -> Result<(), ScopeGuardFailure> {
        if self.mode == ScopeGuardMode::GuardedWrite && self.owned_paths.is_empty() {
            let message = "Pi guarded write mode requires at least one owned path; refusing to launch validation-only adapter without bounded write scope".to_string();
            return Err(ScopeGuardFailure {
                report: self.report("missing_owned_paths", false, &[], &[message.clone()]),
                message,
            });
        }
        Ok(())
    }

    fn validate_touched_paths(
        &self,
        touched_paths: &[String],
    ) -> Result<Option<Value>, ScopeGuardFailure> {
        if self.mode == ScopeGuardMode::Off {
            return Ok(None);
        }
        if touched_paths.is_empty() {
            let status = if self.owned_paths.is_empty() {
                "no_touched_paths_reported"
            } else {
                "no_touched_paths_reported_with_owned_scope"
            };
            return Ok(Some(self.report(status, true, &[], &[])));
        }
        if self.owned_paths.is_empty() {
            let message = "Provider reported touched paths but no owned paths were available for Pi scope validation".to_string();
            let violations = if self.mode == ScopeGuardMode::GuardedWrite {
                vec![message.clone()]
            } else {
                Vec::new()
            };
            let report = self.report(
                "validation_unavailable_no_owned_paths",
                self.mode != ScopeGuardMode::GuardedWrite,
                touched_paths,
                &violations,
            );
            if self.mode == ScopeGuardMode::GuardedWrite {
                return Err(ScopeGuardFailure { message, report });
            }
            return Ok(Some(report));
        }

        let mut violations = Vec::new();
        for touched_path in touched_paths {
            match canonicalize_scope_relative_path(&self.project_root, touched_path) {
                Ok(canonical) => {
                    if !self
                        .owned_paths
                        .iter()
                        .any(|owned_path| canonical.starts_with(&owned_path.canonical))
                    {
                        violations.push(format!(
                            "touched path `{}` is outside owned paths",
                            normalize_slashes(touched_path)
                        ));
                    }
                }
                Err(message) => violations.push(message),
            }
        }

        if violations.is_empty() {
            return Ok(Some(self.report("validated", true, touched_paths, &[])));
        }
        let message = format!("Pi touched-path scope violation: {}", violations.join("; "));
        Err(ScopeGuardFailure {
            report: self.report("violation", false, touched_paths, &violations),
            message,
        })
    }

    fn report(
        &self,
        status: &str,
        valid: bool,
        touched_paths: &[String],
        violations: &[String],
    ) -> Value {
        let pre_write_enforcement = self.mode == ScopeGuardMode::GuardedWrite;
        json!({
            "mode": self.mode.as_str(),
            "enforcement": if pre_write_enforcement { "pre_write_tool_call_guard_and_post_execution_validation" } else { "validation_only_post_execution" },
            "pre_write_enforcement": pre_write_enforcement,
            "pre_write_guard": {
                "active": pre_write_enforcement,
                "mechanism": if pre_write_enforcement { "adapter_owned_explicit_pi_extension" } else { "none" },
                "blocks_bash": pre_write_enforcement,
                "blocks_unknown_mutating_tools": pre_write_enforcement
            },
            "write_profiles_runtime_admission": if pre_write_enforcement { "guarded_by_adapter_pre_write_extension" } else { "fail_closed_until_pre_write_guard" },
            "status": status,
            "valid": valid,
            "project_root": self.project_root.display().to_string(),
            "owned_paths": self.owned_paths.iter().map(|path| path.raw.clone()).collect::<Vec<_>>(),
            "owned_path_sources": self.sources,
            "touched_paths": touched_paths.iter().map(|path| normalize_slashes(path)).collect::<Vec<_>>(),
            "violations": violations,
        })
    }
}

fn resolve_project_root(cli: &Cli) -> Result<PathBuf, String> {
    let root = match cli.workdir.as_ref() {
        Some(workdir) => workdir.clone(),
        None => std::env::current_dir().map_err(|error| format!("Failed to read cwd: {error}"))?,
    };
    fs::canonicalize(&root).map_err(|error| {
        format!(
            "Failed to canonicalize Pi project/workdir root `{}`: {error}",
            root.display()
        )
    })
}

fn env_owned_paths() -> Option<Vec<String>> {
    let raw = std::env::var("VIDA_PI_AGENT_OWNED_PATHS").ok()?;
    Some(parse_owned_paths_value(&raw).unwrap_or_else(|| {
        raw.split(';')
            .flat_map(|segment| segment.split(','))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()
    }))
}

fn dispatch_packet_owned_paths() -> Option<(Vec<String>, PathBuf)> {
    let packet_path = std::env::var("VIDA_DISPATCH_PACKET_PATH")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.exists())?;
    let bytes = fs::read(&packet_path).ok()?;
    let value = serde_json::from_slice::<Value>(&bytes).ok()?;
    let mut paths = Vec::new();
    for pointer in [
        "/owned_paths",
        "/delivery_task_packet/owned_paths",
        "/execution_block_packet/owned_paths",
        "/runtime_contract/owned_paths",
        "/bounded_write_scope/owned_paths",
    ] {
        if let Some(value) = value.pointer(pointer) {
            append_unique_raw_paths(&mut paths, &extract_string_array(value));
        }
    }
    Some((paths, packet_path))
}

fn parse_owned_paths_value(raw: &str) -> Option<Vec<String>> {
    let value = serde_json::from_str::<Value>(raw).ok()?;
    Some(extract_string_array(&value))
}

fn extract_string_array(value: &Value) -> Vec<String> {
    match value {
        Value::String(path) => vec![path.trim().to_string()],
        Value::Array(paths) => paths
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn append_unique_raw_paths(target: &mut Vec<String>, paths: &[String]) {
    for path in paths {
        let path = path.trim();
        if !path.is_empty() && !target.iter().any(|existing| existing == path) {
            target.push(path.to_string());
        }
    }
}

fn canonicalize_scope_relative_path(
    project_root: &Path,
    raw_path: &str,
) -> Result<PathBuf, String> {
    let relative_path = clean_relative_scope_path(raw_path)?;
    canonicalize_under_root(project_root, &relative_path).map_err(|message| {
        format!(
            "path `{}` is outside the Pi owned write scope: {message}",
            normalize_slashes(raw_path)
        )
    })
}

fn clean_relative_scope_path(raw_path: &str) -> Result<PathBuf, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("empty paths are not valid Pi owned/touched scope entries".to_string());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(format!(
            "absolute path `{}` is not allowed in Pi owned/touched scope entries",
            normalize_slashes(trimmed)
        ));
    }
    let mut cleaned = PathBuf::new();
    let mut saw_component = false;
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                if value.is_empty() {
                    return Err(format!(
                        "path `{}` contains an empty component",
                        normalize_slashes(trimmed)
                    ));
                }
                cleaned.push(value);
                saw_component = true;
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(format!(
                    "path `{}` contains an absolute, current-directory, parent-directory, or prefix escape component",
                    normalize_slashes(trimmed)
                ));
            }
        }
    }
    if !saw_component {
        return Err(format!(
            "path `{}` did not contain a normal relative component",
            normalize_slashes(trimmed)
        ));
    }
    Ok(cleaned)
}

fn canonicalize_under_root(project_root: &Path, relative_path: &Path) -> Result<PathBuf, String> {
    let root = fs::canonicalize(project_root).map_err(|error| {
        format!(
            "project root `{}` could not be canonicalized: {error}",
            project_root.display()
        )
    })?;
    let candidate = root.join(relative_path);
    let mut existing = candidate.clone();
    let mut missing = Vec::<OsString>::new();
    while !existing.exists() {
        if existing == root {
            break;
        }
        if let Some(name) = existing.file_name() {
            missing.push(name.to_os_string());
        }
        if !existing.pop() {
            break;
        }
    }
    let existing_canonical = fs::canonicalize(&existing).map_err(|error| {
        format!(
            "existing scope ancestor `{}` could not be canonicalized: {error}",
            existing.display()
        )
    })?;
    if !existing_canonical.starts_with(&root) {
        return Err(format!(
            "canonical ancestor `{}` escapes project root `{}`",
            existing_canonical.display(),
            root.display()
        ));
    }
    let mut canonical = existing_canonical;
    for component in missing.iter().rev() {
        canonical.push(component);
    }
    if !canonical.starts_with(&root) {
        return Err(format!(
            "canonical path `{}` escapes project root `{}`",
            canonical.display(),
            root.display()
        ));
    }
    Ok(canonical)
}

fn parse_provider_result_json(result: &str) -> Option<Value> {
    serde_json::from_str(result.trim()).ok()
}

fn extract_reported_touched_paths(value: &Value) -> Vec<String> {
    let mut paths = BTreeSet::new();
    collect_reported_touched_paths(value, &mut paths);
    paths.into_iter().collect()
}

fn collect_reported_touched_paths(value: &Value, paths: &mut BTreeSet<String>) {
    match value {
        Value::Object(entries) => {
            for key in ["touched_paths", "changed_files"] {
                if let Some(raw_paths) = entries.get(key) {
                    collect_path_values(raw_paths, paths);
                }
            }
            for value in entries.values() {
                collect_reported_touched_paths(value, paths);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_reported_touched_paths(value, paths);
            }
        }
        _ => {}
    }
}

fn collect_path_values(value: &Value, paths: &mut BTreeSet<String>) {
    match value {
        Value::String(path) => {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                paths.insert(trimmed.to_string());
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_path_values(value, paths);
            }
        }
        Value::Object(entries) => {
            for key in ["path", "file", "filename"] {
                if let Some(path) = entries.get(key).and_then(Value::as_str) {
                    let trimmed = path.trim();
                    if !trimmed.is_empty() {
                        paths.insert(trimmed.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

fn normalize_slashes(path: &str) -> String {
    path.trim().replace('\\', "/")
}

fn success_payload(
    result: String,
    provider_result_json: Option<Value>,
    scope_guard: Option<Value>,
) -> Value {
    let mut raw_provider = json!({
        "provider": "pi",
        "mode": "rpc",
        "terminal_event": "agent_end"
    });
    if let Some(provider_result_json) = provider_result_json {
        raw_provider["provider_result_json"] = provider_result_json;
    }
    let mut payload = json!({
        "type": "result",
        "subtype": "success",
        "is_error": false,
        "result": result,
        "raw_provider": raw_provider
    });
    if let Some(scope_guard) = scope_guard {
        payload["scope_guard"] = scope_guard;
    }
    payload
}

fn error_payload(message: &str) -> Value {
    error_payload_with_scope_guard(message, None)
}

fn error_payload_with_scope_guard(message: &str, scope_guard: Option<Value>) -> Value {
    let mut payload = json!({
        "type": "result",
        "subtype": "error_during_execution",
        "is_error": true,
        "error": {
            "message": message
        },
        "raw_provider": {
            "provider": "pi",
            "mode": "rpc"
        }
    });
    if let Some(scope_guard) = scope_guard {
        payload["scope_guard"] = scope_guard;
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_model_ref_requires_provider_and_model_id() {
        assert_eq!(
            split_model_ref("openai-codex/gpt-5.5").unwrap(),
            ("openai-codex", "gpt-5.5")
        );
        assert!(split_model_ref("gpt-5.5").is_err());
        assert!(split_model_ref("provider/").is_err());
    }

    #[test]
    fn extracts_text_from_agent_end_messages() {
        let event = json!({
            "type": "event",
            "event": "agent_end",
            "messages": [
                {"role":"user","content":"hello"},
                {"role":"assistant","content":[{"type":"text","text":"final answer"}]}
            ]
        });
        assert_eq!(extract_agent_end_text(&event).unwrap(), "final answer");
    }

    #[test]
    fn scope_guard_allows_in_scope_touched_path() {
        let root = temp_scope_root("in-scope");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "").unwrap();
        let guard = ScopeGuardConfig {
            mode: ScopeGuardMode::GuardedWrite,
            project_root: fs::canonicalize(&root).unwrap(),
            owned_paths: vec![OwnedScopePath {
                raw: "src".to_string(),
                canonical: canonicalize_scope_relative_path(&root, "src").unwrap(),
            }],
            sources: vec!["test".to_string()],
        };
        let report = guard
            .validate_touched_paths(&["src/lib.rs".to_string()])
            .unwrap()
            .unwrap();
        assert_eq!(report["status"], "validated");
        assert_eq!(report["valid"], true);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scope_guard_rejects_out_of_scope_touched_path() {
        let root = temp_scope_root("out-of-scope");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        let guard = ScopeGuardConfig {
            mode: ScopeGuardMode::GuardedWrite,
            project_root: fs::canonicalize(&root).unwrap(),
            owned_paths: vec![OwnedScopePath {
                raw: "src".to_string(),
                canonical: canonicalize_scope_relative_path(&root, "src").unwrap(),
            }],
            sources: vec!["test".to_string()],
        };
        let error = guard
            .validate_touched_paths(&["docs/spec.md".to_string()])
            .unwrap_err();
        assert_eq!(error.report["status"], "violation");
        assert!(error.message.contains("outside owned paths"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scope_guard_rejects_parent_escape() {
        let root = temp_scope_root("parent-escape");
        fs::create_dir_all(root.join("src")).unwrap();
        let guard = ScopeGuardConfig {
            mode: ScopeGuardMode::GuardedWrite,
            project_root: fs::canonicalize(&root).unwrap(),
            owned_paths: vec![OwnedScopePath {
                raw: "src".to_string(),
                canonical: canonicalize_scope_relative_path(&root, "src").unwrap(),
            }],
            sources: vec!["test".to_string()],
        };
        let error = guard
            .validate_touched_paths(&["../outside.txt".to_string()])
            .unwrap_err();
        assert_eq!(error.report["status"], "violation");
        assert!(error.message.contains("parent-directory"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prewrite_guard_extension_blocks_write_edit_bash_and_unknown_mutators() {
        assert!(VIDA_PI_PREWRITE_GUARD_EXTENSION.contains("toolName === \"write\""));
        assert!(VIDA_PI_PREWRITE_GUARD_EXTENSION.contains("toolName === \"edit\""));
        assert!(VIDA_PI_PREWRITE_GUARD_EXTENSION.contains("toolName === \"bash\""));
        assert!(VIDA_PI_PREWRITE_GUARD_EXTENSION.contains("pi.on(\"user_bash\""));
        assert!(
            VIDA_PI_PREWRITE_GUARD_EXTENSION
                .contains("write|edit|patch|create|delete|remove|rename|move")
        );
        assert!(VIDA_PI_PREWRITE_GUARD_EXTENSION.contains("isInside(owned, canonical)"));
    }

    #[test]
    fn guarded_write_rejects_empty_owned_paths() {
        let root = temp_scope_root("empty-owned");
        let guard = ScopeGuardConfig {
            mode: ScopeGuardMode::GuardedWrite,
            project_root: fs::canonicalize(&root).unwrap(),
            owned_paths: Vec::new(),
            sources: Vec::new(),
        };
        let error = guard.preflight_write_mode().unwrap_err();
        assert_eq!(error.report["status"], "missing_owned_paths");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn scope_guard_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        let root = temp_scope_root("symlink-escape");
        let outside = temp_scope_root("symlink-escape-outside");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(outside.join("pwned.txt"), "").unwrap();
        symlink(&outside, root.join("src/link")).unwrap();
        let guard = ScopeGuardConfig {
            mode: ScopeGuardMode::GuardedWrite,
            project_root: fs::canonicalize(&root).unwrap(),
            owned_paths: vec![OwnedScopePath {
                raw: "src".to_string(),
                canonical: canonicalize_scope_relative_path(&root, "src").unwrap(),
            }],
            sources: vec!["test".to_string()],
        };
        let error = guard
            .validate_touched_paths(&["src/link/pwned.txt".to_string()])
            .unwrap_err();
        assert_eq!(error.report["status"], "violation");
        assert!(
            error.message.contains("escapes project root")
                || error.message.contains("outside owned paths")
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(windows)]
    #[test]
    fn scope_guard_rejects_symlink_escape() {
        use std::os::windows::fs::symlink_dir;
        let root = temp_scope_root("symlink-escape");
        let outside = temp_scope_root("symlink-escape-outside");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(outside.join("pwned.txt"), "").unwrap();
        if symlink_dir(&outside, root.join("src/link")).is_err() {
            let _ = fs::remove_dir_all(root);
            let _ = fs::remove_dir_all(outside);
            return;
        }
        let guard = ScopeGuardConfig {
            mode: ScopeGuardMode::GuardedWrite,
            project_root: fs::canonicalize(&root).unwrap(),
            owned_paths: vec![OwnedScopePath {
                raw: "src".to_string(),
                canonical: canonicalize_scope_relative_path(&root, "src").unwrap(),
            }],
            sources: vec!["test".to_string()],
        };
        let error = guard
            .validate_touched_paths(&["src/link/pwned.txt".to_string()])
            .unwrap_err();
        assert_eq!(error.report["status"], "violation");
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    fn temp_scope_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vida-pi-agent-scope-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
