## Compatibility shim for the modular config surface.
##
## Phase A keeps the public API stable while ownership moves into `src/config/**`.

import ../config/[loader, bundle_builder, accessors, schema, validation/aggregate]

export loader, bundle_builder, accessors, schema, aggregate
