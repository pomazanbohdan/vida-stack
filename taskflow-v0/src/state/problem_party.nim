## VIDA Problem-Party Runtime — Party Chat v2 helper surfaces.
##
## Renders bounded council manifests, consumes project agent-extension registries,
## and writes structured decision receipts that update the run graph.

import std/[json, os, strutils]
import ../core/[config, role_selection, utils]
import ./run_graph

const
  SmallBoardRoleIds = [
    "party_chat_architect",
    "party_chat_runtime_systems",
    "party_chat_quality_verification",
    "party_chat_delivery_cost",
  ]

  LargeBoardRoleIds = [
    "party_chat_architect",
    "party_chat_runtime_systems",
    "party_chat_quality_verification",
    "party_chat_delivery_cost",
    "party_chat_product_scope",
    "party_chat_security_safety",
    "party_chat_sre_observability",
    "party_chat_data_contracts",
    "party_chat_dx_tooling",
    "party_chat_pm_process",
  ]

proc problemPartyDir*(): string =
  vidaRoot() / ".vida" / "logs" / "problem-party"

proc topicSafeName(topic: string): string =
  safeName(topic, "topic")

proc manifestPath*(taskId, topic: string): string =
  problemPartyDir() / (safeName(taskId, "task") & "." & topicSafeName(topic) & ".manifest.json")

proc receiptPath*(taskId, topic: string): string =
  problemPartyDir() / (safeName(taskId, "task") & "." & topicSafeName(topic) & ".json")

proc dispatchPlanPath*(taskId, topic: string): string =
  manifestPath(taskId, topic).replace(".manifest.json", ".dispatch-plan.json")

proc sessionPlanPath*(taskId, topic: string): string =
  manifestPath(taskId, topic).replace(".manifest.json", ".session-plan.json")

proc executeArtifactPath*(taskId, topic: string): string =
  manifestPath(taskId, topic).replace(".manifest.json", ".execution.json")

proc seatPromptsPath*(taskId, topic: string): string =
  manifestPath(taskId, topic).replace(".manifest.json", ".seat-prompts.json")

proc objectArray(node: JsonNode): seq[JsonNode] =
  if not node.isNil and node.kind == JArray:
    for item in node:
      if item.kind == JObject:
        result.add(item)

proc findById(rows: seq[JsonNode], key, value: string): JsonNode =
  for row in rows:
    if dottedGetStr(row, key) == value:
      return row
  return newJObject()

proc boardRoleIds(boardSize: string): seq[string] =
  case boardSize
  of "small":
    @SmallBoardRoleIds
  of "large":
    @LargeBoardRoleIds
  else:
    @SmallBoardRoleIds

proc partyChatConfig(cfg: JsonNode): JsonNode =
  dottedGet(cfg, "party_chat", newJObject())

proc constrainedBoardRoleIds(cfg: JsonNode, boardSize: string): seq[string] =
  let partyCfg = partyChatConfig(cfg)
  let allRoles = boardRoleIds(boardSize)
  let configuredMax = max(1, dottedGetInt(partyCfg, "max_experts", allRoles.len))
  let hardCap = max(2, dottedGetInt(partyCfg, "hard_cap_agents", allRoles.len + 1))
  let maxExpertsByHardCap = max(1, hardCap - 1) # facilitator always takes one slot
  let effectiveMax = min(allRoles.len, min(configuredMax, maxExpertsByHardCap))
  let finalCount = max(1, effectiveMax)
  result = allRoles[0 ..< finalCount]

proc boardFlowId(boardSize: string): string =
  case boardSize
  of "large": "party_chat_council_large"
  else: "party_chat_council_small"

proc profileIdForRole(roleId: string): string =
  case roleId
  of "party_chat_facilitator": "party_chat_facilitator_profile"
  of "party_chat_architect": "party_chat_architect_profile"
  of "party_chat_runtime_systems": "party_chat_runtime_systems_profile"
  of "party_chat_quality_verification": "party_chat_quality_verification_profile"
  of "party_chat_delivery_cost": "party_chat_delivery_cost_profile"
  of "party_chat_product_scope": "party_chat_product_scope_profile"
  of "party_chat_security_safety": "party_chat_security_safety_profile"
  of "party_chat_sre_observability": "party_chat_sre_observability_profile"
  of "party_chat_data_contracts": "party_chat_data_contracts_profile"
  of "party_chat_dx_tooling": "party_chat_dx_tooling_profile"
  of "party_chat_pm_process": "party_chat_pm_process_profile"
  else: ""

proc roleBinding(bundle: JsonNode, roleId: string): JsonNode =
  let roleRows = objectArray(bundle{"project_roles"})
  let roleRow = findById(roleRows, "role_id", roleId)
  %*{
    "role_id": roleId,
    "base_role": dottedGetStr(roleRow, "base_role"),
    "description": dottedGetStr(roleRow, "description"),
  }

proc profileBinding(bundle: JsonNode, roleId: string): JsonNode =
  let profileRows = objectArray(bundle{"project_profiles"})
  let profileId = profileIdForRole(roleId)
  let profileRow = findById(profileRows, "profile_id", profileId)
  %*{
    "profile_id": profileId,
    "role_ref": dottedGetStr(profileRow, "role_ref"),
    "skill_refs": splitCsv(profileRow{"skill_refs"}),
    "stance": dottedGetStr(profileRow, "stance"),
    "preferred_backend": dottedGetStr(profileRow, "preferred_backend"),
    "preferred_model": dottedGetStr(profileRow, "preferred_model"),
  }

proc flowBinding(bundle: JsonNode, boardSize: string): JsonNode =
  let flowRows = objectArray(bundle{"project_flows"})
  let flowId = boardFlowId(boardSize)
  let row = findById(flowRows, "flow_id", flowId)
  %*{
    "flow_id": flowId,
    "description": dottedGetStr(row, "description"),
    "role_chain": splitCsv(row{"role_chain"}),
  }

proc collectBindingErrors(facilitator, flowBinding, roleBindings: JsonNode): JsonNode =
  result = newJArray()
  if dottedGetStr(facilitator, "role.base_role").len == 0:
    result.add(%"missing_facilitator_role")
  if dottedGetStr(facilitator, "profile.role_ref").len == 0:
    result.add(%"missing_facilitator_profile")
  if flowBinding{"role_chain"}.kind != JArray or flowBinding{"role_chain"}.len == 0:
    result.add(%"missing_flow_binding")

  if roleBindings.kind == JArray:
    for entry in roleBindings:
      if entry.kind != JObject:
        result.add(%"invalid_role_binding_entry")
        continue
      let roleId = dottedGetStr(entry, "role.role_id", "unknown_role")
      if dottedGetStr(entry, "role.base_role").len == 0:
        result.add(%("missing_role_binding:" & roleId))
      if dottedGetStr(entry, "profile.role_ref").len == 0:
        result.add(%("missing_profile_binding:" & roleId))

proc collectConfigErrors(manifest: JsonNode): JsonNode =
  result = newJArray()
  let executionMode = dottedGetStr(manifest, "party_chat_config.execution_mode", "multi_agent")
  let configuredMin = dottedGetInt(manifest, "party_chat_config.min_experts", 1)
  let effectiveExperts = dottedGetInt(manifest, "party_chat_config.effective_expert_count", 0)
  if effectiveExperts < configuredMin:
    result.add(%"insufficient_expert_capacity_for_min_experts")
  if executionMode == "single_agent":
    if dottedGetStr(manifest, "party_chat_config.single_agent.backend").len == 0:
      result.add(%"missing_single_agent_backend")
    if dottedGetStr(manifest, "party_chat_config.single_agent.model").len == 0:
      result.add(%"missing_single_agent_model")

proc roleModelBinding(cfg: JsonNode, roleId: string): JsonNode =
  let partyCfg = partyChatConfig(cfg)
  let row = dottedGet(partyCfg, "role_model_bindings." & roleId, newJObject())
  %*{
    "backend": dottedGetStr(row, "backend"),
    "model": dottedGetStr(row, "model"),
  }

proc boardRoleBindings(bundle: JsonNode, boardSize: string): JsonNode =
  let cfg = loadRawConfig()
  result = newJArray()
  for roleId in constrainedBoardRoleIds(cfg, boardSize):
    result.add(%*{
      "role": roleBinding(bundle, roleId),
      "profile": profileBinding(bundle, roleId),
    })

proc defaultRounds(boardSize: string): int =
  if boardSize == "large": 2 else: 1

proc readOptionalText(path: string): string =
  if path.len == 0 or not fileExists(path):
    return ""
  readFile(path).strip()

proc stringArray(node: JsonNode): seq[string] =
  if not node.isNil and node.kind == JArray:
    for item in node:
      let value = policyValue(item, "")
      if value.len > 0:
        result.add(value)

proc collectUniqueStrings(target: var seq[string], values: seq[string]) =
  for value in values:
    if value.len > 0 and value notin target:
      target.add(value)

proc roleNotesArray(roleNotes: JsonNode): seq[JsonNode] =
  if not roleNotes.isNil and roleNotes.kind == JArray:
    for note in roleNotes:
      if note.kind == JObject:
        result.add(note)

proc dispatchPlanForManifest*(manifest: JsonNode): JsonNode

proc sessionPlanForManifest*(manifest: JsonNode): JsonNode =
  let dispatch = dispatchPlanForManifest(manifest)
  let roundCount = max(1, dottedGetInt(manifest, "round_count", 1))
  var rounds = newJArray()
  for roundIndex in 0 ..< roundCount:
    let stage =
      if roundIndex == 0: "proposal"
      elif roundIndex == roundCount - 1: "synthesis"
      else: "critique"
    rounds.add(%*{
      "round": roundIndex + 1,
      "stage": stage,
      "agent_count": dottedGetInt(dispatch, "agent_count", 0),
      "agents": dispatch{"agents"},
    })

  result = %*{
    "task_id": dottedGetStr(manifest, "task_id"),
    "topic": dottedGetStr(manifest, "topic"),
    "board_size": dottedGetStr(manifest, "board_size", "small"),
    "execution_mode": dottedGetStr(dispatch, "execution_mode", "multi_agent"),
    "round_count": roundCount,
    "rounds": rounds,
    "facilitator_role": dottedGetStr(manifest, "facilitator.role.role_id"),
    "bounded": true,
    "binding_validation": manifest{"binding_validation"},
  }

proc seatPromptsForManifest*(manifest: JsonNode): JsonNode =
  let dispatch = dispatchPlanForManifest(manifest)
  let topic = dottedGetStr(manifest, "topic")
  let boardSize = dottedGetStr(manifest, "board_size", "small")
  let problemFrame = dottedGetStr(manifest, "problem_frame")
  result = newJArray()

  if dispatch{"agents"}.kind == JArray:
    for agent in dispatch{"agents"}:
      if agent.kind != JObject:
        continue
      let seat = dottedGetStr(agent, "seat")
      let roleId = dottedGetStr(agent, "role_id", seat)
      let profileId = dottedGetStr(agent, "profile_id")
      result.add(%*{
        "seat": seat,
        "role_id": roleId,
        "profile_id": profileId,
        "backend": dottedGetStr(agent, "backend"),
        "model": dottedGetStr(agent, "model"),
        "prompt": (
          "Party Chat seat: " & seat & "\n" &
          "Topic: " & topic & "\n" &
          "Board: " & boardSize & "\n" &
          "Problem frame:\n" & problemFrame & "\n\n" &
          "Return structured notes with recommendations, verification_checks, execution_steps, and open_risks."
        ),
      })

proc renderProblemPartyManifest*(taskId, topic, boardSize: string,
                                 rounds: int = 0,
                                 problemFile: string = ""): JsonNode =
  let cfg = loadRawConfig()
  let bundle = buildCompiledAgentExtensionBundle(cfg)
  let partyCfg = partyChatConfig(cfg)
  let normalizedBoard = if boardSize in ["small", "large"]: boardSize else: "small"
  let problemFrame = readOptionalText(problemFile)
  let roundCount = if rounds > 0: rounds else: defaultRounds(normalizedBoard)
  let bindings = boardRoleBindings(bundle, normalizedBoard)
  let flow = flowBinding(bundle, normalizedBoard)
  var roleIds = newJArray()
  var profileIds = newJArray()
  var runtimeAgents = newJArray()
  for entry in bindings:
    roleIds.add(entry{"role"}{"role_id"})
    profileIds.add(entry{"profile"}{"profile_id"})
    let roleId = dottedGetStr(entry, "role.role_id")
    let profile = entry{"profile"}
    runtimeAgents.add(%*{
      "seat_role": roleId,
      "profile_id": dottedGetStr(profile, "profile_id"),
      "preferred_backend": dottedGetStr(profile, "preferred_backend"),
      "preferred_model": dottedGetStr(profile, "preferred_model"),
      "role_model_binding": roleModelBinding(cfg, roleId),
    })

  let executionMode = dottedGetStr(partyCfg, "execution_mode", "multi_agent")
  let routingStrategy = dottedGetStr(partyCfg, "model_routing_strategy", "by_role")
  let facilitatorRoleId = "party_chat_facilitator"
  let facilitatorProfile = profileBinding(bundle, facilitatorRoleId)
  let facilitator = %*{
    "role": roleBinding(bundle, facilitatorRoleId),
    "profile": facilitatorProfile,
    "role_model_binding": roleModelBinding(cfg, facilitatorRoleId),
  }

  result = %*{
    "task_id": taskId,
    "topic": topic,
    "board_size": normalizedBoard,
    "round_count": roundCount,
    "facilitator": facilitator,
    "roles": roleIds,
    "problem_frame": problemFrame,
    "constraints": [],
    "options": [],
    "conflict_points": [],
    "decision": "",
    "why_not_others": [],
    "next_execution_step": "",
    "confidence": "medium",
    "budget_summary": {
      "token_budget_mode": "bounded",
      "board_preset": normalizedBoard,
      "default_round_count": roundCount,
    },
    "party_chat_config": %*{
      "enabled": dottedGetBool(partyCfg, "enabled", false),
      "replaces_problem_party": dottedGetBool(partyCfg, "replaces_problem_party", false),
      "execution_mode": executionMode,
      "model_routing_strategy": routingStrategy,
      "default_board_size": dottedGetStr(partyCfg, "default_board_size", "small"),
      "min_experts": dottedGetInt(partyCfg, "min_experts", 2),
      "max_experts": dottedGetInt(partyCfg, "max_experts", roleIds.len),
      "hard_cap_agents": dottedGetInt(partyCfg, "hard_cap_agents", roleIds.len + 2),
      "dei_enabled": dottedGetBool(partyCfg, "dei_enabled", true),
      "effective_expert_count": roleIds.len,
      "effective_agent_count":
        (if executionMode == "single_agent": 1 else: roleIds.len + 1),
      "single_agent": dottedGet(partyCfg, "single_agent", newJObject()),
    },
    "role_bindings": bindings,
    "profile_bindings": profileIds,
    "runtime_agents": runtimeAgents,
    "flow_binding": flow,
    "compiled_bundle_enabled": dottedGetBool(bundle, "enabled", false),
    "generated_at": nowUtc(),
    "source_problem_file": problemFile,
  }
  let bindingErrors = collectBindingErrors(facilitator, flow, bindings)
  let configErrors = collectConfigErrors(result)
  var allErrors = newJArray()
  for err in bindingErrors:
    allErrors.add(err)
  for err in configErrors:
    allErrors.add(err)
  result["binding_validation"] = %*{
    "valid": allErrors.len == 0,
    "errors": allErrors,
  }
  result["status"] = %(if allErrors.len == 0: "ready" else: "invalid_manifest")

proc synthesizeProblemParty*(manifestPath, roleNotesPath: string): JsonNode =
  let manifest = loadJson(manifestPath)
  let roleNotes = loadJson(roleNotesPath, newJArray())
  let notes = roleNotesArray(roleNotes)
  var recommendations: seq[string] = @[]
  var verificationChecks: seq[string] = @[]
  var executionSteps: seq[string] = @[]
  var openRisks: seq[string] = @[]

  for note in notes:
    collectUniqueStrings(recommendations, stringArray(note{"recommendations"}))
    collectUniqueStrings(verificationChecks, stringArray(note{"verification_checks"}))
    collectUniqueStrings(executionSteps, stringArray(note{"execution_steps"}))
    collectUniqueStrings(openRisks, stringArray(note{"open_risks"}))

  let synthesizedDecision =
    if dottedGetStr(manifest, "decision").len > 0: dottedGetStr(manifest, "decision")
    elif recommendations.len > 0: recommendations[0]
    else: "bounded_party_chat_review_completed"
  let nextStep =
    if dottedGetStr(manifest, "next_execution_step").len > 0: dottedGetStr(manifest, "next_execution_step")
    elif executionSteps.len > 0: "writer"
    else: "review_required"
  let confidence =
    if openRisks.len == 0 and notes.len > 0: "high"
    elif notes.len > 0: "medium"
    else: dottedGetStr(manifest, "confidence", "medium")

  result = %*{
    "task_id": dottedGetStr(manifest, "task_id"),
    "topic": dottedGetStr(manifest, "topic"),
    "board_size": dottedGetStr(manifest, "board_size", "small"),
    "roles": manifest{"roles"},
    "decision": synthesizedDecision,
    "confidence": confidence,
    "next_execution_step": nextStep,
    "problem_frame": dottedGetStr(manifest, "problem_frame"),
    "role_notes": roleNotes,
    "decision_packet": %*{
      "decision": synthesizedDecision,
      "why_not_others": manifest{"why_not_others"},
      "recommendations": recommendations,
      "confidence": confidence,
    },
    "verification_packet": %*{
      "required_checks": verificationChecks,
      "open_risks": openRisks,
      "verification_required": verificationChecks.len > 0 or openRisks.len > 0,
    },
    "execution_packet": %*{
      "next_execution_step": nextStep,
      "execution_steps": executionSteps,
      "writer_unblocked": nextStep.toLowerAscii() in ["writer", "writer_ready", "resume_writer", "implementation"],
    },
    "generated_at": nowUtc(),
  }

proc dispatchPlanForManifest*(manifest: JsonNode): JsonNode =
  if not dottedGetBool(manifest, "binding_validation.valid", false):
    return %*{
      "execution_mode": dottedGetStr(manifest, "party_chat_config.execution_mode", "multi_agent"),
      "agent_count": 0,
      "agents": [],
      "status": "invalid_manifest",
      "errors": manifest{"binding_validation"}{"errors"},
    }

  let partyCfg = manifest{"party_chat_config"}
  let executionMode = dottedGetStr(partyCfg, "execution_mode", "multi_agent")
  let facilitator = manifest{"facilitator"}
  let runtimeAgents = manifest{"runtime_agents"}
  if executionMode == "single_agent":
    return %*{
      "execution_mode": executionMode,
      "agent_count": 1,
      "agents": [
        %*{
          "seat": "party_chat_single_agent",
          "backend": dottedGetStr(partyCfg, "single_agent.backend"),
          "model": dottedGetStr(partyCfg, "single_agent.model"),
          "covers_roles": manifest{"roles"},
          "facilitator_profile_id": dottedGetStr(facilitator, "profile.profile_id"),
        }
      ],
    }

  var agents = newJArray()
  agents.add(%*{
    "seat": "facilitator",
    "role_id": dottedGetStr(facilitator, "role.role_id"),
    "profile_id": dottedGetStr(facilitator, "profile.profile_id"),
    "backend": dottedGetStr(facilitator, "role_model_binding.backend"),
    "model": dottedGetStr(facilitator, "role_model_binding.model"),
  })
  if runtimeAgents.kind == JArray:
    for agent in runtimeAgents:
      if agent.kind == JObject:
        agents.add(%*{
          "seat": dottedGetStr(agent, "seat_role"),
          "role_id": dottedGetStr(agent, "seat_role"),
          "profile_id": dottedGetStr(agent, "profile_id"),
          "backend":
            (if dottedGetStr(agent, "role_model_binding.backend").len > 0:
              dottedGetStr(agent, "role_model_binding.backend")
             else:
              dottedGetStr(agent, "preferred_backend")),
          "model":
            (if dottedGetStr(agent, "role_model_binding.model").len > 0:
              dottedGetStr(agent, "role_model_binding.model")
             else:
              dottedGetStr(agent, "preferred_model")),
        })

  result = %*{
    "execution_mode": executionMode,
    "agent_count": agents.len,
    "agents": agents,
  }

proc writeProblemPartyReceipt*(taskId, taskClass, topic: string,
                               decisionArtifact: JsonNode): string =
  let path = receiptPath(taskId, topic)
  var payload = decisionArtifact
  if payload.isNil or payload.kind != JObject:
    payload = newJObject()
  payload["task_id"] = %taskId
  payload["task_class"] = %taskClass
  payload["topic"] = %topic
  payload["receipt_type"] = %"problem_party_receipt"
  payload["written_at"] = %nowUtc()
  saveJson(path, payload)

  discard updateNode(taskId, taskClass, "problem_party", "completed", meta = %*{
    "receipt_path": path,
    "topic": topic,
  })

  let nextStep = dottedGetStr(payload, "next_execution_step").toLowerAscii()
  if nextStep in ["writer", "writer_ready", "resume_writer", "implementation"]:
    discard updateNode(taskId, taskClass, "writer", "ready", meta = %*{
      "reason": "problem_party_unblocked_execution",
      "receipt_path": path,
    })
  return path

proc executeProblemParty*(taskId, taskClass, manifestPathArg, roleNotesPath: string): tuple[exitCode: int, payload: JsonNode] =
  let manifest = loadJson(manifestPathArg)
  if manifest.len == 0:
    return (2, %*{"status": "missing_manifest", "task_id": taskId})

  let topic = dottedGetStr(manifest, "topic", "topic")
  let dispatch = dispatchPlanForManifest(manifest)
  if dottedGetInt(dispatch, "agent_count", 0) == 0:
    return (2, %*{
      "status": "invalid_manifest",
      "task_id": taskId,
      "topic": topic,
      "dispatch_plan": dispatch,
    })

  let sessionPlan = sessionPlanForManifest(manifest)
  let prompts = seatPromptsForManifest(manifest)
  let synthesized = synthesizeProblemParty(manifestPathArg, roleNotesPath)
  let receipt = writeProblemPartyReceipt(taskId, taskClass, topic, synthesized)

  discard writeJson(dispatchPlanPath(taskId, topic), dispatch)
  discard writeJson(sessionPlanPath(taskId, topic), sessionPlan)
  discard writeJson(seatPromptsPath(taskId, topic), prompts)

  result = (0, %*{
    "status": "executed",
    "task_id": taskId,
    "task_class": taskClass,
    "topic": topic,
    "dispatch_plan_path": dispatchPlanPath(taskId, topic),
    "session_plan_path": sessionPlanPath(taskId, topic),
    "seat_prompts_path": seatPromptsPath(taskId, topic),
    "receipt_path": receipt,
    "synthesis": synthesized,
  })
  discard writeJson(executeArtifactPath(taskId, topic), result.payload)

proc cmdProblemParty*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 problem-party render <task_id> "<topic>" [--board small|large] [--rounds N] [--problem-file PATH] [--output-dir DIR]
  taskflow-v0 problem-party dispatch-plan <board_manifest.json> [--output PATH]
  taskflow-v0 problem-party session-plan <board_manifest.json> [--output PATH]
  taskflow-v0 problem-party execute <task_id> <task_class> <board_manifest.json> <role_notes.json> [--output PATH]
  taskflow-v0 problem-party synthesize <board_manifest.json> <role_notes.json> [--output PATH]
  taskflow-v0 problem-party receipt <task_id> <task_class> "<topic>" <decision_artifact.json>"""
    return 1

  case args[0]
  of "render":
    if args.len < 3:
      echo "Usage: taskflow-v0 problem-party render <task_id> \"<topic>\" [--board small|large] [--rounds N] [--problem-file PATH] [--output-dir DIR]"
      return 1
    let taskId = args[1]
    let topic = args[2]
    var boardSize = "small"
    var rounds = 0
    var problemFile = ""
    var outputDir = problemPartyDir()
    var i = 3
    while i < args.len:
      case args[i]
      of "--board":
        if i + 1 < args.len: boardSize = args[i + 1]; i += 2 else: return 1
      of "--rounds":
        if i + 1 < args.len:
          try:
            rounds = parseInt(args[i + 1])
          except ValueError:
            echo "--rounds must be an integer"
            return 1
          i += 2
        else:
          return 1
      of "--problem-file":
        if i + 1 < args.len: problemFile = args[i + 1]; i += 2 else: return 1
      of "--output-dir":
        if i + 1 < args.len: outputDir = args[i + 1]; i += 2 else: return 1
      else:
        echo "Unknown flag: " & args[i]
        return 1
    let payload = renderProblemPartyManifest(taskId, topic, boardSize, rounds, problemFile)
    let outPath = outputDir / (safeName(taskId, "task") & "." & topicSafeName(topic) & ".manifest.json")
    echo writeJson(outPath, payload)
    return (if dottedGetBool(payload, "binding_validation.valid", false): 0 else: 2)

  of "dispatch-plan":
    if args.len < 2:
      echo "Usage: taskflow-v0 problem-party dispatch-plan <board_manifest.json> [--output PATH]"
      return 1
    let manifest = loadJson(args[1])
    let payload = dispatchPlanForManifest(manifest)
    var outPath = ""
    if args.len == 3 and args[2] == "--output":
      echo "--output requires a path"
      return 1
    if args.len >= 4 and args[2] == "--output":
      outPath = args[3]
    if outPath.len == 0:
      outPath = manifestPath(dottedGetStr(manifest, "task_id", "task"),
        dottedGetStr(manifest, "topic", "topic")).replace(".manifest.json", ".dispatch-plan.json")
    echo writeJson(outPath, payload)
    return (if dottedGetInt(payload, "agent_count", 0) > 0: 0 else: 2)

  of "session-plan":
    if args.len < 2:
      echo "Usage: taskflow-v0 problem-party session-plan <board_manifest.json> [--output PATH]"
      return 1
    let manifest = loadJson(args[1])
    let payload = sessionPlanForManifest(manifest)
    var outPath = ""
    if args.len == 3 and args[2] == "--output":
      echo "--output requires a path"
      return 1
    if args.len >= 4 and args[2] == "--output":
      outPath = args[3]
    if outPath.len == 0:
      outPath = manifestPath(dottedGetStr(manifest, "task_id", "task"),
        dottedGetStr(manifest, "topic", "topic")).replace(".manifest.json", ".session-plan.json")
    echo writeJson(outPath, payload)
    return (if dottedGetBool(payload, "binding_validation.valid", false): 0 else: 2)

  of "execute":
    if args.len < 5:
      echo "Usage: taskflow-v0 problem-party execute <task_id> <task_class> <board_manifest.json> <role_notes.json> [--output PATH]"
      return 1
    let (exitCode, payload) = executeProblemParty(args[1], args[2], args[3], args[4])
    var outPath = ""
    if args.len == 6 and args[5] == "--output":
      echo "--output requires a path"
      return 1
    if args.len >= 7 and args[5] == "--output":
      outPath = args[6]
    if outPath.len == 0:
      outPath = executeArtifactPath(args[1], dottedGetStr(payload, "topic", "topic"))
    echo writeJson(outPath, payload)
    return exitCode

  of "synthesize":
    if args.len < 3:
      echo "Usage: taskflow-v0 problem-party synthesize <board_manifest.json> <role_notes.json> [--output PATH]"
      return 1
    let payload = synthesizeProblemParty(args[1], args[2])
    var outPath = ""
    if args.len == 4 and args[3] == "--output":
      echo "--output requires a path"
      return 1
    if args.len >= 5 and args[3] == "--output":
      outPath = args[4]
    if outPath.len == 0:
      outPath = receiptPath(dottedGetStr(payload, "task_id", "task"), dottedGetStr(payload, "topic", "topic"))
    echo writeJson(outPath, payload)
    return 0

  of "receipt":
    if args.len < 5:
      echo "Usage: taskflow-v0 problem-party receipt <task_id> <task_class> \"<topic>\" <decision_artifact.json>"
      return 1
    let decisionArtifact = loadJson(args[4])
    echo writeProblemPartyReceipt(args[1], args[2], args[3], decisionArtifact)
    return 0

  else:
    echo "Unknown problem-party subcommand: " & args[0]
    return 1
