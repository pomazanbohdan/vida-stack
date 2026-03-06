# Thinking Protocol — Unified Thinking Orchestrator

> **MANDATORY.** The single instruction set for routing and executing all VIDA thinking algorithms.
> Processing happens internally; the user receives only the final conclusion by default.

---

## Purpose

Composed orchestrator for all algorithms embedded in this file (`_vida/docs/thinking-protocol.md`):

- `#section-stc`
- `#section-pr-cot`
- `#section-mar`
- `#section-5-solutions`
- `#section-meta-analysis`
- `#section-algorithm-selector`
- `#section-bug-reasoning`
- `#section-web-search`
- `_vida/docs/thinking-protocol.md#section-reasoning-modules`

---

## READ_MANDATORY (⛔ BLOCKING)

```yaml
FILES_REQUIRED_ALWAYS:
  - _vida/docs/thinking-protocol.md#section-algorithm-selector
  - _vida/docs/thinking-protocol.md#section-stc
  - _vida/docs/thinking-protocol.md#section-pr-cot
  - _vida/docs/thinking-protocol.md#section-mar
  - _vida/docs/thinking-protocol.md#section-5-solutions
  - _vida/docs/thinking-protocol.md#section-meta-analysis
  - _vida/docs/thinking-protocol.md#section-bug-reasoning
  - _vida/docs/thinking-protocol.md#section-web-search
  - _vida/docs/thinking-protocol.md#section-reasoning-modules
  - _vida/docs/web-validation-protocol.md

BLOCKING_RULES:
  - Do not proceed if any required file is unread.
  - META is valid only if PR-CoT + MAR + 5-SOL are executed with evidence.
  - Confidence values must come from actual execution artifacts.
```

---

## Core Contract (L0)

```yaml
YOU_MUST:
  - Check overrides first.
  - Compute complexity score for non-override paths.
  - Route to the correct algorithm.
  - Execute selected algorithm fully.
  - Trigger web search in mandatory scenarios.
  - Use root-cause-first policy for bugs.
  - Pass PRE_OUTPUT_GATE before responding.

YOU_MUST_NOT:
  - Skip required phases/rounds/passes.
  - Fabricate consensus, scores, or confidence.
  - Apply fixes before root-cause investigation.
  - Expose intermediate chain-of-thought by default.
```

---

## Router

### 1) Overrides First

```yaml
OVERRIDES:
  - Security/Auth decision -> META
  - Database schema/foundation architecture -> META
  - DEC-XXX creation -> MAR
  - Multiple errors/issues -> 5-SOL or Error Search (batch)
  - "Choose between X/Y/Z" -> 5-SOL
```

### 2) Complexity Score

```yaml
FORMULA: C×3 + R×3 + S×2 + N×2 + F×1
RANGE: 11-55

SELECTION:
  <=15: STC
  16-25: PR-CoT
  26-35: MAR
  36-45: 5-SOL
  >45: META
```

### 3) Bug-Specific Routing (Error Search)

```yaml
BUG_PIPELINE:
  detect: classify error layer/type/regression
  classify: severity + blast radius + data risk + frequency
  trace: pre-check + 5 whys (+ git bisect/lsp/llm-block when triggered)
  hypothesize: 3 mandatory gates with falsifiable hypothesis
  resolve: apply severity->algorithm map

SEVERITY_TO_ALGO:
  low: STC
  medium: PR-CoT
  high: MAR
  critical: 5-SOL/META
```

---

## Mandatory Web Search Integration

```yaml
TRIGGERS:
  - Use _vida/docs/web-validation-protocol.md as canonical source.
  - Run WVP whenever external assumptions can change decisions.
  - For server/API assumptions, LIVE validation is mandatory.
  - Record concise WVP evidence in logs/reports.
```

---

## Algorithm Execution Specs

### STC (Score <=15)

```yaml
FLOW:
  - Generate step
  - Validate internally
  - On failure: localize first broken step
  - Roll back to clean prefix
  - Retry with TRT knowledge list (avoid repeated failed approach)

LIMITS:
  max_retries: 3
  on_fail: escalate to PR-CoT
```

### PR-CoT (Score 16-25)

```yaml
FLOW:
  - Pass 1: 4 independent perspectives (logical, data, arch, alternatives)
  - Build consensus packet (agreements/divergences/key signal)
  - Pass 2: each perspective revises against consensus
  - Final issue count -> proceed/revise/escalate

ESCALATION:
  if_final_issues >= 2: escalate to MAR
```

### MAR (Score 26-35)

```yaml
FLOW:
  - 3 rounds with 4 roles: Actor, Evaluator, Critic, Reflector
  - Evaluator uses adaptive rubrics (RRD)
  - Reflector carries knowledge list across rounds (TRT)
  - Actor must avoid known-failed approaches

EXIT:
  score >= 8/10: accept
  score < 8 after round 3: escalate to META
```

### 5-SOL (Score 36-45)

```yaml
FLOW:
  - Define 4-6 dynamic categories
  - Round 1: generate 5 viable options + score + HYBRID R1
  - Category check (decompose coarse / downweight correlated)
  - Build PACER consensus packet R1->R2
  - Round 2: 5 NEW options conditioned on packet + HYBRID R2
  - Final hybrid: compare R1 vs R2, resolve conflicts

RULES:
  - Never <5 options in R1
  - R2 must not repeat failed R1 patterns
  - At least 2 R2 options explore new angles
```

### META (Score >45 or override)

```yaml
STEP_0_SELF_DISCOVER:
  - Select 3-5 modules from `_vida/docs/thinking-protocol.md#section-reasoning-modules`
  - Adapt prompts to task
  - Pass plan into PR-CoT, MAR, 5-SOL

PASS_1_PARALLEL:
  - Run PR-CoT + MAR + 5-SOL
  - Compare agreements/divergences
  - Compute weighted confidence:
    pr_cot_weight = 1/(1+critical_issues)
    mar_weight = final_score/10
    sol_weight = inter_round_agreement

TRT_LOOP:
  trigger: confidence < 80%
  max_loops: 2
  action: rerun divergent algorithm(s) with shared knowledge packet

EXIT:
  confidence >= 80% -> synthesize
  still <80% after 2 loops -> cautious synthesis or user decision
```

### Error Search v3.1 (Bug Investigation)

```yaml
MANDATORY_GATES_BEFORE_FIX:
  - Full error message read
  - Stack trace traced to origin
  - Bug reproduced twice
  - Recent changes reviewed

TECHNIQUES:
  - Regression -> git bisect
  - Cross-module (3+ files) -> lsp dependency tracing
  - Function >50 LOC -> block analysis

HYPOTHESIS_QUALITY:
  - TMK structure required
  - Confidence >=70%
  - Falsification path required
  - Single-variable test design only
```

---

## Self-Discover Module Selection (from reasoning-modules)

```yaml
DEFAULT_MAPPING:
  STC: [D2, A4]
  PR_COT: [A1, A2, G2, V1]
  MAR: [A5, V2, V4, M1]
  SOL: [G1, G2, G3, D1]
  META: select 3-5 by problem domain

SECURITY_SUGGESTED:
  [A3, V1, V2, V4, M2]

ARCHITECTURE_SUGGESTED:
  [A1, A2, A3, G2, V2]
```

---

## Output Policy

```yaml
DEFAULT:
  user_sees: final conclusion only

TRACE_MODE:
  trigger: user explicitly asks for reasoning/trace
  include:
    - selected algorithm
    - score/override
    - concise execution evidence
    - final decision and risks
```

---

## PRE_OUTPUT_GATE (⛔ MANDATORY)

```yaml
CHECKLIST:
  - [ ] Required files were read
  - [ ] Override check done
  - [ ] Score computed (if no override)
  - [ ] Correct algorithm selected
  - [ ] Mandatory web search performed when triggered
  - [ ] Bug fixes followed root-cause pipeline (if bug task)
  - [ ] Required rounds/passes completed
  - [ ] Confidence is evidence-based (no fabrication)
  - [ ] Output format matches mode (silent/trace)

FAIL_ACTION:
  - Stop
  - Complete missing steps
  - Re-run gate
```

---

## Anti-Patterns

```yaml
FORBIDDEN:
  - "Hotfix" without root cause
  - Skipping Pass 2 in PR-CoT
  - Finishing MAR before Round 3 without passing threshold
  - Reusing 5-SOL R1 options in R2
  - META without running all 3 underlying algorithms
  - Confidence claim without calculable evidence
```

---

*VIDA Framework — Unified Thinking Protocol*
*Composed as a single canonical file with embedded sections in `_vida/docs/thinking-protocol.md`*
*Includes reasoning modules in `#section-reasoning-modules`*


## Embedded Algorithms (Canonical Inlined Sources)

<!-- Canonical inlined dependencies. Do not edit mirrored archived files. -->

## Section: algorithm-selector

# Algorithm Selector v2.1

> **MANDATORY.** Unified router. All algorithms run INTERNALLY — user sees conclusion only.

---

## First Session

```yaml
READ_FIRST: [_vida/docs/thinking-protocol.md#section-stc, _vida/docs/thinking-protocol.md#section-pr-cot, _vida/docs/thinking-protocol.md#section-mar, _vida/docs/thinking-protocol.md#section-5-solutions, _vida/docs/thinking-protocol.md#section-meta-analysis, _vida/docs/thinking-protocol.md#section-web-search]
ON_STUDY_REQUEST: Read silently → output "✅ Algorithm Selector studied. Ready."
```

---

## Enforcement (⛔ BLOCKING)

```yaml
READ_BEFORE_ROUTING:
  mandatory: [_vida/docs/thinking-protocol.md#section-stc, _vida/docs/thinking-protocol.md#section-pr-cot, _vida/docs/thinking-protocol.md#section-mar, _vida/docs/thinking-protocol.md#section-5-solutions, _vida/docs/thinking-protocol.md#section-meta-analysis, _vida/docs/thinking-protocol.md#section-web-search]
  first_session: Read ALL silently → ready
  per_request: Read SELECTED algorithm file(s)

META_SPECIAL_RULE: |
  ⛔ META REQUIRES reading AND executing ALL of:
  - _vida/docs/thinking-protocol.md#section-pr-cot (PR-CoT validation)
  - _vida/docs/thinking-protocol.md#section-mar (MAR refinement)
  - _vida/docs/thinking-protocol.md#section-5-solutions (5-SOL synthesis)
  - _vida/docs/thinking-protocol.md#section-reasoning-modules (Self-Discover Step 0)
  
  ⛔ If you select META but don't execute all 3 algorithms → INVALID.
  ⛔ Fabricated confidence without execution = PROTOCOL VIOLATION.
```

---

## Selection

```toon
selection[5]{score,algorithm,purpose}:
  ≤15,STC,Step-critique (silent)
  16-25,PR-CoT,4 perspectives validation
  26-35,MAR,3 rounds × 4 agents
  36-45,5-SOL,2 rounds × 5 options (alignment)
  >45,META,Ensemble (PR-CoT + MAR + 5-SOL)
```

---

## Overrides (Check First!)

```toon
overrides[5]{scenario,algorithm}:
  Security/Auth decision,META
  Database schema,META
  DEC-XXX creation,MAR
  Multiple errors/issues,5-SOL
  "Choose between X,Y,Z",5-SOL
```

---

## Score Formula

```yaml
FORMULA: C×3 + R×3 + S×2 + N×2 + F×1
RANGE: 11-55
```

```toon
factors[5]{factor,weight,1,3,5}:
  C (Complexity),×3,Simple,Multi-factor,Novel
  R (Reversibility),×3,Easy rollback,Partial,Irreversible
  S (Stakes),×2,Low,Module,System-wide
  N (Novelty),×2,Known,Partial,First time
  F (Frequency),×1,Daily,Weekly,One-time
```

---

## Scoring Examples

```toon
examples[8]{scenario,c,r,s,n,f,score,algo}:
  Syntax fix,1,1,1,1,1,11,STC
  Unit test,1,1,1,3,3,15,STC
  Feature refactor,3,3,3,1,3,23,PR-CoT
  Cross-module bug,3,3,3,3,3,27,MAR
  Multiple errors,5,3,4,3,3,37,5-SOL
  State mgmt choice,5,3,5,3,5,45,5-SOL
  Auth flow,5,5,5,3,5,51,META
  Tech stack,5,5,5,5,5,55,META
```

---

## HADI Creativity Weight

```yaml
CREATIVITY_WEIGHT:
  high (0.7-1.0): ["creative", "innovative", "brainstorm"] → 5-SOL uses Abduction
  balanced (0.4-0.6): default → Normal execution
  safe (0.0-0.3): ["reliable", "proven", "hotfix"] → Prefer heuristics
```

---

## Execution Protocol

```yaml
STEPS:
  1. Check overrides (security, auth, DEC-XXX, multiple_errors)
  2. Calculate score (C×3 + R×3 + S×2 + N×2 + F×1)
  3. Select algorithm by score range
  4. Execute INTERNALLY (no output)
     - If META: Run Self-Discover Step 0 first
  5. Output conclusion only
```

---

## Web Search

```yaml
MANDATORY:
  - Error from build/test/lint → search FIRST
  - New package → verify pub.dev
  - iOS/Android config → search docs
  - Security → search best practices
```

---

## VIDA Command Defaults

```toon
vida_commands[12]{command,decision_point,score,algo}:
  orchestration-protocol,Routing,11,STC
  orchestration-protocol,Phase transition,27,MAR
  /vida-research,Extraction,19,PR-CoT
  /vida-form-task,Scope draft,25,PR-CoT
  /vida-form-task,Scope final,33,MAR
  /vida-spec,Architecture,42,5-SOL
  /vida-spec,Security,48,META
  /vida-form-task,Task pool breakdown,21,PR-CoT
  /vida-implement,Execution,14,STC
  reflection-pack (change impact),Large (≥5),29,MAR
  /vida-bug-fix,Impact Analysis,30,MAR
  /vida-bug-fix,HIGH Blast Radius,42,5-SOL
```

---

## Activity Defaults

```toon
activities[12]{activity,scenario,score,algo}:
  Code,<50 LOC,≤15,STC
  Code,50-200 LOC,16-25,PR-CoT
  Code,Multi-file,26-35,MAR
  Code,API design,36-45,5-SOL
  Bug,Syntax/typo,11,STC
  Bug,Cross-module,22,PR-CoT
  Bug,Multiple errors,40,5-SOL
  Bug,Security,>45,META
  Test,Unit test,13,STC
  Test,Integration,20,PR-CoT
  Arch,State management,42,5-SOL
  Arch,Database/Auth,>45,META
```

---

## Algorithm Comparison

```toon
comparison[6]{aspect,STC,PR-CoT,MAR,5-SOL,META}:
  Score,≤15,16-25,26-35,36-45,>45
  Internal,Step-check,4 persp.,3 rounds,2×5 opts,All 3
  Purpose,Check,Validate,Refine,Align,Ensemble
  Accuracy,+10%,82%,95%,96%,98%
  Speed,Fast,Fast,Medium,Medium,Slow
  Output,Conclusion,Decision,Best ver.,Hybrid,Synth.
```

---

*VIDA Framework — Algorithm Selector v2.1*



## Section: stc

# STC: Stepwise Think-Critique v2.2

> **Internal Algorithm.** Self-check each reasoning step. User sees conclusion only.
> **v2.2:** TRT knowledge list — accumulated failure context between retries.

```yaml
PREREQ: _vida/docs/thinking-protocol.md#section-algorithm-selector  # Understand when STC applies (Score ≤15)
```


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER skip the internal step check.
⛔ NEVER repeat a failed approach; you MUST update and read the TRT knowledge list.
⛔ NEVER output intermediate steps to the user unless explicitly requested.
✅ MUST maintain a clean prefix (keep steps 1 to X-1) during rollback.
✅ MUST escalate to PR-CoT if max retries (3) are reached without a solution.
</constraints>

## Triggers

```toon
triggers[3]{condition,mode}:
  Score ≤ 15,INTERNAL (silent)
  reasoning task,INTERNAL
  logic task,INTERNAL
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

  on_max: Escalate to PR-CoT OR ask user
```

**Flow:** STEP 1 → check → STEP 2 → ... → ERROR? → LOCALIZE → knowledge += failure → ROLLBACK → retry (informed)

---

## Output Rules

```toon
output[4]{state,action}:
  during,NO OUTPUT
  complete,Conclusion only
  error,Simplify + clarify
  on_request,"Full trace if user asks 'show reasoning'"
```

---

## Use Cases

```toon
usecases[6]{scenario,score}:
  Fix syntax error,5-8
  Simple function,10-12
  Refactor (rename),8-10
  Unit test write,11-13
  Local bug fix,12-15
  Calculation,10-14
```

---

## Escalation

```yaml
ESCALATE_IF:
  max_retries: 3
  on_fail: PR-CoT (if score allows) OR ask user
  pass_context: knowledge_list (so PR-CoT sees what STC already tried)
```

---

*VIDA Framework — STC v2.2 (TRT Knowledge List)*


## Section: pr-cot

# PR-CoT: Poly-Reflective Chain-of-Thought v2.2

> **4-perspective validation + consensus revision.** Score 16-25. Escalate to MAR if issues ≥ 2.
> **v2.2:** PACER consensus packet + TMK structured perspectives.

```yaml
PREREQ: _vida/docs/thinking-protocol.md#section-algorithm-selector  # Understand when PR-CoT applies (Score 16-25)
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
triggers[7]{trigger,type}:
  phase_transition,MANDATORY
  scope_formation,MANDATORY
  specification,MANDATORY
  execution_strategy,MANDATORY
  impact_analysis,MANDATORY
  complexity >= 3,CONDITION
  multiple_valid_options,CONDITION
```

**SKIP:** Simple routing, template ops, read-only.

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
  
  step: 6
  action: "0: Proceed | 1: Revise | ≥2: ESCALATE to MAR"
```

---

## Output Format

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

**Final Issues:** {count} → **Action:** {proceed|revise|escalate}
```

---

## Escalation

```yaml
ESCALATE_TO_MAR:
  trigger: issues >= 2 (after Pass 2 revision)
  pass: original_decision, issues_found, perspectives, consensus_packet
```

---

*VIDA Framework — PR-CoT v2.2 (PACER consensus + TMK perspectives)*


## Section: mar

# MAR: Multi-Agent Reflexion v2.2

> **3 rounds × 4 agents.** Score 26-35. Escalate to META if final score < 8.
> **v2.2:** TRT knowledge carry-over + RRD adaptive rubrics.

```yaml
PREREQ: [_vida/docs/thinking-protocol.md#section-algorithm-selector, _vida/docs/thinking-protocol.md#section-pr-cot]  # MAR builds on PR-CoT
```


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER fabricate agent responses; you must simulate 4 distinct expert agents.
⛔ NEVER end the debate before Round 3 unless the Minimum Viable Score (8/10) is achieved.
✅ MUST aggregate scores strictly based on actual agent votes.
✅ MUST explicitly pass context to the user if consensus is not reached after Round 3.
</constraints>

## Triggers

```toon
triggers[5]{trigger}:
  Score 26-35
  DEC-XXX creation
  Complex trade-offs
  Novel solutions
  PR-CoT escalation (issues ≥ 2)
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
  overall: 1-10 (weighted average of rubrics)
  rubric_count: 4 base, up to 12 decomposed
  
  escalation:
    score < 6: "Critical gaps remain"
    score 6-7: "Acceptable with known limitations"
    score >= 8: "Strong solution, minimal gaps"
    
  round_progression:
    expected: "R1: 5-6 → R2: 7-8 → R3: 8-9"
    stagnation: "If R(N) score == R(N-1) score → Reflector must change strategy"
```

---

## Escalation

```yaml
ESCALATE_TO_META:
  trigger: final_score < 8 after 3 rounds
  pass: all_rounds, knowledge_list, best_solution, rubric_scores
```

---

*VIDA Framework — MAR v2.2 (TRT Knowledge + RRD Adaptive Rubrics)*


## Section: 5-solutions

# 5-SOL: 5-Solutions Algorithm v2.2

> **Alignment algorithm.** Score 36-45. Generates 5 options × 2 rounds → synthesizes optimal hybrid.
> **v2.2:** PACER consensus R1→R2 + RRD adaptive category decomposition.

---

## Purpose

- Multiple errors → finds common root cause
- Many valid options → synthesizes optimal hybrid
- Architecture alignment → balanced solution


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
  score,36-45
  keywords,"choose between | which approach | multiple errors | architecture alignment"
  in_meta,Score >45 (parallel with PR-CoT, MAR)
  skip,Score ≤35 OR simple decisions
```

---

## Algorithm Flow

```yaml
FLOW:
  step_0: Generate 4-6 dynamic categories
  step_1: Web research (if error/package/API)
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
  
  FINAL: Compare R1 vs R2 → FINAL HYBRID
  OUTPUT: Confidence % + decision
```

---

## Dynamic Categories

```toon
domains[5]{problem,categories}:
  auth,"Security | Usability | Performance | Maintainability | Compliance"
  ui,"UX | Accessibility | Performance | Consistency | Effort"
  data,"Integrity | Scalability | Query Perf | Migration | Cost"
  state_mgmt,"Learning Curve | Boilerplate | Testability | Scalability | Type Safety"
  error_align,"Root Cause | Side Effects | Regression Risk | Scope | Testing"
```

---

## Self-Discover SELECT (Optional)

```yaml
TRIGGER: creativity_weight >= 0.7 OR multi-domain problem
ACTION: Select 2-3 modules from `_vida/docs/thinking-protocol.md#section-reasoning-modules` → inform categories
MAPPING:
  A2 (systems) → "Integration" category
  A3 (risk) → "Failure Modes" category
  G2 (alternatives) → "Approach Diversity" category
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
  R1 = R2,90-95%
  R1 ~ R2 (consensus improved convergence),85-89%
  R1 ≠ R2 (but consensus addressed gaps),80-84%
  R1 ≠ R2 (unresolved gaps remain),70-79% → web research
```

---

## Output Format

```markdown
## 5-SOL: {Problem}

**Categories:** {C1, C2, C3, C4, C5}

**R1:** {5 options table} → **HYBRID R1:** {synthesis}

**Category Check:** {decomposed: C2→C2a,C2b | merged: C3+C5 | unchanged: C1,C4}

**Consensus Packet:** {top options, gaps, winning elements}

**R2 (informed):** {5 options table} → **HYBRID R2:** {synthesis}

**FINAL HYBRID:** {description}
**Confidence:** {XX}%
**Files:** {affected} | **Order:** {sequence}
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

---

*VIDA Framework — 5-SOL v2.2 (PACER Consensus + RRD Categories + HADI + Self-Discover)*


## Section: meta-analysis

# META: Meta-Analysis v2.2

> **Ensemble.** Score >45. Runs PR-CoT + MAR + 5-SOL in parallel → compares → synthesizes.
> **v2.2:** TRT recursive loop + PACER weighted voting.

```yaml
PREREQ: [_vida/docs/thinking-protocol.md#section-algorithm-selector, _vida/docs/thinking-protocol.md#section-pr-cot, _vida/docs/thinking-protocol.md#section-mar, _vida/docs/thinking-protocol.md#section-5-solutions]  # META combines all
```


---

## Constraints (L1 - Algorithm Logic)

<constraints>
⛔ NEVER skip the execution of PR-CoT, MAR, or 5-SOL when required by complexity score.
⛔ NEVER fabricate the "Confidence" score; it must be calculated from ACTUAL consensus among algorithms.
✅ MUST perform Step 0 (Self-Discover) to load required architectural context.
✅ MUST present a final conclusion based solely on the synthesis of the 3 underlying algorithms.
</constraints>

## Triggers

```toon
triggers[6]{type,condition}:
  score,>45
  override,Security/Auth decisions
  override,Database schema
  override,Foundation architecture
  override,Tech stack selection
  skip,Score ≤45 OR already inside META
```

---

## Step 0: Self-Discover

```yaml
SELF_DISCOVER:
  SELECT: Choose 3-5 modules from `_vida/docs/thinking-protocol.md#section-reasoning-modules`
  ADAPT: Rephrase prompts for specific task
  IMPLEMENT: Create reasoning plan → pass to parallel algorithms
  
  PASS_TO:
    pr_cot: Perspective ordering
    mar: Actor initialization
    5_sol: Category generation
```

---

## Parallel Execution

```toon
algorithms[3]{algo,role,purpose}:
  PR-CoT,Validator,"4 perspectives → find issues (2-pass consensus)"
  MAR,Refiner,"3 rounds × 4 agents → best version (knowledge carry-over)"
  5-SOL,Synthesizer,"2×5 options → optimal hybrid (consensus R1→R2)"
```

---

## Execution Protocol

```yaml
PASS_1:
  step: 1
  action: "Parallel: PR-CoT | MAR | 5-SOL"
  
  step: 2
  action: "Compare: agreements + divergences"
  
  step: 3
  action: "Web check (if weighted confidence <80% or critical divergence)"

WEIGHTED_CONFIDENCE:
  purpose: "Not all algorithms are equally reliable for every query"
  
  calculation:
    pr_cot_weight:
      formula: "1 / (1 + critical_issues_count)"
      rationale: "Fewer critical issues → higher validator trust"
    mar_weight:
      formula: "final_score / 10"
      rationale: "Higher evaluator score → higher refiner trust"
    5_sol_weight:
      formula: "inter_round_agreement"
      rationale: "Higher R1-R2 agreement → more stable synthesizer"
  
  consensus: "weighted_average(pr_cot_weight, mar_weight, 5_sol_weight)"
  
  step: 4
  action: "Evaluate weighted confidence"
  
  step: 5
  decision:
    if_confidence >= 80%: "SYNTHESIZE final decision → done"
    if_confidence < 80%: "TRT LOOP (see below)"

TRT_LOOP:
  purpose: "Recursive improvement instead of immediate user escalation"
  max_loops: 2
  
  on_each_loop:
    1. ANALYZE divergences between algorithms:
       - Which algorithm diverges most?
       - What specific aspects differ?
       - Build knowledge_list: {what's agreed, what's disputed, why}
       
    2. RE-RUN only DIVERGENT algorithm(s):
       - Inject knowledge_list as extra context
       - Knowledge includes findings from OTHER algorithms
       - Algorithm runs with awareness of consensus
       
    3. RE-COMPARE:
       - Calculate new weighted confidence
       - If >= 80%: SYNTHESIZE → done
       - If < 80% and loops < 2: next TRT loop
       - If < 80% and loops == 2: SYNTHESIZE with caution OR user decision
  
  knowledge_format: |
    TRT CONTEXT (Loop {N}):
      Agreed: {points all algorithms concur on}
      Disputed: {specific disagreement}
      PR-CoT says: {summary}
      MAR says: {summary}
      5-SOL says: {summary}
      YOUR TASK: Address {disputed points} specifically
```

---

## Confidence Matrix

```yaml
CONFIDENCE:
  all_agree_weighted_high:
    range: 95-100%
    action: Auto-proceed
    
  mostly_agree:
    range: 85-94%
    action: Proceed + note divergences
    
  partial_agree:
    range: 80-84%
    action: Proceed with caution + flag risks
    
  low_confidence_trt:
    range: 70-79%
    action: TRT loop (re-run divergent, max 2 loops)
    
  full_conflict:
    range: <70%
    action: TRT loop → if still <70% after 2 loops → user decision
```

---

## Output Format

```markdown
## META: {Decision}

### PR-CoT (Validation)
**Issues:** {list} | **Weight:** {X}

### MAR (Refinement)  
**Score:** {X}/10 | **Knowledge:** {key items} | **Weight:** {X}

### 5-SOL (Synthesis)
**Hybrid:** {description} | **R1-R2 Agreement:** {X}% | **Weight:** {X}

### Comparison
| Aspect | PR-CoT | MAR | 5-SOL | Agree |
|--------|--------|-----|-------|:-----:|
| ... | ... | ... | ... | ✓/△/✗ |

**Weighted Confidence:** {XX}%

### TRT Loop (if triggered)
**Loop {N}:** Re-ran {algorithm} with knowledge: {context}
**Result:** Confidence {before}% → {after}%

### FINAL DECISION
{synthesized decision}
**Files:** {list} | **Risks:** {from PR-CoT} | **Trade-offs:** {from 5-SOL}
```

---

## Skip Rules

```yaml
DEFAULT: Run all 3
SKIP_5SOL_IF: "Validate X" not "Choose/Design X"
SKIP_MAR_IF: Already have 5+ external options
NEVER_SKIP: PR-CoT (always validate)
```

---

## Anti-Patterns

```toon
forbidden[5]{action,why}:
  META inside 5-SOL,Circular/token explosion
  Skip PR-CoT,Misses validation
  Ignore minority opinion,May have critical insight
  Force at low confidence,TRT loop first then escalate
  Skip TRT when divergent,Loses self-correction opportunity
```

---

## Execution Proof (⛔ MANDATORY)

> Before output, META must contain proof that each algorithm was executed:

```yaml
PROOF_REQUIRED:
  step_0_self_discover:
    evidence: "Selected modules: {A1, A2, ...}"
    
  pr_cot_execution:
    evidence: "Issues found: {count} — {list}. Weight: {X}"
    
  mar_execution:
    evidence: "Final score: {X}/10 after {N} rounds. Knowledge: {items}. Weight: {X}"
    
  5_sol_execution:
    evidence: "HYBRID R1: {...}, HYBRID R2: {...}. R1-R2 agreement: {X}%. Weight: {X}"
    
  weighted_confidence:
    evidence: "Weighted: {X}% (PR-CoT: {w1}, MAR: {w2}, 5-SOL: {w3})"
    
  trt_loop:
    evidence: "Loop {N}: re-ran {algo}, confidence {before}% → {after}%"
    OR: "Not triggered (confidence >= 80%)"
    
  comparison:
    evidence: "Consensus: {X}/Y agree on: {points}"

NO_PROOF_NO_OUTPUT: |
  ⛔ If you cannot provide SPECIFIC evidence from each algorithm,
  you did NOT execute META correctly.
  
  ⛔ Statements like "94% (3/3 agree)" without execution details = FABRICATION.
  ⛔ Fabrication = PROTOCOL VIOLATION → restart META properly.
  
VALID_EXAMPLE: |
  ✓ "PR-CoT: 3 issues (Logical: implicit prereqs, Data: no enforcement, Arch: missing gate). Weight: 0.67"
  ✓ "MAR R3: 9/10, Knowledge: [AVOID LWW, KEEP field-merge]. Weight: 0.9"
  ✓ "5-SOL HYBRID: Riverpod Notifier (R1 Signals gap resolved). R1-R2: 88%. Weight: 0.88"
  ✓ "Weighted confidence: 87% → SYNTHESIZE"
```

---

*VIDA Framework — META v2.2 (TRT Recursive Loop + PACER Weighted Voting + Ensemble + Self-Discover)*


## Section: bug-reasoning

# Error Search v3.1

> **MANDATORY.** Detect → Classify → Trace → Hypothesize → Resolve errors.
> **Iron Law:** NO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST.
> **v3.1:** TMK-structured hypothesis formation in Phase 3.

---

## What's New in v3.1

| Feature | Trigger |
|---------|---------|
| Git Bisect Protocol | "Used to work" / regression |
| LLM Block Analysis | Function >50 LOC, unclear bug |
| Stricter Checkpoints | All bugs (3 mandatory gates) |
| LSP Dependency Tracing | Cross-module bugs (3+ files) |
| TMK Hypothesis (v3.1) | All bugs (Phase 3 Gate 2) |

---

## Pipeline Overview

```
Phase 0: DETECT    → Layer, type, is_regression?
Phase 1: CLASSIFY  → Severity, blast radius, technique triggers
Phase 2: TRACE     → 5 Whys + Git Bisect + LSP
Phase 3: HYPOTHESIZE → 3 gates, self-correction
Phase 3.5: LLM BLOCK (if >50 LOC)
Phase 4: RESOLVE   → Algorithm selection, fix
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
  sum 17-20: CRITICAL → 5-SOL/META

TECHNIQUE_TRIGGERS:
  git_bisect: is_regression == true
  lsp_tracing: blast_radius >= 3
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
  - Cross-module? → LSP Tracing
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

### LSP Dependency Tracing (if cross-module)

```yaml
PROTOCOL:
  1. lsp_hover at error location → get type
  2. lsp_goto_definition → find source
  3. lsp_find_references → trace usages
  4. Build graph: where value correct → where wrong
  5. Transition point = ROOT CAUSE

INTEGRATE_WITH_5_WHYS: Answer each "Why?" with lsp evidence
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

ENHANCED: Use lsp_find_references to VERIFY each answer
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

### 3 Mandatory Gates (v3.0)

```yaml
GATE_1_INVESTIGATION:
  - Error message read completely
  - Stack trace analyzed
  - Bug reproduced twice
  - Recent changes reviewed
  - Regression check done
  BLOCKING: Cannot form hypothesis until ALL pass

GATE_2_HYPOTHESIS_QUALITY:
  tmk_structure:  # TMK-enhanced (v3.1)
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
DIFFERENT_MODULE_EACH_TIME: → Use LSP Tracing
```

---

## Phase 4: RESOLVE

```yaml
PRE_CHECK:
  - Phase 2 complete
  - Phase 3 gates passed
  - Confidence ≥70%
  - Fix addresses ROOT (not symptom)

ALGORITHM_BY_SEVERITY:
  LOW: STC (direct fix)
  MEDIUM: PR-CoT (thinking protocol)
  HIGH: MAR (/vida-bug-fix)
  CRITICAL: META (/vida-bug-fix --meta)
  MULTI_ERROR: 5-SOL (alignment)

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
  Skip LSP (cross-module),Use lsp_find_references
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

## Codebase-Specific Search Patterns

```toon
search_patterns[4]{type,grep_pattern,files}:
  API,"throw.*Api.*Exception|throw.*AccessDenied","api/**, adapters/*"
  Connection,"Connection|VersionDetection","network/**, api/**"
  Auth,"AuthError\(|AuthErrorType|SessionExpired","auth/**, security/**"
  State,"StateError|DatabaseStateError|CacheStateError","state/**, data/**"
```

---

## Error Hierarchy (generic)

```toon
errors[8]{layer,exception}:
  API,ApiException + AccessDenied + SessionExpired + RecordNotFound + Validation
  Connection,ConnectionException + VersionDetection
  Auth,AuthError (typed variants)
  State,DatabaseStateError
  Offline,OfflineOperation.failed
  UI,User-facing error surface
```

---

## Quick Reference

```toon
error_to_algo[6]{type,algorithm}:
  Syntax/Typo,STC
  Single exception,STC → PR-CoT if unclear
  State/API,PR-CoT → MAR if cross-module
  Auth flow,META (always)
  Data integrity,META (always)
  Multiple errors,5-SOL (alignment)
```

```toon
technique_selection[4]{trigger,technique}:
  "Used to work",Git Bisect
  Cross-module (3+ files),LSP Tracing
  Function >50 LOC,LLM Block Analysis
  All bugs,Stricter Checkpoints
```

---

## VIDA Integration

```toon
commands[5]{severity,command}:
  LOW,Direct fix
  MEDIUM,thinking protocol {error}
  HIGH,/vida-bug-fix T-XX
  CRITICAL,/vida-bug-fix --meta
  MULTI,/vida-bug-fix --batch
```

---

*VIDA Framework — Error Search v3.1*
*Git Bisect + LLM Block + LSP Tracing + Stricter Checkpoints + TMK Hypothesis*


## Section: web-search

# Web Search Integration (Canonical via WVP)

> **⛔ MANDATORY.** Canonical web/internet validation rules live in `_vida/docs/web-validation-protocol.md`.

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

## Command Integration

```yaml
COMMANDS:
  /vida-research: WVP for external/domain evidence.
  /vida-spec: WVP + API reality checks before technical contract freeze.
  /vida-form-task: WVP for dependency/version assumptions in task pool.
  /vida-implement: WVP on errors, package upgrades, platform and security decisions.
  /vida-bug-fix: WVP when bug hypothesis relies on external contracts/known issues.
```

## Log Requirement

When WVP trigger fires, record concise WVP evidence in TODO logs/report.

*VIDA Framework — Web Search Integration (delegated to WVP)*


## Section: reasoning-modules

# Reasoning Modules Library v2.2

> **Source:** Self-Discover Framework (arXiv:2402.03620, Google DeepMind)  
> **Purpose:** 20 curated atomic reasoning modules for VIDA thinking algorithms  
> **Usage:** SELECT relevant modules during META Step 0 or 5-SOL category generation  
> **v2.2:** TMK (Task-Method-Knowledge) structure added to modules. When using a module, apply its T/M/K scaffold for structured reasoning.

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
    vida_mapping: `_vida/docs/thinking-protocol.md#section-bug-reasoning`, MAR Round 1
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
      - creativity_weight >= 0.7
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

### By VIDA Algorithm

toon[6]{algorithm,recommended_modules}:
  STC,D2 (step_by_step) + A4 (core_issue)
  PR-CoT,A1 (critical) + A2 (systems) + G2 (alternatives) + V1 (assumptions)
  MAR,A5 (root_cause) + V2 (risks) + V4 (obstacles) + M1 (reflective)
  5-SOL,G1 (creative) + G2 (alternatives) + G3 (new_solution) + D1 (break_down)
  META,SELECT 3-5 based on problem domain

### By Problem Type

toon[5]{problem_type,recommended_modules}:
  bug_fix,A4 + A5 + D2 + V4
  new_feature,D1 + G1 + G2 + V1 + M2
  refactoring,A2 + D3 + V2 + V3
  architecture,A1 + A2 + A3 + G2 + V2
  security,A3 + V1 + V2 + V4

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

*VIDA Framework — Self-Discover Reasoning Modules v2.2*
*20 curated from 39 original (arXiv:2402.03620) + TMK structured scaffolds*
