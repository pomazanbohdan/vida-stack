#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCHIVE_PATH=""
KEEP_TEMP="${VIDA_INSTALLER_SMOKE_KEEP_TEMP:-0}"

usage() {
  cat <<'EOF'
Usage: bash scripts/local-installer-smoke.sh --archive <dist/vida-stack-v*.tar.gz>

Runs the Linux installer smoke against an already-built VIDA release archive.
This is intentionally separate from release building so it can be used as a
fast local/pre-push proof after an archive exists.
EOF
}

fail() {
  printf '[installer-smoke] ERROR: %s\n' "$*" >&2
  dump_artifacts >&2 || true
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --archive)
      ARCHIVE_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "Unknown argument: $1"
      ;;
  esac
done

if [[ -z "$ARCHIVE_PATH" ]]; then
  ARCHIVE_PATH="$(find "$ROOT_DIR/dist" -maxdepth 1 -name 'vida-stack-v*.tar.gz' | head -n 1 || true)"
fi
[[ -n "$ARCHIVE_PATH" ]] || fail "Missing archive. Build release assets first or pass --archive <path>."
[[ -f "$ARCHIVE_PATH" ]] || fail "Archive does not exist: $ARCHIVE_PATH"

TMP_ROOT="$(mktemp -d)"
INSTALL_ROOT="$TMP_ROOT/install-root"
TMP_BIN="$TMP_ROOT/bin"
TMP_PROJECT="$TMP_ROOT/project"
PROJECT_STATE="$TMP_PROJECT/.vida/data/state"
ARTIFACT_DIR="$TMP_ROOT/artifacts"
mkdir -p "$TMP_BIN" "$TMP_PROJECT" "$ARTIFACT_DIR"

cleanup() {
  if [[ "$KEEP_TEMP" != "1" && "$KEEP_TEMP" != "true" ]]; then
    rm -rf "$TMP_ROOT"
  else
    printf '[installer-smoke] kept temp root: %s\n' "$TMP_ROOT" >&2
  fi
}
trap cleanup EXIT

dump_file() {
  local path="$1"
  [[ -f "$path" ]] || return 0
  printf '\n--- %s ---\n' "$path"
  sed -n '1,200p' "$path"
}

dump_artifacts() {
  [[ -n "${ARTIFACT_DIR:-}" && -d "$ARTIFACT_DIR" ]] || return 0
  printf '\n[installer-smoke] artifact dir: %s\n' "$ARTIFACT_DIR"
  for path in "$ARTIFACT_DIR"/*; do
    dump_file "$path"
  done
}

run_step() {
  local name="$1"
  shift
  local stdout="$ARTIFACT_DIR/${name}.stdout"
  local stderr="$ARTIFACT_DIR/${name}.stderr"
  printf '[installer-smoke] %s\n' "$name"
  if ! "$@" >"$stdout" 2>"$stderr"; then
    printf '[installer-smoke] step failed: %s\n' "$name" >&2
    dump_file "$stdout" >&2
    dump_file "$stderr" >&2
    exit 1
  fi
}

assert_contains() {
  local name="$1"
  local path="$2"
  local pattern="$3"
  grep -q "$pattern" "$path" || fail "$name did not contain expected pattern: $pattern"
}

run_step install \
  bash "$ROOT_DIR/dist/vida-install.sh" install --archive "$ARCHIVE_PATH" --root "$INSTALL_ROOT" --bin-dir "$TMP_BIN" --force

RUNTIME_PATH="$INSTALL_ROOT/current/bin:$TMP_BIN:$PATH"
export VIDA_HOME="$INSTALL_ROOT"
export VIDA_ROOT="$INSTALL_ROOT/current"

run_step installer-doctor \
  bash "$ROOT_DIR/dist/vida-install.sh" doctor --root "$INSTALL_ROOT" --bin-dir "$TMP_BIN"
run_step vida-command env PATH="$RUNTIME_PATH" bash -c 'command -v vida'
run_step vida-help env PATH="$RUNTIME_PATH" vida --help
run_step taskflow-help env PATH="$RUNTIME_PATH" taskflow help
run_step docflow-help env PATH="$RUNTIME_PATH" docflow --help
run_step vida-pi-agent-help env PATH="$RUNTIME_PATH" vida-pi-agent --help
run_step vida-taskflow-help env PATH="$RUNTIME_PATH" vida taskflow help
run_step vida-docflow-help env PATH="$RUNTIME_PATH" vida docflow help
run_step docflow-init env PATH="$RUNTIME_PATH" docflow init
run_step docflow-init-json env PATH="$RUNTIME_PATH" docflow init --json

pushd "$TMP_PROJECT" >/dev/null
run_step vida-init env PATH="$RUNTIME_PATH" vida init
run_step vida-boot env PATH="$RUNTIME_PATH" vida boot --state-dir "$PROJECT_STATE"
run_step vida-doctor env PATH="$RUNTIME_PATH" vida doctor --state-dir "$PROJECT_STATE"
run_step vida-status-json env PATH="$RUNTIME_PATH" vida status --state-dir "$PROJECT_STATE" --json
run_step vida-doctor-json env PATH="$RUNTIME_PATH" vida doctor --state-dir "$PROJECT_STATE" --json
popd >/dev/null

assert_contains taskflow-help "$ARTIFACT_DIR/taskflow-help.stdout" "VIDA TaskFlow runtime family"
assert_contains docflow-help "$ARTIFACT_DIR/docflow-help.stdout" "Standalone DocFlow CLI"
assert_contains vida-pi-agent-help "$ARTIFACT_DIR/vida-pi-agent-help.stdout" "VIDA adapter for one-shot Pi RPC dispatch"
assert_contains docflow-init "$ARTIFACT_DIR/docflow-init.stdout" "mode: agent_bootstrap"
assert_contains docflow-init-json "$ARTIFACT_DIR/docflow-init-json.stdout" '"mode":"agent_bootstrap"'
assert_contains vida-command "$ARTIFACT_DIR/vida-command.stdout" "$INSTALL_ROOT/current/bin/vida"
assert_contains vida-doctor "$ARTIFACT_DIR/vida-doctor.stdout" "storage metadata: pass"
assert_contains vida-status-json "$ARTIFACT_DIR/vida-status-json.stdout" '"surface": "vida status"'

[[ -f "$TMP_PROJECT/AGENTS.md" ]] || fail "AGENTS.md was not bootstrapped"
[[ -f "$TMP_PROJECT/AGENTS.sidecar.md" ]] || fail "AGENTS.sidecar.md was not bootstrapped"
[[ -f "$TMP_PROJECT/vida.config.yaml" ]] || fail "vida.config.yaml was not bootstrapped"

printf '[installer-smoke] passed\n'
