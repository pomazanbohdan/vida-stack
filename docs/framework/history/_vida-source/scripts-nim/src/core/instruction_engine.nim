## VIDA Instruction Engine — root config instruction catalog/bundle/overlay composition.

import std/[json, strutils]
import ./[kernel_config, utils]

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

proc projectOverlayName(catalog: JsonNode): string =
  let overlayRef = policyValue(catalog{"project_overlay_ref"}, "")
  if overlayRef.len == 0:
    return "project_overlay"
  let parts = overlayRef.split('/')
  var name = parts[^1]
  if name.endsWith(".yaml"):
    name = name[0 .. ^6]
  elif name.endsWith(".yml"):
    name = name[0 .. ^5]
  name

proc loadInstructionBundle*(name: string): JsonNode =
  loadKernelArtifact("instructions/bundles", name)

proc loadInstructionOverlay*(name: string): JsonNode =
  loadKernelArtifact("instructions/overlays", name)

proc composeInstructionSurface*(
  role: string,
  machineName: string = "",
  routeName: string = "",
): JsonNode =
  let catalog = loadInstructionCatalog()
  if catalog.kind != JObject or catalog.len == 0:
    return %*{"ok": false, "reason": "missing_instruction_catalog"}

  let roleSpec = dottedGet(catalog, "roles." & role, newJObject())
  if roleSpec.kind != JObject or roleSpec.len == 0:
    return %*{"ok": false, "reason": "unknown_role", "role": role}

  let bundle = loadInstructionBundle(defaultBundleName(catalog))
  let projectOverlay = loadInstructionOverlay(projectOverlayName(catalog))
  let machineSpec =
    if machineName.len > 0: loadMachineSpec(machineName) else: newJObject()
  let routeSpec =
    if routeName.len > 0: dottedGet(loadRouteCatalog(), "routes." & routeName, newJObject())
    else: newJObject()

  result = %*{
    "ok": true,
    "role": role,
    "instruction_catalog": catalog{"artifact_name"},
    "bundle": bundle{"artifact_name"},
    "composition_order": catalog{"composition_order"},
    "bundle_order": bundle{"bundle_order"},
    "role_binding": roleSpec,
    "project_overlay": {
      "artifact_name": projectOverlay{"artifact_name"},
      "project": projectOverlay{"project"},
      "language_policy": projectOverlay{"language_policy"},
      "project_bootstrap": projectOverlay{"project_bootstrap"},
      "autonomous_execution": projectOverlay{"autonomous_execution"},
      "framework_self_diagnosis": projectOverlay{"framework_self_diagnosis"},
    },
    "layers": {
      "system_base": %*{
        "catalog_revision": catalog{"revision"},
      },
      "project_overlay": projectOverlay{"artifact_name"},
      "role_overlay": %role,
      "machine_binding": (if machineSpec.kind == JObject and machineSpec.len > 0: machineSpec{"artifact_name"} else: newJNull()),
      "route_overlay": (if routeSpec.kind == JObject and routeSpec.len > 0: %routeName else: newJNull()),
    },
    "effective_output_contract": roleSpec{"output_contract"},
    "effective_proof_requirements": roleSpec{"proof_requirements"},
  }

  if machineSpec.kind == JObject and machineSpec.len > 0:
    result["machine_binding"] = %*{
      "machine": machineName,
      "entity_type": machineSpec{"entity_type"},
      "guards_catalog": machineSpec{"guards_catalog"},
    }

  if routeSpec.kind == JObject and routeSpec.len > 0:
    result["route_overlay"] = %*{
      "route": routeName,
      "route_stage": routeSpec{"route_stage"},
      "required_roles": routeSpec{"required_roles"},
      "capability": routeSpec{"capability"},
      "independence_class": routeSpec{"independence_class"},
    }
