#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GO_BIN="${GO:-go}"

if [[ "$GO_BIN" == */* || "$GO_BIN" == *\\* ]]; then
  if [[ ! -x "$GO_BIN" ]]; then
    printf '[render-public-release-notes] ERROR: go is required to render the public release body\n' >&2
    exit 1
  fi
else
  GO_BIN="$(command -v "$GO_BIN" || true)"
fi

if [[ -z "$GO_BIN" ]]; then
  printf '[render-public-release-notes] ERROR: go is required to render the public release body\n' >&2
  exit 1
fi

cd "$ROOT_DIR/tools/render-public-release-notes"
VIDA_REPO_ROOT="$ROOT_DIR" exec "$GO_BIN" run . "$@"
