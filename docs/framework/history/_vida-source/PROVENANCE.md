# _vida Source Provenance

Status: preserved raw source snapshot

Purpose: keep one historical `_vida` tree without content loss after the framework/product cutover.

Selection rule:
1. This tree preserves the original `_vida` file content more faithfully than the rewritten mirror used during migration.
2. When the rewritten mirror and this tree disagreed, this tree won for preservation because it retained older source text and older file modification times.
3. Current canon does not live here; current canon lives in `docs/framework/**`, `docs/product/**`, `vida/config/**`, and `vida-v0/**`.

Deduplication note:
1. The temporary `docs/framework/history/_vida-archive/**` mirror was removed after live references were rewritten to this preserved source tree.
2. Internal links and paths inside this tree may still mention historical `_vida/...` locations; treat them as source evidence, not as current active paths.
