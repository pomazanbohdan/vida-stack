# Vida Stack

Vida Stack is currently being developed with the help of OpenAI Codex as the active implementation and orchestration environment for the framework's real-world evolution.

Vida Stack is an agentic engineering framework for building a highly autonomous product-development orchestrator.

Its purpose is not to be another task tracker or prompt collection. The goal is to evolve a real control plane for agent-driven product engineering: planning, execution, verification, documentation sync, telemetry, learning loops, and multi-agent orchestration working as one coherent system.

## Why This Exists

Modern AI-assisted development still breaks in predictable ways:

- workflows drift away from source-of-truth state
- prompts and protocols diverge from runtime behavior
- review and verification stay optional instead of enforced
- parallel agents create noise, conflicts, and duplicated work
- context cost grows faster than execution quality

Vida Stack exists to solve those problems with a framework that is:

- protocol-driven
- verification-first
- orchestration-native
- telemetry-aware
- optimized for real product delivery, not demos

## Project Goal

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

## Target System Shape

The target architecture is organized around a small set of core subsystems:

- `VS-Control` for orchestration, decomposition, routing, and escalation
- `VS-State` for authoritative workflow state, execution history, capsules, and health
- `VS-Memory` for durable operational memory and distilled lessons
- `VS-Verify` for review, policy, test, and approval gates
- `VS-Observe` for telemetry, scorecards, and drift visibility
- `VS-Learn` for reflection, evaluation, and improvement loops
- `VS-DocSync` for documentation actualization and canonical-document promotion

These subsystems are meant to work as one control plane rather than as disconnected scripts, prompts, and docs.

## Current Stage

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

## Development Philosophy

Vida Stack is being developed with a few non-negotiable principles:

1. Real-project validation before platform extraction.
2. Root-cause fixes over cosmetic automation.
3. Single authoritative execution state.
4. Verification as runtime behavior, not team culture.
5. Legacy-zero evolution: no parallel "old and new" truths.
6. Lean by default, richer orchestration only when justified.
7. External trends in AI engineering should inform the roadmap, but only after they prove operational value.

## What Is Being Built

The framework is moving toward a system with these major capabilities:

- problem framing and execution routing
- structured task decomposition
- bounded multi-agent orchestration
- review and verification state machines
- policy-aware execution
- scorecards and telemetry by work type and domain
- documentation and contract synchronization
- compact context hydration and lower token overhead
- learning loops based on observed runtime behavior

## Architecture Baseline

Vida Stack follows a modern agent-platform architecture for production-grade autonomous engineering systems.

At the highest level, the architecture is:

1. User Request
2. Goal Interpreter
3. Planner Agent
4. Agent Control Loop
5. Tool Router
6. Execution Environment
7. Observation Layer
8. Memory System
9. Telemetry and Evaluation

Core principle:

Agents operate in a continuous loop of:

1. observation
2. planning
3. action
4. verification
5. reflection
6. improvement

This means Vida Stack is being designed not as a single prompt runner, but as a persistent agent runtime with control, state, feedback, and governance.

## Core Runtime Model

The target runtime model includes:

1. a goal interpreter that turns user intent into executable work
2. a planner that decomposes and routes tasks
3. a control loop that can adapt after each observation
4. a tool router that validates and dispatches actions
5. an execution environment with:
   - terminal
   - filesystem
   - browser
   - code runtime
   - external API integrations
6. an observation layer that normalizes results
7. a memory layer that preserves useful operational knowledge
8. a telemetry and evaluation layer for continuous improvement

The longer-term runtime direction also includes:

1. an event-oriented workflow kernel with deterministic recovery after interruption or compaction
2. clear separation between workflow state, memory, documentation state, and telemetry
3. protocol rules that exist as docs, machine-readable policy artifacts, and runtime enforcement
4. compact context packets that reduce repeated rereads of large markdown surfaces

## Multi-Agent Model

Vida Stack is being developed toward a role-based multi-agent architecture.

Core roles include:

1. planner
2. researcher
3. executor
4. critic or reviewer
5. integrator
6. supervisor

This role split is important because it allows bounded decomposition, explicit verification, and cleaner ownership of work products.

The target model also assumes explicit leases and ownership:

1. task or block ownership per active agent run
2. optional file or worktree scope where mutation is involved
3. release or expiration rules for parallel work

That ownership model is meant to reduce duplicate work, write conflicts, and noisy integrations.

## Planning and Reasoning Model

Vida Stack is intended to support structured planning methods rather than free-form guessing.

Important planning families include:

1. reasoning plus acting flows
2. plan-and-execute flows
3. branching reasoning for complex decisions
4. graph-like workflow execution for dependent work

The framework should be able to choose lighter or richer reasoning patterns depending on task complexity, risk, and execution profile.

## Memory and Learning Model

Vida Stack is not intended to rely on prompt context alone.

Its target memory shape includes:

1. short-term context
2. episodic memory
3. semantic memory
4. procedural memory
5. decision memory
6. failure memory
7. reflection memory

The learning model is built around three loops:

1. Execution loop:
   `Plan -> Act -> Verify`
2. Learning loop:
   `Telemetry -> Reflection -> Memory`
3. Improvement loop:
   `Evaluation -> Prompt/Policy Updates -> System Upgrade`

## Telemetry, Evaluation, and Safety

Vida Stack is intended to treat telemetry and governance as first-class runtime concerns.

The target telemetry shape includes:

1. task success rate
2. tool success rate
3. latency
4. cost per task
5. human intervention rate
6. reasoning quality proxy
7. verification pass rate
8. scorecards and drift signals

The target safety and governance shape includes:

1. policy-controlled tool access
2. risk-based approval gates
3. human checkpoints for critical actions
4. explicit review and audit surfaces
5. drift detection when models, prompts, tools, or data behavior change

## Cost and Efficiency Model

Vida Stack is being optimized for real engineering throughput, not maximal orchestration theater.

The main efficiency strategies are:

1. model routing by task type
2. prompt and artifact reuse
3. context pruning and compact hydration
4. external-first cheap read-only fanout where justified
5. stronger runtime artifacts to reduce repeated markdown rereads

Over time this should evolve into a lighter compiled-policy layer:

1. boot and execution policy packets
2. required evidence schemas
3. compact handoff and hydration payloads

The goal is lower token burn, less protocol/runtime drift, and easier automation of health and verification checks.

## Minimal Production Shape

The minimum serious production shape for Vida Stack includes:

1. an LLM/runtime model layer
2. a framework control plane
3. persistent workflow and memory layers
4. observability and telemetry
5. evaluation and benchmark capabilities
6. verification and review gates
7. human escalation and approval paths

That is the baseline architecture Release 1 is converging toward while the framework is still validated inside a real project.

## Roadmap

The roadmap is staged on purpose.

### Phase 1: Real-Project Runtime Hardening

Current focus:

- finish orchestration mechanics on a real project
- stabilize task and TODO execution flow
- harden subagent routing, fallback, and arbitration
- improve verification gates and scorecards
- reduce protocol/runtime drift
- capture reusable runtime patterns

This is the phase we are in now.

### Phase 2: Framework Extraction

Next focus:

- separate framework concerns from project-specific concerns cleanly
- tighten machine-enforced runtime contracts
- introduce more formal policy packets and runtime manifests
- improve portability across repositories
- prepare a cleaner public framework surface

### Phase 3: Daemonized Control Plane

Target focus:

- background orchestration services
- reactive status and health monitoring
- richer doc-sync and verification workers
- stronger event-driven runtime behavior

### Phase 4: Full Control Plane

Target focus:

- durable workflow kernel
- richer review and verification fabric
- deeper telemetry and learning loops
- stronger ownership and lease models for parallel agents
- documentation synchronization as a first-class subsystem
- more complete control-plane behavior in line with the VS target architecture

### Phase 5: Rust Reimplementation

The planned endgame is a full system implemented in Rust.

Why Rust:

- stronger runtime integrity
- better performance for long-running orchestration services
- safer concurrency for multi-agent and event-driven execution
- more robust foundation for longer-running daemonized orchestration

The current shell/Python/docs runtime is not wasted work. It is the proving ground that defines what the Rust system should actually implement.

## Open-Source Direction

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
- protocol compiler/runtime artifact generation
- Rust control-plane implementation

## Repository Structure

Today, the framework lives here:

- [AGENTS.md](AGENTS.md)
- [_vida/docs/protocol-index.md](_vida/docs/protocol-index.md)

Key runtime areas:

- [_vida/docs](_vida/docs)
- [_vida/scripts](_vida/scripts)
- [vida.config.yaml](vida.config.yaml)

## Current Reality vs Future Vision

Current reality:

- Vida Stack is an actively used framework layer inside a real project
- many core mechanics already exist and are exercised daily
- some parts are still markdown-heavy and script-heavy by design
- several advanced control-plane ideas are still being validated, not productized

Future vision:

- a polished standalone framework
- stronger machine-enforced runtime contracts
- durable orchestration and verification subsystems
- a Rust-based implementation for the full system
- a contributor-friendly open-source control plane for agentic product engineering

## North Star

The north star for Vida Stack is simple:

Build a complete, high-integrity, highly autonomous product-development orchestrator that can evolve with the state of AI while remaining grounded in real engineering work.
