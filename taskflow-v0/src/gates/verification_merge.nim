## VIDA Verification Merge — executable admissibility and merge helpers.

import std/[algorithm, json, sequtils, sets, strutils]
import ../core/[toon, utils]

proc normalizedStringList(value: JsonNode): seq[string] =
  result = @[]
  if value.isNil:
    return
  case value.kind
  of JArray:
    for item in value:
      let text = (if item.kind == JString: item.getStr() else: $item).strip()
      if text.len > 0 and text notin result:
        result.add(text)
  of JString:
    for item in value.getStr().split(','):
      let text = item.strip()
      if text.len > 0 and text notin result:
        result.add(text)
  else:
    discard

proc verifierId(resultNode: JsonNode): string =
  let direct = policyValue(dottedGet(resultNode, "verifier_id", newJNull()), "")
  if direct.len > 0:
    return direct
  policyValue(dottedGet(resultNode, "agent_backend", newJNull()), "")

proc verifierVerdict(resultNode: JsonNode): string =
  let verdict = policyValue(dottedGet(resultNode, "verdict", newJNull()), "").toLowerAscii()
  if verdict.len > 0:
    return verdict
  if dottedGetBool(resultNode, "approved", false):
    return "passed"
  case policyValue(dottedGet(resultNode, "status", newJNull()), "").toLowerAscii()
  of "ok", "passed", "verified":
    "passed"
  of "failed", "blocked":
    "failed"
  else:
    "inconclusive"

proc verifierIndependent(resultNode: JsonNode): bool =
  if resultNode.isNil or resultNode.kind != JObject:
    return false
  let value = dottedGet(resultNode, "independent", newJNull())
  if value.kind == JBool:
    return value.getBool()
  true

proc schemaCompatible(resultNode: JsonNode): bool =
  if resultNode.isNil or resultNode.kind != JObject:
    return false
  let value = dottedGet(resultNode, "schema_compatible", newJNull())
  if value.kind == JBool:
    return value.getBool()
  true

proc blockingFailure(resultNode: JsonNode): bool =
  if resultNode.isNil or resultNode.kind != JObject:
    return false
  let value = dottedGet(resultNode, "blocking_failure", newJNull())
  if value.kind == JBool:
    return value.getBool()
  verifierVerdict(resultNode) == "failed"

proc mergeAdmissibility*(results: seq[JsonNode], requiredCount: int,
                         requiredProofCategories: seq[string] = @[]): JsonNode =
  let normalizedRequired = max(1, requiredCount)
  var verifierIds = initHashSet[string]()
  var proofCoverage = initHashSet[string]()
  var duplicateVerifiers: seq[string] = @[]
  var missingIndependence: seq[string] = @[]
  var schemaIssues: seq[string] = @[]

  for resultNode in results:
    let id = verifierId(resultNode)
    if id.len > 0:
      if id in verifierIds:
        duplicateVerifiers.add(id)
      verifierIds.incl(id)
    if not verifierIndependent(resultNode):
      missingIndependence.add(if id.len > 0: id else: "unknown_verifier")
    if not schemaCompatible(resultNode):
      schemaIssues.add(if id.len > 0: id else: "unknown_verifier")
    for category in normalizedStringList(dottedGet(resultNode, "proof_category_coverage", newJNull())):
      proofCoverage.incl(category)

  var missingCoverage: seq[string] = @[]
  for category in requiredProofCategories:
    let trimmed = category.strip()
    if trimmed.len > 0 and trimmed notin proofCoverage:
      missingCoverage.add(trimmed)

  var blockers: seq[string] = @[]
  if results.len < normalizedRequired:
    blockers.add("insufficient_verifier_count:" & $results.len & "/" & $normalizedRequired)
  if duplicateVerifiers.len > 0:
    blockers.add("duplicate_verifier_ids:" & duplicateVerifiers.join(","))
  if missingIndependence.len > 0:
    blockers.add("independence_not_satisfied:" & missingIndependence.join(","))
  if schemaIssues.len > 0:
    blockers.add("schema_incompatible:" & schemaIssues.join(","))
  if missingCoverage.len > 0:
    blockers.add("missing_proof_category_coverage:" & missingCoverage.join(","))

  %*{
    "admissible": blockers.len == 0,
    "required_result_count": normalizedRequired,
    "actual_result_count": results.len,
    "required_proof_categories": requiredProofCategories,
    "covered_proof_categories": toSeq(proofCoverage.items).sorted(),
    "blockers": blockers,
  }

proc mergeVerificationResults*(results: seq[JsonNode], policy: string, requiredCount: int,
                               requiredProofCategories: seq[string] = @[],
                               quorum: int = 0): JsonNode =
  let admissibility = mergeAdmissibility(results, requiredCount, requiredProofCategories)
  let normalizedPolicy = policyValue(policy, "all_pass").toLowerAscii()
  if not dottedGetBool(admissibility, "admissible", false):
    return %*{
      "ok": false,
      "policy": normalizedPolicy,
      "merge_state": "manual_reconcile",
      "verdict": "inconclusive",
      "admissibility": admissibility,
      "reason": normalizedStringList(admissibility{"blockers"}).join("; "),
    }

  var passed, failed, inconclusive: seq[string] = @[]
  for resultNode in results:
    let id = verifierId(resultNode)
    let label = if id.len > 0: id else: "unknown_verifier"
    case verifierVerdict(resultNode)
    of "passed":
      passed.add(label)
    of "failed":
      failed.add(label)
    else:
      inconclusive.add(label)

  var mergeState = "manual_reconcile"
  var verdict = "inconclusive"
  var reason = ""

  case normalizedPolicy
  of "all_pass":
    if failed.len > 0:
      mergeState = "merged_fail"
      verdict = "failed"
      reason = "verifier_failed:" & failed.join(",")
    elif inconclusive.len == 0 and passed.len >= max(1, requiredCount):
      mergeState = "merged_pass"
      verdict = "passed"
      reason = "all_required_verifiers_passed"
    else:
      reason = "inconclusive_verifier_results"
  of "quorum_pass":
    let requiredQuorum = if quorum > 0: quorum else: max(1, requiredCount)
    if failed.len > 0 and passed.len < requiredQuorum:
      mergeState = "merged_fail"
      verdict = "failed"
      reason = "quorum_unreachable_due_to_failures"
    elif passed.len >= requiredQuorum:
      mergeState = "merged_pass"
      verdict = "passed"
      reason = "quorum_satisfied"
    else:
      reason = "quorum_not_satisfied"
  of "first_strong_fail":
    var strongFail = ""
    for resultNode in results:
      if blockingFailure(resultNode):
        strongFail = verifierId(resultNode)
        if strongFail.len == 0:
          strongFail = "unknown_verifier"
        break
    if strongFail.len > 0:
      mergeState = "merged_fail"
      verdict = "failed"
      reason = "blocking_failure:" & strongFail
    elif inconclusive.len == 0 and passed.len >= max(1, requiredCount):
      mergeState = "merged_pass"
      verdict = "passed"
      reason = "all_required_verifiers_passed"
    else:
      reason = "insufficient_strong_verification"
  of "manual_reconcile":
    reason = "manual_reconcile_requested"
  else:
    reason = "unsupported_merge_policy:" & normalizedPolicy

  %*{
    "ok": mergeState in ["merged_pass", "merged_fail"],
    "policy": normalizedPolicy,
    "merge_state": mergeState,
    "verdict": verdict,
    "reason": reason,
    "admissibility": admissibility,
    "passed_verifiers": passed,
    "failed_verifiers": failed,
    "inconclusive_verifiers": inconclusive,
  }

proc parseResultsArg(arg: string): seq[JsonNode] =
  let payload =
    if arg == "-":
      parseJson(stdin.readAll())
    else:
      parseJson(readFile(arg))
  if payload.kind == JArray:
    for item in payload:
      if item.kind == JObject:
        result.add(item)

proc cmdVerificationMerge*(args: seq[string]): int =
  if args.len < 3:
    echo """Usage:
  taskflow-v0 verification admissibility <required_results> <results_json_file|-> [required_categories_csv] [--json]
  taskflow-v0 verification merge <policy> <required_results> <results_json_file|-> [required_categories_csv] [quorum] [--json]"""
    return 1

  let asJson = "--json" in args
  case args[0]
  of "admissibility":
    let requiredResults = try: parseInt(args[1]) except ValueError: 1
    let results = parseResultsArg(args[2])
    let categories = if args.len > 3 and args[3] != "--json": args[3].split(',') else: @[]
    let payload = normalizeJson(mergeAdmissibility(results, requiredResults, categories))
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return (if dottedGetBool(payload, "admissible", false): 0 else: 2)
  of "merge":
    let policy = args[1]
    let requiredResults = try: parseInt(args[2]) except ValueError: 1
    let results = parseResultsArg(args[3])
    let categories = if args.len > 4 and args[4] != "--json": args[4].split(',') else: @[]
    let quorum =
      if args.len > 5 and args[5] != "--json":
        try: parseInt(args[5]) except ValueError: 0
      else:
        0
    let payload = normalizeJson(mergeVerificationResults(results, policy, requiredResults, categories, quorum))
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return (if dottedGetBool(payload, "ok", false): 0 else: 2)
  else:
    echo "Unknown verification subcommand: " & args[0]
    return 1
