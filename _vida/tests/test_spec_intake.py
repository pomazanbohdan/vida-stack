from __future__ import annotations

import json
import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parent.parent / "scripts" / "spec-intake.py"
SPEC = importlib.util.spec_from_file_location("spec_intake_test_runtime", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load spec-intake helper: {SCRIPT_PATH}")
spec_intake = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(spec_intake)


class SpecIntakeTests(unittest.TestCase):
    def test_normalize_and_validate_ready_for_scp(self) -> None:
        payload = spec_intake.normalize_payload(
            "mobile-test",
            {
                "intake_class": "research",
                "problem_statement": "Need a better spec path.",
                "requested_outcome": "Define a compact intake artifact.",
                "proposed_scope_in": ["spec formation"],
                "recommended_contract_path": "scp",
                "status": "ready_for_scp",
            },
        )
        ok, reason = spec_intake.validate_payload(payload, "mobile-test")
        self.assertTrue(ok, reason)

    def test_validate_requires_open_decisions_for_negotiation(self) -> None:
        payload = spec_intake.normalize_payload(
            "mobile-test",
            {
                "intake_class": "user_negotiation",
                "problem_statement": "Need to clarify scope.",
                "requested_outcome": "Negotiate accepted scope.",
                "recommended_contract_path": "user_negotiation",
                "status": "needs_user_negotiation",
            },
        )
        ok, reason = spec_intake.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "missing_open_decisions")

    def test_validate_requires_issue_contract_path_for_issue_ready(self) -> None:
        payload = spec_intake.normalize_payload(
            "mobile-test",
            {
                "intake_class": "issue",
                "problem_statement": "Regression reported.",
                "requested_outcome": "Narrow into equivalent bug scope.",
                "proposed_scope_in": ["reported bug"],
                "recommended_contract_path": "scp",
                "status": "ready_for_issue_contract",
            },
        )
        ok, reason = spec_intake.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "issue_contract_path_required")

    def test_validate_requires_spec_delta_path_for_delta_status(self) -> None:
        payload = spec_intake.normalize_payload(
            "mobile-test",
            {
                "intake_class": "mixed",
                "problem_statement": "Behavior change detected.",
                "requested_outcome": "Route through delta reconciliation.",
                "proposed_scope_in": ["settings flow"],
                "recommended_contract_path": "scp",
                "status": "needs_spec_delta",
            },
        )
        ok, reason = spec_intake.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "spec_delta_path_required")

    def test_validate_insufficient_intake_requires_gather_evidence_path(self) -> None:
        payload = spec_intake.normalize_payload(
            "mobile-test",
            {
                "intake_class": "mixed",
                "problem_statement": "Not enough evidence yet.",
                "requested_outcome": "Gather more input.",
                "recommended_contract_path": "scp",
                "status": "insufficient_intake",
            },
        )
        ok, reason = spec_intake.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "gather_evidence_path_required")

    def test_write_and_status_roundtrip(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / "input.json"
            output_path = Path(tmp) / "out.json"
            input_path.write_text(
                json.dumps(
                    {
                        "intake_class": "mixed",
                        "problem_statement": "Research and issue mixed.",
                        "requested_outcome": "Route into SCP.",
                        "research_findings": ["fact"],
                        "issue_signals": ["signal"],
                        "proposed_scope_in": ["bounded scope"],
                        "recommended_contract_path": "scp",
                        "status": "ready_for_scp",
                    }
                ),
                encoding="utf-8",
            )
            code = spec_intake.write_payload("mobile-test", input_path, output_path)
            self.assertEqual(code, 0)
            payload, _ = spec_intake.load_payload("mobile-test", output_path)
            self.assertEqual(payload["status"], "ready_for_scp")


if __name__ == "__main__":
    unittest.main()
