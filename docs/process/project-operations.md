# Project Operations

Current operating baseline:

- bootstrap through `AGENTS.md` followed by the bounded VIDA init surfaces
- use `AGENTS.sidecar.md` as the project documentation map
- while project activation is pending, do not enter TaskFlow execution; use `vida project-activator` and `vida docflow`

Default feature-delivery flow:

1. If the request asks for research, specifications, a plan, and then implementation, start with a bounded design document.
2. Use the local template at `docs/product/spec/templates/feature-design-document.template.md`.
3. Open one feature epic and one spec-pack task in `vida taskflow` before code execution.
4. Keep the design artifact canonical through `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check`.
5. Close the spec-pack task and shape the next work-pool/dev packet in `vida taskflow` after the design document names the bounded file set, proof targets, and rollout.
6. When `.codex/**` is materialized, use the delegated Codex team surface instead of collapsing the root session directly into coding.
7. Treat `vida.config.yaml` as the owner of carrier tiers and optional internal Codex aliases; project-visible activation should still use the selected carrier tier plus explicit runtime role.
8. Let runtime map the current packet role into the cheapest capable carrier tier with a healthy local score from `.vida/state/worker-strategy.json`.
9. Keep the root session in orchestration posture unless an explicit exception path is recorded.
10. Prefer the launcher-owned intake/runtime progression surfaces over manual reconstruction:
   - `vida taskflow consume final "<request>" --json` to materialize the routed intake, dispatch receipt, and first lawful packet
   - `vida taskflow consume continue [--run-id <run-id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]` to resume one persisted chain entry
   - `vida taskflow consume advance [--run-id <run-id>] [--max-rounds <n>] [--json]` to let the bounded scheduler progress ready steps automatically
