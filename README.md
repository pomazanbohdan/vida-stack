<div align="center">
  <h1>🌌 Vida Stack</h1>
  <p><b>The active repository for the <code>VIDA 0.9.x</code> transition line: self-hosting stabilization and runtime hardening before <code>1.0.0</code>.</b></p>

  <p>
    <a href="#"><img src="https://img.shields.io/badge/Status-Active_Development-brightgreen" alt="Status"></a>
    <a href="#"><img src="https://img.shields.io/badge/Release-v0.9.7-blue" alt="Release"></a>
    <a href="#"><img src="https://img.shields.io/badge/Runtime-TaskFlow-orange" alt="Runtime"></a>
    <a href="#"><img src="https://img.shields.io/badge/Docsys-DocFlow-teal" alt="Docsys"></a>
    <a href="#"><img src="https://img.shields.io/badge/Target-VIDA_1.0-purple" alt="Target"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MPL--2.0-brightgreen" alt="License"></a>
  </p>
</div>

> [!NOTE]
> **Current project capability:** the repository is now in the `0.9.x transition` phase (not final `1.0.0` closure). The current packaged release line is `v0.9.7`.
> - agent orchestration already works through the chief/root orchestrator and internal delegated lanes;
> - agent roles, project activation, and bounded team topology can already be configured;
> - framework and project specifications already govern documentation discipline and runtime routing;
> - the project backlog can already live in DB-backed runtime state instead of only in flat files;
> - `vida status` / `vida doctor` are stable on fresh booted state after schema and lock-handling hardening, and release artifacts now carry exact binary-version/build-timestamp evidence with a rendered public commit ledger (`2026-05-24`).
>
> **Current known gaps to `1.0.0`:**
> - DB-first authority for roles/skills/profiles/flows is still not fully closed (YAML projection remains active source);
> - activation/configurator lifecycle still depends on root `vida.config.yaml` bridge surfaces;
> - some proof-surface commands declared in specs are still fail-closed/unsupported in the launcher.
>
> **Internal validation status:** the current framework/spec stack has passed bounded documentation/runtime checks (`check`, `activation-check`, `protocol-coverage-check`, `proofcheck`), exact-version release manifest checks, and post-fix CLI smoke for the installed `vida` binary.
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

Repository make targets (PowerShell-backed, same local gate contract as Windows):

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
irm https://github.com/pomazanbohdan/vida-stack/releases/latest/download/vida-install.ps1 -OutFile vida-install.ps1
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

For local framework development, keep the active repair loop on the debug profile. Update the system launcher from the release profile only when the bounded acceptance target requires installed-runtime validation, packaging, or release admission:

Linux/macOS:

```bash
make vida-dev-script-check
make vida-dev-quick
make vida-test
make vida-test-workspace

# Unix fallback when PowerShell is not available:
cargo nextest run --locked -p vida --profile default
cargo nextest run --locked --workspace --profile ci

# Optional installed-runtime gate after focused proof is green.
cargo build --locked -p vida --release
install -D -m 755 target/release/vida ~/.local/share/vida-stack/current/bin/vida
export PATH="$HOME/.local/share/vida-stack/current/bin:$PATH"
vida status --json
```

Windows PowerShell:

```powershell
# Optional when the bundled Codex ripgrep is blocked by Windows app policy.
bun add -g @vscode/ripgrep
Copy-Item "$env:USERPROFILE\.bun\install\global\node_modules\@vscode\ripgrep\bin\rg.exe" "$env:USERPROFILE\.bun\bin\rg.exe" -Force
rg --version

# Fast package-scoped proof through the repo gate.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode quick -Json

# Smoke the debug runtime before using it for stateful runtime validation.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode runtime-smoke -Json

# Build and install the operator-facing launcher only for installed-runtime or release gates.
vida release install --json
vida status --json
```

For timed Windows proof loops, prefer the repo script:

```powershell
# Script/docs-only proof without Cargo.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode script-check -Json

# Fast debug source proof: fmt plus cargo check.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode quick -Json

# Focused regression proof through nextest.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode focused-nextest -TestFilter close_feedback_inference -Json

# Full vida package proof through nextest.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode package-nextest -Json

# Local workspace test gate mirroring CI's nextest profile.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode workspace-nextest -Json

# Rust doc tests.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode doc-test -Json

# Debug build of runtime entrypoints.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode build-debug -Json

# Cheap policy probe for a checkout or linked worktree before running Cargo.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode target-dir-policy -Json

# Debug runtime smoke: build debug vida and run status from the effective Cargo target dir.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode runtime-smoke -Json

# Windows-native release archive packaging without installing the operator-facing launcher.
powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1 -SkipBuild -Windows -ReleaseBinDir .\.vida\cargo-target\release -Json

# Timed gate wrapper for the same Windows-native skip-build package path.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode release-package -SkipBuild -Windows -ReleaseBinDir .\.vida\cargo-target\release -Json

# Focused no-install package smoke with isolated fixture binaries and manifest validation.
powershell -ExecutionPolicy Bypass -File scripts/check-release-package.ps1 -Json

# Installed runtime proof only when the bounded target requires the operator-facing launcher.
powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode release-install -Json
```

`scripts/vida-dev-gate.ps1` keeps Cargo output visible and deterministic by setting `CARGO_TARGET_DIR` when the caller has not already set it. Normal checkouts use `.vida\cargo-target`; linked worktrees under `.vida\worktrees\...` share the owner checkout's `.vida\cargo-target` so each worktree does not cold-build its own `target` tree. If the caller provides `CARGO_TARGET_DIR`, the script respects it and reports that policy in JSON timing records.

Local test gates use `cargo nextest`; install it before using the script on a fresh host. Use `powershell -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Help` to inspect the current local build/test modes instead of copying older direct Cargo commands.

This keeps ordinary proof runs fast and inspectable. The operator-facing `vida` in `~/.local/share/vida-stack/current/bin` on Unix-like systems or `%LOCALAPPDATA%\vida-stack\current\bin` on Windows should be refreshed from the release profile only when the installed launcher itself is part of the proof.

On Windows, Application Control, Smart App Control, or Device Guard may block any newly generated or downloaded executable, including Cargo build scripts under `target\debug\build\*\build-script-build.exe`, integration-test binaries under `target\debug\deps\*.exe`, nextest-launched test binaries, and release binaries installed under `%LOCALAPPDATA%\vida-stack\current\bin`. If that policy is active, local Windows `cargo build`, `cargo nextest`, and installer smoke may fail before VIDA code runs. Use WSL/Linux or GitHub Actions for proof builds, or allowlist/sign the release binaries before making them the active system runtime.

Known `v0.9.3` Windows host state from the 2026-05-01 release wave:

1. GitHub release packaging and Windows installer smoke passed on `windows-latest`.
2. The local Windows host installed `v0.9.3` under `%LOCALAPPDATA%\vida-stack\releases\v0.9.3`.
3. The same local host's Smart App Control policy initially blocked newly installed `vida.exe`, `taskflow.exe`, and `docflow.exe`.
4. A later explicit Windows installer run switched `%LOCALAPPDATA%\vida-stack\current` to `v0.9.3` and removed the stale PATH-shadowing `C:\Users\pomaz\.bun\bin\vida.exe` into a backup file.
5. Smart App Control was then disabled on the local developer host by setting `HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy\VerifiedAndReputablePolicyState=0` and refreshing Code Integrity with `CiTool.exe -r`.
6. The Windows `vida.cmd`, `taskflow.cmd`, and `docflow.cmd` launchers now resolve and execute the `v0.9.3` active release.
7. WSL on the same machine can also run `vida 0.9.3` and was used for local release proof.

Fresh-state smoke after installing `vida.exe`:

```powershell
$state = Join-Path $env:TEMP "vida-fresh-state-proof-$PID"
vida boot --state-dir $state
vida status --state-dir $state --summary --json
```

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
- file-based batch transport through `vida task import --file <path> --dry-run`
- final `taskflow -> DocFlow` runtime-consumption wiring

For large TaskFlow batches, put task objects in JSONL/YAML and run `vida task import --file <path> --dry-run` before applying. Put many dependency edges in a newline-delimited `issue_id:depends_on_id:edge_type` file and run `vida task dep add-bulk --edge-file <path> --dry-run`. These file workflows avoid command-line size limits and keep the batch reviewable.

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

- CLI-first runtime hardening for one visible `VIDA` operator surface
- host-project integration where the same runtime embeds into another project environment

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

vida task import --file tasks.jsonl --parent-id <parent-id> --dry-run

vida task dep add-bulk --edge-file edges.txt --dry-run

vida taskflow task import-jsonl .vida/imports/tasks.seed.jsonl --json

vida taskflow consume final "Runtime closure proof path" --json

vida taskflow consume continue --json

vida taskflow consume advance --max-rounds 4 --json

vida taskflow protocol-binding sync --json
```

---

## 🧠 Architecture Direction

`VIDA 0.9.0` transition is the self-hosting stabilization and runtime-hardening line before the next stable runtime profile.

Its job is to make the runtime trustworthy enough to build on stable DB-first semantics and predictable operator behavior.

That means:

- `vida taskflow` and `vida docflow` remain the current public runtime-family surfaces
- `vida taskflow consume final|continue|advance` now form the canonical launcher-owned intake and bounded scheduler progression path
- `vida taskflow protocol-binding sync|status|check` are the active protocol-binding synchronization surfaces
- source-of-truth law stays in `docs/product/spec/`, `vida/config/`, and `vida/config/instructions/`
- current release work hardens semantics, schema migrations, and operator stability before `1.0` closure
- Rust `taskflow` and `docflow` remain active parallel implementation tracks for the current runtime profile
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
artifact_revision: '2026-06-12'
schema_version: '1'
status: canonical
source_path: README.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-06-12T00:00:00+03:00'
changelog_ref: README.changelog.jsonl
