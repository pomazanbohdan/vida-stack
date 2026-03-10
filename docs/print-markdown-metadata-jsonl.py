#!/usr/bin/env python3
"""Thin wrapper for codex scan over docs/."""

from __future__ import annotations

from pathlib import Path
import runpy
import sys


def main() -> int:
    script = Path(__file__).resolve().parents[1] / "codex-v0" / "codex.py"
    sys.argv = [str(script), "scan", "--root", str(Path(__file__).resolve().parent), *sys.argv[1:]]
    runpy.run_path(str(script), run_name="__main__")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
