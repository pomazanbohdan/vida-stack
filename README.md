<div align="center">
  <h1>🌌 Vida Stack</h1>
  <p><b>The active development repository for <code>VIDA 0.2.0</code> and the reference architecture for <code>VIDA 1.0</code>.</b></p>
  
  <p>
    <a href="#"><img src="https://img.shields.io/badge/Status-Active_Development-brightgreen" alt="Status"></a>
    <a href="#"><img src="https://img.shields.io/badge/Phase-0.2.0_Proving-blue" alt="Phase"></a>
    <a href="#"><img src="https://img.shields.io/badge/Target-VIDA_1.0-purple" alt="Target"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-Proprietary-red" alt="License"></a>
  </p>
</div>

> [!IMPORTANT]
> **Transitional Architecture Notice:** Vida Stack is actively evolving. `taskflow-v0` and `codex-v0` are the current implementation substrates, but they do not alone define product authority. The canonical specification remains centralized in `docs/product/spec/` and `vida/config/`.

---

## 🎯 Purpose

**Vida Stack** exists to build a real control plane for agent-driven product engineering, moving beyond a loose collection of prompts, scripts, or ad hoc helpers.

The target system unifies:
- ⚡ **Task Execution**
- 🔀 **Routing & Orchestration**
- ✅ **Verification & Approval**
- 🧠 **Memory & Durable State**
- 📜 **Instruction & Command Law**
- 📚 **Canonical Documentation & Inventory**
- 🔄 **Migration & Compatibility Control**

---

## ✨ Core Features & Value Proposition

- **Documentation-First Governance:** Code changes are illegal until the markdown specification and configuration laws are updated and approved.
- **Strict Bootstrapping & Routing:** Agents navigate via predefined lanes and hard invariants (`AGENTS.md`).
- **Agentic "Spec-to-Law" Pipeline:** Automates the transition from human-readable text to executable framework logic.
- **Unified Layer Validation:** Multi-level layer checks ensure code, document sidecars, and configuration never drift.

---

## 🚀 Quick Start / Operations Preview

Development is governed by the `codex-v0` documentation engine. A typical workflow involves checking document health before execution:

```bash
# Get a one-command documentation state overview
python3 codex-v0/codex.py overview

# Run stronger consistency checks before committing
python3 codex-v0/codex.py doctor --root . --show-warnings
```

---

## 🏗️ Current Transitional Architecture

The repository currently has two explicit transitional implementation lines, which are active and useful now. However, neither defines the final authority of `VIDA 1.0` on its own. Product authority remains governed by `docs/product/spec/**`, `vida/config/**`, and the active instruction canon in `vida/config/instructions/**`.

### ⚙️ `taskflow-v0/`
The current `0.2.0` runtime proving ground. 

**Validates and hardens:**
- Task state & execution flow
- Routing & route-law enforcement
- Worker/orchestrator cooperation
- Verification & approval gates
- Runtime memory & state artifacts
- The behavioral spine that `VIDA 1.0` must preserve

### 📖 `codex-v0/`
The current `0.2.0` information-system proving ground.

**Validates and hardens:**
- Canonical document metadata
- Sidecar lineage & changelogs
- Inventory & registry generation
- Validation & consistency gates
- Lawful documentation mutation
- Dependency & impact analysis
- Documentation-first layer development

---

## 🧠 Foundations & Research Axis

Vida Stack is not merely a collection of scripts; it is heavily grounded in external patterns and synthesized AI research. Its functional spine relies on:

1. **Strict "Spec-to-Law" Translation:**
   - Product and system models are defined in `docs/product/spec/`.
   - These are translated into executable laws, configurations, and templates in `vida/config/`.

2. **Absorbed External Architecture Patterns:**
   - **Event-Sourced State & Graph Checkpointing:** For checkpoint, commit, and replay lineage schemas.
   - **Distributed Gateway & Resumable Workflows:** For trigger/bookmark semantics, gateway resumes, and verification merge laws.
   - **Formal State Machines:** For strict machine definition and validation laws.

3. **Agentic Research Synthesis:**
   - Incorporates and normalizes guidance from **frontier AI research labs** and **industry-standard security frameworks**.
   - Grounded in rigorous **Threat Models** and **Anti-Pattern Catalogs**.
   - Applies strict task slicing, cheap-worker packet mechanisms, and consensus/escalation matrices defined via `docs/framework/research/`.

---

## 🛤️ VIDA 1.0 Direction

`VIDA 1.0` is the target durable local binary line. The canonical vector is to separate `taskflow` and `codex` into bounded crates that work independently (as libraries and CLI tools), while the top-level `vida` binary composes them.

- 🧩 **`taskflow`** — Owns runtime/task execution behavior.
- 🧩 **`codex`** — Owns canonical documentation, instruction, and inventory behavior.
- 🌌 **`vida`** — Composes these capabilities into a unified product operator surface.

---

## ⚖️ Current Working Rules

We strictly follow a **documentation-first** rule:

1. 📝 **Spec first:** When a new layer or rule is introduced, canonical documentation is brought into shape first.
2. 🛠️ **Implement second:** Only after the spec is defined may the implementation be changed.
3. 👑 **Spec wins:** If implementation and spec diverge, the spec wins and the implementation must be corrected.

> **Incremental Closure:** Each completed layer must provide standalone value. Future-layer assumptions must never justify current-layer behavior.

---

## 🗺️ Current Maps & Navigation

Primary orientation surfaces to guide you through the framework:

- 🧭 [**Bootstrap Router** (AGENTS.md)](/home/unnamed/project/vida-stack/AGENTS.md)
- 📍 [**Project Context & Notes** (AGENTS.sidecar.md)](/home/unnamed/project/vida-stack/AGENTS.sidecar.md)
- 🗺️ [**Framework Map Protocol**](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.framework-map-protocol.md)
- 📑 [**Protocol Index**](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.protocol-index.md)
- 🗺️ [**Current Spec Map**](/home/unnamed/project/vida-stack/docs/product/spec/current-spec-map.md)
- 📚 [**Canonical Docs & Inventory Layers**](/home/unnamed/project/vida-stack/docs/product/spec/canonical-documentation-and-inventory-layers.md)

---

## 🤝 Contributing & Governance

Contributions to Vida Stack strictly adhere to its documentation-first policy. Unsolicited code PRs will be rejected. Propose changes to the specifications (`docs/product/spec/`) first.

For detailed rules and project governance, read [CONTRIBUTING.md](/home/unnamed/project/vida-stack/CONTRIBUTING.md).

---

## 📌 Version Path & Licensing

- 🛤️ **Version Path:** Defined in [VERSION-PLAN.md](/home/unnamed/project/vida-stack/VERSION-PLAN.md).
- 📄 **License:** Core licensing under [LICENSE](/home/unnamed/project/vida-stack/LICENSE).

-----
artifact_path: project/repository/readme
artifact_type: repository_doc
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: README.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-10T03:39:28+02:00'
changelog_ref: README.changelog.jsonl
