#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/vida-pack-router.sh "<user_request_text>"

Output:
  One of:
    research-pack
    spec-pack
    work-pool-pack
    dev-pack
    bug-pool-pack
    reflection-pack
    mixed-pack
EOF
}

request="${*:-}"
if [[ -z "$request" ]]; then
  usage
  exit 1
fi

text="$(printf '%s' "$request" | tr '[:upper:]' '[:lower:]')"

load_router_aliases() {
  local config_script assignments
  config_script="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/vida-config.py"
  assignments="$(python3 - "$config_script" <<'PY'
import importlib.util
import shlex
import sys

config_script = sys.argv[1]
spec = importlib.util.spec_from_file_location("vida_config_router", config_script)
if spec is None or spec.loader is None:
    raise SystemExit(1)
vida_config = importlib.util.module_from_spec(spec)
spec.loader.exec_module(vida_config)
try:
    cfg = vida_config.load_validated_config()
except (ValueError, vida_config.OverlayValidationError) as exc:
    print(str(exc), file=sys.stderr)
    raise SystemExit(2)

mapping = {
    "research_aliases": "pack_router_keywords.research",
    "spec_aliases": "pack_router_keywords.spec",
    "pool_aliases": "pack_router_keywords.pool",
    "pool_strong_aliases": "pack_router_keywords.pool_strong",
    "pool_dependency_aliases": "pack_router_keywords.pool_dependency",
    "dev_aliases": "pack_router_keywords.dev",
    "bug_aliases": "pack_router_keywords.bug",
    "reflect_aliases": "pack_router_keywords.reflect",
    "reflect_strong_aliases": "pack_router_keywords.reflect_strong",
}

for shell_name, path in mapping.items():
    value = vida_config.dotted_get(cfg, path, "") or ""
    print(f"{shell_name}={shlex.quote(str(value))}")
PY
)"
  eval "$assignments"
}

contains_csv_alias() {
  local haystack="$1"
  local csv="$2"
  local entry trimmed
  [[ -n "$csv" ]] || return 1
  IFS=',' read -r -a _vida_router_aliases <<< "$csv"
  for entry in "${_vida_router_aliases[@]}"; do
    trimmed="$(printf '%s' "$entry" | sed 's/^ *//; s/ *$//')"
    [[ -n "$trimmed" ]] || continue
    if [[ "$haystack" == *"$trimmed"* ]]; then
      return 0
    fi
  done
  return 1
}

matches_any() {
  local haystack="$1"
  shift
  local token
  for token in "$@"; do
    if [[ "$haystack" == *"$token"* ]]; then
      return 0
    fi
  done
  return 1
}

load_router_aliases

score_research=0
score_spec=0
score_pool=0
score_dev=0
score_bug=0
score_reflect=0

if matches_any "$text" "research" || contains_csv_alias "$text" "$research_aliases"; then
  score_research=$((score_research + 2))
fi
if matches_any "$text" "spec" || contains_csv_alias "$text" "$spec_aliases"; then
  score_spec=$((score_spec + 2))
fi
if matches_any "$text" "pool" || contains_csv_alias "$text" "$pool_aliases"; then
  score_pool=$((score_pool + 2))
fi
if matches_any "$text" "form-task" "/vida-form-task" || contains_csv_alias "$text" "$pool_strong_aliases"; then
  score_pool=$((score_pool + 4))
fi
if matches_any "$text" "ready blocked" || contains_csv_alias "$text" "$pool_dependency_aliases"; then
  score_pool=$((score_pool + 2))
fi
if matches_any "$text" "implement" "continue" || contains_csv_alias "$text" "$dev_aliases"; then
  score_dev=$((score_dev + 2))
fi
if matches_any "$text" "bug" "fix" || contains_csv_alias "$text" "$bug_aliases"; then
  score_bug=$((score_bug + 2))
fi
if matches_any "$text" "decision" || contains_csv_alias "$text" "$reflect_aliases"; then
  score_reflect=$((score_reflect + 2))
fi
if matches_any "$text" "subagent" "agent system" "overlay" "initialize" "framework" "self-analysis" || contains_csv_alias "$text" "$reflect_strong_aliases"; then
  score_reflect=$((score_reflect + 3))
fi

scores=("$score_research" "$score_spec" "$score_pool" "$score_dev" "$score_bug" "$score_reflect")
max_score=0
for s in "${scores[@]}"; do
  if (( s > max_score )); then
    max_score="$s"
  fi
done

if (( max_score == 0 )); then
  echo "mixed-pack"
  exit 0
fi

matches=0
winner=""
if (( score_research == max_score )); then matches=$((matches + 1)); winner="research-pack"; fi
if (( score_spec == max_score )); then matches=$((matches + 1)); winner="spec-pack"; fi
if (( score_pool == max_score )); then matches=$((matches + 1)); winner="work-pool-pack"; fi
if (( score_dev == max_score )); then matches=$((matches + 1)); winner="dev-pack"; fi
if (( score_bug == max_score )); then matches=$((matches + 1)); winner="bug-pool-pack"; fi
if (( score_reflect == max_score )); then matches=$((matches + 1)); winner="reflection-pack"; fi

if (( matches > 1 )); then
  echo "mixed-pack"
else
  echo "$winner"
fi
