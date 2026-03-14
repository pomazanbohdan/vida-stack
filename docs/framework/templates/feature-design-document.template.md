# [FEATURE] Design Document

Status: `draft | proposed | approved | implemented | superseded`

Use this template for one bounded feature/change design before implementation.

Structured-template rule:
1. Keep headings stable.
2. Replace placeholders rather than rewriting the shape.
3. Prefer explicit fields and short bullets over long free-form prose.
4. Link separate ADRs when one or more major decisions need durable decision records.

## Summary
- Feature / change:
- Owner layer: `project | framework | runtime-family | mixed`
- Runtime surface: `launcher | docflow | taskflow | project activation | other`
- Status:

## Current Context
- Existing system overview
- Key components and relationships
- Current pain point or gap

## Goal
- What this change should achieve
- What success looks like
- What is explicitly out of scope

## Requirements

### Functional Requirements
- Must-have behavior
- Integration points
- User-visible or operator-visible expectations

### Non-Functional Requirements
- Performance
- Scalability
- Observability
- Security

## Ownership And Canonical Surfaces
- Project docs / specs affected:
- Framework protocols affected:
- Runtime families affected:
- Config / receipts / runtime surfaces affected:

## Design Decisions

### 1. [Decision Title]
Will implement / choose:
- Why
- Trade-offs
- Alternatives considered
- ADR link if this must become a durable decision record

### 2. [Decision Title]
Will implement / choose:
- Why
- Trade-offs
- Alternatives considered
- ADR link if needed

## Technical Design

### Core Components
- Main components
- Key interfaces
- Bounded responsibilities

### Data / State Model
- Important entities
- Receipts / runtime state / config fields
- Migration or compatibility notes

### Integration Points
- APIs
- Runtime-family handoffs
- Cross-document / cross-protocol dependencies

### Bounded File Set
- List every file expected to change
- Keep this list explicit and bounded

## Fail-Closed Constraints
- Forbidden fallback paths
- Required receipts / proofs / gates
- Safety boundaries that must remain true during rollout

## Implementation Plan

### Phase 1
- Initial implementation tasks
- First proof target

### Phase 2
- Integration / refinement tasks
- Second proof target

### Phase 3
- Hardening / rollout tasks
- Final proof target

## Validation / Proof
- Unit tests:
- Integration tests:
- Runtime checks:
- Canonical checks:
  - `activation-check`
  - `protocol-coverage-check`
  - `check`
  - `doctor`

## Observability
- Logging points
- Metrics / counters
- Receipts / runtime state written

## Rollout Strategy
- Development rollout
- Migration / compatibility notes
- Operator or user restart / restart-notice requirements

## Future Considerations
- Follow-up ideas
- Known limitations
- Technical debt left intentionally

## References
- Related specs
- Related protocols
- Related ADRs
- External references
