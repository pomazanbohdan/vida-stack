# VIDA Compatibility Boundary

Legacy command compatibility is resolved before mutation by the contracts layer.

- Canonical operation ids stay authoritative.
- Retained aliases canonicalize to the same `vida.*` operation id before Tower pipeline lookup.
- Ambiguous aliases fail while parsing the command envelope.
- Alias use is measured through `legacy_operation_alias_receipt`.
- Compatibility code must not write TaskFlow, DocFlow, receipt, run-graph, lane, or state-store data directly.

Retained root command aliases are resolved in `compat::resolve_legacy_root_alias`:

- `vida consume ...` -> `vida taskflow consume ...`
- `vida recovery ...` -> `vida taskflow recovery ...`
- `vida route ...` -> `vida taskflow route ...`

Each retained root alias emits `legacy_root_alias_used` deprecation metadata before forwarding to the canonical TaskFlow proxy.
Nested aliases such as `vida consume taskflow ...` fail before proxy dispatch.
