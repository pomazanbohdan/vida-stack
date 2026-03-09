# Debug Escalation Protocol

Purpose: define when autonomous debugging must escalate from local trial-and-error to primary-source lookup and external verification.

## Core Contract

1. One bounded local fix attempt is normal.
2. If the same technical error repeats a second time, escalate.
3. If an external API, crate format, or protocol surface is uncertain, escalate as soon as uncertainty is material.

## Escalation Rule

Escalate to primary-source lookup when any are true:

1. the same compile/runtime error appears twice,
2. the same class of fix fails twice,
3. the API/crate/format/version behavior is not confidently known,
4. the failure concerns external library semantics rather than only local code logic.

## Mandatory Escalation Sequence

When escalation is triggered, use this order unless a stronger law already narrows the path:

1. capture the repeated error and failed local hypotheses in task evidence,
2. dispatch at least one bounded external/catch review agent for an independent diagnosis,
3. perform primary-source lookup,
4. if primary sources remain ambiguous, perform broader web/Google search,
5. synthesize a bounded fix from the combined evidence before editing again.

Hard rule:

1. after the same technical error appears twice, do not continue with solo local trial-and-error only,
2. at least one of `external agent review` or `primary-source/web lookup` must run before the next substantive fix attempt,
3. for external crate/API/version semantics, prefer doing both.

## Preferred Lookup Order

1. local code or docs already present,
2. official crate/docs/source,
3. upstream issue tracker or release notes,
4. broader web search only if primary sources are insufficient.

Google/web-search activation rule:

1. if the same library/API uncertainty survives one primary-source pass, Google/web search becomes mandatory on the next pass,
2. if the runtime error includes parser, version, syntax, or result-shape ambiguity in third-party systems, treat it as Google-eligible immediately,
3. store the chosen source links or bounded evidence summary in task artifacts/receipts.

External-agent activation rule:

1. when subagent mode is not `disabled`, dispatch a bounded catch/review/diagnostic agent in parallel with the first escalation lookup,
2. use that agent for independent diagnosis, alternative fix shape, or API/result-shape validation,
3. if no eligible external agent exists, record explicit `no_eligible_external_agent` evidence and continue with primary-source/web escalation.

## Fail-Closed Rule

1. Do not keep repeating blind edits after repeated API drift failures.
2. Do not pretend confidence about unknown external formats or APIs.
