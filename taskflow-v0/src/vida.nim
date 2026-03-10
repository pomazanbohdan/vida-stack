## VIDA v0 — Nim CLI entry point.
##
## Thin bootstrap adapter over the modular CLI dispatch surface.

import std/[os]
import cli/dispatch

proc main() =
  quit(runCli(commandLineParams()))

when isMainModule:
  main()
