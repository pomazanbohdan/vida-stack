# Library Evaluation Protocol

Purpose: require live-source evaluation of third-party libraries, alternatives comparison, and explicit selection before adopting crates for core runtime behavior.

## Core Contract

1. Do not adopt a library for core runtime behavior based only on memory or first-hit familiarity.
2. Evaluate multiple live alternatives.
3. Record the comparison in a capability matrix.
4. Choose one winner explicitly with reasons.

## Trigger Cases

Run this protocol when:

1. adding a new core crate,
2. selecting a diff/patch/ingest/parser/storage library,
3. replacing a current library,
4. uncertain API or maintenance state matters to the design.

## Required Checks

For each candidate, check live sources for:

1. current release/version,
2. maintenance activity,
3. official docs quality,
4. API fit for the intended use,
5. ecosystem maturity,
6. obvious alternatives,
7. known constraints or risks.

## Mandatory Matrix

The evaluation must include at minimum these columns:

1. candidate
2. purpose fit
3. current version / freshness
4. docs quality
5. implementation complexity
6. runtime suitability
7. migration risk
8. export/debug suitability
9. winner / non-winner rationale

## Selection Rule

1. pick the smallest library set that satisfies the runtime need,
2. prefer stable primary-purpose libraries over clever over-general stacks,
3. prefer canonical runtime formats defined by VIDA over leaking third-party formats into product law,
4. if no library cleanly fits, say so and implement the minimal direct solution.

## Fail-Closed Rule

1. Do not adopt a crate into core runtime without checking live alternatives.
2. Do not rely on stale memory for external API or maintenance assumptions.
