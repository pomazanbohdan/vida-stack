<div align="center">
  <h1>🌌 Vida Stack</h1>
  <p><b>The active repository for <code>VIDA 0.2.0</code>: a documentation-first control plane for agent-driven product engineering.</b></p>
  
  <p>
    <a href="#"><img src="https://img.shields.io/badge/Status-Active_Development-brightgreen" alt="Status"></a>
    <a href="#"><img src="https://img.shields.io/badge/Release-0.2.0-blue" alt="Release"></a>
    <a href="#"><img src="https://img.shields.io/badge/Runtime-taskflow--v0-orange" alt="Runtime"></a>
    <a href="#"><img src="https://img.shields.io/badge/Docsys-codex--v0-teal" alt="Docsys"></a>
    <a href="#"><img src="https://img.shields.io/badge/Target-VIDA_1.0-purple" alt="Target"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-Proprietary-red" alt="License"></a>
  </p>
</div>

> [!IMPORTANT]
> **Transitional Architecture Notice:** `taskflow-v0` and `codex-v0` are the active `0.2.0` runtime substrates, but product authority still lives in canonical maps, specs, and framework law under `docs/product/spec/` and `vida/config/`.

---

## ✨ What Is VIDA?

**Vida Stack** is building a real control plane for agent-driven product engineering.

Instead of treating prompts, scripts, task lists, and docs as disconnected artifacts, VIDA unifies them into one lawful operating model:

- ⚙️ **Task execution** through `taskflow-v0`
- 📚 **Canonical documentation and inventory** through `codex-v0`
- 🧭 **Boot, routing, and map-driven discovery** through `AGENTS.md`, `AGENTS.sidecar.md`, and framework maps
- ✅ **Verification, approval, and proof gates**
- 🧠 **Durable runtime state, receipts, and checkpoints**
- 🔄 **Migration, compatibility, and release discipline**

---

## 🚀 Install

### One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- install
```

### What the installer does

- 📦 downloads the tagged release archive
- 🔐 verifies release checksums
- 🗂️ installs versioned sources under `~/.local/share/vida-stack/releases/<tag>`
- 🔁 updates `~/.local/share/vida-stack/current`
- 🧪 creates an installer-managed Python `venv` for `codex-v0` and `pyturso`
- 🧰 writes launchers into `~/.local/bin`:
  - `vida`
  - `taskflow-v0`
  - `codex-v0`
- 🐚 wires `VIDA_HOME`, `VIDA_ROOT`, and `PATH` into `bash` / `zsh`

### Upgrade / doctor

```bash
vida upgrade --version v0.2.0
vida doctor
vida use --version v0.2.0
```

---

## 🧩 Main Tools

### ⚙️ `taskflow-v0`

The current runtime substrate for tracked execution.

It already covers:

- route- and gate-aware execution
- role selection and conversational modes
- checkpoint / replay / recovery behavior
- verification merge and admissibility
- DB-backed task store with JSONL import
- final `taskflow -> codex` runtime-consumption wiring

### 📚 `codex-v0`

The current canonical documentation and inventory engine.

It already covers:

- metadata and changelog normalization
- protocol and activation coverage checks
- readiness and proof checks
- canonical map health checks
- documentation-first mutation discipline

### 🌌 `vida`

The top-level product direction.

In `0.2.0`, the install surface already gives you a `vida` launcher so the release can be operated as one product while keeping bounded internal tools separate.

---

## 🏗️ Standards Already Developed

This repository is not just “some tooling”. It already contains several hardened standards and canonical maps:

- 🗺️ framework root-map architecture
- 📚 canonical documentation and inventory layer matrix
- ⚙️ canonical runtime layer matrix
- 👥 role / skill / profile / flow model
- 🤖 auto-role and conversational-mode model
- 🔁 checkpoint / recovery / resumability law
- ✅ verification-lane and merge law
- 🧭 bootstrap, governance, runtime-family, and template maps

These standards are designed so each layer is independently coherent and future layers only extend, never retroactively justify, lower ones.

---

## 🗺️ Start Here

### Bootstrap and maps

- 🧭 [Bootstrap Router](AGENTS.md)
- 📍 [Project Sidecar](AGENTS.sidecar.md)
- 🌐 [Framework Root Map](vida/root-map.md)
- 🗂️ [Project Root Map](docs/project-root-map.md)

### Canonical matrices

- ⚙️ [Canonical Runtime Layer Matrix](docs/product/spec/canonical-runtime-layer-matrix.md)
- 📚 [Canonical Documentation & Inventory Layers](docs/product/spec/canonical-documentation-and-inventory-layers.md)

### Spec navigation

- 📑 [Current Spec Map](docs/product/spec/current-spec-map.md)
- 🧱 [Runtime Surface Model](docs/product/spec/root-map-and-runtime-surface-model.md)
- 👥 [Role / Skill / Profile / Flow Model](docs/product/spec/agent-role-skill-profile-flow-model.md)
- 🧠 [Role Selection & Conversation Modes](docs/product/spec/agent-role-selection-and-conversation-mode-model.md)

---

## 🧪 Quick Commands

Typical documentation/runtime proving flow:

```bash
python3 codex-v0/codex.py overview

python3 codex-v0/codex.py readiness-check --profile active-canon

taskflow-v0 task import-jsonl .beads/issues.jsonl --json

taskflow-v0 consume final "Runtime closure proof path"
```

---

## 🧠 Architecture Direction

`VIDA 0.2.0` is the proving release.

Its job is to make the transitional product trustworthy enough that `VIDA 1.0` can be built on stable semantics instead of moving heuristics.

That means:

- `taskflow` owns runtime execution
- `codex` owns bounded documentation and inventory truth checks
- framework law stays in maps, specs, and protocols
- future `vida` composes these bounded systems into the final binary product

---

## 🤝 Contributing & Governance

VIDA is documentation-first and protocol-first.

- propose spec/law changes first
- keep framework truth in canonical maps and protocols
- do not treat implementation drift as a second valid source of truth

For detailed rules, read [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 📌 Version Path & Licensing

- 🛤️ **Version Path:** [VERSION-PLAN.md](VERSION-PLAN.md)
- 📄 **License:** [LICENSE](LICENSE)

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
