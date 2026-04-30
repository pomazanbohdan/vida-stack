pub(crate) const DEFAULT_AGENT_EXTENSION_ROLES_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/roles.yaml");
pub(crate) const DEFAULT_AGENT_EXTENSION_SKILLS_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/skills.yaml");
pub(crate) const DEFAULT_AGENT_EXTENSION_PROFILES_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/profiles.yaml");
pub(crate) const DEFAULT_AGENT_EXTENSION_FLOWS_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/flows.yaml");
pub(crate) const DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_YAML: &str =
    include_str!("../../../docs/process/agent-extensions/dispatch-aliases.yaml");
pub(crate) const DEFAULT_RUNTIME_AGENT_EXTENSIONS_README: &str = r#"# Runtime Agent Extensions

This directory holds the active runtime-owned agent-extension projections for the project.

Runtime rule:

1. `.vida/project/agent-extensions/*.yaml` is the active project-local runtime projection family.
2. Matching `*.sidecar.yaml` files are the editable override surfaces for project-local changes.
3. Root `docs/process/agent-extensions/**` remains source/export/import lineage only; it is not the live runtime source.
4. Edited sidecars become active only through runtime validation and import-safe execution paths.
"#;
pub(crate) const DEFAULT_AGENT_EXTENSION_ROLES_SIDECAR_YAML: &str = "version: 1\nroles: []\n";
pub(crate) const DEFAULT_AGENT_EXTENSION_SKILLS_SIDECAR_YAML: &str = "version: 1\nskills: []\n";
pub(crate) const DEFAULT_AGENT_EXTENSION_PROFILES_SIDECAR_YAML: &str = "version: 1\nprofiles: []\n";
pub(crate) const DEFAULT_AGENT_EXTENSION_FLOWS_SIDECAR_YAML: &str = "version: 1\nflow_sets: []\n";
pub(crate) const DEFAULT_AGENT_EXTENSION_DISPATCH_ALIASES_SIDECAR_YAML: &str =
    "version: 1\ndispatch_aliases: []\n";

pub(crate) const PROJECT_ID_PLACEHOLDER: &str = "__PROJECT_ID__";
pub(crate) const DOCS_ROOT_PLACEHOLDER: &str = "__DOCS_ROOT__";
pub(crate) const PROCESS_ROOT_PLACEHOLDER: &str = "__PROCESS_ROOT__";
pub(crate) const RESEARCH_ROOT_PLACEHOLDER: &str = "__RESEARCH_ROOT__";
pub(crate) const README_DOC_PLACEHOLDER: &str = "__README_DOC__";
pub(crate) const ARCHITECTURE_DOC_PLACEHOLDER: &str = "__ARCHITECTURE_DOC__";
pub(crate) const DECISIONS_DOC_PLACEHOLDER: &str = "__DECISIONS_DOC__";
pub(crate) const ENVIRONMENTS_DOC_PLACEHOLDER: &str = "__ENVIRONMENTS_DOC__";
pub(crate) const PROJECT_OPERATIONS_DOC_PLACEHOLDER: &str = "__PROJECT_OPERATIONS_DOC__";
pub(crate) const AGENT_SYSTEM_DOC_PLACEHOLDER: &str = "__AGENT_SYSTEM_DOC__";
pub(crate) const USER_COMMUNICATION_PLACEHOLDER: &str = "__USER_COMMUNICATION__";
pub(crate) const REASONING_LANGUAGE_PLACEHOLDER: &str = "__REASONING_LANGUAGE__";
pub(crate) const DOCUMENTATION_LANGUAGE_PLACEHOLDER: &str = "__DOCUMENTATION_LANGUAGE__";
pub(crate) const TODO_PROTOCOL_LANGUAGE_PLACEHOLDER: &str = "__TODO_PROTOCOL_LANGUAGE__";

pub(crate) const DEFAULT_PROJECT_DOCS_ROOT: &str = "docs";
pub(crate) const DEFAULT_PROJECT_PROCESS_ROOT: &str = "docs/process";
pub(crate) const DEFAULT_PROJECT_RESEARCH_ROOT: &str = "docs/research";
pub(crate) const DEFAULT_PROJECT_ROOT_MAP: &str = "docs/project-root-map.md";
pub(crate) const DEFAULT_PROJECT_PRODUCT_INDEX: &str = "docs/product/index.md";
pub(crate) const DEFAULT_PROJECT_PRODUCT_SPEC_README: &str = "docs/product/spec/README.md";
pub(crate) const DEFAULT_PROJECT_FEATURE_DESIGN_TEMPLATE: &str =
    "docs/product/spec/templates/feature-design-document.template.md";
pub(crate) const DEFAULT_PROJECT_ARCHITECTURE_DOC: &str = "docs/product/architecture.md";
pub(crate) const DEFAULT_PROJECT_PROCESS_README: &str = "docs/process/README.md";
pub(crate) const DEFAULT_PROJECT_DECISIONS_DOC: &str = "docs/process/decisions.md";
pub(crate) const DEFAULT_PROJECT_ENVIRONMENTS_DOC: &str = "docs/process/environments.md";
pub(crate) const DEFAULT_PROJECT_OPERATIONS_DOC: &str = "docs/process/project-operations.md";
pub(crate) const DEFAULT_PROJECT_AGENT_SYSTEM_DOC: &str = "docs/process/agent-system.md";
pub(crate) const DEFAULT_PROJECT_AGENT_GUIDE_DOC: &str = DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC;
pub(crate) const DEFAULT_PROJECT_HOST_AGENT_GUIDE_DOC: &str =
    "docs/process/codex-agent-configuration-guide.md";
pub(crate) const DEFAULT_PROJECT_DOC_TOOLING_DOC: &str =
    "docs/process/documentation-tooling-map.md";
pub(crate) const DEFAULT_PROJECT_ORCHESTRATOR_STARTUP_BUNDLE: &str =
    "docs/process/project-orchestrator-startup-bundle.md";
pub(crate) const DEFAULT_PROJECT_PACKET_AND_LANE_RUNTIME_CAPSULE: &str =
    "docs/process/project-packet-and-lane-runtime-capsule.md";
pub(crate) const DEFAULT_PROJECT_START_READINESS_RUNTIME_CAPSULE: &str =
    "docs/process/project-start-readiness-runtime-capsule.md";
pub(crate) const DEFAULT_PROJECT_PACKET_RENDERING_RUNTIME_CAPSULE: &str =
    "docs/process/project-packet-rendering-runtime-capsule.md";
pub(crate) const DEFAULT_PROJECT_RESEARCH_README: &str = "docs/research/README.md";
pub(crate) const PROJECT_ACTIVATION_RECEIPT_LATEST: &str =
    ".vida/receipts/project-activation.latest.json";
pub(crate) const SPEC_BOOTSTRAP_RECEIPT_LATEST: &str = ".vida/receipts/spec-bootstrap.latest.json";
