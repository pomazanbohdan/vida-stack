import std/[json, strutils, unittest]
import ../src/gates/verification_merge

suite "verification merge":
  test "admissibility fails on duplicate verifier and missing coverage":
    let results = @[
      %*{"verifier_id": "v1", "verdict": "passed", "proof_category_coverage": ["test_report"]},
      %*{"verifier_id": "v1", "verdict": "passed", "proof_category_coverage": ["diff_summary"]},
    ]
    let payload = mergeAdmissibility(results, 2, @["test_report", "verification_evidence"])
    check payload["admissible"].getBool() == false
    check payload["blockers"].len >= 2

  test "all_pass merge succeeds with independent verifier set":
    let results = @[
      %*{"verifier_id": "v1", "verdict": "passed", "proof_category_coverage": ["test_report", "verification_evidence"], "independent": true, "schema_compatible": true},
      %*{"verifier_id": "v2", "verdict": "passed", "proof_category_coverage": ["test_report", "verification_evidence"], "independent": true, "schema_compatible": true},
    ]
    let payload = mergeVerificationResults(results, "all_pass", 2, @["test_report", "verification_evidence"])
    check payload["ok"].getBool() == true
    check payload["verdict"].getStr() == "passed"
    check payload["merge_state"].getStr() == "merged_pass"

  test "first_strong_fail merge fails on blocking verifier":
    let results = @[
      %*{"verifier_id": "v1", "verdict": "passed", "proof_category_coverage": ["verification_evidence"], "independent": true},
      %*{"verifier_id": "v2", "verdict": "failed", "proof_category_coverage": ["verification_evidence"], "independent": true, "blocking_failure": true},
    ]
    let payload = mergeVerificationResults(results, "first_strong_fail", 2, @["verification_evidence"])
    check payload["ok"].getBool() == true
    check payload["verdict"].getStr() == "failed"
    check payload["reason"].getStr().startsWith("blocking_failure:")
