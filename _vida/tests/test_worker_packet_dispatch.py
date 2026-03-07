import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
DISPATCH_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-dispatch.py"
WORKER_PACKET_GATE_PATH = ROOT_DIR / "_vida" / "scripts" / "worker-packet-gate.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class WorkerPacketDispatchTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.dispatch = load_module("worker_packet_dispatch_test", DISPATCH_PATH)
        cls.packet_gate = load_module("worker_packet_gate_for_dispatch_test", WORKER_PACKET_GATE_PATH)

    def minimal_route(self) -> dict:
        return {
            "task_class": "implementation",
            "dispatch_policy": {
                "direct_internal_bypass_forbidden": "no",
                "internal_escalation_allowed": "no",
                "internal_route_authorized": "no",
            },
            "analysis_plan": {
                "required": "no",
                "receipt_required": "no",
                "route_task_class": "",
                "fanout_subagents": [],
            },
            "route_budget": {},
            "fallback_subagents": [],
        }

    def minimal_subagent_cfg(self) -> dict:
        return {
            "dispatch": {
                "command": "echo",
                "output_mode": "stdout",
                "prompt_mode": "positional",
            },
            "billing_tier": "free",
            "quality_tier": "high",
            "speed_tier": "fast",
            "specialties": ["review"],
            "write_scope": "none",
        }

    def test_subagent_command_inserts_web_search_flag_before_subcommand(self) -> None:
        route = {"web_search_required": "yes"}
        subagent_cfg = {
            "dispatch": {
                "command": "codex",
                "pre_static_args": ["-a", "never"],
                "subcommand": "exec",
                "static_args": ["--ephemeral", "-s", "read-only"],
                "web_search_mode": "flag",
                "web_search_flag": "--search",
                "output_mode": "stdout",
                "prompt_mode": "positional",
            }
        }

        command, use_stdout = self.dispatch.subagent_command(
            "codex_cli",
            "research prompt",
            Path("/tmp/out.txt"),
            ROOT_DIR,
            "gpt-5.1-codex-mini",
            subagent_cfg,
            route,
        )

        self.assertTrue(use_stdout)
        self.assertEqual(
            command[:8],
            ["codex", "-a", "never", "--search", "exec", "--ephemeral", "-s", "read-only"],
        )

    def test_subagent_command_does_not_force_web_search_for_provider_configured_lane(self) -> None:
        route = {"web_search_required": "yes"}
        subagent_cfg = {
            "dispatch": {
                "command": "qwen",
                "static_args": ["-y", "-o", "text"],
                "web_search_mode": "provider_configured",
                "output_mode": "stdout",
                "prompt_mode": "positional",
            }
        }

        command, use_stdout = self.dispatch.subagent_command(
            "qwen_cli",
            "research prompt",
            Path("/tmp/out.txt"),
            ROOT_DIR,
            None,
            subagent_cfg,
            route,
        )

        self.assertTrue(use_stdout)
        self.assertEqual(command[:4], ["qwen", "-y", "-o", "text"])
        self.assertNotIn("--search", command)

    def test_subagent_command_uses_write_static_args_for_write_routes(self) -> None:
        route = {"write_scope": "scoped_only"}
        subagent_cfg = {
            "write_scope": "scoped_only",
            "dispatch": {
                "command": "codex",
                "subcommand": "exec",
                "static_args": ["--ephemeral", "-s", "read-only"],
                "write_static_args": ["--ephemeral", "-s", "workspace-write"],
                "workdir_flag": "-C",
                "model_flag": "-m",
                "output_mode": "file",
                "output_flag": "-o",
                "prompt_mode": "positional",
            }
        }

        command, use_stdout = self.dispatch.subagent_command(
            "codex_cli",
            "implementation prompt",
            Path("/tmp/out.txt"),
            ROOT_DIR,
            "gpt-5.1-codex",
            subagent_cfg,
            route,
        )

        self.assertFalse(use_stdout)
        self.assertEqual(
            command[:6],
            ["codex", "exec", "--ephemeral", "-s", "workspace-write", "-C"],
        )
        self.assertEqual(command[6:10], [str(ROOT_DIR), "-m", "gpt-5.1-codex", "-o"])

    def test_start_subagent_process_blocks_invalid_worker_packet_before_dispatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            prompt_file = tmp_path / "invalid.prompt.txt"
            output_file = tmp_path / "invalid.output.txt"
            prompt_file.write_text(
                "Task: implementation\nScope: _vida/scripts\nVerification:\n- python3 -m unittest\nDeliverable:\n- summary\n",
                encoding="utf-8",
            )

            with mock.patch.object(self.dispatch.subprocess, "Popen") as mocked_popen:
                launch = self.dispatch.start_subagent_process(
                    task_id="unit-task",
                    task_class="implementation",
                    subagent_name="qwen_cli",
                    prompt_file=prompt_file,
                    output_file=output_file,
                    workdir=ROOT_DIR,
                    route=self.minimal_route(),
                    subagent_cfg=self.minimal_subagent_cfg(),
                    dispatch_mode="single",
                )

            mocked_popen.assert_not_called()
            self.assertIn("result", launch)
            self.assertEqual(launch["result"]["status"], "failure")
            self.assertIn("worker packet validation failed", launch["result"]["error"])
            self.assertIn("missing worker_lane_confirmed marker", launch["result"]["worker_packet_errors"])
            self.assertFalse(launch["result"]["worker_output_valid"])
            self.assertFalse(launch["result"]["useful_progress"])

    def test_framework_mutation_paths_detect_new_framework_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            (tmp_path / "AGENTS.md").write_text("bootstrap\n", encoding="utf-8")
            docs_dir = tmp_path / "_vida" / "docs"
            docs_dir.mkdir(parents=True, exist_ok=True)
            (docs_dir / "protocol.md").write_text("protocol\n", encoding="utf-8")
            before = self.dispatch.framework_mutation_snapshot(tmp_path)

            (docs_dir / "mobile-1ic.1-analysis.json").write_text("{}", encoding="utf-8")
            pycache_dir = tmp_path / "_vida" / "scripts" / "__pycache__"
            pycache_dir.mkdir(parents=True, exist_ok=True)
            (pycache_dir / "ignored.pyc").write_bytes(b"pyc")

            after = self.dispatch.framework_mutation_snapshot(tmp_path)
            changed = self.dispatch.framework_mutation_paths(before, after)

            self.assertEqual(changed, ["_vida/docs/mobile-1ic.1-analysis.json"])

    def test_project_mutation_paths_detect_new_project_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            src_dir = tmp_path / "src" / "lib"
            src_dir.mkdir(parents=True, exist_ok=True)
            (src_dir / "existing.dart").write_text("class Existing {}\n", encoding="utf-8")
            before = self.dispatch.project_mutation_snapshot(tmp_path)

            (src_dir / "leak.dart").write_text("class Leak {}\n", encoding="utf-8")
            after = self.dispatch.project_mutation_snapshot(tmp_path)
            changed = self.dispatch.project_mutation_paths(before, after)

            self.assertEqual(changed, ["src/lib/leak.dart"])

    def test_project_mutation_paths_detect_new_custom_project_root_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            before = self.dispatch.project_mutation_snapshot(tmp_path)

            assets_dir = tmp_path / "assets" / "icons"
            assets_dir.mkdir(parents=True, exist_ok=True)
            (assets_dir / "logo.svg").write_text("<svg />\n", encoding="utf-8")

            after = self.dispatch.project_mutation_snapshot(tmp_path)
            changed = self.dispatch.project_mutation_paths(before, after)

            self.assertEqual(changed, ["assets/icons/logo.svg"])

    def test_start_subagent_process_snapshots_read_only_lane_even_for_write_capable_subagent(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            src_dir = tmp_path / "src" / "lib"
            src_dir.mkdir(parents=True, exist_ok=True)
            (src_dir / "existing.dart").write_text("class Existing {}\n", encoding="utf-8")
            prompt_file = tmp_path / "analysis.prompt.txt"
            output_file = tmp_path / "analysis.output.txt"
            prompt_file.write_text(
                self.dispatch.analysis_prompt_text(
                    "Investigate task",
                    "implementation",
                    "analysis",
                ),
                encoding="utf-8",
            )

            subagent_cfg = self.minimal_subagent_cfg()
            subagent_cfg["write_scope"] = "scoped_only"
            route = {
                **self.minimal_route(),
                "task_class": "analysis",
                "write_scope": "none",
            }
            process = mock.Mock()

            with mock.patch.object(self.dispatch, "ROOT_DIR", tmp_path), mock.patch.object(
                self.dispatch.subprocess,
                "Popen",
                return_value=process,
            ):
                launch = self.dispatch.start_subagent_process(
                    task_id="unit-task",
                    task_class="analysis",
                    subagent_name="codex_cli",
                    prompt_file=prompt_file,
                    output_file=output_file,
                    workdir=tmp_path,
                    route=route,
                    subagent_cfg=subagent_cfg,
                    dispatch_mode="single",
                )

            self.assertIn("framework_snapshot_before", launch)
            self.assertIsInstance(launch["framework_snapshot_before"], dict)
            self.assertIn("project_snapshot_before", launch)
            self.assertIsInstance(launch["project_snapshot_before"], dict)

    def test_read_only_prep_task_class_is_treated_as_read_only_boundary(self) -> None:
        required = self.dispatch.read_only_boundary_guard_required(
            "Task packet without explicit mode marker",
            task_class="read_only_prep",
            route={"task_class": "read_only_prep"},
            subagent_cfg={"write_scope": "scoped_only"},
        )

        self.assertTrue(required)

    def test_finalize_subagent_process_fails_when_read_only_lane_mutates_framework_tree(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            log_dir = tmp_path / ".vida" / "logs"
            run_log_path = log_dir / "subagent-runs.jsonl"
            (tmp_path / "AGENTS.md").write_text("bootstrap\n", encoding="utf-8")
            (tmp_path / "_vida" / "docs").mkdir(parents=True, exist_ok=True)
            (tmp_path / "_vida" / "docs" / "protocol.md").write_text("protocol\n", encoding="utf-8")

            prompt_file = tmp_path / "analysis.prompt.txt"
            output_file = tmp_path / "analysis.output.txt"
            stderr_path = tmp_path / "analysis.output.txt.stderr.log"
            prompt_file.write_text(
                self.dispatch.analysis_prompt_text(
                    "Investigate task",
                    "implementation",
                    "analysis",
                ),
                encoding="utf-8",
            )
            output_file.write_text(
                json.dumps(
                    {
                        "status": "done",
                        "question_answered": "yes",
                        "answer": "bounded answer",
                        "evidence_refs": ["src/file.dart:1"],
                        "changed_files": [],
                        "verification_commands": [],
                        "verification_results": [],
                        "merge_ready": "yes",
                        "blockers": [],
                        "notes": "",
                        "recommended_next_action": "proceed_to_writer",
                        "impact_analysis": {
                            "affected_scope": ["scope"],
                            "contract_impact": [],
                            "follow_up_actions": [],
                            "residual_risks": [],
                        },
                        "issue_contract": {
                            "classification": "defect_equivalent",
                            "equivalence_assessment": "equivalent_fix",
                            "reported_behavior": "broken",
                            "expected_behavior": "fixed",
                            "scope_in": ["scope"],
                            "scope_out": [],
                            "acceptance_checks": ["ac"],
                            "spec_sync_targets": [],
                            "wvp_required": "no",
                            "wvp_status": "not_required",
                        },
                    }
                ),
                encoding="utf-8",
            )
            stderr_path.write_text("", encoding="utf-8")

            before_snapshot = self.dispatch.framework_mutation_snapshot(tmp_path)
            (tmp_path / "_vida" / "docs" / "mobile-1ic.1-analysis.json").write_text(
                "{}",
                encoding="utf-8",
            )

            process = mock.Mock()
            process.poll.return_value = 0
            process.returncode = 0
            process.wait.return_value = 0

            launch = {
                "process": process,
                "stdout_handle": None,
                "stderr_handle": None,
                "task_id": "mobile-1ic.1",
                "task_class": "analysis",
                "subagent_name": "qwen_cli",
                "prompt_file": prompt_file,
                "output_file": output_file,
                "stderr_path": stderr_path,
                "workdir": tmp_path,
                "route": {
                    "task_class": "analysis",
                    "dispatch_policy": {
                        "direct_internal_bypass_forbidden": "no",
                        "internal_escalation_allowed": "no",
                        "internal_route_authorized": "no",
                    },
                    "analysis_plan": {"required": "no", "receipt_required": "no", "route_task_class": ""},
                    "route_budget": {},
                    "fallback_subagents": [],
                },
                "subagent_cfg": self.minimal_subagent_cfg(),
                "dispatch_mode": "single",
                "selected_model": None,
                "domain_tags": ["vida_framework"],
                "risk_class": "R1",
                "max_runtime_seconds": 60,
                "min_output_bytes": 50,
                "run_id": "spr-test",
                "ts_start": "2026-03-07T00:00:00Z",
                "started": 0.0,
                "framework_snapshot_before": before_snapshot,
            }

            with mock.patch.object(self.dispatch, "ROOT_DIR", tmp_path), mock.patch.object(
                self.dispatch,
                "LOG_DIR",
                log_dir,
            ), mock.patch.object(self.dispatch, "RUN_LOG_PATH", run_log_path), mock.patch.object(
                self.dispatch.time,
                "monotonic",
                return_value=1.0,
            ):
                payload = self.dispatch.finalize_subagent_process(launch)

            self.assertEqual(payload["status"], "failure")
            self.assertTrue(payload["framework_boundary_violation"])
            self.assertIn("_vida/docs/mobile-1ic.1-analysis.json", payload["framework_boundary_violation_paths"])
            self.assertIn("forbidden framework mutation detected", payload["error"])

    def test_finalize_subagent_process_fails_when_read_only_lane_mutates_project_tree(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            log_dir = tmp_path / ".vida" / "logs"
            run_log_path = log_dir / "subagent-runs.jsonl"
            src_dir = tmp_path / "src" / "lib"
            src_dir.mkdir(parents=True, exist_ok=True)
            (src_dir / "existing.dart").write_text("class Existing {}\n", encoding="utf-8")

            prompt_file = tmp_path / "analysis.prompt.txt"
            output_file = tmp_path / "analysis.output.txt"
            stderr_path = tmp_path / "analysis.output.txt.stderr.log"
            prompt_file.write_text(
                self.dispatch.analysis_prompt_text(
                    "Investigate task",
                    "implementation",
                    "analysis",
                ),
                encoding="utf-8",
            )
            output_file.write_text(
                json.dumps(
                    {
                        "status": "done",
                        "question_answered": "yes",
                        "answer": "bounded answer",
                        "evidence_refs": ["src/lib/existing.dart:1"],
                        "changed_files": [],
                        "verification_commands": [],
                        "verification_results": [],
                        "merge_ready": "yes",
                        "blockers": [],
                        "notes": "",
                        "recommended_next_action": "proceed_to_writer",
                        "impact_analysis": {
                            "affected_scope": ["scope"],
                            "contract_impact": [],
                            "follow_up_actions": [],
                            "residual_risks": [],
                        },
                        "issue_contract": {
                            "classification": "defect_equivalent",
                            "equivalence_assessment": "equivalent_fix",
                            "reported_behavior": "broken",
                            "expected_behavior": "fixed",
                            "scope_in": ["scope"],
                            "scope_out": [],
                            "acceptance_checks": ["ac"],
                            "spec_sync_targets": [],
                            "wvp_required": "no",
                            "wvp_status": "not_required",
                        },
                    }
                ),
                encoding="utf-8",
            )
            stderr_path.write_text("", encoding="utf-8")

            before_snapshot = self.dispatch.project_mutation_snapshot(tmp_path)
            (src_dir / "leak.dart").write_text("class Leak {}\n", encoding="utf-8")

            process = mock.Mock()
            process.poll.return_value = 0
            process.returncode = 0
            process.wait.return_value = 0

            launch = {
                "process": process,
                "stdout_handle": None,
                "stderr_handle": None,
                "task_id": "mobile-1ic.2",
                "task_class": "analysis",
                "subagent_name": "qwen_cli",
                "prompt_file": prompt_file,
                "output_file": output_file,
                "stderr_path": stderr_path,
                "workdir": tmp_path,
                "route": {
                    "task_class": "analysis",
                    "dispatch_policy": {
                        "direct_internal_bypass_forbidden": "no",
                        "internal_escalation_allowed": "no",
                        "internal_route_authorized": "no",
                    },
                    "analysis_plan": {"required": "no", "receipt_required": "no", "route_task_class": ""},
                    "route_budget": {},
                    "fallback_subagents": [],
                },
                "subagent_cfg": self.minimal_subagent_cfg(),
                "dispatch_mode": "single",
                "selected_model": None,
                "domain_tags": ["vida_framework"],
                "risk_class": "R1",
                "max_runtime_seconds": 60,
                "min_output_bytes": 50,
                "run_id": "spr-test",
                "ts_start": "2026-03-07T00:00:00Z",
                "started": 0.0,
                "framework_snapshot_before": None,
                "project_snapshot_before": before_snapshot,
            }

            with mock.patch.object(self.dispatch, "ROOT_DIR", tmp_path), mock.patch.object(
                self.dispatch,
                "LOG_DIR",
                log_dir,
            ), mock.patch.object(self.dispatch, "RUN_LOG_PATH", run_log_path), mock.patch.object(
                self.dispatch.time,
                "monotonic",
                return_value=1.0,
            ):
                payload = self.dispatch.finalize_subagent_process(launch)

            self.assertEqual(payload["status"], "failure")
            self.assertTrue(payload["project_boundary_violation"])
            self.assertIn("src/lib/leak.dart", payload["project_boundary_violation_paths"])
            self.assertIn("forbidden project mutation detected", payload["error"])

    def test_machine_readable_write_output_is_not_merge_ready_when_contract_is_invalid(self) -> None:
        prompt = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: implement packet gate
Scope: _vida/scripts
Blocking Question: What changed?
Verification:
- python3 -m unittest
Deliverable:
- Return the machine-readable summary below.
```json
{
  "status": "done",
  "question_answered": "yes",
  "answer": "x",
  "evidence_refs": [],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
```
"""
        old_style_but_invalid_output = """
## Findings
- evidence: updated _vida/scripts/subagent-dispatch.py
- file: _vida/scripts/subagent-dispatch.py
- recommended: integrate
"""

        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            prompt_file = tmp_path / "implementation.prompt.txt"
            output_file = tmp_path / "implementation.output.txt"
            stderr_file = tmp_path / "implementation.output.txt.stderr.log"
            prompt_file.write_text(prompt, encoding="utf-8")
            output_file.write_text(old_style_but_invalid_output, encoding="utf-8")
            stderr_file.write_text("", encoding="utf-8")

            with mock.patch.object(self.dispatch, "append_jsonl"):
                payload = self.dispatch.subagent_result_payload(
                    task_id="unit-task",
                    task_class="implementation",
                    subagent_name="qwen_cli",
                    selected_model=None,
                    subagent_cfg=self.minimal_subagent_cfg(),
                    dispatch_mode="single",
                    risk_class="R1",
                    domain_tags=["vida_framework"],
                    max_runtime_seconds=60,
                    min_output_bytes=1,
                    output_file=output_file,
                    stderr_path=stderr_file,
                    workdir=ROOT_DIR,
                    prompt_file=prompt_file,
                    route=self.minimal_route(),
                    run_id="spr-test",
                    ts_start="2026-03-07T00:00:00Z",
                    started=0.0,
                    status="success",
                    exit_code=0,
                    error_text="",
                )

        self.assertTrue(payload["machine_readable_contract_required"])
        self.assertFalse(payload["worker_output_valid"])
        self.assertFalse(payload["merge_ready"])
        self.assertIn(
            "worker output must be valid JSON when the prompt requires a machine-readable summary",
            payload["worker_output_errors"],
        )

    def test_machine_readable_output_respects_worker_merge_ready_no(self) -> None:
        prompt = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: implement packet gate
Scope: _vida/scripts
Blocking Question: What changed?
Verification:
- python3 -m unittest
Deliverable:
- Return the machine-readable summary below.
```json
{
  "status": "done",
  "question_answered": "yes",
  "answer": "x",
  "evidence_refs": [],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
```
"""
        valid_but_not_ready_output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implemented validator",
  "evidence_refs": ["_vida/scripts/file.py:10"],
  "changed_files": ["_vida/scripts/file.py"],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> pass"],
  "merge_ready": "no",
  "blockers": ["follow-up review pending"],
  "notes": "",
  "recommended_next_action": "run another review pass",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/file.py"],
    "contract_impact": ["worker packet gate"],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
"""

        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            prompt_file = tmp_path / "implementation.prompt.txt"
            output_file = tmp_path / "implementation.output.txt"
            stderr_file = tmp_path / "implementation.output.txt.stderr.log"
            prompt_file.write_text(prompt, encoding="utf-8")
            output_file.write_text(valid_but_not_ready_output, encoding="utf-8")
            stderr_file.write_text("", encoding="utf-8")

            with mock.patch.object(self.dispatch, "append_jsonl"):
                payload = self.dispatch.subagent_result_payload(
                    task_id="unit-task",
                    task_class="implementation",
                    subagent_name="qwen_cli",
                    selected_model=None,
                    subagent_cfg=self.minimal_subagent_cfg(),
                    dispatch_mode="single",
                    risk_class="R1",
                    domain_tags=["vida_framework"],
                    max_runtime_seconds=60,
                    min_output_bytes=1,
                    output_file=output_file,
                    stderr_path=stderr_file,
                    workdir=ROOT_DIR,
                    prompt_file=prompt_file,
                    route=self.minimal_route(),
                    run_id="spr-test",
                    ts_start="2026-03-07T00:00:00Z",
                    started=0.0,
                    status="success",
                    exit_code=0,
                    error_text="",
                )

        self.assertTrue(payload["worker_output_valid"])
        self.assertFalse(payload["merge_ready"])

    def test_non_machine_readable_chatter_output_is_not_counted_as_valid_progress(self) -> None:
        prompt = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: audit prompt flow
Scope: _vida/scripts
Blocking Question: What broke?
Verification:
- rg -n "worker_lane_confirmed" _vida/scripts/render-subagent-prompt.sh
Deliverable:
- Bullet list: findings, risks, recommended fixes.
"""
        chatter_output = "I will begin by reading the relevant files and then report back."

        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            prompt_file = tmp_path / "audit.prompt.txt"
            output_file = tmp_path / "audit.output.txt"
            stderr_file = tmp_path / "audit.output.txt.stderr.log"
            prompt_file.write_text(prompt, encoding="utf-8")
            output_file.write_text(chatter_output, encoding="utf-8")
            stderr_file.write_text("", encoding="utf-8")

            with mock.patch.object(self.dispatch, "append_jsonl"):
                payload = self.dispatch.subagent_result_payload(
                    task_id="unit-task",
                    task_class="review",
                    subagent_name="qwen_cli",
                    selected_model=None,
                    subagent_cfg=self.minimal_subagent_cfg(),
                    dispatch_mode="single",
                    risk_class="R1",
                    domain_tags=["vida_framework"],
                    max_runtime_seconds=60,
                    min_output_bytes=1,
                    output_file=output_file,
                    stderr_path=stderr_file,
                    workdir=ROOT_DIR,
                    prompt_file=prompt_file,
                    route=self.minimal_route(),
                    run_id="spr-test",
                    ts_start="2026-03-07T00:00:00Z",
                    started=0.0,
                    status="success",
                    exit_code=0,
                    error_text="",
                )

        self.assertFalse(payload["machine_readable_contract_required"])
        self.assertEqual(payload["status"], "failure")
        self.assertFalse(payload["worker_output_valid"])
        self.assertFalse(payload["useful_progress"])
        self.assertFalse(payload["merge_ready"])
        self.assertTrue(payload["chatter_only"])
        self.assertEqual(payload["output_quality_state"], "preamble_only_output")
        self.assertEqual(payload["failure_reason"], "preamble_only_output")

    def test_machine_readable_prompt_without_json_payload_fails_closed(self) -> None:
        prompt = self.dispatch.analysis_prompt_text(
            original_prompt="Task: classify issue contract",
            writer_task_class="implementation",
            analysis_task_class="analysis",
        )
        prose_only_output = "I reviewed the task and will prepare the normalized issue contract next."

        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            prompt_file = tmp_path / "analysis.prompt.txt"
            output_file = tmp_path / "analysis.output.txt"
            stderr_file = tmp_path / "analysis.output.txt.stderr.log"
            prompt_file.write_text(prompt, encoding="utf-8")
            output_file.write_text(prose_only_output, encoding="utf-8")
            stderr_file.write_text("", encoding="utf-8")

            with mock.patch.object(self.dispatch, "append_jsonl"):
                payload = self.dispatch.subagent_result_payload(
                    task_id="unit-task",
                    task_class="analysis",
                    subagent_name="qwen_cli",
                    selected_model=None,
                    subagent_cfg=self.minimal_subagent_cfg(),
                    dispatch_mode="single",
                    risk_class="R1",
                    domain_tags=["vida_framework"],
                    max_runtime_seconds=60,
                    min_output_bytes=1,
                    output_file=output_file,
                    stderr_path=stderr_file,
                    workdir=ROOT_DIR,
                    prompt_file=prompt_file,
                    route=self.minimal_route(),
                    run_id="spr-test",
                    ts_start="2026-03-07T00:00:00Z",
                    started=0.0,
                    status="success",
                    exit_code=0,
                    error_text="",
                )

        self.assertEqual(payload["status"], "failure")
        self.assertTrue(payload["machine_readable_contract_required"])
        self.assertFalse(payload["worker_output_valid"])
        self.assertFalse(payload["useful_progress"])
        self.assertEqual(payload["output_quality_state"], "missing_machine_readable_payload")
        self.assertEqual(payload["failure_reason"], "missing_machine_readable_payload")

    def test_arbitration_prompt_text_passes_worker_packet_gate(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            output_file = tmp_path / "qwen.output.txt"
            output_file.write_text("cluster A is correct because the evidence aligns.", encoding="utf-8")

            prompt = self.dispatch.arbitration_prompt_text(
                original_prompt="Task: review route behavior",
                task_class="review",
                merge_summary={
                    "open_conflicts": [
                        {
                            "cluster_id": "abc123",
                            "subagents": ["qwen_cli"],
                            "sample": "route requires external analysis receipt",
                        }
                    ]
                },
                results=[
                    {
                        "subagent": "qwen_cli",
                        "status": "success",
                        "output_file": str(output_file),
                    }
                ],
            )

        self.assertEqual(self.packet_gate.validate_packet_text(prompt), [])

    def test_verification_prompt_text_passes_worker_packet_gate(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            output_file = tmp_path / "qwen.output.txt"
            output_file.write_text("Validated candidate synthesis with direct evidence refs.", encoding="utf-8")

            prompt = self.dispatch.verification_prompt_text(
                original_prompt="Task: verify routed implementation",
                task_class="implementation",
                verification_task_class="verification",
                merge_summary={},
                post_arbitration_merge_summary={
                    "consensus_mode": "semantic_majority",
                    "decision_ready": True,
                    "dominant_finding": {
                        "cluster_id": "def456",
                        "sample": "worker packet gate should block invalid output contracts",
                    },
                    "success_subagents": ["qwen_cli"],
                    "open_conflicts": [],
                },
                results=[
                    {
                        "subagent": "qwen_cli",
                        "status": "success",
                        "output_file": str(output_file),
                    }
                ],
            )

        self.assertEqual(self.packet_gate.validate_packet_text(prompt), [])

    def test_coach_prompt_text_passes_worker_packet_gate(self) -> None:
        prompt = self.dispatch.coach_prompt_text(
            original_prompt="Implement worker packet gate for write routes.",
            writer_task_class="implementation",
            coach_task_class="coach",
            coach_total=2,
        )

        self.assertEqual(self.packet_gate.validate_packet_text(prompt), [])
        self.assertIn('"merge_ready": "yes"', prompt)
        self.assertIn(
            "Treat readiness for independent verification as an individual coach verdict from your lane;",
            prompt,
        )
        self.assertIn(
            "Do not use pending parallel coach lanes as a blocker or as a reason to set `merge_ready=no`.",
            prompt,
        )
        self.assertIn("Use EXACTLY one of these two final states:", prompt)
        self.assertIn(
            "If a local tool is unavailable in your environment, record that in `verification_results` or `notes`, not in `blockers`, unless the missing tool proves a concrete implementation gap.",
            prompt,
        )

    def test_analysis_prompt_text_passes_worker_packet_gate(self) -> None:
        prompt = self.dispatch.analysis_prompt_text(
            original_prompt="Stabilize Odoo 19 API error propagation and dashboard unread fallbacks.",
            writer_task_class="implementation",
            analysis_task_class="analysis",
        )

        self.assertEqual(self.packet_gate.validate_packet_text(prompt), [])
        self.assertIn(
            "For this read-only analysis lane, `merge_ready=yes` means the analysis artifact itself is complete enough for orchestrator synthesis and writer routing;",
            prompt,
        )
        self.assertIn(
            "set `merge_ready=yes`; use `merge_ready=no` only when the analysis artifact is still incomplete or ambiguous.",
            prompt,
        )

    def test_writer_prompt_text_passes_worker_packet_gate(self) -> None:
        prompt = self.dispatch.writer_prompt_text(
            original_prompt="Implement the validated bugfix scope.",
            writer_task_class="implementation",
            issue_contract={
                "classification": "defect_equivalent",
                "equivalence_assessment": "equivalent_fix",
                "reported_behavior": "server errors degrade into false network errors",
                "expected_behavior": "server error context remains available",
                "scope_in": ["error interceptor stack"],
                "scope_out": ["drawer navigation"],
                "acceptance_checks": ["Errors retain actionable details"],
                "spec_sync_targets": ["docs/specs/api.md"],
                "wvp_required": "no",
                "wvp_status": "not_required",
            },
        )

        self.assertEqual(self.packet_gate.validate_packet_text(prompt), [])
        self.assertIn("Normalized issue contract:", prompt)
        self.assertIn("server errors degrade into false network errors", prompt)

    def test_prepare_execution_renders_analysis_worker_prompt_before_ensemble(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path
            try:
                prompt_file = tmp_path / "implementation.prompt.txt"
                prompt_file.write_text("Fix the validated bug scope for mobile-1ic.1.\n", encoding="utf-8")
                analysis_output = tmp_path / "analysis.output.json"
                analysis_output.write_text(
                    json.dumps(
                        {
                            "status": "done",
                            "question_answered": "yes",
                            "answer": "equivalent defect fix confirmed",
                            "evidence_refs": ["lib/src/issue.dart:10"],
                            "changed_files": [],
                            "verification_commands": [],
                            "verification_results": [],
                            "merge_ready": "yes",
                            "blockers": [],
                            "notes": "",
                            "recommended_next_action": "proceed_to_writer",
                            "impact_analysis": {
                                "affected_scope": ["lib/src/issue.dart"],
                                "contract_impact": [],
                                "follow_up_actions": [],
                                "residual_risks": [],
                            },
                            "issue_contract": {
                                "classification": "defect_equivalent",
                                "equivalence_assessment": "equivalent_fix",
                                "reported_behavior": "validated bug scope",
                                "expected_behavior": "writer can proceed",
                                "scope_in": ["validated bug scope"],
                                "scope_out": [],
                                "acceptance_checks": ["writer receives issue contract"],
                                "spec_sync_targets": [],
                                "wvp_required": "no",
                                "wvp_status": "not_required",
                            },
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "analysis_plan": {
                        "required": "yes",
                        "receipt_required": "yes",
                        "route_task_class": "analysis",
                        "fanout_subagents": ["qwen_cli", "gemini_cli", "kilo_cli"],
                    },
                    "verification_plan": {"required": "no"},
                    "coach_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "route_budget": {},
                    "fallback_subagents": [],
                }

                def fake_subprocess_run(cmd, cwd=None, capture_output=False, text=False, check=False):
                    analysis_prompt_file = Path(cmd[5])
                    self.assertNotEqual(analysis_prompt_file, prompt_file)
                    prompt_text = analysis_prompt_file.read_text(encoding="utf-8")
                    self.assertEqual(self.packet_gate.validate_packet_text(prompt_text), [])
                    manifest_path = Path(cmd[6]) / "manifest.json"
                    manifest_path.parent.mkdir(parents=True, exist_ok=True)
                    manifest_path.write_text(
                        json.dumps(
                            {
                                "status": "completed",
                                "phase": "completed",
                                "synthesis_ready": True,
                                "results": [
                                    {
                                        "subagent": "qwen_cli",
                                        "status": "success",
                                        "output_file": str(analysis_output),
                                    }
                                ],
                            }
                        ),
                        encoding="utf-8",
                    )
                    return self.dispatch.subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout=str(manifest_path) + "\n",
                        stderr="",
                    )

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)), \
                    mock.patch.object(self.dispatch.subprocess, "run", side_effect=fake_subprocess_run):
                    exit_code = self.dispatch.run_prepare_execution(
                        [
                            "subagent-dispatch.py",
                            "prepare-execution",
                            "unit-task",
                            "implementation",
                            str(prompt_file),
                            str(tmp_path / "prepare"),
                            str(ROOT_DIR),
                        ]
                    )
                prepare_manifest = json.loads((tmp_path / "prepare" / "prepare-execution.json").read_text(encoding="utf-8"))
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir

        self.assertEqual(exit_code, 0)
        self.assertEqual(prepare_manifest["status"], "analysis_ready")
        self.assertTrue(bool(prepare_manifest["analysis_receipt_path"]))

    def test_prepare_execution_writes_issue_contract_for_equivalent_fix(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            original_issue_dir = self.dispatch.ISSUE_CONTRACT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
            self.dispatch.ISSUE_CONTRACT_DIR = tmp_path / "issue-contracts"
            try:
                prompt_file = tmp_path / "implementation.prompt.txt"
                prompt_file.write_text(
                    "Runtime proving task: unit-task\n\n"
                    "Implement the validated bugfix scope.\n\n"
                    "Scope:\n- Preserve server error context.\n\n"
                    "Acceptance:\n- Errors retain actionable details.\n",
                    encoding="utf-8",
                )
                analysis_output = tmp_path / "analysis.output.json"
                analysis_output.write_text(
                    json.dumps(
                        {
                            "status": "done",
                            "question_answered": "yes",
                            "answer": "root cause confirmed",
                            "evidence_refs": ["src/lib/core/api/interceptors/error_interceptor.dart:10"],
                            "changed_files": [],
                            "verification_commands": ["flutter test test/core/api/interceptors/error_interceptor_test.dart"],
                            "verification_results": ["flutter test -> pass"],
                            "merge_ready": "yes",
                            "blockers": [],
                            "notes": "equivalent root-cause fix",
                            "recommended_next_action": "proceed_to_writer",
                            "impact_analysis": {
                                "affected_scope": ["src/lib/core/api/interceptors/error_interceptor.dart"],
                                "contract_impact": ["error propagation contract"],
                                "follow_up_actions": [],
                                "residual_risks": [],
                            },
                            "issue_contract": {
                                "classification": "defect_equivalent",
                                "equivalence_assessment": "equivalent_fix",
                                "reported_behavior": "server errors degrade into false network errors",
                                "expected_behavior": "server error context remains available",
                                "reported_scope": ["api error handling stack", "dashboard quick stats symptom"],
                                "proven_scope": ["error interceptor stack"],
                                "symptoms": [
                                    {
                                        "id": "SYM-1",
                                        "summary": "server errors lose response context",
                                        "evidence_status": "red_test",
                                        "disposition": "in_scope",
                                        "evidence_refs": ["test/core/api/interceptors/error_interceptor_test.dart"],
                                    }
                                ],
                                "scope_in": ["error interceptor stack"],
                                "scope_out": ["drawer navigation"],
                                "acceptance_checks": ["Errors retain actionable details"],
                                "spec_sync_targets": ["docs/specs/api.md"],
                                "wvp_required": "no",
                                "wvp_status": "not_required",
                            },
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "analysis_plan": {
                        "required": "yes",
                        "receipt_required": "yes",
                        "route_task_class": "analysis",
                        "fanout_subagents": ["qwen_cli", "gemini_cli", "kilo_cli"],
                    },
                    "verification_plan": {"required": "no"},
                    "coach_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "route_budget": {},
                    "fallback_subagents": [],
                }

                def fake_subprocess_run(cmd, cwd=None, capture_output=False, text=False, check=False):
                    manifest_path = Path(cmd[6]) / "manifest.json"
                    manifest_path.parent.mkdir(parents=True, exist_ok=True)
                    manifest_path.write_text(
                        json.dumps(
                            {
                                "status": "completed",
                                "phase": "completed",
                                "synthesis_ready": True,
                                "results": [
                                    {
                                        "subagent": "qwen_cli",
                                        "status": "success",
                                        "output_file": str(analysis_output),
                                    }
                                ],
                            }
                        ),
                        encoding="utf-8",
                    )
                    return self.dispatch.subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout=str(manifest_path) + "\n",
                        stderr="",
                    )

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)), \
                    mock.patch.object(self.dispatch.subprocess, "run", side_effect=fake_subprocess_run):
                    exit_code = self.dispatch.run_prepare_execution(
                        [
                            "subagent-dispatch.py",
                            "prepare-execution",
                            "unit-task",
                            "implementation",
                            str(prompt_file),
                            str(tmp_path / "prepare"),
                            str(ROOT_DIR),
                        ]
                    )
                prepare_manifest = json.loads((tmp_path / "prepare" / "prepare-execution.json").read_text(encoding="utf-8"))
                issue_contract = json.loads((tmp_path / "issue-contracts" / "unit-task.json").read_text(encoding="utf-8"))
                writer_prompt_text = Path(prepare_manifest["effective_prompt_file"]).read_text(encoding="utf-8")
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir
                self.dispatch.ISSUE_CONTRACT_DIR = original_issue_dir

        self.assertEqual(exit_code, 0)
        self.assertEqual(prepare_manifest["status"], "analysis_ready")
        self.assertEqual(prepare_manifest["issue_contract"]["status"], "writer_ready")
        self.assertEqual(
            prepare_manifest["issue_contract"]["reported_scope"],
            ["api error handling stack", "dashboard quick stats symptom"],
        )
        self.assertEqual(prepare_manifest["issue_contract"]["proven_scope"], ["error interceptor stack"])
        self.assertTrue(prepare_manifest["writer_authorized"])
        self.assertEqual(issue_contract["classification"], "defect_equivalent")
        self.assertEqual(issue_contract["equivalence_assessment"], "equivalent_fix")
        self.assertEqual(self.packet_gate.validate_packet_text(writer_prompt_text), [])
        self.assertIn("Normalized issue contract:", writer_prompt_text)
        self.assertEqual(prepare_manifest["prompt_resolution"]["writer_packet_mode"], "issue_contract_rendered")

    def test_prepare_execution_blocks_when_issue_contract_requires_spec_delta(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            original_issue_dir = self.dispatch.ISSUE_CONTRACT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
            self.dispatch.ISSUE_CONTRACT_DIR = tmp_path / "issue-contracts"
            try:
                prompt_file = tmp_path / "implementation.prompt.txt"
                prompt_file.write_text("Implement the reported bugfix scope.\n", encoding="utf-8")
                analysis_output = tmp_path / "analysis.output.json"
                analysis_output.write_text(
                    json.dumps(
                        {
                            "status": "done",
                            "question_answered": "yes",
                            "answer": "non-equivalent product change required",
                            "evidence_refs": ["docs/specs/ui.md:10"],
                            "changed_files": [],
                            "verification_commands": [],
                            "verification_results": [],
                            "merge_ready": "yes",
                            "blockers": [],
                            "notes": "needs product contract update",
                            "recommended_next_action": "route_to_spec_delta",
                            "impact_analysis": {
                                "affected_scope": ["docs/specs/ui.md"],
                                "contract_impact": ["navigation behavior changes"],
                                "follow_up_actions": ["update spec before writer"],
                                "residual_risks": [],
                            },
                            "issue_contract": {
                                "classification": "feature_delta",
                                "equivalence_assessment": "spec_delta_required",
                                "reported_behavior": "user expects direct navigation",
                                "expected_behavior": "current behavior forces expand first",
                                "reported_scope": ["drawer first-level module behavior", "navigation ownership"],
                                "proven_scope": ["drawer first-level module behavior"],
                                "symptoms": [
                                    {
                                        "id": "SYM-1",
                                        "summary": "drawer first-level modules expand instead of navigate",
                                        "evidence_status": "reproduced",
                                        "disposition": "in_scope",
                                        "evidence_refs": ["docs/specs/ui.md:10"],
                                    },
                                    {
                                        "id": "SYM-2",
                                        "summary": "navigation ownership drift",
                                        "evidence_status": "unproven",
                                        "disposition": "out_of_scope",
                                        "evidence_refs": [],
                                    }
                                ],
                                "scope_in": ["drawer first-level module behavior"],
                                "scope_out": [],
                                "acceptance_checks": ["spec updated before implementation"],
                                "spec_sync_targets": ["docs/specs/ui.md"],
                                "wvp_required": "no",
                                "wvp_status": "not_required",
                            },
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "analysis_plan": {
                        "required": "yes",
                        "receipt_required": "yes",
                        "route_task_class": "analysis",
                        "fanout_subagents": ["qwen_cli", "gemini_cli", "kilo_cli"],
                    },
                    "verification_plan": {"required": "no"},
                    "coach_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "route_budget": {},
                    "fallback_subagents": [],
                }

                def fake_subprocess_run(cmd, cwd=None, capture_output=False, text=False, check=False):
                    manifest_path = Path(cmd[6]) / "manifest.json"
                    manifest_path.parent.mkdir(parents=True, exist_ok=True)
                    manifest_path.write_text(
                        json.dumps(
                            {
                                "status": "completed",
                                "phase": "completed",
                                "synthesis_ready": True,
                                "results": [
                                    {
                                        "subagent": "qwen_cli",
                                        "status": "success",
                                        "output_file": str(analysis_output),
                                    }
                                ],
                            }
                        ),
                        encoding="utf-8",
                    )
                    return self.dispatch.subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout=str(manifest_path) + "\n",
                        stderr="",
                    )

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)), \
                    mock.patch.object(self.dispatch.subprocess, "run", side_effect=fake_subprocess_run):
                    exit_code = self.dispatch.run_prepare_execution(
                        [
                            "subagent-dispatch.py",
                            "prepare-execution",
                            "unit-task",
                            "implementation",
                            str(prompt_file),
                            str(tmp_path / "prepare"),
                            str(ROOT_DIR),
                        ]
                    )
                prepare_manifest = json.loads((tmp_path / "prepare" / "prepare-execution.json").read_text(encoding="utf-8"))
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir
                self.dispatch.ISSUE_CONTRACT_DIR = original_issue_dir

        self.assertEqual(exit_code, 2)
        self.assertEqual(prepare_manifest["status"], "issue_contract_blocked")
        self.assertEqual(prepare_manifest["issue_contract"]["status"], "spec_delta_required")
        self.assertFalse(prepare_manifest["writer_authorized"])

    def test_prepare_execution_blocks_when_writer_ready_issue_contract_has_no_proven_scope(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            original_issue_dir = self.dispatch.ISSUE_CONTRACT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
            self.dispatch.ISSUE_CONTRACT_DIR = tmp_path / "issue-contracts"
            try:
                prompt_file = tmp_path / "implementation.prompt.txt"
                prompt_file.write_text("Implement the reported bugfix scope.\n", encoding="utf-8")
                analysis_output = tmp_path / "analysis.output.json"
                analysis_output.write_text(
                    json.dumps(
                        {
                            "status": "done",
                            "question_answered": "yes",
                            "answer": "scope not proven enough for writer",
                            "evidence_refs": ["docs/specs/ui.md:10"],
                            "changed_files": [],
                            "verification_commands": [],
                            "verification_results": [],
                            "merge_ready": "yes",
                            "blockers": [],
                            "notes": "needs narrower executable proof",
                            "recommended_next_action": "gather more evidence",
                            "impact_analysis": {
                                "affected_scope": ["docs/specs/ui.md"],
                                "contract_impact": [],
                                "follow_up_actions": ["prove executable surface before writer"],
                                "residual_risks": [],
                            },
                            "issue_contract": {
                                "classification": "defect_equivalent",
                                "equivalence_assessment": "equivalent_fix",
                                "reported_behavior": "module redirect is wrong",
                                "expected_behavior": "current screen should persist",
                                "reported_scope": ["router redirect", "locale remount", "account switch"],
                                "proven_scope": [],
                                "symptoms": [
                                    {
                                        "id": "SYM-1",
                                        "summary": "router redirect",
                                        "evidence_status": "reproduced",
                                        "disposition": "in_scope",
                                        "evidence_refs": ["docs/specs/ui.md:10"],
                                    },
                                    {
                                        "id": "SYM-2",
                                        "summary": "locale remount",
                                        "evidence_status": "unproven",
                                        "disposition": "in_scope",
                                        "evidence_refs": [],
                                    }
                                ],
                                "scope_in": [],
                                "scope_out": [],
                                "acceptance_checks": ["writer must stay inside proven scope"],
                                "spec_sync_targets": [],
                                "wvp_required": "no",
                                "wvp_status": "not_required",
                            },
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "analysis_plan": {
                        "required": "yes",
                        "receipt_required": "yes",
                        "route_task_class": "analysis",
                        "fanout_subagents": ["qwen_cli"],
                    },
                    "verification_plan": {"required": "no"},
                    "coach_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "route_budget": {},
                    "fallback_subagents": [],
                }

                def fake_subprocess_run(cmd, cwd=None, capture_output=False, text=False, check=False):
                    manifest_path = Path(cmd[6]) / "manifest.json"
                    manifest_path.parent.mkdir(parents=True, exist_ok=True)
                    manifest_path.write_text(
                        json.dumps(
                            {
                                "status": "completed",
                                "phase": "completed",
                                "synthesis_ready": True,
                                "results": [
                                    {
                                        "subagent": "qwen_cli",
                                        "status": "success",
                                        "output_file": str(analysis_output),
                                    }
                                ],
                            }
                        ),
                        encoding="utf-8",
                    )
                    return self.dispatch.subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout=str(manifest_path) + "\n",
                        stderr="",
                    )

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)), \
                    mock.patch.object(self.dispatch.subprocess, "run", side_effect=fake_subprocess_run):
                    exit_code = self.dispatch.run_prepare_execution(
                        [
                            "subagent-dispatch.py",
                            "prepare-execution",
                            "unit-task",
                            "implementation",
                            str(prompt_file),
                            str(tmp_path / "prepare"),
                            str(ROOT_DIR),
                        ]
                    )
                prepare_manifest = json.loads((tmp_path / "prepare" / "prepare-execution.json").read_text(encoding="utf-8"))
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir
                self.dispatch.ISSUE_CONTRACT_DIR = original_issue_dir

        self.assertEqual(exit_code, 2)
        self.assertEqual(prepare_manifest["status"], "issue_contract_blocked")
        self.assertEqual(prepare_manifest["issue_contract_error"], "missing_proven_scope")
        self.assertFalse(prepare_manifest["writer_authorized"])

    def test_prepare_execution_blocks_when_multi_symptom_issue_has_unproven_in_scope_symptom(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            original_issue_dir = self.dispatch.ISSUE_CONTRACT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
            self.dispatch.ISSUE_CONTRACT_DIR = tmp_path / "issue-contracts"
            try:
                prompt_file = tmp_path / "implementation.prompt.txt"
                prompt_file.write_text("Implement the reported bugfix scope.\n", encoding="utf-8")
                analysis_output = tmp_path / "analysis.output.json"
                analysis_output.write_text(
                    json.dumps(
                        {
                            "status": "done",
                            "question_answered": "yes",
                            "answer": "one symptom proven, one still unproven",
                            "evidence_refs": ["docs/specs/ui.md:10"],
                            "changed_files": [],
                            "verification_commands": [],
                            "verification_results": [],
                            "merge_ready": "yes",
                            "blockers": [],
                            "notes": "multi-symptom issue still ambiguous",
                            "recommended_next_action": "gather more evidence",
                            "impact_analysis": {
                                "affected_scope": ["docs/specs/ui.md"],
                                "contract_impact": [],
                                "follow_up_actions": ["prove second symptom or split scope"],
                                "residual_risks": [],
                            },
                            "issue_contract": {
                                "classification": "defect_equivalent",
                                "equivalence_assessment": "equivalent_fix",
                                "reported_behavior": "two navigation symptoms reported together",
                                "expected_behavior": "navigation remains stable",
                                "reported_scope": ["router redirect", "locale remount"],
                                "proven_scope": ["router redirect"],
                                "symptoms": [
                                    {
                                        "id": "SYM-1",
                                        "summary": "router redirect",
                                        "evidence_status": "red_test",
                                        "disposition": "in_scope",
                                        "evidence_refs": ["test/navigation/router_test.dart"],
                                    },
                                    {
                                        "id": "SYM-2",
                                        "summary": "locale remount",
                                        "evidence_status": "unproven",
                                        "disposition": "in_scope",
                                        "evidence_refs": [],
                                    }
                                ],
                                "scope_in": ["router redirect"],
                                "scope_out": [],
                                "acceptance_checks": ["writer must not run while second symptom is unproven"],
                                "spec_sync_targets": [],
                                "wvp_required": "no",
                                "wvp_status": "not_required",
                            },
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "analysis_plan": {
                        "required": "yes",
                        "receipt_required": "yes",
                        "route_task_class": "analysis",
                        "fanout_subagents": ["qwen_cli"],
                    },
                    "verification_plan": {"required": "no"},
                    "coach_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "route_budget": {},
                    "fallback_subagents": [],
                }

                def fake_subprocess_run(cmd, cwd=None, capture_output=False, text=False, check=False):
                    manifest_path = Path(cmd[6]) / "manifest.json"
                    manifest_path.parent.mkdir(parents=True, exist_ok=True)
                    manifest_path.write_text(
                        json.dumps(
                            {
                                "status": "completed",
                                "phase": "completed",
                                "synthesis_ready": True,
                                "results": [
                                    {
                                        "subagent": "qwen_cli",
                                        "status": "success",
                                        "output_file": str(analysis_output),
                                    }
                                ],
                            }
                        ),
                        encoding="utf-8",
                    )
                    return self.dispatch.subprocess.CompletedProcess(
                        cmd,
                        0,
                        stdout=str(manifest_path) + "\n",
                        stderr="",
                    )

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)), \
                    mock.patch.object(self.dispatch.subprocess, "run", side_effect=fake_subprocess_run):
                    exit_code = self.dispatch.run_prepare_execution(
                        [
                            "subagent-dispatch.py",
                            "prepare-execution",
                            "unit-task",
                            "implementation",
                            str(prompt_file),
                            str(tmp_path / "prepare"),
                            str(ROOT_DIR),
                        ]
                    )
                prepare_manifest = json.loads((tmp_path / "prepare" / "prepare-execution.json").read_text(encoding="utf-8"))
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir
                self.dispatch.ISSUE_CONTRACT_DIR = original_issue_dir

        self.assertEqual(exit_code, 2)
        self.assertEqual(prepare_manifest["status"], "issue_contract_blocked")
        self.assertEqual(prepare_manifest["issue_contract_error"], "unproven_symptoms:SYM-2")
        self.assertFalse(prepare_manifest["writer_authorized"])

    def test_prepare_execution_writes_issue_split_artifact_for_mixed_issue(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            original_issue_dir = self.dispatch.ISSUE_CONTRACT_DIR
            original_split_dir = self.dispatch.ISSUE_SPLIT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
            self.dispatch.ISSUE_CONTRACT_DIR = tmp_path / "issue-contracts"
            self.dispatch.ISSUE_SPLIT_DIR = tmp_path / "issue-splits"
            try:
                prompt_file = tmp_path / "implementation.prompt.txt"
                prompt_file.write_text("Implement the reported bugfix scope.\n", encoding="utf-8")
                analysis_output = tmp_path / "analysis.output.json"
                analysis_output.write_text(
                    json.dumps(
                        {
                            "status": "done",
                            "question_answered": "yes",
                            "answer": "primary symptom is proven; secondary symptom should be deferred",
                            "evidence_refs": ["test/navigation/router_test.dart"],
                            "changed_files": [],
                            "verification_commands": [],
                            "verification_results": [],
                            "merge_ready": "yes",
                            "blockers": [],
                            "notes": "safe to route writer only through primary slice",
                            "recommended_next_action": "proceed_to_writer",
                            "impact_analysis": {
                                "affected_scope": ["router redirect"],
                                "contract_impact": [],
                                "follow_up_actions": ["split locale remount into follow-up"],
                                "residual_risks": [],
                            },
                            "issue_contract": {
                                "classification": "defect_equivalent",
                                "equivalence_assessment": "equivalent_fix",
                                "reported_behavior": "router redirect and locale remount were reported together",
                                "expected_behavior": "router redirect is fixed without widening into locale work",
                                "reported_scope": ["router redirect", "locale remount"],
                                "proven_scope": ["router redirect"],
                                "symptoms": [
                                    {
                                        "id": "SYM-1",
                                        "summary": "router redirect",
                                        "evidence_status": "red_test",
                                        "disposition": "in_scope",
                                        "evidence_refs": ["test/navigation/router_test.dart"],
                                    },
                                    {
                                        "id": "SYM-2",
                                        "summary": "locale remount",
                                        "evidence_status": "unproven",
                                        "disposition": "out_of_scope",
                                        "evidence_refs": [],
                                    }
                                ],
                                "scope_in": ["router redirect"],
                                "scope_out": [],
                                "acceptance_checks": ["writer stays on primary slice only"],
                                "spec_sync_targets": [],
                                "wvp_required": "no",
                                "wvp_status": "not_required",
                            },
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "analysis_plan": {
                        "required": "yes",
                        "receipt_required": "yes",
                        "route_task_class": "analysis",
                        "fanout_subagents": ["qwen_cli"],
                    },
                    "verification_plan": {"required": "no"},
                    "coach_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "route_budget": {},
                    "fallback_subagents": [],
                }

                def fake_subprocess_run(cmd, cwd=None, capture_output=False, text=False, check=False):
                    if "ensemble" in cmd:
                        manifest_path = Path(cmd[6]) / "manifest.json"
                        manifest_path.parent.mkdir(parents=True, exist_ok=True)
                        manifest_path.write_text(
                            json.dumps(
                                {
                                    "status": "completed",
                                    "phase": "completed",
                                    "synthesis_ready": True,
                                    "results": [
                                        {
                                            "subagent": "qwen_cli",
                                            "status": "success",
                                            "output_file": str(analysis_output),
                                        }
                                    ],
                                }
                            ),
                            encoding="utf-8",
                        )
                        return self.dispatch.subprocess.CompletedProcess(
                            cmd,
                            0,
                            stdout=str(manifest_path) + "\n",
                            stderr="",
                        )
                    if "create" in cmd:
                        return self.dispatch.subprocess.CompletedProcess(
                            cmd,
                            0,
                            stdout=json.dumps({"id": "unit-task.1"}) + "\n",
                            stderr="",
                        )
                    raise AssertionError(f"unexpected subprocess command: {cmd}")

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)), \
                    mock.patch.object(self.dispatch.subprocess, "run", side_effect=fake_subprocess_run):
                    exit_code = self.dispatch.run_prepare_execution(
                        [
                            "subagent-dispatch.py",
                            "prepare-execution",
                            "unit-task",
                            "implementation",
                            str(prompt_file),
                            str(tmp_path / "prepare"),
                            str(ROOT_DIR),
                        ]
                    )

                prepare_manifest = json.loads((tmp_path / "prepare" / "prepare-execution.json").read_text(encoding="utf-8"))
                split_payload = json.loads((tmp_path / "issue-splits" / "unit-task.json").read_text(encoding="utf-8"))
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir
                self.dispatch.ISSUE_CONTRACT_DIR = original_issue_dir
                self.dispatch.ISSUE_SPLIT_DIR = original_split_dir

        self.assertEqual(exit_code, 0)
        self.assertEqual(prepare_manifest["status"], "analysis_ready")
        self.assertEqual(split_payload["status"], "mixed_issue_split_detected")
        self.assertEqual(split_payload["primary_executable_slice"]["symptom_ids"], ["SYM-1"])
        self.assertEqual(split_payload["secondary_unresolved_slice"]["symptom_ids"], ["SYM-2"])
        self.assertEqual(split_payload["follow_up_task_id"], "unit-task.1")
        self.assertEqual(prepare_manifest["issue_split_follow_up"]["task_id"], "unit-task.1")

    def test_start_subagent_process_runs_live_web_probe_for_provider_configured_lane(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            prompt_file = tmp_path / "prompt.txt"
            output_file = tmp_path / "output.txt"
            prompt_file.write_text("Runtime Role Packet:\n- worker_lane_confirmed: true\n", encoding="utf-8")
            route = {"web_search_required": "yes", "dispatch_policy": {}, "route_budget": {}, "fallback_subagents": []}
            subagent_cfg = self.minimal_subagent_cfg()
            subagent_cfg["dispatch"]["command"] = "qwen"
            subagent_cfg["dispatch"]["web_search_mode"] = "provider_configured"
            subagent_cfg["subagent_backend_class"] = "external_cli"

            fake_process = mock.Mock()
            fake_process.poll.return_value = 0
            fake_process.returncode = 0

            with mock.patch.object(self.dispatch.worker_packet_gate, "validate_packet_text", return_value=[]), \
                mock.patch.object(self.dispatch, "selected_model_for_subagent", return_value=None), \
                mock.patch.object(self.dispatch.subagent_system, "web_search_probe_subagent", return_value={"web_probe": {"success": True}}) as mocked_probe, \
                mock.patch.object(self.dispatch, "acquire_dispatch_pool_lease", return_value={"status": "acquired", "lease": {"holder": "unit-task:research:run"}}), \
                mock.patch.object(self.dispatch.subprocess, "Popen", return_value=fake_process):
                launch = self.dispatch.start_subagent_process(
                    "unit-task",
                    "research",
                    "qwen_cli",
                    prompt_file,
                    output_file,
                    tmp_path,
                    route,
                    subagent_cfg,
                    "single",
                )
                self.dispatch.close_launch_handles(launch)

        self.assertIn("process", launch)
        mocked_probe.assert_called_once_with("qwen_cli")

    def test_finalize_subagent_process_releases_pool_lease(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            output_file = tmp_path / "output.txt"
            stderr_file = tmp_path / "output.txt.stderr.log"
            prompt_file = tmp_path / "prompt.txt"
            output_file.write_text("{\"merge_ready\":\"yes\"}\n", encoding="utf-8")
            stderr_file.write_text("", encoding="utf-8")
            prompt_file.write_text("", encoding="utf-8")
            fake_process = mock.Mock()
            fake_process.poll.return_value = 0
            fake_process.returncode = 0
            launch = {
                "process": fake_process,
                "stdout_handle": None,
                "stderr_handle": None,
                "task_id": "unit-task",
                "task_class": "analysis",
                "subagent_name": "kilo_cli",
                "selected_model": None,
                "subagent_cfg": self.minimal_subagent_cfg(),
                "dispatch_mode": "single",
                "risk_class": "R0",
                "domain_tags": [],
                "max_runtime_seconds": 60,
                "min_output_bytes": 0,
                "output_file": output_file,
                "stderr_path": stderr_file,
                "workdir": tmp_path,
                "prompt_file": prompt_file,
                "route": self.minimal_route(),
                "run_id": "run-1",
                "ts_start": "2026-03-07T00:00:00Z",
                "started": 0.0,
                "pool_lease": {"holder": "unit-task:analysis:run-1"},
            }
            with mock.patch.object(self.dispatch, "subagent_result_payload", return_value={"status": "success"}) as mocked_payload, \
                mock.patch.object(self.dispatch, "release_dispatch_pool_lease", return_value={"status": "released"}) as mocked_release, \
                mock.patch.object(self.dispatch.time, "monotonic", return_value=1.0):
                result = self.dispatch.finalize_subagent_process(launch)

        mocked_release.assert_called_once()
        mocked_payload.assert_called_once()
        self.assertEqual(result["pool_release"]["status"], "released")

    def test_parse_coach_decision_detects_return_for_rework(self) -> None:
        output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation misses the close gate",
  "evidence_refs": ["_vida/scripts/quality-health-check.sh:280"],
  "changed_files": [],
  "verification_commands": ["bash _vida/scripts/quality-health-check.sh --mode quick unit-task"],
  "verification_results": ["coach review indicates missing gate wiring"],
  "merge_ready": "no",
  "blockers": ["coach gate missing from quality health check"],
  "notes": "return to writer",
  "recommended_next_action": "wire coach gate into close checks",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/quality-health-check.sh"],
    "contract_impact": ["close gate incomplete"],
    "follow_up_actions": ["rerun coach review after patch"],
    "residual_risks": ["writer may still bypass coach if not enforced"]
  },
  "coach_decision": "return_for_rework",
  "rework_required": "yes",
  "coach_feedback": "wire the mandatory coach gate before close"
}
"""

        decision = self.dispatch.parse_coach_decision(output)

        self.assertFalse(decision["approved"])
        self.assertEqual(decision["coach_decision"], "return_for_rework")
        self.assertEqual(decision["rework_required"], "yes")
        self.assertIn("coach gate missing", decision["reason"])

    def test_parse_coach_decision_fails_closed_without_payload(self) -> None:
        decision = self.dispatch.parse_coach_decision("")

        self.assertFalse(decision["approved"])
        self.assertEqual(decision["coach_decision"], "coach_failed")
        self.assertEqual(decision["payload_state"], "missing_payload")
        self.assertEqual(decision["reason"], "missing_coach_decision_payload")

    def test_parse_coach_decision_marks_approved_merge_conflict_invalid(self) -> None:
        output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "looks fine",
  "evidence_refs": ["_vida/scripts/subagent-dispatch.py:2442"],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "no",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_decision": "approved",
  "rework_required": "no",
  "coach_feedback": "ready for verification"
}
"""

        decision = self.dispatch.parse_coach_decision(output)

        self.assertFalse(decision["approved"])
        self.assertEqual(decision["coach_decision"], "invalid_coach_payload.approved_conflict")
        self.assertEqual(decision["payload_state"], "invalid_coach_payload.approved_conflict")
        self.assertIn("approved_conflicts_with_merge_ready", decision["invalid_reasons"])
        self.assertEqual(decision["recommended_next_action"], "rerun_coach_review_with_valid_machine_readable_output")

    def test_parse_coach_decision_accepts_valid_approved_payload(self) -> None:
        output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation is ready",
  "evidence_refs": ["_vida/scripts/subagent-dispatch.py:2442"],
  "changed_files": [],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> pass"],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "approve_for_independent_verification",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/subagent-dispatch.py"],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_decision": "approved",
  "rework_required": "no",
  "coach_feedback": "ready for verification"
}
"""

        decision = self.dispatch.parse_coach_decision(output)

        self.assertTrue(decision["approved"])
        self.assertEqual(decision["coach_decision"], "approved")
        self.assertEqual(decision["payload_state"], "approved")
        self.assertEqual(decision["invalid_reasons"], [])

    def test_parse_coach_decision_marks_approved_rework_conflict_invalid(self) -> None:
        output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation is complete",
  "evidence_refs": ["_vida/scripts/subagent-dispatch.py:2442"],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "yes",
  "blockers": ["missing tests"],
  "notes": "",
  "recommended_next_action": "approve_for_independent_verification",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_decision": "approved",
  "rework_required": "yes",
  "coach_feedback": "ready for verification"
}
"""

        decision = self.dispatch.parse_coach_decision(output)

        self.assertFalse(decision["approved"])
        self.assertEqual(decision["coach_decision"], "invalid_coach_payload.approved_conflict")
        self.assertIn("approved_conflicts_with_rework_required", decision["invalid_reasons"])
        self.assertIn("approved_conflicts_with_blockers", decision["invalid_reasons"])

    def test_parse_coach_decision_marks_missing_finality_invalid(self) -> None:
        output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "review completed",
  "evidence_refs": ["_vida/scripts/subagent-dispatch.py:2442"],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "",
  "blockers": [],
  "notes": "no final verdict recorded",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_feedback": "missing final verdict"
}
"""

        decision = self.dispatch.parse_coach_decision(output)

        self.assertFalse(decision["approved"])
        self.assertEqual(decision["coach_decision"], "invalid_coach_payload.ambiguous_finality")
        self.assertIn("missing_finality_signals", decision["invalid_reasons"])

    def test_coach_decision_from_result_falls_back_to_stderr_json_payload(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            output_file = tmp_path / "coach-output.txt"
            stderr_file = tmp_path / "coach-output.txt.stderr.log"
            output_file.write_text("", encoding="utf-8")
            stderr_file.write_text(
                """
Coach had to print the final payload on stderr.
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation still misses the gate",
  "evidence_refs": ["_vida/scripts/execution-auth-gate.py:1"],
  "changed_files": [],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> fail"],
  "merge_ready": "no",
  "blockers": ["execution gate still missing"],
  "notes": "return to writer",
  "recommended_next_action": "rerun the writer from the original spec with the gate added",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/execution-auth-gate.py"],
    "contract_impact": ["writer gate is incomplete"],
    "follow_up_actions": ["rerun coach review"],
    "residual_risks": ["writer may keep iterating locally"]
  },
  "coach_decision": "return_for_rework",
  "rework_required": "yes",
  "coach_feedback": "write the missing execution gate before verification"
}
""",
                encoding="utf-8",
            )

            decision = self.dispatch.coach_decision_from_result(
                {
                    "subagent": "qwen_cli",
                    "status": "failure",
                    "output_file": str(output_file),
                    "stderr_file": str(stderr_file),
                    "error_text": "command exited with validation chatter on stderr",
                }
            )

        self.assertFalse(decision["approved"])
        self.assertEqual(decision["coach_decision"], "return_for_rework")
        self.assertEqual(decision["feedback_source"], "stderr_json_payload")
        self.assertIn("stderr_json_payload", decision["feedback_sources"])
        self.assertIn("execution gate still missing", decision["reason"])

    def test_coach_decision_from_result_preserves_text_feedback_when_payload_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            output_file = tmp_path / "coach-output.txt"
            output_file.write_text(
                "The implementation still misses the execution gate. Return this to the writer with a fresh-start handoff.\n",
                encoding="utf-8",
            )

            decision = self.dispatch.coach_decision_from_result(
                {
                    "subagent": "qwen_cli",
                    "status": "failure",
                    "output_file": str(output_file),
                    "stderr_file": str(output_file.with_suffix(output_file.suffix + '.stderr.log')),
                    "error_text": "worker output contract invalid",
                }
            )

        self.assertFalse(decision["approved"])
        self.assertEqual(decision["coach_decision"], "coach_failed")
        self.assertEqual(decision["payload_state"], "missing_payload")
        self.assertEqual(decision["feedback_source"], "output_text")
        self.assertIn("output_text", decision["feedback_sources"])
        self.assertIn("execution gate", decision["coach_feedback"])
        self.assertIn("missing_coach_decision_payload", decision["reason"])

    def test_effective_writer_prompt_uses_fresh_rework_handoff(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path
            try:
                prompt_file = tmp_path / "writer.prompt.txt"
                prompt_file.write_text(
                    "Runtime Role Packet:\n"
                    "- worker_lane_confirmed: true\n"
                    "- worker_role: subagent\n"
                    "- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD\n"
                    "- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD\n"
                    "- impact_tail_policy: required_for_non_stc\n"
                    "- impact_analysis_scope: bounded_to_assigned_scope\n"
                    "Task: implement route gate\n"
                    "Scope: _vida/scripts\n"
                    "Blocking Question: What is the minimal safe fix?\n"
                    "Verification:\n"
                    "- python3 -m unittest\n"
                    "Deliverable:\n"
                    "- Return the machine-readable summary below.\n"
                    "```json\n"
                    "{\"status\":\"done\",\"question_answered\":\"yes\",\"answer\":\"x\",\"evidence_refs\":[],\"changed_files\":[],\"verification_commands\":[],\"verification_results\":[],\"merge_ready\":\"yes\",\"blockers\":[],\"notes\":\"\",\"recommended_next_action\":\"\",\"impact_analysis\":{\"affected_scope\":[],\"contract_impact\":[],\"follow_up_actions\":[],\"residual_risks\":[]}}\n"
                    "```\n",
                    encoding="utf-8",
                )
                route = self.minimal_route()
                coach_decision = {
                    "coach_feedback": "rebuild the writer pass from the spec and wire the missing gate",
                    "reason": "missing_execution_gate",
                    "recommended_next_action": "rerun writer from clean spec packet",
                    "blockers": ["execution gate missing"],
                    "evidence_refs": ["_vida/scripts/execution-auth-gate.py:1"],
                    "verification_results": ["python3 -m unittest -> failed before fix"],
                    "impact_analysis": {"affected_scope": ["_vida/scripts/execution-auth-gate.py"]},
                }

                self.dispatch.write_rework_handoff(
                    "unit-task",
                    "implementation",
                    route,
                    original_prompt=prompt_file.read_text(encoding="utf-8"),
                    coach_decision=coach_decision,
                    attempt_count=1,
                    max_coach_passes=2,
                )
                effective_prompt_file, metadata = self.dispatch.effective_writer_prompt(
                    "unit-task",
                    "implementation",
                    route,
                    prompt_file,
                    tmp_path,
                )
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir

            self.assertEqual(metadata["mode"], "fresh_rework_handoff")
            self.assertTrue(effective_prompt_file.exists())
            prompt_text = effective_prompt_file.read_text(encoding="utf-8")
            self.assertIn("Fresh Rework Handoff:", prompt_text)
            self.assertIn("Start a fresh implementation pass from the original prompt/spec above.", prompt_text)
            self.assertIn("execution gate missing", prompt_text)
            self.assertIn("rerun writer from clean spec packet", prompt_text)
            self.assertEqual(self.packet_gate.validate_packet_text(prompt_text), [])

    def test_run_coach_review_writes_rework_handoff(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path
            try:
                prompt_file = tmp_path / "writer.prompt.txt"
                prompt_file.write_text("Implement the execution gate from spec.\n", encoding="utf-8")
                coach_output = tmp_path / "coach-output.txt"
                coach_output.write_text(
                    """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation still misses the gate",
  "evidence_refs": ["_vida/scripts/execution-auth-gate.py:1"],
  "changed_files": [],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> fail"],
  "merge_ready": "no",
  "blockers": ["execution gate still missing"],
  "notes": "return to writer",
  "recommended_next_action": "rerun the writer from the original spec with the gate added",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/execution-auth-gate.py"],
    "contract_impact": ["writer gate is incomplete"],
    "follow_up_actions": ["rerun coach review"],
    "residual_risks": ["writer may keep iterating locally"]
  },
  "coach_decision": "return_for_rework",
  "rework_required": "yes",
  "coach_feedback": "write the missing execution gate before verification"
}
""",
                    encoding="utf-8",
                )
                coach_manifest_path = tmp_path / "coach-manifest.json"
                coach_manifest_path.write_text(
                    json.dumps(
                        {
                            "status": "completed",
                            "phase": "completed",
                            "synthesis_ready": False,
                            "results": [
                                {
                                    "subagent": "qwen_cli",
                                    "status": "success",
                                    "output_file": str(coach_output),
                                }
                            ],
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "coach_plan": {
                        "required": "yes",
                        "route_task_class": "coach",
                        "selected_subagent": "qwen_cli",
                        "max_passes": 2,
                    },
                    "route_budget": {"max_coach_passes": 2},
                    "analysis_plan": {
                        "required": "no",
                        "receipt_required": "no",
                        "route_task_class": "",
                        "fanout_subagents": [],
                    },
                    "verification_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "fallback_subagents": [],
                }
                coach_manifest = {
                    "status": "return_for_rework",
                    "phase": "blocked",
                    "synthesis_ready": False,
                    "result_count": 1,
                    "results": [
                        {
                            "subagent": "qwen_cli",
                            "status": "success",
                            "output_file": str(coach_output),
                        }
                    ],
                }
                coach_decision = self.dispatch.parse_coach_decision(coach_output.read_text(encoding="utf-8"))
                coach_decision["subagent"] = "qwen_cli"
                coach_decision["output_file"] = str(coach_output)

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)):
                    with mock.patch.object(
                        self.dispatch,
                        "run_coach_ensemble",
                        return_value=(coach_manifest_path, coach_manifest, coach_decision),
                    ):
                        exit_code = self.dispatch.run_coach_review(
                            [
                                "subagent-dispatch.py",
                                "coach-review",
                                "unit-task",
                                "implementation",
                                str(prompt_file),
                                str(tmp_path / "coach-review-run"),
                                str(ROOT_DIR),
                            ]
                        )
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir

            self.assertEqual(exit_code, 2)
            handoff_path = tmp_path / "unit-task.implementation.rework-handoff.json"
            blocker_path = tmp_path / "unit-task.implementation.coach-blocker.json"
            self.assertTrue(handoff_path.exists())
            self.assertTrue(blocker_path.exists())
            handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
            blocker = json.loads(blocker_path.read_text(encoding="utf-8"))
            self.assertEqual(handoff["status"], "writer_rework_ready")
            self.assertTrue(handoff["fresh_start_required"])
            self.assertIn("Fresh Rework Handoff:", handoff["fresh_prompt_text"])
            self.assertEqual(handoff["coach_delta"]["feedback_source"], "output_json_payload")
            self.assertEqual(handoff["coach_delta"]["feedback_sources"], ["output_json_payload"])
            self.assertEqual(blocker["status"], "return_for_rework")
            self.assertEqual(blocker["rework_handoff_status"], "writer_rework_ready")

    def test_run_coach_review_treats_missing_payload_as_coach_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path
            try:
                prompt_file = tmp_path / "writer.prompt.txt"
                prompt_file.write_text("Implement the execution gate from spec.\n", encoding="utf-8")
                coach_output = tmp_path / "coach-output.txt"
                coach_output.write_text("", encoding="utf-8")
                coach_manifest_path = tmp_path / "coach-manifest.json"
                coach_manifest_path.write_text(
                    json.dumps(
                        {
                            "status": "blocked",
                            "phase": "budget_blocked",
                            "synthesis_ready": False,
                            "results": [
                                {
                                    "subagent": "qwen_cli",
                                    "status": "failure",
                                    "output_file": str(coach_output),
                                }
                            ],
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "coach_plan": {
                        "required": "yes",
                        "route_task_class": "coach",
                        "selected_subagent": "qwen_cli",
                        "max_passes": 2,
                    },
                    "route_budget": {"max_coach_passes": 2},
                    "analysis_plan": {
                        "required": "no",
                        "receipt_required": "no",
                        "route_task_class": "",
                        "fanout_subagents": [],
                    },
                    "verification_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "fallback_subagents": [],
                }
                coach_manifest = {
                    "status": "coach_failed",
                    "phase": "blocked",
                    "synthesis_ready": False,
                    "result_count": 1,
                    "results": [
                        {
                            "subagent": "qwen_cli",
                            "status": "failure",
                            "output_file": str(coach_output),
                        }
                    ],
                }
                coach_decision = {
                    "approved": False,
                    "coach_decision": "coach_failed",
                    "payload_state": "missing_payload",
                    "invalid_reasons": ["missing_coach_decision_payload"],
                    "rework_required": "yes",
                    "coach_feedback": "",
                    "recommended_next_action": "",
                    "reason": "missing_coach_decision_payload",
                    "parsed_json": False,
                    "blockers": [],
                    "evidence_refs": [],
                    "verification_results": [],
                    "impact_analysis": {},
                    "answer": "",
                    "merge_ready_effective": "no",
                    "raw_merge_ready": "",
                    "raw_rework_required": "",
                    "subagent": "qwen_cli",
                    "output_file": str(coach_output),
                }

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)):
                    with mock.patch.object(
                        self.dispatch,
                        "run_coach_ensemble",
                        return_value=(coach_manifest_path, coach_manifest, coach_decision),
                    ):
                        exit_code = self.dispatch.run_coach_review(
                            [
                                "subagent-dispatch.py",
                                "coach-review",
                                "unit-task",
                                "implementation",
                                str(prompt_file),
                                str(tmp_path / "coach-review-run"),
                                str(ROOT_DIR),
                            ]
                        )
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir

            self.assertEqual(exit_code, 2)
            self.assertFalse((tmp_path / "unit-task.implementation.rework-handoff.json").exists())
            blocker = json.loads((tmp_path / "unit-task.implementation.coach-blocker.json").read_text(encoding="utf-8"))
            self.assertEqual(blocker["status"], "coach_failed")
            self.assertEqual(blocker["rework_handoff_status"], "")

    def test_run_coach_review_treats_invalid_payload_as_coach_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            original_receipt_dir = self.dispatch.ROUTE_RECEIPT_DIR
            self.dispatch.ROUTE_RECEIPT_DIR = tmp_path
            try:
                prompt_file = tmp_path / "writer.prompt.txt"
                prompt_file.write_text("Implement the execution gate from spec.\n", encoding="utf-8")
                coach_output = tmp_path / "coach-output.txt"
                coach_output.write_text(
                    """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation is approved",
  "evidence_refs": ["_vida/scripts/subagent-dispatch.py:2442"],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "no",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "approve_for_independent_verification",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_decision": "approved",
  "rework_required": "no",
  "coach_feedback": "ready for verification"
}
""",
                    encoding="utf-8",
                )
                coach_manifest_path = tmp_path / "coach-manifest.json"
                coach_manifest_path.write_text(
                    json.dumps(
                        {
                            "status": "completed",
                            "phase": "completed",
                            "synthesis_ready": False,
                            "results": [
                                {
                                    "subagent": "qwen_cli",
                                    "status": "success",
                                    "output_file": str(coach_output),
                                }
                            ],
                        }
                    ),
                    encoding="utf-8",
                )
                route = {
                    "task_class": "implementation",
                    "coach_plan": {
                        "required": "yes",
                        "route_task_class": "coach",
                        "selected_subagent": "qwen_cli",
                        "max_passes": 2,
                    },
                    "route_budget": {"max_coach_passes": 2},
                    "analysis_plan": {
                        "required": "no",
                        "receipt_required": "no",
                        "route_task_class": "",
                        "fanout_subagents": [],
                    },
                    "verification_plan": {"required": "no"},
                    "dispatch_policy": {},
                    "fallback_subagents": [],
                }
                coach_manifest = {
                    "status": "coach_failed",
                    "phase": "blocked",
                    "synthesis_ready": False,
                    "result_count": 1,
                    "results": [
                        {
                            "subagent": "qwen_cli",
                            "status": "success",
                            "output_file": str(coach_output),
                        }
                    ],
                }
                coach_decision = self.dispatch.parse_coach_decision(coach_output.read_text(encoding="utf-8"))
                coach_decision["subagent"] = "qwen_cli"
                coach_decision["output_file"] = str(coach_output)

                with mock.patch.object(self.dispatch, "route_snapshot", return_value=({}, route)):
                    with mock.patch.object(
                        self.dispatch,
                        "run_coach_ensemble",
                        return_value=(coach_manifest_path, coach_manifest, coach_decision),
                    ):
                        exit_code = self.dispatch.run_coach_review(
                            [
                                "subagent-dispatch.py",
                                "coach-review",
                                "unit-task",
                                "implementation",
                                str(prompt_file),
                                str(tmp_path / "coach-review-run"),
                                str(ROOT_DIR),
                            ]
                        )
            finally:
                self.dispatch.ROUTE_RECEIPT_DIR = original_receipt_dir

            self.assertEqual(exit_code, 2)
            self.assertFalse((tmp_path / "unit-task.implementation.rework-handoff.json").exists())
            blocker = json.loads((tmp_path / "unit-task.implementation.coach-blocker.json").read_text(encoding="utf-8"))
            self.assertEqual(blocker["status"], "coach_failed")
            self.assertEqual(blocker["coach_decision"]["coach_decision"], "invalid_coach_payload.approved_conflict")

    def test_merge_coach_decisions_approves_only_when_quorum_approves(self) -> None:
        merged = self.dispatch.merge_coach_decisions(
            [
                {
                    "subagent": "qwen_cli",
                    "approved": True,
                    "coach_decision": "approved",
                    "payload_state": "approved",
                    "parsed_json": True,
                    "coach_feedback": "ready",
                    "recommended_next_action": "proceed_to_independent_verification",
                    "blockers": [],
                    "evidence_refs": ["a"],
                    "verification_results": [],
                    "impact_analysis": {},
                    "answer": "ready",
                },
                {
                    "subagent": "gemini_cli",
                    "approved": True,
                    "coach_decision": "approved",
                    "payload_state": "approved",
                    "parsed_json": True,
                    "coach_feedback": "looks good",
                    "recommended_next_action": "proceed_to_independent_verification",
                    "blockers": [],
                    "evidence_refs": ["b"],
                    "verification_results": [],
                    "impact_analysis": {},
                    "answer": "looks good",
                },
            ],
            required_results=2,
            merge_policy="unanimous_approve_rework_bias",
        )

        self.assertTrue(merged["approved"])
        self.assertEqual(merged["coach_decision"], "approved")
        self.assertEqual(merged["valid_result_count"], 2)
        self.assertEqual(merged["selected_subagents"], ["qwen_cli", "gemini_cli"])

    def test_merge_coach_decisions_returns_rework_when_any_valid_coach_requests_it(self) -> None:
        merged = self.dispatch.merge_coach_decisions(
            [
                {
                    "subagent": "qwen_cli",
                    "approved": True,
                    "coach_decision": "approved",
                    "payload_state": "approved",
                    "parsed_json": True,
                    "coach_feedback": "ready",
                    "recommended_next_action": "proceed_to_independent_verification",
                    "blockers": [],
                    "evidence_refs": [],
                    "verification_results": [],
                    "impact_analysis": {},
                    "answer": "ready",
                },
                {
                    "subagent": "gemini_cli",
                    "approved": False,
                    "coach_decision": "return_for_rework",
                    "payload_state": "return_for_rework",
                    "parsed_json": True,
                    "coach_feedback": "missing gate",
                    "recommended_next_action": "return_to_writer",
                    "blockers": ["missing gate"],
                    "evidence_refs": ["_vida/scripts/execution-auth-gate.py:10"],
                    "verification_results": ["gate missing"],
                    "impact_analysis": {"affected_scope": ["_vida/scripts/execution-auth-gate.py"]},
                    "answer": "missing gate",
                },
            ],
            required_results=2,
            merge_policy="unanimous_approve_rework_bias",
        )

        self.assertFalse(merged["approved"])
        self.assertEqual(merged["coach_decision"], "return_for_rework")
        self.assertIn("missing gate", merged["coach_feedback"])
        self.assertIn("gemini_cli", merged["selected_subagents"])

    def test_merge_coach_decisions_fails_closed_without_valid_quorum(self) -> None:
        merged = self.dispatch.merge_coach_decisions(
            [
                {
                    "subagent": "qwen_cli",
                    "approved": True,
                    "coach_decision": "approved",
                    "payload_state": "approved",
                    "parsed_json": True,
                    "coach_feedback": "ready",
                    "recommended_next_action": "proceed_to_independent_verification",
                    "blockers": [],
                    "evidence_refs": [],
                    "verification_results": [],
                    "impact_analysis": {},
                    "answer": "ready",
                },
                {
                    "subagent": "kilo_cli",
                    "approved": False,
                    "coach_decision": "coach_failed",
                    "payload_state": "invalid_coach_payload.ambiguous_finality",
                    "invalid_reasons": ["missing_finality_signals"],
                    "parsed_json": True,
                    "coach_feedback": "",
                    "recommended_next_action": "rerun",
                    "blockers": [],
                    "evidence_refs": [],
                    "verification_results": [],
                    "impact_analysis": {},
                    "answer": "",
                },
            ],
            required_results=2,
            merge_policy="unanimous_approve_rework_bias",
        )

        self.assertFalse(merged["approved"])
        self.assertEqual(merged["coach_decision"], "coach_failed")
        self.assertEqual(merged["valid_result_count"], 1)
        self.assertIn("insufficient_valid_coach_results", merged["reason"])

    def test_run_coach_ensemble_skips_unauthorized_internal_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            prompt_file = tmp_path / "coach.prompt.txt"
            prompt_file.write_text("Coach the implementation.\n", encoding="utf-8")
            qwen_output = tmp_path / "qwen.txt"
            qwen_output.write_text("", encoding="utf-8")
            internal_output = tmp_path / "internal.txt"
            internal_output.write_text(
                """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "needs rework",
  "evidence_refs": [],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "no",
  "blockers": ["missing gate"],
  "notes": "",
  "recommended_next_action": "return_to_writer",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/execution-auth-gate.py"],
    "contract_impact": ["missing gate"],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_decision": "return_for_rework",
  "rework_required": "yes",
  "coach_feedback": "missing gate"
}
""",
                encoding="utf-8",
            )
            calls: list[tuple[str, str]] = []

            def fake_run_subagent(task_id, task_class, subagent_name, prompt_file_arg, output_file, workdir, route, subagent_cfg, dispatch_mode):
                calls.append((subagent_name, dispatch_mode))
                if subagent_name == "qwen_cli":
                    return {
                        "subagent": "qwen_cli",
                        "status": "failure",
                        "output_file": str(qwen_output),
                        "error_text": "missing payload",
                    }
                return {
                    "subagent": "internal_subagents",
                    "status": "success",
                    "output_file": str(internal_output),
                    "error_text": "",
                }

            with mock.patch.object(
                self.dispatch.subagent_system,
                "route_subagent",
                return_value={
                    "task_class": "coach",
                    "dispatch_policy": {
                        "direct_internal_bypass_forbidden": "yes",
                        "internal_escalation_allowed": "yes",
                        "internal_route_authorized": "no",
                    },
                    "analysis_plan": {"required": "no", "receipt_required": "no", "route_task_class": ""},
                    "route_budget": {},
                    "fanout_subagents": [],
                    "fallback_subagents": [],
                },
            ), mock.patch.object(self.dispatch, "run_subagent", side_effect=fake_run_subagent):
                manifest_path, manifest, decision = self.dispatch.run_coach_ensemble(
                    task_id="unit-task",
                    writer_task_class="implementation",
                    coach_task_class="coach",
                    prompt_file=prompt_file,
                    output_dir=tmp_path / "coach",
                    workdir=ROOT_DIR,
                    snapshot={
                        "subagents": {
                            "qwen_cli": {"dispatch": {"command": "qwen"}},
                            "internal_subagents": {"dispatch": {"command": "internal"}},
                        }
                    },
                    route={"task_class": "implementation"},
                    coach_plan={
                        "selected_subagents": ["qwen_cli"],
                        "fallback_subagents": [{"subagent": "internal_subagents"}],
                        "min_results": 1,
                        "merge_policy": "unanimous_approve_rework_bias",
                    },
                )

        self.assertEqual(manifest["manifest_path"], str(manifest_path))
        self.assertEqual(calls, [("qwen_cli", "fanout")])
        self.assertEqual(manifest["status"], "coach_failed")
        self.assertEqual(decision["coach_decision"], "coach_failed")
        self.assertIn("insufficient_valid_coach_results:0/1", decision["invalid_reasons"])

    def test_parse_arbitration_decision_prefers_last_json_object(self) -> None:
        text = """
Draft:
{"decision":"no_decision","selected_cluster_id":"","confidence":"low","rationale":"draft"}

Final:
{"decision":"select_cluster","selected_cluster_id":"abc123","confidence":"high","rationale":"final"}
"""

        payload = self.dispatch.parse_arbitration_decision(text, ["abc123"])

        self.assertEqual(payload["decision"], "select_cluster")
        self.assertEqual(payload["selected_cluster_id"], "abc123")
        self.assertEqual(payload["confidence"], "high")

    def test_validate_arbitration_output_rejects_non_json_text(self) -> None:
        errors = self.dispatch.validate_arbitration_output_text(
            "I think cluster abc123 is best.",
            ["abc123"],
        )

        self.assertIn("arbitration output must be valid JSON", errors)


if __name__ == "__main__":
    unittest.main()
