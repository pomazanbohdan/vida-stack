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
    pub(crate) latest_error_class: Option<String>,
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
    let latest_error_class = normalized_error_class(input.latest_error_class);
    let weighted_failures =
        input.consecutive_failures + latest_error_weight(latest_error_class, config);
    let mut blocker_codes = Vec::new();
    let mut next_actions = Vec::new();
    let cooldown_active = input
        .cooldown_until
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some();
    let status = if latest_error_class == Some("auth_error") {
        blocker_codes.push("provider_auth_failed".to_string());
        next_actions
            .push("repair external provider credentials before routing new work".to_string());
        "blocked"
    } else if cooldown_active {
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
        latest_error_class: latest_error_class.map(str::to_string),
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
    match normalized_error_class(latest_error_class) {
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

pub(crate) fn classify_external_provider_error(text: &str) -> Option<&'static str> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.contains("invalid api key")
        || normalized.contains("missing api key")
        || normalized.contains("authentication failed")
        || normalized.contains("auth failure")
        || normalized.contains("unauthorized")
        || normalized.contains("invalid access token")
        || normalized.contains("token expired")
    {
        return Some("auth_error");
    }
    if normalized.contains("rate limit")
        || normalized.contains("too many requests")
        || normalized.contains("quota exceeded")
        || normalized.contains("exceeded your current quota")
    {
        return Some("rate_limited");
    }
    if normalized.contains("timed out") || normalized.contains("timeout") {
        return Some("provider_timeout");
    }
    if normalized.contains("api error") || normalized.contains("provider error") {
        return Some("provider_error");
    }
    None
}

pub(crate) fn latest_dispatch_result_health_for_backend(
    project_root: &std::path::Path,
    backend_id: &str,
    provider: &str,
) -> Option<ExternalProviderHealthState> {
    let dispatch_results_dir = project_root
        .join(".vida")
        .join("data")
        .join("state")
        .join("runtime-consumption")
        .join("dispatch-results");
    let mut matching_results = matching_dispatch_result_entries(&dispatch_results_dir, backend_id)?;
    matching_results.sort_by(|left, right| right.0.cmp(&left.0));

    for (_modified, value) in matching_results {
        if dispatch_result_is_success(&value) {
            return None;
        }
        let combined_error_text = dispatch_result_error_text(&value);
        if let Some(latest_error_class) = value
            .pointer("/external_provider_health/latest_error_class")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| normalized_error_class(Some(value)))
            .or_else(|| classify_external_provider_error(&combined_error_text))
        {
            return Some(evaluate_external_provider_health(
                ExternalProviderHealthInput {
                    backend_id,
                    provider,
                    last_probe_at: value.get("recorded_at").and_then(serde_json::Value::as_str),
                    latency_ms_avg: None,
                    error_rate_window: 0.0,
                    consecutive_failures: 1,
                    latest_error_class: Some(latest_error_class),
                    cooldown_until: None,
                },
                &Default::default(),
            ));
        }
        return None;
    }
    None
}

fn matching_dispatch_result_entries(
    dispatch_results_dir: &std::path::Path,
    backend_id: &str,
) -> Option<Vec<(std::time::SystemTime, serde_json::Value)>> {
    let mut matching_results = Vec::new();
    for entry in std::fs::read_dir(dispatch_results_dir)
        .ok()?
        .filter_map(Result::ok)
    {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        if !text.contains(backend_id) {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if !dispatch_result_matches_backend(&value, backend_id) {
            continue;
        }
        matching_results.push((modified, value));
    }
    Some(matching_results)
}

fn normalized_error_class(latest_error_class: Option<&str>) -> Option<&'static str> {
    match latest_error_class
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("auth_error" | "authentication_failed" | "invalid_api_key") => Some("auth_error"),
        Some("timeout" | "provider_timeout") => Some("provider_timeout"),
        Some("rate_limited" | "quota_exceeded") => Some("rate_limited"),
        Some("malformed_result" | "invalid_receipt") => Some("malformed_result"),
        Some("provider_error") => Some("provider_error"),
        Some(_) => Some("provider_error"),
        None => None,
    }
}

fn dispatch_result_matches_backend(value: &serde_json::Value, backend_id: &str) -> bool {
    value.get("surface").and_then(serde_json::Value::as_str)
        == Some(format!("external_cli:{backend_id}").as_str())
        || value
            .get("selected_backend")
            .and_then(serde_json::Value::as_str)
            == Some(backend_id)
        || value
            .pointer("/backend_dispatch/selected_backend")
            .and_then(serde_json::Value::as_str)
            == Some(backend_id)
        || value
            .pointer("/backend_dispatch/backend_id")
            .and_then(serde_json::Value::as_str)
            == Some(backend_id)
}

fn dispatch_result_is_success(value: &serde_json::Value) -> bool {
    value.get("status").and_then(serde_json::Value::as_str) == Some("pass")
        || value
            .get("execution_state")
            .and_then(serde_json::Value::as_str)
            == Some("executed")
}

fn dispatch_result_error_text(value: &serde_json::Value) -> String {
    [
        value.get("provider_error"),
        value.get("provider_error_message"),
        value.get("blocker_reason"),
        value.pointer("/external_provider_health/latest_error_class"),
    ]
    .into_iter()
    .flatten()
    .filter_map(serde_json::Value::as_str)
    .collect::<Vec<_>>()
    .join("\n")
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

    #[test]
    fn auth_error_blocks_immediately() {
        let health = evaluate_external_provider_health(
            ExternalProviderHealthInput {
                latest_error_class: Some("auth_error"),
                ..input()
            },
            &Default::default(),
        );

        assert_eq!(health.status, "blocked");
        assert!(health.blocks_candidate());
        assert_eq!(health.latest_error_class.as_deref(), Some("auth_error"));
        assert_eq!(health.blocker_codes, vec!["provider_auth_failed"]);
    }

    #[test]
    fn latest_dispatch_result_health_classifies_invalid_api_key() {
        let root = std::env::temp_dir().join(format!(
            "vida-provider-health-dispatch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        let dispatch_dir = root
            .join(".vida")
            .join("data")
            .join("state")
            .join("runtime-consumption")
            .join("dispatch-results");
        std::fs::create_dir_all(&dispatch_dir).expect("dispatch dir");
        std::fs::write(
            dispatch_dir.join("result.json"),
            r#"{
              "surface": "external_cli:vibe_cli",
              "status": "blocked",
              "execution_state": "blocked",
              "provider_error": "Error: Invalid API key. Please check your API key.",
              "blocker_code": "configured_backend_dispatch_failed",
              "recorded_at": "2026-06-25T14:00:00Z"
            }"#,
        )
        .expect("dispatch result");

        let health = latest_dispatch_result_health_for_backend(&root, "vibe_cli", "vibe")
            .expect("health should be derived");

        assert_eq!(health.status, "blocked");
        assert_eq!(health.latest_error_class.as_deref(), Some("auth_error"));
        assert_eq!(health.blocker_codes, vec!["provider_auth_failed"]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn latest_dispatch_result_health_matches_backend_before_capping_unrelated_newer_results() {
        let root = std::env::temp_dir().join(format!(
            "vida-provider-health-dispatch-scan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        let dispatch_dir = root
            .join(".vida")
            .join("data")
            .join("state")
            .join("runtime-consumption")
            .join("dispatch-results");
        std::fs::create_dir_all(&dispatch_dir).expect("dispatch dir");
        std::fs::write(
            dispatch_dir.join("vibe-result.json"),
            r#"{
              "surface": "external_cli:vibe_cli",
              "status": "blocked",
              "execution_state": "blocked",
              "provider_error": "Error: Invalid API key. Please check your API key.",
              "recorded_at": "2026-06-25T14:00:00Z"
            }"#,
        )
        .expect("vibe dispatch result");
        std::thread::sleep(std::time::Duration::from_millis(10));
        for index in 0..600 {
            std::fs::write(
                dispatch_dir.join(format!("unrelated-{index:03}.json")),
                format!(
                    r#"{{
                      "surface": "internal_subagents",
                      "selected_backend": "internal_subagents",
                      "status": "pass",
                      "execution_state": "executed",
                      "recorded_at": "2026-06-25T14:00:{index:02}Z"
                    }}"#
                ),
            )
            .expect("unrelated dispatch result");
        }

        let health = latest_dispatch_result_health_for_backend(&root, "vibe_cli", "vibe")
            .expect("older matching auth result should remain visible");

        assert_eq!(health.status, "blocked");
        assert_eq!(health.latest_error_class.as_deref(), Some("auth_error"));
        assert_eq!(health.blocker_codes, vec!["provider_auth_failed"]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn latest_dispatch_result_health_reads_oversized_backend_results() {
        let root = std::env::temp_dir().join(format!(
            "vida-provider-health-dispatch-oversized-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        let dispatch_dir = root
            .join(".vida")
            .join("data")
            .join("state")
            .join("runtime-consumption")
            .join("dispatch-results");
        std::fs::create_dir_all(&dispatch_dir).expect("dispatch dir");
        let oversized_error = format!(
            r#"{{
              "surface": "external_cli:vibe_cli",
              "status": "blocked",
              "execution_state": "blocked",
              "provider_error": "Error: Invalid API key. {}",
              "recorded_at": "2026-06-25T14:00:00Z"
            }}"#,
            "x".repeat((256 * 1024) as usize)
        );
        std::fs::write(
            dispatch_dir.join("oversized-vibe-auth.json"),
            oversized_error,
        )
        .expect("oversized vibe dispatch result");

        let health = latest_dispatch_result_health_for_backend(&root, "vibe_cli", "vibe")
            .expect("oversized matching auth result should remain visible");

        assert_eq!(health.status, "blocked");
        assert_eq!(health.latest_error_class.as_deref(), Some("auth_error"));
        assert_eq!(health.blocker_codes, vec!["provider_auth_failed"]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn latest_dispatch_result_health_newer_backend_success_clears_older_auth_failure() {
        let root = std::env::temp_dir().join(format!(
            "vida-provider-health-dispatch-success-clear-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        let dispatch_dir = root
            .join(".vida")
            .join("data")
            .join("state")
            .join("runtime-consumption")
            .join("dispatch-results");
        std::fs::create_dir_all(&dispatch_dir).expect("dispatch dir");
        std::fs::write(
            dispatch_dir.join("older-vibe-auth.json"),
            r#"{
              "surface": "external_cli:vibe_cli",
              "status": "blocked",
              "execution_state": "blocked",
              "provider_error": "Error: Invalid API key. Please check your API key.",
              "recorded_at": "2026-06-25T14:00:00Z"
            }"#,
        )
        .expect("older vibe dispatch result");
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(
            dispatch_dir.join("newer-vibe-success.json"),
            r#"{
              "surface": "external_cli:vibe_cli",
              "status": "pass",
              "execution_state": "executed",
              "recorded_at": "2026-06-25T14:01:00Z"
            }"#,
        )
        .expect("newer vibe dispatch result");

        let health = latest_dispatch_result_health_for_backend(&root, "vibe_cli", "vibe");

        assert_eq!(health, None);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn latest_dispatch_result_health_newer_unknown_backend_result_clears_older_auth_failure() {
        let root = std::env::temp_dir().join(format!(
            "vida-provider-health-dispatch-unknown-clear-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));
        let dispatch_dir = root
            .join(".vida")
            .join("data")
            .join("state")
            .join("runtime-consumption")
            .join("dispatch-results");
        std::fs::create_dir_all(&dispatch_dir).expect("dispatch dir");
        std::fs::write(
            dispatch_dir.join("older-vibe-auth.json"),
            r#"{
              "surface": "external_cli:vibe_cli",
              "status": "blocked",
              "execution_state": "blocked",
              "provider_error": "Error: Invalid API key. Please check your API key.",
              "recorded_at": "2026-06-25T14:00:00Z"
            }"#,
        )
        .expect("older vibe dispatch result");
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(
            dispatch_dir.join("newer-vibe-unknown-blocked.json"),
            r#"{
              "surface": "external_cli:vibe_cli",
              "status": "blocked",
              "execution_state": "blocked",
              "blocker_reason": "operator stopped before details were captured",
              "recorded_at": "2026-06-25T14:01:00Z"
            }"#,
        )
        .expect("newer vibe dispatch result");

        let health = latest_dispatch_result_health_for_backend(&root, "vibe_cli", "vibe");

        assert_eq!(health, None);

        let _ = std::fs::remove_dir_all(root);
    }
}
