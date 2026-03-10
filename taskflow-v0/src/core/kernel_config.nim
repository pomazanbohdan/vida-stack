## VIDA Root Kernel Config Loader — reads product-law artifacts from `vida/config`.

import std/[algorithm, json, os, strutils]
import ./config

proc kernelConfigRoot*(): string =
  vidaRoot() / "vida" / "config"

proc kernelFamilyPath*(family: string): string =
  kernelConfigRoot() / family

proc kernelFamilyExists*(family: string): bool =
  dirExists(kernelFamilyPath(family))

proc kernelArtifactPath*(family, name: string): string =
  let base = kernelFamilyPath(family)
  for ext in [".yaml", ".yml", ".json"]:
    let candidate = base / (name & ext)
    if fileExists(candidate):
      return candidate
  return ""

proc kernelArtifactExists*(family, name: string): bool =
  kernelArtifactPath(family, name).len > 0

proc loadKernelArtifact*(family, name: string): JsonNode =
  let path = kernelArtifactPath(family, name)
  if path.len == 0:
    return newJObject()
  try:
    if path.endsWith(".json"):
      return parseJson(readFile(path))
    return parseYamlSubset(readFile(path))
  except:
    return newJObject()

proc listKernelArtifacts*(family: string): seq[string] =
  let dir = kernelFamilyPath(family)
  if not dirExists(dir):
    return @[]
  result = @[]
  for entry in walkDir(dir):
    if entry.kind != pcFile:
      continue
    let (_, name, ext) = splitFile(entry.path)
    if ext in [".yaml", ".yml", ".json"] and name notin result:
      result.add(name)
  result.sort()

proc loadMachineSpec*(name: string): JsonNode =
  loadKernelArtifact("machines", name)

proc loadRouteCatalog*(): JsonNode =
  loadKernelArtifact("routes", "route_catalog")

proc loadPolicySpec*(name: string): JsonNode =
  loadKernelArtifact("policies", name)

proc loadAgentClasses*(): JsonNode =
  loadKernelArtifact("agents", "agents")

proc loadAgentGroups*(): JsonNode =
  loadKernelArtifact("agents", "agent_groups")

proc loadInstructionCatalog*(): JsonNode =
  loadKernelArtifact("instructions", "instruction_catalog")

proc loadReceiptTaxonomy*(): JsonNode =
  loadKernelArtifact("receipts", "receipt_taxonomy")

proc loadMigrationSpec*(name: string): JsonNode =
  loadKernelArtifact("migration", name)

proc kernelSummary*(): JsonNode =
  %*{
    "root": kernelConfigRoot(),
    "families": {
      "machines": listKernelArtifacts("machines"),
      "routes": listKernelArtifacts("routes"),
      "policies": listKernelArtifacts("policies"),
      "agents": listKernelArtifacts("agents"),
      "instructions": listKernelArtifacts("instructions"),
      "receipts": listKernelArtifacts("receipts"),
      "migration": listKernelArtifacts("migration"),
      "schemas": listKernelArtifacts("schemas"),
    },
  }
