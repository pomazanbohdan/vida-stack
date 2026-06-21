use std::process::ExitCode;

use operator_output::toon_report;

use crate::{PackArgs, PackCommand};

pub(crate) async fn run_pack(args: PackArgs) -> ExitCode {
    match args.command {
        PackCommand::List(args) => run_pack_list(args.json),
        PackCommand::Show(args) => run_pack_show(&args.pack_id, args.json),
        PackCommand::Validate(args) => run_pack_validate(args.pack_id.as_deref(), args.json),
    }
}

fn run_pack_list(json_output: bool) -> ExitCode {
    match pack_payload(None) {
        Ok(payload) => {
            emit("vida pack list", &payload, json_output);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run_pack_show(pack_id: &str, json_output: bool) -> ExitCode {
    match pack_payload(Some(pack_id)) {
        Ok(payload) => {
            let ok = payload["status"].as_str() == Some("ready");
            emit("vida pack show", &payload, json_output);
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run_pack_validate(pack_id: Option<&str>, json_output: bool) -> ExitCode {
    match pack_payload(pack_id) {
        Ok(payload) => {
            let ok = payload["blocker_codes"]
                .as_array()
                .is_some_and(|blockers| blockers.is_empty());
            emit("vida pack validate", &payload, json_output);
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn pack_payload(pack_id: Option<&str>) -> Result<serde_json::Value, String> {
    let root = crate::agent_pack_contract::resolve_config_root_for_pack_surface()?;
    let registry = crate::agent_pack_contract::load_pack_registry_for_root(&root)?;
    let packs = match pack_id {
        Some(pack_id) => {
            let Some(pack) = registry
                .packs
                .iter()
                .find(|pack| crate::agent_pack_contract::pack_id_matches(pack, pack_id))
            else {
                return Ok(serde_json::json!({
                    "status": "blocked",
                    "root": root.display().to_string(),
                    "pack_id": pack_id,
                    "source_path": registry.source_path,
                    "packs": [],
                    "blocker_codes": [format!("unknown_pack:{pack_id}")],
                    "command": "vida pack list",
                    "machine_command": "vida pack list --json"
                }));
            };
            vec![pack.clone()]
        }
        None => registry.packs.clone(),
    };
    let mut blocker_codes = registry.blocker_codes.clone();
    if pack_id.is_some() {
        blocker_codes = packs
            .iter()
            .flat_map(crate::agent_pack_contract::pack_validation_blockers)
            .collect();
    }
    blocker_codes.sort();
    blocker_codes.dedup();
    let status = if blocker_codes.is_empty() {
        "ready"
    } else {
        "blocked"
    };
    Ok(serde_json::json!({
        "status": status,
        "root": root.display().to_string(),
        "source_path": registry.source_path,
        "pack_count": packs.len(),
        "packs": packs,
        "blocker_codes": blocker_codes,
        "command": match pack_id {
            Some(pack_id) => format!("vida pack show {pack_id}"),
            None => "vida pack list".to_string(),
        },
        "machine_command": match pack_id {
            Some(pack_id) => format!("vida pack show {pack_id} --json"),
            None => "vida pack list --json".to_string(),
        }
    }))
}

fn emit(surface: &str, payload: &serde_json::Value, json_output: bool) {
    if json_output {
        crate::print_json_pretty(payload);
    } else {
        println!("{}", toon_report::render_value(surface, payload.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_state::TempStateHarness;
    use crate::test_cli_support::guard_current_dir;

    #[test]
    fn pack_payload_reads_config_root_without_runtime_state() {
        let root = TempStateHarness::new().expect("temp state harness");
        std::fs::write(
            root.path().join("vida.config.yaml"),
            "agent_extensions:\n  registries:\n    packs: packs.yaml\n",
        )
        .expect("config");
        std::fs::write(
            root.path().join("packs.yaml"),
            concat!(
                "version: 1\n",
                "packs:\n",
                "  - pack_id: quick-two-pack\n",
                "    aliases: [quick_two_pack]\n",
                "    flow_id: quick_two_pack_flow\n",
                "    enabled: true\n",
                "    ordered_steps:\n",
                "      - role_id: coder\n",
                "      - role_id: cleaner\n",
                "        proof_target: agent:quick_two_pack:cleaner\n",
            ),
        )
        .expect("packs");
        let _cwd = guard_current_dir(root.path());

        let payload = pack_payload(Some("quick_two_pack")).expect("payload");

        assert_eq!(payload["status"], "ready");
        assert_eq!(payload["packs"][0]["pack_id"], "quick-two-pack");
        assert_eq!(payload["packs"][0]["ordered_steps"][0]["role_id"], "coder");
        assert_eq!(
            payload["packs"][0]["ordered_steps"][1]["worktree_policy"],
            "isolated_per_task"
        );
    }

    #[test]
    fn unknown_pack_fails_closed_with_default_command_hint() {
        let root = TempStateHarness::new().expect("temp state harness");
        std::fs::write(
            root.path().join("vida.config.yaml"),
            "agent_extensions:\n  registries:\n    packs: packs.yaml\n",
        )
        .expect("config");
        std::fs::write(root.path().join("packs.yaml"), "version: 1\npacks: []\n").expect("packs");
        let _cwd = guard_current_dir(root.path());

        let payload = pack_payload(Some("missing")).expect("payload");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["command"], "vida pack list");
        assert_eq!(payload["machine_command"], "vida pack list --json");
    }
}
