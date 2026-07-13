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
    } else {
        blocker_codes.extend(pack_catalog_blockers(&packs));
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

fn pack_catalog_blockers(packs: &[serde_json::Value]) -> Vec<String> {
    let mut blockers = Vec::new();
    if packs.is_empty() {
        blockers.push("pack_catalog_empty".to_string());
    }
    let defaults = packs
        .iter()
        .filter(|pack| pack["default"].as_bool() == Some(true))
        .filter_map(|pack| pack["pack_id"].as_str())
        .collect::<Vec<_>>();
    if defaults.len() != 1 {
        blockers.push(format!(
            "pack_catalog_default_count_mismatch:expected=1:actual={}",
            defaults.len()
        ));
    }
    blockers
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

    fn write_registry_fixture(root: &TempStateHarness, packs: &str) {
        std::fs::write(
            root.path().join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  registries:\n",
                "    packs: packs.yaml\n",
                "    flows: flows.yaml\n",
                "    commands: commands.yaml\n",
            ),
        )
        .expect("config");
        std::fs::write(root.path().join("packs.yaml"), packs).expect("packs");
        std::fs::write(
            root.path().join("flows.yaml"),
            concat!(
                "version: 1\nflow_sets:\n",
                "  - flow_id: quick-two-pack-flow\n",
                "  - flow_id: spec-four-pack-flow\n",
                "  - flow_id: full-six-pack-flow\n",
            ),
        )
        .expect("flows");
        std::fs::write(
            root.path().join("commands.yaml"),
            concat!(
                "version: 1\ncommands:\n",
                "  - command_id: agent-init-worker\n",
                "  - command_id: agent-init-business-analyst\n",
                "  - command_id: agent-init-coach\n",
                "  - command_id: agent-init-solution-architect\n",
                "  - command_id: agent-init-verifier\n",
                "  - command_id: agent-init-prover\n",
            ),
        )
        .expect("commands");
    }

    #[test]
    fn pack_payload_reads_config_root_without_runtime_state() {
        let root = TempStateHarness::new().expect("temp state harness");
        std::fs::write(
            root.path().join("vida.config.yaml"),
            concat!(
                "agent_extensions:\n",
                "  registries:\n",
                "    packs: packs.yaml\n",
                "    flows: flows.yaml\n",
                "    commands: commands.yaml\n",
            ),
        )
        .expect("config");
        std::fs::write(
            root.path().join("packs.yaml"),
            concat!(
                "version: 1\n",
                "packs:\n",
                "  - pack_id: quick-two-pack\n",
                "    flow_id: quick-two-pack-flow\n",
                "    enabled: true\n",
                "    ordered_steps:\n",
                "      - role_id: coder\n",
                "        command_ref: agent-init-worker\n",
                "      - role_id: cleaner\n",
                "        command_ref: agent-init-worker\n",
                "        proof_target: agent:quick-two-pack:cleaner\n",
            ),
        )
        .expect("packs");
        std::fs::write(
            root.path().join("flows.yaml"),
            "version: 1\nflow_sets:\n  - flow_id: quick-two-pack-flow\n",
        )
        .expect("flows");
        std::fs::write(
            root.path().join("commands.yaml"),
            "version: 1\ncommands:\n  - command_id: agent-init-worker\n",
        )
        .expect("commands");
        let _cwd = guard_current_dir(root.path());

        let payload = pack_payload(Some("quick-two-pack")).expect("payload");

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
            concat!(
                "agent_extensions:\n",
                "  registries:\n",
                "    packs: packs.yaml\n",
                "    flows: flows.yaml\n",
                "    commands: commands.yaml\n",
            ),
        )
        .expect("config");
        std::fs::write(root.path().join("packs.yaml"), "version: 1\npacks: []\n").expect("packs");
        std::fs::write(
            root.path().join("flows.yaml"),
            "version: 1\nflow_sets: []\n",
        )
        .expect("flows");
        std::fs::write(
            root.path().join("commands.yaml"),
            "version: 1\ncommands: []\n",
        )
        .expect("commands");
        let _cwd = guard_current_dir(root.path());

        let payload = pack_payload(Some("missing")).expect("payload");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["command"], "vida pack list");
        assert_eq!(payload["machine_command"], "vida pack list --json");
    }

    #[test]
    fn zero_and_malformed_catalogs_fail_closed_in_public_json() {
        for (packs, blocker) in [
            ("version: 1\npacks: []\n", None),
            (
                "version: 1\npacks:\n  - enabled: true\n",
                Some("missing_pack_id:0"),
            ),
        ] {
            let root = TempStateHarness::new().expect("temp state harness");
            write_registry_fixture(&root, packs);
            let _cwd = guard_current_dir(root.path());

            let payload = pack_payload(None).expect("public JSON payload");

            assert_eq!(payload["pack_count"], 0);
            assert_eq!(payload["status"], "blocked");
            assert!(payload["blocker_codes"].as_array().is_some_and(|codes| {
                codes.iter().any(|code| code == "pack_catalog_empty")
                    && codes.iter().any(|code| {
                        code == "pack_catalog_default_count_mismatch:expected=1:actual=0"
                    })
            }));
            if let Some(blocker) = blocker {
                assert!(
                    payload["blocker_codes"]
                        .as_array()
                        .is_some_and(|codes| codes.iter().any(|code| code == blocker))
                );
            }
        }
    }

    #[test]
    fn one_default_arbitrary_pack_is_ready() {
        let root = TempStateHarness::new().expect("temp state harness");
        write_registry_fixture(
            &root,
            concat!(
                "version: 1\npacks:\n",
                "  - pack_id: arbitrary-pack\n",
                "    flow_id: quick-two-pack-flow\n",
                "    enabled: true\n",
                "    default: true\n",
                "    ordered_steps:\n",
                "      - { role_id: coder, command_ref: agent-init-worker, proof_target: 'agent:arbitrary-pack:coder' }\n",
            ),
        );
        let _cwd = guard_current_dir(root.path());

        let payload = pack_payload(None).expect("payload");

        assert_eq!(payload["status"], "ready");
        assert_eq!(payload["packs"][0]["default"], true);
        assert_eq!(payload["packs"][0]["pack_id"], "arbitrary-pack");
        assert_eq!(
            payload["packs"][0]["ordered_steps"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn pack_catalog_rejects_multiple_defaults() {
        let root = TempStateHarness::new().expect("temp state harness");
        write_registry_fixture(
            &root,
            concat!(
                "version: 1\npacks:\n",
                "  - pack_id: first-pack\n",
                "    flow_id: quick-two-pack-flow\n",
                "    default: true\n",
                "    ordered_steps:\n",
                "      - { role_id: coder, command_ref: agent-init-worker, proof_target: 'agent:first-pack:coder' }\n",
                "  - pack_id: second-pack\n",
                "    flow_id: quick-two-pack-flow\n",
                "    default: true\n",
                "    ordered_steps:\n",
                "      - { role_id: coder, command_ref: agent-init-worker, proof_target: 'agent:second-pack:coder' }\n",
            ),
        );
        let _cwd = guard_current_dir(root.path());

        let payload = pack_payload(None).expect("payload");

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"].as_array().is_some_and(|codes| {
            codes
                .iter()
                .any(|code| code == "pack_catalog_default_count_mismatch:expected=1:actual=2")
        }));
    }

    #[test]
    fn project_catalog_exposes_configured_packs_and_one_default() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let _cwd = guard_current_dir(&root);

        let payload = pack_payload(None).expect("project catalog payload");
        let packs = payload["packs"].as_array().expect("packs");

        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0]["pack_id"], "quick-two-pack");
        assert_eq!(packs[0]["ordered_steps"].as_array().unwrap().len(), 2);
        let defaults = packs
            .iter()
            .filter(|pack| pack["default"] == true)
            .collect::<Vec<_>>();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0]["pack_id"], "quick-two-pack");
        assert_eq!(payload["status"], "ready");
    }

    #[test]
    fn duplicate_pack_ids_are_deduplicated_as_public_failure_codes() {
        let root = TempStateHarness::new().expect("temp state harness");
        write_registry_fixture(
            &root,
            concat!(
                "version: 1\npacks:\n",
                "  - &pack\n",
                "    pack_id: arbitrary-pack\n",
                "    flow_id: quick-two-pack-flow\n",
                "    ordered_steps:\n",
                "      - { role_id: coder, command_ref: agent-init-worker }\n",
                "      - { role_id: cleaner, command_ref: agent-init-worker, proof_target: 'agent:arbitrary-pack:cleaner' }\n",
                "  - *pack\n",
            ),
        );
        let _cwd = guard_current_dir(root.path());

        let payload = pack_payload(None).expect("payload");
        let blockers = payload["blocker_codes"].as_array().expect("blockers");

        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            blockers
                .iter()
                .filter(|code| **code == "duplicate_pack_id:arbitrary-pack")
                .count(),
            1
        );
        assert_eq!(payload["command"], "vida pack list");
        assert_eq!(payload["machine_command"], "vida pack list --json");
    }
}
