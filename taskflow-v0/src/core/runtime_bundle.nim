## VIDA Runtime Bundle — compiled kernel/runtime bundle for direct consumption.
##
## Composes kernel law, instruction bundle metadata, runtime inventory,
## and project overlay state into one runtime-owned payload.

import std/[json, strutils]
import ./[agent_inventory, config, instruction_engine, kernel_config, role_selection, toon, utils]

proc defaultBundleName(catalog: JsonNode): string =
  let bundleRef = policyValue(catalog{"default_bundle_ref"}, "")
  if bundleRef.len == 0:
    return "default_runtime"
  let parts = bundleRef.split('/')
  var name = parts[^1]
  if name.endsWith(".yaml"):
    name = name[0 .. ^6]
  elif name.endsWith(".yml"):
    name = name[0 .. ^5]
  name

proc buildRuntimeKernelBundle*(cfg: JsonNode = loadRawConfig()): JsonNode =
  let catalog = loadInstructionCatalog()
  let bundleName = defaultBundleName(catalog)
  let instructionBundle = loadInstructionBundle(bundleName)
  let routeCatalog = loadRouteCatalog()
  let receiptTaxonomy = loadReceiptTaxonomy()
  let compiledExtensions = buildCompiledAgentExtensionBundle(cfg)
  let runtimeInventory = buildRuntimeAgentInventory(cfg)

  result = %*{
    "artifact_name": "taskflow_runtime_bundle",
    "artifact_type": "runtime_bundle",
    "generated_at": nowUtc(),
    "vida_root": vidaRoot(),
    "config_path": configPath(),
    "kernel": {
      "summary": kernelSummary(),
      "canonical_machine_map": "docs/product/spec/canonical-machine-map.md",
      "route_catalog_artifact": routeCatalog{"artifact_name"},
      "receipt_taxonomy_artifact": receiptTaxonomy{"artifact_name"},
      "instruction_catalog_artifact": catalog{"artifact_name"},
    },
    "instruction_bundle": {
      "artifact_name": instructionBundle{"artifact_name"},
      "compatibility_class": instructionBundle{"compatibility_class"},
      "bundle_order": instructionBundle{"bundle_order"},
      "role_defaults": instructionBundle{"role_defaults"},
    },
    "runtime_agent_inventory": runtimeInventory,
    "compiled_agent_extensions": compiledExtensions,
    "project_overlay": {
      "project_id": dottedGet(cfg, "project.id", newJNull()),
      "docs_root": dottedGet(cfg, "project.docs_root", newJNull()),
      "language_policy": dottedGet(cfg, "language_policy", newJObject()),
    },
  }

proc runtimeKernelBundleReady*(payload: JsonNode): JsonNode =
  let bundleOrder = dottedGet(payload, "instruction_bundle.bundle_order", newJArray())
  let compiledExtensions = payload{"compiled_agent_extensions"}
  let runtimeInventory = payload{"runtime_agent_inventory"}
  let routeArtifact = dottedGetStr(payload, "kernel.route_catalog_artifact")
  let receiptArtifact = dottedGetStr(payload, "kernel.receipt_taxonomy_artifact")

  var blockers: seq[string] = @[]
  if bundleOrder.kind != JArray or bundleOrder.len == 0:
    blockers.add("missing_bundle_order")
  if compiledExtensions.kind != JObject or compiledExtensions.len == 0:
    blockers.add("missing_compiled_agent_extensions")
  if runtimeInventory.kind != JObject or runtimeInventory{"agents"}.kind != JArray:
    blockers.add("missing_runtime_agent_inventory")
  if routeArtifact.len == 0:
    blockers.add("missing_route_catalog_artifact")
  if receiptArtifact.len == 0:
    blockers.add("missing_receipt_taxonomy_artifact")

  %*{
    "ok": blockers.len == 0,
    "blockers": blockers,
    "bundle_order": bundleOrder,
    "route_catalog_artifact": routeArtifact,
    "receipt_taxonomy_artifact": receiptArtifact,
  }

proc cmdRuntimeBundle*(args: seq[string]): int =
  let asJson = "--json" in args
  let payload = normalizeJson(buildRuntimeKernelBundle())
  if args.len > 0 and args[0] == "check":
    let checkPayload = normalizeJson(runtimeKernelBundleReady(payload))
    if asJson:
      echo pretty(checkPayload)
    else:
      echo renderToon(checkPayload)
    return (if dottedGetBool(checkPayload, "ok", false): 0 else: 1)

  if asJson:
    echo pretty(payload)
  else:
    echo renderToon(payload)
  0
