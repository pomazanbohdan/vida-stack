#!/usr/bin/env bash
set -euo pipefail

REPO_SLUG="${VIDA_INSTALL_REPO:-pomazanbohdan/vida-stack}"
TARGET_DIR="$(pwd)"
COMMAND="${1:-help}"
VERSION="${VIDA_VERSION:-latest}"
ARCHIVE_FILE="${VIDA_ARCHIVE_FILE:-}"
FORCE="no"
DRY_RUN="no"

usage() {
  cat <<'EOF'
VIDA installer

Usage:
  bash install.sh <init|upgrade|doctor|help> [options]

Options:
  --dir PATH         Target directory. Defaults to current directory.
  --version TAG      Release tag to install. Defaults to latest.
  --force            Overwrite existing framework-owned files.
  --dry-run          Print planned actions without changing files.
  -h, --help         Show this help.

Examples:
  curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- init
  curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- upgrade --dry-run
  bash install/install.sh doctor
EOF
}

log() {
  printf '[vida-installer] %s\n' "$*"
}

fail() {
  printf '[vida-installer] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    init|upgrade|doctor|help)
      COMMAND="$1"
      shift
      ;;
    --dir)
      TARGET_DIR="${2:-}"
      shift 2
      ;;
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --force)
      FORCE="yes"
      shift
      ;;
    --dry-run)
      DRY_RUN="yes"
      shift
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

resolve_version() {
  if [[ "$VERSION" != "latest" ]]; then
    printf '%s\n' "$VERSION"
    return 0
  fi

  curl -fsSL "https://api.github.com/repos/${REPO_SLUG}/releases/latest" \
    | awk -F'"' '/"tag_name":/ { print $4; exit }'
}

download_release_archive() {
  local version="$1"
  local destination="$2"
  if [[ -n "$ARCHIVE_FILE" ]]; then
    log "Using local archive ${ARCHIVE_FILE}"
    if [[ "$DRY_RUN" == "yes" ]]; then
      return 0
    fi
    [[ -f "$ARCHIVE_FILE" ]] || fail "Local archive not found: ${ARCHIVE_FILE}"
    cp "$ARCHIVE_FILE" "$destination"
    return 0
  fi

  local url="https://github.com/${REPO_SLUG}/releases/download/${version}/vida-stack-${version}.tar.gz"
  log "Downloading ${url}"
  if [[ "$DRY_RUN" == "yes" ]]; then
    return 0
  fi
  curl -fsSL "$url" -o "$destination"
}

backup_existing() {
  local target="$1"
  local backup_root="$2"
  local rel_name="$3"

  if [[ ! -e "$target" ]]; then
    return 0
  fi

  mkdir -p "$backup_root"
  cp -R "$target" "${backup_root}/${rel_name}"
}

install_release() {
  local mode="$1"
  local version="$2"
  local temp_dir archive_path extract_dir backup_dir
  temp_dir="$(mktemp -d)"
  archive_path="${temp_dir}/vida-stack-${version}.tar.gz"
  extract_dir="${temp_dir}/extract"
  backup_dir="${TARGET_DIR}/.vida-backups/${version}"

  trap "rm -rf '$temp_dir'" RETURN

  download_release_archive "$version" "$archive_path"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would extract archive into temporary directory"
  else
    mkdir -p "$extract_dir"
    tar -xzf "$archive_path" -C "$extract_dir"
  fi

  local agents_target="${TARGET_DIR}/AGENTS.md"
  local vida_target="${TARGET_DIR}/_vida"
  local agents_source="${extract_dir}/AGENTS.md"
  local vida_source="${extract_dir}/_vida"

  if [[ -f "$agents_target" && "$mode" == "init" && "$FORCE" != "yes" ]]; then
    fail "AGENTS.md already exists. Re-run with --force or use upgrade."
  fi

  if [[ "$mode" == "init" && "$FORCE" != "yes" ]]; then
    [[ ! -e "$vida_target" ]] || fail "_vida already exists. Re-run with --force or use upgrade."
  fi

  if [[ -f "$agents_target" && "$DRY_RUN" == "no" ]]; then
    backup_existing "$agents_target" "$backup_dir" "AGENTS.md"
  fi

  if [[ "$mode" == "upgrade" && "$DRY_RUN" == "no" ]]; then
    backup_existing "$vida_target" "$backup_dir" "_vida"
  fi

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would install framework files into ${TARGET_DIR}"
    log "Would replace: AGENTS.md, _vida/"
    if [[ -f "$agents_target" || "$mode" == "upgrade" ]]; then
      log "Would write backup into ${backup_dir}"
    fi
    return 0
  fi

  [[ -f "$agents_source" ]] || fail "Downloaded archive does not contain AGENTS.md"
  [[ -d "$vida_source" ]] || fail "Downloaded archive does not contain _vida/"

  rm -rf "$agents_target" "$vida_target"
  cp "$agents_source" "$agents_target"
  cp -R "$vida_source" "$vida_target"

  log "Installed VIDA framework ${version} into ${TARGET_DIR}"
}

doctor() {
  local missing=0
  [[ -f "${TARGET_DIR}/AGENTS.md" ]] || { log "Missing AGENTS.md"; missing=1; }
  [[ -d "${TARGET_DIR}/_vida" ]] || { log "Missing _vida/"; missing=1; }
  [[ -d "${TARGET_DIR}/_vida/docs" ]] || { log "Missing _vida/docs/"; missing=1; }
  [[ -d "${TARGET_DIR}/_vida/scripts" ]] || { log "Missing _vida/scripts/"; missing=1; }

  if [[ "$missing" -eq 1 ]]; then
    fail "Doctor found missing framework files."
  fi

  log "Doctor check passed for ${TARGET_DIR}"
}

main() {
  require_cmd curl
  require_cmd tar
  require_cmd mktemp
  require_cmd awk
  require_cmd sed

  case "$COMMAND" in
    help)
      usage
      ;;
    doctor)
      doctor
      ;;
    init|upgrade)
      VERSION="$(resolve_version)"
      [[ -n "$VERSION" ]] || fail "Unable to resolve release version"
      install_release "$COMMAND" "$VERSION"
      ;;
    *)
      fail "Unsupported command: $COMMAND"
      ;;
  esac
}

main "$@"
