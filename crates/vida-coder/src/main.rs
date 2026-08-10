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
            service_dispatch_blocked(&args[2..])
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

    let readiness = provider_readiness(&config);
    if json_output {
        println!("{}", redacted_provider_probe(&config));
    } else {
        println!("status: {:?}", readiness.status);
        println!("provider: {}", readiness.provider);
        println!("model_ref: {}", readiness.model_ref);
        println!("model_profile_id: {}", readiness.model_profile_id);
    }

    if readiness.blocker_codes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn service_dispatch_blocked(args: &[String]) -> ExitCode {
    let json_output = args.iter().any(|arg| arg == "--json");
    let payload = json!({
        "surface": "vida-coder service dispatch",
        "status": "blocked",
        "executes_provider": false,
        "blocker_codes": ["coder_service_dispatch_not_implemented"],
        "next_actions": ["Wire automatic agent-init bootstrap before enabling service dispatch."]
    });

    if json_output {
        println!("{payload}");
    } else {
        println!("status: blocked");
        println!("blocker_codes[1]: coder_service_dispatch_not_implemented");
    }

    ExitCode::from(1)
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
    use super::{AuthRefSource, auth_source, option_value, service_dispatch_blocked};

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
        assert_eq!(
            service_dispatch_blocked(&[]),
            std::process::ExitCode::from(1)
        );
        assert_eq!(
            service_dispatch_blocked(&["--json".to_string()]),
            std::process::ExitCode::from(1)
        );
    }
}
