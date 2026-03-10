# Package
version       = "0.2.0"
author        = "VIDA Stack"
description   = "VIDA v0 runtime — Nim alternative to Python/Shell scripts"
license       = "MIT"
srcDir        = "src"
bin           = @["taskflow-v0"]

# Dependencies
requires "nim >= 2.0.0"
requires "yaml >= 2.1.1"          # Reserved for future full YAML support
requires "cligen >= 1.7.0"        # CLI argument parsing from proc signatures
requires "chronicles >= 0.10.3"   # Structured logging (JSON/text) for run logs
requires "results >= 0.5.0"       # Result[T, E] error handling without exceptions
requires "dotenv >= 2.0.0"        # .env file loading (VIDA_ROOT etc.)
requires "checksums >= 0.2.0"     # SHA256 for boot receipt file hashes
requires "regex >= 0.25.0"        # Pattern matching for pack_router_keywords
requires "termstyle >= 0.2.0"     # Colored terminal output (bold, red, green etc.)

# Tasks
task test, "Run unit tests":
  exec "nim c -r tests/test_utils.nim"
  exec "nim c -r tests/test_config.nim"
