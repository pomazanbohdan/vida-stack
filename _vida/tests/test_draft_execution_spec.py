from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parent.parent / "scripts" / "draft-execution-spec.py"
SPEC = importlib.util.spec_from_file_location("draft_execution_spec_test_runtime", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load draft-execution-spec helper: {SCRIPT_PATH}")
draft_execution_spec = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(draft_execution_spec)


class DraftExecutionSpecTests(unittest.TestCase):
    def test_validate_valid_payload(self) -> None:
        payload = draft_execution_spec.normalize_payload(
            "mobile-test",
            {
                "scope_in": ["settings flow"],
                "scope_out": ["unapproved navigation changes"],
                "acceptance_checks": ["settings open correctly"],
                "recommended_next_path": "/vida-form-task",
            },
        )
        ok, reason = draft_execution_spec.validate_payload(payload, "mobile-test")
        self.assertTrue(ok, reason)

    def test_validate_requires_acceptance_checks(self) -> None:
        payload = draft_execution_spec.normalize_payload(
            "mobile-test",
            {
                "scope_in": ["settings flow"],
                "recommended_next_path": "/vida-form-task",
            },
        )
        ok, reason = draft_execution_spec.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "missing_acceptance_checks")

    def test_validate_rejects_non_canonical_next_path(self) -> None:
        payload = draft_execution_spec.normalize_payload(
            "mobile-test",
            {
                "scope_in": ["settings flow"],
                "acceptance_checks": ["settings render correctly"],
                "recommended_next_path": "user_negotiation",
            },
        )
        ok, reason = draft_execution_spec.validate_payload(payload, "mobile-test")
        self.assertFalse(ok)
        self.assertEqual(reason, "invalid_recommended_next_path")

    def test_write_roundtrip(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / "input.json"
            output_path = Path(tmp) / "out.json"
            input_path.write_text(
                json.dumps(
                    {
                        "scope_in": ["settings flow"],
                        "scope_out": ["drawer"],
                        "acceptance_checks": ["settings open correctly"],
                        "recommended_next_path": "/vida-form-task",
                    }
                ),
                encoding="utf-8",
            )
            code = draft_execution_spec.write_payload("mobile-test", input_path, output_path)
            self.assertEqual(code, 0)
            payload, _ = draft_execution_spec.load_payload("mobile-test", output_path)
            self.assertEqual(payload["recommended_next_path"], "/vida-form-task")


if __name__ == "__main__":
    unittest.main()
