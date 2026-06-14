#!/usr/bin/env bash
set -euo pipefail

find_rg() {
    if [[ -n "${RG:-}" && -x "${RG}" ]]; then
        printf '%s\n' "${RG}"
        return 0
    fi

    if command -v rg >/dev/null 2>&1; then
        command -v rg
        return 0
    fi

    local candidate
    for candidate in \
        "${HOME}/.bun/install/global/node_modules/@vscode/ripgrep-win32-x64/bin/rg.exe" \
        "${HOME}/.bun/install/global/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/codex-path/rg.exe"
    do
        if [[ -x "${candidate}" ]]; then
            printf '%s\n' "${candidate}"
            return 0
        fi
    done

    return 1
}

existing_paths() {
    local pattern match
    for pattern in "$@"; do
        if [[ "${pattern}" == *"*"* ]]; then
            for match in ${pattern}; do
                [[ -d "${match}" ]] && printf '%s\n' "${match}"
            done
        elif [[ -e "${pattern}" ]]; then
            printf '%s\n' "${pattern}"
        fi
    done
}

run_check() {
    local name="$1"
    local pattern="$2"
    local paths_var="$3"
    local globs_var="$4"
    local -n paths_ref="${paths_var}"
    local -n globs_ref="${globs_var}"
    local output exit_code

    set +e
    output="$("${RG_BIN}" --color never --line-number "${pattern}" "${paths_ref[@]}" "${globs_ref[@]}" 2>&1)"
    exit_code=$?
    set -e

    if [[ ${exit_code} -eq 1 ]]; then
        printf -- '- %s: pass\n' "${name}"
        return 0
    fi

    local display_output omitted_count
    display_output="$(printf '%s\n' "${output}" | sed -n '1,80p')"
    omitted_count="$(( $(printf '%s\n' "${output}" | sed '/^$/d' | wc -l) - $(printf '%s\n' "${display_output}" | sed '/^$/d' | wc -l) ))"
    if [[ ${omitted_count} -gt 0 ]]; then
        display_output="${display_output}"$'\n'"... omitted ${omitted_count} additional matches"
    fi

    if [[ ${exit_code} -ne 0 ]]; then
        printf -- '- %s: error\n%s\n' "${name}" "${display_output}"
        return 1
    fi

    printf -- '- %s: blocked\n%s\n' "${name}" "${display_output}"
    return 1
}

path_absent_check() {
    local name="$1"
    shift
    local status=0
    local path
    for path in "$@"; do
        if [[ -e "${path}" ]]; then
            [[ ${status} -eq 0 ]] && printf -- '- %s: blocked\n' "${name}"
            printf '%s\n' "${path}"
            status=1
        fi
    done
    if [[ ${status} -eq 0 ]]; then
        printf -- '- %s: pass\n' "${name}"
    fi
    return "${status}"
}

path_present_check() {
    local name="$1"
    local path="$2"
    if [[ -e "${path}" ]]; then
        printf -- '- %s: pass\n' "${name}"
        return 0
    fi
    printf -- '- %s: blocked\nmissing: %s\n' "${name}" "${path}"
    return 1
}

RG_BIN="$(find_rg)" || {
    printf '%s\n' "ripgrep executable not found. Install rg, put it on PATH, or set RG to the executable path." >&2
    exit 2
}

mapfile -t vida_paths < <(existing_paths crates/vida/src)

common_globs=(-g '!**/tests/**' -g '!**/generated/**' -g '!**/adapters/**')

status=0
printf '%s\n' 'runtime boundary checks:'
path_absent_check 'legacy vida operator facade files removed' \
    crates/vida/src/operator_command_text.rs \
    crates/vida/src/operator_contracts.rs \
    crates/vida/src/operator_toon_report.rs || status=1
path_present_check 'release1 operator output bridge present' crates/vida/src/release1_operator_output.rs || status=1
run_check 'no legacy vida operator facade imports' 'mod operator_(command_text|contracts|toon_report)|crate::operator_(command_text|contracts|toon_report)|use crate::operator_(command_text|contracts|toon_report)::' vida_paths common_globs || status=1
run_check 'no broad runtime_dispatch_state export' 'pub\(crate\) use runtime_dispatch_state::\*' vida_paths common_globs || status=1

exit "${status}"
