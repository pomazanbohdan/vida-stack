#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-}"
RELEASE_SUFFIX="${VIDA_RELEASE_SUFFIX:-}"

fail() {
  printf '[release-build] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

infer_version() {
  local cargo_version
  cargo_version="$(awk -F'"' '/^version/ { print $2; exit }' "$ROOT_DIR/crates/vida/Cargo.toml")"
  [[ -n "$cargo_version" ]] || fail "Unable to infer version from crates/vida/Cargo.toml"
  printf 'v%s\n' "$cargo_version"
}

require_cmd python3
require_cmd cargo
require_cmd tar
require_cmd zip
require_cmd sha256sum

if [[ -z "$VERSION" ]]; then
  VERSION="$(infer_version)"
fi

ARCHIVE_BASE="vida-stack-${VERSION}"
if [[ -n "$RELEASE_SUFFIX" ]]; then
  ARCHIVE_BASE="${ARCHIVE_BASE}-${RELEASE_SUFFIX}"
fi
DIST_DIR="$ROOT_DIR/dist"
PACKAGE_ROOT="$DIST_DIR/package"
STAGE_DIR="$PACKAGE_ROOT/$ARCHIVE_BASE"
VIDA_BIN="$STAGE_DIR/bin/vida"
INSTALL_ASSETS_DIR="$STAGE_DIR/install/assets"
INSTALLER_ASSET="$DIST_DIR/vida-install.sh"
MANIFEST_OUT="$DIST_DIR/${ARCHIVE_BASE}.manifest.json"
RELEASE_NOTES_SRC="$ROOT_DIR/install/release-notes-${VERSION}.md"
RELEASE_NOTES_OUT="$DIST_DIR/release-notes.md"

rm -rf "$DIST_DIR"
mkdir -p "$STAGE_DIR/bin" "$INSTALL_ASSETS_DIR"

cp "$ROOT_DIR/AGENTS.md" "$STAGE_DIR/AGENTS.md"
awk '
  /^-----$/ { exit }
  { print }
' "$ROOT_DIR/install/assets/AGENTS.sidecar.scaffold.md" > "$STAGE_DIR/AGENTS.sidecar.md"
cp -R "$ROOT_DIR/.codex" "$STAGE_DIR/.codex"
cp -R "$ROOT_DIR/vida" "$STAGE_DIR/vida"

find "$STAGE_DIR" -type d -name '__pycache__' -prune -exec rm -rf {} +
find "$STAGE_DIR" -type f -name '*.pyc' -delete

cargo build --release -p vida
RUNTIME_SOURCE="$ROOT_DIR/target/release/vida"
if [[ -f "$ROOT_DIR/target/release/vida.exe" ]]; then
  RUNTIME_SOURCE="$ROOT_DIR/target/release/vida.exe"
fi
[[ -f "$RUNTIME_SOURCE" ]] || fail "Missing built runtime binary: $RUNTIME_SOURCE"
cp "$RUNTIME_SOURCE" "$VIDA_BIN"
chmod +x "$VIDA_BIN"
cp "$ROOT_DIR/docs/framework/templates/vida.config.yaml.template" "$INSTALL_ASSETS_DIR/vida.config.yaml.template"

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
        ".codex/",
        "bin/vida",
        "install/assets/",
        "vida/",
    ],
    "installed_entrypoints": [
        "vida",
        "vida docflow",
        "vida taskflow",
    ],
    "bundled_binaries": [
        "bin/vida",
    ],
    "installer_managed_runtimes": [
        "vida",
    ],
    "launcher_contracts": {
        "taskflow": "vida taskflow",
        "docflow": "vida docflow"
    },
    "installed_compatibility_contracts": {
        "vida docflow": "canonical docflow runtime",
        "vida taskflow": "canonical taskflow runtime"
    },
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
