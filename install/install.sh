#!/usr/bin/env bash
set -euo pipefail

REPO_SLUG="${VIDA_INSTALL_REPO:-pomazanbohdan/vida-stack}"
COMMAND="${1:-help}"
VERSION="${VIDA_VERSION:-latest}"
ARCHIVE_FILE="${VIDA_ARCHIVE_FILE:-}"
INSTALL_TARGET="${VIDA_INSTALL_TARGET:-auto}"
INSTALL_ROOT="${VIDA_HOME:-$HOME/.local/share/vida-stack}"
BIN_DIR="${VIDA_BIN_DIR:-$HOME/.local/bin}"
FORCE="no"
DRY_RUN="no"
KEEP_RELEASES="${VIDA_KEEP_RELEASES:-3}"
INSTALL_BINS="${VIDA_INSTALL_BINS:-all}"
SHELL_REFRESH_COMMAND=""
TARGET_ASSET_LABEL=""
TARGET_ASSET_SUFFIX=""

Color_Off=''
Red=''
Dim=''
Bold_White=''

if [[ -t 1 ]]; then
  Color_Off='\033[0m'
  Red='\033[0;31m'
  Dim='\033[0;2m'
  Bold_White='\033[1m'
fi

usage() {
  cat <<'EOF'
VIDA installer

Usage:
  bash install.sh <install|init|upgrade|use|doctor|help> [options]

Options:
  --version TAG      Release tag to install. Defaults to latest.
  --archive PATH     Local release archive instead of GitHub download.
  --target TARGET    Release asset target: auto|linux-default|macos-arm64|windows-x86_64.
  --bin-dir PATH     Directory for launcher scripts. Defaults to ~/.local/bin.
  --bins LIST        Comma-separated launchers to expose: vida,taskflow,docflow,all.
  --root PATH        Install root. Defaults to ~/.local/share/vida-stack.
  --force            Overwrite an already installed release of the same version.
  --dry-run          Print planned actions without changing files.
  -h, --help         Show this help.

Examples:
  curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- install
  curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- upgrade --version <tag>
  bash install/install.sh use --version <tag>
  bash install/install.sh install --bins taskflow
  bash install/install.sh install --bins docflow
  bash install/install.sh install --bins vida,taskflow,docflow
  bash install/install.sh doctor
EOF
}

log() {
  printf "${Dim}[vida-installer]${Color_Off} %s\n" "$*"
}

print_install_summary() {
  local version="$1"
  local release_root="$2"
  local current_link="$3"
  local env_file="$4"
  local launcher_summary
  local action_label="installed"
  local action_emoji="🎉"
  launcher_summary="$(selected_launcher_paths)"

  case "$COMMAND" in
    upgrade)
      action_label="updated"
      action_emoji="🩹"
      ;;
    init|install)
      action_label="installed"
      action_emoji="🎉"
      ;;
  esac

  cat <<EOF

${action_emoji} VIDA ${version} ${action_label} successfully
✅ Active release: ${current_link}
📦 Release root: ${release_root}
🧭 Launchers: ${launcher_summary}
🔧 Shell env: ${env_file}
🩹 Active patch line: ${version}

Try it now:
  source "${env_file}"
EOF
  if install_bin_selected vida; then
    cat <<EOF
  vida doctor
EOF
  fi
  if install_bin_selected taskflow; then
    cat <<EOF
  taskflow help
EOF
  fi
  if install_bin_selected docflow; then
    cat <<EOF
  docflow help
EOF
  fi
  if install_bin_selected vida; then
    cat <<EOF
  vida taskflow status --json

Examples:
  vida root
  vida taskflow help
EOF
  else
    cat <<EOF

Examples:
EOF
  fi
  if install_bin_selected taskflow; then
    cat <<EOF
  taskflow help
EOF
  fi
  if install_bin_selected vida; then
    cat <<EOF
  vida taskflow task list --json
  vida docflow overview --format toon
EOF
  fi
  if install_bin_selected docflow; then
    cat <<EOF
  docflow overview --format toon
EOF
  fi

  if [[ -n "$SHELL_REFRESH_COMMAND" ]]; then
    cat <<EOF

Reload shell:
  ${SHELL_REFRESH_COMMAND}
EOF
  fi
}

print_project_init_summary() {
  local project_root="$1"

  cat <<EOF

🧭 Current project bootstrap is ready
📁 Project root: ${project_root}
📚 Copied into the project:
  - AGENTS.md
  - AGENTS.sidecar.md
  - vida/
  - .codex/
  - vida.config.yaml

Next steps:
  cd "${project_root}"
  vida doctor
  vida root
  rg --files vida/config/instructions | head

Project usage examples:
  vida taskflow task list --json
  vida docflow help
EOF
}

print_already_installed_summary() {
  local version="$1"
  local release_root="$2"
  local current_link="$3"

  cat <<EOF

🎉 VIDA ${version} is already the active installed version
✅ Active release: ${current_link}
📦 Release root: ${release_root}
🧭 Nothing to download or replace

Try:
  vida doctor
  vida init
EOF
}

fail() {
  printf "${Red}[vida-installer] ERROR:${Color_Off} %s\n" "$*" >&2
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

parse_latest_release_tag() {
  python3 -c '
import json
import sys

payload = json.load(sys.stdin)
tag = payload.get("tag_name", "")
if not tag:
    raise SystemExit("Missing tag_name in latest-release payload")
print(tag)
'
}

extract_version_from_archive_name() {
  local archive_name="$1"
  local raw_version=""
  if [[ "$archive_name" =~ ^vida-stack-(.+)\.tar\.gz$ ]]; then
    raw_version="${BASH_REMATCH[1]}"
  else
    return 1
  fi

  for known_suffix in \
    "-linux-default" \
    "-macos-arm64" \
    "-macos-x86_64" \
    "-windows-x86_64"
  do
    if [[ "$raw_version" == *"$known_suffix" ]]; then
      raw_version="${raw_version%"$known_suffix"}"
      break
    fi
  done

  [[ -n "$raw_version" ]] || return 1
  printf '%s\n' "$raw_version"
}

resolve_install_target() {
  local requested="$INSTALL_TARGET"
  local os=""
  local arch=""

  if [[ "$requested" == "auto" ]]; then
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m | tr '[:upper:]' '[:lower:]')"

    case "$os" in
      linux*)
        case "$arch" in
          x86_64|amd64)
            requested="linux-default"
            ;;
          *)
            fail "Unsupported Linux architecture for automatic install target selection: ${arch}. Supported target: linux-default (x86_64)."
            ;;
        esac
        ;;
      darwin*)
        case "$arch" in
          arm64|aarch64)
            requested="macos-arm64"
            ;;
          *)
            fail "Unsupported macOS architecture for automatic install target selection: ${arch}. Supported target: macos-arm64."
            ;;
        esac
        ;;
      msys*|mingw*|cygwin*)
        case "$arch" in
          x86_64|amd64)
            requested="windows-x86_64"
            ;;
          *)
            fail "Unsupported Windows architecture for automatic install target selection: ${arch}. Supported target: windows-x86_64."
            ;;
        esac
        ;;
      *)
        fail "Unsupported operating system for automatic install target selection: ${os}"
        ;;
    esac
  fi

  case "$requested" in
    linux-default)
      TARGET_ASSET_LABEL="linux-default"
      TARGET_ASSET_SUFFIX=""
      ;;
    macos-arm64)
      TARGET_ASSET_LABEL="macos-arm64"
      TARGET_ASSET_SUFFIX="-macos-arm64"
      ;;
    windows-x86_64)
      TARGET_ASSET_LABEL="windows-x86_64"
      TARGET_ASSET_SUFFIX="-windows-x86_64"
      ;;
    *)
      fail "Unsupported install target: ${requested}. Allowed: auto|linux-default|macos-arm64|windows-x86_64"
      ;;
  esac
}

archive_basename_for_version() {
  local version="$1"
  printf 'vida-stack-%s%s\n' "$version" "$TARGET_ASSET_SUFFIX"
}

tildify() {
  if [[ "$1" == "$HOME/"* ]]; then
    printf '~/%s\n' "${1#$HOME/}"
    return 0
  fi
  printf '%s\n' "$1"
}

download_url_to_file() {
  local url="$1"
  local destination="$2"

  if command -v curl >/dev/null 2>&1; then
    curl --fail --location --progress-bar "$url" --output "$destination"
    return 0
  fi

  if command -v wget >/dev/null 2>&1; then
    wget -q --show-progress -O "$destination" "$url"
    return 0
  fi

  fail "Missing required downloader: curl or wget"
}

download_url_to_stdout() {
  local url="$1"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url"
    return 0
  fi

  if command -v wget >/dev/null 2>&1; then
    wget -qO- "$url"
    return 0
  fi

  fail "Missing required downloader: curl or wget"
}

normalize_install_bins() {
  local raw="${INSTALL_BINS:-all}"
  raw="${raw// /}"
  if [[ "$raw" == "all" ]]; then
    INSTALL_BINS="vida,taskflow,docflow"
    return 0
  fi
  local IFS=','
  local bin
  local normalized=()
  for bin in $raw; do
    case "$bin" in
      vida|taskflow|docflow)
        normalized+=("$bin")
        ;;
      "")
        ;;
      *)
        fail "Unsupported --bins entry: ${bin}. Allowed: vida,taskflow,docflow,all"
        ;;
    esac
  done
  ((${#normalized[@]} > 0)) || fail "--bins must include at least one launcher"
  INSTALL_BINS="$(IFS=','; printf '%s' "${normalized[*]}")"
}

install_bin_selected() {
  local candidate="$1"
  case ",${INSTALL_BINS}," in
    *",${candidate},"*) return 0 ;;
    *) return 1 ;;
  esac
}

selected_launcher_paths() {
  local values=()
  for launcher in vida taskflow docflow; do
    if install_bin_selected "$launcher"; then
      values+=("${BIN_DIR}/${launcher}")
    fi
  done
  (IFS=', '; printf '%s' "${values[*]}")
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
    --bins)
      INSTALL_BINS="${2:-}"
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
    --target)
      INSTALL_TARGET="${2:-}"
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
normalize_install_bins

resolve_version() {
  if [[ "$VERSION" != "latest" ]]; then
    printf '%s\n' "$VERSION"
    return 0
  fi

  if [[ -n "$ARCHIVE_FILE" ]]; then
    local archive_name
    local extracted
    archive_name="$(basename "$ARCHIVE_FILE")"
    if extracted="$(extract_version_from_archive_name "$archive_name")"; then
      printf '%s\n' "$extracted"
      return 0
    fi
    fail "Unable to infer version from local archive name: ${archive_name}"
  fi

  download_url_to_stdout "https://api.github.com/repos/${REPO_SLUG}/releases/latest" \
    | parse_latest_release_tag
}

download_release_archive() {
  local version="$1"
  local archive_basename="$2"
  local destination="$3"
  if [[ -n "$ARCHIVE_FILE" ]]; then
    log "Using local archive ${ARCHIVE_FILE}"
    if [[ "$DRY_RUN" == "yes" ]]; then
      return 0
    fi
    [[ -f "$ARCHIVE_FILE" ]] || fail "Local archive not found: ${ARCHIVE_FILE}"
    cp "$ARCHIVE_FILE" "$destination"
    return 0
  fi

  local url="https://github.com/${REPO_SLUG}/releases/download/${version}/${archive_basename}.tar.gz"
  log "Downloading ${url}"
  if [[ "$DRY_RUN" == "yes" ]]; then
    return 0
  fi
  download_url_to_file "$url" "$destination"
}

download_release_checksum() {
  local version="$1"
  local archive_basename="$2"
  local destination="$3"
  if [[ -n "$ARCHIVE_FILE" ]]; then
    log "Skipping checksum download for local archive"
    return 0
  fi

  local url="https://github.com/${REPO_SLUG}/releases/download/${version}/${archive_basename}.sha256"
  log "Downloading ${url}"
  if [[ "$DRY_RUN" == "yes" ]]; then
    return 0
  fi
  download_url_to_file "$url" "$destination"
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
  local shell_name="${SHELL##*/}"
  local candidates=()
  local target=""

  case "$shell_name" in
    zsh)
      candidates=("$HOME/.zshrc" "$HOME/.zprofile")
      ;;
    bash)
      candidates=("$HOME/.bash_profile" "$HOME/.bashrc")
      ;;
    *)
      candidates=("$HOME/.profile")
      ;;
  esac

  for candidate in "${candidates[@]}"; do
    mkdir -p "$(dirname "$candidate")" 2>/dev/null || true
    if [[ -f "$candidate" ]]; then
      if [[ -w "$candidate" ]]; then
        target="$candidate"
        break
      fi
      continue
    fi
    if touch "$candidate" 2>/dev/null; then
      target="$candidate"
      break
    fi
  done

  if [[ -n "$target" ]]; then
    append_source_line "$target" "$source_line"
    log "Added shell hook to $(tildify "$target")"
    case "$shell_name" in
      zsh)
        SHELL_REFRESH_COMMAND="exec \$SHELL"
        ;;
      bash)
        SHELL_REFRESH_COMMAND="source $(tildify "$target")"
        ;;
      *)
        SHELL_REFRESH_COMMAND="source $(tildify "$target")"
        ;;
    esac
    return 0
  fi

  log "Could not auto-update shell profile for PATH/env hook"
  printf '%s\n' "${Bold_White}Add this line manually:${Color_Off}"
  printf '  %s\n' "$source_line"
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

write_standalone_wrapper() {
  local launcher="$1"
  write_wrapper "$BIN_DIR/${launcher}" '
VIDA_HOME="'"$INSTALL_ROOT"'"
VIDA_ROOT="$VIDA_HOME/current"
RUNTIME_BIN="$VIDA_ROOT/bin/'"$launcher"'"

if [[ -x "$RUNTIME_BIN" ]]; then
  exec "$RUNTIME_BIN" "$@"
else
  cat <<'\''USAGE'\''
Standalone runtime binary is missing.
USAGE
fi
'
}

install_wrappers() {
  if install_bin_selected vida; then
    write_wrapper "$BIN_DIR/vida" '
VIDA_HOME="'"$INSTALL_ROOT"'"
VIDA_ROOT="$VIDA_HOME/current"
RUNTIME_BIN="$VIDA_ROOT/bin/vida"

runtime_usage() {
  if [[ -x "$RUNTIME_BIN" ]]; then
    "$RUNTIME_BIN" --help
  else
    cat <<'\''USAGE'\''
VIDA runtime binary is missing.
USAGE
  fi
}

usage() {
  runtime_usage
  cat <<'\''USAGE'\''

Installer management:
  vida upgrade [--version TAG]
  vida use --version TAG
  vida install [--version TAG]
  vida root
USAGE
}

sub="${1:-help}"
case "$sub" in
  upgrade|install|use)
    exec "$VIDA_HOME/installer/install.sh" "$sub" --root "$VIDA_HOME" --bin-dir "'"$BIN_DIR"'" "${@:2}"
    ;;
  root)
    printf "%s\n" "$VIDA_ROOT"
    ;;
  help|--help|-h)
    usage
    ;;
  *)
    exec "$RUNTIME_BIN" "$@"
    ;;
esac
'
  fi

  if install_bin_selected taskflow; then
    write_standalone_wrapper taskflow
  fi
  if install_bin_selected docflow; then
    write_standalone_wrapper docflow
  fi
}

prepare_python_env() {
  return 0
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

copy_project_file() {
  local source_path="$1"
  local target_path="$2"
  local label="$3"

  [[ -f "$source_path" ]] || fail "Missing project bootstrap source: $source_path"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would copy ${label} into ${target_path}"
    return 0
  fi

  if [[ -e "$target_path" && "$FORCE" != "yes" ]]; then
    log "Keeping existing ${label}: ${target_path}"
    return 0
  fi

  mkdir -p "$(dirname "$target_path")"
  cp "$source_path" "$target_path"
}

copy_project_tree() {
  local source_path="$1"
  local target_path="$2"
  local label="$3"

  [[ -d "$source_path" ]] || fail "Missing project bootstrap tree: $source_path"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would copy ${label} into ${target_path}"
    return 0
  fi

  if [[ -e "$target_path" && "$FORCE" != "yes" ]]; then
    log "Keeping existing ${label}: ${target_path}"
    return 0
  fi

  rm -rf "$target_path"
  mkdir -p "$(dirname "$target_path")"
  cp -R "$source_path" "$target_path"
}

bootstrap_current_project() {
  local release_root="$1"
  local project_root="${VIDA_PROJECT_ROOT:-$PWD}"
  local template_path="$release_root/install/assets/vida.config.yaml.template"

  [[ -d "$project_root" ]] || fail "Missing target project directory: $project_root"

  copy_project_file "$release_root/AGENTS.md" "$project_root/AGENTS.md" "framework bootstrap carrier"
  copy_project_file "$release_root/AGENTS.sidecar.md" "$project_root/AGENTS.sidecar.md" "project sidecar scaffold"
  copy_project_tree "$release_root/vida" "$project_root/vida" "framework protocol tree"
  copy_project_tree "$release_root/.codex" "$project_root/.codex" "project-local Codex configuration"
  copy_project_file "$template_path" "$project_root/vida.config.yaml" "project runtime config scaffold"

  if [[ "$DRY_RUN" != "yes" ]]; then
    print_project_init_summary "$project_root"
  fi
}

bootstrap_protocol_binding() {
  return 0
}

install_management_script() {
  local version="$1"
  local target_dir="$2"
  local target="$target_dir/install.sh"
  local source_script=""

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Would install management script into ${target}"
    return 0
  fi

  mkdir -p "$target_dir"

  if [[ -n "${BASH_SOURCE[0]:-}" && -f "${BASH_SOURCE[0]}" ]]; then
    source_script="${BASH_SOURCE[0]}"
    if [[ "$source_script" == "$target" ]]; then
      log "Installer management script already current at ${target}"
    else
      cp "$source_script" "$target"
    fi
  elif [[ -z "$ARCHIVE_FILE" ]]; then
    download_url_to_file "https://github.com/${REPO_SLUG}/releases/download/${version}/vida-install.sh" "$target"
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
  local temp_dir archive_path checksum_path extract_dir releases_dir current_link release_root env_file archive_basename
  local installer_dir
  resolve_install_target
  releases_dir="${INSTALL_ROOT}/releases"
  current_link="${INSTALL_ROOT}/current"
  release_root="${releases_dir}/${version}"
  installer_dir="${INSTALL_ROOT}/installer"
  env_file="${INSTALL_ROOT}/env.sh"

  if [[ "$FORCE" != "yes" && -d "$release_root" ]]; then
    local active_release=""
    if [[ -e "$current_link" ]]; then
      active_release="$(readlink -f "$current_link" 2>/dev/null || true)"
    fi
    if [[ -n "$active_release" && "$active_release" == "$release_root" ]]; then
      print_already_installed_summary "$version" "$release_root" "$current_link"
      return 0
    fi
  fi

  temp_dir="$(mktemp -d)"
  archive_basename="$(archive_basename_for_version "$version")"
  archive_path="${temp_dir}/${archive_basename}.tar.gz"
  checksum_path="${temp_dir}/${archive_basename}.sha256"
  extract_dir="${temp_dir}/extract"

  trap "rm -rf '$temp_dir'" RETURN

  download_release_archive "$version" "$archive_basename" "$archive_path"
  download_release_checksum "$version" "$archive_basename" "$checksum_path"
  verify_archive_checksum "$archive_path" "$checksum_path"

  if [[ "$DRY_RUN" == "yes" ]]; then
    log "Resolved release target: ${TARGET_ASSET_LABEL}"
    log "Resolved archive: ${archive_basename}.tar.gz"
    log "Would extract archive into temporary directory"
    log "Would install release into ${release_root}"
    log "Would activate ${current_link}"
    log "Would install launchers: $(selected_launcher_paths)"
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
  log "Release target: ${TARGET_ASSET_LABEL}"
  log "Launchers: $(selected_launcher_paths)"
  print_install_summary "$version" "$release_root" "$current_link" "$env_file"
}

doctor() {
  local current_link="${INSTALL_ROOT}/current"
  local missing=0
  [[ -L "$current_link" || -d "$current_link" ]] || { log "Missing active release link: $current_link"; missing=1; }
  if install_bin_selected vida; then
    [[ -x "${BIN_DIR}/vida" ]] || { log "Missing launcher: ${BIN_DIR}/vida"; missing=1; }
  fi
  if install_bin_selected taskflow; then
    [[ -x "${BIN_DIR}/taskflow" ]] || { log "Missing launcher: ${BIN_DIR}/taskflow"; missing=1; }
  fi
  if install_bin_selected docflow; then
    [[ -x "${BIN_DIR}/docflow" ]] || { log "Missing launcher: ${BIN_DIR}/docflow"; missing=1; }
  fi
  [[ -f "${INSTALL_ROOT}/env.sh" ]] || { log "Missing env file: ${INSTALL_ROOT}/env.sh"; missing=1; }
  [[ -x "${INSTALL_ROOT}/installer/install.sh" ]] || { log "Missing installer management script: ${INSTALL_ROOT}/installer/install.sh"; missing=1; }

  if [[ -e "$current_link" ]]; then
    [[ -x "${current_link}/bin/vida" ]] || { log "Missing bundled vida binary"; missing=1; }
    [[ -x "${current_link}/bin/taskflow" ]] || { log "Missing bundled taskflow binary"; missing=1; }
    [[ -x "${current_link}/bin/docflow" ]] || { log "Missing bundled docflow binary"; missing=1; }
    [[ -f "${current_link}/.codex/config.toml" ]] || { log "Missing bundled .codex config: ${current_link}/.codex/config.toml"; missing=1; }
    [[ -d "${current_link}/.codex/agents" ]] || { log "Missing bundled .codex agents directory: ${current_link}/.codex/agents"; missing=1; }
    [[ -f "${current_link}/AGENTS.sidecar.md" ]] || { log "Missing packaged project sidecar scaffold"; missing=1; }
    [[ -f "${current_link}/vida.config.yaml" ]] || { log "Missing scaffolded runtime config: ${current_link}/vida.config.yaml"; missing=1; }
    [[ -f "${current_link}/install/assets/vida.config.yaml.template" ]] || { log "Missing packaged runtime config template: ${current_link}/install/assets/vida.config.yaml.template"; missing=1; }
  fi

  if [[ "$missing" -eq 1 ]]; then
    fail "Doctor found missing installation surfaces."
  fi
  log "Doctor check passed for ${INSTALL_ROOT}"
}

use_release() {
  local version="$1"
  local release_root="${INSTALL_ROOT}/releases/${version}"
  [[ -d "$release_root" ]] || fail "Installed release not found: ${release_root}"
  activate_release "$release_root" "${INSTALL_ROOT}/current"
  install_wrappers
  log "Switched active VIDA release to ${version}"
}

main() {
  command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || fail "Missing required downloader: curl or wget"
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
    install|upgrade)
      VERSION="$(resolve_version)"
      [[ -n "$VERSION" ]] || fail "Unable to resolve release version"
      install_release "$VERSION"
      ;;
    init)
      VERSION="$(resolve_version)"
      [[ -n "$VERSION" ]] || fail "Unable to resolve release version"
      install_release "$VERSION"
      bootstrap_current_project "${INSTALL_ROOT}/current"
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
