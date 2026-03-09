#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path

from toon_format import encode


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] != "-":
        payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    else:
        payload = json.load(sys.stdin)
    sys.stdout.write(encode(payload))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
