## VIDA Issue Contract — normalize, persist, and validate issue contracts.

import std/[json, sets, strutils]
import ../core/utils
import ../agents/route

proc artifactPath*(taskId: string): string =
  route.issueContractPath(taskId)

proc textList(value: JsonNode): JsonNode =
  result = newJArray()
  if value.isNil or value.kind == JNull:
    return
  if value.kind == JArray:
    for item in value:
      let text = policyValue(item, "")
      if text.len > 0:
        result.add(%text)
  else:
    let text = policyValue(value, "")
    if text.len > 0:
      result.add(%text)

proc dedupStrings(values: JsonNode): JsonNode =
  var seen = initHashSet[string]()
  result = newJArray()
  if values.isNil or values.kind != JArray:
    return
  for item in values:
    let text = policyValue(item, "")
    if text.len > 0 and text notin seen:
      seen.incl(text)
      result.add(%text)

proc normalizeSymptoms(raw: JsonNode): JsonNode =
  result = newJArray()
  if raw.isNil or raw.kind != JArray:
    return
  var index = 0
  for item in raw:
    inc index
    if item.kind != JObject:
      continue
    let symptomId =
      (let value = dottedGetStr(item, "id"); if value.len > 0: value else: "SYM-" & $index)
    var evidenceStatus = dottedGetStr(item, "evidence_status", "unproven").toLowerAscii()
    if evidenceStatus notin ["reproduced", "red_test", "live_evidence", "unproven"]:
      evidenceStatus = "unproven"
    var disposition = dottedGetStr(item, "disposition", "in_scope").toLowerAscii()
    if disposition notin ["in_scope", "out_of_scope"]:
      disposition = "in_scope"
    result.add(%*{
      "id": symptomId,
      "summary": dottedGetStr(item, "summary"),
      "evidence_status": evidenceStatus,
      "disposition": disposition,
      "evidence_refs": dedupStrings(textList(item{"evidence_refs"})),
    })

proc issueContractStatus*(classification: string, equivalenceAssessment: string): string =
  let normalizedEquivalence = equivalenceAssessment.strip().toLowerAscii()
  let normalizedClassification = classification.strip().toLowerAscii()
  if normalizedEquivalence == "equivalent_fix":
    return "writer_ready"
  if normalizedEquivalence == "spec_delta_required":
    return "spec_delta_required"
  if normalizedEquivalence in ["as_designed", "not_a_bug"]:
    return "issue_closed_no_fix"
  if normalizedClassification == "defect_equivalent":
    return "writer_ready"
  if normalizedClassification in ["defect_needs_contract_update", "feature_delta"]:
    return "spec_delta_required"
  if normalizedClassification in ["as_designed", "not_a_bug"]:
    return "issue_closed_no_fix"
  "insufficient_evidence"

proc resolutionPath*(status: string): string =
  case status
  of "writer_ready": "implementation"
  of "spec_delta_required": "spec_reconciliation"
  of "issue_closed_no_fix": "close_without_writer"
  else: "blocked"

proc normalizePayload*(taskId, taskClass: string, routePayload, raw: JsonNode): JsonNode =
  let classification = dottedGetStr(raw, "classification", "insufficient_evidence")
  let equivalenceAssessment = dottedGetStr(raw, "equivalence_assessment")
  let status = issueContractStatus(classification, equivalenceAssessment)
  let rawReportedScope = if raw.hasKey("reported_scope"): raw{"reported_scope"} else: raw{"scope_in"}
  let rawProvenScope = if raw.hasKey("proven_scope"): raw{"proven_scope"} else: raw{"scope_in"}
  let reportedScope = dedupStrings(textList(rawReportedScope))
  let provenScope = dedupStrings(textList(rawProvenScope))
  %*{
    "ts": nowUtc(),
    "task_id": taskId,
    "task_class": taskClass,
    "status": status,
    "classification": classification,
    "equivalence_assessment": equivalenceAssessment,
    "reported_behavior": dottedGetStr(raw, "reported_behavior"),
    "expected_behavior": dottedGetStr(raw, "expected_behavior"),
    "reported_scope": reportedScope,
    "proven_scope": provenScope,
    "symptoms": normalizeSymptoms(raw{"symptoms"}),
    "scope_in": provenScope,
    "scope_out": dedupStrings(textList(raw{"scope_out"})),
    "acceptance_checks": dedupStrings(textList(raw{"acceptance_checks"})),
    "spec_sync_targets": dedupStrings(textList(raw{"spec_sync_targets"})),
    "wvp_required": dottedGetStr(raw, "wvp_required", "no"),
    "wvp_status": dottedGetStr(raw, "wvp_status", "unknown"),
    "resolution_path": resolutionPath(status),
    "route_receipt_hash": route.routeReceiptHash(routePayload),
    "route_receipt": route.routeReceiptPayload(routePayload),
  }

proc validatePayload*(payload, routePayload: JsonNode): tuple[ok: bool, reason: string] =
  let status = dottedGetStr(payload, "status")
  if status.len == 0:
    return (false, "missing_issue_contract_status")
  if dottedGetStr(payload, "route_receipt_hash") != route.routeReceiptHash(routePayload):
    return (false, "stale_issue_contract")
  case status
  of "writer_ready":
    if payload{"proven_scope"}.kind != JArray or payload{"proven_scope"}.len == 0:
      return (false, "missing_proven_scope")
    let symptoms = payload{"symptoms"}
    if not symptoms.isNil and symptoms.kind == JArray and symptoms.len > 1:
      var unresolved: seq[string] = @[]
      for item in symptoms:
        if item.kind != JObject:
          unresolved.add("(invalid_symptom)")
          continue
        if dottedGetStr(item, "disposition", "in_scope") == "out_of_scope":
          continue
        let evidenceStatus = dottedGetStr(item, "evidence_status", "unproven")
        if evidenceStatus notin ["reproduced", "red_test", "live_evidence"]:
          let symptomId = dottedGetStr(item, "id", "SYM")
          unresolved.add(symptomId)
      if unresolved.len > 0:
        return (false, "unproven_symptoms:" & unresolved.join(","))
    return (true, "")
  of "spec_delta_required", "issue_closed_no_fix", "insufficient_evidence":
    return (false, status)
  else:
    return (false, "invalid_issue_contract_status")

proc buildSpecDeltaFromIssueContract*(payload: JsonNode): JsonNode =
  if dottedGetStr(payload, "status") != "spec_delta_required":
    return newJObject()
  %*{
    "task_id": dottedGetStr(payload, "task_id"),
    "delta_source": "issue_contract",
    "trigger_status": "spec_delta_required",
    "current_contract": dottedGetStr(payload, "expected_behavior"),
    "proposed_contract": dottedGetStr(payload, "reported_behavior"),
    "delta_summary": "Issue requires non-equivalent contract reconciliation before writer execution.",
    "behavior_change": "user_visible_or_contract_visible",
    "scope_impact": dedupStrings(textList(%*[
      payload{"reported_scope"},
      payload{"proven_scope"},
    ])),
    "user_confirmation_required": "yes",
    "reconciliation_targets": dedupStrings(textList(payload{"spec_sync_targets"})),
    "status": "needs_scp_reconciliation",
  }
