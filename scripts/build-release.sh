#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-}"

fail() {
  printf '[release-build] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

infer_version() {
  local nimble_version
  nimble_version="$(awk -F'"' '/^version/ { print $2; exit }' "$ROOT_DIR/taskflow-v0/vida.nimble")"
  [[ -n "$nimble_version" ]] || fail "Unable to infer version from taskflow-v0/vida.nimble"
  printf 'v%s\n' "$nimble_version"
}

require_cmd python3
require_cmd nim
require_cmd tar
require_cmd zip
require_cmd sha256sum

if [[ -z "$VERSION" ]]; then
  VERSION="$(infer_version)"
fi

ARCHIVE_BASE="vida-stack-${VERSION}"
DIST_DIR="$ROOT_DIR/dist"
PACKAGE_ROOT="$DIST_DIR/package"
STAGE_DIR="$PACKAGE_ROOT/$ARCHIVE_BASE"
TASKFLOW_BIN="$STAGE_DIR/bin/taskflow-v0"
TASKFLOW_HELPERS_DIR="$STAGE_DIR/taskflow-v0/helpers"
INSTALLER_ASSET="$DIST_DIR/vida-install.sh"
MANIFEST_OUT="$DIST_DIR/${ARCHIVE_BASE}.manifest.json"
RELEASE_NOTES_SRC="$ROOT_DIR/install/release-notes-${VERSION}.md"
RELEASE_NOTES_OUT="$DIST_DIR/release-notes.md"
NIMCACHE_DIR="$DIST_DIR/nimcache/release"

rm -rf "$DIST_DIR"
mkdir -p "$STAGE_DIR/bin" "$TASKFLOW_HELPERS_DIR"

cp "$ROOT_DIR/AGENTS.md" "$STAGE_DIR/AGENTS.md"
awk '
  /^-----$/ { exit }
  { print }
' "$ROOT_DIR/install/assets/AGENTS.sidecar.scaffold.md" > "$STAGE_DIR/AGENTS.sidecar.md"
cp -R "$ROOT_DIR/vida" "$STAGE_DIR/vida"
cp -R "$ROOT_DIR/codex-v0" "$STAGE_DIR/codex-v0"

find "$STAGE_DIR" -type d -name '__pycache__' -prune -exec rm -rf {} +
find "$STAGE_DIR" -type f -name '*.pyc' -delete

nim c -d:release --nimcache:"$NIMCACHE_DIR" -o:"$TASKFLOW_BIN" "$ROOT_DIR/taskflow-v0/src/vida.nim"
chmod +x "$TASKFLOW_BIN"
cp "$ROOT_DIR/taskflow-v0/helpers/turso_task_store.py" "$TASKFLOW_HELPERS_DIR/turso_task_store.py"
cp "$ROOT_DIR/taskflow-v0/helpers/toon_render.py" "$TASKFLOW_HELPERS_DIR/toon_render.py"

python3 - <<PY
import json
from datetime import datetime, timezone
from pathlib import Path

manifest_path = Path(${MANIFEST_OUT@Q})
manifest = {
    "artifact_name": ${ARCHIVE_BASE@Q},
    "version": ${VERSION@Q},
    "built_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat(),
    "package_root": ${ARCHIVE_BASE@Q},
    "included_roots": [
        "AGENTS.md",
        "AGENTS.sidecar.md",
        "bin/taskflow-v0",
        "codex-v0/",
        "taskflow-v0/helpers/",
        "vida/",
    ],
    "installed_entrypoints": [
        "vida",
        "taskflow-v0",
        "codex-v0",
    ],
    "bundled_binaries": [
        "bin/taskflow-v0",
    ],
    "installer_managed_runtimes": [
        "taskflow-v0",
        "codex-v0",
    ],
}
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
PY

(
  cd "$PACKAGE_ROOT"
  zip -qr "../${ARCHIVE_BASE}.zip" "$ARCHIVE_BASE"
)
tar -czf "$DIST_DIR/${ARCHIVE_BASE}.tar.gz" -C "$PACKAGE_ROOT" "$ARCHIVE_BASE"

cp "$ROOT_DIR/install/install.sh" "$INSTALLER_ASSET"
chmod +x "$INSTALLER_ASSET"

if [[ -f "$RELEASE_NOTES_SRC" ]]; then
  cp "$RELEASE_NOTES_SRC" "$RELEASE_NOTES_OUT"
else
  awk '
    BEGIN { capture=0 }
    /^## / { if (capture) exit; capture=1 }
    capture { print }
  ' "$ROOT_DIR/README.md" > "$RELEASE_NOTES_OUT"
fi

(
  cd "$DIST_DIR"
  sha256sum "${ARCHIVE_BASE}.tar.gz" "${ARCHIVE_BASE}.zip" "$(basename "$INSTALLER_ASSET")" > "${ARCHIVE_BASE}.sha256"
)

printf '[release-build] Built %s\n' "$ARCHIVE_BASE"
printf '[release-build] Assets:\n'
printf '  - %s\n' "$DIST_DIR/${ARCHIVE_BASE}.tar.gz"
printf '  - %s\n' "$DIST_DIR/${ARCHIVE_BASE}.zip"
printf '  - %s\n' "$INSTALLER_ASSET"
printf '  - %s\n' "$DIST_DIR/${ARCHIVE_BASE}.sha256"
