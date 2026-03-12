# Security Policy

Status: canonical

Purpose: define how vulnerabilities and security-sensitive issues must be reported for the Vida Stack repository.

## Reporting Rule

Do not open a public GitHub issue for vulnerabilities, secrets exposure, supply-chain compromise, or other security-sensitive findings.

Use private reporting instead.

## How To Report

Send a private report to the repository maintainer through the primary repository contact path.

A useful report should include:

1. affected component or path,
2. reproduction steps when safe to share,
3. impact summary,
4. version or commit context,
5. any mitigation already known.

## Response Posture

Best effort is made to:

1. acknowledge the report,
2. assess impact,
3. decide whether the issue requires immediate containment, patching, or coordinated disclosure,
4. publish a fix or mitigation path when appropriate.

No public response timeline is guaranteed in this early repository phase.

## Scope

This policy applies to:

1. released binaries and install surfaces,
2. repository automation,
3. dependency or supply-chain exposure,
4. runtime command and state-handling vulnerabilities,
5. credential or secret leakage in repository-controlled surfaces.

-----
artifact_path: project/repository/security
artifact_type: repository_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: SECURITY.md
created_at: '2026-03-12T10:30:00+02:00'
updated_at: '2026-03-12T08:04:26+02:00'
changelog_ref: SECURITY.changelog.jsonl
