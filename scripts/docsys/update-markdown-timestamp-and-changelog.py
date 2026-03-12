#!/usr/bin/env python3
"""Thin wrapper for codex touch over docs/."""

from __future__ import annotations

from pathlib import Path
import runpy
import sys


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    script = repo_root / "codex-v0" / "codex.py"
    sys.argv = [str(script), "touch", *sys.argv[1:]]
    runpy.run_path(str(script), run_name="__main__")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
