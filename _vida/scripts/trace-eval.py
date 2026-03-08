#!/usr/bin/env python3
"""Local trace grading and dataset export for VIDA routed runs."""

from __future__ import annotations

import json
import importlib.util
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
ROUTE_RECEIPT_DIR = ROOT_DIR / ".vida" / "logs" / "route-receipts"
RUN_GRAPH_DIR = ROOT_DIR / ".vida" / "state" / "run-graphs"
TRACE_EVAL_DIR = ROOT_DIR / ".vida" / "logs" / "trace-evals"
TRACE_DATASET_DIR = ROOT_DIR / ".vida" / "logs" / "trace-datasets"
EVAL_PACK_SCRIPT = SCRIPT_DIR / "eval-pack.sh"

GRADE_ORDER = ("route_correctness", "fallback_correctness", "budget_correctness", "approval_correctness")


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


run_graph_runtime = load_module("vida_trace_eval_run_graph", SCRIPT_DIR / "run-graph.py")


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default


def save_json(path: Path, payload: Any) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def run_eval_pack(task_id: str) -> dict[str, Any]:
    out_path = ROOT_DIR / ".vida" / "logs" / f"eval-pack-{task_id}.json"
    completed = subprocess.run(
        ["bash", str(EVAL_PACK_SCRIPT), "run", task_id],
        cwd=str(ROOT_DIR),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if completed.returncode != 0 and not out_path.exists():
        raise RuntimeError(completed.stderr.strip() or "eval-pack.sh failed")
    return load_json(out_path, {})


def collect_route_receipts(task_id: str) -> list[dict[str, Any]]:
    receipts: list[dict[str, Any]] = []
    for path in sorted(ROUTE_RECEIPT_DIR.glob(f"{task_id}.*.json")):
        payload = load_json(path, {})
        if not isinstance(payload, dict):
            continue
        payload["_path"] = str(path)
        receipts.append(payload)
    return receipts


def load_run_graph(task_id: str) -> dict[str, Any]:
    return load_json(RUN_GRAPH_DIR / f"{task_id}.json", {})


def compact_receipt(receipt: dict[str, Any]) -> dict[str, Any]:
    route_receipt = receipt.get("route_receipt")
    route = route_receipt if isinstance(route_receipt, dict) else {}
    dispatch_policy = route.get("dispatch_policy") if isinstance(route.get("dispatch_policy"), dict) else {}
    route_budget = route.get("route_budget") if isinstance(route.get("route_budget"), dict) else {}
    return {
        "path": receipt.get("_path", ""),
        "status": str(receipt.get("status", "")).strip(),
        "task_class": str(receipt.get("task_class", "")).strip(),
        "review_state": str(receipt.get("review_state", "")).strip(),
        "decision": str(receipt.get("decision", "")).strip(),
        "dispatch_required": str(route.get("dispatch_required", "")).strip(),
        "independent_verification_required": str(route.get("independent_verification_required", "")).strip(),
        "direct_internal_bypass_forbidden": str(dispatch_policy.get("direct_internal_bypass_forbidden", "")).strip(),
        "internal_route_authorized": str(dispatch_policy.get("internal_route_authorized", "")).strip(),
        "local_execution_allowed": str(dispatch_policy.get("local_execution_allowed", "")).strip(),
        "budget_violation": bool(receipt.get("budget_violation", False)),
        "internal_escalation_used": bool(receipt.get("internal_escalation_used", False)),
        "internal_escalation_receipt_error": str(receipt.get("internal_escalation_receipt_error", "")).strip(),
        "max_budget_units": int(route_budget.get("max_budget_units", 0) or 0),
    }


def node_status(run_graph: dict[str, Any], node: str) -> str:
    nodes = run_graph.get("nodes")
    if not isinstance(nodes, dict):
        return ""
    entry = nodes.get(node)
    if not isinstance(entry, dict):
        return ""
    return str(entry.get("status", "")).strip()


def grade_route_correctness(receipts: list[dict[str, Any]], run_graph: dict[str, Any]) -> dict[str, Any]:
    route_receipts = [item for item in receipts if item.get("route_receipt")]
    if not route_receipts:
        if isinstance(run_graph.get("nodes"), dict):
            return {"grade": "partial", "reason": "non_routed_or_framework_local_trace"}
        return {"grade": "partial", "reason": "non_routed_task_without_trace_artifacts"}
    route = route_receipts[0].get("route_receipt", {})
    if not isinstance(route, dict):
        return {"grade": "fail", "reason": "invalid_route_receipt"}
    required_keys = [
        "dispatch_required",
        "analysis_required",
        "coach_required",
        "independent_verification_required",
        "dispatch_policy",
        "route_graph",
    ]
    missing = [key for key in required_keys if key not in route]
    if missing:
        return {"grade": "fail", "reason": "missing_route_keys", "missing_keys": missing}
    analysis = node_status(run_graph, "analysis")
    writer = node_status(run_graph, "writer")
    if analysis in {"", "pending"} and writer in {"ready", "running", "completed"}:
        return {"grade": "fail", "reason": "writer_advanced_without_analysis_trace"}
    return {"grade": "pass", "reason": "route_receipt_and_run_graph_present"}


def grade_fallback_correctness(receipts: list[dict[str, Any]]) -> dict[str, Any]:
    for item in receipts:
        compact = compact_receipt(item)
        if compact["internal_escalation_used"] and compact["internal_escalation_receipt_error"]:
            return {"grade": "fail", "reason": "internal_escalation_without_valid_receipt", "receipt": compact}
        if (
            compact["internal_escalation_used"]
            and compact["direct_internal_bypass_forbidden"] == "yes"
            and compact["internal_route_authorized"] != "yes"
        ):
            return {"grade": "fail", "reason": "internal_bypass_detected", "receipt": compact}
    return {"grade": "pass", "reason": "no_invalid_fallback_or_internal_bypass"}


def grade_budget_correctness(receipts: list[dict[str, Any]]) -> dict[str, Any]:
    violations = [compact_receipt(item) for item in receipts if compact_receipt(item)["budget_violation"]]
    if violations:
        return {"grade": "fail", "reason": "budget_violation_detected", "violations": violations}
    return {"grade": "pass", "reason": "no_budget_violation_detected"}


def grade_approval_correctness(receipts: list[dict[str, Any]], run_graph: dict[str, Any]) -> dict[str, Any]:
    approval = node_status(run_graph, "approval")
    synthesis = node_status(run_graph, "synthesis")
    approval_receipts = [item for item in receipts if str(item.get("review_state", "")).strip()]
    if approval in {"completed", "blocked", "failed"} and not approval_receipts:
        return {"grade": "fail", "reason": "approval_node_without_receipt"}
    if synthesis == "completed" and approval == "blocked":
        return {"grade": "fail", "reason": "synthesis_completed_while_approval_blocked"}
    if not approval_receipts and approval in {"", "pending"}:
        return {"grade": "partial", "reason": "approval_not_reached"}
    return {"grade": "pass", "reason": "approval_trace_consistent"}


def overall_grade(grades: dict[str, dict[str, Any]]) -> str:
    ordered = [grades[key]["grade"] for key in GRADE_ORDER if key in grades]
    if any(item == "fail" for item in ordered):
        return "fail"
    if any(item == "partial" for item in ordered):
        return "partial"
    return "pass"


def build_trace_eval(task_id: str) -> dict[str, Any]:
    receipts = collect_route_receipts(task_id)
    run_graph = load_run_graph(task_id)
    run_graph_status = run_graph_runtime.status_payload(task_id)
    eval_pack = run_eval_pack(task_id)
    grades = {
        "route_correctness": grade_route_correctness(receipts, run_graph),
        "fallback_correctness": grade_fallback_correctness(receipts),
        "budget_correctness": grade_budget_correctness(receipts),
        "approval_correctness": grade_approval_correctness(receipts, run_graph),
    }
    return {
        "generated_at": now_utc(),
        "task_id": task_id,
        "overall_grade": overall_grade(grades),
        "grades": grades,
        "run_graph_path": str(RUN_GRAPH_DIR / f"{task_id}.json"),
        "route_receipt_count": len(receipts),
        "route_receipts": [compact_receipt(item) for item in receipts],
        "run_graph_resume_hint": run_graph_status.get("resume_hint", {}),
        "eval_pack_path": str(ROOT_DIR / ".vida" / "logs" / f"eval-pack-{task_id}.json"),
        "eval_pack": {
            "task_status": eval_pack.get("task_status"),
            "block_total": eval_pack.get("block_total"),
            "block_success_rate": eval_pack.get("block_success_rate"),
            "compact_recovery_scenario": eval_pack.get("compact_recovery_scenario"),
        },
    }


def build_trace_dataset(task_id: str, trace_eval: dict[str, Any]) -> dict[str, Any]:
    return {
        "generated_at": now_utc(),
        "task_id": task_id,
        "dataset_version": "v1",
        "labels": {
            "overall_grade": trace_eval.get("overall_grade", "partial"),
            "route_correctness": trace_eval.get("grades", {}).get("route_correctness", {}).get("grade", "partial"),
            "fallback_correctness": trace_eval.get("grades", {}).get("fallback_correctness", {}).get("grade", "partial"),
            "budget_correctness": trace_eval.get("grades", {}).get("budget_correctness", {}).get("grade", "partial"),
            "approval_correctness": trace_eval.get("grades", {}).get("approval_correctness", {}).get("grade", "partial"),
        },
        "artifacts": {
            "trace_eval_path": str(TRACE_EVAL_DIR / f"trace-eval-{task_id}.json"),
            "run_graph_path": trace_eval.get("run_graph_path", ""),
            "eval_pack_path": trace_eval.get("eval_pack_path", ""),
            "route_receipts": [item.get("path", "") for item in trace_eval.get("route_receipts", [])],
        },
        "summary": {
            "route_receipt_count": trace_eval.get("route_receipt_count", 0),
            "resume_hint": trace_eval.get("run_graph_resume_hint", {}),
            "eval_pack": trace_eval.get("eval_pack", {}),
        },
    }


def grade_command(task_id: str) -> int:
    payload = build_trace_eval(task_id)
    path = save_json(TRACE_EVAL_DIR / f"trace-eval-{task_id}.json", payload)
    print(str(path))
    return 0


def dataset_command(task_id: str) -> int:
    trace_eval = build_trace_eval(task_id)
    trace_eval_path = save_json(TRACE_EVAL_DIR / f"trace-eval-{task_id}.json", trace_eval)
    dataset = build_trace_dataset(task_id, trace_eval)
    dataset["artifacts"]["trace_eval_path"] = str(trace_eval_path)
    path = save_json(TRACE_DATASET_DIR / f"trace-dataset-{task_id}.json", dataset)
    print(str(path))
    return 0


def usage() -> int:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/trace-eval.py grade <task_id>\n"
        "  python3 _vida/scripts/trace-eval.py dataset <task_id>",
        file=sys.stderr,
    )
    return 2


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        return usage()
    command, task_id = argv[1], argv[2]
    if command == "grade":
        return grade_command(task_id)
    if command == "dataset":
        return dataset_command(task_id)
    return usage()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
