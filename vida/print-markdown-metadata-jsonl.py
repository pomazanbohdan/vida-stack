#!/usr/bin/env python3
"""Thin wrapper for `vida docflow scan` over `vida/`."""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    command = [
        "vida",
        "docflow",
        "scan",
        "--root",
        str(Path(__file__).resolve().parent),
        *sys.argv[1:],
    ]
    completed = subprocess.run(command, cwd=repo_root)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
