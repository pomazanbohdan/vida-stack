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

python3 - <<PY
import tarfile
import zipfile
from pathlib import Path

package_root = Path(${PACKAGE_ROOT@Q})
archive_base = ${ARCHIVE_BASE@Q}
dist_dir = Path(${DIST_DIR@Q})
source_dir = package_root / archive_base
zip_path = dist_dir / f"{archive_base}.zip"
tar_path = dist_dir / f"{archive_base}.tar.gz"

with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in sorted(source_dir.rglob("*")):
        if path.is_file():
            zf.write(path, path.relative_to(package_root).as_posix())

with tarfile.open(tar_path, "w:gz") as tf:
    tf.add(source_dir, arcname=archive_base)
PY

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

python3 - <<PY
import hashlib
from pathlib import Path

dist_dir = Path(${DIST_DIR@Q})
archive_base = ${ARCHIVE_BASE@Q}
files = [
    dist_dir / f"{archive_base}.tar.gz",
    dist_dir / f"{archive_base}.zip",
    dist_dir / Path(${INSTALLER_ASSET@Q}).name,
]
out = dist_dir / f"{archive_base}.sha256"

lines = []
for path in files:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    lines.append(f"{digest}  {path.name}")
out.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

printf '[release-build] Built %s\n' "$ARCHIVE_BASE"
printf '[release-build] Assets:\n'
printf '  - %s\n' "$DIST_DIR/${ARCHIVE_BASE}.tar.gz"
printf '  - %s\n' "$DIST_DIR/${ARCHIVE_BASE}.zip"
printf '  - %s\n' "$INSTALLER_ASSET"
printf '  - %s\n' "$DIST_DIR/${ARCHIVE_BASE}.sha256"
