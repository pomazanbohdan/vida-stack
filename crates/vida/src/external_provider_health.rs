#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ExternalProviderHealthInput<'a> {
    pub(crate) backend_id: &'a str,
    pub(crate) provider: &'a str,
    pub(crate) last_probe_at: Option<&'a str>,
    pub(crate) latency_ms_avg: Option<u64>,
    pub(crate) error_rate_window: f64,
    pub(crate) consecutive_failures: u64,
    pub(crate) latest_error_class: Option<&'a str>,
    pub(crate) cooldown_until: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ExternalHealthCircuitBreakerConfig {
    pub(crate) consecutive_failure_limit: u64,
    pub(crate) cooldown_seconds: u64,
    pub(crate) timeout_failure_weight: u64,
    pub(crate) provider_error_failure_weight: u64,
    pub(crate) malformed_result_failure_weight: u64,
}

impl Default for ExternalHealthCircuitBreakerConfig {
    fn default() -> Self {
        Self {
            consecutive_failure_limit: 3,
            cooldown_seconds: 60,
            timeout_failure_weight: 1,
            provider_error_failure_weight: 1,
            malformed_result_failure_weight: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ExternalProviderHealthState {
    pub(crate) backend_id: String,
    pub(crate) provider: String,
    pub(crate) status: &'static str,
    pub(crate) last_probe_at: Option<String>,
    pub(crate) latency_ms_avg: Option<u64>,
    pub(crate) error_rate_window: f64,
    pub(crate) consecutive_failures: u64,
    pub(crate) cooldown_until: Option<String>,
    pub(crate) blocker_codes: Vec<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) selection_penalty_applied: bool,
    pub(crate) hot_path_probe_allowed: bool,
}

impl ExternalProviderHealthState {
    pub(crate) fn blocks_candidate(&self) -> bool {
        matches!(self.status, "cooldown" | "blocked")
    }

    pub(crate) fn penalizes_candidate(&self) -> bool {
        self.selection_penalty_applied || self.blocks_candidate()
    }
}

pub(crate) fn evaluate_external_provider_health(
    input: ExternalProviderHealthInput<'_>,
    config: &ExternalHealthCircuitBreakerConfig,
) -> ExternalProviderHealthState {
    let weighted_failures =
        input.consecutive_failures + latest_error_weight(input.latest_error_class, config);
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let cooldown_active = input
        .cooldown_until
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some();
    let status = if cooldown_active {
        blocker_codes.push("external_provider_cooldown_active".to_string());
        next_actions.push("wait for cooldown or refresh external carrier readiness".to_string());
        "cooldown"
    } else if weighted_failures >= config.consecutive_failure_limit {
        blocker_codes.push("external_provider_circuit_open".to_string());
        next_actions.push("open circuit breaker cooldown before routing new work".to_string());
        "blocked"
    } else if weighted_failures > 0 || input.error_rate_window >= 0.05 {
        "degraded"
    } else {
        "ready"
    };
    let selection_penalty_applied = matches!(status, "degraded" | "cooldown" | "blocked");

    ExternalProviderHealthState {
        backend_id: input.backend_id.trim().to_string(),
        provider: input.provider.trim().to_string(),
        status,
        last_probe_at: input.last_probe_at.map(str::to_string),
        latency_ms_avg: input.latency_ms_avg,
        error_rate_window: round4(input.error_rate_window.max(0.0)),
        consecutive_failures: input.consecutive_failures,
        cooldown_until: input.cooldown_until.map(str::to_string),
        blocker_codes,
        next_actions,
        selection_penalty_applied,
        hot_path_probe_allowed: false,
    }
}

pub(crate) fn latest_error_weight(
    latest_error_class: Option<&str>,
    config: &ExternalHealthCircuitBreakerConfig,
) -> u64 {
    match latest_error_class
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some("timeout") | Some("provider_timeout") => config.timeout_failure_weight,
        Some("provider_error") | Some("auth_error") | Some("rate_limited") => {
            config.provider_error_failure_weight
        }
        Some("malformed_result") | Some("invalid_receipt") => {
            config.malformed_result_failure_weight
        }
        Some(_) => 1,
        None => 0,
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ExternalProviderHealthInput<'static> {
        ExternalProviderHealthInput {
            backend_id: "pi_cli",
            provider: "pi",
            last_probe_at: Some("2026-05-24T18:00:00Z"),
            latency_ms_avg: Some(820),
            error_rate_window: 0.0,
            consecutive_failures: 0,
            latest_error_class: None,
            cooldown_until: None,
        }
    }

    #[test]
    fn ready_health_does_not_penalize_candidate() {
        let health = evaluate_external_provider_health(input(), &Default::default());

        assert_eq!(health.status, "ready");
        assert!(!health.penalizes_candidate());
        assert!(!health.hot_path_probe_allowed);
    }

    #[test]
    fn low_failure_window_degrades_without_blocking() {
        let health = evaluate_external_provider_health(
            ExternalProviderHealthInput {
                error_rate_window: 0.12,
                consecutive_failures: 1,
                latest_error_class: Some("provider_timeout"),
                ..input()
            },
            &Default::default(),
        );

        assert_eq!(health.status, "degraded");
        assert!(health.penalizes_candidate());
        assert!(!health.blocks_candidate());
    }

    #[test]
    fn malformed_result_weight_can_open_circuit() {
        let health = evaluate_external_provider_health(
            ExternalProviderHealthInput {
                consecutive_failures: 1,
                latest_error_class: Some("malformed_result"),
                ..input()
            },
            &Default::default(),
        );

        assert_eq!(health.status, "blocked");
        assert!(health.blocks_candidate());
        assert_eq!(
            health.blocker_codes,
            vec!["external_provider_circuit_open".to_string()]
        );
    }

    #[test]
    fn cooldown_status_blocks_candidate_until_external_readiness_refresh() {
        let health = evaluate_external_provider_health(
            ExternalProviderHealthInput {
                cooldown_until: Some("2026-05-24T18:01:00Z"),
                ..input()
            },
            &Default::default(),
        );

        assert_eq!(health.status, "cooldown");
        assert!(health.blocks_candidate());
        assert_eq!(
            health.next_actions,
            vec!["wait for cooldown or refresh external carrier readiness".to_string()]
        );
    }
}
