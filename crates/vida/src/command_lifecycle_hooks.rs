use std::collections::BTreeMap;
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
    let next_actions = timing_next_actions(over_budget, slowest_phase);
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
                }
            });
            eprintln!("{payload}");
        }
        OutputMode::Quiet => {}
        OutputMode::Verbose | OutputMode::Standard => {
            eprintln!(
                "vida_timing command={} total_ms={} budget_ms={:?} over_budget={} exit_code={} phases_ms={:?} next_actions={:?}",
                record.command,
                total_ms,
                budget_ms,
                over_budget,
                record.exit_code.unwrap_or_default(),
                record.phase_millis(),
                next_actions
            );
        }
    }
}

fn timing_next_actions(
    over_budget: bool,
    slowest_phase: Option<(&'static str, u128)>,
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
