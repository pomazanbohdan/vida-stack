#!/usr/bin/env bash

vida_icon() {
  case "${1:-info}" in
    ok) printf '✅' ;;
    warn) printf '⚠️' ;;
    fail) printf '❌' ;;
    blocked) printf '⛔' ;;
    info) printf 'ℹ️' ;;
    sparkle) printf '✨' ;;
    progress) printf '🔄' ;;
    *)
      printf '•'
      ;;
  esac
}

vida_status_line() {
  local level="${1:-info}"
  shift || true
  printf '%s %s\n' "$(vida_icon "$level")" "$*"
}
