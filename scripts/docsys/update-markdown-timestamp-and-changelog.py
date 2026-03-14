#!/usr/bin/env python3
"""Thin wrapper for `vida docflow touch` over `docs/`."""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    completed = subprocess.run(
        ["vida", "docflow", "touch", *sys.argv[1:]],
        cwd=repo_root,
    )
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
