# Release Formatting Protocol

Purpose: define the canonical project-owned process for turning versioned release-note artifacts into clean public GitHub release pages without leaking canonical document metadata into the public release surface.

## Core Rule

GitHub release pages are publication surfaces, not raw canonical artifact dumps.

Rules:

1. the canonical source for a release note remains `install/release-notes-v*.md`,
2. the public GitHub release body must be rendered from that source only up to the first `-----` separator,
3. metadata footer fields such as `artifact_path`, `artifact_type`, `schema_version`, `source_path`, `created_at`, `updated_at`, and `changelog_ref` must never appear in the public GitHub release body,
4. public release formatting must preserve the human-facing headings and narrative from the canonical release note rather than rewriting the release ad hoc in the GitHub UI.

## Naming Rule

Rules:

1. public release titles must use `Vida Stack vX.Y.Z`,
2. hotfix, slice, and milestone wording belongs in the body, not in inconsistent title variants,
3. tags remain the authoritative version identifiers, but the title must stay human-readable and consistent across the active release line.

## Source Rule

The canonical project-owned release-note source lives under `install/`.

Rules:

1. `install/release-notes-v*.md` is the durable source artifact,
2. the footer below `-----` remains valid for project canon, changelog linkage, and documentation tooling,
3. the footer is not part of the public release body,
4. if the canonical release note changes after publication, the GitHub release page must be updated to match the rendered public body from the current artifact revision.

## Body Structure Rule

The public body should keep the release readable and operator-oriented.

Required sections:

1. release title,
2. highlights or hotfix highlights,
3. what changed or equivalent delta section,
4. practical outcome or operator impact,
5. proof snapshot, direction, or release-positioning section when it materially clarifies the slice.

Rules:

1. keep the body concise and scannable,
2. prefer the canonical headings already used in the source artifact,
3. keep exact commands only when they materially help the operator,
4. avoid internal documentation metadata, internal artifact identifiers, or changelog bookkeeping in the public body.

## Asset Rule

Public release pages must stay aligned with the packaged release.

Rules:

1. attach the current archive assets, installer asset, checksum file, and manifest when they exist for the release line,
2. the release body must not claim assets or bootstrap behavior that the attached artifacts do not actually provide,
3. if the release archive contents change, the release body must be rechecked against the built assets before publication.

## Publication Sequence

For the active `0.2.x` line:

1. build the release assets,
2. confirm the matching `install/release-notes-v*.md` artifact is current,
3. render the public body from that artifact without the metadata footer,
4. create or edit the GitHub release using the rendered body,
5. verify the release title, body, tag, and attached assets on GitHub after publication.

## Tooling Rule

The current thin render helper is:

1. [render-public-release-notes.sh](/home/unnamed/project/vida-stack/scripts/render-public-release-notes.sh)

Rules:

1. this helper renders the public body from the canonical `install/release-notes-v*.md` artifact or a directly supplied file path,
2. it must strip the metadata footer at `-----`,
3. GitHub release publication should consume that rendered output rather than the raw canonical file body.

## Current 0.2.x Interpretation

For the active proving line:

1. `v0.2.0`, `v0.2.1`, and `v0.2.2` public release pages should all follow one title convention: `Vida Stack vX.Y.Z`,
2. the public release page is the operator-facing narrative surface,
3. the canonical release-note artifact remains the documentation-owned source of truth,
4. GitHub release formatting drift is a project-process bug and must be corrected through this protocol rather than by informal manual editing alone.

-----
artifact_path: process/release-formatting-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/process/release-formatting-protocol.md
created_at: '2026-03-12T16:37:07+02:00'
updated_at: '2026-03-12T16:37:07+02:00'
changelog_ref: release-formatting-protocol.changelog.jsonl
