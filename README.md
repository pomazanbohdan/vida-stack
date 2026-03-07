# ⚡ Vida Stack

[![Status: Active Development](https://img.shields.io/badge/Status-Active_Development-blue.svg)](#current-stage)
[![Paradigm: Agentic Orchestration](https://img.shields.io/badge/Paradigm-Agentic_Orchestration-orange.svg)](#architecture-baseline)
[![Future: Rust](https://img.shields.io/badge/Future-Rust-red.svg)](#phase-5-rust-reimplementation)

> **Vida Stack** is an agentic engineering framework for building a highly autonomous product-development orchestrator. Developed with the help of OpenAI Codex, it serves as the active implementation and orchestration environment for the framework's real-world evolution.

Its purpose is not to be *another* task tracker or prompt collection. The goal is to evolve a real **control plane for agent-driven product engineering**: planning, execution, verification, documentation sync, telemetry, learning loops, and multi-agent orchestration working as one coherent system.

---

## ❓ Why This Exists

Modern AI-assisted development still breaks in predictable ways. Vida Stack exists to solve these problems by shifting the paradigm from ad-hoc prompting to a robust control plane.

### ❌ The Problem

- Workflows drift away from source-of-truth state.
- Prompts and protocols diverge from runtime behavior.
- Review and verification stay optional instead of enforced.
- Parallel agents create noise, conflicts, and duplicated work.
- Context cost grows faster than execution quality.

### ✅ The Vida Stack Solution

A framework optimized for real product delivery, not demos, that is:

- **Protocol-driven** and **Verification-first**
- **Orchestration-native** and **Telemetry-aware**

---

## 🎯 Project Goal

The long-term target is an optimized agentic product-engineering system with a clear control-plane architecture for autonomous product delivery.

That target shape includes:

- a real agent control plane
- durable workflow state
- explicit verification and review gates
- structured subagent orchestration
- memory and learning loops
- documentation synchronization
- telemetry, scorecards, and drift detection
- efficient context handling with lower token burn

In simpler terms: Vida Stack is being evolved toward a super-autonomous orchestrator for product development, continuously updated against real AI and software-engineering practice.

---

## ✅ What It Can Do Today

Vida Stack is still in active development, but it can already be used as a working framework layer for:

- automatic work with tasks, epics, queues, and execution state
- request-intent classification into answer, artifact, execution, and mixed flows before heavy runtime engagement
- compact boot snapshots and low-cost status diagnostics before broader task or log discovery
- queue-backed single-writer task-state mutations for subagent-heavy execution
- orchestration of bounded subagents with routing, fallback, lease-aware dispatch, and review-aware control
- subagent-first development execution in supported active modes with orchestrator-owned synthesis
- hard-law route enforcement with fail-closed dispatch when mandatory fanout, verification, or lawful escalation is skipped
- silent framework diagnosis as a background guardrail with deferred framework bug capture
- research flows with structured evidence gathering and validation
- planning and decomposition of work into executable slices
- formation and refinement of specifications and technical contracts
- implementation and development execution through protocol-governed flows, including mixed-issue split handling for issue-driven work

The system is not yet a finished standalone product, but it is already capable of real framework-driven engineering work.

---

## 🧠 Adaptive Thinking Engine

Vida Stack does not use one flat prompting style for every problem. It routes work through a structured reasoning engine that selects the right thinking algorithm based on weighted task scoring, explicit overrides, and escalation rules.

### Algorithm Selector

Every non-trivial task is scored with:

`C×3 + R×3 + S×2 + N×2 + F×1`

Where:

- **C** = Complexity
- **R** = Reversibility
- **S** = Stakes
- **N** = Novelty
- **F** = Frequency

That score determines which reasoning algorithm VIDA runs:

| Score | Algorithm | Description |
| :--- | :--- | :--- |
| `<=15` | **STC** | Stepwise Think-Critique, a lightweight step-checking mode inspired by [Chain-of-Thought Prompting](https://arxiv.org/abs/2201.11903). |
| `16–25` | **PR-CoT** | Poly-Reflective Chain-of-Thought, a structured multi-perspective validation mode related to [Self-Consistency](https://arxiv.org/abs/2203.11171). |
| `26–35` | **MAR** | Multi-Agent Reflexion, a multi-round refinement flow inspired by [Reflexion](https://arxiv.org/abs/2303.11366). |
| `36–45` | **5-SOL** | A 5-option, 2-round synthesis method. This is the author's own algorithm inside VIDA. |
| `>45` | **META** | Full ensemble mode: a combination of all reasoning algorithms in VIDA, specifically **PR-CoT + MAR + 5-SOL**, with weighted confidence and rerun logic. |

### Built-In Overrides

Some problem classes bypass normal scoring and force stronger reasoning:

- Security and authentication decisions -> `META`
- Database and foundation architecture -> `META`
- DEC creation -> `MAR`
- Multiple issues or competing choices -> `5-SOL`

### Why This Matters

This makes reasoning a first-class runtime capability rather than a hidden prompt style:

- simpler tasks use faster low-overhead reasoning
- architectural and ambiguous tasks use deeper multi-pass analysis
- critical decisions use ensemble validation instead of one-shot answers
- bug investigation can route into a dedicated root-cause pipeline before any fix is attempted

In practice, Vida Stack does not just orchestrate agents. It also decides **how the system should think** before it acts.

---

## 🔁 Framework Self-Improvement Loop

Vida Stack is designed to improve not only the project it is working on, but also its own framework behavior. Self-improvement is treated as a runtime capability with explicit protocols, ownership boundaries, and telemetry-backed feedback loops.

### What This Includes

- **framework self-analysis** for detecting protocol friction, instruction conflicts, token overhead, and runtime ergonomics gaps
- **self-reflection checkpoints** during execution to keep decisions evidence-based and reduce drift
- **reflection-pack reconciliation** for synchronizing decisions, docs, and runtime contracts after changes
- **subagent scorecards** that track provider quality, useful progress, merge-readiness, fallback dependence, and failure patterns
- **evaluation loops** that turn runtime telemetry into routing and orchestration improvements

### Why This Matters

Most agent systems focus only on solving the current task. Vida Stack also asks whether the framework itself is creating unnecessary rereads, weak routing decisions, protocol drift, or execution friction.

That means the system can:

- inspect and improve its own orchestration layer
- separate framework problems from project-specific problems
- adapt provider strategy based on observed performance
- keep learning loops tied to measurable runtime evidence instead of intuition

In practice, this gives VIDA a built-in improvement path for its own protocols, not just for the code it helps produce.

---

## 🎯 Target System Shape

The target architecture is organized around a small set of core subsystems working as one control plane rather than as disconnected scripts, prompts, and docs.

| Subsystem | Core Responsibilities |
| :--- | :--- |
| 🎛️ **`VS-Control`** | Orchestration, decomposition, routing, and escalation. |
| 💾 **`VS-State`** | Authoritative workflow state, execution history, capsules, and health. |
| 🧠 **`VS-Memory`** | Durable operational memory and distilled lessons. |
| 🛡️ **`VS-Verify`** | Review, policy, test, and approval gates. |
| 📊 **`VS-Observe`** | Telemetry, scorecards, and drift visibility. |
| 🎓 **`VS-Learn`** | Reflection, evaluation, and improvement loops. |
| 🔄 **`VS-DocSync`** | Documentation actualization and canonical-document promotion. |

---

<a id="current-stage"></a>
## 🚧 Current Stage

Vida Stack is currently being hardened inside a real production-like project.

This repository is not a toy example and not a detached greenfield framework experiment. The current phase is intentional:

- run the framework on real engineering work
- validate algorithms under real pressure
- find protocol/runtime mismatches
- refine orchestration, review, and fallback mechanics
- prove which ideas survive contact with real delivery constraints

Right now, the framework is implemented through a practical stack of:

- `AGENTS.md` as a bootstrap router for orchestrator versus worker lane entry
- `_vida/docs/ORCHESTRATOR-ENTRY.MD` and `_vida/docs/SUBAGENT-ENTRY.MD` as split lane contracts
- `_vida/docs/*` for canonical protocols
- `_vida/scripts/*` for runtime helpers and enforcement
- `br` plus TODO telemetry for execution state
- external-first subagent routing with verification, route receipts, and fallback logic

Recent runtime work has also hardened the framework with a bootstrap split between orchestrator and worker lanes, compact boot snapshots for dev-oriented context, request-intent gating before `br` and pack machinery, subagent-first analysis/review and development execution in supported modes, budget-aware route and escalation metadata, question-driven worker packets with stricter return contracts and non-`STC` impact tails, hard bounded log-read rules for `.vida/*` surfaces, target review-state visibility before dispatch, ensemble lease diagnostics with conflict history, canonical `route_law_summary` and machine-readable `route_receipt` artifacts, verifier-gated `decision_ready` versus `synthesis_ready` completion, pool dependency-graph analysis before multi-task writer selection, and orchestrator-synthesized user reporting that keeps subagent process details hidden by default.

The newest runtime layer also adds queue-backed single-writer task-state mutations for concurrent subagent flows, silent framework diagnosis as a background capture mode, reusable proving-pack templates for product and framework regression surfaces, reusable leased subagent-pool helpers, fail-closed cheap-lane rejection for low-signal outputs, and mixed-issue split artifacts that preserve unresolved secondary symptoms as follow-up work instead of widening the current writer lane.

This phase matters because the objective is to finish the mechanics end-to-end before extracting and replatforming the system.

Recent framework-level changes are tracked in [_vida/CHANGELOG.md](_vida/CHANGELOG.md).

---

## 🧱 Final System Shape

Vida Stack is not intended to remain a shell-and-markdown framework forever.

The current repository is the proving ground where protocols, orchestration rules, task lifecycles, verification behavior, and subagent coordination are validated under real working conditions.

The final target is a **Rust-based control plane** with **SurrealDB-backed state and memory**, where the current script runtime is replaced by a real daemonized orchestration kernel.

### What Changes In The Final System

The current framework relies on:

- shell and Python runtime helpers
- markdown protocol surfaces
- file-based artifacts, logs, and snapshots
- command-driven orchestration glue

The final system is intended to replace that with:

- a Rust daemon as the orchestration kernel
- SurrealDB as the operational state and memory backend
- compact typed control-plane commands instead of long script chains
- memory-backed framework and project knowledge instead of `/docs/*` as the primary runtime source
- structured agent interaction optimized for lower-friction execution

### Command Model

In the final system, agents and operators should work through optimized runtime commands rather than multi-step script choreography.

For example, a command such as:

```bash
vs-task next
```

should be enough to:

- close the current task step if it is valid to close
- inspect blockers, dependencies, and runtime state
- open the next eligible task automatically
- return a compact structured report explaining what was taken into progress, or why nothing could advance

The goal is to reduce orchestration friction while keeping runtime state explicit and machine-verifiable.

### Structured Interaction Format

The final system is also expected to use a compact structured interaction format for agent/runtime exchange.

A strong candidate for this is [TOON](https://github.com/toon-format/toon), which is designed as a compact, schema-aware alternative to verbose JSON for LLM-facing workflows.

That would make command outputs, handoff packets, runtime summaries, and status views more compact and better suited for agent interaction.

### Memory-Backed Framework Runtime

The final system is not intended to keep framework knowledge primarily in repository docs.

Instead, framework rules, project documentation, protocol contracts, and operational memory are expected to move into a memory-backed system similar to [memory-mcp-1file](https://github.com/pomazanbohdan/memory-mcp-1file), which already combines semantic memory, graph memory, code indexing, and a SurrealDB backend.

That means the long-term runtime direction is:

- documentation stored as memory records rather than scattered markdown files
- framework instructions retrieved from memory layers instead of `/docs/*`
- project context, contracts, and operational knowledge queried through runtime memory tools
- a much smaller repository surface for the operator

### Minimal Repo Surface

This transition has already started.

Today, `AGENTS.md` is no longer the full monolithic framework contract. It now acts as a bootstrap router that selects the orchestrator or worker entry path, while the larger lane-specific contracts live in `_vida/docs/ORCHESTRATOR-ENTRY.MD`, `_vida/docs/SUBAGENT-ENTRY.MD`, and `_vida/docs/SUBAGENT-THINKING.MD`.

The final architecture pushes this further by replacing even that bootstrap-heavy repository model with a runtime-loaded session contract.

The repository should expose only a minimal bootstrap instruction in `AGENTS.md`, for example:

```bash
vs-session start
```

From that single entrypoint, the control plane should initialize the full working session automatically:

- load the active framework identity, orchestrator contract, and runtime invariants
- hydrate the current session, task state, and execution position
- attach project-specific memory context
- load framework protocols, overlays, and command contracts from memory-backed storage
- detect available subagents, routing state, recovery state, and health state
- resolve the current execution surface, available commands, and next valid control-plane actions
- present the next valid orchestration path in an optimized form

That means the repository is no longer the primary container of framework instructions or startup logic.

Instead:

- `AGENTS.md` stays a thin bootstrap entrypoint
- framework rules move into runtime-managed memory
- framework protocols stop living primarily as repo-loaded startup instructions
- project documentation moves into project memory layers
- session boot, protocol activation, and context hydration become runtime responsibilities
- operator interaction becomes command-first rather than document-first

The result is a much smaller repository surface and a much more optimized development start flow.

Instead of manually reconstructing framework state from a monolithic bootloader and many framework docs, the user or agent starts a session once, and the system restores the correct working context automatically.

In that final shape, the repository is only the bootstrap edge. The real framework lives in the control plane, memory layer, and persistent runtime state.

---

## ⚙️ Subagent Modes

Vida Stack currently separates subagent operation into a small set of explicit modes.

### System Modes

- `native` — use internal subagents only
- `hybrid` — use internal and external providers together under routing policy
- `disabled` — do not use the subagent system

### Execution Modes

- `fanout` — parallel external-first read-only execution
- `fallback` — deterministic fallback chain when fanout results are insufficient
- `arbitration` — bounded tie-break lane for unresolved decision-relevant conflicts

### Worker Thinking Modes

- `STC` — default mode for direct scoped analysis and small isolated work
- `PR-CoT` — bounded comparison and trade-off reasoning inside a narrow scope
- `MAR` — structured root-cause and multi-pass analysis inside worker scope

Workers do not self-upgrade into `META`; full orchestrator-level reasoning remains outside the default worker lane.

---

<a id="architecture-baseline"></a>
## 🏗️ Architecture Baseline

Vida Stack follows a modern agent-platform architecture for production-grade autonomous engineering systems.

### The Agent Control Loop

Agents operate in a continuous loop of:
`Observation` -> `Planning` -> `Action` -> `Verification` -> `Reflection` -> `Improvement`

### Core Runtime Model

1. **Goal Interpreter:** Turns user intent into executable work.
2. **Planner:** Decomposes and routes tasks.
3. **Control Loop:** Adapts after each observation.
4. **Tool Router:** Validates and dispatches actions.
5. **Execution Environment:** Terminal, filesystem, browser, code runtime, external APIs.
6. **Observation Layer:** Normalizes results.
7. **Memory Layer:** Preserves useful operational knowledge.
8. **Telemetry & Evaluation:** Drives continuous improvement.

The longer-term runtime direction also includes:

1. an event-oriented workflow kernel with deterministic recovery after interruption or compaction
2. clear separation between workflow state, memory, documentation state, and telemetry
3. protocol rules that exist as docs, machine-readable policy artifacts, and runtime enforcement
4. compact context packets that reduce repeated rereads of large markdown surfaces

### Multi-Agent Model

Developed toward a role-based architecture to allow bounded decomposition and cleaner ownership:
*`Planner`* · *`Researcher`* · *`Executor`* · *`Critic/Reviewer`* · *`Integrator`* · *`Supervisor`*

This model assumes explicit leases and ownership:

1. task or block ownership per active agent run
2. optional file or worktree scope where mutation is involved
3. release or expiration rules for parallel work

That ownership model is meant to reduce duplicate work, write conflicts, and noisy integrations.

---

## 💡 Capabilities Deep-Dive

<details>
<summary><b>🧠 Planning & Memory Model</b></summary>
<br>

**Planning:** Supports structured methods over free-form guessing, including reasoning-plus-acting flows, plan-and-execute flows, branching reasoning, and graph-like workflow execution.<br>
**Memory:** Goes beyond prompt context. Includes short-term, episodic, semantic, procedural, decision, failure, and reflection memory.
</details>

<details>
<summary><b>🛡️ Telemetry, Evaluation & Safety</b></summary>
<br>

**Telemetry:** Tracks task success rate, tool success rate, latency, cost per task, human intervention rate, reasoning proxies, verification pass rate, and scorecards.<br>
**Governance:** Uses policy-controlled tool access, risk-based approval gates, human checkpoints, explicit review surfaces, and drift detection.
</details>

<details>
<summary><b>📉 Cost & Efficiency Strategy</b></summary>
<br>

Optimized for real engineering throughput through model routing by task type, prompt and artifact reuse, context pruning, compact hydration, and external-first cheap read-only fanout. The goal is lower token burn, less protocol/runtime drift, and stronger machine-checkable runtime artifacts over time.

This now also includes more explicit agent-system configuration surfaces such as fanout metadata, runtime budget fields, and provider-level dispatch environment settings.
</details>

<details>
<summary><b>📚 Compiled Policy Direction</b></summary>
<br>

Over time, stable rules should evolve into lighter runtime artifacts such as boot and execution policy packets, required evidence schemas, and compact handoff or hydration payloads. This is intended to reduce markdown-only enforcement and make verification easier to automate.
</details>

---

## 🚀 Development Philosophy

Vida Stack is being developed with a few non-negotiable principles:

1. **Real-project validation** before platform extraction.
2. **Root-cause fixes** over cosmetic automation.
3. **Single authoritative execution state.**
4. **Verification as runtime behavior**, not team culture.
5. **Legacy-zero evolution**: no parallel "old and new" truths.
6. **Lean by default**, richer orchestration only when justified.
7. **External trends** in AI engineering should inform the roadmap, but only after they prove operational value.

---

## 🗺️ Roadmap

The roadmap is intentionally staged to evolve from a working concept to a high-performance system.

- [x] **Phase 1: Real-Project Runtime Hardening**
  Hardening inside a real production-like project. Validating algorithms, finding protocol mismatches, and refining fallback mechanics.
- [ ] **Phase 2: Framework Extraction**
  Separating framework from project-specific concerns, tightening machine-enforced contracts, and preparing a clean public surface.
- [ ] **Phase 3: Daemonized Control Plane**
  Background orchestration, reactive health monitoring, richer doc-sync workers, and stronger event-driven runtime behavior.
- [ ] **Phase 4: Full Control Plane**
  Durable workflow kernel, richer verification fabric, deeper telemetry and learning loops, and stronger ownership models for parallel agents.
- [ ] **Phase 5: Rust Reimplementation**
  The planned endgame. A full system in Rust for stronger runtime integrity, safer concurrency, and longer-running daemonized orchestration.

<a id="phase-5-rust-reimplementation"></a>
### Phase 5: Rust Reimplementation

The current shell, Python, and docs runtime is not wasted work. It is the proving ground that defines what the Rust system should actually implement.

Why Rust:

- stronger runtime integrity
- better performance for long-running orchestration services
- safer concurrency for multi-agent and event-driven execution
- more robust foundation for longer-running daemonized orchestration

Why SurrealDB:

- one operational state backend for tasks, blocks, runs, leases, reviews, scorecards, and memory
- queryable graph-friendly runtime model instead of scattered file artifacts
- stronger recovery, telemetry, and orchestration-state persistence

What changes compared with the current runtime:

- today's system proves the protocol and orchestration logic through scripts, docs, logs, and file artifacts
- the final system should own that logic as a typed runtime kernel
- current markdown-heavy framework knowledge should move into memory-backed runtime surfaces
- current script chains should compress into optimized control-plane commands such as `vs-session start` and `vs-task next`

---

## 📦 Installation

The current installer is bash-only and is intended to install the framework payload into an existing repository.

Quick install:

```bash
curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- init
```

Other common commands:

```bash
curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- doctor
curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- upgrade
curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- upgrade --dry-run
```

Installer behavior:

- installs only framework payload files
- supports `init`, `upgrade`, and `doctor`
- supports `--dry-run`, `--force`, `--dir`, and `--version`
- `init` stops if `AGENTS.md` or `_vida/` already exists unless explicitly forced
- `upgrade` replaces existing framework payload and writes backups into `.vida-backups/<version>/`
- installs from GitHub Release archives and supports explicit release tags through `--version`

Release archives are framework-only and exclude repository-level docs, installer sources, and changelog payload files.

---

## 🧩 External CLI Providers

In hybrid development, Vida Stack can work with external CLI coding agents as read-heavy or review-oriented providers alongside the internal orchestrator lane.

The framework is designed to work well with tools that either expose free usage paths directly or provide low-cost external-first access for bounded subagent work.

| Provider | Role In Hybrid Flow | Free Access Note | Install |
| :--- | :--- | :--- | :--- |
| **Qwen Code** | External CLI provider for research, analysis, and read-only fanout | Official Qwen Code docs state that Qwen OAuth provides a free tier with **2,000 free requests/day** | `npm install -g @qwen-code/qwen-code@latest` or `brew install qwen-code` |
| **Kilo Code CLI** | External CLI provider for terminal-first coding and parallel-agent workflows | Kilo documents a free account flow, and official Kilo announcements describe selected models as **free for a limited time** | `npm install -g @kilocode/cli` |
| **OpenCode** | External CLI provider for multi-provider coding, review, and automation workflows | OpenCode states that **free models are included**, and Zen docs list several models available **free for a limited time** | `curl -fsSL https://opencode.ai/install \| bash` or `npm install -g opencode-ai` |
| **Gemini CLI** | External CLI provider for terminal-first coding, large-context analysis, and hybrid read-heavy workflows | Official Gemini CLI pricing docs state that Google login provides a free tier with **1,000 model requests/day** and **60 requests/minute** | `npm install -g @google/gemini-cli`, `brew install gemini-cli`, or `npx @google/gemini-cli` |
| **Mistral Vibe** | External CLI provider for terminal-based coding and prompt-driven patching | In the installation sources used here, a built-in free tier is **not explicitly documented**; typically used with Mistral or provider credentials | `curl -LsSf https://mistral.ai/vibe/install.sh \| bash` or `uv tool install mistral-vibe` |

Source references:

- [Qwen Code](https://github.com/QwenLM/qwen-code)
- [Kilo CLI](https://kilo.ai/docs/cli)
- [Kilo free model note](https://blog.kilocode.ai/p/kilo-code-minimax-m2-free-access)
- [OpenCode](https://opencode.ai/)
- [OpenCode Zen pricing](https://opencode.ai/docs/zen)
- [Gemini CLI](https://geminicli.com/docs/get-started/)
- [Gemini CLI quota and pricing](https://geminicli.com/docs/quota-and-pricing/)
- [Mistral Vibe](https://docs.mistral.ai/mistral-vibe/introduction/install)

---

## 📂 Repository Structure

Current repository layout:

```text
.
├── AGENTS.md
├── README.md
├── RELEASE-1-IMPLEMENTATION-ROADMAP.md
├── RELEASE-1-SCOPE.md
└── _vida/
    ├── commands/
    ├── commands.md
    ├── constitution.md
    ├── constraints.md
    ├── docs/
    ├── dual-model-protocol.md
    ├── owasp-integration.md
    ├── planning.md
    ├── scripts/
    ├── templates/
    ├── transitions.md
    └── workflow.md
```

Key runtime areas:

- [AGENTS.md](AGENTS.md)
- [README.md](README.md)
- [RELEASE-1-SCOPE.md](RELEASE-1-SCOPE.md)
- [RELEASE-1-IMPLEMENTATION-ROADMAP.md](RELEASE-1-IMPLEMENTATION-ROADMAP.md)
- [_vida/commands](_vida/commands)
- [_vida/commands.md](_vida/commands.md)
- [_vida/docs/protocol-index.md](_vida/docs/protocol-index.md)
- [_vida/docs/SUBAGENT-ENTRY.MD](_vida/docs/SUBAGENT-ENTRY.MD)
- [_vida/docs](_vida/docs)
- [_vida/scripts](_vida/scripts)
- [_vida/templates](_vida/templates)
- [_vida/CHANGELOG.md](_vida/CHANGELOG.md)
- [install/install.sh](install/install.sh)

---

## 🌍 Open-Source Direction

Vida Stack is intended to become its own standalone repository.

The reason is straightforward:

- the framework should be developed independently from any one product codebase
- contributors should be able to improve orchestration, verification, telemetry, and runtime design directly
- knowledge should not stay trapped in one private implementation context
- the framework needs a clean public surface for collaboration, experimentation, and transfer of engineering patterns

The future standalone repository should allow people to contribute in areas such as:

- runtime architecture
- orchestration algorithms
- verification and policy systems
- telemetry and scorecards
- memory and learning loops
- protocol compiler and runtime artifact generation
- Rust control-plane implementation

---

## 🔭 Current Reality vs Future Vision

### Current Reality

- Vida Stack is an actively used framework layer inside a real project.
- Many core mechanics already exist and are exercised daily.
- Some parts are still markdown-heavy and script-heavy by design.
- Several advanced control-plane ideas are still being validated, not productized.

### Future Vision

- a polished standalone framework
- stronger machine-enforced runtime contracts
- durable orchestration and verification subsystems
- a Rust-based implementation for the full system
- a contributor-friendly open-source control plane for agentic product engineering

---

## ⭐ North Star

Build a complete, high-integrity, highly autonomous product-development orchestrator that can evolve with the state of AI while remaining grounded in real engineering work.
