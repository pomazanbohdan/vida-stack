use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CommandPhase {
    PreParse,
    PostParse,
    PreExecution,
    Execution,
    PostExecution,
    PreReturn,
}

impl CommandPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PreParse => "pre_parse",
            Self::PostParse => "post_parse",
            Self::PreExecution => "pre_execution",
            Self::Execution => "execution",
            Self::PostExecution => "post_execution",
            Self::PreReturn => "pre_return",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Standard,
    Json,
    Verbose,
    Quiet,
}

impl OutputMode {
    pub(crate) fn from_env_value(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "standard" | "plain" => Some(Self::Standard),
            "json" => Some(Self::Json),
            "verbose" => Some(Self::Verbose),
            "quiet" => Some(Self::Quiet),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Json => "json",
            Self::Verbose => "verbose",
            Self::Quiet => "quiet",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TimingHookConfig {
    pub(crate) enabled: bool,
    pub(crate) min_duration_threshold: Duration,
    pub(crate) latency_budget: Option<Duration>,
    pub(crate) include_phase_timing: bool,
    pub(crate) output_mode: OutputMode,
    pub(crate) print_summary: bool,
}

impl TimingHookConfig {
    pub(crate) fn from_env() -> Self {
        let enabled = env_bool("VIDA_COMMAND_TIMING_ENABLED")
            || std::env::var_os("VIDA_COMMAND_TIMING").is_some();
        let output_mode = std::env::var("VIDA_COMMAND_OUTPUT_MODE")
            .ok()
            .and_then(|raw| OutputMode::from_env_value(&raw))
            .unwrap_or(OutputMode::Standard);
        Self {
            enabled,
            min_duration_threshold: env_u64("VIDA_COMMAND_TIMING_MIN_MS")
                .map(Duration::from_millis)
                .unwrap_or(Duration::ZERO),
            latency_budget: env_u64("VIDA_COMMAND_TIMING_BUDGET_MS").map(Duration::from_millis),
            include_phase_timing: !env_false("VIDA_COMMAND_TIMING_PHASES"),
            output_mode,
            print_summary: enabled
                && (env_bool("VIDA_COMMAND_TIMING_PRINT_SUMMARY")
                    || matches!(output_mode, OutputMode::Verbose | OutputMode::Json)),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CommandTimingRecord {
    command: String,
    phases: BTreeMap<CommandPhase, Duration>,
    started_at: Instant,
    total_duration: Option<Duration>,
    exit_code: Option<i32>,
}

impl CommandTimingRecord {
    pub(crate) fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            phases: BTreeMap::new(),
            started_at: Instant::now(),
            total_duration: None,
            exit_code: None,
        }
    }

    pub(crate) fn record_phase(&mut self, phase: CommandPhase, duration: Duration) {
        self.phases.insert(phase, duration);
    }

    pub(crate) fn finalize(mut self, exit_code: ExitCode) -> Self {
        self.total_duration = Some(self.started_at.elapsed());
        self.exit_code = Some(exit_code_to_i32(exit_code));
        self
    }

    fn phase_millis(&self) -> BTreeMap<&'static str, u128> {
        self.phases
            .iter()
            .map(|(phase, duration)| (phase.as_str(), duration.as_millis()))
            .collect()
    }

    fn slowest_phase(&self) -> Option<(&'static str, u128)> {
        self.phases
            .iter()
            .max_by_key(|(_, duration)| duration.as_nanos())
            .map(|(phase, duration)| (phase.as_str(), duration.as_millis()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CargoTimingContext {
    target_dir_policy: &'static str,
    effective_cargo_target_dir: String,
    artifact_lock_wait_ms: Option<u128>,
    compile_ms: Option<u128>,
    wait_classification: &'static str,
}

impl CargoTimingContext {
    fn as_json(&self) -> serde_json::Value {
        serde_json::json!({
            "target_dir_policy": self.target_dir_policy,
            "effective_cargo_target_dir": self.effective_cargo_target_dir,
            "artifact_lock_wait_ms": self.artifact_lock_wait_ms,
            "compile_ms": self.compile_ms,
            "wait_classification": self.wait_classification,
        })
    }
}

fn cargo_timing_context_for_command(
    command: &str,
    over_budget: bool,
) -> Option<CargoTimingContext> {
    if !command_can_invoke_cargo(command) {
        return None;
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let env_target_dir = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
    Some(cargo_timing_context_from_parts(
        env_target_dir.as_deref(),
        &cwd,
        over_budget,
    ))
}

fn command_can_invoke_cargo(command: &str) -> bool {
    let normalized = command.trim().to_ascii_lowercase();
    normalized == "cargo"
        || normalized.starts_with("cargo ")
        || normalized.contains(" cargo ")
        || normalized.contains("cargo-nextest")
        || normalized.contains("nextest")
        || normalized.contains("vida-dev-gate")
}

fn cargo_timing_context_from_parts(
    env_target_dir: Option<&Path>,
    cwd: &Path,
    over_budget: bool,
) -> CargoTimingContext {
    let (target_dir_policy, effective_target_dir) = match env_target_dir {
        Some(path) => ("caller_provided", path.to_path_buf()),
        None => match shared_cargo_target_dir_for_linked_worktree(cwd) {
            Some(path) => ("repo_local_worktree_shared", path),
            None => ("repo_local_default", cwd.join(".vida").join("cargo-target")),
        },
    };
    CargoTimingContext {
        target_dir_policy,
        effective_cargo_target_dir: effective_target_dir.display().to_string(),
        artifact_lock_wait_ms: None,
        compile_ms: None,
        wait_classification: if over_budget {
            "cargo_wait_unclassified_without_cargo_phase_data"
        } else {
            "not_over_budget"
        },
    }
}

fn shared_cargo_target_dir_for_linked_worktree(cwd: &Path) -> Option<PathBuf> {
    for ancestor in cwd.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == "worktrees") {
            let vida_dir = ancestor.parent()?;
            if vida_dir.file_name().is_some_and(|name| name == ".vida") {
                let owner_root = vida_dir.parent()?;
                return Some(owner_root.join(".vida").join("cargo-target"));
            }
        }
    }
    None
}

#[derive(Debug, Default)]
pub(crate) struct CommandTimingRegistry {
    records: Mutex<Vec<CommandTimingRecord>>,
}

impl CommandTimingRegistry {
    pub(crate) fn add_record(&self, record: CommandTimingRecord) {
        if let Ok(mut records) = self.records.lock() {
            records.push(record);
        }
    }
}

pub(crate) fn global_timing_registry() -> &'static CommandTimingRegistry {
    static REGISTRY: OnceLock<CommandTimingRegistry> = OnceLock::new();
    REGISTRY.get_or_init(CommandTimingRegistry::default)
}

pub(crate) struct CommandTimingContext {
    config: TimingHookConfig,
    record: CommandTimingRecord,
}

impl CommandTimingContext {
    pub(crate) fn from_env(command: impl Into<String>) -> Self {
        Self {
            config: TimingHookConfig::from_env(),
            record: CommandTimingRecord::new(command),
        }
    }

    pub(crate) fn record_phase(&mut self, phase: CommandPhase, duration: Duration) {
        if self.config.enabled && self.config.include_phase_timing {
            self.record.record_phase(phase, duration);
        }
    }

    pub(crate) fn finalize_and_emit(self, exit_code: ExitCode) {
        let config = self.config.clone();
        let record = self.record.finalize(exit_code);
        let total = record.total_duration.unwrap_or_default();
        if !config.enabled || total < config.min_duration_threshold {
            return;
        }
        if config.print_summary {
            emit_timing_summary(&record, &config);
        }
        global_timing_registry().add_record(record);
    }
}

fn emit_timing_summary(record: &CommandTimingRecord, config: &TimingHookConfig) {
    let total_ms = record.total_duration.unwrap_or_default().as_millis();
    let budget_ms = config.latency_budget.map(|budget| budget.as_millis());
    let over_budget = budget_ms.is_some_and(|budget| total_ms >= budget);
    let slowest_phase = record.slowest_phase();
    let cargo_timing = cargo_timing_context_for_command(&record.command, over_budget);
    let next_actions = timing_next_actions(over_budget, slowest_phase, cargo_timing.as_ref());
    match config.output_mode {
        OutputMode::Json => {
            let payload = serde_json::json!({
                "vida_timing": {
                    "command": record.command,
                    "output_mode": config.output_mode.as_str(),
                    "total_ms": total_ms,
                    "budget_ms": budget_ms,
                    "over_budget": over_budget,
                    "slowest_phase": slowest_phase.map(|(phase, ms)| serde_json::json!({
                        "name": phase,
                        "ms": ms,
                    })),
                    "next_actions": next_actions,
                    "exit_code": record.exit_code,
                    "phases_ms": record.phase_millis(),
                    "cargo": cargo_timing.as_ref().map(CargoTimingContext::as_json),
                }
            });
            eprintln!("{payload}");
        }
        OutputMode::Quiet => {}
        OutputMode::Verbose | OutputMode::Standard => {
            let cargo_suffix = cargo_timing
                .as_ref()
                .map(|cargo| {
                    format!(
                        " cargo_target_dir_policy={} effective_cargo_target_dir={} cargo_wait_classification={}",
                        cargo.target_dir_policy,
                        cargo.effective_cargo_target_dir,
                        cargo.wait_classification
                    )
                })
                .unwrap_or_default();
            eprintln!(
                "vida_timing command={} total_ms={} budget_ms={:?} over_budget={} exit_code={} phases_ms={:?} next_actions={:?}{}",
                record.command,
                total_ms,
                budget_ms,
                over_budget,
                record.exit_code.unwrap_or_default(),
                record.phase_millis(),
                next_actions,
                cargo_suffix
            );
        }
    }
}

fn timing_next_actions(
    over_budget: bool,
    slowest_phase: Option<(&'static str, u128)>,
    cargo_timing: Option<&CargoTimingContext>,
) -> Vec<String> {
    if !over_budget {
        return Vec::new();
    }
    let mut actions = Vec::new();
    match slowest_phase {
        Some((phase, ms)) => actions.push(format!(
            "Inspect `{phase}` timing ({ms}ms) and move repeated reads to a cached projection or read-model."
        )),
        None => actions.push(
            "Inspect command timing and move repeated reads to a cached projection or read-model."
                .to_string(),
        ),
    }
    actions.push(
        "Re-run with `VIDA_COMMAND_TIMING_ENABLED=true` and a command-specific budget to verify the fix."
            .to_string(),
    );
    if let Some(cargo_timing) = cargo_timing {
        actions.push(format!(
            "Cargo timing target_dir_policy={} effective_cargo_target_dir={}; lock wait and compile time are unclassified unless the caller also records Cargo phase output.",
            cargo_timing.target_dir_policy, cargo_timing.effective_cargo_target_dir
        ));
        actions.push(
            "Group related focused proof filters in one Cargo/nextest invocation or use `scripts/vida-dev-gate.ps1 -Mode focused-nextest` to avoid repeated artifact-dir lock waits when safe."
                .to_string(),
        );
        actions.push(
            "If parallel proof shards still contend on the artifact directory, serialize the Cargo shard or set an isolated `CARGO_TARGET_DIR` for that shard and record it in timing evidence."
                .to_string(),
        );
    }
    actions
}

fn env_bool(name: &str) -> bool {
    std::env::var(name).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn env_false(name: &str) -> bool {
    std::env::var(name).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        )
    })
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

fn exit_code_to_i32(code: ExitCode) -> i32 {
    if code == ExitCode::SUCCESS {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_timing_context_uses_caller_target_dir() {
        let context = cargo_timing_context_from_parts(
            Some(Path::new("caller-target")),
            Path::new("repo-root"),
            true,
        );

        assert_eq!(context.target_dir_policy, "caller_provided");
        assert_eq!(context.effective_cargo_target_dir, "caller-target");
        assert_eq!(
            context.wait_classification,
            "cargo_wait_unclassified_without_cargo_phase_data"
        );
    }

    #[test]
    fn cargo_timing_context_uses_worktree_shared_target_dir() {
        let context = cargo_timing_context_from_parts(
            None,
            Path::new("repo-root/.vida/worktrees/slice-a"),
            false,
        );

        assert_eq!(context.target_dir_policy, "repo_local_worktree_shared");
        assert!(context
            .effective_cargo_target_dir
            .replace('\\', "/")
            .ends_with("repo-root/.vida/cargo-target"));
        assert_eq!(context.wait_classification, "not_over_budget");
    }

    #[test]
    fn cargo_timing_next_actions_recommend_grouped_proof_and_target_dir_policy() {
        let cargo_timing = cargo_timing_context_from_parts(
            Some(Path::new("caller-target")),
            Path::new("repo-root"),
            true,
        );

        let actions = timing_next_actions(true, Some(("execution", 45_000)), Some(&cargo_timing));

        assert!(actions
            .iter()
            .any(|action| action.contains("target_dir_policy=caller_provided")));
        assert!(actions
            .iter()
            .any(|action| action.contains("Group related focused proof filters")));
        assert!(actions
            .iter()
            .any(|action| action.contains("CARGO_TARGET_DIR")));
    }

    #[test]
    fn non_cargo_timing_next_actions_keep_generic_guidance() {
        let actions = timing_next_actions(true, Some(("execution", 2_500)), None);

        assert!(actions
            .iter()
            .any(|action| action.contains("cached projection or read-model")));
        assert!(!actions
            .iter()
            .any(|action| action.contains("CARGO_TARGET_DIR")));
    }
}
