# Agent-System New-Protocol Development And Update Protocol

Purpose: define the canonical framework protocol for creating new agent-system protocols and updating existing ones as layered, green-closure artifacts rather than as ad hoc command descriptions.

This file is the canonical protocol for authoring new agent-system protocols and updating existing agent-system protocols.

## Scope

This protocol applies when the framework needs to:

1. create a new bounded protocol of the agent system,
2. update, extend, narrow, split, or merge an existing protocol of the agent system,
3. replace command-first thinking with protocol-first authoring,
4. define coverage layers for a new or evolving protocol domain,
5. gather user input layer by layer before promoting a new or updated protocol,
6. keep canonical protocol law formal while user-facing collaboration stays natural,
7. determinize an existing command-shaped surface into separate protocol-bearing artifacts,
8. reproduce the full set of protocol-relevant requirements from incoming requirement sources into canonical protocol law.

## Framework Autonomy Rule

This protocol is framework-owned and must remain autonomous from project-specific requirement law.

Rules:

1. this protocol may consume incoming requirements from project work, user requests, or external requirement sources,
2. but its canonical structure, closure law, state model, blocker taxonomy, acceptance law, and authoring workflow remain framework-owned,
3. project-specific requirements may fill protocol content, but they must not redefine this protocol's authoring law,
4. if an incoming requirement conflicts with framework authoring law, preserve framework law and treat the conflict as bounded input to normalize or reject,
5. project overlays may provide additional domain inputs, but they must not weaken this protocol's fail-closed or layer-order rules.

## Requirement Reproduction Contract

This protocol must be able to reproduce the full protocol-relevant requirement set from incoming requirements into canonical protocol form.

Requirement reproduction means:

1. identifying every requirement that affects protocol identity, flow, gates, ownership, verification, recovery, or activation,
2. assigning each requirement to the earliest lawful layer that owns it,
3. preserving the requirement in canonical protocol wording without losing its functional meaning,
4. marking unsupported or unplaceable requirements as blockers rather than dropping them silently.

Hard rule:

1. no requirement that materially changes protocol behavior may be discarded as "just context",
2. no later-layer requirement may be used to patch over a missing earlier-layer requirement,
3. if a requirement cannot be placed into the layer model, raise a blocker and keep it explicit.

## Requirement Source Normalization

Incoming requirement sources may include:

1. direct user statements,
2. requirement documents,
3. specification artifacts,
4. command descriptions being determinized,
5. existing protocol text being updated.

Normalization rule:

1. convert source wording into bounded requirement statements before layer assignment,
2. merge duplicates only after confirming they are semantically the same requirement,
3. preserve conflict notes when two sources disagree,
4. do not let source phrasing become canonical law automatically.

## Requirement Coverage Classes

Every protocol-relevant requirement must be classifiable into at least one of:

1. `identity`
2. `scope`
3. `trigger`
4. `input`
5. `output`
6. `flow`
7. `gate`
8. `blocker`
9. `evidence`
10. `ownership`
11. `interaction`
12. `verification`
13. `approval`
14. `recovery`
15. `activation`
16. `validation`

Coverage rule:

1. each classified requirement must map to one earliest owning layer,
2. one requirement may influence later layers, but it must still have one earliest owner,
3. if a requirement cannot be classified, raise `BLK_REQUIREMENT_UNMAPPED` until the class is known.

## Requirement-To-Layer Mapping Law

Default earliest-owner mapping:

1. `identity`, `scope` -> `Layer 1`
2. `trigger`, `input`, `output`, `flow` -> `Layer 2`
3. `gate`, `blocker`, `evidence` -> `Layer 3`
4. `ownership`, `interaction` -> `Layer 4`
5. `verification`, `approval`, `recovery` -> `Layer 5`
6. `activation`, `validation` -> `Layer 6`

Mapping rule:

1. the earliest-owner layer must reproduce the requirement first,
2. dependent later layers may refine but must not silently redefine the original requirement,
3. if the mapped layer is not yet covered, the requirement remains open rather than assumed.

## Cross-Protocol Requirement Import Law

If one protocol begins to receive requirements that belong to another bounded domain, the author must decide between refinement and split.

Refinement is lawful only when all are true:

1. the imported requirement fits the same bounded trigger,
2. the imported requirement fits the same owner boundary,
3. the imported requirement fits the same responsibility set,
4. the imported requirement does not create a second canonical home.

Split into a new or separate protocol owner is required when at least one is true:

1. the imported requirement introduces a distinct trigger,
2. the imported requirement introduces a distinct owner boundary,
3. the imported requirement introduces a distinct responsibility set,
4. the imported requirement would make one concept have more than one canonical owner,
5. the imported requirement cannot be placed without widening the current protocol beyond its bounded domain.

Import rule:

1. do not silently absorb foreign-domain requirements into the current protocol,
2. if the requirement belongs elsewhere, open a sibling protocol track or route it to the existing canonical owner,
3. if the correct owner is unclear, raise `BLK_OWNER_CONFLICT` or `BLK_REQUIREMENT_UNMAPPED` until resolved.

## Core Contract

1. The canonical unit being created is a `protocol-bearing artifact`, not a command.
2. The artifact must describe one bounded process or work type of the agent system.
3. The protocol author must define the protocol through independent coverage layers.
4. Each layer must be functionally autonomous:
   - it may depend on already-closed earlier layers,
   - it must not depend on any later layer,
   - it must provide standalone value even if later layers are not yet written.
5. The protocol author must not skip directly to later-layer semantics when earlier-layer requirements are unresolved.
6. The canonical protocol description must stay formal, exact, and framework-owned.
7. User-facing collaboration must use more natural language than the internal canonical phrasing.
8. Natural user language must simplify explanation and questioning, but it must not weaken or replace the canonical protocol law.
9. Command names, shell wrappers, and runtime entrypoints may expose a protocol, but they do not define its law.
10. Updating an existing protocol must follow the same layered discipline as creating a new one.
11. A command-shaped surface must be decomposed into protocol-scoped parts when one command hides more than one bounded responsibility.

## Safe Token-Optimization Law

Token reduction is lawful only when it preserves owner-law authority, fail-closed meaning, and activation clarity.

Safe-by-default optimization classes:

1. `context_pointer`
   - point routine runtime/startup reads to a smaller owner-approved surface rather than replaying every full owner document.
2. `compact_capsule`
   - create a compact projection that keeps full words and stable field names while deferring edge cases to the owner artifact.
3. `runtime_ir`
   - expose machine-readable runtime fields with explicit names rather than prose replay.
4. `cache_partition`
   - split always-on, startup, lane, and dynamic context so only the active bounded subset is loaded.
5. `table_or_field_matrix`
   - replace repeated narrative routing or gate prose with bounded tables or stable field blocks.

Unsafe-by-default optimization classes:

1. `dictionary_abbreviation`
   - replacing canonical normative terms with short abbreviations in owner law.
2. `dsl_replacement`
   - replacing canonical owner-law prose with compact DSL as the primary authoritative surface.
3. `semantic_compression_of_normative_text`
   - summarizing or paraphrasing owner-law rules in a way that may weaken `must`, `forbidden`, `invalid`, `blocked`, or equivalent fail-closed meaning.
4. `authority_hiding`
   - moving material owner semantics into a compact surface without preserving the stronger owner artifact as canonical.

Hard rule:

1. token optimization is allowed only for delivery and consumption surfaces,
2. canonical owner law must stay readable and exact,
3. if an optimization would make the model reconstruct safety semantics heuristically, it is forbidden.

## Canonical Optimization Boundaries

Optimization must preserve this boundary split:

1. owner protocol
   - canonical human-readable law
2. compact capsule or startup bundle
   - routine read projection only
3. compiled runtime bundle or runtime IR
   - execution/consumption projection only
4. cache delivery partition
   - derived delivery boundary only

Boundary rule:

1. a compact capsule, bundle view, or runtime IR must not become a second competing law owner,
2. if a compact surface cannot safely answer an edge case, it must route back to the owner artifact rather than improvising.

## Safe Optimization Acceptance Criteria

An optimization pass is admissible only when all are true:

1. the owner artifact remains canonical,
2. modal safety language in the owner artifact is preserved,
3. the compact or compiled surface uses explicit field names rather than opaque abbreviations,
4. the optimized surface declares when escalation back to the owner artifact is required,
5. activation/discovery wiring stays explicit,
6. validation can prove that the optimized surface still routes lawful behavior.

Fail-closed rule:

1. if exact owner meaning cannot be preserved, keep the longer form,
2. if compression and safety are in tension, prefer safety and record token cost as a residual gap rather than weakening law.

## Canonical Naming And User Language Rule

Hard rule:

1. The framework must keep one formal canonical name for the protocol-bearing artifact.
2. During collaboration with the user, the agent must prefer clearer and more natural wording for questions, explanations, and prompts.
3. The agent must still translate the user-facing wording back into a precise canonical protocol description in the artifact body.
4. User-facing language should ask about the process the user wants, not about framework-internal artifact jargon, unless that jargon is already established in the current task.
5. The protocol must preserve both:
   - `canonical description` for framework law,
   - `natural collaboration language` for user interaction.

Example rule:

1. canonical artifact language may say `agent-system protocol`,
2. user-facing collaboration may say `new process` or `new type of work`,
3. the final artifact must normalize that natural wording back into the formal protocol contract.

## New-Protocol And Update Rule

This protocol governs two lawful authoring modes:

1. `new-protocol mode`
   - used when the domain does not yet have a canonical protocol owner,
2. `update mode`
   - used when the domain already has a canonical protocol owner but the protocol must be clarified, expanded, split, narrowed, merged, or re-layered.

Update-mode rules:

1. preserve the existing canonical owner when the topic remains one bounded domain,
2. create a new sibling protocol only when a distinct trigger, owner boundary, or responsibility set is discovered,
3. do not clone the same law into a second active protocol artifact,
4. when splitting an existing protocol, define the old artifact as pointer or bridge only after the new owners are explicit,
5. when narrowing an existing protocol, remove the law from the older owner rather than leaving duplicated active ownership.

## Absorption And Protocol Removal Law

When one protocol is fully covered by another stronger canonical protocol owner, the weaker protocol must not remain as an active competing law source.

Full absorption is lawful only when all are true:

1. the stronger protocol covers the weaker protocol's identity, trigger, flow, gates, ownership, verification, recovery, and activation semantics,
2. the stronger protocol covers those semantics at the same or stronger layer closure level,
3. no bounded requirement remains that still needs the weaker protocol as a distinct canonical owner,
4. the stronger protocol preserves or explicitly supersedes all material requirements of the weaker protocol,
5. protocol-index and activation wiring can be updated without leaving hidden ownership gaps.

Absorption actions:

1. move or normalize any remaining law into the stronger canonical owner,
2. remove duplicated active law from the weaker protocol,
3. convert the weaker protocol to pointer-only transitional text if temporary migration guidance is still needed,
4. delete the weaker protocol artifact entirely when no lawful active content remains,
5. update protocol-index and activation wiring in the same change.

Removal rule:

1. a protocol may be deleted only when its canonical law has been fully absorbed or explicitly retired,
2. deletion is forbidden if any material requirement would become unmapped after removal,
3. if the weaker protocol still owns one bounded requirement, it must stay as a distinct owner or be split further before deletion,
4. if safe deletion is not yet possible, keep only the smallest pointer/bridge needed during transition.

Fail-closed rule:

1. do not keep obsolete active protocols merely for historical comfort,
2. do not delete a protocol simply because a stronger one looks broader unless full coverage is proven,
3. if full coverage is uncertain, raise `BLK_DUPLICATE_OWNER` or `BLK_REQUIREMENT_UNMAPPED` rather than guessing.

## Update Taxonomy

Every update must be classified before rewriting the protocol.

Allowed update classes:

1. `editorial`
   - wording, clarity, examples, or non-semantic formatting only.
2. `clarification`
   - makes an existing rule more explicit without changing the bounded behavior.
3. `bounded_extension`
   - adds a new detail or subcase inside the same protocol boundary.
4. `boundary_narrowing`
   - removes behavior from the protocol and hands it off elsewhere or forbids it.
5. `boundary_expansion`
   - expands the protocol's owned domain while keeping one owner.
6. `split`
   - decomposes one protocol into multiple bounded protocol owners.
7. `merge`
   - consolidates several weakly split protocol owners into one bounded owner.
8. `breaking_change`
   - changes a previously closed behavior, trigger, gate, evidence requirement, owner boundary, or closure rule in a way that invalidates earlier assumptions.
9. `full_absorption`
   - one protocol is fully absorbed by a stronger canonical owner and the weaker active artifact is reduced to pointer-only or removed.

Classification rule:

1. every update pass must record one primary update class,
2. if several classes apply, use the strongest class as primary,
3. `breaking_change` outranks all other classes,
4. `full_absorption` outranks `merge` when one older protocol is no longer needed as an active owner,
5. if the update class cannot be identified, raise `BLK_UPDATE_SCOPE_UNCLEAR`.

## Breaking-Change Law

An update must be treated as `breaking_change` when at least one of the following changes:

1. bounded protocol identity,
2. activation trigger,
3. mandatory input contract,
4. expected output or terminal outcome,
5. gate semantics,
6. blocker meaning,
7. evidence threshold,
8. owner boundary,
9. approval requirement,
10. recovery rule,
11. admissibility rule for `GREEN_AT_LAYER` or `FRAMEWORK_GREEN`.

Breaking-change rules:

1. breaking changes must reopen the earliest changed layer and all dependent later layers,
2. breaking changes must restate what previous assumption is no longer valid,
3. breaking changes must not be hidden under `clarification` or `editorial` labels,
4. if the protocol author is unsure whether a change is breaking, classify it as `breaking_change` until proven otherwise.

## Layer Reopen Rules

Updates must reopen only the minimum required layers.

Reopen matrix:

1. `editorial`
   - reopen: none by default,
   - note: validation may still be required if wiring text changed.
2. `clarification`
   - reopen: the directly affected layer only.
3. `bounded_extension`
   - reopen: the first affected layer and every later dependent layer.
4. `boundary_narrowing`
   - reopen: `Layer 1` and every later dependent layer.
5. `boundary_expansion`
   - reopen: `Layer 1` and every later dependent layer.
6. `split`
   - reopen: all layers for each new protocol owner and `Layer 6` for rewiring.
7. `merge`
   - reopen: `Layer 1` through `Layer 6` for the merged owner.
8. `breaking_change`
   - reopen: the earliest changed layer and every later dependent layer.
9. `full_absorption`
   - reopen: `Layer 1` through `Layer 6` for the stronger owner plus removal/pointer review for the weaker owner.

Hard reopen rules:

1. if `Layer 1` changes, all later layers become provisional until re-covered,
2. if `Layer 2` changes, `Layers 3-6` must be reviewed and re-covered as needed,
3. if `Layer 3` changes, `Layers 4-6` must be reviewed and re-covered as needed,
4. if `Layer 4` changes, `Layers 5-6` must be reviewed and re-covered as needed,
5. if `Layer 5` changes, `Layer 6` must be reviewed and re-covered as needed,
6. if only `Layer 6` changes, earlier layers remain closed unless the wiring change reveals a hidden law conflict.

## Reopen Evidence Rule

When reopening a layer:

1. restate which earlier layers remain closed and why,
2. restate which later layers became provisional,
3. do not claim `GREEN_AT_LAYER` above the reopened boundary until consistency is restored,
4. if the reopened layer changes canonical meaning, treat unchanged later text as provisional rather than green.

## Layering Rule

Each newly authored protocol must define a bounded coverage stack.

Minimum layer properties:

1. layer name,
2. purpose,
3. standalone value,
4. dependencies on earlier layers only,
5. required user questions,
6. required outputs,
7. blocker conditions,
8. closure signal for the layer,
9. next-layer handoff.

Each layer must be green inside its own boundary before the protocol is promoted as covered at that layer.

## Protocol State Model

This protocol itself uses one canonical authoring-state model.

Allowed states:

1. `NOT_STARTED`
2. `LAYER_IN_PROGRESS`
3. `WAITING_USER_INPUT`
4. `LAYER_COVERED`
5. `BLOCKED`
6. `PROTOCOL_GREEN_PARTIAL`
7. `PROTOCOL_GREEN_FULL`

State rules:

1. `NOT_STARTED` means no lawful layer coverage exists yet.
2. `LAYER_IN_PROGRESS` means one current layer is being actively defined or revised.
3. `WAITING_USER_INPUT` means the current layer cannot close without explicit user answers or confirmation.
4. `LAYER_COVERED` means one bounded layer is green inside its own boundary.
5. `BLOCKED` means a blocker condition prevents lawful progression.
6. `PROTOCOL_GREEN_PARTIAL` means one or more leading layers are green, but the intended target closure layer is not yet reached.
7. `PROTOCOL_GREEN_FULL` means the protocol has reached its intended closure layer and all required discoverability and validation conditions are satisfied.

Transition law:

1. the protocol must move monotonically from lower unresolved layers toward higher layers,
2. later-layer progress must not be reported as `PROTOCOL_GREEN_FULL` while an earlier layer remains unresolved,
3. reopening a lower layer demotes later-layer confidence to provisional until consistency is restored,
4. `BLOCKED` or `WAITING_USER_INPUT` are lawful stop states,
5. `PROTOCOL_GREEN_FULL` is lawful only after required wiring and validation complete.

## Blocker Taxonomy

This protocol uses the following blocker codes:

1. `BLK_BOUNDARY_UNCLEAR`
   - the bounded process cannot yet be distinguished from adjacent domains.
2. `BLK_SCOPE_CONFLICT`
   - scope and non-goals contradict each other or conflict with an existing owner.
3. `BLK_TRIGGER_UNCLEAR`
   - start conditions or activation triggers are not explicit enough.
4. `BLK_OUTCOME_UNCLEAR`
   - the protocol outcome or end state is not yet defined.
5. `BLK_GATE_UNDEFINED`
   - required advancement gates are missing.
6. `BLK_EVIDENCE_UNDEFINED`
   - the proof/evidence model is missing or too ambiguous for fail-closed use.
7. `BLK_OWNER_CONFLICT`
   - ownership or routing boundaries are contradictory or unresolved.
8. `BLK_APPROVAL_UNCLEAR`
   - approval or user confirmation law is missing where required.
9. `BLK_RECOVERY_UNCLEAR`
   - interruption, retry, or resume behavior is underspecified.
10. `BLK_DUPLICATE_OWNER`
   - more than one active artifact currently owns the same law.
11. `BLK_ACTIVATION_UNWIRED`
   - activation binding is missing or invalid.
12. `BLK_INDEX_UNWIRED`
   - protocol-index discoverability is missing or stale.
13. `BLK_VALIDATION_UNPROVEN`
   - required validation or proof has not yet passed.
14. `BLK_COMMAND_NOT_DETERMINIZED`
   - a command-shaped surface still hides multiple bounded protocol responsibilities.
15. `BLK_UPDATE_SCOPE_UNCLEAR`
   - an update changed protocol behavior but the reopened layers are not yet identified correctly.
16. `BLK_REQUIREMENT_UNMAPPED`
   - one or more material requirements could not be assigned to a lawful coverage class or layer.

Blocker rule:

1. when a blocker code applies, the protocol must stop at the affected layer,
2. do not silently continue into a later layer around a blocker,
3. blocker resolution must be explicit in the canonical artifact or its lawful supporting wiring,
4. unmapped requirements must stay explicit until lawfully placed or rejected.

## Evidence Hierarchy

When evidence conflicts during protocol authoring or update, prefer:

1. explicit user-confirmed boundary, outcome, or approval relevant to the active layer,
2. canonical protocol artifact text already closed in lower layers,
3. canonical activation/index wiring and validation results,
4. bounded code/config/runtime evidence,
5. author inference or conversational assumption.

Evidence rule:

1. higher-evidence conflict wins over lower-evidence convenience,
2. lower-evidence material must be corrected or marked provisional when it conflicts,
3. no layer may claim green closure on author inference alone when stronger evidence is required by that layer.

## Completion Verdicts

This protocol uses the following verdicts:

1. `LAYER_COVERED`
   - the active layer is green within its own boundary.
2. `WAITING_USER_INPUT`
   - lawful continuation depends on explicit user answers or confirmation.
3. `BLOCKED`
   - one or more blocker codes prevent progression.
4. `GREEN_AT_LAYER`
   - the protocol is stable through the current highest closed layer.
5. `FRAMEWORK_GREEN`
   - the protocol is fully green for its intended closure layer and framework wiring.

Verdict rule:

1. after each authoring pass, emit exactly one dominant verdict for the current state,
2. `FRAMEWORK_GREEN` requires successful coverage of the intended closure layer plus discoverability and validation closure,
3. `GREEN_AT_LAYER` is stronger than `LAYER_COVERED` because it asserts consistency across all earlier layers too.

Verdict admissibility:

1. `LAYER_COVERED` is admissible only when the active layer meets its green criteria,
2. `GREEN_AT_LAYER` is admissible only when every earlier layer is still closed and no blocker is active,
3. `FRAMEWORK_GREEN` is admissible only when the acceptance law in this protocol is satisfied,
4. if admissibility is uncertain, downgrade to `WAITING_USER_INPUT` or `BLOCKED` rather than over-claim green status.

## Ownership Matrix

Minimum owners for this protocol:

1. `user`
   - provides intent, confirms scope, and answers layer questions that require user authority.
2. `protocol author`
   - gathers answers, drafts the canonical text, maintains layer discipline, and proposes next-layer closure.
3. `canonical owner`
   - the framework-owned artifact that ultimately carries the law.
4. `wiring maintainer`
   - ensures protocol-index and activation coverage are updated when needed.
5. `validator`
   - runs or confirms the required validation/proof checks before green closure.

Ownership rules:

1. the same agent may temporarily play several roles, but the responsibilities must remain conceptually distinct,
2. user intent does not by itself replace canonical authoring or validation duties,
3. authoring is not complete until wiring and validation duties are satisfied,
4. if ownership becomes ambiguous, raise `BLK_OWNER_CONFLICT`.

## Canonical Coverage Layers For New And Updated Protocols

Layer-order law:

1. Each next layer may refine the protocol, but it must not redefine the already-closed lower-layer boundary.
2. A protocol is allowed to be useful at any intermediate layer if that layer is internally green.
3. No layer may borrow hidden authority from future runtime behavior, future approvals, or future validation logic that is not already closed in the current layer.

### `Layer 1: Process Identity And Boundary`

Purpose:

1. define what process is being formalized,
2. define what the protocol owns,
3. define what it explicitly does not own.

Required outputs:

1. canonical protocol name,
2. plain-language user-facing name,
3. purpose statement,
4. scope boundary,
5. non-goals.

Standalone value:

1. the system can distinguish this protocol from adjacent domains without needing later execution or verification details.

Requirement groups:

1. identity requirements,
2. naming requirements,
3. scope requirements,
4. boundary requirements.

Green criteria:

1. the protocol can be named canonically,
2. the same protocol can be explained to the user in natural language,
3. adjacent protocol overlap is bounded,
4. explicit non-goals are written.

### `Layer 2: Entry, Outcome, And Core Flow`

Purpose:

1. define how the process starts,
2. define the expected end result,
3. define the minimal lawful flow from start to finish.

Required outputs:

1. entry triggers,
2. mandatory inputs,
3. expected outputs,
4. end states,
5. minimal step flow.

Standalone value:

1. the system can identify when the protocol should start and what successful completion means.

Depends on:

1. `Layer 1`

Requirement groups:

1. activation requirements,
2. input requirements,
3. outcome requirements,
4. minimal-flow requirements.

Green criteria:

1. a lawful start condition exists,
2. the expected result is explicit,
3. the minimum path from start to finish is inspectable,
4. the protocol does not rely on later verification logic to explain its core flow.

### `Layer 3: Requirements, Gates, And Evidence`

Purpose:

1. define what must be true before advancement,
2. define blocker classes,
3. define what evidence proves the layer is satisfied.

Required outputs:

1. gate list,
2. blocker codes or blocker classes,
3. required evidence,
4. fail-closed rules,
5. layer-level closure criteria.

Standalone value:

1. the system can stop unsafe advancement and can prove why the protocol is or is not ready to continue.

Depends on:

1. `Layers 1-2`

Requirement groups:

1. gate requirements,
2. blocker requirements,
3. evidence requirements,
4. fail-closed requirements.

Green criteria:

1. the protocol can say why progression is allowed,
2. the protocol can say why progression is blocked,
3. evidence classes are explicit,
4. unsupported advancement paths are forbidden rather than implied,
5. applicable blocker codes are known,
6. layer-level evidence uses the stated evidence hierarchy.

### `Layer 4: Ownership, Routing, And Interaction Model`

Purpose:

1. define who owns each part of the process,
2. define where user interaction is required,
3. define how the protocol routes across lanes or roles when needed.

Required outputs:

1. owner boundaries,
2. user interaction points,
3. role or lane responsibilities,
4. handoff or routing boundaries,
5. escalation points.

Standalone value:

1. the system can execute the protocol without ambiguous ownership or hidden routing assumptions.

Depends on:

1. `Layers 1-3`

Requirement groups:

1. ownership requirements,
2. routing requirements,
3. role-boundary requirements,
4. user-interaction requirements.

Green criteria:

1. writer/owner ambiguity is removed,
2. user touchpoints are explicit,
3. routing is bounded to declared lanes or roles,
4. escalation points exist for unresolved ownership or route conflicts.

### `Layer 5: Verification, Approval, And Recovery`

Purpose:

1. define how correctness is checked,
2. define when human/user approval is required,
3. define how the protocol resumes after interruption or revision.

Required outputs:

1. verification requirements,
2. approval gates,
3. recovery/resume rules,
4. revision loop rules,
5. completion verdicts.

Standalone value:

1. the system can safely close, pause, resume, or reject the protocol outcome.

Depends on:

1. `Layers 1-4`

Requirement groups:

1. verification requirements,
2. approval requirements,
3. recovery requirements,
4. completion requirements.

Green criteria:

1. close criteria are inspectable,
2. approval gates are explicit,
3. interruption does not erase the protocol contract,
4. resume and revision behavior are bounded,
5. completion verdicts are explicit.

### `Layer 6: Runtime Adoption And Green Closure`

Purpose:

1. define how the protocol becomes discoverable and consumable by the framework,
2. define what documentation and index updates are mandatory,
3. define when the protocol is considered green at framework level.

Required outputs:

1. canonical file placement,
2. index/map updates,
3. activation triggers,
4. validation/proof expectations,
5. green-closure statement.

Standalone value:

1. the framework can discover, activate, and validate the new protocol without relying on chat memory or ad hoc file knowledge.

Depends on:

1. `Layers 1-5`

Requirement groups:

1. discoverability requirements,
2. activation requirements,
3. validation requirements,
4. promotion requirements.

Green criteria:

1. the protocol is indexed,
2. the activation trigger is explicit,
3. validation coverage is named,
4. framework-level adoption does not rely on informal memory.

## Layer Map Summary

| Layer | Core question | Functional result | Must not depend on |
|---|---|---|---|
| Layer 1 | What process is this? | bounded protocol identity | later execution or verification semantics |
| Layer 2 | When does it start and end? | lawful start/outcome/core flow | later gate or approval logic |
| Layer 3 | What must be true to continue? | gates, blockers, evidence | later routing or runtime wiring |
| Layer 4 | Who owns what and when does the user interact? | ownership, routing, interaction model | later recovery or discoverability wiring |
| Layer 5 | How is it checked, approved, and resumed? | closure, approval, recovery model | later index/map adoption |
| Layer 6 | How does the framework consume it? | discoverable, activatable, green protocol | none; final layer |

## Layer Output Contract

Each layer should be able to emit one compact output packet.

Minimum packet by layer:

1. `Layer 1`
   - `canonical_name`
   - `user_facing_name`
   - `purpose`
   - `scope_boundary`
   - `non_goals`
2. `Layer 2`
   - `entry_triggers`
   - `mandatory_inputs`
   - `expected_outputs`
   - `end_states`
   - `minimal_flow`
3. `Layer 3`
   - `gates`
   - `blockers`
   - `evidence_requirements`
   - `fail_closed_rules`
   - `layer_closure_criteria`
4. `Layer 4`
   - `owner_boundaries`
   - `user_touchpoints`
   - `role_boundaries`
   - `routing_or_handoffs`
   - `escalation_points`
5. `Layer 5`
   - `verification_requirements`
   - `approval_gates`
   - `recovery_rules`
   - `revision_loop`
   - `completion_verdicts`
6. `Layer 6`
   - `canonical_path`
   - `index_updates`
   - `activation_binding`
   - `validation_expectations`
   - `green_closure_statement`
7. `cross-layer`
   - `requirement_inventory`
   - `coverage_classes`
   - `earliest_owner_map`
   - `unmapped_requirements`

Packet rule:

1. the packet may be rendered in prose today,
2. but these fields are canonical and should remain stable enough for future structured validation or template generation.
3. when token reduction is in scope, prefer stable field names and explicit tables over ad hoc abbreviations.

## Requirement Inventory Contract

Each authoring or update pass should maintain a compact requirement inventory with at least:

1. `requirement_id`
2. `source`
3. `normalized_requirement`
4. `coverage_class`
5. `earliest_owner_layer`
6. `current_status`
7. `conflict_note`

Requirement inventory rule:

1. every material incoming requirement should appear in the inventory,
2. every inventory row should be either covered, provisional, blocked, or rejected with explicit reason,
3. silent omission of a material requirement is a protocol violation.

## Requirement Detailing Rule

Each layer should detail requirements under at least the following headings when applicable:

1. `Purpose`
2. `Inputs`
3. `Outputs`
4. `Guards`
5. `Blockers`
6. `Evidence`
7. `User interaction`
8. `Handoff`
9. `Closure`

Rules:

1. if a heading is not applicable at the current layer, the protocol should state that explicitly rather than leaving it ambiguous,
2. later layers may enrich a heading but must not invalidate the already-closed meaning from earlier layers,
3. unresolved future detail must be marked as provisional rather than implied as green.

## Acceptance Law For `FRAMEWORK_GREEN`

This protocol may emit `FRAMEWORK_GREEN` only when all applicable checks pass.

Mandatory acceptance conditions:

1. one canonical owner exists for the active law,
2. update class is known,
3. reopened-layer scope is known when in update mode,
4. intended closure layer is explicit,
5. every required layer through that closure layer is `GREEN_AT_LAYER`,
6. no active blocker code remains,
7. required user confirmations are recorded,
8. protocol-index wiring is current,
9. activation wiring is current,
10. validation requirements were run and passed,
11. no duplicate active owner remains for the same law,
12. no command-shaped surface remains the hidden sole owner when determinization was in scope,
13. all material incoming requirements were mapped or explicitly blocked or rejected.

Strong acceptance conditions:

1. current state is `PROTOCOL_GREEN_FULL`,
2. current verdict is `FRAMEWORK_GREEN`,
3. evidence hierarchy does not contain unresolved higher-tier conflict,
4. any provisional later-layer text was either closed or removed.

Fail-closed rule:

1. if any mandatory acceptance condition is missing, `FRAMEWORK_GREEN` is forbidden,
2. if a strong acceptance condition is missing, downgrade to `GREEN_AT_LAYER` or `BLOCKED` depending on the risk,
3. if determinization is in scope and a command artifact still contains unique hidden law, raise `BLK_COMMAND_NOT_DETERMINIZED`.

## Machine-Checkable Closure Receipt

Each major authoring or update pass should be able to produce a compact closure receipt with at least:

1. `mode`
2. `update_class`
3. `target_closure_layer`
4. `highest_green_layer`
5. `current_state`
6. `current_verdict`
7. `active_blockers`
8. `owner_resolution`
9. `index_wired`
10. `activation_wired`
11. `validation_passed`
12. `requirements_mapped`
13. `requirements_unmapped`
14. `optimization_class`
15. `owner_authority_preserved`
16. `compact_surface_only`

Receipt rule:

1. the receipt may be emitted in prose today,
2. but the fields above are canonical and should remain stable enough for future machine validation.
3. if `optimization_class` is present, `owner_authority_preserved` must be true before green closure is lawful.

## Safe Optimization Rollout Sequence

When protocol/token optimization is the goal, apply changes in this order:

1. `Step 1: owner dedup only`
   - remove repeated prose from non-owner artifacts while keeping the stronger owner unchanged.
2. `Step 2: pointer routing`
   - route routine startup/execution reads toward compact pointers, capsules, or bundles.
3. `Step 3: compact capsule`
   - introduce compact projections using full words and explicit routing/escalation back to the owner artifact.
4. `Step 4: compiled runtime view`
   - materialize the compact projections into runtime/init/bundle surfaces where applicable.
5. `Step 5: cache partitioning`
   - split always-on, startup, lane, triggered, and dynamic context explicitly.
6. `Step 6: proof and drift audit`
   - validate that the optimized path preserves lawful routing on representative failure classes before promotion.

Rollout rule:

1. do not jump directly from long prose owner law to opaque DSL,
2. each step must remain individually safe and reversible,
3. if one step fails proof, stop there and keep the last safe layer rather than forcing deeper compression.

## Self-Assessment Receipt

When this protocol is assessed against itself, the pass should also be able to emit a compact self-assessment receipt with at least:

1. `assessed_protocol`
2. `update_class`
3. `earliest_reopened_layer`
4. `highest_green_layer`
5. `current_state`
6. `current_verdict`
7. `ideal_state_claim`
8. `active_blockers`
9. `residual_gaps`
10. `autonomy_proven`
11. `acceptance_law_satisfied`

Self-assessment receipt rule:

1. if `acceptance_law_satisfied=false`, `FRAMEWORK_GREEN` is forbidden,
2. if `ideal_state_claim=true`, then `residual_gaps` must be empty and `autonomy_proven=true`,
3. if residual gaps remain, keep them explicit rather than compressing them into a vague summary.

## Determinization Receipt

When a command surface is determinized, the pass should also emit a compact determinization receipt with at least:

1. `command_surface`
2. `responsibility_inventory`
3. `existing_protocol_owners`
4. `new_protocol_candidates`
5. `shared_entrypoint_retained`
6. `hidden_law_removed`
7. `requirements_reproduced`

Determinization rule:

1. if `hidden_law_removed=false`, determinization is not green,
2. if one responsibility cannot yet be separated cleanly, mark that protocol track as blocked rather than folding it back into the command by habit,
3. if determinization loses a material requirement during decomposition, raise `BLK_REQUIREMENT_UNMAPPED`.

## Absorption Receipt

When one protocol is fully absorbed by a stronger canonical owner, the pass should also emit a compact absorption receipt with at least:

1. `stronger_owner`
2. `weaker_owner`
3. `coverage_proven`
4. `material_requirements_preserved`
5. `duplicate_law_removed`
6. `pointer_only_retained`
7. `artifact_deleted`
8. `index_rewired`
9. `activation_rewired`
10. `unmapped_requirements_after_absorption`

Absorption receipt rule:

1. if `coverage_proven=false`, full absorption is not green,
2. if `material_requirements_preserved=false`, deletion is forbidden,
3. if `unmapped_requirements_after_absorption` is non-empty, raise `BLK_REQUIREMENT_UNMAPPED`,
4. if `duplicate_law_removed=false`, the weaker protocol remains an active conflict source and closure is forbidden.

## Protocol Self-Assessment Workflow

This protocol should be reviewed against itself in bounded passes.

Self-assessment order:

1. confirm the current update class for this protocol revision,
2. identify the earliest reopened layer,
3. restate the current highest green layer,
4. identify active blockers, if any,
5. evaluate whether `GREEN_AT_LAYER` is lawful,
6. evaluate whether `FRAMEWORK_GREEN` is lawful,
7. record the closure receipt.

Self-assessment rule:

1. this protocol must not declare itself ideal merely because it is canonically wired,
2. unresolved weakness must be expressed as a blocker, reopened layer, or downgraded verdict,
3. claims of ideal state require passing both the acceptance law and the self-assessment workflow without unresolved blockers.

## Ideal-State Law

This protocol may be described as `ideal` only under a stricter condition than ordinary `FRAMEWORK_GREEN`.

Ideal-state conditions:

1. `FRAMEWORK_GREEN` is already lawful,
2. no active blocker exists,
3. no provisional layer text remains,
4. no duplicate active owner ambiguity remains,
5. no unmapped material requirement remains,
6. no command-determinization or absorption path in scope remains partially resolved,
7. self-assessment produces no downgraded verdict and no residual open gap.

Ideal-state rule:

1. `ideal` is a reporting label, not a substitute for the state/verdict model,
2. if any ideal-state condition is missing, use the normal verdicts and do not claim ideality,
3. if certainty about ideal-state conditions is incomplete, downgrade to `FRAMEWORK_GREEN` or lower.

## Residual Gap Register

Even when the protocol is green, bounded residual gaps should be recorded explicitly when discovered.

Minimum residual-gap fields:

1. `gap_id`
2. `layer`
3. `gap_summary`
4. `current_effect`
5. `blocking_status`
6. `next_action`

Residual-gap rule:

1. a non-blocking residual gap does not prevent `GREEN_AT_LAYER` or `FRAMEWORK_GREEN` unless it violates the acceptance law,
2. a gap that affects ideal-state conditions prevents an `ideal` claim,
3. do not hide residual gaps inside vague narrative summary.

## Framework Autonomy Proof

When this protocol claims framework autonomy from project-specific law, that claim should be supportable by a bounded proof.

Minimum autonomy checks:

1. the protocol's state model is framework-owned,
2. the blocker taxonomy is framework-owned,
3. the acceptance law is framework-owned,
4. the layer model is framework-owned,
5. incoming project requirements are treated as content inputs rather than as owner of the authoring law.

Autonomy proof rule:

1. if a project-specific artifact would be required to interpret this protocol's core authoring law, autonomy is not yet closed,
2. references to project or product artifacts may remain secondary examples or supporting context, but they must not become mandatory owner of the protocol's core semantics.

## Protocol Creation Workflow

Create a new protocol in this order:

1. confirm that the topic does not already have a canonical protocol owner,
2. define the bounded process and create the Layer 1 contract,
3. gather user answers needed for Layer 1 and write the canonical formalization,
4. report Layer 1 coverage and explain the value of Layer 2,
5. gather user answers needed for Layer 2 and extend the protocol,
6. continue layer by layer through Layer 6,
7. after each layer, mark what is green, what remains open, and what the next layer adds,
8. update the requirement inventory and confirm that all material requirements are mapped to lawful layers,
9. when the protocol reaches its intended closure layer, update framework discoverability and validation surfaces,
10. validate the changed canon before treating the new protocol as active,
11. emit one lawful state and one lawful verdict for the protocol after the pass.

## Protocol Update Workflow

Update an existing protocol in this order:

1. identify the current canonical owner,
2. identify whether the change is clarification, bounded extension, boundary narrowing, split, merge, or full absorption,
3. restate the current closed lower-layer contract before changing later layers,
4. reopen only the layers that the change truly affects,
5. gather user input only for the reopened layers,
6. update the canonical owner or owners,
7. remove duplicated law from obsolete owners and delete or downgrade weaker absorbed owners when lawful,
8. update protocol-index and activation wiring if ownership, naming, or triggering changed,
9. update the requirement inventory and confirm that all material changed requirements are mapped to lawful layers,
10. validate the changed canon before treating the update as green,
11. emit one lawful state and one lawful verdict for the protocol after the update pass.

## Command Determinization Rule

When an existing `command` surface actually contains several bounded responsibilities:

1. treat that command as a transport surface, not as product law,
2. inventory the distinct responsibilities hidden inside it,
3. convert each bounded responsibility into a separate protocol candidate,
4. run each protocol candidate through this authoring protocol layer by layer,
5. keep the command surface only as an entrypoint or operator convenience if still needed,
6. do not let the command artifact remain the only owner of the law once separate protocol owners exist.

Determinization triggers:

1. one command contains more than one independent trigger or outcome,
2. one command mixes several owner boundaries,
3. one command bundles unrelated gates or evidence models,
4. one command cannot be explained as one bounded process without hand-waving,
5. one command needs different closure logic for different internal parts.

Command determinization workflow:

1. inventory the command's bounded responsibilities,
2. identify which responsibility is already owned by an existing protocol and which is not,
3. extract and normalize the material requirements hidden inside the command,
4. open one protocol track per distinct bounded responsibility,
5. run each track through Layers 1-6 independently,
6. reduce the original command surface to entrypoint or pointer status once protocol owners are green,
7. validate that no hidden law remains only inside the command artifact.

## Per-Layer User Conversation Workflow

When the user asks for a new protocol:

1. ask the plain-language version of the Layer 1 questions first,
2. write the Layer 1 canonical wording after the answers are known,
3. tell the user what Layer 1 now covers,
4. explain what Layer 2 will add and which gaps remain,
5. ask whether to continue,
6. repeat the same pattern for every next layer.

When the user asks to update an existing protocol:

1. explain which current protocol is being updated,
2. explain which layers are already closed and which layers must be reopened,
3. ask only the natural-language questions needed for the reopened layers,
4. rewrite the canonical protocol after each reopened layer is covered,
5. report whether the update preserved one owner or created a lawful split.

When the user asks to convert a command into protocols:

1. first explain that the command will be treated as an entry surface, not as the canonical law owner,
2. inventory the hidden bounded processes inside that command,
3. process each bounded process through this protocol one by one,
4. show the user which protocol layer is being covered for which decomposed process.

Per-layer reporting rule:

1. after each layer, state:
   - what is now covered,
   - what risks or gaps remain,
   - what the next layer is for,
   - what new capability the next layer unlocks,
   - the current state,
   - the current dominant verdict,
   - any active blocker codes.

## Minimum Questions By Layer

The author should use the smallest natural-language question set that can close the layer.

Minimum bounded questions:

1. `Layer 1`
   - what new process or type of work should this protocol formalize,
   - where should this protocol stop,
   - what should remain outside its scope.
2. `Layer 2`
   - what starts this process,
   - what result should it produce,
   - what are the minimum steps between start and result.
3. `Layer 3`
   - what must be true before moving forward,
   - what should block the process,
   - what counts as enough proof or evidence.
4. `Layer 4`
   - who owns which part,
   - where should the user be asked or informed,
   - when should routing or escalation happen.
5. `Layer 5`
   - how is the result verified,
   - when is approval required,
   - how should interruption, revision, or retry work.
6. `Layer 6`
   - how should the framework discover this protocol,
   - what must be updated for activation,
   - what proves the protocol is green enough for adoption.

For update mode, add only the minimum delta questions:

1. what changed in the process boundary,
2. what changed in start, finish, or expected result,
3. what changed in gates, blockers, or proof,
4. what changed in ownership, user interaction, or routing,
5. what changed in verification, approval, or recovery,
6. what changed in activation, discoverability, or canonical ownership.

For command determinization, ask before Layer 1:

1. which parts of this command are actually separate bounded processes,
2. which parts have different triggers or outcomes,
3. which parts have different owners or evidence rules,
4. which parts should become separate protocol candidates.

## New Protocol Artifact Checklist

Before closing work on a newly authored protocol, confirm all applicable items:

1. canonical name exists,
2. user-facing natural phrasing exists,
3. scope and non-goals exist,
4. start and outcome exist,
5. layer map exists,
6. requirements are grouped by layer,
7. material requirements are inventoried and mapped,
8. blocker/evidence logic exists where required,
9. ownership and user interaction points exist where required,
10. verification and approval logic exists where required,
11. activation/discoverability path exists,
12. protocol-index wiring exists,
13. validation expectation exists.
14. current authoring state is explicit.
15. current dominant verdict is explicit.

For update mode, also confirm:

1. obsolete duplicated law was removed or turned into pointer text,
2. the old owner was preserved or deliberately split,
3. any changed trigger or activation rule was rewired,
4. any fully absorbed weaker owner was lawfully deleted or downgraded to pointer-only.

For command determinization, also confirm:

1. the command is no longer the sole law owner,
2. each decomposed bounded process has its own protocol coverage track,
3. the command surface now points to protocol owners instead of hiding the law.

For full absorption or retirement, also confirm:

1. the stronger owner proves full coverage of the weaker owner's material law,
2. no material requirement becomes unmapped after absorption,
3. the weaker owner was deleted or reduced to the smallest lawful pointer-only bridge,
4. protocol-index wiring no longer treats the weaker owner as an active canonical law source,
5. activation wiring no longer activates the weaker owner as an active domain authority.

## Protocol Retirement Checklist

Before deleting or fully retiring a protocol artifact, confirm all applicable items:

1. a stronger owner or explicit retirement decision exists,
2. the weaker protocol no longer owns a distinct bounded trigger,
3. the weaker protocol no longer owns a distinct bounded responsibility set,
4. all material requirements are preserved elsewhere or explicitly retired,
5. duplicate active law has been removed,
6. protocol-index references are updated,
7. activation bindings are updated or removed,
8. no unmapped requirement remains,
9. absorption receipt exists when retirement happens by full absorption,
10. validation still passes after retirement.

## Self-Application Rule

This protocol is itself subject to its own layered authoring law.

Rules:

1. changes to this protocol must be evaluated using this same state model, blocker taxonomy, evidence hierarchy, and verdict model,
2. this protocol must not be declared `FRAMEWORK_GREEN` for a revision if it fails its own required wiring or validation,
3. when this protocol reveals a gap in itself, that gap must be corrected directly or recorded as the explicit current blocker,
4. self-application must not be waived merely because this protocol is already canonical.

## User Collaboration Rule

When authoring a new protocol with the user:

1. start from the earliest unresolved layer,
2. ask only the questions needed to close that layer,
3. ask those questions in natural user language,
4. convert the answers into formal canonical protocol language inside the artifact,
5. report that the current layer is covered before proposing the next layer,
6. explain what the next layer adds and which gaps it closes,
7. ask for user confirmation before deepening into a later layer when new scope or new policy commitment is introduced.

If later-layer descriptive requirements are already obvious from closed earlier layers:

1. the author may draft them provisionally,
2. but they remain provisional until the user interaction for that layer is completed or the current task explicitly authorizes autonomous continuation.

## Layer Question Rule

Minimum collaboration posture by layer:

1. `Layer 1` questions should identify the process in plain terms.
2. `Layer 2` questions should identify start, finish, and expected result.
3. `Layer 3` questions should identify requirements, blockers, and proof.
4. `Layer 4` questions should identify ownership, role boundaries, and required interaction.
5. `Layer 5` questions should identify verification, approvals, and recovery behavior.
6. `Layer 6` questions should identify activation, discoverability, and closure evidence.

The author must not ask the user later-layer questions as a substitute for unresolved earlier-layer definition.

Natural-language rule:

1. Ask the user about the process in everyday language first.
2. Avoid leading with internal artifact jargon when a simpler phrase can collect the same requirement.
3. After the user answers, normalize that answer into the formal canonical protocol wording.
4. Keep the formal artifact precise even when the live conversation stays conversational.

## Promotion Rule

A new protocol may be promoted into active framework canon only when:

1. its bounded domain is distinct and justified,
2. its coverage layers are defined,
3. the current closure layer is explicitly recorded,
4. protocol-index discoverability is updated,
5. activation and validation coverage are not left implicit.

## Validation And Closure Rule

When this protocol creates or materially changes a canonical protocol-bearing artifact:

1. update the canonical protocol artifact first,
2. update `vida/config/instructions/system-maps/protocol.index.md`,
3. run activation/coverage validation appropriate to the changed scope,
4. when token optimization is in scope, validate the optimized surface against at least one normal startup path and one failure-shaped path before promotion,
5. do not treat the protocol as framework-green until those updates and checks are complete.

## Related

1. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
2. `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md`
3. `vida/config/instructions/system-maps/protocol.index.md`
4. `docs/product/spec/instruction-artifact-model.md`
5. `vida/config/instructions/command-instructions/routing.command-layer-protocol.md`
6. `vida/config/instructions/references/protocol.agent-system-new-protocol-artifact-templates.md`
   - non-canonical companion reference for packet and receipt shapes only

-----
artifact_path: config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md
created_at: '2026-03-11T07:38:29+02:00'
updated_at: '2026-03-13T19:20:00+02:00'
changelog_ref: work.agent-system-new-protocol-development-and-update-protocol.changelog.jsonl
