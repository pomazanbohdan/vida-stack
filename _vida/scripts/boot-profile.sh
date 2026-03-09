#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$ROOT_DIR"

vida_icon() {
  case "${1:-info}" in
    ok) printf '✅' ;;
    warn) printf '⚠️' ;;
    fail) printf '❌' ;;
    blocked) printf '⛔' ;;
    info) printf 'ℹ️' ;;
    sparkle) printf '✨' ;;
    progress) printf '🔄' ;;
    *) printf '•' ;;
  esac
}

vida_status_line() {
  local level="${1:-info}"
  shift || true
  printf '%s %s\n' "$(vida_icon "$level")" "$*"
}

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/boot-profile.sh run <lean|standard|full> [task_id] [--non-dev]
  bash _vida/scripts/boot-profile.sh verify-receipt <task_id|session> [profile]

Behavior:
  - Validates canonical read-set for selected boot profile.
  - Runs hydrate-minimal via context capsule when task_id is provided.
  - Writes machine-readable boot receipt under `.vida/logs/boot-receipts/`.
  - Writes a compact boot snapshot artifact for dev-oriented boots.
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
    if path.name.endswith(".boot-packet.json") or path.name.endswith(".boot-snapshot.json"):
        continue
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
  local boot_snapshot_file="$6"
  shift 6
  local read_contract=("$@")

  mkdir -p "$receipt_dir"

  local subject timestamp safe_subject latest_file archive_file packet_file packet_latest snapshot_file snapshot_latest
  subject="${task_id:-session}"
  safe_subject="${subject//[^A-Za-z0-9._-]/_}"
  timestamp="$(date -u +"%Y%m%dT%H%M%SZ")"
  latest_file="$receipt_dir/${safe_subject}.latest.json"
  archive_file="$receipt_dir/${safe_subject}-${timestamp}.json"
  packet_file="$receipt_dir/${safe_subject}-${timestamp}.boot-packet.json"
  packet_latest="$receipt_dir/${safe_subject}.latest.boot-packet.json"
  snapshot_file="$receipt_dir/${safe_subject}-${timestamp}.boot-snapshot.json"
  snapshot_latest="$receipt_dir/${safe_subject}.latest.boot-snapshot.json"

  if [[ "$non_dev" == "yes" ]]; then
    python3 _vida/scripts/boot-packet.py "$profile" --non-dev >"$packet_file"
  else
    python3 _vida/scripts/boot-packet.py "$profile" >"$packet_file"
  fi
  cp "$packet_file" "$packet_latest"

  if [[ -n "$boot_snapshot_file" && -f "$boot_snapshot_file" ]]; then
    cp "$boot_snapshot_file" "$snapshot_file"
    cp "$boot_snapshot_file" "$snapshot_latest"
    boot_snapshot_file="$snapshot_file"
  fi

  python - "$profile" "$task_id" "$non_dev" "$capsule_status" "$status" "$latest_file" "$archive_file" "$packet_file" "$boot_snapshot_file" "${read_contract[@]}" <<'PY'
import json
import hashlib
import sys
from datetime import datetime, timezone
from pathlib import Path

profile, task_id, non_dev, capsule_status, status, latest_file, archive_file, packet_file, boot_snapshot_file, *read_contract = sys.argv[1:]
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
    "boot_snapshot_file": boot_snapshot_file or None,
}
for target in (Path(latest_file), Path(archive_file)):
    target.write_text(json.dumps(payload, indent=2) + "\n")
PY
}

generate_boot_snapshot() {
  local profile="$1"
  local non_dev="$2"
  local snapshot_path="$3"

  if [[ "$non_dev" == "yes" ]]; then
    return 0
  fi

  python3 _vida/scripts/vida-boot-snapshot.py --json >"$snapshot_path"
}

run_checks() {
  local profile="$1"
  local task_id="$2"
  local non_dev="$3"

  local read_contract=()

  local entry f
  if [[ "$non_dev" == "yes" ]]; then
    mapfile -t read_contract < <(python3 _vida/scripts/boot-packet.py read-contract "$profile" --non-dev)
  else
    mapfile -t read_contract < <(python3 _vida/scripts/boot-packet.py read-contract "$profile")
  fi

  for entry in "${read_contract[@]}"; do
    f="${entry%%#*}"
    check_file "$f"
  done

  if [[ -n "$task_id" ]]; then
    if ! VIDA_ROOT="${VIDA_ROOT:-$ROOT_DIR}" VIDA_LEGACY_TURSO_PYTHON="${VIDA_LEGACY_TURSO_PYTHON:-$ROOT_DIR/.venv/bin/python3}" "$ROOT_DIR/_vida/scripts-nim/vida-legacy" context-capsule hydrate "$task_id" --json >/dev/null 2>&1; then
      write_receipt "$profile" "$task_id" "$non_dev" "missing" "blocked" "" "${read_contract[@]}"
      vida_status_line blocked "[boot-profile] BLK_CONTEXT_NOT_HYDRATED task=${task_id}" >&2
      return 1
    fi
  fi

  if [[ -f "vida.config.yaml" ]]; then
    python3 _vida/scripts/vida-config.py validate >/dev/null
    if python3 _vida/scripts/vida-config.py protocol-active agent_system >/dev/null 2>&1; then
      check_file "_vida/docs/subagent-system-protocol.md"
      check_file "_vida/scripts/vida-config.py"
      check_file "_vida/scripts/subagent-system.py"
      python3 _vida/scripts/subagent-system.py init "$task_id" >/dev/null
    fi
  fi

  local temp_snapshot
  temp_snapshot="$(mktemp)"
  if ! generate_boot_snapshot "$profile" "$non_dev" "$temp_snapshot"; then
    rm -f "$temp_snapshot"
    vida_status_line fail "[boot-profile] Failed to generate compact boot snapshot" >&2
    return 1
  fi

  write_receipt "$profile" "$task_id" "$non_dev" "present" "ok" "$temp_snapshot" "${read_contract[@]}"
  rm -f "$temp_snapshot"
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
boot_snapshot_file = payload.get("boot_snapshot_file")
if boot_snapshot_file:
    boot_snapshot_path = Path(boot_snapshot_file)
    if not boot_snapshot_path.exists():
        print(f"[boot-profile] Boot snapshot missing: {boot_snapshot_path}", file=sys.stderr)
        raise SystemExit(1)
print(
    f"✅ boot_receipt={path.name} subject={payload.get('subject')} profile={payload.get('profile')} status={payload.get('status')} contract_files={len(payload.get('contract_files') or [])} boot_packet={boot_packet_path.name} boot_snapshot={'present' if boot_snapshot_file else 'skipped'}"
)
PY

  python3 _vida/scripts/boot-packet.py summary "$subject" >/dev/null
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
