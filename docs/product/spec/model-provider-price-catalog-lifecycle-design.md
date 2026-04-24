# Model Provider Price Catalog Lifecycle Design

Status: proposed

## Summary
- Feature / change: define the first-class model/provider price-catalog lifecycle contract for source-of-truth ownership, provider/model availability inventory, freshness metadata, update receipts, readiness/status projection, and price-aware candidate diagnostics
- Owner layer: `mixed`
- Runtime surface: `project activation | init | status | taskflow`
- Status: `proposed`

## Current Context
- Existing system overview
  - `vida.config.yaml` already carries carrier and backend `model_profiles` with `provider`, `model_ref`, `normalized_cost_units`, and readiness metadata, plus `agent_system.model_selection` policy and `agent_system.pricing.vendor_basis`.
  - `crates/vida/src/runtime_assignment_builder.rs` already selects candidates by role/task/readiness/score/cost and emits `selected_model_profile_id`, `selected_model_ref`, `budget_verdict`, and `rejected_candidates`.
  - `crates/vida/src/taskflow_routing.rs` already projects `selected_candidate`, `candidate_pool`, `budget_verdict`, and `rejected_candidates`, while explicitly marking some route knobs as diagnostic-only or rejected without a runtime consumer.
  - `crates/vida/src/status_surface_external_cli.rs` already projects backend readiness, selected/default model profile truth, and provider-failure detection.
- Key components and relationships
  - current price-like truth is split across:
    - `vida.config.yaml -> host_environment.*.carriers.*.model_profiles.*.normalized_cost_units`
    - `vida.config.yaml -> agent_system.subagents.*.model_profiles.*.normalized_cost_units`
    - `vida.config.yaml -> agent_system.pricing.vendor_basis`
  - project activation law in `docs/product/spec/project-activation-and-configurator-model.md` says long-term active runtime truth belongs under DB-first `.vida/project/**`, while root `vida.config.yaml` remains a bridge/export surface.
  - status-family law in `docs/product/spec/status-families-and-query-surface-model.md` says operator summaries must come from bounded query surfaces rather than ad hoc narration.
- Current pain point or gap
  - there is no canonical price-catalog contract separating:
    - static normalized cost units used for selection,
    - provider/model availability inventory,
    - freshness/source metadata,
    - explicit dry-run/apply update receipts,
    - readiness/status surfaces for stale or missing price data.
  - runtime can explain selected versus rejected candidates, but it cannot explain whether a price is fresh, sourced, stale, manually overridden, or absent by policy.
  - operator-facing surfaces can show readiness and selected model profile truth, but there is no equivalent first-class surface for price-catalog readiness or update lineage.

## Goal
- What this change should achieve
  - define one canonical lifecycle for provider/model price catalog data without widening the hot path into live network lookup
  - make price metadata queryable and receipt-backed
  - distinguish fail-closed pricing blockers from diagnostic-only price visibility gaps
  - align selection diagnostics, init/status readiness, and project-activation ownership around one contract
- What success looks like
  - one canonical config/runtime contract exists for provider inventory, model availability, price records, freshness metadata, and update receipts
  - runtime can cite selected and rejected candidate pricing truth from a bounded inventory snapshot
  - init/status surfaces can report whether pricing data is ready, stale, blocked, or advisory-only
  - explicit update flows produce dry-run/apply receipts instead of mutating price data invisibly
- What is explicitly out of scope
  - implementing provider API fetchers in this slice
  - changing current runtime selection code in this task
  - replacing `normalized_cost_units` with direct currency arithmetic on the dispatch hot path

## Requirements

### Functional Requirements
- Must-have behavior
  - define the bridge source of truth and the long-term runtime source of truth for price catalog data
  - define a provider/model availability inventory that is separate from route-selection receipts
  - define freshness/source metadata for every price-bearing record
  - define explicit dry-run and apply update receipts
  - define readiness/status fields for init and status-family surfaces
  - define fail-closed versus diagnostic-only handling for stale, missing, or ambiguous price data
  - define how selected and rejected candidate diagnostics cite price-catalog truth
- Integration points
  - `vida.config.yaml`
  - future `.vida/project/**` exported model-policy surfaces under DB-first activation
  - `vida project-activator`, `vida orchestrator-init`, `vida agent-init`, `vida status --json`
  - `vida taskflow route explain --json`
  - `vida taskflow validate-routing --json`
- User-visible or operator-visible expectations
  - operators can tell which provider/model entries are available, blocked, stale, or manually overridden
  - operators can see whether runtime cost/price truth came from imported catalog data or legacy profile-only fallback
  - candidate rejection reasons can explicitly cite price/availability/freshness blockers when policy requires them

### Non-Functional Requirements
- Performance
  - runtime selection must consume local catalog snapshots only; no live provider fetch is allowed on the hot path
- Scalability
  - one provider may expose many models and multiple price records without bespoke per-provider schema branches
- Observability
  - every catalog update must be receipt-backed and queryable
- Security
  - stale or unverifiable price data must fail closed where policy requires explicit price truth

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/model-provider-price-catalog-lifecycle-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/README.md`
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/status-families-and-query-surface-model.md`
  - `docs/product/spec/carrier-model-profile-selection-runtime-design.md`
  - `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
- Framework protocols affected:
  - none in this slice beyond existing activation/status/query law
- Runtime families affected:
  - `project activation`
  - `init`
  - `status`
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - bridge source: root `vida.config.yaml`
  - runtime/project source: future `.vida/project/model-policy/price-catalog.yaml` plus DB-first state
  - future `price_catalog_update_receipt`
  - future `price_catalog_snapshot_receipt`
  - init/status/taskflow query JSON that cites price readiness and candidate pricing evidence

## Design Decisions

### 1. Root `vida.config.yaml` remains the bridge authoring surface; DB-first project policy becomes the runtime authority
Will implement / choose:
- define two layers of truth:
  - bridge authoring/import surface: root `vida.config.yaml`
  - active runtime/project truth: DB-backed project activation state with exported mirror under `.vida/project/model-policy/price-catalog.yaml`
- Why
  - this matches project-activation law: root config remains bridge-compatible, while active runtime authority converges under `.vida/project/**`
- Trade-offs
  - the system must expose sync/import status so operators can see when bridge config and runtime catalog drift
- Alternatives considered
  - keep price catalog permanently owned only by root `vida.config.yaml`
  - make selection depend directly on ad hoc provider-side state files
- ADR link if this must become a durable decision record
  - none

### 2. Price catalog is a first-class inventory, not just `normalized_cost_units` scattered inside model profiles
Will implement / choose:
- define explicit entities:
  - `provider_catalog_entry`
  - `model_catalog_entry`
  - `price_record`
  - `availability_record`
  - `price_catalog_snapshot`
- keep `normalized_cost_units` as the hot-path selection field, but require it to cite its catalog/source lineage
- Why
  - current runtime already uses normalized cost data, but there is no contract for where that value came from or how fresh it is
- Trade-offs
  - there will be a bridge period where model profiles keep compatibility cost fields while canonical catalog records become authoritative
- Alternatives considered
  - continue treating every profile-local `normalized_cost_units` field as standalone truth
- ADR link if needed
  - none

### 3. Price freshness and source metadata must be explicit on every price-bearing record
Will implement / choose:
- require each price record to carry:
  - `source_kind`
  - `source_ref`
  - `observed_at`
  - `freshness_ttl_seconds`
  - `freshness_status`
  - `collected_by`
  - `catalog_snapshot_id`
- Why
  - runtime must distinguish trusted fresh prices from stale manual or imported fallback values
- Trade-offs
  - some providers will initially be `manual_seed` or `imported_snapshot` rather than live provider-native feeds
- Alternatives considered
  - infer freshness only from file mtime or changelog age
- ADR link if needed
  - none

### 4. Catalog updates must be receipt-backed with explicit dry-run/apply split
Will implement / choose:
- define two receipt classes:
  - `price_catalog_update_receipt` with `status=dry_run|applied|blocked`
  - `price_catalog_snapshot_receipt` for the activated catalog snapshot/runtime projection
- require dry-run receipts to enumerate proposed provider/model/price changes without mutating active runtime state
- require apply receipts to record accepted changes, rejected changes, source digest, and resulting snapshot id
- Why
  - price/cost changes alter runtime selection posture and must be inspectable before becoming active
- Trade-offs
  - update tooling must maintain explicit change sets instead of mutating inventory inline
- Alternatives considered
  - direct in-place config edits with no receipt lineage
- ADR link if needed
  - none

### 5. Price data has both fail-closed and diagnostic-only modes, chosen by policy class
Will implement / choose:
- fail closed when:
  - a selected candidate lacks required `normalized_cost_units` lineage
  - required price records are stale past policy TTL
  - provider/model availability is explicitly blocked
  - update/apply receipt lineage is missing for a claimed active snapshot
- diagnostic only when:
  - a candidate is free/internal and policy does not require external currency-backed price freshness
  - vendor-basis prose is incomplete but normalized selection cost remains present and sourced
  - freshness is advisory for non-spend-driving exploratory summaries
- Why
  - some price truth is safety-critical for selection/admission, while some is only operator visibility
- Trade-offs
  - runtime/status surfaces must say which rule class was applied instead of one generic warning
- Alternatives considered
  - make all stale price data either always block or always advisory
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - bridge config catalog section in root `vida.config.yaml`
  - DB-first project catalog state and exported `.vida/project/model-policy/price-catalog.yaml`
  - price-catalog snapshot/query surface consumed by init/status/taskflow
  - update/apply receipt family for price imports and reconciliations
- Key interfaces
  - `provider_catalog_entry`
  - `model_catalog_entry`
  - `price_record`
  - `availability_record`
  - `price_catalog_update_receipt`
  - `price_catalog_snapshot_receipt`
  - `price_catalog_readiness`
- Bounded responsibilities
  - project activation owns import/sync/apply lifecycle
  - status/init surfaces own readiness rendering
  - taskflow selection surfaces own selected/rejected pricing diagnostics
  - route explain/validate surfaces own policy-visible selection evidence, not catalog mutation

### Data / State Model
- Important entities
  - `provider_id`
  - `model_ref`
  - `availability_status`
  - `normalized_cost_units`
  - `billing_unit`
  - `currency_code`
  - `source_kind`
  - `freshness_status`
  - `catalog_snapshot_id`
  - `update_receipt_id`
- Receipts / runtime state / config fields
  - bridge config family:
    - `agent_system.pricing_catalog.providers`
    - `agent_system.pricing_catalog.models`
    - `agent_system.pricing_catalog.policy`
  - runtime/project family:
    - `.vida/project/model-policy/price-catalog.yaml`
    - DB-backed `price_catalog_snapshot`
  - receipt fields:
    - `receipt_id`
    - `status`
    - `dry_run`
    - `source_digest`
    - `applied_snapshot_id`
    - `proposed_changes`
    - `applied_changes`
    - `rejected_changes`
    - `blocker_codes`
  - query/readiness fields:
    - `price_catalog_readiness.enabled`
    - `price_catalog_readiness.source_mode`
    - `price_catalog_readiness.active_snapshot_id`
    - `price_catalog_readiness.freshness_status`
    - `price_catalog_readiness.stale_provider_count`
    - `price_catalog_readiness.stale_model_count`
    - `price_catalog_readiness.missing_price_count`
    - `price_catalog_readiness.blocked_provider_count`
    - `price_catalog_readiness.fail_closed`
    - `price_catalog_readiness.next_actions`
- Migration or compatibility notes
  - existing profile-local `normalized_cost_units` remain readable compatibility input
  - canonical catalog rows become the source of lineage and freshness truth for those cost fields
  - bridge mode is explicit until project activation imports and activates the catalog snapshot

### Integration Points
- APIs
  - future project-activation/config query JSON for price catalog status
  - `vida status --json`
  - `vida orchestrator-init --json`
  - `vida agent-init --json`
  - `vida taskflow route explain --json`
  - `vida taskflow validate-routing --json`
- Runtime-family handoffs
  - bridge config import -> project activation catalog snapshot
  - active catalog snapshot -> init/status readiness rendering
  - active catalog snapshot -> runtime assignment selected/rejected candidate diagnostics
  - dry-run/apply update -> receipt lineage -> active snapshot promotion
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/status-families-and-query-surface-model.md`
  - `docs/product/spec/carrier-model-profile-selection-runtime-design.md`
  - `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
  - `docs/product/spec/release-1-operator-surface-contract.md`

### Operator Command Contract
- Command family
  - add one bounded TaskFlow command home: `vida taskflow pricing`
  - keep `vida status --json`, `vida orchestrator-init --json`, and `vida agent-init --json` as the operator-facing summary surfaces that project pricing readiness, not mutation surfaces
  - keep `vida taskflow route explain --json` and `vida taskflow validate-routing --json` as the selection-diagnostics surfaces that cite pricing evidence
- Help semantics
  - `vida taskflow help pricing` must explain:
    - source-of-truth split: bridge config versus active runtime snapshot
    - query commands versus mutation commands
    - dry-run versus apply semantics
    - failure classes: fail-closed blockers versus diagnostic-only warnings
    - the minimum JSON fields emitted by readiness, receipt, and diagnostics commands
  - `vida taskflow pricing --help` must list only the bounded pricing/catalog commands and fail closed on unsupported verbs
- Exact desired commands
  - readiness/query:
    - `vida taskflow pricing status --json`
    - `vida taskflow pricing status --summary --json`
    - `vida taskflow pricing providers --json`
    - `vida taskflow pricing models --provider <provider-id> --json`
    - `vida taskflow pricing receipt <receipt-id> --json`
    - `vida taskflow pricing receipts latest --json`
  - refresh/update:
    - `vida taskflow pricing refresh --provider <provider-id> --dry-run --json`
    - `vida taskflow pricing refresh --provider <provider-id> --apply --json`
    - `vida taskflow pricing refresh --all --dry-run --json`
    - `vida taskflow pricing refresh --all --apply --json`
    - `vida taskflow pricing import --source-file <path> --dry-run --json`
    - `vida taskflow pricing import --source-file <path> --apply --json`
  - selection diagnostics:
    - `vida taskflow route explain --task-class <task-class> --runtime-role <runtime-role> --pricing --json`
    - `vida taskflow validate-routing --pricing --json`
- Exact desired options
  - shared query options:
    - `--json` for machine output
    - `--summary` for condensed readiness view on `status`
    - `--provider <provider-id>` to scope one provider
    - `--model <model-ref>` to scope one model row where applicable
    - `--active-snapshot` to force results from the activated snapshot only
  - refresh/import options:
    - `--dry-run` required for preview-only mutation planning
    - `--apply` required for state-changing update/import
    - `--source-file <path>` for imported catalog data
    - `--receipt-note <text>` optional operator annotation for receipts
    - `--fail-on-stale` to turn advisory stale rows into apply blockers
    - `--max-age-seconds <n>` to override default freshness tolerance for the current refresh
  - diagnostics options:
    - `--pricing` to request pricing-specific diagnostics in route explain/validate output
    - `--include-rejected` to emit full rejected-candidate pricing rows rather than only summary counts
    - `--candidate-limit <n>` to bound candidate detail volume
- Exact help/behavior rules
  - `--dry-run` and `--apply` are mutually exclusive
  - mutation verbs fail closed if neither `--dry-run` nor `--apply` is passed
  - `refresh --all` and `refresh --provider <provider-id>` are mutually exclusive
  - `import` requires `--source-file`
  - readiness/query commands are read-only and must never materialize a new active snapshot implicitly
  - `route explain --pricing` and `validate-routing --pricing` are diagnostic-only; they must not refresh or mutate catalog state
- Exact readiness/status fields
  - `vida taskflow pricing status --json` must emit:
    - `price_catalog_readiness.enabled`
    - `price_catalog_readiness.source_mode`
    - `price_catalog_readiness.active_snapshot_id`
    - `price_catalog_readiness.latest_receipt_id`
    - `price_catalog_readiness.freshness_status`
    - `price_catalog_readiness.stale_provider_count`
    - `price_catalog_readiness.stale_model_count`
    - `price_catalog_readiness.missing_price_count`
    - `price_catalog_readiness.blocked_provider_count`
    - `price_catalog_readiness.fail_closed`
    - `price_catalog_readiness.diagnostic_only_warnings`
    - `price_catalog_readiness.next_actions`
  - `vida status --json`, `vida orchestrator-init --json`, and `vida agent-init --json` should project the same `price_catalog_readiness` object as a bounded summary rather than a second schema
- Exact receipt fields
  - `vida taskflow pricing refresh ... --dry-run --json` and `import ... --dry-run --json` must emit `price_catalog_update_receipt` with:
    - `receipt_id`
    - `status: "dry_run"`
    - `requested_scope`
    - `source_digest`
    - `proposed_changes`
    - `rejected_changes`
    - `blocker_codes`
    - `would_activate_snapshot_id`
    - `next_actions`
  - `vida taskflow pricing refresh ... --apply --json` and `import ... --apply --json` must emit:
    - `receipt_id`
    - `status: "applied" | "blocked"`
    - `applied_snapshot_id`
    - `applied_changes`
    - `rejected_changes`
    - `freshness_status`
    - `blocker_codes`
    - `next_actions`
- Exact selected/rejected candidate pricing diagnostics
  - `vida taskflow route explain --pricing --json` and `vida taskflow validate-routing --pricing --json` must extend existing candidate diagnostics with:
    - `selected_candidate.price_catalog_snapshot_id`
    - `selected_candidate.price_source_kind`
    - `selected_candidate.price_freshness_status`
    - `selected_candidate.normalized_cost_units_source`
    - `selected_candidate.pricing_diagnostic_class`
    - `rejected_candidates[*].price_catalog_snapshot_id`
    - `rejected_candidates[*].price_freshness_status`
    - `rejected_candidates[*].pricing_reasons`
    - `rejected_candidates[*].pricing_fail_closed`
  - allowed `pricing_reasons` include:
    - `price_missing`
    - `price_stale`
    - `provider_blocked`
    - `availability_unknown`
    - `receipt_lineage_missing`
    - `diagnostic_only_price_gap`
- Operator-step reduction contract
  - current operator flow requires piecing together `vida status --json`, route diagnostics, and ad hoc config inspection
  - the pricing command family reduces that to:
    - 1 step for readiness: `vida taskflow pricing status --json`
    - 1 step for preview: `vida taskflow pricing refresh --all --dry-run --json`
    - 1 step for activation: `vida taskflow pricing refresh --all --apply --json`
    - 1 step for selection evidence: `vida taskflow route explain --task-class <task-class> --runtime-role <runtime-role> --pricing --json`
  - reduction rule:
    - operators should not need to manually inspect root config, profile rows, and status surfaces separately to answer whether pricing is fresh, applicable, and selection-relevant
  - fail-closed rule:
    - if a single pricing command cannot answer the bounded question because required snapshot or receipt truth is missing, the command must emit explicit blockers and next actions rather than silently telling the operator to inspect unrelated surfaces

### Bounded File Set
- `docs/product/spec/model-provider-price-catalog-lifecycle-design.md`
- `docs/product/spec/model-provider-price-catalog-lifecycle-design.changelog.jsonl`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/README.md`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no silent fallback from stale or blocked catalog truth to unsourced `normalized_cost_units`
  - no implicit live-provider lookup on the dispatch hot path
  - no claim that an active snapshot is authoritative when no apply receipt or snapshot receipt exists
  - no collapsing provider availability, pricing freshness, and model-profile readiness into one ambiguous boolean
- Required receipts / proofs / gates
  - dry-run update receipts must enumerate proposed changes without mutating active state
  - apply receipts must cite resulting snapshot id and rejected/blocking changes
  - init/status surfaces must expose `price_catalog_readiness`
  - selected and rejected candidates must be able to cite price-source/freshness evidence once implemented
- Safety boundaries that must remain true during rollout
  - runtime selection continues to use bounded local cost metadata
  - bridge config remains readable during migration
  - selection diagnostics keep separate fields for model-profile readiness versus price-catalog freshness

## Implementation Plan

### Phase 1
- land this design doc and register it in the active spec canon
- define canonical schema rows for provider inventory, model inventory, price records, freshness metadata, and update receipts
- First proof target
  - active docs explicitly define bridge source, runtime source, receipt classes, and readiness fields without widening into code changes

### Phase 2
- add runtime/project activation contract docs for import/apply/snapshot lifecycle
- define operator-facing readiness/status and route-explain field additions
- Second proof target
  - init/status/taskflow docs agree on `price_catalog_readiness` and candidate pricing diagnostics

### Phase 3
- wire future implementation work to consume active catalog snapshots, emit receipts, and enforce fail-closed rules where policy requires it
- Final proof target
  - runtime surfaces can show selected/rejected candidate pricing evidence and receipt-backed snapshot freshness

## Validation / Proof
- Unit tests:
  - future runtime proof should cover stale-price rejection, advisory-only price gaps, and receipt-backed snapshot promotion
- Integration tests:
  - future operator proof should cover route explain/validate output, init/status readiness projection, and dry-run/apply receipt shapes
- Runtime checks:
  - current evidence anchor: `vida taskflow validate-routing --json`
  - current evidence anchor: `vida agent-init --json --role business_analyst "<request>"`
  - future runtime check: price catalog status/query surface over active snapshot truth
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/model-provider-price-catalog-lifecycle-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md docs/product/spec/README.md`
  - `vida docflow finalize-edit docs/product/spec/model-provider-price-catalog-lifecycle-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md docs/product/spec/README.md "record model/provider price catalog lifecycle design"`

## Observability
- Logging points
  - future price-catalog import/apply operations should log snapshot id, receipt id, and blocker counts
- Metrics / counters
  - `stale_provider_count`
  - `stale_model_count`
  - `missing_price_count`
  - `price_catalog_blocked_update_count`
- Receipts / runtime state written
  - `price_catalog_update_receipt`
  - `price_catalog_snapshot_receipt`
  - active `price_catalog_readiness` query projection

## Rollout Strategy
- Development rollout
  - docs/spec first
  - future implementation must add bridge import, snapshot query, update receipts, and status/taskflow projection in bounded slices
- Migration / compatibility notes
  - profile-local `normalized_cost_units` stay readable during bridge mode
  - runtime selection may remain partially diagnostic-only for price freshness until catalog snapshots become active
- Operator or user restart / restart-notice requirements
  - none for this docs-only slice

## Future Considerations
- Follow-up ideas
  - separate provider availability outages from pricing freshness outages in operator surfaces
  - add explicit `manual_override_reason` and review cadence for manually seeded prices
  - add provider/model deprecation windows so selection can warn before hard removal
- Known limitations
  - this document defines the lifecycle contract only; it does not implement the query/update surfaces yet
  - current runtime still relies on profile-local cost metadata rather than catalog-backed snapshot truth
- Technical debt left intentionally
  - `agent_system.pricing.vendor_basis` remains prose-only until catalog-backed policy fields replace its ambiguous role

## References
- Related specs
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/status-families-and-query-surface-model.md`
  - `docs/product/spec/carrier-model-profile-selection-runtime-design.md`
  - `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
- Related protocols
  - `docs/process/documentation-tooling-map.md`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/model-provider-price-catalog-lifecycle-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-24'
schema_version: '1'
status: canonical
source_path: docs/product/spec/model-provider-price-catalog-lifecycle-design.md
created_at: '2026-04-24T00:00:00Z'
updated_at: 2026-04-24T16:14:03.947458343Z
changelog_ref: model-provider-price-catalog-lifecycle-design.changelog.jsonl
