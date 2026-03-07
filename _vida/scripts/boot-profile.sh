#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/status-ui.sh"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/boot-profile.sh run <lean|standard|full> [task_id] [--non-dev]
  bash _vida/scripts/boot-profile.sh verify-receipt <task_id|session> [profile]

Behavior:
  - Validates canonical read-set for selected boot profile.
  - Runs hydrate-minimal via context capsule when task_id is provided.
  - Writes machine-readable boot receipt under `.vida/logs/boot-receipts/`.
  - Exits non-zero if required files are missing or hydration fails.
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[boot-profile] Missing command: $1" >&2
    exit 1
  fi
}

require_cmd jq
require_cmd python3

check_file() {
  local f="$1"
  if [[ ! -f "$f" ]]; then
    echo "[boot-profile] Missing required file: $f" >&2
    return 1
  fi
}

receipt_dir="$ROOT_DIR/.vida/logs/boot-receipts"

latest_receipt_path() {
  local subject="$1"
  local safe_subject latest_file
  safe_subject="${subject//[^A-Za-z0-9._-]/_}"
  latest_file="$receipt_dir/${safe_subject}.latest.json"

  python - "$receipt_dir" "$safe_subject" "$latest_file" <<'PY'
import json
import sys
from pathlib import Path

receipt_dir = Path(sys.argv[1])
safe_subject = sys.argv[2]
latest_file = Path(sys.argv[3])

candidates = []
if latest_file.exists():
    try:
        payload = json.loads(latest_file.read_text())
        candidates.append((payload.get("written_at") or "", latest_file))
    except Exception:
        pass

for path in receipt_dir.glob(f"{safe_subject}-*.json"):
    try:
        payload = json.loads(path.read_text())
        candidates.append((payload.get("written_at") or "", path))
    except Exception:
        continue

if not candidates:
    raise SystemExit(1)

candidates.sort(key=lambda item: item[0])
print(candidates[-1][1])
PY
}

write_receipt() {
  local profile="$1"
  local task_id="$2"
  local non_dev="$3"
  local capsule_status="$4"
  local status="$5"
  shift 5
  local read_contract=("$@")

  mkdir -p "$receipt_dir"

  local subject timestamp safe_subject latest_file archive_file packet_file packet_latest
  subject="${task_id:-session}"
  safe_subject="${subject//[^A-Za-z0-9._-]/_}"
  timestamp="$(date -u +"%Y%m%dT%H%M%SZ")"
  latest_file="$receipt_dir/${safe_subject}.latest.json"
  archive_file="$receipt_dir/${safe_subject}-${timestamp}.json"
  packet_file="$receipt_dir/${safe_subject}-${timestamp}.boot-packet.json"
  packet_latest="$receipt_dir/${safe_subject}.latest.boot-packet.json"

  if [[ "$non_dev" == "yes" ]]; then
    python3 _vida/scripts/boot-packet.py "$profile" --non-dev >"$packet_file"
  else
    python3 _vida/scripts/boot-packet.py "$profile" >"$packet_file"
  fi
  cp "$packet_file" "$packet_latest"

  python - "$profile" "$task_id" "$non_dev" "$capsule_status" "$status" "$latest_file" "$archive_file" "$packet_file" "${read_contract[@]}" <<'PY'
import json
import hashlib
import sys
from datetime import datetime, timezone
from pathlib import Path

profile, task_id, non_dev, capsule_status, status, latest_file, archive_file, packet_file, *read_contract = sys.argv[1:]
contract_files = []
for entry in read_contract:
    target = entry.split("#", 1)[0]
    path = Path(target)
    file_info = {"entry": entry, "path": target, "exists": path.exists()}
    if path.exists():
        file_info["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    contract_files.append(file_info)

payload = {
    "written_at": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
    "profile": profile,
    "task_id": task_id or None,
    "subject": task_id or "session",
    "non_dev": non_dev == "yes",
    "capsule_status": capsule_status,
    "status": status,
    "read_contract": read_contract,
    "contract_files": contract_files,
    "boot_packet_file": packet_file,
}
for target in (Path(latest_file), Path(archive_file)):
    target.write_text(json.dumps(payload, indent=2) + "\n")
PY
}

run_checks() {
  local profile="$1"
  local task_id="$2"
  local non_dev="$3"

  local common=(
    "AGENTS.md"
    "_vida/docs/thinking-protocol.md#section-algorithm-selector"
    "_vida/docs/thinking-protocol.md#section-stc"
    "_vida/docs/thinking-protocol.md#section-pr-cot"
    "_vida/docs/thinking-protocol.md#section-mar"
    "_vida/docs/thinking-protocol.md#section-5-solutions"
    "_vida/docs/thinking-protocol.md#section-meta-analysis"
    "_vida/docs/thinking-protocol.md#section-bug-reasoning"
    "_vida/docs/thinking-protocol.md#section-web-search"
    "_vida/docs/thinking-protocol.md#section-reasoning-modules"
    "_vida/docs/web-validation-protocol.md"
    "_vida/docs/beads-protocol.md"
    "_vida/docs/project-overlay-protocol.md"
  )

  local standard=(
    "_vida/docs/todo-protocol.md"
    "_vida/docs/implement-execution-protocol.md"
    "_vida/docs/use-case-packs.md"
  )

  local full=(
    "_vida/docs/orchestration-protocol.md"
    "_vida/docs/pipelines.md"
  )

  local read_contract=()

  local entry f
  for entry in "${common[@]}"; do
    f="${entry%%#*}"
    check_file "$f"
    read_contract+=("$entry")
  done

  if [[ "$profile" == "standard" || "$profile" == "full" ]]; then
    for entry in "${standard[@]}"; do
      f="${entry%%#*}"
      check_file "$f"
      read_contract+=("$entry")
    done
  fi

  if [[ "$profile" == "full" ]]; then
    for entry in "${full[@]}"; do
      f="${entry%%#*}"
      check_file "$f"
      read_contract+=("$entry")
    done
  fi

  if [[ "$non_dev" == "yes" ]]; then
    check_file "_vida/docs/spec-contract-protocol.md"
    read_contract+=("_vida/docs/spec-contract-protocol.md")
  fi

  if [[ -n "$task_id" ]]; then
    if ! bash _vida/scripts/context-capsule.sh hydrate "$task_id" >/dev/null 2>&1; then
      write_receipt "$profile" "$task_id" "$non_dev" "missing" "blocked" "${read_contract[@]}"
      vida_status_line blocked "[boot-profile] BLK_CONTEXT_NOT_HYDRATED task=${task_id}" >&2
      return 1
    fi
  fi

  if [[ -f "vida.config.yaml" ]]; then
    read_contract+=("vida.config.yaml")
    python3 _vida/scripts/vida-config.py validate >/dev/null
    if python3 _vida/scripts/vida-config.py protocol-active agent_system >/dev/null 2>&1; then
      check_file "_vida/docs/subagent-system-protocol.md"
      check_file "_vida/scripts/vida-config.py"
      check_file "_vida/scripts/subagent-system.py"
      read_contract+=("_vida/docs/subagent-system-protocol.md")
      read_contract+=("_vida/scripts/vida-config.py")
      read_contract+=("_vida/scripts/subagent-system.py")
      python3 _vida/scripts/subagent-system.py init "$task_id" >/dev/null
    fi
  fi

  write_receipt "$profile" "$task_id" "$non_dev" "present" "ok" "${read_contract[@]}"
  vida_status_line ok "boot_profile=${profile} task=${task_id:-none} non_dev=${non_dev} capsule=present status=ok"
}

verify_receipt() {
  local subject="$1"
  local expected_profile="${2:-}"
  local latest_file

  latest_file="$(latest_receipt_path "$subject")"

  if [[ ! -f "$latest_file" ]]; then
    vida_status_line fail "[boot-profile] Missing receipt: $latest_file" >&2
    exit 1
  fi

  python - "$latest_file" "$expected_profile" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
expected = sys.argv[2]
payload = json.loads(path.read_text())
if payload.get("status") != "ok":
    print(f"[boot-profile] Receipt status is not ok: {payload.get('status')}", file=sys.stderr)
    raise SystemExit(1)
if expected and payload.get("profile") != expected:
    print(
        f"[boot-profile] Receipt profile mismatch: expected={expected} actual={payload.get('profile')}",
        file=sys.stderr,
    )
    raise SystemExit(1)
boot_packet_file = payload.get("boot_packet_file")
if not boot_packet_file:
    print("[boot-profile] Receipt missing boot_packet_file", file=sys.stderr)
    raise SystemExit(1)
boot_packet_path = Path(boot_packet_file)
if not boot_packet_path.exists():
    print(f"[boot-profile] Boot packet missing: {boot_packet_path}", file=sys.stderr)
    raise SystemExit(1)
boot_packet = json.loads(boot_packet_path.read_text())
if boot_packet.get("profile") != payload.get("profile"):
    print(
        f"[boot-profile] Boot packet profile mismatch: receipt={payload.get('profile')} packet={boot_packet.get('profile')}",
        file=sys.stderr,
    )
    raise SystemExit(1)
print(
    f"✅ boot_receipt={path.name} subject={payload.get('subject')} profile={payload.get('profile')} status={payload.get('status')} contract_files={len(payload.get('contract_files') or [])} boot_packet={boot_packet_path.name}"
)
PY
}

cmd="${1:-}"
if [[ "$cmd" != "run" && "$cmd" != "verify-receipt" ]]; then
  usage
  exit 1
fi

if [[ "$cmd" == "verify-receipt" ]]; then
  subject="${2:-}"
  profile="${3:-}"
  [[ -n "$subject" ]] || { usage; exit 1; }
  verify_receipt "$subject" "$profile"
  exit 0
fi

profile="${2:-}"
task_id="${3:-}"
non_dev="no"

if [[ -n "${4:-}" ]]; then
  if [[ "$4" == "--non-dev" ]]; then
    non_dev="yes"
  else
    echo "[boot-profile] Unknown argument: $4" >&2
    usage
    exit 1
  fi
fi

case "$profile" in
  lean|standard|full)
    run_checks "$profile" "$task_id" "$non_dev"
    ;;
  *)
    usage
    exit 1
    ;;
esac
