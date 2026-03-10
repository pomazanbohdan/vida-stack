type ValidationResult* = object
  valid*: bool
  errors*: seq[string]
  warnings*: seq[string]

const KnownFrameworkRoles* = [
  "orchestrator", "worker", "business_analyst", "pm", "coach", "verifier", "prover",
  "reviewer", "planner", "approver", "analyst", "writer", "synthesizer"
]

const KnownPartyChatRoles* = [
  "party_chat_facilitator",
  "party_chat_architect",
  "party_chat_runtime_systems",
  "party_chat_quality_verification",
  "party_chat_delivery_cost",
  "party_chat_product_scope",
  "party_chat_security_safety",
  "party_chat_sre_observability",
  "party_chat_data_contracts",
  "party_chat_dx_tooling",
  "party_chat_pm_process"
]

const KnownStandardFlowSets* = ["minimal", "reviewed", "verified", "governed", "durable"]
const KnownTrackedFlowEntries* = [
  "research-pack", "spec-pack", "work-pool-pack", "dev-pack", "bug-pool-pack", "reflection-pack"
]
