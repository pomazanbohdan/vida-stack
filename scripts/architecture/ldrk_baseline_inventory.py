#!/usr/bin/env python3
"""Generate the LDRK runtime baseline inventory for TaskFlow task ldr-001."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


TARGET_ROOTS = [
    "crates/taskflow-authority",
    "crates/taskflow-contracts",
    "crates/taskflow-core",
    "crates/taskflow-state",
    "crates/vida",
    "crates/vida-contracts",
]

LDR074_THRESHOLDS = {
    "targeted_production_loc": 182431,
    "duplicate_classifier_candidates": 479,
    "canonical_cli_leaf_command_candidates": 96,
    "command_specific_option_candidates": 263,
    "surface_direct_mutation_candidates": 0,
}

SURFACE_MUTATION_PATH_PATTERNS = [
    re.compile(pattern)
    for pattern in [
        r"crates/vida/src/.*(?:cli|surface|router|transport|tui).*\.rs$",
        r"crates/vida/src/main\.rs$",
    ]
]

SURFACE_AUTHORITY_ADAPTER_PATH_PATTERNS = [
    re.compile(pattern)
    for pattern in [
        r"crates/vida/src/(?:agent_dispatch|approval|doctor|init|lane|project_activator|session|status|task)_surface.*\.rs$",
        r"crates/vida/src/task_cli_render\.rs$",
    ]
]

SURFACE_DIRECT_PATH_PATTERNS = [
    re.compile(pattern)
    for pattern in [
        r"crates/vida/src/(?:cli|main|root_command_router|.*transport.*|.*tui.*)\.rs$",
    ]
]

MUTATION_CALL_PATTERN = re.compile(
    r"(?:\.|::|\b)(write(?:_string)?|create(?:_dir_all)?|remove_(?:file|dir|dir_all)|rename|copy|append|insert|update|upsert|delete|set_\w+|save|persist|record)\s*\("
)

RUNTIME_ENTITY_PATTERNS = {
    "run_graph_state": [r"run[_-]?graph", r"RunGraph"],
    "dispatch_receipt": [r"dispatch[_-]?receipt", r"DispatchReceipt"],
    "lane_packet": [r"lane", r"Lane"],
    "host_bridge_artifact": [r"host[_-]?bridge", r"HostBridge"],
    "continuation_binding": [r"continuation[_-]?binding", r"ContinuationBinding"],
    "claim": [r"\bclaim", r"Claim"],
    "task_record": [r"task[_-]?record", r"TaskRecord", r"\btask\b"],
}

MUTATION_PATTERNS = [
    r"\bwrite(?:_string)?\b",
    r"\bcreate(?:_dir_all)?\b",
    r"\bremove_(?:file|dir|dir_all)\b",
    r"\brename\b",
    r"\bcopy\b",
    r"\bappend\b",
    r"\binsert\b",
    r"\bupdate\b",
    r"\bupsert\b",
    r"\bdelete\b",
    r"\bset_\w+\b",
    r"\bsave\b",
    r"\bpersist\b",
    r"\brecord\b",
]

STATUS_LITERAL_PATTERN = re.compile(
    r'"([^"\n]*(?:pass|blocked|completed|retryable|failed|ready|running)[^"\n]*)"',
    re.IGNORECASE,
)
FUNCTION_PATTERN = re.compile(
    r"\bfn\s+([A-Za-z0-9_]*(?:classif|verdict|status|block|complete|retry|pass|fail)[A-Za-z0-9_]*)\s*\("
)
CFG_TEST_ATTR_PATTERN = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
CFG_TEST_ITEM_START_PATTERN = re.compile(
    r"\b(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:mod|fn)\s+[A-Za-z0-9_]+\b"
)
OUTCOME_CLASSIFIER_NAME_PATTERN = re.compile(
    r"(^|_)(classif[A-Za-z0-9_]*|verdict|outcome|decision|blocker_code|blocker_codes|fail_closed|terminal|retryable|is|has)($|_)"
    r"|^(is|has|blocked|completed|terminal|retry|fail|pass)_"
)
COMMAND_PATTERN = re.compile(r"\b(?:Command|ClapCommand)::new\(\s*\"([^\"]+)\"")
ARG_PATTERN = re.compile(r"\bArg::new\(\s*\"([^\"]+)\"")
SUBCOMMAND_ATTR_PATTERN = re.compile(r"#\s*\[\s*command\s*\(")
ARG_ATTR_PATTERN = re.compile(r"#\s*\[\s*arg\s*\(")
ARG_LONG_PATTERN = re.compile(r"long(?:\s*=\s*\"([^\"]+)\")?")
ENUM_PATTERN = re.compile(r"\b(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Za-z0-9_]+)\s*\{")
ENUM_VARIANT_PATTERN = re.compile(r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:\(|,|$)")
STRUCT_FIELD_PATTERN = re.compile(r"^\s*pub(?:\([^)]*\))?\s+([a-zA-Z0-9_]+)\s*:")

EXCLUDED_PARTS = {
    ".git",
    "target",
    "dist",
    "tmp",
    "temp",
    "tests",
    "testdata",
    "fixtures",
    "fixture",
    "snapshots",
    "generated",
}

EXCLUDED_SUFFIXES = {
    ".lock",
    ".snap",
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".zip",
    ".exe",
    ".dll",
    ".pdb",
}


@dataclass(frozen=True)
class SourceFile:
    path: Path
    rel: str


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def norm(path: Path) -> str:
    return path.as_posix()


def is_excluded(path: Path) -> bool:
    parts = set(path.parts)
    if parts & EXCLUDED_PARTS:
        return True
    if path.suffix.lower() in EXCLUDED_SUFFIXES:
        return True
    name = path.name.lower()
    return name.endswith(("_generated.rs", ".generated.rs"))


def iter_source_files(root: Path) -> list[SourceFile]:
    files: list[SourceFile] = []
    for target in TARGET_ROOTS:
        base = root / target
        if not base.exists():
            continue
        for path in base.rglob("*"):
            rel_path = path.relative_to(root)
            if path.is_file() and path.suffix == ".rs" and not is_excluded(rel_path):
                files.append(SourceFile(path=path, rel=norm(rel_path)))
    return sorted(files, key=lambda item: item.rel)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def iter_rust_lines_by_test_scope(text: str) -> Iterable[tuple[int, str, bool]]:
    brace_depth = 0
    test_scope_depth: int | None = None
    pending_cfg_test = False
    pending_cfg_test_item = False
    for index, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if CFG_TEST_ATTR_PATTERN.search(line):
            pending_cfg_test = True
        if pending_cfg_test and CFG_TEST_ITEM_START_PATTERN.search(line):
            pending_cfg_test_item = True

        is_test_scoped = (
            test_scope_depth is not None or pending_cfg_test or pending_cfg_test_item
        )
        yield index, line, is_test_scoped

        open_count = line.count("{")
        close_count = line.count("}")
        if pending_cfg_test_item and open_count > 0 and test_scope_depth is None:
            test_scope_depth = brace_depth + 1
            pending_cfg_test = False
            pending_cfg_test_item = False

        brace_depth += open_count - close_count
        if test_scope_depth is not None and brace_depth < test_scope_depth:
            test_scope_depth = None
            pending_cfg_test = False
            pending_cfg_test_item = False
        elif pending_cfg_test_item and ";" in line and open_count == 0:
            pending_cfg_test = False
            pending_cfg_test_item = False
        elif (
            pending_cfg_test
            and not pending_cfg_test_item
            and stripped
            and not stripped.startswith("#")
            and not stripped.startswith("//")
        ):
            pending_cfg_test = False


def is_outcome_classifier_name(name: str) -> bool:
    return bool(OUTCOME_CLASSIFIER_NAME_PATTERN.search(name))


def production_loc(files: Iterable[SourceFile]) -> dict[str, object]:
    total = 0
    by_root: dict[str, int] = {target: 0 for target in TARGET_ROOTS}
    by_file: list[dict[str, object]] = []
    for item in files:
        count = 0
        in_block = False
        for _, raw_line, is_test_scoped in iter_rust_lines_by_test_scope(read_text(item.path)):
            if is_test_scoped:
                continue
            line = raw_line.strip()
            if not line:
                continue
            if in_block:
                if "*/" in line:
                    in_block = False
                    line = line.split("*/", 1)[1].strip()
                else:
                    continue
            if line.startswith("/*"):
                if "*/" not in line:
                    in_block = True
                continue
            if line.startswith("//"):
                continue
            count += 1
        total += count
        target_root = next((target for target in TARGET_ROOTS if item.rel.startswith(target)), "unknown")
        by_root[target_root] = by_root.get(target_root, 0) + count
        by_file.append({"path": item.rel, "production_loc": count})
    return {
        "total_production_loc": total,
        "target_roots": [{"root": key, "production_loc": by_root[key]} for key in sorted(by_root)],
        "files": by_file,
    }


def first_matching_entities(text: str) -> list[str]:
    entities: list[str] = []
    for entity, patterns in RUNTIME_ENTITY_PATTERNS.items():
        if any(re.search(pattern, text, flags=re.IGNORECASE) for pattern in patterns):
            entities.append(entity)
    return sorted(entities)


def inventory_mutations(files: Iterable[SourceFile]) -> list[dict[str, object]]:
    mutation_regex = re.compile("|".join(f"(?:{pattern})" for pattern in MUTATION_PATTERNS))
    records: list[dict[str, object]] = []
    for item in files:
        lines = read_text(item.path).splitlines()
        for index, line in enumerate(lines, start=1):
            if not mutation_regex.search(line):
                continue
            entities = first_matching_entities(line)
            if not entities:
                context = "\n".join(lines[max(0, index - 3) : min(len(lines), index + 2)])
                entities = first_matching_entities(context)
            if not entities:
                continue
            records.append(
                {
                    "p": item.rel,
                    "l": index,
                    "e": entities,
                    "op": mutation_regex.search(line).group(0),
                    "owner": item.rel.rsplit("/", 1)[0],
                    "src": line.strip()[:220],
                }
            )
    return sorted(records, key=lambda row: (row["p"], row["l"], row["op"]))


def surface_mutation_records(records: Iterable[dict[str, object]]) -> list[dict[str, object]]:
    return [
        row
        for row in records
        if any(pattern.search(str(row["p"])) for pattern in SURFACE_MUTATION_PATH_PATTERNS)
    ]


def classify_surface_mutation_record(row: dict[str, object]) -> dict[str, object]:
    path = str(row["p"])
    source = str(row.get("src", ""))
    is_call = bool(MUTATION_CALL_PATTERN.search(source))
    classified = dict(row)
    if not is_call:
        classification = "lexical_reference"
    elif any(pattern.search(path) for pattern in SURFACE_AUTHORITY_ADAPTER_PATH_PATTERNS):
        classification = "authority_owned_surface_adapter"
    elif "Command::new" in source or "ClapCommand::new" in source:
        classification = "subprocess_or_cli_builder"
    elif any(pattern.search(path) for pattern in SURFACE_DIRECT_PATH_PATTERNS):
        classification = "unresolved_direct_surface_mutation"
    else:
        classification = "surface_diagnostic_or_artifact_io"
    classified["classification"] = classification
    return classified


def classified_surface_mutation_records(
    records: Iterable[dict[str, object]],
) -> list[dict[str, object]]:
    return [classify_surface_mutation_record(row) for row in surface_mutation_records(records)]


def classification_counts(records: Iterable[dict[str, object]]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for row in records:
        key = str(row.get("classification", "unknown"))
        counts[key] = counts.get(key, 0) + 1
    return dict(sorted(counts.items()))


def ldr074_gate_status(
    *,
    loc: dict[str, object],
    mutations: list[dict[str, object]],
    literals: dict[str, object],
    commands: dict[str, object],
) -> dict[str, object]:
    surface_mutations = classified_surface_mutation_records(mutations)
    unresolved_surface_mutations = [
        row
        for row in surface_mutations
        if row["classification"] == "unresolved_direct_surface_mutation"
    ]
    metrics = {
        "targeted_production_loc": loc["total_production_loc"],
        "duplicate_classifier_candidates": len(literals["classifier_functions"]),
        "canonical_cli_leaf_command_candidates": commands["leaf_command_count"],
        "command_specific_option_candidates": commands["command_specific_option_count"],
        "surface_direct_mutation_candidates": len(unresolved_surface_mutations),
        "surface_lexical_mutation_candidates": len(surface_mutations),
        "all_runtime_lexical_mutation_candidates": len(mutations),
    }
    gate_rows = []
    for metric, threshold in LDR074_THRESHOLDS.items():
        value = int(metrics[metric])
        gate_rows.append(
            {
                "metric": metric,
                "value": value,
                "threshold": threshold,
                "status": "pass" if value <= threshold else "fail",
            }
        )
    return {
        "gate": "ldr-074-final-metrics",
        "status": "pass"
        if all(row["status"] == "pass" for row in gate_rows)
        else "fail",
        "thresholds": LDR074_THRESHOLDS,
        "metrics": metrics,
        "gate_rows": gate_rows,
        "surface_mutation_record_count": len(surface_mutations),
        "surface_mutation_classification_counts": classification_counts(surface_mutations),
        "surface_unresolved_direct_mutation_records": unresolved_surface_mutations,
        "surface_mutation_records": surface_mutations,
        "classification": "fixed" if not unresolved_surface_mutations else "partially_fixed",
        "next_slices": [
            "Close ldr-074c after validator confirms the direct surface mutation gate classification.",
            "Close ldr-074 after final proof bundle and release/self-diagnostic gates pass.",
        ],
    }


def inventory_literals_and_classifiers(files: Iterable[SourceFile]) -> dict[str, object]:
    literals: dict[str, dict[str, object]] = {}
    cfg_test_literals: dict[str, dict[str, object]] = {}
    classifiers: list[dict[str, object]] = []
    cfg_test_classifiers: list[dict[str, object]] = []
    status_helpers: list[dict[str, object]] = []
    cfg_test_status_helpers: list[dict[str, object]] = []
    for item in files:
        for index, line, is_test_scoped in iter_rust_lines_by_test_scope(read_text(item.path)):
            target_literals = cfg_test_literals if is_test_scoped else literals
            for match in STATUS_LITERAL_PATTERN.finditer(line):
                value = match.group(1)
                key = value.lower()
                entry = target_literals.setdefault(
                    key,
                    {"literal": value, "count": 0, "locations": []},
                )
                entry["count"] = int(entry["count"]) + 1
                locations = entry["locations"]
                assert isinstance(locations, list)
                if len(locations) < 3:
                    locations.append({"path": item.rel, "line": index})
            fn_match = FUNCTION_PATTERN.search(line)
            if fn_match:
                row = {
                    "p": item.rel,
                    "l": index,
                    "fn": fn_match.group(1),
                }
                if is_outcome_classifier_name(fn_match.group(1)):
                    target_classifiers = cfg_test_classifiers if is_test_scoped else classifiers
                    target_classifiers.append(row)
                else:
                    target_helpers = (
                        cfg_test_status_helpers if is_test_scoped else status_helpers
                    )
                    target_helpers.append(row)
    return {
        "status_blocker_literals": [literals[key] for key in sorted(literals)],
        "classifier_functions": sorted(classifiers, key=lambda row: (row["p"], row["l"])),
        "status_helper_functions": sorted(
            status_helpers, key=lambda row: (row["p"], row["l"])
        ),
        "cfg_test_status_blocker_literals": [
            cfg_test_literals[key] for key in sorted(cfg_test_literals)
        ],
        "cfg_test_classifier_functions": sorted(
            cfg_test_classifiers, key=lambda row: (row["p"], row["l"])
        ),
        "cfg_test_status_helper_functions": sorted(
            cfg_test_status_helpers, key=lambda row: (row["p"], row["l"])
        ),
    }


def inventory_commands(files: Iterable[SourceFile]) -> dict[str, object]:
    command_records: list[dict[str, object]] = []
    option_records: list[dict[str, object]] = []
    root_command_variants: list[dict[str, object]] = []
    semantic_option_records: list[dict[str, object]] = []
    attr_subcommands = 0
    attr_options = 0
    for item in files:
        if "crates/vida" not in item.rel:
            continue
        current_enum: str | None = None
        enum_depth = 0
        pending_arg_attr = False
        pending_long_name: str | None = None
        for index, line in enumerate(read_text(item.path).splitlines(), start=1):
            enum_match = ENUM_PATTERN.search(line)
            if enum_match:
                current_enum = enum_match.group(1)
                enum_depth = line.count("{") - line.count("}")
                continue
            if current_enum:
                enum_depth += line.count("{") - line.count("}")
                variant_match = ENUM_VARIANT_PATTERN.search(line)
                if current_enum == "Command" and variant_match:
                    root_command_variants.append(
                        {
                            "path": item.rel,
                            "line": index,
                            "command": variant_match.group(1),
                        }
                    )
                if enum_depth <= 0:
                    current_enum = None
            for match in COMMAND_PATTERN.finditer(line):
                command_records.append({"path": item.rel, "line": index, "command": match.group(1)})
            for match in ARG_PATTERN.finditer(line):
                option_records.append({"path": item.rel, "line": index, "option": match.group(1)})
            if SUBCOMMAND_ATTR_PATTERN.search(line):
                attr_subcommands += 1
            if ARG_ATTR_PATTERN.search(line):
                attr_options += 1
                pending_arg_attr = True
                pending_long_name = None
            if pending_arg_attr:
                long_match = ARG_LONG_PATTERN.search(line)
                if long_match:
                    pending_long_name = long_match.group(1)
            if pending_arg_attr:
                field_match = STRUCT_FIELD_PATTERN.search(line)
                if field_match:
                    field_name = field_match.group(1)
                    semantic_option_records.append(
                        {
                            "path": item.rel,
                            "line": index,
                            "option": pending_long_name
                            or field_name.replace("_", "-"),
                        }
                    )
                    pending_arg_attr = False
                    pending_long_name = None
    option_counts: dict[str, int] = {}
    for row in option_records:
        option = str(row["option"])
        option_counts[option] = option_counts.get(option, 0) + 1
    for row in semantic_option_records:
        option = str(row["option"])
        option_counts[option] = option_counts.get(option, 0) + 1
    repeated_global_flags = [
        {"option": option, "count": count}
        for option, count in sorted(option_counts.items())
        if count > 1
    ]
    return {
        "parser": "semantic_root_plus_explicit_clap_commands_v2",
        "leaf_command_count": len(root_command_variants),
        "subprocess_command_name_count": len({row["command"] for row in command_records}),
        "command_specific_option_count": len(
            {
                str(row["option"])
                for row in [*option_records, *semantic_option_records]
            }
        ),
        "root_command_records": root_command_variants,
        "command_records": sorted(command_records, key=lambda row: (row["command"], row["path"], row["line"])),
        "semantic_option_records": sorted(
            semantic_option_records, key=lambda row: (row["option"], row["path"], row["line"])
        ),
        "option_records": sorted(option_records, key=lambda row: (row["option"], row["path"], row["line"])),
        "derive_command_attribute_count": attr_subcommands,
        "legacy_derive_attribute_leaf_candidate_count": attr_subcommands,
        "derive_arg_attribute_count": attr_options,
        "legacy_derive_attribute_option_candidate_count": attr_options,
        "repeated_global_flags": repeated_global_flags,
        "proposed_disposition": "keep canonical generic verbs; globalize repeated flags; convert task-specific knobs to payload fields or aliases during LDRK CLI reduction",
    }


def tool_availability(root: Path) -> list[dict[str, object]]:
    tools = ["rg", "python", "python3", "tokei", "scc", "cargo"]
    records = []
    for tool in tools:
        resolved = shutil.which(tool)
        version = None
        if resolved:
            try:
                proc = subprocess.run(
                    [resolved, "--version"],
                    cwd=root,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    timeout=5,
                    check=False,
                )
                version = proc.stdout.splitlines()[0].strip() if proc.stdout else None
            except Exception as error:  # pragma: no cover - defensive baseline metadata
                version = f"version_probe_failed:{type(error).__name__}"
        records.append({"tool": tool, "available": resolved is not None, "version": version})
    return records


def build_baseline(root: Path) -> dict[str, object]:
    files = iter_source_files(root)
    loc = production_loc(files)
    mutations = inventory_mutations(files)
    literals = inventory_literals_and_classifiers(files)
    commands = inventory_commands(files)
    final_gate = ldr074_gate_status(
        loc=loc,
        mutations=mutations,
        literals=literals,
        commands=commands,
    )
    return {
        "schema_version": "ldrk-baseline-v1",
        "task_id": "ldr-001",
        "generated_at_source": "stable_omitted_for_byte_reproducibility",
        "target_roots": TARGET_ROOTS,
        "tool_availability": tool_availability(root),
        "production_loc": loc,
        "direct_mutation_inventory": {
            "count": len(mutations),
            "record_schema": {
                "p": "path",
                "l": "line",
                "e": "runtime entity groups",
                "op": "operation hint",
                "owner": "owner module",
                "source_of_truth_assumption": "direct runtime mutation or artifact write discovered by lexical baseline",
                "replacement_operation": "route through VidaCommandEnvelope and OperationalJournal port before cutover",
            },
            "records": mutations,
        },
        "status_and_classifier_inventory": {
            "status_blocker_literal_count": len(literals["status_blocker_literals"]),
            "classifier_function_count": len(literals["classifier_functions"]),
            "status_helper_function_count": len(literals["status_helper_functions"]),
            "cfg_test_status_blocker_literal_count": len(
                literals["cfg_test_status_blocker_literals"]
            ),
            "cfg_test_classifier_function_count": len(literals["cfg_test_classifier_functions"]),
            "cfg_test_status_helper_function_count": len(
                literals["cfg_test_status_helper_functions"]
            ),
            "classifier_record_schema": {
                "p": "path",
                "l": "line",
                "fn": "function",
                "replacement_operation": "fold into shared CompletionOutcome/verdict contract",
            },
            **literals,
        },
        "command_inventory": commands,
        "success_metric_baseline": {
            "targeted_production_loc": loc["total_production_loc"],
            "duplicate_classifier_candidates": len(literals["classifier_functions"]),
            "direct_surface_mutation_candidates": final_gate["metrics"]["surface_direct_mutation_candidates"],
            "surface_lexical_mutation_candidates": final_gate["metrics"]["surface_lexical_mutation_candidates"],
            "canonical_cli_leaf_command_candidates": commands["leaf_command_count"],
            "command_specific_option_candidates": commands["command_specific_option_count"],
        },
        "ldr074_final_gate": final_gate,
    }


def stable_json(data: object) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def compact_json(data: object) -> str:
    return json.dumps(data, ensure_ascii=False, separators=(",", ":"), sort_keys=True) + "\n"


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def write_if_changed(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.read_text(encoding="utf-8") == body:
        return
    path.write_text(body, encoding="utf-8", newline="\n")


def markdown_table(headers: list[str], rows: list[list[object]]) -> str:
    output = ["| " + " | ".join(headers) + " |"]
    output.append("| " + " | ".join("---" for _ in headers) + " |")
    for row in rows:
        output.append("| " + " | ".join(str(cell).replace("\n", " ") for cell in row) + " |")
    return "\n".join(output)


def render_drift_map(baseline: dict[str, object]) -> str:
    mutations = baseline["direct_mutation_inventory"]["records"]  # type: ignore[index]
    classifiers = baseline["status_and_classifier_inventory"]["classifier_functions"]  # type: ignore[index]
    commands = baseline["command_inventory"]
    final_gate = baseline["ldr074_final_gate"]
    gate_rows = final_gate["gate_rows"]  # type: ignore[index]
    classification_counts_map = final_gate["surface_mutation_classification_counts"]  # type: ignore[index]
    classified_surface_rows = final_gate["surface_mutation_records"]  # type: ignore[index]
    surface_classification_by_location = {
        (row["p"], row["l"], row["op"]): row["classification"] for row in classified_surface_rows
    }
    top_mutations = mutations[:80]
    mutation_rows = [
        [
            row["p"],
            row["l"],
            ",".join(row["e"]),
            row["op"],
            surface_classification_by_location.get(
                (row["p"], row["l"], row["op"]),
                "all_runtime_lexical_outside_surface_gate",
            ),
            "route through VidaCommandEnvelope and OperationalJournal port before cutover",
        ]
        for row in top_mutations
    ]
    classifier_rows = [
        [row["p"], row["l"], row["fn"], "fold into shared CompletionOutcome/verdict contract"]
        for row in classifiers[:80]
    ]
    body = [
        "# LDRK Baseline Drift Map",
        "",
        "Status: generated baseline artifact for TaskFlow task `ldr-001`.",
        "",
        "## Summary",
        "",
        markdown_table(
            ["Metric", "Value"],
            [
                ["targeted_production_loc", baseline["success_metric_baseline"]["targeted_production_loc"]],  # type: ignore[index]
                ["all_runtime_lexical_mutation_candidates", baseline["direct_mutation_inventory"]["count"]],  # type: ignore[index]
                ["surface_lexical_mutation_candidates", final_gate["metrics"]["surface_lexical_mutation_candidates"]],  # type: ignore[index]
                ["surface_direct_mutation_candidates", final_gate["metrics"]["surface_direct_mutation_candidates"]],  # type: ignore[index]
                ["duplicate_classifier_candidates", baseline["success_metric_baseline"]["duplicate_classifier_candidates"]],  # type: ignore[index]
                ["status_helper_false_positive_candidates", baseline["status_and_classifier_inventory"]["status_helper_function_count"]],  # type: ignore[index]
                ["cfg_test_classifier_candidates", baseline["status_and_classifier_inventory"]["cfg_test_classifier_function_count"]],  # type: ignore[index]
                ["cfg_test_status_helper_candidates", baseline["status_and_classifier_inventory"]["cfg_test_status_helper_function_count"]],  # type: ignore[index]
                ["canonical_cli_leaf_command_candidates", commands["leaf_command_count"]],  # type: ignore[index]
                ["command_specific_option_candidates", commands["command_specific_option_count"]],  # type: ignore[index]
                ["subprocess_command_name_count", commands["subprocess_command_name_count"]],  # type: ignore[index]
                ["legacy_derive_attribute_leaf_candidate_count", commands["legacy_derive_attribute_leaf_candidate_count"]],  # type: ignore[index]
                ["legacy_derive_attribute_option_candidate_count", commands["legacy_derive_attribute_option_candidate_count"]],  # type: ignore[index]
            ],
        ),
        "",
        "## LDR-074 Final Gate Status",
        "",
        f"Status: `{final_gate['status']}`; classification: `{final_gate['classification']}`.",
        "",
        markdown_table(
            ["Metric", "Value", "Threshold", "Status"],
            [
                [row["metric"], row["value"], row["threshold"], row["status"]]
                for row in gate_rows
            ],
        ),
        "",
        "All-runtime lexical mutation candidates remain reported separately because the LDR-074 acceptance gate is scoped to CLI/TUI/transport mutation paths.",
        "Surface lexical mutation candidates remain reported separately because the gate counts only unresolved direct CLI/router/transport/TUI mutation call paths.",
        "Legacy derive command attributes remain reported separately because they count Rust metadata annotations rather than canonical operator command leaves.",
        "Legacy derive arg attributes remain reported separately because they count Rust metadata annotations rather than unique operator option names.",
        "Subprocess command names remain reported separately because `Command::new` calls in runtime helpers are not canonical VIDA CLI leaves.",
        "",
        "Next slices:",
        "",
        *[f"- {item}" for item in final_gate["next_slices"]],  # type: ignore[index]
        "",
        "## Surface Mutation Classification",
        "",
        markdown_table(
            ["Classification", "Count"],
            [[key, value] for key, value in classification_counts_map.items()],
        ),
        "",
        "Unresolved direct surface mutation candidates are the only rows counted by the LDR-074 direct mutation gate.",
        "",
        "## Direct Mutation Candidates",
        "",
        markdown_table(
            ["Path", "Line", "Entity", "Operation", "Classification", "Replacement Operation"],
            mutation_rows,
        )
        if mutation_rows
        else "No direct mutation candidates found by the lexical baseline.",
        "",
        "## Classifier Candidates",
        "",
        markdown_table(
            ["Path", "Line", "Function", "Replacement Operation"],
            classifier_rows,
        )
        if classifier_rows
        else "No classifier candidates found by the lexical baseline.",
        "",
        "## Host-Bridge Defect Path Review",
        "",
        "The current host-bridge defect path maps to the `host_bridge_artifact`, `dispatch_receipt`, `lane_packet`, and `continuation_binding` entity groups. LDRK implementation should replace direct artifact/receipt mutation with a single command envelope and journaled completion outcome before cutover.",
        "",
    ]
    return with_footer(
        "\n".join(body),
        artifact_path="product/spec/ldrk-baseline/drift-map",
        source_path="docs/product/spec/ldrk-baseline/drift-map.md",
        changelog_ref="ldrk-baseline/drift-map.changelog.jsonl",
    )


def render_deletion_candidates(baseline: dict[str, object]) -> str:
    commands = baseline["command_inventory"]
    repeated_flags = commands["repeated_global_flags"]  # type: ignore[index]
    rows = [
        [row["option"], row["count"], "globalize or move to command payload"]
        for row in repeated_flags[:100]
    ]
    body = [
        "# LDRK Baseline Deletion Candidates",
        "",
        "Status: generated baseline artifact for TaskFlow task `ldr-001`.",
        "",
        "## Candidate Classes",
        "",
        "1. Duplicate status/verdict classifiers should move behind `CompletionOutcome`.",
        "2. Direct runtime artifact writes should move behind `VidaCommandEnvelope` and `OperationalJournal`.",
        "3. Repeated command-specific flags should become global context flags or operation payload fields.",
        "4. Legacy command aliases should remain adapter-only until the LDRK CLI reduction slice removes them.",
        "",
        "## Repeated Flag Candidates",
        "",
        markdown_table(["Option", "Count", "Disposition"], rows)
        if rows
        else "No repeated `Arg::new` option names found by the lexical baseline.",
        "",
    ]
    return with_footer(
        "\n".join(body),
        artifact_path="product/spec/ldrk-baseline/deletion-candidates",
        source_path="docs/product/spec/ldrk-baseline/deletion-candidates.md",
        changelog_ref="ldrk-baseline/deletion-candidates.changelog.jsonl",
    )


def render_execution_preparation(baseline: dict[str, object], baseline_hash: str) -> str:
    metrics = baseline["success_metric_baseline"]
    body = [
        "# LDRK Baseline Execution Preparation",
        "",
        "Status: generated execution-preparation artifact for TaskFlow task `ldr-001`.",
        "",
        "## architecture_preparation_report",
        "",
        "Target implementation area: `scripts/architecture` baseline tooling and `docs/product/spec/ldrk-baseline` generated inventory artifacts.",
        "",
        "Relevant architecture context: LDRK moves direct run graph, lane, dispatch, host-bridge, continuation, claim, and task-record mutation toward `VidaCommandEnvelope`, deterministic completion algebra, and a redb-backed `OperationalJournal` for operational records.",
        "",
        "Important invariants: generated artifacts must be reproducible on unchanged sources; domain crates must not depend on redb/SurrealDB/Restate storage types; baseline work is measurement only and does not cut over authority.",
        "",
        "Integration/dependency concerns: `rg` and Python are sufficient for the baseline; `tokei`/`scc` are optional and recorded in `baseline.json` rather than required.",
        "",
        "Expected implementation shape: one deterministic Python script scans owned runtime source roots and emits JSON plus markdown artifacts.",
        "",
        "## developer_handoff_packet",
        "",
        "Prepared task target: implement and maintain `scripts/architecture/ldrk_baseline_inventory.py` and generated artifacts under `docs/product/spec/ldrk-baseline`.",
        "",
        "Intended implementation direction: keep the scanner dependency-free, lexical, deterministic, and explicit about known limitations.",
        "",
        "Bounded next steps for developer lane: refine parser precision only when a later task needs more exact command metadata; do not introduce runtime authority changes in `ldr-001`.",
        "",
        "Required proofs/tests/checks: run the inventory twice, compare stable hashes, inspect the host-bridge drift-map section, run TaskFlow graph validation before task closure.",
        "",
        f"Preparation findings: baseline sha256 `{baseline_hash}`; targeted production LOC `{metrics['targeted_production_loc']}`; direct mutation candidates `{metrics['direct_surface_mutation_candidates']}`; production outcome classifier candidates `{metrics['duplicate_classifier_candidates']}`; status helper false positives `{baseline['status_and_classifier_inventory']['status_helper_function_count']}`; cfg(test) classifier candidates `{baseline['status_and_classifier_inventory']['cfg_test_classifier_function_count']}`; cfg(test) status helper candidates `{baseline['status_and_classifier_inventory']['cfg_test_status_helper_function_count']}`.",
        "",
        "## change_boundary",
        "",
        "May change: `scripts/architecture/**`, `docs/product/spec/ldrk-baseline/**`, and spec map/catalog pointers when needed.",
        "",
        "Must not change: `.vida` runtime state by hand, TaskFlow/DocFlow authority stores, production runtime command behavior, or dependency manifests.",
        "",
        "Reuse rather than rewrite: existing VIDA runtime surfaces, TaskFlow records, DocFlow specs, and release packaging scripts.",
        "",
        "Escalate before mutation: any production Rust runtime authority change, storage dependency change, command contract change, or generated runtime snapshot change.",
        "",
        "## dependency_impact_summary",
        "",
        "Relevant dependencies: Python standard library, `rg` if available, optional `tokei`/`scc` availability recorded as metadata.",
        "",
        "Likely coupling points: Rust CLI definitions, runtime state-store modules, host-bridge and lane receipt paths, TaskFlow artifact registry.",
        "",
        "Migration or compatibility risks: lexical counts are baseline indicators, not semantic proof; later cutover tasks must add contract/integration tests.",
        "",
        "Outward impact to preserve: generated baseline artifacts must remain byte-stable and safe to regenerate locally without mutating runtime state.",
        "",
        "## spec_alignment_summary",
        "",
        "Governing specs/protocols: LDRK epic notes, execution preparation handoff model, canonical inventory law, runtime readiness law, and TaskFlow runtime binding model.",
        "",
        "Required alignment: baseline metrics must support the LDRK code-reduction and drift-reduction gates without moving authority or introducing dual-write behavior.",
        "",
        "Open questions: precise semantic command tree extraction and exact mutation ownership can be improved in later LDRK implementation slices; they do not block this baseline artifact.",
        "",
    ]
    return with_footer(
        "\n".join(body),
        artifact_path="product/spec/ldrk-baseline/execution-preparation",
        source_path="docs/product/spec/ldrk-baseline/execution-preparation.md",
        changelog_ref="ldrk-baseline/execution-preparation.changelog.jsonl",
    )


def with_footer(
    body: str,
    *,
    artifact_path: str,
    source_path: str,
    changelog_ref: str,
) -> str:
    footer = [
        "-----",
        f"artifact_path: {artifact_path}",
        "artifact_type: product_spec",
        "artifact_version: 1",
        "artifact_revision: 2026-06-22",
        "schema_version: 1",
        "status: generated",
        f"source_path: {source_path}",
        "created_at: 2026-06-22T00:00:00Z",
        "updated_at: 2026-06-22T00:00:00Z",
        f"changelog_ref: {changelog_ref}",
        "",
    ]
    return body.rstrip() + "\n\n" + "\n".join(footer)


def run_self_test() -> int:
    sample = """
fn status_is_terminal() { let state = "ready"; }
fn build_status_payload() { let state = "ready"; }
#[cfg(test)]
mod tests {
    fn test_status_classifier() { let state = "blocked"; }
    fn build_status_test_payload() { let state = "blocked"; }
    fn retry_verdict() { let state = "pass"; }
}
fn production_verdict() { let state = "completed"; }
"""
    production_classifiers: list[str] = []
    cfg_test_classifiers: list[str] = []
    production_status_helpers: list[str] = []
    cfg_test_status_helpers: list[str] = []
    production_literals = 0
    cfg_test_literals = 0
    for _, line, is_test_scoped in iter_rust_lines_by_test_scope(sample):
        literal_count = len(list(STATUS_LITERAL_PATTERN.finditer(line)))
        fn_match = FUNCTION_PATTERN.search(line)
        if is_test_scoped:
            cfg_test_literals += literal_count
            if fn_match:
                if is_outcome_classifier_name(fn_match.group(1)):
                    cfg_test_classifiers.append(fn_match.group(1))
                else:
                    cfg_test_status_helpers.append(fn_match.group(1))
        else:
            production_literals += literal_count
            if fn_match:
                if is_outcome_classifier_name(fn_match.group(1)):
                    production_classifiers.append(fn_match.group(1))
                else:
                    production_status_helpers.append(fn_match.group(1))
    expected = {
        "production_classifiers": ["status_is_terminal", "production_verdict"],
        "cfg_test_classifiers": ["test_status_classifier", "retry_verdict"],
        "production_status_helpers": ["build_status_payload"],
        "cfg_test_status_helpers": ["build_status_test_payload"],
        "production_status_literal_count": 3,
        "cfg_test_status_literal_count": 3,
    }
    actual = {
        "production_classifiers": production_classifiers,
        "cfg_test_classifiers": cfg_test_classifiers,
        "production_status_helpers": production_status_helpers,
        "cfg_test_status_helpers": cfg_test_status_helpers,
        "production_status_literal_count": production_literals,
        "cfg_test_status_literal_count": cfg_test_literals,
    }
    if actual != expected:
        print(stable_json({"status": "fail", "expected": expected, "actual": actual}), end="")
        return 1
    classifier_samples = [
        {
            "p": "crates/vida/src/cli.rs",
            "l": 10,
            "e": ["task_record"],
            "op": "record",
            "owner": "crates/vida/src",
            "src": "let recorded_at = task.recorded_at;",
        },
        {
            "p": "crates/vida/src/lane_surface.rs",
            "l": 20,
            "e": ["lane_packet"],
            "op": "write",
            "owner": "crates/vida/src",
            "src": "state.write(lane_packet);",
        },
        {
            "p": "crates/vida/src/root_command_router.rs",
            "l": 30,
            "e": ["task_record"],
            "op": "write",
            "owner": "crates/vida/src",
            "src": "task_record.write(payload);",
        },
    ]
    classified_samples = classified_surface_mutation_records(classifier_samples)
    classification_actual = [
        row["classification"] for row in sorted(classified_samples, key=lambda row: row["l"])
    ]
    classification_expected = [
        "lexical_reference",
        "authority_owned_surface_adapter",
        "unresolved_direct_surface_mutation",
    ]
    gate_sample = ldr074_gate_status(
        loc={"total_production_loc": 1},
        mutations=classifier_samples,
        literals={"classifier_functions": []},
        commands={"leaf_command_count": 0, "command_specific_option_count": 0},
    )
    gate_expected = {
        "surface_direct_mutation_candidates": 1,
        "surface_lexical_mutation_candidates": 3,
        "status": "fail",
    }
    gate_actual = {
        "surface_direct_mutation_candidates": gate_sample["metrics"][
            "surface_direct_mutation_candidates"
        ],
        "surface_lexical_mutation_candidates": gate_sample["metrics"][
            "surface_lexical_mutation_candidates"
        ],
        "status": gate_sample["status"],
    }
    if classification_actual != classification_expected or gate_actual != gate_expected:
        print(
            stable_json(
                {
                    "status": "fail",
                    "classification_expected": classification_expected,
                    "classification_actual": classification_actual,
                    "gate_expected": gate_expected,
                    "gate_actual": gate_actual,
                }
            ),
            end="",
        )
        return 1
    print(
        stable_json(
            {
                "status": "pass",
                "production_classifier_count": len(production_classifiers),
                "cfg_test_classifier_count": len(cfg_test_classifiers),
                "production_status_helper_count": len(production_status_helpers),
                "cfg_test_status_helper_count": len(cfg_test_status_helpers),
                "production_status_literal_count": production_literals,
                "cfg_test_status_literal_count": cfg_test_literals,
                "surface_classification_cases": len(classified_samples),
                "surface_gate_direct_mutation_count": gate_actual[
                    "surface_direct_mutation_candidates"
                ],
            }
        ),
        end="",
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run a focused cfg(test) production-scope regression check and exit.",
    )
    parser.add_argument(
        "--output-dir",
        default="docs/product/spec/ldrk-baseline",
        help="Directory for generated baseline artifacts.",
    )
    args = parser.parse_args()

    if args.self_test:
        return run_self_test()

    root = repo_root()
    output_dir = (root / args.output_dir).resolve()
    baseline = build_baseline(root)
    baseline_body = compact_json(baseline)
    baseline_hash = sha256_text(baseline_body)

    write_if_changed(output_dir / "baseline.json", baseline_body)
    write_if_changed(output_dir / "drift-map.md", render_drift_map(baseline))
    write_if_changed(output_dir / "deletion-candidates.md", render_deletion_candidates(baseline))
    write_if_changed(
        output_dir / "execution-preparation.md",
        render_execution_preparation(baseline, baseline_hash),
    )

    receipt = {
        "status": "pass",
        "task_id": "ldr-001",
        "output_dir": norm(output_dir.relative_to(root)),
        "baseline_sha256": baseline_hash,
        "artifact_count": 4,
        "artifacts": [
            norm((output_dir / "baseline.json").relative_to(root)),
            norm((output_dir / "drift-map.md").relative_to(root)),
            norm((output_dir / "deletion-candidates.md").relative_to(root)),
            norm((output_dir / "execution-preparation.md").relative_to(root)),
        ],
    }
    print(stable_json(receipt), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
