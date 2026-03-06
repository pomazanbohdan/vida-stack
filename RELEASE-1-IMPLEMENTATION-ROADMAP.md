# Vida Stack Release 1 Implementation Roadmap

Purpose: define the step-by-step implementation roadmap for `Vida Stack` Release 1.

This file sits between:

1. [README.MD](README.MD) as framework vision and architecture narrative
2. [RELEASE-1-SCOPE.MD](RELEASE-1-SCOPE.MD) as full Release 1 target capability contract

Rule:

1. `README.MD` explains why the framework exists and where it is going.
2. `RELEASE-1-SCOPE.MD` defines what Release 1 must contain.
3. This roadmap defines how Release 1 should be delivered incrementally.

This is an execution-order document, not a current-state report.

## Roadmap Goal

Release 1 should be delivered by finishing the mechanics that make Vida Stack trustworthy on real engineering work before attempting full platform extraction or Rust reimplementation.

The roadmap is designed to:

1. prioritize runtime integrity over feature count,
2. reduce protocol/runtime drift early,
3. stabilize orchestration before adding broader autonomy,
4. build reusable machine-visible artifacts,
5. leave advanced platform ambitions for later phases unless they directly improve current execution.

## Delivery Strategy

Release 1 should be built in ordered layers.

The dependency logic is:

1. state and execution integrity first,
2. orchestration reliability second,
3. verification and governance third,
4. telemetry and learning fourth,
5. extraction and productization last.

That means:

1. do not prioritize memory richness before runtime integrity,
2. do not prioritize swarm complexity before lease and review discipline,
3. do not prioritize daemon/cloud aspirations before stable local control-plane behavior,
4. do not prioritize Rust migration before current logic is fully validated in the live framework.

## Critical Path

The critical path for Release 1 is:

1. authoritative state and execution flow
2. orchestration and fallback correctness
3. review and verification statefulness
4. risk-aware runtime behavior
5. telemetry and scorecards grounded in observed execution
6. protocol-to-runtime consistency
7. framework extraction readiness

If any of these remain weak, Release 1 should not be considered complete.

## Phase Map

Release 1 should be delivered through these phases:

1. Phase A: Runtime Core Integrity
2. Phase B: Orchestration and Subagent Reliability
3. Phase C: Verification, Review, and Risk Gates
4. Phase D: Telemetry, Scorecards, and Drift Awareness
5. Phase E: Documentation Contract and Protocol Runtime Alignment
6. Phase F: Extraction Readiness and Standalone Framework Preparation

## Phase A: Runtime Core Integrity

Goal:

Make the framework trustworthy as an execution kernel on real work.

Scope:

1. single authoritative task state
2. deterministic TODO execution flow
3. pack lifecycle balance
4. context capsule and hydration reliability
5. boot-profile correctness
6. finish/verify/health gates that actually block bad state

Priority work:

1. stabilize `br` + TODO execution boundaries
2. ensure every non-trivial flow runs through active TODO blocks only
3. harden compact/restore behavior
4. eliminate state drift between logs, task state, and visible task board
5. ensure canonical entrypoints and pack routes remain singular

Exit criteria:

1. no work executes outside active block lifecycle
2. compact restore is deterministic
3. quality checks consistently detect task/log contradictions
4. pack start/end coverage is reliable

Why this comes first:

Without a stable execution kernel, every later subsystem becomes harder to trust.

## Phase B: Orchestration and Subagent Reliability

Goal:

Make external-first orchestration materially reliable, bounded, and debuggable.

Scope:

1. provider routing
2. external-first fanout
3. bridge fallback
4. internal senior escalation
5. bounded arbitration
6. provider timeouts and merge-readiness
7. graceful degradation

Priority work:

1. ensure route outputs are machine-usable and policy-consistent
2. ensure dispatch manifests materialize partial and final state correctly
3. distinguish command success from usable analysis
4. ensure provider exhaustion degrades cleanly instead of crashing
5. harden fallback ordering and arbitration behavior
6. prevent unsupported provider paths from breaking ensemble execution

Exit criteria:

1. external-first fanout completes or degrades cleanly
2. fallback chain respects policy order
3. arbitration is bounded and observable
4. provider runs produce enough artifacts for synthesis and evaluation

Why this comes second:

Vida Stack cannot claim to be an orchestration framework if its orchestration layer is still flaky under real bug-fix and analysis flows.

## Phase C: Verification, Review, and Risk Gates

Goal:

Convert review and verification from conventions into stateful runtime mechanics.

Scope:

1. route-level verification gates
2. merge-ready semantics
3. review-state semantics
4. risk-class semantics
5. escalation rules
6. human approval boundaries

Priority work:

1. add machine-visible `review_state`
2. add machine-visible `risk_class`
3. bind review behavior to route and write scope
4. distinguish low-risk promotion from senior-review-required paths
5. make handoff/close logic aware of review and risk state

Exit criteria:

1. review state exists in runtime artifacts
2. risk class exists in runtime artifacts
3. implementation and orchestration paths can route to different review expectations
4. higher-risk paths require stronger verification or escalation

Why this comes third:

Once orchestration works, the next risk is silent acceptance of low-quality or unsafe outputs.

## Phase D: Telemetry, Scorecards, and Drift Awareness

Goal:

Make runtime decisions evidence-driven instead of intuition-driven.

Scope:

1. provider run telemetry
2. eval-pack integration
3. dynamic scorecards
4. strategy snapshots
5. drift visibility
6. execution quality feedback loops

Priority work:

1. keep scorecards grounded in actual runs
2. preserve task-class and domain-level signal
3. surface degraded providers or repeated failure modes
4. create better route hints from observed performance
5. build the base for future anomaly and drift detection

Exit criteria:

1. scorecards reflect real observed behavior
2. strategy snapshots are usable for routing decisions
3. providers can be penalized or promoted with evidence
4. repeated degradation becomes visible without manual inspection

Why this comes fourth:

Telemetry is most valuable after the runtime and gate semantics are already trustworthy.

## Phase E: Documentation Contract and Protocol Runtime Alignment

Goal:

Reduce markdown drift and make the framework easier to operate, audit, and evolve.

Scope:

1. framework documentation navigation
2. framework/project boundary discipline
3. release-target documentation
4. protocol/runtime consistency
5. early machine-visible policy artifacts

Priority work:

1. keep framework docs coherent and ownership-split
2. maintain `README.MD`, `RELEASE-1-SCOPE.MD`, and this roadmap as top-level strategic documents
3. keep `protocol-index.md` synchronized
4. start introducing smaller machine-visible artifacts from runtime rules
5. reduce repeated heavy rereads where stable packets can replace markdown

Exit criteria:

1. framework docs form a coherent hierarchy
2. key runtime contracts are discoverable and consistent
3. the framework has a clear public-facing explanation and internal release target
4. protocol/runtime drift is reduced materially

Why this comes fifth:

By this point, documentation becomes not just descriptive, but a stable release and extraction surface.

## Phase F: Extraction Readiness and Standalone Framework Preparation

Goal:

Prepare Vida Stack to leave the host project as a clean standalone framework candidate.

Scope:

1. framework/product boundary cleanup
2. standalone framework structure
3. contributor-facing documentation
4. public extraction readiness
5. Rust migration input quality

Priority work:

1. isolate framework-owned logic and documents cleanly
2. reduce host-project coupling
3. identify which runtime mechanics are ready for standalone promotion
4. prepare architecture, roadmap, and release documents for public collaboration
5. capture the implementation lessons the Rust system must preserve

Exit criteria:

1. framework docs are public-repo-ready
2. framework runtime surfaces are sufficiently decoupled
3. the remaining project-specific knowledge is clearly separated
4. Release 1 is credible as a standalone framework milestone

Why this comes last:

Extraction should be the result of maturity, not a substitute for it.

## Ordered Workstreams

The main workstreams for Release 1 should be pursued in this order:

1. execution-state integrity
2. orchestration reliability
3. review/risk runtime state
4. telemetry and evaluation
5. protocol/runtime alignment
6. extraction readiness

Parallel work is allowed only when:

1. scopes do not overlap,
2. write ownership is explicit,
3. integration cost remains low,
4. the work does not depend on unfinished upstream runtime semantics.

## What Should Be Implemented Now

The current priority slice should continue focusing on:

1. orchestration runtime hardening
2. `risk_class` and `review_state` machine visibility
3. better bounded arbitration behavior
4. stronger provider-evaluation realism
5. smaller machine-visible policy/runtime artifacts

These items have the best immediate effect on:

1. current framework reliability
2. current real-project usefulness
3. future extraction quality

## What Should Wait

The following should wait until the earlier phases are stronger:

1. rich memory graph expansion
2. daemon-first automation
3. full DocSync bot automation
4. large-scale protocol compiler work
5. broad swarm execution
6. SaaS/cloud packaging
7. Rust migration execution

These are important, but they should not displace the core Release 1 mechanical finish.

## Relationship To VS Architecture

`VS_ARCHITECTURE_PLAN.md` describes the target system shape.

This roadmap intentionally narrows that vision into a practical Release 1 sequence:

1. finish the real mechanics first,
2. prove them on real work,
3. extract stable abstractions,
4. then carry them into the future fuller control-plane implementation.

That means this roadmap is the bridge from:

1. architecture vision
2. to framework release execution
3. to eventual Rust system design

## Release 1 Completion Standard

Release 1 should be considered truly ready only when:

1. the critical path is stable,
2. the framework is credible as a standalone artifact,
3. the runtime is reliable under real engineering workloads,
4. the documentation hierarchy is coherent,
5. the next-stage Rust reimplementation can inherit a proven logical model rather than unfinished assumptions.
