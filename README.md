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
- orchestration of bounded subagents with routing, fallback, and review-aware control
- research flows with structured evidence gathering and validation
- planning and decomposition of work into executable slices
- formation and refinement of specifications and technical contracts
- implementation and development execution through protocol-governed flows

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

- `AGENTS.md` as the bootloader and top-level contract
- `_vida/docs/*` for canonical protocols
- `_vida/scripts/*` for runtime helpers and enforcement
- `br` plus TODO telemetry for execution state
- external-first subagent routing with verification and fallback logic

Recent runtime work has also hardened the subagent layer with a dedicated worker-entry contract, a worker-lane thinking subset separated from orchestrator reasoning, stronger semantic merge behavior, and richer runtime maturity scorecards.

This phase matters because the objective is to finish the mechanics end-to-end before extracting and replatforming the system.

Recent framework-level changes are tracked in [_vida/CHANGELOG.md](_vida/CHANGELOG.md).

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
| **OpenCode** | External CLI provider for multi-provider coding, review, and automation workflows | OpenCode states that **free models are included**, and Zen docs list several models available **free for a limited time** | `curl -fsSL https://opencode.ai/install | bash` or `npm install -g opencode-ai` |
| **Gemini CLI** | External CLI provider for terminal-first coding, large-context analysis, and hybrid read-heavy workflows | Official Gemini CLI pricing docs state that Google login provides a free tier with **1,000 model requests/day** and **60 requests/minute** | `npm install -g @google/gemini-cli`, `brew install gemini-cli`, or `npx @google/gemini-cli` |
| **Mistral Vibe** | External CLI provider for terminal-based coding and prompt-driven patching | In the installation sources used here, a built-in free tier is **not explicitly documented**; typically used with Mistral or provider credentials | `curl -LsSf https://mistral.ai/vibe/install.sh | bash` or `uv tool install mistral-vibe` |

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
