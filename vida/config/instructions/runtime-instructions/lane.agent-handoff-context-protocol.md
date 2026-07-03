# Agent Handoff And Context Protocol

Purpose: canonical law for agent-to-agent handoff, including orchestrator-to-worker, worker-to-worker, and fresh-start next-agent continuation prompts, so delegation stays packet-driven, bounded, replay-safe, and independent of hidden transcript inheritance.

## Core Contract

1. Handoffs are explicit runtime artifacts, not informal prompting habits.
2. The orchestrator owns handoff construction and downstream synthesis.
3. Receiving lanes must get only the context required for their lane function.
4. Undefined context inheritance is forbidden by default.
5. Rendered next-agent prompts are projections of canonical handoff and continuity artifacts, not free-written transcript summaries.
6. Session experience must be normalized into bounded evidence before it is allowed into a handoff or continuation prompt.

## Canonical Handoff Shape

A lawful handoff must define at minimum: sender lane, receiver lane, blocking question, scope in/out, allowed paths or bounded ownership unit when applicable, evidence refs, explicit verification command/proof target, output contract, fallback/escalation rule.

When rendered as a worker packet, the packet must also obey:

1. `instruction-contracts/lane.worker-dispatch-protocol`
2. `agent-definitions/entry.worker-entry`

Fresh-start continuation prompts for the next agent must also carry: whether the receiver is another orchestrator session or delegated worker lane; authoritative source of truth for active task/runtime state after compact, pause, or process replacement; enough bootstrap/routing context for lawful restart without replaying the prior transcript.

## Context Shaping Rule

Context must be filtered before handoff.

Allowed context classes: exact file refs; exact task/runtime artifact refs; compact embedded facts the receiver cannot cheaply reconstruct; route or receipt refs required for the assignment; bounded proof obligations.

Forbidden default context: unfiltered transcript inheritance; broad repository summaries without scope justification; hidden operator memory; unrelated historical context "just in case".

## Session-Experience Normalization Rule

Session experience is not a free-form narrative blob.

Before handoff construction or prompt rendering, the sender must normalize session experience into bounded classes: durable facts proven by live evidence, runtime state, or validated receipts; active user/protocol constraints still in force; exact completed steps and proof surfaces; exact remaining work/next-step targets; unresolved blockers, open unknowns, or regression watches.

Allowed session-experience sources: `instruction-contracts/overlay.session-context-continuity-protocol` session packet fields; validated task/runtime artifacts; explicit current-user instructions and stop conditions; exact changed-file/artifact refs; proof results and closure/readiness receipts.

Forbidden session-experience sources: speculative intent beyond active evidence; emotional/narrative session retellings; stale assumptions disproven by later evidence; broad transcript replay as a substitute for normalized continuity.

## Embedded Context Rule

Embedded context is allowed only when compact, lane-relevant, not cheaply reconstructable from canonical local artifacts, and not silently widening worker scope.

If embedded context and canonical artifacts disagree, prefer the higher-evidence canonical artifact and treat the packet as drift to correct.

## Next-Agent Prompt Formation Rule

When a handoff must be rendered as a prompt for the next agent, build it from canonical handoff and continuity artifacts rather than from ad hoc prose memory.

Required render order:

1. stable operating invariants and language/mode constraints that remain active,
2. authoritative current-state facts,
3. compact session-experience deltas already normalized under this protocol,
4. one bounded next action, blocking question, or continuation target.

Required prompt fields: repository root/workdir when relevant; receiver lane identity; mandatory bootstrap read-set when receiver may start fresh; active task, pack, or runtime identity plus authoritative status source; current bounded objective/blocking question; proven work with exact evidence refs; active constraints, stop conditions, protected scope; exact file/artifact scope in/out; verification/proof target; unresolved blockers, open unknowns, or next-step hints.

Prompt compactness rules: prefer exact facts and bounded bullets over narrative recap; separate proven facts from inference/expectation; include only info required for lawful restart, not convenience context that widens scope; if a fact is cheaply reconstructable from a canonical local artifact, prefer artifact ref over embedded prose.

If no authoritative next-step target exists, do not guess the next slice; render the blocker or required decision explicitly; fail closed rather than implying silent continuation.

## Rendering Variants

This protocol allows multiple rendered prompt variants from one canonical handoff packet.

Supported variants:

1. `worker packet`
   - explicit worker-lane confirmation,
   - one blocking question,
   - bounded ownership and verification command,
   - must obey `instruction-contracts/lane.worker-dispatch-protocol`.
2. `fresh-start continuation prompt`
   - used when the next agent may start without transcript inheritance,
   - must restate the lawful bootstrap/read path and authoritative state source,
   - must distinguish closed prior work from the still-open next slice.
3. `rework handoff`
   - used when a later pass must consume coach/verifier/escalation outcomes,
   - must include provenance for the feedback that changed the next effective prompt.

Variant rule: rendered wording may change by receiver lane; underlying bounded facts, scope, proof obligations, and stop conditions must remain equivalent.

## Overlay And Routing Materialization Rule

Next-agent prompts must not invent backend, role, model, or provider defaults.

Rules: concrete backend/model/profile bindings may be named only when materialized in active runtime/config surfaces; if the repo uses overlay-driven agent routing, prompt formation must prefer active overlay/config refs and validated lane bindings over remembered chat assumptions; project-specific behavior belongs in validated overlay, registry, or runtime artifacts, while the prompt carries only bounded session-local state needed for the next agent.

## Recovery And Replay Rule

Handoffs must remain usable across compact, restart, and retry.

Rules:

1. a handoff must be reconstructable from canonical packet/runtime artifacts rather than chat memory alone,
2. replaying or retrying a handoff must not silently expand scope,
3. repeated delivery of the same bounded handoff must preserve the same blocking question and ownership boundary unless an explicit updated packet supersedes it.

## Verification Boundary Rule

Each handoff must make verification boundaries explicit.

It must identify:

1. whether the receiver is an author lane, coach lane, verifier lane, or another bounded lane,
2. which proof or verification command closes the slice,
3. what remains outside the receiver's ownership.

## Handoff Closure Rule

1. A handoff remains open until the receiver lane returns a result that is synthesized or explicitly superseded by a newer canonical handoff/redirect receipt.
2. Open handoff state is a closure-relevant runtime fact, not a narrative detail.
3. While a bounded handoff remains open, the sender/orchestrator must not emit a `final` user-facing closure report for the represented request/task.
4. If the sender performs bounded local workaround work while a handoff is still open, that does not silently close the handoff; the orchestrator must still reconcile or supersede it explicitly before final closure.

Progress-report rule:

1. while a bounded handoff remains open, a user-facing progress report may be emitted only as non-blocking commentary,
2. that report must not become the last action of the active execution turn when `in_work=1`,
3. this applies equally to ordinary delegated packets and rework handoffs.
4. a just-dispatched handoff is already an open handoff for this rule even before the first worker return arrives,
5. therefore `dispatch complete, agents running` is not a lawful pause boundary by itself.

Reclaim rule:

1. a delegated lane is reclaimable only after its handoff is synthesized or explicitly superseded,
2. a completed-but-unsynthesized handoff is not reclaimable yet,
3. saturation recovery must check open handoff state before treating a delegated lane as closeable/reusable.

Exception-path interaction rule: an open delegated handoff for the same bounded packet blocks root-session local exception-path writing by default; pre-write exception receipts do not silently close/bypass that handoff; the orchestrator must first synthesize, supersede, or hard-block the delegated handoff before local takeover becomes lawful.

## External Alignment Note

This protocol's historical external-alignment lineage is preserved in:

1. `Git history and active artifact sidecars`

## References

1. `instruction-contracts/lane.worker-dispatch-protocol`
2. `runtime-instructions/core.context-governance-protocol`
3. `instruction-contracts/overlay.session-context-continuity-protocol`
4. `prompt-templates/worker.packet-templates`
5. `Git history and active artifact sidecars`

-----
artifact_path: config/runtime-instructions/agent-handoff-context.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: 2026-07-03T14:40:00+03:00
changelog_ref: lane.agent-handoff-context-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: field-list-compaction+handoff-atom-preserve-exact+gate-preserve-exact
protocol_compression_baseline_ref: 3aefbd5b8:vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md
protocol_compression_audit_at: 2026-07-03T14:40:00+03:00
protocol_compression_before_tokens: 2088
protocol_compression_after_tokens: 2067
protocol_compression_content_sha256: fed5008fc6e791784b2f700ef4917426f746021d10e161736619b3eea4c65234
