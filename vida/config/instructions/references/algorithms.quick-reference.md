# Algorithms Quick Reference

Purpose: algorithm reference preserving triggers/gates.

## Unified Matrix

| Algorithm | When To Use | Mandatory Steps (minimum) | Quality Gate | Escalation |
|---|---|---|---|---|
| STC | Selector score `<=12`; low complexity, local tasks | Step -> check -> localize -> rollback -> retry (<=3) | No unresolved step errors remain | After 3 retries -> PR-CoT; protocol/route ambiguity -> META |
| PR-CoT | Selector score `13-22`; medium complexity, independent perspectives needed | Pass1 (4 perspectives) -> consensus -> Pass2 (revision) | No unresolved critical findings remain | Unresolved critical or >=2 issues after Pass2 -> MAR |
| MAR | Selector score `23-32`; complex tasks with regression risk | 3 rounds (Actor/Evaluator/Critic/Reflector) | Weighted rubric score >= 8/10 and no unresolved critical residual risk | <8 after 3 rounds -> META |
| 5-SOL | Selector score `33-42`; choice between alternatives / design directions | R1: 5 options -> weighted option ledger -> R2: 5 new options -> legal hybrid/top option | Admissible choice with weighted option score + confidence >= 80, or explicit cautious band | Low score/confidence or legality pressure -> META |
| META | Selector score `>42`; high risk/uncertainty, security/auth, explicit meta-analysis | Select domain packet -> choose blocks -> admissibility gate -> family-weighted confidence -> synthesize | Admissible result with confidence >= 80% and proof receipts | If <80% after repair loop -> best admissible option/user decision |
| Error Search / Bug Reasoning | Bugs/incidents/regressions/multi-error pools | scope -> evidence freeze -> cluster -> authority map -> root-cause trace -> falsifiable hypothesis -> fix locus -> proof matrix | Root cause confirmed and proof covers blast radius | Governance/authority ambiguity -> META; alternatives after root cause -> 5-SOL |
| Web-Search Gate | Unstable external knowledge | detect trigger -> find sources -> reconcile | >=2 sources (>=3 for sec/arch) | If sources conflict -> escalate algorithm |

## Unified Scoring Contract

`selector_score` is routing-only, stays on `11-55`, and uses `C×2 + R×3 + S×3 + N×2 + F×1`. Default bands: `STC <=12`, `PR-CoT 13-22`, `MAR 23-32`, `5-SOL 33-42`, `META >42`. `PR-CoT` exports gate result + `validation_signal`; `MAR` keeps `1-10` weighted rubric + `refinement_signal`; `5-SOL` keeps `1-5` category scoring and exports `best option %`, `agreement %`, `options_signal`; `META` uses normalized signals after admissibility gates and task-class weights.

## Routing Escalators

Route directly to `META` for protocol conflict, execution gate mismatch, fail-closed law risk, framework-owned behavior change, or tracked writer `no_eligible_analysis_lane` / `no_eligible_verifier` / `no_eligible_coach` with policy decision required. Keep score-selected route only for mostly local implementation without governance ambiguity. If review/gate/root-cause evidence proves `STC` misclassification, do not reuse `STC` for the same task class in the current pass; promote to at least `PR-CoT`, or `META` for protocol/fail-closed/framework-routing cases.

## Algorithm Cards

### STC
When: simple tasks, score `<=12`. Input: clear local objective. Steps: generate step -> verify -> localize first error -> roll back to clean prefix -> retry. Success: solved without logical gaps. Escalation: after 3 failed retries or protocol/route ambiguity.

### PR-CoT
When: score `13-22`, medium complexity needing validation. Input: multi-aspect task. Steps: 4 perspectives -> consensus -> revision. Success: aligned decision, no unresolved critical findings. Export: `validation_signal`. Escalation: unresolved critical or >=2 issues.

### MAR
When: score `23-32`, complex non-trivial decisions. Input: high impact radius task. Steps: 3 role rounds + lessons learned. Success: weighted rubric >= 8/10 and no unresolved critical residual risk. Weights: correctness `0.35`, completeness `0.25`, alignment `0.25`, simplicity `0.15`. Escalation: score < 8 after 3 rounds.

### 5-SOL
- When: selector score `33-42`, a justified choice between directions is needed.
- Input: a task with alternatives and trade-offs.
- Steps: 5 R1 options, weighted option ledger, 5 new R2 options, legal hybrid or explicit top single option.
- Success: admissible choice with transparent pros/cons, weighted option score, and confidence >= 80% or explicit cautious band.
- Weighting: 2 core categories = `0.25` each; supporting categories share the remaining `0.50`.
- Escalation: low score/confidence between rounds or failed legality.

### META
- When: selector score `>42`, high-stakes decisions, security/auth, framework-owned behavior changes, protocol conflicts, fail-closed law risk, tracked writer routing gaps, or an explicit meta-analysis request.
- Input: a complex task with a high cost of error.
- Steps: select a domain packet, assemble the smallest lawful block flow, run admissibility gate, apply family weights, synthesize.
- Success: admissible result with confidence >= 80% and proof artifacts.
- Family weights: task-class dependent, with validation heavier for security/schema work and options heavier for architecture/tech-stack work.
- Escalation: if confidence remains low.

### Error Search / Bug Reasoning
- When: bugs, incidents, regressions, repeated technical failures, or multi-error pools.
- Input: observable failure evidence, ideally reproducible, with environment/config/timing context when available.
- Steps: trigger scope -> evidence freeze -> symptom clustering -> authority/source-of-truth map -> root-cause trace -> optional delta minimization -> falsifiable hypothesis -> fix locus decision -> proof matrix -> post-fix learning.
- Success: the first wrong transition point is fixed, the proof matrix covers the claimed blast radius, and a regression guard or follow-up exists when the defect is likely to recur.
- Escalation: non-reproducible critical failures, governance/source-of-truth ambiguity, safety or ownership ambiguity -> META; multiple admissible fix designs after root-cause receipt -> 5-SOL.

### Web-Search Gate
- When: external knowledge may be stale.
- Input: dependency/API/security/runtime questions.
- Steps: check trigger -> collect sources -> reconcile versions/dates.
- Success: sources are aligned and current.
- Escalation: conflicting or insufficient sources.

## Matrix: This Specific Question Through All Algorithms

Question: "How can the algorithm descriptions be optimized without losing their essence?"

| Algorithm | Result For This Question | Strength | Limitation |
|---|---|---|---|
| STC | Two-level format (Card + Deep Spec) | Fast and practical | Less alternative validation |
| PR-CoT | Added consensus format and unified card fields | Balance between speed and quality | Requires more time |
| MAR | Added quality gates and scoring | Best for standard stability | Heavier process |
| 5-SOL | Compared 5 documentation formats and chose a hybrid | Transparent trade-offs | Excessive for simple tasks |
| META | Combined standard + governance | Maximum reliability | Highest time cost |

Synthesis: keep `Quick Reference` as operational layer; keep `Deep Spec` in `instruction-contracts/overlay.step-thinking-protocol` as canonical; preserve triggers, quality gates, escalation rules; smoke gate: if quality gate fails -> escalate to the next algorithm.

-----
artifact_path: config/instructions/references/algorithms.quick-reference
artifact_type: reference
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/references/algorithms.quick-reference.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: 2026-07-03T14:05:00+03:00
changelog_ref: algorithms.quick-reference.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: quick-reference-compaction+semantic-atom-coverage+gate-preserve-exact
protocol_compression_baseline_ref: 4aee9451c:vida/config/instructions/references/algorithms.quick-reference.md
protocol_compression_audit_at: 2026-07-03T14:05:00+03:00
protocol_compression_before_tokens: 1947
protocol_compression_after_tokens: 1946
protocol_compression_content_sha256: 585eae2ed5819d7bc839dfc0e5c89c7a4c8ac3ae3be2b8df327993746d9aef45
