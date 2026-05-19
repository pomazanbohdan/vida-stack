use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

fn main() {
    let scenario =
        std::env::var("VIDA_PI_AGENT_FAKE_SCENARIO").unwrap_or_else(|_| "success".to_string());
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line).expect("fake Pi command should be JSON");
        let command_type = value["type"].as_str().unwrap_or("");
        match command_type {
            "get_state" => write_line(
                &mut stdout,
                json!({"id": value["id"], "type":"response", "command":"get_state", "success":true, "data":{"model":{"provider":"openai-codex","id":"gpt-5.5"},"thinkingLevel":"medium"}}),
            ),
            "get_available_models" => write_line(
                &mut stdout,
                json!({"id": value["id"], "type":"response", "command":"get_available_models", "success":true, "data":{"models":[{"provider":"openai-codex","id":"gpt-5.5"},{"provider":"openai-codex","modelId":"gpt-5.4-mini"}]}}),
            ),
            "set_model" => {
                if scenario == "invalid_model" {
                    write_line(
                        &mut stdout,
                        json!({"id": value["id"], "type":"response", "command":"set_model", "success":false, "error":"Model not found"}),
                    );
                    return;
                }
                write_line(
                    &mut stdout,
                    json!({"id": value["id"], "type":"response", "command":"set_model", "success":true, "data":{"provider":value["provider"],"id":value["modelId"]}}),
                );
            }
            "set_thinking_level" => write_line(
                &mut stdout,
                json!({"id": value["id"], "type":"response", "command":"set_thinking_level", "success":true}),
            ),
            "prompt" => {
                write_line(
                    &mut stdout,
                    json!({"id": value["id"], "type":"response", "command":"prompt", "success":true}),
                );
                if scenario == "timeout" {
                    std::thread::sleep(std::time::Duration::from_secs(10));
                    return;
                }
                let prompt = value["message"].as_str().unwrap_or("");
                let content = match scenario.as_str() {
                    "touched_in_scope" => {
                        json!({"summary":"fake final","touched_paths":["src/lib.rs"]}).to_string()
                    }
                    "touched_out_of_scope" => {
                        json!({"summary":"fake final","touched_paths":["docs/spec.md"]}).to_string()
                    }
                    "guard_argv_env" => json!({
                        "summary":"fake final",
                        "argv": std::env::args().skip(1).collect::<Vec<_>>(),
                        "env": {
                            "VIDA_PI_AGENT_SCOPE_GUARD_MODE": std::env::var("VIDA_PI_AGENT_SCOPE_GUARD_MODE").ok(),
                            "VIDA_PI_AGENT_PROJECT_ROOT": std::env::var("VIDA_PI_AGENT_PROJECT_ROOT").ok(),
                            "VIDA_PI_AGENT_OWNED_PATHS_JSON": std::env::var("VIDA_PI_AGENT_OWNED_PATHS_JSON").ok(),
                            "VIDA_PI_AGENT_PREWRITE_GUARD_ACTIVE": std::env::var("VIDA_PI_AGENT_PREWRITE_GUARD_ACTIVE").ok(),
                            "VIDA_PI_AGENT_PREWRITE_GUARD_VERSION": std::env::var("VIDA_PI_AGENT_PREWRITE_GUARD_VERSION").ok()
                        }
                    }).to_string(),
                    _ => format!("fake final: {prompt}"),
                };
                write_line(
                    &mut stdout,
                    json!({"type":"event", "event":"agent_end", "messages":[{"role":"assistant", "content": content}]}),
                );
                return;
            }
            _ => write_line(
                &mut stdout,
                json!({"id": value["id"], "type":"response", "command":command_type, "success":false, "error":"unsupported fake command"}),
            ),
        }
    }
}

fn write_line(stdout: &mut io::Stdout, value: Value) {
    writeln!(
        stdout,
        "{}",
        serde_json::to_string(&value).expect("fake Pi event should render")
    )
    .expect("fake Pi stdout should write");
    stdout.flush().expect("fake Pi stdout should flush");
}
