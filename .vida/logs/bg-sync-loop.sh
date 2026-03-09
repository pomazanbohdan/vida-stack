#!/usr/bin/env bash
set -euo pipefail
cd "/home/unnamed/project/vida-stack"
source "/home/unnamed/project/vida-stack/_vida/scripts/beads-runtime.sh"
while true; do
  beads_snapshot_jsonl "bg-sync" >/dev/null 2>>"/home/unnamed/project/vida-stack/.vida/logs/bg-sync.err" || true
  sleep 600
done
