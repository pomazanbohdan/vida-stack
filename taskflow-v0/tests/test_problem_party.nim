import std/[json, os, strutils, unittest]
import ../src/core/utils
import ../src/state/problem_party
import ../src/state/run_graph

suite "problem party runtime":
  let root = "/tmp/vida_problem_party_runtime"
  discard existsOrCreateDir(root)
  discard existsOrCreateDir(root / "docs")
  discard existsOrCreateDir(root / "docs" / "process")
  discard existsOrCreateDir(root / "docs" / "process" / "agent-extensions")
  discard existsOrCreateDir(root / ".vida")
  discard existsOrCreateDir(root / ".vida" / "logs")
  discard existsOrCreateDir(root / ".vida" / "logs" / "problem-party")
  writeFile(
    root / "vida.config.yaml",
    """
agent_extensions:
  enabled: true
  registries:
    roles: docs/process/agent-extensions/roles.yaml
    skills: docs/process/agent-extensions/skills.yaml
    profiles: docs/process/agent-extensions/profiles.yaml
    flows: docs/process/agent-extensions/flows.yaml
  enabled_framework_roles:
    - orchestrator
    - worker
    - business_analyst
    - pm
    - coach
    - verifier
  enabled_standard_flow_sets:
    - minimal
  enabled_project_roles:
    - party_chat_facilitator
    - party_chat_architect
    - party_chat_runtime_systems
    - party_chat_quality_verification
    - party_chat_delivery_cost
    - party_chat_product_scope
    - party_chat_security_safety
    - party_chat_sre_observability
    - party_chat_data_contracts
    - party_chat_dx_tooling
    - party_chat_pm_process
  enabled_project_skills:
    - party_chat_council_reasoning
    - party_chat_architecture_reasoning
    - party_chat_runtime_reasoning
    - party_chat_verification_reasoning
    - party_chat_delivery_tradeoffs
    - party_chat_product_scope
    - party_chat_security_safety
    - party_chat_observability
    - party_chat_data_contracts
    - party_chat_dx_tooling
    - party_chat_pm_process
  enabled_project_profiles:
    - party_chat_facilitator_profile
    - party_chat_architect_profile
    - party_chat_runtime_systems_profile
    - party_chat_quality_verification_profile
    - party_chat_delivery_cost_profile
    - party_chat_product_scope_profile
    - party_chat_security_safety_profile
    - party_chat_sre_observability_profile
    - party_chat_data_contracts_profile
    - party_chat_dx_tooling_profile
    - party_chat_pm_process_profile
  enabled_project_flows:
    - party_chat_council_small
    - party_chat_council_large
  default_flow_set: minimal
party_chat:
  enabled: true
  replaces_problem_party: true
  execution_mode: multi_agent
  model_routing_strategy: by_role
  default_board_size: small
  min_experts: 2
  max_experts: 6
  hard_cap_agents: 8
  dei_enabled: true
  single_agent:
    backend: qwen_cli
    model: qwen-max
  role_model_bindings:
    party_chat_facilitator:
      backend: qwen_cli
      model: qwen-max
    party_chat_architect:
      backend: qwen_cli
      model: qwen-max
    party_chat_runtime_systems:
      backend: qwen_cli
      model: qwen-plus
    party_chat_quality_verification:
      backend: qwen_cli
      model: qwen-max
    party_chat_delivery_cost:
      backend: minimax_cli
      model: minimax-design
"""
  )
  writeFile(
    root / "docs" / "process" / "agent-extensions" / "roles.yaml",
    """
version: 1
roles:
  - role_id: party_chat_facilitator
    base_role: business_analyst
  - role_id: party_chat_architect
    base_role: worker
  - role_id: party_chat_runtime_systems
    base_role: worker
  - role_id: party_chat_quality_verification
    base_role: verifier
  - role_id: party_chat_delivery_cost
    base_role: pm
  - role_id: party_chat_product_scope
    base_role: pm
  - role_id: party_chat_security_safety
    base_role: verifier
  - role_id: party_chat_sre_observability
    base_role: coach
  - role_id: party_chat_data_contracts
    base_role: worker
  - role_id: party_chat_dx_tooling
    base_role: worker
  - role_id: party_chat_pm_process
    base_role: pm
"""
  )
  writeFile(
    root / "docs" / "process" / "agent-extensions" / "skills.yaml",
    """
version: 1
skills:
  - skill_id: party_chat_council_reasoning
    compatible_base_roles: business_analyst,worker,verifier,pm,coach
  - skill_id: party_chat_architecture_reasoning
    compatible_base_roles: worker,business_analyst
  - skill_id: party_chat_runtime_reasoning
    compatible_base_roles: worker,coach
  - skill_id: party_chat_verification_reasoning
    compatible_base_roles: verifier,coach
  - skill_id: party_chat_delivery_tradeoffs
    compatible_base_roles: pm,coach
  - skill_id: party_chat_product_scope
    compatible_base_roles: pm,business_analyst
  - skill_id: party_chat_security_safety
    compatible_base_roles: verifier,worker,coach
  - skill_id: party_chat_observability
    compatible_base_roles: coach,worker
  - skill_id: party_chat_data_contracts
    compatible_base_roles: worker,verifier
  - skill_id: party_chat_dx_tooling
    compatible_base_roles: worker,coach
  - skill_id: party_chat_pm_process
    compatible_base_roles: pm,coach
"""
  )
  writeFile(
    root / "docs" / "process" / "agent-extensions" / "profiles.yaml",
    """
version: 1
profiles:
  - profile_id: party_chat_facilitator_profile
    role_ref: party_chat_facilitator
    skill_refs: party_chat_council_reasoning
    stance: facilitator
    preferred_backend: qwen_cli
    preferred_model: qwen-max
  - profile_id: party_chat_architect_profile
    role_ref: party_chat_architect
    skill_refs: party_chat_council_reasoning,party_chat_architecture_reasoning
    stance: architect
    preferred_backend: qwen_cli
    preferred_model: qwen-max
  - profile_id: party_chat_runtime_systems_profile
    role_ref: party_chat_runtime_systems
    skill_refs: party_chat_council_reasoning,party_chat_runtime_reasoning
    stance: runtime
    preferred_backend: qwen_cli
    preferred_model: qwen-plus
  - profile_id: party_chat_quality_verification_profile
    role_ref: party_chat_quality_verification
    skill_refs: party_chat_council_reasoning,party_chat_verification_reasoning
    stance: verifier
    preferred_backend: qwen_cli
    preferred_model: qwen-max
  - profile_id: party_chat_delivery_cost_profile
    role_ref: party_chat_delivery_cost
    skill_refs: party_chat_council_reasoning,party_chat_delivery_tradeoffs
    stance: delivery
    preferred_backend: minimax_cli
    preferred_model: minimax-design
  - profile_id: party_chat_product_scope_profile
    role_ref: party_chat_product_scope
    skill_refs: party_chat_council_reasoning,party_chat_product_scope
    stance: scope
  - profile_id: party_chat_security_safety_profile
    role_ref: party_chat_security_safety
    skill_refs: party_chat_council_reasoning,party_chat_security_safety
    stance: security
  - profile_id: party_chat_sre_observability_profile
    role_ref: party_chat_sre_observability
    skill_refs: party_chat_council_reasoning,party_chat_observability
    stance: observability
  - profile_id: party_chat_data_contracts_profile
    role_ref: party_chat_data_contracts
    skill_refs: party_chat_council_reasoning,party_chat_data_contracts
    stance: data
  - profile_id: party_chat_dx_tooling_profile
    role_ref: party_chat_dx_tooling
    skill_refs: party_chat_council_reasoning,party_chat_dx_tooling
    stance: dx
  - profile_id: party_chat_pm_process_profile
    role_ref: party_chat_pm_process
    skill_refs: party_chat_council_reasoning,party_chat_pm_process
    stance: process
"""
  )
  writeFile(
    root / "docs" / "process" / "agent-extensions" / "flows.yaml",
    """
version: 1
flow_sets:
  - flow_id: party_chat_council_small
    role_chain: party_chat_facilitator,party_chat_architect,party_chat_runtime_systems,party_chat_quality_verification,party_chat_delivery_cost
  - flow_id: party_chat_council_large
    role_chain: party_chat_facilitator,party_chat_architect,party_chat_runtime_systems,party_chat_quality_verification,party_chat_delivery_cost,party_chat_product_scope,party_chat_security_safety,party_chat_sre_observability,party_chat_data_contracts,party_chat_dx_tooling,party_chat_pm_process
"""
  )
  putEnv("VIDA_ROOT", root)

  test "render manifest uses project profiles for small board":
    let payload = renderProblemPartyManifest("TASK-PP-1", "runtime decision", "small")
    check payload["board_size"].getStr() == "small"
    check payload["role_bindings"].len == 4
    check payload["facilitator"]["profile"]["profile_id"].getStr() == "party_chat_facilitator_profile"
    check payload["flow_binding"]["flow_id"].getStr() == "party_chat_council_small"
    check payload["profile_bindings"][0].getStr() == "party_chat_architect_profile"
    check payload["party_chat_config"]["execution_mode"].getStr() == "multi_agent"
    check payload["party_chat_config"]["single_agent"]["backend"].getStr() == "qwen_cli"
    check payload["runtime_agents"][0]["preferred_backend"].getStr() == "qwen_cli"
    check payload["runtime_agents"][3]["role_model_binding"]["backend"].getStr() == "minimax_cli"

  test "dispatch plan expands to separate facilitator and role agents in multi-agent mode":
    let payload = renderProblemPartyManifest("TASK-PP-PLAN", "runtime decision", "small")
    let dispatch = dispatchPlanForManifest(payload)
    check dispatch["execution_mode"].getStr() == "multi_agent"
    check dispatch["agent_count"].getInt() == 5
    check dispatch["agents"][0]["seat"].getStr() == "facilitator"
    check dispatch["agents"][1]["backend"].getStr() == "qwen_cli"

  test "session plan expands rounds from manifest":
    let payload = renderProblemPartyManifest("TASK-PP-SESSION", "runtime decision", "large")
    let sessionPlan = sessionPlanForManifest(payload)
    check sessionPlan["round_count"].getInt() == 2
    check sessionPlan["rounds"].len == 2
    check sessionPlan["rounds"][0]["stage"].getStr() == "proposal"
    check sessionPlan["rounds"][1]["stage"].getStr() == "synthesis"
    check sessionPlan["binding_validation"]["valid"].getBool() == true

  test "large board renders full role chain with default two rounds":
    let payload = renderProblemPartyManifest("TASK-PP-LARGE", "cross domain decision", "large")
    check payload["board_size"].getStr() == "large"
    check payload["round_count"].getInt() == 2
    check payload["role_bindings"].len == 6
    check payload["flow_binding"]["flow_id"].getStr() == "party_chat_council_large"
    check payload["party_chat_config"]["effective_expert_count"].getInt() == 6
    check payload["party_chat_config"]["effective_agent_count"].getInt() == 7

  test "custom rounds override default board rounds":
    let payload = renderProblemPartyManifest("TASK-PP-ROUNDS", "runtime decision", "small", rounds = 3)
    check payload["round_count"].getInt() == 3

  test "receipt marks problem_party completed and writer ready":
    discard updateNode("TASK-PP-2", "implementation", "problem_party", "running")
    let decisionPath = root / ".vida" / "logs" / "problem-party" / "decision.json"
    writeFile(
      decisionPath,
      """{
  "decision": "use party chat small board",
  "next_execution_step": "writer",
  "confidence": "high"
}
"""
    )
    let path = writeProblemPartyReceipt("TASK-PP-2", "implementation", "runtime decision", loadJson(decisionPath))
    let graph = loadGraph("TASK-PP-2")
    check fileExists(path)
    check graph["nodes"]["problem_party"]["status"].getStr() == "completed"
    check graph["nodes"]["writer"]["status"].getStr() == "ready"

  test "dispatch plan collapses to one agent in single-agent mode":
    let singleRoot = "/tmp/vida_problem_party_runtime_single"
    discard existsOrCreateDir(singleRoot)
    discard existsOrCreateDir(singleRoot / "docs")
    discard existsOrCreateDir(singleRoot / "docs" / "process")
    discard existsOrCreateDir(singleRoot / "docs" / "process" / "agent-extensions")
    discard existsOrCreateDir(singleRoot / ".vida")
    discard existsOrCreateDir(singleRoot / ".vida" / "logs")
    discard existsOrCreateDir(singleRoot / ".vida" / "logs" / "problem-party")
    writeFile(singleRoot / "vida.config.yaml", readFile(root / "vida.config.yaml").replace(
      "execution_mode: multi_agent", "execution_mode: single_agent"))
    writeFile(singleRoot / "docs" / "process" / "agent-extensions" / "roles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "roles.yaml"))
    writeFile(singleRoot / "docs" / "process" / "agent-extensions" / "skills.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "skills.yaml"))
    writeFile(singleRoot / "docs" / "process" / "agent-extensions" / "profiles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "profiles.yaml"))
    writeFile(singleRoot / "docs" / "process" / "agent-extensions" / "flows.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "flows.yaml"))
    putEnv("VIDA_ROOT", singleRoot)
    let singlePayload = renderProblemPartyManifest("TASK-PP-SINGLE", "runtime decision", "small")
    let singleDispatch = dispatchPlanForManifest(singlePayload)
    check singleDispatch["execution_mode"].getStr() == "single_agent"
    check singleDispatch["agent_count"].getInt() == 1
    check singleDispatch["agents"][0]["backend"].getStr() == "qwen_cli"
    putEnv("VIDA_ROOT", root)

  test "single-agent mode fails closed when backend or model is missing":
    let brokenSingleRoot = "/tmp/vida_problem_party_runtime_single_invalid"
    discard existsOrCreateDir(brokenSingleRoot)
    discard existsOrCreateDir(brokenSingleRoot / "docs")
    discard existsOrCreateDir(brokenSingleRoot / "docs" / "process")
    discard existsOrCreateDir(brokenSingleRoot / "docs" / "process" / "agent-extensions")
    discard existsOrCreateDir(brokenSingleRoot / ".vida")
    discard existsOrCreateDir(brokenSingleRoot / ".vida" / "logs")
    discard existsOrCreateDir(brokenSingleRoot / ".vida" / "logs" / "problem-party")
    writeFile(
      brokenSingleRoot / "vida.config.yaml",
      readFile(root / "vida.config.yaml")
        .replace("execution_mode: multi_agent", "execution_mode: single_agent")
        .replace("    backend: qwen_cli\n    model: qwen-max\n", "    backend: \n    model: \n")
    )
    writeFile(brokenSingleRoot / "docs" / "process" / "agent-extensions" / "roles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "roles.yaml"))
    writeFile(brokenSingleRoot / "docs" / "process" / "agent-extensions" / "skills.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "skills.yaml"))
    writeFile(brokenSingleRoot / "docs" / "process" / "agent-extensions" / "profiles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "profiles.yaml"))
    writeFile(brokenSingleRoot / "docs" / "process" / "agent-extensions" / "flows.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "flows.yaml"))
    putEnv("VIDA_ROOT", brokenSingleRoot)
    let brokenPayload = renderProblemPartyManifest("TASK-PP-SINGLE-BROKEN", "runtime decision", "small")
    let brokenDispatch = dispatchPlanForManifest(brokenPayload)
    check brokenPayload["binding_validation"]["valid"].getBool() == false
    check brokenPayload["binding_validation"]["errors"].len == 2
    check brokenDispatch["status"].getStr() == "invalid_manifest"
    check brokenDispatch["agent_count"].getInt() == 0
    putEnv("VIDA_ROOT", root)

  test "render respects max experts and hard cap":
    let cappedRoot = "/tmp/vida_problem_party_runtime_capped"
    discard existsOrCreateDir(cappedRoot)
    discard existsOrCreateDir(cappedRoot / "docs")
    discard existsOrCreateDir(cappedRoot / "docs" / "process")
    discard existsOrCreateDir(cappedRoot / "docs" / "process" / "agent-extensions")
    discard existsOrCreateDir(cappedRoot / ".vida")
    discard existsOrCreateDir(cappedRoot / ".vida" / "logs")
    discard existsOrCreateDir(cappedRoot / ".vida" / "logs" / "problem-party")
    writeFile(cappedRoot / "vida.config.yaml", readFile(root / "vida.config.yaml")
      .replace("max_experts: 6", "max_experts: 3")
      .replace("hard_cap_agents: 8", "hard_cap_agents: 4"))
    writeFile(cappedRoot / "docs" / "process" / "agent-extensions" / "roles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "roles.yaml"))
    writeFile(cappedRoot / "docs" / "process" / "agent-extensions" / "skills.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "skills.yaml"))
    writeFile(cappedRoot / "docs" / "process" / "agent-extensions" / "profiles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "profiles.yaml"))
    writeFile(cappedRoot / "docs" / "process" / "agent-extensions" / "flows.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "flows.yaml"))
    putEnv("VIDA_ROOT", cappedRoot)
    let cappedPayload = renderProblemPartyManifest("TASK-PP-CAPPED", "cross domain decision", "large")
    let cappedDispatch = dispatchPlanForManifest(cappedPayload)
    check cappedPayload["role_bindings"].len == 3
    check cappedPayload["party_chat_config"]["effective_expert_count"].getInt() == 3
    check cappedDispatch["agent_count"].getInt() == 4
    putEnv("VIDA_ROOT", root)

  test "render fails closed when hard cap cannot satisfy min experts":
    let impossibleRoot = "/tmp/vida_problem_party_runtime_impossible"
    discard existsOrCreateDir(impossibleRoot)
    discard existsOrCreateDir(impossibleRoot / "docs")
    discard existsOrCreateDir(impossibleRoot / "docs" / "process")
    discard existsOrCreateDir(impossibleRoot / "docs" / "process" / "agent-extensions")
    discard existsOrCreateDir(impossibleRoot / ".vida")
    discard existsOrCreateDir(impossibleRoot / ".vida" / "logs")
    discard existsOrCreateDir(impossibleRoot / ".vida" / "logs" / "problem-party")
    writeFile(impossibleRoot / "vida.config.yaml", readFile(root / "vida.config.yaml")
      .replace("min_experts: 2", "min_experts: 4")
      .replace("hard_cap_agents: 8", "hard_cap_agents: 2"))
    writeFile(impossibleRoot / "docs" / "process" / "agent-extensions" / "roles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "roles.yaml"))
    writeFile(impossibleRoot / "docs" / "process" / "agent-extensions" / "skills.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "skills.yaml"))
    writeFile(impossibleRoot / "docs" / "process" / "agent-extensions" / "profiles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "profiles.yaml"))
    writeFile(impossibleRoot / "docs" / "process" / "agent-extensions" / "flows.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "flows.yaml"))
    putEnv("VIDA_ROOT", impossibleRoot)
    let impossiblePayload = renderProblemPartyManifest("TASK-PP-IMPOSSIBLE", "runtime decision", "small")
    let impossibleDispatch = dispatchPlanForManifest(impossiblePayload)
    check impossiblePayload["binding_validation"]["valid"].getBool() == false
    check impossiblePayload["status"].getStr() == "invalid_manifest"
    check impossiblePayload["binding_validation"]["errors"].len >= 1
    check impossibleDispatch["status"].getStr() == "invalid_manifest"
    putEnv("VIDA_ROOT", root)

  test "render reports invalid bindings when required profile is missing":
    let invalidRoot = "/tmp/vida_problem_party_runtime_invalid"
    discard existsOrCreateDir(invalidRoot)
    discard existsOrCreateDir(invalidRoot / "docs")
    discard existsOrCreateDir(invalidRoot / "docs" / "process")
    discard existsOrCreateDir(invalidRoot / "docs" / "process" / "agent-extensions")
    discard existsOrCreateDir(invalidRoot / ".vida")
    discard existsOrCreateDir(invalidRoot / ".vida" / "logs")
    discard existsOrCreateDir(invalidRoot / ".vida" / "logs" / "problem-party")
    writeFile(invalidRoot / "vida.config.yaml", readFile(root / "vida.config.yaml"))
    writeFile(invalidRoot / "docs" / "process" / "agent-extensions" / "roles.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "roles.yaml"))
    writeFile(invalidRoot / "docs" / "process" / "agent-extensions" / "skills.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "skills.yaml"))
    writeFile(
      invalidRoot / "docs" / "process" / "agent-extensions" / "profiles.yaml",
      """
version: 1
profiles:
  - profile_id: party_chat_architect_profile
    role_ref: party_chat_architect
    skill_refs: party_chat_council_reasoning,party_chat_architecture_reasoning
    stance: architect
    preferred_backend: qwen_cli
    preferred_model: qwen-max
  - profile_id: party_chat_runtime_systems_profile
    role_ref: party_chat_runtime_systems
    skill_refs: party_chat_council_reasoning,party_chat_runtime_reasoning
    stance: runtime
    preferred_backend: qwen_cli
    preferred_model: qwen-plus
  - profile_id: party_chat_quality_verification_profile
    role_ref: party_chat_quality_verification
    skill_refs: party_chat_council_reasoning,party_chat_verification_reasoning
    stance: verifier
    preferred_backend: qwen_cli
    preferred_model: qwen-max
  - profile_id: party_chat_delivery_cost_profile
    role_ref: party_chat_delivery_cost
    skill_refs: party_chat_council_reasoning,party_chat_delivery_tradeoffs
    stance: delivery
    preferred_backend: minimax_cli
    preferred_model: minimax-design
  - profile_id: party_chat_product_scope_profile
    role_ref: party_chat_product_scope
    skill_refs: party_chat_council_reasoning,party_chat_product_scope
    stance: scope
  - profile_id: party_chat_security_safety_profile
    role_ref: party_chat_security_safety
    skill_refs: party_chat_council_reasoning,party_chat_security_safety
    stance: security
  - profile_id: party_chat_sre_observability_profile
    role_ref: party_chat_sre_observability
    skill_refs: party_chat_council_reasoning,party_chat_observability
    stance: observability
  - profile_id: party_chat_data_contracts_profile
    role_ref: party_chat_data_contracts
    skill_refs: party_chat_council_reasoning,party_chat_data_contracts
    stance: data
  - profile_id: party_chat_dx_tooling_profile
    role_ref: party_chat_dx_tooling
    skill_refs: party_chat_council_reasoning,party_chat_dx_tooling
    stance: dx
  - profile_id: party_chat_pm_process_profile
    role_ref: party_chat_pm_process
    skill_refs: party_chat_council_reasoning,party_chat_pm_process
    stance: process
"""
    )
    writeFile(invalidRoot / "docs" / "process" / "agent-extensions" / "flows.yaml",
      readFile(root / "docs" / "process" / "agent-extensions" / "flows.yaml"))
    putEnv("VIDA_ROOT", invalidRoot)
    let invalidPayload = renderProblemPartyManifest("TASK-PP-INVALID", "runtime decision", "small")
    check invalidPayload["binding_validation"]["valid"].getBool() == false
    check invalidPayload["status"].getStr() == "invalid_manifest"
    check invalidPayload["binding_validation"]["errors"].len > 0
    putEnv("VIDA_ROOT", root)

  test "dispatch-plan and synthesize reject bare output flag":
    let payload = renderProblemPartyManifest("TASK-PP-CLI", "runtime decision", "small")
    let manifest = root / ".vida" / "logs" / "problem-party" / "TASK-PP-CLI.runtime-decision.manifest.json"
    discard writeJson(manifest, payload)
    let roleNotes = root / ".vida" / "logs" / "problem-party" / "role-notes.json"
    writeFile(roleNotes, "[]")
    check cmdProblemParty(@["dispatch-plan", manifest, "--output"]) == 1
    check cmdProblemParty(@["session-plan", manifest, "--output"]) == 1
    check cmdProblemParty(@["synthesize", manifest, roleNotes, "--output"]) == 1

  test "synthesize emits decision verification and execution packets":
    let payload = renderProblemPartyManifest("TASK-PP-SYNTH", "runtime decision", "small")
    let manifest = root / ".vida" / "logs" / "problem-party" / "TASK-PP-SYNTH.runtime-decision.manifest.json"
    discard writeJson(manifest, payload)
    let roleNotes = root / ".vida" / "logs" / "problem-party" / "role-notes-synth.json"
    writeFile(roleNotes, """[
  {
    "role_id": "party_chat_architect",
    "recommendations": ["use bounded council receipt"],
    "verification_checks": ["nim test"],
    "execution_steps": ["write receipt", "resume writer"]
  },
  {
    "role_id": "party_chat_quality_verification",
    "open_risks": ["missing verifier replay"]
  }
]""")
    let synthesized = synthesizeProblemParty(manifest, roleNotes)
    check synthesized["decision_packet"]["decision"].getStr() == "use bounded council receipt"
    check synthesized["verification_packet"]["required_checks"].len == 1
    check synthesized["verification_packet"]["open_risks"].len == 1
    check synthesized["execution_packet"]["execution_steps"].len == 2
    check synthesized["execution_packet"]["writer_unblocked"].getBool() == true

  test "execute writes session, prompts, and receipt artifacts":
    let payload = renderProblemPartyManifest("TASK-PP-EXEC", "runtime decision", "small")
    let manifest = root / ".vida" / "logs" / "problem-party" / "TASK-PP-EXEC.runtime-decision.manifest.json"
    discard writeJson(manifest, payload)
    let roleNotes = root / ".vida" / "logs" / "problem-party" / "role-notes-exec.json"
    writeFile(roleNotes, """[
  {
    "role_id": "party_chat_architect",
    "recommendations": ["resume bounded writer flow"],
    "verification_checks": ["nim test"],
    "execution_steps": ["write receipt", "resume writer"]
  }
]""")
    let (exitCode, executionPayload) = executeProblemParty("TASK-PP-EXEC", "implementation", manifest, roleNotes)
    check exitCode == 0
    check executionPayload["status"].getStr() == "executed"
    check fileExists(executionPayload["dispatch_plan_path"].getStr())
    check fileExists(executionPayload["session_plan_path"].getStr())
    check fileExists(executionPayload["seat_prompts_path"].getStr())
    check fileExists(executionPayload["receipt_path"].getStr())
    let prompts = loadJson(executionPayload["seat_prompts_path"].getStr())
    check prompts.len == 5
    let graph = loadGraph("TASK-PP-EXEC")
    check graph["nodes"]["problem_party"]["status"].getStr() == "completed"
    check graph["nodes"]["writer"]["status"].getStr() == "ready"
