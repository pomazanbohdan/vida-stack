# Thinking Protocol — Unified Thinking Algorithms

Purpose: keep the canonical full algorithm specifications referenced by VIDA boot/runtime surfaces.

Shared rules:

1. The canonical algorithm owners in this file are the embedded section anchors below.
2. Boot/read activation is owned by `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md` and `vida/config/instructions/agent-definitions.orchestrator-entry.md`.
3. Web/internet validation is owned by `vida/config/instructions/runtime-instructions.web-validation-protocol.md`.
4. User-facing reporting must not expose intermediate chain-of-thought; `Thinking mode` remains a reporting label only.
5. PR-CoT, MAR, 5-SOL, and META must preserve impact analysis covering:
   - affected scope,
   - contract impact,
   - operational impact,
   - follow-up,
   - residual risks.
6. Named algorithms below are canonical flow templates built from reusable reasoning blocks; META may assemble the smallest lawful block flow instead of executing whole named algorithms by default.

## Embedded Algorithms (Canonical Sections)

## Section: algorithm-selector

# Algorithm Selector

> Unified router for selecting the active thinking algorithm.

---

## Selection

```toon
selection[5]{score,algorithm,purpose}:
  ≤12,STC,Step-critique (silent)
  13-22,PR-CoT,4 perspectives validation
  23-32,MAR,3 rounds × 4 agents
  33-42,5-SOL,2 rounds × 5 options (alignment)
  >42,META,Block composer for high-risk tasks
```

---

## Overrides (Check First!)

```toon
overrides[12]{scenario,algorithm}:
  Bug / incident / regression,Error Search
  Security/Auth decision,META
  Database schema,META
  Foundation architecture,META
  Tech stack selection,META
  Framework-owned behavior change,META
  Protocol conflict / protocol mismatch,META
  Execution gate mismatch,META
  Fail-closed law risk,META
  Tracked writer + no eligible lane,META
  DEC-XXX creation,MAR
  Multiple errors/issues,Error Search
  "Choose between X,Y,Z",5-SOL
```

---

## Score Formula

```yaml
FORMULA: C×2 + R×3 + S×3 + N×2 + F×1
RANGE: 11-55
```

```toon
factors[5]{factor,weight,1,3,5}:
  C (Complexity),×2,Simple,Multi-factor,Cross-cutting
  R (Reversibility),×3,Easy rollback,Partial,Irreversible
  S (Stakes),×3,Low,Module,System-wide
  N (Novelty),×2,Known,Partial,First time
  F (Frequency),×1,Daily,Weekly,One-time
```

---

## Scoring Contract

```yaml
SCORING_LAYERS:
  selector_score:
    purpose: "Routing only; never reuse as a quality or confidence score"
    scale: "11-55"
    priority: "stakes and reversibility outrank raw structural complexity"

  algorithm_raw_score:
    PR-CoT: "Issue + severity assessment"
    MAR: "1-10 weighted rubric score"
    5-SOL: "1-5 category scoring per option plus weighted option percent"
    META: "0-100 weighted confidence over active block families"

  handoff_rule:
    - "Every algorithm must export its gate result and a normalized signal"
    - "Admissibility gates override any raw numeric score"
```

```yaml
RISK_ESCALATORS:
  purpose: "Raise routing class when governance risk exceeds apparent implementation complexity"
  rules:
    - "If protocol conflict, execution gate mismatch, or fail-closed policy ambiguity is present, route to META regardless of raw selector score"
    - "If the task mutates framework-owned behavior or canonical routing rules, route to META regardless of raw selector score"
    - "If tracked writer execution encounters no eligible analysis lane, no eligible verifier, or no eligible coach and a policy decision is needed, route to META"
    - "If the task is mostly local implementation with no governance/policy ambiguity, keep the score-selected route"
```

```yaml
RETROSPECTIVE_ESCALATION:
  purpose: "Prevent repeated low-grade routing when STC already misclassified the task class"
  confirmed_stc_misfire:
    definition:
      - "STC selected first"
      - "review, gate, or later evidence proves the primary issue was protocol/policy/route design rather than local execution"
      - "the task required substantive rework or route reinterpretation"
    effect:
      - "ban immediate re-selection of STC for the same task class in the current pass"
      - "promote the next route to at least PR-CoT"
      - "promote directly to META when the misfire involved protocol conflict, fail-closed law, tracked writer routing, or framework-owned behavior"
  evidence:
    - "review finding"
    - "gate finding"
    - "root-cause receipt"
    - "tracked rework evidence"
```

---

## Execution Protocol

```yaml
STEPS:
  1. Check bug-first and high-risk overrides
  2. Calculate score (C×2 + R×3 + S×3 + N×2 + F×1) if no override matched
  3. Apply risk escalators and retrospective escalation rules before binding the final route
  4. Select the named flow template or bug lane
  5. Execute internally:
     - If META: assemble the smallest lawful block flow from the registry below
     - Named algorithms may be used inside META only as exact template shortcuts
  6. If external facts affect the decision, delegate web validation to `vida/config/instructions/runtime-instructions.web-validation-protocol.md`
  7. Preserve concise execution receipts: selected blocks, gates, impact analysis, and any escalation reason
```

---

## Atomic Block Registry

```yaml
BLOCK_REGISTRY:
  routing:
    SEL-01: "override_check -> forced_flow, reason"
    SEL-02: "complexity_score -> score, factor_breakdown"
    SEL-03: "route_bind -> flow_template"
    SEL-04: "web_validation_gate -> wvp_required, validation_scope"
  context:
    CTX-01: "module_select -> module_ids"
    CTX-02: "module_adapt -> adapted_prompts"
    CTX-03: "plan_seed -> execution_plan"
  iteration:
    ITR-01: "step_generate_check -> candidate_step, pass_fail"
    ITR-02: "failure_localize -> first_bad_step, reason"
    ITR-03: "rollback_retry_with_knowledge -> revised_attempt, knowledge_list"
    ITR-04: "escalation_decide -> proceed|revise|escalate|ask_user"
  critique:
    CRT-01: "perspective_pass -> per_perspective_findings"
    CRT-02: "consensus_packet_build -> agreements, divergences, key_signal"
    CRT-03: "post_consensus_revision -> revised_findings, issue_count, critical_findings, validation_signal"
  refinement:
    RFX-01: "role_round_execute -> proposal, scores, gaps, reflector_guidance"
    RFX-02: "knowledge_list_maintain -> avoid_keep_list"
    RFX-03: "round_progress_evaluate -> accept|change_strategy|escalate, final_score, refinement_signal"
  options:
    OPT-01: "category_generate -> categories"
    OPT-02: "candidate_set_generate -> viable_options"
    OPT-03: "criteria_refine -> refined_criteria"
    OPT-04: "option_ledger_and_hybrid_synthesize -> option_ledger, hybrid_candidate, legality_receipt, fallback_option"
    OPT-05: "option_scoring_and_confidence -> best_option_percent, agreement_percent, options_signal, confidence_band"
  ensemble:
    ENS-01: "admissibility_gate -> allowed_to_synthesize, blocking_findings"
    ENS-02: "cross_flow_compare -> agreements, divergences, dominant_signals"
    ENS-03: "weighted_confidence -> normalized_signals, family_weights, confidence_percent"
    ENS-04: "divergence_repair_loop -> rerun_targets, updated_confidence"
    ENS-05: "final_synthesis -> final_decision, residual_risks"
  bug:
    BUG-01: "detect_classify -> bug_class, severity, route_hint"
    BUG-02: "trace_root_cause -> trace_graph, root_cause_receipt"
    BUG-03: "hypothesis_gate -> falsifiable_hypothesis, test_design"
    BUG-04: "resolve_route -> STC|PR-CoT|MAR|5-SOL|META"
  reporting:
    REP-01: "evidence_pack -> concise_execution_receipt"
    REP-02: "impact_analysis -> normalized_impact_section"
    REP-03: "execution_proof -> proof_of_required_blocks"
```

## Named Flow Templates

```yaml
FLOW_TEMPLATES:
  STC: [SEL-01, SEL-02, SEL-03, ITR-01, ITR-02, ITR-03, ITR-04, REP-01]
  PR-CoT: [SEL-01, SEL-02, SEL-03, CRT-01, CRT-02, CRT-03, ITR-04, REP-01, REP-02]
  MAR: [SEL-01, SEL-02, SEL-03, RFX-01, RFX-02, RFX-03, ITR-04, REP-01, REP-02]
  5-SOL: [SEL-01, SEL-02, SEL-03, CTX-01?, CTX-02?, OPT-01, OPT-02, OPT-03, OPT-04, OPT-05, REP-01, REP-02]
  META: [SEL-01, SEL-02, SEL-03, CTX-01, CTX-02, CTX-03, selected_block_families, ENS-01, ENS-02, ENS-03, ENS-04?, ENS-05, REP-01, REP-02, REP-03]
  Error Search: [BUG-01, BUG-02, BUG-03, BUG-04]
```

## Section: stc

# STC: Stepwise Think-Critique

> Internal step-check algorithm for low-complexity and local tasks.

```yaml
PREREQ: vida/config/instructions/instruction-contracts.thinking-protocol.md#section-algorithm-selector  # Understand when STC applies (Score ≤12)
```


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER skip the internal step check.
⛔ NEVER repeat a failed approach; you MUST update and read the TRT knowledge list.
⛔ NEVER expose intermediate reasoning steps to the user.
✅ MUST maintain a clean prefix (keep steps 1 to X-1) during rollback.
✅ MUST escalate to PR-CoT if max retries (3) are reached without a solution.
</constraints>

## Triggers

```toon
triggers[4]{condition,mode}:
  Score ≤ 12,MANDATORY
  local objective,CONDITION
  low blast radius,CONDITION
  bug/root-cause gate required,ESCALATE to Error Search
```

---

## Block Assembly

```yaml
BLOCKS: [SEL-01, SEL-02, SEL-03, ITR-01, ITR-02, ITR-03, ITR-04, REP-01]
QUALITY_GATE:
  - local objective resolved
  - no unresolved step error remains
```

---

## Algorithm (Thought-ICS + TRT Knowledge)

```yaml
STEP_GENERATION:
  format: "STEP {n}: {thought}"
  max_steps: 10
  
  on_each_step:
    1. Generate STEP n
    2. INTERNAL check: correct? (0/1)
    3. If 1 → continue to STEP n+1
    4. If 0 → LOCALIZATION

LOCALIZATION:
  prompt: |
    Analyze each numbered step.
    Find first logical error, wrong calculation, or false assumption.
    Output: "ERROR in STEP X: {reason}"
    
ROLLBACK:
  action: Keep STEPs 1 to X-1 (clean prefix)
  regenerate: New STEP X (alternative approach)
  max_retries: 3
  
  knowledge_list:
    purpose: "Accumulate what FAILED and WHY across retries"
    on_each_failure:
      1. Record: "AVOID: {failed_approach} because {reason}"
      2. Append to knowledge_list
      3. Pass knowledge_list as context to next retry
    format: |
      KNOWLEDGE (retry {N}):
        - AVOID: {approach_1} because {reason_1}
        - AVOID: {approach_2} because {reason_2}
    effect: "Next retry sees ALL previous failures → no blind repeats"

  on_max: Escalate to PR-CoT; ask user only if clarification/data blocker remains
```

**Flow:** STEP 1 → check → STEP 2 → ... → ERROR? → LOCALIZE → knowledge += failure → ROLLBACK → retry (informed)

---

## Reporting Rules

```toon
reporting[4]{state,action}:
  during,No user-visible step trace
  complete,Conclusion only
  error,Simplify + clarify
  on_request,"Provide concise decision/evidence summary only"
```

---

## Escalation

```yaml
ESCALATE_IF:
  max_retries: 3
  on_fail: PR-CoT
  protocol_or_route_ambiguity: META
  confirmed_stc_misfire: PR-CoT_or_META_per_retrospective_escalation
  clarification_blocker: ask_user
  pass_context: knowledge_list (so PR-CoT sees what STC already tried)
```


## Section: pr-cot

# PR-CoT: Poly-Reflective Validation

> 4-perspective validation plus consensus revision. Score 13-22. Escalate to MAR if issues >= 2.

```yaml
PREREQ: vida/config/instructions/instruction-contracts.thinking-protocol.md#section-algorithm-selector  # Understand when PR-CoT applies (Score 13-22)
```


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER skip running all 4 perspectives independently in Pass 1.
⛔ NEVER fabricate consensus. The consensus packet must strictly reflect Pass 1 findings.
⛔ NEVER skip Pass 2 (Post-Consensus Revision). Each perspective MUST review the packet.
✅ MUST escalate to MAR if final issue count >= 2.
✅ MUST evaluate alternatives specifically for trade-offs and simplicity.
</constraints>

## Triggers

```toon
triggers[4]{trigger,type}:
  score 13-22,MANDATORY
  medium-complexity decision,CONDITION
  multiple perspectives needed,CONDITION
  STC exhausted,ESCALATION
```

---

## Block Assembly

```yaml
BLOCKS: [SEL-01, SEL-02, SEL-03, CRT-01, CRT-02, CRT-03, ITR-04, REP-01, REP-02]
QUALITY_GATE:
  - no unresolved critical findings
  - impact analysis ready for closure or handoff
```

---

## 4 Perspectives (TMK-Structured)

```yaml
PERSPECTIVES:

  logical:
    icon: 🔍
    task: "Identify logical flaws in {decision}"
    method:
      - Extract all premises from the decision
      - Verify each inference follows from premises
      - Check for circular dependencies or gaps
    knowledge: "A1 (critical_thinking) — valid inference patterns, common fallacies"
    questions: "Follows premises? Circular logic? Justified jumps?"

  data:
    icon: 📊
    task: "Identify missing context and hidden assumptions"
    method:
      - List all data inputs the decision relies on
      - Check each for availability and accuracy
      - Identify implicit assumptions not stated
    knowledge: "V1 (assumptions) — hidden constraints, domain context"
    questions: "What NOT considered? Implicit assumptions? Hidden constraints?"

  arch:
    icon: 🏗️
    task: "Check alignment with existing architecture and decisions"
    method:
      - Map decision to existing DEC-XXX decisions
      - Check pattern consistency with codebase
      - Assess technical debt impact
    knowledge: "A2 (systems_thinking) — interdependencies, feedback loops"
    questions: "Aligned with decisions? Existing patterns? Creates debt?"

  alternatives:
    icon: 🔀
    task: "Evaluate unexplored approaches and trade-offs"
    method:
      - Generate 2-3 alternative approaches
      - For each: why NOT chosen? What trade-off?
      - Check if simpler solution exists
    knowledge: "G2 (alternative_perspectives) — different viewpoints"
    questions: "Other approaches? Why NOT them? Simpler solution?"
```

---

## Execution (2-Pass with Consensus)

```yaml
PASS_1:
  step: 1
  action: State decision to validate
  
  step: 2
  action: Run 4 perspectives INDEPENDENTLY (TMK-structured)
  
  step: 3
  action: Build consensus packet

CONSENSUS_PACKET:
  format: |
    agreements: [{perspectives that found same issue}]
    unique_findings: [{perspective}: {finding}]
    divergences: [{perspective A} says X, {perspective B} says Y]
    key_signal: {strongest finding across all perspectives}

PASS_2:
  step: 4
  action: Each perspective REVIEWS consensus packet (single revision)
  rules:
    - If another perspective already covers your finding → CONFIRM, don't duplicate
    - If consensus reveals your finding was wrong → REVISE or DROP
    - If consensus has gap you can fill → ADD new insight
    - Max 1 revision per perspective

  step: 5
  action: Count FINAL issues (post-revision, deduplicated)

QUALITY_GATE:
  proceed: "0 issues and no unresolved critical findings"
  revise: "1 non-critical issue"
  escalate: "unresolved critical findings OR >=2 issues"
```

---

## Scoring Export

```yaml
SCORING_EXPORT:
  issue_weights:
    critical: 1.00
    major_non_critical: 0.50
    minor_non_critical: 0.25

  validation_signal:
    formula: "clamp(1 - sum(open_issue_weights), 0, 1)"

  interpretation:
    "1.00": "Proceed"
    "0.75": "Revise once"
    "<=0.50": "Escalate or block"

  handoff:
    pass: critical_findings, final_issue_count, validation_signal
```

---

## Evidence Packet

```markdown
## PR-CoT: {Decision}

### Pass 1: Independent Perspectives
🔍 Logical: {findings or "No issues"}
📊 Data: {findings or "Complete"}
🏗️ Arch: {DEC-XXX refs or "Aligned"}
🔀 Alternatives: {why not chosen}

### Consensus Packet
**Agreements:** {list}
**Divergences:** {list}
**Key Signal:** {strongest finding}

### Pass 2: Post-Consensus Revision
🔍 Logical: {confirmed | revised | dropped}
📊 Data: {confirmed | revised | dropped}
🏗️ Arch: {confirmed | revised | dropped}
🔀 Alternatives: {confirmed | revised | dropped}

**Critical Findings:** {list or "None"}
**Final Issues:** {count} → **Action:** {proceed|revise|escalate}
**Validation Signal:** {0..1}

### Impact Analysis
**Affected Scope:** {files/modules/layers}
**Contract Impact:** {api/data/protocol/dependency impact or "None"}
**Operational Impact:** {user/operator/runtime impact or "None"}
**Follow-up:** {docs/spec/reflection/pool/verification actions or "None"}
**Residual Risks:** {list or "None"}
```

---

## Escalation

```yaml
ESCALATE_TO_MAR:
  trigger: issues >= 2 OR unresolved_critical_findings
  pass: original_decision, issues_found, perspectives, consensus_packet, critical_findings, final_issue_count, validation_signal, impact_analysis, external_validation_evidence
```


## Section: mar

# MAR: Multi-Agent Reflexion

> 3 rounds x 4 roles. Score 23-32. Escalate to META if final score < 8.

```yaml
PREREQ: [vida/config/instructions/instruction-contracts.thinking-protocol.md#section-algorithm-selector, vida/config/instructions/instruction-contracts.thinking-protocol.md#section-pr-cot]  # MAR builds on PR-CoT
```


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER fabricate agent responses; you must simulate 4 distinct expert agents.
⛔ NEVER end the debate before Round 3.
✅ MUST aggregate scores strictly based on actual agent votes.
✅ MUST preserve unresolved disagreements in the evidence packet and final risk report.
✅ MUST block acceptance when unresolved critical contract/safety/root-cause risks remain.
</constraints>

## Triggers

```toon
triggers[5]{trigger}:
  Score 23-32
  DEC-XXX creation
  Complex trade-offs
  Novel solutions
  PR-CoT escalation (issues ≥ 2)
```

---

## Block Assembly

```yaml
BLOCKS: [SEL-01, SEL-02, SEL-03, RFX-01, RFX-02, RFX-03, ITR-04, REP-01, REP-02]
QUALITY_GATE:
  - final_score >= 8
  - no unresolved critical contract/safety/root-cause risks
```

---

## Architecture

```toon
agents[4]{role,task}:
  Actor,Generate solution (with HADI heuristics)
  Evaluator,Score with adaptive rubrics (RRD)
  Critic,Find flaws and gaps
  Reflector,Synthesize + knowledge list for next round
```

---

## Round Flow

```yaml
ROUND_N:
  ACTOR: |
    Generate solution.
    Use HADI heuristics (abduction, constraint inversion).
    IF Round > 1: incorporate Reflector guidance + AVOID items from knowledge_list.
    
  EVALUATOR: |
    Score solution 1-10 using ADAPTIVE RUBRICS:
    
    base_rubrics: [correctness, completeness, alignment, simplicity]
    
    adaptive_decomposition:
      trigger: "rubric scores SAME for R(N) vs R(N-1)"
      action: "Decompose into finer sub-rubrics"
      example:
        correctness → [logic_correctness, edge_case_handling, data_validity]
        completeness → [feature_coverage, error_handling, documentation]
        alignment → [pattern_consistency, dec_xxx_compliance, debt_impact]
        simplicity → [cognitive_complexity, abstraction_level, dependency_count]
    
    misalignment_filter:
      trigger: "sub-rubric conflicts with known-good patterns"
      action: "Remove sub-rubric with documented reason"
    
    correlation_weight:
      trigger: "two sub-rubrics always score identically"
      action: "Downweight one to avoid double-counting"
    
  CRITIC: |
    List flaws, missing cases, risks.
    Reference specific rubric gaps from Evaluator: "Evaluator scored {rubric} low because..."
    
  REFLECTOR: |
    Synthesize for next round:
    
    1. PAIRWISE COMPARE: R(N) solution vs R(N-1) solution
       - What improved? What regressed? What unchanged?
       
    2. KNOWLEDGE LIST (accumulated):
       - Carry forward ALL items from previous rounds
       - Add new: "AVOID: {approach} because {reason}"
       - Add new: "KEEP: {element} because {proven in R(N)}"
       
    3. STRATEGY GENERATION:
       - 2-3 NEW angles Actor hasn't tried
       - Based on Critic gaps + knowledge_list
       
    4. PASS TO ACTOR R(N+1):
       {improvements, knowledge_list, strategies}
```

---

## Knowledge List Format

```yaml
KNOWLEDGE_LIST:
  purpose: "Prevent re-exploration of failed approaches, preserve proven elements"
  
  format: |
    KNOWLEDGE (Round {N}):
      AVOID:
        - {approach_1}: {reason} (from R{X})
        - {approach_2}: {reason} (from R{Y})
      KEEP:
        - {element_1}: {why it works} (proven in R{X})
        - {element_2}: {why it works} (proven in R{Y})
  
  rules:
    - Actor MUST read knowledge_list before generating
    - Actor MUST NOT reuse AVOID items
    - Actor SHOULD preserve KEEP items unless Critic explicitly flags them
    - Reflector MUST carry ALL items forward (never remove from AVOID)
```

---

## Scoring

```yaml
SCORING:
  parent_rubric_weights:
    correctness: 0.35
    completeness: 0.25
    alignment: 0.25
    simplicity: 0.15
  decomposition_rule: "If a parent rubric is decomposed, split that parent's weight evenly across its active sub-rubrics, then aggregate back into the parent before the final average"
  overall: "1-10 weighted average of parent rubric scores"
  normalized_signal: "clamp(final_score / 10, 0, 1)"
  rubric_count: 4 base, up to 12 decomposed
  
  escalation:
    score < 6: "Critical gaps remain"
    score 6-7: "Acceptable with known limitations"
    score >= 8: "Strong solution, minimal gaps"
    
  round_progression:
    expected: "R1: 5-6 → R2: 7-8 → R3: 8-9"
    stagnation: "If R(N) score == R(N-1) score → Reflector must change strategy"

ACCEPT_GATE:
  required:
    - final_score >= 8
    - no unresolved critical contract/safety/root-cause risks
    - knowledge_list retained in final recommendation

EARLY_EXIT:
  default: forbidden
```

---

## Evidence Packet

```markdown
## MAR: {Problem}

### Rounds
**R1:** {score}/10 | {key critique}
**R2:** {score}/10 | {key improvement}
**R3:** {score}/10 | {accepted best solution}

### Knowledge List
**Avoid:** {items}
**Keep:** {items}

### Remaining Disagreements
**Open:** {list or "None"}

### Decision
**Best Solution:** {summary}
**Action:** {accept|escalate}
**Refinement Signal:** {0..1}

### Impact Analysis
**Affected Scope:** {files/modules/layers}
**Contract Impact:** {api/data/protocol/dependency impact or "None"}
**Operational Impact:** {user/operator/runtime impact or "None"}
**Follow-up:** {docs/spec/reflection/pool/verification actions or "None"}
**Residual Risks:** {list or "None"}
```

---

## Escalation

```yaml
ESCALATE_TO_META:
  trigger: final_score < 8 after 3 rounds
  pass: all_rounds, knowledge_list, best_solution, rubric_scores, final_score, refinement_signal, impact_analysis, unresolved_disagreements, external_validation_evidence
```


## Section: 5-solutions

# 5-SOL: 5-Solutions Algorithm

> Alignment algorithm. Score 33-42. Generates 5 options x 2 rounds and synthesizes the best hybrid.

---

## Purpose

- Many valid directions → synthesize the best admissible hybrid or top single option
- Architecture / tech-stack / design choices → balanced solution
- Use only after bug reasoning for regressions; do not substitute for root-cause receipt


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER provide fewer than 5 distinct solutions in Round 1.
⛔ NEVER fabricate options; they must be technically viable and architecturally sound.
✅ MUST generate a HYBRID or optimized solution in Round 2 based on Round 1 analysis.
✅ MUST strictly evaluate Trade-offs and Risks for every proposed option.
</constraints>

## Triggers

```toon
triggers[4]{type,condition}:
  score,33-42
  keywords,"choose between | which approach | architecture alignment | tech stack | migration strategy"
  in_meta,Selected by META only when multiple viable directions remain
  skip,Score ≤32 OR simple decisions
```

---

## Block Assembly

```yaml
BLOCKS: [SEL-01, SEL-02, SEL-03, CTX-01?, CTX-02?, OPT-01, OPT-02, OPT-03, OPT-04, OPT-05, REP-01, REP-02]
QUALITY_GATE:
  - at least one admissible final decision exists
  - best_final_option_percent >= 80
  - confidence_percent >= 80
  - hybrid legality passed OR best single option selected explicitly
  - if either score or confidence is 80-84, record an explicit cautious-band receipt
```

---

## Algorithm Flow

```yaml
FLOW:
  step_0: Generate 4-6 dynamic categories (or load a domain packet)
  step_1: Run WVP when external facts, package/API/platform claims, or security assumptions affect the choice
  R1: Generate 5 options → score → HYBRID R1
  
  CATEGORY_CHECK: # RRD adaptive decomposition
    for_each_category:
      if all 5 options score SAME (±0.5):
        action: "Category too coarse → DECOMPOSE into 2-3 sub-categories"
        example: "Type Safety → compile_time_safety, null_safety, refactoring_confidence"
      if category correlates >0.8 with another:
        action: "MERGE or DOWNWEIGHT (prevent double-counting)"
    rescore: "Re-evaluate R1 with refined categories if decomposed"
  
  CONSENSUS: # PACER consensus packet R1→R2
    build_packet:
      top_options: "R1 top 2-3 scorers with reasons"
      winning_elements: "Best element per category"
      failure_reasons: "Why bottom options lost"
      compatibility_constraints: "Which winning elements can or cannot coexist"
      unresolved_gaps: "What R1 couldn't decide"
    pass_to_R2: true
  
  R2: Generate 5 NEW options CONDITIONED on consensus packet
    rules:
      - MUST address unresolved_gaps from R1
      - MUST NOT repeat R1 failure patterns
      - MAY build on winning_elements from R1
      - At least 2 options must explore NEW angles
    score_with: refined categories (from CATEGORY_CHECK)
    result: HYBRID R2
  
  FINAL: Compare R1 vs R2 → FINAL HYBRID or TOP SINGLE OPTION
  OUTPUT: Best option score % + confidence % + decision
```

---

## Dynamic Categories

```toon
domains[7]{problem,categories}:
  auth,"Security | Compliance | Usability | Performance | Maintainability"
  architecture,"Fit | Migration Path | Operability | Coupling | Future Optionality"
  ui,"UX | Accessibility | Consistency | Performance | Effort"
  data,"Integrity | Migration | Scalability | Query Perf | Cost"
  state_mgmt,"Scalability | Testability | Type Safety | Boilerplate | Learning Curve"
  tech_stack,"Operability | Lock-in | Ecosystem | Migration Cost | Learning Curve"
  error_align,"Root Cause | Regression Risk | Scope | Side Effects | Testing"
```

---

## Category Weighting

```yaml
CATEGORY_WEIGHTING:
  choose_core_categories:
    count: 2
    rule: "Select the 2 decision-critical categories before scoring; they carry more weight than supporting categories"

  weights:
    core_category_1: 0.25
    core_category_2: 0.25
    supporting_categories: "Share the remaining 0.50 equally"

  default_cores:
    auth: [Security, Compliance]
    architecture: [Fit, Migration Path]
    ui: [UX, Accessibility]
    data: [Integrity, Migration]
    state_mgmt: [Scalability, Testability]
    tech_stack: [Operability, Lock-in]
    error_align: [Root Cause, Regression Risk]
```

---

## Self-Discover SELECT (Optional)

```yaml
TRIGGER: multi-domain problem OR explicit need for higher option diversity
ACTION: Select 2-3 modules from `vida/config/instructions/instruction-contracts.thinking-protocol.md#section-reasoning-modules` → inform categories
MAPPING:
  A2 (systems) → "Integration" category
  A3 (risk) → "Failure Modes" category
  G2 (alternatives) → "Approach Diversity" category

DOMAIN_PACKETS:
  security_auth: [A3, V1, V2, V4]
  database_schema: [A2, A3, V1, V2, V4]
  architecture_or_tech_stack: [A1, A2, A3, G2, V2]
  post_root_cause_fix_alignment: [A4, A5, D2, V4]

RULE:
  - If one of these domains drives the choice, load that packet before generating categories.
  - For regression work, this packet is lawful only after bug reasoning has produced a root_cause_receipt.
```

---

## HADI Abduction (Round 1)

```yaml
REQUIREMENT: 2 of 5 options MUST use Abduction
TECHNIQUES:
  constraint_inversion: "What if [constraint] didn't exist?"
  concept_combination: "Combine [A] with [domain B]"
  extreme_scenarios: ideal / catastrophic / unexpected
  analogy_transfer: "How does [other industry] solve this?"
```

---

## Scoring

```toon
scale[5]{score,meaning}:
  1,Poor
  2,Below Average
  3,Average
  4,Good
  5,Excellent
```

```yaml
SCORING_MODEL:
  per_category_scale: "1-5"
  weighted_option_score:
    formula_1_5: "sum(category_score * category_weight)"
    option_percent: "round((weighted_option_score / 5) * 100)"

  options_signal:
    formula: "clamp((0.6 * (best_final_option_percent / 100)) + (0.4 * (agreement_percent / 100)), 0, 1)"

  rule: "Admissibility and hybrid legality override raw option score"
```

---

## Hybrid Formation

```yaml
HYBRID_R1:
  1. Find highest scorer per category
  2. Extract winning elements
  3. Check compatibility
  4. Synthesize coherent solution

HYBRID_R2:
  1. Find highest scorer per REFINED category
  2. Extract winning elements (informed by consensus)
  3. Check compatibility with R1 winning elements
  4. Synthesize coherent solution

FINAL_HYBRID:
  1. Compare R1 vs R2 per category
  2. Pick better approach each
  3. Resolve conflicts (R2 preferred if it addresses R1 gaps)
  4. Calculate confidence
```

---

## Hybrid Legality

```yaml
HYBRID_LEGALITY:
  allow_only_if:
    - winning elements are compatible
    - hybrid improves or preserves the top 2 criteria versus the best single option
    - implementation order is coherent
  otherwise:
    action: "Choose the best single option explicitly; do not force a hybrid"
```

---

## Consensus Packet Format

```yaml
CONSENSUS_PACKET:
  format: |
    ## R1 Consensus
    **Top options:** {option_A (score), option_B (score)}
    **Winning elements:** {category: element, ...}
    **Failed approaches:** {option: reason, ...}
    **Unresolved gaps:** {gap_1, gap_2}
    **Category refinements:** {if any decomposed}
```

---

## Confidence

```toon
confidence[4]{agreement,percent}:
  R1 = R2 and legality stable,90-95%
  R2 improves R1 and keeps core categories admissible,85-89%
  Admissible decision but trade-off tension remains,80-84%
  Unresolved gaps OR legality pressure,<80% → META (WVP only if external claims drive disagreement)
```

---

## WVP Re-entry

```yaml
IF_WVP_RUNS_AFTER_R1_OR_R2:
  - re-score affected categories
  - rebuild option_ledger
  - recompute confidence
  - only then lock FINAL decision
```

---

## Evidence Packet

```markdown
## 5-SOL: {Problem}

**Categories:** {C1, C2, C3, C4, C5}
**Core Categories:** {C1, C2}

**R1:** {5 options table} → **HYBRID R1:** {synthesis}

**Category Check:** {decomposed: C2→C2a,C2b | merged: C3+C5 | unchanged: C1,C4}

**Consensus Packet:** {top options, gaps, winning elements}

**R2 (informed):** {5 options table} → **HYBRID R2:** {synthesis}

**FINAL DECISION:** {hybrid or best single option}
**Best Final Option Score:** {XX}%
**Hybrid Legality:** {pass | fail -> used best single option}
**Confidence:** {XX}% | **Options Signal:** {0..1}
**Files:** {affected} | **Order:** {sequence}

### Impact Analysis
**Affected Scope:** {files/modules/layers}
**Contract Impact:** {api/data/protocol/dependency impact or "None"}
**Operational Impact:** {user/operator/runtime impact or "None"}
**Follow-up:** {docs/spec/reflection/pool/verification actions or "None"}
**Residual Risks:** {list or "None"}
```

---

## Escalation

```yaml
ESCALATE_TO_META:
  trigger: confidence < 80 OR no admissible final decision
  pass: categories, core_categories, option_ledger, final_decision, best_final_option_percent, agreement_percent, options_signal, legality_receipt, impact_analysis, external_validation_evidence
```

---

## Anti-Patterns

```toon
forbidden[6]{action,why,correct}:
  Reuse R1 options in R2,Defeats refinement,Generate 5 NEW options
  Fixed categories,Loses domain insight,Generate dynamic categories
  Skip HYBRID formation,Loses synthesis value,Always form hybrids
  Stop at R1 if "good enough",Misses refinement,Always do both rounds
  Ignore low-scoring options,May have valuable elements,Extract best from each
  Skip consensus packet,R2 ignores R1 learnings,Always build consensus R1→R2
```


## Section: meta-analysis

# META: Meta-Analysis

> Block-level composer for high-risk tasks. Score >42. Builds the smallest lawful flow from reusable blocks, then synthesizes only admissible results.

```yaml
PREREQ: [vida/config/instructions/instruction-contracts.thinking-protocol.md#section-algorithm-selector, vida/config/instructions/instruction-contracts.thinking-protocol.md#section-reasoning-modules]  # META composes from the block registry and domain packets
```


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER default to whole PR-CoT, MAR, and 5-SOL execution when a smaller lawful block flow can answer the task.
⛔ NEVER synthesize past unresolved critical findings, unresolved root cause, or failed hybrid legality.
⛔ NEVER aggregate mixed confidence scales; normalize every active signal to 0..1 first.
✅ MUST start with Step 0 domain classification + block selection.
✅ MUST carry forward impact analysis from every active block family into final synthesis.
</constraints>

## Triggers

```toon
triggers[7]{type,condition}:
  score,>42
  override,Security/Auth decisions
  override,Database schema
  override,Foundation architecture
  override,Tech stack selection
  override,Explicit meta-analysis request
  skip,Score ≤42 OR already inside META
```

---

## Step 0: Domain Classification And Block Selection

```yaml
STEP_0:
  classify_task:
    classes:
      - security_auth
      - database_schema
      - foundation_architecture
      - tech_stack_selection
      - bug_root_cause
      - multi_option_design
      - general_high_risk

  select_domain_packet:
    security_auth: [A3, V1, V2, V4]
    database_schema: [A2, A3, V1, V2, V4]
    foundation_architecture: [A1, A2, A3, G2, V2]
    tech_stack_selection: [A1, A2, A3, G2, V2]
    bug_root_cause: [A4, A5, D2, V4]
    multi_option_design: [A1, A2, G2, V2, M2]
    general_high_risk: "Select 3-5 modules by fit"

  choose_blocks:
    always:
      - CTX-01
      - CTX-02
      - CTX-03
      - ENS-01
      - ENS-02
      - ENS-03
      - ENS-05
      - REP-01
      - REP-02
      - REP-03
    add_if:
      validation_or_assumption_risk: [CRT-01, CRT-02, CRT-03]
      candidate_needs_refinement: [RFX-01, RFX-02, RFX-03]
      multiple_viable_options: [OPT-01, OPT-02, OPT-03, OPT-04, OPT-05]
      bug_or_regression_centered: [BUG-01, BUG-02, BUG-03, BUG-04]
      active_family_divergence: [ENS-04]

  shortcut_rule:
    named_templates: "STC / PR-CoT / MAR / 5-SOL may be used only when their canonical block set exactly matches the chosen flow"
```

---

## Family Weights

```yaml
FAMILY_WEIGHTS:
  default:
    critique: 0.35
    refinement: 0.30
    options: 0.20
    bug: 0.15

  security_auth:
    critique: 0.40
    refinement: 0.30
    options: 0.20
    bug: 0.10

  database_schema:
    critique: 0.35
    refinement: 0.30
    options: 0.15
    bug: 0.20

  foundation_architecture:
    critique: 0.25
    refinement: 0.30
    options: 0.35
    bug: 0.10

  tech_stack_selection:
    critique: 0.20
    refinement: 0.25
    options: 0.45
    bug: 0.10

  bug_root_cause:
    critique: 0.25
    refinement: 0.20
    options: 0.10
    bug: 0.45

  multi_option_design:
    critique: 0.20
    refinement: 0.25
    options: 0.45
    bug: 0.10

  general_high_risk: default
```

---

## Composer Law

```yaml
RULES:
  1. Prefer the smallest block flow that can answer the task lawfully.
  2. Validation blocks decide admissibility.
  3. Refinement blocks improve candidate quality.
  4. Option blocks are required only when 2+ viable directions exist.
  5. Bug blocks are mandatory before any fix synthesis for bugs/incidents/regressions.
```

---

## Execution Protocol

```yaml
PASS_1:
  step: 1
  action: "Execute selected blocks in order; independent families may run in parallel"
  
  step: 2
  action: "Compare outputs from active block families"
  
  step: 3
  action: "Run WVP when SEL-04 says unstable external claims affect the active flow"

  step: 4
  action: "If WVP changes assumptions, risks, or categories -> re-run only affected block families before synthesis"

ENS-01_ADMISSIBILITY_GATE:
  block_synthesis_if:
    - CRT-03 reports unresolved critical findings
    - RFX-03 reports unresolved critical contract/safety/root-cause risks
    - BUG-02 root_cause_receipt missing when bug blocks are active
    - OPT-04 legality_receipt == fail and no fallback_option chosen
  allowed_next_steps:
    - revise flow
    - add a missing block family
    - run ENS-04 divergence repair
    - return a cautious decision only with explicit residual-risk receipt

ENS-03_WEIGHTED_CONFIDENCE:
  normalize_to_0_1:
    critique_signal: "validation_signal if CRT blocks are active, otherwise n/a"
    refinement_signal: "clamp(final_score / 10, 0, 1)"
    options_signal: "OPT-05 exported options_signal"
    bug_signal: "1 if root_cause_receipt confirmed else 0"
  aggregation:
    family_profile: "Load weights from FAMILY_WEIGHTS for the Step 0 task class"
    active_signals_only: true
    renormalize_active_weights: true
    formula: "round(100 * sum(active_signal * normalized_active_family_weight))"
  decision_bands:
    "85-100": "SYNTHESIZE"
    "80-84": "SYNTHESIZE with caution + explicit residual risks"
    "<80": "ENS-04 divergence repair"

ENS-04_DIVERGENCE_REPAIR:
  purpose: "Repair only the divergent part of the composer instead of restarting the whole flow"
  max_loops: 2
  
  on_each_loop:
    1. ANALYZE divergences between active block families:
       - Which family diverges most?
       - What specific issue remains disputed?
       - Build knowledge_list: {what's agreed, what's disputed, why}
       
    2. RE-RUN only the affected blocks/families:
       - Inject knowledge_list as extra context
       - Knowledge includes findings from OTHER active families
       - Preserve existing admissible findings
       
    3. RE-COMPARE:
       - Re-check ENS-01 admissibility gate
       - Recalculate ENS-03 weighted confidence
       - If >= 80% and admissible: SYNTHESIZE → done
       - If < 80% and loops < 2: next repair loop
       - If < 80% and loops == 2: choose best admissible non-hybrid option OR ask user to resolve the trade-off
  
  knowledge_format: |
    TRT CONTEXT (Loop {N}):
      Agreed: {points all active families concur on}
      Disputed: {specific disagreement}
      Validation says: {summary or n/a}
      Refinement says: {summary or n/a}
      Options says: {summary or n/a}
      Bug says: {summary or n/a}
      YOUR TASK: Address {disputed points} specifically
```

---

## Confidence Matrix

```yaml
CONFIDENCE:
  admissible_and_high:
    range: 85-100%
    action: Proceed
    
  admissible_but_cautious:
    range: 80-84%
    action: Proceed with caution + explicit residual risks
    
  repair_needed:
    range: <80%
    action: ENS-04 divergence repair
  
  inadmissible:
    range: any
    action: Block synthesis until ENS-01 passes
```

---

## Evidence Packet

```markdown
## META: {Decision}

### Step 0
**Task Class:** {security_auth|database_schema|foundation_architecture|tech_stack_selection|bug_root_cause|multi_option_design|general_high_risk}
**Selected Modules:** {A1, A2, ...}
**Selected Blocks:** {CRT-01, CRT-02, RFX-01, OPT-04, ...}
**Why This Flow:** {smallest lawful rationale}

### Active Families
**Validation (CRT):** {issues or "not selected"} | **Signal:** {0..1 or "n/a"}
**Refinement (RFX):** {score}/10 or "not selected" | **Signal:** {0..1 or "n/a"}
**Options (OPT):** {hybrid or top single option or "not selected"} | **Legality:** {pass|fail|n/a} | **Signal:** {0..1 or "n/a"}
**Bug (BUG):** {root cause receipt or "not selected"} | **Signal:** {0|1|n/a}

### Composer Gates
**Admissibility:** {pass|fail}
**Blocking Findings:** {list or "None"}
**Weighted Confidence:** {XX}% from normalized signals {list} and active family weights {list}

### Divergence Repair (if triggered)
**Loop {N}:** Re-ran {blocks/family} with knowledge: {context}
**Result:** Confidence {before}% → {after}% | **Admissibility:** {pass|fail}

### FINAL DECISION
{synthesized decision}
**Flow Used:** {ordered block ids}
**Files:** {list} | **Residual Risks:** {list}

### Impact Carry-Forward
**Validation Impact:** {how critique findings affected final decision or "n/a"}
**Refinement Impact:** {how refinement changed the candidate or "n/a"}
**Options Impact:** {which option trade-offs survived into final decision or "n/a"}
**Bug Impact:** {how root cause evidence constrained the fix or "n/a"}

### Impact Analysis
**Affected Scope:** {files/modules/layers}
**Contract Impact:** {api/data/protocol/dependency impact or "None"}
**Operational Impact:** {user/operator/runtime impact or "None"}
**Follow-up:** {docs/spec/reflection/pool/verification actions or "None"}
**Residual Risks:** {list or "None"}
```

---

## Anti-Patterns

```toon
forbidden[5]{action,why}:
  Default to whole named algorithms,Defeats block-level optimization
  Synthesize past critical findings,Unsafe and non-admissible
  Build hybrid without legality,Produces mushy or incoherent result
  Mix raw scales in confidence,Confidence becomes non-reproducible
  Claim blocks ran without receipts,Fabrication
```

---

## Execution Proof (⛔ MANDATORY)

> Before synthesis, META must contain proof that the selected block flow actually ran:

```yaml
PROOF_REQUIRED:
  step_0_selection:
    evidence: "Task class: {...}. Modules: {...}. Blocks: {...}. Why minimal: {...}"

  active_family_execution:
    critique: "CRT-01/02/03 -> issues: {...} OR not selected"
    refinement: "RFX-01/02/03 -> score: {...} OR not selected"
    options: "OPT-01/02/03/04/05 -> final option: {...}, legality: {...} OR not selected"
    bug: "BUG-01/02/03/04 -> root_cause_receipt: {...} OR not selected"

  admissibility_gate:
    evidence: "ENS-01 => pass/fail. Blocking findings: {...}"

  weighted_confidence:
    evidence: "ENS-03 normalized signals: critique={...}, refinement={...}, options={...}, bug={...}. Active family weights={...}. Weighted={...}%"

  divergence_repair:
    evidence: "Loop {N}: re-ran {blocks}, confidence {before}% → {after}%"
    OR: "Not triggered (confidence >= 80%)"

  impact_carry_forward:
    evidence: "Validation impact={...}; Refinement impact={...}; Options impact={...}; Bug impact={...}"

NO_PROOF_NO_SYNTHESIS: |
  ⛔ If you cannot provide SPECIFIC evidence from each selected block family,
  you did NOT execute META correctly.
  
  ⛔ Statements like "94% confident" without normalized signals and gate receipts = FABRICATION.
  ⛔ Fabrication = PROTOCOL VIOLATION → restart META properly.
  
VALID_EXAMPLE: |
  ✓ "Task class: tech_stack_selection. Modules: [A1, A2, A3, G2, V2]. Blocks: [CTX-01, CTX-02, OPT-01..05, CRT-01..03, ENS-01..05]"
  ✓ "CRT: 1 critical lock-in finding resolved after revision. critique_signal=0.5"
  ✓ "OPT: top single option chosen because hybrid legality failed. options_signal=0.86"
  ✓ "ENS-03: normalized average = 84% -> SYNTHESIZE with caution"
```


## Section: bug-reasoning

# Error Search

> **MANDATORY.** Detect → Classify → Trace → Hypothesize → Resolve errors.
> **Iron Law:** NO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST.

---

## Pipeline Overview

```
Phase 0: DETECT    → Layer, type, is_regression?
Phase 1: CLASSIFY  → Severity, blast radius, technique triggers
Phase 2: TRACE     → 5 Whys + Git Bisect + dependency tracing
Phase 3: HYPOTHESIZE → 3 gates, self-correction
Phase 3.5: LLM BLOCK (if >50 LOC)
Phase 4: RESOLVE   → Algorithm selection, fix
```

---

## Block Assembly

```yaml
BLOCKS: [BUG-01, BUG-02, BUG-03, BUG-04]
QUALITY_GATE:
  - root_cause_receipt exists before Phase 4
  - route is chosen from one canonical severity map
```

---

## Phase 0: DETECT

```yaml
PATTERN_MATCH:
  API: ApiException, AccessDenied, SessionExpired, RecordNotFound
  Connection: ConnectionException, VersionDetection
  Auth: AuthError, AuthErrorType
  Network: NetworkException
  Data: FormatException, TypeError
  State: StateError, DatabaseStateError

OUTPUT: layer, type, severity_hint, is_regression
```

---

## Phase 1: CLASSIFY

```toon
factors[4]{factor,1,3,5}:
  blast_radius,"1-2 files","3-5 files","6+ files"
  user_impact,None,Degraded,Crash
  data_risk,None,Display wrong,Data loss
  frequency,Edge case,Common,Critical path
```

```yaml
SEVERITY_MAP:
  sum 1-8: LOW → STC
  sum 9-12: MEDIUM → PR-CoT
  sum 13-16: HIGH → MAR
  sum 17-20: CRITICAL → META
  multi_error_with_shared_root: 5-SOL

PRIORITY_RULE:
  if_critical_and_multi_error: "META first; 5-SOL only after root_cause_receipt when multiple admissible fixes remain"

TECHNIQUE_TRIGGERS:
  git_bisect: is_regression == true
  dependency_tracing: blast_radius >= 3
  llm_block: function LOC > 50
```

---

## Phase 2: TRACE

### Pre-Check (MANDATORY)

```yaml
BEFORE_ANY_FIX:
  - Error message READ completely
  - Stack trace analyzed to origin
  - Bug reproduced TWICE consistently
  - Recent changes reviewed (git log -5)
  - Regression? → Git Bisect first
  - Cross-module? → dependency tracing
```

### Git Bisect (if regression)

```yaml
PROTOCOL:
  1. git bisect start
  2. git bisect bad HEAD
  3. git bisect good <last_working>
  4. git bisect run ./test_script.sh
  5. Analyze first bad commit → focus investigation there
  6. git bisect reset

SKIP_IF: No good version, no automated test, env-dependent bug
```

### Dependency Tracing (if cross-module)

```yaml
PROTOCOL:
  1. Use available code navigation/search tools to inspect the failing symbol or data path
  2. Trace definitions and references across module boundaries
  3. Build graph: where value correct → where wrong
  4. Transition point = ROOT CAUSE

INTEGRATE_WITH_5_WHYS: Answer each "Why?" with definition/reference evidence
```

### LLM Block Analysis (if function >50 LOC)

```yaml
PROTOCOL:
  1. Decompose into basic blocks (control flow boundaries)
  2. Track variables per block
  3. Verify each block: "Is this correct given expected behavior?"
  4. Identify first faulty block
  
OUTPUT: Block #, lines, issue, variable state at failure
```

### 5 Whys (Always)

```yaml
TEMPLATE:
  1. Why {symptom}? → Because {cause_1} [file:line]
  2. Why {cause_1}? → Because {cause_2} [file:line]
  3. Why {cause_2}? → Because {ROOT_CAUSE} [file:line]

ENHANCED: Use definition/reference tracing evidence to VERIFY each answer
```

### Root Cause Receipt (MANDATORY)

```yaml
ROOT_CAUSE_RECEIPT:
  fields:
    - symptom
    - transition_point
    - confirmed_root_cause
    - evidence
    - falsification
    - remaining_unknowns
  required_before: Phase 4
```

### Step Localization (Thought-ICS)

```yaml
DEBUG_STEPS:
  step_1: Read error message completely
  step_2: Analyze stack trace to origin
  step_3: Reproduce bug twice
  step_4: Review recent changes (git log -5)
  step_5: Form hypothesis

ON_HYPOTHESIS_FAIL:
  prompt: |
    Review your debugging steps (step_1 to step_5).
    Which step led to wrong conclusion?
    Return: "ERROR in step_X: {reason}"
    
  action:
    rollback: Go back to step_X-1
    regenerate: Try alternative approach for step_X
```

---

## Phase 3: HYPOTHESIZE

### 3 Mandatory Gates

```yaml
GATE_1_INVESTIGATION:
  - Error message read completely
  - Stack trace analyzed
  - Bug reproduced twice
  - Recent changes reviewed
  - Regression check done
  BLOCKING: Cannot form hypothesis until ALL pass

GATE_2_HYPOTHESIS_QUALITY:
  tmk_structure:
    task: "What is the root cause of {error_type} in {layer}?"
    method:
      step1: "Isolate: which component is under test?"
      step2: "Predict: if hypothesis true, expect {behavior}"
      step3: "Design: change {variable}, observe {outcome}"
    knowledge: "Error patterns from {error_hierarchy}, codebase search patterns"
  FORMAT: |
    HYPOTHESIS: "{specific statement}"
    TMK_TASK: "{what root cause am I testing?}"
    TMK_METHOD: "{step-by-step test design}"
    TMK_KNOWLEDGE: "{relevant error patterns}"
    CONFIDENCE: {N}% (must be ≥70%)
    EVIDENCE: {supporting data}
    FALSIFICATION: {how to prove wrong}
  BLOCKING: Cannot test if <70% or not falsifiable

GATE_3_TEST_DESIGN:
  FORMAT: |
    TEST: {single variable change}
    EXPECTED: {outcome before running}
    IF_WRONG: {next action}
  BLOCKING: Cannot run if multiple variables
```

### Self-Correction Protocol

```yaml
TRIGGER: Hypothesis test failed

1_ACKNOWLEDGE: "Hypothesis '{X}' was WRONG because {evidence}"
   FORBIDDEN: "Let me try another thing"
2_GATHER: What NEW info? What assumption wrong?
3_UPDATE_MODEL: Old → New evidence → New understanding
4_REFORMULATE: New hypothesis MUST differ from failed one
```

### Escalation

```yaml
3_FAILURES: → STOP, question architecture
CONFIDENCE_STUCK_<70%: → Escalate to user
DIFFERENT_MODULE_EACH_TIME: → Use dependency tracing
```

---

## Phase 4: RESOLVE

```yaml
PRE_CHECK:
  - Phase 2 complete
  - Phase 3 gates passed
  - Root cause receipt recorded
  - Confidence ≥70%
  - Fix addresses ROOT (not symptom)

ALGORITHM_BY_SEVERITY:
  LOW: STC
  MEDIUM: PR-CoT
  HIGH: MAR
  CRITICAL: META
  MULTI_ERROR: 5-SOL

FAILURE_ESCALATION:
  trigger: 3+ fixes failed
  action: STOP → Question architecture → Discuss with user
  indicators: Coupling issues, wrong abstraction, missing component
```

---

## Anti-Patterns

```toon
forbidden[8]{action,correct}:
  Fix without trace,Complete Phase 2-3 first
  Skip error message,READ completely
  Multiple fixes at once,One change at a time
  "Just try X",Form hypothesis first
  3+ fixes without pause,STOP and question architecture
  Skip Git Bisect (regression),Binary search first
  Skip dependency tracing (cross-module),Use available definition/reference tracing
  Analyze >50 LOC manually,Use LLM Block Analysis
```

---

## Red Flags → STOP and Return to Phase 2

- "Just try changing X and see"
- "Add multiple changes, run tests"
- "I don't fully understand but might work"
- Each fix reveals new problem elsewhere
- 3+ failed fix attempts

---

## Section: web-search

# Web Validation Integration

> **⛔ MANDATORY.** Canonical web/internet validation rules live in `vida/config/instructions/runtime-instructions.web-validation-protocol.md`.

Use this section only as router-level integration map.

## Algorithm Integration

```yaml
STC:
  - On unknown build/test/lint/runtime error, run WVP before applying fix.

PR_COT:
  - Validate external assumptions (data/API/platform/security) via WVP before final decision.

MAR:
  - During rounds, use WVP when claims depend on external docs, versions, or platform behavior.

SOL_5:
  - Before Round 1 options, run WVP for best-practice/compatibility evidence.

META:
  - Before synthesis, ensure WVP evidence exists for all external claims.
```

## Log Requirement

When WVP trigger fires, record concise WVP evidence in TaskFlow logs/report.


## Section: reasoning-modules

# Reasoning Modules Library

> **Source:** Self-Discover Framework (arXiv:2402.03620, Google DeepMind)  
> **Purpose:** 20 curated atomic reasoning modules for VIDA thinking algorithms  
> **Usage:** SELECT relevant modules during META Step 0 or 5-SOL category generation; domain packets below are the default composer presets  
> **TMK:** when using a module, apply its T/M/K scaffold for structured reasoning.

---

## Module Categories

toon[5]{category,count,purpose}:
  Analysis,5,Understand problem structure
  Decomposition,4,Break into manageable parts
  Generation,5,Create diverse solutions
  Validation,4,Verify and critique
  Meta-cognition,2,Self-reflection and measurement

---

## Analysis Modules (5)

```yaml
ANALYSIS:

  critical_thinking:
    id: A1
    prompt: "Analyze from different perspectives, question assumptions, evaluate evidence"
    tmk:
      task: "Analyze {decision} for logical flaws and unexamined assumptions"
      method:
        - List all stakeholder perspectives
        - For each: question core assumptions
        - Evaluate evidence quality per perspective
      knowledge: "Common cognitive biases, logical fallacies, domain constraints"
    use_when:
      - Complex decisions with trade-offs
      - Multiple stakeholders involved
      - Uncertain requirements
    vida_mapping: PR-CoT Perspective 1 (Logical Integrity)
    
  systems_thinking:
    id: A2
    prompt: "Consider as part of larger system, identify interdependencies and feedback loops"
    tmk:
      task: "Map {component} within the larger system architecture"
      method:
        - Identify all dependencies (upstream and downstream)
        - Trace data flows across module boundaries
        - Check for feedback loops and side effects
      knowledge: "System architecture, module boundaries, data flow patterns"
    use_when:
      - Cross-module changes
      - Architecture decisions
      - State management
    vida_mapping: PR-CoT Perspective 3 (Architectural Alignment)
    
  risk_analysis:
    id: A3
    prompt: "Evaluate potential risks, uncertainties, and tradeoffs of different approaches"
    tmk:
      task: "Assess risks of {approach} in production context"
      method:
        - List failure scenarios (probability × impact)
        - Identify irreversible consequences
        - Design mitigation for top 3 risks
      knowledge: "Production failure patterns, rollback procedures, fallback strategies"
    use_when:
      - Production changes
      - Security implementations
      - Database migrations
    vida_mapping: MAR Critic, META confidence calculation
    
  core_issue:
    id: A4
    prompt: "What is the core issue or problem that needs to be addressed?"
    tmk:
      task: "Isolate the core problem beneath {symptoms}"
      method:
        - Separate symptoms from root cause
        - Trace symptom chain to origin
        - Validate: does fixing root cause resolve ALL symptoms?
      knowledge: "Problem decomposition patterns, symptom-cause relationships"
    use_when:
      - Bug investigation
      - Requirements clarification
      - Scope definition
    vida_mapping: STC Step 1
    
  root_cause:
    id: A5
    prompt: "What are the underlying causes or factors contributing to this problem?"
    tmk:
      task: "Find all contributing factors to {problem}"
      method:
        - Apply 5 Whys from each symptom
        - Cross-reference with recent changes (git log)
        - Verify each cause with evidence (stack trace, log, test)
      knowledge: "Error patterns, codebase-specific search patterns, error hierarchy"
    use_when:
      - Debugging complex issues
      - Refactoring decisions
      - Technical debt analysis
    vida_mapping: `vida/config/instructions/instruction-contracts.thinking-protocol.md#section-bug-reasoning`, MAR Round 1
```

---

## Decomposition Modules (4)

```yaml
DECOMPOSITION:

  break_down:
    id: D1
    prompt: "How can I break down this problem into smaller, more manageable parts?"
    use_when:
      - Large features
      - Epic planning
      - Complex implementations
    vida_mapping: /vida-form-task
    
  step_by_step:
    id: D2
    prompt: "Let's think step by step, making a plan and implementing with clear explanation"
    use_when:
      - Always applicable
      - Default reasoning approach
    vida_mapping: STC core flow
    
  simplify:
    id: D3
    prompt: "How can I simplify this problem so that it is easier to solve?"
    use_when:
      - Over-engineered solutions
      - Complex logic
      - Unclear requirements
    vida_mapping: HADI Heuristics (optimization_problem)
    
  measure_progress:
    id: D4
    prompt: "How can I measure progress on this problem? What indicators MUST I track?"
    use_when:
      - Milestone tracking
      - Test coverage goals
      - Performance optimization
    vida_mapping: /vida-implement verification
```

---

## Generation Modules (5)

```yaml
GENERATION:

  creative_thinking:
    id: G1
    prompt: "Generate innovative, out-of-the-box ideas. Think beyond traditional boundaries"
    use_when:
      - Feature ideation
      - UX exploration
      - explicit need for higher option diversity
    vida_mapping: 5-SOL Round 1, HADI Abduction
    
  alternative_perspectives:
    id: G2
    prompt: "What are the alternative perspectives or viewpoints on this problem?"
    use_when:
      - Architecture decisions
      - API design
      - Trade-off analysis
    vida_mapping: PR-CoT Perspective 4 (Alternatives)
    
  new_solution:
    id: G3
    prompt: "Ignoring the current solution, create an entirely new approach to the problem"
    use_when:
      - Stuck situations
      - Technical debt escape
      - Major refactoring
    vida_mapping: 5-SOL Round 2 diversity requirement
    
  experiment:
    id: G4
    prompt: "How could I devise an experiment or prototype to help solve this problem?"
    use_when:
      - Uncertain requirements
      - Performance questions
      - New technology evaluation
    vida_mapping: /vida-research
    
  analogy:
    id: G5
    prompt: "How does [other industry/domain] solve similar problems?"
    use_when:
      - Novel problems
      - Cross-domain inspiration
      - Innovation required
    vida_mapping: HADI analogy_transfer
```

---

## Validation Modules (4)

```yaml
VALIDATION:

  assumptions:
    id: V1
    prompt: "What are the key assumptions underlying this problem and proposed solution?"
    use_when:
      - Before implementation
      - Specification review
      - Risk assessment
    vida_mapping: PR-CoT Perspective 2 (Data Completeness)
    
  risks_drawbacks:
    id: V2
    prompt: "What are the potential risks and drawbacks of each solution?"
    use_when:
      - Before approval
      - Architecture review
      - Security assessment
    vida_mapping: MAR Critic, 5-SOL scoring
    
  lessons_learned:
    id: V3
    prompt: "What similar approaches were tried before? What were outcomes and lessons?"
    use_when:
      - Retrospectives
      - Pattern recognition
      - Avoiding repeated mistakes
    vida_mapping: Knowledge base consultation
    
  obstacles:
    id: V4
    prompt: "What potential obstacles or challenges might arise in solving this?"
    use_when:
      - Planning phase
      - Risk mitigation
      - Dependency analysis
    vida_mapping: /vida-form-task, MAR Round 2-3
```

---

## Meta-cognition Modules (2)

```yaml
META_COGNITION:

  reflective_thinking:
    id: M1
    prompt: "Step back, examine personal biases and mental models that may influence this"
    use_when:
      - Stuck on problem
      - Repeated failures
      - Conflicting solutions
    vida_mapping: MAR Reflector
    
  success_metrics:
    id: M2
    prompt: "How can success be measured or evaluated? What does 'done' look like?"
    use_when:
      - Feature completion
      - Acceptance criteria
      - Test planning
    vida_mapping: /vida-implement, walkthrough.md
```

---

## Selection Guide

### Composer Domain Packets

```yaml
DOMAIN_PACKETS:
  security_auth: [A3, V1, V2, V4]
  database_schema: [A2, A3, V1, V2, V4]
  foundation_architecture: [A1, A2, A3, G2, V2]
  tech_stack_selection: [A1, A2, A3, G2, V2]
  bug_root_cause: [A4, A5, D2, V4]
  multi_option_design: [A1, A2, G2, V2, M2]

RULE:
  - Use these as Step 0 defaults for META and as category seeds for 5-SOL.
  - Add or remove modules only with task-specific rationale.
```

### By VIDA Algorithm

toon[6]{algorithm,recommended_modules}:
  STC,D2 (step_by_step) + A4 (core_issue)
  PR-CoT,A1 (critical) + A2 (systems) + G2 (alternatives) + V1 (assumptions)
  MAR,A5 (root_cause) + V2 (risks) + V4 (obstacles) + M1 (reflective)
  5-SOL,G1 (creative) + G2 (alternatives) + G3 (new_solution) + D1 (break_down)
  META,SELECT 3-5 based on problem domain

### By Problem Type

toon[7]{problem_type,recommended_modules}:
  bug_fix,A4 + A5 + D2 + V4
  new_feature,D1 + G1 + G2 + V1 + M2
  refactoring,A2 + D3 + V2 + V3
  architecture,A1 + A2 + A3 + G2 + V2
  database_schema,A2 + A3 + V1 + V2 + V4
  tech_stack,A1 + A2 + A3 + G2 + V2
  security_auth,A3 + V1 + V2 + V4

---

## SELECT → ADAPT → IMPLEMENT Example

```yaml
EXAMPLE:
  problem: "Design offline-first sync architecture for mobile app"
  
  SELECT:
    chosen: [A2, A3, G2, V2, M2]
    rationale: |
      - A2 (systems_thinking): Cross-module sync involves many components
      - A3 (risk_analysis): Offline sync has many failure modes
      - G2 (alternative_perspectives): Multiple valid sync strategies exist
      - V2 (risks_drawbacks): Must evaluate each approach's downsides
      - M2 (success_metrics): Need clear definition of "sync complete"
      
  ADAPT:
    A2_adapted: "Map all data flows between local DB, sync queue, and server"
    A3_adapted: "Identify failure scenarios: network drop, conflict, partial sync"
    G2_adapted: "Compare: optimistic vs pessimistic locking, CRDT vs last-write-wins"
    V2_adapted: "For each sync strategy, list data loss and UX impact risks"
    M2_adapted: "Define: sync latency target, conflict resolution accuracy, offline capability"
    
  IMPLEMENT:
    reasoning_plan:
      - step: 1
        module: A2
        action: "Draw data flow diagram with conflict points marked"
      - step: 2
        module: G2
        action: "Generate 3 sync architectures with different tradeoffs"
      - step: 3
        module: A3
        action: "Create failure mode table for each architecture"
      - step: 4
        module: V2
        action: "Score architectures on risk dimensions (1-5)"
      - step: 5
        module: M2
        action: "Define acceptance criteria and test scenarios"
```

---

## TMK Usage Guide

```yaml
WHEN_TO_USE_TMK:
  always: "When selecting a module for META Step 0 or 5-SOL"
  how: |
    1. SELECT module by id
    2. READ tmk.task → substitute {variables}
    3. EXECUTE tmk.method steps in order
    4. REFERENCE tmk.knowledge for domain context
  benefit: "Switches LLM from linguistic to symbolic reasoning mode"
  source: "TMK (arXiv:2602.03900) — +65.8% on planning tasks"
```

---

*20 curated from 39 original (arXiv:2402.03620) + TMK structured scaffolds*

-----
artifact_path: config/instructions/instruction-contracts/thinking.protocol
artifact_type: instruction_contract
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/instruction-contracts.thinking-protocol.md
created_at: 2026-03-06T22:42:30+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: instruction-contracts.thinking-protocol.changelog.jsonl
