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

This phase matters because the objective is to finish the mechanics end-to-end before extracting and replatforming the system.

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

## 📂 Repository Structure

Right now, the framework is implemented through a practical stack:

```text
.
├── AGENTS.md
├── README.md
├── RELEASE-1-SCOPE.md
├── RELEASE-1-IMPLEMENTATION-ROADMAP.md
└── _vida/
    ├── docs/
    │   └── protocol-index.md
    └── scripts/
```

Key runtime areas:

- [AGENTS.md](AGENTS.md)
- [_vida/docs/protocol-index.md](_vida/docs/protocol-index.md)
- [_vida/docs](_vida/docs)
- [_vida/scripts](_vida/scripts)
- [RELEASE-1-SCOPE.md](RELEASE-1-SCOPE.md)
- [RELEASE-1-IMPLEMENTATION-ROADMAP.md](RELEASE-1-IMPLEMENTATION-ROADMAP.md)

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
