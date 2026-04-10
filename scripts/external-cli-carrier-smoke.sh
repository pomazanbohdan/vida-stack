#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROMPT="Reply with OK"

echo "[qwen]"
qwen -y -o text --model coder-model "$PROMPT"

echo "[hermes]"
hermes chat -Q -q "$PROMPT"

echo "[opencode]"
opencode run --model opencode/minimax-m2.5-free --dir "$ROOT_DIR" "$PROMPT"

echo "[kilo]"
kilo run --auto --model kilo/x-ai/grok-code-fast-1:optimized:free --dir "$ROOT_DIR" "$PROMPT"

echo "[vibe]"
vibe -p "$PROMPT" --output text --max-turns 1 --workdir "$ROOT_DIR"
