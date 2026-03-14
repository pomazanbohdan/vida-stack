# Role Profile Protocol

Purpose: define the canonical framework/runtime contract for `role profile` so agent identity and stance remain separate from permissions, authority, and runtime execution law.

## Core Contract

1. `role profile` defines role identity and stable behavioral stance.
2. `role profile` does not own permissions, approval authority, or fallback law.
3. `role profile` is an input to `Agent Definition`, not a replacement for it.

## What A Role Profile Owns

1. role identity
2. mission framing or stance
3. stable tone/interaction posture
4. compatibility with instruction-contract families
5. compatibility with prompt-template families

## What A Role Profile Must Not Own

1. tool permission policy
2. approval authority
3. escalation law
4. fallback ladder
5. output/proof contract

Those remain owned by:

1. `Instruction Contract`
2. broader `Agent Definition` assembly
3. framework/runtime route law

## Minimum Contract

Every role profile should define at minimum:

1. `role_profile_id`
2. `version`
3. `role_mission`
4. `stance`
5. `tone_constraints`
6. `compatible_instruction_contracts`
7. `compatible_prompt_templates`
8. `non_authority_note`

## Separation Rule

If a role profile starts carrying:

1. hidden authority,
2. hidden permissions,
3. implicit fallback behavior,
4. output/proof obligations,

then the profile is drifting into instruction-contract territory and must be corrected.

## Assembly Rule

Use the following relation:

1. role profile provides identity and stance,
2. instruction contract provides behavior law,
3. prompt template configuration renders the behavior,
4. skill attachment remains separate.

## Promotion Rule

When a runtime-bearing role is introduced:

1. define or reference the role profile,
2. bind it to one or more instruction contracts,
3. bind it to one or more prompt template configurations,
4. keep permissions and authority outside the role profile itself.

Project extension rule:

1. project-owned profiles may extend framework role usage for one project, but they must still resolve to a known framework base role or a validated project role derived from one.
2. a project profile may attach project skills and project flow-set preferences only through the validated project-extension path.
3. a project profile must not invent new authority, approval power, or escalation law that the resolved base role does not already permit.
4. framework base roles such as `business_analyst` and `pm` remain valid role-profile anchors for requirement shaping, scope formation, and task-formation flows.

## References

1. `agent-definitions/model.agent-definitions-contract`
2. `docs/product/spec/instruction-artifact-model.md`
3. `docs/product/spec/agent-role-skill-profile-flow-model.md`
4. `docs/process/framework-source-lineage-index.md`

-----
artifact_path: config/instructions/agent-definitions/role.role-profile.contract
artifact_type: agent_definition
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/agent-definitions/role.role-profile-contract.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-12T11:47:59+02:00'
changelog_ref: role.role-profile-contract.changelog.jsonl
