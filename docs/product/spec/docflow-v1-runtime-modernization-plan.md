# DocFlow v1 Runtime Modernization Plan

Status: active product architecture plan and Rust implementation program

Purpose: define the canonical plan for rebuilding `DocFlow` as a standalone Rust documentation/runtime family surface that preserves the closed documentation-layer canon, remains a bounded sibling runtime to `taskflow`, avoids hardcoded project semantics such as models/roles/tools/subagents, and provides an explicit lawful seam for final `taskflow -> docflow` runtime consumption.

## 1. Mission

The target is not a long-lived cleanup of `codex-v0/codex.py`.

The target is:

1. a standalone Rust `DocFlow` runtime family surface,
2. built as componentized crates from the start,
3. aligned to the documentation-layer matrix as the primary architectural canon,
4. kept independent from `taskflow` while remaining explicitly consumable by `taskflow`,
5. bridged from the current Python implementation only where parity or operational continuity requires it,
6. suitable for `VIDA 1.0` direct local runtime consumption without turning `DocFlow` into the owner of closure authority.

Compact rule:

1. `codex-v0` remains the bounded donor, proof source, and compatibility bridge,
2. `docflow-rs` becomes the target runtime family implementation,
3. `taskflow` remains the execution substrate and final closure authority,
4. `DocFlow` remains the bounded documentation/inventory/validation/readiness/proof surface.

## 2. Mandatory Canon Alignment

This plan must align with all of the following:

1. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
2. `docs/product/spec/canonical-inventory-law.md`
3. `docs/product/spec/canonical-relation-law.md`
4. `docs/product/spec/canonical-runtime-readiness-law.md`
5. `docs/product/spec/root-map-and-runtime-surface-model.md`
6. `vida/config/instructions/system-maps/runtime-family.docflow-map.md`
7. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
8. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`

Alignment rule:

1. `docflow-rs` must preserve and implement Layers 1 through 7 directly,
2. `docflow-rs` must be Layer-8-ready but must not claim Layer 8 closure by itself,
3. final Layer 8 closure occurs only when `taskflow` consumes `docflow` surfaces as runtime authority,
4. nothing in `docflow-rs` may weaken the explicit `taskflow -> docflow` downstream branch.

## 3. Non-Negotiable Architectural Rules

### 3.1 Kernel Neutrality Rule

Kernel code may hardcode only:

1. documentation-layer law,
2. metadata/footer/changelog contracts,
3. inventory/validation/relation/readiness/proof result classes,
4. runtime-family ownership boundaries,
5. fail-closed mutation and validation invariants.

Kernel code must not hardcode:

1. model names,
2. provider names,
3. project roles,
4. project skills,
5. project profiles,
6. tool identities,
7. subagent names or classes,
8. project-specific bundles,
9. project-specific command families,
10. project-specific workflow semantics.

### 3.2 Runtime-Family Boundary Rule

1. `DocFlow` is a bounded runtime family, not the hidden owner of framework truth,
2. framework and product law remain in canonical docs and `vida/config/**`,
3. `docflow-rs` consumes and materializes that law operationally,
4. `docflow-rs` must stay independently intelligible while remaining one member of the broader VIDA runtime family.

### 3.3 Layer Authority Rule

1. Layer 1 through Layer 7 law is owned by canonical specs and protocols, not by implementation convenience,
2. `docflow-rs` may implement only behavior already described by those layers,
3. if implementation reveals a green-layer gap, the spec must be corrected before closure.

### 3.4 Generated Artifact Authority Rule

1. canonical registry and readiness artifacts are evidence surfaces, not the primary source of law,
2. generated outputs must never become a second competing source of truth,
3. when generated output conflicts with canonical docs/config, the law wins and the generated artifact must be corrected.

### 3.5 Bridge Rule

The current Python line remains:

1. a semantic oracle,
2. a parity source,
3. an operational compatibility bridge,
4. a migration donor.

The bridge must not remain the place where new permanent architecture is designed.

### 3.6 Mutation Atomicity Rule

For any lawful mutation path:

1. artifact body/footer mutation must be explicit,
2. sidecar changelog write must be explicit,
3. bounded validation must run after mutation,
4. partial-write or drift states must fail closed,
5. mutation logic must remain deterministic and auditable.

### 3.7 Discoverability Rule

1. `docflow-rs` must enter the root-map/runtime-family stack explicitly,
2. no scattered historical path or wrapper may become the hidden discovery route,
3. activation triggers for `DocFlow` surfaces must remain explicit in runtime-family maps and documentation tooling maps.

## 4. Build Strategy

### 4.1 Fast-But-Effective Delivery Rule

The fastest safe path is:

1. library-first,
2. contracts-first,
3. command capability freeze before rewrite,
4. one vertical layer slice at a time,
5. proof and parity at the end of every wave.

### 4.2 Donor And Input Sources

Canonical architecture inputs:

1. the documentation-layer matrix,
2. the inventory/relation/readiness laws,
3. runtime-family maps,
4. direct runtime-consumption protocol,
5. project documentation system law.

Implementation donors:

1. `codex-v0/codex.py`
2. `codex-v0/docsys_policy.yaml`
3. `codex-v0/docsys_schema.yaml`
4. `codex-v0/docsys_project.yaml`
5. helper wrappers under `vida/*.py` and `docs/*.py`
6. installer and launcher surfaces under `install/**`
7. `taskflow-v0/src/core/direct_consumption.nim`

Usage rule:

1. architecture comes from canonical docs,
2. donors provide parity, fixtures, and operational migration inputs,
3. when donor behavior conflicts with canon, the canon wins.

### 4.3 Scope Rule

In scope for `DocFlow v1`:

1. schema/config loading,
2. markdown/footer/changelog mutation kernel,
3. inventory materialization,
4. relation analysis,
5. validation and readiness,
6. operator surfaces,
7. explicit `taskflow -> docflow` consumption seam,
8. launchers and helper-surface migration planning.

Out of scope until the kernel is stable:

1. speculative shared crates with `taskflow-rs`,
2. project-specific artifact-body standards beyond canonical metadata law,
3. runtime-owned latest resolution inside `DocFlow` itself,
4. any execution-closure authority that belongs to `taskflow`.

## 5. Target Cargo Workspace

The Rust rewrite should be a Cargo workspace with one ownership boundary per concern.

```text
docflow-rs/
├── Cargo.toml
├── crates/
│   ├── docflow-core/
│   ├── docflow-contracts/
│   ├── docflow-config/
│   ├── docflow-format-jsonl/
│   ├── docflow-format-toon/
│   ├── docflow-markdown/
│   ├── docflow-inventory/
│   ├── docflow-relations/
│   ├── docflow-validation/
│   ├── docflow-readiness/
│   ├── docflow-operator/
│   ├── docflow-cli/
│   └── docflow-bridge-py/
└── tests/
```

### 5.1 Crate Ownership

1. `docflow-core`
   - shared value types, layer/result vocabulary, error IDs, path/value helpers
2. `docflow-contracts`
   - typed row envelopes, proof/readiness/inventory contracts, exit/result classes
3. `docflow-config`
   - policy/schema/project loading, profiles, scope selection, ignore rules
4. `docflow-format-jsonl`
   - canonical JSONL encoding/decoding for registry, readiness, and issue rows
5. `docflow-format-toon`
   - canonical TOON rendering for operator/proof surfaces
6. `docflow-markdown`
   - footer parsing/rendering, changelog I/O, mutation primitives, link extraction
7. `docflow-inventory`
   - scan, record build, registry rows, summary payloads, inventory classification
8. `docflow-relations`
   - deps, deps-map, links, artifact-impact, task-impact, reference index
9. `docflow-validation`
   - footer/schema/link/activation/protocol/projection validation
10. `docflow-readiness`
    - readiness-check, readiness-write, doctor aggregation, proofcheck aggregation
11. `docflow-operator`
    - overview, summary, layer-status, compact operator views
12. `docflow-cli`
    - command shell only
13. `docflow-bridge-py`
    - parity fixtures and bounded compatibility shims to `codex-v0`

### 5.2 Dependency Law

1. `docflow-core` depends on nothing project-specific
2. `docflow-contracts` depends on `docflow-core`
3. `docflow-config` may depend on `docflow-core` and `docflow-contracts`
4. `docflow-format-jsonl` and `docflow-format-toon` may depend on `docflow-core` and `docflow-contracts`
5. `docflow-markdown` may depend on `docflow-core`, `docflow-contracts`, and `docflow-config`
6. `docflow-inventory`, `docflow-relations`, `docflow-validation`, and `docflow-readiness` may depend on lower-level crates only
7. `docflow-operator` may depend on all non-bridge library crates
8. `docflow-cli` may depend on all workspace crates
9. `docflow-bridge-py` may depend on all workspace crates, but no kernel crate may depend back on it

Shared-family rule:

1. no support crate may be extracted for both `taskflow-rs` and `docflow-rs` until both runtimes prove the same need independently,
2. runtime-family clarity is more important than early deduplication.

## 6. Selected Library Policy

### 6.1 Approved Foundation Dependencies

1. `clap`
2. `serde`
3. `serde_json`
4. `serde-jsonlines`
5. `toon-format`
6. `thiserror`
7. `tracing`
8. `tracing-subscriber`
9. `regex`
10. `globset`
11. `walkdir`
12. `tempfile`
13. `fs-err`

### 6.2 Approved With Limits

1. `anyhow`
   - binaries and glue code only
2. `indexmap`
   - only where deterministic ordering is part of canonical output
3. YAML backend crate
   - mandatory pre-Wave-1 decision through the same library-evaluation gate used for `taskflow-rs`
   - must not force project semantics into the kernel

### 6.3 Rejected For Foundation Use

1. project-specific SDKs or agent SDKs
2. libraries that hardcode provider/model/agent assumptions
3. speculative shared runtime-family helpers

## 7. Canonical Capability Matrix

The rewrite must preserve capabilities, not just command names.

### 7.1 Required Capabilities

1. schema/config/profile loading
2. inventory scan and registry materialization
3. markdown/footer/changelog mutation
4. relation and impact analysis
5. fail-closed validation
6. readiness and grouped proof
7. compact operator surfaces
8. direct `taskflow` consumption seam

### 7.2 Current Transitional Command Families

1. inventory/operator
   - `scan`, `summary`, `overview`, `layer-status`, `registry`, `registry-write`
2. mutation
   - `touch`, `finalize-edit`, `init`, `move`, `rename-artifact`, `migrate-links`
3. relations
   - `deps`, `deps-map`, `artifact-impact`, `task-impact`, `links`
4. validation/readiness/proof
   - `check`, `fastcheck`, `activation-check`, `protocol-coverage-check`, `readiness-check`, `readiness-write`, `proofcheck`, `doctor`

Command rule:

1. command names may evolve,
2. capability coverage may not regress,
3. every current command family must map to explicit target crates and parity fixtures before cutover.

## 8. Canonical Output And Exit Contract

### 8.1 Output Rule

1. operator-facing compact views default to TOON,
2. machine-facing registry/readiness/issue rows remain JSONL-first,
3. output field ordering must be deterministic,
4. output schemas must be explicit and versionable.

### 8.2 Generated Artifact Continuity Rule

The plan must preserve the canonical generated paths unless canon explicitly promotes a replacement.

Current canonical paths:

1. `vida/config/codex-registry.current.jsonl`
2. `vida/config/codex-readiness.current.jsonl`

Rules:

1. `docflow-rs` may replace the producer,
2. it must not silently replace the canonical artifact paths,
3. any path change requires product-law promotion first.

### 8.3 Exit Contract Rule

Before cutover, `docflow-rs` must fix explicit result and exit semantics for:

1. success,
2. blocking validation/readiness/proof failure,
3. usage/configuration failure,
4. mutation-disabled or skipped paths,
5. bridge/parity unavailable conditions.

## 9. Operational Migration Surfaces

The plan must explicitly cover all non-library surfaces that currently depend on `codex-v0`.

### 9.1 Required Migration Inventory

At minimum:

1. runtime-family maps
2. project documentation tooling map
3. installer launchers
4. helper wrapper scripts
5. `taskflow-v0` direct-consumption hook

### 9.2 Launcher And Wrapper Rule

1. `DocFlow v1` must define the future launcher posture explicitly,
2. compatibility wrappers may remain temporarily,
3. wrapper retirement must be tied to parity proof, not to code existence alone.

### 9.3 TaskFlow Seam Rule

The plan must define the minimum parity needed before `taskflow` can switch from `codex-v0` to `docflow-rs` for:

1. overview,
2. readiness,
3. proof evidence,
4. direct runtime-consumption compatibility.

## 10. Testing And Proof Strategy

### 10.1 Required Test Classes

1. unit tests per crate
2. integration tests across crate boundaries
3. golden TOON/JSONL tests
4. mutation atomicity tests
5. relation edge and impact tests
6. readiness/proof aggregation tests
7. parity tests against `codex-v0`
8. `taskflow -> docflow` seam tests

### 10.2 Golden Fixture Corpus

The fixture corpus must cover at minimum:

1. valid canonical markdown artifact
2. footer-optional bootstrap carrier
3. missing changelog
4. broken markdown link
5. broken footer reference
6. missing projection target
7. projection tuple mismatch
8. unsupported compatibility class
9. bundle family gap
10. orphan changelog
11. task-impact history case

### 10.3 Review And Closure Rule

Every wave closes only when:

1. changed-scope tests pass,
2. `qwen` review is complete,
3. `kilo` review is complete,
4. critical and high findings are fixed or explicitly recorded,
5. if canonical docs changed, `codex-v0 finalize-edit/check/proofcheck` pass,
6. if the wave touches the `taskflow` seam, direct-consumption parity remains explicit and green.

## 11. Wave Plan

### Wave C0 — Plan, Parity Freeze, And Capability Ledger

Deliverables:

1. canonical `DocFlow v1` plan promoted
2. capability matrix fixed
3. command parity ledger created
4. operational migration inventory created

Exit criteria:

1. no required capability is unowned,
2. donor sources are explicit,
3. Layer 8 dependency on `taskflow` is explicit.

#### Wave C0 Freeze Packet

The Wave C0 planning gate is closed only when the capability matrix, donor map, launcher migration surfaces, and output contracts are explicit.

Approved target crate ownership for the first implementation line:

1. `docflow-core`
2. `docflow-contracts`
3. `docflow-config`
4. `docflow-format-jsonl`
5. `docflow-format-toon`
6. `docflow-markdown`
7. `docflow-inventory`
8. `docflow-relations`
9. `docflow-validation`
10. `docflow-readiness`
11. `docflow-operator`
12. `docflow-cli`
13. `docflow-bridge-py`

Primary donor/runtime sources inspected for parity:

1. `codex-v0/codex.py`
2. `codex-v0/docsys_policy.yaml`
3. `codex-v0/docsys_schema.yaml`
4. `codex-v0/docsys_project.yaml`
5. `codex-v0/requirements-python.txt`
6. `docs/process/documentation-tooling-map.md`
7. `vida/config/instructions/system-maps/runtime-family.docflow-map.md`
8. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
9. `taskflow-v0/src/core/direct_consumption.nim`
10. `install/install.sh`
11. `scripts/docsys/print-markdown-summary-jsonl.py`
12. `crates/vida/src/main.rs`

Current donor command and operator surfaces inspected:

1. `python3 codex-v0/codex.py check --profile active-canon`
2. `python3 codex-v0/codex.py fastcheck --profile active-canon`
3. `python3 codex-v0/codex.py activation-check --profile active-canon`
4. `python3 codex-v0/codex.py protocol-coverage-check --profile active-canon`
5. `python3 codex-v0/codex.py readiness-check --profile active-canon`
6. `python3 codex-v0/codex.py proofcheck --profile active-canon-strict`
7. `vida docflow help`
8. `vida docflow <delegated-args>`

Current artifact and output-shape expectations preserved or intentionally bridged:

1. active-canon donor profiles stay scoped to `docs/product`, `docs/process`, `docs/project-memory`, and `vida/config/instructions`, while temp/install/dist/runtime-package trees remain ignored by default
2. canonical layer, framework-map, activation-protocol, and protocol-index lookups resolve through the current matrix/map paths rather than renamed legacy flat-file paths
3. owner-layer expectations no longer treat `docs/framework/plans/**` or `docs/framework/research/**` as active project-doc canon
4. packaged compatibility environments must carry an explicit Python dependency manifest through `codex-v0/requirements-python.txt`
5. registry/readiness/proof surfaces remain canonical JSONL evidence outputs
6. operator surfaces remain low-call and TOON-first where the donor path is human-facing
7. footer metadata and sidecar changelog lineage remain explicit and lawful
8. generated evidence artifacts remain secondary to canonical docs/specs/protocols
9. final consumption remains an explicit `taskflow -> docflow` seam rather than hidden shared state

Approved dependency policy for Wave C0 and Wave C1:

1. kernel crates stay neutral to models, providers, roles, tools, subagents, and project-specific semantics
2. foundation crates may use `serde`, `serde_json`, `serde-jsonlines`, `toon-format`, `thiserror`, `tracing`, `regex`, `globset`, and `walkdir`
3. CLI shell may additionally use `clap` and `anyhow`
4. Python remains donor-only through `docflow-bridge-py`; no kernel crate may depend on the Python bridge
5. no shared crate may be extracted with `taskflow-rs` until both sides prove the same need independently

Wave C0 proof rule:

1. the capability matrix must map every donor command family to one target crate/service owner
2. the donor map must name the currently inspected files and command surfaces rather than future placeholders
3. launcher, wrapper, and direct-consumption migration surfaces must stay explicit before Wave C1 implementation expands kernel code
4. donor profile scope, ignore rules, canonical path rewires, and packaged Python dependencies must remain explicit while `codex-v0` is still an operational bridge

### Wave C1 — Core, Contracts, Config, And Formats

Deliverables:

1. `docflow-core`
2. `docflow-contracts`
3. `docflow-config`
4. format crates

Exit criteria:

1. typed contracts exist,
2. no CLI logic owns kernel behavior,
3. config/profile resolution is deterministic and fail-closed.

### Wave C2 — Markdown And Mutation Kernel

Deliverables:

1. `docflow-markdown`
2. footer/changelog/mutation primitives
3. atomic mutation semantics

Exit criteria:

1. mutation paths are deterministic,
2. sidecar lineage semantics are preserved,
3. bootstrap exceptions remain policy-driven rather than ad hoc.

### Wave C3 — Inventory And Relations

Deliverables:

1. `docflow-inventory`
2. `docflow-relations`
3. canonical registry materialization

Exit criteria:

1. inventory coverage matches Layer 2 law,
2. relation outputs match Layer 5 law,
3. deterministic ordering is proven.

### Wave C4 — Validation, Readiness, And Proof

Deliverables:

1. `docflow-validation`
2. `docflow-readiness`
3. explicit issue/proof/readiness result envelopes

Exit criteria:

1. Layer 3 and Layer 7 behaviors are fail-closed,
2. protocol and activation coverage remain explicit,
3. proof aggregation is stable and reviewable.

### Wave C5 — Operator Surfaces And CLI

Deliverables:

1. `docflow-operator`
2. `docflow-cli`
3. TOON-first operator surfaces

Exit criteria:

1. operator surfaces remain low-call,
2. command-home boundaries are explicit,
3. CLI remains a thin shell over library crates.

### Wave C6 — Wrappers, Installer, And Bridge Migration

Deliverables:

1. launcher migration
2. helper-script migration
3. `docflow-bridge-py`
4. operational shim posture documented

Exit criteria:

1. operational surfaces are inventoried,
2. compatibility wrappers are bounded,
3. no hidden `codex-v0` dependency remains unexplained.

#### Wave C6A Task — Donor Scope, Canonical Paths, And Packaging Parity

Bounded development task opened from the post-epic donor audit:

1. promote the current `codex-v0` bridge-operational scope into explicit Rust-wave requirements instead of leaving it implicit in donor scripts.

Required closure for this task:

1. preserve the narrowed `active-canon` / `active-canon-strict` profile scope and the current ignore-set behavior as typed Rust config/parity inputs,
2. preserve the current canonical path rewires for layer-matrix, framework-map, activation-protocol, and protocol-index lookups,
3. preserve the donor ignore-set concretely enough to cover `_temp`, `_vida`, `dist`, `install`, `projects`, `.agents`, `target`, and `.venv` without re-expanding scan scope accidentally,
4. preserve the current owner-layer expectation that `docs/framework/plans/**` and `docs/framework/research/**` are outside the active project-doc canon,
5. preserve the packaged Python dependency manifest as an explicit compatibility input while `codex-v0` remains operational,
6. keep installed and launcher compatibility boundaries explicit rather than rediscovering them later through release regressions.

Task proof:

1. parity tests must show that the Rust config/validation surfaces resolve the same canonical paths as the active donor bridge,
2. parity tests must show that the Rust config/validation surfaces retain the same active-canon scope and ignore behavior as the active donor bridge,
3. install/compatibility proofs must show that packaged dependency expectations remain explicit while the Python bridge is still active,
4. the epic may not treat bridge migration as scoped correctly until donor profile scope, ignore rules, owner-scope boundaries, canonical path rewires, and packaged compatibility inputs are explicit in the implementation program.

### Wave C7 — TaskFlow Seam And Layer 8 Closure Preparation

Deliverables:

1. explicit `taskflow-rs -> docflow-rs` consumption contract implemented
2. direct-consumption parity fixtures
3. Layer 8 spec and proof inputs aligned

Exit criteria:

1. `docflow-rs` is Layer-8-ready,
2. `taskflow` remains the closure authority,
3. final Layer 8 closure conditions are explicit and testable.

## 12. Completion Criteria

This program is complete only when:

1. `docflow-rs` owns Layer 1 through Layer 7 behavior directly,
2. no project-specific roles/models/tools/subagents are hardcoded in kernel crates,
3. runtime-family maps and wrappers point lawfully to the new runtime surface,
4. canonical generated artifact paths remain explicit and stable,
5. the `taskflow -> docflow` consumption seam is explicit and proven,
6. `codex-v0` is reduced to a bounded bridge or retired,
7. every wave has passed tests and independent review,
8. Layer 8 closure remains owned by `taskflow`, not by `DocFlow`.

## 13. Immediate Next Step

The next lawful step after this plan is:

1. add this plan to the active spec canon,
2. create the command parity ledger and operational migration inventory,
3. define the separate Layer 8 runtime-consumption spec/template for `DocFlow v1`,
4. open the bounded Rust development task for donor-scope/path/packaging parity from Wave C6A so the current bridge contract is not lost during rewrite,
5. then start Wave C1 with workspace skeleton plus core/contracts/config/format crates.

-----
artifact_path: product/spec/docflow-v1-runtime-modernization-plan
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/docflow-v1-runtime-modernization-plan.md
created_at: '2026-03-10T23:03:37+02:00'
updated_at: '2026-03-12T18:00:02+02:00'
changelog_ref: docflow-v1-runtime-modernization-plan.changelog.jsonl
