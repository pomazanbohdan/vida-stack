from __future__ import annotations

import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parent.parent / "scripts" / "run-graph.py"
SPEC = importlib.util.spec_from_file_location("run_graph_test_runtime", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load run-graph helper: {SCRIPT_PATH}")
run_graph = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(run_graph)


class RunGraphTests(unittest.TestCase):
    def test_update_node_creates_graph_and_tracks_attempts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            original_env = os.environ.get(run_graph.STATE_DIR_ENV)
            os.environ[run_graph.STATE_DIR_ENV] = tmp
            try:
                path = run_graph.update_node(
                    "mobile-test",
                    "implementation",
                    "analysis",
                    "running",
                    route_task_class="analysis",
                    meta={"manifest_path": "x.json"},
                )
                payload = json.loads(path.read_text(encoding="utf-8"))
                self.assertEqual(payload["task_id"], "mobile-test")
                self.assertEqual(payload["nodes"]["analysis"]["status"], "running")
                self.assertEqual(payload["nodes"]["analysis"]["attempts"], 1)
                self.assertEqual(payload["nodes"]["analysis"]["meta"]["manifest_path"], "x.json")
            finally:
                if original_env is None:
                    os.environ.pop(run_graph.STATE_DIR_ENV, None)
                else:
                    os.environ[run_graph.STATE_DIR_ENV] = original_env

    def test_status_reports_missing_graph(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            original_env = os.environ.get(run_graph.STATE_DIR_ENV)
            os.environ[run_graph.STATE_DIR_ENV] = tmp
            try:
                payload = run_graph.status_payload("missing-task")
                self.assertFalse(payload["present"])
            finally:
                if original_env is None:
                    os.environ.pop(run_graph.STATE_DIR_ENV, None)
                else:
                    os.environ[run_graph.STATE_DIR_ENV] = original_env

    def test_status_payload_includes_resume_hint(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            original_env = os.environ.get(run_graph.STATE_DIR_ENV)
            os.environ[run_graph.STATE_DIR_ENV] = tmp
            try:
                run_graph.update_node(
                    "mobile-test",
                    "implementation",
                    "writer",
                    "blocked",
                    route_task_class="implementation",
                    meta={"reason": "issue_contract_blocked"},
                )
                payload = run_graph.status_payload("mobile-test")
                self.assertTrue(payload["present"])
                self.assertEqual(payload["resume_hint"]["next_node"], "writer")
                self.assertEqual(payload["resume_hint"]["status"], "blocked")
            finally:
                if original_env is None:
                    os.environ.pop(run_graph.STATE_DIR_ENV, None)
                else:
                    os.environ[run_graph.STATE_DIR_ENV] = original_env

    def test_problem_party_is_supported_run_graph_node(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            original_env = os.environ.get(run_graph.STATE_DIR_ENV)
            os.environ[run_graph.STATE_DIR_ENV] = tmp
            try:
                path = run_graph.update_node(
                    "mobile-test",
                    "architecture",
                    "problem_party",
                    "completed",
                    route_task_class="problem_party",
                    meta={"receipt_path": "party.json"},
                )
                payload = json.loads(path.read_text(encoding="utf-8"))
                self.assertEqual(payload["nodes"]["problem_party"]["status"], "completed")
                self.assertEqual(payload["nodes"]["problem_party"]["meta"]["receipt_path"], "party.json")
            finally:
                if original_env is None:
                    os.environ.pop(run_graph.STATE_DIR_ENV, None)
                else:
                    os.environ[run_graph.STATE_DIR_ENV] = original_env


if __name__ == "__main__":
    unittest.main()
