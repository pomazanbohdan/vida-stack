# Pi Primary Environment And Agent Carrier Design

Status: `proposed`

Use this document as the bounded design/TZ for the TaskFlow epic `feature-vida-pi-agent-primary-environment` before any Pi config, template, adapter, readiness, dispatch, or release code changes.

## Summary
- Feature / change: add Pi as a first-class VIDA primary host environment, add `pi_cli` as a write-capable external CLI carrier through a `vida-pi-agent` adapter, and project VIDA-owned runtime roles/profiles into Pi internal-agent files under `.pi/**`.
- Owner layer: `mixed`
- Runtime surface: `project activation | taskflow | agent-init | status | release/install | external cli carrier`
- Status: proposed

## Current Context
- VIDA already has a carrier/runtime-profile model in `vida.config.yaml` and runtime assignment paths that select carriers by runtime role, task class, readiness, score, cost, and quality constraints rather than by a fixed hardcoded agent identity.
- Existing external CLI seams include configured external carrier profiles, readiness reporting in status surfaces, dispatch command allowlisting, external result parsing, and smoke scripts for external carriers.
- Existing host-environment materialization treats on-disk host templates such as `.codex/**` as projections from canonical config/runtime truth, not as authority surfaces.
- Pi is installed in the current operator environment and supports direct prompt mode, JSON event mode, and RPC mode. RPC mode exposes state/model catalog operations, thinking-level control, prompt execution, and terminal `agent_end` events.
- Prior exploratory evidence showed:
  - `pi -p --no-session --no-context-files --no-skills --no-extensions --no-prompt-templates --no-tools "Reply exactly: OK"` returns `OK`.
  - `pi --mode json` emits JSONL events with final answer in `agent_end.messages`.
  - `pi --mode rpc` supports `get_state`, `get_available_models`, `set_thinking_level`, and `prompt`.
  - Invalid model selection fails with an explicit model-not-found error.
- The current gap is that Pi is not represented as a primary VIDA host environment, not represented as a trusted external executor carrier, and not projected into `.pi/**` from VIDA runtime truth.
- Raw `pi` output is not a VIDA execution receipt. VIDA needs a bounded adapter that normalizes Pi execution into canonical external result JSON and preserves process/session, readiness, receipt, and write-scope semantics.

## Goal
- Make Pi available as the preferred implementation-capable external backend when VIDA runtime selection deems it admissible.
- Add a distinct `host_environment.systems.pi` entry so Pi can be selected/materialized as the primary local host CLI environment.
- Add `agent_system.subagents.pi_cli`/carrier profile support so runtime assignment can select Pi dynamically for implementation, specification, review, or verification classes according to config and readiness.
- Add Pi internal-agent projection support so `.pi/settings.json`, `.pi/agents/*.md`, and optional `.pi/chains/*.chain.md` are generated from VIDA roles/carriers/profiles without becoming authority.
- Preserve one-dispatch-one-process execution: each VIDA dispatch starts Pi, runs one bounded packet, records evidence, and exits.
- Preserve fail-closed safety: no unrestricted raw Pi writes, no long-lived Pi daemon/session, no activation/view-only output treated as completion, and no model/profile selection outside VIDA runtime algorithms.
- Out of scope:
  - Directly dispatching raw `pi` as the VIDA external provider without `vida-pi-agent`.
  - Treating Pi-local `.pi/**` files as source of truth.
  - Making Pi write-capable before packet-owned write-scope guard and touched-path validation exist.
  - Replacing VIDA runtime selection with static user-selected Pi models.
  - Adding unrelated qwen/hermes/opencode/vibe behavior changes in this epic.

## Requirements

### Functional Requirements
- `host_environment.systems.pi`
  - Must be a selectable host CLI system in project activation/materialization flows.
  - Must define Pi-specific template/projection roots and setup/readiness expectations.
  - Must not change authority: canonical carrier/profile truth remains in config/runtime state.
- `agent_system.subagents.pi_cli`
  - Must represent Pi as an external CLI carrier backend rather than an internal Codex subagent.
  - Must expose runtime roles, task classes, lifecycle, readiness, write posture, cost units, and model profile ids from config.
  - Must be admissible for implementation only when write-scope guard requirements are met.
- `vida-pi-agent` adapter
  - Must be implemented as a VIDA-owned Rust workspace binary, not a loose script.
  - Must spawn one Pi process per dispatch, prefer `pi --mode rpc`, set selected model/thinking level, send one prompt/packet, wait for terminal `agent_end`, emit canonical VIDA external result JSON, and terminate the process.
  - Must normalize success and error outputs into the external result contract already understood by VIDA dispatch parsing.
- Dynamic model/profile selection
  - `vida.config.yaml` and `docs/framework/templates/vida.config.yaml.template` must list admissible Pi profiles and thinking/reasoning levels.
  - VIDA runtime assignment must choose the profile dynamically by role/task/readiness/score/cost/quality, not by a Pi-local default.
- Readiness/status/preflight
  - Status/readiness surfaces must report adapter command availability, Pi command availability, Pi auth posture when detectable, model catalog availability, selected profile validity, thinking-level support, write-guard readiness, and blocker codes/next actions.
  - Missing command, missing adapter, invalid model, unavailable auth, unsupported thinking level, or missing write guard must fail closed before execution dispatch.
- Receipt-backed dispatch
  - `vida agent-init --execute-dispatch` and downstream dispatch state must require parseable result evidence from `vida-pi-agent`; activation/view-only output is not completion evidence.
  - Success result should include final answer, provider/mode metadata, model/profile metadata when available, usage when available, and touched-path summary when write is enabled.
  - Failure result must remain JSON and must preserve the provider error message.
- Bounded write-scope guard
  - Pi write-capable profiles require packet-owned paths from the VIDA dispatch packet.
  - Write attempts outside owned paths, symlink escapes, absolute path escapes, and `..` path traversal must be denied.
  - Returned `changed_files`/`touched_paths` must be validated against owned paths before completion is accepted.
  - Read/spec/review profiles may be enabled before write support only if tool/write access remains disabled.
- Internal-agent `.pi/**` projection support
  - VIDA must generate Pi agent/projection files from configured carriers, runtime roles, model profiles, and prompt-stack rules.
  - Pi subagent capabilities are host affordances. Canonical delegated execution remains TaskFlow/`vida agent-init` with receipts.
  - Generated Pi agents must include child-agent recursion stop rules and must not instruct Pi child agents to launch their own subagents.
- Config template propagation
  - Live `vida.config.yaml` and canonical `docs/framework/templates/vida.config.yaml.template` must both be updated during config implementation.
  - Generated distribution/install asset templates must be refreshed by release/package flow, not manually edited as source.
- Release packaging/install/CI/smoke
  - Release build/install must include `vida-pi-agent` beside `vida`, including Windows `.exe` handling.
  - Smoke coverage must prove installed `vida` and installed `vida-pi-agent` are both resolvable.
  - CI/smoke should include a no-write Pi probe and bounded adapter tests; live Pi network/provider checks may be optional or gated when credentials are unavailable.

### Non-Functional Requirements
- Performance: normal status/selection/readiness surfaces must stay compact and fast; Pi model catalog probes should be bounded and cached only with explicit freshness invalidation.
- Scalability: model/provider additions should be data-driven through config profiles rather than code branches per model.
- Observability: readiness, selection, adapter execution, result parsing, write-scope validation, and release packaging must expose machine-readable blocker codes and artifact refs.
- Security: write-capable execution must be bounded by packet-owned paths; local `.pi/**` materialization must not silently widen permissions; raw provider output must be sanitized before being treated as receipt truth.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/pi-primary-environment-and-agent-carrier-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `active spec/catalog maps and Git history`
  - operator runbook/process docs added in the docs task
- Framework protocols affected:
  - host CLI setup/materialization protocol surfaces
  - external CLI carrier dispatch/readiness law
  - agent prompt-stack/materialization law where `.pi/**` projections are added
- Runtime families affected:
  - project activation
  - agent-init/dispatch execution
  - status/readiness
  - runtime assignment/model-profile selection
  - release/install/package
  - DocFlow only for documentation validation
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml`
  - `docs/framework/templates/vida.config.yaml.template`
  - generated install assets from release flow
  - `.pi/**` generated projection outputs
  - dispatch packets/results/receipts
  - status/readiness JSON
  - external CLI carrier smoke scripts

## Design Decisions

### 1. Separate primary host environment from external carrier
Will implement / choose:
- Model `host_environment.systems.pi` as the host CLI environment and `pi_cli` as an external executor carrier.
- Why: host materialization and execution dispatch have different lifecycles, authority surfaces, readiness checks, and proof requirements.
- Trade-offs: more config and readiness fields are required, but the split prevents `.pi/**` files or raw Pi defaults from overriding VIDA runtime truth.
- Alternatives considered: only add `pi_cli` as an external carrier. Rejected because the user explicitly wants Pi as the main environment and internal-agent support, not only an execution backend.

### 2. Use a Rust `vida-pi-agent` adapter
Will implement / choose:
- Add a VIDA-owned Rust binary that wraps Pi RPC/JSON behavior and emits canonical VIDA result JSON.
- Why: the adapter needs cross-platform packaging, structured timeouts, JSON parsing, write-scope enforcement, and release/install proof. Rust keeps it inside the same workspace and release pipeline.
- Trade-offs: slightly more implementation work than a script; stronger packaging and testing boundaries.
- Alternatives considered: shell/Node wrapper or raw `pi` dispatch. Rejected because raw Pi does not emit VIDA receipts and scripts are weaker for packaged release/install and write-scope enforcement.

### 3. Config and templates remain authority
Will implement / choose:
- Add Pi profiles to live config and canonical config template, then regenerate installed/dist assets during release.
- Why: new projects must inherit Pi support, and current projects must remain runtime-configurable.
- Trade-offs: more propagation work and template tests, but avoids hardcoded carriers and stale generated assets.
- Alternatives considered: update only live `vida.config.yaml`. Rejected because it would not bootstrap Pi in new VIDA projects.

### 4. Start read/spec/review first unless write guard exists
Will implement / choose:
- Allow early Pi readiness and read/spec/review probes only with no write-capable profile admission unless bounded write guard is present.
- Why: Pi is intended to become write-capable, but unbounded external writes violate VIDA safety semantics.
- Trade-offs: implementation backend rollout may happen in two steps; avoids unsafe writes during adapter bring-up.
- Alternatives considered: grant Pi full filesystem writes immediately. Rejected as unsafe and contrary to packet-owned write-scope law.

### 5. Full write capability requires owned-path guard and touched-path validation
Will implement / choose:
- Admit write-capable Pi profiles only when the dispatch packet carries owned paths and the adapter enforces/validates touched paths.
- Why: external carriers must not escape the active packet scope or silently mutate unrelated project files.
- Trade-offs: the adapter may need either a Pi extension/tool guard or a post-run validation layer plus restricted tool environment. The final implementation must choose the safest feasible enforcement seam.
- Alternatives considered: rely only on Pi prompt instructions. Rejected because prompts are not enforceable write guards.

## Technical Design

### Core Components
- `host_environment.systems.pi`
  - Project activation/materialization entry for Pi.
  - Owns Pi host CLI setup/readiness projection and `.pi/**` output root mapping.
- `pi_cli` carrier/profile catalog
  - External CLI backend entry with runtime roles, task classes, model profiles, cost units, lifecycle status, write posture, and readiness fields.
- `vida-pi-agent`
  - Rust adapter binary.
  - Inputs: selected model/profile, thinking level, dispatch packet/prompt, owned paths, timeout, no-session/process options.
  - Outputs: VIDA external result JSON and optional adapter metadata.
- Runtime assignment/readiness
  - Extends existing carrier/model-profile selection and external CLI readiness rather than adding Pi-only selection logic.
- Dispatch execution/parser
  - Reuses canonical external result parsing after the adapter normalizes Pi output.
- Pi projection materializer
  - Generates `.pi/settings.json`, `.pi/agents/*.md`, and optional `.pi/chains/*.chain.md` from VIDA config/runtime truth.
- Release/package installer
  - Builds and installs both `vida` and `vida-pi-agent`; refreshes scaffold/template assets.

### Data / State Model
- Config fields:
  - `host_environment.systems.pi`
  - `agent_system.subagents.pi_cli`
  - carrier/model profiles such as low/medium/high Pi profiles with provider, model ref, thinking level, normalized cost units, runtime roles, task classes, readiness, and write scope.
- Dispatch packet fields:
  - selected backend/carrier/profile
  - model ref/provider
  - thinking/reasoning level
  - owned paths/write scope
  - timeout/process mode
- Adapter result fields:
  - `type: "result"`
  - `subtype: "success" | "error_during_execution"`
  - `is_error`
  - `result` or `error.message`
  - `raw_provider.provider: "pi"`
  - `raw_provider.mode: "rpc"`
  - terminal event, selected model/profile, usage if available, touched paths when applicable
- Receipts/runtime state:
  - external dispatch result/receipt
  - readiness/preflight evidence
  - write-scope validation evidence
  - release install manifest for adapter binary
- Migration/compatibility notes:
  - existing projects without `host_environment.systems.pi` remain valid.
  - Pi support is additive; missing Pi readiness blocks only Pi carrier admission, not all VIDA operation.

### Integration Points
- Project activation:
  - Pi host CLI selection and materialization.
- Agent system projection:
  - Pi internal-agent file generation from configured roles/carriers.
- Runtime assignment:
  - dynamic Pi model/profile selection through existing selection policy.
- Status/readiness:
  - `pi_cli` readiness and Pi host environment status.
- Dispatch:
  - adapter command rendering and external result parsing.
- Release/install:
  - binary packaging and template propagation.
- Documentation:
  - this design, config template docs, and operator runbook.

### Bounded File Set
- Phase 1: specification/design
  - `docs/product/spec/pi-primary-environment-and-agent-carrier-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `active spec/catalog maps and Git history`
- Phase 2: config/template profiles
  - `vida.config.yaml`
  - `docs/framework/templates/vida.config.yaml.template`
  - config/template tests and generated release assets through release flow
- Phase 3: host environment and Pi projections
  - project activator/materialization modules
  - host runtime materialization modules
  - Pi projection templates/resources
  - `.pi/**` generated outputs in smoke fixtures only where expected
- Phase 4: adapter
  - workspace manifest(s)
  - `crates/vida-pi-agent/**` or equivalent Rust binary target
  - adapter tests/fixtures
- Phase 5: runtime selection/readiness/dispatch
  - runtime assignment modules
  - external CLI readiness/status modules
  - dispatch state/execution modules
  - external result parsing tests
- Phase 6: write-scope guard
  - adapter guard implementation
  - dispatch packet/write-scope validation modules
  - security/path tests
- Phase 7: release/package/smoke/docs
  - release/install modules
  - installer/package manifests
  - adapter/runtime contract tests
  - operator runbook/process docs
  - CI test definitions if present

## Fail-Closed Constraints
- Raw `pi` command output must not be treated as VIDA execution evidence.
- `vida-pi-agent` must not complete successfully without terminal Pi execution evidence.
- Activation/view-only `agent-init` output must not count as Pi execution completion.
- Pi write-capable profiles must be inadmissible until write-scope guard and touched-path validation are implemented and ready.
- Missing/invalid Pi model, missing command, missing adapter, auth unavailable, unsupported thinking level, timeout, malformed provider output, or write-scope validation failure must return blocked/error result JSON.
- Pi local `.pi/**` files must not become a second source of carrier/profile truth.
- Generated dist/install templates must not be manually edited as source; release flow must refresh them.
- No long-lived Pi daemon/session is allowed for VIDA dispatch execution.

## Implementation Plan

### Phase 1
- Task: `feature-vida-pi-agent-primary-environment-spec`
- Create and register this design/TZ.
- First proof target:
  - `vida docflow check --root . docs/product/spec/pi-primary-environment-and-agent-carrier-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Task: `feature-vida-pi-agent-config-and-template-profiles`
- Add Pi config/profile catalog to live config and canonical template.
- First implementation proof target:
  - config projection tests show Pi profiles and template parity.

### Phase 3
- Task: `feature-vida-pi-agent-host-system-materialization`
- Add Pi host environment selection/materialization.
- Proof target:
  - project activation/init surfaces report Pi host system readiness/materialization truth.

### Phase 4
- Task: `feature-vida-pi-agent-internal-agent-projections`
- Generate `.pi/**` projections for VIDA roles/carriers/profiles.
- Proof target:
  - projection tests show `.pi` files mirror config and include recursion stop rules.

### Phase 5
- Task: `feature-vida-pi-agent-adapter-binary`
- Implement `vida-pi-agent` adapter binary.
- Proof target:
  - adapter unit/integration tests for success, invalid model, timeout, malformed output, and process exit.

### Phase 6
- Task: `feature-vida-pi-agent-carrier-runtime-selection`
- Wire Pi into runtime assignment.
- Proof target:
  - runtime selection chooses eligible Pi profiles and rejects ineligible profiles.

### Phase 7
- Task: `feature-vida-pi-agent-readiness-preflight`
- Add status/readiness preflight.
- Proof target:
  - `vida status --json` reports Pi readiness and blocker codes truthfully.

### Phase 8
- Task: `feature-vida-pi-agent-receipt-backed-dispatch`
- Normalize dispatch evidence and receipts.
- Proof target:
  - `vida agent-init --execute-dispatch --json` records parseable Pi-backed external result or fail-closed error.

### Phase 9
- Task: `feature-vida-pi-agent-bounded-write-scope`
- Add bounded write-scope guard and touched-path validation.
- Proof target:
  - in-scope write allowed, out-of-scope write denied, symlink/path traversal denied.

### Phase 10
- Task: `feature-vida-pi-agent-release-packaging`
- Package/install `vida-pi-agent` and regenerate templates/assets.
- Proof target:
  - installed `vida` and `vida-pi-agent` resolve correctly; package includes Pi config template.

### Phase 11
- Task: `feature-vida-pi-agent-smoke-and-ci`
- Add smoke/CI proof matrix.
- Proof target:
  - adapter contract tests cover fake-provider success/failure; optional live provider probe is explicit operator work, not a generic hardcoded smoke script.

### Phase 12
- Task: `feature-vida-pi-agent-docs-runbook-and-closure`
- Add operator docs/runbook and close epic after final proof.
- Final proof target:
  - docflow checks, targeted tests, release install, runtime self-diagnostic, and no unresolved Pi blockers.

## Implementation Closure Notes

As of the `feature-vida-pi-agent-prewrite-tool-guard` slice, the epic implementation includes a real pre-write guarded-write path for Pi write profiles:

- `pi_cli` dispatch uses `vida-pi-agent`, a VIDA-owned one-shot adapter process, rather than raw `pi` output.
- `host_environment.systems.pi` and `.pi/**` projection support are materialized from VIDA config/runtime truth; Pi-local files are not authority.
- Release/install packaging includes `vida-pi-agent` beside `vida`, `taskflow`, and `docflow`; generated install templates are refreshed through release flow.
- Smoke/CI coverage includes adapter tests and installed adapter help checks; optional live Pi/provider smoke remains an explicit operator action outside the generic CI/local gate.
- The bounded write-scope guard now has two layers: a pre-write Pi extension loaded explicitly by `vida-pi-agent` in `guarded-write` mode, plus post-execution touched-path validation/evidence. The extension receives canonical guard data from adapter-owned environment variables, denies `write`/`edit` outside dispatch owned paths before tool execution, blocks `bash`/user bash to prevent shell write bypass, and blocks unknown mutating tools.
- Profiles requiring `guard_required_owned_paths` are implementation-admissible only when readiness reports the adapter pre-write guard active. Read/review profiles must not write.

Concise proof commands recorded across the implementation slices:

- `cargo nextest run --locked -p vida-pi-agent --profile default`
- `cargo test -p vida parse_external_provider_output -- --nocapture`
- `cargo test -p vida guard_required_write_scope -- --nocapture`
- `cargo test -p vida release_install -- --nocapture`
- `cargo build -p vida-pi-agent --bins --locked`
- `vida docflow check-file --path docs/process/external-cli-carrier-operator-procedure.md`
- `vida docflow check-file --path docs/process/agent-system.md`
- `vida docflow check-file --path docs/product/spec/pi-primary-environment-and-agent-carrier-design.md`

## Validation / Proof
- Unit tests:
  - config/template Pi profile parsing and parity
  - runtime assignment eligibility/rejection
  - readiness blocker classification
  - adapter result parsing
  - adapter timeout/error normalization
  - write-scope path canonicalization and escape denial
  - projection generation for `.pi/**`
- Integration tests:
  - project activation with Pi host system
  - agent-system snapshot includes `pi_cli`
  - external dispatch command rendering uses `vida-pi-agent`
  - Pi dispatch success/failure receipts are parsed by VIDA
  - release install includes adapter binary
- Runtime checks:
  - `vida status --json`
  - `vida taskflow consume agent-system --json`
  - `vida agent select --runtime-role worker --task-class implementation --json`
  - `vida agent-init --execute-dispatch --json` with a bounded Pi smoke packet when credentials/readiness allow
  - `vida-pi-agent --version` or equivalent adapter smoke
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/pi-primary-environment-and-agent-carrier-design.md docs/product/spec/current-spec-map.md`
  - `vida docflow check --root . <operator-docs>` after runbook work
  - `vida docflow readiness-check --profile active-canon` when template/readiness law changes
  - runtime self-diagnostic after release/install

## Observability
- Logging points:
  - adapter process start/exit and timeout classification
  - selected provider/model/profile/thinking level
  - readiness probe status and blocker family
  - write-scope guard denial with sanitized path reason
- Metrics / counters:
  - Pi dispatch attempts/success/failure/timeouts
  - readiness probe freshness
  - write-scope denial count
  - selected Pi profile cost units where existing runtime telemetry supports it
- Receipts / runtime state written:
  - external dispatch result receipt
  - readiness/preflight projection
  - write-scope validation evidence
  - release install manifest entry for `vida-pi-agent`
  - Pi projection materialization receipt where project activation/materialization already records host files

## Rollout Strategy
- Development rollout:
  - Land design first.
  - Add config/template rows while keeping Pi blocked until adapter/readiness exists.
  - Add host environment materialization and `.pi/**` projections.
  - Add adapter and read-only smoke.
  - Add runtime selection/readiness.
  - Enable write-capable profiles only after guard proof.
  - Package/install and run smoke/diagnostic.
- Migration / compatibility notes:
  - Existing projects without Pi config remain valid.
  - New projects receive Pi rows from canonical config template after release assets refresh.
  - Pi may be installed but still readiness-blocked when auth/model/guard is missing.
- Operator or user restart / restart-notice requirements:
  - After release/install, operators may need a new shell or PATH refresh so `vida-pi-agent` is discoverable.
  - Pi auth/provider setup remains operator-owned and must be reported as readiness state, not silently inferred.

## Future Considerations
- Add richer Pi provider/model catalog synchronization if Pi exposes stable machine-readable model metadata beyond RPC `get_available_models`.
- Add provider-specific pricing freshness integration if Pi can expose provider/model pricing or if VIDA maintains external price schedules.
- Add optional remote/non-local Pi execution only if it can preserve VIDA packet, receipt, and write-scope law.
- Add richer Pi chain projection only after single-dispatch semantics are proven and recursion/long-lived-session risks are closed.

## References
- Related specs:
  - `docs/product/spec/external-cli-carrier-hardening-design.md`
  - `docs/product/spec/carrier-model-profile-selection-runtime-model.md`
  - `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
  - `docs/product/spec/host-agent-layer-status-matrix.md`
  - `docs/product/spec/hybrid-host-executor-semantics-model.md`
  - `docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md`
- Related process docs:
  - `docs/process/agent-system.md`
  - `docs/process/external-cli-carrier-operator-procedure.md`
  - `docs/process/team-development-and-orchestration-protocol.md`
  - `docs/process/documentation-tooling-map.md`
- Related TaskFlow work:
  - Epic `feature-vida-pi-agent-primary-environment`
  - Task `feature-vida-pi-agent-primary-environment-spec`
- External/Pi local docs consulted in prior research:
  - Pi README
  - Pi RPC docs
  - Pi JSON docs
  - Pi extensions docs

-----
artifact_path: product/spec/pi-primary-environment-and-agent-carrier-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-02
schema_version: 1
status: canonical
source_path: docs/product/spec/pi-primary-environment-and-agent-carrier-design.md
created_at: 2026-05-19T15:27:13.8001375Z
updated_at: 2026-06-02T03:05:00+03:00
changelog_ref: pi-primary-environment-and-agent-carrier-design.changelog.jsonl
