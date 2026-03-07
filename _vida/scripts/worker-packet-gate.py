#!/usr/bin/env python3
"""Validate VIDA worker packets and machine-readable worker outputs."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


SUBAGENT_ENTRY_DOC = "_vida/docs/SUBAGENT-ENTRY.MD"
SUBAGENT_THINKING_DOC = "_vida/docs/SUBAGENT-THINKING.MD"
REQUIRED_PACKET_MARKERS = {
    "worker_lane_confirmed: true": "missing worker_lane_confirmed marker",
    "worker_role: subagent": "missing worker_role marker",
    f"worker_entry: {SUBAGENT_ENTRY_DOC}": "missing worker_entry marker",
    f"worker_thinking: {SUBAGENT_THINKING_DOC}": "missing worker_thinking marker",
    "impact_tail_policy: required_for_non_stc": "missing impact_tail_policy marker",
    "impact_analysis_scope: bounded_to_assigned_scope": "missing impact_analysis_scope marker",
}
PLACEHOLDER_PREFLIGHT = "<active project preflight doc from overlay>"
PLACEHOLDER_BLOCKING_QUESTION = "[provide one explicit blocking question for this worker lane]"
TOP_LEVEL_OUTPUT_KEYS = {
    "status",
    "question_answered",
    "answer",
    "evidence_refs",
    "changed_files",
    "verification_commands",
    "verification_results",
    "merge_ready",
    "blockers",
    "notes",
    "recommended_next_action",
    "impact_analysis",
}
IMPACT_ANALYSIS_KEYS = {
    "affected_scope",
    "contract_impact",
    "follow_up_actions",
    "residual_risks",
}
YES_NO_VALUES = {"yes", "no"}
STATUS_VALUES = {"done", "partial", "blocked"}


def _is_string_list(value: Any) -> bool:
    return isinstance(value, list) and all(isinstance(item, str) for item in value)


def load_text(path_or_dash: str) -> str:
    if path_or_dash == "-":
        return sys.stdin.read()
    return Path(path_or_dash).read_text(encoding="utf-8")


def machine_readable_contract_required(prompt_text: str) -> bool:
    return "return the machine-readable summary below" in prompt_text.casefold()


def extract_balanced_json_prefix(text: str) -> str | None:
    if not text:
        return None
    opener = text[0]
    if opener != "{":
        return None
    closer = "}"
    depth = 0
    in_string = False
    escape_next = False
    for index, char in enumerate(text):
        if escape_next:
            escape_next = False
            continue
        if char == "\\" and in_string:
            escape_next = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char == opener:
            depth += 1
        elif char == closer:
            depth -= 1
            if depth == 0:
                return text[: index + 1]
    return None


def extract_json_payload(text: str) -> dict[str, Any] | None:
    stripped = text.strip()
    if not stripped:
        return None
    candidates: list[str] = []
    fenced = re.findall(r"```json\s*(\{.*?\})\s*```", text, flags=re.DOTALL)
    candidates.extend(reversed(fenced))
    balanced_candidates: list[str] = []
    for match in re.finditer(r"\{", text):
        candidate = extract_balanced_json_prefix(text[match.start() :])
        if candidate:
            balanced_candidates.append(candidate)
    candidates.extend(reversed(balanced_candidates))
    if stripped.startswith("{") and stripped.endswith("}"):
        candidates.append(stripped)
    best_payload: dict[str, Any] | None = None
    best_score = -1
    for candidate in candidates:
        try:
            payload = json.loads(candidate)
        except json.JSONDecodeError:
            continue
        if isinstance(payload, dict):
            overlap = len(set(payload.keys()) & TOP_LEVEL_OUTPUT_KEYS)
            score = overlap * 1000 + len(candidate)
            if score > best_score:
                best_payload = payload
                best_score = score
    return best_payload


def validate_packet_text(text: str) -> list[str]:
    errors: list[str] = []
    for marker, message in REQUIRED_PACKET_MARKERS.items():
        if marker not in text:
            errors.append(message)
    if "Runtime Role Packet:" not in text:
        errors.append("missing Runtime Role Packet section")
    if re.search(r"^Scope:\s*\S.*$", text, flags=re.MULTILINE) is None:
        errors.append("missing Scope section")
    if re.search(r"^Verification:\s*(?:\S.*)?$", text, flags=re.MULTILINE) is None:
        errors.append("missing Verification section")
    if re.search(r"^Deliverable:\s*(?:\S.*)?$", text, flags=re.MULTILINE) is None:
        errors.append("missing Deliverable section")
    blocking_question_match = re.search(r"^Blocking Question:\s*(.+)$", text, flags=re.MULTILINE)
    if blocking_question_match is None:
        errors.append("missing Blocking Question section")
    else:
        question = blocking_question_match.group(1).strip()
        if not question or question == PLACEHOLDER_BLOCKING_QUESTION:
            errors.append("blocking question must be explicit and non-placeholder")
    if PLACEHOLDER_PREFLIGHT in text:
        errors.append("project preflight placeholder must be resolved from overlay")
    if machine_readable_contract_required(text):
        payload = extract_json_payload(text)
        if payload is None:
            errors.append("missing machine-readable contract example")
        else:
            errors.extend(validate_output_payload(payload, prefix="machine-readable contract"))
    return errors


def validate_output_payload(payload: dict[str, Any], *, prefix: str) -> list[str]:
    errors: list[str] = []
    for key in sorted(TOP_LEVEL_OUTPUT_KEYS):
        if key not in payload:
            errors.append(f"{prefix} missing key: {key}")
    if "answer" in payload and not isinstance(payload.get("answer"), str):
        errors.append(f"{prefix} answer must be a string")
    if "notes" in payload and not isinstance(payload.get("notes"), str):
        errors.append(f"{prefix} notes must be a string")
    if "recommended_next_action" in payload and not isinstance(payload.get("recommended_next_action"), str):
        errors.append(f"{prefix} recommended_next_action must be a string")
    for key in ("evidence_refs", "changed_files", "verification_commands", "verification_results", "blockers"):
        if key in payload and not _is_string_list(payload.get(key)):
            errors.append(f"{prefix} {key} must be a list of strings")
    impact_analysis = payload.get("impact_analysis")
    if isinstance(impact_analysis, dict):
        for key in sorted(IMPACT_ANALYSIS_KEYS):
            if key not in impact_analysis:
                errors.append(f"{prefix} impact_analysis missing key: {key}")
            elif not _is_string_list(impact_analysis.get(key)):
                errors.append(f"{prefix} impact_analysis {key} must be a list of strings")
    elif "impact_analysis" in payload:
        errors.append(f"{prefix} impact_analysis must be an object")

    status = payload.get("status")
    if isinstance(status, str) and status not in STATUS_VALUES:
        errors.append(f"{prefix} status must be one of {sorted(STATUS_VALUES)}")
    question_answered = payload.get("question_answered")
    if isinstance(question_answered, str) and question_answered not in YES_NO_VALUES:
        errors.append(f"{prefix} question_answered must be one of {sorted(YES_NO_VALUES)}")
    merge_ready = payload.get("merge_ready")
    if isinstance(merge_ready, str) and merge_ready not in YES_NO_VALUES:
        errors.append(f"{prefix} merge_ready must be one of {sorted(YES_NO_VALUES)}")
    return errors


def validate_output_text(prompt_text: str, output_text: str) -> list[str]:
    if not machine_readable_contract_required(prompt_text):
        return []
    payload = extract_json_payload(output_text)
    if payload is None:
        return ["worker output must be valid JSON when the prompt requires a machine-readable summary"]
    return validate_output_payload(payload, prefix="machine-readable output")


def usage() -> str:
    return (
        "Usage:\n"
        "  python3 _vida/scripts/worker-packet-gate.py check <prompt_file|- >\n"
        "  python3 _vida/scripts/worker-packet-gate.py check-output <prompt_file|-> <output_file|->\n"
    )


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(usage(), file=sys.stderr)
        return 1
    cmd = argv[1]
    if cmd == "check":
        text = load_text(argv[2])
        errors = validate_packet_text(text)
        print(json.dumps({"status": "ok" if not errors else "blocked", "errors": errors}, indent=2))
        return 0 if not errors else 2
    if cmd == "check-output":
        if len(argv) < 4:
            print(usage(), file=sys.stderr)
            return 1
        prompt_text = load_text(argv[2])
        output_text = load_text(argv[3])
        errors = validate_output_text(prompt_text, output_text)
        print(json.dumps({"status": "ok" if not errors else "blocked", "errors": errors}, indent=2))
        return 0 if not errors else 2
    print(usage(), file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
