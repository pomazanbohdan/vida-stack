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
COMMAND_PATTERN = re.compile(r"\b(?:Command|ClapCommand)::new\(\s*\"([^\"]+)\"")
ARG_PATTERN = re.compile(r"\bArg::new\(\s*\"([^\"]+)\"")
SUBCOMMAND_ATTR_PATTERN = re.compile(r"#\s*\[\s*command\s*\(")
ARG_ATTR_PATTERN = re.compile(r"#\s*\[\s*arg\s*\(")

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


def production_loc(files: Iterable[SourceFile]) -> dict[str, object]:
    total = 0
    by_root: dict[str, int] = {target: 0 for target in TARGET_ROOTS}
    by_file: list[dict[str, object]] = []
    for item in files:
        count = 0
        in_block = False
        for raw_line in read_text(item.path).splitlines():
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
                }
            )
    return sorted(records, key=lambda row: (row["p"], row["l"], row["op"]))


def inventory_literals_and_classifiers(files: Iterable[SourceFile]) -> dict[str, object]:
    literals: dict[str, dict[str, object]] = {}
    classifiers: list[dict[str, object]] = []
    for item in files:
        for index, line in enumerate(read_text(item.path).splitlines(), start=1):
            for match in STATUS_LITERAL_PATTERN.finditer(line):
                value = match.group(1)
                key = value.lower()
                entry = literals.setdefault(
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
                classifiers.append(
                    {
                        "p": item.rel,
                        "l": index,
                        "fn": fn_match.group(1),
                    }
                )
    return {
        "status_blocker_literals": [literals[key] for key in sorted(literals)],
        "classifier_functions": sorted(classifiers, key=lambda row: (row["p"], row["l"])),
    }


def inventory_commands(files: Iterable[SourceFile]) -> dict[str, object]:
    command_records: list[dict[str, object]] = []
    option_records: list[dict[str, object]] = []
    attr_subcommands = 0
    attr_options = 0
    for item in files:
        if "crates/vida" not in item.rel:
            continue
        for index, line in enumerate(read_text(item.path).splitlines(), start=1):
            for match in COMMAND_PATTERN.finditer(line):
                command_records.append({"path": item.rel, "line": index, "command": match.group(1)})
            for match in ARG_PATTERN.finditer(line):
                option_records.append({"path": item.rel, "line": index, "option": match.group(1)})
            if SUBCOMMAND_ATTR_PATTERN.search(line):
                attr_subcommands += 1
            if ARG_ATTR_PATTERN.search(line):
                attr_options += 1
    option_counts: dict[str, int] = {}
    for row in option_records:
        option = str(row["option"])
        option_counts[option] = option_counts.get(option, 0) + 1
    repeated_global_flags = [
        {"option": option, "count": count}
        for option, count in sorted(option_counts.items())
        if count > 1
    ]
    return {
        "parser": "lexical_rust_clap_baseline",
        "leaf_command_count": len({row["command"] for row in command_records}) + attr_subcommands,
        "command_specific_option_count": len(option_records) + attr_options,
        "command_records": sorted(command_records, key=lambda row: (row["command"], row["path"], row["line"])),
        "option_records": sorted(option_records, key=lambda row: (row["option"], row["path"], row["line"])),
        "derive_command_attribute_count": attr_subcommands,
        "derive_arg_attribute_count": attr_options,
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
                    [tool, "--version"],
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
            "direct_surface_mutation_candidates": len(mutations),
            "canonical_cli_leaf_command_candidates": commands["leaf_command_count"],
            "command_specific_option_candidates": commands["command_specific_option_count"],
        },
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
    top_mutations = mutations[:80]
    mutation_rows = [
        [
            row["p"],
            row["l"],
            ",".join(row["e"]),
            row["op"],
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
                ["direct_surface_mutation_candidates", baseline["direct_mutation_inventory"]["count"]],  # type: ignore[index]
                ["duplicate_classifier_candidates", baseline["success_metric_baseline"]["duplicate_classifier_candidates"]],  # type: ignore[index]
                ["canonical_cli_leaf_command_candidates", commands["leaf_command_count"]],  # type: ignore[index]
                ["command_specific_option_candidates", commands["command_specific_option_count"]],  # type: ignore[index]
            ],
        ),
        "",
        "## Direct Mutation Candidates",
        "",
        markdown_table(
            ["Path", "Line", "Entity", "Operation", "Replacement Operation"],
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
        f"Preparation findings: baseline sha256 `{baseline_hash}`; targeted production LOC `{metrics['targeted_production_loc']}`; direct mutation candidates `{metrics['direct_surface_mutation_candidates']}`; classifier candidates `{metrics['duplicate_classifier_candidates']}`.",
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        default="docs/product/spec/ldrk-baseline",
        help="Directory for generated baseline artifacts.",
    )
    args = parser.parse_args()

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
