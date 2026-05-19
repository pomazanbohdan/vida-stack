#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROMPT="Reply with OK"

resolve_binary() {
  local name="$1"
  local env_var="${2:-}"
  local configured=""

  if [[ -n "$env_var" ]]; then
    configured="${!env_var:-}"
    if [[ -n "$configured" ]]; then
      if [[ -x "$configured" ]]; then
        printf '%s\n' "$configured"
        return 0
      fi
      echo "[skip] $name: $env_var is set but is not executable: $configured" >&2
      return 1
    fi
  fi

  if command -v "$name" >/dev/null 2>&1; then
    command -v "$name"
    return 0
  fi

  local dir suffix candidate
  for dir in "$ROOT_DIR/target/debug" "$ROOT_DIR/target/release"; do
    for suffix in "" ".exe"; do
      candidate="$dir/$name$suffix"
      if [[ -x "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done
  done

  return 1
}

run_optional_cli() {
  local label="$1"
  local command_name="$2"
  shift 2

  echo "[$label]"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "[skip] $label: $command_name not found"
    return 0
  fi

  "$@"
}

validate_adapter_json() {
  local output_path="$1"
  local python_bin=""
  if command -v python >/dev/null 2>&1; then
    python_bin="$(command -v python)"
  elif command -v python3 >/dev/null 2>&1; then
    python_bin="$(command -v python3)"
  fi

  if [[ -n "$python_bin" ]]; then
    if "$python_bin" - "$output_path" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    payload = json.load(handle)
assert payload.get("type") == "result", payload
assert payload.get("is_error") is False, payload
assert "fake final" in payload.get("result", ""), payload
assert payload.get("raw_provider", {}).get("provider") == "pi", payload
PY
    then
      return 0
    fi
    echo "[warn] Python JSON validation failed; falling back to grep checks" >&2
  fi

  grep -q '"type":"result"' "$output_path"
  grep -q '"is_error":false' "$output_path"
  grep -q 'fake final' "$output_path"
}

run_pi_adapter_smoke() {
  local adapter=""
  local fake_pi=""
  local pi_cmd=""

  echo "[pi:vida-pi-agent-help]"
  if adapter="$(resolve_binary vida-pi-agent VIDA_PI_AGENT_BIN)"; then
    "$adapter" --help >/tmp/vida-pi-agent-smoke-help.txt
    grep -q "VIDA adapter for one-shot Pi RPC dispatch" /tmp/vida-pi-agent-smoke-help.txt
  else
    echo "[skip] pi: vida-pi-agent not found on PATH or in target/debug,target/release"
    return 0
  fi

  echo "[pi:fake-rpc]"
  if fake_pi="$(resolve_binary vida-pi-agent-fake-pi VIDA_PI_AGENT_FAKE_PI_BIN)"; then
    local output_path
    output_path="$(mktemp)"
    VIDA_PI_AGENT_FAKE_SCENARIO=success \
      "$adapter" \
      --pi-command "$fake_pi" \
      --mode rpc \
      --model openai-codex/gpt-5.5 \
      --thinking-level medium \
      --timeout-seconds 10 \
      "$PROMPT" >"$output_path"
    validate_adapter_json "$output_path"
    rm -f "$output_path"
  else
    echo "[skip] pi fake RPC: vida-pi-agent-fake-pi not found on PATH or in target/debug,target/release"
  fi

  echo "[pi:live-rpc]"
  if [[ "${VIDA_PI_LIVE_SMOKE:-0}" != "1" ]]; then
    echo "[skip] pi live RPC: set VIDA_PI_LIVE_SMOKE=1 to enable"
    return 0
  fi
  if ! pi_cmd="$(resolve_binary pi VIDA_PI_COMMAND)"; then
    echo "[skip] pi live RPC: pi command not found"
    return 0
  fi
  VIDA_PI_AGENT_FAKE_SCENARIO= \
    "$adapter" \
    --pi-command "$pi_cmd" \
    --mode rpc \
    --no-session \
    --no-context-files \
    --no-skills \
    --no-extensions \
    --no-prompt-templates \
    --no-tools \
    --timeout-seconds "${VIDA_PI_LIVE_SMOKE_TIMEOUT_SECONDS:-60}" \
    "$PROMPT" >/tmp/vida-pi-agent-live-smoke.json
  grep -q '"type":"result"' /tmp/vida-pi-agent-live-smoke.json
}

if [[ "${VIDA_EXTERNAL_CLI_SMOKE_ONLY_PI:-0}" == "1" ]]; then
  run_pi_adapter_smoke
  exit 0
fi

run_optional_cli qwen qwen qwen -y -o text --model coder-model "$PROMPT"
run_optional_cli hermes hermes hermes chat -Q -q "$PROMPT"
run_optional_cli opencode opencode opencode run --model opencode/minimax-m2.5-free --dir "$ROOT_DIR" "$PROMPT"
run_optional_cli kilo kilo kilo run --auto --model kilo/x-ai/grok-code-fast-1:optimized:free --dir "$ROOT_DIR" "$PROMPT"
run_optional_cli vibe vibe vibe -p "$PROMPT" --output text --max-turns 1 --workdir "$ROOT_DIR"
run_pi_adapter_smoke
