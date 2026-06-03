# Defect Fix Plan: runtime-validation-circular-dependency-blocks-legitimate-task-closure

## PROBLEM ANALYSIS

### Root Cause
VIDA's graph validation in `state_store_task_graph.rs:validate_task_graph_rows()` enforces that NO closed task can have open children. This validation runs on the ENTIRE graph after simulating cascading closure, but does not account for:

1. **Cascading effects**: Closing a leaf task should allow closing its parent if all children are now closed
2. **Core Rule #12**: User conversational approval should override automatic validation
3. **Partial chains**: Parent chain closure stops at first parent with ANY open child, even unrelated ones

### Current Behavior
When user attempts to close `fix-session-identity-scoped-continuation-before-tui-service`:
1. Direct children check: PASS (leaf task has no children)
2. Parent chain closure:
   - Close fix-session-identity-scoped-continuation-before-tui-service
   - Check feature-multi-orchestrator-session-scoped-ownership-clai: all 14 children closed? NO
     (runtime-validation-circular-dependency-blocks-legitimate-task-closure is open)
   - STOP: Do not close parent
3. Graph validation on reconciled_tasks:
   - Checks ALL tasks for closed_parent_has_open_child
   - BLOCKS with: "closed_parent_has_open_child on runtime-normal-operation-recovery-epic"

**Issue**: Error message is misleading - runtime-normal-operation-recovery-epic is NOT closed, it's in_progress.

### Reproduction
```bash
vida task close fix-session-identity-scoped-continuation-before-tui-service --reason "Code complete" --json
# Result: Failed with "closed_parent_has_open_child on runtime-normal-operation-recovery-epic"
```

## SOLUTION ARCHITECTURE

Three layers of defense:

1. **Prevention Layer**: Smarter cascading closure that closes all resolvable parents
2. **Bypass Layer**: Core Rule #12 override for user-authorized closures  
3. **Detection Layer**: Better error messages and self-diagnosis

## IMPLEMENTATION PHASES

### Phase 1: Core Rule #12 Override (P0 - Quick Win)
**Goal**: Allow user-authorized closures to bypass validation

**Changes**:
- Add `core_rule_12_override_allowed(close_reason: &str) -> bool` function
- Modify `close_task()` to check override before returning validation error
- Add tracing/logging for override usage

**File**: `crates/vida/src/state_store_task_store.rs`

**New Function**:
```rust
fn core_rule_12_override_allowed(close_reason: &str) -> bool {
    let lower_reason = close_reason.to_lowercase();
    let approval_phrases = [
        "user authorized",
        "user approval",
        "core rule #12",
        "core rule 12",
        "explicit user authorization",
        "user conversational approval",
    ];
    approval_phrases.iter().any(|phrase| lower_reason.contains(phrase))
}
```

**Modify close_task**:
```rust
let issues = Self::validate_task_graph_rows(&reconciled_tasks);
if let Some(first) = issues.first() {
    if Self::core_rule_12_override_allowed(reason) {
        tracing::info!("Core Rule #12 override: {}", first.issue_type);
    } else {
        return Err(...);
    }
}
```

### Phase 2: Enhanced Cascading Closure (P1 - Root Fix)
**Goal**: Fix circular dependency at source

**Changes**:
- Modify `close_parent_chain_without_active_children()` to track tasks being closed
- Only stop cascading if open child is NOT in the closure chain

**File**: `crates/vida/src/state_store_task_store.rs`

**Logic**:
```rust
// Track which tasks we're closing in this operation
let mut tasks_being_closed = HashSet::from([leaf_task_id]);

// When checking parent's children:
let has_open_child_not_in_chain = children.iter().any(|child_id| {
    if tasks_being_closed.contains(child_id) {
        return false; // This child IS being closed, so it's OK
    }
    tasks.iter().find(|t| t.id == *child_id)
        .map(|child| child.status == "open" || child.status == "in_progress")
        .unwrap_or(false)
});

if has_open_child_not_in_chain {
    break; // Only stop for children NOT in our closure chain
}

// When closing a parent, add to tracking:
tasks_being_closed.insert(parent_id.clone());
```

### Phase 3: Improved Diagnostics (P2 - UX)
**Goal**: Better error messages

**Changes**:
- Enhance error messages in `close_task()` with diagnostic info and defect references

**File**: `crates/vida/src/state_store_task_store.rs`

### Phase 4: Test Suite
**Unit Tests** (in state_store_task_store.rs):
- `test_cascading_closure_leaf_to_parent`
- `test_cascading_closure_multi_level`
- `test_cascading_stops_at_unrelated_open_sibling`
- `test_core_rule_12_override_allows_closure`
- `test_core_rule_12_phrase_detection`

**Integration Tests** (in task_smoke.rs):
- `multi_session_task_closure_cascades_to_epic`
- `multi_session_user_authorization_overrides_validation`

## ACCEPTANCE CRITERIA

### Functional
- [ ] Cascading closure works: leaf -> parent -> grandparent
- [ ] Core Rule #12 override works for any task with explicit authorization
- [ ] Error messages include diagnostic information

### Code Quality
- [ ] `cargo test -p vida state_store_task_store` passes
- [ ] `cargo test -p vida --test task_smoke multi_session` passes
- [ ] `cargo fmt --check` passes
- [ ] `git diff --check` passes

### Test Coverage
- [ ] Unit tests: >=8 new tests
- [ ] Integration tests: >=3 new tests
- [ ] All existing tests still pass

## IMPLEMENTATION ORDER

1. Phase 1: Core Rule #12 Override (~2 hours)
2. Phase 4: Unit Tests for override (~3 hours)
3. Phase 2: Enhanced Cascading (~4 hours)
4. Phase 3: Diagnostics (~1 hour)
5. Phase 4: Integration Tests (~2 hours)
6. Validation & Closure (~1 hour)

**Total**: 13-15 hours

## FILES TO MODIFY

### Code
- `crates/vida/src/state_store_task_store.rs` (primary)

### Tests
- `crates/vida/src/state_store_task_store.rs` (unit tests)
- `crates/vida/tests/task_smoke.rs` (integration tests)

## REFERENCES

- **Defect**: `runtime-validation-circular-dependency-blocks-legitimate-task-closure`
- **Core Rule**: #12 - User conversational approval overrides automatic states
- **Related Tasks**: All multisession tasks now closed
