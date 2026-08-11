use std::process::ExitCode;

use serde_json::json;
use vida_coder::{
    AuthRefSource, PROVIDER_ID, ProviderAuthRef, RigProviderAdapterConfig, provider_readiness,
    redacted_provider_probe,
};

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("provider-check") => provider_check(&args[1..]),
        Some("--service") if args.get(1).map(String::as_str) == Some("dispatch") => {
            let (output, exit_code) = service_dispatch_result(&args[2..]);
            print!("{output}");
            exit_code
        }
        Some("--version") | Some("-V") => {
            println!("vida-coder {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("unsupported vida-coder command");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn provider_check(args: &[String]) -> ExitCode {
    let (output, exit_code) = provider_check_result(args);
    print!("{output}");
    exit_code
}

fn provider_check_config(args: &[String]) -> (bool, RigProviderAdapterConfig) {
    let json_output = args.iter().any(|arg| arg == "--json");
    let model_ref = option_value(args, "--model")
        .or_else(|| option_value(args, "--model-ref"))
        .unwrap_or("vida-coder/provider-configured");
    let model_profile_id =
        option_value(args, "--model-profile").unwrap_or("vida_coder_medium_guarded");
    let auth_ref = option_value(args, "--auth-ref").unwrap_or("env:VIDA_CODER_PROVIDER_AUTH");

    let config = RigProviderAdapterConfig {
        provider: PROVIDER_ID.to_string(),
        model_ref: model_ref.to_string(),
        model_profile_id: model_profile_id.to_string(),
        reasoning_effort: option_value(args, "--reasoning-effort").map(str::to_string),
        auth_ref: ProviderAuthRef {
            profile_ref: auth_ref.to_string(),
            source: auth_source(auth_ref),
        },
    };

    (json_output, config)
}

fn provider_check_result(args: &[String]) -> (String, ExitCode) {
    let (json_output, config) = provider_check_config(args);

    let readiness = provider_readiness(&config);
    let output = if json_output {
        format!("{}\n", redacted_provider_probe(&config))
    } else {
        format!(
            "status: {:?}\nprovider: {}\nmodel_ref: {}\nmodel_profile_id: {}\n",
            readiness.status, readiness.provider, readiness.model_ref, readiness.model_profile_id
        )
    };

    let exit_code = if readiness.blocker_codes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };

    (output, exit_code)
}

fn service_dispatch_result(args: &[String]) -> (String, ExitCode) {
    let json_output = args.iter().any(|arg| arg == "--json");
    let payload = json!({
        "surface": "vida-coder service dispatch",
        "status": "blocked",
        "executes_provider": false,
        "blocker_codes": ["coder_service_dispatch_not_implemented"],
        "next_actions": ["Wire automatic agent-init bootstrap before enabling service dispatch."]
    });

    let output = if json_output {
        format!("{payload}\n")
    } else {
        "status: blocked\nblocker_codes[1]: coder_service_dispatch_not_implemented\n".to_string()
    };

    (output, ExitCode::from(1))
}

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].as_str())
}

fn auth_source(value: &str) -> AuthRefSource {
    if value.starts_with("secret:") {
        AuthRefSource::SecretRef
    } else if value.starts_with("runtime:") {
        AuthRefSource::RuntimeProfile
    } else {
        AuthRefSource::EnvRef
    }
}

fn print_help() {
    println!("vida-coder --version");
    println!("vida-coder provider-check --json");
    println!("vida-coder --service dispatch --json");
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        AuthRefSource, auth_source, option_value, provider_check_config, provider_check_result,
        service_dispatch_result,
    };

    #[test]
    fn provider_check_json_contract_preserves_flag_and_default_projection() {
        let (output, exit_code) = provider_check_result(&["--json".to_string()]);

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);
        let payload: Value = serde_json::from_str(output.trim()).expect("JSON output expected");
        assert_eq!(payload["provider"], "vida-coder");
        assert_eq!(payload["model_ref"], "vida-coder/provider-configured");
        assert_eq!(payload["model_profile_id"], "vida_coder_medium_guarded");
        assert_eq!(payload["auth_ref_source"], "env_ref");
        assert_eq!(payload["auth_ref_present"], true);
        assert_eq!(payload["status"], "ready");
        assert_eq!(payload["blocker_codes"], Value::Array(Vec::new()));
    }

    #[test]
    fn provider_check_config_preserves_explicit_model_profile_auth_and_effort() {
        let args = vec![
            "--model".to_string(),
            "provider/model-primary".to_string(),
            "--model-ref".to_string(),
            "provider/model".to_string(),
            "--model-profile".to_string(),
            "guarded".to_string(),
            "--reasoning-effort".to_string(),
            "high".to_string(),
            "--auth-ref".to_string(),
            "runtime:profile".to_string(),
            "--json".to_string(),
        ];

        let (json_output, config) = provider_check_config(&args);
        assert!(json_output);
        assert_eq!(config.model_ref, "provider/model-primary");
        assert_eq!(config.model_profile_id, "guarded");
        assert_eq!(config.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(config.auth_ref.profile_ref, "runtime:profile");
        assert_eq!(config.auth_ref.source, AuthRefSource::RuntimeProfile);
    }

    #[test]
    fn provider_check_json_contract_reports_blocked_secret_material() {
        let (output, exit_code) = provider_check_result(&[
            "--json".to_string(),
            "--auth-ref".to_string(),
            "secret:sk-raw".to_string(),
        ]);

        assert_eq!(exit_code, std::process::ExitCode::from(1));
        let payload: Value = serde_json::from_str(output.trim()).expect("JSON output expected");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["provider_auth_ref_contains_secret_material"])
        );
    }

    #[test]
    fn option_value_selects_flag_value_without_confusing_similar_flags() {
        let args = vec![
            "--model-profile".to_string(),
            "guarded".to_string(),
            "--model".to_string(),
            "provider/model".to_string(),
            "--model-ref".to_string(),
            "fallback".to_string(),
        ];

        assert_eq!(option_value(&args, "--model"), Some("provider/model"));
        assert_eq!(option_value(&args, "--model-profile"), Some("guarded"));
        assert_eq!(option_value(&args, "--auth-ref"), None);
        assert_eq!(option_value(&["--model".to_string()], "--model"), None);
    }

    #[test]
    fn auth_source_classifies_secret_runtime_and_environment_refs() {
        assert!(matches!(
            auth_source("secret:coder"),
            AuthRefSource::SecretRef
        ));
        assert!(matches!(
            auth_source("runtime:profile"),
            AuthRefSource::RuntimeProfile
        ));
        assert!(matches!(
            auth_source("env:VIDA_CODER_PROVIDER_AUTH"),
            AuthRefSource::EnvRef
        ));
        assert!(matches!(auth_source("plain-ref"), AuthRefSource::EnvRef));
    }

    #[test]
    fn service_dispatch_remains_explicitly_blocked() {
        let (_, default_exit_code) = service_dispatch_result(&[]);
        assert_eq!(
            default_exit_code,
            std::process::ExitCode::from(1)
        );
        let (_, json_exit_code) = service_dispatch_result(&["--json".to_string()]);
        assert_eq!(
            json_exit_code,
            std::process::ExitCode::from(1)
        );
    }

    #[test]
    fn service_dispatch_plain_contract_remains_explicitly_blocked() {
        let (output, exit_code) = service_dispatch_result(&[]);

        assert_eq!(exit_code, std::process::ExitCode::from(1));
        assert_eq!(
            output,
            "status: blocked\nblocker_codes[1]: coder_service_dispatch_not_implemented\n"
        );
    }

    #[test]
    fn service_dispatch_json_contract_remains_explicitly_blocked() {
        let (output, exit_code) = service_dispatch_result(&["--json".to_string()]);

        assert_eq!(exit_code, std::process::ExitCode::from(1));
        let payload: Value = serde_json::from_str(output.trim()).expect("JSON output expected");
        assert_eq!(payload["surface"], "vida-coder service dispatch");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["executes_provider"], false);
        assert_eq!(
            payload["blocker_codes"],
            serde_json::json!(["coder_service_dispatch_not_implemented"])
        );
    }
}
