from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parent.parent / "scripts" / "spec-delta.py"
SPEC = importlib.util.spec_from_file_location("spec_delta_test_runtime", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load spec-delta helper: {SCRIPT_PATH}")
spec_delta = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(spec_delta)


class SpecDeltaTests(unittest.TestCase):
    def test_validate_delta_ready(self) -> None:
        payload = spec_delta.normalize_payload(
            "mobile-test",
            {
                "delta_source": "issue_contract",
                "trigger_status": "spec_delta_required",
                "current_contract": "Current flow keeps same behavior.",
                "proposed_contract": "New flow changes settings navigation.",
                "delta_summary": "Settings navigation behavior changes.",
                "behavior_change": "user_visible",
                "scope_impact": ["settings", "navigation"],
                "reconciliation_targets": ["docs/specs/settings.md"],
                "status": "delta_ready",
            },
        )
        ok, reason = spec_delta.validate_payload(payload, "mobile-test")
        self.assertTrue(ok, reason)

    def test_confirmation_status_requires_yes_flag(self) -> None:
        payload = spec_delta.normalize_payload(
            "mobile-test",
            {
                "delta_source": "release_signal",
                "current_contract": "Old behavior",
                "proposed_contract": "New behavior",
                "delta_summary": "Release changes behavior",
                "behavior_change": "user_visible",
                "status": "needs_user_confirmation",
            },
        )
        ok, reason = spec_delta.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "user_confirmation_required_yes_expected")

    def test_delta_ready_requires_reconciliation_targets(self) -> None:
        payload = spec_delta.normalize_payload(
            "mobile-test",
            {
                "delta_source": "issue_contract",
                "current_contract": "Old behavior",
                "proposed_contract": "New behavior",
                "delta_summary": "Behavior changes",
                "behavior_change": "user_visible",
                "status": "delta_ready",
            },
        )
        ok, reason = spec_delta.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "missing_reconciliation_targets")

    def test_non_not_required_delta_requires_current_contract(self) -> None:
        payload = spec_delta.normalize_payload(
            "mobile-test",
            {
                "delta_source": "issue_contract",
                "proposed_contract": "New behavior",
                "delta_summary": "Behavior changes",
                "behavior_change": "user_visible",
                "reconciliation_targets": ["docs/specs/settings.md"],
                "status": "needs_scp_reconciliation",
            },
        )
        ok, reason = spec_delta.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "missing_current_contract")

    def test_not_required_must_not_describe_delta(self) -> None:
        payload = spec_delta.normalize_payload(
            "mobile-test",
            {
                "delta_source": "research_findings",
                "delta_summary": "Should not be here",
                "status": "not_required",
            },
        )
        ok, reason = spec_delta.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "not_required_should_not_describe_delta")

    def test_write_roundtrip(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / "input.json"
            output_path = Path(tmp) / "out.json"
            input_path.write_text(
                json.dumps(
                    {
                        "delta_source": "spec_intake",
                        "trigger_status": "needs_spec_delta",
                        "current_contract": "Old contract",
                        "proposed_contract": "New contract",
                        "delta_summary": "Behavior changes",
                        "behavior_change": "user_visible",
                        "scope_impact": ["settings"],
                        "reconciliation_targets": ["docs/specs/settings.md"],
                        "status": "needs_scp_reconciliation",
                    }
                ),
                encoding="utf-8",
            )
            code = spec_delta.write_payload("mobile-test", input_path, output_path)
            self.assertEqual(code, 0)
            payload, _ = spec_delta.load_payload("mobile-test", output_path)
            self.assertEqual(payload["status"], "needs_scp_reconciliation")


if __name__ == "__main__":
    unittest.main()
