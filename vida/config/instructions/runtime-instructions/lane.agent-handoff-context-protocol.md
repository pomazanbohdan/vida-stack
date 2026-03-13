# Agent Handoff And Context Protocol

Purpose: define the canonical law for agent-to-agent handoff, including orchestrator-to-worker, worker-to-worker, and fresh-start next-agent continuation prompts, so delegation remains packet-driven, bounded, replay-safe, and independent of hidden transcript inheritance.

## Core Contract

1. Handoffs are explicit runtime artifacts, not informal prompting habits.
2. The orchestrator owns handoff construction and downstream synthesis.
3. Receiving lanes must get only the context required for their lane function.
4. Undefined context inheritance is forbidden by default.
5. Rendered next-agent prompts are projections of canonical handoff and continuity artifacts, not free-written transcript summaries.
6. Session experience must be normalized into bounded evidence before it is allowed into a handoff or continuation prompt.

## Canonical Handoff Shape

A lawful handoff must define at minimum:

1. sender lane
2. receiver lane
3. blocking question
4. scope in
5. scope out
6. allowed paths or bounded ownership unit when applicable
7. evidence references
8. explicit verification command or proof target
9. output contract
10. fallback or escalation rule

When rendered as a worker packet, the packet must also obey:

1. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
2. `vida/config/instructions/agent-definitions/entry.worker-entry.md`

Fresh-start continuation prompts for the next agent must additionally carry:

1. explicit statement of whether the receiver is another orchestrator session or a delegated worker lane,
2. the authoritative source of truth for active task/runtime state when the next agent may resume after compact, pause, or process replacement,
3. enough bootstrap/routing context for lawful restart without replaying the entire prior transcript.

## Context Shaping Rule

Context must be filtered before handoff.

Allowed context classes:

1. exact file references
2. exact task/runtime artifact references
3. compact embedded facts that the receiver cannot cheaply reconstruct
4. route or receipt references required for the assignment
5. bounded proof obligations

Forbidden default context:

1. unfiltered transcript inheritance
2. broad repository summaries without scope justification
3. hidden operator memory
4. unrelated historical context "just in case"

## Session-Experience Normalization Rule

Session experience is not a free-form narrative blob.

Before handoff construction or prompt rendering, the sender must normalize session experience into bounded classes:

1. durable facts proven by live evidence, runtime state, or validated receipts,
2. active user or protocol constraints still in force,
3. exact completed steps and their proof surfaces,
4. exact remaining work or next-step targets,
5. unresolved blockers, open unknowns, or regression watches.

Allowed session-experience sources:

1. `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md` session packet fields,
2. validated task/runtime artifacts,
3. explicit current-user instructions and current stop conditions,
4. exact changed-file or artifact references,
5. proof results and closure/readiness receipts.

Forbidden session-experience sources:

1. speculative interpretation of intent beyond the active evidence,
2. emotional/narrative retellings of the session,
3. stale assumptions disproven by later evidence,
4. broad transcript replay used as a substitute for normalized continuity.

## Embedded Context Rule

Embedded context is allowed only when:

1. it is compact,
2. it is lane-relevant,
3. it cannot be cheaply reconstructed from canonical local artifacts,
4. it does not silently widen worker scope.

If embedded context and canonical artifacts disagree:

1. prefer the higher-evidence canonical artifact,
2. treat the packet as drift to correct.

## Next-Agent Prompt Formation Rule

When a handoff must be rendered as a prompt for the next agent, build it from canonical handoff and continuity artifacts rather than from ad hoc prose memory.

Required render order:

1. stable operating invariants and language/mode constraints that remain active,
2. authoritative current-state facts,
3. compact session-experience deltas already normalized under this protocol,
4. one bounded next action, blocking question, or continuation target.

Required prompt fields:

1. repository root or working directory when relevant,
2. receiver lane identity,
3. mandatory bootstrap read-set when the receiver may start fresh,
4. active task, pack, or runtime identity plus authoritative status source,
5. current bounded objective or blocking question,
6. proven work with exact evidence references,
7. active constraints, stop conditions, and protected scope,
8. exact file or artifact scope in / scope out,
9. verification or proof target,
10. unresolved blockers, open unknowns, or next-step hints.

Prompt compactness rules:

1. prefer exact facts and bounded bullets over narrative recap,
2. separate proven facts from inference or expectation,
3. include only information required for lawful restart, not convenience context that widens scope,
4. if the same fact is reconstructable cheaply from a canonical local artifact, prefer the artifact reference over embedded prose.

If no authoritative next-step target exists:

1. do not guess the next slice,
2. render the blocker or required decision explicitly,
3. fail closed rather than implying silent continuation.

## Rendering Variants

This protocol allows multiple rendered prompt variants from one canonical handoff packet.

Supported variants:

1. `worker packet`
   - explicit worker-lane confirmation,
   - one blocking question,
   - bounded ownership and verification command,
   - must obey `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`.
2. `fresh-start continuation prompt`
   - used when the next agent may start without transcript inheritance,
   - must restate the lawful bootstrap/read path and authoritative state source,
   - must distinguish closed prior work from the still-open next slice.
3. `rework handoff`
   - used when a later pass must consume coach/verifier/escalation outcomes,
   - must include provenance for the feedback that changed the next effective prompt.

Variant rule:

1. rendered wording may change by receiver lane,
2. the underlying bounded facts, scope, proof obligations, and stop conditions must remain equivalent.

## Overlay And Routing Materialization Rule

Next-agent prompts must not invent backend, role, model, or provider defaults.

Rules:

1. concrete backend/model/profile bindings may be named only when they are already materialized in the active runtime/config surfaces,
2. if the current repository uses overlay-driven agent routing, prompt formation must prefer references to the active overlay/config and its validated lane bindings over remembered chat assumptions,
3. project-specific behavior belongs in validated overlay, registry, or runtime artifacts; the prompt should carry only the bounded session-local state needed for the next agent.

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

Exception-path interaction rule:

1. an open delegated handoff for the same bounded packet blocks root-session local exception-path writing by default,
2. pre-write exception receipts do not silently close or bypass that handoff,
3. the orchestrator must first synthesize, supersede, or hard-block the delegated handoff before local takeover becomes lawful.

## External Alignment Note

This protocol's historical external-alignment lineage is preserved in:

1. `docs/process/framework-source-lineage-index.md`

## References

1. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
2. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
3. `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md`
4. `vida/config/instructions/prompt-templates/worker.packet-templates.md`
5. `docs/process/framework-source-lineage-index.md`

-----
artifact_path: config/runtime-instructions/agent-handoff-context.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-12T09:07:43+02:00'
changelog_ref: lane.agent-handoff-context-protocol.changelog.jsonl
