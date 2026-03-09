# Boot Packet Protocol

Purpose: provide a compact machine-readable boot artifact that summarizes the minimal read contract and runtime activation state for a selected VIDA boot profile.

## Why

Boot packet v0 exists to reduce repeated rereads and make boot expectations available as a compact runtime artifact instead of only as long-form markdown.

It is not a full protocol compiler.

## Command

```bash
vida-v0 boot run <lean|standard|full> [task_id] [--non-dev]
vida-v0 boot read-contract <lean|standard|full> [--non-dev]
vida-v0 boot summary <task_id|session>
```

## Output Contract

Boot packet should expose:

1. selected profile,
2. whether the run is `non_dev`,
3. active `language_policy`,
4. active `protocol_activation`,
5. compact `read_contract`,
6. compact invariant list,
7. runtime hints for compact boot-time status access.

## Integration With Boot Receipts

When boot is executed through `vida-v0 boot ...`:

1. a boot packet should be written next to the receipt,
2. receipt should record `boot_packet_file`,
3. dev-oriented boots should also write a compact boot snapshot artifact next to the receipt,
4. `verify-receipt` should fail if the referenced boot packet is missing,
5. `verify-receipt` should fail if receipt profile and boot packet profile diverge,
6. if receipt references a boot snapshot artifact, `verify-receipt` should fail when it is missing,
7. task-scoped health checks should verify the latest receipt and referenced boot packet before close/readiness checks pass.

## Scope

Boot packet is a runtime convenience artifact.

Canonical policy still lives in:

1. `AGENTS.md`
2. `docs/framework/thinking-protocol.md`
3. `docs/framework/project-overlay-protocol.md`
4. `vida-v0`

## Current Version

`v0`

Characteristics:

1. generated on demand,
2. no signature/hash enforcement yet,
3. integrated with boot receipts and receipt verification,
4. `vida-v0 boot run` should consume boot-packet read-contract output instead of duplicating profile file lists,
5. health/verification flows may consume packet summaries as a compact proof surface,
6. dev-oriented boot receipts may include a compact task-state snapshot artifact,
7. intended as the first step toward lighter compiled boot contracts.
