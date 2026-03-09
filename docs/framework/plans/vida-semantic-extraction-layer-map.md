# VIDA Semantic Extraction Layer Map

Purpose: define exactly which semantically valuable layers must be extracted from the current VIDA `0.1` runtime before or during the direct `1.0` binary implementation.

Status: canonical planning artifact for direct semantic extraction.

Date: 2026-03-08

---

## 1. Core Rule

Do not extract files.

Extract:

1. laws
2. states
3. transitions
4. receipts
5. schemas
6. command semantics
7. verification and approval behavior
8. compact/resume behavior

Do not preserve mechanically:

1. shell/Python split
2. `br` topology
3. `.beads/issues.jsonl`
4. queue/log path accidents
5. wrapper-specific CLI glue

Compact formula:

`extract semantics, not topology`

---

## 2. Extraction Order

Recommended minimum order:

1. bootstrap router
2. activation stack
3. intent and tracked-flow gate
4. overlay activation schema
5. command semantics
6. state vocabulary
7. lifecycle and transition law
8. receipt taxonomy
9. authorization and route law
10. verification and approval law
11. instruction/runtime law
12. worker packet and handoff semantics
13. run-graph and resumability semantics
14. memory semantics
15. observability and evaluation semantics
16. bridge/export semantics

Reason:

1. commands and states define the public product contract,
2. bootstrap, activation, and intent gating define when governed execution even starts,
3. route/verification law defines correctness,
4. instruction and migration layers depend on stabilized vocabulary,
5. observability and bridge layers should be derived from the core, not invented early.

---

## 3. Layer Matrix

### Layer 0. Bootstrap Router

Extract:

1. lane resolution
2. global invariants
3. instruction precedence root
4. hard-stop boot rule

Do not carry forward:

1. repo reread ritual as runtime mechanism

Current sources:

1. `AGENTS.md`

Target artifact:

1. `Command Tree Spec`
2. `Instruction Kernel Spec`

### Layer 1. Activation Stack

Extract:

1. instruction activation classes
2. trigger matrix
3. upper-layer vs domain-layer separation
4. decomposition guidance for instruction surfaces

Do not carry forward:

1. broad markdown loading as runtime behavior
2. duplicated policy bodies across files

Current sources:

1. `docs/framework/instruction-activation-protocol.md`
2. `docs/framework/ORCHESTRATOR-ENTRY.MD`

Target artifact:

1. `Instruction Kernel Spec`
2. `Command Tree Spec`

### Layer 2. Intent And Tracked-Flow Gate

Extract:

1. `answer_only`
2. `artifact_flow`
3. `execution_flow`
4. `mixed`
5. TODO/pack engagement boundary

Do not carry forward:

1. heuristic broad tracked-flow behavior

Current sources:

1. `docs/framework/ORCHESTRATOR-ENTRY.MD`
2. `docs/framework/orchestration-protocol.md`

Target artifact:

1. `Command Tree Spec`
2. `State Kernel Schema Spec`

### Layer 3. Overlay Activation Schema

Extract:

1. root overlay sections
2. activation order
3. schema validation boundary
4. project-vs-framework ownership split

Do not carry forward:

1. YAML parsing quirks or helper-specific loading path

Current sources:

1. `docs/framework/project-overlay-protocol.md`
2. `vida.config.yaml`

Target artifact:

1. `Instruction Kernel Spec`
2. `Migration Kernel Spec`

### Layer 4. Command Semantics

Extract:

1. canonical operator actions
2. expected command outcomes
3. compact output expectations
4. error and blocker semantics

Do not carry forward:

1. `bash docs/framework/history/_vida-source/scripts/...` as the long-term operator surface
2. shell-specific argument plumbing

Current sources:

1. `AGENTS.md`
2. `docs/framework/command-layer-protocol.md`
3. `docs/framework/orchestration-protocol.md`
4. `docs/framework/beads-protocol.md`
5. `docs/framework/protocol-index.md`

Target artifact:

1. `Command Tree Spec`

### Layer 5. State Vocabulary

Extract:

1. task states
2. execution states
3. review states
4. approval states
5. blocker codes
6. mode and route state names

Do not carry forward:

1. `br` storage assumptions
2. `.beads` file structure

Current sources:

1. `docs/framework/beads-protocol.md`
2. `docs/framework/todo-protocol.md`
3. `docs/framework/human-approval-protocol.md`
4. `docs/framework/agent-system-protocol.md`

Target artifact:

1. `State Kernel Schema Spec`

### Layer 6. Lifecycle And Transition Law

Extract:

1. task start/checkpoint/finish semantics
2. pack lifecycle
3. block lifecycle
4. redirect/supersede behavior
5. reconciliation behavior

Do not carry forward:

1. current wrapper command names as implementation requirement
2. shell-only sequencing tricks

Current sources:

1. `docs/framework/beads-protocol.md`
2. `docs/framework/task-state-reconciliation-protocol.md`
3. `docs/framework/todo-protocol.md`

Target artifact:

1. `State Kernel Schema Spec`
2. `Route And Receipt Spec`

### Layer 7. Receipt Taxonomy

Extract:

1. route receipts
2. escalation receipts
3. approval receipts
4. verification receipts
5. boot receipts
6. run-graph node receipts
7. context capsule / handoff receipts

Do not carry forward:

1. current file path layout as required runtime topology

Current sources:

1. `docs/framework/human-approval-protocol.md`
2. `docs/framework/run-graph-protocol.md`
3. `docs/framework/boot-packet-protocol.md`
4. `docs/framework/context-governance-protocol.md`
5. `docs/framework/beads-protocol.md`
6. `docs/framework/agent-system-protocol.md`

Target artifact:

1. `Route And Receipt Spec`

### Layer 8. Authorization And Route Law

Extract:

1. when analysis is required
2. writer authorization boundaries
3. independent verification requirements
4. lawful fallback and escalation rules
5. fail-closed behavior when route metadata is missing

Do not carry forward:

1. CLI-provider-specific routing assumptions
2. script-local route assembly as the target design

Current sources:

1. `docs/framework/agent-system-protocol.md`
2. `docs/framework/ORCHESTRATOR-ENTRY.MD`
3. `docs/framework/orchestration-protocol.md`
4. `docs/framework/implement-execution-protocol.md`

Target artifact:

1. `Route And Receipt Spec`
2. `Conformance Spec`

### Layer 9. Verification, Coach, And Approval Law

Extract:

1. coach position in the flow
2. verifier independence
3. approval-required states
4. closure-ready law
5. proof-before-close semantics

Do not carry forward:

1. current script/gate filenames as the product boundary

Current sources:

1. `docs/framework/agent-system-protocol.md`
2. `docs/framework/human-approval-protocol.md`
3. `docs/framework/product-proving-packs-protocol.md`
4. `docs/framework/trace-eval-protocol.md`

Target artifact:

1. `Route And Receipt Spec`
2. `Parity And Conformance Spec`

### Layer 10. Instruction Runtime Law

Extract:

1. `Agent Definition`
2. `Instruction Contract`
3. `Prompt Template Configuration`
4. deterministic no-implied-behavior law
5. fallback and escalation contract

Do not carry forward:

1. provider-specific prompt rendering as source of truth
2. chat-memory-dependent behavior

Current sources:

1. `docs/framework/history/research/2026-03-08-agentic-agent-definition-system.md`
2. `docs/framework/agent-definition-protocol.md`
3. `docs/framework/templates/instruction-contract.yaml`
4. `docs/framework/templates/prompt-template-config.yaml`

Target artifact:

1. `Instruction Kernel Spec`

### Layer 11. Worker Packet And Handoff Semantics

Extract:

1. bounded worker packet requirements
2. required inputs and output contracts
3. compact-safe no-chat-memory rules
4. task-local proof and stop conditions

Do not carry forward:

1. current packet rendering scripts as required architecture

Current sources:

1. `docs/framework/worker-dispatch-protocol.md`
2. `docs/framework/WORKER-ENTRY.MD`
3. `docs/framework/WORKER-THINKING.MD`
4. `docs/framework/history/research/2026-03-08-agentic-epic-slicing-agent-instruction.md`

Target artifact:

1. `Cheap Worker Packet System`
2. `Instruction Kernel Spec`

### Layer 12. Run-Graph And Resumability Semantics

Extract:

1. run node types
2. node status changes
3. resumability requirements
4. compact/handoff invariants
5. context capsule intent preservation

Do not carry forward:

1. current `.vida/state/run-graphs/*.json` layout as product requirement

Current sources:

1. `docs/framework/run-graph-protocol.md`
2. `docs/framework/beads-protocol.md`
3. `docs/framework/context-governance-protocol.md`

Target artifact:

1. `State Kernel Schema Spec`
2. `Route And Receipt Spec`

### Layer 13. Memory Semantics

Extract:

1. framework memory kinds
2. durable lessons/anomalies/corrections
3. memory ownership boundaries
4. memory freshness and lifecycle expectations

Do not carry forward:

1. current file-based ledger implementation as required architecture

Current sources:

1. `docs/framework/framework-memory-protocol.md`
2. `docs/framework/document-lifecycle-protocol.md`
3. `docs/framework/silent-framework-diagnosis-protocol.md`

Target artifact:

1. `Memory Kernel Spec`
2. `Migration Kernel Spec`

### Layer 14. Evaluation And Observability Semantics

Extract:

1. eval object types
2. trace grading expectations
3. operator-facing status surfaces
4. metric vocabulary
5. parity and regression expectations

Do not carry forward:

1. current file-per-eval output layout as product requirement

Current sources:

1. `docs/framework/trace-eval-protocol.md`
2. `docs/framework/history/research/2026-03-08-agentic-metric-glossary.md`
3. `docs/framework/history/research/2026-03-08-agentic-proof-obligation-registry.md`

Target artifact:

1. `Parity And Conformance Spec`
2. `Observability Kernel section inside direct 1.0 program`

### Layer 15. Bridge And Migration Semantics

Extract:

1. which `0.1` artifacts are needed for parity
2. import/export boundaries
3. migration safety law
4. startup compatibility expectations
5. cutover rules

Do not carry forward:

1. current runtime quirks as behavior requirements

Current sources:

1. `docs/framework/history/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
2. `docs/research/vida-framework/vida-migration-registry.md`
3. `vida-stack/VERSION-PLAN.md` via the direct-transition plan

Target artifact:

1. `Migration Kernel Spec`
2. `0.1 Bridge Policy`

---

## 4. Minimal Artifact Mapping

| Layer | First target spec |
|---|---|
| bootstrap router | command tree spec |
| activation stack | instruction kernel spec |
| intent and tracked-flow gate | command tree spec |
| overlay activation schema | instruction kernel spec |
| command semantics | command tree spec |
| state vocabulary | state kernel schema spec |
| lifecycle and transition law | state kernel schema spec |
| receipt taxonomy | route and receipt spec |
| authorization and route law | route and receipt spec |
| verification/coach/approval law | route and receipt spec |
| instruction runtime law | instruction kernel spec |
| worker packet semantics | cheap worker packet system |
| run-graph/resumability | state kernel schema spec |
| memory semantics | memory kernel spec |
| evaluation/observability | parity and conformance spec |
| bridge and migration semantics | migration kernel spec |

---

## 5. Extraction Anti-Patterns

Avoid:

1. extracting scripts instead of behaviors
2. freezing shell command syntax without freezing semantic intent
3. copying path/file layouts into the product spec
4. defining the binary from current helper boundaries
5. treating `br` bugs or JSONL workarounds as long-term product rules

---

## 6. Immediate Next Work

Use this layer map to drive the next spec family in this order:

1. semantic freeze and vocabulary
2. direct `1.0` epic/spec program
3. cheap worker packet system
4. command tree spec
5. state kernel schema spec
6. instruction kernel spec
7. migration kernel spec
8. route and receipt spec
9. parity and conformance spec
