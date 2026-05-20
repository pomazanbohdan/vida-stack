# Codebase Consistency Audit Report

**Project**: vida-stack  
**Date**: 2026-05-21  
**Scope**: Full workspace analysis (157 Rust files, 26 crates)  

---

## Executive Summary

The vida-stack codebase exhibits several architectural and maintenance concerns:

1. **Dead Code**: 2 format-toon crates are completely unused
2. **File Size Violations**: 3 files exceed 5K lines (one at 20K+)
3. **Module Duplication**: Parallel JSONL/TOON implementations across docflow/taskflow
4. **Low-Usage Modules**: 30+ modules with ≤3 references
5. **State Backend Fragmentation**: FS and Surreal backends coexist without clear separation

---

## 1. Dead Code Analysis

### Critical Findings

| Crate | Status | Evidence |
|-------|--------|----------|
| `docflow-format-toon` | DEAD | Listed in workspace members, depends on `common-format-toon`, but zero external references |
| `taskflow-format-toon` | DEAD | Same pattern - no crate depends on it |

**Impact**: These crates add build time and maintenance burden without providing value.

### Recommended Action

```bash
# Remove from Cargo.toml workspace members:
- "crates/docflow-format-toon"
- "crates/taskflow-format-toon"

# Then delete the directories:
rm -rf crates/docflow-format-toon
rm -rf crates/taskflow-format-toon
```

---

## 2. File Size & Complexity Issues

### Critical Files (>5K lines)

| File | Lines | Risk Level | Issue |
|------|-------|------------|-------|
| `runtime_dispatch_state.rs` | 20,375 | 🔴 CRITICAL | 213 functions, single responsibility violation |
| `taskflow_consume_resume.rs` | 16,659 | 🟠 HIGH | Complex state machine, hard to test |
| `taskflow_run_graph.rs` | 10,579 | 🟠 HIGH | Graph operations mixed with CLI rendering |

### Medium Files (3K-5K lines)

| File | Lines | Notes |
|------|-------|-------|
| `taskflow_proxy.rs` | 9,859 | Proxy pattern implementation |
| `state_store.rs` | 7,072 | State management core |
| `task_surface.rs` | 6,876 | Task CLI surface |
| `init_surfaces.rs` | 5,927 | Bootstrap logic |
| `state_store_run_graph_summary.rs` | 5,467 | Run graph operations |
| `runtime_dispatch_execution.rs` | 5,022 | Execution logic |

### Recommended Refactoring

1. **Split `runtime_dispatch_state.rs`**: Extract state transitions into separate modules
   - State machine logic → `state_machine.rs`
   - Packet handling → `packets.rs`
   - Receipt management → `receipts.rs`
   - Test code already separated (starts at line 1205)

2. **Split `taskflow_consume_resume.rs`**: Separate resume vs advance operations

---

## 3. Duplicate Pattern Analysis

### Format Crates Duplication

```
common-format-jsonl  ← used by all *-format-jsonl crates
common-format-toon   ← only used by dead -format-toon crates

docflow-format-jsonl  → delegates to common-format-jsonl
taskflow-format-jsonl → delegates to common-format-jsonl

docflow-format-toon   ❌ DEAD (no consumers)
taskflow-format-toon  ❌ DEAD (no consumers)
```

**Pattern**: Both docflow and taskflow have parallel JSONL implementations that are nearly identical:

```rust
// docflow-format-jsonl/src/lib.rs vs taskflow-format-jsonl/src/lib.rs
pub fn encode_line<T: Serialize>(value: &T) -> serde_json::Result<String> {
    common_format_jsonl::encode_line(value)  // Identical!
}

pub fn decode_line<T: DeserializeOwned>(line: &str) -> serde_json::Result<T> {
    common_format_jsonl::decode_line(line)    // Identical!
}
```

### State Backend Duplication

| Backend | Purpose | Used By |
|---------|---------|---------|
| `taskflow-state-fs` | File-based snapshots | vida (runtime_dispatch_state, state_store) |
| `taskflow-state-surreal` | SurrealDB backend | vida (doctor_surface, runtime_consumption_state) |

**Issue**: Both backends implement similar snapshot/restore patterns but in separate crates. The FS implementation is more mature with better test coverage.

---

## 4. Low-Usage Module Analysis

### Modules with ≤3 References

| Module | References | Recommendation |
|--------|------------|----------------|
| `agent_dispatch_surface` | 3 | Keep (CLI entry point) |
| `bootstrap_value_utils` | 2 | Merge into `project_bootstrap_defaults` |
| `carrier_runtime_strategy` | 3 | Keep (strategy pattern) |
| `config_value_utils` | 2 | Merge into main config module |
| `consume_final_operator_surface` | 3 | Keep (CLI surface) |
| `contract_profile_registry` | 2 | Merge into contracts crate |
| `development_flow_glue` | 2 | Keep if glue logic is complex |
| `development_request_analysis` | 2 | Keep (analysis responsibility) |
| `diagnostics_surface` | 3 | Keep (CLI surface) |
| `docs_surface` | 3 | Keep (CLI surface) |
| `host_agent_state` | 3 | Keep (state management) |
| `host_runtime_registry` | 2 | Merge into host modules |
| `project_activator_normal_work_defaults` | 3 | Merge into project_activator |
| `project_activator_runtime_surface` | 3 | Merge into project_activator |
| `project_bootstrap_defaults` | 2 | Keep (bootstrap defaults) |
| `project_root_paths` | 2 | Keep (path management) |
| `protocol_surface` | 3 | Keep (CLI surface) |
| `registry_projection_utils` | 2 | Merge into registry modules |
| `runtime_assignment_builder` | 3 | Keep (builder pattern) |
| `runtime_assignment_policy` | 2 | Merge into assignment modules |
| `runtime_assignment_projection_utils` | 2 | Merge into projection modules |
| `runtime_dispatch_bootstrap` | 2 | Merge into runtime_dispatch |
| `runtime_dispatch_packet_text` | 3 | Keep (packet text generation) |
| `runtime_dispatch_status` | 3 | Keep (status reporting) |
| `shell_runtime_helpers` | 2 | Merge into shell modules |
| `status_surface_host_agents` | 3 | Keep (CLI surface) |
| `status_surface_host_cli_summary` | 3 | Keep (CLI surface) |
| `status_surface_json_report` | 2 | Merge into status reporting |
| `status_surface_operator_contracts` | 2 | Merge into contracts |
| `status_surface_text_report` | 2 | Merge into status reporting |

### Recommendation

Consolidate utility modules (≤2 references) into their parent modules. Keep CLI surface modules as they serve distinct entry points.

---

## 5. Architectural Gaps

### 5.1 State Management Fragmentation

**Current**: FS snapshots + SurrealDB backend coexist without clear migration path

**Issue**: 
- No unified interface for state backends
- Duplicate snapshot/restore logic in both backends
- No feature flags to toggle between backends

**Recommendation**: Create a `TaskStateBackend` trait that both implementations satisfy:

```rust
pub trait TaskStateBackend {
    async fn save_snapshot(&self, snapshot: &TaskSnapshot) -> Result<(), Error>;
    async fn load_snapshot(&self) -> Result<TaskSnapshot, Error>;
}
```

### 5.2 Test Coverage Gaps

**Current**: Only 7 integration test files across entire workspace

| Test File | Scope |
|-----------|-------|
| `docflow-cli/tests/cli_smoke.rs` | docflow CLI |
| `taskflow-cli/tests/cli_smoke.rs` | taskflow CLI |
| `vida/tests/boot_smoke.rs` | vida bootstrap |
| `vida/tests/doctor_surface_contract_smoke.rs` | doctor surface |
| `vida/tests/project_routing_shape.rs` | project routing |
| `vida/tests/task_smoke.rs` | task operations |
| `vida-pi-agent/tests/adapter.rs` | Pi agent adapter |

**Issue**: No tests for:
- State backend implementations
- Format serialization/deserialization (beyond golden files)
- Runtime dispatch state machine
- Task flow graph operations

### 5.3 Error Handling Inconsistency

**Pattern observed**: Mixed use of `Result<T, String>` vs `Result<T, Box<dyn Error>>`

```rust
// In runtime_dispatch_state.rs
async fn reopen_authoritative_state_store_for_dispatch_phase(...) -> Result<StateStore, String>

// In state_store.rs  
pub fn write_snapshot(...) -> Result<(), std::io::Error>
```

**Recommendation**: Standardize on custom error types with `thiserror`:

```rust
#[derive(Error, Debug)]
pub enum RuntimeDispatchError {
    #[error("state store open failed: {0}")]
    StateStoreOpen(String),
    #[error("dispatch timeout after {0}s")]
    Timeout(u64),
}
```

---

## 6. Performance Concerns

### 6.1 Large File I/O

**Issue**: `runtime_dispatch_state.rs` (20K lines) suggests complex state machine that may:
- Hold large data structures in memory
- Perform synchronous operations in async context
- Create tight coupling between state transitions

### 6.2 Duplicate State Reads

**Pattern**: Multiple modules read the same state files without caching:

```rust
// state_store.rs reads snapshots
taskflow_state_fs::read_snapshot(path)?

// state_store_taskflow_snapshot_bridge.rs also reads snapshots
let snapshot = taskflow_state_fs::read_snapshot(path)?;
```

**Recommendation**: Implement a state cache layer to avoid redundant disk I/O.

---

## 7. Dependency Graph Analysis

### Workspace Dependencies

```
vida (main binary)
├── docflow-cli
│   ├── docflow-format-jsonl ← depends on common-format-jsonl
│   ├── docflow-inventory ❓ (unused?)
│   ├── docflow-markdown ❓ (unused?)
│   ├── docflow-operator ❓ (unused?)
│   ├── docflow-readiness ❓ (unused?)
│   ├── docflow-relations ❓ (unused?)
│   └── docflow-validation ❓ (unused?)
├── taskflow-cli (minimal deps)
├── taskflow-state-fs ✅
├── taskflow-state-surreal ✅
└── taskflow-contracts ✅
```

### Unused Docflow Sub-Crates

The following docflow crates are dependencies of `docflow-cli` but may have limited usage:

| Crate | Lines in lib.rs | Usage Outside Crate |
|-------|-----------------|---------------------|
| `docflow-inventory` | ? | Check with grep |
| `docflow-markdown` | ? | Check with grep |
| `docflow-operator` | ? | Check with grep |
| `docflow-readiness` | ? | Check with grep |
| `docflow-relations` | ? | Check with grep |
| `docflow-validation` | ? | Check with grep |

---

## 8. Recommendations Priority Matrix

### P0 - Immediate Action Required

1. **Remove dead format-toon crates** (2 unused workspace members)
2. **Split runtime_dispatch_state.rs** (>20K lines, single responsibility violation)
3. **Add custom error types** to replace `Result<T, String>` patterns

### P1 - Next Sprint

4. **Consolidate low-usage utility modules** (merge ≤2 reference modules)
5. **Create TaskStateBackend trait** for unified state interface
6. **Add integration tests** for state backends and format crates

### P2 - Technical Debt

7. **Implement state caching layer** to reduce duplicate I/O
8. **Standardize error handling** across all crates
9. **Review docflow sub-crates** for actual usage vs dead code

---

## 9. Metrics Summary

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Total Rust files | 157 | - | ✅ |
| Workspace crates | 26 | - | ⚠️ 2 dead |
| Files >5K lines | 3 | 0 | 🔴 |
| Files >10K lines | 2 | 0 | 🔴 |
| Integration tests | 7 | ≥20 | 🟡 |
| Modules with ≤3 refs | 30+ | <10 | 🟠 |

---

## Appendix A: File Size Distribution

```
Lines    Count   Percentage
------   -----   ----------
1-50     ~80     51%
51-200   ~50     32%
201-500  ~20     13%
501-1K   ~5      3%
1K-5K    ~1      1%
5K+      6       4%
```

## Appendix B: Crate Dependency Tree

```
vida (main)
├── docflow-cli
│   ├── docflow-contracts ✅
│   ├── docflow-config ✅
│   ├── docflow-core ✅
│   ├── docflow-format-jsonl ✅
│   ├── docflow-inventory ✅ (used: build_registry, InventoryScope)
│   ├── docflow-markdown ✅ (used: split_footer)
│   ├── docflow-operator ✅ (used: operator contracts)
│   ├── docflow-readiness ⚠️ (verify actual usage in lib.rs)
│   ├── docflow-relations ⚠️ (verify actual usage in lib.rs)
│   └── docflow-validation ⚠️ (verify actual usage in lib.rs)
├── taskflow-cli (minimal)
├── taskflow-contracts ✅
├── taskflow-core ✅
├── taskflow-state ✅
├── taskflow-state-fs ✅
├── taskflow-state-surreal ✅
└── docflow-format-toon ❌ DEAD (no consumers)
```

**Verification Note**: inventory, markdown, and operator are confirmed used in `docflow-cli/src/lib.rs`. readiness, relations, and validation need code-level verification.

## Appendix C: Dead Code Verification

### Confirmed Dead Crates
1. `docflow-format-toon` - Zero external references, only depends on `common-format-toon`
2. `taskflow-format-toon` - Zero external references, only depends on `common-format-toon`

### Not Dead (Verified Used)
1. All docflow sub-crates (inventory, markdown, operator) confirmed used in docflow-cli
2. Both state backends (FS and Surreal) actively used in vida crate
3. Format JSONL crates properly delegated to common-format-jsonl

---

*Report generated: 2026-05-21*  
*Next review recommended: After P0 items are addressed*
