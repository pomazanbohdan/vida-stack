use std::process::ExitCode;

use serde_json::{json, Value};

const CODER_SURFACE: &str = "vida coder";
const DEFAULT_PROVIDER: &str = "codex";

pub fn run_coder_capabilities(json_output: bool) -> ExitCode {
    emit_payload(capabilities_payload(), json_output);
    ExitCode::SUCCESS
}

pub fn run_coder_provider_check(provider: &str, json_output: bool) -> ExitCode {
    let payload = provider_check_payload(provider);
    emit_payload(payload, json_output);
    ExitCode::from(1)
}

pub fn run_coder(provider: &str, request: Option<&str>, json_output: bool) -> ExitCode {
    let payload = run_blocked_payload(provider, request);
    emit_payload(payload, json_output);
    ExitCode::from(1)
}

fn capabilities_payload() -> Value {
    json!({
        "surface": CODER_SURFACE,
        "schema_version": "1",
        "status": "available",
        "feature": {
            "name": "coder",
            "enabled": cfg!(feature = "coder")
        },
        "provider_execution": {
            "status": "not_implemented",
            "fail_closed_before_execution": true
        },
        "commands": [
            {
                "name": "capabilities",
                "status": "available",
                "json": true
            },
            {
                "name": "provider-check",
                "status": "stub",
                "json": true,
                "executes_provider": false
            },
            {
                "name": "run",
                "status": "blocked",
                "json": true,
                "executes_provider": false
            }
        ],
        "default_provider": DEFAULT_PROVIDER,
        "blocker_codes": [],
        "next_actions": [
            "Run `vida coder provider-check --provider codex` before provider-backed execution."
        ]
    })
}

fn provider_check_payload(provider: &str) -> Value {
    let feature_enabled = cfg!(feature = "coder");
    let blocker_code = if feature_enabled {
        "coder_provider_execution_not_implemented"
    } else {
        "coder_feature_disabled"
    };
    let next_action = if feature_enabled {
        "Implement the coder provider adapter before enabling `vida coder run`."
    } else {
        "Build with `--features coder` after the provider adapter exists."
    };

    json!({
        "surface": "vida coder provider-check",
        "schema_version": "1",
        "status": "blocked",
        "provider": provider,
        "feature": {
            "name": "coder",
            "enabled": feature_enabled
        },
        "executes_provider": false,
        "blocker_codes": [blocker_code],
        "next_actions": [next_action]
    })
}

fn run_blocked_payload(provider: &str, request: Option<&str>) -> Value {
    json!({
        "surface": "vida coder run",
        "schema_version": "1",
        "status": "blocked",
        "provider": provider,
        "request_present": request.is_some_and(|value| !value.trim().is_empty()),
        "executes_provider": false,
        "blocker_codes": ["coder_provider_execution_not_implemented"],
        "next_actions": [
            "Implement and validate a provider adapter before allowing `vida coder run` to execute provider code.",
            "Keep `vida coder provider-check --provider codex` as the preflight gate."
        ]
    })
}

fn emit_payload(payload: Value, json_output: bool) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).expect("coder payload should serialize to JSON")
        );
        return;
    }

    println!("{}", human_output(payload));
}

fn human_output(payload: Value) -> String {
    let surface = payload["surface"]
        .as_str()
        .unwrap_or(CODER_SURFACE)
        .to_string();
    let mut lines = vec![format!(
        "status: {}",
        taskflow_format_toon::sanitize_toon_scalar(payload["status"].as_str().unwrap_or("unknown"))
    )];
    if let Some(provider) = payload.get("provider").and_then(Value::as_str) {
        lines.push(format!(
            "provider: {}",
            taskflow_format_toon::sanitize_toon_scalar(provider)
        ));
    }
    if let Some(feature) = payload.get("feature").and_then(Value::as_object) {
        if let Some(enabled) = feature.get("enabled").and_then(Value::as_bool) {
            lines.push(format!("feature_enabled: {enabled}"));
        }
    }
    if let Some(blockers) = payload["blocker_codes"].as_array() {
        if !blockers.is_empty() {
            lines.push(format!("blocker_codes[{}]:", blockers.len()));
            lines.extend(
                blockers
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|code| format!("- {}", taskflow_format_toon::sanitize_toon_scalar(code))),
            );
        }
    }
    if let Some(actions) = payload["next_actions"].as_array() {
        if !actions.is_empty() {
            lines.push(format!("next_actions[{}]:", actions.len()));
            lines.extend(
                actions.iter().filter_map(Value::as_str).map(|action| {
                    format!("- {}", taskflow_format_toon::sanitize_toon_scalar(action))
                }),
            );
        }
    }
    taskflow_format_toon::render_section(&surface, &lines.join("\n  "))
}

#[cfg(test)]
mod tests {
    use super::{capabilities_payload, provider_check_payload, run_blocked_payload};
    use serde_json::Value;

    #[test]
    fn capabilities_payload_advertises_fail_closed_coder_surface() {
        let payload = capabilities_payload();

        assert_eq!(payload["surface"], "vida coder");
        assert_eq!(payload["status"], "available");
        assert_eq!(
            payload["provider_execution"]["fail_closed_before_execution"],
            true
        );
        assert_eq!(payload["commands"][0]["name"], "capabilities");
        assert_eq!(payload["commands"][1]["name"], "provider-check");
        assert_eq!(payload["commands"][1]["executes_provider"], false);
        assert_eq!(payload["commands"][2]["name"], "run");
        assert_eq!(payload["commands"][2]["status"], "blocked");
        assert!(!payload["next_actions"][0]
            .as_str()
            .expect("next action should be a string")
            .contains("--json"));
    }

    #[test]
    fn provider_check_stub_does_not_execute_provider() {
        let payload = provider_check_payload("codex");

        assert_eq!(payload["surface"], "vida coder provider-check");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["provider"], "codex");
        assert_eq!(payload["executes_provider"], false);
        assert!(payload["blocker_codes"]
            .as_array()
            .expect("blocker codes should be an array")
            .iter()
            .any(|code| code.as_str().unwrap_or_default().starts_with("coder_")));
    }

    #[test]
    fn run_stub_fails_closed_before_provider_execution() {
        let payload = run_blocked_payload("codex", Some("bounded request"));

        assert_eq!(payload["surface"], "vida coder run");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["provider"], "codex");
        assert_eq!(payload["request_present"], true);
        assert_eq!(payload["executes_provider"], false);
        assert_eq!(
            payload["blocker_codes"][0],
            "coder_provider_execution_not_implemented"
        );
        assert!(!payload["next_actions"]
            .as_array()
            .expect("next actions should be an array")
            .iter()
            .filter_map(Value::as_str)
            .any(|action| action.contains("--json")));
    }

    #[test]
    fn human_output_uses_compact_toon_without_json_guidance() {
        let payload = run_blocked_payload("codex", Some("bounded request"));
        let output = super::human_output(payload);

        assert!(output.starts_with("vida coder run\n"));
        assert!(output.contains("status: blocked"));
        assert!(output.contains("blocker_codes[1]:"));
        assert!(output.contains("next_actions[2]:"));
        assert!(!output.contains("--json"));
    }
}
