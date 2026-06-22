# LDRK Top Ten Operator Workflow Walkthrough

Status: generated proof artifact for TaskFlow task `ldr-003`.

Rule: every workflow enters one canonical operation family and carries semantic details in one typed payload.

| # | Current workflow | Current surface | Target canonical operation |
|---|---|---|---|
| 1 | Continue active task by agents | `vida taskflow consume continue --run-id <run>` | `vida plan run.continue then vida apply run.advance` |
| 2 | Dispatch next agent lane | `vida agent dispatch-next / vida agent-init` | `vida plan lane.dispatch then vida apply lane.start` |
| 3 | Complete host bridge verifier result | `vida agent host-bridge --complete --decision --verdict --blocker-code --rework-target --allowed-next-node` | `vida apply host_bridge.complete --payload outcome.json` |
| 4 | Inspect backlog readiness | `vida task ready / vida task next-lawful` | `vida get task.ready --payload query.json` |
| 5 | Create implementation todo | `vida task create --type todo --status in_progress` | `vida apply task.create --payload task.json` |
| 6 | Attach proof evidence | `vida task proof attach-evidence ...` | `vida apply proof.attach --payload evidence.json` |
| 7 | Close task after proof | `vida task close <id> --reason ...` | `vida apply task.close --payload closure.json` |
| 8 | Run runtime diagnostics | `vida diagnostics post-commit` | `vida get diagnostics.post_commit or vida apply diagnostics.record` |
| 9 | Build and install release | `scripts build-release + install.ps1 / vida release install` | `vida service release.install --payload release.json` |
| 10 | Recover blocked lane | `vida lane exception-takeover / vida lane show / vida taskflow recovery status` | `vida repair lane.recover --payload recovery.json` |

## Host Bridge Outcome Payload Shape

```json
{"outcome":"Blocked","blockers":[{"code":"missing_required_proof_artifacts","evidence_refs":["docs/product/spec/ldrk-operation-catalog/operation-cli-map.json"]}],"rework_target":"developer_rework","evidence_refs":["host_bridge_receipt"]}
```

The target payload replaces independent CLI semantic flags for decision, verdict, blocker code, rework target and allowed next node.

-----
artifact_path: product/spec/ldrk-operation-catalog/top-ten-operator-workflow-walkthrough
artifact_type: product_spec
artifact_version: "1"
source_path: docs/product/spec/ldrk-operation-catalog/top-ten-operator-workflow-walkthrough.md
created_at: 2026-06-22T00:00:00+03:00
updated_at: 2026-06-22T00:00:00+03:00
changelog_ref: current-spec-catalog.changelog.jsonl
