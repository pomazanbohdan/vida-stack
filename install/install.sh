#!/usr/bin/env bash
set -euo pipefail

REPO_SLUG="${VIDA_INSTALL_REPO:-pomazanbohdan/vida-stack}"
COMMAND="${1:-help}"
VERSION="${VIDA_VERSION:-latest}"
ARCHIVE_FILE="${VIDA_ARCHIVE_FILE:-}"
INSTALL_ROOT="${VIDA_HOME:-$HOME/.local/share/vida-stack}"
BIN_DIR="${VIDA_BIN_DIR:-$HOME/.local/bin}"
FORCE="no"
DRY_RUN="no"
KEEP_RELEASES="${VIDA_KEEP_RELEASES:-3}"

usage() {
  cat <<'EOF'
VIDA installer

Usage:
  bash install.sh <install|init|upgrade|use|doctor|help> [options]

Options:
  --version TAG      Release tag to install. Defaults to latest.
  --archive PATH     Local release archive instead of GitHub download.
  --bin-dir PATH     Directory for launcher scripts. Defaults to ~/.local/bin.
  --root PATH        Install root. Defaults to ~/.local/share/vida-stack.
  --force            Overwrite an already installed release of the same version.
  --dry-run          Print planned actions without changing files.
  -h, --help         Show this help.

Examples:
  curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- install
  curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- upgrade --version v0.2.2
  bash install/install.sh use --version v0.2.2
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

checksum_cmd() {
  if command -v sha256sum >/dev/null 2>&1; then
    printf 'sha256sum\n'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    printf 'shasum -a 256\n'
    return 0
  fi
  fail "Missing required checksum command: sha256sum or shasum"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    install|init|upgrade|use|doctor|help)
      COMMAND="$1"
      shift
      ;;
    --root)
      INSTALL_ROOT="${2:-}"
      shift 2
      ;;
    --bin-dir)
      BIN_DIR="${2:-}"
      shift 2
      ;;
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --archive)
      ARCHIVE_FILE="${2:-}"
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

  if [[ -n "$ARCHIVE_FILE" ]]; then
    local archive_name
    archive_name="$(basename "$ARCHIVE_FILE")"
    if [[ "$archive_name" =~ ^vida-stack-(.+)\.tar\.gz$ ]]; then
      printf '%s\n' "${BASH_REMATCH[1]}"
      return 0
    fi
    fail "Unable to infer version from local archive name: ${archive_name}"
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

download_release_checksum() {
  local version="$1"
  local destination="$2"
  if [[ -n "$ARCHIVE_FILE" ]]; then
    log "Skipping checksum download for local archive"
    return 0
  fi

  local url="https://github.com/${REPO_SLUG}/releases/download/${version}/vida-stack-${version}.sha256"
  log "Downloading ${url}"
  if [[ "$DRY_RUN" == "yes" ]]; then
    return 0
  fi
  curl -fsSL "$url" -o "$destination"
}

verify_archive_checksum() {
  local archive_path="$1"
  local checksum_path="$2"
  [[ -f "$checksum_path" ]] || return 0

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would verify checksum for $(basename "$archive_path")"
    return 0
  fi

  local cmd
  cmd="$(checksum_cmd)"
  (
    cd "$(dirname "$archive_path")"
    grep " $(basename "$archive_path")\$" "$checksum_path" > .vida-check.tmp
    [[ -s .vida-check.tmp ]] || fail "Checksum file does not contain entry for $(basename "$archive_path")"
    if [[ "$cmd" == "sha256sum" ]]; then
      sha256sum -c .vida-check.tmp
    else
      shasum -a 256 -c .vida-check.tmp
    fi
    rm -f .vida-check.tmp
  )
}

append_source_line() {
  local file="$1"
  local line="$2"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  if grep -Fq "$line" "$file"; then
    return 0
  fi
  printf '\n%s\n' "$line" >> "$file"
}

write_env_file() {
  local env_file="$1"
  mkdir -p "$(dirname "$env_file")"
  cat > "$env_file" <<EOF
export VIDA_HOME="\${VIDA_HOME:-$INSTALL_ROOT}"
export VIDA_ROOT="\${VIDA_ROOT:-\$VIDA_HOME/current}"
case ":\$PATH:" in
  *:"$BIN_DIR":*) ;;
  *) export PATH="$BIN_DIR:\$PATH" ;;
esac
EOF
}

install_profile_hooks() {
  local env_file="$1"
  local source_line="[[ -f \"$env_file\" ]] && source \"$env_file\""
  append_source_line "$HOME/.bashrc" "$source_line"
  append_source_line "$HOME/.zshrc" "$source_line"
  if [[ -f "$HOME/.bash_profile" ]]; then
    append_source_line "$HOME/.bash_profile" "$source_line"
  fi
  if [[ -f "$HOME/.zprofile" ]]; then
    append_source_line "$HOME/.zprofile" "$source_line"
  fi
}

write_wrapper() {
  local path="$1"
  local body="$2"
  mkdir -p "$(dirname "$path")"
  cat > "$path" <<EOF
#!/usr/bin/env bash
set -euo pipefail
$body
EOF
  chmod +x "$path"
}

install_wrappers() {
  write_wrapper "$BIN_DIR/taskflow-v0" '
VIDA_HOME="${VIDA_HOME:-'"$INSTALL_ROOT"'}"
VIDA_ROOT="${VIDA_ROOT:-$VIDA_HOME/current}"
exec "$VIDA_ROOT/bin/taskflow-v0" "$@"
'
  write_wrapper "$BIN_DIR/codex-v0" '
VIDA_HOME="${VIDA_HOME:-'"$INSTALL_ROOT"'}"
VIDA_ROOT="${VIDA_ROOT:-$VIDA_HOME/current}"
if [[ "${1:-}" == "help" || "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'\''USAGE'\''
Codex v0 compatibility wrapper

Usage:
  codex-v0 help
  codex-v0 overview [args...]

Notes:
  - `vida docflow` is the canonical installed launcher contract
  - installed `codex-v0` is migration-only compatibility
  - installed-mode compatibility remains `help|overview only`
USAGE
  exit 0
fi
exec "'"$BIN_DIR"'/vida" docflow "$@"
'
  write_wrapper "$BIN_DIR/vida" '
VIDA_HOME="${VIDA_HOME:-'"$INSTALL_ROOT"'}"
VIDA_ROOT="${VIDA_ROOT:-$VIDA_HOME/current}"
usage() {
  cat <<'\''USAGE'\''
VIDA launcher

Usage:
  vida taskflow <args...>
  vida docflow <args...>
  vida doctor
  vida upgrade [--version TAG]
  vida use --version TAG
  vida root
USAGE
}

docflow_usage() {
  cat <<'\''USAGE'\''
VIDA DocFlow compatibility bridge

Usage:
  vida docflow help
  vida docflow overview [args...]

Notes:
  - installed-mode `vida docflow` compatibility contract is `help|overview only`
  - the bundled donor runtime remains `codex-v0/codex.py`
  - unsupported commands fail closed instead of passing through raw donor args
USAGE
}

sub="${1:-help}"
case "$sub" in
  taskflow)
    shift
    exec "'"$BIN_DIR"'/taskflow-v0" "$@"
    ;;
  docflow)
    shift
    docflow_sub="${1:-help}"
    case "$docflow_sub" in
      help|--help|-h)
        docflow_usage
        ;;
      overview)
        exec "$VIDA_ROOT/.venv/bin/python3" "$VIDA_ROOT/codex-v0/codex.py" "$@"
        ;;
      *)
        printf "vida docflow: unsupported installed-mode command: %s\n\n" "$docflow_sub" >&2
        docflow_usage >&2
        exit 1
        ;;
    esac
    ;;
  doctor|upgrade|install|use)
    exec "$VIDA_HOME/installer/install.sh" "$sub" --root "$VIDA_HOME" --bin-dir "'"$BIN_DIR"'" "${@:2}"
    ;;
  root)
    printf "%s\n" "$VIDA_ROOT"
    ;;
  help|--help|-h)
    usage
    ;;
  *)
    usage
    exit 1
    ;;
esac
'
}

prepare_python_env() {
  local release_root="$1"
  local venv_dir="$release_root/.venv"
  local requirements="$release_root/codex-v0/requirements-python.txt"
  if [[ ! -f "$requirements" ]]; then
    requirements="$release_root/install/requirements-python.txt"
  fi
  [[ -f "$requirements" ]] || fail "Missing runtime requirements: $requirements"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would create Python venv in ${venv_dir}"
    log "Would install Python requirements from ${requirements}"
    return 0
  fi

  python3 -m venv "$venv_dir"
  "$venv_dir/bin/python3" -m ensurepip --upgrade >/dev/null 2>&1 || true
  "$venv_dir/bin/python3" -m pip install --upgrade pip
  "$venv_dir/bin/python3" -m pip install -r "$requirements"
}

ensure_runtime_config_scaffold() {
  local release_root="$1"
  local target_config="$release_root/vida.config.yaml"
  local packaged_template="$release_root/install/assets/vida.config.yaml.template"
  local source_tree_template="$release_root/docs/framework/templates/vida.config.yaml.template"
  local template_path=""

  if [[ -f "$target_config" ]]; then
    return 0
  fi

  if [[ -f "$packaged_template" ]]; then
    template_path="$packaged_template"
  elif [[ -f "$source_tree_template" ]]; then
    template_path="$source_tree_template"
  else
    fail "Missing runtime config template: expected ${packaged_template} or ${source_tree_template}"
  fi

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would scaffold ${target_config} from ${template_path}"
    return 0
  fi

  cp "$template_path" "$target_config"
}

bootstrap_protocol_binding() {
  local release_root="$1"
  local runtime_bin="$release_root/bin/taskflow-v0"
  [[ -x "$runtime_bin" ]] || fail "Missing bundled taskflow binary for protocol binding bootstrap: $runtime_bin"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would build protocol-binding compiled payload under ${release_root}/taskflow-v0/generated"
    log "Would sync protocol-binding state into ${release_root}/.vida/state/taskflow-state.db"
    return 0
  fi

  log "Building protocol-binding compiled payload"
  VIDA_ROOT="$release_root" "$runtime_bin" protocol-binding build --json >/dev/null
  log "Syncing protocol-binding state into the TaskFlow DB"
  VIDA_ROOT="$release_root" "$runtime_bin" protocol-binding sync --json >/dev/null
  log "Validating protocol-binding state"
  VIDA_ROOT="$release_root" "$runtime_bin" protocol-binding check --json >/dev/null
}

install_management_script() {
  local version="$1"
  local target_dir="$2"
  local target="$target_dir/install.sh"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would install management script into ${target}"
    return 0
  fi

  mkdir -p "$target_dir"

  if [[ -n "${BASH_SOURCE[0]:-}" && -f "${BASH_SOURCE[0]}" ]]; then
    cp "${BASH_SOURCE[0]}" "$target"
  elif [[ -z "$ARCHIVE_FILE" ]]; then
    curl -fsSL "https://github.com/${REPO_SLUG}/releases/download/${version}/vida-install.sh" -o "$target"
  else
    fail "Unable to install management script from the current invocation while using a local archive."
  fi

  chmod +x "$target"
}

cleanup_old_releases() {
  local releases_dir="$1"
  [[ -d "$releases_dir" ]] || return 0

  local count=0
  count="$(find "$releases_dir" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')"
  if (( count <= KEEP_RELEASES )); then
    return 0
  fi

  while (( count > KEEP_RELEASES )); do
    local oldest
    oldest="$(find "$releases_dir" -mindepth 1 -maxdepth 1 -type d | sort | head -n 1)"
    [[ -n "$oldest" ]] || break
    rm -rf "$oldest"
    count=$((count - 1))
  done
}

extract_release_root() {
  local extract_dir="$1"
  local root
  root="$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
  [[ -n "$root" ]] || fail "Unable to resolve extracted release root"
  printf '%s\n' "$root"
}

activate_release() {
  local release_root="$1"
  local current_link="$2"
  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would point ${current_link} -> ${release_root}"
    return 0
  fi
  ln -sfn "$release_root" "$current_link"
}

install_release() {
  local version="$1"
  local temp_dir archive_path checksum_path extract_dir releases_dir current_link release_root env_file
  local installer_dir
  temp_dir="$(mktemp -d)"
  archive_path="${temp_dir}/vida-stack-${version}.tar.gz"
  checksum_path="${temp_dir}/vida-stack-${version}.sha256"
  extract_dir="${temp_dir}/extract"
  releases_dir="${INSTALL_ROOT}/releases"
  current_link="${INSTALL_ROOT}/current"
  release_root="${releases_dir}/${version}"
  env_file="${INSTALL_ROOT}/env.sh"
  installer_dir="${INSTALL_ROOT}/installer"

  trap "rm -rf '$temp_dir'" RETURN

  download_release_archive "$version" "$archive_path"
  download_release_checksum "$version" "$checksum_path"
  verify_archive_checksum "$archive_path" "$checksum_path"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would extract archive into temporary directory"
    log "Would install release into ${release_root}"
    log "Would activate ${current_link}"
    log "Would install wrappers into ${BIN_DIR}"
    log "Would update shell hooks for bash/zsh"
    return 0
  fi

  mkdir -p "$extract_dir"
  tar -xzf "$archive_path" -C "$extract_dir"

  local extracted_root
  extracted_root="$(extract_release_root "$extract_dir")"

  if [[ -e "$release_root" && "$FORCE" != "yes" ]]; then
    fail "Release ${version} already exists at ${release_root}. Re-run with --force to replace it."
  fi

  mkdir -p "$releases_dir"
  rm -rf "$release_root"
  mv "$extracted_root" "$release_root"

  prepare_python_env "$release_root"
  install_management_script "$version" "$installer_dir"
  ensure_runtime_config_scaffold "$release_root"
  write_env_file "$env_file"
  install_profile_hooks "$env_file"
  install_wrappers
  bootstrap_protocol_binding "$release_root"
  activate_release "$release_root" "$current_link"
  cleanup_old_releases "$releases_dir"

  log "Installed VIDA ${version} into ${release_root}"
  log "Active release: ${current_link}"
  log "Launchers: ${BIN_DIR}/vida ${BIN_DIR}/taskflow-v0"
  log "Installed compatibility wrapper: ${BIN_DIR}/codex-v0 -> vida docflow"
  log "Canonical DocFlow bridge: vida docflow -> codex-v0/codex.py"
}

doctor() {
  local current_link="${INSTALL_ROOT}/current"
  local missing=0
  [[ -L "$current_link" || -d "$current_link" ]] || { log "Missing active release link: $current_link"; missing=1; }
  [[ -x "${BIN_DIR}/vida" ]] || { log "Missing launcher: ${BIN_DIR}/vida"; missing=1; }
  [[ -x "${BIN_DIR}/taskflow-v0" ]] || { log "Missing launcher: ${BIN_DIR}/taskflow-v0"; missing=1; }
  [[ -f "${INSTALL_ROOT}/env.sh" ]] || { log "Missing env file: ${INSTALL_ROOT}/env.sh"; missing=1; }
  [[ -x "${INSTALL_ROOT}/installer/install.sh" ]] || { log "Missing installer management script: ${INSTALL_ROOT}/installer/install.sh"; missing=1; }

  if [[ -e "$current_link" ]]; then
    [[ -x "${current_link}/bin/taskflow-v0" ]] || { log "Missing bundled taskflow binary"; missing=1; }
    [[ -x "${current_link}/.venv/bin/python3" ]] || { log "Missing installer-managed Python runtime"; missing=1; }
    [[ -f "${current_link}/codex-v0/codex.py" ]] || { log "Missing bundled codex runtime surface"; missing=1; }
    [[ -f "${current_link}/.codex/config.toml" ]] || { log "Missing bundled .codex config: ${current_link}/.codex/config.toml"; missing=1; }
    [[ -d "${current_link}/.codex/agents" ]] || { log "Missing bundled .codex agents directory: ${current_link}/.codex/agents"; missing=1; }
    [[ -f "${current_link}/AGENTS.sidecar.md" ]] || { log "Missing packaged project sidecar scaffold"; missing=1; }
    [[ -f "${current_link}/vida.config.yaml" ]] || { log "Missing scaffolded runtime config: ${current_link}/vida.config.yaml"; missing=1; }
    [[ -f "${current_link}/install/assets/vida.config.yaml.template" ]] || { log "Missing packaged runtime config template: ${current_link}/install/assets/vida.config.yaml.template"; missing=1; }
    [[ -f "${current_link}/taskflow-v0/helpers/turso_task_store.py" ]] || { log "Missing installed helper: ${current_link}/taskflow-v0/helpers/turso_task_store.py"; missing=1; }
    [[ -f "${current_link}/taskflow-v0/helpers/toon_render.py" ]] || { log "Missing installed helper: ${current_link}/taskflow-v0/helpers/toon_render.py"; missing=1; }
    [[ -f "${current_link}/taskflow-v0/config/protocol_binding.seed.json" ]] || { log "Missing installed protocol-binding seed: ${current_link}/taskflow-v0/config/protocol_binding.seed.json"; missing=1; }
    [[ -f "${current_link}/taskflow-v0/generated/protocol_binding.compiled.json" ]] || { log "Missing installed protocol-binding compiled payload: ${current_link}/taskflow-v0/generated/protocol_binding.compiled.json"; missing=1; }
    [[ -f "${current_link}/.vida/state/taskflow-state.db" ]] || { log "Missing protocol-binding DB state: ${current_link}/.vida/state/taskflow-state.db"; missing=1; }
  fi

  if [[ "$missing" -eq 1 ]]; then
    fail "Doctor found missing installation surfaces."
  fi

  if [[ -e "$current_link" ]]; then
    VIDA_ROOT="$current_link" "${current_link}/bin/taskflow-v0" protocol-binding check --json >/dev/null || \
      fail "Doctor found protocol-binding state drift or missing import."
  fi

  log "Doctor check passed for ${INSTALL_ROOT}"
}

use_release() {
  local version="$1"
  local release_root="${INSTALL_ROOT}/releases/${version}"
  [[ -d "$release_root" ]] || fail "Installed release not found: ${release_root}"
  activate_release "$release_root" "${INSTALL_ROOT}/current"
  log "Switched active VIDA release to ${version}"
}

main() {
  require_cmd curl
  require_cmd tar
  require_cmd mktemp
  require_cmd awk
  require_cmd sed
  require_cmd python3

  case "$COMMAND" in
    help)
      usage
      ;;
    doctor)
      doctor
      ;;
    install|init|upgrade)
      VERSION="$(resolve_version)"
      [[ -n "$VERSION" ]] || fail "Unable to resolve release version"
      install_release "$VERSION"
      ;;
    use)
      [[ "$VERSION" != "latest" ]] || fail "use requires --version <tag>"
      use_release "$VERSION"
      ;;
    *)
      fail "Unsupported command: $COMMAND"
      ;;
  esac
}

main "$@"
