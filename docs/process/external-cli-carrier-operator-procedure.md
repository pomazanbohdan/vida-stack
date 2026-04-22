# External CLI Carrier Operator Procedure

Purpose: define the bounded project-side operator procedure for external CLI carrier activation, auth repair, model fixation, and smoke validation.

## Scope

This procedure covers the current external CLI carriers wired into the active project runtime:

1. `hermes_cli`
2. `opencode_cli`
3. `kilo_cli`
4. `vibe_cli`

It does not redefine runtime law. It explains how an operator should activate and validate the already-defined project/runtime posture.

## Preconditions

1. Use this procedure only when external CLI carrier status matters for the active session.
2. If interactive auth or model repair is required, the user must disable sandbox for that session first.
3. Treat carrier-local auth/state as an operational dependency, not as the project-owned source of truth.

## Canonical Checks

1. Inspect project/runtime readiness:
   - `vida status --json | jq '.host_agents.external_cli_preflight'`
2. Inspect the active carrier registry:
   - `vida taskflow consume agent-system --json | jq '.snapshot.carriers'`
3. Inspect the bounded design/proof surface:
   - `docs/product/spec/external-cli-carrier-hardening-design.md`

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

## Canonical Repair Procedure

1. Check whether sandbox is active:
   - `vida status --json | jq '.host_agents.external_cli_preflight.sandbox_active'`
2. If interactive auth or model repair is needed and sandbox is active:
   - stop and rerun outside sandbox
3. Re-check carrier readiness:
   - `vida status --json | jq '.host_agents.external_cli_preflight.carrier_readiness'`
4. Repair auth or model posture only for the failing carrier.
5. Re-run the repeatable smoke script:
   - `scripts/external-cli-carrier-smoke.sh`
6. Re-check:
   - `vida status --json | jq '.host_agents.external_cli_preflight'`

## Smoke Validation

Use the repeatable bounded smoke surface:

1. `scripts/external-cli-carrier-smoke.sh`

The script runs one one-shot prompt per enabled carrier using the current project-safe invocation pattern.

## Failure Handling

1. If a carrier still fails after auth repair, do not silently route production work through it.
2. If `carrier_ready_with_override` is reported, the carrier may still be used through the project dispatch path.
3. If `model_not_pinned` is reported, fix the pinning posture before using the carrier for delegated execution.
4. If a carrier-specific provider path regresses, record that in TaskFlow notes before changing project policy.

## References

1. `docs/process/agent-system.md`
2. `docs/product/spec/external-cli-carrier-hardening-design.md`
3. `vida.config.yaml`
4. `scripts/external-cli-carrier-smoke.sh`

-----
artifact_path: process/external-cli-carrier-operator-procedure
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-04-10
schema_version: '1'
status: canonical
source_path: docs/process/external-cli-carrier-operator-procedure.md
created_at: '2026-04-10T11:20:00+03:00'
updated_at: 2026-04-10T08:13:46.69414148Z
changelog_ref: external-cli-carrier-operator-procedure.changelog.jsonl
