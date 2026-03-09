import importlib.util
import json
import io
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT_DIR / "_vida" / "scripts" / "execution-auth-gate.py"


def load_execution_auth_gate():
    spec = importlib.util.spec_from_file_location("execution_auth_gate", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ExecutionAuthGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load_execution_auth_gate()
        self.temp_dir = tempfile.TemporaryDirectory()
        self.receipt_dir = Path(self.temp_dir.name)
        self.module.ROOT_DIR = self.receipt_dir
        self.module.ROUTE_RECEIPT_DIR = self.receipt_dir
        self.task_id = "unit-task"
        self.task_class = "implementation"
        self.route = {
            "analysis_plan": {
                "required": "yes",
                "receipt_required": "yes",
            },
            "verification_plan": {
                "required": "yes",
                "selected_subagent": "gemini_cli",
            },
            "dispatch_policy": {
                "local_execution_allowed": "no",
                "required_dispatch_path": ["analysis_external_zero_budget", "analysis_receipt"],
            },
        }
        self.route_payload = {"task_class": self.task_class, "dispatch_policy": self.route["dispatch_policy"]}
        self.route_receipt_path = self.receipt_dir / f"{self.task_id}.{self.task_class}.route.json"
        self.analysis_receipt_path = self.receipt_dir / f"{self.task_id}.{self.task_class}.analysis.json"
        self.analysis_blocker_path = self.receipt_dir / f"{self.task_id}.{self.task_class}.analysis-blocker.json"
        self.issue_contract_path = self.receipt_dir / "issue-contracts" / f"{self.task_id}.json"
        self.spec_intake_path = self.receipt_dir / "spec-intake" / f"{self.task_id}.json"
        self.spec_delta_path = self.receipt_dir / "spec-deltas" / f"{self.task_id}.json"
        self.draft_execution_spec_path = self.receipt_dir / "draft-execution-specs" / f"{self.task_id}.json"

        def fake_write_route_receipt(task_id: str, task_class: str, route: dict):
            self.route_receipt_path.parent.mkdir(parents=True, exist_ok=True)
            self.route_receipt_path.write_text(json.dumps({"task_id": task_id, "task_class": task_class, "route": route}))
            return self.route_receipt_path

        self.dispatch_patches = [
            mock.patch.object(self.module.dispatch_runtime, "route_snapshot", return_value=({}, self.route)),
            mock.patch.object(self.module.dispatch_runtime, "write_route_receipt", side_effect=fake_write_route_receipt),
            mock.patch.object(self.module.dispatch_runtime, "route_receipt_payload", return_value=self.route_payload),
            mock.patch.object(
                self.module.dispatch_runtime,
                "route_receipt_hash",
                return_value=self.module.json_hash(self.route_payload),
            ),
            mock.patch.object(self.module.dispatch_runtime, "load_analysis_receipt", return_value={}),
            mock.patch.object(self.module.dispatch_runtime, "analysis_receipt_path", return_value=self.analysis_receipt_path),
            mock.patch.object(self.module.dispatch_runtime, "analysis_blocker_path", return_value=self.analysis_blocker_path),
            mock.patch.object(
                self.module.dispatch_runtime,
                "load_analysis_blocker",
                side_effect=lambda task_id, task_class: self.module.load_json(self.analysis_blocker_path),
            ),
            mock.patch.object(self.module.dispatch_runtime, "issue_contract_path", return_value=self.issue_contract_path),
            mock.patch.object(self.module.dispatch_runtime, "spec_intake_path", return_value=self.spec_intake_path),
            mock.patch.object(self.module.dispatch_runtime, "spec_delta_path", return_value=self.spec_delta_path),
            mock.patch.object(self.module.dispatch_runtime, "draft_execution_spec_path", return_value=self.draft_execution_spec_path),
            mock.patch.object(
                self.module.dispatch_runtime,
                "load_issue_contract",
                side_effect=lambda task_id: self.module.load_json(self.issue_contract_path),
            ),
            mock.patch.object(self.module.dispatch_runtime, "validate_spec_intake", return_value=(True, {}, "")),
            mock.patch.object(self.module.dispatch_runtime, "validate_spec_delta", return_value=(True, {}, "")),
            mock.patch.object(self.module.dispatch_runtime, "validate_draft_execution_spec", return_value=(True, {}, "")),
            mock.patch.object(
                self.module.dispatch_runtime,
                "validate_issue_contract",
                side_effect=lambda task_id, task_class, route: (
                    (lambda payload: (
                        bool(payload) and payload.get("status") == "writer_ready",
                        payload,
                        "" if (bool(payload) and payload.get("status") == "writer_ready") else (payload.get("status") if payload else "missing_issue_contract"),
                    ))(self.module.load_json(self.issue_contract_path))
                ),
            ),
        ]
        for patcher in self.dispatch_patches:
            patcher.start()
        self.addCleanup(self._cleanup_patches)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def _cleanup_patches(self) -> None:
        for patcher in reversed(self.dispatch_patches):
            patcher.stop()

    def _write_local_override_receipt(self) -> None:
        payload = {
            "reason": "emergency_override",
            "scope": "writer_block",
            "notes": "allow local writer for explicit override path",
            "route_receipt_hash": self.module.json_hash(self.route_payload),
        }
        self.module.write_json(self.module.local_execution_receipt_path(self.task_id, self.task_class), payload)

    def _write_structured_override_receipt(self, reason: str = "no_eligible_analysis_lane") -> None:
        payload = {
            "reason": reason,
            "notes": "framework-owned tracked remediation requires explicit execution-auth override",
            "route_receipt_hash": self.module.json_hash(self.route_payload),
        }
        self.module.write_json(self.module.override_receipt_path(self.task_id, self.task_class), payload)

    def _write_issue_metadata(self, *, labels: list[str]) -> None:
        issues_path = self.receipt_dir / ".beads" / "issues.jsonl"
        issues_path.parent.mkdir(parents=True, exist_ok=True)
        issues_path.write_text(
            json.dumps(
                {
                    "id": self.task_id,
                    "issue_type": "task",
                    "labels": labels,
                }
            )
            + "\n",
            encoding="utf-8",
        )

    def test_gate_blocks_when_analysis_receipt_and_override_are_missing(self) -> None:
        exit_code, payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 2)
        self.assertIn("missing_analysis_receipt", payload["blockers"])
        self.assertIn("missing_local_execution_receipt", payload["blockers"])

    def test_gate_accepts_analysis_failure_with_emergency_override(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "fanout_min_results_not_met",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready"}))

        exit_code, payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["authorized_via"], "local_emergency_override")
        self.assertEqual(payload["blockers"], [])

    def test_gate_blocks_when_analysis_route_is_not_ready_even_with_override(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "blocked_missing_analysis_route",
                    "reason": "framework_wave_start_main_lane",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready"}))

        exit_code, payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 2)
        self.assertIn("analysis_route_not_ready", payload["blockers"])

    def test_gate_blocks_when_issue_contract_is_missing(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "fanout_min_results_not_met",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()

        exit_code, payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 2)
        self.assertIn("missing_issue_contract", payload["blockers"])

    def test_gate_accepts_explicit_no_eligible_verifier(self) -> None:
        self.route["verification_plan"] = {
            "required": "yes",
            "selected_subagent": None,
            "reason": "no_eligible_verifier",
        }
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "fanout_min_results_not_met",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready", "proven_scope": ["x"]}))

        exit_code, payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["verification_prereq_via"], "no_eligible_verifier")
        self.assertEqual(payload["blockers"], [])

    def test_gate_accepts_framework_structured_override_for_no_eligible_analysis_lane(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "no_eligible_analysis_lane",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_issue_metadata(labels=["framework", "vida-stack"])
        self._write_structured_override_receipt()
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready", "proven_scope": ["x"]}))

        exit_code, payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["authorized_via"], "structured_unavailability_override")
        self.assertEqual(payload["analysis_prereq_via"], "analysis_blocker")
        self.assertEqual(payload["blockers"], [])

    def test_gate_blocks_non_framework_task_from_using_structured_override(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "no_eligible_analysis_lane",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_issue_metadata(labels=["product"])
        self._write_structured_override_receipt()
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready", "proven_scope": ["x"]}))

        exit_code, payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 2)
        self.assertIn("execution_auth_override_not_allowed", payload["blockers"])

    def test_authorize_skip_writes_framework_override_receipt(self) -> None:
        self._write_issue_metadata(labels=["framework", "vida-stack"])

        with mock.patch("sys.stdout", new=io.StringIO()) as stdout:
            exit_code = self.module.authorize_skip(
                [
                    "execution-auth-gate.py",
                    "authorize-skip",
                    self.task_id,
                    self.task_class,
                    "no_eligible_analysis_lane",
                    "framework tracked remediation",
                    "focused tests passed",
                    "orchestrator",
                ]
            )

        self.assertEqual(exit_code, 0)
        receipt_path = Path(stdout.getvalue().strip())
        payload = self.module.load_json(receipt_path)
        self.assertEqual(payload["reason"], "no_eligible_analysis_lane")
        self.assertEqual(payload["task_id"], self.task_id)
        self.assertTrue(payload["route_receipt_hash"])

    def test_authorize_skip_rejects_non_framework_task(self) -> None:
        self._write_issue_metadata(labels=["product"])

        with mock.patch("sys.stderr", new=io.StringIO()) as stderr:
            exit_code = self.module.authorize_skip(
                [
                    "execution-auth-gate.py",
                    "authorize-skip",
                    self.task_id,
                    self.task_class,
                    "no_eligible_analysis_lane",
                    "should fail",
                ]
            )

        self.assertEqual(exit_code, 1)
        self.assertIn("framework-labeled tasks", stderr.getvalue())

    def test_gate_blocks_stale_structured_override_receipt(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "no_eligible_analysis_lane",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_issue_metadata(labels=["framework", "vida-stack"])
        payload = {
            "reason": "no_eligible_analysis_lane",
            "notes": "framework-owned tracked remediation requires explicit execution-auth override",
            "route_receipt_hash": "stale-hash",
        }
        self.module.write_json(self.module.override_receipt_path(self.task_id, self.task_class), payload)
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready", "proven_scope": ["x"]}))

        exit_code, gate_payload = self.module.check_gate(
            self.task_id,
            self.task_class,
            local_write=True,
            block_id="P02",
        )

        self.assertEqual(exit_code, 2)
        self.assertIn("stale_execution_auth_override", gate_payload["blockers"])

    def test_gate_blocks_when_issue_contract_has_no_proven_scope(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "fanout_min_results_not_met",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready", "proven_scope": []}))

        with mock.patch.object(
            self.module.dispatch_runtime,
            "validate_issue_contract",
            return_value=(False, {"status": "writer_ready", "proven_scope": []}, "missing_proven_scope"),
        ):
            exit_code, payload = self.module.check_gate(
                self.task_id,
                self.task_class,
                local_write=True,
                block_id="P02",
            )

        self.assertEqual(exit_code, 2)
        self.assertIn("missing_proven_scope", payload["blockers"])

    def test_gate_blocks_when_spec_delta_is_open(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "fanout_min_results_not_met",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()
        self.issue_contract_path.parent.mkdir(parents=True, exist_ok=True)
        self.issue_contract_path.write_text(json.dumps({"status": "writer_ready", "proven_scope": ["x"]}))

        with mock.patch.object(
            self.module.dispatch_runtime,
            "validate_spec_delta",
            return_value=(False, {"status": "needs_scp_reconciliation"}, "spec_delta_needs_scp_reconciliation"),
        ):
            exit_code, payload = self.module.check_gate(
                self.task_id,
                self.task_class,
                local_write=True,
                block_id="P02",
            )

        self.assertEqual(exit_code, 2)
        self.assertIn("spec_delta_needs_scp_reconciliation", payload["blockers"])

    def test_gate_allows_spec_driven_path_without_issue_contract(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "fanout_min_results_not_met",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()

        with mock.patch.object(
            self.module.dispatch_runtime,
            "validate_draft_execution_spec",
            return_value=(
                True,
                {
                    "task_id": self.task_id,
                    "scope_in": ["settings flow"],
                    "acceptance_checks": ["settings render correctly"],
                    "recommended_next_path": "/vida-form-task",
                },
                "",
            ),
        ):
            exit_code, payload = self.module.check_gate(
                self.task_id,
                self.task_class,
                local_write=True,
                block_id="P02",
            )

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertFalse(payload["issue_contract_required"])
        self.assertEqual(payload["blockers"], [])
        self.assertTrue(payload["draft_execution_spec_present"])

    def test_gate_allows_non_issue_driven_task_without_issue_contract(self) -> None:
        self.analysis_blocker_path.write_text(
            json.dumps(
                {
                    "status": "analysis_failed",
                    "reason": "fanout_min_results_not_met",
                    "route_receipt_hash": self.module.json_hash(self.route_payload),
                }
            )
        )
        self._write_local_override_receipt()

        with mock.patch.object(self.module, "task_is_issue_driven", return_value=False):
            exit_code, payload = self.module.check_gate(
                self.task_id,
                self.task_class,
                local_write=True,
                block_id="P02",
            )

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertFalse(payload["issue_contract_required"])
        self.assertEqual(payload["analysis_prereq_via"], "analysis_blocker")


if __name__ == "__main__":
    unittest.main()
