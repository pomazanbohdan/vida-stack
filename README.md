<div align="center">
  <h1>🌌 Vida Stack</h1>
  <p><b>The active repository for the <code>VIDA 0.9.0</code> transition slice: self-hosting stabilization and runtime hardening before <code>1.0.0</code>.</b></p>
  
  <p>
    <a href="#"><img src="https://img.shields.io/badge/Status-Active_Development-brightgreen" alt="Status"></a>
    <a href="#"><img src="https://img.shields.io/badge/Release-0.9.0__transition-blue" alt="Release"></a>
    <a href="#"><img src="https://img.shields.io/badge/Runtime-TaskFlow-orange" alt="Runtime"></a>
    <a href="#"><img src="https://img.shields.io/badge/Docsys-DocFlow-teal" alt="Docsys"></a>
    <a href="#"><img src="https://img.shields.io/badge/Target-VIDA_1.0-purple" alt="Target"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MPL--2.0-brightgreen" alt="License"></a>
  </p>
</div>

> [!NOTE]
> **Current project capability:** the repository is now in the `0.9.0 transition` phase (not final `1.0.0` closure).
> - agent orchestration already works through the chief/root orchestrator and internal delegated lanes;
> - agent roles, project activation, and bounded team topology can already be configured;
> - framework and project specifications already govern documentation discipline and runtime routing;
> - the project backlog can already live in DB-backed runtime state instead of only in flat files;
> - `vida status` / `vida doctor` are now stable on fresh booted state after schema and lock-handling hardening (`2026-03-15`).
>
> **Current known gaps to `1.0.0`:**
> - DB-first authority for roles/skills/profiles/flows is still not fully closed (YAML projection remains active source);
> - activation/configurator lifecycle still depends on root `vida.config.yaml` bridge surfaces;
> - some proof-surface commands declared in specs are still fail-closed/unsupported in the launcher.
>
> **Internal validation status:** the current framework/spec stack has passed bounded documentation/runtime checks (`check`, `activation-check`, `protocol-coverage-check`, `proofcheck`) and post-fix CLI smoke for the installed `vida` binary.
> - consolidated audit: [docs/process/framework-three-layer-refactoring-audit.md](docs/process/framework-three-layer-refactoring-audit.md)
> - proven environment/status conditions: [docs/process/vida1-development-conditions.md](docs/process/vida1-development-conditions.md)

> [!IMPORTANT]
> **Execution-carrier model (canonical):**
> - `agent` means execution carrier (model/tier/cost/effectiveness), not runtime lane identity;
> - `role` is a separate runtime activation state (`worker`, `coach`, `verifier`, `solution_architect`, ...);
> - runtime selects by capability/admissibility first, then local score/telemetry guard, then cheapest eligible carrier.

## ✨ What Is VIDA?

**Vida Stack** is building a real control plane for agent-driven product engineering.

Instead of treating prompts, scripts, task lists, and docs as disconnected artifacts, VIDA keeps one lawful operating model with clear proof/runtime boundaries:

- ⚙️ **Task execution runtime family** through `vida taskflow`
- 📚 **Documentation/inventory runtime family** through `vida docflow`
- 🧭 **Boot, routing, and map-driven discovery** through `AGENTS.md`, `AGENTS.sidecar.md`, and framework maps
- ✅ **Verification, approval, and proof gates**
- 🧠 **Durable runtime state, receipts, and checkpoints**
- 🔄 **Migration and release discipline**

At the top level, VIDA is not designed as a `/commands`-first interaction shell.
It is designed as a trigger-driven protocol system where conversational operator intent activates the lawful runtime path, which makes it possible to drive complex development processes through bounded natural-language control instead of a rigid command-only interface.
The repository itself is also a live demonstration surface for those agentic-autonomous engineering standards rather than only a passive specification set.

The current target is one visible `VIDA` system where:

- 🧩 framework and project law stay canonical in docs and config
- 🗃️ operational truth is DB-first with synchronized filesystem projection
- 🎭 roles, skills, profiles, flows, and teams become explicit project activation state
- 📦 orchestration consumes compiled runtime bundles instead of re-reading raw canon on every step
- 🚦 planning, execution, artifacts, and approvals become bounded operator-facing surfaces

## ✨ Framework Features

### Step-Scoped Thinking Algorithms

VIDA keeps reasoning as an explicit framework surface instead of leaving it as an undocumented habit.

- `STC` — Stepwise Think-Critique for low-risk local steps
- `PR-CoT` — Poly-Reflective Validation for 5-perspective review
- `MAR` — Multi-Agent Reflexion for heavier structured refinement
- `5-SOL` — 5-Solutions comparison for competing viable directions
- `META` — block-composed meta-analysis for high-risk or protocol-heavy work
- `Error Search` — bug-first reasoning lane for regressions, incidents, and root-cause work
- canonical owner: [vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md](vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md)

### Product-Formation Standards

The current VIDA direction is grounded in orchestrator-led multi-agent product engineering patterns from official vendor specifications.

- **OpenAI Agents SDK** — manager-style orchestration with agents-as-tools, explicit handoffs, guardrails, and tracing.  
  Sources: [OpenAI Agents SDK overview](https://openai.github.io/openai-agents-python/), [OpenAI agent orchestration](https://openai.github.io/openai-agents-python/multi_agent/), [OpenAI tools](https://openai.github.io/openai-agents-python/tools/)
- **Anthropic Claude Code** — specialized subagents, hooks, and bounded delegation with explicit tool permissions.  
  Sources: [Claude Code subagents](https://docs.anthropic.com/en/docs/claude-code/sub-agents), [Claude Code hooks](https://docs.anthropic.com/en/docs/claude-code/hooks), [Claude Code settings](https://docs.anthropic.com/en/docs/claude-code/settings)
- **Microsoft Semantic Kernel / Agent Framework** — explicit orchestration patterns and coordination architecture for multi-agent runtime design.  
  Sources: [Semantic Kernel agent architecture](https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-architecture), [Semantic Kernel agent orchestration](https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-orchestration/), [Azure AI agent design patterns](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns)

### Framework Protocol Categories

| Framework protocol category | Current purpose |
|---|---|
| `Bootstrap / Entry` | start the session, select the lane, and bind the lawful boot path |
| `Thinking / Session` | keep step-scoped reasoning and cross-step continuity explicit |
| `Core Orchestration` | route work, select lanes, govern admissibility, context, and resumability |
| `Lane / Verification` | dispatch bounded work, shape handoffs, and return verification results |
| `Runtime / State Machines` | materialize execution state, route progression, approval, coach, and verification lifecycles |

> [!IMPORTANT]
> **Current Runtime Notice:** `vida taskflow` and `vida docflow` are the active runtime-family surfaces on the current line. The source of truth remains the canonical product/spec and instruction surfaces under `docs/product/spec/`, `vida/config/`, and `vida/config/instructions/`.

---

## 🚀 Install

### One-line install

Linux/macOS:

```bash
mkdir myproject
cd myproject
curl -fsSL https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.sh | bash -s -- install
vida init
```

Windows PowerShell:

```powershell
mkdir myproject
cd myproject
irm https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/install/install.ps1 -OutFile vida-install.ps1
pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 install
vida init
```

Windows PowerShell also supports Linux-style installer options:

```powershell
pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 install --force
pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 install --bins taskflow --force
pwsh -ExecutionPolicy Bypass -File .\vida-install.ps1 install --bins docflow --force
```

### What the installer does

- 📦 downloads the tagged release archive
- 🔐 verifies release checksums
- 🗂️ installs versioned sources under `~/.local/share/vida-stack/releases/<tag>` on Unix-like systems or `%LOCALAPPDATA%\vida-stack\releases\<tag>` on Windows
- 🔁 updates the active `current` release pointer
- 🧩 ships host CLI runtime templates (`.codex/`, `.qwen/`, `.kilo/`, `.opencode/`) and materializes the selected one through `vida project-activator`
- 📍 deploys a clean `AGENTS.sidecar.md` scaffold for the external project owner
- 🧱 scaffolds `vida.config.yaml` from the packaged template when the installed release root does not already have one
- 🧰 writes launchers into `~/.local/bin` on Unix-like systems or `%LOCALAPPDATA%\vida-stack\bin` on Windows:
  - `vida`
- 🐚 wires `VIDA_HOME`, `VIDA_ROOT`, and `PATH` into `bash` / `zsh` or the Windows user `PATH`

### Bootstrap the current project folder

```bash
vida init
```

This copies the current project bootstrap surfaces into the working directory:

- `AGENTS.md`
- `AGENTS.sidecar.md`
- `vida.config.yaml`
- `.vida/config/`
- `.vida/db/`
- `.vida/cache/`
- `.vida/framework/`
- `.vida/project/`
- `.vida/receipts/`
- `.vida/runtime/`
- `.vida/scratchpad/`

### Local developer binary contract

For local framework development, keep the system launcher on a release build and keep proofs/tests on the debug profile:

```bash
cargo build -p vida --release
install -m 755 target/release/vida ~/.local/bin/vida
cargo test -p vida -- --nocapture
```

This keeps the operator-facing `vida` in `~/.local/bin` aligned with the release binary while preserving faster and more inspectable local proof runs on the debug build.

### Project activation survey

```bash
vida project-activator
vida project-activator --json
```

This surfaces the bounded project-activation view for the current directory:

- project shape (`empty|partial|structured`)
- bootstrap-carrier state
- activation posture (`pending|partial|ready_enough_for_normal_work`)
- explicit blockers and next steps
- whether later restart or host-template initialization is still required

Host CLI selection and external agents:

- select host system with `vida project-activator --host-cli-system <codex|qwen|kilo|opencode> ...`
- external CLI routing and command templates are owned by `vida.config.yaml -> agent_system.subagents.*`
- when external CLI is configured, `vida status --json` exposes `host_agents.external_cli_preflight`
- if sandbox is active and network is unavailable, preflight returns `blocked` with explicit `next_actions`

### Upgrade / doctor

```bash
vida init
vida project-activator
vida upgrade --version <tag>
vida doctor
vida use --version <tag>
```

---

## 🧩 Main Tools

### ⚙️ `vida taskflow`

The current tracked-execution runtime family surface for the `0.9.0` transition line.

It already covers:

- route- and gate-aware execution
- role selection and conversational modes
- checkpoint / replay / recovery behavior
- verification merge and admissibility
- DB-backed task store with JSONL import
- final `taskflow -> DocFlow` runtime-consumption wiring

### 📚 `DocFlow`

The current documentation and inventory proof runtime for the transition line toward `1.0.0`.

It already covers:

- metadata and changelog normalization
- protocol and activation coverage checks
- readiness and proof checks
- canonical map health checks
- documentation-first mutation discipline

### 🌌 `vida`

The top-level product surface and release direction.

In the current `0.9.0` transition line, the installer gives you one `vida` launcher and the active operator/runtime surfaces are `vida taskflow` and `vida docflow`.

The next product target behind that launcher is:

- `Release 1`: host-shell CLI integration for one visible `VIDA` operator surface
- `Release 2`: host-project integration where the same runtime embeds into another project environment

---

## 🏗️ Standards Already Developed

This repository is not just “some tooling”. It already contains several hardened standards and canonical maps:

- 🗺️ framework root-map architecture
- 📚 canonical documentation and inventory layer matrix
- ⚙️ canonical runtime layer matrix
- 👥 role / skill / profile / flow model
- 🤖 auto-role and conversational-mode model
- 📦 compiled runtime bundle contract
- 🗃️ DB-first project activation and configurator model
- 👥 team coordination model
- 📊 status-family and query-surface model
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
- 📚 [Canonical Documentation & Inventory Layer Matrix](docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md)

### Spec navigation

- 📑 [Current Spec Map](docs/product/spec/current-spec-map.md)
- 🧭 [Release 1](docs/product/spec/release-1-plan.md)
- 🎯 [Compiled Autonomous Delivery Runtime Architecture](docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md)
- 🧱 [Runtime Surface Model](docs/product/spec/root-map-and-runtime-surface-model.md)
- 👥 [Role / Skill / Profile / Flow Model](docs/product/spec/agent-role-skill-profile-flow-model.md)
- 🧠 [Role Selection & Conversation Modes](docs/product/spec/agent-lane-selection-and-conversation-mode-model.md)
- 🗃️ [Project Activation & Configurator Model](docs/product/spec/project-activation-and-configurator-model.md)
- 📦 [Compiled Runtime Bundle Contract](docs/product/spec/compiled-runtime-bundle-contract.md)

---

## 🧪 Quick Commands

Typical documentation/runtime proving flow:

```bash
vida docflow overview

vida docflow readiness-check --profile active-canon

vida taskflow task import-jsonl .vida/imports/tasks.seed.jsonl --json

vida taskflow consume final "Runtime closure proof path" --json

vida taskflow consume continue --json

vida taskflow consume advance --max-rounds 4 --json

vida taskflow protocol-binding sync --json
```

---

## 🧠 Architecture Direction

`VIDA 0.9.0` transition is the self-hosting stabilization and runtime-hardening line before `Release 1`.

Its job is to make the runtime trustworthy enough that `Release 1` can be built on stable DB-first semantics and predictable operator behavior.

That means:

- `vida taskflow` and `vida docflow` remain the current public runtime-family surfaces
- `vida taskflow consume final|continue|advance` now form the canonical launcher-owned intake and bounded scheduler progression path
- `vida taskflow protocol-binding sync|status|check` are the active protocol-binding synchronization surfaces
- source-of-truth law stays in `docs/product/spec/`, `vida/config/`, and `vida/config/instructions/`
- current release work hardens semantics, schema migrations, and operator stability before `1.0` closure
- Rust `taskflow` and `docflow` remain active parallel implementation tracks for `Release 1`
- future `vida` composes those compiled runtimes into one CLI-first autonomous delivery runtime

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
- 🤝 **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)
- 🧭 **Support:** [SUPPORT.md](SUPPORT.md)
- 🔐 **Security:** [SECURITY.md](SECURITY.md)
- 🫶 **Conduct:** [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

-----
artifact_path: project/repository/readme
artifact_type: repository_doc
artifact_version: '1'
artifact_revision: '2026-03-15'
schema_version: '1'
status: canonical
source_path: README.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-15T09:05:34+02:00'
changelog_ref: README.changelog.jsonl
