## VIDA Core Types — all shared type definitions in one place.
##
## Replaces the ubiquitous `dict[str, Any]` pattern from Python scripts.
## Every struct here maps to a concrete JSON shape used by the VIDA runtime.

import std/[json, options, tables]

# ─────────────────────────── Enumerations ───────────────────────────

type
  TaskClass* = enum
    tcAnalysis = "analysis"
    tcCoach = "coach"
    tcVerification = "verification"
    tcVerificationEnsemble = "verification_ensemble"
    tcReviewEnsemble = "review_ensemble"
    tcProblemParty = "problem_party"
    tcReadOnlyPrep = "read_only_prep"
    tcImplementation = "implementation"
    tcSmallPatch = "small_patch"
    tcSmallPatchWrite = "small_patch_write"
    tcUiPatch = "ui_patch"
    tcArchitecture = "architecture"
    tcMetaAnalysis = "meta_analysis"
    tcDefault = "default"

  WriteScope* = enum
    wsNone = "none"
    wsScopedOnly = "scoped_only"
    wsSandbox = "sandbox"
    wsPatch = "patch"
    wsOrchestratorNative = "orchestrator_native"
    wsExternalWrite = "external_write"
    wsRepoWrite = "repo_write"

  RiskClass* = enum
    rcR0 = "R0"
    rcR1 = "R1"
    rcR2 = "R2"
    rcR3 = "R3"

  NodeStatus* = enum
    nsPending = "pending"
    nsReady = "ready"
    nsRunning = "running"
    nsCompleted = "completed"
    nsBlocked = "blocked"
    nsFailed = "failed"
    nsSkipped = "skipped"

  LifecycleStage* = enum
    lsDeclared = "declared"
    lsProbation = "probation"
    lsActive = "active"
    lsDemoted = "demoted"
    lsRetired = "retired"

  LeaseStatus* = enum
    lsActive = "active"
    lsReleased = "released"
    lsExpired = "expired"

  AgentMode* = enum
    amDisabled = "disabled"
    amNative = "native"
    amHybrid = "hybrid"

  IssueStatus* = enum
    isOpen = "open"
    isInProgress = "in_progress"
    isBlocked = "blocked"
    isClosed = "closed"
    isDone = "done"

  BootProfile* = enum
    bpLean = "lean"
    bpStandard = "standard"
    bpFull = "full"

  CostClass* = enum
    ccFree = "free"
    ccCheap = "cheap"
    ccPaid = "paid"
    ccExpensive = "expensive"

# ─────────────────────────── BR Issue ───────────────────────────

type
  BrIssue* = object
    id*: string
    title*: string
    status*: string
    priority*: int
    labels*: seq[string]
    issueType*: string
    parent*: string
    createdAt*: string
    updatedAt*: string

  BrDependent* = object
    id*: string
    title*: string
    status*: string
    dependencyType*: string

# ─────────────────────────── Config: Top-Level Sections ───────────────────────────

type
  ProjectConfig* = object
    id*: string
    overlayVersion*: int

  ProjectBootstrap* = object
    enabled*: bool
    docsRoot*: string
    processRoot*: string
    researchRoot*: string
    readmeDoc*: string
    architectureDoc*: string
    decisionsDoc*: string
    environmentsDoc*: string
    projectOperationsDoc*: string
    agentSystemDoc*: string
    allowScaffoldMissing*: bool
    requireLaunchConfirmation*: bool

  LanguagePolicy* = object
    userCommunication*: string
    reasoning*: string
    documentation*: string
    todoProtocol*: string

  ProtocolActivation* = object
    agentSystem*: bool

  AutonomousExecution* = object
    nextTaskBoundaryAnalysis*: bool
    nextTaskBoundaryReport*: string
    nextTaskBoundaryReportGating*: bool
    dependentCoverageAutoupdate*: bool

  FrameworkSelfDiagnosis* = object
    enabled*: bool
    silentMode*: bool
    autoCaptureBugs*: bool
    parentIssue*: string
    deferFixUntilTaskBoundary*: bool
    sessionReflectionRequired*: bool
    platformDirection*: string
    qualityTokenEfficiency*: string
    sessionReflectionCriteria*: seq[string]

  PackRouterKeywords* = object
    research*: string
    spec*: string
    pool*: string
    poolStrong*: string
    poolDependency*: string
    dev*: string
    bug*: string
    reflect*: string
    reflectStrong*: string

# ─────────────────────────── Config: Agent System ───────────────────────────

type
  ## Full dispatch config per agent backend — matches overlay fields in vida.config.yaml
  AgentBackendDispatchConfig* = object
    command*: string
    preStaticArgs*: seq[string]
    subcommand*: string
    staticArgs*: seq[string]
    writeStaticArgs*: seq[string]
    probeStaticArgs*: seq[string]
    workdirFlag*: string
    modelFlag*: string
    modelsCachePath*: string
    env*: OrderedTable[string, string]
    # Web search
    webSearchMode*: string        # "flag", "provider_configured"
    webSearchFlag*: string
    webProbePrompt*: string
    webProbeExpectSubstring*: string
    webProbeTimeoutSeconds*: int
    # Probe
    probePrompt*: string
    probeExpectSubstring*: string
    probeTimeoutSeconds*: int
    # Output
    outputMode*: string           # "file", "stdout"
    outputFlag*: string
    promptMode*: string           # "positional", "flag"
    promptFlag*: string
    # Timeouts
    startupTimeoutSeconds*: int
    noOutputTimeoutSeconds*: int
    progressIdleTimeoutSeconds*: int
    maxRuntimeExtensionSeconds*: int

  AgentBackendConfig* = object
    enabled*: bool
    subagentBackendClass*: string   # "internal", "external_cli"
    detectCommand*: string
    role*: string                   # "senior_internal", "bridge_fallback", "free_primary"
    orchestrationTier*: string      # "senior", "bridge", "external_free"
    costPriority*: string           # "premium", "fallback", "highest"
    budgetCostUnits*: int
    modelsHint*: seq[string]
    defaultModel*: string
    profiles*: seq[string]
    defaultProfile*: string
    capabilityBand*: seq[string]    # "read_only", "review_safe", "bounded_write_safe", "web_search"
    writeScope*: string
    maxRuntimeSeconds*: int
    minOutputBytes*: int
    billingTier*: string            # "internal", "low", "free"
    speedTier*: string              # "fast", "medium"
    qualityTier*: string            # "high", "medium"
    specialties*: seq[string]
    dispatch*: AgentBackendDispatchConfig

  ScoringConfig* = object
    consecutiveFailureLimit*: int
    promotionScore*: int
    demotionScore*: int
    probationSuccessRuns*: int
    probationTaskRuns*: int
    retirementFailureLimit*: int

  AgentSystemConfig* = object
    initOnBoot*: bool
    mode*: string                   # "disabled", "native", "hybrid"
    stateOwner*: string             # "orchestrator_only"
    maxParallelAgents*: int
    scoring*: ScoringConfig
    agentBackends*: OrderedTable[string, AgentBackendConfig]

# ─────────────────────────── Config: Routing ───────────────────────────

type
  ## Full routing profile — matches all 35+ fields per route in vida.config.yaml
  RoutingProfile* = object
    # Agent-backend selection
    agentBackends*: seq[string]
    models*: OrderedTable[string, string]      # per-agent-backend model override
    profiles*: OrderedTable[string, string]    # per-agent-backend profile override
    # Fanout
    fanoutAgentBackends*: seq[string]
    fanoutMinResults*: int
    mergePolicy*: string            # "consensus_with_conflict_flag", "unanimous_approve_rework_bias"
    # Scope & gates
    writeScope*: string
    verificationGate*: string
    # Dispatch
    dispatchRequired*: string       # "external_first_when_eligible", "fanout_then_synthesize", etc.
    externalFirstRequired*: bool
    localExecutionAllowed*: bool
    localExecutionPreferred*: bool
    cliDispatchRequiredIfDelegating*: bool
    directInternalBypassForbidden*: bool
    bridgeFallbackAgentBackend*: string
    internalEscalationTrigger*: string
    allowedInternalReasons*: seq[string]
    # Web search
    webSearchRequired*: bool
    # Graph / budget
    graphStrategy*: string          # "deterministic_then_escalate"
    deterministicFirst*: bool
    budgetPolicy*: string           # "balanced", "strict"
    maxBudgetUnits*: int
    maxCliAgentBackendCalls*: int
    maxCoachPasses*: int
    maxVerificationPasses*: int
    maxFallbackHops*: int
    maxTotalRuntimeSeconds*: int
    maxRuntimeSeconds*: int
    minOutputBytes*: int
    # Verification
    verificationRouteTaskClass*: string
    independentVerificationRequired*: bool
    # Analysis sub-flow
    analysisRequired*: bool
    analysisRouteTaskClass*: string
    analysisFanoutAgentBackends*: seq[string]
    analysisFanoutMinResults*: int
    analysisMergePolicy*: string
    analysisExternalFirstRequired*: bool
    analysisReceiptRequired*: bool
    analysisZeroBudgetRequired*: bool
    analysisDefaultInBoot*: bool
    # Coach sub-flow
    coachRequired*: bool
    coachRouteTaskClass*: string

  RoutingConfig* = object
    routes*: OrderedTable[string, RoutingProfile]

# ─────────────────────────── Config: Root ───────────────────────────

type
  VidaConfig* = object
    project*: ProjectConfig
    projectBootstrap*: ProjectBootstrap
    languagePolicy*: LanguagePolicy
    protocolActivation*: ProtocolActivation
    autonomousExecution*: AutonomousExecution
    frameworkSelfDiagnosis*: FrameworkSelfDiagnosis
    packRouterKeywords*: PackRouterKeywords
    agentSystem*: AgentSystemConfig
    routing*: RoutingConfig

# ─────────────────────────── Run Graph ───────────────────────────

type
  RunGraphNode* = object
    status*: string
    updatedAt*: string
    attempts*: int
    meta*: JsonNode

  RunGraph* = object
    taskId*: string
    taskClass*: string
    routeTaskClass*: string
    updatedAt*: string
    nodes*: OrderedTable[string, RunGraphNode]

  ResumeHint* = object
    nextNode*: string
    status*: string
    reason*: string

# ─────────────────────────── Leases ───────────────────────────

type
  Lease* = object
    resourceType*: string
    resourceId*: string
    holder*: string
    acquiredAt*: string
    expiresAt*: string
    fencingToken*: int
    status*: string
    conflictCount*: int
    renewedAt*: string
    releasedAt*: string
    expiredAt*: string
    lastConflictAt*: string
    lastConflictHolder*: string

  LeaseStore* = object
    leases*: OrderedTable[string, Lease]
    nextFencingToken*: int
    history*: seq[JsonNode]

# ─────────────────────────── Scorecards ───────────────────────────

type
  ScoreGlobal* = object
    score*: int
    successCount*: int
    failureCount*: int
    consecutiveFailures*: int
    state*: string
    usefulProgressCount*: int
    chatterOnlyCount*: int
    preambleOnlyOutputCount*: int
    missingMachineReadablePayloadCount*: int
    lowSignalOutputCount*: int
    timeoutAfterProgressCount*: int
    startupTimeoutCount*: int
    noOutputTimeoutCount*: int
    stalledAfterProgressCount*: int
    timeToFirstUsefulOutputSamples*: int
    avgTimeToFirstUsefulOutputMs*: int
    usefulProgressRate*: int
    agentBackendState*: string
    failureReason*: string
    cooldownUntil*: string
    probeRequired*: bool
    lastQuotaExhaustedAt*: string
    recoveryAttemptCount*: int
    recoverySuccessCount*: int
    lastRecoveryAt*: string
    lastRecoveryStatus*: string
    authoredRunsCount*: int
    authoredVerifiedPassCount*: int
    authoredVerifiedFailCount*: int
    verifierRunsCount*: int
    verifierSuccessCount*: int
    verifierCatchCount*: int
    lifecycleStage*: string
    retirementReason*: string

  Scorecard* = object
    global*: ScoreGlobal
    byTaskClass*: OrderedTable[string, ScoreGlobal]
    byDomain*: OrderedTable[string, ScoreGlobal]

# ─────────────────────────── Boot ───────────────────────────

type
  BootPacket* = object
    generatedAt*: string
    profile*: string
    nonDev*: bool
    languagePolicy*: LanguagePolicy
    protocolActivation*: ProtocolActivation
    readContract*: seq[string]
    invariants*: seq[string]
    runtimeHints*: OrderedTable[string, string]

  BootSnapshotSummary* = object
    topLevelInProgress*: int
    topLevelOpen*: int
    topLevelBlocked*: int
    readyTotal*: int
    readyOpen*: int
    readyInProgress*: int
    activeRunGraphs*: int

  IssueEntry* = object
    id*: string
    title*: string
    status*: string
    mode*: string
    priority*: int
    updatedAt*: string
    runGraph*: JsonNode
    reconciliation*: JsonNode
    subtasks*: seq[JsonNode]
    hiddenSubtasks*: int

  BootSnapshot* = object
    generatedAt*: string
    executionContinueDefault*: JsonNode
    summary*: BootSnapshotSummary
    frameworkSelfDiagnosis*: JsonNode
    inProgress*: seq[IssueEntry]
    readyHead*: seq[IssueEntry]
    decisionRequired*: seq[JsonNode]
    limits*: JsonNode

# ─────────────────────────── Route / Dispatch (Runtime) ───────────────────────────
## These types represent COMPUTED route results (not in YAML, built by route resolution)

type
  FallbackAgentBackend* = object
    agentBackend*: string
    model*: string
    profile*: string
    reason*: string

  RouteBudget* = object
    budgetPolicy*: string           # "balanced", "strict"
    maxBudgetUnits*: int
    maxBudgetCostClass*: string     # "free", "cheap", "paid"
    estimatedRouteCostClass*: string
    maxCliAgentBackendCalls*: int
    maxTotalRuntimeSeconds*: int

  ResolvedRoute* = object
    ## The full route result — merges YAML config + runtime computation.
    ## This is what Python's route resolution returns as a dict.
    # Identity
    taskId*: string
    taskClass*: string
    routeTaskClass*: string
    generatedAt*: string
    # Selected agent backend (computed)
    selectedAgentBackend*: string
    selectedModel*: string
    selectedProfile*: string
    available*: bool                  # detect_command check result
    # Fallback chain (computed)
    fallbackAgentBackends*: seq[FallbackAgentBackend]
    # Risk (inferred from write_scope + verification_gate)
    riskClass*: string                # "R0"-"R3"
    # Config pass-through
    writeScope*: string
    verificationGate*: string
    dispatchRequired*: string
    externalFirstRequired*: bool
    webSearchRequired*: bool
    independentVerificationRequired*: bool
    bridgeFallbackAgentBackend*: string
    internalEscalationTrigger*: string
    # Fanout (pass-through)
    fanoutAgentBackends*: seq[string]
    fanoutMinResults*: int
    mergePolicy*: string
    # Budget (assembled from config fields)
    routeBudget*: RouteBudget
    # Sub-flow plans (assembled from analysis_*/coach_*/verification_* fields)
    analysisRequired*: bool
    analysisPlan*: JsonNode
    coachRequired*: bool
    coachPlan*: JsonNode
    verificationPlan*: JsonNode
    routeGraph*: JsonNode
    # Auth gate result
    internalRouteAuthorized*: bool
    # Dispatch policy (assembled)
    dispatchPolicy*: JsonNode

  DispatchPolicy* = object
    analysisRequired*: string
    localExecutionAllowed*: string
    externalFirstRequired*: string
    bridgeFallbackAgentBackend*: string
    internalEscalationAllowed*: string
    internalEscalationTrigger*: string
    allowedInternalReasons*: seq[string]
    requiredDispatchPath*: seq[string]

  RouteReceipt* = object
    taskId*: string
    taskClass*: string
    routeTaskClass*: string
    generatedAt*: string
    selectedAgentBackend*: string
    riskClass*: string
    writeScope*: string
    verificationGate*: string
    dispatchPolicy*: DispatchPolicy
    analysisPlan*: JsonNode
    verificationPlan*: JsonNode

# ─────────────────────────── Context Governance ───────────────────────────

type
  ContextSource* = object
    sourceClass*: string
    path*: string
    freshness*: string
    provenance*: string
    roleScope*: string
    notes*: string

  ContextGovernanceEntry* = object
    ts*: string
    taskId*: string
    phase*: string
    sources*: seq[ContextSource]
    notes*: string

# ─────────────────────────── Framework Memory ───────────────────────────

type
  FrameworkMemoryKind* = enum
    fmkLesson = "lesson"
    fmkCorrection = "correction"
    fmkAnomaly = "anomaly"

  FrameworkMemoryEntry* = object
    ts*: string
    kind*: string
    summary*: string
    sourceTask*: string
    details*: JsonNode

# ─────────────────────────── Worker Packet ───────────────────────────

type
  WorkerOutput* = object
    status*: string
    questionAnswered*: string
    answer*: string
    evidenceRefs*: seq[string]
    changedFiles*: seq[string]
    verificationCommands*: seq[string]
    verificationResults*: seq[string]
    mergeReady*: string
    blockers*: seq[string]
    notes*: string
    recommendedNextAction*: string
    impactAnalysis*: Option[JsonNode]

# ─────────────────────────── Helpers ───────────────────────────

const
  DefaultNodes* = ["analysis", "writer", "coach", "problem_party",
                    "verifier", "approval", "synthesis"]

  WriteProducingTaskClasses* = [
    "small_patch", "small_patch_write", "ui_patch", "implementation"
  ]

  DomainTagAliases* = {
    "odoo_api": "api_contract",
    "flutter_ui": "frontend_ui",
    "riverpod_state": "state_management",
  }.toTable

func costClassForUnits*(units: int): CostClass =
  if units <= 0: ccFree
  elif units <= 2: ccCheap
  elif units <= 6: ccPaid
  else: ccExpensive

func inferredRiskClass*(taskClass, writeScope, verificationGate: string): RiskClass =
  if writeScope in ["orchestrator_native", "external_write", "repo_write"]:
    rcR3
  elif writeScope in ["scoped_only", "sandbox", "patch"]:
    rcR2
  elif verificationGate in ["architectural_review", "targeted_verification"]:
    rcR1
  elif taskClass == "architecture":
    rcR1
  else:
    rcR0

func targetReviewState*(riskClass: RiskClass): string =
  case riskClass
  of rcR0: "review_passed"
  of rcR1: "policy_gate_required"
  of rcR2: "senior_review_required"
  of rcR3: "human_gate_required"

func analysisRequired*(taskClass, writeScope: string): bool =
  taskClass in WriteProducingTaskClasses or writeScope != "none"
