const TEST_VERSION_OUTPUT_ENV: &str = "VIDA_TEST_CODEX_CLI_VERSION_OUTPUT";
#[cfg(test)]
pub(crate) static CODEX_CLI_TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct CodexCliVersion(Vec<u64>);

impl CodexCliVersion {
    fn parse(raw: &str) -> Option<Self> {
        let version_start = raw.find(|ch: char| ch.is_ascii_digit())?;
        let version = raw[version_start..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
            .collect::<String>();
        if version.is_empty() {
            return None;
        }
        let parts = version
            .split('.')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if parts.is_empty() {
            return None;
        }
        Some(Self(parts))
    }

    fn satisfies_minimum(&self, minimum: &Self) -> bool {
        let max_len = self.0.len().max(minimum.0.len()).max(3);
        for index in 0..max_len {
            let current = self.0.get(index).copied().unwrap_or(0);
            let required = minimum.0.get(index).copied().unwrap_or(0);
            match current.cmp(&required) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => {}
            }
        }
        true
    }

    fn render(&self) -> String {
        self.0
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(".")
    }
}

fn command_contains_path_separator(command: &str) -> bool {
    command.contains('/') || command.contains('\\')
}

fn command_path_candidates(base: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut candidates = vec![base.to_path_buf()];
    if cfg!(windows) && base.extension().is_none() {
        let pathext =
            std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        for extension in pathext.split(';').map(str::trim) {
            if !extension.is_empty() {
                candidates.push(base.with_extension(extension.trim_start_matches('.')));
            }
        }
    }
    candidates
}

fn command_is_resolvable(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    #[cfg(test)]
    if command == "sh" {
        return true;
    }
    let command_path = std::path::Path::new(command);
    if command_path.is_absolute() || command_contains_path_separator(command) {
        return command_path_candidates(command_path)
            .iter()
            .any(|candidate| candidate.is_file());
    }
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| {
        command_path_candidates(&dir.join(command))
            .iter()
            .any(|candidate| candidate.is_file())
    })
}

fn readiness_min_version(profile: &serde_json::Value) -> Option<String> {
    let readiness = &profile["readiness"];
    readiness["codex_cli"]["min_version"]
        .as_str()
        .or_else(|| readiness["cli"]["min_version"].as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn is_codex_profile_readiness(profile: &serde_json::Value) -> bool {
    profile["readiness"]["mode"].as_str().map(str::trim) == Some("codex_profile")
}

fn command_version_output(command: &str) -> Result<(String, serde_json::Value), serde_json::Value> {
    if let Ok(output) = std::env::var(TEST_VERSION_OUTPUT_ENV) {
        return Ok((
            output,
            serde_json::json!({
                "status": "test_override",
                "source": TEST_VERSION_OUTPUT_ENV,
            }),
        ));
    }
    if !command_is_resolvable(command) {
        return Err(serde_json::json!({
            "status": "command_not_found",
        }));
    }
    match std::process::Command::new(command)
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Ok((
                if stdout.is_empty() { stderr } else { stdout },
                serde_json::json!({
                    "status": "resolved",
                    "version_probe_exit_code": output.status.code(),
                }),
            ))
        }
        Ok(output) => Err(serde_json::json!({
            "status": "version_probe_failed",
            "version_probe_exit_code": output.status.code(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })),
        Err(error) => Err(serde_json::json!({
            "status": "version_probe_failed",
            "error": error.to_string(),
        })),
    }
}

pub(crate) fn codex_cli_readiness_verdict_for_profile(
    selected_cli_system: Option<&str>,
    command_source: &str,
    dispatch_command: Option<&str>,
    carrier_id: &str,
    profile: &serde_json::Value,
) -> Option<serde_json::Value> {
    if !is_codex_profile_readiness(profile) {
        return None;
    }
    let selected_model_profile = profile["profile_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let model_ref = profile["model_ref"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let command = dispatch_command
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("codex");
    let selected_cli_system = selected_cli_system.unwrap_or("codex");
    let Some(required_cli_version) = readiness_min_version(profile) else {
        return Some(serde_json::json!({
            "status": "codex_cli_ready",
            "blocked": false,
            "blocker_code": serde_json::Value::Null,
            "selected_cli_system": selected_cli_system,
            "carrier_id": carrier_id,
            "command_resolution": {
                "source": command_source,
                "status": "not_required",
                "command": command,
            },
            "detected_cli_version": serde_json::Value::Null,
            "required_cli_version": serde_json::Value::Null,
            "selected_model_profile": selected_model_profile,
            "model_ref": model_ref,
            "next_actions": [],
        }));
    };
    let mut command_resolution = serde_json::json!({
        "source": command_source,
        "command": command,
    });
    let (version_output, resolution) = match command_version_output(command) {
        Ok((output, resolution)) => (output, resolution),
        Err(resolution) => {
            if let Some(map) = command_resolution.as_object_mut() {
                if let Some(resolution) = resolution.as_object() {
                    map.extend(resolution.clone());
                }
            }
            return Some(serde_json::json!({
                "status": "codex_cli_not_ready",
                "blocked": true,
                "blocker_code": "codex_cli_not_ready",
                "selected_cli_system": selected_cli_system,
                "carrier_id": carrier_id,
                "command_resolution": command_resolution,
                "detected_cli_version": serde_json::Value::Null,
                "required_cli_version": required_cli_version,
                "selected_model_profile": selected_model_profile,
                "model_ref": model_ref,
                "next_actions": [
                    format!("Install or expose `{command}` on PATH, or update `host_environment.systems.{selected_cli_system}.dispatch.command`."),
                    "Rerun `vida agent-init --json` after restoring Codex CLI readiness."
                ],
            }));
        }
    };
    if let Some(map) = command_resolution.as_object_mut() {
        if let Some(resolution) = resolution.as_object() {
            map.extend(resolution.clone());
        }
    }
    let detected_cli_version = match CodexCliVersion::parse(&version_output) {
        Some(version) => version,
        None => {
            if let Some(map) = command_resolution.as_object_mut() {
                map.insert(
                    "status".to_string(),
                    serde_json::json!("version_parse_failed"),
                );
                map.insert(
                    "version_output".to_string(),
                    serde_json::json!(version_output),
                );
            }
            return Some(serde_json::json!({
                "status": "codex_cli_not_ready",
                "blocked": true,
                "blocker_code": "codex_cli_not_ready",
                "selected_cli_system": selected_cli_system,
                "carrier_id": carrier_id,
                "command_resolution": command_resolution,
                "detected_cli_version": serde_json::Value::Null,
                "required_cli_version": required_cli_version,
                "selected_model_profile": selected_model_profile,
                "model_ref": model_ref,
                "next_actions": [
                    format!("Repair `{command} --version` output parsing or configure a compatible Codex CLI command."),
                    "Rerun `vida agent-init --json` after Codex CLI readiness is restored."
                ],
            }));
        }
    };
    let Some(required) = CodexCliVersion::parse(&required_cli_version) else {
        return Some(serde_json::json!({
            "status": "codex_cli_not_ready",
            "blocked": true,
            "blocker_code": "codex_cli_not_ready",
            "selected_cli_system": selected_cli_system,
            "carrier_id": carrier_id,
            "command_resolution": command_resolution,
            "detected_cli_version": detected_cli_version.render(),
            "required_cli_version": required_cli_version,
            "selected_model_profile": selected_model_profile,
            "model_ref": model_ref,
            "next_actions": [
                format!("Fix invalid Codex CLI min_version `{required_cli_version}` in the selected model profile readiness policy.")
            ],
        }));
    };
    let detected_rendered = detected_cli_version.render();
    if !detected_cli_version.satisfies_minimum(&required) {
        return Some(serde_json::json!({
            "status": "codex_cli_model_incompatible",
            "blocked": true,
            "blocker_code": "codex_cli_model_incompatible",
            "selected_cli_system": selected_cli_system,
            "carrier_id": carrier_id,
            "command_resolution": command_resolution,
            "detected_cli_version": detected_rendered,
            "required_cli_version": required.render(),
            "selected_model_profile": selected_model_profile,
            "model_ref": model_ref,
            "next_actions": [
                format!("Upgrade Codex CLI to at least `{}` before dispatching model `{model_ref}`.", required.render()),
                "Select an eligible fallback model profile whose Codex CLI min_version is compatible with the detected CLI."
            ],
        }));
    }
    Some(serde_json::json!({
        "status": "codex_cli_ready",
        "blocked": false,
        "blocker_code": serde_json::Value::Null,
        "selected_cli_system": selected_cli_system,
        "carrier_id": carrier_id,
        "command_resolution": command_resolution,
        "detected_cli_version": detected_rendered,
        "required_cli_version": required.render(),
        "selected_model_profile": selected_model_profile,
        "model_ref": model_ref,
        "next_actions": [],
    }))
}

pub(crate) fn codex_cli_readiness_verdict_for_carrier(
    selected_cli_system: &str,
    selected_cli_entry: Option<&serde_yaml::Value>,
    carrier: &serde_json::Value,
) -> Option<serde_json::Value> {
    let selected_profile_id = carrier["selected_model_profile_id"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let profile = crate::model_profile_contract::selected_model_profile_from_json_row(
        carrier,
        selected_profile_id,
    )?;
    let dispatch_command = selected_cli_entry
        .and_then(|entry| crate::yaml_lookup(entry, &["dispatch", "command"]))
        .and_then(serde_yaml::Value::as_str);
    let command_source = if dispatch_command.is_some() {
        format!("host_environment.systems.{selected_cli_system}.dispatch.command")
    } else {
        "fallback.codex".to_string()
    };
    codex_cli_readiness_verdict_for_profile(
        Some(selected_cli_system),
        &command_source,
        dispatch_command,
        carrier["role_id"].as_str().unwrap_or_default(),
        &profile,
    )
}

pub(crate) fn codex_dispatch_command_from_json_config(
    config: &serde_json::Value,
) -> (String, Option<String>, String) {
    let selected_cli_system = config["host_environment"]["cli_system"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("codex")
        .to_string();
    let path = format!("host_environment.systems.{selected_cli_system}.dispatch.command");
    let command = config["host_environment"]["systems"][&selected_cli_system]["dispatch"]
        ["command"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let source = if command.is_some() {
        path
    } else {
        "fallback.codex".to_string()
    };
    (selected_cli_system, command, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_cli_version_output() {
        let parsed =
            CodexCliVersion::parse("codex-cli 0.114.0").expect("codex-cli version should parse");
        assert_eq!(parsed.render(), "0.114.0");
        assert!(parsed.satisfies_minimum(&CodexCliVersion::parse("0.113.9").unwrap()));
        assert!(!parsed.satisfies_minimum(&CodexCliVersion::parse("0.115.0").unwrap()));
    }

    #[test]
    fn rejects_gpt55_when_detected_cli_is_below_minimum() {
        let _guard = CODEX_CLI_TEST_ENV_LOCK
            .lock()
            .expect("Codex CLI test env lock should not be poisoned");
        std::env::set_var(TEST_VERSION_OUTPUT_ENV, "codex-cli 0.114.0");
        let profile = serde_json::json!({
            "profile_id": "codex_gpt55_low_write",
            "model_ref": "gpt-5.5",
            "readiness": {
                "mode": "codex_profile",
                "required": true,
                "codex_cli": {
                    "min_version": "0.115.0"
                }
            }
        });
        let verdict = codex_cli_readiness_verdict_for_profile(
            Some("codex"),
            "test",
            Some("codex"),
            "junior",
            &profile,
        )
        .expect("codex profile verdict should render");
        std::env::remove_var(TEST_VERSION_OUTPUT_ENV);

        assert_eq!(verdict["blocked"], true);
        assert_eq!(verdict["blocker_code"], "codex_cli_model_incompatible");
        assert_eq!(verdict["detected_cli_version"], "0.114.0");
        assert_eq!(verdict["required_cli_version"], "0.115.0");
        assert_eq!(verdict["model_ref"], "gpt-5.5");
    }
}
