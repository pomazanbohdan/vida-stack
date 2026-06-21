#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-}"
RELEASE_SUFFIX="${VIDA_RELEASE_SUFFIX:-}"
WINDOWS_RELEASE="no"
SKIP_BUILD="${VIDA_RELEASE_SKIP_BUILD:-0}"
CARGO_BIN="${CARGO:-cargo}"

fail() {
  printf '[release-build] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

skip_build_enabled() {
  case "$SKIP_BUILD" in
    1|true|TRUE|yes|YES) return 0 ;;
    *) return 1 ;;
  esac
}

normalize_path_for_python() {
  local raw="$1"
  local drive=""
  local rest=""
  local drive_upper=""

  case "${OSTYPE:-}" in
    msys*|cygwin*|win32*)
      if [[ "$raw" =~ ^/([A-Za-z])(/.*)$ ]]; then
        drive="${BASH_REMATCH[1]}"
        rest="${BASH_REMATCH[2]}"
        drive_upper="$(printf '%s' "$drive" | tr '[:lower:]' '[:upper:]')"
        printf '%s:%s\n' "$drive_upper" "$rest"
        return 0
      fi
      ;;
  esac

  printf '%s\n' "$raw"
}

infer_version() {
  local cargo_version
  cargo_version="$(awk -F'"' '/^version/ { print $2; exit }' "$ROOT_DIR/crates/vida/Cargo.toml")"
  [[ -n "$cargo_version" ]] || fail "Unable to infer version from crates/vida/Cargo.toml"
  printf 'v%s\n' "$cargo_version"
}

infer_cargo_host_triple() {
  "$CARGO_BIN" -vV | awk '/^host:/ { print $2; exit }'
}

select_python() {
  local candidate
  for candidate in python3 python; do
    if command -v "$candidate" >/dev/null 2>&1 && "$candidate" --version >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  fail "Missing working Python command: tried python3, python"
}

if ! skip_build_enabled; then
  require_cmd "$CARGO_BIN"
fi
PYTHON_BIN="$(select_python)"

if [[ -z "$VERSION" ]]; then
  VERSION="$(infer_version)"
fi

if [[ -z "$RELEASE_SUFFIX" ]]; then
  if command -v "$CARGO_BIN" >/dev/null 2>&1; then
    CARGO_HOST_TRIPLE="$(infer_cargo_host_triple || true)"
    if [[ "$CARGO_HOST_TRIPLE" == *windows* ]]; then
      RELEASE_SUFFIX="windows-x86_64"
    fi
  else
    case "${OSTYPE:-}" in
      msys*|cygwin*|win32*) RELEASE_SUFFIX="windows-x86_64" ;;
    esac
  fi
fi

ARCHIVE_BASE="vida-stack-${VERSION}"
if [[ -n "$RELEASE_SUFFIX" ]]; then
  ARCHIVE_BASE="${ARCHIVE_BASE}-${RELEASE_SUFFIX}"
fi
if [[ "$RELEASE_SUFFIX" == "windows-x86_64" ]]; then
  WINDOWS_RELEASE="yes"
fi
DIST_DIR="$ROOT_DIR/dist"
PACKAGE_ROOT="$DIST_DIR/package"
STAGE_DIR="$PACKAGE_ROOT/$ARCHIVE_BASE"
CARGO_TARGET_ROOT="${CARGO_TARGET_DIR:-$ROOT_DIR/.vida/cargo-target}"
RELEASE_BIN_DIR="${VIDA_RELEASE_BIN_DIR:-$CARGO_TARGET_ROOT/release}"
if [[ "$WINDOWS_RELEASE" == "yes" ]]; then
  VIDA_BIN="$STAGE_DIR/bin/vida.exe"
  TASKFLOW_BIN="$STAGE_DIR/bin/taskflow.exe"
  DOCFLOW_BIN="$STAGE_DIR/bin/docflow.exe"
  PI_AGENT_BIN="$STAGE_DIR/bin/vida-pi-agent.exe"
  VIDA_CODER_BIN="$STAGE_DIR/bin/vida-coder.exe"
else
  VIDA_BIN="$STAGE_DIR/bin/vida"
  TASKFLOW_BIN="$STAGE_DIR/bin/taskflow"
  DOCFLOW_BIN="$STAGE_DIR/bin/docflow"
  PI_AGENT_BIN="$STAGE_DIR/bin/vida-pi-agent"
  VIDA_CODER_BIN="$STAGE_DIR/bin/vida-coder"
fi
INSTALL_ASSETS_DIR="$STAGE_DIR/install/assets"
INSTALLER_ASSET="$DIST_DIR/vida-install.sh"
WINDOWS_INSTALLER_ASSET="$DIST_DIR/vida-install.ps1"
MANIFEST_OUT="$DIST_DIR/${ARCHIVE_BASE}.manifest.json"
RELEASE_NOTES_SRC="$ROOT_DIR/install/release-notes-${VERSION}.md"
RELEASE_NOTES_OUT="$DIST_DIR/release-notes.md"

rm -rf "$DIST_DIR"
mkdir -p "$STAGE_DIR/bin" "$INSTALL_ASSETS_DIR"

cp "$ROOT_DIR/AGENTS.md" "$STAGE_DIR/AGENTS.md"
awk '
  /^-----$/ { exit }
  { print }
' "$ROOT_DIR/AGENTS.sidecar.md" > "$STAGE_DIR/AGENTS.sidecar.md"
cp -R "$ROOT_DIR/.codex" "$STAGE_DIR/.codex"
for host_template in .qwen .kilo .opencode; do
  if [[ -d "$ROOT_DIR/$host_template" ]]; then
    cp -R "$ROOT_DIR/$host_template" "$STAGE_DIR/$host_template"
  fi
done
cp -R "$ROOT_DIR/vida" "$STAGE_DIR/vida"

find "$STAGE_DIR" -type d -name '__pycache__' -prune -exec rm -rf {} +
find "$STAGE_DIR" -type f -name '*.pyc' -delete

if skip_build_enabled; then
    printf '[release-build] Using existing release binaries from %s\n' "$RELEASE_BIN_DIR"
else
    "$CARGO_BIN" build --release -p vida -p taskflow-cli -p docflow-cli -p vida-pi-agent -p vida-coder
fi
copy_runtime_binary() {
  local binary_name="$1"
  local destination="$2"
  local source="$RELEASE_BIN_DIR/$binary_name"
  if [[ "$WINDOWS_RELEASE" == "yes" ]]; then
    source="$RELEASE_BIN_DIR/${binary_name}.exe"
  fi
  [[ -f "$source" ]] || fail "Missing built runtime binary for release target ${RELEASE_SUFFIX:-default}: $source"
  cp "$source" "$destination"
  chmod +x "$destination"
  if [[ -f "${source}.version" ]]; then
    cp "${source}.version" "${destination}.version"
  fi
}

verify_runtime_binary_version() {
  local binary_label="$1"
  local binary_path="$2"
  local expected_version="${VERSION#v}"
  local actual
  [[ -x "$binary_path" || -f "$binary_path" ]] || fail "Missing packaged runtime binary: $binary_path"
  actual="$("$binary_path" --version 2>/dev/null | head -n 1 | tr -d '\r' || true)"
  if [[ -z "$actual" && "$WINDOWS_RELEASE" == "yes" && -f "${binary_path}.version" ]]; then
    actual="$(head -n 1 "${binary_path}.version" | tr -d '\r')"
  fi
  case "$actual" in
    "$binary_label $expected_version"|"$binary_label $expected_version (built "*) ;;
    *)
      fail "Packaged $binary_label version mismatch: expected '$binary_label $expected_version' with optional build timestamp, got '${actual:-<no output>}' from $binary_path"
      ;;
  esac
}

runtime_binary_version_line() {
  local binary_path="$1"
  local actual
  actual="$("$binary_path" --version 2>/dev/null | head -n 1 | tr -d '\r' || true)"
  if [[ -z "$actual" && "$WINDOWS_RELEASE" == "yes" && -f "${binary_path}.version" ]]; then
    actual="$(head -n 1 "${binary_path}.version" | tr -d '\r')"
  fi
  printf '%s\n' "$actual"
}

copy_runtime_binary vida "$VIDA_BIN"
copy_runtime_binary taskflow "$TASKFLOW_BIN"
copy_runtime_binary docflow "$DOCFLOW_BIN"
copy_runtime_binary vida-pi-agent "$PI_AGENT_BIN"
copy_runtime_binary vida-coder "$VIDA_CODER_BIN"
verify_runtime_binary_version vida "$VIDA_BIN"
verify_runtime_binary_version taskflow "$TASKFLOW_BIN"
verify_runtime_binary_version docflow "$DOCFLOW_BIN"
verify_runtime_binary_version vida-coder "$VIDA_CODER_BIN"
VIDA_VERSION_LINE="$(runtime_binary_version_line "$VIDA_BIN")"
TASKFLOW_VERSION_LINE="$(runtime_binary_version_line "$TASKFLOW_BIN")"
DOCFLOW_VERSION_LINE="$(runtime_binary_version_line "$DOCFLOW_BIN")"
VIDA_CODER_VERSION_LINE="$(runtime_binary_version_line "$VIDA_CODER_BIN")"
"$PI_AGENT_BIN" --help >/dev/null 2>&1 || fail "Packaged vida-pi-agent help check failed: $PI_AGENT_BIN"
"$VIDA_CODER_BIN" provider-check --json >/dev/null 2>&1 || fail "Packaged vida-coder provider readiness check failed: $VIDA_CODER_BIN"
rm -f "${VIDA_BIN}.version" "${TASKFLOW_BIN}.version" "${DOCFLOW_BIN}.version" "${PI_AGENT_BIN}.version" "${VIDA_CODER_BIN}.version"
cp "$ROOT_DIR/docs/framework/templates/vida.config.yaml.template" "$INSTALL_ASSETS_DIR/vida.config.yaml.template"
cp "$ROOT_DIR/docs/product/spec/templates/feature-design-document.template.md" "$INSTALL_ASSETS_DIR/feature-design-document.template.md"

PY_MANIFEST_OUT="$(normalize_path_for_python "$MANIFEST_OUT")"
PY_PACKAGE_ROOT="$(normalize_path_for_python "$PACKAGE_ROOT")"
PY_DIST_DIR="$(normalize_path_for_python "$DIST_DIR")"
PY_INSTALLER_ASSET="$(normalize_path_for_python "$INSTALLER_ASSET")"
PY_WINDOWS_INSTALLER_ASSET="$(normalize_path_for_python "$WINDOWS_INSTALLER_ASSET")"

MANIFEST_OUT="$PY_MANIFEST_OUT" ARCHIVE_BASE="$ARCHIVE_BASE" VERSION="$VERSION" WINDOWS_RELEASE="$WINDOWS_RELEASE" VIDA_VERSION_LINE="$VIDA_VERSION_LINE" TASKFLOW_VERSION_LINE="$TASKFLOW_VERSION_LINE" DOCFLOW_VERSION_LINE="$DOCFLOW_VERSION_LINE" VIDA_CODER_VERSION_LINE="$VIDA_CODER_VERSION_LINE" "$PYTHON_BIN" - <<'PY'
import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path

manifest_path = Path(os.environ["MANIFEST_OUT"])
archive_base = os.environ["ARCHIVE_BASE"]
version = os.environ["VERSION"]
expected_version = version.removeprefix("v")
windows_release = os.environ["WINDOWS_RELEASE"] == "yes"
binary_roots = ["bin/vida.exe", "bin/taskflow.exe", "bin/docflow.exe", "bin/vida-pi-agent.exe", "bin/vida-coder.exe"] if windows_release else ["bin/vida", "bin/taskflow", "bin/docflow", "bin/vida-pi-agent", "bin/vida-coder"]

def binary_version_record(label: str, path: str, line: str) -> dict:
    match = re.fullmatch(rf"{re.escape(label)}\s+(\S+)(?:\s+\(built\s+([^)]+)\))?", line)
    return {
        "path": path,
        "version_line": line,
        "expected_version": expected_version,
        "matches_expected_version": bool(match and match.group(1) == expected_version),
        "build_timestamp": match.group(2) if match else None,
    }

manifest = {
    "artifact_name": archive_base,
    "version": version,
    "built_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat(),
    "package_root": archive_base,
    "included_roots": [
        "AGENTS.md",
        "AGENTS.sidecar.md",
        ".codex/",
        ".qwen/",
        ".kilo/",
        ".opencode/",
        *binary_roots,
        "install/assets/",
        "vida/",
    ],
    "installed_entrypoints": [
        "vida",
        "taskflow",
        "docflow",
        "vida-pi-agent",
        "vida-coder",
        "vida docflow",
        "vida taskflow",
    ],
    "bundled_binaries": binary_roots,
    "binary_versions": {
        "vida": binary_version_record("vida", binary_roots[0], os.environ["VIDA_VERSION_LINE"]),
        "taskflow": binary_version_record("taskflow", binary_roots[1], os.environ["TASKFLOW_VERSION_LINE"]),
        "docflow": binary_version_record("docflow", binary_roots[2], os.environ["DOCFLOW_VERSION_LINE"]),
        "vida-coder": binary_version_record("vida-coder", binary_roots[4], os.environ["VIDA_CODER_VERSION_LINE"]),
    },
    "installer_managed_runtimes": [
        "vida",
        "taskflow",
        "docflow",
        "vida-pi-agent",
        "vida-coder",
    ],
    "launcher_contracts": {
        "taskflow": "vida taskflow",
        "docflow": "vida docflow"
    },
    "installed_compatibility_contracts": {
        "taskflow": "canonical taskflow runtime",
        "docflow": "canonical docflow runtime",
        "vida docflow": "canonical docflow runtime",
        "vida taskflow": "canonical taskflow runtime"
    },
}
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
PY

PACKAGE_ROOT="$PY_PACKAGE_ROOT" ARCHIVE_BASE="$ARCHIVE_BASE" DIST_DIR="$PY_DIST_DIR" "$PYTHON_BIN" - <<'PY'
import tarfile
import zipfile
import os
from pathlib import Path

package_root = Path(os.environ["PACKAGE_ROOT"])
archive_base = os.environ["ARCHIVE_BASE"]
dist_dir = Path(os.environ["DIST_DIR"])
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
cp "$ROOT_DIR/install/install.ps1" "$WINDOWS_INSTALLER_ASSET"

if [[ -f "$RELEASE_NOTES_SRC" ]]; then
  cp "$RELEASE_NOTES_SRC" "$RELEASE_NOTES_OUT"
else
  awk '
    BEGIN { capture=0 }
    /^## / { if (capture) exit; capture=1 }
    capture { print }
  ' "$ROOT_DIR/README.md" > "$RELEASE_NOTES_OUT"
fi

DIST_DIR="$PY_DIST_DIR" ARCHIVE_BASE="$ARCHIVE_BASE" INSTALLER_ASSET="$PY_INSTALLER_ASSET" WINDOWS_INSTALLER_ASSET="$PY_WINDOWS_INSTALLER_ASSET" "$PYTHON_BIN" - <<'PY'
import hashlib
import os
from pathlib import Path

dist_dir = Path(os.environ["DIST_DIR"])
archive_base = os.environ["ARCHIVE_BASE"]
installer_asset = os.environ["INSTALLER_ASSET"]
windows_installer_asset = os.environ["WINDOWS_INSTALLER_ASSET"]
files = [
    dist_dir / f"{archive_base}.tar.gz",
    dist_dir / f"{archive_base}.zip",
    dist_dir / Path(installer_asset).name,
    dist_dir / Path(windows_installer_asset).name,
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
printf '  - %s\n' "$WINDOWS_INSTALLER_ASSET"
printf '  - %s\n' "$DIST_DIR/${ARCHIVE_BASE}.sha256"
