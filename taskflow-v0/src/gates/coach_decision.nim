## VIDA Coach Decision helpers — parse and merge coach outputs.

import std/[json, strutils, sets]
import ../core/[toon, utils]
import ./worker_packet

proc normalizedStringList(value: JsonNode): seq[string] =
  result = @[]
  if value.isNil or value.kind != JArray:
    return
  for item in value:
    let text = (if item.kind == JString: item.getStr() else: $item).strip()
    if text.len > 0:
      result.add(text)

proc dedupedStrings(items: seq[string]): seq[string] =
  var seen = initHashSet[string]()
  result = @[]
  for item in items:
    let text = item.strip()
    if text.len > 0 and text notin seen:
      seen.incl(text)
      result.add(text)

proc normalizedImpactAnalysis(value: JsonNode): JsonNode =
  if value.isNil or value.kind != JObject:
    return newJObject()
  let payload = %*{
    "affected_scope": dedupedStrings(normalizedStringList(value{"affected_scope"})),
    "contract_impact": dedupedStrings(normalizedStringList(value{"contract_impact"})),
    "follow_up_actions": dedupedStrings(normalizedStringList(value{"follow_up_actions"})),
    "residual_risks": dedupedStrings(normalizedStringList(value{"residual_risks"})),
  }
  var resultPayload = newJObject()
  for key, node in payload:
    if node.kind == JArray and node.len > 0:
      resultPayload[key] = node
  resultPayload

proc coachDecisionIsValid*(decision: JsonNode): bool =
  decision.kind == JObject and
    decision{"parsed_json"}.kind == JBool and
    decision{"parsed_json"}.getBool() and
    decision{"coach_decision"}.kind == JString and
    decision{"coach_decision"}.getStr() in ["approved", "return_for_rework"]

proc coachFeedbackSummary(decisions: seq[JsonNode]): string =
  var lines: seq[string] = @[]
  for decision in decisions:
    let feedback = dottedGetStr(decision, "coach_feedback")
    let fallbackReason = dottedGetStr(decision, "reason")
    let fallbackAnswer = dottedGetStr(decision, "answer")
    let selected = if feedback.len > 0: feedback elif fallbackReason.len > 0: fallbackReason else: fallbackAnswer
    if selected.len == 0:
      continue
    let agentBackend = dottedGetStr(decision, "agent_backend")
    let line = (if agentBackend.len > 0: "[" & agentBackend & "] " else: "") & selected
    if line notin lines:
      lines.add(line)
  lines.join("\n")

proc mergeImpactAnalyses(decisions: seq[JsonNode]): JsonNode =
  var affected, impact, followUp, residual: seq[string] = @[]
  for decision in decisions:
    let normalized = normalizedImpactAnalysis(decision{"impact_analysis"})
    affected.add(normalizedStringList(normalized{"affected_scope"}))
    impact.add(normalizedStringList(normalized{"contract_impact"}))
    followUp.add(normalizedStringList(normalized{"follow_up_actions"}))
    residual.add(normalizedStringList(normalized{"residual_risks"}))
  var payload = newJObject()
  let affectedDedup = dedupedStrings(affected)
  let impactDedup = dedupedStrings(impact)
  let followUpDedup = dedupedStrings(followUp)
  let residualDedup = dedupedStrings(residual)
  if affectedDedup.len > 0: payload["affected_scope"] = %affectedDedup
  if impactDedup.len > 0: payload["contract_impact"] = %impactDedup
  if followUpDedup.len > 0: payload["follow_up_actions"] = %followUpDedup
  if residualDedup.len > 0: payload["residual_risks"] = %residualDedup
  payload

proc coachPayloadConflictState(requestedDecision, mergeReady, reworkRequired: string, blockers: seq[string]): tuple[state: string, invalidReasons: seq[string]] =
  result.invalidReasons = @[]
  if requestedDecision == "approved":
    if mergeReady == "no":
      result.invalidReasons.add("approved_conflicts_with_merge_ready")
    if reworkRequired == "yes":
      result.invalidReasons.add("approved_conflicts_with_rework_required")
    if blockers.len > 0:
      result.invalidReasons.add("approved_conflicts_with_blockers")
    if result.invalidReasons.len > 0:
      result.state = "invalid_coach_payload.approved_conflict"
  elif requestedDecision == "return_for_rework":
    if mergeReady == "yes" and reworkRequired == "no" and blockers.len == 0:
      result.invalidReasons.add("rework_conflicts_with_clean_approval")
      result.state = "invalid_coach_payload.rework_conflict"

proc parseCoachDecision*(outputText: string): JsonNode =
  let payload = worker_packet.extractJsonPayload(outputText)
  if payload.isNil or payload.kind != JObject:
    return %*{
      "approved": false,
      "coach_decision": "coach_failed",
      "payload_state": "missing_payload",
      "invalid_reasons": ["missing_coach_decision_payload"],
      "rework_required": "yes",
      "coach_feedback": "",
      "recommended_next_action": "",
      "reason": "missing_coach_decision_payload",
      "parsed_json": false,
      "blockers": [],
      "evidence_refs": [],
      "verification_results": [],
      "impact_analysis": {},
      "answer": "",
      "merge_ready_effective": "no",
      "raw_merge_ready": "",
      "raw_rework_required": "",
    }

  let mergeReady = (if payload{"merge_ready"}.kind == JString: payload{"merge_ready"}.getStr().strip().toLowerAscii() else: "")
  var requestedDecision = (if payload{"coach_decision"}.kind == JString: payload{"coach_decision"}.getStr().strip().toLowerAscii() else: "")
  let reworkRequired = (if payload{"rework_required"}.kind == JString: payload{"rework_required"}.getStr().strip().toLowerAscii() else: "")
  let blockers = normalizedStringList(payload{"blockers"})
  let coachFeedback =
    (if payload{"coach_feedback"}.kind == JString: payload{"coach_feedback"}.getStr().strip() else:
      if payload{"notes"}.kind == JString: payload{"notes"}.getStr().strip() else: "")
  let answer = if payload{"answer"}.kind == JString: payload{"answer"}.getStr().strip() else: ""
  var recommendedNextAction = if payload{"recommended_next_action"}.kind == JString: payload{"recommended_next_action"}.getStr().strip() else: ""
  let evidenceRefs = normalizedStringList(payload{"evidence_refs"})
  let verificationResults = normalizedStringList(payload{"verification_results"})
  let impactAnalysis = normalizedImpactAnalysis(payload{"impact_analysis"})

  if requestedDecision notin ["approved", "return_for_rework"]:
    if reworkRequired == "yes" or mergeReady == "no" or blockers.len > 0:
      requestedDecision = "return_for_rework"
    elif mergeReady == "yes" or reworkRequired == "no":
      requestedDecision = "approved"
    else:
      requestedDecision = ""

  var (payloadState, invalidReasons) = coachPayloadConflictState(requestedDecision, mergeReady, reworkRequired, blockers)
  if requestedDecision.len == 0 and invalidReasons.len == 0:
    payloadState = "invalid_coach_payload.ambiguous_finality"
    invalidReasons = @["missing_finality_signals"]

  var approved = false
  var normalizedDecision = ""
  var mergeReadyEffective = "no"
  var reason = ""
  if invalidReasons.len > 0:
    normalizedDecision = payloadState
    recommendedNextAction = if recommendedNextAction.len > 0: recommendedNextAction else: "rerun_coach_review_with_valid_machine_readable_output"
    reason = invalidReasons.join("; ")
    let feedback = if coachFeedback.len > 0: coachFeedback else: answer
    if feedback.len > 0:
      reason &= ": " & feedback
  else:
    approved = requestedDecision == "approved" and reworkRequired != "yes" and mergeReady != "no" and blockers.len == 0
    normalizedDecision = if approved: "approved" else: "return_for_rework"
    mergeReadyEffective = if approved: "yes" else: "no"
    if not approved:
      reason = if blockers.len > 0: blockers.join("; ")
        elif coachFeedback.len > 0: coachFeedback
        elif answer.len > 0: answer
        else: "coach_return_for_rework"

  %*{
    "approved": approved,
    "coach_decision": normalizedDecision,
    "payload_state": (if payloadState.len > 0: payloadState else: normalizedDecision),
    "invalid_reasons": invalidReasons,
    "rework_required": (if approved: "no" else: "yes"),
    "coach_feedback": (if coachFeedback.len > 0: coachFeedback else: answer),
    "recommended_next_action": recommendedNextAction,
    "reason": reason,
    "parsed_json": true,
    "blockers": blockers,
    "evidence_refs": evidenceRefs,
    "verification_results": verificationResults,
    "impact_analysis": impactAnalysis,
    "answer": answer,
    "merge_ready_effective": mergeReadyEffective,
    "raw_merge_ready": mergeReady,
    "raw_rework_required": reworkRequired,
  }

proc mergeCoachDecisions*(decisions: seq[JsonNode], requiredResults: int, mergePolicy: string): JsonNode =
  let normalizedRequired = max(1, requiredResults)
  var valid: seq[JsonNode] = @[]
  for decision in decisions:
    if coachDecisionIsValid(decision):
      valid.add(decision)
  let validResultCount = valid.len

  if validResultCount < normalizedRequired:
    var reasons = @["insufficient_valid_coach_results:" & $validResultCount & "/" & $normalizedRequired]
    for decision in decisions:
      reasons.add(normalizedStringList(decision{"invalid_reasons"}))
    let invalidReasons = dedupedStrings(reasons)
    return %*{
      "approved": false,
      "coach_decision": "coach_failed",
      "payload_state": "coach_failed",
      "invalid_reasons": invalidReasons,
      "rework_required": "yes",
      "coach_feedback": "",
      "recommended_next_action": "rerun_coach_review_with_independent_valid_outputs",
      "reason": invalidReasons.join("; "),
      "parsed_json": false,
      "blockers": [],
      "evidence_refs": dedupedStrings(@[]),
      "verification_results": dedupedStrings(@[]),
      "impact_analysis": mergeImpactAnalyses(valid),
      "answer": "",
      "merge_ready_effective": "no",
      "raw_merge_ready": "",
      "raw_rework_required": "",
      "valid_result_count": validResultCount,
      "required_result_count": normalizedRequired,
      "merge_policy": mergePolicy,
    }

  var reworkDecisions: seq[JsonNode] = @[]
  for decision in valid:
    if dottedGetStr(decision, "coach_decision") == "return_for_rework" or not dottedGetBool(decision, "approved", false):
      reworkDecisions.add(decision)

  if mergePolicy == "unanimous_approve_rework_bias" and reworkDecisions.len == 0:
    let feedback = coachFeedbackSummary(valid)
    return %*{
      "approved": true,
      "coach_decision": "approved",
      "payload_state": "approved",
      "invalid_reasons": [],
      "rework_required": "no",
      "coach_feedback": feedback,
      "recommended_next_action": "proceed_to_independent_verification",
      "reason": "all_required_coaches_approved",
      "parsed_json": true,
      "blockers": [],
      "evidence_refs": dedupedStrings(@[]),
      "verification_results": dedupedStrings(@[]),
      "impact_analysis": mergeImpactAnalyses(valid),
      "answer": feedback,
      "merge_ready_effective": "yes",
      "raw_merge_ready": "",
      "raw_rework_required": "",
      "valid_result_count": validResultCount,
      "required_result_count": normalizedRequired,
      "merge_policy": mergePolicy,
    }

  let blockingDecisions = if reworkDecisions.len > 0: reworkDecisions else: valid
  var blockers, refs, verifications, nextActions: seq[string] = @[]
  for decision in blockingDecisions:
    blockers.add(normalizedStringList(decision{"blockers"}))
    refs.add(normalizedStringList(decision{"evidence_refs"}))
    verifications.add(normalizedStringList(decision{"verification_results"}))
    let action = dottedGetStr(decision, "recommended_next_action")
    if action.len > 0:
      nextActions.add(action)
  let dedupBlockers = dedupedStrings(blockers)
  let feedback = coachFeedbackSummary(blockingDecisions)
  let reason =
    if dedupBlockers.len > 0:
      dedupBlockers.join("; ")
    elif feedback.len > 0:
      feedback
    else:
      "coach_rework_required"
  %*{
    "approved": false,
    "coach_decision": "return_for_rework",
    "payload_state": "return_for_rework",
    "invalid_reasons": [],
    "rework_required": "yes",
    "coach_feedback": feedback,
    "recommended_next_action": dedupedStrings(nextActions).join("; "),
    "reason": reason,
    "parsed_json": true,
    "blockers": dedupBlockers,
    "evidence_refs": dedupedStrings(refs),
    "verification_results": dedupedStrings(verifications),
    "impact_analysis": mergeImpactAnalyses(blockingDecisions),
    "answer": feedback,
    "merge_ready_effective": "no",
    "raw_merge_ready": "",
    "raw_rework_required": "",
    "valid_result_count": validResultCount,
    "required_result_count": normalizedRequired,
    "merge_policy": mergePolicy,
  }

proc cmdCoachDecision*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 coach-decision parse <output_file|->
  taskflow-v0 coach-decision merge <required_results> <merge_policy> <decisions_json_file|->"""
    return 1

  case args[0]
  of "parse":
    if args.len < 2:
      echo "Usage: taskflow-v0 coach-decision parse <output_file|->"
      return 1
    let text = if args[1] == "-": stdin.readAll() else: readFile(args[1])
    let payload = normalizeJson(parseCoachDecision(text))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  of "merge":
    if args.len < 4:
      echo "Usage: taskflow-v0 coach-decision merge <required_results> <merge_policy> <decisions_json_file|->"
      return 1
    let requiredResults = try: parseInt(args[1]) except ValueError: 1
    let decisionsPayload =
      if args[3] == "-": parseJson(stdin.readAll())
      else: loadJson(args[3], newJArray())
    if decisionsPayload.kind != JArray:
      echo "{\"status\":\"blocked\",\"reason\":\"decisions_json_must_be_array\"}"
      return 2
    var decisions: seq[JsonNode] = @[]
    for item in decisionsPayload:
      decisions.add(item)
    let payload = normalizeJson(mergeCoachDecisions(decisions, requiredResults, args[2]))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  else:
    echo "Unknown coach-decision subcommand: " & args[0]
    return 1
