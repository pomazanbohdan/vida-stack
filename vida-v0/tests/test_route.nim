## Tests for agents/route module

import std/[json, os, sequtils, unittest]
import ../src/agents/route
import ../src/core/projection_engine

suite "route snapshot":
  let root = "/tmp/vida_scripts_nim_route"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)
  writeFile(
    root / "vida.config.yaml",
    """
agent_system:
  mode: hybrid
  routing:
    implementation:
      subagents:
        - codex_cli
      write_scope: scoped_only
      external_first_required: true
  subagents:
    codex_cli:
      enabled: true
      detect_command: sh
      write_scope: scoped_only
      capability_band:
        - bounded_write_safe
      billing_tier: low
      speed_tier: medium
      quality_tier: high
"""
  )

  test "implementation route respects config-driven scoped write policy":
    let (_, route) = routeSnapshot("implementation", "vida-route-1")
    check route["selected_subagent"].getStr() == "codex_cli"
    check route["write_scope"].getStr() == "scoped_only"
    check route["risk_class"].getStr() == "R2"
    check route["dispatch_policy"]["local_execution_allowed"].getStr() == "no"
    check route["dispatch_policy"]["external_first_required"].getStr() == "yes"
    check route["verification_plan"]["reason"].getStr() == "no_eligible_verifier"

suite "route snapshot with root config law":
  let root = "/tmp/vida_scripts_nim_route_root_config"
  discard existsOrCreateDir(root)
  discard existsOrCreateDir(root / "vida")
  discard existsOrCreateDir(root / "vida" / "config")
  putEnv("VIDA_ROOT", root)
  discard existsOrCreateDir(root / "vida" / "config" / "routes")
  discard existsOrCreateDir(root / "vida" / "config" / "agents")
  discard existsOrCreateDir(root / "vida" / "config" / "policies")

  writeFile(
    root / "vida.config.yaml",
    """
protocol_activation:
  agent_system: true
agent_system:
  mode: hybrid
  subagents:
    internal_subagents:
      enabled: true
      subagent_backend_class: internal
      orchestration_tier: senior
      cost_priority: premium
      budget_cost_units: 10
      billing_tier: internal
      speed_tier: medium
      quality_tier: high
      capability_band:
        - implementation_safe
        - review_safe
      write_scope: orchestrator_native
    codex_cli:
      enabled: true
      detect_command: sh
      subagent_backend_class: external_cli
      cost_priority: fallback
      budget_cost_units: 1
      billing_tier: low
      speed_tier: medium
      quality_tier: high
      capability_band:
        - bounded_write_safe
        - review_safe
      write_scope: scoped_only
    qwen_cli:
      enabled: true
      detect_command: sh
      subagent_backend_class: external_cli
      cost_priority: highest
      budget_cost_units: 0
      billing_tier: free
      speed_tier: medium
      quality_tier: high
      capability_band:
        - read_only
        - review_safe
        - web_search
      write_scope: none
    gemini_cli:
      enabled: true
      detect_command: sh
      subagent_backend_class: external_cli
      cost_priority: highest
      budget_cost_units: 0
      billing_tier: free
      speed_tier: medium
      quality_tier: high
      capability_band:
        - review_safe
      write_scope: none
"""
  )
  writeFile(
    root / "vida" / "config" / "routes" / "route_catalog.yaml",
    """
artifact_name: route_catalog
defaults:
  primary_lane: analysis_lane
task_class_bindings:
  implementation:
    primary_lane: writer_lane
    external_first_required: true
  review_ensemble:
    primary_lane: verification_lane
  coach:
    primary_lane: coach_lane
routes:
  analysis_lane:
    required_roles:
      - analyst
    capability: analysis
    selection_strategy: capability_score
    assignment_mode: single
    independence_class: none
    preferred_capabilities:
      - web_search
  writer_lane:
    required_roles:
      - writer
    capability: implementation
    selection_strategy: least_loaded
    assignment_mode: single
    independence_class: none
  coach_lane:
    required_roles:
      - coach
    capability: review
    selection_strategy: strict_independent
    assignment_mode: single
    independence_class: not_same_agent
    external_first_required: true
    preferred_capabilities:
      - web_search
  verification_lane:
    required_roles:
      - verifier
    capability: verification
    selection_strategy: strict_independent
    assignment_mode: single
    independence_class: not_same_route_chain
    external_first_required: true
"""
  )
  writeFile(
    root / "vida" / "config" / "agents" / "agent_groups.yaml",
    """
artifact_name: agent_groups
groups:
  analysis_pool:
    workflow_role: analyst
    selector:
      capability_band_any:
        - read_only
        - review_safe
  writer_pool:
    workflow_role: writer
    selector:
      capability_band_any:
        - bounded_write_safe
        - implementation_safe
      write_scopes_any:
        - scoped_only
        - orchestrator_native
  coach_pool:
    workflow_role: coach
    selector:
      capability_band_any:
        - review_safe
  verification_pool:
    workflow_role: verifier
    selector:
      capability_band_any:
        - review_safe
  synthesis_pool:
    workflow_role: synthesizer
    selector:
      agent_types_any:
        - system_agent
      orchestration_tiers_any:
        - senior
"""
  )
  writeFile(
    root / "vida" / "config" / "policies" / "assignment_policy.yaml",
    """
artifact_name: assignment_policy
runtime:
  mode: hybrid
defaults:
  analysis_lane: capability_score
  writer_lane: least_loaded
  coach_lane: strict_independent
  verification_lane: strict_independent
"""
  )

  test "implementation route uses root config assignment and external-first":
    let (_, route) = routeSnapshot("implementation", "vida-route-root-1")
    check route["assignment_source"].getStr() == "root_config"
    check route["inventory_source"].getStr() == "overlay_runtime"
    check route["route_lane"].getStr() == "writer_lane"
    check route["selected_subagent"].getStr() == "codex_cli"
    check route["coach_plan"]["selected_subagent"].getStr() == "qwen_cli"
    check route["verification_plan"]["selected_subagent"].getStr() == "gemini_cli"

  test "route receipt hash is stable and changes with selected subagent":
    let (_, route) = routeSnapshot("implementation", "vida-route-root-2")
    let firstHash = routeReceiptHash(route)
    let secondHash = routeReceiptHash(route)
    check firstHash == secondHash

    var mutated = route
    mutated["selected_subagent"] = %"internal_subagents"
    check routeReceiptHash(mutated) != firstHash

  test "route projection emits checkpoint and listener intents":
    let payload = projectRouteSnapshot("implementation", "vida-route-root-3")
    check payload["ok"].getBool() == true
    check payload["projection"]["projection_type"].getStr() == "route_projection"
    check payload["projection"]["route_lane"].getStr() == "writer_lane"
    check payload["checkpoint"]["checkpoint_type"].getStr() == "route_checkpoint"
    check payload["checkpoint"]["next_action"].getStr() == "analysis_external_zero_budget_then_analysis_receipt"
    check "projection.refresh" in payload["listener_intents"].getElems().mapIt(it.getStr())
    check "subscription.notify" in payload["listener_intents"].getElems().mapIt(it.getStr())
