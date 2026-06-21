# External CLI Carrier Operator Procedure

Purpose: define the bounded project-side operator procedure for external CLI carrier activation, auth repair, model fixation, and smoke validation.

## Scope

This procedure covers the current external CLI carriers wired into the active project runtime:

1. `hermes_cli`
2. `opencode_cli`
3. `kilo_cli`
4. `vibe_cli`
5. `pi_cli`
6. `vida_coder`

It does not redefine runtime law. It explains how an operator should activate and validate the already-defined project/runtime posture.

## Preconditions

1. Use this procedure only when external CLI carrier status matters for the active session.
2. If interactive auth or model repair is required, the user must disable sandbox for that session first.
3. Treat carrier-local auth/state as an operational dependency, not as the project-owned source of truth.

## Canonical Checks

1. Inspect compact project/runtime readiness:
   - `vida status --summary --json`
2. Inspect external CLI preflight details when carrier readiness matters:
   - `vida status --json | jq '.host_agents.external_cli_preflight'`
3. Inspect the active carrier registry and profile truth:
   - `vida taskflow consume agent-system --json | jq '.snapshot.carriers'`
4. Inspect Pi adapter availability when `pi_cli` is selected or under repair:
   - `pi --version`
   - `vida-pi-agent --help`
5. Inspect the bounded design/proof surfaces:
    - `docs/product/spec/external-cli-carrier-hardening-contract.md`
    - `docs/product/spec/pi-primary-environment-agent-carrier-spec.md`
6. Inspect VIDA Coder service-executor readiness when `vida_coder` is selected or under repair:
   - `vida-coder provider-check --json`
   - `vida taskflow consume agent-system --json | jq '.snapshot.carriers.vida_coder'`

## Readiness States

Interpret `host_agents.external_cli_preflight` as follows:

1. `sandbox_blocked`
   - network/interactive carrier work is blocked by sandbox posture
2. `interactive_auth_required`
   - the carrier is present, but auth material is missing
3. `provider_auth_failed`
   - the carrier auth path exists, but the provider/model path is still failing
4. `model_not_pinned`
   - the carrier-local model path does not match project intent and dispatch cannot safely override it
5. `carrier_ready`
   - the carrier is ready on its own current auth/model path
6. `carrier_ready_with_override`
   - the carrier-local model differs, but project dispatch pinning will execution-enforce the canonical model

## Carrier Rules

### hermes

1. CLI supports both `--model` and `--provider`.
2. The active local carrier may still be provider-configured rather than project-pinned.
3. If Hermes is meant to remain provider-configured, treat that as an explicit operator decision, not an accidental default.

### opencode

1. Always treat `opencode/minimax-m2.5-free` as the current canonical working model unless project config changes again.
2. Project dispatch now overrides ambient recent-model drift through `--model`.
3. If auth breaks:
   - preferred path: operator runs `opencode auth login -p <provider>` directly outside sandbox
4. If local state drifts:
   - inspect `~/.local/state/opencode/model.json`
   - re-run `vida status --json`

### kilo

1. CLI supports direct model pinning with `--model`.
2. The current project profile expects `kilo/x-ai/grok-code-fast-1:optimized:free`.
3. If auth breaks:
   - inspect `~/.local/share/kilo/auth.json`
   - re-run bounded smoke validation

### vibe

1. `vibe` is config-driven rather than CLI-model-flag-driven.
2. The current project profile expects `active_model = "devstral-2"` in `~/.vibe/config.toml`.
3. If auth breaks:
   - inspect `~/.vibe/.env`
   - inspect `~/.vibe/config.toml`
   - re-run bounded smoke validation

### pi

1. `pi_cli` is an external CLI carrier backend executed through the VIDA-owned `vida-pi-agent` adapter. Do not configure raw `pi` as the dispatch command.
2. Each dispatch is one process/session: VIDA starts `vida-pi-agent`, the adapter starts one Pi RPC process, the bounded packet is executed, parseable VIDA result JSON is emitted, and the process exits. No long-lived Pi daemon/session is part of the VIDA contract.
3. Model/profile truth is owned by `vida.config.yaml -> agent_system.subagents.pi_cli.model_profiles` and the runtime-selected profile. Pi-local defaults and `.pi/**` files are projections or provider state only; they must not override VIDA selection.
4. Setup/readiness checks:
   - `pi --version`
   - `vida-pi-agent --help`
   - `vida status --summary --json`
   - `vida status --json | jq '.host_agents.external_cli_preflight'`
5. `vida-pi-agent` is packaged beside `vida`, `taskflow`, and `docflow`; release/install exposes it as a direct binary on the installed runtime `bin` path.
6. Read/spec/review Pi profiles must not write. Implementation/write profiles that require `guard_required_owned_paths` are admissible only when `vida status --json` reports `write_scope_guard.pre_write_enforcement=true` and `write_scope_guard.status=active` for the selected Pi profile.
7. In guarded-write mode, `vida-pi-agent` explicitly loads a VIDA-owned Pi extension into the one-shot Pi process. The extension receives canonical guard data through `VIDA_PI_AGENT_SCOPE_GUARD_MODE`, `VIDA_PI_AGENT_PROJECT_ROOT`, and `VIDA_PI_AGENT_OWNED_PATHS_JSON`, blocks `write`/`edit` paths outside dispatch owned paths before execution, blocks `bash`/user bash to prevent shell write bypass, and blocks unknown mutating tools. The adapter still performs post-execution touched-path validation as defense-in-depth.

### vida-coder

1. `vida_coder` is a service-executor backend exposed through the packaged `vida-coder` binary. Do not route bounded coder packets through a raw provider command.
2. Model/profile truth is owned by `vida.config.yaml -> agent_system.subagents.vida_coder.model_profiles`; ambient provider configuration is readiness evidence only and must not override VIDA selection.
3. Setup/readiness checks:
   - `vida-coder provider-check --json`
   - `vida status --summary --json`
   - `vida taskflow consume agent-system --json | jq '.snapshot.carriers.vida_coder'`
4. Auth and model readiness must be reference-based. Do not write secrets, raw API keys, tokens, or provider credentials into prompts, receipts, task notes, CLI arguments, or release manifests.
5. `vida-coder --service dispatch --json` remains fail-closed until the service scheduler and guarded runtime-tool pipeline are executable for live delegated packets.
6. `vida-coder` is packaged beside `vida`, `taskflow`, `docflow`, and `vida-pi-agent`; release proof includes `vida-coder provider-check --json` from the packaged binary.

## Canonical Repair Procedure

1. Check whether sandbox is active:
   - `vida status --json | jq '.host_agents.external_cli_preflight.sandbox_active'`
2. If interactive auth or model repair is needed and sandbox is active:
   - stop and rerun outside sandbox
3. Re-check carrier readiness:
   - `vida status --json | jq '.host_agents.external_cli_preflight.carrier_readiness'`
4. Repair auth or model posture only for the failing carrier. For Pi, verify both adapter and provider layers: `vida-pi-agent --help` for the adapter and `pi --version` plus live/provider auth outside sandbox for Pi itself.
5. Re-run the current bounded adapter/runtime proof. For Pi adapter changes, prefer the package-scoped nextest contract proof:
   - `cargo nextest run --locked -p vida-pi-agent --profile default`
   - `cargo build -p vida-pi-agent --bins --locked` only when a local binary is needed for manual operator inspection
   - optional live provider probes must be explicit, credential-aware operator actions and must not be hidden in a hardcoded smoke script
6. Re-check:
   - `vida status --json | jq '.host_agents.external_cli_preflight'`

## Smoke Validation

Use the repeatable bounded proof surfaces:

1. `cargo nextest run --locked -p vida-pi-agent --profile default`
2. `vida status --json | jq '.host_agents.external_cli_preflight'`
3. `vida taskflow consume agent-system --json | jq '.snapshot.carriers'`

Carrier proof must stay config/runtime-derived. Do not add hardcoded one-shot prompt scripts for a fixed list of host CLIs; they drift from `vida.config.yaml`, carrier registries, and provider readiness policy.

Pi-specific smoke modes:

1. Build/run adapter tests when changing the adapter or its contract:
   - `cargo nextest run --locked -p vida-pi-agent --profile default`
2. Build Pi binaries only when local manual inspection needs them:
   - `cargo build -p vida-pi-agent --bins --locked`
3. Run optional live Pi/provider smoke only as an explicit operator action outside the generic CI/local gate, with credentials and network posture intentionally available.
4. When a live probe is needed, record `VIDA_PI_COMMAND`, adapter path, timeout, prompt, result JSON, and credential posture in the TaskFlow note instead of relying on ambient defaults.

## Failure Handling

1. If a carrier still fails after auth repair, do not silently route production work through it.
2. If `carrier_ready_with_override` is reported, the carrier may still be used through the project dispatch path.
3. If `model_not_pinned` is reported, fix the pinning posture before using the carrier for delegated execution.
4. If a carrier-specific provider path regresses, record that in TaskFlow notes before changing project policy.
5. For Pi adapter failures:
   - missing `vida-pi-agent`: install/build the VIDA runtime package that includes the adapter, then re-check `vida-pi-agent --help`
   - missing `pi`: install or expose the Pi provider CLI on `PATH`, then re-check `pi --version`
   - invalid model: update the VIDA config profile only after confirming the Pi provider catalog; do not rely on Pi ambient defaults
   - auth/provider errors: repair Pi provider auth outside sandbox, then re-run status and smoke
   - `write_scope_guard_required` or `write_scope_inadmissible_for_task_class`: do not force write dispatch; update/install `vida-pi-agent` and re-check that `write_scope_guard.pre_write_enforcement=true` and `write_scope_guard.status=active` before using guarded write profiles
   - parse or timeout errors: preserve the adapter JSON/error message in TaskFlow notes before changing runtime policy

## References

1. `docs/process/agent-system.md`
2. `docs/product/spec/external-cli-carrier-hardening-contract.md`
3. `vida.config.yaml`
4. `docs/product/spec/pi-primary-environment-agent-carrier-spec.md`

-----
artifact_path: process/external-cli-carrier-operator-procedure
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-02
schema_version: '1'
status: canonical
source_path: docs/process/external-cli-carrier-operator-procedure.md
created_at: '2026-04-10T11:20:00+03:00'
updated_at: 2026-06-02T03:05:00+03:00
changelog_ref: external-cli-carrier-operator-procedure.changelog.jsonl
